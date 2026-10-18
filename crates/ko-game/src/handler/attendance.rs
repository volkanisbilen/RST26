//! WIZ_ATTENDANCE (0xB7) handler — daily login attendance calendar.
//!
//! v2525 client's native attendance panel (Table G dispatch at `0x8300E0`).
//! Replaces ext_hook (0xE9) daily_reward which is outside the v2525 dispatch range.
//!
//! ## Client RE
//!
//! - Panel object: `[esi+0x600]` — created on UI open, null-checked before dispatch
//! - Dispatch: Table G jump table `0x8300E0` (wire sub-types 1–9, index 0–8)
//! - C2S: sub=1 (open panel / request data), sub=8 (claim today's reward)
//! - S2C: sub=1 (result/control), sub=2 (calendar init), sub=3 (board data),
//!   sub=4 (reward notify), sub=5 (item slot), sub=6 (day entry), sub=7 (full refresh),
//!   sub=8 (claim result), sub=9 (special NPC reward)
//!
//! ## DB
//!
//! Reuses `daily_reward` (25 items + counts), `daily_reward_user` (per-user progress),
//! and `daily_reward_cumulative` (milestone rewards at day 7/14/21).
//!
//! ## Binary/ Reference
//!
//! `HandleDailyRewardGive` @ 0x140285200 — daily claim logic.
//! `HandleDailyCumRewardGive` @ 0x140286180 — cumulative milestone claim.
//! Both use ext_hook 0xE9 (unusable in v2525). Our handler uses native 0xB7.

use chrono::Datelike;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, info, warn};

use crate::session::{ClientSession, SessionState};

// ── S2C Sub-type constants ────────────────────────────────────────────────

/// Sub 1: Result / panel control (inner switch on result_code 0–6).
const ATT_SUB_RESULT: u8 = 1;

/// Sub 3: Board data / progress update.
#[cfg(test)]
const ATT_SUB_BOARD_DATA: u8 = 3;

/// Sub 4: Reward item notification — `[u8 sub_result=1][string item_name][u8 tier]`.
const ATT_SUB_REWARD_NOTIFY: u8 = 4;

/// Sub 8: Claim result — `[u8 flag]` (1=success, 0=refresh).
const ATT_SUB_CLAIM_RESULT: u8 = 8;

// ── Result codes for Sub 1 ───────────────────────────────────────────────

/// result_code=0: Open/refresh panel — `[u8 day_index]`.
const RESULT_OPEN_PANEL: u8 = 0;

/// result_code=1: Item added to list — `[u8 day_index][u16 count][u8 complete]`.
const RESULT_ITEM_ADDED: u8 = 1;

/// result_code=3: Timer/cooldown message (string 0x464F = 17999).
const RESULT_TIMER: u8 = 3;

/// result_code=4: Already claimed message (string 0x4650 = 18000).
const RESULT_ALREADY_CLAIMED: u8 = 4;

/// result_code=5: Inventory full / cannot give item.
const RESULT_INVENTORY_FULL: u8 = 5;

/// result_code=6: Event not available (string 0xAD86 = 44422).
const RESULT_NOT_AVAILABLE: u8 = 6;

/// Maximum attendance calendar slots (client validates 0–8).
const MAX_SLOTS: usize = 9;

/// Total daily reward days in a cycle.
const TOTAL_DAYS: usize = 25;

/// Cumulative reward milestone: first bonus at day 7.
const CUM_MILESTONE_1: usize = 7;
/// Cumulative reward milestone: second bonus at day 14.
const CUM_MILESTONE_2: usize = 14;
/// Cumulative reward milestone: third bonus at day 21.
const CUM_MILESTONE_3: usize = 21;

// ── S2C Packet Builders ──────────────────────────────────────────────────

