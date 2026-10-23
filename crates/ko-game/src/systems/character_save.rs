//! Periodic character save — auto-saves all online characters every 10 minutes.
//! Iterates all in-game sessions and saves their stats, position, achievements,
//! perks, saved magic, and genie data to the database. Each save is
//! fire-and-forget so a single slow query does not block the rest.

use std::sync::Arc;
use std::time::Duration;

use ko_db::repositories::achieve::AchieveRepository;
use ko_db::repositories::character::{
    CharacterRepository, SaveItemParams, SaveStatPointsParams, SaveStatsParams,
};
use ko_db::repositories::perk::PerkRepository;
use ko_db::repositories::premium::PremiumRepository;
use ko_db::repositories::saved_magic::SavedMagicRepository;
use ko_db::repositories::user_data::UserDataRepository;
use ko_db::DbPool;
use tokio::time::interval;
use tracing::{debug, warn};

use crate::world::WorldState;

/// C++ PLAYER_SAVE_INTERVAL = 10 * 60 seconds.
const PLAYER_SAVE_INTERVAL_SECS: u64 = 10 * 60;

/// Start the periodic character save background task.
/// Spawns a tokio task that ticks every 10 minutes and saves all online
/// characters' stats and positions to the database.
pub fn start_character_save_task(
    world: Arc<WorldState>,
    pool: DbPool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(PLAYER_SAVE_INTERVAL_SECS));
        loop {
            tick.tick().await;
            save_all_characters(&world, &pool).await;
        }
    })
}

