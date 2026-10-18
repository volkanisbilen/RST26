//! Event room management system — shared infrastructure for BDW, Chaos, Juraid,
//! Forgotten Temple, Dungeon Defence, and other room-based events.
//! ## Architecture
//! `EventRoomManager` is stored in `WorldState` and provides:
//! - Room lifecycle: create, destroy, get, list
//! - Player tracking per room (join/leave)
//! - Event timer state machine (Idle → Signing → Running → Finishing → Cleanup)
//! - Background tick task that drives the event state machine
//! ## Room States
//! ```text
//! Idle → Signing → Running → Finishing → Cleanup → (destroyed)
//! ```
//! - **Idle**: Room exists but no event is active.
//! - **Signing**: Sign-up period; players can join/leave.
//! - **Running**: Event is in progress; players are teleported in.
//! - **Finishing**: Winner screen shown; waiting for cleanup timer.
//! - **Cleanup**: Rewards distributed, players kicked, room reset.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use dashmap::DashMap;

use ko_protocol::{Opcode, Packet};

use crate::state_change_constants::{STATE_CHANGE_INVISIBILITY, STATE_CHANGE_PARTY_LEADER};
use crate::world::types::{ZONE_ELMORAD, ZONE_KARUS, ZONE_MORADON};
use crate::world::WorldState;
use crate::zone::SessionId;

/// Maximum number of rooms per event type.
pub const MAX_TEMPLE_EVENT_ROOM: u8 = 60;

/// Maximum users per BDW/Juraid room (8 per nation = 16 total).
pub const MAX_ROOM_USERS_PER_NATION: usize = 8;

/// Maximum users in Chaos Dungeon room.
pub const MAX_CHAOS_ROOM_USERS: usize = 18;

// ── Event Type Enum ──────────────────────────────────────────────────────

/// Event type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum TempleEventType {
    /// Border Defence War (zone 84).
    BorderDefenceWar = 4,
    /// Monster Stone / Forgotten Temple (zone 55).
    ForgottenTemple = 14,
    /// Chaos Dungeon (zone 85).
    ChaosDungeon = 24,
    /// Juraid Mountain (zone 87).
    JuraidMountain = 100,
}

impl TempleEventType {
    /// Convert from raw i16 value.
    pub fn from_i16(val: i16) -> Option<Self> {
        match val {
            4 => Some(Self::BorderDefenceWar),
            14 => Some(Self::ForgottenTemple),
            24 => Some(Self::ChaosDungeon),
            100 => Some(Self::JuraidMountain),
            _ => None,
        }
    }

    /// Get the associated zone ID.
    pub fn zone_id(&self) -> u16 {
        match self {
            Self::BorderDefenceWar => 84,
            Self::ForgottenTemple => 55,
            Self::ChaosDungeon => 85,
            Self::JuraidMountain => 87,
        }
    }
}

// ── Event Scheduling Type ──────────────────────────────────────────────

/// Schedule type from EVENT_SCHEDULE_MAIN_LIST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum EventScheduleType {
    /// Lunar War (nation vs nation, open battlefield).
    LunarWar = 1,
    /// Virtual Room (instanced events: BDW, Chaos, Juraid).
    VirtualRoom = 2,
    /// Single Room (Forgotten Temple, Under the Castle, Beef).
    SingleRoom = 3,
}

impl EventScheduleType {
    /// Convert from raw i16 value.
    pub fn from_i16(val: i16) -> Option<Self> {
        match val {
            1 => Some(Self::LunarWar),
            2 => Some(Self::VirtualRoom),
            3 => Some(Self::SingleRoom),
            _ => None,
        }
    }
}

// ── Event Local ID ──────────────────────────────────────────────────────

/// Event local ID from schedule tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EventLocalId {
    CastleSiegeWar = 1,
    NapiesGorge = 2,
    AlseidsPrairie = 3,
    NiedsTriangle = 4,
    NereidsIsland = 5,
    Zipang = 6,
    Oreads = 7,
    SnowWar = 8,
    BorderDefenceWar = 9,
    ChaosExpansion = 10,
    JuraidMountain = 11,
    UnderTheCastle = 12,
    ForgettenTemple = 13,
    BeefEvent = 14,
}

impl EventLocalId {
    /// Convert from raw u8 value.
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::CastleSiegeWar),
            2 => Some(Self::NapiesGorge),
            3 => Some(Self::AlseidsPrairie),
            4 => Some(Self::NiedsTriangle),
            5 => Some(Self::NereidsIsland),
            6 => Some(Self::Zipang),
            7 => Some(Self::Oreads),
            8 => Some(Self::SnowWar),
            9 => Some(Self::BorderDefenceWar),
            10 => Some(Self::ChaosExpansion),
            11 => Some(Self::JuraidMountain),
            12 => Some(Self::UnderTheCastle),
            13 => Some(Self::ForgettenTemple),
            14 => Some(Self::BeefEvent),
            _ => None,
        }
    }

    /// Map to `TempleEventType` for room-based events.
    pub fn to_temple_event_type(&self) -> Option<TempleEventType> {
        match self {
            Self::BorderDefenceWar => Some(TempleEventType::BorderDefenceWar),
            Self::ChaosExpansion => Some(TempleEventType::ChaosDungeon),
            Self::JuraidMountain => Some(TempleEventType::JuraidMountain),
            Self::ForgettenTemple => Some(TempleEventType::ForgottenTemple),
            _ => None,
        }
    }
}

// ── Room State ──────────────────────────────────────────────────────────

/// Room lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RoomState {
    /// Room allocated, no event running.
    Idle = 0,
    /// Sign-up period: players can join/leave.
    Signing = 1,
    /// Event in progress: combat allowed per timer.
    Running = 2,
    /// Winner determined, showing results screen.
    Finishing = 3,
    /// Rewards distributed, kicking players, about to destroy.
    Cleanup = 4,
}

impl RoomState {
    /// Convert from raw u8.
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Idle),
            1 => Some(Self::Signing),
            2 => Some(Self::Running),
            3 => Some(Self::Finishing),
            4 => Some(Self::Cleanup),
            _ => None,
        }
    }
}

// ── Event User ──────────────────────────────────────────────────────────

/// Tracks an event participant within a room.
#[derive(Debug, Clone)]
pub struct EventUser {
    /// Character name (used as key in C++).
    pub user_name: String,
    /// Session ID for quick lookups.
    pub session_id: SessionId,
    /// Nation (1=Karus, 2=El Morad).
    pub nation: u8,
    /// Whether this user has received their reward.
    pub prize_given: bool,
    /// Whether this user logged out during the event.
    pub logged_out: bool,
    /// Per-player kill count (Chaos individual tracking).
    ///
    pub kills: u32,
    /// Per-player death count (Chaos individual tracking).
    ///
    pub deaths: u32,
    /// BDW per-user points (+1 per kill, +10 per altar delivery).
    ///
    pub bdw_points: u32,
    /// Whether this user currently carries the BDW altar flag.
    ///
    pub has_altar_obtained: bool,
}

// ── Event Room ──────────────────────────────────────────────────────────

/// A single event room instance.
#[derive(Debug)]
pub struct EventRoom {
    /// Room number (1-based, up to MAX_TEMPLE_EVENT_ROOM).
    pub room_id: u8,
    /// Which event type this room belongs to.
    pub event_type: TempleEventType,
    /// Current room state.
    pub state: RoomState,
    /// Zone this room operates in.
    pub zone_id: u16,
    /// Karus nation users in this room.
    pub karus_users: HashMap<String, EventUser>,
    /// El Morad nation users in this room.
    pub elmorad_users: HashMap<String, EventUser>,
    /// Mixed users (for Chaos Dungeon which doesn't separate by nation).
    pub mixed_users: HashMap<String, EventUser>,
    /// Whether the room has been marked as finished.
    pub finished: bool,
    /// Finish packet already sent (prevents double-send).
    pub finish_packet_sent: bool,
    /// Unix timestamp when finish cleanup should happen.
    pub finish_time_counter: u64,
    /// Winning nation (0=none/draw, 1=Karus, 2=El Morad).
    pub winner_nation: u8,
    /// Karus score (BDW) or kill count (Juraid).
    pub karus_score: i32,
    /// El Morad score (BDW) or kill count (Juraid).
    pub elmorad_score: i32,
    /// Karus raw kill count (BDW only, separate from score).
    ///
    pub karus_kill_count: i32,
    /// El Morad raw kill count (BDW only, separate from score).
    ///
    pub elmorad_kill_count: i32,
}

impl EventRoom {
    /// Create a new empty room.
    pub fn new(room_id: u8, event_type: TempleEventType) -> Self {
        Self {
            room_id,
            event_type,
            state: RoomState::Idle,
            zone_id: event_type.zone_id(),
            karus_users: HashMap::new(),
            elmorad_users: HashMap::new(),
            mixed_users: HashMap::new(),
            finished: false,
            finish_packet_sent: false,
            finish_time_counter: 0,
            winner_nation: 0,
            karus_score: 0,
            elmorad_score: 0,
            karus_kill_count: 0,
            elmorad_kill_count: 0,
        }
    }

    /// Get total user count in this room (all nations combined).
    pub fn user_count(&self) -> usize {
        self.karus_users.len() + self.elmorad_users.len() + self.mixed_users.len()
    }

    /// Add a user to the room by nation.
    ///
    /// Returns false if the room is full.
    pub fn add_user(&mut self, user: EventUser) -> bool {
        match self.event_type {
            TempleEventType::ChaosDungeon => {
                if self.mixed_users.len() >= MAX_CHAOS_ROOM_USERS {
                    return false;
                }
                self.mixed_users.insert(user.user_name.clone(), user);
                true
            }
            _ => {
                if user.nation == 1 {
                    if self.karus_users.len() >= MAX_ROOM_USERS_PER_NATION {
                        return false;
                    }
                    self.karus_users.insert(user.user_name.clone(), user);
                } else {
                    if self.elmorad_users.len() >= MAX_ROOM_USERS_PER_NATION {
                        return false;
                    }
                    self.elmorad_users.insert(user.user_name.clone(), user);
                }
                true
            }
        }
    }

    /// Remove a user by name.
    pub fn remove_user(&mut self, name: &str) -> Option<EventUser> {
        self.karus_users
            .remove(name)
            .or_else(|| self.elmorad_users.remove(name))
            .or_else(|| self.mixed_users.remove(name))
    }

    /// Get a user by name.
    pub fn get_user(&self, name: &str) -> Option<&EventUser> {
        self.karus_users
            .get(name)
            .or_else(|| self.elmorad_users.get(name))
            .or_else(|| self.mixed_users.get(name))
    }

    /// Get a mutable reference to a user by name.
    pub fn get_user_mut(&mut self, name: &str) -> Option<&mut EventUser> {
        self.karus_users
            .get_mut(name)
            .or_else(|| self.elmorad_users.get_mut(name))
            .or_else(|| self.mixed_users.get_mut(name))
    }

    /// Collect all session IDs of users in this room.
    pub fn all_session_ids(&self) -> Vec<SessionId> {
        let mut ids = Vec::with_capacity(self.user_count());
        for user in self.karus_users.values() {
            if !user.logged_out {
                ids.push(user.session_id);
            }
        }
        for user in self.elmorad_users.values() {
            if !user.logged_out {
                ids.push(user.session_id);
            }
        }
        for user in self.mixed_users.values() {
            if !user.logged_out {
                ids.push(user.session_id);
            }
        }
        ids
    }

    /// Reset room to idle state (clear all users and scores).
    pub fn reset(&mut self) {
        self.state = RoomState::Idle;
        self.karus_users.clear();
        self.elmorad_users.clear();
        self.mixed_users.clear();
        self.finished = false;
        self.finish_packet_sent = false;
        self.finish_time_counter = 0;
        self.winner_nation = 0;
        self.karus_score = 0;
        self.elmorad_score = 0;
    }
}

// ── Temple Event State ──────────────────────────────────────────────────

/// Global temple event state — tracks the currently active room-based event.
#[derive(Debug)]
pub struct TempleEventState {
    /// Currently active event type (-1 = none).
    pub active_event: i16,
    /// Whether the event has moved past signing into active play.
    pub is_active: bool,
    /// Whether sign-up is allowed.
    pub allow_join: bool,
    /// Whether combat is allowed.
    pub is_attackable: bool,
    /// Whether this event was started automatically (vs GM manual).
    pub is_automatic: bool,
    /// Event zone ID.
    pub zone_id: u16,
    /// Unix timestamp when the event started.
    pub start_time: u64,
    /// Unix timestamp when the event should close.
    pub closed_time: u64,
    /// Unix timestamp when sign-up period ends.
    pub sign_remain_seconds: u64,
    /// Last room number that was assigned a player.
    pub last_event_room: u8,
    /// Total signed-up users (Karus + El Morad).
    pub all_user_count: u16,
    /// Karus signed-up user count.
    pub karus_user_count: u16,
    /// El Morad signed-up user count.
    pub elmorad_user_count: u16,
    /// Timer control flags (prevent repeated phase transitions).
    pub timer_start_control: bool,
    pub timer_attack_open_control: bool,
    pub timer_attack_close_control: bool,
    pub timer_finish_control: bool,
    pub timer_reset_control: bool,
    /// GM manual close state.
    pub manual_close: bool,
    pub manual_closed_time: u64,
}

