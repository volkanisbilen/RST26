//! Startup DB table loading for WorldState.
//! Extracted from `WorldState::load()` to reduce the size of `mod.rs`.
//! All methods here populate DashMap/RwLock fields on an already-constructed
//! WorldState by reading rows from PostgreSQL via repository pattern.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use ko_db::repositories::achieve::AchieveRepository;
use ko_db::repositories::bot_system::BotSystemRepository;
use ko_db::repositories::burning::BurningRepository;
use ko_db::repositories::cash_shop::CashShopRepository;
use ko_db::repositories::chaos_stone::ChaosStoneRepository;
use ko_db::repositories::char_creation::CharCreationRepository;
use ko_db::repositories::cinderella::CinderellaRepository;
use ko_db::repositories::coefficient::CoefficientRepository;
use ko_db::repositories::daily_quest::DailyQuestRepository;
use ko_db::repositories::draki_tower::DrakiTowerRepository;
use ko_db::repositories::dungeon_defence::DungeonDefenceRepository;
use ko_db::repositories::event_schedule::EventScheduleRepository;
use ko_db::repositories::forgotten_temple::ForgottenTempleRepository;
use ko_db::repositories::item::ItemRepository;
use ko_db::repositories::item_tables::ItemTablesRepository;
use ko_db::repositories::item_upgrade_ext::ItemUpgradeExtRepository;
use ko_db::repositories::jackpot::JackPotRepository;
use ko_db::repositories::king::KingRepository;
use ko_db::repositories::knights::KnightsRepository;
use ko_db::repositories::knights_cape::KnightsCapeRepository;
use ko_db::repositories::level_up::LevelUpRepository;
use ko_db::repositories::magic::MagicRepository;
use ko_db::repositories::manes_survival::ManesSurvivalRepository;
use ko_db::repositories::mining::MiningRepository;
use ko_db::repositories::monster_event::MonsterEventRepository;
use ko_db::repositories::npc::NpcRepository;
use ko_db::repositories::perk::PerkRepository;
use ko_db::repositories::pet::PetRepository;
use ko_db::repositories::premium::PremiumRepository;
use ko_db::repositories::quest::QuestRepository;
use ko_db::repositories::quest_text::QuestTextRepository;
use ko_db::repositories::server_settings::ServerSettingsRepository;
use ko_db::repositories::siege::SiegeRepository;
use ko_db::repositories::under_castle::UnderCastleRepository;
use ko_db::repositories::zone::ZoneRepository;
use ko_db::repositories::zone_rewards::ZoneRewardsRepository;
use ko_db::DbPool;
use ko_protocol::smd::SmdFile;

use crate::clan_constants::CLAN_TYPE_ACCREDITED5;
use crate::npc::{NpcInstance, NpcTemplate};
use crate::systems::daily_reset::get_knights_grade;
use crate::zone::{calc_region, ObjectEventInfo, ZoneState};

use super::{
    events_from_rows, is_gate_npc_type, is_guard_npc_type, map_act_type, zone_info_from_row,
    BurningFeatureRates, ElectionListEntry, KingSystem, KnightsAlliance, KnightsInfo,
    NominationEntry, NpcAiState, NpcState, SiegeWarfare, WorldState, ZONE_ARDREAM,
    ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE,
};

impl WorldState {
    /// Load all startup DB tables into an already-constructed WorldState.
    ///
    /// This is the main loading entrypoint called by `WorldState::load()`.
    /// Each sub-section loads one or more related tables from PostgreSQL
    /// and populates the corresponding DashMap/RwLock field.
    ///
    pub(crate) async fn load_all_tables(
        &self,
        pool: &DbPool,
        map_dir: &Path,
    ) -> anyhow::Result<()> {
        // ─── Coefficient Table Loading ──────────────────────────────────────────
        let coeff_repo = CoefficientRepository::new(pool);
        let coeff_rows = coeff_repo.load_all_coefficients().await?;
        for row in &coeff_rows {
            self.coefficients.insert(row.s_class as u16, row.clone());
        }
        tracing::info!(count = coeff_rows.len(), "coefficient table loaded");

        // ─── Level-Up Table Loading ─────────────────────────────────────────────
        let level_up_repo = LevelUpRepository::new(pool);
        let level_up_rows = level_up_repo.load_all().await?;
        for row in &level_up_rows {
            self.level_up_table
                .insert((row.level as u8, row.rebirth_level as u8), row.exp);
        }
        tracing::info!(count = level_up_rows.len(), "level-up table loaded");

        // ─── King System Loading ────────────────────────────────────────────────
        self.load_king_system(pool).await;

        // ─── Siege Warfare Loading ──────────────────────────────────────────────
        self.load_siege_warfare(pool).await;

        // ─── Item Table Loading ─────────────────────────────────────────────────
        let item_repo = ItemRepository::new(pool);
        let item_rows = item_repo.load_all().await?;
        for row in &item_rows {
            self.items.insert(row.num as u32, row.clone());
        }
        tracing::info!(count = item_rows.len(), "item table loaded");

        // ─── Item Upgrade / Reference Tables ────────────────────────────────────
        self.load_item_tables(pool).await?;

        // ─── Premium System Loading ─────────────────────────────────────────────
        self.load_premium_tables(pool).await;

        // ─── Cash Shop (PUS) Loading ────────────────────────────────────────────
        self.load_cash_shop(pool).await;

        // ─── Mining & Fishing Item Loading ──────────────────────────────────────
        self.load_mining_tables(pool).await;
        self.load_mining_exchange_table(pool).await;
        self.load_right_top_titles(pool).await;

        // ─── Bot System Loading ─────────────────────────────────────────────────
        self.load_bot_tables(pool).await;

        // ─── User Rankings (LoadUserRankings) ──────────────────────────────────
        self.reload_user_rankings(pool).await;

        // ─── Pet System Loading ─────────────────────────────────────────────────
        self.load_pet_tables(pool).await;

        // ─── Achievement Table Loading ──────────────────────────────────────────
        self.load_achievement_tables(pool).await;

        // ─── Magic Table Loading ────────────────────────────────────────────────
        self.load_magic_tables(pool).await?;

        // ─── Zone / Map Loading ─────────────────────────────────────────────────
        self.load_zones(pool, map_dir).await?;

        // ─── NPC / Monster Loading ──────────────────────────────────────────────
        self.load_npcs_and_monsters(pool).await?;

        // ─── Native MORANKER Statues (top 3 per nation by NP) ──────────────────
        self.reload_moraranker(pool, false).await?;

        // ─── Manes Survival Runtime Configuration ──────────────────────────────
        if let Err(e) = self.load_manes_survival(pool).await {
            tracing::warn!(
                zone = 96,
                error = %e,
                "Manes Survival configuration failed to load; continuing without it"
            );
        }

        // ─── Knights (Clan) Loading ─────────────────────────────────────────────
        self.load_knights(pool).await?;

        // ─── Quest Table Loading ────────────────────────────────────────────────
        self.load_quest_tables(pool).await?;

        // ─── Server Settings Loading ────────────────────────────────────────────
        self.load_server_settings(pool).await;

        // ─── Monster Summon / Respawn / Boss Loading ────────────────────────────
        self.load_monster_event_tables(pool).await;

        // ─── Perk Definitions Loading ───────────────────────────────────────────
        self.load_perk_tables(pool).await;
        self.load_jackpot_settings(pool).await;

        //─── Event Schedule Loading ─────────────────────────────────────────────
        self.load_event_schedule(pool).await;

        // ─── Forgotten Temple / DD / Draki / Chaos Stone Loading ────────────────
        self.load_dungeon_tables(pool).await;

        // ─── Under The Castle / Daily Quest Loading ─────────────────────────────
        self.load_misc_event_tables(pool).await;

        // ─── Character Creation Data Loading ────────────────────────────────────
        self.load_char_creation(pool).await;

        // ─── Knights Cape Tables Loading ────────────────────────────────────────
        self.load_cape_tables(pool).await;

        // ─── Monster Event Spawns Loading ───────────────────────────────────────
        self.load_monster_event_spawns(pool).await;

        // ─── Cinderella War Data Loading ────────────────────────────────────────
        self.load_cinderella_war(pool).await;

        // ─── Zone Reward Table Loading ──────────────────────────────────────────
        self.load_zone_rewards(pool).await;

        // ─── Anti-AFK NPC List Loading ────────────────────────────────────────
        self.load_anti_afk_list(pool).await;

        // ─── RANKBUG Configuration ──────────────────────────────────────────
        self.load_rankbug(pool).await;

        // ─── Banish of Winner Loading ─────────────────────────────────────────
        self.load_banish_of_winner(pool).await;

        Ok(())
    }

    /// Load the validated zone-96 runtime spawn configuration.
    async fn load_manes_survival(&self, pool: &DbPool) -> anyhow::Result<()> {
        let repository = ManesSurvivalRepository::new(pool);
        let rows = repository.load_spawns().await?;
        let count = rows.len();
        self.manes_survival_manager.set_spawns(rows)?;
        let magic = repository.load_magic().await?;
        let magic_count = magic.len();
        self.manes_survival_manager.set_magic(magic)?;
        tracing::info!(
            spawns = count,
            magic_rows = magic_count,
            zone = 96,
            "Manes Survival configuration loaded"
        );
        Ok(())
    }

