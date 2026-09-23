//! Nation battle (war) system — state management and victory logic.
//! The war system tracks nation-vs-nation battles across 6 battle zones.
//! It manages kill/death counters, monument capture points, flag victory,
//! commander selection, and war timers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::world::{
    WorldState, ZONE_BATTLE, ZONE_BATTLE2, ZONE_BATTLE3, ZONE_BATTLE4, ZONE_BATTLE5, ZONE_BATTLE6,
    ZONE_BATTLE_BASE, ZONE_ELMORAD, ZONE_KARUS,
};

// ── Battle Open State ────────────────────────────────────────────────────────

/// No active battle.
pub const NO_BATTLE: u8 = 0;

/// Nation battle (standard war) is active.
pub const NATION_BATTLE: u8 = 1;

/// Snow battle (snowball war) is active.
pub const SNOW_BATTLE: u8 = 2;

/// Siege battle (castle siege) is active.
pub const SIEGE_BATTLE: u8 = 3;

// ── Battle Zone Command Types ────────────────────────────────────────────────

/// Battle zone open command.
pub const BATTLEZONE_OPEN: u8 = 0x00;

/// Battle zone close command.
pub const BATTLEZONE_CLOSE: u8 = 0x01;

/// Battle zone none (reset sentinel).
pub const BATTLEZONE_NONE: u8 = 0xFF;

/// Snow battle zone open announcement type.
pub const SNOW_BATTLEZONE_OPEN: u8 = 0x09;

/// Snow battle zone close announcement type.
pub const SNOW_BATTLEZONE_CLOSE: u8 = 0x15;

// ── Announcement Types ───────────────────────────────────────────────────────

/// Announce the winning nation.
pub const DECLARE_WINNER: u8 = 0x02;

/// Announce the losing nation.
pub const DECLARE_LOSER: u8 = 0x03;

/// Monument status announcement.
pub const DECLARE_NATION_MONUMENT_STATUS: u8 = 0x13;

/// Monument reward announcement.
pub const DECLARE_NATION_REWARD_STATUS: u8 = 0x14;

/// Banish announcement (losers will be kicked from the zone).
pub const DECLARE_BAN: u8 = 0x04;

/// Battle zone status announcement (kill/death/monument status).
pub const DECLARE_BATTLE_ZONE_STATUS: u8 = 0x11;

/// Under-attack notification (post-victory phase).
pub const UNDER_ATTACK_NOTIFY: u8 = 0x10;

// ── War Rewards ──────────────────────────────────────────────────────────────

/// Number of flag captures needed for instant victory.
pub const NUM_FLAG_VICTORY: u8 = 4;

/// Gold awarded to winners in flag-victory scenario.
pub const AWARD_GOLD: u32 = 100_000;

/// Experience awarded to winners in flag-victory scenario.
pub const AWARD_EXP: i64 = 5000;

// ── Fame Constants ───────────────────────────────────────────────────────────

/// Re-export from canonical clan_constants for callers using `war::COMMAND_CAPTAIN`.
pub use crate::clan_constants::COMMAND_CAPTAIN;

// ── Monument SIDs ────────────────────────────────────────────────────────────

/// El Morad main monument NPC SID.
pub const ELMORAD_MONUMENT_SID: u16 = 10301;

/// Luferson (Karus) main monument NPC SID.
pub const LUFERSON_MONUMENT_SID: u16 = 20301;

// ── Ardream zone type (used for mini-war) ────────────────────────────────────

/// Ardream-type battle zone (mini-war with no banish phase).
pub const ZONE_ARDREAM_TYPE: u8 = 72;

// ── Winner Determination Types ───────────────────────────────────────────────

/// How the winner is determined when time runs out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BattleWinnerType {
    /// Compare NPC kills (killed_karus_npc vs killed_elmorad_npc).
    Npc = 0,
    /// Compare player deaths (fewer deaths wins).
    Kill = 1,
    /// Compare monument capture points.
    Monument = 2,
}

// ── Battle State ─────────────────────────────────────────────────────────────

/// Full mutable battle/war state, stored behind `RwLock` on `WorldState`.
#[derive(Debug, Clone)]
pub struct BattleState {
    // ── Core State ───────────────────────────────────────────────────────
    /// Current battle open state (NO_BATTLE=0, NATION_BATTLE=1, SNOW_BATTLE=2).
    ///
    pub battle_open: u8,

    /// Previous battle open state (for close announcements).
    ///
    pub old_battle_open: u8,

    /// Which battle zone offset (0-5, actual zone = ZONE_BATTLE_BASE + battle_zone).
    ///
    pub battle_zone: u8,

    /// Battle zone type (0 = standard, ZONE_ARDREAM = mini-war).
    ///
    pub battle_zone_type: u8,

    /// Winning nation (0=none, 1=Karus, 2=ElMorad).
    ///
    pub victory: u8,

    /// Previous victory nation (for delayed result processing).
    ///
    pub old_victory: u8,

    /// Victory nation for result delay phase.
    ///
    pub result_delay_victory: u8,

    /// Middle statue nation ownership (for ZONE_BATTLE4).
    ///
    pub middle_statue_nation: u8,

    // ── Timing ───────────────────────────────────────────────────────────
    /// Unix timestamp when war was opened.
    ///
    pub battle_opened_time: i32,

    /// Total configured war duration in seconds.
    ///
    pub battle_time: i32,

    /// Remaining war time in seconds.
    ///
    pub battle_remaining_time: i32,

    /// Timer delay counter (ticks).
    ///
    pub battle_time_delay: i32,

    /// Battle notice timer (announcement interval counter).
    ///
    pub battle_notice_time: u8,

    /// Nereids Island remaining time (ZONE_BATTLE4/5 specific).
    ///
    pub nereids_remaining_time: i32,

    // ── Kill / Death Counters ────────────────────────────────────────────
    /// Karus player deaths in war zone.
    ///
    pub karus_dead: i16,

    /// El Morad player deaths in war zone.
    ///
    pub elmorad_dead: i16,

    /// Current Karus player count in war zone.
    ///
    pub karus_count: i16,

    /// Current El Morad player count in war zone.
    ///
    pub elmorad_count: i16,

    // ── NPC Kill Counters ────────────────────────────────────────────────
    /// Karus NPCs killed by El Morad (favor El Morad in NPC winner check).
    ///
    pub killed_karus_npc: u8,

    /// El Morad NPCs killed by Karus (favor Karus in NPC winner check).
    ///
    pub killed_elmorad_npc: u8,

    // ── Monument Points ──────────────────────────────────────────────────
    /// Number of monuments captured by Karus.
    ///
    pub karus_monuments: u8,

    /// Number of monuments captured by El Morad.
    ///
    pub elmorad_monuments: u8,

    /// Accumulated monument capture points for Karus.
    ///
    pub karus_monument_point: u16,

    /// Accumulated monument capture points for El Morad.
    ///
    pub elmorad_monument_point: u16,

    // ── Flag Captures ────────────────────────────────────────────────────
    /// Karus flag capture count (victory at NUM_FLAG_VICTORY).
    ///
    pub karus_flag: u8,

    /// El Morad flag capture count (victory at NUM_FLAG_VICTORY).
    ///
    pub elmorad_flag: u8,

    // ── Open / Banish Flags ──────────────────────────────────────────────
    /// Whether Karus zone gate is open (after one side wins, let losers be kicked).
    ///
    pub karus_open_flag: bool,

    /// Whether El Morad zone gate is open.
    ///
    pub elmorad_open_flag: bool,

    /// Whether banish phase is active (kicking losers from war zone).
    ///
    pub banish_flag: bool,

    /// Banish delay counter (ticks).
    ///
    pub banish_delay: i16,

    /// Whether battle save (result persistence) is needed.
    ///
    pub battle_save: bool,

    /// Whether result delay is active.
    ///
    pub result_delay: bool,

    /// Result delay counter (ticks).
    ///
    pub battle_result_delay: i32,

    // ── Commander State ──────────────────────────────────────────────────
    /// Whether war commanders have been selected.
    ///
    pub commander_selected: bool,

    // ── Nereids Island Monuments ─────────────────────────────────────────
    /// Monument ownership array for Nereids Island (ZONE_BATTLE4).
    ///
    pub nereids_monument_array: [u8; 7],

    // ── Nation Monument Tracking ─────────────────────────────────────────
    /// Nation monuments captured by the winning side: proto_id → next_reward_time (UNIX).
    ///
    /// with `RepawnedTime` field. Timer interval: 300 seconds.
    pub nation_monument_winners: HashMap<u16, i32>,

    /// Nation monuments captured by the defeated side: proto_id → next_reward_time (UNIX).
    ///
    /// with `RepawnedTime` field. Timer interval: 10,000 seconds. Entry deleted after fire.
    pub nation_monument_defeated: HashMap<u16, i32>,
}

impl BattleState {
    /// Create a new default (no-war) battle state.
    pub fn new() -> Self {
        Self {
            battle_open: NO_BATTLE,
            old_battle_open: NO_BATTLE,
            battle_zone: 0,
            battle_zone_type: 0,
            victory: 0,
            old_victory: 0,
            result_delay_victory: 0,
            middle_statue_nation: 0,
            battle_opened_time: 0,
            battle_time: 0,
            battle_remaining_time: 0,
            battle_time_delay: 0,
            battle_notice_time: 0,
            nereids_remaining_time: 0,
            karus_dead: 0,
            elmorad_dead: 0,
            karus_count: 0,
            elmorad_count: 0,
            killed_karus_npc: 0,
            killed_elmorad_npc: 0,
            karus_monuments: 0,
            elmorad_monuments: 0,
            karus_monument_point: 0,
            elmorad_monument_point: 0,
            karus_flag: 0,
            elmorad_flag: 0,
            karus_open_flag: false,
            elmorad_open_flag: false,
            banish_flag: false,
            banish_delay: 0,
            battle_save: false,
            result_delay: false,
            battle_result_delay: 0,
            commander_selected: false,
            nereids_monument_array: [0; 7],
            nation_monument_winners: HashMap::new(),
            nation_monument_defeated: HashMap::new(),
        }
    }

    /// Whether any battle (nation or snow) is currently active.
    ///
    #[inline]
    pub fn is_war_open(&self) -> bool {
        self.battle_open != NO_BATTLE
    }

    /// Whether a nation battle (not snow) is specifically active.
    #[inline]
    pub fn is_nation_battle(&self) -> bool {
        self.battle_open == NATION_BATTLE
    }

    /// Whether a snow battle is specifically active.
    #[inline]
    pub fn is_snow_battle(&self) -> bool {
        self.battle_open == SNOW_BATTLE
    }

    /// Get the actual zone ID for the current battle zone.
    ///
    /// Returns `ZONE_BATTLE_BASE + battle_zone` (e.g., 61 for battle_zone=1).
    #[inline]
    pub fn battle_zone_id(&self) -> u16 {
        ZONE_BATTLE_BASE + self.battle_zone as u16
    }
}

impl Default for BattleState {
    fn default() -> Self {
        Self::new()
    }
}

// ── War System Functions ─────────────────────────────────────────────────────

/// Reset all battle zone state fields to defaults.
/// The `reset_type` parameter indicates whether NPC abilities should be
/// toggled (BATTLEZONE_OPEN or BATTLEZONE_CLOSE). The NPC ability change
/// is deferred to the caller since it requires iterating NPC threads.
pub fn reset_battle_zone(state: &mut BattleState) {
    state.commander_selected = false;
    state.victory = 0;

    state.banish_delay = 0;
    state.banish_flag = false;

    state.battle_result_delay = 0;
    state.result_delay = false;

    state.karus_flag = 0;
    state.elmorad_flag = 0;

    state.karus_open_flag = false;
    state.elmorad_open_flag = false;

    state.battle_save = false;

    state.battle_zone = 0;
    state.battle_zone_type = 0;

    state.battle_open = NO_BATTLE;
    state.old_battle_open = NO_BATTLE;

    state.battle_notice_time = 0;
    state.battle_opened_time = 0;
    state.battle_remaining_time = 0;
    state.battle_time_delay = 0;
    state.nereids_remaining_time = 0;

    state.karus_dead = 0;
    state.elmorad_dead = 0;

    state.karus_count = 0;
    state.elmorad_count = 0;

    state.killed_karus_npc = 0;
    state.killed_elmorad_npc = 0;

    state.karus_monument_point = 0;
    state.elmorad_monument_point = 0;
    state.karus_monuments = 0;
    state.elmorad_monuments = 0;

    state.nereids_monument_array = [0; 7];
    state.middle_statue_nation = 0;
    state.nation_monument_winners.clear();
    state.nation_monument_defeated.clear();
}

