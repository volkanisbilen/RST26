//! Field bots -- players-like bots that hunt monsters in the NORMAL (non-PK) maps.
//!
//! The PK bot system in [`bot_ai`](super::bot_ai) fights enemy-nation players in Ronark Land /
//! Ardream. This module is the PvE counterpart: a bot spawned here lives in a normal map
//! (Luferson, El Morad, the Eslants, Moradon), picks a level-appropriate monster group, walks to
//! it with the same SMD walkability rules players are bound by, fights with its real class skills,
//! rests when hurt and never stands idle. Real players can party up with them and level together.
//!
//! # Rules (every one of them is covered by a test)
//!
//! * **Off by default.** Nothing spawns until an admin enables a zone (`field_bot_zone` table,
//!   admin panel "Saha Botlari"). Counts, levels and the class mix are per zone.
//! * **No artificial limits.** Only a very high technical safety ceiling
//!   ([`MAX_FIELD_BOTS_PER_NATION_ZONE`] per nation and zone, [`MAX_FIELD_BOTS_TOTAL`] overall, bounded by
//!   the bot-id range) and the per-tick work budget below.
//! * **Zone ownership.** A Karus land only gets Karus bots, an El Morad land only El Morad bots;
//!   neutral Moradon gets both. Hostile-nation field bots therefore only exist in Moradon (where
//!   nobody can attack the other nation anyway).
//! * **Nobody can farm them.** Killing a field bot never pays exp, NP, gold, kill credit or
//!   Collection Race progress, in any zone, for anyone (see [`player_can_hurt`] and the
//!   `bot_on_death` early-out). Players of the *same* nation, or any player where the zone forbids
//!   attacking the other nation, cannot damage them at all. Enemy-nation players in a PvP land can
//!   hit them (they are defenders of that land) but gain nothing from it.
//! * **Monsters pay nobody for a bot's kill.** A solo bot that kills a monster gives no exp/loot/
//!   quest credit to anybody and takes no damage rights, so it can never be used to feed a player.
//!   It also never takes a kill: it skips monsters a player has touched, that target a player, or
//!   that stand within [`POLITE_RADIUS`] of a real player.
//! * **Party play.** A real player invites a field bot with the normal party UI (or from the seeking-party
//!   window, where every free field bot is listed). The bot accepts by rule (nation/zone/level/health), follows
//!   the leader, priests heal and buff, and the bots fight like any other party member -- including the last
//!   hit. A kill made by a bot of a party is paid to the real party (exp, NP, quest and daily-quest credit, loot,
//!   as if the real member with the most damage had dealt it); the bot itself never receives exp or loot.
//! * **Bounded cost.** At most [`TICK_BUDGET_MS`] of work per tick, a round-robin cursor so every
//!   bot is served, a scan budget per tick, and bots in zones without any player are frozen.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ko_protocol::{Opcode, Packet};
use parking_lot::{Mutex, RwLock};
use rand::Rng;
use tracing::{debug, info, warn};

use crate::handler::chat::{build_chat_packet, ChatType};
use crate::magic_constants::{MAGIC_CASTING, MAGIC_EFFECTING};
use crate::npc::{NpcId, NpcInstance, NpcTemplate};
use crate::systems::bot_ai;
use crate::systems::pathfind::nearest_bot_point;
use crate::world::{
    BotAiState, BotId, BotInstance, BotPresence, NpcState, WorldState, BOT_ID_BASE,
    NATION_ELMORAD, NATION_KARUS,
};
use crate::zone::{calc_region, SessionId};

// ── Caps and tuning ──────────────────────────────────────────────────────

/// Technical safety ceiling: field bots of one nation in one zone.
pub const MAX_FIELD_BOTS_PER_NATION_ZONE: u16 = 1000;
/// Technical safety ceiling: field bots in the whole world (the bot-id range is 5000).
pub const MAX_FIELD_BOTS_TOTAL: usize = 5000;
/// Field bots a single real party may hold (the rest of the 8 slots stay for people).
pub const MAX_BOTS_PER_PARTY: usize = 6;
/// Highest level a bot can be configured to.
pub const MAX_BOT_LEVEL: u8 = 83;
/// A bot used to stop at this share of a monster's HP so a player always took the last hit; bots now finish
/// monsters for their party like a normal player, so the floor is 0.
pub const ASSIST_FLOOR_PERCENT: i32 = 0;

const TICK_MS: u64 = 250;
const MANAGE_INTERVAL_MS: u64 = 2_000;
const SPAWNS_PER_MANAGE: usize = 3;
const DESPAWNS_PER_MANAGE: usize = 12;
/// Work budget of one [`tick`] call.
const TICK_BUDGET_MS: u64 = 25;
/// Prey scans (3x3 region sweeps) allowed per tick, shared by all bots.
const SCANS_PER_TICK: u32 = 6;
const CONFIG_REFRESH_MS: u64 = 60_000;
const STATUS_WRITE_MS: u64 = 10_000;

const HUNT_SEARCH_RANGE: f32 = 40.0;
const LEASH_RADIUS: f32 = 48.0;
/// Do not take a monster that stands this close to a real (non-party) player.
pub const POLITE_RADIUS: f32 = 14.0;
const PARTY_ASSIST_RADIUS: f32 = 42.0;
const PARTY_FOLLOW_TRIGGER: f32 = 9.0;
const PARTY_WARP_DISTANCE: f32 = 70.0;
const PARTY_INVITE_RANGE: f32 = 75.0;
/// Monsters that take longer than this to respawn (mini bosses, captains, rare spawns: up to
/// four hours in the live data) are never touched by bots -- they belong to the players.
const MAX_PREY_REGEN_MS: u64 = 120_000;
const CLAIM_TTL_MS: u64 = 20_000;
const BLOCK_TTL_MS: u64 = 25_000;
const MOB_REACH_MELEE: f32 = 4.5;
const MOB_REACH_RANGED: f32 = 18.0;
/// Simulated monster retaliation can never push a bot below this share of its HP.
const RETALIATION_FLOOR_PERCENT: i32 = 12;
const REST_ENTER_HP_PERCENT: i32 = 40;
const REST_LEAVE_HP_PERCENT: i32 = 85;
const MELEE_COOLDOWN_MS: u64 = 1_500;
const RANGED_COOLDOWN_MS: u64 = 2_000;
const GROUND_BIN: f32 = 64.0;
const GROUND_TTL_MS: u64 = 180_000;
const REPLY_QUEUE_CAP: usize = 48;
const BOT_CHAT_COOLDOWN_MS: u64 = 7_000;
const GLOBAL_CHAT_GAP_MS: u64 = 600;
const CHAT_HEAR_RANGE: f32 = 26.0;

/// Order of the class-mix weights: warrior, rogue, mage, priest, kurian/porutu.
pub const DEFAULT_MIX: [u8; 5] = [30, 20, 25, 25, 0];
/// GM shorthand classes understood by `spawn_gm_bot` (same order as the mix).
const MIX_GM_CLASSES: [u16; 5] = [1, 2, 3, 4, 13];

// ── Zone profiles ────────────────────────────────────────────────────────

/// Which nation a normal map belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneOwner {
    Neutral,
    Karus,
    Elmorad,
}

impl ZoneOwner {
    /// May a bot of `nation` live in a zone with this owner?
    pub fn allows(self, nation: u8) -> bool {
        match self {
            ZoneOwner::Neutral => nation == NATION_KARUS || nation == NATION_ELMORAD,
            ZoneOwner::Karus => nation == NATION_KARUS,
            ZoneOwner::Elmorad => nation == NATION_ELMORAD,
        }
    }
}

/// A normal map field bots may be configured for.
#[derive(Clone, Copy, Debug)]
pub struct ZoneProfile {
    pub zone_id: u16,
    pub name: &'static str,
    pub owner: ZoneOwner,
    /// Lowest level the zone itself admits (`zone_info.min_level`).
    pub min_level: u8,
}

/// The supported normal maps (PK maps 71-73 stay with the PK bot system).
pub const ZONE_PROFILES: [ZoneProfile; 5] = [
    ZoneProfile { zone_id: 1, name: "Luferson (Karus)", owner: ZoneOwner::Karus, min_level: 35 },
    ZoneProfile { zone_id: 2, name: "El Morad", owner: ZoneOwner::Elmorad, min_level: 35 },
    ZoneProfile { zone_id: 11, name: "Karus Eslant", owner: ZoneOwner::Karus, min_level: 60 },
    ZoneProfile { zone_id: 12, name: "El Morad Eslant", owner: ZoneOwner::Elmorad, min_level: 60 },
    ZoneProfile { zone_id: 21, name: "Moradon", owner: ZoneOwner::Neutral, min_level: 1 },
];

pub fn zone_profile(zone_id: u16) -> Option<&'static ZoneProfile> {
    ZONE_PROFILES.iter().find(|p| p.zone_id == zone_id)
}

// ── Configuration ────────────────────────────────────────────────────────

/// One `field_bot_zone` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldZoneConfig {
    pub zone_id: u16,
    pub enabled: bool,
    pub karus_count: u16,
    pub elmorad_count: u16,
    pub level_min: u8,
    pub level_max: u8,
    /// warrior, rogue, mage, priest, kurian weights (0..=100 each).
    pub mix: [u8; 5],
}

impl FieldZoneConfig {
    /// Clamp the row to the supported ranges. `None` for unsupported zones.
    pub fn sanitized(&self) -> Option<FieldZoneConfig> {
        let profile = zone_profile(self.zone_id)?;
        let mut c = self.clone();
        c.level_min = c.level_min.clamp(profile.min_level.max(1), MAX_BOT_LEVEL);
        c.level_max = c.level_max.clamp(c.level_min, MAX_BOT_LEVEL);
        c.karus_count = if profile.owner.allows(NATION_KARUS) {
            c.karus_count.min(MAX_FIELD_BOTS_PER_NATION_ZONE)
        } else {
            0
        };
        c.elmorad_count = if profile.owner.allows(NATION_ELMORAD) {
            c.elmorad_count.min(MAX_FIELD_BOTS_PER_NATION_ZONE)
        } else {
            0
        };
        for w in &mut c.mix {
            *w = (*w).min(100);
        }
        if c.mix.iter().all(|w| *w == 0) {
            c.mix = DEFAULT_MIX;
        }
        Some(c)
    }

    /// Target population of `nation` (0 while disabled).
    pub fn target_for(&self, nation: u8) -> usize {
        if !self.enabled {
            return 0;
        }
        match nation {
            NATION_KARUS => self.karus_count as usize,
            NATION_ELMORAD => self.elmorad_count as usize,
            _ => 0,
        }
    }
}

// ── Runtime state ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Hunt,
    Rest,
}

/// Per-bot runtime state (the `BotInstance` itself lives in `WorldState::bots`).
#[derive(Clone, Debug)]
struct FieldBotRt {
    /// Centre of the hunting ground this bot is attached to.
    home: (f32, f32),
    mode: Mode,
    claim: Option<NpcId>,
    next_scan_ms: u64,
    roam_goal: Option<(f32, f32)>,
    roam_until_ms: u64,
    idle_until_ms: u64,
    /// Next simulated monster hit on this bot.
    mob_hit_ms: u64,
    /// Stuck detection: time of the last progress sample.
    stuck_since_ms: u64,
    stuck_hits: u8,
    blocked: Vec<(NpcId, u64)>,
    /// Stable personal offset so several bots on one monster spread around it.
    angle: f32,
    party_id: Option<u16>,
    party_check_ms: u64,
    in_party: bool,
    /// Walking to its ring slot around the party leader (hysteresis so bots settle, not jitter).
    following: bool,
    leader_missing_since_ms: u64,
    no_prey_scans: u16,
    last_rest_tick_ms: u64,
    last_chat_ms: u64,
    was_inactive: bool,
}

impl FieldBotRt {
    fn new(home: (f32, f32), angle: f32, now_ms: u64) -> Self {
        Self {
            home,
            mode: Mode::Hunt,
            claim: None,
            next_scan_ms: now_ms,
            roam_goal: None,
            roam_until_ms: 0,
            idle_until_ms: 0,
            mob_hit_ms: 0,
            stuck_since_ms: now_ms,
            stuck_hits: 0,
            blocked: Vec::new(),
            angle,
            party_id: None,
            party_check_ms: 0,
            in_party: false,
            following: false,
            leader_missing_since_ms: 0,
            no_prey_scans: 0,
            last_rest_tick_ms: now_ms,
            last_chat_ms: 0,
            was_inactive: false,
        }
    }
}

#[derive(Clone, Debug)]
struct Mob {
    x: f32,
    z: f32,
    level: u8,
}

/// A cluster of monster spawns (one 64x64 map bin).
#[derive(Clone, Debug)]
struct Ground {
    mobs: Vec<Mob>,
}

impl Ground {
    fn in_band(&self, level: u8) -> impl Iterator<Item = &Mob> {
        self.mobs.iter().filter(move |m| level_in_band(level, m.level))
    }
}

#[derive(Clone, Copy, Debug)]
enum Channel {
    General,
    Whisper(SessionId),
    Party(u16),
}

#[derive(Clone, Debug)]
struct PendingReply {
    bot_id: BotId,
    deliver_at_ms: u64,
    text: String,
    channel: Channel,
}

/// Everything the field-bot system keeps per world.
#[derive(Default)]
pub struct FieldBotRegistry {
    config: RwLock<HashMap<u16, FieldZoneConfig>>,
    rt: Mutex<HashMap<BotId, FieldBotRt>>,
    claims: Mutex<HashMap<NpcId, (BotId, u64)>>,
    replies: Mutex<VecDeque<PendingReply>>,
    grounds: Mutex<HashMap<u16, (u64, Arc<Vec<Ground>>)>>,
    hold: Mutex<HashSet<BotId>>,
    warned: Mutex<HashMap<String, u64>>,
    last_manage_ms: AtomicU64,
    last_global_chat_ms: AtomicU64,
    cursor: AtomicUsize,
    /// Why a zone could not (fully) spawn its bots, shown in the admin panel ("why 0").
    spawn_note: Mutex<HashMap<u16, String>>,
}

impl FieldBotRegistry {
    /// Current sanitized configuration (all zones, sorted by zone id).
    pub fn config_snapshot(&self) -> Vec<FieldZoneConfig> {
        let mut v: Vec<FieldZoneConfig> = self.config.read().values().cloned().collect();
        v.sort_by_key(|c| c.zone_id);
        v
    }

    /// Replace the configuration. Unsupported zones are dropped, every row is sanitized.
    pub fn apply_config(&self, rows: impl IntoIterator<Item = FieldZoneConfig>) -> usize {
        let map: HashMap<u16, FieldZoneConfig> = rows
            .into_iter()
            .filter_map(|r| r.sanitized())
            .map(|c| (c.zone_id, c))
            .collect();
        let n = map.len();
        *self.config.write() = map;
        // Re-evaluate the population at the next tick instead of waiting for the interval.
        self.last_manage_ms.store(0, Ordering::Relaxed);
        n
    }

    fn set_note(&self, zone_id: u16, note: Option<String>) {
        let mut n = self.spawn_note.lock();
        match note {
            Some(t) => {
                n.insert(zone_id, t);
            }
            None => {
                n.remove(&zone_id);
            }
        }
    }

    /// Last spawn problem of a zone, if any.
    pub fn note_for(&self, zone_id: u16) -> Option<String> {
        self.spawn_note.lock().get(&zone_id).cloned()
    }

    fn warn_once(&self, key: String, now_ms: u64, msg: &str) {
        let mut w = self.warned.lock();
        if w.len() > 64 {
            w.retain(|_, at| now_ms.saturating_sub(*at) < 300_000);
        }
        if w.get(&key).is_none_or(|at| now_ms.saturating_sub(*at) >= 300_000) {
            w.insert(key, now_ms);
            warn!("{msg}");
        }
    }
}

// ── Public predicates ────────────────────────────────────────────────────

/// Is this bot one of ours?
pub fn is_field_bot(bot: &BotInstance) -> bool {
    bot.ai_state == BotAiState::Field || bot.original_ai_state == BotAiState::Field
}

/// May the player `attacker_sid` damage this bot? Always `true` for non-field bots (their own
/// rules apply). Field bots can only be hurt by an enemy-nation player in a zone where attacking
/// the other nation is allowed; a GM can always hit them. Nobody is ever *paid* for it.
pub fn player_can_hurt(world: &WorldState, attacker_sid: SessionId, bot: &BotInstance) -> bool {
    if !is_field_bot(bot) {
        return true;
    }
    let Some(ch) = world.get_character_info(attacker_sid) else {
        return false;
    };
    if ch.authority == 0 {
        return true;
    }
    if ch.nation == bot.nation {
        return false;
    }
    world
        .get_zone(bot.zone_id)
        .is_some_and(|zone| zone.can_attack_other_nation())
}

/// Find a living field bot by (case-insensitive) character name.
pub fn find_field_bot_by_name(world: &WorldState, name: &str) -> Option<BotInstance> {
    if name.is_empty() || name.len() > 32 {
        return None;
    }
    world
        .bots
        .iter()
        .find(|e| {
            let b = e.value();
            is_field_bot(b) && b.in_game && b.name.eq_ignore_ascii_case(name)
        })
        .map(|e| e.value().clone())
}

