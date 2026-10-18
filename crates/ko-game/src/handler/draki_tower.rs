//! Draki Tower instance dungeon handler.
//! ## Architecture
//! Draki Tower is a **per-user instance dungeon** with 5 dungeons, each containing
//! multiple sub-stages. Unlike shared events (BDW, FT), each player gets their own
//! private room via the EventRoomManager.
//! ## Stage Structure (41 entries)
//! - Dungeons 1-4: 6 sub-stages each (monster/NPC alternating at sub-stages 3 and 6)
//! - Dungeon 5: 8 sub-stages (NPC stage only at sub-stage 8)
//! - NPC stages (draki_tower_npc_state=1) are rest/safe zones between combat rounds
//! ## Timers
//! - `SUB_STAGE_TIME_LIMIT` (300s): Time to clear each sub-stage
//! - `BETWEEN_STAGE_WAIT` (180s): Rest time between stages
//! - `KICK_TIMER` (20s): Grace period before teleporting player out
//! - `ROOM_CLOSE_TIMER` (7200s): Idle room cleanup
//! ## Entry Requirements
//! - Must be in nation castle (Luferson for Karus, El Morad castle for El Morad)
//! - Not in another instance dungeon
//! - Not dead
//! - No war active
//! - Certificate item (`CERTIFIKAOFDRAKI` = 810595000) required if no free entries
//! - Max 3 entries per day, resets at 18:00

use std::collections::HashMap;

use ko_db::models::draki_tower::{DrakiMonsterListRow, DrakiTowerStageRow};

// ── Constants ──────────────────────────────────────────────────────────────

pub use crate::world::types::ZONE_DRAKI_TOWER;

/// Certificate item required for entry.
pub const CERTIFIKAOFDRAKI: u32 = 810_595_000;

/// Monster kill time for spawned Draki Tower monsters (seconds).
pub const DRAKI_TOWER_MONSTER_KILL_TIME: u32 = 315;

/// Time limit per sub-stage in seconds.
pub const SUB_STAGE_TIME_LIMIT: u32 = 300;

/// Wait time between stages (rest period) in seconds.
pub const BETWEEN_STAGE_WAIT: u32 = 180;

/// Kick timer duration in seconds.
pub const KICK_TIMER: u32 = 20;

/// Room close timer in seconds (idle cleanup).
pub const ROOM_CLOSE_TIMER: u32 = 7200;

/// Maximum daily entrance limit.
pub const MAX_ENTRANCE_LIMIT: u8 = 3;

/// Achievement threshold: if tower completed in under this many seconds.
pub const ACHIEVEMENT_TIME_THRESHOLD: u32 = 1200;

/// Maximum event room index.
pub const EVENT_MAX_ROOM: u16 = 60;

// ── Sub-opcodes (under WIZ_EVENT) ──────────────────────────────────────

pub const TEMPLE_DRAKI_TOWER_ENTER: u8 = 33;
pub const TEMPLE_DRAKI_TOWER_LIST: u8 = 34;
pub const TEMPLE_DRAKI_TOWER_TIMER: u8 = 35;
pub const TEMPLE_DRAKI_TOWER_OUT1: u8 = 36;
pub const TEMPLE_DRAKI_TOWER_OUT2: u8 = 37;
pub const TEMPLE_DRAKI_TOWER_TOWN: u8 = 38;

// ── Entry error codes ──────────────────────────────────────────────────

/// Teleporting disabled / wrong zone.
pub const ENTER_ERR_TELEPORT_DISABLED: u32 = 1;
/// Already in an instance dungeon.
pub const ENTER_ERR_ALREADY_IN_INSTANCE: u32 = 2;
/// Clear info mismatch.
pub const ENTER_ERR_CLEAR_MISMATCH: u32 = 4;
/// No entrance item or limit spent.
pub const ENTER_ERR_NO_ITEM_OR_LIMIT: u32 = 5;
/// Instance generation failed.
pub const ENTER_ERR_INSTANCE_FAILED: u32 = 6;
/// Missing entrance item.
pub const ENTER_ERR_MISSING_ITEM: u32 = 7;
/// Cannot enter right now (war active).
pub const ENTER_ERR_WAR_ACTIVE: u32 = 8;
/// Cannot enter while dead.
pub const ENTER_ERR_DEAD: u32 = 9;
/// Server internal mismatch.
pub const ENTER_ERR_INTERNAL: u32 = 10;

// ── Spawn positions per dungeon ────────────────────────────────────────

/// Get the spawn (entry) position for a given dungeon level (1-5).
pub fn dungeon_spawn_position(dungeon: u8) -> (i32, i32) {
    match dungeon {
        1 => (40, 451),
        2 => (78, 58),
        3 => (315, 439),
        4 => (304, 271),
        5 => (71, 195),
        _ => (40, 451),
    }
}

// ── Class mapping ─────────────────────────────────────────────────────

/// Map game class to Draki Tower class ID.
///   Warrior(1,5,6,7,8)→1, Rogue(2,9,10)→2, Mage(3,11,12)→3,
///   Priest(4,15,16)→4, PortuKurian(13,14)→13
pub fn draki_class(game_class: u16) -> i32 {
    match game_class % 100 {
        1 | 5 | 6 | 7 | 8 => 1,
        2 | 9 | 10 => 2,
        3 | 11 | 12 => 3,
        4 | 15 | 16 => 4,
        13 | 14 => 13,
        _ => 0,
    }
}

/// Get the display name for a Draki Tower class.
pub fn draki_class_name(draki_class: i32) -> &'static str {
    match draki_class {
        1 => "Warrior",
        2 => "Rogue",
        3 => "Mage",
        4 => "Priest",
        13 => "PortuKurian",
        _ => "Unknown",
    }
}