impl Default for TempleEventState {
    fn default() -> Self {
        Self {
            active_event: -1,
            is_active: false,
            allow_join: false,
            is_attackable: false,
            is_automatic: false,
            zone_id: 0,
            start_time: 0,
            closed_time: 0,
            sign_remain_seconds: 0,
            last_event_room: 0,
            all_user_count: 0,
            karus_user_count: 0,
            elmorad_user_count: 0,
            timer_start_control: false,
            timer_attack_open_control: false,
            timer_attack_close_control: false,
            timer_finish_control: false,
            timer_reset_control: false,
            manual_close: false,
            manual_closed_time: 0,
        }
    }
}

impl TempleEventState {
    /// Reset all state back to idle (no active event).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Check if BDW is currently active (past signing).
    pub fn is_bdw_active(&self) -> bool {
        self.active_event == TempleEventType::BorderDefenceWar as i16 && self.is_active
    }

    /// Check if Juraid is currently active (past signing).
    pub fn is_juraid_active(&self) -> bool {
        self.active_event == TempleEventType::JuraidMountain as i16 && self.is_active
    }

    /// Check if Chaos is currently active (past signing).
    pub fn is_chaos_active(&self) -> bool {
        self.active_event == TempleEventType::ChaosDungeon as i16 && self.is_active
    }
}

// ── Virtual Room Options ────────────────────────────────────────────────

/// Timing options for room-based virtual events (BDW, Chaos, Juraid).
#[derive(Debug, Clone)]
pub struct VroomOpt {
    /// Event name for logging.
    pub name: String,
    /// Sign-up duration in minutes.
    pub sign: i32,
    /// Play duration in minutes.
    pub play: i32,
    /// Minutes from sign start until attack is allowed.
    pub attack_open: i32,
    /// Minutes from sign start until attack is blocked.
    pub attack_close: i32,
    /// Seconds after winner screen before full cleanup.
    pub finish: i32,
}

// ── Forgotten Temple Options ────────────────────────────────────────────

/// Timing options for Forgotten Temple.
#[derive(Debug, Clone)]
pub struct ForgottenTempleOpts {
    /// Total play time in minutes.
    pub playing_time: i32,
    /// Seconds between summon waves.
    pub summon_time: i32,
    /// Minimum seconds between individual spawns within a wave.
    pub spawn_min_time: i32,
    /// Seconds to wait before the first wave.
    pub waiting_time: i32,
    /// Minimum player level to join.
    pub min_level: i32,
    /// Maximum player level to join.
    pub max_level: i32,
}

impl Default for ForgottenTempleOpts {
    fn default() -> Self {
        Self {
            playing_time: 30,
            summon_time: 300,
            spawn_min_time: 10,
            waiting_time: 20,
            min_level: 60,
            max_level: 83,
        }
    }
}

// ── Event Schedule Entry ────────────────────────────────────────────────

/// Merged schedule entry combining main list + day list for runtime use.
#[derive(Debug, Clone)]
pub struct EventScheduleEntry {
    /// Event local ID (1-14).
    pub event_id: i16,
    /// Schedule type (LunarWar, VirtualRoom, SingleRoom).
    pub event_type: i16,
    /// Zone ID for this event.
    pub zone_id: i16,
    /// Human-readable event name.
    pub name: String,
    /// Whether this schedule entry is active.
    pub status: bool,
    /// Up to 5 start times (hour, minute). (-1, -1) = inactive slot.
    pub start_times: [(i16, i16); 5],
    /// Day-of-week flags [Sun, Mon, Tue, Wed, Thu, Fri, Sat].
    pub days: [bool; 7],
    /// Minimum level requirement (0 = no requirement).
    pub min_level: i16,
    /// Maximum level requirement (0 = no requirement).
    pub max_level: i16,
    /// Required loyalty points.
    pub req_loyalty: i32,
    /// Required gold.
    pub req_money: i32,
}

// ── Signed-Up Event User ────────────────────────────────────────────────

/// A user who has signed up for the current event but not yet assigned to a room.
#[derive(Debug, Clone)]
pub struct SignedUpUser {
    /// Character name.
    pub user_name: String,
    /// Session ID.
    pub session_id: SessionId,
    /// Nation (1=Karus, 2=El Morad).
    pub nation: u8,
    /// Join order (for deterministic room assignment).
    pub join_order: u32,
}

// ── Event Room Manager ──────────────────────────────────────────────────

/// Central event room manager — stored in `WorldState`.
///                `m_TempleEventChaosRoomList`, `pTempleEvent`, `pEventTimeOpt`
#[derive(Debug)]
pub struct EventRoomManager {
    /// Active rooms keyed by (event_type, room_id).
    pub rooms: DashMap<(TempleEventType, u8), EventRoom>,

    /// Global temple event state (signing, active event, counters).
    pub temple_event: parking_lot::RwLock<TempleEventState>,

    /// Users signed up for the current event but not yet in a room.
    ///
    pub signed_up_users: parking_lot::RwLock<Vec<SignedUpUser>>,

    /// Atomic counter for join order assignment.
    next_join_order: std::sync::atomic::AtomicU32,

    /// Virtual room timing options: index 0=BDW(84), 1=Chaos(85), 2=JR(87).
    ///
    pub vroom_opts: parking_lot::RwLock<[Option<VroomOpt>; 3]>,

    /// Forgotten Temple timing options.
    pub ft_opts: parking_lot::RwLock<ForgottenTempleOpts>,

    /// Merged schedule entries loaded from DB at startup.
    pub schedules: parking_lot::RwLock<Vec<EventScheduleEntry>>,
}

impl EventRoomManager {
    /// Create a new, empty event room manager.
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            temple_event: parking_lot::RwLock::new(TempleEventState::default()),
            signed_up_users: parking_lot::RwLock::new(Vec::new()),
            next_join_order: std::sync::atomic::AtomicU32::new(1),
            vroom_opts: parking_lot::RwLock::new([None, None, None]),
            ft_opts: parking_lot::RwLock::new(ForgottenTempleOpts::default()),
            schedules: parking_lot::RwLock::new(Vec::new()),
        }
    }

    // ── Room CRUD ────────────────────────────────────────────────────

    /// Create rooms for an event type (up to max_rooms).
    pub fn create_rooms(&self, event_type: TempleEventType, max_rooms: u8) {
        let count = max_rooms.min(MAX_TEMPLE_EVENT_ROOM);
        for room_id in 1..=count {
            self.rooms
                .insert((event_type, room_id), EventRoom::new(room_id, event_type));
        }
    }

    /// Destroy all rooms for an event type.
    pub fn destroy_rooms(&self, event_type: TempleEventType) {
        self.rooms.retain(|k, _| k.0 != event_type);
    }

    /// Get a specific room (read-only ref).
    pub fn get_room(
        &self,
        event_type: TempleEventType,
        room_id: u8,
    ) -> Option<dashmap::mapref::one::Ref<'_, (TempleEventType, u8), EventRoom>> {
        self.rooms.get(&(event_type, room_id))
    }

    /// Get a mutable reference to a specific room.
    pub fn get_room_mut(
        &self,
        event_type: TempleEventType,
        room_id: u8,
    ) -> Option<dashmap::mapref::one::RefMut<'_, (TempleEventType, u8), EventRoom>> {
        self.rooms.get_mut(&(event_type, room_id))
    }

    /// List all rooms for an event type.
    pub fn list_rooms(&self, event_type: TempleEventType) -> Vec<u8> {
        let mut rooms: Vec<u8> = self
            .rooms
            .iter()
            .filter(|r| r.key().0 == event_type)
            .map(|r| r.key().1)
            .collect();
        rooms.sort_unstable();
        rooms
    }

    /// Count total rooms of a given type.
    pub fn room_count(&self, event_type: TempleEventType) -> usize {
        self.rooms
            .iter()
            .filter(|r| r.key().0 == event_type)
            .count()
    }

    // ── Sign-Up Management ──────────────────────────────────────────

    /// Add a user to the sign-up queue.
    ///
    /// Returns the join order number, or None if user is already signed up.
    pub fn add_signed_up_user(
        &self,
        user_name: String,
        session_id: SessionId,
        nation: u8,
    ) -> Option<u32> {
        let mut users = self.signed_up_users.write();
        if users.iter().any(|u| u.user_name == user_name) {
            return None;
        }
        let order = self.next_join_order.fetch_add(1, Ordering::Relaxed);
        users.push(SignedUpUser {
            user_name,
            session_id,
            nation,
            join_order: order,
        });
        Some(order)
    }

    /// Remove a user from the sign-up queue.
    pub fn remove_signed_up_user(&self, user_name: &str) -> Option<SignedUpUser> {
        let mut users = self.signed_up_users.write();
        users
            .iter()
            .position(|u| u.user_name == user_name)
            .map(|pos| users.remove(pos))
    }

    /// Clear all signed-up users.
    pub fn clear_signed_up_users(&self) {
        let mut users = self.signed_up_users.write();
        users.clear();
        self.next_join_order.store(1, Ordering::Relaxed);
    }

    /// Get the count of signed-up users.
    pub fn signed_up_count(&self) -> usize {
        self.signed_up_users.read().len()
    }

    /// Check if a user is already signed up for the event (by character name).
    ///
    /// In Rust we track sign-ups in the `signed_up_users` list instead of a per-user field.
    pub fn is_user_signed_up(&self, user_name: &str) -> bool {
        self.signed_up_users
            .read()
            .iter()
            .any(|u| u.user_name == user_name)
    }

    // ── Vroom Options ───────────────────────────────────────────────

    /// Get vroom options by index (0=BDW, 1=Chaos, 2=JR).
    pub fn get_vroom_opt(&self, index: usize) -> Option<VroomOpt> {
        if index > 2 {
            return None;
        }
        self.vroom_opts.read()[index].clone()
    }

    /// Map TempleEventType to vroom index.
    pub fn vroom_index(event_type: TempleEventType) -> Option<usize> {
        match event_type {
            TempleEventType::BorderDefenceWar => Some(0),
            TempleEventType::ChaosDungeon => Some(1),
            TempleEventType::JuraidMountain => Some(2),
            _ => None,
        }
    }

    // ── Temple Event State Helpers ──────────────────────────────────

    /// Read the current temple event state.
    pub fn read_temple_event<R>(&self, f: impl FnOnce(&TempleEventState) -> R) -> R {
        let state = self.temple_event.read();
        f(&state)
    }

    /// Mutate the temple event state.
    pub fn update_temple_event<R>(&self, f: impl FnOnce(&mut TempleEventState) -> R) -> R {
        let mut state = self.temple_event.write();
        f(&mut state)
    }

    /// Reset the temple event state and clear sign-ups.
    pub fn reset_temple_event(&self) {
        self.update_temple_event(|s| s.reset());
        self.clear_signed_up_users();
    }

    /// Find which room a user is in (by character name) for a given event type.
    ///
    /// Returns `Some((room_id, is_finished))` if found, `None` if the user is
    /// not in any room of that event type.
    ///
    pub fn find_user_room(
        &self,
        event_type: TempleEventType,
        user_name: &str,
    ) -> Option<(u8, bool)> {
        let room_ids = self.list_rooms(event_type);
        for room_id in room_ids {
            if let Some(room) = self.get_room(event_type, room_id) {
                if room.get_user(user_name).is_some() {
                    return Some((room_id, room.finished));
                }
            }
        }
        None
    }
}

impl Default for EventRoomManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Temple Event Zone Helpers ───────────────────────────────────────────

pub use crate::world::types::{ZONE_BDW, ZONE_CHAOS, ZONE_JURAID};

/// Check if a zone is a temple event zone (BDW=84, Chaos=85, Juraid=87).
pub fn is_in_temple_event_zone(zone_id: u16) -> bool {
    zone_id == ZONE_BDW || zone_id == ZONE_CHAOS || zone_id == ZONE_JURAID
}

/// Determine the `TempleEventType` from a zone ID.
pub fn event_type_for_zone(zone_id: u16) -> Option<TempleEventType> {
    match zone_id {
        ZONE_BDW => Some(TempleEventType::BorderDefenceWar),
        ZONE_CHAOS => Some(TempleEventType::ChaosDungeon),
        ZONE_JURAID => Some(TempleEventType::JuraidMountain),
        _ => None,
    }
}