/// Free (partyless) field bots of `nation` as seeking-party list rows for the party BBS window.
pub fn seeking_entries(world: &WorldState, nation: u8) -> Vec<crate::world::SeekingPartyUser> {
    let mut rows: Vec<crate::world::SeekingPartyUser> = world
        .bots
        .iter()
        .filter(|e| {
            let b = e.value();
            is_field_bot(b)
                && b.in_game
                && b.is_alive()
                && (nation == 0 || b.nation == nation)
                && world.get_party_id(b.id as SessionId).is_none()
        })
        .map(|e| {
            let b = e.value();
            crate::world::SeekingPartyUser {
                sid: b.id as u16,
                class: b.class,
                is_party_leader: 0,
                level: b.level as i16,
                zone: b.zone_id as u8,
                seeking_note: "Party araniyor".to_string(), // the client reads at most 40 ASCII bytes per note
                name: b.name.clone(),
                nation: b.nation,
                party_id: 0,
                seek_type: 0,
                login_type: 0,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.zone.cmp(&b.zone).then(b.level.cmp(&a.level)).then_with(|| a.name.cmp(&b.name)));
    rows
}

/// Number of field bots per (zone, nation), for the admin status columns.
pub fn live_counts(world: &WorldState) -> HashMap<(u16, u8), usize> {
    let mut out = HashMap::new();
    for e in world.bots.iter() {
        let b = e.value();
        if is_field_bot(b) {
            *out.entry((b.zone_id, b.nation)).or_insert(0) += 1;
        }
    }
    out
}

// ── Small helpers ────────────────────────────────────────────────────────

fn dist2(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    let (dx, dz) = (ax - bx, az - bz);
    dx * dx + dz * dz
}

fn level_in_band(bot_level: u8, mob_level: u8) -> bool {
    mob_level >= bot_level.saturating_sub(8) && mob_level <= bot_level.saturating_add(4)
}

/// Nearest walkable tile (the zone map decides). Zones without map data accept the point.
fn walkable_near(world: &WorldState, zone_id: u16, x: f32, z: f32, radius: i32) -> Option<(f32, f32)> {
    let zone = world.get_zone(zone_id)?;
    match zone.map_data.as_ref() {
        Some(map) => nearest_bot_point(map, x, z, radius),
        None => Some((x, z)),
    }
}

fn terrain_y(world: &WorldState, zone_id: u16, x: f32, z: f32, fallback: f32) -> f32 {
    world
        .get_zone(zone_id)
        .and_then(|zone| zone.map_data.as_ref().and_then(|m| m.terrain_height(x, z)))
        .unwrap_or(fallback)
}

fn class_name(bot: &BotInstance, turkish: bool) -> &'static str {
    match (bot_ai::bot_class_group(bot.class), turkish) {
        (1, true) => "Savaşçı",
        (1, false) => "Warrior",
        (2, _) => "Rogue",
        (3, true) => "Büyücü",
        (3, false) => "Mage",
        (4, true) => "Rahip",
        (4, false) => "Priest",
        (_, true) => "Kurian",
        (_, false) => "Kurian",
    }
}

fn pick_class(mix: &[u8; 5], rng: &mut impl Rng) -> u16 {
    let total: u32 = mix.iter().map(|w| *w as u32).sum();
    if total == 0 {
        return MIX_GM_CLASSES[0];
    }
    let mut roll = rng.gen_range(0..total);
    for (i, w) in mix.iter().enumerate() {
        if roll < *w as u32 {
            return MIX_GM_CLASSES[i];
        }
        roll -= *w as u32;
    }
    MIX_GM_CLASSES[0]
}

// ── Hunting grounds ──────────────────────────────────────────────────────

/// Is `npc` a monster field bots may fight? (Normal monsters only: no bosses, event, quest or
/// scripted spawns.)
fn prey_template(world: &WorldState, npc: &NpcInstance) -> Option<Arc<NpcTemplate>> {
    if !npc.is_monster
        || npc.is_event_npc
        || npc.event_room != 0
        || npc.summon_type != 0
        || npc.special_type != 0
    {
        return None;
    }
    let tmpl = world.get_npc_template(npc.proto_id, true)?;
    // "[Captine] ..." and "[Quest] ..." templates are special spawns.
    if tmpl.npc_type != 0 || tmpl.level == 0 || tmpl.max_hp == 0 || tmpl.name.starts_with('[') {
        return None;
    }
    Some(tmpl)
}

fn grounds_for(world: &WorldState, zone_id: u16, now_ms: u64) -> Arc<Vec<Ground>> {
    let reg = &world.field_bots;
    if let Some((built, g)) = reg.grounds.lock().get(&zone_id) {
        if now_ms.saturating_sub(*built) < GROUND_TTL_MS {
            return g.clone();
        }
    }
    let mut bins: HashMap<(i32, i32), Vec<Mob>> = HashMap::new();
    for npc in world.npcs_near(zone_id, 0.0, 0.0, f32::MAX, 0) {
        let Some(tmpl) = prey_template(world, &npc) else {
            continue;
        };
        let ordinary = world
            .with_npc_ai(npc.nid, |ai| (1..=MAX_PREY_REGEN_MS).contains(&ai.regen_time_ms))
            .unwrap_or(false);
        if !ordinary {
            continue;
        }
        let key = ((npc.x / GROUND_BIN) as i32, (npc.z / GROUND_BIN) as i32);
        bins.entry(key).or_default().push(Mob {
            x: npc.x,
            z: npc.z,
            level: tmpl.level,
        });
    }
    let mut keys: Vec<(i32, i32)> = bins.keys().copied().collect();
    keys.sort_unstable();
    let grounds: Vec<Ground> = keys
        .into_iter()
        .filter_map(|k| bins.remove(&k))
        .map(|mobs| Ground { mobs })
        .collect();
    let arc = Arc::new(grounds);
    reg.grounds.lock().insert(zone_id, (now_ms, arc.clone()));
    arc
}

/// Pick a hunting ground for a bot of `level`: returns its anchor (centroid of the in-band
/// monsters). Grounds with few existing bots are preferred so the population spreads out.
fn choose_ground(
    world: &WorldState,
    zone_id: u16,
    level: u8,
    now_ms: u64,
    occupied: &[(f32, f32)],
    rng: &mut impl Rng,
) -> Option<(f32, f32)> {
    let grounds = grounds_for(world, zone_id, now_ms);
    let mut best: Vec<((f32, f32), f32)> = Vec::new();
    for g in grounds.iter() {
        let band: Vec<&Mob> = g.in_band(level).collect();
        if band.len() < 3 {
            continue;
        }
        let n = band.len() as f32;
        let cx = band.iter().map(|m| m.x).sum::<f32>() / n;
        let cz = band.iter().map(|m| m.z).sum::<f32>() / n;
        let crowd = occupied
            .iter()
            .filter(|(ox, oz)| dist2(*ox, *oz, cx, cz) < LEASH_RADIUS * LEASH_RADIUS)
            .count() as f32;
        best.push(((cx, cz), n / (1.0 + crowd * 1.5)));
    }
    if best.is_empty() {
        return None;
    }
    let total: f32 = best.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.gen_range(0.0..total.max(0.001));
    for (anchor, w) in &best {
        if roll < *w {
            return Some(*anchor);
        }
        roll -= *w;
    }
    best.last().map(|(a, _)| *a)
}

// ── Spawning / population management ─────────────────────────────────────

fn field_bot_ids(world: &WorldState) -> Vec<BotId> {
    let mut ids: Vec<BotId> = world
        .bots
        .iter()
        .filter(|e| is_field_bot(e.value()))
        .map(|e| *e.key())
        .collect();
    ids.sort_unstable();
    ids
}

fn spawn_one(
    world: &WorldState,
    cfg: &FieldZoneConfig,
    nation: u8,
    now_ms: u64,
    occupied: &[(f32, f32)],
) -> Option<BotId> {
    let mut rng = rand::thread_rng();
    let class = pick_class(&cfg.mix, &mut rng);
    // Level first, then a ground that has monsters for it; a few tries before giving up.
    let mut picked: Option<(u8, (f32, f32))> = None;
    for _ in 0..6 {
        let level = rng.gen_range(cfg.level_min..=cfg.level_max);
        if let Some(anchor) = choose_ground(world, cfg.zone_id, level, now_ms, occupied, &mut rng) {
            picked = Some((level, anchor));
            break;
        }
    }
    let Some((level, anchor)) = picked else {
        world.field_bots.set_note(
            cfg.zone_id,
            Some(format!("Seviye {}-{} icin uygun canavar grubu yok", cfg.level_min, cfg.level_max)),
        );
        world.field_bots.warn_once(
            format!("noground:{}", cfg.zone_id),
            now_ms,
            &format!(
                "field bots: zone {} has no monster group for levels {}-{}; nothing spawned",
                cfg.zone_id, cfg.level_min, cfg.level_max
            ),
        );
        return None;
    };
    let (jx, jz) = (rng.gen_range(-6.0..6.0f32), rng.gen_range(-6.0..6.0f32));
    let Some((x, z)) = walkable_near(world, cfg.zone_id, anchor.0 + jx, anchor.1 + jz, 12)
        .or_else(|| walkable_near(world, cfg.zone_id, anchor.0, anchor.1, 24))
    else {
        world.field_bots.set_note(cfg.zone_id, Some("Av alaninda yurunebilir zemin bulunamadi".to_string()));
        world.field_bots.warn_once(
            format!("nowalk:{}", cfg.zone_id),
            now_ms,
            &format!("field bots: zone {} has no walkable tile near its hunting ground; nothing spawned", cfg.zone_id),
        );
        return None;
    };
    world.field_bots.set_note(cfg.zone_id, None);
    let y = terrain_y(world, cfg.zone_id, x, z, 0.0);
    let id = bot_ai::spawn_gm_bot(
        world,
        bot_ai::SpawnGmBotParams {
            zone_id: cfg.zone_id,
            x,
            y,
            z,
            class,
            level,
            nation,
            ai_state: BotAiState::Field,
        },
    );
    finish_spawn(world, id, anchor, rng.gen_range(-0.9..0.9f32), now_ms);
    debug!(bot_id = id, zone = cfg.zone_id, level, class, nation, "field bot spawned");
    Some(id)
}

/// Turn a freshly spawned GM bot into a registered field bot.
fn finish_spawn(world: &WorldState, id: BotId, home: (f32, f32), angle: f32, now_ms: u64) {
    world.update_bot(id, |b| {
        // Field bots are permanent (the GM default expires after 60 minutes), carry no NP
        // balance of their own and advertise "looking for a party".
        b.duration_minutes = 0;
        b.loyalty = 0;
        b.loyalty_monthly = 0;
        b.loyalty_daily = 0;
        b.need_party = 1;
    });
    world.field_bots.rt.lock().insert(id, FieldBotRt::new(home, angle, now_ms));
}

/// Bring every zone's population in line with the configuration. Bounded per call.
pub fn manage(world: &WorldState, now_ms: u64) {
    let reg = &world.field_bots;
    reg.last_manage_ms.store(now_ms, Ordering::Relaxed);
    let cfgs = reg.config_snapshot();
    let ids = field_bot_ids(world);

    // Forget runtime state of bots that no longer exist.
    {
        let alive: HashSet<BotId> = ids.iter().copied().collect();
        reg.rt.lock().retain(|id, _| alive.contains(id));
        reg.hold.lock().retain(|id| alive.contains(id));
        reg.claims
            .lock()
            .retain(|_, (bot, until)| alive.contains(bot) && *until > now_ms);
    }

    // Count current population and remember positions (for spreading new bots).
    let mut by_zone_nation: HashMap<(u16, u8), Vec<BotId>> = HashMap::new();
    let mut positions: HashMap<u16, Vec<(f32, f32)>> = HashMap::new();
    for id in &ids {
        if let Some(b) = world.get_bot(*id) {
            by_zone_nation.entry((b.zone_id, b.nation)).or_default().push(*id);
            positions.entry(b.zone_id).or_default().push((b.x, b.z));
        }
    }

    let mut despawned = 0usize;
    let mut spawned = 0usize;
    let mut total = ids.len();

    // Zones that are not (or no longer) enabled lose all their bots.
    let enabled: HashMap<u16, &FieldZoneConfig> =
        cfgs.iter().filter(|c| c.enabled).map(|c| (c.zone_id, c)).collect();
    for ((zone, _), list) in by_zone_nation.iter() {
        if enabled.contains_key(zone) {
            continue;
        }
        for id in list {
            if despawned >= DESPAWNS_PER_MANAGE {
                break;
            }
            bot_ai::despawn_bot(world, *id);
            despawned += 1;
            total = total.saturating_sub(1);
        }
    }

    for cfg in cfgs.iter().filter(|c| c.enabled) {
        for nation in [NATION_KARUS, NATION_ELMORAD] {
            let target = cfg.target_for(nation);
            let have = by_zone_nation
                .get(&(cfg.zone_id, nation))
                .cloned()
                .unwrap_or_default();
            if have.len() > target {
                // Trim the excess, bots that are not in a party first.
                let mut extra: Vec<BotId> = have.clone();
                extra.sort_by_key(|id| world.get_party_id(*id as SessionId).is_some());
                for id in extra.into_iter().take(have.len() - target) {
                    if despawned >= DESPAWNS_PER_MANAGE {
                        break;
                    }
                    bot_ai::despawn_bot(world, id);
                    despawned += 1;
                    total = total.saturating_sub(1);
                }
            } else if have.len() < target {
                let room_global = MAX_FIELD_BOTS_TOTAL.saturating_sub(total);
                let room_capacity = 4_900usize.saturating_sub(world.bot_count());
                if room_capacity == 0 {
                    reg.set_note(cfg.zone_id, Some("Bot kimlik araligi dolu".to_string()));
                }
                let want = (target - have.len())
                    .min(room_global)
                    .min(room_capacity)
                    .min(SPAWNS_PER_MANAGE.saturating_sub(spawned));
                for _ in 0..want {
                    let occupied = positions.get(&cfg.zone_id).cloned().unwrap_or_default();
                    if let Some(id) = spawn_one(world, cfg, nation, now_ms, &occupied) {
                        spawned += 1;
                        total += 1;
                        if let Some(b) = world.get_bot(id) {
                            positions.entry(cfg.zone_id).or_default().push((b.x, b.z));
                        }
                    }
                }
            }
        }
    }
    if spawned > 0 || despawned > 0 {
        info!(spawned, despawned, total, "field bot population adjusted");
    }
}

/// Remove every field bot of `zone_id` (all zones when `None`). They respawn at the next
/// management pass for zones that are still enabled.
pub fn clear(world: &WorldState, zone_id: Option<u16>) -> usize {
    let ids: Vec<BotId> = world
        .bots
        .iter()
        .filter(|e| is_field_bot(e.value()) && zone_id.is_none_or(|z| e.value().zone_id == z))
        .map(|e| *e.key())
        .collect();
    for id in &ids {
        bot_ai::despawn_bot(world, *id);
    }
    world.field_bots.last_manage_ms.store(0, Ordering::Relaxed);
    ids.len()
}

// ── Packets and world mutations ──────────────────────────────────────────

fn out_packet(bot_id: BotId) -> Packet {
    let mut p = Packet::new(Opcode::WizUserInout as u8);
    p.write_u8(2); // INOUT_OUT
    p.write_u8(0);
    p.write_u32(bot_id);
    p
}

/// Tell observers who gained/lost sight of the bot because it crossed a region border. PK bots
/// skip this (they fight inside one crowd); a wandering hunter must not pop in and out.
fn announce_region_change(world: &WorldState, bot: &BotInstance, old_rx: u16, old_rz: u16) {
    let Some(zone) = world.get_zone(bot.zone_id) else {
        return;
    };
    let near = |cx: i32, cz: i32, rx: u16, rz: u16| {
        (cx - rx as i32).abs() <= 1 && (cz - rz as i32).abs() <= 1
    };
    let in_pkt = bot_ai::build_bot_inout_packet(bot, world, 1);
    let out_pkt = out_packet(bot.id);
    let (nrx, nrz) = (bot.region_x, bot.region_z);
    for cx in (old_rx.min(nrx) as i32 - 1)..=(old_rx.max(nrx) as i32 + 1) {
        for cz in (old_rz.min(nrz) as i32 - 1)..=(old_rz.max(nrz) as i32 + 1) {
            if cx < 0 || cz < 0 {
                continue;
            }
            let was = near(cx, cz, old_rx, old_rz);
            let is = near(cx, cz, nrx, nrz);
            if was == is {
                continue;
            }
            for sid in zone.get_users_in_region(cx as u16, cz as u16) {
                world.send_to_session_owned(sid, if is { in_pkt.clone() } else { out_pkt.clone() });
            }
        }
    }
}

/// Walk one step: broadcast it, update the bot and fix up observers on a region change.
fn commit_move(world: &WorldState, bot: &BotInstance, nx: f32, ny: f32, nz: f32, echo: u8, now_ms: u64) {
    bot_ai::broadcast_bot_move_ex(world, bot, nx, ny, nz, echo);
    let (nrx, nrz) = (calc_region(nx), calc_region(nz));
    world.update_bot(bot.id, |b| {
        b.x = nx;
        b.y = ny;
        b.z = nz;
        b.region_x = nrx;
        b.region_z = nrz;
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
    });
    if nrx != bot.region_x || nrz != bot.region_z {
        if let Some(moved) = world.get_bot(bot.id) {
            announce_region_change(world, &moved, bot.region_x, bot.region_z);
        }
    }
}

/// Teleport a bot (used for stuck recovery, respawn and far-away party followers).
fn warp_bot(world: &WorldState, bot_id: BotId, x: f32, z: f32) {
    let Some(bot) = world.get_bot(bot_id) else {
        return;
    };
    let y = terrain_y(world, bot.zone_id, x, z, bot.y);
    bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &out_packet(bot_id));
    let (rx, rz) = (calc_region(x), calc_region(z));
    world.update_bot(bot_id, |b| {
        b.x = x;
        b.y = y;
        b.z = z;
        b.region_x = rx;
        b.region_z = rz;
    });
    if let Some(moved) = world.get_bot(bot_id) {
        let pkt = bot_ai::build_bot_inout_packet(&moved, world, 3);
        bot_ai::broadcast_to_bot_region(world, moved.zone_id, rx, rz, &pkt);
    }
}

fn set_sitting(world: &WorldState, bot: &BotInstance, sitting: bool) {
    let presence = if sitting { BotPresence::Sitting } else { BotPresence::Standing };
    if bot.presence == presence {
        return;
    }
    world.update_bot(bot.id, |b| b.presence = presence);
    let pkt = crate::handler::regene::build_state_change_broadcast(bot.id, 1, if sitting { 2 } else { 1 });
    bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &pkt);
}

fn set_need_party(world: &WorldState, bot: &BotInstance, need: bool) {
    world.update_bot(bot.id, |b| b.need_party = need as u8);
    let pkt = crate::handler::regene::build_state_change_broadcast(bot.id, 2, need as u32);
    bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &pkt);
}

// ── Chat output ──────────────────────────────────────────────────────────

fn chat_packet(bot: &BotInstance, chat_type: ChatType, text: &str) -> Packet {
    build_chat_packet(chat_type as u8, bot.nation, bot.id as u16, &bot.name, text, -1, 1, 1)
}

fn say_general(world: &WorldState, bot: &BotInstance, text: &str) {
    let pkt = chat_packet(bot, ChatType::General, text);
    bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &pkt);
}

fn say_party(world: &WorldState, party_id: u16, bot: &BotInstance, text: &str) {
    let pkt = chat_packet(bot, ChatType::Party, text);
    world.send_to_party(party_id, &pkt);
}

fn say_whisper(world: &WorldState, bot: &BotInstance, to: SessionId, text: &str) {
    let pkt = chat_packet(bot, ChatType::Private, text);
    world.send_to_session_owned(to, pkt);
}

fn queue_reply(world: &WorldState, bot_id: BotId, delay_ms: u64, text: String, channel: Channel, now_ms: u64) {
    let mut q = world.field_bots.replies.lock();
    if q.len() >= REPLY_QUEUE_CAP {
        return;
    }
    q.push_back(PendingReply {
        bot_id,
        deliver_at_ms: now_ms + delay_ms,
        text,
        channel,
    });
}

fn deliver_replies(world: &WorldState, now_ms: u64) {
    let due: Vec<PendingReply> = {
        let mut q = world.field_bots.replies.lock();
        if q.is_empty() {
            return;
        }
        let mut due = Vec::new();
        let mut keep = VecDeque::with_capacity(q.len());
        while let Some(r) = q.pop_front() {
            if r.deliver_at_ms <= now_ms {
                due.push(r);
            } else {
                keep.push_back(r);
            }
        }
        *q = keep;
        due
    };
    for r in due {
        let Some(bot) = world.get_bot(r.bot_id) else {
            continue;
        };
        if !bot.is_alive() {
            continue;
        }
        match r.channel {
            Channel::General => say_general(world, &bot, &r.text),
            Channel::Whisper(to) => say_whisper(world, &bot, to, &r.text),
            Channel::Party(pid) => {
                if world.get_party_id(bot.id as SessionId) == Some(pid) {
                    say_party(world, pid, &bot, &r.text);
                }
            }
        }
    }
}

// ── Combat helpers ───────────────────────────────────────────────────────

/// How many bot hits an equal-level monster takes. Deliberately independent of the bot's gear:
/// the same table applies to every class so a level-40 field bot behaves like a level-40
/// player without a gear-quality lottery.
pub fn hits_to_kill(bot_level: u8, mob_level: u8) -> f32 {
    let diff = mob_level as i32 - bot_level as i32;
    let hits = 9.0 + diff.max(0) as f32 * 1.2 - (-diff).max(0) as f32 * 0.4;
    hits.clamp(3.0, 40.0)
}

