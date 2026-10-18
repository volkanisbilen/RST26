//! Juraid Mountain event system — PvE-focused room event.
//! Juraid Mountain is a PvE event where Karus and El Morad teams compete
//! to kill monsters. Each room has separate areas for each nation.
//! The nation that kills more monsters wins.
//! ## Zone Layout
//! Zone 87 has three bridge gates that open at timed intervals:
//! - Bridge 0 opens at start_time + 20 minutes (1200s)
//! - Bridge 1 opens at start_time + 30 minutes (1800s)
//! - Bridge 2 opens at start_time + 40 minutes (2400s)
//! Each bridge has separate Karus/El Morad versions (different NPC IDs).
//! ## Scoring
//! - Each monster kill adds 1 to the killing nation's score
//! - The nation with more kills wins at the end of the play timer
//! - On tie: draw (0)
//! ## Rewards
//! Loaded from `event_rewards` table (local_id=11 for Juraid).

use std::collections::HashMap;

use ko_db::models::MonsterJuraidRespawnRow;

use crate::systems::event_room::{
    EventRoom, EventRoomManager, EventUser, RoomState, TempleEventType, MAX_ROOM_USERS_PER_NATION,
};
use crate::world::WorldState;

pub use crate::world::types::ZONE_JURAID;

/// Default maximum rooms for Juraid.
/// The 2615 database contains families 21..28 (eight instanced rooms).
pub const DEFAULT_JURAID_ROOMS: u8 = 8;

/// Number of bridge gates in Juraid Mountain.
pub const NUM_BRIDGES: usize = 3;

/// Each strong monster death releases five lower monsters.
/// This matches the C++ `HandleJuraidKill()` implementation, which summons
/// `sCount = 5`; four children leave each side short of the documented
/// 20/40/60 bridge thresholds and prevent the final Deva room from opening.
pub const ROOM_CHILD_MONSTER_COUNT: u16 = 5;

/// Monster kill totals required to open the three room bridges.
pub const ROOM_BRIDGE_KILL_THRESHOLDS: [i32; NUM_BRIDGES] = [20, 40, 60];
pub const ROOM_MAIN_KILL_THRESHOLDS: [u16; NUM_BRIDGES] = [4, 8, 12];

/// Bridge open delays in seconds from event start.
// The event lasts 35 minutes and each section has a documented 10-minute
// fallback. 20/30/40 made the last fallback occur after the event had ended.
pub const BRIDGE_OPEN_DELAYS: [u64; NUM_BRIDGES] = [600, 1200, 1800];

pub const DEVA_BIRD_SID: u16 = 8106;
pub const BRIDGE_SID: u16 = 8110;
// KO_DATABASE_SERVER_001 K_MONSTER2369 contains the actual nation monument
// models as 9702/9703 (PIDs 14003/14004).  8113/8114 are quest/exchange IDs;
// the earlier synthetic templates reused bridge PID 6700 and could therefore
// never render as the two Deva-room monuments.
pub const KARUS_MONUMENT_SID: u16 = 9702;
pub const ELMORAD_MONUMENT_SID: u16 = 9703;
pub const JURAID_CHILD_SIDS: [u16; 3] = [2152, 8007, 1772];
pub const MONUMENT_RESPAWN_SECS: u64 = 60;
/// Deva is at (510, 510); place both objectives 16 metres to either side so
/// they remain inside the final combat area instead of the old 48m offsets.
pub const KARUS_MONUMENT_POSITION: (f32, f32) = (494.0, 510.0);
pub const ELMORAD_MONUMENT_POSITION: (f32, f32) = (526.0, 510.0);

pub const SUMMON_JURAID_MAIN: u8 = 2;
pub const SUMMON_JURAID_CHILD: u8 = 3;
pub const SUMMON_JURAID_DEVA: u8 = 4;
pub const SUMMON_JURAID_MONUMENT: u8 = 5;
pub const SUMMON_JURAID_BRIDGE: u8 = 6;

pub const GEM_GREEN: u32 = 389201000;
pub const GEM_BLUE: u32 = 389199000;
pub const GEM_YELLOW: u32 = 389198000;
pub const GEM_RED: u32 = 389197000;
pub const GEM_SILVERY: u32 = 389196000;
pub const GEM_FORTIFIED_STERLING: u32 = 811137000;

pub fn is_deva_bird(sid: u16) -> bool {
    sid == DEVA_BIRD_SID
}

pub fn is_bridge(sid: u16) -> bool {
    sid == BRIDGE_SID
}

pub fn is_juraid_monument(sid: u16) -> bool {
    sid == KARUS_MONUMENT_SID || sid == ELMORAD_MONUMENT_SID
}

pub fn monument_nation(sid: u16) -> u8 {
    match sid {
        KARUS_MONUMENT_SID => 1,
        ELMORAD_MONUMENT_SID => 2,
        _ => 0,
    }
}

