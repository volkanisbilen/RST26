//! LS_CRYPTION (0xF2) handler — encryption key exchange + re-key ACK.
//!
//! ## v2600 PCAP-verified behavior
//!
//! This handler serves TWO purposes depending on session state:
//!
//! ### 1. Initial key exchange (AES not yet enabled)
//! Client sends 0xF2 → Server generates AES key, sends it back, enables AES.
//!
//! ### 2. Re-key ACK (AES enabled, session rekeyed)
//! Client sends 0xF2 after receiving the re-key (0xF2 from 0xF6 handler).
//! Server responds with 0xA1 SERVER LIST/REDIRECT.
//!
//! ```text
//! PCAP flow:
//!   C→S 0xF2 (initial) → S→C 0xF2 AES key
//!   ...login...
//!   C→S 0xF6 → S→C 0xF2 RE-KEY (news.rs handler)
//!   C→S 0xF2 (re-key ACK) → S→C 0xA1 SERVER LIST ← THIS HANDLER
//! ```

use ko_db::repositories::server_list::ServerListRepository;
use ko_protocol::{LoginOpcode, Packet};

use crate::login_session::LoginSession;

/// Handle LS_CRYPTION (0xF2) from the client.
pub async fn handle(session: &mut LoginSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.is_rekeyed() {
        // Re-key ACK — client acknowledged the new AES key.
        // Respond with 0xA1 SERVER LIST/REDIRECT.
        return handle_rekey_ack(session).await;
    }

    // Initial key exchange — generate AES key and enable encryption.
    let key = ko_protocol::AesCryption::generate_key();

    // Sniffer verified: [key_len:u8=16] [key:16 bytes]
    let mut response = Packet::new(LoginOpcode::LsCryption as u8);
    response.write_u8(16); // key_len
    response.data.extend_from_slice(&key);

    // Send key BEFORE enabling encryption (this packet is plaintext)
    session.send_packet(&response).await?;

    // Enable AES-only for subsequent packets (PCAP verified).
    session.aes_mut().set_key(key);
    session.aes_mut().enable();

    tracing::info!(
        "[{}] LS 0xF2: AES enabled (key={})",
        session.addr(),
        String::from_utf8_lossy(&key),
    );
    Ok(())
}

/// Handle re-key ACK — send 0xA1 SERVER LIST/REDIRECT.
///
/// PCAP: After client acknowledges the re-key (sends 0xF2),
/// server responds with 0xA1 containing server list data.
///
/// NOTE: The original server sends 14 bytes in 0xA1 (format not yet
/// fully RE'd). We send the full server list format as a first attempt.
/// If the client expects a different format.
async fn handle_rekey_ack(session: &mut LoginSession) -> anyhow::Result<()> {
    let repo = ServerListRepository::new(session.pool());
    let servers = match repo.load_all().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[{}] server_list load_all DB error: {e}", session.addr());
            Vec::new()
        }
    };

    // Send server list with opcode 0xA1 (PCAP: server redirect/list)
    let mut response = Packet::new(LoginOpcode::LsServerRedirect as u8); // 0xA1
    let echo = session.server_index();
    response.write_u16(echo);
    response.write_u8(servers.len() as u8);

    for srv in &servers {
        response.write_string(&srv.lan_ip);
        response.write_string(&srv.server_ip);
        response.write_string(&srv.server_name);
        let user_count = if srv.concurrent_users >= srv.player_cap as i32 {
            -1i16
        } else {
            srv.concurrent_users as i16
        };
        response.write_i16(user_count);
        response.write_i16(srv.server_id);
        response.write_i16(srv.group_id);
        response.write_i16(srv.player_cap);
        response.write_i16(srv.free_player_cap);
        response.write_u8(0); // reserved
        response.write_u8(srv.screen_type as u8);
        response.write_string(&srv.karus_king);
        response.write_string(&srv.karus_notice);
        response.write_string(&srv.elmorad_king);
        response.write_string(&srv.elmorad_notice);
    }

    session.send_packet(&response).await?;

    tracing::info!(
        "[{}] 0xF2 re-key ACK → sent 0xA1 server list ({} servers, echo={})",
        session.addr(),
        servers.len(),
        echo,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use ko_protocol::{LoginOpcode, Packet};

    /// Opcode value is 0xF2.
    #[test]
    fn test_cryption_opcode() {
        assert_eq!(LoginOpcode::LsCryption as u8, 0xF2);
    }

    /// C2S is empty (just opcode, no data).
    #[test]
    fn test_cryption_c2s_empty() {
        let pkt = Packet::new(LoginOpcode::LsCryption as u8);
        assert!(pkt.data.is_empty());
    }

    /// S→C response: [key_len:u8=16] [key:16 bytes] — PCAP verified.
    #[test]
    fn test_cryption_response_format() {
        let key = *b"A4713669LKKWXABQ";
        let mut pkt = Packet::new(LoginOpcode::LsCryption as u8);
        pkt.write_u8(16);
        pkt.data.extend_from_slice(&key);

        assert_eq!(pkt.opcode, 0xF2);
        assert_eq!(pkt.data.len(), 17);
        assert_eq!(pkt.data[0], 16);
        assert_eq!(&pkt.data[1..17], &key);
    }

    /// Wire frame matches PCAP: AA 55 12 00 F2 10 [16B key] 55 AA.
    #[test]
    fn test_cryption_wire_frame() {
        let key = *b"A4713669LKKWXABQ";
        let mut pkt = Packet::new(LoginOpcode::LsCryption as u8);
        pkt.write_u8(16);
        pkt.data.extend_from_slice(&key);

        let frame = pkt.to_outbound_frame();
        assert_eq!(frame.len(), 24);
        assert_eq!(&frame[0..2], &[0xAA, 0x55]);
        assert_eq!(&frame[2..4], &[0x12, 0x00]);
        assert_eq!(frame[4], 0xF2);
        assert_eq!(frame[5], 0x10);
        assert_eq!(&frame[6..22], &key);
        assert_eq!(&frame[22..24], &[0x55, 0xAA]);
    }

    /// Key must be 16 bytes of printable ASCII (0x21..=0x7E).
    #[test]
    fn test_cryption_key_printable() {
        let key = ko_protocol::AesCryption::generate_key();
        for &b in &key {
            assert!((0x21..=0x7E).contains(&b));
        }
    }

    /// 0xA1 opcode value.
    #[test]
    fn test_server_redirect_opcode() {
        assert_eq!(LoginOpcode::LsServerRedirect as u8, 0xA1);
    }

    /// 0xA6 opcode value.
    #[test]
    fn test_server_select_opcode() {
        assert_eq!(LoginOpcode::LsServerSelect as u8, 0xA6);
    }
}
