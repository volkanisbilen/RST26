//! Chaos Dungeon event system — free-for-all PvP room event.
//! Chaos Dungeon is an FFA PvP event where all players compete against each
//! other regardless of nation. Each room holds up to 18 players.
//! ## Differences from BDW/Juraid
//! - **No nation split**: players from both nations are mixed in the same room.
//! - **No priest balancing**: players are assigned FIFO by join order.
//! - **No automatic party creation**: Chaos has no `TempleEventCreateParties()` call.
//! - **Kill/death ranking**: individual per-player ranking, not nation-based.
//! ## Scoring
//! - Each player tracks kills and deaths independently.
//! - Ranking is calculated as: `kills * 5 - deaths` (C++ EXP formula base).
//! - Chaos entry requires a "Chaos Map" item which is consumed on teleport.
//! ## Rewards
//! Loaded from `event_rewards` table (local_id=10 for Chaos).
//! Rewards are rank-based, not nation-based.

use std::collections::HashMap;

use crate::systems::event_room::{
    EventRoomManager, EventUser, RoomState, TempleEventType, MAX_CHAOS_ROOM_USERS,
};

pub use crate::world::types::ZONE_CHAOS;

/// Default maximum rooms for Chaos Dungeon.
pub const DEFAULT_CHAOS_ROOMS: u8 = 10;

/// Fixed max HP of every fighter inside the Chaos Dungeon (C++ `m_MaxHp = 10000 / 10`).
pub const CHAOS_MAX_HP: i16 = 1000;

/// Max HP forced on a player standing in `zone_id` (Some only for the Chaos Dungeon).
pub fn max_hp_override_for_zone(zone_id: u16) -> Option<i16> {
    (zone_id == ZONE_CHAOS).then_some(CHAOS_MAX_HP)
}

/// Maximum users per Chaos room.
pub const MAX_USERS_PER_ROOM: usize = MAX_CHAOS_ROOM_USERS;

// ── Chaos Player Stats ─────────────────────────────────────────────────────

/// Per-player kill/death tracking inside a Chaos room.
#[derive(Debug, Clone, Default)]
pub struct ChaosPlayerStats {
    /// Number of kills in this event session.
    pub kills: u32,
    /// Number of deaths in this event session.
    pub deaths: u32,
}

impl ChaosPlayerStats {
    /// Calculate the ranking score (higher is better).
    ///
    pub fn ranking_score(&self) -> i64 {
        (self.kills as i64) * 5 - (self.deaths as i64)
    }
}

// ── Chaos Manager ──────────────────────────────────────────────────────────

/// Chaos Dungeon event manager — coordinates Chaos-specific logic.
/// Stored alongside `EventRoomManager` in the event system background task.
/// Provides Chaos-specific operations that build on the generic room system.
#[derive(Debug)]
pub struct ChaosManager {
    /// Per-player stats keyed by character name, per room.
    /// Outer key: room_id, Inner key: character name.
    pub room_player_stats: HashMap<u8, HashMap<String, ChaosPlayerStats>>,
    /// Number of rooms to create for this event.
    pub max_rooms: u8,
}

impl ChaosManager {
    /// Create a new Chaos manager.
    pub fn new(max_rooms: u8) -> Self {
        Self {
            room_player_stats: HashMap::new(),
            max_rooms,
        }
    }

    /// Initialize Chaos rooms in the event room manager and create per-room stats.
    pub fn init_rooms(&mut self, erm: &EventRoomManager) {
        erm.create_rooms(TempleEventType::ChaosDungeon, self.max_rooms);
        self.room_player_stats.clear();
        for room_id in 1..=self.max_rooms {
            self.room_player_stats.insert(room_id, HashMap::new());
        }
    }

    /// Destroy all Chaos rooms and clear stats.
    pub fn destroy_rooms(&mut self, erm: &EventRoomManager) {
        erm.destroy_rooms(TempleEventType::ChaosDungeon);
        self.room_player_stats.clear();
    }

