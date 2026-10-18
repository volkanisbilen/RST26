//! LS_VERSION_REQ (0x01) handler — version check for the launcher.
//! ## Request (Client → Server)
//! Empty (just opcode).
//! ## Response (Server → Client)
//! | Offset | Type  | Description               |
//! |--------|-------|---------------------------|
//! | 0      | u16le | Launcher patch version (2625) |

use ko_protocol::{LoginOpcode, Packet};

use crate::login_session::LoginSession;

/// Handle LS_VERSION_REQ from the launcher.
pub async fn handle(session: &mut LoginSession, _pkt: Packet) -> anyhow::Result<()> {
    let mut response = Packet::new(LoginOpcode::LsVersionReq as u8);
    response.write_u16(session.config().version);
    session.send_packet(&response).await?;

    tracing::info!(
        "[{}] Sent version: {}",
        session.addr(),
        session.config().version
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use ko_protocol::{LoginOpcode, Packet, PacketReader};

    #[test]
    fn test_version_response_format() {
        // Response: [u16le version]
        let version: u16 = 2625;
        let mut response = Packet::new(LoginOpcode::LsVersionReq as u8);
        response.write_u16(version);

        assert_eq!(response.opcode, LoginOpcode::LsVersionReq as u8);
        let mut reader = PacketReader::new(&response.data);
        assert_eq!(reader.read_u16(), Some(2625));
    }

    #[test]
    fn test_version_opcode_value() {
        assert_eq!(LoginOpcode::LsVersionReq as u8, 0x01);
    }

    #[test]
    fn test_version_various_values() {
        // Different server versions should all serialize correctly
        for ver in [1, 2369, 2525, 2625, u16::MAX] {
            let mut pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
            pkt.write_u16(ver);

            let mut reader = PacketReader::new(&pkt.data);
            assert_eq!(reader.read_u16(), Some(ver), "version {ver} roundtrip");
        }
    }

    /// Response is exactly 2 bytes (u16).
    #[test]
    fn test_version_response_length() {
        let mut pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        pkt.write_u16(2625);
        assert_eq!(pkt.data.len(), 2);
    }

    /// Version 2625 LE byte order: [0x41, 0x0A].
    #[test]
    fn test_version_le_byte_order() {
        let mut pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        pkt.write_u16(2625);
        assert_eq!(pkt.data[0], 0x41); // 2625 & 0xFF
        assert_eq!(pkt.data[1], 0x0A); // 2625 >> 8
    }

    // ── Sprint 940: Additional coverage ──────────────────────────────

    /// C2S is empty (just opcode).
    #[test]
    fn test_version_c2s_empty() {
        let pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        assert!(pkt.data.is_empty());
    }

    /// Version zero roundtrip.
    #[test]
    fn test_version_zero() {
        let mut pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        pkt.write_u16(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(0));
    }

    /// Reader exhausted after reading version.
    #[test]
    fn test_version_reader_exhausted() {
        let mut pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        pkt.write_u16(2625);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u16();
        assert_eq!(r.remaining(), 0);
    }

    /// Version 2369 (old version) LE bytes.
    #[test]
    fn test_version_2369_le() {
        let mut pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        pkt.write_u16(2369);
        // 2369 = 0x0941 → LE: [0x41, 0x09]
        assert_eq!(pkt.data[0], 0x41);
        assert_eq!(pkt.data[1], 0x09);
    }

    /// Packet opcode matches LsVersionReq.
    #[test]
    fn test_version_packet_opcode() {
        let pkt = Packet::new(LoginOpcode::LsVersionReq as u8);
        assert_eq!(pkt.opcode, 0x01);
    }
}
