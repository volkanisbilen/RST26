//! Lottery Event handler — WIZ_EXT_HOOK (0xE9) sub-opcode LOTTERY (0xC7).
//! ## Overview
//! The Lottery Event is a server-wide timed event where players spend required
//! items/gold to earn lottery tickets. When time expires, up to 4 random winners
//! are drawn from participants and each receives a reward item via the in-game
//! letter system.
//! ## Sub-opcode flow (ExtSub::LOTTERY = 0xC7)
//! | Client→Server sub | Description              |
//! |--------------------|--------------------------|
//! | 3                  | Join lottery (buy ticket) |
//! | Server→Client sub | Description                             |
//! |--------------------|-----------------------------------------|
//! | 1                  | Event started / current state broadcast |
//! | 2                  | Participant count updated (all)         |
//! | 3                  | Join result (success/fail + ticket count) |
//! | 4                  | Event ended / reset                     |
//! ## Packet formats
//! ### Start broadcast (sub=1, sent to all or on game entry)
//! ```text
//! WIZ_EXT_HOOK << u8(0xC7) << u8(1)
//!   << [5x: u32 req_item_id, u32 req_item_count]
//!   << [4x: u32 reward_item_id]
//!   << u32(user_limit) << u32(remaining_secs) << u32(join_count) << u32(my_ticket_count)
//! ```
//! ### Participant count update (sub=2, broadcast to all)
//! ```text
//! WIZ_EXT_HOOK << u8(0xC7) << u8(2)
//! ```
//! ### Join result (sub=3, sent to joining player)
//! ```text
//! WIZ_EXT_HOOK << u8(0xC7) << u8(3) << u8(result) << u32(ticket_count_or_error_msg)
//! result: 0 = fail (followed by length-prefixed error string), 1 = success (followed by u32 ticket_count)
//! ```
//! ### End broadcast (sub=4, sent to all on reset)
//! ```text
//! WIZ_EXT_HOOK << u8(0xC7) << u8(4)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use rand::Rng;
use tracing::{debug, info, warn};

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::handler::chat::{build_chat_packet, ChatType};
use crate::session::{ClientSession, SessionState};
use crate::world::ITEM_GOLD;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) use super::ext_hook::EXT_SUB_LOTTERY;

/// Sub-opcode sent by client to join the lottery.
pub const SUB_JOIN: u8 = 3;

/// Sub-opcode sent to all players when event starts.
pub const SUB_START: u8 = 1;

/// Sub-opcode broadcast to all when participant count changes.
pub const SUB_COUNT_UPDATE: u8 = 2;

/// Sub-opcode sent to joining player with result (success/fail).
pub const SUB_JOIN_RESULT: u8 = 3;

/// Sub-opcode broadcast to all when event ends/resets.
pub const SUB_END: u8 = 4;

/// Maximum number of winners drawn per lottery.
pub const MAX_WINNERS: usize = 4;

/// Maximum number of required items per lottery.
pub const MAX_REQ_ITEMS: usize = 5;

/// Maximum number of reward items per lottery.
pub const MAX_REWARD_ITEMS: usize = 4;

// ─────────────────────────────────────────────────────────────────────────────
// Lottery State
// ─────────────────────────────────────────────────────────────────────────────

/// Per-user lottery state (mirrors `_RIMA_LOOTERY_USER_INFO`).
#[derive(Debug, Clone)]
pub struct LotteryUserInfo {
    /// In-game character name (uppercase).
    pub name: String,
    /// Number of tickets purchased.
    pub ticket_count: u32,
    /// Whether this user already received a gift this event.
    pub is_gift: bool,
}

/// Global lottery runtime state (mirrors `_RIMA_LOTTERY_PROCESS`).
#[derive(Debug, Clone)]
pub struct LotteryProcess {
    /// Whether the event is currently active.
    pub lottery_start: bool,
    /// Whether the timer is ticking.
    pub timer_control: bool,
    /// Whether gift distribution is pending.
    pub send_gift_activate: bool,
    /// Required items (up to 5 slots): (item_id, count).
    pub req_items: [(u32, u32); MAX_REQ_ITEMS],
    /// Reward items (up to 4 slots): item_id (0 = empty).
    pub reward_items: [u32; MAX_REWARD_ITEMS],
    /// Maximum number of participants allowed.
    pub user_limit: u32,
    /// Unix timestamp of event start.
    pub event_start_time: u32,
    /// Unix timestamp of event end.
    pub event_process_time: u32,
    /// Total duration in seconds.
    pub event_time: u32,
    /// Current participant count.
    pub user_join_counter: u32,
    /// All participants keyed by uppercase name.
    pub participants: HashMap<String, LotteryUserInfo>,
}

impl Default for LotteryProcess {
    fn default() -> Self {
        Self {
            lottery_start: false,
            timer_control: false,
            send_gift_activate: false,
            req_items: [(0, 0); MAX_REQ_ITEMS],
            reward_items: [0u32; MAX_REWARD_ITEMS],
            user_limit: 0,
            event_start_time: 0,
            event_process_time: 0,
            event_time: 0,
            user_join_counter: 0,
            participants: HashMap::new(),
        }
    }
}

impl LotteryProcess {
    /// Return remaining seconds until event ends.
    pub fn remaining_secs(&self, now: u32) -> u32 {
        self.event_process_time.saturating_sub(now)
    }
}

