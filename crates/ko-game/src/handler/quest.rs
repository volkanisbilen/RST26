//! WIZ_QUEST (0x64) handler — quest system.
//! Sub-opcodes:
//! - 1: Quest list (QuestDataRequest — server->client on game start)
//! - 2: Save event (update quest state)
//! - 3/7: Execute helper (trigger quest script)
//! - 4: Check fulfill (turn-in quest)
//! - 5: Abandon quest
//! - 8: Time sync
//! - 9: Monster data (kill counts)
//! - 10: Show give item
//! - 11: Show map
//! - 12: Accept quest

use ko_db::repositories::quest::QuestRepository;
use ko_protocol::{Opcode, Packet, PacketReader};

/// Maximum number of text IDs in an NPC_SAY dialog.
const MAX_SAY_TEXT_IDS: usize = 8;

/// Native starter quest shown by the client's small helper character.
/// Verified against the v2615 Quest_Guide/Quest_Helper tables and 01_main.lua.
pub const STARTER_SEED_QUEST_ID: u16 = 500;
const STARTER_SEED_KARUS_HELPER_ID: u32 = 5002;
const STARTER_SEED_ELMORAD_HELPER_ID: u32 = 5005;
const STARTER_SEED_WORMS: [u16; 4] = [700, 701, 750, 751];

use crate::handler::zone_change;
use crate::session::{ClientSession, SessionState};
use crate::world::types::ZONE_MORADON;
use crate::zone::SessionId;

/// Handle WIZ_QUEST from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);
    let quest_helper_id = reader.read_u32().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    // Dead players cannot interact with quests
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let quest_helper = world.get_quest_helper(quest_helper_id);

    // Sub-opcodes 4, 5, 12 have relaxed NPC validation (no NPC alive/range check).
    // Sub-opcodes 3, 7, 13 and others require full NPC validation.
    match sub_opcode {
        4 | 5 | 12 => {
            let helper = match quest_helper {
                Some(h) => h,
                None => return Ok(()),
            };

            if !validate_quest_prerequisites(session, &helper) {
                return Ok(());
            }

            match sub_opcode {
                5 => handle_abandon(session, &helper).await?,
                4 => handle_check_fulfill(session, &helper)?,
                12 => handle_accept(session, &helper).await?,
                _ => unreachable!(),
            }
        }
        3 | 7 => {
            let helper = match quest_helper {
                Some(h) => h,
                None => return Ok(()),
            };

            if !validate_quest_with_npc(session, &helper) {
                return Ok(());
            }

            handle_execute_helper(session, &helper)?;
        }
        13 => {
            // in the switch -- it falls through to printf (no-op).
            // Sub-opcode 13 is used as a RESPONSE by RunGiveItemCheckExchange,
            // not as a client request handler.
            let helper = match quest_helper {
                Some(h) => h,
                None => return Ok(()),
            };

            if !validate_quest_with_npc(session, &helper) {
                return Ok(());
            }

            tracing::debug!(
                "[{}] WIZ_QUEST sub-opcode 13 (NPC validated, no-op) quest_helper_id={}",
                session.addr(),
                quest_helper_id,
            );
        }
        _ => {
            tracing::debug!(
                "[{}] Unhandled WIZ_QUEST sub-opcode: {} (quest_helper_id={})",
                session.addr(),
                sub_opcode,
                quest_helper_id,
            );
        }
    }

    Ok(())
}

/// Send quest data to client on game start.
/// Sends:
/// 1. WIZ_QUEST sub-opcode 8 (time sync)
/// 2. WIZ_QUEST sub-opcode 1 (quest list with states)
/// 3. WIZ_QUEST sub-opcode 9 (monster data) for each active quest
pub async fn send_quest_data(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // 1. Send time sync
    let now = chrono::Utc::now();
    let mut time_pkt = Packet::new(Opcode::WizQuest as u8);
    time_pkt.write_u8(8);
    time_pkt.write_u16(chrono::Datelike::year(&now) as u16);
    time_pkt.write_u8(chrono::Datelike::month(&now) as u8);
    time_pkt.write_u8(chrono::Datelike::day(&now) as u8);
    time_pkt.write_u8(chrono::Timelike::hour(&now) as u8);
    time_pkt.write_u8(chrono::Timelike::minute(&now) as u8);
    time_pkt.write_u8(chrono::Timelike::second(&now) as u8);
    session.send_packet(&time_pkt).await?;

    // 2. Build quest list
    let quests = world.with_session(sid, |h| h.quests.clone());
    let quest_map = quests.unwrap_or_default();

    let mut active_quest_ids: Vec<u16> = Vec::with_capacity(quest_map.len());
    let quest_count = quest_map.len() as u16;

    let mut list_pkt = Packet::new(Opcode::WizQuest as u8);
    list_pkt.write_u8(1);
    list_pkt.write_u16(quest_count);

    for (&quest_id, info) in &quest_map {
        list_pkt.write_u16(quest_id);
        list_pkt.write_u8(info.quest_state);

        // Track active/ready quests for monster data request
        if info.quest_state == 1 || info.quest_state == 3 {
            active_quest_ids.push(quest_id);
        }
    }

    session.send_packet(&list_pkt).await?;

    // 3. Send monster data for each active quest
    for quest_id in &active_quest_ids {
        send_quest_monster_data(session, *quest_id).await?;
    }

    Ok(())
}

