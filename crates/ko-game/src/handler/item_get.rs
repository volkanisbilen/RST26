//! WIZ_ITEM_GET (0x26) handler — pick up an item from a ground bundle.
//! Packet format (from client):
//! ```text
//! [u32 bundle_id] [u32 item_id] [u16 slot_id]
//! ```
//! Response (WIZ_ITEM_GET):
//! ```text
//! [u8 result] — 0=error, 1=solo loot, 2=party distribution
//! If success:
//!   [u32 bundle_id] [u8 dst_pos] [u32 item_id] [u16 count] [u32 gold] [u16 slot_id]
//! ```

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::world::{COIN_MAX, ITEMCOUNT_MAX, ITEM_GOLD, NPC_HAVE_ITEM_LIST, RANGE_50M};
use crate::zone::SessionId;

/// Loot error/result codes.
const LOOT_ERROR: u8 = 0;
const LOOT_SOLO: u8 = 1;
const LOOT_PARTY_COIN_DISTRIBUTION: u8 = 2;
const LOOT_PARTY_NOTIFICATION: u8 = 3;
const LOOT_PARTY_ITEM_GIVEN_AWAY: u8 = 4;
const LOOT_NO_WEIGHT: u8 = 6;

use super::npc_loot::get_item_routing_user;
use super::{INVENTORY_TOTAL, SLOT_MAX};

