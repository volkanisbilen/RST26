//! Ultima's Dungeon - a registered raid on the Forgotten Temple map (zone 55).
//!
//! Flow (see docs/KNIGHT_ONLINE_ETKINLIKLERI_CLAUDE_RAPORU.md):
//! 1. A GM opens the registration (`+ultimaopen [minutes]`); players register at the Ultima Keeper NPC.
//! 2. When the registration ends (or `+ultimastart`) the registrants are validated, split into rooms
//!    (max 30) and teleported into the temple; each room is its own `event_room`.
//! 3. 2 minutes after entering, the seal and the boss appear. The boss cannot lose HP until the seal
//!    is destroyed; the seal only takes a fixed hit per basic attack.
//! 4. Once the seal is down the room has 30 minutes. At 70% / 40% / 10% boss HP four untargetable
//!    escape structures appear; after 5 seconds everybody outside a safe circle dies. If nobody dies,
//!    the boss is lethal for 10 seconds (warriors deal 1.5x).
//! 5. Between those attacks the boss hits a random player's surroundings for 30% of their HP.
//! 6. Rewards depend on how many destructive attacks a player survived.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use ko_protocol::Packet;
use parking_lot::Mutex;
use rand::Rng;
use tracing::{info, warn};

use crate::session::ClientSession;
use crate::systems::timed_notice::build_notice_packet;
use crate::world::types::ZONE_MORADON;
use crate::world::WorldState;
use crate::zone::SessionId;

pub const ZONE_ULTIMA: u16 = 55;
/// Client proto of the Ultima Keeper NPC (registration), see migrations/20261011000008.
pub const NPC_ULTIMA_KEEPER: u16 = 31997;
const NPC_BOSS_SMALL: u16 = 31990;
const NPC_BOSS_MEDIUM: u16 = 31991;
const NPC_BOSS_LARGE: u16 = 31992;
const NPC_SEAL: u16 = 31993;
const NPC_ESCAPE: u16 = 31994;

pub const EVENT_REGISTER: i32 = 1_300_001;
pub const EVENT_CANCEL: i32 = 1_300_002;

// Client text ids (Quest_Menu / Quest_Talk of the 2625 client)
const TEXT_HEADER: i32 = 45099;
const TEXT_ENTER: i32 = 45131;
const TEXT_CANCEL: i32 = 45134;

const MIN_LEVEL: u8 = 80;
const MIN_LADDER: u32 = 50;
const MIN_PLAYERS: usize = 4;
const MIN_NEW_ROOM: usize = 5;
const MAX_ROOM: usize = 30;
const DEFAULT_REG_MINUTES: u64 = 5;
const SPAWN_DELAY_SECS: u64 = 120;
const FIGHT_SECS: u64 = 1_800;
const WARNING_SECS: u64 = 5;
const LETHAL_SECS: u64 = 10;
const LEAVE_SECS: u64 = 15;
const AOE_INTERVAL_SECS: u64 = 30;
const AOE_RADIUS: f32 = 7.0;
const SAFE_RADIUS: f32 = 6.0;
const STRUCTURE_OFFSET: f32 = 14.0;
const SEAL_HIT: i32 = 500;
const ROOM_BASE: u16 = 200;
const THRESHOLDS: [i64; 3] = [70, 40, 10];

const ITEM_ASCENT_SCROLL: u32 = 800_051_000;
const ITEM_ARROGANCE: u32 = 389_160_000;
const ITEM_GLUTTONY: u32 = 389_161_000;
const ITEM_LUNAR_TATTOO: u32 = 810_432_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Registration,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoomState {
    Entering,
    Sealed,
    Fight,
    Finished,
    Closed,
}

struct Player {
    sid: SessionId,
    name: String,
    /// Destructive attacks survived when the player was eliminated.
    out_stage: Option<u8>,
}

struct Room {
    event_room: u16,
    state: RoomState,
    players: Vec<Player>,
    entered_at: u64,
    center: (f32, f32),
    boss: Option<u32>,
    boss_max_hp: i32,
    seal: Option<u32>,
    structures: Vec<(u32, f32, f32)>,
    attacks_done: u8,
    warning_until: u64,
    lethal_until: u64,
    next_aoe: u64,
    fight_deadline: u64,
    leave_at: u64,
}

