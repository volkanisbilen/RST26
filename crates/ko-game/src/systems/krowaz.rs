//! Krowaz Dominion (zone 75) hazards, as in the original server:
//!
//! * **Saw blades** (`SAW_BLADE_SSID` 32153, npc_spawn `act_type` 101 = "move type 1": random walk around the spawn
//!   point inside `spawn_range`): they wander; touching one costs 370 HP (`CUser::ClientEvent`: `HpChange(-370)`).
//! * **Rolling stones** (`NPC_ROLLINGSTONE` 181, proto 32152, `act_type` 105 = path): a stone appears at its spawn, rolls
//!   along the route stored in `npc_spawn.path` (`xxxxzzzz` pairs), disappears at the end and comes back `regen_time` later.
//!   Touching one kills at once (`HpChange(-GetMaxHealth())`).
//! * **Poison gas** (object events of type 13, `EventTrapProcess`): standing inside the 14 x 6 box around the object costs
//!   1500 HP on every movement packet. GMs are immune.
//! A death by saw / stone / gas costs no exp (`OnDeathKilledNpc`), a saw or stone death costs 100 NP ("Krowaz Dominion").

use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use ko_protocol::{Opcode, Packet};
use rand::Rng;

use crate::npc::{build_npc_inout, NpcId, NPC_IN, NPC_OUT};
use crate::world::WorldState;
use crate::zone::SessionId;

pub const ZONE: u16 = 75;
pub const SAW_PROTO: u16 = 32153;
pub const STONE_PROTO: u16 = 32152;
/// HP a saw touch costs.
pub const SAW_DAMAGE: i16 = 370;
/// HP the poison gas costs per movement packet.
pub const GAS_DAMAGE: i16 = 1500;
/// Half sizes of the poison gas box (`+-7` in x, `+-3` in z).
pub const GAS_HALF_X: f32 = 7.0;
pub const GAS_HALF_Z: f32 = 3.0;
/// NP lost by a saw / stone death.
pub const NP_LOSS: i32 = 100;
const TICK_MS: u64 = 250;
const SAW_RADIUS: f32 = 2.5;
const STONE_RADIUS: f32 = 3.0;
const HIT_COOLDOWN_MS: u64 = 700;
const DEFAULT_SPEED: f32 = 15.0;

struct Saw {
    home: (f32, f32),
    range: f32,
    speed: f32,
    from: (f32, f32),
    to: (f32, f32),
    start_ms: u64,
    arrive_ms: u64,
    idle_until_ms: u64,
}

enum StoneState {
    Hidden { until_ms: u64 },
    Rolling { idx: usize, from: (f32, f32), start_ms: u64, arrive_ms: u64 },
}

struct Stone {
    path: Vec<(f32, f32)>,
    start: (f32, f32),
    regen_ms: u64,
    speed: f32,
    state: StoneState,
}

fn saws() -> &'static DashMap<NpcId, Saw> {
    static M: OnceLock<DashMap<NpcId, Saw>> = OnceLock::new();
    M.get_or_init(DashMap::new)
}

fn stones() -> &'static DashMap<NpcId, Stone> {
    static M: OnceLock<DashMap<NpcId, Stone>> = OnceLock::new();
    M.get_or_init(DashMap::new)
}

fn hits() -> &'static DashMap<SessionId, u64> {
    static M: OnceLock<DashMap<SessionId, u64>> = OnceLock::new();
    M.get_or_init(DashMap::new)
}

fn wall_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// A wandering saw (called by the NPC spawn loader for proto 32153 in zone 75 with a spawn range).
pub fn register_saw(nid: NpcId, x: f32, z: f32, range: f32, speed: f32) {
    if range <= 0.0 {
        return;
    }
    saws().insert(
        nid,
        Saw {
            home: (x, z),
            range,
            speed: if speed > 0.0 { speed } else { DEFAULT_SPEED },
            from: (x, z),
            to: (x, z),
            start_ms: 0,
            arrive_ms: 0,
            idle_until_ms: wall_ms() + rand::thread_rng().gen_range(0..3000),
        },
    );
}