/// Calculate max dungeon progress from a linear stage index.
pub fn max_stages_from_linear(stage: i16) -> u32 {
    let s = stage as u32;
    let base = (s.saturating_sub(s % 6)) / 6;
    let result = if !s.is_multiple_of(6) { base + 1 } else { base };
    result.max(1)
}

// ── Per-Room Instance State ────────────────────────────────────────────

/// Runtime state for a single Draki Tower room instance.
#[derive(Debug)]
pub struct DrakiTowerRoomInfo {
    /// Room ID (1-based, from EventRoomManager).
    pub room_id: u16,
    /// Current dungeon number (1-5).
    pub draki_stage: u16,
    /// Current sub-stage within the dungeon.
    pub draki_sub_stage: u16,
    /// Saved previous stage (for progress persistence).
    pub saved_draki_stage: u8,
    /// Saved max stage reached.
    pub saved_draki_max_stage: u8,
    /// Saved draki time (total elapsed seconds).
    pub saved_draki_time: u32,
    /// Saved entrance limit.
    pub saved_draki_limit: u8,
    /// Temp stage (for intermediate calculations).
    pub draki_temp_stage: u16,
    /// Temp sub-stage.
    pub draki_temp_sub_stage: u16,
    /// Monster kill counter for the current stage.
    pub draki_monster_kill: u32,
    /// Unix timestamp when the tower run started.
    pub draki_timer: u64,
    /// Unix timestamp when current sub-stage expires.
    pub draki_sub_timer: u64,
    /// Spare timer (accumulated elapsed time at rest phases).
    pub draki_spare_timer: u64,
    /// Unix timestamp when kick-out should happen.
    pub draki_out_timer: u64,
    /// Unix timestamp when town-exit should happen.
    pub draki_town_out_timer: u64,
    /// Room close countdown (decrements each second, starts at 7200).
    pub draki_room_close_timer: u32,
    /// Whether the tower run has started.
    pub tower_started: bool,
    /// Whether the out (kick) timer is active.
    pub out_timer_active: bool,
    /// Whether stage change tracking is active.
    pub is_draki_stage_change: bool,
    /// Town return request flag.
    pub town_request: bool,
    /// Town out timer active flag.
    pub town_out_timer_active: bool,
    /// User name in this room.
    pub user_name: String,
}

impl DrakiTowerRoomInfo {
    /// Create a new default room info.
    pub fn new(room_id: u16) -> Self {
        Self {
            room_id,
            draki_stage: 0,
            draki_sub_stage: 0,
            saved_draki_stage: 0,
            saved_draki_max_stage: 0,
            saved_draki_time: 0,
            saved_draki_limit: 0,
            draki_temp_stage: 0,
            draki_temp_sub_stage: 0,
            draki_monster_kill: 0,
            draki_timer: 0,
            draki_sub_timer: 0,
            draki_spare_timer: 0,
            draki_out_timer: 0,
            draki_town_out_timer: 0,
            draki_room_close_timer: ROOM_CLOSE_TIMER,
            tower_started: false,
            out_timer_active: false,
            is_draki_stage_change: false,
            town_request: false,
            town_out_timer_active: false,
            user_name: String::new(),
        }
    }

    /// Reset the room to idle state.
    ///
    pub fn reset(&mut self) {
        self.draki_stage = 0;
        self.draki_sub_stage = 0;
        self.saved_draki_stage = 0;
        self.saved_draki_max_stage = 0;
        self.saved_draki_time = 0;
        self.saved_draki_limit = 0;
        self.draki_temp_stage = 0;
        self.draki_temp_sub_stage = 0;
        self.draki_monster_kill = 0;
        self.draki_timer = 0;
        self.draki_sub_timer = 0;
        self.draki_spare_timer = 0;
        self.draki_out_timer = 0;
        self.draki_town_out_timer = 0;
        self.draki_room_close_timer = ROOM_CLOSE_TIMER;
        self.tower_started = false;
        self.out_timer_active = false;
        self.is_draki_stage_change = false;
        self.town_request = false;
        self.town_out_timer_active = false;
        self.user_name.clear();
    }
}

// ── Stage Lookup Functions ─────────────────────────────────────────────

/// Find the stage index (in the stages list) that matches the given
/// dungeon/sub-stage and NPC state.
pub fn find_stage_index(
    stages: &[DrakiTowerStageRow],
    draki_stage: u16,
    draki_sub_stage: u16,
    npc_state: i16,
) -> Option<usize> {
    stages.iter().position(|s| {
        s.draki_stage == draki_stage as i16
            && s.draki_sub_stage == draki_sub_stage as i16
            && s.draki_tower_npc_state == npc_state
    })
}

/// Get the stage row at a given index (0-based).
pub fn get_stage_at(stages: &[DrakiTowerStageRow], index: usize) -> Option<&DrakiTowerStageRow> {
    stages.get(index)
}

/// Get all monster spawn entries for a given stage ID.
///                filtering by `bDrakiStage == RoomIndex`
pub fn get_monsters_for_stage(
    monsters: &[DrakiMonsterListRow],
    stage_id: i16,
) -> Vec<&DrakiMonsterListRow> {
    monsters.iter().filter(|m| m.stage_id == stage_id).collect()
}

/// Build a lookup table: stage_id -> Vec of monster entries.
pub fn build_monster_stage_map(
    monsters: &[DrakiMonsterListRow],
) -> HashMap<i16, Vec<DrakiMonsterListRow>> {
    let mut map: HashMap<i16, Vec<DrakiMonsterListRow>> = HashMap::new();
    for m in monsters {
        map.entry(m.stage_id).or_default().push(m.clone());
    }
    map
}

