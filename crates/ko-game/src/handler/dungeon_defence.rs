//! Dungeon Defence (Full Moon Rift) instance handler.
//! ## Instance Lifecycle
//! 1. **Sign**: Party leader initiates entry via `DungeonDefenceSign()`.
//!    Validates party (all in Moradon, alive, have rift voucher), determines difficulty.
//! 2. **Room Assign**: Finds a free DD room, sets initial state, teleports party in.
//! 3. **Guardian Spawn**: 6 guardian NPCs (31737-31740) are spawned in the room.
//! 4. **Initial Timer**: 60s countdown before first monster wave.
//! 5. **Stage Loop**: `DungeonDefenceTimer()` ticks every 1s, spawning monster waves.
//!    After all monsters in a stage are killed, `ChangeDungeonDefenceStage()` advances.
//! 6. **Finish**: After the final stage boss dies, 30s finish timer starts.
//! 7. **Kick**: All players teleported back to Moradon, room reset.
//! ## Difficulty Levels
//! | Difficulty | Party Size | Stages    | Stage IDs |
//! |------------|------------|-----------|-----------|
//! | Easy (1)   | 2-3        | 5 waves   | 1-5       |
//! | Normal (2) | 4-7        | 15 waves  | 1-15      |
//! | Hard (3)   | 8+         | 20 waves  | 1-20      |
//! Waves 1-5 Castle Army 1, 6-10 Castle Army 2, 11-15 Draki Army, 16-20 Bifrost Army;
//! the last wave of each army holds that army's boss.
//! ## Rewards (per monster kill)
//! - **Full Moon Rift Jewel**: 1 for Castle/Draki, 2 for the Bifrost Army, none for Astaroth (killer only)
//! - **Monster Coin**: 2 for killer, 1 for each other party member
//! - **Lunar Order Token**: 1 per party member when boss dies (10000, 10005, 10011, 10019)

use std::collections::HashMap;
use rand::Rng;
use std::sync::atomic::{AtomicBool, AtomicI16, AtomicU16, AtomicU32, Ordering};

use ko_db::models::dungeon_defence::{DfMonsterRow, DfStageRow};

// ── Zone & Item Constants ──────────────────────────────────────────────

pub use crate::world::types::ZONE_DUNGEON_DEFENCE;

/// Full Moon Rift Voucher item (required to enter).
pub const DUNGEON_DEFENCE_RIFT_ITEM: u32 = 914057000;

/// Monster Coin item (reward per kill).
pub const MONSTER_COIN_ITEM: u32 = 914058000;

/// Full Moon Rift Jewel item (reward per kill, 1 or 2 depending on stage).
pub const MONSTER_RIFT_JEWEL: u32 = 914069000;

/// Lunar Order Token item (boss-kill reward for whole party).
pub const LUNAR_ORDER_TOKEN: u32 = 810977000;

/// Maximum number of DD rooms available.
pub const DD_MAX_ROOMS: u16 = 60;

/// Initial spawn timer (seconds before first wave).
pub const DD_INITIAL_SPAWN_TIME: u32 = 60;

/// Room close timer (seconds before forced eviction).
pub const DD_ROOM_CLOSE_TIME: u32 = 7200;

/// Finish timer (seconds after final stage before kick).
pub const DD_FINISH_TIME: u16 = 30;

/// WIZ_EVENT sub-opcode for dungeon defence sign.
pub const TEMPLE_EVENT_DUNGEON_SIGN: u8 = 58;

/// WIZ_EVENT sub-opcode for stage counter display.
pub const TEMPLE_EVENT_STAGE_COUNTER: u8 = 60;

// ── Guardian NPC Templates ─────────────────────────────────────────────

/// Guardian NPC spawn definitions.
/// Format: (npc_id, pos_x, pos_z)
pub const GUARDIAN_NPCS: [(u16, i16, i16); 6] = [
    (31737, 199, 200),
    (31737, 52, 124),
    (31737, 211, 38),
    (31740, 167, 212),
    (31739, 65, 134),
    (31738, 196, 54),
];

// ── Boss Monster IDs ───────────────────────────────────────────────────

/// Boss monster proto IDs that grant Lunar Order Token on death.
pub const BOSS_MONSTER_IDS: [u16; 4] = [10000, 10005, 10011, 10019];

/// Astaroth: a suicide monster that only targets the towers; gives no coin and no jewel.
pub const ASTAROTH: u16 = 10001;

/// Gluttony: a monster that magic cannot hurt.
pub const GLUTTONY: u16 = 10013;

/// First wave of the Bifrost Army (kills there give 2 jewels).
pub const BIFROST_FIRST_STAGE: i16 = 16;

/// All valid DD monster proto IDs (for reward eligibility).
pub const DD_MONSTER_IDS: [u16; 23] = [
    9997, 9998, 9999, 10000, 10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010,
    10011, 10012, 10013, 10014, 10015, 10016, 10017, 10018, 10019,
];

// ── Sign Result Codes ──────────────────────────────────────────────────

/// Error/result codes sent in the TEMPLE_EVENT_DUNGEON_SIGN response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DdSignResult {
    /// System error / instance generation failed.
    SystemError = 0,
    /// Generic failure (no room, already in event, etc.).
    Failed = 1,
    /// Party members not all in Moradon.
    NotInMoradon = 2,
    /// Another instance is being created.
    InstanceBusy = 3,
    /// A party member is dead.
    MemberDead = 4,
    /// Not in a party.
    NoParty = 5,
    /// Not a Full Moon Rift party.
    WrongPartyType = 6,
    /// A party member lacks the rift voucher.
    NoRiftItem = 7,
}

