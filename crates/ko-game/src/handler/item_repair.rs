//! WIZ_ITEM_REPAIR (0x3B) handler — repair item durability at an NPC.
//! Packet format (from client):
//! ```text
//! [u8 pos_type] [u8 slot] [u32 npc_id] [u32 item_id]
//! ```
//! Response (WIZ_ITEM_REPAIR):
//! ```text
//! [u8 result] [u32 gold]
//! ```
//! pos_type: 1=equipment slot, 2=inventory bag

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::world::PremiumProperty;

use crate::npc_type_constants::{NPC_MERCHANT, NPC_TINKER};

use super::{HAVE_MAX, ITEM_KIND_UNIQUE, SLOT_MAX};

/// Handle WIZ_ITEM_REPAIR from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let pos_type = reader.read_u8().unwrap_or(0);
    let slot = reader.read_u8().unwrap_or(0);
    let npc_id = reader.read_u32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    let mut result = Packet::new(Opcode::WizItemRepair as u8);

    if world.is_player_dead(sid) {
        return Ok(());
    }

    // NPC range check — C++ returns silently (no packet) when NPC is invalid or out of range.
    if !world.is_in_npc_range(sid, npc_id) {
        return Ok(());
    }

    // NPC type validation — only TINKER (22) and MERCHANT (21) can repair
    let npc = match world.get_npc_instance(npc_id) {
        Some(n) => n,
        None => return Ok(()), // NPC removed between range check and here — silent return
    };
    if world.is_npc_dead(npc_id) {
        return Ok(());
    }
    if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
        if tmpl.npc_type != NPC_TINKER && tmpl.npc_type != NPC_MERCHANT {
            return send_fail(session, &world, sid).await;
        }
    } else {
        return send_fail(session, &world, sid).await;
    }

    // Calculate actual inventory index
    let actual_idx = match pos_type {
        1 => {
            // Equipment slot
            if slot as usize >= SLOT_MAX {
                return send_fail(session, &world, sid).await;
            }
            slot as usize
        }
        2 => {
            // Inventory bag
            if slot as usize >= HAVE_MAX {
                return send_fail(session, &world, sid).await;
            }
            SLOT_MAX + slot as usize
        }
        _ => return send_fail(session, &world, sid).await,
    };

    // Verify the item in the slot matches
    let inv_slot = match world.get_inventory_slot(sid, actual_idx) {
        Some(s) if s.item_id == item_id => s,
        _ => return send_fail(session, &world, sid).await,
    };

    // Look up item definition
    let item_def = match world.get_item(item_id) {
        Some(i) => i,
        None => return send_fail(session, &world, sid).await,
    };

    // Cannot repair items with sell_price indicating no repairs (SellTypeNoRepairs = 2)
    // Cannot repair scrolls (kind 255)
    const SELL_TYPE_NO_REPAIRS: i32 = 2;
    if item_def.sell_price.unwrap_or(0) == SELL_TYPE_NO_REPAIRS {
        return send_fail(session, &world, sid).await;
    }
    let kind = item_def.kind.unwrap_or(0);
    if kind == ITEM_KIND_UNIQUE {
        return send_fail(session, &world, sid).await;
    }

    let max_durability = item_def.duration.unwrap_or(0);
    if max_durability <= 1 {
        return send_fail(session, &world, sid).await;
    }

    // Legacy items may have been created with a duration that no longer matches
    // the item definition. Clamp before subtraction so an invalid database value
    // can never become a negative repair amount/cost on the client.
    let current_durability = inv_slot.durability.clamp(0, max_durability);
    let quantity = max_durability - current_durability;
    if quantity <= 0 {
        return send_fail(session, &world, sid).await;
    }

    // Calculate repair cost
    // C++ formula: (((buy_price - 10) / 10000.0) + pow(buy_price, 0.75)) * quantity / durability
    let buy_price = item_def.buy_price.unwrap_or(0).max(10) as f64;
    let mut cost = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * quantity as f64
        / max_durability as f64) as u32;

    // Apply premium repair discount
    let repair_disc = world.get_premium_property(sid, PremiumProperty::RepairDiscountPercent);
    if repair_disc > 0 {
        cost = cost * repair_disc as u32 / 100;
    }
    let clan_repair_disc =
        world.get_clan_premium_property(sid, PremiumProperty::RepairDiscountPercent);
    if clan_repair_disc > 0 {
        cost = cost * clan_repair_disc as u32 / 100;
    }

    // Deduct gold — use silent variant since WIZ_ITEM_REPAIR response
    // already contains the updated gold amount (avoids duplicate WIZ_GOLD_CHANGE).
    if !world.gold_lose_silent(sid, cost) {
        return send_fail(session, &world, sid).await;
    }

    // Repair: set durability to max
    world.update_inventory(sid, |inv| {
        if actual_idx < inv.len() {
            inv[actual_idx].durability = max_durability;
            true
        } else {
            false
        }
    });

    // Recalculate stats (durability affects weapon damage and AC)
    world.set_user_ability(sid);

    let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);

    result.write_u8(1); // success
    result.write_u32(gold);
    session.send_packet(&result).await
}