/// Build a Sub 1 result_code=0 (open/refresh panel) packet.
///
/// Client RE: `0x7023A0` case 0 — sets header text (string 0xA88D or 0xA88E),
/// stores day_index at panel `[+0x150]`.
///
/// Wire: `[0xB7][0x01][u8 result=0][u8 day_index]`
fn build_open_panel(day_index: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizAttendance as u8);
    pkt.write_u8(ATT_SUB_RESULT);
    pkt.write_u8(RESULT_OPEN_PANEL);
    pkt.write_u8(day_index);
    pkt
}

/// Build a Sub 1 result_code=1 (item added to list) packet.
///
/// Client RE: `0x7023A0` case 1 — adds entry to list widget, plays sound 0x53092.
///
/// Wire: `[0xB7][0x01][u8 result=1][u8 day_index][u16 item_count][u8 complete_flag]`
fn build_item_added(day_index: u8, item_count: u16, complete_flag: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizAttendance as u8);
    pkt.write_u8(ATT_SUB_RESULT);
    pkt.write_u8(RESULT_ITEM_ADDED);
    pkt.write_u8(day_index);
    pkt.write_u16(item_count);
    pkt.write_u8(complete_flag);
    pkt
}

/// Build a Sub 1 error message packet (result_code=3/4/5/6).
///
/// Client RE: result_code 3–6 show localized notice strings as yellow text.
///
/// Wire: `[0xB7][0x01][u8 result_code]`
fn build_result_msg(result_code: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizAttendance as u8);
    pkt.write_u8(ATT_SUB_RESULT);
    pkt.write_u8(result_code);
    pkt
}

/// Build a Sub 4 (reward notification) packet.
///
/// Client RE: `0x700080` — shows yellow notice: "string_0xA892 + item_name".
/// Only processes sub_result=1.
///
/// Wire: `[0xB7][0x04][u8 sub_result=1][string item_name][u8 tier_flag]`
fn build_reward_notify(item_name: &str, tier_flag: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizAttendance as u8);
    pkt.write_u8(ATT_SUB_REWARD_NOTIFY);
    pkt.write_u8(1); // sub_result = 1 (required)
    pkt.write_string(item_name);
    pkt.write_u8(tier_flag);
    pkt
}

/// Build a Sub 8 (claim result) packet.
///
/// Client RE: inline at `0x82F7DD` — flag=1 destroys panel (success),
/// flag=0 refreshes 3D display.
///
/// Wire: `[0xB7][0x08][u8 result_flag]`
fn build_claim_result(result_flag: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizAttendance as u8);
    pkt.write_u8(ATT_SUB_CLAIM_RESULT);
    pkt.write_u8(result_flag);
    pkt
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Calculate the 9-slot window start centered on the first unclaimed day.
fn calc_window_start(first_unclaimed: usize) -> usize {
    if first_unclaimed < 4 {
        0
    } else if first_unclaimed + 5 > TOTAL_DAYS {
        TOTAL_DAYS.saturating_sub(MAX_SLOTS)
    } else {
        first_unclaimed - 4
    }
}

/// Check if progress needs monthly reset.
///
/// Returns true if any claimed day has a `last_claim_month` that doesn't
/// match the current month — indicating a new month has started.
fn needs_monthly_reset(
    user_rows: &[ko_db::models::daily_reward::DailyRewardUserRow],
    current_month: i16,
) -> bool {
    for row in user_rows {
        if row.claimed && row.last_claim_month != 0 && row.last_claim_month != current_month {
            return true;
        }
    }
    false
}

// ── C2S Handler ──────────────────────────────────────────────────────────

/// Handle WIZ_ATTENDANCE (0xB7) from the client.
///
/// Client sends:
/// - sub=1: Open attendance panel / request calendar data
/// - sub=8: Claim today's reward
///
/// The panel at `[esi+0x600]` is created client-side when the user opens
/// the attendance UI. Before that, all S2C packets are dropped (null check).
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);

    match sub {
        1 => handle_open(session).await,
        8 => handle_claim(session).await,
        _ => {
            debug!(
                "[{}] WIZ_ATTENDANCE unknown C2S sub={} ({}B)",
                session.addr(),
                sub,
                reader.remaining()
            );
            Ok(())
        }
    }
}