/// Whether killing this monument grants one scoreboard point. Objectives are
/// enemy-only, and the design document permits scoring only while tied or
/// trailing.
pub fn can_monument_score(
    killer_nation: u8,
    monument_sid: u16,
    karus_score: i32,
    elmorad_score: i32,
) -> bool {
    let owner = monument_nation(monument_sid);
    if owner == 0 || owner == killer_nation {
        return false;
    }
    match killer_nation {
        1 => karus_score <= elmorad_score,
        2 => elmorad_score <= karus_score,
        _ => false,
    }
}

pub fn is_wave_monster(row: &MonsterJuraidRespawnRow) -> bool {
    row.b_type == 0
        && !is_deva_bird(row.s_sid as u16)
        && !is_bridge(row.s_sid as u16)
        && !is_juraid_monument(row.s_sid as u16)
}

pub fn is_main_monster(world: &WorldState, room_id: u8, sid: u16) -> bool {
    let family = 20 + room_id as i16;
    let rows = world.get_juraid_respawn_family(family);
    rows.iter()
        .any(|row| is_wave_monster(row) && row.s_sid as u16 == sid)
}

pub fn spawn_deva_bird(world: &WorldState, room_id: u8) -> usize {
    let family = 20 + room_id as i16;
    let rows = world.get_juraid_respawn_family(family);
    let (x, z) = rows
        .iter()
        .find(|row| row.s_sid as u16 == DEVA_BIRD_SID)
        .map(|row| (row.x as f32, row.z as f32))
        // Old databases may have the Deva template but no respawn-list row.
        .unwrap_or((510.0, 510.0));

    world
        .spawn_event_npc_ex(
            DEVA_BIRD_SID,
            true,
            ZONE_JURAID,
            x,
            z,
            1,
            room_id as u16,
            SUMMON_JURAID_DEVA,
        )
        .len()
}

pub fn spawn_monument(world: &WorldState, room_id: u8, nation: u8) -> usize {
    let (sid, x, z) = match nation {
        1 => (
            KARUS_MONUMENT_SID,
            KARUS_MONUMENT_POSITION.0,
            KARUS_MONUMENT_POSITION.1,
        ),
        2 => (
            ELMORAD_MONUMENT_SID,
            ELMORAD_MONUMENT_POSITION.0,
            ELMORAD_MONUMENT_POSITION.1,
        ),
        _ => return 0,
    };
    world
        .spawn_event_npc_ex(
            sid,
            true,
            ZONE_JURAID,
            x,
            z,
            1,
            room_id as u16,
            SUMMON_JURAID_MONUMENT,
        )
        .len()
}

pub fn spawn_deva_room_monuments(world: &WorldState, room_id: u8) -> usize {
    spawn_monument(world, room_id, 1) + spawn_monument(world, room_id, 2)
}

pub fn juraid_winner_gem(level: u8, rebirth_level: u8) -> Option<u32> {
    match level {
        75 | 76 => Some(GEM_GREEN),
        77 | 78 => Some(GEM_BLUE),
        79 | 80 => Some(GEM_YELLOW),
        81 | 82 => Some(GEM_RED),
        83 if rebirth_level >= 6 => Some(GEM_FORTIFIED_STERLING),
        83 => Some(GEM_SILVERY),
        _ => None,
    }
}

pub fn juraid_reward_gem_count(is_winner: bool) -> u16 {
    if is_winner {
        10
    } else {
        3
    }
}

// ── Juraid Bridge State ─────────────────────────────────────────────────────

/// Tracks bridge gate state for a Juraid room.
/// `pkBridges[3]`, `peBridges[3]`, `m_sKarusBridges[3]`, `m_sElmoBridges[3]`
#[derive(Debug, Clone)]
pub struct JuraidBridgeState {
    /// Whether each Karus bridge gate has been opened.
    pub karus_bridges: [bool; NUM_BRIDGES],
    /// Whether each El Morad bridge gate has been opened.
    pub elmorad_bridges: [bool; NUM_BRIDGES],
    /// NPC IDs for Karus bridge gates (0 = no NPC).
    pub karus_bridge_npcs: [u32; NUM_BRIDGES],
    /// NPC IDs for El Morad bridge gates (0 = no NPC).
    pub elmorad_bridge_npcs: [u32; NUM_BRIDGES],
    /// Legacy C++ progression counters. A gate opens at main/sub pairs
    /// 4/20, 8/40 and 12/60 for each nation independently.
    pub karus_main_kills: u16,
    pub karus_sub_kills: u16,
    pub elmorad_main_kills: u16,
    pub elmorad_sub_kills: u16,
}

impl JuraidBridgeState {
    /// Create a new default bridge state (all closed).
    pub fn new() -> Self {
        Self {
            karus_bridges: [false; NUM_BRIDGES],
            elmorad_bridges: [false; NUM_BRIDGES],
            karus_bridge_npcs: [0; NUM_BRIDGES],
            elmorad_bridge_npcs: [0; NUM_BRIDGES],
            karus_main_kills: 0,
            karus_sub_kills: 0,
            elmorad_main_kills: 0,
            elmorad_sub_kills: 0,
        }
    }