/// Thread-safe shared lottery process state.
/// Wrapped in `Arc<RwLock<...>>` to allow access from multiple
/// handler calls and background timer task.
pub type SharedLotteryProcess = Arc<RwLock<LotteryProcess>>;

/// Create a new default lottery process.
pub fn new_lottery_process() -> SharedLotteryProcess {
    Arc::new(RwLock::new(LotteryProcess::default()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Packet Builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build a lottery start/state packet (sub=1).
/// Wire format (all little-endian):
/// ```text
/// WIZ_EXT_HOOK(0xE9) << u8(0xC7) << u8(1)
///   << [5x: u32 req_item_id << u32 req_item_count]
///   << [4x: u32 reward_item_id]
///   << u32(user_limit) << u32(remaining_secs) << u32(join_count) << u32(my_ticket_count)
/// ```
pub fn build_start_packet(proc: &LotteryProcess, now: u32, my_ticket_count: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_LOTTERY);
    pkt.write_u8(SUB_START);

    for (item_id, item_count) in &proc.req_items {
        pkt.write_u32(*item_id);
        pkt.write_u32(*item_count);
    }
    for reward_id in &proc.reward_items {
        pkt.write_u32(*reward_id);
    }

    pkt.write_u32(proc.user_limit);
    pkt.write_u32(proc.remaining_secs(now));
    pkt.write_u32(proc.user_join_counter);
    pkt.write_u32(my_ticket_count);
    pkt
}

/// Build participant count update broadcast (sub=2).
/// Wire format:
/// ```text
/// WIZ_EXT_HOOK(0xE9) << u8(0xC7) << u8(2)
/// ```
pub fn build_count_update_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_LOTTERY);
    pkt.write_u8(SUB_COUNT_UPDATE);
    pkt
}

/// Build a join-result success packet (sub=3, result=1).
/// Wire format:
/// ```text
/// WIZ_EXT_HOOK(0xE9) << u8(0xC7) << u8(3) << u8(1) << u32(ticket_count)
/// ```
pub fn build_join_success_packet(ticket_count: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_LOTTERY);
    pkt.write_u8(SUB_JOIN_RESULT);
    pkt.write_u8(1); // success
    pkt.write_u32(ticket_count);
    pkt
}

/// Build a join-result failure packet (sub=3, result=0) with message.
/// Wire format:
/// ```text
/// WIZ_EXT_HOOK(0xE9) << u8(0xC7) << u8(3) << u8(0) << sbyte_string(msg)
/// ```
pub fn build_join_fail_packet(msg: &str) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_LOTTERY);
    pkt.write_u8(SUB_JOIN_RESULT);
    pkt.write_u8(0); // fail
    pkt.write_string(msg);
    pkt
}

/// Build event-end broadcast packet (sub=4).
/// Wire format:
/// ```text
/// WIZ_EXT_HOOK(0xE9) << u8(0xC7) << u8(4)
/// ```
pub fn build_end_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_LOTTERY);
    pkt.write_u8(SUB_END);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Chat helper (mirrors C++ SendChat<PUBLIC_CHAT>)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a server-wide PUBLIC_CHAT announcement packet (type=7).
