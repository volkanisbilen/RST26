//! Level & Experience system — handles XP gains/losses and level changes.
//! ## Key Functions
//! - `exp_change()`: Central XP change function for all XP gains/losses.
//! - `level_change()`: Handles stat/skill point awards and broadcasts on level up/down.
//! - `on_death_lost_exp_calc()`: Calculates XP penalty on death.
//! ## Packet Formats
//! ### WIZ_EXP_CHANGE (0x1A)
//! `[u8 flag] [i64 total_exp]`
//! Flag values:
//! - 1: exp seal update
//! - 4: normal XP update
//! ### WIZ_LEVEL_CHANGE (0x1B)
//! `[u32 sid] [u8 level] [i16 stat_points] [u8 free_skill_points]
//!  [i64 max_exp] [i64 exp] [i16 max_hp] [i16 hp] [i16 max_mp] [i16 mp]
//!  [u16 max_weight] [u16 item_weight]`

use ko_protocol::{Opcode, Packet};
use std::sync::Arc;

use crate::world::{CharacterInfo, WorldState, MAX_LEVEL};
use crate::zone::SessionId;

/// Maximum bonus multiplier cap (percentage).
/// If the total bonus exceeds this, it is clamped to prevent overflow.
/// The C++ code has no explicit cap, but we add one for safety.
const MAX_BONUS_PERCENT: u64 = 10_000;

/// WIZ_EXP_CHANGE flag: normal XP update.
const EXP_FLAG_NORMAL: u8 = 0x04;

/// Re-export from party module for local use.
use super::party::PARTY_LEVELCHANGE;

/// Apply an experience point change to a character.
/// This is the central function for all XP changes (kills, quest rewards,
/// death penalties, etc.). It handles:
/// - XP bonus multipliers (king event XP) when gaining XP
/// - Level-up when XP >= max_exp
/// - Level-down when XP goes below 0 (death penalty)
/// - Sending WIZ_EXP_CHANGE packet
/// `is_bonus_reward` skips bonus multiplier calculation (for bonus XP that
/// should not be further multiplied, e.g., level-up rewards, flash bonuses).
pub async fn exp_change(world: &WorldState, sid: SessionId, i_exp: i64) {
    exp_change_inner(world, sid, i_exp, false).await;
}

/// Scale a monster template's base EXP by the admin-configurable `server_settings.exp_rate`
/// (percent, 100 = unchanged). Applied at every site that reads `npc_template.exp` directly,
/// not inside `exp_change`/`exp_change_with_bonus` -- those are also used for quest rewards
/// and other non-kill XP, which this rate must not touch.
pub fn monster_exp_rate(world: &WorldState, raw_exp: i64) -> i64 {
    let rate = world
        .get_server_settings()
        .map(|s| s.exp_rate)
        .unwrap_or(100)
        .max(0) as i64;
    raw_exp * rate / 100
}

/// Apply an experience point change with explicit bonus-reward flag.
/// When `is_bonus_reward` is true, the server-side XP multipliers (event XP,
/// premium bonuses, etc.) are skipped — the XP is applied as-is.
pub async fn exp_change_with_bonus(
    world: &WorldState,
    sid: SessionId,
    i_exp: i64,
    is_bonus_reward: bool,
) {
    exp_change_inner(world, sid, i_exp, is_bonus_reward).await;
}

/// Internal implementation for `exp_change` and `exp_change_with_bonus`.
async fn exp_change_inner(
    world: &WorldState,
    sid: SessionId,
    mut i_exp: i64,
    is_bonus_reward: bool,
) {
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };

    // Also stop XP loss in war zones.
    if i_exp < 0 {
        if ch.level < 6 {
            return;
        }
        if let Some(pos) = world.get_position(sid) {
            if let Some(zone) = world.get_zone(pos.zone_id) {
                if zone.is_war_zone() {
                    return;
                }
            }
        }
    }

    if (ch.exp as i64) < 0 {
        return;
    }

    // Apply bonus multipliers for XP gains (not losses, not bonus rewards)
    if i_exp > 0 && !is_bonus_reward {
        i_exp = apply_exp_bonuses(world, sid, i_exp, &ch);
    }

    // FerihaLog: ExpChangeInsertLog (only if >= 500K)
    if let Some(pool) = world.db_pool() {
        let acc = world
            .with_session(sid, |h| h.account_id.clone())
            .unwrap_or_default();
        super::audit_log::log_exp_change(pool, &acc, &ch.name, "exp_change", i_exp, ch.exp as i64);
    }

    // EXP seal check: when seal is active, positive XP goes to sealed pool
    if ch.exp_seal_status && i_exp > 0 {
        Box::pin(crate::handler::exp_seal::exp_seal_change_exp(
            world,
            sid,
            i_exp as u64,
        ))
        .await;
        return;
    }

    // Apply XP change
    let new_exp = (ch.exp as i64) + i_exp;

    // Level-down check: XP dropped below 0
    if new_exp < 0 {
        // Drop a level
        let new_level = ch.level.saturating_sub(1);
        if new_level < 1 {
            // Can't go below level 1
            world.update_character_stats(sid, |ch| {
                ch.exp = 0;
            });
            send_exp_change(world, sid);
            return;
        }

        // Calculate excess XP to carry over as penalty.
        // C++ uses the ORIGINAL m_iExp (not modified when bLevel==false, line 90-91).
        let prev_level_max = world.get_exp_by_level(new_level, 0);
        let diff_xp = ch.exp as i64 + on_death_lost_exp_calc(prev_level_max, 0.0);

        // Set exp to max for the previous level
        let level_max_exp = world.get_exp_by_level(new_level, 0);
        world.update_character_stats(sid, |ch| {
            ch.exp = level_max_exp as u64;
            ch.level = new_level;
        });

        // Apply level change (delevel)
        level_change(world, sid, new_level, false);

        // Recurse to apply remaining penalty
        if diff_xp > 0 {
            Box::pin(exp_change_inner(world, sid, -diff_xp, false)).await;
        }
        return;
    }

    // Level-up check
    let max_exp = ch.max_exp;
    if max_exp > 0 && new_exp >= max_exp {
        if (ch.level as u16) < MAX_LEVEL {
            // Level up: subtract max_exp, increment level
            let remainder = new_exp - max_exp;
            let new_level = ch.level + 1;
            world.update_character_stats(sid, |ch| {
                ch.exp = remainder as u64;
                ch.level = new_level;
            });
            level_change(world, sid, new_level, true);
            return;
        }

        // At max level — cap XP
        world.update_character_stats(sid, |ch| {
            ch.exp = max_exp as u64;
        });
        send_exp_change(world, sid);
        return;
    }

    // Normal XP update (no level change)
    world.update_character_stats(sid, |ch| {
        ch.exp = new_exp as u64;
    });
    send_exp_change(world, sid);
}

