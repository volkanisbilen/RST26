//! Chaos Dungeon arena objects: cubes (skill pickups) and patrolling saw blades.
//!
//! Reference (C++ `NPCHandler.cpp`, `CUser::ClientEvent`):
//! * `CHAOS_CUBE_SSID` (31527): the first player to click a live cube gets ONE random skill item
//!   (1/4 Killing Blade, 1/4 Light Pit, 2/4 Drain Restore), the cube plays effect 251 for the room
//!   and is killed.
//! * `SAW_BLADE_SSID` (32153): touching a saw in the Chaos Dungeon costs `5000 / 10` = 500 HP.
//! * Skill items are removed again by `RobChaosSkillItems` on death, zone exit and login
//!   (`handler::dead::rob_chaos_skill_items`); they never transfer to the killer.
//!
//! The saw blades are non-monster NPC spawns (`act_type` 101) whose `npc_spawn.path` column holds
//! the patrol route as consecutive `xxxxzzzz` 4-digit pairs. Nothing in the generic NPC AI walks
//! such a path, so a small driver (`start_saw_task`) does it.

use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use ko_protocol::{Opcode, Packet};

use crate::handler::dead::{ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE, ITEM_LIGHT_PIT};
use crate::npc::NpcId;
use crate::world::{WorldState, ZONE_CHAOS_DUNGEON};
use crate::zone::{calc_region, SessionId};

/// Proto id of the Chaos cube NPC (`CHAOS_CUBE_SSID`).
pub const CHAOS_CUBE_PROTO: u16 = 31527;
/// Proto id of the saw blade NPC (`SAW_BLADE_SSID`).
pub const SAW_BLADE_PROTO: u16 = 32153;
/// HP a saw touch costs inside the Chaos Dungeon (`HpChange(-5000 / 10)`).
pub const SAW_DAMAGE: i16 = 500;
/// Visual effect shown when a cube is opened (`ShowNpcEffect(.., 251, ..)`).
pub const CUBE_EFFECT_ID: u32 = 251;
/// Speed (units per second) used when the template has none.
const DEFAULT_SAW_SPEED: f32 = 15.0;
/// Driver period.
const SAW_TICK_MS: u64 = 250;
/// A player cannot be hurt by saws more often than this (the client re-sends the touch).
const SAW_HIT_COOLDOWN_MS: u64 = 700;

/// Item granted for a d4 roll (`rand() % 4 + 1`): 1 = Killing Blade, 2 = Light Pit, 3 and 4 =
/// Drain Restore. Any other value gives nothing (`default: break`).
pub fn cube_item_for_roll(roll: u32) -> u32 {
    match roll {
        1 => ITEM_KILLING_BLADE,
        2 => ITEM_LIGHT_PIT,
        3 | 4 => ITEM_DRAIN_RESTORE,
        _ => 0,
    }
}

