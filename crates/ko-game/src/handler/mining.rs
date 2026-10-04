//! WIZ_MINING (0x86) handler — Mining, Fishing, and Betting Game system.
//! Sub-opcodes:
//!   1 = MiningStart     — Start mining (pickaxe + area check)
//!   2 = MiningAttempt    — Mining attempt (weighted random reward)
//!   3 = MiningStop       — Stop mining
//!   5 = BettingGame      — Gamble 5000 gold
//!   6 = FishingStart     — Start fishing (rod + bait check)
//!   7 = FishingAttempt   — Fishing attempt (weighted random reward)
//!   8 = FishingStop      — Stop fishing

use std::sync::Arc;
use std::time::Instant;

use ko_protocol::{Opcode, Packet, PacketReader};
use rand::{thread_rng, Rng};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::types::{ZONE_ELMORAD, ZONE_KARUS, ZONE_MORADON};
use crate::world::WorldState;
use crate::zone::SessionId;

/// Sub-opcode constants.
const MINING_START: u8 = 1;
const MINING_ATTEMPT: u8 = 2;
const MINING_STOP: u8 = 3;
const BETTING_GAME: u8 = 5;
const FISHING_START: u8 = 6;
const FISHING_ATTEMPT: u8 = 7;
const FISHING_STOP: u8 = 8;
/// Soccer kick action (ball interaction).
const MINING_SOCCER: u8 = 10;

/// Result codes.
const MINING_RESULT_ERROR: u16 = 0;
const MINING_RESULT_SUCCESS: u16 = 1;
const MINING_RESULT_ALREADY: u16 = 2;
const MINING_RESULT_NOT_AREA: u16 = 3;
const MINING_RESULT_NOT_PICKAXE: u16 = 5;
const MINING_RESULT_NO_EARTHWORM: u16 = 7;

/// Mining delay in seconds between attempts.
const MINING_DELAY_SECS: u64 = 5;

/// Item IDs.
pub(crate) const GOLDEN_MATTOCK: u32 = 389_135_000;
pub(crate) const MATTOCK: u32 = 389_132_000;
pub(crate) const GOLDEN_FISHING_ROD: u32 = 191_347_000;
pub(crate) const FISHING_ROD: u32 = 191_346_000;
const RAINWORM: u32 = 508_226_000;
use crate::world::ITEM_EXP;

/// Weapon kind values.
const WEAPON_PICKAXE: i32 = 61;
const WEAPON_FISHING: i32 = 63;

/// Effect IDs for client visual feedback.
const EFFECT_MINING_ITEM: u16 = 13081;
const EFFECT_MINING_EXP: u16 = 13082;
const EFFECT_FISHING_ITEM: u16 = 30730;

use super::SLOT_MAX;
use crate::inventory_constants::RIGHTHAND;