// ── Difficulty Enum ────────────────────────────────────────────────────

/// Dungeon Defence difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DdDifficulty {
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl DdDifficulty {
    /// Determine difficulty from party member count.
    ///
    pub fn from_party_size(count: u16) -> Option<Self> {
        match count {
            2..=3 => Some(Self::Easy),
            4..=7 => Some(Self::Normal),
            8.. => Some(Self::Hard),
            _ => None,
        }
    }

    /// Get the starting stage ID for this difficulty.
    ///
    pub fn starting_stage(&self) -> i16 {
        match self {
            Self::Easy | Self::Normal | Self::Hard => 1,
        }
    }

    /// Get the final stage ID for this difficulty.
    ///
    pub fn final_stage(&self) -> i16 {
        match self {
            Self::Easy => 5,
            Self::Normal => 15,
            Self::Hard => 20,
        }
    }

    /// Get the maximum number of stages in this difficulty.
    ///
    pub fn max_stages(&self) -> u8 {
        match self {
            Self::Easy => 5,
            Self::Normal => 15,
            Self::Hard => 20,
        }
    }

    /// Get the stage offset to compute the display stage number.
    ///
    ///   Easy: stage_id directly, Normal: stage_id - 5, Hard: stage_id - 17
    pub fn stage_offset(&self) -> i16 {
        0
    }
}

// ── Room State ─────────────────────────────────────────────────────────

/// Runtime state for a single Dungeon Defence room instance.
pub struct DdRoomInfo {
    /// Room ID (1-based).
    pub room_id: AtomicU16,
    /// Whether this room is currently in use.
    pub is_started: AtomicBool,
    /// Current difficulty level.
    pub difficulty: AtomicU16,
    /// Current stage ID.
    pub stage_id: AtomicI16,
    /// Kill count tracking (for stage advancement).
    pub kill_count: AtomicU32,
    /// Seconds until next monster spawn.
    pub spawn_time: AtomicU32,
    /// Seconds until room forced close (2 hours).
    pub room_close: AtomicU32,
    /// Seconds until players are kicked after finishing.
    pub out_time: AtomicU16,
    /// Whether the initial spawn countdown is active.
    pub monster_beginner_spawned: AtomicBool,
    /// Whether monsters have been spawned and we're waiting for kills.
    pub monster_spawned: AtomicBool,
    /// Whether the event is finished (all stages cleared).
    pub is_finished: AtomicBool,
    /// User list in this room (character names).
    pub users: parking_lot::RwLock<HashMap<usize, String>>,
    /// Deaths used per player (3 allowed).
    pub deaths: parking_lot::RwLock<HashMap<u16, u8>>,
    /// GM-spawned test bots defending this room.
    pub bots: parking_lot::RwLock<Vec<u32>>,
}

