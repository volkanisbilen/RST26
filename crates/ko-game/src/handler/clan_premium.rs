//! WIZ_CLAN_PREMIUM (0x5C) handler — clan premium status indicator.
//!
//! S2C-only opcode that notifies the client about a clan's premium status.
//!
//! ## Client RE
//!
//! v2600 dispatch table does NOT have a case 0x5C entry in the main switch
//! (the opcode falls through to default). This matches the C++ server which
//! defines the constant but never sends the packet in the normal flow.
//! The packet format is kept minimal for future use when premium clans are
//! activated.
//!
//! ## S2C Packet Format
//!
//! ```text
//! [u8 sub_opcode] [u8 result]
//! ```
//!
//! Sub-opcodes:
//! - 1 = Query result (response to clan premium status check)
//! - 2 = Status change notification (premium activated/expired)
//!
//! Result values:
//! - 0 = Not premium
//! - 1 = Premium active
//!
//! ## C2S Packets
//!
//! S2C-only — client never sends this opcode.

use ko_protocol::{Opcode, Packet};

// ── S2C Builders ──────────────────────────────────────────────────────

/// Build a WIZ_CLAN_PREMIUM (0x5C) status packet.
///
/// - `sub_opcode`: 1 = query result, 2 = status change
/// - `result`: 0 = not premium, 1 = premium active
///
/// Wire: `[u8 sub_opcode][u8 result]`
pub fn build_clan_premium(sub_opcode: u8, result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizClanPremium as u8);
    pkt.write_u8(sub_opcode);
    pkt.write_u8(result);
    pkt
}

/// Refreshes the clan premium state of every online member.
///
/// The client drops its clan premium display 60 seconds after the last status packet (it then falls
/// back to its "PC cafe" label), so the server repeats the packet every ~25 seconds with the
/// remaining time, and switches it off for everybody when the clan premium runs out.
pub fn tick(world: &crate::world::WorldState) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    for sid in world.collect_sessions_by(|h| h.clan_premium_in_use > 0) {
        let clan_id = world
            .get_character_info(sid)
            .map(|c| c.knights_id)
            .unwrap_or(0);
        let premium_time = world.get_knights(clan_id).map(|k| k.premium_time).unwrap_or(0);
        if clan_id > 0 && premium_time > now {
            let minutes = (premium_time - now) / 60;
            world.send_to_session_owned(sid, super::premium::build_clan_premium_packet(true, minutes.max(1)));
        } else {
            world.update_session(sid, |h| h.clan_premium_in_use = 0);
            world.send_to_session_owned(sid, super::premium::build_clan_premium_packet(false, 0));
        }
    }
}

/// Clan premium sub-opcode: query result.
pub const CLAN_PREMIUM_QUERY: u8 = 1;

/// Clan premium sub-opcode: status change notification.
pub const CLAN_PREMIUM_STATUS_CHANGE: u8 = 2;

/// Clan premium result: not premium.
pub const CLAN_PREMIUM_INACTIVE: u8 = 0;

/// Clan premium result: premium active.
pub const CLAN_PREMIUM_ACTIVE: u8 = 1;

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_build_clan_premium_opcode() {
        let pkt = build_clan_premium(1, 0);
        assert_eq!(pkt.opcode, Opcode::WizClanPremium as u8);
    }

    #[test]
    fn test_build_clan_premium_opcode_value() {
        assert_eq!(Opcode::WizClanPremium as u8, 0x5C);
    }

    #[test]
    fn test_build_clan_premium_query_inactive() {
        let pkt = build_clan_premium(CLAN_PREMIUM_QUERY, CLAN_PREMIUM_INACTIVE);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub = query
        assert_eq!(r.read_u8(), Some(0)); // result = inactive
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_clan_premium_query_active() {
        let pkt = build_clan_premium(CLAN_PREMIUM_QUERY, CLAN_PREMIUM_ACTIVE);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub = query
        assert_eq!(r.read_u8(), Some(1)); // result = active
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_clan_premium_status_change() {
        let pkt = build_clan_premium(CLAN_PREMIUM_STATUS_CHANGE, CLAN_PREMIUM_ACTIVE);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub = status change
        assert_eq!(r.read_u8(), Some(1)); // result = active
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_clan_premium_data_length() {
        // u8 + u8 = 2 bytes
        assert_eq!(build_clan_premium(0, 0).data.len(), 2);
    }

    #[test]
    fn test_build_clan_premium_all_sub_result_combos() {
        for sub in [CLAN_PREMIUM_QUERY, CLAN_PREMIUM_STATUS_CHANGE] {
            for result in [CLAN_PREMIUM_INACTIVE, CLAN_PREMIUM_ACTIVE] {
                let pkt = build_clan_premium(sub, result);
                let mut r = PacketReader::new(&pkt.data);
                assert_eq!(r.read_u8(), Some(sub));
                assert_eq!(r.read_u8(), Some(result));
                assert_eq!(r.remaining(), 0);
            }
        }
    }

    #[test]
    fn test_build_clan_premium_constants() {
        assert_eq!(CLAN_PREMIUM_QUERY, 1);
        assert_eq!(CLAN_PREMIUM_STATUS_CHANGE, 2);
        assert_eq!(CLAN_PREMIUM_INACTIVE, 0);
        assert_eq!(CLAN_PREMIUM_ACTIVE, 1);
    }
}
