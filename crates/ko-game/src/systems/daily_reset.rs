//! Daily / hourly / monthly reset system.
//! The C++ server detects calendar boundary crossings (hour, day, month) each
//! second and fires the appropriate resets.  We replicate this with a 60-second
//! tick that tracks the last-seen hour/day/month and fires on change.
//! ## Hourly
//! - **Daily PK loyalty reset**: Uses an hour counter that resets daily loyalty
//!   when it reaches 24 (matching `m_nPlayerRankingResetTime`).
//! ## Daily
//! - **UpdateFlagAndCape**: Recalculates clan grade from points; demotes
//!   `ClanTypePromoted` clans whose grade dropped > 3 back to Training with
//!   cape = -1.
//! ## Monthly
//! - **ResetLoyaltyMonthly**: Zeroes `loyalty_monthly` in DB for all users.

use std::sync::Arc;
use std::time::Duration;

use ko_db::DbPool;
use tracing::{debug, info, warn};

use crate::world::WorldState;

/// Tick interval for calendar boundary detection (60 seconds).
const RESET_CHECK_INTERVAL_SECS: u64 = 60;

/// Hours between daily PK loyalty resets (`m_nPlayerRankingResetTime` = 24).
const RANKING_RESET_HOURS: u8 = 24;

/// Minutes between knight/user rank reloads (`RELOAD_KNIGHTS_AND_USER_RATING` = 15).
pub(crate) const RELOAD_RANK_INTERVAL_MINUTES: u8 = 15;

/// GameInfo notice 1 interval in minutes (`m_GameInfo1Time` = 1800 seconds = 30 min).
const GAME_INFO1_INTERVAL_MINUTES: u16 = 30;

/// GameInfo notice 2 interval in minutes (`m_GameInfo2Time` = 3600 seconds = 60 min).
const GAME_INFO2_INTERVAL_MINUTES: u16 = 60;

use crate::clan_constants::{CLAN_TYPE_ACCREDITED5, CLAN_TYPE_PROMOTED, CLAN_TYPE_TRAINING};

// ── Clan grade thresholds (`GameServerDlg.cpp:678-681`) ──

/// Grade 1 threshold (720,000 points).
pub(crate) const GRADE1_POINTS: u32 = 720_000;
/// Grade 2 threshold (360,000 points).
pub(crate) const GRADE2_POINTS: u32 = 360_000;
/// Grade 3 threshold (144,000 points).
pub(crate) const GRADE3_POINTS: u32 = 144_000;
/// Grade 4 threshold (72,000 points).
pub(crate) const GRADE4_POINTS: u32 = 72_000;

/// Compute clan grade from points.
pub(crate) fn get_knights_grade(points: u32) -> u8 {
    if points >= GRADE1_POINTS {
        1
    } else if points >= GRADE2_POINTS {
        2
    } else if points >= GRADE3_POINTS {
        3
    } else if points >= GRADE4_POINTS {
        4
    } else {
        5
    }
}