    /// Reset all bridge state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Check if a specific bridge index is valid.
    pub fn is_valid_bridge(index: usize) -> bool {
        index < NUM_BRIDGES
    }

    /// Open a bridge for a specific nation.
    ///
    /// Returns true if the bridge was newly opened (was closed before).
    pub fn open_bridge(&mut self, index: usize, nation: u8) -> bool {
        if index >= NUM_BRIDGES {
            return false;
        }

        match nation {
            1 => {
                if self.karus_bridges[index] {
                    return false;
                }
                self.karus_bridges[index] = true;
                true
            }
            2 => {
                if self.elmorad_bridges[index] {
                    return false;
                }
                self.elmorad_bridges[index] = true;
                true
            }
            _ => false,
        }
    }

    /// Check if a bridge is open for a given nation.
    pub fn is_bridge_open(&self, index: usize, nation: u8) -> bool {
        if index >= NUM_BRIDGES {
            return false;
        }
        match nation {
            1 => self.karus_bridges[index],
            2 => self.elmorad_bridges[index],
            _ => false,
        }
    }

    /// Count how many bridges are open for a nation.
    pub fn open_count(&self, nation: u8) -> usize {
        match nation {
            1 => self.karus_bridges.iter().filter(|&&b| b).count(),
            2 => self.elmorad_bridges.iter().filter(|&&b| b).count(),
            _ => 0,
        }
    }
}

impl Default for JuraidBridgeState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Juraid Room Extended State ──────────────────────────────────────────────

/// Extended Juraid room state beyond the generic `EventRoom`.
#[derive(Debug, Clone)]
pub struct JuraidRoomState {
    /// Monster kill count for Karus.
    pub karus_kills: i32,
    /// Monster kill count for El Morad.
    pub elmorad_kills: i32,
    /// Bridge gate state.
    pub bridges: JuraidBridgeState,
}

impl JuraidRoomState {
    /// Create a new default Juraid room state.
    pub fn new() -> Self {
        Self {
            karus_kills: 0,
            elmorad_kills: 0,
            bridges: JuraidBridgeState::new(),
        }
    }

    /// Reset the Juraid room state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for JuraidRoomState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Juraid Manager ──────────────────────────────────────────────────────────

/// Juraid Mountain event manager — coordinates Juraid-specific logic.
/// Stored alongside `EventRoomManager` in WorldState.
#[derive(Debug)]
pub struct JuraidManager {
    /// Extended Juraid state per room: keyed by room_id.
    pub room_states: HashMap<u8, JuraidRoomState>,
    /// Number of rooms to create for this event.
    pub max_rooms: u8,
    /// Whether the bridge timer system is active.
    pub bridge_active: bool,
    /// Unix timestamp when the bridge timer started.
    pub bridge_start_time: u64,
    /// Global bridge check flags (shared across all rooms).
    ///
    pub bridge_checks: [bool; NUM_BRIDGES],
}

impl JuraidManager {
    /// Create a new Juraid manager.
    pub fn new(max_rooms: u8) -> Self {
        Self {
            room_states: HashMap::new(),
            max_rooms,
            bridge_active: false,
            bridge_start_time: 0,
            bridge_checks: [false; NUM_BRIDGES],
        }
    }

    /// Initialize Juraid rooms in the event room manager and create extended state.
    pub fn init_rooms(&mut self, erm: &EventRoomManager) {
        erm.create_rooms(TempleEventType::JuraidMountain, self.max_rooms);
        self.room_states.clear();
        for room_id in 1..=self.max_rooms {
            self.room_states.insert(room_id, JuraidRoomState::new());
        }
    }

    /// Destroy all Juraid rooms and clear extended state.
    pub fn destroy_rooms(&mut self, erm: &EventRoomManager) {
        erm.destroy_rooms(TempleEventType::JuraidMountain);
        self.room_states.clear();
        self.bridge_active = false;
        self.bridge_start_time = 0;
        self.bridge_checks = [false; NUM_BRIDGES];
    }

    /// Get extended Juraid state for a room.
    pub fn get_room_state(&self, room_id: u8) -> Option<&JuraidRoomState> {
        self.room_states.get(&room_id)
    }

    /// Get mutable extended Juraid state for a room.
    pub fn get_room_state_mut(&mut self, room_id: u8) -> Option<&mut JuraidRoomState> {
        self.room_states.get_mut(&room_id)
    }

    /// Start the bridge timer.
    ///
    pub fn start_bridge_timer(&mut self, now: u64) {
        self.bridge_active = true;
        self.bridge_start_time = now;
        self.bridge_checks = [false; NUM_BRIDGES];
    }