impl DdRoomInfo {
    /// Create a new empty room.
    pub fn new(room_id: u16) -> Self {
        Self {
            room_id: AtomicU16::new(room_id),
            is_started: AtomicBool::new(false),
            difficulty: AtomicU16::new(0),
            stage_id: AtomicI16::new(0),
            kill_count: AtomicU32::new(0),
            spawn_time: AtomicU32::new(DD_INITIAL_SPAWN_TIME),
            room_close: AtomicU32::new(DD_ROOM_CLOSE_TIME),
            out_time: AtomicU16::new(0),
            monster_beginner_spawned: AtomicBool::new(false),
            monster_spawned: AtomicBool::new(false),
            is_finished: AtomicBool::new(false),
            users: parking_lot::RwLock::new(HashMap::new()),
            deaths: parking_lot::RwLock::new(HashMap::new()),
            bots: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Reset the room to idle state.
    ///
    pub fn reset(&self) {
        self.kill_count.store(0, Ordering::Relaxed);
        self.spawn_time
            .store(DD_INITIAL_SPAWN_TIME, Ordering::Relaxed);
        self.room_close.store(DD_ROOM_CLOSE_TIME, Ordering::Relaxed);
        self.out_time.store(0, Ordering::Relaxed);
        self.is_started.store(false, Ordering::Relaxed);
        self.monster_beginner_spawned
            .store(false, Ordering::Relaxed);
        self.monster_spawned.store(false, Ordering::Relaxed);
        self.is_finished.store(false, Ordering::Relaxed);
        self.stage_id.store(0, Ordering::Relaxed);
        self.difficulty.store(0, Ordering::Relaxed);
        self.users.write().clear();
        self.deaths.write().clear();
        self.bots.write().clear();
    }

    /// Atomically claim and initialize a room for a new DD run.
    ///
    /// Uses CAS on `is_started` to prevent TOCTOU races when multiple
    /// parties try to claim rooms concurrently.
    ///
    ///
    /// Returns `true` if the room was successfully claimed and initialized,
    /// `false` if it was already started by another party.
    pub fn try_claim_and_start(&self, difficulty: DdDifficulty) -> bool {
        // Atomically flip is_started from false → true
        if self
            .is_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }
        self.kill_count.store(0, Ordering::Relaxed);
        self.spawn_time
            .store(DD_INITIAL_SPAWN_TIME, Ordering::Relaxed);
        self.room_close.store(DD_ROOM_CLOSE_TIME, Ordering::Relaxed);
        self.out_time.store(0, Ordering::Relaxed);
        self.monster_beginner_spawned.store(true, Ordering::Relaxed);
        self.monster_spawned.store(false, Ordering::Relaxed);
        self.is_finished.store(false, Ordering::Relaxed);
        self.deaths.write().clear();
        self.difficulty.store(difficulty as u16, Ordering::Relaxed);
        self.stage_id
            .store(difficulty.starting_stage(), Ordering::Relaxed);
        true
    }
}

/// Atomically find and claim a free DD room, initializing it for the given difficulty.
/// Combines the find + start into a single atomic operation to prevent
/// TOCTOU races where two parties could claim the same room.
pub fn try_claim_free_room(rooms: &[DdRoomInfo], difficulty: DdDifficulty) -> Option<u16> {
    for room in rooms {
        if room.try_claim_and_start(difficulty) {
            return Some(room.room_id.load(Ordering::Relaxed));
        }
    }
    None
}

// ── Timer Tick Result ──────────────────────────────────────────────────

/// Result of a single DD timer tick for a room.
#[derive(Debug, PartialEq, Eq)]
pub enum DdTickResult {
    /// No action needed.
    Idle,
    /// Spawn monsters for the current stage and send stage counter.
    SpawnStage {
        /// Stage ID to spawn monsters for.
        stage_id: i16,
        /// Difficulty for display packet.
        difficulty: u16,
    },
    /// Room time expired or finish timer expired — kick all players.
    KickAll,
}

/// Run one tick of the DD timer for a room.
/// Called every 1 second for each active room.
pub fn timer_tick(room: &DdRoomInfo) -> DdTickResult {
    if !room.is_started.load(Ordering::Relaxed) {
        return DdTickResult::Idle;
    }

    // Decrement spawn timer
    let spawn_time = room.spawn_time.load(Ordering::Relaxed);
    if spawn_time > 0 {
        room.spawn_time.store(spawn_time - 1, Ordering::Relaxed);
    }

    // Check: spawn timer expired and we need to spawn
    let spawn_time = room.spawn_time.load(Ordering::Relaxed);
    if spawn_time == 0
        && (room.monster_beginner_spawned.load(Ordering::Relaxed)
            || room.monster_spawned.load(Ordering::Relaxed))
    {
        // Clear beginner flag after first spawn
        if room.monster_beginner_spawned.load(Ordering::Relaxed) {
            room.monster_beginner_spawned
                .store(false, Ordering::Relaxed);
        }
        room.monster_spawned.store(false, Ordering::Relaxed);

        let difficulty = room.difficulty.load(Ordering::Relaxed);
        let stage_id = room.stage_id.load(Ordering::Relaxed);
        return DdTickResult::SpawnStage {
            stage_id,
            difficulty,
        };
    }

    // Decrement room close timer
    let close = room.room_close.load(Ordering::Relaxed);
    if close > 0 {
        room.room_close.store(close - 1, Ordering::Relaxed);
    }

    // Decrement out timer
    let out_time = room.out_time.load(Ordering::Relaxed);
    if out_time > 0 {
        room.out_time.store(out_time - 1, Ordering::Relaxed);
    }

    // Check: room close expired
    let close = room.room_close.load(Ordering::Relaxed);
    if close == 0 && room.is_started.load(Ordering::Relaxed) {
        room.is_started.store(false, Ordering::Relaxed);
        return DdTickResult::KickAll;
    }

    // Check: finish out timer expired
    let out_time = room.out_time.load(Ordering::Relaxed);
    if out_time == 0 && room.is_finished.load(Ordering::Relaxed) {
        room.is_finished.store(false, Ordering::Relaxed);
        return DdTickResult::KickAll;
    }

    DdTickResult::Idle
}

// ── Stage Advancement ──────────────────────────────────────────────────

/// Result of advancing to the next stage.
#[derive(Debug, PartialEq, Eq)]
pub enum StageAdvanceResult {
    /// Advanced to the next stage successfully. Spawn timer set.
    NextStage {
        /// The new stage ID.
        new_stage_id: i16,
        /// Spawn delay in seconds.
        spawn_delay: u32,
    },
    /// All stages cleared — trigger finish timer.
    Finished,
    /// Error: could not find next stage data.
    Error,
}

/// Advance to the next stage after all monsters in the current stage are killed.
pub fn advance_stage(room: &DdRoomInfo, stages: &[DfStageRow]) -> StageAdvanceResult {
    let difficulty = room.difficulty.load(Ordering::Relaxed);
    let current_stage = room.stage_id.load(Ordering::Relaxed);

    let dd_difficulty = match difficulty {
        1 => DdDifficulty::Easy,
        2 => DdDifficulty::Normal,
        3 => DdDifficulty::Hard,
        _ => return StageAdvanceResult::Error,
    };

    // Check if current stage is the final stage for this difficulty
    if current_stage == dd_difficulty.final_stage() {
        return StageAdvanceResult::Finished;
    }

    // Find the next stage ID from the stage list
    // C++ increments the stage_id and looks up in the monster list
    let next_stage = current_stage + 1;

    // Verify the stage exists in the stage table
    let stage_exists = stages
        .iter()
        .any(|s| s.difficulty == difficulty as i16 && s.stage_id == next_stage);

    if !stage_exists {
        // No more stages — finished
        return StageAdvanceResult::Finished;
    }

    // Set the new stage
    room.stage_id.store(next_stage, Ordering::Relaxed);
    room.monster_spawned.store(true, Ordering::Relaxed);

    // Determine spawn delay based on difficulty and stage
    let delay = get_spawn_delay(difficulty, next_stage);
    room.spawn_time.store(delay, Ordering::Relaxed);

    StageAdvanceResult::NextStage {
        new_stage_id: next_stage,
        spawn_delay: delay,
    }
}

/// Get the spawn delay for a given difficulty and stage.
pub fn get_spawn_delay(_difficulty: u16, stage_id: i16) -> u32 {
    // waves 1-5: 10 s, 6-12: 20 s, 13 and later: 30 s
    match stage_id {
        ..=5 => 10,
        6..=12 => 20,
        _ => 30,
    }
}

// ── Monster Kill Processing ────────────────────────────────────────────

/// Check if a monster proto ID is a valid DD monster.
pub fn is_dd_monster(proto_id: u16) -> bool {
    DD_MONSTER_IDS.contains(&proto_id)
}

/// Check if a monster proto ID is a DD boss (grants Lunar Order Token).
pub fn is_dd_boss(proto_id: u16) -> bool {
    BOSS_MONSTER_IDS.contains(&proto_id)
}

/// Reward calculation for a DD monster kill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdKillReward {
    /// Number of Full Moon Rift Jewels for the killer.
    pub rift_jewel_count: u16,
    /// Number of Monster Coins for the killer.
    pub killer_coin_count: u16,
    /// Number of Monster Coins for other party members.
    pub party_coin_count: u16,
    /// Whether Lunar Order Tokens should be given to party.
    pub lunar_token: bool,
}

