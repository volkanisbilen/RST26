//! Border Defence War (BDW) system — event-specific logic.
//! BDW is a PvP room-based event where Karus and El Morad teams fight in zone 84.
//! Each room holds up to 8 players per nation (16 total).
//! ## Scoring
//! - Kill an enemy player: `+1 * nation_user_count` to scoreboard, +1 bdw_points to killer
//! - Deliver altar flag: `+10 * nation_user_count` to scoreboard, +10 bdw_points to all same-nation
//! - Score >= dynamic threshold → that nation wins early
//! ## Altar System ()
//! 1. Altar NPC (ALTAR_OF_MANES = 9840) spawns in BDW zone 84
//! 2. When killed, the killer picks up the flag (`has_altar_obtained = true`)
//! 3. Carrier walks to their nation's base area (coordinate check)
//! 4. On delivery: scoreboard += 10 * nation_count, +10 bdw_points to all same-nation
//! 5. Altar respawns after 60 seconds
//! ## Win Condition ()
//! Dynamic threshold based on total players in room:
//! - total >= 16 → 600, >= 10 → 400, >= 5 → 300, else → 130

use std::collections::HashMap;

use crate::systems::event_room::{
    EventRoom, EventRoomManager, EventUser, RoomState, TempleEventType, MAX_ROOM_USERS_PER_NATION,
};

pub use crate::world::types::ZONE_BDW;

/// Default maximum rooms for BDW.
pub const DEFAULT_BDW_ROOMS: u8 = 10;

/// Per-kill score multiplier (base points per kill, multiplied by nation_count).
pub const KILL_POINTS: i32 = 1;

/// Per-altar-delivery score multiplier (base points, multiplied by nation_count).
pub const ALTAR_DELIVERY_POINTS: i32 = 10;

/// Per-user BDW points awarded to ALL same-nation users on altar delivery.
pub const ALTAR_DELIVERY_USER_POINTS: u32 = 10;

/// NPC proto ID for the BDW altar.
pub const ALTAR_OF_MANES: u16 = 9840;

pub use crate::npc_type_constants::NPC_BORDER_MONUMENT;

/// Altar respawn delay in seconds after delivery or carrier logout.
pub const ALTAR_RESPAWN_DELAY_SECS: u64 = 60;

/// Buff skill applied to the altar flag carrier.
pub const BUFF_FRAGMENT_OF_MANES_SKILL: u32 = 492063;

pub use crate::buff_constants::BUFF_TYPE_FRAGMENT_OF_MANES;

/// Speed modifier applied by the Fragment of Manes buff.
/// `m_bSpeedAmount = pType->bSpeed` — 50 means 50% movement speed.
/// Default (no buff) is 100. The exact DB value may differ; 50 is a safe
/// reference default matching common KO server setups.
pub const FRAGMENT_SPEED_VALUE: i32 = 75;

// ── Delivery Zone Coordinates ────────────────────────────────────────────

/// Karus base delivery zone bounds.
pub const KARUS_BASE_X_MIN: f32 = 28.0;
pub const KARUS_BASE_X_MAX: f32 = 35.0;
pub const KARUS_BASE_Z_MIN: f32 = 128.0;
pub const KARUS_BASE_Z_MAX: f32 = 135.0;

/// El Morad base delivery zone bounds.
pub const ELMORAD_BASE_X_MIN: f32 = 220.0;
pub const ELMORAD_BASE_X_MAX: f32 = 227.0;
pub const ELMORAD_BASE_Z_MIN: f32 = 127.0;
pub const ELMORAD_BASE_Z_MAX: f32 = 135.0;

// ── BDW Room Extended State ─────────────────────────────────────────────────

/// Extended BDW room state beyond the generic `EventRoom`.
/// Tracks altar respawn timing and monument counts for BDW-specific mechanics.
#[derive(Debug, Clone)]
pub struct BdwRoomState {
    /// NPC ID of the altar in this room (0 = not yet spawned).
    ///
    pub altar_npc_id: u32,
    /// Whether altar respawn is pending (waiting for timer expiry).
    ///
    pub altar_respawn_pending: bool,
    /// Unix timestamp when altar should respawn (0 = not set).
    ///
    pub altar_respawn_time: u64,
    /// Number of altar deliveries by Karus.
    ///
    pub karus_monument_count: u32,
    /// Number of altar deliveries by El Morad.
    ///
    pub elmorad_monument_count: u32,
}

impl BdwRoomState {
    /// Create a new default BDW room state.
    pub fn new() -> Self {
        Self {
            altar_npc_id: 0,
            altar_respawn_pending: false,
            altar_respawn_time: 0,
            karus_monument_count: 0,
            elmorad_monument_count: 0,
        }
    }

    /// Reset the BDW room state.
    ///
    pub fn reset(&mut self) {
        let altar = self.altar_npc_id;
        *self = Self::new();
        self.altar_npc_id = altar;
    }
}

impl Default for BdwRoomState {
    fn default() -> Self {
        Self::new()
    }
}

// ── BDW Manager ─────────────────────────────────────────────────────────────

/// BDW event manager — coordinates BDW-specific logic.
/// Stored alongside `EventRoomManager` in WorldState.
/// Provides BDW-specific operations that build on the generic room system.
#[derive(Debug)]
pub struct BdwManager {
    /// Extended BDW state per room: keyed by room_id.
    pub room_states: HashMap<u8, BdwRoomState>,
    /// Number of rooms to create for this event.
    pub max_rooms: u8,
}

impl BdwManager {
    /// Create a new BDW manager.
    pub fn new(max_rooms: u8) -> Self {
        Self {
            room_states: HashMap::new(),
            max_rooms,
        }
    }

