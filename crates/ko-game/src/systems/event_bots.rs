//! Event-specific bot system.
//!
//! Lets the server owner fill the room-based events with runtime bots (`BotInstance`s from
//! `bot_ai.rs`) so that an event can run with few human participants. Everything is OFF by default
//! and configured per event from the admin panel (`event_bot_config` table, "Etkinlik Botları").
//!
//! See `docs/EVENT_BOTS.md` for the per-event design. In short:
//!
//! | Event | Support | What the bots do |
//! |-------|---------|------------------|
//! | Chaos Dungeon | full | join the FFA room, hunt everybody (humans and bots), kills/deaths tracked by name |
//! | Border Defence War | partial | fill both nations, fight the enemy nation; altar flag stays a human objective |
//! | Ultima's Dungeon | full | extra raiders (existing `ultima.rs` bot driver), count towards the minimum player count |
//! | Full Moon Rift | partial | support bots that soften monsters but never land the killing blow |
//! | Juraid Mountain | partial | follow a party's player or walk the nation's lane clearing every stage (`event_bots_juraid.rs`); never take chests |
//! | Forgotten Temple / Manes | not supported | see the design doc |
//!
//! ## Rules every bot follows
//! * Bots never get rewards. Reward loops are session based and skip bot ids; Chaos ranks are
//!   computed among humans only (`is_event_bot_sid`).
//! * Bots never open a room on their own: nothing is spawned unless at least `min_humans` real
//!   players are in that room.
//! * Bots are removed when the event ends (cleanup hooks) and a periodic sweep removes anything
//!   whose event/room no longer exists, so a missed hook cannot leak bots.
//! * Event bots are only visible to (and only attackable by) players of their own event room.
//! * Killing an event bot gives the killer no NP/gold (`handle_bot_death` replaces the PK-zone
//!   reward path of `bot_on_death`).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ko_protocol::{Opcode, Packet};
use parking_lot::{Mutex, RwLock};
use rand::Rng;
use tracing::{debug, info, warn};

use crate::systems::bot_ai::{self, SpawnGmBotParams};
use crate::systems::event_room::{EventUser, TempleEventType};
use crate::world::types::USER_DEAD;
use crate::world::{BotAiState, BotId, BotInstance, BotPresence, WorldState, BOT_ID_BASE};
use crate::zone::{calc_region, SessionId};

mod bdw_ai;

// ── Kinds ────────────────────────────────────────────────────────────────

/// An event the bot system knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventBotKind {
    Bdw,
    Chaos,
    Juraid,
    Ultima,
    Rift,
    ForgottenTemple,
    Manes,
}

/// How completely the bot system supports an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    Full,
    Partial,
    Unsupported,
}

impl Support {
    pub fn as_str(self) -> &'static str {
        match self {
            Support::Full => "full",
            Support::Partial => "partial",
            Support::Unsupported => "unsupported",
        }
    }
}

pub const ALL_KINDS: [EventBotKind; 7] = [
    EventBotKind::Chaos,
    EventBotKind::Bdw,
    EventBotKind::Ultima,
    EventBotKind::Rift,
    EventBotKind::Juraid,
    EventBotKind::ForgottenTemple,
    EventBotKind::Manes,
];

impl EventBotKind {
    /// Key used in `event_bot_config.event_key` and by the admin panel.
    pub fn key(self) -> &'static str {
        match self {
            EventBotKind::Bdw => "bdw",
            EventBotKind::Chaos => "chaos",
            EventBotKind::Juraid => "juraid",
            EventBotKind::Ultima => "ultima",
            EventBotKind::Rift => "rift",
            EventBotKind::ForgottenTemple => "ft",
            EventBotKind::Manes => "manes",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        ALL_KINDS.iter().copied().find(|k| k.key() == key)
    }

    pub fn support(self) -> Support {
        match self {
            EventBotKind::Chaos | EventBotKind::Ultima | EventBotKind::Bdw => Support::Full,
            EventBotKind::Rift | EventBotKind::Juraid => Support::Partial,
            EventBotKind::ForgottenTemple | EventBotKind::Manes => Support::Unsupported,
        }
    }

    /// Maximum participants of one "team": per nation in a BDW room, the whole Chaos room,
    /// one Ultima room, one Rift group.
    pub fn team_capacity(self) -> usize {
        match self {
            EventBotKind::Bdw | EventBotKind::Juraid => crate::systems::event_room::MAX_ROOM_USERS_PER_NATION,
            EventBotKind::Chaos => crate::systems::event_room::MAX_CHAOS_ROOM_USERS,
            EventBotKind::Ultima => 30,
            EventBotKind::Rift => 8,
            EventBotKind::ForgottenTemple | EventBotKind::Manes => 0,
        }
    }

    /// The temple event the kind runs on (rooms managed by `EventRoomManager`).
    pub fn temple_type(self) -> Option<TempleEventType> {
        match self {
            EventBotKind::Bdw => Some(TempleEventType::BorderDefenceWar),
            EventBotKind::Chaos => Some(TempleEventType::ChaosDungeon),
            EventBotKind::Juraid => Some(TempleEventType::JuraidMountain),
            _ => None,
        }
    }

    /// Chaos is free-for-all, every other kind is nation versus nation.
    pub fn is_ffa(self) -> bool {
        self == EventBotKind::Chaos
    }

    pub fn from_temple(event_type: TempleEventType) -> Option<Self> {
        match event_type {
            TempleEventType::BorderDefenceWar => Some(EventBotKind::Bdw),
            TempleEventType::ChaosDungeon => Some(EventBotKind::Chaos),
            TempleEventType::JuraidMountain => Some(EventBotKind::Juraid),
            _ => None,
        }
    }
}

// ── Configuration ────────────────────────────────────────────────────────

/// Per-event admin configuration (`event_bot_config` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBotConfig {
    /// Master switch. Default OFF.
    pub enabled: bool,
    /// Bots per team (per room for Chaos, per Ultima room / Rift group).
    pub bots_per_team: u8,
    pub min_level: u8,
    pub max_level: u8,
    /// `true`: top each team up to `bots_per_team` participants (humans + bots).
    /// `false`: always add `bots_per_team` bots per team (capped by the room capacity).
    pub fill_only_when_short: bool,
    /// Bots only appear when at least this many humans take part (never below 1).
    pub min_humans: u8,
}

impl Default for EventBotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bots_per_team: 4,
            min_level: 60,
            max_level: 83,
            fill_only_when_short: true,
            min_humans: 1,
        }
    }
}

impl EventBotConfig {
    /// Clamp every field into the range the event can actually use.
    pub fn sanitized(mut self, kind: EventBotKind) -> Self {
        let cap = kind.team_capacity().min(u8::MAX as usize) as u8;
        self.bots_per_team = self.bots_per_team.min(cap);
        self.min_level = self.min_level.clamp(1, 83);
        self.max_level = self.max_level.clamp(1, 83);
        if self.min_level > self.max_level {
            std::mem::swap(&mut self.min_level, &mut self.max_level);
        }
        self.min_humans = self.min_humans.max(1);
        if kind.support() == Support::Unsupported {
            self.enabled = false;
        }
        self
    }

    /// A random level inside the configured range.
    pub fn pick_level(&self, rng: &mut impl Rng) -> u8 {
        let (lo, hi) = (self.min_level.min(self.max_level), self.min_level.max(self.max_level));
        rng.gen_range(lo..=hi)
    }
}

/// How many bots one team needs. Pure function, see the tests for the contract.
pub fn bots_needed(cfg: &EventBotConfig, humans_in_team: usize, capacity: usize) -> usize {
    if !cfg.enabled {
        return 0;
    }
    let want = if cfg.fill_only_when_short {
        (cfg.bots_per_team as usize).saturating_sub(humans_in_team)
    } else {
        cfg.bots_per_team as usize
    };
    want.min(capacity.saturating_sub(humans_in_team))
}

/// Bots to add to a room, per team: `[karus, elmorad]` for nation events, `[room, 0]` for Chaos.
pub fn plan_room_fill(kind: EventBotKind, cfg: &EventBotConfig, humans: [usize; 2]) -> [usize; 2] {
    if !cfg.enabled || kind.support() == Support::Unsupported {
        return [0, 0];
    }
    let total = humans[0] + humans[1];
    if total < cfg.min_humans.max(1) as usize {
        return [0, 0];
    }
    if kind.is_ffa() {
        return [bots_needed(cfg, total, kind.team_capacity()), 0];
    }
    if kind == EventBotKind::Juraid {
        return crate::systems::event_bots_juraid::plan_fill(cfg, humans);
    }
    plan_nation_fill(cfg, humans, kind.team_capacity())
}

/// Bots per nation for a nation-versus-nation event (BDW, Juraid): every team is topped up by
/// the usual rule ([`bots_needed`]) and the smaller team is then **mirrored** up with extra bots
/// until both teams have the same total size (humans + bots), capped by `capacity` per nation.
/// So a 5-man party meets 5 opponents even when `bots_per_team` is 4, and a lone player gets an
/// even match instead of a fixed crowd.
pub fn plan_nation_fill(cfg: &EventBotConfig, humans: [usize; 2], capacity: usize) -> [usize; 2] {
    let mut plan = [
        bots_needed(cfg, humans[0], capacity),
        bots_needed(cfg, humans[1], capacity),
    ];
    let target = (humans[0] + plan[0]).max(humans[1] + plan[1]);
    for side in 0..2 {
        let want = target.saturating_sub(humans[side]);
        plan[side] = plan[side]
            .max(want)
            .min(capacity.saturating_sub(humans[side]));
    }
    plan
}

// ── Registry ─────────────────────────────────────────────────────────────

/// Bot movement per driver tick: `speed 45 * 250 ms / 10_000` (bot_ai `movement_step`).
pub(crate) const STEP_PER_TICK: f32 = 1.125;
const DRIVER_TICK_MS: u64 = 250;
const RESPAWN_MS: u64 = 8_000;
const SWEEP_MS: u64 = 3_000;
const MELEE_RANGE: f32 = 5.5;
const RANGED_RANGE: f32 = 16.0;
const MELEE_COOLDOWN_MS: u64 = 2_000;
const RANGED_COOLDOWN_MS: u64 = 3_000;
pub(crate) const MISS_PERCENT: u32 = 12;
/// Backstop: the bot-AI expiry removes a bot that somehow escaped every cleanup.
const BOT_BACKSTOP_MINUTES: u32 = 150;
const MOVE_SPEED_FIELD: i16 = 45;

/// Runtime bookkeeping of one registered event bot.
#[derive(Debug, Clone)]
pub struct BotEntry {
    pub kind: EventBotKind,
    /// `event_room` id (what `ClientSession::event_room` holds for the humans of that room).
    pub room: u16,
    pub nation: u8,
    pub name: String,
    pub(crate) next_attack_ms: u64,
    /// 0 = alive.
    respawn_at_ms: u64,
}

impl BotEntry {
    pub fn new(kind: EventBotKind, room: u16, nation: u8, name: String) -> Self {
        Self {
            kind,
            room,
            nation,
            name,
            next_attack_ms: 0,
            respawn_at_ms: 0,
        }
    }
}

/// Per-world event-bot state: configuration + the registry of live event bots.
#[derive(Default)]
pub struct EventBotRegistry {
    cfg: RwLock<HashMap<EventBotKind, EventBotConfig>>,
    bots: Mutex<HashMap<BotId, BotEntry>>,
    /// Rift rooms whose helpers may only soften monsters (never kill).
    rift_support_rooms: Mutex<HashSet<u16>>,
    /// Helper bots owned by `ultima.rs` / `dungeon_defence.rs` (they drive their own bots):
    /// bot id -> event room, so the bots are only broadcast to / listed for that room.
    aux_rooms: Mutex<HashMap<BotId, u16>>,
    last_sweep_ms: AtomicU64,
    /// Juraid Mountain lane/stage bookkeeping (`event_bots_juraid.rs`).
    pub(crate) juraid: crate::systems::event_bots_juraid::JuraidRt,
}

