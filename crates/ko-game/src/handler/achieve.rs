//! WIZ_USER_ACHIEVE (0x99) handler — achievement system.
//! Manages user achievements, titles, and rewards.
//! Sub-opcodes:
//! - 2: AchieveGiftRequest — claim reward item for a completed achievement
//! - 3: AchieveDetailShow — view achievement detail status
//! - 4: AchieveSummary — summary stats (play time, kills, deaths, medals, type counts)
//! - 6: AchieveStart — start a timed challenge
//! - 7: AchieveStop — cancel a timed challenge
//! - 16: AchieveCoverTitle — equip a cover (display) title
//! - 17: AchieveSkillTitle — equip a skill title (with stat bonuses)
//! - 18: AchieveCoverTitleReset — unequip cover title
//! - 19: AchieveSkillTitleReset — unequip skill title (clear stat bonuses)

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::world::types::UserAchieveInfo;
use crate::world::WorldState;
use crate::zone::SessionId;

#[repr(u8)]
#[allow(dead_code)]
enum AchieveOpcode {
    Error = 0,
    Success = 1,
    GiftRequest = 2,
    DetailShow = 3,
    Summary = 4,
    Failed = 5,
    Start = 6,
    Stop = 7,
    ChallengeFailed = 8,
    CountScreen = 9,
    CoverTitle = 16,
    SkillTitle = 17,
    CoverTitleReset = 18,
    SkillTitleReset = 19,
}

#[repr(u8)]
enum AchieveStatus {
    ChallengeIncomplete = 0,
    Incomplete = 1,
    Finished = 4,
    Completed = 5,
}

/// Populate the per-character map from the complete client-backed master table.
/// The original GameServer creates one entry for every ACHIEVE_main row when a
/// character has no saved blob; missing rows are also added after client table updates.
pub fn initialize_achievement_map(world: &WorldState, sid: SessionId) -> usize {
    let definitions = world.all_achieve_main();
    let mut inserted = 0usize;
    world.update_session(sid, |h| {
        for definition in &definitions {
            let id = definition.s_index as u16;
            h.achieve_map.entry(id).or_insert_with(|| {
                inserted += 1;
                UserAchieveInfo {
                    status: if definition.byte2 == 41 || definition.byte2 == 42 {
                        AchieveStatus::ChallengeIncomplete as u8
                    } else {
                        AchieveStatus::Incomplete as u8
                    },
                    count: [0, 0],
                }
            });
        }
        h.achieve_login_time = unix_time();
    });
    inserted
}

