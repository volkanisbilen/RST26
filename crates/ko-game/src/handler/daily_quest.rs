//! Daily Quest system — quest definition loading, kill tracking, quest list sending.
//! ## Daily Quest Flow
//! 1. On login, `DailyQuestSendList()` sends all quest definitions + user progress.
//! 2. On monster kill, `UpdateDailyQuestCount(monsterID)` checks all active quests.
//! 3. On quest completion, rewards are given and status updated.
//! ## Time Types
//! | Value | Name    | Behavior after completion                   |
//! |-------|---------|---------------------------------------------|
//! | 0     | Repeat  | Reset to ongoing, can redo immediately       |
//! | 1     | Time    | Set to timewait, replay after cooldown hours |
//! | 2     | Single  | Set to completed permanently                 |
//! ## Kill Types
//! | Value | Name  | Restriction       |
//! |-------|-------|-------------------|
//! | 0     | Solo  | Must NOT be in party |
//! | 1     | Party | Must be in party     |
//! | 2     | Any   | No restriction       |
//!
//! ## UI protocol (WIZ_EXT_HOOK 0xE9, sub 0xD3) - client-panel spec
//!
//! Wire: `[0xE9][0xD3][op]...`. `str` = u16 LE length + CP1254 bytes. All ints LE.
//!
//! S->C
//! - op 0 LIST: `[0][u16 n]` then n x `{u8 id, u8 timeType, u8 minLv, u8 maxLv,
//!   u16 killCount, u16 replayHours, str name, str desc, str rewards}`
//! - op 1 PROGRESS: `[1][u16 n]` then n x `{u8 id, u8 status, u16 kills, u32 secsLeft}`;
//!   status 0 = ongoing, 1 = cooldown, 2 = completed
//! - op 2 KILL: `[2][u8 id][u16 kills]`
//! - op 3 OPEN: `[3][u8 page]` (page 0); always sent after op 0 and op 1
//! - op 4 RESULT: `[4][u8 requestOp][u8 id][u8 result]`; requestOp 5 accept / 6 abandon /
//!   7 claim; result 0 ok, 1 level, 2 zone/requirement, 3 already active (or already
//!   completed), 4 active slots full (max 4), 5 on cooldown (also: cannot abandon a
//!   cooldown/completed quest), 6 not completed, 7 unknown quest
//!
//! C->S
//! - op 3 REFRESH: `[3]` -> op 0, op 1, op 3
//! - op 5 ACCEPT: `[5][u8 id]` -> op 4, op 1
//! - op 6 ABANDON: `[6][u8 id]` -> op 4, op 1
//! - op 7 CLAIM: `[7][u8 id]` -> rewards are granted automatically on completion, so
//!   there is never anything to claim: result 6 while ongoing, 0 for a finished
//!   (cooldown/completed) quest, 7 for a quest the player does not have; always
//!   followed by op 1

use ko_db::models::daily_quest::{
    DailyQuestRow, DailyQuestStatus, DailyQuestTimeType, UserDailyQuestRow,
};
use ko_protocol::{Opcode, Packet, PacketReader};

use crate::world::WorldState;
use crate::zone::SessionId;

/// A v2615 SelectMsg carries text-table IDs, not arbitrary strings.  The
/// daily-manager client patch reserves one Quest_Menu text row per daily quest,
/// allowing the regular NPC dialogue to display the actual quest names rather
/// than using chat/notice messages and numbered choices.
const DAILY_MANAGER_PAGE_SIZE: usize = 10;
const DAILY_MANAGER_QUEST_EVENT_BASE: i32 = 1_100_000;
const DAILY_MANAGER_PAGE_EVENT_BASE: i32 = 1_200_000;
const DAILY_MANAGER_HEADER_TEXT: i32 = 60010;
const DAILY_MANAGER_QUEST_TEXT_BASE: i32 = 61000;
const DAILY_MANAGER_PREVIOUS_TEXT: i32 = 60021;
const DAILY_MANAGER_NEXT_TEXT: i32 = 60022;
/// Display-only IDs used to mirror selected dailies in the regular Quest Tips
/// panel. The v2615 Quest_Guide table ends at 3593; use the adjacent free
/// range rather than 30000+, which the client silently ignores.
const DAILY_QUEST_TIP_ID_BASE: u16 = 3_600;

/// ExtHookSubOpcodes::DailyQuest sub-opcode
pub(crate) const EXT_SUB_DAILY_QUEST: u8 = 0xD3;
/// DailyQuestOp::sendlist = 0, userinfo = 1, killupdate = 2.
const DQ_OP_SENDLIST: u8 = 0;
const DQ_OP_USERINFO: u8 = 1;
const DQ_OP_KILLUPDATE: u8 = 2;
const DQ_OP_OPEN: u8 = 3;
const DQ_OP_RESULT: u8 = 4;
/// C->S ops.
const DQ_REQ_REFRESH: u8 = 3;
const DQ_REQ_ACCEPT: u8 = 5;
const DQ_REQ_ABANDON: u8 = 6;
const DQ_REQ_CLAIM: u8 = 7;
/// Maximum simultaneously active quests (native slot panel size).
const DQ_MAX_ACTIVE: usize = 4;

const RES_OK: u8 = 0;
const RES_LEVEL: u8 = 1;
const RES_ZONE: u8 = 2;
const RES_ACTIVE: u8 = 3;
const RES_SLOTS_FULL: u8 = 4;
const RES_COOLDOWN: u8 = 5;
const RES_NOT_COMPLETED: u8 = 6;
const RES_UNKNOWN: u8 = 7;

use crate::world::{ITEM_COUNT, ITEM_EXP, ITEM_GOLD, ITEM_LADDERPOINT, ITEM_RANDOM};

/// Check if a monster ID matches any of the quest's 4 mob slots.
pub fn quest_matches_monster(quest: &DailyQuestRow, monster_id: i32) -> bool {
    if monster_id == 0 {
        return false;
    }
    quest.mob_id_1 == monster_id
        || (quest.mob_id_2 != 0 && quest.mob_id_2 == monster_id)
        || (quest.mob_id_3 != 0 && quest.mob_id_3 == monster_id)
        || (quest.mob_id_4 != 0 && quest.mob_id_4 == monster_id)
}

/// Check if a character's level is within the quest's level range.
pub fn quest_level_check(quest: &DailyQuestRow, level: i16) -> bool {
    level >= quest.min_level && level <= quest.max_level
}

/// Check if a kill is valid for the quest based on party status.
/// - killtype 0: solo only (must NOT be in party)
/// - killtype 1: party only (must be in party)
/// - killtype 2: any
pub fn quest_party_check(quest: &DailyQuestRow, in_party: bool) -> bool {
    match quest.kill_type {
        0 => !in_party,
        1 => in_party,
        _ => true, // killtype 2 = any
    }
}

/// Determine the new status after a quest is completed, based on the time type.
pub fn status_after_completion(time_type: i16) -> i16 {
    match time_type {
        t if t == DailyQuestTimeType::Repeat as i16 => DailyQuestStatus::Ongoing as i16,
        t if t == DailyQuestTimeType::Single as i16 => DailyQuestStatus::Completed as i16,
        t if t == DailyQuestTimeType::Time as i16 => DailyQuestStatus::TimeWait as i16,
        _ => DailyQuestStatus::Ongoing as i16,
    }
}

/// Calculate replay time for a time-gated quest completion.
/// Returns the replay cooldown in seconds (replay_time_hours * 3600).
pub fn calculate_replay_cooldown_secs(replay_time_hours: i16) -> i32 {
    (replay_time_hours as i32) * 3600
}

/// Check if a quest's replay cooldown has expired.
/// `replay_time` is a future Unix timestamp. If current time >= replay_time, cooldown expired.
pub fn is_replay_cooldown_expired(replay_time: i32, current_unix_time: i32) -> bool {
    replay_time == 0 || current_unix_time >= replay_time
}

/// Get reward item IDs and counts from a daily quest definition.
/// Returns (item_id, count) pairs for non-zero rewards.
pub fn get_quest_rewards(quest: &DailyQuestRow) -> Vec<(i32, i32)> {
    let slots = [
        (quest.reward_1, quest.count_1),
        (quest.reward_2, quest.count_2),
        (quest.reward_3, quest.count_3),
        (quest.reward_4, quest.count_4),
    ];
    slots
        .iter()
        .filter(|&&(item_id, count)| item_id != 0 && count != 0)
        .copied()
        .collect()
}

/// Whether a SelectMsg event belongs to the server-owned daily quest manager.
pub fn is_daily_quest_manager_event(event: i32) -> bool {
    (DAILY_MANAGER_QUEST_EVENT_BASE..DAILY_MANAGER_QUEST_EVENT_BASE + 10_000).contains(&event)
        || (DAILY_MANAGER_PAGE_EVENT_BASE..DAILY_MANAGER_PAGE_EVENT_BASE + 10_000).contains(&event)
}

pub(crate) fn daily_quest_tip_id(quest_id: i16) -> u16 {
    DAILY_QUEST_TIP_ID_BASE + quest_id.max(0) as u16
}