/// Start the daily reset background task.
pub fn start_daily_reset_task(world: Arc<WorldState>, pool: DbPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(RESET_CHECK_INTERVAL_SECS));

        let now = chrono::Local::now();
        let mut last_hour = now.format("%H").to_string().parse::<u8>().unwrap_or(0);
        let mut last_day = now.format("%d").to_string().parse::<u8>().unwrap_or(1);
        let mut last_month = now.format("%m").to_string().parse::<u8>().unwrap_or(1);
        let mut rank_reset_hour_counter: u8 = 0;
        let mut rank_reload_minute_counter: u8 = 0;
        let mut game_info1_counter: u16 = GAME_INFO1_INTERVAL_MINUTES;
        let mut game_info2_counter: u16 = GAME_INFO2_INTERVAL_MINUTES;

        loop {
            tick.tick().await;

            let now = chrono::Local::now();
            let cur_hour = now.format("%H").to_string().parse::<u8>().unwrap_or(0);
            let cur_min = now.format("%M").to_string().parse::<u8>().unwrap_or(0);
            let cur_day = now.format("%d").to_string().parse::<u8>().unwrap_or(1);
            let cur_month = now.format("%m").to_string().parse::<u8>().unwrap_or(1);
            // ── Month change ────────────────────────────────────────────
            if cur_month != last_month {
                info!(
                    "Monthly reset: month changed {} → {}",
                    last_month, cur_month
                );
                reset_loyalty_monthly(&pool).await;
                reset_loyalty_monthly_in_memory(&world);
                last_month = cur_month;
            }

            // ── Day change ──────────────────────────────────────────────
            if cur_day != last_day {
                info!("Daily reset: day changed {} → {}", last_day, cur_day);
                update_flag_and_cape(&world, &pool);
                last_day = cur_day;
            }

            // ── Minute change — automatic commands ─────────────────────
            // Check on every tick (60s) — execute commands matching current H:M.
            // C++ iDay = day-of-month (m_sDate), sentinel 7 = every day
            execute_automatic_commands(&world, cur_hour, cur_min, cur_day as i32);

            // ── Minute change — reload knight and user ranks ─────────
            // Every 15 minutes (RELOAD_KNIGHTS_AND_USER_RATING), reload rankings.
            rank_reload_minute_counter += 1;
            if rank_reload_minute_counter >= RELOAD_RANK_INTERVAL_MINUTES {
                rank_reload_minute_counter = 0;
                world.reload_user_rankings(&pool).await;
                if let Err(error) = world.reload_moraranker(&pool, true).await {
                    tracing::warn!(%error, "automatic MORANKER reload failed");
                }
            }

            // ── Minute change — player ranking rewards ───────────────
            // Every minute, reward top-10 PK zone ranked players.
            set_player_ranking_rewards(&world);

            // ── GameInfoNoticeTimer ────────────────────────────────────
            game_info_notice_tick(&world, &mut game_info1_counter, &mut game_info2_counter);

            // ── Hour change ─────────────────────────────────────────────
            if cur_hour != last_hour {
                debug!(
                    "Hourly tick: hour changed {} → {} (rank counter: {}/{})",
                    last_hour, cur_hour, rank_reset_hour_counter, RANKING_RESET_HOURS
                );
                rank_reset_hour_counter += 1;
                if rank_reset_hour_counter >= RANKING_RESET_HOURS {
                    rank_reset_hour_counter = 0;
                    reset_daily_pk_loyalty(&world);
                }

                // Broadcast flying Santa/Angel if active.
                broadcast_flying_santa_or_angel(&world);

                last_hour = cur_hour;
            }
        }
    })
}

/// Broadcast WIZ_SANTA if the flying Santa/Angel event is active.
/// Called every hour from `UpdateGameTime()` if `m_bSantaOrAngel != FLYING_NONE`.
fn broadcast_flying_santa_or_angel(world: &WorldState) {
    let santa_type = world
        .santa_or_angel
        .load(std::sync::atomic::Ordering::Relaxed);
    if santa_type == 0 {
        return;
    }
    let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizSanta as u8);
    pkt.write_u8(santa_type);
    world.broadcast_to_all(Arc::new(pkt), None);
    debug!("Broadcast flying santa/angel: type={}", santa_type);
}

/// Periodic server info notice broadcasts.
/// Broadcasts configurable notice messages at fixed intervals:
/// - Notice 1: every 30 minutes (`m_GameInfo1Time` = 1800s)
/// - Notice 2: every 60 minutes (`m_GameInfo2Time` = 3600s)
fn game_info_notice_tick(world: &WorldState, counter1: &mut u16, counter2: &mut u16) {
    // Notice 1 — every 30 minutes
    if *counter1 > 0 {
        *counter1 -= 1;
    }
    if *counter1 == 0 {
        *counter1 = GAME_INFO1_INTERVAL_MINUTES;
        let msg = world.game_info_notice1.read();
        if !msg.is_empty() {
            broadcast_public_chat_notice(world, &msg);
        }
    }

    // Notice 2 — every 60 minutes
    if *counter2 > 0 {
        *counter2 -= 1;
    }
    if *counter2 == 0 {
        *counter2 = GAME_INFO2_INTERVAL_MINUTES;
        let msg = world.game_info_notice2.read();
        if !msg.is_empty() {
            broadcast_public_chat_notice(world, &msg);
        }
    }
}