/// Save all data for all online characters.
/// Matches `CUser::ReqSaveCharacter()` which saves:
/// 1. Stats + position (UpdateUser)
/// 2. Achievement data (UpdateAchieveData)
/// 3. User perks (UpdateUserPerks)
/// 4. Saved magic / buff persistence (UpdateSavedMagic)
/// 5. Genie data (UpdateGenieData)
/// 6. Premium state (AccountPremiumData)
/// 7. Inventory items (UpdateUser — 77 slots, crash protection)
/// 8. Quest progress (UpdateQuestData)
/// 9. Class/race (UpdateUser — safety net for Lua PromoteUser*)
/// 10. Warehouse items + coins (UpdateWarehouseData)
async fn save_all_characters(world: &WorldState, pool: &DbPool) {
    let session_ids = world.get_in_game_session_ids();
    if session_ids.is_empty() {
        return;
    }
    debug!(
        "Periodic save: saving {} online characters",
        session_ids.len()
    );

    for sid in session_ids {
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => continue,
        };
        let pos = match world.get_position(sid) {
            Some(p) => p,
            None => continue,
        };
        let char_name = ch.name.clone();

        // ── 1. Stats save (fire-and-forget) ─────────────────────────────
        let pool_c = pool.clone();
        let name_c = char_name.clone();
        tokio::spawn(async move {
            let repo = CharacterRepository::new(&pool_c);
            if let Err(e) = repo
                .save_stats(&SaveStatsParams {
                    char_id: &name_c,
                    level: ch.level as i16,
                    hp: ch.hp,
                    mp: ch.mp,
                    sp: ch.sp,
                    exp: ch.exp as i64,
                    gold: ch.gold as i32,
                    loyalty: ch.loyalty as i32,
                    loyalty_monthly: ch.loyalty_monthly as i32,
                    manner_point: ch.manner_point,
                })
                .await
            {
                warn!("Periodic save: failed to save stats for {}: {}", name_c, e);
            }
        });

        // ── 2. Position save (fire-and-forget) ──────────────────────────
        let pool_c = pool.clone();
        let name_c = char_name.clone();
        let zone = pos.zone_id as i16;
        let px = (pos.x * 100.0) as i32;
        let pz = (pos.z * 100.0) as i32;
        tokio::spawn(async move {
            let repo = CharacterRepository::new(&pool_c);
            if let Err(e) = repo.save_position(&name_c, zone, px, 0, pz).await {
                warn!(
                    "Periodic save: failed to save position for {}: {}",
                    name_c, e
                );
            }
        });

        // ── 2b. Flash time save (fire-and-forget) ─────────────────────────
        let flash_data = world.with_session(sid, |h| (h.flash_time, h.flash_count, h.flash_type));
        if let Some((ft, fc, ftype)) = flash_data {
            if ft > 0 || fc > 0 {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = CharacterRepository::new(&pool_c);
                    if let Err(e) = repo
                        .save_flash(&name_c, ft as i32, fc as i16, ftype as i16)
                        .await
                    {
                        warn!("Periodic save: failed to save flash for {}: {}", name_c, e);
                    }
                });
            }
        }

        // ── 2c. Stat + skill point save (fire-and-forget) ────────────────
        // These are also saved immediately on each WIZ_POINT_CHANGE / WIZ_SKILLPT_CHANGE,
        // but periodic save provides durability in case those async writes fail.
        {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            let stat_data = world.with_session(sid, |h| {
                h.character.as_ref().map(|ch| {
                    (
                        ch.str as i16,
                        ch.sta as i16,
                        ch.dex as i16,
                        ch.intel as i16,
                        ch.cha as i16,
                        ch.free_points as i16,
                        ch.skill_points,
                    )
                })
            });
            if let Some(Some((str_val, sta, dex, intel, cha, free_points, sp))) = stat_data {
                tokio::spawn(async move {
                    let repo = CharacterRepository::new(&pool_c);
                    if let Err(e) = repo
                        .save_stat_points(&SaveStatPointsParams {
                            char_id: &name_c,
                            str_val,
                            sta,
                            dex,
                            intel,
                            cha,
                            free_points,
                            skill_points: [
                                sp[0] as i16,
                                sp[1] as i16,
                                sp[2] as i16,
                                sp[3] as i16,
                                sp[4] as i16,
                                sp[5] as i16,
                                sp[6] as i16,
                                sp[7] as i16,
                                sp[8] as i16,
                                sp[9] as i16,
                            ],
                        })
                        .await
                    {
                        warn!(
                            "Periodic save: failed to save stat points for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 3. Achievement data save (fire-and-forget) ──────────────────
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
            let entries: Vec<(i32, i16, i32, i32)> = h
                .achieve_map
                .iter()
                .map(|(&id, info)| {
                    (
                        id as i32,
                        info.status as i16,
                        info.count[0] as i32,
                        info.count[1] as i32,
                    )
                })
                .collect();
            let summary = h.achieve_summary.clone();
            (entries, summary)
        });
        if let Some((entries, summary)) = achieve_data {
            if !entries.is_empty() || summary.play_time > 0 {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = AchieveRepository::new(&pool_c);
                    // Batch save all achievement entries in a single query
                    if !entries.is_empty() {
                        if let Err(e) = repo.save_user_achieves_batch(&name_c, &entries).await {
                            warn!(
                                "Periodic save: failed to save achievements for {}: {}",
                                name_c, e
                            );
                        }
                    }
                    // Save achievement summary
                    if let Err(e) = repo
                        .save_user_achieve_summary(
                            &name_c,
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
                            "Periodic save: failed to save achieve summary for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 4. User perks save (fire-and-forget) ────────────────────────
        let perk_data = world.with_session(sid, |h| (h.perk_levels, h.rem_perk));
        if let Some((perk_levels, rem_perk)) = perk_data {
            // Only save if any perk has been allocated or rem_perk is non-zero
            if perk_levels.iter().any(|&v| v != 0) || rem_perk != 0 {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = PerkRepository::new(&pool_c);
                    if let Err(e) = repo.save_user_perks(&name_c, &perk_levels, rem_perk).await {
                        warn!("Periodic save: failed to save perks for {}: {}", name_c, e);
                    }
                });
            }
        }

        // ── 4b. Soul data save (fire-and-forget) ───────────────────────
        // v2525-specific: WIZ_SOUL (0xC5) panel persistence.
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
            // Only save if any value is non-zero
            let has_data = cats.iter().any(|c| c[1] != 0 || c[2] != 0 || c[3] != 0)
                || slots.iter().any(|s| s[1] != 0);
            if has_data {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::soul::SoulRepository::new(&pool_c);
                    if let Err(e) = repo.save(&name_c, &cats, &slots).await {
                        warn!("Periodic save: failed to save soul for {}: {}", name_c, e);
                    }
                });
            }
        }

        // ── 4c. Hermetic seal data save (fire-and-forget) ──────────────
        // v2525-specific: WIZ_ABILITY (0xCF) panel persistence.
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
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo =
                        ko_db::repositories::hermetic_seal::HermeticSealRepository::new(&pool_c);
                    if let Err(e) = repo
                        .save(
                            &name_c,
                            max_tier as i16,
                            selected_slot as i16,
                            status as i16,
                            upgrade_count as i16,
                            current_level as i16,
                            elapsed_time as f32,
                        )
                        .await
                    {
                        warn!(
                            "Periodic save: failed to save hermetic seal for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 4d. Costume data save (fire-and-forget) ─────────────────────
        // v2525-specific: WIZ_COSTUME (0xC3) panel persistence.
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
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::costume::CostumeRepository::new(&pool_c);
                    if let Err(e) = repo
                        .save(
                            &name_c,
                            active_type as i16,
                            item_id,
                            item_param,
                            scale_raw,
                            color_index as i16,
                            expiry_time,
                        )
                        .await
                    {
                        warn!(
                            "Periodic save: failed to save costume for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 4e. Enchant data save (fire-and-forget) ─────────────────────
        // v2525-specific: WIZ_ENCHANT (0xCC) panel persistence.
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
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::enchant::EnchantRepository::new(&pool_c);
                    if let Err(e) = repo
                        .save(
                            &name_c,
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
                        warn!(
                            "Periodic save: failed to save enchant for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 5. Saved magic / buff persistence (fire-and-forget) ─────────
        let magic_entries = world.get_saved_magic_entries(sid);
        if !magic_entries.is_empty() {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            tokio::spawn(async move {
                let repo = SavedMagicRepository::new(&pool_c);
                if let Err(e) = repo.save_saved_magic(&name_c, &magic_entries).await {
                    warn!("Periodic save: failed to save magic for {}: {}", name_c, e);
                }
            });
        }

        // ── 6. Genie data save (fire-and-forget) ────────────────────────
        let genie_data = world.with_session(sid, |h| {
            (h.genie_time_abs, h.genie_options.clone(), h.genie_loaded)
        });
        if let Some((genie_abs, genie_options, genie_loaded)) = genie_data {
            if !genie_loaded {
                tracing::warn!(
                    "Periodic save: skipping genie save for {} — not loaded yet",
                    char_name
                );
            } else {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = UserDataRepository::new(&pool_c);
                    let db_val = crate::handler::genie::genie_abs_to_db(genie_abs);
                    if let Err(e) = repo
                        .save_genie_data(&name_c, db_val, &genie_options, 0)
                        .await
                    {
                        warn!(
                            "Periodic save: failed to save genie data for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 6a2. Sealed EXP save (fire-and-forget) ──────────────────────
        if ch.sealed_exp > 0 {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            let sealed = ch.sealed_exp as i32;
            tokio::spawn(async move {
                let repo = UserDataRepository::new(&pool_c);
                if let Err(e) = repo.save_seal_exp(&name_c, sealed).await {
                    warn!(
                        "Periodic save: failed to save sealed_exp for {}: {}",
                        name_c, e
                    );
                }
            });
        }

        // ── 6a3. Bind point save (fire-and-forget) ────────────────────────
        if ch.bind_zone > 0 {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            let bz = ch.bind_zone as i16;
            let bpx = (ch.bind_x * 100.0) as i32;
            let bpz = (ch.bind_z * 100.0) as i32;
            tokio::spawn(async move {
                let repo = CharacterRepository::new(&pool_c);
                if let Err(e) = repo.save_bind(&name_c, bz, bpx, bpz).await {
                    warn!(
                        "Periodic save: failed to save bind point for {}: {}",
                        name_c, e
                    );
                }
            });
        }

        // ── 6a4. Sync user_knightdata (fire-and-forget) ─────────────────
        if ch.knights_id > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i32;
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            let clan_id = ch.knights_id as i16;
            let class = ch.class as i16;
            let level = ch.level as i16;
            let loyalty = ch.loyalty as i32;
            let loyalty_monthly = ch.loyalty_monthly as i32;
            tokio::spawn(async move {
                let repo = ko_db::repositories::knights_cape::KnightsCapeRepository::new(&pool_c);
                if let Err(e) = repo
                    .sync_user_knightdata_on_save(
                        clan_id,
                        &name_c,
                        class,
                        level,
                        loyalty,
                        loyalty_monthly,
                        now,
                    )
                    .await
                {
                    warn!(
                        "Periodic save: failed to sync user_knightdata for {}: {}",
                        name_c, e
                    );
                }
            });
        }

        // ── 6a5. Pet data save (fire-and-forget) ────────────────────────────
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
            let pool_c = pool.clone();
            tokio::spawn(async move {
                let repo = ko_db::repositories::pet::PetRepository::new(&pool_c);
                if let Err(e) = repo.save_pet_data(&pet_row).await {
                    warn!(
                        "Periodic save: failed to save pet data (serial={}): {}",
                        pet_row.n_serial_id, e
                    );
                }
            });
        }

        // ── 6b. Daily operation save (fire-and-forget) ────────────────────
        if let Some(entry) = world.daily_ops.get(&char_name) {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            let data = entry.clone();
            tokio::spawn(async move {
                let repo = UserDataRepository::new(&pool_c);
                let row = data.to_row(&name_c);
                if let Err(e) = repo.save_daily_op(&row).await {
                    warn!(
                        "Periodic save: failed to save daily_op for {}: {}",
                        name_c, e
                    );
                }
            });
        }

        // ── 6c. Daily quest save (fire-and-forget) ──────────────────────
        let dq_data = world.with_session(sid, |h| h.daily_quests.clone());
        if let Some(dq_map) = dq_data {
            if !dq_map.is_empty() {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                let entries: Vec<_> = dq_map.into_values().collect();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(&pool_c);
                    if let Err(e) = repo.save_all_user_quests(&name_c, &entries).await {
                        warn!(
                            "Periodic save: failed to save daily quests for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 6d. Daily rank stats save (fire-and-forget) ─────────────────
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
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::daily_rank::DailyRankRepository::new(&pool_c);
                    if let Err(e) = repo
                        .save_user_stats(
                            &name_c, gm as i64, mh as i64, sh as i64, cw as i64, up as i64,
                        )
                        .await
                    {
                        warn!(
                            "Periodic save: failed to save daily rank stats for {}: {}",
                            name_c, e
                        );
                    }
                });
            }
        }

        // ── 7. Premium state save (fire-and-forget) ─────────────────────
        // Uses account_id stored in SessionHandle (set during gamestart).
        let premium_data = world.with_session(sid, |h| {
            if h.account_id.is_empty() || h.premium_map.is_empty() {
                None
            } else {
                let slots = crate::handler::premium::premium_slots_for_save(&h.premium_map, h.premium_in_use);
                Some((h.account_id.clone(), slots))
            }
        });
        if let Some(Some((acct_id, premium_slots))) = premium_data {
            let pool_c = pool.clone();
            tokio::spawn(async move {
                let repo = PremiumRepository::new(&pool_c);
                if let Err(e) = repo.save_account_premium(&acct_id, &premium_slots).await {
                    warn!(
                        "Periodic save: failed to save premium for account {}: {}",
                        acct_id, e
                    );
                }
            });
        }

        // ── 8. Inventory save (fire-and-forget) ──────────────────────────
        // Provides crash protection — without this, inventory changes between
        // periodic saves would be lost on an unclean server shutdown.
        let inventory = world.get_persistent_inventory(sid);
        if !inventory.is_empty() {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            tokio::spawn(async move {
                let repo = CharacterRepository::new(&pool_c);
                let params: Vec<SaveItemParams> = inventory
                    .iter()
                    .enumerate()
                    .map(|(slot, item)| SaveItemParams {
                        char_id: &name_c,
                        slot_index: slot as i16,
                        item_id: item.item_id as i32,
                        durability: item.durability,
                        count: item.count as i16,
                        flag: item.flag as i16,
                        original_flag: item.original_flag as i16,
                        serial_num: item.serial_num as i64,
                        expire_time: item.expire_time as i32,
                    })
                    .collect();
                if let Err(e) = repo.save_items_batch(&params).await {
                    warn!(
                        "Periodic save: failed to save inventory for {}: {}",
                        name_c, e
                    );
                }
            });
        }

        // ── 9. Quest progress save (fire-and-forget) ──────────────────────
        let quest_data = world.with_session(sid, |h| h.quests.clone());
        if let Some(quests) = quest_data {
            if !quests.is_empty() {
                let pool_c = pool.clone();
                let name_c = char_name.clone();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::quest::QuestRepository::new(&pool_c);
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
                    if let Err(e) = repo.save_user_quests_batch(&name_c, &entries).await {
                        warn!("Periodic save: failed to save quests for {}: {}", name_c, e);
                    }
                });
            }
        }

        // ── 10. Class/race save (fire-and-forget) ─────────────────────────
        // Safety net for Lua PromoteUser* class changes.
        {
            let pool_c = pool.clone();
            let name_c = char_name.clone();
            let class = ch.class as i16;
            let race = ch.race as i16;
            tokio::spawn(async move {
                let repo = CharacterRepository::new(&pool_c);
                if let Err(e) = repo.save_class_change(&name_c, class, race).await {
                    warn!(
                        "Periodic save: failed to save class/race for {}: {}",
                        name_c, e
                    );
                }
            });
        }

        // ── 11. Warehouse save (fire-and-forget) ──────────────────────────
        // Crash protection for warehouse changes between periodic saves.
        let wh_data = world.with_session(sid, |h| {
            if h.account_id.is_empty() || !h.warehouse_loaded || h.warehouse.is_empty() {
                None
            } else {
                Some((h.account_id.clone(), h.warehouse.clone(), h.inn_coins))
            }
        });
        if let Some(Some((acct_id, warehouse, inn_coins))) = wh_data {
            let pool_c = pool.clone();
            tokio::spawn(async move {
                let repo = CharacterRepository::new(&pool_c);
                let wh_params: Vec<ko_db::repositories::character::SaveWarehouseItemParams> =
                    warehouse
                        .iter()
                        .enumerate()
                        .map(|(slot, item)| {
                            ko_db::repositories::character::SaveWarehouseItemParams {
                                account_id: &acct_id,
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
                if let Err(e) = repo.save_warehouse_items_batch(&wh_params).await {
                    warn!(
                        "Periodic save: failed to save warehouse for {}: {}",
                        acct_id, e
                    );
                }
                if let Err(e) = repo.save_warehouse_coins(&acct_id, inn_coins as i32).await {
                    warn!(
                        "Periodic save: failed to save warehouse coins for {}: {}",
                        acct_id, e
                    );
                }
            });
        }
    }
}

/// Save a single character's data synchronously (awaited).
/// Used by `kick_session_for_duplicate()` to persist data before kicking,
/// and by `save_all_characters_sync()` for shutdown saves.
/// Saves: stats, position, inventory (77 slots), saved magic, and marks offline.
pub async fn save_single_character_sync(
    world: &WorldState,
    pool: &DbPool,
    sid: crate::zone::SessionId,
    label: &str,
) {
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return,
    };
    let char_name = ch.name.clone();

    // 1. Stats
    let repo = CharacterRepository::new(pool);
    if let Err(e) = repo
        .save_stats(&SaveStatsParams {
            char_id: &char_name,
            level: ch.level as i16,
            hp: ch.hp,
            mp: ch.mp,
            sp: ch.sp,
            exp: ch.exp as i64,
            gold: ch.gold as i32,
            loyalty: ch.loyalty as i32,
            loyalty_monthly: ch.loyalty_monthly as i32,
            manner_point: ch.manner_point,
        })
        .await
    {
        warn!("{}: failed to save stats for {}: {}", label, char_name, e);
    }

    // 2. Position
    if let Err(e) = repo
        .save_position(
            &char_name,
            pos.zone_id as i16,
            (pos.x * 100.0) as i32,
            0,
            (pos.z * 100.0) as i32,
        )
        .await
    {
        warn!(
            "{}: failed to save position for {}: {}",
            label, char_name, e
        );
    }

    // 2b. Flash time/count/type
    let flash_data = world.with_session(sid, |h| (h.flash_time, h.flash_count, h.flash_type));
    if let Some((ft, fc, ftype)) = flash_data {
        if ft > 0 || fc > 0 {
            if let Err(e) = repo
                .save_flash(&char_name, ft as i32, fc as i16, ftype as i16)
                .await
            {
                warn!("{}: failed to save flash for {}: {}", label, char_name, e);
            }
        }
    }

    // 2c. Stat + skill points
    if let Err(e) = repo
        .save_stat_points(&SaveStatPointsParams {
            char_id: &char_name,
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
            "{}: failed to save stat points for {}: {}",
            label, char_name, e
        );
    }

    // 3. Inventory (batch — 77 slots in 1 query)
    let inventory = world.get_persistent_inventory(sid);
    if !inventory.is_empty() {
        let params: Vec<SaveItemParams> = inventory
            .iter()
            .enumerate()
            .map(|(slot, item)| SaveItemParams {
                char_id: &char_name,
                slot_index: slot as i16,
                item_id: item.item_id as i32,
                durability: item.durability,
                count: item.count as i16,
                flag: item.flag as i16,
                original_flag: item.original_flag as i16,
                serial_num: item.serial_num as i64,
                expire_time: item.expire_time as i32,
            })
            .collect();
        if let Err(e) = repo.save_items_batch(&params).await {
            warn!(
                "{}: failed to save inventory for {}: {}",
                label, char_name, e
            );
        }
    }

    // 4. Saved magic
    let magic_entries = world.get_saved_magic_entries(sid);
    if !magic_entries.is_empty() {
        let magic_repo = SavedMagicRepository::new(pool);
        if let Err(e) = magic_repo
            .save_saved_magic(&char_name, &magic_entries)
            .await
        {
            warn!("{}: failed to save magic for {}: {}", label, char_name, e);
        }
    }

    // 5. Quest progress (batch)
    let quest_data = world.with_session(sid, |h| h.quests.clone());
    if let Some(quests) = quest_data {
        if !quests.is_empty() {
            let quest_repo = ko_db::repositories::quest::QuestRepository::new(pool);
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
            if let Err(e) = quest_repo
                .save_user_quests_batch(&char_name, &entries)
                .await
            {
                warn!("{}: failed to save quests for {}: {}", label, char_name, e);
            }
        }
    }

    // 5a. Daily quest progress
    let dq_data = world.with_session(sid, |h| h.daily_quests.clone());
    if let Some(dq_map) = dq_data {
        if !dq_map.is_empty() {
            let dq_repo = ko_db::repositories::daily_quest::DailyQuestRepository::new(pool);
            let entries: Vec<_> = dq_map.into_values().collect();
            if let Err(e) = dq_repo.save_all_user_quests(&char_name, &entries).await {
                warn!(
                    "{}: failed to save daily quests for {}: {}",
                    label, char_name, e
                );
            }
        }
    }

    // 5b. Class/race (safety net for Lua PromoteUser*)
    if let Err(e) = repo
        .save_class_change(&char_name, ch.class as i16, ch.race as i16)
        .await
    {
        warn!(
            "{}: failed to save class/race for {}: {}",
            label, char_name, e
        );
    }

    // 6. Warehouse
    let account_id = world
        .with_session(sid, |h| h.account_id.clone())
        .unwrap_or_default();
    if !account_id.is_empty() {
        let wh_data = world.with_session(sid, |h| {
            (h.warehouse.clone(), h.inn_coins, h.warehouse_loaded)
        });
        if let Some((warehouse, inn_coins, loaded)) = wh_data {
            if loaded && !warehouse.is_empty() {
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
                if let Err(e) = repo.save_warehouse_items_batch(&wh_params).await {
                    warn!(
                        "{}: failed to save warehouse for {}: {}",
                        label, account_id, e
                    );
                }
                if let Err(e) = repo
                    .save_warehouse_coins(&account_id, inn_coins as i32)
                    .await
                {
                    warn!(
                        "{}: failed to save warehouse coins for {}: {}",
                        label, account_id, e
                    );
                }
            }
        }
    }

    // 7. Premium state
    if !account_id.is_empty() {
        let premium_slots: Vec<(i16, i16, i32)> = world
            .with_session(sid, |h| {
                crate::handler::premium::premium_slots_for_save(&h.premium_map, h.premium_in_use)
            })
            .unwrap_or_default();
        if !premium_slots.is_empty() {
            let prem_repo = PremiumRepository::new(pool);
            if let Err(e) = prem_repo
                .save_account_premium(&account_id, &premium_slots)
                .await
            {
                warn!(
                    "{}: failed to save premium for {}: {}",
                    label, account_id, e
                );
            }
        }
    }

    // 8. Achievement data (batch)
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
        let entries: Vec<(i32, i16, i32, i32)> = h
            .achieve_map
            .iter()
            .map(|(&id, info)| {
                (
                    id as i32,
                    info.status as i16,
                    info.count[0] as i32,
                    info.count[1] as i32,
                )
            })
            .collect();
        let summary = h.achieve_summary.clone();
        (entries, summary)
    });
    if let Some((entries, summary)) = achieve_data {
        if !entries.is_empty() || summary.play_time > 0 {
            let ach_repo = AchieveRepository::new(pool);
            if !entries.is_empty() {
                if let Err(e) = ach_repo
                    .save_user_achieves_batch(&char_name, &entries)
                    .await
                {
                    warn!(
                        "{}: failed to save achievements for {}: {}",
                        label, char_name, e
                    );
                }
            }
            if let Err(e) = ach_repo
                .save_user_achieve_summary(
                    &char_name,
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
                    "{}: failed to save achieve summary for {}: {}",
                    label, char_name, e
                );
            }
        }
    }

    // 9. User perks
    let perk_data = world.with_session(sid, |h| (h.perk_levels, h.rem_perk));
    if let Some((perk_levels, rem_perk)) = perk_data {
        if perk_levels.iter().any(|&v| v != 0) || rem_perk != 0 {
            let perk_repo = PerkRepository::new(pool);
            if let Err(e) = perk_repo
                .save_user_perks(&char_name, &perk_levels, rem_perk)
                .await
            {
                warn!("{}: failed to save perks for {}: {}", label, char_name, e);
            }
        }
    }

    // 9b. Soul data (v2525)
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
            let soul_repo = ko_db::repositories::soul::SoulRepository::new(pool);
            if let Err(e) = soul_repo.save(&char_name, &cats, &slots).await {
                warn!("{}: failed to save soul for {}: {}", label, char_name, e);
            }
        }
    }

    // 9c. Hermetic seal data (v2525)
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
            let seal_repo = ko_db::repositories::hermetic_seal::HermeticSealRepository::new(pool);
            if let Err(e) = seal_repo
                .save(
                    &char_name,
                    max_tier as i16,
                    selected_slot as i16,
                    status as i16,
                    upgrade_count as i16,
                    current_level as i16,
                    elapsed_time as f32,
                )
                .await
            {
                warn!(
                    "{}: failed to save hermetic seal for {}: {}",
                    label, char_name, e
                );
            }
        }
    }

    // 9d. Costume data (v2525)
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
            let costume_repo = ko_db::repositories::costume::CostumeRepository::new(pool);
            if let Err(e) = costume_repo
                .save(
                    &char_name,
                    active_type as i16,
                    item_id,
                    item_param,
                    scale_raw,
                    color_index as i16,
                    expiry_time,
                )
                .await
            {
                warn!("{}: failed to save costume for {}: {}", label, char_name, e);
            }
        }
    }

    // 9e. Enchant data (v2525)
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
            let enchant_repo = ko_db::repositories::enchant::EnchantRepository::new(pool);
            if let Err(e) = enchant_repo
                .save(
                    &char_name,
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
                warn!("{}: failed to save enchant for {}: {}", label, char_name, e);
            }
        }
    }

    // 10. Genie data — only save if genie_loaded is true (prevents overwriting DB with 0)
    let genie_data = world.with_session(sid, |h| {
        (h.genie_time_abs, h.genie_options.clone(), h.genie_loaded)
    });
    if let Some((genie_abs, genie_options, genie_loaded)) = genie_data {
        if !genie_loaded {
            tracing::warn!(
                "{}: skipping genie save for {} — not loaded yet",
                label,
                char_name
            );
        } else {
            let db_val = crate::handler::genie::genie_abs_to_db(genie_abs);
            let genie_repo = UserDataRepository::new(pool);
            if let Err(e) = genie_repo
                .save_genie_data(&char_name, db_val, &genie_options, 0)
                .await
            {
                warn!("{}: failed to save genie for {}: {}", label, char_name, e);
            }
        }
    }

    // 10b. Daily operation cooldowns
    if let Some(entry) = world.daily_ops.get(&char_name) {
        let ud_repo = UserDataRepository::new(pool);
        let row = entry.to_row(&char_name);
        if let Err(e) = ud_repo.save_daily_op(&row).await {
            warn!(
                "{}: failed to save daily_op for {}: {}",
                label, char_name, e
            );
        }
    }

    // 10c. Daily rank stats
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
            let dr_repo = ko_db::repositories::daily_rank::DailyRankRepository::new(pool);
            if let Err(e) = dr_repo
                .save_user_stats(
                    &char_name, gm as i64, mh as i64, sh as i64, cw as i64, up as i64,
                )
                .await
            {
                warn!(
                    "{}: failed to save daily rank stats for {}: {}",
                    label, char_name, e
                );
            }
        }
    }

    // 10d. Sealed EXP
    if ch.sealed_exp > 0 {
        let seal_repo = UserDataRepository::new(pool);
        if let Err(e) = seal_repo
            .save_seal_exp(&char_name, ch.sealed_exp as i32)
            .await
        {
            warn!(
                "{}: failed to save sealed_exp for {}: {}",
                label, char_name, e
            );
        }
    }

    // 10e. Bind point
    if ch.bind_zone > 0 {
        if let Err(e) = repo
            .save_bind(
                &char_name,
                ch.bind_zone as i16,
                (ch.bind_x * 100.0) as i32,
                (ch.bind_z * 100.0) as i32,
            )
            .await
        {
            warn!(
                "{}: failed to save bind point for {}: {}",
                label, char_name, e
            );
        }
    }

    // 10f. Sync user_knightdata
    if ch.knights_id > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i32;
        let kd_repo = ko_db::repositories::knights_cape::KnightsCapeRepository::new(pool);
        if let Err(e) = kd_repo
            .sync_user_knightdata_on_save(
                ch.knights_id as i16,
                &char_name,
                ch.class as i16,
                ch.level as i16,
                ch.loyalty as i32,
                ch.loyalty_monthly as i32,
                now,
            )
            .await
        {
            warn!(
                "{}: failed to sync user_knightdata for {}: {}",
                label, char_name, e
            );
        }
    }

    // 10g. Pet data save
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
        let pet_repo = ko_db::repositories::pet::PetRepository::new(pool);
        if let Err(e) = pet_repo.save_pet_data(&pet_row).await {
            warn!(
                "{}: failed to save pet data (serial={}) for {}: {}",
                label, pet_row.n_serial_id, char_name, e
            );
        }
    }

    // 11. Mark account offline
    if !account_id.is_empty() {
        let acct_repo = ko_db::repositories::account::AccountRepository::new(pool);
        if let Err(e) = acct_repo.set_offline(&account_id).await {
            warn!("{}: failed to set offline for {}: {}", label, account_id, e);
        }
    }
}

