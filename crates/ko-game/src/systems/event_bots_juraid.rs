//! Juraid Mountain event bots (zone 87, `EventBotKind::Juraid`).
//!
//! Juraid is a PvE race: each nation walks its own lane, every lane has three monster stages and
//! both lanes meet in the Deva room where the Deva Bird decides the event.
//!
//! ```text
//! Karus     spawn (224,272) -> (224,558) -> (224,848) -> (511,848) -> Deva room (508,513)
//! El Morad  spawn (799,748) -> (799,459) -> (799,171) -> (511,172) -> Deva room (508,513)
//! ```
//!
//! (The Karus lane is the x = 224 side: the human respawn points of `handler::regene` use the
//! same sides, Karus bridge 1 at (224,671), El Morad bridge 1 at (800,349).)
//!
//! ## What the bots do
//! * **Follow** - when a real player of the bot's nation is alive in the room the bot joins that
//!   player's party and stays next to him, hitting the monsters around him.
//! * **Lane runner** - when no real player of that nation is left the bots walk the lane
//!   themselves. A stage is "cleared" when no stage monster (main or child) is alive within
//!   [`STAGE_RADIUS`] of its waypoint; only then does the team move on to the next waypoint. The
//!   last leg ends in the Deva room.
//! * **Mirror fill** - both teams end up the same size: each nation is topped up by the usual
//!   config and the smaller team then gets extra bots until humans + bots match the other side
//!   ([`plan_fill`], the rule shared with BDW in `event_bots::plan_nation_fill`).
//!
//! ## Rewards / chests
//! * Bots only ever damage room monsters (never players, never the monuments).
//! * A bot kill goes through [`crate::handler::dead::track_juraid_monster_kill_by_name`] (child
//!   monsters, bridge counters, Deva finish) and then removes the monster without a loot roll, so
//!   no chest/drop exists for a bot kill, and bots get no exp/NP/items.
//! * While any real player is in the room the bots leave the Deva Bird at 10 % HP: the killing
//!   blow, and with it the chest, always belongs to a real player.
//! * Bots are room members flagged `prize_given` (reward loops skip them) and have no session, so
//!   party loot routing cannot select them either ([`crate::handler::npc_loot`] also refuses
//!   bot ids explicitly).

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use rand::Rng;

use crate::systems::bot_ai;
use crate::systems::event_bots::{
    attack_cooldown, attack_range, is_event_bot_sid, room_broadcast, step_toward, move_packet,
    BotEntry, EventBotConfig, EventBotKind, STEP_PER_TICK,
};
use crate::systems::event_room::{TempleEventType, MAX_ROOM_USERS_PER_NATION};
use crate::systems::juraid::{
    SUMMON_JURAID_CHILD, SUMMON_JURAID_DEVA, SUMMON_JURAID_MAIN, ZONE_JURAID,
};
use crate::world::types::USER_DEAD;
use crate::world::{BotId, BotInstance, WorldState};
use crate::zone::{calc_region, SessionId};
use ko_protocol::{Opcode, Packet};

// ── Geometry ─────────────────────────────────────────────────────────────

pub const KARUS_SPAWN: (f32, f32) = (224.0, 272.0);
pub const ELMORAD_SPAWN: (f32, f32) = (799.0, 748.0);
pub const DEVA_ROOM: (f32, f32) = (508.0, 513.0);
/// Waypoints per nation: three stage points and the Deva room.
pub const KARUS_CHAIN: [(f32, f32); 4] = [(224.0, 558.0), (224.0, 848.0), (511.0, 848.0), DEVA_ROOM];
pub const ELMORAD_CHAIN: [(f32, f32); 4] = [(799.0, 459.0), (799.0, 171.0), (511.0, 172.0), DEVA_ROOM];
/// Index of the Deva room in a chain.
pub const DEVA_STAGE: u8 = 3;

/// Stage monsters live within this many metres of their waypoint.
pub const STAGE_RADIUS: f32 = 60.0;
/// Monsters this close to the Deva room centre are part of the last fight.
const DEVA_RADIUS: f32 = 45.0;
/// A follower helps with monsters up to this far from the player it follows.
const ASSIST_RADIUS: f32 = 40.0;
/// A follower stays this close to its player.
const FOLLOW_DISTANCE: f32 = 6.0;
/// Start picking monsters (instead of just walking) this far before a stage waypoint.
const ENGAGE_MARGIN: f32 = 30.0;
/// Bot damage per swing = class damage x this (stands in for the skill rotation).
const DAMAGE_MULT: i32 = 8;
const SCAN_MS: u64 = 1_000;
const PARTY_CHECK_MS: u64 = 2_000;
const MAX_PARTY_SIZE: usize = 8;

pub fn chain(nation: u8) -> &'static [(f32, f32); 4] {
    if nation == 1 {
        &KARUS_CHAIN
    } else {
        &ELMORAD_CHAIN
    }
}

/// Where a bot of `nation` appears in `zone_id` (only Juraid has a fixed spawn).
pub fn spawn_override(zone_id: u16, nation: u8) -> Option<(f32, f32)> {
    if zone_id != ZONE_JURAID {
        return None;
    }
    match nation {
        1 => Some(KARUS_SPAWN),
        2 => Some(ELMORAD_SPAWN),
        _ => None,
    }
}

