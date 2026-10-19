//! Bot brain -- the shared, player-like decision layer for runtime bots.
//!
//! `bot_ai.rs` owns the mechanics (packets, damage, skill tables, navigation),
//! `field_bots.rs` the PvE hunters and `event_bots*.rs` the event rooms. This
//! module owns the *decisions* and the player-like imperfections that all of
//! them share. Design and knobs: `docs/BOT_AI.md`.
//!
//! The layer is a utility-scoring state machine, evaluated once per AI tick
//! (250 ms) for every PK bot:
//!
//! 1. **Perception** -- one zone snapshot per tick (players + bots, bucketed on
//!    a coarse grid). Every bot queries its surroundings from the snapshot, so
//!    the cost per tick is `O(units)` for the snapshot plus `O(nearby)` per bot;
//!    never `bots x bots x players`.
//! 2. **Threat assessment** -- own HP/MP ratio, enemies vs allies within
//!    [`THREAT_RADIUS`], whether somebody hit the bot recently, party health.
//! 3. **Decision** -- [`decide`] picks a [`Mode`]: `Engage`, `Kite`, `Retreat`,
//!    `HealUp`, `Regroup` or `Objective`. `Retreat` runs to the nearest friendly
//!    group (or away from the enemies); `HealUp` ends when the bot is healed
//!    again, so a bot that fled comes back.
//! 4. **Target selection** -- [`pick_target`] scores every candidate (low HP,
//!    priests, isolated targets, whoever is hitting me, stickiness, distance)
//!    and, with a configurable chance, makes a *mistake* and takes the second
//!    best one.
//! 5. **Execution** -- a human-like reaction delay before the first hit on a
//!    new target, a cast-time movement lock after every cast, no cast outside
//!    the skill's own range (`bot_attack_range_for_skill`), mana management
//!    (basic attack / MP potion when dry), class formation around the target
//!    (warrior fronts, rogue flanks, mage/priest stay at range) and natural
//!    party spacing (3-8 m, per member, reshuffled).
//! 6. **Objective plug-in** -- [`Objective`] is implemented per event: PK zones
//!    (hotspots / field patrol), Chaos (free for all), BDW and Juraid (driven by
//!    their event module, the trait adapters only document the contract).
//!
//! Consumables are *virtual*: every bot gets a configurable stock of HP/MP
//! potions, transformation scrolls and buff scrolls with the real 2.4 s potion
//! cooldown; no inventory rows exist. Kills of PK bots pay a small, configurable
//! share of a player kill's NP with an anti-farm window per (killer, bot) and a
//! grace period after a respawn. Event bots stay reward free.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use parking_lot::{Mutex, RwLock};
use rand::Rng;
use tracing::{debug, trace};

use crate::magic_constants::{MAGIC_CASTING, MAGIC_EFFECTING};
use crate::systems::bot_ai;
use crate::world::types::{ZONE_ARDREAM, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE};
use crate::world::{BotAiState, BotId, BotInstance, WorldState, BOT_ID_BASE, NATION_KARUS};
use crate::zone::{calc_region, SessionId};

// ── Tuning that is not exposed as a knob ─────────────────────────────────

/// Radius of the threat assessment (world units).
pub const THREAT_RADIUS: f32 = 30.0;
/// A bot counts as "being hit" for this long after its HP dropped.
const ATTACKED_MEMORY_MS: u64 = 3_000;
/// Re-evaluate the current target at most this often (stickiness).
const TARGET_STICK_MS: u64 = 1_500;
/// Formation spacing/angle is reshuffled in this interval (ms).
const RESHUFFLE_MIN_MS: u64 = 12_000;
const RESHUFFLE_MAX_MS: u64 = 28_000;
/// A hotspot bot picks a new hotspot after this long.
const HOTSPOT_MIN_MS: u64 = 90_000;
const HOTSPOT_MAX_MS: u64 = 240_000;
/// Radius around a hotspot inside which the bot wanders.
const HOTSPOT_RADIUS: f32 = 14.0;
/// A standing bot picks a new wander point this often.
const WANDER_MIN_MS: u64 = 2_500;
const WANDER_MAX_MS: u64 = 6_000;
/// Distance to the bot-party leader above which a member walks back.
const BOT_PARTY_COHESION: f32 = 24.0;
/// HP ratio at which `HealUp` / `Retreat` end.
const RECOVERED_HP_RATIO: f32 = 0.70;
/// Nearest-enemy distance below which a ranged class wants to kite.
const KITE_TRIGGER: f32 = 6.5;
/// Casters/priests keep at least this distance from their target.
const RANGED_KEEP_MIN: f32 = 8.0;
/// Verdicts about a bot kill are valid for this long (the PvP NP hooks run
/// right after `bot_on_death` in the same handler).
const VERDICT_TTL_MS: u64 = 2_000;
/// Fallback potion amounts when the magic table has no potion rows (tests).
const FALLBACK_HP_POTION: i16 = 360;
const FALLBACK_MP_POTION: i16 = 300;

// ── Configuration ────────────────────────────────────────────────────────

/// Admin knobs (`bot_brain_config`, panel page "Bot Zekâsı").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrainConfig {
    /// Master switch; off = legacy `tick_fighting` behaviour.
    pub enabled: bool,
    /// Share of PK-zone bots that patrol the field (the rest gather at hotspots).
    pub patrol_percent: u8,
    /// Human-like reaction delay before the first hit on a new target (ms).
    pub reaction_min_ms: u16,
    pub reaction_max_ms: u16,
    /// Chance (%) to pick the second-best target instead of the best.
    pub target_mistake_percent: u8,
    /// Chance (%) that a ranged class fails to kite this tick.
    pub kite_miss_percent: u8,
    /// Virtual stock per bot life.
    pub hp_potions: u16,
    pub mp_potions: u16,
    pub ts_scrolls: u16,
    pub buff_scrolls: u16,
    /// Potion/scroll cooldown (players: 2400 ms).
    pub potion_cooldown_ms: u16,
    /// NP for a PK-zone bot kill as a percentage of the zone's player-kill NP.
    pub np_percent: u8,
    /// The same killer earns nothing for the same bot again within this window (s).
    pub antifarm_secs: u16,
    /// Bots killed this soon after a respawn pay nothing (s).
    pub spawn_grace_secs: u16,
    /// Bot party composition.
    pub party_max_bots: u8,
    pub party_max_per_class: u8,
    /// Own HP (%) below which a bot retreats / heals up.
    pub retreat_hp_percent: u8,
    /// Retreat when `enemies * 100 >= allies * this` (200 = twice as many).
    pub outnumbered_percent: u16,
    /// Natural party spacing range (m).
    pub spacing_min: u8,
    pub spacing_max: u8,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            patrol_percent: 40,
            reaction_min_ms: 300,
            reaction_max_ms: 900,
            target_mistake_percent: 10,
            kite_miss_percent: 25,
            hp_potions: 30,
            mp_potions: 30,
            ts_scrolls: 2,
            buff_scrolls: 4,
            potion_cooldown_ms: 2_400,
            np_percent: 33,
            antifarm_secs: 300,
            spawn_grace_secs: 20,
            party_max_bots: 6,
            party_max_per_class: 2,
            retreat_hp_percent: 35,
            outnumbered_percent: 200,
            spacing_min: 3,
            spacing_max: 8,
        }
    }
}

impl BrainConfig {
    /// Clamp every knob into a sane range.
    pub fn sanitized(mut self) -> Self {
        self.patrol_percent = self.patrol_percent.min(100);
        self.reaction_min_ms = self.reaction_min_ms.min(5_000);
        self.reaction_max_ms = self.reaction_max_ms.clamp(self.reaction_min_ms, 5_000);
        self.target_mistake_percent = self.target_mistake_percent.min(100);
        self.kite_miss_percent = self.kite_miss_percent.min(100);
        self.potion_cooldown_ms = self.potion_cooldown_ms.max(500);
        self.np_percent = self.np_percent.min(100);
        self.party_max_bots = self.party_max_bots.clamp(2, 8);
        self.party_max_per_class = self.party_max_per_class.clamp(1, 8);
        self.retreat_hp_percent = self.retreat_hp_percent.min(90);
        self.outnumbered_percent = self.outnumbered_percent.max(100);
        self.spacing_min = self.spacing_min.clamp(1, 20);
        self.spacing_max = self.spacing_max.clamp(self.spacing_min, 30);
        self
    }
}

static CONFIG: OnceLock<RwLock<BrainConfig>> = OnceLock::new();

/// Current configuration (defaults until the table is loaded).
pub fn config() -> BrainConfig {
    CONFIG.get_or_init(|| RwLock::new(BrainConfig::default())).read().clone()
}

/// Replace the configuration (tests and `reload_config`).
pub fn set_config(cfg: BrainConfig) {
    *CONFIG.get_or_init(|| RwLock::new(BrainConfig::default())).write() = cfg.sanitized();
}

/// Load the single `bot_brain_config` row. A missing table keeps the defaults.
pub async fn reload_config(pool: &ko_db::DbPool) -> anyhow::Result<usize> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT enabled, patrol_percent, reaction_min_ms, reaction_max_ms, target_mistake_percent, \
         kite_miss_percent, hp_potions, mp_potions, ts_scrolls, buff_scrolls, potion_cooldown_ms, \
         np_percent, antifarm_secs, spawn_grace_secs, party_max_bots, party_max_per_class, \
         retreat_hp_percent, outnumbered_percent, spacing_min, spacing_max \
         FROM bot_brain_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        set_config(BrainConfig::default());
        return Ok(0);
    };
    let i16c = |col: &str| -> anyhow::Result<i16> { Ok(row.try_get::<i16, _>(col)?) };
    let cfg = BrainConfig {
        enabled: row.try_get("enabled")?,
        patrol_percent: i16c("patrol_percent")?.clamp(0, 100) as u8,
        reaction_min_ms: i16c("reaction_min_ms")?.max(0) as u16,
        reaction_max_ms: i16c("reaction_max_ms")?.max(0) as u16,
        target_mistake_percent: i16c("target_mistake_percent")?.clamp(0, 100) as u8,
        kite_miss_percent: i16c("kite_miss_percent")?.clamp(0, 100) as u8,
        hp_potions: i16c("hp_potions")?.max(0) as u16,
        mp_potions: i16c("mp_potions")?.max(0) as u16,
        ts_scrolls: i16c("ts_scrolls")?.max(0) as u16,
        buff_scrolls: i16c("buff_scrolls")?.max(0) as u16,
        potion_cooldown_ms: i16c("potion_cooldown_ms")?.max(0) as u16,
        np_percent: i16c("np_percent")?.clamp(0, 100) as u8,
        antifarm_secs: i16c("antifarm_secs")?.max(0) as u16,
        spawn_grace_secs: i16c("spawn_grace_secs")?.max(0) as u16,
        party_max_bots: i16c("party_max_bots")?.clamp(0, 8) as u8,
        party_max_per_class: i16c("party_max_per_class")?.clamp(0, 8) as u8,
        retreat_hp_percent: i16c("retreat_hp_percent")?.clamp(0, 100) as u8,
        outnumbered_percent: i16c("outnumbered_percent")?.max(0) as u16,
        spacing_min: i16c("spacing_min")?.clamp(0, 30) as u8,
        spacing_max: i16c("spacing_max")?.clamp(0, 30) as u8,
    };
    set_config(cfg);
    Ok(1)
}

// ── Roles / modes ────────────────────────────────────────────────────────

/// Combat role derived from the class (`bot_class_group`: 1 warrior, 2 rogue,
/// 3 mage, 4 priest; kurian counts as a warrior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassRole {
    Warrior,
    Rogue,
    Mage,
    Priest,
}

impl ClassRole {
    pub fn of(bot: &BotInstance) -> Self {
        Self::of_class(bot.class)
    }

    pub fn of_class(class: u16) -> Self {
        match class % 100 {
            2 | 7 | 8 => ClassRole::Rogue,
            3 | 9 | 10 => ClassRole::Mage,
            4 | 11 | 12 => ClassRole::Priest,
            _ => ClassRole::Warrior,
        }
    }

    pub fn is_ranged(self) -> bool {
        matches!(self, ClassRole::Mage | ClassRole::Priest)
    }

    /// Group index used by the party composition rules (`bot_class_group`).
    pub fn group(self) -> usize {
        match self {
            ClassRole::Warrior => 1,
            ClassRole::Rogue => 2,
            ClassRole::Mage => 3,
            ClassRole::Priest => 4,
        }
    }
}

/// What a PK bot does when no fight is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Walk the surveyed field routes and harass whoever grinds there.
    Patrol,
    /// Gather at a known fight spot (bowl, bridges, base fronts).
    Hotspot,
}

/// Decision of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No enemy: objective layer (hotspot / patrol / regroup).
    Objective,
    /// Fight the chosen target.
    Engage,
    /// Ranged class backs off from a melee enemy.
    Kite,
    /// Low HP or outnumbered: run to friends / away.
    Retreat,
    /// Out of danger but hurt: potions / self-heal until recovered.
    HealUp,
    /// Walk back to the party.
    Regroup,
}