impl EventBotRegistry {
    pub fn config(&self, kind: EventBotKind) -> EventBotConfig {
        self.cfg.read().get(&kind).cloned().unwrap_or_default()
    }

    pub fn set_config(&self, kind: EventBotKind, cfg: EventBotConfig) {
        self.cfg.write().insert(kind, cfg.sanitized(kind));
    }

    pub fn replace_configs(&self, map: HashMap<EventBotKind, EventBotConfig>) {
        *self.cfg.write() = map;
    }

    pub fn insert(&self, id: BotId, entry: BotEntry) {
        self.bots.lock().insert(id, entry);
    }

    pub fn remove(&self, id: BotId) -> Option<BotEntry> {
        self.bots.lock().remove(&id)
    }

    pub fn get(&self, id: BotId) -> Option<BotEntry> {
        self.bots.lock().get(&id).cloned()
    }

    pub(crate) fn update(&self, id: BotId, f: impl FnOnce(&mut BotEntry)) {
        if let Some(e) = self.bots.lock().get_mut(&id) {
            f(e);
        }
    }

    pub fn is_event_bot(&self, id: BotId) -> bool {
        self.bots.lock().contains_key(&id)
    }

    /// `event_room` the bot lives in (`0` for ordinary bots).
    pub fn event_room_of(&self, id: BotId) -> u16 {
        if let Some(e) = self.bots.lock().get(&id) {
            return e.room;
        }
        self.aux_rooms.lock().get(&id).copied().unwrap_or(0)
    }

    /// Region/INOUT visibility: ordinary bots are visible to everybody, event bots only to the
    /// viewers of their own event room.
    pub fn visible_in_room(&self, bot_id: BotId, viewer_room: u16) -> bool {
        if let Some(e) = self.bots.lock().get(&bot_id) {
            return e.room == viewer_room;
        }
        match self.aux_rooms.lock().get(&bot_id) {
            Some(room) => *room == viewer_room,
            None => true,
        }
    }

    /// Register a helper bot that `ultima.rs` / `dungeon_defence.rs` drives itself.
    pub fn register_aux_bot(&self, bot_id: BotId, room: u16) {
        self.aux_rooms.lock().insert(bot_id, room);
    }

    pub fn unregister_aux_bot(&self, bot_id: BotId) {
        self.aux_rooms.lock().remove(&bot_id);
    }

    pub fn aux_count(&self) -> usize {
        self.aux_rooms.lock().len()
    }

    pub fn ids(&self) -> Vec<BotId> {
        self.bots.lock().keys().copied().collect()
    }

    pub fn ids_of(&self, kind: EventBotKind) -> Vec<BotId> {
        self.bots
            .lock()
            .iter()
            .filter(|(_, e)| e.kind == kind)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn count(&self, kind: EventBotKind) -> usize {
        self.bots.lock().values().filter(|e| e.kind == kind).count()
    }

    pub fn total(&self) -> usize {
        self.bots.lock().len()
    }

    pub fn mark_rift_support_room(&self, room_id: u16) {
        self.rift_support_rooms.lock().insert(room_id);
    }

    pub fn rift_room_is_support(&self, room_id: u16) -> bool {
        self.rift_support_rooms.lock().contains(&room_id)
    }

    pub fn clear_rift_room(&self, room_id: u16) {
        self.rift_support_rooms.lock().remove(&room_id);
    }
}

/// `true` when a session-band id (< 65536) actually names an event bot. Used by the reward loops
/// to keep bots out of rankings and payouts.
pub fn is_event_bot_sid(world: &WorldState, sid: SessionId) -> bool {
    (sid as u32) >= BOT_ID_BASE && world.event_bots.is_event_bot(sid as u32)
}

// ── Config reload (DB) ───────────────────────────────────────────────────

/// Reload every `event_bot_config` row into the world registry. Reads everything before touching
/// the live map, so a database error keeps the previous configuration.
pub async fn reload_config(pool: &ko_db::DbPool, world: &WorldState) -> anyhow::Result<usize> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT event_key, enabled, bots_per_team, min_level, max_level, fill_only_when_short, min_humans \
         FROM event_bot_config",
    )
    .fetch_all(pool)
    .await?;
    let mut map = HashMap::new();
    for row in &rows {
        let key: String = row.try_get("event_key")?;
        let Some(kind) = EventBotKind::from_key(&key) else {
            continue;
        };
        let small = |col: &str| -> anyhow::Result<u8> {
            Ok(row.try_get::<i16, _>(col)?.clamp(0, 255) as u8)
        };
        let cfg = EventBotConfig {
            enabled: row.try_get("enabled")?,
            bots_per_team: small("bots_per_team")?,
            min_level: small("min_level")?,
            max_level: small("max_level")?,
            fill_only_when_short: row.try_get("fill_only_when_short")?,
            min_humans: small("min_humans")?,
        }
        .sanitized(kind);
        map.insert(kind, cfg);
    }
    let count = map.len();
    world.event_bots.replace_configs(map);
    info!(count, "event bot configuration reloaded");
    Ok(count)
}

// ── Spawning ─────────────────────────────────────────────────────────────

/// Where a bot of `nation` appears in `zone_id`: the zone's start position (same rule humans use
/// when an event teleports them in), snapped to a walkable cell when the map is loaded.
fn spawn_point(world: &WorldState, zone_id: u16, nation: u8, rng: &mut impl Rng) -> Option<(f32, f32)> {
    // Juraid: each nation starts at its own end of the mountain (fixed by the event design).
    if let Some((jx, jz)) = crate::systems::event_bots_juraid::spawn_override(zone_id, nation) {
        let (x, z) = (jx + rng.gen_range(-3..=3) as f32, jz + rng.gen_range(-3..=3) as f32);
        if let Some(map) = world.get_zone(zone_id).as_ref().and_then(|z| z.map_data.as_ref()) {
            return crate::systems::pathfind::nearest_bot_point(map, x, z, 8);
        }
        return Some((x, z));
    }
    let mut base: Option<(f32, f32)> = None;
    let (mut range_x, mut range_z) = (0i16, 0i16);
    if let Some(sp) = world.get_start_position(zone_id) {
        let (x, z) = if nation == 1 {
            (sp.karus_x as f32, sp.karus_z as f32)
        } else {
            (sp.elmorad_x as f32, sp.elmorad_z as f32)
        };
        if x != 0.0 || z != 0.0 {
            base = Some((x, z));
            range_x = sp.range_x.max(0);
            range_z = sp.range_z.max(0);
        }
    }
    let zone = world.get_zone(zone_id);
    if base.is_none() {
        if let Some(z) = zone.as_ref() {
            let (x, zz, _) = z.spawn_position();
            if x != 0.0 || zz != 0.0 {
                base = Some((x, zz));
            }
        }
    }
    let (bx, bz) = base?;
    let x = bx + if range_x > 0 { rng.gen_range(0..=range_x) as f32 } else { 0.0 };
    let z = bz + if range_z > 0 { rng.gen_range(0..=range_z) as f32 } else { 0.0 };
    if let Some(map) = zone.as_ref().and_then(|z| z.map_data.as_ref()) {
        return crate::systems::pathfind::nearest_bot_point(map, x, z, 8);
    }
    Some((x, z))
}

/// Spawn one event bot and register it. Returns its id, or `None` (and leaves nothing behind)
/// when the bot cannot be placed.
fn spawn_event_bot(
    world: &WorldState,
    kind: EventBotKind,
    room: u16,
    zone_id: u16,
    nation: u8,
    class_slot: usize,
    level: u8,
    taken_names: &HashSet<String>,
) -> Option<(BotId, String)> {
    let mut rng = rand::thread_rng();
    let (x, z) = spawn_point(world, zone_id, nation, &mut rng)?;
    let id = bot_ai::spawn_gm_bot(
        world,
        SpawnGmBotParams {
            zone_id,
            x,
            y: 0.0,
            z,
            class: (class_slot % 4) as u16 + 1,
            level,
            nation,
            ai_state: BotAiState::Afk,
        },
    );
    // Party / room lists carry bots as 16-bit session-band ids.
    if id > SessionId::MAX as u32 {
        warn!(id, "event bot id left the session id range; refusing to use it");
        world.remove_bot(id);
        return None;
    }
    let mut name = world.get_bot(id)?.name;
    if taken_names.contains(&name) {
        let mut n = 2u32;
        while taken_names.contains(&format!("{name}{n}")) {
            n += 1;
        }
        name = format!("{name}{n}");
    }
    world.update_bot(id, |b| {
        b.name = name.clone();
        b.duration_minutes = BOT_BACKSTOP_MINUTES;
    });
    world
        .event_bots
        .insert(id, BotEntry::new(kind, room, nation, name.clone()));
    Some((id, name))
}

/// Add the configured bots to every room of a freshly started temple event. Called right
/// before the humans are teleported in. Returns the number of bots added.
pub fn fill_temple_rooms(world: &WorldState, event_type: TempleEventType) -> usize {
    let Some(kind) = EventBotKind::from_temple(event_type) else {
        return 0;
    };
    let cfg = world.event_bots.config(kind);
    if !cfg.enabled {
        return 0;
    }
    let erm = &world.event_room_manager;
    let zone_id = event_type.zone_id();
    let mut rooms = erm.list_rooms(event_type);
    rooms.sort_unstable();
    let mut added = 0usize;
    let mut rng = rand::thread_rng();

    for room_id in rooms {
        // snapshot who is in the room
        let (humans, mut taken) = {
            let Some(room) = erm.get_room(event_type, room_id) else {
                continue;
            };
            let alive = |u: &&EventUser| !u.logged_out;
            let humans = [
                room.karus_users.values().filter(alive).count(),
                room.elmorad_users.values().filter(alive).count(),
                room.mixed_users.values().filter(alive).count(),
            ];
            let taken: HashSet<String> = room
                .karus_users
                .keys()
                .chain(room.elmorad_users.keys())
                .chain(room.mixed_users.keys())
                .cloned()
                .collect();
            (humans, taken)
        };
        let plan = if kind.is_ffa() {
            plan_room_fill(kind, &cfg, [humans[2], 0])
        } else {
            plan_room_fill(kind, &cfg, [humans[0], humans[1]])
        };
        if kind == EventBotKind::Juraid {
            world.event_bots.juraid.reset_room(room_id as u16);
        }
        let class_offset = rng.gen_range(0..4usize);
        for (team_idx, count) in plan.iter().copied().enumerate() {
            for i in 0..count {
                // Chaos is FFA: alternate the nations so both look populated.
                let nation: u8 = if kind.is_ffa() { (i % 2) as u8 + 1 } else { team_idx as u8 + 1 };
                let level = cfg.pick_level(&mut rng);
                let Some((id, name)) = spawn_event_bot(
                    world,
                    kind,
                    room_id as u16,
                    zone_id,
                    nation,
                    class_offset + i,
                    level,
                    &taken,
                ) else {
                    continue;
                };
                let user = EventUser {
                    user_name: name.clone(),
                    session_id: id as SessionId,
                    nation,
                    prize_given: true, // bots are never rewarded
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                };
                let ok = erm
                    .get_room_mut(event_type, room_id)
                    .map(|mut room| room.add_user(user))
                    .unwrap_or(false);
                if ok {
                    taken.insert(name);
                    added += 1;
                } else {
                    // room full / gone: take the bot back
                    despawn_event_bot(world, id);
                }
            }
        }
    }
    if added > 0 {
        info!(kind = kind.key(), added, "event bots added to the rooms");
    }
    added
}

// ── Cleanup ──────────────────────────────────────────────────────────────

/// BDW: fill a real player's party with the free bots of his nation in the room.
pub fn fill_party_with_room_bots(world: &WorldState, party_id: u16) -> usize {
    bdw_ai::fill_party(world, party_id)
}

