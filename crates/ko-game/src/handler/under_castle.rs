//! Under The Castle (Castellan Dungeon) event handler.
//! ## Event Lifecycle
//! 1. **Activation**: Event scheduler sets `is_active = true`, `is_summon = false`,
//!    `start_time = 180 * 60` (3 hours countdown).
//! 2. **Monster Spawn**: On next timer tick with `is_summon == false`,
//!    all entries from `monster_under_the_castle` are spawned via `SpawnEventNpc`.
//!    Sets `is_summon = true`.
//! 3. **Movie Trigger**: When `start_time == start_move_time`, a WIZ_UTC_MOVIE
//!    packet is broadcast to the zone (cutscene trigger).
//! 4. **Event End**: When `start_time == 0`, all monsters are killed,
//!    players are kicked to Moradon, state is reset.
//! ## Entry Validation
//! Only CSW-winning clan members (and their allies) may enter zone 86.
//! Minimum level: 70.
//! ## Monster Death Processing
//! Each boss death triggers a WIZ_UTC_MOVIE packet with a specific movie_id.
//! Some bosses also trigger rewards and gate openings.
//! | Proto ID | Name                       | Movie ID | Gate | Reward |
//! |----------|----------------------------|----------|------|--------|
//! | 9501     | Emperor Mammoth I          | 7        |      |        |
//! | 9502     | Emperor Mammoth II         | 8        |      |        |
//! | 9503     | Emperor Mammoth III        | 2        |      | room 1 |
//! | 9504-9506| Creshergimmic I-III        | 6        |      |        |
//! | 9507     | Creshergimmic VI           | 3        | 0    | room 2 |
//! | 9508-9510| Purious I-III              | 6        |      |        |
//! | 9511     | Purious VI                 | 4        | 1    | room 3 |
//! | 9512-9513| Fluwiton Room 3 I-II       | 6        |      |        |
//! | 9514     | Fluwiton Room 3 III        | 5        | 2    | room 4 |
//! | 9515-9517| Fluwiton Room 4 I-III      | 6        |      |        |
//! | 9518     | Fluwiton Room 4 VI         | --       |      | room 5 |

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use ko_protocol::Packet;

pub use crate::world::types::ZONE_UNDER_CASTLE;

/// WIZ_UTC_MOVIE opcode value (0x92).
pub const WIZ_UTC_MOVIE: u8 = 0x92;

/// Minimum level to enter Under The Castle.
pub const MIN_LEVEL_UNDER_CASTLE: u8 = 70;

/// NPC type for fast-spawn UTC monsters.
pub const NPC_UTC_SPAWN_FAST: u16 = 214;

// ── Monster Proto IDs ──────────────────────────────────────────────────

pub const MONSTER_EMPEROR_MAMMOTH_I: u16 = 9501;
pub const MONSTER_EMPEROR_MAMMOTH_II: u16 = 9502;
pub const MONSTER_EMPEROR_MAMMOTH_III: u16 = 9503;
pub const MONSTER_CRESHERGIMMIC_I: u16 = 9504;
pub const MONSTER_CRESHERGIMMIC_II: u16 = 9505;
pub const MONSTER_CRESHERGIMMIC_III: u16 = 9506;
pub const MONSTER_CRESHERGIMMIC_VI: u16 = 9507;
pub const MONSTER_PURIOUS_I: u16 = 9508;
pub const MONSTER_PURIOUS_II: u16 = 9509;
pub const MONSTER_PURIOUS_III: u16 = 9510;
pub const MONSTER_PURIOUS_VI: u16 = 9511;
pub const MONSTER_FLUWITON_ROOM_3_I: u16 = 9512;
pub const MONSTER_FLUWITON_ROOM_3_II: u16 = 9513;
pub const MONSTER_FLUWITON_ROOM_3_III: u16 = 9514;
pub const MONSTER_FLUWITON_ROOM_4_I: u16 = 9515;
pub const MONSTER_FLUWITON_ROOM_4_II: u16 = 9516;
pub const MONSTER_FLUWITON_ROOM_4_III: u16 = 9517;
pub const MONSTER_FLUWITON_ROOM_4_VI: u16 = 9518;

/// NPC ID for the exit portal spawned after final boss death.
pub const UTC_EXIT_PORTAL_NPC: u16 = 29197;

// ── Internal UTC transition gate ─────────────────────────────────────

/// The final internal transition is a map trigger, not an NPC interaction.
///
/// Reference: `reference_cpp/GameServer/EventTrapSystem.cpp`,
/// `CUser::EventTrapProcess()` for `ZONE_UNDER_CASTLE`.
/// Entering this five-unit radius sends the player to the separate final-boss
/// area in the same `bossmode.smd` map via `WIZ_WARP`.
pub const UTC_FINAL_GATE_X: f32 = 646.0;
pub const UTC_FINAL_GATE_Z: f32 = 236.0;
pub const UTC_FINAL_GATE_RADIUS: f32 = 5.0;
pub const UTC_FINAL_BOSS_WARP_X: f32 = 824.0;
pub const UTC_FINAL_BOSS_WARP_Z: f32 = 932.0;

// ── UTC Reward Item IDs ──────────────────────────────────────────────

/// Trophy of Flame — awarded in every room.
pub const TROPHY_OF_FLAME: u32 = 800_149_000;

/// Dented Ironmass — room 2 bonus reward.
pub const DENTED_IRONMASS: u32 = 508_147_000;

/// Petrified Weapon Shrapnel — room 3 bonus reward.
pub const PETRIFIED_WEAPON_SHRAPNEL: u32 = 508_149_000;

/// Iron Powder of Chain — room 4 bonus reward.
pub const IRON_POWDER_OF_CHAIN: u32 = 508_151_000;

/// Plwitoon's Tear — room 5 bonus reward.
pub const PLWITOONS_TEAR: u32 = 508_152_000;

/// Horn of Pluwitoon — room 5 bonus reward.
pub const HORN_OF_PLUWITOON: u32 = 810_479_000;

// ── UTC Room Geometry ──────────────────────────────────────────────────