/// Uses `ChatType::Public = 7`, nation=ALL(0), sender_id=0, name="SYSTEM".
pub fn build_lottery_announce(msg: &str) -> Packet {
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
// Game-entry: send current lottery state to a newly logged-in player
// ─────────────────────────────────────────────────────────────────────────────

/// Send lottery state to a player on game entry.
/// Called from `gamestart` when a player enters the world while a lottery
/// event is active. Mirrors `CUser::LotteryGameStartSend()`.
pub async fn send_on_game_entry(
    session: &mut ClientSession,
    lottery: &SharedLotteryProcess,
) -> anyhow::Result<()> {
    // Build the packet inside a block so the lock guard is dropped
    // before any await point (avoids holding parking_lot lock across .await).
    let maybe_pkt = {
        let proc_guard = lottery.read();

        if !proc_guard.lottery_start || proc_guard.event_time == 0 {
            None
        } else {
            let char_name = session
                .world()
                .get_character_info(session.session_id())
                .map(|c| c.name.to_uppercase())
                .unwrap_or_default();

            let my_tickets = proc_guard
                .participants
                .get(&char_name)
                .map(|p| p.ticket_count)
                .unwrap_or(0);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;

            Some((build_start_packet(&proc_guard, now, my_tickets), my_tickets))
        }
        // proc_guard dropped here
    };

    if let Some((pkt, my_tickets)) = maybe_pkt {
        session.send_packet(&pkt).await?;
        // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
        let remaining = {
            let proc = lottery.read();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            proc.remaining_secs(now)
        };
        let chat_msg = format!(
            "[Lottery] Event active! {} min remaining. Your tickets: {}",
            remaining / 60,
            my_tickets
        );
        let chat_pkt = crate::systems::timed_notice::build_notice_packet(7, &chat_msg);
        session.send_packet(&chat_pkt).await?;
        debug!(
            "[{}] Sent lottery game-entry state (my_tickets={})",
            session.addr(),
            my_tickets
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handle client join request
// ─────────────────────────────────────────────────────────────────────────────

/// Handle a LOTTERY join request from the client (sub=3).
/// and `CUser::LotteryJoinFunction()`
/// Flow:
/// 1. Check lottery is active.
/// 2. Check user-limit not exceeded.
/// 3. Verify player has all required items (CheckExistItem per slot).
/// 4. Deduct gold first (GoldLose), then non-gold items (RobItem).
/// 5. Add ticket, increment counter.
/// 6. Send success to player + count-update broadcast.
pub async fn handle_join(
    session: &mut ClientSession,
    lottery: &SharedLotteryProcess,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // ── 1. Validate session state ─────────────────────────────────────────
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let char_info = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    let char_name = char_info.name.to_uppercase();

    // ── 2. Check lottery active + user limit ─────────────────────────────
    // Build fail packet inside a block so the lock is dropped before .await.
    let pre_check_fail: Option<Packet> = {
        let proc = lottery.read();
        if !proc.lottery_start {
            Some(build_join_fail_packet("The Lottery Event has not started."))
        } else if proc.user_join_counter >= proc.user_limit {
            Some(build_join_fail_packet("Limit is insufficient."))
        } else {
            None
        }
        // proc dropped here
    };
    if let Some(pkt) = pre_check_fail {
        session.send_packet(&pkt).await?;
        // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
        let fail_msg = if !lottery.read().lottery_start {
            "[Lottery] The event has not started."
        } else {
            "[Lottery] Limit is insufficient."
        };
        let chat_pkt = crate::systems::timed_notice::build_notice_packet(7, fail_msg);
        session.send_packet(&chat_pkt).await?;
        return Ok(());
    }

    // ── 4. Snapshot required items (avoid holding lock during inventory ops) ─
    let (req_items, reward_items_snapshot) = {
        let proc = lottery.read();
        (proc.req_items, proc.reward_items)
    };
    let _ = reward_items_snapshot; // used in start packet only

    // ── 5. Check required items exist ────────────────────────────────────
    for (item_id, item_count) in &req_items {
        if *item_id == 0 || *item_count == 0 {
            continue;
        }
        // ITEM_GOLD is handled via GoldLose, not CheckExistItem
        if *item_id == ITEM_GOLD {
            continue;
        }
        if !world.check_exist_item(sid, *item_id, (*item_count).min(u16::MAX as u32) as u16) {
            let pkt = build_join_fail_packet("No items");
            session.send_packet(&pkt).await?;
            // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
            let chat_pkt = crate::systems::timed_notice::build_notice_packet(
                7,
                "[Lottery] You don't have the required items.",
            );
            session.send_packet(&chat_pkt).await?;
            return Ok(());
        }
    }

    // ── 6. Deduct gold ────────────────────────────────────────────────────
    let mut total_gold_cost: u32 = 0;
    for (item_id, item_count) in &req_items {
        if *item_id == ITEM_GOLD && *item_count > 0 {
            total_gold_cost = total_gold_cost.saturating_add(*item_count);
        }
    }

    if total_gold_cost > 0 {
        if !world.gold_lose(sid, total_gold_cost) {
            let pkt = build_join_fail_packet("You don't have enough money for the lottery event.");
            session.send_packet(&pkt).await?;
            // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
            let chat_pkt =
                crate::systems::timed_notice::build_notice_packet(7, "[Lottery] Not enough gold.");
            session.send_packet(&chat_pkt).await?;
            return Ok(());
        }
    }

    // ── 7. Deduct non-gold items ──────────────────────────────────────────
    // NOTE: C++ uses nReqItem[i] twice for ItemCount (appears to be a C++ bug),
    // but the intent is to rob the item. We use the correct item_count.
    for (item_id, item_count) in &req_items {
        if *item_id == 0 || *item_count == 0 || *item_id == ITEM_GOLD {
            continue;
        }
        // Check item is not countable kind 2 (non-stackable quest items)
        // We attempt rob; if it fails we continue (matches `if (!RobItem) continue`)
        if !world.rob_item(sid, *item_id, (*item_count).min(u16::MAX as u32) as u16) {
            warn!(
                "[{}] Lottery: rob_item({}, {}) failed for {}",
                session.addr(),
                item_id,
                item_count,
                char_name
            );
            // C++ continues on failure, not abort
            continue;
        }
    }

    // ── 8. Register/update participant and increment counter ──────────────
    // Use an enum to communicate the write-lock outcome without holding it across .await.
    enum RegisterResult {
        Success(u32),
        LimitExceeded,
    }

    let reg_result: RegisterResult = {
        let mut proc = lottery.write();

        // Re-check limit under write lock (race condition guard)
        if proc.user_join_counter >= proc.user_limit {
            RegisterResult::LimitExceeded
        } else {
            let entry = proc
                .participants
                .entry(char_name.clone())
                .or_insert_with(|| LotteryUserInfo {
                    name: char_name.clone(),
                    ticket_count: 0,
                    is_gift: false,
                });
            entry.ticket_count += 1;
            let tickets = entry.ticket_count;
            proc.user_join_counter += 1;
            RegisterResult::Success(tickets)
        }
        // proc dropped here
    };

    let ticket_count = match reg_result {
        RegisterResult::LimitExceeded => {
            let pkt = build_join_fail_packet("Limit is insufficient.");
            session.send_packet(&pkt).await?;
            // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
            let chat_pkt = crate::systems::timed_notice::build_notice_packet(
                7,
                "[Lottery] Limit is insufficient.",
            );
            session.send_packet(&chat_pkt).await?;
            return Ok(());
        }
        RegisterResult::Success(t) => t,
    };

    // ── 9. Send result to player ──────────────────────────────────────────
    let success_pkt = build_join_success_packet(ticket_count);
    session.send_packet(&success_pkt).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_msg = format!(
        "[Lottery] Ticket purchased! Total tickets: {}",
        ticket_count
    );
    let chat_pkt = crate::systems::timed_notice::build_notice_packet(7, &chat_msg);
    session.send_packet(&chat_pkt).await?;

    // ── 10. Broadcast count update to all players ─────────────────────────
    let count_pkt = build_count_update_packet();
    world.broadcast_to_all(Arc::new(count_pkt), None);

    info!(
        "[Lottery] {} joined. ticket_count={} total_participants={}",
        char_name,
        ticket_count,
        lottery.read().user_join_counter
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Start the lottery event (called by GM command or event scheduler)
// ─────────────────────────────────────────────────────────────────────────────

/// Start a lottery event from the given settings.
/// `req_items`: array of (item_id, item_count) — up to 5 slots
/// `reward_items`: array of item_ids — up to 4 slots
/// `user_limit`: max participants
/// `event_time_secs`: duration in seconds
/// Returns `false` if required items or rewards are all zero (invalid config).
pub fn start_lottery(
    lottery: &SharedLotteryProcess,
    req_items: [(u32, u32); MAX_REQ_ITEMS],
    reward_items: [u32; MAX_REWARD_ITEMS],
    user_limit: u32,
    event_time_secs: u32,
) -> bool {
    // Validate: at least one req item and one reward must exist
    let has_req = req_items.iter().any(|(id, cnt)| *id > 0 && *cnt > 0);
    let has_reward = reward_items.iter().any(|id| *id > 0);

    if !has_req || !has_reward {
        warn!("[Lottery] start_lottery: invalid config (no req items or no rewards)");
        return false;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let mut proc = lottery.write();

    if proc.lottery_start {
        warn!("[Lottery] start_lottery: already running");
        return false;
    }

    proc.req_items = req_items;
    proc.reward_items = reward_items;
    proc.user_limit = user_limit;
    proc.event_time = event_time_secs;
    proc.event_start_time = now;
    proc.event_process_time = now + event_time_secs;
    proc.timer_control = true;
    proc.lottery_start = true;
    proc.participants.clear();
    proc.user_join_counter = 0;
    proc.send_gift_activate = false;

    info!(
        "[Lottery] Event started: limit={} duration={}s",
        user_limit, event_time_secs
    );
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw winners and reset
// ─────────────────────────────────────────────────────────────────────────────

/// Draw winners from participants using the same algorithm as C++.
/// Selects up to `MAX_WINNERS` (4) random unique indices from the participant
/// list. Returns a vec of (winner_name, reward_item_index) pairs.
/// The reward_item_index (0-3) maps to `proc.reward_items[index]`.
pub fn draw_winners(proc: &LotteryProcess) -> Vec<(String, usize)> {
    let participants: Vec<&LotteryUserInfo> =
        proc.participants.values().filter(|p| !p.is_gift).collect();

    let total = participants.len();
    if total == 0 {
        return Vec::new();
    }

    let num_winners = total.min(MAX_WINNERS);
    let mut rng = rand::thread_rng();
    let mut selected_indices: Vec<usize> = Vec::with_capacity(num_winners);

    while selected_indices.len() < num_winners {
        let idx = rng.gen_range(0..total);
        if !selected_indices.contains(&idx) {
            selected_indices.push(idx);
        }
    }

    let participants_list: Vec<&LotteryUserInfo> = participants.into_iter().collect();

    selected_indices
        .iter()
        .enumerate()
        .map(|(reward_idx, &part_idx)| {
            let winner = participants_list[part_idx];
            (winner.name.clone(), reward_idx)
        })
        .collect()
}

/// Reset the lottery event state.
pub fn reset_lottery(proc: &mut LotteryProcess) {
    proc.lottery_start = false;
    proc.timer_control = false;
    proc.send_gift_activate = false;
    proc.event_time = 0;
    proc.user_join_counter = 0;
    proc.req_items = [(0, 0); MAX_REQ_ITEMS];
    proc.reward_items = [0u32; MAX_REWARD_ITEMS];
    proc.user_limit = 0;
    proc.event_start_time = 0;
    proc.event_process_time = 0;
    proc.participants.clear();
}

// ─────────────────────────────────────────────────────────────────────────────
// Timer tick — called every second from event_system
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a lottery timer tick.
#[derive(Debug, PartialEq, Eq)]
pub enum LotteryTickResult {
    /// No action needed (lottery inactive or still running).
    Idle,
    /// Countdown warning: remaining minutes to broadcast.
    ///
    CountdownWarning(u32),
    /// Time expired: draw winners and reset.
    ///
    /// Contains the list of `(winner_name, reward_item_id)` pairs and the
    /// snapshot of reward items for letter delivery.
    Expired {
        /// Winners: `(name, reward_item_id)` pairs.
        winners: Vec<(String, u32)>,
    },
}

/// Process one timer tick for the lottery event.
/// Called every second from the event system background task. Checks if the
/// lottery is active, sends countdown warnings at key intervals, and when
/// time expires draws winners and resets the lottery state.
/// The caller is responsible for:
/// - Broadcasting countdown warning announcements
/// - Sending reward letters to winners via `create_system_letter()`
/// - Broadcasting end packet and announcement to all players
pub fn lottery_timer_tick(lottery: &SharedLotteryProcess, now: u32) -> LotteryTickResult {
    // Quick read-lock check: is lottery active?
    {
        let proc = lottery.read();
        if !proc.lottery_start || proc.event_time == 0 || !proc.timer_control {
            return LotteryTickResult::Idle;
        }
    }

    // Check remaining time (still under read lock)
    let remaining = {
        let proc = lottery.read();
        proc.remaining_secs(now)
    };

    if remaining > 0 {
        let warn_minutes = match remaining {
            900 => Some(15),
            600 => Some(10),
            300 => Some(5),
            180 => Some(3),
            120 => Some(2),
            60 => Some(1),
            _ => None,
        };
        if let Some(minutes) = warn_minutes {
            return LotteryTickResult::CountdownWarning(minutes);
        }
        return LotteryTickResult::Idle;
    }

    // ── Time expired: draw winners, reset ─────────────────────────────────
    //   pLotteryProc.TimerControl = false;
    //   pLotteryProc.SendGitfActivate = true;
    //   LotterySendGift();
    //   LotterySystemReset();
    let mut proc = lottery.write();

    proc.timer_control = false;
    proc.send_gift_activate = true;

    // Draw winners before reset clears participants
    let winner_names = draw_winners(&proc);

    // Map winner (name, reward_index) to (name, reward_item_id)
    let winners: Vec<(String, u32)> = winner_names
        .into_iter()
        .filter_map(|(name, reward_idx)| {
            let item_id = proc.reward_items.get(reward_idx).copied().unwrap_or(0);
            if item_id > 0 {
                // Mark participant as gifted
                if let Some(p) = proc.participants.get_mut(&name) {
                    p.is_gift = true;
                }
                Some((name, item_id))
            } else {
                None
            }
        })
        .collect();

    // Reset the lottery state
    reset_lottery(&mut proc);

    LotteryTickResult::Expired { winners }
}

/// Format a winner ordinal string matching C++ output.
///   case 1: "%dst Winner Player : %s"
///   case 2: "%dnd Winner Player : %s"
///   case 3: "%drd Winner Player : %s"
///   case 4: "%dth Winner Player : %s"
pub fn format_winner_message(rank: usize, name: &str) -> String {
    let suffix = match rank {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    };
    format!("{}{} Winner Player : {} ", rank, suffix, name)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public handler entry-point (dispatched from handle_ext_hook)
// ─────────────────────────────────────────────────────────────────────────────

/// Handle WIZ_EXT_HOOK (0xE9) sub-opcode LOTTERY (0xC7) from client.
/// The first byte of `pkt.data` is the LOTTERY sub-opcode:
/// - `3` = join request → `handle_join()`
/// - Others are ignored (server → client only).
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);

    let world = session.world().clone();
    let lottery = world.lottery_process();

    match sub {
        SUB_JOIN => handle_join(session, lottery).await,
        _ => {
            debug!("[{}] LOTTERY unhandled sub=0x{:02X}", session.addr(), sub);
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── Packet format tests ────────────────────────────────────────────────

    fn make_active_proc() -> LotteryProcess {
        LotteryProcess {
            lottery_start: true,
            user_limit: 1000,
            event_time: 3500,
            event_start_time: 1_000_000,
            event_process_time: 1_003_500,
            req_items: [(900_000_000, 1_000_000), (0, 0), (0, 0), (0, 0), (0, 0)],
            reward_items: [700_089_000, 700_084_000, 700_083_000, 700_082_000],
            ..Default::default()
        }
    }

    #[test]
    fn test_build_start_packet_format() {
        // result << u8(1) << [5x req_item, req_count] << [4x reward] << user_limit << remaining << join_count << ticket_count
        let proc = make_active_proc();
        let pkt = build_start_packet(&proc, 1_000_000, 3);

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_LOTTERY)); // 0xC7
        assert_eq!(r.read_u8(), Some(SUB_START)); // 1

        // 5 req item slots
        assert_eq!(r.read_u32(), Some(900_000_000)); // req_item[0]
        assert_eq!(r.read_u32(), Some(1_000_000)); // req_count[0]
        for _ in 1..MAX_REQ_ITEMS {
            assert_eq!(r.read_u32(), Some(0)); // item_id = 0
            assert_eq!(r.read_u32(), Some(0)); // count = 0
        }

        // 4 reward slots
        assert_eq!(r.read_u32(), Some(700_089_000));
        assert_eq!(r.read_u32(), Some(700_084_000));
        assert_eq!(r.read_u32(), Some(700_083_000));
        assert_eq!(r.read_u32(), Some(700_082_000));

        // user_limit, remaining, join_count, ticket_count
        assert_eq!(r.read_u32(), Some(1000)); // user_limit
        assert_eq!(r.read_u32(), Some(3500)); // remaining (event_process_time - now = 1_003_500 - 1_000_000)
        assert_eq!(r.read_u32(), Some(0)); // join_counter
        assert_eq!(r.read_u32(), Some(3)); // my_ticket_count

        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_start_packet_size() {
        // Expected: 2 (sub-opcode bytes) + 5*8 (req) + 4*4 (rewards) + 4*4 (counts) = 2 + 40 + 16 + 16 = 74
        let proc = make_active_proc();
        let pkt = build_start_packet(&proc, 0, 0);
        assert_eq!(pkt.data.len(), 74);
    }

    #[test]
    fn test_build_count_update_packet() {
        let pkt = build_count_update_packet();
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_LOTTERY));
        assert_eq!(r.read_u8(), Some(SUB_COUNT_UPDATE));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_join_success_packet() {
        // pkt << u8(0xC7) << u8(3) << u8(1) << ticket_count
        let pkt = build_join_success_packet(5);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_LOTTERY));
        assert_eq!(r.read_u8(), Some(SUB_JOIN_RESULT));
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.read_u32(), Some(5));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_join_fail_packet() {
        // pkt << u8(0xC7) << u8(3) << u8(0) << string(msg)
        let pkt = build_join_fail_packet("No items");
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_LOTTERY));
        assert_eq!(r.read_u8(), Some(SUB_JOIN_RESULT));
        assert_eq!(r.read_u8(), Some(0)); // fail
                                          // sbyte_string: u8 len + bytes
        let msg = r.read_string().unwrap_or_default();
        assert_eq!(msg, "No items");
    }

    #[test]
    fn test_build_end_packet() {
        let pkt = build_end_packet();
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_LOTTERY));
        assert_eq!(r.read_u8(), Some(SUB_END));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_ext_sub_lottery_opcode_constant() {
        assert_eq!(EXT_SUB_LOTTERY, 0xC7);
    }

    #[test]
    fn test_start_packet_little_endian() {
        // Verify user_limit (1000 = 0x000003E8) is stored as little-endian
        let proc = make_active_proc();
        let pkt = build_start_packet(&proc, 1_000_000, 0);

        // data[0]=0xC7 (LOTTERY), data[1]=1 (SUB_START)
        // After 5*8=40 bytes of req items and 4*4=16 bytes of rewards = offset 58
        // user_limit at byte 58: 1000 LE = [0xE8, 0x03, 0x00, 0x00]
        assert_eq!(pkt.data[58], 0xE8); // low byte of 1000
        assert_eq!(pkt.data[59], 0x03);
        assert_eq!(pkt.data[60], 0x00);
        assert_eq!(pkt.data[61], 0x00);
    }

    // ── LotteryProcess logic tests ─────────────────────────────────────────

    #[test]
    fn test_remaining_secs_when_future() {
        let proc = LotteryProcess {
            event_process_time: 5000,
            ..Default::default()
        };
        assert_eq!(proc.remaining_secs(3000), 2000);
    }

    #[test]
    fn test_remaining_secs_when_expired() {
        let proc = LotteryProcess {
            event_process_time: 3000,
            ..Default::default()
        };
        assert_eq!(proc.remaining_secs(5000), 0);
    }

    #[test]
    fn test_remaining_secs_exact() {
        let proc = LotteryProcess {
            event_process_time: 5000,
            ..Default::default()
        };
        assert_eq!(proc.remaining_secs(5000), 0);
    }

    #[test]
    fn test_start_lottery_valid() {
        let lottery = new_lottery_process();
        let req = [
            (900_000_000u32, 1_000_000u32),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ];
        let rewards = [700_089_000u32, 0, 0, 0];

        let started = start_lottery(&lottery, req, rewards, 1000, 3600);
        assert!(started);

        let proc = lottery.read();
        assert!(proc.lottery_start);
        assert!(proc.timer_control);
        assert_eq!(proc.user_limit, 1000);
        assert_eq!(proc.event_time, 3600);
        assert_eq!(proc.user_join_counter, 0);
        assert!(proc.participants.is_empty());
    }

    #[test]
    fn test_start_lottery_no_req_items_fails() {
        let lottery = new_lottery_process();
        let req = [(0u32, 0u32); MAX_REQ_ITEMS];
        let rewards = [700_089_000u32, 0, 0, 0];

        let started = start_lottery(&lottery, req, rewards, 1000, 3600);
        assert!(!started);
        assert!(!lottery.read().lottery_start);
    }

    #[test]
    fn test_start_lottery_no_rewards_fails() {
        let lottery = new_lottery_process();
        let req = [
            (900_000_000u32, 1_000_000u32),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ];
        let rewards = [0u32; MAX_REWARD_ITEMS];

        let started = start_lottery(&lottery, req, rewards, 1000, 3600);
        assert!(!started);
    }

    #[test]
    fn test_start_lottery_already_running_fails() {
        let lottery = new_lottery_process();
        let req = [
            (900_000_000u32, 1_000_000u32),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ];
        let rewards = [700_089_000u32, 0, 0, 0];

        assert!(start_lottery(&lottery, req, rewards, 1000, 3600));
        // Second call should fail
        assert!(!start_lottery(&lottery, req, rewards, 1000, 3600));
    }

    #[test]
    fn test_reset_lottery_clears_state() {
        let lottery = new_lottery_process();
        let req = [
            (900_000_000u32, 1_000_000u32),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ];
        let rewards = [700_089_000u32, 0, 0, 0];

        start_lottery(&lottery, req, rewards, 1000, 3600);

        {
            let mut proc = lottery.write();
            proc.participants.insert(
                "TESTPLAYER".to_string(),
                LotteryUserInfo {
                    name: "TESTPLAYER".to_string(),
                    ticket_count: 3,
                    is_gift: false,
                },
            );
            proc.user_join_counter = 1;
        }

        {
            let mut proc = lottery.write();
            reset_lottery(&mut proc);
        }

        let proc = lottery.read();
        assert!(!proc.lottery_start);
        assert!(!proc.timer_control);
        assert_eq!(proc.user_join_counter, 0);
        assert!(proc.participants.is_empty());
        assert_eq!(proc.user_limit, 0);
        assert_eq!(proc.event_time, 0);
    }

    // ── Winner drawing tests ───────────────────────────────────────────────

    #[test]
    fn test_draw_winners_empty_participants() {
        let proc = LotteryProcess::default();
        let winners = draw_winners(&proc);
        assert!(winners.is_empty());
    }

    #[test]
    fn test_draw_winners_one_participant() {
        let mut proc = LotteryProcess::default();
        proc.participants.insert(
            "ALICE".to_string(),
            LotteryUserInfo {
                name: "ALICE".to_string(),
                ticket_count: 1,
                is_gift: false,
            },
        );

        let winners = draw_winners(&proc);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].0, "ALICE");
        assert_eq!(winners[0].1, 0); // reward index 0
    }

    #[test]
    fn test_draw_winners_max_four() {
        let mut proc = LotteryProcess::default();
        for i in 0..10usize {
            let name = format!("PLAYER{}", i);
            proc.participants.insert(
                name.clone(),
                LotteryUserInfo {
                    name,
                    ticket_count: 1,
                    is_gift: false,
                },
            );
        }

        let winners = draw_winners(&proc);
        // Should draw exactly MAX_WINNERS = 4
        assert_eq!(winners.len(), MAX_WINNERS);

        // All winners should be unique
        let mut names: Vec<String> = winners.iter().map(|(n, _)| n.clone()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), MAX_WINNERS);
    }

    #[test]
    fn test_draw_winners_fewer_than_max() {
        let mut proc = LotteryProcess::default();
        for i in 0..2usize {
            let name = format!("PLAYER{}", i);
            proc.participants.insert(
                name.clone(),
                LotteryUserInfo {
                    name,
                    ticket_count: 1,
                    is_gift: false,
                },
            );
        }

        let winners = draw_winners(&proc);
        assert_eq!(winners.len(), 2); // only 2 participants
    }

    #[test]
    fn test_draw_winners_skips_already_gifted() {
        let mut proc = LotteryProcess::default();
        proc.participants.insert(
            "GIFTED".to_string(),
            LotteryUserInfo {
                name: "GIFTED".to_string(),
                ticket_count: 5,
                is_gift: true, // already received gift
            },
        );
        proc.participants.insert(
            "FRESH".to_string(),
            LotteryUserInfo {
                name: "FRESH".to_string(),
                ticket_count: 1,
                is_gift: false,
            },
        );

        let winners = draw_winners(&proc);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].0, "FRESH");
    }

    #[test]
    fn test_draw_winners_reward_indices_sequential() {
        let mut proc = LotteryProcess::default();
        for i in 0..MAX_WINNERS {
            let name = format!("P{}", i);
            proc.participants.insert(
                name.clone(),
                LotteryUserInfo {
                    name,
                    ticket_count: 1,
                    is_gift: false,
                },
            );
        }

        let winners = draw_winners(&proc);
        // Reward indices should be 0, 1, 2, 3 in order
        let mut reward_indices: Vec<usize> = winners.iter().map(|(_, idx)| *idx).collect();
        reward_indices.sort();
        assert_eq!(reward_indices, vec![0, 1, 2, 3]);
    }

    // ── Announce packet test ───────────────────────────────────────────────

    #[test]
    fn test_build_lottery_announce_format() {
        let pkt = build_lottery_announce("Lottery Event started.");
        assert_eq!(pkt.opcode, Opcode::WizChat as u8);

        let mut r = PacketReader::new(&pkt.data);
        let chat_type = r.read_u8().unwrap();
        assert_eq!(chat_type, ChatType::Public as u8); // 7

        let _nation = r.read_u8();
        let _sender_id = r.read_u32();
        let name = r.read_sbyte_string().unwrap_or_default();
        assert_eq!(name, "SYSTEM");
    }

    #[test]
    fn test_join_fail_various_messages() {
        let messages = [
            "The Lottery Event has not started.",
            "Limit is insufficient.",
            "No items",
            "You don't have enough money for the lottery event.",
            "You don't have enough money for the event.",
        ];

        for msg in &messages {
            let pkt = build_join_fail_packet(msg);
            let mut r = PacketReader::new(&pkt.data);
            assert_eq!(r.read_u8(), Some(EXT_SUB_LOTTERY));
            assert_eq!(r.read_u8(), Some(SUB_JOIN_RESULT));
            assert_eq!(r.read_u8(), Some(0)); // fail
            let decoded = r.read_string().unwrap_or_default();
            assert_eq!(&decoded, msg);
        }
    }

    #[test]
    fn test_join_success_ticket_counts() {
        for count in [1u32, 5, 100, u32::MAX] {
            let pkt = build_join_success_packet(count);
            let mut r = PacketReader::new(&pkt.data);
            r.read_u8(); // EXT_SUB_LOTTERY
            r.read_u8(); // SUB_JOIN_RESULT
            r.read_u8(); // success=1
            assert_eq!(r.read_u32(), Some(count));
        }
    }

    // ── Timer tick tests ──────────────────────────────────────────────────

    #[test]
    fn test_timer_tick_idle_when_inactive() {
        let lottery = new_lottery_process();
        assert_eq!(
            lottery_timer_tick(&lottery, 1_000_000),
            LotteryTickResult::Idle
        );
    }

    #[test]
    fn test_timer_tick_idle_when_time_remaining() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        // Plenty of time remaining (not at a warning threshold)
        assert_eq!(
            lottery_timer_tick(&lottery, 1_001_000),
            LotteryTickResult::Idle
        );
    }

    #[test]
    fn test_timer_tick_countdown_15min() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        // 900 seconds remaining = 15 minutes
        let now = 1_003_600 - 900;
        assert_eq!(
            lottery_timer_tick(&lottery, now),
            LotteryTickResult::CountdownWarning(15)
        );
    }

    #[test]
    fn test_timer_tick_countdown_10min() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        let now = 1_003_600 - 600;
        assert_eq!(
            lottery_timer_tick(&lottery, now),
            LotteryTickResult::CountdownWarning(10)
        );
    }

    #[test]
    fn test_timer_tick_countdown_5min() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        let now = 1_003_600 - 300;
        assert_eq!(
            lottery_timer_tick(&lottery, now),
            LotteryTickResult::CountdownWarning(5)
        );
    }

    #[test]
    fn test_timer_tick_countdown_3min() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        let now = 1_003_600 - 180;
        assert_eq!(
            lottery_timer_tick(&lottery, now),
            LotteryTickResult::CountdownWarning(3)
        );
    }

    #[test]
    fn test_timer_tick_countdown_2min() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        let now = 1_003_600 - 120;
        assert_eq!(
            lottery_timer_tick(&lottery, now),
            LotteryTickResult::CountdownWarning(2)
        );
    }

    #[test]
    fn test_timer_tick_countdown_1min() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
        }
        let now = 1_003_600 - 60;
        assert_eq!(
            lottery_timer_tick(&lottery, now),
            LotteryTickResult::CountdownWarning(1)
        );
    }

    #[test]
    fn test_timer_tick_expired_no_participants() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
            proc.reward_items = [700_089_000, 700_084_000, 0, 0];
        }
        let result = lottery_timer_tick(&lottery, 1_003_601);
        assert_eq!(result, LotteryTickResult::Expired { winners: vec![] });

        // State should be reset
        let proc = lottery.read();
        assert!(!proc.lottery_start);
        assert!(!proc.timer_control);
        assert_eq!(proc.user_join_counter, 0);
    }

    #[test]
    fn test_timer_tick_expired_with_participants() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
            proc.reward_items = [700_089_000, 700_084_000, 700_083_000, 700_082_000];
            proc.user_join_counter = 1;
            proc.participants.insert(
                "TESTPLAYER".to_string(),
                LotteryUserInfo {
                    name: "TESTPLAYER".to_string(),
                    ticket_count: 3,
                    is_gift: false,
                },
            );
        }
        let result = lottery_timer_tick(&lottery, 1_003_600);
        match result {
            LotteryTickResult::Expired { winners } => {
                assert_eq!(winners.len(), 1);
                assert_eq!(winners[0].0, "TESTPLAYER");
                assert_eq!(winners[0].1, 700_089_000);
            }
            other => panic!("Expected Expired, got {:?}", other),
        }

        // State should be reset
        let proc = lottery.read();
        assert!(!proc.lottery_start);
    }

    #[test]
    fn test_timer_tick_expired_with_multiple_winners() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = true;
            proc.event_time = 3600;
            proc.event_start_time = 1_000_000;
            proc.event_process_time = 1_003_600;
            proc.reward_items = [700_089_000, 700_084_000, 700_083_000, 700_082_000];
            for i in 0..5 {
                let name = format!("PLAYER{}", i);
                proc.participants.insert(
                    name.clone(),
                    LotteryUserInfo {
                        name,
                        ticket_count: 1,
                        is_gift: false,
                    },
                );
            }
            proc.user_join_counter = 5;
        }
        let result = lottery_timer_tick(&lottery, 1_003_600);
        match result {
            LotteryTickResult::Expired { winners } => {
                assert_eq!(winners.len(), MAX_WINNERS); // 4
                                                        // Each winner should have a valid reward item
                for (name, item_id) in &winners {
                    assert!(!name.is_empty());
                    assert!(*item_id > 0);
                }
                // All winners should be unique
                let mut names: Vec<&str> = winners.iter().map(|(n, _)| n.as_str()).collect();
                names.sort();
                names.dedup();
                assert_eq!(names.len(), MAX_WINNERS);
            }
            other => panic!("Expected Expired, got {:?}", other),
        }
    }

    #[test]
    fn test_timer_tick_idle_when_timer_control_false() {
        let lottery = new_lottery_process();
        {
            let mut proc = lottery.write();
            proc.lottery_start = true;
            proc.timer_control = false; // timer disabled
            proc.event_time = 3600;
        }
        assert_eq!(
            lottery_timer_tick(&lottery, 999_999_999),
            LotteryTickResult::Idle
        );
    }

    // ── Format winner message tests ───────────────────────────────────────

    #[test]
    fn test_format_winner_message_ordinals() {
        assert_eq!(
            format_winner_message(1, "Alice"),
            "1st Winner Player : Alice "
        );
        assert_eq!(format_winner_message(2, "Bob"), "2nd Winner Player : Bob ");
        assert_eq!(
            format_winner_message(3, "Charlie"),
            "3rd Winner Player : Charlie "
        );
        assert_eq!(
            format_winner_message(4, "Dave"),
            "4th Winner Player : Dave "
        );
    }
}