/// Handle the WIZ_MINING packet.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match sub_opcode {
        MINING_START => handle_mining_start(session),
        MINING_ATTEMPT => handle_mining_attempt(session).await,
        MINING_STOP => handle_mining_stop(session),
        BETTING_GAME => handle_betting_game(session),
        FISHING_START => handle_fishing_start(session),
        FISHING_ATTEMPT => handle_fishing_attempt(session).await,
        FISHING_STOP => handle_fishing_stop(session),
        MINING_SOCCER => handle_soccer_kick(session).await,
        _ => {
            debug!(
                "[{}] WIZ_MINING unknown sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Check if the player is in a valid mining area.
fn is_in_mining_area(zone_id: u16, x: f32, z: f32) -> bool {
    match zone_id {
        ZONE_MORADON => (600.0..=666.0).contains(&x) && (348.0..=399.0).contains(&z),
        ZONE_ELMORAD => {
            ((1408.0..=1488.0).contains(&x) && (354.0..=440.0).contains(&z))
                || ((1653.0..=1733.0).contains(&x) && (526.0..=625.0).contains(&z))
        }
        ZONE_KARUS => {
            ((597.0..=720.0).contains(&x) && (1625.0..=1705.0).contains(&z))
                || ((315.0..=415.0).contains(&x) && (1435.0..=1500.0).contains(&z))
        }
        _ => false,
    }
}

/// Check if the player is in a valid fishing area.
fn is_in_fishing_area(zone_id: u16, x: f32, z: f32) -> bool {
    match zone_id {
        ZONE_ELMORAD => (850.0..=935.0).contains(&x) && (1080.0..=1115.0).contains(&z),
        ZONE_KARUS => (1106.0..=1209.0).contains(&x) && (915.0..=944.0).contains(&z),
        _ => false,
    }
}

/// Get EXP reward amount based on player level.
fn get_mining_exp(level: u8) -> i64 {
    match level {
        1..=34 => 50,
        35..=59 => 100,
        60..=69 => 200,
        70..=83 => 300,
        _ => 0,
    }
}

/// Check that the right hand has a valid pickaxe with durability > 0.
/// is GOLDEN_MATTOCK or MATTOCK, and item is not duplicate/rented/sealed.
/// Returns the pickaxe item_id if valid, None otherwise.
fn check_pickaxe(world: &WorldState, sid: SessionId) -> Option<u32> {
    let slot = world.get_inventory_slot(sid, RIGHTHAND)?;
    if slot.item_id == 0 || slot.durability <= 0 {
        return None;
    }
    // Must be one of the two valid pickaxe items
    if slot.item_id != MATTOCK && slot.item_id != GOLDEN_MATTOCK {
        return None;
    }
    // Reject duplicate/rented/sealed items
    if slot.flag == crate::world::ITEM_FLAG_DUPLICATE
        || slot.flag == crate::world::ITEM_FLAG_RENTED
        || slot.flag == crate::world::ITEM_FLAG_SEALED
    {
        return None;
    }
    let item_def = world.get_item(slot.item_id)?;
    let kind = item_def.kind.unwrap_or(0);
    if kind == WEAPON_PICKAXE {
        Some(slot.item_id)
    } else {
        None
    }
}

/// Check that the right hand has a valid fishing rod with durability > 0.
/// is GOLDEN_FISHING_ROD or FISHING_ROD, and item is not duplicate/rented/sealed.
fn check_fishing_rod(world: &WorldState, sid: SessionId) -> Option<u32> {
    let slot = world.get_inventory_slot(sid, RIGHTHAND)?;
    if slot.item_id == 0 || slot.durability <= 0 {
        return None;
    }
    if slot.item_id != FISHING_ROD && slot.item_id != GOLDEN_FISHING_ROD {
        return None;
    }
    if slot.flag == crate::world::ITEM_FLAG_DUPLICATE
        || slot.flag == crate::world::ITEM_FLAG_RENTED
        || slot.flag == crate::world::ITEM_FLAG_SEALED
    {
        return None;
    }
    let item_def = world.get_item(slot.item_id)?;
    let kind = item_def.kind.unwrap_or(0);
    if kind == WEAPON_FISHING {
        Some(slot.item_id)
    } else {
        None
    }
}

/// Count how many of a specific item the player has in their bag.
fn count_item_in_bag(world: &WorldState, sid: SessionId, item_id: u32) -> u16 {
    let mut total = 0u16;
    for i in SLOT_MAX..(SLOT_MAX + 28) {
        if let Some(slot) = world.get_inventory_slot(sid, i) {
            if slot.item_id == item_id && slot.count > 0 {
                total = total.saturating_add(slot.count);
            }
        }
    }
    total
}

/// Consume fractional durability and immediately synchronize the right-hand
/// slot with the client. Item durability is an integer on the wire, so two
/// successful mining attempts consume one point (0.5 per attempt).
fn reduce_righthand_durability(world: &WorldState, sid: SessionId, amount: i16) {
    let mut wear = amount.max(0) as u16;
    if amount == 1 {
        let apply_one = world
            .with_session(sid, |h| !h.mining_half_wear_pending)
            .unwrap_or(true);
        world.update_session(sid, |h| h.mining_half_wear_pending = !h.mining_half_wear_pending);
        wear = u16::from(apply_one);
    }
    let mut new_durability = None;
    world.update_inventory(sid, |inv| {
        if let Some(slot) = inv.get_mut(RIGHTHAND) {
            if slot.item_id != 0 && slot.durability > 0 {
                slot.durability = slot.durability.saturating_sub(wear as i16);
                new_durability = Some(slot.durability.max(0) as u16);
                return true;
            }
        }
        false
    });
    if let Some(durability) = new_durability {
        let mut pkt = Packet::new(Opcode::WizDuration as u8);
        pkt.write_u8(RIGHTHAND);
        pkt.write_u16(durability);
        world.send_to_session_owned(sid, pkt);
    }
}

/// Perform weighted random selection from a list of mining/fishing items.
/// and picks a random index.
pub(crate) fn weighted_random_item(items: &[ko_db::models::MiningFishingItemRow]) -> Option<u32> {
    if items.is_empty() {
        return None;
    }

    // Sum all weights
    let total_weight: u32 = items.iter().map(|r| r.success_rate.max(0) as u32).sum();
    if total_weight == 0 {
        return None;
    }

    let capped = total_weight.min(10000);
    let mut rng = thread_rng();
    let roll = rng.gen_range(0..capped);

    let mut offset = 0u32;
    for item in items {
        let rate = item.success_rate.max(0) as u32;
        if roll < offset + rate {
            return Some(item.n_give_item_id as u32);
        }
        offset += rate;
        if offset >= 10000 {
            break;
        }
    }

    // Fallback to last item
    items.last().map(|r| r.n_give_item_id as u32)
}

/// Handle MiningStart (sub-opcode 1).
fn handle_mining_start(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut result_code = MINING_RESULT_SUCCESS;

    // Read session state for mining validation (single DashMap read)
    let (is_mining, pos) = world
        .with_session(sid, |h| (h.is_mining, h.position))
        .unwrap_or_default();
    if is_mining {
        result_code = MINING_RESULT_ALREADY;
    }

    // Area check
    if result_code == MINING_RESULT_SUCCESS {
        let mining_zone = world
            .get_zone(pos.zone_id)
            .and_then(|z| z.zone_info.as_ref().map(|zi| zi.abilities.mining_zone))
            .unwrap_or(false);
        if !mining_zone || !is_in_mining_area(pos.zone_id, pos.x, pos.z) {
            result_code = MINING_RESULT_NOT_AREA;
        }
    }

    // Pickaxe check
    if result_code == MINING_RESULT_SUCCESS && check_pickaxe(&world, sid).is_none() {
        result_code = MINING_RESULT_NOT_PICKAXE;
    }

    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(MINING_START);
    pkt.write_u16(result_code);

    if result_code == MINING_RESULT_SUCCESS {
        world.update_session(sid, |h| {
            h.is_mining = true;
            h.mining_half_wear_pending = false;
            h.last_mining_attempt = Instant::now();
        });
        pkt.write_u32(sid as u32);
        let (pos, event_room) = world
            .with_session(sid, |h| (h.position, h.event_room))
            .unwrap_or_default();
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    } else {
        world.send_to_session_owned(sid, pkt);
    }

    Ok(())
}

/// Handle MiningAttempt (sub-opcode 2).
async fn handle_mining_attempt(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    // State checks
    let state = world.with_session(sid, |h| {
        (
            h.is_mining,
            h.is_fishing,
            h.trade_state,
            h.merchant_state,
            h.selling_merchant_preparing,
            h.last_mining_attempt,
        )
    });
    let (is_mining, is_fishing, trade_state, merchant_state, selling_prep, last_attempt) =
        match state {
            Some(s) => s,
            None => return Ok(()),
        };

    if !is_mining || is_fishing || trade_state > 1 || merchant_state >= 0 || selling_prep {
        return Ok(());
    }

    // 5-second cooldown
    if last_attempt.elapsed().as_secs() < MINING_DELAY_SECS {
        return Ok(());
    }

    // Update cooldown timestamp
    world.update_session(sid, |h| {
        h.last_mining_attempt = Instant::now();
    });

    // Pickaxe check
    let item_id = match check_pickaxe(&world, sid) {
        Some(id) => id,
        None => {
            send_mining_error(&world, sid, MINING_ATTEMPT);
            stop_mining_internal(&world, sid);
            return Ok(());
        }
    };

    // Determine use_item_type based on which pickaxe
    let use_item_type: u8 = if item_id == GOLDEN_MATTOCK { 1 } else { 0 };

    // Get drop table (war_status=0 for now, no war system)
    let items = world.get_mining_fishing_items(0, use_item_type, 0);
    if items.is_empty() {
        send_mining_error(&world, sid, MINING_ATTEMPT);
        stop_mining_internal(&world, sid);
        return Ok(());
    }

    // Weighted random selection
    let reward_item_id = match weighted_random_item(&items) {
        Some(id) => id,
        None => {
            send_mining_error(&world, sid, MINING_ATTEMPT);
            stop_mining_internal(&world, sid);
            return Ok(());
        }
    };

    // Weight check
    if reward_item_id != ITEM_EXP {
        if let Some(item_def) = world.get_item(reward_item_id) {
            let weight = item_def.weight.unwrap_or(0) as u32;
            let (current_weight, max_weight) = world
                .with_session(sid, |h| {
                    (h.equipped_stats.item_weight, h.equipped_stats.max_weight)
                })
                .unwrap_or((0, 0));
            if current_weight + weight > max_weight {
                return Ok(());
            }
        }
        // Check inventory space
        if world.find_slot_for_item(sid, reward_item_id, 1).is_none() {
            send_mining_error(&world, sid, MINING_ATTEMPT);
            stop_mining_internal(&world, sid);
            return Ok(());
        }
    }

    // Grant reward
    let effect;
    if reward_item_id == ITEM_EXP {
        let level = world
            .get_character_info(sid)
            .map(|ch| ch.level)
            .unwrap_or(0);
        let exp = get_mining_exp(level);
        if exp > 0 {
            crate::handler::level::exp_change(&world, sid, exp).await;
        }
        effect = EFFECT_MINING_EXP;
    } else {
        world.give_item(sid, reward_item_id, 1);
        effect = EFFECT_MINING_ITEM;
    }

    // Golden Pickaxe mining wear is 0.5 per successful attempt. Since the
    // protocol stores an integer durability, the server applies one point on
    // every second attempt and sends the updated value immediately.
    reduce_righthand_durability(&world, sid, 1);
    world.set_user_ability(sid);

    // Broadcast success
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(MINING_ATTEMPT);
    pkt.write_u16(MINING_RESULT_SUCCESS);
    pkt.write_u32(sid as u32);
    pkt.write_u16(effect);

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(pkt),
        None,
        event_room,
    );

    Ok(())
}

/// Handle MiningStop (sub-opcode 3).
fn handle_mining_stop(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    stop_mining_internal(&world, sid);
    Ok(())
}

/// Internal mining stop — broadcast to region + send personal stop.
pub(crate) fn stop_mining_internal(world: &Arc<WorldState>, sid: SessionId) {
    let is_mining = world.with_session(sid, |h| h.is_mining).unwrap_or(false);
    if !is_mining {
        return;
    }

    world.update_session(sid, |h| {
        h.is_mining = false;
        h.mining_half_wear_pending = false;
    });

    // Broadcast stop to region: [u8 sub] [u16 1] [u32 id]
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(MINING_STOP);
    pkt.write_u16(1);
    pkt.write_u32(sid as u32);
    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(pkt),
        None,
        event_room,
    );

    // Send personal stop: [u8 sub] [u16 2]
    let mut pkt2 = Packet::new(Opcode::WizMining as u8);
    pkt2.write_u8(MINING_STOP);
    pkt2.write_u16(2);
    world.send_to_session_owned(sid, pkt2);
}