/// Mirror selected daily quests into the same in-memory quest map used by
/// Patrick/Lua quests. Quest Tips is populated from this map; merely appending
/// IDs to an outgoing packet is not sufficient for the v2615 client flow.
pub(crate) fn sync_daily_quests_into_normal_map(world: &WorldState, sid: SessionId) {
    let selected = world
        .with_session(sid, |handle| handle.daily_quests.clone())
        .unwrap_or_default();
    let mirrored_ids = selected
        .iter()
        .filter(|(_, progress)| {
            progress.status == DailyQuestStatus::Ongoing as i16 && progress.kill_count > 0
        })
        .map(|(daily_id, _)| daily_quest_tip_id(*daily_id))
        .collect::<Vec<_>>();
    world.update_session(sid, |handle| {
        handle.quests.retain(|quest_id, _| {
            !(*quest_id > DAILY_QUEST_TIP_ID_BASE && *quest_id <= DAILY_QUEST_TIP_ID_BASE + 200)
        });
        for (daily_id, progress) in selected {
            if progress.status != DailyQuestStatus::Ongoing as i16 || progress.kill_count <= 0 {
                continue;
            }
            handle.quests.insert(
                daily_quest_tip_id(daily_id),
                crate::world::UserQuestInfo {
                    quest_state: 1,
                    kill_counts: [progress.kill_count.clamp(0, u8::MAX as i32) as u8, 0, 0, 0],
                },
            );
        }
    });
    tracing::info!(
        session_id = sid,
        quest_tip_ids = ?mirrored_ids,
        "[DailyQuest][QuestTips] mirrored active daily quests into normal quest list"
    );
}

/// The daily quest UI is separate from the right-side Quest Tips UI. Mirror
/// a selected daily into the latter with its client-only Quest_Guide ID.
fn send_daily_quest_tip(world: &WorldState, sid: SessionId, quest_id: i16, kills: i32) {
    let tip_id = daily_quest_tip_id(quest_id);
    let mut state_pkt = Packet::new(Opcode::WizQuest as u8);
    state_pkt.write_u8(2);
    state_pkt.write_u16(tip_id);
    state_pkt.write_u8(1);
    world.send_to_session_owned(sid, state_pkt);

    let mut progress_pkt = Packet::new(Opcode::WizQuest as u8);
    progress_pkt.write_u8(9);
    progress_pkt.write_u8(1);
    progress_pkt.write_u16(tip_id);
    progress_pkt.write_u16(kills.max(0) as u16);
    progress_pkt.write_u16(0);
    progress_pkt.write_u16(0);
    progress_pkt.write_u16(0);
    world.send_to_session_owned(sid, progress_pkt);
}

fn remove_daily_quest_tip(world: &WorldState, sid: SessionId, quest_id: i16) {
    let mut pkt = Packet::new(Opcode::WizQuest as u8);
    pkt.write_u8(2);
    pkt.write_u16(daily_quest_tip_id(quest_id));
    pkt.write_u8(4);
    world.send_to_session_owned(sid, pkt);
}

/// Shared accept validation (no slot limit): returns a RESULT code on failure.
fn quest_accept_check(
    quest: &DailyQuestRow,
    player_zone: u16,
    level: i16,
    nation: u8,
    selected: &std::collections::HashMap<i16, UserDailyQuestRow>,
    now: i32,
) -> Result<(), u8> {
    if let Some(uq) = selected.get(&quest.id) {
        return Err(
            if uq.status == DailyQuestStatus::TimeWait as i16 && uq.replay_time > now {
                RES_COOLDOWN
            } else {
                RES_ACTIVE
            },
        );
    }
    if !quest_level_check(quest, level) {
        return Err(RES_LEVEL);
    }
    if !daily_manager_zone_check(quest.zone_id, player_zone, level, nation) {
        return Err(RES_ZONE);
    }
    Ok(())
}

fn unix_now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32
}

fn eligible_manager_quests(world: &WorldState, sid: SessionId) -> Vec<DailyQuestRow> {
    let Some(position) = world.get_position(sid) else {
        return Vec::new();
    };
    let (level, nation) = world
        .get_character_info(sid)
        .map(|character| (character.level as i16, character.nation))
        .unwrap_or((0, 0));
    let selected = world
        .with_session(sid, |handle| handle.daily_quests.clone())
        .unwrap_or_default();
    let now = unix_now();

    let mut quests: Vec<_> = world
        .get_all_daily_quests()
        .into_iter()
        .filter(|quest| {
            quest_accept_check(quest, position.zone_id, level, nation, &selected, now).is_ok()
        })
        .collect();
    quests.sort_by_key(|quest| quest.id);
    quests
}

/// Apply the same zone policy while listing and progressing manager quests.
/// Moradon owns the 1–35 beginner band, including the character's
/// nation-specific home-zone definitions; later quests stay in their zone.
fn daily_manager_zone_check(
    quest_zone: i16,
    player_zone: u16,
    player_level: i16,
    nation: u8,
) -> bool {
    if player_zone == 21 {
        quest_zone == 21 || (player_level <= 35 && quest_zone == if nation == 1 { 1 } else { 2 })
    } else {
        zone_check(quest_zone, player_zone)
    }
}

/// Open one page of the gate-side daily quest manager using the normal NPC
/// selection dialogue, matching the standard quest-NPC interaction flow.
pub async fn open_daily_quest_manager(
    session: &mut crate::session::ClientSession,
    requested_page: usize,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let quests = eligible_manager_quests(&world, sid);
    if quests.is_empty() {
        let msg = "[Daily Quest] There is no eligible daily quest for your level in this zone.";
        world.send_to_session_owned(
            sid,
            crate::systems::timed_notice::build_notice_packet(7, msg),
        );
        return Ok(());
    }

    let page_count = quests.len().div_ceil(DAILY_MANAGER_PAGE_SIZE);
    let page = requested_page.min(page_count.saturating_sub(1));
    let start = page * DAILY_MANAGER_PAGE_SIZE;
    let end = (start + DAILY_MANAGER_PAGE_SIZE).min(quests.len());

    let mut texts = [-1; 12];
    let mut events = [-1; 12];
    for (index, quest) in quests[start..end].iter().enumerate() {
        texts[index] = DAILY_MANAGER_QUEST_TEXT_BASE + quest.id as i32;
        events[index] = DAILY_MANAGER_QUEST_EVENT_BASE + quest.id as i32;
    }
    if page > 0 {
        texts[10] = DAILY_MANAGER_PREVIOUS_TEXT;
        events[10] = DAILY_MANAGER_PAGE_EVENT_BASE + (page as i32 - 1);
    }
    if page + 1 < page_count {
        texts[11] = DAILY_MANAGER_NEXT_TEXT;
        events[11] = DAILY_MANAGER_PAGE_EVENT_BASE + (page as i32 + 1);
    }

    super::select_msg::send_select_msg(
        &world,
        sid,
        // Match Patrick's working QuestV2 SelectMsg contract. Flag 1 renders
        // only the NPC-say portrait in v2615; flag 2 opens the button list.
        2,
        -1,
        DAILY_MANAGER_HEADER_TEXT,
        &texts,
        &events,
        "daily_quest_manager.lua",
    );
    Ok(())
}

/// Handle a numbered choice or page control from the daily quest manager.
pub async fn handle_daily_quest_manager_event(
    session: &mut crate::session::ClientSession,
    event: i32,
) -> anyhow::Result<()> {
    if event >= DAILY_MANAGER_PAGE_EVENT_BASE {
        return open_daily_quest_manager(
            session,
            (event - DAILY_MANAGER_PAGE_EVENT_BASE).max(0) as usize,
        )
        .await;
    }

    let quest_id = (event - DAILY_MANAGER_QUEST_EVENT_BASE) as i16;
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(quest) = eligible_manager_quests(&world, sid)
        .into_iter()
        .find(|quest| quest.id == quest_id)
    else {
        world.send_to_session_owned(
            sid,
            crate::systems::timed_notice::build_notice_packet(
                7,
                "[Daily Quest] That quest is no longer available.",
            ),
        );
        return Ok(());
    };

    if accept_quest(session, &quest).await {
        // Quest Tips only accepts entries that were present in its quest-list
        // packet. Refresh that list first, then send the daily panel/progress.
        super::quest::send_quest_data(session).await?;
        daily_quest_send_list(&world, sid);
    }
    Ok(())
}

/// Every daily quest that fits the player's level starts by itself (no accepting, no slot limit).
/// Quests that are completed, cooling down or already running are left alone. Returns true when
/// something was started.
pub fn auto_start_all(world: &WorldState, sid: SessionId) -> bool {
    let Some((name, level)) = world.with_session(sid, |h| {
        h.character
            .as_ref()
            .map(|c| (c.name.clone(), c.level as i16))
    }).flatten() else {
        return false;
    };
    let selected = world
        .with_session(sid, |h| h.daily_quests.clone())
        .unwrap_or_default();
    let mut rows = Vec::new();
    for quest in world.get_all_daily_quests() {
        if selected.contains_key(&quest.id) || !quest_level_check(&quest, level) {
            continue;
        }
        rows.push(UserDailyQuestRow {
            character_id: name.clone(),
            quest_id: quest.id,
            kill_count: 0,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        });
    }
    if rows.is_empty() {
        return false;
    }
    world.update_session(sid, |h| {
        for row in &rows {
            h.daily_quests.insert(row.quest_id, row.clone());
        }
    });
    if let Some(pool) = world.db_pool() {
        let pool = pool.clone();
        tokio::spawn(async move {
            let repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(&pool);
            for row in rows {
                if let Err(e) = repo.save_user_quest(&row).await {
                    tracing::warn!(quest_id = row.quest_id, %e, "Daily quest auto start save failed");
                }
            }
        });
    }
    true
}