// ── Entry Validation ───────────────────────────────────────────────────

/// Validate entry conditions for a Draki Tower dungeon.
/// Returns `Ok(())` if all checks pass, or `Err(error_code)` with the
/// appropriate error code to send to the client.
#[allow(clippy::too_many_arguments)]
pub fn validate_entry(
    is_dead: bool,
    is_war_open: bool,
    is_in_cinderella_event: bool,
    current_zone_id: u16,
    event_room: u16,
    nation: u8,
    is_in_nation_castle: bool,
    enter_dungeon: u8,
    item_id: u32,
    entrance_limit: u8,
    has_certificate: bool,
) -> Result<(), u32> {
    if is_war_open {
        return Err(ENTER_ERR_WAR_ACTIVE);
    }

    if is_dead {
        return Err(ENTER_ERR_DEAD);
    }

    if is_in_cinderella_event {
        return Err(ENTER_ERR_DEAD);
    }

    // Check if already in an instance zone
    let forbidden_zones: [u16; 4] = [81, 82, 83, ZONE_DRAKI_TOWER];
    if forbidden_zones.contains(&current_zone_id) || event_room > 0 {
        return Err(ENTER_ERR_ALREADY_IN_INSTANCE);
    }

    // Must be in nation castle
    if !is_in_nation_castle {
        return Err(ENTER_ERR_TELEPORT_DISABLED);
    }

    // Dungeon must be 1-5
    if !(1..=5).contains(&enter_dungeon) {
        return Err(ENTER_ERR_TELEPORT_DISABLED);
    }

    // Item validation
    if item_id != 0 && item_id != CERTIFIKAOFDRAKI {
        return Err(ENTER_ERR_NO_ITEM_OR_LIMIT);
    }

    // Must have entrance limit remaining OR certificate item
    if entrance_limit == 0 && !has_certificate {
        return Err(ENTER_ERR_MISSING_ITEM);
    }

    let _ = nation;
    Ok(())
}

/// Find a free (inactive) room from the room pool.
pub fn find_free_room(rooms: &HashMap<u16, DrakiTowerRoomInfo>) -> Option<u16> {
    for room_id in 1..=EVENT_MAX_ROOM {
        if let Some(room) = rooms.get(&room_id) {
            if !room.tower_started {
                return Some(room_id);
            }
        }
    }
    None
}

// ── Stage Progression ──────────────────────────────────────────────────

/// Result of advancing to the next stage.
#[derive(Debug, PartialEq, Eq)]
pub enum StageAdvanceResult {
    /// Monster stage: summon monsters, start sub-stage timer.
    MonsterStage { stage_index: usize, stage_id: i16 },
    /// NPC (rest) stage: spawn NPCs, wait between stages.
    NpcStage { stage_index: usize, stage_id: i16 },
    /// Tower complete (dungeon 5, sub-stage 8 cleared).
    TowerComplete {
        stage_index: usize,
        stage_id: i16,
        elapsed_seconds: u32,
    },
    /// No valid next stage found (data error).
    InvalidStage,
}

/// Advance the room to the next stage.
/// Finds the current stage index, then looks at the next stage to determine
/// if it's a monster or NPC stage.
pub fn advance_stage(
    room: &DrakiTowerRoomInfo,
    stages: &[DrakiTowerStageRow],
    now: u64,
) -> StageAdvanceResult {
    // Find current monster-stage index
    let current_index = match find_stage_index(
        stages,
        room.draki_stage,
        room.draki_sub_stage,
        0, // monster stage
    ) {
        Some(idx) => idx,
        None => return StageAdvanceResult::InvalidStage,
    };

    // Next stage is current_index + 1
    let next_index = current_index + 1;
    let next_stage = match get_stage_at(stages, next_index) {
        Some(s) => s,
        None => return StageAdvanceResult::InvalidStage,
    };

    match next_stage.draki_tower_npc_state {
        0 => StageAdvanceResult::MonsterStage {
            stage_index: next_index,
            stage_id: next_stage.id,
        },
        1 => {
            // Check if this is the final NPC stage (dungeon 5, sub 8)
            if next_stage.draki_stage == 5 && next_stage.draki_sub_stage == 8 {
                let elapsed = if room.draki_timer > 0 {
                    (now.saturating_sub(room.draki_timer)) as u32
                } else {
                    0
                };
                StageAdvanceResult::TowerComplete {
                    stage_index: next_index,
                    stage_id: next_stage.id,
                    elapsed_seconds: elapsed,
                }
            } else {
                StageAdvanceResult::NpcStage {
                    stage_index: next_index,
                    stage_id: next_stage.id,
                }
            }
        }
        _ => StageAdvanceResult::InvalidStage,
    }
}

/// Apply a monster-stage transition to the room.
pub fn apply_monster_stage(room: &mut DrakiTowerRoomInfo, stage: &DrakiTowerStageRow, now: u64) {
    room.draki_stage = stage.draki_stage as u16;
    room.draki_sub_stage = stage.draki_sub_stage as u16;
    room.draki_sub_timer = now + SUB_STAGE_TIME_LIMIT as u64;
    room.draki_timer = now.saturating_sub(room.draki_spare_timer);
    // C++ ChangeDrakiMode: m_bSavedDrakiStage = m_RoomID
    room.saved_draki_stage = stage.id as u8;
}

