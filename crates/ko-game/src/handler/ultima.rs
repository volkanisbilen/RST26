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
use std::sync::atomic::{AtomicI16, Ordering};
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
/// Boss tiers by room size. These are the client's own "[Boss] Ultima" monster ids (mob_us.tbl).
const NPC_BOSS_SMALL: u16 = 10100;
const NPC_BOSS_MEDIUM: u16 = 10110;
const NPC_BOSS_LARGE: u16 = 10111;
/// The seal that has to be destroyed first: the client's "[Artifact]" NPC (type 61, attackable like a CSW artifact).
const NPC_SEAL: u16 = 10101;
/// "[Protection] Ultima Seal Artifact": the four untargetable escape structures.
const NPC_ESCAPE: u16 = 10103;

/// Temple-event id the client's registration window is opened with (`+ultimaid <n>` changes it live).
static EVENT_ID: AtomicI16 = AtomicI16::new(47);

pub fn event_id() -> i16 {
    EVENT_ID.load(Ordering::Relaxed)
}

pub fn set_event_id(id: i16) {
    EVENT_ID.store(id, Ordering::Relaxed);
}

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

#[allow(dead_code)]
const ITEM_ASCENT_SCROLL: u32 = 800_051_000;
#[allow(dead_code)]
const ITEM_ARROGANCE: u32 = 389_160_000;
#[allow(dead_code)]
const ITEM_GLUTTONY: u32 = 389_161_000;
#[allow(dead_code)]
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
    /// GM-spawned test bots fighting in this room
    bots: Vec<u32>,
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
    /// bots added during the registration (+eventbotfill); they enter with the first room
    pending_bots: usize,
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
            pending_bots: 0,
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

// ── Registration (the client's temple-event registration window) ─────────

const SUB_JOIN: u8 = 8;
const SUB_DISBAND: u8 = 9;
const SUB_COUNTER: u8 = 16;

fn counter_packet(count: u16) -> Packet {
    let mut pkt = Packet::new(ko_protocol::Opcode::WizEvent as u8);
    pkt.write_u8(SUB_COUNTER);
    pkt.write_u16(event_id() as u16);
    pkt.write_u16(count);
    pkt
}

fn reply(op: u8, result: u8) -> Packet {
    let mut pkt = Packet::new(ko_protocol::Opcode::WizEvent as u8);
    pkt.write_u8(op);
    pkt.write_u8(result);
    pkt.write_i16(event_id());
    pkt
}

/// "idle" | "registration" | "running" (for the event list of the web panel).
pub fn phase_name() -> &'static str {
    match state().lock().phase {
        Phase::Idle => "idle",
        Phase::Registration => "registration",
        Phase::Running => "running",
    }
}

/// Is the Ultima registration window currently open?
pub fn registration_open() -> bool {
    state().lock().phase == Phase::Registration
}

fn registered_count(st: &State) -> u16 {
    (st.registrants.len() + st.pending_bots).min(u16::MAX as usize) as u16
}

/// Open the registration window for everybody online.
fn broadcast_window(world: &WorldState, st: &State, now: u64) {
    let remaining = st.reg_deadline.saturating_sub(now).min(u16::MAX as u64) as u16;
    let title = Arc::new(super::survival::build_window_title("Ultima Dungeon"));
    let open = Arc::new(crate::systems::event_room::build_event_start_broadcast(event_id(), remaining));
    let count = Arc::new(counter_packet(registered_count(st)));
    for sid in world.get_in_game_session_ids() {
        world.send_to_session_arc(sid, Arc::clone(&open));
        world.send_to_session_arc(sid, Arc::clone(&count));
        world.send_to_session_arc(sid, Arc::clone(&title));
    }
}

fn broadcast_count(world: &WorldState, st: &State) {
    let count = Arc::new(counter_packet(registered_count(st)));
    for sid in world.get_in_game_session_ids() {
        world.send_to_session_arc(sid, Arc::clone(&count));
    }
}

fn is_gm(world: &WorldState, sid: SessionId) -> bool {
    world
        .get_character_info(sid)
        .is_some_and(|c| c.authority == 0 || c.authority == 2)
}

/// WIZ_EVENT join (Apply pressed) while Ultima's registration is open.
pub async fn temple_join(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let result = match try_register(&world, sid) {
        Ok(()) => 0u8, // the client's "registered" state (Apply turns into Cancel)
        Err(msg) => {
            notice(&world, sid, msg);
            4u8
        }
    };
    session.send_packet(&reply(SUB_JOIN, result)).await?;
    broadcast_count(&world, &state().lock());
    Ok(())
}

