//! Login Server packet handler dispatch — routes LS opcodes to handler functions.
//!
//! ## v2600 login flow (C++ echo pattern + PCAP verified)
//!
//! ```text
//! C→S 0xF2  →  S→C 0xF2 AES key (initial key exchange)
//! C→S 0xF3  →  S→C 0xF3 login result
//! C→S 0xF5  →  (store index only, no response — sniffer verified)
//! C→S 0xFD  →  S→C 0xFD OTP sync (echo + u16=0)
//! C→S 0xF6  →  S→C 0xF6 news/notice (echo + "INotice" + text)
//! C→S 0xA6  →  (server select, client connects to game server)
//! ```

pub mod cryption;
pub mod download_info;
pub mod login;
pub mod news;
pub mod ping;
pub mod server_list;
pub mod version;

use ko_protocol::{LoginOpcode, Packet};
use tracing::{debug, warn};

use crate::login_session::LoginSession;

/// Dispatch an incoming packet to the appropriate login handler.
pub async fn dispatch(session: &mut LoginSession, packet: Packet) -> anyhow::Result<()> {
    let opcode = LoginOpcode::from_byte(packet.opcode);

    debug!(
        "[{}] LS received opcode: {:?} (0x{:02X}), data({} bytes)",
        session.addr(),
        opcode,
        packet.opcode,
        packet.data.len(),
    );

    match opcode {
        Some(LoginOpcode::LsHandshake) => {
            // v2600: 0x51 = first packet from game client (account + version_int).
            // Parse and log client version, then respond with 0xFA (crypto key).
            //
            // v2600 RE (deep_login_flow.md Phase 1-2):
            // - Client sends: [0x51][u16 account_len][account][u32 version_int]
            // - Server responds with: [0xFA][i16 sub_opcode=0x01][u8 key_len=0]
            // - Client receives 0xFA sub 0x01 → sets crypto flag, shows login UI, sends 0xF2
            let mut reader = ko_protocol::PacketReader::new(&packet.data);
            let account = reader.read_string().unwrap_or_default();
            let client_version = reader.read_u32().unwrap_or(0);
            tracing::info!(
                "[{}] Handshake 0x51: account={}, client_version={}",
                session.addr(),
                account,
                client_version
            );

            let mut response = Packet::new(LoginOpcode::LsOtp as u8); // 0xFA
            response.write_i16(1); // sub-opcode 0x01 (primary key delivery)
            response.write_u8(0); // key_len = 0 (no encryption key)
            session.send_packet(&response).await
        }
        Some(LoginOpcode::LsVersionReq) => version::handle(session, packet).await,
        Some(LoginOpcode::LsCryption) => cryption::handle(session, packet).await,
        Some(LoginOpcode::LsLoginReq) => login::handle(session, packet).await,

        // ── Server selection (C++ echo pattern) ───────────────────────
        // 0xF5 → store index only, no response (sniffer verified)
        Some(LoginOpcode::LsServerList) => server_list::handle(session, packet).await,

        // 0xF6 → echo 0xF6 + "INotice" + notice text (C++ HandleNews, LS_NEWS=0xF6)
        Some(LoginOpcode::LsVersionCheck) => news::handle(session, packet).await,

        // 0xFD → echo 0xFD + u16(0) (C++ HandleOTPSync, LS_OTP_SYNC=0xFD)
        Some(LoginOpcode::LsNews) => {
            // Non-authenticated 0xFD = stale connection from login failure.
            if !session.aes_enabled() {
                tracing::info!(
                    "[{}] LS 0xFD on non-auth connection — closing to unblock retry",
                    session.addr(),
                );
                anyhow::bail!("0xFD on non-auth connection — close to unblock retry");
            }

            // C++ HandleOTPSync: Packet result(pkt.GetOpcode()); result << uint16(0);
            let mut response = Packet::new(packet.opcode); // echo 0xFD
            response.write_u16(0);
            session.send_packet(&response).await?;

            tracing::info!("[{}] 0xFD → echo OTP sync (u16=0)", session.addr());
            Ok(())
        }

        // 0xA6 → server select (client will connect to game server)
        Some(LoginOpcode::LsServerSelect) => {
            tracing::info!(
                "[{}] 0xA6 server select received ({} bytes)",
                session.addr(),
                packet.data.len(),
            );

            // PCAP session 4 seq 7: After 0xA6, server disables AES and
            // drops back to PLAINTEXT. All subsequent ping/download packets
            // are plaintext (crypto_algo=NONE).
            session.aes_mut().disable();

            // PCAP session 4 seq 7 S→C: Server responds with PING (version=2599)
            let mut ping = Packet::new(LoginOpcode::LsVersionReq as u8); // 0x01
            ping.write_u16(session.config().version);
            session.send_packet(&ping).await?;

            tracing::info!(
                "[{}] 0xA6 → AES disabled, sent ping (v{}), connection stays for launcher",
                session.addr(),
                session.config().version,
            );
            Ok(())
        }

        // 0xA1 is S→C only, should never come from client
        Some(LoginOpcode::LsServerRedirect) => {
            warn!(
                "[{}] Unexpected C→S 0xA1 (server redirect is S→C only)",
                session.addr(),
            );
            Ok(())
        }

        // ── Other handlers ─────────────────────────────────────────────
        Some(LoginOpcode::LsKoreakoLauncherPing) => ping::handle(session, packet).await,
        Some(LoginOpcode::LsUnkF7) => {
            let mut response = Packet::new(packet.opcode);
            response.write_u16(0);
            session.send_packet(&response).await
        }
        Some(LoginOpcode::LsDownloadInfoReq) => download_info::handle(session, packet).await,
        Some(LoginOpcode::LsPasswordLogin) => {
            tracing::info!(
                "[{}] LS 0xEA password_login: data({} bytes) = {:02X?} — closing connection",
                session.addr(),
                packet.data.len(),
                &packet.data[..packet.data.len().min(16)],
            );
            anyhow::bail!("0xEA handled — close connection to unblock client retry");
        }
        Some(LoginOpcode::LsOtp) => {
            debug!(
                "[{}] Stub LS opcode: 0x{:02X} (OTP)",
                session.addr(),
                packet.opcode
            );
            Ok(())
        }
        _ => {
            warn!(
                "[{}] Unhandled LS opcode: 0x{:02X}",
                session.addr(),
                packet.opcode
            );
            Ok(())
        }
    }
}
