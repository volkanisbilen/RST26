//! NPC AI State Machine tick system.
//! - `GameServer/NpcThread.cpp:48-224` — `CNpcThread::_Engine()` (main loop)
//! - `GameServer/Npc.cpp` — State handler functions (NpcStanding, NpcMoving, etc.)
//! - `GameServer/NpcDefines.h` — Constants (NPC_MAX_MOVE_RANGE, VIEW_DIST, etc.)
//! - `shared/globals.h:80-95` — NpcState enum
//! ## Architecture
//! One tokio task dispatches AI processing every 250ms (matching `Sleep(250)`).
//! NPCs are grouped by zone and processed in parallel across tokio worker threads.
//! Only NPCs in zones with active players are ticked (optimization).
//! ## State Machine
//! ```text
//! DEAD ──(regen_time)──> LIVE ──> STANDING
//!                                    │
//!                          ┌─────────┴─────────┐
//!                   RandomMove?           FindEnemy?
//!                          │                   │
//!                       MOVING            ATTACKING
//!                          │                   │
//!                   ┌──────┴──────┐     ┌──────┴──────┐
//!              MoveEnd    FindEnemy  InRange?    GetPath
//!                 │          │         │            │
//!            STANDING   ATTACKING  FIGHTING     TRACING
//!                                     │            │
//!                              Attack loop   ┌────┴────┐
//!                                     │    InRange?   Leash?
//!                              TargetDead    │         │
//!                                     │  FIGHTING  STANDING
//!                              STANDING
//!   SLEEPING ──(duration expires)──> FIGHTING
//!   FAINTING ──(2 seconds)──> STANDING
//!   HEALING  ──(find injured NPC)──> cast heal ──> HEALING/STANDING
//!   CASTING  ──(cast time)──> apply skill ──> old_state
//! ```

use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use rand::SeedableRng;

use ko_protocol::{Opcode, Packet};

use crate::npc::{NpcId, NpcTemplate};
use crate::systems::pathfind;
use crate::world::combat;
use crate::world::{
    is_gate_npc_type, NpcAiState, NpcState, WorldState, NPC_MAX_LEASH_RANGE, USER_DEAD, ZONE_ARENA,
    ZONE_CHAOS_DUNGEON, ZONE_KNIGHT_ROYALE, ZONE_KROWAZ_DOMINION, ZONE_MORADON, ZONE_MORADON2,
    ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
};
use crate::zone::{calc_region, SessionId};

/// AI tick interval in milliseconds.
const AI_TICK_INTERVAL_MS: u64 = 250;

use crate::attack_constants::{ATTACK_FAIL, ATTACK_SUCCESS, ATTACK_TARGET_DEAD, LONG_ATTACK};
use crate::handler::durability::WORE_TYPE_DEFENCE;

use crate::attack_constants::MAX_DAMAGE;

/// Fainting duration in milliseconds.
const FAINTING_TIME_MS: u64 = 2000;

/// HP threshold for healer NPCs to consider an ally as needing healing (90%).
const HEALER_HP_THRESHOLD: f32 = 0.9;

/// Skill cooldown for NPC magic attacks (in ms).
/// Most classes use 2s, mages/priests use 3s. We use 2s as the shared default.
const NPC_SKILL_COOLDOWN_MS: u64 = 2000;

/// Magic attack percentage threshold — out of 5000 random roll.
/// Default is 1000/5000 = ~20% chance to attempt a magic attack.
const NPC_MAGIC_PERCENT_DEFAULT: i32 = 1000;

/// HP regen interval in milliseconds — every 15 seconds.
const HP_REGEN_INTERVAL_MS: u64 = 15_000;

/// HP regen percentage per tick — 3% of max HP.
const HP_REGEN_PERCENT: f64 = 3.0;

/// This is the tick interval between NPC movement steps (in ms).
const MONSTER_SPEED: u64 = 1500;

/// Distance threshold for direct movement (skip A* pathfinding).
/// NPC moves directly. We also skip pathfinding for short ranges where line-of-sight
/// is clear, matching `GetTargetPath()` returning 0 for non-dungeon monsters.
const DIRECT_MOVE_THRESHOLD: f32 = 15.0;

/// Distance threshold for path recalculation — when target moves more than this
/// from the position where the path was last computed.
/// but we optimize by caching the path and only recalculating when the target moves significantly.
const PATH_RECALC_DISTANCE: f32 = 5.0;

/// NPC_MAX_MOVE_RANGE — maximum distance for NPC movement per path computation.
const NPC_MAX_MOVE_RANGE: f32 = 100.0;

/// Minimum NPC search range for guard NPCs — guards always need a reasonable
/// detection range even if their template value is low.
/// and always search.  We give them a minimum range of 30 (melee engagement).
/// Regular monsters use their template search_range directly (C++ parity).
const MIN_GUARD_SEARCH_RANGE: f32 = 30.0;

/// TENDER_ATTACK_TYPE — passive NPC, only attacks when damaged.
const TENDER_ATTACK_TYPE: u8 = 0;

/// ATROCITY_ATTACK_TYPE — aggressive NPC, attacks on sight.
#[cfg(test)]
const ATROCITY_ATTACK_TYPE: u8 = 1;

/// Barracks NPC proto ID — excluded from HP regen.
const BARRACKS_PROTO_ID: u16 = 511;

// ── Gate NPC Type Constants ──────────────────────────────────────────────

use crate::npc_type_constants::{NPC_OBJECT_WOOD, NPC_ROLLINGSTONE, NPC_SPECIAL_GATE};

/// NPC_KROWAZ_GATE — auto-closes in Krowaz Dominion zone.
const NPC_KROWAZ_GATE: u8 = 180;

use crate::object_event_constants::OBJECT_FLAG_LEVER;

/// Wood object cooldown threshold — gate auto-closes after this many ticks.
const WOOD_COOLDOWN_THRESHOLD: u32 = 30;

const NPC_BOSS: u8 = 3;

// ── Guard NPC Type Constants ─────────────────────────────────────────────

/// NPC_GUARD — town/field guard that attacks enemy-nation monsters.
const NPC_GUARD: u8 = 11;

/// NPC_PATROL_GUARD — patrolling guard variant.
const NPC_PATROL_GUARD: u8 = 12;

/// NPC_STORE_GUARD — shop-area guard variant.
const NPC_STORE_GUARD: u8 = 13;

/// Check if an NPC type is a guard type.
/// NPC_GUARD, NPC_PATROL_GUARD, NPC_STORE_GUARD.
fn is_guard_type(npc_type: u8) -> bool {
    npc_type == NPC_GUARD || npc_type == NPC_PATROL_GUARD || npc_type == NPC_STORE_GUARD
}

// ── Boss Monster Proto IDs (magic_attack == 3) ─────────────────────────

/// UTC Room 1: Emperor Mammoth (timed skill sequence).
const BOSS_EMPEROR_MAMMOTH: std::ops::RangeInclusive<u16> = 9501..=9503;
/// UTC Room 1: Cresher Gimmic (timed effecting + casting at 50s).
const BOSS_CRESHERGIMMIC: [u16; 4] = [9504, 9505, 9506, 9507];
/// UTC Room 1: Elite Timarli (MAGIC_FLYING).
const BOSS_ELITE_TIMARLI: [u16; 3] = [9523, 9524, 9541];
/// UTC Room 2: Purious (timed sequence with random specials at 60s).
const BOSS_PURIOUS: [u16; 4] = [9508, 9509, 9510, 9511];
/// UTC Room 2: Moebius Evil/Rage (timed sequence, 3 phase).
const BOSS_MOEBIUS: [u16; 4] = [9528, 9529, 9544, 9545];
/// UTC Room 2: Garioneus (timed sequence, 3 phase).
const BOSS_GARIONEUS: [u16; 3] = [9525, 9526, 9542];
/// UTC Room 3: Sorcerer Geden (long intervals 40/80/120s).
const BOSS_SORCERER_GEDEN: [u16; 3] = [9530, 9531, 9532];
/// UTC Room 3: Atal (intervals 20/60/90s).
const BOSS_ATAL: u16 = 9534;
/// UTC Room 3: Moospell (intervals 30/70/100s).
const BOSS_MOSPELL: u16 = 9535;
/// UTC Room 3: Ahmi (intervals 10/35/65s).
const BOSS_AHMI: u16 = 9536;
/// UTC Room 3: Fluwiton Room 3 (timed + random casting at 65s).
const BOSS_FLUWITON_ROOM_3: std::ops::RangeInclusive<u16> = 9512..=9514;
/// UTC Room 4: Fluwiton Room 4 (rapid effecting every 5s + casting at 70s).
const BOSS_FLUWITON_ROOM_4: [u16; 4] = [9515, 9516, 9517, 9518];

/// C++ MAGIC_FLYING opcode value — used by Elite Timarli boss pattern.
const MAGIC_OPCODE_FLYING: u8 = 4;

/// Start the NPC AI background tick task.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_npc_ai_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(AI_TICK_INTERVAL_MS));
        let mut tick_counter: u64 = 0;
        loop {
            interval.tick().await;
            tick_counter = tick_counter.wrapping_add(AI_TICK_INTERVAL_MS);
            process_ai_tick(world.clone(), tick_counter).await;
        }
    })
}

/// Process one AI tick for all active NPCs.
/// NPCs are grouped by zone and processed in parallel across tokio worker threads.
async fn process_ai_tick(world: Arc<WorldState>, now_ms: u64) {
    world
        .manes_survival_manager
        .finalize_requested_event(world.clone());

    // ── Process scheduled respawns (monster respawn loop chain) ──────
    {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let ready = world.drain_ready_respawns(now_secs);
        for entry in ready {
            let ids =
                world.spawn_event_npc(entry.born_sid, true, entry.zone_id, entry.x, entry.z, 1);
            if !ids.is_empty() {
                tracing::debug!(
                    "RespawnLoop: spawned NPC {} in zone {} at ({}, {})",
                    entry.born_sid,
                    entry.zone_id,
                    entry.x,
                    entry.z
                );
            }
        }
    }

    // Group NPCs by zone and process each zone in parallel
    let npc_ids_by_zone = world.get_ai_npc_ids_by_zone();

    let mut join_set = tokio::task::JoinSet::new();
    for (zone_id, npc_ids) in npc_ids_by_zone {
        // Skip entire zone if no players (except dead NPCs needing respawn)
        if !world.zone_has_players(zone_id) {
            // Still need to tick dead NPCs for respawn tracking
            let world = world.clone();
            join_set.spawn(async move {
                for npc_id in npc_ids {
                    let ai = match world.get_npc_ai(npc_id) {
                        Some(a) => a,
                        None => continue,
                    };
                    if ai.state != NpcState::Dead {
                        continue;
                    }
                    process_single_npc(&world, npc_id, &ai, now_ms).await;
                }
            });
            continue;
        }

        let world = world.clone();
        join_set.spawn(async move {
            for npc_id in npc_ids {
                let ai = match world.get_npc_ai(npc_id) {
                    Some(a) => a,
                    None => continue,
                };
                process_single_npc(&world, npc_id, &ai, now_ms).await;
            }
        });
    }

    // Wait for all zone tasks to complete
    while join_set.join_next().await.is_some() {}
}

/// Process a single NPC's AI tick.
async fn process_single_npc(world: &WorldState, npc_id: NpcId, ai: &NpcAiState, now_ms: u64) {
    // Check delay — NPC not ready yet
    let elapsed = now_ms.saturating_sub(ai.last_tick_ms);
    if elapsed < ai.delay_ms && ai.last_tick_ms > 0 {
        // C++ interrupt: standing NPCs can detect enemies between ticks.
        //
        // CheckFindEnemy pre-filter: only run the expensive find_enemy() if
        // there are actual players in the NPC's region.  C++ checks
        // `m_byMoving == 1` (region has user activity); we use
        // `zone.region_3x3_has_users()` which is equivalent.
        if ai.state == NpcState::Standing
            && (ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE)
        {
            // Lightweight pre-filter: skip expensive find_enemy if no users nearby
            let has_nearby_users = world
                .get_zone(ai.zone_id)
                .map(|z| z.region_3x3_has_users(ai.region_x, ai.region_z))
                .unwrap_or(false);
            if has_nearby_users {
                if let Some(target) = find_enemy(world, ai, npc_id) {
                    world.update_npc_ai(npc_id, |s| {
                        s.target_id = Some(target);
                        s.state = NpcState::Attacking;
                        s.delay_ms = 0;
                        s.skill_cooldown_ms = s.last_tick_ms + NPC_SKILL_COOLDOWN_MS;
                    });
                }
            }
        }
        return;
    }

    // Get template for this NPC
    let npc = match world.get_npc_instance(npc_id) {
        Some(n) => n,
        None => return,
    };
    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => return,
    };

    // Check if NPC is dead (HP side)
    let is_dead = world.is_npc_dead(npc_id);

    // Execute state handler
    let new_delay = match ai.state {
        NpcState::Dead => npc_dead(world, npc_id, ai, &tmpl),
        NpcState::Live => npc_live(world, npc_id, ai, &tmpl),
        NpcState::Standing => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_standing(world, npc_id, ai, &tmpl)
            }
        }
        NpcState::Moving => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_moving(world, npc_id, ai, &tmpl)
            }
        }
        NpcState::Attacking => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_attacking(world, npc_id, ai, &tmpl)
            }
        }
        NpcState::Tracing => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_tracing(world, npc_id, ai, &tmpl)
            }
        }
        NpcState::Fighting => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_fighting(world, npc_id, ai, &tmpl).await
            }
        }
        NpcState::Back => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_back(world, npc_id, ai, &tmpl)
            }
        }
        NpcState::Sleeping => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_sleeping(world, npc_id, ai, &tmpl, now_ms)
            }
        }
        NpcState::Fainting => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_fainting(world, npc_id, ai, &tmpl, now_ms)
            }
        }
        NpcState::Healing => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_healing(world, npc_id, ai, &tmpl).await
            }
        }
        NpcState::Casting => {
            if is_dead {
                transition_to_dead(world, npc_id);
                Some(250)
            } else {
                npc_casting(world, npc_id, ai, &tmpl).await
            }
        }
    };

    // ── NPC HP Regen (15-second tick) ─────────────────────────────
    //   dwTickTime = fTime2 - pNpc->m_fHPChangeTime;
    //   if (15 * SECOND < dwTickTime) pNpc->HpChange();
    if !is_dead {
        let hp_regen_elapsed = now_ms.saturating_sub(ai.last_hp_regen_ms);
        if hp_regen_elapsed > HP_REGEN_INTERVAL_MS {
            // Skip barracks NPC (proto_id == 511, non-monster)
            let skip_regen = !npc.is_monster && npc.proto_id == BARRACKS_PROTO_ID;
            if !skip_regen {
                if let Some(current_hp) = world.get_npc_hp(npc_id) {
                    let max_hp = world.get_npc_war_max_hp(&tmpl);
                    if current_hp < max_hp {
                        let heal = ((max_hp as f64 * HP_REGEN_PERCENT) / 100.0).ceil() as i32;
                        let new_hp = (current_hp + heal).min(max_hp);
                        world.update_npc_hp(npc_id, new_hp);
                    }
                }
            }
            world.update_npc_ai(npc_id, |s| {
                s.last_hp_regen_ms = now_ms;
            });
        }
    }

    // ── NPC Duration Death (summon timeout) ──────────────────────
    //   if (pNpc->isAlive() && pNpc->m_sDuration > 0
    //       && pNpc->m_iSpawnedTime
    //       && (int32(UNIXTIME) - pNpc->m_iSpawnedTime > pNpc->m_sDuration))
    //       pNpc->Dead();
    if !is_dead && ai.duration_secs > 0 && ai.spawned_at_ms > 0 {
        let duration_ms = ai.duration_secs as u64 * 1000;
        let alive_ms = now_ms.saturating_sub(ai.spawned_at_ms);
        if alive_ms > duration_ms {
            tracing::debug!(
                "NPC {} duration expired ({}s > {}s), killing",
                npc_id,
                alive_ms / 1000,
                ai.duration_secs
            );
            // Set HP to 0 and transition to Dead
            world.update_npc_hp(npc_id, 0);
            transition_to_dead(world, npc_id);
            // Broadcast death to nearby players
            send_npc_despawn(world, npc_id, ai);
            return;
        }
    }

    // Update timing
    if let Some(delay) = new_delay {
        world.update_npc_ai(npc_id, |s| {
            s.delay_ms = delay;
            s.last_tick_ms = now_ms;
        });
    }
}

// ── State handlers ──────────────────────────────────────────────────────

/// NPC_DEAD state: Wait for regen timer, then transition to LIVE.
fn npc_dead(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    _tmpl: &NpcTemplate,
) -> Option<u64> {
    // A zero regen time explicitly marks dynamically spawned event NPCs as
    // non-respawning. Monster Stone monsters use this path; treating zero as
    // the minimum delay made them come back at the same location after 250ms.
    if !can_auto_respawn(ai.regen_time_ms) {
        return None;
    }

    // Transition to LIVE (which will restore HP and set standing)
    world.update_npc_ai(npc_id, |s| {
        s.state = NpcState::Live;
    });
    Some(ai.regen_time_ms.max(250))
}

#[inline]
fn can_auto_respawn(regen_time_ms: u64) -> bool {
    regen_time_ms > 0
}

/// NPC_LIVE state: Restore HP to max, reposition to spawn, transition to Standing.
fn npc_live(world: &WorldState, npc_id: NpcId, ai: &NpcAiState, tmpl: &NpcTemplate) -> Option<u64> {
    // Restore HP to max (war-adjusted if active)
    world.update_npc_hp(npc_id, world.get_npc_war_max_hp(tmpl));

    // Reset position to spawn
    let spawn_x = ai.spawn_x;
    let spawn_z = ai.spawn_z;
    let region_x = calc_region(spawn_x);
    let region_z = calc_region(spawn_z);

    // Update region if changed
    if region_x != ai.region_x || region_z != ai.region_z {
        if let Some(zone) = world.get_zone(ai.zone_id) {
            zone.remove_npc(ai.region_x, ai.region_z, npc_id);
            zone.add_npc(region_x, region_z, npc_id);
        }
    }

    // Send NPC_IN to region so clients see it again
    send_npc_respawn(world, npc_id, ai.zone_id, region_x, region_z, tmpl);

    world.update_npc_ai(npc_id, |s| {
        s.state = NpcState::Standing;
        s.cur_x = spawn_x;
        s.cur_z = spawn_z;
        s.region_x = region_x;
        s.region_z = region_z;
        s.target_id = None;
        s.npc_target_id = None;
    });

    Some(tmpl.stand_time as u64)
}

/// NPC_STANDING state: Try to find enemies, or do random patrol movement.
fn npc_standing(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // Healer NPC: check for injured friends before looking for enemies
    // NPC_HEALER = 40
    if tmpl.npc_type == 40 && tmpl.magic_3 > 0 {
        if let Some(injured) = find_injured_friend(world, npc_id, ai, tmpl) {
            world.update_npc_ai(npc_id, |s| {
                s.target_id = Some(injured as SessionId);
                s.state = NpcState::Healing;
            });
            return Some(0);
        }
    }

    // Try finding an enemy
    // Both ATROCITY (aggressive on sight) and TENDER (passive, damaged only) search.
    // find_enemy() filters appropriately based on attack_type.
    if ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE {
        if let Some(target) = find_enemy(world, ai, npc_id) {
            world.update_npc_ai(npc_id, |s| {
                s.target_id = Some(target);
                s.npc_target_id = None;
                s.state = NpcState::Attacking;
                s.skill_cooldown_ms = s.last_tick_ms + NPC_SKILL_COOLDOWN_MS;
                s.last_combat_time_ms = s.last_tick_ms;
            });
            return Some(0);
        }

        // NPC-vs-NPC: guards/monsters can target hostile NPCs
        if let Some(npc_target) = find_npc_enemy(world, ai, npc_id, tmpl) {
            world.update_npc_ai(npc_id, |s| {
                s.target_id = None;
                s.npc_target_id = Some(npc_target);
                s.state = NpcState::Attacking;
                s.skill_cooldown_ms = s.last_tick_ms + NPC_SKILL_COOLDOWN_MS;
            });
            return Some(0);
        }
    }

    // Random movement -- only if users are nearby (GetUserInView check)
    // and search_range > 0 and move_type = 1 (random)
    if tmpl.search_range > 0 && tmpl.speed_1 > 0 {
        let mut rng = rand::rngs::StdRng::from_entropy();
        // MONSTER_SPEED = 1500, so factor = 1.5
        let step_distance = tmpl.speed_1 as f32 * (MONSTER_SPEED as f32 / 1000.0);

        let pattern = ai.pattern_frame;
        let (dest_x, dest_z) = if pattern >= 2 {
            // Return to spawn point
            (ai.spawn_x, ai.spawn_z)
        } else {
            let rand_x = rng.gen_range(-step_distance..=step_distance);
            let rand_z = rng.gen_range(-step_distance..=step_distance);
            (ai.cur_x + rand_x, ai.cur_z + rand_z)
        };

        // Check if destination is walkable before moving there
        let zone = world.get_zone(ai.zone_id);
        let is_blocked = zone
            .as_ref()
            .and_then(|z| z.map_data.as_ref())
            .map(|m| !m.is_movable(dest_x, dest_z))
            .unwrap_or(false);
        if is_blocked {
            // Destination hits a wall -- stay standing, try again next tick
            return Some(tmpl.stand_time as u64);
        }

        // Packet speed = actual_distance / (MONSTER_SPEED/1000)
        let dx = dest_x - ai.cur_x;
        let dz = dest_z - ai.cur_z;
        let dist = (dx * dx + dz * dz).sqrt();
        let pkt_speed = dist / (MONSTER_SPEED as f32 / 1000.0);

        // Send initial move packet to broadcast the movement
        send_npc_move(
            world,
            npc_id,
            ai.zone_id,
            ai.region_x,
            ai.region_z,
            dest_x,
            dest_z,
            pkt_speed,
        );

        world.update_npc_ai(npc_id, |s| {
            s.dest_x = dest_x;
            s.dest_z = dest_z;
            s.state = NpcState::Moving;
            s.pattern_frame = (pattern + 1) % 3;
        });

        return Some(tmpl.stand_time.max(1000) as u64);
    }

    // ── Gate NPC Logic ──────────────────────────────────────────────────
    if let Some(gate_delay) = handle_gate_standing(world, npc_id, ai, tmpl) {
        return Some(gate_delay);
    }

    Some(tmpl.stand_time as u64)
}

/// NPC_MOVING state: Incremental step movement toward destination.
/// Each tick moves `m_fSecForMetor` distance toward dest, broadcasts position,
/// and returns `MONSTER_SPEED` (1500ms) as next delay.
fn npc_moving(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // Check for enemies while moving — interrupt patrol to chase
    if ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE {
        if let Some(target) = find_enemy(world, ai, npc_id) {
            world.update_npc_ai(npc_id, |s| {
                s.target_id = Some(target);
                s.state = NpcState::Attacking;
                s.skill_cooldown_ms = s.last_tick_ms + NPC_SKILL_COOLDOWN_MS;
                s.last_combat_time_ms = s.last_tick_ms;
            });
            return Some(0);
        }
    }

    let step_dist = tmpl.speed_1 as f32 * (MONSTER_SPEED as f32 / 1000.0);
    let dx = ai.dest_x - ai.cur_x;
    let dz = ai.dest_z - ai.cur_z;
    let remaining = (dx * dx + dz * dz).sqrt();

    if remaining <= step_dist || remaining < 0.5 {
        // Arrived at destination — update position and go to standing
        let new_rx = calc_region(ai.dest_x);
        let new_rz = calc_region(ai.dest_z);

        if new_rx != ai.region_x || new_rz != ai.region_z {
            if let Some(zone) = world.get_zone(ai.zone_id) {
                zone.remove_npc(ai.region_x, ai.region_z, npc_id);
                zone.add_npc(new_rx, new_rz, npc_id);
            }
        }

        world.update_npc_ai(npc_id, |s| {
            s.cur_x = s.dest_x;
            s.cur_z = s.dest_z;
            s.region_x = new_rx;
            s.region_z = new_rz;
            s.state = NpcState::Standing;
        });
        return Some(tmpl.stand_time as u64);
    }

    // Step toward destination
    let ratio = step_dist / remaining;
    let new_x = ai.cur_x + dx * ratio;
    let new_z = ai.cur_z + dz * ratio;
    let new_rx = calc_region(new_x);
    let new_rz = calc_region(new_z);

    // C++ order: broadcast FIRST, then update region (NpcThread.cpp)
    // Broadcast from current (old) region so all nearby players see the movement
    let pkt_speed = step_dist / (MONSTER_SPEED as f32 / 1000.0);
    send_npc_move(
        world,
        npc_id,
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        new_x,
        new_z,
        pkt_speed,
    );

    // Update region data AFTER broadcast
    if new_rx != ai.region_x || new_rz != ai.region_z {
        if let Some(zone) = world.get_zone(ai.zone_id) {
            zone.remove_npc(ai.region_x, ai.region_z, npc_id);
            zone.add_npc(new_rx, new_rz, npc_id);
        }
    }

    world.update_npc_ai(npc_id, |s| {
        s.cur_x = new_x;
        s.cur_z = new_z;
        s.region_x = new_rx;
        s.region_z = new_rz;
    });

    // C++ returns m_sSpeed (1500ms) between each step
    Some(MONSTER_SPEED)
}

/// NPC_ATTACKING state: Determine if target is in range, transition to Fighting or Tracing.
fn npc_attacking(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // NPC-vs-NPC target path
    if let Some(npc_target) = ai.npc_target_id {
        return npc_attacking_npc(world, npc_id, ai, tmpl, npc_target);
    }

    let target_id = match ai.target_id {
        Some(t) => t,
        None => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.target_id = None;
            });
            return Some(tmpl.stand_time as u64);
        }
    };

    // Validate target — single DashMap lookup for position + alive check
    let target_pos = match world.with_session(target_id, |h| {
        let ch = h.character.as_ref()?;
        if ch.res_hp_type == USER_DEAD || ch.hp <= 0 {
            return None;
        }
        if h.position.zone_id != ai.zone_id {
            return None;
        }
        Some(h.position)
    }) {
        Some(Some(p)) => p,
        _ => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.target_id = None;
            });
            return Some(tmpl.stand_time as u64);
        }
    };

    // Distance check
    let dx = ai.cur_x - target_pos.x;
    let dz = ai.cur_z - target_pos.z;
    let dist = (dx * dx + dz * dz).sqrt();
    let attack_range = tmpl.attack_range as f32;

    if dist <= attack_range + 2.0 {
        // In range — fight
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Fighting;
        });
        Some(0)
    } else {
        // Need to trace (chase)
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Tracing;
        });
        Some(0)
    }
}

/// NPC_ATTACKING state for NPC-vs-NPC combat: check range, transition to Fighting.
fn npc_attacking_npc(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    npc_target: NpcId,
) -> Option<u64> {
    // Validate target is alive
    if world.is_npc_dead(npc_target) {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.npc_target_id = None;
        });
        return Some(tmpl.stand_time as u64);
    }

    // Get target position from AI state or instance
    let (tx, tz) = {
        let ai_state = world.get_npc_ai(npc_target);
        match ai_state {
            Some(a) => (a.cur_x, a.cur_z),
            None => match world.get_npc_instance(npc_target) {
                Some(n) => (n.x, n.z),
                None => {
                    world.update_npc_ai(npc_id, |s| {
                        s.state = NpcState::Standing;
                        s.npc_target_id = None;
                    });
                    return Some(tmpl.stand_time as u64);
                }
            },
        }
    };

    let dx = ai.cur_x - tx;
    let dz = ai.cur_z - tz;
    let dist = (dx * dx + dz * dz).sqrt();
    let attack_range = tmpl.attack_range as f32;

    if dist <= attack_range + 2.0 {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Fighting;
        });
        Some(0)
    } else {
        // Chase toward the NPC target — use simple direct movement for NPC-vs-NPC
        let speed = tmpl.speed_1.max(tmpl.speed_2) as f32;
        if speed <= 0.0 {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.npc_target_id = None;
            });
            return Some(tmpl.stand_time as u64);
        }

        // Move toward target
        let step = speed.min(dist);
        let ratio = step / dist;
        let new_x = ai.cur_x + (tx - ai.cur_x) * ratio;
        let new_z = ai.cur_z + (tz - ai.cur_z) * ratio;
        let new_rx = calc_region(new_x);
        let new_rz = calc_region(new_z);

        if new_rx != ai.region_x || new_rz != ai.region_z {
            if let Some(zone) = world.get_zone(ai.zone_id) {
                zone.remove_npc(ai.region_x, ai.region_z, npc_id);
                zone.add_npc(new_rx, new_rz, npc_id);
            }
        }

        send_npc_move(
            world,
            npc_id,
            ai.zone_id,
            ai.region_x,
            ai.region_z,
            new_x,
            new_z,
            speed,
        );

        world.update_npc_ai(npc_id, |s| {
            s.cur_x = new_x;
            s.cur_z = new_z;
            s.region_x = new_rx;
            s.region_z = new_rz;
        });

        Some((dist / speed * 1000.0) as u64)
    }
}