    /// Load king system data from DB.
    async fn load_king_system(&self, pool: &DbPool) {
        let king_repo = KingRepository::new(pool);
        match king_repo.load_all().await {
            Ok(king_rows) => {
                for row in &king_rows {
                    let nation = row.by_nation as u8;
                    self.king_systems.insert(
                        nation,
                        KingSystem {
                            nation,
                            election_type: row.by_type as u8,
                            year: row.s_year as u16,
                            month: row.by_month as u8,
                            day: row.by_day as u8,
                            hour: row.by_hour as u8,
                            minute: row.by_minute as u8,
                            im_type: row.by_im_type as u8,
                            im_year: row.s_im_year as u16,
                            im_month: row.by_im_month as u8,
                            im_day: row.by_im_day as u8,
                            im_hour: row.by_im_hour as u8,
                            im_minute: row.by_im_minute as u8,
                            noah_event: row.by_noah_event as u8,
                            noah_event_day: row.by_noah_event_day as u8,
                            noah_event_hour: row.by_noah_event_hour as u8,
                            noah_event_minute: row.by_noah_event_minute as u8,
                            noah_event_duration: row.s_noah_event_duration as u16,
                            exp_event: row.by_exp_event as u8,
                            exp_event_day: row.by_exp_event_day as u8,
                            exp_event_hour: row.by_exp_event_hour as u8,
                            exp_event_minute: row.by_exp_event_minute as u8,
                            exp_event_duration: row.s_exp_event_duration as u16,
                            tribute: row.n_tribute as u32,
                            territory_tariff: row.by_territory_tariff as u8,
                            territory_tax: row.n_territory_tax as u32,
                            national_treasury: row.n_national_treasury as u32,
                            king_name: row.str_king_name.trim().to_string(),
                            king_clan_id: row.s_king_clan_id as u16,
                            im_request_id: row.str_im_request_id.trim().to_string(),
                            election_under_progress: false,
                            sent_first_message: false,
                            top10_clan_set: Vec::new(),
                            senator_list: Vec::new(),
                            candidate_list: Vec::new(),
                            nomination_list: Vec::new(),
                            notice_board: Vec::new(),
                            resigned_candidates: Vec::new(),
                            new_king_name: row.str_new_king_name.trim().to_string(),
                            king_votes: row.king_votes as u32,
                            total_votes: row.total_votes as u32,
                        },
                    );
                }
                tracing::info!(count = king_rows.len(), "king system loaded");

                // If an election is in progress, reload election lists from DB.
                for row in &king_rows {
                    let nation = row.by_nation as u8;
                    let election_type = row.by_type as u8;
                    if election_type == 0 {
                        continue;
                    }

                    if let Ok(el_rows) = king_repo.load_election_list(row.by_nation).await {
                        let mut senators = Vec::new();
                        let mut candidates = Vec::new();
                        for el in &el_rows {
                            let entry = ElectionListEntry {
                                name: el.str_name.trim().to_string(),
                                knights_id: el.n_knights as u16,
                                votes: el.n_money as u32,
                            };
                            match el.by_type {
                                3 => senators.push(entry),
                                4 => candidates.push(entry),
                                _ => {}
                            }
                        }
                        let top10: Vec<u16> = senators.iter().map(|e| e.knights_id).collect();
                        let s_count = senators.len();
                        let c_count = candidates.len();
                        self.update_king_system(nation, |ks| {
                            ks.senator_list = senators;
                            ks.candidate_list = candidates;
                            ks.top10_clan_set = top10;
                        });
                        tracing::info!(
                            nation,
                            senators = s_count,
                            candidates = c_count,
                            "king election lists loaded"
                        );
                    }

                    if let Ok(nom_rows) = king_repo.load_nomination_list(row.by_nation).await {
                        let n_count = nom_rows.len();
                        let nominations: Vec<NominationEntry> = nom_rows
                            .iter()
                            .map(|n| NominationEntry {
                                nominator: n.str_nominator.trim().to_string(),
                                nominee: n.str_nominee.trim().to_string(),
                            })
                            .collect();
                        self.update_king_system(nation, |ks| {
                            ks.nomination_list = nominations;
                        });
                        tracing::info!(nation, nominations = n_count, "king nominations loaded");
                    }

                    if let Ok(nb_rows) = king_repo.load_notice_board(row.by_nation).await {
                        let nb_count = nb_rows.len();
                        let board: Vec<(String, String)> = nb_rows
                            .iter()
                            .map(|nb| (nb.str_user_id.trim().to_string(), nb.str_notice.clone()))
                            .collect();
                        self.update_king_system(nation, |ks| {
                            ks.notice_board = board;
                        });
                        tracing::info!(nation, notices = nb_count, "king notice board loaded");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("king_system table not found or error: {e}, skipping");
            }
        }
    }

    /// Load siege warfare data from DB.
    async fn load_siege_warfare(&self, pool: &DbPool) {
        let siege_repo = SiegeRepository::new(pool);
        match siege_repo.load_all().await {
            Ok(rows) => {
                if let Some(row) = rows.first() {
                    let sw = SiegeWarfare {
                        castle_index: row.s_castle_index as u16,
                        master_knights: row.s_master_knights as u16,
                        siege_type: row.by_siege_type as u8,
                        war_day: row.by_war_day as u8,
                        war_time: row.by_war_time as u8,
                        war_minute: row.by_war_minute as u8,
                        challenge_list: [
                            row.s_challenge_list_1 as u16,
                            row.s_challenge_list_2 as u16,
                            row.s_challenge_list_3 as u16,
                            row.s_challenge_list_4 as u16,
                            row.s_challenge_list_5 as u16,
                            row.s_challenge_list_6 as u16,
                            row.s_challenge_list_7 as u16,
                            row.s_challenge_list_8 as u16,
                            row.s_challenge_list_9 as u16,
                            row.s_challenge_list_10 as u16,
                        ],
                        war_request_day: row.by_war_request_day as u8,
                        war_request_time: row.by_war_request_time as u8,
                        war_request_minute: row.by_war_request_minute as u8,
                        guerrilla_war_day: row.by_guerrilla_war_day as u8,
                        guerrilla_war_time: row.by_guerrilla_war_time as u8,
                        guerrilla_war_minute: row.by_guerrilla_war_minute as u8,
                        challenge_list_str: row.str_challenge_list.trim().to_string(),
                        moradon_tariff: row.s_moradon_tariff as u16,
                        delos_tariff: row.s_delos_tariff as u16,
                        dungeon_charge: row.n_dungeon_charge,
                        moradon_tax: row.n_moradon_tax,
                        delos_tax: row.n_delos_tax,
                        request_list: [
                            row.s_request_list_1 as u16,
                            row.s_request_list_2 as u16,
                            row.s_request_list_3 as u16,
                            row.s_request_list_4 as u16,
                            row.s_request_list_5 as u16,
                            row.s_request_list_6 as u16,
                            row.s_request_list_7 as u16,
                            row.s_request_list_8 as u16,
                            row.s_request_list_9 as u16,
                            row.s_request_list_10 as u16,
                        ],
                    };
                    *self.siege_war.write().await = sw;
                    tracing::info!(
                        castle_index = row.s_castle_index,
                        master_knights = row.s_master_knights,
                        moradon_tariff = row.s_moradon_tariff,
                        delos_tariff = row.s_delos_tariff,
                        "siege warfare loaded"
                    );
                }
            }
            Err(e) => {
                tracing::warn!("knights_siege_warfare table not found or error: {e}, skipping");
            }
        }
    }

    /// Load item upgrade, crafting, exchange, and reference tables.
    async fn load_item_tables(&self, pool: &DbPool) -> anyhow::Result<()> {
        let item_tables_repo = ItemTablesRepository::new(pool);

        let upgrade_rows = item_tables_repo.load_all_new_upgrades().await?;
        for row in &upgrade_rows {
            self.upgrade_recipes
                .entry(row.origin_number)
                .or_default()
                .push(row.clone());
        }
        tracing::info!(count = upgrade_rows.len(), "new_upgrade table loaded");

        let settings_rows = item_tables_repo.load_all_upgrade_settings().await?;
        for (idx, row) in settings_rows.iter().enumerate() {
            self.upgrade_settings.insert(idx as u32, row.clone());
        }
        tracing::info!(
            count = settings_rows.len(),
            "item_upgrade_settings table loaded"
        );

        // Item Upgrade Probability
        let upgrade_ext_repo = ItemUpgradeExtRepository::new(pool);
        let prob_rows = upgrade_ext_repo.load_all_itemup_probability().await?;
        if let Some(first) = prob_rows.into_iter().next() {
            *self.itemup_probability.write() = Some(first);
        }
        tracing::info!("itemup_probability table loaded");

        // Item Op, Set Item, Monster/NPC items, Exchange, Upgrade, Make, Rental tables
        match item_tables_repo.load_all_item_ops().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_ops.entry(r.item_id).or_default().push(r.clone());
                }
                tracing::info!(count = rows.len(), "item_op table loaded");
            }
            Err(e) => tracing::warn!("item_op table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_set_items().await {
            Ok(rows) => {
                for r in &rows {
                    self.set_items.insert(r.set_index, r.clone());
                }
                tracing::info!(count = rows.len(), "set_item table loaded");
            }
            Err(e) => tracing::warn!("set_item table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_monster_items().await {
            Ok(rows) => {
                for r in &rows {
                    self.monster_items.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "monster_item table loaded");
            }
            Err(e) => tracing::warn!("monster_item table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_npc_items().await {
            Ok(rows) => {
                for r in &rows {
                    self.npc_items.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "npc_item table loaded");
            }
            Err(e) => tracing::warn!("npc_item table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_item_exchanges().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_exchanges.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "item_exchange table loaded");
            }
            Err(e) => tracing::warn!("item_exchange table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_item_upgrades().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_upgrades.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "item_upgrade table loaded");
            }
            Err(e) => tracing::warn!("item_upgrade table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_make_weapons().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_weapons.insert(r.by_level, r.clone());
                }
                tracing::info!(count = rows.len(), "make_weapon table loaded");
            }
            Err(e) => tracing::warn!("make_weapon table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_make_defensives().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_defensives.insert(r.by_level, r.clone());
                }
                tracing::info!(count = rows.len(), "make_defensive table loaded");
            }
            Err(e) => tracing::warn!("make_defensive table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_make_grade_codes().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_grade_codes.insert(r.item_index, r.clone());
                }
                tracing::info!(count = rows.len(), "make_item_gradecode table loaded");
            }
            Err(e) => tracing::warn!("make_item_gradecode table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_make_lare_codes().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_lare_codes.insert(r.level_grade, r.clone());
                }
                tracing::info!(count = rows.len(), "make_item_larecode table loaded");
            }
            Err(e) => tracing::warn!("make_item_larecode table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_make_item_groups().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_item_groups.insert(r.group_num, r.clone());
                }
                tracing::info!(count = rows.len(), "make_item_group table loaded");
            }
            Err(e) => tracing::warn!("make_item_group table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_make_item_group_randoms().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_item_group_randoms.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "make_item_group_random table loaded");
            }
            Err(e) => {
                tracing::warn!("make_item_group_random table not found or error: {e}, skipping")
            }
        }
        match item_tables_repo.load_all_make_items().await {
            Ok(rows) => {
                for r in &rows {
                    self.make_items.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "make_item table loaded");
            }
            Err(e) => tracing::warn!("make_item table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_rental_items().await {
            Ok(rows) => {
                for r in &rows {
                    self.rental_items.insert(r.rental_index, r.clone());
                }
                tracing::info!(count = rows.len(), "rental_item table loaded");
                self.init_rental_index_counter();
            }
            Err(e) => tracing::warn!("rental_item table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_sell_table().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.item_sell_table
                        .entry(r.i_selling_group)
                        .or_default()
                        .push(r);
                }
                tracing::info!(
                    count,
                    groups = self.item_sell_table.len(),
                    "item_sell_table loaded"
                );
            }
            Err(e) => tracing::warn!("item_sell_table not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_special_sewing().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.special_sewing
                        .entry(r.npc_id as i32)
                        .or_default()
                        .push(r);
                }
                tracing::info!(
                    count,
                    npcs = self.special_sewing.len(),
                    "item_special_sewing loaded"
                );
            }
            Err(e) => tracing::warn!("item_special_sewing not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_item_smash().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_smash.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "item_smash loaded");
            }
            Err(e) => tracing::warn!("item_smash not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_special_stones().await {
            Ok(rows) => {
                for r in &rows {
                    self.special_stones.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "k_special_stone loaded");
            }
            Err(e) => tracing::warn!("k_special_stone not found or error: {e}, skipping"),
        }
        {
            let mr_repo =
                ko_db::repositories::monster_resource::MonsterResourceRepository::new(pool);
            match mr_repo.load_all().await {
                Ok(rows) => {
                    for r in &rows {
                        self.monster_resources.insert(r.sid, r.clone());
                    }
                    tracing::info!(count = rows.len(), "monster_resource loaded");
                }
                Err(e) => tracing::warn!("monster_resource not found or error: {e}, skipping"),
            }
        }
        match item_tables_repo.load_all_item_random().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_random.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "item_random loaded");
            }
            Err(e) => tracing::warn!("item_random not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_item_groups().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_groups.insert(r.group_id, r.clone());
                }
                tracing::info!(count = rows.len(), "item_group loaded");
            }
            Err(e) => tracing::warn!("item_group not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_item_exchange_exp().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_exchange_exp.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "item_exchange_exp loaded");
            }
            Err(e) => tracing::warn!("item_exchange_exp not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_item_give_exchange().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_give_exchange.insert(r.exchange_index, r.clone());
                }
                tracing::info!(count = rows.len(), "item_give_exchange loaded");
            }
            Err(e) => tracing::warn!("item_give_exchange not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_right_click_exchange().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_right_click_exchange.insert(r.item_id, r.clone());
                }
                tracing::info!(count = rows.len(), "item_right_click_exchange loaded");
            }
            Err(e) => tracing::warn!("item_right_click_exchange not found or error: {e}, skipping"),
        }
        match item_tables_repo.load_all_right_exchange().await {
            Ok(rows) => {
                for r in &rows {
                    self.item_right_exchange.insert(r.item_id, r.clone());
                }
                tracing::info!(count = rows.len(), "item_right_exchange loaded");
            }
            Err(e) => tracing::warn!("item_right_exchange not found or error: {e}, skipping"),
        }

        Ok(())
    }

    /// Load premium item types and exp tables.
    async fn load_premium_tables(&self, pool: &DbPool) {
        let premium_repo = PremiumRepository::new(pool);
        match premium_repo.load_all_premium_types().await {
            Ok(rows) => {
                for r in &rows {
                    self.premium_item_types
                        .insert(r.premium_type as u8, r.clone());
                }
                tracing::info!(count = rows.len(), "premium_item_types loaded");
            }
            Err(e) => tracing::warn!("premium_item_types not found or error: {e}, skipping"),
        }
        match premium_repo.load_all_premium_exp().await {
            Ok(rows) => {
                let count = rows.len();
                *self.premium_item_exp.write() = rows;
                tracing::info!(count, "premium_item_exp loaded");
            }
            Err(e) => tracing::warn!("premium_item_exp not found or error: {e}, skipping"),
        }
        match premium_repo.load_all_premium_gift_items().await {
            Ok(rows) => {
                let mut count = 0usize;
                for r in &rows {
                    let pt = r.premium_type.unwrap_or(0) as u8;
                    let gift = super::PremiumGiftItem {
                        item_id: r.bonus_item_num.unwrap_or(0) as u32,
                        count: r.count.unwrap_or(0) as u16,
                        sender: r.sender.clone().unwrap_or_default(),
                        subject: r.subject.clone().unwrap_or_default(),
                        message: r.message.clone().unwrap_or_default(),
                    };
                    self.premium_gift_items.entry(pt).or_default().push(gift);
                    count += 1;
                }
                tracing::info!(count, "premium_gift_items loaded");
            }
            Err(e) => tracing::warn!("premium_gift_items not found or error: {e}, skipping"),
        }
    }

    /// Load PUS (cash shop) categories and items.
    async fn load_cash_shop(&self, pool: &DbPool) {
        let cash_shop_repo = CashShopRepository::new(pool);
        match cash_shop_repo.load_all_categories().await {
            Ok(rows) => {
                for r in &rows {
                    self.pus_categories.insert(r.category_id, r.clone());
                }
                tracing::info!(count = rows.len(), "pus_categories loaded");
            }
            Err(e) => tracing::warn!("pus_categories not found or error: {e}, skipping"),
        }
        match cash_shop_repo.load_all_items().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.pus_items_by_id.insert(r.id, r.clone());
                    self.pus_items_by_category
                        .entry(r.category)
                        .or_default()
                        .push(r);
                }
                tracing::info!(
                    count,
                    categories = self.pus_items_by_category.len(),
                    "pus_items loaded"
                );
            }
            Err(e) => tracing::warn!("pus_items not found or error: {e}, skipping"),
        }
    }

    /// Load mining/fishing item tables.
    async fn load_mining_tables(&self, pool: &DbPool) {
        let mining_repo = MiningRepository::new(pool);
        match mining_repo.load_all_mining_items().await {
            Ok(rows) => {
                for r in &rows {
                    self.mining_fishing_items.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "mining_fishing_item table loaded");
            }
            Err(e) => tracing::warn!("mining_fishing_item table not found or error: {e}, skipping"),
        }
    }

    /// Load mining exchange (ore craft) table.
    async fn load_mining_exchange_table(&self, pool: &DbPool) {
        let mining_repo = MiningRepository::new(pool);
        match mining_repo.load_all_mining_exchanges().await {
            Ok(rows) => {
                for r in &rows {
                    self.mining_exchanges.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "mining_exchange table loaded");
            }
            Err(e) => tracing::warn!("mining_exchange table not found or error: {e}, skipping"),
        }
    }

    /// Load right-top title messages from DB.
    ///
    async fn load_right_top_titles(&self, pool: &DbPool) {
        let repo = ko_db::repositories::server_settings::ServerSettingsRepository::new(pool);
        match repo.load_right_top_titles().await {
            Ok(titles) => {
                let count = titles.len();
                self.set_right_top_titles(titles);
                tracing::info!(count, "right_top_title table loaded");
            }
            Err(e) => tracing::warn!("right_top_title table not found or error: {e}, skipping"),
        }
    }

    /// Load bot system tables (farm, merchant, user bots, rankings).
    async fn load_bot_tables(&self, pool: &DbPool) {
        let bot_repo = BotSystemRepository::new(pool);
        match bot_repo.load_all_farm_bots().await {
            Ok(rows) => {
                for r in &rows {
                    self.bot_farm_data.insert(r.id, r.clone());
                }
                tracing::info!(count = rows.len(), "bot_handler_farm table loaded");
            }
            Err(e) => tracing::warn!("bot_handler_farm table not found or error: {e}, skipping"),
        }
        match bot_repo.load_all_merchant_templates().await {
            Ok(rows) => {
                for r in &rows {
                    self.bot_merchant_templates.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "bot_handler_merchant table loaded");
            }
            Err(e) => {
                tracing::warn!("bot_handler_merchant table not found or error: {e}, skipping")
            }
        }
        match bot_repo.load_all_merchant_data().await {
            Ok(rows) => {
                for r in &rows {
                    self.bot_merchant_data.insert(r.n_index, r.clone());
                }
                tracing::info!(count = rows.len(), "bot_merchant_data table loaded");
            }
            Err(e) => tracing::warn!("bot_merchant_data table not found or error: {e}, skipping"),
        }
        match bot_repo.load_all_user_bots().await {
            Ok(rows) => {
                for r in &rows {
                    self.user_bots.insert(r.id, r.clone());
                }
                tracing::info!(count = rows.len(), "user_bots table loaded");
            }
            Err(e) => tracing::warn!("user_bots table not found or error: {e}, skipping"),
        }
        match bot_repo.load_all_knights_ranks().await {
            Ok(rows) => {
                let count = rows.len();
                *self.bot_knights_rank.write() = rows;
                tracing::info!(count, "bot_knights_rank table loaded");
            }
            Err(e) => tracing::warn!("bot_knights_rank table not found or error: {e}, skipping"),
        }
        match bot_repo.load_all_personal_ranks().await {
            Ok(rows) => {
                let count = rows.len();
                *self.bot_personal_rank.write() = rows;
                tracing::info!(count, "bot_personal_rank table loaded");
            }
            Err(e) => tracing::warn!("bot_personal_rank table not found or error: {e}, skipping"),
        }
    }

    /// Load user rankings from DB (update_ranks + load tables).
    ///
    pub async fn reload_user_rankings(&self, pool: &DbPool) {
        use ko_db::repositories::ranking::RankingRepository;

        let repo = RankingRepository::new(pool);

        // Step 1: Recalculate rankings in DB
        if let Err(e) = repo.update_ranks().await {
            tracing::warn!("update_ranks() failed: {e}");
            return;
        }

        // Step 2: Load personal ranks
        let mut personal_map = std::collections::HashMap::new();
        match repo.load_user_personal_rank().await {
            Ok(rows) => {
                for row in &rows {
                    let rank = row.rank_pos as u8;
                    if !row.karus_user_id.is_empty() {
                        personal_map.insert(row.karus_user_id.to_uppercase(), rank);
                    }
                    if !row.elmo_user_id.is_empty() {
                        personal_map.insert(row.elmo_user_id.to_uppercase(), rank);
                    }
                }
                tracing::info!(count = personal_map.len(), "user_personal_rank loaded");
            }
            Err(e) => tracing::warn!("user_personal_rank load error: {e}"),
        }
        *self.user_personal_rank.write() = personal_map;

        // Step 3: Load knights ranks
        let mut knights_map = std::collections::HashMap::new();
        match repo.load_user_knights_rank().await {
            Ok(rows) => {
                for row in &rows {
                    let rank = row.rank_pos as u8;
                    if !row.karus_user_id.is_empty() {
                        knights_map.insert(row.karus_user_id.to_uppercase(), rank);
                    }
                    if !row.elmo_user_id.is_empty() {
                        knights_map.insert(row.elmo_user_id.to_uppercase(), rank);
                    }
                }
                tracing::info!(count = knights_map.len(), "user_knights_rank loaded");
            }
            Err(e) => tracing::warn!("user_knights_rank load error: {e}"),
        }
        *self.user_knights_rank.write() = knights_map;

        // Step 3b: Compute and apply knights (clan) ratings
        //
        // Every 15 minutes, recompute per-nation clan rankings from points.
        // Updates knights_rating table AND knights.ranking column in DB,
        // then applies to in-memory KnightsInfo for each clan.
        if let Err(e) = repo.compute_knights_rating().await {
            tracing::warn!("compute_knights_rating() failed: {e}");
        } else {
            match repo.load_knights_rating().await {
                Ok(rows) => {
                    use crate::clan_constants::CLAN_TYPE_ACCREDITED5;
                    use crate::systems::daily_reset::get_knights_grade;

                    // Reset all clan rankings to 0
                    let clan_ids = self.get_all_knights_ids();
                    for clan_id in &clan_ids {
                        self.update_knights(*clan_id, |k| k.ranking = 0);
                    }

                    // Apply computed rankings and recompute grade
                    let mut updated = 0u32;
                    for row in &rows {
                        let clan_id = row.clan_id as u16;
                        let rank = row.rank_pos as u8;
                        let points = row.points as u32;
                        self.update_knights(clan_id, |k| {
                            k.ranking = rank;
                            // Recompute grade: Accredited+ clans always grade 1
                            let new_grade = if k.flag >= CLAN_TYPE_ACCREDITED5 {
                                1
                            } else {
                                get_knights_grade(points)
                            };
                            if k.grade != new_grade {
                                k.grade = new_grade;
                            }
                        });
                        updated += 1;
                    }
                    tracing::info!(
                        count = updated,
                        "knights_rating applied: clan rankings + grades updated"
                    );
                }
                Err(e) => tracing::warn!("load_knights_rating() failed: {e}"),
            }
        }

        // Step 4: Reload bot rankings (C++ also reloads bot rank tables)
        let bot_repo = ko_db::repositories::bot_system::BotSystemRepository::new(pool);
        match bot_repo.load_all_knights_ranks().await {
            Ok(rows) => {
                let count = rows.len();
                *self.bot_knights_rank.write() = rows;
                tracing::debug!(count, "bot_knights_rank reloaded");
            }
            Err(e) => tracing::warn!("bot_knights_rank reload error: {e}"),
        }
        match bot_repo.load_all_personal_ranks().await {
            Ok(rows) => {
                let count = rows.len();
                *self.bot_personal_rank.write() = rows;
                tracing::debug!(count, "bot_personal_rank reloaded");
            }
            Err(e) => tracing::warn!("bot_personal_rank reload error: {e}"),
        }

        // Step 5: Apply ranks to online sessions and bots
        self.apply_user_ranks_to_sessions();
        self.apply_user_ranks_to_bots();

        // Step 6: Recompute daily ranks and reload cache
        let dr_repo = ko_db::repositories::daily_rank::DailyRankRepository::new(pool);
        if let Err(e) = dr_repo.compute_ranks().await {
            tracing::warn!("compute_daily_ranks() failed during reload: {e}");
        }
        match dr_repo.load_all().await {
            Ok(rows) => {
                let count = rows.len();
                *self.daily_rank_cache.write() = rows;
                tracing::info!(count, "daily_rank reloaded");
            }
            Err(e) => tracing::warn!("daily_rank reload error: {e}"),
        }
    }

    /// Load pet stats and image change tables.
    async fn load_pet_tables(&self, pool: &DbPool) {
        let pet_repo = PetRepository::new(pool);
        match pet_repo.load_all_stats_info().await {
            Ok(rows) => {
                for r in &rows {
                    self.pet_stats_info.insert(r.pet_level as u8, r.clone());
                }
                tracing::info!(count = rows.len(), "pet_stats_info table loaded");
            }
            Err(e) => tracing::warn!("pet_stats_info table not found or error: {e}, skipping"),
        }
        match pet_repo.load_all_image_changes().await {
            Ok(rows) => {
                for r in &rows {
                    self.pet_image_changes.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "pet_image_change table loaded");
            }
            Err(e) => tracing::warn!("pet_image_change table not found or error: {e}, skipping"),
        }
    }

    /// Load achievement tables (main, war, normal, monster, com, title).
    async fn load_achievement_tables(&self, pool: &DbPool) {
        let achieve_repo = AchieveRepository::new(pool);
        match achieve_repo.load_achieve_main().await {
            Ok(rows) => {
                for r in &rows {
                    self.achieve_main.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "achieve_main table loaded");
            }
            Err(e) => tracing::warn!("achieve_main table not found or error: {e}, skipping"),
        }
        match achieve_repo.load_achieve_war().await {
            Ok(rows) => {
                for r in &rows {
                    self.achieve_war.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "achieve_war table loaded");
            }
            Err(e) => tracing::warn!("achieve_war table not found or error: {e}, skipping"),
        }
        match achieve_repo.load_achieve_normal().await {
            Ok(rows) => {
                for r in &rows {
                    self.achieve_normal.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "achieve_normal table loaded");
            }
            Err(e) => tracing::warn!("achieve_normal table not found or error: {e}, skipping"),
        }
        match achieve_repo.load_achieve_monster().await {
            Ok(rows) => {
                for r in &rows {
                    self.achieve_monster.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "achieve_monster table loaded");
            }
            Err(e) => tracing::warn!("achieve_monster table not found or error: {e}, skipping"),
        }
        match achieve_repo.load_achieve_com().await {
            Ok(rows) => {
                for r in &rows {
                    self.achieve_com.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "achieve_com table loaded");
            }
            Err(e) => tracing::warn!("achieve_com table not found or error: {e}, skipping"),
        }
        match achieve_repo.load_achieve_title().await {
            Ok(rows) => {
                for r in &rows {
                    self.achieve_title.insert(r.s_index, r.clone());
                }
                tracing::info!(count = rows.len(), "achieve_title table loaded");
            }
            Err(e) => tracing::warn!("achieve_title table not found or error: {e}, skipping"),
        }
    }

    /// Load magic tables (master + type1..9).
    async fn load_magic_tables(&self, pool: &DbPool) -> anyhow::Result<()> {
        let magic_repo = MagicRepository::new(pool);

        let rows = magic_repo.load_magic_table().await?;
        for r in &rows {
            self.magic_table.insert(r.magic_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic table loaded");

        let rows = magic_repo.load_magic_type1().await?;
        for r in &rows {
            self.magic_type1.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type1 loaded");

        let rows = magic_repo.load_magic_type2().await?;
        for r in &rows {
            self.magic_type2.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type2 loaded");

        let rows = magic_repo.load_magic_type3().await?;
        for r in &rows {
            self.magic_type3.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type3 loaded");

        let rows = magic_repo.load_magic_type4().await?;
        for r in &rows {
            self.magic_type4.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type4 loaded");

        let rows = magic_repo.load_magic_type5().await?;
        for r in &rows {
            self.magic_type5.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type5 loaded");

        let rows = magic_repo.load_magic_type6().await?;
        for r in &rows {
            self.magic_type6.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type6 loaded");

        let rows = magic_repo.load_magic_type7().await?;
        for r in &rows {
            self.magic_type7.insert(r.n_index, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type7 loaded");

        let rows = magic_repo.load_magic_type8().await?;
        for r in &rows {
            self.magic_type8.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type8 loaded");

        let rows = magic_repo.load_magic_type9().await?;
        for r in &rows {
            self.magic_type9.insert(r.i_num, r.clone());
        }
        tracing::info!(count = rows.len(), "magic_type9 loaded");

        Ok(())
    }

    /// Load zone configs, SMD files, events, and object events.
    async fn load_zones(&self, pool: &DbPool, map_dir: &Path) -> anyhow::Result<()> {
        let repo = ZoneRepository::new(pool);

        let zone_rows = repo.load_all_zones().await?;
        tracing::info!("loaded {} zone configs from database", zone_rows.len());

        let event_rows = repo.load_all_events().await?;
        tracing::info!("loaded {} game events from database", event_rows.len());

        let mut events_by_zone: HashMap<i16, Vec<ko_db::models::GameEventRow>> = HashMap::new();
        for row in event_rows {
            events_by_zone.entry(row.zone_no).or_default().push(row);
        }

        let obj_event_rows = repo.load_all_object_events().await?;
        tracing::info!(
            "loaded {} object events from database",
            obj_event_rows.len()
        );

        let mut obj_events_by_zone: HashMap<i16, HashMap<i16, ObjectEventInfo>> = HashMap::new();
        for row in &obj_event_rows {
            let info = ObjectEventInfo {
                index: row.id,
                zone_id: row.zone_id as u16,
                belong: row.belong,
                s_index: row.s_index,
                obj_type: row.obj_type,
                control_npc: row.control_npc,
                status: row.status,
                pos_x: row.pos_x,
                pos_y: row.pos_y,
                pos_z: row.pos_z,
                by_life: row.by_life,
            };
            // Key by s_index — C++ uses GetData(objectindex) which searches by s_index
            obj_events_by_zone
                .entry(row.zone_id)
                .or_default()
                .insert(row.s_index, info);
        }

        let mut loaded_count = 0u32;
        let mut smd_loaded = 0u32;
        let mut smd_failed = 0u32;

        for row in &zone_rows {
            let zone_id = row.zone_no as u16;
            let zone_info = zone_info_from_row(row);

            let smd_path = map_dir.join(&row.smd_name);
            let map_data = match SmdFile::load(&smd_path) {
                Ok(smd) => {
                    tracing::debug!(zone_id, smd_name = %row.smd_name, map_size = smd.map_size, map_width = smd.map_width, warps = smd.warps.len(), "SMD loaded");
                    smd_loaded += 1;
                    Some(crate::zone::MapData::new(smd))
                }
                Err(e) => {
                    tracing::warn!(zone_id, smd_name = %row.smd_name, error = %e, "SMD file not found or failed to parse");
                    smd_failed += 1;
                    None
                }
            };

            let events = match events_by_zone.remove(&row.zone_no) {
                Some(rows) => events_from_rows(&rows),
                None => HashMap::new(),
            };

            let obj_events = obj_events_by_zone.remove(&row.zone_no).unwrap_or_default();

            let zone = ZoneState::new_with_data(zone_id, zone_info, map_data, events, obj_events);
            self.zones.insert(zone_id, Arc::new(zone));
            loaded_count += 1;
        }

        tracing::info!(
            zones = loaded_count,
            smd_ok = smd_loaded,
            smd_fail = smd_failed,
            "zones initialized"
        );
        Ok(())
    }

    /// Load NPC templates and spawn instances.
    async fn load_npcs_and_monsters(&self, pool: &DbPool) -> anyhow::Result<()> {
        let npc_repo = NpcRepository::new(pool);

        let template_rows = npc_repo.load_all_templates().await?;
        for row in &template_rows {
            let tmpl = Arc::new(NpcTemplate {
                s_sid: row.s_sid as u16,
                is_monster: row.is_monster,
                name: row.str_name.clone().unwrap_or_default(),
                pid: row.s_pid as u16,
                size: row.s_size as u16,
                weapon_1: row.i_weapon_1 as u32,
                weapon_2: row.i_weapon_2 as u32,
                group: row.by_group as u8,
                act_type: row.by_act_type as u8,
                npc_type: row.by_type as u8,
                family_type: row.by_family as u8,
                selling_group: row.i_selling_group as u32,
                level: row.s_level as u8,
                max_hp: row.i_hp_point as u32,
                max_mp: row.s_mp_point as u16,
                attack: row.s_atk as u16,
                ac: row.s_ac as u16,
                hit_rate: row.s_hit_rate as u16,
                evade_rate: row.s_evade_rate as u16,
                damage: row.s_damage as u16,
                attack_delay: row.s_attack_delay as u16,
                speed_1: row.by_speed_1 as u16,
                speed_2: row.by_speed_2 as u16,
                stand_time: row.s_standtime as u16,
                search_range: row.by_search_range as u8,
                attack_range: row.by_attack_range as u8,
                direct_attack: row.by_direct_attack as u8,
                tracing_range: row.by_tracing_range as u8,
                magic_1: row.i_magic_1.max(0) as u32,
                magic_2: row.i_magic_2.max(0) as u32,
                magic_3: row.i_magic_3.max(0) as u32,
                magic_attack: row.by_magic_attack as u8,
                fire_r: row.s_fire_r,
                cold_r: row.s_cold_r,
                lightning_r: row.s_lightning_r,
                magic_r: row.s_magic_r,
                disease_r: row.s_disease_r,
                poison_r: row.s_poison_r,
                exp: row.i_exp.max(0) as u32,
                loyalty: row.i_loyalty.max(0) as u32,
                money: row.i_money.max(0) as u32,
                item_table: row.s_item,
                area_range: row.area_range,
            });
            self.npc_templates
                .insert((tmpl.s_sid, tmpl.is_monster), tmpl);
        }
        tracing::info!(templates = template_rows.len(), "NPC templates loaded");

        let spawn_rows = npc_repo.load_all_spawns().await?;
        let mut total_instances = 0u32;
        let mut skipped_spawns = 0u32;

        for spawn in &spawn_rows {
            let zone_id = spawn.zone_id as u16;
            let npc_id = spawn.npc_id as u16;

            let tmpl = self
                .npc_templates
                .get(&(npc_id, spawn.is_monster))
                .map(|t| t.clone())
                .or_else(|| {
                    self.npc_templates
                        .get(&(npc_id, !spawn.is_monster))
                        .map(|t| t.clone())
                });

            let tmpl = match tmpl {
                Some(t) => t,
                None => {
                    skipped_spawns += 1;
                    continue;
                }
            };

            let zone = match self.get_zone(zone_id) {
                Some(z) => z,
                None => {
                    skipped_spawns += 1;
                    continue;
                }
            };

            let num = spawn.num_npc.max(1) as u32;
            let base_x = spawn.left_x as f32;
            let base_z = spawn.top_z as f32;
            let range = spawn.spawn_range as f32;

            for i in 0..num {
                let nid = self.allocate_npc_id();

                let (x, z) = if num == 1 || range <= 0.0 {
                    (base_x, base_z)
                } else {
                    let offset = (i as f32 / num as f32) * std::f32::consts::TAU;
                    let scatter = range * 0.5;
                    (
                        base_x + offset.cos() * scatter,
                        base_z + offset.sin() * scatter,
                    )
                };

                let region_x = calc_region(x);
                let region_z = calc_region(z);

                // spawn.room as their event_room for instance isolation.
                let event_room = if spawn.room > 0 { spawn.room as u16 } else { 0 };

                let instance = Arc::new(NpcInstance {
                    nid,
                    proto_id: npc_id,
                    is_monster: tmpl.is_monster,
                    zone_id,
                    x,
                    y: 0.0,
                    z,
                    direction: spawn.direction.rem_euclid(256) as u8,
                    region_x,
                    region_z,
                    gate_open: 0,
                    object_type: 0,
                    nation: if tmpl.is_monster { 0 } else { tmpl.group },
                    special_type: spawn.special_type,
                    trap_number: spawn.trap_number,
                    event_room,
                    is_event_npc: false,
                    summon_type: 0,
                    user_name: String::new(),
                    pet_name: String::new(),
                    clan_name: String::new(),
                    clan_id: 0,
                    clan_mark_version: 0,
                });

                zone.add_npc(region_x, region_z, nid);
                self.npc_instances.insert(nid, instance);
                // Non-monster NPCs (merchants, event NPCs, etc.) never die in combat.
                // If their template HP is 0, use 1 so is_npc_dead() doesn't filter them out.
                let init_hp = if !tmpl.is_monster && tmpl.max_hp == 0 {
                    1
                } else {
                    tmpl.max_hp as i32
                };
                self.npc_hp.insert(nid, init_hp);

                if tmpl.is_monster && tmpl.search_range > 0 {
                    let regen_ms = if spawn.regen_time > 0 {
                        spawn.regen_time as u64 * 1000
                    } else {
                        30_000
                    };
                    let has_friends = matches!(tmpl.act_type, 3 | 4)
                        && !matches!(
                            zone_id,
                            ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
                        );
                    self.npc_ai.insert(
                        nid,
                        NpcAiState {
                            state: NpcState::Standing,
                            spawn_x: x,
                            spawn_z: z,
                            cur_x: x,
                            cur_z: z,
                            target_id: None,
                            npc_target_id: None,
                            delay_ms: tmpl.stand_time as u64,
                            last_tick_ms: 0,
                            regen_time_ms: regen_ms,
                            is_aggressive: !matches!(tmpl.act_type, 1..=4),
                            zone_id,
                            region_x,
                            region_z,
                            fainting_until_ms: 0,
                            old_state: NpcState::Standing,
                            active_skill_id: 0,
                            active_target_id: -1,
                            active_cast_time_ms: 0,
                            has_friends,
                            family_type: tmpl.family_type,
                            skill_cooldown_ms: 0,
                            nation: tmpl.group,
                            is_tower_owner: false,
                            attack_type: map_act_type(tmpl.act_type),
                            last_combat_time_ms: 0,
                            duration_secs: 0,
                            spawned_at_ms: 0,
                            last_hp_regen_ms: 0,
                            gate_open: 0,
                            wood_cooldown_count: 0,
                            utc_second: 0,
                            path_waypoints: Vec::new(),
                            path_index: 0,
                            path_target_x: 0.0,
                            path_target_z: 0.0,
                            path_is_direct: false,
                            dest_x: 0.0,
                            dest_z: 0.0,
                            pattern_frame: 0,
                        },
                    );
                }

                // Guard NPCs need AI to detect and attack enemy players.
                //   CheckFindEnemy() → FindEnemy() in the AI loop.
                if is_guard_npc_type(tmpl.npc_type) && !self.npc_ai.contains_key(&nid) {
                    let regen_ms = if spawn.regen_time > 0 {
                        spawn.regen_time as u64 * 1000
                    } else {
                        30_000
                    };
                    self.npc_ai.insert(
                        nid,
                        NpcAiState {
                            state: NpcState::Standing,
                            spawn_x: x,
                            spawn_z: z,
                            cur_x: x,
                            cur_z: z,
                            target_id: None,
                            npc_target_id: None,
                            delay_ms: tmpl.stand_time as u64,
                            last_tick_ms: 0,
                            regen_time_ms: regen_ms,
                            // Guards are always aggressive
                            is_aggressive: true,
                            zone_id,
                            region_x,
                            region_z,
                            fainting_until_ms: 0,
                            old_state: NpcState::Standing,
                            active_skill_id: 0,
                            active_target_id: -1,
                            active_cast_time_ms: 0,
                            has_friends: false,
                            family_type: tmpl.family_type,
                            skill_cooldown_ms: 0,
                            nation: tmpl.group,
                            is_tower_owner: false,
                            attack_type: map_act_type(tmpl.act_type),
                            last_combat_time_ms: 0,
                            duration_secs: 0,
                            spawned_at_ms: 0,
                            last_hp_regen_ms: 0,
                            gate_open: 0,
                            wood_cooldown_count: 0,
                            utc_second: 0,
                            path_waypoints: Vec::new(),
                            path_index: 0,
                            path_target_x: 0.0,
                            path_target_z: 0.0,
                            path_is_direct: false,
                            dest_x: 0.0,
                            dest_z: 0.0,
                            pattern_frame: 0,
                        },
                    );
                }

                if is_gate_npc_type(tmpl.npc_type) && !self.npc_ai.contains_key(&nid) {
                    let stand_time = if tmpl.stand_time > 0 {
                        tmpl.stand_time as u64
                    } else {
                        3000
                    };
                    self.npc_ai.insert(
                        nid,
                        NpcAiState {
                            state: NpcState::Standing,
                            spawn_x: x,
                            spawn_z: z,
                            cur_x: x,
                            cur_z: z,
                            target_id: None,
                            npc_target_id: None,
                            delay_ms: stand_time,
                            last_tick_ms: 0,
                            regen_time_ms: 0,
                            is_aggressive: false,
                            zone_id,
                            region_x,
                            region_z,
                            fainting_until_ms: 0,
                            old_state: NpcState::Standing,
                            active_skill_id: 0,
                            active_target_id: -1,
                            active_cast_time_ms: 0,
                            has_friends: false,
                            family_type: 0,
                            skill_cooldown_ms: 0,
                            nation: tmpl.group,
                            is_tower_owner: false,
                            attack_type: 0,
                            last_combat_time_ms: 0,
                            duration_secs: 0,
                            spawned_at_ms: 0,
                            last_hp_regen_ms: 0,
                            gate_open: 0,
                            wood_cooldown_count: 0,
                            utc_second: 0,
                            path_waypoints: Vec::new(),
                            path_index: 0,
                            path_target_x: 0.0,
                            path_target_z: 0.0,
                            path_is_direct: false,
                            dest_x: 0.0,
                            dest_z: 0.0,
                            pattern_frame: 0,
                        },
                    );
                }

                total_instances += 1;
            }
        }

        if skipped_spawns > 0 {
            tracing::warn!(
                skipped = skipped_spawns,
                "NPC spawns skipped (missing template or zone)"
            );
        }
        tracing::info!(
            templates = template_rows.len(),
            instances = total_instances,
            "NPC instances created"
        );

        // ── Spawn NPC instances from object_event_pos ───────────────────
        // Objects like gates, anvils, bind points, etc. are spawned as static NPCs
        // using their s_index as the template ID.
        let mut obj_npc_count = 0u32;
        let mut obj_npc_skipped = 0u32;
        for zone_entry in self.zones.iter() {
            let zone = zone_entry.value().clone();
            let zone_id = zone.zone_id;
            let events: Vec<crate::zone::ObjectEventInfo> =
                zone.object_events.values().cloned().collect();
            for event in &events {
                // C++ _LoadAllObjects (NpcThread.cpp:541-552): only these types
                // are spawned as object NPCs.
                // OBJECT_BIND(0), OBJECT_GATE(1), OBJECT_GATE2(2),
                // OBJECT_GATE_LEVER(3), OBJECT_WALL(6), OBJECT_ANVIL(8),
                // OBJECT_ARTIFACT(9), OBJECT_KROWASGATE(12),
                // OBJECT_POISONGAS(13), OBJECT_WOOD(14),
                // OBJECT_WOOD_LEVER(15), OBJECT_EFECKT(50)
                let should_spawn = matches!(
                    event.obj_type,
                    0 | 1 | 2 | 3 | 6 | 8 | 9 | 12 | 13 | 14 | 15 | 50
                );
                if !should_spawn {
                    continue;
                }

                // C++ _AddObjectEventNpc (NpcThread.cpp:566-581):
                // For "object" types, template_id = s_index; otherwise = control_npc
                let is_object = matches!(event.obj_type, 0 | 1 | 2 | 3 | 6 | 8 | 9 | 14 | 15 | 50);
                let template_id = if is_object {
                    event.s_index as u16
                } else {
                    event.control_npc as u16
                };

                if template_id == 0 {
                    obj_npc_skipped += 1;
                    continue;
                }

                // Look up the NPC template (try both is_monster flags)
                let tmpl = self
                    .npc_templates
                    .get(&(template_id, false))
                    .map(|t| t.clone())
                    .or_else(|| {
                        self.npc_templates
                            .get(&(template_id, true))
                            .map(|t| t.clone())
                    });
                let tmpl = match tmpl {
                    Some(t) => t,
                    None => {
                        obj_npc_skipped += 1;
                        continue;
                    }
                };

                let nid = self.allocate_npc_id();
                let x = event.pos_x;
                let z = event.pos_z;
                let region_x = calc_region(x);
                let region_z = calc_region(z);

                // C++ _AddObjectEventNpc: m_byObjectType = SPECIAL_OBJECT (1)
                // gate_open = event.status, move_type = 104 (stationary)
                let instance = Arc::new(NpcInstance {
                    nid,
                    proto_id: template_id,
                    is_monster: false,
                    zone_id,
                    x,
                    y: event.pos_y,
                    z,
                    direction: 0,
                    region_x,
                    region_z,
                    gate_open: event.status as u8,
                    object_type: 1, // SPECIAL_OBJECT
                    nation: tmpl.group,
                    special_type: 0,
                    trap_number: 0,
                    event_room: 0,
                    is_event_npc: false,
                    summon_type: 0,
                    user_name: String::new(),
                    pet_name: String::new(),
                    clan_name: String::new(),
                    clan_id: 0,
                    clan_mark_version: 0,
                });

                zone.add_npc(region_x, region_z, nid);
                self.npc_instances.insert(nid, instance);
                let init_hp = if tmpl.max_hp == 0 {
                    1
                } else {
                    tmpl.max_hp as i32
                };
                self.npc_hp.insert(nid, init_hp);
                obj_npc_count += 1;
            }
        }
        if obj_npc_skipped > 0 {
            tracing::info!(
                skipped = obj_npc_skipped,
                "object event NPCs skipped (missing template — normal for bind stones in some zones)"
            );
        }
        tracing::info!(
            object_event_npcs = obj_npc_count,
            "object event NPC instances created (anvils, gates, bind points)"
        );

        // Diagnostic: count NPCs actually in zone region grids
        let mut total_in_regions = 0u32;
        for entry in self.zones.iter() {
            let zone = entry.value();
            let mut zone_count = 0u32;
            for rx in 0..zone.max_region_x {
                for rz in 0..zone.max_region_z {
                    if let Some(region) = zone.get_region(rx, rz) {
                        let count = region.npcs.read().len() as u32;
                        zone_count += count;
                    }
                }
            }
            if zone_count > 0 {
                tracing::info!(
                    zone_id = zone.zone_id,
                    npcs_in_regions = zone_count,
                    grid = zone.max_region_x,
                    "zone NPC region count"
                );
            }
            total_in_regions += zone_count;
        }
        tracing::info!(
            total_in_regions,
            total_instances,
            "NPC region placement summary"
        );

        Ok(())
    }

    /// Load knights (clans) and alliances.
    async fn load_knights(&self, pool: &DbPool) -> anyhow::Result<()> {
        let knights_repo = KnightsRepository::new(pool);
        let knights_rows = knights_repo.load_all().await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        for row in &knights_rows {
            let valid_mark =
                row.s_mark_version > 0 && row.s_mark_len == 2400 && row.mark.len() == 2400;
            if row.s_mark_version > 0 && !valid_mark {
                tracing::warn!(
                    clan_id = row.id_num,
                    mark_version = row.s_mark_version,
                    s_mark_len = row.s_mark_len,
                    mark_bytes = row.mark.len(),
                    "invalid clan mark ignored while loading knights"
                );
            }
            let info = KnightsInfo {
                id: row.id_num as u16,
                flag: row.flag as u8,
                nation: row.nation as u8,
                // Accredited and Royal clan grades are rank-driven and the
                // client expects grade 1 even before KNIGHTS_RATING reloads.
                // Training/Promoted clans retain the point-based grade.
                grade: if row.flag as u8 >= CLAN_TYPE_ACCREDITED5 {
                    1
                } else {
                    get_knights_grade(row.points.max(0) as u32)
                },
                ranking: row.ranking as u8,
                name: row.id_name.clone(),
                chief: row.chief.clone(),
                vice_chief_1: row.vice_chief_1.clone().unwrap_or_default(),
                vice_chief_2: row.vice_chief_2.clone().unwrap_or_default(),
                vice_chief_3: row.vice_chief_3.clone().unwrap_or_default(),
                members: row.members as u16,
                points: row.points as u32,
                clan_point_fund: row.clan_point_fund as u32,
                notice: row.str_clan_notice.clone().unwrap_or_default(),
                cape: row.s_cape as u16,
                cape_r: row.b_cape_r as u8,
                cape_g: row.b_cape_g as u8,
                cape_b: row.b_cape_b as u8,
                mark_version: if valid_mark {
                    row.s_mark_version as u16
                } else {
                    0
                },
                mark_data: if valid_mark {
                    row.mark.clone()
                } else {
                    Vec::new()
                },
                alliance: row.s_alliance_knights as u16,
                castellan_cape: row.s_cast_cape >= 0 && i64::from(row.b_cast_time) >= now,
                cast_cape_id: row.s_cast_cape,
                cast_cape_r: row.b_cast_cape_r as u8,
                cast_cape_g: row.b_cast_cape_g as u8,
                cast_cape_b: row.b_cast_cape_b as u8,
                cast_cape_time: row.b_cast_time as u32,
                alliance_req: 0,
                clan_point_method: row.clan_point_method as u8,
                premium_time: row.s_premium_time as u32,
                premium_in_use: row.s_premium_in_use as u8,
                online_members: 0,
                online_np_count: 0,
                online_exp_count: 0,
            };
            self.knights.insert(info.id, info);
        }
        tracing::info!(count = knights_rows.len(), "knights (clans) loaded");

        let alliance_rows = knights_repo.load_all_alliances().await?;
        for row in &alliance_rows {
            let alliance = KnightsAlliance {
                main_clan: row.s_main_alliance_knights as u16,
                sub_clan: row.s_sub_alliance_knights as u16,
                mercenary_1: row.s_mercenary_clan_1 as u16,
                mercenary_2: row.s_mercenary_clan_2 as u16,
                notice: row.str_alliance_notice.clone(),
            };
            self.alliances.insert(alliance.main_clan, alliance);
        }
        tracing::info!(count = alliance_rows.len(), "alliances loaded");
        Ok(())
    }

    /// Load quest helper, monster, menu, and talk tables.
    async fn load_quest_tables(&self, pool: &DbPool) -> anyhow::Result<()> {
        let quest_repo = QuestRepository::new(pool);
        let helper_rows = quest_repo.load_quest_helpers().await?;
        for row in &helper_rows {
            let n_index = row.n_index as u32;
            let npc_id = row.s_npc_id as u16;
            self.quest_npc_list.entry(npc_id).or_default().push(n_index);
            self.quest_helpers.insert(n_index, row.clone());
        }
        tracing::info!(count = helper_rows.len(), "quest_helper table loaded");

        // Read collection requirements, never exchange rewards. Zone 19 is
        // the shared Eslant quest catalogue (Beldan/Agata), not a live map.
        let collection_items: Vec<i32> = sqlx::query_scalar(
            "SELECT DISTINCT x.item FROM quest_helper q \
             JOIN item_exchange e ON e.n_index=q.n_exchange_index \
             CROSS JOIN LATERAL (VALUES \
             (e.origin_item_num1,e.origin_item_count1), \
             (e.origin_item_num2,e.origin_item_count2), \
             (e.origin_item_num3,e.origin_item_count3), \
             (e.origin_item_num4,e.origin_item_count4), \
             (e.origin_item_num5,e.origin_item_count5)) x(item,qty) \
             WHERE q.b_zone IN (1,11,12,19) AND x.item>=100000000 AND x.qty>0",
        )
        .fetch_all(pool)
        .await?;
        self.collection_drop_items.clear();
        for item in collection_items {
            self.collection_drop_items.insert(item as u32, ());
        }
        tracing::info!(
            count = self.collection_drop_items.len(),
            "Collection drop policy loaded: Luferson/Eslant x1.60; other monster items x1.15"
        );

        let monster_rows = quest_repo.load_quest_monsters().await?;
        for row in &monster_rows {
            self.quest_monsters
                .insert(row.s_quest_num as u16, row.clone());
        }
        tracing::info!(count = monster_rows.len(), "quest_monster table loaded");

        let qt_repo = QuestTextRepository::new(pool);
        let menu_rows = qt_repo.load_quest_menus().await?;
        for row in &menu_rows {
            self.quest_menus.insert(row.i_num, row.clone());
        }
        tracing::info!(count = menu_rows.len(), "quest_menu table loaded");

        let talk_rows = qt_repo.load_quest_talks().await?;
        for row in &talk_rows {
            self.quest_talks.insert(row.i_num, row.clone());
        }
        tracing::info!(count = talk_rows.len(), "quest_talk table loaded");

        let closed_check_rows = qt_repo.load_quest_skills_closed_check().await?;
        for row in &closed_check_rows {
            self.quest_skills_closed_check
                .insert(row.n_index, row.clone());
        }
        tracing::info!(
            count = closed_check_rows.len(),
            "quest_skills_closed_check loaded"
        );

        let open_set_up_rows = qt_repo.load_quest_skills_open_set_up().await?;
        for row in &open_set_up_rows {
            self.quest_skills_open_set_up
                .insert(row.n_index, row.clone());
        }
        tracing::info!(
            count = open_set_up_rows.len(),
            "quest_skills_open_set_up loaded"
        );
        Ok(())
    }

    /// Load server settings, damage settings, burning features, home positions.
    async fn load_server_settings(&self, pool: &DbPool) {
        let ss_repo = ServerSettingsRepository::new(pool);
        match ss_repo.load_server_settings().await {
            Ok(row) => {
                tracing::info!(
                    "server_settings loaded (max_level={}, max_player_hp={})",
                    row.maximum_level,
                    row.max_player_hp
                );
                // Wire auto_wanted → wanted_auto_enabled (wanted event lifecycle tick).
                if row.auto_wanted != 0 {
                    self.wanted_auto_enabled
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    tracing::info!("wanted_auto_enabled=true (from server_settings.auto_wanted)");
                }
                *self.server_settings.write() = Some(row);
            }
            Err(e) => tracing::warn!("server_settings not found or error: {e}, using defaults"),
        }
        match ss_repo.load_damage_settings().await {
            Ok(row) => {
                tracing::info!(
                    "damage_settings loaded (mon_take_damage={}, r_damage={}, mage_magic={})",
                    row.mon_take_damage,
                    row.r_damage,
                    row.mage_magic_damage
                );
                *self.damage_settings.write() = Some(row);
            }
            Err(e) => tracing::warn!("damage_settings not found or error: {e}, using defaults"),
        }
        let burning_repo = BurningRepository::new(pool);
        match burning_repo.load_all().await {
            Ok(rows) => {
                let mut features = [BurningFeatureRates::default(); 3];
                for row in &rows {
                    let idx = row.burn_level as usize;
                    if (1..=3).contains(&idx) {
                        features[idx - 1] = BurningFeatureRates {
                            np_rate: row.np_rate as u8,
                            money_rate: row.money_rate as u8,
                            exp_rate: row.exp_rate as u8,
                            drop_rate: row.drop_rate as u8,
                        };
                    }
                }
                *self.burning_features.write() = features;
                tracing::info!(count = rows.len(), "burning_features loaded");
            }
            Err(e) => tracing::warn!("burning_features not found or error: {e}, using defaults"),
        }
        match ss_repo.load_home_positions().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.home_positions.insert(r.nation as u8, r);
                }
                tracing::info!(count, "home positions loaded");
            }
            Err(e) => tracing::warn!("home table not found or error: {e}, skipping"),
        }
        // Load start_position table (per-zone nation-specific spawn coords)
        match ss_repo.load_start_positions().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.start_positions.insert(r.zone_id as u16, r);
                }
                tracing::info!(count, "start_positions loaded");
            }
            Err(e) => tracing::warn!("start_position table not found or error: {e}, skipping"),
        }
        // Load start_position_random table (random spawn points for special zones)
        match ss_repo.load_start_positions_random().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.start_positions_random
                        .entry(r.zone_id as u16)
                        .or_default()
                        .push(r);
                }
                tracing::info!(count, "start_position_random loaded");
            }
            Err(e) => {
                tracing::warn!("start_position_random table not found or error: {e}, skipping")
            }
        }
        // Load persistent send_messages (type=1) for login broadcast
        let sched_repo = ko_db::repositories::scheduled_tasks::ScheduledTasksRepository::new(pool);
        match sched_repo.load_messages().await {
            Ok(rows) => {
                let login_msgs: Vec<_> = rows.into_iter().filter(|m| m.send_type == 1).collect();
                tracing::info!(count = login_msgs.len(), "send_messages (type=1) loaded");
                *self.send_messages.write() = login_msgs;
            }
            Err(e) => tracing::warn!("send_messages not found or error: {e}, skipping"),
        }

        // Load automatic commands
        match sched_repo.load_active_commands().await {
            Ok(rows) => {
                tracing::info!(count = rows.len(), "automatic_commands loaded");
                *self.automatic_commands.write() = rows;
            }
            Err(e) => tracing::warn!("automatic_commands not found or error: {e}, skipping"),
        }
    }

    /// Load monster summon, respawn loop, and boss random spawn.
    async fn load_monster_event_tables(&self, pool: &DbPool) {
        let npc_repo = NpcRepository::new(pool);
        match npc_repo.load_monster_summon_list().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.monster_summon_list.insert(r.s_sid, r);
                }
                tracing::info!(count, "monster_summon_list loaded");
            }
            Err(e) => tracing::warn!("monster_summon_list not found or error: {e}, skipping"),
        }
        match npc_repo.load_monster_respawn_loop().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.monster_respawn_loop.insert(r.idead, r);
                }
                tracing::info!(count, "monster_respawn_loop loaded");
            }
            Err(e) => tracing::warn!("monster_respawn_loop not found or error: {e}, skipping"),
        }
        match npc_repo.load_boss_random_spawn().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.boss_random_spawn.entry(r.stage).or_default().push(r);
                }
                tracing::info!(count, "boss_random_spawn loaded");
            }
            Err(e) => tracing::warn!("boss_random_spawn not found or error: {e}, skipping"),
        }
    }

    /// Load perk definitions.
    async fn load_perk_tables(&self, pool: &DbPool) {
        let perk_repo = PerkRepository::new(pool);
        match perk_repo.load_all_perks().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.perk_definitions.insert(r.p_index, r);
                }
                tracing::info!(count, "perk definitions loaded");
            }
            Err(e) => tracing::warn!("perks table not found or error: {e}, skipping"),
        }
    }

    /// Load jackpot settings (2 rows: EXP + Noah).
    ///
    async fn load_jackpot_settings(&self, pool: &DbPool) {
        let repo = JackPotRepository::new(pool);
        match repo.load_all().await {
            Ok(rows) => {
                let mut settings = self.jackpot_settings.write();
                for r in &rows {
                    let idx = r.i_type as usize;
                    if idx > 1 {
                        continue;
                    }
                    settings[idx].rate = r.rate as u16;
                    settings[idx].x_1000 = r.x_1000 as u16;
                    settings[idx].x_500 = r.x_500 as u16;
                    settings[idx].x_100 = r.x_100 as u16;
                    settings[idx].x_50 = r.x_50 as u16;
                    settings[idx].x_10 = r.x_10 as u16;
                    settings[idx].x_2 = r.x_2 as u16;
                }
                tracing::info!(count = rows.len(), "jackpot settings loaded");
            }
            Err(e) => tracing::warn!("jackpot_settings table not found or error: {e}, skipping"),
        }
    }

    /// Load event schedule, vroom opts, FT opts, rewards.
    async fn load_event_schedule(&self, pool: &DbPool) {
        use crate::systems::event_room::{EventScheduleEntry, ForgottenTempleOpts, VroomOpt};
        let evt_repo = EventScheduleRepository::new(pool);

        match evt_repo.load_vroom_opts().await {
            Ok(rows) => {
                let mut opts = self.event_room_manager.vroom_opts.write();
                for row in &rows {
                    let vopt = VroomOpt {
                        name: row.name.trim().to_string(),
                        sign: row.sign,
                        play: row.play,
                        attack_open: row.attackopen,
                        attack_close: row.attackclose,
                        finish: row.finish,
                    };
                    match row.zoneid {
                        84 => opts[0] = Some(vopt),
                        85 => opts[1] = Some(vopt),
                        87 => opts[2] = Some(vopt),
                        _ => {}
                    }
                }
                tracing::info!(count = rows.len(), "event_opt_vroom loaded");
            }
            Err(e) => tracing::warn!("event_opt_vroom not found: {e}, skipping"),
        }

        match evt_repo.load_ft_opts().await {
            Ok(Some(row)) => {
                let mut ft = self.event_room_manager.ft_opts.write();
                *ft = ForgottenTempleOpts {
                    playing_time: row.playing_time,
                    summon_time: row.summon_time,
                    spawn_min_time: row.spawn_min_time,
                    waiting_time: row.waiting_time,
                    min_level: row.min_level,
                    max_level: row.max_level,
                };
                tracing::info!(
                    "event_opt_ft loaded (play={}min, levels={}-{})",
                    row.playing_time,
                    row.min_level,
                    row.max_level
                );
            }
            Ok(None) => tracing::info!("event_opt_ft empty, using defaults"),
            Err(e) => tracing::warn!("event_opt_ft error: {e}, using defaults"),
        }

        match (
            evt_repo.load_main_list().await,
            evt_repo.load_day_list().await,
        ) {
            (Ok(main_rows), Ok(day_rows)) => {
                let day_map: std::collections::HashMap<i16, _> =
                    day_rows.into_iter().map(|d| (d.eventid, d)).collect();
                let mut schedules = Vec::with_capacity(main_rows.len());
                for m in &main_rows {
                    let days = if let Some(d) = day_map.get(&m.eventid) {
                        [
                            d.sunday != 0,
                            d.monday != 0,
                            d.tuesday != 0,
                            d.wednesday != 0,
                            d.thursday != 0,
                            d.friday != 0,
                            d.saturday != 0,
                        ]
                    } else {
                        [false; 7]
                    };
                    schedules.push(EventScheduleEntry {
                        event_id: m.eventid,
                        event_type: m.event_type,
                        zone_id: m.zoneid,
                        name: m.name.trim().to_string(),
                        status: m.status != 0,
                        start_times: [
                            (m.hour1, m.minute1),
                            (m.hour2, m.minute2),
                            (m.hour3, m.minute3),
                            (m.hour4, m.minute4),
                            (m.hour5, m.minute5),
                        ],
                        days,
                        min_level: m.min_level,
                        max_level: m.max_level,
                        req_loyalty: m.req_loyalty,
                        req_money: m.req_money,
                    });
                }
                let count = schedules.len();
                *self.event_room_manager.schedules.write() = schedules;
                tracing::info!(count, "event schedules loaded");
            }
            (Err(e), _) | (_, Err(e)) => {
                tracing::warn!("event schedule tables not found: {e}, skipping")
            }
        }

        match evt_repo.load_rewards().await {
            Ok(rows) => {
                let count = rows.len();
                for r in rows {
                    self.event_rewards.entry(r.local_id).or_default().push(r);
                }
                tracing::info!(
                    count,
                    groups = self.event_rewards.len(),
                    "event_rewards loaded"
                );
            }
            Err(e) => tracing::warn!("event_rewards error: {e}, skipping"),
        }

        match evt_repo.load_timer_show_list().await {
            Ok(rows) => {
                let count = rows.len();
                *self.event_timer_show_list.write() = rows;
                tracing::info!(count, "event_timer_show_list loaded");
            }
            Err(e) => tracing::warn!("event_timer_show_list error: {e}, skipping"),
        }
    }

    /// Load FT, DD, Draki, Chaos Stone tables.
    async fn load_dungeon_tables(&self, pool: &DbPool) {
        let ft_repo = ForgottenTempleRepository::new(pool);
        match ft_repo.load_all_stages().await {
            Ok(rows) => {
                let count = rows.len();
                *self.ft_stages.write() = rows;
                tracing::info!(count, "ft_stages loaded");
            }
            Err(e) => tracing::warn!("ft_stages table not found: {e}, skipping"),
        }
        match ft_repo.load_all_summons().await {
            Ok(rows) => {
                let count = rows.len();
                *self.ft_summons.write() = rows;
                tracing::info!(count, "ft_summon_list loaded");
            }
            Err(e) => tracing::warn!("ft_summon_list table not found: {e}, skipping"),
        }

        let dd_repo = DungeonDefenceRepository::new(pool);
        match dd_repo.load_all_stages().await {
            Ok(rows) => {
                let count = rows.len();
                *self.dd_stages.write() = rows;
                tracing::info!(count, "df_stage_list loaded");
            }
            Err(e) => tracing::warn!("df_stage_list table not found: {e}, skipping"),
        }
        match dd_repo.load_all_monsters().await {
            Ok(rows) => {
                let count = rows.len();
                *self.dd_monsters.write() = rows;
                tracing::info!(count, "df_monster_list loaded");
            }
            Err(e) => tracing::warn!("df_monster_list table not found: {e}, skipping"),
        }

        let dt_repo = DrakiTowerRepository::new(pool);
        match dt_repo.load_all_stages().await {
            Ok(rows) => {
                let count = rows.len();
                *self.draki_tower_stages.write() = rows;
                tracing::info!(count, "draki_tower_stages loaded");
            }
            Err(e) => tracing::warn!("draki_tower_stages table not found: {e}, skipping"),
        }
        match dt_repo.load_all_monsters().await {
            Ok(rows) => {
                let count = rows.len();
                *self.draki_monster_list.write() = rows;
                tracing::info!(count, "draki_monster_list loaded");
            }
            Err(e) => tracing::warn!("draki_monster_list table not found: {e}, skipping"),
        }

        let cs_repo = ChaosStoneRepository::new(pool);
        match cs_repo.load_all_spawns().await {
            Ok(rows) => {
                let count = rows.len();
                // Build runtime info map from rank-1 spawn entries.
                let infos = crate::handler::chaos_stone::load_chaos_stones(&rows);
                let info_count = infos.len();
                *self.chaos_stone_spawns.write() = rows;
                *self.chaos_stone_infos.write() = infos;
                tracing::info!(count, info_count, "chaos_stone_spawn loaded");
            }
            Err(e) => tracing::warn!("chaos_stone_spawn table not found: {e}, skipping"),
        }
        match cs_repo.load_all_summon_list().await {
            Ok(rows) => {
                let count = rows.len();
                *self.chaos_stone_summon_list.write() = rows;
                tracing::info!(count, "chaos_stone_summon_list loaded");
            }
            Err(e) => tracing::warn!("chaos_stone_summon_list table not found: {e}, skipping"),
        }
        match cs_repo.load_all_stages().await {
            Ok(rows) => {
                let count = rows.len();
                *self.chaos_stone_stages.write() = rows;
                tracing::info!(count, "chaos_stone_summon_stage loaded");
            }
            Err(e) => tracing::warn!("chaos_stone_summon_stage table not found: {e}, skipping"),
        }
        match cs_repo.load_all_rewards().await {
            Ok(rows) => {
                let count = rows.len();
                *self.chaos_stone_rewards.write() = rows;
                tracing::info!(count, "event_chaos_rewards loaded");
            }
            Err(e) => tracing::warn!("event_chaos_rewards table not found: {e}, skipping"),
        }
    }

    /// Load Under The Castle spawns and daily quest definitions.
    async fn load_misc_event_tables(&self, pool: &DbPool) {
        match UnderCastleRepository::fetch_all(pool).await {
            Ok(rows) => {
                let count = rows.len();
                *self.utc_spawns.write() = rows;
                tracing::info!(count, "monster_under_the_castle loaded");
            }
            Err(e) => tracing::warn!("monster_under_the_castle table not found: {e}, skipping"),
        }
        let dq_repo = DailyQuestRepository::new(pool);
        match dq_repo.load_all_definitions().await {
            Ok(rows) => {
                let count = rows.len();
                for row in rows {
                    self.daily_quests.insert(row.id, row);
                }
                tracing::info!(count, "daily_quests loaded");
            }
            Err(e) => tracing::warn!("daily_quests table not found: {e}, skipping"),
        }

        // ── Daily Rank Computation + Cache ──
        let dr_repo = ko_db::repositories::daily_rank::DailyRankRepository::new(pool);
        if let Err(e) = dr_repo.compute_ranks().await {
            tracing::warn!("compute_daily_ranks() failed: {e}, skipping");
        }
        match dr_repo.load_all().await {
            Ok(rows) => {
                let count = rows.len();
                *self.daily_rank_cache.write() = rows;
                tracing::info!(count, "daily_rank loaded");
            }
            Err(e) => tracing::warn!("daily_rank table not found: {e}, skipping"),
        }
    }

    /// Load character creation (new char set/value) tables.
    async fn load_char_creation(&self, pool: &DbPool) {
        let cc_repo = CharCreationRepository::new(pool);
        match cc_repo.load_all_char_set().await {
            Ok(rows) => {
                let count = rows.len();
                for row in rows {
                    self.new_char_set
                        .entry(row.class_type)
                        .or_default()
                        .push(row);
                }
                tracing::info!(count, "create_new_char_set loaded");
            }
            Err(e) => tracing::warn!("create_new_char_set table not found: {e}, skipping"),
        }
        match cc_repo.load_all_char_value().await {
            Ok(rows) => {
                let count = rows.len();
                for row in &rows {
                    self.new_char_value
                        .insert((row.class_type, row.job_type), row.clone());
                }
                tracing::info!(count, "create_new_char_value loaded");
            }
            Err(e) => tracing::warn!("create_new_char_value table not found: {e}, skipping"),
        }
    }

    /// Load knights cape, castellan bonus, and CSW opt tables.
    async fn load_cape_tables(&self, pool: &DbPool) {
        let cape_repo = KnightsCapeRepository::new(pool);
        match cape_repo.load_all_capes().await {
            Ok(rows) => {
                let count = rows.len();
                for row in rows {
                    self.knights_capes.insert(row.s_cape_index, row);
                }
                tracing::info!(count, "knights_cape table loaded");
            }
            Err(e) => tracing::warn!("knights_cape table not found: {e}, skipping"),
        }
        match cape_repo.load_all_castellan_bonuses().await {
            Ok(rows) => {
                let count = rows.len();
                for row in rows {
                    self.castellan_bonuses.insert(row.bonus_type, row);
                }
                tracing::info!(count, "knights_cape_castellan_bonus table loaded");
            }
            Err(e) => tracing::warn!("knights_cape_castellan_bonus table not found: {e}, skipping"),
        }
        match cape_repo.load_csw_opt().await {
            Ok(opt) => {
                if opt.is_some() {
                    tracing::info!("knights_csw_opt loaded");
                }
                *self.csw_opt.write() = opt;
            }
            Err(e) => tracing::warn!("knights_csw_opt table not found: {e}, skipping"),
        }
    }

    /// Load monster event spawn tables (stone respawn, boss stages, juraid, challenge).
    async fn load_monster_event_spawns(&self, pool: &DbPool) {
        let me_repo = MonsterEventRepository::new(pool);
        match me_repo.load_stone_respawn().await {
            Ok(rows) => {
                let count = rows.len();
                *self.monster_stone_respawn.write() = rows;
                tracing::info!(count, "monster_stone_respawn_list loaded");
            }
            Err(e) => tracing::warn!("monster_stone_respawn_list not found: {e}, skipping"),
        }
        match me_repo.load_boss_random_stages().await {
            Ok(rows) => {
                let count = rows.len();
                *self.monster_boss_random_stages.write() = rows;
                tracing::info!(count, "monster_boss_random_stages loaded");
            }
            Err(e) => tracing::warn!("monster_boss_random_stages not found: {e}, skipping"),
        }
        match me_repo.load_juraid_respawn().await {
            Ok(rows) => {
                let count = rows.len();
                *self.monster_juraid_respawn.write() = rows;
                tracing::info!(count, "monster_juraid_respawn_list loaded");
            }
            Err(e) => tracing::warn!("monster_juraid_respawn_list not found: {e}, skipping"),
        }
        match me_repo.load_challenge_config().await {
            Ok(rows) => {
                let count = rows.len();
                *self.monster_challenge.write() = rows;
                tracing::info!(count, "monster_challenge loaded");
            }
            Err(e) => tracing::warn!("monster_challenge not found: {e}, skipping"),
        }
        match me_repo.load_challenge_summon_list().await {
            Ok(rows) => {
                let count = rows.len();
                *self.monster_challenge_summon.write() = rows;
                tracing::info!(count, "monster_challenge_summon_list loaded");
            }
            Err(e) => tracing::warn!("monster_challenge_summon_list not found: {e}, skipping"),
        }
    }

    /// Load Cinderella War data tables.
    async fn load_cinderella_war(&self, pool: &DbPool) {
        let cind_repo = CinderellaRepository::new(pool);
        match cind_repo.load_all_settings().await {
            Ok(rows) => {
                let count = rows.len();
                *self.cindwar_settings.write() = rows;
                tracing::info!(count, "cindwar_setting loaded");
            }
            Err(e) => tracing::warn!("cindwar_setting table not found: {e}, skipping"),
        }
        match cind_repo.load_all_items().await {
            Ok(rows) => {
                let count = rows.len();
                *self.cindwar_items.write() = rows;
                tracing::info!(count, "cindwar_items loaded");
            }
            Err(e) => tracing::warn!("cindwar_items table not found: {e}, skipping"),
        }
        match cind_repo.load_all_rewards().await {
            Ok(rows) => {
                let count = rows.len();
                *self.cindwar_rewards.write() = rows;
                tracing::info!(count, "cindwar_reward loaded");
            }
            Err(e) => tracing::warn!("cindwar_reward table not found: {e}, skipping"),
        }
        match cind_repo.load_all_reward_items().await {
            Ok(rows) => {
                let count = rows.len();
                *self.cindwar_reward_items.write() = rows;
                tracing::info!(count, "cindwar_reward_item loaded");
            }
            Err(e) => tracing::warn!("cindwar_reward_item table not found: {e}, skipping"),
        }
        match cind_repo.load_all_stats().await {
            Ok(rows) => {
                let count = rows.len();
                *self.cindwar_stats.write() = rows;
                tracing::info!(count, "cindwar_stat loaded");
            }
            Err(e) => tracing::warn!("cindwar_stat table not found: {e}, skipping"),
        }
    }

    /// Load zone kill and online reward tables.
    async fn load_zone_rewards(&self, pool: &DbPool) {
        let zone_reward_repo = ZoneRewardsRepository::new(pool);
        match zone_reward_repo.load_kill_rewards().await {
            Ok(rows) => {
                let count = rows.len();
                *self.zone_kill_rewards.write() = rows;
                tracing::info!(count, "zone_kill_reward table loaded");
            }
            Err(e) => tracing::warn!("zone_kill_reward table not found: {e}, skipping"),
        }
        match zone_reward_repo.load_online_rewards().await {
            Ok(rows) => {
                let count = rows.len();
                *self.zone_online_rewards.write() = rows;
                tracing::info!(count, "zone_online_reward table loaded");
            }
            Err(e) => tracing::warn!("zone_online_reward table not found: {e}, skipping"),
        }
    }

    /// Load the anti-AFK NPC ID list from DB.
    ///
    async fn load_anti_afk_list(&self, pool: &DbPool) {
        let repo = ko_db::repositories::anti_afk::AntiAfkRepository::new(pool);
        match repo.load_all().await {
            Ok(rows) => {
                let ids: Vec<u16> = rows.iter().map(|r| r.npc_id as u16).collect();
                let count = ids.len();
                self.set_anti_afk_npc_ids(ids);
                tracing::info!(count, "anti_afk_list loaded");
            }
            Err(e) => tracing::warn!("anti_afk_list table not found: {e}, skipping"),
        }
    }

    /// Load RANKBUG configuration from DB.
    ///
    async fn load_rankbug(&self, pool: &DbPool) {
        let repo = ko_db::repositories::rankbug::RankBugRepository::new(pool);
        match repo.load().await {
            Ok(cfg) => {
                tracing::info!(
                    cz_rank = cfg.cz_rank,
                    border_join = cfg.border_join,
                    "rankbug config loaded"
                );
                *self.rank_bug.write() = cfg;
            }
            Err(e) => tracing::warn!("rankbug table not found: {e}, using defaults"),
        }
    }

    /// Load banish-of-winner spawn definitions from DB.
    ///
    async fn load_banish_of_winner(&self, pool: &DbPool) {
        let repo = ko_db::repositories::banish::BanishRepository::new(pool);
        match repo.load_all().await {
            Ok(rows) => {
                tracing::info!(count = rows.len(), "banish_of_winner table loaded");
                self.set_banish_of_winner(rows);
            }
            Err(e) => tracing::warn!("banish_of_winner load failed: {e}"),
        }
    }
}