fn class_damage_factor(bot: &BotInstance) -> f32 {
    match bot_ai::bot_class_group(bot.class) {
        2 => 1.1,
        3 => 1.15,
        4 => 0.7,
        _ => 1.0,
    }
}

fn bot_hit_damage(bot: &BotInstance, tmpl: &NpcTemplate, rng: &mut impl Rng) -> i32 {
    let base = tmpl.max_hp as f32 / hits_to_kill(bot.level, tmpl.level);
    // A mild gear/stat influence: well-geared bots hit a little harder than naked ones.
    let stat = (bot_ai::calculate_bot_damage(bot) as f32 / (bot.level.max(1) as f32 * 14.0)).clamp(0.6, 1.25);
    let jitter = rng.gen_range(0.8..1.2f32);
    ((base * stat * class_damage_factor(bot) * jitter).round() as i32).max(1)
}

fn attack_cooldown_ms(bot: &BotInstance, ranged: bool) -> u64 {
    if ranged || bot.is_mage() || bot.is_priest() {
        RANGED_COOLDOWN_MS
    } else {
        MELEE_COOLDOWN_MS
    }
}

#[derive(Clone)]
struct PreyView {
    npc: Arc<NpcInstance>,
    tmpl: Arc<NpcTemplate>,
    x: f32,
    z: f32,
    hp: i32,
    target: Option<SessionId>,
    state: NpcState,
}

fn prey_view(world: &WorldState, id: NpcId) -> Option<PreyView> {
    let npc = world.get_npc_instance(id)?;
    let tmpl = prey_template(world, &npc)?;
    let hp = world.get_npc_hp(id)?;
    if hp <= 0 {
        return None;
    }
    let (state, x, z, target, regen) = world
        .with_npc_ai(id, |a| (a.state, a.cur_x, a.cur_z, a.target_id, a.regen_time_ms))?;
    if !(1..=MAX_PREY_REGEN_MS).contains(&regen) || state == NpcState::Dead {
        return None;
    }
    Some(PreyView { npc, tmpl, x, z, hp, target, state })
}

/// Kill a monster on behalf of a bot: no exp, no loot, no quest credit, no damage rights.
fn finish_prey(world: &WorldState, p: &PreyView) {
    let id = p.npc.nid;
    world.update_npc_hp(id, 0);
    world.update_npc_ai(id, |s| {
        s.state = NpcState::Dead;
        s.target_id = None;
        s.npc_target_id = None;
    });
    world.clear_npc_damage(id);
    let mut death = Packet::new(Opcode::WizDead as u8);
    death.write_u32(id);
    world.broadcast_to_3x3(
        p.npc.zone_id,
        p.npc.region_x,
        p.npc.region_z,
        Arc::new(death),
        None,
        p.npc.event_room,
    );
}

/// Kills made by a party bot, waiting for the async death handler (exp / loot / quest credit for the party).
static PENDING_KILLS: Mutex<Vec<(SessionId, NpcId)>> = Mutex::new(Vec::new());

/// The real party member a bot's kill is credited to: the one with the most damage on the monster, else the
/// first real member in the bot's zone.
fn party_credit_sid(world: &WorldState, bot: &BotInstance, npc_id: NpcId) -> Option<SessionId> {
    let pid = world.get_party_id(bot.id as SessionId)?;
    let party = world.get_party(pid)?;
    let members: Vec<SessionId> = party.active_members().into_iter().filter(|s| is_real(*s)).collect();
    let best = world
        .get_npc_damage_entries(npc_id)
        .into_iter()
        .filter(|(sid, _)| members.contains(sid))
        .max_by_key(|(_, d)| *d)
        .map(|(sid, _)| sid);
    best.or_else(|| {
        members
            .into_iter()
            .find(|sid| world.get_position(*sid).is_some_and(|p| p.zone_id == bot.zone_id) && !world.is_player_dead(*sid))
    })
}

/// A party bot lands the last hit: the monster dies through the normal death path on behalf of `killer`.
fn queue_party_kill(world: &WorldState, killer: SessionId, p: &PreyView) {
    let id = p.npc.nid;
    if world.get_npc_damage_entries(id).iter().all(|(sid, _)| *sid != killer) {
        // the exp split needs at least one real damage entry
        world.record_npc_damage(id, killer, 1);
    }
    world.update_npc_hp(id, 0);
    PENDING_KILLS.lock().push((killer, id));
}

/// Pay out the kills queued by party bots (called from the field bot task).
pub async fn drain_party_kills(world: &WorldState) {
    let kills: Vec<(SessionId, NpcId)> = std::mem::take(&mut *PENDING_KILLS.lock());
    for (killer, id) in kills {
        let Some(npc) = world.get_npc_instance(id) else { continue };
        let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) else { continue };
        if !world.is_npc_dead(id) {
            continue; // protected target: nothing to pay
        }
        crate::handler::attack::handle_npc_death(world, killer, id, &npc, &tmpl, false).await;
    }
}

/// Radius of a Type3 "area enemy" skill, 0 for everything else.
fn area_radius(world: &WorldState, skill: u32) -> f32 {
    let Some(magic) = world.get_magic(skill as i32) else { return 0.0 };
    if magic.moral.unwrap_or(0) != crate::magic_constants::MORAL_AREA_ENEMY || magic.type1.unwrap_or(0) != 3 {
        return 0.0;
    }
    world
        .get_magic_type3(magic.magic_num)
        .and_then(|t| t.radius)
        .map_or(0.0, |r| r as f32)
}

/// Real members of the party `bot` belongs to.
fn party_real_members(world: &WorldState, bot: &BotInstance) -> Vec<SessionId> {
    world
        .get_party_id(bot.id as SessionId)
        .and_then(|pid| world.get_party(pid))
        .map(|p| p.active_members().into_iter().filter(|s| is_real(*s)).collect())
        .unwrap_or_default()
}

/// Area damage around the primary target: every other monster of the group the party is fighting takes 80 %.
fn splash(world: &WorldState, bot: &BotInstance, primary: &PreyView, skill: u32, damage: i32, radius: f32, rng: &mut impl Rng) {
    let Some(zone) = world.get_zone(bot.zone_id) else { return };
    let real = party_real_members(world, bot);
    let r2 = radius.min(14.0) * radius.min(14.0);
    for id in zone.get_npcs_in_3x3(primary.npc.region_x, primary.npc.region_z) {
        if id == primary.npc.nid {
            continue;
        }
        let Some(p) = prey_view(world, id) else { continue };
        if dist2(p.x, p.z, primary.x, primary.z) > r2 {
            continue;
        }
        let engaged = p.target.is_some_and(|t| real.contains(&t)) || real.iter().any(|m| world.npc_damage_contains(id, *m));
        if !engaged {
            continue;
        }
        let dmg = ((damage as f32) * 0.8 * rng.gen_range(0.9..1.1f32)).round().max(1.0) as i32;
        let hp = (p.hp - dmg).max(0);
        world.update_npc_hp(id, hp);
        let effect = bot_ai::build_bot_magic_packet(
            MAGIC_EFFECTING,
            skill,
            bot.id,
            id,
            [bot.x as i32, bot.y as i32, bot.z as i32, -dmg, 0, 0, 0],
        );
        bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &effect);
        if hp <= 0 {
            match party_credit_sid(world, bot, id) {
                Some(killer) => queue_party_kill(world, killer, &p),
                None => finish_prey(world, &p),
            }
        }
    }
}

/// Simulated monster hit on the bot. Monsters ignore bots in their own AI; this keeps the HP bar
/// moving so bots rest and priests heal like players, but it can never kill the bot.
fn mob_retaliate(world: &WorldState, bot: &BotInstance, tmpl: &NpcTemplate, rng: &mut impl Rng) {
    let raw = tmpl.damage.max(tmpl.attack / 4) as f32;
    if raw <= 0.0 {
        return;
    }
    let ac = bot_ai::bot_defense_ac(world, bot).max(0) as f32;
    let dmg = (raw * 240.0 / (ac + 240.0) * 0.5 * rng.gen_range(0.6..1.0f32)).round().max(1.0) as i32;
    let floor = (bot.max_hp as i32 * RETALIATION_FLOOR_PERCENT / 100).max(1);
    world.update_bot(bot.id, |b| {
        b.hp = ((b.hp as i32 - dmg).max(floor.min(b.hp as i32))) as i16;
    });
    bot_ai::broadcast_bot_party_hp(world, bot.id);
}

// ── Party plumbing ───────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct PartyCtx {
    /// Real members present in the bot's zone, alive, in roster order.
    present: Vec<SessionId>,
    /// All real members (any zone), roster order.
    real: Vec<SessionId>,
    /// This bot's index among the party's bots (ring slot).
    slot: usize,
    bots: usize,
}

fn is_real(sid: SessionId) -> bool {
    (sid as u32) < BOT_ID_BASE
}

fn resolve_party(world: &WorldState, bot: &BotInstance, st: &mut FieldBotRt, now_ms: u64) -> Option<PartyCtx> {
    if now_ms.saturating_sub(st.party_check_ms) >= 1_000 || st.party_id.is_none() {
        st.party_check_ms = now_ms;
        st.party_id = world.get_party_id(bot.id as SessionId);
    }
    let pid = st.party_id?;
    let party = world.get_party(pid)?;
    let me = bot.id as SessionId;
    if party.find_slot(me).is_none() {
        st.party_id = None;
        return None;
    }
    let mut present = Vec::new();
    let mut real = Vec::new();
    let mut bots = 0usize;
    let mut slot = 0usize;
    for sid in party.active_members() {
        if is_real(sid) {
            real.push(sid);
            let here = world.get_position(sid).is_some_and(|p| p.zone_id == bot.zone_id)
                && !world.is_player_dead(sid);
            if here {
                present.push(sid);
            }
        } else {
            if sid == me {
                slot = bots;
            }
            bots += 1;
        }
    }
    Some(PartyCtx { present, real, slot, bots })
}

/// Leave the party politely, restoring the "looking for party" flag.
fn leave_party(world: &WorldState, bot: &BotInstance, text: Option<&str>) {
    let me = bot.id as SessionId;
    let Some(pid) = world.get_party_id(me) else {
        return;
    };
    let old_leader = world.get_party(pid).and_then(|p| p.leader_sid());
    if let Some(t) = text {
        say_party(world, pid, bot, t);
    }
    world.cleanup_party_on_disconnect(me);
    if world.get_party(pid).is_none() {
        // Disbanded: the former leader loses the "P" crown.
        if let Some(l) = old_leader.filter(|l| is_real(*l)) {
            if let Some((pos, room)) = world.with_session(l, |h| (h.position, h.event_room)) {
                let sc = crate::handler::regene::build_state_change_broadcast(
                    l as u32,
                    crate::state_change_constants::STATE_CHANGE_PARTY_LEADER,
                    0,
                );
                world.broadcast_to_region_sync(pos.zone_id, pos.region_x, pos.region_z, Arc::new(sc), None, room);
            }
        }
    }
    if let Some(b) = world.get_bot(bot.id) {
        set_need_party(world, &b, true);
    }
}

// ── The per-bot brain ────────────────────────────────────────────────────

struct Ctx<'a> {
    now_ms: u64,
    positions: &'a [(BotId, u16, f32, f32)],
    scans_left: u32,
}

fn pos_of(world: &WorldState, sid: SessionId) -> Option<(f32, f32, f32)> {
    world.get_position(sid).map(|p| (p.x, p.y, p.z))
}

/// Crowded spot? (another field bot within `r`)
fn crowded(ctx: &Ctx<'_>, me: BotId, zone: u16, x: f32, z: f32, r: f32) -> bool {
    ctx.positions
        .iter()
        .any(|(id, zn, ox, oz)| *id != me && *zn == zone && dist2(*ox, *oz, x, z) < r * r)
}

fn release_claim(world: &WorldState, bot_id: BotId, st: &mut FieldBotRt) {
    if let Some(npc) = st.claim.take() {
        let mut claims = world.field_bots.claims.lock();
        if claims.get(&npc).is_some_and(|(owner, _)| *owner == bot_id) {
            claims.remove(&npc);
        }
    }
}

fn try_claim(world: &WorldState, bot_id: BotId, npc: NpcId, now_ms: u64) -> bool {
    let mut claims = world.field_bots.claims.lock();
    if claims.len() > 512 {
        claims.retain(|_, (_, until)| *until > now_ms);
    }
    match claims.get(&npc) {
        Some((owner, until)) if *owner != bot_id && *until > now_ms => false,
        _ => {
            claims.insert(npc, (bot_id, now_ms + CLAIM_TTL_MS));
            true
        }
    }
}

fn block_prey(st: &mut FieldBotRt, npc: NpcId, now_ms: u64) {
    st.blocked.retain(|(_, until)| *until > now_ms);
    if st.blocked.len() >= 8 {
        st.blocked.remove(0);
    }
    st.blocked.push((npc, now_ms + BLOCK_TTL_MS));
}

fn is_blocked(st: &FieldBotRt, npc: NpcId, now_ms: u64) -> bool {
    st.blocked.iter().any(|(id, until)| *id == npc && *until > now_ms)
}

/// Solo prey must be untouched: no damage record, not chasing anybody, not near a player.
fn solo_prey_ok(world: &WorldState, p: &PreyView, players: &[(f32, f32)]) -> bool {
    matches!(p.state, NpcState::Standing | NpcState::Moving)
        && p.target.is_none()
        && !world.npc_has_damage_record(p.npc.nid)
        && !players
            .iter()
            .any(|(px, pz)| dist2(*px, *pz, p.x, p.z) < POLITE_RADIUS * POLITE_RADIUS)
}

fn nearby_players(world: &WorldState, bot: &BotInstance) -> Vec<(f32, f32)> {
    let Some(zone) = world.get_zone(bot.zone_id) else {
        return Vec::new();
    };
    zone.get_users_in_3x3(bot.region_x, bot.region_z)
        .into_iter()
        .filter_map(|sid| world.get_position(sid))
        .filter(|p| p.zone_id == bot.zone_id)
        .map(|p| (p.x, p.z))
        .collect()
}

fn find_solo_prey(world: &WorldState, bot: &BotInstance, st: &FieldBotRt, now_ms: u64) -> Option<PreyView> {
    let zone = world.get_zone(bot.zone_id)?;
    let players = nearby_players(world, bot);
    let mut best: Option<(f32, PreyView)> = None;
    for id in zone.get_npcs_in_3x3(bot.region_x, bot.region_z) {
        if is_blocked(st, id, now_ms) {
            continue;
        }
        if world
            .field_bots
            .claims
            .lock()
            .get(&id)
            .is_some_and(|(owner, until)| *owner != bot.id && *until > now_ms)
        {
            continue;
        }
        let Some(p) = prey_view(world, id) else {
            continue;
        };
        if !level_in_band(bot.level, p.tmpl.level) || !solo_prey_ok(world, &p, &players) {
            continue;
        }
        let from_home = dist2(p.x, p.z, st.home.0, st.home.1);
        let from_bot = dist2(p.x, p.z, bot.x, bot.z);
        if from_home > LEASH_RADIUS * LEASH_RADIUS || from_bot > HUNT_SEARCH_RANGE * HUNT_SEARCH_RANGE {
            continue;
        }
        if best.as_ref().is_none_or(|(d, _)| from_bot < *d) {
            best = Some((from_bot, p));
        }
    }
    best.map(|(_, p)| p)
}

fn find_party_prey(
    world: &WorldState,
    bot: &BotInstance,
    party: &PartyCtx,
    leader: SessionId,
    now_ms: u64,
    st: &FieldBotRt,
) -> Option<PreyView> {
    let (lx, _, lz) = pos_of(world, leader)?;
    let zone = world.get_zone(bot.zone_id)?;
    let (lrx, lrz) = (calc_region(lx), calc_region(lz));
    let mut best: Option<(f32, PreyView)> = None;
    for id in zone.get_npcs_in_3x3(lrx, lrz) {
        if is_blocked(st, id, now_ms) {
            continue;
        }
        let Some(p) = prey_view(world, id) else {
            continue;
        };
        let near_leader = dist2(p.x, p.z, lx, lz);
        if near_leader > PARTY_ASSIST_RADIUS * PARTY_ASSIST_RADIUS {
            continue;
        }
        // Engaged by the party: it hunts a member, or a member has hurt it.
        let engaged = p.target.is_some_and(|t| party.real.contains(&t))
            || party.real.iter().any(|m| world.npc_damage_contains(p.npc.nid, *m));
        if !engaged {
            continue;
        }
        if p.hp <= p.tmpl.max_hp as i32 * ASSIST_FLOOR_PERCENT / 100 {
            continue;
        }
        // closest to the leader first, and a monster that is already hurt (the party is on it) before a fresh one,
        // so the bots finish what the party started instead of spreading over every monster in range
        let hp_pct = p.hp as f32 * 100.0 / (p.tmpl.max_hp as f32).max(1.0);
        let mut score = near_leader.sqrt() + hp_pct * 0.08;
        if world.npc_damage_contains(p.npc.nid, leader) {
            score -= 1000.0; // focus fire: whatever the leader is hitting comes first
        }
        if best.as_ref().is_none_or(|(d, _)| score < *d) {
            best = Some((score, p));
        }
    }
    best.map(|(_, p)| p)
}

enum Engagement {
    /// Walked toward the prey.
    Moved,
    /// Landed a hit (or was waiting for the cooldown in range).
    Fighting,
    /// The fight is over (killed, softened or the prey vanished).
    Over,
    /// No route to the prey.
    Blocked,
}

