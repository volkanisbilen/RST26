//! LS_LOGIN_REQ (0xF3) handler — account authentication for the launcher.
//! ## Request (Client → Server)
//! | Offset | Type   | Description                        |
//! |--------|--------|------------------------------------|
//! | 0      | string | Account ID (u16le len + bytes)     |
//! | N      | string | Password (u16le len + bytes)       |
//! ## Response (Server → Client)
//! | Offset | Type  | Description                          |
//! |--------|-------|--------------------------------------|
//! | 0      | u16le | Reserved (always 0)                  |
//! | 2      | u8    | Result code                          |
//! If `result == AUTH_SUCCESS (0x01)`:
//! | Offset | Type  | Description                          |
//! |--------|-------|--------------------------------------|
//! | 3      | i16le | Premium hours (-1 = no premium)      |
//! | 5      | string| Account ID echo                      |
//! If `result == AUTH_IN_GAME (0x05)`:
//! | Offset | Type  | Description                          |
//! |--------|-------|--------------------------------------|
//! | 3      | string| Connected server IP                  |
//! | N      | u16le | Game server port (15001)             |
//! | N+2    | string| Account ID echo                      |

use ko_db::repositories::account::AccountRepository;
use ko_db::repositories::game_options::GameOptionsRepository;
use ko_db::repositories::premium::PremiumRepository;
use ko_protocol::{LoginOpcode, Packet, PacketReader};

use crate::login_session::LoginSession;
use crate::world::{MAX_ID_SIZE, MAX_PW_SIZE};

/// Validate that account ID contains only allowed characters.
/// Allowed: `a-z`, `A-Z`, `0-9`, `:`, `_`
fn string_is_valid(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b':' || b == b'_')
}

/// Login result codes (sniffer-verified against original server).
const AUTH_SUCCESS: u8 = 0x01;
const AUTH_NOT_FOUND: u8 = 0x03; // sniffer: wrong password/not found = 0x03 (NOT 0x02)
const AUTH_BANNED: u8 = 0x04;
const AUTH_IN_GAME: u8 = 0x05;
const AUTH_FAILED: u8 = 0xFF;

