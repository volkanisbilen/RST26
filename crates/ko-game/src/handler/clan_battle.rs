//! WIZ_CLAN_BATTLE (0x63) handler — clan battle notification trigger.
//!
//! S2C-only opcode that notifies the client about clan battle status.
//!
//! ## Client RE (IDA sub_634BA0, v2600)
//!
//! The client's 0x63 handler is minimal:
//! 1. Receives opcode 0x63 (no payload read)
//! 2. Calls `sub_62FBC0(2, 0)` which builds and sends a C2S packet:
//!    `[opcode=0xD0(WIZ_KNIGHTS_PROCESS)][sub=6][type=5][val=2]`
//!    (if val != 2, also appends val as i16 — but here val=2, so skipped)
//! 3. Refreshes the clan battle UI panel (sub_675A00)
//!
//! The server sends this as an empty-payload notification. The client does
//! NOT parse any data from it — it purely triggers a clan battle status
//! refresh request back to the server.
//!
//! ## S2C Packet Format
//!
//! ```text
//! [opcode=0x63]
//! ```
//!
//! No payload. The opcode alone is the notification.
//!
//! ## C2S Response (triggered by client)
//!
//! After receiving 0x63, client sends:
//! ```text
//! [opcode=0xD0(208)] [u8 sub=6] [u8 type=5] [u8 val=2]
//! ```
//! This is handled by the existing WIZ_KNIGHTS_PROCESS handler.

use ko_protocol::{Opcode, Packet};

// ── S2C Builder ─────────────────────────────────────────────────────

/// Build a WIZ_CLAN_BATTLE (0x63) notification packet.
///
/// This is an empty-payload notification that triggers the client to
/// send a WIZ_KNIGHTS_PROCESS sub=6/type=5 request for clan battle
/// status data.
///
/// Wire: `[opcode=0x63]` (no data bytes)
pub fn build_notification() -> Packet {
    Packet::new(Opcode::WizClanBattle as u8)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clan_battle_opcode_value() {
        assert_eq!(Opcode::WizClanBattle as u8, 0x63);
    }

    #[test]
    fn test_build_notification_opcode() {
        let pkt = build_notification();
        assert_eq!(pkt.opcode, Opcode::WizClanBattle as u8);
    }

    #[test]
    fn test_build_notification_empty_payload() {
        let pkt = build_notification();
        assert_eq!(
            pkt.data.len(),
            0,
            "0x63 has no payload — client reads nothing"
        );
    }

    #[test]
    fn test_build_notification_is_s2c_only() {
        // S2C-only: server sends, client responds with WIZ_KNIGHTS_PROCESS.
        // The packet itself carries no data.
        let pkt = build_notification();
        assert_eq!(pkt.opcode, 0x63);
        assert!(pkt.data.is_empty());
    }
}