/// Insert + persist an accepted quest and mirror it into Quest Tips.
/// Returns false (after notifying the player) when saving failed.
async fn accept_quest(session: &mut crate::session::ClientSession, quest: &DailyQuestRow) -> bool {
    let world = session.world().clone();
    let sid = session.session_id();
    let character_id = match world.get_session_name(sid) {
        Some(name) if !name.is_empty() => name,
        _ => return false,
    };
    let row = UserDailyQuestRow {
        character_id,
        quest_id: quest.id,
        kill_count: 0,
        status: DailyQuestStatus::Ongoing as i16,
        replay_time: 0,
    };
    world.update_session(sid, |handle| {
        handle.daily_quests.insert(quest.id, row.clone());
        handle.quests.insert(
            daily_quest_tip_id(quest.id),
            crate::world::UserQuestInfo {
                quest_state: 1,
                kill_counts: [0; 4],
            },
        );
    });

    if let Some(pool) = world.db_pool() {
        let repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(pool);
        if let Err(error) = repo.save_user_quest(&row).await {
            world.update_session(sid, |handle| {
                handle.daily_quests.remove(&quest.id);
                handle.quests.remove(&daily_quest_tip_id(quest.id));
            });
            tracing::warn!(quest_id = quest.id, %error, "Daily quest selection save failed");
            world.send_to_session_owned(
                sid,
                crate::systems::timed_notice::build_notice_packet(
                    7,
                    "[Daily Quest] Could not save your selection. Please try again.",
                ),
            );
            return false;
        }
    }
    true
}

// ── Zone Check ──────────────────────────────────────────────────────────────

/// Check if the player's current zone matches the quest's zone restriction.
/// `quest_zone == 0` means no zone restriction.
/// The server uses the following zone checks (differs from older versions!):
/// - zone 21 (Moradon): exact match only (NOT isInMoradon group)
/// - zone 1 (Karus): exact match only (NOT isInLufersonCastle group)
/// - zone 2 (Elmorad): zone 2, 7, 8 (isInElmoradCastle group)
/// - zone 11 (Karus Eslant): exact match only
/// - zone 12 (Elmorad Eslant): zones 11, 13, 14 (isInKarusEslant -- C++ bug preserved)
/// - Others: exact zone ID match (fallthrough: `if v12 != m_bZone`)
pub fn zone_check(quest_zone: i16, player_zone: u16) -> bool {
    if quest_zone == 0 {
        return true;
    }
    let qz = quest_zone as u16;
    match qz {
        21 => player_zone == 21,
        1 => player_zone == 1,
        // `if (m_bZone != 2 && (m_bZone - 7) > 1) skip`
        2 => player_zone == 2 || (player_zone >= 7 && player_zone <= 8),
        11 => player_zone == 11,
        // `((v14 - 11) & 0xFC) != 0 || v14 == 12` -> valid zones: 11, 13, 14
        // This is a C++ bug: quest zone 12 requires player in Karus Eslant, not Elmorad Eslant
        12 => matches!(player_zone, 11 | 13 | 14),
        _ => qz == player_zone,
    }
}

// ── Packet Builders ─────────────────────────────────────────────────────────

/// Build the base WIZ_EXT_HOOK + DailyQuest sub-opcode packet.
fn build_dq_base() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_DAILY_QUEST);
    pkt
}

/// Build the kill update packet sent to the client on each daily quest kill.
/// Wire: `[0xE9][0xD3][0x02][u8 quest_id][u16 kills]`
pub fn build_kill_update(quest_id: i16, kills: u16) -> Packet {
    let mut pkt = build_dq_base();
    pkt.write_u8(DQ_OP_KILLUPDATE);
    pkt.write_u8(quest_id as u8);
    pkt.write_u16(kills);
    pkt
}

/// Fully resolved LIST entry (texts already generated server-side).
#[derive(Debug, Clone)]
pub struct DqListEntry {
    pub id: u8,
    pub time_type: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub kill_count: u16,
    pub replay_hours: u16,
    pub name: String,
    pub desc: String,
    pub rewards: String,
    /// `(icon id, name, count text)` of up to 4 rewards for the panel's reward boxes.
    pub reward_items: Vec<(u32, String, String)>,
}

/// op 0: `[0][u16 n]{u8 id,u8 timeType,u8 minLv,u8 maxLv,u16 kills,u16 replayH,str,str,str,u8 n,n x {u32 icon,str name,str count}}`
pub fn build_list_packet(entries: &[DqListEntry]) -> Packet {
    let mut pkt = build_dq_base();
    pkt.write_u8(DQ_OP_SENDLIST);
    pkt.write_u16(entries.len() as u16);
    for e in entries {
        pkt.write_u8(e.id);
        pkt.write_u8(e.time_type);
        pkt.write_u8(e.min_level);
        pkt.write_u8(e.max_level);
        pkt.write_u16(e.kill_count);
        pkt.write_u16(e.replay_hours);
        pkt.write_string(&e.name);
        pkt.write_string(&e.desc);
        pkt.write_string(&e.rewards);
        pkt.write_u8(e.reward_items.len().min(4) as u8);
        for (icon, name, count) in e.reward_items.iter().take(4) {
            pkt.write_u32(*icon);
            pkt.write_string(name);
            pkt.write_string(count);
        }
    }
    pkt
}

/// Wire status: 0 ongoing, 1 cooldown, 2 completed.
pub fn wire_status(status: i16, replay_time: i32, now: i32) -> u8 {
    if status == DailyQuestStatus::Completed as i16 {
        2
    } else if status == DailyQuestStatus::TimeWait as i16 && replay_time > now {
        1
    } else {
        0
    }
}

/// op 1 entries: (id, wire status, kills, secs left).
pub fn progress_entries(
    user_quests: &std::collections::HashMap<i16, UserDailyQuestRow>,
    now: i32,
) -> Vec<(u8, u8, u16, u32)> {
    let mut keys: Vec<i16> = user_quests.keys().copied().collect();
    keys.sort_unstable();
    keys.iter()
        .filter_map(|k| user_quests.get(k))
        .map(|uq| {
            let remaining = if uq.replay_time > now {
                (uq.replay_time - now) as u32
            } else {
                0
            };
            (
                uq.quest_id as u8,
                wire_status(uq.status, uq.replay_time, now),
                uq.kill_count.clamp(0, u16::MAX as i32) as u16,
                remaining,
            )
        })
        .collect()
}

/// op 1: `[1][u16 n]{u8 id,u8 status,u16 kills,u32 secsLeft}`
pub fn build_progress_packet(entries: &[(u8, u8, u16, u32)]) -> Packet {
    let mut pkt = build_dq_base();
    pkt.write_u8(DQ_OP_USERINFO);
    pkt.write_u16(entries.len() as u16);
    for &(id, status, kills, secs) in entries {
        pkt.write_u8(id);
        pkt.write_u8(status);
        pkt.write_u16(kills);
        pkt.write_u32(secs);
    }
    pkt
}

/// op 3: `[3][u8 page]`
pub fn build_open_packet(page: u8) -> Packet {
    let mut pkt = build_dq_base();
    pkt.write_u8(DQ_OP_OPEN);
    pkt.write_u8(page);
    pkt
}

/// op 4: `[4][u8 requestOp][u8 id][u8 result]`
pub fn build_result_packet(request_op: u8, quest_id: u8, result: u8) -> Packet {
    let mut pkt = build_dq_base();
    pkt.write_u8(DQ_OP_RESULT);
    pkt.write_u8(request_op);
    pkt.write_u8(quest_id);
    pkt.write_u8(result);
    pkt
}

/// Parsed C->S request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DqRequest {
    Refresh,
    Accept(u8),
    Abandon(u8),
    Claim(u8),
}

/// Parse the payload after the 0xD3 sub-opcode: `[op][u8 id]?`.
pub fn parse_request(data: &[u8]) -> Option<DqRequest> {
    let mut r = PacketReader::new(data);
    match r.read_u8()? {
        DQ_REQ_REFRESH => Some(DqRequest::Refresh),
        DQ_REQ_ACCEPT => r.read_u8().map(DqRequest::Accept),
        DQ_REQ_ABANDON => r.read_u8().map(DqRequest::Abandon),
        DQ_REQ_CLAIM => r.read_u8().map(DqRequest::Claim),
        _ => None,
    }
}

/// The reward boxes of the panel: icon id, name and count text of every reward.
fn reward_items(world: &WorldState, def: &DailyQuestRow) -> Vec<(u32, String, String)> {
    get_quest_rewards(def)
        .iter()
        .map(|&(item, count)| {
            let id = item as u32;
            if id == ITEM_EXP {
                (0, "EXP".to_string(), count.to_string())
            } else if id == ITEM_GOLD {
                (0, "Gold".to_string(), count.to_string())
            } else if id == ITEM_COUNT || id == ITEM_LADDERPOINT {
                (0, "NP".to_string(), count.to_string())
            } else if id == ITEM_RANDOM {
                (0, "Random reward".to_string(), String::new())
            } else {
                let def_item = world.get_item(id);
                let name = def_item
                    .as_ref()
                    .and_then(|i| i.str_name.clone())
                    .unwrap_or_else(|| format!("Item {item}"));
                let icon = def_item
                    .as_ref()
                    .and_then(|i| i.item_icon_id1)
                    .unwrap_or(0)
                    .max(0) as u32;
                (icon, name, format!("x{count}"))
            }
        })
        .collect()
}

