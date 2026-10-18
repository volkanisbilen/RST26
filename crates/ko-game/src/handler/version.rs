//! Game server version check handler.
//!
//! ## Wire Format Test Handler
//!
//! This handler replies to the GameServer-side WIZ_VERSION_CHECK packet.
//! The 26xx client currently reaches this point with:
//!
//! Client -> Server:
//! opcode=0x2B, data=[FF FF]
//!
//! We keep the historical response modes behind KO_VERSION_MODE for protocol
//! diagnostics. Normal operation uses mode 99 and the DB-backed version.
//!
//! Test modes:
//! 0 -> [00][2599][10][key][00]
//! 1 -> [00][2599][10][key][00]
//! 2 -> [00][2602][10][key][00]
//! 3 -> [00][2602][10][key][00]
//! 4 -> [2599][10][key][00]
//! 5 -> [2602][10][key][00]
//! 6 -> [00][2599][10][key] no trailer
//! 7 -> [00][2602][10][key] no trailer

use ko_protocol::{Opcode, Packet};

use crate::session::{ClientSession, SessionState};

/// Fallback version if server_settings is not loaded yet.
///
/// v2625 `sub_7AE420` returns 2625 and `sub_7B5420` rejects any other
/// GameServer wire version.
pub const DEFAULT_SERVER_VERSION: u16 = 2625;

/// Resolve the game version from DB: server_settings.game_version.
fn resolve_version(session: &ClientSession) -> u16 {
    session
        .world()
        .get_server_settings()
        .map(|s| s.game_version as u16)
        .unwrap_or(DEFAULT_SERVER_VERSION)
}

/// Read KO_VERSION_MODE from environment.
///
/// Default mode is 99:
/// [00][server_settings.game_version][10][key][00]
fn get_version_mode() -> u8 {
    std::env::var("KO_VERSION_MODE")
        .ok()
        .and_then(|v| v.trim().parse::<u8>().ok())
        .unwrap_or(99)
}

/// Return the wire version used by the selected test mode.
fn mode_wire_version(mode: u8, db_version: u16) -> u16 {
    match mode {
        0 | 1 | 4 | 6 => 2599,
        2 | 3 | 5 | 7 => 2602,
        _ => db_version,
    }
}

/// Human-readable mode description.
fn mode_description(mode: u8) -> &'static str {
    match mode {
        0 => "[00][2599][10][key][00]",
        1 => "[00][2599][10][key][00]",
        2 => "[00][2602][10][key][00]",
        3 => "[00][2602][10][key][00]",
        4 => "[2599][10][key][00]",
        5 => "[2602][10][key][00]",
        6 => "[00][2599][10][key] no trailer",
        7 => "[00][2602][10][key] no trailer",
        _ => "[00][db_version][10][key][00] fallback",
    }
}

/// Build 0x2B version response payload according to KO_VERSION_MODE.
/// The leading byte selects the cloak catalog, NOT a success status.
/// v2625 sub_7B5420: zero loads Cloak.tbl; nonzero loads Cloak_PVP.tbl.
/// The shipped PVP table has a legacy 7-column layout rejected by sub_52A840.
fn build_version_response_payload(mode: u8, db_version: u16, key: &[u8; 16]) -> Packet {
    let mut response = Packet::new(Opcode::WizVersionCheck as u8);

    match mode {
        // [00][2599][10][key][00]
        0 => {
            response.write_u8(0);
            response.write_u16(2599);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }

        // [00][2599][10][key][00]
        1 => {
            response.write_u8(0);
            response.write_u16(2599);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }

        // [00][2602][10][key][00]
        2 => {
            response.write_u8(0);
            response.write_u16(2602);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }

        // [00][2602][10][key][00]
        3 => {
            response.write_u8(0);
            response.write_u16(2602);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }

        // [2599][10][key][00]
        4 => {
            response.write_u16(2599);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }

        // [2602][10][key][00]
        5 => {
            response.write_u16(2602);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }

        // [00][2599][10][key] no trailer
        6 => {
            response.write_u8(0);
            response.write_u16(2599);
            response.write_u8(16);
            response.write_bytes(key);
        }

        // [00][2602][10][key] no trailer
        7 => {
            response.write_u8(0);
            response.write_u16(2602);
            response.write_u8(16);
            response.write_bytes(key);
        }

        // Fallback: [00][db_version][10][key][00]
        _ => {
            response.write_u8(0);
            response.write_u16(db_version);
            response.write_u8(16);
            response.write_bytes(key);
            response.write_u8(0);
        }
    }

    response
}