    /// Initialize BDW rooms in the event room manager and create extended state.
    pub fn init_rooms(&mut self, erm: &EventRoomManager) {
        erm.create_rooms(TempleEventType::BorderDefenceWar, self.max_rooms);
        self.room_states.clear();
        for room_id in 1..=self.max_rooms {
            self.room_states.insert(room_id, BdwRoomState::new());
        }
    }

    /// Destroy all BDW rooms and clear extended state.
    pub fn destroy_rooms(&mut self, erm: &EventRoomManager) {
        erm.destroy_rooms(TempleEventType::BorderDefenceWar);
        self.room_states.clear();
    }

    /// Get extended BDW state for a room.
    pub fn get_room_state(&self, room_id: u8) -> Option<&BdwRoomState> {
        self.room_states.get(&room_id)
    }

    /// Get mutable extended BDW state for a room.
    pub fn get_room_state_mut(&mut self, room_id: u8) -> Option<&mut BdwRoomState> {
        self.room_states.get_mut(&room_id)
    }

    /// Reset all room states.
    pub fn reset_all(&mut self) {
        for state in self.room_states.values_mut() {
            state.reset();
        }
    }
}

impl Default for BdwManager {
    fn default() -> Self {
        Self::new(DEFAULT_BDW_ROOMS)
    }
}

// ── Room Assignment ─────────────────────────────────────────────────────────