// ── Per-bot state ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BrainState {
    pub role: Role,
    pub mode: Mode,
    pub target: Option<u32>,
    pub reaction_until_ms: u64,
    pub target_until_ms: u64,
    pub cast_lock_until_ms: u64,
    pub hp_potions: u16,
    pub mp_potions: u16,
    pub ts_scrolls: u16,
    pub buff_scrolls: u16,
    pub next_potion_ms: u64,
    pub ts_until_ms: u64,
    pub buff_until_ms: u64,
    pub spawned_ms: u64,
    pub spacing: f32,
    pub formation_angle: f32,
    pub reshuffle_at_ms: u64,
    pub hotspot: Option<(f32, f32)>,
    pub hotspot_until_ms: u64,
    pub wander_goal: Option<(f32, f32)>,
    pub wander_at_ms: u64,
    pub last_hp: i16,
    pub last_attacked_ms: u64,
    pub last_attacker: Option<u32>,
    pub mode_until_ms: u64,
}

impl BrainState {
    pub fn new(cfg: &BrainConfig, now_ms: u64, rng: &mut impl Rng) -> Self {
        let role = if rng.gen_range(0..100u32) < cfg.patrol_percent as u32 {
            Role::Patrol
        } else {
            Role::Hotspot
        };
        let mut st = Self {
            role,
            mode: Mode::Objective,
            target: None,
            reaction_until_ms: 0,
            target_until_ms: 0,
            cast_lock_until_ms: 0,
            hp_potions: cfg.hp_potions,
            mp_potions: cfg.mp_potions,
            ts_scrolls: cfg.ts_scrolls,
            buff_scrolls: cfg.buff_scrolls,
            next_potion_ms: 0,
            ts_until_ms: 0,
            buff_until_ms: 0,
            spawned_ms: now_ms,
            spacing: 0.0,
            formation_angle: 0.0,
            reshuffle_at_ms: 0,
            hotspot: None,
            hotspot_until_ms: 0,
            wander_goal: None,
            wander_at_ms: 0,
            last_hp: i16::MAX,
            last_attacked_ms: 0,
            last_attacker: None,
            mode_until_ms: 0,
        };
        st.reshuffle(cfg, now_ms, rng);
        st
    }

    /// New spacing / formation side, due again in 12-28 s.
    pub fn reshuffle(&mut self, cfg: &BrainConfig, now_ms: u64, rng: &mut impl Rng) {
        let (lo, hi) = (cfg.spacing_min as f32, cfg.spacing_max as f32);
        self.spacing = if hi > lo { rng.gen_range(lo..=hi) } else { lo };
        self.formation_angle = rng.gen_range(-std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2);
        self.reshuffle_at_ms = now_ms + rng.gen_range(RESHUFFLE_MIN_MS..=RESHUFFLE_MAX_MS);
    }

    /// Refill the virtual stock (a new life).
    pub fn refill(&mut self, cfg: &BrainConfig, now_ms: u64) {
        self.hp_potions = cfg.hp_potions;
        self.mp_potions = cfg.mp_potions;
        self.ts_scrolls = cfg.ts_scrolls;
        self.buff_scrolls = cfg.buff_scrolls;
        self.spawned_ms = now_ms;
        self.target = None;
        self.mode = Mode::Objective;
        self.mode_until_ms = 0;
        self.cast_lock_until_ms = 0;
        self.ts_until_ms = 0;
        self.buff_until_ms = 0;
        self.last_hp = i16::MAX;
        self.last_attacked_ms = 0;
        self.last_attacker = None;
        self.hotspot = None;
        self.wander_goal = None;
    }
}

static STATES: OnceLock<Mutex<HashMap<BotId, BrainState>>> = OnceLock::new();

fn states() -> &'static Mutex<HashMap<BotId, BrainState>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Snapshot of a bot's brain state (creates it on first use).
pub fn state_of(bot_id: BotId, now_ms: u64) -> BrainState {
    let cfg = config();
    let mut map = states().lock();
    map.entry(bot_id)
        .or_insert_with(|| BrainState::new(&cfg, now_ms, &mut rand::thread_rng()))
        .clone()
}

fn store_state(bot_id: BotId, st: BrainState) {
    states().lock().insert(bot_id, st);
}

/// Forget a despawned bot.
pub fn forget(bot_id: BotId) {
    states().lock().remove(&bot_id);
}

/// A bot came back to life: refill its stock and start the spawn grace.
pub fn on_bot_respawn(bot_id: BotId, now_ms: u64) {
    let cfg = config();
    let mut map = states().lock();
    let st = map
        .entry(bot_id)
        .or_insert_with(|| BrainState::new(&cfg, now_ms, &mut rand::thread_rng()));
    st.refill(&cfg, now_ms);
}

// ── Zone snapshot ────────────────────────────────────────────────────────

/// One living unit (player or bot) of a zone, as seen by the brain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub id: u32,
    pub is_bot: bool,
    pub nation: u8,
    pub x: f32,
    pub z: f32,
    pub hp: i16,
    pub max_hp: i16,
    pub role: ClassRole,
    pub level: u8,
    /// Who this unit is attacking (bots only; players: `None`).
    pub targeting: Option<u32>,
}

impl Unit {
    pub fn hp_ratio(&self) -> f32 {
        if self.max_hp <= 0 {
            1.0
        } else {
            (self.hp.max(0) as f32 / self.max_hp as f32).min(1.0)
        }
    }

    pub fn dist(&self, x: f32, z: f32) -> f32 {
        ((self.x - x).powi(2) + (self.z - z).powi(2)).sqrt()
    }
}

const GRID_CELL: f32 = 16.0;

/// All units of a zone at one tick, bucketed on a coarse grid.
pub struct Snapshot {
    pub zone_id: u16,
    pub built_ms: u64,
    pub units: Vec<Unit>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl Snapshot {
    pub fn from_units(zone_id: u16, built_ms: u64, units: Vec<Unit>) -> Self {
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, u) in units.iter().enumerate() {
            grid.entry(cell(u.x, u.z)).or_default().push(i);
        }
        Self { zone_id, built_ms, units, grid }
    }

    /// Units within `radius` of `(x, z)`, excluding `exclude_id`.
    pub fn near(&self, x: f32, z: f32, radius: f32, exclude_id: u32) -> Vec<Unit> {
        let (cx, cz) = cell(x, z);
        let span = (radius / GRID_CELL).ceil() as i32 + 1;
        let r2 = radius * radius;
        let mut out = Vec::new();
        for gx in (cx - span)..=(cx + span) {
            for gz in (cz - span)..=(cz + span) {
                if let Some(idx) = self.grid.get(&(gx, gz)) {
                    for &i in idx {
                        let u = &self.units[i];
                        if u.id == exclude_id {
                            continue;
                        }
                        let dx = u.x - x;
                        let dz = u.z - z;
                        if dx * dx + dz * dz <= r2 {
                            out.push(*u);
                        }
                    }
                }
            }
        }
        out
    }

    pub fn get(&self, id: u32) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }
}

fn cell(x: f32, z: f32) -> (i32, i32) {
    ((x / GRID_CELL).floor() as i32, (z / GRID_CELL).floor() as i32)
}

/// Keyed by (world, zone): tests run several worlds in one process.
static SNAPSHOTS: OnceLock<Mutex<HashMap<(usize, u16), Arc<Snapshot>>>> = OnceLock::new();

/// The zone snapshot of this tick (built once per zone per `now_ms`).
pub fn snapshot(world: &WorldState, zone_id: u16, now_ms: u64) -> Arc<Snapshot> {
    let key = (world as *const WorldState as usize, zone_id);
    let cache = SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(s) = cache.lock().get(&key) {
        if s.built_ms == now_ms {
            return Arc::clone(s);
        }
    }
    let snap = Arc::new(build_snapshot(world, zone_id, now_ms));
    let mut cache = cache.lock();
    if cache.len() > 256 {
        cache.retain(|_, s| s.built_ms + 60_000 > now_ms);
    }
    cache.insert(key, Arc::clone(&snap));
    snap
}

fn build_snapshot(world: &WorldState, zone_id: u16, now_ms: u64) -> Snapshot {
    let mut units = Vec::new();
    for sid in world.sessions_in_zone(zone_id) {
        let (Some(ch), Some(pos)) = (world.get_character_info(sid), world.get_position(sid)) else {
            continue;
        };
        if ch.hp <= 0 || pos.zone_id != zone_id {
            continue;
        }
        units.push(Unit {
            id: sid as u32,
            is_bot: false,
            nation: ch.nation,
            x: pos.x,
            z: pos.z,
            hp: ch.hp,
            max_hp: ch.max_hp,
            role: ClassRole::of_class(ch.class),
            level: ch.level,
            targeting: None,
        });
    }
    for entry in world.bots.iter() {
        let b = entry.value();
        if b.zone_id != zone_id || !b.is_alive() {
            continue;
        }
        units.push(Unit {
            id: b.id,
            is_bot: true,
            nation: b.nation,
            x: b.x,
            z: b.z,
            hp: b.hp,
            max_hp: b.max_hp,
            role: ClassRole::of(b),
            level: b.level,
            targeting: (b.target_id >= 0).then_some(b.target_id as u32),
        });
    }
    Snapshot::from_units(zone_id, now_ms, units)
}

// ── Threat assessment ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threat {
    pub hp_ratio: f32,
    pub mp_ratio: f32,
    /// Enemies within [`THREAT_RADIUS`].
    pub enemies: usize,
    /// Allies within [`THREAT_RADIUS`], the bot itself included.
    pub allies: usize,
    /// Somebody damaged the bot within [`ATTACKED_MEMORY_MS`].
    pub under_attack: bool,
    /// Distance to the nearest enemy (`f32::MAX` when none).
    pub nearest_enemy: f32,
    /// Lowest HP ratio among nearby allies (1.0 when alone).
    pub ally_low_ratio: f32,
}

/// Pure threat assessment from the bot's vitals and its surroundings
/// (nation based: everybody of another nation is an enemy).
pub fn assess(
    hp: i16,
    max_hp: i16,
    mp: i16,
    max_mp: i16,
    nation: u8,
    x: f32,
    z: f32,
    near: &[Unit],
    under_attack: bool,
) -> Threat {
    let enemies: Vec<Unit> = near.iter().filter(|u| u.nation != nation).copied().collect();
    let allies: Vec<Unit> = near.iter().filter(|u| u.nation == nation).copied().collect();
    assess_units(hp, max_hp, mp, max_mp, x, z, &enemies, &allies, under_attack)
}

/// Threat assessment over explicit enemy / ally lists (the Chaos free-for-all
/// counts only party mates as allies, so nation equality is not the rule there).
pub fn assess_units(
    hp: i16,
    max_hp: i16,
    mp: i16,
    max_mp: i16,
    x: f32,
    z: f32,
    enemies: &[Unit],
    allies: &[Unit],
    under_attack: bool,
) -> Threat {
    let ratio = |v: i16, m: i16| if m <= 0 { 1.0 } else { (v.max(0) as f32 / m as f32).min(1.0) };
    let nearest_enemy = enemies.iter().map(|e| e.dist(x, z)).fold(f32::MAX, f32::min);
    let ally_low_ratio = allies.iter().map(|a| a.hp_ratio()).fold(1.0f32, f32::min);
    Threat {
        hp_ratio: ratio(hp, max_hp),
        mp_ratio: ratio(mp, max_mp),
        enemies: enemies.len(),
        allies: allies.len() + 1,
        under_attack,
        nearest_enemy,
        ally_low_ratio,
    }
}

/// Pure decision: which mode should the bot be in this tick?
///
/// * low HP with enemies around -> `Retreat`; low HP and quiet -> `HealUp`
/// * outnumbered (`enemies*100 >= allies*outnumbered_percent`, at least two
///   enemies) -> `Retreat`
/// * a target in a ranged class with a melee enemy at its feet -> `Kite`
///   (unless the imperfect-kiting roll fails)
/// * a target -> `Engage`; otherwise `Regroup` (party far) or `Objective`
pub fn decide(
    threat: &Threat,
    cfg: &BrainConfig,
    role: ClassRole,
    has_target: bool,
    party_far: bool,
    kite_roll: u32,
    previous: Mode,
) -> Mode {
    let low_hp = threat.hp_ratio < cfg.retreat_hp_percent as f32 / 100.0;
    // Hysteresis: a bot that is healing up keeps doing so until recovered.
    let still_hurt = matches!(previous, Mode::HealUp | Mode::Retreat) && threat.hp_ratio < RECOVERED_HP_RATIO;
    if (low_hp || still_hurt) && threat.enemies > 0 {
        return Mode::Retreat;
    }
    if low_hp || still_hurt {
        return Mode::HealUp;
    }
    if threat.enemies >= 2 && threat.enemies * 100 >= threat.allies * cfg.outnumbered_percent as usize {
        return Mode::Retreat;
    }
    if has_target {
        if role.is_ranged()
            && threat.nearest_enemy <= KITE_TRIGGER
            && kite_roll >= cfg.kite_miss_percent as u32
        {
            return Mode::Kite;
        }
        return Mode::Engage;
    }
    if party_far {
        return Mode::Regroup;
    }
    Mode::Objective
}

// ── Target selection ─────────────────────────────────────────────────────