/// Handle WIZ_VERSION_CHECK from the GameServer client.
///
/// Important:
/// This packet itself is sent plaintext/direct.
/// AES is enabled only after this response is sent.
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::Connected {
        return Ok(());
    }

    let db_version = resolve_version(session);
    let mode = get_version_mode();
    let wire_version = mode_wire_version(mode, db_version);
    let mode_desc = mode_description(mode);

    let key = ko_protocol::aes_crypt::AesCryption::generate_key();

    let response = build_version_response_payload(mode, db_version, &key);

    session.send_packet(&response).await?;

    session.aes_mut().set_key(key);
    session.aes_mut().enable();

    session.set_state(SessionState::VersionChecked);

    tracing::warn!(
        "[{}] Version mode test: KO_VERSION_MODE={} db_version={} wire_version={} desc={} response_len={} AES enabled key={:02X?}",
        session.addr(),
        mode,
        db_version,
        wire_version,
        mode_desc,
        response.data.len(),
        &key,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_version_response_payload, mode_description, mode_wire_version, DEFAULT_SERVER_VERSION,
    };
    use ko_protocol::aes_crypt::AesCryption;
    use ko_protocol::{Opcode, PacketReader};

    #[test]
    fn test_version_check_opcode() {
        assert_eq!(Opcode::WizVersionCheck as u8, 0x2B);
    }

    #[test]
    fn test_default_server_version() {
        assert_eq!(DEFAULT_SERVER_VERSION, 2625);
    }

    #[test]
    fn test_mode_description() {
        assert_eq!(mode_description(3), "[00][2602][10][key][00]");
        assert_eq!(mode_description(7), "[00][2602][10][key] no trailer");
    }

    #[test]
    fn test_mode_wire_version() {
        assert_eq!(mode_wire_version(0, 2602), 2599);
        assert_eq!(mode_wire_version(1, 2602), 2599);
        assert_eq!(mode_wire_version(2, 2599), 2602);
        assert_eq!(mode_wire_version(3, 2599), 2602);
        assert_eq!(mode_wire_version(7, 2599), 2602);
        assert_eq!(mode_wire_version(99, 2625), 2625);
    }

    #[test]
    fn test_v2625_db_response_with_aes_key() {
        let key = [0x41u8; 16];
        let pkt = build_version_response_payload(99, 2625, &key);

        assert_eq!(pkt.opcode, Opcode::WizVersionCheck as u8);
        assert_eq!(pkt.data.len(), 21);
        assert_eq!(&pkt.data[..4], &[0x00, 0x41, 0x0A, 0x10]);
        assert_eq!(&pkt.data[4..20], &key);
        assert_eq!(pkt.data[20], 0x00);
    }

    #[test]
    fn test_mode_3_response_with_aes_key() {
        let key = [0x41u8; 16];
        let pkt = build_version_response_payload(3, 2602, &key);

        assert_eq!(pkt.opcode, Opcode::WizVersionCheck as u8);
        assert_eq!(pkt.data.len(), 21);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(2602));
        assert_eq!(r.read_u8(), Some(16));

        let mut read_key = [0u8; 16];
        for byte in &mut read_key {
            *byte = r.read_u8().expect("key byte");
        }

        assert_eq!(read_key, key);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_mode_7_response_without_trailer() {
        let key = [0x41u8; 16];
        let pkt = build_version_response_payload(7, 2602, &key);

        assert_eq!(pkt.opcode, Opcode::WizVersionCheck as u8);
        assert_eq!(pkt.data.len(), 20);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(2602));
        assert_eq!(r.read_u8(), Some(16));

        let mut read_key = [0u8; 16];
        for byte in &mut read_key {
            *byte = r.read_u8().expect("key byte");
        }

        assert_eq!(read_key, key);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn all_prefixed_modes_select_normal_cloak_catalog() {
        let key = [0x42; 16];
        for mode in [0, 1, 2, 3, 6, 7, 99] {
            let pkt = build_version_response_payload(mode, 2614, &key);
            let mut expected = vec![0];
            expected.extend_from_slice(&mode_wire_version(mode, 2614).to_le_bytes());
            expected.push(16);
            expected.extend_from_slice(&key);
            if mode != 6 && mode != 7 {
                expected.push(0);
            }
            assert_eq!(pkt.data.as_slice(), expected.as_slice(), "mode {mode}");
        }
    }

    #[test]
    fn test_aes_key_generation() {
        let key = AesCryption::generate_key();
        assert_eq!(key.len(), 16);

        for &b in &key {
            assert!((0x21..=0x7E).contains(&b));
        }
    }
}