    /// Record a kill for a player in a room.
    pub fn record_kill(&mut self, room_id: u8, killer_name: &str) {
        if let Some(room_stats) = self.room_player_stats.get_mut(&room_id) {
            room_stats.entry(killer_name.to_string()).or_default().kills += 1;
        }
    }

    /// Record a death for a player in a room.
    pub fn record_death(&mut self, room_id: u8, victim_name: &str) {
        if let Some(room_stats) = self.room_player_stats.get_mut(&room_id) {
            room_stats
                .entry(victim_name.to_string())
                .or_default()
                .deaths += 1;
        }
    }

    /// Get a player's stats in a specific room.
    pub fn get_player_stats(&self, room_id: u8, name: &str) -> Option<&ChaosPlayerStats> {
        self.room_player_stats
            .get(&room_id)
            .and_then(|m| m.get(name))
    }

    /// Get all player stats for a room, sorted by ranking score (descending).
    pub fn get_room_rankings(&self, room_id: u8) -> Vec<(String, ChaosPlayerStats)> {
        let Some(room_stats) = self.room_player_stats.get(&room_id) else {
            return Vec::new();
        };
        let mut rankings: Vec<_> = room_stats
            .iter()
            .map(|(name, stats)| (name.clone(), stats.clone()))
            .collect();
        rankings.sort_by(|a, b| b.1.ranking_score().cmp(&a.1.ranking_score()));
        rankings
    }

    /// Reset all room stats.
    pub fn reset_all(&mut self) {
        for stats in self.room_player_stats.values_mut() {
            stats.clear();
        }
    }
}

impl Default for ChaosManager {
    fn default() -> Self {
        Self::new(DEFAULT_CHAOS_ROOMS)
    }
}

// ── Room Assignment ────────────────────────────────────────────────────────

/// Assign signed-up users to Chaos Dungeon rooms.
/// Unlike BDW/Juraid, Chaos does NOT separate by nation or balance priests.
/// Players are assigned FIFO (by join order) up to 18 per room.
/// Returns the number of users assigned to rooms.
pub fn assign_users_to_rooms(erm: &EventRoomManager, chaos: &mut ChaosManager) -> usize {
    let users = erm.signed_up_users.read().clone();
    if users.is_empty() {
        return 0;
    }

    // Sort by join order for deterministic FIFO assignment
    let mut sorted_users = users;
    sorted_users.sort_by_key(|u| u.join_order);

    let mut user_iter = sorted_users.into_iter();
    let mut total_assigned = 0;

    let mut room_ids = erm.list_rooms(TempleEventType::ChaosDungeon);
    room_ids.sort();

    for room_id in &room_ids {
        let mut room = match erm.get_room_mut(TempleEventType::ChaosDungeon, *room_id) {
            Some(r) => r,
            None => continue,
        };

        let slots = MAX_USERS_PER_ROOM - room.mixed_users.len();
        for _ in 0..slots {
            let Some(signed_up) = user_iter.next() else {
                break;
            };

            let user = EventUser {
                user_name: signed_up.user_name.clone(),
                session_id: signed_up.session_id,
                nation: signed_up.nation,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            };

            if room.add_user(user) {
                // Initialize player stats for this room
                if let Some(room_stats) = chaos.room_player_stats.get_mut(room_id) {
                    room_stats.insert(signed_up.user_name.clone(), ChaosPlayerStats::default());
                }
                total_assigned += 1;
            }
        }

        if !room.mixed_users.is_empty() {
            room.state = RoomState::Running;
        }
    }

    total_assigned
}

