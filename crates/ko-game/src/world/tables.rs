//! Static data table accessors (coefficients, magic, items, quests, events, etc.).

use super::*;

impl WorldState {
    /// Get the coefficient row for a given class.
    ///
    pub fn get_coefficient(&self, class: u16) -> Option<CoefficientRow> {
        self.coefficients.get(&class).map(|c| c.clone())
    }
    /// Get the required XP for a given level and rebirth level.
    ///
    ///
    /// Returns 0 if the level is not found in the table.
    pub fn get_exp_by_level(&self, level: u8, rebirth_level: u8) -> i64 {
        // C++ logic: if nLevel < MAX_LEVEL and RebithLevel > 0, use RebithLevel=0
        let key = if (level as u16) < MAX_LEVEL && rebirth_level > 0 {
            (level, 0u8)
        } else {
            (level, rebirth_level)
        };
        self.level_up_table.get(&key).map(|v| *v).unwrap_or(0)
    }
    /// Insert a level-up XP entry (for testing).
    #[cfg(test)]
    pub(crate) fn insert_level_up(&self, key: (u8, u8), exp: i64) {
        self.level_up_table.insert(key, exp);
    }
    /// Look up a master magic row by magic_num.
    ///
    pub fn get_magic(&self, magic_num: i32) -> Option<MagicRow> {
        self.magic_table.get(&magic_num).map(|r| r.clone())
    }
    /// Look up magic type 1 (melee) parameters by i_num.
    pub fn get_magic_type1(&self, i_num: i32) -> Option<MagicType1Row> {
        self.magic_type1.get(&i_num).map(|r| r.clone())
    }
    /// Look up magic type 2 (ranged) parameters by i_num.
    pub fn get_magic_type2(&self, i_num: i32) -> Option<MagicType2Row> {
        self.magic_type2.get(&i_num).map(|r| r.clone())
    }
    /// Look up magic type 3 (DOT/direct) parameters by i_num.
    pub fn get_magic_type3(&self, i_num: i32) -> Option<MagicType3Row> {
        self.magic_type3.get(&i_num).map(|r| r.clone())
    }
    /// Look up magic type 4 (buff/debuff) parameters by i_num.
    pub fn get_magic_type4(&self, i_num: i32) -> Option<MagicType4Row> {
        self.magic_type4.get(&i_num).map(|r| r.clone())
    }
    /// Insert a magic row into the master magic table. Used for tests.
    #[cfg(test)]
    pub(crate) fn insert_magic(&self, row: MagicRow) {
        self.magic_table.insert(row.magic_num, row);
    }
    /// Insert a magic type 4 (buff/debuff) row. Used for tests.
    #[cfg(test)]
    pub(crate) fn insert_magic_type4(&self, row: MagicType4Row) {
        self.magic_type4.insert(row.i_num, row);
    }
    /// Look up magic type 5 (resurrection) parameters by i_num.
    pub fn get_magic_type5(&self, i_num: i32) -> Option<MagicType5Row> {
        self.magic_type5.get(&i_num).map(|r| r.clone())
    }
    /// Look up magic type 6 (transform) parameters by i_num.
    pub fn get_magic_type6(&self, i_num: i32) -> Option<MagicType6Row> {
        self.magic_type6.get(&i_num).map(|r| r.clone())
    }
    /// Look up magic type 7 (summon/CC) parameters by n_index.
    pub fn get_magic_type7(&self, n_index: i32) -> Option<MagicType7Row> {
        self.magic_type7.get(&n_index).map(|r| r.clone())
    }
    /// Look up magic type 8 (teleport) parameters by i_num.
    pub fn get_magic_type8(&self, i_num: i32) -> Option<MagicType8Row> {
        self.magic_type8.get(&i_num).map(|r| r.clone())
    }
    /// Look up magic type 9 (advanced CC) parameters by i_num.
    pub fn get_magic_type9(&self, i_num: i32) -> Option<MagicType9Row> {
        self.magic_type9.get(&i_num).map(|r| r.clone())
    }
    /// Look up an item definition by item num.
    ///
    pub fn get_item(&self, item_id: u32) -> Option<Item> {
        self.items.get(&item_id).map(|r| r.clone())
    }
    /// Insert an item template (for testing).
    #[cfg(test)]
    pub(crate) fn insert_item(&self, item_id: u32, item: Item) {
        self.items.insert(item_id, item);
    }
    /// Get wheel of fun settings (cached in memory).
    ///
    pub fn get_wheel_of_fun_settings(
        &self,
    ) -> Vec<ko_db::models::wheel_of_fun::WheelOfFunSettings> {
        self.wheel_of_fun_settings.read().clone()
    }

    /// Set wheel of fun settings (loaded at startup).
    pub fn set_wheel_of_fun_settings(
        &self,
        settings: Vec<ko_db::models::wheel_of_fun::WheelOfFunSettings>,
    ) {
        *self.wheel_of_fun_settings.write() = settings;
    }

    /// Get the anti-AFK NPC ID list (sent to client on game entry).
    ///
    pub fn get_anti_afk_npc_ids(&self) -> Vec<u16> {
        self.anti_afk_npc_ids.read().clone()
    }

    /// Set anti-AFK NPC ID list (loaded at startup).
    pub fn set_anti_afk_npc_ids(&self, ids: Vec<u16>) {
        *self.anti_afk_npc_ids.write() = ids;
    }

    /// Look up a knights cape definition by cape index.
    ///
    pub fn get_knights_cape(&self, cape_index: i16) -> Option<KnightsCapeRow> {
        self.knights_capes.get(&cape_index).map(|r| r.clone())
    }
    /// Look up a castellan cape bonus by bonus type.
    ///
    pub fn get_castellan_bonus(&self, bonus_type: i16) -> Option<KnightsCapeCastellanBonusRow> {
        self.castellan_bonuses.get(&bonus_type).map(|r| r.clone())
    }
    /// Get the CSW configuration options.
    ///
    pub fn get_csw_opt(&self) -> Option<KnightsCswOptRow> {
        self.csw_opt.read().clone()
    }
    /// Validate that an item is sold by a given selling group.
    ///
    /// Primary check: `item_sell_table` row where `ItemIDs[slot_index] == item_id`.
    /// Fallback: item table `selling_group` field (×1000 = NPC selling_group).
    /// This handles cases where the client .tbl has items our sell table doesn't.
    ///
    pub fn validate_sell_table_item(
        &self,
        selling_group: i32,
        slot_index: usize,
        item_id: i32,
    ) -> bool {
        // Primary: exact sell table match
        if let Some(rows) = self.item_sell_table.get(&selling_group) {
            for row in rows.iter() {
                if row.item_at(slot_index) == item_id {
                    return true;
                }
            }
        }
        // Fallback: item's own selling_group (×1000) matches NPC selling_group
        if let Some(item) = self.get_item(item_id as u32) {
            if let Some(sg) = item.selling_group {
                if sg as i32 * 1000 == selling_group {
                    return true;
                }
            }
        }
        false
    }
    /// Look up a premium item type definition by premium_type.
    ///
    pub fn get_premium_item(&self, premium_type: u8) -> Option<PremiumItemRow> {
        self.premium_item_types
            .get(&premium_type)
            .map(|r| r.clone())
    }
    /// Look up premium gift items for a given premium type.
    ///
    pub fn get_premium_gift_items(&self, premium_type: u8) -> Vec<super::PremiumGiftItem> {
        self.premium_gift_items
            .get(&premium_type)
            .map(|v| v.clone())
            .unwrap_or_default()
    }
    /// Get a specific premium property for a session's active premium.
    ///
    /// Returns the requested property value, or 0 if no premium is active
    /// or the premium type is not found.
    ///
    pub fn get_premium_property(&self, sid: SessionId, property: PremiumProperty) -> i32 {
        let premium_type = self.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
        if premium_type == 0 {
            return 0;
        }
        self.premium_property_for_type(premium_type, property)
    }
    /// Get a specific premium property for a session's clan premium.
    ///
    pub fn get_clan_premium_property(&self, sid: SessionId, property: PremiumProperty) -> i32 {
        let clan_premium = self
            .with_session(sid, |h| h.clan_premium_in_use)
            .unwrap_or(0);
        if clan_premium == 0 {
            return 0;
        }
        self.premium_property_for_type(clan_premium, property)
    }
    /// Get the ExpRestorePercent for a session's active premium (float).
    ///
    pub fn get_premium_exp_restore(&self, sid: SessionId) -> f64 {
        let premium_type = self.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
        if premium_type == 0 {
            return 0.0;
        }
        self.premium_item_types
            .get(&premium_type)
            .map(|r| r.exp_restore_pct)
            .unwrap_or(0.0)
    }
    /// Get the XP gain bonus percent for a session's active premium,
    /// based on the player's current level.
    ///
    /// iterates `m_PremiumItemExpArray` for matching type + level range.
    pub fn get_premium_exp_percent(&self, sid: SessionId, level: u8) -> u16 {
        let premium_type = self.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
        if premium_type == 0 {
            return 0;
        }
        self.premium_exp_for_type(premium_type, level)
    }
    /// Get the XP gain bonus percent for clan premium by level.
    ///
    pub fn get_clan_premium_exp_percent(&self, sid: SessionId, level: u8) -> u16 {
        let clan_premium = self
            .with_session(sid, |h| h.clan_premium_in_use)
            .unwrap_or(0);
        if clan_premium == 0 {
            return 0;
        }
        self.premium_exp_for_type(clan_premium, level)
    }
    /// Internal: look up a premium property for a given type.
    fn premium_property_for_type(&self, premium_type: u8, property: PremiumProperty) -> i32 {
        let row = match self.premium_item_types.get(&premium_type) {
            Some(r) => r,
            None => return 0,
        };
        match property {
            PremiumProperty::NoahPercent => row.noah_pct as i32,
            PremiumProperty::DropPercent => row.drop_pct as i32,
            PremiumProperty::BonusLoyalty => row.bonus_loyalty,
            PremiumProperty::RepairDiscountPercent => row.repair_disc_pct as i32,
            PremiumProperty::ItemSellPercent => row.item_sell_pct as i32,
        }
    }
    /// Internal: look up premium XP bonus for a type + level.
    fn premium_exp_for_type(&self, premium_type: u8, level: u8) -> u16 {
        let exp_list = self.premium_item_exp.read();
        for entry in exp_list.iter() {
            if entry.premium_type == premium_type as i16
                && level >= entry.min_level as u8
                && level <= entry.max_level as u8
            {
                return entry.s_percent as u16;
            }
        }
        0
    }
    /// Look up an achievement main definition by s_index.
    ///
    pub fn achieve_main(&self, s_index: i32) -> Option<AchieveMainRow> {
        self.achieve_main.get(&s_index).map(|r| r.clone())
    }
    /// Snapshot all achievement master definitions.
    pub fn all_achieve_main(&self) -> Vec<AchieveMainRow> {
        self.achieve_main
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }
    /// Look up an achievement title by title index.
    ///
    pub fn achieve_title(&self, s_index: i32) -> Option<AchieveTitleRow> {
        self.achieve_title.get(&s_index).map(|r| r.clone())
    }
    /// Look up a war-type achievement by s_index.
    ///
    pub fn achieve_war(&self, s_index: i32) -> Option<AchieveWarRow> {
        self.achieve_war.get(&s_index).map(|r| r.clone())
    }
    pub fn all_achieve_war(&self) -> Vec<AchieveWarRow> {
        self.achieve_war.iter().map(|r| r.value().clone()).collect()
    }
    /// Look up a normal-type achievement by s_index.
    ///
    pub fn achieve_normal(&self, s_index: i32) -> Option<AchieveNormalRow> {
        self.achieve_normal.get(&s_index).map(|r| r.clone())
    }
    pub fn all_achieve_normal(&self) -> Vec<AchieveNormalRow> {
        self.achieve_normal
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }
    /// Look up a monster-kill achievement by s_index.
    ///
    pub fn achieve_monster(&self, s_index: i32) -> Option<AchieveMonsterRow> {
        self.achieve_monster.get(&s_index).map(|r| r.clone())
    }
    pub fn all_achieve_monster(&self) -> Vec<AchieveMonsterRow> {
        self.achieve_monster
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }
    /// Look up a composite (requirement-based) achievement by s_index.
    ///
    pub fn achieve_com(&self, s_index: i32) -> Option<AchieveComRow> {
        self.achieve_com.get(&s_index).map(|r| r.clone())
    }
    pub fn all_achieve_com(&self) -> Vec<AchieveComRow> {
        self.achieve_com.iter().map(|r| r.value().clone()).collect()
    }
    /// Get filtered mining/fishing item list based on table type and tool type.
    ///
    pub fn get_mining_fishing_items(
        &self,
        table_type: i32,
        use_item_type: u8,
        war_status: i32,
    ) -> Vec<MiningFishingItemRow> {
        let mut result = Vec::new();
        for entry in self.mining_fishing_items.iter() {
            let row = entry.value();
            if row.n_table_type != table_type {
                continue;
            }
            if row.use_item_type != use_item_type as i16 {
                continue;
            }
            if row.n_war_status != war_status {
                continue;
            }
            result.push(row.clone());
        }
        result
    }
    /// Get mining exchange entries filtered by ore type and NPC ID.
    ///
    pub fn get_mining_exchanges(&self, ore_type: i16, npc_id: i16) -> Vec<MiningExchangeRow> {
        self.mining_exchanges
            .iter()
            .filter(|e| e.ore_type == ore_type && e.s_npc_id == npc_id)
            .map(|e| e.value().clone())
            .collect()
    }
    /// Look up upgrade recipes for a given origin item number.
    ///
    pub fn get_upgrade_recipes(&self, origin_number: i32) -> Option<Vec<NewUpgradeRow>> {
        self.upgrade_recipes.get(&origin_number).map(|r| r.clone())
    }
    /// Find the recipe that produced an upgraded item with one of the given materials.
    ///
    /// Used by accessory disassemble: +N accessory -> three copies of +(N-1).
    pub fn find_upgrade_recipe_by_new_number_and_req_items(
        &self,
        new_number: i32,
        req_items: &[i32],
    ) -> Option<NewUpgradeRow> {
        for entry in self.upgrade_recipes.iter() {
            for recipe in entry.value() {
                if recipe.new_number == new_number && req_items.contains(&recipe.req_item) {
                    return Some(recipe.clone());
                }
            }
        }
        None
    }
    /// Iterate all upgrade settings to find a matching entry.
    ///
    pub fn find_upgrade_setting(
        &self,
        item_type: i16,
        item_grade: i16,
        req_item1: i32,
        req_item2: i32,
    ) -> Option<ItemUpgradeSettingsRow> {
        for entry in self.upgrade_settings.iter() {
            let s = entry.value();
            if s.item_type != item_type {
                continue;
            }
            if s.item_grade != item_grade && s.item_grade != 99 {
                continue;
            }
            // Check if required items match (either order)
            let matches_req1 = s.req_item_id1 == req_item1 || s.req_item_id2 == req_item1;
            let matches_req2 = s.req_item_id1 == req_item2 || s.req_item_id2 == req_item2;
            if matches_req1 && matches_req2 {
                return Some(s.clone());
            }
        }
        None
    }
    /// Get the item upgrade probability configuration.
    ///
    pub fn get_itemup_probability(&self) -> Option<ItemUpProbabilityRow> {
        self.itemup_probability.read().clone()
    }
    // ── Item Reference Table Accessors ─────────────────────────────────