struct State {
    phase: Phase,
    reg_deadline: u64,
    registrants: Vec<(SessionId, String)>,
    rooms: Vec<Room>,
    /// character -> day number of the last entry (one entry per day)
    entered: HashMap<String, u64>,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            phase: Phase::Idle,
            reg_deadline: 0,
            registrants: Vec::new(),
            rooms: Vec::new(),
            entered: HashMap::new(),
        })
    })
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn notice(world: &WorldState, sid: SessionId, msg: &str) {
    world.send_to_session_owned(sid, build_notice_packet(8, msg));
}

fn announce_all(world: &WorldState, msg: &str) {
    world.broadcast_to_all(Arc::new(build_notice_packet(8, msg)), None);
}

fn room_notice(world: &WorldState, room: &Room, msg: &str) {
    for p in &room.players {
        if p.out_stage.is_none() {
            notice(world, p.sid, msg);
        }
    }
}

// ── Registration (NPC) ───────────────────────────────────────────────────

/// Right-click on the Ultima Keeper NPC.
pub async fn npc_click(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let mut texts = [-1; 12];
    let mut events = [-1; 12];
    texts[0] = TEXT_ENTER;
    events[0] = EVENT_REGISTER;
    texts[1] = TEXT_CANCEL;
    events[1] = EVENT_CANCEL;
    super::select_msg::send_select_msg(&world, sid, 2, -1, TEXT_HEADER, &texts, &events, "ultima.lua");
    Ok(())
}

/// A button of the Keeper dialog was chosen.
pub async fn handle_event(session: &mut ClientSession, event: i32) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    match event {
        EVENT_REGISTER => register(&world, sid),
        EVENT_CANCEL => cancel(&world, sid),
        _ => {}
    }
    Ok(())
}

pub fn is_ultima_event(event: i32) -> bool {
    matches!(event, EVENT_REGISTER | EVENT_CANCEL)
}

fn register(world: &WorldState, sid: SessionId) {
    let Some(ch) = world.get_character_info(sid) else {
        return;
    };
    let mut st = state().lock();
    if st.phase != Phase::Registration {
        let msg = if st.phase == Phase::Running {
            "Players are fighting Ultima. Please try again later."
        } else {
            "Ultima's Dungeon registration is not open."
        };
        drop(st);
        notice(world, sid, msg);
        return;
    }
    if ch.level < MIN_LEVEL {
        drop(st);
        notice(world, sid, &format!("You need to be level {MIN_LEVEL} or higher."));
        return;
    }
    if ch.loyalty_monthly < MIN_LADDER {
        drop(st);
        notice(world, sid, &format!("You need at least {MIN_LADDER} monthly Ladder Points."));
        return;
    }
    if world.is_player_dead(sid) {
        return;
    }
    if world.get_character_info(sid).is_some_and(|c| c.party_id.is_some()) {
        drop(st);
        notice(world, sid, "You cannot register while you are in a party.");
        return;
    }
    let day = now_unix() / 86_400;
    if st.entered.get(&ch.name) == Some(&day) {
        drop(st);
        notice(world, sid, "Ultima's Dungeon can be entered once a day.");
        return;
    }
    if st.registrants.iter().any(|(s, _)| *s == sid) {
        drop(st);
        notice(world, sid, "You are already registered.");
        return;
    }
    st.registrants.push((sid, ch.name.clone()));
    let count = st.registrants.len();
    drop(st);
    notice(world, sid, &format!("You are registered for Ultima's Dungeon ({count} registered)."));
}

fn cancel(world: &WorldState, sid: SessionId) {
    let mut st = state().lock();
    let before = st.registrants.len();
    st.registrants.retain(|(s, _)| *s != sid);
    let removed = st.registrants.len() != before;
    drop(st);
    notice(
        world,
        sid,
        if removed { "Your Ultima's Dungeon registration was cancelled." } else { "You are not registered." },
    );
}

// ── GM control ───────────────────────────────────────────────────────────

/// `+ultimaopen [minutes]`
pub fn gm_open(world: &WorldState, minutes: Option<u64>) -> &'static str {
    let mut st = state().lock();
    if st.phase != Phase::Idle {
        return "Ultima's Dungeon is already open or running.";
    }
    let minutes = minutes.unwrap_or(DEFAULT_REG_MINUTES).clamp(1, 120);
    st.phase = Phase::Registration;
    st.reg_deadline = now_unix() + minutes * 60;
    st.registrants.clear();
    drop(st);
    announce_all(
        world,
        &format!("Ultima's Dungeon registration is open for {minutes} minutes. Register at the Ultima Keeper in Moradon."),
    );
    "Ultima's Dungeon registration opened."
}