/// WIZ_EVENT disband (Cancel pressed) while Ultima's registration is open.
pub async fn temple_disband(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    state().lock().registrants.retain(|(s, _)| *s != sid);
    session.send_packet(&reply(SUB_DISBAND, 1)).await?;
    broadcast_count(&world, &state().lock());
    Ok(())
}

fn try_register(world: &WorldState, sid: SessionId) -> Result<(), &'static str> {
    let Some(ch) = world.get_character_info(sid) else {
        return Err("Registration failed.");
    };
    let gm = ch.authority == 0 || ch.authority == 2;
    let mut st = state().lock();
    if st.phase != Phase::Registration {
        return Err("Ultima's Dungeon registration is not open.");
    }
    if st.registrants.iter().any(|(s, _)| *s == sid) {
        return Err("You are already registered.");
    }
    if !gm {
        if ch.level < MIN_LEVEL {
            return Err("You need to be level 80 or higher.");
        }
        if ch.loyalty_monthly < MIN_LADDER {
            return Err("You need at least 50 monthly Ladder Points.");
        }
        if world.is_player_dead(sid) {
            return Err("You cannot register while dead.");
        }
        if ch.party_id.is_some() {
            return Err("You cannot register while you are in a party.");
        }
        let day = now_unix() / 86_400;
        if st.entered.get(&ch.name) == Some(&day) {
            return Err("Ultima's Dungeon can be entered once a day.");
        }
    }
    st.registrants.push((sid, ch.name.clone()));
    Ok(())
}

// ── GM control ───────────────────────────────────────────────────────────