/// Remove one event bot completely: registry entry, room membership, runtime bot and the
/// "disappear" packet for its room.
pub fn despawn_event_bot(world: &WorldState, id: BotId) {
    let Some(entry) = world.event_bots.remove(id) else {
        return;
    };
    if let Some(et) = entry.kind.temple_type() {
        if let Some(mut room) = world.event_room_manager.get_room_mut(et, entry.room as u8) {
            room.remove_user(&entry.name);
        }
    }
    if let Ok(sid) = SessionId::try_from(id) {
        world.cleanup_party_on_disconnect(sid);
    }
    if let Some(bot) = world.remove_bot(id) {
        let mut out = Packet::new(Opcode::WizUserInout as u8);
        out.write_u8(2); // INOUT_OUT
        out.write_u8(0);
        out.write_u32(bot.id);
        room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, out);
    }
}

/// Remove every bot of `kind` (event over). Returns how many were removed.
pub fn cleanup_kind(world: &WorldState, kind: EventBotKind) -> usize {
    let ids = world.event_bots.ids_of(kind);
    let n = ids.len();
    for id in ids {
        despawn_event_bot(world, id);
    }
    n
}

/// Remove the bots of one room (the room's finish countdown ended).
pub fn cleanup_room(world: &WorldState, kind: EventBotKind, room: u16) -> usize {
    let ids: Vec<BotId> = world
        .event_bots
        .ids()
        .into_iter()
        .filter(|id| {
            world
                .event_bots
                .get(*id)
                .is_some_and(|e| e.kind == kind && e.room == room)
        })
        .collect();
    let n = ids.len();
    for id in ids {
        despawn_event_bot(world, id);
    }
    n
}

/// Remove every registered event bot of every kind (admin "clear" button).
pub fn cleanup_all(world: &WorldState) -> usize {
    let ids = world.event_bots.ids();
    let n = ids.len();
    for id in ids {
        despawn_event_bot(world, id);
    }
    n
}

/// Leak guard: drop bots whose event is no longer running or whose room is gone.
pub fn sweep(world: &WorldState) -> usize {
    let erm = &world.event_room_manager;
    // helper bots of Ultima / Rift whose bot is gone (room closed) must not stay mapped
    {
        let gone: Vec<BotId> = world
            .event_bots
            .aux_rooms
            .lock()
            .keys()
            .copied()
            .filter(|id| world.get_bot(*id).is_none())
            .collect();
        for id in gone {
            world.event_bots.unregister_aux_bot(id);
        }
    }
    let active = erm.read_temple_event(|s| s.active_event);
    let mut stale = Vec::new();
    for id in world.event_bots.ids() {
        let Some(entry) = world.event_bots.get(id) else {
            continue;
        };
        let Some(et) = entry.kind.temple_type() else {
            // not a temple-room bot; nothing else registers here
            stale.push(id);
            continue;
        };
        let room_exists = erm.get_room(et, entry.room as u8).is_some();
        let bot_exists = world.get_bot(id).is_some();
        if active != et as i16 || !room_exists || !bot_exists {
            stale.push(id);
        }
    }
    let n = stale.len();
    for id in stale {
        despawn_event_bot(world, id);
    }
    {
        let rooms: Vec<u16> = world
            .event_bots
            .ids_of(EventBotKind::Juraid)
            .into_iter()
            .filter_map(|i| world.event_bots.get(i).map(|e| e.room))
            .collect();
        world.event_bots.juraid.housekeeping(&rooms);
    }
    if n > 0 {
        debug!(removed = n, "event bot sweep removed stale bots");
    }
    n
}

// ── Packets ──────────────────────────────────────────────────────────────

pub(crate) fn room_broadcast(world: &WorldState, zone_id: u16, rx: u16, rz: u16, room: u16, pkt: Packet) {
    world.broadcast_to_region_sync(zone_id, rx, rz, Arc::new(pkt), None, room);
}

pub(crate) fn move_packet(bot: &BotInstance, x: f32, y: f32, z: f32, echo: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMove as u8);
    pkt.write_u32(bot.id);
    pkt.write_u16((x * 10.0) as u16);
    pkt.write_u16((z * 10.0) as u16);
    pkt.write_u16((y * 10.0) as u16);
    pkt.write_i16(MOVE_SPEED_FIELD);
    pkt.write_u8(echo);
    pkt
}

fn magic_packet(opcode: u8, skill_id: u32, caster: u32, target: u32, data: [i32; 7]) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(opcode);
    pkt.write_u32(skill_id);
    pkt.write_u32(caster);
    pkt.write_u32(target);
    for d in data {
        pkt.write_u32(d as u32);
    }
    pkt
}

// ── Gates used by the human attack paths ────────────────────────────────

/// `true` when `attacker` must not be allowed to hurt event bot `bot_id` (wrong room, event not
/// in its fighting phase, or a same-nation bot in a nation-versus-nation event).
pub fn deny_player_attack(world: &WorldState, attacker: SessionId, bot_id: BotId) -> bool {
    let Some(entry) = world.event_bots.get(bot_id) else {
        return false;
    };
    let reason = if world.get_event_room(attacker) != entry.room {
        Some("other room")
    } else if !fighting_phase(world) {
        Some("combat not open")
    } else if !entry.kind.is_ffa()
        && world.get_character_info(attacker).map(|c| c.nation).unwrap_or(0) == entry.nation
    {
        Some("same nation")
    } else {
        None
    };
    if let Some(reason) = reason {
        // Diagnosable in the release log (warn level): the owner's live test "enemy bots cannot be
        // attacked" leaves no trace otherwise. At most one line per attacker per 5 s.
        let now = bot_ai::tick_ms();
        let mut log = DENY_LOG.get_or_init(|| Mutex::new(HashMap::new())).lock();
        if log.len() > 512 {
            log.clear();
        }
        let due = log.get(&attacker).is_none_or(|t| now.saturating_sub(*t) >= 5_000);
        if due {
            log.insert(attacker, now);
            let (atk_room, atk_nation) = (
                world.get_event_room(attacker),
                world.get_character_info(attacker).map(|c| c.nation).unwrap_or(0),
            );
            warn!(
                attacker,
                bot_id,
                reason,
                kind = entry.kind.key(),
                bot_room = entry.room,
                bot_nation = entry.nation,
                atk_room,
                atk_nation,
                phase = ?crate::systems::event_system::current_phase(&world.event_room_manager),
                attackable = world.event_room_manager.read_temple_event(|s| s.is_attackable),
                "event bot attack denied"
            );
        }
        return true;
    }
    false
}

static DENY_LOG: std::sync::OnceLock<Mutex<HashMap<SessionId, u64>>> = std::sync::OnceLock::new();

/// The temple event is past sign-up and combat is currently open.
fn fighting_phase(world: &WorldState) -> bool {
    use crate::systems::event_system::{current_phase, EventPhase};
    let erm = &world.event_room_manager;
    current_phase(erm) == EventPhase::Active && erm.read_temple_event(|s| s.is_attackable)
}

// ── Death handling ───────────────────────────────────────────────────────

/// Replaces the PK-zone death path of `bot_ai::bot_on_death` for event bots. Returns `true`
/// when `bot_id` is an event bot (the caller must then return immediately).
pub fn handle_bot_death(world: &WorldState, bot_id: BotId, now_ms: u64) -> bool {
    let Some(entry) = world.event_bots.get(bot_id) else {
        return false;
    };
    let Some(bot) = world.get_bot(bot_id) else {
        world.event_bots.remove(bot_id);
        return true;
    };
    if bot.presence == BotPresence::Dead && entry.respawn_at_ms > 0 {
        return true; // already processed
    }
    let killer_id = bot.last_attacker_id;
    world.update_bot(bot_id, |b| {
        b.hp = 0;
        b.presence = BotPresence::Dead;
        b.ai_state = BotAiState::Afk;
        b.target_id = -1;
        b.target_changed = false;
        b.regene_at_ms = 0; // the bot AI must not "regene" it into a PK spawn
        b.last_attacker_id = -1;
    });
    crate::systems::bot_buffs::clear(world, bot_id);
    world.clear_npc_dots(bot_id);
    if entry.kind == EventBotKind::Bdw {
        bdw_ai::on_bot_death(world, &entry, bot_id);
    }
    world.event_bots.update(bot_id, |e| e.respawn_at_ms = now_ms.saturating_add(RESPAWN_MS));
    crate::systems::bot_brain::forget(bot_id);
    let mut dead = Packet::new(Opcode::WizDead as u8);
    dead.write_u32(bot_id);
    room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, dead);

    if killer_id >= 0 {
        credit_kill(world, &entry, &bot, killer_id as u32);
    }
    true
}

/// Event bookkeeping + kill feed for "`killer_id` killed event bot `victim`".
fn credit_kill(world: &WorldState, victim_entry: &BotEntry, victim: &BotInstance, killer_id: u32) {
    let (killer_name, killer_nation) = if killer_id >= BOT_ID_BASE {
        match world.event_bots.get(killer_id) {
            Some(e) => (e.name, e.nation),
            None => return,
        }
    } else {
        match world.get_character_info(killer_id as SessionId) {
            Some(c) => (c.name, c.nation),
            None => return,
        }
    };
    record_event_kill(world, victim_entry.kind, &killer_name, killer_nation, &victim.name);
    world.send_death_notice_to_zone(
        victim.zone_id,
        killer_id as SessionId,
        victim.id as SessionId,
        &killer_name,
        &victim.name,
        None,
        victim.x.max(0.0) as u16,
        victim.z.max(0.0) as u16,
    );
}

/// Score a kill in the right event (name based, works for bots and humans).
fn record_event_kill(world: &WorldState, kind: EventBotKind, killer: &str, killer_nation: u8, victim: &str) {
    match kind {
        EventBotKind::Bdw => crate::handler::dead::track_bdw_kill_by_name(world, killer, killer_nation),
        EventBotKind::Chaos => crate::handler::dead::track_chaos_kill_by_name(world, killer, victim),
        _ => {}
    }
}

// ── Driver ───────────────────────────────────────────────────────────────

/// Start the 250 ms driver task. Idle (one registry lock) while no event bot exists.
pub fn start_driver_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(DRIVER_TICK_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if world.event_bots.total() == 0 && world.event_bots.aux_count() == 0 {
                continue;
            }
            let now_ms = bot_ai::tick_ms();
            let w = Arc::clone(&world);
            // A bug in one tick must not stop the driver for the rest of the server's life.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tick(&w, now_ms)));
            if result.is_err() {
                warn!("event bot driver tick panicked; continuing");
            }
        }
    })
}

/// A unit the bot fights: id (session band for players, bot id for bots) and position.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Target {
    pub id: u32,
    pub x: f32,
    pub z: f32,
}

/// One driver pass over all registered event bots.
pub fn tick(world: &WorldState, now_ms: u64) {
    let reg = &world.event_bots;
    crate::systems::bot_buffs::expire(world, now_ms);
    let last = reg.last_sweep_ms.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) >= SWEEP_MS {
        reg.last_sweep_ms.store(now_ms, Ordering::Relaxed);
        sweep(world);
    }
    let fighting = fighting_phase(world);
    for id in reg.ids() {
        tick_bot(world, id, now_ms, fighting);
    }
}

/// One bot: respawn bookkeeping, then the shared bot brain with the event objective
/// (`bot_brain::tick_event`: threat, targeting, kite / retreat / heal, formation,
/// consumables; BDW altar / Chaos hunt / Juraid lane as the objective layer).
fn tick_bot(world: &WorldState, id: BotId, now_ms: u64, fighting: bool) {
    let reg = &world.event_bots;
    let Some(entry) = reg.get(id) else { return };
    let Some(bot) = world.get_bot(id) else {
        reg.remove(id);
        return;
    };

    // dead: wait for the respawn timer
    if bot.presence == BotPresence::Dead || bot.hp <= 0 {
        if entry.respawn_at_ms == 0 {
            // died through a path that did not use handle_bot_death
            handle_bot_death(world, id, now_ms);
        } else if now_ms >= entry.respawn_at_ms && event_still_running(world, &entry) {
            respawn_bot(world, id, &entry);
        }
        return;
    }
    if !fighting || !room_open(world, &entry) {
        return;
    }
    // Juraid bots join the party of a real player of their nation (assist, never follow).
    if entry.kind == EventBotKind::Juraid {
        crate::systems::event_bots_juraid::party_tick(world, &entry, &bot, now_ms);
    }
    crate::systems::bot_brain::tick_event(world, &entry, &bot, now_ms);
}

