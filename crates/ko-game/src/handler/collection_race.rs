//! Collection Race event handler — WIZ_EXT_HOOK (0xE9) sub-opcode CR (0xAF).
//! ## Overview
//! Collection Race is a server-wide timed kill-count event. Players must kill
//! specified monsters (up to 3 types, each with a required count) to finish.
//! ## Sub-opcode flow (ExtSub::CR = 0xAF) — SERVER → CLIENT only
//! | sub | Direction          | Description                                    |
//! |-----|--------------------|------------------------------------------------|
//! | 0x00| Server → Client    | Event start broadcast (new event begins)       |
//! | 0x01| Server → Client    | Game-entry / refresh (for reconnecting player) |
//! | 0x02| Server → Client    | Kill progress update (one monster killed)      |
//! | 0x03| Server → Client    | Total count update broadcast (counter change)  |
//! | 0x04| Server → Client    | Event end / finished (hide UI)                 |
//! | 0x05| Server → Client    | Hide event UI (player out of range/level)      |
//! ## Packet formats (WIZ_EXT_HOOK = 0xE9)
//! ### Start (sub=0x00) — sent to all eligible players on event start
//! ```text
//! 0xE9 << u8(0xAF) << u8(0x00)
//!   << [3x: u16 proto_id, u16 kill_count_required]
//!   << [3x: u32 reward_item_id, u32 reward_item_count, u8 reward_rate]
//!   << u32(event_duration_secs) << u16(total_count * rank_bug)
//!   << u16(user_limit) << u8(nation) << sbyte_string(event_name) << u8(zone_id)
//! ```
//! ### Game-entry / refresh (sub=0x01) — sent to reconnecting player
//! ```text
//! 0xE9 << u8(0xAF) << u8(0x01)
//!   << [3x: u16 proto_id, u16 kill_count_required, u16 my_kill_count]
//!   << [3x: u32 reward_item_id, u32 reward_item_count, u8 reward_rate]
//!   << u32(remaining_secs) << u16(total_count * rank_bug)
//!   << u16(user_limit) << u8(nation) << sbyte_string(event_name) << u8(zone_id)
//! ```
//! ### Kill progress (sub=0x02) — sent to the killing player on each kill
//! ```text
//! 0xE9 << u8(0xAF) << u8(0x02)
//!   << u16(proto_id) << u16(kill_count[0]) << u16(kill_count[1]) << u16(kill_count[2])
//! ```
//! ### Total count update (sub=0x03) — broadcast to all eligible on completion
//! ```text
//! 0xE9 << u8(0xAF) << u8(0x03) << u16(total_count * rank_bug) << u16(user_limit)
//! ```
//! ### End / finish (sub=0x04) — sent to finisher or all on event end
//! ```text
//! 0xE9 << u8(0xAF) << u8(0x04)
//! ```
//! ### Hide (sub=0x05) — sent to player leaving eligible level/zone
//! ```text
//! 0xE9 << u8(0xAF) << u8(0x05)
//! ```
//! ## WIZ_QUEST sub=0x0A — reward packet sent on completion
//! ```text
//! WIZ_QUEST(0x64) << u8(0x0A)
//!   << [3x: u32 item_id, u32 item_count]
//!   << u32(0) << u32(0) << u32(0) << u32(0)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::{debug, info, warn};

use ko_protocol::{Opcode, Packet};

use crate::handler::chat::{build_chat_packet, ChatType};
use crate::handler::level::exp_change;
use crate::session::ClientSession;
use crate::systems::loyalty::send_loyalty_change;
use crate::world::{WorldState, ITEM_GOLD};
use crate::zone::SessionId;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) use super::ext_hook::EXT_SUB_COLLECTION_RACE;

/// Sub-opcode: event start broadcast (sent to all eligible players).
pub const SUB_START: u8 = 0x00;

/// Sub-opcode: game-entry / refresh (sent when player enters during active event).
pub const SUB_REFRESH: u8 = 0x01;

/// Sub-opcode: kill progress update (sent to killer on each qualifying kill).
pub const SUB_KILL_UPDATE: u8 = 0x02;

/// Sub-opcode: total count update broadcast (sent to all on finisher).
pub const SUB_COUNT_UPDATE: u8 = 0x03;

/// Sub-opcode: event end / player finished (sent to finisher or all on end).
pub const SUB_END: u8 = 0x04;

/// Sub-opcode: hide event UI.
pub const SUB_HIDE: u8 = 0x05;

/// Number of monster slots per event (always 3).
pub const CR_SLOTS: usize = 3;

/// Number of reward slots per event (always 3, same as monster slots).
pub const CR_REWARD_SLOTS: usize = 3;

use crate::world::{ITEM_COUNT, ITEM_EXP, ITEM_RANDOM};

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// Per-user kill progress in the active Collection Race.
#[derive(Debug, Clone, Default)]
pub struct CrUserList {
    /// How many of each monster type the user has killed.
    pub kill_counts: [u16; CR_SLOTS],
    /// Whether the user has completed this race round.
    pub is_finish: bool,
    /// How many full completions the user has done (for repeat-mode).
    ///
    pub user_status: u8,
}

/// Active Collection Race event state (server-wide singleton).
#[derive(Debug, Clone)]
pub struct CollectionRaceEvent {
    /// Whether an event is currently active.
    pub is_active: bool,
    /// Whether a start request has been sent to DB (prevents duplicate requests).
    pub request_pending: bool,
    /// Unix timestamp when the event ends.
    pub event_end_time: u32,
    /// Minimum level to participate.
    pub min_level: u8,
    /// Maximum level to participate.
    pub max_level: u8,
    /// Zone ID restriction (currently informational only — all zones allowed).
    pub zone_id: u8,
    /// Maximum number of finishers.
    pub user_limit: u16,
    /// Display name shown in UI.
    pub event_name: String,
    /// Repeat mode: 0=finish&close, 1=repeat×2.5, 2=all-repeat
    ///
    pub event_list_status: u8,
    /// Monster proto IDs for each slot (0 = unused).
    pub proto_ids: [u16; CR_SLOTS],
    /// Required kill counts per slot (0 = unused).
    pub kill_counts_req: [u16; CR_SLOTS],
    /// Reward item IDs (ITEM_GOLD / ITEM_EXP / ITEM_COUNT / real item).
    pub reward_item_ids: [u32; CR_REWARD_SLOTS],
    /// Reward item counts.
    pub reward_item_counts: [u32; CR_REWARD_SLOTS],
    /// Reward item duration (0 = permanent).
    pub reward_item_times: [u32; CR_REWARD_SLOTS],
    /// Reward rate (0-100 %). 0 means 100 % guaranteed.
    pub reward_item_rates: [u8; CR_REWARD_SLOTS],
    /// Reward session IDs (for random-item selection, 0 = not random).
    pub reward_sessions: [u8; CR_REWARD_SLOTS],
    /// Total number of players who have finished.
    pub total_count: u16,
    /// Random rank multiplier for display
    pub rank_bug: u32,
    /// Per-user kill progress and finish status, keyed by character name (uppercase).
    pub user_list: HashMap<String, CrUserList>,
}