/// Apply an NPC-stage (rest) transition to the room.
pub fn apply_npc_stage(
    room: &mut DrakiTowerRoomInfo,
    stage: &DrakiTowerStageRow,
    now: u64,
    is_final: bool,
) {
    room.draki_stage = stage.draki_stage as u16;
    room.draki_sub_stage = stage.draki_sub_stage as u16;
    room.draki_sub_timer = now + BETWEEN_STAGE_WAIT as u64;
    room.draki_spare_timer = now.saturating_sub(room.draki_timer);
    // C++ ChangeDrakiMode: m_bSavedDrakiStage = m_RoomID
    room.saved_draki_stage = stage.id as u8;

    if is_final {
        // Tower complete — no further timer, use special sentinel
        room.draki_sub_timer = u64::MAX;
    }
}

/// Initialize the room for entry (first sub-stage of selected dungeon).
pub fn initialize_room_for_entry(
    room: &mut DrakiTowerRoomInfo,
    enter_dungeon: u8,
    user_name: &str,
    saved_stage: u8,
    now: u64,
) {
    room.draki_timer = now;
    room.draki_sub_timer = now + SUB_STAGE_TIME_LIMIT as u64;
    room.draki_temp_stage = saved_stage as u16;
    room.draki_stage = enter_dungeon as u16;
    room.draki_sub_stage = 1;
    room.tower_started = true;
    room.draki_room_close_timer = ROOM_CLOSE_TIMER;
    room.user_name = user_name.to_string();

    if enter_dungeon == 1 {
        room.is_draki_stage_change = true;
    }
}

// ── Timer Tick ─────────────────────────────────────────────────────────

/// Result of a timer tick for a single Draki Tower room.
#[derive(Debug, PartialEq, Eq)]
pub enum DrakiTickResult {
    /// No action needed.
    Idle,
    /// Sub-stage timer expired — kick the player.
    SubStageExpired,
    /// Out timer expired — teleport player to town.
    KickOut,
    /// Town out timer expired — teleport player to town.
    TownOut,
}

/// Process one timer tick for a Draki Tower room.
/// Called every second for each active room.
pub fn room_timer_tick(room: &DrakiTowerRoomInfo, now: u64) -> DrakiTickResult {
    if !room.tower_started {
        return DrakiTickResult::Idle;
    }

    // Town out timer check (player requested exit)
    if room.town_out_timer_active && room.draki_town_out_timer <= now {
        return DrakiTickResult::TownOut;
    }

    // Sub-stage timer expired (use <= to avoid missed ticks)
    // Guard: only fire if out_timer is not yet active (prevents re-trigger)
    if !room.out_timer_active && room.draki_sub_timer > 0 && room.draki_sub_timer <= now {
        return DrakiTickResult::SubStageExpired;
    }

    // Kick-out timer check
    if room.out_timer_active && room.draki_out_timer <= now {
        return DrakiTickResult::KickOut;
    }

    DrakiTickResult::Idle
}

/// Process room close timer tick.
/// Returns true if the room should be closed (timer reached 0).
pub fn room_close_tick(room: &mut DrakiTowerRoomInfo) -> bool {
    if !room.tower_started {
        return false;
    }

    if room.draki_room_close_timer > 0 {
        room.draki_room_close_timer -= 1;
    }

    room.draki_room_close_timer == 0
}

/// Apply kick-out state to the room.
pub fn apply_kickout(room: &mut DrakiTowerRoomInfo, now: u64) {
    // The final NPC can be clicked repeatedly while OUT1 is visible.  Never
    // extend an evacuation that is already counting down; otherwise each click
    // postpones the exit another 20 seconds and can keep the room alive.
    if !room.out_timer_active {
        room.draki_out_timer = now + KICK_TIMER as u64;
        room.out_timer_active = true;
    }
}

/// Apply town-return state to the room.
pub fn apply_town_return(room: &mut DrakiTowerRoomInfo, now: u64) {
    room.draki_town_out_timer = now + KICK_TIMER as u64;
    room.town_out_timer_active = true;
    room.town_request = true;
}

/// Get the exit zone for a player based on their level.
///                others go to nation zone.
pub fn get_exit_zone(level: u16, nation: u8) -> u16 {
    if (1..=34).contains(&level) {
        21 // ZONE_MORADON
    } else {
        nation as u16
    }
}