/// Calculate rewards for a DD monster kill.
pub fn calculate_kill_reward(proto_id: u16, current_stage: i16) -> Option<DdKillReward> {
    if !is_dd_monster(proto_id) {
        return None;
    }

    // Astaroth (suicide monster) gives no coin and no jewel
    if proto_id == ASTAROTH {
        return None;
    }

    // Full Moon Rift Jewel: 1 for Castle/Draki, 2 for the Bifrost Army
    let rift_jewel_count = if current_stage >= BIFROST_FIRST_STAGE {
        2
    } else {
        1
    };

    // Monster Coin: 2 for killer, 1 for party
    let killer_coin_count = 2;
    let party_coin_count = 1;

    // Lunar Order Token: only for boss kills
    let lunar_token = is_dd_boss(proto_id);

    Some(DdKillReward {
        rift_jewel_count,
        killer_coin_count,
        party_coin_count,
        lunar_token,
    })
}

// ── Stage Counter Display ──────────────────────────────────────────────

/// Build the stage counter display values for a room.
/// Returns (max_stage, display_stage) for the TEMPLE_EVENT_STAGE_COUNTER packet.
pub fn get_stage_display(difficulty: u16, stage_id: i16) -> Option<(u8, u8)> {
    let dd_diff = match difficulty {
        1 => DdDifficulty::Easy,
        2 => DdDifficulty::Normal,
        3 => DdDifficulty::Hard,
        _ => return None,
    };

    let max_stage = dd_diff.max_stages();
    let display_stage = (stage_id - dd_diff.stage_offset()) as u8;
    Some((max_stage, display_stage))
}

// ── Finish Timer ───────────────────────────────────────────────────────

/// Trigger the finish sequence for a DD room.
/// Sets the room to finished state and configures the out timer.
pub fn trigger_finish(room: &DdRoomInfo) {
    room.monster_beginner_spawned
        .store(false, Ordering::Relaxed);
    room.monster_spawned.store(false, Ordering::Relaxed);
    room.is_finished.store(true, Ordering::Relaxed);
    room.out_time.store(DD_FINISH_TIME, Ordering::Relaxed);
}

// ── Stage Lookup ───────────────────────────────────────────────────────

/// Find the current stage entry in the stage list.
pub fn select_stage(stages: &[DfStageRow], difficulty: u16, stage_id: i16) -> Option<&DfStageRow> {
    stages
        .iter()
        .find(|s| s.stage_id == stage_id && s.difficulty == difficulty as i16)
}

/// Get all monster spawns for a given stage.
pub fn get_stage_monsters(monsters: &[DfMonsterRow], stage_id: i16) -> Vec<&DfMonsterRow> {
    monsters
        .iter()
        .filter(|m| m.id / 100 == stage_id as i32)
        .collect()
}

// ── Monster Coin Cleanup ───────────────────────────────────────────────

/// Check if a user needs Monster Coin cleanup on DD zone enter.
/// Removes all Monster Coins from the user when entering the DD zone.
pub fn should_rob_monster_coins() -> bool {
    // Always rob monster coins on DD entry
    true
}

// ── Towers, deaths, defeat ─────────────────────────────────────────────