/// `+ultimastart`: end the registration now and send the registrants in.
pub fn gm_start(world: &WorldState) -> &'static str {
    let mut st = state().lock();
    if st.phase != Phase::Registration {
        return "Ultima's Dungeon registration is not open.";
    }
    start_rooms(world, &mut st, now_unix());
    "Ultima's Dungeon is starting."
}

/// `+ultimaend`: cancel the registration or close every room.
pub fn gm_end(world: &WorldState) -> &'static str {
    let mut st = state().lock();
    if st.phase == Phase::Idle {
        return "Ultima's Dungeon is not active.";
    }
    let now = now_unix();
    for room in st.rooms.iter_mut() {
        if room.state != RoomState::Finished && room.state != RoomState::Closed {
            finish_room(world, room, false, now);
        }
        room.leave_at = now;
    }
    if st.rooms.is_empty() {
        st.phase = Phase::Idle;
        st.registrants.clear();
        drop(st);
        announce_all(world, "Ultima's Dungeon registration was cancelled.");
    }
    "Ultima's Dungeon is being closed."
}

// ── Room setup ───────────────────────────────────────────────────────────

fn start_rooms(world: &WorldState, st: &mut State, now: u64) {
    let day = now / 86_400;
    // re-validate: online, level, alive, ladder
    let mut valid: Vec<(SessionId, String)> = Vec::new();
    for (sid, name) in std::mem::take(&mut st.registrants) {
        let Some(ch) = world.get_character_info(sid) else { continue };
        if ch.name != name || ch.level < MIN_LEVEL || ch.loyalty_monthly < MIN_LADDER {
            continue;
        }
        if world.is_player_dead(sid) || world.get_event_room(sid) != 0 {
            continue;
        }
        valid.push((sid, name));
    }

    if valid.len() < MIN_PLAYERS {
        for (sid, _) in &valid {
            notice(world, *sid, "Ultima's Dungeon was cancelled: not enough players registered.");
        }
        announce_all(world, "Ultima's Dungeon was cancelled: not enough players registered.");
        st.phase = Phase::Idle;
        return;
    }

    // room sizes: at most 30; a new room needs MIN_NEW_ROOM players, otherwise they join the previous room
    let mut sizes: Vec<usize> = Vec::new();
    let mut left = valid.len();
    while left > 0 {
        let take = left.min(MAX_ROOM);
        if !sizes.is_empty() && take < MIN_NEW_ROOM {
            // too few for a new room: spread over the others if they have space, otherwise drop
            let mut extra = take;
            for s in sizes.iter_mut() {
                while extra > 0 && *s < MAX_ROOM {
                    *s += 1;
                    extra -= 1;
                }
            }
            break;
        }
        sizes.push(take);
        left -= take;
    }

    let center = arena_center(world);
    let mut it = valid.into_iter();
    for (idx, size) in sizes.into_iter().enumerate() {
        let event_room = ROOM_BASE + idx as u16;
        let mut players = Vec::new();
        for _ in 0..size {
            let Some((sid, name)) = it.next() else { break };
            st.entered.insert(name.clone(), day);
            players.push(Player { sid, name, out_stage: None });
        }
        for p in &players {
            super::zone_change::server_teleport_to_zone(world, p.sid, ZONE_ULTIMA, 0.0, 0.0);
            world.update_session(p.sid, |h| h.event_room = event_room);
            notice(
                world,
                p.sid,
                &format!("Welcome to Ultima's Dungeon. Ultima and its seal appear in {SPAWN_DELAY_SECS} seconds."),
            );
        }
        info!(event_room, players = players.len(), "Ultima's Dungeon: room opened");
        st.rooms.push(Room {
            event_room,
            state: RoomState::Entering,
            players,
            entered_at: now,
            center,
            boss: None,
            boss_max_hp: 0,
            seal: None,
            structures: Vec::new(),
            attacks_done: 0,
            warning_until: 0,
            lethal_until: 0,
            next_aoe: 0,
            fight_deadline: 0,
            leave_at: 0,
        });
    }
    st.phase = Phase::Running;
    announce_all(world, "Ultima's Dungeon has started.");
}