/// Per-room center coordinates and range for determining nearby players.
/// Each entry is `(center_x, center_z, range)` used by `isInRangeSlow()`.
/// The boss kill position is always checked with range 15.
/// Room 1 & 2 share the same center: (121.0, 297.0, 80).
/// Room 3: (520.0, 494.0, 115).
/// Room 4: (642.0, 351.0, 100).
/// Room 5: (803.0, 839.0, 110).
pub fn get_room_center(room: u8) -> Option<(f32, f32, f32)> {
    match room {
        1 | 2 => Some((121.0, 297.0, 80.0)),
        3 => Some((520.0, 494.0, 115.0)),
        4 => Some((642.0, 351.0, 100.0)),
        5 => Some((803.0, 839.0, 110.0)),
        _ => None,
    }
}

/// Boss kill proximity range — players within 15 units of the boss death position.
pub const UTC_BOSS_KILL_RANGE: f32 = 15.0;

/// Calculate Euclidean distance between two 2D points.
fn distance_2d(x1: f32, z1: f32, x2: f32, z2: f32) -> f32 {
    ((x1 - x2) * (x1 - x2) + (z1 - z2) * (z1 - z2)).sqrt()
}

/// Check if a point is within range of a center (XZ plane).
pub fn is_in_range_slow(px: f32, pz: f32, cx: f32, cz: f32, range: f32) -> bool {
    distance_2d(px, pz, cx, cz) <= range
}

/// Whether a UTC movement position has reached the final-boss transition gate.
pub fn is_at_final_boss_transition_gate(x: f32, z: f32) -> bool {
    is_in_range_slow(
        x,
        z,
        UTC_FINAL_GATE_X,
        UTC_FINAL_GATE_Z,
        UTC_FINAL_GATE_RADIUS,
    )
}

/// Determine the reward items for a UTC room.
/// Returns: `(trophy_count, bonus_items)` where:
/// - `trophy_count`: how many TROPHY_OF_FLAME to give (1 or 2 based on proximity).
/// - `bonus_items`: additional room-specific items (each given with count=1).
/// # Logic per room:
/// - **Room 1**: Trophy of Flame only (1 or 2 based on proximity).
/// - **Room 2**: Trophy of Flame (1 or 2) + Dented Ironmass.
/// - **Room 3**: Trophy of Flame (1 or 2) + Petrified Weapon Shrapnel.
/// - **Room 4**: Trophy of Flame (1 or 2) + Iron Powder of Chain.
/// - **Room 5**: Trophy of Flame (1 or 2) + Plwitoon's Tear + Horn of Pluwitoon.
/// The trophy count is 2 if the player is in BOTH the room center range AND
/// the boss kill range, or 1 if in either one (but not both).
pub fn get_utc_room_reward_items(room: u8) -> Vec<u32> {
    match room {
        1 => vec![],                                  // Trophy only (no bonus)
        2 => vec![DENTED_IRONMASS],                   // + Dented Ironmass
        3 => vec![PETRIFIED_WEAPON_SHRAPNEL],         // + Petrified Weapon Shrapnel
        4 => vec![IRON_POWDER_OF_CHAIN],              // + Iron Powder of Chain
        5 => vec![PLWITOONS_TEAR, HORN_OF_PLUWITOON], // + 2 bonus items
        _ => vec![],
    }
}

/// Calculate trophy count based on player proximity to room center and boss kill position.
/// - `isInRangeSlow(room_center_x, room_center_z, room_range)` AND `isInRangeSlow(GetX, GetZ, 15)` -> 2 trophies
/// - `isInRangeSlow(room_center_x, room_center_z, room_range)` OR `isInRangeSlow(GetX, GetZ, 15)` -> 1 trophy
/// - Neither -> 0 (not eligible for any reward)
/// Returns 0, 1, or 2.
pub fn calculate_trophy_count(
    player_x: f32,
    player_z: f32,
    room_center_x: f32,
    room_center_z: f32,
    room_range: f32,
    boss_x: f32,
    boss_z: f32,
) -> u16 {
    let in_room = is_in_range_slow(player_x, player_z, room_center_x, room_center_z, room_range);
    let near_boss = is_in_range_slow(player_x, player_z, boss_x, boss_z, UTC_BOSS_KILL_RANGE);

    if in_room && near_boss {
        2
    } else if in_room || near_boss {
        1
    } else {
        0
    }
}

// ── In-memory spawn entry ──────────────────────────────────────────────

/// In-memory spawn entry loaded from DB at startup.
#[derive(Debug, Clone)]
pub struct UtcSpawnEntry {
    /// Primary key index.
    pub s_index: i16,
    /// NPC template ID (s_sid).
    pub s_sid: i16,
    /// Monster name.
    pub name: String,
    /// 0 = monster, 1 = NPC.
    pub b_type: i16,
    /// Trap/door number for gate mechanics.
    pub trap_number: i16,
    /// X coordinate.
    pub x: i16,
    /// Y coordinate.
    pub y: i16,
    /// Z coordinate.
    pub z: i16,
    /// Spawn direction.
    pub direction: i16,
    /// Spawn count.
    pub count: i16,
    /// Spawn radius.
    pub radius: i16,
}

// ── Under The Castle state ─────────────────────────────────────────────

/// Under The Castle runtime event state.
pub struct UnderTheCastleState {
    /// Whether the event is currently active.
    pub is_active: AtomicBool,
    /// Whether monsters have been summoned for this run.
    pub is_summon: AtomicBool,
    /// Countdown timer in seconds (starts at 180*60 = 10800).
    /// Decremented each tick. Event ends when reaching 0.
    pub start_time: AtomicU32,
    /// Time value at which the movie cutscene triggers.
    /// C++ sets this to `start_time - 15` on activation.
    pub start_move_time: AtomicU32,
    /// Minimum player level.
    pub min_level: std::sync::atomic::AtomicU8,
    /// Maximum player level.
    pub max_level: std::sync::atomic::AtomicU8,
    /// Tracked spawned monster NPC IDs (proto_id -> npc_runtime_id).
    /// Used for cleanup on event end.
    pub monster_list: RwLock<Vec<u32>>,
    /// Gate NPC runtime IDs: one or more physical door pieces for each stage.
    /// UTC's source table contains two pieces for the third gate, and C++ kills
    /// the doors on boss death rather than merely toggling an object flag.
    ///
    pub gate_ids: RwLock<[Vec<u32>; 3]>,
}