/// Determine "winners" for all Chaos rooms.
/// Chaos is FFA — there is no nation-based winner. This returns `(room_id, 0)`
/// for each active room to maintain the same interface as BDW/Juraid.
/// The actual per-player ranking is handled by `ChaosManager::get_room_rankings()`.
/// Returns a list of `(room_id, winner_nation)` — winner_nation is always 0 for Chaos.
pub fn determine_all_winners(erm: &EventRoomManager) -> Vec<(u8, u8)> {
    let room_ids = erm.list_rooms(TempleEventType::ChaosDungeon);
    let mut results = Vec::with_capacity(room_ids.len());

    for room_id in room_ids {
        let mut room = match erm.get_room_mut(TempleEventType::ChaosDungeon, room_id) {
            Some(r) => r,
            None => continue,
        };

        if room.mixed_users.is_empty() {
            continue;
        }

        // Chaos has no nation-based winner — always 0
        // NOTE: `finished` must stay false here: `distribute_chaos_finish_exp` (which runs
        // right after this) skips rooms that are already finished, which used to swallow
        // every Chaos reward. The distributor sets the flag itself (C++ m_bFinished).
        room.winner_nation = 0;
        results.push((room_id, 0u8));
    }

    results
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::event_room::EventRoomManager;

    fn make_erm_with_chaos_rooms(max_rooms: u8) -> (EventRoomManager, ChaosManager) {
        let erm = EventRoomManager::new();
        let mut chaos = ChaosManager::new(max_rooms);
        chaos.init_rooms(&erm);
        (erm, chaos)
    }

    #[test]
    fn test_assign_users_basic() {
        let (erm, mut chaos) = make_erm_with_chaos_rooms(2);

        // Add 4 users (mixed nations)
        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("e1".to_string(), 2, 2);
        erm.add_signed_up_user("k2".to_string(), 3, 1);
        erm.add_signed_up_user("e2".to_string(), 4, 2);

        let assigned = assign_users_to_rooms(&erm, &mut chaos);
        assert_eq!(assigned, 4);

        // All should be in room 1 (only 4 users, room fits 18)
        let room1 = erm
            .get_room(TempleEventType::ChaosDungeon, 1)
            .expect("room 1");
        assert_eq!(room1.mixed_users.len(), 4);
        assert_eq!(room1.state, RoomState::Running);
    }

    #[test]
    fn test_assign_users_overflow_to_room2() {
        let (erm, mut chaos) = make_erm_with_chaos_rooms(3);

        // Add 20 users — room 1 fits 18, room 2 gets 2
        for i in 0..20 {
            erm.add_signed_up_user(format!("p{}", i), i as u16, if i % 2 == 0 { 1 } else { 2 });
        }

        let assigned = assign_users_to_rooms(&erm, &mut chaos);
        assert_eq!(assigned, 20);

        let room1 = erm
            .get_room(TempleEventType::ChaosDungeon, 1)
            .expect("room 1");
        assert_eq!(room1.mixed_users.len(), 18);

        let room2 = erm
            .get_room(TempleEventType::ChaosDungeon, 2)
            .expect("room 2");
        assert_eq!(room2.mixed_users.len(), 2);
    }

    #[test]
    fn test_assign_users_empty() {
        let (erm, mut chaos) = make_erm_with_chaos_rooms(1);

        let assigned = assign_users_to_rooms(&erm, &mut chaos);
        assert_eq!(assigned, 0);
    }

    #[test]
    fn test_chaos_no_nation_separation() {
        let (erm, mut chaos) = make_erm_with_chaos_rooms(1);

        // Add all Karus users — should still go to mixed_users
        for i in 0..5 {
            erm.add_signed_up_user(format!("k{}", i), i as u16, 1);
        }

        let assigned = assign_users_to_rooms(&erm, &mut chaos);
        assert_eq!(assigned, 5);

        let room = erm
            .get_room(TempleEventType::ChaosDungeon, 1)
            .expect("room 1");
        // All in mixed_users, not karus_users
        assert_eq!(room.mixed_users.len(), 5);
        assert_eq!(room.karus_users.len(), 0);
        assert_eq!(room.elmorad_users.len(), 0);
    }

    #[test]
    fn test_player_stats_tracking() {
        let mut chaos = ChaosManager::new(1);
        let erm = EventRoomManager::new();
        chaos.init_rooms(&erm);

        // Simulate kills and deaths
        chaos.record_kill(1, "player1");
        chaos.record_kill(1, "player1");
        chaos.record_death(1, "player1");
        chaos.record_kill(1, "player2");

        let stats1 = chaos.get_player_stats(1, "player1").unwrap();
        assert_eq!(stats1.kills, 2);
        assert_eq!(stats1.deaths, 1);
        assert_eq!(stats1.ranking_score(), 9); // 2*5 - 1

        let stats2 = chaos.get_player_stats(1, "player2").unwrap();
        assert_eq!(stats2.kills, 1);
        assert_eq!(stats2.deaths, 0);
        assert_eq!(stats2.ranking_score(), 5); // 1*5 - 0
    }

    #[test]
    fn test_room_rankings() {
        let mut chaos = ChaosManager::new(1);
        let erm = EventRoomManager::new();
        chaos.init_rooms(&erm);

        // Player1: 3 kills, 1 death = score 14
        chaos.record_kill(1, "player1");
        chaos.record_kill(1, "player1");
        chaos.record_kill(1, "player1");
        chaos.record_death(1, "player1");

        // Player2: 1 kill, 0 deaths = score 5
        chaos.record_kill(1, "player2");

        // Player3: 0 kills, 2 deaths = score -2
        chaos.record_death(1, "player3");
        chaos.record_death(1, "player3");

        let rankings = chaos.get_room_rankings(1);
        assert_eq!(rankings.len(), 3);
        assert_eq!(rankings[0].0, "player1"); // highest score
        assert_eq!(rankings[1].0, "player2");
        assert_eq!(rankings[2].0, "player3"); // lowest score
    }

    #[test]
    fn test_determine_all_winners_ffa() {
        let (erm, _chaos) = make_erm_with_chaos_rooms(2);

        // Add users to room 1
        erm.add_signed_up_user("p1".to_string(), 1, 1);
        erm.add_signed_up_user("p2".to_string(), 2, 2);

        // Manually place users in rooms
        {
            let mut room = erm.get_room_mut(TempleEventType::ChaosDungeon, 1).unwrap();
            room.add_user(EventUser {
                user_name: "p1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
            room.add_user(EventUser {
                user_name: "p2".to_string(),
                session_id: 2,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
        }

        let results = determine_all_winners(&erm);
        assert_eq!(results.len(), 1); // Only room 1 has users
        assert_eq!(results[0].0, 1); // room_id
        assert_eq!(results[0].1, 0); // winner_nation always 0 for Chaos
        // regression: rooms must NOT be pre-finished, or the reward pass skips them
        assert!(!erm.get_room(TempleEventType::ChaosDungeon, 1).unwrap().finished);
    }

    #[test]
    fn test_chaos_manager_init_and_destroy() {
        let erm = EventRoomManager::new();
        let mut chaos = ChaosManager::new(5);

        chaos.init_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::ChaosDungeon), 5);
        assert_eq!(chaos.room_player_stats.len(), 5);

        chaos.destroy_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::ChaosDungeon), 0);
        assert!(chaos.room_player_stats.is_empty());
    }

    #[test]
    fn test_stats_for_nonexistent_room() {
        let chaos = ChaosManager::new(1);
        // No init_rooms called, so room_player_stats is empty
        assert!(chaos.get_player_stats(1, "player1").is_none());
        assert!(chaos.get_room_rankings(99).is_empty());
    }

    #[test]
    fn test_assign_initializes_player_stats() {
        let (erm, mut chaos) = make_erm_with_chaos_rooms(1);

        erm.add_signed_up_user("p1".to_string(), 1, 1);
        erm.add_signed_up_user("p2".to_string(), 2, 2);

        assign_users_to_rooms(&erm, &mut chaos);

        // Player stats should be initialized with 0 kills/deaths
        let stats1 = chaos.get_player_stats(1, "p1").unwrap();
        assert_eq!(stats1.kills, 0);
        assert_eq!(stats1.deaths, 0);

        let stats2 = chaos.get_player_stats(1, "p2").unwrap();
        assert_eq!(stats2.kills, 0);
        assert_eq!(stats2.deaths, 0);
    }
}
