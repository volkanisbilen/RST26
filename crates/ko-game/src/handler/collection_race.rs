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
use crate::world::{WorldState, ITEM_GOLD, ZONE_ARDREAM, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE};
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

/// Reserved "proto_id" an admin can put in a CR target slot to mean "a kill in
/// the PK zones (Ronark Land / Ardream / Ronark Land Base), real player or PK
/// bot, counts toward this target" instead of a specific monster. The admin
/// panel's target SID field is i16 (DB column `smallint`), so this has to fit
/// that range -- `i16::MAX` rather than `u16::MAX`. `validate_collection_race_setting`
/// (ko-admin, legacy.rs) special-cases this exact value to skip the normal
/// "must be a real npc_template.s_sid" check.
pub const CR_PK_BOT_KILL_PROTO_ID: u16 = i16::MAX as u16;

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

/// Append the optional proxy meta block (`u8 1`, target names + icons, reward names + icons,
/// zone name) so the client panel shows monster/item names and item icons instead of ids.
pub fn append_meta(pkt: &mut Packet, world: &WorldState, ev: &CollectionRaceEvent) {
    pkt.write_u8(1);
    for i in 0..CR_SLOTS {
        let name = if ev.proto_ids[i] == 0 {
            String::new()
        } else {
            world
                .get_npc_template(ev.proto_ids[i], true)
                .or_else(|| world.get_npc_template(ev.proto_ids[i], false))
                .map(|t| t.name.clone())
                .unwrap_or_else(|| format!("Monster {}", ev.proto_ids[i]))
        };
        pkt.write_string(&name);
        pkt.write_u32(0);
    }
    for i in 0..CR_REWARD_SLOTS {
        let id = ev.reward_item_ids[i];
        let (name, icon) = match world.get_item(id) {
            Some(it) => (
                it.str_name.clone().unwrap_or_default(),
                it.item_icon_id1.or(it.item_icon_id2).unwrap_or(0).max(0) as u32,
            ),
            None if id == ITEM_GOLD => ("Gold".to_string(), 0),
            None if id == ITEM_EXP => ("Experience".to_string(), 0),
            None => (String::new(), 0),
        };
        pkt.write_string(&name);
        pkt.write_u32(icon);
    }
    let zone = world
        .get_zone(ev.zone_id as u16)
        .and_then(|z| z.zone_info.as_ref().map(|i| i.zone_name.clone()))
        .unwrap_or_default();
    pkt.write_string(&zone);
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
                let remaining = ev.event_end_time.saturating_sub(now);
                let mut pkt = build_refresh_packet(&ev, my_kills, 0, remaining, nation);
                append_meta(&mut pkt, &world, &ev);
                Some(pkt)
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

/// Credit a real (non-bot) opposing-nation player kill toward Collection Race,
/// using the same PK-zone sentinel a PK bot kill uses (`CR_PK_BOT_KILL_PROTO_ID`'s
/// doc comment always described this as covering "real player or PK bot" —
/// only the bot half was ever wired up). Call this from every PvP kill
/// resolution site (melee, skill damage) alongside the existing
/// `set_who_killed_me`/`pvp_loyalty_on_death` calls.
pub async fn credit_pk_zone_player_kill(
    world: &WorldState,
    killer_sid: SessionId,
    victim_sid: SessionId,
    zone_id: u16,
) {
    if !matches!(
        zone_id,
        ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
    ) {
        return;
    }
    let same_nation = world
        .get_character_info(killer_sid)
        .zip(world.get_character_info(victim_sid))
        .is_some_and(|(killer, victim)| killer.nation == victim.nation);
    if same_nation {
        return;
    }
    let cr = world.collection_race_event().clone();
    if let Err(e) = handle_kill(world, killer_sid, CR_PK_BOT_KILL_PROTO_ID, &cr).await {
        warn!("CR handle_kill error (PK zone player kill): {}", e);
    }
}

/// Max distance (game units, per axis-plane) between the killer and a party
/// member for the member to share the Collection Race kill credit.
const CR_PARTY_SHARE_RANGE: f32 = 100.0;

/// Outcome of counting one kill for one player (pure state transition).
#[derive(Debug, Clone, PartialEq)]
struct CrKillOutcome {
    kill_counts: [u16; CR_SLOTS],
    /// The player completed a round with this kill.
    finished: bool,
    /// The player is done for good (single-finish event): close their UI.
    close_for_player: bool,
}

/// Count `proto_id` for player `name` in the event. Progress lives only in
/// `ev.user_list` (reset together with the event). Repeat events
/// (`event_list_status != 0`) restart the player's progress after each
/// completion with the SAME target, until the event ends; `user_status`
/// counts completed rounds. Returns `None` when the kill does not count.
fn apply_kill(ev: &mut CollectionRaceEvent, name: &str, proto_id: u16) -> Option<CrKillOutcome> {
    let req = ev.kill_counts_req;
    let proto_ids = ev.proto_ids;
    let repeat = ev.event_list_status != 0;
    let entry = ev.user_list.entry(name.to_string()).or_default();
    if entry.is_finish {
        return None;
    }
    let mut matched = false;
    for i in 0..CR_SLOTS {
        if proto_ids[i] == 0 || proto_ids[i] != proto_id || req[i] == 0 {
            continue;
        }
        if entry.kill_counts[i] < req[i] {
            entry.kill_counts[i] += 1;
            matched = true;
        }
    }
    if !matched {
        return None;
    }
    let kill_counts = entry.kill_counts;
    let all_done = (0..CR_SLOTS).all(|i| req[i] == 0 || kill_counts[i] >= req[i]);
    if !all_done {
        return Some(CrKillOutcome {
            kill_counts,
            finished: false,
            close_for_player: false,
        });
    }
    entry.user_status = entry.user_status.saturating_add(1);
    let close_for_player = !repeat;
    if close_for_player {
        entry.is_finish = true;
    } else {
        entry.kill_counts = [0; CR_SLOTS];
    }
    ev.total_count = ev.total_count.saturating_add(1);
    Some(CrKillOutcome {
        kill_counts,
        finished: true,
        close_for_player,
    })
}

/// Players who get credit for a kill by `killer`: the killer plus every party
/// member that is alive, in the same zone and within `CR_PARTY_SHARE_RANGE`.
/// `members` = (sid, alive, Some((zone, x, z))). The killer is never duplicated.
fn share_recipients(
    killer: SessionId,
    killer_pos: Option<(u16, f32, f32)>,
    members: &[(SessionId, bool, Option<(u16, f32, f32)>)],
) -> Vec<SessionId> {
    let mut out = vec![killer];
    let Some((kz, kx, kzz)) = killer_pos else {
        return out;
    };
    for &(sid, alive, pos) in members {
        if sid == killer || out.contains(&sid) || !alive {
            continue;
        }
        if let Some((z, x, zz)) = pos {
            let d = ((x - kx).powi(2) + (zz - kzz).powi(2)).sqrt();
            if z == kz && d <= CR_PARTY_SHARE_RANGE {
                out.push(sid);
            }
        }
    }
    out
}

/// Process a monster / PK-zone kill for the Collection Race. The kill counts
/// for the killer and for every eligible party member (see `share_recipients`).
pub async fn handle_kill(
    world: &WorldState,
    killer_sid: SessionId,
    proto_id: u16,
    cr: &SharedCollectionRaceEvent,
) -> anyhow::Result<()> {
    if !cr.read().is_active {
        return Ok(());
    }
    let pos_of = |sid: SessionId| world.get_position(sid).map(|p| (p.zone_id, p.x, p.z));
    let mut members = Vec::new();
    if let Some(party) = world
        .get_party_id(killer_sid)
        .and_then(|pid| world.get_party(pid))
    {
        for m in party.members.iter().filter_map(|&m| m) {
            let alive = world.get_character_info(m).is_some_and(|c| c.hp > 0);
            members.push((m, alive, pos_of(m)));
        }
    }
    for sid in share_recipients(killer_sid, pos_of(killer_sid), &members) {
        handle_kill_single(world, sid, proto_id, cr).await?;
    }
    Ok(())
}

/// Count one kill for a single player (no party logic).
async fn handle_kill_single(
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

    let (outcome, rewards, new_total, rank_bug, user_limit) = {
        let mut ev = cr.write();
        if !ev.is_active || level < ev.min_level || level > ev.max_level {
            return Ok(());
        }
        let Some(outcome) = apply_kill(&mut ev, &char_name, proto_id) else {
            return Ok(());
        };
        (
            outcome,
            (
                ev.reward_item_ids,
                ev.reward_item_counts,
                ev.reward_item_times,
                ev.reward_item_rates,
            ),
            ev.total_count,
            ev.rank_bug,
            ev.user_limit,
        )
    }; // lock released
    let (reward_ids, reward_counts, reward_times, reward_rates) = rewards;
    let event_ended = outcome.finished && new_total >= user_limit;

    if !outcome.finished {
        world.send_to_session_owned(
            killer_sid,
            build_kill_update_packet(proto_id, outcome.kill_counts),
        );
        return Ok(());
    }
    let kill_counts = outcome.kill_counts;
    let should_close_for_player = outcome.close_for_player;
    {
        {
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
            } else {
                // Repeat event: the next round starts from zero.
                world.send_to_session_owned(
                    killer_sid,
                    build_kill_update_packet(proto_id, [0u16; CR_SLOTS]),
                );
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

// ─────────────────────────────────────────────────────────────────────────────
// Automatic start schedule
// ─────────────────────────────────────────────────────────────────────────────
//
// Every `collection_race_settings` row carries `auto_start` / `auto_hour` /
// `auto_minute` (edited on the admin panel's Collection Race page). The row
// fires every day at that server-local time -- the same clock the generic
// `event_auto_schedule` table uses (`event_control::tick`). The table has no
// weekday column, so "every day" is the only mode; twelve rows two hours apart
// give twelve runs a day around the clock.
//
// Like the original C++ timer, a scheduled start is skipped while another
// Collection Race is still running (it never replaces a running event), and when
// two rows share the same minute the lowest `event_index` wins.

/// Whether `def` is due at server-local `hour:minute` (all values validated).
pub fn auto_start_due(def: &CrEventDef, hour: i32, minute: i32) -> bool {
    def.auto_start
        && (0..24).contains(&def.auto_hour)
        && (0..60).contains(&def.auto_minute)
        && def.auto_hour == hour
        && def.auto_minute == minute
        && def.event_time_mins > 0
        && def
            .proto_ids
            .iter()
            .zip(def.kill_counts.iter())
            .any(|(&proto, &count)| proto != 0 && count != 0)
}

/// The event to start at `hour:minute`, if any (lowest event id on a tie).
pub fn resolve_auto_start(defs: &[CrEventDef], hour: i32, minute: i32) -> Option<i16> {
    defs.iter()
        .filter(|def| auto_start_due(def, hour, minute))
        .map(|def| def.event_id)
        .min()
}

/// Last wall-clock minute (unix seconds / 60) the schedule was evaluated for.
static LAST_AUTO_MINUTE: std::sync::atomic::AtomicI64 =
    std::sync::atomic::AtomicI64::new(i64::MIN);

/// Called from the one-second event tick; evaluates the schedule once per
/// wall-clock minute and starts the due event through the same path as the
/// admin panel / `+cropen` (`event_control::start("cr", <event index>)`).
pub fn auto_start_tick(world: &Arc<WorldState>) {
    use chrono::{Local, Timelike};
    use std::sync::atomic::Ordering;

    let now = Local::now();
    let minute_key = now.timestamp() / 60;
    if LAST_AUTO_MINUTE.swap(minute_key, Ordering::Relaxed) == minute_key {
        return;
    }

    let defs: Vec<CrEventDef> = world
        .collection_race_settings
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    let Some(event_id) = resolve_auto_start(&defs, now.hour() as i32, now.minute() as i32) else {
        return;
    };

    if world.collection_race_event().read().is_active {
        warn!(
            "[CR] scheduled start of event {} skipped: another Collection Race is still running",
            event_id
        );
        return;
    }
    let Some(pool) = world.db_pool().cloned() else {
        return;
    };

    let world = world.clone();
    tokio::spawn(async move {
        match crate::event_control::start(&world, &pool, "cr", &event_id.to_string(), false).await {
            Ok(text) => info!("[CR] scheduled start of event {}: {}", event_id, text),
            Err(e) => warn!("[CR] scheduled start of event {} failed: {}", event_id, e),
        }
    });
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
                let mut pkt = build_start_packet(&ev, duration_secs, nation);
                append_meta(&mut pkt, world, &ev);
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

/// Send the start packet to every eligible online player and reset their per-user CR state.
/// Used by `+cropen` after `start_event` so the client panel opens.
pub fn broadcast_start(world: &WorldState, duration_secs: u32) {
    let cr = world.collection_race_event();
    let ev = cr.read();
    for sid in world.all_ingame_session_ids() {
        let (level, nation) = match world.get_character_info(sid) {
            Some(c) => (c.level, c.nation),
            None => continue,
        };
        if level < ev.min_level || level > ev.max_level {
            continue;
        }
        let mut pkt = build_start_packet(&ev, duration_secs, nation);
        append_meta(&mut pkt, world, &ev);
        world.send_to_session_owned(sid, pkt);
        world.update_session(sid, |h| {
            h.cr_kill_counts = [0u16; CR_SLOTS];
            h.cr_check_finish = false;
        });
    }
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

    fn repeat_event(status: u8) -> CollectionRaceEvent {
        let mut ev = CollectionRaceEvent::default();
        ev.is_active = true;
        ev.event_list_status = status;
        ev.proto_ids = [100, 0, 0];
        ev.kill_counts_req = [50, 0, 0];
        ev.user_limit = 5000;
        ev
    }

    #[test]
    fn test_repeat_event_pays_every_round_with_same_target() {
        let mut ev = repeat_event(1);
        for round in 1..=3u16 {
            for k in 1..=50u16 {
                let o = apply_kill(&mut ev, "A", 100).expect("counts");
                assert_eq!(o.finished, k == 50, "round {round} kill {k}");
                assert!(!o.close_for_player);
            }
            assert_eq!(ev.total_count, round);
            assert_eq!(ev.user_list["A"].kill_counts, [0; CR_SLOTS]);
            assert!(!ev.user_list["A"].is_finish);
        }
    }

    #[test]
    fn test_single_finish_event_closes_after_first_round() {
        let mut ev = repeat_event(0);
        let mut last = None;
        for _ in 0..50 {
            last = apply_kill(&mut ev, "A", 100);
        }
        let o = last.unwrap();
        assert!(o.finished && o.close_for_player);
        assert!(apply_kill(&mut ev, "A", 100).is_none());
    }

    #[test]
    fn test_unrelated_proto_does_not_count() {
        let mut ev = repeat_event(1);
        assert!(apply_kill(&mut ev, "A", 999).is_none());
    }

    #[test]
    fn test_share_recipients_party_zone_range_no_double() {
        let near = Some((71u16, 105.0, 100.0));
        let far = Some((71u16, 900.0, 100.0));
        let other_zone = Some((21u16, 100.0, 100.0));
        let members = [
            (1, true, Some((71u16, 100.0, 100.0))), // killer himself
            (2, true, near),
            (3, true, far),
            (4, true, other_zone),
            (5, false, near), // dead
            (2, true, near),  // duplicate entry
        ];
        let r = share_recipients(1, Some((71, 100.0, 100.0)), &members);
        assert_eq!(r, vec![1, 2]);
        assert_eq!(share_recipients(1, None, &members), vec![1]);
        assert_eq!(share_recipients(1, Some((71, 0.0, 0.0)), &[]), vec![1]);
    }

    #[test]
    fn test_pk_sentinel_counts_for_party_members() {
        let mut ev = repeat_event(1);
        ev.proto_ids = [CR_PK_BOT_KILL_PROTO_ID, 0, 0];
        for sid in share_recipients(1, Some((71, 0.0, 0.0)), &[(2, true, Some((71, 1.0, 1.0)))]) {
            apply_kill(&mut ev, &format!("P{sid}"), CR_PK_BOT_KILL_PROTO_ID).unwrap();
        }
        assert_eq!(ev.user_list["P1"].kill_counts[0], 1);
        assert_eq!(ev.user_list["P2"].kill_counts[0], 1);
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

    // ── Automatic start schedule / timer ───────────────────────────────────

    fn scheduled_def(event_id: i16, hour: i32, minute: i32) -> CrEventDef {
        CrEventDef {
            event_id,
            event_name: format!("CR {event_id}"),
            proto_ids: [CR_PK_BOT_KILL_PROTO_ID, 0, 0],
            kill_counts: [30, 0, 0],
            min_level: 1,
            max_level: 83,
            zone_id: 71,
            event_time_mins: 90,
            user_limit: 5000,
            event_list_status: 1,
            auto_start: true,
            auto_hour: hour,
            auto_minute: minute,
        }
    }

    /// The live schedule: twelve events, one every two hours at HH:15.
    fn twelve_a_day() -> Vec<CrEventDef> {
        (1..=12)
            .map(|i| scheduled_def(i as i16, (i - 1) * 2, 15))
            .collect()
    }

    #[test]
    fn test_auto_start_due_matches_exact_minute_only() {
        let def = scheduled_def(3, 4, 15);
        assert!(auto_start_due(&def, 4, 15));
        assert!(!auto_start_due(&def, 4, 14));
        assert!(!auto_start_due(&def, 4, 16));
        assert!(!auto_start_due(&def, 5, 15));
        assert!(!auto_start_due(&def, 16, 15));
    }

    #[test]
    fn test_auto_start_due_requires_flag_and_valid_fields() {
        let mut def = scheduled_def(1, 0, 15);
        def.auto_start = false;
        assert!(!auto_start_due(&def, 0, 15));

        // -1 is the "unset" sentinel the C++ port used; out-of-range values never fire
        for (h, m) in [(-1, 15), (24, 15), (0, -1), (0, 60)] {
            let d = scheduled_def(1, h, m);
            assert!(!auto_start_due(&d, h, m), "{h}:{m} must never fire");
        }

        // an event with no time or no kill target would start and end instantly
        let mut no_time = scheduled_def(1, 0, 15);
        no_time.event_time_mins = 0;
        assert!(!auto_start_due(&no_time, 0, 15));
        let mut no_target = scheduled_def(1, 0, 15);
        no_target.kill_counts = [0, 0, 0];
        assert!(!auto_start_due(&no_target, 0, 15));
        let mut no_proto = scheduled_def(1, 0, 15);
        no_proto.proto_ids = [0, 0, 0];
        assert!(!auto_start_due(&no_proto, 0, 15));
    }

    #[test]
    fn test_resolve_auto_start_lowest_event_id_wins_on_tie() {
        let defs = vec![
            scheduled_def(9, 8, 15),
            scheduled_def(4, 8, 15),
            scheduled_def(6, 8, 15),
        ];
        assert_eq!(resolve_auto_start(&defs, 8, 15), Some(4));
        assert_eq!(resolve_auto_start(&defs, 8, 16), None);
        assert_eq!(resolve_auto_start(&[], 8, 15), None);
    }

    #[test]
    fn test_twelve_events_fire_every_two_hours_around_the_clock() {
        let defs = twelve_a_day();
        let mut fired: Vec<(i32, i32, i16)> = Vec::new();
        for hour in 0..24 {
            for minute in 0..60 {
                if let Some(id) = resolve_auto_start(&defs, hour, minute) {
                    fired.push((hour, minute, id));
                }
            }
        }
        assert_eq!(fired.len(), 12, "exactly 12 runs a day");
        for (n, &(hour, minute, id)) in fired.iter().enumerate() {
            assert_eq!(hour, n as i32 * 2, "run {n} starts on an even hour");
            assert_eq!(minute, 15);
            assert_eq!(id as usize, n + 1, "event ids rotate in order");
        }
        // gaps of exactly two hours, including the wrap 22:15 -> 00:15
        for pair in fired.windows(2) {
            assert_eq!(pair[1].0 - pair[0].0, 2);
        }
        assert_eq!((fired[0].0 + 24) - fired[11].0, 2);
    }

    #[test]
    fn test_scheduled_event_fits_inside_the_two_hour_slot() {
        // 90-minute events leave a 30-minute gap; the next start therefore never
        // meets a still-running event.
        let defs = twelve_a_day();
        assert!(defs.iter().all(|d| d.event_time_mins < 120));
    }

    #[test]
    fn test_migration_schedule_matches_the_twelve_a_day_pattern() {
        // The data migration is the source of truth for the live schedule: it must
        // list events 1..=12 on 00:15, 02:15 ... 22:15.
        let sql = include_str!(
            "../../../../migrations/20261018140000_collection_race_schedule_rewards.sql"
        );
        let start = sql
            .find("-- SCHEDULE-BEGIN")
            .expect("schedule marker (begin)");
        let end = sql.find("-- SCHEDULE-END").expect("schedule marker (end)");
        let mut slots: Vec<(i32, i32)> = Vec::new();
        for line in sql[start..end].lines() {
            let t = line.trim().trim_end_matches(',');
            if let Some(rest) = t.strip_prefix('(') {
                let nums: Vec<i32> = rest
                    .trim_end_matches(')')
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                if nums.len() == 3 {
                    slots.push((nums[0], nums[1] * 100 + nums[2]));
                }
            }
        }
        assert_eq!(slots.len(), 12);
        for (i, &(event, hhmm)) in slots.iter().enumerate() {
            assert_eq!(event, i as i32 + 1);
            assert_eq!(hhmm, i as i32 * 200 + 15, "event {event} at HH:15 on an even hour");
        }
    }

    #[test]
    fn test_tick_timer_ends_the_event_exactly_when_time_is_up() {
        let world = crate::world::WorldState::new();
        let cr = new_collection_race_event();
        let def = scheduled_def(1, 0, 15);
        assert!(start_event(&cr, &def, &[], 1, 10_000));
        assert_eq!(cr.read().event_end_time, 10_000 + 90 * 60);

        // one second before the end: still running
        tick_timer(&world, &cr, 10_000 + 90 * 60 - 1);
        assert!(cr.read().is_active);

        // at the end: closed and reset
        tick_timer(&world, &cr, 10_000 + 90 * 60);
        assert!(!cr.read().is_active);
        assert_eq!(cr.read().event_end_time, 0);
        // a closed event stays closed
        tick_timer(&world, &cr, 10_000 + 90 * 60 + 5);
        assert!(!cr.read().is_active);
    }

    #[test]
    fn test_tick_timer_late_tick_still_closes() {
        let world = crate::world::WorldState::new();
        let cr = new_collection_race_event();
        assert!(start_event(&cr, &scheduled_def(2, 2, 15), &[], 1, 50_000));
        tick_timer(&world, &cr, 50_000 + 90 * 60 + 3600);
        assert!(!cr.read().is_active);
    }

    #[test]
    fn test_tick_timer_announcement_does_not_close() {
        let world = crate::world::WorldState::new();
        let cr = new_collection_race_event();
        assert!(start_event(&cr, &scheduled_def(2, 2, 15), &[], 1, 50_000));
        // 15 minutes before the end is only an announcement
        tick_timer(&world, &cr, 50_000 + 90 * 60 - 900);
        assert!(cr.read().is_active);
    }

    #[test]
    fn test_start_event_rejects_oversized_rewards() {
        let cr = new_collection_race_event();
        let def = scheduled_def(1, 0, 15);
        // box counts above 9999 and gold/exp above 9 999 999 are refused
        assert!(!start_event(&cr, &def, &[(999_915_000, 10_000, 0, 100, 0)], 1, 1));
        assert!(!start_event(&cr, &def, &[(ITEM_GOLD, 10_000_000, 0, 100, 0)], 1, 1));
        assert!(!cr.read().is_active);
        assert!(start_event(&cr, &def, &[(999_915_000, 2, 0, 60, 0)], 1, 1));
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