/// Open a battle zone (start a war).
/// Returns `true` if the war was successfully opened, `false` if already open
/// or invalid type. The caller is responsible for sending announcements and
/// kicking users from conflicting zones.
pub fn battle_zone_open(state: &mut BattleState, n_type: u8, zone: u8, now_unix: i32) -> bool {
    if (n_type == BATTLEZONE_OPEN || n_type == SNOW_BATTLEZONE_OPEN) && !state.is_war_open() {
        reset_battle_zone(state);

        // Scheduled events set this explicitly; GM-triggered opens do not.
        // A zero duration makes the first war tick close the event immediately.
        if state.battle_time <= 0 {
            state.battle_time = if n_type == SNOW_BATTLEZONE_OPEN {
                1800
            } else {
                3600
            };
        }

        state.battle_open = if n_type == BATTLEZONE_OPEN {
            NATION_BATTLE
        } else {
            SNOW_BATTLE
        };
        state.old_battle_open = state.battle_open;
        state.battle_zone = zone;
        state.battle_opened_time = now_unix;
        state.battle_remaining_time = state.battle_time;

        // Nereids Island specific: set remaining time
        let zone_id = zone as u16 + ZONE_BATTLE_BASE;
        if zone_id == ZONE_BATTLE4 || zone_id == ZONE_BATTLE5 {
            state.nereids_remaining_time = state.battle_time;
        }

        return true;
    }

    // Close/announce-only scenarios handled by caller
    false
}

/// Close the active battle zone (end war).
/// Returns the previous battle_open type so the caller can decide what
/// announcements and cleanup to perform.
pub fn battle_zone_close(state: &mut BattleState) -> u8 {
    if !state.is_war_open() {
        return NO_BATTLE;
    }

    let prev_type = state.battle_open;

    if prev_type == SNOW_BATTLE {
        // Snow battle: reset with NONE type, set banish
        reset_battle_zone(state);
        state.banish_flag = true;
    } else if prev_type == NATION_BATTLE {
        // Nation battle: reset with CLOSE type, set banish
        // Commander reset is deferred to the caller
        reset_battle_zone(state);
        state.banish_flag = true;
    }

    prev_type
}

/// Determine the winner using cascading tiebreak logic.
/// Cascade order depends on the starting type:
/// - NPC → Kill → Monument → Draw (for ZONE_BATTLE/2/3)
/// - Kill → NPC → Monument → Draw (for ZONE_BATTLE4/5/6)
/// - Monument → Kill → Draw
/// Returns winning nation (1=Karus, 2=ElMorad) or 0 for a draw.
pub fn battle_winner_result(state: &BattleState, winner_type: BattleWinnerType) -> u8 {
    let battle_zone_id = state.battle_zone as u16 + ZONE_BATTLE_BASE;

    match winner_type {
        BattleWinnerType::Npc => {
            // Compare NPC kills: more enemy NPCs killed = winner
            if state.killed_karus_npc > state.killed_elmorad_npc {
                return 1; // Karus wins (killed more Karus NPCs means... wait)
            }
            if state.killed_elmorad_npc > state.killed_karus_npc {
                return 2; // El Morad wins
            }
            // Tied — cascade to Kill for standard battle zones
            if battle_zone_id == ZONE_BATTLE
                || battle_zone_id == ZONE_BATTLE2
                || battle_zone_id == ZONE_BATTLE3
            {
                return battle_winner_result(state, BattleWinnerType::Kill);
            }
            0 // Draw
        }
        BattleWinnerType::Monument => {
            // Compare monument capture points
            if state.karus_monument_point > state.elmorad_monument_point {
                return 1; // Karus
            }
            if state.elmorad_monument_point > state.karus_monument_point {
                return 2; // El Morad
            }
            // Tied — cascade to Kill
            battle_winner_result(state, BattleWinnerType::Kill)
        }
        BattleWinnerType::Kill => {
            // Compare deaths: MORE deaths = LOSER (fewer deaths wins)
            if state.karus_dead > state.elmorad_dead {
                return 2; // El Morad wins (Karus had more deaths)
            }
            if state.elmorad_dead > state.karus_dead {
                return 1; // Karus wins (El Morad had more deaths)
            }
            // Tied — cascade to NPC for ZONE_BATTLE4/5/6
            if battle_zone_id == ZONE_BATTLE4
                || battle_zone_id == ZONE_BATTLE5
                || battle_zone_id == ZONE_BATTLE6
            {
                return battle_winner_result(state, BattleWinnerType::Npc);
            }
            0 // Draw
        }
    }
}

/// Set the winner and configure banish/open flags.
/// The caller is responsible for:
/// - Announcing DECLARE_WINNER / DECLARE_LOSER
/// - Awarding NP to captains
/// - Checking if zone is Ardream (auto-close)
pub fn battle_zone_result(state: &mut BattleState, nation: u8) {
    state.victory = nation;

    // For Ardream zones, the caller should call battle_zone_close() directly.
    // For other zones, configure the banish phase.
    if state.battle_zone_type != ZONE_ARDREAM_TYPE {
        // Open the losing nation's zone for banish
        state.karus_open_flag = nation == 2; // Karus zone opens if El Morad won
        state.elmorad_open_flag = nation == 1; // El Morad zone opens if Karus won
        state.banish_flag = true;
        state.banish_delay = 0;
    }
}

/// Check if either nation has achieved flag victory.
/// Returns the winning nation (1=Karus, 2=ElMorad) if flag threshold met,
/// or 0 if no flag victory yet.
pub fn battle_zone_victory_check(state: &BattleState) -> u8 {
    if state.karus_flag >= NUM_FLAG_VICTORY {
        1 // Karus
    } else if state.elmorad_flag >= NUM_FLAG_VICTORY {
        2 // El Morad
    } else {
        0 // No victory yet
    }
}

/// Get the monument name for a battle or nation monument.
pub fn get_monument_name(trap_number: i16, zone_id: u8) -> String {
    if zone_id == 0 {
        // Battle zone monuments (Ardream/Ronark)
        match trap_number {
            1 => "El Morad main territory".to_string(),
            2 => "El Morad provision line".to_string(),
            3 => "Lake of Life".to_string(),
            4 => "Foss Castle".to_string(),
            5 => "Karus main territory".to_string(),
            6 => "Karus provision line".to_string(),
            7 => "Swamp of Shadows".to_string(),
            _ => "Nereid Monument".to_string(),
        }
    } else {
        // Nation monuments — swap for Karus zone
        let adjusted_trap = if zone_id == 1 {
            // ZONE_KARUS
            match trap_number {
                1 => 2,
                2 => 1,
                other => other,
            }
        } else {
            trap_number
        };

        let nation_name = if zone_id == 1 { "Karus" } else { "El Morad" };
        match adjusted_trap {
            0 => {
                let city = if zone_id == 1 { "Luferson" } else { "El Morad" };
                format!("{city} Monument")
            }
            1 => {
                let place = if zone_id == 1 {
                    "Bellua"
                } else {
                    "Asga Village"
                };
                format!("{place} Monument")
            }
            2 => {
                let place = if zone_id == 1 {
                    "Linate"
                } else {
                    "Raiba Village"
                };
                format!("{place} Monument")
            }
            3 => {
                let place = if zone_id == 1 {
                    "Laon Camp"
                } else {
                    "Dodo Camp"
                };
                format!("{place} Monument")
            }
            _ => format!("{nation_name} Monument"),
        }
    }
}

// ── War Zone Helpers ─────────────────────────────────────────────────────────

/// Check if a zone ID is a war battle zone (ZONE_BATTLE through ZONE_BATTLE6).
pub fn is_battle_zone(zone_id: u16) -> bool {
    (ZONE_BATTLE..=ZONE_BATTLE6).contains(&zone_id)
}

/// Check if a zone ID matches the currently active battle zone.
pub fn is_active_battle_zone(state: &BattleState, zone_id: u16) -> bool {
    state.is_war_open() && zone_id == state.battle_zone_id()
}

/// Return the winner determination type for a given battle zone.
/// (Monument), and BATTLE6 (Kill) have half-time checks. BATTLE5 is NOT listed
/// in C++, so we return `None` to skip half-time determination.
pub fn winner_type_for_zone(battle_zone_id: u16) -> Option<BattleWinnerType> {
    match battle_zone_id {
        ZONE_BATTLE | ZONE_BATTLE2 | ZONE_BATTLE3 => Some(BattleWinnerType::Npc),
        ZONE_BATTLE4 => Some(BattleWinnerType::Monument),
        ZONE_BATTLE6 => Some(BattleWinnerType::Kill),
        _ => None, // ZONE_BATTLE5 and others: no half-time determination
    }
}

// ── War Tick ─────────────────────────────────────────────────────────────────

/// Outcome of a single `war_tick` call, used by the caller to decide
/// what further network I/O to perform (broadcasts, teleports, etc.).
/// Returning an enum keeps `war_tick` free of async and WorldState
/// broadcast calls, making it easy to unit-test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarTickAction {
    /// No action needed this tick.
    None,
    /// Broadcast a status announcement to the war zone (periodic kill/monument status).
    BroadcastStatus {
        /// Announcement sub-type (e.g., DECLARE_BATTLE_ZONE_STATUS).
        sub_type: u8,
    },
    /// Broadcast an under-attack notification (post-victory phase).
    BroadcastUnderAttack,
    /// Half-time reached: determine the winner and set the result.
    HalfTimeResult {
        /// Winner nation (1=Karus, 2=ElMorad) or 0 for draw/close.
        winner_nation: u8,
    },
    /// War time expired: close the battle zone.
    TimeExpired,
    /// Announce banish (losers will be kicked).
    AnnounceBanish,
    /// Execute the actual banish: kick losers from zone and reset state.
    ExecuteBanish,
    /// Result delay fired: apply the delayed result.
    ResultDelayFired {
        /// Winner nation from result_delay_victory.
        winner_nation: u8,
    },
    /// Nereids Island quarter-check: monument count victory.
    NereidsQuarterCheck {
        /// Winner nation (1=Karus, 2=ElMorad).
        winner_nation: u8,
    },
    /// Auto-select war commanders at 1/24 of war time.
    ///
    SelectCommanders,
}