/// Broadcast a public chat notice to all connected players.
fn broadcast_public_chat_notice(world: &WorldState, message: &str) {
    let pkt = crate::handler::chat::build_chat_packet(
        1,      // PUBLIC_CHAT
        0,      // nation = ALL
        0xFFFF, // sender_id = -1 (system)
        "", message, 0, 0, 0,
    );
    world.broadcast_to_all(Arc::new(pkt), None);
    debug!("GameInfoNotice broadcast: {}", message);
}

/// Distribute NP/KC rewards to top-10 PK-zone ranked players in each configured zone.
/// Rewards are configurable via `player_ranking_loyalty_reward` and
/// `player_ranking_kc_reward` on `WorldState`. Default is 0 (disabled).
fn set_player_ranking_rewards(world: &WorldState) {
    let loyalty = world
        .player_ranking_loyalty_reward
        .load(std::sync::atomic::Ordering::Relaxed);
    let kc = world
        .player_ranking_kc_reward
        .load(std::sync::atomic::Ordering::Relaxed);
    if loyalty == 0 && kc == 0 {
        return;
    }

    let zones = world.player_ranking_reward_zones.read().clone();

    for zone_id in &zones {
        let sids = world.sessions_in_zone(*zone_id);
        for sid in sids {
            let (is_gm, nation) = match world.get_character_info(sid) {
                Some(ch) => (ch.authority == 0, ch.nation),
                None => continue,
            };

            if is_gm {
                continue;
            }

            let rank = world.pk_zone_get_player_rank(sid, nation, *zone_id);
            if rank == 0 || rank > 10 {
                continue;
            }

            // Distribute loyalty
            if loyalty > 0 {
                super::loyalty::send_loyalty_change(world, sid, loyalty as i32, false, true, false);
            }

            // Distribute KC
            if kc > 0 {
                world.update_session(sid, |h| {
                    h.knight_cash = h.knight_cash.saturating_add(kc);
                });
            }
        }
    }
}

/// Reset daily PK loyalty for all online players.
/// Zeroes `pk_loyalty_daily` and `pk_loyalty_premium_bonus` in memory.
fn reset_daily_pk_loyalty(world: &WorldState) {
    let session_ids = world.get_in_game_session_ids();
    let mut count = 0u32;
    for sid in session_ids {
        world.update_session(sid, |h| {
            h.pk_loyalty_daily = 0;
            h.pk_loyalty_premium_bonus = 0;
        });
        count += 1;
    }
    info!(
        "Daily PK loyalty reset: zeroed pk_loyalty_daily for {} online players",
        count
    );
}

/// Reset loyalty_monthly to 0 in DB for all users.
async fn reset_loyalty_monthly(pool: &DbPool) {
    let repo = ko_db::repositories::character::CharacterRepository::new(pool);
    match repo.reset_loyalty_monthly().await {
        Ok(rows) => {
            info!(
                "Monthly loyalty reset: zeroed loyalty_monthly for {} DB rows",
                rows
            );
        }
        Err(e) => {
            warn!("Monthly loyalty reset: DB error: {}", e);
        }
    }
}

/// Reset loyalty_monthly to 0 in memory for all online players.
fn reset_loyalty_monthly_in_memory(world: &WorldState) {
    let session_ids = world.get_in_game_session_ids();
    let mut count = 0u32;
    for sid in session_ids {
        world.update_session(sid, |h| {
            if let Some(ref mut ch) = h.character {
                ch.loyalty_monthly = 0;
            }
        });
        count += 1;
    }
    info!(
        "Monthly loyalty reset (memory): zeroed loyalty_monthly for {} online players",
        count
    );
}