/// Utility score of `t` as a target for `me`. Higher is better.
///
/// * low HP: up to +60
/// * priests +25, mages +10
/// * isolated: +30 when no ally of the target stands within 12 m, -8 per ally
/// * attacking me: +25
/// * current target (stickiness): +20
/// * distance: -1.5 per unit
pub fn score_target(me_id: u32, t: &Unit, allies_near_target: usize, current: Option<u32>, dist: f32) -> f32 {
    let mut s = 100.0;
    s += (1.0 - t.hp_ratio()) * 60.0;
    s += match t.role {
        ClassRole::Priest => 25.0,
        ClassRole::Mage => 10.0,
        _ => 0.0,
    };
    s += if allies_near_target == 0 { 30.0 } else { -8.0 * allies_near_target as f32 };
    if t.targeting == Some(me_id) {
        s += 25.0;
    }
    if current == Some(t.id) {
        s += 20.0;
    }
    s -= dist * 1.5;
    s
}

/// Simplified score for callers that only know distance/HP/class (event bots).
pub fn target_score_basic(dist: f32, hp: i16, max_hp: i16, class: u16) -> f32 {
    let u = Unit {
        id: 0,
        is_bot: false,
        nation: 0,
        x: 0.0,
        z: 0.0,
        hp,
        max_hp,
        role: ClassRole::of_class(class),
        level: 0,
        targeting: None,
    };
    score_target(u32::MAX, &u, 1, None, dist)
}

/// Choose a target among `enemies` (all already validated as attackable).
/// `all_near` is used to count the target's own allies (isolation).
/// `mistake_roll` in `0..100`: below `cfg.target_mistake_percent` the second
/// best candidate is taken -- an average player does not always pick right.
pub fn pick_target(
    me_id: u32,
    me_x: f32,
    me_z: f32,
    enemies: &[Unit],
    all_near: &[Unit],
    current: Option<u32>,
    cfg: &BrainConfig,
    mistake_roll: u32,
) -> Option<Unit> {
    pick_target_with(me_id, me_x, me_z, enemies, all_near, current, cfg, mistake_roll, &|_| 0.0)
}

/// [`pick_target`] with an objective bonus per candidate (e.g. the enemy altar
/// carrier in BDW, somebody hitting a party mate).
#[allow(clippy::too_many_arguments)]
pub fn pick_target_with(
    me_id: u32,
    me_x: f32,
    me_z: f32,
    enemies: &[Unit],
    all_near: &[Unit],
    current: Option<u32>,
    cfg: &BrainConfig,
    mistake_roll: u32,
    bonus: &dyn Fn(&Unit) -> f32,
) -> Option<Unit> {
    if enemies.is_empty() {
        return None;
    }
    let mut scored: Vec<(f32, Unit)> = enemies
        .iter()
        .map(|e| {
            let allies = all_near
                .iter()
                .filter(|o| o.id != e.id && o.nation == e.nation && o.dist(e.x, e.z) <= 12.0)
                .count();
            (score_target(me_id, e, allies, current, e.dist(me_x, me_z)) + bonus(e), *e)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let idx = if scored.len() > 1 && mistake_roll < cfg.target_mistake_percent as u32 { 1 } else { 0 };
    Some(scored[idx].1)
}

// ── Formation ────────────────────────────────────────────────────────────

/// Where a bot of `role` wants to stand relative to its target:
/// warriors go straight in, rogues flank (side chosen by `angle`'s sign) at
/// `spacing`, mages/priests stop at `range - 2` (at least [`RANGED_KEEP_MIN`]).
pub fn formation_point(
    role: ClassRole,
    bot: (f32, f32),
    target: (f32, f32),
    spacing: f32,
    angle: f32,
    skill_range: f32,
) -> (f32, f32) {
    let dx = target.0 - bot.0;
    let dz = target.1 - bot.1;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist < 0.01 {
        return target;
    }
    let (ux, uz) = (dx / dist, dz / dist);
    match role {
        ClassRole::Warrior => target,
        ClassRole::Rogue => {
            let side = if angle >= 0.0 { 1.0 } else { -1.0 };
            // perpendicular to the approach vector
            (target.0 - uz * spacing * side, target.1 + ux * spacing * side)
        }
        ClassRole::Mage | ClassRole::Priest => {
            let keep = (skill_range - 2.0).max(RANGED_KEEP_MIN);
            if dist <= keep {
                return bot;
            }
            (target.0 - ux * keep, target.1 - uz * keep)
        }
    }
}

/// Party anti-clump: push `goal` away from the nearest party mate closer than
/// `spacing`. Returns the nudged goal (or the original when nobody is close).
pub fn spread_from_mates(bot: (f32, f32), goal: (f32, f32), mates: &[(f32, f32)], spacing: f32) -> (f32, f32) {
    let mut nearest: Option<(f32, f32, f32)> = None;
    for &(mx, mz) in mates {
        let dx = bot.0 - mx;
        let dz = bot.1 - mz;
        let d2 = dx * dx + dz * dz;
        if d2 < spacing * spacing && nearest.is_none_or(|(nd, _, _)| d2 < nd) {
            let d = d2.sqrt();
            let (nx, nz) = if d > 0.01 { (dx / d, dz / d) } else { (1.0, 0.0) };
            nearest = Some((d2, nx, nz));
        }
    }
    match nearest {
        Some((_, nx, nz)) => (goal.0 + nx * spacing, goal.1 + nz * spacing),
        None => goal,
    }
}

/// Spacing a bot keeps from its party mates (per member, reshuffled).
pub fn spacing_of(bot_id: BotId, now_ms: u64) -> f32 {
    let cfg = config();
    let mut map = states().lock();
    let st = map
        .entry(bot_id)
        .or_insert_with(|| BrainState::new(&cfg, now_ms, &mut rand::thread_rng()));
    if now_ms >= st.reshuffle_at_ms {
        st.reshuffle(&cfg, now_ms, &mut rand::thread_rng());
    }
    st.spacing
}

// ── Party composition rules ──────────────────────────────────────────────

/// May a bot of `candidate_group` join a party that already holds `bots`
/// bots with `counts[group]` per group? Pure rule: at most `party_max_bots`
/// bots and `party_max_per_class` of one class, never a role-less bot.
pub fn party_accepts(cfg: &BrainConfig, bots: usize, counts: &[usize; 5], candidate_group: usize) -> bool {
    candidate_group != 0
        && candidate_group < counts.len()
        && bots < cfg.party_max_bots as usize
        && counts[candidate_group] < cfg.party_max_per_class as usize
}

/// Sort key for choosing the next bot to add to a party: a priest first when
/// the party has none, then the least represented role, then the id.
pub fn recruit_key(counts: &[usize; 5], candidate_group: usize, id: u32) -> (usize, usize, u32) {
    let priest_first = if candidate_group == 4 && counts[4] == 0 { 0 } else { 1 };
    (priest_first, counts.get(candidate_group).copied().unwrap_or(usize::MAX), id)
}

// ── NP policy ────────────────────────────────────────────────────────────

static ANTIFARM: OnceLock<Mutex<HashMap<(SessionId, BotId), u64>>> = OnceLock::new();
static VERDICTS: OnceLock<Mutex<HashMap<SessionId, (bool, u64)>>> = OnceLock::new();

fn antifarm() -> &'static Mutex<HashMap<(SessionId, BotId), u64>> {
    ANTIFARM.get_or_init(|| Mutex::new(HashMap::new()))
}

fn verdicts() -> &'static Mutex<HashMap<SessionId, (bool, u64)>> {
    VERDICTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Pure anti-farm rule. `last_until` is the stored expiry for (killer, bot).
pub fn kill_pays(cfg: &BrainConfig, spawned_ms: u64, last_until: Option<u64>, now_ms: u64) -> bool {
    if now_ms < spawned_ms.saturating_add(cfg.spawn_grace_secs as u64 * 1000) {
        return false;
    }
    !last_until.is_some_and(|until| now_ms < until)
}

/// Scale a player-kill NP amount to the bot share.
pub fn scale_np(cfg: &BrainConfig, np: i32) -> i32 {
    if np <= 0 {
        return 0;
    }
    (np * cfg.np_percent as i32 / 100).max(if cfg.np_percent > 0 { 1 } else { 0 })
}

/// NP a player earns for killing PK bot `bot_id` when the zone's player-kill
/// NP would be `base_np`. Records the anti-farm window and the kill verdict
/// (used by `pvp_loyalty_on_bot_kill`). Returns 0 for a farmed / fresh bot.
pub fn bot_kill_np(killer: SessionId, bot_id: BotId, base_np: i32, now_ms: u64) -> i32 {
    let cfg = config();
    if !cfg.enabled {
        return base_np;
    }
    let spawned = states().lock().get(&bot_id).map(|s| s.spawned_ms).unwrap_or(0);
    let mut farm = antifarm().lock();
    if farm.len() > 8192 {
        farm.retain(|_, until| *until > now_ms);
    }
    let pays = kill_pays(&cfg, spawned, farm.get(&(killer, bot_id)).copied(), now_ms);
    if pays {
        farm.insert((killer, bot_id), now_ms + cfg.antifarm_secs as u64 * 1000);
    }
    drop(farm);
    verdicts().lock().insert(killer, (pays, now_ms));
    if pays {
        scale_np(&cfg, base_np)
    } else {
        0
    }
}

/// Did the killer's most recent bot kill (within [`VERDICT_TTL_MS`]) pay?
/// Unknown kills pay (legacy behaviour).
pub fn kill_rewardable(killer: SessionId, now_ms: u64) -> bool {
    verdicts()
        .lock()
        .get(&killer)
        .is_none_or(|(pays, at)| *pays || now_ms.saturating_sub(*at) > VERDICT_TTL_MS)
}

// ── Reaction delay (shared with event bots) ──────────────────────────────

/// `true` when the bot may hit `target_id` now. A new target starts a human
/// reaction delay; the same target stays ready.
pub fn reaction_ready(bot_id: BotId, target_id: u32, now_ms: u64) -> bool {
    let cfg = config();
    if !cfg.enabled {
        return true;
    }
    let mut map = states().lock();
    let st = map
        .entry(bot_id)
        .or_insert_with(|| BrainState::new(&cfg, now_ms, &mut rand::thread_rng()));
    if st.target != Some(target_id) {
        st.target = Some(target_id);
        st.reaction_until_ms = now_ms + reaction_delay(&cfg, &mut rand::thread_rng());
    }
    now_ms >= st.reaction_until_ms
}

fn reaction_delay(cfg: &BrainConfig, rng: &mut impl Rng) -> u64 {
    let (lo, hi) = (cfg.reaction_min_ms as u64, cfg.reaction_max_ms as u64);
    if hi > lo { rng.gen_range(lo..=hi) } else { lo }
}

// ── Consumables ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consumable {
    HpPotion,
    MpPotion,
    Transform,
    BuffScroll,
}

/// Pure stock/cooldown rule: which consumable (if any) a bot with these
/// vitals uses now. HP first (< 60 %), then MP for casters (< 25 %), then a
/// transformation scroll / buff scroll when idle enough (no enemies close).
pub fn choose_consumable(
    st: &BrainState,
    hp_ratio: f32,
    mp_ratio: f32,
    role: ClassRole,
    enemies_near: usize,
    now_ms: u64,
) -> Option<Consumable> {
    if now_ms < st.next_potion_ms {
        return None;
    }
    if hp_ratio < 0.60 && st.hp_potions > 0 {
        return Some(Consumable::HpPotion);
    }
    if role.is_ranged() && mp_ratio < 0.25 && st.mp_potions > 0 {
        return Some(Consumable::MpPotion);
    }
    if !role.is_ranged() && mp_ratio < 0.10 && st.mp_potions > 0 {
        return Some(Consumable::MpPotion);
    }
    if enemies_near == 0 && st.buff_scrolls > 0 && now_ms >= st.buff_until_ms {
        return Some(Consumable::BuffScroll);
    }
    if enemies_near == 0 && st.ts_scrolls > 0 && now_ms >= st.ts_until_ms {
        return Some(Consumable::Transform);
    }
    None
}

/// Apply the stock/cooldown bookkeeping of a use. Returns the duration the
/// scroll effect lasts (0 for potions).
pub fn consume(st: &mut BrainState, what: Consumable, cfg: &BrainConfig, now_ms: u64) {
    st.next_potion_ms = now_ms + cfg.potion_cooldown_ms as u64;
    match what {
        Consumable::HpPotion => st.hp_potions = st.hp_potions.saturating_sub(1),
        Consumable::MpPotion => st.mp_potions = st.mp_potions.saturating_sub(1),
        Consumable::Transform => st.ts_scrolls = st.ts_scrolls.saturating_sub(1),
        Consumable::BuffScroll => st.buff_scrolls = st.buff_scrolls.saturating_sub(1),
    }
}

/// `(skill id, direct type 1=HP 2=MP, amount)` of every item potion in the magic table.
#[derive(Debug, Clone, Copy)]
pub struct PotionSkill {
    pub skill_id: i32,
    pub direct_type: i32,
    pub amount: i32,
}

/// `(skill id, duration s)` of the self buff scrolls / transformation scrolls.
#[derive(Debug, Clone, Copy)]
pub struct ScrollSkill {
    pub skill_id: i32,
    pub duration: i32,
}

#[derive(Default, Clone)]
struct ConsumableCatalog {
    potions: Vec<PotionSkill>,
    buffs: Vec<ScrollSkill>,
    transforms: Vec<ScrollSkill>,
}

static CATALOG: OnceLock<Mutex<Option<(usize, Arc<ConsumableCatalog>)>>> = OnceLock::new();