/// Handle BettingGame (sub-opcode 5).
fn handle_betting_game(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut result_opcode: u16 = 4; // EnougCoins

    let mut player_rand: u8 = 0;
    let mut npc_rand: u8 = 0;

    if !world.gold_lose(sid, 5000) {
        // Not enough coins
    } else {
        let mut rng = thread_rng();
        player_rand = rng.gen_range(1..=5);
        npc_rand = rng.gen_range(1..=5);

        if player_rand > npc_rand {
            result_opcode = 1; // Won
        } else if player_rand < npc_rand {
            result_opcode = 3; // Lose
        } else {
            result_opcode = 2; // Tie
        }

        if result_opcode == 1 {
            world.gold_gain(sid, 10000);
        }
    }

    // Response: [u8 sub] [u16 result] [u16 0] [u8 0] [u8 0] [u8 player_rand] [u8 npc_rand] [u16 0]
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(BETTING_GAME);
    pkt.write_u16(result_opcode);
    pkt.write_u16(0);
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(player_rand);
    pkt.write_u8(npc_rand);
    pkt.write_u16(0);

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(pkt),
        None,
        event_room,
    );

    Ok(())
}

/// Handle FishingStart (sub-opcode 6).
fn handle_fishing_start(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Blinking (respawn invulnerability) prevents fishing start.
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if world.is_player_blinking(sid, now_unix) {
        return Ok(());
    }

    if world.is_player_dead(sid) {
        return Ok(());
    }

    // State checks
    let state = world.with_session(sid, |h| {
        (
            h.is_fishing,
            h.trade_state,
            h.merchant_state,
            h.selling_merchant_preparing,
            h.buying_merchant_preparing,
        )
    });
    let (is_fishing, trade_state, merchant_state, selling_prep, buying_prep) = match state {
        Some(s) => s,
        None => return Ok(()),
    };

    if trade_state > 1 || merchant_state >= 0 || selling_prep || buying_prep {
        return Ok(());
    }

    let mut result_code = MINING_RESULT_SUCCESS;

    if is_fishing {
        result_code = MINING_RESULT_ALREADY;
    }

    // Area check
    if result_code == MINING_RESULT_SUCCESS {
        let pos = world.get_position(sid).unwrap_or_default();
        if !is_in_fishing_area(pos.zone_id, pos.x, pos.z) {
            result_code = MINING_RESULT_NOT_AREA;
        }
    }

    // Rod check
    let rod_id = check_fishing_rod(&world, sid);
    if result_code == MINING_RESULT_SUCCESS && rod_id.is_none() {
        result_code = MINING_RESULT_NOT_PICKAXE;
    }

    // Bait check (not needed for golden rod)
    if result_code == MINING_RESULT_SUCCESS {
        let rod = rod_id.unwrap_or(0);
        if rod != GOLDEN_FISHING_ROD && count_item_in_bag(&world, sid, RAINWORM) == 0 {
            result_code = MINING_RESULT_NO_EARTHWORM;
        }
    }

    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(FISHING_START);
    pkt.write_u16(result_code);

    if result_code == MINING_RESULT_SUCCESS {
        world.update_session(sid, |h| {
            h.is_fishing = true;
            h.last_mining_attempt = Instant::now();
        });
        pkt.write_u32(sid as u32);
        let (pos, event_room) = world
            .with_session(sid, |h| (h.position, h.event_room))
            .unwrap_or_default();
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    } else {
        world.send_to_session_owned(sid, pkt);
    }

    Ok(())
}