/// Handle C2S sub=1: Open panel — load and send calendar state.
///
/// The v2615 client loads reward presentation from `Data\\Attendance.tbl`.
/// Only the panel's own sub=1 control packet belongs here. Sub=5 and sub=6
/// are global inventory item-slot handlers in this client and corrupt client
/// inventory/UI state when they are used to populate Attendance rewards.
async fn handle_open(session: &mut ClientSession) -> anyhow::Result<()> {
    let pool = session.pool().clone();
    let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);

    // Load reward config (25 items)
    let reward_config = match repo.load_all().await {
        Ok(v) => v,
        Err(e) => {
            warn!("[{}] attendance load_all DB error: {e}", session.addr());
            Vec::new()
        }
    };
    if reward_config.is_empty() {
        // No attendance data configured — send "not available"
        let msg = build_result_msg(RESULT_NOT_AVAILABLE);
        session.send_packet(&msg).await?;
        return Ok(());
    }

    // Load user progress
    let char_name = session
        .world()
        .get_character_info(session.session_id())
        .map(|c| c.name.clone())
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(());
    }

    let mut user_rows = match repo.load_user_progress(&char_name).await {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "[{}] attendance load_user_progress DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };

    // Monthly reset: if claimed days are from a previous month, reset
    let now = chrono::Utc::now();
    let current_month = now.month() as i16;
    if needs_monthly_reset(&user_rows, current_month) {
        if let Err(e) = repo.reset_user_progress(&char_name).await {
            warn!("Failed to reset attendance for {}: {}", char_name, e);
        }
        // Re-fetch fresh state
        user_rows = match repo.load_user_progress(&char_name).await {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "[{}] attendance load_user_progress (post-reset) DB error: {e}",
                    session.addr()
                );
                Vec::new()
            }
        };
    }

    let mut claimed = [false; TOTAL_DAYS];
    for row in &user_rows {
        let idx = row.day_index as usize;
        if idx < TOTAL_DAYS {
            claimed[idx] = row.claimed;
        }
    }

    // Find first unclaimed day
    let first_unclaimed = claimed.iter().position(|&c| !c).unwrap_or(TOTAL_DAYS);

    // Determine the 9-slot window: center on first_unclaimed, clamp to bounds
    let window_start = calc_window_start(first_unclaimed);

    let today_day = now.day() as u8;
    let claimed_count = claimed.iter().filter(|&&c| c).count() as u16;
    // Open the native panel at the first unclaimed day. Its reward rows and
    // item models come from the client's Attendance.tbl.
    let slot_for_unclaimed =
        if first_unclaimed >= window_start && first_unclaimed < window_start + MAX_SLOTS {
            (first_unclaimed - window_start) as u8
        } else {
            0
        };
    let open = build_open_panel(slot_for_unclaimed);
    session.send_packet(&open).await?;

    debug!(
        "[{}] WIZ_ATTENDANCE open: {} claimed/{}, window={}-{}, today={}",
        session.addr(),
        claimed_count,
        TOTAL_DAYS,
        window_start,
        (window_start + MAX_SLOTS).min(TOTAL_DAYS),
        today_day,
    );

    Ok(())
}

/// Handle C2S sub=8: Claim today's reward.
///
/// Validation (ported from Binary/ `HandleDailyRewardGive`):
/// 1. Find first unclaimed day
/// 2. If day > 0: previous day must be claimed (sequential)
/// 3. If day > 0: previous day must not be same calendar day (one per day)
/// 4. Give item, mark claimed, save to DB
/// 5. If milestone (day 7/14/21): also give cumulative reward
async fn handle_claim(session: &mut ClientSession) -> anyhow::Result<()> {
    handle_claim_impl(session, true).await.map(|_| ())
}