/// Deaths a player may use before being sent back to Moradon.
pub const DD_MAX_DEATHS: u8 = 3;
/// Tower proto ids: outer, inner, nexus.
pub const OUTER_TOWER: u16 = 31738;
pub const INNER_TOWER: u16 = 31739;
pub const NEXUS_TOWER: u16 = 31740;
/// Metres within which a monster attacks a tower.
const TOWER_REACH: f32 = 10.0;
/// Damage an Astaroth deals to a tower when it blows itself up.
const ASTAROTH_BLAST: i32 = 5_000;

fn room_notice(world: &crate::world::WorldState, room_id: u16, msg: &str) {
    let pkt = crate::systems::timed_notice::build_notice_packet(8, msg);
    world.broadcast_to_zone_event_room(ZONE_DUNGEON_DEFENCE, room_id, std::sync::Arc::new(pkt), None);
}

/// Send a player home: revive if dead, drop the Monster Coins, restore HP, teleport to Moradon.
pub fn remove_player(world: &crate::world::WorldState, sid: u16) {
    if world.is_player_dead(sid) {
        world.update_res_hp_type(sid, crate::world::USER_STANDING);
        world.update_session(sid, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 0;
        });
    }
    clear_rift_items(world, sid);
    world.update_session(sid, |h| h.event_room = 0);
    super::zone_change::server_teleport_to_zone(world, sid, crate::world::types::ZONE_MORADON, 0.0, 0.0);
    restore_stats(world, sid);
}

/// Undo the rift's 10 000 HP equalization: real max HP/MP back, full HP, client told.
pub fn restore_stats(world: &crate::world::WorldState, sid: u16) {
    if restore_saved(world, sid).is_none() {
        world.set_user_ability(sid);
        world.recalculate_max_hp_mp(sid);
    }
    if let Some(ch) = world.get_character_info(sid) {
        world.update_character_hp(sid, ch.max_hp);
        world.send_to_session_owned(sid, crate::handler::max_hp_change::build_max_hp_change(ch.max_hp as i32));
        world.send_to_session_owned(sid, crate::systems::regen::build_hp_change_packet(ch.max_hp, ch.max_hp));
    }
}

/// A player died inside a Rift room: 3 deaths are allowed, the third one sends them to Moradon.
/// When nobody is left the rift is lost.
pub fn on_player_death(world: &crate::world::WorldState, sid: u16) {
    let room_id = world.with_session(sid, |h| h.event_room).unwrap_or(0);
    if room_id == 0 {
        return;
    }
    let Some(room) = world
        .dd_rooms()
        .iter()
        .find(|r| r.room_id.load(Ordering::Relaxed) == room_id && r.is_started.load(Ordering::Relaxed))
    else {
        return;
    };
    let used = {
        let mut d = room.deaths.write();
        let e = d.entry(sid).or_insert(0);
        *e += 1;
        *e
    };
    if used < DD_MAX_DEATHS {
        let msg = format!("You died. Deaths left: {}.", DD_MAX_DEATHS - used);
        world.send_to_session_owned(sid, crate::systems::timed_notice::build_notice_packet(8, &msg));
        return;
    }
    world.send_to_session_owned(
        sid,
        crate::systems::timed_notice::build_notice_packet(8, "You have used all your deaths and leave the rift."),
    );
    remove_player(world, sid);
    let left = world.collect_sessions_by(|h| h.event_room == room_id && h.position.zone_id == ZONE_DUNGEON_DEFENCE);
    if left.is_empty() {
        room.room_close.store(1, Ordering::Relaxed);
    }
}

/// Monsters within reach hit the towers; Astaroth blows up on a tower. A fallen Nexus loses the rift.
/// Called once a second.
pub fn tower_tick(world: &crate::world::WorldState) {
    for room in world.dd_rooms() {
        if !room.is_started.load(Ordering::Relaxed) || room.is_finished.load(Ordering::Relaxed) {
            continue;
        }
        let room_id = room.room_id.load(Ordering::Relaxed);
        guard_lane(world, room_id);
        bots_defend(world, room, room_id);
        for &(proto, x, z) in &GUARDIAN_NPCS {
            if !(OUTER_TOWER..=NEXUS_TOWER).contains(&proto) {
                continue;
            }
            let near = world.npcs_near(ZONE_DUNGEON_DEFENCE, x as f32, z as f32, TOWER_REACH, room_id);
            let Some(tower) = near.iter().find(|n| n.proto_id == proto && !n.is_monster) else { continue };
            if world.is_npc_dead(tower.nid) {
                continue;
            }
            let mut dmg = 0i32;
            for m in near.iter().filter(|n| n.is_monster && !world.is_npc_dead(n.nid)) {
                if m.proto_id == ASTAROTH {
                    dmg += ASTAROTH_BLAST;
                    world.kill_npc(m.nid);
                    crate::handler::attack::dd_decrement_and_advance(world, room_id);
                } else if let Some(t) = world.get_npc_template(m.proto_id, true) {
                    dmg += (t.damage as i32 / 5).max(1);
                }
            }
            if dmg == 0 {
                continue;
            }
            let hp = world.get_npc_hp(tower.nid).unwrap_or(0) - dmg;
            if hp > 0 {
                world.update_npc_hp(tower.nid, hp);
                continue;
            }
            world.kill_npc(tower.nid);
            match proto {
                NEXUS_TOWER => {
                    room_notice(world, room_id, "The Nexus has fallen. The rift is lost.");
                    room.room_close.store(1, Ordering::Relaxed);
                }
                OUTER_TOWER => room_notice(world, room_id, "The Outer Tower has fallen!"),
                _ => room_notice(world, room_id, "The Inner Tower has fallen!"),
            }
        }
    }
}