#[allow(clippy::too_many_arguments)]
fn engage(
    world: &WorldState,
    bot: &BotInstance,
    st: &mut FieldBotRt,
    prey: &PreyView,
    assist: bool,
    now_ms: u64,
) -> Engagement {
    let mut rng = rand::thread_rng();
    let npc_id = prey.npc.nid;
    // Re-read the monster: it may have died or been taken since the scan.
    let Some(live) = prey_view(world, npc_id) else {
        return Engagement::Over;
    };
    if !assist && !solo_prey_ok(world, &live, &[]) {
        // A player touched it (or it started chasing one): hands off.
        block_prey(st, npc_id, now_ms);
        return Engagement::Over;
    }
    let floor = live.tmpl.max_hp as i32 * ASSIST_FLOOR_PERCENT / 100;
    if assist && live.hp <= floor {
        return Engagement::Over;
    }

    let skill = bot_ai::select_bot_attack_skill(world, bot, npc_id as SessionId, now_ms);
    let range = bot_ai::bot_attack_range_for_skill(world, bot, skill, npc_id as SessionId);
    let ranged = range > 12.0;
    let reach = range.min(if ranged { MOB_REACH_RANGED } else { MOB_REACH_MELEE });
    let dx = live.x - bot.x;
    let dz = live.z - bot.z;
    let dist = (dx * dx + dz * dz).sqrt();

    if dist > reach * 0.9 {
        // Approach a point on our own side of the monster, fanned out by the personal angle.
        let off = (reach * 0.55).clamp(1.2, 9.0);
        let base = (bot.z - live.z).atan2(bot.x - live.x) + st.angle;
        let (tx, tz) = (live.x + base.cos() * off, live.z + base.sin() * off);
        let Some((nx, ny, nz)) = bot_ai::bot_path_step_toward(world, bot, tx, live.npc.y, tz) else {
            return Engagement::Blocked;
        };
        let echo = if dist - 1.2 <= reach { 0 } else { 3 };
        commit_move(world, bot, nx, ny, nz, echo, now_ms);
        return Engagement::Moved;
    }

    // In range: swing when the cooldown allows.
    let cd = attack_cooldown_ms(bot, ranged);
    if now_ms.saturating_sub(bot.skill_cooldown[0]) < cd {
        return Engagement::Fighting;
    }
    let mut skill = skill;
    let cost = world.get_magic(skill as i32).and_then(|m| m.msp).unwrap_or(0).max(0);
    if cost > 0 {
        if bot.mp < cost {
            // Out of mana: fall back to the free basic attack instead of standing around.
            skill = if bot.nation == NATION_ELMORAD { 201_001 } else { 101_001 };
        } else {
            world.update_bot(bot.id, |b| b.mp = (b.mp - cost).max(0));
        }
    }
    let damage = bot_hit_damage(bot, &live.tmpl, &mut rng);
    let hp_now = world.get_npc_hp(npc_id).unwrap_or(0);
    let new_hp = if assist {
        (hp_now - damage).max(floor)
    } else {
        (hp_now - damage).max(0)
    };
    world.update_npc_hp(npc_id, new_hp);

    let cast = bot_ai::build_bot_magic_packet(
        MAGIC_CASTING,
        skill,
        bot.id,
        npc_id,
        [bot.x as i32, bot.y as i32, bot.z as i32, 0, 0, 0, 0],
    );
    bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &cast);
    let effect = bot_ai::build_bot_magic_packet(
        MAGIC_EFFECTING,
        skill,
        bot.id,
        npc_id,
        [bot.x as i32, bot.y as i32, bot.z as i32, -damage, 0, 0, 0],
    );
    bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &effect);

    // Area skills of a party mage bot hit every monster of the pulled group, like a player's AoE would.
    if assist && bot.is_mage() {
        let radius = area_radius(world, skill);
        if radius > 0.0 {
            splash(world, bot, &live, skill, damage, radius, &mut rng);
        }
    }

    world.update_bot(bot.id, |b| {
        b.skill_cooldown[0] = now_ms;
        b.target_id = npc_id as i16;
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
    });

    if new_hp <= 0 {
        match if assist { party_credit_sid(world, bot, npc_id) } else { None } {
            Some(killer) => queue_party_kill(world, killer, &live),
            None => finish_prey(world, &live),
        }
        return Engagement::Over;
    }
    if now_ms >= st.mob_hit_ms {
        mob_retaliate(world, bot, &live.tmpl, &mut rng);
        st.mob_hit_ms = now_ms + (live.tmpl.attack_delay as u64).clamp(1_200, 4_000);
    }
    Engagement::Fighting
}

fn pick_roam_goal(world: &WorldState, bot: &BotInstance, st: &FieldBotRt, ctx: &Ctx<'_>, now_ms: u64) -> Option<(f32, f32)> {
    let mut rng = rand::thread_rng();
    let grounds = grounds_for(world, bot.zone_id, now_ms);
    let near: Vec<&Mob> = grounds
        .iter()
        .flat_map(|g| g.in_band(bot.level))
        .filter(|m| dist2(m.x, m.z, st.home.0, st.home.1) < LEASH_RADIUS * LEASH_RADIUS)
        .collect();
    for _ in 0..8 {
        let (gx, gz) = if near.is_empty() {
            let a = rng.gen_range(0.0..std::f32::consts::TAU);
            let r = rng.gen_range(6.0..20.0f32);
            (st.home.0 + a.cos() * r, st.home.1 + a.sin() * r)
        } else {
            let m = near[rng.gen_range(0..near.len())];
            (m.x + rng.gen_range(-5.0..5.0), m.z + rng.gen_range(-5.0..5.0))
        };
        if dist2(gx, gz, bot.x, bot.z) < 36.0 || crowded(ctx, bot.id, bot.zone_id, gx, gz, 6.0) {
            continue;
        }
        if let Some(p) = walkable_near(world, bot.zone_id, gx, gz, 6) {
            return Some(p);
        }
    }
    None
}

fn solo_hunt(world: &WorldState, bot: &BotInstance, st: &mut FieldBotRt, ctx: &mut Ctx<'_>) {
    let now_ms = ctx.now_ms;

    // 1. Keep fighting the current claim.
    if let Some(npc) = st.claim {
        if try_claim(world, bot.id, npc, now_ms) {
            if let Some(prey) = prey_view(world, npc) {
                match engage(world, bot, st, &prey, false, now_ms) {
                    Engagement::Moved | Engagement::Fighting => return,
                    Engagement::Blocked => {
                        block_prey(st, npc, now_ms);
                        release_claim(world, bot.id, st);
                        st.stuck_hits = st.stuck_hits.saturating_add(1);
                    }
                    Engagement::Over => {
                        release_claim(world, bot.id, st);
                        st.idle_until_ms = now_ms + rand::thread_rng().gen_range(300..1_100);
                        st.no_prey_scans = 0;
                        return;
                    }
                }
            } else {
                release_claim(world, bot.id, st);
            }
        } else {
            st.claim = None;
        }
    }

    // 2. Look for new prey (bounded by the shared scan budget).
    if now_ms >= st.next_scan_ms && ctx.scans_left > 0 {
        ctx.scans_left -= 1;
        st.next_scan_ms = now_ms + rand::thread_rng().gen_range(500..1_100);
        if let Some(prey) = find_solo_prey(world, bot, st, now_ms) {
            if try_claim(world, bot.id, prey.npc.nid, now_ms) {
                st.claim = Some(prey.npc.nid);
                st.no_prey_scans = 0;
                st.roam_goal = None;
                if let Engagement::Blocked = engage(world, bot, st, &prey, false, now_ms) {
                    block_prey(st, prey.npc.nid, now_ms);
                    release_claim(world, bot.id, st);
                }
                return;
            }
        } else {
            st.no_prey_scans = st.no_prey_scans.saturating_add(1);
        }
    }

    // 3. Nothing to fight right now: move on, never stand idle.
    if st.no_prey_scans > 14 {
        // The ground is hunted out (or all its monsters are respawning): relocate.
        let occupied: Vec<(f32, f32)> = ctx
            .positions
            .iter()
            .filter(|(id, zn, _, _)| *id != bot.id && *zn == bot.zone_id)
            .map(|(_, _, x, z)| (*x, *z))
            .collect();
        let mut rng = rand::thread_rng();
        if let Some(anchor) = choose_ground(world, bot.zone_id, bot.level, now_ms, &occupied, &mut rng) {
            st.home = anchor;
            st.roam_goal = walkable_near(world, bot.zone_id, anchor.0, anchor.1, 12);
            st.roam_until_ms = now_ms + 60_000;
        }
        st.no_prey_scans = 0;
    }
    if now_ms < st.idle_until_ms {
        return;
    }
    if st.roam_goal.is_none() || now_ms > st.roam_until_ms {
        st.roam_goal = pick_roam_goal(world, bot, st, ctx, now_ms);
        st.roam_until_ms = now_ms + 18_000;
        if st.roam_goal.is_none() {
            st.idle_until_ms = now_ms + 1_500;
            return;
        }
    }
    let Some((gx, gz)) = st.roam_goal else {
        return;
    };
    if dist2(bot.x, bot.z, gx, gz) < 4.0 {
        st.roam_goal = None;
        st.idle_until_ms = now_ms + rand::thread_rng().gen_range(1_200..4_000);
        return;
    }
    match bot_ai::bot_path_step_toward(world, bot, gx, bot.y, gz) {
        Some((nx, ny, nz)) => commit_move(world, bot, nx, ny, nz, 3, now_ms),
        None => {
            st.roam_goal = None;
            st.stuck_hits = st.stuck_hits.saturating_add(1);
            st.idle_until_ms = now_ms + 600;
        }
    }
}

fn party_behaviour(world: &WorldState, bot: &BotInstance, st: &mut FieldBotRt, party: &PartyCtx, ctx: &mut Ctx<'_>) {
    let now_ms = ctx.now_ms;
    if world.field_bots.hold.lock().contains(&bot.id) {
        return;
    }
    let Some(&leader) = party.present.first() else {
        return;
    };
    let Some((lx, ly, lz)) = pos_of(world, leader) else {
        return;
    };

    // Assist whatever the party is fighting.
    if let Some(npc) = st.claim {
        if let Some(prey) = prey_view(world, npc) {
            match engage(world, bot, st, &prey, true, now_ms) {
                Engagement::Moved | Engagement::Fighting => return,
                Engagement::Blocked => {
                    block_prey(st, npc, now_ms);
                    st.claim = None;
                }
                Engagement::Over => st.claim = None,
            }
        } else {
            st.claim = None;
        }
    }
    if ctx.scans_left > 0 && now_ms >= st.next_scan_ms {
        ctx.scans_left -= 1;
        st.next_scan_ms = now_ms + 500;
        if let Some(prey) = find_party_prey(world, bot, party, leader, now_ms, st) {
            st.claim = Some(prey.npc.nid);
            if let Engagement::Blocked = engage(world, bot, st, &prey, true, now_ms) {
                block_prey(st, prey.npc.nid, now_ms);
                st.claim = None;
            }
            return;
        }
    }

    // Otherwise stay around the leader on a personal ring slot.
    let dist_leader = dist2(bot.x, bot.z, lx, lz).sqrt();
    if dist_leader > PARTY_WARP_DISTANCE {
        let ang = party.slot as f32 * 2.1 + 0.7;
        if let Some((wx, wz)) = walkable_near(world, bot.zone_id, lx + ang.cos() * 5.0, lz + ang.sin() * 5.0, 8) {
            warp_bot(world, bot.id, wx, wz);
        }
        return;
    }
    let ang = party.slot as f32 * (std::f32::consts::TAU / party.bots.max(3) as f32) + 0.9;
    let radius = 4.5 + (party.slot % 2) as f32 * 1.8;
    let (gx, gz) = (lx + ang.cos() * radius, lz + ang.sin() * radius);
    let dist_goal = dist2(bot.x, bot.z, gx, gz).sqrt();
    if dist_leader > PARTY_FOLLOW_TRIGGER || dist_goal > 14.0 {
        st.following = true;
    }
    if st.following {
        if dist_goal < 1.8 {
            st.following = false;
        } else if let Some((nx, ny, nz)) = bot_ai::bot_path_step_toward(world, bot, gx, ly, gz) {
            if dist2(nx, nz, bot.x, bot.z) < 0.0025 {
                // No progress possible (unwalkable slot): settle where we are.
                st.following = false;
            } else {
                let echo = if dist2(nx, nz, gx, gz) < 2.0 { 0 } else { 3 };
                commit_move(world, bot, nx, ny, nz, echo, now_ms);
            }
        } else {
            st.following = false;
            st.stuck_hits = st.stuck_hits.saturating_add(1);
        }
    }
}

/// Stuck recovery: a bot that wants to go somewhere (or is after a monster) but has neither
/// moved nor fought for a while drops the goal; after repeated failures it is warped home.
fn check_stuck(world: &WorldState, bot: &BotInstance, st: &mut FieldBotRt, now_ms: u64) {
    if now_ms.saturating_sub(st.stuck_since_ms) < 6_000 {
        return;
    }
    st.stuck_since_ms = now_ms;
    let idle_for = now_ms.saturating_sub(bot.last_move_ms);
    let wants_move = st.claim.is_some() || st.roam_goal.is_some();
    if idle_for < 5_000 {
        st.stuck_hits = 0;
        return;
    }
    if !wants_move || bot.presence == BotPresence::Sitting {
        return;
    }
    st.stuck_hits = st.stuck_hits.saturating_add(1);
    if let Some(npc) = st.claim {
        block_prey(st, npc, now_ms);
    }
    release_claim(world, bot.id, st);
    st.roam_goal = None;
    if st.stuck_hits >= 3 {
        st.stuck_hits = 0;
        if let Some((x, z)) = walkable_near(world, bot.zone_id, st.home.0, st.home.1, 16) {
            warp_bot(world, bot.id, x, z);
        }
    }
}

fn needs_mana(bot: &BotInstance) -> bool {
    bot.is_mage() || bot.is_priest()
}

fn think(world: &WorldState, bot: &BotInstance, st: &mut FieldBotRt, ctx: &mut Ctx<'_>) {
    let now_ms = ctx.now_ms;

    // A bot that came back from death (or from a frozen zone) starts from a clean slate.
    if st.was_inactive {
        st.was_inactive = false;
        st.mode = Mode::Hunt;
        st.roam_goal = None;
        st.following = false;
        st.mob_hit_ms = 0;
        release_claim(world, bot.id, st);
        if bot.presence == BotPresence::Sitting {
            // Frozen while resting (empty zone): stand up, the rest check below sits it down again
            // if it is still hurt.
            set_sitting(world, bot, false);
        }
    }

    // Party bookkeeping.
    let party = resolve_party(world, bot, st, now_ms);
    if party.is_some() {
        st.in_party = true;
    } else if st.in_party {
        // Removed from (or the end of) a party: back to hunting, and look for company again.
        st.in_party = false;
        st.claim = None;
        // The party may have led it far from its ground: re-anchor on the next idle scan.
        st.no_prey_scans = 20;
        set_need_party(world, bot, true);
    }
    if let Some(p) = &party {
        if p.present.is_empty() {
            if st.leader_missing_since_ms == 0 {
                st.leader_missing_since_ms = now_ms;
            }
            if now_ms.saturating_sub(st.leader_missing_since_ms) > 12_000 {
                st.leader_missing_since_ms = 0;
                st.party_id = None;
                leave_party(world, bot, Some(if bot.level >= 30 { "Hoşça kal, iyi avlar!" } else { "Bye bye!" }));
                return;
            }
        } else {
            st.leader_missing_since_ms = 0;
        }
    } else {
        st.leader_missing_since_ms = 0;
    }

    // Support classes buff/heal (and self-buff when alone). A cast pins the bot for this tick.
    if (bot.is_priest() || bot.is_rogue()) && bot_ai::tick_bot_support(world, bot, now_ms) {
        return;
    }
    // Virtual potions / scrolls (shared bot brain; never blocks movement).
    crate::systems::bot_brain::tick_consumables_for(world, bot, now_ms);

    // Resting.
    let hp_pct = if bot.max_hp > 0 { bot.hp as i32 * 100 / bot.max_hp as i32 } else { 100 };
    let mp_pct = if bot.max_mp > 0 { bot.mp as i32 * 100 / bot.max_mp as i32 } else { 100 };
    match st.mode {
        Mode::Hunt => {
            if hp_pct < REST_ENTER_HP_PERCENT || (needs_mana(bot) && mp_pct < 12) {
                st.mode = Mode::Rest;
                st.last_rest_tick_ms = now_ms;
                release_claim(world, bot.id, st);
                st.roam_goal = None;
                set_sitting(world, bot, true);
                return;
            }
        }
        Mode::Rest => {
            if now_ms.saturating_sub(st.last_rest_tick_ms) >= 1_000 {
                st.last_rest_tick_ms = now_ms;
                world.update_bot(bot.id, |b| {
                    b.hp = (b.hp as i32 + (b.max_hp as i32 * 25 / 1000).max(1)).min(b.max_hp as i32) as i16;
                    b.mp = (b.mp as i32 + (b.max_mp as i32 * 30 / 1000).max(1)).min(b.max_mp as i32) as i16;
                });
                bot_ai::broadcast_bot_party_hp(world, bot.id);
            }
            let rested = world.get_bot(bot.id);
            let (h, m) = rested
                .as_ref()
                .map(|b| {
                    (
                        if b.max_hp > 0 { b.hp as i32 * 100 / b.max_hp as i32 } else { 100 },
                        if b.max_mp > 0 { b.mp as i32 * 100 / b.max_mp as i32 } else { 100 },
                    )
                })
                .unwrap_or((100, 100));
            if h >= REST_LEAVE_HP_PERCENT && (!needs_mana(bot) || m >= 50) {
                st.mode = Mode::Hunt;
                if let Some(b) = rested {
                    set_sitting(world, &b, false);
                }
            }
            return;
        }
    }

    match party {
        Some(p) if !p.present.is_empty() => party_behaviour(world, bot, st, &p, ctx),
        // In a party but the leader is elsewhere: wait where we are.
        Some(_) => {}
        None => solo_hunt(world, bot, st, ctx),
    }
    check_stuck(world, bot, st, now_ms);
}

// ── Tick driver ──────────────────────────────────────────────────────────

/// One scheduler pass: population management, chat delivery and every bot's brain, with a work
/// budget. Safe to call from any thread; never panics on missing data.
pub fn tick(world: &WorldState, now_ms: u64) {
    tick_with_budget(world, now_ms, Duration::from_millis(TICK_BUDGET_MS));
}

/// Same as [`tick`] with an explicit time budget (tests use a tiny one).
pub fn tick_with_budget(world: &WorldState, now_ms: u64, budget: Duration) {
    let started = Instant::now();
    let reg = &world.field_bots;
    if now_ms.saturating_sub(reg.last_manage_ms.load(Ordering::Relaxed)) >= MANAGE_INTERVAL_MS {
        manage(world, now_ms);
    }
    deliver_replies(world, now_ms);

    let mut ids: Vec<BotId> = reg.rt.lock().keys().copied().collect();
    if ids.is_empty() {
        return;
    }
    ids.sort_unstable();

    // Snapshot once: bots and their positions.
    let mut bots: Vec<BotInstance> = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(b) = world.get_bot(*id) {
            bots.push(b);
        }
    }
    let positions: Vec<(BotId, u16, f32, f32)> =
        bots.iter().map(|b| (b.id, b.zone_id, b.x, b.z)).collect();
    let mut ctx = Ctx { now_ms, positions: &positions, scans_left: SCANS_PER_TICK };

    let n = bots.len();
    let start = reg.cursor.load(Ordering::Relaxed) % n.max(1);
    let mut processed = 0usize;
    let mut active_zone: HashMap<u16, bool> = HashMap::new();
    for off in 0..n {
        // Always serve at least one bot per tick, whatever the budget says.
        if processed > 0 && started.elapsed() >= budget {
            break;
        }
        let bot = &bots[(start + off) % n];
        processed += 1;
        let active = *active_zone
            .entry(bot.zone_id)
            .or_insert_with(|| world.zone_has_players(bot.zone_id));
        let Some(mut st) = reg.rt.lock().get(&bot.id).cloned() else {
            continue;
        };
        if !bot.in_game || bot.presence == BotPresence::Dead || bot.ai_state != BotAiState::Field || !active {
            let alive_field = bot.in_game && bot.presence != BotPresence::Dead && bot.ai_state == BotAiState::Field;
            let mut dirty = false;
            if !st.was_inactive {
                st.was_inactive = true;
                release_claim(world, bot.id, &mut st);
                dirty = true;
            }
            if alive_field {
                dirty |= frozen_party_hygiene(world, bot, &mut st, now_ms);
            }
            if dirty {
                reg.rt.lock().insert(bot.id, st);
            }
            continue;
        }
        think(world, bot, &mut st, &mut ctx);
        reg.rt.lock().insert(bot.id, st);
    }
    reg.cursor.store((start + processed) % n.max(1), Ordering::Relaxed);
}