// ── Mirror fill ──────────────────────────────────────────────────────────

/// Bots to add per nation, `[karus, elmorad]`, for a room that holds `humans = [karus, elmorad]`
/// real players: the usual top-up per nation, then the smaller team is mirrored up until both
/// teams (humans + bots) are the same size ([`crate::systems::event_bots::plan_nation_fill`]).
pub fn plan_fill(cfg: &EventBotConfig, humans: [usize; 2]) -> [usize; 2] {
    if !cfg.enabled || humans[0] + humans[1] < cfg.min_humans.max(1) as usize {
        return [0, 0];
    }
    crate::systems::event_bots::plan_nation_fill(cfg, humans, MAX_ROOM_USERS_PER_NATION)
}

// ── Stage logic (pure) ───────────────────────────────────────────────────

/// A live monster of a Juraid room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mob {
    pub nid: u32,
    pub proto: u16,
    pub summon: u8,
    pub x: f32,
    pub z: f32,
}

impl Mob {
    pub fn is_stage_mob(&self) -> bool {
        self.summon == SUMMON_JURAID_MAIN || self.summon == SUMMON_JURAID_CHILD
    }
    pub fn is_deva(&self) -> bool {
        self.summon == SUMMON_JURAID_DEVA
    }
}

fn dist2(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
}

/// Alive stage monsters within [`STAGE_RADIUS`] of `point`.
pub fn stage_mobs_near(mobs: &[Mob], point: (f32, f32)) -> usize {
    mobs.iter()
        .filter(|m| m.is_stage_mob() && dist2((m.x, m.z), point) <= STAGE_RADIUS * STAGE_RADIUS)
        .count()
}

/// How far a nation's team has got: the waypoint index it is clearing / heading for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TeamState {
    pub stage: u8,
    /// The room's monsters have been seen at least once (they spawn right after the bots do, an
    /// empty room must not count as "everything cleared").
    pub seen: bool,
}

/// Move `state` forward over every waypoint whose stage is fully cleared. Returns `true` when
/// the stage changed.
pub fn advance_stage(state: &mut TeamState, nation: u8, mobs: &[Mob]) -> bool {
    if !state.seen {
        state.seen = mobs.iter().any(|m| m.is_stage_mob() || m.is_deva());
        return false;
    }
    let ch = chain(nation);
    let before = state.stage;
    while state.stage < DEVA_STAGE && stage_mobs_near(mobs, ch[state.stage as usize]) == 0 {
        state.stage += 1;
    }
    state.stage != before
}

/// What a bot should do this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Plan {
    Attack(Mob),
    Walk((f32, f32)),
    Idle,
}