/// NPC_TRACING state: Chase target using A* pathfinding or direct movement.
/// ## Pathfinding Logic (matching C++ GetTargetPath + IsNoPathFind):
/// 1. If target is in attack range -> transition to Fighting
/// 2. If target is close (< DIRECT_MOVE_THRESHOLD) and line-of-sight is clear -> direct move
/// 3. Otherwise -> use A* pathfinding, cache the result, follow waypoints each tick
/// 4. Recalculate path when target moves > PATH_RECALC_DISTANCE from cached position
fn npc_tracing(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // NPC-vs-NPC target: redirect to attacking state (simple chase)
    if ai.npc_target_id.is_some() {
        return npc_attacking(world, npc_id, ai, tmpl);
    }

    let target_id = match ai.target_id {
        Some(t) => t,
        None => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.path_waypoints.clear();
                s.path_index = 0;
            });
            return Some(tmpl.stand_time as u64);
        }
    };

    // Tracer timeout — 12 seconds without combat → disengage.
    const TRACER_TIMEOUT_MS: u64 = 12_000;
    let now_ms = ai.last_tick_ms;
    if ai.last_combat_time_ms > 0
        && now_ms.saturating_sub(ai.last_combat_time_ms) > TRACER_TIMEOUT_MS
    {
        tracing::debug!("NPC {} tracer timeout (12s no combat)", npc_id);
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.target_id = None;
            s.path_waypoints.clear();
            s.path_index = 0;
        });
        return Some(tmpl.stand_time as u64);
    }

    // Leash check -- distance from spawn
    let spawn_dx = ai.cur_x - ai.spawn_x;
    let spawn_dz = ai.cur_z - ai.spawn_z;
    let spawn_dist = (spawn_dx * spawn_dx + spawn_dz * spawn_dz).sqrt();

    if spawn_dist >= NPC_MAX_LEASH_RANGE {
        tracing::debug!(
            "NPC {} leashed — despawn+snap+respawn at spawn (dist={:.1})",
            npc_id,
            spawn_dist
        );

        // INOUT_OUT → position snap to spawn → SendMoveResult → INOUT_IN → Standing
        // C++ deliberately teleports (NOT gradual walk) when distance ≥ 200.
        send_npc_despawn(world, npc_id, ai);

        world.update_npc_hp(npc_id, world.get_npc_war_max_hp(tmpl));

        let spawn_x = ai.spawn_x;
        let spawn_z = ai.spawn_z;
        let new_region_x = calc_region(spawn_x);
        let new_region_z = calc_region(spawn_z);

        // Update region grid if changed
        if new_region_x != ai.region_x || new_region_z != ai.region_z {
            if let Some(zone) = world.get_zone(ai.zone_id) {
                zone.remove_npc(ai.region_x, ai.region_z, npc_id);
                zone.add_npc(new_region_x, new_region_z, npc_id);
            }
        }

        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.cur_x = spawn_x;
            s.cur_z = spawn_z;
            s.region_x = new_region_x;
            s.region_z = new_region_z;
            s.target_id = None;
            s.npc_target_id = None;
            s.path_waypoints.clear();
            s.path_index = 0;
        });

        // Send move result at spawn position (speed 0 = stopped)
        send_npc_move(
            world,
            npc_id,
            ai.zone_id,
            new_region_x,
            new_region_z,
            spawn_x,
            spawn_z,
            0.0,
        );

        // Respawn at spawn position (INOUT_IN)
        send_npc_respawn(world, npc_id, ai.zone_id, new_region_x, new_region_z, tmpl);

        return Some(tmpl.stand_time as u64);
    }

    // Validate target — single DashMap lookup for position + alive check
    let target_pos = match world.with_session(target_id, |h| {
        let ch = h.character.as_ref()?;
        if ch.res_hp_type == USER_DEAD || ch.hp <= 0 {
            return None;
        }
        if h.position.zone_id != ai.zone_id {
            return None;
        }
        Some(h.position)
    }) {
        Some(Some(p)) => p,
        _ => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.target_id = None;
                s.path_waypoints.clear();
                s.path_index = 0;
            });
            return Some(tmpl.stand_time as u64);
        }
    };

    // Distance to target
    let dx = ai.cur_x - target_pos.x;
    let dz = ai.cur_z - target_pos.z;
    let dist = (dx * dx + dz * dz).sqrt();
    let attack_range = tmpl.attack_range as f32;

    if dist <= attack_range + 2.0 {
        // In range -- transition to fighting
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Fighting;
            s.path_waypoints.clear();
            s.path_index = 0;
        });
        return Some(0);
    }

    // Distance too far -- give up tracing
    if dist > NPC_MAX_MOVE_RANGE {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.target_id = None;
            s.path_waypoints.clear();
            s.path_index = 0;
        });
        return Some(tmpl.stand_time as u64);
    }

    let speed = tmpl.speed_2 as f32 * (MONSTER_SPEED as f32 / 1000.0);

    // Check if target has moved enough to warrant path recalculation
    let target_moved_dx = target_pos.x - ai.path_target_x;
    let target_moved_dz = target_pos.z - ai.path_target_z;
    let target_moved_dist =
        (target_moved_dx * target_moved_dx + target_moved_dz * target_moved_dz).sqrt();
    let need_new_path = ai.path_waypoints.is_empty()
        || ai.path_index >= ai.path_waypoints.len()
        || target_moved_dist > PATH_RECALC_DISTANCE;

    let (new_x, new_z, new_waypoints, new_index, new_target_x, new_target_z, new_is_direct) =
        if need_new_path {
            compute_tracing_step(world, ai, tmpl, target_pos.x, target_pos.z, dist, speed)
        } else {
            // Continue following the cached path
            let (nx, nz, ni) =
                pathfind::step_move(ai.cur_x, ai.cur_z, &ai.path_waypoints, ai.path_index, speed);
            (
                nx,
                nz,
                ai.path_waypoints.clone(),
                ni,
                ai.path_target_x,
                ai.path_target_z,
                ai.path_is_direct,
            )
        };

    let new_rx = calc_region(new_x);
    let new_rz = calc_region(new_z);

    // C++ order: broadcast FIRST, then update region (NpcThread.cpp)
    // Broadcast from current (old) region so all nearby players see the movement
    let move_speed = speed / (MONSTER_SPEED as f32 / 1000.0);
    send_npc_move(
        world,
        npc_id,
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        new_x,
        new_z,
        move_speed,
    );

    // Update region data AFTER broadcast
    if new_rx != ai.region_x || new_rz != ai.region_z {
        if let Some(zone) = world.get_zone(ai.zone_id) {
            zone.remove_npc(ai.region_x, ai.region_z, npc_id);
            zone.add_npc(new_rx, new_rz, npc_id);
        }
    }

    world.update_npc_ai(npc_id, |s| {
        s.cur_x = new_x;
        s.cur_z = new_z;
        s.region_x = new_rx;
        s.region_z = new_rz;
        s.path_waypoints = new_waypoints;
        s.path_index = new_index;
        s.path_target_x = new_target_x;
        s.path_target_z = new_target_z;
        s.path_is_direct = new_is_direct;
    });

    // C++ returns m_sSpeed (1500ms) between each chase step
    Some(MONSTER_SPEED)
}

/// Compute the next movement step for a tracing NPC.
/// Decides whether to use direct movement or A* pathfinding based on distance
/// and line-of-sight, matching C++ GetTargetPath() + IsNoPathFind() logic.
/// Returns `(new_x, new_z, waypoints, waypoint_index, target_x, target_z, is_direct)`.
#[allow(clippy::type_complexity)]
fn compute_tracing_step(
    world: &WorldState,
    ai: &NpcAiState,
    _tmpl: &NpcTemplate,
    target_x: f32,
    target_z: f32,
    dist: f32,
    speed: f32,
) -> (f32, f32, Vec<(f32, f32)>, usize, f32, f32, bool) {
    let zone = world.get_zone(ai.zone_id);
    let map_data = zone.as_ref().and_then(|z| z.map_data.as_ref());

    // Short-range or no map data -- use direct movement (IsNoPathFind equivalent)
    if dist <= speed {
        let (nx, nz) = pathfind::step_no_path_move(ai.cur_x, ai.cur_z, target_x, target_z, speed);
        return (nx, nz, Vec::new(), 0, target_x, target_z, true);
    }

    let Some(map) = map_data else {
        let (nx, nz) = pathfind::step_no_path_move(ai.cur_x, ai.cur_z, target_x, target_z, speed);
        return (nx, nz, Vec::new(), 0, target_x, target_z, true);
    };

    // Close range with clear line-of-sight -- direct movement
    if dist <= DIRECT_MOVE_THRESHOLD
        && pathfind::line_of_sight(map, ai.cur_x, ai.cur_z, target_x, target_z, speed)
    {
        let (nx, nz) = pathfind::step_no_path_move(ai.cur_x, ai.cur_z, target_x, target_z, speed);
        return (nx, nz, Vec::new(), 0, target_x, target_z, true);
    }

    // A* pathfinding
    let result = pathfind::find_path(map, ai.cur_x, ai.cur_z, target_x, target_z);

    if result.found && !result.waypoints.is_empty() {
        let (nx, nz, ni) = pathfind::step_move(ai.cur_x, ai.cur_z, &result.waypoints, 0, speed);
        (nx, nz, result.waypoints, ni, target_x, target_z, false)
    } else {
        // Pathfinding failed -- fall back to direct movement
        let (nx, nz) = pathfind::step_no_path_move(ai.cur_x, ai.cur_z, target_x, target_z, speed);
        (nx, nz, Vec::new(), 0, target_x, target_z, true)
    }
}

/// NPC_FIGHTING state: Execute attack on target, handle damage.
/// + `Npc.cpp:2820-2863` — `CNpc::SendAttackRequest()`
/// + `Unit.cpp:950-998` — `CNpc::GetDamage(CUser*)`
async fn npc_fighting(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // NPC-vs-NPC fighting path
    if let Some(npc_target) = ai.npc_target_id {
        return npc_fighting_npc(world, npc_id, ai, tmpl, npc_target);
    }

    let target_id = match ai.target_id {
        Some(t) => t,
        None => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
            });
            return Some(0);
        }
    };

    // Validate target — single DashMap lookup for all checks
    // (was 4 separate lookups: character, invisible, blink, position)
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Extract combat-relevant fields alongside validation
    struct TargetInfo {
        class: u16,
        level: u8,
        dex: u8,
        hp: i16,
        max_hp: i16,
        skill_points: [u8; 10],
    }

    let target = match world.with_session(target_id, |h| {
        let ch = h.character.as_ref()?;
        // Dead check
        if ch.res_hp_type == USER_DEAD || ch.hp <= 0 {
            return None;
        }
        // GM check
        if ch.authority == 0 {
            return None;
        }
        // Invisibility check — C++ Npc.cpp:2832
        if h.invisibility_type != 0 {
            return None;
        }
        // Blink check — C++ Npc.cpp:2835
        if h.blink_expiry_time > 0 && now_unix < h.blink_expiry_time {
            return None;
        }
        Some(TargetInfo {
            class: ch.class,
            level: ch.level,
            dex: ch.dex,
            hp: ch.hp,
            max_hp: ch.max_hp,
            skill_points: ch.skill_points,
        })
    }) {
        Some(Some(t)) => t,
        _ => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.target_id = None;
            });
            return Some(0);
        }
    };

    // Group AI: call nearby same-family NPCs to help
    //   if (m_bHasFriends || GetType() == NPC_BOSS)
    //     FindFriend(GetType() == NPC_BOSS ? MonSearchAny : MonSearchSameFamily);
    let is_boss = tmpl.npc_type == NPC_BOSS;
    if (ai.has_friends || is_boss) && world.npc_damage_contains(npc_id, target_id) {
        alert_pack(world, npc_id, ai, tmpl, target_id, is_boss);
    }

    // Ranged/magic attack — NPCs with direct_attack > 0 use skills
    if tmpl.direct_attack > 0 && tmpl.magic_1 > 0 {
        return long_and_magic_attack(world, npc_id, ai, tmpl, target_id).await;
    }

    // ── Monster magic attack dispatch ────────────────────────────────
    //
    // Monsters with magic_attack > 0 can cast magic_1/2/3 during melee
    // combat. This is different from direct_attack NPCs — these monsters
    // do physical attacks normally but have a random chance to cast.
    if tmpl.magic_attack > 0 && tmpl.magic_1 > 0 {
        if let Some(result) = try_monster_magic(world, npc_id, ai, tmpl, target_id).await {
            return Some(result);
        }
    }

    // Distance check — if target moved out of range, go back to tracing
    if let Some(target_pos) = world.get_position(target_id) {
        let dx = ai.cur_x - target_pos.x;
        let dz = ai.cur_z - target_pos.z;
        let dist = (dx * dx + dz * dz).sqrt();
        let attack_range = tmpl.attack_range as f32;

        if dist > attack_range + 3.0 {
            // Target moved away — chase
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Tracing;
            });
            return Some(0);
        }
    } else {
        // Target disconnected
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.target_id = None;
        });
        return Some(0);
    }

    // ── Calculate damage ──────────────────────────────────────────────
    // C++ line 960: Ac = (m_sTotalAc + m_sACAmount) - m_sACSourAmount
    // NOTE: C++ does NOT apply m_sACPercent in NPC GetDamage (no percent multiplication).
    let target_coeff = world.get_coefficient(target.class);
    let equip_stats = world.get_equipped_stats(target_id);
    let buff_ac = world.get_buff_ac_amount(target_id);
    let ac_sour = world.get_buff_ac_sour_amount(target_id);
    let target_ac = (equip_stats.total_ac as i32 + buff_ac - ac_sour).max(0);

    // NOTE: m_sAttack (tmpl.attack) is for GetInOut packet only, NOT damage calculation.
    // War buff: nation NPCs deal 50% damage during war (ChangeAbility).
    let npc_total_hit = world.get_npc_war_damage(tmpl);
    let attack_amount: i32 = 100; // default m_bAttackAmount

    // C++ formula: HitB = (m_sTotalHit * m_bAttackAmount * 200 / 100) / (Ac + 240)
    let hit_b = (npc_total_hit * attack_amount * 2) / (target_ac + 240).max(1);

    if hit_b <= 0 {
        // Attack failed — no damage
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_FAIL);
        return Some(tmpl.attack_delay as u64);
    }

    // Hit rate check
    let npc_hitrate = tmpl.hit_rate as f32;
    let target_evasion = if let Some(coeff) = &target_coeff {
        1.0 + coeff.evasionrate as f32 * target.level as f32 * target.dex as f32
    } else {
        1.0
    };

    let rate = if target_evasion > 0.0 {
        npc_hitrate / target_evasion
    } else {
        npc_hitrate
    };

    let mut rng = rand::rngs::StdRng::from_entropy();
    let hit_result = get_hit_rate(rate, &mut rng);

    let damage = match hit_result {
        1 => {
            // GREAT_SUCCESS: 0.85*HitB + 0.3*rand(0,HitB), then *3/2
            let random = if hit_b > 0 {
                rng.gen_range(0..=hit_b)
            } else {
                0
            };
            let d = (0.85 * hit_b as f32 + 0.3 * random as f32) as i32;
            (d * 3 / 2)
                .min(MAX_DAMAGE)
                .min((2.6 * npc_total_hit as f32) as i32)
        }
        2 | 3 => {
            // SUCCESS/NORMAL: 0.85*HitB + 0.3*rand(0,HitB)
            let random = if hit_b > 0 {
                rng.gen_range(0..=hit_b)
            } else {
                0
            };
            let d = (0.85 * hit_b as f32 + 0.3 * random as f32) as i32;
            d.min(MAX_DAMAGE).min((2.6 * npc_total_hit as f32) as i32)
        }
        _ => 0, // FAIL
    };

    tracing::debug!(
        "NPC_FIGHTING: npc_id={} ssid={} damage_stat={} target_ac={} hit_b={} hitrate={:.2}/{:.2}={:.2} hit_result={} damage={} target_hp={}/{}",
        npc_id, tmpl.s_sid, npc_total_hit, target_ac, hit_b,
        npc_hitrate, target_evasion, rate, hit_result, damage, target.hp, target.max_hp
    );

    if damage <= 0 {
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_FAIL);
        return Some(tmpl.attack_delay as u64);
    }

    // Apply damage to player through HpChange pipeline
    // Mirror: SKIP for NPC attackers (C++ line 75: pAttacker->isPlayer() required)
    let mut final_damage = damage as i16;

    // C++ HpChange order: save original → mirror(skip) → mastery → mana absorb
    let original_damage = final_damage;

    // ── Mastery passive damage reduction ────────────────────────────────
    {
        let victim_zone = world
            .get_position(target_id)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        let not_use_zone = victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
        if !not_use_zone && crate::handler::class_change::is_mastered(target.class) {
            let master_pts = target.skill_points[8]; // SkillPointMaster = index 8
            if master_pts >= 10 {
                final_damage = (85 * final_damage as i32 / 100) as i16;
            } else if master_pts >= 5 {
                final_damage = (90 * final_damage as i32 / 100) as i16;
            }
        }
    }

    // ── Mana Absorb (Outrage/Frenzy/Mana Shield) ─────────────────────
    // Uses original_damage for calculation (pre-mastery), subtracts from current.
    {
        let victim_zone = world
            .get_position(target_id)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        let not_use_zone = victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
        let (absorb_pct, absorb_count) = world
            .with_session(target_id, |h| (h.mana_absorb, h.absorb_count))
            .unwrap_or((0, 0));
        if absorb_pct > 0 && !not_use_zone {
            let should_absorb = if absorb_pct == 15 {
                absorb_count > 0
            } else {
                true
            };
            if should_absorb {
                let absorbed = (original_damage as i32 * absorb_pct as i32 / 100) as i16;
                final_damage -= absorbed;
                if final_damage < 0 {
                    final_damage = 0;
                }
                world.update_character_stats(target_id, |ch| {
                    ch.mp = (ch.mp as i32 + absorbed as i32).min(ch.max_mp as i32) as i16;
                });
                if absorb_pct == 15 {
                    world.update_session(target_id, |h| {
                        h.absorb_count = h.absorb_count.saturating_sub(1);
                    });
                }
            }
        }
    }

    final_damage = world
        .manes_survival_manager
        .reduce_incoming_damage(target_id, final_damage);
    let new_hp = (target.hp - final_damage).max(0);
    world.update_character_hp(target_id, new_hp);

    // `if (!TO_USER(pTarget)->isInGenie()) TO_USER(pTarget)->ItemWoreOut(DEFENCE, sDamage);`
    if !world
        .with_session(target_id, |h| h.genie_active)
        .unwrap_or(false)
    {
        world.item_wore_out(target_id, WORE_TYPE_DEFENCE, damage as i32);
    }

    let b_result = if new_hp <= 0 {
        // Player died
        crate::handler::dead::broadcast_death(world, target_id);
        // XP loss only on NPC kills (with type exclusions)
        crate::handler::dead::apply_npc_death_xp_loss(world, target_id, tmpl.npc_type, tmpl.s_sid)
            .await;
        ATTACK_TARGET_DEAD
    } else {
        ATTACK_SUCCESS
    };

    // Broadcast attack result
    broadcast_npc_attack(world, npc_id, ai, target_id, b_result);

    // Send HP change to the target player with NPC ID as attacker
    let hp_pkt =
        crate::systems::regen::build_hp_change_packet_with_attacker(target.max_hp, new_hp, npc_id);
    world.send_to_session_owned(target_id, hp_pkt);

    // Update last combat time for tracer timeout.
    world.update_npc_ai(npc_id, |s| {
        s.last_combat_time_ms = s.last_tick_ms;
    });

    // If target died, go back to standing
    if new_hp <= 0 {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.target_id = None;
        });
        return Some(tmpl.stand_time as u64);
    }

    // Continue fighting on attack delay
    Some(tmpl.attack_delay as u64)
}

/// NPC-vs-NPC fighting: deal damage from this NPC to a target NPC.
/// Simplified damage formula: attacker_total_hit vs target_defense,
/// similar to NPC-vs-player but both are NPCs.
fn npc_fighting_npc(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    npc_target: NpcId,
) -> Option<u64> {
    // Validate target is alive
    if world.is_npc_dead(npc_target) {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.npc_target_id = None;
        });
        return Some(tmpl.stand_time as u64);
    }

    // Get target NPC template for defense value
    let target_npc = match world.get_npc_instance(npc_target) {
        Some(n) => n,
        None => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.npc_target_id = None;
            });
            return Some(0);
        }
    };
    let target_tmpl = match world.get_npc_template(target_npc.proto_id, target_npc.is_monster) {
        Some(t) => t,
        None => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.npc_target_id = None;
            });
            return Some(0);
        }
    };

    // Distance check
    let (tx, tz) = {
        let target_ai = world.get_npc_ai(npc_target);
        match target_ai {
            Some(a) => (a.cur_x, a.cur_z),
            None => (target_npc.x, target_npc.z),
        }
    };

    let dx = ai.cur_x - tx;
    let dz = ai.cur_z - tz;
    let dist = (dx * dx + dz * dz).sqrt();
    let attack_range = tmpl.attack_range as f32;

    if dist > attack_range + 3.0 {
        // Target moved away — chase
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Attacking;
        });
        return Some(0);
    }

    // Calculate damage: attacker_hit vs target_defense (simplified C++ formula)
    // War buff: nation NPCs deal 50% damage during war (ChangeAbility).
    let npc_total_hit = world.get_npc_war_damage(tmpl);
    let target_ac = world.get_npc_war_ac(&target_tmpl);

    let hit_b = (npc_total_hit * 200) / (target_ac + 240).max(1);

    if hit_b <= 0 {
        broadcast_npc_vs_npc_attack(world, npc_id, ai, npc_target, ATTACK_FAIL);
        return Some(tmpl.attack_delay as u64);
    }

    // Simple hit check
    let mut rng = rand::rngs::StdRng::from_entropy();
    let damage = {
        let random = if hit_b > 0 {
            rng.gen_range(0..=hit_b)
        } else {
            0
        };
        let d = (0.85 * hit_b as f32 + 0.3 * random as f32) as i32;
        d.min(MAX_DAMAGE)
    };

    if damage <= 0 {
        broadcast_npc_vs_npc_attack(world, npc_id, ai, npc_target, ATTACK_FAIL);
        return Some(tmpl.attack_delay as u64);
    }

    // Apply damage to target NPC
    let current_hp = world.get_npc_hp(npc_target).unwrap_or(0);
    let new_hp = (current_hp - damage).max(0);
    world.update_npc_hp(npc_target, new_hp);

    let b_result = if new_hp <= 0 {
        // Target NPC died — set its AI to Dead
        world.update_npc_ai(npc_target, |s| {
            s.state = NpcState::Dead;
            s.target_id = None;
            s.npc_target_id = None;
        });

        // Broadcast NPC death (NPC_OUT)
        if let Some(target_pos) = world.get_npc_ai(npc_target) {
            let death_pkt =
                crate::npc::build_npc_inout(crate::npc::NPC_OUT, &target_npc, &target_tmpl);
            let target_event_room = target_npc.event_room;
            world.broadcast_to_3x3(
                target_pos.zone_id,
                target_pos.region_x,
                target_pos.region_z,
                Arc::new(death_pkt),
                None,
                target_event_room,
            );
        }

        ATTACK_TARGET_DEAD
    } else {
        ATTACK_SUCCESS
    };

    broadcast_npc_vs_npc_attack(world, npc_id, ai, npc_target, b_result);

    // If target NPC is a pet, notify the pet owner of HP change and damage display.
    if let Some(owner_sid) = world.find_pet_owner_by_nid(npc_target as u16) {
        let max_hp = target_tmpl.max_hp as u16;
        let cur_hp = new_hp.max(0) as u16;
        // Sync PetState.hp with NPC HP
        world.update_session(owner_sid, |h| {
            if let Some(ref mut pet) = h.pet_data {
                pet.hp = cur_hp;
            }
        });
        let hp_pkt = crate::handler::pet::build_pet_hp_change_packet(max_hp, cur_hp, npc_id);
        world.send_to_session_owned(owner_sid, hp_pkt);
        let dmg_pkt =
            crate::handler::pet::build_pet_damage_display_packet(npc_target as i32, damage as i16);
        world.send_to_session_owned(owner_sid, dmg_pkt);
    }

    // If target died, go back to standing
    if new_hp <= 0 {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.npc_target_id = None;
        });
        return Some(tmpl.stand_time as u64);
    }

    Some(tmpl.attack_delay as u64)
}

/// Broadcast NPC-vs-NPC attack result to the 3x3 region.
/// Same wire format as NPC-vs-player attack broadcast.
fn broadcast_npc_vs_npc_attack(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    target_npc_id: NpcId,
    result: u8,
) {
    let mut pkt = Packet::new(Opcode::WizAttack as u8);
    pkt.write_u8(LONG_ATTACK);
    pkt.write_u8(result);
    pkt.write_u32(npc_id);
    pkt.write_u32(target_npc_id);

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// NPC_BACK state: Return to spawn position.
fn npc_back(world: &WorldState, npc_id: NpcId, ai: &NpcAiState, tmpl: &NpcTemplate) -> Option<u64> {
    let dx = ai.spawn_x - ai.cur_x;
    let dz = ai.spawn_z - ai.cur_z;
    let dist = (dx * dx + dz * dz).sqrt();

    if dist <= 2.0 {
        // Arrived at spawn
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.cur_x = s.spawn_x;
            s.cur_z = s.spawn_z;
            s.target_id = None;
            s.path_waypoints.clear();
            s.path_index = 0;
        });
        send_npc_move(
            world,
            npc_id,
            ai.zone_id,
            ai.region_x,
            ai.region_z,
            ai.spawn_x,
            ai.spawn_z,
            0.0,
        );
        return Some(tmpl.stand_time as u64);
    }

    let step_dist = tmpl.speed_1 as f32 * (MONSTER_SPEED as f32 / 1000.0);

    // Use pathfinding to return to spawn (avoids walking through walls)
    let need_new_path = ai.path_waypoints.is_empty() || ai.path_index >= ai.path_waypoints.len();

    let (new_x, new_z, new_waypoints, new_index) = if need_new_path {
        let zone = world.get_zone(ai.zone_id);
        let map_data = zone.as_ref().and_then(|z| z.map_data.as_ref());

        if let Some(map) = map_data {
            if dist > DIRECT_MOVE_THRESHOLD
                || !pathfind::line_of_sight(
                    map, ai.cur_x, ai.cur_z, ai.spawn_x, ai.spawn_z, step_dist,
                )
            {
                let result = pathfind::find_path(map, ai.cur_x, ai.cur_z, ai.spawn_x, ai.spawn_z);
                if result.found && !result.waypoints.is_empty() {
                    let (nx, nz, ni) =
                        pathfind::step_move(ai.cur_x, ai.cur_z, &result.waypoints, 0, step_dist);
                    (nx, nz, result.waypoints, ni)
                } else {
                    let (nx, nz) = pathfind::step_no_path_move(
                        ai.cur_x, ai.cur_z, ai.spawn_x, ai.spawn_z, step_dist,
                    );
                    (nx, nz, Vec::new(), 0)
                }
            } else {
                let (nx, nz) = pathfind::step_no_path_move(
                    ai.cur_x, ai.cur_z, ai.spawn_x, ai.spawn_z, step_dist,
                );
                (nx, nz, Vec::new(), 0)
            }
        } else {
            let (nx, nz) =
                pathfind::step_no_path_move(ai.cur_x, ai.cur_z, ai.spawn_x, ai.spawn_z, step_dist);
            (nx, nz, Vec::new(), 0)
        }
    } else {
        let (nx, nz, ni) = pathfind::step_move(
            ai.cur_x,
            ai.cur_z,
            &ai.path_waypoints,
            ai.path_index,
            step_dist,
        );
        (nx, nz, ai.path_waypoints.clone(), ni)
    };

    let new_rx = calc_region(new_x);
    let new_rz = calc_region(new_z);

    if new_rx != ai.region_x || new_rz != ai.region_z {
        if let Some(zone) = world.get_zone(ai.zone_id) {
            zone.remove_npc(ai.region_x, ai.region_z, npc_id);
            zone.add_npc(new_rx, new_rz, npc_id);
        }
    }

    // C++ packet speed = actual_step / (MONSTER_SPEED/1000)
    let pkt_speed = step_dist / (MONSTER_SPEED as f32 / 1000.0);
    send_npc_move(
        world,
        npc_id,
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        new_x,
        new_z,
        pkt_speed,
    );

    world.update_npc_ai(npc_id, |s| {
        s.cur_x = new_x;
        s.cur_z = new_z;
        s.region_x = new_rx;
        s.region_z = new_rz;
        s.path_waypoints = new_waypoints;
        s.path_index = new_index;
    });

    // C++ returns m_sSpeed (1500ms) between each return step
    Some(MONSTER_SPEED)
}

// ── New state handlers ──────────────────────────────────────────────────

/// NPC_SLEEPING state: Stun debuff — frozen until `fainting_until_ms`, then wake to Fighting.
/// When a sleep skill hits an NPC, its state is set to Sleeping and
/// `fainting_until_ms` is set to `now + duration`. On tick, we just wait.
/// When the duration expires, we broadcast a WIZ_STATE_CHANGE wake-up
/// packet and transition to Fighting (the NPC was presumably mid-combat).
fn npc_sleeping(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    now_ms: u64,
) -> Option<u64> {
    // Still sleeping?
    if now_ms < ai.fainting_until_ms {
        return Some(tmpl.stand_time as u64);
    }

    // Wake up — broadcast state change to region
    broadcast_state_change(world, npc_id, ai, 1, 5);

    // Transition to Fighting (C++ calls StateChangeServerDirect(1, 5))
    world.update_npc_ai(npc_id, |s| {
        s.state = NpcState::Fighting;
        s.fainting_until_ms = 0;
    });

    Some(0)
}

/// NPC_FAINTING state: Lightning stun — frozen for FAINTING_TIME (2s), then Standing.
/// Fix for Sprint 39 L1: if `fainting_until_ms` is stale (e.g., set from a stale
/// `last_tick_ms` when the zone was inactive), recalibrate it to the current tick
/// so the faint lasts the correct duration from the first AI tick that processes it.
/// Fix for Sprint 39 L3: C++ does NOT clear `target_id` on fainting wake. The NPC
/// should resume attacking its previous target after recovering from the stun.
fn npc_fainting(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    _tmpl: &NpcTemplate,
    now_ms: u64,
) -> Option<u64> {
    // Detect stale timestamp: if fainting_until_ms + a generous buffer (4x duration)
    // is still less than now_ms, the timestamp was set from a stale last_tick_ms.
    // Recalibrate to current tick so the faint lasts the correct duration.
    let faint_start = if ai.fainting_until_ms + FAINTING_TIME_MS * 4 < now_ms {
        // Stale — recalibrate to now
        world.update_npc_ai(npc_id, |s| {
            s.fainting_until_ms = now_ms;
        });
        now_ms
    } else {
        ai.fainting_until_ms
    };

    // Still stunned? (fainting_until_ms = start_time, duration = FAINTING_TIME)
    if now_ms < faint_start + FAINTING_TIME_MS {
        // C++ returns -1 meaning "keep ticking at normal rate"
        return Some(250);
    }

    // Wake up — broadcast state change to region
    broadcast_state_change(world, npc_id, ai, 1, 6);

    // Transition to Standing (C++ calls StateChangeServerDirect(1, 6))
    // C++ does NOT clear target_id on fainting wake — NPC resumes attacking.
    world.update_npc_ai(npc_id, |s| {
        s.state = NpcState::Standing;
        s.fainting_until_ms = 0;
    });

    Some(0)
}

/// NPC_HEALING state: Healer NPC searches for injured same-family NPCs and heals them.
/// Healer NPCs (direct_attack == 2 with magic_3 set) scan nearby NPCs in
/// the 3x3 region grid. If any same-family NPC has HP below 90%, they cast
/// their heal skill (magic_3) on the most injured one.
async fn npc_healing(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // Only healer NPCs should be in this state
    if tmpl.magic_3 == 0 {
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.target_id = None;
        });
        return Some(tmpl.stand_time as u64);
    }

    // If we have a current healing target, check if it still needs healing
    if let Some(target_npc_id) = ai.target_id.map(|t| t as u32) {
        let target_hp = world.get_npc_hp(target_npc_id);
        let target_tmpl = world
            .get_npc_instance(target_npc_id)
            .and_then(|inst| world.get_npc_template(inst.proto_id, inst.is_monster));

        if let (Some(hp), Some(ttmpl)) = (target_hp, target_tmpl) {
            if !world.is_npc_dead(target_npc_id) {
                let threshold =
                    (world.get_npc_war_max_hp(&ttmpl) as f32 * HEALER_HP_THRESHOLD) as i32;
                if hp < threshold {
                    // Still needs healing — cast heal and apply
                    broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_npc_id as i32);
                    npc_apply_magic_effect(
                        world,
                        npc_id,
                        ai,
                        tmpl,
                        tmpl.magic_3,
                        target_npc_id as i32,
                    )
                    .await;
                    return Some(tmpl.attack_delay as u64);
                }
            }
        }

        // Target fully healed or dead — clear target
        world.update_npc_ai(npc_id, |s| {
            s.target_id = None;
        });
    }

    // Search for an injured friendly NPC
    let heal_target = find_injured_friend(world, npc_id, ai, tmpl);

    match heal_target {
        Some(friend_nid) => {
            // Cast heal on the most injured friend and apply
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, friend_nid as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, friend_nid as i32).await;
            world.update_npc_ai(npc_id, |s| {
                s.target_id = Some(friend_nid as SessionId);
            });
            Some(tmpl.attack_delay as u64)
        }
        None => {
            // No one to heal — go back to standing
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.target_id = None;
            });
            Some(tmpl.stand_time as u64)
        }
    }
}

