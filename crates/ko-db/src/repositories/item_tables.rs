//! Item tables repository — loads item-related reference data from PostgreSQL.
//! - `GameServer/LoadServerData.cpp` — various `Load*Table()` functions
//! - `shared/database/ItemTableSet.h` — DB loaders
//! All tables are bulk-loaded at startup and cached in-memory by the game server.

use crate::models::item_tables::{
    ItemExchangeExpRow, ItemExchangeRow, ItemGiveExchangeRow, ItemGroupRow, ItemOpRow,
    ItemRandomRow, ItemRightClickExchangeRow, ItemRightExchangeRow, ItemSellTableRow, ItemSmashRow,
    ItemSpecialSewingRow, ItemUpgradeRow, ItemUpgradeSettingsRow, MakeDefensiveRow,
    MakeItemGradeCodeRow, MakeItemGroupRandomRow, MakeItemGroupRow, MakeItemLareCodeRow,
    MakeItemRow, MakeWeaponRow, MonsterItemRow, NewUpgradeRow, NpcItemRow, RentalItemRow,
    SealedItemRow, SetItemRow, SpecialStoneRow,
};
use crate::DbPool;

/// Repository for item-related reference table access.
pub struct ItemTablesRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ItemTablesRepository<'a> {
    /// Create a new repository backed by the given connection pool.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load all item special effect rows (bulk load at startup).
    ///
    /// Returns all item_op entries (2,703 rows).
    pub async fn load_all_item_ops(&self) -> Result<Vec<ItemOpRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemOpRow>(
            "SELECT item_id, trigger_type, skill_id, trigger_rate \
             FROM item_op ORDER BY item_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all set item bonus rows (bulk load at startup).
    ///
    /// Returns all set_item entries (1,165 rows).
    pub async fn load_all_set_items(&self) -> Result<Vec<SetItemRow>, sqlx::Error> {
        sqlx::query_as::<_, SetItemRow>(
            "SELECT set_index, set_name, ac_bonus, hp_bonus, mp_bonus, \
             strength_bonus, stamina_bonus, dexterity_bonus, intel_bonus, charisma_bonus, \
             flame_resistance, glacier_resistance, lightning_resistance, poison_resistance, \
             magic_resistance, curse_resistance, \
             xp_bonus_percent, coin_bonus_percent, ap_bonus_percent, \
             ap_bonus_class_type, ap_bonus_class_percent, \
             ac_bonus_class_type, ac_bonus_class_percent, \
             max_weight_bonus, np_bonus, \
             unk1, unk2, unk3, unk4, unk5, unk6, unk7, \
             unk8, unk9, unk10, unk11, unk12, unk13, unk14, unk15 \
             FROM set_item ORDER BY set_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all monster drop table rows (bulk load at startup).
    ///
    /// Returns all monster_item entries (2,154 rows).
    pub async fn load_all_monster_items(&self) -> Result<Vec<MonsterItemRow>, sqlx::Error> {
        sqlx::query_as::<_, MonsterItemRow>(
            "SELECT s_index, \
             item01, percent01, item02, percent02, item03, percent03, \
             item04, percent04, item05, percent05, item06, percent06, \
             item07, percent07, item08, percent08, item09, percent09, \
             item10, percent10, item11, percent11, item12, percent12 \
             FROM monster_item ORDER BY s_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item exchange/crafting recipes (bulk load at startup).
    ///
    /// Returns all item_exchange entries (5,023 rows).
    pub async fn load_all_item_exchanges(&self) -> Result<Vec<ItemExchangeRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemExchangeRow>(
            "SELECT n_index, random_flag, \
             origin_item_num1, origin_item_count1, \
             origin_item_num2, origin_item_count2, \
             origin_item_num3, origin_item_count3, \
             origin_item_num4, origin_item_count4, \
             origin_item_num5, origin_item_count5, \
             exchange_item_num1, exchange_item_count1, \
             exchange_item_num2, exchange_item_count2, \
             exchange_item_num3, exchange_item_count3, \
             exchange_item_num4, exchange_item_count4, \
             exchange_item_num5, exchange_item_count5, \
             exchange_item_time1, exchange_item_time2, \
             exchange_item_time3, exchange_item_time4, exchange_item_time5 \
             FROM item_exchange ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item upgrade settings (bulk load at startup).
    ///
    /// Returns all item_upgrade_settings entries (257 rows).
    pub async fn load_all_upgrade_settings(
        &self,
    ) -> Result<Vec<ItemUpgradeSettingsRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemUpgradeSettingsRow>(
            "SELECT req_item_id1, req_item_name1, req_item_id2, req_item_name2, \
             upgrade_note, item_type, item_rate, item_grade, \
             item_req_coins, success_rate \
             FROM item_upgrade_settings ORDER BY item_type, item_rate, item_grade",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all new upgrade recipes (bulk load at startup).
    ///
    /// Returns all new_upgrade entries (18,148 rows).
    pub async fn load_all_new_upgrades(&self) -> Result<Vec<NewUpgradeRow>, sqlx::Error> {
        sqlx::query_as::<_, NewUpgradeRow>(
            "SELECT n_index, str_note, origin_number, n_str_note, \
             new_number, req_item, grade \
             FROM new_upgrade ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all NPC drop table rows (bulk load at startup).
    ///
    pub async fn load_all_npc_items(&self) -> Result<Vec<NpcItemRow>, sqlx::Error> {
        sqlx::query_as::<_, NpcItemRow>(
            "SELECT s_index, \
             item01, percent01, item02, percent02, item03, percent03, \
             item04, percent04, item05, percent05, item06, percent06, \
             item07, percent07, item08, percent08, item09, percent09, \
             item10, percent10, item11, percent11, item12, percent12 \
             FROM npc_item ORDER BY s_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item upgrade recipes (bulk load at startup).
    ///
    pub async fn load_all_item_upgrades(&self) -> Result<Vec<ItemUpgradeRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemUpgradeRow>(
            "SELECT n_index, npc_num, origin_type, origin_item, \
             req_item1, req_item2, req_item3, req_item4, \
             req_item5, req_item6, req_item7, req_item8, \
             req_noah, rate_type, gen_rate, trina_rate, karivdis_rate, give_item \
             FROM item_upgrade ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all weapon crafting templates (bulk load at startup).
    ///
    pub async fn load_all_make_weapons(&self) -> Result<Vec<MakeWeaponRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeWeaponRow>(
            "SELECT by_level, class_1, class_2, class_3, class_4, class_5, class_6, \
             class_7, class_8, class_9, class_10, class_11, class_12 \
             FROM make_weapon ORDER BY by_level",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all defensive crafting templates (bulk load at startup).
    ///
    pub async fn load_all_make_defensives(&self) -> Result<Vec<MakeDefensiveRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeDefensiveRow>(
            "SELECT by_level, class_1, class_2, class_3, class_4, class_5, class_6, class_7 \
             FROM make_defensive ORDER BY by_level",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all crafting grade codes (bulk load at startup).
    ///
    pub async fn load_all_make_grade_codes(
        &self,
    ) -> Result<Vec<MakeItemGradeCodeRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeItemGradeCodeRow>(
            "SELECT item_index, grade_1, grade_2, grade_3, grade_4, grade_5, \
             grade_6, grade_7, grade_8, grade_9 \
             FROM make_item_gradecode ORDER BY item_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all crafting rarity codes (bulk load at startup).
    ///
    pub async fn load_all_make_lare_codes(&self) -> Result<Vec<MakeItemLareCodeRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeItemLareCodeRow>(
            "SELECT level_grade, lare_item, magic_item, general_item \
             FROM make_item_larecode ORDER BY level_grade",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all crafting item groups (bulk load at startup).
    ///
    pub async fn load_all_make_item_groups(&self) -> Result<Vec<MakeItemGroupRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeItemGroupRow>(
            "SELECT group_num, items FROM make_item_group ORDER BY group_num",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all random crafting item group mappings (bulk load at startup).
    ///
    pub async fn load_all_make_item_group_randoms(
        &self,
    ) -> Result<Vec<MakeItemGroupRandomRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeItemGroupRandomRow>(
            "SELECT n_index, item_id, group_no \
             FROM make_item_group_random ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all rental items (bulk load at startup).
    ///
    pub async fn load_all_rental_items(&self) -> Result<Vec<RentalItemRow>, sqlx::Error> {
        sqlx::query_as::<_, RentalItemRow>(
            "SELECT rental_index, item_index, durability, serial_number, \
             reg_type, item_type, item_class, rental_time, rental_money, \
             lender_char_id, borrower_char_id \
             FROM rental_item ORDER BY rental_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all NPC sell table rows (bulk load at startup).
    ///
    /// Returns all item_sell_table entries (457 rows, 42 selling groups).
    /// Used to validate buy requests: the requested item must exist in
    /// the sell table for the NPC's selling group.
    ///
    pub async fn load_all_sell_table(&self) -> Result<Vec<ItemSellTableRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemSellTableRow>(
            "SELECT n_index, i_selling_group, \
             item1, item2, item3, item4, item5, item6, \
             item7, item8, item9, item10, item11, item12, \
             item13, item14, item15, item16, item17, item18, \
             item19, item20, item21, item22, item23, item24 \
             FROM item_sell_table ORDER BY i_selling_group, n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item special sewing (crafting) recipes (bulk load at startup).
    ///
    /// Returns all item_special_sewing entries (2,468 rows).
    pub async fn load_all_special_sewing(&self) -> Result<Vec<ItemSpecialSewingRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemSpecialSewingRow>(
            "SELECT n_index, description, \
             req_item_id_1, req_item_count_1, req_item_id_2, req_item_count_2, \
             req_item_id_3, req_item_count_3, req_item_id_4, req_item_count_4, \
             req_item_id_5, req_item_count_5, req_item_id_6, req_item_count_6, \
             req_item_id_7, req_item_count_7, req_item_id_8, req_item_count_8, \
             req_item_id_9, req_item_count_9, req_item_id_10, req_item_count_10, \
             give_item_id, give_item_count, success_rate, npc_id, is_notice, \
             is_shadow_success \
             FROM item_special_sewing ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item smash entries (bulk load at startup).
    ///
    /// Returns all item_smash entries (205 rows).
    pub async fn load_all_item_smash(&self) -> Result<Vec<ItemSmashRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemSmashRow>(
            "SELECT n_index, item_id, count, rate \
             FROM item_smash ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all special stone definitions (bulk load at startup).
    ///
    /// Returns all k_special_stone entries (18 rows).
    pub async fn load_all_special_stones(&self) -> Result<Vec<SpecialStoneRow>, sqlx::Error> {
        sqlx::query_as::<_, SpecialStoneRow>(
            "SELECT n_index, zone_id, main_npc, monster_name, summon_npc, summon_count, status \
             FROM k_special_stone ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all random item entries (bulk load at startup).
    ///
    /// Returns all item_random entries (58 rows).
    pub async fn load_all_item_random(&self) -> Result<Vec<ItemRandomRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemRandomRow>(
            "SELECT n_index, str_item_name, item_id, item_count, rental_time, session_id, status \
             FROM item_random ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item group entries (bulk load at startup).
    ///
    /// Returns all item_group entries (4 rows).
    pub async fn load_all_item_groups(&self) -> Result<Vec<ItemGroupRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemGroupRow>(
            "SELECT group_id, name, items FROM item_group ORDER BY group_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item exchange experience entries (bulk load at startup).
    ///
    /// Returns all item_exchange_exp entries (204 rows).
    pub async fn load_all_item_exchange_exp(&self) -> Result<Vec<ItemExchangeExpRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemExchangeExpRow>(
            "SELECT n_index, random_flag, \
             exchange_item_num1, exchange_item_count1, \
             exchange_item_num2, exchange_item_count2, \
             exchange_item_num3, exchange_item_count3, \
             exchange_item_num4, exchange_item_count4, \
             exchange_item_num5, exchange_item_count5, \
             exchange_item_time1, exchange_item_time2, exchange_item_time3, \
             exchange_item_time4, exchange_item_time5 \
             FROM item_exchange_exp ORDER BY n_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all item give exchange entries (bulk load at startup).
    ///
    /// Returns all item_give_exchange entries (661 rows).
    pub async fn load_all_item_give_exchange(
        &self,
    ) -> Result<Vec<ItemGiveExchangeRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemGiveExchangeRow>(
            "SELECT exchange_index, rob_item_ids, rob_item_counts, \
             give_item_ids, give_item_counts, give_item_times \
             FROM item_give_exchange ORDER BY exchange_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all right-click exchange mappings (bulk load at startup).
    ///
    /// Returns all item_right_click_exchange entries (96 rows).
    pub async fn load_all_right_click_exchange(
        &self,
    ) -> Result<Vec<ItemRightClickExchangeRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemRightClickExchangeRow>(
            "SELECT item_id, opcode FROM item_right_click_exchange ORDER BY item_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all right exchange entries (bulk load at startup).
    ///
    /// Returns all item_right_exchange entries (66 rows).
    pub async fn load_all_right_exchange(&self) -> Result<Vec<ItemRightExchangeRow>, sqlx::Error> {
        sqlx::query_as::<_, ItemRightExchangeRow>(
            "SELECT item_id, str_name, exchange_type, description, exchange_count, \
             exchange_items, exchange_counts, expiration_times \
             FROM item_right_exchange ORDER BY item_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load sealed items for a specific account (loaded per-login).
    ///
    pub async fn load_sealed_items_by_account(
        &self,
        account_id: &str,
    ) -> Result<Vec<SealedItemRow>, sqlx::Error> {
        sqlx::query_as::<_, SealedItemRow>(
            "SELECT id, account_id, character_id, item_serial, item_id, \
             seal_type, original_seal_type, prelock_state \
             FROM sealed_items WHERE account_id = $1 ORDER BY id",
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await
    }

    /// Load all make_item rows (bulk load at startup, 10,000 rows).
    ///
    /// Used by the loot system (ItemProdution) to map sIndex -> (item_code, item_level).
    pub async fn load_all_make_items(&self) -> Result<Vec<MakeItemRow>, sqlx::Error> {
        sqlx::query_as::<_, MakeItemRow>(
            "SELECT s_index, item_code, item_level \
             FROM make_item ORDER BY s_index",
        )
        .fetch_all(self.pool)
        .await
    }
}