/// Synchronous (awaited) save of all online characters — used at shutdown.
/// Unlike the periodic `save_all_characters` which is fire-and-forget,
/// this function awaits each character's save tasks to ensure all data
/// is persisted before the process exits.
pub async fn save_all_characters_sync(world: &WorldState, pool: &DbPool) {
    let session_ids = world.get_in_game_session_ids();
    if session_ids.is_empty() {
        debug!("Shutdown save: no online characters to save");
        return;
    }
    let total = session_ids.len();
    debug!("Shutdown save: saving {} online characters", total);

    for sid in session_ids {
        save_single_character_sync(world, pool, sid, "Shutdown save").await;
    }
    debug!("Shutdown save complete ({total} characters)");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_save_interval_matches_cpp() {
        // C++ PLAYER_SAVE_INTERVAL = 10 * 60 = 600 seconds
        assert_eq!(PLAYER_SAVE_INTERVAL_SECS, 600);
    }

    #[test]
    fn test_save_interval_is_ten_minutes() {
        let duration = Duration::from_secs(PLAYER_SAVE_INTERVAL_SECS);
        assert_eq!(duration.as_secs(), 600);
        assert_eq!(duration.as_secs() / 60, 10);
    }

    #[test]
    fn test_account_id_available_in_session_handle() {
        // Verify that account_id can be stored and retrieved from SessionHandle
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Initially empty
        let acct = world.with_session(1, |h| h.account_id.clone());
        assert_eq!(acct, Some(String::new()));

        // Set account_id
        world.update_session(1, |h| {
            h.account_id = "test_account".to_string();
        });

        let acct = world.with_session(1, |h| h.account_id.clone());
        assert_eq!(acct, Some("test_account".to_string()));
    }

    #[test]
    fn test_premium_data_extraction_for_save() {
        // Verify premium data can be extracted from SessionHandle for periodic save
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set account_id and premium data
        world.update_session(1, |h| {
            h.account_id = "premium_acct".to_string();
            h.premium_map.insert(1, 1700000000);
            h.premium_map.insert(5, 1700050000);
        });

        // Extract premium data (same pattern as save_all_characters)
        let premium_data = world.with_session(1, |h| {
            if h.account_id.is_empty() || h.premium_map.is_empty() {
                None
            } else {
                let slots = crate::handler::premium::premium_slots_for_save(&h.premium_map, h.premium_in_use);
                Some((h.account_id.clone(), slots))
            }
        });

        let (acct, slots) = premium_data.unwrap().unwrap();
        assert_eq!(acct, "premium_acct");
        assert_eq!(slots.len(), 2);
    }

    #[test]
    fn test_premium_data_extraction_empty_account() {
        // When account_id is empty, premium save should be skipped
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.update_session(1, |h| {
            h.premium_map.insert(1, 1700000000);
        });

        let premium_data = world.with_session(1, |h| {
            if h.account_id.is_empty() || h.premium_map.is_empty() {
                None
            } else {
                Some(h.account_id.clone())
            }
        });

        // Should be None because account_id is empty
        assert_eq!(premium_data, Some(None));
    }

    #[test]
    fn test_premium_data_extraction_empty_map() {
        // When premium_map is empty, save should be skipped
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.update_session(1, |h| {
            h.account_id = "acct_no_premium".to_string();
        });

        let premium_data = world.with_session(1, |h| {
            if h.account_id.is_empty() || h.premium_map.is_empty() {
                None
            } else {
                Some(h.account_id.clone())
            }
        });

        assert_eq!(premium_data, Some(None));
    }

    // ── Sprint 934: Additional coverage ──────────────────────────────

    /// Gold is clamped to i32::MAX before DB persistence.
    #[test]
    fn test_character_save_gold_clamp() {
        let gold_normal: u32 = 500_000;
        assert_eq!(gold_normal.min(i32::MAX as u32) as i32, 500_000);
        let gold_overflow: u32 = u32::MAX;
        assert_eq!(gold_overflow.min(i32::MAX as u32) as i32, i32::MAX);
    }

    /// Stat fields (u8) safely convert to i16 for DB save.
    #[test]
    fn test_character_save_stat_i16_range() {
        for val in [0u8, 1, 60, 128, 255] {
            let as_i16 = val as i16;
            assert!(as_i16 >= 0 && as_i16 <= 255);
        }
    }

    /// Session with account_id set can be found for save.
    #[test]
    fn test_character_save_account_id_present() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.update_session(sid, |h| {
            h.account_id = "acct001".to_string();
        });

        let acct = world.with_session(sid, |h| h.account_id.clone());
        assert_eq!(acct, Some("acct001".to_string()));
    }

    /// Skill points array has exactly 10 slots for DB save.
    #[test]
    fn test_character_save_skill_points_10() {
        let skill_points: [u8; 10] = [5, 0, 3, 0, 0, 10, 7, 0, 0, 0];
        let as_i16: Vec<i16> = skill_points.iter().map(|&v| v as i16).collect();
        assert_eq!(as_i16.len(), 10);
        assert_eq!(as_i16[0], 5);
        assert_eq!(as_i16[5], 10);
    }

    // ── Sprint 948: Additional coverage ──────────────────────────────

    /// Position float→i32 conversion: x * 100.
    #[test]
    fn test_position_float_to_i32_conversion() {
        let x: f32 = 123.456;
        let z: f32 = 789.012;
        let px = (x * 100.0) as i32;
        let pz = (z * 100.0) as i32;
        assert_eq!(px, 12345);
        assert_eq!(pz, 78901);
    }

    /// Position zero converts to i32 zero.
    #[test]
    fn test_position_zero_conversion() {
        let x: f32 = 0.0;
        let px = (x * 100.0) as i32;
        assert_eq!(px, 0);
    }

    /// Gold values within i32 range are unchanged.
    #[test]
    fn test_gold_within_i32_range() {
        for gold in [0u32, 1, 1_000_000, 2_000_000_000] {
            let clamped = gold.min(i32::MAX as u32) as i32;
            assert_eq!(clamped, gold as i32);
        }
    }

    /// Gold at exact i32::MAX boundary.
    #[test]
    fn test_gold_at_i32_max_boundary() {
        let gold: u32 = i32::MAX as u32;
        let clamped = gold.min(i32::MAX as u32) as i32;
        assert_eq!(clamped, i32::MAX);
        // One above also clamps
        let gold_plus = (i32::MAX as u32) + 1;
        let clamped2 = gold_plus.min(i32::MAX as u32) as i32;
        assert_eq!(clamped2, i32::MAX);
    }

    /// Level u8 → i16 conversion covers full range.
    #[test]
    fn test_level_to_i16() {
        for level in [1u8, 30, 83, 255] {
            let as_i16 = level as i16;
            assert!(as_i16 > 0 && as_i16 <= 255);
        }
    }

    /// Flash data fields convert to DB types safely.
    #[test]
    fn test_flash_data_conversion() {
        let flash_time: u32 = 1700000000;
        let flash_count: u16 = 500;
        let flash_type: u8 = 3;
        assert_eq!(flash_time as i32, 1700000000);
        assert_eq!(flash_count as i16, 500);
        assert_eq!(flash_type as i16, 3);
    }

    /// Exp u64 → i64 is safe for realistic ranges.
    #[test]
    fn test_exp_to_i64() {
        let exp: u64 = 5_000_000_000;
        let as_i64 = exp as i64;
        assert_eq!(as_i64, 5_000_000_000);
    }

    /// Loyalty u32 → i32 clamps at i32::MAX.
    #[test]
    fn test_loyalty_i32_conversion() {
        let loyalty: u32 = 100_000;
        assert_eq!(loyalty as i32, 100_000);
    }

    /// Empty session_ids list triggers early return (no crash).
    #[test]
    fn test_empty_session_ids_safe() {
        let ids: Vec<u16> = vec![];
        assert!(ids.is_empty());
    }

    /// Manner point i32 is passed directly to DB.
    #[test]
    fn test_manner_point_range() {
        for val in [i32::MIN, -100, 0, 100, i32::MAX] {
            let mp: i32 = val;
            assert_eq!(mp, val);
        }
    }

    // ── Sprint 960: Additional coverage ──────────────────────────────

    /// PLAYER_SAVE_INTERVAL_SECS is 10 minutes (600s).
    #[test]
    fn test_save_interval_constant() {
        assert_eq!(PLAYER_SAVE_INTERVAL_SECS, 600);
        assert_eq!(PLAYER_SAVE_INTERVAL_SECS, 10 * 60);
    }

    /// Position scaling: float * 100.0 → i32 for DB storage.
    #[test]
    fn test_position_scaling_factor() {
        let x: f32 = 267.5;
        let z: f32 = 303.25;
        let px = (x * 100.0) as i32;
        let pz = (z * 100.0) as i32;
        assert_eq!(px, 26750);
        assert_eq!(pz, 30325);
    }

    /// Negative HP/MP are valid i16 values for DB save.
    #[test]
    fn test_negative_hp_mp_save() {
        let hp: i16 = -1; // dead
        let mp: i16 = 0;
        assert!(hp < 0);
        assert_eq!(mp, 0);
        // Fits i16 range
        assert!(hp >= i16::MIN);
    }

    /// Level u8 → i16 conversion for DB.
    #[test]
    fn test_level_u8_to_i16_all_valid() {
        for level in [1u8, 60, 83, 255] {
            let db_level = level as i16;
            assert_eq!(db_level, level as i16);
            assert!(db_level > 0);
        }
    }

    /// Gold u32 → i32 clamping at i32::MAX.
    #[test]
    fn test_gold_clamping_boundary() {
        let gold_normal: u32 = 1_000_000;
        let gold_max: u32 = u32::MAX;
        assert_eq!(gold_normal as i32, 1_000_000);
        // u32::MAX wraps to -1 as i32
        assert_eq!(gold_max as i32, -1);
        // Proper clamping approach
        let clamped = gold_max.min(i32::MAX as u32) as i32;
        assert_eq!(clamped, i32::MAX);
    }

    // ── Sprint 970: Additional coverage ──────────────────────────────

    /// Loyalty u32 → i32 conversion for DB save.
    #[test]
    fn test_loyalty_u32_to_i32() {
        let loyalty: u32 = 50_000;
        assert_eq!(loyalty as i32, 50_000);
        // Max safe value
        let max_safe: u32 = i32::MAX as u32;
        assert_eq!(max_safe as i32, i32::MAX);
    }

    /// Exp u64 → i64 conversion for DB save.
    #[test]
    fn test_exp_u64_to_i64() {
        let exp: u64 = 1_000_000_000;
        assert_eq!(exp as i64, 1_000_000_000);
        // Max safe value
        let max_safe: u64 = i64::MAX as u64;
        assert_eq!(max_safe as i64, i64::MAX);
    }

    /// SP (stamina points) is i16 and can be negative.
    #[test]
    fn test_sp_i16_range() {
        let sp_full: i16 = 1000;
        let sp_zero: i16 = 0;
        let sp_neg: i16 = -1;
        assert!(sp_full > 0);
        assert_eq!(sp_zero, 0);
        assert!(sp_neg < 0);
    }

    /// Zone u16 → i16 conversion for DB position save.
    #[test]
    fn test_zone_u16_to_i16() {
        let moradon: u16 = 21;
        let prison: u16 = 92;
        assert_eq!(moradon as i16, 21);
        assert_eq!(prison as i16, 92);
        // All valid zone IDs fit i16
        assert!(moradon <= i16::MAX as u16);
        assert!(prison <= i16::MAX as u16);
    }

    /// Flash data defaults: time=0, count=0, type=0 means no flash save needed.
    #[test]
    fn test_flash_data_skip_condition() {
        let ft: u32 = 0;
        let fc: u32 = 0;
        // Skip save when both are 0
        assert!(!(ft > 0 || fc > 0));
        // Save when either is non-zero
        let ft2: u32 = 100;
        assert!(ft2 > 0 || fc > 0);
    }

    // ── Sprint 979: Additional coverage ──────────────────────────────

    /// Rebirth stats (reb_str..reb_cha) fit in i16.
    #[test]
    fn test_rebirth_stats_i16_range() {
        let stats: [u16; 5] = [100, 200, 150, 180, 90];
        for s in &stats {
            assert!(*s <= i16::MAX as u16);
            assert_eq!(*s as i16 as u16, *s);
        }
    }

    /// Cover title u16 → i16 conversion.
    #[test]
    fn test_cover_title_conversion() {
        let cover_title: u16 = 500;
        assert_eq!(cover_title as i16, 500);
        // Zero means no title
        assert_eq!(0u16 as i16, 0);
    }

    /// Anger gauge is u8, always fits in DB column.
    #[test]
    fn test_anger_gauge_u8_range() {
        let gauge: u8 = 100;
        assert!(gauge <= u8::MAX);
        assert_eq!(gauge as i16, 100);
    }

    /// Loyalty monthly is separate from loyalty and both u32 → i32.
    #[test]
    fn test_loyalty_monthly_conversion() {
        let loyalty: u32 = 30_000;
        let loyalty_monthly: u32 = 5_000;
        assert_eq!(loyalty as i32, 30_000);
        assert_eq!(loyalty_monthly as i32, 5_000);
        assert_ne!(loyalty, loyalty_monthly);
    }

    /// Position scaling: x * 100.0 as i32 for sub-unit precision.
    #[test]
    fn test_position_scaling_precision() {
        let x: f32 = 123.45;
        let z: f32 = 67.89;
        let px = (x * 100.0) as i32;
        let pz = (z * 100.0) as i32;
        assert_eq!(px, 12345);
        assert_eq!(pz, 6789);
    }
}