/// Calendar state of the native Attendance window: which of the 25 days are taken and whether
/// a reward was already taken today (one reward per calendar day). A new month starts over.
pub(super) async fn native_state(session: &mut ClientSession) -> ([bool; TOTAL_DAYS], bool) {
    let pool = session.pool().clone();
    let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);
    let name = session
        .world()
        .get_character_info(session.session_id())
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let mut taken = [false; TOTAL_DAYS];
    if name.is_empty() {
        return (taken, false);
    }
    let mut rows = repo.load_user_progress(&name).await.unwrap_or_default();
    let now = chrono::Utc::now();
    if needs_monthly_reset(&rows, now.month() as i16) {
        if repo.reset_user_progress(&name).await.is_ok() {
            rows = repo.load_user_progress(&name).await.unwrap_or_default();
        }
    }
    let today = now.day() as i16;
    let mut claimed_today = false;
    for row in &rows {
        let idx = row.day_index as usize;
        if idx < TOTAL_DAYS && row.claimed {
            taken[idx] = true;
            if row.day_of_month == today {
                claimed_today = true;
            }
        }
    }
    (taken, claimed_today)
}

/// The player clicked one reward tile of the native Attendance window (`9C 05`, tile key
/// 1..=25 for the daily rewards). Only today's next tile can be taken; anything else is refused.
pub(super) async fn claim_native_tile(
    session: &mut ClientSession,
    tile_key: i32,
) -> anyhow::Result<bool> {
    if !(1..=TOTAL_DAYS as i32).contains(&tile_key) {
        // the 7/14/21-day bonus is handed over together with the daily reward that completes it
        return Ok(false);
    }
    let (taken, claimed_today) = native_state(session).await;
    let Some(next) = taken.iter().position(|t| !*t) else {
        return Ok(false);
    };
    if claimed_today || next as i32 + 1 != tile_key {
        return Ok(false);
    }
    handle_claim_impl(session, false).await
}