pub(crate) fn attack_range(bot: &BotInstance) -> f32 {
    if bot.is_mage() || bot.is_priest() {
        RANGED_RANGE
    } else {
        MELEE_RANGE
    }
}

pub(crate) fn attack_cooldown(bot: &BotInstance) -> u64 {
    if bot.is_mage() || bot.is_priest() {
        RANGED_COOLDOWN_MS
    } else {
        MELEE_COOLDOWN_MS
    }
}

fn event_still_running(world: &WorldState, entry: &BotEntry) -> bool {
    use crate::systems::event_system::{current_phase, EventPhase};
    let erm = &world.event_room_manager;
    matches!(current_phase(erm), EventPhase::Active) && room_open(world, entry)
}

/// The room exists and has not produced a winner yet.
fn room_open(world: &WorldState, entry: &BotEntry) -> bool {
    let Some(et) = entry.kind.temple_type() else {
        return false;
    };
    world
        .event_room_manager
        .get_room(et, entry.room as u8)
        .is_some_and(|r| !r.finish_packet_sent && !r.finished)
}

pub(crate) fn hostile(kind: EventBotKind, a: u8, b: u8) -> bool {
    kind.is_ffa() || a != b
}

/// Is `id` still a valid enemy of `entry`'s bot? Returns its current position when it is.
pub(crate) fn resolve_target(world: &WorldState, entry: &BotEntry, bot: &BotInstance, id: u32) -> Option<Target> {
    if id >= BOT_ID_BASE {
        let other_entry = world.event_bots.get(id)?;
        if other_entry.room != entry.room || other_entry.kind != entry.kind {
            return None;
        }
        let other = world.get_bot(id)?;
        if !other.is_alive() || !hostile(entry.kind, entry.nation, other.nation) {
            return None;
        }
        return Some(Target { id, x: other.x, z: other.z });
    }
    let sid = id as SessionId;
    let ch = world.get_character_info(sid)?;
    let pos = world.get_position(sid)?;
    if !human_attackable(world, entry, sid, &ch, pos.zone_id, bot.zone_id) {
        return None;
    }
    Some(Target { id, x: pos.x, z: pos.z })
}

pub(crate) fn human_attackable(
    world: &WorldState,
    entry: &BotEntry,
    sid: SessionId,
    ch: &crate::world::CharacterInfo,
    zone: u16,
    bot_zone: u16,
) -> bool {
    if zone != bot_zone || ch.hp <= 0 || ch.res_hp_type == USER_DEAD {
        return false;
    }
    if ch.authority == 0 || !hostile(entry.kind, entry.nation, ch.nation) {
        return false;
    }
    if world.get_event_room(sid) != entry.room {
        return false;
    }
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    !world.is_player_blinking(sid, now_unix)
}

/// Best enemy of the bot inside its room (humans and enemy bots), scored like
/// a player would (`bot_brain::target_score_basic`). Diagnostics / tests; the
/// live decision is the brain's `pick_target`.
pub(crate) fn find_target(world: &WorldState, entry: &BotEntry, bot: &BotInstance) -> Option<Target> {
    let mut best: Option<(f32, Target)> = None;
    let mut consider = |t: Target, hp: i16, max_hp: i16, class: u16| {
        let d = ((t.x - bot.x).powi(2) + (t.z - bot.z).powi(2)).sqrt();
        let score = crate::systems::bot_brain::target_score_basic(d, hp, max_hp, class);
        if best.as_ref().is_none_or(|(bs, _)| score > *bs) {
            best = Some((score, t));
        }
    };
    for sid in world.sessions_in_zone(bot.zone_id) {
        let (Some(ch), Some(pos)) = (world.get_character_info(sid), world.get_position(sid)) else {
            continue;
        };
        if human_attackable(world, entry, sid, &ch, pos.zone_id, bot.zone_id) {
            consider(Target { id: sid as u32, x: pos.x, z: pos.z }, ch.hp, ch.max_hp, ch.class);
        }
    }
    for other_id in world.event_bots.ids_of(entry.kind) {
        if other_id == bot.id {
            continue;
        }
        if let Some(t) = resolve_target(world, entry, bot, other_id) {
            let (hp, max_hp, class) = world.get_bot(other_id).map(|b| (b.hp, b.max_hp, b.class)).unwrap_or((1, 1, 1));
            consider(t, hp, max_hp, class);
        }
    }
    best.map(|(_, t)| t)
}

// ── Movement / attack hooks used by the brain ───────────────────────────

/// Straight step for zones without map data (unit tests, maps without an SMD).
/// Never used when a map is loaded: there the common A* stepping decides.
pub(crate) fn no_map_step(world: &WorldState, bot: &BotInstance, gx: f32, gz: f32) -> Option<(f32, f32, f32)> {
    if world.get_zone(bot.zone_id).is_some_and(|z| z.map_data.is_some()) {
        return None;
    }
    let dx = gx - bot.x;
    let dz = gz - bot.z;
    let dist = (dx * dx + dz * dz).sqrt();
    if !dist.is_finite() || dist < 0.001 {
        return None;
    }
    let t = (STEP_PER_TICK / dist).min(1.0);
    Some((bot.x + dx * t, bot.y, bot.z + dz * t))
}

/// Broadcast one committed step of an event bot to its room only: WIZ_MOVE, plus the
/// INOUT pair when the step crosses a region (WIZ_MOVE alone does not make an unknown
/// bot appear). The caller writes the new position into the bot afterwards.
pub(crate) fn brain_move(world: &WorldState, entry: &BotEntry, bot: &BotInstance, nx: f32, ny: f32, nz: f32, echo: u8) {
    room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, move_packet(bot, nx, ny, nz, echo));
    let (rx, rz) = (calc_region(nx), calc_region(nz));
    if rx != bot.region_x || rz != bot.region_z {
        let mut out = Packet::new(Opcode::WizUserInout as u8);
        out.write_u8(2);
        out.write_u8(0);
        out.write_u32(bot.id);
        room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, out);
        let mut moved = bot.clone();
        moved.x = nx;
        moved.y = ny;
        moved.z = nz;
        moved.region_x = rx;
        moved.region_z = rz;
        let pkt = bot_ai::build_bot_inout_packet(&moved, world, 1);
        room_broadcast(world, bot.zone_id, rx, rz, entry.room, pkt);
    }
}

/// The brain decided to hit `target_id` (already in range, reaction delay over):
/// apply the event attack mechanics with the class cooldown. Returns `true` when a
/// swing happened.
pub(crate) fn brain_attack(world: &WorldState, entry: &BotEntry, bot: &BotInstance, target_id: u32, tx: f32, tz: f32, now_ms: u64) -> bool {
    if now_ms < entry.next_attack_ms {
        return false;
    }
    world.event_bots.update(bot.id, |e| e.next_attack_ms = now_ms + attack_cooldown(bot));
    attack(world, entry, bot, Target { id: target_id, x: tx, z: tz }, now_ms);
    true
}

/// BDW objective of the brain (altar / carrier / intercept / roam), see `bdw_ai`.
pub(crate) fn bdw_objective(
    world: &WorldState,
    entry: &BotEntry,
    bot: &BotInstance,
    st: &mut crate::systems::bot_brain::BrainState,
    now_ms: u64,
) -> crate::systems::bot_brain::Drive {
    bdw_ai::objective(world, entry, bot, st, now_ms)
}

/// BDW target priorities for the brain's scoring (the enemy altar carrier).
pub(crate) fn bdw_target_bonus(world: &WorldState, entry: &BotEntry, bot: &BotInstance, u: &crate::systems::bot_brain::Unit) -> f32 {
    bdw_ai::target_bonus(world, entry, bot, u)
}

/// Damage a bot of this class deals to a target with `ac` armour class.
pub fn mitigated_damage(base: i16, ac: i32) -> i16 {
    let ac = ac.max(0);
    let d = (base.max(0) as i64 * 240) / (ac as i64 + 240);
    d.clamp(0, i16::MAX as i64) as i16
}

fn attack(world: &WorldState, entry: &BotEntry, bot: &BotInstance, target: Target, now_ms: u64) {
    let skill = bot_ai::select_bot_skill_with_weapon(bot, Some(world));
    let base = bot_ai::calculate_bot_damage(bot);
    let missed = rand::thread_rng().gen_range(0..100) < MISS_PERCENT;

    let damage: i16 = if missed {
        0
    } else if target.id >= BOT_ID_BASE {
        base.max(1)
    } else {
        let ac = world.get_equipped_stats(target.id as SessionId).total_ac as i32;
        mitigated_damage(base, ac).max(1)
    };

    // animation: cast + effect, only seen by the bot's own room
    let pos = [bot.x as i32, bot.y as i32, bot.z as i32];
    room_broadcast(
        world,
        bot.zone_id,
        bot.region_x,
        bot.region_z,
        entry.room,
        magic_packet(1, skill, bot.id, target.id, [pos[0], pos[1], pos[2], 0, 0, 0, 0]),
    );
    room_broadcast(
        world,
        bot.zone_id,
        bot.region_x,
        bot.region_z,
        entry.room,
        magic_packet(3, skill, bot.id, target.id, [pos[0], pos[1], pos[2], -(damage as i32), 0, 0, 0]),
    );
    if damage == 0 {
        return;
    }
    if target.id >= BOT_ID_BASE {
        hit_bot(world, bot, target.id, damage, now_ms);
    } else {
        hit_player(world, entry, bot, target.id as SessionId, damage);
    }
}

fn hit_bot(world: &WorldState, attacker: &BotInstance, victim_id: u32, damage: i16, now_ms: u64) {
    let Some(victim) = world.get_bot(victim_id) else { return };
    if !victim.is_alive() {
        return;
    }
    let new_hp = (victim.hp - damage).max(0);
    world.update_bot(victim_id, |b| {
        b.hp = new_hp;
        b.last_attacker_id = attacker.id as i32;
    });
    if new_hp <= 0 {
        handle_bot_death(world, victim_id, now_ms);
    }
}

fn hit_player(world: &WorldState, entry: &BotEntry, bot: &BotInstance, victim: SessionId, damage: i16) {
    let Some(ch) = world.get_character_info(victim) else { return };
    if ch.hp <= 0 || ch.res_hp_type == USER_DEAD {
        return;
    }
    let new_hp = (ch.hp - damage).max(0);
    world.update_session(victim, |h| {
        if let Some(c) = h.character.as_mut() {
            c.hp = new_hp;
        }
    });
    world.send_to_session_owned(
        victim,
        crate::systems::regen::build_hp_change_packet_with_attacker(ch.max_hp, new_hp, bot.id),
    );
    if let Some(pos) = world.get_position(victim) {
        let hp_bar = crate::handler::target_hp::build_target_hp_packet(
            victim as u32,
            0,
            ch.max_hp as u32,
            new_hp as u32,
            -(damage as i32),
            -(damage as i32),
        );
        room_broadcast(world, pos.zone_id, pos.region_x, pos.region_z, entry.room, hp_bar);
    }
    if new_hp > 0 {
        return;
    }
    // the player died: same death path a human killer's hit ends in, minus NP / gold rewards
    crate::handler::dead::broadcast_death(world, victim);
    crate::handler::dead::set_who_killed_me(world, victim, bot.id as SessionId);
    record_event_kill(world, entry.kind, &entry.name, entry.nation, &ch.name);
    let (vx, vz) = world
        .get_position(victim)
        .map(|p| (p.x.max(0.0) as u16, p.z.max(0.0) as u16))
        .unwrap_or((0, 0));
    world.send_death_notice_to_zone(
        bot.zone_id,
        bot.id as SessionId,
        victim,
        &entry.name,
        &ch.name,
        None,
        vx,
        vz,
    );
}