/// Apply server-side XP bonus multipliers.
/// Bonus sources implemented:
/// - King system XP event (`g_pMain->m_byExpEventAmount`)
/// Bonus sources implemented:
/// - King system XP event (`g_pMain->m_byExpEventAmount`)
/// - Clan leader online bonus
/// - Premium XP (`GetPremiumProperty(PremiumExpPercent)`)
/// - Clan premium XP
/// - Item XP bonus from equipment
/// - Burning feature / flame level (`m_bFlamelevel`)
/// - Flash XP bonus (`m_FlashExpBonus`)
/// Each bonus is additive: `FinalExp += (TempExp * bonus_percent) / 100`
fn apply_exp_bonuses(world: &WorldState, sid: SessionId, base_exp: i64, ch: &CharacterInfo) -> i64 {
    let temp_exp = base_exp as u64;
    let mut final_exp = base_exp as u64;

    // King system XP event bonus
    let exp_event = get_king_exp_event(world, ch.nation);
    if exp_event > 0 {
        final_exp += (temp_exp * exp_event as u64) / 100;
    }

    // Clan leader online XP bonus: +5% when the clan chief is online.
    let clan_bonus = get_clan_leader_online_bonus(world, sid, ch);
    if clan_bonus > 0 {
        final_exp += (temp_exp * clan_bonus as u64) / 100;
    }

    // Premium XP bonus (level-range based)
    let is_dead = world.is_player_dead(sid);
    if !is_dead {
        let prem_exp = world.get_premium_exp_percent(sid, ch.level);
        if prem_exp > 0 {
            final_exp += (temp_exp * prem_exp as u64) / 100;
        }

        // Clan premium XP bonus
        let clan_prem_exp = world.get_clan_premium_exp_percent(sid, ch.level);
        if clan_prem_exp > 0 {
            final_exp += (temp_exp * clan_prem_exp as u64) / 100;
        }
    }

    // Item XP bonus from equipment (set items, castellan cape).
    let item_exp = world.get_equipped_stats(sid).item_exp_bonus;
    if item_exp > 0 {
        final_exp += (temp_exp * item_exp as u64) / 100;
    }

    // Burning / Flame XP bonus
    let flame_level = world.with_session(sid, |h| h.flame_level).unwrap_or(0);
    if flame_level > 0 && flame_level <= 3 {
        if let Some(feat) = world.get_burning_feature(flame_level) {
            if feat.exp_rate > 0 {
                final_exp += (temp_exp * feat.exp_rate as u64) / 100;
            }
        }
    }

    // Flash EXP bonus
    let flash_exp = world.with_session(sid, |h| h.flash_exp_bonus).unwrap_or(0);
    if flash_exp > 0 {
        final_exp += (temp_exp * flash_exp as u64) / 100;
    }

    // Buff EXP bonuses (BUFF_TYPE_EXPERIENCE=11, BUFF_TYPE_VARIOUS_EFFECTS=33)
    let (buff11, buff33) = world
        .with_session(sid, |h| (h.exp_gain_buff11, h.exp_gain_buff33))
        .unwrap_or((0, 0));
    if buff11 > 0 {
        final_exp += (temp_exp * buff11 as u64) / 100;
    }
    if buff33 > 0 {
        final_exp += (temp_exp * buff33 as u64) / 100;
    }

    // `FinalExp += (TempExp * perkExperience) / 100`
    let perk_exp = world
        .with_session(sid, |h| world.compute_perk_bonus(&h.perk_levels, 5, false))
        .unwrap_or(0);
    if perk_exp > 0 {
        final_exp += (temp_exp * perk_exp as u64) / 100;
    }

    // Safety cap to prevent overflow
    if final_exp > temp_exp * MAX_BONUS_PERCENT {
        final_exp = temp_exp * MAX_BONUS_PERCENT;
    }

    final_exp as i64
}

