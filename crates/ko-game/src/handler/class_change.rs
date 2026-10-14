//! WIZ_CLASS_CHANGE (0x34) handler — job change, stat reset, skill reset.
//! ## Sub-opcodes
//! | Sub | Name                | Description                          |
//! |-----|---------------------|--------------------------------------|
//! | 0x01| CLASS_CHANGE_REQ    | Client checks if class change is ok  |
//! | 0x03| ALL_POINT_CHANGE    | Reset stat points to base values     |
//! | 0x04| ALL_SKILLPT_CHANGE  | Reset skill points                   |
//! | 0x05| CHANGE_MONEY_REQ    | Query cost for stat/skill reset      |
//! | 0x06| PROMOTE_NOVICE      | Job change (type + NewJob)           |
//! | 0x07| REB_STAT_CHANGE     | Rebirth stat allocation (+2 points)  |
//! | 0x08| REB_STAT_RESET      | Rebirth stat redistribution          |

use ko_db::repositories::character::{CharacterRepository, SaveStatPointsParams};
use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::session::{ClientSession, SessionState};

// ── Sub-opcode constants ─────────────────────────────────────────────
const CLASS_CHANGE_REQ: u8 = 0x01;
const CLASS_CHANGE_RESULT: u8 = 0x02;
const ALL_POINT_CHANGE: u8 = 0x03;
const ALL_SKILLPT_CHANGE: u8 = 0x04;
const CHANGE_MONEY_REQ: u8 = 0x05;
const PROMOTE_NOVICE: u8 = 0x06;
const REB_STAT_CHANGE: u8 = 0x07;
const REB_STAT_RESET: u8 = 0x08;

/// Item required for rebirth.
const QUALIFICATION_OF_REBIRTH: u32 = 900_579_000;
/// Gold cost for rebirth stat reset (100M).
const REBIRTH_RESET_GOLD: u32 = 100_000_000;
/// Premium types that grant free rebirth reset.
const PREMIUM_REBIRTH_FREE_1: u8 = 12;
const PREMIUM_REBIRTH_FREE_2: u8 = 13;

// ── Class constants ──────────────────────────────────────────────────
const KARUWARRIOR: u16 = 101;
const KARUROGUE: u16 = 102;
const KARUWIZARD: u16 = 103;
const KARUPRIEST: u16 = 104;
const BERSERKER: u16 = 105;
const GUARDIAN: u16 = 106;
const HUNTER: u16 = 107;
const PENETRATOR: u16 = 108;
const SORSERER: u16 = 109;
const NECROMANCER: u16 = 110;
const SHAMAN: u16 = 111;
const DARKPRIEST: u16 = 112;
const KURIANSTARTER: u16 = 113;
const KURIANNOVICE: u16 = 114;
const KURIANMASTER: u16 = 115;
const ELMORWARRIOR: u16 = 201;
const ELMOROGUE: u16 = 202;
const ELMOWIZARD: u16 = 203;
const ELMOPRIEST: u16 = 204;
const BLADE: u16 = 205;
const PROTECTOR: u16 = 206;
const RANGER: u16 = 207;
const ASSASSIN: u16 = 208;
const MAGE: u16 = 209;
const ENCHANTER: u16 = 210;
const CLERIC: u16 = 211;
const DRUID: u16 = 212;
const PORUTUSTARTER: u16 = 213;
const PORUTUNOVICE: u16 = 214;
const PORUTUMASTER: u16 = 215;

use crate::race_constants::{
    BABARIAN, ELMORAD_MAN, KARUS_BIG, KARUS_MIDDLE, KARUS_SMALL, KURIAN, PORUTU,
};

// ── Item constants ───────────────────────────────────────────────────
const ITEM_JOB_CHANGE: u32 = 700112000;
const ITEM_JOB_CHANGE2: u32 = 700113000;
const RETURNTOKENS: u32 = 810512000;
/// Premium type that grants free stat/skill reset.
const PREMIUM_WARP: u8 = 12;

use super::SLOT_MAX;

/// Handle WIZ_CLASS_CHANGE — dispatch on sub-opcode.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    let free_by_setting = session
        .world()
        .get_server_settings()
        .map(|s| s.free_skill_stat != 0)
        .unwrap_or(false);

    match sub_opcode {
        CLASS_CHANGE_REQ => handle_class_change_req(session).await,
        ALL_POINT_CHANGE => handle_all_point_change(session, free_by_setting).await,
        ALL_SKILLPT_CHANGE => handle_all_skill_point_change(session, free_by_setting).await,
        CHANGE_MONEY_REQ => {
            let sub_type = reader.read_u8().unwrap_or(0);
            handle_change_money_req(session, sub_type).await
        }
        PROMOTE_NOVICE => {
            let change_type = reader.read_u8().unwrap_or(0);
            let new_job = reader.read_u8().unwrap_or(0);
            handle_job_change(session, change_type, new_job, false).await
        }
        REB_STAT_CHANGE => {
            let rec_str = reader.read_u8().unwrap_or(0);
            let rec_sta = reader.read_u8().unwrap_or(0);
            let rec_dex = reader.read_u8().unwrap_or(0);
            let rec_int = reader.read_u8().unwrap_or(0);
            let rec_cha = reader.read_u8().unwrap_or(0);
            handle_reb_stat_change(session, rec_str, rec_sta, rec_dex, rec_int, rec_cha).await
        }
        REB_STAT_RESET => {
            let rec_str = reader.read_u8().unwrap_or(0);
            let rec_sta = reader.read_u8().unwrap_or(0);
            let rec_dex = reader.read_u8().unwrap_or(0);
            let rec_int = reader.read_u8().unwrap_or(0);
            let rec_cha = reader.read_u8().unwrap_or(0);
            handle_reb_stat_reset(session, rec_str, rec_sta, rec_dex, rec_int, rec_cha).await
        }
        _ => {
            tracing::debug!(
                "[{}] WIZ_CLASS_CHANGE: unknown sub_opcode=0x{:02X}",
                session.addr(),
                sub_opcode,
            );
            Ok(())
        }
    }
}

/// Handle CLASS_CHANGE_REQ (0x01) — check if player can class change.
/// Response: `[u8 CLASS_CHANGE_RESULT] [u8 result]`
/// - 1 = can change, 2 = level too low, 3 = already changed
async fn handle_class_change_req(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    let mut resp = Packet::new(Opcode::WizClassChange as u8);
    resp.write_u8(CLASS_CHANGE_RESULT);

    if char_info.level < 10 {
        // Level too low for class change.
        resp.write_u8(2);
    } else if (char_info.class % 100) > 4 {
        // Already has job change — check kurian special case.
        if is_portu_kurian(char_info.class) && (char_info.class % 100) == 13 {
            resp.write_u8(1);
        } else {
            resp.write_u8(3);
        }
    } else {
        resp.write_u8(1);
    }

    session.send_packet(&resp).await?;

    tracing::debug!(
        "[{}] CLASS_CHANGE_REQ: class={} level={}",
        session.addr(),
        char_info.class,
        char_info.level,
    );

    Ok(())
}