/// Middle of the Forgotten Temple arena (mean of the monster spawn points).
fn arena_center(world: &WorldState) -> (f32, f32) {
    let rows = world.ft_summons();
    if !rows.is_empty() {
        let (sx, sz) = rows.iter().fold((0f64, 0f64), |a, r| (a.0 + r.pos_x as f64, a.1 + r.pos_z as f64));
        let n = rows.len() as f64;
        return ((sx / n) as f32, (sz / n) as f32);
    }
    drop(rows);
    world
        .get_start_position(ZONE_ULTIMA)
        .map(|s| (s.karus_x as f32, s.karus_z as f32 + 30.0))
        .unwrap_or((512.0, 512.0))
}

// ── Per-second tick ──────────────────────────────────────────────────────

/// Called every second from the event system.
pub fn tick(world: &WorldState) {
    let now = now_unix();
    let mut st = state().lock();
    match st.phase {
        Phase::Idle => {}
        Phase::Registration => {
            if now >= st.reg_deadline {
                start_rooms(world, &mut st, now);
            }
        }
        Phase::Running => {
            for room in st.rooms.iter_mut() {
                tick_room(world, room, now);
            }
            st.rooms.retain(|r| r.state != RoomState::Closed);
            if st.rooms.is_empty() {
                st.phase = Phase::Idle;
                info!("Ultima's Dungeon: all rooms closed");
            }
        }
    }
}

fn player_alive_here(world: &WorldState, room: &Room, sid: SessionId) -> bool {
    !world.is_player_dead(sid)
        && world.get_event_room(sid) == room.event_room
        && world.get_position(sid).is_some_and(|p| p.zone_id == ZONE_ULTIMA)
}

fn mark_eliminated(world: &WorldState, room: &mut Room) {
    let done = room.attacks_done;
    let event_room = room.event_room;
    for p in room.players.iter_mut() {
        if p.out_stage.is_some() {
            continue;
        }
        let here = !world.is_player_dead(p.sid)
            && world.get_event_room(p.sid) == event_room
            && world.get_position(p.sid).is_some_and(|pos| pos.zone_id == ZONE_ULTIMA);
        if !here {
            p.out_stage = Some(done);
        }
    }
}

fn tick_room(world: &WorldState, room: &mut Room, now: u64) {
    match room.state {
        RoomState::Closed => {}
        RoomState::Finished => {
            if now >= room.leave_at {
                close_room(world, room);
            }
        }
        _ => {
            mark_eliminated(world, room);
            if room.players.iter().all(|p| p.out_stage.is_some()) {
                finish_room(world, room, false, now);
                return;
            }
            match room.state {
                RoomState::Entering => {
                    if now >= room.entered_at + SPAWN_DELAY_SECS {
                        spawn_boss_and_seal(world, room);
                    }
                }
                RoomState::Sealed => {
                    if room.seal.is_some_and(|n| world.is_npc_dead(n)) {
                        if let Some(boss) = room.boss {
                            world.ultima_set_boss_guard(boss, false);
                        }
                        room.state = RoomState::Fight;
                        room.fight_deadline = now + FIGHT_SECS;
                        room.next_aoe = now + AOE_INTERVAL_SECS;
                        room_notice(world, room, "The seal is broken! Defeat Ultima within 30 minutes.");
                    }
                }
                RoomState::Fight => tick_fight(world, room, now),
                _ => {}
            }
        }
    }
}

fn spawn_boss_and_seal(world: &WorldState, room: &mut Room) {
    let alive = room.players.iter().filter(|p| p.out_stage.is_none()).count();
    let boss_proto = match alive {
        0..=10 => NPC_BOSS_SMALL,
        11..=20 => NPC_BOSS_MEDIUM,
        _ => NPC_BOSS_LARGE,
    };
    let (cx, cz) = room.center;
    let boss = world.spawn_event_npc_ex(boss_proto, true, ZONE_ULTIMA, cx, cz, 1, room.event_room, 0);
    let seal = world.spawn_event_npc_ex(NPC_SEAL, true, ZONE_ULTIMA, cx + 10.0, cz, 1, room.event_room, 0);
    let (Some(&boss_nid), Some(&seal_nid)) = (boss.first(), seal.first()) else {
        warn!("Ultima's Dungeon: could not spawn the boss/seal (templates missing)");
        room_notice(world, room, "Ultima's Dungeon could not start. Please contact a GM.");
        finish_room(world, room, false, now_unix());
        return;
    };
    room.boss = Some(boss_nid);
    room.boss_max_hp = world.get_npc_hp(boss_nid).unwrap_or(1_000_000);
    room.seal = Some(seal_nid);
    world.ultima_set_boss_guard(boss_nid, true);
    world.ultima_register_seal(seal_nid, SEAL_HIT);
    room.state = RoomState::Sealed;
    room_notice(world, room, "Ultima has appeared, protected by a seal. Destroy the seal first!");
}