/// Claim a room, move `members` in and spawn the guardians. No voucher/party checks here.
pub fn open_room(world: &crate::world::WorldState, members: &[u16], difficulty: DdDifficulty) -> Option<u16> {
    let room_id = try_claim_free_room(world.dd_rooms(), difficulty)?;
    for &m in members {
        super::zone_change::server_teleport_to_zone(world, m, ZONE_DUNGEON_DEFENCE, 0.0, 0.0);
        world.update_session(m, |h| h.event_room = room_id);
    }
    for &(npc_id, x, z) in &GUARDIAN_NPCS {
        world.spawn_event_npc_ex(npc_id, false, ZONE_DUNGEON_DEFENCE, x as f32, z as f32, 1, room_id, 0);
    }
    Some(room_id)
}

/// `+riftopen [easy|normal|hard]`: the GM opens a rift for their whole party (no voucher or level check).
pub fn gm_open(world: &crate::world::WorldState, sid: u16, difficulty: DdDifficulty) -> String {
    let mut members = world
        .get_party_id(sid)
        .and_then(|pid| world.get_party(pid))
        .map(|p| p.active_members())
        .unwrap_or_default();
    if !members.contains(&sid) {
        members.push(sid);
    }
    match open_room(world, &members, difficulty) {
        Some(room_id) => format!(
            "Full Moon Rift opened (room {room_id}, {} waves). First wave in {DD_INITIAL_SPAWN_TIME} s.",
            difficulty.max_stages()
        ),
        None => "No free Full Moon Rift room.".to_string(),
    }
}

// ── Test bots (GM) ───────────────────────────────────────────────────────

/// Damage one bot deals to a monster per second.
const BOT_DPS: i32 = 1_500;
/// Bots hold the line at the Outer Tower and hit anything within this many metres.
const BOT_REACH: f32 = 12.0;

/// `+eventbotfill <count>` inside a rift: bots join the GM's room and hold the Outer Tower.
pub fn gm_spawn_bots(world: &crate::world::WorldState, gm_sid: u16, count: usize, class: u16) -> String {
    let room_id = world.get_event_room(gm_sid);
    let Some(room) = world
        .dd_rooms()
        .iter()
        .find(|r| r.room_id.load(Ordering::Relaxed) == room_id && r.is_started.load(Ordering::Relaxed))
    else {
        return "You must be inside a Full Moon Rift room.".to_string();
    };
    let nation = world.get_character_info(gm_sid).map(|c| c.nation).unwrap_or(1);
    let (ox, oz) = GUARDIAN_NPCS
        .iter()
        .find(|g| g.0 == OUTER_TOWER)
        .map(|g| (g.1 as f32, g.2 as f32))
        .unwrap_or((196.0, 54.0));
    let mut rng = rand::thread_rng();
    let mut n = 0;
    for i in 0..count.clamp(1, 30) {
        let id = crate::systems::bot_ai::spawn_gm_bot(
            world,
            crate::systems::bot_ai::SpawnGmBotParams {
                zone_id: ZONE_DUNGEON_DEFENCE,
                x: ox - 4.0 + rng.gen_range(-3.0..3.0),
                y: 0.0,
                z: oz + rng.gen_range(-4.0..4.0),
                class: (i % 4) as u16 + 1 + 0 * class,
                level: 83,
                nation,
                ai_state: crate::world::BotAiState::Afk,
            },
        );
        room.bots.write().push(id);
        n += 1;
    }
    format!("{n} bot(s) joined the rift ({} in total).", room.bots.read().len())
}

fn bots_defend(world: &crate::world::WorldState, room: &DdRoomInfo, room_id: u16) {
    let bots: Vec<u32> = room.bots.read().clone();
    for id in bots {
        let Some(bot) = world.get_bot(id) else { continue };
        if !bot.is_alive() {
            continue;
        }
        let near = world.npcs_near(ZONE_DUNGEON_DEFENCE, bot.x, bot.z, BOT_REACH, room_id);
        let Some(target) = near
            .iter()
            .filter(|n| n.is_monster && !world.is_npc_dead(n.nid))
            .min_by(|a, b| {
                let da = (a.x - bot.x).powi(2) + (a.z - bot.z).powi(2);
                let db = (b.x - bot.x).powi(2) + (b.z - bot.z).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
        else {
            continue;
        };
        crate::systems::bot_ai::event_bot_swing(world, id, target.nid);
        let hp = world.get_npc_hp(target.nid).unwrap_or(0) - BOT_DPS;
        if hp > 0 {
            world.update_npc_hp(target.nid, hp);
        } else {
            world.kill_npc(target.nid);
            crate::handler::attack::dd_decrement_and_advance(world, room_id);
        }
    }
}

/// Remove the room's bots (room closed).
pub fn despawn_bots(world: &crate::world::WorldState, room_id: u16) {
    for room in world.dd_rooms() {
        if room.room_id.load(Ordering::Relaxed) == room_id {
            for id in room.bots.write().drain(..) {
                crate::systems::bot_ai::despawn_bot(world, id);
            }
        }
    }
}

/// The lane the waves walk: spawn corner, then the points the map's designer gave us.
/// (Outer Tower 199,54 -> bend 205,109 -> Inner Tower 65,133 -> bend 73,212 -> Nexus 167,211.)
const ROUTE: [(f32, f32); 6] = [
    (60.0, 42.0),
    (199.0, 54.0),
    (205.0, 109.0),
    (65.0, 133.0),
    (73.0, 212.0),
    (167.0, 211.0),
];

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, abz) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + abz * abz;
    let t = if len2 <= f32::EPSILON {
        0.0
    } else {
        (((p.0 - a.0) * abx + (p.1 - a.1) * abz) / len2).clamp(0.0, 1.0)
    };
    let (cx, cz) = (a.0 + abx * t, a.1 + abz * t);
    ((p.0 - cx).powi(2) + (p.1 - cz).powi(2)).sqrt()
}