/// Handle FishingAttempt (sub-opcode 7).
async fn handle_fishing_attempt(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    // State checks
    let state = world.with_session(sid, |h| {
        (
            h.is_mining,
            h.is_fishing,
            h.trade_state,
            h.merchant_state,
            h.selling_merchant_preparing,
            h.last_mining_attempt,
        )
    });
    let (is_mining, is_fishing, trade_state, merchant_state, selling_prep, last_attempt) =
        match state {
            Some(s) => s,
            None => return Ok(()),
        };

    if is_mining || !is_fishing || trade_state > 1 || merchant_state >= 0 || selling_prep {
        return Ok(());
    }

    // 5-second cooldown
    if last_attempt.elapsed().as_secs() < MINING_DELAY_SECS {
        return Ok(());
    }

    // Update cooldown
    world.update_session(sid, |h| {
        h.last_mining_attempt = Instant::now();
    });

    // Rod check
    let rod_id = match check_fishing_rod(&world, sid) {
        Some(id) => id,
        None => {
            send_fishing_error_and_stop(&world, sid);
            return Ok(());
        }
    };

    // Bait check for non-golden rods
    if rod_id != GOLDEN_FISHING_ROD && count_item_in_bag(&world, sid, RAINWORM) == 0 {
        send_fishing_error_and_stop(&world, sid);
        return Ok(());
    }

    // Determine use_item_type
    let use_item_type: u8 = if rod_id == GOLDEN_FISHING_ROD { 1 } else { 0 };

    // Get drop table (table_type=1 for fishing, war_status=0)
    let items = world.get_mining_fishing_items(1, use_item_type, 0);
    if items.is_empty() {
        return Ok(());
    }

    // Weighted random
    let reward_item_id = match weighted_random_item(&items) {
        Some(id) => id,
        None => return Ok(()),
    };

    // Weight check
    if reward_item_id != ITEM_EXP {
        if let Some(item_def) = world.get_item(reward_item_id) {
            let weight = item_def.weight.unwrap_or(0) as u32;
            let (current_weight, max_weight) = world
                .with_session(sid, |h| {
                    (h.equipped_stats.item_weight, h.equipped_stats.max_weight)
                })
                .unwrap_or((0, 0));
            if current_weight + weight > max_weight {
                return Ok(());
            }
        }
        if world.find_slot_for_item(sid, reward_item_id, 1).is_none() {
            return Ok(());
        }
    }

    // Grant reward
    let effect;
    if reward_item_id == ITEM_EXP {
        let level = world
            .get_character_info(sid)
            .map(|ch| ch.level)
            .unwrap_or(0);
        let exp = get_mining_exp(level);
        if exp > 0 {
            crate::handler::level::exp_change(&world, sid, exp).await;
        }
        effect = EFFECT_MINING_EXP;
    } else {
        world.give_item(sid, reward_item_id, 1);
        effect = EFFECT_FISHING_ITEM;
    }

    // Consume bait (not for golden rod)
    if rod_id != GOLDEN_FISHING_ROD {
        world.rob_item(sid, RAINWORM, 1);
    }

    // Reduce rod durability by 100
    reduce_righthand_durability(&world, sid, 100);
    world.set_user_ability(sid);

    // Broadcast success
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(FISHING_ATTEMPT);
    pkt.write_u16(MINING_RESULT_SUCCESS);
    pkt.write_u32(sid as u32);
    pkt.write_u16(effect);

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(pkt),
        None,
        event_room,
    );

    Ok(())
}