impl UnderTheCastleState {
    /// Create a new default (inactive) UTC state.
    pub fn new() -> Self {
        Self {
            is_active: AtomicBool::new(false),
            is_summon: AtomicBool::new(false),
            start_time: AtomicU32::new(0),
            start_move_time: AtomicU32::new(0),
            min_level: std::sync::atomic::AtomicU8::new(0),
            max_level: std::sync::atomic::AtomicU8::new(0),
            monster_list: RwLock::new(Vec::new()),
            gate_ids: RwLock::new(std::array::from_fn(|_| Vec::new())),
        }
    }

    /// Reset the UTC state (called on event end).
    ///
    pub fn reset(&self) {
        self.is_active.store(false, Ordering::Relaxed);
        self.is_summon.store(false, Ordering::Relaxed);
        self.start_time.store(0, Ordering::Relaxed);
        self.start_move_time.store(0, Ordering::Relaxed);
        self.min_level.store(0, Ordering::Relaxed);
        self.max_level.store(0, Ordering::Relaxed);
        {
            let mut list = self.monster_list.write();
            list.clear();
        }
        {
            let mut gates = self.gate_ids.write();
            *gates = std::array::from_fn(|_| Vec::new());
        }
    }
}

impl Default for UnderTheCastleState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Event activation ───────────────────────────────────────────────────

/// Activate the Under The Castle event.
/// Sets `start_time = duration_minutes * 60`, `start_move_time = start_time - 15`,
/// marks as active, clears summon flag.
pub fn activate_event(
    state: &UnderTheCastleState,
    duration_minutes: u32,
    min_level: u8,
    max_level: u8,
) -> bool {
    if state.is_active.load(Ordering::Relaxed) {
        return false;
    }
    if duration_minutes == 0 {
        return false;
    }

    state.reset();
    let total_seconds = duration_minutes * 60;
    state.start_time.store(total_seconds, Ordering::Relaxed);
    state
        .start_move_time
        .store(total_seconds.saturating_sub(15), Ordering::Relaxed);
    state.min_level.store(min_level, Ordering::Relaxed);
    state.max_level.store(max_level, Ordering::Relaxed);
    state.is_active.store(true, Ordering::Relaxed);
    state.is_summon.store(false, Ordering::Relaxed);
    true
}

/// Forcibly stop the Under The Castle event.
pub fn force_stop_event(state: &UnderTheCastleState) {
    if !state.is_active.load(Ordering::Relaxed) {
        return;
    }
    // Set time very low to trigger end on next tick
    state.start_time.store(1, Ordering::Relaxed);
}

// ── Timer tick ─────────────────────────────────────────────────────────

/// Result of a single timer tick.
#[derive(Debug, PartialEq, Eq)]
pub enum UtcTickResult {
    /// No action needed (event not active).
    Idle,
    /// Monsters should be spawned (first tick after activation).
    SpawnMonsters,
    /// The movie cutscene should be triggered (WIZ_UTC_MOVIE broadcast).
    TriggerMovie,
    /// Event has ended. Kill all monsters and kick users.
    Finish,
    /// Normal tick — decrement timer, nothing special.
    Tick,
}

/// Run one tick of the Under The Castle timer.
/// This should be called every second while the event is active.
/// Also handles the 1-second decrement of `start_time` via
/// `SingleOtherEventLocalTimer()` (EventMainTimer.cpp:322-323).
pub fn timer_tick(state: &UnderTheCastleState) -> UtcTickResult {
    if !state.is_active.load(Ordering::Relaxed) {
        return UtcTickResult::Idle;
    }

    // Phase 1: Spawn monsters if not yet summoned
    if !state.is_summon.load(Ordering::Relaxed) {
        state.is_summon.store(true, Ordering::Relaxed);
        return UtcTickResult::SpawnMonsters;
    }

    let current_time = state.start_time.load(Ordering::Relaxed);
    let move_time = state.start_move_time.load(Ordering::Relaxed);

    // Phase 2: Check for movie trigger
    if current_time == move_time && move_time > 0 {
        // Decrement and return movie trigger
        state
            .start_time
            .store(current_time.saturating_sub(1), Ordering::Relaxed);
        return UtcTickResult::TriggerMovie;
    }

    // Phase 3: Check for event end
    if current_time == 0 {
        return UtcTickResult::Finish;
    }

    // Normal tick: decrement timer
    state
        .start_time
        .store(current_time.saturating_sub(1), Ordering::Relaxed);
    UtcTickResult::Tick
}

// ── Entry validation ───────────────────────────────────────────────────

/// Check if a player can enter the Under The Castle zone.
/// Requirements:
/// - Event must be active
/// - Player level must be >= MIN_LEVEL_UNDER_CASTLE (70)
/// - Player must belong to the CSW-winning clan (or allied clan)
pub fn can_player_enter(
    is_active: bool,
    player_level: u8,
    player_clan_id: u16,
    is_csw_winner: bool,
) -> UtcEntryResult {
    if !is_active {
        return UtcEntryResult::EventNotActive;
    }
    if player_level < MIN_LEVEL_UNDER_CASTLE {
        return UtcEntryResult::LevelTooLow;
    }
    if player_clan_id == 0 {
        return UtcEntryResult::NoClan;
    }
    if !is_csw_winner {
        return UtcEntryResult::NotCswWinner;
    }
    UtcEntryResult::Allowed
}

/// Result of entry validation.
#[derive(Debug, PartialEq, Eq)]
pub enum UtcEntryResult {
    /// Player can enter.
    Allowed,
    /// Event is not active.
    EventNotActive,
    /// Player level is too low.
    LevelTooLow,
    /// Player has no clan.
    NoClan,
    /// Player's clan is not the CSW winner (or allied).
    NotCswWinner,
}