/// Where a monster standing at (x, z) walks next: the end of the lane segment it is on
/// (the one after that when it is already at the bend).
pub fn route_target(x: f32, z: f32) -> (f32, f32) {
    let mut best = (f32::MAX, 0usize);
    for i in 0..ROUTE.len() - 1 {
        let d = dist_to_segment((x, z), ROUTE[i], ROUTE[i + 1]);
        if d < best.0 {
            best = (d, i);
        }
    }
    let mut end = best.1 + 1;
    let (ex, ez) = ROUTE[end];
    if ((ex - x).powi(2) + (ez - z).powi(2)).sqrt() < 6.0 && end + 1 < ROUTE.len() {
        end += 1;
    }
    ROUTE[end]
}

/// Items that only exist for the rift: Monster Coins and the skills bought with them.
pub const RIFT_ITEMS: [u32; 11] = [
    MONSTER_COIN_ITEM,
    914059000, // Light Feet
    914060000, // Drain Restore
    914061000, // Fire Strike
    914062000, // Bloody Arrow
    914063000, // Ankle Freeze
    914064000, // Inferior Guard Summon
    914065000, // Guard Summon
    914066000, // Summon Guard (default)
    914077000, // Tear of Goddess
    914078000, // God's Shield
];

/// Take every rift-only item (coins and bought skills) from the player.
pub fn clear_rift_items(world: &crate::world::WorldState, sid: u16) {
    for item in RIFT_ITEMS {
        world.rob_all_of_item(sid, item);
    }
}

/// Players' equalized HP inside the rift.
pub const RIFT_HP: i16 = 10_000;

fn saved_stats() -> &'static parking_lot::Mutex<HashMap<u16, (i16, i16)>> {
    static S: std::sync::OnceLock<parking_lot::Mutex<HashMap<u16, (i16, i16)>>> = std::sync::OnceLock::new();
    S.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

/// Entering the rift: remember the real max HP/MP, give everybody 10 000 HP and tell the client.
pub fn equalize(world: &crate::world::WorldState, sid: u16) {
    if let Some(ch) = world.get_character_info(sid) {
        saved_stats().lock().entry(sid).or_insert((ch.max_hp, ch.max_mp));
    }
    world.update_character_stats(sid, |ch| {
        ch.max_hp = RIFT_HP;
        ch.hp = RIFT_HP;
    });
    world.send_to_session_owned(sid, crate::handler::max_hp_change::build_max_hp_change(RIFT_HP as i32));
    world.send_to_session_owned(sid, crate::systems::regen::build_hp_change_packet(RIFT_HP, RIFT_HP));
}

/// Put the remembered real max HP/MP back (no packets). Returns the values when there were any.
pub fn restore_saved(world: &crate::world::WorldState, sid: u16) -> Option<(i16, i16)> {
    let saved = saved_stats().lock().remove(&sid)?;
    world.update_character_stats(sid, |ch| {
        ch.max_hp = saved.0;
        ch.max_mp = saved.1;
        ch.hp = saved.0;
    });
    Some(saved)
}

/// Disconnect inside the rift: the saved character must not keep the equalized HP.
pub fn restore_on_disconnect(world: &crate::world::WorldState, sid: u16) {
    if world.get_position(sid).is_some_and(|p| p.zone_id == ZONE_DUNGEON_DEFENCE) {
        clear_rift_items(world, sid);
    }
    restore_saved(world, sid);
}

/// Players may not walk back past this line towards the monster spawn (x < 110 on the lane z < 70).
const LANE_LIMIT_X: f32 = 110.0;
const LANE_LIMIT_Z: f32 = 70.0;

/// The tower the monsters head for: outer, then inner, then the Nexus (first one still standing).
pub fn march_goal(world: &crate::world::WorldState, room_id: u16) -> Option<(f32, f32)> {
    for want in [OUTER_TOWER, INNER_TOWER, NEXUS_TOWER] {
        for &(proto, x, z) in &GUARDIAN_NPCS {
            if proto != want {
                continue;
            }
            let near = world.npcs_near(ZONE_DUNGEON_DEFENCE, x as f32, z as f32, 3.0, room_id);
            if near.iter().any(|n| n.proto_id == proto && !n.is_monster && !world.is_npc_dead(n.nid)) {
                return Some((x as f32, z as f32));
            }
        }
    }
    None
}