/// Parse an `npc_spawn.path` string (`xxxxzzzz` repeated, decimal) into waypoints.
pub fn parse_path(path: &str) -> Vec<(f32, f32)> {
    let digits: Vec<u8> = path.bytes().filter(|b| b.is_ascii_digit()).collect();
    digits
        .chunks_exact(8)
        .map(|c| {
            let num = |s: &[u8]| s.iter().fold(0u32, |a, d| a * 10 + (*d - b'0') as u32) as f32;
            (num(&c[0..4]), num(&c[4..8]))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct SawPatrol {
    waypoints: Vec<(f32, f32)>,
    /// Waypoint the saw is currently travelling to (None before the first leg).
    heading_to: Option<usize>,
    arrive_at_ms: u64,
}

fn patrols() -> &'static DashMap<NpcId, SawPatrol> {
    static MAP: OnceLock<DashMap<NpcId, SawPatrol>> = OnceLock::new();
    MAP.get_or_init(DashMap::new)
}

fn hit_log() -> &'static DashMap<SessionId, u64> {
    static MAP: OnceLock<DashMap<SessionId, u64>> = OnceLock::new();
    MAP.get_or_init(DashMap::new)
}

/// Remember the patrol route of a freshly spawned saw (called by the NPC spawn loader).
pub fn register_saw(nid: NpcId, path: &str) {
    let waypoints = parse_path(path);
    if waypoints.len() < 2 {
        return;
    }
    patrols().insert(
        nid,
        SawPatrol {
            waypoints,
            heading_to: None,
            arrive_at_ms: 0,
        },
    );
}

/// Forget every saw route (NPC tables are being reloaded).
pub fn clear_saws() {
    patrols().clear();
}

/// Number of registered saws (diagnostics / tests).
pub fn saw_count() -> usize {
    patrols().len()
}

/// Next leg of a patrol: index of the waypoint to head for after `current`.
fn next_leg(current: Option<usize>, len: usize) -> usize {
    match current {
        Some(i) => (i + 1) % len,
        None => 0,
    }
}

/// Milliseconds needed to cover `dist` units at `speed` units per second (at least one tick).
fn travel_ms(dist: f32, speed: f32) -> u64 {
    let speed = if speed > 0.0 { speed } else { DEFAULT_SAW_SPEED };
    (((dist / speed) * 1000.0) as u64).max(SAW_TICK_MS)
}

fn send_npc_move(world: &WorldState, nid: NpcId, x: f32, z: f32, speed: f32) {
    let Some(npc) = world.get_npc_instance(nid) else {
        return;
    };
    let mut pkt = Packet::new(Opcode::WizNpcMove as u8);
    pkt.write_u8(1);
    pkt.write_u32(nid);
    pkt.write_u16((x * 10.0) as u16);
    pkt.write_u16((z * 10.0) as u16);
    pkt.write_u16(0);
    pkt.write_u16((speed * 10.0) as u16);
    world.broadcast_to_3x3(
        npc.zone_id,
        npc.region_x,
        npc.region_z,
        Arc::new(pkt),
        None,
        npc.event_room,
    );
}

/// One driver pass: advance every saw whose current leg is finished.
pub fn tick(world: &WorldState, now_ms: u64) {
    let chaos_running = world
        .event_room_manager()
        .read_temple_event(|s| s.is_chaos_active());
    if !chaos_running {
        return;
    }
    let ids: Vec<NpcId> = patrols().iter().map(|e| *e.key()).collect();
    for nid in ids {
        let Some(npc) = world.get_npc_instance(nid) else {
            patrols().remove(&nid);
            continue;
        };
        let speed = world
            .get_npc_template(npc.proto_id, npc.is_monster)
            .map(|t| t.speed_1 as f32)
            .filter(|s| *s > 0.0)
            .unwrap_or(DEFAULT_SAW_SPEED);
        let Some(mut patrol) = patrols().get_mut(&nid) else {
            continue;
        };
        if now_ms < patrol.arrive_at_ms {
            continue;
        }
        // arrived at the waypoint of the previous leg: commit the position
        if let Some(i) = patrol.heading_to {
            let (x, z) = patrol.waypoints[i];
            world.move_runtime_npc(nid, x, z);
        }
        let next = next_leg(patrol.heading_to, patrol.waypoints.len());
        let (tx, tz) = patrol.waypoints[next];
        let (cx, cz) = world
            .get_npc_instance(nid)
            .map(|n| (n.x, n.z))
            .unwrap_or((npc.x, npc.z));
        let dist = ((tx - cx).powi(2) + (tz - cz).powi(2)).sqrt();
        patrol.heading_to = Some(next);
        patrol.arrive_at_ms = now_ms + travel_ms(dist, speed);
        drop(patrol);
        send_npc_move(world, nid, tx, tz, speed);
    }
}

/// Start the saw driver (idle while no saw is registered).
pub fn start_saw_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(SAW_TICK_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let started = std::time::Instant::now();
        loop {
            interval.tick().await;
            if patrols().is_empty() {
                continue;
            }
            let now_ms = started.elapsed().as_millis() as u64;
            let w = Arc::clone(&world);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tick(&w, now_ms)));
        }
    })
}

/// A player touched a saw blade (ClientEvent on proto 32153). Returns `true` when the NPC was a
/// saw (the click is consumed either way).
pub fn handle_saw_touch(world: &WorldState, sid: SessionId, now_ms: u64) -> bool {
    let in_chaos = world
        .get_position(sid)
        .is_some_and(|p| p.zone_id == ZONE_CHAOS_DUNGEON);
    if !in_chaos || world.is_player_dead(sid) {
        return true;
    }
    if let Some(last) = hit_log().get(&sid).map(|v| *v) {
        if now_ms.saturating_sub(last) < SAW_HIT_COOLDOWN_MS {
            return true;
        }
    }
    hit_log().insert(sid, now_ms);
    let Some(ch) = world.get_character_info(sid) else {
        return true;
    };
    let new_hp = (ch.hp - SAW_DAMAGE).max(0);
    world.update_character_stats(sid, |c| c.hp = new_hp);
    world.send_to_session_owned(
        sid,
        crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp),
    );
    if new_hp <= 0 {
        crate::handler::dead::broadcast_death(world, sid);
    }
    true
}