impl Default for CollectionRaceEvent {
    fn default() -> Self {
        Self {
            is_active: false,
            request_pending: false,
            event_end_time: 0,
            min_level: 1,
            max_level: 83,
            zone_id: 0,
            user_limit: 0,
            event_name: String::new(),
            event_list_status: 0,
            proto_ids: [0u16; CR_SLOTS],
            kill_counts_req: [0u16; CR_SLOTS],
            reward_item_ids: [0u32; CR_REWARD_SLOTS],
            reward_item_counts: [0u32; CR_REWARD_SLOTS],
            reward_item_times: [0u32; CR_REWARD_SLOTS],
            reward_item_rates: [0u8; CR_REWARD_SLOTS],
            reward_sessions: [0u8; CR_REWARD_SLOTS],
            total_count: 0,
            rank_bug: 1,
            user_list: HashMap::new(),
        }
    }
}

/// Thread-safe shared Collection Race event state.
pub type SharedCollectionRaceEvent = Arc<RwLock<CollectionRaceEvent>>;

/// Create a new default Collection Race event.
pub fn new_collection_race_event() -> SharedCollectionRaceEvent {
    Arc::new(RwLock::new(CollectionRaceEvent::default()))
}

/// Static event definition loaded from DB.
#[derive(Debug, Clone)]
pub struct CrEventDef {
    /// Event ID (primary key from DB).
    pub event_id: i16,
    /// Display name.
    pub event_name: String,
    /// Monster proto IDs for the 3 slots.
    pub proto_ids: [u16; CR_SLOTS],
    /// Required kill counts per slot.
    pub kill_counts: [u16; CR_SLOTS],
    /// Minimum player level.
    pub min_level: u8,
    /// Maximum player level.
    pub max_level: u8,
    /// Zone ID.
    pub zone_id: u8,
    /// Duration in minutes.
    pub event_time_mins: u32,
    /// Maximum finishers.
    pub user_limit: u16,
    /// Repeat mode.
    pub event_list_status: u8,
    /// Auto-start enabled.
    pub auto_start: bool,
    /// Auto-start hour.
    pub auto_hour: i32,
    /// Auto-start minute.
    pub auto_minute: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Packet Builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build a CR start broadcast packet (sub=0x00).
/// Sent to all eligible players when a new Collection Race event begins.
/// Wire format (all little-endian):
/// ```text
/// 0xE9 << u8(0xAF) << u8(0x00)
///   << [3x: u16 proto_id, u16 kill_count_req]
///   << [3x: u32 reward_id, u32 reward_count, u8 reward_rate]
///   << u32(event_duration_secs)
///   << u16(total_count * rank_bug) << u16(user_limit)
///   << u8(nation) << sbyte_string(event_name) << u8(zone_id)
/// ```
pub fn build_start_packet(ev: &CollectionRaceEvent, duration_secs: u32, nation: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_COLLECTION_RACE);
    pkt.write_u8(SUB_START);

    for i in 0..CR_SLOTS {
        pkt.write_u16(ev.proto_ids[i]);
        pkt.write_u16(ev.kill_counts_req[i]);
    }
    for i in 0..CR_REWARD_SLOTS {
        pkt.write_u32(ev.reward_item_ids[i]);
        pkt.write_u32(ev.reward_item_counts[i]);
        pkt.write_u8(ev.reward_item_rates[i]);
    }

    pkt.write_u32(duration_secs);
    pkt.write_u16((ev.total_count as u32 * ev.rank_bug) as u16);
    pkt.write_u16(ev.user_limit);
    pkt.write_u8(nation);
    pkt.write_string(&ev.event_name);
    pkt.write_u8(ev.zone_id);
    pkt
}

/// Build a CR refresh/game-entry packet (sub=0x01).
/// Sent to a player who logs in while an event is active. Includes their
/// per-user kill progress and the adjusted required counts (×2.5 if in repeat).
/// Wire format:
/// ```text
/// 0xE9 << u8(0xAF) << u8(0x01)
///   << [3x: u16 proto_id, u16 kill_count_req_adjusted, u16 my_kill_count]
///   << [3x: u32 reward_id, u32 reward_count, u8 reward_rate]
///   << u32(remaining_secs) << u16(total_count * rank_bug)
///   << u16(user_limit) << u8(nation) << sbyte_string(event_name) << u8(zone_id)
/// ```
pub fn build_refresh_packet(
    ev: &CollectionRaceEvent,
    my_kill_counts: [u16; CR_SLOTS],
    user_status: u8,
    remaining_secs: u32,
    nation: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_COLLECTION_RACE);
    pkt.write_u8(SUB_REFRESH);

    for ((proto_id, kill_req), my_count) in ev
        .proto_ids
        .iter()
        .zip(ev.kill_counts_req.iter())
        .zip(my_kill_counts.iter())
    {
        // C++ multiplies required by 2.5 if user_status > 0 (repeat mode)
        let required = if user_status > 0 {
            (*kill_req as f32 * 2.5) as u16
        } else {
            *kill_req
        };
        pkt.write_u16(*proto_id);
        pkt.write_u16(required);
        pkt.write_u16(*my_count);
    }
    for i in 0..CR_REWARD_SLOTS {
        pkt.write_u32(ev.reward_item_ids[i]);
        pkt.write_u32(ev.reward_item_counts[i]);
        pkt.write_u8(ev.reward_item_rates[i]);
    }

    pkt.write_u32(remaining_secs);
    pkt.write_u16((ev.total_count as u32 * ev.rank_bug) as u16);
    pkt.write_u16(ev.user_limit);
    pkt.write_u8(nation);
    pkt.write_string(&ev.event_name);
    pkt.write_u8(ev.zone_id);
    pkt
}

/// Build a kill-progress update packet (sub=0x02).
/// Sent to the killer after each qualifying kill.
/// Wire format:
/// ```text
/// 0xE9 << u8(0xAF) << u8(0x02)
///   << u16(proto_id)
///   << u16(kill_count[0]) << u16(kill_count[1]) << u16(kill_count[2])
/// ```
pub fn build_kill_update_packet(proto_id: u16, kill_counts: [u16; CR_SLOTS]) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_COLLECTION_RACE);
    pkt.write_u8(SUB_KILL_UPDATE);
    pkt.write_u16(proto_id);
    for kc in &kill_counts {
        pkt.write_u16(*kc);
    }
    pkt
}

/// Build a total count update broadcast packet (sub=0x03).
/// Broadcast to all eligible players when a player finishes.
/// Wire format:
/// ```text
/// 0xE9 << u8(0xAF) << u8(0x03) << u16(total_count * rank_bug) << u16(user_limit)
/// ```
pub fn build_count_update_packet(total_count: u16, rank_bug: u32, user_limit: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_COLLECTION_RACE);
    pkt.write_u8(SUB_COUNT_UPDATE);
    pkt.write_u16((total_count as u32 * rank_bug) as u16);
    pkt.write_u16(user_limit);
    pkt
}

/// Build an event-end / finish packet (sub=0x04).
/// Sent to the player who finishes (closes the CR UI) or to all when event ends.
/// Wire format:
/// ```text
/// 0xE9 << u8(0xAF) << u8(0x04)
/// ```
pub fn build_end_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_COLLECTION_RACE);
    pkt.write_u8(SUB_END);
    pkt
}