    /// Reset all room states.
    pub fn reset_all(&mut self) {
        for state in self.room_states.values_mut() {
            state.reset();
        }
        self.bridge_active = false;
        self.bridge_start_time = 0;
        self.bridge_checks = [false; NUM_BRIDGES];
    }
}

impl Default for JuraidManager {
    fn default() -> Self {
        Self::new(DEFAULT_JURAID_ROOMS)
    }
}

// ── Room Assignment ─────────────────────────────────────────────────────────

/// Assign signed-up users to Juraid rooms.
/// Similar to BDW assignment: distributes users evenly across rooms.
/// Returns the number of users assigned.
pub fn assign_users_to_rooms(erm: &EventRoomManager, _juraid: &mut JuraidManager) -> usize {
    let users = erm.signed_up_users.read().clone();
    if users.is_empty() {
        return 0;
    }

    let mut karus_queue: Vec<_> = users.iter().filter(|u| u.nation == 1).collect();
    let mut elmorad_queue: Vec<_> = users.iter().filter(|u| u.nation == 2).collect();

    karus_queue.sort_by_key(|u| u.join_order);
    elmorad_queue.sort_by_key(|u| u.join_order);

    let mut total_assigned = 0;
    let mut room_ids = erm.list_rooms(TempleEventType::JuraidMountain);
    room_ids.sort();

    for room_id in &room_ids {
        if karus_queue.is_empty() && elmorad_queue.is_empty() {
            break;
        }

        let mut room = match erm.get_room_mut(TempleEventType::JuraidMountain, *room_id) {
            Some(r) => r,
            None => continue,
        };

        // Fill Karus slots
        let karus_slots = MAX_ROOM_USERS_PER_NATION - room.karus_users.len();
        for _ in 0..karus_slots {
            if let Some(signed_up) = karus_queue.first().cloned() {
                let user = EventUser {
                    user_name: signed_up.user_name.clone(),
                    session_id: signed_up.session_id,
                    nation: 1,
                    prize_given: false,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                };
                if room.add_user(user) {
                    karus_queue.remove(0);
                    total_assigned += 1;
                }
            }
        }

        // Fill El Morad slots
        let elmorad_slots = MAX_ROOM_USERS_PER_NATION - room.elmorad_users.len();
        for _ in 0..elmorad_slots {
            if let Some(signed_up) = elmorad_queue.first().cloned() {
                let user = EventUser {
                    user_name: signed_up.user_name.clone(),
                    session_id: signed_up.session_id,
                    nation: 2,
                    prize_given: false,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                };
                if room.add_user(user) {
                    elmorad_queue.remove(0);
                    total_assigned += 1;
                }
            }
        }

        room.state = RoomState::Running;
    }

    total_assigned
}

// ── Monster Kill Tracking ───────────────────────────────────────────────────

/// Record a monster kill in a Juraid room.
/// The killing nation gets +1 to their kill count. Also updates the generic
/// room score for winner determination.
/// Returns the updated (karus_kills, elmorad_kills).
pub fn record_monster_kill(
    room: &mut EventRoom,
    juraid_state: &mut JuraidRoomState,
    killer_nation: u8,
) -> (i32, i32) {
    match killer_nation {
        1 => {
            juraid_state.karus_kills += 1;
            room.karus_score += 1;
        }
        2 => {
            juraid_state.elmorad_kills += 1;
            room.elmorad_score += 1;
        }
        _ => {}
    }
    (juraid_state.karus_kills, juraid_state.elmorad_kills)
}

// ── Bridge Timer ────────────────────────────────────────────────────────────

/// Check and open bridges that are due based on elapsed time.
/// Returns a list of bridge indices that were newly opened.
pub fn check_bridge_timers(juraid: &mut JuraidManager, now: u64) -> Vec<usize> {
    if !juraid.bridge_active {
        return Vec::new();
    }

    let mut opened = Vec::with_capacity(BRIDGE_OPEN_DELAYS.len());

    for (i, &delay) in BRIDGE_OPEN_DELAYS.iter().enumerate() {
        if juraid.bridge_checks[i] {
            continue;
        }

        if now >= juraid.bridge_start_time + delay {
            juraid.bridge_checks[i] = true;
            opened.push(i);
        }
    }

    opened
}

/// Open a bridge for all rooms.
/// Returns the number of rooms where bridges were opened.
pub fn open_bridge_for_all_rooms(juraid: &mut JuraidManager, bridge_index: usize) -> usize {
    if bridge_index >= NUM_BRIDGES {
        return 0;
    }

    let mut count = 0;
    for state in juraid.room_states.values_mut() {
        let opened_k = state.bridges.open_bridge(bridge_index, 1);
        let opened_e = state.bridges.open_bridge(bridge_index, 2);
        if opened_k || opened_e {
            count += 1;
        }
    }
    count
}

// ── Winner Determination ────────────────────────────────────────────────────

/// Determine the winner of a Juraid room based on monster kill counts.
/// compares `m_iElmoradKillCount` vs `m_iKarusKillCount`
pub fn determine_winner(juraid_state: &JuraidRoomState) -> u8 {
    if juraid_state.karus_kills > juraid_state.elmorad_kills {
        1 // Karus
    } else if juraid_state.elmorad_kills > juraid_state.karus_kills {
        2 // El Morad
    } else {
        0 // Draw
    }
}

/// Determine winners for all Juraid rooms and set the room winner_nation.
/// Returns a list of (room_id, winner_nation) pairs.
pub fn determine_all_winners(erm: &EventRoomManager, juraid: &JuraidManager) -> Vec<(u8, u8)> {
    let room_ids = erm.list_rooms(TempleEventType::JuraidMountain);
    let mut results = Vec::with_capacity(room_ids.len());

    for room_id in room_ids {
        if let Some(mut room) = erm.get_room_mut(TempleEventType::JuraidMountain, room_id) {
            if room.finished || room.state != RoomState::Running {
                continue;
            }
            let winner = if room.winner_nation != 0 {
                room.winner_nation
            } else if room.karus_score > room.elmorad_score {
                1
            } else if room.elmorad_score > room.karus_score {
                2
            } else {
                juraid
                    .get_room_state(room_id)
                    .map(determine_winner)
                    .unwrap_or(0)
            };
            room.winner_nation = winner;
            room.finish_packet_sent = true;
            results.push((room_id, winner));
        }
    }

    results
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::event_room::EventRoomManager;

    fn make_erm_with_juraid_rooms(count: u8) -> (EventRoomManager, JuraidManager) {
        let erm = EventRoomManager::new();
        let mut juraid = JuraidManager::new(count);
        juraid.init_rooms(&erm);
        (erm, juraid)
    }

    // ── Bridge State Tests ──────────────────────────────────────────────

    #[test]
    fn test_bridge_state_default() {
        let state = JuraidBridgeState::new();
        assert_eq!(state.karus_bridges, [false; NUM_BRIDGES]);
        assert_eq!(state.elmorad_bridges, [false; NUM_BRIDGES]);
    }

    #[test]
    fn test_bridge_open() {
        let mut state = JuraidBridgeState::new();

        assert!(state.open_bridge(0, 1)); // Karus bridge 0
        assert!(state.is_bridge_open(0, 1));
        assert!(!state.is_bridge_open(0, 2)); // El Morad not opened

        assert!(state.open_bridge(0, 2)); // El Morad bridge 0
        assert!(state.is_bridge_open(0, 2));
    }

    #[test]
    fn test_bridge_open_already_open() {
        let mut state = JuraidBridgeState::new();

        assert!(state.open_bridge(1, 1));
        assert!(!state.open_bridge(1, 1)); // Already open
    }

    #[test]
    fn test_bridge_open_invalid_index() {
        let mut state = JuraidBridgeState::new();
        assert!(!state.open_bridge(3, 1));
        assert!(!state.open_bridge(255, 2));
    }

    #[test]
    fn test_bridge_open_invalid_nation() {
        let mut state = JuraidBridgeState::new();
        assert!(!state.open_bridge(0, 0));
        assert!(!state.open_bridge(0, 3));
    }

    #[test]
    fn test_bridge_is_valid() {
        assert!(JuraidBridgeState::is_valid_bridge(0));
        assert!(JuraidBridgeState::is_valid_bridge(1));
        assert!(JuraidBridgeState::is_valid_bridge(2));
        assert!(!JuraidBridgeState::is_valid_bridge(3));
    }

    #[test]
    fn test_bridge_open_count() {
        let mut state = JuraidBridgeState::new();
        assert_eq!(state.open_count(1), 0);
        assert_eq!(state.open_count(2), 0);

        state.open_bridge(0, 1);
        state.open_bridge(2, 1);
        assert_eq!(state.open_count(1), 2);
        assert_eq!(state.open_count(2), 0);

        state.open_bridge(1, 2);
        assert_eq!(state.open_count(2), 1);
    }

    #[test]
    fn test_bridge_reset() {
        let mut state = JuraidBridgeState::new();
        state.open_bridge(0, 1);
        state.open_bridge(1, 2);

        state.reset();
        assert_eq!(state.karus_bridges, [false; NUM_BRIDGES]);
        assert_eq!(state.elmorad_bridges, [false; NUM_BRIDGES]);
    }

    #[test]
    fn test_bridge_is_open_invalid_index() {
        let state = JuraidBridgeState::new();
        assert!(!state.is_bridge_open(3, 1));
        assert!(!state.is_bridge_open(0, 0));
    }

    // ── Juraid Room State Tests ─────────────────────────────────────────

    #[test]
    fn test_juraid_room_state_default() {
        let state = JuraidRoomState::new();
        assert_eq!(state.karus_kills, 0);
        assert_eq!(state.elmorad_kills, 0);
    }

    #[test]
    fn test_juraid_room_state_reset() {
        let mut state = JuraidRoomState::new();
        state.karus_kills = 50;
        state.elmorad_kills = 30;
        state.bridges.open_bridge(0, 1);

        state.reset();
        assert_eq!(state.karus_kills, 0);
        assert_eq!(state.elmorad_kills, 0);
        assert!(!state.bridges.is_bridge_open(0, 1));
    }

    // ── Juraid Manager Tests ────────────────────────────────────────────

    #[test]
    fn test_juraid_manager_init_destroy() {
        let erm = EventRoomManager::new();
        let mut juraid = JuraidManager::new(5);

        juraid.init_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::JuraidMountain), 5);
        assert_eq!(juraid.room_states.len(), 5);
        assert!(juraid.get_room_state(1).is_some());
        assert!(juraid.get_room_state(5).is_some());
        assert!(juraid.get_room_state(6).is_none());