    /// Look up item special effects (procs) for a given item ID.
    ///
    pub fn get_item_ops(&self, item_id: i32) -> Option<Vec<ItemOpRow>> {
        self.item_ops.get(&item_id).map(|r| r.clone())
    }
    /// Look up set item bonuses by set index.
    ///
    pub fn get_set_item(&self, set_index: i32) -> Option<SetItemRow> {
        self.set_items.get(&set_index).map(|r| r.clone())
    }
    /// Look up monster drop table by index.
    ///
    pub fn get_monster_item(&self, s_index: i16) -> Option<MonsterItemRow> {
        self.monster_items.get(&s_index).map(|r| r.clone())
    }
    /// Look up NPC drop table by index.
    ///
    pub fn get_npc_item(&self, s_index: i16) -> Option<NpcItemRow> {
        self.npc_items.get(&s_index).map(|r| r.clone())
    }
    /// Look up an item exchange/crafting recipe by index.
    ///
    pub fn get_item_exchange(&self, n_index: i32) -> Option<ItemExchangeRow> {
        self.item_exchanges.get(&n_index).map(|r| r.clone())
    }
    /// Get all item exchange entries matching a Bifrost piece origin item.
    ///
    /// Returns entries where `random_flag` is 1, 2, or 3 **and**
    /// `origin_item_num1` equals the given piece item ID.
    ///
    /// `m_ItemExchangeArray`, filters `bRandomFlag IN (1,2,3)` and
    /// `nOriginItemNum[0] == nExchangeItemID`.
    pub fn get_bifrost_exchanges(&self, piece_item_id: u32) -> Vec<ItemExchangeRow> {
        let piece_id = piece_item_id as i32;
        self.item_exchanges
            .iter()
            .filter(|entry| {
                let r = entry.value();
                (1..=3).contains(&r.random_flag) && r.origin_item_num1 == piece_id
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
    /// Get all item exchange entries for generator exchange (random_flag 1,2,3,101).
    ///
    pub fn get_generator_exchanges(&self, origin_item_id: u32) -> Vec<ItemExchangeRow> {
        let origin_id = origin_item_id as i32;
        self.item_exchanges
            .iter()
            .filter(|entry| {
                let r = entry.value();
                matches!(r.random_flag, 1 | 2 | 3 | 101) && r.origin_item_num1 == origin_id
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
    /// Insert an item exchange row (for testing).
    #[cfg(test)]
    pub(crate) fn insert_item_exchange(&self, n_index: i32, row: ItemExchangeRow) {
        self.item_exchanges.insert(n_index, row);
    }
    /// Look up an item upgrade recipe by index.
    ///
    pub fn get_item_upgrade(&self, n_index: i32) -> Option<ItemUpgradeRow> {
        self.item_upgrades.get(&n_index).map(|r| r.clone())
    }
    /// Look up a weapon crafting template by level.
    ///
    pub fn get_make_weapon(&self, by_level: i16) -> Option<MakeWeaponRow> {
        self.make_weapons.get(&by_level).map(|r| r.clone())
    }
    /// Look up a defensive crafting template by level.
    ///
    pub fn get_make_defensive(&self, by_level: i16) -> Option<MakeDefensiveRow> {
        self.make_defensives.get(&by_level).map(|r| r.clone())
    }
    /// Look up a crafting grade code by item index.
    ///
    pub fn get_make_grade_code(&self, item_index: i16) -> Option<MakeItemGradeCodeRow> {
        self.make_grade_codes.get(&item_index).map(|r| r.clone())
    }
    /// Look up a crafting rarity code by level grade.
    ///
    pub fn get_make_lare_code(&self, level_grade: i16) -> Option<MakeItemLareCodeRow> {
        self.make_lare_codes.get(&level_grade).map(|r| r.clone())
    }
    /// Look up a crafting item group by group number.
    ///
    pub fn get_make_item_group(&self, group_num: i32) -> Option<MakeItemGroupRow> {
        self.make_item_groups.get(&group_num).map(|r| r.clone())
    }
    /// Look up a random crafting group mapping by index.
    ///
    pub fn get_make_item_group_random(&self, n_index: i32) -> Option<MakeItemGroupRandomRow> {
        self.make_item_group_randoms
            .get(&n_index)
            .map(|r| r.clone())
    }
    /// Check if any MakeItemGroupRandom entry exists for the given group number.
    ///
    pub fn has_make_item_group_random(&self, group_num: i32) -> bool {
        self.make_item_group_randoms
            .iter()
            .any(|entry| entry.value().group_no == group_num)
    }
    /// Look up a make_item entry by s_index.
    ///
    pub fn get_make_item(&self, s_index: i16) -> Option<MakeItemRow> {
        self.make_items.get(&s_index).map(|r| r.clone())
    }
    /// Get the sheriff reports map.
    ///
    pub fn sheriff_reports(&self) -> Arc<SheriffReportMap> {
        self.sheriff_reports.clone()
    }
    // ── Cinderella War Accessors ──────────────────────────────────────

    /// Get a read lock on the Cinderella War tier settings.
    ///
    pub fn cindwar_settings(&self) -> parking_lot::RwLockReadGuard<'_, Vec<CindwarSettingRow>> {
        self.cindwar_settings.read()
    }
    /// Get a read lock on the Cinderella War equipment items.
    ///
    pub fn cindwar_items(&self) -> parking_lot::RwLockReadGuard<'_, Vec<CindwarItemRow>> {
        self.cindwar_items.read()
    }
    /// Get a read lock on the Cinderella War rank rewards.
    ///
    pub fn cindwar_rewards(&self) -> parking_lot::RwLockReadGuard<'_, Vec<CindwarRewardRow>> {
        self.cindwar_rewards.read()
    }
    /// Get a read lock on the Cinderella War reward items.
    ///
    pub fn cindwar_reward_items(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, Vec<CindwarRewardItemRow>> {
        self.cindwar_reward_items.read()
    }
    /// Get a read lock on the Cinderella War stat/skill presets.
    ///
    pub fn cindwar_stats(&self) -> parking_lot::RwLockReadGuard<'_, Vec<CindwarStatRow>> {
        self.cindwar_stats.read()
    }
    /// Access the shared soccer event state (per-zone rooms).
    ///
    pub fn soccer_state(&self) -> &crate::handler::soccer::SharedSoccerState {
        &self.soccer_state
    }
    // ── Battle (War) System Accessors ────────────────────────────────────

    /// Check if any war is currently open.
    ///
    pub fn is_war_open(&self) -> bool {
        let state = self.battle_state.read();
        state.is_war_open()
    }
    /// Get a clone of the current battle state.
    pub fn get_battle_state(&self) -> crate::systems::war::BattleState {
        self.battle_state.read().clone()
    }
    /// Mutate the battle state through a closure.
    ///
    /// Acquires a write lock and passes a mutable reference to the closure.
    pub fn update_battle_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut crate::systems::war::BattleState) -> R,
    {
        let mut state = self.battle_state.write();
        f(&mut state)
    }

    /// Get the current discount state: 0=off, 1=winning nation, 2=all.
    ///
    pub fn get_discount(&self) -> u8 {
        self.discount.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Check if the gold discount applies to a player of the given nation.
    ///
    pub fn is_discount_active(&self, player_nation: u8) -> bool {
        let disc = self.get_discount();
        if disc == 2 {
            return true;
        }
        if disc == 1 {
            let old_victory = self.battle_state.read().old_victory;
            return old_victory == player_nation;
        }
        false
    }

    /// Apply or revert NPC war buffs for all nation NPCs (type > 10, nation 1 or 2).
    ///
    ///
    /// On BATTLEZONE_OPEN: HP×1.2, AC×1.2, Damage×0.5, Resist×2 — monsters become
    /// tankier but deal less damage during war.
    /// On BATTLEZONE_CLOSE: Revert to template values.
    pub fn change_ability_all_npcs(&self, open: bool) {
        use std::sync::atomic::Ordering::Relaxed;

        self.npc_war_buffed.store(open, Relaxed);

        // Update HP for all affected NPCs
        for entry in self.npc_instances.iter() {
            let inst = entry.value();
            let tmpl = match self.get_npc_template(inst.proto_id, inst.is_monster) {
                Some(t) => t,
                None => continue,
            };

            // C++ filter: GetType() > 10 && (GetNation() == KARUS || GetNation() == ELMORAD)
            if tmpl.npc_type <= 10 || (tmpl.group != 1 && tmpl.group != 2) {
                continue;
            }

            let nid = inst.nid;
            if open {
                // War opened: buff HP to template * 1.2
                let new_max_hp = (tmpl.max_hp as f64 * 1.2) as i32;
                let current_hp = self.get_npc_hp(nid).unwrap_or(tmpl.max_hp as i32);
                if new_max_hp > current_hp {
                    // C++ sets HP = new_max_hp - 50 when max > current
                    self.update_npc_hp(nid, new_max_hp - 50);
                }
            } else {
                // War closed: revert HP to template max
                let original_max = tmpl.max_hp as i32;
                let current_hp = self.get_npc_hp(nid).unwrap_or(original_max);
                if original_max > current_hp {
                    self.update_npc_hp(nid, original_max - 50);
                } else {
                    self.update_npc_hp(nid, original_max);
                }
            }
        }

        let label = if open {
            "OPEN (buffed)"
        } else {
            "CLOSE (reverted)"
        };
        tracing::info!("ChangeAbilityAllNPCs: NPC war buffs {label}");
    }

    /// Check if a template qualifies for NPC war buffs.
    ///
    /// C++ filter: `GetType() > 10 && (GetNation() == KARUS || GetNation() == ELMORAD)`
    fn is_npc_war_target(&self, tmpl: &crate::npc::NpcTemplate) -> bool {
        self.npc_war_buffed
            .load(std::sync::atomic::Ordering::Relaxed)
            && tmpl.npc_type > 10
            && (tmpl.group == 1 || tmpl.group == 2)
    }

    /// Get war-adjusted NPC damage for combat calculations.
    ///
    pub fn get_npc_war_damage(&self, tmpl: &crate::npc::NpcTemplate) -> i32 {
        let base = tmpl.damage as i32;
        if self.is_npc_war_target(tmpl) {
            (base as f64 * 0.5) as i32
        } else {
            base
        }
    }

    /// Get war-adjusted NPC AC (defense) for combat calculations.
    ///
    pub fn get_npc_war_ac(&self, tmpl: &crate::npc::NpcTemplate) -> i32 {
        let base = tmpl.ac as i32;
        if self.is_npc_war_target(tmpl) {
            (base as f64 * 1.2) as i32
        } else {
            base
        }
    }

    /// Get war-adjusted NPC max HP for combat calculations.
    ///
    pub fn get_npc_war_max_hp(&self, tmpl: &crate::npc::NpcTemplate) -> i32 {
        let base = tmpl.max_hp as i32;
        if self.is_npc_war_target(tmpl) {
            (base as f64 * 1.2) as i32
        } else {
            base
        }
    }

    /// Get war-adjusted NPC resistance for a given element.
    ///
    pub fn get_npc_war_resist(&self, base_resist: i16, tmpl: &crate::npc::NpcTemplate) -> i16 {
        if self.is_npc_war_target(tmpl) {
            base_resist.saturating_mul(2)
        } else {
            base_resist
        }
    }

    /// Check if a character name is a designated war commander.
    ///
    pub fn is_war_commander(&self, name: &str) -> bool {
        self.war_commanders.read().contains(name)
    }

    /// Add a name to the war commander set.
    ///
    pub fn add_war_commander(&self, name: String) {
        self.war_commanders.write().insert(name);
    }

    /// Clear all war commanders (called on war reset).
    ///
    pub fn clear_war_commanders(&self) {
        self.war_commanders.write().clear();
    }

    /// Get a snapshot of all war commander names.
    pub fn get_war_commander_names(&self) -> Vec<String> {
        self.war_commanders.read().iter().cloned().collect()
    }

    /// Count players in a specific zone by nation.
    ///
    /// Returns (total, karus, elmorad).
    pub fn count_players_in_zone(&self, zone_id: u16) -> (u16, u16, u16) {
        let mut total = 0u16;
        let mut karus = 0u16;
        let mut elmorad = 0u16;
        for entry in self.sessions.iter() {
            let h = entry.value();
            if let Some(ref ch) = h.character {
                if h.position.zone_id == zone_id {
                    total += 1;
                    if ch.nation == 1 {
                        karus += 1;
                    } else {
                        elmorad += 1;
                    }
                }
            }
        }
        (total, karus, elmorad)
    }

    /// Count online players at a specific level.
    pub fn count_players_at_level(&self, level: u8) -> u16 {
        let mut count = 0u16;
        for entry in self.sessions.iter() {
            let h = entry.value();
            if let Some(ref ch) = h.character {
                if ch.level == level {
                    count += 1;
                }
            }
        }
        count
    }
    /// Get the current battle zone ID (ZONE_BATTLE_BASE + offset).
    pub fn get_battle_zone_id(&self) -> u16 {
        let state = self.battle_state.read();
        state.battle_zone_id()
    }
    /// Get monument capture points for both nations from the battle state.
    ///
    /// Returns `(karus_monument_point, elmorad_monument_point)` as i16 values.
    ///
    pub fn get_battle_monument_points(&self) -> (i16, i16) {
        let state = self.battle_state.read();
        (
            state.karus_monument_point as i16,
            state.elmorad_monument_point as i16,
        )
    }
    /// Get player death counts for both nations from the battle state.
    ///
    /// Returns `(karus_dead, elmorad_dead)`.
    ///
    pub fn get_battle_dead_counts(&self) -> (i16, i16) {
        let state = self.battle_state.read();
        (state.karus_dead, state.elmorad_dead)
    }
    /// Get the current victory nation (0=none, 1=Karus, 2=ElMorad).
    pub fn battle_victory_nation(&self) -> u8 {
        let state = self.battle_state.read();
        state.victory
    }
    /// Increment the death counter for a nation.
    ///
    /// Called when a player dies in a war zone.
    pub fn increment_war_death(&self, nation: u8) {
        let mut state = self.battle_state.write();
        match nation {
            1 => state.karus_dead = state.karus_dead.saturating_add(1),
            2 => state.elmorad_dead = state.elmorad_dead.saturating_add(1),
            _ => {}
        }
    }
    /// Increment the NPC kill counter for a nation.
    ///
    /// Called when a war NPC is killed (the nation parameter is the NPC's nation).
    pub fn increment_war_npc_kill(&self, npc_nation: u8) {
        let mut state = self.battle_state.write();
        match npc_nation {
            1 => state.killed_karus_npc = state.killed_karus_npc.saturating_add(1),
            2 => state.killed_elmorad_npc = state.killed_elmorad_npc.saturating_add(1),
            _ => {}
        }
    }
    /// Add monument capture points for a nation.
    pub fn add_monument_points(&self, nation: u8, points: u16) {
        let mut state = self.battle_state.write();
        match nation {
            1 => {
                state.karus_monument_point = state.karus_monument_point.saturating_add(points);
                state.karus_monuments = state.karus_monuments.saturating_add(1);
            }
            2 => {
                state.elmorad_monument_point = state.elmorad_monument_point.saturating_add(points);
                state.elmorad_monuments = state.elmorad_monuments.saturating_add(1);
            }
            _ => {}
        }
    }
    /// Increment a flag capture for a nation and return whether it triggers victory.
    pub fn increment_war_flag(&self, nation: u8) -> bool {
        let mut state = self.battle_state.write();
        match nation {
            1 => {
                state.karus_flag = state.karus_flag.saturating_add(1);
                state.karus_flag >= crate::systems::war::NUM_FLAG_VICTORY
            }
            2 => {
                state.elmorad_flag = state.elmorad_flag.saturating_add(1);
                state.elmorad_flag >= crate::systems::war::NUM_FLAG_VICTORY
            }
            _ => false,
        }
    }
    /// Update zone user counts for the current war zone.
    ///
    pub fn update_battle_zone_user_counts(&self) {
        let battle_zone_id = {
            let state = self.battle_state.read();
            if !state.is_war_open() {
                return;
            }
            state.battle_zone_id()
        };

        let mut karus_count: i16 = 0;
        let mut elmorad_count: i16 = 0;

        for entry in self.sessions.iter() {
            let handle: &SessionHandle = entry.value();
            if handle.position.zone_id != battle_zone_id {
                continue;
            }
            if let Some(ref ch) = handle.character {
                match ch.nation {
                    1 => karus_count = karus_count.saturating_add(1),
                    2 => elmorad_count = elmorad_count.saturating_add(1),
                    _ => {}
                }
            }
        }

        let mut state = self.battle_state.write();
        state.karus_count = karus_count;
        state.elmorad_count = elmorad_count;
    }
    /// Look up a rental item by rental index.
    ///
    pub fn get_rental_item(&self, rental_index: i32) -> Option<RentalItemRow> {
        self.rental_items.get(&rental_index).map(|r| r.clone())
    }

    /// Get all rental items available for browsing.
    ///
    /// Returns all items in the rental catalog. Used by RENTAL_OPEN
    /// to send the rental list to the client.
    pub fn get_all_rental_items(&self) -> Vec<RentalItemRow> {
        self.rental_items
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Insert a new rental item into the in-memory catalog.
    ///
    /// Called when a player registers an item for rental via RENTAL_REGISTER.
    pub fn insert_rental_item(&self, item: RentalItemRow) {
        self.rental_items.insert(item.rental_index, item);
    }

    /// Remove a rental item from the in-memory catalog by index.
    ///
    /// Called when a player cancels a rental registration via RENTAL_ITEM_CANCEL.
    pub fn remove_rental_item(&self, rental_index: i32) -> Option<RentalItemRow> {
        self.rental_items.remove(&rental_index).map(|(_k, v)| v)
    }

    /// Generate the next rental index atomically.
    ///
    /// Uses an `AtomicI32` counter to avoid race conditions when multiple
    /// sessions register rental items concurrently.
    pub fn next_rental_index(&self) -> i32 {
        self.rental_index_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    }

    /// Initialize the rental index counter from existing data (call after loading).
    pub fn init_rental_index_counter(&self) {
        let max_idx = self
            .rental_items
            .iter()
            .map(|r| *r.key())
            .max()
            .unwrap_or(0);
        self.rental_index_counter
            .store(max_idx, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get a mutable reference to a rental item entry for in-place updates.
    ///
    /// Used by RENTAL_LEND to set the borrower_char_id on the catalog entry.
    /// Returns `None` if the rental_index is not found.
    pub fn rental_items_entry(
        &self,
        rental_index: i32,
    ) -> Option<dashmap::mapref::one::RefMut<'_, i32, RentalItemRow>> {
        self.rental_items.get_mut(&rental_index)
    }
    // ── Cash Shop (PUS) Accessors ────────────────────────────────────

    /// Get all active PUS categories.
    pub fn get_pus_categories(&self) -> Vec<PusCategoryRow> {
        self.pus_categories
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }
    /// Get all PUS items (for ext_hook catalog send).
    pub fn get_pus_items_all(&self) -> Vec<PusItemRow> {
        self.pus_items_by_id
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }
    /// Get PUS items for a specific category.
    pub fn get_pus_items_by_category(&self, category_id: i16) -> Vec<PusItemRow> {
        self.pus_items_by_category
            .get(&category_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }
    /// Look up a PUS item by its listing ID.
    pub fn get_pus_item(&self, listing_id: i32) -> Option<PusItemRow> {
        self.pus_items_by_id
            .get(&listing_id)
            .map(|r| r.value().clone())
    }
    /// Get total PUS item count (for diagnostics).
    pub fn pus_item_count(&self) -> usize {
        self.pus_items_by_id.len()
    }
    /// Get total PUS category count (for diagnostics).
    pub fn pus_category_count(&self) -> usize {
        self.pus_categories.len()
    }
    /// Get all crafting recipes for a given NPC ID.
    ///
    /// Returns the list of `ItemSpecialSewingRow` entries that match the NPC.
    pub fn get_special_sewing_recipes(&self, npc_id: i32) -> Option<Vec<ItemSpecialSewingRow>> {
        self.special_sewing.get(&npc_id).map(|r| r.clone())
    }
    /// Get all item smash entries within a given index range (e.g., 2000000..3000000).
    ///
    pub fn get_item_smash_in_range(&self, range_start: i32, range_end: i32) -> Vec<ItemSmashRow> {
        self.item_smash
            .iter()
            .filter(|entry| {
                let idx = *entry.key();
                idx >= range_start && idx < range_end
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
    /// Look up an item smash entry by its index.
    ///
    pub fn get_item_smash(&self, n_index: i32) -> Option<ItemSmashRow> {
        self.item_smash.get(&n_index).map(|r| r.clone())
    }
    // ── Quest Accessors ──────────────────────────────────────────────

    /// Look up a quest helper definition by nIndex.
    ///
    pub fn get_quest_helper(&self, n_index: u32) -> Option<QuestHelperRow> {
        self.quest_helpers.get(&n_index).map(|r| r.clone())
    }

    pub fn is_boosted_collection_drop(&self, zone: u16, item: u32) -> bool {
        matches!(zone, 1 | 11 | 12) && self.collection_drop_items.contains_key(&item)
    }
    /// Look up a quest monster definition by quest num (sEventDataIndex).
    ///
    pub fn get_quest_monster(&self, quest_num: u16) -> Option<QuestMonsterRow> {
        self.quest_monsters.get(&quest_num).map(|r| r.clone())
    }
    /// Get the list of quest helper indices for an NPC.
    ///
    pub fn get_quest_npc_helpers(&self, npc_id: u16) -> Option<Vec<u32>> {
        self.quest_npc_list.get(&npc_id).map(|r| r.clone())
    }
    /// Look up a quest menu option by ID.
    ///
    pub fn get_quest_menu(&self, i_num: i32) -> Option<QuestMenuRow> {
        self.quest_menus.get(&i_num).map(|r| r.clone())
    }
    /// Look up a quest talk text by ID.
    ///
    pub fn get_quest_talk(&self, i_num: i32) -> Option<QuestTalkRow> {
        self.quest_talks.get(&i_num).map(|r| r.clone())
    }
    /// Look up a quest skill closed check entry by index.
    pub fn get_quest_skills_closed_check(&self, n_index: i32) -> Option<QuestSkillsClosedCheckRow> {
        self.quest_skills_closed_check
            .get(&n_index)
            .map(|r| r.clone())
    }
    /// Look up a quest skill open setup entry by index.
    pub fn get_quest_skills_open_set_up(&self, n_index: i32) -> Option<QuestSkillsOpenSetUpRow> {
        self.quest_skills_open_set_up
            .get(&n_index)
            .map(|r| r.clone())
    }
    /// Get the total number of loaded quest menu entries.
    pub fn quest_menu_count(&self) -> usize {
        self.quest_menus.len()
    }
    /// Get the total number of loaded quest talk entries.
    pub fn quest_talk_count(&self) -> usize {
        self.quest_talks.len()
    }
    /// Update a player's loyalty (Nation Points).
    ///
    pub fn update_character_loyalty(&self, id: SessionId, loyalty: u32) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                ch.loyalty = loyalty;
            }
        }
    }
    /// Update a player's monthly loyalty (monthly Nation Points).
    ///
    pub fn update_character_loyalty_monthly(&self, id: SessionId, loyalty_monthly: u32) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                ch.loyalty_monthly = loyalty_monthly;
            }
        }
    }
    /// Add a player to the PK zone ranking.
    ///
    pub fn pk_zone_add_player(&self, sid: SessionId, nation: u8, zone_id: u16) {
        if !(1..=2).contains(&nation) {
            return;
        }
        let nation_idx = (nation - 1) as usize;
        let other_idx = if nation_idx == 0 { 1 } else { 0 };
        self.pk_zone_rankings[other_idx].remove(&sid);
        self.pk_zone_rankings[nation_idx].insert(
            sid,
            PkZoneRanking {
                session_id: sid,
                zone_id,
                nation,
                loyalty_daily: 0,
                loyalty_premium_bonus: 0,
            },
        );
    }
    /// Remove a player from the PK zone ranking.
    pub fn pk_zone_remove_player(&self, sid: SessionId) {
        self.pk_zone_rankings[0].remove(&sid);
        self.pk_zone_rankings[1].remove(&sid);
    }
    /// Update a player's PK zone ranking loyalty.
    pub fn pk_zone_update_player(
        &self,
        sid: SessionId,
        nation: u8,
        loyalty_daily: u32,
        loyalty_premium_bonus: u16,
    ) {
        if !(1..=2).contains(&nation) {
            return;
        }
        let idx = (nation - 1) as usize;
        if let Some(mut r) = self.pk_zone_rankings[idx].get_mut(&sid) {
            r.loyalty_daily = loyalty_daily;
            r.loyalty_premium_bonus = loyalty_premium_bonus;
        }
    }
    /// Increment a player's daily loyalty in the PK zone ranking.
    ///
    /// then calls `UpdatePlayerKillingRank()`.
    pub fn pk_zone_increment_daily(&self, sid: SessionId, nation: u8, amount: u32) {
        if !(1..=2).contains(&nation) || amount == 0 {
            return;
        }
        let zone_id = self.with_session(sid, |h| h.position.zone_id).unwrap_or(0);
        let mut loyalty_daily = amount;
        let mut loyalty_premium_bonus = 0;
        self.update_session(sid, |h| {
            h.pk_loyalty_daily = h.pk_loyalty_daily.saturating_add(amount).min(2_100_000_000);
            loyalty_daily = h.pk_loyalty_daily;
            loyalty_premium_bonus = h.pk_loyalty_premium_bonus;
        });

        let idx = (nation - 1) as usize;
        if let Some(mut r) = self.pk_zone_rankings[idx].get_mut(&sid) {
            r.zone_id = zone_id;
            r.loyalty_daily = loyalty_daily;
            r.loyalty_premium_bonus = loyalty_premium_bonus;
        } else {
            // Zone changes do not re-run GAMESTART, so lazily register the
            // player on their first PK gain if the rank entry is absent.
            let other_idx = if idx == 0 { 1 } else { 0 };
            self.pk_zone_rankings[other_idx].remove(&sid);
            self.pk_zone_rankings[idx].insert(
                sid,
                PkZoneRanking {
                    session_id: sid,
                    zone_id,
                    nation,
                    loyalty_daily,
                    loyalty_premium_bonus,
                },
            );
        }
    }
    /// Get sorted PK zone rankings for a nation, filtered by zone.
    pub fn pk_zone_get_sorted(&self, nation_idx: usize, zone_id: u16) -> Vec<PkZoneRanking> {
        if nation_idx > 1 {
            return Vec::new();
        }
        let mut v: Vec<PkZoneRanking> = self.pk_zone_rankings[nation_idx]
            .iter()
            .filter(|e| e.value().zone_id == zone_id)
            .map(|e| e.value().clone())
            .collect();
        v.sort_by(|a, b| b.loyalty_daily.cmp(&a.loyalty_daily));
        v
    }
    /// Get a player's rank position in PK zone rankings.
    pub fn pk_zone_get_player_rank(&self, sid: SessionId, nation: u8, zone_id: u16) -> u16 {
        if !(1..=2).contains(&nation) {
            return 0;
        }
        let sorted = self.pk_zone_get_sorted((nation - 1) as usize, zone_id);
        for (i, e) in sorted.iter().enumerate() {
            if e.session_id == sid {
                return (i + 1) as u16;
            }
        }
        0
    }
    /// Add a player to BDW ranking.
    pub fn bdw_add_player(&self, sid: SessionId, nation: u8, event_room: i16) {
        if !(1..=2).contains(&nation) {
            return;
        }
        let idx = (nation - 1) as usize;
        let other = if idx == 0 { 1 } else { 0 };
        self.bdw_rankings[other].remove(&sid);
        self.bdw_rankings[idx].insert(
            sid,
            BdwRanking {
                session_id: sid,
                event_room,
                nation,
                user_point: 0,
            },
        );
    }
    /// Remove a player from BDW ranking.
    pub fn bdw_remove_player(&self, sid: SessionId) {
        self.bdw_rankings[0].remove(&sid);
        self.bdw_rankings[1].remove(&sid);
    }
    /// Update a player's BDW points.
    pub fn bdw_update_player(&self, sid: SessionId, nation: u8, user_point: u32) {
        if !(1..=2).contains(&nation) {
            return;
        }
        let idx = (nation - 1) as usize;
        if let Some(mut r) = self.bdw_rankings[idx].get_mut(&sid) {
            r.user_point = user_point;
        }
    }
    /// Add a player to Chaos Expansion ranking.
    pub fn chaos_add_player(&self, sid: SessionId, event_room: i16) {
        self.chaos_rankings.insert(
            sid,
            ChaosRanking {
                session_id: sid,
                event_room,
                kill_count: 0,
                death_count: 0,
            },
        );
    }
    /// Remove a player from Chaos Expansion ranking.
    pub fn chaos_remove_player(&self, sid: SessionId) {
        self.chaos_rankings.remove(&sid);
    }
    /// Update a player's Chaos kill/death counts.
    pub fn chaos_update_player(&self, sid: SessionId, kill_count: u16, death_count: u16) {
        if let Some(mut r) = self.chaos_rankings.get_mut(&sid) {
            r.kill_count = kill_count;
            r.death_count = death_count;
        }
    }
    /// Reset all PK zone rankings.
    ///
    pub fn reset_pk_zone_rankings(&self) {
        self.ranking_update_in_progress
            .store(true, Ordering::Relaxed);
        for idx in 0..2 {
            for mut e in self.pk_zone_rankings[idx].iter_mut() {
                e.value_mut().loyalty_daily = 0;
                e.value_mut().loyalty_premium_bonus = 0;
            }
        }
        for mut e in self.sessions.iter_mut() {
            e.value_mut().pk_loyalty_daily = 0;
            e.value_mut().pk_loyalty_premium_bonus = 0;
        }
        self.ranking_update_in_progress
            .store(false, Ordering::Relaxed);
    }
    /// Add a player to the Zindan War (special event) ranking.
    ///
    pub fn zindan_add_player(&self, sid: SessionId, nation: u8, zone_id: u16) {
        if !(1..=2).contains(&nation) {
            return;
        }
        let nation_idx = (nation - 1) as usize;
        let other_idx = if nation_idx == 0 { 1 } else { 0 };
        self.zindan_rankings[other_idx].remove(&sid);
        self.zindan_rankings[nation_idx].insert(
            sid,
            PkZoneRanking {
                session_id: sid,
                zone_id,
                nation,
                loyalty_daily: 0,
                loyalty_premium_bonus: 0,
            },
        );
    }
    /// Remove a player from Zindan War ranking.
    pub fn zindan_remove_player(&self, sid: SessionId) {
        self.zindan_rankings[0].remove(&sid);
        self.zindan_rankings[1].remove(&sid);
    }
    /// Update a player's Zindan War ranking loyalty.
    pub fn zindan_update_player(
        &self,
        sid: SessionId,
        nation: u8,
        loyalty_daily: u32,
        loyalty_premium_bonus: u16,
    ) {
        if !(1..=2).contains(&nation) {
            return;
        }
        let idx = (nation - 1) as usize;
        if let Some(mut r) = self.zindan_rankings[idx].get_mut(&sid) {
            r.loyalty_daily = loyalty_daily;
            r.loyalty_premium_bonus = loyalty_premium_bonus;
        }
    }
    /// Get sorted Zindan War rankings for a nation, filtered by zone.
    pub fn zindan_get_sorted(&self, nation_idx: usize, zone_id: u16) -> Vec<PkZoneRanking> {
        if nation_idx > 1 {
            return Vec::new();
        }
        let mut v: Vec<PkZoneRanking> = self.zindan_rankings[nation_idx]
            .iter()
            .filter(|e| e.value().zone_id == zone_id)
            .map(|e| e.value().clone())
            .collect();
        v.sort_by(|a, b| b.loyalty_daily.cmp(&a.loyalty_daily));
        v
    }
    // ── Pet System Accessors ────────────────────────────────────────────

    /// Look up pet stats info by pet level.
    ///
    pub fn get_pet_stats_info(&self, level: u8) -> Option<PetStatsInfoRow> {
        self.pet_stats_info.get(&level).map(|r| r.clone())
    }
    /// Look up a pet image change recipe by index.
    ///
    pub fn get_pet_image_change(&self, index: i32) -> Option<PetImageChangeRow> {
        self.pet_image_changes.get(&index).map(|r| r.clone())
    }
    /// Find all pet image change recipes matching a required item.
    ///
    pub fn find_pet_transforms_by_item(&self, item_id: i32) -> Vec<PetImageChangeRow> {
        self.pet_image_changes
            .iter()
            .filter(|e| e.value().n_req_item0 == item_id)
            .map(|e| e.value().clone())
            .collect()
    }
    // ── Server Settings Accessors ─────────────────────────────────────

    /// Get a clone of the server settings (or None if not loaded).
    ///
    pub fn get_server_settings(&self) -> Option<ServerSettingsRow> {
        self.server_settings.read().clone()
    }
    /// Get persistent login messages (send_type=1).
    ///
    pub fn get_login_messages(&self) -> Vec<ko_db::models::SendMessage> {
        self.send_messages.read().clone()
    }
    /// Get a clone of the damage settings (or None if not loaded).
    ///
    pub fn get_damage_settings(&self) -> Option<DamageSettingsRow> {
        self.damage_settings.read().clone()
    }
    /// Get burning feature rates for a given flame level (1-3).
    ///
    /// Returns None if level is 0 or out of range.
    pub fn get_burning_feature(&self, flame_level: u16) -> Option<BurningFeatureRates> {
        if flame_level == 0 || flame_level > 3 {
            return None;
        }
        let features = self.burning_features.read();
        Some(features[(flame_level - 1) as usize])
    }

    /// Get the configured flash time duration (in minutes) from server settings.
    ///
    pub fn get_flash_time_setting(&self) -> u32 {
        self.get_server_settings()
            .map(|s| s.flash_time as u32)
            .unwrap_or(180)
    }

    /// Get home position for a nation.
    ///
    /// Source: `HOME` table, keyed by nation (1=Karus, 2=Elmorad).
    pub fn get_home_position(&self, nation: u8) -> Option<HomeRow> {
        self.home_positions.get(&nation).map(|r| r.clone())
    }
    /// Get the start position for a zone.
    ///
    pub fn get_start_position(&self, zone_id: u16) -> Option<ko_db::models::StartPositionRow> {
        self.start_positions.get(&zone_id).map(|r| r.clone())
    }

    /// Pick a random spawn point from the start_position_random table for a zone.
    ///
    /// Returns (x, z) with radius offset applied.
    pub fn get_start_position_random(&self, zone_id: u16) -> Option<(f32, f32)> {
        use rand::Rng;
        let entry = self.start_positions_random.get(&zone_id)?;
        let points = entry.value();
        if points.is_empty() {
            return None;
        }
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..points.len());
        let p = &points[idx];
        // C++ uses myrand(0, Radius) — positive-only offset
        let x = p.pos_x as f32
            + if p.radius > 0 {
                rng.gen_range(0..=p.radius) as f32
            } else {
                0.0
            };
        let z = p.pos_z as f32
            + if p.radius > 0 {
                rng.gen_range(0..=p.radius) as f32
            } else {
                0.0
            };
        Some((x, z))
    }

    /// Insert a start position row (for testing).
    #[cfg(test)]
    pub(crate) fn insert_start_position(&self, row: ko_db::models::StartPositionRow) {
        self.start_positions.insert(row.zone_id as u16, row);
    }

    /// Insert random spawn points for a zone (for testing).
    #[cfg(test)]
    pub(crate) fn insert_start_position_random(
        &self,
        zone_id: u16,
        rows: Vec<ko_db::models::StartPositionRandomRow>,
    ) {
        self.start_positions_random.insert(zone_id, rows);
    }

    // -- Monster Summon / Respawn / Boss Spawn Accessors ------------------

    /// Look up a monster summon entry by NPC template s_sid.
    pub fn get_monster_summon(&self, s_sid: i16) -> Option<MonsterSummonRow> {
        self.monster_summon_list.get(&s_sid).map(|r| r.clone())
    }
    /// Get all monster summon entries of a given type (1 or 2).
    pub fn get_monster_summons_by_type(&self, b_type: i16) -> Vec<MonsterSummonRow> {
        self.monster_summon_list
            .iter()
            .filter(|r| r.b_type == b_type)
            .map(|r| r.clone())
            .collect()
    }
    /// Look up a respawn chain entry: when monster `dead_sid` dies, what spawns?
    pub fn get_respawn_chain(&self, dead_sid: i16) -> Option<MonsterRespawnLoopRow> {
        self.monster_respawn_loop.get(&dead_sid).map(|r| r.clone())
    }

    /// Return Juraid Mountain spawn rows for a family/stage.
    ///
    /// Families 21-28 are the per-room Juraid waves loaded from
    /// `monster_juraid_respawn_list`.
    pub fn get_juraid_respawn_family(
        &self,
        family: i16,
    ) -> Vec<ko_db::models::MonsterJuraidRespawnRow> {
        self.monster_juraid_respawn
            .read()
            .iter()
            .filter(|row| row.family == family)
            .cloned()
            .collect()
    }

    /// Return the configured Juraid Deva Bird template for a family.
    pub fn get_juraid_deva_sid(&self, family: i16) -> Option<u16> {
        self.monster_juraid_respawn
            .read()
            .iter()
            .find(|row| row.family == family && row.s_sid == 8106)
            .map(|row| row.s_sid as u16)
    }
    /// Get all boss random spawn candidates for a given stage.
    pub fn get_boss_spawn_candidates(&self, stage: i32) -> Vec<MonsterBossRandomSpawnRow> {
        self.boss_random_spawn
            .get(&stage)
            .map(|v| v.clone())
            .unwrap_or_default()
    }
    /// Get the number of distinct boss spawn stages.
    pub fn boss_spawn_stage_count(&self) -> usize {
        self.boss_random_spawn.len()
    }
    /// Get all monster boss random stages (for startup spawn).
    ///
    pub fn get_boss_random_stages(&self) -> Vec<MonsterBossRandomStageRow> {
        self.monster_boss_random_stages.read().clone()
    }
    /// Queue a delayed NPC respawn (respawn chain from `monster_respawn_loop`).
    ///
    pub fn schedule_respawn(&self, entry: ScheduledRespawn) {
        self.scheduled_respawns.lock().push(entry);
    }
    /// Drain all ready-to-spawn entries (spawn_at <= now).
    ///
    /// Returns entries whose deadline has passed; they should be spawned by the caller.
    pub fn drain_ready_respawns(&self, now_secs: u64) -> Vec<ScheduledRespawn> {
        let mut queue = self.scheduled_respawns.lock();
        let mut ready = Vec::new();
        queue.retain(|e| {
            if e.spawn_at <= now_secs {
                ready.push(e.clone());
                false
            } else {
                true
            }
        });
        ready
    }
    /// Get the total number of monster summon entries.
    pub fn monster_summon_count(&self) -> usize {
        self.monster_summon_list.len()
    }
    /// Get the total number of respawn loop entries.
    pub fn respawn_loop_count(&self) -> usize {
        self.monster_respawn_loop.len()
    }
    /// Set the remaining seconds for the bifrost event.
    pub fn set_bifrost_remaining_secs(&self, secs: u32) {
        self.bifrost_remaining_secs
            .store(secs, std::sync::atomic::Ordering::Relaxed);
    }
    /// Get the remaining seconds for the active bifrost event (0 = inactive).
    ///
    pub fn get_bifrost_remaining_secs(&self) -> u32 {
        self.bifrost_remaining_secs
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    // ── Bowl Event Accessors ─────────────────────────────────────────

    /// Check if the bowl event is active.
    pub fn is_bowl_event_active(&self) -> bool {
        self.bowl_event_active
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Set the bowl event active state.
    pub fn set_bowl_event_active(&self, active: bool) {
        self.bowl_event_active
            .store(active, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the remaining seconds for the bowl event.
    pub fn get_bowl_event_time(&self) -> u16 {
        self.bowl_event_time
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Set the remaining seconds for the bowl event.
    pub fn set_bowl_event_time(&self, secs: u16) {
        self.bowl_event_time
            .store(secs, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the zone ID for the bowl event.
    pub fn get_bowl_event_zone(&self) -> u8 {
        self.bowl_event_zone
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Set the zone ID for the bowl event.
    pub fn set_bowl_event_zone(&self, zone: u8) {
        self.bowl_event_zone
            .store(zone, std::sync::atomic::Ordering::Relaxed);
    }

    /// Close the bowl event (reset all fields).
    ///
    pub fn close_bowl_event(&self) {
        self.set_bowl_event_active(false);
        self.set_bowl_event_time(0);
        self.set_bowl_event_zone(0);
    }

    // ── Item Exchange / Special Stone Accessors ─────────────────────

    /// Look up a special stone by index.
    pub fn get_special_stone(&self, n_index: i32) -> Option<SpecialStoneRow> {
        self.special_stones.get(&n_index).map(|v| v.clone())
    }
    /// Get all special stone rows as a Vec.
    ///
    /// Used by `SpecialStoneDeath` to filter by proto_id + zone_id.
    pub fn get_all_special_stones(&self) -> Vec<SpecialStoneRow> {
        self.special_stones
            .iter()
            .map(|e| e.value().clone())
            .collect()
    }
    /// Look up a monster resource kill notice by NPC proto_id (sid).
    ///
    pub fn get_monster_resource(&self, sid: i16) -> Option<ko_db::models::MonsterResource> {
        self.monster_resources.get(&sid).map(|v| v.clone())
    }
    /// Look up a random item entry by index.
    pub fn get_item_random(&self, n_index: i32) -> Option<ItemRandomRow> {
        self.item_random.get(&n_index).map(|v| v.clone())
    }
    /// Get all item random entries for a given session_id.
    pub fn get_item_random_by_session(&self, session_id: i16) -> Vec<ItemRandomRow> {
        self.item_random
            .iter()
            .filter(|e| e.value().session_id == session_id && e.value().status == 1)
            .map(|e| e.value().clone())
            .collect()
    }
    /// Look up an item group by group_id.
    pub fn get_item_group(&self, group_id: i16) -> Option<ItemGroupRow> {
        self.item_groups.get(&group_id).map(|v| v.clone())
    }
    /// Look up an item exchange exp entry by index.
    pub fn get_item_exchange_exp(&self, n_index: i32) -> Option<ItemExchangeExpRow> {
        self.item_exchange_exp.get(&n_index).map(|v| v.clone())
    }
    /// Look up an item give exchange entry by index.
    pub fn get_item_give_exchange(&self, exchange_index: i32) -> Option<ItemGiveExchangeRow> {
        self.item_give_exchange
            .get(&exchange_index)
            .map(|v| v.clone())
    }
    /// Insert an item give exchange row (for testing).
    #[cfg(test)]
    pub(crate) fn insert_item_give_exchange(&self, exchange_index: i32, row: ItemGiveExchangeRow) {
        self.item_give_exchange.insert(exchange_index, row);
    }
    /// Look up a right-click exchange mapping by item_id.
    pub fn get_right_click_exchange(&self, item_id: i32) -> Option<ItemRightClickExchangeRow> {
        self.item_right_click_exchange
            .get(&item_id)
            .map(|v| v.clone())
    }
    /// Look up a right exchange definition by item_id.
    pub fn get_right_exchange(&self, item_id: i32) -> Option<ItemRightExchangeRow> {
        self.item_right_exchange.get(&item_id).map(|v| v.clone())
    }

    /// Get all right-click exchange item IDs grouped by exchange type.
    ///
    /// `s_HShieldSoftwareRightExchangeArray` by `sType` (1-7).
    pub fn get_right_exchange_by_type(&self) -> std::collections::HashMap<u8, Vec<u32>> {
        let mut result: std::collections::HashMap<u8, Vec<u32>> = std::collections::HashMap::new();
        for entry in self.item_right_exchange.iter() {
            let exchange_type = entry.exchange_type.unwrap_or(0) as u8;
            if exchange_type > 0 {
                result
                    .entry(exchange_type)
                    .or_default()
                    .push(entry.item_id as u32);
            }
        }
        result
    }

    // ── Daily Quest Accessors ───────────────────────────────────────

    /// Look up a daily quest definition by ID.
    ///
    pub fn get_daily_quest(&self, id: i16) -> Option<DailyQuestRow> {
        self.daily_quests.get(&id).map(|v| v.clone())
    }
    /// Get all daily quest definitions as a vector.
    ///
    pub fn get_all_daily_quests(&self) -> Vec<DailyQuestRow> {
        self.daily_quests
            .iter()
            .map(|e| e.value().clone())
            .collect()
    }
    /// Get the cached daily rank data (loaded at startup).
    ///
    pub fn get_daily_rank_cache(&self) -> Vec<ko_db::models::daily_rank::DailyRankRow> {
        self.daily_rank_cache.read().clone()
    }

    // ── Character Creation Data ────────────────────────────────────

    /// Get starting equipment for a class type (items with item_id > 0).
    ///
    pub fn get_starting_equipment(&self, class_type: i16) -> Vec<CreateNewCharSetRow> {
        self.new_char_set
            .get(&class_type)
            .map(|v| v.clone())
            .unwrap_or_default()
    }
    /// Get starting stats for a class + job_type combination (job_type 0 = base).
    ///
    pub fn get_starting_stats(
        &self,
        class_type: i16,
        job_type: i16,
    ) -> Option<CreateNewCharValueRow> {
        self.new_char_value
            .get(&(class_type, job_type))
            .map(|v| v.clone())
    }
    // ── Event Room Manager ─────────────────────────────────────────

    /// Get a reference to the event room manager.
    ///
    pub fn event_room_manager(&self) -> &EventRoomManager {
        &self.event_room_manager
    }

    /// Get a read lock on the BDW manager.
    pub fn bdw_manager_read(&self) -> parking_lot::RwLockReadGuard<'_, BdwManager> {
        self.bdw_manager.read()
    }

    /// Get a write lock on the BDW manager.
    pub fn bdw_manager_write(&self) -> parking_lot::RwLockWriteGuard<'_, BdwManager> {
        self.bdw_manager.write()
    }

    // ── Juraid Bridge State ─────────────────────────────────────

    /// Update Juraid bridge state for a room. Called from event_system when bridges open.
    ///
    pub fn set_juraid_bridge_state(
        &self,
        room_id: u8,
        state: crate::systems::juraid::JuraidBridgeState,
    ) {
        self.juraid_bridge_states.insert(room_id, state);
    }

    /// Get the Juraid bridge state for a specific room.
    ///
    /// Returns `None` if no state is tracked for the room (event not active).
    pub fn get_juraid_bridge_state(
        &self,
        room_id: u8,
    ) -> Option<crate::systems::juraid::JuraidBridgeState> {
        self.juraid_bridge_states.get(&room_id).map(|bs| bs.clone())
    }

    /// Check if all 3 bridges are open for a nation in a specific Juraid room.
    ///
    pub fn are_all_juraid_bridges_open(&self, room_id: u8, nation: u8) -> bool {
        self.juraid_bridge_states
            .get(&room_id)
            .map(|bs| bs.open_count(nation) >= 3)
            .unwrap_or(false)
    }

    /// Clear all Juraid bridge states (called on event cleanup).
    pub fn clear_juraid_bridge_states(&self) {
        self.juraid_bridge_states.clear();
    }

    /// Schedule a Juraid Monument respawn for a specific room and nation.
    pub fn set_juraid_monument_respawn(&self, room_id: u8, nation: u8, due_at: u64) {
        if room_id == 0 || !matches!(nation, 1 | 2) {
            return;
        }
        self.juraid_monument_respawns
            .insert((room_id, nation), due_at);
    }

    /// Drain Juraid Monument respawns whose due timestamp has passed.
    pub fn take_due_juraid_monument_respawns(&self, now: u64) -> Vec<(u8, u8)> {
        let due: Vec<(u8, u8)> = self
            .juraid_monument_respawns
            .iter()
            .filter_map(|entry| {
                if *entry.value() <= now {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect();

        for key in &due {
            self.juraid_monument_respawns.remove(key);
        }
        due
    }

    /// Clear pending Juraid Monument respawns (called on event start/cleanup).
    pub fn clear_juraid_monument_respawns(&self) {
        self.juraid_monument_respawns.clear();
    }

    // ── Monster Stone ────────────────────────────────────────────

    /// Get a read lock on the Monster Stone manager.
    pub fn monster_stone_read(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, super::MonsterStoneManager> {
        self.monster_stone_manager.read()
    }

    /// Get a write lock on the Monster Stone manager.
    pub fn monster_stone_write(
        &self,
    ) -> parking_lot::RwLockWriteGuard<'_, super::MonsterStoneManager> {
        self.monster_stone_manager.write()
    }

    /// Get Monster Stone respawn entries filtered by zone and family.
    ///
    pub fn get_monster_stone_spawns(
        &self,
        zone_id: u8,
        family: u16,
    ) -> Vec<ko_db::models::MonsterStoneRespawnRow> {
        let list = self.monster_stone_respawn.read();
        list.iter()
            .filter(|r| r.zone_id == zone_id as i16 && r.family == family as i16)
            .cloned()
            .collect()
    }

    // ── Forgotten Temple ─────────────────────────────────────────

    /// Get a read lock on the FT stage definitions.
    ///
    pub fn ft_stages(&self) -> parking_lot::RwLockReadGuard<'_, Vec<FtStageRow>> {
        self.ft_stages.read()
    }
    /// Get a read lock on the FT summon definitions.
    ///
    pub fn ft_summons(&self) -> parking_lot::RwLockReadGuard<'_, Vec<FtSummonRow>> {
        self.ft_summons.read()
    }
    /// Get a reference to the Forgotten Temple runtime state.
    ///
    pub fn forgotten_temple_state(&self) -> &ForgettenTempleState {
        &self.forgotten_temple_state
    }
    // ── Dungeon Defence Accessors ─────────────────────────────────────

    /// Get a read lock on the DD stage definitions.
    ///
    pub fn dd_stages(&self) -> parking_lot::RwLockReadGuard<'_, Vec<DfStageRow>> {
        self.dd_stages.read()
    }
    /// Get a read lock on the DD monster spawn definitions.
    ///
    pub fn dd_monsters(&self) -> parking_lot::RwLockReadGuard<'_, Vec<DfMonsterRow>> {
        self.dd_monsters.read()
    }
    /// Get a reference to the DD runtime room pool (60 rooms).
    ///
    pub fn dd_rooms(&self) -> &[crate::handler::dungeon_defence::DdRoomInfo] {
        &self.dd_rooms
    }
    // ── Draki Tower Accessors ────────────────────────────────────────

    /// Get a read lock on the Draki Tower stage definitions.
    ///
    pub fn draki_tower_stages(&self) -> parking_lot::RwLockReadGuard<'_, Vec<DrakiTowerStageRow>> {
        self.draki_tower_stages.read()
    }
    /// Get a read lock on the Draki Tower monster list definitions.
    ///
    pub fn draki_monster_list(&self) -> parking_lot::RwLockReadGuard<'_, Vec<DrakiMonsterListRow>> {
        self.draki_monster_list.read()
    }
    /// Get a read lock on the Draki Tower runtime room pool.
    ///
    pub fn draki_tower_rooms_read(
        &self,
    ) -> parking_lot::RwLockReadGuard<
        '_,
        std::collections::HashMap<u16, crate::handler::draki_tower::DrakiTowerRoomInfo>,
    > {
        self.draki_tower_rooms.read()
    }
    /// Get a write lock on the Draki Tower runtime room pool.
    pub fn draki_tower_rooms_write(
        &self,
    ) -> parking_lot::RwLockWriteGuard<
        '_,
        std::collections::HashMap<u16, crate::handler::draki_tower::DrakiTowerRoomInfo>,
    > {
        self.draki_tower_rooms.write()
    }
    // ── Under The Castle Accessors ─────────────────────────────────────

    /// Get a reference to the Under The Castle spawn definitions.
    pub fn utc_spawns(&self) -> &parking_lot::RwLock<Vec<MonsterUnderTheCastleRow>> {
        &self.utc_spawns
    }
    /// Get a reference to the Under The Castle runtime state.
    ///
    pub fn under_the_castle_state(&self) -> &crate::handler::under_castle::UnderTheCastleState {
        &self.under_the_castle_state
    }
    // ── Chaos Stone Accessors ───────────────────────────────────────

    /// Get a read lock on the chaos stone spawn point definitions.
    ///
    pub fn chaos_stone_spawns(&self) -> parking_lot::RwLockReadGuard<'_, Vec<ChaosStoneSpawnRow>> {
        self.chaos_stone_spawns.read()
    }
    /// Get a read lock on the chaos stone monster summon list.
    ///
    pub fn chaos_stone_summon_list(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, Vec<ChaosStoneSummonListRow>> {
        self.chaos_stone_summon_list.read()
    }
    /// Get a read lock on the chaos stone stage/family definitions.
    ///
    pub fn chaos_stone_stages(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, Vec<ChaosStoneSummonStageRow>> {
        self.chaos_stone_stages.read()
    }
    /// Get a read lock on the chaos stone event rewards.
    ///
    pub fn chaos_stone_rewards(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, Vec<EventChaosRewardRow>> {
        self.chaos_stone_rewards.read()
    }

    /// Get a write lock on the chaos stone event rewards (for testing/seeding).
    #[cfg(test)]
    pub(crate) fn chaos_stone_rewards_mut(
        &self,
    ) -> parking_lot::RwLockWriteGuard<'_, Vec<EventChaosRewardRow>> {
        self.chaos_stone_rewards.write()
    }

    /// Get event rewards for a given local_id (e.g. 9=BDW, 11=Juraid).
    ///
    pub fn get_event_rewards(&self, local_id: i16) -> Option<Vec<EventRewardRow>> {
        self.event_rewards.get(&local_id).map(|r| r.clone())
    }

    // ── War / Monument / CSW Accessors ──────────────────────────────────

    /// Get the war victory state.
    ///
    pub(crate) fn get_victory(&self) -> u8 {
        self.battle_state.read().victory
    }

    /// Get the PVP monument nation for a specific zone.
    ///
    pub(crate) fn get_pvp_monument_nation(&self, zone_id: u16) -> u8 {
        self.pvp_monument_nation
            .get(&zone_id)
            .map(|v| *v)
            .unwrap_or(0)
    }

    /// Set the PVP monument nation for a specific zone.
    ///
    pub(crate) fn set_pvp_monument_nation(&self, zone_id: u16, nation: u8) {
        self.pvp_monument_nation.insert(zone_id, nation);
    }

    /// Get the middle statue nation ownership.
    ///
    pub(crate) fn get_middle_statue_nation(&self) -> u8 {
        self.battle_state.read().middle_statue_nation
    }

    /// Get the CSW (Castle Siege War) master knights clan ID.
    ///
    pub(crate) fn get_csw_master_knights(&self) -> u16 {
        self.siege_war.try_read().map(|state| state.master_knights).unwrap_or(0)
    }

    /// Get a player's rebirth level from session.
    ///
    pub(crate) fn get_rebirth_level(&self, sid: SessionId) -> u8 {
        self.with_session(sid, |h| {
            h.character.as_ref().map(|c| c.rebirth_level).unwrap_or(0)
        })
        .unwrap_or(0)
    }

    // ── Event State Accessors (for Lua bindings) ─────────────────────

    /// Check if the Forgotten Temple join phase is open and the player's
    /// level is within the allowed range.
    ///
    /// (Confusing name — actually checks FT, not Monster Challenge.)
    pub(crate) fn is_ft_join_open_for_level(&self, level: u8) -> bool {
        let ft = self.forgotten_temple_state();
        if !ft.is_active.load(Ordering::Relaxed) || !ft.is_join.load(Ordering::Relaxed) {
            return false;
        }
        let min_level = ft.min_level.load(Ordering::Relaxed) as u8;
        let max_level = ft.max_level.load(Ordering::Relaxed) as u8;
        if min_level > 0 && level < min_level {
            return false;
        }
        if max_level > 0 && level > max_level {
            return false;
        }
        true
    }

    /// Get the number of users signed up for Forgotten Temple.
    ///
    /// Uses `pForgettenTemple.UserList.size()` — we approximate with all_user_count from temple event.
    pub(crate) fn get_ft_user_count(&self) -> u16 {
        self.event_room_manager
            .read_temple_event(|s| s.all_user_count)
    }

    /// Check if the Under The Castle event is currently active.
    ///
    pub(crate) fn is_under_castle_active(&self) -> bool {
        self.under_the_castle_state()
            .is_active
            .load(Ordering::Relaxed)
    }

    /// Get the number of users currently in the Under The Castle zone.
    ///
    /// C++ uses `pUnderTheCastle.UserList.size()` — we count sessions in the UTC zone.
    pub(crate) fn get_under_castle_user_count(&self) -> u16 {
        self.get_users_in_zone(ZONE_UNDER_CASTLE).len() as u16
    }

    /// Check if the Juraid Mountain join phase is open.
    ///
    pub(crate) fn is_juraid_join_open(&self) -> bool {
        self.event_room_manager.read_temple_event(|s| {
            // TEMPLE_EVENT_JURAD_MOUNTAIN = 100 (shared/packets.h)
            s.active_event == 100 && s.allow_join
        })
    }

    /// Check if the beef roast event is active with farming play and a winner.
    ///
    /// Returns true if beef event is active, farming is on, and winner nation != 0.
    pub(crate) fn is_beef_event_farming(&self) -> bool {
        let state = self.beef_event.read();
        state.is_active && state.is_farming_play && state.winner_nation != 0
    }

    /// Mutate the beef event state through a closure.
    ///
    pub(crate) fn update_beef_event<F>(&self, f: F)
    where
        F: FnOnce(&mut crate::world::BeefEventState),
    {
        let mut state = self.beef_event.write();
        f(&mut state);
    }

    /// Get a clone of the current beef event state.
    pub(crate) fn get_beef_event(&self) -> crate::world::BeefEventState {
        self.beef_event.read().clone()
    }

    // ── Zone User Query ──────────────────────────────────────────────

    /// Get all session IDs of players currently in a given zone.
    ///
    pub(crate) fn get_users_in_zone(&self, zone_id: u16) -> Vec<SessionId> {
        let mut sids = Vec::new();
        for entry in self.sessions.iter() {
            if entry.value().character.is_some() && entry.value().position.zone_id == zone_id {
                sids.push(*entry.key());
            }
        }
        sids
    }

    // ── Daily Operation System ───────────────────────────────────────

    /// Check if a daily operation is available (cooldown expired) and if so,
    /// update the timestamp. Returns 1 if allowed, 0 if still on cooldown.
    ///
    pub(crate) fn get_user_daily_op(&self, char_name: &str, op_type: u8) -> u8 {
        let op = match DailyOpCode::from_u8(op_type) {
            Some(op) => op,
            None => return 0,
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i32;

        // Check existing entry
        if let Some(mut entry) = self.daily_ops.get_mut(char_name) {
            let last_time = entry.get(op);
            if last_time == -1 {
                // Never performed — allow and set
                entry.set(op, now);
                return 1;
            }
            // Check if cooldown elapsed (1440 minutes)
            let elapsed_minutes = (now.wrapping_sub(last_time)) as i64 / 60;
            if elapsed_minutes > DAILY_OPERATIONS_MINUTE {
                entry.set(op, now);
                return 1;
            }
            // Still on cooldown
            0
        } else {
            // No entry yet — create and allow
            let mut data = UserDailyOp::new();
            data.set(op, now);
            self.daily_ops.insert(char_name.to_string(), data);
            1
        }
    }

    // ── Zone Reward Accessors ─────────────────────────────────────────

    /// Get all zone kill rewards for a specific zone.
    ///
    pub fn get_zone_kill_rewards(&self, zone_id: u16) -> Vec<ZoneKillReward> {
        let rewards = self.zone_kill_rewards.read();
        rewards
            .iter()
            .filter(|r| r.zone_id as u16 == zone_id && r.status == 1)
            .cloned()
            .collect()
    }

    /// Get all zone online reward definitions (global list).
    ///
    pub fn get_zone_online_rewards(&self) -> Vec<ZoneOnlineReward> {
        self.zone_online_rewards.read().clone()
    }

    /// Get the count of loaded zone kill reward entries.
    pub fn zone_kill_reward_count(&self) -> usize {
        self.zone_kill_rewards.read().len()
    }

    /// Get the count of loaded zone online reward entries.
    pub fn zone_online_reward_count(&self) -> usize {
        self.zone_online_rewards.read().len()
    }

    /// Insert zone kill reward entries (for testing).
    #[cfg(test)]
    pub(crate) fn insert_zone_kill_rewards(&self, rewards: Vec<ZoneKillReward>) {
        *self.zone_kill_rewards.write() = rewards;
    }

    /// Insert zone online reward entries (for testing).
    #[cfg(test)]
    pub(crate) fn insert_zone_online_rewards(&self, rewards: Vec<ZoneOnlineReward>) {
        *self.zone_online_rewards.write() = rewards;
    }

    /// Get the experience percentage (0-100) for a player.
    ///
    #[cfg(test)]
    pub(crate) fn get_exp_percent(&self, sid: SessionId) -> i32 {
        self.with_session(sid, |h| {
            if let Some(ref ch) = h.character {
                let max_exp = ch.max_exp;
                if max_exp <= 0 {
                    return 0;
                }
                ((ch.exp as f64 / max_exp as f64) * 100.0) as i32
            } else {
                0
            }
        })
        .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_world_with_session() -> (WorldState, SessionId) {
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        let info = CharacterInfo {
            session_id: sid,
            name: "TestPlayer".into(),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 1000,
            hp: 1000,
            max_mp: 500,
            mp: 500,
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
            skill_points: [0; 10],
            gold: 1000,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 5000,
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
        };
        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(sid, info, pos);
        (world, sid)
    }

    #[test]
    fn test_get_victory_default_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_victory(), 0);
    }

    #[test]
    fn test_get_victory_after_set() {
        let world = WorldState::new();
        world.update_battle_state(|state| {
            state.victory = 1;
        });
        assert_eq!(world.get_victory(), 1);
    }

    #[test]
    fn test_get_pvp_monument_nation_default_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_pvp_monument_nation(71), 0);
    }

    #[test]
    fn test_set_get_pvp_monument_nation() {
        let world = WorldState::new();
        world.set_pvp_monument_nation(71, 2); // El Morad owns zone 71
        assert_eq!(world.get_pvp_monument_nation(71), 2);
        assert_eq!(world.get_pvp_monument_nation(72), 0); // other zone unset
    }

    #[test]
    fn test_get_middle_statue_nation_default_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_middle_statue_nation(), 0);
    }

    #[test]
    fn test_get_middle_statue_nation_after_set() {
        let world = WorldState::new();
        world.update_battle_state(|state| {
            state.middle_statue_nation = 1;
        });
        assert_eq!(world.get_middle_statue_nation(), 1);
    }

    #[test]
    fn test_get_csw_master_knights_default_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_csw_master_knights(), 0);
    }

