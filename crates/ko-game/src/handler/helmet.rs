//! WIZ_HELMET (0x87) handler — helmet and cospre visibility toggle.
//! ## Client -> Server
//! `[u8 hide_helmet (bool)] [u8 hide_cospre (bool)]`
//! ## Server -> Region
//! `[u8 hide_helmet] [u8 hide_cospre] [u32 session_id]`
//! Toggles helmet and costume visibility. The server reads the two boolean
//! flags, then broadcasts the state to all nearby players in the region.

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::session::{ClientSession, SessionState};

/// Handle WIZ_HELMET from the client.
pub fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Dead players cannot toggle helmet
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    // Read helmet and cospre hide flags (C++ bool = 1 byte)
    let hide_helmet = reader.read_u8().unwrap_or(0) != 0;
    let hide_cospre = reader.read_u8().unwrap_or(0) != 0;

    // Persist flags in session state for use by item_move broadcast suppression.
    world.update_session(sid, |h| {
        h.is_hiding_helmet = hide_helmet;
        h.is_hiding_cospre = hide_cospre;
    });

    // Build broadcast packet: [u8 hide_helmet] [u8 hide_cospre] [u32 socket_id]
    let result = build_helmet_packet(hide_helmet, hide_cospre, sid);

    // Broadcast to region (3x3 grid)
    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(result),
        None,
        event_room,
    );

    // When hiding costume, send base equipment appearance updates so nearby players
    // see the player's actual armor instead of the costume overlay.
    if hide_cospre {
        const COSPRE_SLOTS: [u8; 4] = [1, 4, 10, 12]; // BREAST, LEG, GLOVE, FOOT
        for &slot in &COSPRE_SLOTS {
            let (item_id, durability) = world
                .get_inventory_slot(sid, slot as usize)
                .map(|s| (s.item_id, s.durability as u16))
                .unwrap_or((0, 0));

            let mut look_pkt = Packet::new(Opcode::WizUserlookChange as u8);
            look_pkt.write_u32(sid as u32);
            look_pkt.write_u8(slot);
            look_pkt.write_u32(item_id);
            look_pkt.write_u16(durability);
            look_pkt.write_u8(0); // reserved

            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(look_pkt),
                None,
                event_room,
            );
        }
    }

    tracing::debug!(
        "[{}] WIZ_HELMET: hide_helmet={}, hide_cospre={}, sid={}",
        session.addr(),
        hide_helmet,
        hide_cospre,
        sid,
    );

    Ok(())
}

/// Build a WIZ_HELMET broadcast packet for testing.
/// Wire: `[u8 hide_helmet] [u8 hide_cospre] [u32 session_id]`
pub fn build_helmet_packet(hide_helmet: bool, hide_cospre: bool, session_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizHelmet as u8);
    pkt.write_u8(hide_helmet as u8);
    pkt.write_u8(hide_cospre as u8);
    pkt.write_u32(session_id as u32);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_helmet_request_format() {
        // Client -> Server: [u8 hide_helmet] [u8 hide_cospre]
        let mut pkt = Packet::new(Opcode::WizHelmet as u8);
        pkt.write_u8(1); // hide helmet
        pkt.write_u8(0); // show cospre

        assert_eq!(pkt.opcode, Opcode::WizHelmet as u8);
        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // hide_helmet = true
        assert_eq!(r.read_u8(), Some(0)); // hide_cospre = false
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_helmet_broadcast_format() {
        // Server -> Region: [u8 hide_helmet] [u8 hide_cospre] [u32 session_id]
        let pkt = build_helmet_packet(true, false, 42);

        assert_eq!(pkt.opcode, Opcode::WizHelmet as u8);
        assert_eq!(pkt.data.len(), 6); // 1 + 1 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // hide_helmet = true
        assert_eq!(r.read_u8(), Some(0)); // hide_cospre = false
        assert_eq!(r.read_u32(), Some(42)); // session_id
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_helmet_both_hidden() {
        let pkt = build_helmet_packet(true, true, 100);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // hide_helmet
        assert_eq!(r.read_u8(), Some(1)); // hide_cospre
        assert_eq!(r.read_u32(), Some(100));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_helmet_both_visible() {
        let pkt = build_helmet_packet(false, false, 1);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0)); // helmet visible
        assert_eq!(r.read_u8(), Some(0)); // cospre visible
        assert_eq!(r.read_u32(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 927: Additional coverage ──────────────────────────────

    /// Broadcast packet is always 6 bytes: hide_helmet(1) + hide_cospre(1) + sid(4).
    #[test]
    fn test_helmet_broadcast_data_length() {
        for (h, c) in [(true, true), (false, false), (true, false), (false, true)] {
            let pkt = build_helmet_packet(h, c, 500);
            assert_eq!(pkt.data.len(), 6);
        }
    }

    /// C2S request is always 2 bytes: hide_helmet(1) + hide_cospre(1).
    #[test]
    fn test_helmet_request_data_length() {
        let mut pkt = Packet::new(Opcode::WizHelmet as u8);
        pkt.write_u8(1);
        pkt.write_u8(1);
        assert_eq!(pkt.data.len(), 2);
    }

    /// Cospre slots: BREAST=1, LEG=4, GLOVE=10, FOOT=12.
    #[test]
    fn test_helmet_cospre_slot_constants() {
        let cospre_slots: [u8; 4] = [1, 4, 10, 12];
        assert_eq!(cospre_slots[0], 1, "BREAST");
        assert_eq!(cospre_slots[1], 4, "LEG");
        assert_eq!(cospre_slots[2], 10, "GLOVE");
        assert_eq!(cospre_slots[3], 12, "FOOT");
    }

    /// UserLookChange packet format: [u32 sid][u8 slot][u32 item_id][u16 durability][u8 reserved].
    #[test]
    fn test_helmet_userlookchange_format() {
        let mut pkt = Packet::new(Opcode::WizUserlookChange as u8);
        pkt.write_u32(42); // sid
        pkt.write_u8(1); // slot (BREAST)
        pkt.write_u32(150001); // item_id
        pkt.write_u16(5000); // durability
        pkt.write_u8(0); // reserved

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(150001));
        assert_eq!(r.read_u16(), Some(5000));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Opcode value is 0x87.
    #[test]
    fn test_helmet_opcode_value() {
        assert_eq!(Opcode::WizHelmet as u8, 0x87);
    }

    // ── Sprint 932: Additional coverage ──────────────────────────────

    /// Opcode from_byte roundtrip.
    #[test]
    fn test_helmet_opcode_from_byte() {
        assert_eq!(Opcode::from_byte(0x87), Some(Opcode::WizHelmet));
    }
}