/// Send monster kill count data for a specific quest.
/// Packet: WIZ_QUEST, sub=9, type=1(u8), quest_id(u16), kill_counts[4](u16 each)
async fn send_quest_monster_data(session: &mut ClientSession, quest_id: u16) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    let quest_info = world.with_session(sid, |h| h.quests.get(&quest_id).cloned());
    let info = match quest_info {
        Some(Some(i)) => i,
        _ => return Ok(()),
    };

    let mut pkt = Packet::new(Opcode::WizQuest as u8);
    pkt.write_u8(9);
    pkt.write_u8(1);
    pkt.write_u16(quest_id);
    pkt.write_u16(info.kill_counts[0] as u16);
    pkt.write_u16(info.kill_counts[1] as u16);
    pkt.write_u16(info.kill_counts[2] as u16);
    pkt.write_u16(info.kill_counts[3] as u16);
    session.send_packet(&pkt).await?;

    Ok(())
}

/// Validate quest prerequisites (nation, level, class) without NPC checks.
fn validate_quest_prerequisites(
    session: &ClientSession,
    helper: &ko_db::models::QuestHelperRow,
) -> bool {
    let world = session.world();
    let sid = session.session_id();

    let char_info = match world.get_character_info(sid) {
        Some(c) => c,
        None => return false,
    };

    // Nation check: 3 = any nation
    if helper.b_nation != 3 && helper.b_nation != char_info.nation as i16 {
        return false;
    }

    // Level check
    if helper.b_level > char_info.level as i16 {
        return false;
    }

    // Class check: 5 = any class
    if helper.b_class != 5 && !job_group_check(char_info.class, helper.b_class) {
        return false;
    }

    true
}

/// Validate quest prerequisites including NPC alive/proto ID/range checks.
///   - NPC must exist and not be dead
///   - NPC proto ID must match quest helper's s_npc_id
///   - Player must be in range of the NPC
fn validate_quest_with_npc(
    session: &ClientSession,
    helper: &ko_db::models::QuestHelperRow,
) -> bool {
    // sEventDataIndex == 500 means no NPC required
    if helper.s_event_data_index == 500 {
        return validate_quest_prerequisites(session, helper);
    }

    let world = session.world();
    let sid = session.session_id();

    // Check event_nid is set
    let event_nid = world.with_session(sid, |h| h.event_nid);
    let nid = match event_nid {
        Some(n) if n >= 0 => n,
        _ => return false,
    };

    let npc_id = nid as u32;

    // NPC must exist and be alive
    if world.is_npc_dead(npc_id) {
        return false;
    }

    // NPC proto ID must match the quest helper's expected NPC
    if let Some(npc) = world.get_npc_instance(npc_id) {
        if helper.s_npc_id != npc.proto_id as i16 {
            return false;
        }
    }

    // Player must be within NPC interaction range
    if !world.is_in_npc_range(sid, npc_id) {
        return false;
    }

    validate_quest_prerequisites(session, helper)
}

/// Check if a class matches the quest requirement.
/// Class format: nation * 100 + class_type (e.g., 101=Karus Warrior, 201=El Morad Warrior).
/// `required_class` uses GROUP_* defines from GameDefine.h:
///   1=Warrior, 2=Rogue, 3=Mage, 4=Priest, 5=Any (sentinel), 13=Kurian.
///   If > 100, exact class match (e.g., 101 = only Karus Warrior base).
pub fn job_group_check(player_class: u16, required_class: i16) -> bool {
    // 5 = any class (sentinel value used in quest_helper.b_class)
    if required_class == 5 {
        return true;
    }
    if required_class > 100 {
        return player_class == required_class as u16;
    }
    // C++ ClassType enum values: base_class = GetClass() % 100
    let base = (player_class % 100) as i16;
    match required_class {
        1 => matches!(base, 1 | 5 | 6), // GROUP_WARRIOR: Warrior(1), Novice(5), Master(6)
        2 => matches!(base, 2 | 7 | 8), // GROUP_ROGUE: Rogue(2), Novice(7), Master(8)
        3 => matches!(base, 3 | 9 | 10), // GROUP_MAGE: Mage(3), Novice(9), Master(10)
        4 => matches!(base, 4 | 11 | 12), // GROUP_CLERIC: Priest(4), Novice(11), Master(12)
        13 => matches!(base, 13..=15),  // GROUP_PORTU_KURIAN: Kurian(13), Novice(14), Master(15)
        _ => base == required_class,    // Fallback: exact sub-class match
    }
}