// ── Monster death processing ───────────────────────────────────────────

/// Result of processing a monster death in UTC.
#[derive(Debug, PartialEq, Eq)]
pub struct UtcMonsterDeathResult {
    /// Movie ID to broadcast via WIZ_UTC_MOVIE, or 0 if none.
    pub movie_id: u32,
    /// Reward room number (1-5) to send items to nearby users, or 0 if none.
    pub reward_room: u8,
    /// Gate index to open (0-2), or None if no gate opens.
    pub gate_index: Option<u8>,
    /// Whether to spawn exit portal NPCs (only after FLUWITON_ROOM_4_VI).
    pub spawn_exit_portals: bool,
    /// Whether this is a fast-despawn NPC (NPC_UTC_SPAWN_FAST type).
    pub despawn_fast: bool,
}

/// Process a monster death in the Under The Castle zone.
/// Returns the actions to take based on which monster died.
pub fn on_monster_death(proto_id: u16, npc_type: u16) -> UtcMonsterDeathResult {
    let despawn_fast = npc_type == NPC_UTC_SPAWN_FAST;

    let (movie_id, reward_room, gate_index, spawn_exit_portals) = match proto_id {
        // First room (v2615 UTC spawn table index 1002).
        MONSTER_EMPEROR_MAMMOTH_I => (7, 0, Some(0), false),
        MONSTER_EMPEROR_MAMMOTH_II => (8, 0, None, false),
        MONSTER_EMPEROR_MAMMOTH_III => (2, 1, None, false),
        MONSTER_CRESHERGIMMIC_I => (6, 0, None, false),
        MONSTER_CRESHERGIMMIC_II => (6, 0, None, false),
        MONSTER_CRESHERGIMMIC_III => (6, 0, None, false),
        // The supplied old C++ source uses an earlier set of intermediary
        // boss IDs. The v2615 `monster_under_the_castle` table has one boss
        // per remaining room: 9508 at (522,493), 9512 at (646,291), and
        // 9515 at (804,837). These are the IDs observed in the current log.
        MONSTER_CRESHERGIMMIC_VI => (3, 2, None, false),
        MONSTER_PURIOUS_I => (3, 2, Some(1), false),
        MONSTER_PURIOUS_II => (6, 0, None, false),
        MONSTER_PURIOUS_III => (6, 0, None, false),
        MONSTER_PURIOUS_VI => (4, 3, Some(1), false),
        MONSTER_FLUWITON_ROOM_3_I => (5, 4, Some(2), false),
        MONSTER_FLUWITON_ROOM_3_II => (6, 0, None, false),
        MONSTER_FLUWITON_ROOM_3_III => (5, 4, Some(2), false),
        MONSTER_FLUWITON_ROOM_4_I => (5, 5, None, true),
        MONSTER_FLUWITON_ROOM_4_II => (6, 0, None, false),
        MONSTER_FLUWITON_ROOM_4_III => (6, 0, None, false),
        MONSTER_FLUWITON_ROOM_4_VI => (0, 5, None, true),
        _ => (0, 0, None, false),
    };

    UtcMonsterDeathResult {
        movie_id,
        reward_room,
        gate_index,
        spawn_exit_portals,
        despawn_fast,
    }
}

/// Remove a monster from the tracked monster list.
pub fn remove_from_monster_list(state: &UnderTheCastleState, npc_id: u32) {
    {
        let mut list = state.monster_list.write();
        list.retain(|&id| id != npc_id);
    }
}

/// Register a gate NPC ID for a given gate index (0-2).
pub fn set_gate_id(state: &UnderTheCastleState, gate_index: u8, npc_id: u32) {
    if gate_index > 2 {
        return;
    }
    let mut gates = state.gate_ids.write();
    let gate_ids = &mut gates[gate_index as usize];
    if !gate_ids.contains(&npc_id) {
        gate_ids.push(npc_id);
    }
}

/// Get every runtime NPC ID belonging to a gate stage.
pub fn get_gate_ids(state: &UnderTheCastleState, gate_index: u8) -> Vec<u32> {
    if gate_index > 2 {
        return Vec::new();
    }
    state.gate_ids.read()[gate_index as usize].clone()
}

// ── Packet builders ────────────────────────────────────────────────────

/// Build a WIZ_UTC_MOVIE packet.
///                `result << uint8(1) << uint16(1) << uint32(movie_id);`
/// Format: `[0x92][u8:2][u8:1][u16:1][u32:movie_id]`
pub fn build_utc_movie_packet(movie_id: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_UTC_MOVIE);
    pkt.write_u8(2);
    pkt.write_u8(1);
    pkt.write_u16(1);
    pkt.write_u32(movie_id);
    pkt
}

/// Collect all monster NPC IDs for cleanup on event end.
pub fn get_all_monster_ids(state: &UnderTheCastleState) -> Vec<u32> {
    state.monster_list.read().clone()
}