/// A bot frozen because its zone is empty still leaves a party that has no real member left
/// (for example the leader logged out), so such parties do not linger. Returns whether `st` changed.
fn frozen_party_hygiene(world: &WorldState, bot: &BotInstance, st: &mut FieldBotRt, now_ms: u64) -> bool {
    if now_ms.saturating_sub(st.party_check_ms) < 2_000 {
        return false;
    }
    st.party_check_ms = now_ms;
    let Some(pid) = world.get_party_id(bot.id as SessionId) else {
        st.party_id = None;
        return true;
    };
    let Some(party) = world.get_party(pid) else {
        return true;
    };
    let any_real = party
        .active_members()
        .iter()
        .any(|s| is_real(*s) && world.get_position(*s).is_some());
    if !any_real {
        st.party_id = None;
        leave_party(world, bot, None);
    }
    true
}

/// Where a dead field bot comes back: its hunting ground.
pub fn respawn_point(world: &WorldState, bot: &BotInstance) -> Option<(f32, f32)> {
    let home = world.field_bots.rt.lock().get(&bot.id).map(|s| s.home)?;
    let mut rng = rand::thread_rng();
    walkable_near(
        world,
        bot.zone_id,
        home.0 + rng.gen_range(-4.0..4.0),
        home.1 + rng.gen_range(-4.0..4.0),
        12,
    )
}

// ── Party invitation hook ────────────────────────────────────────────────

fn party_error(world: &WorldState, sid: SessionId, code: i16) {
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(crate::handler::party::PARTY_INSERT);
    pkt.write_i16(code);
    world.send_to_session_owned(sid, pkt);
}

const PARTY_ERR_DECLINED: i16 = -1;
const PARTY_ERR_LEVEL: i16 = -2;
const PARTY_ERR_NATION: i16 = -3;
const PARTY_ERR_ZONE: i16 = -5;
const PARTY_ERR_GENERIC: i16 = -7;

/// A real player invites `target_name` to a party. Returns `true` when the name belongs to a
/// field bot (the request is fully handled -- accepted or politely declined) and `false` for any
/// other name, which the caller then treats as before.
///
/// Rules a bot applies, in order: it must be alive and in the inviter's zone and view; same
/// nation unless the zone allows cross-nation parties (Moradon, Eslants); the normal party level
/// rule; healthy and not already grouped; and the inviter's party must be led by the inviter,
/// have room, and hold fewer than [`MAX_BOTS_PER_PARTY`] bots.
pub fn try_party_invite(world: &WorldState, inviter_sid: SessionId, target_name: &str, party_type: i8) -> bool {
    let Some(bot) = find_field_bot_by_name(world, target_name) else {
        return false;
    };
    let Some(inviter) = world.get_character_info(inviter_sid) else {
        return true;
    };
    let Some(ipos) = world.get_position(inviter_sid) else {
        return true;
    };
    let now_ms = bot_ai::tick_ms();
    let busy_reply = |text: &str| {
        let t = text.to_string();
        queue_reply(world, bot.id, 400, t, Channel::General, now_ms);
    };

    if !bot.is_alive() || ipos.zone_id != bot.zone_id {
        party_error(world, inviter_sid, PARTY_ERR_ZONE);
        return true;
    }
    if world.get_party_id(bot.id as SessionId).is_some() || world.has_party_invitation(bot.id as SessionId) {
        party_error(world, inviter_sid, PARTY_ERR_GENERIC);
        return true;
    }
    if inviter.nation != bot.nation && !crate::handler::party::is_partner_party_zone(ipos.zone_id) {
        party_error(world, inviter_sid, PARTY_ERR_NATION);
        return true;
    }
    let chicken = inviter.level < 30 || bot.level < 30;
    if !chicken && !crate::handler::party::is_level_compatible(inviter.level, bot.level) {
        party_error(world, inviter_sid, PARTY_ERR_LEVEL);
        return true;
    }
    if dist2(ipos.x, ipos.z, bot.x, bot.z) > PARTY_INVITE_RANGE * PARTY_INVITE_RANGE {
        party_error(world, inviter_sid, PARTY_ERR_DECLINED);
        busy_reply("Çok uzaktasın, yaklaş da öyle davet et. / Too far, come closer.");
        return true;
    }
    let hp_pct = if bot.max_hp > 0 { bot.hp as i32 * 100 / bot.max_hp as i32 } else { 100 };
    let resting = world
        .field_bots
        .rt
        .lock()
        .get(&bot.id)
        .is_some_and(|s| s.mode == Mode::Rest);
    if hp_pct < 30 || resting {
        party_error(world, inviter_sid, PARTY_ERR_DECLINED);
        busy_reply("Dinleniyorum, birazdan tekrar dene. / Resting, try again in a moment.");
        return true;
    }

    // The inviter's side of the party.
    let existing = world.get_party_id(inviter_sid);
    let party_id = match existing {
        Some(pid) => {
            let Some(party) = world.get_party(pid) else {
                party_error(world, inviter_sid, PARTY_ERR_GENERIC);
                return true;
            };
            if !party.is_leader(inviter_sid) || party.is_full() {
                party_error(world, inviter_sid, PARTY_ERR_GENERIC);
                return true;
            }
            let bots = party.active_members().iter().filter(|s| !is_real(**s)).count();
            if bots >= MAX_BOTS_PER_PARTY {
                party_error(world, inviter_sid, PARTY_ERR_DECLINED);
                busy_reply("Partide yeterince bot var, bir gerçek oyuncu al. / Enough bots in this party.");
                return true;
            }
            pid
        }
        None => {
            if world.has_party_invitation(inviter_sid) {
                party_error(world, inviter_sid, PARTY_ERR_GENERIC);
                return true;
            }
            world.update_session(inviter_sid, |h| h.party_type = party_type);
            let Some(pid) = world.create_party(inviter_sid) else {
                party_error(world, inviter_sid, PARTY_ERR_GENERIC);
                return true;
            };
            if let Some((pos, room)) = world.with_session(inviter_sid, |h| (h.position, h.event_room)) {
                let leader_sc = crate::handler::regene::build_state_change_broadcast(
                    inviter_sid as u32,
                    crate::state_change_constants::STATE_CHANGE_PARTY_LEADER,
                    1,
                );
                world.broadcast_to_region_sync(pos.zone_id, pos.region_x, pos.region_z, Arc::new(leader_sc), None, room);
            }
            pid
        }
    };

    if !world.add_party_member(party_id, bot.id as SessionId) {
        party_error(world, inviter_sid, PARTY_ERR_GENERIC);
        return true;
    }
    let target_number = world.get_party(party_id).map_or(-1, |p| p.target_number_id);
    let info = crate::handler::party::build_bot_party_member_info(&bot, 1, target_number);
    world.send_to_party(party_id, &info);
    bot_ai::broadcast_bot_party_hp(world, bot.id);
    set_need_party(world, &bot, false);
    {
        let mut rt = world.field_bots.rt.lock();
        if let Some(st) = rt.get_mut(&bot.id) {
            st.party_id = Some(party_id);
            st.party_check_ms = now_ms;
            st.claim = None;
            st.roam_goal = None;
        }
    }
    world.field_bots.hold.lock().remove(&bot.id);
    let greet = party_greeting(&bot, &inviter.name);
    queue_reply(world, bot.id, 500, greet, Channel::Party(party_id), now_ms);
    true
}

fn party_greeting(bot: &BotInstance, player: &str) -> String {
    let variants: [String; 3] = [
        format!("Selam {player}! {} Lv{} hazır, avlanalım.", class_name(bot, true), bot.level),
        format!("Partiye katıldım {player}. Ben {} (Lv{}), kolay gelsin!", class_name(bot, true), bot.level),
        format!("Hi {player}, {} Lv{} here. Lead the way!", class_name(bot, false), bot.level),
    ];
    let i = (bot.id as usize + bot.level as usize) % variants.len();
    variants[i].clone()
}

// ── Chat understanding ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Greeting,
    HowAreYou,
    PartyRequest,
    AskLevel,
    AreYouBot,
    Thanks,
    Bye,
    Leave,
    Hold,
    Follow,
}

/// ASCII-fold Turkish letters so "nasılsın" and "nasilsin" match the same keyword.
fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ı' | 'İ' | 'I' => 'i',
            'ş' | 'Ş' => 's',
            'ğ' | 'Ğ' => 'g',
            'ü' | 'Ü' => 'u',
            'ö' | 'Ö' => 'o',
            'ç' | 'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn words_of(folded: &str) -> Vec<&str> {
    folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect()
}

/// Does the message look Turkish? (Turkish letters or common Turkish words.)
pub fn is_turkish(message: &str) -> bool {
    if message.chars().any(|c| "ıİşŞğĞüÜöÖçÇ".contains(c)) {
        return true;
    }
    let f = fold(message);
    const TR: [&str; 22] = [
        "selam", "slm", "merhaba", "mrb", "nasilsin", "naber", "nbr", "tesekkur", "sagol", "gorusuruz",
        "hosca", "kolay", "gel", "parti", "davet", "kac", "musun", "misin", "var", "ben", "bana", "abi",
    ];
    words_of(&f).iter().any(|w| TR.contains(w))
}

/// Classify a chat line. `None` when the line is not for a bot.
pub fn detect_intent(message: &str) -> Option<Intent> {
    let f = fold(message);
    let words = words_of(&f);
    let has = |list: &[&str]| words.iter().any(|w| list.contains(w));
    let has_phrase = |phrases: &[&str]| phrases.iter().any(|p| f.contains(p));
    // Party-control words are commands only as short messages ("dur", "bekle", "bots leave").
    let short = words.len() <= 3;

    if has_phrase(&["bot musun", "bot mu", "bot musunuz", "are you a bot", "are u a bot", "bot?"])
        || (has(&["bot"]) && has(&["musun", "misin", "mu", "mi", "you", "u"]))
    {
        return Some(Intent::AreYouBot);
    }
    if short && (has(&["ayril", "leave", "kick", "cik"]) || has_phrase(&["party birak", "partiden cik"])) {
        return Some(Intent::Leave);
    }
    if short && has(&["bekle", "dur", "stay", "wait", "hold"]) {
        return Some(Intent::Hold);
    }
    if short && has(&["takip", "follow", "yanima", "gel"]) && !has(&["party", "parti", "pt"]) {
        return Some(Intent::Follow);
    }
    if has(&["party", "parti", "pt", "grup", "davet", "invite", "partyye", "partiye", "join"]) {
        return Some(Intent::PartyRequest);
    }
    if has(&["level", "lvl", "lv", "seviye", "seviyen", "seviyesi"]) {
        return Some(Intent::AskLevel);
    }
    if has_phrase(&["nasilsin", "naber", "nbr", "how are", "how r", "whats up", "what's up", "wassup"]) {
        return Some(Intent::HowAreYou);
    }
    if has(&["tesekkur", "tesekkurler", "sagol", "thanks", "thx", "ty", "eyw", "eyvallah", "tsk"]) {
        return Some(Intent::Thanks);
    }
    if has(&["bye", "gorusuruz", "hosca", "bb", "gtg", "cya"]) || has_phrase(&["hosca kal", "see you", "see ya"]) {
        return Some(Intent::Bye);
    }
    if has(&["selam", "slm", "merhaba", "mrb", "hello", "hi", "hey", "heyy", "sa", "selamlar", "yo", "sea"]) {
        return Some(Intent::Greeting);
    }
    None
}

/// The canned reply for an intent. `variant` picks one of the alternatives.
pub fn reply_text(intent: Intent, turkish: bool, bot: &BotInstance, player: &str, variant: usize) -> String {
    let lvl = bot.level;
    let cls = class_name(bot, turkish);
    let pick = |opts: &[String]| opts[variant % opts.len()].clone();
    if turkish {
        match intent {
            Intent::Greeting => pick(&[
                format!("Selam {player}!"),
                format!("Merhaba {player}, kolay gelsin."),
                format!("Selamlar! Ben {cls}, Lv{lvl}."),
            ]),
            Intent::HowAreYou => pick(&[
                format!("İyiyim {player}, avlanıyorum. Sen nasılsın?"),
                "Fena değil, canavar kesiyoruz :)".to_string(),
                format!("Gayet iyi, {cls} Lv{lvl} olarak av devam."),
            ]),
            Intent::PartyRequest => pick(&[
                format!("Olur! Beni davet et ({cls} Lv{lvl}), birlikte avlanalım."),
                "Party'ye hazırım, sağ tık > davet et yeterli.".to_string(),
                format!("{cls} olarak yardımcı olurum, davet bekliyorum."),
            ]),
            Intent::AskLevel => pick(&[
                format!("Lv{lvl} {cls}'im."),
                format!("Seviyem {lvl}, sınıfım {cls}."),
            ]),
            Intent::AreYouBot => pick(&[
                "Evet, ben sunucunun saha botuyum :) Party'ye alırsan seninle avlanırım.".to_string(),
                format!("Doğru tahmin, bot'um! {cls} Lv{lvl}. Davet edersen yardımcı olurum."),
            ]),
            Intent::Thanks => pick(&["Rica ederim!".to_string(), "Ne demek, iyi avlar!".to_string()]),
            Intent::Bye => pick(&["Görüşürüz!".to_string(), "Hoşça kal, iyi avlar!".to_string()]),
            Intent::Leave => "Tamam, ayrılıyorum. İyi avlar!".to_string(),
            Intent::Hold => "Burada bekliyorum.".to_string(),
            Intent::Follow => "Peşindeyim!".to_string(),
        }
    } else {
        match intent {
            Intent::Greeting => pick(&[
                format!("Hi {player}!"),
                format!("Hello {player}, good hunting."),
                format!("Hey! {cls} Lv{lvl} here."),
            ]),
            Intent::HowAreYou => pick(&[
                format!("Doing fine {player}, hunting. You?"),
                "Not bad, grinding some mobs :)".to_string(),
                format!("All good, {cls} Lv{lvl} on the hunt."),
            ]),
            Intent::PartyRequest => pick(&[
                format!("Sure! Invite me ({cls} Lv{lvl}) and we hunt together."),
                "Ready for a party, just invite me.".to_string(),
                format!("I can help as {cls}, waiting for the invite."),
            ]),
            Intent::AskLevel => pick(&[
                format!("I'm a level {lvl} {cls}."),
                format!("Lv{lvl} {cls}."),
            ]),
            Intent::AreYouBot => pick(&[
                "Yes, I'm one of the server's field bots :) Party me and I'll hunt with you.".to_string(),
                format!("Guilty, I'm a bot! {cls} Lv{lvl}. Invite me if you want a hand."),
            ]),
            Intent::Thanks => pick(&["No problem!".to_string(), "Anytime, happy hunting!".to_string()]),
            Intent::Bye => pick(&["See you!".to_string(), "Bye, good hunting!".to_string()]),
            Intent::Leave => "Alright, leaving the party. Good hunting!".to_string(),
            Intent::Hold => "Holding position here.".to_string(),
            Intent::Follow => "Following you!".to_string(),
        }
    }
}

fn whisper_fallback(turkish: bool, bot: &BotInstance, variant: usize) -> String {
    let cls = class_name(bot, turkish);
    let opts: [String; 3] = if turkish {
        [
            format!("Ben {cls} Lv{} saha botuyum. Beni party'ye davet edebilirsin :)", bot.level),
            "Şu an avdayım; 'party', 'level' ya da 'selam' yazabilirsin.".to_string(),
            "Seninle sohbet etmek güzel ama canavarlar beni bekliyor!".to_string(),
        ]
    } else {
        [
            format!("I'm a {cls} Lv{} field bot. You can invite me to a party :)", bot.level),
            "Out hunting right now; try 'party', 'level' or 'hello'.".to_string(),
            "Nice chatting, but the mobs are waiting!".to_string(),
        ]
    };
    opts[variant % opts.len()].clone()
}

fn bot_ready_to_talk(world: &WorldState, bot_id: BotId, now_ms: u64) -> bool {
    let mut rt = world.field_bots.rt.lock();
    match rt.get_mut(&bot_id) {
        Some(st) if now_ms.saturating_sub(st.last_chat_ms) >= BOT_CHAT_COOLDOWN_MS => {
            st.last_chat_ms = now_ms;
            true
        }
        _ => false,
    }
}

fn global_chat_gate(world: &WorldState, now_ms: u64) -> bool {
    let last = world.field_bots.last_global_chat_ms.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) < GLOBAL_CHAT_GAP_MS {
        return false;
    }
    world.field_bots.last_global_chat_ms.store(now_ms, Ordering::Relaxed);
    true
}

/// A real player said something in the normal (nearby) chat. The nearest eligible field bot may
/// answer once, a moment later. Bounded by per-bot and global cooldowns and a reply queue cap.
pub fn on_nearby_chat(world: &WorldState, sender_sid: SessionId, message: &str) {
    if message.is_empty() || message.len() > 200 {
        return;
    }
    let Some(intent) = detect_intent(message) else {
        return;
    };
    // Party control words only make sense in party chat.
    if matches!(intent, Intent::Leave | Intent::Hold | Intent::Follow) {
        return;
    }
    let (Some(pos), Some(ch)) = (world.get_position(sender_sid), world.get_character_info(sender_sid)) else {
        return;
    };
    let now_ms = bot_ai::tick_ms();
    // Nearest living field bot of a compatible nation within hearing range.
    let mut best: Option<(f32, BotInstance)> = None;
    for e in world.bots.iter() {
        let b = e.value();
        if !is_field_bot(b) || !b.is_alive() || b.zone_id != pos.zone_id {
            continue;
        }
        if b.nation != ch.nation && !crate::handler::party::is_partner_party_zone(b.zone_id) {
            continue;
        }
        let d = dist2(b.x, b.z, pos.x, pos.z);
        if d <= CHAT_HEAR_RANGE * CHAT_HEAR_RANGE && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
            best = Some((d, b.clone()));
        }
    }
    let Some((_, bot)) = best else {
        return;
    };
    if !global_chat_gate(world, now_ms) || !bot_ready_to_talk(world, bot.id, now_ms) {
        return;
    }
    let turkish = is_turkish(message);
    let variant = rand::thread_rng().gen_range(0..64usize);
    let text = reply_text(intent, turkish, &bot, &ch.name, variant);
    let delay = rand::thread_rng().gen_range(900..2_600u64);
    queue_reply(world, bot.id, delay, text, Channel::General, now_ms);
}

/// A player whispered a field bot. Always answered (rate limited), in the player's language.
pub fn on_whisper(world: &WorldState, sender_sid: SessionId, bot_id: BotId, message: &str) {
    let Some(bot) = world.get_bot(bot_id).filter(|b| is_field_bot(b) && b.is_alive()) else {
        return;
    };
    if message.is_empty() || message.len() > 200 {
        return;
    }
    let now_ms = bot_ai::tick_ms();
    if !global_chat_gate(world, now_ms) || !bot_ready_to_talk(world, bot_id, now_ms) {
        return;
    }
    let turkish = is_turkish(message);
    let name = world.get_character_info(sender_sid).map(|c| c.name).unwrap_or_default();
    let variant = rand::thread_rng().gen_range(0..64usize);
    let text = match detect_intent(message) {
        Some(Intent::Leave | Intent::Hold | Intent::Follow) | None => whisper_fallback(turkish, &bot, variant),
        Some(intent) => reply_text(intent, turkish, &bot, &name, variant),
    };
    queue_reply(world, bot_id, rand::thread_rng().gen_range(700..1_800u64), text, Channel::Whisper(sender_sid), now_ms);
}