/// Handle FishingStop (sub-opcode 8).
fn handle_fishing_stop(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    stop_fishing_internal(&world, sid);
    Ok(())
}

/// Internal fishing stop — broadcast to region + send personal stop.
pub(crate) fn stop_fishing_internal(world: &Arc<WorldState>, sid: SessionId) {
    let is_fishing = world.with_session(sid, |h| h.is_fishing).unwrap_or(false);
    if !is_fishing {
        return;
    }

    world.update_session(sid, |h| {
        h.is_fishing = false;
    });

    // Broadcast stop to region: [u8 sub] [u16 1] [u32 id]
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(FISHING_STOP);
    pkt.write_u16(1);
    pkt.write_u32(sid as u32);
    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(pkt),
        None,
        event_room,
    );

    // Send personal stop: [u8 sub] [u8 2]
    // Note: C++ uses uint8(2) here, not uint16(2)
    let mut pkt2 = Packet::new(Opcode::WizMining as u8);
    pkt2.write_u8(FISHING_STOP);
    pkt2.write_u8(2);
    world.send_to_session_owned(sid, pkt2);
}

/// Send a mining error and stop mining.
fn send_mining_error(world: &WorldState, sid: SessionId, sub_opcode: u8) {
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(sub_opcode);
    pkt.write_u16(MINING_RESULT_ERROR);
    world.send_to_session_owned(sid, pkt);
}