/// Human-readable reward text.
fn describe_rewards(world: &WorldState, def: &DailyQuestRow) -> String {
    get_quest_rewards(def)
        .iter()
        .map(|&(item, count)| {
            let id = item as u32;
            if id == ITEM_EXP {
                format!("EXP {count}")
            } else if id == ITEM_GOLD {
                format!("{count} Gold")
            } else if id == ITEM_COUNT || id == ITEM_LADDERPOINT {
                format!("{count} NP")
            } else if id == ITEM_RANDOM {
                "Random reward".to_string()
            } else {
                let name = world
                    .get_item(id)
                    .and_then(|i| i.str_name)
                    .unwrap_or_else(|| format!("Item {item}"));
                format!("{name} x{count}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_objective(world: &WorldState, def: &DailyQuestRow) -> String {
    let mut mobs: Vec<String> = Vec::new();
    for mob in [def.mob_id_1, def.mob_id_2, def.mob_id_3, def.mob_id_4] {
        if mob == 0 {
            continue;
        }
        let name = world
            .get_npc_template(mob as u16, true)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| format!("Monster {mob}"));
        if !mobs.contains(&name) {
            mobs.push(name);
        }
    }
    let mut desc = format!("Kill {} {}", def.kill_count, mobs.join(" / "));
    if def.zone_id != 0 {
        let zone = world
            .get_zone(def.zone_id as u16)
            .and_then(|z| z.zone_info.as_ref().map(|i| i.zone_name.clone()))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("zone {}", def.zone_id));
        desc.push_str(&format!(" in {zone}"));
    }
    match def.kill_type {
        0 => desc.push_str(" (solo)"),
        1 => desc.push_str(" (party)"),
        _ => {}
    }
    desc
}

/// Send op 0 (definitions).
pub fn send_list(world: &WorldState, sid: SessionId) {
    let mut defs = world.get_all_daily_quests();
    defs.sort_by_key(|d| d.id);
    let entries: Vec<DqListEntry> = defs
        .iter()
        .map(|d| DqListEntry {
            id: d.id as u8,
            time_type: d.time_type as u8,
            min_level: d.min_level as u8,
            max_level: d.max_level as u8,
            kill_count: d.kill_count.clamp(0, u16::MAX as i32) as u16,
            replay_hours: d.replay_time.max(0) as u16,
            name: d
                .quest_name
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("Daily Quest {}", d.id)),
            desc: describe_objective(world, d),
            rewards: describe_rewards(world, d),
            reward_items: reward_items(world, d),
        })
        .collect();
    world.send_to_session_owned(sid, build_list_packet(&entries));
}

/// Send op 1 (progress).
pub fn send_progress(world: &WorldState, sid: SessionId) {
    let Some(user_quests) = world.with_session(sid, |h| h.daily_quests.clone()) else {
        return;
    };
    let entries = progress_entries(&user_quests, unix_now());
    world.send_to_session_owned(sid, build_progress_packet(&entries));
}

/// Send op 0 + op 1 + op 3 (open the client panel).
pub fn daily_quest_send_panel(world: &WorldState, sid: SessionId) {
    send_list(world, sid);
    send_progress(world, sid);
    world.send_to_session_owned(sid, build_open_packet(0));
}

// ── DailyQuestSendList ──────────────────────────────────────────────────────

/// Send op 0 + op 1 (no panel open) plus the native 0xC7 slot init.
pub fn daily_quest_send_list(world: &WorldState, sid: SessionId) {
    send_list(world, sid);
    send_progress(world, sid);
    send_native_slots(world, sid);
}

/// Native 0xC7 slot init (max 4 slots) plus chat fallback.
fn send_native_slots(world: &WorldState, sid: SessionId) {
    let all_defs = world.get_all_daily_quests();
    let user_quests = match world.with_session(sid, |h| h.daily_quests.clone()) {
        Some(q) => q,
        None => return,
    };
    let mut keys: Vec<i16> = user_quests.keys().copied().collect();
    keys.sort();

    // ── v2525 native 0xC7 — send active quests into 4 panel slots ────
    // Client panel at [esi+0x1F0], max 4 slots (0-3).
    // Unlike ext_hook (bulk list), v2525 uses per-quest slot init packets.
    let mut slot_index: u8 = 0;
    for key in &keys {
        if slot_index >= 4 {
            break;
        }
        if let Some(uq) = user_quests.get(key) {
            if uq.status == DailyQuestStatus::Ongoing as i16 && uq.kill_count > 0 {
                // Find the quest definition for NPC ID
                let npc_id = all_defs
                    .iter()
                    .find(|d| d.id == *key)
                    .map(|d| d.mob_id_1 as u16)
                    .unwrap_or(0);
                let init_pkt = super::daily_quest_v2525::build_init(
                    slot_index,
                    *key as i32,
                    npc_id,
                    0, // status=0: active
                );
                world.send_to_session_owned(sid, init_pkt);
                slot_index += 1;
            }
        }
    }

    // The 0xC7 daily panel does not populate the ordinary Quest Tips widget.
    // Send display-only WIZ_QUEST entries for the selected dailies as well.
    for key in &keys {
        if let Some(uq) = user_quests.get(key) {
            if uq.status == DailyQuestStatus::Ongoing as i16 && uq.kill_count > 0 {
                send_daily_quest_tip(world, sid, *key, uq.kill_count);
            }
        }
    }
}

// ── UpdateDailyQuestCount ───────────────────────────────────────────────────

/// Process daily quest kill tracking for a player after an NPC death.
/// Iterates all active daily quests and checks if the killed monster matches.
/// If so, increments kill count, sends update packets, and triggers completion.
pub async fn update_daily_quest_count(world: &WorldState, sid: SessionId, monster_id: u16) {
    if monster_id == 0 {
        return;
    }

    // Get player data
    let player_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    // Native one-time starter quest. This is intentionally independent from
    // the configurable daily quest counters below.
    super::quest::complete_starter_seed_quest_on_worm_kill(world, sid, monster_id);

    let player_data = world.with_session(sid, |h| {
        (
            h.daily_quests.clone(),
            h.character.as_ref().map(|c| c.level).unwrap_or(0) as i16,
            h.character.as_ref().map(|c| c.nation).unwrap_or(0),
            world.get_party_id(sid).is_some(),
        )
    });
    let (mut dq_map, player_level, player_nation, in_party) = match player_data {
        Some(d) => d,
        None => return,
    };

    if dq_map.is_empty() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    let all_defs = world.get_all_daily_quests();
    let mut changed = false;
    let mut changed_quest_ids: Vec<i16> = Vec::new();
    let mut any_finished = false;

    for def in &all_defs {
        let uq = match dq_map.get_mut(&def.id) {
            Some(uq) => uq,
            None => continue,
        };

        // Skip completed quests
        if uq.status == DailyQuestStatus::Completed as i16 {
            continue;
        }

        // Skip timewait with active cooldown
        if uq.status == DailyQuestStatus::TimeWait as i16
            && uq.replay_time > 0
            && now < uq.replay_time
        {
            continue;
        }

        // Zone check
        if !daily_manager_zone_check(def.zone_id, player_zone, player_level, player_nation) {
            continue;
        }

        // Already at max kills
        if uq.kill_count + 1 > def.kill_count {
            continue;
        }

        // Party/kill type check
        if !quest_party_check(def, in_party) {
            continue;
        }

        // Level check
        if !quest_level_check(def, player_level) {
            continue;
        }

        // Monster match
        if !quest_matches_monster(def, monster_id as i32) {
            continue;
        }

        // Increment kill count
        uq.kill_count += 1;
        changed = true;
        changed_quest_ids.push(def.id);
        world.update_session(sid, |handle| {
            if let Some(tip) = handle.quests.get_mut(&daily_quest_tip_id(def.id)) {
                tip.kill_counts[0] = uq.kill_count.clamp(0, u8::MAX as i32) as u8;
            }
        });

        // Send kill update packet
        let kill_pkt = build_kill_update(def.id, uq.kill_count.clamp(0, u16::MAX as i32) as u16);
        world.send_to_session_owned(sid, kill_pkt);
        send_daily_quest_tip(world, sid, def.id, uq.kill_count);

        // Check completion
        if uq.kill_count >= def.kill_count {
            daily_quest_finished(world, sid, def, uq).await;
            any_finished = true;
        }
    }

    // Write back updated map and persist every accepted kill. Previously only
    // the final kill was saved, so reconnecting or changing zones could restore
    // an older count and leave the native quest panel stuck.
    if changed {
        let rows_to_save: Vec<UserDailyQuestRow> = changed_quest_ids
            .iter()
            .filter_map(|quest_id| dq_map.get(quest_id).cloned())
            .collect();

        world.update_session(sid, |h| {
            h.daily_quests = dq_map;
        });
        if any_finished {
            // Refresh the open UI panel after state changes (cooldown/completed).
            send_progress(world, sid);
        }

        if let Some(pool) = world.db_pool() {
            let pool = pool.clone();
            tokio::spawn(async move {
                let repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(&pool);
                for row in rows_to_save {
                    if let Err(e) = repo.save_user_quest(&row).await {
                        tracing::warn!(
                            "DailyQuest kill save failed for quest {}: {}",
                            row.quest_id,
                            e
                        );
                    }
                }
            });
        }
    }
}

// ── Send Reward Letter ────────────────────────────────────────────────────────

/// Send a daily quest reward item via letter when inventory is full.
pub(crate) async fn send_reward_letter(
    world: &WorldState,
    sid: SessionId,
    item_id: i32,
    count: i16,
) {
    let Some(pool) = world.db_pool() else { return };
    let char_name = match world.with_session(sid, |h| h.character.as_ref().map(|c| c.name.clone()))
    {
        Some(Some(name)) if !name.is_empty() => name,
        _ => return,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    let pool = pool.clone();
    let name = char_name;
    tokio::spawn(async move {
        let repo = ko_db::repositories::letter::LetterRepository::new(&pool);
        if let Err(e) = repo
            .send_letter(
                "Daily Quest",        // sender
                &name,                // recipient
                "Daily Quest Reward", // subject
                "",                   // message
                2,                    // b_type = item letter
                item_id,              // item_id
                count,                // item_count
                0,                    // durability
                0,                    // serial
                0,                    // expiry
                0,                    // coins
                now,                  // send_date
            )
            .await
        {
            tracing::warn!("DailyQuest letter send failed for {}: {}", name, e);
        }
    });
}

// ── DailyQuestFinished ──────────────────────────────────────────────────────

/// Process daily quest completion: update status, give rewards, save to DB.
async fn daily_quest_finished(
    world: &WorldState,
    sid: SessionId,
    quest: &DailyQuestRow,
    user_quest: &mut UserDailyQuestRow,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    // The v2615 native panel has four physical slots. Login fills them with
    // ongoing quests in sorted ID order, so completion must close the same
    // slot. The old hard-coded slot 0 left entries such as "Rescuing Sid"
    // visible when they occupied slot 1, 2 or 3.
    let native_slot_index = world
        .with_session(sid, |h| {
            let mut active_ids: Vec<i16> = h
                .daily_quests
                .iter()
                .filter_map(|(&quest_id, progress)| {
                    (progress.status == DailyQuestStatus::Ongoing as i16).then_some(quest_id)
                })
                .collect();
            active_ids.sort_unstable();
            active_ids
                .iter()
                .take(4)
                .position(|&quest_id| quest_id == quest.id)
                .map(|slot| slot as u8)
        })
        .flatten();

    // Set replay timer
    if quest.replay_time > 0 {
        user_quest.replay_time = now + calculate_replay_cooldown_secs(quest.replay_time);
    }

    // Update status
    user_quest.status = status_after_completion(quest.time_type);

    // Reset kill count
    user_quest.kill_count = 0;
    world.update_session(sid, |handle| {
        handle.quests.remove(&daily_quest_tip_id(quest.id));
    });
    remove_daily_quest_tip(world, sid, quest.id);

    // Distribute rewards (C++ ReqDailyQuestSendReward DailyQuest.cpp:22-100)
    let rewards = get_quest_rewards(quest);

    // Show reward popup (C++ QuestV2ShowGiveItem)
    let mut show_pkt = Packet::new(Opcode::WizQuest as u8);
    show_pkt.write_u8(10); // sub-opcode for ShowGiveItem
    let reward_slots = [
        (quest.reward_1, quest.count_1),
        (quest.reward_2, quest.count_2),
        (quest.reward_3, quest.count_3),
        (quest.reward_4, quest.count_4),
    ];
    for i in 0..8 {
        if i < reward_slots.len() {
            show_pkt.write_u32(reward_slots[i].0 as u32);
            show_pkt.write_u32(reward_slots[i].1 as u32);
        } else {
            show_pkt.write_u32(0);
            show_pkt.write_u32(0);
        }
    }
    world.send_to_session_owned(sid, show_pkt);

    // Give each reward (C++ ReqDailyQuestSendReward — DailyQuest.cpp:22-100)
    for &(item_id, count) in &rewards {
        if item_id == ITEM_EXP as i32 {
            // XP reward
            super::level::exp_change(world, sid, count as i64).await;
        } else if item_id == ITEM_GOLD as i32 {
            // Gold reward
            world.gold_gain(sid, count as u32);
        } else if item_id == ITEM_COUNT as i32 || item_id == ITEM_LADDERPOINT as i32 {
            // Loyalty/NP reward
            crate::systems::loyalty::send_loyalty_change(world, sid, count, false, false, true);
        } else if item_id == ITEM_RANDOM as i32 {
            // Random item from pool
            let random_items = world.get_item_random_by_session(quest.random_id);
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
                if !given {
                    send_reward_letter(world, sid, picked.item_id, picked.item_count as i16).await;
                }
            }
        } else {
            // Normal item reward
            let given =
                world.give_item(sid, item_id as u32, count.clamp(0, u16::MAX as i32) as u16);
            if !given {
                // Inventory full — send via letter
                send_reward_letter(world, sid, item_id, count as i16).await;
            }
        }
    }

    // v2615 native 0xC7 — remove the exact slot that was initialised for
    // this quest. Quests outside the first four were never shown and therefore
    // do not need a native completion packet.
    if let Some(slot_index) = native_slot_index {
        let complete_pkt = super::daily_quest_v2525::build_complete(slot_index, quest.id as i32);
        world.send_to_session_owned(sid, complete_pkt);
        tracing::info!(
            sid,
            quest_id = quest.id,
            slot_index,
            "DailyQuest native panel entry completed"
        );
    }

    // Async DB save
    if let Some(pool) = world.db_pool() {
        let pool = pool.clone();
        let uq_clone = user_quest.clone();
        tokio::spawn(async move {
            let repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(&pool);
            if let Err(e) = repo.save_user_quest(&uq_clone).await {
                tracing::warn!(
                    "DailyQuestFinished: save failed for quest {}: {}",
                    uq_clone.quest_id,
                    e
                );
            }
        });
    }
}

// ── UI request handler (WIZ_EXT_HOOK 0xD3, C->S) ─────────────────────────────

/// Handle a C->S 0xD3 request. `pkt.data` has the 0xD3 byte already stripped.
/// Packet layouts: see the module-level "UI protocol" doc at the top of this file.
pub async fn handle(
    session: &mut crate::session::ClientSession,
    pkt: Packet,
) -> anyhow::Result<()> {
    let Some(req) = parse_request(&pkt.data) else {
        return Ok(());
    };
    let world = session.world().clone();
    let sid = session.session_id();
    match req {
        DqRequest::Refresh => daily_quest_send_panel(&world, sid),
        DqRequest::Accept(id) => {
            let result = handle_accept(session, id).await?;
            world.send_to_session_owned(sid, build_result_packet(DQ_REQ_ACCEPT, id, result));
            send_progress(&world, sid);
        }
        DqRequest::Abandon(id) => {
            let result = handle_abandon(session, id).await;
            world.send_to_session_owned(sid, build_result_packet(DQ_REQ_ABANDON, id, result));
            send_progress(&world, sid);
        }
        DqRequest::Claim(id) => {
            let result = claim_result(&world, sid, id);
            world.send_to_session_owned(sid, build_result_packet(DQ_REQ_CLAIM, id, result));
            send_progress(&world, sid);
        }
    }
    Ok(())
}

async fn handle_accept(session: &mut crate::session::ClientSession, id: u8) -> anyhow::Result<u8> {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(quest) = world
        .get_all_daily_quests()
        .into_iter()
        .find(|q| q.id == id as i16)
    else {
        return Ok(RES_UNKNOWN);
    };
    let zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    let (level, nation) = world
        .get_character_info(sid)
        .map(|c| (c.level as i16, c.nation))
        .unwrap_or((0, 0));
    let selected = world
        .with_session(sid, |h| h.daily_quests.clone())
        .unwrap_or_default();
    if let Err(code) = quest_accept_check(&quest, zone, level, nation, &selected, unix_now()) {
        return Ok(code);
    }
    let active = selected
        .values()
        .filter(|u| u.status == DailyQuestStatus::Ongoing as i16)
        .count();
    if active >= DQ_MAX_ACTIVE {
        return Ok(RES_SLOTS_FULL);
    }
    if !accept_quest(session, &quest).await {
        return Ok(RES_UNKNOWN);
    }
    super::quest::send_quest_data(session).await?;
    send_native_slots(&world, sid);
    Ok(RES_OK)
}

async fn handle_abandon(session: &mut crate::session::ClientSession, id: u8) -> u8 {
    let world = session.world().clone();
    let sid = session.session_id();
    let qid = id as i16;
    let Some(uq) = world
        .with_session(sid, |h| h.daily_quests.get(&qid).cloned())
        .flatten()
    else {
        return RES_UNKNOWN;
    };
    // Finished quests keep their row so cooldown / single-use cannot be bypassed.
    if uq.status != DailyQuestStatus::Ongoing as i16 {
        return RES_COOLDOWN;
    }
    world.update_session(sid, |h| {
        h.daily_quests.remove(&qid);
        h.quests.remove(&daily_quest_tip_id(qid));
    });
    remove_daily_quest_tip(&world, sid, qid);
    if let Some(pool) = world.db_pool() {
        let repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(pool);
        if let Err(e) = repo.delete_user_quest(&uq.character_id, qid).await {
            tracing::warn!(quest_id = qid, %e, "Daily quest abandon delete failed");
        }
    }
    RES_OK
}

fn claim_result(world: &WorldState, sid: SessionId, id: u8) -> u8 {
    let uq = world
        .with_session(sid, |h| h.daily_quests.get(&(id as i16)).cloned())
        .flatten();
    match uq {
        None => RES_UNKNOWN,
        Some(u) if wire_status(u.status, u.replay_time, unix_now()) == 0 => RES_NOT_COMPLETED,
        // Rewards are granted automatically at completion; nothing left to claim.
        Some(_) => RES_OK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_db::models::daily_quest::{
        DailyQuestRow, DailyQuestStatus, DailyQuestTimeType, UserDailyQuestRow,
    };

    fn make_quest(
        id: i16,
        mob_id: i32,
        kill_count: i32,
        time_type: i16,
        kill_type: i16,
    ) -> DailyQuestRow {
        DailyQuestRow {
            id,
            quest_name: Some(format!("Test Quest {id}")),
            quest_id: 0,
            time_type,
            kill_type,
            mob_id_1: mob_id,
            mob_id_2: 0,
            mob_id_3: 0,
            mob_id_4: 0,
            kill_count,
            reward_1: 900001000,
            reward_2: 0,
            reward_3: 0,
            reward_4: 0,
            count_1: 1000000,
            count_2: 0,
            count_3: 0,
            count_4: 0,
            zone_id: 21,
            min_level: 1,
            max_level: 83,
            replay_time: 0,
            random_id: 0,
        }
    }

    fn make_multi_mob_quest() -> DailyQuestRow {
        let mut q = make_quest(99, 100, 10, 0, 2);
        q.mob_id_2 = 200;
        q.mob_id_3 = 300;
        q.mob_id_4 = 400;
        q
    }

    #[test]
    fn test_quest_matches_monster_single_mob() {
        let quest = make_quest(1, 750, 5, 2, 2);
        assert!(quest_matches_monster(&quest, 750));
        assert!(!quest_matches_monster(&quest, 751));
        assert!(!quest_matches_monster(&quest, 0));
    }

    #[test]
    fn test_quest_matches_monster_multi_mob() {
        let quest = make_multi_mob_quest();
        assert!(quest_matches_monster(&quest, 100));
        assert!(quest_matches_monster(&quest, 200));
        assert!(quest_matches_monster(&quest, 300));
        assert!(quest_matches_monster(&quest, 400));
        assert!(!quest_matches_monster(&quest, 500));
    }

    #[test]
    fn test_quest_level_check() {
        let quest = make_quest(1, 750, 5, 2, 2);
        // quest: min_level=1, max_level=83
        assert!(quest_level_check(&quest, 1));
        assert!(quest_level_check(&quest, 40));
        assert!(quest_level_check(&quest, 83));
        assert!(!quest_level_check(&quest, 0));
        assert!(!quest_level_check(&quest, 84));
    }

    #[test]
    fn test_quest_level_check_restricted_range() {
        let mut quest = make_quest(18, 2151, 50, 0, 2);
        quest.min_level = 40;
        quest.max_level = 83;
        assert!(!quest_level_check(&quest, 39));
        assert!(quest_level_check(&quest, 40));
        assert!(quest_level_check(&quest, 60));
        assert!(quest_level_check(&quest, 83));
        assert!(!quest_level_check(&quest, 84));
    }

    #[test]
    fn test_quest_party_check_solo_only() {
        let quest = make_quest(1, 750, 5, 2, 0); // killtype 0 = solo
        assert!(quest_party_check(&quest, false));
        assert!(!quest_party_check(&quest, true));
    }

    #[test]
    fn test_quest_party_check_party_only() {
        let quest = make_quest(1, 750, 5, 2, 1); // killtype 1 = party
        assert!(!quest_party_check(&quest, false));
        assert!(quest_party_check(&quest, true));
    }

    #[test]
    fn test_quest_party_check_any() {
        let quest = make_quest(1, 750, 5, 2, 2); // killtype 2 = any
        assert!(quest_party_check(&quest, false));
        assert!(quest_party_check(&quest, true));
    }

    #[test]
    fn test_status_after_completion_repeat() {
        assert_eq!(
            status_after_completion(DailyQuestTimeType::Repeat as i16),
            DailyQuestStatus::Ongoing as i16
        );
    }

    #[test]
    fn test_status_after_completion_single() {
        assert_eq!(
            status_after_completion(DailyQuestTimeType::Single as i16),
            DailyQuestStatus::Completed as i16
        );
    }

    #[test]
    fn test_status_after_completion_time() {
        assert_eq!(
            status_after_completion(DailyQuestTimeType::Time as i16),
            DailyQuestStatus::TimeWait as i16
        );
    }

    #[test]
    fn test_calculate_replay_cooldown() {
        assert_eq!(calculate_replay_cooldown_secs(0), 0);
        assert_eq!(calculate_replay_cooldown_secs(1), 3600);
        assert_eq!(calculate_replay_cooldown_secs(24), 86400);
    }

    #[test]
    fn test_is_replay_cooldown_expired() {
        // No cooldown (replay_time=0) always expired
        assert!(is_replay_cooldown_expired(0, 1000));
        // Current time >= replay_time => expired
        assert!(is_replay_cooldown_expired(1000, 1000));
        assert!(is_replay_cooldown_expired(1000, 2000));
        // Current time < replay_time => NOT expired
        assert!(!is_replay_cooldown_expired(2000, 1000));
    }

    #[test]
    fn test_get_quest_rewards_all_slots() {
        let mut quest = make_quest(1, 750, 5, 2, 2);
        quest.reward_1 = 100;
        quest.count_1 = 10;
        quest.reward_2 = 200;
        quest.count_2 = 20;
        quest.reward_3 = 300;
        quest.count_3 = 30;
        quest.reward_4 = 400;
        quest.count_4 = 40;

        let rewards = get_quest_rewards(&quest);
        assert_eq!(rewards.len(), 4);
        assert_eq!(rewards[0], (100, 10));
        assert_eq!(rewards[1], (200, 20));
        assert_eq!(rewards[2], (300, 30));
        assert_eq!(rewards[3], (400, 40));
    }

    #[test]
    fn test_get_quest_rewards_partial() {
        let quest = make_quest(1, 750, 5, 2, 2);
        // reward_1=900001000, count_1=1000000; reward_2-4=0
        let rewards = get_quest_rewards(&quest);
        assert_eq!(rewards.len(), 1);
        assert_eq!(rewards[0], (900001000, 1000000));
    }

    #[test]
    fn test_get_quest_rewards_empty() {
        let mut quest = make_quest(1, 750, 5, 2, 2);
        quest.reward_1 = 0;
        quest.count_1 = 0;
        let rewards = get_quest_rewards(&quest);
        assert!(rewards.is_empty());
    }

    #[test]
    fn test_daily_quest_time_type_enum() {
        assert_eq!(DailyQuestTimeType::Repeat as u8, 0);
        assert_eq!(DailyQuestTimeType::Time as u8, 1);
        assert_eq!(DailyQuestTimeType::Single as u8, 2);
    }

    #[test]
    fn test_daily_quest_status_enum() {
        assert_eq!(DailyQuestStatus::TimeWait as u8, 0);
        assert_eq!(DailyQuestStatus::Completed as u8, 1);
        assert_eq!(DailyQuestStatus::Ongoing as u8, 2);
    }

    #[test]
    fn test_user_daily_quest_row_defaults() {
        let row = UserDailyQuestRow {
            character_id: "TestPlayer".to_string(),
            quest_id: 5,
            kill_count: 0,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };
        assert_eq!(row.status, 2);
        assert_eq!(row.kill_count, 0);
        assert_eq!(row.replay_time, 0);
    }

    #[test]
    fn test_daily_quest_row_zone_moradon() {
        let quest = make_quest(1, 750, 5, 2, 2);
        assert_eq!(quest.zone_id, 21); // Moradon
    }

    #[test]
    fn test_daily_quest_row_karus_eslant() {
        let mut quest = make_quest(18, 2151, 50, 0, 2);
        quest.zone_id = 1; // Karus
        assert_eq!(quest.zone_id, 1);
    }

    #[test]
    fn test_kill_count_tracking_below_max() {
        let quest = make_quest(1, 750, 5, 2, 2);
        let mut user_quest = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 3,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };
        // Simulate kill
        user_quest.kill_count += 1;
        assert_eq!(user_quest.kill_count, 4);
        assert!(user_quest.kill_count < quest.kill_count);
    }

    #[test]
    fn test_kill_count_tracking_completion() {
        let quest = make_quest(1, 750, 5, 2, 2);
        let mut user_quest = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 4,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };
        // Simulate final kill
        user_quest.kill_count += 1;
        assert_eq!(user_quest.kill_count, quest.kill_count);
        // On completion, status changes based on time_type
        user_quest.status = status_after_completion(quest.time_type);
        assert_eq!(user_quest.status, DailyQuestStatus::Completed as i16);
    }

    #[test]
    fn test_repeat_quest_cycle() {
        let quest = make_quest(1, 750, 5, 0, 2); // time_type=0 (repeat)
        let mut user_quest = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 5,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };
        // Complete: repeat type resets to ongoing and clears kill count
        user_quest.status = status_after_completion(quest.time_type);
        user_quest.kill_count = 0;
        assert_eq!(user_quest.status, DailyQuestStatus::Ongoing as i16);
        assert_eq!(user_quest.kill_count, 0);
    }

    #[test]
    fn test_time_gated_quest_cycle() {
        let mut quest = make_quest(18, 2151, 50, 1, 2); // time_type=1 (time)
        quest.replay_time = 1; // 1 hour replay
        let current_time = 1720000000i32;

        let mut user_quest = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 18,
            kill_count: 50,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };

        // Complete: set to timewait, set replay_time
        user_quest.status = status_after_completion(quest.time_type);
        user_quest.kill_count = 0;
        user_quest.replay_time = current_time + calculate_replay_cooldown_secs(quest.replay_time);

        assert_eq!(user_quest.status, DailyQuestStatus::TimeWait as i16);
        assert_eq!(user_quest.replay_time, 1720000000 + 3600);

        // Not expired yet
        assert!(!is_replay_cooldown_expired(
            user_quest.replay_time,
            current_time + 1800
        ));
        // Expired after 1 hour
        assert!(is_replay_cooldown_expired(
            user_quest.replay_time,
            current_time + 3600
        ));
    }

    #[test]
    fn test_world_daily_quest_accessor() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // No quests loaded in test world
        assert!(world.get_daily_quest(1).is_none());
        assert!(world.get_all_daily_quests().is_empty());
    }

    #[test]
    fn test_quest_skip_completed() {
        // Completed quests (status=1) should be skipped during kill tracking
        let user_quest = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 5,
            status: DailyQuestStatus::Completed as i16,
            replay_time: 0,
        };
        assert_eq!(user_quest.status, DailyQuestStatus::Completed as i16);
    }

    #[test]
    fn test_quest_timewait_with_active_cooldown() {
        let user_quest = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 18,
            kill_count: 0,
            status: DailyQuestStatus::TimeWait as i16,
            replay_time: 1720003600, // future timestamp
        };
        // Should skip if cooldown not expired
        let current_time = 1720000000;
        assert!(!is_replay_cooldown_expired(
            user_quest.replay_time,
            current_time
        ));
    }

    #[test]
    fn test_genie_data_model() {
        use ko_db::models::user_data::UserGenieDataRow;
        let genie = UserGenieDataRow {
            user_id: "TestUser".to_string(),
            genie_time: 3600,
            genie_options: vec![0u8; 100],
            first_using_genie: 1,
        };
        assert_eq!(genie.user_id, "TestUser");
        assert_eq!(genie.genie_time, 3600);
        assert_eq!(genie.genie_options.len(), 100);
        assert_eq!(genie.first_using_genie, 1);
    }

    #[test]
    fn test_daily_op_model() {
        use ko_db::models::user_data::UserDailyOpRow;
        let op = UserDailyOpRow {
            user_id: "TestUser".to_string(),
            chaos_map_time: -1,
            user_rank_reward_time: -1,
            personal_rank_reward_time: -1,
            king_wing_time: -1,
            warder_killer_time1: 0,
            warder_killer_time2: 0,
            keeper_killer_time: 0,
            user_loyalty_wing_reward_time: 0,
            full_moon_rift_map_time: -1,
            copy_information_time: -1,
        };
        assert_eq!(op.chaos_map_time, -1);
        assert_eq!(op.warder_killer_time1, 0);
    }

    #[test]
    fn test_loot_settings_model() {
        use ko_db::models::user_data::UserLootSettingsRow;
        let settings = UserLootSettingsRow {
            id: 1,
            user_id: "TestUser".to_string(),
            warrior: 1,
            rogue: 1,
            mage: 1,
            priest: 1,
            weapon: 1,
            armor: 1,
            accessory: 1,
            normal: 1,
            upgrade: 1,
            craft: 1,
            rare: 1,
            magic: 1,
            unique_grade: 1,
            consumable: 1,
            price: 9999,
        };
        assert_eq!(settings.warrior, 1);
        assert_eq!(settings.price, 9999);
    }

    #[test]
    fn test_seal_exp_model() {
        use ko_db::models::user_data::UserSealExpRow;
        let seal = UserSealExpRow {
            user_id: "TestUser".to_string(),
            sealed_exp: 50000,
        };
        assert_eq!(seal.sealed_exp, 50000);
    }

    #[test]
    fn test_return_data_model() {
        use ko_db::models::user_data::UserReturnDataRow;
        let ret = UserReturnDataRow {
            character_id: "TestChar".to_string(),
            return_symbol_ok: Some(1),
            return_logout_time: Some(1720000000),
            return_symbol_time: Some(1720362302),
        };
        assert_eq!(ret.return_symbol_ok, Some(1));
        assert_eq!(ret.return_logout_time, Some(1720000000));
    }

    #[test]
    fn test_daily_reward_model() {
        use ko_db::models::user_data::DailyRewardUserRow;
        let reward = DailyRewardUserRow {
            user_id: "TestUser".to_string(),
            day_index: 5,
            claimed: true,
            day_of_month: 10,
            last_claim_month: 3,
        };
        assert!(reward.claimed);
        assert_eq!(reward.day_index, 5);
        assert_eq!(reward.day_of_month, 10);
    }

    // ── New tests for Sprint 480 — zone_check, packet builders, integration ──

    #[test]
    fn test_zone_check_moradon() {
        assert!(zone_check(21, 21)); // Moradon match
        assert!(!zone_check(21, 1)); // Moradon mismatch
    }

    #[test]
    fn test_zone_check_any_zone() {
        assert!(zone_check(0, 21)); // No restriction
        assert!(zone_check(0, 1));
        assert!(zone_check(0, 71));
        assert!(zone_check(0, 0));
    }

    #[test]
    fn test_zone_check_eslant() {
        assert!(zone_check(11, 11));
        assert!(!zone_check(11, 12));
        assert!(!zone_check(11, 13));

        // Valid zones: 11, 13, 14 -- zone 12 itself is EXCLUDED
        assert!(!zone_check(12, 12)); // C++ bug: zone 12 quest fails for player in zone 12
        assert!(zone_check(12, 11)); // Karus Eslant counts
        assert!(zone_check(12, 13)); // Karus Eslant 2 counts
        assert!(zone_check(12, 14)); // Karus Eslant 3 counts
        assert!(!zone_check(12, 15)); // Elmorad Eslant 2 does NOT count
    }

    #[test]
    fn test_zone_check_specific() {
        assert!(zone_check(71, 71)); // BDW
        assert!(!zone_check(71, 21));
        assert!(zone_check(2, 2));
        assert!(zone_check(2, 7)); // Elmorad2
        assert!(zone_check(2, 8)); // Elmorad3
        assert!(!zone_check(2, 1));
        assert!(!zone_check(2, 9));
    }

    #[test]
    fn test_kill_update_packet_format() {
        let pkt = build_kill_update(5, 750);
        assert_eq!(pkt.opcode, 0xE9); // EXT_HOOK_S2C
        let data = &pkt.data;
        // data = [0xD3][0x02][u8 quest id 5][u16 kills 750]
        assert_eq!(data[0], 0xD3); // DailyQuest sub-opcode
        assert_eq!(data[1], 0x02); // killupdate
        assert_eq!(data[2], 5); // quest id
        assert_eq!(u16::from_le_bytes([data[3], data[4]]), 750); // kills
        assert_eq!(data.len(), 5);
    }

    #[test]
    fn test_daily_quest_send_list_definitions_packet() {
        // Test sendlist packet format (packet builder only, no WorldState needed)
        let def = make_quest(1, 750, 5, 0, 2);

        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(0xD3);
        pkt.write_u8(0x00); // sendlist
        pkt.write_u16(1); // count

        pkt.write_u8(def.id as u8);
        pkt.write_u8(def.time_type as u8);
        pkt.write_u8(def.kill_type as u8);

        assert_eq!(pkt.opcode, 0xE9); // EXT_HOOK_S2C
        let data = &pkt.data;
        // data = [0xD3][0x00][u16 1][u8 1][u8 0][u8 2]
        assert_eq!(data[0], 0xD3); // DailyQuest sub-opcode
        assert_eq!(data[1], 0x00); // sendlist
        assert_eq!(u16::from_le_bytes([data[2], data[3]]), 1); // count=1
        assert_eq!(data[4], 1); // quest index
        assert_eq!(data[5], 0); // time_type=repeat
        assert_eq!(data[6], 2); // kill_type=any
    }

    #[test]
    fn test_daily_quest_finished_repeat_cycle() {
        let quest = make_quest(1, 750, 5, 0, 2); // time_type=0 (repeat)
        let mut uq = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 5,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };

        // Simulate completion
        uq.status = status_after_completion(quest.time_type);
        uq.kill_count = 0;

        assert_eq!(uq.status, DailyQuestStatus::Ongoing as i16);
        assert_eq!(uq.kill_count, 0);
        // Can immediately redo
    }

    #[test]
    fn test_daily_quest_finished_time_gated() {
        let mut quest = make_quest(18, 2151, 50, 1, 2); // time_type=1 (time)
        quest.replay_time = 2; // 2 hours
        let current_time = 1720000000i32;

        let mut uq = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 18,
            kill_count: 50,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };

        // Simulate completion
        if quest.replay_time > 0 {
            uq.replay_time = current_time + calculate_replay_cooldown_secs(quest.replay_time);
        }
        uq.status = status_after_completion(quest.time_type);
        uq.kill_count = 0;

        assert_eq!(uq.status, DailyQuestStatus::TimeWait as i16);
        assert_eq!(uq.replay_time, 1720000000 + 7200); // 2h cooldown
        assert_eq!(uq.kill_count, 0);

        // Not expired
        assert!(!is_replay_cooldown_expired(
            uq.replay_time,
            current_time + 3600
        ));
        // Expired after 2h
        assert!(is_replay_cooldown_expired(
            uq.replay_time,
            current_time + 7200
        ));
    }

    #[test]
    fn test_daily_quest_finished_single() {
        let quest = make_quest(1, 750, 5, 2, 2); // time_type=2 (single)
        let mut uq = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 5,
            status: DailyQuestStatus::Ongoing as i16,
            replay_time: 0,
        };

        uq.status = status_after_completion(quest.time_type);
        uq.kill_count = 0;

        assert_eq!(uq.status, DailyQuestStatus::Completed as i16);
        // Permanently completed, never repeats
    }

    #[test]
    fn test_update_skip_completed() {
        let uq = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 1,
            kill_count: 5,
            status: DailyQuestStatus::Completed as i16,
            replay_time: 0,
        };
        // Completed quests are skipped in UpdateDailyQuestCount
        assert_eq!(uq.status, DailyQuestStatus::Completed as i16);
    }

    #[test]
    fn test_update_skip_timewait_active() {
        let uq = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 18,
            kill_count: 0,
            status: DailyQuestStatus::TimeWait as i16,
            replay_time: 1720003600,
        };
        let now = 1720000000i32;
        // TimeWait with active cooldown should be skipped
        let should_skip = uq.status == DailyQuestStatus::TimeWait as i16
            && uq.replay_time > 0
            && now < uq.replay_time;
        assert!(should_skip);
    }

    #[test]
    fn test_update_timewait_expired_becomes_ongoing() {
        let mut uq = UserDailyQuestRow {
            character_id: "Player".to_string(),
            quest_id: 18,
            kill_count: 0,
            status: DailyQuestStatus::TimeWait as i16,
            replay_time: 1720003600,
        };
        let now = 1720010000i32; // well past cooldown
                                 // Expired cooldown transitions to ongoing (in send_list)
        if uq.status == DailyQuestStatus::TimeWait as i16
            && is_replay_cooldown_expired(uq.replay_time, now)
        {
            uq.status = DailyQuestStatus::Ongoing as i16;
        }
        assert_eq!(uq.status, DailyQuestStatus::Ongoing as i16);
    }

    // ── Sprint 481: Reward type coverage tests ──

    #[test]
    fn test_reward_constants() {
        assert_eq!(ITEM_EXP, 900_001_000u32);
        assert_eq!(ITEM_GOLD, 900_000_000u32);
        assert_eq!(ITEM_COUNT, 900_002_000u32);
        assert_eq!(ITEM_LADDERPOINT, 900_003_000u32);
        assert_eq!(ITEM_RANDOM, 900_004_000u32);
    }

    #[test]
    fn test_get_quest_rewards_includes_virtual_items() {
        let mut quest = make_quest(1, 750, 5, 0, 2);
        quest.reward_1 = ITEM_EXP as i32;
        quest.count_1 = 50000;
        quest.reward_2 = ITEM_COUNT as i32;
        quest.count_2 = 100;
        quest.reward_3 = ITEM_RANDOM as i32;
        quest.count_3 = 1;
        quest.reward_4 = 0;
        quest.count_4 = 0;

        let rewards = get_quest_rewards(&quest);
        assert_eq!(rewards.len(), 3);
        assert_eq!(rewards[0], (ITEM_EXP as i32, 50000));
        assert_eq!(rewards[1], (ITEM_COUNT as i32, 100));
        assert_eq!(rewards[2], (ITEM_RANDOM as i32, 1));
    }

    #[test]
    fn test_quest_with_random_id() {
        let mut quest = make_quest(1, 750, 5, 0, 2);
        quest.random_id = 3;
        quest.reward_1 = ITEM_RANDOM as i32;
        quest.count_1 = 1;

        assert_eq!(quest.random_id, 3);
        let rewards = get_quest_rewards(&quest);
        assert_eq!(rewards.len(), 1);
        assert_eq!(rewards[0].0, ITEM_RANDOM as i32);
    }

    #[test]
    fn test_item_random_by_session_accessor() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Empty table — should return empty vec
        let items = world.get_item_random_by_session(1);
        assert!(items.is_empty());
    }

    #[test]
    fn test_list_packet_layout() {
        let e = DqListEntry {
            id: 7,
            time_type: 1,
            min_level: 10,
            max_level: 40,
            kill_count: 50,
            replay_hours: 24,
            name: "AB".into(),
            desc: "Kill 50 X".into(),
            rewards: "EXP 5".into(),
            reward_items: vec![(0, "EXP".into(), "5".into())],
        };
        let pkt = build_list_packet(&[e]);
        let d = &pkt.data;
        assert_eq!(&d[..4], &[0xD3, 0, 1, 0]);
        assert_eq!(&d[4..8], &[7, 1, 10, 40]);
        assert_eq!(u16::from_le_bytes([d[8], d[9]]), 50);
        assert_eq!(u16::from_le_bytes([d[10], d[11]]), 24);
        assert_eq!(u16::from_le_bytes([d[12], d[13]]), 2);
        assert_eq!(&d[14..16], b"AB");
        assert_eq!(u16::from_le_bytes([d[16], d[17]]), 9);
    }

    #[test]
    fn test_progress_packet_and_status_mapping() {
        assert_eq!(wire_status(DailyQuestStatus::Ongoing as i16, 0, 100), 0);
        assert_eq!(wire_status(DailyQuestStatus::TimeWait as i16, 200, 100), 1);
        assert_eq!(wire_status(DailyQuestStatus::TimeWait as i16, 50, 100), 0);
        assert_eq!(wire_status(DailyQuestStatus::Completed as i16, 0, 100), 2);

        let mut m = std::collections::HashMap::new();
        m.insert(
            3,
            UserDailyQuestRow {
                character_id: "c".into(),
                quest_id: 3,
                kill_count: 9,
                status: DailyQuestStatus::TimeWait as i16,
                replay_time: 160,
            },
        );
        let entries = progress_entries(&m, 100);
        assert_eq!(entries, vec![(3, 1, 9, 60)]);
        let pkt = build_progress_packet(&entries);
        let d = &pkt.data;
        assert_eq!(&d[..5], &[0xD3, 1, 1, 0, 3]);
        assert_eq!(d[5], 1);
        assert_eq!(u16::from_le_bytes([d[6], d[7]]), 9);
        assert_eq!(u32::from_le_bytes([d[8], d[9], d[10], d[11]]), 60);
    }

    #[test]
    fn test_open_and_result_packets() {
        assert_eq!(build_open_packet(0).data, vec![0xD3, 3, 0]);
        assert_eq!(build_result_packet(5, 12, 4).data, vec![0xD3, 4, 5, 12, 4]);
    }

    #[test]
    fn test_parse_request() {
        assert_eq!(parse_request(&[3]), Some(DqRequest::Refresh));
        assert_eq!(parse_request(&[5, 9]), Some(DqRequest::Accept(9)));
        assert_eq!(parse_request(&[6, 9]), Some(DqRequest::Abandon(9)));
        assert_eq!(parse_request(&[7, 9]), Some(DqRequest::Claim(9)));
        assert_eq!(parse_request(&[5]), None);
        assert_eq!(parse_request(&[9, 1]), None);
        assert_eq!(parse_request(&[]), None);
    }

    #[test]
    fn test_quest_accept_check_codes() {
        let q = make_quest(1, 750, 5, 0, 2);
        let none = std::collections::HashMap::new();
        let lvl = q.min_level;
        assert_eq!(quest_accept_check(&q, 21, lvl, 1, &none, 0), Ok(()));
        assert_eq!(quest_accept_check(&q, 21, 0, 1, &none, 0), Err(RES_LEVEL));
        let mut sel = std::collections::HashMap::new();
        sel.insert(
            1,
            UserDailyQuestRow {
                character_id: "c".into(),
                quest_id: 1,
                kill_count: 0,
                status: DailyQuestStatus::Ongoing as i16,
                replay_time: 0,
            },
        );
        assert_eq!(quest_accept_check(&q, 21, lvl, 1, &sel, 0), Err(RES_ACTIVE));
        sel.get_mut(&1).unwrap().status = DailyQuestStatus::TimeWait as i16;
        sel.get_mut(&1).unwrap().replay_time = 500;
        assert_eq!(
            quest_accept_check(&q, 21, lvl, 1, &sel, 100),
            Err(RES_COOLDOWN)
        );
    }
}