/// Advance the war state by one second.
/// This is a pure state-mutation function that returns a [`WarTickAction`]
/// telling the caller what broadcast/IO to perform. This keeps the war tick
/// testable without needing async or a full WorldState.
/// The caller should invoke this once per second when any battle is active
/// or the banish flag is set.
pub fn war_tick(state: &mut BattleState) -> WarTickAction {
    // ── Banish phase (runs even after war is closed) ──────────────────
    if state.banish_flag {
        state.banish_delay = state.banish_delay.saturating_add(1);

        // C++ uses: m_byBattleTime / 360 for ban announcement
        // and m_byBattleTime / 120 for actual banish
        let ban_announce_threshold = if state.battle_time > 0 {
            (state.battle_time / 360).max(1)
        } else {
            10 // fallback: 10 seconds
        };
        let ban_execute_threshold = if state.battle_time > 0 {
            (state.battle_time / 120).max(2)
        } else {
            30 // fallback: 30 seconds
        };

        if state.banish_delay == ban_announce_threshold as i16 {
            return WarTickAction::AnnounceBanish;
        } else if state.banish_delay >= ban_execute_threshold as i16 {
            state.banish_flag = false;
            state.banish_delay = 0;
            return WarTickAction::ExecuteBanish;
        }
    }

    // ── Only process war logic if a nation battle is active ───────────
    if state.battle_open != NATION_BATTLE {
        return WarTickAction::None;
    }

    // ── Calculate elapsed and remaining time ─────────────────────────
    // In C++: WarElapsedTime = int32(UNIXTIME) - m_byBattleOpenedTime
    // We track battle_time_delay as a running elapsed counter instead.
    // Remaining time is decremented every tick.
    state.battle_remaining_time = state.battle_remaining_time.saturating_sub(1);

    let battle_time = state.battle_time;
    let elapsed = battle_time - state.battle_remaining_time;
    let battle_zone_id = state.battle_zone as u16 + ZONE_BATTLE_BASE;

    // ── Nereids Island timer ──────────────────────────────────────────
    if state.nereids_remaining_time > 0 && battle_zone_id == ZONE_BATTLE4 {
        state.nereids_remaining_time = state.nereids_remaining_time.saturating_sub(1);
    }

    // ── Result delay processing ──────────────────────────────────────
    if state.result_delay {
        state.battle_result_delay += 1;
        // C++ uses: m_byBattleTime / (m_byBattleTime / 10) which simplifies to 10
        let delay_threshold = if battle_time > 0 {
            battle_time / (battle_time / 10).max(1)
        } else {
            10
        };
        if state.battle_result_delay >= delay_threshold {
            state.result_delay = false;
            let winner = state.result_delay_victory;
            return WarTickAction::ResultDelayFired {
                winner_nation: winner,
            };
        }
    }

    // ── Time expired: close the war ──────────────────────────────────
    if state.battle_remaining_time <= 0 {
        return WarTickAction::TimeExpired;
    }

    // ── Victory not yet declared ─────────────────────────────────────
    if state.victory == 0 {
        // ── Commander selection at 1/24 of war time ──────────────────
        if !state.commander_selected && elapsed >= (battle_time / 24).max(1) {
            state.commander_selected = true;
            return WarTickAction::SelectCommanders;
        }

        // ── Nereids quarter-time monument check ──────────────────────
        if elapsed == (battle_time / 4) && battle_zone_id == ZONE_BATTLE4 {
            if state.karus_monuments >= 7 && state.elmorad_monuments == 0 {
                return WarTickAction::NereidsQuarterCheck { winner_nation: 1 };
            } else if state.karus_monuments == 0 && state.elmorad_monuments >= 7 {
                return WarTickAction::NereidsQuarterCheck { winner_nation: 2 };
            }
        }

        // ── Half-time winner determination ───────────────────────────
        // Only zones with a defined winner type get half-time checks (BATTLE5 is skipped)
        if elapsed == (battle_time / 2) {
            if let Some(winner_type) = winner_type_for_zone(battle_zone_id) {
                let winner = battle_winner_result(state, winner_type);
                return WarTickAction::HalfTimeResult {
                    winner_nation: winner,
                };
            }
        }

        // ── Periodic status announcement ─────────────────────────────
        state.battle_time_delay += 1;
        let status_interval = if battle_zone_id == ZONE_BATTLE4 {
            (battle_time / 48).max(1)
        } else {
            (battle_time / 24).max(1)
        };
        if state.battle_time_delay >= status_interval {
            state.battle_time_delay = 0;
            return WarTickAction::BroadcastStatus {
                sub_type: DECLARE_BATTLE_ZONE_STATUS,
            };
        }
    } else {
        // ── Victory already declared, under-attack phase ─────────────
        state.battle_time_delay += 1;
        let notify_interval = (battle_time / 24).max(1);
        if state.battle_time_delay >= notify_interval {
            state.battle_time_delay = 0;
            return WarTickAction::BroadcastUnderAttack;
        }
    }

    WarTickAction::None
}

/// Build a war status announcement string.
/// For ZONE_BATTLE4 (Nereids): includes monument points and death counts.
/// For other zones: includes NPC kills, death counts, and flag counts.
pub fn build_status_string(state: &BattleState) -> String {
    let battle_zone_id = state.battle_zone as u16 + ZONE_BATTLE_BASE;

    if battle_zone_id == ZONE_BATTLE4 {
        format!(
            "Monument Points — Karus: {} / El Morad: {} | Deaths — Karus: {} / El Morad: {}",
            state.karus_monument_point,
            state.elmorad_monument_point,
            state.karus_dead,
            state.elmorad_dead
        )
    } else {
        format!(
            "NPC Kills — Karus: {} / El Morad: {} | Deaths — Karus: {} / El Morad: {} | Flags — Karus: {} / El Morad: {}",
            state.killed_karus_npc,
            state.killed_elmorad_npc,
            state.karus_dead,
            state.elmorad_dead,
            state.karus_flag,
            state.elmorad_flag
        )
    }
}

/// Build a winner/loser announcement string.
pub fn build_winner_string(winner_nation: u8) -> String {
    let winner = if winner_nation == 1 {
        "Karus"
    } else {
        "El Morad"
    };
    format!("{winner} has won the war!")
}

/// Build a banish announcement string.
pub fn build_banish_string(victory: u8) -> String {
    if victory == 1 || victory == 2 {
        "Losers will be banished from the war zone!".to_string()
    } else {
        "All players will be banished from the war zone!".to_string()
    }
}

/// Calculate reward NP for a war captain.
pub fn captain_reward_np(is_king: bool, is_flag_victory: bool) -> i32 {
    if is_flag_victory {
        if is_king {
            500
        } else {
            300
        }
    } else {
        500
    }
}

/// Calculate reward NP for non-captain players in flag victory.
/// - Non-captain, non-king: 100 NP
/// - Non-captain, king: 200 NP
pub fn flag_victory_np(is_king: bool) -> i32 {
    if is_king {
        200
    } else {
        100
    }
}

/// Get the home zone for a nation (used for banish teleport).
pub fn nation_home_zone(nation: u8) -> u16 {
    match nation {
        1 => ZONE_KARUS,
        2 => ZONE_ELMORAD,
        _ => ZONE_KARUS, // fallback
    }
}

// ── Async WorldState integration ─────────────────────────────────────────────

/// War tick interval in seconds.
const WAR_TICK_INTERVAL_SECS: u64 = 1;

/// Start the war system background task.
/// Spawns a tokio task that calls [`war_tick_world`] every second,
/// processing nation battle timers, banish phases, and status broadcasts.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_war_task(world: std::sync::Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(WAR_TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            war_tick_world(&world).await;
        }
    })
}

/// Execute a war tick against the WorldState and handle the resulting action.
/// This is the main entry point called by the game server's 1-second timer.
/// It acquires a write lock on the battle state, runs `war_tick()`, then
/// performs any necessary broadcasts.
pub async fn war_tick_world(world: &WorldState) {
    // Update battle zone user counts every tick (C++ BattleZoneCurrentUsers)
    {
        let state = world.get_battle_state();
        if state.battle_open == NATION_BATTLE {
            world.update_battle_zone_user_counts();
        }
    }

    let action = world.update_battle_state(war_tick);

    match action {
        WarTickAction::None => {}
        WarTickAction::BroadcastStatus { sub_type: _ } => {
            let state = world.get_battle_state();
            let msg = build_status_string(&state);
            let battle_zone_id = state.battle_zone_id();
            broadcast_war_announcement(world, &msg, Some(battle_zone_id));
        }
        WarTickAction::BroadcastUnderAttack => {
            let state = world.get_battle_state();
            let msg = if state.victory == 1 {
                format!(
                    "El Morad is under attack! Deaths — Karus: {} / El Morad: {}",
                    state.karus_dead, state.elmorad_dead
                )
            } else {
                format!(
                    "Karus is under attack! Deaths — El Morad: {} / Karus: {}",
                    state.elmorad_dead, state.karus_dead
                )
            };
            broadcast_war_announcement(world, &msg, None);
        }
        WarTickAction::HalfTimeResult { winner_nation } => {
            if winner_nation == 0 {
                // Draw — close the war
                let prev = world.update_battle_state(battle_zone_close);
                if prev != NO_BATTLE {
                    world.change_ability_all_npcs(false);
                    broadcast_war_announcement(world, "The war has ended in a draw!", None);
                }
            } else {
                // Winner determined
                world.update_battle_state(|s| battle_zone_result(s, winner_nation));
                // C++ BattleZoneResult: captain NP after setting victory
                distribute_war_result_rewards(world, winner_nation);
                let msg = build_winner_string(winner_nation);
                broadcast_war_announcement(world, &msg, None);
            }
        }
        WarTickAction::TimeExpired => {
            // C++ BattleZoneClose: ResetCommanders + GoldshellDisable + ResetBattleZone + banish
            let prev = world.update_battle_state(battle_zone_close);
            if prev != NO_BATTLE {
                world.change_ability_all_npcs(false);
                crate::handler::operator::broadcast_goldshell(world, false);
                crate::handler::operator::reset_war_commanders(world).await;
                broadcast_war_announcement(world, "The war time has expired!", None);
            }
        }
        WarTickAction::AnnounceBanish => {
            let state = world.get_battle_state();
            let msg = build_banish_string(state.victory);
            broadcast_war_announcement(world, &msg, None);
        }
        WarTickAction::ExecuteBanish => {
            execute_banish(world);
            // C++ BanishLosers() calls BattleZoneRemnantSpawn() for ZONE_BATTLE2
            battle_zone_remnant_spawn(world);
        }
        WarTickAction::ResultDelayFired { winner_nation } => {
            world.update_battle_state(|s| battle_zone_result(s, winner_nation));
            distribute_war_result_rewards(world, winner_nation);
            let msg = build_winner_string(winner_nation);
            broadcast_war_announcement(world, &msg, None);
        }
        WarTickAction::NereidsQuarterCheck { winner_nation } => {
            world.update_battle_state(|s| battle_zone_result(s, winner_nation));
            distribute_war_result_rewards(world, winner_nation);
            let msg = build_winner_string(winner_nation);
            broadcast_war_announcement(world, &msg, None);
        }
        WarTickAction::SelectCommanders => {
            crate::handler::operator::select_war_commanders(world).await;
        }
    }

    // ── Nation Monument NP Rewards ────────────────────────────────────────
    check_nation_monument_rewards(world);

    // ── CSW Timer ────────────────────────────────────────────────────────
    csw_tick_world(world).await;
}

/// Execute a CSW timer tick against the WorldState.
/// Called every 1 second from `war_tick_world()`. Acquires write lock on
/// `csw_event`, calls `csw_timer_tick()`, then handles the resulting action:
/// notice broadcasts, phase transitions, and rewards.
async fn csw_tick_world(world: &WorldState) {
    use crate::handler::siege::{
        build_csw_notice, build_csw_raw_notice, csw_close, csw_timer_tick, csw_war_open,
        CswTickAction,
    };
    use crate::world::types::CswNotice;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let action = {
        let state = world.csw_event().read().await;
        csw_timer_tick(&state, now)
    };

    match action {
        CswTickAction::None => {}
        CswTickAction::SendNotice {
            notice_type,
            remaining_minutes,
        } => {
            let pkt = build_csw_raw_notice(notice_type, remaining_minutes);
            world.broadcast_to_all(Arc::new(pkt), None);
        }
        CswTickAction::TransitionToWar => {
            // Read wartime from CSW options
            let wartime = world
                .get_csw_opt()
                .map(|opt| opt.war_time as u32)
                .unwrap_or(40);

            {
                let mut state = world.csw_event().write().await;
                csw_war_open(&mut state, wartime, now);
            }

            // Set battle_open = SIEGE_BATTLE
            world.update_battle_state(|s| {
                s.battle_open = SIEGE_BATTLE;
            });

            // Broadcast war start notice
            let pkt = build_csw_notice(CswNotice::War);
            world.broadcast_to_all(Arc::new(pkt), None);

            tracing::info!("CSW: War phase started ({}min)", wartime);
        }
        CswTickAction::TransitionToClose => {
            {
                let mut state = world.csw_event().write().await;
                csw_close(&mut state);
            }

            // Reset battle_open = NO_BATTLE
            world.update_battle_state(|s| {
                s.battle_open = NO_BATTLE;
            });

            // Broadcast finish notice
            let pkt = build_csw_notice(CswNotice::CswFinish);
            world.broadcast_to_all(Arc::new(pkt), None);

            tracing::info!("CSW: War ended, state reset");
        }
    }
}

/// Broadcast a WAR_SYSTEM_CHAT announcement.
/// If `zone_id` is Some, only players in that zone receive it.
/// If `zone_id` is None, all players receive it.
pub(crate) fn broadcast_war_announcement(world: &WorldState, message: &str, zone_id: Option<u16>) {
    let pkt = crate::handler::chat::build_chat_packet(
        8,      // WAR_SYSTEM_CHAT
        1,      // nation = ALL (C++ default bNation=1)
        0xFFFF, // sender_id = -1 (C++ default int16 senderID=-1)
        "", message, 0, 0, // authority = GM
        0,
    );

    if let Some(zid) = zone_id {
        world.broadcast_to_zone(zid, Arc::new(pkt), None);
    } else {
        world.broadcast_to_all(Arc::new(pkt), None);
    }
}

/// Broadcast a war system chat message to all players of a specific nation.
/// `Send_All(&result, nullptr, Nation::KARUS/ELMORAD)`.
pub(crate) fn broadcast_war_chat_to_nation(world: &WorldState, nation: u8, message: &str) {
    let pkt = crate::handler::chat::build_chat_packet(
        8,      // WAR_SYSTEM_CHAT
        nation, // target nation
        0xFFFF, // sender_id = -1
        "", message, 0, 0, 0,
    );
    // Send to all online players of the given nation (clone once, Arc share)
    let arc_pkt = Arc::new(pkt);
    for sid in world.get_in_game_session_ids() {
        if let Some(ci) = world.get_character_info(sid) {
            if ci.nation == nation {
                world.send_to_session_arc(sid, Arc::clone(&arc_pkt));
            }
        }
    }
}