/// Build a hide packet (sub=0x05).
/// Sent when player's level is outside event range or on other exclusion events.
/// Wire format:
/// ```text
/// 0xE9 << u8(0xAF) << u8(0x05)
/// ```
pub fn build_hide_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_COLLECTION_RACE);
    pkt.write_u8(SUB_HIDE);
    pkt
}

/// Build a WIZ_QUEST sub=0x0A reward packet (sent to finisher).
/// Wire format:
/// ```text
/// WIZ_QUEST(0x64) << u8(0x0A)
///   << [3x: u32 item_id, u32 item_count]
///   << u32(0) << u32(0) << u32(0) << u32(0)
/// ```
pub fn build_quest_reward_packet(
    item_ids: [u32; CR_REWARD_SLOTS],
    item_counts: [u32; CR_REWARD_SLOTS],
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizQuest as u8);
    pkt.write_u8(0x0A);
    for i in 0..CR_REWARD_SLOTS {
        pkt.write_u32(item_ids[i]);
        pkt.write_u32(item_counts[i]);
    }
    // Four trailing u32 zeros
    pkt.write_u32(0);
    pkt.write_u32(0);
    pkt.write_u32(0);
    pkt.write_u32(0);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Chat Helper
// ─────────────────────────────────────────────────────────────────────────────