/// A rolling stone with its route (called by the NPC spawn loader for proto 32152 in zone 75).
/// The stone starts hidden and shows up after its own `regen_time`, so the four stones are staggered.
pub fn register_stone(world: &WorldState, nid: NpcId, x: f32, z: f32, path: &str, regen_secs: u64, speed: f32) {
    let path = crate::systems::chaos_arena::parse_path(path);
    if path.is_empty() {
        return;
    }
    let regen_ms = regen_secs.max(5) * 1000;
    world.update_npc_hp(nid, 0);
    stones().insert(
        nid,
        Stone {
            path,
            start: (x, z),
            regen_ms,
            speed: if speed > 0.0 { speed } else { 8.0 },
            state: StoneState::Hidden { until_ms: wall_ms() + regen_ms },
        },
    );
}

/// Forget every route (the NPC tables are being reloaded).
pub fn clear() {
    saws().clear();
    stones().clear();
}

fn broadcast_npc(world: &WorldState, nid: NpcId, inout: u8) {
    let Some(npc) = world.get_npc_instance(nid) else { return };
    let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) else { return };
    let pkt = build_npc_inout(inout, &npc, &tmpl);
    world.broadcast_to_3x3(npc.zone_id, npc.region_x, npc.region_z, Arc::new(pkt), None, npc.event_room);
}

fn travel_ms(dist: f32, speed: f32) -> u64 {
    (((dist / speed.max(1.0)) * 1000.0) as u64).max(TICK_MS)
}

/// Damage a player: HP packet with the attacker, death (no exp loss), the dead notice.
fn hurt(world: &WorldState, sid: SessionId, attacker: u32, damage: i16, kill: bool, np_loss: bool) {
    let Some(ch) = world.get_character_info(sid) else { return };
    if ch.hp <= 0 || world.is_player_dead(sid) {
        return;
    }
    let dmg = if kill { ch.max_hp.max(ch.hp) } else { damage };
    let new_hp = (ch.hp - dmg).max(0);
    world.update_character_hp(sid, new_hp);
    world.send_to_session_owned(sid, crate::systems::regen::build_hp_change_packet_with_attacker(ch.max_hp, new_hp, attacker));
    if new_hp <= 0 {
        crate::handler::dead::broadcast_death_with_killer(world, sid, attacker, 0);
        let mut atk = Packet::new(Opcode::WizAttack as u8);
        atk.write_u8(crate::attack_constants::LONG_ATTACK);
        atk.write_u8(crate::attack_constants::ATTACK_TARGET_DEAD);
        atk.write_u32(attacker);
        atk.write_u32(sid as u32);
        world.send_to_session_owned(sid, atk);
        if np_loss {
            crate::systems::loyalty::send_loyalty_change(world, sid, -NP_LOSS, false, false, false);
        }
    }
}

fn cooled_down(sid: SessionId, now_ms: u64) -> bool {
    if let Some(last) = hits().get(&sid).map(|v| *v) {
        if now_ms.saturating_sub(last) < HIT_COOLDOWN_MS {
            return false;
        }
    }
    hits().insert(sid, now_ms);
    true
}

fn in_krowaz(world: &WorldState, sid: SessionId) -> bool {
    world.get_position(sid).is_some_and(|p| p.zone_id == ZONE) && !world.is_gm(sid) && !world.is_player_dead(sid)
}

/// The client's own touch event on a saw (WIZ_CLIENT_EVENT on proto 32153).
pub fn touch_saw(world: &WorldState, sid: SessionId, nid: NpcId) {
    if in_krowaz(world, sid) && cooled_down(sid, wall_ms()) {
        hurt(world, sid, nid, SAW_DAMAGE, false, true);
    }
}

/// The client's own touch event on a rolling stone (type 181): instant death.
pub fn touch_stone(world: &WorldState, sid: SessionId, nid: NpcId) {
    if in_krowaz(world, sid) && !world.is_npc_dead(nid) {
        hurt(world, sid, nid, 0, true, true);
    }
}