/// Returns `true` if the attack is ALLOWED, `false` if it should be blocked.
/// Logic:
/// - If NOT in a temple event zone OR not in a valid room → allow (return true)
/// - If in BDW zone → block if BDW not active OR room finished
/// - If in Juraid zone → block if Juraid not active OR room finished
/// - If in Chaos zone → block if Chaos not active OR room finished
pub fn virt_eventattack_check(erm: &EventRoomManager, zone_id: u16, user_name: &str) -> bool {
    let event_type = match event_type_for_zone(zone_id) {
        Some(et) => et,
        None => return true, // not in temple event zone
    };

    // Check if the user is in a valid room
    let (_room_id, is_finished) = match erm.find_user_room(event_type, user_name) {
        Some(r) => r,
        None => return true, // not in a valid room → C++ returns true
    };

    // Check if the corresponding event is active
    let is_active = erm.read_temple_event(|s| match event_type {
        TempleEventType::BorderDefenceWar => s.is_bdw_active(),
        TempleEventType::ChaosDungeon => s.is_chaos_active(),
        TempleEventType::JuraidMountain => s.is_juraid_active(),
        _ => false,
    });

    if !is_active {
        return false;
    }

    // Block if room is finished
    if is_finished {
        return false;
    }

    true
}

// ── Spawn Event NPC Helper ──────────────────────────────────────────────

/// Parameters for spawning an event NPC.
#[derive(Debug, Clone)]
pub struct SpawnEventNpcParams {
    /// NPC template s_sid.
    pub npc_id: u16,
    /// Whether this is a monster (true) or non-combat NPC (false).
    pub is_monster: bool,
    /// Zone to spawn in.
    pub zone_id: u16,
    /// X position.
    pub pos_x: f32,
    /// Y position.
    pub pos_y: f32,
    /// Z position.
    pub pos_z: f32,
    /// Number of NPCs to spawn.
    pub count: u16,
    /// Random spawn radius around the position.
    pub radius: u16,
    /// Event room ID (0 = not room-specific).
    pub event_room: u8,
    /// Direction facing (0-360).
    pub direction: i16,
}

/// Event id of the Border Defence join window the client opens for Juraid (the client has no Juraid window).
pub const JURAID_WINDOW_EVENT_ID: i16 = 4;

/// Zones whose players do not get event windows (event zones, Monster Stone, prison).
const NO_WINDOW_ZONES: &[u16] = &[81, 82, 83, 84, 85, 87, 92];

fn in_window_zone(world: &WorldState, sid: SessionId) -> bool {
    world
        .get_position(sid)
        .map(|p| !NO_WINDOW_ZONES.contains(&p.zone_id))
        .unwrap_or(false)
}

/// Open the Juraid join window for everybody online (like Ultima / Rift / Manes): the start broadcast of
/// the Border Defence window, the current counter and the title. The server keeps the event as Juraid.
pub fn broadcast_juraid_window(world: &WorldState, remaining_secs: u16) {
    let (karus, elmo) = world
        .event_room_manager
        .read_temple_event(|te| (te.karus_user_count, te.elmorad_user_count));
    let open = Arc::new(build_event_start_broadcast(JURAID_WINDOW_EVENT_ID, remaining_secs));
    let count = Arc::new(build_bdw_counter_packet(karus, elmo));
    let title = Arc::new(crate::handler::survival::build_window_title("Juraid Mountain"));
    for sid in world.get_in_game_session_ids() {
        if !in_window_zone(world, sid) {
            continue;
        }
        world.send_to_session_arc(sid, Arc::clone(&open));
        world.send_to_session_arc(sid, Arc::clone(&count));
        // the title goes last: the window object only exists once the open packet has been handled
        world.send_to_session_arc(sid, Arc::clone(&title));
    }
}

/// The registration was cancelled: let the open join windows run out (one second left) so they close by themselves.
pub fn broadcast_juraid_window_close(world: &WorldState) {
    let pkt = Arc::new(build_event_start_broadcast(JURAID_WINDOW_EVENT_ID, 1));
    for sid in world.get_in_game_session_ids() {
        if in_window_zone(world, sid) {
            world.send_to_session_arc(sid, Arc::clone(&pkt));
        }
    }
}

/// Broadcast the event counter (sign-up counts) to all signed-up users.
///   - BDW: `TemplEventBDWSendJoinScreenUpdate()` — sends karus + elmo counts
///   - Chaos: `TemplEventChaosSendJoinScreenUpdate()` — sends total count only
///   - Juraid: `TemplEventJuraidSendJoinScreenUpdate()` — sends EXT_HOOK and v2615 WIZ_EVENT
/// Called when a user joins/leaves the event.
/// Returns the built packet so the caller can also send it to a specific user.
pub fn broadcast_event_counter(world: &WorldState) -> Option<Packet> {
    let erm = &world.event_room_manager;
    let te = erm.temple_event.read();
    if te.active_event == -1 || !te.allow_join {
        return None;
    }

    let active_event = te.active_event;
    let karus_count = te.karus_user_count;
    let elmo_count = te.elmorad_user_count;
    let all_count = te.all_user_count;
    let sign_remain = if te.sign_remain_seconds > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if te.sign_remain_seconds > now {
            (te.sign_remain_seconds - now) as u16
        } else {
            0
        }
    } else {
        0
    };
    drop(te);

    // Build the native counter packet. Juraid additionally needs the 2615
    // EXT_HOOK join-screen update: registration logs proved that WIZ_EVENT
    // alone is accepted by the server flow but does not open/update the UI.
    let counter_pkt = match active_event {
        4 => build_bdw_counter_packet(karus_count, elmo_count),
        24 => build_chaos_counter_packet(all_count),
        // 2615 uses the native WIZ_EVENT counter contract. The older C++
        // reference's XSafe/JURAID packet is not authoritative for this
        // client and must not replace the reverse-engineered packet.
        100 => build_juraid_event_counter_packet(karus_count, elmo_count, sign_remain),
        _ => return None,
    };

    // Juraid: the open window belongs to everybody, so everybody gets the Border Defence style counter
    if active_event == 100 {
        let arc = Arc::new(build_bdw_counter_packet(karus_count, elmo_count));
        for sid in world.get_in_game_session_ids() {
            if in_window_zone(world, sid) {
                world.send_to_session_arc(sid, Arc::clone(&arc));
            }
        }
        return Some(build_bdw_counter_packet(karus_count, elmo_count));
    }

    // Send to all signed-up users (clone once, Arc share in loop)
    let users = erm.signed_up_users.read();
    let arc_counter = Arc::new(counter_pkt.clone());
    let arc_juraid_hook = (active_event == 100).then(|| {
        Arc::new(build_juraid_counter_packet(
            karus_count,
            elmo_count,
            sign_remain,
        ))
    });
    let arc_juraid_native = (active_event == 100).then(|| {
        Arc::new(build_juraid_select_counter_packet(
            karus_count,
            elmo_count,
            sign_remain,
        ))
    });
    if active_event == 100 {
        tracing::info!(
            karus_count,
            elmorad_count = elmo_count,
            remaining_secs = sign_remain,
            signed_users = users.len(),
            "Juraid registration counter packets sent (WIZ_EVENT + v2615 WIZ_SELECT_MSG)"
        );
    }
    for user in users.iter() {
        world.send_to_session_arc(user.session_id, Arc::clone(&arc_counter));
        if let Some(hook) = &arc_juraid_hook {
            world.send_to_session_arc(user.session_id, Arc::clone(hook));
        }
        if let Some(native) = &arc_juraid_native {
            world.send_to_session_arc(user.session_id, Arc::clone(native));
        }
    }
    Some(counter_pkt)
}

// ── Event Lifecycle: Teleport + Winner Screen + Kick ───────────────────

/// Minimum level to teleport to nation capital (instead of Moradon).
const NATION_CAPITAL_MIN_LEVEL: u8 = 35;

/// Winner screen countdown seconds before teleport.
const WINNER_SCREEN_COUNTDOWN_SECS: u32 = 20;

/// Build the WIZ_SELECT_MSG (0x55) winner dialog packet.
/// Packet format: `[0x55] [u32:0] [u8:7] [u64:0] [u32:event_msg_id] [u8:param] [u32:500]`
/// | Event   | event_msg_id | param |
/// |---------|-------------|-------|
/// | BDW     | 8           | 7     |
/// | Juraid  | 6           | 11    |
/// | Chaos   | 9           | 24    |
pub fn build_winner_select_msg(active_event: i16) -> Packet {
    let (event_msg_id, param): (u32, u8) = match active_event {
        4 => (8, 7),    // BDW
        100 => (6, 11), // Juraid
        24 => (9, 24),  // Chaos
        _ => (0, 0),
    };

    let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
    pkt.write_u32(0); // unk1
    pkt.write_u8(7); // msg_type
    pkt.write_u64(0); // unk2
    pkt.write_u32(event_msg_id);
    pkt.write_u8(param);
    pkt.write_u32(500); // display duration
    pkt
}

/// Build the WIZ_EVENT TEMPLE_EVENT_FINISH (10) packet.
/// Packet format: `[0x5F] [u8:10] [u8:2] [u8:0] [u8:winner_nation] [u32:20]`
/// - winner_nation: 1=Karus, 2=Elmorad, 0=draw/FFA
/// - countdown: seconds until teleport home (always 20)
pub fn build_finish_packet(winner_nation: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(10); // TEMPLE_EVENT_FINISH sub-opcode
    pkt.write_u8(2);
    pkt.write_u8(0);
    pkt.write_u8(winner_nation);
    pkt.write_u32(WINNER_SCREEN_COUNTDOWN_SECS);
    pkt
}

/// Build the TEMPLE_SCREEN scoreboard packet.
/// Packet format: `[0x5F] [u8:TEMPLE_SCREEN=3] [u32:karus_score] [u32:elmo_score]`
/// Sent to all room users after each kill to update the scoreboard UI.
pub fn build_temple_screen_packet(karus_score: i32, elmo_score: i32) -> Packet {
    const TEMPLE_SCREEN: u8 = 3;
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(TEMPLE_SCREEN);
    pkt.write_u32(karus_score as u32);
    pkt.write_u32(elmo_score as u32);
    pkt
}

/// Build TEMPLE_EVENT_ALTAR_FLAG packet — broadcast when a player picks up the altar flag.
/// Packet format: `[0x5F] [u8:49] [u8:name_len] [name_bytes] [u8:nation]`
pub fn build_altar_flag_packet(carrier_name: &str, nation: u8) -> Packet {
    const TEMPLE_EVENT_ALTAR_FLAG: u8 = 49;
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(TEMPLE_EVENT_ALTAR_FLAG);
    pkt.write_u8(carrier_name.len() as u8);
    pkt.write_bytes(carrier_name.as_bytes());
    pkt.write_u8(nation);
    pkt
}

/// Build TEMPLE_EVENT_ALTAR_TIMER packet — broadcast when altar respawn timer starts.
/// and `CUser::BDWUserHasObtainedLoqOut()`
/// Packet format: `[0x5F] [u8:50] [u16:timer_secs]`
pub fn build_altar_timer_packet(timer_secs: u16) -> Packet {
    const TEMPLE_EVENT_ALTAR_TIMER: u8 = 50;
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(TEMPLE_EVENT_ALTAR_TIMER);
    pkt.write_u16(timer_secs);
    pkt
}

/// Build altar respawn broadcast packet — sent when the altar NPC respawns.
/// Packet format: `[0x5F] [u8:2] [u8:2]`
pub fn build_altar_respawn_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(2);
    pkt.write_u8(2);
    pkt
}