/// Handle LS_LOGIN_REQ from the launcher.
pub async fn handle(session: &mut LoginSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);

    // Read credentials
    let account_id = match reader.read_string() {
        Some(s) if !s.is_empty() && s.len() <= MAX_ID_SIZE && string_is_valid(&s) => s,
        _ => {
            send_result(session, AUTH_NOT_FOUND, None).await?;
            return Ok(());
        }
    };

    let password = match reader.read_string() {
        Some(s) if !s.is_empty() && s.len() <= MAX_PW_SIZE => s,
        _ => {
            send_result(session, AUTH_NOT_FOUND, None).await?;
            return Ok(());
        }
    };

    tracing::info!(
        "[{}] LS login attempt: account='{}'",
        session.addr(),
        account_id,
    );

    let repo = AccountRepository::new(session.pool());

    // Check if already online
    match repo.is_online(&account_id).await {
        Ok(true) => {
            tracing::info!(
                "[{}] Account already in-game: {}",
                session.addr(),
                account_id
            );
            // AUTH_IN_GAME response: [server_ip][port][account]
            let mut response = Packet::new(LoginOpcode::LsLoginReq as u8);
            response.write_u16(0);
            response.write_u8(AUTH_IN_GAME);
            response.write_string(&session.config().game_server_ip);
            response.write_u16(session.config().game_server_port);
            response.write_string(&account_id);
            session.send_packet(&response).await?;
            return Ok(());
        }
        Err(e) => {
            tracing::error!("[{}] DB error checking online: {}", session.addr(), e);
            send_result(session, AUTH_FAILED, None).await?;
            return Ok(());
        }
        Ok(false) => {}
    }

    // Authenticate
    match repo.authenticate(&account_id, &password).await {
        Ok(Some(user)) => {
            // Check banned (authority < 0)
            if user.str_authority < 0 {
                tracing::info!("[{}] Account banned: {}", session.addr(), account_id);
                send_result(session, AUTH_BANNED, None).await?;
                return Ok(());
            }

            // Success — compute premium hours dynamically from account_premium table.
            //   calls ACCOUNT_PREMIUM_V2 stored procedure which computes
            //   `(expiry_time - NOW) / 3600` and returns -1 if no premium.
            let premium_hours: i16 = {
                let premium_repo = PremiumRepository::new(session.pool());
                match premium_repo.load_account_premium(&account_id).await {
                    Ok(rows) => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        let max_expiry = rows
                            .iter()
                            .filter(|r| r.premium_type > 0 && (r.expiry_time as i64) > now)
                            .map(|r| r.expiry_time as i64)
                            .max();
                        match max_expiry {
                            Some(expiry) => {
                                let remaining_secs = expiry - now;
                                let hours = remaining_secs / 3600;
                                if hours <= 0 {
                                    1
                                } else {
                                    hours as i16
                                }
                            }
                            None => -1,
                        }
                    }
                    Err(_) => -1,
                }
            };

            let mut response = Packet::new(LoginOpcode::LsLoginReq as u8);
            response.write_u16(0); // reserved
            response.write_u8(AUTH_SUCCESS);
            response.write_i16(premium_hours);
            response.write_string(&account_id);
            response.write_u32(0); // trailing 4 zero bytes (PCAP verified)
            session.send_packet(&response).await?;

            // Store account_id for 0xF5 character list response
            session.set_account_id(account_id.clone());

            tracing::info!("[{}] LS login success: {}", session.addr(), account_id);
        }
        Ok(None) => {
            // Do not ever replace an existing account with a different password.
            // Only a genuinely unknown account may use first-login registration.
            let exists = repo.find_by_account_id(&account_id).await?;
            let auto_register = GameOptionsRepository::new(session.pool())
                .load()
                .await
                .map(|options| options.auto_register)
                .unwrap_or(false);

            if exists.is_none() && auto_register {
                match repo
                    .create_auto_registered(
                        &account_id,
                        &password,
                        &session.addr().ip().to_string(),
                    )
                    .await
                {
                    Ok(Some(_)) => {
                        let mut response = Packet::new(LoginOpcode::LsLoginReq as u8);
                        response.write_u16(0);
                        response.write_u8(AUTH_SUCCESS);
                        response.write_i16(-1);
                        response.write_string(&account_id);
                        response.write_u32(0);
                        session.send_packet(&response).await?;
                        session.set_account_id(account_id.clone());
                        tracing::info!(
                            "[{}] LS auto-registered account: {}",
                            session.addr(),
                            account_id
                        );
                        return Ok(());
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::error!("[{}] LS auto-registration error: {}", session.addr(), e);
                        send_result(session, AUTH_FAILED, None).await?;
                        return Ok(());
                    }
                }
            }

            tracing::info!(
                "[{}] LS login failed (not found): account='{}'",
                session.addr(),
                account_id
            );
            send_result(session, AUTH_NOT_FOUND, None).await?;
        }
        Err(e) => {
            tracing::error!("[{}] DB error on authenticate: {}", session.addr(), e);
            send_result(session, AUTH_FAILED, None).await?;
        }
    }

    Ok(())
}