/// `CUser::EventTrapProcess`: called for every movement packet of a player in Krowaz Dominion.
pub fn poison_gas(world: &WorldState, sid: SessionId, x: f32, z: f32) {
    if world.is_gm(sid) || world.is_player_dead(sid) {
        return;
    }
    let Some(zone) = world.get_zone(ZONE) else { return };
    let inside = zone
        .object_events
        .values()
        .any(|e| e.obj_type == crate::object_event_constants::OBJECT_POISON_GAS as i16 && (x - e.pos_x).abs() <= GAS_HALF_X && (z - e.pos_z).abs() <= GAS_HALF_Z);
    if inside {
        hurt(world, sid, sid as u32, GAS_DAMAGE, false, false);
    }
}

fn players(world: &WorldState) -> Vec<(SessionId, f32, f32)> {
    world
        .sessions_in_zone(ZONE)
        .into_iter()
        .filter(|sid| !world.is_gm(*sid) && !world.is_player_dead(*sid))
        .filter_map(|sid| world.get_position(sid).map(|p| (sid, p.x, p.z)))
        .collect()
}

/// One driver pass (idle while nobody is in the zone).
pub fn tick(world: &WorldState, now_ms: u64) {
    if saws().is_empty() && stones().is_empty() {
        return;
    }
    let ppl = players(world);
    if ppl.is_empty() {
        return;
    }
    let zone = world.get_zone(ZONE);
    let wall = now_ms;
    let mut rng = rand::thread_rng();

    let saw_ids: Vec<NpcId> = saws().iter().map(|e| *e.key()).collect();
    for nid in saw_ids {
        let Some(npc) = world.get_npc_instance(nid) else {
            saws().remove(&nid);
            continue;
        };
        let (x, z) = {
            let Some(mut s) = saws().get_mut(&nid) else { continue };
            let (x, z);
            if now_ms < s.arrive_ms {
                let p = crate::systems::chaos_arena::interpolate(s.from, s.to, s.start_ms, s.arrive_ms, now_ms);
                x = p.0;
                z = p.1;
            } else {
                x = s.to.0;
                z = s.to.1;
                if now_ms >= s.idle_until_ms {
                    // next random leg inside the box around the spawn point
                    let mut dest = None;
                    for _ in 0..6 {
                        let tx = s.home.0 + rng.gen_range(-s.range..=s.range);
                        let tz = s.home.1 + rng.gen_range(-s.range..=s.range);
                        if zone.as_ref().is_some_and(|zn| zn.is_valid_position(tx, tz)) {
                            dest = Some((tx, tz));
                            break;
                        }
                    }
                    if let Some((tx, tz)) = dest {
                        let dist = ((tx - x).powi(2) + (tz - z).powi(2)).sqrt();
                        s.from = (x, z);
                        s.to = (tx, tz);
                        s.start_ms = now_ms;
                        s.arrive_ms = now_ms + travel_ms(dist, s.speed);
                        s.idle_until_ms = s.arrive_ms + rng.gen_range(500..2500);
                        let speed = s.speed;
                        drop(s);
                        crate::systems::chaos_arena::send_npc_move(world, nid, tx, tz, speed);
                    } else {
                        s.idle_until_ms = now_ms + 2000;
                    }
                }
            }
            (x, z)
        };
        world.move_runtime_npc(nid, x, z);
        let r2 = SAW_RADIUS * SAW_RADIUS;
        for (sid, px, pz) in &ppl {
            if (px - x).powi(2) + (pz - z).powi(2) <= r2 && cooled_down(*sid, wall) {
                hurt(world, *sid, nid, SAW_DAMAGE, false, true);
            }
        }
        let _ = npc;
    }

    let stone_ids: Vec<NpcId> = stones().iter().map(|e| *e.key()).collect();
    for nid in stone_ids {
        let Some(npc) = world.get_npc_instance(nid) else {
            stones().remove(&nid);
            continue;
        };
        // 0 nothing, 1 appears, 2 disappears, 3 starts the next leg
        let mut next = 0u8;
        let mut cur_pos: Option<(f32, f32)> = None;
        let mut leg_to: Option<(f32, f32)> = None;
        let mut speed = 8.0f32;
        {
            let Some(mut st) = stones().get_mut(&nid) else { continue };
            let start = st.start;
            speed = st.speed;
            let regen_ms = st.regen_ms;
            let mut new_state: Option<StoneState> = None;
            match &st.state {
                StoneState::Hidden { until_ms } => {
                    if now_ms >= *until_ms {
                        let first = st.path[0];
                        let dist = ((first.0 - start.0).powi(2) + (first.1 - start.1).powi(2)).sqrt();
                        new_state = Some(StoneState::Rolling { idx: 0, from: start, start_ms: now_ms, arrive_ms: now_ms + travel_ms(dist, speed) });
                        next = 1;
                        cur_pos = Some(start);
                        leg_to = Some(first);
                    }
                }
                StoneState::Rolling { idx, from, start_ms, arrive_ms } => {
                    let (idx, from, start_ms, arrive_ms) = (*idx, *from, *start_ms, *arrive_ms);
                    let target = st.path[idx];
                    if now_ms < arrive_ms {
                        cur_pos = Some(crate::systems::chaos_arena::interpolate(from, target, start_ms, arrive_ms, now_ms));
                    } else if idx + 1 < st.path.len() {
                        let nxt = st.path[idx + 1];
                        let dist = ((nxt.0 - target.0).powi(2) + (nxt.1 - target.1).powi(2)).sqrt();
                        new_state = Some(StoneState::Rolling { idx: idx + 1, from: target, start_ms: now_ms, arrive_ms: now_ms + travel_ms(dist, speed) });
                        cur_pos = Some(target);
                        leg_to = Some(nxt);
                        next = 3;
                    } else {
                        // end of the route: gone until the regen time is over
                        new_state = Some(StoneState::Hidden { until_ms: now_ms + regen_ms });
                        next = 2;
                    }
                }
            }
            if let Some(ns) = new_state {
                st.state = ns;
            }
        }
        match next {
            1 => {
                if let Some((x, z)) = cur_pos {
                    world.move_runtime_npc(nid, x, z);
                }
                let hp = world.get_npc_template(npc.proto_id, npc.is_monster).map(|t| t.max_hp as i32).unwrap_or(1).max(1);
                world.update_npc_hp(nid, hp);
                broadcast_npc(world, nid, NPC_IN);
                if let Some((x, z)) = leg_to {
                    crate::systems::chaos_arena::send_npc_move(world, nid, x, z, speed);
                }
            }
            2 => {
                broadcast_npc(world, nid, NPC_OUT);
                world.update_npc_hp(nid, 0);
            }
            _ => {
                if let Some((x, z)) = cur_pos {
                    world.move_runtime_npc(nid, x, z);
                    stone_touch(world, nid, (x, z), &ppl);
                }
                if next == 3 {
                    if let Some((x, z)) = leg_to {
                        crate::systems::chaos_arena::send_npc_move(world, nid, x, z, speed);
                    }
                }
            }
        }
    }
}

fn stone_touch(world: &WorldState, nid: NpcId, at: (f32, f32), ppl: &[(SessionId, f32, f32)]) {
    if world.is_npc_dead(nid) {
        return;
    }
    let r2 = STONE_RADIUS * STONE_RADIUS;
    for (sid, px, pz) in ppl {
        if (px - at.0).powi(2) + (pz - at.1).powi(2) <= r2 {
            hurt(world, *sid, nid, 0, true, true);
        }
    }
}

/// Start the driver.
pub fn start_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(TICK_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if saws().is_empty() && stones().is_empty() {
                continue;
            }
            let now_ms = wall_ms();
            let w = Arc::clone(&world);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tick(&w, now_ms)));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_match_the_original() {
        assert_eq!(SAW_DAMAGE, 370);
        assert_eq!(GAS_DAMAGE, 1500);
        assert_eq!((GAS_HALF_X, GAS_HALF_Z), (7.0, 3.0));
    }
}