fn catalog(world: &WorldState) -> Arc<ConsumableCatalog> {
    let key = world as *const WorldState as usize;
    let cell = CATALOG.get_or_init(|| Mutex::new(None));
    if let Some((k, c)) = cell.lock().as_ref() {
        if *k == key {
            return Arc::clone(c);
        }
    }
    let mut cat = ConsumableCatalog::default();
    for (skill_id, direct_type, amount) in world.potion_skills() {
        cat.potions.push(PotionSkill { skill_id, direct_type, amount });
    }
    for (skill_id, duration) in world.scroll_buff_skills() {
        cat.buffs.push(ScrollSkill { skill_id, duration });
    }
    for (skill_id, duration) in world.transform_scroll_skills() {
        cat.transforms.push(ScrollSkill { skill_id, duration });
    }
    let cat = Arc::new(cat);
    *cell.lock() = Some((key, Arc::clone(&cat)));
    cat
}

/// Largest potion of `direct_type` that does not exceed `cap` (else the smallest).
fn best_potion(cat: &ConsumableCatalog, direct_type: i32, cap: i32) -> Option<PotionSkill> {
    let mut fitting: Option<PotionSkill> = None;
    let mut smallest: Option<PotionSkill> = None;
    for p in cat.potions.iter().filter(|p| p.direct_type == direct_type && p.amount > 0) {
        if p.amount <= cap && fitting.is_none_or(|f| p.amount > f.amount) {
            fitting = Some(*p);
        }
        if smallest.is_none_or(|s| p.amount < s.amount) {
            smallest = Some(*p);
        }
    }
    fitting.or(smallest)
}

fn broadcast_effect(world: &WorldState, bot: &BotInstance, opcode: u8, skill_id: u32, data: [i32; 7]) {
    let pkt = bot_ai::build_bot_magic_packet(opcode, skill_id, bot.id, bot.id, data);
    if let Some(entry) = world.event_bots.get(bot.id) {
        crate::systems::event_bots::room_broadcast(world, bot.zone_id, bot.region_x, bot.region_z, entry.room, pkt);
    } else {
        bot_ai::broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &pkt);
    }
}

/// Virtual potion / scroll use for any bot kind (PK, field, event). Returns
/// what was used. Does not block movement -- a player drinks while running.
pub fn tick_consumables_for(world: &WorldState, bot: &BotInstance, now_ms: u64) -> Option<Consumable> {
    let cfg = config();
    if !cfg.enabled || !bot.is_alive() {
        return None;
    }
    let mut st = state_of(bot.id, now_ms);
    let snap = snapshot(world, bot.zone_id, now_ms);
    let enemies_near = snap
        .near(bot.x, bot.z, THREAT_RADIUS, bot.id)
        .iter()
        .filter(|u| u.nation != bot.nation)
        .count();
    let used = use_consumable(world, bot, &mut st, &cfg, enemies_near, now_ms);
    store_state(bot.id, st);
    used
}

fn use_consumable(
    world: &WorldState,
    bot: &BotInstance,
    st: &mut BrainState,
    cfg: &BrainConfig,
    enemies_near: usize,
    now_ms: u64,
) -> Option<Consumable> {
    let hp_ratio = if bot.max_hp > 0 { bot.hp as f32 / bot.max_hp as f32 } else { 1.0 };
    let mp_ratio = if bot.max_mp > 0 { bot.mp as f32 / bot.max_mp as f32 } else { 1.0 };
    let what = choose_consumable(st, hp_ratio, mp_ratio, ClassRole::of(bot), enemies_near, now_ms)?;
    let cat = catalog(world);
    match what {
        Consumable::HpPotion | Consumable::MpPotion => {
            let (direct, cap, fallback) = if what == Consumable::HpPotion {
                (1, (bot.max_hp as i32 * 6 / 10).max(1), FALLBACK_HP_POTION)
            } else {
                (2, (bot.max_mp as i32 * 6 / 10).max(1), FALLBACK_MP_POTION)
            };
            let (skill_id, amount) = best_potion(&cat, direct, cap)
                .map(|p| (p.skill_id as u32, p.amount.clamp(1, i16::MAX as i32) as i16))
                .unwrap_or((0, fallback));
            world.update_bot(bot.id, |b| {
                if what == Consumable::HpPotion {
                    b.hp = b.hp.saturating_add(amount).min(b.max_hp);
                    b.last_hp_change_ms = now_ms;
                } else {
                    b.mp = b.mp.saturating_add(amount).min(b.max_mp);
                }
            });
            if skill_id != 0 {
                broadcast_effect(
                    world,
                    bot,
                    MAGIC_EFFECTING,
                    skill_id,
                    [bot.x as i32, bot.y as i32, bot.z as i32, amount as i32, 0, 0, 0],
                );
            }
            bot_ai::broadcast_bot_party_hp(world, bot.id);
        }
        Consumable::BuffScroll => {
            let Some(scroll) = cat.buffs.get(rand::thread_rng().gen_range(0..cat.buffs.len().max(1))).copied() else {
                // nothing in the table: drop the stock silently
                st.buff_scrolls = 0;
                return None;
            };
            st.buff_until_ms = now_ms + scroll.duration.max(10) as u64 * 1000;
            broadcast_effect(world, bot, MAGIC_CASTING, scroll.skill_id as u32, [bot.x as i32, bot.y as i32, bot.z as i32, 0, 0, 0, 0]);
            broadcast_effect(world, bot, MAGIC_EFFECTING, scroll.skill_id as u32, [0, 1, 0, scroll.duration, 0, 0, 0]);
        }
        Consumable::Transform => {
            let Some(scroll) = cat.transforms.first().copied() else {
                st.ts_scrolls = 0;
                return None;
            };
            st.ts_until_ms = now_ms + scroll.duration.max(10) as u64 * 1000;
            broadcast_effect(world, bot, MAGIC_EFFECTING, scroll.skill_id as u32, [0, 1, 0, scroll.duration, 0, 0, 0]);
        }
    }
    consume(st, what, cfg, now_ms);
    trace!(bot_id = bot.id, ?what, "bot used a consumable");
    Some(what)
}

// ── Objective plug-in ────────────────────────────────────────────────────

/// What the objective layer wants this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drive {
    /// The objective moved/acted; the brain is done for this tick.
    Handled,
    /// Nothing special: fight whatever is near, idle otherwise.
    FightNearest,
    /// Walk to this point.
    Goal(f32, f32),
    /// Walk to this point at a fraction of the normal step (the BDW altar carrier).
    GoalScaled(f32, f32, f32),
}

/// Event-specific objective layer. PK zones, Chaos, BDW and Juraid implement
/// it; the brain asks the objective only when no fight is on. `ev` is the event
/// bot registry entry (`None` for PK bots).
pub trait Objective: Send + Sync {
    fn name(&self) -> &'static str;
    fn drive(
        &self,
        world: &WorldState,
        bot: &BotInstance,
        st: &mut BrainState,
        ev: Option<&EventEntry>,
        now_ms: u64,
    ) -> Drive;
    /// Extra target score for `u` (objective priorities such as the enemy carrier).
    fn target_bonus(&self, _world: &WorldState, _bot: &BotInstance, _ev: Option<&EventEntry>, _u: &Unit) -> f32 {
        0.0
    }
    /// Does the bot fight players at all? (Juraid lanes are PvE.)
    fn fights_players(&self) -> bool {
        true
    }
    /// Dynamic obstacles `(x, z, radius)` for the bot's path queries this tick.
    fn dyn_blocks(&self, _world: &WorldState, _bot: &BotInstance, _ev: Option<&EventEntry>) -> Vec<(f32, f32, f32)> {
        Vec::new()
    }
}

/// Event bot registry entry (re-exported for the objectives).
pub type EventEntry = crate::systems::event_bots::BotEntry;

/// PK zones: hotspots for the `Hotspot` role, surveyed patrol routes for `Patrol`.
pub struct PkObjective;
/// Chaos Dungeon: free for all; hunts across the arena when nobody is near.
pub struct ChaosObjective;
/// BDW: altar / carrier / intercept / roam (`event_bots::bdw_objective`).
pub struct BdwObjective;
/// Juraid: lane chain, stage clearing, Deva (`event_bots_juraid::objective`).
pub struct JuraidObjective;

impl Objective for PkObjective {
    fn name(&self) -> &'static str {
        "pk"
    }
    fn drive(&self, world: &WorldState, bot: &BotInstance, st: &mut BrainState, _: Option<&EventEntry>, now_ms: u64) -> Drive {
        match st.role {
            Role::Patrol => {
                if bot_ai::tick_waypoint_patrol(world, bot, now_ms) {
                    Drive::Handled
                } else {
                    hotspot_drive(world, bot, st, now_ms)
                }
            }
            Role::Hotspot => hotspot_drive(world, bot, st, now_ms),
        }
    }
}

/// How long a roaming event bot keeps one roam point.
const ROAM_MIN_MS: u64 = 15_000;
const ROAM_MAX_MS: u64 = 40_000;

/// Roam the arena: random reachable points over the whole map (the bounds
/// come from the map), re-picked when reached or after a while. Keeps the bot
/// hunting instead of waiting for an enemy to walk into its 32 m search range.
pub fn roam_drive(world: &WorldState, bot: &BotInstance, st: &mut BrainState, now_ms: u64) -> Drive {
    let mut rng = rand::thread_rng();
    let reached = st.hotspot.is_some_and(|(hx, hz)| ((hx - bot.x).powi(2) + (hz - bot.z).powi(2)).sqrt() < 4.0);
    if st.hotspot.is_none() || reached || now_ms >= st.hotspot_until_ms {
        let Some(zone) = world.get_zone(bot.zone_id) else { return Drive::FightNearest };
        let Some(map) = zone.map_data.as_ref() else { return Drive::FightNearest };
        let w = map.map_width().max(32.0);
        let mut picked = None;
        for _ in 0..12 {
            let rx = rng.gen_range(8.0..(w - 8.0));
            let rz = rng.gen_range(8.0..(w - 8.0));
            if let Some(p) = crate::systems::pathfind::nearest_bot_point(map, rx, rz, 4) {
                picked = Some(p);
                break;
            }
        }
        let Some(p) = picked else { return Drive::FightNearest };
        st.hotspot = Some(p);
        st.hotspot_until_ms = now_ms + rng.gen_range(ROAM_MIN_MS..=ROAM_MAX_MS);
    }
    match st.hotspot {
        Some((x, z)) => Drive::Goal(x, z),
        None => Drive::FightNearest,
    }
}

/// Drift around `(cx, cz)` within `radius` with the hotspot wander pauses.
pub fn wander_around(bot: &BotInstance, st: &mut BrainState, cx: f32, cz: f32, radius: f32, now_ms: u64) -> Drive {
    let mut rng = rand::thread_rng();
    let d = ((cx - bot.x).powi(2) + (cz - bot.z).powi(2)).sqrt();
    if d > radius {
        st.wander_goal = None;
        return Drive::Goal(cx, cz);
    }
    if st.wander_goal.is_none_or(|(gx, gz)| ((gx - bot.x).powi(2) + (gz - bot.z).powi(2)).sqrt() < 1.5)
        || now_ms >= st.wander_at_ms
    {
        if now_ms < st.wander_at_ms && st.wander_goal.is_some() {
            return Drive::Handled;
        }
        let a = rng.gen_range(0.0..std::f32::consts::TAU);
        let r = rng.gen_range(2.0..radius.max(2.5));
        st.wander_goal = Some((cx + a.cos() * r, cz + a.sin() * r));
        st.wander_at_ms = now_ms + rng.gen_range(WANDER_MIN_MS..=WANDER_MAX_MS);
    }
    match st.wander_goal {
        Some((gx, gz)) => Drive::Goal(gx, gz),
        None => Drive::Handled,
    }
}

impl Objective for ChaosObjective {
    fn name(&self) -> &'static str {
        "chaos"
    }
    fn drive(&self, world: &WorldState, bot: &BotInstance, st: &mut BrainState, _: Option<&EventEntry>, now_ms: u64) -> Drive {
        roam_drive(world, bot, st, now_ms)
    }
}

impl Objective for BdwObjective {
    fn name(&self) -> &'static str {
        "bdw"
    }
    fn drive(&self, world: &WorldState, bot: &BotInstance, st: &mut BrainState, ev: Option<&EventEntry>, now_ms: u64) -> Drive {
        match ev {
            Some(entry) => crate::systems::event_bots::bdw_objective(world, entry, bot, st, now_ms),
            None => Drive::FightNearest,
        }
    }
    fn target_bonus(&self, world: &WorldState, bot: &BotInstance, ev: Option<&EventEntry>, u: &Unit) -> f32 {
        match ev {
            Some(entry) => crate::systems::event_bots::bdw_target_bonus(world, entry, bot, u),
            None => 0.0,
        }
    }
}

impl Objective for JuraidObjective {
    fn name(&self) -> &'static str {
        "juraid"
    }
    fn drive(&self, world: &WorldState, bot: &BotInstance, st: &mut BrainState, ev: Option<&EventEntry>, now_ms: u64) -> Drive {
        match ev {
            Some(entry) => crate::systems::event_bots_juraid::objective(world, entry, bot, st, now_ms),
            None => Drive::Handled,
        }
    }
    fn fights_players(&self) -> bool {
        false
    }
    fn dyn_blocks(&self, world: &WorldState, _bot: &BotInstance, ev: Option<&EventEntry>) -> Vec<(f32, f32, f32)> {
        match ev {
            Some(entry) => crate::systems::event_bots_juraid::closed_bridge_blocks(world, entry),
            None => Vec::new(),
        }
    }
}