    #[test]
    fn test_get_rebirth_level_no_session() {
        let world = WorldState::new();
        assert_eq!(world.get_rebirth_level(999), 0);
    }

    #[test]
    fn test_get_rebirth_level_default() {
        let (world, sid) = setup_world_with_session();
        assert_eq!(world.get_rebirth_level(sid), 0);
    }

    #[test]
    fn test_get_rebirth_level_after_set() {
        let (world, sid) = setup_world_with_session();
        world.update_character_stats(sid, |ch| {
            ch.rebirth_level = 3;
        });
        assert_eq!(world.get_rebirth_level(sid), 3);
    }

    #[test]
    fn test_constants_match_cpp() {
        assert_eq!(ZONE_DELOS_CASTELLAN, 35);
        assert_eq!(DODO_CAMP_WARP_X, 10540);
        assert_eq!(DODO_CAMP_WARP_Z, 11410);
        assert_eq!(LAON_CAMP_WARP_X, 10120);
        assert_eq!(LAON_CAMP_WARP_Z, 9140);
        assert_eq!(DODO_LAON_WARP_RADIUS, 5);
        assert_eq!(NPC_SPECIAL_TYPE_CYCLE_SPAWN, 7);
    }

    // ── Event State Accessor Tests ─────────────────────────────────

