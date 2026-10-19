//! Zone and region grid for spatial partitioning, map data, and zone rules.
//! -  — C3DMap (zone flags, events, abilities)
//! -  — CRegion
//! -  — CGameEvent
//! Each zone (map) is divided into a 2D grid of regions.
//! Region size = VIEW_DISTANCE (48 units).
//! Nearby players = current region + 8 surrounding (3×3 grid).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ko_protocol::nav::{NavMask, WalkedMap};
use ko_protocol::smd::{SmdFile, WarpInfo};
use parking_lot::RwLock;
use smallvec::SmallVec;

use crate::npc::NpcId;

/// Region cell size in world units.
pub const VIEW_DISTANCE: u16 = 48;

/// Session identifier — unique per connection.
pub type SessionId = u16;

// ─── Zone Ability Types ─────────────────────────────────────────────────

/// Zone type enum matching the `ZoneAbilityType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZoneAbilityType {
    /// Safe zone — no PvP, no penalties.
    Neutral = 0,
    /// Player vs Player enabled.
    PvP = 1,
    /// Spectator zone.
    Spectator = 2,
    /// Siege warfare zones (1-3).
    Siege1 = 3,
    Siege2 = 4,
    Siege3 = 5,
    /// Siege disabled.
    SiegeDisabled = 6,
    /// Caithros arena.
    CaitharosArena = 7,
    /// PvP with neutral NPCs.
    PvpNeutralNpcs = 8,
    /// PvP with stone NPCs.
    PvpStoneNpcs = 9,
}

impl ZoneAbilityType {
    /// Convert from raw DB value.
    pub fn from_i16(val: i16) -> Self {
        match val {
            1 => Self::PvP,
            2 => Self::Spectator,
            3 => Self::Siege1,
            4 => Self::Siege2,
            5 => Self::Siege3,
            6 => Self::SiegeDisabled,
            7 => Self::CaitharosArena,
            8 => Self::PvpNeutralNpcs,
            9 => Self::PvpStoneNpcs,
            _ => Self::Neutral,
        }
    }
}

// ─── Game Event Types ───────────────────────────────────────────────────

/// Game event type — loaded from the `game_event` DB table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameEventType {
    /// Teleport to another zone (exec1=zone, exec2=x, exec3=z).
    ZoneChange = 1,
    /// Instant death trap.
    TrapDead = 2,
    /// Area damage trap.
    TrapArea = 3,
}

impl GameEventType {
    /// Convert from raw DB value.
    pub fn from_i16(val: i16) -> Option<Self> {
        match val {
            1 => Some(Self::ZoneChange),
            2 => Some(Self::TrapDead),
            3 => Some(Self::TrapArea),
            _ => None,
        }
    }
}

/// A game event loaded from the database.
#[derive(Debug, Clone)]
pub struct GameEvent {
    pub event_type: GameEventType,
    pub cond: [i32; 5],
    pub exec: [i32; 5],
}

// ─── Object Events ──────────────────────────────────────────────────────

/// Object type enum for interactive world objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum ObjectType {
    /// Bind point (respawn location).
    Bind = 0,
    /// Gate (openable/closable by NPC).
    Gate = 1,
    /// Gate lever (toggle associated gate during war/siege).
    GateLever = 3,
    /// Flag lever (captures flags during battle).
    FlagLever = 4,
    /// Warp gate (opens warp list UI).
    WarpGate = 5,
    /// Remove bind point.
    RemoveBind = 7,
    /// Anvil (opens item upgrade UI).
    Anvil = 8,
    /// Krowaz key gate (requires key items to open).
    KrowazGate = 12,
    /// Wood (burning log lever).
    Wood = 14,
    /// Wood lever.
    WoodLever = 15,
}

impl ObjectType {
    /// Convert from raw DB value.
    pub fn from_i16(val: i16) -> Option<Self> {
        match val {
            0 => Some(Self::Bind),
            1 => Some(Self::Gate),
            3 => Some(Self::GateLever),
            4 => Some(Self::FlagLever),
            5 => Some(Self::WarpGate),
            7 => Some(Self::RemoveBind),
            8 => Some(Self::Anvil),
            12 => Some(Self::KrowazGate),
            14 => Some(Self::Wood),
            15 => Some(Self::WoodLever),
            _ => None,
        }
    }
}

/// An interactive object event in a zone (bind point, warp gate, lever, anvil, etc.).
#[derive(Debug, Clone)]
pub struct ObjectEventInfo {
    /// Auto-assigned index (position in array).
    pub index: i32,
    /// Zone this object belongs to.
    pub zone_id: u16,
    /// Nation restriction (0=all, 1=karus, 2=elmorad).
    pub belong: i16,
    /// Object index within the zone.
    pub s_index: i16,
    /// Object type.
    pub obj_type: i16,
    /// Associated NPC ID or warp group.
    pub control_npc: i16,
    /// Status: 0=inactive, 1=active.
    pub status: i16,
    /// World X coordinate.
    pub pos_x: f32,
    /// World Y coordinate.
    pub pos_y: f32,
    /// World Z coordinate.
    pub pos_z: f32,
    /// Life flag (`byLife`). Only objects with by_life == 1 are valid bind points.
    pub by_life: i16,
}