/// GM support: finish every v2615 achievement for an online character.
/// Returns the number of entries changed from incomplete to finished.
pub fn unlock_all_achievements(world: &WorldState, sid: SessionId) -> usize {
    initialize_achievement_map(world, sid);
    let mut definitions = world.all_achieve_main();
    definitions.sort_by_key(|row| row.s_index);
    let mut changed_ids = Vec::new();
    world.update_session(sid, |h| {
        for definition in &definitions {
            let id = definition.s_index as u16;
            let Some(info) = h.achieve_map.get_mut(&id) else {
                continue;
            };
            if is_finished(info.status) {
                continue;
            }
            info.status = AchieveStatus::Finished as u8;
            changed_ids.push(id);
        }
        h.achieve_summary.total_medal = definitions
            .iter()
            .filter(|row| {
                h.achieve_map
                    .get(&(row.s_index as u16))
                    .is_some_and(|i| is_finished(i.status))
            })
            .map(|row| row.point.max(0) as u32)
            .fold(0u32, u32::saturating_add);
        for id in changed_ids.iter().rev().take(3).rev() {
            h.achieve_summary.recent_achieve.rotate_right(1);
            h.achieve_summary.recent_achieve[0] = *id;
        }
    });

    for id in &changed_ids {
        let mut pkt = Packet::new(Opcode::WizUserAchieve as u8);
        pkt.write_u8(AchieveOpcode::Success as u8);
        pkt.write_u16(*id);
        pkt.write_u8(AchieveStatus::Finished as u8);
        world.send_to_session_owned(sid, pkt);
    }

    if let (Some(pool), Some(character_name)) =
        (world.db_pool().cloned(), world.get_session_name(sid))
    {
        let entries = world
            .with_session(sid, |h| {
                h.achieve_map
                    .iter()
                    .map(|(&id, info)| {
                        (
                            id as i32,
                            info.status as i16,
                            info.count[0] as i32,
                            info.count[1] as i32,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        tokio::spawn(async move {
            let repo = ko_db::repositories::achieve::AchieveRepository::new(&pool);
            if let Err(error) = repo
                .save_user_achieves_batch(&character_name, &entries)
                .await
            {
                tracing::error!(character = %character_name, %error,
                    "GM achievement unlock batch could not be persisted");
            }
        });
    }
    changed_ids.len()
}

fn unix_time() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

fn is_finished(status: u8) -> bool {
    status == AchieveStatus::Finished as u8 || status == AchieveStatus::Completed as u8
}

fn persist_achievement(world: &WorldState, sid: SessionId, id: u16) {
    let entry = world
        .with_session(sid, |h| h.achieve_map.get(&id).cloned())
        .flatten();
    let (Some(entry), Some(pool), Some(character_name)) =
        (entry, world.db_pool().cloned(), world.get_session_name(sid))
    else {
        return;
    };
    tokio::spawn(async move {
        let repo = ko_db::repositories::achieve::AchieveRepository::new(&pool);
        if let Err(error) = repo
            .save_user_achieve(
                &character_name,
                id as i32,
                entry.status as i16,
                entry.count[0] as i32,
                entry.count[1] as i32,
            )
            .await
        {
            tracing::error!(character = %character_name, achievement_id = id, %error,
                "Failed to persist achievement progress");
        }
    });
}

/// Mark an achievement finished exactly once and apply all summary side effects.
fn finish_achievement(world: &WorldState, sid: SessionId, id: u16) -> bool {
    let Some(main) = world.achieve_main(id as i32) else {
        return false;
    };
    let mut changed = false;
    world.update_session(sid, |h| {
        let Some(info) = h.achieve_map.get_mut(&id) else {
            return;
        };
        if is_finished(info.status) {
            return;
        }
        info.status = AchieveStatus::Finished as u8;
        h.achieve_summary.recent_achieve[2] = h.achieve_summary.recent_achieve[1];
        h.achieve_summary.recent_achieve[1] = h.achieve_summary.recent_achieve[0];
        h.achieve_summary.recent_achieve[0] = id;
        h.achieve_summary.total_medal = h
            .achieve_summary
            .total_medal
            .saturating_add(main.point.max(0) as u32);
        changed = true;
    });
    if !changed {
        return false;
    }
    let mut pkt = Packet::new(Opcode::WizUserAchieve as u8);
    pkt.write_u8(AchieveOpcode::Success as u8);
    pkt.write_u16(id);
    pkt.write_u8(AchieveStatus::Finished as u8);
    world.send_to_session_owned(sid, pkt);
    persist_achievement(world, sid, id);
    evaluate_achievement_dependencies(world, sid);
    true
}

fn achievement_is_finished(world: &WorldState, sid: SessionId, id: u16) -> bool {
    world
        .with_session(sid, |h| {
            h.achieve_map
                .get(&id)
                .is_some_and(|i| is_finished(i.status))
        })
        .unwrap_or(false)
}

/// Evaluate ACHIEVE_com rows. Repeats because one composite may unlock another.
pub fn evaluate_achievement_dependencies(world: &WorldState, sid: SessionId) {
    loop {
        let mut newly_finished = Vec::new();
        for row in world.all_achieve_com() {
            let id = row.s_index as u16;
            if achievement_is_finished(world, sid, id) {
                continue;
            }
            let requirement_met = match row.r#type {
                1 => world
                    .with_session(sid, |h| {
                        let done = |qid: i32| {
                            qid <= 0
                                || h.quests
                                    .get(&(qid as u16))
                                    .is_some_and(|q| q.quest_state == 2)
                        };
                        done(row.req1) && done(row.req2)
                    })
                    .unwrap_or(false),
                2 => {
                    achievement_is_finished(world, sid, row.req1 as u16)
                        && (row.req2 <= 0 || achievement_is_finished(world, sid, row.req2 as u16))
                }
                _ => false,
            };
            if requirement_met {
                newly_finished.push(id);
            }
        }
        if newly_finished.is_empty() {
            break;
        }
        // Inline completion prevents recursive evaluation from nesting forever.
        for id in newly_finished {
            let Some(main) = world.achieve_main(id as i32) else {
                continue;
            };
            let mut changed = false;
            world.update_session(sid, |h| {
                let Some(info) = h.achieve_map.get_mut(&id) else {
                    return;
                };
                if is_finished(info.status) {
                    return;
                }
                info.status = AchieveStatus::Finished as u8;
                info.count[0] = 1;
                h.achieve_summary.recent_achieve.rotate_right(1);
                h.achieve_summary.recent_achieve[0] = id;
                h.achieve_summary.total_medal = h
                    .achieve_summary
                    .total_medal
                    .saturating_add(main.point.max(0) as u32);
                changed = true;
            });
            if changed {
                let mut pkt = Packet::new(Opcode::WizUserAchieve as u8);
                pkt.write_u8(AchieveOpcode::Success as u8);
                pkt.write_u16(id);
                pkt.write_u8(AchieveStatus::Finished as u8);
                world.send_to_session_owned(sid, pkt);
                persist_achievement(world, sid, id);
            }
        }
    }
}

pub fn on_quest_completed(world: &WorldState, sid: SessionId) {
    evaluate_achievement_dependencies(world, sid);
}

/// Advance exact-monster, consecutive-monster and Ronark monster achievements.
pub fn on_monster_killed(world: &WorldState, sid: SessionId, npc_sid: u16, zone_id: u16) {
    let now = unix_time();
    for row in world.all_achieve_monster() {
        let id = row.s_index as u16;
        let Some(main) = world.achieve_main(row.s_index) else {
            continue;
        };
        if main.zone_id > 0 && main.zone_id as u16 != zone_id {
            continue;
        }
        if main.req_time > 0
            && !world
                .with_session(sid, |h| {
                    h.achieve_timed
                        .get(&id)
                        .is_some_and(|expiration| *expiration >= now)
                })
                .unwrap_or(false)
        {
            continue;
        }
        let groups = [
            (
                [
                    row.monster1_1,
                    row.monster1_2,
                    row.monster1_3,
                    row.monster1_4,
                ],
                row.mon_count1,
            ),
            (
                [
                    row.monster2_1,
                    row.monster2_2,
                    row.monster2_3,
                    row.monster2_4,
                ],
                row.mon_count2,
            ),
        ];
        for (group, (monsters, required)) in groups.iter().enumerate() {
            if *required <= 0 || !monsters.contains(&(npc_sid as i32)) {
                continue;
            }
            let mut changed = false;
            let mut complete = false;
            world.update_session(sid, |h| {
                let Some(info) = h.achieve_map.get_mut(&id) else {
                    return;
                };
                if is_finished(info.status) {
                    return;
                }
                info.count[group] = info.count[group].saturating_add(1).min(*required as u32);
                complete = info.count[0] >= row.mon_count1.max(0) as u32
                    && info.count[1] >= row.mon_count2.max(0) as u32;
                changed = true;
            });
            if changed {
                persist_achievement(world, sid, id);
            }
            if complete {
                finish_achievement(world, sid, id);
            }
        }
    }
    // Normal type 8: defeat monsters without dying.
    for row in world
        .all_achieve_normal()
        .into_iter()
        .filter(|r| r.r#type == 8)
    {
        advance_single_counter(
            world,
            sid,
            row.s_index as u16,
            row.count.max(0) as u32,
            zone_id,
        );
    }
    // War type 20: monster kills in the row's configured PK zone.
    for row in world
        .all_achieve_war()
        .into_iter()
        .filter(|r| r.r#type == 20)
    {
        advance_single_counter(
            world,
            sid,
            row.s_index as u16,
            row.s_count.max(0) as u32,
            zone_id,
        );
    }
}

/// Advance enemy-user defeat achievements (type 1), respecting ACHIEVE_main.ZoneID.
pub fn on_enemy_user_killed(world: &WorldState, sid: SessionId, zone_id: u16) {
    for row in world
        .all_achieve_war()
        .into_iter()
        .filter(|r| r.r#type == 1)
    {
        advance_single_counter(
            world,
            sid,
            row.s_index as u16,
            row.s_count.max(0) as u32,
            zone_id,
        );
    }
}

/// Advance event-result war achievements such as Juraid win (6) and Draki finish (21).
pub fn on_war_event_result(world: &WorldState, sid: SessionId, war_type: i16) {
    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    for row in world
        .all_achieve_war()
        .into_iter()
        .filter(|row| row.r#type == war_type)
    {
        advance_single_counter(
            world,
            sid,
            row.s_index as u16,
            row.s_count.max(0) as u32,
            zone_id,
        );
    }
}

/// Reset the progress for normal type 8 ("defeat monsters without dying").
pub fn on_player_died(world: &WorldState, sid: SessionId) {
    for row in world
        .all_achieve_normal()
        .into_iter()
        .filter(|row| row.r#type == 8)
    {
        let id = row.s_index as u16;
        let mut changed = false;
        world.update_session(sid, |h| {
            if let Some(info) = h.achieve_map.get_mut(&id) {
                if !is_finished(info.status) && info.count[0] != 0 {
                    info.count[0] = 0;
                    changed = true;
                }
            }
        });
        if changed {
            persist_achievement(world, sid, id);
        }
    }
}

fn advance_single_counter(
    world: &WorldState,
    sid: SessionId,
    id: u16,
    required: u32,
    zone_id: u16,
) {
    if required == 0 {
        return;
    }
    let Some(main) = world.achieve_main(id as i32) else {
        return;
    };
    if main.zone_id > 0 && main.zone_id as u16 != zone_id {
        return;
    }
    let mut changed = false;
    let mut complete = false;
    world.update_session(sid, |h| {
        let Some(info) = h.achieve_map.get_mut(&id) else {
            return;
        };
        if is_finished(info.status) {
            return;
        }
        info.count[0] = info.count[0].saturating_add(1).min(required);
        complete = info.count[0] >= required;
        changed = true;
    });
    if changed {
        persist_achievement(world, sid, id);
    }
    if complete {
        finish_achievement(world, sid, id);
    }
}

/// Atomically consume an item cost and finish an achievement from a Lua NPC.
///
/// Return codes are intentionally numeric for Lua:
/// 0=invalid/failure, 1=success, 2=already completed, 3=insufficient items.
pub fn exchange_item_for_achievement(
    world: &WorldState,
    sid: SessionId,
    item_id: u32,
    item_count: u16,
    achievement_id: u16,
) -> i32 {
    if item_count == 0 || world.achieve_main(achievement_id as i32).is_none() {
        tracing::warn!(
            "Achievement exchange rejected: invalid definition achievement_id={} item_id={} count={} sid={}",
            achievement_id,
            item_id,
            item_count,
            sid
        );
        return 0;
    }

    let existing_status = world
        .with_session(sid, |h| {
            h.achieve_map.get(&achievement_id).map(|entry| entry.status)
        })
        .flatten();
    if existing_status.is_some_and(|status| status >= AchieveStatus::Finished as u8) {
        return 2;
    }

    let owned_count: u32 = world
        .get_inventory(sid)
        .iter()
        .filter(|slot| slot.item_id == item_id)
        .map(|slot| slot.count as u32)
        .sum();
    if owned_count < item_count as u32 {
        return 3;
    }
    if !world.rob_item(sid, item_id, item_count) {
        tracing::warn!(
            "Achievement exchange failed to consume item_id={} count={} achievement_id={} sid={}",
            item_id,
            item_count,
            achievement_id,
            sid
        );
        return 0;
    }

    world.update_session(sid, |h| {
        let info = h
            .achieve_map
            .entry(achievement_id)
            .or_insert(UserAchieveInfo {
                status: AchieveStatus::Incomplete as u8,
                count: [0, 0],
            });
        info.count[0] = item_count as u32;
    });
    if !finish_achievement(world, sid, achievement_id) {
        // Consumption succeeded, so a failure here means the definition/session
        // disappeared concurrently. Log it explicitly instead of reporting success.
        tracing::error!(
            sid,
            achievement_id,
            item_id,
            item_count,
            "Achievement exchange consumed items but could not finish achievement"
        );
        return 0;
    }

    tracing::info!(
        "Achievement exchange completed: sid={} achievement_id={} item_id={} item_count={}",
        sid,
        achievement_id,
        item_id,
        item_count
    );
    1
}

/// Handle WIZ_USER_ACHIEVE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }
    let mut r = PacketReader::new(&pkt.data);
    let sub_opcode = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match sub_opcode {
        2 => handle_gift_request(session, &mut r).await,
        3 => handle_detail_show(session, &mut r).await,
        4 => handle_summary(session).await,
        6 => handle_start(session, &mut r).await,
        7 => handle_stop(session, &mut r).await,
        16 => handle_cover_title(session, &mut r).await,
        17 => handle_skill_title(session, &mut r).await,
        18 => handle_cover_title_reset(session).await,
        19 => handle_skill_title_reset(session).await,
        _ => {
            tracing::debug!(
                "[{}] WIZ_USER_ACHIEVE: unknown sub-opcode {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Sub-opcode 2: AchieveGiftRequest — claim reward for a finished achievement.
/// Client sends: [u8 opcode=2][u16 s_index]
/// Server responds: [u8 opcode=2][u16 s_index][u16 result] (1=success, 0=fail)
async fn handle_gift_request(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let s_index = match r.read_u16() {
        Some(v) => v as i32,
        None => return Ok(()),
    };

    let world = session.world().clone();

    // Look up achievement main entry
    let main_entry = match world.achieve_main(s_index) {
        Some(m) => m,
        None => {
            let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
            resp.write_u8(AchieveOpcode::GiftRequest as u8);
            resp.write_u16(s_index as u16);
            resp.write_u16(0);
            session.send_packet(&resp).await?;
            return Ok(());
        }
    };

    // Check the item exists
    let item_num = main_entry.item_num as u32;
    let item_count = main_entry.count as u16;
    if item_num == 0 || world.get_item(item_num).is_none() {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::GiftRequest as u8);
        resp.write_u16(s_index as u16);
        resp.write_u16(0);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check achievement status is Finished (4)
    let sid = session.session_id();
    let achieve_status = world.with_session(sid, |h| {
        h.achieve_map.get(&(s_index as u16)).map(|info| info.status)
    });

    match achieve_status {
        Some(Some(status)) if status == AchieveStatus::Finished as u8 => {}
        _ => {
            let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
            resp.write_u8(AchieveOpcode::GiftRequest as u8);
            resp.write_u16(s_index as u16);
            resp.write_u16(0);
            session.send_packet(&resp).await?;
            return Ok(());
        }
    }

    // Give the reward item
    if !world.give_item(sid, item_num, item_count) {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::GiftRequest as u8);
        resp.write_u16(s_index as u16);
        resp.write_u16(0);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Update status to Completed (5)
    world.update_session(sid, |h| {
        if let Some(info) = h.achieve_map.get_mut(&(s_index as u16)) {
            info.status = AchieveStatus::Completed as u8;
        }
    });

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::GiftRequest as u8);
    resp.write_u16(s_index as u16);
    resp.write_u16(1);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Sub-opcode 3: AchieveDetailShow — view achievement detail.
/// Client sends: [u8 opcode=3][u16 count][u16 achieve_id * count]
/// Server responds: [u8 opcode=3][u16 count]([u16 id][u8 status][u32 count] * count)
async fn handle_detail_show(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let num = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    // Safety: limit to 10 like C++
    if num >= 10 {
        tracing::warn!(
            "[{}] WIZ_USER_ACHIEVE DetailShow: num={} exceeds limit",
            session.addr(),
            num
        );
        return Ok(());
    }

    let mut achieve_ids = Vec::with_capacity(num as usize);
    for _ in 0..num {
        if let Some(id) = r.read_u16() {
            achieve_ids.push(id);
        }
    }

    let sid = session.session_id();
    let world = session.world().clone();

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::DetailShow as u8);
    resp.write_u16(num);

    for &id in &achieve_ids {
        let (status, count) = world
            .with_session(sid, |h| {
                h.achieve_map
                    .get(&id)
                    .map(|info| (info.status, info.count[0]))
            })
            .flatten()
            .unwrap_or((AchieveStatus::Incomplete as u8, 0));

        resp.write_u16(id);
        resp.write_u8(status);
        resp.write_u32(count);
    }

    session.send_packet(&resp).await?;
    Ok(())
}

/// Sub-opcode 4: AchieveSummary — summary stats screen.
/// Server responds: [u8 opcode=4][u32 play_time_minutes][u32 monster_kills]
///   [u32 user_kills][u32 user_deaths][u32 total_medal]
///   [u16 recent1][u16 recent2][u16 recent3]
///   [u16 normal_count][u16 quest_count][u16 war_count]
///   [u16 adventure_count][u16 challenge_count]
async fn handle_summary(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // Update play_time before reading summary.
    world.update_session(sid, |h| {
        if h.achieve_login_time > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            if now > h.achieve_login_time {
                h.achieve_summary.play_time += now - h.achieve_login_time;
            }
            h.achieve_login_time = now;
        }
    });

    let summary = world.with_session(sid, |h| h.achieve_summary.clone());
    let summary = summary.unwrap_or_default();

    // Count per-type completions from achieve_map
    let type_counts = world.with_session(sid, |h| {
        let mut normal = 0u16;
        let mut quest = 0u16;
        let mut war = 0u16;
        let mut adventure = 0u16;
        let mut challenge = 0u16;

        for (id, info) in &h.achieve_map {
            if info.status != AchieveStatus::Finished as u8
                && info.status != AchieveStatus::Completed as u8
            {
                continue;
            }
            if let Some(main) = world.achieve_main(*id as i32) {
                match main.achieve_type {
                    0 => normal += 1,
                    1 => quest += 1,
                    2 => war += 1,
                    3 => adventure += 1,
                    4 => challenge += 1,
                    _ => {}
                }
            }
        }
        (normal, quest, war, adventure, challenge)
    });

    let (normal_count, quest_count, war_count, adventure_count, challenge_count) =
        type_counts.unwrap_or((0, 0, 0, 0, 0));

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::Summary as u8);
    resp.write_u32(summary.play_time / 60); // C++ sends minutes
    resp.write_u32(summary.monster_defeat_count);
    resp.write_u32(summary.user_defeat_count);
    resp.write_u32(summary.user_death_count);
    resp.write_u32(summary.total_medal);
    resp.write_u16(summary.recent_achieve[0]);
    resp.write_u16(summary.recent_achieve[1]);
    resp.write_u16(summary.recent_achieve[2]);
    resp.write_u16(normal_count);
    resp.write_u16(quest_count);
    resp.write_u16(war_count);
    resp.write_u16(adventure_count);
    resp.write_u16(challenge_count);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Sub-opcode 6: AchieveStart — start a timed challenge.
/// Client sends: [u8 opcode=6][u16 s_index]
/// Server responds: [u8 opcode=6][u16 s_index][u16 result][u16 req_time]
async fn handle_start(session: &mut ClientSession, r: &mut PacketReader<'_>) -> anyhow::Result<()> {
    let s_index = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let world = session.world().clone();

    // Validate main entry exists
    let main_entry = match world.achieve_main(s_index as i32) {
        Some(m) => m,
        None => {
            let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
            resp.write_u8(AchieveOpcode::Start as u8);
            resp.write_u16(0);
            resp.write_u16(0xFFFE); // -2
            session.send_packet(&resp).await?;
            return Ok(());
        }
    };

    let sid = session.session_id();

    // Check user has this achievement in their map
    let has_achieve = world.with_session(sid, |h| h.achieve_map.contains_key(&s_index));
    if !has_achieve.unwrap_or(false) {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::Start as u8);
        resp.write_u16(0);
        resp.write_u16(0xFFFE); // -2
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Special challenge check (458/459 always fail)
    if main_entry.achieve_type == 4 && (main_entry.s_index == 458 || main_entry.s_index == 459) {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::ChallengeFailed as u8);
        resp.write_u16(s_index);
        resp.write_u16(1);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check not already in a timed challenge
    let already_timed = world.with_session(sid, |h| {
        h.achieve_challenge_active || h.achieve_timed.contains_key(&s_index)
    });
    if already_timed.unwrap_or(false) {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::Start as u8);
        resp.write_u16(0);
        resp.write_u16(0xFFFF); // -1
        session.send_packet(&resp).await?;
        return Ok(());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    let expiration = now + main_entry.req_time as u32;
    let req_time = main_entry.req_time as u16;

    // Set timed challenge state
    world.update_session(sid, |h| {
        h.achieve_challenge_active = true;
        h.achieve_timed.insert(s_index, expiration);
        if let Some(info) = h.achieve_map.get_mut(&s_index) {
            info.status = AchieveStatus::Incomplete as u8;
        }
    });

    // Send start success
    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::Start as u8);
    resp.write_u16(s_index);
    resp.write_u16(1);
    resp.write_u16(req_time);
    session.send_packet(&resp).await?;

    // Send count screen update
    let counts = world.with_session(sid, |h| {
        h.achieve_map
            .get(&s_index)
            .map(|info| (info.count[0] as u16, info.count[1] as u16))
    });
    let (c0, c1) = counts.flatten().unwrap_or((0, 0));

    let mut count_resp = Packet::new(Opcode::WizUserAchieve as u8);
    count_resp.write_u8(AchieveOpcode::CountScreen as u8);
    count_resp.write_u16(s_index);
    count_resp.write_u8(1);
    count_resp.write_u16(c0);
    count_resp.write_u16(c1);
    session.send_packet(&count_resp).await?;

    Ok(())
}

/// Sub-opcode 7: AchieveStop — cancel a timed challenge.
/// Client sends: [u8 opcode=7][u16 s_index]
/// Server responds: [u8 opcode=7][u16 s_index][u16 result]
async fn handle_stop(session: &mut ClientSession, r: &mut PacketReader<'_>) -> anyhow::Result<()> {
    let s_index = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let world = session.world().clone();
    let sid = session.session_id();

    // Validate main entry exists
    if world.achieve_main(s_index as i32).is_none() {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::Stop as u8);
        resp.write_u16(0);
        resp.write_u16(0xFFFE); // -2
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check user has this achievement
    let has_achieve = world.with_session(sid, |h| h.achieve_map.contains_key(&s_index));
    if !has_achieve.unwrap_or(false) {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::Stop as u8);
        resp.write_u16(0);
        resp.write_u16(0xFFFE); // -2
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check timed info exists and challenge is active
    let has_timed = world.with_session(sid, |h| {
        h.achieve_challenge_active && h.achieve_timed.contains_key(&s_index)
    });
    if !has_timed.unwrap_or(false) {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::Stop as u8);
        resp.write_u16(0);
        resp.write_u16(0xFFFF); // -1
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Remove timed info, reset counts
    world.update_session(sid, |h| {
        h.achieve_timed.remove(&s_index);
        h.achieve_challenge_active = false;
        if let Some(info) = h.achieve_map.get_mut(&s_index) {
            info.status = AchieveStatus::ChallengeIncomplete as u8;
            info.count[0] = 0;
            info.count[1] = 0;
        }
    });

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::Stop as u8);
    resp.write_u16(s_index);
    resp.write_u16(1);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Sub-opcode 16: AchieveCoverTitle — equip a cover (display) title.
/// Client sends: [u8 opcode=16][u16 cover_id][u16 skill_id]
/// Server responds: [u8 opcode][u16 cover_id][u16 skill_id][u8 result][u8 0]
async fn handle_cover_title(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let cover_id = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };
    let skill_id = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let world = session.world().clone();
    let sid = session.session_id();

    // Check achievement status
    let status_ok = world.with_session(sid, |h| {
        h.achieve_map.get(&cover_id).map(|info| {
            info.status == AchieveStatus::Finished as u8
                || info.status == AchieveStatus::Completed as u8
        })
    });

    let main_entry = world.achieve_main(cover_id as i32);

    if !status_ok.flatten().unwrap_or(false) || main_entry.is_none() {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::CoverTitleReset as u8);
        resp.write_u16(cover_id);
        resp.write_u16(skill_id);
        resp.write_u8(1);
        resp.write_u8(0);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Safety: main_entry is guaranteed Some by the `is_none()` check above
    let title_id = match main_entry {
        Some(m) => m.title_id as u16,
        None => return Ok(()),
    };

    world.update_session(sid, |h| {
        h.achieve_summary.cover_id = cover_id;
        h.achieve_summary.cover_title = title_id;
    });
    // Update CharacterInfo so broadcasts show the new cover title
    world.update_character_stats(sid, |ci| {
        ci.cover_title = title_id;
    });

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::CoverTitle as u8);
    resp.write_u16(cover_id);
    resp.write_u16(skill_id);
    resp.write_u8(1);
    resp.write_u8(0);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Sub-opcode 17: AchieveSkillTitle — equip a skill title (with stat bonuses).
/// Client sends: [u8 opcode=17][u16 cover_id][u16 skill_id]
async fn handle_skill_title(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let cover_id = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };
    let skill_id = match r.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let world = session.world().clone();
    let sid = session.session_id();

    // Check achievement status
    let status_ok = world.with_session(sid, |h| {
        h.achieve_map.get(&cover_id).map(|info| {
            info.status == AchieveStatus::Finished as u8
                || info.status == AchieveStatus::Completed as u8
        })
    });

    let main_entry = world.achieve_main(cover_id as i32);
    // The client sends two different IDs: cover_id authorizes the completed
    // achievement, while skill_id selects the actual ACHIEVE_TITLE stat row.
    // Looking the bonus up through cover_id silently applied a different (or
    // empty) title whenever the player selected an attack/defence variant.
    let title_entry = world.achieve_title(skill_id as i32);

    if !status_ok.flatten().unwrap_or(false) || main_entry.is_none() || title_entry.is_none() {
        // Reset skill title
        world.update_session(sid, |h| {
            h.achieve_summary.skill_title = 0;
        });
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::SkillTitleReset as u8);
        resp.write_u16(cover_id);
        resp.write_u16(skill_id);
        resp.write_u8(1);
        resp.write_u8(0);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    let main = match main_entry {
        Some(m) => m,
        None => return Ok(()),
    };
    let title = match title_entry {
        Some(t) => t,
        None => return Ok(()),
    };

    world.update_session(sid, |h| {
        h.achieve_summary.skill_id = cover_id;
        h.achieve_summary.skill_title = main.title_id as u16;
        // Apply stat bonuses
        h.achieve_stat_bonuses = [
            title.str,
            title.hp,
            title.dex,
            title.int,
            title.mp,
            title.attack,
            title.defence,
        ];
    });

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::SkillTitle as u8);
    resp.write_u16(cover_id);
    resp.write_u16(main.title_id as u16);
    resp.write_u8(1);
    resp.write_u8(0);
    session.send_packet(&resp).await?;

    // Recalculate stats with new title bonuses
    world.set_user_ability(sid);
    Ok(())
}

/// Sub-opcode 18: AchieveCoverTitleReset — unequip cover title.
async fn handle_cover_title_reset(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    world.update_session(sid, |h| {
        h.achieve_summary.cover_title = 0;
        h.achieve_summary.cover_id = 0;
    });
    // Clear CharacterInfo cover_title so broadcasts stop showing it
    world.update_character_stats(sid, |ci| {
        ci.cover_title = 0;
    });

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::CoverTitleReset as u8);
    resp.write_u8(1);
    resp.write_u8(0);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Sub-opcode 19: AchieveSkillTitleReset — unequip skill title (clear stat bonuses).
async fn handle_skill_title_reset(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    world.update_session(sid, |h| {
        h.achieve_summary.skill_title = 0;
        h.achieve_summary.skill_id = 0;
        h.achieve_stat_bonuses = [0i16; 7];
    });

    let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
    resp.write_u8(AchieveOpcode::SkillTitleReset as u8);
    resp.write_u8(1);
    resp.write_u8(0);
    session.send_packet(&resp).await?;

    // Recalculate stats after clearing title bonuses
    world.set_user_ability(sid);
    Ok(())
}

/// Send completed achievement notifications on game entry.
/// Sniffer-verified (session 3, seq 38-39): original server sends
/// `[sub=1][achieve_id:u16][status:u8]` for each completed achievement.
/// Called from Phase 2 game entry after achievements are loaded into session.
/// Send completed achievement notifications on game entry.
/// Sniffer-verified (session 3, seq 38-39): original server sends
/// `[sub=1][achieve_id:u16][status:u8]` for each completed achievement.
pub fn send_achieve_status_on_login(world: &crate::world::WorldState, sid: crate::zone::SessionId) {
    let achieves: Vec<(u16, u8)> = world
        .with_session(sid, |h| {
            h.achieve_map
                .iter()
                .filter(|(_, info)| info.status >= AchieveStatus::Finished as u8)
                .map(|(&id, info)| (id, info.status))
                .collect()
        })
        .unwrap_or_default();

    for (achieve_id, status) in achieves {
        let mut pkt = Packet::new(Opcode::WizUserAchieve as u8);
        pkt.write_u8(AchieveOpcode::Success as u8); // sub=1
        pkt.write_u16(achieve_id);
        pkt.write_u8(status);
        world.send_to_session_owned(sid, pkt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::AchieveSummary;

    /// Build a summary response packet from the given data.
    ///
    /// Matches the `CUser::HandleUserAchieveSummary()` wire format exactly.
    fn build_summary_packet(
        summary: &AchieveSummary,
        type_counts: (u16, u16, u16, u16, u16),
    ) -> Packet {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::Summary as u8);
        resp.write_u32(summary.play_time / 60);
        resp.write_u32(summary.monster_defeat_count);
        resp.write_u32(summary.user_defeat_count);
        resp.write_u32(summary.user_death_count);
        resp.write_u32(summary.total_medal);
        resp.write_u16(summary.recent_achieve[0]);
        resp.write_u16(summary.recent_achieve[1]);
        resp.write_u16(summary.recent_achieve[2]);
        resp.write_u16(type_counts.0);
        resp.write_u16(type_counts.1);
        resp.write_u16(type_counts.2);
        resp.write_u16(type_counts.3);
        resp.write_u16(type_counts.4);
        resp
    }

    /// Build a detail show response packet.
    ///
    /// Matches `CUser::HandleUserAchieveUserDetail()` wire format.
    fn build_detail_show_packet(details: &[(u16, u8, u32)]) -> Packet {
        let mut resp = Packet::new(Opcode::WizUserAchieve as u8);
        resp.write_u8(AchieveOpcode::DetailShow as u8);
        resp.write_u16(details.len() as u16);
        for &(id, status, count) in details {
            resp.write_u16(id);
            resp.write_u8(status);
            resp.write_u32(count);
        }
        resp
    }

    const WIZ_USER_ACHIEVE: u8 = 0x99;

    #[test]
    fn test_achieve_opcode_values() {
        assert_eq!(AchieveOpcode::Error as u8, 0);
        assert_eq!(AchieveOpcode::Success as u8, 1);
        assert_eq!(AchieveOpcode::GiftRequest as u8, 2);
        assert_eq!(AchieveOpcode::DetailShow as u8, 3);
        assert_eq!(AchieveOpcode::Summary as u8, 4);
        assert_eq!(AchieveOpcode::Failed as u8, 5);
        assert_eq!(AchieveOpcode::Start as u8, 6);
        assert_eq!(AchieveOpcode::Stop as u8, 7);
        assert_eq!(AchieveOpcode::ChallengeFailed as u8, 8);
        assert_eq!(AchieveOpcode::CountScreen as u8, 9);
        assert_eq!(AchieveOpcode::CoverTitle as u8, 16);
        assert_eq!(AchieveOpcode::SkillTitle as u8, 17);
        assert_eq!(AchieveOpcode::CoverTitleReset as u8, 18);
        assert_eq!(AchieveOpcode::SkillTitleReset as u8, 19);
    }

    #[test]
    fn test_achieve_status_values() {
        assert_eq!(AchieveStatus::ChallengeIncomplete as u8, 0);
        assert_eq!(AchieveStatus::Incomplete as u8, 1);
        assert_eq!(AchieveStatus::Finished as u8, 4);
        assert_eq!(AchieveStatus::Completed as u8, 5);
    }

    #[test]
    fn test_summary_packet_format() {
        let summary = AchieveSummary {
            play_time: 7200, // 120 minutes in seconds
            monster_defeat_count: 500,
            user_defeat_count: 50,
            user_death_count: 10,
            total_medal: 300,
            recent_achieve: [100, 50, 25],
            cover_id: 0,
            cover_title: 0,
            skill_id: 0,
            skill_title: 0,
        };
        let type_counts = (5u16, 3u16, 2u16, 1u16, 0u16);
        let pkt = build_summary_packet(&summary, type_counts);

        assert_eq!(pkt.opcode, WIZ_USER_ACHIEVE);
        let r = &pkt.data;

        // sub-opcode
        assert_eq!(r[0], AchieveOpcode::Summary as u8);

        // play_time in minutes: 7200/60 = 120 = 0x78
        let play_time = u32::from_le_bytes([r[1], r[2], r[3], r[4]]);
        assert_eq!(play_time, 120);

        // monster_defeat_count
        let monster = u32::from_le_bytes([r[5], r[6], r[7], r[8]]);
        assert_eq!(monster, 500);

        // user_defeat_count
        let user_defeat = u32::from_le_bytes([r[9], r[10], r[11], r[12]]);
        assert_eq!(user_defeat, 50);

        // user_death_count
        let user_death = u32::from_le_bytes([r[13], r[14], r[15], r[16]]);
        assert_eq!(user_death, 10);

        // total_medal
        let medal = u32::from_le_bytes([r[17], r[18], r[19], r[20]]);
        assert_eq!(medal, 300);

        // recent achieves (3 x u16)
        let ra0 = u16::from_le_bytes([r[21], r[22]]);
        let ra1 = u16::from_le_bytes([r[23], r[24]]);
        let ra2 = u16::from_le_bytes([r[25], r[26]]);
        assert_eq!(ra0, 100);
        assert_eq!(ra1, 50);
        assert_eq!(ra2, 25);

        // type counts (5 x u16)
        let nc = u16::from_le_bytes([r[27], r[28]]);
        let qc = u16::from_le_bytes([r[29], r[30]]);
        let wc = u16::from_le_bytes([r[31], r[32]]);
        let ac = u16::from_le_bytes([r[33], r[34]]);
        let cc = u16::from_le_bytes([r[35], r[36]]);
        assert_eq!(nc, 5);
        assert_eq!(qc, 3);
        assert_eq!(wc, 2);
        assert_eq!(ac, 1);
        assert_eq!(cc, 0);

        // Total expected size: 1 (sub) + 5*4 (u32s) + 3*2 (recent) + 5*2 (types) = 37
        assert_eq!(r.len(), 37);
    }

    #[test]
    fn test_detail_show_packet_format() {
        let details = vec![
            (10u16, AchieveStatus::Finished as u8, 100u32),
            (20u16, AchieveStatus::Incomplete as u8, 0u32),
        ];
        let pkt = build_detail_show_packet(&details);

        assert_eq!(pkt.opcode, WIZ_USER_ACHIEVE);
        let r = &pkt.data;

        // sub-opcode
        assert_eq!(r[0], AchieveOpcode::DetailShow as u8);

        // count
        let count = u16::from_le_bytes([r[1], r[2]]);
        assert_eq!(count, 2);

        // entry 0: id=10, status=4(Finished), count=100
        let id0 = u16::from_le_bytes([r[3], r[4]]);
        assert_eq!(id0, 10);
        assert_eq!(r[5], 4);
        let cnt0 = u32::from_le_bytes([r[6], r[7], r[8], r[9]]);
        assert_eq!(cnt0, 100);

        // entry 1: id=20, status=1(Incomplete), count=0
        let id1 = u16::from_le_bytes([r[10], r[11]]);
        assert_eq!(id1, 20);
        assert_eq!(r[12], 1);
        let cnt1 = u32::from_le_bytes([r[13], r[14], r[15], r[16]]);
        assert_eq!(cnt1, 0);

        // Total: 1 (sub) + 2 (count) + 2*(2+1+4) = 17
        assert_eq!(r.len(), 17);
    }

    #[test]
    fn test_summary_packet_zero_defaults() {
        let summary = AchieveSummary::default();
        let type_counts = (0u16, 0u16, 0u16, 0u16, 0u16);
        let pkt = build_summary_packet(&summary, type_counts);

        assert_eq!(pkt.opcode, WIZ_USER_ACHIEVE);
        // All values should be zero
        // 1 sub + 5*4=20 u32 + 3*2=6 recent + 5*2=10 types = 37
        assert_eq!(pkt.data.len(), 37);
        // After the sub-opcode, all bytes should be 0 for default
        for &b in &pkt.data[1..] {
            assert_eq!(b, 0);
        }
    }

    #[test]
    fn test_detail_show_empty() {
        let pkt = build_detail_show_packet(&[]);
        assert_eq!(pkt.opcode, WIZ_USER_ACHIEVE);
        assert_eq!(pkt.data.len(), 3); // sub + count(u16)
        assert_eq!(pkt.data[0], AchieveOpcode::DetailShow as u8);
        let count = u16::from_le_bytes([pkt.data[1], pkt.data[2]]);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_gift_request_response_format() {
        // Simulate a success gift response packet
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::GiftRequest as u8);
        resp.write_u16(42); // s_index
        resp.write_u16(1); // success

        assert_eq!(resp.opcode, WIZ_USER_ACHIEVE);
        assert_eq!(resp.data[0], 2); // GiftRequest sub-opcode
        let idx = u16::from_le_bytes([resp.data[1], resp.data[2]]);
        assert_eq!(idx, 42);
        let result = u16::from_le_bytes([resp.data[3], resp.data[4]]);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_start_response_format() {
        // Simulate start success: [sub=6][u16 s_index][u16 result=1][u16 req_time]
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::Start as u8);
        resp.write_u16(100); // s_index
        resp.write_u16(1); // success
        resp.write_u16(300); // req_time seconds

        assert_eq!(resp.data[0], 6);
        let idx = u16::from_le_bytes([resp.data[1], resp.data[2]]);
        assert_eq!(idx, 100);
        let res = u16::from_le_bytes([resp.data[3], resp.data[4]]);
        assert_eq!(res, 1);
        let req_time = u16::from_le_bytes([resp.data[5], resp.data[6]]);
        assert_eq!(req_time, 300);
    }

    #[test]
    fn test_start_error_format() {
        // Error -2 (0xFFFE): main entry not found
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::Start as u8);
        resp.write_u16(0); // s_index=0
        resp.write_u16(0xFFFE); // -2

        let res = u16::from_le_bytes([resp.data[3], resp.data[4]]);
        assert_eq!(res, 0xFFFE);
    }

    #[test]
    fn test_stop_response_format() {
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::Stop as u8);
        resp.write_u16(55); // s_index
        resp.write_u16(1); // success

        assert_eq!(resp.data[0], 7);
        let idx = u16::from_le_bytes([resp.data[1], resp.data[2]]);
        assert_eq!(idx, 55);
    }

    #[test]
    fn test_cover_title_response_format() {
        // [sub=16][u16 cover_id][u16 skill_id][u8 result=1][u8 0]
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::CoverTitle as u8);
        resp.write_u16(200); // cover_id
        resp.write_u16(5); // skill_id
        resp.write_u8(1);
        resp.write_u8(0);

        assert_eq!(resp.data[0], 16);
        let cid = u16::from_le_bytes([resp.data[1], resp.data[2]]);
        assert_eq!(cid, 200);
        let sid = u16::from_le_bytes([resp.data[3], resp.data[4]]);
        assert_eq!(sid, 5);
        assert_eq!(resp.data[5], 1);
        assert_eq!(resp.data[6], 0);
    }

    #[test]
    fn test_skill_title_reset_response_format() {
        // [sub=19][u8 result=1][u8 0]
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::SkillTitleReset as u8);
        resp.write_u8(1);
        resp.write_u8(0);

        assert_eq!(resp.data[0], 19);
        assert_eq!(resp.data[1], 1);
        assert_eq!(resp.data[2], 0);
        assert_eq!(resp.data.len(), 3);
    }

    #[test]
    fn test_count_screen_response_format() {
        // [sub=9][u16 s_index][u8 1][u16 count0][u16 count1]
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::CountScreen as u8);
        resp.write_u16(100);
        resp.write_u8(1);
        resp.write_u16(5);
        resp.write_u16(3);

        assert_eq!(resp.data[0], 9);
        let idx = u16::from_le_bytes([resp.data[1], resp.data[2]]);
        assert_eq!(idx, 100);
        assert_eq!(resp.data[3], 1);
        let c0 = u16::from_le_bytes([resp.data[4], resp.data[5]]);
        assert_eq!(c0, 5);
        let c1 = u16::from_le_bytes([resp.data[6], resp.data[7]]);
        assert_eq!(c1, 3);
    }

    #[test]
    fn test_challenge_failed_response_format() {
        // [sub=8][u16 s_index][u16 result=1]
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::ChallengeFailed as u8);
        resp.write_u16(458);
        resp.write_u16(1);

        assert_eq!(resp.data[0], 8);
        let idx = u16::from_le_bytes([resp.data[1], resp.data[2]]);
        assert_eq!(idx, 458);
    }

    // ── Sprint 953: Additional coverage ──────────────────────────────

    /// WIZ_USER_ACHIEVE opcode is 0x99.
    #[test]
    fn test_achieve_opcode_hex() {
        assert_eq!(WIZ_USER_ACHIEVE, 0x99);
        assert_eq!(WIZ_USER_ACHIEVE, 153);
    }

    /// Title opcodes are in 16-19 range.
    #[test]
    fn test_achieve_title_opcode_range() {
        assert_eq!(AchieveOpcode::CoverTitle as u8, 16);
        assert_eq!(AchieveOpcode::SkillTitle as u8, 17);
        assert_eq!(AchieveOpcode::CoverTitleReset as u8, 18);
        assert_eq!(AchieveOpcode::SkillTitleReset as u8, 19);
        // Gap between Stop(7) and CoverTitle(16)
        assert!(AchieveOpcode::CoverTitle as u8 > AchieveOpcode::CountScreen as u8);
    }

    /// AchieveStatus: Completed > Finished > Incomplete.
    #[test]
    fn test_achieve_status_ordering() {
        assert!(AchieveStatus::Completed as u8 > AchieveStatus::Finished as u8);
        assert!(AchieveStatus::Finished as u8 > AchieveStatus::Incomplete as u8);
        assert!(AchieveStatus::Incomplete as u8 > AchieveStatus::ChallengeIncomplete as u8);
    }

    /// Error response: [sub=0][u16 index][i16 result].
    #[test]
    fn test_achieve_error_response() {
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::Error as u8);
        resp.write_u16(100);
        resp.write_i16(-1);
        assert_eq!(resp.data[0], 0);
        assert_eq!(resp.data.len(), 5);
    }

    /// Success response: [sub=1].
    #[test]
    fn test_achieve_success_response() {
        let mut resp = Packet::new(WIZ_USER_ACHIEVE);
        resp.write_u8(AchieveOpcode::Success as u8);
        assert_eq!(resp.data[0], 1);
        assert_eq!(resp.data.len(), 1);
    }
}