fn tick_fight(world: &WorldState, room: &mut Room, now: u64) {
    let Some(boss) = room.boss else { return };

    if world.is_npc_dead(boss) {
        finish_room(world, room, true, now);
        return;
    }
    if now >= room.fight_deadline {
        room_notice(world, room, "Time is up! Ultima could not be defeated.");
        finish_room(world, room, false, now);
        return;
    }

    // end of the lethal window
    if room.lethal_until != 0 && now >= room.lethal_until {
        room.lethal_until = 0;
        world.ultima_set_lethal(boss, false);
    }

    // destructive attack in progress
    if room.warning_until != 0 {
        if now >= room.warning_until {
            resolve_destructive_attack(world, room, now);
        }
        return;
    }

    // next destructive attack by boss HP
    let hp = world.get_npc_hp(boss).unwrap_or(0) as i64;
    let idx = room.attacks_done as usize;
    if idx < THRESHOLDS.len() && room.boss_max_hp > 0 && hp * 100 <= room.boss_max_hp as i64 * THRESHOLDS[idx] {
        begin_warning(world, room, now);
        return;
    }

    // targeted area attack
    if now >= room.next_aoe {
        room.next_aoe = now + AOE_INTERVAL_SECS;
        boss_area_attack(world, room);
    }
}

fn begin_warning(world: &WorldState, room: &mut Room, now: u64) {
    let (cx, cz) = room.center;
    let spots = [
        (cx + STRUCTURE_OFFSET, cz),
        (cx - STRUCTURE_OFFSET, cz),
        (cx, cz + STRUCTURE_OFFSET),
        (cx, cz - STRUCTURE_OFFSET),
    ];
    room.structures.clear();
    for (x, z) in spots {
        if let Some(&nid) = world.spawn_event_npc_ex(NPC_ESCAPE, false, ZONE_ULTIMA, x, z, 1, room.event_room, 0).first() {
            room.structures.push((nid, x, z));
        }
    }
    room.warning_until = now + WARNING_SECS;
    room_notice(
        world,
        room,
        "Ultima is gathering power for a devastating attack! Get inside a safe circle within 5 seconds!",
    );
}

fn resolve_destructive_attack(world: &WorldState, room: &mut Room, now: u64) {
    let structures = room.structures.clone();
    let mut died = 0u32;
    let done_before = room.attacks_done;
    for p in room.players.iter_mut() {
        if p.out_stage.is_some() {
            continue;
        }
        let Some(pos) = world.get_position(p.sid) else { continue };
        let safe = structures.iter().any(|&(_, x, z)| {
            let (dx, dz) = (pos.x - x, pos.z - z);
            (dx * dx + dz * dz).sqrt() <= SAFE_RADIUS
        });
        if !safe {
            kill_player(world, p.sid);
            p.out_stage = Some(done_before);
            died += 1;
        }
    }
    for &(nid, _, _) in &structures {
        world.kill_npc(nid);
    }
    room.structures.clear();
    room.warning_until = 0;
    room.attacks_done += 1;

    if died == 0 {
        room.lethal_until = now + LETHAL_SECS;
        if let Some(boss) = room.boss {
            world.ultima_set_lethal(boss, true);
        }
        room_notice(world, room, "Nobody fell! Ultima is exhausted for 10 seconds - warriors deal 1.5x damage!");
    } else {
        room_notice(world, room, &format!("The attack claimed {died} fighter(s). Survive the next one!"));
    }
    room.next_aoe = now + AOE_INTERVAL_SECS;
}

fn boss_area_attack(world: &WorldState, room: &Room) {
    let alive: Vec<SessionId> = room
        .players
        .iter()
        .filter(|p| p.out_stage.is_none() && player_alive_here(world, room, p.sid))
        .map(|p| p.sid)
        .collect();
    if alive.is_empty() {
        return;
    }
    let target = alive[rand::thread_rng().gen_range(0..alive.len())];
    let Some(tpos) = world.get_position(target) else { return };
    for sid in alive {
        let Some(pos) = world.get_position(sid) else { continue };
        let (dx, dz) = (pos.x - tpos.x, pos.z - tpos.z);
        if (dx * dx + dz * dz).sqrt() > AOE_RADIUS {
            continue;
        }
        let Some(ch) = world.get_character_info(sid) else { continue };
        let loss = ((ch.max_hp as i32) * 30 / 100).max(1) as i16;
        let new_hp = (ch.hp - loss).max(1);
        world.update_character_hp(sid, new_hp);
        world.send_to_session_owned(sid, crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp));
    }
}