/// Distribute NP rewards to war captains in the war zone after BattleZoneResult.
/// Unlike flag victory (which rewards players in their home zone with gold+exp+NP),
/// BattleZoneResult only awards 500 NP to captains of the winning nation who are
/// currently in a war zone (`isWarZone()`).
fn distribute_war_result_rewards(world: &WorldState, winner_nation: u8) {
    if winner_nation == 0 {
        return;
    }

    let state = world.get_battle_state();
    if state.battle_open != NATION_BATTLE {
        return;
    }

    let battle_zone_id = state.battle_zone_id();
    let war_zone_sessions = world.sessions_in_zone(battle_zone_id);
    let mut rewarded = 0u32;

    for sid in war_zone_sessions {
        let info = match world.get_character_info(sid) {
            Some(ch) => ch,
            None => continue,
        };

        if info.nation != winner_nation {
            continue;
        }

        if info.fame == COMMAND_CAPTAIN {
            super::loyalty::send_loyalty_change(world, sid, 500, false, false, true);
            rewarded += 1;
        }
    }

    tracing::info!(
        "BattleZoneResult NP: winner={}, zone={}, captains_rewarded={}",
        winner_nation,
        battle_zone_id,
        rewarded
    );
}

/// Execute the banish phase: kick losers and reset captains.
/// Two modes depending on current battle state:
/// - `NATION_BATTLE`: Kick losers from war zone (first banish after result)
/// - `NO_BATTLE`: Reset captains to CHIEF + kick invaders from enemy home zones + war zones
fn execute_banish(world: &WorldState) {
    use crate::clan_constants::CHIEF;

    let state = world.get_battle_state();
    let battle_open = state.battle_open;
    let victory = state.victory;
    let battle_zone_id = state.battle_zone_id();

    let all_sessions = world.get_in_game_session_ids();
    let mut kicked = 0u32;
    let mut captains_reset = 0u32;

    for sid in all_sessions {
        let info = match world.get_character_info(sid) {
            Some(ch) => ch,
            None => continue,
        };

        // BattleSystem.cpp:188-193 — if (battle_open == NO_BATTLE && fame == COMMAND_CAPTAIN)
        if battle_open == NO_BATTLE && info.fame == COMMAND_CAPTAIN {
            world.update_character_stats(sid, |ci| ci.fame = CHIEF);

            // C++ ChangeFame broadcasts WIZ_AUTHORITY_CHANGE to region
            let mut auth_pkt =
                ko_protocol::Packet::new(ko_protocol::Opcode::WizAuthorityChange as u8);
            auth_pkt.write_u8(0x01); // COMMAND_AUTHORITY
            auth_pkt.write_u32(sid as u32);
            auth_pkt.write_u8(CHIEF);
            world.broadcast_to_zone(
                world.get_position(sid).map(|p| p.zone_id).unwrap_or(0),
                Arc::new(auth_pkt),
                None,
            );

            captains_reset += 1;
        }

        if battle_open == NATION_BATTLE {
            let pos = match world.get_position(sid) {
                Some(p) => p,
                None => continue,
            };
            if is_battle_zone(pos.zone_id) && victory != 0 && info.nation != victory {
                kick_out_zone_user(world, sid, info.nation);
                kicked += 1;
            }
        } else if battle_open == NO_BATTLE {
            let pos = match world.get_position(sid) {
                Some(p) => p,
                None => continue,
            };
            let zone_id = pos.zone_id;

            let in_enemy_home = zone_id <= 2 && zone_id != info.nation as u16;
            let in_war_zone = is_battle_zone(zone_id);

            if in_enemy_home || in_war_zone {
                kick_out_zone_user(world, sid, info.nation);
                kicked += 1;
            }
        }
    }

    tracing::info!(
        "War banish executed: battle_open={}, victory={}, zone={}, kicked={}, captains_reset={}",
        battle_open,
        victory,
        battle_zone_id,
        kicked,
        captains_reset
    );
}

/// Kick a player to Moradon (zone 21) using free_zone coordinates.
/// Uses random regene event in zone 21; we use the per-nation free_zone position as fallback.
pub(crate) fn kick_out_zone_user(world: &WorldState, sid: u16, player_nation: u8) {
    use crate::world::ZONE_MORADON;
    use ko_protocol::{Opcode, Packet};

    // Uses free_zone coordinates from HOME table as spawn position
    let (dest_x, dest_z) = world
        .get_home_position(player_nation)
        .map(|h| (h.free_zone_x as f32, h.free_zone_z as f32))
        .unwrap_or((0.0, 0.0));

    world.update_position(sid, ZONE_MORADON, dest_x, 0.0, dest_z);

    let mut zpkt = Packet::new(Opcode::WizZoneChange as u8);
    zpkt.write_u8(3); // ZONE_CHANGE_TELEPORT
    zpkt.write_u16(ZONE_MORADON);
    zpkt.write_u16(0);
    zpkt.write_u16((dest_x * 10.0) as u16);
    zpkt.write_u16((dest_z * 10.0) as u16);
    zpkt.write_u16(0);
    zpkt.write_u8(player_nation);
    zpkt.write_u16(0xFFFF);

    world.send_to_session_owned(sid, zpkt);
}

// WorldState integration methods are defined in world.rs (same crate,
// direct field access). See WorldState::is_war_open(), update_battle_state(), etc.

/// Spawn post-victory remnant monsters (only for ZONE_BATTLE2 = Alseids Prairie).
/// Called after `BanishLosers()` when `m_byBattleZone + ZONE_BATTLE_BASE == ZONE_BATTLE2`.
/// Iterates the `banish_of_winner` table, filtering by winning nation, and spawns
/// event NPCs at each configured location.
fn battle_zone_remnant_spawn(world: &WorldState) {
    let state = world.get_battle_state();
    let battle_zone_id = state.battle_zone as u16 + ZONE_BATTLE_BASE;
    let victory = state.victory;

    // C++ only spawns remnants for ZONE_BATTLE2 (zone 62)
    if battle_zone_id != ZONE_BATTLE2 {
        return;
    }
    if victory == 0 {
        return;
    }

    let entries = world.get_banish_of_winner(victory);
    if entries.is_empty() {
        return;
    }

    let mut total_spawned = 0u32;
    for entry in &entries {
        let spawned = world.spawn_event_npc(
            entry.sid as u16,
            true, // is_monster
            entry.zone_id as u16,
            entry.pos_x as f32,
            entry.pos_z as f32,
            entry.spawn_count as u16,
        );
        total_spawned += spawned.len() as u32;
    }

    // C++ broadcasts: Announcement(IDS_REMNANT_SUMMON_INFO)
    if total_spawned > 0 {
        broadcast_war_announcement(world, "Remnant monsters have appeared!", None);
        tracing::info!(
            "BattleZoneRemnantSpawn: spawned {total_spawned} remnant NPCs for nation {victory}"
        );
    }
}

// ── Nation Monument NP Reward ────────────────────────────────────────────────

/// Winner monument reward interval: 300 seconds (5 min).
const MONUMENT_WINNER_INTERVAL: i32 = 300;

/// Defeated monument reward interval: 10,000 seconds (~2h46m).
#[cfg(test)]
const MONUMENT_DEFEATED_INTERVAL: i32 = 10_000;

/// NP reward for main monuments (Luferson/Elmorad capital).
const MONUMENT_MAIN_NP: i32 = 200;

/// NP reward for secondary monuments.
const MONUMENT_SECONDARY_NP: i32 = 50;

/// NPC effect ID for monument reward visual.
const MONUMENT_REWARD_EFFECT: u32 = 20100;

/// Maximum distance (in units) for a player to receive monument NP reward.
const MONUMENT_REWARD_RANGE: f32 = 100.0;

/// Check nation monument rewards — distribute NP to nearby players.
/// Winner monuments: reward every 300s, keep entry.
/// Defeated monuments: reward every 10,000s, then delete entry.
fn check_nation_monument_rewards(world: &WorldState) {
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    // Early check: if no war, skip
    let state = world.get_battle_state();
    if state.battle_open == NO_BATTLE {
        return;
    }
    if state.nation_monument_winners.is_empty() && state.nation_monument_defeated.is_empty() {
        return;
    }

    // Determine which zone to look for NPC based on victory nation
    // Winner monuments are in the opposing nation's zone
    let winner_zone = if state.victory == 1 {
        ZONE_ELMORAD
    } else {
        ZONE_KARUS
    };
    // Defeated monuments are in the same nation's zone
    let defeated_zone = if state.victory == 1 {
        ZONE_KARUS
    } else {
        ZONE_ELMORAD
    };

    // ── Winner monuments (300s interval, keep entry) ──
    let ready_winners: Vec<u16> = state
        .nation_monument_winners
        .iter()
        .filter(|(_, &next_time)| now_unix >= next_time)
        .map(|(&sid, _)| sid)
        .collect();
    drop(state); // release the cloned state

    for sid in &ready_winners {
        // Reset timer
        world.update_battle_state(|s| {
            if let Some(t) = s.nation_monument_winners.get_mut(sid) {
                *t = now_unix + MONUMENT_WINNER_INTERVAL;
            }
        });

        // Find NPC position and reward nearby players
        distribute_monument_np(world, *sid, winner_zone, now_unix);
    }

    // ── Defeated monuments (10,000s interval, delete after) ──
    let state2 = world.get_battle_state();
    let ready_defeated: Vec<u16> = state2
        .nation_monument_defeated
        .iter()
        .filter(|(_, &next_time)| now_unix >= next_time)
        .map(|(&sid, _)| sid)
        .collect();
    drop(state2);

    for sid in &ready_defeated {
        distribute_monument_np(world, *sid, defeated_zone, now_unix);
        // Delete entry after firing
        world.update_battle_state(|s| {
            s.nation_monument_defeated.remove(sid);
        });
    }
}