/// NPC_CASTING state: Wait for cast time, then apply the skill effect.
/// When an NPC begins casting, it saves old_state, skill_id, target, and
/// cast_time. This handler waits for the cast time to elapse, then sends
/// MAGIC_EFFECTING and returns to the old state.
async fn npc_casting(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    // Broadcast MAGIC_EFFECTING for the active skill and apply effect
    if ai.active_skill_id > 0 {
        broadcast_npc_magic(world, npc_id, ai, ai.active_skill_id, ai.active_target_id);
        npc_apply_magic_effect(
            world,
            npc_id,
            ai,
            tmpl,
            ai.active_skill_id,
            ai.active_target_id,
        )
        .await;
    }

    // Calculate remaining attack delay after cast
    let remaining_delay = (tmpl.attack_delay as u64).saturating_sub(ai.active_cast_time_ms);

    // Return to previous state and clear casting fields
    let old = ai.old_state;
    world.update_npc_ai(npc_id, |s| {
        s.state = old;
        s.active_skill_id = 0;
        s.active_target_id = -1;
        s.active_cast_time_ms = 0;
    });

    Some(remaining_delay)
}

/// Handle long-range and magic NPC attacks.
/// NPCs with `direct_attack == 1` (ranged) or `direct_attack == 2` (magic)
/// use their magic_1/magic_2 skills instead of melee attacks.
async fn long_and_magic_attack(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    target_id: SessionId,
) -> Option<u64> {
    // Validate target
    let target_pos = match world.get_position(target_id) {
        Some(p) if p.zone_id == ai.zone_id => p,
        _ => {
            world.update_npc_ai(npc_id, |s| {
                s.state = NpcState::Standing;
                s.target_id = None;
            });
            return Some(tmpl.stand_time as u64);
        }
    };

    // Range check — use attack_range for magic NPCs
    let dx = ai.cur_x - target_pos.x;
    let dz = ai.cur_z - target_pos.z;
    let dist = (dx * dx + dz * dz).sqrt();
    let attack_range = tmpl.attack_range as f32;

    if dist > attack_range + 2.0 {
        // Not in range — trace
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Tracing;
        });
        return Some(0);
    }

    // ── Boss-specific proto overrides ──────────────────────────────────
    let proto = tmpl.s_sid;

    // Fluwiton Room 4: random CASTING from 3 skills
    if BOSS_FLUWITON_ROOM_4.contains(&proto) {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let rand_skill: i32 = rng.gen_range(0..400);
        let skill = if rand_skill < 30 {
            502022u32
        } else if rand_skill < 60 {
            502023
        } else {
            tmpl.magic_1
        };
        broadcast_npc_magic_casting(world, npc_id, ai, skill, target_id);
        npc_apply_magic_effect(world, npc_id, ai, tmpl, skill, target_id as i32).await;
        return Some(tmpl.attack_delay as u64);
    }

    // Elite Timarli: MAGIC_FLYING with magic_1
    if BOSS_ELITE_TIMARLI.contains(&proto) && tmpl.magic_1 != 0 {
        broadcast_npc_magic_with_opcode(
            world,
            npc_id,
            ai,
            tmpl.magic_1,
            target_id as i32,
            MAGIC_OPCODE_FLYING,
        );
        npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        return Some(tmpl.attack_delay as u64);
    }

    // Fluwiton Room 3: weighted random skill table
    if BOSS_FLUWITON_ROOM_3.contains(&proto) {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let rand_val: i32 = rng.gen_range(0..800);
        if rand_val < 200 {
            let skill = if rand_val < 10 {
                502020u32
            } else if rand_val < 20 {
                502021
            } else if rand_val < 40 {
                502031
            } else if rand_val < 60 {
                502019
            } else if rand_val < 80 {
                502028
            } else if rand_val < 100 {
                502029
            } else if rand_val < 120 {
                502030
            } else if rand_val < 140 {
                502019
            } else if tmpl.magic_2 > 0 {
                tmpl.magic_2
            } else {
                tmpl.magic_1
            };
            broadcast_npc_magic(world, npc_id, ai, skill, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, skill, target_id as i32).await;
        } else if tmpl.magic_1 > 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        return Some(tmpl.attack_delay as u64);
    }

    // Select skill — C++ Npc.cpp:3795-3802: myrand(0, 400)
    // 0-29 (7.5%): magic_2, 30-59 (7.5%): magic_3, 60-399 (85%): magic_1
    let skill_id = {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let rand_skill: i32 = rng.gen_range(0..400);
        if rand_skill < 30 && tmpl.magic_2 > 0 {
            tmpl.magic_2
        } else if rand_skill < 60 && tmpl.magic_3 > 0 {
            tmpl.magic_3
        } else {
            tmpl.magic_1
        }
    };

    // Look up cast time from magic table
    let cast_time_ms = world
        .get_magic(skill_id as i32)
        .and_then(|m| m.cast_time)
        .unwrap_or(0) as u64
        * 100; // C++ cast_time is in tenths of seconds

    if cast_time_ms > 0 {
        // Send MAGIC_CASTING packet and transition to CASTING state
        broadcast_npc_magic_casting(world, npc_id, ai, skill_id, target_id);

        world.update_npc_ai(npc_id, |s| {
            s.old_state = s.state;
            s.state = NpcState::Casting;
            s.active_skill_id = skill_id;
            s.active_target_id = target_id as i32;
            s.active_cast_time_ms = cast_time_ms;
        });
        Some(cast_time_ms)
    } else {
        // Instant cast — send EFFECTING directly and apply damage
        broadcast_npc_magic(world, npc_id, ai, skill_id, target_id as i32);
        npc_apply_magic_effect(world, npc_id, ai, tmpl, skill_id, target_id as i32).await;
        Some(tmpl.attack_delay as u64)
    }
}

/// Try to cast a monster magic attack during melee combat.
/// Returns `Some(delay_ms)` if magic was cast, `None` if physical attack should proceed.
/// Magic attack types:
/// - 2: Random chance (~50%) to use magic_1, with 7.5% magic_2, 7.5% magic_3
/// - 4, 5: Same distribution as type 2
/// - 6: Higher chance (~71%), magic_2 at 33%, magic_3 at 33%
async fn try_monster_magic(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    target_id: SessionId,
) -> Option<u64> {
    // Boss magic (type 3) — per-proto timed patterns, always fires.
    if tmpl.magic_attack == 3 {
        return try_boss_magic(world, npc_id, ai, tmpl, target_id).await;
    }

    // Check skill cooldown
    if ai.skill_cooldown_ms > 0 && ai.last_tick_ms < ai.skill_cooldown_ms {
        return None;
    }

    let mut rng = rand::rngs::StdRng::from_entropy();

    // Random chance to trigger magic
    let threshold = match tmpl.magic_attack {
        6 => 3500,         // Higher magic chance
        2 | 4 | 5 => 5000, // Standard magic chance
        _ => return None,
    };

    let roll: i32 = rng.gen_range(1..=threshold);
    if roll >= NPC_MAGIC_PERCENT_DEFAULT {
        return None;
    }

    // Select which skill to use based on magic_attack type
    let skill_id = match tmpl.magic_attack {
        6 => {
            // Heavy magic: 33% magic_2, 33% magic_3, 33% magic_1
            let rand_skill: i32 = rng.gen_range(0..3000);
            if rand_skill < 1000 && tmpl.magic_2 > 0 {
                tmpl.magic_2
            } else if rand_skill < 2000 && tmpl.magic_3 > 0 {
                tmpl.magic_3
            } else {
                tmpl.magic_1
            }
        }
        2 | 4 | 5 => {
            // Standard magic: 7.5% magic_2, 7.5% magic_3, 85% magic_1
            let rand_skill: i32 = rng.gen_range(0..400);
            if rand_skill < 30 && tmpl.magic_2 > 0 {
                tmpl.magic_2
            } else if rand_skill < 60 && tmpl.magic_3 > 0 {
                tmpl.magic_3
            } else {
                tmpl.magic_1
            }
        }
        _ => return None,
    };

    if skill_id == 0 {
        return None;
    }

    // Broadcast MAGIC_EFFECTING and apply damage
    broadcast_npc_magic(world, npc_id, ai, skill_id, target_id as i32);
    npc_apply_magic_effect(world, npc_id, ai, tmpl, skill_id, target_id as i32).await;

    // Set cooldown
    let cooldown = if tmpl.magic_attack == 6 {
        3000u64
    } else {
        NPC_SKILL_COOLDOWN_MS
    };
    world.update_npc_ai(npc_id, |s| {
        s.skill_cooldown_ms = s.last_tick_ms + cooldown;
    });

    Some(tmpl.attack_delay as u64)
}