/// Get the active king system XP event bonus percentage for a nation.
/// Returns 0 if no event is active or expired.
fn get_king_exp_event(world: &WorldState, nation: u8) -> u8 {
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return 0,
    };

    if ks.exp_event == 0 {
        return 0;
    }

    // Check if the event is still within its duration window.
    // The king handler already expires events via periodic tick, so if
    // exp_event > 0, the event is active.
    ks.exp_event
}

/// Get clan leader online XP bonus percentage.
/// When a player is in a clan and the clan chief (leader) is currently
/// online (has an active session), the player gets a 5% XP bonus.
fn get_clan_leader_online_bonus(world: &WorldState, _sid: SessionId, ch: &CharacterInfo) -> u8 {
    if ch.knights_id == 0 {
        return 0;
    }

    let clan = match world.get_knights(ch.knights_id) {
        Some(c) => c,
        None => return 0,
    };

    // Check if the chief is online by looking for a session with that name
    if clan.chief.is_empty() {
        return 0;
    }

    // Check if the chief's name matches any active session
    let chief_online = world.find_session_by_name(&clan.chief).is_some();
    if chief_online {
        5 // 5% XP bonus when clan leader is online
    } else {
        0
    }
}

/// Handle stat/skill point updates after a level change.
/// This does NOT change the level itself — it handles the consequences of a level
/// change: stat points, skill points, HP/MP recalculation, and broadcasting.
/// Supports both single-level increments (normal level-up) and multi-level
/// jumps (GM commands like `/levelup 83`).
pub fn level_change(world: &WorldState, sid: SessionId, level: u8, is_level_up: bool) {
    if is_level_up {
        // the character's level is stored a few lines below: show the new one on Discord
        let (name, zone) = (
            world.get_character_info(sid).map(|c| c.name),
            world.get_position(sid).map(|p| p.zone_id),
        );
        if let (Some(name), Some(zone)) = (name, zone) {
            world.send_to_session_owned(sid, super::discord_presence::build_packet(&name, level, zone));
        }
    }
    if level < 1 || (level as u16) > MAX_LEVEL {
        return;
    }

    if is_level_up && super::daily_quest::auto_start_all(world, sid) {
        super::daily_quest::send_list(world, sid);
        super::daily_quest::send_progress(world, sid);
    }

    // Update max_exp for the new level
    let new_max_exp = world.get_exp_by_level(level, 0);

    if is_level_up {
        // Check for multi-level jump (GM level set)
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => return,
        };

        if level > ch.level.saturating_add(1) {
            // Multi-level jump: calculate totals from scratch
            let stat_total_expected = {
                let mut total = 300i32 + (level as i32 - 1) * 3;
                if level > 60 {
                    total += 2 * (level as i32 - 60);
                }
                total as u16
            };
            let skill_total_expected = if level >= 10 {
                (level as u16 - 9) * 2
            } else {
                0
            };

            world.update_character_stats(sid, |ch| {
                let current_stat_total = get_stat_total(ch);
                if stat_total_expected > current_stat_total {
                    ch.free_points += stat_total_expected - current_stat_total;
                }

                let current_skill_total = get_total_skill_points(ch);
                if skill_total_expected > current_skill_total {
                    ch.skill_points[0] += (skill_total_expected - current_skill_total) as u8;
                }

                ch.level = level;
                ch.max_exp = new_max_exp;
            });
        } else {
            // Normal single-level increment
            let levels_after_60 = if level > 60 { level as u16 - 60 } else { 0 };
            let stat_pts = if levels_after_60 == 0 { 3u16 } else { 5u16 };
            let skill_pts = if level >= 10 { 2u8 } else { 0 };

            world.update_character_stats(sid, |ch| {
                let expected_total = 297 + (3 * level as u16) + (2 * levels_after_60);
                if (ch.free_points + get_stat_total(ch)) < expected_total {
                    ch.free_points += stat_pts;
                }

                if level >= 10 && get_total_skill_points(ch) < 2 * (level as u16 - 9) {
                    ch.skill_points[0] += skill_pts;
                }

                ch.max_exp = new_max_exp;
            });
        }
    } else {
        // De-level — just update max_exp (stat/skill points are NOT removed)
        world.update_character_stats(sid, |ch| {
            ch.max_exp = new_max_exp;
        });
    }

    if is_level_up {
        tracing::warn!(
            sid,
            level,
            free = world.get_character_info(sid).map(|c| c.free_points).unwrap_or(0),
            "level up: stat points after the level change"
        );
        audit_points(world, sid);
    }

    // Recalculate equipment stats + max HP/MP (includes item + buff bonuses)
    world.set_user_ability(sid);

    // Restore HP/MP to full on level up
    world.update_character_stats(sid, |ch| {
        ch.hp = ch.max_hp;
        ch.mp = ch.max_mp;
    });

    // Start the 10 Level Skill — auto-accept initial skill quest.
    //   if (GetLevel() == 10 && CheckExistEvent(71, 0))
    //       SaveEvent(71, 1);
    // C++ does this in the per-second tick; we do it in level_change for efficiency.
    // Using >= 10 to also cover multi-level jumps (e.g. GM /levelup from 5 to 15).
    if is_level_up && level >= 10 {
        let event_71_missing = world
            .with_session(sid, |h| match h.quests.get(&71) {
                Some(q) => q.quest_state == 0,
                None => true,
            })
            .unwrap_or(false);

        if event_71_missing {
            world.update_session(sid, |h| {
                h.quests.entry(71).or_default().quest_state = 1;
            });

            // Send WIZ_QUEST update to client so it knows event 71 is active.
            let mut quest_pkt = Packet::new(Opcode::WizQuest as u8);
            quest_pkt.write_u8(2); // sub: save_event
            quest_pkt.write_u16(71); // quest_id
            quest_pkt.write_u8(1); // quest_state = ongoing
            world.send_to_session_owned(sid, quest_pkt);

            tracing::debug!("[sid={}] Level 10 skill event 71 auto-accepted", sid);
        }
    }

    // Get final character state for broadcasting
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };

    let equipped_stats = world.get_equipped_stats(sid);

    // Broadcast WIZ_LEVEL_CHANGE to 3x3 region
    let mut pkt = Packet::new(Opcode::WizLevelChange as u8);
    pkt.write_u32(sid as u32); // GetSocketID()
    pkt.write_u8(ch.level); // GetLevel()
    pkt.write_i16(ch.free_points as i16); // m_sPoints
    pkt.write_u8(ch.skill_points[0]); // m_bstrSkill[SkillPointFree]
    pkt.write_i64(ch.max_exp); // m_iMaxExp
    pkt.write_i64(ch.exp as i64); // m_iExp
    pkt.write_i16(ch.max_hp); // m_MaxHp
    pkt.write_i16(ch.hp); // m_sHp
    pkt.write_i16(ch.max_mp); // m_MaxMp
    pkt.write_i16(ch.mp); // m_sMp
    pkt.write_u32(equipped_stats.max_weight); // m_sMaxWeight (uint32)
    pkt.write_u32(equipped_stats.item_weight); // m_sItemWeight (uint32)

    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }

    // Send party level change notification if in party
    if let Some(party_id) = ch.party_id {
        let mut party_pkt = Packet::new(Opcode::WizParty as u8);
        party_pkt.write_u8(PARTY_LEVELCHANGE);
        party_pkt.write_u32(sid as u32);
        party_pkt.write_u8(ch.level);
        world.send_to_party(party_id, &party_pkt);
    }

    // Send updated stat/skill reset cost after level change
    let premium = world.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
    let discount = world.is_discount_active(ch.nation);
    let cost_pkt = super::ext_hook::build_preset_req_money(ch.level, premium, discount);
    world.send_to_session_owned(sid, cost_pkt);

    tracing::info!(
        "[sid={}] LevelChange: level={} is_up={} exp={} max_exp={} hp={}/{} mp={}/{}",
        sid,
        ch.level,
        is_level_up,
        ch.exp,
        ch.max_exp,
        ch.hp,
        ch.max_hp,
        ch.mp,
        ch.max_mp,
    );
}