    #[test]
    fn test_ft_join_closed_by_default() {
        let world = WorldState::new();
        assert!(!world.is_ft_join_open_for_level(60));
    }

    #[test]
    fn test_ft_join_open_level_in_range() {
        let world = WorldState::new();
        let ft = world.forgotten_temple_state();
        ft.is_active.store(true, Ordering::Relaxed);
        ft.is_join.store(true, Ordering::Relaxed);
        ft.min_level.store(40, Ordering::Relaxed);
        ft.max_level.store(70, Ordering::Relaxed);
        assert!(world.is_ft_join_open_for_level(60));
    }

    #[test]
    fn test_ft_join_level_too_low() {
        let world = WorldState::new();
        let ft = world.forgotten_temple_state();
        ft.is_active.store(true, Ordering::Relaxed);
        ft.is_join.store(true, Ordering::Relaxed);
        ft.min_level.store(40, Ordering::Relaxed);
        ft.max_level.store(70, Ordering::Relaxed);
        assert!(!world.is_ft_join_open_for_level(30));
    }

    #[test]
    fn test_ft_join_level_too_high() {
        let world = WorldState::new();
        let ft = world.forgotten_temple_state();
        ft.is_active.store(true, Ordering::Relaxed);
        ft.is_join.store(true, Ordering::Relaxed);
        ft.min_level.store(40, Ordering::Relaxed);
        ft.max_level.store(70, Ordering::Relaxed);
        assert!(!world.is_ft_join_open_for_level(71));
    }