/// Party chat addressed to the bots of the sender's party. Returns `true` when a field-bot
/// command (leave / hold / follow) was recognised and executed.
pub fn on_party_chat(world: &WorldState, party_id: u16, sender_sid: SessionId, message: &str) -> bool {
    let Some(party) = world.get_party(party_id) else {
        return false;
    };
    let field_bots: Vec<BotInstance> = party
        .active_members()
        .into_iter()
        .filter(|s| !is_real(*s))
        .filter_map(|s| world.get_bot(s as u32))
        .filter(|b| is_field_bot(b) && b.is_alive())
        .collect();
    if field_bots.is_empty() {
        return false;
    }
    let intent = detect_intent(message);
    let turkish = is_turkish(message);
    let now_ms = bot_ai::tick_ms();
    match intent {
        Some(Intent::Leave) => {
            if !party.is_leader(sender_sid) {
                return false;
            }
            for b in &field_bots {
                leave_party(world, b, Some(&reply_text(Intent::Leave, turkish, b, "", 0)));
            }
            true
        }
        Some(Intent::Hold) => {
            for b in &field_bots {
                world.field_bots.hold.lock().insert(b.id);
            }
            if let Some(b) = field_bots.first() {
                queue_reply(world, b.id, 400, reply_text(Intent::Hold, turkish, b, "", 0), Channel::Party(party_id), now_ms);
            }
            true
        }
        Some(Intent::Follow) => {
            for b in &field_bots {
                world.field_bots.hold.lock().remove(&b.id);
            }
            if let Some(b) = field_bots.first() {
                queue_reply(world, b.id, 400, reply_text(Intent::Follow, turkish, b, "", 0), Channel::Party(party_id), now_ms);
            }
            true
        }
        Some(i @ (Intent::Greeting | Intent::HowAreYou | Intent::AskLevel | Intent::Thanks | Intent::Bye | Intent::AreYouBot)) => {
            // One bot answers, the one that has not spoken for the longest.
            let name = world.get_character_info(sender_sid).map(|c| c.name).unwrap_or_default();
            for b in &field_bots {
                if global_chat_gate(world, now_ms) && bot_ready_to_talk(world, b.id, now_ms) {
                    let variant = rand::thread_rng().gen_range(0..64usize);
                    queue_reply(world, b.id, 600, reply_text(i, turkish, b, &name, variant), Channel::Party(party_id), now_ms);
                    break;
                }
            }
            false
        }
        _ => false,
    }
}

// ── Persistence / background task ────────────────────────────────────────

/// Load `field_bot_zone` and apply it. Keeps the previous configuration on a database error.
pub async fn reload_config(world: &WorldState, pool: &ko_db::DbPool) -> anyhow::Result<usize> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT zone_id, enabled, karus_count, elmorad_count, level_min, level_max, \
         mix_warrior, mix_rogue, mix_mage, mix_priest, mix_kurian FROM field_bot_zone",
    )
    .fetch_all(pool)
    .await?;
    let clamp_u8 = |v: i16| v.clamp(0, 255) as u8;
    let parsed: Vec<FieldZoneConfig> = rows
        .iter()
        .map(|r| FieldZoneConfig {
            zone_id: r.try_get::<i16, _>("zone_id").unwrap_or(0).max(0) as u16,
            enabled: r.try_get::<bool, _>("enabled").unwrap_or(false),
            karus_count: r.try_get::<i16, _>("karus_count").unwrap_or(0).max(0) as u16,
            elmorad_count: r.try_get::<i16, _>("elmorad_count").unwrap_or(0).max(0) as u16,
            level_min: clamp_u8(r.try_get::<i16, _>("level_min").unwrap_or(1)),
            level_max: clamp_u8(r.try_get::<i16, _>("level_max").unwrap_or(1)),
            mix: [
                clamp_u8(r.try_get::<i16, _>("mix_warrior").unwrap_or(0)),
                clamp_u8(r.try_get::<i16, _>("mix_rogue").unwrap_or(0)),
                clamp_u8(r.try_get::<i16, _>("mix_mage").unwrap_or(0)),
                clamp_u8(r.try_get::<i16, _>("mix_priest").unwrap_or(0)),
                clamp_u8(r.try_get::<i16, _>("mix_kurian").unwrap_or(0)),
            ],
        })
        .collect();
    let n = world.field_bots.apply_config(parsed);
    info!(zones = n, "field bot configuration loaded");
    Ok(n)
}

async fn write_status(world: &WorldState, pool: &ko_db::DbPool) {
    let counts = live_counts(world);
    for cfg in world.field_bots.config_snapshot() {
        let k = counts.get(&(cfg.zone_id, NATION_KARUS)).copied().unwrap_or(0) as i16;
        let e = counts.get(&(cfg.zone_id, NATION_ELMORAD)).copied().unwrap_or(0) as i16;
        let target = cfg.target_for(NATION_KARUS) + cfg.target_for(NATION_ELMORAD);
        let live = (k.max(0) + e.max(0)) as usize;
        let note = if !cfg.enabled {
            "Bolge kapali".to_string()
        } else if target == 0 {
            "Hedef 0 bot".to_string()
        } else if let Some(n) = world.field_bots.note_for(cfg.zone_id) {
            n
        } else if live < target {
            "Dogma suruyor".to_string()
        } else {
            String::new()
        };
        let _ = sqlx::query(
            "UPDATE field_bot_zone SET live_karus=$2, live_elmorad=$3, live_note=$4, live_updated_at=NOW() WHERE zone_id=$1",
        )
        .bind(cfg.zone_id as i16)
        .bind(k)
        .bind(e)
        .bind(note)
        .execute(pool)
        .await;
    }
}