// ─── Zone Info ──────────────────────────────────────────────────────────

/// Zone configuration from the database.
#[derive(Debug, Clone)]
pub struct ZoneInfo {
    pub smd_name: String,
    pub zone_name: String,
    pub zone_type: ZoneAbilityType,
    pub min_level: u8,
    pub max_level: u8,
    /// Spawn position (world coords = DB value / 100).
    pub init_x: f32,
    pub init_z: f32,
    pub init_y: f32,
    pub abilities: ZoneAbilities,
    /// Zone status (`m_Status`). 0=inactive, 1=active.
    pub status: i16,
}

/// Zone ability flags — determines what actions are allowed in this zone.
#[derive(Debug, Clone, Default)]
pub struct ZoneAbilities {
    pub trade_other_nation: bool,
    pub talk_other_nation: bool,
    pub attack_other_nation: bool,
    pub attack_same_nation: bool,
    pub friendly_npc: bool,
    pub war_zone: bool,
    pub clan_updates: bool,
    pub teleport: bool,
    pub gate: bool,
    pub escape: bool,
    pub calling_friend: bool,
    pub teleport_friend: bool,
    pub blink: bool,
    pub pet_spawn: bool,
    pub exp_lost: bool,
    pub give_loyalty: bool,
    pub guard_summon: bool,
    pub military_zone: bool,
    pub mining_zone: bool,
    pub blink_zone: bool,
    pub auto_loot: bool,
    pub gold_lose: bool,
}

// ─── Map Data ───────────────────────────────────────────────────────────

/// Parsed map data from an SMD file, plus the client-derived bot navigation
/// data (`Map/nav/<zone>.nav`, see `ko_protocol::nav`) when it exists.
#[derive(Debug)]
pub struct MapData {
    /// Parsed SMD file with event grid, warps, dimensions.
    smd: SmdFile,
    /// The event grid carries no data at all (e.g. Moradon): bots then rely on the terrain
    /// mask alone. Maps with a real event grid (PK zones) keep the strict rule.
    bot_ignore_event: bool,
    /// Event arenas (BDW / Chaos / Juraid): the event grid is walkable in addition to the client
    /// mask. Their doors and bridges are dynamic objects, so the baked collision alone leaves
    /// the spawn rooms closed pockets; the server-authored event grid knows the arena layout.
    bot_event_only: bool,
    /// Authored openings of an event arena (`systems::event_nav`), world rectangles bots may
    /// walk in addition to the mask / event grid.
    event_openings: &'static [crate::systems::event_nav::Rect],
    /// Client-derived mask + player trace, swapped atomically on reload.
    bot_nav: parking_lot::RwLock<Arc<BotNav>>,
}

/// Minimum number of player-walked cells before the trace influences bot
/// routing (`KO_NAV_WALKED_MIN_CELLS`, default 1500).
pub fn walked_min_cells() -> usize {
    static MIN: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *MIN.get_or_init(|| {
        std::env::var("KO_NAV_WALKED_MIN_CELLS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500)
    })
}

/// One generation of bot navigation data for a zone.
#[derive(Debug)]
pub struct BotNav {
    /// Client-derived walkability (`None`: fall back to the SMD-derived mask).
    pub nav: Option<NavMask>,
    /// Cells/edges real players were seen walking (grid of the active mask).
    pub walked: Arc<WalkedMap>,
    /// Grid cells per axis of the active mask.
    pub grid_size: i32,
}

impl BotNav {
    /// Enough trace data to let bots prefer player-walked cells.
    pub fn walked_ready(&self) -> bool {
        self.walked.walked_count() >= walked_min_cells()
    }
}

/// Snapshot of a map's bot navigation state for one query (no lock per cell).
pub struct BotView<'a> {
    map: &'a MapData,
    nav: Arc<BotNav>,
    walked_ready: bool,
}

impl BotView<'_> {
    #[inline]
    pub fn grid_size(&self) -> i32 {
        self.nav.grid_size
    }

    #[inline]
    pub fn walkable(&self, x: i32, z: i32) -> bool {
        self.map.bot_cell_walkable(&self.nav, x, z)
    }

    /// Extra A* cost for stepping into `(x, z)`: cells no real player has
    /// walked cost more once the zone has enough trace data.
    #[inline]
    pub fn step_penalty(&self, x: i32, z: i32) -> i32 {
        if self.walked_ready && !self.nav.walked.is_walked(x, z) {
            crate::systems::pathfind::COST_UNWALKED_PENALTY
        } else {
            0
        }
    }

    pub fn walked_ready(&self) -> bool {
        self.walked_ready
    }
}