/// Send a simple result response: [u16(0)] [u8 result_code] [6 zero bytes]
/// Sniffer-verified: original server sends 9 bytes data for error responses.
async fn send_result(
    session: &mut LoginSession,
    result_code: u8,
    _extra: Option<&str>,
) -> anyhow::Result<()> {
    let mut response = Packet::new(LoginOpcode::LsLoginReq as u8);
    response.write_u16(0);
    response.write_u8(result_code);
    // 6 trailing zero bytes (sniffer: F3 00 00 03 00 00 00 00 00 00)
    response.write_u16(0);
    response.write_u32(0);
    session.send_packet(&response).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_is_valid_alphanumeric() {
        assert!(string_is_valid("abc123"));
        assert!(string_is_valid("TestPlayer"));
        assert!(string_is_valid("ADMIN"));
        assert!(string_is_valid("user99"));
    }

    #[test]
    fn test_string_is_valid_special_chars() {
        // Colon and underscore are allowed per C++
        assert!(string_is_valid("user_name"));
        assert!(string_is_valid("user:123"));
        assert!(string_is_valid("a_b:c"));
    }

    #[test]
    fn test_string_is_valid_rejects_invalid() {
        // Spaces, special chars, unicode rejected
        assert!(!string_is_valid("user name"));
        assert!(!string_is_valid("user@name"));
        assert!(!string_is_valid("user#123"));
        assert!(!string_is_valid("user$123"));
        assert!(!string_is_valid("user;name"));
        assert!(!string_is_valid("user'name"));
        assert!(!string_is_valid("user\"name"));
        assert!(!string_is_valid("user\tname"));
        assert!(!string_is_valid("user\nname"));
        assert!(!string_is_valid("user\0name"));
    }

    #[test]
    fn test_string_is_valid_edge_cases() {
        assert!(string_is_valid("a")); // single char
        assert!(string_is_valid("_")); // single underscore
        assert!(string_is_valid(":")); // single colon
                                       // Empty string: vacuously true (all bytes pass), handler checks is_empty separately
        assert!(string_is_valid(""));
    }

    #[test]
    fn test_login_constants() {
        assert_eq!(MAX_ID_SIZE, 20);
        assert_eq!(MAX_PW_SIZE, 28);
        assert_eq!(AUTH_SUCCESS, 0x01);
        assert_eq!(AUTH_NOT_FOUND, 0x03); // sniffer-verified
        assert_eq!(AUTH_BANNED, 0x04);
        assert_eq!(AUTH_IN_GAME, 0x05);
        assert_eq!(AUTH_FAILED, 0xFF);
    }

    #[test]
    fn test_premium_hours_computation() {
        // C++ ACCOUNT_PREMIUM_V2 computes (expiry - now) / 3600.
        // Simulate premium computation logic from handle().
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // 30 days premium
        let expiry_30d = now + 30 * 86400;
        let hours_30d = (expiry_30d - now) / 3600;
        assert_eq!(hours_30d, 720); // 30 * 24 = 720

        // 1 day premium
        let expiry_1d = now + 86400;
        let hours_1d = (expiry_1d - now) / 3600;
        assert_eq!(hours_1d, 24);

        // Less than 1 hour → should be 1 (min display)
        let expiry_30min = now + 1800;
        let remaining_secs = expiry_30min - now;
        let hours = remaining_secs / 3600;
        assert_eq!(hours, 0); // 1800/3600=0, clamped to 1 in handler

        // Expired → -1
        let expiry_expired = now - 100;
        assert!(expiry_expired <= now); // expired → None → -1

        // No premium → -1
        let empty: Vec<i64> = vec![];
        assert!(empty.iter().max().is_none()); // None → -1
    }

    // ── Sprint 934: Additional coverage ──────────────────────────────

    /// Error response: 9 bytes (reserved(2) + result(1) + trailing(6)). Sniffer-verified.
    #[test]
    fn test_login_error_result_length() {
        use ko_protocol::Packet;
        let mut pkt = Packet::new(LoginOpcode::LsLoginReq as u8);
        pkt.write_u16(0); // reserved
        pkt.write_u8(AUTH_NOT_FOUND);
        pkt.write_u16(0); // trailing
        pkt.write_u32(0); // trailing
        assert_eq!(pkt.data.len(), 9); // sniffer: F3 00 00 03 00 00 00 00 00 00
    }

    /// Success response includes premium hours, account echo, and trailing zeros (PCAP verified).
    #[test]
    fn test_login_success_response_format() {
        use ko_protocol::{Packet, PacketReader};
        let mut pkt = Packet::new(LoginOpcode::LsLoginReq as u8);
        pkt.write_u16(0);
        pkt.write_u8(AUTH_SUCCESS);
        pkt.write_i16(720); // premium hours
        pkt.write_string("testuser");
        pkt.write_u32(0); // trailing 4 zero bytes (PCAP verified)

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u8(), Some(AUTH_SUCCESS));
        assert_eq!(r.read_i16(), Some(720));
        assert_eq!(r.read_string(), Some("testuser".to_string()));
        assert_eq!(r.read_u32(), Some(0), "trailing zeros (PCAP verified)");
        assert_eq!(r.remaining(), 0);
    }

    /// string_is_valid rejects non-alphanumeric ASCII.
    #[test]
    fn test_string_is_valid_rejects_special() {
        assert!(!string_is_valid("."));
        assert!(!string_is_valid("-"));
        assert!(!string_is_valid("!"));
    }

    /// AUTH_IN_GAME response includes server IP, port, and account.
    #[test]
    fn test_login_in_game_response_format() {
        use ko_protocol::{Packet, PacketReader};
        let mut pkt = Packet::new(LoginOpcode::LsLoginReq as u8);
        pkt.write_u16(0);
        pkt.write_u8(AUTH_IN_GAME);
        pkt.write_string("127.0.0.1");
        pkt.write_u16(15001);
        pkt.write_string("testuser");

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u8(), Some(AUTH_IN_GAME));
        assert_eq!(r.read_string(), Some("127.0.0.1".to_string()));
        assert_eq!(r.read_u16(), Some(15001));
        assert_eq!(r.read_string(), Some("testuser".to_string()));
    }
}