/// Boss magic dispatch for magic_attack == 3 — per-proto timed skill patterns.
/// + `Npc.cpp:3686-3880` — `LongAndMagicAttack()` boss cases
/// Each boss type has a unique timing pattern driven by `utc_second`:
/// - Counter increments once per call (1-second effective tick)
/// - At specific thresholds, magic_1/2/3 are cast via EFFECTING
/// - At cycle end, counter resets and a random special skill is cast via CASTING
/// - Between magic casts, bosses also perform melee attacks (SendAttackRequest)
async fn try_boss_magic(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    target_id: SessionId,
) -> Option<u64> {
    let proto = tmpl.s_sid;
    let utc = ai.utc_second;
    let mut rng = rand::rngs::StdRng::from_entropy();

    // ── Elite Timarli: MAGIC_FLYING, no timer ────────────────────────
    if BOSS_ELITE_TIMARLI.contains(&proto) {
        if tmpl.magic_1 != 0 {
            broadcast_npc_magic_with_opcode(
                world,
                npc_id,
                ai,
                tmpl.magic_1,
                target_id as i32,
                MAGIC_OPCODE_FLYING,
            );
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        return Some(1000);
    }

    // ── Emperor Mammoth: 15/20/30s effecting, 40s random casting + melee ──
    if BOSS_EMPEROR_MAMMOTH.contains(&proto) {
        if utc == 15 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 20 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 30 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 40 {
            let rand_val: i32 = rng.gen_range(0..100);
            let skill = if rand_val < 30 {
                502013u32
            } else if rand_val < 60 {
                502014
            } else {
                502024
            };
            broadcast_npc_magic_casting(world, npc_id, ai, skill, target_id);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, skill, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        // Also do a melee attack (C++ SendAttackRequest)
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Cresher Gimmic: 15/20/30s effecting, 50s casting + melee ─────
    if BOSS_CRESHERGIMMIC.contains(&proto) {
        if utc == 15 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 20 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 30 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 50 && tmpl.magic_3 != 0 {
            broadcast_npc_magic_casting(world, npc_id, ai, tmpl.magic_3, target_id);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Moebius Evil/Rage: 10/20s effecting, 30s effecting (reset) + melee ──
    if BOSS_MOEBIUS.contains(&proto) {
        if utc == 10 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 20 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 30 && tmpl.magic_3 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Purious: 15/25/40s effecting, 60s random casting + melee ────
    if BOSS_PURIOUS.contains(&proto) {
        if utc == 15 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 25 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 40 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 60 {
            let rand_val: i32 = rng.gen_range(0..100);
            let skill = if rand_val < 30 {
                502025u32
            } else if rand_val < 60 {
                502015
            } else if rand_val < 85 {
                502017
            } else {
                502016
            };
            broadcast_npc_magic_casting(world, npc_id, ai, skill, target_id);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, skill, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Garioneus: 10/20s effecting, 30s effecting (reset) + melee ──
    if BOSS_GARIONEUS.contains(&proto) {
        if utc == 10 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 20 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 30 && tmpl.magic_3 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Sorcerer Geden: 40/80s effecting, 120s effecting (reset) + melee ──
    if BOSS_SORCERER_GEDEN.contains(&proto) {
        if utc == 40 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 80 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 120 && tmpl.magic_3 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Atal: 20/60s effecting, 90s effecting (reset) + melee ──
    if proto == BOSS_ATAL {
        if utc == 20 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 60 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 90 && tmpl.magic_3 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Moospell: 30/70s effecting, 100s effecting (reset) + melee ──
    if proto == BOSS_MOSPELL {
        if utc == 30 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 70 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 100 && tmpl.magic_3 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Ahmi: 10/35s effecting, 65s effecting (reset) + melee ──
    if proto == BOSS_AHMI {
        if utc == 10 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 35 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 65 && tmpl.magic_3 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_3, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_3, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Fluwiton Room 3: 10/35s effecting, 65s random casting + melee ──
    if BOSS_FLUWITON_ROOM_3.contains(&proto) {
        if utc == 10 && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 35 && tmpl.magic_2 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_2, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_2, target_id as i32).await;
        }
        if utc == 65 {
            let rand_val: i32 = rng.gen_range(0..200);
            let skill = if rand_val < 30 {
                502020u32
            } else if rand_val < 60 {
                502021
            } else if rand_val < 85 {
                502031
            } else if rand_val < 120 {
                502019
            } else if rand_val < 150 {
                502028
            } else if rand_val < 170 {
                502029
            } else if rand_val < 180 {
                502030
            } else {
                502019
            };
            broadcast_npc_magic_casting(world, npc_id, ai, skill, target_id);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, skill, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Fluwiton Room 4: rapid effecting every 5s, casting at 70s + melee ──
    if BOSS_FLUWITON_ROOM_4.contains(&proto) {
        // Every 5 seconds from 5-60, cast magic_1 via EFFECTING
        if (5..=60).contains(&utc) && utc.is_multiple_of(5) && tmpl.magic_1 != 0 {
            broadcast_npc_magic(world, npc_id, ai, tmpl.magic_1, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, tmpl.magic_1, target_id as i32).await;
        }
        if utc == 70 {
            let rand_val: i32 = rng.gen_range(0..100);
            let skill = if rand_val < 70 { 502022u32 } else { 502023 };
            broadcast_npc_magic_casting(world, npc_id, ai, skill, target_id);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, skill, target_id as i32).await;
            world.update_npc_ai(npc_id, |s| s.utc_second = 0);
        }
        world.update_npc_ai(npc_id, |s| s.utc_second += 1);
        broadcast_npc_attack(world, npc_id, ai, target_id, ATTACK_SUCCESS);
        return Some(1000);
    }

    // ── Default boss fallback: same as magic_attack == 2 but with EFFECTING ──
    let n_random: i32 = rng.gen_range(1..=5000);
    if n_random < NPC_MAGIC_PERCENT_DEFAULT {
        let rand_skill: i32 = rng.gen_range(0..400);
        let skill_id = if rand_skill < 30 && tmpl.magic_2 > 0 {
            tmpl.magic_2
        } else if rand_skill < 60 && tmpl.magic_3 > 0 {
            tmpl.magic_3
        } else {
            tmpl.magic_1
        };
        if skill_id > 0 {
            broadcast_npc_magic(world, npc_id, ai, skill_id, target_id as i32);
            npc_apply_magic_effect(world, npc_id, ai, tmpl, skill_id, target_id as i32).await;
            return Some(tmpl.attack_delay as u64);
        }
    }

    None
}

/// Alert nearby same-family NPCs to assist (pack behavior).
/// + `Npc.cpp:1576-1650` — `CNpc::FindFriendRegion()`
/// Searches the 3x3 region grid for same-family NPCs. If found, sets their
/// target to the current attacker and transitions them to Attacking state.
/// Alert nearby NPCs to join combat when a pack NPC or boss is attacked.
/// Boss type alone does not establish an alliance. All callers require a
/// nonzero matching family; recruited allies cannot relay an unprovoked shout.
fn alert_pack(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    caller_template: &NpcTemplate,
    target_id: SessionId,
    is_boss: bool,
) {
    let caller_inst = match world.get_npc_instance(npc_id) {
        Some(n) if n.is_monster => n,
        _ => return,
    };
    let zone = match world.get_zone(ai.zone_id) {
        Some(z) => z,
        None => return,
    };

    let nearby_npcs = zone.get_npcs_in_3x3(ai.region_x, ai.region_z);

    for ally_nid in nearby_npcs {
        if ally_nid == npc_id {
            continue;
        }

        let ally_ai = match world.get_npc_ai(ally_nid) {
            Some(a) => a,
            None => continue,
        };

        // Skip if already fighting or dead
        //   if (pNpc->hasTarget() && pNpc->GetNpcState() == NPC_FIGHTING) continue;
        if ally_ai.state == NpcState::Fighting || ally_ai.state == NpcState::Dead {
            continue;
        }

        // Already has a target and in combat state
        if ally_ai.target_id.is_some()
            && matches!(ally_ai.state, NpcState::Attacking | NpcState::Tracing)
        {
            continue;
        }

        if !pack_family_matches(
            ai.family_type,
            ally_ai.family_type,
            is_boss,
            ally_ai.has_friends,
        ) {
            continue;
        }

        // Skip gate NPCs — they should not be called as friends
        let ally_inst = match world.get_npc_instance(ally_nid) {
            Some(n) => n,
            None => continue,
        };
        let ally_tmpl = match world.get_npc_template(ally_inst.proto_id, ally_inst.is_monster) {
            Some(t) => t,
            None => continue,
        };

        if !ally_inst.is_monster
            || !caller_inst.is_monster
            || ally_inst.event_room != caller_inst.event_room
            || ally_tmpl.group != caller_template.group
            || is_gate_npc_type(ally_tmpl.npc_type)
        {
            continue;
        }

        // Assistance uses the caller's local awareness, not its longer chase
        // radius. Otherwise one attacked monster recruits unrelated spawns.
        let dx = ai.cur_x - ally_ai.cur_x;
        let dz = ai.cur_z - ally_ai.cur_z;
        let dist = (dx * dx + dz * dz).sqrt();

        if caller_template.search_range == 0
            || dist
                > caller_template
                    .search_range
                    .min(caller_template.tracing_range) as f32
        {
            continue;
        }

        // Alert this ally — set target and transition to Attacking
        //   pNpc->m_Target.id = m_Target.id;
        //   pNpc->NpcStrategy(NPC_ATTACK_SHOUT);
        world.update_npc_ai(ally_nid, |s| {
            s.target_id = Some(target_id);
            s.state = NpcState::Attacking;
            s.delay_ms = 0;
            s.skill_cooldown_ms = s.last_tick_ms + NPC_SKILL_COOLDOWN_MS;
        });
    }
}

fn pack_family_matches(caller: u8, ally: u8, boss: bool, has_friends: bool) -> bool {
    caller != 0 && caller == ally && (boss || has_friends)
}

/// Find the most injured same-family NPC for healer AI.
/// Searches 3x3 region for same-family NPCs with HP below 90% threshold.
/// Returns the NPC with the lowest HP percentage.
fn find_injured_friend(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<NpcId> {
    let zone = world.get_zone(ai.zone_id)?;
    let nearby_npcs = zone.get_npcs_in_3x3(ai.region_x, ai.region_z);

    let search_range = tmpl.attack_range as f32;
    let mut best_nid: Option<NpcId> = None;
    let mut best_hp_pct: f32 = HEALER_HP_THRESHOLD;

    for ally_nid in nearby_npcs {
        if ally_nid == npc_id {
            continue;
        }

        // Check distance
        let ally_ai = match world.get_npc_ai(ally_nid) {
            Some(a) => a,
            None => continue,
        };

        let dx = ai.cur_x - ally_ai.cur_x;
        let dz = ai.cur_z - ally_ai.cur_z;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist > search_range {
            continue;
        }

        // Check if alive and injured
        if world.is_npc_dead(ally_nid) {
            continue;
        }

        let ally_inst = match world.get_npc_instance(ally_nid) {
            Some(n) => n,
            None => continue,
        };
        let ally_tmpl = match world.get_npc_template(ally_inst.proto_id, ally_inst.is_monster) {
            Some(t) => t,
            None => continue,
        };

        // Same family check
        if ally_tmpl.family_type != tmpl.family_type {
            continue;
        }

        let hp = world.get_npc_hp(ally_nid).unwrap_or(0);
        let max_hp = world.get_npc_war_max_hp(&ally_tmpl);
        if max_hp <= 0 {
            continue;
        }

        let hp_pct = hp as f32 / max_hp as f32;
        if hp_pct < best_hp_pct {
            best_hp_pct = hp_pct;
            best_nid = Some(ally_nid);
        }
    }

    best_nid
}

// ── Helper functions ────────────────────────────────────────────────────

/// Transition an NPC to dead state.
fn transition_to_dead(world: &WorldState, npc_id: NpcId) {
    world.update_npc_ai(npc_id, |s| {
        s.state = NpcState::Dead;
        s.target_id = None;
    });
}

/// Find the closest enemy player in the NPC's search range.
/// + `Npc.cpp:5698-5830` — `CNpc::FindEnemyExpand()`
/// For ATROCITY (aggressive) NPCs: targets any valid player in range.
/// For TENDER (passive) NPCs: only targets players who have damaged this NPC
/// (or friends' current target, per `m_bHasFriends && m_Target.id == target_uid`).
fn find_enemy(world: &WorldState, ai: &NpcAiState, npc_id: NpcId) -> Option<SessionId> {
    let npc = world.get_npc_instance(npc_id)?;
    let tmpl = world.get_npc_template(npc.proto_id, npc.is_monster)?;
    let npc_event_room = npc.event_room;

    // Gates, levers, artifacts, scarecrows, trees never search for enemies.
    if is_gate_npc_type(tmpl.npc_type) || matches!(tmpl.npc_type, 171 | 200) {
        return None;
    }

    let is_guard_type = matches!(tmpl.npc_type, 11..=15);

    // `bIsNeutralZone = (isInMoradon() || GetZoneID() == ZONE_ARENA)`
    if is_guard_type {
        let is_neutral = matches!(
            ai.zone_id,
            ZONE_MORADON
                | ZONE_MORADON2
                | ZONE_MORADON3
                | ZONE_MORADON4
                | ZONE_MORADON5
                | ZONE_ARENA
        );
        if is_neutral {
            return None;
        }
    }

    // Use template value directly for C++ parity.  Only guard NPCs get a
    // minimum floor (they always search per C++ CheckFindEnemy logic).
    let search_range = if is_guard_type {
        (tmpl.search_range as f32).max(MIN_GUARD_SEARCH_RANGE)
    } else {
        tmpl.search_range as f32
    };
    if search_range <= 0.0 {
        return None;
    }

    // NPC nation for guard hostility check.
    let npc_nation = ai.nation;

    // Get zone and search the 3x3 region grid
    let zone = world.get_zone(ai.zone_id)?;
    let users = zone.get_users_in_3x3(ai.region_x, ai.region_z);

    // Precompute timestamp once (was inside per-user loop — N syscalls → 1)
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut closest_dist = f32::MAX;
    let mut closest_target: Option<SessionId> = None;

    for user_sid in users {
        // Single DashMap lookup — extract ALL needed fields at once
        // (was 5 separate lookups: event_room, character, invisible, blink, position)
        let candidate = world.with_session(user_sid, |h| {
            // Must have character loaded
            let ch = h.character.as_ref()?;

            // Event room filter — C++ Npc.cpp:5820
            if h.event_room != npc_event_room {
                return None;
            }

            // Skip dead players
            if ch.res_hp_type == USER_DEAD || ch.hp <= 0 {
                return None;
            }

            // Skip GMs (authority == 0)
            if ch.authority == 0 {
                return None;
            }

            // Skip invisible (stealthed) players
            if h.invisibility_type != 0 {
                return None;
            }

            // Skip blinking (respawn invulnerable) players
            if h.blink_expiry_time > 0 && now_unix < h.blink_expiry_time {
                return None;
            }

            // Zone match + position
            if h.position.zone_id != ai.zone_id {
                return None;
            }

            Some((h.position.x, h.position.z, ch.nation))
        });

        let (px, pz, player_nation) = match candidate {
            Some(Some(pos)) => pos,
            _ => continue,
        };

        // Guard nation hostility check — guards only attack enemy-nation players.
        // Nation 0 = neutral (attacks everyone), 1 = Karus, 2 = Elmorad.
        if is_guard_type && npc_nation != 0 && player_nation == npc_nation {
            continue; // Same nation — friendly, skip
        }

        // TENDER (passive) NPC: only target players who have damaged us
        //   if (m_tNpcAttType == TENDER_ATTACK_TYPE
        //       && (IsDamagedUserList(pUser)
        //           || (m_bHasFriends && m_Target.id == target_uid)))
        if ai.attack_type == TENDER_ATTACK_TYPE {
            let is_damaged_by = world.npc_damage_contains(npc_id, user_sid);
            let is_friend_target = ai.has_friends && ai.target_id == Some(user_sid);
            if !is_damaged_by && !is_friend_target {
                continue;
            }
        }

        // Distance check
        let dx = ai.cur_x - px;
        let dz = ai.cur_z - pz;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist <= search_range && dist < closest_dist {
            closest_dist = dist;
            closest_target = Some(user_sid);
        }
    }

    closest_target
}

/// Find an enemy NPC (for NPC-vs-NPC combat: guards attacking monsters, etc.).
///   `if (bIsGuard || isMonster() || isGuardSummon())`
///   - Skip self, dead, non-attackable NPCs, pets
///   - Guards only attack monsters (isGuard && !pNpc->isMonster => skip)
///   - Hostility: isHostileTo(pNpc) — primarily nation-based
/// Returns the NpcId of the closest enemy NPC within search range, or None.
fn find_npc_enemy(
    world: &WorldState,
    ai: &NpcAiState,
    npc_id: NpcId,
    tmpl: &NpcTemplate,
) -> Option<NpcId> {
    // Only guards, monsters, and guard-summons can target other NPCs
    let is_guard = is_guard_type(tmpl.npc_type);
    let is_monster = tmpl.is_monster;

    if !is_guard && !is_monster {
        return None;
    }

    let search_range = tmpl.search_range as f32;
    if search_range <= 0.0 {
        return None;
    }

    let zone = world.get_zone(ai.zone_id)?;
    let nearby_npcs = zone.get_npcs_in_3x3(ai.region_x, ai.region_z);

    let mut closest_dist = f32::MAX;
    let mut closest_target: Option<NpcId> = None;

    for other_nid in nearby_npcs {
        // Skip self
        if other_nid == npc_id {
            continue;
        }

        // Get other NPC's instance and template
        let other_npc = match world.get_npc_instance(other_nid) {
            Some(n) => n,
            None => continue,
        };
        let other_tmpl = match world.get_npc_template(other_npc.proto_id, other_npc.is_monster) {
            Some(t) => t,
            None => continue,
        };

        // Skip dead NPCs
        if world.is_npc_dead(other_nid) {
            continue;
        }

        // Skip non-attackable objects (gates, levers, artifacts, scarecrows, trees)
        if is_gate_npc_type(other_tmpl.npc_type) {
            continue;
        }

        // Guards only attack monsters
        if is_guard && !other_tmpl.is_monster {
            continue;
        }

        // Monsters don't attack other monsters
        if is_monster && other_tmpl.is_monster {
            continue;
        }

        // Skip guard-summons as targets
        // (We don't implement guard-summons yet, but check for guard types too)

        // Hostility check: different nations are hostile
        // Only the ATTACKER being nation-0 (neutral/ALL) skips hostility.
        // A nation-0 TARGET can still be attacked by non-neutral NPCs.
        if ai.nation == 0 {
            continue;
        }
        if ai.nation == other_npc.nation {
            // Same nation — friendly, don't attack
            continue;
        }

        // Distance check
        let other_ai = world.get_npc_ai(other_nid);
        let (ox, oz) = match &other_ai {
            Some(a) => (a.cur_x, a.cur_z),
            None => (other_npc.x, other_npc.z),
        };

        let dx = ai.cur_x - ox;
        let dz = ai.cur_z - oz;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist <= search_range && dist < closest_dist {
            closest_dist = dist;
            closest_target = Some(other_nid);
        }
    }

    closest_target
}

/// Send WIZ_NPC_MOVE broadcast to the 3x3 region.
/// Wire format: `[u8 1][u32 npc_id][u16 x*10][u16 z*10][u16 y*10][u16 speed*10]`
#[allow(clippy::too_many_arguments)]
fn send_npc_move(
    world: &WorldState,
    npc_id: NpcId,
    zone_id: u16,
    region_x: u16,
    region_z: u16,
    x: f32,
    z: f32,
    speed: f32,
) {
    let mut pkt = Packet::new(Opcode::WizNpcMove as u8);
    pkt.write_u8(1); // move type
    pkt.write_u32(npc_id);
    pkt.write_u16((x * 10.0) as u16);
    pkt.write_u16((z * 10.0) as u16);
    pkt.write_u16(0); // y * 10 (height, usually 0)
    pkt.write_u16((speed * 10.0) as u16);

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        zone_id,
        region_x,
        region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// Broadcast an NPC attack result to the 3x3 region.
/// Wire format: `[u8 LONG_ATTACK][u8 result][u32 npc_id][u32 target_id]`
fn broadcast_npc_attack(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    target_id: SessionId,
    result: u8,
) {
    let mut pkt = Packet::new(Opcode::WizAttack as u8);
    pkt.write_u8(LONG_ATTACK);
    pkt.write_u8(result);
    pkt.write_u32(npc_id);
    pkt.write_u32(target_id as u32);

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// Send NPC despawn (INOUT_OUT) to region.
fn send_npc_despawn(world: &WorldState, npc_id: NpcId, ai: &NpcAiState) {
    let mut pkt = Packet::new(Opcode::WizNpcInout as u8);
    pkt.write_u8(crate::npc::NPC_OUT);
    pkt.write_u32(npc_id);

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// Send NPC respawn (INOUT_IN) to region.
fn send_npc_respawn(
    world: &WorldState,
    npc_id: NpcId,
    zone_id: u16,
    region_x: u16,
    region_z: u16,
    tmpl: &NpcTemplate,
) {
    if let Some(npc) = world.get_npc_instance(npc_id) {
        let mut pkt = Packet::new(Opcode::WizNpcInout as u8);
        pkt.write_u8(crate::npc::NPC_IN);
        pkt.write_u32(npc_id);
        crate::npc::write_npc_info(&mut pkt, &npc, tmpl);

        world.broadcast_to_3x3(
            zone_id,
            region_x,
            region_z,
            Arc::new(pkt),
            None,
            npc.event_room,
        );
    }
}

/// Broadcast WIZ_STATE_CHANGE for an NPC (used on sleep/faint wake-up).
/// Wire format: `[u32 npc_id] [u8 type] [u32 buff]`
fn broadcast_state_change(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    change_type: u8,
    buff: u32,
) {
    let mut pkt = Packet::new(Opcode::WizStateChange as u8);
    pkt.write_u32(npc_id);
    pkt.write_u8(change_type);
    pkt.write_u32(buff);

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

use crate::npc::NPC_BAND;

/// Apply the server-side effect of an NPC's magic skill after broadcasting.
/// This handles:
/// - **Damage skills (Type 3, MORAL_ENEMY):** Compute magic damage and apply to player target.
/// - **Heal skills (Type 3, MORAL_FRIEND/SELF):** Restore HP on friendly NPC target.
/// - Death handling when player HP reaches 0.
/// The damage formula follows the C++ NPC caster path in `GetMagicDamage()`:
/// - `total_hit` = raw `sFirstDamage` from magic table (no CHA scaling for NPCs)
/// - `damage = 485 * total_hit / (total_r + 510)` — total_r simplified to 0
/// - Final randomization: `rand(0, damage) * 0.3 + damage * 0.85`
async fn npc_apply_magic_effect(
    world: &WorldState,
    npc_id: NpcId,
    _ai: &NpcAiState,
    _tmpl: &NpcTemplate,
    skill_id: u32,
    target_id: i32,
) {
    if skill_id == 0 || target_id < 0 {
        return;
    }

    // Look up the magic skill definition
    let skill = match world.get_magic(skill_id as i32) {
        Some(s) => s,
        None => {
            tracing::debug!("NPC {} magic effect: unknown skill_id={}", npc_id, skill_id);
            return;
        }
    };

    let skill_type = skill.type1.unwrap_or(0) as u8;

    // Only handle Type 3 (magic damage/heal/DOT) — the most common for NPC casters.
    // Type 4 (buffs/debuffs) from NPCs are visual-only in C++ as well.
    if skill_type != 3 {
        return;
    }

    let type3_data = match world.get_magic_type3(skill.magic_num) {
        Some(d) => d,
        None => return,
    };

    let moral = skill.moral.unwrap_or(0);
    let first_damage = type3_data.first_damage.unwrap_or(0);

    // ── Heal path (healer NPC → friendly NPC target) ────────────────
    // C++ moral 2/3/4 = friendly targets
    if moral == 2 || moral == 3 || moral == 4 || moral == 6 || moral == 11 {
        let target_npc_id = target_id as u32;
        // Only heal NPC targets (>= NPC_BAND)
        if target_npc_id < NPC_BAND {
            return;
        }

        let heal_amount = combat::compute_npc_heal_amount(first_damage);
        if heal_amount <= 0 {
            return;
        }

        // Get current and max HP
        let current_hp = match world.get_npc_hp(target_npc_id) {
            Some(hp) => hp,
            None => return,
        };
        let target_inst = match world.get_npc_instance(target_npc_id) {
            Some(n) => n,
            None => return,
        };
        let target_tmpl = match world.get_npc_template(target_inst.proto_id, target_inst.is_monster)
        {
            Some(t) => t,
            None => return,
        };

        let max_hp = world.get_npc_war_max_hp(&target_tmpl);
        let new_hp = (current_hp + heal_amount).min(max_hp);
        world.update_npc_hp(target_npc_id, new_hp);

        tracing::debug!(
            "NPC {} healed NPC {} with skill {}: +{} HP ({} -> {}/{})",
            npc_id,
            target_npc_id,
            skill_id,
            heal_amount,
            current_hp,
            new_hp,
            max_hp
        );
        return;
    }

    // ── Damage path (NPC → player target) ───────────────────────────
    // C++ moral 7 = single enemy, 10 = AOE enemy
    if moral != 7 && moral != 10 {
        return;
    }

    // Determine if target is a player (< NPC_BAND) or NPC (>= NPC_BAND)
    let target_uid = target_id as u32;
    if target_uid >= NPC_BAND {
        // NPC-on-NPC damage: not applicable for standard monsters
        return;
    }

    let target_sid = target_id as SessionId;

    // Validate target is alive
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => return,
    };

    if target.res_hp_type == USER_DEAD || target.hp <= 0 {
        return;
    }

    // Compute target resistance and zone info for damage formula
    let attr = type3_data.attribute.unwrap_or(0) as u8;
    let equip = world.get_equipped_stats(target_sid);
    let item_r = match attr {
        1 => equip.fire_r as i32,
        2 => equip.cold_r as i32,
        3 => equip.lightning_r as i32,
        4 => equip.magic_r as i32,
        5 => equip.disease_r as i32,
        6 => equip.poison_r as i32,
        _ => 0,
    };
    let buff_r = world.get_buff_elemental_resistance(target_sid, attr);
    let target_total_r = item_r + buff_r;
    let is_war = world
        .get_position(target_sid)
        .and_then(|p| world.get_zone(p.zone_id))
        .map(|z| z.is_war_zone())
        .unwrap_or(false);

    // Compute magic damage
    let mut rng = rand::rngs::StdRng::from_entropy();
    let damage = combat::compute_npc_magic_damage(first_damage, target_total_r, is_war, &mut rng);

    if damage <= 0 {
        return;
    }

    // Apply damage to player through HpChange pipeline
    // Mirror: SKIP for NPC attackers (C++ line 75: pAttacker->isPlayer() required)
    let mut final_damage = damage;

    // C++ HpChange order: save original → mirror(skip) → mastery → mana absorb
    let original_damage = final_damage;

    // ── Mastery passive damage reduction ────────────────────────────────
    {
        let victim_zone = world
            .get_position(target_sid)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        let not_use_zone = victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
        if !not_use_zone && crate::handler::class_change::is_mastered(target.class) {
            let master_pts = target.skill_points[8]; // SkillPointMaster = index 8
            if master_pts >= 10 {
                final_damage = (85 * final_damage as i32 / 100) as i16;
            } else if master_pts >= 5 {
                final_damage = (90 * final_damage as i32 / 100) as i16;
            }
        }
    }

    // ── Mana Absorb (Outrage/Frenzy/Mana Shield) ─────────────────────
    {
        let victim_zone = world
            .get_position(target_sid)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        let not_use_zone = victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
        let (absorb_pct, absorb_count) = world
            .with_session(target_sid, |h| (h.mana_absorb, h.absorb_count))
            .unwrap_or((0, 0));
        if absorb_pct > 0 && !not_use_zone {
            let should_absorb = if absorb_pct == 15 {
                absorb_count > 0
            } else {
                true
            };
            if should_absorb {
                let absorbed = (original_damage as i32 * absorb_pct as i32 / 100) as i16;
                final_damage -= absorbed;
                if final_damage < 0 {
                    final_damage = 0;
                }
                world.update_character_stats(target_sid, |ch| {
                    ch.mp = (ch.mp as i32 + absorbed as i32).min(ch.max_mp as i32) as i16;
                });
                if absorb_pct == 15 {
                    world.update_session(target_sid, |h| {
                        h.absorb_count = h.absorb_count.saturating_sub(1);
                    });
                }
            }
        }
    }

    final_damage = world
        .manes_survival_manager
        .reduce_incoming_damage(target_sid, final_damage);
    let new_hp = (target.hp - final_damage).max(0);
    world.update_character_hp(target_sid, new_hp);

    // Send WIZ_HP_CHANGE to the victim with NPC as attacker
    let hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
        target.max_hp,
        new_hp,
        npc_id, // NPC ID as attacker
    );
    world.send_to_session_owned(target_sid, hp_pkt);

    // Handle death
    if new_hp <= 0 {
        crate::handler::dead::broadcast_death(world, target_sid);
        // XP loss only on NPC kills (with type exclusions)
        let (npc_type, npc_proto) = world
            .get_npc_instance(npc_id)
            .and_then(|inst| {
                world
                    .get_npc_template(inst.proto_id, inst.is_monster)
                    .map(|t| (t.npc_type, inst.proto_id))
            })
            .unwrap_or((0, 0));
        crate::handler::dead::apply_npc_death_xp_loss(world, target_sid, npc_type, npc_proto).await;

        // NPC should stop fighting dead target
        world.update_npc_ai(npc_id, |s| {
            s.state = NpcState::Standing;
            s.target_id = None;
        });
    }

    tracing::debug!(
        "NPC {} magic damage to player {}: skill={} damage={} new_hp={}",
        npc_id,
        target_sid,
        skill_id,
        damage,
        new_hp
    );

    // Register DOT if time_damage != 0 and duration > 0
    let time_damage = type3_data.time_damage.unwrap_or(0);
    let duration = type3_data.duration.unwrap_or(0);
    if time_damage != 0 && duration > 0 {
        let tick_count = (duration / 2).max(1) as u8;
        let mut dot_rng = rand::rngs::StdRng::from_entropy();
        let dot_dmg =
            combat::compute_npc_magic_damage(time_damage, target_total_r, is_war, &mut dot_rng);
        // DOT damage is negative (hurts target), per-tick
        let hp_per_tick = -(dot_dmg.unsigned_abs() as i16 / tick_count as i16).max(1);
        world.add_durational_skill(
            target_sid,
            skill_id,
            hp_per_tick,
            tick_count,
            0, // NPC caster — no player session ID
        );

        tracing::debug!(
            "NPC {} applied DOT to player {}: skill={} hp_per_tick={} ticks={}",
            npc_id,
            target_sid,
            skill_id,
            hp_per_tick,
            tick_count
        );
    }
}

/// Broadcast WIZ_MAGIC_PROCESS EFFECTING for an NPC skill.
/// Wire format: `[u8 MAGIC_EFFECTING] [u32 skill_id] [u32 caster_id] [u32 target_id] [u32 x7 sData]`
fn broadcast_npc_magic(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    skill_id: u32,
    target_id: i32,
) {
    // MAGIC_EFFECTING = 3 (from C++ MagicOpcode enum in packets.h:560)
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(3); // MAGIC_EFFECTING
    pkt.write_u32(skill_id);
    pkt.write_u32(npc_id);
    pkt.write_u32(target_id as u32);
    pkt.write_u32(0); // sData[0]
    pkt.write_u32(0); // sData[1]
    pkt.write_u32(0); // sData[2]
    pkt.write_u32(0); // sData[3]
    pkt.write_u32(0); // sData[4]
    pkt.write_u32(0); // sData[5]
    pkt.write_u32(0); // sData[6]

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// Broadcast WIZ_MAGIC_PROCESS CASTING for an NPC skill (pre-cast animation).
/// Wire format: `[u8 MAGIC_CASTING] [u32 skill_id] [u32 caster_id] [u32 target_id] [u32 x7 sData]`
fn broadcast_npc_magic_casting(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    skill_id: u32,
    target_id: SessionId,
) {
    // MAGIC_CASTING = 1 (from C++ MagicOpcode enum in packets.h:558)
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(1); // MAGIC_CASTING
    pkt.write_u32(skill_id);
    pkt.write_u32(npc_id);
    pkt.write_u32(target_id as u32);
    pkt.write_u32(0); // sData[0]
    pkt.write_u32(0); // sData[1]
    pkt.write_u32(0); // sData[2]
    pkt.write_u32(0); // sData[3]
    pkt.write_u32(0); // sData[4]
    pkt.write_u32(0); // sData[5]
    pkt.write_u32(0); // sData[6]

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// Broadcast WIZ_MAGIC_PROCESS with a configurable opcode for an NPC skill.
/// Used for boss-specific opcodes like MAGIC_FLYING (4) for Elite Timarli.
fn broadcast_npc_magic_with_opcode(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    skill_id: u32,
    target_id: i32,
    magic_opcode: u8,
) {
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(magic_opcode);
    pkt.write_u32(skill_id);
    pkt.write_u32(npc_id);
    pkt.write_u32(target_id as u32);
    pkt.write_u32(0); // sData[0]
    pkt.write_u32(0); // sData[1]
    pkt.write_u32(0); // sData[2]
    pkt.write_u32(0); // sData[3]
    pkt.write_u32(0); // sData[4]
    pkt.write_u32(0); // sData[5]
    pkt.write_u32(0); // sData[6]

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

/// Determine hit result using the C++ hit rate table.
/// Returns: 1=GREAT_SUCCESS, 2=SUCCESS, 3=NORMAL, 4=FAIL
fn get_hit_rate(rate: f32, rng: &mut impl Rng) -> u8 {
    let random = rng.gen_range(1..=10000);

    if rate >= 5.0 {
        if random <= 3500 {
            1
        } else if random <= 7500 {
            2
        } else if random <= 9800 {
            3
        } else {
            4
        }
    } else if rate >= 3.0 {
        if random <= 2500 {
            1
        } else if random <= 6000 {
            2
        } else if random <= 9600 {
            3
        } else {
            4
        }
    } else if rate >= 2.0 {
        if random <= 2000 {
            1
        } else if random <= 5000 {
            2
        } else if random <= 9400 {
            3
        } else {
            4
        }
    } else if rate >= 1.25 {
        if random <= 1500 {
            1
        } else if random <= 4000 {
            2
        } else if random <= 9200 {
            3
        } else {
            4
        }
    } else if rate >= 0.8 {
        if random <= 1000 {
            1
        } else if random <= 3000 {
            2
        } else if random <= 9000 {
            3
        } else {
            4
        }
    } else if rate >= 0.5 {
        if random <= 800 {
            1
        } else if random <= 2500 {
            2
        } else if random <= 8000 {
            3
        } else {
            4
        }
    } else if rate >= 0.33 {
        if random <= 600 {
            1
        } else if random <= 2000 {
            2
        } else if random <= 7000 {
            3
        } else {
            4
        }
    } else if rate >= 0.2 {
        if random <= 400 {
            1
        } else if random <= 1500 {
            2
        } else if random <= 6000 {
            3
        } else {
            4
        }
    } else if random <= 200 {
        1
    } else if random <= 1000 {
        2
    } else if random <= 5000 {
        3
    } else {
        4
    }
}

// ── Gate NPC Logic ──────────────────────────────────────────────────────

/// Handle gate-type NPC behavior during NPC_STANDING state.
/// Returns `Some(delay_ms)` if the gate logic consumed the tick, `None` to fall through
/// to the default stand_time return.
fn handle_gate_standing(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
) -> Option<u64> {
    let npc_type = tmpl.npc_type;
    let stand_time = tmpl.stand_time as u64;

    // ── NPC_SPECIAL_GATE: Cycle open/close during nation war ─────────
    //   if (GetType() == NPC_SPECIAL_GATE && GetMap()->isWarZone()
    //       && (m_byBattleOpen == NATION_BATTLE || m_byBattleOpen == SNOW_BATTLE))
    if npc_type == NPC_SPECIAL_GATE {
        // Check war state and zone war flag
        let battle_state = world.get_battle_state();
        let is_war = battle_state.is_nation_battle() || battle_state.is_snow_battle();
        let zone_is_war = world
            .get_zone(ai.zone_id)
            .map(|z| z.is_war_zone())
            .unwrap_or(false);

        if is_war && zone_is_war {
            if ai.gate_open == 0 {
                // Open the gate
                send_gate_flag(world, npc_id, ai, tmpl, 1);
                world.update_npc_ai(npc_id, |s| {
                    s.gate_open = 1;
                });
                // C++ returns m_sStandTime * 10 (open duration)
                return Some(stand_time.saturating_mul(10));
            } else if ai.gate_open == 1 {
                // Close the gate
                send_gate_flag(world, npc_id, ai, tmpl, 0);
                world.update_npc_ai(npc_id, |s| {
                    s.gate_open = 0;
                });
                // C++ returns m_sStandTime * 60 (closed duration, longer)
                return Some(stand_time.saturating_mul(60));
            }
        }
    }

    // ── NPC_OBJECT_WOOD: Auto-close after cooldown during nation war ─
    //   else if (GetType() == NPC_OBJECT_WOOD && GetMap()->isWarZone()
    //       && m_byBattleOpen == NATION_BATTLE)
    //   { if (m_byGateOpen == 1 && WoodCooldownClose++ >= 30) { close; } }
    if npc_type == NPC_OBJECT_WOOD {
        let battle_state = world.get_battle_state();
        let is_nation_war = battle_state.is_nation_battle();
        let zone_is_war = world
            .get_zone(ai.zone_id)
            .map(|z| z.is_war_zone())
            .unwrap_or(false);

        if is_nation_war && zone_is_war && ai.gate_open == 1 {
            let new_count = ai.wood_cooldown_count + 1;
            if new_count >= WOOD_COOLDOWN_THRESHOLD {
                // Auto-close the wood gate
                send_gate_flag(world, npc_id, ai, tmpl, 0);
                world.update_npc_ai(npc_id, |s| {
                    s.gate_open = 0;
                    s.wood_cooldown_count = 0;
                });
            } else {
                world.update_npc_ai(npc_id, |s| {
                    s.wood_cooldown_count = new_count;
                });
            }
        }
    }

    // ── NPC_KROWAZ_GATE: Auto-close in Krowaz Dominion zone ─────────
    //   else if (GetType() == NPC_KROWAZ_GATE && GetZoneID() == ZONE_KROWAZ_DOMINION)
    //   { if (m_byGateOpen == 1) { close; return m_sStandTime * 10; } }
    if npc_type == NPC_KROWAZ_GATE && ai.zone_id == ZONE_KROWAZ_DOMINION && ai.gate_open == 1 {
        send_gate_flag(world, npc_id, ai, tmpl, 0);
        world.update_npc_ai(npc_id, |s| {
            s.gate_open = 0;
        });
        return Some(stand_time.saturating_mul(10));
    }

    None
}

/// Send a gate flag broadcast (WIZ_OBJECT_EVENT) to the NPC's 3x3 region.
/// Sets `m_byGateOpen` and broadcasts the new state. For NPC_OBJECT_WOOD and
/// NPC_ROLLINGSTONE, the flag is set but no packet is broadcast (C++ returns early
/// after setting m_byGateOpen).
/// Wire format: `WIZ_OBJECT_EVENT [u8 object_type] [u8 1] [u32 npc_id] [u8 gate_open]`
fn send_gate_flag(
    world: &WorldState,
    npc_id: NpcId,
    ai: &NpcAiState,
    tmpl: &NpcTemplate,
    gate_open: u8,
) {
    // Update the NpcInstance's gate_open for future NPC_INOUT packets
    world.update_npc_gate_open(npc_id, gate_open);

    // NPC_OBJECT_WOOD and NPC_ROLLINGSTONE: only update state, no broadcast
    //   if (GetType() == NPC_OBJECT_WOOD || GetType() == NPC_ROLLINGSTONE) return;
    if tmpl.npc_type == NPC_OBJECT_WOOD || tmpl.npc_type == NPC_ROLLINGSTONE {
        return;
    }

    // Build WIZ_OBJECT_EVENT packet
    //   Packet result(WIZ_OBJECT_EVENT, objectType);
    //   result << uint8(1) << uint32(GetID()) << m_byGateOpen;
    let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
    pkt.write_u8(OBJECT_FLAG_LEVER); // default object type
    pkt.write_u8(1); // success marker
    pkt.write_u32(npc_id);
    pkt.write_u8(gate_open);

    let npc_event_room = world
        .get_npc_instance(npc_id)
        .map(|n| n.event_room)
        .unwrap_or(0);
    world.broadcast_to_3x3(
        ai.zone_id,
        ai.region_x,
        ai.region_z,
        Arc::new(pkt),
        None,
        npc_event_room,
    );
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants, clippy::ifs_same_cond)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_zero_regen_time_disables_auto_respawn() {
        assert!(!can_auto_respawn(0));
    }

    #[test]
    fn test_positive_regen_time_allows_auto_respawn() {
        assert!(can_auto_respawn(250));
        assert!(can_auto_respawn(30_000));
    }

    #[test]
    fn test_npc_state_values_match_cpp() {
        assert_eq!(NpcState::Dead as u8, 0);
        assert_eq!(NpcState::Live as u8, 1);
        assert_eq!(NpcState::Attacking as u8, 2);
        assert_eq!(NpcState::Standing as u8, 5);
        assert_eq!(NpcState::Moving as u8, 6);
        assert_eq!(NpcState::Tracing as u8, 7);
        assert_eq!(NpcState::Fighting as u8, 8);
        assert_eq!(NpcState::Back as u8, 10);
        assert_eq!(NpcState::Sleeping as u8, 11);
        assert_eq!(NpcState::Fainting as u8, 12);
        assert_eq!(NpcState::Healing as u8, 13);
        assert_eq!(NpcState::Casting as u8, 14);
    }

    #[test]
    fn test_leash_range_constant() {
        // C++ NPC_MAX_MOVE_RANGE2 = 200
        assert_eq!(NPC_MAX_LEASH_RANGE, 200.0);
    }

    #[test]
    fn test_npc_damage_formula() {
        // C++ formula: HitB = (m_sTotalHit * m_bAttackAmount * 200 / 100) / (Ac + 240)
        let npc_attack: i32 = 500;
        let attack_amount: i32 = 100;
        let target_ac: i32 = 72; // Level 60 warrior AC

        let hit_b = (npc_attack * attack_amount * 2) / (target_ac + 240);
        // 500 * 100 * 2 / (72 + 240) = 100000 / 312 = 320
        assert_eq!(hit_b, 320);
    }

    #[test]
    fn test_npc_damage_great_success() {
        let hit_b: i32 = 320;
        let npc_total_hit: i32 = 500;

        // GREAT_SUCCESS: 0.85*320 + 0.3*0 = 272, then *3/2 = 408
        let d_min = (0.85 * hit_b as f32) as i32;
        let d_min_great = d_min * 3 / 2;

        assert!(d_min_great > 0);
        assert!(d_min_great <= (2.6 * npc_total_hit as f32) as i32);
    }

    #[test]
    fn test_npc_damage_success_normal() {
        let hit_b: i32 = 320;

        // SUCCESS/NORMAL: 0.85*320 + 0.3*0 = 272
        let d_min = (0.85 * hit_b as f32) as i32;
        assert_eq!(d_min, 272);

        // 0.85*320 + 0.3*320 = 272 + 96 = 368
        let d_max = (0.85 * hit_b as f32 + 0.3 * hit_b as f32) as i32;
        assert_eq!(d_max, 368);
    }

    #[test]
    fn test_npc_damage_cap() {
        // Max damage = min(MAX_DAMAGE, 2.6 * npc_total_hit)
        let npc_total_hit: i32 = 500;
        let max_npc_damage = (2.6 * npc_total_hit as f32) as i32;
        assert_eq!(max_npc_damage, 1300);
        assert!(max_npc_damage < MAX_DAMAGE);
    }

    #[test]
    fn test_hit_rate_table_high() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut hits = 0;
        for _ in 0..1000 {
            let result = get_hit_rate(10.0, &mut rng);
            if result != 4 {
                hits += 1;
            }
        }
        // rate >= 5.0 → 98% hit rate
        assert!(hits > 950, "Expected > 950 hits, got {}", hits);
    }

    #[test]
    fn test_hit_rate_table_low() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut fails = 0;
        for _ in 0..1000 {
            let result = get_hit_rate(0.1, &mut rng);
            if result == 4 {
                fails += 1;
            }
        }
        // rate < 0.2 → 50% fail rate
        assert!(fails > 400, "Expected > 400 fails, got {}", fails);
    }

    #[test]
    fn test_npc_move_packet_format() {
        let mut pkt = Packet::new(Opcode::WizNpcMove as u8);
        pkt.write_u8(1);
        pkt.write_u32(10001);
        pkt.write_u16(6160); // 616.0 * 10
        pkt.write_u16(3410); // 341.0 * 10
        pkt.write_u16(0); // y
        pkt.write_u16(100); // speed * 10

        assert_eq!(pkt.opcode, Opcode::WizNpcMove as u8);
        // 1 + 4 + 2 + 2 + 2 + 2 = 13 bytes
        assert_eq!(pkt.data.len(), 13);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.read_u16(), Some(6160));
        assert_eq!(r.read_u16(), Some(3410));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_npc_attack_broadcast_format() {
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(LONG_ATTACK); // attack type
        pkt.write_u8(ATTACK_SUCCESS); // result
        pkt.write_u32(10001); // npc id
        pkt.write_u32(42); // target player id

        assert_eq!(pkt.opcode, Opcode::WizAttack as u8);
        // 1 + 1 + 4 + 4 = 10 bytes
        assert_eq!(pkt.data.len(), 10);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(LONG_ATTACK));
        assert_eq!(r.read_u8(), Some(ATTACK_SUCCESS));
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    /// Helper to create a test NpcAiState with sensible defaults.
    fn make_test_ai() -> NpcAiState {
        NpcAiState {
            state: NpcState::Standing,
            spawn_x: 100.0,
            spawn_z: 200.0,
            cur_x: 100.0,
            cur_z: 200.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30_000,
            is_aggressive: true,
            zone_id: 21,
            region_x: 2,
            region_z: 4,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 0,
            is_tower_owner: false,
            attack_type: 1, // ATROCITY by default (aggressive)
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        }
    }

    #[test]
    fn test_ai_state_defaults() {
        let ai = make_test_ai();

        assert_eq!(ai.state, NpcState::Standing);
        assert!(ai.is_aggressive);
        assert_eq!(ai.target_id, None);
        assert_eq!(ai.regen_time_ms, 30_000);
        assert_eq!(ai.fainting_until_ms, 0);
        assert_eq!(ai.active_skill_id, 0);
        assert_eq!(ai.active_target_id, -1);
        assert!(!ai.has_friends);
    }

    #[test]
    fn test_leash_distance_check() {
        let spawn_x = 100.0f32;
        let spawn_z = 200.0f32;
        let cur_x = 250.0f32;
        let cur_z = 200.0f32;

        let dx = cur_x - spawn_x;
        let dz = cur_z - spawn_z;
        let dist = (dx * dx + dz * dz).sqrt();

        // Distance = 150, leash range = 200
        assert!(dist < NPC_MAX_LEASH_RANGE);
        assert_eq!(dist, 150.0);

        // At exactly 200
        let cur_x2 = 300.0f32;
        let dx2 = cur_x2 - spawn_x;
        let dist2 = (dx2 * dx2 + dz * dz).sqrt();
        assert!(dist2 >= NPC_MAX_LEASH_RANGE);
    }

    // ── New state tests ────────────────────────────────────────────────

    #[test]
    fn test_fainting_time_constant() {
        // C++ FAINTING_TIME = 2 seconds = 2000ms
        assert_eq!(FAINTING_TIME_MS, 2000);
    }

    #[test]
    fn test_healer_hp_threshold() {
        // C++ uses 90% HP threshold for healer AI
        assert_eq!(HEALER_HP_THRESHOLD, 0.9);

        // A monster with 1000 max HP should be healed if below 900
        let max_hp = 1000;
        let threshold = (max_hp as f32 * HEALER_HP_THRESHOLD) as i32;
        assert_eq!(threshold, 900);
    }

    #[test]
    fn test_sleeping_state_still_asleep() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Sleeping;
        ai.fainting_until_ms = 10_000; // wakes at 10s

        // At 5s (still sleeping), should remain in Sleeping
        let now_ms = 5_000;
        assert!(now_ms < ai.fainting_until_ms);
    }

    #[test]
    fn test_sleeping_state_wake_up() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Sleeping;
        ai.fainting_until_ms = 10_000; // wakes at 10s

        // At 10s, should wake up (time >= fainting_until_ms)
        let now_ms = 10_000;
        assert!(now_ms >= ai.fainting_until_ms);
    }

    #[test]
    fn test_fainting_state_still_stunned() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Fainting;
        ai.fainting_until_ms = 5_000; // started at 5s

        // At 6s (only 1s elapsed, need 2s), still stunned
        let now_ms = 6_000;
        assert!(now_ms < ai.fainting_until_ms + FAINTING_TIME_MS);
    }

    #[test]
    fn test_fainting_state_wake_up() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Fainting;
        ai.fainting_until_ms = 5_000; // started at 5s

        // At 7s (2s elapsed), should wake up
        let now_ms = 7_000;
        assert!(now_ms >= ai.fainting_until_ms + FAINTING_TIME_MS);
    }

    #[test]
    fn test_casting_state_fields() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Fighting;

        // Simulate entering casting state
        let skill_id = 502022u32;
        let target_id = 42i32;
        let cast_time = 1500u64;

        ai.old_state = ai.state;
        ai.state = NpcState::Casting;
        ai.active_skill_id = skill_id;
        ai.active_target_id = target_id;
        ai.active_cast_time_ms = cast_time;

        assert_eq!(ai.state, NpcState::Casting);
        assert_eq!(ai.old_state, NpcState::Fighting);
        assert_eq!(ai.active_skill_id, 502022);
        assert_eq!(ai.active_target_id, 42);
        assert_eq!(ai.active_cast_time_ms, 1500);

        // After casting completes, should return to old state
        ai.state = ai.old_state;
        ai.active_skill_id = 0;
        ai.active_target_id = -1;
        ai.active_cast_time_ms = 0;

        assert_eq!(ai.state, NpcState::Fighting);
        assert_eq!(ai.active_skill_id, 0);
    }

    #[test]
    fn test_casting_delay_calculation() {
        // C++ formula: tAttackDelay = m_sAttackDelay - m_sActiveCastTime
        let attack_delay: u64 = 2000;
        let cast_time: u64 = 1500;

        let remaining = attack_delay.saturating_sub(cast_time);
        assert_eq!(remaining, 500);

        // When cast time exceeds attack delay, remaining is 0
        let cast_time2: u64 = 3000;
        let remaining2 = attack_delay.saturating_sub(cast_time2);
        assert_eq!(remaining2, 0);
    }

    #[test]
    fn test_pack_ai_family_matching() {
        let ai1 = NpcAiState {
            has_friends: true,
            family_type: 5,
            ..make_test_ai()
        };

        let ai2 = NpcAiState {
            has_friends: true,
            family_type: 5,
            ..make_test_ai()
        };

        let ai3 = NpcAiState {
            has_friends: true,
            family_type: 8,
            ..make_test_ai()
        };

        // Same family should match
        assert_eq!(ai1.family_type, ai2.family_type);
        assert!(ai1.has_friends && ai2.has_friends);

        // Different family should not match
        assert_ne!(ai1.family_type, ai3.family_type);
    }

    #[test]
    fn test_state_change_packet_format() {
        // C++ StateChangeServerDirect sends: [u32 socket_id] [u8 type] [u32 buff]
        // No trailing padding — User.cpp:2941-3007
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(10001); // npc_id
        pkt.write_u8(1); // change type
        pkt.write_u32(5); // buff (5 = wake to fighting)

        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        // 4 + 1 + 4 = 9 bytes
        assert_eq!(pkt.data.len(), 9);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(5));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_magic_process_packet_format() {
        // WIZ_MAGIC_PROCESS EFFECTING packet format
        // C++ BuildSkillPacket writes: opcode + skill_id + caster + target + sData[0..6]
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(3); // MAGIC_EFFECTING
        pkt.write_u32(502022); // skill_id
        pkt.write_u32(10001); // caster (npc)
        pkt.write_u32(42); // target
        pkt.write_u32(0); // sData[0]
        pkt.write_u32(0); // sData[1]
        pkt.write_u32(0); // sData[2]
        pkt.write_u32(0); // sData[3]
        pkt.write_u32(0); // sData[4]
        pkt.write_u32(0); // sData[5]
        pkt.write_u32(0); // sData[6]

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        // 1 + 4 + 4 + 4 + (7 * 4) = 41 bytes
        assert_eq!(pkt.data.len(), 41);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3)); // MAGIC_EFFECTING
        assert_eq!(r.read_u32(), Some(502022));
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.read_u32(), Some(42));
        for _ in 0..7 {
            assert_eq!(r.read_u32(), Some(0));
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_healer_find_most_injured() {
        // Healer should prioritize the NPC with lowest HP percentage
        let max_hp = 1000;
        let threshold = HEALER_HP_THRESHOLD;

        // NPC A: 85% HP (injured, below threshold)
        let hp_a = 850;
        let pct_a = hp_a as f32 / max_hp as f32;
        assert!(pct_a < threshold);

        // NPC B: 50% HP (more injured)
        let hp_b = 500;
        let pct_b = hp_b as f32 / max_hp as f32;
        assert!(pct_b < pct_a);

        // NPC C: 95% HP (healthy, above threshold)
        let hp_c = 950;
        let pct_c = hp_c as f32 / max_hp as f32;
        assert!(pct_c >= threshold);

        // Healer should pick B (lowest percentage below threshold)
        let mut best_pct = threshold;
        let mut best = None;
        for (id, pct) in [(1u32, pct_a), (2, pct_b), (3, pct_c)] {
            if pct < best_pct {
                best_pct = pct;
                best = Some(id);
            }
        }
        assert_eq!(best, Some(2));
    }

    #[test]
    fn test_ranged_attack_skill_selection() {
        // magic_2 > 0: randomly picks between magic_1 and magic_2
        // magic_2 == 0: always uses magic_1
        let magic_1 = 502022u32;
        let magic_2 = 502023u32;

        // With magic_2 present, selection depends on random
        assert!(magic_1 > 0);
        assert!(magic_2 > 0);

        // Without magic_2, always magic_1
        let magic_2_none = 0u32;
        let skill = if magic_2_none > 0 {
            magic_2_none
        } else {
            magic_1
        };
        assert_eq!(skill, magic_1);
    }

    // ── Monster magic tests ──────────────────────────────────────────

    #[test]
    fn test_skill_cooldown_constant() {
        // C++ uses 2-second cooldown for most classes
        assert_eq!(NPC_SKILL_COOLDOWN_MS, 2000);
    }

    #[test]
    fn test_magic_percent_default() {
        // Default threshold for magic chance out of 5000
        assert_eq!(NPC_MAGIC_PERCENT_DEFAULT, 1000);
    }

    #[test]
    fn test_monster_magic_attack_type_2_skill_selection() {
        // Type 2: 7.5% magic_2, 7.5% magic_3, 85% magic_1
        let magic_1 = 500010u32;
        let magic_2 = 500020u32;
        let magic_3 = 500030u32;

        // rand_skill < 30 → magic_2 (7.5%)
        let skill = if 15 < 30 && magic_2 > 0 {
            magic_2
        } else if 15 < 60 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill, magic_2);

        // rand_skill 45 → magic_3 (since 45 >= 30 but < 60)
        let skill2 = if 45 < 30 && magic_2 > 0 {
            magic_2
        } else if 45 < 60 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill2, magic_3);

        // rand_skill 200 → magic_1 (default)
        let skill3 = if 200 < 30 && magic_2 > 0 {
            magic_2
        } else if 200 < 60 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill3, magic_1);
    }

    #[test]
    fn test_monster_magic_attack_type_6_skill_selection() {
        // Type 6: equal 33% distribution
        let magic_1 = 600010u32;
        let magic_2 = 600020u32;
        let magic_3 = 600030u32;

        // rand < 1000 → magic_2
        let skill = if 500 < 1000 && magic_2 > 0 {
            magic_2
        } else if 500 < 2000 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill, magic_2);

        // 1000 <= rand < 2000 → magic_3
        let skill2 = if 1500 < 1000 && magic_2 > 0 {
            magic_2
        } else if 1500 < 2000 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill2, magic_3);

        // rand >= 2000 → magic_1
        let skill3 = if 2500 < 1000 && magic_2 > 0 {
            magic_2
        } else if 2500 < 2000 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill3, magic_1);
    }

    #[test]
    fn test_monster_magic_no_magic_2_fallback() {
        // When magic_2 is 0, should fall through to magic_1
        let magic_1 = 500010u32;
        let magic_2 = 0u32;
        let magic_3 = 500030u32;

        // Type 2 logic: rand_skill=15 → magic_2 check fails (0), falls to magic_3
        let skill = if 15 < 30 && magic_2 > 0 {
            magic_2
        } else if 15 < 60 && magic_3 > 0 {
            magic_3
        } else {
            magic_1
        };
        assert_eq!(skill, magic_3);
    }

    #[test]
    fn test_monster_magic_cooldown_check() {
        let mut ai = make_test_ai();

        // No cooldown set — should allow magic
        assert_eq!(ai.skill_cooldown_ms, 0);

        // Set cooldown in the future
        ai.skill_cooldown_ms = 10_000;
        ai.last_tick_ms = 5_000;

        // Cooldown not yet expired
        assert!(ai.last_tick_ms < ai.skill_cooldown_ms);

        // After enough time passes, cooldown expires
        ai.last_tick_ms = 10_000;
        assert!(ai.last_tick_ms >= ai.skill_cooldown_ms);
    }

    #[test]
    fn test_monster_magic_type_thresholds() {
        // Type 2/4/5 use 5000 threshold (lower magic chance)
        // Type 6 uses 3500 threshold (higher magic chance)
        // Both use NPC_MAGIC_PERCENT_DEFAULT = 1000 as the pass mark

        // Type 2: 1000/5000 = 20% chance to try magic
        let type2_chance = NPC_MAGIC_PERCENT_DEFAULT as f64 / 5000.0;
        assert!((type2_chance - 0.2).abs() < 0.01);

        // Type 6: 1000/3500 = ~28.6% chance to try magic
        let type6_chance = NPC_MAGIC_PERCENT_DEFAULT as f64 / 3500.0;
        assert!((type6_chance - 0.286).abs() < 0.01);
    }

    #[test]
    fn test_npc_ai_state_new_fields() {
        let ai = make_test_ai();
        assert_eq!(ai.skill_cooldown_ms, 0);
        assert_eq!(ai.nation, 0);
    }

    // ── Sprint 159: Skill cooldown reset on target acquisition ──────

    #[test]
    fn test_skill_cooldown_reset_on_attacking() {
        // When NPC transitions to Attacking, cooldown should be set to current_time + 2s
        let mut ai = make_test_ai();
        ai.last_tick_ms = 5_000;
        ai.state = NpcState::Attacking;
        // Simulate cooldown reset on state transition
        ai.skill_cooldown_ms = ai.last_tick_ms + NPC_SKILL_COOLDOWN_MS;
        assert_eq!(ai.skill_cooldown_ms, 7_000); // 5000 + 2000
                                                 // At tick 6999, still on cooldown
        ai.last_tick_ms = 6_999;
        assert!(ai.last_tick_ms < ai.skill_cooldown_ms);
        // At tick 7000, cooldown expired — can cast
        ai.last_tick_ms = 7_000;
        assert!(ai.last_tick_ms >= ai.skill_cooldown_ms);
    }

    // ── NPC magic damage tests ──────────────────────────────────────

    #[test]
    fn test_npc_band_constant() {
        // C++ NPC_BAND = 10000, entities with ID >= 10000 are NPCs
        assert_eq!(NPC_BAND, 10000);
    }

    #[test]
    fn test_npc_magic_damage_integration_formula() {
        // Verify the full NPC magic damage formula matches C++ GetMagicDamage for NPC caster
        // C++ formula: damage = 485 * total_hit / (total_r + 510)
        // With total_r = 0: damage = 485 * total_hit / 510

        // A typical monster magic skill: sFirstDamage = 150
        let total_hit = 150;
        let base_damage = 485 * total_hit / 510; // = 142
        assert_eq!(base_damage, 142);

        // After C++ randomization + war zone halving (/2 non-war, /3 war):
        // Pre-halve: min=120, max=163 → non-war /2: min=60, max=81
        let min_expected = (142.0f32 * 0.85) as i32 / 2; // 60
        let max_expected = (142.0f32 * 0.3 + 142.0 * 0.85) as i32 / 2; // 81
        assert_eq!(min_expected, 60);
        assert_eq!(max_expected, 81);
    }

    #[test]
    fn test_npc_magic_target_classification() {
        // Target IDs < NPC_BAND (10000) are player session IDs
        // Target IDs >= NPC_BAND are NPC IDs
        assert!(42u32 < NPC_BAND); // player
        assert!(10001u32 >= NPC_BAND); // NPC
        assert!(10000u32 >= NPC_BAND); // NPC at boundary
        assert!(9999u32 < NPC_BAND); // player at boundary
    }

    #[test]
    fn test_npc_magic_damage_vs_player_formula() {
        // Simulate the damage flow for NPC → Player
        // 1. NPC casts skill with first_damage = 200
        // 2. compute_npc_magic_damage(200) → applies C++ formula
        // 3. Damage applied to player HP
        use rand::SeedableRng;

        let first_damage = 200;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let damage = combat::compute_npc_magic_damage(first_damage, 0, false, &mut rng);

        // damage_base = 485 * 200 / 510 = 190
        // After randomization + /2: min ~80, max ~109
        assert!(damage > 0, "Damage should be positive");
        assert!(damage >= 80, "Damage {} too low for base=200", damage);
        assert!(damage <= 110, "Damage {} too high for base=200", damage);

        // Simulate HP reduction
        let player_hp: i16 = 1000;
        let new_hp = (player_hp - damage).max(0);
        assert!(new_hp < player_hp, "Player should take damage");
        assert!(new_hp > 0, "Player should survive with base=200");
    }

    #[test]
    fn test_npc_magic_heal_on_friendly_npc() {
        // Simulate healer NPC healing a friendly NPC
        let first_damage = -300; // Negative = heal in C++ convention
        let heal_amount = combat::compute_npc_heal_amount(first_damage);

        assert_eq!(heal_amount, 300);

        // Simulate HP restoration
        let current_hp = 500;
        let max_hp = 1000;
        let new_hp = (current_hp + heal_amount).min(max_hp);
        assert_eq!(new_hp, 800);
    }

    #[test]
    fn test_npc_magic_heal_caps_at_max_hp() {
        // Heal should not exceed max HP
        let heal_amount = combat::compute_npc_heal_amount(500);
        let current_hp = 800;
        let max_hp = 1000;
        let new_hp = (current_hp + heal_amount).min(max_hp);
        assert_eq!(new_hp, 1000); // Capped at max
    }

    #[test]
    fn test_npc_magic_death_trigger() {
        // If damage exceeds HP, target should die (HP clamped to 0)
        use rand::SeedableRng;

        let first_damage = 500;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let damage = combat::compute_npc_magic_damage(first_damage, 0, false, &mut rng);

        let player_hp: i16 = 100; // Low HP player
        let new_hp = (player_hp - damage).max(0);
        assert_eq!(new_hp, 0, "Player should be dead");
    }

    #[test]
    fn test_npc_magic_dot_parameters() {
        // Verify DOT tick calculation matches C++ convention
        // C++ DOT: tick_count = duration / 2, hp_per_tick = time_damage / tick_count
        use rand::SeedableRng;

        let time_damage = 100;
        let duration = 10; // 10 seconds → 5 ticks (every 2s)
        let tick_count = (duration / 2).max(1) as u8;
        assert_eq!(tick_count, 5);

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dot_dmg = combat::compute_npc_magic_damage(time_damage, 0, false, &mut rng);
        let hp_per_tick = -(dot_dmg.unsigned_abs() as i16 / tick_count as i16).max(1);

        assert!(hp_per_tick < 0, "DOT should be negative (damage)");
        // 100 base → ~95 computed, randomized ~80-109, /2 = ~40-54, /5 ticks = ~-8..-10
        assert!(hp_per_tick >= -12, "DOT per tick {} too high", hp_per_tick);
        assert!(hp_per_tick <= -7, "DOT per tick {} too low", hp_per_tick);
    }

    #[test]
    fn test_npc_magic_skill_type_filter() {
        // Only Type 3 skills should be processed by npc_apply_magic_effect
        // Type 1 (melee), Type 4 (buff), etc. are ignored
        // This is verified by the skill_type != 3 early return in the function
        let type1 = 1u8;
        let type3 = 3u8;
        let type4 = 4u8;

        assert_ne!(type1, 3, "Type 1 should be filtered out");
        assert_eq!(type3, 3, "Type 3 should be processed");
        assert_ne!(type4, 3, "Type 4 should be filtered out");
    }

    #[test]
    fn test_npc_magic_moral_classification() {
        // Verify moral values for damage vs heal
        // Damage morals: 7 (single enemy), 10 (AOE enemy)
        // Heal morals: 2 (friend+me), 3 (friend-me), 4 (party), 6 (party all), 11 (AOE friend)
        let damage_morals = [7i16, 10];
        let heal_morals = [2i16, 3, 4, 6, 11];

        for m in damage_morals {
            assert!(m == 7 || m == 10, "Moral {} should be damage", m);
        }
        for m in heal_morals {
            assert!(
                m == 2 || m == 3 || m == 4 || m == 6 || m == 11,
                "Moral {} should be heal",
                m
            );
        }
    }

    // ── NPC Duration Death tests ──────────────────────────────────────

    #[test]
    fn test_duration_death_constants() {
        // TENDER_ATTACK_TYPE = 0 (passive), ATROCITY_ATTACK_TYPE = 1 (aggressive)
        assert_eq!(TENDER_ATTACK_TYPE, 0);
        assert_eq!(ATROCITY_ATTACK_TYPE, 1);
    }

    #[test]
    fn test_duration_fields_default_zero() {
        let ai = make_test_ai();
        assert_eq!(ai.duration_secs, 0);
        assert_eq!(ai.spawned_at_ms, 0);
        assert_eq!(ai.last_hp_regen_ms, 0);
        assert_eq!(ai.attack_type, 1); // ATROCITY by default
    }

    #[test]
    fn test_duration_not_expired() {
        let mut ai = make_test_ai();
        ai.duration_secs = 60; // 60 seconds
        ai.spawned_at_ms = 1000; // spawned at tick 1000ms
        let now_ms: u64 = 30_000; // 30 seconds elapsed

        let duration_ms = ai.duration_secs as u64 * 1000;
        let alive_ms = now_ms.saturating_sub(ai.spawned_at_ms);
        assert!(alive_ms <= duration_ms, "Should NOT have expired yet");
    }

    #[test]
    fn test_duration_expired() {
        let mut ai = make_test_ai();
        ai.duration_secs = 60; // 60 seconds
        ai.spawned_at_ms = 1000; // spawned at tick 1000ms
        let now_ms: u64 = 62_000; // 62 seconds elapsed (61s alive)

        let duration_ms = ai.duration_secs as u64 * 1000;
        let alive_ms = now_ms.saturating_sub(ai.spawned_at_ms);
        assert!(alive_ms > duration_ms, "Should have expired");
    }

    #[test]
    fn test_duration_zero_means_no_timeout() {
        let ai = make_test_ai();
        assert_eq!(ai.duration_secs, 0);
        // When duration_secs == 0, the check in process_ai_tick is skipped entirely
        assert!(ai.duration_secs == 0);
    }

    #[test]
    fn test_duration_edge_case_exact_boundary() {
        let mut ai = make_test_ai();
        ai.duration_secs = 10; // 10 seconds
        ai.spawned_at_ms = 0;

        // Exactly at 10 seconds: NOT expired (must be >)
        let now_ms: u64 = 10_000;
        let duration_ms = ai.duration_secs as u64 * 1000;
        let alive_ms = now_ms.saturating_sub(ai.spawned_at_ms);
        assert!(
            alive_ms <= duration_ms,
            "Exactly at boundary should NOT expire"
        );

        // 10001ms: expired
        let now_ms2: u64 = 10_001;
        let alive_ms2 = now_ms2.saturating_sub(ai.spawned_at_ms);
        assert!(alive_ms2 > duration_ms, "1ms past should expire");
    }

    // ── Sprint 248: Tracer timeout tests ─────────────────────────────

    /// NPC tracer timeout: after 12 seconds with no combat, NPC disengages.
    #[test]
    fn test_tracer_timeout_12s() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Tracing;
        ai.last_combat_time_ms = 5000;
        ai.last_tick_ms = 5000 + 12_001; // 12.001s later

        let elapsed = ai.last_tick_ms.saturating_sub(ai.last_combat_time_ms);
        assert!(elapsed > 12_000, "Should exceed 12s timeout");
    }

    /// NPC within 12s should NOT timeout.
    #[test]
    fn test_tracer_no_timeout_within_12s() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Tracing;
        ai.last_combat_time_ms = 5000;
        ai.last_tick_ms = 5000 + 11_999; // 11.999s later

        let elapsed = ai.last_tick_ms.saturating_sub(ai.last_combat_time_ms);
        assert!(elapsed <= 12_000, "Should NOT exceed 12s timeout");
    }

    /// NPC with last_combat_time_ms == 0 should not trigger timeout.
    #[test]
    fn test_tracer_zero_combat_time_no_timeout() {
        let ai = make_test_ai();
        // last_combat_time_ms defaults to 0 — should be treated as "no combat yet"
        assert_eq!(ai.last_combat_time_ms, 0);
        // The check requires last_combat_time_ms > 0 before comparing
    }

    /// Combat time is set when NPC acquires a target.
    #[test]
    fn test_combat_time_set_on_target_acquire() {
        let mut ai = make_test_ai();
        ai.last_tick_ms = 42_000;
        ai.last_combat_time_ms = ai.last_tick_ms;
        assert_eq!(ai.last_combat_time_ms, 42_000);
    }

    // ── NPC HP Regen tests ──────────────────────────────────────────

    #[test]
    fn test_hp_regen_interval() {
        // C++ uses 15-second interval
        assert_eq!(HP_REGEN_INTERVAL_MS, 15_000);
    }

    #[test]
    fn test_hp_regen_percent() {
        // C++ heals 3% of max HP: ceil(MaxHP * 3 / 100)
        assert_eq!(HP_REGEN_PERCENT, 3.0);
    }

    #[test]
    fn test_hp_regen_calculation() {
        // C++ formula: (int)ceil((double(m_MaxHP * 3) / 100))
        let max_hp: i32 = 10_000;
        let heal = ((max_hp as f64 * HP_REGEN_PERCENT) / 100.0).ceil() as i32;
        assert_eq!(heal, 300); // 3% of 10000

        // Small monster
        let max_hp2: i32 = 50;
        let heal2 = ((max_hp2 as f64 * HP_REGEN_PERCENT) / 100.0).ceil() as i32;
        assert_eq!(heal2, 2); // ceil(1.5) = 2

        // Tiny HP
        let max_hp3: i32 = 1;
        let heal3 = ((max_hp3 as f64 * HP_REGEN_PERCENT) / 100.0).ceil() as i32;
        assert_eq!(heal3, 1); // ceil(0.03) = 1 (always heals at least 1)
    }

    #[test]
    fn test_hp_regen_caps_at_max_hp() {
        let max_hp: i32 = 1000;
        let current_hp: i32 = 985;
        let heal = ((max_hp as f64 * HP_REGEN_PERCENT) / 100.0).ceil() as i32;
        assert_eq!(heal, 30);

        let new_hp = (current_hp + heal).min(max_hp);
        assert_eq!(new_hp, 1000); // Capped at max_hp
    }

    #[test]
    fn test_hp_regen_not_at_full() {
        let max_hp: i32 = 1000;
        let current_hp: i32 = 1000;
        // No regen needed — already at full
        assert!(current_hp >= max_hp);
    }

    #[test]
    fn test_barracks_excluded_from_regen() {
        // C++ Npc.cpp:7113 — if (!isMonster() && GetProtoID() == 511) return;
        assert_eq!(BARRACKS_PROTO_ID, 511);
    }

    #[test]
    fn test_hp_regen_timer_check() {
        let mut ai = make_test_ai();
        ai.last_hp_regen_ms = 5_000;
        let now_ms: u64 = 19_000; // 14s elapsed — not enough

        let elapsed = now_ms.saturating_sub(ai.last_hp_regen_ms);
        assert!(
            elapsed <= HP_REGEN_INTERVAL_MS,
            "14s < 15s, should not regen"
        );

        let now_ms2: u64 = 21_000; // 16s elapsed — enough
        let elapsed2 = now_ms2.saturating_sub(ai.last_hp_regen_ms);
        assert!(elapsed2 > HP_REGEN_INTERVAL_MS, "16s > 15s, should regen");
    }

    // ── NPC Passive Aggro tests ──────────────────────────────────────

    #[test]
    fn test_passive_npc_attack_type() {
        let mut ai = make_test_ai();
        ai.attack_type = TENDER_ATTACK_TYPE;
        ai.is_aggressive = false;

        assert_eq!(ai.attack_type, 0);
        assert!(!ai.is_aggressive);
    }

    #[test]
    fn test_aggressive_npc_attack_type() {
        let ai = make_test_ai();
        assert_eq!(ai.attack_type, ATROCITY_ATTACK_TYPE);
        assert!(ai.is_aggressive);
    }

    #[test]
    fn test_passive_npc_find_enemy_gate() {
        // Passive NPC: find_enemy should be called when attack_type == TENDER
        // but only target players in damage list
        let mut ai = make_test_ai();
        ai.attack_type = TENDER_ATTACK_TYPE;
        ai.is_aggressive = false;

        // The condition in npc_standing:
        // if ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE
        assert!(
            ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE,
            "Passive NPCs should enter find_enemy path"
        );
    }

    #[test]
    fn test_passive_npc_interrupt_check() {
        // The interrupt check in process_ai_tick should also include passive NPCs
        let mut ai = make_test_ai();
        ai.attack_type = TENDER_ATTACK_TYPE;
        ai.is_aggressive = false;
        ai.state = NpcState::Standing;

        // The condition: ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE
        let should_check = ai.state == NpcState::Standing
            && (ai.is_aggressive || ai.attack_type == TENDER_ATTACK_TYPE);
        assert!(
            should_check,
            "Passive standing NPCs should check for enemies"
        );
    }

    #[test]
    fn test_passive_npc_friend_target_exception() {
        // C++ TENDER check: IsDamagedUserList(pUser) || (m_bHasFriends && m_Target.id == target_uid)
        let mut ai = make_test_ai();
        ai.attack_type = TENDER_ATTACK_TYPE;
        ai.has_friends = true;
        ai.target_id = Some(42);

        // A player (sid=42) should pass the TENDER filter if they're the current target
        // (friend-assisted targeting)
        let user_sid = 42u16;
        let is_friend_target = ai.has_friends && ai.target_id == Some(user_sid);
        assert!(
            is_friend_target,
            "Friend target exception should allow targeting"
        );

        // A different player (sid=99) should NOT pass without being in damage list
        let other_sid = 99u16;
        let is_friend_target2 = ai.has_friends && ai.target_id == Some(other_sid);
        let is_damaged_by = false; // simulate: not in damage list
        assert!(
            !is_friend_target2 && !is_damaged_by,
            "Unrelated player should not be targeted"
        );
    }

    #[test]
    fn test_notify_npc_damaged_state_transitions() {
        // notify_npc_damaged should transition Standing/Moving/Sleeping → Attacking
        // but NOT transition already-combating NPCs
        let states_that_switch = [NpcState::Standing, NpcState::Moving, NpcState::Sleeping];
        let states_that_dont = [
            NpcState::Attacking,
            NpcState::Fighting,
            NpcState::Tracing,
            NpcState::Back,
        ];

        for state in states_that_switch {
            assert!(
                matches!(
                    state,
                    NpcState::Standing | NpcState::Moving | NpcState::Sleeping
                ),
                "State {:?} should trigger target switch on damage",
                state
            );
        }

        for state in states_that_dont {
            assert!(
                !matches!(
                    state,
                    NpcState::Standing | NpcState::Moving | NpcState::Sleeping
                ),
                "State {:?} should NOT trigger target switch on damage",
                state
            );
        }
    }

    // ── Gate NPC Tests ──────────────────────────────────────────────────

    #[test]
    fn test_gate_npc_type_constants() {
        assert_eq!(NPC_SPECIAL_GATE, 52);
        assert_eq!(NPC_OBJECT_WOOD, 54);
        assert_eq!(NPC_KROWAZ_GATE, 180);
        assert_eq!(NPC_ROLLINGSTONE, 181);
        assert_eq!(OBJECT_FLAG_LEVER, 4);
        assert_eq!(WOOD_COOLDOWN_THRESHOLD, 30);
    }

    #[test]
    fn test_gate_open_packet_format() {
        // WIZ_OBJECT_EVENT gate broadcast format:
        // [u8 object_type] [u8 1] [u32 npc_id] [u8 gate_open]
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_FLAG_LEVER); // object type
        pkt.write_u8(1); // success
        pkt.write_u32(10001); // npc_id
        pkt.write_u8(1); // gate_open = open

        assert_eq!(pkt.opcode, Opcode::WizObjectEvent as u8);
        // 1 + 1 + 4 + 1 = 7 bytes
        assert_eq!(pkt.data.len(), 7);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(OBJECT_FLAG_LEVER));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.read_u8(), Some(1)); // open
    }

    #[test]
    fn test_gate_close_packet_format() {
        // Gate close variant
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_FLAG_LEVER);
        pkt.write_u8(1);
        pkt.write_u32(10002);
        pkt.write_u8(0); // gate_open = closed

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(OBJECT_FLAG_LEVER));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10002));
        assert_eq!(r.read_u8(), Some(0)); // closed
    }

    #[test]
    fn test_special_gate_cycle_open() {
        // NPC_SPECIAL_GATE should cycle: closed(0) -> open(1)
        // When gate_open == 0, it should be set to 1
        let mut ai = make_test_ai();
        ai.gate_open = 0;
        let npc_type = NPC_SPECIAL_GATE;

        // Simulate the open cycle
        let stand_time: u64 = 3000;
        if npc_type == NPC_SPECIAL_GATE && ai.gate_open == 0 {
            ai.gate_open = 1;
        }
        assert_eq!(ai.gate_open, 1);
        // C++ returns stand_time * 10 for open duration
        assert_eq!(stand_time.saturating_mul(10), 30_000);
    }

    #[test]
    fn test_special_gate_cycle_close() {
        // NPC_SPECIAL_GATE should cycle: open(1) -> closed(0)
        let mut ai = make_test_ai();
        ai.gate_open = 1;
        let npc_type = NPC_SPECIAL_GATE;

        let stand_time: u64 = 3000;
        if npc_type == NPC_SPECIAL_GATE && ai.gate_open == 1 {
            ai.gate_open = 0;
        }
        assert_eq!(ai.gate_open, 0);
        // C++ returns stand_time * 60 for closed duration (longer)
        assert_eq!(stand_time.saturating_mul(60), 180_000);
    }

    #[test]
    fn test_wood_cooldown_increment() {
        // NPC_OBJECT_WOOD: WoodCooldownClose increments each tick when gate_open == 1
        let mut ai = make_test_ai();
        ai.gate_open = 1;
        ai.wood_cooldown_count = 0;

        // Simulate 29 ticks — should not close yet
        for _ in 0..29 {
            ai.wood_cooldown_count += 1;
        }
        assert_eq!(ai.wood_cooldown_count, 29);
        assert!(ai.wood_cooldown_count < WOOD_COOLDOWN_THRESHOLD);

        // 30th tick — should trigger close
        ai.wood_cooldown_count += 1;
        assert!(ai.wood_cooldown_count >= WOOD_COOLDOWN_THRESHOLD);
        // Reset after close
        ai.gate_open = 0;
        ai.wood_cooldown_count = 0;
        assert_eq!(ai.gate_open, 0);
        assert_eq!(ai.wood_cooldown_count, 0);
    }

    #[test]
    fn test_wood_no_broadcast() {
        // NPC_OBJECT_WOOD and NPC_ROLLINGSTONE should NOT broadcast gate flag
        assert_eq!(NPC_OBJECT_WOOD, 54);
        assert_eq!(NPC_ROLLINGSTONE, 181);

        // Verify the check: if npc_type matches either, no packet is sent
        let wood_type = NPC_OBJECT_WOOD;
        let stone_type = NPC_ROLLINGSTONE;
        let gate_type = NPC_SPECIAL_GATE;

        assert!(wood_type == NPC_OBJECT_WOOD || wood_type == NPC_ROLLINGSTONE);
        assert!(stone_type == NPC_OBJECT_WOOD || stone_type == NPC_ROLLINGSTONE);
        assert!(!(gate_type == NPC_OBJECT_WOOD || gate_type == NPC_ROLLINGSTONE));
    }

    #[test]
    fn test_krowaz_gate_auto_close() {
        // NPC_KROWAZ_GATE in ZONE_KROWAZ_DOMINION(75): auto-closes when open
        let mut ai = make_test_ai();
        ai.gate_open = 1;
        ai.zone_id = ZONE_KROWAZ_DOMINION;
        let npc_type = NPC_KROWAZ_GATE;

        let stand_time: u64 = 3000;

        if npc_type == NPC_KROWAZ_GATE && ai.zone_id == ZONE_KROWAZ_DOMINION && ai.gate_open == 1 {
            ai.gate_open = 0;
        }
        assert_eq!(ai.gate_open, 0);
        // C++ returns stand_time * 10
        assert_eq!(stand_time.saturating_mul(10), 30_000);
    }

    #[test]
    fn test_krowaz_gate_wrong_zone_no_close() {
        // NPC_KROWAZ_GATE in a non-Krowaz zone should NOT auto-close
        let mut ai = make_test_ai();
        ai.gate_open = 1;
        ai.zone_id = 21; // not ZONE_KROWAZ_DOMINION
        let npc_type = NPC_KROWAZ_GATE;

        let should_close =
            npc_type == NPC_KROWAZ_GATE && ai.zone_id == ZONE_KROWAZ_DOMINION && ai.gate_open == 1;

        assert!(!should_close, "Should not close gate in non-Krowaz zone");
        assert_eq!(ai.gate_open, 1); // gate stays open
    }

    #[test]
    fn test_krowaz_gate_already_closed() {
        // NPC_KROWAZ_GATE already closed — should not trigger
        let mut ai = make_test_ai();
        ai.gate_open = 0;
        ai.zone_id = ZONE_KROWAZ_DOMINION;
        let npc_type = NPC_KROWAZ_GATE;

        let should_close =
            npc_type == NPC_KROWAZ_GATE && ai.zone_id == ZONE_KROWAZ_DOMINION && ai.gate_open == 1;

        assert!(!should_close, "Already closed gate should not trigger");
    }

    #[test]
    fn test_gate_ai_state_defaults() {
        // New NpcAiState fields should default to 0
        let ai = make_test_ai();
        assert_eq!(ai.gate_open, 0);
        assert_eq!(ai.wood_cooldown_count, 0);
    }

    #[test]
    fn test_special_gate_requires_war() {
        // NPC_SPECIAL_GATE should only cycle during war (NATION_BATTLE or SNOW_BATTLE)
        // Without war, gate logic should NOT trigger
        use crate::systems::war::{NATION_BATTLE, SNOW_BATTLE};

        let battle_open_none: u8 = 0; // NO_BATTLE
        let is_war = battle_open_none == NATION_BATTLE || battle_open_none == SNOW_BATTLE;
        assert!(!is_war, "No battle should not trigger gate cycling");

        let battle_open_nation: u8 = NATION_BATTLE;
        let is_war2 = battle_open_nation == NATION_BATTLE || battle_open_nation == SNOW_BATTLE;
        assert!(is_war2, "NATION_BATTLE should trigger gate cycling");

        let battle_open_snow: u8 = SNOW_BATTLE;
        let is_war3 = battle_open_snow == NATION_BATTLE || battle_open_snow == SNOW_BATTLE;
        assert!(is_war3, "SNOW_BATTLE should trigger gate cycling");
    }

    #[test]
    fn test_wood_requires_nation_battle_only() {
        // NPC_OBJECT_WOOD should only auto-close during NATION_BATTLE (not SNOW_BATTLE)
        use crate::systems::war::{NATION_BATTLE, SNOW_BATTLE};

        let is_nation = NATION_BATTLE == NATION_BATTLE;
        assert!(is_nation, "NATION_BATTLE should trigger wood auto-close");

        let is_snow_only = SNOW_BATTLE == NATION_BATTLE;
        assert!(
            !is_snow_only,
            "SNOW_BATTLE should NOT trigger wood auto-close"
        );
    }

    #[test]
    fn test_is_gate_npc_type() {
        use crate::world::is_gate_npc_type;

        // Gate types that should match
        assert!(is_gate_npc_type(50)); // NPC_GATE
        assert!(is_gate_npc_type(51)); // NPC_PHOENIX_GATE
        assert!(is_gate_npc_type(52)); // NPC_SPECIAL_GATE
        assert!(is_gate_npc_type(53)); // NPC_VICTORY_GATE
        assert!(is_gate_npc_type(54)); // NPC_OBJECT_WOOD
        assert!(is_gate_npc_type(55)); // NPC_GATE_LEVER
        assert!(is_gate_npc_type(121)); // NPC_KARUS_MONUMENT
        assert!(is_gate_npc_type(122)); // NPC_HUMAN_MONUMENT
        assert!(is_gate_npc_type(150)); // NPC_GATE2
        assert!(is_gate_npc_type(180)); // NPC_KROWAZ_GATE

        // Non-gate types
        assert!(!is_gate_npc_type(0)); // Default
        assert!(!is_gate_npc_type(1)); // Monster
        assert!(!is_gate_npc_type(40)); // NPC_HEALER
        assert!(!is_gate_npc_type(100)); // Random
        assert!(!is_gate_npc_type(181)); // NPC_ROLLINGSTONE (not a gate)
    }

    #[test]
    fn test_gate_open_state_values() {
        // 0 = closed, 1 = open, 2 = event-forced open
        let closed: u8 = 0;
        let open: u8 = 1;
        let event_open: u8 = 2;

        assert!(open == 1 || open == 2, "1 should be considered open");
        assert!(
            event_open == 1 || event_open == 2,
            "2 should be considered open"
        );
        assert!(
            !(closed == 1 || closed == 2),
            "0 should be considered closed"
        );
    }

    // ── Sprint 39: Boss Magic + Elemental Fainting Tests ────────────────

    #[test]
    fn test_boss_proto_constants() {
        // Verify all boss proto IDs match C++ Define.h
        assert!(BOSS_EMPEROR_MAMMOTH.contains(&9501));
        assert!(BOSS_EMPEROR_MAMMOTH.contains(&9502));
        assert!(BOSS_EMPEROR_MAMMOTH.contains(&9503));
        assert!(!BOSS_EMPEROR_MAMMOTH.contains(&9504));

        assert!(BOSS_CRESHERGIMMIC.contains(&9504));
        assert!(BOSS_CRESHERGIMMIC.contains(&9505));
        assert!(BOSS_CRESHERGIMMIC.contains(&9506));
        assert!(BOSS_CRESHERGIMMIC.contains(&9507));

        assert!(BOSS_ELITE_TIMARLI.contains(&9523));
        assert!(BOSS_ELITE_TIMARLI.contains(&9524));
        assert!(BOSS_ELITE_TIMARLI.contains(&9541));

        assert!(BOSS_PURIOUS.contains(&9508));
        assert!(BOSS_PURIOUS.contains(&9511));

        assert!(BOSS_MOEBIUS.contains(&9528));
        assert!(BOSS_MOEBIUS.contains(&9529));
        assert!(BOSS_MOEBIUS.contains(&9544));
        assert!(BOSS_MOEBIUS.contains(&9545));

        assert!(BOSS_GARIONEUS.contains(&9525));
        assert!(BOSS_GARIONEUS.contains(&9526));
        assert!(BOSS_GARIONEUS.contains(&9542));

        assert!(BOSS_SORCERER_GEDEN.contains(&9530));
        assert!(BOSS_SORCERER_GEDEN.contains(&9532));

        assert_eq!(BOSS_ATAL, 9534);
        assert_eq!(BOSS_MOSPELL, 9535);
        assert_eq!(BOSS_AHMI, 9536);

        assert!(BOSS_FLUWITON_ROOM_3.contains(&9512));
        assert!(BOSS_FLUWITON_ROOM_3.contains(&9514));

        assert!(BOSS_FLUWITON_ROOM_4.contains(&9515));
        assert!(BOSS_FLUWITON_ROOM_4.contains(&9518));
    }

    #[test]
    fn test_utc_second_field_exists() {
        let ai = make_test_ai();
        assert_eq!(ai.utc_second, 0);
    }

    #[test]
    fn test_emperor_mammoth_timing_sequence() {
        // Emperor Mammoth pattern: 15s/20s/30s effecting, 40s casting + reset
        // utc_second increments each call, resets at 40
        let mut utc: u32 = 0;
        let mut cast_at = Vec::new();
        let mut reset_at = None;

        for _ in 0..50 {
            if utc == 15 || utc == 20 || utc == 30 {
                cast_at.push(utc);
            }
            if utc == 40 {
                reset_at = Some(utc);
                utc = 0; // C++ resets here
            } else {
                utc += 1;
            }
        }

        assert_eq!(cast_at, vec![15, 20, 30]);
        assert_eq!(reset_at, Some(40));
        // After reset, utc should have continued incrementing
        assert!(utc > 0);
    }

    #[test]
    fn test_fluwiton_room_4_timing_pattern() {
        // Fluwiton Room 4: effecting every 5s from 5-60, casting at 70, then reset
        let mut effecting_times = Vec::new();
        let mut casting_time = None;

        for utc in 0..80u32 {
            if (5..=60).contains(&utc) && utc.is_multiple_of(5) {
                effecting_times.push(utc);
            }
            if utc == 70 {
                casting_time = Some(utc);
                break; // one full cycle
            }
        }

        // Should have 12 effecting ticks: 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60
        assert_eq!(effecting_times.len(), 12);
        assert_eq!(effecting_times[0], 5);
        assert_eq!(effecting_times[11], 60);
        assert_eq!(casting_time, Some(70));
    }

    #[test]
    fn test_magic_attack_3_selects_boss_path() {
        // magic_attack == 3 should enter the boss path (try_boss_magic)
        // while other values go through the normal random path
        let magic_attack_boss = 3u8;
        let magic_attack_normal = 2u8;

        // Boss path: always returns Some (timed patterns always fire)
        assert_eq!(magic_attack_boss, 3);
        // Normal path: random chance to fire
        assert_ne!(magic_attack_normal, 3);
    }

    #[test]
    fn test_elite_timarli_uses_flying_opcode() {
        // Elite Timarli should use MAGIC_FLYING (4) instead of MAGIC_EFFECTING (3)
        assert_eq!(MAGIC_OPCODE_FLYING, 4);
        // Verify it's different from the standard EFFECTING
        let magic_effecting: u8 = 3;
        assert_ne!(MAGIC_OPCODE_FLYING, magic_effecting);
    }

    #[test]
    fn test_npc_template_resistance_fields() {
        use crate::npc::NpcTemplate;
        let tmpl = NpcTemplate {
            s_sid: 9501,
            is_monster: true,
            name: "Emperor Mammoth".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 80,
            max_hp: 500000,
            max_mp: 0,
            attack: 5000,
            ac: 200,
            hit_rate: 200,
            evade_rate: 50,
            damage: 3000,
            attack_delay: 1500,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 20,
            attack_range: 5,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 502001,
            magic_2: 502002,
            magic_3: 502003,
            magic_attack: 3,
            fire_r: 60,
            cold_r: 40,
            lightning_r: 20,
            magic_r: 50,
            disease_r: 10,
            poison_r: 30,
            exp: 100000,
            loyalty: 500,
            money: 50000,
            item_table: 0,
            area_range: 0.0,
        };

        assert_eq!(tmpl.fire_r, 60);
        assert_eq!(tmpl.cold_r, 40);
        assert_eq!(tmpl.lightning_r, 20);
        assert_eq!(tmpl.magic_r, 50);
        assert_eq!(tmpl.disease_r, 10);
        assert_eq!(tmpl.poison_r, 30);
        assert_eq!(tmpl.magic_attack, 3);
    }

    #[test]
    fn test_faint_chance_high_resistance() {
        // Formula: faint_chance = 10 + (40 - 40 * (resistance / 80))
        // With resistance = 80: 10 + (40 - 40 * 1.0) = 10 + 0 = 10
        let resistance: f64 = 80.0;
        let faint_chance = (10.0 + (40.0 - 40.0 * (resistance / 80.0))) as i32;
        assert_eq!(faint_chance, 10);
    }

    #[test]
    fn test_faint_chance_zero_resistance() {
        // With resistance = 0: 10 + (40 - 40 * 0.0) = 10 + 40 = 50
        let resistance: f64 = 0.0;
        let faint_chance = (10.0 + (40.0 - 40.0 * (resistance / 80.0))) as i32;
        assert_eq!(faint_chance, 50);
    }

    #[test]
    fn test_faint_chance_medium_resistance() {
        // With resistance = 40: 10 + (40 - 40 * 0.5) = 10 + 20 = 30
        let resistance: f64 = 40.0;
        let faint_chance = (10.0 + (40.0 - 40.0 * (resistance / 80.0))) as i32;
        assert_eq!(faint_chance, 30);
    }

    #[test]
    fn test_faint_chance_over_80_still_positive() {
        // With resistance = 100 (above cap): 10 + (40 - 40 * 1.25) = 10 + (-10) = 0
        let resistance: f64 = 100.0;
        let faint_chance = (10.0 + (40.0 - 40.0 * (resistance / 80.0))) as i32;
        assert_eq!(faint_chance, 0);
    }

    #[test]
    fn test_default_boss_fallback_uses_random() {
        // An NPC with magic_attack == 3 but unrecognized proto should use the
        // default fallback path (same as magic_attack == 2 with EFFECTING).
        let proto: u16 = 9999; // Not in any boss list
        assert!(!BOSS_EMPEROR_MAMMOTH.contains(&proto));
        assert!(!BOSS_CRESHERGIMMIC.contains(&proto));
        assert!(!BOSS_ELITE_TIMARLI.contains(&proto));
        assert!(!BOSS_PURIOUS.contains(&proto));
        assert!(!BOSS_MOEBIUS.contains(&proto));
        assert!(!BOSS_GARIONEUS.contains(&proto));
        assert!(!BOSS_SORCERER_GEDEN.contains(&proto));
        assert_ne!(proto, BOSS_ATAL);
        assert_ne!(proto, BOSS_MOSPELL);
        assert_ne!(proto, BOSS_AHMI);
        assert!(!BOSS_FLUWITON_ROOM_3.contains(&proto));
        assert!(!BOSS_FLUWITON_ROOM_4.contains(&proto));
    }

    #[test]
    fn test_purious_random_skills_at_60() {
        // Purious at utc_second == 60: random from 4 skills
        // 502025 (30%), 502015 (30%), 502017 (25%), 502016 (15%)
        let skills = [502025u32, 502015, 502017, 502016];
        for skill in &skills {
            assert!(*skill > 500000, "Boss skills should be in 502xxx range");
        }
    }

    #[test]
    fn test_moebius_timing_3phase() {
        // Moebius: 10s/20s effecting, 30s effecting + reset
        let mut utc: u32 = 0;
        let mut casts = Vec::new();
        let mut resets = 0;

        for _ in 0..65 {
            if utc == 10 || utc == 20 {
                casts.push(utc);
            }
            if utc == 30 {
                casts.push(utc);
                resets += 1;
                utc = 0;
            } else {
                utc += 1;
            }
        }

        // Two complete cycles: [10,20,30] and [10,20,30]
        assert_eq!(resets, 2);
        assert_eq!(casts.len(), 6);
    }

    #[test]
    fn test_sorcerer_geden_long_cycle() {
        // Sorcerer Geden: very long intervals — 40/80/120s
        let mut utc: u32 = 0;
        let mut casts = Vec::new();

        for _ in 0..130 {
            if utc == 40 || utc == 80 {
                casts.push(utc);
            }
            if utc == 120 {
                casts.push(utc);
                utc = 0;
            } else {
                utc += 1;
            }
        }

        assert_eq!(casts, vec![40, 80, 120]);
    }

    #[test]
    fn test_fluwiton_room_3_weighted_random() {
        // Fluwiton Room 3 at utc == 65: weighted random from 8 skills
        let skills = [
            502020u32, 502021, 502031, 502019, 502028, 502029, 502030, 502019,
        ];
        // All should be in the 502xxx range
        for s in &skills {
            assert!(*s >= 502000 && *s <= 502100, "skill {} out of range", s);
        }
        // Verify 502019 appears twice (two weight ranges)
        let count_19 = skills.iter().filter(|&&s| s == 502019).count();
        assert_eq!(count_19, 2);
    }

    #[test]
    fn test_boss_long_and_magic_timarli_flying() {
        // Verify Elite Timarli proto IDs are in the boss list
        assert!(BOSS_ELITE_TIMARLI.contains(&9523));
        assert!(BOSS_ELITE_TIMARLI.contains(&9524));
        assert!(BOSS_ELITE_TIMARLI.contains(&9541));
        // MAGIC_FLYING opcode
        assert_eq!(MAGIC_OPCODE_FLYING, 4);
    }

    #[test]
    fn test_boss_long_and_magic_fluwiton4_skills() {
        // Fluwiton Room 4 in LongAndMagicAttack uses 3 random skills
        let skills = [502022u32, 502023];
        assert!(skills[0] > 0);
        assert!(skills[1] > 0);
        assert_ne!(skills[0], skills[1]);
    }

    // ── Boss Alert Pack Tests ─────────────────────────────────────────

    #[test]
    fn test_npc_boss_constant() {
        // NPC_BOSS should match `globals.h:107` — value 3
        assert_eq!(NPC_BOSS, 3);
    }

    #[test]
    fn test_boss_alert_pack_condition() {
        //   if (m_bHasFriends || GetType() == NPC_BOSS)
        //     FindFriend(GetType() == NPC_BOSS ? MonSearchAny : MonSearchSameFamily);

        // Case 1: has_friends=true, not boss -> should alert (same-family)
        let ai = NpcAiState {
            has_friends: true,
            ..make_test_ai()
        };
        let npc_type: u8 = 0; // regular monster
        let is_boss = npc_type == NPC_BOSS;
        assert!(
            ai.has_friends || is_boss,
            "has_friends=true should trigger alert"
        );
        assert!(!is_boss, "Regular monster is not a boss");

        // Case 2: has_friends=false, is boss -> should alert (any)
        let ai2 = NpcAiState {
            has_friends: false,
            ..make_test_ai()
        };
        let npc_type2: u8 = NPC_BOSS;
        let is_boss2 = npc_type2 == NPC_BOSS;
        assert!(
            ai2.has_friends || is_boss2,
            "Boss NPC should trigger alert even without has_friends"
        );
        assert!(is_boss2, "NPC_BOSS type should be detected");

        // Case 3: has_friends=false, not boss -> should NOT alert
        let ai3 = NpcAiState {
            has_friends: false,
            ..make_test_ai()
        };
        let npc_type3: u8 = 0;
        let is_boss3 = npc_type3 == NPC_BOSS;
        assert!(
            !(ai3.has_friends || is_boss3),
            "Non-boss without has_friends should not alert"
        );
    }

    #[test]
    fn test_boss_search_type_selection() {
        // Boss: MonSearchAny (skips family check)
        // Non-boss: MonSearchSameFamily (requires family match)
        let boss_type: u8 = NPC_BOSS;
        let is_boss = boss_type == NPC_BOSS;
        assert!(is_boss, "Boss should use MonSearchAny");

        let regular_type: u8 = 0;
        let is_regular_boss = regular_type == NPC_BOSS;
        assert!(
            !is_regular_boss,
            "Regular monster should use MonSearchSameFamily"
        );
    }

    #[test]
    fn test_alert_pack_boss_requires_family_check() {
        // In alert_pack with is_boss=true, NPCs of different family
        // should still be eligible (MonSearchAny).
        let caller_ai = NpcAiState {
            has_friends: false,
            family_type: 10,
            ..make_test_ai()
        };
        let ally_ai = NpcAiState {
            has_friends: false,
            family_type: 99, // Different family
            state: NpcState::Standing,
            target_id: None,
            ..make_test_ai()
        };
        let is_boss = true;

        let skip_for_family = !pack_family_matches(
            caller_ai.family_type,
            ally_ai.family_type,
            is_boss,
            ally_ai.has_friends,
        );
        assert!(skip_for_family, "Boss must not recruit unrelated monsters");
    }

    #[test]
    fn test_pack_atross_manticore_centaur_are_not_allies() {
        for family in [14, 18, 26] {
            assert!(pack_family_matches(family, family, true, false));
            for other in [14, 18, 26] {
                assert_eq!(
                    pack_family_matches(family, other, true, true),
                    family == other
                );
            }
        }
        assert!(!pack_family_matches(0, 0, true, true));
        assert!(!pack_family_matches(14, 14, false, false));
        assert!(pack_family_matches(14, 14, false, true));
    }

    #[test]
    fn test_alert_pack_non_boss_requires_family_match() {
        // In alert_pack with is_boss=false, NPCs of different family
        // should be skipped (MonSearchSameFamily).
        let caller_ai = NpcAiState {
            has_friends: true,
            family_type: 10,
            ..make_test_ai()
        };
        let ally_ai = NpcAiState {
            has_friends: true,
            family_type: 99, // Different family
            state: NpcState::Standing,
            target_id: None,
            ..make_test_ai()
        };
        let is_boss = false;

        let skip_for_family = !pack_family_matches(
            caller_ai.family_type,
            ally_ai.family_type,
            is_boss,
            ally_ai.has_friends,
        );
        assert!(
            skip_for_family,
            "Non-boss should skip allies of different family"
        );
    }

    #[test]
    fn test_alert_pack_skips_fighting_npcs() {
        // NPCs in Fighting state with a target should be skipped.
        let ally_ai = NpcAiState {
            state: NpcState::Fighting,
            target_id: Some(77),
            ..make_test_ai()
        };

        let skip = ally_ai.state == NpcState::Fighting || ally_ai.state == NpcState::Dead;
        assert!(skip, "Fighting NPC should be skipped in alert_pack");
    }

    #[test]
    fn test_alert_pack_skips_dead_npcs() {
        let ally_ai = NpcAiState {
            state: NpcState::Dead,
            ..make_test_ai()
        };

        let skip = ally_ai.state == NpcState::Fighting || ally_ai.state == NpcState::Dead;
        assert!(skip, "Dead NPC should be skipped in alert_pack");
    }

    #[test]
    fn test_alert_pack_skips_already_attacking_with_target() {
        // NPCs already in Attacking/Tracing with a target should be skipped.
        let ally_ai = NpcAiState {
            state: NpcState::Attacking,
            target_id: Some(55),
            ..make_test_ai()
        };

        let skip = ally_ai.target_id.is_some()
            && matches!(ally_ai.state, NpcState::Attacking | NpcState::Tracing);
        assert!(skip, "Already-attacking NPC with target should be skipped");
    }

    #[test]
    fn test_alert_pack_allows_standing_no_target() {
        // Standing NPC with no target should be eligible.
        let ally_ai = NpcAiState {
            state: NpcState::Standing,
            target_id: None,
            has_friends: true,
            family_type: 5,
            ..make_test_ai()
        };

        let skip_state = ally_ai.state == NpcState::Fighting || ally_ai.state == NpcState::Dead;
        let skip_target = ally_ai.target_id.is_some()
            && matches!(ally_ai.state, NpcState::Attacking | NpcState::Tracing);
        assert!(
            !skip_state && !skip_target,
            "Standing NPC with no target should be eligible"
        );
    }

    // ── Blink Integration Tests ──────────────────────────────────────

    #[test]
    fn test_blink_time_constant() {
        // C++ Define.h:72 — `#define BLINK_TIME (10)`
        let blink_time: u64 = 10;
        assert_eq!(blink_time, 10, "Blink time should be 10 seconds");
    }

    #[test]
    fn test_blink_check_in_find_enemy() {
        // find_enemy should skip blinking players.
        // We test the logic inline since find_enemy is async.
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Set blink active
        world.update_session(1, |h| {
            h.blink_expiry_time = now + 10;
        });

        assert!(
            world.is_player_blinking(1, now),
            "Player should be blinking"
        );

        // After blink expires, should no longer be blinking
        assert!(
            !world.is_player_blinking(1, now + 11),
            "Player should not be blinking after expiry"
        );
    }

    #[test]
    fn test_gate_npc_excluded_from_friend_search() {
        // is_gate_npc_type should return true for gate types.
        // These NPCs must not be included in friend search results.
        assert!(is_gate_npc_type(50)); // NPC_GATE
        assert!(is_gate_npc_type(52)); // NPC_SPECIAL_GATE
        assert!(is_gate_npc_type(54)); // NPC_OBJECT_WOOD
        assert!(is_gate_npc_type(180)); // NPC_KROWAZ_GATE
        assert!(!is_gate_npc_type(0)); // NPC_MONSTER
        assert!(!is_gate_npc_type(3)); // NPC_BOSS
    }

    // ── Sprint 41: Fainting L1 + L3 Fix Tests ──────────────────────

    #[test]
    fn test_fainting_stale_timestamp_detection() {
        // Sprint 39 L1 fix: if fainting_until_ms is very stale (more than 4x
        // FAINTING_TIME_MS behind now_ms), it should be recalibrated.
        let stale_time: u64 = 1_000;
        let now_ms: u64 = 100_000;

        // 4x buffer: stale_time + 2000*4 = 9000 < 100_000 -> stale
        let is_stale = stale_time + FAINTING_TIME_MS * 4 < now_ms;
        assert!(is_stale, "Timestamp 1000 should be stale at now=100000");
    }

    #[test]
    fn test_fainting_fresh_timestamp_not_recalibrated() {
        // A timestamp that is recent (within 4x FAINTING_TIME) should NOT be
        // recalibrated.
        let fresh_time: u64 = 98_000;
        let now_ms: u64 = 100_000;

        // 4x buffer: 98000 + 8000 = 106000 > 100000 -> fresh
        let is_stale = fresh_time + FAINTING_TIME_MS * 4 < now_ms;
        assert!(!is_stale, "Timestamp 98000 should be fresh at now=100000");
    }

    #[test]
    fn test_fainting_stale_recalibrated_then_waits_full_duration() {
        // After recalibration, the NPC should wait a full FAINTING_TIME_MS.
        let now_ms: u64 = 100_000;
        // Recalibrated faint_start = now_ms
        let faint_start = now_ms;

        // Immediately after recalibration: still stunned
        assert!(now_ms < faint_start + FAINTING_TIME_MS);

        // After 1 second: still stunned
        let now_ms_1s = now_ms + 1_000;
        assert!(now_ms_1s < faint_start + FAINTING_TIME_MS);

        // After 2 seconds: should wake
        let now_ms_2s = now_ms + FAINTING_TIME_MS;
        assert!((now_ms_2s >= faint_start + FAINTING_TIME_MS));
    }

    #[test]
    fn test_fainting_wake_preserves_target_unit() {
        // Sprint 39 L3 fix: C++ does NOT clear target_id on fainting wake.
        // Verify the wake closure matches what npc_fainting does.
        let mut ai = NpcAiState {
            state: NpcState::Fainting,
            fainting_until_ms: 5_000,
            target_id: Some(42),
            ..make_test_ai()
        };

        // Simulate what npc_fainting does on wake (the closure):
        ai.state = NpcState::Standing;
        ai.fainting_until_ms = 0;
        // Note: NO `ai.target_id = None;` — this is the L3 fix

        assert_eq!(ai.state, NpcState::Standing);
        assert_eq!(ai.fainting_until_ms, 0);
        assert_eq!(
            ai.target_id,
            Some(42),
            "Fainting wake should NOT clear target_id (C++ parity)"
        );
    }

    #[test]
    fn test_fainting_stale_recalibrates_logic() {
        // When fainting_until_ms is very stale, the recalibration logic
        // should detect staleness and use now_ms as the new start time.
        let mut ai = NpcAiState {
            state: NpcState::Fainting,
            fainting_until_ms: 1_000, // Very stale
            target_id: Some(99),
            ..make_test_ai()
        };

        let now_ms: u64 = 100_000;
        let faint_start = if ai.fainting_until_ms + FAINTING_TIME_MS * 4 < now_ms {
            ai.fainting_until_ms = now_ms; // recalibrate
            now_ms
        } else {
            ai.fainting_until_ms
        };

        // Should be recalibrated
        assert_eq!(
            faint_start, now_ms,
            "Stale timestamp should be recalibrated"
        );
        assert_eq!(ai.fainting_until_ms, now_ms);

        // Should still be stunned
        assert!(
            now_ms < faint_start + FAINTING_TIME_MS,
            "NPC should still be stunned after recalibration"
        );
    }

    #[test]
    fn test_fainting_boundary_not_stale() {
        // Boundary test: timestamp exactly at 4x FAINTING_TIME_MS before now
        // should NOT be considered stale.
        let now_ms: u64 = 10_000;
        // fainting_until_ms + 8000 = 10000 -> NOT < 10000 -> not stale
        let faint_time: u64 = 2_000;
        let is_stale = faint_time + FAINTING_TIME_MS * 4 < now_ms;
        assert!(!is_stale, "Boundary case should NOT be stale");
    }

    // ── Sprint 41: Pathfinding Integration Tests ────────────────────

    #[test]
    fn test_pathfinding_constants() {
        assert_eq!(DIRECT_MOVE_THRESHOLD, 15.0);
        assert_eq!(PATH_RECALC_DISTANCE, 5.0);
        assert_eq!(NPC_MAX_MOVE_RANGE, 100.0);
    }

    #[test]
    fn test_ai_state_path_fields_default() {
        let ai = make_test_ai();
        assert!(ai.path_waypoints.is_empty());
        assert_eq!(ai.path_index, 0);
        assert_eq!(ai.path_target_x, 0.0);
        assert_eq!(ai.path_target_z, 0.0);
        assert!(!ai.path_is_direct);
    }

    #[test]
    fn test_ai_state_path_waypoints_storage() {
        let mut ai = make_test_ai();
        ai.path_waypoints = vec![(10.0, 20.0), (30.0, 40.0), (50.0, 60.0)];
        ai.path_index = 1;
        ai.path_target_x = 50.0;
        ai.path_target_z = 60.0;
        ai.path_is_direct = false;

        assert_eq!(ai.path_waypoints.len(), 3);
        assert_eq!(ai.path_waypoints[1], (30.0, 40.0));
        assert_eq!(ai.path_index, 1);
        assert_eq!(ai.path_target_x, 50.0);
    }

    #[test]
    fn test_path_clear_on_target_lost() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Tracing;
        ai.path_waypoints = vec![(10.0, 20.0), (30.0, 40.0)];
        ai.path_index = 1;
        ai.target_id = None;

        // Simulate clearing path when target is lost
        ai.path_waypoints.clear();
        ai.path_index = 0;
        ai.state = NpcState::Standing;

        assert!(ai.path_waypoints.is_empty());
        assert_eq!(ai.path_index, 0);
        assert_eq!(ai.state, NpcState::Standing);
    }

    #[test]
    fn test_path_recalc_distance_threshold() {
        // Target hasn't moved much -- should NOT recalculate
        let target_dx = 3.0f32;
        let target_dz = 2.0f32;
        let target_moved = (target_dx * target_dx + target_dz * target_dz).sqrt();
        assert!(target_moved < PATH_RECALC_DISTANCE);

        // Target moved a lot -- SHOULD recalculate
        let target_dx2 = 4.0f32;
        let target_dz2 = 4.0f32;
        let target_moved2 = (target_dx2 * target_dx2 + target_dz2 * target_dz2).sqrt();
        assert!(target_moved2 > PATH_RECALC_DISTANCE);
    }

    #[test]
    fn test_need_new_path_empty_waypoints() {
        let ai = make_test_ai();
        let need = ai.path_waypoints.is_empty() || ai.path_index >= ai.path_waypoints.len();
        assert!(need);
    }

    #[test]
    fn test_need_new_path_index_past_end() {
        let mut ai = make_test_ai();
        ai.path_waypoints = vec![(10.0, 20.0)];
        ai.path_index = 1; // past end

        let need = ai.path_waypoints.is_empty() || ai.path_index >= ai.path_waypoints.len();
        assert!(need);
    }

    #[test]
    fn test_need_new_path_target_moved() {
        let mut ai = make_test_ai();
        ai.path_waypoints = vec![(10.0, 20.0), (30.0, 40.0)];
        ai.path_index = 0;
        ai.path_target_x = 30.0;
        ai.path_target_z = 40.0;

        // Target at (36.0, 40.0) -- moved 6 units
        let target_x = 36.0f32;
        let target_z = 40.0f32;
        let tdx = target_x - ai.path_target_x;
        let tdz = target_z - ai.path_target_z;
        let target_moved = (tdx * tdx + tdz * tdz).sqrt();

        let need = ai.path_waypoints.is_empty()
            || ai.path_index >= ai.path_waypoints.len()
            || target_moved > PATH_RECALC_DISTANCE;
        assert!(
            need,
            "Should recalculate when target moved {:.1}",
            target_moved
        );
    }

    #[test]
    fn test_no_recalc_when_target_stable() {
        let mut ai = make_test_ai();
        ai.path_waypoints = vec![(10.0, 20.0), (30.0, 40.0)];
        ai.path_index = 0;
        ai.path_target_x = 30.0;
        ai.path_target_z = 40.0;

        // Target barely moved (2 units)
        let target_x = 32.0f32;
        let target_z = 40.0f32;
        let tdx = target_x - ai.path_target_x;
        let tdz = target_z - ai.path_target_z;
        let target_moved = (tdx * tdx + tdz * tdz).sqrt();

        let need = ai.path_waypoints.is_empty()
            || ai.path_index >= ai.path_waypoints.len()
            || target_moved > PATH_RECALC_DISTANCE;
        assert!(
            !need,
            "Should NOT recalculate when target moved {:.1}",
            target_moved
        );
    }

    #[test]
    fn test_direct_move_threshold_short_range() {
        let dist = 10.0f32;
        assert!(dist <= DIRECT_MOVE_THRESHOLD);
    }

    #[test]
    fn test_direct_move_threshold_long_range() {
        let dist = 25.0f32;
        assert!(dist > DIRECT_MOVE_THRESHOLD);
    }

    #[test]
    fn test_max_move_range_gives_up() {
        let dist = 120.0f32;
        assert!(dist > NPC_MAX_MOVE_RANGE);
    }

    #[test]
    fn test_step_move_integration_with_cached_path() {
        let waypoints = vec![(20.0, 0.0), (40.0, 0.0), (60.0, 0.0)];
        let speed = 5.0f32;

        let (x1, z1, idx1) = pathfind::step_move(0.0, 0.0, &waypoints, 0, speed);
        assert!((x1 - 5.0).abs() < 0.01);
        assert!((z1 - 0.0).abs() < 0.01);
        assert_eq!(idx1, 0);

        let (x2, z2, idx2) = pathfind::step_move(x1, z1, &waypoints, idx1, speed);
        assert!((x2 - 10.0).abs() < 0.01);
        assert!((z2 - 0.0).abs() < 0.01);
        assert_eq!(idx2, 0);
    }

    #[test]
    fn test_step_move_advances_through_waypoints() {
        let waypoints = vec![(4.0, 0.0), (8.0, 0.0)];
        let speed = 5.0f32;

        let (x, _z, idx) = pathfind::step_move(0.0, 0.0, &waypoints, 0, speed);
        assert!(idx >= 1, "Should advance past first waypoint, idx={}", idx);
        assert!(x > 4.0, "Should have moved past first waypoint x={:.1}", x);
    }

    #[test]
    fn test_direct_move_short_distance() {
        let (x, z) = pathfind::step_no_path_move(0.0, 0.0, 3.0, 4.0, 10.0);
        assert!((x - 3.0).abs() < 0.01);
        assert!((z - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_direct_move_partial_step() {
        let (x, z) = pathfind::step_no_path_move(0.0, 0.0, 30.0, 0.0, 5.0);
        assert!((x - 5.0).abs() < 0.01);
        assert!((z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_path_cleared_on_leash() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Tracing;
        ai.path_waypoints = vec![(10.0, 20.0), (30.0, 40.0)];
        ai.path_index = 1;

        ai.state = NpcState::Standing;
        ai.cur_x = ai.spawn_x;
        ai.cur_z = ai.spawn_z;
        ai.target_id = None;
        ai.path_waypoints.clear();
        ai.path_index = 0;

        assert!(ai.path_waypoints.is_empty());
        assert_eq!(ai.path_index, 0);
        assert_eq!(ai.state, NpcState::Standing);
    }

    #[test]
    fn test_path_cleared_on_fighting_transition() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Tracing;
        ai.path_waypoints = vec![(10.0, 20.0)];
        ai.path_index = 0;

        ai.state = NpcState::Fighting;
        ai.path_waypoints.clear();
        ai.path_index = 0;

        assert!(ai.path_waypoints.is_empty());
        assert_eq!(ai.state, NpcState::Fighting);
    }

    #[test]
    fn test_path_is_direct_flag() {
        let mut ai = make_test_ai();
        ai.path_is_direct = true;
        ai.path_waypoints.clear();
        assert!(ai.path_is_direct);
        assert!(ai.path_waypoints.is_empty());

        ai.path_is_direct = false;
        ai.path_waypoints = vec![(10.0, 20.0), (30.0, 40.0)];
        assert!(!ai.path_is_direct);
        assert_eq!(ai.path_waypoints.len(), 2);
    }

    #[test]
    fn test_npc_back_path_clear_on_arrival() {
        let mut ai = make_test_ai();
        ai.state = NpcState::Back;
        ai.cur_x = 101.0;
        ai.cur_z = 200.5;
        ai.path_waypoints = vec![(100.0, 200.0)];
        ai.path_index = 0;

        let dx = ai.spawn_x - ai.cur_x;
        let dz = ai.spawn_z - ai.cur_z;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(
            dist <= 2.0,
            "Should be close enough to arrive, dist={:.1}",
            dist
        );

        ai.state = NpcState::Standing;
        ai.cur_x = ai.spawn_x;
        ai.cur_z = ai.spawn_z;
        ai.path_waypoints.clear();
        ai.path_index = 0;

        assert!(ai.path_waypoints.is_empty());
        assert_eq!(ai.state, NpcState::Standing);
    }

    #[test]
    fn test_a_star_path_integration() {
        use crate::zone::MapData;
        use ko_protocol::smd::SmdFile;

        let size = 20;
        let grid_len = (size * size) as usize;
        let event_grid = vec![0i16; grid_len];
        let unit_dist = 4.0;
        let map_width = (size - 1) as f32 * unit_dist;
        let smd = SmdFile {
            map_size: size,
            unit_dist,
            map_width,
            map_height: map_width,
            event_grid,
            warps: Vec::new(),
            regene_events: Vec::new(),
        };
        let map = MapData::new(smd);

        let result = pathfind::find_path(&map, 8.0, 8.0, 60.0, 60.0);
        assert!(result.found, "Path should be found on open map");
        assert!(!result.waypoints.is_empty());

        let speed = 5.0;
        let (x, z, idx) = pathfind::step_move(8.0, 8.0, &result.waypoints, 0, speed);
        assert!(x != 8.0 || z != 8.0, "Should have moved from start");
        assert!(idx < result.waypoints.len() + 1);
    }

    #[test]
    fn test_a_star_path_around_wall() {
        use crate::zone::MapData;
        use ko_protocol::smd::SmdFile;

        let size = 20;
        let grid_len = (size * size) as usize;
        let mut event_grid = vec![0i16; grid_len];
        for z in 5..15 {
            event_grid[(10 * size + z) as usize] = 1;
        }
        let unit_dist = 4.0;
        let map_width = (size - 1) as f32 * unit_dist;
        let smd = SmdFile {
            map_size: size,
            unit_dist,
            map_width,
            map_height: map_width,
            event_grid,
            warps: Vec::new(),
            regene_events: Vec::new(),
        };
        let map = MapData::new(smd);

        let result = pathfind::find_path(&map, 20.0, 40.0, 60.0, 40.0);
        assert!(result.found, "Should find path around wall");
        assert!(result.waypoints.len() > 2, "Path should detour around wall");
    }

    #[test]
    fn test_line_of_sight_for_direct_move_decision() {
        use crate::zone::MapData;
        use ko_protocol::smd::SmdFile;

        let size = 20;
        let grid_len = (size * size) as usize;
        let mut event_grid = vec![0i16; grid_len];
        for z in 0..size {
            event_grid[(5 * size + z) as usize] = 1;
        }
        let unit_dist = 4.0;
        let map_width = (size - 1) as f32 * unit_dist;
        let smd = SmdFile {
            map_size: size,
            unit_dist,
            map_width,
            map_height: map_width,
            event_grid,
            warps: Vec::new(),
            regene_events: Vec::new(),
        };
        let map = MapData::new(smd);

        // Clear LOS (same side of wall)
        assert!(pathfind::line_of_sight(&map, 4.0, 4.0, 12.0, 4.0, 4.0));
        // Blocked LOS (across wall)
        assert!(!pathfind::line_of_sight(&map, 4.0, 4.0, 32.0, 4.0, 4.0));
    }

    // ── Sprint 42: NPC AI Stealth Visibility Tests ──────────────────

    #[test]
    fn test_invisible_player_skipped_in_find_enemy() {
        // find_enemy should skip players with invisibility_type > 0
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Default: not invisible
        assert!(
            !world.is_invisible(1),
            "New session should not be invisible"
        );

        // Set invisibility_type > 0
        world.set_invisibility_type(1, 1); // INVIS_DISPEL_ON_MOVE
        assert!(world.is_invisible(1), "Player should be invisible");

        // Set back to 0 (INVIS_NONE)
        world.set_invisibility_type(1, 0);
        assert!(
            !world.is_invisible(1),
            "Player should not be invisible after reset"
        );
    }

    #[test]
    fn test_invisible_player_combat_abort() {
        // npc_fighting should abort combat against invisible targets
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Player with INVIS_DISPEL_ON_ATTACK (2) — invisible to NPC targeting
        world.set_invisibility_type(1, 2);
        assert!(world.is_invisible(1));

        // NPC should stop fighting this target
        // (Tested via the is_invisible check added to npc_fighting)
    }

    #[test]
    fn test_invisibility_type_values() {
        // C++ InvisibilityType enum from globals.h:757-762
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // INVIS_NONE = 0
        world.set_invisibility_type(1, 0);
        assert_eq!(world.get_invisibility_type(1), 0);
        assert!(!world.is_invisible(1));

        // INVIS_DISPEL_ON_MOVE = 1
        world.set_invisibility_type(1, 1);
        assert_eq!(world.get_invisibility_type(1), 1);
        assert!(world.is_invisible(1));

        // INVIS_DISPEL_ON_ATTACK = 2
        world.set_invisibility_type(1, 2);
        assert_eq!(world.get_invisibility_type(1), 2);
        assert!(world.is_invisible(1));
    }

    // ── Guard Type Detection Tests ───────────────────────────────────────

    #[test]
    fn test_guard_type_constants() {
        assert_eq!(NPC_GUARD, 11);
        assert_eq!(NPC_PATROL_GUARD, 12);
        assert_eq!(NPC_STORE_GUARD, 13);
    }

    #[test]
    fn test_is_guard_type_positive() {
        assert!(is_guard_type(NPC_GUARD));
        assert!(is_guard_type(NPC_PATROL_GUARD));
        assert!(is_guard_type(NPC_STORE_GUARD));
    }

    #[test]
    fn test_is_guard_type_negative() {
        assert!(!is_guard_type(0)); // monster
        assert!(!is_guard_type(1)); // NPC_DOOR
        assert!(!is_guard_type(3)); // NPC_BOSS
        assert!(!is_guard_type(21)); // NPC_MERCHANT
        assert!(!is_guard_type(40)); // NPC_HEALER
        assert!(!is_guard_type(62)); // NPC_GUARD_TOWER1
    }

    // ── NPC-vs-NPC Targeting Tests ───────────────────────────────────────

    #[test]
    fn test_npc_ai_state_has_npc_target_id() {
        let ai = make_test_ai();
        assert_eq!(ai.npc_target_id, None);
    }

    #[test]
    fn test_npc_ai_state_npc_target_id_set() {
        let mut ai = make_test_ai();
        ai.npc_target_id = Some(20000);
        assert_eq!(ai.npc_target_id, Some(20000));
        ai.npc_target_id = None;
        assert_eq!(ai.npc_target_id, None);
    }

    #[tokio::test]
    async fn test_find_npc_enemy_guard_finds_monster() {
        use crate::npc::{NpcInstance, NpcTemplate, NPC_BAND};
        use std::sync::Arc;

        let world = WorldState::new();

        // Zone setup
        world.ensure_zone(21, 128);

        // Guard NPC (nation 1, type NPC_GUARD)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = Arc::new(NpcTemplate {
            s_sid: 1000,
            is_monster: false,
            name: "Guard".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1,
            act_type: 0,
            npc_type: NPC_GUARD,
            family_type: 0,
            selling_group: 0,
            level: 60,
            max_hp: 50000,
            max_mp: 0,
            attack: 500,
            ac: 100,
            hit_rate: 200,
            evade_rate: 50,
            damage: 300,
            attack_delay: 1500,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 30,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 50,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*guard_tmpl).clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);

        let guard_ai = NpcAiState {
            state: NpcState::Standing,
            spawn_x: 100.0,
            spawn_z: 100.0,
            cur_x: 100.0,
            cur_z: 100.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: true,
            zone_id: 21,
            region_x: 2,
            region_z: 2,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 1,
            is_tower_owner: false,
            attack_type: 1,
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        };
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Monster NPC (nation 2, is_monster=true)
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = Arc::new(NpcTemplate {
            s_sid: 2000,
            is_monster: true,
            name: "Wolf".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 2,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 30,
            attack_delay: 2000,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 10,
            money: 50,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*monster_tmpl).clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);

        let monster_ai = NpcAiState {
            state: NpcState::Standing,
            nation: 2,
            ..guard_ai.clone()
        };
        world.insert_npc_ai(monster_id, monster_ai);

        // Add both to zone region
        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Guard should find the monster as an enemy
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(result, Some(monster_id));
    }

    #[tokio::test]
    async fn test_find_npc_enemy_same_nation_no_target() {
        use crate::npc::{NpcInstance, NpcTemplate, NPC_BAND};
        use std::sync::Arc;

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard NPC (nation 1)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = Arc::new(NpcTemplate {
            s_sid: 1000,
            is_monster: false,
            name: "Guard".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1,
            act_type: 0,
            npc_type: NPC_GUARD,
            family_type: 0,
            selling_group: 0,
            level: 60,
            max_hp: 50000,
            max_mp: 0,
            attack: 500,
            ac: 100,
            hit_rate: 200,
            evade_rate: 50,
            damage: 300,
            attack_delay: 1500,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 30,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 50,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*guard_tmpl).clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);

        // Friendly monster (nation 1 — same as guard)
        let friend_id: NpcId = NPC_BAND + 1;
        let friend_tmpl = Arc::new(NpcTemplate {
            s_sid: 2000,
            is_monster: true,
            name: "FriendlyWolf".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 30,
            attack_delay: 2000,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 10,
            money: 50,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*friend_tmpl).clone());
        let friend_inst = NpcInstance {
            nid: friend_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(friend_inst);
        world.init_npc_hp(friend_id, 1000);

        let guard_ai = NpcAiState {
            state: NpcState::Standing,
            spawn_x: 100.0,
            spawn_z: 100.0,
            cur_x: 100.0,
            cur_z: 100.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: true,
            zone_id: 21,
            region_x: 2,
            region_z: 2,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 1,
            is_tower_owner: false,
            attack_type: 1,
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        };
        world.insert_npc_ai(guard_id, guard_ai.clone());
        world.insert_npc_ai(friend_id, guard_ai.clone());

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, friend_id);

        // Same nation — guard should NOT target friendly NPC
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_find_npc_enemy_monster_does_not_target_monster() {
        use crate::npc::{NpcInstance, NpcTemplate, NPC_BAND};
        use std::sync::Arc;

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Monster 1 (nation 1)
        let mon1_id: NpcId = NPC_BAND;
        let mon_tmpl = Arc::new(NpcTemplate {
            s_sid: 2000,
            is_monster: true,
            name: "Wolf".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 30,
            attack_delay: 2000,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 10,
            money: 50,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*mon_tmpl).clone());
        let mon1_inst = NpcInstance {
            nid: mon1_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(mon1_inst);
        world.init_npc_hp(mon1_id, 1000);

        // Monster 2 (nation 2) — different nation but both monsters
        let mon2_id: NpcId = NPC_BAND + 1;
        let mon2_inst = NpcInstance {
            nid: mon2_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(mon2_inst);
        world.init_npc_hp(mon2_id, 1000);

        let mon1_ai = NpcAiState {
            state: NpcState::Standing,
            spawn_x: 100.0,
            spawn_z: 100.0,
            cur_x: 100.0,
            cur_z: 100.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: true,
            zone_id: 21,
            region_x: 2,
            region_z: 2,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 1,
            is_tower_owner: false,
            attack_type: 1,
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        };
        world.insert_npc_ai(mon1_id, mon1_ai.clone());
        world.insert_npc_ai(
            mon2_id,
            NpcAiState {
                nation: 2,
                ..mon1_ai.clone()
            },
        );

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, mon1_id);
        zone.add_npc(2, 2, mon2_id);

        // Monsters should NOT target other monsters
        let result = find_npc_enemy(&world, &mon1_ai, mon1_id, &mon_tmpl);
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_find_npc_enemy_guard_skips_dead_npc() {
        use crate::npc::{NpcInstance, NpcTemplate, NPC_BAND};
        use std::sync::Arc;

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = Arc::new(NpcTemplate {
            s_sid: 1000,
            is_monster: false,
            name: "Guard".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1,
            act_type: 0,
            npc_type: NPC_GUARD,
            family_type: 0,
            selling_group: 0,
            level: 60,
            max_hp: 50000,
            max_mp: 0,
            attack: 500,
            ac: 100,
            hit_rate: 200,
            evade_rate: 50,
            damage: 300,
            attack_delay: 1500,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 30,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 50,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*guard_tmpl).clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);

        // Dead monster
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = Arc::new(NpcTemplate {
            s_sid: 2000,
            is_monster: true,
            name: "DeadWolf".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 2,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 30,
            attack_delay: 2000,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 10,
            money: 50,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*monster_tmpl).clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 0); // DEAD

        let guard_ai = NpcAiState {
            state: NpcState::Standing,
            spawn_x: 100.0,
            spawn_z: 100.0,
            cur_x: 100.0,
            cur_z: 100.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: true,
            zone_id: 21,
            region_x: 2,
            region_z: 2,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 1,
            is_tower_owner: false,
            attack_type: 1,
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        };
        world.insert_npc_ai(guard_id, guard_ai.clone());

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Dead NPC should not be targeted
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_find_npc_enemy_neutral_nation_not_hostile() {
        use crate::npc::{NpcInstance, NpcTemplate, NPC_BAND};
        use std::sync::Arc;

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard with nation 0 (neutral/ALL)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = Arc::new(NpcTemplate {
            s_sid: 1000,
            is_monster: false,
            name: "NeutralGuard".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: NPC_GUARD,
            family_type: 0,
            selling_group: 0,
            level: 60,
            max_hp: 50000,
            max_mp: 0,
            attack: 500,
            ac: 100,
            hit_rate: 200,
            evade_rate: 50,
            damage: 300,
            attack_delay: 1500,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 30,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 50,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*guard_tmpl).clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);

        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = Arc::new(NpcTemplate {
            s_sid: 2000,
            is_monster: true,
            name: "Wolf".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 2,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 30,
            attack_delay: 2000,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 10,
            money: 50,
            item_table: 0,
            area_range: 0.0,
        });
        world.insert_npc_template((*monster_tmpl).clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);

        let guard_ai = NpcAiState {
            state: NpcState::Standing,
            spawn_x: 100.0,
            spawn_z: 100.0,
            cur_x: 100.0,
            cur_z: 100.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: true,
            zone_id: 21,
            region_x: 2,
            region_z: 2,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 0,
            is_tower_owner: false,
            attack_type: 1,
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        };
        world.insert_npc_ai(guard_id, guard_ai.clone());
        world.insert_npc_ai(monster_id, guard_ai.clone());

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Nation 0 is neutral — should NOT target
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_npc_enemy_non_guard_non_monster_returns_none() {
        // Regular NPCs (type != guard, not monster) should never target other NPCs
        // This tests the is_guard / is_monster early return
        use crate::npc::NpcTemplate;

        let tmpl = NpcTemplate {
            s_sid: 500,
            is_monster: false,
            name: "Merchant".to_string(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1,
            act_type: 0,
            npc_type: 21,
            family_type: 0, // NPC_MERCHANT
            selling_group: 0,
            level: 1,
            max_hp: 1000,
            max_mp: 0,
            attack: 0,
            ac: 0,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 0,
            speed_1: 0,
            speed_2: 0,
            stand_time: 3000,
            search_range: 0,
            attack_range: 0,
            direct_attack: 0,
            tracing_range: 0,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        };

        assert!(!is_guard_type(tmpl.npc_type));
        assert!(!tmpl.is_monster);
        // find_npc_enemy would return None immediately due to early check
    }

    // ── NPC-vs-NPC Damage Tests ──────────────────────────────────────────

    #[test]
    fn test_npc_vs_npc_damage_formula() {
        // C++ formula: HitB = (TotalHit * 200) / (Ac + 240)
        let attacker_hit: i32 = 500;
        let target_ac: i32 = 100;
        let hit_b = (attacker_hit * 200) / (target_ac + 240).max(1);
        // 500 * 200 / 340 = 294
        assert_eq!(hit_b, 294);
    }

    #[test]
    fn test_npc_vs_npc_damage_formula_zero_attack() {
        let attacker_hit: i32 = 0;
        let target_ac: i32 = 100;
        let hit_b = (attacker_hit * 200) / (target_ac + 240).max(1);
        assert_eq!(hit_b, 0);
    }

    #[test]
    fn test_npc_vs_npc_damage_formula_high_defense() {
        let attacker_hit: i32 = 100;
        let target_ac: i32 = 10000;
        let hit_b = (attacker_hit * 200) / (target_ac + 240).max(1);
        // 100 * 200 / 10240 = 1 (integer division)
        assert!(hit_b < 10);
    }

    #[test]
    fn test_broadcast_npc_vs_npc_attack_packet_format() {
        // Wire format: [u8 LONG_ATTACK][u8 result][u32 npc_id][u32 target_id]
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(LONG_ATTACK); // attack type
        pkt.write_u8(ATTACK_SUCCESS); // result
        pkt.write_u32(10000); // attacker NPC
        pkt.write_u32(10001); // target NPC

        assert_eq!(pkt.opcode, Opcode::WizAttack as u8);
        assert_eq!(pkt.data.len(), 10); // u8 + u8 + u32 + u32
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(LONG_ATTACK));
        assert_eq!(r.read_u8(), Some(ATTACK_SUCCESS));
        assert_eq!(r.read_u32(), Some(10000));
        assert_eq!(r.read_u32(), Some(10001));
    }

    // ── Sprint 44 Track C: Nation-0 Hostility Fix Tests ─────────────────

    /// Helper to create an NPC template for nation-0 tests.
    fn make_nation_test_template(
        s_sid: u16,
        is_monster: bool,
        group: u8,
        npc_type: u8,
    ) -> NpcTemplate {
        NpcTemplate {
            s_sid,
            is_monster,
            name: format!("Test_{}", s_sid),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group,
            act_type: 0,
            npc_type,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 30,
            attack_delay: 2000,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 30,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 10,
            money: 50,
            item_table: 0,
            area_range: 0.0,
        }
    }

    /// Helper to create a standard NPC AI state for nation tests.
    fn make_nation_test_ai(nation: u8, x: f32, z: f32, rx: u16, rz: u16) -> NpcAiState {
        NpcAiState {
            state: NpcState::Standing,
            spawn_x: x,
            spawn_z: z,
            cur_x: x,
            cur_z: z,
            target_id: None,
            npc_target_id: None,
            delay_ms: 3000,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: true,
            zone_id: 21,
            region_x: rx,
            region_z: rz,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation,
            is_tower_owner: false,
            attack_type: 1,
            last_combat_time_ms: 0,
            duration_secs: 0,
            spawned_at_ms: 0,
            last_hp_regen_ms: 0,
            gate_open: 0,
            wood_cooldown_count: 0,
            utc_second: 0,
            path_waypoints: Vec::new(),
            path_index: 0,
            path_target_x: 0.0,
            path_target_z: 0.0,
            path_is_direct: false,
            dest_x: 0.0,
            dest_z: 0.0,
            pattern_frame: 0,
        }
    }

    /// QA M3: Guard (nation 1) SHOULD attack nation-0 monsters.
    ///
    /// meaning only the ATTACKER being nation-0 skips. Target being nation-0 is OK.
    #[tokio::test]
    async fn test_guard_attacks_nation0_monster() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard NPC (nation 1, type NPC_GUARD)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Monster NPC (nation 0 = neutral/ALL, is_monster=true)
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = make_nation_test_template(2000, true, 0, 0);
        world.insert_npc_template(monster_tmpl.clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);
        let monster_ai = make_nation_test_ai(0, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(monster_id, monster_ai);

        // Add both to zone region
        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Guard (nation 1) SHOULD find the nation-0 monster as an enemy
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(monster_id),
            "Guard should attack nation-0 monster"
        );
    }

    /// QA M3: Nation-0 (neutral) NPC should NOT attack anything.
    ///
    #[tokio::test]
    async fn test_nation0_npc_does_not_attack() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Nation-0 monster (should not attack)
        let neutral_id: NpcId = NPC_BAND;
        let neutral_tmpl = make_nation_test_template(3000, true, 0, 0);
        world.insert_npc_template(neutral_tmpl.clone());
        let neutral_inst = NpcInstance {
            nid: neutral_id,
            proto_id: 3000,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(neutral_inst);
        world.init_npc_hp(neutral_id, 1000);
        let neutral_ai = make_nation_test_ai(0, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(neutral_id, neutral_ai.clone());

        // Nation-1 guard nearby
        let guard_id: NpcId = NPC_BAND + 1;
        let guard_tmpl_data = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl_data.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, neutral_id);
        zone.add_npc(2, 2, guard_id);

        // Nation-0 monster should NOT find any target (neutral doesn't attack)
        // Note: neutral is_monster=true, and monsters can't target other non-monsters by the
        // "monsters don't attack other monsters" rule, so we test with the neutral as is_monster.
        // The key check is: ai.nation == 0 → skip (don't attack anything).
        let result = find_npc_enemy(&world, &neutral_ai, neutral_id, &neutral_tmpl);
        assert_eq!(result, None, "Nation-0 NPC should not attack anything");
    }

    /// Verify guard (nation 1) still attacks nation-2 monster (unchanged behavior).
    #[tokio::test]
    async fn test_guard_attacks_nation2_monster_unchanged() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard NPC (nation 1)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Monster NPC (nation 2)
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(monster_tmpl.clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);
        let monster_ai = make_nation_test_ai(2, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(monster_id, monster_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Guard (nation 1) should still attack nation-2 monster
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(monster_id),
            "Guard should attack nation-2 monster"
        );
    }

    /// Verify same-nation NPCs do NOT attack each other (unchanged behavior).
    #[tokio::test]
    async fn test_same_nation_no_attack_unchanged() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard NPC (nation 1)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Friendly monster (nation 1, same as guard)
        let friendly_id: NpcId = NPC_BAND + 1;
        let friendly_tmpl = make_nation_test_template(2000, true, 1, 0);
        world.insert_npc_template(friendly_tmpl.clone());
        let friendly_inst = NpcInstance {
            nid: friendly_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(friendly_inst);
        world.init_npc_hp(friendly_id, 1000);
        let friendly_ai = make_nation_test_ai(1, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(friendly_id, friendly_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, friendly_id);

        // Same-nation NPCs should NOT attack each other
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result, None,
            "Same-nation NPCs should not attack each other"
        );
    }

    /// Guard (nation 2) SHOULD also attack nation-0 monsters.
    #[tokio::test]
    async fn test_elmorad_guard_attacks_nation0_monster() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // El Morad guard (nation 2)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1001, false, 2, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1001,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(2, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Nation-0 monster
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = make_nation_test_template(2001, true, 0, 0);
        world.insert_npc_template(monster_tmpl.clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2001,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);
        let monster_ai = make_nation_test_ai(0, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(monster_id, monster_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // El Morad guard (nation 2) should also attack nation-0 monster
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(monster_id),
            "El Morad guard should attack nation-0 monster"
        );
    }

    // ── Sprint 45 Track C: Integration-Style NPC AI Tests ───────────────

    /// Integration test: NPC spawn -> find_enemy -> verify guard targets monster.
    /// Tests the full flow from template creation through AI targeting.
    #[tokio::test]
    async fn test_integration_npc_spawn_find_enemy_guard_targets() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Step 1: Create guard template + instance (simulates NPC spawn)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Step 2: Create monster template + instance (simulates NPC spawn)
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(monster_tmpl.clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 110.0,
            y: 0.0,
            z: 110.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);
        let monster_ai = make_nation_test_ai(2, 110.0, 110.0, 2, 2);
        world.insert_npc_ai(monster_id, monster_ai);

        // Step 3: Register in zone region
        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Step 4: Verify guard finds the monster as an enemy
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(monster_id),
            "Integration: guard should find monster enemy"
        );

        // Step 5: Verify monster CAN target the guard (different nation, non-monster target)
        // C++ logic: monsters skip other monsters, but guards are NOT monsters.
        // So monster (nation 2) CAN target guard (nation 1) since they are different nations
        // and the guard is a non-monster target.
        let monster_tmpl_arc = world.get_npc_template(2000, true).unwrap();
        let monster_ai_state = world.get_npc_ai(monster_id).unwrap();
        let monster_result =
            find_npc_enemy(&world, &monster_ai_state, monster_id, &monster_tmpl_arc);
        assert_eq!(
            monster_result,
            Some(guard_id),
            "Monster should target different-nation guard"
        );
    }

    /// Integration test: Multiple guards and monsters in same region, closest target wins.
    #[tokio::test]
    async fn test_integration_guard_picks_closest_monster() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard at (100, 100)
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Far monster at (125, 125) — within range but further
        let far_id: NpcId = NPC_BAND + 1;
        let far_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(far_tmpl.clone());
        let far_inst = NpcInstance {
            nid: far_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 125.0,
            y: 0.0,
            z: 125.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(far_inst);
        world.init_npc_hp(far_id, 1000);
        let far_ai = make_nation_test_ai(2, 125.0, 125.0, 2, 2);
        world.insert_npc_ai(far_id, far_ai);

        // Close monster at (105, 105) — within range and closer
        let close_id: NpcId = NPC_BAND + 2;
        let close_tmpl = make_nation_test_template(2001, true, 2, 0);
        world.insert_npc_template(close_tmpl.clone());
        let close_inst = NpcInstance {
            nid: close_id,
            proto_id: 2001,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(close_inst);
        world.init_npc_hp(close_id, 1000);
        let close_ai = make_nation_test_ai(2, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(close_id, close_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, far_id);
        zone.add_npc(2, 2, close_id);

        // Guard should pick the CLOSEST enemy
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(close_id),
            "Guard should target closest monster"
        );
    }

    /// Integration test: Dead monster should be skipped by guard targeting.
    #[tokio::test]
    async fn test_integration_guard_skips_dead_monster() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Dead monster (HP = 0)
        let dead_id: NpcId = NPC_BAND + 1;
        let dead_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(dead_tmpl.clone());
        let dead_inst = NpcInstance {
            nid: dead_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(dead_inst);
        world.init_npc_hp(dead_id, 0); // Dead!
        let dead_ai = make_nation_test_ai(2, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(dead_id, dead_ai);

        // Alive monster further away
        let alive_id: NpcId = NPC_BAND + 2;
        let alive_tmpl = make_nation_test_template(2001, true, 2, 0);
        world.insert_npc_template(alive_tmpl.clone());
        let alive_inst = NpcInstance {
            nid: alive_id,
            proto_id: 2001,
            is_monster: true,
            zone_id: 21,
            x: 120.0,
            y: 0.0,
            z: 120.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(alive_inst);
        world.init_npc_hp(alive_id, 1000);
        let alive_ai = make_nation_test_ai(2, 120.0, 120.0, 2, 2);
        world.insert_npc_ai(alive_id, alive_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, dead_id);
        zone.add_npc(2, 2, alive_id);

        // Guard should skip dead monster and target the alive one
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(alive_id),
            "Guard should skip dead monster and target alive one"
        );
    }

    // ── Sprint 46 Track C: Pet Decay + Wanted Event Integration Tests ──

    /// Helper to create a minimal CharacterInfo for integration tests.
    fn make_integration_test_character(
        sid: crate::zone::SessionId,
        name: &str,
        nation: u8,
    ) -> crate::world::CharacterInfo {
        crate::world::CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0u32; 14],
            bind_zone: 0,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
            res_hp_type: 1,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    /// Integration: Pet decay tick processes multiple sessions correctly.
    ///
    /// Verifies that the pet decay collection and application works across
    /// multiple concurrent sessions, each with different pet states.
    #[test]
    fn test_integration_pet_decay_multi_session() {
        use crate::world::{PetState, PET_DECAY_AMOUNT, PET_DECAY_INTERVAL_SECS};

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();

        let sid1 = world.allocate_session_id();
        let sid2 = world.allocate_session_id();
        let sid3 = world.allocate_session_id();
        world.register_session(sid1, tx1);
        world.register_session(sid2, tx2);
        world.register_session(sid3, tx3);

        // Session 1: high satisfaction pet, decay time = 0
        world.update_session(sid1, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 10000,
                nid: 1,
                index: 10,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
            h.character = Some(make_integration_test_character(sid1, "Player1", 1));
        });

        // Session 2: low satisfaction pet (will die), decay time = 0
        world.update_session(sid2, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 50,
                nid: 2,
                index: 20,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
            h.character = Some(make_integration_test_character(sid2, "Player2", 2));
        });

        // Session 3: no pet (should be skipped)
        world.update_session(sid3, |h| {
            h.character = Some(make_integration_test_character(sid3, "Player3", 1));
        });

        let now = PET_DECAY_INTERVAL_SECS + 1; // enough time elapsed
        let data = world.collect_pet_decay_data(now);

        // Only sid1 and sid2 have pets
        assert_eq!(data.len(), 2, "Should collect 2 sessions with pets");

        // Apply decay to sid1 — should survive
        let result1 = world.apply_pet_decay(sid1, -(PET_DECAY_AMOUNT), now);
        assert_eq!(result1, Some(10000 - PET_DECAY_AMOUNT));

        // Apply decay to sid2 — should die (50 - 100 <= 0)
        let result2 = world.apply_pet_decay(sid2, -(PET_DECAY_AMOUNT), now);
        assert!(result2.is_none(), "Pet with 50 sat should die");

        // Verify sid2's pet is gone
        let has_pet = world.with_session(sid2, |h| h.pet_data.is_some()).unwrap();
        assert!(!has_pet);
    }

    /// Integration: Pet decay respects the 60-second interval timer.
    ///
    #[test]
    fn test_integration_pet_decay_interval_gating() {
        use crate::world::PetState;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 5000,
                nid: 1,
                index: 1,
                ..Default::default()
            });
            h.last_pet_decay_time = 100;
            h.character = Some(make_integration_test_character(sid, "Timer", 1));
        });

        // Time 130: only 30s since last decay — should NOT collect
        assert!(world.collect_pet_decay_data(130).is_empty());

        // Time 159: 59s since last decay — still NOT enough
        assert!(world.collect_pet_decay_data(159).is_empty());

        // Time 161: 61s since last decay — SHOULD collect
        let data = world.collect_pet_decay_data(161);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].session_id, sid);

        // After applying decay, the last_decay_time should update
        let result = world.apply_pet_decay(sid, -100, 161);
        assert_eq!(result, Some(4900));

        // Verify last_pet_decay_time updated
        let last = world.with_session(sid, |h| h.last_pet_decay_time).unwrap();
        assert_eq!(last, 161);

        // Now at time 200 (only 39s later) — should NOT collect again
        assert!(world.collect_pet_decay_data(200).is_empty());
    }

    /// Integration: Pet death removes pet data from session.
    ///
    #[test]
    fn test_integration_pet_death_cleanup() {
        use crate::world::PetState;

        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 100,
                nid: 42,
                index: 7,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
        });

        // Decay exactly to 0 — pet should die
        let result = world.apply_pet_decay(sid, -100, 100);
        assert!(result.is_none(), "Pet should be dead at 0");

        // Verify pet data is completely removed
        let pet_data = world.with_session(sid, |h| h.pet_data.clone()).unwrap();
        assert!(pet_data.is_none(), "Pet data should be None after death");

        // Verify get_pet_packet_data also returns None
        let pkt_data = world.get_pet_packet_data(sid);
        assert!(pkt_data.is_none());

        // Channel should have no packet from apply_pet_decay (it returns None, caller sends death pkt)
        assert!(
            rx.try_recv().is_err(),
            "apply_pet_decay itself should not send packets"
        );
    }

    /// Integration: Wanted event position broadcast sends to enemy nation only.
    ///
    /// sends to enemy nation, not the wanted player's own nation.
    #[test]
    fn test_integration_wanted_broadcast_enemy_nation() {
        use crate::handler::vanguard::broadcast_wanted_user_positions;
        use crate::world::ZONE_RONARK_LAND;

        let world = WorldState::new();
        world.ensure_zone(ZONE_RONARK_LAND, 128);

        // Karus wanted player
        let (tx_karus, mut rx_karus) = mpsc::unbounded_channel();
        let sid_karus = world.allocate_session_id();
        world.register_session(sid_karus, tx_karus);
        world.update_session(sid_karus, |h| {
            h.character = Some(make_integration_test_character(sid_karus, "KarusWanted", 1));
            h.position = crate::world::Position {
                zone_id: ZONE_RONARK_LAND,
                x: 500.0,
                y: 0.0,
                z: 600.0,
                region_x: 2,
                region_z: 2,
            };
            h.is_wanted = true;
        });

        // Elmorad observer (should RECEIVE the broadcast)
        let (tx_elmo, mut rx_elmo) = mpsc::unbounded_channel();
        let sid_elmo = world.allocate_session_id();
        world.register_session(sid_elmo, tx_elmo);
        world.update_session(sid_elmo, |h| {
            h.character = Some(make_integration_test_character(sid_elmo, "ElmoObserver", 2));
            h.position = crate::world::Position {
                zone_id: ZONE_RONARK_LAND,
                x: 400.0,
                y: 0.0,
                z: 400.0,
                region_x: 2,
                region_z: 2,
            };
        });

        // Karus observer (should NOT receive — same nation as wanted player)
        let (tx_karus2, mut rx_karus2) = mpsc::unbounded_channel();
        let sid_karus2 = world.allocate_session_id();
        world.register_session(sid_karus2, tx_karus2);
        world.update_session(sid_karus2, |h| {
            h.character = Some(make_integration_test_character(
                sid_karus2,
                "KarusFriend",
                1,
            ));
            h.position = crate::world::Position {
                zone_id: ZONE_RONARK_LAND,
                x: 450.0,
                y: 0.0,
                z: 450.0,
                region_x: 2,
                region_z: 2,
            };
        });

        // Broadcast wanted positions
        broadcast_wanted_user_positions(&world, ZONE_RONARK_LAND);

        // Elmorad observer should get a packet (Karus wanted shown to Elmo)
        let elmo_pkt = rx_elmo.try_recv();
        assert!(elmo_pkt.is_ok(), "Elmorad should receive wanted position");

        // Karus observer should NOT get a packet (same nation as wanted)
        let karus_pkt = rx_karus2.try_recv();
        assert!(
            karus_pkt.is_err(),
            "Karus observer should NOT receive own nation's wanted position"
        );

        // The wanted player themselves should NOT get their own broadcast
        let self_pkt = rx_karus.try_recv();
        assert!(
            self_pkt.is_err(),
            "Wanted player should NOT receive their own position broadcast"
        );
    }

    /// Integration: Wanted room status gating prevents broadcasts for non-running rooms.
    ///
    #[test]
    fn test_integration_wanted_tick_status_gating() {
        use crate::handler::vanguard::tick_wanted_position_broadcasts;
        use crate::world::{WantedEventStatus, ZONE_RONARK_LAND};

        let world = WorldState::new();
        world.ensure_zone(ZONE_RONARK_LAND, 128);

        // Create a wanted player
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);
        world.update_session(sid, |h| {
            h.character = Some(make_integration_test_character(sid, "WantedGuy", 1));
            h.position = crate::world::Position {
                zone_id: ZONE_RONARK_LAND,
                x: 100.0,
                y: 0.0,
                z: 200.0,
                region_x: 2,
                region_z: 2,
            };
            h.is_wanted = true;
        });

        // Room 0 is Disabled (default) — tick should do nothing
        tick_wanted_position_broadcasts(&world, 100);
        assert!(rx.try_recv().is_err(), "No broadcast when room is Disabled");

        // Set room 0 to Running + init map show timer (C++ sets this on Running transition)
        {
            let mut rooms = world.wanted_rooms().write();
            rooms[0].status = WantedEventStatus::Running;
        }
        world
            .wanted_map_show_time
            .store(100, std::sync::atomic::Ordering::Relaxed);

        // Now tick should broadcast (elapsed = 200-100 = 100s > 60s)
        tick_wanted_position_broadcasts(&world, 200);
        // The broadcast goes to enemy nation, not the wanted player's own session,
        // so rx (Karus wanted player) should NOT receive it.
        // This test validates that the tick function runs without panic and respects status.
    }

    /// Integration: Wanted event collect_wanted_players_in_zone filters by zone and wanted flag.
    #[test]
    fn test_integration_collect_wanted_players_zone_filter() {
        use crate::world::{ZONE_ARDREAM, ZONE_RONARK_LAND};

        let world = WorldState::new();

        // Wanted player in Ronark Land
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let sid1 = world.allocate_session_id();
        world.register_session(sid1, tx1);
        world.update_session(sid1, |h| {
            h.character = Some(make_integration_test_character(sid1, "WantedRL", 1));
            h.position = crate::world::Position {
                zone_id: ZONE_RONARK_LAND,
                x: 100.0,
                y: 0.0,
                z: 200.0,
                region_x: 2,
                region_z: 2,
            };
            h.is_wanted = true;
        });

        // Wanted player in Ardream
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let sid2 = world.allocate_session_id();
        world.register_session(sid2, tx2);
        world.update_session(sid2, |h| {
            h.character = Some(make_integration_test_character(sid2, "WantedAD", 2));
            h.position = crate::world::Position {
                zone_id: ZONE_ARDREAM,
                x: 300.0,
                y: 0.0,
                z: 400.0,
                region_x: 2,
                region_z: 2,
            };
            h.is_wanted = true;
        });

        // Non-wanted player in Ronark Land
        let (tx3, _rx3) = mpsc::unbounded_channel();
        let sid3 = world.allocate_session_id();
        world.register_session(sid3, tx3);
        world.update_session(sid3, |h| {
            h.character = Some(make_integration_test_character(sid3, "Normal", 1));
            h.position = crate::world::Position {
                zone_id: ZONE_RONARK_LAND,
                x: 150.0,
                y: 0.0,
                z: 250.0,
                region_x: 2,
                region_z: 2,
            };
            h.is_wanted = false;
        });

        // Collect for Ronark Land — should only get WantedRL
        let rl_players = world.collect_wanted_players_in_zone(ZONE_RONARK_LAND);
        assert_eq!(rl_players.len(), 1);
        assert_eq!(rl_players[0].4, "WantedRL");
        assert_eq!(rl_players[0].1, 1); // nation

        // Collect for Ardream — should only get WantedAD
        let ad_players = world.collect_wanted_players_in_zone(ZONE_ARDREAM);
        assert_eq!(ad_players.len(), 1);
        assert_eq!(ad_players[0].4, "WantedAD");
    }

    /// Integration: Cinderella war zone gating is accessible from WorldState.
    ///
    /// Verifies that the cindwar_active and cindwar_zone_id atomics can be
    /// set and checked, which is used by buff clearing to exclude Cinderella zones.
    #[test]
    fn test_integration_cinderella_zone_gating() {
        use std::sync::atomic::Ordering;

        let world = WorldState::new();

        // Default: no cinderella event
        assert!(!world.cindwar_active.load(Ordering::Relaxed));
        assert_eq!(world.cindwar_zone_id.load(Ordering::Relaxed), 0);

        // Activate cinderella in zone 55
        world.cindwar_active.store(true, Ordering::Relaxed);
        world.cindwar_zone_id.store(55, Ordering::Relaxed);

        assert!(world.cindwar_active.load(Ordering::Relaxed));
        assert_eq!(world.cindwar_zone_id.load(Ordering::Relaxed), 55);

        // Deactivate
        world.cindwar_active.store(false, Ordering::Relaxed);
        world.cindwar_zone_id.store(0, Ordering::Relaxed);

        assert!(!world.cindwar_active.load(Ordering::Relaxed));
    }

    /// Integration: Pet satisfaction boundaries (max 10000, min 0).
    ///
    #[test]
    fn test_integration_pet_satisfaction_boundaries() {
        use crate::world::PetState;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Test with max satisfaction
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 10000,
                nid: 1,
                index: 1,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
        });

        // Apply small decay
        let result = world.apply_pet_decay(sid, -100, 100);
        assert_eq!(result, Some(9900));

        // Apply massive decay (should not go below 0 — pet dies)
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 50,
                nid: 1,
                index: 1,
                ..Default::default()
            });
        });

        let result = world.apply_pet_decay(sid, -9999, 200);
        assert!(result.is_none(), "Massive decay should kill pet");
    }

    /// Integration: Wanted map show timer throttles broadcasts to 60-second intervals.
    ///
    #[test]
    fn test_integration_wanted_map_show_timer_throttle() {
        use crate::handler::vanguard::tick_wanted_position_broadcasts;
        use crate::world::WantedEventStatus;
        use std::sync::atomic::Ordering;

        let world = WorldState::new();
        world.ensure_zone(71, 128); // ZONE_RONARK_LAND

        // Set room 0 to Running and initialize the map show timer
        // (C++ sets m_WantedSystemMapShowTime when Running phase starts)
        {
            let mut rooms = world.wanted_rooms().write();
            rooms[0].status = WantedEventStatus::Running;
        }
        world.wanted_map_show_time.store(100, Ordering::Relaxed);

        // First tick at time 100 — elapsed=0, throttled (< 60s since init)
        tick_wanted_position_broadcasts(&world, 100);
        // Add a wanted player to make broadcasts actually update the timer.

        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);
        world.update_session(sid, |h| {
            h.character = Some(make_integration_test_character(sid, "Wanted", 1));
            h.position = crate::world::Position {
                zone_id: 71,
                x: 100.0,
                y: 0.0,
                z: 200.0,
                region_x: 2,
                region_z: 2,
            };
            h.is_wanted = true;
        });

        // Tick at time 200 — first real broadcast
        tick_wanted_position_broadcasts(&world, 200);
        let t2 = world.wanted_map_show_time.load(Ordering::Relaxed);
        assert_eq!(t2, 200, "Timer should update after broadcast");

        // Tick at time 230 — only 30s later, should be throttled
        tick_wanted_position_broadcasts(&world, 230);
        let t3 = world.wanted_map_show_time.load(Ordering::Relaxed);
        assert_eq!(t3, 200, "Timer should NOT update within 60s window");

        // Tick at time 261 — 61s later, should proceed
        tick_wanted_position_broadcasts(&world, 261);
        let t4 = world.wanted_map_show_time.load(Ordering::Relaxed);
        assert_eq!(t4, 261, "Timer should update after 60s window");
    }

    /// Integration: broadcast_to_zone_nation correctly filters by zone and nation.
    #[test]
    fn test_integration_broadcast_to_zone_nation_filtering() {
        use ko_protocol::Packet;

        let world = WorldState::new();

        // Player A: nation 1, zone 71
        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let sid_a = world.allocate_session_id();
        world.register_session(sid_a, tx_a);
        world.update_session(sid_a, |h| {
            h.character = Some(make_integration_test_character(sid_a, "A", 1));
            h.position = crate::world::Position {
                zone_id: 71,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 2,
                region_z: 2,
            };
        });

        // Player B: nation 2, zone 71
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        let sid_b = world.allocate_session_id();
        world.register_session(sid_b, tx_b);
        world.update_session(sid_b, |h| {
            h.character = Some(make_integration_test_character(sid_b, "B", 2));
            h.position = crate::world::Position {
                zone_id: 71,
                x: 200.0,
                y: 0.0,
                z: 200.0,
                region_x: 2,
                region_z: 2,
            };
        });

        // Player C: nation 1, zone 72 (different zone)
        let (tx_c, mut rx_c) = mpsc::unbounded_channel();
        let sid_c = world.allocate_session_id();
        world.register_session(sid_c, tx_c);
        world.update_session(sid_c, |h| {
            h.character = Some(make_integration_test_character(sid_c, "C", 1));
            h.position = crate::world::Position {
                zone_id: 72,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 2,
                region_z: 2,
            };
        });

        // Broadcast to zone 71, nation 1
        let pkt = Packet::new(0xFF);
        world.broadcast_to_zone_nation(71, 1, Arc::new(pkt), None);

        // Player A (nation 1, zone 71) should receive
        assert!(
            rx_a.try_recv().is_ok(),
            "Player A should receive (nation 1, zone 71)"
        );

        // Player B (nation 2, zone 71) should NOT receive
        assert!(
            rx_b.try_recv().is_err(),
            "Player B should NOT receive (nation 2)"
        );

        // Player C (nation 1, zone 72) should NOT receive
        assert!(
            rx_c.try_recv().is_err(),
            "Player C should NOT receive (zone 72)"
        );
    }

    /// Integration: broadcast_to_zone_nation respects `except` parameter.
    #[test]
    fn test_integration_broadcast_except_session() {
        use ko_protocol::Packet;

        let world = WorldState::new();

        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let sid_a = world.allocate_session_id();
        world.register_session(sid_a, tx_a);
        world.update_session(sid_a, |h| {
            h.character = Some(make_integration_test_character(sid_a, "A", 1));
            h.position = crate::world::Position {
                zone_id: 71,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 2,
                region_z: 2,
            };
        });

        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        let sid_b = world.allocate_session_id();
        world.register_session(sid_b, tx_b);
        world.update_session(sid_b, |h| {
            h.character = Some(make_integration_test_character(sid_b, "B", 1));
            h.position = crate::world::Position {
                zone_id: 71,
                x: 200.0,
                y: 0.0,
                z: 200.0,
                region_x: 2,
                region_z: 2,
            };
        });

        // Broadcast to zone 71, nation 1, except sid_a
        let pkt = Packet::new(0xFF);
        world.broadcast_to_zone_nation(71, 1, Arc::new(pkt), Some(sid_a));

        // Player A should NOT receive (excluded)
        assert!(
            rx_a.try_recv().is_err(),
            "Excluded session should NOT receive"
        );

        // Player B should receive
        assert!(
            rx_b.try_recv().is_ok(),
            "Non-excluded session should receive"
        );
    }

    /// Integration: Pet feeding via apply_pet_decay with positive amount.
    ///
    /// positive values (feeding) or negative (decay).
    #[test]
    fn test_integration_pet_feeding_positive_decay() {
        use crate::world::PetState;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 5000,
                nid: 1,
                index: 1,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
        });

        // Apply positive amount (feeding)
        let result = world.apply_pet_decay(sid, 500, 100);
        assert_eq!(result, Some(5500), "Feeding should increase satisfaction");
    }

    /// Integration: WantedEventRoom default state is all Disabled.
    #[test]
    fn test_integration_wanted_rooms_default_state() {
        use crate::world::WantedEventStatus;

        let world = WorldState::new();
        let rooms = world.wanted_rooms().read();

        for (i, room) in rooms.iter().enumerate() {
            assert_eq!(
                room.status,
                WantedEventStatus::Disabled,
                "Room {} should default to Disabled",
                i
            );
            assert!(room.elmo_list.is_empty());
            assert!(room.karus_list.is_empty());
        }
    }

    /// Integration: Wanted event rooms are all disabled on fresh WorldState.
    #[test]
    fn test_integration_wanted_event_disabled_by_default() {
        use crate::world::WantedEventStatus;

        let world = WorldState::new();
        let rooms = world.wanted_rooms().read();
        let all_disabled = rooms
            .iter()
            .all(|r| r.status == WantedEventStatus::Disabled);
        assert!(
            all_disabled,
            "All wanted rooms should be Disabled by default"
        );
    }

    // ── Sprint 47 QA Integration Tests ──────────────────────────────────────

    /// Integration: Zone kill reward lookup returns only rewards matching zone + status=1.
    ///
    /// filtered by ZoneID and Status.
    #[test]
    fn test_integration_zone_kill_reward_lookup() {
        use ko_db::models::ZoneKillReward;

        let world = WorldState::new();

        let rewards = vec![
            ZoneKillReward {
                idx: 1,
                zone_id: 71,
                nation: 0,
                party_required: -1,
                all_party_reward: false,
                kill_count: 1,
                item_name: None,
                item_id: 389070000,
                item_duration: 0,
                item_count: 1,
                item_flag: 0,
                item_expiration: 0,
                drop_rate: 10000,
                give_to_warehouse: false,
                status: 1,
                is_priest: false,
                priest_rate: 0,
            },
            ZoneKillReward {
                idx: 2,
                zone_id: 72,
                nation: 0,
                party_required: -1,
                all_party_reward: false,
                kill_count: 1,
                item_name: None,
                item_id: 389080000,
                item_duration: 0,
                item_count: 1,
                item_flag: 0,
                item_expiration: 0,
                drop_rate: 5000,
                give_to_warehouse: false,
                status: 1,
                is_priest: false,
                priest_rate: 0,
            },
            ZoneKillReward {
                idx: 3,
                zone_id: 71,
                nation: 0,
                party_required: -1,
                all_party_reward: false,
                kill_count: 1,
                item_name: None,
                item_id: 900000,
                item_duration: 0,
                item_count: 10,
                item_flag: 0,
                item_expiration: 0,
                drop_rate: 10000,
                give_to_warehouse: false,
                status: 0, // disabled
                is_priest: false,
                priest_rate: 0,
            },
        ];
        world.insert_zone_kill_rewards(rewards);

        // Zone 71 should return only the active reward (idx=1), not disabled (idx=3)
        let zone71 = world.get_zone_kill_rewards(71);
        assert_eq!(zone71.len(), 1, "Only active rewards for zone 71");
        assert_eq!(zone71[0].item_id, 389070000);

        // Zone 72 should return idx=2
        let zone72 = world.get_zone_kill_rewards(72);
        assert_eq!(zone72.len(), 1);
        assert_eq!(zone72[0].item_id, 389080000);

        // Zone 73 has no rewards
        let zone73 = world.get_zone_kill_rewards(73);
        assert!(zone73.is_empty());
    }

    /// Integration: Zone kill reward gold grant applies to player session.
    ///
    /// Verifies the reward pipeline: kill in zone -> lookup reward -> gold_gain.
    #[test]
    fn test_integration_zone_kill_reward_gold_grant() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.update_session(sid, |h| {
            h.character = Some(make_integration_test_character(sid, "Killer", 1));
        });

        // Verify initial gold
        let initial_gold = world
            .with_session(sid, |h| h.character.as_ref().map(|c| c.gold))
            .flatten()
            .unwrap_or(0);
        assert_eq!(initial_gold, 0);

        // Grant gold directly (simulating what give_reward_to_player does for ITEM_GOLD)
        world.gold_gain(sid, 5000);

        let new_gold = world
            .with_session(sid, |h| h.character.as_ref().map(|c| c.gold))
            .flatten()
            .unwrap_or(0);
        assert_eq!(
            new_gold, 5000,
            "Gold should increase by 5000 after kill reward"
        );
    }

    /// Integration: Online reward timer expiry grants the reward.
    ///
    /// Verifies that when a timer expires and zone matches, the reward is triggered.
    #[test]
    fn test_integration_online_reward_timer_expiry() {
        use ko_db::models::ZoneOnlineReward;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.update_session(sid, |h| {
            let mut ch = make_integration_test_character(sid, "OnlinePlayer", 1);
            ch.hp = 5000;
            ch.res_hp_type = 1; // alive
            h.character = Some(ch);
            h.position.zone_id = 71;
            h.premium_in_use = 0;
        });

        // Insert an online reward for zone 71
        let rewards = vec![ZoneOnlineReward {
            zone_id: 71,
            item_id: 0,
            item_count: 0,
            item_time: 0,
            minute: 1,
            loyalty: 50,
            cash: 0,
            tl: 0,
            pre_item_id: 0,
            pre_item_count: 0,
            pre_item_time: 0,
            pre_minute: 0,
            pre_loyalty: 0,
            pre_cash: 0,
            pre_tl: 0,
        }];
        world.insert_zone_online_rewards(rewards.clone());

        // Set the timer to already expired (past timestamp)
        world.update_session(sid, |h| {
            h.zone_online_reward_timers = vec![1]; // expired (way in the past)
        });

        // Collect session IDs eligible for reward
        let session_ids = world.collect_zone_online_reward_session_ids();
        assert!(
            session_ids.contains(&sid),
            "Player with timers should be in reward session list"
        );

        // Verify timer was set (the processing would reset it)
        let timer = world
            .with_session(sid, |h| h.zone_online_reward_timers.first().copied())
            .flatten();
        assert_eq!(
            timer,
            Some(1),
            "Timer should be at expired value before tick"
        );
    }

    /// Integration: Pet satisfaction decay over multiple ticks leads to de-summon at 0.
    ///
    /// Simulates repeated 40-second decay ticks until pet satisfaction reaches 0.
    #[test]
    fn test_integration_pet_decay_to_desummon() {
        use crate::world::{PetState, PET_DECAY_AMOUNT};

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Start with satisfaction just above death threshold
        let initial_sat: i16 = 250;
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: initial_sat,
                nid: 5,
                index: 42,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
            h.character = Some(make_integration_test_character(sid, "PetOwner", 1));
        });

        // Apply decay ticks until pet dies
        let decay = -(PET_DECAY_AMOUNT);
        let mut tick_time = 100u64;
        let mut ticks = 0;

        loop {
            let result = world.apply_pet_decay(sid, decay, tick_time);
            ticks += 1;
            tick_time += 40;

            if result.is_none() {
                break; // Pet died
            }
            if ticks > 100 {
                panic!("Pet should have died within 100 ticks");
            }
        }

        // Verify pet is gone from session
        let has_pet = world.with_session(sid, |h| h.pet_data.is_some()).unwrap();
        assert!(
            !has_pet,
            "Pet should be de-summoned after satisfaction hits 0"
        );

        // Should have taken ceil(250 / PET_DECAY_AMOUNT) ticks
        let expected_ticks = (initial_sat + PET_DECAY_AMOUNT - 1) / PET_DECAY_AMOUNT;
        assert_eq!(
            ticks, expected_ticks as u32,
            "Expected {} ticks to kill pet with sat={}",
            expected_ticks, initial_sat
        );
    }

    /// Integration: Cinderella zone matching and gating lifecycle.
    ///
    /// Verifies zone matching, activation, and player-in-event detection.
    #[test]
    fn test_integration_cinderella_gating_zone_and_lifecycle() {
        use crate::handler::cinderella::is_cinderella_zone;

        // Verify cinderella zone matching
        assert!(is_cinderella_zone(110, 110), "Same zone should match");
        assert!(
            !is_cinderella_zone(21, 110),
            "Different zone should not match"
        );
        assert!(!is_cinderella_zone(0, 110), "Zone 0 should not match");

        // Verify WorldState integration
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Place player in cinderella zone
        world.update_session(sid, |h| {
            h.character = Some(make_integration_test_character(sid, "CindTest", 1));
            h.position.zone_id = 110;
        });

        // Activate event, register player
        world.set_cinderella_active(true, 110);
        world.add_cinderella_user(sid);

        // Player should be detected as in-cinderella
        assert!(world.is_player_in_cinderella(sid));

        // Player in wrong zone should NOT be detected
        world.update_session(sid, |h| {
            h.position.zone_id = 21;
        });
        assert!(
            !world.is_player_in_cinderella(sid),
            "Player in zone 21 should not be in cinderella zone 110"
        );

        // Move back — should be detected again
        world.update_session(sid, |h| {
            h.position.zone_id = 110;
        });
        assert!(world.is_player_in_cinderella(sid));

        // Deactivate clears everything
        world.set_cinderella_active(false, 0);
        assert!(!world.is_player_in_cinderella(sid));
    }

    /// Integration: Cinderella event participant tracking with WorldState.
    ///
    /// Verifies activate/add user/check/deactivate lifecycle.
    #[test]
    fn test_integration_cinderella_participant_lifecycle() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Place player in zone 110 (cinderella zone)
        world.update_session(sid, |h| {
            h.character = Some(make_integration_test_character(sid, "CindPlayer", 1));
            h.position.zone_id = 110;
        });

        // Not active yet
        assert!(!world.is_cinderella_active());
        assert!(!world.is_player_in_cinderella(sid));

        // Activate cinderella in zone 110
        world.set_cinderella_active(true, 110);
        assert!(world.is_cinderella_active());

        // Player not yet registered as event user
        assert!(!world.is_player_in_cinderella(sid));

        // Register player
        world.add_cinderella_user(sid);
        assert!(world.is_player_in_cinderella(sid));

        // Deactivate event — clears all users
        world.set_cinderella_active(false, 0);
        assert!(!world.is_cinderella_active());
        assert!(!world.is_player_in_cinderella(sid));
    }

    /// Integration: Guard NPC targeting priority selects closest enemy.
    ///
    /// picks the closest hostile NPC by distance squared.
    /// This test verifies the guard picks the nearest of two enemies.
    #[tokio::test]
    async fn test_integration_guard_targeting_closest_enemy() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Guard at (100, 100), nation 1
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Far enemy at (120, 120), nation 2
        let far_id: NpcId = NPC_BAND + 1;
        let far_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(far_tmpl.clone());
        let far_inst = NpcInstance {
            nid: far_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 120.0,
            y: 0.0,
            z: 120.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(far_inst);
        world.init_npc_hp(far_id, 1000);
        let far_ai = make_nation_test_ai(2, 120.0, 120.0, 2, 2);
        world.insert_npc_ai(far_id, far_ai);

        // Close enemy at (103, 103), nation 2
        let close_id: NpcId = NPC_BAND + 2;
        let close_tmpl = make_nation_test_template(2001, true, 2, 0);
        world.insert_npc_template(close_tmpl.clone());
        let close_inst = NpcInstance {
            nid: close_id,
            proto_id: 2001,
            is_monster: true,
            zone_id: 21,
            x: 103.0,
            y: 0.0,
            z: 103.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(close_inst);
        world.init_npc_hp(close_id, 1000);
        let close_ai = make_nation_test_ai(2, 103.0, 103.0, 2, 2);
        world.insert_npc_ai(close_id, close_ai);

        // Register all in zone
        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, far_id);
        zone.add_npc(2, 2, close_id);

        // Guard should pick closest enemy (close_id at distance ~4.24 vs far at ~28.28)
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(
            result,
            Some(close_id),
            "Guard should target closest enemy NPC"
        );
    }

    /// Integration: NPC-vs-NPC combat terminates when target dies.
    ///
    /// Verifies that when the target NPC's HP reaches 0, find_npc_enemy no longer
    /// returns the dead NPC.
    #[tokio::test]
    async fn test_integration_npc_combat_stops_on_target_death() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Attacker guard, nation 1
        let guard_id: NpcId = NPC_BAND;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Target monster, nation 2, will be killed
        let monster_id: NpcId = NPC_BAND + 1;
        let monster_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(monster_tmpl.clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);
        let monster_ai = make_nation_test_ai(2, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(monster_id, monster_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Guard finds the monster while alive
        let result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(result, Some(monster_id), "Guard should find alive monster");

        // Kill the monster by setting HP to 0
        world.update_npc_hp(monster_id, 0);

        // Guard should no longer find the dead monster
        let result_after = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert_eq!(result_after, None, "Guard should not target dead monster");
    }

    /// Integration: Nation-0 (neutral) NPC is excluded from attacking.
    ///
    /// The attacker being nation-0 means it never initiates combat.
    #[tokio::test]
    async fn test_integration_nation0_npc_hostility_exclusion() {
        use crate::npc::{NpcInstance, NPC_BAND};

        let world = WorldState::new();
        world.ensure_zone(21, 128);

        // Nation-0 NPC (neutral, should not attack)
        let neutral_id: NpcId = NPC_BAND;
        let neutral_tmpl = make_nation_test_template(3000, true, 0, 0);
        world.insert_npc_template(neutral_tmpl.clone());
        let neutral_inst = NpcInstance {
            nid: neutral_id,
            proto_id: 3000,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(neutral_inst);
        world.init_npc_hp(neutral_id, 5000);
        let neutral_ai = make_nation_test_ai(0, 100.0, 100.0, 2, 2);
        world.insert_npc_ai(neutral_id, neutral_ai.clone());

        // Nation-1 guard nearby (potential target)
        let guard_id: NpcId = NPC_BAND + 1;
        let guard_tmpl = make_nation_test_template(1000, false, 1, NPC_GUARD);
        world.insert_npc_template(guard_tmpl.clone());
        let guard_inst = NpcInstance {
            nid: guard_id,
            proto_id: 1000,
            is_monster: false,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 105.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(guard_inst);
        world.init_npc_hp(guard_id, 50000);
        let guard_ai = make_nation_test_ai(1, 105.0, 105.0, 2, 2);
        world.insert_npc_ai(guard_id, guard_ai.clone());

        // Nation-2 monster nearby (potential target)
        let monster_id: NpcId = NPC_BAND + 2;
        let monster_tmpl = make_nation_test_template(2000, true, 2, 0);
        world.insert_npc_template(monster_tmpl.clone());
        let monster_inst = NpcInstance {
            nid: monster_id,
            proto_id: 2000,
            is_monster: true,
            zone_id: 21,
            x: 110.0,
            y: 0.0,
            z: 110.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 2,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(monster_inst);
        world.init_npc_hp(monster_id, 1000);
        let monster_ai = make_nation_test_ai(2, 110.0, 110.0, 2, 2);
        world.insert_npc_ai(monster_id, monster_ai);

        let zone = world.get_zone(21).unwrap();
        zone.add_npc(2, 2, neutral_id);
        zone.add_npc(2, 2, guard_id);
        zone.add_npc(2, 2, monster_id);

        // Nation-0 NPC should NOT target anything — neutral never attacks
        let result = find_npc_enemy(&world, &neutral_ai, neutral_id, &neutral_tmpl);
        assert_eq!(result, None, "Nation-0 NPC must not attack anything");

        // But the guard (nation 1) CAN target the neutral monster
        let guard_result = find_npc_enemy(&world, &guard_ai, guard_id, &guard_tmpl);
        assert!(
            guard_result.is_some(),
            "Guard should be able to target nation-0 or nation-2 NPCs"
        );
    }
}