thread_local! {
    /// Extra circles `(x, z, radius)` (world units) a bot must treat as
    /// blocked for the duration of one path query: closed Juraid bridge
    /// gates of the bot's own room and nation. Per room state cannot live in
    /// the zone-wide mask, so the caller installs it around its path step
    /// (`DynBlockGuard`).
    static BOT_DYN_BLOCKS: std::cell::RefCell<Vec<(f32, f32, f32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Installs per-bot dynamic obstacles for the current thread until dropped.
pub struct DynBlockGuard;

impl DynBlockGuard {
    pub fn install(blocks: Vec<(f32, f32, f32)>) -> Self {
        BOT_DYN_BLOCKS.with(|b| *b.borrow_mut() = blocks);
        DynBlockGuard
    }
}

impl Drop for DynBlockGuard {
    fn drop(&mut self) {
        BOT_DYN_BLOCKS.with(|b| b.borrow_mut().clear());
    }
}

fn dyn_blocked(unit: f32, x: i32, z: i32) -> bool {
    BOT_DYN_BLOCKS.with(|b| {
        let b = b.borrow();
        if b.is_empty() {
            return false;
        }
        let cx = (x as f32 + 0.5) * unit;
        let cz = (z as f32 + 0.5) * unit;
        b.iter().any(|&(bx, bz, r)| (cx - bx).powi(2) + (cz - bz).powi(2) <= r * r)
    })
}

impl MapData {
    fn bot_cell_walkable(&self, nav: &BotNav, x: i32, z: i32) -> bool {
        if dyn_blocked(self.smd.unit_dist, x, z) {
            return false;
        }
        // Event arenas (BDW / Chaos / Juraid): the SMD event grid is the arena
        // layout the server authored (doors and bridges are dynamic objects
        // that the client's baked collision does not contain, so the client
        // mask alone seals the spawn rooms). A cell is walkable when either
        // the event grid or the client mask says so.
        if self.bot_event_only {
            let on_event_grid = x >= 0
                && z >= 0
                && x < self.smd.map_size
                && z < self.smd.map_size
                && self.smd.get_event_id(x, z) > 0;
            return on_event_grid
                || nav.nav.as_ref().is_some_and(|m| m.is_walkable(x, z))
                || (!self.event_openings.is_empty()
                    && x >= 0
                    && z >= 0
                    && crate::systems::event_nav::contains(
                        self.event_openings,
                        (x as f32 + 0.5) * self.smd.unit_dist,
                        (z as f32 + 0.5) * self.smd.unit_dist,
                    ));
        }
        // Client-derived mask: authoritative. The SMD event grid is not
        // consulted, it may belong to a different revision of the map.
        if let Some(mask) = &nav.nav {
            return mask.is_walkable(x, z);
        }
        // Legacy: terrain mask (zero = blocked) from the SMD.
        x >= 0 && z >= 0 && x < self.smd.map_size && z < self.smd.map_size
            && (self.bot_ignore_event || self.smd.get_event_id(x, z) > 0) && (self.smd.bot_blocked.is_empty()
            || !self.smd.bot_blocked[(x * self.smd.map_size + z) as usize])
    }

    pub fn is_bot_movable_grid(&self, x: i32, z: i32) -> bool {
        let nav = self.bot_nav.read().clone();
        self.bot_cell_walkable(&nav, x, z)
    }

    pub fn is_bot_movable(&self, x: f32, z: f32) -> bool {
        x.is_finite() && z.is_finite() && self.is_valid_position(x, z)
            && self.is_bot_movable_grid((x / self.unit_dist()) as i32, (z / self.unit_dist()) as i32)
    }

    /// Snapshot for a path query.
    pub fn bot_view(&self) -> BotView<'_> {
        let nav = self.bot_nav.read().clone();
        let walked_ready = nav.walked_ready();
        BotView { map: self, nav, walked_ready }
    }

    /// Current bot navigation generation (mask + player trace).
    pub fn bot_nav(&self) -> Arc<BotNav> {
        self.bot_nav.read().clone()
    }

    /// Cells per axis of the grid bots navigate on (the client map's when a
    /// `.nav` is loaded; it can differ from the SMD, e.g. zone 73).
    pub fn bot_grid_size(&self) -> i32 {
        self.bot_nav.read().grid_size
    }

    /// Install (or clear) the client-derived mask. The player trace is kept
    /// when the grid size is unchanged, otherwise restarted.
    pub fn set_nav(&self, nav: Option<NavMask>) {
        let grid_size = nav.as_ref().map_or(self.smd.map_size, |n| n.size);
        let mut slot = self.bot_nav.write();
        let walked = if slot.grid_size == grid_size {
            slot.walked.clone()
        } else {
            Arc::new(WalkedMap::new(grid_size, self.smd.unit_dist))
        };
        *slot = Arc::new(BotNav { nav, walked, grid_size });
    }

    /// Replace the player trace (e.g. loaded from `Map/nav/<zone>.walked`).
    pub fn set_walked(&self, walked: WalkedMap) -> Result<(), WalkedMap> {
        let mut slot = self.bot_nav.write();
        if walked.size != slot.grid_size {
            return Err(walked);
        }
        *slot = Arc::new(BotNav { nav: slot.nav.clone(), walked: Arc::new(walked), grid_size: slot.grid_size });
        Ok(())
    }

    /// Record a validated real-player move into the trace.
    pub fn record_player_move(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        self.bot_nav.read().walked.mark_move(from, to)
    }

    /// Terrain height under `(x, z)`, computed exactly like the client does
    /// (`sub_8C7B40`: checkerboard triangulation, not bilinear), so the Y we
    /// send for bots is the Y the client would stand a player at. Uses the
    /// client terrain from the `.nav` file when loaded (the SMD terrain of
    /// zones 71/73 is a different map revision).
    pub fn terrain_height(&self, x: f32, z: f32) -> Option<f32> {
        if !self.is_valid_position(x, z) {
            return None;
        }
        let nav = self.bot_nav.read().clone();
        if let Some(mask) = nav.nav.as_ref().filter(|m| !m.heights.is_empty()) {
            return ko_protocol::smd::client_terrain_height(&mask.heights, mask.size, mask.unit, x, z);
        }
        if self.smd.heights.is_empty() {
            return None;
        }
        ko_protocol::smd::client_terrain_height(&self.smd.heights, self.smd.map_size, self.smd.unit_dist, x, z)
    }

    /// Would the real client refuse to walk a player from `(x, z)` in
    /// direction `(dir_x, dir_z)` because the terrain triangle is too steep
    /// (>40 degrees uphill, `sub_8C9210`)?
    pub fn client_incline_blocks(&self, x: f32, z: f32, dir_x: f32, dir_z: f32) -> bool {
        let nav = self.bot_nav.read().clone();
        if let Some(mask) = nav.nav.as_ref().filter(|m| !m.heights.is_empty()) {
            return ko_protocol::smd::client_incline_blocks(&mask.heights, mask.size, mask.unit, x, z, dir_x, dir_z);
        }
        !self.smd.heights.is_empty()
            && ko_protocol::smd::client_incline_blocks(
                &self.smd.heights,
                self.smd.map_size,
                self.smd.unit_dist,
                x,
                z,
                dir_x,
                dir_z,
            )
    }

    /// Event arena navigation: the SMD event grid and the authored openings of
    /// `zone_id` (`systems::event_nav`) are walkable in addition to the client
    /// mask (see `bot_event_only`). Used for zones 84 / 85 / 87.
    pub fn with_event_navigation(mut self, zone_id: u16) -> Self {
        self.bot_event_only = true;
        self.event_openings = crate::systems::event_nav::openings(zone_id);
        self
    }

    /// Create map data from a parsed SMD file.
    pub fn new(smd: SmdFile) -> Self {
        // Only trust the terrain mask when it exists, otherwise an empty-grid map would be fully open.
        let bot_ignore_event = !smd.bot_blocked.is_empty() && smd.event_grid.iter().all(|&v| v == 0);
        let bot_nav = BotNav {
            nav: None,
            walked: Arc::new(WalkedMap::new(smd.map_size, smd.unit_dist)),
            grid_size: smd.map_size,
        };
        Self {
            smd,
            bot_ignore_event,
            bot_event_only: false,
            event_openings: &[],
            bot_nav: parking_lot::RwLock::new(Arc::new(bot_nav)),
        }
    }

    /// Create map data with a client-derived mask already installed.
    pub fn with_nav(smd: SmdFile, nav: NavMask) -> Self {
        let map = Self::new(smd);
        map.set_nav(Some(nav));
        map
    }

    /// Check if a world position is within map boundaries.
    ///
    pub fn is_valid_position(&self, x: f32, z: f32) -> bool {
        self.smd.is_valid_position(x, z)
    }

    /// Check if a world position is on a walkable tile.
    ///
    pub fn is_movable(&self, x: f32, z: f32) -> bool {
        self.smd.is_movable_at(x, z)
    }

    /// Get the event ID at a world position.
    ///
    pub fn get_event_id_at(&self, x: f32, z: f32) -> i16 {
        self.smd.get_event_id_at(x, z)
    }

    /// Get a warp by ID.
    ///
    pub fn get_warp(&self, warp_id: i16) -> Option<&WarpInfo> {
        self.smd.warps.iter().find(|w| w.warp_id == warp_id)
    }

    /// Get all warps matching a warp group.
    ///
    pub fn get_warp_list(&self, warp_group: i32) -> Vec<&WarpInfo> {
        self.smd
            .warps
            .iter()
            .filter(|w| (w.warp_id as i32 / 10) == warp_group)
            .collect()
    }

    /// Map width in world meters.
    pub fn map_width(&self) -> f32 {
        self.smd.map_width
    }

    /// Map height in world meters.
    pub fn map_height(&self) -> f32 {
        self.smd.map_height
    }

    /// Grid unit distance in world meters (TILE_SIZE, typically 4.0).
    ///
    pub fn unit_dist(&self) -> f32 {
        self.smd.unit_dist
    }

    /// Grid size (cells per axis). Same as SMD map_size.
    pub fn grid_size(&self) -> i32 {
        self.smd.map_size
    }

    /// Check if a grid cell is walkable (event_id == 0).
    ///
    /// Takes grid indices, not world coordinates.
    pub fn is_movable_grid(&self, gx: i32, gz: i32) -> bool {
        self.smd.is_movable(gx, gz)
    }
}

// ─── Zone State ─────────────────────────────────────────────────────────

/// Per-zone state including region grid, map data, events, and zone rules.
pub struct ZoneState {
    /// Zone/map identifier.
    pub zone_id: u16,
    /// Maximum region X index (exclusive).
    pub max_region_x: u16,
    /// Maximum region Z index (exclusive).
    pub max_region_z: u16,
    /// 2D grid of regions: `regions[x][z]`.
    regions: Vec<Vec<Region>>,

    /// Zone configuration from the database (None for legacy/test zones).
    pub zone_info: Option<ZoneInfo>,
    /// Map data from the SMD file (None if file not found).
    pub map_data: Option<MapData>,
    /// Events keyed by event_num (from game_event table).
    pub events: HashMap<i16, GameEvent>,
    /// Interactive object events keyed by `s_index` (the in-zone object identifier).
    ///
    /// C++ uses `GetData(objectindex)` which searches by `s_index`, NOT by array position.
    pub object_events: HashMap<i16, ObjectEventInfo>,
}

/// A single cell in the region grid.
/// Tracks which sessions (players) and NPCs are currently in this region.
pub struct Region {
    /// Set of session IDs in this region.
    pub users: RwLock<HashSet<SessionId>>,
    /// Set of NPC instance IDs in this region.
    pub npcs: RwLock<HashSet<NpcId>>,
}

impl Region {
    /// Create a new empty region.
    fn new() -> Self {
        Self {
            users: RwLock::new(HashSet::new()),
            npcs: RwLock::new(HashSet::new()),
        }
    }
}

/// Calculate region coordinate from world position.
pub fn calc_region(pos: f32) -> u16 {
    (pos as u16) / VIEW_DISTANCE
}

/// Build a region grid for the given map dimensions.
fn build_region_grid(map_size: u16) -> (u16, Vec<Vec<Region>>) {
    let grid_size = (map_size / VIEW_DISTANCE) + 1;
    let mut regions = Vec::with_capacity(grid_size as usize);
    for _ in 0..grid_size {
        let mut col = Vec::with_capacity(grid_size as usize);
        for _ in 0..grid_size {
            col.push(Region::new());
        }
        regions.push(col);
    }
    (grid_size, regions)
}

impl ZoneState {
    /// Create a new zone with a region grid sized for the given map dimensions.
    ///
    /// Grid size = `map_size / VIEW_DISTANCE + 1` per axis.
    /// This is the simple constructor — no map data or zone info.
    pub fn new(zone_id: u16, map_size: u16) -> Self {
        let (grid_size, regions) = build_region_grid(map_size);
        Self {
            zone_id,
            max_region_x: grid_size,
            max_region_z: grid_size,
            regions,
            zone_info: None,
            map_data: None,
            events: HashMap::new(),
            object_events: HashMap::new(),
        }
    }

    /// Create a fully-configured zone with map data, events, and zone info.
    ///
    pub fn new_with_data(
        zone_id: u16,
        zone_info: ZoneInfo,
        map_data: Option<MapData>,
        events: HashMap<i16, GameEvent>,
        object_events: HashMap<i16, ObjectEventInfo>,
    ) -> Self {
        // Determine map_size for region grid from SMD if available, else default
        let map_size = match &map_data {
            Some(md) => md.map_width() as u16,
            None => 1024, // reasonable default
        };

        let (grid_size, regions) = build_region_grid(map_size);
        Self {
            zone_id,
            max_region_x: grid_size,
            max_region_z: grid_size,
            regions,
            zone_info: Some(zone_info),
            map_data,
            events,
            object_events,
        }
    }

    /// Look up an object event by its `s_index` (in-zone object identifier).
    ///
    /// NOT by array position.
    pub fn get_object_event(&self, index: u16) -> Option<&ObjectEventInfo> {
        self.object_events.get(&(index as i16))
    }

    // ─── Movement Validation ────────────────────────────────────────

    /// Check if a world position is within map boundaries.
    ///
    /// Returns true if no map data is loaded (permissive fallback).
    ///
    pub fn is_valid_position(&self, x: f32, z: f32) -> bool {
        match &self.map_data {
            Some(md) => md.is_valid_position(x, z),
            None => true, // No map data → allow (permissive)
        }
    }

    /// Check if a world position is on a walkable tile.
    ///
    /// Returns true if no map data is loaded (permissive fallback).
    ///
    pub fn is_movable(&self, x: f32, z: f32) -> bool {
        match &self.map_data {
            Some(md) => md.is_movable(x, z),
            None => true,
        }
    }

    /// Look up a game event triggered by stepping on a tile.
    ///
    /// Returns `None` if no event or no map data.
    ///
    pub fn check_event(&self, x: f32, z: f32) -> Option<&GameEvent> {
        let event_id = match &self.map_data {
            Some(md) => md.get_event_id_at(x, z),
            None => return None,
        };

        // Event IDs < 2 are not real events (0=walkable, 1=blocked)
        if event_id < 2 {
            return None;
        }

        self.events.get(&event_id)
    }

    // ─── Zone Rule Helpers ──────────────────────────────────────────

    /// Can players attack the other nation in this zone?
    pub fn can_attack_other_nation(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.attack_other_nation)
            .unwrap_or(false)
    }

    /// Can players attack same nation in this zone?
    pub fn can_attack_same_nation(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.attack_same_nation)
            .unwrap_or(false)
    }

    /// Is this a war zone?
    pub fn is_war_zone(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.war_zone)
            .unwrap_or(false)
    }

    /// Can players trade with the other nation in this zone?
    ///
    pub fn can_trade_other_nation(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.trade_other_nation)
            .unwrap_or(false)
    }

    /// Can players talk to the other nation in this zone?
    ///
    pub fn can_talk_other_nation(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.talk_other_nation)
            .unwrap_or(false)
    }

    /// Get the zone ability type.
    pub fn zone_type(&self) -> ZoneAbilityType {
        self.zone_info
            .as_ref()
            .map(|zi| zi.zone_type)
            .unwrap_or(ZoneAbilityType::Neutral)
    }

    /// Check if teleport is allowed.
    pub fn can_teleport(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.teleport)
            .unwrap_or(true)
    }

    /// Check if gate use is allowed.
    pub fn can_use_gate(&self) -> bool {
        self.zone_info
            .as_ref()
            .map(|zi| zi.abilities.gate)
            .unwrap_or(true)
    }

    /// Get spawn position for this zone.
    pub fn spawn_position(&self) -> (f32, f32, f32) {
        self.zone_info
            .as_ref()
            .map(|zi| (zi.init_x, zi.init_z, zi.init_y))
            .unwrap_or((0.0, 0.0, 0.0))
    }

    /// Get level restrictions for this zone.
    pub fn level_range(&self) -> (u8, u8) {
        self.zone_info
            .as_ref()
            .map(|zi| (zi.min_level, zi.max_level))
            .unwrap_or((1, 83))
    }

    // ─── Region Grid Methods ────────────────────────────────────────

    /// Get a reference to a region cell, or `None` if out of bounds.
    pub fn get_region(&self, rx: u16, rz: u16) -> Option<&Region> {
        if rx >= self.max_region_x || rz >= self.max_region_z {
            return None;
        }
        Some(&self.regions[rx as usize][rz as usize])
    }

    /// Add a user to a region.
    pub fn add_user(&self, rx: u16, rz: u16, id: SessionId) {
        if let Some(region) = self.get_region(rx, rz) {
            region.users.write().insert(id);
        }
    }

    /// Remove a user from a region.
    pub fn remove_user(&self, rx: u16, rz: u16, id: SessionId) {
        if let Some(region) = self.get_region(rx, rz) {
            region.users.write().remove(&id);
        }
    }

    /// Get a snapshot of all user IDs in a region.
    pub fn get_users_in_region(&self, rx: u16, rz: u16) -> Vec<SessionId> {
        if let Some(region) = self.get_region(rx, rz) {
            region.users.read().iter().copied().collect()
        } else {
            Vec::new()
        }
    }

    /// Get all user IDs in a 3×3 region grid centered on (rx, rz).
    ///
    /// Returns a `SmallVec` to avoid heap allocation for typical region sizes
    /// (≤64 players in a 3×3 area). Each player is in exactly one region cell,
    /// so no deduplication is needed.
    ///
    pub fn get_users_in_3x3(&self, rx: u16, rz: u16) -> SmallVec<[SessionId; 64]> {
        let mut result = SmallVec::new();
        for dx in -1i16..=1 {
            for dz in -1i16..=1 {
                let nx = rx as i16 + dx;
                let nz = rz as i16 + dz;
                if nx >= 0 && nz >= 0 {
                    if let Some(region) = self.get_region(nx as u16, nz as u16) {
                        let guard = region.users.read();
                        result.extend(guard.iter().copied());
                    }
                }
            }
        }
        result
    }

    /// Lightweight check: are there ANY users in the 3×3 region grid around (rx, rz)?
    ///
    /// `m_byMoving == 1` on the NPC's region.  We extend to 3×3 to match
    /// the actual search area of `find_enemy()`.
    ///
    /// This is much cheaper than `get_users_in_3x3()` because it returns
    /// as soon as ANY user is found (no allocation, no iteration).
    pub fn region_3x3_has_users(&self, rx: u16, rz: u16) -> bool {
        for dx in -1i16..=1 {
            for dz in -1i16..=1 {
                let nx = rx as i16 + dx;
                let nz = rz as i16 + dz;
                if nx >= 0 && nz >= 0 {
                    if let Some(region) = self.get_region(nx as u16, nz as u16) {
                        if !region.users.read().is_empty() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    // ─── NPC Region Methods ─────────────────────────────────────────

    /// Add an NPC to a region.
    pub fn add_npc(&self, rx: u16, rz: u16, nid: NpcId) {
        if let Some(region) = self.get_region(rx, rz) {
            region.npcs.write().insert(nid);
        }
    }

    /// Remove an NPC from a region.
    pub fn remove_npc(&self, rx: u16, rz: u16, nid: NpcId) {
        if let Some(region) = self.get_region(rx, rz) {
            region.npcs.write().remove(&nid);
        }
    }

    /// Get a snapshot of all NPC IDs in a region.
    pub fn get_npcs_in_region(&self, rx: u16, rz: u16) -> Vec<NpcId> {
        if let Some(region) = self.get_region(rx, rz) {
            region.npcs.read().iter().copied().collect()
        } else {
            Vec::new()
        }
    }

    /// Get all NPC IDs in a 3×3 region grid centered on (rx, rz).
    ///
    /// Returns a `SmallVec` to avoid heap allocation for typical region sizes
    /// (≤64 NPCs in a 3×3 area). Each NPC is in exactly one region cell,
    /// so no deduplication is needed.
    pub fn get_npcs_in_3x3(&self, rx: u16, rz: u16) -> SmallVec<[NpcId; 64]> {
        let mut result = SmallVec::new();
        for dx in -1i16..=1 {
            for dz in -1i16..=1 {
                let nx = rx as i16 + dx;
                let nz = rz as i16 + dz;
                if nx >= 0 && nz >= 0 {
                    if let Some(region) = self.get_region(nx as u16, nz as u16) {
                        let guard = region.npcs.read();
                        result.extend(guard.iter().copied());
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_region_boundaries() {
        assert_eq!(calc_region(0.0), 0);
        assert_eq!(calc_region(47.0), 0);
        assert_eq!(calc_region(48.0), 1);
        assert_eq!(calc_region(95.0), 1);
        assert_eq!(calc_region(96.0), 2);
        assert_eq!(calc_region(4095.0), 85);
    }

    #[test]
    fn test_zone_grid_size() {
        let zone = ZoneState::new(21, 4096);
        // 4096 / 48 + 1 = 86
        assert_eq!(zone.max_region_x, 86);
        assert_eq!(zone.max_region_z, 86);
    }

    #[test]
    fn test_zone_get_region_bounds() {
        let zone = ZoneState::new(21, 4096);
        assert!(zone.get_region(0, 0).is_some());
        assert!(zone.get_region(85, 85).is_some());
        assert!(zone.get_region(86, 0).is_none());
        assert!(zone.get_region(0, 86).is_none());
    }

    #[test]
    fn test_add_remove_user() {
        let zone = ZoneState::new(21, 4096);
        zone.add_user(5, 5, 100);
        zone.add_user(5, 5, 200);

        let users = zone.get_users_in_region(5, 5);
        assert!(users.contains(&100));
        assert!(users.contains(&200));
        assert_eq!(users.len(), 2);

        zone.remove_user(5, 5, 100);
        let users = zone.get_users_in_region(5, 5);
        assert!(!users.contains(&100));
        assert!(users.contains(&200));
    }

    #[test]
    fn test_get_users_in_3x3() {
        let zone = ZoneState::new(21, 4096);
        zone.add_user(5, 5, 1);
        zone.add_user(4, 4, 2); // diagonal neighbor
        zone.add_user(6, 6, 3); // diagonal neighbor
        zone.add_user(5, 7, 4); // outside 3x3

        let nearby = zone.get_users_in_3x3(5, 5);
        assert!(nearby.contains(&1));
        assert!(nearby.contains(&2));
        assert!(nearby.contains(&3));
        assert!(!nearby.contains(&4)); // (5,7) is outside 3x3 of (5,5)
    }

    #[test]
    fn test_3x3_at_origin() {
        let zone = ZoneState::new(21, 4096);
        zone.add_user(0, 0, 1);
        zone.add_user(1, 1, 2);

        // At (0,0), negative regions are skipped
        let nearby = zone.get_users_in_3x3(0, 0);
        assert!(nearby.contains(&1));
        assert!(nearby.contains(&2));
    }

    #[test]
    fn test_zone_ability_type_from_i16() {
        assert_eq!(ZoneAbilityType::from_i16(0), ZoneAbilityType::Neutral);
        assert_eq!(ZoneAbilityType::from_i16(1), ZoneAbilityType::PvP);
        assert_eq!(
            ZoneAbilityType::from_i16(7),
            ZoneAbilityType::CaitharosArena
        );
        assert_eq!(ZoneAbilityType::from_i16(99), ZoneAbilityType::Neutral);
    }

    #[test]
    fn test_game_event_type_from_i16() {
        assert_eq!(GameEventType::from_i16(1), Some(GameEventType::ZoneChange));
        assert_eq!(GameEventType::from_i16(2), Some(GameEventType::TrapDead));
        assert_eq!(GameEventType::from_i16(3), Some(GameEventType::TrapArea));
        assert_eq!(GameEventType::from_i16(0), None);
        assert_eq!(GameEventType::from_i16(4), None);
    }

    #[test]
    fn test_zone_defaults_without_data() {
        let zone = ZoneState::new(21, 1024);
        // No zone_info → permissive defaults
        assert!(zone.is_valid_position(100.0, 100.0));
        assert!(zone.is_movable(100.0, 100.0));
        assert!(zone.check_event(100.0, 100.0).is_none());
        assert!(!zone.can_attack_other_nation());
        assert!(!zone.is_war_zone());
        assert!(zone.can_teleport());
        assert_eq!(zone.zone_type(), ZoneAbilityType::Neutral);
        assert_eq!(zone.level_range(), (1, 83));
    }

    #[test]
    fn test_add_remove_npc() {
        let zone = ZoneState::new(21, 4096);
        zone.add_npc(5, 5, 10001);
        zone.add_npc(5, 5, 10002);

        let npcs = zone.get_npcs_in_region(5, 5);
        assert!(npcs.contains(&10001));
        assert!(npcs.contains(&10002));
        assert_eq!(npcs.len(), 2);

        zone.remove_npc(5, 5, 10001);
        let npcs = zone.get_npcs_in_region(5, 5);
        assert!(!npcs.contains(&10001));
        assert!(npcs.contains(&10002));
    }

    #[test]
    fn test_get_npcs_in_3x3() {
        let zone = ZoneState::new(21, 4096);
        zone.add_npc(5, 5, 10001);
        zone.add_npc(4, 4, 10002);
        zone.add_npc(6, 6, 10003);
        zone.add_npc(5, 7, 10004); // outside 3x3

        let nearby = zone.get_npcs_in_3x3(5, 5);
        assert!(nearby.contains(&10001));
        assert!(nearby.contains(&10002));
        assert!(nearby.contains(&10003));
        assert!(!nearby.contains(&10004));
    }

    #[test]
    fn test_zone_with_events() {
        let mut events = HashMap::new();
        events.insert(
            5,
            GameEvent {
                event_type: GameEventType::ZoneChange,
                cond: [0; 5],
                exec: [21, 500, 500, 0, 0],
            },
        );

        let zone_info = ZoneInfo {
            smd_name: "test.smd".to_string(),
            zone_name: "Test Zone".to_string(),
            zone_type: ZoneAbilityType::PvP,
            min_level: 35,
            max_level: 83,
            init_x: 100.0,
            init_z: 200.0,
            init_y: 0.0,
            status: 1,
            abilities: ZoneAbilities {
                attack_other_nation: true,
                war_zone: true,
                teleport: false,
                ..Default::default()
            },
        };

        let zone = ZoneState::new_with_data(1, zone_info, None, events, HashMap::new());

        assert!(zone.can_attack_other_nation());
        assert!(zone.is_war_zone());
        assert!(!zone.can_teleport());
        assert_eq!(zone.zone_type(), ZoneAbilityType::PvP);
        assert_eq!(zone.spawn_position(), (100.0, 200.0, 0.0));
        assert_eq!(zone.level_range(), (35, 83));
    }

    // ── Sprint 952: Additional coverage ──────────────────────────────

    /// VIEW_DISTANCE is 48 world units.
    #[test]
    fn test_view_distance_constant() {
        assert_eq!(VIEW_DISTANCE, 48);
    }

    /// calc_region converts world position to region index.
    #[test]
    fn test_calc_region_basic() {
        assert_eq!(calc_region(0.0), 0);
        assert_eq!(calc_region(47.0), 0); // still in region 0
        assert_eq!(calc_region(48.0), 1); // region 1
        assert_eq!(calc_region(96.0), 2); // region 2
        assert_eq!(calc_region(500.0), 500 / 48);
    }

    /// ZoneAbilityType from_i16 covers all variants including unknown.
    #[test]
    fn test_zone_ability_type_from_i16_full() {
        assert_eq!(ZoneAbilityType::from_i16(0), ZoneAbilityType::Neutral);
        assert_eq!(ZoneAbilityType::from_i16(1), ZoneAbilityType::PvP);
        assert_eq!(ZoneAbilityType::from_i16(2), ZoneAbilityType::Spectator);
        assert_eq!(
            ZoneAbilityType::from_i16(7),
            ZoneAbilityType::CaitharosArena
        );
        assert_eq!(ZoneAbilityType::from_i16(99), ZoneAbilityType::Neutral); // unknown defaults
    }

    /// ZoneAbilities default has all flags false.
    #[test]
    fn test_zone_abilities_default() {
        let ab = ZoneAbilities::default();
        assert!(!ab.attack_other_nation);
        assert!(!ab.war_zone);
        assert!(!ab.teleport);
        assert!(!ab.exp_lost);
        assert!(!ab.pet_spawn);
    }

    /// ZoneAbilityType repr u8 values.
    #[test]
    fn test_zone_ability_type_repr() {
        assert_eq!(ZoneAbilityType::Neutral as u8, 0);
        assert_eq!(ZoneAbilityType::PvP as u8, 1);
        assert_eq!(ZoneAbilityType::Siege1 as u8, 3);
        assert_eq!(ZoneAbilityType::PvpStoneNpcs as u8, 9);
    }
}