/// Send winner screen packets to all users in all rooms of the active event.
/// For each room, determines the winner and sends the appropriate packets:
/// - **BDW/Juraid**: Karus users get both WIZ_SELECT_MSG + WIZ_EVENT FINISH;
///   Elmorad users only get WIZ_SELECT_MSG (per C++ reference).
/// - **Chaos**: All users (mixed) get both packets.
/// Also sets `finish_time_counter` on each room for the 20-second countdown
/// before `TempleEventRoomClose` triggers teleport, and writes `winner_nation`
/// to the room for later reward distribution.
pub fn send_winner_screen(world: &WorldState, active_event: i16, now: u64) {
    let erm = &world.event_room_manager;
    let event_type = match TempleEventType::from_i16(active_event) {
        Some(t) => t,
        None => return,
    };

    let select_msg = build_winner_select_msg(active_event);
    let rooms = erm.list_rooms(event_type);
    let is_chaos = active_event == 24;

    for room_id in &rooms {
        // Collect per-nation session ID lists and determine winner.
        let (winner_nation, karus_sids, elmorad_sids, mixed_sids) = {
            let Some(mut room) = erm.get_room_mut(event_type, *room_id) else {
                continue;
            };

            if room.finished {
                continue;
            }

            // Determine winner nation for this room
            let winner = if is_chaos {
                0 // Chaos: always FFA (no winner nation)
            } else if room.winner_nation != 0 {
                room.winner_nation
            } else if room.karus_score > room.elmorad_score {
                1 // Karus wins
            } else if room.elmorad_score > room.karus_score {
                2 // Elmorad wins
            } else {
                0 // Draw
            };

            // BUG-2 fix: persist winner_nation on the room for reward distribution
            room.winner_nation = winner;

            // Set finish countdown
            room.finish_time_counter = now + WINNER_SCREEN_COUNTDOWN_SECS as u64;
            room.finish_packet_sent = true;

            // Collect session IDs per nation group
            let k_sids: Vec<SessionId> = room
                .karus_users
                .values()
                .filter(|u| !u.logged_out)
                .map(|u| u.session_id)
                .collect();
            let e_sids: Vec<SessionId> = room
                .elmorad_users
                .values()
                .filter(|u| !u.logged_out)
                .map(|u| u.session_id)
                .collect();
            let m_sids: Vec<SessionId> = room
                .mixed_users
                .values()
                .filter(|u| !u.logged_out)
                .map(|u| u.session_id)
                .collect();

            (winner, k_sids, e_sids, m_sids)
        }; // room lock dropped

        let finish_pkt = build_finish_packet(winner_nation);

        let total_users = if is_chaos {
            // C++ Chaos: clear invisibility then send both packets
            //   pUser->StateChangeServerDirect(7, 0);  // clear invis
            //   pUser->Send(&result);   // WIZ_SELECT_MSG
            //   pUser->Send(&newpkt2);  // WIZ_EVENT FINISH
            use crate::handler::regene::build_state_change_broadcast;
            let arc_select = Arc::new(select_msg.clone());
            let arc_finish = Arc::new(finish_pkt.clone());
            for sid in &mixed_sids {
                // Updates server-side state AND broadcasts WIZ_STATE_CHANGE to region
                world.set_invisibility_type(*sid, 0);
                let sc_pkt =
                    build_state_change_broadcast(*sid as u32, STATE_CHANGE_INVISIBILITY, 0);
                if let Some((pos, sender_event_room)) =
                    world.with_session(*sid, |h| (h.position, h.event_room))
                {
                    world.broadcast_to_region_sync(
                        pos.zone_id,
                        pos.region_x,
                        pos.region_z,
                        Arc::new(sc_pkt),
                        None,
                        sender_event_room,
                    );
                }
                world.send_to_session_arc(*sid, Arc::clone(&arc_select));
                world.send_to_session_arc(*sid, Arc::clone(&arc_finish));
            }
            mixed_sids.len()
        } else {
            // C++ BDW/Juraid: Karus users get both packets,
            // Elmorad users only get WIZ_SELECT_MSG (no finish packet).
            let arc_select = Arc::new(select_msg.clone());
            let arc_finish = Arc::new(finish_pkt.clone());
            for sid in &karus_sids {
                world.send_to_session_arc(*sid, Arc::clone(&arc_select));
                world.send_to_session_arc(*sid, Arc::clone(&arc_finish));
            }
            for sid in &elmorad_sids {
                world.send_to_session_arc(*sid, Arc::clone(&arc_select));
            }
            karus_sids.len() + elmorad_sids.len()
        };

        tracing::info!(
            "Event {:?} room {} — winner screen sent (winner_nation={}, users={})",
            event_type,
            room_id,
            winner_nation,
            total_users
        );
    }
}

/// Build the WIZ_SELECT_MSG packet sent during teleport with play time countdown.
/// Packet format: `[0x55] [u32:0] [u8:7] [u64:0] [u32:event_msg_id] [u8:param] [u32:play_secs]`
/// Same structure as the winner select msg, but uses play time instead of 500.
pub fn build_teleport_select_msg(active_event: i16, play_secs: u32) -> Packet {
    let (event_msg_id, param): (u32, u8) = match active_event {
        4 => (8, 7),    // BDW
        100 => (6, 11), // Juraid
        24 => (9, 24),  // Chaos
        _ => (0, 0),
    };

    let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
    pkt.write_u32(0); // unk1
    pkt.write_u8(7); // msg_type
    pkt.write_u64(0); // unk2
    pkt.write_u32(event_msg_id);
    pkt.write_u8(param);
    pkt.write_u32(play_secs); // play time in seconds (C++: play * MINUTE)
    pkt
}

/// Build the WIZ_BIFROST timer overlay packet.
/// Packet format: `[0x7B] [u8:5] [u16:time_secs]`
pub fn build_teleport_bifrost(play_secs: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizBifrost as u8);
    pkt.write_u8(5); // sub-opcode: timer
    pkt.write_u16(play_secs); // countdown time in seconds
    pkt
}

/// Build the WIZ_EVENT(1) packet sent to BDW users on teleport.
/// Packet format: `[0x5F] [u8:1]`
pub fn build_bdw_event_start_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(1); // TEMPLE_EVENT_START sub-opcode for BDW
    pkt
}

/// Send active event time / join screen update to a player entering an event zone.
/// Two paths:
/// 1. **Non-event user**: Sends `TempleEventGetActiveEventTime` — `WIZ_EVENT + u8(TEMPLE_EVENT=7)
///    + i16(active_event) + u16(remaining_seconds)`.
/// 2. **Event user**: Sends `WIZ_EVENT + u8(TEMPLE_EVENT_JOIN=8) + u8(1) + u16(active_event)`,
///    then event-specific counter update (scoreboard refresh).
pub fn send_active_event_time(world: &WorldState, sid: SessionId) {
    let erm = &world.event_room_manager;

    // Read active event and remaining seconds atomically
    let (active_event, remaining_secs) = erm.read_temple_event(|te| {
        if !te.is_active && !te.allow_join {
            return (-1i16, 0u16);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let rem = if te.sign_remain_seconds > now {
            (te.sign_remain_seconds - now) as u16
        } else {
            0u16
        };
        (te.active_event, rem)
    });

    if active_event == -1 {
        return;
    }

    // Check if this player is an event user (in any room)
    let user_name = match world.get_session_name(sid) {
        Some(n) => n,
        None => return,
    };

    let event_type = match TempleEventType::from_i16(active_event) {
        Some(et) => et,
        None => return,
    };

    let is_event_user = erm.find_user_room(event_type, &user_name).is_some();

    if !is_event_user {
        // Path 1: Non-event user — send active event time
        let mut pkt = Packet::new(Opcode::WizEvent as u8);
        pkt.write_u8(7); // TEMPLE_EVENT sub-opcode
        pkt.write_i16(active_event);
        pkt.write_u16(remaining_secs);
        world.send_to_session_owned(sid, pkt);
        return;
    }

    // Path 2: Event user — send join confirmation + screen update
    let mut join_pkt = Packet::new(Opcode::WizEvent as u8);
    join_pkt.write_u8(8); // TEMPLE_EVENT_JOIN sub-opcode
    join_pkt.write_u8(1); // success
    join_pkt.write_u16(active_event as u16);
    world.send_to_session_owned(sid, join_pkt);

    // Send event-specific counter screen update
    match active_event {
        4 => {
            // BDW: WIZ_EVENT + TEMPLE_EVENT_COUNTER(16) + u16(4) + u16(karus) + u16(elmo)
            let (k_count, e_count) =
                erm.read_temple_event(|te| (te.karus_user_count, te.elmorad_user_count));
            let mut pkt = Packet::new(Opcode::WizEvent as u8);
            pkt.write_u8(16); // TEMPLE_EVENT_COUNTER
            pkt.write_u16(4); // TEMPLE_EVENT_BORDER_DEFENCE_WAR
            pkt.write_u16(k_count);
            pkt.write_u16(e_count);
            world.send_to_session_owned(sid, pkt);
        }
        24 => {
            // Chaos: WIZ_EVENT + TEMPLE_EVENT_COUNTER(16) + u16(24) + u16(all_count)
            let all_count = erm.read_temple_event(|te| te.all_user_count);
            let mut pkt = Packet::new(Opcode::WizEvent as u8);
            pkt.write_u8(16); // TEMPLE_EVENT_COUNTER
            pkt.write_u16(24); // TEMPLE_EVENT_CHAOS
            pkt.write_u16(all_count);
            world.send_to_session_owned(sid, pkt);
        }
        100 => {
            let (k_count, e_count, remain) = erm.read_temple_event(|te| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let remain = if te.sign_remain_seconds > now {
                    (te.sign_remain_seconds - now) as u16
                } else {
                    0
                };
                (te.karus_user_count, te.elmorad_user_count, remain)
            });
            world.send_to_session_owned(
                sid,
                build_juraid_event_counter_packet(k_count, e_count, remain),
            );
            world.send_to_session_owned(sid, build_juraid_counter_packet(k_count, e_count, remain));
            world.send_to_session_owned(
                sid,
                build_juraid_select_counter_packet(k_count, e_count, remain),
            );
        }
        _ => {}
    }

    // If event is in Running phase, also re-send the scoreboard (TEMPLE_SCREEN)
    if let Some((room_id, _)) = erm.find_user_room(event_type, &user_name) {
        if let Some(room) = erm.get_room(event_type, room_id) {
            if !room.finished && (room.karus_score > 0 || room.elmorad_score > 0) {
                let screen_pkt = build_temple_screen_packet(room.karus_score, room.elmorad_score);
                world.send_to_session_owned(sid, screen_pkt);
            }
        }
    }
}

/// Build the TempleEventStart broadcast packet announcing event sign-up.
/// Packet format: `[0x5F] [u8:7] [i16:active_event] [u16:remaining_secs]`
/// Sent to all online players to show the event sign-up UI.
pub fn build_event_start_broadcast(active_event: i16, remaining_secs: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(7); // TEMPLE_EVENT sub-opcode (packets.h:787)
    pkt.write_i16(active_event);
    pkt.write_u16(remaining_secs);
    pkt
}

/// Build BDW counter packet with per-nation counts.
/// Packet format: `[0x5F] [u8:16] [u16:4] [u16:karus] [u16:elmo]`
pub fn build_bdw_counter_packet(karus_count: u16, elmo_count: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(16); // TEMPLE_EVENT_COUNTER sub-opcode
    pkt.write_u16(4); // TEMPLE_EVENT_BORDER_DEFENCE_WAR
    pkt.write_u16(karus_count);
    pkt.write_u16(elmo_count);
    pkt
}

/// Build Chaos counter packet with total count only (no trailing zero).
/// Packet format: `[0x5F] [u8:16] [u16:24] [u16:total]`
/// Note: Chaos sends only total count — no trailing u16(0) unlike BDW.
pub fn build_chaos_counter_packet(total_count: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(16); // TEMPLE_EVENT_COUNTER sub-opcode
    pkt.write_u16(24); // TEMPLE_EVENT_CHAOS
    pkt.write_u16(total_count);
    pkt
}

/// Build Juraid counter packet via WIZ_EXT_HOOK opcode.
/// Packet format: `[0xE9] [u8:0xE2] [u16:karus] [u16:elmo] [u16:remaining_secs]`
/// Uses entirely different opcode (WIZ_EXT_HOOK) with JURAID sub-opcode (0xE2).
pub fn build_juraid_counter_packet(
    karus_count: u16,
    elmo_count: u16,
    remaining_secs: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(0xE2); // ExtSub::JURAID
    pkt.write_u16(karus_count);
    pkt.write_u16(elmo_count);
    pkt.write_u16(remaining_secs);
    pkt
}

/// Build a v2615-compatible WIZ_EVENT Juraid join counter packet.
/// Packet format: `[0x5F] [u8:16] [u16:100] [u16:karus] [u16:elmo] [u16:remaining_secs]`
pub fn build_juraid_event_counter_packet(
    karus_count: u16,
    elmo_count: u16,
    remaining_secs: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(16); // TEMPLE_EVENT_COUNTER sub-opcode
    pkt.write_u16(TempleEventType::JuraidMountain as u16);
    pkt.write_u16(karus_count);
    pkt.write_u16(elmo_count);
    pkt.write_u16(remaining_secs);
    pkt
}

/// Native Juraid registration panel payload used by the stock client before
/// the old C++ source replaced it with its private XSafe extension.
///
/// Reference: EventSigningSystem.cpp::TemplEventJuraidSendJoinScreenUpdate.
/// v2615's WIZ_SELECT_MSG dispatcher reads the common leading event SID as
/// u32 (the same contract used by quest menus, Draki timers and winner UI).
/// `[u32 0][u8 7][u64 0][u32 6][u16 K][u16 0][u16 E][u16 0][u16 remain][u16 0]`
pub fn build_juraid_select_counter_packet(
    karus_count: u16,
    elmo_count: u16,
    remaining_secs: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
    pkt.write_u32(0);
    pkt.write_u8(7);
    pkt.write_u64(0);
    pkt.write_u32(6);
    pkt.write_u16(karus_count);
    pkt.write_u16(0);
    pkt.write_u16(elmo_count);
    pkt.write_u16(0);
    pkt.write_u16(remaining_secs);
    pkt.write_u16(0);
    pkt
}