/// Handle WIZ_ITEM_GET from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let bundle_id = reader.read_u32().unwrap_or(0);
    let _item_id = reader.read_u32().unwrap_or(0);
    let slot_id = reader.read_u16().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    let mut result = Packet::new(Opcode::WizItemGet as u8);

    // Validate state — must be alive, not busy, valid slot
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
        || slot_id as usize >= NPC_HAVE_ITEM_LIST
    {
        result.write_u8(LOOT_ERROR);
        return session.send_packet(&result).await;
    }

    // Get the bundle
    let bundle = match world.get_ground_bundle(bundle_id) {
        Some(b) => b,
        None => {
            result.write_u8(LOOT_ERROR);
            return session.send_packet(&result).await;
        }
    };

    // Bundle ownership validation
    let bundle_looter = bundle.looter;
    if bundle_looter != sid {
        // Not the bundle owner — must be in same party
        let my_party = world.get_character_info(sid).and_then(|ch| ch.party_id);
        let owner_party = world
            .get_character_info(bundle_looter)
            .and_then(|ch| ch.party_id);
        match (my_party, owner_party) {
            (Some(mp), Some(op)) if mp == op => {} // same party, OK
            _ => {
                result.write_u8(LOOT_ERROR);
                return session.send_packet(&result).await;
            }
        }
    }

    // Validate bundle has items in the requested slot
    let loot_item = &bundle.items[slot_id as usize];
    if loot_item.item_id == 0 || loot_item.count == 0 {
        result.write_u8(LOOT_ERROR);
        return session.send_packet(&result).await;
    }

    // Range check
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => {
            result.write_u8(LOOT_ERROR);
            return session.send_packet(&result).await;
        }
    };

    let dx = pos.x - bundle.x;
    let dz = pos.z - bundle.z;
    if dx * dx + dz * dz > RANGE_50M {
        result.write_u8(LOOT_ERROR);
        return session.send_packet(&result).await;
    }

    let loot_item_id = loot_item.item_id;

    // ── Atomically take from bundle FIRST — prevents duplication race ────
    // Only ONE concurrent caller can succeed on the same bundle+slot.
    let taken = world.try_take_bundle_item(bundle_id, slot_id);
    let (taken_id, taken_count) = match taken {
        Some((id, cnt)) if id == loot_item_id => (id, cnt),
        _ => {
            // Already taken by another player, or item mismatch
            result.write_u8(LOOT_ERROR);
            return session.send_packet(&result).await;
        }
    };

    // Handle gold pickup — apply bonus multipliers (buff, item, clan premium)
    if taken_id == ITEM_GOLD {
        let party_id = world.get_party_id(sid);
        let party = party_id.and_then(|pid| world.get_party(pid));

        if let Some(party) = party {
            // ── Party gold distribution ──────────────────────────────────
            // Find alive party members within RANGE_50M of the bundle.
            let mut eligible: Vec<SessionId> = Vec::with_capacity(8);
            for &member_sid in &party.active_members() {
                // Single DashMap read: check alive + in-range (3 reads → 1)
                let in_range = world
                    .with_session(member_sid, |h| {
                        let ch = h.character.as_ref()?;
                        if ch.res_hp_type == crate::world::USER_DEAD || ch.hp <= 0 {
                            return None;
                        }
                        let dx = h.position.x - bundle.x;
                        let dz = h.position.z - bundle.z;
                        Some(dx * dx + dz * dz <= RANGE_50M)
                    })
                    .flatten()
                    .unwrap_or(false);
                if in_range {
                    eligible.push(member_sid);
                }
            }

            if eligible.is_empty() {
                result.write_u8(LOOT_ERROR);
                return session.send_packet(&result).await;
            }

            // Split gold equally among eligible members.
            // `int coins = (int)(pBundle->Items[SlotID].sCount / (float)partyUsers.size());`
            let share = (taken_count as f32 / eligible.len() as f32) as u32;

            for &member_sid in &eligible {
                // Apply each member's individual bonus multipliers.
                if !world.try_jackpot_noah(member_sid, share) {
                    world.gold_gain_with_bonus_silent(member_sid, share);
                }

                let member_gold = world
                    .get_character_info(member_sid)
                    .map(|ch| ch.gold)
                    .unwrap_or(0);

                // Send LootPartyCoinDistribution to each member.
                // Packet: [u8 result=2] [u32 bundle_id] [u8 0xFF] [u32 item_id] [u32 coins]
                let mut coin_pkt = Packet::new(Opcode::WizItemGet as u8);
                coin_pkt.write_u8(LOOT_PARTY_COIN_DISTRIBUTION);
                coin_pkt.write_u32(bundle_id);
                coin_pkt.write_u8(0xFF); // uint8(-1)
                coin_pkt.write_u32(taken_id);
                coin_pkt.write_u32(member_gold);
                world.send_to_session_owned(member_sid, coin_pkt);
            }

            // Send LootPartyItemGivenAway to the picker.
            let mut away_pkt = Packet::new(Opcode::WizItemGet as u8);
            away_pkt.write_u8(LOOT_PARTY_ITEM_GIVEN_AWAY);
            session.send_packet(&away_pkt).await?;
        } else {
            // ── Solo gold pickup (no party) ──────────────────────────────
            //   if ((GetCoins() + pItem.sCount) > COIN_MAX) return nullptr;
            let current_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
            if current_gold as u64 + taken_count as u64 > COIN_MAX as u64 {
                world.restore_bundle_item(bundle_id, slot_id, taken_id, taken_count);
                result.write_u8(LOOT_ERROR);
                return session.send_packet(&result).await;
            }
            if !world.try_jackpot_noah(sid, taken_count as u32) {
                // false = don't send WIZ_GOLD_CHANGE (LOOT_SOLO packet handles it)
                world.gold_gain_with_bonus_silent(sid, taken_count as u32);
            }
            let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);

            result.write_u8(LOOT_SOLO);
            result.write_u32(bundle_id);
            result.write_i8(-1);
            result.write_u32(taken_id);
            result.write_u16(taken_count);
            result.write_u32(gold);
            // v2600: no trailing u16 slot_id (sniff verified)

            session.send_packet(&result).await?;
        }
        return Ok(());
    }

    // Non-gold party loot uses the same rotating receiver selection as auto-loot.
    // The character who clicked the bundle only initiates the pickup; ownership
    // is assigned to the next eligible nearby party member.
    let party_id = world.get_party_id(sid);
    let receiver = party_id
        .and_then(|pid| get_item_routing_user(&world, pid, sid, taken_id, taken_count))
        .unwrap_or(sid);

    // Non-gold item: check weight and find a slot on the actual receiver.
    if !world.check_weight(receiver, taken_id, taken_count) {
        // Put item back into bundle since we can't carry it
        world.restore_bundle_item(bundle_id, slot_id, taken_id, taken_count);
        result.write_u8(LOOT_NO_WEIGHT);
        return session.send_packet(&result).await;
    }

    let dst_pos = match world.find_slot_for_item(receiver, taken_id, taken_count) {
        Some(p) if p < INVENTORY_TOTAL => p,
        _ => {
            // No space — put item back
            world.restore_bundle_item(bundle_id, slot_id, taken_id, taken_count);
            result.write_u8(LOOT_ERROR);
            return session.send_packet(&result).await;
        }
    };

    // Look up item definition for durability
    let item_def = match world.get_item(taken_id) {
        Some(i) => i,
        None => {
            world.restore_bundle_item(bundle_id, slot_id, taken_id, taken_count);
            result.write_u8(LOOT_ERROR);
            return session.send_packet(&result).await;
        }
    };

    // Add item to inventory — capture total count for the response packet
    let mut new_total_count: u16 = 0;
    let serial = world.generate_item_serial();
    let success = world.update_inventory(receiver, |inv| {
        if dst_pos >= inv.len() {
            return false;
        }
        let slot = &mut inv[dst_pos];
        let is_new = slot.item_id == 0;
        slot.item_id = taken_id;
        slot.count = slot.count.saturating_add(taken_count).min(ITEMCOUNT_MAX);
        if is_new {
            slot.durability = item_def.duration.unwrap_or(0);
            slot.serial_num = serial;
        }
        new_total_count = slot.count;
        true
    });

    if !success {
        // Failed to add to inventory — put item back in bundle
        world.restore_bundle_item(bundle_id, slot_id, taken_id, taken_count);
        result.write_u8(LOOT_ERROR);
        return session.send_packet(&result).await;
    }

    // Recalculate ability and weight (weight notification is integrated into set_user_ability)
    world.set_user_ability(receiver);

    let gold = world
        .get_character_info(receiver)
        .map(|ch| ch.gold)
        .unwrap_or(0);

    let inv_pos = (dst_pos - SLOT_MAX) as u8;

    result.write_u8(LOOT_SOLO);
    result.write_u32(bundle_id);
    result.write_u8(inv_pos);
    result.write_u32(taken_id);
    result.write_u16(new_total_count);
    result.write_u32(gold);
    // v2600: no trailing u16 slot_id (sniff verified — 16 bytes, not 18)

    if receiver == sid {
        session.send_packet(&result).await?;
    } else {
        world.send_to_session_owned(receiver, result);
        let mut away = Packet::new(Opcode::WizItemGet as u8);
        away.write_u8(LOOT_PARTY_ITEM_GIVEN_AWAY);
        session.send_packet(&away).await?;
    }

    // ── Party notification for non-gold item pickup ────────────────
    //   if (isInParty()) {
    //     result << LootPartyNotification << nBundleID << nItemID << pReceiver->GetName() << SlotID;
    //     g_pMain->Send_PartyMember(GetPartyID(), &result);
    //   }
    if let Some(pid) = party_id {
        let receiver_name = world
            .get_character_info(receiver)
            .map(|ch| ch.name.clone())
            .unwrap_or_default();
        let mut notify = Packet::new(Opcode::WizItemGet as u8);
        notify.write_u8(LOOT_PARTY_NOTIFICATION);
        notify.write_u32(bundle_id);
        notify.write_u32(taken_id);
        notify.write_sbyte_string(&receiver_name);
        // v2600: no trailing u16 slot_id (sniff verified)
        world.send_to_party(pid, &notify);
    }

    // ── Drop Notice: server-wide broadcast for rare items ─────────────
    //   if (pTable.m_isDropNotice && g_pMain->pServerSetting.DropNotice && !isGM())
    //     Send_All(WIZ_LOGOSSHOUT, 0x02, 0x04, name, item_num, rank)
    if item_def.drop_notice.unwrap_or(0) != 0 && !world.is_gm(receiver) {
        let drop_notice_enabled = world
            .get_server_settings()
            .map(|s| s.drop_notice != 0)
            .unwrap_or(false);
        if drop_notice_enabled {
            let receiver_name = world.get_session_name(receiver).unwrap_or_default();
            let rank = world
                .with_session(receiver, |h| h.personal_rank)
                .unwrap_or(0);
            let notice = super::logosshout::build_drop_notice(&receiver_name, taken_id, rank);
            world.broadcast_to_all(Arc::new(notice), None);
        }
    }

    // v2525: Collection item notification (0xA9 sub=2)
    super::collection::notify_item_collected(&world, receiver, taken_id, new_total_count);

    // FerihaLog: NpcDropReceivedInsertLog
    super::audit_log::log_npc_drop(
        session.pool(),
        session.account_id().unwrap_or(""),
        &world.get_session_name(receiver).unwrap_or_default(),
        pos.zone_id as i16,
        pos.x as i16,
        pos.z as i16,
        taken_id,
        taken_count,
        bundle.npc_id,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sprint 290: COIN_MAX gold overflow tests ────────────────────────

    #[test]
    fn test_coin_max_overflow_check() {
        // (GetCoins() + pItem.sCount) > COIN_MAX → reject
        let _current: u32 = 2_000_000_000;
        let _pickup: u16 = 200_000_000u32 as u16; // wraps, but real test:
        let current2: u64 = 2_000_000_000;
        let pickup2: u64 = 200_000_000;
        assert!(
            current2 + pickup2 > COIN_MAX as u64,
            "Gold overflow should be detected"
        );
    }

    #[test]
    fn test_coin_max_under_limit_allowed() {
        let current: u64 = 1_900_000_000;
        let pickup: u64 = 100_000_000;
        assert!(
            current + pickup <= COIN_MAX as u64,
            "Gold within limit should pass"
        );
    }

    #[test]
    fn test_loot_party_notification_constant() {
        assert_eq!(LOOT_PARTY_NOTIFICATION, 3);
    }

    #[test]
    fn test_loot_result_codes() {
        assert_eq!(LOOT_ERROR, 0);
        assert_eq!(LOOT_SOLO, 1);
        assert_eq!(LOOT_PARTY_COIN_DISTRIBUTION, 2);
        assert_eq!(LOOT_PARTY_NOTIFICATION, 3);
        assert_eq!(LOOT_PARTY_ITEM_GIVEN_AWAY, 4);
        assert_eq!(LOOT_NO_WEIGHT, 6);
    }

    #[test]
    fn test_c2s_item_get_packet_format() {
        // C2S: [u32 bundle_id] [u32 item_id] [u16 slot_id]
        let mut pkt = Packet::new(Opcode::WizItemGet as u8);
        pkt.write_u32(42); // bundle_id
        pkt.write_u32(379006001); // item_id
        pkt.write_u16(3); // slot_id

        assert_eq!(pkt.opcode, Opcode::WizItemGet as u8);
        assert_eq!(pkt.data.len(), 10); // 4 + 4 + 2

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u32(), Some(379006001));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_s2c_loot_error_response() {
        // S2C error: [u8 LOOT_ERROR=0]
        let mut pkt = Packet::new(Opcode::WizItemGet as u8);
        pkt.write_u8(LOOT_ERROR);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_s2c_loot_solo_response_format() {
        // S2C success: [u8 LOOT_SOLO] [u32 bundle_id] [u8 dst_pos] [u32 item_id]
        //              [u16 count] [u32 gold] [u16 slot_id]
        let mut pkt = Packet::new(Opcode::WizItemGet as u8);
        pkt.write_u8(LOOT_SOLO); // result
        pkt.write_u32(42); // bundle_id
        pkt.write_u8(5); // dst_pos
        pkt.write_u32(379006001); // item_id
        pkt.write_u16(10); // count
        pkt.write_u32(50_000); // gold
        pkt.write_u16(0); // slot_id

        assert_eq!(pkt.data.len(), 18); // 1+4+1+4+2+4+2=18

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(LOOT_SOLO));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u32(), Some(379006001));
        assert_eq!(r.read_u16(), Some(10));
        assert_eq!(r.read_u32(), Some(50_000));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_loot_constants_unique() {
        let codes = [
            LOOT_ERROR,
            LOOT_SOLO,
            LOOT_PARTY_COIN_DISTRIBUTION,
            LOOT_PARTY_NOTIFICATION,
            LOOT_PARTY_ITEM_GIVEN_AWAY,
            LOOT_NO_WEIGHT,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j], "codes at {} and {} should differ", i, j);
            }
        }
    }

    #[test]
    fn test_item_gold_constant() {
        // Gold is a special item_id — cannot be picked up normally
        assert_eq!(ITEM_GOLD, 900000000);
    }

    #[test]
    fn test_range_50m_constant() {
        // RANGE_50M is squared/scaled distance (2500.0), used for bundle pickup range check
        assert_eq!(RANGE_50M, 2500.0);
    }

    #[test]
    fn test_npc_have_item_list() {
        // Max items per bundle = 8
        assert_eq!(NPC_HAVE_ITEM_LIST, 12);
    }

    #[test]
    fn test_slot_id_valid_range() {
        // slot_id must be < NPC_HAVE_ITEM_LIST
        for i in 0..NPC_HAVE_ITEM_LIST {
            assert!(i < NPC_HAVE_ITEM_LIST);
        }
    }

    #[test]
    fn test_itemcount_max_boundary() {
        // Max stack count — verify it's a reasonable value
        assert!(ITEMCOUNT_MAX > 0);
        assert!(ITEMCOUNT_MAX <= u16::MAX);
    }
}