/// `+ultimaopen [minutes]`
pub fn gm_open(world: &WorldState, minutes: Option<u64>) -> &'static str {
    let mut st = state().lock();
    if st.phase != Phase::Idle {
        return "Ultima's Dungeon is already open or running.";
    }
    if forgotten_temple_running(world) {
        return "Ultima's Dungeon shares its map with the Forgotten Temple, which is running.";
    }
    let minutes = minutes.unwrap_or(DEFAULT_REG_MINUTES).clamp(1, 120);
    st.phase = Phase::Registration;
    st.reg_deadline = now_unix() + minutes * 60;
    st.registrants.clear();
    st.pending_bots = 0;
    broadcast_window(world, &st, now_unix());
    drop(st);
    announce_all(
        world,
        &format!("Ultima's Dungeon registration is open for {minutes} minutes. Use the registration window to apply."),
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

/// Ultima's Dungeon and the Forgotten Temple are both played on zone 55: the temple's kick-out,
/// kill counter and rewards work on the whole zone, so the two events must never overlap.
fn forgotten_temple_running(world: &WorldState) -> bool {
    world
        .forgotten_temple_state()
        .is_active
        .load(Ordering::Relaxed)
}

/// Is Ultima's Dungeon registering or running? (The Forgotten Temple refuses to open meanwhile.)
pub fn is_busy() -> bool {
    state().lock().phase != Phase::Idle
}

fn start_rooms(world: &WorldState, st: &mut State, now: u64) {
    let day = now / 86_400;
    if forgotten_temple_running(world) {
        for (sid, _) in &st.registrants {
            notice(
                world,
                *sid,
                "Ultima's Dungeon was cancelled: the Forgotten Temple is using the same map.",
            );
        }
        announce_all(world, "Ultima's Dungeon was cancelled: the Forgotten Temple is using the same map.");
        st.registrants.clear();
        st.pending_bots = 0;
        st.phase = Phase::Idle;
        return;
    }
    // re-validate: online, level, alive, ladder
    let mut valid: Vec<(SessionId, String)> = Vec::new();
    let mut any_gm = false;
    for (sid, name) in std::mem::take(&mut st.registrants) {
        let Some(ch) = world.get_character_info(sid) else { continue };
        let gm = ch.authority == 0 || ch.authority == 2;
        if ch.name != name {
            continue;
        }
        if !gm && (ch.level < MIN_LEVEL || ch.loyalty_monthly < MIN_LADDER) {
            continue;
        }
        if world.is_player_dead(sid) || world.get_event_room(sid) != 0 {
            continue;
        }
        any_gm |= gm;
        valid.push((sid, name));
    }

    // a GM in the list lets the event start with any number of players (testing)
    let mut bots_waiting = std::mem::take(&mut st.pending_bots);
    // admin-configured helper bots (default OFF; never without at least one human)
    let configured_bots = if valid.is_empty() {
        0
    } else {
        crate::systems::event_bots::auxiliary_bot_count(
            world,
            crate::systems::event_bots::EventBotKind::Ultima,
            valid.len().min(MAX_ROOM),
        )
    };
    bots_waiting += configured_bots;
    if valid.is_empty() || (valid.len() + bots_waiting < MIN_PLAYERS && !any_gm) {
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
            if !is_gm(world, sid) {
                st.entered.insert(name.clone(), day);
            }
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
            bots: Vec::new(),
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
    // registered bots join the first room
    if bots_waiting > 0 {
        if let Some(room) = st.rooms.first_mut() {
            let room_size = room.players.len();
            let take = bots_waiting.min(MAX_ROOM.saturating_sub(room_size));
            let (x, z) = world
                .get_start_position(ZONE_ULTIMA)
                .map(|p| (p.karus_x as f32, p.karus_z as f32))
                .unwrap_or(room.center);
            let nation = room
                .players
                .first()
                .and_then(|p| world.get_character_info(p.sid))
                .map(|c| c.nation)
                .unwrap_or(1);
            let mut rng = rand::thread_rng();
            for i in 0..take {
                let id = crate::systems::bot_ai::spawn_gm_bot(
                    world,
                    crate::systems::bot_ai::SpawnGmBotParams {
                        zone_id: ZONE_ULTIMA,
                        x: x + rng.gen_range(-4.0..4.0),
                        y: 0.0,
                        z: z + rng.gen_range(-4.0..4.0),
                        class: (i % 4) as u16 + 1,
                        level: if configured_bots > 0 {
                            crate::systems::event_bots::auxiliary_bot_level(
                                world,
                                crate::systems::event_bots::EventBotKind::Ultima,
                            )
                        } else {
                            83
                        },
                        nation,
                        ai_state: crate::world::BotAiState::Afk,
                    },
                );
                world.event_bots.register_aux_bot(id, room.event_room);
                room.bots.push(id);
            }
        }
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
        let in_zone = world.get_event_room(p.sid) == event_room
            && world.get_position(p.sid).is_some_and(|pos| pos.zone_id == ZONE_ULTIMA);
        let here = !world.is_player_dead(p.sid) && in_zone;
        if !here {
            p.out_stage = Some(done);
            if in_zone {
                // died in the dungeon: bring them back to life and out (the generic Forgotten Temple kick of a
                // dead player left the character stuck "dead" in the zone)
                world.update_session(p.sid, |h| h.event_room = 0);
                revive_and_leave(world, p.sid);
            }
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
            drive_bots(world, room);
            if now % 3 == 0 {
                bot_party_tick(world, room);
            }
        }
    }
}

fn spawn_boss_and_seal(world: &WorldState, room: &mut Room) {
    let alive = room.players.iter().filter(|p| p.out_stage.is_none()).count() + room.bots.len();
    let boss_proto = match alive {
        0..=10 => NPC_BOSS_SMALL,
        11..=20 => NPC_BOSS_MEDIUM,
        _ => NPC_BOSS_LARGE,
    };
    let (cx, cz) = room.center;
    let boss = world.spawn_event_npc_ex(boss_proto, true, ZONE_ULTIMA, cx, cz, 1, room.event_room, 0);
    let seal = world.spawn_event_npc_ex(NPC_SEAL, false, ZONE_ULTIMA, cx + 10.0, cz, 1, room.event_room, 0);
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
        if let Some(&nid) = world.spawn_event_npc_ex(NPC_ESCAPE, true, ZONE_ULTIMA, x, z, 1, room.event_room, 0).first() {
            world.ultima_set_boss_guard(nid, true); // cannot be damaged
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
        world.ultima_set_boss_guard(nid, false);
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

    // the rewards come from the web panel (event_reward_rule, event "ultima"); old built-in ones when none
    let rules = world.reward_rules_for("ultima");
    for p in &room.players {
        let survivor = victory && p.out_stage.is_none();
        let stage = p.out_stage.unwrap_or(done);
        let cond = if survivor {
            "survive"
        } else {
            match stage {
                0 => "",
                1 => "stage1",
                2 => "stage2",
                _ => "stage3",
            }
        };
        for r in rules.iter().filter(|r| !cond.is_empty() && r.cond == cond) {
            if r.chance < 100 && rand::thread_rng().gen_range(0..100) >= r.chance {
                continue;
            }
            let name = world.get_item(r.item_id).and_then(|i| i.str_name.clone()).unwrap_or_else(|| format!("#{}", r.item_id));
            // a full bag mails the reward (and says so) instead of dropping it
            match world.give_rule_item_or_mail(p.sid, r, "Ultima") {
                crate::reward_rules::Delivery::Bag => {
                    notice(world, p.sid, &format!("Ultima's Dungeon reward: {name}."));
                }
                crate::reward_rules::Delivery::Mail => {}
                crate::reward_rules::Delivery::Failed => {
                    notice(world, p.sid, &format!("{name} could not be given. Please contact a GM."));
                }
            }
        }
        if survivor {
            // boss box: the more damage, the better the chance (damage share x 3 %, at most the rule's chance)
            let share = damage.get(&p.sid).copied().unwrap_or(0) as i64 * 100 / total;
            for r in rules.iter().filter(|r| r.cond == "box") {
                let chance = (share * 3).clamp(1, r.chance.clamp(1, 100) as i64);
                if rand::thread_rng().gen_range(0..100) < chance {
                    let name = world.get_item(r.item_id).and_then(|i| i.str_name.clone()).unwrap_or_else(|| format!("#{}", r.item_id));
                    match world.give_rule_item_or_mail(p.sid, r, "Ultima") {
                        crate::reward_rules::Delivery::Bag => {
                            notice(world, p.sid, &format!("The boss box held {name}!"));
                        }
                        crate::reward_rules::Delivery::Mail => {}
                        crate::reward_rules::Delivery::Failed => {
                            notice(world, p.sid, &format!("{name} could not be given. Please contact a GM."));
                        }
                    }
                }
            }
        }
    }

    for &(nid, _, _) in &room.structures {
        world.ultima_set_boss_guard(nid, false);
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
    for id in room.bots.drain(..) {
        crate::systems::bot_ai::despawn_bot(world, id);
    }
    for nid in [room.boss, room.seal].into_iter().flatten() {
        world.ultima_set_boss_guard(nid, false);
        world.ultima_set_lethal(nid, false);
        world.kill_npc(nid);
    }
    for p in &room.players {
        if world.get_event_room(p.sid) == room.event_room {
            world.update_session(p.sid, |h| h.event_room = 0);
            if world.get_position(p.sid).is_some_and(|pos| pos.zone_id == ZONE_ULTIMA) {
                revive_and_leave(world, p.sid);
            }
        }
    }
    room.state = RoomState::Closed;
    info!(event_room = room.event_room, "Ultima's Dungeon: room closed");
}

/// Fighters that died in the dungeon are revived (full HP, standing) and answered with a
/// respawn packet before the zone change, otherwise they arrive in Moradon still "dead" with 0 HP.
fn revive_and_leave(world: &WorldState, sid: SessionId) {
    if world.is_player_dead(sid) {
        world.update_res_hp_type(sid, crate::world::USER_STANDING);
        world.update_session(sid, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 0;
        });
        if let Some(ch) = world.get_character_info(sid) {
            world.update_character_hp(sid, ch.max_hp);
            world.send_to_session_owned(sid, crate::systems::regen::build_hp_change_packet(ch.max_hp, ch.max_hp));
            let (x, z) = world
                .get_start_position(ZONE_MORADON)
                .map(|s| if ch.nation == 1 { (s.karus_x as f32, s.karus_z as f32) } else { (s.elmorad_x as f32, s.elmorad_z as f32) })
                .filter(|&(x, z)| x != 0.0 || z != 0.0)
                .unwrap_or((814.0, 437.0));
            let mut resp = Packet::new(ko_protocol::Opcode::WizRegene as u8);
            resp.write_u16((x * 10.0) as u16);
            resp.write_u16((z * 10.0) as u16);
            resp.write_u16(0);
            resp.write_u8(1);
            world.send_to_session_owned(sid, resp);
        }
    }
    super::zone_change::server_teleport_to_zone(world, sid, ZONE_MORADON, 0.0, 0.0);
}

// ── Test bots (GM) ───────────────────────────────────────────────────────

/// Damage one bot deals to the boss per second.
const BOT_DPS: i32 = 2_500;

/// `+ultimabot [count] [class 1-4]` - spawn bots into the GM's own Ultima room. They attack the
/// seal and then the boss.
pub fn gm_spawn_bots(world: &WorldState, gm_sid: SessionId, count: usize, class: u16) -> String {
    let event_room = world.get_event_room(gm_sid);
    let mut st = state().lock();
    let Some(room) = st.rooms.iter_mut().find(|r| r.event_room == event_room && r.state != RoomState::Closed) else {
        return "You must be inside an Ultima's Dungeon room.".to_string();
    };
    let Some(gm_pos) = world.get_position(gm_sid) else {
        return "No position.".to_string();
    };
    let nation = world.get_character_info(gm_sid).map(|c| c.nation).unwrap_or(1);
    let count = count.clamp(1, 30);
    let mut rng = rand::thread_rng();
    let mut spawned = 0;
    for _ in 0..count {
        let id = crate::systems::bot_ai::spawn_gm_bot(
            world,
            crate::systems::bot_ai::SpawnGmBotParams {
                zone_id: ZONE_ULTIMA,
                x: gm_pos.x + rng.gen_range(-4.0..4.0),
                y: gm_pos.y,
                z: gm_pos.z + rng.gen_range(-4.0..4.0),
                class,
                level: 83,
                nation,
                ai_state: crate::world::BotAiState::Afk,
            },
        );
        world.event_bots.register_aux_bot(id, room.event_room);
        room.bots.push(id);
        spawned += 1;
    }
    format!("{spawned} bot(s) joined the Ultima room ({} in total).", room.bots.len())
}

/// Bots walk up to the seal / boss and hit it once per second; at a destructive attack they stand
/// in a safe circle (and are never killed).
fn drive_bots(world: &WorldState, room: &mut Room) {
    if room.bots.is_empty() {
        return;
    }
    let target = match room.state {
        RoomState::Sealed => room.seal,
        RoomState::Fight => room.boss,
        _ => None,
    };
    let target = target.filter(|&n| !world.is_npc_dead(n));
    let Some(inst) = target.and_then(|n| world.get_npc_instance(n)) else { return };
    let nid = inst.nid;
    let (tx, tz) = (inst.x, inst.z);

    let warning = room.warning_until != 0 && !room.structures.is_empty();
    let mut rng = rand::thread_rng();
    for (i, &id) in room.bots.clone().iter().enumerate() {
        let Some(bot) = world.get_bot(id) else { continue };
        if !bot.is_alive() {
            continue;
        }
        // stand next to the target, or inside a safe circle while the boss gathers power
        let (px, pz) = if warning {
            let (_, sx, sz) = room.structures[i % room.structures.len()];
            (sx + 1.0, sz + 1.0)
        } else {
            (tx + rng.gen_range(-3.0..3.0), tz + rng.gen_range(-3.0..3.0))
        };
        let (dx, dz) = (bot.x - px, bot.z - pz);
        if dx * dx + dz * dz > 25.0 {
            crate::systems::bot_ai::event_bot_place(world, id, px, pz);
        }
        if warning {
            continue;
        }
        crate::systems::bot_ai::event_bot_swing(world, id, nid);
        let hp = world.get_npc_hp(nid).unwrap_or(0);
        if world.ultima_seal_hit(nid).is_some() {
            world.update_npc_hp(nid, hp - 1); // the seal takes its fixed hit
        } else {
            world.update_npc_hp(nid, (hp - BOT_DPS).max(0));
        }
        if world.get_npc_hp(nid).unwrap_or(0) <= 0 {
            world.kill_npc(nid);
            break;
        }
    }
}

/// `+eventbotfill <count>` while Ultima's registration is open: register `count` bots.
pub fn gm_registration_bots(world: &WorldState, count: usize) -> Option<String> {
    let mut st = state().lock();
    if st.phase != Phase::Registration {
        return None;
    }
    st.pending_bots = (st.pending_bots + count.clamp(1, 30)).min(30);
    let now = now_unix();
    broadcast_count(world, &st);
    let _ = now;
    Some(format!("{} bot(s) registered for Ultima's Dungeon.", st.pending_bots))
}

// ── Bot parties (Ultima rooms) ───────────────────────────────────────────

use ko_protocol::Opcode;

const BOT_PARTY_MAX_BOTS: usize = 4;

fn send_bot_party_chat(world: &WorldState, party_id: u16, bot_id: u32, text: &str) {
    let Some(bot) = world.get_bot(bot_id) else { return };
    let pkt = super::chat::build_chat_packet(
        super::chat::ChatType::Party as u8,
        bot.nation,
        bot.id as u16,
        &bot.name,
        text,
        -1,
        1,
        0,
    );
    world.send_to_party(party_id, &pkt);
}

/// A player in an Ultima room asked for a party (typed "party" or opened Seek): a bot leader of the
/// player's nation invites them. Called from the chat handler and from the room tick.
pub fn party_request(world: &WorldState, sid: SessionId) {
    let room = world.get_event_room(sid);
    if room < ROOM_BASE || world.has_party_invitation(sid) || world.get_party_id(sid).is_some() {
        return;
    }
    let Some(nation) = world.get_character_info(sid).map(|c| c.nation) else { return };
    let st = state().lock();
    let Some(r) = st.rooms.iter().find(|r| r.event_room == room) else { return };
    // a free bot of the player's nation
    let Some(bot_id) = r.bots.iter().copied().find(|&id| {
        world.get_bot(id).is_some_and(|b| b.is_alive() && b.nation == nation)
            && world.get_party_id(id as SessionId).is_none()
    }) else {
        notice(world, sid, "There is no free bot of your nation to party with.");
        return;
    };
    let Some(bot) = world.get_bot(bot_id) else { return };
    let Some(party_id) = world.create_party(bot_id as SessionId) else { return };
    world.set_party_invitation(sid, party_id, bot_id as SessionId);
    let mut permit = Packet::new(Opcode::WizParty as u8);
    permit.write_u8(0x02); // PARTY_PERMIT
    permit.write_u32(bot.id);
    permit.write_string(&bot.name);
    world.send_to_session_owned(sid, permit);
}

/// Grow bot-led parties that contain a real player with more bots of the same nation.
fn fill_bot_parties(world: &WorldState, room: &Room) {
    for &leader in &room.bots {
        let Some(party_id) = world.get_party_id(leader as SessionId) else { continue };
        let Some(party) = world.get_party(party_id) else { continue };
        if !party.is_leader(leader as SessionId) || party.is_full() {
            continue;
        }
        let members = party.active_members();
        let has_player = members.iter().any(|m| (*m as u32) < crate::world::BOT_ID_BASE);
        let bot_count = members.iter().filter(|m| (**m as u32) >= crate::world::BOT_ID_BASE).count();
        if !has_player || bot_count >= BOT_PARTY_MAX_BOTS {
            continue;
        }
        let Some(lead) = world.get_bot(leader) else { continue };
        let ally = room.bots.iter().copied().find(|&id| {
            id != leader
                && world.get_party_id(id as SessionId).is_none()
                && world.get_bot(id).is_some_and(|b| b.is_alive() && b.nation == lead.nation)
        });
        if let Some(ally_id) = ally {
            if let Some(ally) = world.get_bot(ally_id) {
                if world.add_party_member(party_id, ally_id as SessionId) {
                    let info = super::party::build_bot_party_member_info(&ally, 1, party.target_number_id);
                    world.send_to_party(party_id, &info);
                    send_bot_party_chat(world, party_id, ally_id, "Joined the party.");
                }
            }
        }
    }
}

/// Room tick: answer players who opened the Seek-party icon and fill bot parties.
fn bot_party_tick(world: &WorldState, room: &Room) {
    if room.bots.is_empty() {
        return;
    }
    for p in &room.players {
        if p.out_stage.is_some() {
            continue;
        }
        let seeking = world.with_session(p.sid, |h| h.need_party == 1).unwrap_or(false);
        if seeking {
            // party_request locks the global state, which the caller already holds: do it lazily
            party_request_locked(world, room, p.sid);
        }
    }
    fill_bot_parties(world, room);
}

/// Same as `party_request` but for callers that already hold the state lock.
fn party_request_locked(world: &WorldState, room: &Room, sid: SessionId) {
    if world.has_party_invitation(sid) || world.get_party_id(sid).is_some() {
        return;
    }
    let Some(nation) = world.get_character_info(sid).map(|c| c.nation) else { return };
    let Some(bot_id) = room.bots.iter().copied().find(|&id| {
        world.get_bot(id).is_some_and(|b| b.is_alive() && b.nation == nation)
            && world.get_party_id(id as SessionId).is_none()
    }) else {
        return;
    };
    let Some(bot) = world.get_bot(bot_id) else { return };
    let Some(party_id) = world.create_party(bot_id as SessionId) else { return };
    world.set_party_invitation(sid, party_id, bot_id as SessionId);
    let mut permit = Packet::new(Opcode::WizParty as u8);
    permit.write_u8(0x02);
    permit.write_u32(bot.id);
    permit.write_string(&bot.name);
    world.send_to_session_owned(sid, permit);
}