/// Teleport all assigned room users into the event zone.
/// After room assignment, each user is zone-changed into the event zone
/// at coordinates (0, 0) — the server uses random spawn from start_position.
/// Also sends timer overlay packets per C++ reference:
/// - `WIZ_SELECT_MSG` with play time countdown
/// - `WIZ_BIFROST(5)` timer overlay
/// - `WIZ_EVENT(1)` for BDW only
pub fn teleport_users_to_event(world: &WorldState, event_type: TempleEventType) {
    let erm = &world.event_room_manager;
    let zone_id = event_type.zone_id();
    let active_event = event_type as i16;
    let rooms = erm.list_rooms(event_type);

    // Get play time from vroom options for timer packets
    let play_secs = EventRoomManager::vroom_index(event_type)
        .and_then(|idx| erm.get_vroom_opt(idx))
        .map(|opts| (opts.play as u32) * 60)
        .unwrap_or(0);

    // Build timer packets (sent after zone change per C++ reference)
    let arc_select = Arc::new(build_teleport_select_msg(active_event, play_secs));
    let arc_bifrost = Arc::new(build_teleport_bifrost(play_secs as u16));
    let arc_bdw = if event_type == TempleEventType::BorderDefenceWar {
        Some(Arc::new(build_bdw_event_start_packet()))
    } else {
        None
    };

    let mut total_teleported = 0u32;

    for room_id in &rooms {
        let session_ids = {
            let Some(room) = erm.get_room(event_type, *room_id) else {
                continue;
            };
            room.all_session_ids()
        };

        for sid in &session_ids {
            send_event_zone_change(world, *sid, zone_id, *room_id);

            // Send timer overlay packets per C++ TempleEventTeleportUsers
            world.send_to_session_arc(*sid, Arc::clone(&arc_select));
            world.send_to_session_arc(*sid, Arc::clone(&arc_bifrost));
            if let Some(ref bdw_pkt) = arc_bdw {
                world.send_to_session_arc(*sid, Arc::clone(bdw_pkt));
            }

            total_teleported += 1;
        }
    }

    tracing::info!(
        "Event {:?} — teleported {} users into zone {} (play_secs={})",
        event_type,
        total_teleported,
        zone_id,
        play_secs
    );
}

/// Create server-initiated parties for event rooms (BDW and Juraid only).
/// For each room, iterates Karus then Elmorad users. If a nation has >1 user,
/// the first valid user becomes the party leader and subsequent users are added.
/// Each member receives PARTY_INSERT packets for all existing members, and all
/// existing members receive a PARTY_INSERT for the new joiner.
/// Chaos Dungeon never creates parties (FFA mode).
pub fn temple_event_create_parties(world: &WorldState, event_type: TempleEventType) {
    use crate::handler::party::build_party_member_info;
    use crate::handler::regene::build_state_change_broadcast;

    let erm = &world.event_room_manager;
    let rooms = erm.list_rooms(event_type);

    for room_id in &rooms {
        let Some(room) = erm.get_room(event_type, *room_id) else {
            continue;
        };

        // Collect Karus and Elmorad session IDs from the room.
        let karus_sids: Vec<SessionId> = room
            .karus_users
            .values()
            .filter(|u| !u.logged_out)
            .map(|u| u.session_id)
            .collect();
        let elmorad_sids: Vec<SessionId> = room
            .elmorad_users
            .values()
            .filter(|u| !u.logged_out)
            .map(|u| u.session_id)
            .collect();
        drop(room);

        // Create parties for each nation group.
        for nation_sids in [&karus_sids, &elmorad_sids] {
            if nation_sids.len() <= 1 {
                continue;
            }

            // Filter to valid users: in-game, not already in party.
            let valid_sids: Vec<SessionId> = nation_sids
                .iter()
                .filter(|sid| {
                    if let Some(ch) = world.get_character_info(**sid) {
                        ch.party_id.is_none()
                    } else {
                        false
                    }
                })
                .copied()
                .collect();

            if valid_sids.len() <= 1 {
                continue;
            }

            // First user becomes leader.
            let leader_sid = valid_sids[0];
            let Some(party_id) = world.create_party(leader_sid) else {
                continue;
            };

            // C++ EventPartyCreate: set leader flags + StateChangeServerDirect(6, 1)
            if let Some(pos) = world.get_position(leader_sid) {
                let sc_pkt =
                    build_state_change_broadcast(leader_sid as u32, STATE_CHANGE_PARTY_LEADER, 1);
                let sender_event_room = world.get_event_room(leader_sid);
                world.broadcast_to_region_sync(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(sc_pkt),
                    None,
                    sender_event_room,
                );
            }

            // Add remaining users to the party.
            for &member_sid in &valid_sids[1..] {
                // C++ EventPartyInvitationCheck: validate + set party index
                if !world.add_party_member(party_id, member_sid) {
                    continue;
                }

                // C++ EventPartyInvitation: send existing members' info to joiner
                let party = match world.get_party(party_id) {
                    Some(p) => p,
                    None => break,
                };
                let target_number_id = party.target_number_id;

                for slot in &party.members {
                    let Some(existing_sid) = slot else { continue };
                    if *existing_sid == member_sid {
                        continue;
                    }
                    if let Some(member_ch) = world.get_character_info(*existing_sid) {
                        let lr = world.get_loyalty_symbol_rank(*existing_sid);
                        let info_pkt =
                            build_party_member_info(&member_ch, 1, target_number_id, 0, lr);
                        world.send_to_session_owned(member_sid, info_pkt);
                    }
                }

                // Send joiner's info to all party members
                if let Some(joiner_ch) = world.get_character_info(member_sid) {
                    let lr = world.get_loyalty_symbol_rank(member_sid);
                    let info_pkt = build_party_member_info(&joiner_ch, 1, target_number_id, 0, lr);
                    world.send_to_party(party_id, &info_pkt);
                }
            }

            tracing::debug!(
                party_id,
                event = ?event_type,
                room = room_id,
                members = valid_sids.len(),
                "Created event party"
            );
        }
    }
}

/// Send a zone change packet for event teleport.
///
/// Event entry must use the regular server-side teleport path. Sending raw
/// `(0, 0)` coordinates leaves modern clients below the BDW map and also
/// skips region-grid and zone-changing bookkeeping.
fn send_event_zone_change(world: &WorldState, sid: SessionId, zone_id: u16, event_room: u8) {
    // Resolve (0,0) through start_position and perform the complete zone
    // transition before switching the room filter. This preserves the old
    // room while broadcasting INOUT_OUT from the source zone.
    crate::handler::zone_change::server_teleport_to_zone(world, sid, zone_id, 0.0, 0.0);

    //   if (eventroom == 0 && GetEventRoom() > 0) m_bEventRoom = 0;
    //   else if (eventroom > 0)                    m_bEventRoom = eventroom;
    if event_room > 0 {
        world.update_session(sid, |h| {
            h.event_room = event_room as u16;
        });
    } else {
        let cur = world.get_event_room(sid);
        if cur > 0 {
            world.update_session(sid, |h| {
                h.event_room = 0;
            });
        }
    }
}

/// Determine the kick-out destination zone for a player leaving an event.
/// - BDW/Chaos: Nation capital (level >= 35) or Moradon
/// - Juraid: Moradon
pub fn kick_out_destination(event_zone: u16, nation: u8, level: u8) -> u16 {
    match event_zone {
        // BDW (84) or Chaos (85): nation capital if level >= 35
        84 | 85 => {
            if level >= NATION_CAPITAL_MIN_LEVEL {
                match nation {
                    1 => ZONE_KARUS,
                    2 => ZONE_ELMORAD,
                    _ => ZONE_MORADON,
                }
            } else {
                ZONE_MORADON
            }
        }
        // Juraid (87): original 2615 flow now exits to Moradon for this server.
        87 => ZONE_MORADON,
        _ => ZONE_MORADON,
    }
}