async fn handle_claim_impl(
    session: &mut ClientSession,
    send_legacy_ui: bool,
) -> anyhow::Result<bool> {
    let pool = session.pool().clone();
    let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);

    // Load reward config
    let reward_config = match repo.load_all().await {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "[{}] attendance claim load_all DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };
    if reward_config.is_empty() {
        return Ok(false);
    }

    let mut item_ids = [0i32; TOTAL_DAYS];
    let mut item_counts = [1i16; TOTAL_DAYS];
    for row in &reward_config {
        let idx = row.day_index as usize;
        if idx < TOTAL_DAYS {
            item_ids[idx] = row.item_id;
            item_counts[idx] = row.item_count.max(1);
        }
    }

    // Load user progress
    let char_name = session
        .world()
        .get_character_info(session.session_id())
        .map(|c| c.name.clone())
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(false);
    }

    let mut user_rows = match repo.load_user_progress(&char_name).await {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "[{}] attendance claim load_user_progress DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };

    // Monthly reset check
    let now = chrono::Utc::now();
    let current_month = now.month() as i16;
    let today_day = now.day() as u8;

    if needs_monthly_reset(&user_rows, current_month) {
        if let Err(e) = repo.reset_user_progress(&char_name).await {
            warn!("Failed to reset attendance for {}: {}", char_name, e);
        }
        user_rows = match repo.load_user_progress(&char_name).await {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "[{}] attendance claim load_user_progress (post-reset) DB error: {e}",
                    session.addr()
                );
                Vec::new()
            }
        };
    }

    let mut sb_type = [0u8; TOTAL_DAYS]; // 0=unclaimed, 1=claimed
    let mut s_get_day = [0u8; TOTAL_DAYS]; // day-of-month when claimed
    for row in &user_rows {
        let idx = row.day_index as usize;
        if idx < TOTAL_DAYS {
            sb_type[idx] = if row.claimed { 1 } else { 0 };
            s_get_day[idx] = row.day_of_month as u8;
        }
    }

    // Find the first unclaimed day
    let claim_idx = match sb_type.iter().position(|&t| t == 0) {
        Some(idx) => idx,
        None => {
            // All 25 days claimed — cycle complete
            let msg = build_result_msg(RESULT_ALREADY_CLAIMED);
            if send_legacy_ui {
                session.send_packet(&msg).await?;
            }
            return Ok(false);
        }
    };

    // Validate sequential: previous day must be claimed (except day 0)
    if claim_idx > 0 && sb_type[claim_idx - 1] == 0 {
        let msg = build_result_msg(RESULT_TIMER);
        if send_legacy_ui {
            session.send_packet(&msg).await?;
        }
        return Ok(false);
    }

    // Validate same-day: previous day must not be claimed on the same calendar day
    // Binary/ Reference: HandleDailyRewardGive — "can only claim once per calendar day"
    if claim_idx > 0 && s_get_day[claim_idx - 1] == today_day {
        let msg = build_result_msg(RESULT_ALREADY_CLAIMED);
        if send_legacy_ui {
            session.send_packet(&msg).await?;
        }
        return Ok(false);
    }

    // Valid claim!
    let item_id = item_ids[claim_idx] as u32;
    let count = item_counts[claim_idx] as u16;
    let day_index = claim_idx as u8;
    let complete = if claim_idx == TOTAL_DAYS - 1 {
        1u8
    } else {
        0u8
    };

    // Give item to player — check for inventory full
    let world = session.world().clone();
    let gave = world.give_item(session.session_id(), item_id, count);
    if !gave {
        // Inventory full — send error, do NOT mark as claimed
        let msg = build_result_msg(RESULT_INVENTORY_FULL);
        if send_legacy_ui {
            session.send_packet(&msg).await?;
        }
        return Ok(false);
    }

    // Send success: Sub 1 result=1 (item added)
    if send_legacy_ui {
        let added = build_item_added(day_index, count, complete);
        session.send_packet(&added).await?;
    }

    // Send reward notification: Sub 4
    let item_name = world
        .get_item(item_id)
        .and_then(|i| i.str_name.clone())
        .unwrap_or_else(|| format!("Item #{}", item_id));
    if send_legacy_ui {
        let notify = build_reward_notify(&item_name, 0);
        session.send_packet(&notify).await?;
    }

    // Send claim success: Sub 8 result=1 (close panel)
    if send_legacy_ui {
        let result = build_claim_result(1);
        session.send_packet(&result).await?;
    }

    // Cumulative milestone check: give bonus items at days 7, 14, 21
    let claimed_so_far = claim_idx + 1; // 1-based count after this claim
    let cum_config = match repo.load_cumulative().await {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "[{}] attendance load_cumulative DB error: {e}",
                session.addr()
            );
            None
        }
    };
    if let Some(cum) = cum_config {
        let bonus_item = match claimed_so_far {
            CUM_MILESTONE_1 => cum.item1,
            CUM_MILESTONE_2 => cum.item2,
            CUM_MILESTONE_3 => cum.item3,
            _ => None,
        };
        if let Some(bonus_id) = bonus_item {
            let bonus_gave = world.give_item(session.session_id(), bonus_id as u32, 1);
            if bonus_gave {
                let bonus_name = world
                    .get_item(bonus_id as u32)
                    .and_then(|i| i.str_name.clone())
                    .unwrap_or_else(|| format!("Bonus #{}", bonus_id));
                if send_legacy_ui {
                    let bonus_notify = build_reward_notify(&bonus_name, 1);
                    session.send_packet(&bonus_notify).await?;
                }
                info!(
                    "[{}] WIZ_ATTENDANCE cumulative milestone day {}: item {} ({})",
                    session.addr(),
                    claimed_so_far,
                    bonus_id,
                    char_name,
                );
            }
        }
    }

    // Persist before returning/refreshing the native calendar. Awaiting this
    // write also closes the duplicate-open race that the former fire-and-forget
    // update allowed.
    repo.update_user_day_with_month(
        &char_name,
        claim_idx as i16,
        true,
        today_day as i16,
        current_month,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to save attendance claim for {char_name}: {e}"))?;

    info!(
        "[{}] WIZ_ATTENDANCE claimed: day {} item {}×{} ({})",
        session.addr(),
        claim_idx,
        item_id,
        count,
        char_name,
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, PacketReader};

    #[test]
    fn test_attendance_opcode_value() {
        assert_eq!(Opcode::WizAttendance as u8, 0xB7);
        assert_eq!(Opcode::from_byte(0xB7), Some(Opcode::WizAttendance));
    }

    #[test]
    fn test_build_open_panel() {
        let pkt = build_open_panel(5);
        assert_eq!(pkt.opcode, 0xB7);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT)); // sub=1
        assert_eq!(r.read_u8(), Some(RESULT_OPEN_PANEL)); // result=0
        assert_eq!(r.read_u8(), Some(5)); // day_index
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_item_added() {
        let pkt = build_item_added(10, 1, 0);
        assert_eq!(pkt.opcode, 0xB7);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT)); // sub=1
        assert_eq!(r.read_u8(), Some(RESULT_ITEM_ADDED)); // result=1
        assert_eq!(r.read_u8(), Some(10)); // day_index
        assert_eq!(r.read_u16(), Some(1)); // item_count
        assert_eq!(r.read_u8(), Some(0)); // complete_flag
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_item_added_complete() {
        let pkt = build_item_added(24, 1, 1);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT));
        assert_eq!(r.read_u8(), Some(RESULT_ITEM_ADDED));
        assert_eq!(r.read_u8(), Some(24));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(1)); // complete on last day
    }

    #[test]
    fn test_build_item_added_with_count() {
        let pkt = build_item_added(5, 3, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT));
        assert_eq!(r.read_u8(), Some(RESULT_ITEM_ADDED));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u16(), Some(3)); // count = 3
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_build_result_msg_timer() {
        let pkt = build_result_msg(RESULT_TIMER);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT));
        assert_eq!(r.read_u8(), Some(3)); // timer message
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_result_msg_already_claimed() {
        let pkt = build_result_msg(RESULT_ALREADY_CLAIMED);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT));
        assert_eq!(r.read_u8(), Some(4)); // already claimed
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_result_msg_inventory_full() {
        let pkt = build_result_msg(RESULT_INVENTORY_FULL);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_RESULT));
        assert_eq!(r.read_u8(), Some(5)); // inventory full
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_reward_notify() {
        let pkt = build_reward_notify("Test Item", 0);
        assert_eq!(pkt.opcode, 0xB7);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_REWARD_NOTIFY)); // sub=4
        assert_eq!(r.read_u8(), Some(1)); // sub_result (must be 1)
        let name = r.read_string();
        assert!(name.is_some());
        assert_eq!(name.unwrap(), "Test Item");
        assert_eq!(r.read_u8(), Some(0)); // tier_flag
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_reward_notify_cumulative() {
        let pkt = build_reward_notify("Milestone Reward", 1);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_REWARD_NOTIFY));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_string().unwrap(), "Milestone Reward");
        assert_eq!(r.read_u8(), Some(1)); // tier=1 for cumulative
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_claim_result_success() {
        let pkt = build_claim_result(1);
        assert_eq!(pkt.opcode, 0xB7);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_CLAIM_RESULT)); // sub=8
        assert_eq!(r.read_u8(), Some(1)); // success — destroy panel
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_claim_result_refresh() {
        let pkt = build_claim_result(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ATT_SUB_CLAIM_RESULT));
        assert_eq!(r.read_u8(), Some(0)); // refresh 3D display
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_c2s_open_format() {
        // Client sends: [0xB7][0x01] — 1 byte payload
        let mut pkt = Packet::new(Opcode::WizAttendance as u8);
        pkt.write_u8(1);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub=1 open
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_c2s_claim_format() {
        // Client sends: [0xB7][0x08] — 1 byte payload
        let mut pkt = Packet::new(Opcode::WizAttendance as u8);
        pkt.write_u8(8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(8)); // sub=8 claim
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_slot_window_calculation() {
        // Test window centering logic
        // first_unclaimed=0: window starts at 0
        assert_eq!(calc_window_start(0), 0);
        // first_unclaimed=3: window starts at 0 (< 4)
        assert_eq!(calc_window_start(3), 0);
        // first_unclaimed=4: window starts at 0 (4 - 4 = 0)
        assert_eq!(calc_window_start(4), 0);
        // first_unclaimed=10: window starts at 6 (10 - 4)
        assert_eq!(calc_window_start(10), 6);
        // first_unclaimed=23: window starts at 16 (25 - 9)
        assert_eq!(calc_window_start(23), 16);
        // first_unclaimed=24: window starts at 16
        assert_eq!(calc_window_start(24), 16);
    }

    #[test]
    fn test_attendance_owned_sub_type_constants() {
        // Only sub-types dispatched to the native Attendance panel are kept.
        assert_eq!(ATT_SUB_RESULT, 1);
        assert_eq!(ATT_SUB_BOARD_DATA, 3);
        assert_eq!(ATT_SUB_REWARD_NOTIFY, 4);
        assert_eq!(ATT_SUB_CLAIM_RESULT, 8);
    }

    #[test]
    fn test_cumulative_milestones() {
        // Verify milestone days
        assert_eq!(CUM_MILESTONE_1, 7);
        assert_eq!(CUM_MILESTONE_2, 14);
        assert_eq!(CUM_MILESTONE_3, 21);
    }

    #[test]
    fn test_needs_monthly_reset_no_claims() {
        // No claims: no reset needed
        let rows: Vec<ko_db::models::daily_reward::DailyRewardUserRow> = vec![];
        assert!(!needs_monthly_reset(&rows, 3));
    }

    #[test]
    fn test_needs_monthly_reset_same_month() {
        let rows = vec![ko_db::models::daily_reward::DailyRewardUserRow {
            user_id: "test".to_string(),
            day_index: 0,
            claimed: true,
            day_of_month: 5,
            last_claim_month: 3,
        }];
        // Same month (3) — no reset
        assert!(!needs_monthly_reset(&rows, 3));
    }

    #[test]
    fn test_needs_monthly_reset_different_month() {
        let rows = vec![ko_db::models::daily_reward::DailyRewardUserRow {
            user_id: "test".to_string(),
            day_index: 0,
            claimed: true,
            day_of_month: 15,
            last_claim_month: 2,
        }];
        // Different month (2 vs 3) — needs reset
        assert!(needs_monthly_reset(&rows, 3));
    }

    #[test]
    fn test_needs_monthly_reset_unclaimed_old_month() {
        let rows = vec![ko_db::models::daily_reward::DailyRewardUserRow {
            user_id: "test".to_string(),
            day_index: 0,
            claimed: false,
            day_of_month: 0,
            last_claim_month: 2,
        }];
        // Unclaimed row with old month — no reset (only check claimed rows)
        assert!(!needs_monthly_reset(&rows, 3));
    }

    #[test]
    fn test_needs_monthly_reset_zero_month() {
        let rows = vec![ko_db::models::daily_reward::DailyRewardUserRow {
            user_id: "test".to_string(),
            day_index: 0,
            claimed: true,
            day_of_month: 5,
            last_claim_month: 0,
        }];
        // Month=0 means never set — skip (don't reset)
        assert!(!needs_monthly_reset(&rows, 3));
    }
}