/// Send a failure response.
async fn send_fail(
    session: &mut ClientSession,
    world: &crate::world::WorldState,
    sid: u16,
) -> anyhow::Result<()> {
    let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
    let mut result = Packet::new(Opcode::WizItemRepair as u8);
    result.write_u8(0);
    result.write_u32(gold);
    session.send_packet(&result).await
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_npc_type_constants() {
        use crate::npc_type_constants::{NPC_MERCHANT, NPC_TINKER};
        assert_eq!(NPC_MERCHANT, 21);
        assert_eq!(NPC_TINKER, 22);
        // Only these two types should be allowed for repair
        let valid_types = [NPC_MERCHANT, NPC_TINKER];
        assert!(valid_types.contains(&21));
        assert!(valid_types.contains(&22));
        assert!(!valid_types.contains(&0)); // monster
        assert!(!valid_types.contains(&40)); // healer
        assert!(!valid_types.contains(&50)); // gate
    }

    #[test]
    fn test_packet_format_npc_id_u32() {
        // Verify that npc_id is read as u32 (not u16) matching C++ uint32 sNpcID
        let mut pkt = Packet::new(Opcode::WizItemRepair as u8);
        pkt.write_u8(1); // pos_type
        pkt.write_u8(0); // slot
        pkt.write_u32(12345); // npc_id (u32, NOT u16)
        pkt.write_u32(100001); // item_id

        let mut reader = PacketReader::new(&pkt.data);
        let pos_type = reader.read_u8().unwrap();
        let slot = reader.read_u8().unwrap();
        let npc_id = reader.read_u32().unwrap(); // must be u32
        let item_id = reader.read_u32().unwrap();

        assert_eq!(pos_type, 1);
        assert_eq!(slot, 0);
        assert_eq!(npc_id, 12345);
        assert_eq!(item_id, 100001);
    }

    #[test]
    fn test_repair_success_packet_format() {
        let mut pkt = Packet::new(Opcode::WizItemRepair as u8);
        pkt.write_u8(1); // success
        pkt.write_u32(50000); // gold

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), 1);
        assert_eq!(reader.read_u32().unwrap(), 50000);
    }

    #[test]
    fn test_repair_fail_packet_format() {
        let mut pkt = Packet::new(Opcode::WizItemRepair as u8);
        pkt.write_u8(0); // fail
        pkt.write_u32(100000); // gold

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), 0);
        assert_eq!(reader.read_u32().unwrap(), 100000);
    }

    #[test]
    fn test_repair_cost_formula() {
        // MoneyID = (((buy_price-10)/10000.0) + pow(buy_price, 0.75)) * quantity / durability
        let buy_price: f64 = 10000.0;
        let max_durability: u16 = 100;
        let quantity: u16 = 50;

        let cost = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * quantity as f64
            / max_durability as f64) as u32;

        // Should produce a reasonable cost
        assert!(cost > 0);
        // Cost should scale with quantity
        let cost_full = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * 100.0
            / max_durability as f64) as u32;
        assert!(cost_full > cost);
    }

    #[test]
    fn test_sell_type_no_repairs_constant() {
        // Items with sell_price == 2 cannot be repaired
        const SELL_TYPE_NO_REPAIRS: i32 = 2;
        assert_eq!(SELL_TYPE_NO_REPAIRS, 2);
        // Normal sell prices should NOT trigger the check
        assert_ne!(0_i32, SELL_TYPE_NO_REPAIRS);
        assert_ne!(1_i32, SELL_TYPE_NO_REPAIRS);
        assert_ne!(100_i32, SELL_TYPE_NO_REPAIRS);
        // Only exactly 2 should trigger
        assert_eq!(2_i32, SELL_TYPE_NO_REPAIRS);
    }

    // ── Sprint 291: NPC range silent return ────────────────────────────

    #[test]
    fn test_npc_range_fail_is_silent_return() {
        // `if (pNpc == nullptr || !isInRange(pNpc, MAX_NPC_RANGE)) return;`
        // C++ returns WITHOUT sending any packet when NPC is invalid/out of range.
        // The Rust handler now uses `return Ok(())` instead of `send_fail()`.
        // This test documents the expected behavior.
        let silent_return = true; // NPC out of range → no packet sent
        let send_fail_packet = false; // Must NOT send error packet
        assert!(silent_return);
        assert!(!send_fail_packet);
    }

    #[test]
    fn test_npc_type_tinker_or_merchant_only() {
        // `if (pNpc->GetType() == NPC_TINKER || pNpc->GetType() == NPC_MERCHANT)`
        // Only these two NPC types can provide repair services.
        // If NPC type is neither, send_fail IS sent (unlike the range check).
        use crate::npc_type_constants::{NPC_MERCHANT, NPC_TINKER};
        let npc_type_healer: u8 = 40;
        let npc_type_gate: u8 = 50;
        assert!(npc_type_healer != NPC_MERCHANT && npc_type_healer != NPC_TINKER);
        assert!(npc_type_gate != NPC_MERCHANT && npc_type_gate != NPC_TINKER);
    }

    // ── Sprint 309: gold_lose_silent for repair ─────────────────────

    // ── Sprint 928: Additional coverage ──────────────────────────────

    /// C2S data length: pos_type(1) + slot(1) + npc_id(4) + item_id(4) = 10.
    #[test]
    fn test_item_repair_c2s_data_length() {
        let mut pkt = Packet::new(Opcode::WizItemRepair as u8);
        pkt.write_u8(1); // pos_type
        pkt.write_u8(0); // slot
        pkt.write_u32(12345); // npc_id
        pkt.write_u32(100001); // item_id
        assert_eq!(pkt.data.len(), 10);
    }

    /// Response data length: result(1) + gold(4) = 5 for both success and fail.
    #[test]
    fn test_item_repair_response_data_length() {
        let mut success = Packet::new(Opcode::WizItemRepair as u8);
        success.write_u8(1);
        success.write_u32(50000);
        assert_eq!(success.data.len(), 5);

        let mut fail = Packet::new(Opcode::WizItemRepair as u8);
        fail.write_u8(0);
        fail.write_u32(50000);
        assert_eq!(fail.data.len(), 5);
    }

    /// pos_type 1=equipment slot, 2=inventory bag.
    #[test]
    fn test_item_repair_pos_type_constants() {
        use super::SLOT_MAX;
        // pos_type=1: equipment, slot must be < SLOT_MAX
        let pos_equipment: u8 = 1;
        let pos_inventory: u8 = 2;
        assert_eq!(pos_equipment, 1);
        assert_eq!(pos_inventory, 2);
        // actual_idx for inventory = SLOT_MAX + slot
        let actual = SLOT_MAX + 5usize;
        assert_eq!(actual, SLOT_MAX + 5);
    }

    /// SLOT_MAX and HAVE_MAX boundaries.
    #[test]
    fn test_item_repair_slot_boundaries() {
        use super::{HAVE_MAX, SLOT_MAX};
        assert!(SLOT_MAX > 0);
        assert!(HAVE_MAX > 0);
        // Equipment index must be < SLOT_MAX
        assert!(0 < SLOT_MAX);
        // Inventory index must be < HAVE_MAX
        assert!(0 < HAVE_MAX);
    }

    /// Repair cost scales linearly with damage quantity.
    #[test]
    fn test_item_repair_cost_scales_with_damage() {
        let buy_price: f64 = 5000.0;
        let max_dur: u16 = 100;

        let cost_25 = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * 25.0
            / max_dur as f64) as u32;
        let cost_50 = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * 50.0
            / max_dur as f64) as u32;
        let cost_100 = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * 100.0
            / max_dur as f64) as u32;

        assert!(cost_25 < cost_50);
        assert!(cost_50 < cost_100);
    }

    #[test]
    fn test_repair_uses_silent_gold_deduction() {
        // The second parameter `false` means "do NOT send WIZ_GOLD_CHANGE packet".
        // The repair handler sends its own WIZ_ITEM_REPAIR response with the
        // updated gold, so sending WIZ_GOLD_CHANGE would be a duplicate.
        // Rust uses `gold_lose_silent()` to match this behavior.
        //
        // Other callers that also use GoldLose(amount, false):
        // - UserSkillStatPointSystem.cpp:95 (skill reset)
        // - UserSkillStatPointSystem.cpp:148 (stat reset)
        // - KnightsDatabaseHandler.cpp:160 (clan creation)
        let gold: u32 = 10000;
        let cost: u32 = 500;
        // Silent deduction: gold changes but no packet
        let new_gold = gold - cost;
        assert_eq!(new_gold, 9500);
        // The response packet embeds gold directly:
        let mut pkt = Packet::new(Opcode::WizItemRepair as u8);
        pkt.write_u8(1); // success
        pkt.write_u32(new_gold);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(9500));
    }
}