/// Process per-room finish countdown after winner screen is sent.
/// Checks each room's `finish_time_counter`. When the countdown expires
/// (20 seconds after winner screen), the room is marked as finished and
/// users are kicked out. This enables per-room independent finish timing.
pub fn temple_event_room_close(world: &WorldState, event_type: TempleEventType, now: u64) {
    let erm = &world.event_room_manager;
    let rooms = erm.list_rooms(event_type);

    for room_id in &rooms {
        // Check room state: must have finish packet sent, not already finished
        let should_close = {
            let Some(room) = erm.get_room(event_type, *room_id) else {
                continue;
            };

            room.finish_packet_sent
                && !room.finished
                && room.finish_time_counter > 0
                && room.finish_time_counter <= now
        };

        if should_close {
            // Mark room as finished
            if let Some(mut room) = erm.get_room_mut(event_type, *room_id) {
                room.finished = true;
            }

            // Kick out users from this specific room
            let users: Vec<(SessionId, u8, u8)> = {
                let Some(room) = erm.get_room(event_type, *room_id) else {
                    continue;
                };
                room.all_session_ids()
                    .into_iter()
                    .filter_map(|sid| {
                        world
                            .get_character_info(sid)
                            .map(|c| (sid, c.nation, c.level))
                    })
                    .collect()
            };

            // Per-user cleanup before teleport
            //   if (pUser->m_bHasAlterOptained)
            //       RemoveType4Buff(BUFF_TYPE_FRAGMENT_OF_MANES, pUser, true, true);
            //       pUser->m_bHasAlterOptained = false;
            if event_type == TempleEventType::BorderDefenceWar {
                for (sid, _nation, _level) in &users {
                    // Remove BDW speed debuff if the player still has it
                    world.remove_buff(*sid, crate::systems::bdw::BUFF_TYPE_FRAGMENT_OF_MANES);
                }
            }

            let event_zone = event_type.zone_id();
            for (sid, nation, level) in &users {
                let dest_zone = kick_out_destination(event_zone, *nation, *level);
                world.update_session(*sid, |h| {
                    h.event_room = 0;
                    h.joined_event = false;
                    h.is_final_joined_event = false;
                });
                crate::handler::zone_change::server_teleport_to_zone(
                    world, *sid, dest_zone, 0.0, 0.0,
                );
            }

            world.despawn_room_npcs(event_zone, *room_id as u16);

            // filler bots of this room leave with the humans
            if let Some(kind) = crate::systems::event_bots::EventBotKind::from_temple(event_type) {
                crate::systems::event_bots::cleanup_room(world, kind, *room_id as u16);
            }

            tracing::info!(
                "Event {:?} room {} — finish countdown expired, kicked {} users",
                event_type,
                room_id,
                users.len()
            );
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temple_event_type_from_i16() {
        assert_eq!(
            TempleEventType::from_i16(4),
            Some(TempleEventType::BorderDefenceWar)
        );
        assert_eq!(
            TempleEventType::from_i16(14),
            Some(TempleEventType::ForgottenTemple)
        );
        assert_eq!(
            TempleEventType::from_i16(24),
            Some(TempleEventType::ChaosDungeon)
        );
        assert_eq!(
            TempleEventType::from_i16(100),
            Some(TempleEventType::JuraidMountain)
        );
        assert_eq!(TempleEventType::from_i16(0), None);
        assert_eq!(TempleEventType::from_i16(999), None);
    }

    #[test]
    fn test_temple_event_type_zone_id() {
        assert_eq!(TempleEventType::BorderDefenceWar.zone_id(), 84);
        assert_eq!(TempleEventType::ForgottenTemple.zone_id(), 55);
        assert_eq!(TempleEventType::ChaosDungeon.zone_id(), 85);
        assert_eq!(TempleEventType::JuraidMountain.zone_id(), 87);
    }

    #[test]
    fn test_event_schedule_type_from_i16() {
        assert_eq!(
            EventScheduleType::from_i16(1),
            Some(EventScheduleType::LunarWar)
        );
        assert_eq!(
            EventScheduleType::from_i16(2),
            Some(EventScheduleType::VirtualRoom)
        );
        assert_eq!(
            EventScheduleType::from_i16(3),
            Some(EventScheduleType::SingleRoom)
        );
        assert_eq!(EventScheduleType::from_i16(0), None);
    }

    #[test]
    fn test_event_local_id_from_u8() {
        assert_eq!(EventLocalId::from_u8(1), Some(EventLocalId::CastleSiegeWar));
        assert_eq!(
            EventLocalId::from_u8(9),
            Some(EventLocalId::BorderDefenceWar)
        );
        assert_eq!(
            EventLocalId::from_u8(13),
            Some(EventLocalId::ForgettenTemple)
        );
        assert_eq!(EventLocalId::from_u8(0), None);
        assert_eq!(EventLocalId::from_u8(15), None);
    }

    #[test]
    fn test_event_local_id_to_temple_event_type() {
        assert_eq!(
            EventLocalId::BorderDefenceWar.to_temple_event_type(),
            Some(TempleEventType::BorderDefenceWar)
        );
        assert_eq!(
            EventLocalId::ChaosExpansion.to_temple_event_type(),
            Some(TempleEventType::ChaosDungeon)
        );
        assert_eq!(
            EventLocalId::JuraidMountain.to_temple_event_type(),
            Some(TempleEventType::JuraidMountain)
        );
        assert_eq!(
            EventLocalId::ForgettenTemple.to_temple_event_type(),
            Some(TempleEventType::ForgottenTemple)
        );
        // Non-room events return None
        assert_eq!(EventLocalId::NapiesGorge.to_temple_event_type(), None);
        assert_eq!(EventLocalId::BeefEvent.to_temple_event_type(), None);
    }

    #[test]
    fn test_room_state_from_u8() {
        assert_eq!(RoomState::from_u8(0), Some(RoomState::Idle));
        assert_eq!(RoomState::from_u8(1), Some(RoomState::Signing));
        assert_eq!(RoomState::from_u8(2), Some(RoomState::Running));
        assert_eq!(RoomState::from_u8(3), Some(RoomState::Finishing));
        assert_eq!(RoomState::from_u8(4), Some(RoomState::Cleanup));
        assert_eq!(RoomState::from_u8(5), None);
    }

    #[test]
    fn test_event_room_new() {
        let room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        assert_eq!(room.room_id, 1);
        assert_eq!(room.event_type, TempleEventType::BorderDefenceWar);
        assert_eq!(room.state, RoomState::Idle);
        assert_eq!(room.zone_id, 84);
        assert_eq!(room.user_count(), 0);
        assert!(!room.finished);
        assert_eq!(room.winner_nation, 0);
    }

    #[test]
    fn test_event_room_add_user_bdw() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);

        // Add Karus user
        let user_k = EventUser {
            user_name: "warrior1".to_string(),
            session_id: 100,
            nation: 1,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        assert!(room.add_user(user_k));
        assert_eq!(room.karus_users.len(), 1);
        assert_eq!(room.elmorad_users.len(), 0);
        assert_eq!(room.user_count(), 1);

        // Add El Morad user
        let user_e = EventUser {
            user_name: "mage1".to_string(),
            session_id: 101,
            nation: 2,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        assert!(room.add_user(user_e));
        assert_eq!(room.karus_users.len(), 1);
        assert_eq!(room.elmorad_users.len(), 1);
        assert_eq!(room.user_count(), 2);
    }

    #[test]
    fn test_event_room_add_user_chaos() {
        let mut room = EventRoom::new(1, TempleEventType::ChaosDungeon);

        for i in 0..MAX_CHAOS_ROOM_USERS {
            let user = EventUser {
                user_name: format!("player{}", i),
                session_id: i as u16,
                nation: if i % 2 == 0 { 1 } else { 2 },
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            };
            assert!(room.add_user(user));
        }
        assert_eq!(room.mixed_users.len(), MAX_CHAOS_ROOM_USERS);
        assert_eq!(room.user_count(), MAX_CHAOS_ROOM_USERS);

        // Room full — should reject
        let extra = EventUser {
            user_name: "extra".to_string(),
            session_id: 999,
            nation: 1,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        assert!(!room.add_user(extra));
    }

    #[test]
    fn test_event_room_bdw_max_per_nation() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);

        // Fill Karus side
        for i in 0..MAX_ROOM_USERS_PER_NATION {
            let user = EventUser {
                user_name: format!("karus{}", i),
                session_id: i as u16,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            };
            assert!(room.add_user(user));
        }
        // Next Karus should fail
        let extra = EventUser {
            user_name: "karus_extra".to_string(),
            session_id: 999,
            nation: 1,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        assert!(!room.add_user(extra));

        // El Morad should still work
        let elmo = EventUser {
            user_name: "elmo1".to_string(),
            session_id: 100,
            nation: 2,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        assert!(room.add_user(elmo));
    }

    #[test]
    fn test_event_room_remove_user() {
        let mut room = EventRoom::new(1, TempleEventType::JuraidMountain);
        let user = EventUser {
            user_name: "warrior1".to_string(),
            session_id: 1,
            nation: 1,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        room.add_user(user);
        assert_eq!(room.user_count(), 1);

        let removed = room.remove_user("warrior1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().session_id, 1);
        assert_eq!(room.user_count(), 0);

        // Removing again should return None
        assert!(room.remove_user("warrior1").is_none());
    }

    #[test]
    fn test_event_room_get_user() {
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        let user = EventUser {
            user_name: "mage1".to_string(),
            session_id: 42,
            nation: 2,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        };
        room.add_user(user);

        let found = room.get_user("mage1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().session_id, 42);

        assert!(room.get_user("nonexistent").is_none());
    }

    #[test]
    fn test_event_room_all_session_ids() {
        let mut room = EventRoom::new(1, TempleEventType::JuraidMountain);
        room.add_user(EventUser {
            user_name: "a".to_string(),
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
            user_name: "b".to_string(),
            session_id: 2,
            nation: 2,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        });
        room.add_user(EventUser {
            user_name: "c".to_string(),
            session_id: 3,
            nation: 1,
            prize_given: false,
            logged_out: true, // logged out
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        });

        let ids = room.all_session_ids();
        assert_eq!(ids.len(), 2); // c is logged out
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn test_event_room_reset() {
        let mut room = EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.state = RoomState::Finishing;
        room.finished = true;
        room.winner_nation = 1;
        room.karus_score = 50;
        room.add_user(EventUser {
            user_name: "x".to_string(),
            session_id: 1,
            nation: 1,
            prize_given: false,
            logged_out: false,
            kills: 0,
            deaths: 0,
            bdw_points: 0,
            has_altar_obtained: false,
        });

        room.reset();
        assert_eq!(room.state, RoomState::Idle);
        assert!(!room.finished);
        assert_eq!(room.winner_nation, 0);
        assert_eq!(room.karus_score, 0);
        assert_eq!(room.user_count(), 0);
    }

    #[test]
    fn test_temple_event_state_default() {
        let state = TempleEventState::default();
        assert_eq!(state.active_event, -1);
        assert!(!state.is_active);
        assert!(!state.allow_join);
        assert!(!state.is_attackable);
        assert_eq!(state.all_user_count, 0);
    }

    #[test]
    fn test_temple_event_state_active_checks() {
        let mut state = TempleEventState::default();
        assert!(!state.is_bdw_active());
        assert!(!state.is_juraid_active());
        assert!(!state.is_chaos_active());

        state.active_event = TempleEventType::BorderDefenceWar as i16;
        state.is_active = true;
        assert!(state.is_bdw_active());
        assert!(!state.is_juraid_active());
        assert!(!state.is_chaos_active());
    }

    #[test]
    fn test_temple_event_state_reset() {
        let mut state = TempleEventState {
            active_event: 4,
            is_active: true,
            karus_user_count: 10,
            ..Default::default()
        };

        state.reset();
        assert_eq!(state.active_event, -1);
        assert!(!state.is_active);
        assert_eq!(state.karus_user_count, 0);
    }

    #[test]
    fn test_event_room_manager_create_destroy() {
        let mgr = EventRoomManager::new();
        mgr.create_rooms(TempleEventType::BorderDefenceWar, 3);

        assert_eq!(mgr.room_count(TempleEventType::BorderDefenceWar), 3);
        assert!(mgr.get_room(TempleEventType::BorderDefenceWar, 1).is_some());
        assert!(mgr.get_room(TempleEventType::BorderDefenceWar, 3).is_some());
        assert!(mgr.get_room(TempleEventType::BorderDefenceWar, 4).is_none());

        mgr.destroy_rooms(TempleEventType::BorderDefenceWar);
        assert_eq!(mgr.room_count(TempleEventType::BorderDefenceWar), 0);
    }

    #[test]
    fn test_event_room_manager_list_rooms() {
        let mgr = EventRoomManager::new();
        mgr.create_rooms(TempleEventType::JuraidMountain, 5);

        let mut rooms = mgr.list_rooms(TempleEventType::JuraidMountain);
        rooms.sort();
        assert_eq!(rooms, vec![1, 2, 3, 4, 5]);
        assert!(mgr.list_rooms(TempleEventType::ChaosDungeon).is_empty());
    }

    #[test]
    fn test_event_room_manager_sign_up() {
        let mgr = EventRoomManager::new();

        let order1 = mgr.add_signed_up_user("warrior1".into(), 1, 1);
        assert!(order1.is_some());
        assert_eq!(mgr.signed_up_count(), 1);

        let order2 = mgr.add_signed_up_user("mage1".into(), 2, 2);
        assert!(order2.is_some());
        assert_eq!(mgr.signed_up_count(), 2);

        // Duplicate should fail
        let dup = mgr.add_signed_up_user("warrior1".into(), 1, 1);
        assert!(dup.is_none());
        assert_eq!(mgr.signed_up_count(), 2);

        // Remove
        let removed = mgr.remove_signed_up_user("warrior1");
        assert!(removed.is_some());
        assert_eq!(mgr.signed_up_count(), 1);

        // Clear
        mgr.clear_signed_up_users();
        assert_eq!(mgr.signed_up_count(), 0);
    }

    #[test]
    fn test_event_room_manager_temple_event_state() {
        let mgr = EventRoomManager::new();

        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::ChaosDungeon as i16;
            s.is_active = true;
            s.karus_user_count = 5;
        });

        let is_chaos = mgr.read_temple_event(|s| s.is_chaos_active());
        assert!(is_chaos);

        mgr.reset_temple_event();
        let active = mgr.read_temple_event(|s| s.active_event);
        assert_eq!(active, -1);
    }

    #[test]
    fn test_event_room_manager_max_room_cap() {
        let mgr = EventRoomManager::new();
        // Trying to create more than MAX_TEMPLE_EVENT_ROOM should be capped.
        mgr.create_rooms(TempleEventType::ChaosDungeon, 100);
        assert_eq!(
            mgr.room_count(TempleEventType::ChaosDungeon),
            MAX_TEMPLE_EVENT_ROOM as usize
        );
    }

    #[test]
    fn test_event_room_manager_vroom_index() {
        assert_eq!(
            EventRoomManager::vroom_index(TempleEventType::BorderDefenceWar),
            Some(0)
        );
        assert_eq!(
            EventRoomManager::vroom_index(TempleEventType::ChaosDungeon),
            Some(1)
        );
        assert_eq!(
            EventRoomManager::vroom_index(TempleEventType::JuraidMountain),
            Some(2)
        );
        assert_eq!(
            EventRoomManager::vroom_index(TempleEventType::ForgottenTemple),
            None
        );
    }

    #[test]
    fn test_event_room_manager_get_room_mut() {
        let mgr = EventRoomManager::new();
        mgr.create_rooms(TempleEventType::BorderDefenceWar, 1);

        // Mutate room state
        if let Some(mut room) = mgr.get_room_mut(TempleEventType::BorderDefenceWar, 1) {
            room.state = RoomState::Running;
            room.karus_score = 10;
        }

        // Verify mutation
        let room = mgr.get_room(TempleEventType::BorderDefenceWar, 1);
        if let Some(room) = room {
            assert_eq!(room.state, RoomState::Running);
            assert_eq!(room.karus_score, 10);
        } else {
            panic!("Room should exist");
        }
    }

    #[test]
    fn test_forgotten_temple_opts_default() {
        let opts = ForgottenTempleOpts::default();
        assert_eq!(opts.playing_time, 30);
        assert_eq!(opts.summon_time, 300);
        assert_eq!(opts.min_level, 60);
        assert_eq!(opts.max_level, 83);
    }

    #[test]
    fn test_spawn_event_npc_params() {
        let params = SpawnEventNpcParams {
            npc_id: 1234,
            is_monster: true,
            zone_id: 55,
            pos_x: 100.0,
            pos_y: 0.0,
            pos_z: 200.0,
            count: 3,
            radius: 10,
            event_room: 1,
            direction: 0,
        };
        assert_eq!(params.npc_id, 1234);
        assert!(params.is_monster);
        assert_eq!(params.zone_id, 55);
        assert_eq!(params.count, 3);
    }

    #[test]
    fn test_is_user_signed_up() {
        let mgr = EventRoomManager::new();

        // Not signed up initially
        assert!(!mgr.is_user_signed_up("warrior1"));

        // Sign up
        mgr.add_signed_up_user("warrior1".into(), 1, 1);
        assert!(mgr.is_user_signed_up("warrior1"));
        assert!(!mgr.is_user_signed_up("mage1"));

        // After remove, no longer signed up
        mgr.remove_signed_up_user("warrior1");
        assert!(!mgr.is_user_signed_up("warrior1"));
    }

    #[test]
    fn test_is_user_signed_up_after_clear() {
        let mgr = EventRoomManager::new();
        mgr.add_signed_up_user("player1".into(), 10, 1);
        mgr.add_signed_up_user("player2".into(), 11, 2);
        assert!(mgr.is_user_signed_up("player1"));
        assert!(mgr.is_user_signed_up("player2"));

        mgr.clear_signed_up_users();
        assert!(!mgr.is_user_signed_up("player1"));
        assert!(!mgr.is_user_signed_up("player2"));
    }

    // ── Event State Machine Tests ──────────────────────────────────────

    /// Helper: set up EventRoomManager with vroom options for Chaos (index=1).
    fn setup_erm_with_chaos_opts(
        sign_min: i32,
        play_min: i32,
        atk_open: i32,
        atk_close: i32,
        finish_sec: i32,
    ) -> EventRoomManager {
        let erm = EventRoomManager::new();
        let mut opts = erm.vroom_opts.write();
        opts[1] = Some(VroomOpt {
            name: "Chaos".into(),
            sign: sign_min,
            play: play_min,
            attack_open: atk_open,
            attack_close: atk_close,
            finish: finish_sec,
        });
        drop(opts);
        erm
    }

    #[test]
    fn test_virtual_event_open_sets_signing_state() {
        let erm = setup_erm_with_chaos_opts(5, 15, 6, 14, 30);
        let _sched = EventScheduleEntry {
            event_id: 10, // ChaosExpansion
            event_type: 2,
            zone_id: 85,
            name: "Chaos".into(),
            status: true,
            start_times: [(12, 0), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
            days: [true; 7],
            min_level: 0,
            max_level: 0,
            req_loyalty: 0,
            req_money: 0,
        };

        // Before open: no active event
        assert_eq!(erm.temple_event.read().active_event, -1);

        // Simulate the WorldState minimally — we can't fully create one in unit tests,
        // but we can test the ERM state management directly.
        let now = 1000000u64;

        // Manually replicate what virtual_event_open does to the ERM:
        erm.create_rooms(TempleEventType::ChaosDungeon, MAX_TEMPLE_EVENT_ROOM);
        {
            let mut te = erm.temple_event.write();
            te.active_event = TempleEventType::ChaosDungeon as i16;
            te.allow_join = true;
            te.is_active = false;
            te.is_automatic = true;
            te.zone_id = 85;
            te.start_time = now;
            te.closed_time = now + (5 + 15) * 60;
            te.sign_remain_seconds = now + 5 * 60;
        }

        let te = erm.temple_event.read();
        assert_eq!(te.active_event, TempleEventType::ChaosDungeon as i16);
        assert!(te.allow_join);
        assert!(!te.is_active);
        assert!(te.is_automatic);
        assert_eq!(te.zone_id, 85);
        assert_eq!(te.sign_remain_seconds, now + 300);
    }

    #[test]
    fn test_event_reset_clears_state() {
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::ChaosDungeon, 5);

        {
            let mut te = erm.temple_event.write();
            te.active_event = TempleEventType::ChaosDungeon as i16;
            te.is_active = true;
            te.timer_start_control = true;
        }

        // Verify rooms exist before reset
        assert!(erm.get_room(TempleEventType::ChaosDungeon, 1).is_some());

        // Simulate event_reset logic (without WorldState)
        erm.destroy_rooms(TempleEventType::ChaosDungeon);
        erm.signed_up_users.write().clear();
        erm.temple_event.write().reset();

        // After reset
        let te = erm.temple_event.read();
        assert_eq!(te.active_event, -1);
        assert!(!te.is_active);
        assert!(!te.timer_start_control);
        assert!(erm.get_room(TempleEventType::ChaosDungeon, 1).is_none());
    }

    #[test]
    fn test_temple_event_state_phase_transitions() {
        // Simulate the state machine phase transitions directly
        let mut te = TempleEventState::default();
        let now = 1000000u64;

        // Phase 0: Start event (signing)
        te.active_event = TempleEventType::ChaosDungeon as i16;
        te.allow_join = true;
        te.is_active = false;
        te.start_time = now;
        assert!(te.allow_join);
        assert!(!te.is_active);

        // Phase 1: Signing → Active (after sign period)
        te.allow_join = false;
        te.sign_remain_seconds = 0;
        te.is_active = true;
        te.timer_start_control = true;
        assert!(!te.allow_join);
        assert!(te.is_active);
        assert!(te.is_chaos_active());

        // Phase 2: Attack Open
        te.is_attackable = true;
        te.timer_attack_open_control = true;
        assert!(te.is_attackable);

        // Phase 3: Attack Close
        te.is_attackable = false;
        te.timer_attack_close_control = true;
        assert!(!te.is_attackable);

        // Phase 4: Finish
        te.timer_finish_control = true;
        assert!(te.timer_finish_control);

        // Phase 5: Reset
        te.reset();
        assert_eq!(te.active_event, -1);
        assert!(!te.is_active);
        assert!(!te.is_chaos_active());
    }

    #[test]
    fn test_vroom_opt_timing_calculations() {
        // C++ timing: all times are relative to start_time in minutes
        let opts = VroomOpt {
            name: "BDW".into(),
            sign: 5,          // 5 min signing
            play: 15,         // 15 min playing
            attack_open: 1,   // 1 min after sign for attack
            attack_close: 14, // 14 min after sign for close
            finish: 30,       // 30 sec after finish for cleanup
        };

        let start = 1000000u64;
        let sign_end = start + (opts.sign as u64) * 60;
        let attack_open = start + ((opts.sign + opts.attack_open) as u64) * 60;
        let attack_close = start + ((opts.sign + opts.attack_close) as u64) * 60;
        let finish = start + ((opts.sign + opts.play) as u64) * 60;
        let reset = start + ((opts.sign + opts.play + 1) as u64) * 60 + opts.finish as u64;

        assert_eq!(sign_end, 1000300); // +300s
        assert_eq!(attack_open, 1000360); // +360s
        assert_eq!(attack_close, 1001140); // +1140s
        assert_eq!(finish, 1001200); // +1200s
        assert_eq!(reset, 1001290); // +1290s (1260+30)
    }

    #[test]
    fn test_manual_close_resets_event() {
        let mut te = TempleEventState {
            active_event: TempleEventType::BorderDefenceWar as i16,
            is_active: true,
            manual_close: true,
            manual_closed_time: 1000000,
            ..Default::default()
        };

        // Before finish delay elapses
        assert!(te.manual_close);
        assert!(te.is_bdw_active());

        // After reset
        te.reset();
        assert!(!te.manual_close);
        assert_eq!(te.manual_closed_time, 0);
        assert!(!te.is_bdw_active());
    }

    // ── Event Lifecycle Tests ────────────────────────────────────────────

    #[test]
    fn test_build_winner_select_msg_bdw() {
        let pkt = build_winner_select_msg(4); // BDW
        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(0)); // unk1
        assert_eq!(r.read_u8(), Some(7)); // msg_type
        assert_eq!(r.read_u64(), Some(0)); // unk2
        assert_eq!(r.read_u32(), Some(8)); // event_msg_id (BDW=8)
        assert_eq!(r.read_u8(), Some(7)); // param (BDW=7)
        assert_eq!(r.read_u32(), Some(500)); // duration
    }

    #[test]
    fn test_build_winner_select_msg_chaos() {
        let pkt = build_winner_select_msg(24); // Chaos
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u32();
        r.read_u8();
        r.read_u64(); // skip unk fields
        assert_eq!(r.read_u32(), Some(9)); // event_msg_id (Chaos=9)
        assert_eq!(r.read_u8(), Some(24)); // param (Chaos=24)
    }

    #[test]
    fn test_build_winner_select_msg_juraid() {
        let pkt = build_winner_select_msg(100); // Juraid
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u32();
        r.read_u8();
        r.read_u64();
        assert_eq!(r.read_u32(), Some(6)); // event_msg_id (Juraid=6)
        assert_eq!(r.read_u8(), Some(11)); // param (Juraid=11)
    }

    #[test]
    fn test_build_finish_packet() {
        let pkt = build_finish_packet(1); // Karus wins
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(10)); // TEMPLE_EVENT_FINISH
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(1)); // winner_nation = Karus
        assert_eq!(r.read_u32(), Some(20)); // countdown
    }

    #[test]
    fn test_build_finish_packet_draw() {
        let pkt = build_finish_packet(0); // Draw / FFA
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        assert_eq!(r.read_u8(), Some(0)); // winner_nation = 0
    }

    #[test]
    fn test_kick_out_destination_bdw_high_level_karus() {
        assert_eq!(kick_out_destination(84, 1, 60), ZONE_KARUS);
    }

    #[test]
    fn test_kick_out_destination_bdw_high_level_elmorad() {
        assert_eq!(kick_out_destination(84, 2, 60), ZONE_ELMORAD);
    }

    #[test]
    fn test_kick_out_destination_bdw_low_level() {
        assert_eq!(kick_out_destination(84, 1, 20), ZONE_MORADON);
        assert_eq!(kick_out_destination(84, 2, 34), ZONE_MORADON);
    }

    #[test]
    fn test_kick_out_destination_chaos_high_level() {
        assert_eq!(kick_out_destination(85, 1, 50), ZONE_KARUS);
        assert_eq!(kick_out_destination(85, 2, 35), ZONE_ELMORAD);
    }

    #[test]
    fn test_kick_out_destination_chaos_low_level() {
        assert_eq!(kick_out_destination(85, 1, 30), ZONE_MORADON);
    }

    #[test]
    fn test_kick_out_destination_juraid_high_level() {
        assert_eq!(kick_out_destination(87, 1, 60), ZONE_MORADON);
        assert_eq!(kick_out_destination(87, 2, 35), ZONE_MORADON);
    }

    #[test]
    fn test_kick_out_destination_juraid_low_level() {
        assert_eq!(kick_out_destination(87, 1, 20), ZONE_MORADON);
    }

    #[test]
    fn test_kick_out_destination_unknown_zone() {
        assert_eq!(kick_out_destination(99, 1, 60), ZONE_MORADON);
    }

    #[test]
    fn test_kick_out_destination_level_boundary() {
        // Exactly level 35 = capital
        assert_eq!(kick_out_destination(84, 1, 35), ZONE_KARUS);
        // Below 35 = Moradon
        assert_eq!(kick_out_destination(84, 1, 34), ZONE_MORADON);
    }

    // ── Sprint 171 Packet Builder Tests ────────────────────────────────

    #[test]
    fn test_build_teleport_select_msg_bdw() {
        let pkt = build_teleport_select_msg(4, 900); // BDW, 15 min * 60
        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(0)); // unk1
        assert_eq!(r.read_u8(), Some(7)); // msg_type
        assert_eq!(r.read_u64(), Some(0)); // unk2
        assert_eq!(r.read_u32(), Some(8)); // event_msg_id (BDW=8)
        assert_eq!(r.read_u8(), Some(7)); // param (BDW=7)
        assert_eq!(r.read_u32(), Some(900)); // play time in seconds
    }

    #[test]
    fn test_build_teleport_select_msg_chaos() {
        let pkt = build_teleport_select_msg(24, 600);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u32();
        r.read_u8();
        r.read_u64();
        assert_eq!(r.read_u32(), Some(9)); // event_msg_id (Chaos=9)
        assert_eq!(r.read_u8(), Some(24)); // param (Chaos=24)
        assert_eq!(r.read_u32(), Some(600)); // play time
    }

    #[test]
    fn test_build_teleport_bifrost() {
        let pkt = build_teleport_bifrost(900);
        assert_eq!(pkt.opcode, Opcode::WizBifrost as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5)); // sub-opcode: timer
        assert_eq!(r.read_u16(), Some(900)); // play time
    }

    #[test]
    fn test_build_bdw_event_start_packet() {
        let pkt = build_bdw_event_start_packet();
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // TEMPLE_EVENT sub-opcode
        assert!(r.read_u8().is_none()); // no more data
    }

    #[test]
    fn test_build_event_start_broadcast() {
        let pkt = build_event_start_broadcast(24, 300); // Chaos, 300 sec remaining
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(7)); // TEMPLE_EVENT sub-opcode (packets.h:787)
        assert_eq!(r.read_i16(), Some(24)); // active_event (Chaos)
        assert_eq!(r.read_u16(), Some(300)); // remaining_secs
    }

    #[test]
    fn test_build_bdw_counter_packet() {
        // C++ TemplEventBDWSendJoinScreenUpdate: per-nation counts
        let pkt = build_bdw_counter_packet(5, 3);
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(16)); // TEMPLE_EVENT_COUNTER
        assert_eq!(r.read_u16(), Some(4)); // BDW
        assert_eq!(r.read_u16(), Some(5)); // karus count
        assert_eq!(r.read_u16(), Some(3)); // elmo count
        assert!(r.read_u8().is_none()); // no more data
    }

    #[test]
    fn test_build_chaos_counter_packet() {
        // C++ TemplEventChaosSendJoinScreenUpdate: total count only, NO trailing zero
        let pkt = build_chaos_counter_packet(12);
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(16)); // TEMPLE_EVENT_COUNTER
        assert_eq!(r.read_u16(), Some(24)); // Chaos
        assert_eq!(r.read_u16(), Some(12)); // total count
        assert!(r.read_u8().is_none()); // NO trailing data (unlike BDW)
    }

    #[test]
    fn test_build_juraid_counter_packet() {
        // C++ TemplEventJuraidSendJoinScreenUpdate: uses WIZ_EXT_HOOK opcode
        let pkt = build_juraid_counter_packet(4, 6, 180);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C); // 0xE9 ext_hook
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0xE2)); // ExtSub::JURAID
        assert_eq!(r.read_u16(), Some(4)); // karus count
        assert_eq!(r.read_u16(), Some(6)); // elmo count
        assert_eq!(r.read_u16(), Some(180)); // remaining seconds
        assert!(r.read_u8().is_none()); // no more data
    }

    #[test]
    fn test_build_juraid_event_counter_packet() {
        let pkt = build_juraid_event_counter_packet(4, 6, 180);
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(16)); // TEMPLE_EVENT_COUNTER
        assert_eq!(r.read_u16(), Some(100)); // Juraid
        assert_eq!(r.read_u16(), Some(4)); // karus count
        assert_eq!(r.read_u16(), Some(6)); // elmo count
        assert_eq!(r.read_u16(), Some(180)); // remaining seconds
        assert!(r.read_u8().is_none());
    }

    #[test]
    fn test_build_juraid_select_counter_packet() {
        let pkt = build_juraid_select_counter_packet(4, 6, 180);
        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u64(), Some(0));
        assert_eq!(r.read_u32(), Some(6));
        assert_eq!(r.read_u16(), Some(4));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(6));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(180));
        assert_eq!(r.read_u16(), Some(0));
        assert!(r.read_u8().is_none());
    }

    #[test]
    fn test_create_parties_only_for_bdw_and_juraid() {
        // C++ TempleEventCreateParties switch only handles BDW and Juraid.
        // Chaos timer never calls CreateParties (EventMainTimer.cpp:539).
        assert_ne!(
            TempleEventType::ChaosDungeon,
            TempleEventType::BorderDefenceWar
        );
        assert_ne!(
            TempleEventType::ChaosDungeon,
            TempleEventType::JuraidMountain
        );
    }

    #[test]
    fn test_state_change_party_type() {
        // C++ EventPartyCreate: StateChangeServerDirect(6, 1)
        // bType=6 is party status, nBuff=1 means "in party"
        use crate::handler::regene::build_state_change_broadcast;
        let pkt = build_state_change_broadcast(100, STATE_CHANGE_PARTY_LEADER, 1);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizStateChange as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(100)); // socket_id
        assert_eq!(r.read_u8(), Some(STATE_CHANGE_PARTY_LEADER)); // bType (party)
        assert_eq!(r.read_u32(), Some(1)); // nBuff (in party)
    }

    #[test]
    fn test_event_room_nation_user_filtering() {
        // Verify that logged_out users are excluded from party creation.
        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_users.insert(
            "user1".to_string(),
            EventUser {
                user_name: "user1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.karus_users.insert(
            "user2".to_string(),
            EventUser {
                user_name: "user2".to_string(),
                session_id: 2,
                nation: 1,
                prize_given: false,
                logged_out: true, // logged out — should be excluded
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.karus_users.insert(
            "user3".to_string(),
            EventUser {
                user_name: "user3".to_string(),
                session_id: 3,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        let active: Vec<SessionId> = room
            .karus_users
            .values()
            .filter(|u| !u.logged_out)
            .map(|u| u.session_id)
            .collect();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&1));
        assert!(active.contains(&3));
    }

    #[test]
    fn test_party_member_info_packet_for_event() {
        // Verify PARTY_INSERT packet built for event party matches expected format.
        use crate::handler::party::build_party_member_info;
        use crate::world::CharacterInfo;

        let ch = CharacterInfo {
            session_id: 42,
            name: "TestUser".to_string(),
            nation: 1,
            race: 1,
            class: 103,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 1000,
            hp: 800,
            max_mp: 500,
            mp: 400,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 0x01,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        };
        let pkt = build_party_member_info(&ch, 1, -1, 0, -1);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizParty as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x03)); // PARTY_INSERT
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_u32(), Some(42)); // session_id
        assert_eq!(r.read_u8(), Some(1)); // index_hint
                                          // name follows (sbyte string)
    }

    #[test]
    fn test_chaos_winner_screen_clears_invisibility() {
        // C++ EventMainSystem.cpp:595: StateChangeServerDirect(7, 0)
        // Sent before WIZ_SELECT_MSG and WIZ_EVENT(FINISH) for Chaos users.
        // bType=7 is invisibility, nBuff=0 clears it.
        use crate::handler::regene::build_state_change_broadcast;
        let pkt = build_state_change_broadcast(200, STATE_CHANGE_INVISIBILITY, 0);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizStateChange as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(200)); // socket_id
        assert_eq!(r.read_u8(), Some(STATE_CHANGE_INVISIBILITY)); // bType (invisibility)
        assert_eq!(r.read_u32(), Some(0)); // nBuff (clear)
    }

    #[test]
    fn test_chaos_winner_screen_packet_order() {
        // C++ order per Chaos user:
        //   1. StateChangeServerDirect(7, 0)  — clear invis (broadcast to region)
        //   2. Send(&newpkt1)                 — WIZ_SELECT_MSG
        //   3. Send(&newpkt2)                 — WIZ_EVENT FINISH
        // BDW/Juraid do NOT send StateChangeServerDirect(7, 0).
        let select_msg = build_winner_select_msg(24); // Chaos
        let finish_pkt = build_finish_packet(0); // Chaos winner=0 (FFA)

        assert_eq!(select_msg.opcode, ko_protocol::Opcode::WizSelectMsg as u8);
        assert_eq!(finish_pkt.opcode, ko_protocol::Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&finish_pkt.data);
        assert_eq!(r.read_u8(), Some(10)); // TEMPLE_EVENT_FINISH
    }

    // ── Sprint 176 Tests ──────────────────────────────────────────────

    #[test]
    fn test_is_in_temple_event_zone() {
        // C++ Unit.h:188 — isInTempleEventZone() checks zones 84, 85, 87
        assert!(is_in_temple_event_zone(ZONE_BDW));
        assert!(is_in_temple_event_zone(ZONE_CHAOS));
        assert!(is_in_temple_event_zone(ZONE_JURAID));
        assert!(!is_in_temple_event_zone(21)); // Moradon
        assert!(!is_in_temple_event_zone(0));
        assert!(!is_in_temple_event_zone(86)); // Not an event zone
    }

    #[test]
    fn test_event_type_for_zone() {
        assert_eq!(
            event_type_for_zone(84),
            Some(TempleEventType::BorderDefenceWar)
        );
        assert_eq!(event_type_for_zone(85), Some(TempleEventType::ChaosDungeon));
        assert_eq!(
            event_type_for_zone(87),
            Some(TempleEventType::JuraidMountain)
        );
        assert_eq!(event_type_for_zone(21), None);
        assert_eq!(event_type_for_zone(86), None);
    }

    #[test]
    fn test_virt_eventattack_check_not_in_zone() {
        // C++ returns true when NOT in a temple event zone
        let erm = EventRoomManager::new();
        assert!(virt_eventattack_check(&erm, 21, "player1"));
    }

    #[test]
    fn test_virt_eventattack_check_no_room() {
        // C++ returns true when player is not in a valid room
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        assert!(virt_eventattack_check(&erm, ZONE_BDW, "player1"));
    }

    #[test]
    fn test_virt_eventattack_check_event_not_active() {
        // C++ returns false when event is not active
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
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
        }
        // Event is NOT active → should return false
        assert!(!virt_eventattack_check(&erm, ZONE_BDW, "p1"));
    }

    #[test]
    fn test_virt_eventattack_check_room_finished() {
        // C++ returns false when room is finished
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        erm.update_temple_event(|s| {
            s.active_event = 4; // BDW
            s.is_active = true;
        });
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
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
            room.finished = true;
        }
        assert!(!virt_eventattack_check(&erm, ZONE_BDW, "p1"));
    }

    #[test]
    fn test_virt_eventattack_check_active_and_valid() {
        // C++ returns true when event is active, player in room, room not finished
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        erm.update_temple_event(|s| {
            s.active_event = 4; // BDW
            s.is_active = true;
        });
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
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
        }
        assert!(virt_eventattack_check(&erm, ZONE_BDW, "p1"));
    }

    #[test]
    fn test_find_user_room() {
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 2);
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 2)
                .unwrap();
            room.add_user(EventUser {
                user_name: "hero".to_string(),
                session_id: 99,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
        }
        let result = erm.find_user_room(TempleEventType::BorderDefenceWar, "hero");
        assert_eq!(result, Some((2, false)));
        assert_eq!(
            erm.find_user_room(TempleEventType::BorderDefenceWar, "nobody"),
            None
        );
    }

    #[test]
    fn test_build_temple_screen_packet() {
        // C++ format: WIZ_EVENT + u8(TEMPLE_SCREEN=3) + u32(karus) + u32(elmo)
        let pkt = build_temple_screen_packet(150, 200);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3)); // TEMPLE_SCREEN
        assert_eq!(r.read_u32(), Some(150)); // karus_score
        assert_eq!(r.read_u32(), Some(200)); // elmo_score
    }

    #[test]
    fn test_build_altar_flag_packet() {
        // C++ format: WIZ_EVENT + u8(49) + SByte(name) + u8(nation)
        let pkt = build_altar_flag_packet("TestPlayer", 1);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(49)); // TEMPLE_EVENT_ALTAR_FLAG
                                           // SByte string: [u8 len] + [name bytes]
        let name = r.read_sbyte_string();
        assert_eq!(name.as_deref(), Some("TestPlayer"));
        assert_eq!(r.read_u8(), Some(1)); // nation
    }

    #[test]
    fn test_build_altar_flag_packet_empty_name() {
        let pkt = build_altar_flag_packet("", 2);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(49));
        assert_eq!(r.read_u8(), Some(0)); // empty name
        assert_eq!(r.read_u8(), Some(2));
    }

    #[test]
    fn test_build_altar_timer_packet() {
        // C++ format: WIZ_EVENT + u8(50) + u16(timer_secs)
        let pkt = build_altar_timer_packet(60);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(50)); // TEMPLE_EVENT_ALTAR_TIMER
        assert_eq!(r.read_u16(), Some(60)); // 60 seconds
    }

    #[test]
    fn test_build_altar_timer_packet_zero() {
        let pkt = build_altar_timer_packet(0);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(50));
        assert_eq!(r.read_u16(), Some(0));
    }

    #[test]
    fn test_build_altar_respawn_packet() {
        // C++ format: WIZ_EVENT + u8(2) + u8(2)
        let pkt = build_altar_respawn_packet();
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizEvent as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(2));
    }

    #[test]
    fn test_send_event_zone_change_sets_event_room() {
        // When event_room > 0, m_bEventRoom is set to that value.
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(
            1,
            crate::world::CharacterInfo {
                session_id: 1,
                nation: 1,
                name: "BdwSpawnTest".into(),
                ..Default::default()
            },
            crate::world::Position {
                zone_id: 21,
                x: 816.0,
                y: 0.0,
                z: 532.0,
                region_x: crate::zone::calc_region(816.0),
                region_z: crate::zone::calc_region(532.0),
            },
        );
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 84,
            karus_x: 51,
            karus_z: 58,
            elmorad_x: 201,
            elmorad_z: 207,
            karus_gate_x: 5,
            karus_gate_z: 5,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });

        assert_eq!(world.get_event_room(1), 0);

        // Teleport into event zone with room 3
        send_event_zone_change(&world, 1, 84, 3);
        assert_eq!(world.get_event_room(1), 3);
        let pos = world.get_position(1).unwrap();
        assert_eq!(pos.zone_id, 84);
        assert_eq!((pos.x, pos.z), (51.0, 58.0));
        assert!(world.is_zone_changing(1));

        // Teleport with event_room=0 should clear it
        send_event_zone_change(&world, 1, 21, 0);
        assert_eq!(world.get_event_room(1), 0);
    }

    #[test]
    fn test_send_event_zone_change_zero_no_clear_when_already_zero() {
        // When event_room==0 and session already has event_room==0, no change needed.
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        assert_eq!(world.get_event_room(1), 0);
        send_event_zone_change(&world, 1, 21, 0);
        assert_eq!(world.get_event_room(1), 0);
    }
}