        juraid.destroy_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::JuraidMountain), 0);
        assert!(juraid.room_states.is_empty());
        assert!(!juraid.bridge_active);
    }

    #[test]
    fn test_juraid_manager_bridge_timer() {
        let mut juraid = JuraidManager::new(1);
        assert!(!juraid.bridge_active);

        juraid.start_bridge_timer(10000);
        assert!(juraid.bridge_active);
        assert_eq!(juraid.bridge_start_time, 10000);
        assert_eq!(juraid.bridge_checks, [false; NUM_BRIDGES]);
    }

    #[test]
    fn test_juraid_manager_reset_all() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(3);

        juraid.get_room_state_mut(1).unwrap().karus_kills = 10;
        juraid.get_room_state_mut(2).unwrap().elmorad_kills = 20;
        juraid.bridge_active = true;
        juraid.bridge_start_time = 99999;
        juraid.bridge_checks = [true, false, true];

        juraid.reset_all();

        assert_eq!(juraid.get_room_state(1).unwrap().karus_kills, 0);
        assert_eq!(juraid.get_room_state(2).unwrap().elmorad_kills, 0);
        assert!(!juraid.bridge_active);
        assert_eq!(juraid.bridge_start_time, 0);
        assert_eq!(juraid.bridge_checks, [false; NUM_BRIDGES]);

        // rooms in erm still exist (reset_all doesn't destroy rooms)
        assert_eq!(erm.room_count(TempleEventType::JuraidMountain), 3);
    }

    // ── Monster Kill Tests ──────────────────────────────────────────────

    #[test]
    fn test_record_monster_kill_karus() {
        let mut room = EventRoom::new(1, TempleEventType::JuraidMountain);
        let mut state = JuraidRoomState::new();

        let (k, e) = record_monster_kill(&mut room, &mut state, 1);
        assert_eq!(k, 1);
        assert_eq!(e, 0);
        assert_eq!(room.karus_score, 1);
        assert_eq!(room.elmorad_score, 0);
    }

    #[test]
    fn test_record_monster_kill_elmorad() {
        let mut room = EventRoom::new(1, TempleEventType::JuraidMountain);
        let mut state = JuraidRoomState::new();

        let (k, e) = record_monster_kill(&mut room, &mut state, 2);
        assert_eq!(k, 0);
        assert_eq!(e, 1);
        assert_eq!(room.karus_score, 0);
        assert_eq!(room.elmorad_score, 1);
    }

    #[test]
    fn test_record_monster_kill_invalid() {
        let mut room = EventRoom::new(1, TempleEventType::JuraidMountain);
        let mut state = JuraidRoomState::new();

        let (k, e) = record_monster_kill(&mut room, &mut state, 0);
        assert_eq!(k, 0);
        assert_eq!(e, 0);
    }

    #[test]
    fn test_record_monster_kill_accumulates() {
        let mut room = EventRoom::new(1, TempleEventType::JuraidMountain);
        let mut state = JuraidRoomState::new();

        for _ in 0..10 {
            record_monster_kill(&mut room, &mut state, 1);
        }
        for _ in 0..7 {
            record_monster_kill(&mut room, &mut state, 2);
        }

        assert_eq!(state.karus_kills, 10);
        assert_eq!(state.elmorad_kills, 7);
        assert_eq!(room.karus_score, 10);
        assert_eq!(room.elmorad_score, 7);
    }

    // ── Bridge Timer Tests ──────────────────────────────────────────────

    #[test]
    fn test_check_bridge_timers_not_active() {
        let mut juraid = JuraidManager::new(1);
        let opened = check_bridge_timers(&mut juraid, 99999);
        assert!(opened.is_empty());
    }

    #[test]
    fn test_check_bridge_timers_sequential() {
        let mut juraid = JuraidManager::new(1);
        juraid.start_bridge_timer(10000);

        // Before any bridge opens
        let opened = check_bridge_timers(&mut juraid, 10000 + 599);
        assert!(opened.is_empty());

        // Bridge 0 opens at +600
        let opened = check_bridge_timers(&mut juraid, 10000 + 600);
        assert_eq!(opened, vec![0]);
        assert!(juraid.bridge_checks[0]);

        // Bridge 0 already opened, no re-trigger
        let opened = check_bridge_timers(&mut juraid, 10000 + 600);
        assert!(opened.is_empty());

        // Bridge 1 opens at +1200
        let opened = check_bridge_timers(&mut juraid, 10000 + 1200);
        assert_eq!(opened, vec![1]);

        // Bridge 2 opens at +1800
        let opened = check_bridge_timers(&mut juraid, 10000 + 1800);
        assert_eq!(opened, vec![2]);

        // All opened, nothing more
        let opened = check_bridge_timers(&mut juraid, 10000 + 99999);
        assert!(opened.is_empty());
    }

    #[test]
    fn test_check_bridge_timers_all_at_once() {
        let mut juraid = JuraidManager::new(1);
        juraid.start_bridge_timer(0);

        // All three should open at once if enough time has passed
        let opened = check_bridge_timers(&mut juraid, 3000);
        assert_eq!(opened.len(), 3);
        assert!(opened.contains(&0));
        assert!(opened.contains(&1));
        assert!(opened.contains(&2));
    }

    #[test]
    fn test_open_bridge_for_all_rooms() {
        let (_, mut juraid) = make_erm_with_juraid_rooms(3);

        let count = open_bridge_for_all_rooms(&mut juraid, 0);
        assert_eq!(count, 3);

        // All rooms should have bridge 0 open for both nations
        for state in juraid.room_states.values() {
            assert!(state.bridges.is_bridge_open(0, 1));
            assert!(state.bridges.is_bridge_open(0, 2));
        }

        // Opening again should return 0 (already open)
        let count = open_bridge_for_all_rooms(&mut juraid, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_open_bridge_for_all_rooms_invalid_index() {
        let (_, mut juraid) = make_erm_with_juraid_rooms(2);
        let count = open_bridge_for_all_rooms(&mut juraid, 5);
        assert_eq!(count, 0);
    }

    // ── Winner Determination Tests ──────────────────────────────────────

    #[test]
    fn test_determine_winner_karus() {
        let state = JuraidRoomState {
            karus_kills: 50,
            elmorad_kills: 30,
            bridges: JuraidBridgeState::new(),
        };
        assert_eq!(determine_winner(&state), 1);
    }

    #[test]
    fn test_determine_winner_elmorad() {
        let state = JuraidRoomState {
            karus_kills: 20,
            elmorad_kills: 45,
            bridges: JuraidBridgeState::new(),
        };
        assert_eq!(determine_winner(&state), 2);
    }

    #[test]
    fn test_determine_winner_draw() {
        let state = JuraidRoomState {
            karus_kills: 25,
            elmorad_kills: 25,
            bridges: JuraidBridgeState::new(),
        };
        assert_eq!(determine_winner(&state), 0);
    }

    #[test]
    fn test_determine_winner_both_zero() {
        let state = JuraidRoomState::new();
        assert_eq!(determine_winner(&state), 0);
    }

    #[test]
    fn test_determine_all_winners() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(3);

        // Set room states
        juraid.get_room_state_mut(1).unwrap().karus_kills = 50;
        juraid.get_room_state_mut(1).unwrap().elmorad_kills = 30;
        juraid.get_room_state_mut(2).unwrap().karus_kills = 20;
        juraid.get_room_state_mut(2).unwrap().elmorad_kills = 40;
        juraid.get_room_state_mut(3).unwrap().karus_kills = 15;
        juraid.get_room_state_mut(3).unwrap().elmorad_kills = 15;

        // Set generic room states to Running
        for i in 1..=3 {
            if let Some(mut r) = erm.get_room_mut(TempleEventType::JuraidMountain, i) {
                r.state = RoomState::Running;
            }
        }

        let results = determine_all_winners(&erm, &juraid);
        assert_eq!(results.len(), 3);

        let r1 = results.iter().find(|(id, _)| *id == 1).unwrap();
        let r2 = results.iter().find(|(id, _)| *id == 2).unwrap();
        let r3 = results.iter().find(|(id, _)| *id == 3).unwrap();

        assert_eq!(r1.1, 1); // Karus
        assert_eq!(r2.1, 2); // El Morad
        assert_eq!(r3.1, 0); // Draw
    }

    #[test]
    fn test_determine_all_winners_skips_finished() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(2);

        juraid.get_room_state_mut(1).unwrap().karus_kills = 10;
        juraid.get_room_state_mut(2).unwrap().karus_kills = 20;

        // Room 1: Running
        if let Some(mut r) = erm.get_room_mut(TempleEventType::JuraidMountain, 1) {
            r.state = RoomState::Running;
        }
        // Room 2: Already finished
        if let Some(mut r) = erm.get_room_mut(TempleEventType::JuraidMountain, 2) {
            r.state = RoomState::Running;
            r.finished = true;
        }

        let results = determine_all_winners(&erm, &juraid);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    // ── Room Assignment Tests ───────────────────────────────────────────

    #[test]
    fn test_assign_users_basic() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(2);

        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("e1".to_string(), 2, 2);
        erm.add_signed_up_user("k2".to_string(), 3, 1);
        erm.add_signed_up_user("e2".to_string(), 4, 2);

        let assigned = assign_users_to_rooms(&erm, &mut juraid);
        assert_eq!(assigned, 4);

        let room = erm.get_room(TempleEventType::JuraidMountain, 1).unwrap();
        assert_eq!(room.karus_users.len(), 2);
        assert_eq!(room.elmorad_users.len(), 2);
        assert_eq!(room.state, RoomState::Running);
    }

    #[test]
    fn test_assign_users_empty() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(2);
        let assigned = assign_users_to_rooms(&erm, &mut juraid);
        assert_eq!(assigned, 0);
    }

    #[test]
    fn test_assign_users_overflow() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(2);

        // 10 per nation
        for i in 0..10 {
            erm.add_signed_up_user(format!("k{}", i), i as u16, 1);
        }
        for i in 0..10 {
            erm.add_signed_up_user(format!("e{}", i), (100 + i) as u16, 2);
        }

        let assigned = assign_users_to_rooms(&erm, &mut juraid);
        assert_eq!(assigned, 20);

        let room1 = erm.get_room(TempleEventType::JuraidMountain, 1).unwrap();
        assert_eq!(room1.karus_users.len(), MAX_ROOM_USERS_PER_NATION);
        assert_eq!(room1.elmorad_users.len(), MAX_ROOM_USERS_PER_NATION);

        let room2 = erm.get_room(TempleEventType::JuraidMountain, 2).unwrap();
        assert_eq!(room2.karus_users.len(), 2);
        assert_eq!(room2.elmorad_users.len(), 2);
    }

    // ── Constants Tests ─────────────────────────────────────────────────

    #[test]
    fn test_constants() {
        assert_eq!(ZONE_JURAID, 87);
        assert_eq!(NUM_BRIDGES, 3);
        assert_eq!(BRIDGE_OPEN_DELAYS, [600, 1200, 1800]);
        assert_eq!(DEFAULT_JURAID_ROOMS, 8);
        assert_eq!(KARUS_MONUMENT_POSITION, (494.0, 510.0));
        assert_eq!(ELMORAD_MONUMENT_POSITION, (526.0, 510.0));
    }

    #[test]
    fn test_monument_score_requires_enemy_and_tied_or_trailing() {
        // Karus may destroy the El Morad monument while tied/trailing.
        assert!(can_monument_score(1, ELMORAD_MONUMENT_SID, 0, 0));
        assert!(can_monument_score(1, ELMORAD_MONUMENT_SID, 0, 1));
        assert!(!can_monument_score(1, ELMORAD_MONUMENT_SID, 2, 1));

        // Own monument and non-monument entities never score.
        assert!(!can_monument_score(1, KARUS_MONUMENT_SID, 0, 0));
        assert!(!can_monument_score(2, ELMORAD_MONUMENT_SID, 0, 0));
        assert!(!can_monument_score(1, DEVA_BIRD_SID, 0, 0));
    }

    #[test]
    fn test_juraid_reward_gem_quantities() {
        assert_eq!(juraid_reward_gem_count(true), 10);
        assert_eq!(juraid_reward_gem_count(false), 3);
    }

    // ── Full Lifecycle Test ─────────────────────────────────────────────

    #[test]
    fn test_juraid_full_lifecycle() {
        let (erm, mut juraid) = make_erm_with_juraid_rooms(1);

        // Add users
        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("k2".to_string(), 2, 1);
        erm.add_signed_up_user("e1".to_string(), 3, 2);

        // Assign rooms
        let assigned = assign_users_to_rooms(&erm, &mut juraid);
        assert_eq!(assigned, 3);

        // Start bridge timer
        juraid.start_bridge_timer(10000);

        // Record some kills
        {
            let mut room = erm
                .get_room_mut(TempleEventType::JuraidMountain, 1)
                .unwrap();
            let state = juraid.get_room_state_mut(1).unwrap();

            for _ in 0..15 {
                record_monster_kill(&mut room, state, 1);
            }
            for _ in 0..12 {
                record_monster_kill(&mut room, state, 2);
            }
        }

        // Check bridges
        let opened = check_bridge_timers(&mut juraid, 10000 + 1500);
        assert_eq!(opened, vec![0, 1]);

        open_bridge_for_all_rooms(&mut juraid, 0);
        assert!(juraid
            .get_room_state(1)
            .unwrap()
            .bridges
            .is_bridge_open(0, 1));

        // Determine winner
        let state = juraid.get_room_state(1).unwrap();
        let winner = determine_winner(state);
        assert_eq!(winner, 1); // Karus: 15 kills > El Morad: 12 kills

        // Cleanup
        juraid.destroy_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::JuraidMountain), 0);
    }
}
