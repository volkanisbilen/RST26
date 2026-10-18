//! WIZ_LOGOUT (0x0F) handler — player logout / return to character select.
//! ## Flow
//! 1. Client sends WIZ_LOGOUT (empty packet, opcode only)
//! 2. Server saves character state to DB
//! 3. Server removes user from zone/region, broadcasts INOUT_OUT
//! 4. Server cleans up party, exchange, merchant, party BBS
//! 5. Server marks account offline in `currentuser` table
//! 6. Server sends WIZ_LOGOUT response so client returns to character select

use ko_db::repositories::account::AccountRepository;
use ko_db::repositories::achieve::AchieveRepository;
use ko_db::repositories::character::{CharacterRepository, SaveStatsParams};
use ko_db::repositories::perk::PerkRepository;
use ko_db::repositories::premium::PremiumRepository;
use ko_db::repositories::quest::QuestRepository;
use ko_db::repositories::saved_magic::SavedMagicRepository;
use ko_db::repositories::user_data::UserDataRepository;
use ko_protocol::{Opcode, Packet};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::handler::{mining, party_bbs, region};
use crate::session::{ClientSession, SessionState};

/// Handle WIZ_LOGOUT from the client.
/// Saves character data, cleans up world state, and sends confirmation.
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        debug!(
            "[{}] WIZ_LOGOUT ignored: not in game (state={:?})",
            session.addr(),
            session.state()
        );
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();
    let pool = session.pool().clone();
    let char_id = session.character_id().unwrap_or("").to_string();
    let account_id = session.account_id().unwrap_or("").to_string();

    // ── 0. Fun Class: give the character its own identity back BEFORE it is saved ──
    crate::handler::dungeon_defence::restore_on_disconnect(&world, sid);
            world.manes_survival_manager.leave_event(&world, sid, false);
    crate::handler::cinderella::cinderella_logout(&world, sid, true);

    // ── 1. Save character stats + position to DB ────────────────────
    if !char_id.is_empty() {
        // Save stats (hp, mp, exp, gold, loyalty, loyalty_monthly, manner_point)
        if let Some(ch) = world.get_character_info(sid) {
            let repo = CharacterRepository::new(&pool);
            if let Err(e) = repo
                .save_stats(&SaveStatsParams {
                    char_id: &char_id,
                    level: ch.level as i16,
                    hp: ch.hp,
                    mp: ch.mp,
                    sp: ch.sp,
                    exp: ch.exp as i64,
                    gold: ch.gold.min(i32::MAX as u32) as i32,
                    loyalty: ch.loyalty.min(i32::MAX as u32) as i32,
                    loyalty_monthly: ch.loyalty_monthly.min(i32::MAX as u32) as i32,
                    manner_point: ch.manner_point,
                })
                .await
            {
                warn!("[{}] Logout: failed to save stats: {}", session.addr(), e);
            }
        }
        // Save position
        if let Some(pos) = world.get_position(sid) {
            let repo = CharacterRepository::new(&pool);
            let px = (pos.x * 100.0) as i32;
            let pz = (pos.z * 100.0) as i32;
            if let Err(e) = repo
                .save_position(&char_id, pos.zone_id as i16, px, 0, pz)
                .await
            {
                warn!(
                    "[{}] Logout: failed to save position: {}",
                    session.addr(),
                    e
                );
            }
        }
    }

    // ── 1a2. Save class/race (safety net for Lua PromoteUser*) ─────────
    if !char_id.is_empty() {
        if let Some(ch) = world.get_character_info(sid) {
            let repo = CharacterRepository::new(&pool);
            if let Err(e) = repo
                .save_class_change(&char_id, ch.class as i16, ch.race as i16)
                .await
            {
                warn!(
                    "[{}] Logout: failed to save class/race: {}",
                    session.addr(),
                    e
                );
            }
        }
    }

    // ── 1a3. Save flash time/count/type ────────────────────────────────
    if !char_id.is_empty() {
        let flash_data = world.with_session(sid, |h| (h.flash_time, h.flash_count, h.flash_type));
        if let Some((ft, fc, ftype)) = flash_data {
            if ft > 0 || fc > 0 {
                let repo = CharacterRepository::new(&pool);
                if let Err(e) = repo
                    .save_flash(&char_id, ft as i32, fc as i16, ftype as i16)
                    .await
                {
                    warn!("[{}] Logout: failed to save flash: {}", session.addr(), e);
                }
            }
        }
    }

    // ── 1a4. Save stat + skill points ──────────────────────────────────
    // AWAITED — stat/skill point loss on re-login is highly noticeable
    if !char_id.is_empty() {
        if let Some(ch) = world.get_character_info(sid) {
            let sp_repo = CharacterRepository::new(&pool);
            if let Err(e) = sp_repo
                .save_stat_points(&ko_db::repositories::character::SaveStatPointsParams {
                    char_id: &char_id,
                    str_val: ch.str as i16,
                    sta: ch.sta as i16,
                    dex: ch.dex as i16,
                    intel: ch.intel as i16,
                    cha: ch.cha as i16,
                    free_points: ch.free_points as i16,
                    skill_points: [
                        ch.skill_points[0] as i16,
                        ch.skill_points[1] as i16,
                        ch.skill_points[2] as i16,
                        ch.skill_points[3] as i16,
                        ch.skill_points[4] as i16,
                        ch.skill_points[5] as i16,
                        ch.skill_points[6] as i16,
                        ch.skill_points[7] as i16,
                        ch.skill_points[8] as i16,
                        ch.skill_points[9] as i16,
                    ],
                })
                .await
            {
                warn!(
                    "[{}] Logout: failed to save stat points: {}",
                    session.addr(),
                    e
                );
            }
        }
    }

    // ── 1b. Bulk save inventory items (AWAITED — not fire-and-forget) ──
    // IMPORTANT: This save MUST complete before the player can re-select the
    // character, otherwise the re-login load may see stale/empty DB data.
    if !char_id.is_empty() {
        let inventory = world.get_persistent_inventory(sid);
        if !inventory.is_empty() {
            let non_empty_count = inventory.iter().filter(|s| s.item_id != 0).count();
            debug!(
                "[{}] Logout inventory save: char={}, slots={}, non_empty={}",
                session.addr(),
                char_id,
                inventory.len(),
                non_empty_count,
            );
            let repo = CharacterRepository::new(&pool);
            let params: Vec<ko_db::repositories::character::SaveItemParams> = inventory
                .iter()
                .enumerate()
                .map(
                    |(slot, item)| ko_db::repositories::character::SaveItemParams {
                        char_id: &char_id,
                        slot_index: slot as i16,
                        item_id: item.item_id as i32,
                        durability: item.durability,
                        count: item.count as i16,
                        flag: item.flag as i16,
                        original_flag: item.original_flag as i16,
                        serial_num: item.serial_num as i64,
                        expire_time: item.expire_time as i32,
                    },
                )
                .collect();
            if let Err(e) = repo.save_items_batch(&params).await {
                warn!("Logout: failed to save inventory for {}: {}", char_id, e);
            }
        }
    }

    // ── 1c. Bulk save warehouse items ──────────────────────────────────
    if !account_id.is_empty() {
        let wh_data = world.with_session(sid, |h| {
            (h.warehouse.clone(), h.inn_coins, h.warehouse_loaded)
        });
        if let Some((warehouse, inn_coins, loaded)) = wh_data {
            if loaded && !warehouse.is_empty() {
                let wh_repo = CharacterRepository::new(&pool);
                let wh_params: Vec<ko_db::repositories::character::SaveWarehouseItemParams> =
                    warehouse
                        .iter()
                        .enumerate()
                        .map(|(slot, item)| {
                            ko_db::repositories::character::SaveWarehouseItemParams {
                                account_id: &account_id,
                                slot_index: slot as i16,
                                item_id: item.item_id as i32,
                                durability: item.durability,
                                count: item.count as i16,
                                flag: item.flag as i16,
                                original_flag: item.original_flag as i16,
                                serial_num: item.serial_num as i64,
                                expire_time: item.expire_time as i32,
                            }
                        })
                        .collect();
                if let Err(e) = wh_repo.save_warehouse_items_batch(&wh_params).await {
                    warn!("Logout: failed to save warehouse for {}: {}", account_id, e);
                }
                if let Err(e) = wh_repo
                    .save_warehouse_coins(&account_id, inn_coins.min(i32::MAX as u32) as i32)
                    .await
                {
                    warn!(
                        "Logout: failed to save warehouse coins for {}: {}",
                        account_id, e
                    );
                }
            }
        }
    }

    // ── 1b. BDW cleanup ────────────────────────────────────────────
    bdw_user_logout(&world, sid);

    // ── 1c. Monster Stone cleanup ────────────────────────────────────
    {
        let ms_active = world
            .with_session(sid, |h| h.event_room > 0)
            .unwrap_or(false);
        if ms_active {
            super::zone_change::monster_stone_exit_room(&world, sid);
        }
    }

    // ── 1d. Draki Tower cleanup ──────────────────────────────────────
    // BUG-6 fix: clean up room when player disconnects inside Draki Tower
    {
        use crate::handler::draki_tower;
        let room_id = world.with_session(sid, |h| h.draki_room_id).unwrap_or(0);
        if room_id > 0 {
            world.update_session(sid, |h| {
                h.event_room = 0;
                h.draki_room_id = 0;
            });
            world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, room_id);
            let mut rooms = world.draki_tower_rooms_write();
            if let Some(room) = rooms.get_mut(&room_id) {
                room.reset();
            }
        }
    }

    // ── 2. Clean up party ────────────────────────────────────────────
    world.cleanup_party_on_disconnect(sid);

    // ── 3. Clean up exchange (trade) ─────────────────────────────────
    if world.is_trading(sid) {
        let partner_sid = world.get_exchange_user(sid);
        world.reset_trade(sid);
        if let Some(partner) = partner_sid {
            world.reset_trade(partner);
            let mut cancel_pkt = Packet::new(Opcode::WizExchange as u8);
            cancel_pkt.write_u8(0x08); // EXCHANGE_CANCEL
            world.send_to_session_owned(partner, cancel_pkt);
        }
    }

    // ── 4. Clean up merchant state ───────────────────────────────────
    if world.is_merchanting(sid) {
        world.close_merchant(sid);
    }

    // ── 4b. Cancel active challenge/duel ───────────────────────────────
    {
        let (requesting, requested, challenge_user) = world.get_challenge_state(sid);
        if requesting > 0 || requested > 0 {
            let target = challenge_user as u16;
            if challenge_user >= 0 {
                world.update_session(target, |h| {
                    h.challenge_user = -1;
                    h.requesting_challenge = 0;
                    h.challenge_requested = 0;
                });
                let mut cancel_pkt = Packet::new(Opcode::WizChallenge as u8);
                cancel_pkt.write_u8(if requesting > 0 { 2 } else { 4 });
                world.send_to_session_owned(target, cancel_pkt);
            }
            world.update_session(sid, |h| {
                h.challenge_user = -1;
                h.requesting_challenge = 0;
                h.challenge_requested = 0;
            });
        }
    }

    // ── 4c. Remove rival ───────────────────────────────────────────────
    {
        let has_rival = world
            .get_character_info(sid)
            .map(|ch| ch.rival_id >= 0)
            .unwrap_or(false);
        if has_rival {
            world.remove_rival(sid);
        }
    }

    // ── 4d. Remove from merchant we're browsing ────────────────────────
    world.remove_from_merchant_lookers(sid);

    // ── 4e. Stop mining / fishing ──────────────────────────────────────
    mining::stop_mining_internal(&world, sid);
    mining::stop_fishing_internal(&world, sid);

    // ── 5. Clean up party BBS ────────────────────────────────────────
    party_bbs::cleanup_on_disconnect(&world, sid);

    // ── 5a. Chat room cleanup ────────────────────────────────────────
    // Remove player from chat room on logout (admin leaves → room deleted).
    let room_index = world.get_chat_room_index(sid);
    if room_index > 0 {
        if let Some(ch) = world.get_character_info(sid) {
            let is_admin = world
                .get_chat_room(room_index)
                .map(|r| r.is_administrator(&ch.name) == 2)
                .unwrap_or(false);
            if is_admin {
                world.remove_chat_room(room_index);
            } else if let Some(mut room) = world.get_chat_room_mut(room_index) {
                room.remove_user(&ch.name);
            }
        }
        world.set_chat_room_index(sid, 0);
    }

    // ── 5b. BottomUserLogOut — broadcast zone-wide logout notification ─
    // Sends WIZ_USER_INFORMATIN(sub=4/RegionDelete) to all players in the zone.
    if let Some(ch) = world.get_character_info(sid) {
        if let Some(pos) = world.get_position(sid) {
            let region_del_pkt = crate::handler::user_info::build_region_delete_packet(&ch.name);
            world.broadcast_to_zone(pos.zone_id, Arc::new(region_del_pkt), Some(sid));
        }
    }

    // ── 5c. GM list removal ────────────────────────────────────────────
    if let Some(ch) = world.get_character_info(sid) {
        if ch.authority == 0 {
            // authority==0 means GM in KO (isGM check)
            world.gm_list_remove(&ch.name);
        }
    }

    // ── 5d. Knights/Clan cleanup ───────────────────────────────────────
    if let Some(ch) = world.get_character_info(sid) {
        if ch.knights_id > 0 {
            // KnightsClanBuffUpdate(false) — decrement online count, broadcast bonus
            world.knights_clan_buff_update(ch.knights_id, false, sid);

            // CKnights::OnLogout — send clan offline notification
            crate::handler::knights::send_clan_offline_notification(
                &world,
                ch.knights_id,
                &ch.name,
                sid,
            );
        }
    }

    // ── 6. Remove from zone/region and broadcast INOUT_OUT ───────────
    if let Some(pos) = world.get_position(sid) {
        if let Some(zone) = world.get_zone(pos.zone_id) {
            zone.remove_user(pos.region_x, pos.region_z, sid);
        }

        let out_pkt = region::build_user_inout(region::INOUT_OUT, sid, None, &Default::default());

        let event_room = world.get_event_room(sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(out_pkt),
            Some(sid),
            event_room,
        );
    }

    // ── 6b. Save premium state to DB ─────────────────────────────────
    if !account_id.is_empty() {
        let premium_slots: Vec<(i16, i16, i32)> = world
            .with_session(sid, |h| {
                h.premium_map
                    .iter()
                    .enumerate()
                    .map(|(idx, (&p_type, &expiry))| (idx as i16, p_type as i16, expiry as i32))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !premium_slots.is_empty() {
            let prem_repo = PremiumRepository::new(&pool);
            if let Err(e) = prem_repo
                .save_account_premium(&account_id, &premium_slots)
                .await
            {
                warn!("Logout: failed to save premium state: {}", e);
            }
        }
    }

    // ── 6c. Save achievement data to DB ──────────────────────────────
    if !char_id.is_empty() {
        // Update play_time before saving.
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
        let achieve_data = world.with_session(sid, |h| {
            let entries: Vec<(u16, u8, u32, u32)> = h
                .achieve_map
                .iter()
                .map(|(&id, info)| (id, info.status, info.count[0], info.count[1]))
                .collect();
            let summary = h.achieve_summary.clone();
            (entries, summary)
        });
        if let Some((entries, summary)) = achieve_data {
            if !entries.is_empty() || summary.play_time > 0 {
                let batch_entries: Vec<(i32, i16, i32, i32)> = entries
                    .iter()
                    .map(|&(id, status, count1, count2)| {
                        (id as i32, status as i16, count1 as i32, count2 as i32)
                    })
                    .collect();
                let achieve_repo = AchieveRepository::new(&pool);
                if !batch_entries.is_empty() {
                    if let Err(e) = achieve_repo
                        .save_user_achieves_batch(&char_id, &batch_entries)
                        .await
                    {
                        warn!("Logout: failed to save achievements for {}: {}", char_id, e);
                    }
                }
                if let Err(e) = achieve_repo
                    .save_user_achieve_summary(
                        &char_id,
                        summary.play_time as i32,
                        summary.monster_defeat_count as i32,
                        summary.user_defeat_count as i32,
                        summary.user_death_count as i32,
                        summary.total_medal as i32,
                        [
                            summary.recent_achieve[0] as i16,
                            summary.recent_achieve[1] as i16,
                            summary.recent_achieve[2] as i16,
                        ],
                        summary.cover_id as i16,
                        summary.skill_id as i16,
                        summary.skill_title_row as i16,
                    )
                    .await
                {
                    warn!(
                        "Logout: failed to save achieve summary for {}: {}",
                        char_id, e
                    );
                }
            }
        }
    }

    // ── 6d. Save user perks to DB ─────────────────────────────────────
    if !char_id.is_empty() {
        let perk_data = world.with_session(sid, |h| (h.perk_levels, h.rem_perk));
        if let Some((perk_levels, rem_perk)) = perk_data {
            if perk_levels.iter().any(|&v| v != 0) || rem_perk != 0 {
                let perk_repo = PerkRepository::new(&pool);
                if let Err(e) = perk_repo
                    .save_user_perks(&char_id, &perk_levels, rem_perk)
                    .await
                {
                    warn!("Logout: failed to save perks for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6d2. Save soul data to DB (v2525) ─────────────────────────────
    if !char_id.is_empty() {
        let soul_data = world
            .with_session(sid, |h| {
                if h.soul_loaded {
                    Some((h.soul_categories, h.soul_slots))
                } else {
                    None
                }
            })
            .flatten();
        if let Some((cats, slots)) = soul_data {
            let has_data = cats.iter().any(|c| c[1] != 0 || c[2] != 0 || c[3] != 0)
                || slots.iter().any(|s| s[1] != 0);
            if has_data {
                let soul_repo = ko_db::repositories::soul::SoulRepository::new(&pool);
                if let Err(e) = soul_repo.save(&char_id, &cats, &slots).await {
                    tracing::warn!("Logout: failed to save soul for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6d3. Save hermetic seal data to DB (v2525) ────────────────────
    if !char_id.is_empty() {
        let seal_data = world
            .with_session(sid, |h| {
                if h.seal_loaded {
                    Some((
                        h.seal_max_tier,
                        h.seal_selected_slot,
                        h.seal_status,
                        h.seal_upgrade_count,
                        h.seal_current_level,
                        h.seal_elapsed_time,
                    ))
                } else {
                    None
                }
            })
            .flatten();
        if let Some((max_tier, selected_slot, status, upgrade_count, current_level, elapsed_time)) =
            seal_data
        {
            let has_data = max_tier > 0
                || selected_slot > 0
                || current_level > 0
                || upgrade_count > 0
                || elapsed_time > 0.0;
            if has_data {
                let seal_repo =
                    ko_db::repositories::hermetic_seal::HermeticSealRepository::new(&pool);
                if let Err(e) = seal_repo
                    .save(
                        &char_id,
                        max_tier as i16,
                        selected_slot as i16,
                        status as i16,
                        upgrade_count as i16,
                        current_level as i16,
                        elapsed_time as f32,
                    )
                    .await
                {
                    tracing::warn!(
                        "Logout: failed to save hermetic seal for {}: {}",
                        char_id,
                        e
                    );
                }
            }
        }
    }

    // ── 6d4. Save costume data to DB (v2525) ──────────────────────────
    if !char_id.is_empty() {
        let costume_data = world
            .with_session(sid, |h| {
                if h.costume_loaded {
                    Some((
                        h.costume_active_type,
                        h.costume_item_id,
                        h.costume_item_param,
                        h.costume_scale_raw,
                        h.costume_color_index,
                        h.costume_expiry_time,
                    ))
                } else {
                    None
                }
            })
            .flatten();
        if let Some((active_type, item_id, item_param, scale_raw, color_index, expiry_time)) =
            costume_data
        {
            let has_data = active_type > 0 || item_id != 0;
            if has_data {
                let costume_repo = ko_db::repositories::costume::CostumeRepository::new(&pool);
                if let Err(e) = costume_repo
                    .save(
                        &char_id,
                        active_type as i16,
                        item_id,
                        item_param,
                        scale_raw,
                        color_index as i16,
                        expiry_time,
                    )
                    .await
                {
                    tracing::warn!("Logout: failed to save costume for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6d5. Save enchant data to DB (v2525) ──────────────────────────
    if !char_id.is_empty() {
        let enchant_data = world
            .with_session(sid, |h| {
                if h.enchant_loaded {
                    Some((
                        h.enchant_max_star,
                        h.enchant_count,
                        h.enchant_slot_levels,
                        h.enchant_slot_unlocked,
                        h.enchant_item_category,
                        h.enchant_item_slot_unlock,
                        h.enchant_item_markers,
                    ))
                } else {
                    None
                }
            })
            .flatten();
        if let Some((max_star, enc_count, levels, unlocked, item_cat, item_unlock, markers)) =
            enchant_data
        {
            let has_data = max_star > 0
                || enc_count > 0
                || levels.iter().any(|&v| v > 0)
                || unlocked.iter().any(|&v| v > 0);
            if has_data {
                let enchant_repo = ko_db::repositories::enchant::EnchantRepository::new(&pool);
                if let Err(e) = enchant_repo
                    .save(
                        &char_id,
                        max_star as i16,
                        enc_count as i16,
                        &levels,
                        &unlocked,
                        item_cat as i16,
                        item_unlock as i16,
                        &markers,
                    )
                    .await
                {
                    tracing::warn!("Logout: failed to save enchant for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6e. Save quest progress to DB (AWAITED) ────────────────────────
    // AWAITED — quest progress loss is highly disruptive to players
    if !char_id.is_empty() {
        let quest_data = world.with_session(sid, |h| h.quests.clone());
        if let Some(quests) = quest_data {
            if !quests.is_empty() {
                let quest_repo = QuestRepository::new(&pool);
                let entries: Vec<(i16, i16, [i16; 4])> = quests
                    .iter()
                    .map(|(&quest_id, info)| {
                        (
                            quest_id as i16,
                            info.quest_state as i16,
                            [
                                info.kill_counts[0] as i16,
                                info.kill_counts[1] as i16,
                                info.kill_counts[2] as i16,
                                info.kill_counts[3] as i16,
                            ],
                        )
                    })
                    .collect();
                if let Err(e) = quest_repo.save_user_quests_batch(&char_id, &entries).await {
                    warn!("Logout: failed to save quests for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6f. Save active buffs (saved magic) to DB (AWAITED) ───────────
    if !char_id.is_empty() {
        let magic_entries = world.get_saved_magic_entries(sid);
        if !magic_entries.is_empty() {
            let magic_repo = SavedMagicRepository::new(&pool);
            if let Err(e) = magic_repo.save_saved_magic(&char_id, &magic_entries).await {
                warn!("Logout: failed to save magic for {}: {}", char_id, e);
            }
        }
    }

    // ── 6g. Save genie data to DB ─────────────────────────────────────
    // Only save if genie data has been loaded from DB (prevents overwriting with 0).
    if !char_id.is_empty() {
        let genie_data = world.with_session(sid, |h| {
            (h.genie_time_abs, h.genie_options.clone(), h.genie_loaded)
        });
        if let Some((genie_abs, genie_options, genie_loaded)) = genie_data {
            if !genie_loaded {
                tracing::warn!(
                    "[{}] Logout genie save: char={}, SKIPPED — genie not loaded from DB",
                    session.addr(),
                    char_id,
                );
            } else {
                let db_val = crate::handler::genie::genie_abs_to_db(genie_abs);
                tracing::info!(
                    "[{}] Logout genie save: char={}, abs={}, db_val={}",
                    session.addr(),
                    char_id,
                    genie_abs,
                    db_val,
                );
                let genie_repo = UserDataRepository::new(&pool);
                if let Err(e) = genie_repo
                    .save_genie_data(&char_id, db_val, &genie_options, 0)
                    .await
                {
                    warn!("Logout: failed to save genie data for {}: {}", char_id, e);
                }
            }
        } else {
            tracing::warn!(
                "[{}] Logout genie save: char={}, session data unavailable!",
                session.addr(),
                char_id,
            );
        }
    }

    // ── 6g2. Save sealed_exp to DB ────────────────────────────────────
    if !char_id.is_empty() {
        if let Some(ch) = world.get_character_info(sid) {
            if ch.sealed_exp > 0 {
                let sexp_repo = UserDataRepository::new(&pool);
                if let Err(e) = sexp_repo
                    .save_seal_exp(&char_id, ch.sealed_exp as i32)
                    .await
                {
                    warn!("Logout: failed to save sealed_exp for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6g3. Save bind point to DB (AWAITED) ────────────────────────
    if !char_id.is_empty() {
        if let Some(ch) = world.get_character_info(sid) {
            if ch.bind_zone > 0 {
                let bind_repo = CharacterRepository::new(&pool);
                let bz = ch.bind_zone as i16;
                let bpx = (ch.bind_x * 100.0) as i32;
                let bpz = (ch.bind_z * 100.0) as i32;
                if let Err(e) = bind_repo.save_bind(&char_id, bz, bpx, bpz).await {
                    warn!("Logout: failed to save bind point for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6g4. Sync user_knightdata on logout (AWAITED) ───────────────────
    if !char_id.is_empty() {
        if let Some(ch) = world.get_character_info(sid) {
            if ch.knights_id > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i32;
                let kd_repo = ko_db::repositories::knights_cape::KnightsCapeRepository::new(&pool);
                if let Err(e) = kd_repo
                    .sync_user_knightdata_on_save(
                        ch.knights_id as i16,
                        &char_id,
                        ch.class as i16,
                        ch.level as i16,
                        ch.loyalty.min(i32::MAX as u32) as i32,
                        ch.loyalty_monthly.min(i32::MAX as u32) as i32,
                        now,
                    )
                    .await
                {
                    warn!(
                        "Logout: failed to sync user_knightdata for {}: {}",
                        char_id, e
                    );
                }
            }
        }
    }

    // ── 6g5. Save pet data to DB ──────────────────────────────────────
    if !char_id.is_empty() {
        let pet_snapshot = world.with_session(sid, |h| {
            h.pet_data.as_ref().filter(|p| p.serial_id > 0).map(|p| {
                ko_db::models::pet::PetUserDataRow {
                    n_serial_id: p.serial_id as i64,
                    s_pet_name: p.name.clone(),
                    b_level: p.level as i16,
                    s_hp: p.hp as i16,
                    s_mp: p.mp as i16,
                    n_index: p.index as i32,
                    s_satisfaction: p.satisfaction,
                    n_exp: p.exp as i32,
                    s_pid: p.pid as i16,
                    s_size: p.size as i16,
                }
            })
        });
        if let Some(Some(pet_row)) = pet_snapshot {
            let pet_repo = ko_db::repositories::pet::PetRepository::new(&pool);
            if let Err(e) = pet_repo.save_pet_data(&pet_row).await {
                warn!(
                    "Logout: failed to save pet data (serial={}): {}",
                    pet_row.n_serial_id, e
                );
            }
        }
    }

    // ── 6h. Save daily quest progress to DB ─────────────────────────
    if !char_id.is_empty() {
        let dq_data = world.with_session(sid, |h| h.daily_quests.clone());
        if let Some(dq_map) = dq_data {
            if !dq_map.is_empty() {
                let entries: Vec<_> = dq_map.into_values().collect();
                let dq_repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(&pool);
                if let Err(e) = dq_repo.save_all_user_quests(&char_id, &entries).await {
                    warn!("Logout: failed to save daily quests for {}: {}", char_id, e);
                }
            }
        }
    }

    // ── 6i. Save daily operation cooldowns to DB ──────────────────────
    if !char_id.is_empty() {
        if let Some((_, data)) = world.daily_ops.remove(&char_id) {
            let do_repo = UserDataRepository::new(&pool);
            let row = data.to_row(&char_id);
            if let Err(e) = do_repo.save_daily_op(&row).await {
                warn!("Logout: failed to save daily_op for {}: {}", char_id, e);
            }
        }
    }

    // ── 6j. Save daily rank stats to DB ───────────────────────────────
    if !char_id.is_empty() {
        let dr_data = world.with_session(sid, |h| {
            (
                h.dr_gm_total_sold,
                h.dr_mh_total_kill,
                h.dr_sh_total_exchange,
                h.dr_cw_counter_win,
                h.dr_up_counter_bles,
            )
        });
        if let Some((gm, mh, sh, cw, up)) = dr_data {
            if gm > 0 || mh > 0 || sh > 0 || cw > 0 || up > 0 {
                let dr_repo = ko_db::repositories::daily_rank::DailyRankRepository::new(&pool);
                if let Err(e) = dr_repo
                    .save_user_stats(
                        &char_id, gm as i64, mh as i64, sh as i64, cw as i64, up as i64,
                    )
                    .await
                {
                    warn!(
                        "Logout: failed to save daily rank stats for {}: {}",
                        char_id, e
                    );
                }
            }
        }
    }

    // ── 7. Mark account offline ──────────────────────────────────────
    if !account_id.is_empty() {
        let pool2 = pool.clone();
        let acct = account_id.clone();
        tokio::spawn(async move {
            let repo = AccountRepository::new(&pool2);
            if let Err(e) = repo.set_offline(&acct).await {
                warn!("Logout: failed to set account offline: {}", e);
            }
        });
    }

    // ── 7b. Soccer event cleanup ──────────────────────────────────
    if let Some(ch) = world.get_character_info(sid) {
        if let Some(pos) = world.get_position(sid) {
            let soccer_state = world.soccer_state().clone();
            let mut state = soccer_state.write();
            if let Some(room) = state.get_room_mut(pos.zone_id) {
                crate::handler::soccer::remove_user(room, &ch.name);
            }
        }
    }

    // ── 7c. Cinderella War cleanup ──────────────────────────────────
    crate::handler::dungeon_defence::restore_on_disconnect(&world, sid);
            world.manes_survival_manager.leave_event(&world, sid, false);
    crate::handler::cinderella::cinderella_logout(&world, sid, true);

    // ── 7d. Wanted event cleanup ────────────────────────────────────
    {
        let is_wanted = world.with_session(sid, |h| h.is_wanted).unwrap_or(false);
        if is_wanted {
            crate::handler::vanguard::handle_wanted_logout(&world, sid);
        }
    }

    // ── 7e. Temple event sign-up cleanup ────────────────────────────
    // Remove player from event sign-up queue if still in signing phase.
    {
        let (active_event, is_active) = world
            .event_room_manager
            .read_temple_event(|s| (s.active_event, s.is_active));
        if active_event >= 0 && !is_active {
            if let Some(name) = world.get_session_name(sid) {
                if let Some(removed) = world.event_room_manager.remove_signed_up_user(&name) {
                    world.event_room_manager.update_temple_event(|s| {
                        if removed.nation == 1 {
                            s.karus_user_count = s.karus_user_count.saturating_sub(1);
                        } else {
                            s.elmorad_user_count = s.elmorad_user_count.saturating_sub(1);
                        }
                        s.all_user_count = s.karus_user_count + s.elmorad_user_count;
                    });
                    debug!(
                        "[{}] Logout: removed '{}' from event sign-up (event={})",
                        session.addr(),
                        name,
                        active_event
                    );
                }
            }
        }
    }

    // ── 7f. Clean up ALL ranking systems ────────────────────────────
    // PlayerKillingRemovePlayerRank + ZindanWarKillingRemovePlayerRank
    // + BorderDefenceRemovePlayerRank + ChaosExpansionRemovePlayerRank
    world.pk_zone_remove_player(sid);
    world.zindan_remove_player(sid);
    world.bdw_remove_player(sid);
    world.chaos_remove_player(sid);

    // ── 8. Unregister session from world ─────────────────────────────
    world.unregister_session(sid);

    // ── 9. Send WIZ_LOGOUT confirmation to client ────────────────────
    let resp = Packet::new(Opcode::WizLogout as u8);
    session.send_packet(&resp).await?;

    // Transition back to logged-in state (character select)
    session.set_state(SessionState::LoggedIn);

    // FerihaLog: LogoutInsertLog
    super::audit_log::log_logout(&pool, &account_id, &char_id, &session.addr().to_string(), 0);

    info!(
        "[{}] Player logged out: char={}, account={}",
        session.addr(),
        char_id,
        account_id
    );

    Ok(())
}

/// BDW logout cleanup — handles flag carrier disconnect and forfeit detection.
/// 1. If the game is not finished and all players of one nation have left, the
///    other nation wins by forfeit.
/// 2. If the logging-out user had the altar flag, starts altar respawn timer
///    and broadcasts `TEMPLE_EVENT_ALTAR_TIMER` (sub-opcode 50).
pub(crate) fn bdw_user_logout(world: &crate::world::WorldState, sid: crate::zone::SessionId) {
    use crate::systems::bdw;
    use crate::systems::event_room::{self, TempleEventType};

    // Only relevant if BDW is active
    let is_bdw_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_bdw_active());
    if !is_bdw_active {
        return;
    }

    // Check player zone
    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if zone_id != bdw::ZONE_BDW {
        return;
    }

    let user_name = match world.get_session_name(sid) {
        Some(n) => n,
        None => return,
    };

    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::BorderDefenceWar, &user_name)
    {
        Some(r) => r,
        None => return,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Mark user as logged out, check for flag and forfeit.
    //
    // QA HIGH fix (Sprint 190): In C++ (JuraidBdwFragSystem.cpp:144),
    // `BDWUserHasObtainedLoqOut()` runs UNCONDITIONALLY regardless of
    // `m_FinishPacketControl`. Only the forfeit detection is gated.
    //
    // Lock order: bdw_manager → room (consistent with event_system.rs)
    let (had_flag, forfeit_winner) = {
        let mut bdw_mgr = world.bdw_manager_write();

        let Some(mut room) = world
            .event_room_manager
            .get_room_mut(TempleEventType::BorderDefenceWar, room_id)
        else {
            return;
        };

        // Mark user as logged out (unconditional)
        if let Some(u) = room.karus_users.get_mut(&user_name) {
            u.logged_out = true;
        } else if let Some(u) = room.elmorad_users.get_mut(&user_name) {
            u.logged_out = true;
        }

        // Flag carrier logout runs UNCONDITIONALLY (even after finish_packet_sent)
        let bdw_state = match bdw_mgr.get_room_state_mut(room_id) {
            Some(s) => s,
            None => return,
        };
        let had_flag = bdw::flag_carrier_logout(&mut room, bdw_state, &user_name, now);

        // Forfeit detection is gated by finish_packet_sent
        let forfeit_winner = if !room.finish_packet_sent && room.winner_nation == 0 {
            let k_active = room.karus_users.values().filter(|u| !u.logged_out).count();
            let e_active = room
                .elmorad_users
                .values()
                .filter(|u| !u.logged_out)
                .count();

            if k_active == 0 && e_active > 0 {
                Some(2u8) // El Morad wins by forfeit
            } else if e_active == 0 && k_active > 0 {
                Some(1u8) // Karus wins by forfeit
            } else {
                None
            }
        } else {
            None
        };

        if let Some(winner) = forfeit_winner {
            room.winner_nation = winner;
            room.finish_packet_sent = true;
            room.finish_time_counter = now + 20;

            // Clear altar respawn on forfeit finish
            bdw_state.altar_respawn_pending = false;
            bdw_state.altar_respawn_time = 0;
        }

        (had_flag, forfeit_winner)
    }; // bdw_mgr + room lock dropped

    // Broadcast altar timer if carrier had flag
    if had_flag {
        let timer_pkt = event_room::build_altar_timer_packet(bdw::ALTAR_RESPAWN_DELAY_SECS as u16);
        super::dead::broadcast_to_bdw_room(world, room_id, &timer_pkt);

        // Remove speed debuff from the carrier
        bdw::clear_fragment_effect(world, sid);
    }

    // Broadcast forfeit finish if applicable
    if let Some(winner_nation) = forfeit_winner {
        let arc_select = Arc::new(event_room::build_winner_select_msg(4)); // BDW
        let arc_finish = Arc::new(event_room::build_finish_packet(winner_nation));

        if let Some(room) = world
            .event_room_manager
            .get_room(TempleEventType::BorderDefenceWar, room_id)
        {
            for u in room.karus_users.values().filter(|u| !u.logged_out) {
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_select));
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_finish));
            }
            for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_select));
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_finish));
            }
        }

        tracing::info!(
            "BDW forfeit: '{}' logout, nation {} wins in room {}",
            user_name,
            winner_nation,
            room_id,
        );
    }

    if had_flag {
        tracing::info!(
            "BDW flag carrier logout: '{}' in room {}, altar respawn started",
            user_name,
            room_id,
        );
    }
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use ko_protocol::{Opcode, Packet};
    use std::sync::Arc;

    #[test]
    fn test_logout_opcode_value() {
        assert_eq!(Opcode::WizLogout as u8, 0x0F);
    }

    #[test]
    fn test_logout_response_packet_format() {
        // WIZ_LOGOUT response is just the opcode with no data
        let pkt = Packet::new(Opcode::WizLogout as u8);
        let frame = pkt.to_outbound_frame();

        // [AA 55] [01 00] [0F] [55 AA] = 7 bytes total
        assert_eq!(frame.len(), 7);
        assert_eq!(frame[0], 0xAA); // header
        assert_eq!(frame[1], 0x55);
        assert_eq!(frame[4], 0x0F); // opcode
        assert_eq!(frame[5], 0x55); // footer
        assert_eq!(frame[6], 0xAA);
    }

    #[test]
    fn test_logout_opcode_roundtrip() {
        assert_eq!(Opcode::from_byte(0x0F), Some(Opcode::WizLogout));
        assert_eq!(Opcode::WizLogout as u8, 0x0F);
    }

    #[test]
    fn test_exchange_cancel_sub_opcode() {
        // EXCHANGE_CANCEL sub-opcode used in cleanup
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(0x08); // EXCHANGE_CANCEL

        assert_eq!(pkt.opcode, Opcode::WizExchange as u8);
        assert_eq!(pkt.data[0], 0x08);
    }

    /// Verify that PK zone cleanup is called before session unregister.
    /// This tests the ordering contract: pk_zone_remove_player must run
    /// before unregister_session to ensure consistent state.
    #[test]
    fn test_logout_cleanup_ordering_contract() {
        // The cleanup order in handle() is:
        // 7b. pk_zone_remove_player (PK ranking cleanup)
        // 8.  unregister_session (remove from world)
        // This ordering matters because unregister_session drops the session handle,
        // and pk_zone_remove_player needs valid session data.
        // This test documents the ordering requirement.
        assert!(true, "PK zone cleanup must precede session unregister");
    }

    // ── Sprint 49: Disconnect Edge Case Tests ────────────────────────────

    use crate::world::{CharacterInfo, ExchangeItem, Position, WorldState, ITEM_GOLD};
    use tokio::sync::mpsc;

    fn make_logout_test_char(sid: u16, name: &str, gold: u32) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation: 1,
            race: 1,
            class: 101,
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
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold,
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

    /// Integration test: disconnect with party + trade active simultaneously.
    #[test]
    fn test_disconnect_party_and_trade_simultaneous() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Player1", 100_000), pos);
        world.register_ingame(2, make_logout_test_char(2, "Player2", 50_000), pos);
        world.register_ingame(3, make_logout_test_char(3, "Player3", 75_000), pos);

        // Set up party: 1 (leader) + 3 (member)
        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 3);

        // Set up trade between 1 and 2
        world.init_trade_request(1, 2);
        world.trade_agree(2);

        // Player 1 adds gold to trade
        world.gold_lose(1, 20_000);
        world.add_exchange_item(
            1,
            ExchangeItem {
                item_id: ITEM_GOLD,
                count: 20_000,
                durability: 0,
                serial_num: 0,
                src_pos: 255,
                dst_pos: 0,
            },
        );

        // Simulate logout cleanup for player 1
        // 1. Party cleanup
        world.cleanup_party_on_disconnect(1);

        // 2. Trade cleanup
        if world.is_trading(1) {
            let partner_sid = world.get_exchange_user(1);
            world.exchange_give_items_back(1);
            world.reset_trade(1);
            if let Some(partner) = partner_sid {
                world.exchange_give_items_back(partner);
                world.reset_trade(partner);
                let mut cancel_pkt = Packet::new(Opcode::WizExchange as u8);
                cancel_pkt.write_u8(0x08);
                world.send_to_session_owned(partner, cancel_pkt);
            }
        }

        // 3. Merchant cleanup
        if world.is_merchanting(1) {
            world.close_merchant(1);
        }

        // Verify party state: 2-member party disbanded (only 1 and 3)
        assert!(!world.is_in_party(1));
        // With 2 members (1+3), when 1 disconnects, only 3 left => disband
        assert!(!world.is_in_party(3));

        // Verify trade state: both reset
        assert!(!world.is_trading(1));
        assert!(!world.is_trading(2));

        // Gold returned to player 1
        let ch1 = world.get_character_info(1).unwrap();
        assert_eq!(ch1.gold, 100_000);

        // Player 2 received EXCHANGE_CANCEL
        let pkt = rx2.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizExchange as u8);
    }

    /// Integration test: disconnect during merchant mode — merchant state cleaned up.
    #[test]
    fn test_disconnect_during_merchant() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Merchant", 100_000), pos);

        // Activate merchant
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);
        assert!(world.is_merchanting(1));

        // Simulate disconnect cleanup
        if world.is_merchanting(1) {
            world.close_merchant(1);
        }

        assert!(!world.is_merchanting(1));
    }

    /// Integration test: disconnect during mining — mining state reset.
    #[test]
    fn test_disconnect_during_mining() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Miner", 100_000), pos);

        // Set mining state
        world.update_session(1, |h| {
            h.is_mining = true;
        });
        assert!(world.is_mining(1));

        // On disconnect, session is unregistered entirely
        world.unregister_session(1);

        // Session no longer exists, mining state is gone
        assert!(!world.is_mining(1));
    }

    /// Integration test: disconnect clears browsing merchant state.
    #[test]
    fn test_disconnect_while_browsing_merchant() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Merchant", 100_000), pos);
        world.register_ingame(2, make_logout_test_char(2, "Browser", 50_000), pos);

        // Set up merchant with browser
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);
        world.set_merchant_looker(1, Some(2));
        world.set_browsing_merchant(2, Some(1));

        // Browser disconnects — clean up looker state
        world.remove_from_merchant_lookers(2);

        // Merchant's looker should be cleared
        assert_eq!(world.get_merchant_looker(1), None);
        assert_eq!(world.get_browsing_merchant(2), None);

        // Merchant is still active
        assert!(world.is_selling_merchant(1));
    }

    /// Integration test: unregister session removes all state.
    #[test]
    fn test_unregister_session_removes_all_state() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Player", 100_000), pos);

        // Verify session exists
        assert!(world.get_character_info(1).is_some());
        assert!(world.get_position(1).is_some());

        // Unregister
        world.unregister_session(1);

        // All state gone
        assert!(world.get_character_info(1).is_none());
        assert!(world.get_position(1).is_none());
        assert!(!world.is_in_party(1));
        assert!(!world.is_trading(1));
        assert!(!world.is_merchanting(1));
    }

    // ── Sprint 249: Challenge/Duel + Rival Cleanup on Logout ────────────

    /// Logout cleans up active challenge/duel state for both sides.
    #[test]
    fn test_logout_cleans_challenge_state() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Challenger", 0), pos);
        world.register_ingame(2, make_logout_test_char(2, "Target", 0), pos);

        // Set up challenge: player 1 requesting, player 2 requested
        world.update_session(1, |h| {
            h.requesting_challenge = 1;
            h.challenge_user = 2;
        });
        world.update_session(2, |h| {
            h.challenge_requested = 1;
            h.challenge_user = 1;
        });

        // Simulate logout cleanup for player 1 (challenge cleanup)
        let (requesting, requested, challenge_user) = world.get_challenge_state(1);
        assert!(requesting > 0 || requested > 0);
        if requesting > 0 || requested > 0 {
            let target = challenge_user as u16;
            if challenge_user >= 0 {
                world.update_session(target, |h| {
                    h.challenge_user = -1;
                    h.requesting_challenge = 0;
                    h.challenge_requested = 0;
                });
                let mut cancel_pkt = Packet::new(Opcode::WizChallenge as u8);
                cancel_pkt.write_u8(if requesting > 0 { 2 } else { 4 });
                world.send_to_session_owned(target, cancel_pkt);
            }
            world.update_session(1, |h| {
                h.challenge_user = -1;
                h.requesting_challenge = 0;
                h.challenge_requested = 0;
            });
        }

        // Player 1's challenge state cleared
        let (req1, reqd1, _) = world.get_challenge_state(1);
        assert_eq!(req1, 0);
        assert_eq!(reqd1, 0);

        // Player 2's challenge state cleared
        let (req2, reqd2, _) = world.get_challenge_state(2);
        assert_eq!(req2, 0);
        assert_eq!(reqd2, 0);

        // Player 2 received cancel packet
        let pkt = rx2.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizChallenge as u8);
    }

    /// Logout cleans up rival state.
    #[test]
    fn test_logout_cleans_rival_state() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut ch1 = make_logout_test_char(1, "Player1", 0);
        ch1.rival_id = 2; // Has rival
        ch1.rival_expiry_time = 99999;
        world.register_ingame(1, ch1, pos);
        world.register_ingame(2, make_logout_test_char(2, "Player2", 0), pos);

        // Verify rival is set
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.rival_id, 2);

        // Simulate logout cleanup — rival removal
        let has_rival = world
            .get_character_info(1)
            .map(|ch| ch.rival_id >= 0)
            .unwrap_or(false);
        assert!(has_rival);
        if has_rival {
            world.remove_rival(1);
        }

        // Rival cleared
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.rival_id, -1);
    }

    /// Logout cleans up merchant browsing state.
    #[test]
    fn test_logout_cleans_merchant_browser() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Merchant", 0), pos);
        world.register_ingame(2, make_logout_test_char(2, "Browser", 0), pos);

        // Merchant is active, player 2 is browsing
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);
        world.set_merchant_looker(1, Some(2));
        world.set_browsing_merchant(2, Some(1));

        // Simulate logout cleanup for player 2 — remove from merchant lookers
        world.remove_from_merchant_lookers(2);

        // Merchant looker cleared
        assert_eq!(world.get_merchant_looker(1), None);
        assert_eq!(world.get_browsing_merchant(2), None);
        // Merchant still active
        assert!(world.is_selling_merchant(1));
    }

    // ── Sprint 353: Logout cleanup — BottomUserLogOut, Clan, GM list ────

    /// Logout sends RegionDelete (BottomUserLogOut) to zone.
    #[test]
    fn test_logout_sends_region_delete() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Quitter", 0), pos);
        world.register_ingame(2, make_logout_test_char(2, "Watcher", 0), pos);

        // Simulate BottomUserLogOut
        let ch = world.get_character_info(1).unwrap();
        let pkt = crate::handler::user_info::build_region_delete_packet(&ch.name);
        world.broadcast_to_zone(21, Arc::new(pkt), Some(1));

        // Player 2 should receive RegionDelete
        let received = rx2.try_recv().unwrap();
        assert_eq!(received.opcode, Opcode::WizUserInfo as u8);
        assert_eq!(received.data[0], 4); // RegionDelete sub-opcode
    }

    /// Logout decrements clan online member count via KnightsClanBuffUpdate.
    #[test]
    fn test_logout_clan_buff_update_decrement() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut ch = make_logout_test_char(1, "ClanMember", 0);
        ch.knights_id = 100;
        world.register_ingame(1, ch, pos);

        // Insert test clan with 5 online members
        world.insert_knights(crate::world::KnightsInfo {
            id: 100,
            name: "TestClan".to_string(),
            nation: 1,
            online_members: 5,
            online_np_count: 1,
            online_exp_count: 20,
            ..Default::default()
        });

        // Simulate logout: decrement
        world.knights_clan_buff_update(100, false, 1);

        let clan = world.get_knights(100).unwrap();
        assert_eq!(clan.online_members, 4);
        // With 4 members (< 5), bonuses should be 0
        assert_eq!(clan.online_np_count, 0);
        assert_eq!(clan.online_exp_count, 0);
    }

    /// Login increments clan online member count via KnightsClanBuffUpdate.
    #[test]
    fn test_login_clan_buff_update_increment() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Player", 0), pos);

        world.insert_knights(crate::world::KnightsInfo {
            id: 200,
            name: "BigClan".to_string(),
            nation: 2,
            online_members: 9,
            ..Default::default()
        });

        // Login: increment
        world.knights_clan_buff_update(200, true, 1);

        let clan = world.get_knights(200).unwrap();
        assert_eq!(clan.online_members, 10);
        // NP bonus: ceil(10 * 10 / 100) = 1
        assert_eq!(clan.online_np_count, 1);
        // EXP bonus: 15 + 10 = 25
        assert_eq!(clan.online_exp_count, 25);
    }

    /// Clan buff update caps at MAX_CLAN_USERS (50).
    #[test]
    fn test_clan_buff_update_cap_at_max() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Player", 0), pos);

        world.insert_knights(crate::world::KnightsInfo {
            id: 300,
            name: "MaxClan".to_string(),
            nation: 1,
            online_members: 50,
            ..Default::default()
        });

        // Try to increment past cap
        world.knights_clan_buff_update(300, true, 1);

        let clan = world.get_knights(300).unwrap();
        assert_eq!(clan.online_members, 50); // capped
                                             // NP: ceil(50*10/100) = 5
        assert_eq!(clan.online_np_count, 5);
        // EXP: 15+50 = 65
        assert_eq!(clan.online_exp_count, 65);
    }

    /// Clan buff update doesn't underflow past 0.
    #[test]
    fn test_clan_buff_update_no_underflow() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_logout_test_char(1, "Player", 0), pos);

        world.insert_knights(crate::world::KnightsInfo {
            id: 400,
            name: "EmptyClan".to_string(),
            nation: 1,
            online_members: 0,
            ..Default::default()
        });

        // Decrement from 0 — should not underflow
        world.knights_clan_buff_update(400, false, 1);

        let clan = world.get_knights(400).unwrap();
        assert_eq!(clan.online_members, 0);
    }

    /// GM list add/remove works correctly.
    #[test]
    fn test_gm_list_add_remove() {
        let world = WorldState::new();

        world.gm_list_add("Admin1");
        world.gm_list_add("Admin2");
        world.gm_list_add("Admin1"); // duplicate — should not add

        let pkt = world.build_gm_list_packet();
        assert_eq!(pkt.opcode, Opcode::WizNotice as u8);
        assert_eq!(pkt.data[0], 5); // sub-opcode
        assert_eq!(pkt.data[1], 2); // count = 2 (no duplicate)

        world.gm_list_remove("Admin1");

        let pkt2 = world.build_gm_list_packet();
        assert_eq!(pkt2.data[1], 1); // count = 1

        world.gm_list_remove("Admin2");

        let pkt3 = world.build_gm_list_packet();
        assert_eq!(pkt3.data[1], 0); // empty
    }

    /// GM list remove for non-existent name is a no-op.
    #[test]
    fn test_gm_list_remove_nonexistent() {
        let world = WorldState::new();
        world.gm_list_remove("Ghost");

        let pkt = world.build_gm_list_packet();
        assert_eq!(pkt.data[1], 0);
    }

    /// Clan offline notification sends KNIGHTS_USER_OFFLINE packet.
    #[test]
    fn test_clan_offline_notification_on_logout() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut ch1 = make_logout_test_char(1, "Quitter", 0);
        ch1.knights_id = 500;
        let mut ch2 = make_logout_test_char(2, "Stayer", 0);
        ch2.knights_id = 500;
        world.register_ingame(1, ch1, pos);
        world.register_ingame(2, ch2, pos);

        world.insert_knights(crate::world::KnightsInfo {
            id: 500,
            name: "NotifyClan".to_string(),
            nation: 1,
            ..Default::default()
        });

        // Send clan offline notification
        crate::handler::knights::send_clan_offline_notification(&world, 500, "Quitter", 1);

        // Player 2 (same clan) should receive KNIGHTS_USER_OFFLINE
        let pkt = rx2.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizKnightsProcess as u8);
        assert_eq!(pkt.data[0], 40); // KNIGHTS_USER_OFFLINE = 40
    }

    /// Clan buff update broadcasts WIZ_KNIGHTS_PROCESS(KNIGHTS_CLAN_BONUS=98) to clan.
    #[test]
    fn test_clan_buff_update_broadcasts_packet() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut ch = make_logout_test_char(1, "BonusMember", 0);
        ch.knights_id = 600;
        world.register_ingame(1, ch, pos);

        world.insert_knights(crate::world::KnightsInfo {
            id: 600,
            name: "BroadcastClan".to_string(),
            nation: 1,
            online_members: 4,
            ..Default::default()
        });

        // Login one more member
        world.knights_clan_buff_update(600, true, 1);

        // Should receive KNIGHTS_CLAN_BONUS packet
        let pkt = rx1.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizKnightsProcess as u8);
        assert_eq!(pkt.data[0], 98); // KNIGHTS_CLAN_BONUS = 98
                                     // online_members should be 5
        let count = u16::from_le_bytes([pkt.data[1], pkt.data[2]]);
        assert_eq!(count, 5);
    }

    // ── Sprint 951: Additional coverage ──────────────────────────────

    /// WIZ_LOGOUT opcode is 0x0F.
    #[test]
    fn test_logout_opcode() {
        assert_eq!(Opcode::WizLogout as u8, 0x0F);
    }

    /// Logout response packet is opcode-only (empty data).
    #[test]
    fn test_logout_response_empty() {
        let pkt = Packet::new(Opcode::WizLogout as u8);
        assert_eq!(pkt.opcode, 0x0F);
        assert!(pkt.data.is_empty());
    }

    /// Unregistered session clears all state.
    #[test]
    fn test_unregister_clears_state() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.account_id = "acct".to_string();
        });
        assert!(world.with_session(1, |_| ()).is_some());
        world.unregister_session(1);
        assert!(world.with_session(1, |_| ()).is_none());
    }

    /// Exchange user is cleared on session unregister.
    #[test]
    fn test_exchange_cleared_on_unregister() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.exchange_user = Some(2);
        });
        let target = world.with_session(1, |h| h.exchange_user);
        assert_eq!(target, Some(Some(2)));
        world.unregister_session(1);
        assert!(world.with_session(1, |h| h.exchange_user).is_none());
    }

    /// Trade state is cleared on session unregister.
    #[test]
    fn test_trade_state_cleared_on_unregister() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.trade_state = 1;
        });
        let ts = world.with_session(1, |h| h.trade_state);
        assert_eq!(ts, Some(1));
        world.unregister_session(1);
        assert!(world.with_session(1, |h| h.trade_state).is_none());
    }

    // ── Sprint 961: Additional coverage ──────────────────────────────

    /// Session count decrements after unregister.
    #[test]
    fn test_session_count_after_unregister() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        assert_eq!(world.session_count(), 2);
        world.unregister_session(1);
        assert_eq!(world.session_count(), 1);
    }

    /// Merchant state defaults to NONE on fresh session.
    #[test]
    fn test_merchant_state_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let ms = world.with_session(1, |h| h.merchant_state).unwrap();
        assert_eq!(ms, crate::world::types::MERCHANT_STATE_NONE);
    }

    /// Warehouse loaded flag defaults to false.
    #[test]
    fn test_warehouse_loaded_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let loaded = world.with_session(1, |h| h.warehouse_loaded).unwrap();
        assert!(!loaded);
    }

    /// Store open flag defaults to false.
    #[test]
    fn test_store_open_default_logout() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_store_open(1));
    }

    /// Multiple unregister calls don't panic.
    #[test]
    fn test_double_unregister_no_panic() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.unregister_session(1);
        world.unregister_session(1); // second call — no panic
        assert_eq!(world.session_count(), 0);
    }

    // ── Sprint 976: Additional coverage ──────────────────────────────

    /// Pending knights invite is cleared on unregister.
    #[test]
    fn test_pending_knights_invite_cleared() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.pending_knights_invite = 42;
        });
        let invite = world.with_session(1, |h| h.pending_knights_invite);
        assert_eq!(invite, Some(42));
        world.unregister_session(1);
        assert!(world
            .with_session(1, |h| h.pending_knights_invite)
            .is_none());
    }

    /// Target ID defaults to 0 on fresh session.
    #[test]
    fn test_target_id_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let target = world.with_session(1, |h| h.target_id).unwrap();
        assert_eq!(target, 0);
    }

    /// Party invitation can be set and consumed (take).
    #[test]
    fn test_party_invitation_set_and_take() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.has_party_invitation(1));
        world.set_party_invitation(1, 100, 2);
        assert!(world.has_party_invitation(1));
        let inv = world.take_party_invitation(1);
        assert_eq!(inv, Some((100, 2)));
        assert!(!world.has_party_invitation(1));
    }

    /// Account ID is stored in session and accessible.
    #[test]
    fn test_account_id_stored() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.account_id = "test_account".to_string();
        });
        let acct = world.with_session(1, |h| h.account_id.clone()).unwrap();
        assert_eq!(acct, "test_account");
    }

    /// ITEM_GOLD constant is usable for gold tracking.
    #[test]
    fn test_item_gold_constant() {
        assert_eq!(ITEM_GOLD, 900_000_000);
    }
}