fn respawn_bot(world: &WorldState, id: BotId, entry: &BotEntry) {
    let Some(old) = world.get_bot(id) else { return };
    let mut rng = rand::thread_rng();
    let (x, z) = spawn_point(world, old.zone_id, entry.nation, &mut rng).unwrap_or((old.x, old.z));
    // old body disappears, the new one appears at the spawn point
    let mut out = Packet::new(Opcode::WizUserInout as u8);
    out.write_u8(2);
    out.write_u8(0);
    out.write_u32(id);
    room_broadcast(world, old.zone_id, old.region_x, old.region_z, entry.room, out);
    let (rx, rz) = (calc_region(x), calc_region(z));
    world.update_bot(id, |b| {
        b.hp = b.max_hp.max(1);
        b.mp = b.max_mp;
        b.presence = BotPresence::Standing;
        b.ai_state = BotAiState::Afk;
        b.x = x;
        b.z = z;
        b.region_x = rx;
        b.region_z = rz;
        b.target_id = -1;
        b.last_attacker_id = -1;
        b.regene_at_ms = 0;
    });
    world.event_bots.update(id, |e| {
        e.respawn_at_ms = 0;
        e.next_attack_ms = 0;
    });
    crate::systems::bot_brain::on_bot_respawn(id, bot_ai::tick_ms());
    if let Some(b) = world.get_bot(id) {
        let pkt = bot_ai::build_bot_inout_packet(&b, world, 1);
        room_broadcast(world, b.zone_id, rx, rz, entry.room, pkt);
    }
}

// ── Ultima / Rift helpers ────────────────────────────────────────────────

/// Extra bots a room-less event (`Ultima`, `Rift`) should add for `humans` participants, or 0.
pub fn auxiliary_bot_count(world: &WorldState, kind: EventBotKind, humans: usize) -> usize {
    let cfg = world.event_bots.config(kind);
    if !cfg.enabled || kind.support() == Support::Unsupported || humans < cfg.min_humans.max(1) as usize {
        return 0;
    }
    bots_needed(&cfg, humans, kind.team_capacity())
}