/// The objective implementation for a bot.
pub fn objective_for(world: &WorldState, bot: &BotInstance) -> &'static dyn Objective {
    use crate::systems::event_bots::EventBotKind;
    static PK: PkObjective = PkObjective;
    static CHAOS: ChaosObjective = ChaosObjective;
    static BDW: BdwObjective = BdwObjective;
    static JURAID: JuraidObjective = JuraidObjective;
    match world.event_bots.get(bot.id).map(|e| e.kind) {
        Some(EventBotKind::Chaos) => &CHAOS,
        Some(EventBotKind::Bdw) => &BDW,
        Some(EventBotKind::Juraid) => &JURAID,
        _ => &PK,
    }
}

/// Known fight spots per PK zone: `(x, z, weight)`; fronts are listed per
/// nation (`nation` = the side whose base it is, 0 = shared).
struct Hotspot {
    x: f32,
    z: f32,
    weight: u32,
    base_of: u8,
}

const RONARK_HOTSPOTS: &[Hotspot] = &[
    Hotspot { x: 1050.0, z: 842.0, weight: 30, base_of: 0 },  // bowl, Karus side
    Hotspot { x: 886.0, z: 1102.0, weight: 30, base_of: 0 },  // bowl, El Morad side
    Hotspot { x: 1212.0, z: 970.0, weight: 15, base_of: 1 },  // Karus bridge
    Hotspot { x: 754.0, z: 1000.0, weight: 15, base_of: 2 },  // El Morad bridge
    Hotspot { x: 1327.0, z: 1060.0, weight: 10, base_of: 1 }, // Karus base front
    Hotspot { x: 660.0, z: 927.0, weight: 10, base_of: 2 },   // El Morad base front
];
const ARDREAM_HOTSPOTS: &[Hotspot] = &[
    Hotspot { x: 520.0, z: 516.0, weight: 40, base_of: 0 },
    Hotspot { x: 686.0, z: 326.0, weight: 15, base_of: 1 },
    Hotspot { x: 355.0, z: 706.0, weight: 15, base_of: 2 },
    Hotspot { x: 800.0, z: 200.0, weight: 10, base_of: 1 },
    Hotspot { x: 240.0, z: 840.0, weight: 10, base_of: 2 },
];
const RLB_HOTSPOTS: &[Hotspot] = &[
    Hotspot { x: 514.0, z: 510.0, weight: 40, base_of: 0 },
    Hotspot { x: 515.0, z: 307.0, weight: 15, base_of: 1 },
    Hotspot { x: 513.0, z: 713.0, weight: 15, base_of: 2 },
    Hotspot { x: 515.0, z: 180.0, weight: 10, base_of: 1 },
    Hotspot { x: 513.0, z: 840.0, weight: 10, base_of: 2 },
];

fn hotspots_for(zone_id: u16) -> &'static [Hotspot] {
    match zone_id {
        ZONE_RONARK_LAND => RONARK_HOTSPOTS,
        ZONE_ARDREAM => ARDREAM_HOTSPOTS,
        ZONE_RONARK_LAND_BASE => RLB_HOTSPOTS,
        _ => &[],
    }
}

/// Weighted hotspot pick; enemy fronts weigh double (harass), own fronts half.
pub fn pick_hotspot(zone_id: u16, nation: u8, roll: u32) -> Option<(f32, f32)> {
    let spots = hotspots_for(zone_id);
    if spots.is_empty() {
        return None;
    }
    let w = |h: &Hotspot| -> u32 {
        if h.base_of == 0 {
            h.weight
        } else if h.base_of == nation {
            h.weight / 2
        } else {
            h.weight * 2
        }
    };
    let total: u32 = spots.iter().map(w).sum();
    let mut r = roll % total.max(1);
    for h in spots {
        let hw = w(h);
        if r < hw {
            return Some((h.x, h.z));
        }
        r -= hw;
    }
    spots.first().map(|h| (h.x, h.z))
}

fn hotspot_drive(world: &WorldState, bot: &BotInstance, st: &mut BrainState, now_ms: u64) -> Drive {
    let mut rng = rand::thread_rng();
    if st.hotspot.is_none() || now_ms >= st.hotspot_until_ms {
        let Some((hx, hz)) = pick_hotspot(bot.zone_id, bot.nation, rng.gen()) else {
            return Drive::FightNearest;
        };
        // project onto walkable ground; an unreachable spot is skipped next time
        let spot = world.get_zone(bot.zone_id).and_then(|zone| {
            zone.map_data
                .as_ref()
                .and_then(|map| crate::systems::pathfind::nearest_bot_point(map, hx, hz, 48))
        });
        st.hotspot = Some(spot.unwrap_or((hx, hz)));
        st.hotspot_until_ms = now_ms + rng.gen_range(HOTSPOT_MIN_MS..=HOTSPOT_MAX_MS);
        st.wander_goal = None;
    }
    let (hx, hz) = st.hotspot.unwrap_or((bot.x, bot.z));
    let d = ((hx - bot.x).powi(2) + (hz - bot.z).powi(2)).sqrt();
    if d > HOTSPOT_RADIUS {
        return Drive::Goal(hx, hz);
    }
    // At the spot: never stand still in the open -- drift around it.
    if st.wander_goal.is_none_or(|(gx, gz)| ((gx - bot.x).powi(2) + (gz - bot.z).powi(2)).sqrt() < 1.5)
        || now_ms >= st.wander_at_ms
    {
        if now_ms < st.wander_at_ms && st.wander_goal.is_some() {
            return Drive::Handled; // pausing between two wanders
        }
        let a = rng.gen_range(0.0..std::f32::consts::TAU);
        let r = rng.gen_range(2.0..HOTSPOT_RADIUS);
        st.wander_goal = Some((hx + a.cos() * r, hz + a.sin() * r));
        st.wander_at_ms = now_ms + rng.gen_range(WANDER_MIN_MS..=WANDER_MAX_MS);
    }
    match st.wander_goal {
        Some((gx, gz)) => Drive::Goal(gx, gz),
        None => Drive::Handled,
    }
}

// ── PK tick ──────────────────────────────────────────────────────────────

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A player the bot may attack (same rules as `find_nearest_enemy`).
fn valid_player_enemy(world: &WorldState, bot: &BotInstance, sid: SessionId, now_unix: u64) -> bool {
    let Some(ch) = world.get_character_info(sid) else { return false };
    let Some(pos) = world.get_position(sid) else { return false };
    if ch.hp <= 0 || ch.nation == bot.nation || ch.authority == 0 {
        return false;
    }
    if !matches!(pos.zone_id, ZONE_RONARK_LAND..=ZONE_RONARK_LAND_BASE) {
        return false;
    }
    if world.with_session(sid, |h| h.genie_active).unwrap_or(false) {
        return false;
    }
    !world.is_player_blinking(sid, now_unix)
}

fn valid_bot_enemy(world: &WorldState, bot: &BotInstance, id: BotId) -> bool {
    world
        .get_bot(id)
        .is_some_and(|o| o.is_alive() && o.nation != bot.nation && o.zone_id == bot.zone_id && o.is_in_pk_zone())
}

/// Event-room rules: same room, hostile by event kind (Chaos is free for all),
/// no GMs, no blinking players. The fighting phase is checked by the driver.
fn valid_event_enemy(world: &WorldState, entry: &EventEntry, bot: &BotInstance, u: &Unit) -> bool {
    use crate::systems::event_bots;
    if u.is_bot {
        return event_bots::resolve_target(world, entry, bot, u.id).is_some();
    }
    let sid = u.id as SessionId;
    match (world.get_character_info(sid), world.get_position(sid)) {
        (Some(ch), Some(pos)) => event_bots::human_attackable(world, entry, sid, &ch, pos.zone_id, bot.zone_id),
        _ => false,
    }
}

/// One pathed step toward `(gx, gz)`; broadcasts and commits the move. `scale`
/// shortens the step (1.0 = normal). Event bots broadcast to their room only.
fn step_to_ex(
    world: &WorldState,
    bot: &BotInstance,
    ev: Option<&EventEntry>,
    gx: f32,
    gz: f32,
    echo: u8,
    now_ms: u64,
    scale: f32,
) -> bool {
    let Some((mut nx, ny, mut nz)) = bot_ai::bot_path_step_toward(world, bot, gx, bot.y, gz)
        .or_else(|| ev.and_then(|_| crate::systems::event_bots::no_map_step(world, bot, gx, gz)))
    else {
        return false;
    };
    if scale < 1.0 {
        // Shorter step along the same validated segment.
        nx = bot.x + (nx - bot.x) * scale.max(0.05);
        nz = bot.z + (nz - bot.z) * scale.max(0.05);
    }
    match ev {
        Some(entry) => crate::systems::event_bots::brain_move(world, entry, bot, nx, ny, nz, echo),
        None => bot_ai::broadcast_bot_move_ex(world, bot, nx, ny, nz, echo),
    }
    world.update_bot(bot.id, |b| {
        b.x = nx;
        b.y = ny;
        b.z = nz;
        b.region_x = calc_region(nx);
        b.region_z = calc_region(nz);
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
    });
    true
}

fn mark_tick(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
}

/// Positions of the bot's party mates in its zone (players and bots), the
/// leader's position, whether a real player is among them, and the mates' ids
/// (the Chaos ally rule).
fn party_mates_ex(world: &WorldState, bot: &BotInstance) -> (Vec<(f32, f32)>, Option<(f32, f32)>, bool, Vec<u32>) {
    let Some(pid) = world.get_party_id(bot.id as SessionId) else {
        return (Vec::new(), None, false, Vec::new());
    };
    let Some(party) = world.get_party(pid) else {
        return (Vec::new(), None, false, Vec::new());
    };
    let mut mates = Vec::new();
    let mut ids = Vec::new();
    let mut leader = None;
    let mut has_human = false;
    for sid in party.active_members() {
        if sid as u32 == bot.id {
            continue;
        }
        ids.push(sid as u32);
        let pos = if (sid as u32) < BOT_ID_BASE {
            world
                .get_position(sid)
                .filter(|p| p.zone_id == bot.zone_id)
                .map(|p| (p.x, p.z))
        } else {
            world
                .get_bot(sid as u32)
                .filter(|b| b.zone_id == bot.zone_id && b.is_alive())
                .map(|b| (b.x, b.z))
        };
        let Some(p) = pos else { continue };
        if (sid as u32) < BOT_ID_BASE {
            has_human = true;
        }
        if party.is_leader(sid) {
            leader = Some(p);
        }
        mates.push(p);
    }
    (mates, leader, has_human, ids)
}

/// Where to run when retreating: the centroid of the nearest friendly group
/// (allies within 60 m, excluding those standing among the enemies), else a
/// point 30 m away from the enemies' centroid.
pub fn retreat_point(me: (f32, f32), nation: u8, near: &[Unit]) -> (f32, f32) {
    let enemies: Vec<Unit> = near.iter().filter(|u| u.nation != nation).copied().collect();
    let allies: Vec<Unit> = near.iter().filter(|u| u.nation == nation).copied().collect();
    retreat_point_units(me, &enemies, &allies)
}

/// [`retreat_point`] over explicit enemy / ally lists.
pub fn retreat_point_units(me: (f32, f32), enemies: &[Unit], allies: &[Unit]) -> (f32, f32) {
    let (ecx, ecz) = if enemies.is_empty() {
        me
    } else {
        let n = enemies.len() as f32;
        (enemies.iter().map(|e| e.x).sum::<f32>() / n, enemies.iter().map(|e| e.z).sum::<f32>() / n)
    };
    let safe: Vec<&Unit> = allies.iter().filter(|u| u.dist(ecx, ecz) > 12.0).collect();
    if safe.len() >= 2 {
        let n = safe.len() as f32;
        return (safe.iter().map(|a| a.x).sum::<f32>() / n, safe.iter().map(|a| a.z).sum::<f32>() / n);
    }
    let dx = me.0 - ecx;
    let dz = me.1 - ecz;
    let d = (dx * dx + dz * dz).sqrt();
    if d < 0.01 {
        let a = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
        return (me.0 + a.cos() * 30.0, me.1 + a.sin() * 30.0);
    }
    (me.0 + dx / d * 30.0, me.1 + dz / d * 30.0)
}

/// Brain tick for a PK-zone bot. Returns `true` when the brain drove the bot
/// (the legacy `tick_fighting` must then not run).
pub fn tick_pk(world: &WorldState, bot: &BotInstance, now_ms: u64) -> bool {
    let cfg = config();
    if !cfg.enabled || bot.ai_state != BotAiState::Pk || !bot.is_alive() {
        return false;
    }
    tick_core(world, bot, None, now_ms)
}

/// Brain tick for an event bot (BDW / Chaos / Juraid). The event driver has
/// already checked that the bot is alive, the room is open and combat is on.
/// Always drives the bot (the event objective replaces the legacy loop).
pub fn tick_event(world: &WorldState, entry: &EventEntry, bot: &BotInstance, now_ms: u64) {
    if !bot.is_alive() {
        return;
    }
    tick_core(world, bot, Some(entry), now_ms);
}