/// Players who walk past the lane limit towards the spawn are sent back to the defence area.
pub fn guard_lane(world: &crate::world::WorldState, room_id: u16) {
    let Some((gx, gz)) = march_goal(world, room_id) else { return };
    for sid in world.collect_sessions_by(|h| h.event_room == room_id && h.position.zone_id == ZONE_DUNGEON_DEFENCE) {
        let Some(pos) = world.get_position(sid) else { continue };
        if pos.x < LANE_LIMIT_X && pos.z < LANE_LIMIT_Z {
            super::region::relocate_user_within_zone(world, sid, gx + 2.0, pos.y, gz + 2.0);
            world.send_to_session_owned(
                sid,
                crate::systems::timed_notice::build_notice_packet(8, "You cannot go any further towards the monster spawn."),
            );
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn stages(diff: i16, last: i16) -> Vec<DfStageRow> {
        (1..=last)
            .map(|n| DfStageRow {
                id: diff as i32 * 100 + n as i32,
                difficulty: diff,
                difficulty_name: None,
                stage_id: n,
            })
            .collect()
    }

    fn mon(id: i32, monster_id: i16, count: i16) -> DfMonsterRow {
        DfMonsterRow {
            id,
            difficulty: Some(1),
            monster_id,
            is_monster: false,
            pos_x: 60,
            pos_z: 42,
            s_count: Some(count),
            s_direction: 0,
            s_radius_range: Some(10),
        }
    }

    #[test]
    fn difficulty_from_party_size() {
        assert_eq!(DdDifficulty::from_party_size(1), None);
        assert_eq!(DdDifficulty::from_party_size(2), Some(DdDifficulty::Easy));
        assert_eq!(DdDifficulty::from_party_size(5), Some(DdDifficulty::Normal));
        assert_eq!(DdDifficulty::from_party_size(8), Some(DdDifficulty::Hard));
    }

    #[test]
    fn difficulty_wave_counts() {
        assert_eq!(DdDifficulty::Easy.final_stage(), 5);
        assert_eq!(DdDifficulty::Normal.final_stage(), 15);
        assert_eq!(DdDifficulty::Hard.final_stage(), 20);
        assert_eq!(DdDifficulty::Hard.max_stages(), 20);
        assert_eq!(DdDifficulty::Hard.starting_stage(), 1);
    }

    #[test]
    fn spawn_delays() {
        assert_eq!(get_spawn_delay(3, 5), 10);
        assert_eq!(get_spawn_delay(3, 6), 20);
        assert_eq!(get_spawn_delay(3, 12), 20);
        assert_eq!(get_spawn_delay(3, 13), 30);
        assert_eq!(get_spawn_delay(3, 20), 30);
    }

    #[test]
    fn rewards() {
        let r = calculate_kill_reward(9997, 3).unwrap();
        assert_eq!((r.rift_jewel_count, r.killer_coin_count, r.party_coin_count), (1, 2, 1));
        assert!(!r.lunar_token);
        assert_eq!(calculate_kill_reward(10012, 16).unwrap().rift_jewel_count, 2);
        assert!(calculate_kill_reward(10000, 5).unwrap().lunar_token);
        assert!(calculate_kill_reward(10019, 20).unwrap().lunar_token);
        assert!(calculate_kill_reward(ASTAROTH, 7).is_none());
        assert!(calculate_kill_reward(1, 1).is_none());
    }

    #[test]
    fn stage_display() {
        assert_eq!(get_stage_display(1, 3), Some((5, 3)));
        assert_eq!(get_stage_display(2, 15), Some((15, 15)));
        assert_eq!(get_stage_display(3, 20), Some((20, 20)));
        assert_eq!(get_stage_display(0, 1), None);
    }

    #[test]
    fn stage_monsters_grouped_by_wave() {
        let m = vec![mon(101, 9997, 5), mon(102, 9998, 5), mon(201, 9997, 3)];
        assert_eq!(get_stage_monsters(&m, 1).len(), 2);
        assert_eq!(get_stage_monsters(&m, 2).len(), 1);
        assert!(get_stage_monsters(&m, 3).is_empty());
    }

    #[test]
    fn advance_until_finished() {
        let st = stages(1, 5);
        let room = DdRoomInfo::new(1);
        assert!(room.try_claim_and_start(DdDifficulty::Easy));
        for expect in 2..=5 {
            match advance_stage(&room, &st) {
                StageAdvanceResult::NextStage { new_stage_id, spawn_delay } => {
                    assert_eq!(new_stage_id, expect);
                    assert_eq!(spawn_delay, 10);
                }
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(advance_stage(&room, &st), StageAdvanceResult::Finished);
    }

    #[test]
    fn hard_runs_twenty_waves() {
        let st = stages(3, 20);
        let room = DdRoomInfo::new(1);
        assert!(room.try_claim_and_start(DdDifficulty::Hard));
        let mut n = 1;
        while let StageAdvanceResult::NextStage { .. } = advance_stage(&room, &st) {
            n += 1;
        }
        assert_eq!(n, 20);
    }

    #[test]
    fn claim_is_exclusive() {
        let rooms = vec![DdRoomInfo::new(1), DdRoomInfo::new(2)];
        assert_eq!(try_claim_free_room(&rooms, DdDifficulty::Easy), Some(1));
        assert_eq!(try_claim_free_room(&rooms, DdDifficulty::Easy), Some(2));
        assert_eq!(try_claim_free_room(&rooms, DdDifficulty::Easy), None);
    }
}