/// Level for a helper bot of `kind`, from the configured range.
pub fn auxiliary_bot_level(world: &WorldState, kind: EventBotKind) -> u8 {
    world
        .event_bots
        .config(kind)
        .pick_level(&mut rand::thread_rng())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::event_room::EventRoomManager;
    use crate::world::{CharacterInfo, Position};

    fn on(kind: EventBotKind) -> EventBotConfig {
        EventBotConfig { enabled: true, ..Default::default() }.sanitized(kind)
    }

    #[test]
    fn default_config_is_off() {
        let cfg = EventBotConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(bots_needed(&cfg, 0, 8), 0);
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [3, 0]), [0, 0]);
    }

    #[test]
    fn fill_short_tops_up_to_target() {
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 6;
        cfg.fill_only_when_short = true;
        assert_eq!(bots_needed(&cfg, 0, 8), 6);
        assert_eq!(bots_needed(&cfg, 2, 8), 4);
        assert_eq!(bots_needed(&cfg, 6, 8), 0);
        assert_eq!(bots_needed(&cfg, 8, 8), 0);
    }

    #[test]
    fn always_mode_adds_the_full_count_but_respects_capacity() {
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 5;
        cfg.fill_only_when_short = false;
        assert_eq!(bots_needed(&cfg, 0, 8), 5);
        assert_eq!(bots_needed(&cfg, 6, 8), 2);
        assert_eq!(bots_needed(&cfg, 8, 8), 0);
    }

    #[test]
    fn bots_never_open_a_room_without_humans() {
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 6;
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [0, 0]), [0, 0]);
        cfg.min_humans = 3;
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [1, 1]), [0, 0]);
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [2, 1]), [4, 5]);
    }

    #[test]
    fn nation_events_end_up_with_equal_team_sizes() {
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 4;
        // a lone Karus player: Karus gets 3 teammates, El Morad (no human) mirrors the 4
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [1, 0]), [3, 4]);
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [0, 3]), [4, 1]);
        // 5 Karus players (more than bots_per_team): 5 mirror bots on the empty El Morad side
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [5, 0]), [0, 5]);
        // the live case: a 5-man party against a side that got only 1 bot before -> now 5
        let mut five = on(EventBotKind::Bdw);
        five.bots_per_team = 5;
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &five, [5, 0]), [0, 5]);
        // capped by the team capacity
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [8, 0])[1], 8);
        // both sides have humans: the plain top-up already gives 4 vs 4
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [2, 1]), [2, 3]);
        // one side above bots_per_team: the other side is mirrored up to it
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &cfg, [6, 2]), [0, 4]);
        for humans in [[1usize, 0usize], [0, 3], [5, 0], [2, 1], [6, 2], [3, 3]] {
            let p = plan_room_fill(EventBotKind::Bdw, &cfg, humans);
            assert_eq!(humans[0] + p[0], humans[1] + p[1], "equal teams for {humans:?}: {p:?}");
        }
        // fill_only_when_short = false: always bots_per_team extra, still equal
        let mut always = on(EventBotKind::Bdw);
        always.bots_per_team = 2;
        always.fill_only_when_short = false;
        assert_eq!(plan_room_fill(EventBotKind::Bdw, &always, [1, 0]), [2, 3]);
    }

    #[test]
    fn chaos_counts_the_whole_room() {
        let mut cfg = on(EventBotKind::Chaos);
        cfg.bots_per_team = 6;
        assert_eq!(plan_room_fill(EventBotKind::Chaos, &cfg, [2, 1]), [3, 0]);
        cfg.bots_per_team = 18;
        cfg.fill_only_when_short = false;
        assert_eq!(plan_room_fill(EventBotKind::Chaos, &cfg, [10, 5]), [3, 0]); // capped at 18
    }

    #[test]
    fn unsupported_events_never_get_bots() {
        for kind in [EventBotKind::ForgottenTemple, EventBotKind::Manes] {
            let cfg = EventBotConfig { enabled: true, ..Default::default() }.sanitized(kind);
            assert!(!cfg.enabled, "{kind:?} must stay disabled");
            assert_eq!(plan_room_fill(kind, &cfg, [2, 2]), [0, 0]);
        }
    }

    #[test]
    fn sanitize_clamps_everything() {
        let cfg = EventBotConfig {
            enabled: true,
            bots_per_team: 200,
            min_level: 90,
            max_level: 0,
            fill_only_when_short: true,
            min_humans: 0,
        }
        .sanitized(EventBotKind::Bdw);
        assert_eq!(cfg.bots_per_team, 8);
        assert_eq!((cfg.min_level, cfg.max_level), (1, 83));
        assert_eq!(cfg.min_humans, 1);
    }

    #[test]
    fn pick_level_stays_in_range() {
        let cfg = EventBotConfig { min_level: 70, max_level: 75, ..Default::default() };
        let mut rng = rand::thread_rng();
        for _ in 0..200 {
            let l = cfg.pick_level(&mut rng);
            assert!((70..=75).contains(&l));
        }
    }

    #[test]
    fn kind_keys_round_trip() {
        for kind in ALL_KINDS {
            assert_eq!(EventBotKind::from_key(kind.key()), Some(kind));
        }
        assert_eq!(EventBotKind::from_key("nope"), None);
    }

    #[test]
    fn mitigation_scales_with_armour() {
        assert_eq!(mitigated_damage(1000, 0), 1000);
        assert_eq!(mitigated_damage(1000, 240), 500);
        assert!(mitigated_damage(1000, 2000) < 150);
        assert_eq!(mitigated_damage(-5, 10), 0);
    }

    #[test]
    fn visibility_is_limited_to_the_bots_room() {
        let reg = EventBotRegistry::default();
        reg.insert(20_001, BotEntry::new(EventBotKind::Chaos, 3, 1, "A".into()));
        assert!(reg.visible_in_room(20_001, 3));
        assert!(!reg.visible_in_room(20_001, 4));
        assert!(!reg.visible_in_room(20_001, 0));
        // ordinary bots stay visible everywhere
        assert!(reg.visible_in_room(20_002, 4));
        assert_eq!(reg.event_room_of(20_001), 3);
        assert_eq!(reg.event_room_of(20_002), 0);
    }

    // ── real map navigation (client nav mask + authored openings, exactly as the server loads them) ──

    /// Load a zone the way `load_zones` does: server SMD + `Map/nav/<zone>.nav` + the authored
    /// event openings. `None` when the files are not checked out (the tests then skip).
    fn real_event_map(zone: u16, smd: &str) -> Option<crate::zone::MapData> {
        use ko_protocol::nav::NavMask;
        use ko_protocol::smd::SmdFile;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map");
        let smd_path = root.join(smd);
        let nav_path = root.join("nav").join(format!("{zone}.nav"));
        if !smd_path.exists() || !nav_path.exists() {
            eprintln!("skipping: {} / {} not found", smd_path.display(), nav_path.display());
            return None;
        }
        let map = crate::zone::MapData::new(SmdFile::load(&smd_path).expect("load SMD")).with_event_navigation(zone);
        map.set_nav(Some(NavMask::load(&nav_path).expect("load nav")));
        Some(map)
    }

    fn assert_route(map: &crate::zone::MapData, what: &str, from: (f32, f32), to: (f32, f32)) {
        use crate::systems::pathfind::{find_bot_path, nearest_bot_point};
        let s = nearest_bot_point(map, from.0, from.1, 8).unwrap_or_else(|| panic!("{what}: start {from:?} not walkable"));
        let g = nearest_bot_point(map, to.0, to.1, 8).unwrap_or_else(|| panic!("{what}: goal {to:?} not walkable"));
        let r = find_bot_path(map, s.0, s.1, g.0, g.1);
        assert!(r.found, "{what}: no bot route {from:?} -> {to:?} (snapped {s:?} -> {g:?})");
    }

    /// BDW (84): both spawns reach the altar, the altar reaches both delivery zones, and the
    /// bases reach each other. The client mask closes the spawn ledges and the two base gates
    /// (see `event_nav`); the authored openings reopen exactly those.
    #[test]
    fn bdw_real_map_has_bot_routes_spawn_altar_and_bases() {
        let Some(map) = real_event_map(84, "In_dungeon04.smd") else { return };
        let altar = (127.0, 131.0);
        assert_route(&map, "karus spawn -> altar", (51.0, 58.0), altar);
        assert_route(&map, "elmo spawn -> altar", (201.0, 207.0), altar);
        assert_route(&map, "altar -> karus base", altar, bdw_ai::base_centre(1));
        assert_route(&map, "altar -> elmo base", altar, bdw_ai::base_centre(2));
        assert_route(&map, "karus spawn -> elmo base", (51.0, 58.0), bdw_ai::base_centre(2));
        assert_route(&map, "elmo spawn -> karus base", (201.0, 207.0), bdw_ai::base_centre(1));
    }

    /// BDW (84): the arena's rocks and walls stay blocked. The points are the centres of
    /// baked collision objects read from `in_dungeon04.opd` (the two pillars in the field
    /// around the altar, the bastions beside each base gate) plus the SMD-only cells the old
    /// event-grid union let bots walk (the SMD is a different 1024 m map, walkable everywhere).
    #[test]
    fn bdw_real_map_keeps_rocks_and_walls_blocked() {
        let Some(map) = real_event_map(84, "In_dungeon04.smd") else { return };
        let rocks = [
            (143.7, 120.6, "field pillar east of the altar (.opd faces 1123..1137)"),
            (136.3, 137.3, "field pillar north-east of the altar (.opd faces 1139..1154)"),
            (74.0, 122.0, "karus base south bastion"),
            (70.0, 142.0, "karus base north bastion"),
            (186.0, 118.0, "elmo base south bastion"),
            (182.0, 142.0, "elmo base north bastion"),
            (10.0, 10.0, "map corner (SMD event grid says walkable)"),
            (128.0, 246.0, "north edge (SMD event grid says walkable)"),
        ];
        for (x, z, what) in rocks {
            assert!(!map.is_bot_movable(x, z), "{what} at ({x}, {z}) must stay blocked for bots");
        }
        // The event grid of the foreign SMD is no longer consulted: the arena keeps the
        // client mask's shape (the whole map would be ~95% walkable otherwise).
        let n = map.bot_grid_size();
        let walkable = (0..n).flat_map(|x| (0..n).map(move |z| (x, z))).filter(|&(x, z)| map.is_bot_movable_grid(x, z)).count();
        assert!(walkable < (n * n) as usize / 2, "BDW bot grid is {walkable} / {} walkable: the SMD grid leaked in", n * n);
    }

    /// Chaos (85): the whole arena is one connected hunting ground from the start position,
    /// including the raised south room behind its gate; the arena's inner walls stay blocked.
    #[test]
    fn chaos_real_map_arena_is_connected_from_the_start_position() {
        let Some(map) = real_event_map(85, "sky_war_2009.smd") else { return };
        let start = (126.0, 130.0);
        for corner in [(40.0, 40.0), (216.0, 40.0), (40.0, 216.0), (216.0, 216.0), (126.0, 60.0), (126.0, 200.0), (126.0, 40.0)] {
            assert_route(&map, "chaos start -> arena point", start, corner);
        }
        for (x, z) in [(62.0, 130.0), (190.0, 130.0), (80.0, 102.0), (80.0, 154.0)] {
            assert!(!map.is_bot_movable(x, z), "chaos inner wall at ({x}, {z}) must stay blocked");
        }
    }

    /// Juraid (87): every lane leg (spawn -> stage 1 -> 2 -> 3 -> Deva room) is a bot route for
    /// both nations, so the lane runners can clear the stages themselves.
    #[test]
    fn juraid_real_map_lanes_and_deva_room_are_reachable_from_both_spawns() {
        use crate::systems::event_bots_juraid::{chain, ELMORAD_SPAWN, KARUS_SPAWN};
        let Some(map) = real_event_map(87, "In_Dungeon05.smd") else { return };
        for (nation, spawn) in [(1u8, KARUS_SPAWN), (2u8, ELMORAD_SPAWN)] {
            let mut from = spawn;
            for (i, wp) in chain(nation).iter().enumerate() {
                assert_route(&map, &format!("nation {nation} leg {i}"), from, *wp);
                from = *wp;
            }
        }
    }

    /// Diagnostic: `cargo test -p ko-game --lib -- dump_event_map_grids --ignored --nocapture`
    /// prints the bot walkability of the Chaos and Juraid arenas as the server sees it.
    #[test]
    #[ignore]
    fn dump_event_map_grids() {
        for (zone, smd, step, marks) in [
            (85u16, "sky_war_2009.smd", 1usize, vec![(126.0f32, 130.0f32, 'S')]),
            (87, "In_Dungeon05.smd", 2, vec![(224.0, 272.0, 'K'), (799.0, 748.0, 'E'), (224.0, 558.0, '1'), (224.0, 848.0, '2'), (511.0, 848.0, '3'), (508.0, 513.0, 'D'), (799.0, 459.0, '1'), (799.0, 171.0, '2'), (511.0, 172.0, '3')]),
        ] {
            let Some(map) = real_event_map(zone, smd) else { continue };
            let n = map.bot_grid_size();
            let unit = map.unit_dist();
            let ev = map.grid_size();
            eprintln!("zone {zone}: bot grid {n} smd grid {ev} unit {unit} width {}", map.map_width());
            let nz = map.bot_grid_size();
            let mut z = nz - 1;
            while z >= 0 {
                let mut row = String::new();
                let mut x = 0;
                while x < nz {
                    let mut c = if map.is_bot_movable_grid(x, z) { '.' } else { '#' };
                    for (mx, mz, ch) in &marks {
                        if (((*mx / unit) as i32) / step as i32) == x / step as i32 && (((*mz / unit) as i32) / step as i32) == z / step as i32 {
                            c = *ch;
                        }
                    }
                    row.push(c);
                    x += step as i32;
                }
                eprintln!("{:5} {}", z as f32 * unit, row);
                z -= step as i32;
            }
        }
    }

    // ── world level tests ────────────────────────────────────────────

    fn world_with_zone_start(zone: u16) -> WorldState {
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: zone as i16,
            karus_x: 51,
            karus_z: 58,
            elmorad_x: 201,
            elmorad_z: 207,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });
        world
    }

    fn add_human(world: &WorldState, sid: SessionId, nation: u8, zone: u16, room: u16, name: &str) {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                name: name.into(),
                nation,
                level: 70,
                hp: 5000,
                max_hp: 5000,
                authority: 1,
                ..Default::default()
            },
            Position { zone_id: zone, x: 100.0, y: 0.0, z: 100.0, region_x: calc_region(100.0), region_z: calc_region(100.0) },
        );
        world.update_session(sid, |h| h.event_room = room);
    }

    fn open_room_with_humans(
        world: &WorldState,
        et: TempleEventType,
        humans: &[(SessionId, u8, &str)],
    ) {
        let erm = &world.event_room_manager;
        erm.create_rooms(et, 2);
        for (sid, nation, name) in humans {
            let mut room = erm.get_room_mut(et, 1).unwrap();
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
            s.active_event = et as i16;
            s.is_active = true;
            s.is_attackable = true;
        });
    }

    #[test]
    fn disabled_config_adds_no_bots() {
        let world = world_with_zone_start(84);
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        assert_eq!(fill_temple_rooms(&world, TempleEventType::BorderDefenceWar), 0);
        assert_eq!(world.bot_count(), 0);
        assert_eq!(world.event_bots.total(), 0);
    }

    #[test]
    fn bdw_fill_registers_bots_in_room_and_cleanup_removes_them() {
        let world = world_with_zone_start(84);
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 3;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);

        let added = fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        // Karus: 3 - 1 human = 2 teammates; El Morad has no human: 3 mirror bots (equal teams)
        assert_eq!(added, 5);
        assert_eq!(world.event_bots.count(EventBotKind::Bdw), 5);
        assert_eq!(world.bot_count(), 5);
        {
            let room = world
                .event_room_manager
                .get_room(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            assert_eq!(room.karus_users.len(), 3);
            assert_eq!(room.elmorad_users.len(), 3);
            // bots are flagged as already rewarded and carry bot-band session ids
            assert!(room.elmorad_users.values().all(|u| u.prize_given && u.session_id as u32 >= BOT_ID_BASE));
        }
        for id in world.event_bots.ids() {
            assert_eq!(world.get_bot(id).unwrap().ai_state, BotAiState::Afk);
            assert!(is_event_bot_sid(&world, id as SessionId));
        }
        assert!(!is_event_bot_sid(&world, 1));

        assert_eq!(cleanup_kind(&world, EventBotKind::Bdw), 5);
        assert_eq!(world.bot_count(), 0);
        assert_eq!(world.event_bots.total(), 0);
        let room = world
            .event_room_manager
            .get_room(TempleEventType::BorderDefenceWar, 1)
            .unwrap();
        assert_eq!(room.karus_users.len(), 1);
        assert!(room.elmorad_users.is_empty());
    }

    #[test]
    fn no_humans_means_no_bots() {
        let world = world_with_zone_start(85);
        open_room_with_humans(&world, TempleEventType::ChaosDungeon, &[]);
        let mut cfg = on(EventBotKind::Chaos);
        cfg.bots_per_team = 10;
        world.event_bots.set_config(EventBotKind::Chaos, cfg);
        assert_eq!(fill_temple_rooms(&world, TempleEventType::ChaosDungeon), 0);
        assert_eq!(world.bot_count(), 0);
    }

    #[test]
    fn chaos_fill_tops_up_the_room() {
        let world = world_with_zone_start(85);
        open_room_with_humans(
            &world,
            TempleEventType::ChaosDungeon,
            &[(1, 1, "A"), (2, 2, "B")],
        );
        let mut cfg = on(EventBotKind::Chaos);
        cfg.bots_per_team = 6;
        world.event_bots.set_config(EventBotKind::Chaos, cfg);
        assert_eq!(fill_temple_rooms(&world, TempleEventType::ChaosDungeon), 4);
        let room = world.event_room_manager.get_room(TempleEventType::ChaosDungeon, 1).unwrap();
        assert_eq!(room.mixed_users.len(), 6);
    }

    #[test]
    fn sweep_removes_bots_of_a_finished_event() {
        let world = world_with_zone_start(85);
        open_room_with_humans(&world, TempleEventType::ChaosDungeon, &[(1, 1, "A")]);
        let mut cfg = on(EventBotKind::Chaos);
        cfg.bots_per_team = 4;
        world.event_bots.set_config(EventBotKind::Chaos, cfg);
        assert_eq!(fill_temple_rooms(&world, TempleEventType::ChaosDungeon), 3);
        assert_eq!(sweep(&world), 0, "running event keeps its bots");

        // the event is over: rooms destroyed, state reset
        world.event_room_manager.destroy_rooms(TempleEventType::ChaosDungeon);
        world.event_room_manager.reset_temple_event();
        assert_eq!(sweep(&world), 3);
        assert_eq!(world.bot_count(), 0);
        assert_eq!(world.event_bots.total(), 0);
    }

    #[test]
    fn sweep_removes_bots_of_a_different_active_event() {
        let world = world_with_zone_start(85);
        open_room_with_humans(&world, TempleEventType::ChaosDungeon, &[(1, 1, "A")]);
        let mut cfg = on(EventBotKind::Chaos);
        cfg.bots_per_team = 3;
        world.event_bots.set_config(EventBotKind::Chaos, cfg);
        fill_temple_rooms(&world, TempleEventType::ChaosDungeon);
        world.event_room_manager.update_temple_event(|s| s.active_event = TempleEventType::BorderDefenceWar as i16);
        assert_eq!(sweep(&world), 2);
    }

    #[test]
    fn room_cleanup_only_touches_that_room() {
        let world = WorldState::new();
        for (id, room) in [(20_100u32, 1u16), (20_101, 2)] {
            world.event_bots.insert(id, BotEntry::new(EventBotKind::Chaos, room, 1, format!("B{id}")));
        }
        // no BotInstance behind them: despawn must still clear the registry
        assert_eq!(cleanup_room(&world, EventBotKind::Chaos, 1), 1);
        assert_eq!(world.event_bots.total(), 1);
        assert_eq!(cleanup_all(&world), 1);
        assert_eq!(world.event_bots.total(), 0);
    }

    #[test]
    fn rift_support_room_flag() {
        let reg = EventBotRegistry::default();
        assert!(!reg.rift_room_is_support(5));
        reg.mark_rift_support_room(5);
        assert!(reg.rift_room_is_support(5));
        reg.clear_rift_room(5);
        assert!(!reg.rift_room_is_support(5));
    }

    #[test]
    fn auxiliary_counts_follow_the_config() {
        let world = WorldState::new();
        assert_eq!(auxiliary_bot_count(&world, EventBotKind::Ultima, 1), 0);
        let mut cfg = on(EventBotKind::Ultima);
        cfg.bots_per_team = 10;
        cfg.fill_only_when_short = true;
        world.event_bots.set_config(EventBotKind::Ultima, cfg);
        assert_eq!(auxiliary_bot_count(&world, EventBotKind::Ultima, 1), 9);
        assert_eq!(auxiliary_bot_count(&world, EventBotKind::Ultima, 12), 0);
        assert_eq!(auxiliary_bot_count(&world, EventBotKind::Ultima, 0), 0);
    }

    // ── combat ───────────────────────────────────────────────────────

    fn fill_chaos_with_one_human_and_bots(world: &WorldState, bots: u8) {
        add_human(world, 1, 1, 85, 1, "Human");
        open_room_with_humans(world, TempleEventType::ChaosDungeon, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Chaos);
        cfg.bots_per_team = bots + 1;
        world.event_bots.set_config(EventBotKind::Chaos, cfg);
        fill_temple_rooms(world, TempleEventType::ChaosDungeon);
    }

    fn phase_active(world: &WorldState) {
        // current_phase() reads these flags
        world.event_room_manager.update_temple_event(|s| {
            s.is_active = true;
            s.is_attackable = true;
            s.timer_start_control = true;
        });
    }

    #[test]
    fn bot_death_marks_dead_credits_killer_and_schedules_respawn() {
        let world = world_with_zone_start(85);
        fill_chaos_with_one_human_and_bots(&world, 1);
        phase_active(&world);
        let bot_id = world.event_bots.ids()[0];
        world.update_bot(bot_id, |b| {
            b.hp = 1;
            b.last_attacker_id = 1; // the human killed it
        });
        assert!(handle_bot_death(&world, bot_id, 1_000));
        let bot = world.get_bot(bot_id).unwrap();
        assert_eq!(bot.presence, BotPresence::Dead);
        assert_eq!(bot.regene_at_ms, 0, "bot AI must not regene event bots");
        let entry = world.event_bots.get(bot_id).unwrap();
        assert_eq!(entry.respawn_at_ms, 1_000 + RESPAWN_MS);
        // the kill reached the Chaos room bookkeeping
        let room = world.event_room_manager.get_room(TempleEventType::ChaosDungeon, 1).unwrap();
        assert_eq!(room.mixed_users.get("Human").unwrap().kills, 1);
        assert_eq!(room.mixed_users.get(&entry.name).unwrap().deaths, 1);
        // handled twice is a no-op
        drop(room);
        assert!(handle_bot_death(&world, bot_id, 2_000));
        assert_eq!(world.event_bots.get(bot_id).unwrap().respawn_at_ms, 1_000 + RESPAWN_MS);
        // ordinary bots are not ours
        assert!(!handle_bot_death(&world, 29_999, 0));
    }

    #[test]
    fn dead_bot_respawns_after_the_timer() {
        let world = world_with_zone_start(85);
        fill_chaos_with_one_human_and_bots(&world, 1);
        phase_active(&world);
        let bot_id = world.event_bots.ids()[0];
        world.update_bot(bot_id, |b| b.last_attacker_id = -1);
        handle_bot_death(&world, bot_id, 1_000);
        tick_bot(&world, bot_id, 1_000 + RESPAWN_MS - 1, true);
        assert_eq!(world.get_bot(bot_id).unwrap().presence, BotPresence::Dead);
        tick_bot(&world, bot_id, 1_000 + RESPAWN_MS, true);
        let bot = world.get_bot(bot_id).unwrap();
        assert_eq!(bot.presence, BotPresence::Standing);
        assert!(bot.hp > 0);
        assert_eq!(world.event_bots.get(bot_id).unwrap().respawn_at_ms, 0);
    }

    #[test]
    fn bots_walk_to_and_hurt_a_human_in_their_room() {
        let world = world_with_zone_start(85);
        fill_chaos_with_one_human_and_bots(&world, 1);
        phase_active(&world);
        let bot_id = world.event_bots.ids()[0];
        // put the human next to the bot
        let bot = world.get_bot(bot_id).unwrap();
        world.update_position(1, 85, bot.x + 3.0, 0.0, bot.z);
        let hp_before = world.get_character_info(1).unwrap().hp;
        tick_bot(&world, bot_id, 10_000, true);
        let hp_after = world.get_character_info(1).unwrap().hp;
        assert!(hp_after <= hp_before);
        // a human 20 m away (inside the brain's search range) is walked to, not hit
        world.update_position(1, 85, bot.x + 20.0, 0.0, bot.z);
        crate::systems::bot_brain::forget(bot_id);
        let before = world.get_bot(bot_id).unwrap();
        tick_bot(&world, bot_id, 20_000, true);
        let after = world.get_bot(bot_id).unwrap();
        let moved = ((after.x - before.x).powi(2) + (after.z - before.z).powi(2)).sqrt();
        assert!(moved > 0.5 && moved <= STEP_PER_TICK + 0.01, "moved {moved}");
        assert!(after.x > before.x, "walks toward the human");
    }

    #[test]
    fn bots_ignore_humans_of_other_rooms_and_do_nothing_outside_the_fighting_phase() {
        let world = world_with_zone_start(85);
        fill_chaos_with_one_human_and_bots(&world, 1);
        phase_active(&world);
        let bot_id = world.event_bots.ids()[0];
        let bot = world.get_bot(bot_id).unwrap();
        world.update_position(1, 85, bot.x + 2.0, 0.0, bot.z);
        // human in another room: not a valid target
        world.update_session(1, |h| h.event_room = 2);
        assert!(find_target(&world, &world.event_bots.get(bot_id).unwrap(), &bot).is_none());
        world.update_session(1, |h| h.event_room = 1);
        assert!(find_target(&world, &world.event_bots.get(bot_id).unwrap(), &bot).is_some());
        // not the fighting phase: nothing happens
        let hp = world.get_character_info(1).unwrap().hp;
        tick_bot(&world, bot_id, 5_000, false);
        assert_eq!(world.get_character_info(1).unwrap().hp, hp);
    }

    #[test]
    fn players_cannot_hit_bots_of_their_own_nation_or_other_rooms() {
        let world = world_with_zone_start(84);
        add_human(&world, 1, 1, 84, 1, "Human");
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 2;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);
        fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        phase_active(&world);
        let ids = world.event_bots.ids();
        let friend = ids.iter().copied().find(|id| world.get_bot(*id).unwrap().nation == 1).unwrap();
        let enemy = ids.iter().copied().find(|id| world.get_bot(*id).unwrap().nation == 2).unwrap();
        assert!(deny_player_attack(&world, 1, friend), "same nation");
        assert!(!deny_player_attack(&world, 1, enemy), "enemy in the same room");
        world.update_session(1, |h| h.event_room = 2);
        assert!(deny_player_attack(&world, 1, enemy), "other room");
        world.update_session(1, |h| h.event_room = 1);
        world.event_room_manager.update_temple_event(|s| s.is_attackable = false);
        assert!(deny_player_attack(&world, 1, enemy), "combat not open yet");
        // ordinary bots are never gated here
        assert!(!deny_player_attack(&world, 1, 29_998));
    }

    /// The attack handler needs the target's class coefficient row too (a bot is hit with the
    /// player formula); live the whole table is loaded, tests get one row per bot class.
    fn insert_bot_class_coefficients(world: &WorldState) {
        for id in world.event_bots.ids() {
            let class = world.get_bot(id).unwrap().class;
            world.insert_coefficient(ko_db::models::CoefficientRow {
                s_class: class as i16,
                short_sword: 0.001,
                jamadar: 0.001,
                sword: 0.002,
                axe: 0.002,
                club: 0.002,
                spear: 0.002,
                pole: 0.001,
                staff: 0.001,
                bow: 0.001,
                hp: 1.0,
                mp: 1.0,
                sp: 1.0,
                ac: 0.05,
                hitrate: 0.001,
                evasionrate: 0.001,
            });
        }
    }

    /// The real R-attack path: a player standing next to an enemy-nation BDW bot hurts it, a
    /// same-nation bot is untouchable. Guards the whole chain (field-bot rule, event gate, zone and
    /// range checks), not only `deny_player_attack`.
    #[tokio::test]
    async fn bdw_players_hurt_enemy_bots_through_the_attack_handler() {
        let world = Arc::new(world_with_zone_start(84));
        add_human(&world, 1, 1, 84, 1, "Human");
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 2;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);
        fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        phase_active(&world);
        // the human has no gear in this world: use GM damage so the swing cannot miss for 0; the
        // attack path still needs the class coefficient row (live: loaded from the DB)
        world.update_character_stats(1, |c| {
            c.authority = 0;
            c.class = 101;
        });
        world.insert_coefficient(
            ko_db::models::CoefficientRow {
                s_class: 101,
                short_sword: 0.001,
                jamadar: 0.001,
                sword: 0.002,
                axe: 0.002,
                club: 0.002,
                spear: 0.002,
                pole: 0.001,
                staff: 0.001,
                bow: 0.001,
                hp: 1.0,
                mp: 1.0,
                sp: 1.0,
                ac: 0.05,
                hitrate: 0.001,
                evasionrate: 0.001,
            },
        );
        insert_bot_class_coefficients(&world);
        let ids = world.event_bots.ids();
        let enemy = ids.iter().copied().find(|id| world.get_bot(*id).unwrap().nation == 2).unwrap();
        let friend = ids.iter().copied().find(|id| world.get_bot(*id).unwrap().nation == 1).unwrap();

        let enemy_bot = world.get_bot(enemy).unwrap();
        world.update_position(1, 84, enemy_bot.x + 2.0, 0.0, enemy_bot.z);
        let pos = world.get_position(1).unwrap();
        crate::handler::attack::handle_npc_attack(Arc::clone(&world), 1, pos, enemy, 1, 0).await;
        let after = world.get_bot(enemy).unwrap();
        assert!(after.hp < enemy_bot.hp, "enemy bot must take damage ({} -> {})", enemy_bot.hp, after.hp);

        let friend_bot = world.get_bot(friend).unwrap();
        world.update_position(1, 84, friend_bot.x + 2.0, 0.0, friend_bot.z);
        let pos = world.get_position(1).unwrap();
        crate::handler::attack::handle_npc_attack(Arc::clone(&world), 1, pos, friend, 1, 0).await;
        assert_eq!(world.get_bot(friend).unwrap().hp, friend_bot.hp, "same-nation bot stays unhurt");
    }

    /// Chaos is free for all: a player hurts a bot of his OWN nation through the real R-attack
    /// handler (the room / phase gate must not treat Chaos as nation versus nation).
    #[tokio::test]
    async fn chaos_players_hurt_same_nation_bots_through_the_attack_handler() {
        let world = Arc::new(world_with_zone_start(85));
        fill_chaos_with_one_human_and_bots(&world, 4);
        phase_active(&world);
        world.update_character_stats(1, |c| {
            c.authority = 0;
            c.class = 101;
        });
        world.insert_coefficient(ko_db::models::CoefficientRow {
            s_class: 101,
            short_sword: 0.001,
            jamadar: 0.001,
            sword: 0.002,
            axe: 0.002,
            club: 0.002,
            spear: 0.002,
            pole: 0.001,
            staff: 0.001,
            bow: 0.001,
            hp: 1.0,
            mp: 1.0,
            sp: 1.0,
            ac: 0.05,
            hitrate: 0.001,
            evasionrate: 0.001,
        });
        insert_bot_class_coefficients(&world);
        let ids = world.event_bots.ids();
        let same = ids.iter().copied().find(|id| world.get_bot(*id).unwrap().nation == 1).expect("a nation-1 bot");
        assert!(!deny_player_attack(&world, 1, same), "FFA: same nation is a valid target");
        let bot = world.get_bot(same).unwrap();
        world.update_position(1, 85, bot.x + 2.0, 0.0, bot.z);
        let pos = world.get_position(1).unwrap();
        crate::handler::attack::handle_npc_attack(Arc::clone(&world), 1, pos, same, 1, 0).await;
        assert!(world.get_bot(same).unwrap().hp < bot.hp, "same-nation Chaos bot must take damage");
    }

    /// The deny gate reports the exact reason (room / phase / nation), so a live "cannot attack"
    /// report is diagnosable from the warn log.
    #[test]
    fn deny_gate_distinguishes_room_phase_and_nation() {
        let world = world_with_zone_start(84);
        add_human(&world, 1, 1, 84, 1, "Human");
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 1;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);
        fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        let enemy = world.event_bots.ids().into_iter().find(|id| world.get_bot(*id).unwrap().nation == 2).unwrap();
        // registration phase: combat not open
        assert!(deny_player_attack(&world, 1, enemy));
        phase_active(&world);
        assert!(!deny_player_attack(&world, 1, enemy));
        // a player that never got his room (event_room 0) cannot hit anybody
        world.update_session(1, |h| h.event_room = 0);
        assert!(deny_player_attack(&world, 1, enemy));
    }

    #[test]
    fn missing_bot_instance_is_dropped_from_the_registry() {
        let world = WorldState::new();
        world.event_bots.insert(20_500, BotEntry::new(EventBotKind::Chaos, 1, 1, "Ghost".into()));
        tick_bot(&world, 20_500, 1_000, true);
        assert!(world.event_bots.get(20_500).is_none());
    }

    // ── BDW objective / party tests ──────────────────────────────────

    fn altar_template() -> crate::npc::NpcTemplate {
        crate::systems::bdw::test_altar_template()
    }

    /// A BDW room 1 with one Karus human (sid 1), bots enabled; returns the world.
    fn bdw_world_with_bots() -> WorldState {
        let world = world_with_zone_start(84);
        add_human(&world, 1, 1, 84, 1, "Human");
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        world.bdw_manager_write().room_states.insert(1, crate::systems::bdw::BdwRoomState::new());
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 4;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);
        fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        world
    }

    fn bots_of(world: &WorldState, nation: u8) -> Vec<BotId> {
        world
            .event_bots
            .ids_of(EventBotKind::Bdw)
            .into_iter()
            .filter(|id| world.get_bot(*id).unwrap().nation == nation)
            .collect()
    }

    #[test]
    fn bdw_event_party_contains_the_nations_bots() {
        let world = bdw_world_with_bots();
        assert_eq!(bots_of(&world, 1).len(), 3);
        crate::systems::event_room::temple_event_create_parties(&world, TempleEventType::BorderDefenceWar);
        let pid = world.get_party_id(1).expect("the lone human gets a party because bots exist");
        let party = world.get_party(pid).unwrap();
        assert_eq!(party.leader_sid(), Some(1));
        for id in bots_of(&world, 1) {
            assert_eq!(world.get_party_id(id as SessionId), Some(pid), "Karus bot {id} in the party");
        }
        for id in bots_of(&world, 2) {
            assert_eq!(world.get_party_id(id as SessionId), None, "enemy bot {id} not in the party");
        }
        assert_eq!(party.member_count(), 4);
    }

    #[test]
    fn bdw_party_fill_respects_the_size_limit_and_keeps_slots_free() {
        let world = bdw_world_with_bots();
        for i in 0..5 {
            let (id, name) =
                spawn_event_bot(&world, EventBotKind::Bdw, 1, 84, 1, i, 70, &HashSet::new()).unwrap();
            world
                .event_room_manager
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap()
                .add_user(EventUser {
                    user_name: name,
                    session_id: id as SessionId,
                    nation: 1,
                    prize_given: true,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                });
        }
        let pid = world.create_party(1).unwrap();
        let added = fill_party_with_room_bots(&world, pid);
        assert_eq!(world.get_party(pid).unwrap().member_count(), 6, "two slots stay free for humans");
        assert_eq!(added, 5);
        assert_eq!(fill_party_with_room_bots(&world, pid), 0, "idempotent");
    }

    #[test]
    fn bdw_party_fill_does_nothing_outside_a_bdw_room() {
        let world = bdw_world_with_bots();
        let pid = world.create_party(1).unwrap();
        world.update_session(1, |h| h.event_room = 0);
        assert_eq!(fill_party_with_room_bots(&world, pid), 0);
    }

    #[test]
    fn bdw_bot_kills_the_altar_carries_it_home_and_scores_without_a_reward() {
        let world = bdw_world_with_bots();
        world.ensure_zone(84, 512);
        world.insert_npc_template(altar_template());
        let altar = world.spawn_event_npc(crate::systems::bdw::ALTAR_OF_MANES, true, 84, 127.0, 131.0, 1)[0];
        let mut inst = (*world.get_npc_instance(altar).unwrap()).clone();
        inst.event_room = 1;
        world.insert_npc_instance(inst);

        let bot_id = bots_of(&world, 1)[0];
        world.update_bot(bot_id, |b| {
            b.x = 127.0;
            b.z = 131.0;
        });
        let entry = world.event_bots.get(bot_id).unwrap();
        let mut st = crate::systems::bot_brain::state_of(bot_id, 10_000);
        st.role = crate::systems::bot_brain::Role::Hotspot; // an altar taker
        let mut now = 10_000u64;
        for _ in 0..40 {
            let bot = world.get_bot(bot_id).unwrap();
            let entry = world.event_bots.get(bot_id).unwrap();
            assert_eq!(bdw_ai::objective(&world, &entry, &bot, &mut st, now), crate::systems::bot_brain::Drive::Handled);
            now += 2_000;
            if world.get_npc_hp(altar) == Some(0) {
                break;
            }
        }
        assert_eq!(world.get_npc_hp(altar), Some(0), "the bots hit the altar down");
        {
            let room = world.event_room_manager.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
            let carrier = crate::systems::bdw::find_flag_carrier(&room).expect("a bot carries the altar");
            assert_eq!(carrier.0, entry.name);
            assert!(room.karus_users[&entry.name].prize_given, "the bot is never rewarded");
        }

        // the carrier walks home (slowly) and delivers inside the base
        let (hx, hz) = bdw_ai::base_centre(1);
        world.update_bot(bot_id, |b| {
            b.x = hx;
            b.z = hz;
        });
        let bot = world.get_bot(bot_id).unwrap();
        assert_eq!(bdw_ai::objective(&world, &entry, &bot, &mut st, now), crate::systems::bot_brain::Drive::Handled);
        let room = world.event_room_manager.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
        assert!(room.karus_score >= 10, "delivery scored: {}", room.karus_score);
        assert!(crate::systems::bdw::find_flag_carrier(&room).is_none());
        drop(room);
        let state = world.bdw_manager_read();
        assert!(state.get_room_state(1).unwrap().altar_respawn_pending, "altar respawn timer started");
    }

    #[test]
    fn bdw_carrier_bot_is_slower_than_a_normal_step() {
        assert!(bdw_ai::carrier_step() < STEP_PER_TICK);
        assert!(bdw_ai::carrier_step() > 0.0);
    }

    #[test]
    fn dead_bdw_carrier_bot_drops_the_altar_and_starts_the_respawn() {
        let world = bdw_world_with_bots();
        let bot_id = bots_of(&world, 1)[0];
        let name = world.event_bots.get(bot_id).unwrap().name;
        {
            let mut room = world.event_room_manager.get_room_mut(TempleEventType::BorderDefenceWar, 1).unwrap();
            crate::systems::bdw::flag_pickup(&mut room, &name);
        }
        world.update_bot(bot_id, |b| b.last_attacker_id = -1);
        handle_bot_death(&world, bot_id, 5_000);
        let room = world.event_room_manager.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
        assert!(crate::systems::bdw::find_flag_carrier(&room).is_none());
        drop(room);
        assert!(world.bdw_manager_read().get_room_state(1).unwrap().altar_respawn_pending);
    }

    #[test]
    fn rooms_without_registration_do_not_confuse_the_manager() {
        // EventRoomManager::new() must stay usable standalone (guards the test helpers above)
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::ChaosDungeon, 1);
        assert_eq!(erm.list_rooms(TempleEventType::ChaosDungeon).len(), 1);
    }

    // ── Support casting (heals / buffs on room mates) ─────────────────────

    fn karus_heal_rows(world: &WorldState) {
        world.insert_magic(ko_db::models::MagicRow {
            magic_num: 112_545,
            en_name: Some("Heal".into()),
            kr_name: None,
            description: None,
            t_1: Some(0),
            before_action: Some(0),
            target_action: Some(0),
            self_effect: Some(0),
            flying_effect: Some(0),
            target_effect: Some(0),
            moral: Some(2),
            skill_level: Some(45),
            skill: Some(45),
            msp: Some(0),
            hp: Some(0),
            s_sp: Some(0),
            item_group: Some(0),
            use_item: Some(0),
            cast_time: Some(0),
            recast_time: Some(0),
            success_rate: Some(100),
            type1: Some(3),
            type2: Some(0),
            range: Some(30),
            etc: Some(0),
            use_standing: Some(0),
            skill_check: Some(0),
            icelightrate: Some(0),
        });
        world.insert_magic_type3(ko_db::models::MagicType3Row {
            i_num: 112_545,
            direct_type: Some(1),
            first_damage: Some(500),
            time_damage: Some(0),
            duration: Some(0),
            attribute: Some(0),
            radius: Some(0),
            add_dmg_perc_to_user: None,
            add_dmg_perc_to_npc: None,
        });
    }

    fn karus_priest(world: &WorldState) -> BotId {
        any_priest(world, Some(1))
    }

    /// Any bot of `nation` (any nation: `None`), turned into a priest (class slots depend on the
    /// mirror fill, so the class is forced instead of searched).
    fn any_priest(world: &WorldState, nation: Option<u8>) -> BotId {
        let id = world
            .event_bots
            .ids()
            .into_iter()
            .find(|id| world.get_bot(*id).is_some_and(|b| nation.is_none_or(|n| b.nation == n)))
            .expect("a bot of that nation");
        world.update_bot(id, |b| b.class = if b.nation == 1 { 112 } else { 212 });
        id
    }

    /// BDW: a priest bot heals a wounded real player of its nation and room (not its party)
    /// through the PK bots' support rotation, with the real heal row.
    #[test]
    fn bdw_priest_bot_heals_wounded_same_nation_room_member() {
        let world = world_with_zone_start(84);
        karus_heal_rows(&world);
        add_human(&world, 1, 1, 84, 1, "Human");
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 4;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);
        fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        phase_active(&world);
        let priest = karus_priest(&world);
        let bot = world.get_bot(priest).unwrap();
        world.update_position(1, 84, bot.x + 3.0, 0.0, bot.z);
        world.update_character_stats(1, |c| c.hp = 1000);
        let entry = world.event_bots.get(priest).unwrap();
        assert!(
            bot_ai::tick_event_bot_support(&world, &bot, &entry, 10_000),
            "the priest must cast on the wounded room member"
        );
        assert_eq!(world.get_character_info(1).unwrap().hp, 1500);
    }

    /// The same wounded player listed in another room is nobody's business.
    #[test]
    fn bdw_priest_bot_ignores_other_rooms() {
        let world = world_with_zone_start(84);
        karus_heal_rows(&world);
        add_human(&world, 1, 1, 84, 1, "Human");
        open_room_with_humans(&world, TempleEventType::BorderDefenceWar, &[(1, 1, "Human")]);
        let mut cfg = on(EventBotKind::Bdw);
        cfg.bots_per_team = 4;
        world.event_bots.set_config(EventBotKind::Bdw, cfg);
        fill_temple_rooms(&world, TempleEventType::BorderDefenceWar);
        phase_active(&world);
        let priest = karus_priest(&world);
        let bot = world.get_bot(priest).unwrap();
        world.update_position(1, 84, bot.x + 3.0, 0.0, bot.z);
        world.update_character_stats(1, |c| c.hp = 1000);
        let mut entry = world.event_bots.get(priest).unwrap();
        entry.room = 2; // the bot belongs to room 2, the human is listed in room 1
        assert!(!bot_ai::tick_event_bot_support(&world, &bot, &entry, 10_000));
        assert_eq!(world.get_character_info(1).unwrap().hp, 1000);
    }

    /// Chaos is free for all: a priest bot never heals anybody but itself.
    #[test]
    fn chaos_priest_bot_buffs_nobody_else() {
        let world = world_with_zone_start(85);
        karus_heal_rows(&world);
        fill_chaos_with_one_human_and_bots(&world, 4);
        phase_active(&world);
        let priest = any_priest(&world, None);
        let bot = world.get_bot(priest).unwrap();
        world.update_position(1, 85, bot.x + 3.0, 0.0, bot.z);
        world.update_character_stats(1, |c| c.hp = 1000);
        let entry = world.event_bots.get(priest).unwrap();
        assert!(!bot_ai::tick_event_bot_support(&world, &bot, &entry, 10_000));
        assert_eq!(world.get_character_info(1).unwrap().hp, 1000);
    }
}