fn kill_player(world: &WorldState, sid: SessionId) {
    let max_hp = world.get_character_info(sid).map(|c| c.max_hp).unwrap_or(0);
    world.update_character_hp(sid, 0);
    world.send_to_session_owned(sid, crate::systems::regen::build_hp_change_packet(max_hp, 0));
    super::dead::broadcast_death(world, sid);
}

// ── Finish / rewards ─────────────────────────────────────────────────────

fn finish_room(world: &WorldState, room: &mut Room, victory: bool, now: u64) {
    if matches!(room.state, RoomState::Finished | RoomState::Closed) {
        return;
    }
    mark_eliminated(world, room);
    if let Some(boss) = room.boss {
        world.ultima_set_boss_guard(boss, false);
        world.ultima_set_lethal(boss, false);
    }
    let damage: HashMap<SessionId, i32> = room
        .boss
        .map(|b| world.get_npc_damage_entries(b).into_iter().collect())
        .unwrap_or_default();
    let total: i64 = damage.values().map(|&d| d as i64).sum::<i64>().max(1);
    let done = room.attacks_done;

    for p in &room.players {
        let survivor = victory && p.out_stage.is_none();
        let stage = p.out_stage.unwrap_or(done);
        let mut rewards: Vec<(u32, &str)> = Vec::new();
        if survivor {
            rewards.push((ITEM_GLUTTONY, "Fragment of Gluttony"));
            rewards.push((ITEM_ARROGANCE, "Fragment of Arrogance"));
        } else {
            match stage {
                0 => {}
                1 => rewards.push((ITEM_ASCENT_SCROLL, "Ascent Scroll")),
                2 => rewards.push((ITEM_ARROGANCE, "Fragment of Arrogance")),
                _ => rewards.push((ITEM_GLUTTONY, "Fragment of Gluttony")),
            }
        }
        for (item, name) in rewards {
            if world.give_item(p.sid, item, 1) {
                notice(world, p.sid, &format!("Ultima's Dungeon reward: {name}."));
            } else {
                notice(world, p.sid, &format!("Your bag is full: {name} could not be given."));
            }
        }
        if survivor {
            // boss box: the more damage, the better the chance of a 30 day Lunar Tattoo
            let share = damage.get(&p.sid).copied().unwrap_or(0) as i64 * 100 / total;
            let chance = (share * 3).clamp(1, 60);
            if rand::thread_rng().gen_range(0..100) < chance {
                if world.give_item_with_expiry(p.sid, ITEM_LUNAR_TATTOO, 1, 30) {
                    notice(world, p.sid, "The boss box held a Lunar Tattoo (30 days)!");
                }
            }
        }
    }

    for &(nid, _, _) in &room.structures {
        world.kill_npc(nid);
    }
    room.structures.clear();
    room_notice_all(
        world,
        room,
        if victory {
            "Ultima has been defeated! You will leave in 15 seconds."
        } else {
            "Ultima's Dungeon is over. You will leave in 15 seconds."
        },
    );
    room.state = RoomState::Finished;
    room.leave_at = now + LEAVE_SECS;
}

fn room_notice_all(world: &WorldState, room: &Room, msg: &str) {
    for p in &room.players {
        notice(world, p.sid, msg);
    }
}

fn close_room(world: &WorldState, room: &mut Room) {
    for nid in [room.boss, room.seal].into_iter().flatten() {
        world.ultima_set_boss_guard(nid, false);
        world.ultima_set_lethal(nid, false);
        world.kill_npc(nid);
    }
    for p in &room.players {
        if world.get_event_room(p.sid) == room.event_room {
            world.update_session(p.sid, |h| h.event_room = 0);
            if world.get_position(p.sid).is_some_and(|pos| pos.zone_id == ZONE_ULTIMA) {
                super::zone_change::server_teleport_to_zone(world, p.sid, ZONE_MORADON, 0.0, 0.0);
            }
        }
    }
    room.state = RoomState::Closed;
    info!(event_room = room.event_room, "Ultima's Dungeon: room closed");
}

#[allow(dead_code)]
fn _packet_type_check(_: Packet) {}
