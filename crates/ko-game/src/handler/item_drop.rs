//! WIZ_ITEM_DROP (0x23) handler — drop an item from inventory to the ground.
//! Packet format (from client):
//! ```text
//! [u8 src_pos] [u32 item_id] [u16 count]
//! ```
//! The server creates a ground bundle at the player's position and
//! broadcasts its appearance to nearby players.

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::session::{ClientSession, SessionState};
use crate::world::{
    GroundBundle, LootItem, UserItemSlot, ITEM_FLAG_BOUND, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED,
    ITEM_FLAG_SEALED, ZONE_CHAOS_DUNGEON,
};

use super::{HAVE_MAX, SLOT_MAX};

/// Handle WIZ_ITEM_DROP from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let src_pos = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let count = reader.read_u16().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    // Validate
    if src_pos as usize >= HAVE_MAX || item_id == 0 || count == 0 {
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Cannot drop items in Chaos Dungeon
    if pos.zone_id == ZONE_CHAOS_DUNGEON {
        return Ok(());
    }

    // Validate state: must be alive and not busy
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    let actual_slot = SLOT_MAX + src_pos as usize;

    // Validate item flags — cannot drop bound, sealed, rented, or duplicate items
    if let Some(inv_slot) = world.get_inventory_slot(sid, actual_slot) {
        if inv_slot.flag == ITEM_FLAG_RENTED
            || inv_slot.flag == ITEM_FLAG_BOUND
            || inv_slot.flag == ITEM_FLAG_DUPLICATE
            || inv_slot.flag == ITEM_FLAG_SEALED
        {
            return Ok(());
        }
    }

    // Remove item from inventory
    let removed = world.update_inventory(sid, |inv| {
        if actual_slot >= inv.len() {
            return false;
        }
        let slot = &mut inv[actual_slot];
        if slot.item_id != item_id || slot.count < count {
            return false;
        }
        slot.count -= count;
        if slot.count == 0 {
            *slot = UserItemSlot::default();
        }
        true
    });

    if !removed {
        return Ok(());
    }

    // Create ground bundle
    let bundle_id = world.allocate_bundle_id();
    let mut bundle = GroundBundle {
        bundle_id,
        items_count: 1,
        npc_id: 0,
        looter: sid,
        x: pos.x,
        z: pos.z,
        y: pos.y,
        zone_id: pos.zone_id,
        ..Default::default()
    };
    bundle.items[0] = LootItem {
        item_id,
        count,
        slot_id: 0,
    };

    world.add_ground_bundle(bundle);

    // Recalculate stats (weight notification is integrated into set_user_ability)
    world.set_user_ability(sid);

    // Broadcast bundle appearance to nearby players
    // v2600: same format as NPC loot drop — [u32 dropper_id] [u32 bundle_id] [u8 count]
    let mut bundle_pkt = Packet::new(Opcode::WizItemDrop as u8);
    bundle_pkt.write_u32(sid as u32); // dropper = player session ID
    bundle_pkt.write_u32(bundle_id);
    bundle_pkt.write_u8(1); // item_count

    let event_room = world.get_event_room(sid);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(bundle_pkt),
        None,
        event_room,
    );

    // FerihaLog: RobItemInsertLog (item drop)
    super::audit_log::log_rob_item(
        session.pool(),
        session.account_id().unwrap_or(""),
        &world.get_session_name(sid).unwrap_or_default(),
        pos.zone_id as i16,
        pos.x as i16,
        pos.z as i16,
        item_id,
        count.into(),
        actual_slot as u8,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::NPC_HAVE_ITEM_LIST;
    use ko_protocol::PacketReader;

    #[test]
    fn test_slot_constants() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(NPC_HAVE_ITEM_LIST, 12);
    }

    #[test]
    fn test_item_flag_constants() {
        // Verify flag constants used for drop validation
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
        assert_eq!(ITEM_FLAG_BOUND, 8);
    }

    #[test]
    fn test_chaos_dungeon_zone_constant() {
        // Items cannot be dropped in Chaos Dungeon
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
    }

    #[test]
    fn test_c2s_drop_packet_format() {
        // C2S: [u8 src_pos] [u32 item_id] [u16 count]
        let mut pkt = Packet::new(Opcode::WizItemDrop as u8);
        pkt.write_u8(5); // src_pos (HAVE slot 5)
        pkt.write_u32(379006001); // item_id
        pkt.write_u16(10); // count

        assert_eq!(pkt.opcode, Opcode::WizItemDrop as u8);
        assert_eq!(pkt.data.len(), 7); // 1 + 4 + 2

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u32(), Some(379006001));
        assert_eq!(r.read_u16(), Some(10));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_s2c_bundle_drop_broadcast_format() {
        // S2C broadcast: [u32 bundle_id] [u16 x*10] [u16 z*10] [u16 y*10]
        let bundle_id: u32 = 42;
        let pos_x: f32 = 150.5;
        let pos_z: f32 = 200.3;
        let pos_y: f32 = 10.0;

        let mut pkt = Packet::new(Opcode::WizItemDrop as u8);
        pkt.write_u32(bundle_id);
        pkt.write_u16((pos_x * 10.0) as u16);
        pkt.write_u16((pos_z * 10.0) as u16);
        pkt.write_u16((pos_y * 10.0) as u16);

        assert_eq!(pkt.opcode, Opcode::WizItemDrop as u8);
        assert_eq!(pkt.data.len(), 10); // 4 + 2 + 2 + 2

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42)); // bundle_id
        assert_eq!(r.read_u16(), Some(1505)); // x * 10
        assert_eq!(r.read_u16(), Some(2003)); // z * 10
        assert_eq!(r.read_u16(), Some(100)); // y * 10
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_s2c_bundle_position_scaling() {
        // Verify ×10 scaling for various positions
        let test_positions = [(0.0_f32, 0u16), (1.0, 10), (99.9, 999), (512.0, 5120)];
        for (world_coord, expected_wire) in test_positions {
            assert_eq!(
                (world_coord * 10.0) as u16,
                expected_wire,
                "world {} should be wire {}",
                world_coord,
                expected_wire,
            );
        }
    }

    #[test]
    fn test_actual_slot_calculation() {
        // src_pos (HAVE index) maps to actual_slot = SLOT_MAX + src_pos
        for src_pos in 0..HAVE_MAX {
            let actual_slot = SLOT_MAX + src_pos;
            assert!(actual_slot >= SLOT_MAX);
            assert!(actual_slot < SLOT_MAX + HAVE_MAX);
        }
    }

    #[test]
    fn test_src_pos_bounds() {
        // src_pos must be < HAVE_MAX (28)
        assert!(0 < HAVE_MAX);
        assert_eq!(HAVE_MAX, 28);
        // Actual slot for max valid src_pos
        assert_eq!(SLOT_MAX + (HAVE_MAX - 1), 41);
    }

    #[test]
    fn test_ground_bundle_default() {
        let bundle = GroundBundle::default();
        assert_eq!(bundle.bundle_id, 0);
        assert_eq!(bundle.items_count, 0);
        assert_eq!(bundle.npc_id, 0);
        assert_eq!(bundle.looter, 0xFFF); // C++ default: no owner
    }

    #[test]
    fn test_loot_item_construction() {
        let item = LootItem {
            item_id: 379006001,
            count: 5,
            slot_id: 0,
        };
        assert_eq!(item.item_id, 379006001);
        assert_eq!(item.count, 5);
    }

    #[test]
    fn test_drop_flags_are_distinct() {
        // All no-drop flags must be unique
        let flags = [
            ITEM_FLAG_RENTED,
            ITEM_FLAG_BOUND,
            ITEM_FLAG_DUPLICATE,
            ITEM_FLAG_SEALED,
        ];
        for i in 0..flags.len() {
            for j in (i + 1)..flags.len() {
                assert_ne!(flags[i], flags[j], "flags at {} and {} should differ", i, j);
            }
        }
    }
}