/// The shared decision tick. `ev` selects the event rules (targets, allies,
/// movement broadcasts, attack mechanics) and the objective.
fn tick_core(world: &WorldState, bot: &BotInstance, ev: Option<&EventEntry>, now_ms: u64) -> bool {
    let cfg = config();
    let mut st = state_of(bot.id, now_ms);
    let mut rng = rand::thread_rng();
    let objective = objective_for(world, bot);

    // Memory of being hit.
    if bot.hp < st.last_hp && st.last_hp != i16::MAX {
        st.last_attacked_ms = now_ms;
        st.last_attacker = (bot.last_attacker_id >= 0).then_some(bot.last_attacker_id as u32);
    }
    st.last_hp = bot.hp;
    if now_ms >= st.reshuffle_at_ms {
        st.reshuffle(&cfg, now_ms, &mut rng);
    }

    // Perception.
    let snap = snapshot(world, bot.zone_id, now_ms);
    let near = snap.near(bot.x, bot.z, bot_ai::BOT_SEARCH_RANGE, bot.id);
    let now_unix = unix_now();
    let (mates, leader, has_human, mate_ids) = party_mates_ex(world, bot);
    let ffa = ev.is_some_and(|e| e.kind.is_ffa());
    let enemies: Vec<Unit> = if !objective.fights_players() {
        Vec::new()
    } else {
        near.iter()
            .filter(|u| match ev {
                Some(entry) => valid_event_enemy(world, entry, bot, u),
                None => {
                    u.nation != bot.nation
                        && if u.is_bot {
                            valid_bot_enemy(world, bot, u.id)
                        } else {
                            valid_player_enemy(world, bot, u.id as SessionId, now_unix)
                        }
                }
            })
            .filter(|u| !bot_ai::target_is_parked_unreachable(bot.id, u.id as i16, now_ms))
            .copied()
            .collect()
    };
    let is_ally = |u: &Unit| if ffa { mate_ids.contains(&u.id) } else { u.nation == bot.nation };
    let threat_enemies: Vec<Unit> = enemies.iter().filter(|u| u.dist(bot.x, bot.z) <= THREAT_RADIUS).copied().collect();
    let threat_allies: Vec<Unit> = near
        .iter()
        .filter(|u| u.dist(bot.x, bot.z) <= THREAT_RADIUS && is_ally(u))
        .copied()
        .collect();
    let under_attack = now_ms.saturating_sub(st.last_attacked_ms) <= ATTACKED_MEMORY_MS;
    let threat = assess_units(
        bot.hp,
        bot.max_hp,
        bot.mp,
        bot.max_mp,
        bot.x,
        bot.z,
        &threat_enemies,
        &threat_allies,
        under_attack,
    );

    // Consumables (never block movement).
    let enemies_near_for_scrolls = if objective.fights_players() { threat.enemies } else { usize::from(under_attack) };
    use_consumable(world, bot, &mut st, &cfg, enemies_near_for_scrolls, now_ms);
    let bot = &world.get_bot(bot.id).unwrap_or_else(|| bot.clone());

    // A cast in progress pins the bot.
    if now_ms < st.cast_lock_until_ms {
        store_state(bot.id, st);
        mark_tick(world, bot, now_ms);
        return true;
    }

    // Target selection (sticky; mistakes; objective bonus).
    let current = st.target.filter(|id| enemies.iter().any(|e| e.id == *id));
    let target = if current.is_some() && now_ms < st.target_until_ms {
        current.and_then(|id| enemies.iter().find(|e| e.id == id).copied())
    } else {
        let bonus = |u: &Unit| {
            let mut b = objective.target_bonus(world, bot, ev, u);
            // Assist the party: whoever hits a mate is worth more.
            if u.targeting.is_some_and(|t| mate_ids.contains(&t)) {
                b += 15.0;
            }
            b
        };
        let picked = pick_target_with(bot.id, bot.x, bot.z, &enemies, &near, current, &cfg, rng.gen_range(0..100), &bonus);
        if let Some(t) = picked {
            if st.target != Some(t.id) {
                st.reaction_until_ms = now_ms + reaction_delay(&cfg, &mut rng);
            }
            st.target = Some(t.id);
            st.target_until_ms = now_ms + TARGET_STICK_MS;
        } else {
            st.target = None;
        }
        picked
    };

    // Event bots act on their own: a party with a real player is assisted,
    // never followed (no Regroup toward the leader).
    let party_far = ev.is_none()
        && leader.is_some_and(|(lx, lz)| ((lx - bot.x).powi(2) + (lz - bot.z).powi(2)).sqrt() > BOT_PARTY_COHESION);
    let role = ClassRole::of(bot);
    let mode = decide(&threat, &cfg, role, target.is_some(), party_far, rng.gen_range(0..100), st.mode);
    if mode != st.mode {
        trace!(bot_id = bot.id, ?mode, hp = bot.hp, enemies = threat.enemies, allies = threat.allies, "brain mode");
    }
    st.mode = mode;

    // Per-bot dynamic obstacles (closed Juraid bridges) for every path query of this tick.
    let _dyn_guard = crate::zone::DynBlockGuard::install(objective.dyn_blocks(world, bot, ev));

    let handled = match mode {
        Mode::Retreat => {
            let (gx, gz) = retreat_point_units((bot.x, bot.z), &threat_enemies, &threat_allies);
            st.target = None;
            world.update_bot(bot.id, |b| b.target_id = -1);
            if !step_to_ex(world, bot, ev, gx, gz, 1, now_ms, 1.0) {
                mark_tick(world, bot, now_ms);
            }
            true
        }
        Mode::HealUp => {
            // Potions already fired above; priests self-heal through bot_ai.
            // Drift toward the party meanwhile so the bot is not alone.
            st.target = None;
            world.update_bot(bot.id, |b| b.target_id = -1);
            if let Some((lx, lz)) = leader.filter(|_| party_far) {
                let (gx, gz) = spread_from_mates((bot.x, bot.z), (lx, lz), &mates, st.spacing);
                step_to_ex(world, bot, ev, gx, gz, 3, now_ms, 1.0);
            } else {
                mark_tick(world, bot, now_ms);
            }
            true
        }
        Mode::Kite => {
            let Some(t) = target else { unreachable!("Kite needs a target") };
            // Back off along the line target -> bot, to the ranged keep distance.
            let dx = bot.x - t.x;
            let dz = bot.z - t.z;
            let d = (dx * dx + dz * dz).sqrt().max(0.01);
            let goal = (bot.x + dx / d * RANGED_KEEP_MIN, bot.z + dz / d * RANGED_KEEP_MIN);
            if !step_to_ex(world, bot, ev, goal.0, goal.1, 3, now_ms, 1.0) {
                // Cornered: stand and fight.
                engage(world, bot, ev, &mut st, t, &cfg, &mates, now_ms);
            }
            true
        }
        Mode::Engage => {
            let Some(t) = target else { unreachable!("Engage needs a target") };
            engage(world, bot, ev, &mut st, t, &cfg, &mates, now_ms);
            true
        }
        Mode::Regroup => {
            world.update_bot(bot.id, |b| b.target_id = -1);
            if has_human && ev.is_none() && bot_ai::move_toward_party_anchor(world, bot, now_ms) {
                true
            } else if let Some((lx, lz)) = leader {
                let goal = formation_point(role, (bot.x, bot.z), (lx, lz), st.spacing, st.formation_angle, 8.0 + st.spacing);
                let goal = spread_from_mates((bot.x, bot.z), goal, &mates, st.spacing);
                if !step_to_ex(world, bot, ev, goal.0, goal.1, 3, now_ms, 1.0) {
                    mark_tick(world, bot, now_ms);
                }
                true
            } else {
                false
            }
        }
        Mode::Objective => {
            world.update_bot(bot.id, |b| {
                b.target_id = -1;
                b.target_changed = false;
            });
            if has_human && ev.is_none() && bot_ai::move_toward_party_anchor(world, bot, now_ms) {
                true
            } else {
                match objective.drive(world, bot, &mut st, ev, now_ms) {
                    Drive::Handled => {
                        mark_tick(world, bot, now_ms);
                        true
                    }
                    Drive::FightNearest => {
                        if ev.is_some() {
                            mark_tick(world, bot, now_ms);
                            true
                        } else {
                            false
                        }
                    }
                    drive @ (Drive::Goal(..) | Drive::GoalScaled(..)) => {
                        let (gx, gz, scale) = match drive {
                            Drive::GoalScaled(x, z, s) => (x, z, s),
                            Drive::Goal(x, z) => (x, z, 1.0),
                            _ => unreachable!(),
                        };
                        let (gx, gz) = spread_from_mates((bot.x, bot.z), (gx, gz), &mates, st.spacing);
                        if !step_to_ex(world, bot, ev, gx, gz, 3, now_ms, scale) {
                            // unreachable spot: pick another one next tick
                            st.hotspot = None;
                            st.wander_goal = None;
                            mark_tick(world, bot, now_ms);
                        }
                        true
                    }
                }
            }
        }
    };
    if handled {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
    }
    store_state(bot.id, st);
    handled
}

/// Fight `t`: pick the skill, respect its range, the reaction delay, mana and
/// the cast-time lock; otherwise walk to the class formation point. Event bots
/// use the event attack mechanics (room-aware packets, event kill credit).
#[allow(clippy::too_many_arguments)]
fn engage(
    world: &WorldState,
    bot: &BotInstance,
    ev: Option<&EventEntry>,
    st: &mut BrainState,
    t: Unit,
    cfg: &BrainConfig,
    mates: &[(f32, f32)],
    now_ms: u64,
) {
    let target_sid = t.id as SessionId;
    let (skill_id, range) = match ev {
        Some(_) => (0u32, crate::systems::event_bots::attack_range(bot)),
        None => {
            let mut skill_id = bot_ai::select_bot_attack_skill(world, bot, target_sid, now_ms);
            // Mana management: a dry caster falls back to the free basic attack.
            let mp_cost = world.get_magic(skill_id as i32).and_then(|m| m.msp).unwrap_or(0);
            if mp_cost > 0 && bot.mp < mp_cost {
                skill_id = if bot.nation == NATION_KARUS { 101_001 } else { 201_001 };
            }
            (skill_id, bot_ai::bot_attack_range_for_skill(world, bot, skill_id, target_sid))
        }
    };
    let dist = t.dist(bot.x, bot.z);
    let role = ClassRole::of(bot);
    if dist <= range {
        if now_ms < st.reaction_until_ms {
            // Seen the target, not reacted yet: stand (a player turns first).
            world.update_bot(bot.id, |b| {
                b.target_id = target_sid as i16;
                b.last_tick_ms = now_ms;
            });
            return;
        }
        if let Some(entry) = ev {
            crate::systems::event_bots::brain_attack(world, entry, bot, t.id, t.x, t.z, now_ms);
            world.update_bot(bot.id, |b| b.target_id = target_sid as i16);
            return;
        }
        let ty = if t.is_bot {
            world.get_bot(t.id).map(|b| b.y).unwrap_or(bot.y)
        } else {
            world.get_position(target_sid).map(|p| p.y).unwrap_or(bot.y)
        };
        let before = world.get_bot(bot.id).map(|b| b.skill_cooldown[0]).unwrap_or(0);
        bot_ai::bot_perform_attack(world, bot, skill_id, target_sid, (t.x, ty, t.z), now_ms);
        let after = world.get_bot(bot.id).map(|b| b.skill_cooldown[0]).unwrap_or(0);
        if after != before {
            // A cast with a cast time locks movement for its duration.
            let cast_ms = world
                .get_magic(skill_id as i32)
                .and_then(|m| m.cast_time)
                .unwrap_or(0)
                .max(0) as u64
                * 100;
            if cast_ms > 0 {
                st.cast_lock_until_ms = now_ms + cast_ms;
            }
        }
        return;
    }
    if dist > bot_ai::BOT_SEARCH_RANGE {
        st.target = None;
        world.update_bot(bot.id, |b| {
            b.target_id = -1;
            b.last_tick_ms = now_ms;
        });
        return;
    }
    let goal = formation_point(role, (bot.x, bot.z), (t.x, t.z), st.spacing, st.formation_angle, range);
    let goal = spread_from_mates((bot.x, bot.z), goal, mates, st.spacing);
    let echo = if bot.target_id != target_sid as i16 { 1 } else { 3 };
    if step_to_ex(world, bot, ev, goal.0, goal.1, echo, now_ms, 1.0) {
        world.update_bot(bot.id, |b| {
            b.target_id = target_sid as i16;
            b.target_changed = false;
        });
    } else {
        bot_ai::park_unreachable_target(bot.id, target_sid as i16, now_ms);
        st.target = None;
        world.update_bot(bot.id, |b| {
            b.target_id = -1;
            b.last_tick_ms = now_ms;
        });
    }
    let _ = cfg;
}

// ── Party invitations from real players ──────────────────────────────────

const PARTY_ERR_DECLINED: i16 = -1;
const PARTY_ERR_LEVEL: i16 = -2;
const PARTY_ERR_NATION: i16 = -3;
const PARTY_ERR_ZONE: i16 = -5;
const PARTY_ERR_GENERIC: i16 = -7;

fn party_error(world: &WorldState, sid: SessionId, code: i16) {
    let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizParty as u8);
    pkt.write_u8(crate::handler::party::PARTY_INSERT);
    pkt.write_i16(code);
    world.send_to_session_owned(sid, pkt);
}