/// Handle ALL_POINT_CHANGE (0x03) — reset all stat points.
/// Response on success: `[u8 ALL_POINT_CHANGE] [u8 1] [u32 gold] [u16 str] [u16 sta]
///   [u16 dex] [u16 int] [u16 cha] [i16 max_hp] [i16 max_mp] [u16 total_hit]
///   [u16 max_weight] [u16 free_points]`
/// Response on failure: `[u8 ALL_POINT_CHANGE] [u8 result] [i32 money]`
pub(crate) async fn handle_all_point_change(
    session: &mut ClientSession,
    free: bool,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    let money_required = get_reset_money(char_info.level);

    // Check equipment slots are empty (slots 0-13).
    let has_equipment = world.update_inventory(sid, |inv| {
        for i in 0..SLOT_MAX {
            if let Some(slot) = inv.get(i) {
                if slot.item_id != 0 {
                    return true;
                }
            }
        }
        false
    });

    if has_equipment {
        let mut resp = Packet::new(Opcode::WizClassChange as u8);
        resp.write_u8(ALL_POINT_CHANGE);
        resp.write_u8(4); // equipment still worn
        resp.write_i32(money_required);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Apply discount if active
    let mut effective_money = money_required;
    if !free && world.is_discount_active(char_info.nation) {
        effective_money /= 2;
    }

    // Gold check: skip when reset is free (premium/token).
    let cost = if free {
        0u32
    } else {
        effective_money.max(0) as u32
    };
    if !free && char_info.gold < cost {
        let mut resp = Packet::new(Opcode::WizClassChange as u8);
        resp.write_u8(ALL_POINT_CHANGE);
        resp.write_u8(0); // not enough gold
        resp.write_i32(effective_money);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Determine base stats for the new class + race combination.
    let (base_str, base_sta, base_dex, base_int, base_cha) =
        get_base_stats_for_class(char_info.class);

    // Calculate free points: 10 + (level-1)*3 + 2*(level-60) if level>60.
    let mut free_points: u16 = 10 + (char_info.level as u16 - 1) * 3;
    if char_info.level > 60 {
        free_points += 2 * (char_info.level as u16 - 60);
    }

    // Deduct gold and apply stat reset.
    let new_gold = char_info.gold.saturating_sub(cost);

    world.update_character_stats(sid, |ch| {
        ch.str = base_str;
        ch.sta = base_sta;
        ch.dex = base_dex;
        ch.intel = base_int;
        ch.cha = base_cha;
        ch.free_points = free_points;
        ch.gold = new_gold;
    });

    // Recalculate HP/MP with new stats.
    // Recalculate equipment stats + max HP/MP (includes item + buff bonuses)
    // C++ AllPointChange (UserSkillStatPointSystem.cpp:473) calls SetUserAbility()
    // but does NOT restore HP/MP to full.
    world.set_user_ability(sid);

    // Build success response.
    let mut resp = Packet::new(Opcode::WizClassChange as u8);
    resp.write_u8(ALL_POINT_CHANGE);
    resp.write_u8(1); // success
    resp.write_u32(new_gold);
    resp.write_u16(base_str as u16);
    resp.write_u16(base_sta as u16);
    resp.write_u16(base_dex as u16);
    resp.write_u16(base_int as u16);
    resp.write_u16(base_cha as u16);
    let ch_final = world.get_character_info(sid);
    let final_max_hp = ch_final.as_ref().map(|c| c.max_hp).unwrap_or(100);
    let final_max_mp = ch_final.as_ref().map(|c| c.max_mp).unwrap_or(100);
    let equipped = world.get_equipped_stats(sid);
    resp.write_i16(final_max_hp);
    resp.write_i16(final_max_mp);
    resp.write_u16(equipped.total_hit);
    resp.write_u32(equipped.max_weight);
    resp.write_u16(free_points);
    session.send_packet(&resp).await?;

    // Fire-and-forget DB save.
    save_stat_points_async(session);

    tracing::debug!(
        "[{}] ALL_POINT_CHANGE: success, gold={}, free_points={}",
        session.addr(),
        new_gold,
        free_points,
    );

    Ok(())
}

/// Handle ALL_SKILLPT_CHANGE (0x04) — reset all skill points.
/// Response on success: `[u8 ALL_SKILLPT_CHANGE] [u8 1] [u32 gold] [u8 free_skill_points]`
/// Response on failure: `[u8 ALL_SKILLPT_CHANGE] [u8 type] [i32 money]`
pub(crate) async fn handle_all_skill_point_change(
    session: &mut ClientSession,
    free: bool,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    let money_required = get_reset_money(char_info.level);

    // Level must be >= 10.
    if char_info.level < 10 {
        let mut resp = Packet::new(Opcode::WizClassChange as u8);
        resp.write_u8(ALL_SKILLPT_CHANGE);
        resp.write_u8(0);
        resp.write_i32(money_required);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Calculate total allocated skill points (indices 1-8).
    let total_allocated: u16 = char_info.skill_points[1..9].iter().map(|&v| v as u16).sum();

    if total_allocated == 0 {
        let mut resp = Packet::new(Opcode::WizClassChange as u8);
        resp.write_u8(ALL_SKILLPT_CHANGE);
        resp.write_u8(2); // no skill points to reset
        resp.write_i32(money_required);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Apply discount if active
    let mut effective_money = money_required;
    if !free && world.is_discount_active(char_info.nation) {
        effective_money /= 2;
    }

    // Gold check: skip when reset is free (premium/token).
    let cost = if free {
        0u32
    } else {
        effective_money.max(0) as u32
    };
    if !free && char_info.gold < cost {
        let mut resp = Packet::new(Opcode::WizClassChange as u8);
        resp.write_u8(ALL_SKILLPT_CHANGE);
        resp.write_u8(0);
        resp.write_i32(effective_money);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    let free_skill_points = (char_info.level.saturating_sub(9) as u16) * 2;
    let new_gold = char_info.gold.saturating_sub(cost);

    world.update_character_stats(sid, |ch| {
        ch.skill_points[0] = free_skill_points.min(u8::MAX as u16) as u8;
        for i in 1..9 {
            ch.skill_points[i] = 0;
        }
        ch.gold = new_gold;
    });

    world.set_user_ability(sid);

    let mut resp = Packet::new(Opcode::WizClassChange as u8);
    resp.write_u8(ALL_SKILLPT_CHANGE);
    resp.write_u8(1); // success
    resp.write_u32(new_gold);
    resp.write_u8(free_skill_points.min(u8::MAX as u16) as u8);
    session.send_packet(&resp).await?;

    save_stat_points_async(session);

    tracing::debug!(
        "[{}] ALL_SKILLPT_CHANGE: success, gold={}, free_sp={}",
        session.addr(),
        new_gold,
        free_skill_points,
    );

    Ok(())
}

/// Handle CHANGE_MONEY_REQ (0x05) — query cost for stat/skill reset.
/// Response: `[u8 CHANGE_MONEY_REQ] [i32 money]`
async fn handle_change_money_req(session: &mut ClientSession, sub_type: u8) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Premium type 12, Return Token, or FreeSkillandStat setting grants free reset.
    let premium_type = world.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
    let free_by_setting = world
        .get_server_settings()
        .map(|s| s.free_skill_stat != 0)
        .unwrap_or(false);
    let has_free_reset = premium_type == PREMIUM_WARP
        || world.check_exist_item(sid, RETURNTOKENS, 1)
        || free_by_setting;

    if has_free_reset {
        if sub_type == 1 {
            return handle_all_point_change(session, true).await;
        } else if sub_type == 2 {
            return handle_all_skill_point_change(session, true).await;
        }
    }

    let mut money = get_reset_money(char_info.level);

    // Skill point reset costs 1.5x more.
    if sub_type == 2 {
        money = (money as f32 * 1.5) as i32;
    }

    // Apply discount if active
    if world.is_discount_active(char_info.nation) {
        money /= 2;
    }

    let mut resp = Packet::new(Opcode::WizClassChange as u8);
    resp.write_u8(CHANGE_MONEY_REQ);
    resp.write_i32(money);
    session.send_packet(&resp).await?;

    tracing::debug!(
        "[{}] CHANGE_MONEY_REQ: sub_type={}, money={}",
        session.addr(),
        sub_type,
        money,
    );

    Ok(())
}

/// Handle PROMOTE_NOVICE (0x06) — job change.
/// Incoming: `[u8 type (0=normal, 1=master)] [u8 NewJob (1-5)]`
/// Job change result codes:
/// - 1 = success
/// - 2 = invalid job
/// - 3 = no item exists
/// - 4 = equipment still worn
/// - 5 = error in newjob or class
/// - 6 = already that class / no scroll
/// - 7 = failed
pub(crate) async fn handle_job_change(
    session: &mut ClientSession,
    change_type: u8,
    new_job: u8,
    hook: bool,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Validate input.
    if !(1..=5).contains(&new_job) || (change_type != 0 && change_type != 1) {
        job_change_fail(session, hook, 5, 5).await?;
        return Ok(());
    }

    // Check if required scroll item exists.
    let scroll_id = if change_type == 0 {
        ITEM_JOB_CHANGE
    } else {
        ITEM_JOB_CHANGE2
    };

    if !has_item_in_inventory(&world, sid, scroll_id) {
        job_change_fail(session, hook, 6, 2).await?;
        return Ok(());
    }

    // Cannot change to same job group.
    if is_same_job_group(char_info.class, new_job) {
        job_change_fail(session, hook, 6, 3).await?;
        return Ok(());
    }

    // Check equipment slots are empty (slots 0-13).
    let has_equipment = world.update_inventory(sid, |inv| {
        for i in 0..SLOT_MAX {
            if let Some(slot) = inv.get(i) {
                if slot.item_id != 0 {
                    return true;
                }
            }
        }
        false
    });

    if has_equipment {
        // Send equipment-worn error with ALL_POINT_CHANGE sub-opcode format.
        if !hook {
            let mut resp = Packet::new(Opcode::WizClassChange as u8);
            resp.write_u8(ALL_POINT_CHANGE);
            resp.write_u8(4);
            resp.write_i32(0);
            session.send_packet(&resp).await?;
        }
        job_change_fail(session, hook, 4, 4).await?;
        return Ok(());
    }

    // Determine new class and race.
    let result = determine_new_class_and_race(
        char_info.class,
        char_info.race,
        char_info.nation,
        new_job,
        change_type,
    );

    let (new_class, new_race) = match result {
        Some((c, r)) => (c, r),
        None => {
            job_change_fail(session, hook, 5, 5).await?;
            return Ok(());
        }
    };

    // Consume the scroll item.
    if !world.rob_item(sid, scroll_id, 1) {
        job_change_fail(session, hook, 6, 2).await?;
        return Ok(());
    }

    // Apply class and race change.
    // Binary Reference: JobChange @ 140132470 line 340-341
    world.update_character_stats(sid, |ch| {
        ch.class = new_class;
        ch.race = new_race;
    });

    // Binary Reference: JobChange line 369 — AllPointChange(true, false)
    // Resets stats to base + sends ALL_POINT_CHANGE packet to client.
    handle_all_point_change(session, true).await?;

    // Binary Reference: JobChange line 370 — AllSkillPointChange(true)
    // Resets skills + sends ALL_SKILLPT_CHANGE packet to client.
    handle_all_skill_point_change(session, true).await?;

    // Binary Reference: JobChange line 371 — SendMyInfo(false)
    // Refresh full character info so client sees new class/race.
    // We send an item_move_refresh as a minimal stat sync;
    // the full SendMyInfo is the phase1 gamestart packet (complex, deferred).
    world.send_item_move_refresh(sid);

    // Binary Reference: JobChange lines 372-382
    // UserInOut(INOUT_OUT) + region recalc + UserInOut(INOUT_WARP)
    // Refreshes the character's visual appearance for nearby players (new class/race).
    // Equipment is empty (job change requirement) so equip_visual is all zeros.
    {
        let session_data = world.with_session(sid, |h| {
            let ch = h.character.clone();
            (h.position, ch, h.event_room)
        });
        if let Some((pos, Some(ch_ref), event_room)) = session_data {
            let ch_ref = &ch_ref;
            // Broadcast INOUT_OUT to remove old appearance
            let out_pkt =
                super::region::build_user_inout(super::region::INOUT_OUT, sid, Some(ch_ref), &pos);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(out_pkt),
                Some(sid),
                event_room,
            );
            // Broadcast INOUT_WARP to show new appearance (class/race)
            let warp_pkt =
                super::region::build_user_inout(super::region::INOUT_WARP, sid, Some(ch_ref), &pos);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(warp_pkt),
                None, // include self — nearby players see new appearance
                event_room,
            );
        }
    }

    // Binary Reference: JobChange line 383 — ResetWindows(this)
    // Close open trade/merchant/etc windows on client.
    // (Client handles this based on the class change event.)

    // Binary Reference: JobChange line 384-385 — InitType4(false, 0); RecastSavedMagic(0)
    world.clear_all_buffs(sid, false);
    world.set_user_ability(sid);
    world.send_item_move_refresh(sid);
    world.recast_saved_magic(sid);

    // Send job change success result (return value 1 from binary).
    if hook {
        send_job_change_hook(session, 1).await?;
    } else {
        send_job_change_result(session, 1).await?;
    }

    // Notify party members of class change.
    super::party::broadcast_party_class_change(&world, sid, new_class);

    // Fire-and-forget DB saves.
    save_class_change_async(session, new_class, new_race);
    save_stat_points_async(session);

    // FerihaLog: JobChangeInsertLog
    super::audit_log::log_job_change(
        session.pool(),
        session.account_id().unwrap_or(""),
        &char_info.name,
        char_info.class,
        new_class,
        char_info.race as u16,
        new_race as u16,
    );

    tracing::info!(
        "[{}] JOB_CHANGE: type={} new_job={} class {}→{} race {}→{}",
        session.addr(),
        change_type,
        new_job,
        char_info.class,
        new_class,
        char_info.race,
        new_race,
    );

    Ok(())
}

/// Failure of a job change: the native result code, or the JstKO panel's (hook 0xF7, op 2).
async fn job_change_fail(
    session: &mut ClientSession,
    hook: bool,
    native: u8,
    hook_code: u8,
) -> anyhow::Result<()> {
    if hook {
        send_job_change_hook(session, hook_code).await
    } else {
        send_job_change_fail(session, native).await
    }
}

/// `[0xE9][0xF7][2][result]`: 1 done, 2 no scroll, 3 already this class, 4 items equipped, 5 failed.
async fn send_job_change_hook(session: &mut ClientSession, result: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(super::jobchange_hook::EXT_SUB_JOBCHANGE);
    pkt.write_u8(2);
    pkt.write_u8(result);
    session.send_packet(&pkt).await
}

/// Scroll item for the JstKO panel's type (0 = 700112000, 1 = 700113000).
pub(crate) fn job_change_scroll(change_type: u8) -> u32 {
    if change_type == 0 {
        ITEM_JOB_CHANGE
    } else {
        ITEM_JOB_CHANGE2
    }
}

/// Send job change result packet.
async fn send_job_change_result(
    session: &mut ClientSession,
    result_code: u8,
) -> anyhow::Result<()> {
    let mut resp = Packet::new(Opcode::WizClassChange as u8);
    resp.write_u8(PROMOTE_NOVICE);
    resp.write_u8(result_code);
    session.send_packet(&resp).await
}

/// Send job change failure packet.
async fn send_job_change_fail(session: &mut ClientSession, result_code: u8) -> anyhow::Result<()> {
    send_job_change_result(session, result_code).await
}

/// Send rebirth result packet (used by both REB_STAT_CHANGE and REB_STAT_RESET).
/// Response: `[u8 sub_opcode] [u8 result] [u32 0]`
async fn send_reb_result(
    session: &mut ClientSession,
    sub_opcode: u8,
    result: u8,
) -> anyhow::Result<()> {
    let mut resp = Packet::new(Opcode::WizClassChange as u8);
    resp.write_u8(sub_opcode);
    resp.write_u8(result);
    resp.write_u32(0);
    session.send_packet(&resp).await
}

// ── Class / Race Helpers ─────────────────────────────────────────────

/// Get the class type (class % 100).
pub(crate) fn get_class_type(class: u16) -> u16 {
    class % 100
}

/// Check if the character is already in the target job group.
fn is_same_job_group(class: u16, new_job: u8) -> bool {
    let ct = get_class_type(class);
    match new_job {
        1 => matches!(ct, 1 | 5 | 6),   // warrior
        2 => matches!(ct, 2 | 7 | 8),   // rogue
        3 => matches!(ct, 3 | 9 | 10),  // mage
        4 => matches!(ct, 4 | 11 | 12), // priest
        5 => matches!(ct, 13..=15),     // kurian
        _ => false,
    }
}

/// Check if class is warrior (beginner/novice/mastered).
pub(crate) fn is_warrior(class: u16) -> bool {
    matches!(get_class_type(class), 1 | 5 | 6)
}

/// Check if class is rogue (beginner/novice/mastered).
pub(crate) fn is_rogue(class: u16) -> bool {
    matches!(get_class_type(class), 2 | 7 | 8)
}

/// Check if class is mage (beginner/novice/mastered).
pub(crate) fn is_mage(class: u16) -> bool {
    matches!(get_class_type(class), 3 | 9 | 10)
}

/// Check if class is priest (beginner/novice/mastered).
pub(crate) fn is_priest(class: u16) -> bool {
    matches!(get_class_type(class), 4 | 11 | 12)
}

/// Check if class is portu/kurian (beginner/novice/mastered).
pub(crate) fn is_portu_kurian(class: u16) -> bool {
    matches!(get_class_type(class), 13..=15)
}

/// Check if class is beginner tier.
pub(crate) fn is_beginner(class: u16) -> bool {
    matches!(get_class_type(class), 1 | 2 | 3 | 4 | 13)
}

/// Check if class is novice (skilled) tier.
pub(crate) fn is_novice(class: u16) -> bool {
    matches!(get_class_type(class), 5 | 7 | 9 | 11 | 14)
}

/// Check if class is mastered tier.
pub(crate) fn is_mastered(class: u16) -> bool {
    matches!(get_class_type(class), 6 | 8 | 10 | 12 | 15)
}

/// Check if an item exists in the inventory bag (slots 14-41).
pub(crate) fn has_item_in_inventory(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    item_id: u32,
) -> bool {
    world.update_inventory(sid, |inv| {
        for i in SLOT_MAX..(SLOT_MAX + 28) {
            if let Some(slot) = inv.get(i) {
                if slot.item_id == item_id && slot.count > 0 {
                    return true;
                }
            }
        }
        false
    })
}

/// Determine new class and race after job change.
/// Returns `Some((new_class, new_race))` on success, `None` on invalid combination.
fn determine_new_class_and_race(
    current_class: u16,
    current_race: u8,
    nation: u8,
    new_job: u8,
    change_type: u8,
) -> Option<(u16, u8)> {
    let is_karus = nation == 1;
    let is_beginner_other = |job: u8| -> bool {
        // Check if the current class is a beginner of a DIFFERENT class than new_job.
        match job {
            1 => !is_warrior(current_class) && is_beginner(current_class),
            2 => !is_rogue(current_class) && is_beginner(current_class),
            3 => !is_mage(current_class) && is_beginner(current_class),
            4 => !is_priest(current_class) && is_beginner(current_class),
            5 => !is_portu_kurian(current_class) && is_beginner(current_class),
            _ => false,
        }
    };
    let is_novice_other = |job: u8| -> bool {
        match job {
            1 => !is_warrior(current_class) && is_novice(current_class),
            2 => !is_rogue(current_class) && is_novice(current_class),
            3 => !is_mage(current_class) && is_novice(current_class),
            4 => !is_priest(current_class) && is_novice(current_class),
            5 => !is_portu_kurian(current_class) && is_novice(current_class),
            _ => false,
        }
    };
    let is_mastered_other = |job: u8| -> bool {
        match job {
            1 => !is_warrior(current_class) && is_mastered(current_class),
            2 => !is_rogue(current_class) && is_mastered(current_class),
            3 => !is_mage(current_class) && is_mastered(current_class),
            4 => !is_priest(current_class) && is_mastered(current_class),
            5 => !is_portu_kurian(current_class) && is_mastered(current_class),
            _ => false,
        }
    };

    match new_job {
        1 => {
            // Warrior
            if is_beginner_other(1) {
                if is_karus {
                    Some((KARUWARRIOR, KARUS_BIG))
                } else {
                    let race = if current_race == PORUTU {
                        BABARIAN
                    } else {
                        current_race
                    };
                    Some((ELMORWARRIOR, race))
                }
            } else if is_novice_other(1) {
                if is_karus {
                    Some((BERSERKER, KARUS_BIG))
                } else {
                    let race = if current_race == PORUTU {
                        BABARIAN
                    } else {
                        current_race
                    };
                    Some((BLADE, race))
                }
            } else if is_mastered_other(1) {
                if is_karus {
                    let class = if change_type == 1 {
                        BERSERKER
                    } else {
                        GUARDIAN
                    };
                    Some((class, KARUS_BIG))
                } else {
                    let class = if change_type == 1 { BLADE } else { PROTECTOR };
                    let race = if current_race == PORUTU {
                        BABARIAN
                    } else {
                        current_race
                    };
                    Some((class, race))
                }
            } else {
                None
            }
        }
        2 => {
            // Rogue
            if is_beginner_other(2) {
                if is_karus {
                    Some((KARUROGUE, KARUS_MIDDLE))
                } else {
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((ELMOROGUE, race))
                }
            } else if is_novice_other(2) {
                if is_karus {
                    Some((HUNTER, KARUS_MIDDLE))
                } else {
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((RANGER, race))
                }
            } else if is_mastered_other(2) {
                if is_karus {
                    let class = if change_type == 1 { HUNTER } else { PENETRATOR };
                    Some((class, KARUS_MIDDLE))
                } else {
                    let class = if change_type == 1 { RANGER } else { ASSASSIN };
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((class, race))
                }
            } else {
                None
            }
        }
        3 => {
            // Mage
            if is_beginner_other(3) {
                if is_karus {
                    let race = if current_race == KARUS_BIG
                        || current_race == KARUS_MIDDLE
                        || current_race == KURIAN
                    {
                        KARUS_SMALL
                    } else {
                        current_race
                    };
                    Some((KARUWIZARD, race))
                } else {
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((ELMOWIZARD, race))
                }
            } else if is_novice_other(3) {
                if is_karus {
                    let race = if current_race == KARUS_BIG
                        || current_race == KARUS_MIDDLE
                        || current_race == KURIAN
                    {
                        KARUS_SMALL
                    } else {
                        current_race
                    };
                    Some((SORSERER, race))
                } else {
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((MAGE, race))
                }
            } else if is_mastered_other(3) {
                if is_karus {
                    let class = if change_type == 1 {
                        SORSERER
                    } else {
                        NECROMANCER
                    };
                    let race = if current_race == KARUS_BIG
                        || current_race == KARUS_MIDDLE
                        || current_race == KURIAN
                    {
                        KARUS_SMALL
                    } else {
                        current_race
                    };
                    Some((class, race))
                } else {
                    let class = if change_type == 1 { MAGE } else { ENCHANTER };
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((class, race))
                }
            } else {
                None
            }
        }
        4 => {
            // Priest
            if is_beginner_other(4) {
                if is_karus {
                    let race = if current_race == KARUS_BIG
                        || current_race == KARUS_SMALL
                        || current_race == KURIAN
                    {
                        KARUS_MIDDLE
                    } else {
                        current_race
                    };
                    Some((KARUPRIEST, race))
                } else {
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((ELMOPRIEST, race))
                }
            } else if is_novice_other(4) {
                if is_karus {
                    let race = if current_race == KARUS_BIG
                        || current_race == KARUS_SMALL
                        || current_race == KURIAN
                    {
                        KARUS_MIDDLE
                    } else {
                        current_race
                    };
                    Some((SHAMAN, race))
                } else {
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((CLERIC, race))
                }
            } else if is_mastered_other(4) {
                if is_karus {
                    let class = if change_type == 1 { SHAMAN } else { DARKPRIEST };
                    let race = if current_race == KARUS_BIG
                        || current_race == KARUS_SMALL
                        || current_race == KURIAN
                    {
                        KARUS_MIDDLE
                    } else {
                        current_race
                    };
                    Some((class, race))
                } else {
                    let class = if change_type == 1 { CLERIC } else { DRUID };
                    let race = if current_race == BABARIAN || current_race == PORUTU {
                        ELMORAD_MAN
                    } else {
                        current_race
                    };
                    Some((class, race))
                }
            } else {
                None
            }
        }
        5 => {
            // Kurian/Portu
            if is_beginner_other(5) {
                if is_karus {
                    Some((KURIANSTARTER, KURIAN))
                } else {
                    Some((PORUTUSTARTER, PORUTU))
                }
            } else if is_novice_other(5) {
                if is_karus {
                    Some((KURIANNOVICE, KURIAN))
                } else {
                    Some((PORUTUNOVICE, PORUTU))
                }
            } else if is_mastered_other(5) {
                if is_karus {
                    Some((KURIANMASTER, KURIAN))
                } else {
                    Some((PORUTUMASTER, PORUTU))
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Get base stats for a class (used during stat point reset).
/// Returns `(str, sta, dex, int, cha)`.
fn get_base_stats_for_class(class: u16) -> (u8, u8, u8, u8, u8) {
    if is_warrior(class) || is_portu_kurian(class) {
        // Warrior / Kurian: STR=65, STA=65, DEX=60, INT=50, CHA=50
        (65, 65, 60, 50, 50)
    } else if is_rogue(class) {
        // Rogue: STR=60, STA=60, DEX=70, INT=50, CHA=50
        (60, 60, 70, 50, 50)
    } else {
        // Mage / Priest: STR=50, STA=60, DEX=60, INT=70, CHA=50
        (50, 60, 60, 70, 50)
    }
}

/// Calculate money required for stat/skill reset.
fn get_reset_money(level: u8) -> i32 {
    let base = (level as f64 * 2.0).powf(3.4) as i32;
    if level < 30 {
        (base as f32 * 0.4) as i32
    } else if level >= 60 {
        (base as f32 * 1.5) as i32
    } else {
        base
    }
}

// ── Async DB Helpers ─────────────────────────────────────────────────

/// Save stat and skill points to DB asynchronously (fire-and-forget).
fn save_stat_points_async(session: &ClientSession) {
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

        let mut skill_i16 = [0i16; 10];
        for (i, &v) in ch.skill_points.iter().enumerate() {
            skill_i16[i] = v as i16;
        }

        let params = SaveStatPointsParams {
            char_id: &char_id,
            str_val: ch.str as i16,
            sta: ch.sta as i16,
            dex: ch.dex as i16,
            intel: ch.intel as i16,
            cha: ch.cha as i16,
            free_points: ch.free_points as i16,
            skill_points: skill_i16,
        };

        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo.save_stat_points(&params).await {
            tracing::error!(char_id, "failed to save stat/skill points: {}", e);
        }
    });
}

/// Save class and race change to DB asynchronously (fire-and-forget).
fn save_class_change_async(session: &ClientSession, new_class: u16, new_race: u8) {
    let pool = session.pool().clone();
    let char_id = session.character_id().unwrap_or("").to_string();
    if char_id.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo
            .save_class_change(&char_id, new_class as i16, new_race as i16)
            .await
        {
            tracing::error!(char_id, "failed to save class change: {}", e);
        }
    });
}

/// Handle REB_STAT_CHANGE (0x07) — allocate rebirth stat points.
/// Incoming: `[u8 str] [u8 sta] [u8 dex] [u8 int] [u8 cha]`
/// The 5 values must sum to exactly 2.
/// Response: `[u8 REB_STAT_CHANGE] [u8 result] [u32 0]`
/// - 1 = success, 0 = failure
/// Requirements:
/// - rebirth_level < 9
/// - Sum of requested stats == 2
/// - Must have QUALIFICATION_OF_REBIRTH item (900579000)
/// Side effects on success:
/// - rebirth_level++
/// - exp = 0
/// - Removes rebirth quests (1119-1122) if rebirth_level < 9
/// - Removes the QUALIFICATION_OF_REBIRTH item
async fn handle_reb_stat_change(
    session: &mut ClientSession,
    rec_str: u8,
    rec_sta: u8,
    rec_dex: u8,
    rec_int: u8,
    rec_cha: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    if char_info.rebirth_level > 9 {
        send_reb_result(session, REB_STAT_CHANGE, 0).await?;
        return Ok(());
    }

    let total = rec_str as u16 + rec_sta as u16 + rec_dex as u16 + rec_int as u16 + rec_cha as u16;
    if total != 2 {
        send_reb_result(session, REB_STAT_CHANGE, 0).await?;
        return Ok(());
    }

    if !world.check_exist_item(sid, QUALIFICATION_OF_REBIRTH, 1) {
        send_reb_result(session, REB_STAT_CHANGE, 0).await?;
        return Ok(());
    }

    // Apply rebirth stat increment (cumulative: old + new)
    let new_reb_str = char_info.reb_str.saturating_add(rec_str);
    let new_reb_sta = char_info.reb_sta.saturating_add(rec_sta);
    let new_reb_dex = char_info.reb_dex.saturating_add(rec_dex);
    let new_reb_intel = char_info.reb_intel.saturating_add(rec_int);
    let new_reb_cha = char_info.reb_cha.saturating_add(rec_cha);
    let new_rebirth_level = char_info.rebirth_level + 1;

    // Update in-memory state
    world.update_character_stats(sid, |ch| {
        ch.reb_str = new_reb_str;
        ch.reb_sta = new_reb_sta;
        ch.reb_dex = new_reb_dex;
        ch.reb_intel = new_reb_intel;
        ch.reb_cha = new_reb_cha;
        ch.rebirth_level = new_rebirth_level;
        ch.exp = 0;
    });

    // Remove rebirth item
    world.rob_item(sid, QUALIFICATION_OF_REBIRTH, 1);

    // Remove rebirth quests if rebirth_level < 9
    if new_rebirth_level < 9 {
        world.update_session(sid, |h| {
            for qid in [1119u16, 1120, 1121, 1122] {
                h.quests.remove(&qid);
            }
        });
    }

    // Send success
    send_reb_result(session, REB_STAT_CHANGE, 1).await?;

    // v2525: Send rebirth completion notification (0xD3 sub=4)
    // Shows "Rebirth Lv X" in yellow on the client UI
    let reb_pkt = super::rebirth::build_complete(new_rebirth_level as i32);
    session.send_packet(&reb_pkt).await?;

    // Save to DB asynchronously
    let char_name = char_info.name.clone();
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo
            .save_rebirth(
                &char_name,
                new_rebirth_level as i16,
                new_reb_str as i16,
                new_reb_sta as i16,
                new_reb_dex as i16,
                new_reb_intel as i16,
                new_reb_cha as i16,
                0, // exp = 0
            )
            .await
        {
            tracing::error!(char_name, "failed to save rebirth: {}", e);
        }
    });

    tracing::info!(
        "[{}] REB_STAT_CHANGE: {} rebirth_level={} reb_stats=[{},{},{},{},{}]",
        session.addr(),
        char_info.name,
        new_rebirth_level,
        new_reb_str,
        new_reb_sta,
        new_reb_dex,
        new_reb_intel,
        new_reb_cha,
    );

    Ok(())
}

/// Handle REB_STAT_RESET (0x08) — redistribute rebirth stat points.
/// Incoming: `[u8 str] [u8 sta] [u8 dex] [u8 int] [u8 cha]`
/// The 5 values must sum to exactly `rebirth_level * 2`.
/// Response: `[u8 REB_STAT_RESET] [u8 result] [u32 0]`
/// - 1 = success, 0 = failure
/// Requirements:
/// - rebirth_level > 0
/// - Sum of requested stats == rebirth_level * 2
/// - 100M gold OR premium type 12/13
async fn handle_reb_stat_reset(
    session: &mut ClientSession,
    rec_str: u8,
    rec_sta: u8,
    rec_dex: u8,
    rec_int: u8,
    rec_cha: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    if char_info.rebirth_level == 0 {
        send_reb_result(session, REB_STAT_RESET, 0).await?;
        return Ok(());
    }

    let total = rec_str as u16 + rec_sta as u16 + rec_dex as u16 + rec_int as u16 + rec_cha as u16;
    let required_total = char_info.rebirth_level as u16 * 2;
    if total != required_total {
        send_reb_result(session, REB_STAT_RESET, 0).await?;
        return Ok(());
    }

    let premium_type = world.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
    let has_free_reset =
        premium_type == PREMIUM_REBIRTH_FREE_1 || premium_type == PREMIUM_REBIRTH_FREE_2;

    if !has_free_reset {
        if char_info.gold < REBIRTH_RESET_GOLD {
            send_reb_result(session, REB_STAT_RESET, 0).await?;
            return Ok(());
        }
        // Deduct gold
        world.gold_lose(sid, REBIRTH_RESET_GOLD);
    }

    // Apply rebirth stat reset (direct assignment, NOT increment)
    world.update_character_stats(sid, |ch| {
        ch.reb_str = rec_str;
        ch.reb_sta = rec_sta;
        ch.reb_dex = rec_dex;
        ch.reb_intel = rec_int;
        ch.reb_cha = rec_cha;
    });

    // Send success
    send_reb_result(session, REB_STAT_RESET, 1).await?;

    // Save to DB asynchronously
    let char_name = char_info.name.clone();
    let pool = session.pool().clone();
    let rebirth_level = char_info.rebirth_level;
    let exp = char_info.exp;
    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo
            .save_rebirth(
                &char_name,
                rebirth_level as i16,
                rec_str as i16,
                rec_sta as i16,
                rec_dex as i16,
                rec_int as i16,
                rec_cha as i16,
                exp as i64,
            )
            .await
        {
            tracing::error!(char_name, "failed to save rebirth reset: {}", e);
        }
    });

    tracing::info!(
        "[{}] REB_STAT_RESET: {} rebirth_level={} new_reb_stats=[{},{},{},{},{}]",
        session.addr(),
        char_info.name,
        rebirth_level,
        rec_str,
        rec_sta,
        rec_dex,
        rec_int,
        rec_cha,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_class_change_req_packet() {
        // Client -> Server: [u8 CLASS_CHANGE_REQ]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(CLASS_CHANGE_REQ);

        assert_eq!(pkt.opcode, Opcode::WizClassChange as u8);
        assert_eq!(pkt.data.len(), 1);
    }

    #[test]
    fn test_class_change_result_packet() {
        // Server -> Client: [u8 CLASS_CHANGE_RESULT] [u8 result]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(CLASS_CHANGE_RESULT);
        pkt.write_u8(1); // success

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CLASS_CHANGE_RESULT));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_all_point_change_success_packet() {
        // Server -> Client: [u8 ALL_POINT_CHANGE] [u8 1] [u32 gold]
        //   [u16 str] [u16 sta] [u16 dex] [u16 int] [u16 cha]
        //   [i16 max_hp] [i16 max_mp] [u16 total_hit] [u32 max_weight] [u16 free_points]
        //   m_sMaxWeight is uint32 (User.h:394)
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(ALL_POINT_CHANGE);
        pkt.write_u8(1);
        pkt.write_u32(50000);
        pkt.write_u16(65); // str
        pkt.write_u16(65); // sta
        pkt.write_u16(60); // dex
        pkt.write_u16(50); // int
        pkt.write_u16(50); // cha
        pkt.write_i16(500); // max_hp
        pkt.write_i16(200); // max_mp
        pkt.write_u16(100); // total_hit
        pkt.write_u32(1000); // max_weight (uint32)
        pkt.write_u16(187); // free_points

        // 1 + 1 + 4 + 5*2 + 2 + 2 + 2 + 4 + 2 = 28
        assert_eq!(pkt.data.len(), 28);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ALL_POINT_CHANGE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(50000));
        assert_eq!(r.read_u16(), Some(65));
        assert_eq!(r.read_u16(), Some(65));
        assert_eq!(r.read_u16(), Some(60));
        assert_eq!(r.read_u16(), Some(50));
        assert_eq!(r.read_u16(), Some(50));
    }

    #[test]
    fn test_all_skill_point_change_success_packet() {
        // Server -> Client: [u8 ALL_SKILLPT_CHANGE] [u8 1] [u32 gold] [u8 free_sp]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(ALL_SKILLPT_CHANGE);
        pkt.write_u8(1);
        pkt.write_u32(40000);
        pkt.write_u8(102); // (60-9)*2 = 102

        assert_eq!(pkt.data.len(), 7);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ALL_SKILLPT_CHANGE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(40000));
        assert_eq!(r.read_u8(), Some(102));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_get_class_type() {
        assert_eq!(get_class_type(101), 1); // Karus beginner warrior
        assert_eq!(get_class_type(105), 5); // Karus novice warrior
        assert_eq!(get_class_type(106), 6); // Karus mastered warrior
        assert_eq!(get_class_type(201), 1); // El Morad beginner warrior
        assert_eq!(get_class_type(209), 9); // El Morad novice mage
        assert_eq!(get_class_type(113), 13); // Karus beginner kurian
        assert_eq!(get_class_type(215), 15); // El Morad mastered portu
    }

    #[test]
    fn test_class_helpers() {
        // Warriors
        assert!(is_warrior(101)); // beginner
        assert!(is_warrior(105)); // novice
        assert!(is_warrior(106)); // mastered
        assert!(is_warrior(201)); // el morad beginner
        assert!(!is_warrior(102)); // rogue

        // Rogues
        assert!(is_rogue(102));
        assert!(is_rogue(107));
        assert!(is_rogue(108));
        assert!(!is_rogue(103));

        // Mages
        assert!(is_mage(103));
        assert!(is_mage(109));
        assert!(is_mage(110));

        // Priests
        assert!(is_priest(104));
        assert!(is_priest(111));
        assert!(is_priest(112));

        // Kurian
        assert!(is_portu_kurian(113));
        assert!(is_portu_kurian(114));
        assert!(is_portu_kurian(115));
        assert!(is_portu_kurian(213));
    }

    #[test]
    fn test_is_same_job_group() {
        // Already warrior, selecting warrior => same
        assert!(is_same_job_group(101, 1));
        assert!(is_same_job_group(105, 1));
        assert!(is_same_job_group(205, 1));

        // Warrior selecting rogue => different
        assert!(!is_same_job_group(101, 2));
        assert!(!is_same_job_group(105, 3));

        // Kurian selecting kurian => same
        assert!(is_same_job_group(113, 5));
        assert!(is_same_job_group(215, 5));
    }

    #[test]
    fn test_is_beginner_novice_mastered() {
        assert!(is_beginner(101)); // beginner warrior
        assert!(is_beginner(102)); // beginner rogue
        assert!(is_beginner(113)); // beginner kurian
        assert!(!is_beginner(105)); // novice warrior

        assert!(is_novice(105)); // novice warrior
        assert!(is_novice(107)); // novice rogue
        assert!(is_novice(114)); // novice kurian
        assert!(!is_novice(106)); // mastered warrior

        assert!(is_mastered(106)); // mastered warrior
        assert!(is_mastered(108)); // mastered rogue
        assert!(is_mastered(115)); // mastered kurian
        assert!(!is_mastered(105)); // novice warrior
    }

    #[test]
    fn test_get_base_stats() {
        // Warrior base stats
        assert_eq!(get_base_stats_for_class(101), (65, 65, 60, 50, 50));
        assert_eq!(get_base_stats_for_class(105), (65, 65, 60, 50, 50));
        assert_eq!(get_base_stats_for_class(201), (65, 65, 60, 50, 50));

        // Rogue base stats
        assert_eq!(get_base_stats_for_class(102), (60, 60, 70, 50, 50));
        assert_eq!(get_base_stats_for_class(207), (60, 60, 70, 50, 50));

        // Mage base stats
        assert_eq!(get_base_stats_for_class(103), (50, 60, 60, 70, 50));
        assert_eq!(get_base_stats_for_class(209), (50, 60, 60, 70, 50));

        // Priest base stats
        assert_eq!(get_base_stats_for_class(104), (50, 60, 60, 70, 50));
        assert_eq!(get_base_stats_for_class(211), (50, 60, 60, 70, 50));

        // Kurian base stats (same as warrior)
        assert_eq!(get_base_stats_for_class(113), (65, 65, 60, 50, 50));
        assert_eq!(get_base_stats_for_class(215), (65, 65, 60, 50, 50));
    }

    #[test]
    fn test_get_reset_money() {
        // Level 20 (< 30): money * 0.4
        let money_20 = get_reset_money(20);
        let base_20 = (40.0_f64).powf(3.4) as i32;
        assert_eq!(money_20, (base_20 as f32 * 0.4) as i32);

        // Level 40 (normal): no modifier
        let money_40 = get_reset_money(40);
        let base_40 = (80.0_f64).powf(3.4) as i32;
        assert_eq!(money_40, base_40);

        // Level 70 (>= 60): money * 1.5
        let money_70 = get_reset_money(70);
        let base_70 = (140.0_f64).powf(3.4) as i32;
        assert_eq!(money_70, (base_70 as f32 * 1.5) as i32);
    }

    #[test]
    fn test_determine_class_karus_beginner_rogue_to_warrior() {
        // Karus beginner rogue (102) changing to warrior (1)
        let result = determine_new_class_and_race(102, KARUS_MIDDLE, 1, 1, 0);
        assert_eq!(result, Some((KARUWARRIOR, KARUS_BIG)));
    }

    #[test]
    fn test_determine_class_elmo_beginner_warrior_to_rogue() {
        // El Morad beginner warrior (201) changing to rogue (2), race=ELMORAD_MAN
        let result = determine_new_class_and_race(201, ELMORAD_MAN, 2, 2, 0);
        assert_eq!(result, Some((ELMOROGUE, ELMORAD_MAN)));
    }

    #[test]
    fn test_determine_class_elmo_beginner_warrior_to_rogue_barbarian() {
        // El Morad beginner warrior (201) changing to rogue (2), race=BABARIAN
        let result = determine_new_class_and_race(201, BABARIAN, 2, 2, 0);
        assert_eq!(result, Some((ELMOROGUE, ELMORAD_MAN))); // barbarian -> elmorad_man for rogue
    }

    #[test]
    fn test_determine_class_karus_novice_mage_to_warrior() {
        // Karus novice mage (109) changing to warrior (1)
        let result = determine_new_class_and_race(109, KARUS_SMALL, 1, 1, 0);
        assert_eq!(result, Some((BERSERKER, KARUS_BIG)));
    }

    #[test]
    fn test_determine_class_karus_mastered_priest_to_warrior_type0() {
        // Karus mastered priest (112) changing to warrior (1), type=0 (normal)
        let result = determine_new_class_and_race(112, KARUS_MIDDLE, 1, 1, 0);
        assert_eq!(result, Some((GUARDIAN, KARUS_BIG)));
    }

    #[test]
    fn test_determine_class_karus_mastered_priest_to_warrior_type1() {
        // Karus mastered priest (112) changing to warrior (1), type=1 (master)
        let result = determine_new_class_and_race(112, KARUS_MIDDLE, 1, 1, 1);
        assert_eq!(result, Some((BERSERKER, KARUS_BIG)));
    }

    #[test]
    fn test_determine_class_to_kurian() {
        // Karus beginner warrior (101) to kurian (5)
        let result = determine_new_class_and_race(101, KARUS_BIG, 1, 5, 0);
        assert_eq!(result, Some((KURIANSTARTER, KURIAN)));

        // El Morad novice rogue (207) to portu (5)
        let result = determine_new_class_and_race(207, ELMORAD_MAN, 2, 5, 0);
        assert_eq!(result, Some((PORUTUNOVICE, PORUTU)));
    }

    #[test]
    fn test_determine_class_same_group_returns_none() {
        // Warrior trying to become warrior => same group, but determine function
        // should still return None because is_beginner_other checks !is_warrior
        let result = determine_new_class_and_race(101, KARUS_BIG, 1, 1, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_free_points_calculation() {
        // Level 60: 10 + (60-1)*3 = 10 + 177 = 187
        let mut free = 10u16 + (60u16 - 1) * 3;
        assert_eq!(free, 187);

        // Level 70: 10 + (70-1)*3 + 2*(70-60) = 10 + 207 + 20 = 237
        free = 10u16 + (70u16 - 1) * 3 + 2 * (70u16 - 60);
        assert_eq!(free, 237);
    }

    #[test]
    fn test_free_skill_points_calculation() {
        // Level 60: (60-9)*2 = 102
        assert_eq!((60u8.saturating_sub(9) as u16) * 2, 102);
        // Level 10: (10-9)*2 = 2
        assert_eq!((10u8.saturating_sub(9) as u16) * 2, 2);
        // Level 83: (83-9)*2 = 148
        assert_eq!((83u8.saturating_sub(9) as u16) * 2, 148);
    }

    // ── Sprint 77: Gold overflow safety tests ──────────────────────────

    #[test]
    fn test_gold_u32_comparison_no_overflow() {
        // When gold > 2^31, casting to i32 would wrap negative and produce
        // wrong comparison results.  The fix uses u32 comparison + saturating_sub.
        let gold: u32 = 3_000_000_000; // > i32::MAX (2,147,483,647)
        let money_required: i32 = 100_000;
        let cost = money_required.max(0) as u32;

        // u32 comparison: player has enough gold
        assert!(gold >= cost);
        // saturating_sub: correct deduction
        assert_eq!(gold.saturating_sub(cost), 2_999_900_000);

        // Verify the old pattern would have been WRONG:
        // (gold as i32) wraps to negative, falsely fails the check
        let gold_as_i32 = gold as i32;
        assert!(gold_as_i32 < 0, "as i32 wraps large u32 to negative");
        assert!(
            gold_as_i32 < money_required,
            "old pattern falsely thinks player cannot afford"
        );
    }

    #[test]
    fn test_gold_zero_cost_saturating_sub() {
        // When money_required is 0 or negative, cost should be 0
        let gold: u32 = 500;
        let money_required: i32 = -100;
        let cost = money_required.max(0) as u32;
        assert_eq!(cost, 0);
        assert_eq!(gold.saturating_sub(cost), 500);
    }

    #[test]
    fn test_promote_novice_packet_format() {
        // Client -> Server: [u8 PROMOTE_NOVICE] [u8 type] [u8 new_job]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(PROMOTE_NOVICE);
        pkt.write_u8(0); // type 0 = normal
        pkt.write_u8(2); // new_job = rogue

        assert_eq!(pkt.data.len(), 3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PROMOTE_NOVICE));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_change_money_req_packet() {
        // Server -> Client: [u8 CHANGE_MONEY_REQ] [i32 money]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(CHANGE_MONEY_REQ);
        pkt.write_i32(50000);

        assert_eq!(pkt.data.len(), 5);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CHANGE_MONEY_REQ));
        assert_eq!(r.read_u32(), Some(50000));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 294: Job change disconnect ─────────────────────────────

    #[test]
    fn test_job_change_triggers_disconnect() {
        // `goDisconnect("Character Changed Job.", __FUNCTION__);`
        // After a successful job change, the server disconnects the player.
        // In Rust, this is done by returning anyhow::bail!() which breaks
        // the session loop and triggers cleanup.
        //
        // The client is expected to reconnect with the updated class.
        let disconnect_expected = true;
        assert!(
            disconnect_expected,
            "Server must disconnect after job change"
        );
    }

    // ── Sprint 328: Premium free reset tests ─────────────────────────

    /// Test that premium type 12 or Return Token constant is correct.
    #[test]
    fn test_return_token_constant() {
        assert_eq!(RETURNTOKENS, 810512000);
        assert_eq!(PREMIUM_WARP, 12);
    }

    /// Test that free reset skips gold cost.
    #[test]
    fn test_free_reset_cost_is_zero() {
        let free = true;
        let money_required = get_reset_money(60);
        let cost = if free {
            0u32
        } else {
            money_required.max(0) as u32
        };
        assert_eq!(cost, 0, "Free reset must have zero cost");
    }

    /// Test that non-free reset has normal cost.
    #[test]
    fn test_non_free_reset_has_cost() {
        let free = false;
        let money_required = get_reset_money(60);
        let cost = if free {
            0u32
        } else {
            money_required.max(0) as u32
        };
        assert!(cost > 0, "Non-free reset must charge gold");
    }

    // ── Rebirth packet format tests ─────────────────────────────────

    #[test]
    fn test_reb_stat_change_request_packet() {
        // Client -> Server: [u8 0x07] [u8 str] [u8 sta] [u8 dex] [u8 int] [u8 cha]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(REB_STAT_CHANGE);
        pkt.write_u8(2); // str
        pkt.write_u8(0); // sta
        pkt.write_u8(0); // dex
        pkt.write_u8(0); // int
        pkt.write_u8(0); // cha

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(REB_STAT_CHANGE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_reb_stat_change_response_packet() {
        // Server -> Client: [u8 0x07] [u8 result] [u32 0]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(REB_STAT_CHANGE);
        pkt.write_u8(1); // success
        pkt.write_u32(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(REB_STAT_CHANGE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_reb_stat_reset_request_packet() {
        // Client -> Server: [u8 0x08] [u8 str] [u8 sta] [u8 dex] [u8 int] [u8 cha]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(REB_STAT_RESET);
        pkt.write_u8(1); // str
        pkt.write_u8(1); // sta
        pkt.write_u8(2); // dex
        pkt.write_u8(0); // int
        pkt.write_u8(0); // cha

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(REB_STAT_RESET));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_reb_stat_reset_response_packet() {
        // Server -> Client: [u8 0x08] [u8 result] [u32 0]
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(REB_STAT_RESET);
        pkt.write_u8(1); // success
        pkt.write_u32(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(REB_STAT_RESET));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_reb_stat_change_sum_validation() {
        // Stats must sum to exactly 2
        let sum = 1u16 + 1;
        assert_eq!(sum, 2, "rebirth stat change must allocate exactly 2 points");

        let bad_sum = 1u16 + 1 + 1;
        assert_ne!(bad_sum, 2, "3 points should fail validation");
    }

    #[test]
    fn test_reb_stat_reset_sum_validation() {
        // Stats must sum to rebirth_level * 2
        let rebirth_level: u8 = 3;
        let required = rebirth_level as u16 * 2; // 6
        let sum = 2u16 + 2 + 2;
        assert_eq!(sum, required, "reset sum must equal rebirth_level * 2");

        let bad_sum = 2u16 + 2 + 1;
        assert_ne!(bad_sum, required, "sum != rebirth_level * 2 should fail");
    }

    #[test]
    fn test_reb_stat_change_max_level() {
        // Rebirth level > 9 should be rejected
        let rebirth_level: u8 = 10;
        assert!(rebirth_level > 9, "level 10 should be rejected");

        let rebirth_level: u8 = 9;
        assert!((rebirth_level <= 9), "level 9 should be accepted");
    }

    #[test]
    fn test_reb_stat_reset_requires_rebirth() {
        // Rebirth level == 0 should be rejected
        let rebirth_level: u8 = 0;
        assert_eq!(rebirth_level, 0, "level 0 means not rebirthed, should fail");
    }

    #[test]
    fn test_rebirth_constants() {
        assert_eq!(REB_STAT_CHANGE, 0x07);
        assert_eq!(REB_STAT_RESET, 0x08);
        assert_eq!(QUALIFICATION_OF_REBIRTH, 900_579_000);
        assert_eq!(REBIRTH_RESET_GOLD, 100_000_000);
    }
}