/// Start the field-bot scheduler. Call once after the world tables are loaded.
pub fn start_field_bot_task(world: Arc<WorldState>, pool: ko_db::DbPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = reload_config(&world, &pool).await {
            warn!(error = %e, "field_bot_zone not readable yet; field bots stay off until the migration is applied");
        }
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut last_cfg = bot_ai::tick_ms();
        let mut last_status = last_cfg;
        loop {
            interval.tick().await;
            let now = bot_ai::tick_ms();
            tick(&world, now);
            drain_party_kills(&world).await;
            if now.saturating_sub(last_cfg) >= CONFIG_REFRESH_MS {
                last_cfg = now;
                if let Err(e) = reload_config(&world, &pool).await {
                    debug!(error = %e, "field bot config refresh failed; keeping the previous one");
                }
            }
            if now.saturating_sub(last_status) >= STATUS_WRITE_MS {
                last_status = now;
                write_status(&world, &pool).await;
            }
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests -- real SMD map data (Moradon), simulated clock, no database.
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CharacterInfo, Position};
    use crate::zone::{MapData, ZoneAbilities, ZoneAbilityType, ZoneInfo};
    use std::path::Path;
    use tokio::sync::mpsc;

    const MORADON: u16 = 21;
    const T_STEP: u64 = 250;

    fn smd(file: &str) -> MapData {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map").join(file);
        MapData::new(ko_protocol::smd::SmdFile::load(&path).expect("load SMD"))
    }

    fn zone_info(name: &str, attack_other_nation: bool) -> ZoneInfo {
        ZoneInfo {
            smd_name: String::new(),
            zone_name: name.to_string(),
            zone_type: ZoneAbilityType::Neutral,
            min_level: 1,
            max_level: 83,
            init_x: 0.0,
            init_z: 0.0,
            init_y: 0.0,
            abilities: ZoneAbilities { attack_other_nation, ..ZoneAbilities::default() },
            status: 1,
        }
    }

    /// Moradon with its real map.
    fn moradon_world() -> WorldState {
        let world = WorldState::new();
        world.set_zone_with_map(MORADON, zone_info("Moradon", false), smd("moradon_0826.smd"));
        world
    }

    /// The `skip`-th spot of the zone map that has open, walkable ground all around it.
    fn open_spot(world: &WorldState, zone_id: u16, skip: usize) -> (f32, f32) {
        let zone = world.get_zone(zone_id).unwrap();
        let map = zone.map_data.as_ref().unwrap();
        let mut found = 0;
        let mut x = 200.0f32;
        while x < map.map_width() - 200.0 {
            let mut z = 200.0f32;
            while z < map.map_height() - 200.0 {
                let ok = (-3..=3).all(|i| (-3..=3).all(|j| map.is_bot_movable(x + i as f32 * 6.0, z + j as f32 * 6.0)));
                if ok {
                    if found == skip {
                        return (x, z);
                    }
                    found += 1;
                }
                z += 24.0;
            }
            x += 24.0;
        }
        panic!("no open ground found on the test map");
    }

    /// An open spot between `lo` and `hi` units from `from`.
    fn near_open_spot(world: &WorldState, zone_id: u16, from: (f32, f32), lo: f32, hi: f32) -> (f32, f32) {
        let zone = world.get_zone(zone_id).unwrap();
        let map = zone.map_data.as_ref().unwrap();
        // walk outward in rings around `from`
        let mut r = lo;
        while r <= hi {
            let mut a = 0.0f32;
            while a < std::f32::consts::TAU {
                let (x, z) = (from.0 + a.cos() * r, from.1 + a.sin() * r);
                let ok = (-2..=2).all(|i| (-2..=2).all(|j| map.is_bot_movable(x + i as f32 * 4.0, z + j as f32 * 4.0)));
                if ok {
                    return (x, z);
                }
                a += 0.5;
            }
            r += 6.0;
        }
        panic!("no open ground between {lo} and {hi} units away");
    }

    /// An open spot at least `min_dist` away from `from`.
    fn far_open_spot(world: &WorldState, zone_id: u16, from: (f32, f32), min_dist: f32) -> (f32, f32) {
        let zone = world.get_zone(zone_id).unwrap();
        let map = zone.map_data.as_ref().unwrap();
        let mut x = 100.0f32;
        while x < map.map_width() - 100.0 {
            let mut z = 100.0f32;
            while z < map.map_height() - 100.0 {
                let ok = (-3..=3).all(|i| (-3..=3).all(|j| map.is_bot_movable(x + i as f32 * 6.0, z + j as f32 * 6.0)));
                if ok && dist2(x, z, from.0, from.1).sqrt() >= min_dist {
                    return (x, z);
                }
                z += 24.0;
            }
            x += 24.0;
        }
        panic!("no second open ground at {min_dist} units on the test map");
    }

    fn monster_template(s_sid: u16, level: u8, hp: u32, damage: u16) -> NpcTemplate {
        NpcTemplate {
            s_sid,
            is_monster: true,
            name: format!("Mob{s_sid}"),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 1,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level,
            max_hp: hp,
            max_mp: 0,
            attack: 0,
            ac: 100,
            hit_rate: 50,
            evade_rate: 10,
            damage,
            attack_delay: 1500,
            speed_1: 50,
            speed_2: 100,
            stand_time: 3000,
            search_range: 10,
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

    /// A normal respawning monster (the event-spawn helper marks NPCs as non-respawning event
    /// units, so patch those two flags).
    fn add_monster(world: &WorldState, zone_id: u16, x: f32, z: f32, level: u8, hp: u32) -> NpcId {
        let s_sid = 3000 + level as u16;
        if world.get_npc_template(s_sid, true).is_none() {
            world.insert_npc_template(monster_template(s_sid, level, hp, 40));
        }
        let nid = world.spawn_event_npc(s_sid, true, zone_id, x, z, 1)[0];
        let mut inst = (*world.get_npc_instance(nid).unwrap()).clone();
        inst.is_event_npc = false;
        world.insert_npc_instance(inst);
        world.update_npc_ai(nid, |a| a.regen_time_ms = 30_000);
        nid
    }

    fn add_player(
        world: &WorldState,
        sid: SessionId,
        nation: u8,
        level: u8,
        zone_id: u16,
        x: f32,
        z: f32,
    ) -> mpsc::UnboundedReceiver<Arc<Packet>> {
        let (tx, rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);
        let (rgx, rgz) = (calc_region(x), calc_region(z));
        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                name: format!("Player{sid}"),
                nation,
                level,
                class: if nation == NATION_KARUS { 101 } else { 201 },
                hp: 2000,
                max_hp: 2000,
                authority: 1,
                ..CharacterInfo::default()
            },
            Position { zone_id, x, y: 0.0, z, region_x: rgx, region_z: rgz },
        );
        if let Some(zone) = world.get_zone(zone_id) {
            zone.add_user(rgx, rgz, sid);
        }
        rx
    }

    fn move_player(world: &WorldState, sid: SessionId, x: f32, z: f32) {
        world.update_session(sid, |h| {
            h.position.x = x;
            h.position.z = z;
            h.position.region_x = calc_region(x);
            h.position.region_z = calc_region(z);
        });
    }

    /// Spawn a registered field bot of the given class (GM shorthand) at a position.
    fn add_bot(world: &WorldState, zone_id: u16, nation: u8, class: u16, level: u8, x: f32, z: f32) -> BotId {
        let id = bot_ai::spawn_gm_bot(
            world,
            bot_ai::SpawnGmBotParams { zone_id, x, y: 0.0, z, class, level, nation, ai_state: BotAiState::Field },
        );
        finish_spawn(world, id, (x, z), 0.3, bot_ai::tick_ms());
        // Hand-placed bots must not be culled by the population manager (no zone is configured
        // in these tests): push the next management pass out of reach.
        world.field_bots.last_manage_ms.store(u64::MAX, Ordering::Relaxed);
        id
    }

    fn drain(rx: &mut mpsc::UnboundedReceiver<Arc<Packet>>) -> Vec<Arc<Packet>> {
        let mut v = Vec::new();
        while let Ok(p) = rx.try_recv() {
            v.push(p);
        }
        v
    }

    fn run(world: &WorldState, t0: u64, ticks: u64) -> u64 {
        let mut now = t0;
        for _ in 0..ticks {
            now += T_STEP;
            tick_with_budget(world, now, Duration::from_secs(5));
        }
        now
    }

    fn cfg(zone: u16, enabled: bool, k: u16, e: u16, lo: u8, hi: u8) -> FieldZoneConfig {
        FieldZoneConfig { zone_id: zone, enabled, karus_count: k, elmorad_count: e, level_min: lo, level_max: hi, mix: DEFAULT_MIX }
    }

    #[test]
    fn empty_event_grid_map_is_walkable_by_terrain_for_bots() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map/moradon_0826.smd");
        let mut raw = ko_protocol::smd::SmdFile::load(&path).expect("load SMD");
        raw.event_grid.iter_mut().for_each(|v| *v = 0);
        let map = MapData::new(raw);
        let (x, z) = nearest_bot_point(&map, 700.0, 700.0, 40).expect("terrain is walkable even with an all-zero event grid");
        assert!(map.is_bot_movable(x, z));
        assert!(!map.is_bot_movable(-5.0, 10.0), "out of bounds stays blocked");
    }

    // ── configuration, caps, determinism ────────────────────────────────

    #[test]
    fn config_is_clamped_to_zone_rules_and_caps() {
        let c = cfg(1, true, 5000, 90, 1, 200).sanitized().unwrap();
        assert_eq!(c.karus_count, MAX_FIELD_BOTS_PER_NATION_ZONE, "technical per nation/zone ceiling");
        assert_eq!(c.elmorad_count, 0, "Luferson is a Karus land: no El Morad bots");
        assert_eq!((c.level_min, c.level_max), (35, MAX_BOT_LEVEL), "zone minimum level, global maximum");
        let m = cfg(21, true, 99, 99, 50, 10).sanitized().unwrap();
        assert_eq!((m.karus_count, m.elmorad_count), (99, 99), "neutral Moradon keeps both nations, no 40 cap");
        assert!(m.level_min <= m.level_max);
        assert!(cfg(71, true, 5, 5, 1, 83).sanitized().is_none(), "PK zones are not field-bot zones");
        assert!(cfg(72, true, 5, 5, 1, 83).sanitized().is_none());
        let mut z = cfg(21, true, 1, 1, 1, 83);
        z.mix = [0; 5];
        assert_eq!(z.sanitized().unwrap().mix, DEFAULT_MIX, "an all-zero mix falls back to the default");
        z.mix = [250, 0, 0, 0, 0];
        assert_eq!(z.sanitized().unwrap().mix[0], 100);
        assert_eq!(cfg(21, false, 10, 10, 1, 83).target_for(NATION_KARUS), 0, "disabled means zero");
    }

    #[test]
    fn nothing_spawns_by_default_and_unsupported_rows_are_dropped() {
        let world = moradon_world();
        for i in 0..6 {
            add_monster(&world, MORADON, 500.0 + i as f32, 500.0, 20, 600);
        }
        manage(&world, 1_000_000);
        assert_eq!(world.bot_count(), 0, "no configuration -> no bots");
        let n = world.field_bots.apply_config([cfg(71, true, 10, 10, 1, 83), cfg(21, false, 10, 10, 1, 83)]);
        assert_eq!(n, 1, "the PK zone row is dropped");
        manage(&world, 1_010_000);
        assert_eq!(world.bot_count(), 0, "a disabled zone spawns nothing");
    }

    #[test]
    fn class_mix_respects_weights() {
        let mut rng = rand::thread_rng();
        for _ in 0..400 {
            assert_eq!(pick_class(&[0, 0, 0, 100, 0], &mut rng), 4, "only priests have weight");
            let c = pick_class(&[50, 50, 0, 0, 0], &mut rng);
            assert!(c == 1 || c == 2);
        }
        let mut seen = HashSet::new();
        for _ in 0..800 {
            seen.insert(pick_class(&DEFAULT_MIX, &mut rng));
        }
        assert_eq!(seen, HashSet::from([1, 2, 3, 4]), "default mix covers the four main classes, no Kurian");
    }

    #[test]
    fn leveling_pace_is_sane() {
        assert_eq!(hits_to_kill(40, 40), 9.0);
        assert!(hits_to_kill(40, 44) > hits_to_kill(40, 40));
        assert!(hits_to_kill(40, 30) < hits_to_kill(40, 40));
        for (b, m) in [(1u8, 1u8), (20, 24), (40, 36), (60, 64), (83, 83), (83, 1), (1, 83)] {
            let h = hits_to_kill(b, m);
            assert!((3.0..=40.0).contains(&h), "{b} vs {m} -> {h}");
        }
        let secs = hits_to_kill(40, 40) * MELEE_COOLDOWN_MS as f32 / 1000.0;
        assert!((8.0..=25.0).contains(&secs), "equal-level kill takes {secs}s");
    }

    // ── population management ───────────────────────────────────────────

    #[test]
    fn manage_fills_exactly_the_target_and_names_are_unique() {
        let world = moradon_world();
        let (ox, oz) = open_spot(&world, MORADON, 0);
        for i in 0..10 {
            add_monster(&world, MORADON, ox + (i % 5) as f32 * 3.0, oz + (i / 5) as f32 * 3.0, 20, 600);
        }
        world.field_bots.apply_config([cfg(MORADON, true, 5, 4, 15, 25)]);
        let mut now = 1_000_000;
        for _ in 0..6 {
            now += MANAGE_INTERVAL_MS;
            manage(&world, now);
        }
        let counts = live_counts(&world);
        assert_eq!(counts.get(&(MORADON, NATION_KARUS)), Some(&5));
        assert_eq!(counts.get(&(MORADON, NATION_ELMORAD)), Some(&4));
        let mut names = HashSet::new();
        let zone = world.get_zone(MORADON).unwrap();
        for e in world.bots.iter() {
            let b = e.value();
            assert_eq!(b.ai_state, BotAiState::Field);
            assert_eq!(b.duration_minutes, 0, "field bots are permanent");
            assert_eq!(b.loyalty, 0);
            assert!((15..=25).contains(&b.level), "level {} outside the configured range", b.level);
            assert!(!b.name.is_empty() && !b.name.starts_with("Bot"), "nickname pool name expected: {}", b.name);
            assert!(names.insert(b.name.clone()), "duplicate nickname {}", b.name);
            assert!(zone.map_data.as_ref().unwrap().is_bot_movable(b.x, b.z), "spawned on blocked terrain");
            assert_eq!(b.need_party, 1, "advertises 'looking for a party'");
        }
        manage(&world, now + MANAGE_INTERVAL_MS);
        assert_eq!(world.bot_count(), 9, "stable once the target is reached");
        // shrinking the target trims the excess, disabling removes everything
        world.field_bots.apply_config([cfg(MORADON, true, 2, 1, 15, 25)]);
        for _ in 0..3 {
            now += MANAGE_INTERVAL_MS;
            manage(&world, now);
        }
        assert_eq!(world.bot_count(), 3);
        world.field_bots.apply_config([cfg(MORADON, false, 2, 1, 15, 25)]);
        manage(&world, now + 10_000);
        assert_eq!(world.bot_count(), 0, "switching a zone off removes its bots");
        manage(&world, now + 20_000);
        assert!(world.field_bots.rt.lock().is_empty(), "runtime state is pruned with the bots");
    }

    #[test]
    fn old_caps_are_gone_but_owner_rules_hold() {
        let world = WorldState::new();
        for z in [21u16, 1, 2, 11, 12] {
            world.ensure_zone(z, 4096);
            let mob_level = if z == 11 || z == 12 { 70 } else { 50 };
            for i in 0..6 {
                add_monster(&world, z, 700.0 + i as f32 * 2.0, 700.0, mob_level, 4000);
            }
        }
        world.field_bots.apply_config([
            cfg(21, true, 99, 99, 40, 60),
            cfg(1, true, 99, 99, 40, 60),
            cfg(2, true, 99, 99, 40, 60),
            cfg(11, true, 99, 99, 66, 78),
            cfg(12, true, 99, 99, 66, 78),
        ]);
        let mut now = 5_000_000;
        for _ in 0..120 {
            now += MANAGE_INTERVAL_MS;
            manage(&world, now);
        }
        let total = live_counts(&world).values().sum::<usize>();
        assert!(total > 150 && total <= MAX_FIELD_BOTS_TOTAL, "the old global cap of 150 is gone: {total}");
        for ((zone, nation), n) in live_counts(&world) {
            assert!(n <= MAX_FIELD_BOTS_PER_NATION_ZONE as usize, "zone {zone} nation {nation}: {n}");
            let owner = zone_profile(zone).unwrap().owner;
            assert!(owner.allows(nation), "nation {nation} must not live in zone {zone}");
        }
    }

    #[test]
    fn bots_spread_over_hunting_grounds() {
        let world = moradon_world();
        let (ax, az) = open_spot(&world, MORADON, 0);
        let (bx, bz) = far_open_spot(&world, MORADON, (ax, az), 200.0);
        for i in 0..6 {
            add_monster(&world, MORADON, ax + i as f32 * 2.0, az, 20, 600);
            add_monster(&world, MORADON, bx + i as f32 * 2.0, bz, 20, 600);
        }
        world.field_bots.apply_config([cfg(MORADON, true, 8, 0, 18, 22)]);
        let mut now = 2_000_000;
        for _ in 0..5 {
            now += MANAGE_INTERVAL_MS;
            manage(&world, now);
        }
        let homes: Vec<(f32, f32)> = world.field_bots.rt.lock().values().map(|s| s.home).collect();
        assert_eq!(homes.len(), 8);
        let near_a = homes.iter().filter(|h| dist2(h.0, h.1, ax, az) < 100.0 * 100.0).count();
        assert!((1..=7).contains(&near_a), "bots must use both grounds ({near_a} of 8 on A)");
    }

    #[test]
    fn alloc_bot_id_wraps_inside_the_band_and_skips_live_ids() {
        use crate::world::BOT_ID_SPAN;
        let world = WorldState::new();
        let keep = bot_ai::spawn_gm_bot(
            &world,
            bot_ai::SpawnGmBotParams { zone_id: 21, x: 10.0, y: 0.0, z: 10.0, class: 1, level: 10, nation: 1, ai_state: BotAiState::Idle },
        );
        assert_eq!(keep, BOT_ID_BASE);
        for _ in 0..(BOT_ID_SPAN * 2 + 17) {
            let id = world.alloc_bot_id();
            assert!((BOT_ID_BASE..BOT_ID_BASE + BOT_ID_SPAN).contains(&id), "id {id} left the band");
            assert_ne!(id, keep, "an id that is still in use was handed out again");
        }
    }

    // ── protection: nobody can farm a field bot ─────────────────────────

    #[test]
    fn only_enemy_players_in_pvp_zones_can_hurt_field_bots() {
        let world = WorldState::new();
        world.set_zone_info(1, zone_info("Luferson", true));
        world.set_zone_info(MORADON, zone_info("Moradon", false));
        let karus_bot = add_bot(&world, 1, NATION_KARUS, 1, 40, 100.0, 100.0);
        let moradon_bot = add_bot(&world, MORADON, NATION_KARUS, 1, 40, 100.0, 100.0);
        let _rx1 = add_player(&world, 1, NATION_KARUS, 40, 1, 110.0, 100.0);
        let _rx2 = add_player(&world, 2, NATION_ELMORAD, 40, 1, 110.0, 100.0);
        let _rx3 = add_player(&world, 3, NATION_ELMORAD, 40, MORADON, 110.0, 100.0);
        let kb = world.get_bot(karus_bot).unwrap();
        let mb = world.get_bot(moradon_bot).unwrap();
        assert!(!player_can_hurt(&world, 1, &kb), "same nation: never");
        assert!(player_can_hurt(&world, 2, &kb), "enemy nation in a PvP land may fight the defenders");
        assert!(!player_can_hurt(&world, 3, &mb), "Moradon forbids cross-nation attacks");
        assert!(!player_can_hurt(&world, 999, &kb), "unknown attacker");
        // non-field bots keep their own rules
        let mut pk = kb.clone();
        pk.ai_state = BotAiState::Pk;
        pk.original_ai_state = BotAiState::Pk;
        assert!(player_can_hurt(&world, 1, &pk));
        // a GM can always clean up
        world.update_character_stats(1, |c| c.authority = 0);
        assert!(player_can_hurt(&world, 1, &kb));
    }

    #[test]
    fn killing_a_field_bot_pays_nothing() {
        let world = WorldState::new();
        world.set_zone_info(1, zone_info("Luferson", true));
        let _rx = add_player(&world, 2, NATION_ELMORAD, 40, 1, 110.0, 100.0);
        world.update_character_stats(2, |c| {
            c.gold = 1_000;
            c.loyalty = 500;
            c.loyalty_monthly = 500;
        });
        let id = add_bot(&world, 1, NATION_KARUS, 1, 40, 100.0, 100.0);
        world.update_bot(id, |b| {
            b.hp = 1;
            b.last_attacker_id = 2;
        });
        bot_ai::bot_on_death(&world, id, 1_000);
        let ch = world.get_character_info(2).unwrap();
        assert_eq!(ch.gold, 1_000, "no gold");
        assert_eq!(ch.loyalty, 500, "no NP");
        assert_eq!(ch.loyalty_monthly, 500, "no monthly NP");
        let dead = world.get_bot(id).unwrap();
        assert_eq!(dead.presence, BotPresence::Dead);
        assert_eq!(dead.original_ai_state, BotAiState::Field, "remembered so it respawns as a field bot");
        assert!(is_field_bot(&dead));
    }

    // ── chat understanding ──────────────────────────────────────────────

    #[test]
    fn intents_and_languages() {
        assert_eq!(detect_intent("selam"), Some(Intent::Greeting));
        assert_eq!(detect_intent("Merhaba kanka"), Some(Intent::Greeting));
        assert_eq!(detect_intent("hello there"), Some(Intent::Greeting));
        assert_eq!(detect_intent("nasılsın"), Some(Intent::HowAreYou));
        assert_eq!(detect_intent("how are you"), Some(Intent::HowAreYou));
        assert_eq!(detect_intent("party atalım mı"), Some(Intent::PartyRequest));
        assert_eq!(detect_intent("anyone want to join pt?"), Some(Intent::PartyRequest));
        assert_eq!(detect_intent("kaç level"), Some(Intent::AskLevel));
        assert_eq!(detect_intent("what lvl are you"), Some(Intent::AskLevel));
        assert_eq!(detect_intent("bot musun"), Some(Intent::AreYouBot));
        assert_eq!(detect_intent("are you a bot?"), Some(Intent::AreYouBot));
        assert_eq!(detect_intent("teşekkürler"), Some(Intent::Thanks));
        assert_eq!(detect_intent("thx"), Some(Intent::Thanks));
        assert_eq!(detect_intent("görüşürüz"), Some(Intent::Bye));
        assert_eq!(detect_intent("dur"), Some(Intent::Hold));
        assert_eq!(detect_intent("takip et"), Some(Intent::Follow));
        assert_eq!(detect_intent("ayrıl"), Some(Intent::Leave));
        assert_eq!(detect_intent("kimse yok mu burada acaba ne yapsak simdi"), None);
        // control words inside a long sentence are just talk
        assert_eq!(detect_intent("bence biraz bekle ve sonra tekrar bakarız bir daha"), None);
        assert!(is_turkish("nasılsın"));
        assert!(is_turkish("selam abi"));
        assert!(!is_turkish("hello there"));
    }

    #[test]
    fn every_intent_has_varied_replies_in_both_languages_and_mentions_the_bot() {
        let world = WorldState::new();
        let id = add_bot(&world, MORADON, NATION_KARUS, 4, 47, 100.0, 100.0);
        let bot = world.get_bot(id).unwrap();
        let all = [
            Intent::Greeting,
            Intent::HowAreYou,
            Intent::PartyRequest,
            Intent::AskLevel,
            Intent::AreYouBot,
            Intent::Thanks,
            Intent::Bye,
            Intent::Leave,
            Intent::Hold,
            Intent::Follow,
        ];
        for intent in all {
            for turkish in [true, false] {
                let texts: HashSet<String> = (0..9).map(|v| reply_text(intent, turkish, &bot, "Ali", v)).collect();
                assert!(texts.iter().all(|t| !t.is_empty() && t.len() < 190), "{intent:?}");
                if matches!(intent, Intent::Greeting | Intent::HowAreYou | Intent::PartyRequest | Intent::AreYouBot) {
                    assert!(texts.len() >= 2, "{intent:?}/{turkish} should have several variants");
                }
            }
        }
        let lvl = reply_text(Intent::AskLevel, false, &bot, "Ali", 0);
        assert!(lvl.contains("47") && lvl.contains("Priest"), "{lvl}");
        let lvl_tr = reply_text(Intent::AskLevel, true, &bot, "Ali", 0);
        assert!(lvl_tr.contains("47") && lvl_tr.contains("Rahip"), "{lvl_tr}");
        assert!(reply_text(Intent::AreYouBot, false, &bot, "Ali", 0).to_lowercase().contains("bot"));
        assert!(reply_text(Intent::AreYouBot, true, &bot, "Ali", 1).to_lowercase().contains("bot"));
    }

    #[test]
    fn nearby_chat_is_answered_once_by_the_nearest_bot_and_rate_limited() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let near = add_bot(&world, MORADON, NATION_KARUS, 1, 30, x + 4.0, z);
        let _far = add_bot(&world, MORADON, NATION_KARUS, 2, 30, x + 200.0, z);
        let mut rx = add_player(&world, 7, NATION_KARUS, 30, MORADON, x, z);
        on_nearby_chat(&world, 7, "selam");
        on_nearby_chat(&world, 7, "selam"); // immediate repeat: global/bot cooldown
        assert_eq!(world.field_bots.replies.lock().len(), 1, "one answer only");
        // nothing is said before the reply delay has passed
        deliver_replies(&world, bot_ai::tick_ms());
        assert!(drain(&mut rx).iter().all(|p| p.opcode != Opcode::WizChat as u8));
        deliver_replies(&world, bot_ai::tick_ms() + 10_000);
        let chats: Vec<_> = drain(&mut rx).into_iter().filter(|p| p.opcode == Opcode::WizChat as u8).collect();
        assert_eq!(chats.len(), 1);
        let mut r = ko_protocol::PacketReader::new(&chats[0].data);
        assert_eq!(r.read_u8(), Some(ChatType::General as u8));
        let _nation = r.read_u8();
        assert_eq!(r.read_u32(), Some(near), "spoken by the nearest bot");
        // unrelated chat and out-of-range speakers get no answer
        on_nearby_chat(&world, 7, "random words about nothing in particular here");
        assert!(world.field_bots.replies.lock().is_empty());
        let _rx_far = add_player(&world, 8, NATION_KARUS, 30, MORADON, x + 900.0, z + 600.0);
        on_nearby_chat(&world, 8, "selam");
        assert!(world.field_bots.replies.lock().is_empty(), "nobody in hearing range");
    }

    #[test]
    fn whispers_are_answered_privately() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let bot = add_bot(&world, MORADON, NATION_KARUS, 3, 33, x, z);
        let mut rx = add_player(&world, 7, NATION_KARUS, 30, MORADON, x + 5.0, z);
        let name = world.get_bot(bot).unwrap().name;
        assert_eq!(find_field_bot_by_name(&world, &name.to_ascii_uppercase()).map(|b| b.id), Some(bot));
        on_whisper(&world, 7, bot, "how are you doing");
        deliver_replies(&world, bot_ai::tick_ms() + 10_000);
        let pkts = drain(&mut rx);
        let chat = pkts.iter().find(|p| p.opcode == Opcode::WizChat as u8).expect("whisper reply");
        let mut r = ko_protocol::PacketReader::new(&chat.data);
        assert_eq!(r.read_u8(), Some(ChatType::Private as u8));
        // an unrecognised whisper still gets a canned answer; a player-owned id is ignored
        world.field_bots.rt.lock().get_mut(&bot).unwrap().last_chat_ms = 0;
        world.field_bots.last_global_chat_ms.store(0, Ordering::Relaxed);
        on_whisper(&world, 7, bot, "asdf qwer");
        assert_eq!(world.field_bots.replies.lock().len(), 1);
        world.field_bots.replies.lock().clear();
        on_whisper(&world, 7, 7, "hi");
        assert!(world.field_bots.replies.lock().is_empty());
    }

    // ── hunting on the real map ─────────────────────────────────────────

    #[test]
    fn solo_bot_walks_fights_and_kills_a_monster_for_nobody() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        // a player far away only keeps the zone "active"
        let _rx = add_player(&world, 5, NATION_KARUS, 30, MORADON, x + 600.0, z + 600.0);
        world.update_character_stats(5, |c| c.gold = 777);
        let mob = add_monster(&world, MORADON, x + 14.0, z, 20, 600);
        let bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        let mut killed = false;
        let mut closest = f32::MAX;
        let mut now = bot_ai::tick_ms();
        for _ in 0..600 {
            now += T_STEP;
            tick_with_budget(&world, now, Duration::from_secs(5));
            let b = world.get_bot(bot).unwrap();
            let m = world.get_npc_ai(mob).unwrap();
            closest = closest.min(dist2(b.x, b.z, m.cur_x, m.cur_z).sqrt());
            if world.get_npc_hp(mob) == Some(0) {
                killed = true;
                break;
            }
        }
        assert!(killed, "the bot never killed the lone monster");
        assert!(closest < 5.0, "it had to walk up to it (closest {closest})");
        assert_eq!(world.get_npc_ai(mob).unwrap().state, NpcState::Dead, "the normal respawn path takes over");
        assert!(!world.npc_has_damage_record(mob), "no damage rights for anybody");
        assert_eq!(world.get_character_info(5).unwrap().gold, 777);
        let b = world.get_bot(bot).unwrap();
        assert!(b.skill_cooldown[0] > 0, "it attacked with a class skill");
        assert!(b.is_alive());
    }

    #[test]
    fn solo_bot_never_takes_a_monster_a_player_is_fighting_or_standing_next_to() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let _rx = add_player(&world, 5, NATION_KARUS, 20, MORADON, x - 30.0, z + 20.0);
        let touched = add_monster(&world, MORADON, x + 8.0, z, 20, 600);
        world.record_npc_damage(touched, 5, 10);
        let chasing = add_monster(&world, MORADON, x + 8.0, z + 6.0, 20, 600);
        world.update_npc_ai(chasing, |a| {
            a.target_id = Some(5);
            a.state = NpcState::Tracing;
        });
        let guarded = add_monster(&world, MORADON, x + 8.0, z - 6.0, 20, 600);
        move_player(&world, 5, x + 8.0, z - 12.0); // within POLITE_RADIUS of `guarded`
        let _bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        run(&world, bot_ai::tick_ms(), 200);
        assert_eq!(world.get_npc_hp(touched), Some(600), "monster a player already touched");
        assert_eq!(world.get_npc_hp(chasing), Some(600), "monster chasing a player");
        assert_eq!(world.get_npc_hp(guarded), Some(600), "monster next to a real player");
    }

    #[test]
    fn frozen_without_players_and_two_bots_never_share_a_monster() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let mob = add_monster(&world, MORADON, x + 10.0, z, 20, 800);
        let a = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        let b = add_bot(&world, MORADON, NATION_KARUS, 2, 20, x, z + 3.0);
        let before = (world.get_bot(a).unwrap().x, world.get_bot(a).unwrap().z);
        let mut now = run(&world, bot_ai::tick_ms(), 40);
        assert_eq!(world.get_npc_hp(mob), Some(800), "no player in the zone: bots are frozen");
        assert_eq!((world.get_bot(a).unwrap().x, world.get_bot(a).unwrap().z), before);
        // a player arrives (far away) -> the bots wake up; only one of them may own the monster
        let _rx = add_player(&world, 5, NATION_KARUS, 20, MORADON, x + 700.0, z + 700.0);
        for _ in 0..6 {
            now = run(&world, now, 1);
            let owners: Vec<BotId> = [a, b]
                .into_iter()
                .filter(|id| world.field_bots.rt.lock().get(id).is_some_and(|s| s.claim == Some(mob)))
                .collect();
            assert!(owners.len() <= 1, "both bots claimed the same monster: {owners:?}");
        }
    }

    #[test]
    fn hurt_bots_sit_down_and_get_back_up() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let _rx = add_player(&world, 5, NATION_KARUS, 20, MORADON, x + 600.0, z + 600.0);
        let bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        world.update_bot(bot, |b| b.hp = b.max_hp / 5);
        let mut now = run(&world, bot_ai::tick_ms(), 2);
        assert_eq!(world.field_bots.rt.lock().get(&bot).unwrap().mode, Mode::Rest);
        assert_eq!(world.get_bot(bot).unwrap().presence, BotPresence::Sitting);
        let pos = {
            let b = world.get_bot(bot).unwrap();
            (b.x, b.z)
        };
        for _ in 0..1_000 {
            now = run(&world, now, 1);
            if world.field_bots.rt.lock().get(&bot).unwrap().mode == Mode::Hunt {
                break;
            }
        }
        let b = world.get_bot(bot).unwrap();
        assert_eq!(world.field_bots.rt.lock().get(&bot).unwrap().mode, Mode::Hunt, "rested back to health");
        assert_eq!(b.presence, BotPresence::Standing);
        assert!(b.hp as i32 * 100 / b.max_hp as i32 >= REST_LEAVE_HP_PERCENT);
        assert_eq!((b.x, b.z), pos, "resting does not walk around");
    }

    #[test]
    fn idle_bots_keep_moving_between_hunting_spots() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let _rx = add_player(&world, 5, NATION_KARUS, 20, MORADON, x + 600.0, z + 600.0);
        // monsters exist (so the ground is valid) but are out of the bot's level band: nothing to fight
        for i in 0..4 {
            add_monster(&world, MORADON, x + 10.0 + i as f32 * 2.0, z, 60, 5000);
        }
        let bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        let mut spots = HashSet::new();
        let mut now = bot_ai::tick_ms();
        let zone = world.get_zone(MORADON).unwrap();
        for _ in 0..240 {
            now = run(&world, now, 1);
            let b = world.get_bot(bot).unwrap();
            assert!(
                zone.map_data.as_ref().unwrap().is_bot_movable(b.x, b.z),
                "bot walked onto blocked terrain at ({}, {})",
                b.x,
                b.z
            );
            spots.insert(((b.x / 10.0) as i32, (b.z / 10.0) as i32));
        }
        assert!(spots.len() >= 3, "a bot without prey must wander, visited {} cells", spots.len());
    }

    #[test]
    fn dead_bots_come_back_as_field_bots_at_their_ground() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        for i in 0..4 {
            add_monster(&world, MORADON, x + 5.0 + i as f32, z, 20, 600);
        }
        let bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        world.update_bot(bot, |b| {
            b.x = x + 40.0;
            b.z = z + 40.0;
            b.hp = 1;
        });
        bot_ai::bot_on_death(&world, bot, bot_ai::tick_ms());
        assert_eq!(world.get_bot(bot).unwrap().ai_state, BotAiState::Idle);
        world.update_bot(bot, |b| b.regene_at_ms = 1); // due
        bot_ai::tick_bots(&world);
        let b = world.get_bot(bot).unwrap();
        assert_eq!(b.presence, BotPresence::Standing);
        assert_eq!(b.ai_state, BotAiState::Field, "restored to the field AI");
        assert_eq!(b.hp, b.max_hp);
        assert!(dist2(b.x, b.z, x, z) < 25.0 * 25.0, "respawned on its hunting ground, not where it died");
    }

    #[test]
    fn crossing_a_region_border_tells_observers_who_gained_or_lost_sight() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        // the bot steps from region R to R+1 along x
        let border = (calc_region(x) + 1) as f32 * 48.0;
        let bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, border - 0.5, z);
        let old_region = world.get_bot(bot).unwrap().region_x;
        let mut old_only = add_player(&world, 10, NATION_KARUS, 20, MORADON, (old_region as f32 - 1.0) * 48.0 + 10.0, z);
        let mut both = add_player(&world, 11, NATION_KARUS, 20, MORADON, old_region as f32 * 48.0 + 10.0, z);
        let mut new_only = add_player(&world, 12, NATION_KARUS, 20, MORADON, (old_region as f32 + 2.0) * 48.0 + 10.0, z);
        let snapshot = world.get_bot(bot).unwrap();
        commit_move(&world, &snapshot, border + 0.5, 0.0, z, 3, 1);
        let kinds = |rx: &mut mpsc::UnboundedReceiver<Arc<Packet>>| -> Vec<(u8, u8)> {
            drain(rx).into_iter().map(|p| (p.opcode, p.data.first().copied().unwrap_or(0))).collect()
        };
        let inout = Opcode::WizUserInout as u8;
        assert!(kinds(&mut old_only).contains(&(inout, 2)), "left the view of the old-only observer: OUT");
        assert!(kinds(&mut new_only).contains(&(inout, 1)), "entered the view of the new-only observer: IN");
        let k = kinds(&mut both);
        assert!(!k.contains(&(inout, 1)) && !k.contains(&(inout, 2)), "an observer that kept sight gets no IN/OUT");
        assert_eq!(world.get_bot(bot).unwrap().region_x, old_region + 1);
    }

    #[test]
    fn small_work_budget_still_serves_every_bot_round_robin() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let _rx = add_player(&world, 5, NATION_KARUS, 20, MORADON, x + 600.0, z + 600.0);
        for i in 0..8 {
            add_monster(&world, MORADON, x + 20.0 + i as f32, z, 20, 700);
        }
        let ids: Vec<BotId> = (0..8).map(|i| add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z + i as f32 * 3.0)).collect();
        let mut now = bot_ai::tick_ms();
        let mut served = HashSet::new();
        for _ in 0..30 {
            now += T_STEP;
            let before: Vec<u64> = ids.iter().map(|id| world.get_bot(*id).unwrap().last_move_ms).collect();
            tick_with_budget(&world, now, Duration::ZERO); // exactly one bot per call
            for (i, id) in ids.iter().enumerate() {
                if world.get_bot(*id).unwrap().last_move_ms != before[i] {
                    served.insert(*id);
                }
            }
        }
        assert_eq!(served.len(), ids.len(), "a zero budget must still rotate through every bot");
    }

    #[test]
    fn rare_special_and_event_monsters_are_never_prey() {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        let _rx = add_player(&world, 5, NATION_KARUS, 20, MORADON, x + 600.0, z + 600.0);
        // long-respawn "mini boss": regen 15 minutes
        let rare = add_monster(&world, MORADON, x + 8.0, z, 20, 600);
        world.update_npc_ai(rare, |a| a.regen_time_ms = 900_000);
        // captain / quest templates are named "[...]"
        let mut captain = monster_template(4000, 20, 600, 40);
        captain.name = "[Captine] Test".to_string();
        world.insert_npc_template(captain);
        let cap = world.spawn_event_npc(4000, true, MORADON, x + 8.0, z + 4.0, 1)[0];
        let mut inst = (*world.get_npc_instance(cap).unwrap()).clone();
        inst.is_event_npc = false;
        world.insert_npc_instance(inst);
        world.update_npc_ai(cap, |a| a.regen_time_ms = 30_000);
        // a real event spawn (no respawn) and a boss type
        let event = world.spawn_event_npc(3020, true, MORADON, x + 8.0, z - 4.0, 1);
        let mut boss_t = monster_template(4001, 20, 600, 40);
        boss_t.npc_type = 3;
        world.insert_npc_template(boss_t);
        let boss = world.spawn_event_npc(4001, true, MORADON, x - 8.0, z, 1);
        for id in [rare, cap].into_iter().chain(event).chain(boss) {
            let npc = world.get_npc_instance(id);
            assert!(
                prey_view(&world, id).is_none() || npc.is_none(),
                "npc {id} must not be prey"
            );
        }
        let _bot = add_bot(&world, MORADON, NATION_KARUS, 1, 20, x, z);
        run(&world, bot_ai::tick_ms(), 200);
        for id in [rare, cap] {
            assert_eq!(world.get_npc_hp(id), Some(600), "npc {id} was attacked");
        }
        // none of them forms a hunting ground either
        assert!(grounds_for(&world, MORADON, bot_ai::tick_ms()).is_empty());
    }

    // ── party play ──────────────────────────────────────────────────────

    fn party_setup() -> (WorldState, f32, f32) {
        let world = moradon_world();
        let (x, z) = open_spot(&world, MORADON, 0);
        (world, x, z)
    }

    fn last_party_error(rx: &mut mpsc::UnboundedReceiver<Arc<Packet>>) -> Option<i16> {
        drain(rx).into_iter().rev().find_map(|p| {
            if p.opcode != Opcode::WizParty as u8 || p.data.first() != Some(&crate::handler::party::PARTY_INSERT) {
                return None;
            }
            // error packet: [PARTY_INSERT][i16 code]; success packets carry u16 1 plus more data
            (p.data.len() == 3).then(|| i16::from_le_bytes([p.data[1], p.data[2]]))
        })
    }

    fn join_party(world: &WorldState, player: SessionId, bot: BotId) {
        let name = world.get_bot(bot).unwrap().name;
        assert!(try_party_invite(world, player, &name, 0));
        assert!(world.get_party_id(bot as SessionId).is_some());
    }

    #[test]
    fn invitation_is_accepted_and_the_party_has_a_bot_cap() {
        let (world, x, z) = party_setup();
        let mut rx = add_player(&world, 7, NATION_KARUS, 40, MORADON, x, z);
        let bot = add_bot(&world, MORADON, NATION_KARUS, 4, 40, x + 5.0, z);
        let name = world.get_bot(bot).unwrap().name;

        assert!(!try_party_invite(&world, 7, "NoSuchPlayerHere", 0), "not a bot: the caller handles it");
        assert!(try_party_invite(&world, 7, &name, 0), "handled");
        let pid = world.get_party_id(7).expect("a party was created for the inviter");
        let party = world.get_party(pid).unwrap();
        assert!(party.is_leader(7) && party.contains(bot as SessionId));
        assert_eq!(world.get_party_id(bot as SessionId), Some(pid));
        assert_eq!(world.get_bot(bot).unwrap().need_party, 0);
        let pkts = drain(&mut rx);
        assert!(
            pkts.iter().any(|p| p.opcode == Opcode::WizParty as u8 && p.data.first() == Some(&crate::handler::party::PARTY_INSERT)),
            "the leader's UI gets the new member"
        );

        // already grouped -> generic error
        assert!(try_party_invite(&world, 7, &name, 0));
        assert_eq!(last_party_error(&mut rx), Some(PARTY_ERR_GENERIC));

        // capacity: at most MAX_BOTS_PER_PARTY bots
        let mut names = vec![name];
        for i in 0..MAX_BOTS_PER_PARTY {
            let b = add_bot(&world, MORADON, NATION_KARUS, 1, 40, x + 6.0 + i as f32, z + 2.0);
            names.push(world.get_bot(b).unwrap().name);
        }
        for n in &names[1..MAX_BOTS_PER_PARTY] {
            assert!(try_party_invite(&world, 7, n, 0));
        }
        let bots_in_party = world.get_party(pid).unwrap().active_members().iter().filter(|s| !is_real(**s)).count();
        assert_eq!(bots_in_party, MAX_BOTS_PER_PARTY);
        let refused = names[MAX_BOTS_PER_PARTY].clone();
        assert!(try_party_invite(&world, 7, &refused, 0));
        assert_eq!(last_party_error(&mut rx), Some(PARTY_ERR_DECLINED), "bot cap per party");
        assert!(world.get_party_id(find_field_bot_by_name(&world, &refused).unwrap().id as SessionId).is_none());
    }

    #[test]
    fn invitation_refusals_zone_level_distance_health_and_nation() {
        let (world, x, z) = party_setup();
        world.set_zone_info(1, zone_info("Luferson", true));
        let mut rx = add_player(&world, 7, NATION_KARUS, 50, MORADON, x, z);
        let far = add_bot(&world, MORADON, NATION_KARUS, 1, 50, x + 300.0, z);
        let weak = add_bot(&world, MORADON, NATION_KARUS, 1, 50, x + 6.0, z);
        let lowlvl = add_bot(&world, MORADON, NATION_KARUS, 1, 30, x + 7.0, z);
        let rival = add_bot(&world, MORADON, NATION_ELMORAD, 1, 50, x + 8.0, z);
        let elsewhere = add_bot(&world, 1, NATION_KARUS, 1, 50, x, z);
        let name = |id: BotId| world.get_bot(id).unwrap().name;

        assert!(try_party_invite(&world, 7, &name(elsewhere), 0));
        assert_eq!(last_party_error(&mut rx), Some(PARTY_ERR_ZONE), "other zone");
        assert!(try_party_invite(&world, 7, &name(far), 0));
        assert_eq!(last_party_error(&mut rx), Some(PARTY_ERR_DECLINED), "too far away");
        world.update_bot(weak, |b| b.hp = b.max_hp / 10);
        assert!(try_party_invite(&world, 7, &name(weak), 0));
        assert_eq!(last_party_error(&mut rx), Some(PARTY_ERR_DECLINED), "too hurt");
        assert!(try_party_invite(&world, 7, &name(lowlvl), 0));
        assert_eq!(last_party_error(&mut rx), Some(PARTY_ERR_LEVEL), "level gap beyond the normal party rule");
        assert!(world.get_party_id(7).is_none(), "refusals never leave a half-created party behind");
        // Moradon is a partner-party zone: the other nation may join
        assert!(try_party_invite(&world, 7, &name(rival), 0));
        assert!(world.get_party_id(rival as SessionId).is_some(), "cross-nation party in Moradon");

        // an enemy-nation bot in a normal land is refused
        let (world2, x2, z2) = party_setup();
        world2.set_zone_info(1, zone_info("Luferson", true));
        let mut rx2 = add_player(&world2, 7, NATION_KARUS, 50, 1, x2, z2);
        let rival2 = add_bot(&world2, 1, NATION_ELMORAD, 1, 50, x2 + 3.0, z2);
        assert!(try_party_invite(&world2, 7, &world2.get_bot(rival2).unwrap().name, 0));
        assert_eq!(last_party_error(&mut rx2), Some(PARTY_ERR_NATION));
    }

    #[test]
    fn party_bots_follow_the_leader_and_stay_spread_out() {
        let (world, x, z) = party_setup();
        let _rx = add_player(&world, 7, NATION_KARUS, 40, MORADON, x, z);
        let bots: Vec<BotId> = (0..3)
            .map(|i| add_bot(&world, MORADON, NATION_KARUS, 1 + i as u16, 40, x + 2.0, z + 2.0 * i as f32))
            .collect();
        for b in &bots {
            join_party(&world, 7, *b);
        }
        // the leader walks ~55 units away
        let (tx, tz) = near_open_spot(&world, MORADON, (x, z), 50.0, 90.0);
        move_player(&world, 7, tx, tz);
        let mut now = bot_ai::tick_ms();
        let zone = world.get_zone(MORADON).unwrap();
        for _ in 0..300 {
            now = run(&world, now, 1);
            for b in &bots {
                let bot = world.get_bot(*b).unwrap();
                assert!(zone.map_data.as_ref().unwrap().is_bot_movable(bot.x, bot.z));
            }
        }
        let pos: Vec<(f32, f32)> = bots
            .iter()
            .map(|b| {
                let bot = world.get_bot(*b).unwrap();
                (bot.x, bot.z)
            })
            .collect();
        for p in &pos {
            assert!(dist2(p.0, p.1, tx, tz).sqrt() < 14.0, "bot did not catch up with its leader: {p:?}");
        }
        for i in 0..pos.len() {
            for j in (i + 1)..pos.len() {
                assert!(dist2(pos[i].0, pos[i].1, pos[j].0, pos[j].1).sqrt() > 1.5, "bots {i} and {j} clumped");
            }
        }
    }

    #[test]
    fn party_bots_finish_engaged_monsters_for_the_party() {
        let (world, x, z) = party_setup();
        let _rx = add_player(&world, 7, NATION_KARUS, 40, MORADON, x, z);
        let engaged = add_monster(&world, MORADON, x + 8.0, z, 40, 3000);
        let bystander = add_monster(&world, MORADON, x + 8.0, z + 15.0, 40, 3000);
        let bots: Vec<BotId> = (0..3)
            .map(|i| add_bot(&world, MORADON, NATION_KARUS, 1 + i as u16, 40, x + 2.0, z + i as f32 * 2.0))
            .collect();
        for b in &bots {
            join_party(&world, 7, *b);
        }
        // unengaged monsters are left alone, no matter how close
        let mut now = run(&world, bot_ai::tick_ms(), 80);
        assert_eq!(world.get_npc_hp(engaged), Some(3000), "nobody fought it yet");
        assert_eq!(world.get_npc_hp(bystander), Some(3000), "an AFK leader cannot be carried");
        // the leader pulls one monster: the bots now help...
        world.record_npc_damage(engaged, 7, 1);
        world.update_npc_ai(engaged, |a| {
            a.target_id = Some(7);
            a.state = NpcState::Fighting;
        });
        for _ in 0..400 {
            now = run(&world, now, 1);
        }
        assert_eq!(world.get_npc_hp(engaged), Some(0), "the party bots finish it themselves");
        assert!(
            PENDING_KILLS.lock().iter().any(|(killer, npc)| *killer == 7 && *npc == engaged),
            "the kill is credited to the real party member"
        );
        assert_eq!(world.get_npc_hp(bystander), Some(3000), "still ignoring the unengaged one");
        // damage rights belong to real players only
        assert!(
            world.get_npc_damage_entries(engaged).iter().all(|(sid, _)| (*sid as u32) < BOT_ID_BASE),
            "bots never appear in the damage list"
        );
    }

    #[test]
    fn party_bots_leave_when_told_to_and_when_nobody_real_is_left() {
        let (world, x, z) = party_setup();
        let _rx = add_player(&world, 7, NATION_KARUS, 40, MORADON, x, z);
        let a = add_bot(&world, MORADON, NATION_KARUS, 1, 40, x + 2.0, z);
        let b = add_bot(&world, MORADON, NATION_KARUS, 4, 40, x + 4.0, z);
        join_party(&world, 7, a);
        join_party(&world, 7, b);
        let pid = world.get_party_id(7).unwrap();
        // "leave" from a non-leader does nothing, from the leader everyone goes
        assert!(!on_party_chat(&world, pid, 8, "ayrıl"));
        assert!(world.get_party_id(a as SessionId).is_some());
        assert!(on_party_chat(&world, pid, 7, "ayrıl"));
        assert!(world.get_party_id(a as SessionId).is_none() && world.get_party_id(b as SessionId).is_none());
        assert!(world.get_party(pid).is_none(), "an emptied party is disbanded");
        assert_eq!(world.get_bot(a).unwrap().need_party, 1, "looking for a party again");

        // the leader logs out: the bot notices and walks away from the party
        join_party(&world, 7, a);
        world.unregister_session(7);
        run(&world, bot_ai::tick_ms(), 80); // 20 s of simulated time
        assert!(world.get_party_id(a as SessionId).is_none(), "bot stayed in a party nobody is in");
    }

    #[test]
    fn hold_and_follow_commands_in_party_chat() {
        let (world, x, z) = party_setup();
        let _rx = add_player(&world, 7, NATION_KARUS, 40, MORADON, x, z);
        let a = add_bot(&world, MORADON, NATION_KARUS, 1, 40, x + 2.0, z);
        join_party(&world, 7, a);
        let pid = world.get_party_id(7).unwrap();
        let (tx, tz) = near_open_spot(&world, MORADON, (x, z), 50.0, 90.0);
        assert!(on_party_chat(&world, pid, 7, "bekle"));
        move_player(&world, 7, tx, tz);
        let start = (world.get_bot(a).unwrap().x, world.get_bot(a).unwrap().z);
        let now = run(&world, bot_ai::tick_ms(), 60);
        let held = (world.get_bot(a).unwrap().x, world.get_bot(a).unwrap().z);
        assert_eq!(start, held, "a held bot stays where it is");
        assert!(on_party_chat(&world, pid, 7, "takip et"));
        run(&world, now, 200);
        let b = world.get_bot(a).unwrap();
        assert!(dist2(b.x, b.z, tx, tz).sqrt() < 14.0, "released bot follows again");
    }
}