/// Build a server-wide PUBLIC_CHAT announcement (mirrors `SendChat<PUBLIC_CHAT>`).
pub fn build_cr_announce(msg: &str) -> Packet {
    build_chat_packet(
        ChatType::Public as u8,
        0, // nation ALL
        0, // sender_id 0 (server)
        "SYSTEM",
        msg,
        0,  // personal_rank
        0,  // authority (GM)
        20, // system_msg = GM color
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Start / Stop Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Start a Collection Race event from the given definition.
/// Called by the GM command `+cropen <index>` → `CollectionRaceStart()`.
/// Returns `false` if another event is already running or definition is invalid.
pub fn start_event(
    cr: &SharedCollectionRaceEvent,
    def: &CrEventDef,
    rewards: &[(u32, u32, u32, u8, u8)], // (item_id, count, time, rate, session)
    rank_bug: u32,
    now: u32,
) -> bool {
    // Validate rewards don't exceed limits
    for (item_id, count, _time, _rate, _session) in rewards {
        if *item_id == ITEM_GOLD || *item_id == ITEM_EXP {
            if *count > 9_999_999 {
                warn!("[CR] start_event: gold/exp reward too large ({})", count);
                return false;
            }
        } else if *item_id != 0 && *count > 9999 {
            warn!(
                "[CR] start_event: item count too large ({} x {})",
                item_id, count
            );
            return false;
        }
    }

    let mut ev = cr.write();

    if ev.is_active {
        drop(ev);
        reset_event(cr);
        let mut ev = cr.write();
        apply_start(&mut ev, def, rewards, rank_bug, now);
    } else {
        apply_start(&mut ev, def, rewards, rank_bug, now);
    }

    info!(
        "[CR] Event started: '{}' limit={} duration={}min",
        def.event_name, def.user_limit, def.event_time_mins
    );
    true
}

fn apply_start(
    ev: &mut CollectionRaceEvent,
    def: &CrEventDef,
    rewards: &[(u32, u32, u32, u8, u8)],
    rank_bug: u32,
    now: u32,
) {
    ev.is_active = true;
    ev.request_pending = false;
    ev.event_end_time = now + def.event_time_mins * 60;
    ev.min_level = def.min_level;
    ev.max_level = def.max_level;
    ev.zone_id = def.zone_id;
    ev.user_limit = def.user_limit;
    ev.event_name = def.event_name.clone();
    ev.event_list_status = def.event_list_status;
    ev.proto_ids = def.proto_ids;
    ev.kill_counts_req = def.kill_counts;
    ev.total_count = 0;
    ev.rank_bug = if rank_bug == 0 { 1 } else { rank_bug };
    ev.user_list.clear();

    // Fill rewards (up to CR_REWARD_SLOTS)
    for i in 0..CR_REWARD_SLOTS {
        if let Some(&(item_id, count, time, rate, session)) = rewards.get(i) {
            let capped_rate = rate.min(100);
            ev.reward_item_ids[i] = item_id;
            ev.reward_item_counts[i] = count;
            ev.reward_item_times[i] = time;
            ev.reward_item_rates[i] = capped_rate;
            ev.reward_sessions[i] = session;
        } else {
            ev.reward_item_ids[i] = 0;
            ev.reward_item_counts[i] = 0;
            ev.reward_item_times[i] = 0;
            ev.reward_item_rates[i] = 0;
            ev.reward_sessions[i] = 0;
        }
    }
}

/// Reset/end the active Collection Race event.
pub fn reset_event(cr: &SharedCollectionRaceEvent) {
    let mut ev = cr.write();
    ev.is_active = false;
    ev.request_pending = false;
    ev.event_end_time = 0;
    ev.min_level = 1;
    ev.max_level = 83;
    ev.zone_id = 0;
    ev.user_limit = 0;
    ev.event_name.clear();
    ev.event_list_status = 0;
    ev.proto_ids = [0u16; CR_SLOTS];
    ev.kill_counts_req = [0u16; CR_SLOTS];
    ev.reward_item_ids = [0u32; CR_REWARD_SLOTS];
    ev.reward_item_counts = [0u32; CR_REWARD_SLOTS];
    ev.reward_item_times = [0u32; CR_REWARD_SLOTS];
    ev.reward_item_rates = [0u8; CR_REWARD_SLOTS];
    ev.reward_sessions = [0u8; CR_REWARD_SLOTS];
    ev.total_count = 0;
    ev.rank_bug = 1;
    ev.user_list.clear();
}

// ─────────────────────────────────────────────────────────────────────────────
// Game-entry: send current CR state to a newly logged-in player
// ─────────────────────────────────────────────────────────────────────────────

/// Send Collection Race state to a player on game entry.
/// Called from `gamestart` when a player enters the world while a CR event
/// is active. Mirrors `CUser::CollectionRaceFirstLoad()`.
pub async fn send_on_game_entry(
    session: &mut ClientSession,
    cr: &SharedCollectionRaceEvent,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    let char_info = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    let char_name = char_info.name.to_uppercase();
    let level = char_info.level;
    let nation = char_info.nation;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    // Build packet inside a block to drop the lock before .await
    let maybe_pkt: Option<Packet> = {
        let mut ev = cr.write();

        if !ev.is_active || ev.event_end_time <= now || level < ev.min_level || level > ev.max_level
        {
            None
        } else {
            // Ensure user is in the list
            let user_entry = ev.user_list.entry(char_name.clone()).or_default();
            if user_entry.is_finish {
                None
            } else {
                let my_kills = user_entry.kill_counts;
                let user_status = user_entry.user_status;
                let remaining = ev.event_end_time.saturating_sub(now);
                Some(build_refresh_packet(
                    &ev,
                    my_kills,
                    user_status,
                    remaining,
                    nation,
                ))
            }
        }
        // ev dropped here
    };

    if let Some(pkt) = maybe_pkt {
        session.send_packet(&pkt).await?;
        debug!(
            "[{}] CR: sent game-entry refresh to {}",
            session.addr(),
            char_name
        );
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// NPC Kill Handling
// ─────────────────────────────────────────────────────────────────────────────

/// Process a monster kill by a player for the active Collection Race event.
/// Called from `handle_npc_death` for the killer when an NPC dies.
/// This function:
/// 1. Checks the CR is active and the killer is in level range.
/// 2. Increments the appropriate kill counter if the proto_id matches.
/// 3. Checks if the player has completed all required kills.
/// 4. If finished, awards rewards and sends counter broadcast.
/// 5. If event user_limit reached, ends the event.
/// Returns a packet to send to the killer (kill-progress update).
pub async fn handle_kill(
    world: &WorldState,
    killer_sid: SessionId,
    proto_id: u16,
    cr: &SharedCollectionRaceEvent,
) -> anyhow::Result<()> {
    let char_info = match world.get_character_info(killer_sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    let char_name = char_info.name.to_uppercase();
    let level = char_info.level;

    // ── 1. Check event active + level range ─────────────────────────────
    let (is_active, min_level, max_level) = {
        let ev = cr.read();
        (ev.is_active, ev.min_level, ev.max_level)
    };

    if !is_active {
        return Ok(());
    }
    if level < min_level || level > max_level {
        return Ok(());
    }

    // Check already finished
    let already_finished = world
        .with_session(killer_sid, |h| h.cr_check_finish)
        .unwrap_or(true);
    if already_finished {
        return Ok(());
    }

    // ── 2. Process kill ──────────────────────────────────────────────────
    // We need write access to update kill counts and check finish.
    // All logic runs under the write lock to avoid partial-state issues.
    enum KillResult {
        /// Just incremented a counter — send kill-progress update.
        Progress { kill_counts: [u16; CR_SLOTS] },
        /// Player completed all required kills.
        Finished {
            kill_counts: [u16; CR_SLOTS],
            should_close_for_player: bool,
            event_ended: bool,
            new_total: u16,
            rank_bug: u32,
            user_limit: u16,
            reward_ids: [u32; CR_REWARD_SLOTS],
            reward_counts: [u32; CR_REWARD_SLOTS],
            reward_times: [u32; CR_REWARD_SLOTS],
            reward_rates: [u8; CR_REWARD_SLOTS],
        },
    }

    let kill_result: KillResult = {
        let mut ev = cr.write();

        // Copy immutable fields BEFORE taking the user_entry mutable borrow.
        // This avoids the Rust borrow-conflict between ev.user_list (mutable)
        // and ev.proto_ids / ev.kill_counts_req (immutable fields of the same struct).
        let proto_ids_snap = ev.proto_ids;
        let kill_counts_req_snap = ev.kill_counts_req;
        let list_status = ev.event_list_status;
        let reward_item_ids = ev.reward_item_ids;
        let reward_item_counts = ev.reward_item_counts;
        let reward_item_times = ev.reward_item_times;
        let reward_item_rates = ev.reward_item_rates;

        // Ensure user is registered
        let user_entry = ev.user_list.entry(char_name.clone()).or_default();
        if user_entry.is_finish {
            // Already finished (e.g. repeated-mode and closed)
            drop(ev);
            return Ok(());
        }

        // Get current session kill counts (from session state)
        let mut session_kills = world
            .with_session(killer_sid, |h| h.cr_kill_counts)
            .unwrap_or([0u16; CR_SLOTS]);

        let user_status = user_entry.user_status;
        let mut matched = false;

        // Find matching slot and increment
        for i in 0..CR_SLOTS {
            if proto_ids_snap[i] == 0 {
                continue;
            }
            if proto_ids_snap[i] != proto_id {
                continue;
            }
            // Compute required (adjusted for repeat status)
            let mut required = kill_counts_req_snap[i];
            if user_status > 0 {
                required = (required as f32 * 2.5) as u16;
            }
            if required == 0 {
                continue;
            }
            if session_kills[i] < required {
                session_kills[i] += 1;
                user_entry.kill_counts[i] = session_kills[i];
                matched = true;
            }
        }

        // Update session kill counts
        world.update_session(killer_sid, |h| {
            h.cr_kill_counts = session_kills;
        });

        if !matched {
            drop(ev);
            return Ok(()); // no matching slot or already at cap
        }

        // Check if player has completed all slots
        let mut all_done = true;
        for i in 0..CR_SLOTS {
            let mut required = kill_counts_req_snap[i];
            if user_status > 0 {
                required = (required as f32 * 2.5) as u16;
            }
            if required == 0 {
                continue; // unused slot
            }
            if session_kills[i] < required {
                all_done = false;
                break;
            }
        }

        if !all_done {
            KillResult::Progress {
                kill_counts: session_kills,
            }
        } else {
            // Player finished! Determine close behaviour
            let should_save_finish = match list_status {
                0 => true, // single-finish: always close for player
                1 => {
                    // repeat ×2.5: close after 2nd completion
                    user_entry.user_status += 1;
                    user_entry.user_status > 1
                }
                _ => false, // status==2: never close for player (infinite repeat)
            };

            if should_save_finish {
                user_entry.is_finish = true;
                world.update_session(killer_sid, |h| {
                    h.cr_check_finish = true;
                });
            }

            let should_close_for_player = should_save_finish && list_status != 2;

            // If repeat mode and not closing, reset kill counts for next round
            if !should_close_for_player {
                for i in 0..CR_SLOTS {
                    user_entry.kill_counts[i] = 0;
                }
                world.update_session(killer_sid, |h| {
                    h.cr_kill_counts = [0u16; CR_SLOTS];
                });
            }

            ev.total_count += 1;
            let new_total = ev.total_count;
            let rank_bug = ev.rank_bug;
            let user_limit = ev.user_limit;
            let event_ended = new_total >= user_limit;

            KillResult::Finished {
                kill_counts: session_kills,
                should_close_for_player,
                event_ended,
                new_total,
                rank_bug,
                user_limit,
                reward_ids: reward_item_ids,
                reward_counts: reward_item_counts,
                reward_times: reward_item_times,
                reward_rates: reward_item_rates,
            }
        }
    }; // lock released

    match kill_result {
        KillResult::Progress { kill_counts } => {
            // Send kill-progress update to killer
            let pkt = build_kill_update_packet(proto_id, kill_counts);
            world.send_to_session_owned(killer_sid, pkt);
        }
        KillResult::Finished {
            kill_counts,
            should_close_for_player,
            event_ended,
            new_total,
            rank_bug,
            user_limit,
            reward_ids,
            reward_counts,
            reward_times,
            reward_rates,
        } => {
            // 1. Send kill-progress update (sub=0x02) to killer
            let kill_pkt = build_kill_update_packet(proto_id, kill_counts);
            world.send_to_session_owned(killer_sid, kill_pkt);

            // 2. If player's UI should close, send end packet (sub=0x04)
            if should_close_for_player {
                let end_pkt = build_end_packet();
                world.send_to_session_owned(killer_sid, end_pkt);

                // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
                let chat_pkt = crate::systems::timed_notice::build_notice_packet(
                    7,
                    "[Collection Race] You have completed the event! Rewards incoming.",
                );
                world.send_to_session_owned(killer_sid, chat_pkt);
            }

            // 3. Broadcast counter update to all eligible players (sub=0x03)
            {
                let ev_snap = cr.read();
                let count_pkt = build_count_update_packet(new_total, rank_bug, user_limit);
                broadcast_to_eligible(world, &count_pkt, &ev_snap);
            }

            // 4. Give rewards to the finisher
            give_rewards(
                world,
                killer_sid,
                reward_ids,
                reward_counts,
                reward_times,
                reward_rates,
            )
            .await;

            debug!(
                "[CR] {} finished! total_finishers={}/{} event_ended={}",
                char_name, new_total, user_limit, event_ended
            );

            // 5. End event if limit reached
            if event_ended {
                end_event(world, cr);
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Reward Distribution
// ─────────────────────────────────────────────────────────────────────────────

/// Give Collection Race rewards to the finisher.
async fn give_rewards(
    world: &WorldState,
    sid: SessionId,
    reward_ids: [u32; CR_REWARD_SLOTS],
    reward_counts: [u32; CR_REWARD_SLOTS],
    reward_times: [u32; CR_REWARD_SLOTS],
    reward_rates: [u8; CR_REWARD_SLOTS],
) {
    // Phase 1: apply rate checks and build the final list of rewards to grant.
    // ThreadRng is not Send, so we must drop it before any .await point.
    // Collect all accepted rewards into a plain Vec first.
    #[allow(clippy::type_complexity)]
    let accepted: Vec<(usize, u32, u32, u32)> = {
        use rand::Rng as _;
        let mut rng = rand::thread_rng();
        let mut acc = Vec::new();
        for i in 0..CR_REWARD_SLOTS {
            let item_id = reward_ids[i];
            let count = reward_counts[i];
            let rate = reward_rates[i];
            let item_time = reward_times[i];

            if item_id == 0 || count == 0 {
                continue;
            }

            // Rate check (0 = guaranteed, 1-100 = percentage)
            if rate > 0 {
                let roll: u32 = rng.gen_range(0..10000);
                if (rate as u32 * 100) < roll {
                    debug!("[CR] reward slot {} failed rate check ({}/100)", i, rate);
                    continue;
                }
            }

            acc.push((i, item_id, count, item_time));
        }
        acc
        // rng is dropped here, before any await
    };

    // Phase 2: distribute accepted rewards (may await).
    let mut tmp_reward_ids = [0u32; CR_REWARD_SLOTS];
    let mut tmp_reward_counts = [0u32; CR_REWARD_SLOTS];

    for (i, item_id, count, item_time) in accepted {
        match item_id {
            ITEM_GOLD => {
                world.gold_gain(sid, count);
                debug!("[CR] gave gold reward: {} gold to sid={}", count, sid);
            }
            ITEM_EXP => {
                // exp_change is async — call and await directly
                exp_change(world, sid, count as i64).await;
                debug!("[CR] gave exp reward: {} exp to sid={}", count, sid);
            }
            ITEM_COUNT => {
                send_loyalty_change(world, sid, count as i32, false, false, true);
                debug!("[CR] gave loyalty reward: {} NP to sid={}", count, sid);
            }
            ITEM_RANDOM => {
                // Random item from pool — same pattern as daily_quest.rs:516-532
                let random_items = world.get_item_random_by_session(0);
                if !random_items.is_empty() {
                    let idx = (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .subsec_nanos() as usize)
                        % random_items.len();
                    let picked = &random_items[idx];
                    let given = world.give_item(
                        sid,
                        picked.item_id as u32,
                        picked.item_count.clamp(0, u16::MAX as i32) as u16,
                    );
                    if given {
                        tmp_reward_ids[i] = picked.item_id as u32;
                        tmp_reward_counts[i] = picked.item_count as u32;
                    } else {
                        super::daily_quest::send_reward_letter(
                            world,
                            sid,
                            picked.item_id,
                            picked.item_count as i16,
                        )
                        .await;
                    }
                }
                debug!("[CR] gave random item reward to sid={}", sid);
            }
            _ => {
                // Real item — give directly (with optional expiry time)
                let gave = if item_time > 0 {
                    world.give_item_with_expiry(sid, item_id, count as u16, item_time)
                } else {
                    world.give_item(sid, item_id, count as u16)
                };
                if gave {
                    tmp_reward_ids[i] = item_id;
                    tmp_reward_counts[i] = count;
                } else {
                    // Inventory full — send via letter (C++ parity: mail fallback)
                    super::daily_quest::send_reward_letter(
                        world,
                        sid,
                        item_id as i32,
                        count as i16,
                    )
                    .await;
                    warn!(
                        "[CR] inventory full — sent letter for item {} x{} sid={}",
                        item_id, count, sid
                    );
                }
                debug!("[CR] gave item {} x{} to sid={}", item_id, count, sid);
            }
        }
    }

    // Send the WIZ_QUEST 0x0A reward display packet to the finisher
    let reward_pkt = build_quest_reward_packet(tmp_reward_ids, tmp_reward_counts);
    world.send_to_session_owned(sid, reward_pkt);
}

// ─────────────────────────────────────────────────────────────────────────────
// Timer / Broadcast Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// End the active Collection Race event.
/// Broadcasts end packet to all eligible players and resets state.
pub fn end_event(world: &WorldState, cr: &SharedCollectionRaceEvent) {
    {
        let ev = cr.read();
        if ev.is_active {
            let announce = build_cr_announce("Collection Race Event has end.");
            world.broadcast_to_all(Arc::new(announce), None);

            let end_pkt = build_end_packet();
            broadcast_to_eligible(world, &end_pkt, &ev);
        }
    }
    reset_event(cr);
    info!("[CR] Event ended.");
}

/// Check Collection Race timer each second.
/// Called from the second-tick background task.
pub fn tick_timer(world: &WorldState, cr: &SharedCollectionRaceEvent, now: u32) {
    let ev = cr.read();
    if !ev.is_active {
        return;
    }

    let remaining = ev.event_end_time.saturating_sub(now);

    // Announce remaining time milestones (C++ uses LogosYolla announcements)
    let msg = match remaining {
        900 => Some("Collection Race: 15 minutes remaining."),
        600 => Some("Collection Race: 10 minutes remaining."),
        300 => Some("Collection Race: 5 minutes remaining."),
        180 => Some("Collection Race: 3 minutes remaining."),
        120 => Some("Collection Race: 2 minutes remaining."),
        60 => Some("Collection Race: 1 minute remaining."),
        _ => None,
    };
    if let Some(msg) = msg {
        let announce = build_cr_announce(msg);
        drop(ev); // release read lock before broadcasting
        world.broadcast_to_all(Arc::new(announce), None);
        return;
    }

    if remaining == 0 {
        drop(ev); // release read lock before calling end_event
        end_event(world, cr);
    }
}

/// Broadcast a packet to all eligible players (in level range).
fn broadcast_to_eligible(world: &WorldState, pkt: &Packet, ev: &CollectionRaceEvent) {
    let arc_pkt = Arc::new(pkt.clone());
    let sids: Vec<SessionId> = world.all_ingame_session_ids();
    for sid in sids {
        let level = match world.get_character_info(sid) {
            Some(c) => c.level,
            None => continue,
        };
        if level >= ev.min_level && level <= ev.max_level {
            world.send_to_session_arc(sid, Arc::clone(&arc_pkt));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GM Command Wrappers
// ─────────────────────────────────────────────────────────────────────────────

/// GM +cropen command: open a Collection Race event by index.
pub fn gm_open(world: &WorldState, event_index: i16) -> Result<&'static str, &'static str> {
    let def = match world.get_collection_race_def(event_index) {
        Some(d) => d,
        None => return Err("CollectionRace sEventIndex is nullptr"),
    };

    let cr = world.collection_race_event();

    {
        let ev = cr.read();
        if ev.is_active {
            drop(ev);
            end_event(world, cr);
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    // For now rewards are empty (loaded separately in full impl)
    let rewards: Vec<(u32, u32, u32, u8, u8)> = Vec::new();

    if !start_event(cr, &def, &rewards, 1, now) {
        return Err("Failed to start Collection Race event");
    }

    // Announce and send start packet to all eligible players
    let announce = build_cr_announce("Collection Race Event started.");
    world.broadcast_to_all(Arc::new(announce), None);

    {
        let ev = cr.read();
        let sids: Vec<SessionId> = world.all_ingame_session_ids();
        for sid in sids {
            let (level, nation) = match world.get_character_info(sid) {
                Some(c) => (c.level, c.nation),
                None => continue,
            };
            if level >= ev.min_level && level <= ev.max_level {
                let duration_secs = def.event_time_mins * 60;
                let pkt = build_start_packet(&ev, duration_secs, nation);
                world.send_to_session_owned(sid, pkt);

                // Reset per-user state
                world.update_session(sid, |h| {
                    h.cr_kill_counts = [0u16; CR_SLOTS];
                    h.cr_check_finish = false;
                });

                // Add to user_list
            }
        }
    }

    Ok("Collection Race start request sent.")
}

/// GM +crclose command: close the active Collection Race event.
pub fn gm_close(world: &WorldState) -> Result<&'static str, &'static str> {
    let cr = world.collection_race_event();
    {
        let ev = cr.read();
        if !ev.is_active {
            return Err("CR event is already closed.");
        }
    }
    end_event(world, cr);
    Ok("Collection Race closed.")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── Packet format tests ────────────────────────────────────────────────

    fn make_active_event() -> CollectionRaceEvent {
        CollectionRaceEvent {
            is_active: true,
            request_pending: false,
            event_end_time: 2_000_000,
            min_level: 1,
            max_level: 83,
            zone_id: 71,
            user_limit: 2500,
            event_name: "CR Test".to_string(),
            event_list_status: 0,
            proto_ids: [8013, 8017, 8851],
            kill_counts_req: [50, 50, 50],
            reward_item_ids: [389_196_000, 700_093_000, 0],
            reward_item_counts: [20, 1, 0],
            reward_item_times: [0, 0, 0],
            reward_item_rates: [100, 75, 0],
            reward_sessions: [0, 0, 0],
            total_count: 5,
            rank_bug: 2,
            user_list: HashMap::new(),
        }
    }

    #[test]
    fn test_ext_sub_collection_race_opcode_constant() {
        assert_eq!(EXT_SUB_COLLECTION_RACE, 0xAF);
    }

    #[test]
    fn test_reward_virtual_item_constants() {
        assert_eq!(ITEM_GOLD, 900_000_000);
        assert_eq!(ITEM_EXP, 900_001_000);
        assert_eq!(ITEM_COUNT, 900_002_000);
        assert_eq!(ITEM_RANDOM, 900_004_000);
    }

    #[test]
    fn test_build_start_packet_format() {
        let ev = make_active_event();
        let pkt = build_start_packet(&ev, 3600, 2); // nation=2

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_COLLECTION_RACE)); // 0xAF
        assert_eq!(r.read_u8(), Some(SUB_START)); // 0x00

        // 3 monster slots
        assert_eq!(r.read_u16(), Some(8013)); // proto_id[0]
        assert_eq!(r.read_u16(), Some(50)); // kill_count_req[0]
        assert_eq!(r.read_u16(), Some(8017)); // proto_id[1]
        assert_eq!(r.read_u16(), Some(50)); // kill_count_req[1]
        assert_eq!(r.read_u16(), Some(8851)); // proto_id[2]
        assert_eq!(r.read_u16(), Some(50)); // kill_count_req[2]

        // 3 reward slots
        assert_eq!(r.read_u32(), Some(389_196_000)); // reward_id[0]
        assert_eq!(r.read_u32(), Some(20)); // reward_count[0]
        assert_eq!(r.read_u8(), Some(100)); // reward_rate[0]
        assert_eq!(r.read_u32(), Some(700_093_000)); // reward_id[1]
        assert_eq!(r.read_u32(), Some(1)); // reward_count[1]
        assert_eq!(r.read_u8(), Some(75)); // reward_rate[1]
        assert_eq!(r.read_u32(), Some(0)); // reward_id[2]
        assert_eq!(r.read_u32(), Some(0)); // reward_count[2]
        assert_eq!(r.read_u8(), Some(0)); // reward_rate[2]

        // duration, total_count*rank_bug, user_limit, nation, event_name, zone_id
        assert_eq!(r.read_u32(), Some(3600)); // duration
        assert_eq!(r.read_u16(), Some(10)); // total(5) * rank_bug(2) = 10
        assert_eq!(r.read_u16(), Some(2500)); // user_limit
        assert_eq!(r.read_u8(), Some(2)); // nation
        assert_eq!(r.read_string(), Some("CR Test".to_string()));
        assert_eq!(r.read_u8(), Some(71)); // zone_id

        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_refresh_packet_format() {
        let ev = make_active_event();
        let my_kills = [10u16, 20u16, 5u16];
        let pkt = build_refresh_packet(&ev, my_kills, 0, 1800, 1);

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_COLLECTION_RACE));
        assert_eq!(r.read_u8(), Some(SUB_REFRESH));

        // Slot 0: proto_id=8013, required=50, my_kill=10
        assert_eq!(r.read_u16(), Some(8013));
        assert_eq!(r.read_u16(), Some(50));
        assert_eq!(r.read_u16(), Some(10));

        // Slot 1: proto_id=8017, required=50, my_kill=20
        assert_eq!(r.read_u16(), Some(8017));
        assert_eq!(r.read_u16(), Some(50));
        assert_eq!(r.read_u16(), Some(20));

        // Slot 2: proto_id=8851, required=50, my_kill=5
        assert_eq!(r.read_u16(), Some(8851));
        assert_eq!(r.read_u16(), Some(50));
        assert_eq!(r.read_u16(), Some(5));
    }

    #[test]
    fn test_build_refresh_packet_repeat_multiplier() {
        // When user_status > 0, required kill count is multiplied by 2.5
        let ev = make_active_event(); // kill_counts_req = [50, 50, 50]
        let pkt = build_refresh_packet(&ev, [0u16; CR_SLOTS], 1 /* user_status > 0 */, 900, 1);

        let mut r = PacketReader::new(&pkt.data);
        r.read_u8(); // EXT_SUB_COLLECTION_RACE
        r.read_u8(); // SUB_REFRESH

        // Slot 0: required should be 50 * 2.5 = 125
        r.read_u16(); // proto_id
        assert_eq!(r.read_u16(), Some(125)); // adjusted required
        r.read_u16(); // my_kill
    }

    #[test]
    fn test_build_kill_update_packet_format() {
        let pkt = build_kill_update_packet(8013, [15, 20, 5]);

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_COLLECTION_RACE));
        assert_eq!(r.read_u8(), Some(SUB_KILL_UPDATE));
        assert_eq!(r.read_u16(), Some(8013)); // proto_id
        assert_eq!(r.read_u16(), Some(15)); // kill_count[0]
        assert_eq!(r.read_u16(), Some(20)); // kill_count[1]
        assert_eq!(r.read_u16(), Some(5)); // kill_count[2]
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_count_update_packet_format() {
        let pkt = build_count_update_packet(3, 2, 2500);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_COLLECTION_RACE));
        assert_eq!(r.read_u8(), Some(SUB_COUNT_UPDATE));
        assert_eq!(r.read_u16(), Some(6)); // total(3) * rank_bug(2) = 6
        assert_eq!(r.read_u16(), Some(2500)); // user_limit
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_end_packet_format() {
        let pkt = build_end_packet();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_COLLECTION_RACE));
        assert_eq!(r.read_u8(), Some(SUB_END));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_hide_packet_format() {
        let pkt = build_hide_packet();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_COLLECTION_RACE));
        assert_eq!(r.read_u8(), Some(SUB_HIDE));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_quest_reward_packet_format() {
        // WIZ_QUEST(0x64) << u8(0x0A) << [3x: u32 id, u32 count] << 4x u32(0)
        let ids = [389_196_000u32, 0, 0];
        let counts = [20u32, 0, 0];
        let pkt = build_quest_reward_packet(ids, counts);

        assert_eq!(pkt.opcode, Opcode::WizQuest as u8);
        assert_eq!(pkt.opcode, 0x64);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x0A)); // sub

        assert_eq!(r.read_u32(), Some(389_196_000));
        assert_eq!(r.read_u32(), Some(20));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));

        // 4 trailing zeros
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));

        assert_eq!(r.remaining(), 0);
    }

    // ── State logic tests ──────────────────────────────────────────────────

    #[test]
    fn test_reset_event_clears_state() {
        let cr = new_collection_race_event();
        {
            let mut ev = cr.write();
            ev.is_active = true;
            ev.total_count = 99;
            ev.event_name = "Test".to_string();
            ev.user_list.insert(
                "PLAYER".to_string(),
                CrUserList {
                    kill_counts: [10, 20, 30],
                    is_finish: true,
                    user_status: 1,
                },
            );
        }
        reset_event(&cr);

        let ev = cr.read();
        assert!(!ev.is_active);
        assert_eq!(ev.total_count, 0);
        assert!(ev.event_name.is_empty());
        assert!(ev.user_list.is_empty());
    }

    #[test]
    fn test_start_event_sets_fields() {
        let cr = new_collection_race_event();
        let def = CrEventDef {
            event_id: 1,
            event_name: "CR Event".to_string(),
            proto_ids: [8013, 8017, 0],
            kill_counts: [50, 50, 0],
            min_level: 10,
            max_level: 83,
            zone_id: 71,
            event_time_mins: 60,
            user_limit: 2500,
            event_list_status: 0,
            auto_start: false,
            auto_hour: -1,
            auto_minute: -1,
        };
        let rewards = vec![(389_196_000u32, 20u32, 0u32, 100u8, 0u8)];

        let ok = start_event(&cr, &def, &rewards, 1, 1_000_000);
        assert!(ok);

        let ev = cr.read();
        assert!(ev.is_active);
        assert_eq!(ev.event_name, "CR Event");
        assert_eq!(ev.min_level, 10);
        assert_eq!(ev.user_limit, 2500);
        assert_eq!(ev.proto_ids[0], 8013);
        assert_eq!(ev.kill_counts_req[0], 50);
        assert_eq!(ev.reward_item_ids[0], 389_196_000);
        assert_eq!(ev.reward_item_counts[0], 20);
        assert_eq!(ev.reward_item_rates[0], 100);
    }

    #[test]
    fn test_start_event_while_active_restarts() {
        let cr = new_collection_race_event();
        let def = CrEventDef {
            event_id: 1,
            event_name: "First".to_string(),
            proto_ids: [1, 0, 0],
            kill_counts: [10, 0, 0],
            min_level: 1,
            max_level: 83,
            zone_id: 0,
            event_time_mins: 30,
            user_limit: 100,
            event_list_status: 0,
            auto_start: false,
            auto_hour: -1,
            auto_minute: -1,
        };
        start_event(&cr, &def, &[], 1, 1_000_000);

        let def2 = CrEventDef {
            event_name: "Second".to_string(),
            event_id: 2,
            ..def.clone()
        };
        start_event(&cr, &def2, &[], 1, 1_000_100);

        let ev = cr.read();
        assert!(ev.is_active);
        assert_eq!(ev.event_name, "Second");
    }

    #[test]
    fn test_start_event_clamps_rate_to_100() {
        let cr = new_collection_race_event();
        let def = CrEventDef {
            event_id: 1,
            event_name: "Test".to_string(),
            proto_ids: [1, 0, 0],
            kill_counts: [5, 0, 0],
            min_level: 1,
            max_level: 83,
            zone_id: 0,
            event_time_mins: 10,
            user_limit: 50,
            event_list_status: 0,
            auto_start: false,
            auto_hour: -1,
            auto_minute: -1,
        };
        // rate > 100 should be clamped to 100
        let rewards = vec![(100u32, 1u32, 0u32, 150u8, 0u8)];
        start_event(&cr, &def, &rewards, 1, 1_000_000);

        let ev = cr.read();
        assert_eq!(ev.reward_item_rates[0], 100);
    }

    #[test]
    fn test_cr_user_list_default() {
        let user = CrUserList::default();
        assert_eq!(user.kill_counts, [0, 0, 0]);
        assert!(!user.is_finish);
        assert_eq!(user.user_status, 0);
    }

    #[test]
    fn test_start_packet_little_endian() {
        // user_limit = 2500 = 0x09C4 → LE bytes [0xC4, 0x09]
        let ev = make_active_event();
        let pkt = build_start_packet(&ev, 3600, 1);

        // Locate user_limit bytes:
        // header: 2 bytes (EXT_SUB_COLLECTION_RACE + SUB_START)
        // 3 slots × (u16 + u16) = 12 bytes
        // 3 rewards × (u32 + u32 + u8) = 27 bytes
        // duration: u32 = 4 bytes
        // total×rank: u16 = 2 bytes
        // user_limit at offset = 2 + 12 + 27 + 4 + 2 = 47
        assert_eq!(pkt.data[47], 0xC4); // low byte of 2500
        assert_eq!(pkt.data[48], 0x09); // high byte of 2500
    }

    #[test]
    fn test_count_update_overflow_clamped() {
        // total_count * rank_bug can overflow u16 — ensure we cast correctly
        let pkt = build_count_update_packet(30000, 3, 100);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8(); // opcode
        r.read_u8(); // sub
                     // 30000 * 3 = 90000, as u16 = 90000 & 0xFFFF = 24464
        let val = r.read_u16().unwrap();
        assert_eq!(val, (90000u32 as u16));
    }

    #[test]
    fn test_build_start_packet_all_zeros_reward() {
        let mut ev = make_active_event();
        ev.reward_item_ids = [0; CR_REWARD_SLOTS];
        ev.reward_item_counts = [0; CR_REWARD_SLOTS];
        ev.reward_item_rates = [0; CR_REWARD_SLOTS];
        let pkt = build_start_packet(&ev, 600, 1);

        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8(); // header
        for _ in 0..CR_SLOTS {
            r.read_u16();
            r.read_u16();
        } // slots
        for _ in 0..CR_REWARD_SLOTS {
            assert_eq!(r.read_u32(), Some(0));
            assert_eq!(r.read_u32(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
        }
    }

    #[test]
    fn test_announce_packet_format() {
        let pkt = build_cr_announce("Collection Race Event started.");
        assert_eq!(pkt.opcode, Opcode::WizChat as u8);
        // Verify it starts with PUBLIC_CHAT type byte
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ChatType::Public as u8)); // 7
    }

    // ── Sprint 951: Additional coverage ──────────────────────────────

    /// CR sub-opcodes are sequential 0x00–0x05.
    #[test]
    fn test_cr_sub_opcodes_sequential() {
        assert_eq!(SUB_START, 0x00);
        assert_eq!(SUB_REFRESH, 0x01);
        assert_eq!(SUB_KILL_UPDATE, 0x02);
        assert_eq!(SUB_COUNT_UPDATE, 0x03);
        assert_eq!(SUB_END, 0x04);
        assert_eq!(SUB_HIDE, 0x05);
    }

    /// CR_SLOTS and CR_REWARD_SLOTS are both 3.
    #[test]
    fn test_cr_slot_constants() {
        assert_eq!(CR_SLOTS, 3);
        assert_eq!(CR_REWARD_SLOTS, 3);
    }

    /// CollectionRaceEvent default is inactive with empty arrays.
    #[test]
    fn test_cr_event_default() {
        let ev = CollectionRaceEvent::default();
        assert!(!ev.is_active);
        assert!(!ev.request_pending);
        assert_eq!(ev.proto_ids, [0; CR_SLOTS]);
        assert_eq!(ev.kill_counts_req, [0; CR_SLOTS]);
        assert_eq!(ev.total_count, 0);
        assert!(ev.user_list.is_empty());
    }

    /// CR event name defaults to empty, levels 1-83.
    #[test]
    fn test_cr_event_name_default() {
        let ev = CollectionRaceEvent::default();
        assert!(ev.event_name.is_empty());
        assert_eq!(ev.min_level, 1);
        assert_eq!(ev.max_level, 83);
    }

    /// CR reward arrays default to zeros.
    #[test]
    fn test_cr_reward_defaults() {
        let ev = CollectionRaceEvent::default();
        assert_eq!(ev.reward_item_ids, [0; CR_REWARD_SLOTS]);
        assert_eq!(ev.reward_item_counts, [0; CR_REWARD_SLOTS]);
        assert_eq!(ev.reward_item_rates, [0; CR_REWARD_SLOTS]);
    }

    // ── Sprint 964: Additional coverage ──────────────────────────────

    /// CR sub-opcodes are sequential 0x00-0x05.
    #[test]
    fn test_cr_sub_opcodes_values() {
        assert_eq!(SUB_START, 0x00);
        assert_eq!(SUB_REFRESH, 0x01);
        assert_eq!(SUB_KILL_UPDATE, 0x02);
        assert_eq!(SUB_COUNT_UPDATE, 0x03);
        assert_eq!(SUB_END, 0x04);
        assert_eq!(SUB_HIDE, 0x05);
    }

    /// CR_SLOTS and CR_REWARD_SLOTS are both 3.
    #[test]
    fn test_cr_slots_equal() {
        assert_eq!(CR_SLOTS, 3);
        assert_eq!(CR_REWARD_SLOTS, 3);
        assert_eq!(CR_SLOTS, CR_REWARD_SLOTS);
    }

    /// EXT_SUB_COLLECTION_RACE matches ext_hook constant.
    #[test]
    fn test_ext_sub_collection_race_value() {
        assert_eq!(EXT_SUB_COLLECTION_RACE, 0xAF);
    }

    /// CR event monster proto_ids default to zero.
    #[test]
    fn test_cr_event_monster_defaults() {
        let ev = CollectionRaceEvent::default();
        assert_eq!(ev.proto_ids, [0; CR_SLOTS]);
        assert_eq!(ev.kill_counts_req, [0; CR_SLOTS]);
    }

    /// CR end packet is minimal (just sub-opcode).
    #[test]
    fn test_cr_end_packet_minimal() {
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_COLLECTION_RACE);
        pkt.write_u8(SUB_END);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], EXT_SUB_COLLECTION_RACE);
        assert_eq!(pkt.data[1], SUB_END);
    }

    /// CR sub-opcodes: 0x00-0x05 form a contiguous 6-value range.
    #[test]
    fn test_cr_sub_opcodes_contiguous_range() {
        assert_eq!(SUB_START, 0x00);
        assert_eq!(SUB_REFRESH, 0x01);
        assert_eq!(SUB_KILL_UPDATE, 0x02);
        assert_eq!(SUB_COUNT_UPDATE, 0x03);
        assert_eq!(SUB_END, 0x04);
        assert_eq!(SUB_HIDE, 0x05);
        // Exactly 6 sub-opcodes, contiguous
        assert_eq!(SUB_HIDE - SUB_START, 5);
    }

    /// CrUserList default: all zeros, not finished, status 0.
    #[test]
    fn test_cr_user_list_fields_default() {
        let user = CrUserList::default();
        assert_eq!(user.kill_counts, [0, 0, 0]);
        assert!(!user.is_finish);
        assert_eq!(user.user_status, 0);
        // 3 kill count slots match CR_SLOTS
        assert_eq!(user.kill_counts.len(), CR_SLOTS);
    }

    /// Virtual item IDs used as rewards: ITEM_GOLD, ITEM_EXP, ITEM_COUNT, ITEM_RANDOM.
    #[test]
    fn test_cr_virtual_reward_item_ids() {
        assert_eq!(ITEM_GOLD, 900_000_000);
        assert_eq!(ITEM_EXP, 900_001_000);
        assert_eq!(ITEM_COUNT, 900_002_000);
        assert_eq!(ITEM_RANDOM, 900_004_000);
        // All in 900M range
        assert!(ITEM_GOLD >= 900_000_000 && ITEM_GOLD < 901_000_000);
        assert!(ITEM_RANDOM >= 900_000_000 && ITEM_RANDOM < 901_000_000);
    }

    /// CollectionRaceEvent default state: not active, no participants.
    #[test]
    fn test_cr_event_inactive_default() {
        let event = CollectionRaceEvent::default();
        assert!(!event.is_active);
        assert_eq!(event.total_count, 0);
        assert_eq!(event.user_limit, 0);
        assert_eq!(event.event_end_time, 0);
        assert_eq!(event.zone_id, 0);
    }

    /// CR hide packet is same size as end packet (minimal 2-byte body).
    #[test]
    fn test_cr_hide_packet_same_size_as_end() {
        let mut end_pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        end_pkt.write_u8(EXT_SUB_COLLECTION_RACE);
        end_pkt.write_u8(SUB_END);

        let mut hide_pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        hide_pkt.write_u8(EXT_SUB_COLLECTION_RACE);
        hide_pkt.write_u8(SUB_HIDE);

        assert_eq!(end_pkt.data.len(), hide_pkt.data.len());
        assert_ne!(end_pkt.data[1], hide_pkt.data[1]); // Different sub-opcode
    }
}