fn send_party_chat(world: &WorldState, party_id: u16, bot: &BotInstance, text: &str) {
    let pkt = crate::handler::chat::build_chat_packet(
        crate::handler::chat::ChatType::Party as u8,
        bot.nation,
        bot.id as u16,
        &bot.name,
        text,
        -1,
        1,
        1,
    );
    world.send_to_party(party_id, &pkt);
}

/// A real player invites PK bot `target_name`. The bot leaves its bot party
/// and joins the player's party (2-per-class rule respected). Returns `true`
/// when the name belongs to a PK bot (request fully handled).
pub fn try_party_invite(world: &WorldState, inviter_sid: SessionId, target_name: &str, party_type: i8) -> bool {
    let Some(bot) = world
        .bots
        .iter()
        .find(|e| e.ai_state == BotAiState::Pk && e.name.eq_ignore_ascii_case(target_name))
        .map(|e| e.value().clone())
    else {
        return false;
    };
    let Some(inviter) = world.get_character_info(inviter_sid) else { return true };
    let Some(ipos) = world.get_position(inviter_sid) else { return true };
    if !bot.is_alive() || ipos.zone_id != bot.zone_id {
        party_error(world, inviter_sid, PARTY_ERR_ZONE);
        return true;
    }
    if inviter.nation != bot.nation {
        party_error(world, inviter_sid, PARTY_ERR_NATION);
        return true;
    }
    if !crate::handler::party::is_level_compatible(inviter.level, bot.level) {
        party_error(world, inviter_sid, PARTY_ERR_LEVEL);
        return true;
    }
    let cfg = config();
    let bot_sid = bot.id as SessionId;

    // The inviter's party (created when he has none).
    let party_id = match world.get_party_id(inviter_sid) {
        Some(pid) => {
            let Some(party) = world.get_party(pid) else {
                party_error(world, inviter_sid, PARTY_ERR_GENERIC);
                return true;
            };
            if !party.is_leader(inviter_sid) || party.is_full() {
                party_error(world, inviter_sid, PARTY_ERR_GENERIC);
                return true;
            }
            let mut counts = [0usize; 5];
            let mut bots = 0usize;
            for sid in party.active_members() {
                if (sid as u32) >= BOT_ID_BASE {
                    bots += 1;
                    if let Some(b) = world.get_bot(sid as u32) {
                        let g = bot_ai::bot_class_group(b.class) as usize;
                        if g < 5 {
                            counts[g] += 1;
                        }
                    }
                }
            }
            if !party_accepts(&cfg, bots, &counts, bot_ai::bot_class_group(bot.class) as usize) {
                party_error(world, inviter_sid, PARTY_ERR_DECLINED);
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
                let sc = crate::handler::regene::build_state_change_broadcast(
                    inviter_sid as u32,
                    crate::state_change_constants::STATE_CHANGE_PARTY_LEADER,
                    1,
                );
                world.broadcast_to_region_sync(pos.zone_id, pos.region_x, pos.region_z, Arc::new(sc), None, room);
            }
            pid
        }
    };

    // Leave the bot party first (a player's invitation beats the bot group).
    if let Some(old) = world.get_party_id(bot_sid) {
        if old != party_id {
            send_party_chat(world, old, &bot, "Bir oyuncu çağırdı, ona katılıyorum.");
            world.cleanup_party_on_disconnect(bot_sid);
        }
    }
    if !world.add_party_member(party_id, bot_sid) {
        party_error(world, inviter_sid, PARTY_ERR_GENERIC);
        return true;
    }
    let target_number = world.get_party(party_id).map_or(-1, |p| p.target_number_id);
    let info = crate::handler::party::build_bot_party_member_info(&bot, 1, target_number);
    world.send_to_party(party_id, &info);
    bot_ai::broadcast_bot_party_hp(world, bot.id);
    send_party_chat(world, party_id, &bot, "Partiye katıldım, birlikte ilerleyelim.");
    debug!(bot_id = bot.id, inviter_sid, party_id, "PK bot joined a player's party");
    true
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn unit(id: u32, nation: u8, x: f32, z: f32, hp: i16, max_hp: i16, role: ClassRole) -> Unit {
        Unit { id, is_bot: false, nation, x, z, hp, max_hp, role, level: 70, targeting: None }
    }

    fn cfg() -> BrainConfig {
        BrainConfig::default()
    }

    // ── target selection ─────────────────────────────────────────────

    #[test]
    fn low_hp_priest_beats_nearest_full_hp_warrior() {
        let me = unit(1, 1, 0.0, 0.0, 1000, 1000, ClassRole::Warrior);
        let warrior = unit(10, 2, 3.0, 0.0, 1000, 1000, ClassRole::Warrior);
        let priest = unit(11, 2, 12.0, 0.0, 250, 1000, ClassRole::Priest);
        let near = vec![warrior, priest];
        let pick = pick_target(me.id, me.x, me.z, &near, &near, None, &cfg(), 99).unwrap();
        assert_eq!(pick.id, priest.id, "the hurt priest is the better target even though it is farther");
    }

    #[test]
    fn isolated_target_preferred_over_one_in_a_group() {
        let me = unit(1, 1, 0.0, 0.0, 1000, 1000, ClassRole::Rogue);
        // the lone target is a bit farther away and clearly outside the group's 12 m
        let alone = unit(10, 2, 0.0, 14.0, 1000, 1000, ClassRole::Warrior);
        let grouped = unit(11, 2, 8.0, 0.0, 1000, 1000, ClassRole::Warrior);
        let g1 = unit(12, 2, 9.0, 2.0, 1000, 1000, ClassRole::Warrior);
        let g2 = unit(13, 2, 9.0, -2.0, 1000, 1000, ClassRole::Priest);
        let enemies = vec![alone, grouped];
        let all = vec![alone, grouped, g1, g2];
        let pick = pick_target(me.id, me.x, me.z, &enemies, &all, None, &cfg(), 99).unwrap();
        assert_eq!(pick.id, alone.id);
    }

    #[test]
    fn whoever_hits_me_gets_priority_and_mistake_roll_takes_second_best() {
        let me_id = 1;
        let mut attacker = unit(10, 2, 9.0, 0.0, 1000, 1000, ClassRole::Warrior);
        attacker.targeting = Some(me_id);
        let other = unit(11, 2, 8.0, 0.0, 1000, 1000, ClassRole::Warrior);
        let near = vec![attacker, other];
        let best = pick_target(me_id, 0.0, 0.0, &near, &near, None, &cfg(), 50).unwrap();
        assert_eq!(best.id, attacker.id);
        let mistake = pick_target(me_id, 0.0, 0.0, &near, &near, None, &cfg(), 0).unwrap();
        assert_eq!(mistake.id, other.id, "roll below target_mistake_percent picks the runner-up");
    }

    #[test]
    fn current_target_is_sticky() {
        let a = unit(10, 2, 9.0, 0.0, 1000, 1000, ClassRole::Warrior);
        let b = unit(11, 2, 8.0, 0.0, 1000, 1000, ClassRole::Warrior);
        let near = vec![a, b];
        let pick = pick_target(1, 0.0, 0.0, &near, &near, Some(a.id), &cfg(), 99).unwrap();
        assert_eq!(pick.id, a.id);
    }

    #[test]
    fn no_enemies_no_target() {
        assert!(pick_target(1, 0.0, 0.0, &[], &[], None, &cfg(), 0).is_none());
    }

    // ── threat / decisions ───────────────────────────────────────────

    #[test]
    fn assess_counts_sides_and_nearest_enemy() {
        let near = vec![
            unit(10, 2, 5.0, 0.0, 500, 1000, ClassRole::Warrior),
            unit(11, 2, 20.0, 0.0, 500, 1000, ClassRole::Mage),
            unit(12, 1, 3.0, 3.0, 200, 1000, ClassRole::Priest),
        ];
        let t = assess(800, 1000, 50, 100, 1, 0.0, 0.0, &near, true);
        assert_eq!(t.enemies, 2);
        assert_eq!(t.allies, 2);
        assert!((t.nearest_enemy - 5.0).abs() < 1e-3);
        assert!((t.hp_ratio - 0.8).abs() < 1e-6);
        assert!((t.ally_low_ratio - 0.2).abs() < 1e-6);
        assert!(t.under_attack);
    }

    fn threat(hp: f32, enemies: usize, allies: usize, nearest: f32) -> Threat {
        Threat { hp_ratio: hp, mp_ratio: 1.0, enemies, allies, under_attack: enemies > 0, nearest_enemy: nearest, ally_low_ratio: 1.0 }
    }

    #[test]
    fn low_hp_with_enemies_retreats_and_quiet_low_hp_heals_up() {
        let c = cfg();
        assert_eq!(decide(&threat(0.2, 1, 1, 5.0), &c, ClassRole::Warrior, true, false, 99, Mode::Engage), Mode::Retreat);
        assert_eq!(decide(&threat(0.2, 0, 1, f32::MAX), &c, ClassRole::Warrior, false, false, 99, Mode::Retreat), Mode::HealUp);
    }

    #[test]
    fn healed_bot_returns_to_the_fight() {
        let c = cfg();
        // still below the recovery threshold: keeps healing up
        assert_eq!(decide(&threat(0.5, 0, 1, f32::MAX), &c, ClassRole::Warrior, false, false, 99, Mode::HealUp), Mode::HealUp);
        // recovered: back to engaging
        assert_eq!(decide(&threat(0.75, 1, 1, 5.0), &c, ClassRole::Warrior, true, false, 99, Mode::HealUp), Mode::Engage);
    }

    #[test]
    fn outnumbered_bot_retreats() {
        let c = cfg();
        assert_eq!(decide(&threat(0.9, 2, 1, 10.0), &c, ClassRole::Warrior, true, false, 99, Mode::Engage), Mode::Retreat);
        assert_eq!(decide(&threat(0.9, 2, 2, 10.0), &c, ClassRole::Warrior, true, false, 99, Mode::Engage), Mode::Engage);
        assert_eq!(decide(&threat(0.9, 1, 1, 10.0), &c, ClassRole::Warrior, true, false, 99, Mode::Engage), Mode::Engage, "a single enemy is never a reason to run");
    }

    #[test]
    fn ranged_class_kites_imperfectly() {
        let c = cfg(); // kite_miss_percent 25
        assert_eq!(decide(&threat(0.9, 1, 1, 3.0), &c, ClassRole::Mage, true, false, 99, Mode::Engage), Mode::Kite);
        assert_eq!(decide(&threat(0.9, 1, 1, 3.0), &c, ClassRole::Mage, true, false, 10, Mode::Engage), Mode::Engage, "roll below kite_miss_percent: forgot to kite");
        assert_eq!(decide(&threat(0.9, 1, 1, 3.0), &c, ClassRole::Warrior, true, false, 99, Mode::Engage), Mode::Engage);
    }

    #[test]
    fn no_target_means_regroup_or_objective() {
        let c = cfg();
        assert_eq!(decide(&threat(1.0, 0, 1, f32::MAX), &c, ClassRole::Rogue, false, true, 99, Mode::Objective), Mode::Regroup);
        assert_eq!(decide(&threat(1.0, 0, 1, f32::MAX), &c, ClassRole::Rogue, false, false, 99, Mode::Objective), Mode::Objective);
    }

    #[test]
    fn retreat_point_runs_to_friends_or_away_from_enemies() {
        let near = vec![
            unit(10, 2, 10.0, 0.0, 1000, 1000, ClassRole::Warrior),
            unit(20, 1, -30.0, 0.0, 1000, 1000, ClassRole::Priest),
            unit(21, 1, -34.0, 2.0, 1000, 1000, ClassRole::Mage),
        ];
        let (x, _) = retreat_point((0.0, 0.0), 1, &near);
        assert!(x < -25.0, "runs to the friendly group, got {x}");
        let lonely = vec![unit(10, 2, 10.0, 0.0, 1000, 1000, ClassRole::Warrior)];
        let (x, _) = retreat_point((0.0, 0.0), 1, &lonely);
        assert!((x + 30.0).abs() < 1e-3, "runs 30 m away from the enemy, got {x}");
    }

    // ── formation / spacing ──────────────────────────────────────────

    #[test]
    fn formation_points_per_role() {
        let bot = (0.0, 0.0);
        let target = (20.0, 0.0);
        assert_eq!(formation_point(ClassRole::Warrior, bot, target, 5.0, 0.3, 7.0), target);
        let (rx, rz) = formation_point(ClassRole::Rogue, bot, target, 5.0, 0.3, 7.0);
        assert!((rx - 20.0).abs() < 1e-3 && (rz.abs() - 5.0).abs() < 1e-3, "rogue flanks at spacing: ({rx},{rz})");
        let (mx, mz) = formation_point(ClassRole::Mage, bot, target, 5.0, 0.3, 18.0);
        assert!((mx - 4.0).abs() < 1e-3 && mz.abs() < 1e-3, "mage stops at range-2 = 16 from the target: ({mx},{mz})");
        let (px, _) = formation_point(ClassRole::Priest, (15.0, 0.0), target, 5.0, 0.3, 18.0);
        assert!((px - 15.0).abs() < 1e-3, "already inside keep range: stays");
    }

    #[test]
    fn spacing_is_reshuffled_inside_the_configured_band() {
        let c = cfg();
        let mut rng = StdRng::seed_from_u64(7);
        let mut st = BrainState::new(&c, 0, &mut rng);
        let mut seen = std::collections::HashSet::new();
        for i in 0..50u64 {
            st.reshuffle(&c, i * 1000, &mut rng);
            assert!(st.spacing >= c.spacing_min as f32 && st.spacing <= c.spacing_max as f32);
            assert!(st.reshuffle_at_ms > i * 1000);
            seen.insert((st.spacing * 10.0) as i32);
        }
        assert!(seen.len() > 5, "spacing varies per reshuffle");
    }

    #[test]
    fn spread_from_mates_pushes_away_only_when_too_close() {
        let goal = (10.0, 10.0);
        assert_eq!(spread_from_mates((0.0, 0.0), goal, &[(20.0, 0.0)], 5.0), goal);
        let (gx, gz) = spread_from_mates((0.0, 0.0), goal, &[(2.0, 0.0)], 5.0);
        assert!(gx < 10.0 && (gz - 10.0).abs() < 1e-3, "pushed away from the mate on +x: ({gx},{gz})");
    }

    #[test]
    fn roles_split_roughly_by_patrol_percent() {
        let c = BrainConfig { patrol_percent: 40, ..cfg() };
        let mut rng = StdRng::seed_from_u64(1);
        let patrol = (0..2000).filter(|_| BrainState::new(&c, 0, &mut rng).role == Role::Patrol).count();
        assert!((700..=900).contains(&patrol), "about 40 % patrol, got {patrol}/2000");
    }

    // ── party composition ────────────────────────────────────────────

    #[test]
    fn party_rules_six_bots_two_per_class_priest_first() {
        let c = cfg();
        assert!(party_accepts(&c, 5, &[0, 2, 2, 1, 0], 4));
        assert!(!party_accepts(&c, 6, &[0, 2, 2, 1, 1], 3), "six bots: full");
        assert!(!party_accepts(&c, 4, &[0, 2, 1, 1, 0], 1), "two warriors already");
        assert!(!party_accepts(&c, 0, &[0; 5], 0), "role-less bot never");
        let counts = [0usize, 2, 1, 0, 0];
        let priest = recruit_key(&counts, 4, 50);
        let mage = recruit_key(&counts, 3, 10);
        assert!(priest < mage, "a party without priest recruits the priest first");
        let with_priest = [0usize, 2, 1, 0, 1];
        assert!(recruit_key(&with_priest, 3, 10) < recruit_key(&with_priest, 4, 5), "with a priest: least represented role");
    }

    // ── consumables ──────────────────────────────────────────────────

    #[test]
    fn potions_follow_stock_and_cooldown() {
        let c = cfg();
        let mut rng = StdRng::seed_from_u64(3);
        let mut st = BrainState::new(&c, 0, &mut rng);
        st.hp_potions = 2;
        assert_eq!(choose_consumable(&st, 0.5, 1.0, ClassRole::Warrior, 1, 1000), Some(Consumable::HpPotion));
        consume(&mut st, Consumable::HpPotion, &c, 1000);
        assert_eq!(st.hp_potions, 1);
        assert_eq!(choose_consumable(&st, 0.3, 1.0, ClassRole::Warrior, 1, 2000), None, "2.4 s cooldown");
        assert_eq!(choose_consumable(&st, 0.3, 1.0, ClassRole::Warrior, 1, 3500), Some(Consumable::HpPotion));
        consume(&mut st, Consumable::HpPotion, &c, 3500);
        assert_eq!(choose_consumable(&st, 0.3, 1.0, ClassRole::Warrior, 1, 10_000), None, "stock empty");
        assert_eq!(choose_consumable(&st, 0.9, 1.0, ClassRole::Warrior, 1, 10_000), None, "healthy: nothing");
    }

    #[test]
    fn casters_drink_mana_and_scrolls_only_when_quiet() {
        let c = cfg();
        let mut rng = StdRng::seed_from_u64(3);
        let mut st = BrainState::new(&c, 0, &mut rng);
        assert_eq!(choose_consumable(&st, 0.9, 0.2, ClassRole::Mage, 2, 1000), Some(Consumable::MpPotion));
        assert_eq!(choose_consumable(&st, 0.9, 0.2, ClassRole::Warrior, 2, 1000), None, "a warrior at 20 % MP does not care");
        assert_eq!(choose_consumable(&st, 0.9, 0.9, ClassRole::Warrior, 1, 1000), None, "enemies near: no scrolls");
        assert_eq!(choose_consumable(&st, 0.9, 0.9, ClassRole::Warrior, 0, 1000), Some(Consumable::BuffScroll));
        st.buff_scrolls = 0;
        assert_eq!(choose_consumable(&st, 0.9, 0.9, ClassRole::Warrior, 0, 1000), Some(Consumable::Transform));
        st.ts_scrolls = 0;
        assert_eq!(choose_consumable(&st, 0.9, 0.9, ClassRole::Warrior, 0, 1000), None);
    }

    #[test]
    fn respawn_refills_the_stock() {
        let c = cfg();
        let mut rng = StdRng::seed_from_u64(3);
        let mut st = BrainState::new(&c, 0, &mut rng);
        st.hp_potions = 0;
        st.refill(&c, 5000);
        assert_eq!(st.hp_potions, c.hp_potions);
        assert_eq!(st.spawned_ms, 5000);
    }

    // ── NP anti-farm ─────────────────────────────────────────────────

    #[test]
    fn np_share_and_antifarm_rules() {
        let c = cfg(); // 33 %, 300 s window, 20 s grace
        assert_eq!(scale_np(&c, 64), 21);
        assert_eq!(scale_np(&c, 32), 10);
        assert_eq!(scale_np(&c, 1), 1, "never rounds a positive reward to zero");
        assert_eq!(scale_np(&BrainConfig { np_percent: 0, ..cfg() }, 64), 0);
        assert!(!kill_pays(&c, 100_000, None, 110_000), "killed 10 s after respawn: grace");
        assert!(kill_pays(&c, 100_000, None, 130_000));
        assert!(!kill_pays(&c, 0, Some(200_000), 150_000), "same killer inside the window");
        assert!(kill_pays(&c, 0, Some(200_000), 200_000));
    }

    #[test]
    fn bot_kill_np_records_window_and_verdict() {
        set_config(cfg());
        let bot_id = BOT_ID_BASE + 777;
        let killer: SessionId = 4242;
        on_bot_respawn(bot_id, 1_000_000);
        // fresh spawn: nothing
        assert_eq!(bot_kill_np(killer, bot_id, 64, 1_005_000), 0);
        assert!(!kill_rewardable(killer, 1_005_000));
        // after the grace: the share, once
        assert_eq!(bot_kill_np(killer, bot_id, 64, 1_030_000), 21);
        assert!(kill_rewardable(killer, 1_030_000));
        assert_eq!(bot_kill_np(killer, bot_id, 64, 1_040_000), 0, "farming the same bot");
        assert!(!kill_rewardable(killer, 1_040_000));
        assert!(kill_rewardable(killer, 1_040_000 + VERDICT_TTL_MS + 1), "stale verdicts do not block later kills");
        // a different killer pays
        assert_eq!(bot_kill_np(4243, bot_id, 64, 1_040_000), 21);
        // window over
        assert_eq!(bot_kill_np(killer, bot_id, 64, 1_030_000 + 300_001), 21);
        forget(bot_id);
    }

    // ── reaction delay / snapshot / hotspots ─────────────────────────

    #[test]
    fn reaction_delay_blocks_only_the_first_hit_on_a_new_target() {
        set_config(cfg());
        let bot_id = BOT_ID_BASE + 778;
        forget(bot_id);
        assert!(!reaction_ready(bot_id, 5, 10_000), "new target: wait at least 300 ms");
        assert!(reaction_ready(bot_id, 5, 10_000 + 1000), "900 ms at most");
        assert!(reaction_ready(bot_id, 5, 11_100));
        assert!(!reaction_ready(bot_id, 6, 11_100), "target switch starts a new delay");
        forget(bot_id);
    }

    #[test]
    fn snapshot_grid_finds_units_within_radius() {
        let units = vec![
            unit(1, 1, 0.0, 0.0, 1, 1, ClassRole::Warrior),
            unit(2, 2, 10.0, 0.0, 1, 1, ClassRole::Warrior),
            unit(3, 2, 100.0, 100.0, 1, 1, ClassRole::Warrior),
            unit(4, 2, 31.0, 0.0, 1, 1, ClassRole::Warrior),
        ];
        let s = Snapshot::from_units(71, 1, units);
        let near = s.near(0.0, 0.0, 32.0, 1);
        let ids: Vec<u32> = near.iter().map(|u| u.id).collect();
        assert!(ids.contains(&2) && ids.contains(&4) && !ids.contains(&3) && !ids.contains(&1));
    }

    #[test]
    fn hotspots_exist_for_every_pk_zone_and_prefer_enemy_fronts() {
        for zone in [ZONE_RONARK_LAND, ZONE_ARDREAM, ZONE_RONARK_LAND_BASE] {
            assert!(pick_hotspot(zone, 1, 12345).is_some());
        }
        assert!(pick_hotspot(21, 1, 1).is_none());
        // Karus bot: the El Morad front (660,927) weighs 20, the Karus front (1327,1060) weighs 5
        let mut elmo = 0;
        let mut karus = 0;
        for roll in 0..1000u32 {
            match pick_hotspot(ZONE_RONARK_LAND, 1, roll * 7919) {
                Some((x, _)) if (x - 660.0).abs() < 1.0 => elmo += 1,
                Some((x, _)) if (x - 1327.0).abs() < 1.0 => karus += 1,
                _ => {}
            }
        }
        assert!(elmo > karus * 2, "enemy front {elmo} vs own front {karus}");
    }

    #[test]
    fn config_is_sanitized() {
        let c = BrainConfig {
            patrol_percent: 200,
            reaction_min_ms: 900,
            reaction_max_ms: 300,
            party_max_bots: 20,
            spacing_min: 10,
            spacing_max: 2,
            ..cfg()
        }
        .sanitized();
        assert_eq!(c.patrol_percent, 100);
        assert!(c.reaction_max_ms >= c.reaction_min_ms);
        assert_eq!(c.party_max_bots, 8);
        assert!(c.spacing_max >= c.spacing_min);
    }

    // ── integration: a PK bot with the real tick ─────────────────────

    #[test]
    fn brain_takes_over_the_pk_tick_and_moves_toward_a_hotspot() {
        use crate::systems::bot_ai::{spawn_gm_bot, SpawnGmBotParams};
        let world = WorldState::new();
        let id = spawn_gm_bot(
            &world,
            SpawnGmBotParams { zone_id: ZONE_ARDREAM, x: 300.0, y: 0.0, z: 300.0, class: 1, level: 59, nation: 1, ai_state: BotAiState::Pk },
        );
        // The config is process-global (other tests may change it concurrently):
        // pin this bot's role directly instead of relying on patrol_percent.
        forget(id);
        {
            let mut st = state_of(id, 10_000);
            st.role = Role::Hotspot;
            store_state(id, st);
        }
        let bot = world.get_bot(id).unwrap();
        // no map data in a bare test world: pathing refuses, but the brain still owns the tick
        assert!(tick_pk(&world, &bot, 10_000));
        let st = state_of(id, 10_000);
        assert_eq!(st.role, Role::Hotspot);
        assert_eq!(st.mode, Mode::Objective, "alone in the zone: objective layer");
        // (without map data the step is refused and the spot is dropped for a re-pick,
        // so only role/mode are asserted here; hotspot picking is covered above)
        forget(id);
    }

    #[test]
    fn two_enemy_bots_engage_each_other_with_reaction_delay() {
        use crate::systems::bot_ai::{spawn_gm_bot, SpawnGmBotParams};
        // (a single candidate cannot be "mistaken", so no config change is needed)
        let world = WorldState::new();
        let a = spawn_gm_bot(
            &world,
            SpawnGmBotParams { zone_id: ZONE_ARDREAM, x: 300.0, y: 0.0, z: 300.0, class: 1, level: 59, nation: 1, ai_state: BotAiState::Pk },
        );
        let b = spawn_gm_bot(
            &world,
            SpawnGmBotParams { zone_id: ZONE_ARDREAM, x: 303.0, y: 0.0, z: 300.0, class: 1, level: 59, nation: 2, ai_state: BotAiState::Pk },
        );
        let bot = world.get_bot(a).unwrap();
        assert!(tick_pk(&world, &bot, 20_000));
        let st = state_of(a, 20_000);
        assert_eq!(st.mode, Mode::Engage);
        assert_eq!(st.target, Some(b));
        assert!(st.reaction_until_ms > 20_000, "first contact: reaction delay running");
        let victim_before = world.get_bot(b).unwrap().hp;
        // within the delay: no hit yet
        let bot = world.get_bot(a).unwrap();
        tick_pk(&world, &bot, 20_100);
        assert_eq!(world.get_bot(b).unwrap().hp, victim_before);
        // after the delay: the attack fires (the bot keeps the target)
        let bot = world.get_bot(a).unwrap();
        tick_pk(&world, &bot, 25_000);
        let after = world.get_bot(a).unwrap();
        assert_eq!(after.target_id, b as i16);
        forget(a);
        forget(b);
        set_config(cfg());
    }
}