/// Assign signed-up users to BDW rooms.
/// Distributes priests evenly first, then fills remaining slots with other classes.
/// This ensures each room has healer coverage.
/// Returns the number of users assigned to rooms.
pub fn assign_users_to_rooms(erm: &EventRoomManager, _bdw: &mut BdwManager) -> usize {
    let users = erm.signed_up_users.read().clone();
    if users.is_empty() {
        return 0;
    }

    // Separate into nation queues
    let mut karus_queue: Vec<_> = users.iter().filter(|u| u.nation == 1).collect();
    let mut elmorad_queue: Vec<_> = users.iter().filter(|u| u.nation == 2).collect();

    // Sort by join order for deterministic assignment
    karus_queue.sort_by_key(|u| u.join_order);
    elmorad_queue.sort_by_key(|u| u.join_order);

    let mut total_assigned = 0;
    let room_ids = erm.list_rooms(TempleEventType::BorderDefenceWar);
    let mut sorted_rooms = room_ids;
    sorted_rooms.sort();

    for room_id in &sorted_rooms {
        if karus_queue.is_empty() && elmorad_queue.is_empty() {
            break;
        }

        let mut room = match erm.get_room_mut(TempleEventType::BorderDefenceWar, *room_id) {
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

// ── Scoring ─────────────────────────────────────────────────────────────────

/// Record a kill in a BDW room (simple version for unit tests).
/// The actual kill scoring with nation_count multiplier is in `dead.rs:track_bdw_player_kill`.
/// This helper adds basic KILL_POINTS for tests that don't need the full scoring pipeline.
/// Returns the updated (karus_score, elmorad_score).
pub fn record_kill(room: &mut EventRoom, killer_nation: u8) -> (i32, i32) {
    match killer_nation {
        1 => room.karus_score += KILL_POINTS,
        2 => room.elmorad_score += KILL_POINTS,
        _ => {}
    }
    (room.karus_score, room.elmorad_score)
}

// ── Win Condition ───────────────────────────────────────────────────────────

/// Compute the win threshold based on total players in the room.
/// ```text
/// if (t_count >= 16) TotalPoint = 600;
/// else if (t_count >= 10) TotalPoint = 400;
/// else if (t_count >= 5) TotalPoint = 300;
/// else TotalPoint = 130;
/// ```
pub fn compute_win_threshold(total_players: usize) -> i32 {
    if total_players >= 16 {
        600
    } else if total_players >= 10 {
        400
    } else if total_players >= 5 {
        300
    } else {
        130
    }
}

/// Check win condition and set winner_nation if threshold exceeded.
/// Returns `Some(winner_nation)` if a winner was found, `None` otherwise.
pub fn check_win_condition(room: &mut EventRoom) -> Option<u8> {
    if room.winner_nation != 0 {
        return None; // already decided
    }

    let active_karus = room.karus_users.values().filter(|u| !u.logged_out).count();
    let active_elmorad = room
        .elmorad_users
        .values()
        .filter(|u| !u.logged_out)
        .count();
    let threshold = compute_win_threshold(active_karus + active_elmorad);

    if room.elmorad_score >= threshold {
        room.winner_nation = 2;
        Some(2)
    } else if room.karus_score >= threshold {
        room.winner_nation = 1;
        Some(1)
    } else {
        None
    }
}

/// Determine the winner of a BDW room by final score comparison.
/// Higher score wins. On tie, returns 0 (draw).
pub fn determine_winner(room: &EventRoom) -> u8 {
    if room.karus_score > room.elmorad_score {
        1 // Karus
    } else if room.elmorad_score > room.karus_score {
        2 // El Morad
    } else {
        0 // Draw
    }
}

/// Determine winners for all BDW rooms and set their winner_nation field.
/// Returns a list of (room_id, winner_nation) pairs.
pub fn determine_all_winners(erm: &EventRoomManager) -> Vec<(u8, u8)> {
    let room_ids = erm.list_rooms(TempleEventType::BorderDefenceWar);
    let mut results = Vec::with_capacity(room_ids.len());

    for room_id in room_ids {
        if let Some(mut room) = erm.get_room_mut(TempleEventType::BorderDefenceWar, room_id) {
            if room.finished || room.state != RoomState::Running {
                continue;
            }
            let winner = determine_winner(&room);
            room.winner_nation = winner;
            room.finish_packet_sent = true;
            results.push((room_id, winner));
        }
    }

    results
}

// ── Altar Delivery ──────────────────────────────────────────────────────────

/// Check if a player is in their nation's altar delivery zone.
/// - Karus base: X=[28-35], Z=[128-135]
/// - El Morad base: X=[220-227], Z=[127-135]
pub fn is_in_delivery_zone(nation: u8, x: f32, z: f32) -> bool {
    match nation {
        1 => {
            (KARUS_BASE_X_MIN..=KARUS_BASE_X_MAX).contains(&x)
                && (KARUS_BASE_Z_MIN..=KARUS_BASE_Z_MAX).contains(&z)
        }
        2 => {
            (ELMORAD_BASE_X_MIN..=ELMORAD_BASE_X_MAX).contains(&x)
                && (ELMORAD_BASE_Z_MIN..=ELMORAD_BASE_Z_MAX).contains(&z)
        }
        _ => false,
    }
}

/// Process an altar delivery: update scoreboard, monument count, and per-user points.
/// Called when a flag carrier enters their nation's base zone.
/// Returns `(new_karus_score, new_elmorad_score, winner_nation_if_finished)`.
pub fn altar_delivery_score_change(
    room: &mut EventRoom,
    bdw_state: &mut BdwRoomState,
    delivering_nation: u8,
    now: u64,
) -> (i32, i32, Option<u8>) {
    if room.finish_packet_sent {
        return (room.karus_score, room.elmorad_score, None);
    }

    let e_count = room
        .elmorad_users
        .values()
        .filter(|u| !u.logged_out)
        .count() as i32;
    let k_count = room.karus_users.values().filter(|u| !u.logged_out).count() as i32;

    // Score += 10 * nation_user_count for the delivering nation
    if delivering_nation == 2 && e_count > 0 {
        room.elmorad_score += ALTAR_DELIVERY_POINTS * e_count;
    } else if delivering_nation == 1 && k_count > 0 {
        room.karus_score += ALTAR_DELIVERY_POINTS * k_count;
    }

    // Increment monument count
    if delivering_nation == 2 {
        bdw_state.elmorad_monument_count += 1;
    } else {
        bdw_state.karus_monument_count += 1;
    }

    // Award +10 bdw_points to ALL same-nation users
    if delivering_nation == 1 {
        for u in room.karus_users.values_mut() {
            if !u.logged_out {
                u.bdw_points += ALTAR_DELIVERY_USER_POINTS;
            }
        }
    } else if delivering_nation == 2 {
        for u in room.elmorad_users.values_mut() {
            if !u.logged_out {
                u.bdw_points += ALTAR_DELIVERY_USER_POINTS;
            }
        }
    }

    // Check win condition
    let winner = check_win_condition(room);

    // On win: set finish state and clear altar respawn timer.
    if winner.is_some() {
        room.finish_packet_sent = true;
        room.finish_time_counter = now + 20;
        bdw_state.altar_respawn_pending = false;
        bdw_state.altar_respawn_time = 0;
    }

    (room.karus_score, room.elmorad_score, winner)
}

/// Set up altar respawn timer after delivery or carrier logout.
pub fn start_altar_respawn_timer(bdw_state: &mut BdwRoomState, now: u64) {
    bdw_state.altar_respawn_time = now + ALTAR_RESPAWN_DELAY_SECS;
    bdw_state.altar_respawn_pending = true;
}

/// Check if the altar respawn timer has expired for a room.
/// Returns true if the altar should be respawned now.
pub fn altar_timer_tick(bdw_state: &BdwRoomState, now: u64) -> bool {
    if !bdw_state.altar_respawn_pending {
        return false;
    }
    if bdw_state.altar_respawn_time == 0 {
        return false;
    }
    now >= bdw_state.altar_respawn_time
}

/// Complete altar respawn: clear the respawn timer.
pub fn altar_respawn_complete(bdw_state: &mut BdwRoomState) {
    bdw_state.altar_respawn_pending = false;
    bdw_state.altar_respawn_time = 0;
}

/// Handle flag carrier logout: clear flag and start respawn timer.
/// Returns true if the carrier had the flag (timer was started).
pub fn flag_carrier_logout(
    room: &mut EventRoom,
    bdw_state: &mut BdwRoomState,
    user_name: &str,
    now: u64,
) -> bool {
    // Find user in either nation's list and check if they have the flag
    let had_flag = if let Some(u) = room.karus_users.get_mut(user_name) {
        if u.has_altar_obtained {
            u.has_altar_obtained = false;
            true
        } else {
            false
        }
    } else if let Some(u) = room.elmorad_users.get_mut(user_name) {
        if u.has_altar_obtained {
            u.has_altar_obtained = false;
            true
        } else {
            false
        }
    } else {
        false
    };

    if had_flag {
        start_altar_respawn_timer(bdw_state, now);
    }
    had_flag
}

/// Mark a user as the flag carrier after they killed the altar NPC.
/// Returns the carrier's nation (1 or 2), or 0 if user not found.
pub fn flag_pickup(room: &mut EventRoom, user_name: &str) -> u8 {
    if let Some(u) = room.karus_users.get_mut(user_name) {
        u.has_altar_obtained = true;
        return 1;
    }
    if let Some(u) = room.elmorad_users.get_mut(user_name) {
        u.has_altar_obtained = true;
        return 2;
    }
    0
}

/// Find the current flag carrier in a room.
/// Returns `Some((user_name, session_id, nation))` if someone has the flag.
pub fn find_flag_carrier(room: &EventRoom) -> Option<(String, u16, u8)> {
    for u in room.karus_users.values() {
        if u.has_altar_obtained && !u.logged_out {
            return Some((u.user_name.clone(), u.session_id, 1));
        }
    }
    for u in room.elmorad_users.values() {
        if u.has_altar_obtained && !u.logged_out {
            return Some((u.user_name.clone(), u.session_id, 2));
        }
    }
    None
}

// ── Fragment of Manes (altar carrier) effect ─────────────────────────────────

/// Give the altar carrier the Fragment of Manes state: a permanent Type4 buff (server-side
/// speed 75 from `magic_type4` skill 492063, fallback [`FRAGMENT_SPEED_VALUE`]) plus the client
/// side `MAGIC_EFFECTING` for skill 492063 (C++ `BDWMonumentAltarSystem` runs the same skill on the
/// carrier), which makes the client slow the character down and play the skill's effect on it.
/// Cleared by [`clear_fragment_effect`] on delivery, death, logout, leave and event end.
pub fn apply_fragment_effect(world: &crate::world::WorldState, sid: crate::zone::SessionId) {
    use crate::world::ActiveBuff;
    use ko_protocol::{Opcode, Packet};

    let t4 = world.get_magic_type4(BUFF_FRAGMENT_OF_MANES_SKILL as i32);
    let speed = t4
        .as_ref()
        .and_then(|t| t.speed)
        .filter(|s| (1..100).contains(s))
        .unwrap_or(FRAGMENT_SPEED_VALUE);
    world.apply_buff(
        sid,
        ActiveBuff {
            skill_id: BUFF_FRAGMENT_OF_MANES_SKILL,
            buff_type: BUFF_TYPE_FRAGMENT_OF_MANES,
            caster_sid: sid,
            start_time: std::time::Instant::now(),
            duration_secs: 0, // permanent until explicitly removed
            attack_speed: 0,
            speed,
            ac: 0,
            ac_pct: 0,
            attack: 0,
            magic_attack: 0,
            max_hp: 0,
            max_hp_pct: 0,
            max_mp: 0,
            max_mp_pct: 0,
            str_mod: 0,
            sta_mod: 0,
            dex_mod: 0,
            intel_mod: 0,
            cha_mod: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            hit_rate: 0,
            avoid_rate: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: true, // C++ classifies this as buff (not debuff)
        },
    );
    world.set_user_ability(sid);

    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(3); // MAGIC_EFFECTING
    pkt.write_u32(BUFF_FRAGMENT_OF_MANES_SKILL);
    pkt.write_u32(sid as u32);
    pkt.write_u32(sid as u32);
    for v in [0u32, 1, 0, 0, 0, speed as u32, 0] {
        pkt.write_u32(v);
    }
    if let Some((pos, room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            std::sync::Arc::new(pkt),
            None,
            room,
        );
    }
}

/// Undo [`apply_fragment_effect`]: drop the buff, tell the client the buff ended and refresh the
/// derived stats. Returns whether the player was carrying the effect.
pub fn clear_fragment_effect(world: &crate::world::WorldState, sid: crate::zone::SessionId) -> bool {
    if world.remove_buff(sid, BUFF_TYPE_FRAGMENT_OF_MANES).is_none() {
        return false;
    }
    world.send_to_session_owned(
        sid,
        crate::systems::buff_tick::build_buff_expired_packet(BUFF_TYPE_FRAGMENT_OF_MANES as u8),
    );
    world.set_user_ability(sid);
    true
}

/// Test helper: the Altar of Manes template (max HP 1000).
#[cfg(test)]
pub(crate) fn test_altar_template() -> crate::npc::NpcTemplate {
    crate::npc::NpcTemplate {
        s_sid: ALTAR_OF_MANES,
        is_monster: true,
        name: "Altar".into(),
        pid: 0,
        size: 100,
        weapon_1: 0,
        weapon_2: 0,
        group: 0,
        act_type: 1,
        npc_type: NPC_BORDER_MONUMENT,
        family_type: 0,
        selling_group: 0,
        level: 1,
        max_hp: 1000,
        max_mp: 0,
        attack: 0,
        ac: 0,
        hit_rate: 0,
        evade_rate: 0,
        damage: 0,
        attack_delay: 1500,
        speed_1: 0,
        speed_2: 0,
        stand_time: 3000,
        search_range: 0,
        attack_range: 0,
        direct_attack: 0,
        tracing_range: 0,
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
        exp: 0,
        loyalty: 0,
        money: 0,
        item_table: 0,
        area_range: 0.0,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::systems::event_room::EventRoomManager;

    fn make_erm_with_bdw_rooms(count: u8) -> EventRoomManager {
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, count);
        erm
    }

    fn make_user(name: &str, sid: u16, nation: u8) -> EventUser {
        EventUser {
            user_name: name.to_string(),
            session_id: sid,
            nation,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        }
    }

    #[test]
    fn test_bdw_room_state_default() {
        let state = BdwRoomState::new();
        assert_eq!(state.altar_npc_id, 0);
        assert!(!state.altar_respawn_pending);
        assert_eq!(state.altar_respawn_time, 0);
        assert_eq!(state.karus_monument_count, 0);
        assert_eq!(state.elmorad_monument_count, 0);
    }

    #[test]
    fn test_bdw_room_state_reset_preserves_altar_npc() {
        let mut state = BdwRoomState::new();
        state.altar_npc_id = 42;
        state.altar_respawn_pending = true;
        state.altar_respawn_time = 99999;
        state.karus_monument_count = 5;
        state.elmorad_monument_count = 3;

        state.reset();
        assert_eq!(state.altar_npc_id, 42); // preserved
        assert!(!state.altar_respawn_pending);
        assert_eq!(state.altar_respawn_time, 0);
        assert_eq!(state.karus_monument_count, 0);
        assert_eq!(state.elmorad_monument_count, 0);
    }

    #[test]
    fn test_bdw_manager_init_destroy() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(5);

        bdw.init_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::BorderDefenceWar), 5);
        assert_eq!(bdw.room_states.len(), 5);
        assert!(bdw.get_room_state(1).is_some());
        assert!(bdw.get_room_state(5).is_some());
        assert!(bdw.get_room_state(6).is_none());

        bdw.destroy_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::BorderDefenceWar), 0);
        assert!(bdw.room_states.is_empty());
    }

    #[test]
    fn test_bdw_manager_reset_all() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(3);
        bdw.init_rooms(&erm);

        bdw.get_room_state_mut(1).unwrap().altar_npc_id = 100;
        bdw.get_room_state_mut(1).unwrap().karus_monument_count = 5;
        bdw.get_room_state_mut(2).unwrap().elmorad_monument_count = 3;

        bdw.reset_all();

        // altar_npc_id preserved, counts reset
        assert_eq!(bdw.get_room_state(1).unwrap().altar_npc_id, 100);
        assert_eq!(bdw.get_room_state(1).unwrap().karus_monument_count, 0);
        assert_eq!(bdw.get_room_state(2).unwrap().elmorad_monument_count, 0);
    }

    #[test]
    fn test_record_kill_karus() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let (k, e) = record_kill(&mut room, 1);
        assert_eq!(k, KILL_POINTS);
        assert_eq!(e, 0);

        let (k2, _) = record_kill(&mut room, 1);
        assert_eq!(k2, KILL_POINTS * 2);
    }

    #[test]
    fn test_record_kill_elmorad() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let (k, e) = record_kill(&mut room, 2);
        assert_eq!(k, 0);
        assert_eq!(e, KILL_POINTS);
    }

    #[test]
    fn test_record_kill_invalid_nation() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let (k, e) = record_kill(&mut room, 0);
        assert_eq!(k, 0);
        assert_eq!(e, 0);

        let (k, e) = record_kill(&mut room, 3);
        assert_eq!(k, 0);
        assert_eq!(e, 0);
    }

    #[test]
    fn test_compute_win_threshold() {
        assert_eq!(compute_win_threshold(16), 600);
        assert_eq!(compute_win_threshold(20), 600);
        assert_eq!(compute_win_threshold(10), 400);
        assert_eq!(compute_win_threshold(15), 400);
        assert_eq!(compute_win_threshold(5), 300);
        assert_eq!(compute_win_threshold(9), 300);
        assert_eq!(compute_win_threshold(4), 130);
        assert_eq!(compute_win_threshold(1), 130);
        assert_eq!(compute_win_threshold(0), 130);
    }

    #[test]
    fn test_is_in_delivery_zone_karus() {
        // Inside Karus base
        assert!(is_in_delivery_zone(1, 30.0, 130.0));
        assert!(is_in_delivery_zone(1, 28.0, 128.0)); // min bounds
        assert!(is_in_delivery_zone(1, 35.0, 135.0)); // max bounds

        // Outside Karus base
        assert!(!is_in_delivery_zone(1, 27.9, 130.0));
        assert!(!is_in_delivery_zone(1, 35.1, 130.0));
        assert!(!is_in_delivery_zone(1, 30.0, 127.9));
        assert!(!is_in_delivery_zone(1, 30.0, 135.1));

        // Karus player in Elmorad base — not valid
        assert!(!is_in_delivery_zone(1, 223.0, 130.0));
    }

    #[test]
    fn test_is_in_delivery_zone_elmorad() {
        // Inside Elmorad base
        assert!(is_in_delivery_zone(2, 223.0, 130.0));
        assert!(is_in_delivery_zone(2, 220.0, 127.0)); // min bounds
        assert!(is_in_delivery_zone(2, 227.0, 135.0)); // max bounds

        // Outside Elmorad base
        assert!(!is_in_delivery_zone(2, 219.9, 130.0));
        assert!(!is_in_delivery_zone(2, 227.1, 130.0));

        // Elmorad player in Karus base — not valid
        assert!(!is_in_delivery_zone(2, 30.0, 130.0));
    }

    #[test]
    fn test_is_in_delivery_zone_invalid_nation() {
        assert!(!is_in_delivery_zone(0, 30.0, 130.0));
        assert!(!is_in_delivery_zone(3, 223.0, 130.0));
    }

    #[test]
    fn test_altar_delivery_score_change_karus() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        // Add 4 Karus + 4 Elmorad users
        for i in 0..4 {
            room.add_user(make_user(&format!("k{i}"), i, 1));
            room.add_user(make_user(&format!("e{i}"), 100 + i, 2));
        }

        // Karus delivers altar
        let (k, e, winner) = altar_delivery_score_change(&mut room, &mut bdw_state, 1, 1000);

        // Score: 10 * 4 (Karus nation count) = 40
        assert_eq!(k, 40);
        assert_eq!(e, 0);
        assert_eq!(bdw_state.karus_monument_count, 1);
        assert_eq!(bdw_state.elmorad_monument_count, 0);

        // All Karus users should have +10 bdw_points
        for u in room.karus_users.values() {
            assert_eq!(u.bdw_points, 10);
        }
        // Elmorad users should have 0
        for u in room.elmorad_users.values() {
            assert_eq!(u.bdw_points, 0);
        }

        // 40 < 130 threshold for 8 players, no winner yet
        assert!(winner.is_none());
    }

    #[test]
    fn test_altar_delivery_score_change_elmorad() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("k2", 2, 1));
        room.add_user(make_user("e1", 101, 2));
        room.add_user(make_user("e2", 102, 2));
        room.add_user(make_user("e3", 103, 2));

        // Elmorad delivers altar
        let (k, e, _) = altar_delivery_score_change(&mut room, &mut bdw_state, 2, 1000);

        // Score: 10 * 3 (Elmorad nation count) = 30
        assert_eq!(k, 0);
        assert_eq!(e, 30);
        assert_eq!(bdw_state.elmorad_monument_count, 1);

        // All Elmorad users should have +10
        for u in room.elmorad_users.values() {
            assert_eq!(u.bdw_points, 10);
        }
    }

    #[test]
    fn test_altar_delivery_triggers_win() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        // 2 users total → threshold = 130
        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("e1", 101, 2));

        // Pre-set karus score close to threshold
        room.karus_score = 125;

        // Karus delivers altar: +10 * 1 = 10 → total = 135 > 130
        let (k, _e, winner) = altar_delivery_score_change(&mut room, &mut bdw_state, 1, 1000);
        assert_eq!(k, 135);
        assert_eq!(winner, Some(1)); // Karus wins!
        assert_eq!(room.winner_nation, 1);

        // C++ lines 276-280: finish state must be set on win
        assert!(room.finish_packet_sent);
        assert_eq!(room.finish_time_counter, 1020); // now + 20
        assert!(!bdw_state.altar_respawn_pending);
        assert_eq!(bdw_state.altar_respawn_time, 0);
    }

    #[test]
    fn test_altar_delivery_skips_logged_out_users() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("k2", 2, 1));
        // Mark k2 as logged out
        room.karus_users.get_mut("k2").unwrap().logged_out = true;

        room.add_user(make_user("e1", 101, 2));

        // Karus delivers: only 1 active Karus user → score += 10 * 1 = 10
        let (k, _, _) = altar_delivery_score_change(&mut room, &mut bdw_state, 1, 1000);
        assert_eq!(k, 10);

        // Only active users get +10 bdw_points
        assert_eq!(room.karus_users.get("k1").unwrap().bdw_points, 10);
        assert_eq!(room.karus_users.get("k2").unwrap().bdw_points, 0); // logged out
    }

    #[test]
    fn test_altar_delivery_no_op_if_finish_sent() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        room.add_user(make_user("k1", 1, 1));
        room.finish_packet_sent = true;

        let (k, e, winner) = altar_delivery_score_change(&mut room, &mut bdw_state, 1, 1000);
        assert_eq!(k, 0);
        assert_eq!(e, 0);
        assert!(winner.is_none());
    }

    #[test]
    fn test_altar_respawn_timer() {
        let mut bdw_state = BdwRoomState::new();

        // Not pending → tick returns false
        assert!(!altar_timer_tick(&bdw_state, 100));

        // Start timer at t=100
        start_altar_respawn_timer(&mut bdw_state, 100);
        assert!(bdw_state.altar_respawn_pending);
        assert_eq!(bdw_state.altar_respawn_time, 160);

        // Too early → false
        assert!(!altar_timer_tick(&bdw_state, 130));
        assert!(!altar_timer_tick(&bdw_state, 159));

        // Exactly at threshold → true
        assert!(altar_timer_tick(&bdw_state, 160));
        assert!(altar_timer_tick(&bdw_state, 200));

        // Complete respawn
        altar_respawn_complete(&mut bdw_state);
        assert!(!bdw_state.altar_respawn_pending);
        assert_eq!(bdw_state.altar_respawn_time, 0);
        assert!(!altar_timer_tick(&bdw_state, 200));
    }

    #[test]
    fn test_flag_pickup() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.add_user(make_user("kUser", 1, 1));
        room.add_user(make_user("eUser", 2, 2));

        // Karus user picks up flag
        assert_eq!(flag_pickup(&mut room, "kUser"), 1);
        assert!(room.karus_users.get("kUser").unwrap().has_altar_obtained);

        // Unknown user
        assert_eq!(flag_pickup(&mut room, "nobody"), 0);

        // Elmorad user picks up flag
        assert_eq!(flag_pickup(&mut room, "eUser"), 2);
        assert!(room.elmorad_users.get("eUser").unwrap().has_altar_obtained);
    }

    #[test]
    fn test_find_flag_carrier() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("e1", 2, 2));

        // No carrier initially
        assert!(find_flag_carrier(&room).is_none());

        // k1 picks up flag
        room.karus_users.get_mut("k1").unwrap().has_altar_obtained = true;
        let carrier = find_flag_carrier(&room).unwrap();
        assert_eq!(carrier.0, "k1");
        assert_eq!(carrier.1, 1);
        assert_eq!(carrier.2, 1);

        // Logged out carrier is not found
        room.karus_users.get_mut("k1").unwrap().logged_out = true;
        assert!(find_flag_carrier(&room).is_none());
    }

    #[test]
    fn test_flag_carrier_logout() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        room.add_user(make_user("k1", 1, 1));
        room.karus_users.get_mut("k1").unwrap().has_altar_obtained = true;

        // Carrier logs out
        let had_flag = flag_carrier_logout(&mut room, &mut bdw_state, "k1", 1000);
        assert!(had_flag);
        assert!(!room.karus_users.get("k1").unwrap().has_altar_obtained);
        assert!(bdw_state.altar_respawn_pending);
        assert_eq!(bdw_state.altar_respawn_time, 1060);

        // Non-carrier logout
        room.add_user(make_user("k2", 2, 1));
        let had_flag = flag_carrier_logout(&mut room, &mut bdw_state, "k2", 1000);
        assert!(!had_flag);
    }

    #[test]
    fn test_check_win_condition_no_winner() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("e1", 2, 2));
        room.karus_score = 50;
        room.elmorad_score = 50;

        assert!(check_win_condition(&mut room).is_none());
    }

    #[test]
    fn test_check_win_condition_karus_wins() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("e1", 2, 2));
        room.karus_score = 130; // threshold for 2 players = 130

        assert_eq!(check_win_condition(&mut room), Some(1));
        assert_eq!(room.winner_nation, 1);
    }

    #[test]
    fn test_check_win_condition_already_decided() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.winner_nation = 1;
        room.elmorad_score = 9999;

        // Already decided → returns None
        assert!(check_win_condition(&mut room).is_none());
    }

    #[test]
    fn test_determine_winner_karus_wins() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_score = 15;
        room.elmorad_score = 10;
        assert_eq!(determine_winner(&room), 1);
    }

    #[test]
    fn test_determine_winner_elmorad_wins() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_score = 5;
        room.elmorad_score = 12;
        assert_eq!(determine_winner(&room), 2);
    }

    #[test]
    fn test_determine_winner_draw() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_score = 10;
        room.elmorad_score = 10;
        assert_eq!(determine_winner(&room), 0);
    }

    #[test]
    fn test_determine_winner_both_zero() {
        let room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        assert_eq!(determine_winner(&room), 0);
    }

    #[test]
    fn test_determine_all_winners() {
        let erm = make_erm_with_bdw_rooms(3);

        if let Some(mut r) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 1) {
            r.state = RoomState::Running;
            r.karus_score = 20;
            r.elmorad_score = 10;
        }
        if let Some(mut r) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 2) {
            r.state = RoomState::Running;
            r.karus_score = 5;
            r.elmorad_score = 15;
        }
        if let Some(mut r) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 3) {
            r.state = RoomState::Running;
            r.karus_score = 8;
            r.elmorad_score = 8;
        }

        let results = determine_all_winners(&erm);
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
        let erm = make_erm_with_bdw_rooms(2);

        if let Some(mut r) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 1) {
            r.state = RoomState::Running;
            r.karus_score = 10;
        }
        if let Some(mut r) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 2) {
            r.state = RoomState::Running;
            r.finished = true;
            r.karus_score = 20;
        }

        let results = determine_all_winners(&erm);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn test_assign_users_to_rooms_basic() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(2);
        bdw.init_rooms(&erm);

        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("k2".to_string(), 2, 1);
        erm.add_signed_up_user("e1".to_string(), 3, 2);
        erm.add_signed_up_user("e2".to_string(), 4, 2);

        let assigned = assign_users_to_rooms(&erm, &mut bdw);
        assert_eq!(assigned, 4);

        let room = erm.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
        assert_eq!(room.karus_users.len(), 2);
        assert_eq!(room.elmorad_users.len(), 2);
        assert_eq!(room.state, RoomState::Running);
    }

    #[test]
    fn test_assign_users_overflow_to_second_room() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(2);
        bdw.init_rooms(&erm);

        for i in 0..10 {
            erm.add_signed_up_user(format!("k{}", i), i as u16, 1);
        }
        for i in 0..10 {
            erm.add_signed_up_user(format!("e{}", i), (100 + i) as u16, 2);
        }

        let assigned = assign_users_to_rooms(&erm, &mut bdw);
        assert_eq!(assigned, 20);

        let room1 = erm.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
        assert_eq!(room1.karus_users.len(), MAX_ROOM_USERS_PER_NATION);
        assert_eq!(room1.elmorad_users.len(), MAX_ROOM_USERS_PER_NATION);

        let room2 = erm.get_room(TempleEventType::BorderDefenceWar, 2).unwrap();
        assert_eq!(room2.karus_users.len(), 2);
        assert_eq!(room2.elmorad_users.len(), 2);
    }

    #[test]
    fn test_assign_users_empty() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(2);
        bdw.init_rooms(&erm);

        let assigned = assign_users_to_rooms(&erm, &mut bdw);
        assert_eq!(assigned, 0);
    }

    #[test]
    fn test_assign_users_imbalanced_nations() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(2);
        bdw.init_rooms(&erm);

        for i in 0..5 {
            erm.add_signed_up_user(format!("k{}", i), i as u16, 1);
        }
        for i in 0..2 {
            erm.add_signed_up_user(format!("e{}", i), (100 + i) as u16, 2);
        }

        let assigned = assign_users_to_rooms(&erm, &mut bdw);
        assert_eq!(assigned, 7);
    }

    #[test]
    fn test_constants() {
        assert_eq!(ZONE_BDW, 84);
        assert_eq!(KILL_POINTS, 1);
        assert_eq!(ALTAR_DELIVERY_POINTS, 10);
        assert_eq!(ALTAR_DELIVERY_USER_POINTS, 10);
        assert_eq!(ALTAR_OF_MANES, 9840);
        assert_eq!(NPC_BORDER_MONUMENT, 212);
        assert_eq!(ALTAR_RESPAWN_DELAY_SECS, 60);
        assert_eq!(BUFF_TYPE_FRAGMENT_OF_MANES, 52);
        assert_eq!(BUFF_FRAGMENT_OF_MANES_SKILL, 492063);
        assert!(FRAGMENT_SPEED_VALUE < 100); // debuff reduces speed below 100%
    }

    #[test]
    fn test_flag_carrier_logout_after_finish_still_clears_flag() {
        // Regression test for QA HIGH: flag carrier cleanup must run
        // even after finish_packet_sent is true.
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        room.add_user(make_user("k1", 1, 1));
        room.karus_users.get_mut("k1").unwrap().has_altar_obtained = true;
        room.finish_packet_sent = true; // game already finished

        // flag_carrier_logout should still clear the flag and start respawn
        let had_flag = flag_carrier_logout(&mut room, &mut bdw_state, "k1", 1000);
        assert!(had_flag);
        assert!(!room.karus_users.get("k1").unwrap().has_altar_obtained);
        assert!(bdw_state.altar_respawn_pending);
    }

    #[test]
    fn test_altar_npc_id_stored_on_pickup() {
        let mut bdw_state = BdwRoomState::new();
        assert_eq!(bdw_state.altar_npc_id, 0);

        // Simulate storing altar NPC ID on flag pickup
        bdw_state.altar_npc_id = 42;
        assert_eq!(bdw_state.altar_npc_id, 42);

        // Reset preserves altar_npc_id
        bdw_state.reset();
        assert_eq!(bdw_state.altar_npc_id, 42);
    }

    #[test]
    fn test_bdw_full_room_lifecycle_with_altar() {
        let erm = EventRoomManager::new();
        let mut bdw = BdwManager::new(1);
        bdw.init_rooms(&erm);

        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("k2".to_string(), 2, 1);
        erm.add_signed_up_user("e1".to_string(), 3, 2);
        erm.add_signed_up_user("e2".to_string(), 4, 2);

        let assigned = assign_users_to_rooms(&erm, &mut bdw);
        assert_eq!(assigned, 4);

        // Record kills: Karus kills x2
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            record_kill(&mut room, 1);
            record_kill(&mut room, 1);
        }

        // Altar pickup by k1
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            let nation = flag_pickup(&mut room, "k1");
            assert_eq!(nation, 1);
        }

        // Check carrier
        {
            let room = erm.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
            let carrier = find_flag_carrier(&room);
            assert_eq!(carrier.unwrap().0, "k1");
        }

        // Deliver altar
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            let bdw_state = bdw.get_room_state_mut(1).unwrap();

            // Clear the carrier flag
            room.karus_users.get_mut("k1").unwrap().has_altar_obtained = false;

            let (k, e, _winner) = altar_delivery_score_change(&mut room, bdw_state, 1, 1000);
            // Kill score: 2, altar score: 10 * 2 = 20, total = 22
            assert_eq!(k, 22);
            assert_eq!(e, 0);
        }

        // Start respawn timer
        {
            let bdw_state = bdw.get_room_state_mut(1).unwrap();
            start_altar_respawn_timer(bdw_state, 1000);
        }

        // Determine winner
        {
            let room = erm.get_room(TempleEventType::BorderDefenceWar, 1).unwrap();
            let winner = determine_winner(&room);
            assert_eq!(winner, 1); // Karus
        }

        bdw.destroy_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::BorderDefenceWar), 0);
    }

    #[test]
    fn test_multiple_deliveries_accumulate() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let mut bdw_state = BdwRoomState::new();

        room.add_user(make_user("k1", 1, 1));
        room.add_user(make_user("k2", 2, 1));
        room.add_user(make_user("e1", 3, 2));

        // First Karus delivery: +10 * 2 = 20
        altar_delivery_score_change(&mut room, &mut bdw_state, 1, 1000);
        assert_eq!(room.karus_score, 20);
        assert_eq!(bdw_state.karus_monument_count, 1);

        // Second Karus delivery: +10 * 2 = 20 → total 40
        altar_delivery_score_change(&mut room, &mut bdw_state, 1, 1000);
        assert_eq!(room.karus_score, 40);
        assert_eq!(bdw_state.karus_monument_count, 2);

        // Each Karus user got +10 twice = 20
        assert_eq!(room.karus_users.get("k1").unwrap().bdw_points, 20);
        assert_eq!(room.karus_users.get("k2").unwrap().bdw_points, 20);
    }

    // ── Fragment of Manes (carrier slow + effect) ────────────────────

    fn carrier_world(sid: u16) -> (crate::world::WorldState, tokio::sync::mpsc::UnboundedReceiver<std::sync::Arc<ko_protocol::Packet>>) {
        use crate::world::{CharacterInfo, Position};
        let world = crate::world::WorldState::new();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                name: "Carrier".into(),
                nation: 1,
                level: 70,
                hp: 100,
                max_hp: 100,
                authority: 1,
                ..Default::default()
            },
            Position {
                zone_id: ZONE_BDW,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: crate::zone::calc_region(100.0),
                region_z: crate::zone::calc_region(100.0),
            },
        );
        (world, rx)
    }

    #[test]
    fn fragment_effect_slows_the_carrier_and_tells_the_client() {
        let (world, mut rx) = carrier_world(1);
        apply_fragment_effect(&world, 1);
        let buffs = world.get_active_buffs(1);
        let b = buffs
            .iter()
            .find(|b| b.buff_type == BUFF_TYPE_FRAGMENT_OF_MANES)
            .expect("carrier buff stored");
        assert!(b.speed > 0 && b.speed < 100, "slowed, not frozen: {}", b.speed);
        assert_eq!(b.skill_id, BUFF_FRAGMENT_OF_MANES_SKILL);
        // the client gets the MAGIC_EFFECTING of skill 492063 (speed in data[5])
        let mut effecting = false;
        while let Ok(p) = rx.try_recv() {
            if p.opcode == ko_protocol::Opcode::WizMagicProcess as u8
                && p.data.first() == Some(&3)
                && p.data.get(1..5) == Some(&BUFF_FRAGMENT_OF_MANES_SKILL.to_le_bytes()[..])
            {
                effecting = true;
            }
        }
        assert!(effecting, "MAGIC_EFFECTING for the Fragment of Manes skill was sent");
    }

    #[test]
    fn fragment_effect_clears_once_and_notifies_the_client() {
        let (world, mut rx) = carrier_world(1);
        apply_fragment_effect(&world, 1);
        while rx.try_recv().is_ok() {}
        assert!(clear_fragment_effect(&world, 1));
        assert!(!world.has_buff(1, BUFF_TYPE_FRAGMENT_OF_MANES));
        let mut ended = false;
        while let Ok(p) = rx.try_recv() {
            if p.opcode == ko_protocol::Opcode::WizMagicProcess as u8 {
                ended = true;
            }
        }
        assert!(ended, "buff-expired packet sent so the client drops the effect");
        assert!(!clear_fragment_effect(&world, 1), "second clear is a no-op");
    }
}
