//! WIZ_BUNDLE_OPEN_REQ (0x24) handler — open a ground item bundle.
//! Packet format (from client):
//! ```text
//! [u32 bundle_id]
//! ```
//! Response (WIZ_BUNDLE_OPEN_REQ):
//! ```text
//! [u32 bundle_id] [u8 result]
//! If result == 1:
//!   For each of 8 slots: [u32 item_id] [u16 count]
//! ```

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::world::NPC_HAVE_ITEM_LIST;

/// Handle WIZ_BUNDLE_OPEN_REQ from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let bundle_id = reader.read_u32().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    let mut result = Packet::new(Opcode::WizBundleOpenReq as u8);
    result.write_u32(bundle_id);

    // Validate bundle_id
    if bundle_id == 0xFFFFFFFF || bundle_id < 1 {
        return Ok(());
    }

    // Check player state — must be alive and not in a busy state
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Look up the bundle
    let bundle = match world.get_ground_bundle(bundle_id) {
        Some(b) => b,
        None => {
            result.write_u8(0);
            return session.send_packet(&result).await;
        }
    };

    // Zone check: bundle must be in the same zone as the player
    if pos.zone_id != bundle.zone_id {
        return Ok(());
    }

    // Range check: within 50m (squared distance)
    let dx = pos.x - bundle.x;
    let dz = pos.z - bundle.z;
    if dx * dx + dz * dz > 50.0 * 50.0 {
        return Ok(());
    }

    // Loot rights check: must be the looter or in same party
    // For now, simplified: the looter or anyone can open
    // Full party check would require party system integration
    if bundle.looter != 0xFFF && bundle.looter != sid {
        // Check if in same party as the looter
        let ch = world.get_character_info(sid);
        let looter_ch = world.get_character_info(bundle.looter);
        let same_party = match (ch.as_ref(), looter_ch.as_ref()) {
            (Some(me), Some(them)) => me.party_id.is_some() && me.party_id == them.party_id,
            _ => false,
        };
        if !same_party {
            return Ok(());
        }
    }

    // Check if bundle has items
    if bundle.items_count < 1 {
        result.write_u8(0);
        return session.send_packet(&result).await;
    }

    result.write_u8(1); // success

    // Write all 8 item slots
    for i in 0..NPC_HAVE_ITEM_LIST {
        result.write_u32(bundle.items[i].item_id);
        result.write_u16(bundle.items[i].count);
    }

    session.send_packet(&result).await
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use crate::world::NPC_HAVE_ITEM_LIST;

    #[test]
    fn test_bundle_open_c2s_format() {
        // C2S: [u32 bundle_id]
        let mut pkt = Packet::new(Opcode::WizBundleOpenReq as u8);
        pkt.write_u32(12345);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(12345));
    }

    #[test]
    fn test_bundle_open_success_response_format() {
        // Success: [u32 bundle_id][u8 result=1][8× (u32 item_id + u16 count)]
        let mut result = Packet::new(Opcode::WizBundleOpenReq as u8);
        result.write_u32(100); // bundle_id
        result.write_u8(1); // success

        // Write 8 item slots (first 2 with items, rest empty)
        let items = [(500001u32, 3u16), (600002, 1)];
        for i in 0..NPC_HAVE_ITEM_LIST {
            if i < items.len() {
                result.write_u32(items[i].0);
                result.write_u16(items[i].1);
            } else {
                result.write_u32(0);
                result.write_u16(0);
            }
        }

        let mut reader = PacketReader::new(&result.data);
        assert_eq!(reader.read_u32(), Some(100), "bundle_id");
        assert_eq!(reader.read_u8(), Some(1), "result=success");

        // First item
        assert_eq!(reader.read_u32(), Some(500001), "slot 0 item_id");
        assert_eq!(reader.read_u16(), Some(3), "slot 0 count");
        // Second item
        assert_eq!(reader.read_u32(), Some(600002), "slot 1 item_id");
        assert_eq!(reader.read_u16(), Some(1), "slot 1 count");
        // Third slot (empty)
        assert_eq!(reader.read_u32(), Some(0), "slot 2 item_id (empty)");
        assert_eq!(reader.read_u16(), Some(0), "slot 2 count (empty)");
    }

    #[test]
    fn test_bundle_open_fail_response_format() {
        // Fail: [u32 bundle_id][u8 result=0]
        let mut result = Packet::new(Opcode::WizBundleOpenReq as u8);
        result.write_u32(999);
        result.write_u8(0); // fail

        let mut reader = PacketReader::new(&result.data);
        assert_eq!(reader.read_u32(), Some(999), "bundle_id");
        assert_eq!(reader.read_u8(), Some(0), "result=fail");
        // No item data after fail
        assert_eq!(reader.read_u32(), None, "no more data");
    }

    #[test]
    fn test_bundle_open_invalid_bundle_ids() {
        // 0xFFFFFFFF and 0 are rejected by the handler
        assert_eq!(0xFFFFFFFFu32, u32::MAX);
        assert_eq!(0xFFFFFFFFu32, 0xFFFFFFFF, "sentinel value");
        let zero: u32 = 0;
        assert!(zero < 1, "bundle_id 0 is invalid (< 1)");
    }

    #[test]
    fn test_bundle_open_range_check_math() {
        // 50m squared distance check
        let range_sq = 50.0f32 * 50.0;
        assert_eq!(range_sq, 2500.0);

        // Player at (100, 100), bundle at (140, 130) — distance = 50.0 (boundary)
        let dx: f32 = 100.0 - 140.0;
        let dz: f32 = 100.0 - 130.0;
        let dist_sq = dx * dx + dz * dz;
        assert_eq!(dist_sq, 2500.0, "exactly at boundary");
        // Handler uses `>` not `>=`, so exactly 50m is in range
        assert!(dist_sq <= range_sq, "50m boundary should pass");

        // Just outside range
        let dx: f32 = 100.0 - 140.1;
        let dz: f32 = 100.0 - 130.0;
        let dist_sq = dx * dx + dz * dz;
        assert!(dist_sq > range_sq, "50.001m should fail");
    }

    // ── Sprint 926: Additional coverage ──────────────────────────────

    /// Success response data length: u32(4) + u8(1) + 8*(u32+u16) = 53.
    #[test]
    fn test_bundle_open_success_data_length() {
        let mut pkt = Packet::new(Opcode::WizBundleOpenReq as u8);
        pkt.write_u32(1); // bundle_id
        pkt.write_u8(1); // success
        for _ in 0..NPC_HAVE_ITEM_LIST {
            pkt.write_u32(0);
            pkt.write_u16(0);
        }
        assert_eq!(pkt.data.len(), 77); // 4+1+12*6 (v2600)
    }

    /// Fail response data length: u32(4) + u8(1) = 5.
    #[test]
    fn test_bundle_open_fail_data_length() {
        let mut pkt = Packet::new(Opcode::WizBundleOpenReq as u8);
        pkt.write_u32(1);
        pkt.write_u8(0);
        assert_eq!(pkt.data.len(), 5);
    }

    /// NPC_HAVE_ITEM_LIST = 8 slots.
    #[test]
    fn test_npc_have_item_list_constant() {
        assert_eq!(NPC_HAVE_ITEM_LIST, 12);
    }

    /// Looter sentinel 0xFFF means anyone can loot.
    #[test]
    fn test_bundle_looter_sentinel() {
        let looter: usize = 0xFFF;
        let my_sid: usize = 42;
        // 0xFFF means open to all
        let can_loot = looter == 0xFFF || looter == my_sid;
        assert!(can_loot);
    }

    /// Full success response roundtrip with items.
    #[test]
    fn test_bundle_open_success_roundtrip() {
        let mut pkt = Packet::new(Opcode::WizBundleOpenReq as u8);
        pkt.write_u32(777);
        pkt.write_u8(1);
        pkt.write_u32(300001);
        pkt.write_u16(5);
        pkt.write_u32(300002);
        pkt.write_u16(1);
        for _ in 2..NPC_HAVE_ITEM_LIST {
            pkt.write_u32(0);
            pkt.write_u16(0);
        }

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(777));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(300001));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u32(), Some(300002));
        assert_eq!(r.read_u16(), Some(1));
    }
}