    #[test]
    fn test_ft_user_count_default_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_ft_user_count(), 0);
    }

    #[test]
    fn test_under_castle_inactive_by_default() {
        let world = WorldState::new();
        assert!(!world.is_under_castle_active());
    }

    #[test]
    fn test_under_castle_active_after_set() {
        let world = WorldState::new();
        world
            .under_the_castle_state()
            .is_active
            .store(true, Ordering::Relaxed);
        assert!(world.is_under_castle_active());
    }

    #[test]
    fn test_under_castle_user_count_default_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_under_castle_user_count(), 0);
    }

    #[test]
    fn test_juraid_join_closed_by_default() {
        let world = WorldState::new();
        assert!(!world.is_juraid_join_open());
    }

    #[test]
    fn test_beef_event_farming_default_false() {
        let world = WorldState::new();
        assert!(!world.is_beef_event_farming());
    }

    // ── Zone User Query Tests ──────────────────────────────────────

    #[test]
    fn test_get_users_in_zone_empty() {
        let world = WorldState::new();
        assert!(world.get_users_in_zone(21).is_empty());
    }

    #[test]
    fn test_get_users_in_zone_with_session() {
        let (world, sid) = setup_world_with_session();
        let users = world.get_users_in_zone(21);
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], sid);
    }

    #[test]
    fn test_get_users_in_zone_different_zone() {
        let (world, _sid) = setup_world_with_session();
        // Session is in zone 21, should not appear in zone 22
        assert!(world.get_users_in_zone(22).is_empty());
    }

    // ── Daily Operation Tests ──────────────────────────────────────

    #[test]
    fn test_daily_op_first_use_allowed() {
        let world = WorldState::new();
        let result = world.get_user_daily_op("TestChar", 1);
        assert_eq!(result, 1); // Allowed on first use
    }

    #[test]
    fn test_daily_op_second_use_blocked() {
        let world = WorldState::new();
        world.get_user_daily_op("TestChar", 1); // First use
        let result = world.get_user_daily_op("TestChar", 1); // Second use
        assert_eq!(result, 0); // Blocked (within 1440 min cooldown)
    }

    #[test]
    fn test_daily_op_different_types_independent() {
        let world = WorldState::new();
        world.get_user_daily_op("TestChar", 1); // ChaosMap
        let result = world.get_user_daily_op("TestChar", 2); // UserRankReward
        assert_eq!(result, 1); // Different type, should be allowed
    }

    #[test]
    fn test_daily_op_invalid_type_returns_zero() {
        let world = WorldState::new();
        assert_eq!(world.get_user_daily_op("TestChar", 0), 0);
        assert_eq!(world.get_user_daily_op("TestChar", 99), 0);
    }

    #[test]
    fn test_daily_op_different_characters_independent() {
        let world = WorldState::new();
        world.get_user_daily_op("Char1", 1);
        let result = world.get_user_daily_op("Char2", 1);
        assert_eq!(result, 1); // Different character, should be allowed
    }

    // ── Exp Percent Tests ──────────────────────────────────────────

    #[test]
    fn test_exp_percent_zero_exp() {
        let (world, sid) = setup_world_with_session();
        assert_eq!(world.get_exp_percent(sid), 0);
    }

    #[test]
    fn test_exp_percent_half() {
        let (world, sid) = setup_world_with_session();
        world.update_character_stats(sid, |ch| {
            ch.exp = 50000;
            ch.max_exp = 100000;
        });
        assert_eq!(world.get_exp_percent(sid), 50);
    }

    #[test]
    fn test_exp_percent_no_session() {
        let world = WorldState::new();
        assert_eq!(world.get_exp_percent(999), 0);
    }

    #[test]
    fn test_daily_operations_minute_constant() {
        assert_eq!(DAILY_OPERATIONS_MINUTE, 1440);
    }

    #[test]
    fn test_get_wheel_of_fun_settings_empty() {
        let world = WorldState::new();
        assert!(world.get_wheel_of_fun_settings().is_empty());
    }

    // ── Discount System Tests ────────────────────────────────────────

    #[test]
    fn test_discount_default_off() {
        let world = WorldState::new();
        assert_eq!(world.get_discount(), 0);
        assert!(!world.is_discount_active(1));
        assert!(!world.is_discount_active(2));
    }

    #[test]
    fn test_discount_all_nations() {
        let world = WorldState::new();
        world
            .discount
            .store(2, std::sync::atomic::Ordering::Relaxed);
        assert!(world.is_discount_active(1));
        assert!(world.is_discount_active(2));
    }

    #[test]
    fn test_discount_winning_nation_only() {
        let world = WorldState::new();
        world
            .discount
            .store(1, std::sync::atomic::Ordering::Relaxed);
        // Set old_victory to Karus (1)
        world.update_battle_state(|s| {
            s.old_victory = 1;
        });
        assert!(world.is_discount_active(1)); // Karus = winning nation
        assert!(!world.is_discount_active(2)); // Elmorad = not winning
    }

    // ── NPC War Buff Tests ───────────────────────────────────────────

    #[test]
    fn test_npc_war_damage_no_buff() {
        let world = WorldState::new();
        let tmpl = crate::npc::NpcTemplate {
            s_sid: 100,
            is_monster: false,
            npc_type: 11, // > 10
            group: 1,     // Karus
            damage: 200,
            ac: 100,
            max_hp: 10000,
            ..default_npc_template()
        };
        // No war buff active
        assert_eq!(world.get_npc_war_damage(&tmpl), 200);
        assert_eq!(world.get_npc_war_ac(&tmpl), 100);
        assert_eq!(world.get_npc_war_max_hp(&tmpl), 10000);
    }

    #[test]
    fn test_npc_war_damage_buffed() {
        let world = WorldState::new();
        world
            .npc_war_buffed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let tmpl = crate::npc::NpcTemplate {
            s_sid: 100,
            is_monster: false,
            npc_type: 11, // > 10
            group: 1,     // Karus
            damage: 200,
            ac: 100,
            max_hp: 10000,
            ..default_npc_template()
        };
        assert_eq!(world.get_npc_war_damage(&tmpl), 100); // 200 * 0.5
        assert_eq!(world.get_npc_war_ac(&tmpl), 120); // 100 * 1.2
        assert_eq!(world.get_npc_war_max_hp(&tmpl), 12000); // 10000 * 1.2
    }

    #[test]
    fn test_npc_war_buff_neutral_not_affected() {
        let world = WorldState::new();
        world
            .npc_war_buffed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let tmpl = crate::npc::NpcTemplate {
            s_sid: 50,
            is_monster: true,
            npc_type: 11,
            group: 0, // Neutral — NOT affected
            damage: 200,
            ac: 100,
            max_hp: 10000,
            ..default_npc_template()
        };
        assert_eq!(world.get_npc_war_damage(&tmpl), 200); // unchanged
        assert_eq!(world.get_npc_war_ac(&tmpl), 100); // unchanged
    }

    #[test]
    fn test_npc_war_buff_low_type_not_affected() {
        let world = WorldState::new();
        world
            .npc_war_buffed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let tmpl = crate::npc::NpcTemplate {
            s_sid: 50,
            is_monster: true,
            npc_type: 5, // <= 10 — NOT affected
            group: 1,
            damage: 200,
            ac: 100,
            max_hp: 10000,
            ..default_npc_template()
        };
        assert_eq!(world.get_npc_war_damage(&tmpl), 200); // unchanged
    }

    #[test]
    fn test_npc_war_resist_doubled() {
        let world = WorldState::new();
        world
            .npc_war_buffed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let tmpl = crate::npc::NpcTemplate {
            s_sid: 100,
            is_monster: false,
            npc_type: 11,
            group: 2, // Elmorad
            ..default_npc_template()
        };
        assert_eq!(world.get_npc_war_resist(50, &tmpl), 100); // 50 * 2
        assert_eq!(world.get_npc_war_resist(0, &tmpl), 0); // 0 * 2
    }

    // ── Sprint 978: Additional coverage ──────────────────────────────

    /// get_coefficient returns None for unknown class.
    #[test]
    fn test_get_coefficient_unknown_class() {
        let world = WorldState::new();
        assert!(world.get_coefficient(9999).is_none());
    }

    /// get_item returns None for unknown item ID.
    #[test]
    fn test_get_item_unknown_id() {
        let world = WorldState::new();
        assert!(world.get_item(999999999).is_none());
    }

    /// get_magic returns None for unknown magic_num.
    #[test]
    fn test_get_magic_unknown() {
        let world = WorldState::new();
        assert!(world.get_magic(999999).is_none());
    }

    /// get_exp_by_level returns 0 for non-existent level.
    #[test]
    fn test_get_exp_by_level_missing() {
        let world = WorldState::new();
        assert_eq!(world.get_exp_by_level(99, 0), 0);
    }

    /// get_anti_afk_npc_ids returns empty by default.
    #[test]
    fn test_anti_afk_npc_ids_default_empty() {
        let world = WorldState::new();
        assert!(world.get_anti_afk_npc_ids().is_empty());
    }

    /// Helper to create a minimal NpcTemplate for tests.
    fn default_npc_template() -> crate::npc::NpcTemplate {
        crate::npc::NpcTemplate {
            s_sid: 0,
            is_monster: false,
            name: String::new(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 100,
            max_mp: 0,
            attack: 10,
            ac: 10,
            hit_rate: 100,
            evade_rate: 0,
            damage: 10,
            attack_delay: 1500,
            speed_1: 60,
            speed_2: 100,
            stand_time: 0,
            search_range: 0,
            attack_range: 100,
            direct_attack: 0,
            tracing_range: 0,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }
}