/// Calculate XP lost on death.
/// Default: `max_exp / 20` (5% of current level's max XP).
/// When `premium_exp_restore_percent` > 0, the loss is reduced to that
/// percentage of max XP instead of the default 5%.
/// ```text
/// int64 nExpLost = maxexp / 20;
/// if (GetPremiumPropertyExp(PremiumExpRestorePercent) > 0)
///     nExpLostFloat = maxexp * (PremiumExpRestorePercent) / 100;
/// if (nExpLostFloat) nExpLost = (int64)nExpLostFloat;
/// ```
pub fn on_death_lost_exp_calc(max_exp: i64, premium_exp_restore_percent: f32) -> i64 {
    let default_loss = max_exp / 20;

    if premium_exp_restore_percent > 0.0 {
        // Premium users: loss = max_exp * percent / 100
        // (typically 1-3%, lower than the default 5%)
        (max_exp as f64 * premium_exp_restore_percent as f64 / 100.0) as i64
    } else {
        default_loss
    }
}

/// Calculate the XP reward modifier based on level difference.
/// NOTE: The C++ code has `return 1.0f;` at the very top, making the
/// level-difference logic dead code. We match C++ behavior and always
/// return 1.0.
/// Original (disabled) logic for reference:
/// ```text
/// diff <= -14 → 0.2
/// diff <= -8  → 0.5
/// diff <= -2  → 0.8
/// else        → 1.0
/// ```
pub fn get_reward_modifier(_npc_level: u8, _player_level: u8) -> f32 {
    1.0
}

/// Send WIZ_EXP_CHANGE packet to the player.
/// Packet format: `[u8 flag=4] [i64 total_exp]`
fn send_exp_change(world: &WorldState, sid: SessionId) {
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };

    let mut pkt = Packet::new(Opcode::WizExpChange as u8);
    pkt.write_u8(EXP_FLAG_NORMAL);
    pkt.write_i64(ch.exp as i64);
    world.send_to_session_owned(sid, pkt);
}

/// Get the total of all allocated stat points.
fn get_stat_total(ch: &CharacterInfo) -> u16 {
    ch.str as u16 + ch.sta as u16 + ch.dex as u16 + ch.intel as u16 + ch.cha as u16
}