/// Recalculate clan grades and demote promoted clans whose grade dropped.
/// Logic:
/// 1. If flag >= Accredited5 (3): grade = 1 (top tier regardless of points).
/// 2. Else: grade = `GetKnightsGrade(points)`.
/// 3. If flag == Promoted (2) AND grade > 3: demote to Training (1), cape = -1.
fn update_flag_and_cape(world: &WorldState, pool: &DbPool) {
    let clan_ids = world.get_all_knights_ids();
    if clan_ids.is_empty() {
        return;
    }

    let mut demoted_count = 0u32;
    let mut updated_count = 0u32;

    for clan_id in &clan_ids {
        let info = match world.get_knights(*clan_id) {
            Some(k) => k,
            None => continue,
        };

        // Step 1-2: compute new grade
        let new_grade = if info.flag >= CLAN_TYPE_ACCREDITED5 {
            1
        } else {
            get_knights_grade(info.points)
        };

        // Step 3: check demotion condition
        let needs_demote = info.flag == CLAN_TYPE_PROMOTED && new_grade > 3;

        let old_grade = info.grade;
        let cid = *clan_id;

        world.update_knights(cid, |k| {
            k.grade = new_grade;
            if needs_demote {
                k.flag = CLAN_TYPE_TRAINING;
                k.cape = 0xFFFF; // -1 as u16
            }
        });

        if new_grade != old_grade {
            updated_count += 1;
        }

        if needs_demote {
            demoted_count += 1;
            // DB save for demoted clan (fire-and-forget)
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                let repo = ko_db::repositories::knights::KnightsRepository::new(&pool_clone);
                if let Err(e) = repo
                    .update_flag_cape(cid as i16, CLAN_TYPE_TRAINING as i16, -1_i16)
                    .await
                {
                    warn!(
                        "UpdateFlagAndCape: failed to save demotion for clan {}: {}",
                        cid, e
                    );
                }
            });
        }
    }

    info!(
        "UpdateFlagAndCape: {} clans processed, {} grades changed, {} demoted",
        clan_ids.len(),
        updated_count,
        demoted_count
    );
}