/// Add a spawned monster NPC ID to the tracking list.
pub fn add_monster_id(state: &UnderTheCastleState, npc_id: u32) {
    {
        let mut list = state.monster_list.write();
        list.push(npc_id);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── State tests ────────────────────────────────────────────────

    #[test]
    fn state_new_is_inactive() {
        let state = UnderTheCastleState::new();
        assert!(!state.is_active.load(Ordering::Relaxed));
        assert!(!state.is_summon.load(Ordering::Relaxed));
        assert_eq!(state.start_time.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn state_default_is_inactive() {
        let state = UnderTheCastleState::default();
        assert!(!state.is_active.load(Ordering::Relaxed));
    }

    #[test]
    fn state_reset_clears_all_fields() {
        let state = UnderTheCastleState::new();
        state.is_active.store(true, Ordering::Relaxed);
        state.is_summon.store(true, Ordering::Relaxed);
        state.start_time.store(9999, Ordering::Relaxed);
        state.start_move_time.store(9984, Ordering::Relaxed);
        state.min_level.store(70, Ordering::Relaxed);
        state.max_level.store(83, Ordering::Relaxed);
        add_monster_id(&state, 100);
        set_gate_id(&state, 0, 200);

        state.reset();

        assert!(!state.is_active.load(Ordering::Relaxed));
        assert!(!state.is_summon.load(Ordering::Relaxed));
        assert_eq!(state.start_time.load(Ordering::Relaxed), 0);
        assert_eq!(state.start_move_time.load(Ordering::Relaxed), 0);
        assert_eq!(state.min_level.load(Ordering::Relaxed), 0);
        assert_eq!(state.max_level.load(Ordering::Relaxed), 0);
        assert!(get_all_monster_ids(&state).is_empty());
        assert!(get_gate_ids(&state, 0).is_empty());
    }

    // ── Activation tests ───────────────────────────────────────────

    #[test]
    fn activate_event_success() {
        let state = UnderTheCastleState::new();
        let result = activate_event(&state, 180, 70, 83);

        assert!(result);
        assert!(state.is_active.load(Ordering::Relaxed));
        assert!(!state.is_summon.load(Ordering::Relaxed));
        assert_eq!(state.start_time.load(Ordering::Relaxed), 10800);
        assert_eq!(state.start_move_time.load(Ordering::Relaxed), 10785);
        assert_eq!(state.min_level.load(Ordering::Relaxed), 70);
        assert_eq!(state.max_level.load(Ordering::Relaxed), 83);
    }

    #[test]
    fn activate_event_already_active_fails() {
        let state = UnderTheCastleState::new();
        activate_event(&state, 180, 70, 83);
        let result = activate_event(&state, 60, 70, 83);
        assert!(!result);
    }

    #[test]
    fn activate_event_zero_duration_fails() {
        let state = UnderTheCastleState::new();
        let result = activate_event(&state, 0, 70, 83);
        assert!(!result);
        assert!(!state.is_active.load(Ordering::Relaxed));
    }

    // ── Force stop tests ───────────────────────────────────────────

    #[test]
    fn force_stop_sets_low_timer() {
        let state = UnderTheCastleState::new();
        activate_event(&state, 180, 70, 83);
        force_stop_event(&state);
        assert_eq!(state.start_time.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn force_stop_inactive_noop() {
        let state = UnderTheCastleState::new();
        force_stop_event(&state);
        assert_eq!(state.start_time.load(Ordering::Relaxed), 0);
    }

    // ── Timer tick tests ───────────────────────────────────────────

    #[test]
    fn tick_idle_when_not_active() {
        let state = UnderTheCastleState::new();
        assert_eq!(timer_tick(&state), UtcTickResult::Idle);
    }

    #[test]
    fn tick_spawn_monsters_on_first_tick() {
        let state = UnderTheCastleState::new();
        activate_event(&state, 180, 70, 83);

        assert_eq!(timer_tick(&state), UtcTickResult::SpawnMonsters);
        assert!(state.is_summon.load(Ordering::Relaxed));
    }

    #[test]
    fn tick_normal_decrement() {
        let state = UnderTheCastleState::new();
        activate_event(&state, 180, 70, 83);
        // First tick spawns
        timer_tick(&state);

        let before = state.start_time.load(Ordering::Relaxed);
        assert_eq!(timer_tick(&state), UtcTickResult::Tick);
        let after = state.start_time.load(Ordering::Relaxed);
        assert_eq!(before - 1, after);
    }

    #[test]
    fn tick_movie_trigger() {
        let state = UnderTheCastleState::new();
        activate_event(&state, 180, 70, 83);
        // Spawn monsters
        timer_tick(&state);

        // Set timer to exactly the movie trigger point
        let move_time = state.start_move_time.load(Ordering::Relaxed);
        state.start_time.store(move_time, Ordering::Relaxed);

        assert_eq!(timer_tick(&state), UtcTickResult::TriggerMovie);
        // Timer should have been decremented
        assert_eq!(state.start_time.load(Ordering::Relaxed), move_time - 1);
    }

    #[test]
    fn tick_finish_at_zero() {
        let state = UnderTheCastleState::new();
        activate_event(&state, 180, 70, 83);
        // Spawn monsters
        timer_tick(&state);
        // Set to zero
        state.start_time.store(0, Ordering::Relaxed);

        assert_eq!(timer_tick(&state), UtcTickResult::Finish);
    }

    // ── Entry validation tests ─────────────────────────────────────

    #[test]
    fn entry_allowed() {
        assert_eq!(
            can_player_enter(true, 75, 42, true),
            UtcEntryResult::Allowed
        );
    }

    #[test]
    fn entry_event_not_active() {
        assert_eq!(
            can_player_enter(false, 75, 42, true),
            UtcEntryResult::EventNotActive
        );
    }

    #[test]
    fn entry_level_too_low() {
        assert_eq!(
            can_player_enter(true, 69, 42, true),
            UtcEntryResult::LevelTooLow
        );
    }

    #[test]
    fn entry_level_exactly_minimum() {
        assert_eq!(
            can_player_enter(true, 70, 42, true),
            UtcEntryResult::Allowed
        );
    }

    #[test]
    fn entry_no_clan() {
        assert_eq!(can_player_enter(true, 75, 0, true), UtcEntryResult::NoClan);
    }

    #[test]
    fn entry_not_csw_winner() {
        assert_eq!(
            can_player_enter(true, 75, 42, false),
            UtcEntryResult::NotCswWinner
        );
    }

    // ── Monster death tests ────────────────────────────────────────

    #[test]
    fn death_emperor_mammoth_i_opens_gate_0() {
        let result = on_monster_death(MONSTER_EMPEROR_MAMMOTH_I, 0);
        assert_eq!(result.movie_id, 7);
        assert_eq!(result.reward_room, 0);
        assert_eq!(result.gate_index, Some(0));
        assert!(!result.spawn_exit_portals);
    }

    #[test]
    fn death_emperor_mammoth_ii() {
        let result = on_monster_death(MONSTER_EMPEROR_MAMMOTH_II, 0);
        assert_eq!(result.movie_id, 8);
        assert_eq!(result.reward_room, 0);
    }

    #[test]
    fn death_emperor_mammoth_iii_rewards_room_1() {
        let result = on_monster_death(MONSTER_EMPEROR_MAMMOTH_III, 0);
        assert_eq!(result.movie_id, 2);
        assert_eq!(result.reward_room, 1);
        assert_eq!(result.gate_index, None);
    }

    #[test]
    fn death_creshergimmic_vi_rewards_room_2_without_gate() {
        let result = on_monster_death(MONSTER_CRESHERGIMMIC_VI, 0);
        assert_eq!(result.movie_id, 3);
        assert_eq!(result.reward_room, 2);
        assert_eq!(result.gate_index, None);
    }

    #[test]
    fn death_purious_vi_opens_gate_1() {
        let result = on_monster_death(MONSTER_PURIOUS_VI, 0);
        assert_eq!(result.movie_id, 4);
        assert_eq!(result.reward_room, 3);
        assert_eq!(result.gate_index, Some(1));
    }

    #[test]
    fn death_fluwiton_room_3_iii_opens_gate_2() {
        let result = on_monster_death(MONSTER_FLUWITON_ROOM_3_III, 0);
        assert_eq!(result.movie_id, 5);
        assert_eq!(result.reward_room, 4);
        assert_eq!(result.gate_index, Some(2));
    }

    #[test]
    fn death_fluwiton_room_4_vi_spawns_exit() {
        let result = on_monster_death(MONSTER_FLUWITON_ROOM_4_VI, 0);
        assert_eq!(result.movie_id, 0);
        assert_eq!(result.reward_room, 5);
        assert!(result.spawn_exit_portals);
        assert_eq!(result.gate_index, None);
    }

    #[test]
    fn death_generic_monster_no_effect() {
        let result = on_monster_death(9525, 0); // Garioneus — generic monster
        assert_eq!(result.movie_id, 0);
        assert_eq!(result.reward_room, 0);
        assert_eq!(result.gate_index, None);
        assert!(!result.spawn_exit_portals);
    }

    #[test]
    fn death_creshergimmic_i_ii_iii_movie_6() {
        for proto_id in [
            MONSTER_CRESHERGIMMIC_I,
            MONSTER_CRESHERGIMMIC_II,
            MONSTER_CRESHERGIMMIC_III,
        ] {
            let result = on_monster_death(proto_id, 0);
            assert_eq!(
                result.movie_id, 6,
                "proto_id={proto_id} should have movie_id=6"
            );
            assert_eq!(result.reward_room, 0);
            assert_eq!(result.gate_index, None);
        }
    }

    #[test]
    fn death_purious_i_is_v2615_room_2_boss() {
        let result = on_monster_death(MONSTER_PURIOUS_I, 0);
        assert_eq!(result.movie_id, 3);
        assert_eq!(result.reward_room, 2);
        assert_eq!(result.gate_index, Some(1));
    }

    #[test]
    fn death_purious_ii_iii_movie_6() {
        for proto_id in [MONSTER_PURIOUS_II, MONSTER_PURIOUS_III] {
            let result = on_monster_death(proto_id, 0);
            assert_eq!(
                result.movie_id, 6,
                "proto_id={proto_id} should have movie_id=6"
            );
        }
    }

    #[test]
    fn death_fluwiton_room_3_i_is_v2615_room_3_boss() {
        let result = on_monster_death(MONSTER_FLUWITON_ROOM_3_I, 0);
        assert_eq!(result.movie_id, 5);
        assert_eq!(result.reward_room, 4);
        assert_eq!(result.gate_index, Some(2));
    }

    #[test]
    fn death_fluwiton_room_4_i_is_v2615_final_boss() {
        let result = on_monster_death(MONSTER_FLUWITON_ROOM_4_I, 0);
        assert_eq!(result.movie_id, 5);
        assert_eq!(result.reward_room, 5);
        assert!(result.spawn_exit_portals);
    }

    #[test]
    fn death_fluwiton_room_4_ii_iii_movie_6() {
        for proto_id in [MONSTER_FLUWITON_ROOM_4_II, MONSTER_FLUWITON_ROOM_4_III] {
            let result = on_monster_death(proto_id, 0);
            assert_eq!(
                result.movie_id, 6,
                "proto_id={proto_id} should have movie_id=6"
            );
        }
    }

    #[test]
    fn death_fast_despawn_npc() {
        let result = on_monster_death(MONSTER_EMPEROR_MAMMOTH_I, NPC_UTC_SPAWN_FAST);
        assert!(result.despawn_fast);
    }

    #[test]
    fn death_normal_npc_not_fast() {
        let result = on_monster_death(MONSTER_EMPEROR_MAMMOTH_I, 0);
        assert!(!result.despawn_fast);
    }

    // ── Monster list tracking tests ────────────────────────────────

    #[test]
    fn add_and_get_monster_ids() {
        let state = UnderTheCastleState::new();
        add_monster_id(&state, 100);
        add_monster_id(&state, 200);
        add_monster_id(&state, 300);

        let ids = get_all_monster_ids(&state);
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&100));
        assert!(ids.contains(&200));
        assert!(ids.contains(&300));
    }

    #[test]
    fn remove_monster_id() {
        let state = UnderTheCastleState::new();
        add_monster_id(&state, 100);
        add_monster_id(&state, 200);

        remove_from_monster_list(&state, 100);

        let ids = get_all_monster_ids(&state);
        assert_eq!(ids.len(), 1);
        assert!(!ids.contains(&100));
        assert!(ids.contains(&200));
    }

    #[test]
    fn remove_nonexistent_monster_id_noop() {
        let state = UnderTheCastleState::new();
        add_monster_id(&state, 100);

        remove_from_monster_list(&state, 999);

        let ids = get_all_monster_ids(&state);
        assert_eq!(ids.len(), 1);
    }

    // ── Gate ID tests ──────────────────────────────────────────────

    #[test]
    fn set_and_get_gate_ids() {
        let state = UnderTheCastleState::new();
        set_gate_id(&state, 0, 1000);
        set_gate_id(&state, 0, 1001);
        set_gate_id(&state, 1, 2000);
        set_gate_id(&state, 2, 3000);
        set_gate_id(&state, 2, 3001);

        assert_eq!(get_gate_ids(&state, 0), vec![1000, 1001]);
        assert_eq!(get_gate_ids(&state, 1), vec![2000]);
        assert_eq!(get_gate_ids(&state, 2), vec![3000, 3001]);
    }

    #[test]
    fn get_gate_ids_uninitialized_returns_empty() {
        let state = UnderTheCastleState::new();
        assert!(get_gate_ids(&state, 0).is_empty());
        assert!(get_gate_ids(&state, 1).is_empty());
        assert!(get_gate_ids(&state, 2).is_empty());
    }

    #[test]
    fn set_gate_id_out_of_bounds_ignored() {
        let state = UnderTheCastleState::new();
        set_gate_id(&state, 3, 9999); // should be ignored
        set_gate_id(&state, 255, 9999); // should be ignored
        assert!(get_gate_ids(&state, 3).is_empty());
    }

    // ── Packet builder tests ───────────────────────────────────────

    #[test]
    fn build_utc_movie_packet_format() {
        let pkt = build_utc_movie_packet(7);
        assert_eq!(pkt.opcode, WIZ_UTC_MOVIE);
        // Data: [u8:2, u8:1, u16:1 (le), u32:7 (le)]
        assert_eq!(pkt.data.len(), 8);
        assert_eq!(pkt.data[0], 2);
        assert_eq!(pkt.data[1], 1);
        // u16 LE: 1 -> [0x01, 0x00]
        assert_eq!(pkt.data[2], 0x01);
        assert_eq!(pkt.data[3], 0x00);
        // u32 LE: 7 -> [0x07, 0x00, 0x00, 0x00]
        assert_eq!(pkt.data[4], 0x07);
        assert_eq!(pkt.data[5], 0x00);
        assert_eq!(pkt.data[6], 0x00);
        assert_eq!(pkt.data[7], 0x00);
    }

    #[test]
    fn build_utc_movie_packet_different_ids() {
        for movie_id in [1, 2, 3, 4, 5, 6, 7, 8] {
            let pkt = build_utc_movie_packet(movie_id);
            let id_bytes = &pkt.data[4..8];
            let reconstructed =
                u32::from_le_bytes([id_bytes[0], id_bytes[1], id_bytes[2], id_bytes[3]]);
            assert_eq!(reconstructed, movie_id);
        }
    }

    #[test]
    fn final_boss_transition_gate_matches_reference_event_trap() {
        assert!(is_at_final_boss_transition_gate(646.0, 236.0));
        assert!(is_at_final_boss_transition_gate(651.0, 236.0));
        assert!(!is_at_final_boss_transition_gate(651.1, 236.0));
        assert_eq!(UTC_FINAL_BOSS_WARP_X, 824.0);
        assert_eq!(UTC_FINAL_BOSS_WARP_Z, 932.0);
    }

    // ── Full lifecycle test ────────────────────────────────────────

    #[test]
    fn full_event_lifecycle() {
        let state = UnderTheCastleState::new();

        // 1. Activate
        assert!(activate_event(&state, 180, 70, 83));
        assert!(state.is_active.load(Ordering::Relaxed));

        // 2. First tick = spawn monsters
        assert_eq!(timer_tick(&state), UtcTickResult::SpawnMonsters);

        // 3. Normal ticks decrement
        for _ in 0..5 {
            let result = timer_tick(&state);
            assert!(result == UtcTickResult::Tick || result == UtcTickResult::TriggerMovie);
        }

        // 4. Simulate countdown to movie trigger
        let move_time = state.start_move_time.load(Ordering::Relaxed);
        state.start_time.store(move_time, Ordering::Relaxed);
        assert_eq!(timer_tick(&state), UtcTickResult::TriggerMovie);

        // 5. Run down to zero
        state.start_time.store(0, Ordering::Relaxed);
        assert_eq!(timer_tick(&state), UtcTickResult::Finish);

        // 6. Reset
        state.reset();
        assert!(!state.is_active.load(Ordering::Relaxed));
        assert_eq!(timer_tick(&state), UtcTickResult::Idle);
    }

    // ── Start time edge cases ──────────────────────────────────────

    #[test]
    fn activate_with_small_duration() {
        let state = UnderTheCastleState::new();
        assert!(activate_event(&state, 1, 70, 83));
        assert_eq!(state.start_time.load(Ordering::Relaxed), 60);
        assert_eq!(state.start_move_time.load(Ordering::Relaxed), 45);
    }

    #[test]
    fn constants_match_cpp() {
        assert_eq!(ZONE_UNDER_CASTLE, 86);
        assert_eq!(WIZ_UTC_MOVIE, 0x92);
        assert_eq!(MIN_LEVEL_UNDER_CASTLE, 70);
        assert_eq!(NPC_UTC_SPAWN_FAST, 214);
        assert_eq!(MONSTER_EMPEROR_MAMMOTH_I, 9501);
        assert_eq!(MONSTER_FLUWITON_ROOM_4_VI, 9518);
        assert_eq!(UTC_EXIT_PORTAL_NPC, 29197);
    }

    // ── Reward item constant tests ────────────────────────────────────

    #[test]
    fn reward_item_constants_match_cpp() {
        // C++ Define.h:356-361
        assert_eq!(TROPHY_OF_FLAME, 800_149_000);
        assert_eq!(DENTED_IRONMASS, 508_147_000);
        assert_eq!(PETRIFIED_WEAPON_SHRAPNEL, 508_149_000);
        assert_eq!(IRON_POWDER_OF_CHAIN, 508_151_000);
        assert_eq!(PLWITOONS_TEAR, 508_152_000);
        assert_eq!(HORN_OF_PLUWITOON, 810_479_000);
    }

    // ── Room center geometry tests ────────────────────────────────────

    #[test]
    fn room_center_rooms_1_and_2_same() {
        let c1 = get_room_center(1).unwrap();
        let c2 = get_room_center(2).unwrap();
        assert_eq!(c1, c2);
        assert_eq!(c1, (121.0, 297.0, 80.0));
    }

    #[test]
    fn room_center_room_3() {
        let c = get_room_center(3).unwrap();
        assert_eq!(c, (520.0, 494.0, 115.0));
    }

    #[test]
    fn room_center_room_4() {
        let c = get_room_center(4).unwrap();
        assert_eq!(c, (642.0, 351.0, 100.0));
    }

    #[test]
    fn room_center_room_5() {
        let c = get_room_center(5).unwrap();
        assert_eq!(c, (803.0, 839.0, 110.0));
    }

    #[test]
    fn room_center_invalid_returns_none() {
        assert!(get_room_center(0).is_none());
        assert!(get_room_center(6).is_none());
        assert!(get_room_center(255).is_none());
    }

    // ── is_in_range_slow tests ────────────────────────────────────────

    #[test]
    fn in_range_exact_distance() {
        // Point at distance exactly equal to range
        assert!(is_in_range_slow(0.0, 0.0, 10.0, 0.0, 10.0));
    }

    #[test]
    fn in_range_inside() {
        assert!(is_in_range_slow(5.0, 5.0, 0.0, 0.0, 10.0));
    }

    #[test]
    fn in_range_outside() {
        assert!(!is_in_range_slow(20.0, 0.0, 0.0, 0.0, 10.0));
    }

    #[test]
    fn in_range_same_point() {
        assert!(is_in_range_slow(100.0, 200.0, 100.0, 200.0, 0.0));
    }

    // ── Trophy count calculation tests ────────────────────────────────

    #[test]
    fn trophy_both_ranges_gives_2() {
        // Player at room center AND near boss
        let trophy = calculate_trophy_count(
            121.0, 297.0, // player at room center
            121.0, 297.0, 80.0, // room center
            121.0, 297.0, // boss at same position
        );
        assert_eq!(trophy, 2);
    }

    #[test]
    fn trophy_room_only_gives_1() {
        // Player in room center range but far from boss
        let trophy = calculate_trophy_count(
            121.0, 297.0, // player at room center
            121.0, 297.0, 80.0, // room center
            900.0, 900.0, // boss far away
        );
        assert_eq!(trophy, 1);
    }

    #[test]
    fn trophy_boss_only_gives_1() {
        // Player near boss but far from room center
        let trophy = calculate_trophy_count(
            500.0, 500.0, // player far from room center
            121.0, 297.0, 80.0, // room 1/2 center (far)
            500.0, 500.0, // boss at player position
        );
        assert_eq!(trophy, 1);
    }

    #[test]
    fn trophy_neither_gives_0() {
        // Player far from both
        let trophy = calculate_trophy_count(
            500.0, 500.0, // player
            121.0, 297.0, 80.0, // room center
            800.0, 800.0, // boss far away too
        );
        assert_eq!(trophy, 0);
    }

    // ── Room reward items tests ───────────────────────────────────────

    #[test]
    fn room_1_no_bonus_items() {
        let items = get_utc_room_reward_items(1);
        assert!(items.is_empty());
    }

    #[test]
    fn room_2_dented_ironmass() {
        let items = get_utc_room_reward_items(2);
        assert_eq!(items, vec![DENTED_IRONMASS]);
    }

    #[test]
    fn room_3_petrified_weapon_shrapnel() {
        let items = get_utc_room_reward_items(3);
        assert_eq!(items, vec![PETRIFIED_WEAPON_SHRAPNEL]);
    }

    #[test]
    fn room_4_iron_powder_of_chain() {
        let items = get_utc_room_reward_items(4);
        assert_eq!(items, vec![IRON_POWDER_OF_CHAIN]);
    }

    #[test]
    fn room_5_two_bonus_items() {
        let items = get_utc_room_reward_items(5);
        assert_eq!(items, vec![PLWITOONS_TEAR, HORN_OF_PLUWITOON]);
    }

    #[test]
    fn room_invalid_no_bonus_items() {
        assert!(get_utc_room_reward_items(0).is_empty());
        assert!(get_utc_room_reward_items(6).is_empty());
    }

    // ── Boss kill range constant ──────────────────────────────────────

    #[test]
    fn boss_kill_range_matches_cpp() {
        // C++ uses isInRangeSlow(GetX, GetZ, 15)
        assert_eq!(UTC_BOSS_KILL_RANGE, 15.0);
    }

    // ── Full trophy calculation with real room geometry ───────────────

    #[test]
    fn room_1_trophy_at_center() {
        // Player at (121, 297), boss at (130, 290)
        // Distance to center: 0.0, within 80 -> in room
        // Distance to boss: sqrt(81+49) = ~11.4, within 15 -> near boss
        let trophy = calculate_trophy_count(121.0, 297.0, 121.0, 297.0, 80.0, 130.0, 290.0);
        assert_eq!(trophy, 2);
    }

    #[test]
    fn room_3_player_at_edge() {
        // Room 3 center (520, 494), range 115
        // Player at (520 + 100, 494) = (620, 494), distance = 100 < 115 -> in room
        // Boss at (520, 494), player distance to boss = 100, > 15 -> not near boss
        let trophy = calculate_trophy_count(620.0, 494.0, 520.0, 494.0, 115.0, 520.0, 494.0);
        assert_eq!(trophy, 1);
    }

    #[test]
    fn room_5_both_ranges() {
        // Room 5 center (803, 839), range 110
        // Player at (803, 839) -> in room (distance 0)
        // Boss at (810, 840) -> player distance = sqrt(49+1) = ~7.07 < 15 -> near boss
        let trophy = calculate_trophy_count(803.0, 839.0, 803.0, 839.0, 110.0, 810.0, 840.0);
        assert_eq!(trophy, 2);
    }

    #[test]
    fn room_4_out_of_range() {
        // Room 4 center (642, 351), range 100
        // Player at (100, 100), far from everything
        let trophy = calculate_trophy_count(100.0, 100.0, 642.0, 351.0, 100.0, 650.0, 360.0);
        assert_eq!(trophy, 0);
    }
}