/// Check if it's time for the daily entrance limit reset (18:00).
pub fn should_reset_limits(hour: u32, minute: u32, second: u32) -> bool {
    hour == 18 && minute == 0 && second == 0
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stages() -> Vec<DrakiTowerStageRow> {
        vec![
            DrakiTowerStageRow {
                id: 1,
                draki_stage: 1,
                draki_sub_stage: 1,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 2,
                draki_stage: 1,
                draki_sub_stage: 2,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 3,
                draki_stage: 1,
                draki_sub_stage: 3,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 4,
                draki_stage: 1,
                draki_sub_stage: 3,
                draki_tower_npc_state: 1,
            },
            DrakiTowerStageRow {
                id: 5,
                draki_stage: 1,
                draki_sub_stage: 4,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 6,
                draki_stage: 1,
                draki_sub_stage: 5,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 7,
                draki_stage: 1,
                draki_sub_stage: 6,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 8,
                draki_stage: 1,
                draki_sub_stage: 6,
                draki_tower_npc_state: 1,
            },
            // Dungeon 5 final stages for tower-complete test
            DrakiTowerStageRow {
                id: 40,
                draki_stage: 5,
                draki_sub_stage: 8,
                draki_tower_npc_state: 0,
            },
            DrakiTowerStageRow {
                id: 41,
                draki_stage: 5,
                draki_sub_stage: 8,
                draki_tower_npc_state: 1,
            },
        ]
    }

    fn make_monsters() -> Vec<DrakiMonsterListRow> {
        vec![
            DrakiMonsterListRow {
                id: 1,
                stage_id: 1,
                monster_id: 9728,
                pos_x: 51,
                pos_z: 449,
                s_direction: 0,
                is_monster: false,
            },
            DrakiMonsterListRow {
                id: 2,
                stage_id: 1,
                monster_id: 9728,
                pos_x: 57,
                pos_z: 442,
                s_direction: 0,
                is_monster: false,
            },
            DrakiMonsterListRow {
                id: 3,
                stage_id: 1,
                monster_id: 9728,
                pos_x: 65,
                pos_z: 437,
                s_direction: 0,
                is_monster: false,
            },
            DrakiMonsterListRow {
                id: 17,
                stage_id: 4,
                monster_id: 25257,
                pos_x: 58,
                pos_z: 431,
                s_direction: 0,
                is_monster: true,
            },
        ]
    }

    // ── Room State Tests ──────────────────────────────────────────────

    #[test]
    fn test_room_info_new() {
        let room = DrakiTowerRoomInfo::new(1);
        assert_eq!(room.room_id, 1);
        assert!(!room.tower_started);
        assert_eq!(room.draki_stage, 0);
        assert_eq!(room.draki_sub_stage, 0);
        assert_eq!(room.draki_room_close_timer, ROOM_CLOSE_TIMER);
        assert!(room.user_name.is_empty());
    }

    #[test]
    fn test_room_info_reset() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.tower_started = true;
        room.draki_stage = 3;
        room.draki_sub_stage = 5;
        room.draki_timer = 1000;
        room.user_name = "TestUser".to_string();

        room.reset();

        assert!(!room.tower_started);
        assert_eq!(room.draki_stage, 0);
        assert_eq!(room.draki_sub_stage, 0);
        assert_eq!(room.draki_timer, 0);
        assert!(room.user_name.is_empty());
        assert_eq!(room.draki_room_close_timer, ROOM_CLOSE_TIMER);
    }

    // ── Spawn Position Tests ──────────────────────────────────────────

    #[test]
    fn test_dungeon_spawn_positions() {
        assert_eq!(dungeon_spawn_position(1), (40, 451));
        assert_eq!(dungeon_spawn_position(2), (78, 58));
        assert_eq!(dungeon_spawn_position(3), (315, 439));
        assert_eq!(dungeon_spawn_position(4), (304, 271));
        assert_eq!(dungeon_spawn_position(5), (71, 195));
        // Invalid defaults to dungeon 1 position
        assert_eq!(dungeon_spawn_position(0), (40, 451));
        assert_eq!(dungeon_spawn_position(6), (40, 451));
    }

    // ── Entry Validation Tests ────────────────────────────────────────

    #[test]
    fn test_validate_entry_success() {
        let result = validate_entry(false, false, false, 21, 0, 1, true, 1, 0, 3, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_entry_war_active() {
        let result = validate_entry(false, true, false, 21, 0, 1, true, 1, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_WAR_ACTIVE));
    }

    #[test]
    fn test_validate_entry_dead() {
        let result = validate_entry(true, false, false, 21, 0, 1, true, 1, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_DEAD));
    }

    #[test]
    fn test_validate_entry_already_in_instance() {
        // In ZONE_DRAKI_TOWER
        let result = validate_entry(
            false,
            false,
            false,
            ZONE_DRAKI_TOWER,
            0,
            1,
            true,
            1,
            0,
            3,
            true,
        );
        assert_eq!(result, Err(ENTER_ERR_ALREADY_IN_INSTANCE));

        // Has event_room > 0
        let result = validate_entry(false, false, false, 21, 1, 1, true, 1, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_ALREADY_IN_INSTANCE));
    }

    #[test]
    fn test_validate_entry_wrong_zone() {
        let result = validate_entry(false, false, false, 21, 0, 1, false, 1, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_TELEPORT_DISABLED));
    }

    #[test]
    fn test_validate_entry_invalid_dungeon() {
        let result = validate_entry(false, false, false, 21, 0, 1, true, 0, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_TELEPORT_DISABLED));

        let result = validate_entry(false, false, false, 21, 0, 1, true, 6, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_TELEPORT_DISABLED));
    }

    #[test]
    fn test_validate_entry_bad_item() {
        let result = validate_entry(false, false, false, 21, 0, 1, true, 1, 999, 3, true);
        assert_eq!(result, Err(ENTER_ERR_NO_ITEM_OR_LIMIT));
    }

    #[test]
    fn test_validate_entry_no_limit_no_cert() {
        let result = validate_entry(false, false, false, 21, 0, 1, true, 1, 0, 0, false);
        assert_eq!(result, Err(ENTER_ERR_MISSING_ITEM));
    }

    #[test]
    fn test_validate_entry_no_limit_but_has_cert() {
        let result = validate_entry(false, false, false, 21, 0, 1, true, 1, 0, 0, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_entry_has_limit_no_cert() {
        let result = validate_entry(false, false, false, 21, 0, 1, true, 1, 0, 2, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_entry_cinderella_blocked() {
        let result = validate_entry(false, false, true, 21, 0, 1, true, 1, 0, 3, true);
        assert_eq!(result, Err(ENTER_ERR_DEAD));
    }

    // ── Stage Lookup Tests ────────────────────────────────────────────

    #[test]
    fn test_find_stage_index_monster() {
        let stages = make_stages();
        let idx = find_stage_index(&stages, 1, 1, 0);
        assert_eq!(idx, Some(0));

        let idx = find_stage_index(&stages, 1, 3, 0);
        assert_eq!(idx, Some(2));

        let idx = find_stage_index(&stages, 1, 3, 1);
        assert_eq!(idx, Some(3));

        // Non-existent
        let idx = find_stage_index(&stages, 1, 7, 0);
        assert_eq!(idx, None);
    }

    #[test]
    fn test_get_monsters_for_stage() {
        let monsters = make_monsters();
        let stage1 = get_monsters_for_stage(&monsters, 1);
        assert_eq!(stage1.len(), 3);
        assert!(stage1.iter().all(|m| m.monster_id == 9728));

        let stage4 = get_monsters_for_stage(&monsters, 4);
        assert_eq!(stage4.len(), 1);
        assert_eq!(stage4[0].monster_id, 25257);
        assert!(stage4[0].is_monster);

        let stage99 = get_monsters_for_stage(&monsters, 99);
        assert!(stage99.is_empty());
    }

    #[test]
    fn test_build_monster_stage_map() {
        let monsters = make_monsters();
        let map = build_monster_stage_map(&monsters);
        assert_eq!(map.len(), 2); // stages 1 and 4
        assert_eq!(map.get(&1).unwrap().len(), 3);
        assert_eq!(map.get(&4).unwrap().len(), 1);
    }

    // ── Stage Advancement Tests ──────────────────────────────────────

    #[test]
    fn test_advance_stage_to_monster() {
        let stages = make_stages();
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_stage = 1;
        room.draki_sub_stage = 1;
        room.draki_timer = 1000;

        let result = advance_stage(&room, &stages, 1300);
        match result {
            StageAdvanceResult::MonsterStage {
                stage_index,
                stage_id,
            } => {
                assert_eq!(stage_index, 1);
                assert_eq!(stage_id, 2); // id=2 is (1, 2, 0)
            }
            _ => panic!("Expected MonsterStage, got {:?}", result),
        }
    }

    #[test]
    fn test_advance_stage_to_npc() {
        let stages = make_stages();
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_stage = 1;
        room.draki_sub_stage = 3; // Next after (1,3,0) is (1,3,1) NPC
        room.draki_timer = 1000;

        let result = advance_stage(&room, &stages, 1300);
        match result {
            StageAdvanceResult::NpcStage {
                stage_index,
                stage_id,
            } => {
                assert_eq!(stage_index, 3);
                assert_eq!(stage_id, 4); // id=4 is (1, 3, 1)
            }
            _ => panic!("Expected NpcStage, got {:?}", result),
        }
    }

    #[test]
    fn test_advance_stage_tower_complete() {
        let stages = make_stages();
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_stage = 5;
        room.draki_sub_stage = 8; // Final monster stage
        room.draki_timer = 1000;

        let result = advance_stage(&room, &stages, 2200);
        match result {
            StageAdvanceResult::TowerComplete {
                stage_index,
                stage_id,
                elapsed_seconds,
            } => {
                assert_eq!(elapsed_seconds, 1200);
                assert!(stage_index > 0);
                assert!(stage_id > 0);
            }
            _ => panic!("Expected TowerComplete, got {:?}", result),
        }
    }

    #[test]
    fn test_advance_stage_invalid() {
        let stages = make_stages();
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_stage = 99;
        room.draki_sub_stage = 1;

        let result = advance_stage(&room, &stages, 1300);
        assert_eq!(result, StageAdvanceResult::InvalidStage);
    }

    // ── Apply Stage Transition Tests ─────────────────────────────────

    #[test]
    fn test_apply_monster_stage() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_spare_timer = 50;
        let stage = DrakiTowerStageRow {
            id: 5,
            draki_stage: 1,
            draki_sub_stage: 4,
            draki_tower_npc_state: 0,
        };

        apply_monster_stage(&mut room, &stage, 1000);

        assert_eq!(room.draki_stage, 1);
        assert_eq!(room.draki_sub_stage, 4);
        assert_eq!(room.draki_sub_timer, 1000 + SUB_STAGE_TIME_LIMIT as u64);
        assert_eq!(room.draki_timer, 950); // 1000 - 50
    }

    #[test]
    fn test_apply_npc_stage() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_timer = 900;
        let stage = DrakiTowerStageRow {
            id: 4,
            draki_stage: 1,
            draki_sub_stage: 3,
            draki_tower_npc_state: 1,
        };

        apply_npc_stage(&mut room, &stage, 1000, false);

        assert_eq!(room.draki_stage, 1);
        assert_eq!(room.draki_sub_stage, 3);
        assert_eq!(room.draki_sub_timer, 1000 + BETWEEN_STAGE_WAIT as u64);
        assert_eq!(room.draki_spare_timer, 100); // 1000 - 900
    }

    #[test]
    fn test_apply_npc_stage_final() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.draki_timer = 900;
        let stage = DrakiTowerStageRow {
            id: 41,
            draki_stage: 5,
            draki_sub_stage: 8,
            draki_tower_npc_state: 1,
        };

        apply_npc_stage(&mut room, &stage, 1000, true);

        assert_eq!(room.draki_sub_timer, u64::MAX);
    }

    // ── Room Initialization Tests ────────────────────────────────────

    #[test]
    fn test_initialize_room_for_entry() {
        let mut room = DrakiTowerRoomInfo::new(5);
        initialize_room_for_entry(&mut room, 3, "TestPlayer", 0, 5000);

        assert_eq!(room.draki_stage, 3);
        assert_eq!(room.draki_sub_stage, 1);
        assert!(room.tower_started);
        assert_eq!(room.draki_timer, 5000);
        assert_eq!(room.draki_sub_timer, 5000 + SUB_STAGE_TIME_LIMIT as u64);
        assert_eq!(room.user_name, "TestPlayer");
        assert_eq!(room.draki_room_close_timer, ROOM_CLOSE_TIMER);
        assert!(!room.is_draki_stage_change); // dungeon 3, not 1
    }

    #[test]
    fn test_initialize_room_dungeon1_sets_stage_change() {
        let mut room = DrakiTowerRoomInfo::new(1);
        initialize_room_for_entry(&mut room, 1, "Warrior", 0, 5000);
        assert!(room.is_draki_stage_change);
    }

    // ── Timer Tick Tests ─────────────────────────────────────────────

    #[test]
    fn test_room_timer_tick_idle() {
        let room = DrakiTowerRoomInfo::new(1);
        assert_eq!(room_timer_tick(&room, 1000), DrakiTickResult::Idle);
    }

    #[test]
    fn test_room_timer_tick_substage_expired() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.tower_started = true;
        room.draki_sub_timer = 1300;

        assert_eq!(room_timer_tick(&room, 1299), DrakiTickResult::Idle);
        assert_eq!(
            room_timer_tick(&room, 1300),
            DrakiTickResult::SubStageExpired
        );
        // Still fires if tick was missed (<=)
        assert_eq!(
            room_timer_tick(&room, 1301),
            DrakiTickResult::SubStageExpired
        );
        // After apply_kickout sets out_timer_active, sub-stage won't re-trigger
        apply_kickout(&mut room, 1301); // sets out_timer_active=true, out_timer=1321
        assert_eq!(room_timer_tick(&room, 1302), DrakiTickResult::Idle);
    }

    #[test]
    fn test_room_timer_tick_kickout() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.tower_started = true;
        room.out_timer_active = true;
        room.draki_out_timer = 1320;
        room.draki_sub_timer = u64::MAX;

        assert_eq!(room_timer_tick(&room, 1319), DrakiTickResult::Idle);
        assert_eq!(room_timer_tick(&room, 1320), DrakiTickResult::KickOut);
    }

    #[test]
    fn test_room_timer_tick_town_out() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.tower_started = true;
        room.town_out_timer_active = true;
        room.draki_town_out_timer = 1320;
        room.draki_sub_timer = u64::MAX;

        assert_eq!(room_timer_tick(&room, 1319), DrakiTickResult::Idle);
        assert_eq!(room_timer_tick(&room, 1320), DrakiTickResult::TownOut);
    }

    #[test]
    fn test_room_timer_tick_town_out_before_kick() {
        // Town out should be checked before kick out
        let mut room = DrakiTowerRoomInfo::new(1);
        room.tower_started = true;
        room.town_out_timer_active = true;
        room.draki_town_out_timer = 1320;
        room.out_timer_active = true;
        room.draki_out_timer = 1320;
        room.draki_sub_timer = u64::MAX;

        assert_eq!(room_timer_tick(&room, 1320), DrakiTickResult::TownOut);
    }

    // ── Room Close Timer Tests ──────────────────────────────────────

    #[test]
    fn test_room_close_tick_inactive() {
        let mut room = DrakiTowerRoomInfo::new(1);
        assert!(!room_close_tick(&mut room));
    }

    #[test]
    fn test_room_close_tick_countdown() {
        let mut room = DrakiTowerRoomInfo::new(1);
        room.tower_started = true;
        room.draki_room_close_timer = 3;

        assert!(!room_close_tick(&mut room)); // 3 -> 2
        assert_eq!(room.draki_room_close_timer, 2);
        assert!(!room_close_tick(&mut room)); // 2 -> 1
        assert_eq!(room.draki_room_close_timer, 1);
        // Timer reaches 0 — room should close
        assert!(room_close_tick(&mut room)); // 1 -> 0
        assert_eq!(room.draki_room_close_timer, 0);
    }

    // ── Apply State Tests ───────────────────────────────────────────

    #[test]
    fn test_apply_kickout() {
        let mut room = DrakiTowerRoomInfo::new(1);
        apply_kickout(&mut room, 1000);
        assert!(room.out_timer_active);
        assert_eq!(room.draki_out_timer, 1000 + KICK_TIMER as u64);

        apply_kickout(&mut room, 1010);
        assert_eq!(
            room.draki_out_timer,
            1000 + KICK_TIMER as u64,
            "re-clicking the final NPC must not extend the active exit timer"
        );
    }

    #[test]
    fn test_apply_town_return() {
        let mut room = DrakiTowerRoomInfo::new(1);
        apply_town_return(&mut room, 1000);
        assert!(room.town_out_timer_active);
        assert!(room.town_request);
        assert_eq!(room.draki_town_out_timer, 1000 + KICK_TIMER as u64);
    }

    // ── Exit Zone Tests ─────────────────────────────────────────────

    #[test]
    fn test_get_exit_zone() {
        // Low level goes to Moradon
        assert_eq!(get_exit_zone(1, 1), 21);
        assert_eq!(get_exit_zone(34, 2), 21);
        // High level goes to nation zone
        assert_eq!(get_exit_zone(35, 1), 1);
        assert_eq!(get_exit_zone(83, 2), 2);
    }

    // ── Limit Reset Tests ───────────────────────────────────────────

    #[test]
    fn test_should_reset_limits() {
        assert!(should_reset_limits(18, 0, 0));
        assert!(!should_reset_limits(17, 59, 59));
        assert!(!should_reset_limits(18, 0, 1));
        assert!(!should_reset_limits(18, 1, 0));
        assert!(!should_reset_limits(0, 0, 0));
    }

    // ── Find Free Room Tests ────────────────────────────────────────

    #[test]
    fn test_find_free_room_empty() {
        let rooms = HashMap::new();
        assert_eq!(find_free_room(&rooms), None);
    }

    #[test]
    fn test_find_free_room_available() {
        let mut rooms = HashMap::new();
        let mut r1 = DrakiTowerRoomInfo::new(1);
        r1.tower_started = true;
        rooms.insert(1, r1);
        rooms.insert(2, DrakiTowerRoomInfo::new(2));

        assert_eq!(find_free_room(&rooms), Some(2));
    }

    #[test]
    fn test_find_free_room_all_busy() {
        let mut rooms = HashMap::new();
        for i in 1..=3 {
            let mut r = DrakiTowerRoomInfo::new(i);
            r.tower_started = true;
            rooms.insert(i, r);
        }
        assert_eq!(find_free_room(&rooms), None);
    }

    // ── Constants Tests ─────────────────────────────────────────────

    #[test]
    fn test_constants() {
        assert_eq!(ZONE_DRAKI_TOWER, 95);
        assert_eq!(CERTIFIKAOFDRAKI, 810_595_000);
        assert_eq!(DRAKI_TOWER_MONSTER_KILL_TIME, 315);
        assert_eq!(SUB_STAGE_TIME_LIMIT, 300);
        assert_eq!(BETWEEN_STAGE_WAIT, 180);
        assert_eq!(KICK_TIMER, 20);
        assert_eq!(ROOM_CLOSE_TIMER, 7200);
        assert_eq!(MAX_ENTRANCE_LIMIT, 3);
        assert_eq!(ACHIEVEMENT_TIME_THRESHOLD, 1200);
        assert_eq!(EVENT_MAX_ROOM, 60);
    }

    #[test]
    fn test_sub_opcodes() {
        assert_eq!(TEMPLE_DRAKI_TOWER_ENTER, 33);
        assert_eq!(TEMPLE_DRAKI_TOWER_LIST, 34);
        assert_eq!(TEMPLE_DRAKI_TOWER_TIMER, 35);
        assert_eq!(TEMPLE_DRAKI_TOWER_OUT1, 36);
        assert_eq!(TEMPLE_DRAKI_TOWER_OUT2, 37);
        assert_eq!(TEMPLE_DRAKI_TOWER_TOWN, 38);
    }

    // ── Class Mapping Tests ─────────────────────────────────────────────

    #[test]
    fn test_draki_class_warrior() {
        assert_eq!(draki_class(101), 1); // Warrior base
        assert_eq!(draki_class(105), 1); // Warrior sub 5
        assert_eq!(draki_class(106), 1); // Warrior sub 6
        assert_eq!(draki_class(107), 1); // Warrior sub 7
        assert_eq!(draki_class(108), 1); // Warrior sub 8
        assert_eq!(draki_class(201), 1); // Nation 2 Warrior
    }

    #[test]
    fn test_draki_class_rogue() {
        assert_eq!(draki_class(102), 2);
        assert_eq!(draki_class(109), 2);
        assert_eq!(draki_class(110), 2);
    }

    #[test]
    fn test_draki_class_mage() {
        assert_eq!(draki_class(103), 3);
        assert_eq!(draki_class(111), 3);
        assert_eq!(draki_class(112), 3);
    }

    #[test]
    fn test_draki_class_priest() {
        assert_eq!(draki_class(104), 4);
        assert_eq!(draki_class(115), 4);
        assert_eq!(draki_class(116), 4);
    }

    #[test]
    fn test_draki_class_portu_kurian() {
        assert_eq!(draki_class(113), 13);
        assert_eq!(draki_class(114), 13);
    }

    #[test]
    fn test_draki_class_unknown() {
        assert_eq!(draki_class(0), 0);
        assert_eq!(draki_class(99), 0);
    }

    #[test]
    fn test_draki_class_name_all() {
        assert_eq!(draki_class_name(1), "Warrior");
        assert_eq!(draki_class_name(2), "Rogue");
        assert_eq!(draki_class_name(3), "Mage");
        assert_eq!(draki_class_name(4), "Priest");
        assert_eq!(draki_class_name(13), "PortuKurian");
        assert_eq!(draki_class_name(0), "Unknown");
    }

    // ── max_stages_from_linear Tests ────────────────────────────────────

    #[test]
    fn test_max_stages_from_linear() {
        // C++ formula: (s - (s % 6)) / 6 + (s % 6 > 0 ? 1 : 0)
        assert_eq!(max_stages_from_linear(0), 1); // C++ floor: max(1, 0) = 1
        assert_eq!(max_stages_from_linear(1), 1); // dungeon 1 started
        assert_eq!(max_stages_from_linear(6), 1); // dungeon 1 complete
        assert_eq!(max_stages_from_linear(7), 2); // dungeon 2 started
        assert_eq!(max_stages_from_linear(12), 2); // dungeon 2 complete
        assert_eq!(max_stages_from_linear(30), 5); // dungeon 5 complete
        assert_eq!(max_stages_from_linear(41), 7); // beyond (full clear)
    }

    // ── saved_draki_stage update Tests ──────────────────────────────────

    #[test]
    fn test_apply_monster_stage_updates_saved() {
        let mut room = DrakiTowerRoomInfo::new(1);
        let stage = DrakiTowerStageRow {
            id: 5,
            draki_stage: 2,
            draki_sub_stage: 1,
            draki_tower_npc_state: 0,
        };
        apply_monster_stage(&mut room, &stage, 1000);
        assert_eq!(room.saved_draki_stage, 5);
    }

    #[test]
    fn test_apply_npc_stage_updates_saved() {
        let mut room = DrakiTowerRoomInfo::new(1);
        let stage = DrakiTowerStageRow {
            id: 8,
            draki_stage: 2,
            draki_sub_stage: 3,
            draki_tower_npc_state: 1,
        };
        apply_npc_stage(&mut room, &stage, 1000, false);
        assert_eq!(room.saved_draki_stage, 8);
    }
}
