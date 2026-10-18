//! WIZ_CONTINOUS_PACKET_DATA (0x9C) — client resource transfer protocol.
//!
//! v2525-specific opcode (C++ uses 0x44; remapped in v2525). This is a
//! **server-initiated resource download/patching system** — NOT a generic
//! container and NOT usable for ext_hook or game features.
//!
//! ## Client RE Summary
//!
//! Three independent resource subsystems on the client object:
//!
//! | Offset | Sub-opcodes | System |
//! |--------|-------------|--------|
//! | `[esi+0x358]` | 0xF0 | Premium/NPC base texture reload |
//! | `[esi+0x458]` | 0x02, 0x03 | Compressed resource ACK |
//! | `[esi+0x660]` | 0x04-0x09 | File download/patch + chunk data |
//! | `[esi+0xBC0]` | 0x0E | Patch update system |
//!
//! ### Sub-opcode Dispatch (`0x82FA16`)
//!
//! ```text
//! 0xF0: Resource reload — [no data]. Cleanup + reload Premium/NPC_Base textures.
//! 0x02: Compressed resource ACK (match)  — [i32 file_id][i32 checksum]
//! 0x03: Compressed resource ACK (mismatch) — [i32 file_id][i32 checksum]
//! 0x04: File transfer init — [i32 unknown][i32 result_code]
//! 0x05: File transfer result — [i32 file_id][i32 result_code]
//! 0x06: Download data chunks — [i32 seq][i32 result][i32 total][{i32,i32,i32}×N]
//! 0x07: Download completion — [i32 seq][i32 result][i32 idx][i32 size][i32 crc]
//! 0x08: Download finalize — [i32 seq][i32 result]
//! 0x09: File apply/install — [i32 result]
//! 0x0E: Patch manifest — [i32 result][u16 f1][u16 f2][i32 total][u16 count][...]
//! ```
//!
//! ### C2S (Client → Server)
//!
//! Only sent in response to server-initiated downloads:
//! - Sub=0x02: `[0x9C][0x02][i32 file_handle]` — resource download request
//! - Sub=0x06: `[0x9C][0x06]` — download start request
//!
//! ### Why Stub is Correct
//!
//! This is a CDN/patching protocol. The server never initiates downloads
//! for the game server (that's the patch server's job). These C2S packets
//! will never arrive during normal gameplay. If ever needed (GM-initiated
//! texture reload), only sub=0xF0 would be useful — a 1-byte S2C packet.
//!
//! ### String IDs
//!
//! 7045, 10502, 10503, 16810, 16820, 18311, 18631, 33215, 33250,
//! 33607, 33621-33624, 37431-37436, 37444, 37445

use ko_protocol::Packet;
use ko_protocol::PacketReader;
use tracing::debug;

use crate::session::ClientSession;

// ── S2C Builders (only the useful one) ──────────────────────────────────

/// Build S2C resource reload packet (sub=0xF0).
///
/// Wire: `[0x9C][0xF0]`
///
/// Client cleans up and reloads Premium + NPC_Base textures.
/// Potentially useful as a GM command to force texture refresh.
pub fn build_resource_reload() -> Packet {
    let mut pkt = Packet::new(ko_protocol::Opcode::WizContinousPacketData as u8);
    pkt.write_u8(0xF0);
    pkt
}

// ── C2S Handler ─────────────────────────────────────────────────────────

/// Handle WIZ_CONTINOUS_PACKET_DATA (0x9C) C2S — resource transfer protocol.
///
/// Logs and discards. The server does not initiate resource downloads,
/// so C2S responses (sub=0x02, sub=0x06) should never arrive.
pub fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&packet.data);
    let sub = reader.read_u8().unwrap_or(0);
    debug!(
        "[{}] WIZ_CONTINOUS_PACKET_DATA sub=0x{:02X} ({}B remaining)",
        session.addr(),
        sub,
        reader.remaining()
    );
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::Opcode;

    #[test]
    fn test_resource_reload_wire_format() {
        let pkt = build_resource_reload();
        assert_eq!(pkt.opcode, Opcode::WizContinousPacketData as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0xF0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_resource_reload_data_length() {
        // u8(1) = 1 byte
        assert_eq!(build_resource_reload().data.len(), 1);
    }

    #[test]
    fn test_opcode_value() {
        assert_eq!(build_resource_reload().opcode, 0x9C);
    }

    // ── Sprint 928: Additional coverage ──────────────────────────────

    /// Known sub-opcodes: 0xF0, 0x02, 0x03, 0x04-0x09, 0x0E.
    #[test]
    fn test_continuous_sub_opcodes() {
        let known_subs: [u8; 11] = [
            0xF0, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0E, 0xF0,
        ];
        assert!(known_subs.contains(&0xF0), "resource reload");
        assert!(known_subs.contains(&0x02), "compressed ACK match");
        assert!(known_subs.contains(&0x06), "download data chunks");
        assert!(known_subs.contains(&0x0E), "patch manifest");
    }

    /// C2S sub=0x02 format: [0x9C][0x02][i32 file_handle].
    #[test]
    fn test_continuous_c2s_sub02_format() {
        let mut pkt = Packet::new(Opcode::WizContinousPacketData as u8);
        pkt.write_u8(0x02);
        pkt.write_i32(42); // file_handle

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x02));
        assert_eq!(r.read_i32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    /// C2S sub=0x06 format: [0x9C][0x06] (no extra data).
    #[test]
    fn test_continuous_c2s_sub06_format() {
        let mut pkt = Packet::new(Opcode::WizContinousPacketData as u8);
        pkt.write_u8(0x06);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x06));
        assert_eq!(r.remaining(), 0);
    }

    /// Opcode from_byte roundtrip for 0x9C.
    #[test]
    fn test_continuous_opcode_from_byte() {
        assert_eq!(
            Opcode::from_byte(0x9C),
            Some(Opcode::WizContinousPacketData)
        );
        assert_eq!(Opcode::WizContinousPacketData as u8, 0x9C);
    }

    /// Resource reload is exactly 1 byte (sub=0xF0 only).
    #[test]
    fn test_continuous_resource_reload_minimal() {
        let pkt = build_resource_reload();
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 0xF0);
    }

    // ── Sprint 932: Additional coverage ──────────────────────────────

    /// S2C sub=0x04 file transfer init: [u8 sub][i32 unknown][i32 result_code].
    #[test]
    fn test_continuous_s2c_sub04_format() {
        let mut pkt = Packet::new(Opcode::WizContinousPacketData as u8);
        pkt.write_u8(0x04);
        pkt.write_i32(1);
        pkt.write_i32(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x04));
        assert_eq!(r.read_i32(), Some(1));
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Opcode 0x9C is in v2525 dispatch range (0x06-0xD7).
    #[test]
    fn test_continuous_dispatch_range() {
        let op = Opcode::WizContinousPacketData as u8;
        assert!(op >= 0x06 && op <= 0xD7);
    }
}