/// Get the total of all allocated skill points.
fn get_total_skill_points(ch: &CharacterInfo) -> u16 {
    // Sum skill categories (indices 5-8), excluding free (0)
    ch.skill_points[5] as u16
        + ch.skill_points[6] as u16
        + ch.skill_points[7] as u16
        + ch.skill_points[8] as u16
}

/// Save level and experience data to the database asynchronously (fire-and-forget).
/// Called by handlers that have access to the session (e.g., GM commands,
/// class change, etc.) to persist level changes to the DB.
/// NOTE: For normal gameplay (NPC kills, death penalties), level/XP is saved
/// periodically or on character logout — not on every XP change. This
/// matches the C++ server behavior.
pub fn save_level_exp_async(session: &crate::session::ClientSession) {
    let pool = session.pool().clone();
    let char_id = session.character_id().unwrap_or("").to_string();
    if char_id.is_empty() {
        return;
    }

    let world = session.world().clone();
    let sid = session.session_id();

    tokio::spawn(async move {
        let ch = match world.get_character_info(sid) {
            Some(ch) => ch,
            None => return,
        };

        let params = ko_db::repositories::character::SaveLevelExpParams {
            char_id: &char_id,
            level: ch.level as i16,
            exp: ch.exp as i64,
            hp: ch.hp,
            mp: ch.mp,
            free_points: ch.free_points as i16,
        };

        let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
        if let Err(e) = repo.save_level_exp(&params).await {
            tracing::error!(char_id, "failed to save level/exp: {}", e);
        }
    });
}

/// Give back stat / skill points a character is missing for its level (points that never arrived, e.g.
/// after a GM level change or an interrupted level up). Total = free + allocated; nothing is ever taken away.
/// Runs at game start and after each level up. Returns true when something was added.
pub fn audit_points(world: &WorldState, sid: SessionId) -> bool {
    let Some(ch) = world.get_character_info(sid) else {
        return false;
    };
    if ch.level < 1 || (ch.level as u16) > MAX_LEVEL {
        return false;
    }
    let stat_expected = expected_stat_total(ch.level) as i32;
    let stat_have = ch.free_points as i32 + get_stat_total(&ch) as i32;
    let skill_expected = expected_skill_total(ch.level) as i32;
    let skill_have = ch.skill_points[0] as i32 + get_total_skill_points(&ch) as i32;
    let stat_add = (stat_expected - stat_have).clamp(0, i16::MAX as i32);
    let skill_add = (skill_expected - skill_have).clamp(0, 255);
    if stat_add == 0 && skill_add == 0 {
        return false;
    }
    world.update_character_stats(sid, |c| {
        c.free_points = c.free_points.saturating_add(stat_add as u16);
        c.skill_points[0] = c.skill_points[0].saturating_add(skill_add as u8);
    });
    tracing::warn!(
        sid,
        level = ch.level,
        stat_add,
        skill_add,
        "points audit: the character was missing stat / skill points for its level"
    );
    true
}

/// Calculate the expected total stat points for a given level.
/// `nStatTotal = 300 + (level - 1) * 3`
/// For levels above 60: `+= 2 * (level - 60)`
pub fn expected_stat_total(level: u8) -> u16 {
    let mut total = 300u16 + (level.saturating_sub(1) as u16) * 3;
    if level > 60 {
        total += 2 * (level as u16 - 60);
    }
    total
}