/// A player clicked a Chaos cube. Gives the rolled skill item and removes the cube for the room.
/// Returns `true` when something was granted.
pub fn handle_cube_click(world: &WorldState, sid: SessionId, npc_nid: NpcId, roll: u32) -> bool {
    let Some(npc) = world.get_npc_instance(npc_nid) else {
        return false;
    };
    if npc.proto_id != CHAOS_CUBE_PROTO || world.is_npc_dead(npc_nid) {
        return false;
    }
    let item = cube_item_for_roll(roll);
    if item == 0 {
        return false;
    }
    if !world.give_item(sid, item, 1) {
        // bag full: leave the cube where it is, the player can try again
        return false;
    }
    let mut effect = Packet::new(Opcode::WizObjectEvent as u8);
    effect.write_u8(crate::object_event_constants::OBJECT_NPC);
    effect.write_u8(3);
    effect.write_u32(npc_nid);
    effect.write_u32(CUBE_EFFECT_ID);
    world.broadcast_to_zone_event_room(npc.zone_id, npc.event_room, Arc::new(effect), None);
    world.kill_npc(npc_nid);
    true
}

/// The three Chaos skill items.
pub const SKILL_ITEMS: [u32; 3] = [ITEM_LIGHT_PIT, ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE];

/// `true` for the skill items that must never leave the event (vault, trade, mail).
pub fn is_chaos_skill_item(item_id: u32) -> bool {
    SKILL_ITEMS.contains(&item_id)
}

/// Remove the skill items from the player's vault as well (they must not survive the event there).
pub fn purge_warehouse(world: &WorldState, sid: SessionId) -> usize {
    let mut removed = 0usize;
    world.update_warehouse(sid, |wh, _coins| {
        for slot in wh.iter_mut() {
            if is_chaos_skill_item(slot.item_id) {
                *slot = Default::default();
                removed += 1;
            }
        }
        removed > 0
    });
    removed
}

// keep calc_region referenced for route sanity checks in tests
#[allow(dead_code)]
fn region_of(x: f32) -> u16 {
    calc_region(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_roll_table_matches_reference() {
        assert_eq!(cube_item_for_roll(1), ITEM_KILLING_BLADE);
        assert_eq!(cube_item_for_roll(2), ITEM_LIGHT_PIT);
        assert_eq!(cube_item_for_roll(3), ITEM_DRAIN_RESTORE);
        assert_eq!(cube_item_for_roll(4), ITEM_DRAIN_RESTORE);
        assert_eq!(cube_item_for_roll(0), 0);
        assert_eq!(cube_item_for_roll(5), 0);
    }

    #[test]
    fn path_parses_four_digit_pairs() {
        let p = parse_path("01130138012601490113016401190164");
        assert_eq!(p, vec![(113.0, 138.0), (126.0, 149.0), (113.0, 164.0), (119.0, 164.0)]);
        assert!(parse_path("").is_empty());
        // a dangling half pair is ignored
        assert_eq!(parse_path("011301380126").len(), 1);
    }

    #[test]
    fn long_saw_route_parses() {
        let a = "01130138012601490113016401190164013301640141016401260149013801380138012101380121";
        assert_eq!(parse_path(a).len(), 10);
    }

    #[test]
    fn patrol_is_a_loop() {
        assert_eq!(next_leg(None, 4), 0);
        assert_eq!(next_leg(Some(0), 4), 1);
        assert_eq!(next_leg(Some(3), 4), 0);
    }

    #[test]
    fn travel_time_scales_with_distance() {
        assert_eq!(travel_ms(30.0, 15.0), 2000);
        assert_eq!(travel_ms(0.0, 15.0), SAW_TICK_MS);
        assert_eq!(travel_ms(15.0, 0.0), 1000);
    }

    #[test]
    fn skill_items_are_recognised() {
        assert!(is_chaos_skill_item(700037000));
        assert!(is_chaos_skill_item(700040000));
        assert!(is_chaos_skill_item(700041000));
        assert!(!is_chaos_skill_item(1));
    }

    #[test]
    fn hp_is_fixed_in_chaos_only() {
        assert_eq!(crate::systems::chaos::max_hp_override_for_zone(85), Some(1000));
        assert_eq!(crate::systems::chaos::max_hp_override_for_zone(21), None);
    }
}