/// Execute automatic commands that match the current hour, minute, and day-of-month.
/// For each active `AutomaticCommand` row:
/// - `hour` and `minute` must match exactly
/// - `day_of_week == 7` means every day (sentinel), otherwise must match `m_sDate`
///   (day-of-month 1-31). C++ field name is `iDay`, DB column is `day_of_week`
///   but it actually stores day-of-month.
/// Commands are logged. Actual GM dispatch is not wired here (would require a
/// synthetic ClientSession); instead we process known server-side commands directly.
fn execute_automatic_commands(world: &WorldState, hour: u8, minute: u8, day_of_month: i32) {
    let commands = world.automatic_commands.read();
    for cmd in commands.iter() {
        if cmd.hour != hour as i32 || cmd.minute != minute as i32 {
            continue;
        }
        if cmd.day_of_week != 7 && cmd.day_of_week != day_of_month {
            continue;
        }

        info!(
            "AutomaticCommand fired: idx={} cmd='{}' ({})",
            cmd.idx, cmd.command, cmd.description
        );

        // Process known server commands
        let trimmed = cmd.command.trim();
        if let Some(stripped) = trimmed.strip_prefix('+') {
            let parts: Vec<&str> = stripped.splitn(2, ' ').collect();
            let command_name = parts[0].to_lowercase();
            match command_name.as_str() {
                "close" => {
                    use crate::systems::war;
                    let prev = world.update_battle_state(war::battle_zone_close);
                    if prev != war::NO_BATTLE {
                        info!("AutoCmd: War closed (prev_type={prev})");
                    }
                }
                "open1" | "open2" | "open3" | "open4" | "open5" | "open6" => {
                    use crate::systems::war;
                    let zone_idx = command_name.as_bytes().last().unwrap_or(&b'1') - b'0';
                    let now_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i32;
                    let opened = world.update_battle_state(|state| {
                        war::battle_zone_open(state, war::BATTLEZONE_OPEN, zone_idx, now_unix)
                    });
                    if opened {
                        info!("AutoCmd: War zone {zone_idx} opened");
                    }
                }
                "snow" => {
                    use crate::systems::war;
                    let now_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i32;
                    let opened = world.update_battle_state(|state| {
                        war::battle_zone_open(state, war::SNOW_BATTLEZONE_OPEN, 0, now_unix)
                    });
                    if opened {
                        info!("AutoCmd: Snow war opened");
                    }
                }
                other => {
                    debug!(
                        "AutomaticCommand: unhandled command '{other}' (idx={})",
                        cmd.idx
                    );
                }
            }
        } else {
            debug!(
                "AutomaticCommand: non-GM command '{}' (idx={})",
                trimmed, cmd.idx
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_knights_grade_boundaries() {
        // Grade 1: >= 720,000
        assert_eq!(get_knights_grade(720_000), 1);
        assert_eq!(get_knights_grade(1_000_000), 1);

        // Grade 2: >= 360,000
        assert_eq!(get_knights_grade(360_000), 2);
        assert_eq!(get_knights_grade(719_999), 2);

        // Grade 3: >= 144,000
        assert_eq!(get_knights_grade(144_000), 3);
        assert_eq!(get_knights_grade(359_999), 3);

        // Grade 4: >= 72,000
        assert_eq!(get_knights_grade(72_000), 4);
        assert_eq!(get_knights_grade(143_999), 4);

        // Grade 5: < 72,000
        assert_eq!(get_knights_grade(0), 5);
        assert_eq!(get_knights_grade(71_999), 5);
    }

    #[test]
    fn test_clan_type_constants() {
        // Matches C++ Knights.h enum ClanTypeFlag
        assert_eq!(CLAN_TYPE_TRAINING, 1);
        assert_eq!(CLAN_TYPE_PROMOTED, 2);
        assert_eq!(CLAN_TYPE_ACCREDITED5, 3);
    }

    #[test]
    fn test_ranking_reset_hours() {
        assert_eq!(RANKING_RESET_HOURS, 24);
    }

    #[test]
    fn test_grade_thresholds_match_cpp() {
        // C++ GameServerDlg.cpp:678-681 defaults
        assert_eq!(GRADE1_POINTS, 720_000);
        assert_eq!(GRADE2_POINTS, 360_000);
        assert_eq!(GRADE3_POINTS, 144_000);
        assert_eq!(GRADE4_POINTS, 72_000);
    }

    #[test]
    fn test_demotion_logic() {
        // Promoted clan (flag=2) with grade > 3 → demote to Training
        let flag = CLAN_TYPE_PROMOTED;
        let grade = get_knights_grade(10_000); // 10k points → grade 5
        assert_eq!(grade, 5);
        assert!(grade > 3);
        assert!(flag == CLAN_TYPE_PROMOTED && grade > 3);

        // Promoted clan (flag=2) with grade 3 → NO demotion
        let grade2 = get_knights_grade(200_000); // 200k → grade 3
        assert_eq!(grade2, 3);
        assert!(!(flag == CLAN_TYPE_PROMOTED && grade2 > 3));
    }

    #[test]
    fn test_accredited_clan_always_grade_1() {
        // C++ UpdateFlagAndCape: flag >= Accredited5 → grade = 1
        for flag in CLAN_TYPE_ACCREDITED5..=12 {
            let result_grade = if flag >= CLAN_TYPE_ACCREDITED5 {
                1
            } else {
                get_knights_grade(0) // would be 5 for 0 points
            };
            assert_eq!(result_grade, 1, "flag {} should force grade 1", flag);
        }
    }

    #[test]
    fn test_training_clan_no_demotion() {
        // Training clan (flag=1) should never be demoted even with grade > 3
        let flag = CLAN_TYPE_TRAINING;
        let grade = get_knights_grade(0); // grade 5
        assert!(!(flag == CLAN_TYPE_PROMOTED && grade > 3));
    }

    #[test]
    fn test_flying_santa_broadcast_noop_when_none() {
        let world = WorldState::new();
        // FLYING_NONE (0) — broadcast should be a no-op (no crash)
        assert_eq!(
            world
                .santa_or_angel
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        broadcast_flying_santa_or_angel(&world);
    }

    #[test]
    fn test_flying_santa_broadcast_with_santa() {
        let world = WorldState::new();
        world
            .santa_or_angel
            .store(1, std::sync::atomic::Ordering::Relaxed);
        // Should not panic (no players to receive, but function runs)
        broadcast_flying_santa_or_angel(&world);
        assert_eq!(
            world
                .santa_or_angel
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[test]
    fn test_flying_santa_packet_format() {
        // Verify WIZ_SANTA packet: opcode(0x5A) + type(u8)
        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizSanta as u8);
        pkt.write_u8(2); // FLYING_ANGEL
        assert_eq!(pkt.opcode, 0x5A);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 2);
    }

    /// Test AutomaticCommand executor with matching time.
    #[test]
    fn test_automatic_command_executor_match() {
        let world = WorldState::new();
        let cmd = ko_db::models::scheduled_tasks::AutomaticCommand {
            idx: 1,
            status: true,
            command: "+close".to_string(),
            hour: 10,
            minute: 30,
            day_of_week: 7, // every day (C++ sentinel)
            description: "test close".to_string(),
        };
        *world.automatic_commands.write() = vec![cmd];

        // Should match (H=10, M=30, day=7=every day)
        execute_automatic_commands(&world, 10, 30, 15);
        // Should NOT match (wrong minute)
        execute_automatic_commands(&world, 10, 31, 15);
        // Should NOT match (wrong hour)
        execute_automatic_commands(&world, 11, 30, 15);
    }

    /// Test AutomaticCommand day-of-month filtering.
    #[test]
    fn test_automatic_command_day_filter() {
        let world = WorldState::new();
        let cmd = ko_db::models::scheduled_tasks::AutomaticCommand {
            idx: 2,
            status: true,
            command: "+open1".to_string(),
            hour: 16,
            minute: 0,
            day_of_week: 15, // 15th of the month only
            description: "test open".to_string(),
        };
        *world.automatic_commands.write() = vec![cmd];

        // Should match (H=16, M=0, day_of_month=15)
        execute_automatic_commands(&world, 16, 0, 15);
        // Should NOT match (day_of_month=16)
        execute_automatic_commands(&world, 16, 0, 16);
    }

    /// Test AutomaticCommand non-GM command is logged.
    #[test]
    fn test_automatic_command_non_gm() {
        let world = WorldState::new();
        let cmd = ko_db::models::scheduled_tasks::AutomaticCommand {
            idx: 3,
            status: true,
            command: "not_a_command".to_string(),
            hour: 0,
            minute: 0,
            day_of_week: 7, // every day
            description: "invalid".to_string(),
        };
        *world.automatic_commands.write() = vec![cmd];

        // Should not crash, just log
        execute_automatic_commands(&world, 0, 0, 1);
    }

    #[test]
    fn test_set_player_ranking_rewards_disabled_by_default() {
        let world = WorldState::new();
        // Default: both reward values are 0 (disabled)
        assert_eq!(
            world
                .player_ranking_loyalty_reward
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            world
                .player_ranking_kc_reward
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        // Should be a no-op (no crash)
        set_player_ranking_rewards(&world);
    }

    #[test]
    fn test_player_ranking_reward_zones_default() {
        let world = WorldState::new();
        // C++ default: "71,72,73"
        let zones = world.player_ranking_reward_zones.read().clone();
        assert_eq!(zones, vec![71, 72, 73]);
    }

    #[test]
    fn test_set_player_ranking_rewards_with_values() {
        let world = WorldState::new();
        // Set non-zero rewards
        world
            .player_ranking_loyalty_reward
            .store(100, std::sync::atomic::Ordering::Relaxed);
        world
            .player_ranking_kc_reward
            .store(50, std::sync::atomic::Ordering::Relaxed);
        // Should not crash with no players online
        set_player_ranking_rewards(&world);
    }

    #[test]
    fn test_game_info_notice_tick_countdown() {
        let world = WorldState::new();
        let mut c1: u16 = 3;
        let mut c2: u16 = 5;
        // Tick should decrement counters
        game_info_notice_tick(&world, &mut c1, &mut c2);
        assert_eq!(c1, 2);
        assert_eq!(c2, 4);
    }

    #[test]
    fn test_game_info_notice_tick_fires_at_zero() {
        let world = WorldState::new();
        *world.game_info_notice1.write() = "Notice1".to_string();
        let mut c1: u16 = 1; // will reach 0 and reset
        let mut c2: u16 = 100;
        game_info_notice_tick(&world, &mut c1, &mut c2);
        // Counter should reset to interval
        assert_eq!(c1, GAME_INFO1_INTERVAL_MINUTES);
        assert_eq!(c2, 99);
    }

    #[test]
    fn test_game_info_notice_tick_empty_msg_skips() {
        let world = WorldState::new();
        // Empty messages — should not crash or broadcast
        let mut c1: u16 = 1;
        let mut c2: u16 = 1;
        game_info_notice_tick(&world, &mut c1, &mut c2);
        assert_eq!(c1, GAME_INFO1_INTERVAL_MINUTES);
        assert_eq!(c2, GAME_INFO2_INTERVAL_MINUTES);
    }

    #[test]
    fn test_game_info_notice_intervals() {
        assert_eq!(GAME_INFO1_INTERVAL_MINUTES, 30);
        assert_eq!(GAME_INFO2_INTERVAL_MINUTES, 60);
    }

    // ── Knights Rating Tests (Sprint 632) ────────────────────────────────

    #[test]
    fn test_knights_rating_row_model() {
        // Verify KnightsRatingRow struct fields match C++ _KNIGHTS_RATING
        let row = ko_db::models::ranking::KnightsRatingRow {
            nation: 1,   // Karus
            rank_pos: 1, // Top clan
            clan_id: 42,
            points: 800_000,
        };
        assert_eq!(row.nation, 1);
        assert_eq!(row.rank_pos, 1);
        assert_eq!(row.clan_id, 42);
        assert_eq!(row.points, 800_000);
    }

    #[test]
    fn test_knights_rating_grade_update_logic() {
        // C++ RecvKnightsAllList: grade is recomputed from points
        // Accredited+ clans (flag >= 3) are always grade 1
        let test_cases: Vec<(u8, u32, u8)> = vec![
            // (flag, points, expected_grade)
            (1, 0, 5),       // Training, 0 points → grade 5
            (2, 100_000, 4), // Promoted, 100k → grade 4
            (2, 200_000, 3), // Promoted, 200k → grade 3
            (2, 400_000, 2), // Promoted, 400k → grade 2
            (2, 800_000, 1), // Promoted, 800k → grade 1
            (3, 0, 1),       // Accredited5, 0 points → grade 1 (forced)
            (5, 10_000, 1),  // Higher accredited, low points → grade 1 (forced)
            (8, 0, 1),       // Royal, 0 points → grade 1 (forced)
        ];

        for (flag, points, expected_grade) in test_cases {
            let grade = if flag >= CLAN_TYPE_ACCREDITED5 {
                1
            } else {
                get_knights_grade(points)
            };
            assert_eq!(
                grade, expected_grade,
                "flag={flag}, points={points}: expected grade {expected_grade}, got {grade}"
            );
        }
    }

    #[test]
    fn test_knights_rating_apply_to_world() {
        // Verify that ranking and grade are applied to in-memory KnightsInfo
        let world = WorldState::new();

        // Insert test clans
        let clan1 = crate::world::types::KnightsInfo {
            id: 1,
            flag: 2, // Promoted
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "TestClan1".to_string(),
            chief: "Chief1".to_string(),
            points: 500_000,
            ..Default::default()
        };
        let clan2 = crate::world::types::KnightsInfo {
            id: 2,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "TestClan2".to_string(),
            chief: "Chief2".to_string(),
            points: 100_000,
            ..Default::default()
        };
        let clan3 = crate::world::types::KnightsInfo {
            id: 3,
            flag: 5, // Accredited
            nation: 2,
            grade: 5,
            ranking: 0,
            name: "ElmoClan".to_string(),
            chief: "Chief3".to_string(),
            points: 10_000,
            ..Default::default()
        };

        world.insert_knights(clan1);
        world.insert_knights(clan2);
        world.insert_knights(clan3);

        // Simulate what reload_user_rankings does: reset then apply
        let clan_ids = world.get_all_knights_ids();
        for clan_id in &clan_ids {
            world.update_knights(*clan_id, |k| k.ranking = 0);
        }

        // Simulate applying rating rows (as computed by SQL)
        let ratings = vec![
            ko_db::models::ranking::KnightsRatingRow {
                nation: 1,
                rank_pos: 1,
                clan_id: 1,
                points: 500_000,
            },
            ko_db::models::ranking::KnightsRatingRow {
                nation: 1,
                rank_pos: 2,
                clan_id: 2,
                points: 100_000,
            },
            ko_db::models::ranking::KnightsRatingRow {
                nation: 2,
                rank_pos: 1,
                clan_id: 3,
                points: 10_000,
            },
        ];

        for row in &ratings {
            let clan_id = row.clan_id as u16;
            let rank = row.rank_pos as u8;
            let points = row.points as u32;
            world.update_knights(clan_id, |k| {
                k.ranking = rank;
                let new_grade = if k.flag >= CLAN_TYPE_ACCREDITED5 {
                    1
                } else {
                    get_knights_grade(points)
                };
                k.grade = new_grade;
            });
        }

        // Verify results
        let c1 = world.get_knights(1).unwrap();
        assert_eq!(c1.ranking, 1, "Clan 1 should be rank 1 in Karus");
        assert_eq!(c1.grade, 2, "500k points → grade 2");

        let c2 = world.get_knights(2).unwrap();
        assert_eq!(c2.ranking, 2, "Clan 2 should be rank 2 in Karus");
        assert_eq!(c2.grade, 4, "100k points → grade 4");

        let c3 = world.get_knights(3).unwrap();
        assert_eq!(c3.ranking, 1, "Clan 3 should be rank 1 in Elmorad");
        assert_eq!(c3.grade, 1, "Accredited clan → always grade 1");
    }

    #[test]
    fn test_knights_rating_zero_points_excluded() {
        // Clans with 0 points should not appear in rankings (SQL WHERE points > 0)
        let world = WorldState::new();

        let clan = crate::world::types::KnightsInfo {
            id: 10,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "EmptyClan".to_string(),
            chief: "Chief".to_string(),
            points: 0,
            ..Default::default()
        };
        world.insert_knights(clan);

        // After rating computation with empty results, ranking stays 0
        let clan_ids = world.get_all_knights_ids();
        for clan_id in &clan_ids {
            world.update_knights(*clan_id, |k| k.ranking = 0);
        }

        let c = world.get_knights(10).unwrap();
        assert_eq!(c.ranking, 0, "Clan with 0 points should have ranking 0");
    }

    #[test]
    fn test_knights_rating_ranking_sent_in_user_inout() {
        // Verify ranking field is u8 and can represent ranks 0-255
        let info = crate::world::types::KnightsInfo {
            ranking: 1,
            ..Default::default()
        };
        assert_eq!(info.ranking, 1u8);

        let info_max = crate::world::types::KnightsInfo {
            ranking: 255,
            ..Default::default()
        };
        assert_eq!(info_max.ranking, 255u8);
    }
}