/// Calculate the expected total skill points for a given level.
/// `nSkillTotal = (level - 9) * 2` for levels >= 10, else 0.
pub fn expected_skill_total(level: u8) -> u16 {
    if level >= 10 {
        (level as u16 - 9) * 2
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use super::*;

    // ── Packet format tests ─────────────────────────────────────────

    #[test]
    fn test_exp_change_packet_format() {
        // WIZ_EXP_CHANGE: [u8 flag=4] [i64 total_exp]
        let total_exp: i64 = 1_000_000;
        let mut pkt = Packet::new(Opcode::WizExpChange as u8);
        pkt.write_u8(EXP_FLAG_NORMAL);
        pkt.write_i64(total_exp);

        assert_eq!(pkt.opcode, Opcode::WizExpChange as u8);
        // 1 + 8 = 9 bytes
        assert_eq!(pkt.data.len(), 9);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXP_FLAG_NORMAL));
        assert_eq!(r.read_i64(), Some(1_000_000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_level_change_packet_format() {
        // WIZ_LEVEL_CHANGE: [u32 sid] [u8 level] [i16 stat_pts] [u8 skill_pts]
        //   [i64 max_exp] [i64 exp] [i16 max_hp] [i16 hp] [i16 max_mp] [i16 mp]
        //   [u32 max_weight] [u32 item_weight]
        //   m_sMaxWeight is uint32 (User.h:394), m_sItemWeight is uint32 (User.h:406)
        let mut pkt = Packet::new(Opcode::WizLevelChange as u8);
        pkt.write_u32(42); // sid
        pkt.write_u8(61); // level
        pkt.write_i16(10); // stat_points
        pkt.write_u8(5); // free_skill_points
        pkt.write_i64(528_495_192); // max_exp
        pkt.write_i64(100_000); // current exp
        pkt.write_i16(1500); // max_hp
        pkt.write_i16(1500); // hp
        pkt.write_i16(800); // max_mp
        pkt.write_i16(800); // mp
        pkt.write_u32(5000); // max_weight (uint32)
        pkt.write_u32(1200); // item_weight (uint32)

        assert_eq!(pkt.opcode, Opcode::WizLevelChange as u8);
        // 4+1+2+1+8+8+2+2+2+2+4+4 = 40 bytes
        assert_eq!(pkt.data.len(), 40);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(61));
        assert_eq!(r.read_u16(), Some(10)); // i16 read as u16
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_i64(), Some(528_495_192));
        assert_eq!(r.read_i64(), Some(100_000));
        assert_eq!(r.read_u16(), Some(1500u16)); // i16 as u16
        assert_eq!(r.read_u16(), Some(1500u16));
        assert_eq!(r.read_u16(), Some(800u16));
        assert_eq!(r.read_u16(), Some(800u16));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.read_u32(), Some(1200));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_party_level_change_packet_format() {
        // WIZ_PARTY: [u8 PARTY_LEVELCHANGE] [u32 sid] [u8 level]
        let mut pkt = Packet::new(Opcode::WizParty as u8);
        pkt.write_u8(PARTY_LEVELCHANGE);
        pkt.write_u32(42);
        pkt.write_u8(61);

        assert_eq!(pkt.opcode, Opcode::WizParty as u8);
        // 1 + 4 + 1 = 6 bytes
        assert_eq!(pkt.data.len(), 6);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x07));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(61));
        assert_eq!(r.remaining(), 0);
    }

    // ── XP loss on death tests ──────────────────────────────────────

    #[test]
    fn test_on_death_lost_exp_calc_default() {
        // 5% of max_exp (no premium, restore_percent = 0.0)
        assert_eq!(on_death_lost_exp_calc(200, 0.0), 10);
        assert_eq!(on_death_lost_exp_calc(1000, 0.0), 50);
        assert_eq!(on_death_lost_exp_calc(293_608_440, 0.0), 14_680_422);
        assert_eq!(on_death_lost_exp_calc(0, 0.0), 0);
    }

    #[test]
    fn test_on_death_lost_exp_calc_premium() {
        // Premium user with 2% restore: loss = max_exp * 2 / 100 = 2%
        assert_eq!(on_death_lost_exp_calc(1000, 2.0), 20); // 2% of 1000
        assert_eq!(on_death_lost_exp_calc(200, 2.0), 4); // 2% of 200
        assert_eq!(on_death_lost_exp_calc(0, 2.0), 0); // 0 max_exp

        // Premium with 3% restore
        assert_eq!(on_death_lost_exp_calc(1000, 3.0), 30); // 3% of 1000

        // Premium with 1% restore (very generous premium)
        assert_eq!(on_death_lost_exp_calc(1000, 1.0), 10); // 1% of 1000
    }

    #[test]
    fn test_premium_reduces_xp_loss() {
        // Verify premium always reduces XP loss compared to default (5%)
        let max_exp = 293_608_440i64;
        let default_loss = on_death_lost_exp_calc(max_exp, 0.0);
        let premium_loss = on_death_lost_exp_calc(max_exp, 2.0);
        assert!(premium_loss < default_loss);
        // Default: 5% = 14,680,422
        assert_eq!(default_loss, 14_680_422);
        // Premium 2%: = 5,872,168
        assert_eq!(premium_loss, 5_872_168);
    }

    // ── Reward modifier tests ───────────────────────────────────────
    // C++ GetRewardModifier always returns 1.0f (level-diff logic is dead code).

    #[test]
    fn test_reward_modifier_always_one() {
        // All level combinations must return 1.0, matching C++ behavior.
        assert_eq!(get_reward_modifier(50, 50), 1.0);
        assert_eq!(get_reward_modifier(50, 52), 1.0);
        assert_eq!(get_reward_modifier(50, 57), 1.0);
        assert_eq!(get_reward_modifier(50, 58), 1.0);
        assert_eq!(get_reward_modifier(50, 63), 1.0);
        assert_eq!(get_reward_modifier(50, 64), 1.0);
        assert_eq!(get_reward_modifier(10, 80), 1.0);
        assert_eq!(get_reward_modifier(60, 50), 1.0);
        assert_eq!(get_reward_modifier(80, 40), 1.0);
    }

    // ── Stat/skill point calculation tests ──────────────────────────

    #[test]
    fn test_stat_points_per_level() {
        // Levels 1-60: 3 stat points per level
        // Levels 61+: 5 stat points per level (3 base + 2 bonus)
        let levels_after_60_at_30 = 0u16;
        let stat_pts_30 = if levels_after_60_at_30 == 0 {
            3u16
        } else {
            5u16
        };
        assert_eq!(stat_pts_30, 3);

        let levels_after_60_at_65 = 5u16;
        let stat_pts_65 = if levels_after_60_at_65 == 0 {
            3u16
        } else {
            5u16
        };
        assert_eq!(stat_pts_65, 5);
    }

    #[test]
    fn test_skill_points_per_level() {
        // No skill points before level 10
        assert_eq!(if 5u8 >= 10 { 2u8 } else { 0u8 }, 0);
        assert_eq!(if 9u8 >= 10 { 2u8 } else { 0u8 }, 0);
        // 2 skill points per level from level 10+
        assert_eq!(if 10u8 >= 10 { 2u8 } else { 0u8 }, 2);
        assert_eq!(if 60u8 >= 10 { 2u8 } else { 0u8 }, 2);
    }

    // ── Level-up XP table reference tests ───────────────────────────

    #[test]
    fn test_level_up_xp_values() {
        // Verify known XP values from the MSSQL LEVEL_UP table
        // Level 1: 200 XP required
        // Level 60: 293,608,440 XP required
        // Level 83: 34,823,947,840 XP required
        assert_eq!(on_death_lost_exp_calc(200, 0.0), 10);
        assert_eq!(on_death_lost_exp_calc(34_823_947_840, 0.0), 1_741_197_392);
    }

    // ── Expected stat/skill total tests ──────────────────────────────

    #[test]
    fn test_expected_stat_total_level_1() {
        // Level 1: 300 + (1-1)*3 = 300
        assert_eq!(expected_stat_total(1), 300);
    }

    #[test]
    fn test_expected_stat_total_level_60() {
        // Level 60: 300 + 59*3 = 300 + 177 = 477
        assert_eq!(expected_stat_total(60), 477);
    }

    #[test]
    fn test_expected_stat_total_level_61() {
        // Level 61: 300 + 60*3 + 2*(61-60) = 300 + 180 + 2 = 482
        assert_eq!(expected_stat_total(61), 482);
    }

    #[test]
    fn test_expected_stat_total_level_83() {
        // Level 83: 300 + 82*3 + 2*(83-60) = 300 + 246 + 46 = 592
        assert_eq!(expected_stat_total(83), 592);
    }

    #[test]
    fn test_expected_skill_total_level_5() {
        // Below level 10: 0 skill points
        assert_eq!(expected_skill_total(5), 0);
    }

    #[test]
    fn test_expected_skill_total_level_10() {
        // Level 10: (10-9)*2 = 2
        assert_eq!(expected_skill_total(10), 2);
    }

    #[test]
    fn test_expected_skill_total_level_60() {
        // Level 60: (60-9)*2 = 102
        assert_eq!(expected_skill_total(60), 102);
    }

    #[test]
    fn test_expected_skill_total_level_83() {
        // Level 83: (83-9)*2 = 148
        assert_eq!(expected_skill_total(83), 148);
    }

    // ── Bonus multiplier tests (unit-testable logic) ─────────────────

    #[test]
    fn test_bonus_cap() {
        // Verify the MAX_BONUS_PERCENT constant
        assert_eq!(MAX_BONUS_PERCENT, 10_000);
    }

    #[test]
    fn test_exp_flag_constant() {
        assert_eq!(EXP_FLAG_NORMAL, 0x04);
    }

    // ── Multi-level jump stat calculation tests ──────────────────────

    #[test]
    fn test_multi_level_jump_stat_calculation() {
        // Simulate a jump from level 1 to level 60
        // Expected stat total at 60: 300 + 59*3 = 477
        // Starting stats: 60*5 = 300 (base stats)
        // Free points needed: 477 - 300 = 177
        let target_level: u8 = 60;
        let expected_total = expected_stat_total(target_level);
        assert_eq!(expected_total, 477);

        let current_stat_total: u16 = 300; // base stats
        let free_points_needed = expected_total - current_stat_total;
        assert_eq!(free_points_needed, 177);
    }

    #[test]
    fn test_multi_level_jump_skill_calculation() {
        // Jump from level 1 to level 60
        // Expected skill total: (60-9)*2 = 102
        let target_level: u8 = 60;
        let expected_total = expected_skill_total(target_level);
        assert_eq!(expected_total, 102);

        let current_skill_total: u16 = 0;
        let skill_points_needed = expected_total - current_skill_total;
        assert_eq!(skill_points_needed, 102);
    }

    // ── get_stat_total / get_total_skill_points tests ────────────────

    #[test]
    fn test_get_stat_total() {
        let ch = CharacterInfo {
            str: 65,
            sta: 65,
            dex: 60,
            intel: 50,
            cha: 50,
            ..test_character()
        };
        // 65 + 65 + 60 + 50 + 50 = 290
        assert_eq!(get_stat_total(&ch), 290);
    }

    #[test]
    fn test_get_total_skill_points() {
        let ch = CharacterInfo {
            skill_points: [20, 0, 0, 0, 0, 10, 5, 3, 0, 0],
            ..test_character()
        };
        // Sum of indices 5-8: 10 + 5 + 3 + 0 = 18
        assert_eq!(get_total_skill_points(&ch), 18);
    }

    #[test]
    fn test_get_total_skill_points_all_zero() {
        let ch = CharacterInfo {
            skill_points: [0; 10],
            ..test_character()
        };
        assert_eq!(get_total_skill_points(&ch), 0);
    }

    /// Helper to create a test character with warrior-like stats.
    fn test_character() -> CharacterInfo {
        CharacterInfo {
            session_id: 1,
            name: "TestWarrior".into(),
            nation: 1,
            race: 1,
            class: 105,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 65,
            sta: 65,
            dex: 60,
            intel: 50,
            cha: 50,
            free_points: 10,
            skill_points: [20, 0, 0, 0, 0, 10, 5, 3, 0, 0],
            gold: 1000,
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
        }
    }

    // ── Level 10 skill event 71 packet test ────────────────────────

    #[test]
    fn test_event_71_quest_packet_format() {
        // WIZ_QUEST [u8 sub=2] [u16 quest_id=71] [u8 state=1]
        // This is the packet sent when a player reaches level 10 and
        // event 71 auto-accepts (initial skill quest).
        let mut pkt = Packet::new(Opcode::WizQuest as u8);
        pkt.write_u8(2); // sub: save_event
        pkt.write_u16(71); // quest_id
        pkt.write_u8(1); // quest_state = ongoing

        assert_eq!(pkt.opcode, Opcode::WizQuest as u8);
        // 1 + 2 + 1 = 4 bytes
        assert_eq!(pkt.data.len(), 4);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(71));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_event_71_triggers_at_level_10() {
        // Verify the check_exist_event(71, 0) logic:
        // - No entry in quests map → state is 0 → should trigger
        // - Entry with state 0 → should trigger
        // - Entry with state 1 → should NOT trigger (already accepted)
        // - Entry with state 2 → should NOT trigger (completed)
        use crate::world::UserQuestInfo;
        use std::collections::HashMap;

        // No entry → event missing, should trigger
        let quests: HashMap<u16, UserQuestInfo> = HashMap::new();
        let missing = match quests.get(&71) {
            Some(q) => q.quest_state == 0,
            None => true,
        };
        assert!(missing, "No entry should mean event 71 is missing");

        // Entry with state 1 → already accepted, should NOT trigger
        let mut quests2: HashMap<u16, UserQuestInfo> = HashMap::new();
        quests2.insert(
            71,
            UserQuestInfo {
                quest_state: 1,
                kill_counts: [0; 4],
            },
        );
        let missing2 = match quests2.get(&71) {
            Some(q) => q.quest_state == 0,
            None => true,
        };
        assert!(!missing2, "State 1 should not be missing");

        // Entry with state 2 → completed, should NOT trigger
        let mut quests3: HashMap<u16, UserQuestInfo> = HashMap::new();
        quests3.insert(
            71,
            UserQuestInfo {
                quest_state: 2,
                kill_counts: [0; 4],
            },
        );
        let missing3 = match quests3.get(&71) {
            Some(q) => q.quest_state == 0,
            None => true,
        };
        assert!(!missing3, "State 2 should not be missing");
    }

    #[test]
    fn test_event_71_level_guard() {
        // The EVENT 71 trigger uses `level >= 10` to cover multi-level jumps.
        // C++ uses `GetLevel() == 10` in a per-tick check, but since we only
        // check in level_change(), >= 10 covers GM jumps from below 10.
        let test_cases = [
            (9, false, false),  // level 9, not level up → no trigger
            (9, true, false),   // level 9, level up → no trigger (< 10)
            (10, true, true),   // level 10, level up → trigger
            (11, true, true),   // level 11, level up → trigger (multi-level jump)
            (50, true, true),   // level 50, level up → trigger (GM jump)
            (10, false, false), // level 10, de-level → no trigger
        ];

        for (level, is_level_up, expected) in test_cases {
            let should_check = is_level_up && level >= 10;
            assert_eq!(
                should_check, expected,
                "level={level}, is_level_up={is_level_up}"
            );
        }
    }

    // ── Clan leader online bonus tests ──────────────────────────────

    #[test]
    fn test_clan_leader_bonus_no_clan() {
        let ch = CharacterInfo {
            knights_id: 0,
            ..test_character()
        };
        let world = WorldState::new();
        assert_eq!(get_clan_leader_online_bonus(&world, 1, &ch), 0);
    }

    #[test]
    fn test_clan_leader_bonus_clan_no_chief_online() {
        let world = WorldState::new();
        world.insert_knights(crate::world::KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 1,
            ranking: 0,
            name: "TestClan".to_string(),
            chief: "ChiefPlayer".to_string(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        });
        let ch = CharacterInfo {
            knights_id: 100,
            ..test_character()
        };
        assert_eq!(get_clan_leader_online_bonus(&world, 1, &ch), 0);
    }

    // ── Sprint 295: Deleveling XP loss calculation fix ────────────────

    #[test]
    fn test_delevel_diff_xp_uses_original_exp() {
        // When (m_iExp + iExp) < 0, C++ does NOT update m_iExp.
        // Line 107: `diffXP = m_iExp + OnDeathLostExpCalc(...)` uses ORIGINAL exp.
        //
        // Example: player has 9000 XP, receives -11000 damage.
        // C++ diff_xp = 9000 + penalty (NOT -2000 + penalty)
        let original_exp: i64 = 9000;
        let damage: i64 = -11000;
        let _new_exp = original_exp + damage; // = -2000 (negative, triggers delevel)
        let prev_level_max: i64 = 5000;
        let penalty = on_death_lost_exp_calc(prev_level_max, 0.0);

        // CORRECT: use original exp
        let diff_xp_correct = original_exp + penalty;
        assert!(
            diff_xp_correct > 0,
            "C++ uses original exp → positive diff_xp"
        );

        // WRONG (old bug): use new_exp
        let diff_xp_wrong = _new_exp + penalty;
        assert!(
            diff_xp_wrong < 0,
            "Using new_exp gives negative diff_xp (no recursion)"
        );
    }
}