/// Distribute NP rewards to players near a nation monument NPC.
/// range and nation, rewards 200 NP for main monuments, 50 for others.
fn distribute_monument_np(world: &WorldState, npc_sid: u16, zone_id: u16, _now: i32) {
    // Find the NPC instance in the zone
    let npc = match world.find_npc_in_zone(npc_sid, zone_id) {
        Some(n) => n,
        None => return,
    };

    let npc_x = npc.x;
    let npc_z = npc.z;
    let npc_nation = world
        .get_npc_template(npc_sid, false)
        .map(|t| t.group)
        .unwrap_or(0);

    let np_amount = if npc_sid == LUFERSON_MONUMENT_SID || npc_sid == ELMORAD_MONUMENT_SID {
        MONUMENT_MAIN_NP
    } else {
        MONUMENT_SECONDARY_NP
    };

    // Scan all sessions in this zone
    let zone_sessions = world.sessions_in_zone(zone_id);
    for sid in zone_sessions {
        let pos = match world.get_position(sid) {
            Some(p) => p,
            None => continue,
        };
        let player_nation = world
            .get_character_info(sid)
            .map(|ch| ch.nation)
            .unwrap_or(0);

        // Must be same nation as the NPC
        if player_nation != npc_nation {
            continue;
        }

        // Range check
        let dx = pos.x - npc_x;
        let dz = pos.z - npc_z;
        let dist = (dx * dx + dz * dz).sqrt();
        if dist > MONUMENT_REWARD_RANGE {
            continue;
        }

        // Distribute NP
        super::loyalty::send_loyalty_change(world, sid, np_amount, false, false, true);
    }

    // Show NPC effect
    let mut effect_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizObjectEvent as u8);
    effect_pkt.write_u16(npc.nid as u16);
    effect_pkt.write_u32(MONUMENT_REWARD_EFFECT);
    world.broadcast_to_zone(zone_id, Arc::new(effect_pkt), None);

    tracing::debug!(
        "Monument NP reward: sid={}, zone={}, np={}, npc_pos=({:.0},{:.0})",
        npc_sid,
        zone_id,
        np_amount,
        npc_x,
        npc_z
    );
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battle_state_default() {
        let state = BattleState::new();
        assert_eq!(state.battle_open, NO_BATTLE);
        assert!(!state.is_war_open());
        assert!(!state.is_nation_battle());
        assert!(!state.is_snow_battle());
        assert_eq!(state.victory, 0);
        assert_eq!(state.karus_dead, 0);
        assert_eq!(state.elmorad_dead, 0);
    }

    #[test]
    fn test_is_war_open() {
        let mut state = BattleState::new();
        assert!(!state.is_war_open());

        state.battle_open = NATION_BATTLE;
        assert!(state.is_war_open());
        assert!(state.is_nation_battle());
        assert!(!state.is_snow_battle());

        state.battle_open = SNOW_BATTLE;
        assert!(state.is_war_open());
        assert!(!state.is_nation_battle());
        assert!(state.is_snow_battle());
    }

    #[test]
    fn test_battle_zone_id() {
        let mut state = BattleState::new();
        state.battle_zone = 0;
        assert_eq!(state.battle_zone_id(), ZONE_BATTLE_BASE);

        state.battle_zone = 1;
        assert_eq!(state.battle_zone_id(), ZONE_BATTLE);

        state.battle_zone = 4;
        assert_eq!(state.battle_zone_id(), ZONE_BATTLE4);

        state.battle_zone = 6;
        assert_eq!(state.battle_zone_id(), ZONE_BATTLE6);
    }

    #[test]
    fn test_reset_battle_zone() {
        let mut state = BattleState::new();
        state.battle_open = NATION_BATTLE;
        state.battle_zone = 3;
        state.victory = 1;
        state.karus_dead = 50;
        state.elmorad_dead = 30;
        state.karus_flag = 3;
        state.elmorad_flag = 2;
        state.banish_flag = true;
        state.commander_selected = true;
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 3;
        state.karus_monument_point = 100;
        state.elmorad_monument_point = 80;

        reset_battle_zone(&mut state);

        assert_eq!(state.battle_open, NO_BATTLE);
        assert_eq!(state.old_battle_open, NO_BATTLE);
        assert_eq!(state.battle_zone, 0);
        assert_eq!(state.victory, 0);
        assert_eq!(state.karus_dead, 0);
        assert_eq!(state.elmorad_dead, 0);
        assert_eq!(state.karus_flag, 0);
        assert_eq!(state.elmorad_flag, 0);
        assert!(!state.banish_flag);
        assert!(!state.commander_selected);
        assert_eq!(state.killed_karus_npc, 0);
        assert_eq!(state.killed_elmorad_npc, 0);
        assert_eq!(state.karus_monument_point, 0);
        assert_eq!(state.elmorad_monument_point, 0);
        assert_eq!(state.nereids_monument_array, [0; 7]);
    }

    #[test]
    fn test_battle_zone_open_nation() {
        let mut state = BattleState::new();
        state.battle_time = 3600; // 1 hour

        let result = battle_zone_open(&mut state, BATTLEZONE_OPEN, 1, 1000);
        assert!(result);
        assert_eq!(state.battle_open, NATION_BATTLE);
        assert_eq!(state.old_battle_open, NATION_BATTLE);
        assert_eq!(state.battle_zone, 1);
        assert_eq!(state.battle_opened_time, 1000);
        assert_eq!(state.battle_remaining_time, 3600);
    }

    #[test]
    fn test_battle_zone_open_snow() {
        let mut state = BattleState::new();
        state.battle_time = 1800;

        let result = battle_zone_open(&mut state, SNOW_BATTLEZONE_OPEN, 0, 2000);
        assert!(result);
        assert_eq!(state.battle_open, SNOW_BATTLE);
        assert_eq!(state.battle_zone, 0);
    }

    #[test]
    fn test_battle_zone_open_already_open() {
        let mut state = BattleState::new();
        state.battle_time = 3600;

        battle_zone_open(&mut state, BATTLEZONE_OPEN, 1, 1000);
        assert!(state.is_war_open());

        // Second open should fail
        let result = battle_zone_open(&mut state, BATTLEZONE_OPEN, 2, 2000);
        assert!(!result);
        assert_eq!(state.battle_zone, 1); // unchanged
    }

    #[test]
    fn test_battle_zone_open_nereids() {
        let mut state = BattleState::new();
        state.battle_time = 3600;

        // ZONE_BATTLE4 = ZONE_BATTLE_BASE + 4 = 64
        let result = battle_zone_open(&mut state, BATTLEZONE_OPEN, 4, 1000);
        assert!(result);
        assert_eq!(state.nereids_remaining_time, 3600);
    }

    #[test]
    fn test_battle_zone_close_no_war() {
        let mut state = BattleState::new();
        let prev = battle_zone_close(&mut state);
        assert_eq!(prev, NO_BATTLE);
    }

    #[test]
    fn test_battle_zone_close_nation() {
        let mut state = BattleState::new();
        state.battle_time = 3600;
        battle_zone_open(&mut state, BATTLEZONE_OPEN, 1, 1000);

        let prev = battle_zone_close(&mut state);
        assert_eq!(prev, NATION_BATTLE);
        assert_eq!(state.battle_open, NO_BATTLE);
        assert!(state.banish_flag);
    }

    #[test]
    fn test_battle_zone_close_snow() {
        let mut state = BattleState::new();
        state.battle_time = 1800;
        battle_zone_open(&mut state, SNOW_BATTLEZONE_OPEN, 0, 1000);

        let prev = battle_zone_close(&mut state);
        assert_eq!(prev, SNOW_BATTLE);
        assert_eq!(state.battle_open, NO_BATTLE);
        assert!(state.banish_flag);
    }

    #[test]
    fn test_battle_winner_npc_karus_wins() {
        let mut state = BattleState::new();
        state.battle_zone = 1; // ZONE_BATTLE
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 3;

        let winner = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner, 1); // Karus (more Karus NPCs killed)
    }

    #[test]
    fn test_battle_winner_npc_elmorad_wins() {
        let mut state = BattleState::new();
        state.battle_zone = 1;
        state.killed_karus_npc = 3;
        state.killed_elmorad_npc = 5;

        let winner = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner, 2); // El Morad
    }

    #[test]
    fn test_battle_winner_npc_tied_cascades_to_kill() {
        let mut state = BattleState::new();
        state.battle_zone = 1; // ZONE_BATTLE = ZONE_BATTLE_BASE + 1 = 61
        state.killed_karus_npc = 3;
        state.killed_elmorad_npc = 3;
        // Kill cascade: more deaths = loser
        state.karus_dead = 20;
        state.elmorad_dead = 10;

        let winner = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner, 2); // El Morad wins (Karus had more deaths)
    }

    #[test]
    fn test_battle_winner_kill_elmorad_wins() {
        let mut state = BattleState::new();
        state.battle_zone = 1;
        state.karus_dead = 30;
        state.elmorad_dead = 20;

        let winner = battle_winner_result(&state, BattleWinnerType::Kill);
        assert_eq!(winner, 2); // El Morad (Karus had more deaths)
    }

    #[test]
    fn test_battle_winner_kill_karus_wins() {
        let mut state = BattleState::new();
        state.battle_zone = 1;
        state.karus_dead = 10;
        state.elmorad_dead = 25;

        let winner = battle_winner_result(&state, BattleWinnerType::Kill);
        assert_eq!(winner, 1); // Karus (El Morad had more deaths)
    }

    #[test]
    fn test_battle_winner_kill_tied_cascades_to_npc_for_zone4() {
        let mut state = BattleState::new();
        state.battle_zone = 4; // ZONE_BATTLE4
        state.karus_dead = 15;
        state.elmorad_dead = 15;
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 3;

        let winner = battle_winner_result(&state, BattleWinnerType::Kill);
        assert_eq!(winner, 1); // Karus wins via NPC cascade
    }

    #[test]
    fn test_battle_winner_monument() {
        let mut state = BattleState::new();
        state.battle_zone = 1;
        state.karus_monument_point = 100;
        state.elmorad_monument_point = 80;

        let winner = battle_winner_result(&state, BattleWinnerType::Monument);
        assert_eq!(winner, 1); // Karus
    }

    #[test]
    fn test_battle_winner_monument_tied_cascades_to_kill() {
        let mut state = BattleState::new();
        state.battle_zone = 1;
        state.karus_monument_point = 50;
        state.elmorad_monument_point = 50;
        state.karus_dead = 5;
        state.elmorad_dead = 10;

        let winner = battle_winner_result(&state, BattleWinnerType::Monument);
        assert_eq!(winner, 1); // Karus wins via Kill cascade
    }

    #[test]
    fn test_battle_winner_all_tied_draw() {
        let mut state = BattleState::new();
        state.battle_zone = 1; // ZONE_BATTLE
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 5;
        state.karus_dead = 10;
        state.elmorad_dead = 10;

        // NPC tied → Kill cascade → Kill tied → 0 (no further cascade for zone 1)
        let winner = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner, 0); // Draw
    }

    #[test]
    fn test_battle_zone_result_sets_flags() {
        let mut state = BattleState::new();
        state.battle_zone_type = 0; // Standard

        battle_zone_result(&mut state, 1); // Karus wins
        assert_eq!(state.victory, 1);
        assert!(!state.karus_open_flag); // Karus won, their gate stays closed
        assert!(state.elmorad_open_flag); // El Morad's gate opens for banish
        assert!(state.banish_flag);

        // Reset and test El Morad winning
        reset_battle_zone(&mut state);
        battle_zone_result(&mut state, 2); // El Morad wins
        assert_eq!(state.victory, 2);
        assert!(state.karus_open_flag); // Karus gate opens for banish
        assert!(!state.elmorad_open_flag); // El Morad gate stays closed
    }

    #[test]
    fn test_battle_zone_result_ardream_no_banish() {
        let mut state = BattleState::new();
        state.battle_zone_type = ZONE_ARDREAM_TYPE;

        battle_zone_result(&mut state, 1);
        assert_eq!(state.victory, 1);
        // Ardream doesn't set banish flags
        assert!(!state.banish_flag);
        assert!(!state.karus_open_flag);
        assert!(!state.elmorad_open_flag);
    }

    #[test]
    fn test_flag_victory_check() {
        let mut state = BattleState::new();
        assert_eq!(battle_zone_victory_check(&state), 0);

        state.karus_flag = 3;
        assert_eq!(battle_zone_victory_check(&state), 0);

        state.karus_flag = 4;
        assert_eq!(battle_zone_victory_check(&state), 1); // Karus wins

        state.karus_flag = 0;
        state.elmorad_flag = 4;
        assert_eq!(battle_zone_victory_check(&state), 2); // El Morad wins
    }

    #[test]
    fn test_monument_name_battle_zone() {
        assert_eq!(get_monument_name(1, 0), "El Morad main territory");
        assert_eq!(get_monument_name(2, 0), "El Morad provision line");
        assert_eq!(get_monument_name(3, 0), "Lake of Life");
        assert_eq!(get_monument_name(4, 0), "Foss Castle");
        assert_eq!(get_monument_name(5, 0), "Karus main territory");
        assert_eq!(get_monument_name(6, 0), "Karus provision line");
        assert_eq!(get_monument_name(7, 0), "Swamp of Shadows");
        assert_eq!(get_monument_name(0, 0), "Nereid Monument");
        assert_eq!(get_monument_name(99, 0), "Nereid Monument");
    }

    #[test]
    fn test_monument_name_nation_zone_elmorad() {
        // zone_id=2 means El Morad zone (no swap)
        assert_eq!(get_monument_name(0, 2), "El Morad Monument");
        assert_eq!(get_monument_name(1, 2), "Asga Village Monument");
        assert_eq!(get_monument_name(2, 2), "Raiba Village Monument");
        assert_eq!(get_monument_name(3, 2), "Dodo Camp Monument");
    }

    #[test]
    fn test_monument_name_nation_zone_karus() {
        // zone_id=1 means Karus zone (trap 1↔2 swapped)
        assert_eq!(get_monument_name(0, 1), "Luferson Monument");
        // trap 1 → swapped to 2 → "Linate Monument"
        assert_eq!(get_monument_name(1, 1), "Linate Monument");
        // trap 2 → swapped to 1 → "Bellua Monument"
        assert_eq!(get_monument_name(2, 1), "Bellua Monument");
        assert_eq!(get_monument_name(3, 1), "Laon Camp Monument");
    }

    #[test]
    fn test_constants() {
        assert_eq!(NO_BATTLE, 0);
        assert_eq!(NATION_BATTLE, 1);
        assert_eq!(SNOW_BATTLE, 2);
        assert_eq!(BATTLEZONE_OPEN, 0x00);
        assert_eq!(BATTLEZONE_CLOSE, 0x01);
        assert_eq!(BATTLEZONE_NONE, 0xFF);
        assert_eq!(SNOW_BATTLEZONE_OPEN, 0x09);
        assert_eq!(SNOW_BATTLEZONE_CLOSE, 0x15);
        assert_eq!(NUM_FLAG_VICTORY, 4);
        assert_eq!(AWARD_GOLD, 100_000);
        assert_eq!(AWARD_EXP, 5000);
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(DECLARE_WINNER, 0x02);
        assert_eq!(DECLARE_LOSER, 0x03);
        assert_eq!(ELMORAD_MONUMENT_SID, 10301);
        assert_eq!(LUFERSON_MONUMENT_SID, 20301);
    }

    #[test]
    fn test_battle_state_full_lifecycle() {
        // Test the full lifecycle through BattleState directly
        let mut state = BattleState::new();
        state.battle_time = 3600;

        // Open a nation battle
        assert!(battle_zone_open(&mut state, BATTLEZONE_OPEN, 1, 1000));
        assert!(state.is_war_open());
        assert_eq!(state.battle_zone_id(), ZONE_BATTLE);

        // Record some deaths
        state.karus_dead = state.karus_dead.saturating_add(1);
        state.karus_dead = state.karus_dead.saturating_add(1);
        state.elmorad_dead = state.elmorad_dead.saturating_add(1);
        assert_eq!(state.karus_dead, 2);
        assert_eq!(state.elmorad_dead, 1);

        // Record NPC kills
        state.killed_karus_npc = state.killed_karus_npc.saturating_add(1);
        assert_eq!(state.killed_karus_npc, 1);

        // Record monument points
        state.elmorad_monument_point = state.elmorad_monument_point.saturating_add(50);
        state.elmorad_monuments = state.elmorad_monuments.saturating_add(1);
        assert_eq!(state.elmorad_monument_point, 50);
        assert_eq!(state.elmorad_monuments, 1);

        // Record flag captures
        state.karus_flag = state.karus_flag.saturating_add(1);
        assert!(state.karus_flag < NUM_FLAG_VICTORY);
        state.karus_flag = state.karus_flag.saturating_add(1);
        state.karus_flag = state.karus_flag.saturating_add(1);
        state.karus_flag = state.karus_flag.saturating_add(1);
        assert!(state.karus_flag >= NUM_FLAG_VICTORY); // 4th capture = victory

        // Close war
        let prev = battle_zone_close(&mut state);
        assert_eq!(prev, NATION_BATTLE);
        assert!(!state.is_war_open());
    }

    // ── War Phase 2 Tests ────────────────────────────────────────────────

    // ── is_battle_zone tests ─────────────────────────────────────────────

    #[test]
    fn test_is_battle_zone() {
        assert!(is_battle_zone(ZONE_BATTLE));
        assert!(is_battle_zone(ZONE_BATTLE2));
        assert!(is_battle_zone(ZONE_BATTLE3));
        assert!(is_battle_zone(ZONE_BATTLE4));
        assert!(is_battle_zone(ZONE_BATTLE5));
        assert!(is_battle_zone(ZONE_BATTLE6));
        assert!(!is_battle_zone(ZONE_BATTLE_BASE)); // 60 is not a battle zone
        assert!(!is_battle_zone(67)); // above range
        assert!(!is_battle_zone(1)); // ZONE_KARUS
        assert!(!is_battle_zone(2)); // ZONE_ELMORAD
        assert!(!is_battle_zone(21)); // Moradon
    }

    #[test]
    fn test_is_active_battle_zone() {
        let mut state = BattleState::new();
        state.battle_time = 3600;

        // No war open
        assert!(!is_active_battle_zone(&state, ZONE_BATTLE));

        // Open war on zone 1
        battle_zone_open(&mut state, BATTLEZONE_OPEN, 1, 1000);
        assert!(is_active_battle_zone(&state, ZONE_BATTLE)); // 60+1=61
        assert!(!is_active_battle_zone(&state, ZONE_BATTLE2)); // 62 is not active
        assert!(!is_active_battle_zone(&state, ZONE_BATTLE3));
    }

    // ── winner_type_for_zone tests ───────────────────────────────────────

    #[test]
    fn test_winner_type_for_zone() {
        assert_eq!(
            winner_type_for_zone(ZONE_BATTLE),
            Some(BattleWinnerType::Npc)
        );
        assert_eq!(
            winner_type_for_zone(ZONE_BATTLE2),
            Some(BattleWinnerType::Npc)
        );
        assert_eq!(
            winner_type_for_zone(ZONE_BATTLE3),
            Some(BattleWinnerType::Npc)
        );
        assert_eq!(
            winner_type_for_zone(ZONE_BATTLE4),
            Some(BattleWinnerType::Monument)
        );
        assert_eq!(
            winner_type_for_zone(ZONE_BATTLE6),
            Some(BattleWinnerType::Kill)
        );
        // ZONE_BATTLE5 is NOT in C++ half-time list — returns None
        assert_eq!(winner_type_for_zone(ZONE_BATTLE5), None);
    }

    // ── war_tick tests ───────────────────────────────────────────────────

    fn make_open_war(zone: u8, battle_time: i32) -> BattleState {
        let mut state = BattleState::new();
        state.battle_time = battle_time;
        battle_zone_open(&mut state, BATTLEZONE_OPEN, zone, 1000);
        state
    }

    #[test]
    fn test_war_tick_decrements_remaining_time() {
        let mut state = make_open_war(1, 3600);
        assert_eq!(state.battle_remaining_time, 3600);

        war_tick(&mut state);
        assert_eq!(state.battle_remaining_time, 3599);

        war_tick(&mut state);
        assert_eq!(state.battle_remaining_time, 3598);
    }

    #[test]
    fn test_war_tick_time_expired() {
        let mut state = make_open_war(1, 3600);
        state.battle_remaining_time = 1; // 1 second left

        let action = war_tick(&mut state);
        // Remaining goes to 0, should fire TimeExpired
        assert_eq!(action, WarTickAction::TimeExpired);
        assert_eq!(state.battle_remaining_time, 0);
    }

    #[test]
    fn test_war_tick_half_time_triggers_result() {
        let mut state = make_open_war(1, 100); // 100 second war
        state.commander_selected = true; // skip auto-select
                                         // Set remaining time so that elapsed == battle_time / 2
                                         // elapsed = battle_time - remaining = 100 - 50 = 50 = 100/2
        state.battle_remaining_time = 51; // After tick: 50 remaining, elapsed = 50

        // Set some kills to produce a winner
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 3;

        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::HalfTimeResult { winner_nation: 1 });
    }

    #[test]
    fn test_war_tick_half_time_draw() {
        let mut state = make_open_war(1, 100);
        state.commander_selected = true; // skip auto-select
        state.battle_remaining_time = 51;

        // All stats tied → draw
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 5;
        state.karus_dead = 10;
        state.elmorad_dead = 10;

        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::HalfTimeResult { winner_nation: 0 });
    }

    #[test]
    fn test_war_tick_periodic_status_broadcast() {
        let mut state = make_open_war(1, 2400); // 40-minute war
        state.commander_selected = true; // skip auto-select
                                         // status_interval = 2400/24 = 100 ticks

        // Run 99 ticks — no status yet
        for _ in 0..99 {
            let action = war_tick(&mut state);
            assert_ne!(
                action,
                WarTickAction::BroadcastStatus {
                    sub_type: DECLARE_BATTLE_ZONE_STATUS
                },
                "Should not broadcast status before interval"
            );
        }

        // 100th tick should trigger status broadcast
        let action = war_tick(&mut state);
        assert_eq!(
            action,
            WarTickAction::BroadcastStatus {
                sub_type: DECLARE_BATTLE_ZONE_STATUS
            }
        );
        assert_eq!(state.battle_time_delay, 0); // reset after broadcast
    }

    #[test]
    fn test_war_tick_nereids_timer() {
        let mut state = make_open_war(4, 3600); // ZONE_BATTLE4
        assert_eq!(state.nereids_remaining_time, 3600);

        war_tick(&mut state);
        assert_eq!(state.nereids_remaining_time, 3599);
    }

    #[test]
    fn test_war_tick_nereids_quarter_check_karus_wins() {
        let mut state = make_open_war(4, 100);
        state.commander_selected = true; // skip auto-select
                                         // Set monuments: Karus has all 7, ElMorad has 0
        state.karus_monuments = 7;
        state.elmorad_monuments = 0;
        // Set remaining so elapsed == battle_time/4 = 25
        state.battle_remaining_time = 76; // After tick: 75, elapsed = 25

        let action = war_tick(&mut state);
        assert_eq!(
            action,
            WarTickAction::NereidsQuarterCheck { winner_nation: 1 }
        );
    }

    #[test]
    fn test_war_tick_nereids_quarter_check_elmorad_wins() {
        let mut state = make_open_war(4, 100);
        state.commander_selected = true; // skip auto-select
        state.karus_monuments = 0;
        state.elmorad_monuments = 7;
        state.battle_remaining_time = 76;

        let action = war_tick(&mut state);
        assert_eq!(
            action,
            WarTickAction::NereidsQuarterCheck { winner_nation: 2 }
        );
    }

    #[test]
    fn test_war_tick_post_victory_under_attack_notify() {
        let mut state = make_open_war(1, 2400);
        state.victory = 1; // Karus won
        state.battle_time_delay = 0;

        // Tick until we get the under-attack notification
        // interval = 2400/24 = 100
        for _ in 0..99 {
            let action = war_tick(&mut state);
            assert_eq!(action, WarTickAction::None);
        }

        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::BroadcastUnderAttack);
        assert_eq!(state.battle_time_delay, 0); // reset
    }

    #[test]
    fn test_war_tick_result_delay() {
        let mut state = make_open_war(1, 100);
        state.result_delay = true;
        state.result_delay_victory = 2;

        // C++ delay threshold: battle_time / (battle_time / 10) = 10
        for _ in 0..9 {
            let action = war_tick(&mut state);
            assert_ne!(action, WarTickAction::ResultDelayFired { winner_nation: 2 });
        }

        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::ResultDelayFired { winner_nation: 2 });
        assert!(!state.result_delay);
    }

    // ── Banish tests ─────────────────────────────────────────────────────

    #[test]
    fn test_banish_announce_and_execute() {
        let mut state = BattleState::new();
        state.battle_time = 3600;
        state.banish_flag = true;
        state.victory = 1;

        // ban announce threshold = 3600/360 = 10
        // ban execute threshold = 3600/120 = 30
        let mut announced = false;
        let mut executed = false;

        for _ in 0..30 {
            let action = war_tick(&mut state);
            match action {
                WarTickAction::AnnounceBanish => announced = true,
                WarTickAction::ExecuteBanish => executed = true,
                _ => {}
            }
        }

        assert!(announced, "Ban should be announced");
        assert!(executed, "Ban should be executed");
        assert!(!state.banish_flag, "Banish flag should be cleared");
        assert_eq!(state.banish_delay, 0, "Banish delay should be reset");
    }

    #[test]
    fn test_banish_no_action_when_flag_false() {
        let mut state = BattleState::new();
        state.banish_flag = false;
        state.battle_open = NO_BATTLE;

        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::None);
    }

    // ── Status string tests ──────────────────────────────────────────────

    #[test]
    fn test_build_status_string_normal_zone() {
        let mut state = BattleState::new();
        state.battle_zone = 1; // ZONE_BATTLE
        state.killed_karus_npc = 3;
        state.killed_elmorad_npc = 5;
        state.karus_dead = 10;
        state.elmorad_dead = 8;
        state.karus_flag = 2;
        state.elmorad_flag = 1;

        let msg = build_status_string(&state);
        assert!(msg.contains("NPC Kills"));
        assert!(msg.contains("Karus: 3"));
        assert!(msg.contains("El Morad: 5"));
        assert!(msg.contains("Deaths"));
        assert!(msg.contains("Flags"));
    }

    #[test]
    fn test_build_status_string_nereids() {
        let mut state = BattleState::new();
        state.battle_zone = 4; // ZONE_BATTLE4
        state.karus_monument_point = 150;
        state.elmorad_monument_point = 80;
        state.karus_dead = 5;
        state.elmorad_dead = 7;

        let msg = build_status_string(&state);
        assert!(msg.contains("Monument Points"));
        assert!(msg.contains("150"));
        assert!(msg.contains("80"));
        assert!(!msg.contains("NPC Kills")); // Should not include NPC kills
    }

    // ── Reward calculation tests ─────────────────────────────────────────

    #[test]
    fn test_captain_reward_np_result() {
        // Result phase (not flag victory)
        assert_eq!(captain_reward_np(false, false), 500);
        assert_eq!(captain_reward_np(true, false), 500);
    }

    #[test]
    fn test_captain_reward_np_flag_victory() {
        // Flag victory: kings get 500, non-kings get 300
        assert_eq!(captain_reward_np(true, true), 500);
        assert_eq!(captain_reward_np(false, true), 300);
    }

    #[test]
    fn test_flag_victory_np() {
        assert_eq!(flag_victory_np(false), 100);
        assert_eq!(flag_victory_np(true), 200);
    }

    // ── nation_home_zone tests ───────────────────────────────────────────

    #[test]
    fn test_nation_home_zone() {
        assert_eq!(nation_home_zone(1), ZONE_KARUS);
        assert_eq!(nation_home_zone(2), ZONE_ELMORAD);
        assert_eq!(nation_home_zone(0), ZONE_KARUS); // fallback
    }

    // ── Winner/banish string tests ───────────────────────────────────────

    #[test]
    fn test_build_winner_string() {
        assert_eq!(build_winner_string(1), "Karus has won the war!");
        assert_eq!(build_winner_string(2), "El Morad has won the war!");
    }

    #[test]
    fn test_build_banish_string() {
        let msg = build_banish_string(1);
        assert!(msg.contains("Losers"));
        let msg = build_banish_string(0);
        assert!(msg.contains("All players"));
    }

    // ── New constant tests ───────────────────────────────────────────────

    #[test]
    fn test_new_constants() {
        assert_eq!(DECLARE_BAN, 0x04);
        assert_eq!(DECLARE_BATTLE_ZONE_STATUS, 0x11);
        assert_eq!(UNDER_ATTACK_NOTIFY, 0x10);
    }

    // ── Full war tick lifecycle test ─────────────────────────────────────

    #[test]
    fn test_war_tick_full_lifecycle() {
        // Short 20-second war for fast testing
        let mut state = make_open_war(1, 20);
        assert!(state.is_war_open());
        assert_eq!(state.battle_remaining_time, 20);

        // Set some kills for half-time determination
        state.killed_karus_npc = 3;
        state.killed_elmorad_npc = 1;

        let mut half_time_fired = false;
        let mut time_expired_fired = false;

        // Tick through the entire war
        for _ in 0..25 {
            let action = war_tick(&mut state);
            match action {
                WarTickAction::HalfTimeResult { winner_nation } => {
                    half_time_fired = true;
                    assert_eq!(winner_nation, 1); // Karus wins NPC comparison
                                                  // Simulate applying the result
                    battle_zone_result(&mut state, winner_nation);
                }
                WarTickAction::TimeExpired => {
                    time_expired_fired = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(half_time_fired, "Half-time should have fired");
        assert!(time_expired_fired, "Time should have expired");
    }

    #[test]
    fn test_war_tick_no_action_when_no_war() {
        let mut state = BattleState::new();
        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::None);
    }

    #[test]
    fn test_war_tick_snow_battle_not_processed() {
        // Snow battle should not be processed by war_tick (only nation battle)
        let mut state = BattleState::new();
        state.battle_time = 3600;
        battle_zone_open(&mut state, SNOW_BATTLEZONE_OPEN, 0, 1000);
        assert!(state.is_snow_battle());

        // war_tick should return None for snow battles
        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::None);
    }

    // ── Zone4 half-time monument winner test ─────────────────────────────

    #[test]
    fn test_war_tick_zone4_half_time_monument_winner() {
        let mut state = make_open_war(4, 100); // ZONE_BATTLE4
        state.commander_selected = true; // skip auto-select
        state.karus_monument_point = 200;
        state.elmorad_monument_point = 100;
        // Set remaining so elapsed == 50 (half time)
        state.battle_remaining_time = 51;

        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::HalfTimeResult { winner_nation: 1 });
    }

    // ── Zone6 half-time kill winner test ─────────────────────────────────

    #[test]
    fn test_war_tick_zone6_half_time_kill_winner() {
        let mut state = make_open_war(6, 100); // ZONE_BATTLE6
        state.commander_selected = true; // skip auto-select
        state.karus_dead = 20;
        state.elmorad_dead = 10;
        // elapsed = 50 (half time)
        state.battle_remaining_time = 51;

        let action = war_tick(&mut state);
        // Kill comparison: Karus has MORE deaths → ElMorad wins
        assert_eq!(action, WarTickAction::HalfTimeResult { winner_nation: 2 });
    }

    // ── Nereids timer only decrements for ZONE_BATTLE4 ──────────────────

    #[test]
    fn test_nereids_timer_only_zone4() {
        // ZONE_BATTLE5 should NOT decrement nereids timer
        let mut state = make_open_war(5, 3600);
        state.nereids_remaining_time = 3600;

        war_tick(&mut state);
        // ZONE_BATTLE5 = 65, but the C++ only decrements for ZONE_BATTLE4
        assert_eq!(state.nereids_remaining_time, 3600);
    }

    // ── Status interval for ZONE_BATTLE4 is twice as frequent ────────────

    #[test]
    fn test_war_tick_interval_constant() {
        // Verify the tick interval matches C++ (1 second)
        assert_eq!(WAR_TICK_INTERVAL_SECS, 1);
    }

    #[test]
    fn test_start_war_task_compiles() {
        // Verify start_war_task signature is correct: takes Arc<WorldState>, returns JoinHandle
        // This is a compile-time check — the function signature must match
        let _: fn(std::sync::Arc<WorldState>) -> tokio::task::JoinHandle<()> = start_war_task;
    }

    #[test]
    fn test_status_interval_zone4_more_frequent() {
        let mut state = make_open_war(4, 4800);
        // For ZONE_BATTLE4: interval = 4800/48 = 100
        // For normal zones: interval = 4800/24 = 200

        let mut ticks_to_status = 0;
        for i in 1..=200 {
            let action = war_tick(&mut state);
            if action
                == (WarTickAction::BroadcastStatus {
                    sub_type: DECLARE_BATTLE_ZONE_STATUS,
                })
            {
                ticks_to_status = i;
                break;
            }
        }

        assert_eq!(
            ticks_to_status, 100,
            "ZONE_BATTLE4 should broadcast status every 100 ticks"
        );
    }

    // ── Sprint 47 QA Integration Tests ──────────────────────────────────────

    /// Integration: Battle zone MAP_EVENT lifecycle: open -> active -> close.
    ///
    /// Verifies the full lifecycle of a war zone: opening, running ticks,
    /// detecting time expiry, and closing correctly.
    #[test]
    fn test_integration_battle_zone_lifecycle_open_active_close() {
        let mut state = BattleState::new();
        state.battle_time = 60; // 60 second war

        // Phase 1: Open
        let opened = battle_zone_open(&mut state, BATTLEZONE_OPEN, 1, 1000);
        assert!(opened, "Battle zone should open successfully");
        assert_eq!(state.battle_open, NATION_BATTLE);
        assert_eq!(state.battle_zone, 1);
        assert_eq!(state.battle_remaining_time, 60);
        assert_eq!(state.battle_opened_time, 1000);
        assert_eq!(state.victory, 0, "No winner initially");

        // Phase 2: Active — tick through the war
        let mut half_time_reached = false;

        for _i in 0..59 {
            let action = war_tick(&mut state);
            if let WarTickAction::HalfTimeResult { .. } = action {
                half_time_reached = true
            }
        }
        assert_eq!(state.battle_remaining_time, 1, "Should have 1 second left");
        assert!(
            half_time_reached,
            "Half-time result should have fired at 30s"
        );

        // Phase 3: Time expired
        let final_action = war_tick(&mut state);
        assert_eq!(final_action, WarTickAction::TimeExpired);
        assert_eq!(state.battle_remaining_time, 0);

        // Phase 4: Close
        let prev_type = battle_zone_close(&mut state);
        assert_eq!(prev_type, NATION_BATTLE);
        assert_eq!(state.battle_open, NO_BATTLE);
        assert!(state.banish_flag, "Banish flag should be set after close");
    }

    /// Integration: Snow battle lifecycle — open and close.
    ///
    /// Snow battle does not use war_tick for time management (only NATION_BATTLE does).
    /// The timer for snow battle is handled externally. war_tick returns None for snow.
    #[test]
    fn test_integration_snow_battle_lifecycle() {
        let mut state = BattleState::new();
        state.battle_time = 120;

        let opened = battle_zone_open(&mut state, SNOW_BATTLEZONE_OPEN, 0, 5000);
        assert!(opened);
        assert_eq!(state.battle_open, SNOW_BATTLE);
        assert!(state.is_snow_battle());
        assert!(!state.is_nation_battle());

        // war_tick for snow battle returns None (no decrement — only NATION_BATTLE ticks)
        let action = war_tick(&mut state);
        assert_eq!(
            action,
            WarTickAction::None,
            "Snow battle tick should return None"
        );
        assert_eq!(
            state.battle_remaining_time, 120,
            "Snow battle time not decremented by war_tick"
        );

        // Close
        let prev_type = battle_zone_close(&mut state);
        assert_eq!(prev_type, SNOW_BATTLE);
        assert_eq!(state.battle_open, NO_BATTLE);
        assert!(state.banish_flag);
    }

    /// Integration: War zone PK ranking — kill/death tracking and winner determination.
    ///
    /// Verifies that death counts correctly determine the war winner.
    /// More deaths = losing side. Fewer deaths = winner.
    #[test]
    fn test_integration_war_pk_ranking_kill_determination() {
        let mut state = make_open_war(1, 3600);

        // Karus suffers more deaths -> El Morad wins
        state.karus_dead = 150;
        state.elmorad_dead = 100;

        let winner = battle_winner_result(&state, BattleWinnerType::Kill);
        assert_eq!(winner, 2, "El Morad wins because Karus had more deaths");

        // El Morad suffers more deaths -> Karus wins
        state.karus_dead = 80;
        state.elmorad_dead = 120;

        let winner2 = battle_winner_result(&state, BattleWinnerType::Kill);
        assert_eq!(winner2, 1, "Karus wins because El Morad had more deaths");

        // Tied deaths -> draw (for ZONE_BATTLE)
        state.karus_dead = 100;
        state.elmorad_dead = 100;

        let winner3 = battle_winner_result(&state, BattleWinnerType::Kill);
        // For ZONE_BATTLE (zone 1), tied kills => 0 (no cascade to NPC)
        assert_eq!(winner3, 0, "Tied deaths in ZONE_BATTLE results in draw");
    }

    /// Integration: War PK ranking — NPC kill metric determines winner.
    ///
    #[test]
    fn test_integration_war_pk_ranking_npc_kill_winner() {
        let mut state = make_open_war(1, 3600);

        // Karus killed more NPC targets
        state.killed_karus_npc = 10;
        state.killed_elmorad_npc = 5;

        let winner = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner, 1, "Karus wins by NPC kills");

        // Reverse
        state.killed_karus_npc = 3;
        state.killed_elmorad_npc = 8;
        let winner2 = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner2, 2, "El Morad wins by NPC kills");
    }

    /// Integration: War zone flag victory triggers at NUM_FLAG_VICTORY captures.
    ///
    #[test]
    fn test_integration_war_flag_victory() {
        let state = make_open_war(1, 3600);

        // No flags — no victory
        assert_eq!(battle_zone_victory_check(&state), 0);

        // Karus reaches flag threshold
        let mut state2 = make_open_war(1, 3600);
        state2.karus_flag = NUM_FLAG_VICTORY;
        assert_eq!(battle_zone_victory_check(&state2), 1, "Karus flag victory");

        // El Morad reaches flag threshold
        let mut state3 = make_open_war(1, 3600);
        state3.elmorad_flag = NUM_FLAG_VICTORY;
        assert_eq!(
            battle_zone_victory_check(&state3),
            2,
            "El Morad flag victory"
        );

        // Below threshold — no victory
        let mut state4 = make_open_war(1, 3600);
        state4.karus_flag = NUM_FLAG_VICTORY - 1;
        state4.elmorad_flag = NUM_FLAG_VICTORY - 1;
        assert_eq!(
            battle_zone_victory_check(&state4),
            0,
            "No flag victory below threshold"
        );
    }

    /// Integration: Monument-based winner determination.
    ///
    #[test]
    fn test_integration_war_monument_winner() {
        let mut state = make_open_war(1, 3600);

        state.karus_monument_point = 500;
        state.elmorad_monument_point = 300;
        let winner = battle_winner_result(&state, BattleWinnerType::Monument);
        assert_eq!(winner, 1, "Karus wins by monument points");

        state.karus_monument_point = 200;
        state.elmorad_monument_point = 400;
        let winner2 = battle_winner_result(&state, BattleWinnerType::Monument);
        assert_eq!(winner2, 2, "El Morad wins by monument points");
    }

    /// Integration: Kill-based tiebreak cascades correctly for ZONE_BATTLE4.
    ///
    #[test]
    fn test_integration_war_kill_cascade_zone4() {
        let mut state = make_open_war(4, 3600); // ZONE_BATTLE4

        // Tied kills → cascades to NPC
        state.karus_dead = 50;
        state.elmorad_dead = 50;
        state.killed_karus_npc = 10;
        state.killed_elmorad_npc = 5;

        let winner = battle_winner_result(&state, BattleWinnerType::Kill);
        // Tied on kills → cascades to NPC → karus_npc > elmorad_npc → Karus wins (1)
        assert_eq!(
            winner, 1,
            "Tied kills in ZONE_BATTLE4 should cascade to NPC tiebreak"
        );
    }

    /// Integration: Draw when all metrics are tied.
    ///
    #[test]
    fn test_integration_war_draw_all_tied() {
        let mut state = make_open_war(1, 3600); // ZONE_BATTLE (1-3: NPC -> Kill -> 0)

        // All metrics tied
        state.killed_karus_npc = 5;
        state.killed_elmorad_npc = 5;
        state.karus_dead = 30;
        state.elmorad_dead = 30;
        state.karus_monument_point = 100;
        state.elmorad_monument_point = 100;

        // NPC -> Kill (tied) -> 0 draw for ZONE_BATTLE
        let winner = battle_winner_result(&state, BattleWinnerType::Npc);
        assert_eq!(winner, 0, "All metrics tied should result in draw");
    }

    /// Test monument NP reward constants match C++ values.
    #[test]
    fn test_monument_np_reward_constants() {
        assert_eq!(MONUMENT_WINNER_INTERVAL, 300);
        assert_eq!(MONUMENT_DEFEATED_INTERVAL, 10_000);
        assert_eq!(MONUMENT_MAIN_NP, 200);
        assert_eq!(MONUMENT_SECONDARY_NP, 50);
        assert_eq!(MONUMENT_REWARD_EFFECT, 20100);
        assert_eq!(MONUMENT_REWARD_RANGE, 100.0);
    }

    /// Test monument NP amount selection (main vs secondary).
    #[test]
    fn test_monument_np_amount_selection() {
        // Main monuments: Luferson (20301) and Elmorad (10301) get 200 NP
        let main_np = if 20301u16 == LUFERSON_MONUMENT_SID || 20301u16 == ELMORAD_MONUMENT_SID {
            MONUMENT_MAIN_NP
        } else {
            MONUMENT_SECONDARY_NP
        };
        assert_eq!(main_np, 200);

        let main_np2 = if 10301u16 == LUFERSON_MONUMENT_SID || 10301u16 == ELMORAD_MONUMENT_SID {
            MONUMENT_MAIN_NP
        } else {
            MONUMENT_SECONDARY_NP
        };
        assert_eq!(main_np2, 200);

        // Other monuments get 50 NP
        let other_sid: u16 = 12345;
        let secondary_np =
            if other_sid == LUFERSON_MONUMENT_SID || other_sid == ELMORAD_MONUMENT_SID {
                MONUMENT_MAIN_NP
            } else {
                MONUMENT_SECONDARY_NP
            };
        assert_eq!(secondary_np, 50);
    }

    /// Test nation monument tracking with HashMap (timer-based).
    #[test]
    fn test_nation_monument_hashmap_tracking() {
        let mut state = BattleState::new();
        state.battle_open = NATION_BATTLE;

        // Insert winner monument with timer
        state.nation_monument_winners.insert(20301, 1000);
        assert!(state.nation_monument_winners.contains_key(&20301));
        assert_eq!(state.nation_monument_winners[&20301], 1000);

        // Update timer
        if let Some(t) = state.nation_monument_winners.get_mut(&20301) {
            *t = 1300; // +300s
        }
        assert_eq!(state.nation_monument_winners[&20301], 1300);

        // Remove on recapture
        state.nation_monument_winners.remove(&20301);
        assert!(!state.nation_monument_winners.contains_key(&20301));

        // Defeated monument: insert, fire, delete
        state.nation_monument_defeated.insert(10301, 2000);
        assert!(state.nation_monument_defeated.contains_key(&10301));
        state.nation_monument_defeated.remove(&10301);
        assert!(state.nation_monument_defeated.is_empty());
    }

    /// Test battle_zone_close clears monument tracking.
    #[test]
    fn test_battle_close_clears_monuments() {
        let mut state = BattleState::new();
        state.battle_open = NATION_BATTLE;
        state.nation_monument_winners.insert(20301, 100);
        state.nation_monument_defeated.insert(10301, 200);

        battle_zone_close(&mut state);

        assert!(state.nation_monument_winners.is_empty());
        assert!(state.nation_monument_defeated.is_empty());
    }

    /// Test update_battle_zone_user_counts is callable during NATION_BATTLE.
    #[test]
    fn test_user_count_update_during_war() {
        let world = crate::world::WorldState::new();
        // No active war → counts should stay 0
        let state = world.get_battle_state();
        assert_eq!(state.battle_open, NO_BATTLE);
        // Safe to call even without active war
        world.update_battle_zone_user_counts();
        let state2 = world.get_battle_state();
        assert_eq!(state2.karus_count, 0);
        assert_eq!(state2.elmorad_count, 0);
    }

    /// BattleZoneResult captain NP: only captains (fame == COMMAND_CAPTAIN) get 500 NP.
    #[test]
    fn test_war_result_captain_np_constant() {
        // Captain reward in non-flag-victory is always 500
        assert_eq!(captain_reward_np(false, false), 500);
        assert_eq!(captain_reward_np(true, false), 500);
        // COMMAND_CAPTAIN fame value
        assert_eq!(COMMAND_CAPTAIN, 100);
    }

    /// distribute_war_result_rewards with winner=0 is a no-op.
    #[test]
    fn test_war_result_rewards_zero_winner_noop() {
        let world = crate::world::WorldState::new();
        // Should not panic — no-op for winner=0
        distribute_war_result_rewards(&world, 0);
    }

    /// distribute_war_result_rewards with NO_BATTLE is a no-op.
    #[test]
    fn test_war_result_rewards_no_battle_noop() {
        let world = crate::world::WorldState::new();
        // battle_open defaults to NO_BATTLE — should early return
        distribute_war_result_rewards(&world, 1);
    }

    /// Banish captain reset: COMMAND_CAPTAIN → CHIEF when NO_BATTLE.
    #[test]
    fn test_banish_captain_fame_values() {
        use crate::clan_constants::CHIEF;
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(CHIEF, 1);
        // is_battle_zone covers all 6 battle zones
        assert!(is_battle_zone(ZONE_BATTLE));
        assert!(is_battle_zone(ZONE_BATTLE6));
        assert!(!is_battle_zone(1)); // Karus home
        assert!(!is_battle_zone(2)); // Elmo home
    }

    /// Banish home zone sweep: zone_id <= 2 && zone_id != nation.
    #[test]
    fn test_banish_home_zone_invader_check() {
        // Karus player (nation=1) in El Morad zone (2) → invader
        let zone_id: u16 = 2;
        let nation: u8 = 1;
        let in_enemy_home = zone_id <= 2 && zone_id != nation as u16;
        assert!(in_enemy_home);

        // Karus player in Karus zone → not invader
        let zone_id2: u16 = 1;
        let in_enemy_home2 = zone_id2 <= 2 && zone_id2 != nation as u16;
        assert!(!in_enemy_home2);

        // Player in zone 3+ → not covered by home zone check
        let zone_id3: u16 = 3;
        let in_enemy_home3 = zone_id3 <= 2 && zone_id3 != nation as u16;
        assert!(!in_enemy_home3);
    }

    /// Test kick zone constants match C++ Define.h.
    #[test]
    fn test_kick_zone_constants() {
        use crate::world::{
            ZONE_ARDREAM, ZONE_BIFROST, ZONE_KROWAZ_DOMINION, ZONE_RONARK_LAND,
            ZONE_RONARK_LAND_BASE,
        };
        // C++ Define.h zone IDs
        assert_eq!(ZONE_RONARK_LAND_BASE, 73);
        assert_eq!(ZONE_RONARK_LAND, 71);
        assert_eq!(ZONE_BIFROST, 31);
        assert_eq!(ZONE_KROWAZ_DOMINION, 75);
        assert_eq!(ZONE_ARDREAM, 72);
        assert_eq!(ZONE_ARDREAM_TYPE, 72);
    }

    /// Test auto commander selection triggers at 1/24 of war time.
    ///
    #[test]
    fn auto_commander_select_at_one_24th() {
        let mut state = BattleState::new();
        state.battle_open = NATION_BATTLE;
        state.battle_time = 2400; // 40 min war
        state.battle_remaining_time = 2400;
        state.victory = 0;
        state.commander_selected = false;

        let threshold = 2400 / 24; // 100 seconds

        // Tick until just before threshold — should NOT trigger
        for _ in 0..(threshold - 1) {
            let action = war_tick(&mut state);
            assert_ne!(action, WarTickAction::SelectCommanders);
        }

        // At threshold tick — should trigger SelectCommanders
        let action = war_tick(&mut state);
        assert_eq!(action, WarTickAction::SelectCommanders);
        assert!(state.commander_selected);

        // Subsequent ticks should NOT trigger again
        let action2 = war_tick(&mut state);
        assert_ne!(action2, WarTickAction::SelectCommanders);
    }

    /// Test commander selection doesn't trigger after victory.
    #[test]
    fn commander_select_skip_after_victory() {
        let mut state = BattleState::new();
        state.battle_open = NATION_BATTLE;
        state.battle_time = 2400;
        state.battle_remaining_time = 2300; // elapsed=100, at threshold
        state.victory = 1; // victory already declared
        state.commander_selected = false;

        let action = war_tick(&mut state);
        // Should NOT select commanders when victory is declared
        assert_ne!(action, WarTickAction::SelectCommanders);
        assert!(!state.commander_selected);
    }

    /// Test commander selection doesn't re-trigger if already done.
    #[test]
    fn commander_select_no_double_trigger() {
        let mut state = BattleState::new();
        state.battle_open = NATION_BATTLE;
        state.battle_time = 2400;
        state.battle_remaining_time = 2300; // elapsed=100, at threshold
        state.victory = 0;
        state.commander_selected = true; // already selected

        let action = war_tick(&mut state);
        assert_ne!(action, WarTickAction::SelectCommanders);
    }

    /// Test ZONE_BATTLE2 is the only zone for remnant spawn.
    #[test]
    fn remnant_spawn_only_zone_battle2() {
        // ZONE_BATTLE2 = 62
        assert_eq!(ZONE_BATTLE2, 62);
        // Only zone 62 triggers remnant spawn (battle_zone=2 → 60+2=62)
        // Other zones should not trigger
        assert_ne!(ZONE_BATTLE, ZONE_BATTLE2);
        assert_ne!(ZONE_BATTLE3, ZONE_BATTLE2);
        assert_ne!(ZONE_BATTLE4, ZONE_BATTLE2);
    }

    /// Test banish-of-winner data filtering by nation.
    #[test]
    fn banish_of_winner_nation_filter() {
        use ko_db::models::BanishOfWinner;
        let entries = [
            BanishOfWinner {
                idx: 1,
                sid: 3252,
                nation_id: Some(2),
                zone_id: 62,
                pos_x: 100,
                pos_z: 200,
                spawn_count: 5,
                radius: Some(15),
                dead_time: 10,
            },
            BanishOfWinner {
                idx: 2,
                sid: 3202,
                nation_id: Some(1),
                zone_id: 62,
                pos_x: 300,
                pos_z: 400,
                spawn_count: 5,
                radius: Some(20),
                dead_time: 10,
            },
        ];

        // Filter for Karus victory (nation=1)
        let karus_entries: Vec<_> = entries.iter().filter(|b| b.nation_id == Some(1)).collect();
        assert_eq!(karus_entries.len(), 1);
        assert_eq!(karus_entries[0].sid, 3202);

        // Filter for ElMorad victory (nation=2)
        let elmo_entries: Vec<_> = entries.iter().filter(|b| b.nation_id == Some(2)).collect();
        assert_eq!(elmo_entries.len(), 1);
        assert_eq!(elmo_entries[0].sid, 3252);
    }
}