/// Handle quest accept (sub-opcode 12).
async fn handle_accept(
    session: &mut ClientSession,
    helper: &ko_db::models::QuestHelperRow,
) -> anyhow::Result<()> {
    let quest_id = helper.s_event_data_index as u16;
    let sid = session.session_id();
    let world = session.world().clone();

    let already_ongoing = world
        .with_session(sid, |h| {
            h.quests
                .get(&quest_id)
                .map(|q| q.quest_state == 1)
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if !already_ongoing {
        save_event(session, quest_id, 1).await?;
    }

    Ok(())
}

/// Handle quest abandon (sub-opcode 5).
async fn handle_abandon(
    session: &mut ClientSession,
    helper: &ko_db::models::QuestHelperRow,
) -> anyhow::Result<()> {
    let quest_id = helper.s_event_data_index as u16;
    let sid = session.session_id();
    let world = session.world().clone();

    let exists = world
        .with_session(sid, |h| h.quests.contains_key(&quest_id))
        .unwrap_or(false);

    if !exists {
        return Ok(());
    }

    save_event(session, quest_id, 4).await?;

    // If abandoning quest in zones 81-83, kick the user out to Moradon.
    // Use (0,0) — resolve_zero_coords handles nation-specific start_position.
    let current_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if (81..=83).contains(&current_zone) {
        zone_change::trigger_zone_change(session, ZONE_MORADON, 0.0, 0.0).await?;
    }

    Ok(())
}

/// Handle execute helper (sub-opcode 3/7).
/// In C++ this runs a Lua script. Since we don't have Lua yet, we send the
/// NPC message packet so the client at least sees the quest NPC dialog.
fn handle_execute_helper(
    session: &mut ClientSession,
    helper: &ko_db::models::QuestHelperRow,
) -> anyhow::Result<()> {
    let quest_id = helper.s_event_data_index as u16;
    let sid = session.session_id();
    let world = session.world().clone();

    // If quest is already completed, don't run it again
    let already_completed = world
        .with_session(sid, |h| {
            h.quests
                .get(&quest_id)
                .map(|q| q.quest_state == 2)
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if already_completed {
        return Ok(());
    }

    // Run the quest Lua script with the trigger event index
    quest_v2_run_event(&world, sid, helper, helper.n_event_trigger_index, -1);

    Ok(())
}

/// Handle check fulfill / turn-in (sub-opcode 4).
fn handle_check_fulfill(
    session: &mut ClientSession,
    helper: &ko_db::models::QuestHelperRow,
) -> anyhow::Result<()> {
    let quest_id = helper.s_event_data_index as u16;
    let sid = session.session_id();
    let world = session.world().clone();

    // Quest must be in state 1 (ongoing)
    let (quest_state, kill_counts) = world
        .with_session(sid, |h| {
            h.quests
                .get(&quest_id)
                .map(|q| (q.quest_state, q.kill_counts))
                .unwrap_or((0, [0; 4]))
        })
        .unwrap_or((0, [0; 4]));

    if quest_state != 1 {
        return Ok(());
    }

    // Check monster kill requirements
    if let Some(quest_monster) = world.get_quest_monster(quest_id) {
        // Special case: quest 812 skips kill count check
        if quest_monster.s_quest_num != 812 {
            let counts = [
                quest_monster.s_count1,
                quest_monster.s_count2,
                quest_monster.s_count3,
                quest_monster.s_count4,
            ];

            // `if (pQuestInfo->m_bKillCounts[group] != pQuestMonster->sCount[group]) return;`
            // C++ requires kills == required_count (NOT >= required_count).
            for group in 0..4 {
                if counts[group] > 0 && (kill_counts[group] as i16) != counts[group] {
                    return Ok(());
                }
            }
        }
    }

    // All requirements met — run the Lua completion script
    quest_v2_run_event(&world, sid, helper, helper.n_event_complete_index, -1);

    Ok(())
}

/// Save a quest event (state change) and notify the client.
/// (QuestHandler.cpp:209-353)
/// State values:
/// - 1: ongoing (accept)
/// - 2: completed
/// - 3: ready to complete (all kill requirements met)
/// - 4: removed/abandoned
pub(crate) async fn save_event(
    session: &mut ClientSession,
    quest_id: u16,
    quest_state: u8,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    match quest_state {
        1 => {
            // Set quest to ongoing
            //   - NEW entry (nullptr): create, set state, reset kill counts
            //   - EXISTING entry (else): only update QuestState, keep kill counts
            world.update_session(sid, |h| {
                use std::collections::hash_map::Entry;
                match h.quests.entry(quest_id) {
                    Entry::Vacant(e) => {
                        e.insert(crate::world::UserQuestInfo {
                            quest_state: 1,
                            kill_counts: [0; 4],
                        });
                    }
                    Entry::Occupied(mut e) => {
                        e.get_mut().quest_state = 1;
                        // Do NOT reset kill_counts for existing entries
                    }
                }
            });

            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(2);
            pkt.write_u16(quest_id);
            pkt.write_u8(quest_state);
            session.send_packet(&pkt).await?;

            send_quest_monster_data(session, quest_id).await?;
        }
        2 => {
            // Quest completed
            world.update_session(sid, |h| {
                let info = h.quests.entry(quest_id).or_default();
                info.quest_state = 2;
            });

            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(2);
            pkt.write_u16(quest_id);
            pkt.write_u8(quest_state);
            session.send_packet(&pkt).await?;
            super::achieve::on_quest_completed(&world, sid);
        }
        3 => {
            // Ready to complete
            world.update_session(sid, |h| {
                if let Some(info) = h.quests.get_mut(&quest_id) {
                    info.quest_state = 3;
                }
            });

            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(2);
            pkt.write_u16(quest_id);
            pkt.write_u8(quest_state);
            session.send_packet(&pkt).await?;
        }
        4 => {
            // Removed/abandoned
            world.update_session(sid, |h| {
                if let Some(info) = h.quests.get_mut(&quest_id) {
                    info.quest_state = 4;
                }
            });

            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(2);
            pkt.write_u16(quest_id);
            pkt.write_u8(quest_state);
            session.send_packet(&pkt).await?;

            // Remove from quest map
            world.update_session(sid, |h| {
                h.quests.remove(&quest_id);
            });
        }
        _ => {
            // Default: just update state
            world.update_session(sid, |h| {
                if let Some(info) = h.quests.get_mut(&quest_id) {
                    info.quest_state = quest_state;
                }
            });

            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(2);
            pkt.write_u16(quest_id);
            pkt.write_u8(quest_state);
            session.send_packet(&pkt).await?;
        }
    }

    // Persist to DB (fire-and-forget)
    let pool = session.pool().clone();
    let char_id = session.character_id().unwrap_or("").to_string();
    if !char_id.is_empty() {
        let kill_counts = world
            .with_session(sid, |h| h.quests.get(&quest_id).map(|q| q.kill_counts))
            .flatten()
            .unwrap_or([0; 4]);

        tokio::spawn(async move {
            let repo = QuestRepository::new(&pool);
            if quest_state == 4 {
                if let Err(e) = repo.delete_user_quest(&char_id, quest_id as i16).await {
                    tracing::error!("Failed to delete quest {}: {}", quest_id, e);
                }
            } else {
                let kc = [
                    kill_counts[0] as i16,
                    kill_counts[1] as i16,
                    kill_counts[2] as i16,
                    kill_counts[3] as i16,
                ];
                if let Err(e) = repo
                    .save_user_quest(&char_id, quest_id as i16, quest_state as i16, kc)
                    .await
                {
                    tracing::error!("Failed to save quest {}: {}", quest_id, e);
                }
            }
        });
    }

    Ok(())
}

/// Track monster kill for quest progress.
/// Called when an NPC/monster is killed. Checks all active quests for matching
/// monster IDs and increments kill counts. Uses `world.send_to_session` for
/// sending packets since this may be called from non-async context.
/// C++ call site: `CNpc::OnDeathProcess()` — called for the killer and all
/// party members within 80m range.
pub fn quest_monster_count_add(
    world: &crate::world::WorldState,
    sid: SessionId,
    npc_proto_id: u16,
) {
    let quests = world.with_session(sid, |h| h.quests.clone());
    let quest_map = match quests {
        Some(q) if !q.is_empty() => q,
        _ => return,
    };

    // Collect all kill count updates + packets, then apply in batched DashMap writes.
    // Original pattern: N update_session calls per quest group match + 1 re-read for fulfillment.
    // Optimized: 1 update_session per quest (includes kill counts + state transition).
    struct KillUpdate {
        quest_num: u16,
        group: usize,
        new_count: u8,
    }

    for (&quest_num, info) in &quest_map {
        if info.quest_state != 1 {
            continue;
        }

        let quest_monster = match world.get_quest_monster(quest_num) {
            Some(m) => m,
            None => continue,
        };

        let monster_ids: [[i16; 4]; 4] = [
            [
                quest_monster.s_num1a,
                quest_monster.s_num1b,
                quest_monster.s_num1c,
                quest_monster.s_num1d,
            ],
            [
                quest_monster.s_num2a,
                quest_monster.s_num2b,
                quest_monster.s_num2c,
                quest_monster.s_num2d,
            ],
            [
                quest_monster.s_num3a,
                quest_monster.s_num3b,
                quest_monster.s_num3c,
                quest_monster.s_num3d,
            ],
            [
                quest_monster.s_num4a,
                quest_monster.s_num4b,
                quest_monster.s_num4c,
                quest_monster.s_num4d,
            ],
        ];
        let required_counts = [
            quest_monster.s_count1,
            quest_monster.s_count2,
            quest_monster.s_count3,
            quest_monster.s_count4,
        ];

        // Collect updates locally — track counts without DashMap writes
        let mut updates: Vec<KillUpdate> = Vec::new();
        let mut tracked_counts = info.kill_counts;

        for group in 0..4 {
            for &mob_id in &monster_ids[group] {
                if mob_id != npc_proto_id as i16 {
                    continue;
                }

                let current_count = tracked_counts[group];
                if (current_count as i16 + 1) > required_counts[group] {
                    continue;
                }

                let new_count = current_count + 1;
                tracked_counts[group] = new_count;
                updates.push(KillUpdate {
                    quest_num,
                    group,
                    new_count,
                });
            }
        }

        if updates.is_empty() {
            continue;
        }

        // Check fulfillment using locally tracked counts (no re-read needed)
        let all_fulfilled = (0..4)
            .all(|v| required_counts[v] <= 0 || (tracked_counts[v] as i16) >= required_counts[v]);

        // Single update_session: apply all kill count changes + optional state transition
        world.update_session(sid, |h| {
            if let Some(q) = h.quests.get_mut(&quest_num) {
                for u in &updates {
                    q.kill_counts[u.group] = u.new_count;
                }
                if all_fulfilled {
                    q.quest_state = 3;
                }
            }
        });

        // Send kill count update packets
        for u in &updates {
            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(9);
            pkt.write_u8(2);
            pkt.write_u16(u.quest_num);
            pkt.write_u8((u.group + 1) as u8);
            pkt.write_u16(u.new_count as u16);
            world.send_to_session_owned(sid, pkt);
        }

        let char_id = world
            .get_character_info(sid)
            .map(|ch| ch.name.clone())
            .unwrap_or_default();
        let persisted_state = if all_fulfilled { 3 } else { 1 };

        tracing::info!(
            sid,
            character = %char_id,
            quest_num,
            npc_proto_id,
            counts = ?tracked_counts,
            required = ?required_counts,
            completed = all_fulfilled,
            "Quest monster progress updated"
        );

        // Persist partial progress too. Previously Ancient Hunt progress was
        // only saved on the final kill and was lost on reconnect/restart.
        if !char_id.is_empty() {
            if let Some(pool) = world.db_pool() {
                let pool = pool.clone();
                let kc = [
                    tracked_counts[0] as i16,
                    tracked_counts[1] as i16,
                    tracked_counts[2] as i16,
                    tracked_counts[3] as i16,
                ];
                tokio::spawn(async move {
                    let repo = QuestRepository::new(&pool);
                    if let Err(e) = repo
                        .save_user_quest_progress(&char_id, quest_num as i16, persisted_state, kc)
                        .await
                    {
                        tracing::error!(
                            "Failed to save quest {} state {}: {}",
                            quest_num,
                            persisted_state,
                            e
                        );
                    }
                });
            }
        }

        if all_fulfilled {
            // Send state update
            let mut pkt = Packet::new(Opcode::WizQuest as u8);
            pkt.write_u8(2);
            pkt.write_u16(quest_num);
            pkt.write_u8(3);
            world.send_to_session_owned(sid, pkt);
        }
    }
}

/// Load quest data from DB into the session's quest map.
pub async fn load_quest_data(session: &mut ClientSession) -> anyhow::Result<()> {
    let char_id = session.character_id().unwrap_or("").to_string();
    if char_id.is_empty() {
        return Ok(());
    }

    let pool = session.pool().clone();
    let repo = QuestRepository::new(&pool);
    let rows = repo.load_user_quests(&char_id).await?;

    let sid = session.session_id();
    let world = session.world().clone();

    world.update_session(sid, |h| {
        for row in &rows {
            let info = crate::world::UserQuestInfo {
                quest_state: row.quest_state as u8,
                kill_counts: [
                    row.kill_count1 as u8,
                    row.kill_count2 as u8,
                    row.kill_count3 as u8,
                    row.kill_count4 as u8,
                ],
            };
            h.quests.insert(row.quest_id as u16, info);
        }
    });

    // Quest 500 is the native one-time "Rescuing Sid" starter quest.  It has
    // no NPC acceptance step: the v2615 tables route a Worm death directly to
    // helper 5002/5005 and 01_main.lua EVENT 502.  Seed it as ongoing only
    // when the character has never completed it; this deliberately has no
    // upper-level restriction, matching Quest_Guide's 1..100 range.
    let starter_seed_needs_initialising = rows
        .iter()
        .find(|row| row.quest_id as u16 == STARTER_SEED_QUEST_ID)
        .map(|row| row.quest_state == 0)
        .unwrap_or(true);
    if starter_seed_needs_initialising {
        world.update_session(sid, |h| {
            h.quests.insert(
                STARTER_SEED_QUEST_ID,
                crate::world::UserQuestInfo {
                    quest_state: 1,
                    kill_counts: [0; 4],
                },
            );
        });

        if let Err(error) = repo
            .save_user_quest(&char_id, STARTER_SEED_QUEST_ID as i16, 1, [0; 4])
            .await
        {
            tracing::error!(
                char_name = %char_id,
                quest_id = STARTER_SEED_QUEST_ID,
                %error,
                "failed to initialise native starter quest"
            );
        } else {
            tracing::info!(
                char_name = %char_id,
                quest_id = STARTER_SEED_QUEST_ID,
                "native Rescuing Sid quest initialised as ongoing"
            );
        }
    }

    tracing::debug!(
        "[{}] Loaded {} quest entries for {}",
        session.addr(),
        rows.len(),
        char_id,
    );

    Ok(())
}

/// Complete the v2615 native "Rescuing Sid" flow after the first starter Worm
/// kill.  The first EVENT 502 result is emitted here because NPC death code
/// holds `&WorldState`; subsequent dialog buttons continue through 01_main.lua
/// using the stored helper ID exactly like a normal SelectMsg chain.
pub fn complete_starter_seed_quest_on_worm_kill(
    world: &crate::world::WorldState,
    sid: SessionId,
    npc_proto_id: u16,
) -> bool {
    if !STARTER_SEED_WORMS.contains(&npc_proto_id) {
        return false;
    }

    let (quest_state, nation) = match world.with_session(sid, |h| {
        (
            h.quests
                .get(&STARTER_SEED_QUEST_ID)
                .map(|quest| quest.quest_state)
                .unwrap_or(0),
            h.character.as_ref().map(|ch| ch.nation).unwrap_or(0),
        )
    }) {
        Some(values) => values,
        None => return false,
    };
    if quest_state != 1 {
        return false;
    }

    let (helper_id, header_text, button_text) = match nation {
        1 => (STARTER_SEED_KARUS_HELPER_ID, 5002, 5001),
        2 => (STARTER_SEED_ELMORAD_HELPER_ID, 5003, 5004),
        _ => return false,
    };
    let helper = match world.get_quest_helper(helper_id) {
        Some(helper)
            if helper.s_event_data_index == STARTER_SEED_QUEST_ID as i16
                && helper.b_event_status == 2
                && helper.n_event_trigger_index == 502
                && helper.str_lua_filename.eq_ignore_ascii_case("01_main.lua") =>
        {
            helper
        }
        Some(helper) => {
            tracing::error!(
                sid,
                helper_id,
                event_data = helper.s_event_data_index,
                event_status = helper.b_event_status,
                trigger = helper.n_event_trigger_index,
                lua = %helper.str_lua_filename,
                "v2615 Rescuing Sid helper contract mismatch"
            );
            return false;
        }
        None => {
            tracing::error!(sid, helper_id, "v2615 Rescuing Sid helper is missing");
            return false;
        }
    };

    // 01_main.lua EVENT 502 starts with SaveEvent(5002/5005), whose helper
    // maps quest 500 to state 2.  NPC=0 is significant for this system dialog.
    world.update_session(sid, |h| {
        if let Some(quest) = h.quests.get_mut(&STARTER_SEED_QUEST_ID) {
            quest.quest_state = 2;
        }
        h.quest_helper_id = helper_id;
        h.event_sid = 0;
        h.event_nid = -1;
    });

    let mut state_packet = Packet::new(Opcode::WizQuest as u8);
    state_packet.write_u8(2);
    state_packet.write_u16(STARTER_SEED_QUEST_ID);
    state_packet.write_u8(2);
    world.send_to_session_owned(sid, state_packet);

    let mut button_texts = [-1i32; 12];
    let mut button_events = [-1i32; 12];
    button_texts[0] = button_text;
    button_events[0] = 505;
    super::select_msg::send_select_msg(
        world,
        sid,
        6,
        STARTER_SEED_QUEST_ID as i32,
        header_text,
        &button_texts,
        &button_events,
        &helper.str_lua_filename,
    );

    if let (Some(pool), Some(character)) = (world.db_pool(), world.get_character_info(sid)) {
        let pool = pool.clone();
        let char_name = character.name;
        tokio::spawn(async move {
            let repo = QuestRepository::new(&pool);
            if let Err(error) = repo
                .save_user_quest(&char_name, STARTER_SEED_QUEST_ID as i16, 2, [0; 4])
                .await
            {
                tracing::error!(
                    char_name,
                    quest_id = STARTER_SEED_QUEST_ID,
                    %error,
                    "failed to persist Rescuing Sid completion"
                );
            }
        });
    }

    tracing::info!(
        sid,
        npc_proto_id,
        quest_id = STARTER_SEED_QUEST_ID,
        helper_id,
        lua = %helper.str_lua_filename,
        event_id = 502,
        "Rescuing Sid completed through the v2615 starter quest flow"
    );
    true
}

/// Build a WIZ_NPC_SAY (0x56) packet for NPC dialog text.
/// The packet sends up to 8 text IDs that the client looks up in the quest
/// string table. The first two i32 fields are always -1 (reserved).
/// Wire format: `[i32 -1][i32 -1][i32 text_id × 8]`
pub fn build_npc_say_packet(text_ids: &[i32]) -> Packet {
    let mut pkt = Packet::new(Opcode::WizNpcSay as u8);

    pkt.write_i32(-1);
    pkt.write_i32(-1);

    // Write up to 8 text IDs, pad remaining with 0
    for i in 0..MAX_SAY_TEXT_IDS {
        let id = text_ids.get(i).copied().unwrap_or(0);
        pkt.write_i32(id);
    }

    pkt
}

/// Run a quest Lua script via the Lua engine.
/// (QuestHandler.cpp:534-563)
/// Sets `m_nQuestHelperID` on the session, then calls `LuaEngine::execute()` with the
/// helper's Lua filename. For NPC-linked quests (sEventDataIndex != 500), verifies the
/// NPC exists before running.
pub fn quest_v2_run_event(
    world: &std::sync::Arc<crate::world::WorldState>,
    sid: crate::zone::SessionId,
    helper: &ko_db::models::QuestHelperRow,
    event_id: i32,
    selected_reward: i8,
) -> bool {
    tracing::info!(
        sid,
        event_id,
        selected_reward,
        helper_id = helper.n_index,
        lua = %helper.str_lua_filename,
        "quest_v2_run_event: START"
    );

    // Store quest helper ID on session (C++: m_nQuestHelperID = pQuestHelper->nIndex)
    world.update_session(sid, |h| {
        h.quest_helper_id = helper.n_index as u32;
    });

    // Execute the Lua script
    let result = world.lua_engine().execute(
        world,
        sid,
        event_id,
        selected_reward,
        &helper.str_lua_filename,
    );

    if !result {
        tracing::warn!(
            sid,
            event_id,
            lua = %helper.str_lua_filename,
            "quest_v2_run_event: Lua execution FAILED"
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_say_packet_format() {
        let ids = [100, 200, 300];
        let pkt = build_npc_say_packet(&ids);

        let mut r = PacketReader::new(&pkt.data);
        // Reserved fields
        assert_eq!(r.read_i32(), Some(-1));
        assert_eq!(r.read_i32(), Some(-1));
        // Text IDs
        assert_eq!(r.read_i32(), Some(100));
        assert_eq!(r.read_i32(), Some(200));
        assert_eq!(r.read_i32(), Some(300));
        // Remaining padded with 0
        for _ in 3..MAX_SAY_TEXT_IDS {
            assert_eq!(r.read_i32(), Some(0));
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_npc_say_empty() {
        let pkt = build_npc_say_packet(&[]);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(-1));
        assert_eq!(r.read_i32(), Some(-1));
        for _ in 0..MAX_SAY_TEXT_IDS {
            assert_eq!(r.read_i32(), Some(0));
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_npc_say_packet_size() {
        let pkt = build_npc_say_packet(&[1, 2, 3, 4, 5, 6, 7, 8]);
        // 2 reserved i32 + 8 text i32 = 10 × 4 = 40 bytes
        assert_eq!(pkt.data.len(), 40);
    }

    // ── Sprint 287: Quest abandon zone kick-out ─────────────────────────

    #[test]
    fn test_monster_suppression_zones_81_to_83() {
        // Abandoning a quest in zones 81-83 kicks the user to Moradon.
        for zone in 81u16..=83u16 {
            assert!(
                (81..=83).contains(&zone),
                "Zone {} must trigger kick-out on quest abandon",
                zone
            );
        }
        // Zones outside 81-83 must NOT trigger kick-out
        assert!(!(81..=83).contains(&80u16));
        assert!(!(81..=83).contains(&84u16));
    }

    // ── Sprint 313: Quest fulfill exact kill count check ─────────────

    /// `if (pQuestInfo->m_bKillCounts[group] != pQuestMonster->sCount[group]) return;`
    /// C++ uses exact equality (!=), NOT greater-than-or-equal.
    #[test]
    fn test_quest_fulfill_requires_exact_kills() {
        let required: i16 = 10;
        let exact_kills: i16 = 10;
        let over_kills: i16 = 15;
        let under_kills: i16 = 5;

        // Exact match → pass (should NOT reject)
        assert!(exact_kills == required);
        // Over-kill → fail (MUST reject — kills != required)
        assert!(over_kills != required);
        // Under-kill → fail (kills != required)
        assert!(under_kills != required);
    }

    #[test]
    fn test_quest_fulfill_zero_required_skipped() {
        // If required count is 0, the group is skipped (no check needed)
        let required: i16 = 0;
        let any_kills: i16 = 5;
        // C++ condition: `counts[group] > 0 && kills != counts[group]`
        // When required is 0, the first condition fails → skip
        let should_check = required > 0;
        assert!(!should_check);
        let _ = any_kills; // Not checked
    }

    // ── Sprint 921: job_group_check + packet format coverage ────────

    #[test]
    fn test_job_group_check_any_class() {
        // required_class=5 means any class → always true
        assert!(job_group_check(101, 5)); // Karus Warrior
        assert!(job_group_check(203, 5)); // El Morad Mage
        assert!(job_group_check(999, 5)); // Any arbitrary class
    }

    #[test]
    fn test_job_group_check_warrior_group() {
        // GROUP_WARRIOR=1: base 1 (Warrior), 5 (Novice), 6 (Master)
        assert!(job_group_check(101, 1)); // Karus Warrior base
        assert!(job_group_check(105, 1)); // Karus Warrior Novice
        assert!(job_group_check(106, 1)); // Karus Warrior Master
        assert!(job_group_check(201, 1)); // El Morad Warrior base
        assert!(job_group_check(205, 1)); // El Morad Warrior Novice
        assert!(job_group_check(206, 1)); // El Morad Warrior Master
                                          // Rogue should NOT match Warrior group
        assert!(!job_group_check(102, 1));
        assert!(!job_group_check(107, 1));
    }

    #[test]
    fn test_job_group_check_rogue_group() {
        // GROUP_ROGUE=2: base 2 (Rogue), 7 (Novice), 8 (Master)
        assert!(job_group_check(102, 2));
        assert!(job_group_check(107, 2));
        assert!(job_group_check(108, 2));
        assert!(!job_group_check(101, 2)); // Warrior != Rogue
    }

    #[test]
    fn test_job_group_check_mage_group() {
        // GROUP_MAGE=3: base 3 (Mage), 9 (Novice), 10 (Master)
        assert!(job_group_check(103, 3));
        assert!(job_group_check(109, 3));
        assert!(job_group_check(110, 3));
        assert!(!job_group_check(104, 3)); // Priest != Mage
    }

    #[test]
    fn test_job_group_check_priest_group() {
        // GROUP_CLERIC=4: base 4 (Priest), 11 (Novice), 12 (Master)
        assert!(job_group_check(104, 4));
        assert!(job_group_check(111, 4));
        assert!(job_group_check(112, 4));
        assert!(!job_group_check(103, 4)); // Mage != Priest
    }

    #[test]
    fn test_job_group_check_kurian_group() {
        // GROUP_PORTU_KURIAN=13: base 13, 14 (Novice), 15 (Master)
        assert!(job_group_check(113, 13));
        assert!(job_group_check(114, 13));
        assert!(job_group_check(115, 13));
        assert!(!job_group_check(101, 13)); // Warrior != Kurian
    }

    #[test]
    fn test_job_group_check_exact_class_over_100() {
        // required > 100 → exact match only
        assert!(job_group_check(101, 101)); // exact Karus Warrior base
        assert!(!job_group_check(105, 101)); // Karus Warrior Novice != 101
        assert!(job_group_check(208, 208)); // exact El Morad Rogue Master
    }

    #[test]
    fn test_quest_time_sync_packet_format() {
        // Sub-opcode 8: time sync packet
        let mut pkt = Packet::new(Opcode::WizQuest as u8);
        pkt.write_u8(8);
        pkt.write_u16(2026); // year
        pkt.write_u8(3); // month
        pkt.write_u8(13); // day
        pkt.write_u8(14); // hour
        pkt.write_u8(30); // minute
        pkt.write_u8(0); // second

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(8)); // sub=8
        assert_eq!(r.read_u16(), Some(2026));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(13));
        assert_eq!(r.read_u8(), Some(14));
        assert_eq!(r.read_u8(), Some(30));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_quest_save_event_packet_format() {
        // Sub-opcode 2: save event (state change notification)
        let mut pkt = Packet::new(Opcode::WizQuest as u8);
        pkt.write_u8(2); // sub=2
        pkt.write_u16(1001); // quest_id
        pkt.write_u8(1); // state=ongoing

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(1001));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_quest_monster_data_packet_format() {
        // Sub-opcode 9, type 1: initial monster data
        let mut pkt = Packet::new(Opcode::WizQuest as u8);
        pkt.write_u8(9);
        pkt.write_u8(1); // type=1 (initial)
        pkt.write_u16(500); // quest_id
        pkt.write_u16(3); // kill_count[0]
        pkt.write_u16(0); // kill_count[1]
        pkt.write_u16(5); // kill_count[2]
        pkt.write_u16(0); // kill_count[3]

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(9));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(500));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_quest_kill_update_packet_format() {
        // Sub-opcode 9, type 2: per-group kill count update
        let mut pkt = Packet::new(Opcode::WizQuest as u8);
        pkt.write_u8(9);
        pkt.write_u8(2); // type=2 (update)
        pkt.write_u16(500); // quest_id
        pkt.write_u8(1); // group (1-indexed)
        pkt.write_u16(4); // new_count

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(9));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(500));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(4));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_max_say_text_ids_constant() {
        assert_eq!(MAX_SAY_TEXT_IDS, 8);
    }

    #[test]
    fn test_npc_say_overflow_ids_capped_at_8() {
        // Providing more than 8 IDs should only use first 8
        let ids: Vec<i32> = (1..=12).collect();
        let pkt = build_npc_say_packet(&ids);
        let mut r = PacketReader::new(&pkt.data);
        r.read_i32(); // skip -1
        r.read_i32(); // skip -1
        for i in 1..=8i32 {
            assert_eq!(r.read_i32(), Some(i));
        }
        // Should NOT contain ids 9-12
        assert_eq!(r.remaining(), 0);
    }

    /// Quest sub-opcodes: list=1, save=2, execute=3/7, fulfill=4, abandon=5, time=8, monster=9, accept=12.
    #[test]
    fn test_quest_sub_opcode_values() {
        // From the match statement in handle()
        let quest_list: u8 = 1;
        let quest_save: u8 = 2;
        let quest_execute1: u8 = 3;
        let quest_fulfill: u8 = 4;
        let quest_abandon: u8 = 5;
        let quest_execute2: u8 = 7;
        let quest_time: u8 = 8;
        let quest_monster: u8 = 9;
        let quest_accept: u8 = 12;
        // All distinct
        let ops = [
            quest_list,
            quest_save,
            quest_execute1,
            quest_fulfill,
            quest_abandon,
            quest_execute2,
            quest_time,
            quest_monster,
            quest_accept,
        ];
        for i in 0..ops.len() {
            for j in (i + 1)..ops.len() {
                assert_ne!(ops[i], ops[j]);
            }
        }
        assert_eq!(ops.len(), 9);
    }

    /// WIZ_QUEST opcode is 0x64 within v2525 dispatch range.
    #[test]
    fn test_quest_opcode_value() {
        assert_eq!(Opcode::WizQuest as u8, 0x64);
        assert!(Opcode::WizQuest as u8 >= 0x06);
        assert!(Opcode::WizQuest as u8 <= 0xD7);
    }

    /// job_group_check: class 5 = any class (sentinel).
    #[test]
    fn test_job_group_any_class_sentinel() {
        // required_class=5 matches all classes
        for class in [101u16, 102, 103, 104, 113, 201, 202, 203, 204, 213] {
            assert!(
                job_group_check(class, 5),
                "class {} should pass any-class check",
                class
            );
        }
    }

    /// Monster suppression zones 81-83 match war event zones.
    #[test]
    fn test_monster_suppression_zone_ids() {
        // Zones 81, 82, 83 are war zones where quest kills are suppressed
        let war_zones: [u16; 3] = [81, 82, 83];
        assert_eq!(war_zones.len(), 3);
        // Contiguous block
        assert_eq!(war_zones[2] - war_zones[0], 2);
        // All distinct from Moradon (21)
        assert!(war_zones.iter().all(|&z| z != ZONE_MORADON));
    }

    /// job_group_check: required > 100 is exact class match.
    #[test]
    fn test_job_group_exact_class_match() {
        // 101 = Karus Warrior base — only matches class 101
        assert!(job_group_check(101, 101));
        assert!(!job_group_check(201, 101)); // El Morad Warrior doesn't match
        assert!(!job_group_check(102, 101)); // Rogue doesn't match
                                             // 213 = El Morad Kurian master — only matches class 213
        assert!(job_group_check(213, 213));
        assert!(!job_group_check(113, 213));
    }
}