fn nearest<'a>(pos: (f32, f32), it: impl Iterator<Item = &'a Mob>) -> Option<Mob> {
    it.min_by(|a, b| {
        dist2(pos, (a.x, a.z))
            .partial_cmp(&dist2(pos, (b.x, b.z)))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
    .copied()
}

fn engage(pos: (f32, f32), range: f32, mob: Mob) -> Plan {
    if dist2(pos, (mob.x, mob.z)).sqrt() <= range {
        Plan::Attack(mob)
    } else {
        Plan::Walk((mob.x, mob.z))
    }
}

/// Decide the action of one bot.
/// * `follow`: position of the nearest living real player of the bot's nation, if any.
/// * `stage`: the nation team's current waypoint index.
pub fn plan_action(pos: (f32, f32), range: f32, nation: u8, stage: u8, mobs: &[Mob], follow: Option<(f32, f32)>) -> Plan {
    if let Some(leader) = follow {
        let helped = nearest(
            pos,
            mobs.iter()
                .filter(|m| (m.is_stage_mob() || m.is_deva()) && dist2((m.x, m.z), leader) <= ASSIST_RADIUS * ASSIST_RADIUS),
        );
        if let Some(m) = helped {
            return engage(pos, range, m);
        }
        return if dist2(pos, leader) > FOLLOW_DISTANCE * FOLLOW_DISTANCE {
            Plan::Walk(leader)
        } else {
            Plan::Idle
        };
    }
    let stage = stage.min(DEVA_STAGE);
    let point = chain(nation)[stage as usize];
    if stage == DEVA_STAGE {
        let target = nearest(
            pos,
            mobs.iter()
                .filter(|m| (m.is_deva() || m.is_stage_mob()) && dist2((m.x, m.z), point) <= DEVA_RADIUS * DEVA_RADIUS),
        );
        return match target {
            Some(m) => engage(pos, range, m),
            None => Plan::Walk(point),
        };
    }
    // lane stage: fight once close to the waypoint, walk there before that
    let near_point = dist2(pos, point) <= (STAGE_RADIUS + ENGAGE_MARGIN).powi(2);
    if near_point {
        let target = nearest(
            pos,
            mobs.iter()
                .filter(|m| m.is_stage_mob() && dist2((m.x, m.z), point) <= STAGE_RADIUS * STAGE_RADIUS),
        );
        if let Some(m) = target {
            return engage(pos, range, m);
        }
    }
    Plan::Walk(point)
}

/// Whether the bots may deliver the killing blow to `mob`. The Deva Bird (the chest holder) is
/// left to the real players while any of them is in the room.
pub fn may_kill(mob: &Mob, humans_in_room: usize) -> bool {
    !mob.is_deva() || humans_in_room == 0
}

// ── Runtime state ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Human {
    pub sid: SessionId,
    pub nation: u8,
    pub x: f32,
    pub z: f32,
}

/// Snapshot of one Juraid room, refreshed once per [`SCAN_MS`] (not per bot).
#[derive(Debug, Default)]
pub struct RoomView {
    pub at_ms: u64,
    pub mobs: Vec<Mob>,
    /// Living real players standing in the zone.
    pub humans: Vec<Human>,
    /// Real players that belong to the room (alive or not, not logged out).
    pub human_total: usize,
}

impl RoomView {
    fn nearest_human(&self, nation: u8, pos: (f32, f32)) -> Option<Human> {
        self.humans
            .iter()
            .filter(|h| h.nation == nation)
            .min_by(|a, b| {
                dist2(pos, (a.x, a.z))
                    .partial_cmp(&dist2(pos, (b.x, b.z)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }
}

#[derive(Default)]
struct RoomRt {
    view: Option<Arc<RoomView>>,
    /// Index 0 = Karus, 1 = El Morad.
    teams: [TeamState; 2],
    party_next_ms: HashMap<BotId, u64>,
}

/// Per-world Juraid bookkeeping, owned by the `EventBotRegistry`.
#[derive(Default)]
pub struct JuraidRt {
    rooms: Mutex<HashMap<u16, RoomRt>>,
}

impl JuraidRt {
    pub fn reset_room(&self, room: u16) {
        self.rooms.lock().remove(&room);
    }

    /// Drop the state of rooms that no longer hold a Juraid bot.
    pub fn housekeeping(&self, rooms_with_bots: &[u16]) {
        self.rooms.lock().retain(|room, _| rooms_with_bots.contains(room));
    }

    pub fn team_stage(&self, room: u16, nation: u8) -> u8 {
        self.rooms
            .lock()
            .get(&room)
            .map(|r| r.teams[team_idx(nation)].stage)
            .unwrap_or(0)
    }

    fn cached_view(&self, room: u16, now_ms: u64) -> Option<Arc<RoomView>> {
        self.rooms
            .lock()
            .get(&room)
            .and_then(|r| r.view.clone())
            .filter(|v| now_ms.saturating_sub(v.at_ms) < SCAN_MS)
    }

    /// Store a fresh view and advance both teams' stages with it.
    fn store_view(&self, room: u16, view: Arc<RoomView>) {
        let mut rooms = self.rooms.lock();
        let rt = rooms.entry(room).or_default();
        for nation in [1u8, 2u8] {
            advance_stage(&mut rt.teams[team_idx(nation)], nation, &view.mobs);
        }
        rt.view = Some(view);
    }

    fn party_due(&self, room: u16, bot: BotId, now_ms: u64) -> bool {
        let mut rooms = self.rooms.lock();
        let rt = rooms.entry(room).or_default();
        let next = rt.party_next_ms.entry(bot).or_insert(0);
        if now_ms >= *next {
            *next = now_ms + PARTY_CHECK_MS;
            true
        } else {
            false
        }
    }
}

fn team_idx(nation: u8) -> usize {
    if nation == 1 {
        0
    } else {
        1
    }
}

fn scan_room(world: &WorldState, room: u16, now_ms: u64) -> RoomView {
    let mobs: Vec<Mob> = world
        .npcs_near(ZONE_JURAID, 512.0, 512.0, 900.0, room)
        .into_iter()
        .filter(|n| {
            n.is_monster
                && matches!(n.summon_type, SUMMON_JURAID_MAIN | SUMMON_JURAID_CHILD | SUMMON_JURAID_DEVA)
                && !world.is_npc_dead(n.nid)
        })
        .map(|n| Mob { nid: n.nid, proto: n.proto_id, summon: n.summon_type, x: n.x, z: n.z })
        .collect();

    let mut humans = Vec::new();
    let mut human_total = 0usize;
    if let Some(r) = world.event_room_manager.get_room(TempleEventType::JuraidMountain, room as u8) {
        for u in r.karus_users.values().chain(r.elmorad_users.values()) {
            if u.logged_out || is_event_bot_sid(world, u.session_id) {
                continue;
            }
            human_total += 1;
            let (Some(ch), Some(pos)) = (world.get_character_info(u.session_id), world.get_position(u.session_id)) else {
                continue;
            };
            if pos.zone_id != ZONE_JURAID || ch.hp <= 0 || ch.res_hp_type == USER_DEAD {
                continue;
            }
            humans.push(Human { sid: u.session_id, nation: u.nation, x: pos.x, z: pos.z });
        }
    }
    RoomView { at_ms: now_ms, mobs, humans, human_total }
}

fn room_view(world: &WorldState, room: u16, now_ms: u64) -> Arc<RoomView> {
    let rt = &world.event_bots.juraid;
    if let Some(v) = rt.cached_view(room, now_ms) {
        return v;
    }
    let view = Arc::new(scan_room(world, room, now_ms));
    rt.store_view(room, Arc::clone(&view));
    view
}

// ── Driver ───────────────────────────────────────────────────────────────

/// One tick of one alive Juraid bot (called by `event_bots::tick_bot` once the room is fighting).
pub fn tick_bot(world: &WorldState, entry: &BotEntry, bot: &BotInstance, now_ms: u64) {
    let rt = &world.event_bots.juraid;
    let view = room_view(world, entry.room, now_ms);
    if rt.party_due(entry.room, bot.id, now_ms) {
        join_party(world, bot, entry.nation, &view);
    }
    let pos = (bot.x, bot.z);
    let follow = view.nearest_human(entry.nation, pos).map(|h| (h.x, h.z));
    let stage = rt.team_stage(entry.room, entry.nation);
    match plan_action(pos, attack_range(bot), entry.nation, stage, &view.mobs, follow) {
        Plan::Attack(mob) => {
            if now_ms >= entry.next_attack_ms {
                world
                    .event_bots
                    .update(bot.id, |e| e.next_attack_ms = now_ms + attack_cooldown(bot));
                attack_mob(world, entry, bot, mob, view.human_total);
            }
        }
        Plan::Walk(goal) => walk(world, entry, bot, goal),
        Plan::Idle => {}
    }
}

fn attack_mob(world: &WorldState, entry: &BotEntry, bot: &BotInstance, mob: Mob, humans_in_room: usize) {
    let Some(hp) = world.get_npc_hp(mob.nid).filter(|h| *h > 0) else { return };
    if world.get_npc_instance(mob.nid).is_none() {
        return;
    }
    let floor = if may_kill(&mob, humans_in_room) {
        0
    } else {
        // the chest holder: stop at 10 % and wait for a real player
        world
            .get_npc_template(mob.proto, true)
            .map(|t| (t.max_hp as i32 / 10).max(1))
            .unwrap_or(1)
    };
    if hp <= floor {
        return;
    }
    bot_ai::event_bot_swing(world, bot.id, mob.nid);
    if rand::thread_rng().gen_range(0..100) < crate::systems::event_bots::MISS_PERCENT {
        return;
    }
    let damage = bot_ai::calculate_bot_damage(bot) as i32 * DAMAGE_MULT;
    let new_hp = hp - damage;
    if new_hp > floor {
        world.update_npc_hp(mob.nid, new_hp);
    } else if floor > 0 {
        world.update_npc_hp(mob.nid, floor);
    } else {
        kill_mob(world, entry, &mob);
    }
}

/// The bot lands the killing blow: score it like a player kill (children, bridge counters, Deva
/// finish) and remove the monster. No loot roll, no exp, no NP: a bot kill never makes a chest.
fn kill_mob(world: &WorldState, entry: &BotEntry, mob: &Mob) {
    crate::handler::dead::track_juraid_monster_kill_by_name(
        world,
        &entry.name,
        entry.nation,
        mob.proto,
        entry.room,
        mob.summon,
        mob.x,
        mob.z,
    );
    world.clear_npc_damage(mob.nid);
    world.kill_npc(mob.nid);
}

/// One walking step toward `goal`, broadcast to the bot's room (same movement rules as the other
/// event bots: A* on the SMD walkability mask).
fn walk(world: &WorldState, entry: &BotEntry, bot: &BotInstance, goal: (f32, f32)) {
    let mut path = entry.path.clone();
    let Some((nx, ny, nz)) = step_toward(world, bot, goal, STEP_PER_TICK, &mut path) else {
        world.event_bots.update(bot.id, |e| e.path = None);
        return;
    };
    room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, move_packet(bot, nx, ny, nz, 1));
    let (rx, rz) = (calc_region(nx), calc_region(nz));
    let crossed = rx != bot.region_x || rz != bot.region_z;
    if crossed {
        let mut out = Packet::new(Opcode::WizUserInout as u8);
        out.write_u8(2);
        out.write_u8(0);
        out.write_u32(bot.id);
        room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, out);
    }
    world.update_bot(bot.id, |b| {
        b.x = nx;
        b.y = ny;
        b.z = nz;
        b.region_x = rx;
        b.region_z = rz;
        b.target_id = -1;
    });
    if crossed {
        if let Some(b) = world.get_bot(bot.id) {
            let pkt = bot_ai::build_bot_inout_packet(&b, world, 1);
            room_broadcast(world, b.zone_id, rx, rz, entry.room, pkt);
        }
    }
    world.event_bots.update(bot.id, |e| e.path = path);
}

// ── Party ────────────────────────────────────────────────────────────────

/// Join the party of a living real player of the bot's nation (creating a party around a lone
/// player). No-op when the bot already has a party or no player of its nation is alive.
fn join_party(world: &WorldState, bot: &BotInstance, nation: u8, view: &RoomView) {
    let me = bot.id as SessionId;
    if world.get_party_id(me).is_some() {
        return;
    }
    let mut candidates: Vec<&Human> = view.humans.iter().filter(|h| h.nation == nation).collect();
    candidates.sort_by(|a, b| {
        dist2((bot.x, bot.z), (a.x, a.z))
            .partial_cmp(&dist2((bot.x, bot.z), (b.x, b.z)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for h in candidates {
        let pid = match world.get_party_id(h.sid) {
            Some(pid) => {
                if world.get_party(pid).is_none_or(|p| p.active_members().len() >= MAX_PARTY_SIZE) {
                    continue;
                }
                pid
            }
            None => {
                let Some(pid) = world.create_party(h.sid) else { continue };
                if let Some((pos, room)) = world.with_session(h.sid, |s| (s.position, s.event_room)) {
                    let sc = crate::handler::regene::build_state_change_broadcast(
                        h.sid as u32,
                        crate::state_change_constants::STATE_CHANGE_PARTY_LEADER,
                        1,
                    );
                    world.broadcast_to_region_sync(pos.zone_id, pos.region_x, pos.region_z, Arc::new(sc), None, room);
                }
                pid
            }
        };
        if !world.add_party_member(pid, me) {
            continue;
        }
        let target_number = world.get_party(pid).map_or(-1, |p| p.target_number_id);
        let info = crate::handler::party::build_bot_party_member_info(bot, 1, target_number);
        world.send_to_party(pid, &info);
        bot_ai::broadcast_bot_party_hp(world, bot.id);
        return;
    }
}

/// Number of bots of a Juraid room that currently stand in a party (diagnostics / tests).
pub fn bots_in_party(world: &WorldState) -> usize {
    world
        .event_bots
        .ids_of(EventBotKind::Juraid)
        .into_iter()
        .filter(|id| world.get_party_id(*id as SessionId).is_some())
        .count()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(per_team: u8) -> EventBotConfig {
        EventBotConfig { enabled: true, bots_per_team: per_team, ..Default::default() }.sanitized(EventBotKind::Juraid)
    }

    fn mob(nid: u32, summon: u8, p: (f32, f32)) -> Mob {
        Mob { nid, proto: 8100, summon, x: p.0, z: p.1 }
    }

    #[test]
    fn chains_match_the_design() {
        // Karus runs the x = 224 lane, El Morad the x = 799 lane (owner-confirmed after a live test)
        assert_eq!(KARUS_CHAIN, [(224.0, 558.0), (224.0, 848.0), (511.0, 848.0), (508.0, 513.0)]);
        assert_eq!(ELMORAD_CHAIN, [(799.0, 459.0), (799.0, 171.0), (511.0, 172.0), (508.0, 513.0)]);
        assert_eq!(spawn_override(87, 1), Some((224.0, 272.0)));
        assert_eq!(spawn_override(87, 2), Some((799.0, 748.0)));
        assert_eq!(chain(1), &KARUS_CHAIN);
        assert_eq!(chain(2), &ELMORAD_CHAIN);
        assert_eq!(spawn_override(85, 1), None);
    }

    #[test]
    fn lanes_agree_with_the_human_respawn_sides() {
        // handler::regene puts a dead Karus player on bridge 1 at (224,671), El Morad at (800,349)
        assert!((KARUS_CHAIN[0].0 - 224.0).abs() < 1.0 && (KARUS_SPAWN.0 - 224.0).abs() < 1.0);
        assert!((ELMORAD_CHAIN[0].0 - 799.0).abs() < 1.5 && (ELMORAD_SPAWN.0 - 799.0).abs() < 1.5);
    }

    #[test]
    fn stage_clear_moves_to_the_next_waypoint() {
        let mut st = TeamState::default();
        let ch = chain(2);
        let mut mobs = vec![
            mob(1, SUMMON_JURAID_MAIN, (ch[0].0 + 4.0, ch[0].1)),
            mob(2, SUMMON_JURAID_CHILD, (ch[0].0, ch[0].1 + 9.0)),
            mob(3, SUMMON_JURAID_MAIN, (ch[1].0, ch[1].1)),
            mob(4, SUMMON_JURAID_MAIN, (ch[2].0, ch[2].1)),
            mob(5, SUMMON_JURAID_DEVA, DEVA_ROOM),
        ];
        // first sight of the room's monsters only arms the tracker
        assert!(!advance_stage(&mut st, 2, &[]));
        assert!(!st.seen, "an empty room is not 'cleared'");
        assert!(!advance_stage(&mut st, 2, &mobs));
        assert!(st.seen);
        assert_eq!(st.stage, 0);
        // one monster of stage 0 left: the team stays
        mobs.remove(0);
        assert!(!advance_stage(&mut st, 2, &mobs));
        assert_eq!(st.stage, 0);
        // the last one dies: waypoint 1
        mobs.remove(0);
        assert!(advance_stage(&mut st, 2, &mobs));
        assert_eq!(st.stage, 1);
        // stage 1 and 2 clear in one go -> straight to the Deva room, and it stops there
        mobs.retain(|m| m.is_deva());
        assert!(advance_stage(&mut st, 2, &mobs));
        assert_eq!(st.stage, DEVA_STAGE);
        assert!(!advance_stage(&mut st, 2, &mobs));
    }

    #[test]
    fn nations_progress_independently() {
        let mut k = TeamState::default();
        let mut e = TeamState::default();
        let mobs = vec![
            mob(1, SUMMON_JURAID_MAIN, KARUS_CHAIN[0]),
            mob(2, SUMMON_JURAID_MAIN, ELMORAD_CHAIN[0]),
            mob(3, SUMMON_JURAID_MAIN, ELMORAD_CHAIN[1]),
        ];
        advance_stage(&mut k, 1, &mobs);
        advance_stage(&mut e, 2, &mobs);
        // El Morad's first stage dies, Karus' does not
        let mobs: Vec<Mob> = mobs.into_iter().filter(|m| m.nid != 2).collect();
        advance_stage(&mut k, 1, &mobs);
        advance_stage(&mut e, 2, &mobs);
        assert_eq!(k.stage, 0);
        assert_eq!(e.stage, 1);
    }

    #[test]
    fn lane_runner_walks_then_fights_then_moves_on() {
        let ch = chain(1);
        let range = 5.5;
        let stage_mob = mob(1, SUMMON_JURAID_MAIN, (ch[0].0 + 3.0, ch[0].1));
        // far away: walk to the waypoint
        assert_eq!(plan_action(KARUS_SPAWN, range, 1, 0, &[stage_mob], None), Plan::Walk(ch[0]));
        // arrived next to the monster: hit it
        let at = (ch[0].0 + 1.0, ch[0].1);
        assert_eq!(plan_action(at, range, 1, 0, &[stage_mob], None), Plan::Attack(stage_mob));
        // near the waypoint but the monster is out of reach: close the distance
        let pos = (ch[0].0 - 20.0, ch[0].1);
        assert_eq!(plan_action(pos, range, 1, 0, &[stage_mob], None), Plan::Walk((stage_mob.x, stage_mob.z)));
        // cleared and advanced: head for the second waypoint
        assert_eq!(plan_action(at, range, 1, 1, &[], None), Plan::Walk(ch[1]));
        // the last leg ends at the Deva bird
        let deva = mob(9, SUMMON_JURAID_DEVA, DEVA_ROOM);
        assert_eq!(plan_action(KARUS_CHAIN[2], range, 1, DEVA_STAGE, &[deva], None), Plan::Walk((DEVA_ROOM.0, DEVA_ROOM.1)));
        assert_eq!(plan_action((DEVA_ROOM.0 + 2.0, DEVA_ROOM.1), range, 1, DEVA_STAGE, &[deva], None), Plan::Attack(deva));
    }

    #[test]
    fn followers_stay_with_their_player_and_help_nearby() {
        let leader = (300.0, 600.0);
        let near = mob(1, SUMMON_JURAID_CHILD, (leader.0 + 10.0, leader.1));
        let far = mob(2, SUMMON_JURAID_MAIN, (leader.0 + 200.0, leader.1));
        // monster beside the player: engage it
        assert_eq!(plan_action((leader.0 + 9.0, leader.1), 5.5, 2, 0, &[near, far], Some(leader)), Plan::Attack(near));
        // nothing near the player: follow him, not the far monster or the lane
        assert_eq!(plan_action((100.0, 600.0), 5.5, 2, 0, &[far], Some(leader)), Plan::Walk(leader));
        // arrived: stand still
        assert_eq!(plan_action((leader.0 + 2.0, leader.1), 5.5, 2, 0, &[far], Some(leader)), Plan::Idle);
    }

    #[test]
    fn deva_last_hit_belongs_to_players() {
        let deva = mob(9, SUMMON_JURAID_DEVA, DEVA_ROOM);
        let normal = mob(1, SUMMON_JURAID_MAIN, ELMORAD_CHAIN[0]);
        assert!(!may_kill(&deva, 1));
        assert!(may_kill(&deva, 0), "a bot-only room can still finish");
        assert!(may_kill(&normal, 5));
    }

    #[test]
    fn mirror_fill_makes_both_teams_the_same_size() {
        let c = cfg(4);
        // 3 Karus players, nobody on El Morad: Karus topped up to 4, El Morad mirrors the 4
        assert_eq!(plan_fill(&c, [3, 0]), [1, 4]);
        // 5 Karus players (more than bots_per_team): El Morad gets 5 bots
        assert_eq!(plan_fill(&c, [0, 5]), [5, 0]);
        assert_eq!(plan_fill(&c, [5, 0]), [0, 5]);
        // the owner's live case: a 5-man Karus party, bots_per_team 5 -> 5 El Morad bots
        assert_eq!(plan_fill(&cfg(5), [5, 0]), [0, 5]);
        // both sides populated: plain top-up already makes 4 vs 4
        assert_eq!(plan_fill(&c, [2, 3]), [2, 1]);
        // both sides populated, one above bots_per_team: the other side is mirrored up to it
        assert_eq!(plan_fill(&c, [6, 1]), [0, 5]);
        // never more than a nation can hold
        assert_eq!(plan_fill(&cfg(8), [0, 8]), [8, 0]);
        // no humans / disabled / min_humans: nothing
        assert_eq!(plan_fill(&c, [0, 0]), [0, 0]);
        assert_eq!(plan_fill(&EventBotConfig::default(), [3, 0]), [0, 0]);
        let mut strict = cfg(4);
        strict.min_humans = 3;
        assert_eq!(plan_fill(&strict, [1, 1]), [0, 0]);
    }

    #[test]
    fn team_state_is_cleared_with_the_room() {
        let rt = JuraidRt::default();
        rt.store_view(3, Arc::new(RoomView { mobs: vec![mob(1, SUMMON_JURAID_MAIN, KARUS_CHAIN[0])], ..Default::default() }));
        rt.store_view(3, Arc::new(RoomView::default()));
        assert_eq!(rt.team_stage(3, 1), 3, "everything dead after the monsters were seen");
        rt.housekeeping(&[4]);
        assert_eq!(rt.team_stage(3, 1), 0);
        rt.store_view(5, Arc::new(RoomView::default()));
        rt.reset_room(5);
        assert_eq!(rt.team_stage(5, 1), 0);
    }

    // ── world level ──────────────────────────────────────────────────────

    use crate::systems::event_room::EventUser;
    use crate::world::{CharacterInfo, Position};

    fn template(sid: u16, hp: u32) -> crate::npc::NpcTemplate {
        crate::npc::NpcTemplate {
            s_sid: sid,
            is_monster: true,
            name: "Juraid".to_string(),
            pid: sid,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 1,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 80,
            max_hp: hp,
            max_mp: 0,
            attack: 1,
            ac: 0,
            hit_rate: 100,
            evade_rate: 50,
            damage: 10,
            attack_delay: 1500,
            speed_1: 1000,
            speed_2: 500,
            stand_time: 3000,
            search_range: 0,
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
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }

    /// Juraid world: zone 87, one running room holding `humans` = (sid, nation, name).
    fn juraid_world(humans: &[(SessionId, u8, &str)]) -> WorldState {
        let world = WorldState::new();
        world.ensure_zone(ZONE_JURAID, 1024);
        world.insert_npc_template(template(8100, 1_000));
        world.insert_npc_template(template(8106, 1_000));
        let erm = &world.event_room_manager;
        erm.create_rooms(TempleEventType::JuraidMountain, 1);
        for (sid, nation, name) in humans {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            world.register_session(*sid, tx);
            world.register_ingame(
                *sid,
                CharacterInfo {
                    session_id: *sid,
                    name: (*name).into(),
                    nation: *nation,
                    level: 70,
                    hp: 5000,
                    max_hp: 5000,
                    authority: 1,
                    ..Default::default()
                },
                Position {
                    zone_id: ZONE_JURAID,
                    x: 300.0,
                    y: 0.0,
                    z: 600.0,
                    region_x: calc_region(300.0),
                    region_z: calc_region(600.0),
                },
            );
            world.update_session(*sid, |h| h.event_room = 1);
            let mut room = erm.get_room_mut(TempleEventType::JuraidMountain, 1).unwrap();
            assert!(room.add_user(EventUser {
                user_name: name.to_string(),
                session_id: *sid,
                nation: *nation,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            }));
        }
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
            s.is_active = true;
            s.is_attackable = true;
        });
        world
    }

    fn fill(world: &WorldState, per_team: u8) -> usize {
        world.event_bots.set_config(EventBotKind::Juraid, cfg(per_team));
        crate::systems::event_bots::fill_temple_rooms(world, TempleEventType::JuraidMountain)
    }

    fn first_bot(world: &WorldState) -> (BotId, BotEntry) {
        let id = world.event_bots.ids_of(EventBotKind::Juraid)[0];
        (id, world.event_bots.get(id).unwrap())
    }

    #[test]
    fn mirror_fill_spawns_the_opposing_nation_at_its_spawn_point() {
        let world = juraid_world(&[(1, 1, "KarusMan"), (2, 1, "KarusTwo")]);
        // per-team 1: Karus already has players (no teammates), El Morad gets 2 mirror bots
        assert_eq!(fill(&world, 1), 2);
        let ids = world.event_bots.ids_of(EventBotKind::Juraid);
        assert_eq!(ids.len(), 2);
        for id in ids {
            let b = world.get_bot(id).unwrap();
            assert_eq!(b.nation, 2);
            assert!((b.x - ELMORAD_SPAWN.0).abs() <= 3.5 && (b.z - ELMORAD_SPAWN.1).abs() <= 3.5, "{} {}", b.x, b.z);
            assert!(is_event_bot_sid(&world, id as SessionId));
        }
        let room = world.event_room_manager.get_room(TempleEventType::JuraidMountain, 1).unwrap();
        assert_eq!(room.elmorad_users.len(), 2);
        assert!(room.elmorad_users.values().all(|u| u.prize_given), "bots never get rewards");
        drop(room);
        // the event ends: the sweep removes every bot
        world.event_room_manager.destroy_rooms(TempleEventType::JuraidMountain);
        world.event_room_manager.reset_temple_event();
        assert_eq!(crate::systems::event_bots::sweep(&world), 2);
        assert_eq!(world.event_bots.total(), 0);
    }

    #[test]
    fn a_bot_kill_scores_the_bridge_counter_and_leaves_no_loot() {
        let world = juraid_world(&[(1, 1, "KarusMan")]);
        assert_eq!(fill(&world, 1), 1);
        let (id, entry) = first_bot(&world);
        let bot = world.get_bot(id).unwrap();
        let nid = world.spawn_event_npc_ex(
            8100,
            true,
            ZONE_JURAID,
            ELMORAD_CHAIN[0].0,
            ELMORAD_CHAIN[0].1,
            1,
            1,
            SUMMON_JURAID_MAIN,
        )[0];
        world.update_npc_hp(nid, 1);
        let m = Mob { nid, proto: 8100, summon: SUMMON_JURAID_MAIN, x: ELMORAD_CHAIN[0].0, z: ELMORAD_CHAIN[0].1 };
        for _ in 0..50 {
            attack_mob(&world, &entry, &bot, m, 1); // 12 % miss chance per swing
            if world.is_npc_dead(nid) {
                break;
            }
        }
        assert!(world.is_npc_dead(nid));
        assert!(world.get_npc_instance(nid).is_none(), "monster removed");
        let st = world.get_juraid_bridge_state(1).unwrap_or_default();
        assert_eq!(st.elmorad_main_kills, 1, "counted for the bot's nation (El Morad)");
        assert_eq!(st.karus_main_kills, 0);
    }

    #[test]
    fn bots_leave_the_deva_to_real_players_and_finish_it_only_without_any() {
        let world = juraid_world(&[(1, 1, "KarusMan")]);
        assert_eq!(fill(&world, 1), 1);
        let (id, entry) = first_bot(&world);
        let bot = world.get_bot(id).unwrap();
        let nid =
            world.spawn_event_npc_ex(8106, true, ZONE_JURAID, DEVA_ROOM.0, DEVA_ROOM.1, 1, 1, SUMMON_JURAID_DEVA)[0];
        let m = Mob { nid, proto: 8106, summon: SUMMON_JURAID_DEVA, x: DEVA_ROOM.0, z: DEVA_ROOM.1 };
        world.update_npc_hp(nid, 400);
        // a real player is in the room: the bot softens to the 10 % floor (100 of 1000) and stops
        for _ in 0..60 {
            attack_mob(&world, &entry, &bot, m, 1);
        }
        assert_eq!(world.get_npc_hp(nid), Some(100), "real players keep the killing blow (and the chest)");
        {
            let room = world.event_room_manager.get_room(TempleEventType::JuraidMountain, 1).unwrap();
            assert_eq!(room.winner_nation, 0);
        }
        // nobody real left: the bots may end it
        for _ in 0..60 {
            attack_mob(&world, &entry, &bot, m, 0);
            if world.is_npc_dead(nid) {
                break;
            }
        }
        assert!(world.is_npc_dead(nid));
        let room = world.event_room_manager.get_room(TempleEventType::JuraidMountain, 1).unwrap();
        assert_eq!(room.winner_nation, 2, "the bot's nation (El Morad mirror) won");
    }

    #[test]
    fn bots_never_receive_party_loot() {
        let world = juraid_world(&[(1, 1, "KarusMan")]);
        assert_eq!(fill(&world, 1), 1);
        let (id, _) = first_bot(&world);
        let bot_sid = id as SessionId;
        let pid = world.create_party(1).unwrap();
        assert!(world.add_party_member(pid, bot_sid));
        for _ in 0..16 {
            let r = crate::handler::npc_loot::get_item_routing_user(&world, pid, 1, 389_196_000, 1);
            assert_ne!(r, Some(bot_sid), "a bot must never be picked as chest/loot receiver");
        }
    }

    #[test]
    fn bots_join_the_party_of_a_real_player() {
        let world = juraid_world(&[(1, 1, "KarusMan")]);
        assert_eq!(fill(&world, 3), 5); // 2 Karus teammates + 3 El Morad mirror bots (equal teams)
        let karus_bots: Vec<BotId> = world
            .event_bots
            .ids_of(EventBotKind::Juraid)
            .into_iter()
            .filter(|i| world.event_bots.get(*i).is_some_and(|e| e.nation == 1))
            .collect();
        assert_eq!(karus_bots.len(), 2, "3 per team minus the human");
        let view = RoomView {
            humans: vec![Human { sid: 1, nation: 1, x: 300.0, z: 600.0 }],
            human_total: 1,
            ..Default::default()
        };
        for id in &karus_bots {
            let bot = world.get_bot(*id).unwrap();
            join_party(&world, &bot, 1, &view);
        }
        let pid = world.get_party_id(1).expect("a lone player gets a party around him");
        let party = world.get_party(pid).unwrap();
        for id in &karus_bots {
            assert!(party.active_members().contains(&(*id as SessionId)));
        }
        assert_eq!(bots_in_party(&world), 2);
    }
}