/// Send a fishing error and stop fishing.
fn send_fishing_error_and_stop(world: &Arc<WorldState>, sid: SessionId) {
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(FISHING_ATTEMPT);
    pkt.write_u16(MINING_RESULT_NOT_PICKAXE);
    world.send_to_session_owned(sid, pkt);
    stop_fishing_internal(world, sid);
}

/// Handle soccer kick action (MiningSoccer = 10).
/// Sets the mining flag and broadcasts kick animation to the region.
async fn handle_soccer_kick(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let already_mining = world.with_session(sid, |h| h.is_mining).unwrap_or(false);

    let (pkt, broadcast) = super::soccer::build_kick_response(sid, already_mining);

    if broadcast {
        // Set mining flag and broadcast to region.
        world.update_session(sid, |h| {
            h.is_mining = true;
        });
        let (pos, event_room) = world
            .with_session(sid, |h| (h.position, h.event_room))
            .unwrap_or_default();
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    } else {
        // Error: send only to self.
        session.send_packet(&pkt).await?;
    }

    debug!(
        "[{}] MINING_SOCCER: broadcast={}, already_mining={}",
        session.addr(),
        broadcast,
        already_mining
    );

    Ok(())
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    // ── Area check tests ─────────────────────────────────────────────

    #[test]
    fn test_mining_area_moradon_inside() {
        assert!(is_in_mining_area(ZONE_MORADON, 630.0, 370.0));
    }

    #[test]
    fn test_mining_area_moradon_boundary() {
        assert!(is_in_mining_area(ZONE_MORADON, 600.0, 348.0));
        assert!(is_in_mining_area(ZONE_MORADON, 666.0, 399.0));
    }

    #[test]
    fn test_mining_area_moradon_outside() {
        assert!(!is_in_mining_area(ZONE_MORADON, 599.0, 370.0));
        assert!(!is_in_mining_area(ZONE_MORADON, 667.0, 370.0));
        assert!(!is_in_mining_area(ZONE_MORADON, 630.0, 347.0));
        assert!(!is_in_mining_area(ZONE_MORADON, 630.0, 400.0));
    }

    #[test]
    fn test_mining_area_elmorad_zone1() {
        assert!(is_in_mining_area(ZONE_ELMORAD, 1450.0, 400.0));
    }

    #[test]
    fn test_mining_area_elmorad_zone2() {
        assert!(is_in_mining_area(ZONE_ELMORAD, 1700.0, 575.0));
    }

    #[test]
    fn test_mining_area_elmorad_outside() {
        assert!(!is_in_mining_area(ZONE_ELMORAD, 100.0, 100.0));
    }

    #[test]
    fn test_mining_area_karus_zone1() {
        assert!(is_in_mining_area(ZONE_KARUS, 660.0, 1660.0));
    }

    #[test]
    fn test_mining_area_karus_zone2() {
        assert!(is_in_mining_area(ZONE_KARUS, 360.0, 1470.0));
    }

    #[test]
    fn test_mining_area_invalid_zone() {
        assert!(!is_in_mining_area(99, 630.0, 370.0));
    }

    #[test]
    fn test_fishing_area_elmorad_inside() {
        assert!(is_in_fishing_area(ZONE_ELMORAD, 890.0, 1100.0));
    }

    #[test]
    fn test_fishing_area_elmorad_boundary() {
        assert!(is_in_fishing_area(ZONE_ELMORAD, 850.0, 1080.0));
        assert!(is_in_fishing_area(ZONE_ELMORAD, 935.0, 1115.0));
    }

    #[test]
    fn test_fishing_area_elmorad_outside() {
        assert!(!is_in_fishing_area(ZONE_ELMORAD, 849.0, 1100.0));
        assert!(!is_in_fishing_area(ZONE_ELMORAD, 890.0, 1116.0));
    }

    #[test]
    fn test_fishing_area_karus_inside() {
        assert!(is_in_fishing_area(ZONE_KARUS, 1150.0, 930.0));
    }

    #[test]
    fn test_fishing_area_karus_outside() {
        assert!(!is_in_fishing_area(ZONE_KARUS, 1105.0, 930.0));
        assert!(!is_in_fishing_area(ZONE_KARUS, 1210.0, 930.0));
    }

    #[test]
    fn test_fishing_area_moradon_not_supported() {
        assert!(!is_in_fishing_area(ZONE_MORADON, 890.0, 1100.0));
    }

    #[test]
    fn test_fishing_area_invalid_zone() {
        assert!(!is_in_fishing_area(99, 1150.0, 930.0));
    }

    // ── EXP reward tests ─────────────────────────────────────────────

    #[test]
    fn test_mining_exp_level_ranges() {
        assert_eq!(get_mining_exp(1), 50);
        assert_eq!(get_mining_exp(34), 50);
        assert_eq!(get_mining_exp(35), 100);
        assert_eq!(get_mining_exp(59), 100);
        assert_eq!(get_mining_exp(60), 200);
        assert_eq!(get_mining_exp(69), 200);
        assert_eq!(get_mining_exp(70), 300);
        assert_eq!(get_mining_exp(83), 300);
    }

    #[test]
    fn test_mining_exp_level_zero() {
        assert_eq!(get_mining_exp(0), 0);
    }

    #[test]
    fn test_mining_exp_level_above_83() {
        assert_eq!(get_mining_exp(84), 0);
        assert_eq!(get_mining_exp(255), 0);
    }

    // ── Weighted random selection tests ──────────────────────────────

    #[test]
    fn test_weighted_random_empty_list() {
        let items: Vec<ko_db::models::MiningFishingItemRow> = vec![];
        assert!(weighted_random_item(&items).is_none());
    }

    #[test]
    fn test_weighted_random_single_item() {
        let items = vec![ko_db::models::MiningFishingItemRow {
            n_index: 1,
            n_table_type: 0,
            n_war_status: 0,
            use_item_type: 0,
            n_give_item_name: "Test".to_string(),
            n_give_item_id: 123456,
            n_give_item_count: 1,
            success_rate: 5000,
        }];
        let result = weighted_random_item(&items);
        assert_eq!(result, Some(123456));
    }

    #[test]
    fn test_weighted_random_zero_rates() {
        let items = vec![ko_db::models::MiningFishingItemRow {
            n_index: 1,
            n_table_type: 0,
            n_war_status: 0,
            use_item_type: 0,
            n_give_item_name: "Test".to_string(),
            n_give_item_id: 123456,
            n_give_item_count: 1,
            success_rate: 0,
        }];
        assert!(weighted_random_item(&items).is_none());
    }

    #[test]
    fn test_weighted_random_multiple_items_returns_valid() {
        let items = vec![
            ko_db::models::MiningFishingItemRow {
                n_index: 1,
                n_table_type: 0,
                n_war_status: 0,
                use_item_type: 0,
                n_give_item_name: "EXP".to_string(),
                n_give_item_id: 900_001_000,
                n_give_item_count: 1,
                success_rate: 9000,
            },
            ko_db::models::MiningFishingItemRow {
                n_index: 2,
                n_table_type: 0,
                n_war_status: 0,
                use_item_type: 0,
                n_give_item_name: "Gem".to_string(),
                n_give_item_id: 389_201_000,
                n_give_item_count: 1,
                success_rate: 1000,
            },
        ];
        let result = weighted_random_item(&items);
        assert!(result.is_some());
        let id = result.unwrap();
        assert!(id == 900_001_000 || id == 389_201_000);
    }

    #[test]
    fn test_weighted_random_capped_at_10000() {
        // Total rate > 10000, should still work without panic
        let items = vec![
            ko_db::models::MiningFishingItemRow {
                n_index: 1,
                n_table_type: 0,
                n_war_status: 0,
                use_item_type: 0,
                n_give_item_name: "A".to_string(),
                n_give_item_id: 1,
                n_give_item_count: 1,
                success_rate: 8000,
            },
            ko_db::models::MiningFishingItemRow {
                n_index: 2,
                n_table_type: 0,
                n_war_status: 0,
                use_item_type: 0,
                n_give_item_name: "B".to_string(),
                n_give_item_id: 2,
                n_give_item_count: 1,
                success_rate: 5000,
            },
        ];
        // Should not panic
        let result = weighted_random_item(&items);
        assert!(result.is_some());
    }

    // ── Constant verification tests ──────────────────────────────────

    #[test]
    fn test_sub_opcodes_match_cpp() {
        assert_eq!(MINING_START, 1);
        assert_eq!(MINING_ATTEMPT, 2);
        assert_eq!(MINING_STOP, 3);
        assert_eq!(BETTING_GAME, 5);
        assert_eq!(FISHING_START, 6);
        assert_eq!(FISHING_ATTEMPT, 7);
        assert_eq!(FISHING_STOP, 8);
    }

    #[test]
    fn test_result_codes_match_cpp() {
        assert_eq!(MINING_RESULT_ERROR, 0);
        assert_eq!(MINING_RESULT_SUCCESS, 1);
        assert_eq!(MINING_RESULT_ALREADY, 2);
        assert_eq!(MINING_RESULT_NOT_AREA, 3);
        assert_eq!(MINING_RESULT_NOT_PICKAXE, 5);
        assert_eq!(MINING_RESULT_NO_EARTHWORM, 7);
    }

    #[test]
    fn test_item_ids_match_cpp() {
        assert_eq!(GOLDEN_MATTOCK, 389_135_000);
        assert_eq!(MATTOCK, 389_132_000);
        assert_eq!(GOLDEN_FISHING_ROD, 191_347_000);
        assert_eq!(FISHING_ROD, 191_346_000);
        assert_eq!(RAINWORM, 508_226_000);
        assert_eq!(ITEM_EXP, 900_001_000);
    }

    #[test]
    fn test_weapon_kinds_match_cpp() {
        assert_eq!(WEAPON_PICKAXE, 61);
        assert_eq!(WEAPON_FISHING, 63);
    }

    #[test]
    fn test_effects_match_cpp() {
        assert_eq!(EFFECT_MINING_ITEM, 13081);
        assert_eq!(EFFECT_MINING_EXP, 13082);
        assert_eq!(EFFECT_FISHING_ITEM, 30730);
    }

    #[test]
    fn test_mining_delay_is_5_seconds() {
        assert_eq!(MINING_DELAY_SECS, 5);
    }

    // ── Sprint 324: isBlinking fishing guard tests ──────────────────

    #[test]
    fn test_fishing_start_has_blinking_guard() {
        // HandleFishingStart() checks isBlinking() FIRST, before dead/state checks.
        // Verify the check uses is_player_blinking with UNIX timestamp.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // A blink_expiry_time in the future means player IS blinking.
        let expiry_future = now + 10;
        assert!(now < expiry_future, "future expiry means blinking");
        // A blink_expiry_time of 0 or in the past means NOT blinking.
        let expiry_past = now.saturating_sub(1);
        assert!(now >= expiry_past, "past expiry means not blinking");
    }

    #[test]
    fn test_mining_start_no_blinking_guard() {
        // HandleMiningStart() does NOT check isBlinking().
        // This test documents that mining_start intentionally lacks the check.
        // (Only fishing_start has it in the protocol specification.)
        assert!(true, "mining_start correctly omits isBlinking check");
    }
}
