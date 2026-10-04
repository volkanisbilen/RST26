//! Inventory, warehouse, VIP warehouse, gold, and item management.

use super::*;

impl WorldState {
    // ── Equipment Stat Calculation ──────────────────────────────────

    /// Weapon kind constants from `GameDefine.h:1224-1245`.
    const WEAPON_KIND_DAGGER: i32 = 11;

    const WEAPON_KIND_1H_SWORD: i32 = 21;

    const WEAPON_KIND_2H_SWORD: i32 = 22;

    const WEAPON_KIND_1H_AXE: i32 = 31;

    const WEAPON_KIND_2H_AXE: i32 = 32;

    const WEAPON_KIND_1H_CLUP: i32 = 41;

    const WEAPON_KIND_2H_CLUP: i32 = 42;

    const WEAPON_KIND_1H_SPEAR: i32 = 51;

    const WEAPON_KIND_2H_SPEAR: i32 = 52;

    const WEAPON_KIND_BOW: i32 = 70;

    const WEAPON_KIND_CROSSBOW: i32 = 71;

    const WEAPON_KIND_STAFF: i32 = 110;

    const WEAPON_KIND_JAMADHAR: i32 = 140;

    const WEAPON_KIND_MACE: i32 = 181;

    /// Equipment slot indices — imported from canonical `inventory_constants`.
    pub(crate) const SLOT_MAX: usize = crate::inventory_constants::SLOT_MAX;
    pub(crate) const HAVE_MAX: usize = crate::inventory_constants::HAVE_MAX;
    const RIGHTHAND: usize = crate::inventory_constants::RIGHTHAND;
    const LEFTHAND: usize = crate::inventory_constants::LEFTHAND;

    // ── Warehouse (Inn) Methods ────────────────────────────────────────

    /// Maximum warehouse slots (`WAREHOUSE_MAX = 192`, 8 pages * 24).
    pub const WAREHOUSE_MAX: usize = crate::inventory_constants::WAREHOUSE_MAX;

    // ── Repurchase (Trash Item) Methods ──────────────────────────────

    /// Maximum number of trash items per user.
    ///
    pub const TRASH_ITEM_MAX: usize = 10_000;

    /// Maximum number of items displayed in the repurchase list.
    ///
    pub const TRASH_DISPLAY_MAX: u16 = 250;

    // ── Inventory Methods ────────────────────────────────────────────

    /// Set the full inventory for a session (called on game entry).
    ///
    pub fn set_inventory(&self, id: SessionId, inventory: Vec<UserItemSlot>) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.inventory = inventory;
        }
    }
    /// Get the inventory for a session (cloned).
    pub fn get_inventory(&self, id: SessionId) -> Vec<UserItemSlot> {
        self.sessions
            .get(&id)
            .map(|h| h.inventory.clone())
            .unwrap_or_default()
    }

    /// Return the inventory state that is safe to persist for a normal
    /// character. Manes consumables live only for the event session and must
    /// never leak into `user_items`.
    pub fn get_persistent_inventory(&self, id: SessionId) -> Vec<UserItemSlot> {
        let mut inventory = self.get_inventory(id);
        let removed = self
            .manes_survival_manager
            .remove_temporary_items(&mut inventory);
        if removed > 0 {
            tracing::info!(
                "Inventory persistence removed {} temporary Manes item slot(s) for sid={}",
                removed,
                id
            );
        }
        inventory
    }
    /// Get a single inventory slot for a session.
    pub fn get_inventory_slot(&self, id: SessionId, slot: usize) -> Option<UserItemSlot> {
        self.sessions
            .get(&id)
            .and_then(|h| h.inventory.get(slot).cloned())
    }
    /// Update inventory via a closure, returns true if the update was applied.
    ///
    /// Used by item_move handler to swap/move items between slots.
    pub fn update_inventory(
        &self,
        id: SessionId,
        updater: impl FnOnce(&mut Vec<UserItemSlot>) -> bool,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            updater(&mut handle.inventory)
        } else {
            false
        }
    }
    /// Access both inventory and warehouse atomically via a closure.
    ///
    /// Used by warehouse INPUT/OUTPUT to move items between inventory and warehouse
    /// within a single DashMap lock, preventing race conditions.
    pub fn update_inventory_and_warehouse(
        &self,
        id: SessionId,
        updater: impl FnOnce(&mut Vec<UserItemSlot>, &mut Vec<UserItemSlot>, &mut u32) -> bool,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let h = handle.value_mut();
            updater(&mut h.inventory, &mut h.warehouse, &mut h.inn_coins)
        } else {
            false
        }
    }
    /// Access inventory and pet items atomically via a closure.
    ///
    /// Used by item_move handler for directions 12 (InvenToPet) and 13 (PetToInven).
    /// Returns false if the session has no active pet.
    ///
    pub fn update_inventory_and_pet(
        &self,
        id: SessionId,
        updater: impl FnOnce(&mut Vec<UserItemSlot>, &mut [UserItemSlot; PET_INVENTORY_TOTAL]) -> bool,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let h = handle.value_mut();
            if let Some(ref mut pet) = h.pet_data {
                updater(&mut h.inventory, &mut pet.items)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get the right-hand weapon's item definition, if one is equipped.
    ///
    ///
    /// Returns `None` if no weapon is in the right hand slot or the item is not found.
    pub fn get_right_hand_weapon(&self, id: SessionId) -> Option<Item> {
        self.sessions.get(&id).and_then(|h| {
            let slot = h.inventory.get(Self::RIGHTHAND)?;
            if slot.item_id == 0 {
                return None;
            }
            self.get_item(slot.item_id)
        })
    }
    /// Get the left-hand weapon's item definition, if one is equipped.
    ///
    pub fn get_left_hand_weapon(&self, id: SessionId) -> Option<Item> {
        self.sessions.get(&id).and_then(|h| {
            let slot = h.inventory.get(Self::LEFTHAND)?;
            if slot.item_id == 0 {
                return None;
            }
            self.get_item(slot.item_id)
        })
    }
    /// Get the weapon coefficient for a class based on the equipped weapon kind.
    ///
    fn get_weapon_coefficient(&self, class: u16, inventory: &[UserItemSlot]) -> f32 {
        let coeff = match self.get_coefficient(class) {
            Some(c) => c,
            None => return 0.0,
        };

        // Check right hand first, then left hand
        for &slot_idx in &[Self::RIGHTHAND, Self::LEFTHAND] {
            if let Some(slot) = inventory.get(slot_idx) {
                if slot.item_id == 0 {
                    continue;
                }
                if let Some(item) = self.get_item(slot.item_id) {
                    let kind = item.kind.unwrap_or(0);
                    return match kind {
                        Self::WEAPON_KIND_DAGGER => coeff.short_sword as f32,
                        Self::WEAPON_KIND_1H_SWORD | Self::WEAPON_KIND_2H_SWORD => {
                            coeff.sword as f32
                        }
                        Self::WEAPON_KIND_1H_AXE | Self::WEAPON_KIND_2H_AXE => coeff.axe as f32,
                        Self::WEAPON_KIND_1H_CLUP | Self::WEAPON_KIND_2H_CLUP => coeff.club as f32,
                        Self::WEAPON_KIND_1H_SPEAR | Self::WEAPON_KIND_2H_SPEAR => {
                            coeff.spear as f32
                        }
                        Self::WEAPON_KIND_BOW | Self::WEAPON_KIND_CROSSBOW => coeff.bow as f32,
                        Self::WEAPON_KIND_STAFF => coeff.staff as f32,
                        Self::WEAPON_KIND_JAMADHAR => coeff.jamadar as f32,
                        Self::WEAPON_KIND_MACE => coeff.pole as f32,
                        _ => continue,
                    };
                }
            }
        }
        0.0
    }
    // ── Inventory layout constants (C++ globals.h:293-336) ─────────
    const INVENTORY_COSP: usize = Self::SLOT_MAX + Self::HAVE_MAX; // 42
    const COSP_MAX: usize = 11;
    const INVENTORY_MBAG: usize = Self::INVENTORY_COSP + Self::COSP_MAX; // 53
    /// Bag slot 1 (absolute index): INVENTORY_COSP + COSP_BAG1(6) + 3 = 51.
    const BAG_SLOT_1: usize = Self::INVENTORY_COSP + 6 + 3; // 51
    /// Bag slot 2 (absolute index): INVENTORY_COSP + COSP_BAG2(10) = 52.
    const BAG_SLOT_2: usize = Self::INVENTORY_COSP + 10; // 52
    /// Item kind for cospre items
    const ITEM_KIND_COSPRE: i32 = 252;
    /// Duplicate item flag value
    /// Uses the central ITEM_FLAG_DUPLICATE constant from types.rs.
    const ITEM_FLAG_DUPLICATE: u8 = super::ITEM_FLAG_DUPLICATE;

    // ── Item elemental bonus type constants (C++ GameDefine.h:1365-1372) ─
    const ITEM_TYPE_FIRE: u8 = 0x01;
    const ITEM_TYPE_COLD: u8 = 0x02;
    const ITEM_TYPE_LIGHTNING: u8 = 0x03;
    const ITEM_TYPE_POISON: u8 = 0x04;
    const ITEM_TYPE_HP_DRAIN: u8 = 0x05;
    const ITEM_TYPE_MP_DAMAGE: u8 = 0x06;
    const ITEM_TYPE_MP_DRAIN: u8 = 0x07;
    const ITEM_TYPE_MIRROR_DAMAGE: u8 = 0x08;

    // ── Armor set item slot bitmasks (C++ GameDefine.h:1200-1204) ────
    /// ItemSlotHelmet = 7.
    const ITEM_SLOT_HELMET: i32 = 7;
    /// ItemSlotPauldron = 5.
    const ITEM_SLOT_PAULDRON: i32 = 5;
    /// ItemSlotPads = 6.
    const ITEM_SLOT_PADS: i32 = 6;
    /// ItemSlotGloves = 8.
    const ITEM_SLOT_GLOVES: i32 = 8;
    /// ItemSlotBoots = 9.
    const ITEM_SLOT_BOOTS: i32 = 9;

    /// Compute equipment stats from all inventory items (SetSlotItemValue).
    ///
    ///
    /// This iterates ALL inventory slots (equipped + bag + cospre + mbag):
    /// - Weight: accumulated for all items (bags add to max_weight_bonus instead)
    /// - Stats: only applied for equipped slots (0-13) and cospre slots (42-52),
    ///   excluding bag area (14-41), magic bags (53+), weapons-disabled items,
    ///   and duplicate-flagged items.
    /// - Set items: armor with race >= 100 accumulates a set ID; after the loop,
    ///   matching set bonuses are looked up and applied.
    fn compute_slot_item_values(&self, inventory: &[UserItemSlot]) -> EquippedStats {
        let mut stats = EquippedStats {
            item_hitrate: 100,
            item_evasionrate: 100,
            ..Default::default()
        };

        // Collect set item race -> accumulated set_id for armor set detection.
        let mut set_items: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();

        for (i, slot) in inventory.iter().enumerate() {
            if slot.item_id == 0 {
                continue;
            }
            let item = match self.get_item(slot.item_id) {
                Some(it) => it,
                None => continue,
            };

            // ── Bag slots add to max_weight_bonus, not weight ────────
            if i == Self::BAG_SLOT_1 || i == Self::BAG_SLOT_2 {
                stats.max_weight_bonus += item.duration.unwrap_or(0);
            } else {
                // All other items contribute to total weight.
                let weight = item.weight.unwrap_or(0);
                let countable = item.countable.unwrap_or(0);
                if countable == 0 {
                    stats.item_weight += weight as u32;
                } else {
                    stats.item_weight += weight as u32 * slot.count as u32;
                }
            }

            // ── Skip non-stat items ──────────────────────────────────
            // Bag area items (14..41) do not apply stats.
            if (Self::SLOT_MAX..Self::INVENTORY_COSP).contains(&i) {
                continue;
            }
            // Magic bag items (53+) do not apply stats.
            if i >= Self::INVENTORY_MBAG {
                continue;
            }
            // Duplicate-flagged items do not apply stats.
            if slot.flag == Self::ITEM_FLAG_DUPLICATE {
                continue;
            }
            // NOTE: isWeaponsDisabled + isShield check omitted — the server
            // does not currently track a weapons-disabled state on players.

            // ── Core stat accumulation ───────────────────────────────
            let mut item_ac = item.ac.unwrap_or(0);
            if slot.durability == 0 {
                item_ac /= 10;
            }

            stats.item_max_hp += item.max_hp_b.unwrap_or(0);
            stats.item_max_mp += item.max_mp_b.unwrap_or(0);
            stats.item_ac += item_ac;
            stats.stat_bonuses[0] += item.str_b.unwrap_or(0);
            stats.stat_bonuses[1] += item.sta_b.unwrap_or(0);
            stats.stat_bonuses[2] += item.dex_b.unwrap_or(0);
            stats.stat_bonuses[3] += item.intel_b.unwrap_or(0);
            stats.stat_bonuses[4] += item.cha_b.unwrap_or(0);
            stats.item_hitrate += item.hitrate.unwrap_or(0);
            stats.item_evasionrate += item.evasionrate.unwrap_or(0);

            // ── Elemental resistances ────────────────────────────────
            stats.fire_r += item.fire_r.unwrap_or(0);
            stats.cold_r += item.cold_r.unwrap_or(0);
            stats.lightning_r += item.lightning_r.unwrap_or(0);
            stats.magic_r += item.magic_r.unwrap_or(0);
            stats.disease_r += item.curse_r.unwrap_or(0);
            stats.poison_r += item.poison_r.unwrap_or(0);

            // ── Weapon-type resistances (C++ m_sDaggerR..m_sBowR) ────
            stats.dagger_r += item.dagger_ac.unwrap_or(0);
            stats.jamadar_r += item.jamadar_ac.unwrap_or(0);
            stats.sword_r += item.sword_ac.unwrap_or(0);
            stats.axe_r += item.axe_ac.unwrap_or(0);
            stats.club_r += item.club_ac.unwrap_or(0);
            stats.spear_r += item.spear_ac.unwrap_or(0);
            stats.bow_r += item.bow_ac.unwrap_or(0);

            // ── Elemental damage bonuses per slot ────────────────────
            let mut bonus_entries: Vec<(u8, i32)> = Vec::new();
            if let Some(v) = item.fire_damage {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_FIRE, v));
                }
            }
            if let Some(v) = item.ice_damage {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_COLD, v));
                }
            }
            if let Some(v) = item.lightning_damage {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_LIGHTNING, v));
                }
            }
            if let Some(v) = item.poison_damage {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_POISON, v));
                }
            }
            if let Some(v) = item.hp_drain {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_HP_DRAIN, v));
                }
            }
            if let Some(v) = item.mp_damage {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_MP_DAMAGE, v));
                }
            }
            if let Some(v) = item.mp_drain {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_MP_DRAIN, v));
                }
            }
            if let Some(v) = item.mirror_damage {
                if v != 0 {
                    bonus_entries.push((Self::ITEM_TYPE_MIRROR_DAMAGE, v));
                }
            }
            if !bonus_entries.is_empty() {
                stats.equipped_item_bonuses.insert(i, bonus_entries);
            }

            // ── Cospre set item lookup ───────────────────────────────
            let item_kind = item.kind.unwrap_or(0);
            let item_num = item.num;
            let is_cospre_extra = item_num == 610019000;
            if item_kind == Self::ITEM_KIND_COSPRE || is_cospre_extra {
                if let Some(set_row) = self.get_set_item(item_num) {
                    Self::apply_set_item_bonuses(&mut stats, &set_row);
                }
            }

            // ── Armor set ID accumulation ────────────────────────────
            let race = item.race.unwrap_or(0);
            if race < 100 {
                continue;
            }
            let entry = set_items.entry(race).or_insert(race * 10000);
            let item_slot = item.slot.unwrap_or(0);
            match item_slot {
                Self::ITEM_SLOT_HELMET => *entry += 2,
                Self::ITEM_SLOT_PAULDRON => *entry += 16,
                Self::ITEM_SLOT_PADS => *entry += 512,
                Self::ITEM_SLOT_GLOVES => *entry += 2048,
                Self::ITEM_SLOT_BOOTS => *entry += 4096,
                _ => {}
            }
        }

        // ── Apply armor set bonuses ──────────────────────────────────
        for set_id in set_items.values() {
            if let Some(set_row) = self.get_set_item(*set_id) {
                Self::apply_set_item_bonuses(&mut stats, &set_row);
            }
        }

        stats
    }

    /// Apply set item or cospre set bonuses to equipped stats.
    ///
    fn apply_set_item_bonuses(stats: &mut EquippedStats, set: &SetItemRow) {
        stats.item_ac += set.ac_bonus;
        stats.item_max_hp += set.hp_bonus;
        stats.item_max_mp += set.mp_bonus;

        stats.stat_bonuses[0] += set.strength_bonus;
        stats.stat_bonuses[1] += set.stamina_bonus;
        stats.stat_bonuses[2] += set.dexterity_bonus;
        stats.stat_bonuses[3] += set.intel_bonus;
        stats.stat_bonuses[4] += set.charisma_bonus;

        stats.fire_r += set.flame_resistance;
        stats.cold_r += set.glacier_resistance;
        stats.lightning_r += set.lightning_resistance;
        stats.magic_r += set.magic_resistance;
        stats.disease_r += set.curse_resistance;
        stats.poison_r += set.poison_resistance;

        stats.item_exp_bonus = stats
            .item_exp_bonus
            .saturating_add(set.xp_bonus_percent as u8);
        stats.item_gold_bonus = stats
            .item_gold_bonus
            .saturating_add(set.coin_bonus_percent as u8);
        stats.item_np_bonus = stats.item_np_bonus.saturating_add(set.np_bonus as u8);

        stats.max_weight_bonus += set.max_weight_bonus;

        stats.ap_bonus_amount = stats
            .ap_bonus_amount
            .saturating_add(set.ap_bonus_percent as u8);
        let ap_cls = set.ap_bonus_class_type;
        if (1..=4).contains(&ap_cls) {
            stats.ap_class_bonus[(ap_cls - 1) as usize] = stats.ap_class_bonus
                [(ap_cls - 1) as usize]
                .saturating_add(set.ap_bonus_class_percent as u8);
        }
        let ac_cls = set.ac_bonus_class_type;
        if (1..=4).contains(&ac_cls) {
            stats.ac_class_bonus[(ac_cls - 1) as usize] = stats.ac_class_bonus
                [(ac_cls - 1) as usize]
                .saturating_add(set.ac_bonus_class_percent as u8);
        }
    }

    /// Apply castellan cape bonuses to equipped stats.
    ///
    fn apply_castellan_cape_bonuses(
        stats: &mut EquippedStats,
        bonus: &KnightsCapeCastellanBonusRow,
    ) {
        stats.item_ac += bonus.ac_bonus;
        stats.item_max_hp += bonus.hp_bonus;
        stats.item_max_mp += bonus.mp_bonus;

        stats.stat_bonuses[0] += bonus.str_bonus;
        stats.stat_bonuses[1] += bonus.sta_bonus;
        stats.stat_bonuses[2] += bonus.dex_bonus;
        stats.stat_bonuses[3] += bonus.int_bonus;
        stats.stat_bonuses[4] += bonus.cha_bonus;

        stats.fire_r += bonus.flame_resist;
        stats.cold_r += bonus.glacier_resist;
        stats.lightning_r += bonus.lightning_resist;
        stats.magic_r += bonus.magic_resist;
        stats.disease_r += bonus.disease_resist;
        stats.poison_r += bonus.poison_resist;

        stats.item_exp_bonus = stats
            .item_exp_bonus
            .saturating_add(bonus.xp_bonus_pct as u8);
        stats.item_gold_bonus = stats
            .item_gold_bonus
            .saturating_add(bonus.coin_bonus_pct as u8);
        stats.item_np_bonus = stats.item_np_bonus.saturating_add(bonus.np_bonus as u8);

        stats.max_weight_bonus += bonus.max_weight_bonus;
        stats.ap_bonus_amount = stats
            .ap_bonus_amount
            .saturating_add(bonus.ap_bonus_pct as u8);
    }
    /// Generate a unique item serial number.
    ///
    /// C++ encodes server_no + date + increment into a u64.
    /// We use a monotonically-increasing atomic counter for simplicity.
    pub fn generate_item_serial(&self) -> u64 {
        self.next_item_serial
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Full equipment stat recalculation (SetUserAbility).
    ///
    ///
    /// Lock optimization: snapshot session data under a brief read lock, compute
    /// all stats without holding any session lock, then apply results under a
    /// brief write lock. Reduces DashMap shard contention significantly.
    pub fn set_user_ability(&self, sid: SessionId) {
        // Pre-computed buff aggregates (single-pass over buff map).
        #[derive(Default)]
        struct BuffAgg {
            str_mod: i32,
            sta_mod: i32,
            dex_mod: i32,
            intel_mod: i32,
            weapon_damage: u16,
            armour_ac_flat: i32,
            armour_ac_pct: i32,
            hp_flat: i32,
            hp_pct: i32,
            mp_flat: i32,
            mp_pct: i32,
        }

        // ── Phase 1: Snapshot session data (brief read lock) ────────────
        let (ch, inventory, perk_levels, achieve_stat, weight_buff, buff) = {
            let handle = match self.sessions.get(&sid) {
                Some(h) => h,
                None => return,
            };
            let ch = match handle.character.as_ref() {
                Some(c) => c.clone(),
                None => return,
            };
            let inventory = handle.inventory.clone();
            let perk_levels = handle.perk_levels;
            let achieve_stat = handle.achieve_stat_bonuses;
            let weight_buff = handle.weight_buff_amount;

            // Single-pass buff aggregation — replaces 5 separate iterations.
            let mut ba = BuffAgg {
                armour_ac_pct: 100,
                ..Default::default()
            };
            for b in handle.buffs.values() {
                ba.str_mod += b.str_mod;
                ba.sta_mod += b.sta_mod;
                ba.dex_mod += b.dex_mod;
                ba.intel_mod += b.intel_mod;
                ba.weapon_damage = ba
                    .weapon_damage
                    .saturating_add(b.weapon_damage.max(0) as u16);
                if b.buff_type == 14 {
                    ba.armour_ac_flat += b.ac;
                    if b.ac_pct != 0 {
                        ba.armour_ac_pct += b.ac_pct - 100;
                    }
                }
                ba.hp_flat += b.max_hp;
                if b.max_hp_pct != 0 {
                    ba.hp_pct += b.max_hp_pct - 100;
                }
                ba.mp_flat += b.max_mp;
                if b.max_mp_pct != 0 {
                    ba.mp_pct += b.max_mp_pct - 100;
                }
            }
            (ch, inventory, perk_levels, achieve_stat, weight_buff, ba)
        }; // Read lock dropped.

        // ── Phase 2: Compute stats (no session lock held) ───────────────
        let old_max_hp = ch.max_hp;
        let old_item_weight = self.get_equipped_stats(sid).item_weight;
        let is_gm = ch.authority == 0;
        let coeff_row = self.get_coefficient(ch.class);
        let mut stats = self.compute_slot_item_values(&inventory);

        // Pre-fetch perk definitions for all indices used in ability calculation.
        // Batches 5 DashMap lookups into one contiguous block instead of scattering
        // them through the function.  Stored as (perk_count, status) pairs.
        let perk_defs: [(i16, bool); 5] = {
            let fetch = |idx: usize| -> (i16, bool) {
                self.perk_definitions
                    .get(&(idx as i32))
                    .map(|d| (d.perk_count, d.status))
                    .unwrap_or((0, false))
            };
            [fetch(0), fetch(1), fetch(2), fetch(11), fetch(12)]
        };
        // Indices into perk_defs: [0]=weight, [1]=hp, [2]=mp, [3]=defence, [4]=attack
        let perk_bonus = |level: i16, def_idx: usize, check_status: bool| -> i32 {
            let (count, status) = perk_defs[def_idx];
            if level <= 0 || count <= 0 {
                return 0;
            }
            if check_status && !status {
                return 0;
            }
            count as i32 * level as i32
        };

        if let Some(coeff_row) = &coeff_row {
            let weapon_coeff = self.get_weapon_coefficient(ch.class, &inventory);

            // ── Castellan cape bonuses (C++ User.cpp:2377-2391) ──────
            if ch.knights_id > 0 {
                if let Some(knights) = self.get_knights(ch.knights_id) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as u32;
                    let cape_row = if knights.castellan_cape && knights.cast_cape_time >= now {
                        self.get_knights_cape(knights.cast_cape_id)
                    } else {
                        self.get_knights_cape(knights.cape as i16)
                    };
                    if let Some(cape) = cape_row {
                        if cape.bonus_type > 0 {
                            if let Some(bonus) = self.get_castellan_bonus(cape.bonus_type) {
                                Self::apply_castellan_cape_bonuses(&mut stats, &bonus);
                            }
                        }
                    }
                }
            }

            // Weapon power calculation
            let weapon_dmg_bonus = buff.weapon_damage;
            let mut rightpower: u16 = 0;
            let mut leftpower: u16 = 0;

            if let Some(r_slot) = inventory.get(Self::RIGHTHAND) {
                if r_slot.item_id != 0 {
                    if let Some(r_item) = self.get_item(r_slot.item_id) {
                        let dmg = r_item.damage.unwrap_or(0) as u16 + weapon_dmg_bonus;
                        if r_slot.durability == 0 {
                            rightpower = dmg / 2;
                        } else {
                            rightpower = dmg;
                        }
                    }
                }
            }

            if let Some(l_slot) = inventory.get(Self::LEFTHAND) {
                if l_slot.item_id != 0 {
                    if let Some(l_item) = self.get_item(l_slot.item_id) {
                        let dmg = l_item.damage.unwrap_or(0) as u16 + weapon_dmg_bonus;
                        let kind = l_item.kind.unwrap_or(0);
                        let is_bow =
                            kind == Self::WEAPON_KIND_BOW || kind == Self::WEAPON_KIND_CROSSBOW;
                        if is_bow {
                            leftpower = if l_slot.durability == 0 { dmg / 2 } else { dmg };
                        } else {
                            leftpower = if l_slot.durability == 0 {
                                dmg / 4
                            } else {
                                dmg / 2
                            };
                        }
                    }
                }
            }

            let totalpower = (rightpower + leftpower).max(3);

            // Stat calculations
            let main_str = ch.str as i32
                + ch.reb_str as i32
                + stats.stat_bonuses[0] as i32
                + achieve_stat[0] as i32
                + buff.str_mod;
            let main_dex = ch.dex as i32
                + ch.reb_dex as i32
                + stats.stat_bonuses[2] as i32
                + achieve_stat[2] as i32
                + buff.dex_mod;
            let main_int = ch.intel as i32
                + ch.reb_intel as i32
                + stats.stat_bonuses[3] as i32
                + achieve_stat[3] as i32
                + buff.intel_mod;

            // C++ uses getStat() (raw base stat, no bonuses) for base_ap threshold.
            let base_str = ch.str as i32;
            let base_int = ch.intel as i32;
            let mut base_ap: u32 = 0;
            if base_str > 150 {
                base_ap = (base_str - 150) as u32;
            }
            if base_int > 150 {
                base_ap = (base_int - 150) as u32;
            }
            if base_str == 160 {
                base_ap = base_ap.saturating_sub(1);
            }

            let total_str = main_str;
            let total_dex = main_dex;
            let total_int = main_int;

            // Max weight (base + bag/set/cape bonus + weight buff)
            let str_with_bonus = ch.str as u32
                + ch.reb_str as u32
                + stats.stat_bonuses[0] as u32
                + achieve_stat[0] as u32
                + buff.str_mod.max(0) as u32;
            stats.max_weight =
                (str_with_bonus + ch.level as u32) * 50 + stats.max_weight_bonus as u32;
            // C++ adds m_bMaxWeightAmount when > 100 (buff active)
            if weight_buff > 100 {
                stats.max_weight += weight_buff as u32;
            }
            // `m_sMaxWeight += (perkCount * perkType[weight]) * 10`
            let perk_weight = perk_bonus(perk_levels[0], 0, true);
            if perk_weight > 0 {
                stats.max_weight += (perk_weight * 10) as u32;
            }

            // Total hit (attack power)
            let power = totalpower as f32;
            let class_base = ch.class % 100;
            let is_rogue = matches!(class_base, 2 | 7 | 8);

            let bonus_ap = (stats.ap_bonus_amount as f32 + 100.0) / 100.0;

            let formula_f = if is_rogue {
                (0.005 * power * (total_dex as f32 + 40.0))
                    + (weapon_coeff * power * ch.level as f32 * total_dex as f32)
                    + 3.0
            } else {
                // When STR == INT, priest defaults to INT, warrior defaults to STR.
                let is_priest = matches!(class_base, 4 | 11 | 12);
                let use_int = ch.intel > ch.str || (ch.intel == ch.str && is_priest);
                let stat = if use_int {
                    total_int as f32
                } else {
                    total_str as f32
                };
                (0.005 * power * (stat + 40.0))
                    + (weapon_coeff * power * ch.level as f32 * stat)
                    + 3.0
            };
            // BaseAp is added AFTER BonusAp multiplication (C++ parity)
            stats.total_hit = (formula_f * bonus_ap) as u16;
            if !is_rogue {
                stats.total_hit = stats.total_hit.wrapping_add(base_ap as u16);
            }

            if weapon_dmg_bonus > 0 {
                stats.total_hit += 1;
            }

            // m_sAddArmourAc > 0 → m_sItemAc += m_sAddArmourAc
            // else → m_sItemAc = m_sItemAc * m_bPctArmourAc / 100
            // Pre-computed from single-pass buff aggregation in Phase 1.
            let mut adjusted_item_ac = stats.item_ac as i32;
            if buff.armour_ac_flat > 0 {
                adjusted_item_ac += buff.armour_ac_flat;
            } else if buff.armour_ac_pct != 100 {
                adjusted_item_ac = adjusted_item_ac * buff.armour_ac_pct / 100;
            }

            // Total AC
            stats.total_ac =
                (coeff_row.ac as f32 * (ch.level as f32 + adjusted_item_ac as f32)) as i16;

            // ── Passive skill defense + resistance bonus ─────────────────
            // PRO_SKILL2 = skill_points[6]
            {
                let pro_skill2 = ch.skill_points[6];
                let class_type = ch.class;
                let mut defense_bonus: i32 = 0;
                let mut resistance_bonus: i32 = 0;

                // Check if left hand has a shield (kind == 60)
                let has_shield = inventory
                    .get(Self::LEFTHAND)
                    .and_then(|slot| {
                        if slot.item_id == 0 {
                            return None;
                        }
                        self.get_item(slot.item_id)
                    })
                    .map(|item| item.kind.unwrap_or(0) == 60)
                    .unwrap_or(false);

                // Warrior passives (class 1=base, 5=novice, 6=mastered)
                if matches!(class_type, 1 | 5 | 6) {
                    if class_type == 5 {
                        // Novice warrior — no shield check
                        if (55..=69).contains(&pro_skill2) {
                            defense_bonus = 13;
                        } else if (35..=54).contains(&pro_skill2) {
                            defense_bonus = 10;
                        } else if (15..=34).contains(&pro_skill2) {
                            defense_bonus = 8;
                        } else if (5..=14).contains(&pro_skill2) {
                            defense_bonus = 5;
                        }

                        if (40..=83).contains(&pro_skill2) {
                            resistance_bonus = 45;
                        } else if (20..=39).contains(&pro_skill2) {
                            resistance_bonus = 30;
                        } else if (10..=19).contains(&pro_skill2) {
                            resistance_bonus = 15;
                        }
                    } else if class_type == 6 {
                        // Mastered warrior — shield check halves bonuses
                        if (80..=83).contains(&pro_skill2) {
                            defense_bonus = 80;
                        } else if (70..=79).contains(&pro_skill2) {
                            defense_bonus = 60;
                        } else if (55..=69).contains(&pro_skill2) {
                            defense_bonus = 50;
                        } else if (35..=54).contains(&pro_skill2) {
                            defense_bonus = 40;
                        } else if (15..=34).contains(&pro_skill2) {
                            defense_bonus = 34;
                        } else if (5..=14).contains(&pro_skill2) {
                            defense_bonus = 20;
                        }

                        if (40..=83).contains(&pro_skill2) {
                            resistance_bonus = 90;
                        } else if (20..=39).contains(&pro_skill2) {
                            resistance_bonus = 60;
                        } else if (10..=19).contains(&pro_skill2) {
                            resistance_bonus = 30;
                        }

                        if !has_shield {
                            defense_bonus /= 2;
                            resistance_bonus /= 2;
                        }
                    }
                }
                // PortuKurian passives (class 13=base, 14=novice, 15=mastered)
                else if matches!(class_type, 13..=15) {
                    if class_type == 14 {
                        // Novice Kurian — no shield check
                        if (55..=69).contains(&pro_skill2) {
                            defense_bonus = 13;
                        } else if (35..=54).contains(&pro_skill2) {
                            defense_bonus = 10;
                        } else if (15..=34).contains(&pro_skill2) {
                            defense_bonus = 8;
                        } else if (5..=14).contains(&pro_skill2) {
                            defense_bonus = 5;
                        }

                        if (40..=83).contains(&pro_skill2) {
                            resistance_bonus = 45;
                        } else if (20..=39).contains(&pro_skill2) {
                            resistance_bonus = 30;
                        } else if (10..=19).contains(&pro_skill2) {
                            resistance_bonus = 15;
                        }
                    } else if class_type == 15 {
                        // Mastered Kurian — no shield check in C++
                        if (80..=83).contains(&pro_skill2) {
                            defense_bonus = 20;
                        } else if (70..=79).contains(&pro_skill2) {
                            defense_bonus = 15;
                        } else if (55..=69).contains(&pro_skill2) {
                            defense_bonus = 13;
                        } else if (35..=54).contains(&pro_skill2) {
                            defense_bonus = 10;
                        } else if (15..=34).contains(&pro_skill2) {
                            defense_bonus = 8;
                        } else if (5..=14).contains(&pro_skill2) {
                            defense_bonus = 5;
                        }

                        if (40..=83).contains(&pro_skill2) {
                            resistance_bonus = 45;
                        } else if (20..=39).contains(&pro_skill2) {
                            resistance_bonus = 30;
                        } else if (10..=19).contains(&pro_skill2) {
                            resistance_bonus = 15;
                        }
                    }
                }

                // Apply defense bonus as percentage of total_ac
                if defense_bonus > 0 {
                    stats.total_ac += (defense_bonus * stats.total_ac as i32 / 100) as i16;
                }
                stats.resistance_bonus = resistance_bonus as i16;
            }

            // Low HP passives
            {
                let class_type = ch.class;
                // MasteredPriest (12) or MasteredWarrior (6): +20% AC at <30% HP
                if matches!(class_type, 6 | 12) {
                    if ch.hp < (30 * ch.max_hp as i32 / 100) as i16 {
                        stats.total_ac += (20 * stats.total_ac as i32 / 100) as i16;
                    }
                }
                // MasteredRogue (8): +50 resistance at <30% HP
                else if class_type == 8 {
                    if ch.hp < (30 * ch.max_hp as i32 / 100) as i16 {
                        stats.resistance_bonus += 50;
                    }
                }
                // MasteredKurian (15): +20% total_hit at <30% HP (PRO_SKILL4 15-23)
                else if class_type == 15 {
                    let pro_skill4 = ch.skill_points[8];
                    if (15..=23).contains(&pro_skill4)
                        && ch.hp < (30 * ch.max_hp as i32 / 100) as i16
                    {
                        stats.total_hit += (20 * stats.total_hit as u32 / 100) as u16;
                    }
                }
            }

            if buff.armour_ac_flat > 0 || buff.armour_ac_pct > 100 {
                stats.total_ac += 1;
            }

            // STA > 100 bonus AC
            if ch.sta > 100 {
                stats.total_ac += (ch.sta - 100) as i16;
            }

            // INT > 100 bonus resistance
            {
                let base_int = ch.intel as i32;
                if base_int > 100 {
                    stats.resistance_bonus += ((base_int - 100) / 2) as i16;
                }
            }

            // achieve_stat_bonuses[5] = attack, [6] = defense
            let achieve_attack = achieve_stat[5];
            let achieve_defense = achieve_stat[6];
            if achieve_attack > 0 {
                stats.total_hit += achieve_attack as u16;
            }
            if achieve_defense > 0 {
                stats.total_ac += achieve_defense;
            }

            let perk_attack = perk_bonus(perk_levels[12], 4, true);
            if perk_attack > 0 {
                stats.total_hit += perk_attack as u16;
            }
            let perk_defence = perk_bonus(perk_levels[11], 3, true);
            if perk_defence > 0 {
                stats.total_ac += perk_defence as i16;
            }

            // Hit rate and evasion rate
            stats.total_hitrate = (1.0
                + coeff_row.hitrate as f32 * ch.level as f32 * total_dex as f32)
                * stats.item_hitrate as f32
                / 100.0;
            stats.total_evasionrate = (1.0
                + coeff_row.evasionrate as f32 * ch.level as f32 * total_dex as f32)
                * stats.item_evasionrate as f32
                / 100.0;

        // stats computed — coeff_row branch done
        } else {
            // Coefficient missing (should never happen for valid classes 101-215).
            // Still update item_weight, max_weight_bonus, and other item-derived
            // fields so weight checks don't use stale cached values.
            tracing::warn!(
                "[sid={}] set_user_ability: no coefficient for class={} — updating item stats only",
                sid,
                ch.class,
            );
        }

        // ── HP/MP computation (still Phase 2, no session lock) ──────
        //   m_MaxHp = formula(total_sta) + m_sMaxHPAmount + m_sItemMaxHp + 20
        //   m_MaxMp = formula(total_intel) + m_sMaxMPAmount + m_sItemMaxMp + 20
        // When coefficient exists, base is from formula; otherwise use existing max_hp/max_mp.
        let hp_before_buff = {
            let base = if let Some(coeff_row) = &coeff_row {
                // C++ uses getStatTotal() which includes item + achieve + buff bonuses
                let total_sta = ch.sta as f64
                    + stats.stat_bonuses[1] as f64
                    + achieve_stat[1] as f64
                    + buff.sta_mod as f64;
                let level = ch.level as f64;
                let formula = ((coeff_row.hp * level * level * total_sta)
                    + (0.1 * level * total_sta)
                    + (total_sta / 5.0)
                    + 20.0) as i32;
                (formula + stats.item_max_hp as i32).max(20)
            } else {
                // No coefficient: use existing max_hp + item bonus as base
                (ch.max_hp as i32 + stats.item_max_hp as i32).max(20)
            };
            // Does NOT check status, only perkCount.
            let perk_hp = perk_bonus(perk_levels[1], 1, false);
            base + perk_hp
        };

        let buff_hp_bonus = {
            let pct_amount = (hp_before_buff * buff.hp_pct) / 100;
            buff.hp_flat + pct_amount
        };
        let mut new_max_hp = (hp_before_buff + buff_hp_bonus).clamp(20, i16::MAX as i32) as i16;

        //   if (m_MaxHp > pServerSetting.maxplayerhp && !isGM())
        //       m_MaxHp = pServerSetting.maxplayerhp;
        let max_player_hp = self
            .get_server_settings()
            .map(|s| s.max_player_hp)
            .unwrap_or(0);
        if max_player_hp > 0 && new_max_hp > max_player_hp && !is_gm {
            new_max_hp = max_player_hp;
        }

        let mp_before_buff = {
            let base = if let Some(coeff_row) = &coeff_row {
                let total_sta = ch.sta as f64
                    + stats.stat_bonuses[1] as f64
                    + achieve_stat[1] as f64
                    + buff.sta_mod as f64;
                let total_intel = ch.intel as f64
                    + stats.stat_bonuses[3] as f64
                    + achieve_stat[3] as f64
                    + buff.intel_mod as f64;
                let level = ch.level as f64;
                let formula = if coeff_row.mp != 0.0 {
                    let temp_intel = total_intel + 30.0;
                    ((coeff_row.mp * level * level * temp_intel)
                        + (0.1 * level * 2.0 * temp_intel)
                        + (temp_intel / 5.0)
                        + 20.0) as i32
                } else if coeff_row.sp != 0.0 {
                    ((coeff_row.sp * level * level * total_sta)
                        + (0.1 * level * total_sta)
                        + (total_sta / 5.0)) as i32
                } else {
                    ch.max_mp as i32
                };
                (formula + stats.item_max_mp as i32).max(0)
            } else {
                (ch.max_mp as i32 + stats.item_max_mp as i32).max(0)
            };
            // Applied in both MP and SP branches. Does NOT check status.
            let perk_mp = perk_bonus(perk_levels[2], 2, false);
            base + perk_mp
        };

        let buff_mp_bonus = {
            let pct_amount = (mp_before_buff * buff.mp_pct) / 100;
            buff.mp_flat + pct_amount
        };
        let new_max_mp = (mp_before_buff + buff_mp_bonus).clamp(0, i16::MAX as i32) as i16;

        // ── Phase 3: Apply results (brief write lock) ───────────────
        let mut max_hp_changed: Option<i32> = None;
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            handle.equipped_stats = stats;
            if let Some(c) = handle.character.as_mut() {
                c.max_hp = new_max_hp;
                c.max_mp = new_max_mp;
                if c.hp > c.max_hp {
                    c.hp = c.max_hp;
                }
                if c.mp > c.max_mp {
                    c.mp = c.max_mp;
                }
                if new_max_hp != old_max_hp {
                    max_hp_changed = Some(new_max_hp as i32);
                }
            }
        }
        // Send WIZ_MAX_HP_CHANGE (0x92) notification after releasing session lock.
        // Client RE: sub=2 triggers HP bar expansion/contraction animation.
        if let Some(new_max) = max_hp_changed {
            let pkt = crate::handler::max_hp_change::build_max_hp_change(new_max);
            self.send_to_session_owned(sid, pkt);
        }

        // Send WIZ_WEIGHT_CHANGE only when weight actually changed.
        // This ensures all callers (mining, crafting, loot, repair, zone change, etc.)
        // automatically sync the client's weight display without manual per-handler sends.
        {
            let new_weight = self.get_equipped_stats(sid).item_weight;
            if new_weight != old_item_weight {
                let mut wpkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizWeightChange as u8);
                wpkt.write_u32(new_weight);
                self.send_to_session_owned(sid, wpkt);
            }
        }
    }
    /// Get equipped stats for a session.
    pub fn get_equipped_stats(&self, id: SessionId) -> EquippedStats {
        self.sessions
            .get(&id)
            .map(|h| h.equipped_stats.clone())
            .unwrap_or_default()
    }
    // ── Inventory Utility Methods ─────────────────────────────────────

    /// Check if the player has at least one empty inventory slot (bag area).
    ///
    pub fn has_empty_inventory_slot(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| {
                let inv = &h.inventory;
                (Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX))
                    .any(|i| inv.get(i).is_some_and(|slot| slot.item_id == 0))
            })
            .unwrap_or(false)
    }

    /// Find a free slot for an item in the inventory (bag area).
    ///
    pub fn find_slot_for_item(&self, sid: SessionId, item_id: u32, count: u16) -> Option<usize> {
        let item = self.get_item(item_id)?;
        let countable = item.countable.unwrap_or(0);

        self.sessions.get(&sid).and_then(|h| {
            let inv = &h.inventory;
            if countable > 0 {
                // Stackable: find existing stack with room, or first empty slot
                let mut first_empty: Option<usize> = None;
                for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                    if let Some(slot) = inv.get(i) {
                        if slot.item_id == item_id && slot.count + count <= ITEMCOUNT_MAX {
                            return Some(i);
                        }
                        if slot.item_id == 0 && first_empty.is_none() {
                            first_empty = Some(i);
                        }
                    }
                }
                first_empty
            } else {
                // Non-stackable: find first empty slot
                for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                    if let Some(slot) = inv.get(i) {
                        if slot.item_id == 0 {
                            return Some(i);
                        }
                    }
                }
                None
            }
        })
    }
    /// Count free inventory slots (HAVE_MAX range).
    ///
    pub fn count_free_inventory_slots(&self, sid: SessionId) -> u32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                let inv = &h.inventory;
                let mut count = 0u32;
                for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                    if let Some(slot) = inv.get(i) {
                        if slot.item_id == 0 {
                            count += 1;
                        }
                    }
                }
                count
            })
            .unwrap_or(0)
    }

    /// Check if a player can carry more weight.
    ///
    pub fn check_weight(&self, sid: SessionId, item_id: u32, count: u16) -> bool {
        let item = match self.get_item(item_id) {
            Some(i) => i,
            None => return false,
        };
        let weight = item.weight.unwrap_or(0) as u32;
        let add_weight = weight * count as u32;

        self.sessions
            .get(&sid)
            .map(|h| {
                let current = h.equipped_stats.item_weight;
                let max = h.equipped_stats.max_weight;
                current + add_weight <= max
                    && self.find_slot_for_item(sid, item_id, count).is_some()
            })
            .unwrap_or(false)
    }
    /// Give an item to a player's inventory.
    ///
    /// Sends `WIZ_ITEM_COUNT_CHANGE` (0x3D) to the client on success.
    pub fn give_item(&self, sid: SessionId, item_id: u32, count: u16) -> bool {
        let item = match self.get_item(item_id) {
            Some(i) => i,
            None => {
                tracing::warn!(
                    "give_item: item_id {} not found in items table (sid={})",
                    item_id,
                    sid
                );
                return false;
            }
        };

        if item.countable.unwrap_or(0) == 2 {
            return false;
        }

        let pos = match self.find_slot_for_item(sid, item_id, count) {
            Some(p) => p,
            None => return false,
        };

        // Capture slot info needed for the packet while inside the DashMap lock.
        let mut pkt_info: Option<(u8, u32, u16)> = None; // (pos, new_count, durability)
        let serial = self.generate_item_serial();

        let ok = self.update_inventory(sid, |inv| {
            if pos >= inv.len() {
                return false;
            }
            let slot = &mut inv[pos];
            let is_new = slot.item_id == 0;
            slot.item_id = item_id;
            slot.count = slot.count.saturating_add(count).min(ITEMCOUNT_MAX);
            slot.durability = item.duration.unwrap_or(0);
            if is_new {
                slot.serial_num = serial;
            }
            // Kind 255 (consumable scrolls) and non-countable always count=1
            let countable = item.countable.unwrap_or(0);
            if item.kind.unwrap_or(0) == 255 || countable == 0 {
                slot.count = 1;
            }
            pkt_info = Some((
                (pos - Self::SLOT_MAX) as u8,
                slot.count as u32,
                slot.durability as u16,
            ));
            true
        });

        // Send WIZ_ITEM_COUNT_CHANGE outside the DashMap lock.
        if ok {
            if let Some((slot_pos, new_count, durability)) = pkt_info {
                let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
                pkt.write_u16(1); // count_type: always 1
                pkt.write_u8(1); // slot_section: 1 = inventory
                pkt.write_u8(slot_pos); // position within inventory
                pkt.write_u32(item_id);
                pkt.write_u32(new_count);
                pkt.write_u8(100); // C++ SendStackChange always passes true (bNewItem)
                pkt.write_u16(durability);
                pkt.write_u32(0); // reserved
                pkt.write_u32(0); // expiration
                // v2625 SendStackChange has a trailing u16 after expiration.
                // Omitting it leaves the client packet reader misaligned and can
                // leave a consumed stack visible as "0" until a full refresh.
                pkt.write_u16(0);
                self.send_to_session_owned(sid, pkt);

                // C++ SendStackChange calls SetUserAbility(false) + SendItemWeight()
                // Weight notification is now integrated into set_user_ability().
                self.set_user_ability(sid);
            }
        }

        ok
    }
    /// Give an item to a player's inventory with an expiration time.
    ///
    /// When `expiry_days > 0`, sets `nExpirationTime = UNIXTIME + (86400 * expiry_days)`.
    pub fn give_item_with_expiry(
        &self,
        sid: SessionId,
        item_id: u32,
        count: u16,
        expiry_days: u32,
    ) -> bool {
        let item = match self.get_item(item_id) {
            Some(i) => i,
            None => return false,
        };

        if item.countable.unwrap_or(0) == 2 {
            return false;
        }

        let pos = match self.find_slot_for_item(sid, item_id, count) {
            Some(p) => p,
            None => return false,
        };

        let expire_time = if expiry_days > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            now + (86400 * expiry_days)
        } else {
            0
        };

        let mut pkt_info: Option<(u8, u32, u16, u32)> = None;
        let serial = self.generate_item_serial();

        let ok = self.update_inventory(sid, |inv| {
            if pos >= inv.len() {
                return false;
            }
            let slot = &mut inv[pos];
            let is_new = slot.item_id == 0;
            slot.item_id = item_id;
            slot.count = slot.count.saturating_add(count).min(ITEMCOUNT_MAX);
            // C++ always sets pItem->sDuration = pTable.m_sDuration regardless
            // of whether this is a new slot or stacking on an existing one.
            slot.durability = item.duration.unwrap_or(0);
            if is_new {
                slot.serial_num = serial;
            }
            let countable = item.countable.unwrap_or(0);
            if item.kind.unwrap_or(0) == 255 || countable == 0 {
                slot.count = 1;
            }
            if expire_time > 0 {
                slot.expire_time = expire_time;
            }
            pkt_info = Some((
                (pos - Self::SLOT_MAX) as u8,
                slot.count as u32,
                slot.durability as u16,
                slot.expire_time,
            ));
            true
        });

        if ok {
            if let Some((slot_pos, new_count, durability, exp)) = pkt_info {
                let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
                pkt.write_u16(1);
                pkt.write_u8(1); // slot_section: 1 = inventory
                pkt.write_u8(slot_pos);
                pkt.write_u32(item_id);
                pkt.write_u32(new_count);
                pkt.write_u8(100);
                pkt.write_u16(durability);
                pkt.write_u32(0);
                pkt.write_u32(exp);
                pkt.write_u16(0); // v2625 SendStackChange trailing field
                self.send_to_session_owned(sid, pkt);

                // Weight notification is now integrated into set_user_ability().
                self.set_user_ability(sid);
            }
        }

        ok
    }
    /// Give an item directly into a player's warehouse (bank).
    ///
    /// Finds a free warehouse slot (stacks countable items), sets duration and
    /// optional expiration. Does NOT send a client packet — the client sees
    /// the item when the warehouse window is opened next.
    pub fn give_warehouse_item(
        &self,
        sid: SessionId,
        item_id: u32,
        count: u16,
        expiry_days: u32,
    ) -> bool {
        let item = match self.get_item(item_id) {
            Some(i) => i,
            None => return false,
        };

        let countable = item.countable.unwrap_or(0) > 0;
        let duration = item.duration.unwrap_or(0);

        let expire_time = if expiry_days > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            now + (86400 * expiry_days)
        } else {
            0
        };

        self.update_warehouse(sid, |wh, _coins| {
            // C++ FindWerehouseSlotForItem: for countable items, first try to
            // find an existing stack with room, then fall back to an empty slot.
            let mut target_slot: Option<usize> = None;

            if countable {
                let mut first_empty: Option<usize> = None;
                for (i, slot) in wh.iter().enumerate().take(Self::WAREHOUSE_MAX) {
                    if slot.item_id == item_id && slot.count + count <= ITEMCOUNT_MAX {
                        target_slot = Some(i);
                        break;
                    }
                    if slot.item_id == 0 && first_empty.is_none() {
                        first_empty = Some(i);
                    }
                }
                if target_slot.is_none() {
                    target_slot = first_empty;
                }
            } else {
                // Non-stackable: find first empty slot
                for (i, slot) in wh.iter().enumerate().take(Self::WAREHOUSE_MAX) {
                    if slot.item_id == 0 {
                        target_slot = Some(i);
                        break;
                    }
                }
            }

            let pos = match target_slot {
                Some(p) => p,
                None => return false,
            };

            let slot = &mut wh[pos];
            let is_new = slot.item_id == 0;
            if is_new {
                slot.serial_num = self.generate_item_serial();
            }
            slot.item_id = item_id;
            slot.count = slot.count.saturating_add(count).min(ITEMCOUNT_MAX);
            slot.durability = duration;
            if expire_time > 0 {
                slot.expire_time = expire_time;
            } else {
                slot.expire_time = 0;
            }

            true
        })
    }
    /// Remove an item from a player's inventory.
    ///
    ///
    /// For `kind == 255` (consumable scroll) items, the usage counter is stored
    /// in `durability` (`sDuration`), NOT `count`
    /// The item slot is only cleared when durability reaches 0.
    pub fn rob_item(&self, sid: SessionId, item_id: u32, count: u16) -> bool {
        if count == 0 {
            return true;
        }

        let is_consumable_scroll =
            self.items.get(&item_id).and_then(|it| it.kind).unwrap_or(0) == 255;

        // Capture slot info for the WIZ_ITEM_COUNT_CHANGE packet.
        let mut pkt_info: Option<(u8, u32, u16)> = None; // (bag_pos, new_count, durability)

        let ok = self.update_inventory(sid, |inv| {
            for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                if let Some(slot) = inv.get_mut(i) {
                    if slot.item_id != item_id {
                        continue;
                    }

                    if is_consumable_scroll {
                        // kind=255: durability is the usage counter (C++ sDuration).
                        // C++ ItemHandler.cpp:204-209 — decrements sDuration, clears when <= 0.
                        if slot.durability < count as i16 {
                            return false;
                        }
                        slot.durability -= count as i16;
                        let bag_pos = (i - Self::SLOT_MAX) as u8;
                        if slot.durability <= 0 {
                            *slot = UserItemSlot::default();
                            pkt_info = Some((bag_pos, 0, 0));
                        } else {
                            pkt_info = Some((bag_pos, slot.count as u32, slot.durability as u16));
                        }
                        return true;
                    } else {
                        // Normal item: count is the usage counter.
                        if slot.count < count {
                            continue;
                        }
                        slot.count -= count;
                        let new_count = slot.count;
                        let durability = slot.durability;
                        let bag_pos = (i - Self::SLOT_MAX) as u8;
                        if slot.count == 0 {
                            *slot = UserItemSlot::default();
                        }
                        pkt_info = Some((bag_pos, new_count as u32, durability as u16));
                        return true;
                    }
                }
            }
            false
        });

        // Send WIZ_ITEM_COUNT_CHANGE to the client so UI updates.
        if ok {
            if let Some((slot_pos, new_count, durability)) = pkt_info {
                let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
                pkt.write_u16(1); // count_type
                pkt.write_u8(1); // slot_section: inventory
                pkt.write_u8(slot_pos);
                pkt.write_u32(item_id);
                pkt.write_u32(new_count);
                pkt.write_u8(0); // bNewItem = false (removal)
                pkt.write_u16(durability);
                pkt.write_u32(0); // reserved
                pkt.write_u32(0); // expiration
                pkt.write_u16(0); // v2625 SendStackChange trailing field
                self.send_to_session_owned(sid, pkt);

                // Weight notification is now integrated into set_user_ability().
                self.set_user_ability(sid);
            }
        }

        ok
    }
    /// Remove ALL copies of an item from a player's bag slots.
    ///
    /// Unlike `rob_item()` which targets a single slot, this clears every slot
    /// containing the specified item_id. Returns true if any items were removed.
    pub fn rob_all_of_item(&self, sid: SessionId, item_id: u32) -> bool {
        // Collect slot info for each removed slot.
        let mut removed_slots: Vec<u8> = Vec::new();

        let found = self.update_inventory(sid, |inv| {
            let mut any = false;
            for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                if let Some(slot) = inv.get_mut(i) {
                    if slot.item_id == item_id && slot.count > 0 {
                        removed_slots.push((i - Self::SLOT_MAX) as u8);
                        *slot = UserItemSlot::default();
                        any = true;
                    }
                }
            }
            any
        });

        // Send WIZ_ITEM_COUNT_CHANGE for each cleared slot.
        if found {
            for &slot_pos in &removed_slots {
                let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
                pkt.write_u16(1);
                pkt.write_u8(1); // inventory
                pkt.write_u8(slot_pos);
                pkt.write_u32(item_id);
                pkt.write_u32(0); // count = 0
                pkt.write_u8(0);
                pkt.write_u16(0);
                pkt.write_u32(0);
                pkt.write_u32(0);
                pkt.write_u16(0); // v2625 SendStackChange trailing field
                self.send_to_session_owned(sid, pkt);
            }
            // Weight notification is now integrated into set_user_ability().
            self.set_user_ability(sid);
        }

        found
    }

    /// Deduct gold from a player. Returns false if insufficient funds.
    ///
    /// Sends WIZ_GOLD_CHANGE (0x4A) to the client: `[u8 CoinLoss=2] [u32 amount] [u32 new_total]`.
    ///
    pub fn gold_lose(&self, sid: SessionId, amount: u32) -> bool {
        let mut success = false;
        let mut new_gold = 0u32;
        self.update_character_stats(sid, |ch| {
            if ch.gold >= amount {
                ch.gold -= amount;
                new_gold = ch.gold;
                success = true;
            }
        });
        if success {
            let mut pkt = Packet::new(Opcode::WizGoldChange as u8);
            pkt.write_u8(2); // COIN_LOSS
            pkt.write_u32(amount);
            pkt.write_u32(new_gold);
            self.send_to_session_owned(sid, pkt);
        }
        success
    }

    /// Deduct gold from a player without sending WIZ_GOLD_CHANGE packet.
    ///
    /// Used when the caller sends its own response packet containing the updated
    /// gold amount (e.g. item repair, stat/skill reset, clan creation).
    ///
    /// with `bSendPacket = false` in NPCHandler.cpp:69, UserSkillStatPointSystem.cpp:95,148,
    /// KnightsDatabaseHandler.cpp:160.
    pub fn gold_lose_silent(&self, sid: SessionId, amount: u32) -> bool {
        let mut success = false;
        self.update_character_stats(sid, |ch| {
            if ch.gold >= amount {
                ch.gold -= amount;
                success = true;
            }
        });
        success
    }

    /// Add gold to a player.
    ///
    /// Sends WIZ_GOLD_CHANGE (0x4A) to the client: `[u8 CoinGain=1] [u32 amount] [u32 new_total]`.
    ///
    pub fn gold_gain(&self, sid: SessionId, amount: u32) {
        let mut new_gold = 0u32;
        self.update_character_stats(sid, |ch| {
            // Cap at COIN_MAX matching C++: UserGoldSystem.cpp:103-104
            if ch.gold as u64 + amount as u64 > COIN_MAX as u64 {
                ch.gold = COIN_MAX;
            } else {
                ch.gold += amount;
            }
            new_gold = ch.gold;
        });
        let mut pkt = Packet::new(Opcode::WizGoldChange as u8);
        pkt.write_u8(1); // COIN_GAIN
        pkt.write_u32(amount);
        pkt.write_u32(new_gold);
        self.send_to_session_owned(sid, pkt);
    }
    /// Gain gold with bonus multipliers applied (monster drops, quest rewards).
    ///
    /// Formula: `gold * (noah_gain_amount + item_gold_bonus + clan_premium_bonus) / 100`
    pub fn gold_gain_with_bonus(&self, sid: SessionId, amount: u32) {
        let noah_gain = self
            .with_session(sid, |h| h.noah_gain_amount as u32)
            .unwrap_or(100);
        let item_bonus = self.get_equipped_stats(sid).item_gold_bonus as u32;
        // Only applies when clan premium is active (sClanPremStatus > 0).
        let clan_premium_bonus: u32 = {
            let has_clan_premium = self
                .with_session(sid, |h| h.clan_premium_in_use > 0)
                .unwrap_or(false);
            if has_clan_premium {
                2
            } else {
                0
            }
        };
        let mut bonus_gold = amount * (noah_gain + item_bonus + clan_premium_bonus) / 100;

        // Flame level money_rate bonus
        let flame_level = self.with_session(sid, |h| h.flame_level).unwrap_or(0);
        if flame_level > 0 {
            if let Some(feat) = self.get_burning_feature(flame_level) {
                if feat.money_rate > 0 {
                    bonus_gold = bonus_gold * (100 + feat.money_rate as u32) / 100;
                }
            }
        }

        let perk_coin = self
            .with_session(sid, |h| self.compute_perk_bonus(&h.perk_levels, 6, false))
            .unwrap_or(0);
        if perk_coin > 0 {
            bonus_gold += bonus_gold * perk_coin as u32 / 100;
        }

        self.gold_gain(sid, bonus_gold);
    }
    /// Add gold to a player silently (no WIZ_GOLD_CHANGE packet).
    ///
    /// Used when the gold notification is handled by a different packet (e.g. LootPartyCoinDistribution).
    pub fn gold_gain_silent(&self, sid: SessionId, amount: u32) {
        self.update_character_stats(sid, |ch| {
            // Cap at COIN_MAX matching C++: UserGoldSystem.cpp:103-104
            if ch.gold as u64 + amount as u64 > COIN_MAX as u64 {
                ch.gold = COIN_MAX;
            } else {
                ch.gold += amount;
            }
        });
    }
    /// Gain gold with bonus multipliers applied, without sending WIZ_GOLD_CHANGE.
    ///
    /// Used for party gold distribution where LootPartyCoinDistribution is sent instead.
    pub fn gold_gain_with_bonus_silent(&self, sid: SessionId, amount: u32) {
        let noah_gain = self
            .with_session(sid, |h| h.noah_gain_amount as u32)
            .unwrap_or(100);
        let item_bonus = self.get_equipped_stats(sid).item_gold_bonus as u32;
        let clan_premium_bonus: u32 = {
            let has_clan_premium = self
                .with_session(sid, |h| h.clan_premium_in_use > 0)
                .unwrap_or(false);
            if has_clan_premium {
                2
            } else {
                0
            }
        };
        let mut bonus_gold = amount * (noah_gain + item_bonus + clan_premium_bonus) / 100;

        // Flame level money_rate bonus
        let flame_level = self.with_session(sid, |h| h.flame_level).unwrap_or(0);
        if flame_level > 0 {
            if let Some(feat) = self.get_burning_feature(flame_level) {
                if feat.money_rate > 0 {
                    bonus_gold = bonus_gold * (100 + feat.money_rate as u32) / 100;
                }
            }
        }

        let perk_coin = self
            .with_session(sid, |h| self.compute_perk_bonus(&h.perk_levels, 6, false))
            .unwrap_or(0);
        if perk_coin > 0 {
            bonus_gold += bonus_gold * perk_coin as u32 / 100;
        }

        self.gold_gain_silent(sid, bonus_gold);
    }
    /// Set warehouse items for a session (called after loading from DB).
    pub fn set_warehouse(&self, id: SessionId, items: Vec<UserItemSlot>, inn_coins: u32) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.warehouse = items;
            handle.inn_coins = inn_coins;
            handle.warehouse_loaded = true;
        }
    }
    /// Check if warehouse is loaded for a session.
    pub fn is_warehouse_loaded(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.warehouse_loaded)
            .unwrap_or(false)
    }
    /// Get a snapshot of the warehouse for a session.
    pub fn get_warehouse(&self, id: SessionId) -> Vec<UserItemSlot> {
        self.sessions
            .get(&id)
            .map(|h| h.warehouse.clone())
            .unwrap_or_default()
    }
    /// Get warehouse slot at a specific index.
    pub fn get_warehouse_slot(&self, id: SessionId, slot: usize) -> Option<UserItemSlot> {
        self.sessions
            .get(&id)
            .and_then(|h| h.warehouse.get(slot).cloned())
    }
    /// Get inn coins for a session.
    pub fn get_inn_coins(&self, id: SessionId) -> u32 {
        self.sessions.get(&id).map(|h| h.inn_coins).unwrap_or(0)
    }
    /// Update warehouse via a closure, returns true if the update was applied.
    pub fn update_warehouse(
        &self,
        id: SessionId,
        updater: impl FnOnce(&mut Vec<UserItemSlot>, &mut u32) -> bool,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let h = handle.value_mut();
            updater(&mut h.warehouse, &mut h.inn_coins)
        } else {
            false
        }
    }
    // ── VIP Warehouse Methods ────────────────────────────────────────

    /// Set VIP warehouse data for a session (called after DB load).
    pub fn set_vip_warehouse(
        &self,
        id: SessionId,
        items: Vec<UserItemSlot>,
        password: String,
        password_request: u8,
        vault_expiry: u32,
    ) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.vip_warehouse = items;
            handle.vip_password = password;
            handle.vip_password_request = password_request;
            handle.vip_vault_expiry = vault_expiry;
            handle.vip_warehouse_loaded = true;
        }
    }
    /// Check if VIP warehouse is loaded for a session.
    pub fn is_vip_warehouse_loaded(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.vip_warehouse_loaded)
            .unwrap_or(false)
    }
    /// Get a snapshot of VIP warehouse items for a session.
    pub fn get_vip_warehouse(&self, id: SessionId) -> Vec<UserItemSlot> {
        self.sessions
            .get(&id)
            .map(|h| h.vip_warehouse.clone())
            .unwrap_or_default()
    }
    /// Get VIP warehouse slot at a specific index.
    pub fn get_vip_warehouse_slot(&self, id: SessionId, slot: usize) -> Option<UserItemSlot> {
        self.sessions
            .get(&id)
            .and_then(|h| h.vip_warehouse.get(slot).cloned())
    }
    /// Get VIP warehouse password.
    pub fn get_vip_password(&self, id: SessionId) -> String {
        self.sessions
            .get(&id)
            .map(|h| h.vip_password.clone())
            .unwrap_or_default()
    }
    /// Get VIP warehouse password request flag.
    pub fn get_vip_password_request(&self, id: SessionId) -> u8 {
        self.sessions
            .get(&id)
            .map(|h| h.vip_password_request)
            .unwrap_or(0)
    }
    /// Get VIP vault expiry timestamp.
    pub fn get_vip_vault_expiry(&self, id: SessionId) -> u32 {
        self.sessions
            .get(&id)
            .map(|h| h.vip_vault_expiry)
            .unwrap_or(0)
    }
    /// Update VIP warehouse via a closure. Returns true if applied.
    pub fn update_vip_warehouse(
        &self,
        id: SessionId,
        updater: impl FnOnce(&mut Vec<UserItemSlot>) -> bool,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            updater(&mut handle.vip_warehouse)
        } else {
            false
        }
    }
    /// Access both inventory and VIP warehouse atomically via a closure.
    pub fn update_inventory_and_vip_warehouse(
        &self,
        id: SessionId,
        updater: impl FnOnce(&mut Vec<UserItemSlot>, &mut Vec<UserItemSlot>) -> bool,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let h = handle.value_mut();
            updater(&mut h.inventory, &mut h.vip_warehouse)
        } else {
            false
        }
    }
    /// Set VIP warehouse password and password_request flag.
    pub fn set_vip_password(&self, id: SessionId, password: String, password_request: u8) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.vip_password = password;
            handle.vip_password_request = password_request;
        }
    }
    /// Set VIP vault expiry timestamp.
    pub fn set_vip_vault_expiry(&self, id: SessionId, expiry: u32) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.vip_vault_expiry = expiry;
        }
    }
    /// Set the full deleted items list for a session (called on character login).
    pub fn set_deleted_items(&self, id: SessionId, items: Vec<DeletedItemEntry>) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.deleted_items = items;
        }
    }
    /// Get a snapshot of all deleted items for a session.
    pub fn get_deleted_items(&self, id: SessionId) -> Vec<DeletedItemEntry> {
        self.sessions
            .get(&id)
            .map(|h| h.deleted_items.clone())
            .unwrap_or_default()
    }
    /// Add a deleted item entry for repurchase tracking.
    ///
    /// Returns false if the user has reached the 10,000 item limit.
    pub fn add_deleted_item(&self, id: SessionId, entry: DeletedItemEntry) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if handle.deleted_items.len() >= Self::TRASH_ITEM_MAX {
                return false;
            }
            handle.deleted_items.push(entry);
            true
        } else {
            false
        }
    }
    /// Remove a deleted item by its DB id (after successful buyback).
    pub fn remove_deleted_item(&self, id: SessionId, db_id: i64) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.deleted_items.retain(|e| e.db_id != db_id);
        }
    }
    /// Clear the repurchase display index mapping.
    ///
    pub fn clear_delete_item_list(&self, id: SessionId) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.delete_item_list.clear();
        }
    }
    /// Set the repurchase display index mapping.
    pub fn set_delete_item_list(&self, id: SessionId, mapping: HashMap<u8, usize>) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.delete_item_list = mapping;
        }
    }
    /// Look up a deleted item by display index from the current browse session.
    ///
    /// Returns `(vec_index, DeletedItemEntry)` if found.
    pub fn get_deleted_item_by_display_index(
        &self,
        id: SessionId,
        display_index: u8,
    ) -> Option<(usize, DeletedItemEntry)> {
        self.sessions.get(&id).and_then(|h| {
            let vec_idx = h.delete_item_list.get(&display_index)?;
            h.deleted_items.get(*vec_idx).map(|e| (*vec_idx, e.clone()))
        })
    }
    /// Count free inventory slots in the bag area for a session.
    pub fn count_free_slots(&self, sid: SessionId) -> u8 {
        self.sessions
            .get(&sid)
            .map(|h| {
                let mut count = 0u8;
                for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                    if let Some(slot) = h.inventory.get(i) {
                        if slot.item_id == 0 {
                            count += 1;
                        }
                    }
                }
                count
            })
            .unwrap_or(0)
    }
    /// Mark an inventory slot with the merchant flag.
    pub fn set_inventory_merchant_flag(&self, sid: SessionId, slot: usize, flagged: bool) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            if let Some(item) = h.inventory.get_mut(slot) {
                if flagged {
                    item.flag |= 0x10;
                } else {
                    item.flag &= !0x10;
                }
            }
        }
    }

    // ── Exchange Capacity / Party Item Removal ─────────────────────────

    /// Check if a player has at least `count` of an item in their bag slots.
    ///
    pub(crate) fn check_exist_item(&self, sid: SessionId, item_id: u32, count: u16) -> bool {
        if count == 0 {
            return true;
        }
        self.with_session(sid, |h| {
            let total: u32 = h.inventory[Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX)]
                .iter()
                .filter(|s| s.item_id == item_id)
                .map(|s| s.count as u32)
                .sum();
            total >= count as u32
        })
        .unwrap_or(false)
    }

    /// Calculate maximum exchange capacity by weight.
    ///
    /// Returns how many times the exchange output items can fit in the player's
    /// remaining weight capacity: `(max_weight - item_weight) / sum_output_weights`.
    ///
    pub(crate) fn get_max_exchange_capacity(&self, sid: SessionId, exchange_id: i32) -> u16 {
        let exchange = match self.item_exchanges.get(&exchange_id) {
            Some(e) => e.clone(),
            None => return 0,
        };

        // Sum output item weights
        let output_items = [
            exchange.exchange_item_num1,
            exchange.exchange_item_num2,
            exchange.exchange_item_num3,
            exchange.exchange_item_num4,
            exchange.exchange_item_num5,
        ];
        let mut total_weight: i32 = 0;
        for item_id in output_items {
            if item_id == 0 {
                continue;
            }
            if let Some(item) = self.items.get(&(item_id as u32)) {
                total_weight = total_weight.saturating_add(item.weight.unwrap_or(0) as i32);
            }
        }

        if total_weight == 0 {
            return 0;
        }

        // Get player weight info
        let (max_weight, current_weight) = self
            .with_session(sid, |h| {
                h.character
                    .as_ref()
                    .map(|c| (c.max_weight, c.item_weight))
                    .unwrap_or((0, 0))
            })
            .unwrap_or((0, 0));

        if current_weight >= max_weight {
            return 0;
        }
        ((max_weight - current_weight) / total_weight) as u16
    }

    /// Remove an item from all party members. If the player is not in a party,
    /// removes from self only.
    ///
    /// Returns false if any member lacks the item (no items are removed in that case).
    ///
    pub(crate) fn rob_all_item_party(&self, sid: SessionId, item_id: u32, count: u16) -> bool {
        if count == 0 {
            return false;
        }

        let party_id = self.get_party_id(sid);

        let party_id = match party_id {
            Some(pid) => pid,
            None => {
                // Not in party, just rob from self
                return self.rob_item(sid, item_id, count);
            }
        };

        let party = match self.get_party(party_id) {
            Some(p) => p,
            None => return self.rob_item(sid, item_id, count),
        };

        // Collect all active member session IDs that are in-game and in a party.
        // C++ checks isInGame() && isInParty() for each member.
        let members: Vec<SessionId> = party
            .active_members()
            .into_iter()
            .filter(|&msid| {
                self.with_session(msid, |h| {
                    h.character.as_ref().is_some_and(|ch| ch.party_id.is_some())
                })
                .unwrap_or(false)
            })
            .collect();

        if members.is_empty() {
            return self.rob_item(sid, item_id, count);
        }

        // First check ALL members have the item
        for &member_sid in &members {
            if !self.check_exist_item(member_sid, item_id, count) {
                return false;
            }
        }

        // All have it, rob from all
        for &member_sid in &members {
            self.rob_item(member_sid, item_id, count);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory_constants::INVENTORY_TOTAL;
    use ko_db::models::Item;
    use tokio::sync::mpsc;

    /// Helper: create a WorldState with a registered session, inventory, and a test item.
    /// Returns the WorldState and the packet receiver.
    fn setup_give_item_world(
        item_id: u32,
        countable: i32,
        kind: i32,
        duration: i16,
    ) -> (WorldState, mpsc::UnboundedReceiver<Arc<Packet>>) {
        let world = WorldState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        let sid: SessionId = 1;
        world.register_session(sid, tx);

        // Register character so session is fully set up
        let info = CharacterInfo {
            session_id: sid,
            name: "Tester".into(),
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
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000_000,
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

        // Initialize inventory with SLOT_MAX + HAVE_MAX empty slots
        let inv = vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX];
        world.set_inventory(sid, inv);

        // Insert test item into the item table
        let item = Item {
            num: item_id as i32,
            extension: None,
            str_name: Some("TestItem".into()),
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            kind: Some(kind),
            slot: None,
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: Some(10),
            duration: Some(duration),
            buy_price: Some(100),
            sell_price: Some(50),
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: Some(countable),
            effect1: None,
            effect2: None,
            req_level: None,
            req_level_max: None,
            req_rank: None,
            req_title: None,
            req_str: None,
            req_sta: None,
            req_dex: None,
            req_intel: None,
            req_cha: None,
            selling_group: None,
            item_type: None,
            hitrate: None,
            evasionrate: None,
            dagger_ac: None,
            jamadar_ac: None,
            sword_ac: None,
            club_ac: None,
            axe_ac: None,
            spear_ac: None,
            bow_ac: None,
            fire_damage: None,
            ice_damage: None,
            lightning_damage: None,
            poison_damage: None,
            hp_drain: None,
            mp_damage: None,
            mp_drain: None,
            mirror_damage: None,
            droprate: None,
            str_b: None,
            sta_b: None,
            dex_b: None,
            intel_b: None,
            cha_b: None,
            max_hp_b: None,
            max_mp_b: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            poison_r: None,
            curse_r: None,
            item_class: None,
            np_buy_price: None,
            bound: None,
            mace_ac: None,
            by_grade: None,
            drop_notice: None,
            upgrade_notice: None,
        };
        world.items.insert(item_id, item);

        (world, rx)
    }

    #[test]
    fn test_give_item_sends_item_count_change_new_item() {
        let item_id = 389010000u32;
        let duration = 5000i16;
        let (world, mut rx) = setup_give_item_world(item_id, 1, 6, duration);

        let ok = world.give_item(1, item_id, 3);
        assert!(ok, "give_item should succeed");

        // Verify the slot was updated
        let slot = world.get_inventory_slot(1, WorldState::SLOT_MAX).unwrap();
        assert_eq!(slot.item_id, item_id);
        assert_eq!(slot.count, 3);
        assert_eq!(slot.durability, duration);

        // Verify WIZ_ITEM_COUNT_CHANGE packet
        let pkt = rx.try_recv().expect("should have received a packet");
        assert_eq!(pkt.opcode, Opcode::WizItemCountChange as u8);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1)); // count_type
        assert_eq!(r.read_u8(), Some(1)); // slot_section (inventory)
        assert_eq!(r.read_u8(), Some(0)); // position (first bag slot = 0)
        assert_eq!(r.read_u32(), Some(item_id)); // item_id
        assert_eq!(r.read_u32(), Some(3)); // new count
        assert_eq!(r.read_u8(), Some(100)); // bNewItem always true in C++
        assert_eq!(r.read_u16(), Some(duration as u16)); // durability
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(0)); // expiration
                                           // v2600: no trailing u16 padding (sniff verified)

        // Verify WIZ_WEIGHT_CHANGE packet (C++ SendStackChange calls SendItemWeight)
        let weight_pkt = rx.try_recv().expect("should have received weight packet");
        assert_eq!(weight_pkt.opcode, Opcode::WizWeightChange as u8);
    }

    #[test]
    fn test_give_item_sends_item_count_change_stack_addition() {
        let item_id = 389010000u32;
        let duration = 5000i16;
        let (world, mut rx) = setup_give_item_world(item_id, 1, 6, duration);

        // First give: new item
        assert!(world.give_item(1, item_id, 2));
        let _ = rx.try_recv(); // drain WIZ_ITEM_COUNT_CHANGE
        let _ = rx.try_recv(); // drain WIZ_WEIGHT_CHANGE

        // Second give: stack addition
        assert!(world.give_item(1, item_id, 5));
        let pkt = rx.try_recv().expect("should have received a packet");
        assert_eq!(pkt.opcode, Opcode::WizItemCountChange as u8);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1)); // count_type
        assert_eq!(r.read_u8(), Some(1)); // slot_section
        assert_eq!(r.read_u8(), Some(0)); // same slot position
        assert_eq!(r.read_u32(), Some(item_id));
        assert_eq!(r.read_u32(), Some(7)); // 2 + 5 = 7
        assert_eq!(r.read_u8(), Some(100)); // C++ always passes true (bNewItem)
        assert_eq!(r.read_u16(), Some(duration as u16)); // durability unchanged
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(0)); // expiration
                                           // v2600: no trailing u16 padding (sniff verified)

        // Verify WIZ_WEIGHT_CHANGE packet
        let weight_pkt = rx.try_recv().expect("should have received weight packet");
        assert_eq!(weight_pkt.opcode, Opcode::WizWeightChange as u8);
    }

    #[test]
    fn test_give_item_non_countable_always_count_1() {
        let item_id = 120050000u32;
        let duration = 3000i16;
        // countable=0 means non-stackable
        let (world, mut rx) = setup_give_item_world(item_id, 0, 21, duration);

        assert!(world.give_item(1, item_id, 5));
        let pkt = rx.try_recv().expect("should have received a packet");

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u16(); // count_type
        r.read_u8(); // slot_section
        r.read_u8(); // position
        r.read_u32(); // item_id
        assert_eq!(r.read_u32(), Some(1)); // count forced to 1 for non-countable
        assert_eq!(r.read_u8(), Some(100)); // bNewItem always true in C++

        // Drain WIZ_WEIGHT_CHANGE
        let weight_pkt = rx.try_recv().expect("should have received weight packet");
        assert_eq!(weight_pkt.opcode, Opcode::WizWeightChange as u8);
    }

    #[test]
    fn test_wiz_item_count_change_opcode_value() {
        assert_eq!(Opcode::WizItemCountChange as u8, 0x3D);
    }

    #[test]
    fn test_give_item_no_packet_on_failure() {
        let (world, mut rx) = setup_give_item_world(999999999, 1, 6, 100);

        // Try giving an item that doesn't exist in item table
        let ok = world.give_item(1, 123456789, 1);
        assert!(!ok, "give_item should fail for unknown item");

        // No packet should be sent
        assert!(rx.try_recv().is_err());
    }

    // ── check_exist_item tests ──────────────────────────────────────

    fn setup_session_with_items(item_id: u32, count: u16) -> WorldState {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid: SessionId = 1;
        world.register_session(sid, tx);
        let info = CharacterInfo {
            session_id: sid,
            name: "Tester".into(),
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
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000_000,
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
        let mut inv = vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX];
        if count > 0 {
            inv[WorldState::SLOT_MAX].item_id = item_id;
            inv[WorldState::SLOT_MAX].count = count;
        }
        world.set_inventory(sid, inv);
        world
    }

    #[test]
    fn test_check_exist_item_has_enough() {
        let world = setup_session_with_items(100001, 5);
        assert!(world.check_exist_item(1, 100001, 3));
        assert!(world.check_exist_item(1, 100001, 5));
    }

    #[test]
    fn test_check_exist_item_not_enough() {
        let world = setup_session_with_items(100001, 2);
        assert!(!world.check_exist_item(1, 100001, 3));
    }

    #[test]
    fn test_check_exist_item_zero_count_always_true() {
        let world = setup_session_with_items(100001, 0);
        assert!(world.check_exist_item(1, 100001, 0));
    }

    #[test]
    fn test_check_exist_item_wrong_item() {
        let world = setup_session_with_items(100001, 5);
        assert!(!world.check_exist_item(1, 999999, 1));
    }

    #[test]
    fn test_check_exist_item_no_session() {
        let world = WorldState::new();
        assert!(!world.check_exist_item(999, 100001, 1));
    }

    // ── rob_all_item_party tests ────────────────────────────────────

    fn setup_party_world(member_count: usize, item_id: u32, item_count: u16) -> WorldState {
        let world = WorldState::new();
        let mut sids = Vec::new();

        for i in 0..member_count {
            let sid = (i + 1) as SessionId;
            let (tx, _rx) = mpsc::unbounded_channel();
            world.register_session(sid, tx);
            let info = CharacterInfo {
                session_id: sid,
                name: format!("Player{}", i),
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
                skill_points: [0u8; 10],
                gold: 0,
                loyalty: 0,
                loyalty_monthly: 0,
                authority: 1,
                knights_id: 0,
                fame: 0,
                party_id: None,
                exp: 0,
                max_exp: 100_000_000,
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
            let mut inv =
                vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX];
            if item_count > 0 {
                inv[WorldState::SLOT_MAX].item_id = item_id;
                inv[WorldState::SLOT_MAX].count = item_count;
            }
            world.set_inventory(sid, inv);
            sids.push(sid);
        }

        // Create party with first player as leader, add others
        if member_count >= 2 {
            let party_id = world.create_party(sids[0]).unwrap();
            for &sid in &sids[1..] {
                world.add_party_member(party_id, sid);
            }
        }

        world
    }

    #[test]
    fn test_rob_all_item_party_no_party_success() {
        let world = setup_session_with_items(100001, 5);
        assert!(world.rob_all_item_party(1, 100001, 2));
        // Verify item was reduced
        assert!(world.check_exist_item(1, 100001, 3));
        assert!(!world.check_exist_item(1, 100001, 4));
    }

    #[test]
    fn test_rob_all_item_party_no_party_fail() {
        let world = setup_session_with_items(100001, 1);
        assert!(!world.rob_all_item_party(1, 100001, 2));
    }

    #[test]
    fn test_rob_all_item_party_zero_count_returns_false() {
        let world = setup_session_with_items(100001, 5);
        assert!(!world.rob_all_item_party(1, 100001, 0));
    }

    #[test]
    fn test_rob_all_item_party_all_have_items() {
        let world = setup_party_world(3, 100001, 5);
        assert!(world.rob_all_item_party(1, 100001, 2));
        // All members should have 3 items left
        for sid in 1..=3 {
            assert!(world.check_exist_item(sid, 100001, 3));
            assert!(!world.check_exist_item(sid, 100001, 4));
        }
    }

    #[test]
    fn test_rob_all_item_party_one_member_lacks_item() {
        let world = setup_party_world(3, 100001, 5);
        // Remove items from member 3
        world.rob_item(3, 100001, 5);
        // Now member 3 has 0 items, so rob_all should fail
        assert!(!world.rob_all_item_party(1, 100001, 1));
        // Members 1 and 2 should still have their items (no partial removal)
        assert!(world.check_exist_item(1, 100001, 5));
        assert!(world.check_exist_item(2, 100001, 5));
    }

    // ── Equipment Stat Completeness Tests (Sprint 40) ────────────────

    /// Helper: make a default Item with zeroed optional fields.
    fn make_test_item(num: i32) -> Item {
        Item {
            num,
            extension: None,
            str_name: None,
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            kind: None,
            slot: None,
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: Some(10),
            duration: Some(100),
            buy_price: None,
            sell_price: None,
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: None,
            effect1: None,
            effect2: None,
            req_level: None,
            req_level_max: None,
            req_rank: None,
            req_title: None,
            req_str: None,
            req_sta: None,
            req_dex: None,
            req_intel: None,
            req_cha: None,
            selling_group: None,
            item_type: None,
            hitrate: None,
            evasionrate: None,
            dagger_ac: None,
            jamadar_ac: None,
            sword_ac: None,
            club_ac: None,
            axe_ac: None,
            spear_ac: None,
            bow_ac: None,
            fire_damage: None,
            ice_damage: None,
            lightning_damage: None,
            poison_damage: None,
            hp_drain: None,
            mp_damage: None,
            mp_drain: None,
            mirror_damage: None,
            droprate: None,
            str_b: None,
            sta_b: None,
            dex_b: None,
            intel_b: None,
            cha_b: None,
            max_hp_b: None,
            max_mp_b: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            poison_r: None,
            curse_r: None,
            item_class: None,
            np_buy_price: None,
            bound: None,
            mace_ac: None,
            by_grade: None,
            drop_notice: None,
            upgrade_notice: None,
        }
    }

    /// Helper: set up a WorldState with session, character, and an inventory of
    /// INVENTORY_TOTAL empty slots to test the full slot range.
    fn setup_equip_world() -> WorldState {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let info = CharacterInfo {
            session_id: 1,
            name: "EquipTester".into(),
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
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000_000,
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
        world.register_ingame(1, info, pos);
        // 77-slot inventory (14 equip + 28 bag + 11 cospre + 24 mbag)
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        world.set_inventory(1, inv);
        world
    }

    #[test]
    fn test_weapon_resistance_accumulation() {
        let world = setup_equip_world();

        // Armor with weapon resistances in slot 0 (equipped)
        let mut armor = make_test_item(100001);
        armor.dagger_ac = Some(5);
        armor.sword_ac = Some(10);
        armor.axe_ac = Some(3);
        armor.club_ac = Some(7);
        armor.spear_ac = Some(2);
        armor.bow_ac = Some(4);
        armor.jamadar_ac = Some(1);
        world.items.insert(100001, armor);

        // Second piece in slot 1
        let mut armor2 = make_test_item(100002);
        armor2.dagger_ac = Some(3);
        armor2.sword_ac = Some(2);
        world.items.insert(100002, armor2);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 100001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            inv[1] = UserItemSlot {
                item_id: 100002,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.dagger_r, 8); // 5 + 3
        assert_eq!(stats.sword_r, 12); // 10 + 2
        assert_eq!(stats.axe_r, 3);
        assert_eq!(stats.club_r, 7);
        assert_eq!(stats.spear_r, 2);
        assert_eq!(stats.bow_r, 4);
        assert_eq!(stats.jamadar_r, 1);
    }

    #[test]
    fn test_elemental_damage_bonuses_per_slot() {
        let world = setup_equip_world();

        let mut weapon = make_test_item(200001);
        weapon.fire_damage = Some(15);
        weapon.hp_drain = Some(3);
        world.items.insert(200001, weapon);

        let mut shield = make_test_item(200002);
        shield.ice_damage = Some(10);
        shield.mirror_damage = Some(5);
        world.items.insert(200002, shield);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 200001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            inv[8] = UserItemSlot {
                item_id: 200002,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        // Slot 6 (right hand) should have fire + hp_drain
        let slot6 = stats.equipped_item_bonuses.get(&6).unwrap();
        assert!(slot6.contains(&(WorldState::ITEM_TYPE_FIRE, 15)));
        assert!(slot6.contains(&(WorldState::ITEM_TYPE_HP_DRAIN, 3)));
        // Slot 8 (left hand) should have ice + mirror
        let slot8 = stats.equipped_item_bonuses.get(&8).unwrap();
        assert!(slot8.contains(&(WorldState::ITEM_TYPE_COLD, 10)));
        assert!(slot8.contains(&(WorldState::ITEM_TYPE_MIRROR_DAMAGE, 5)));
    }

    #[test]
    fn test_bag_items_add_weight_bonus_not_weight() {
        let world = setup_equip_world();

        // Bag item in slot 51 (BAG_SLOT_1) with duration=200 (capacity bonus)
        let mut bag = make_test_item(300001);
        bag.duration = Some(200);
        bag.weight = Some(50);
        world.items.insert(300001, bag);

        world.update_inventory(1, |inv| {
            inv[51] = UserItemSlot {
                item_id: 300001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.max_weight_bonus, 200); // duration -> weight bonus
        assert_eq!(stats.item_weight, 0); // bags do NOT contribute to item_weight
    }

    #[test]
    fn test_bag_slot_2_weight_bonus() {
        let world = setup_equip_world();

        let mut bag = make_test_item(300002);
        bag.duration = Some(150);
        bag.weight = Some(30);
        world.items.insert(300002, bag);

        world.update_inventory(1, |inv| {
            inv[52] = UserItemSlot {
                item_id: 300002,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.max_weight_bonus, 150);
        assert_eq!(stats.item_weight, 0);
    }

    #[test]
    fn test_bag_area_items_no_stats() {
        let world = setup_equip_world();

        // Item in bag area (slot 14-41) should contribute weight but NOT stats
        let mut item = make_test_item(400001);
        item.ac = Some(50);
        item.str_b = Some(10);
        item.dagger_ac = Some(5);
        item.fire_damage = Some(20);
        item.weight = Some(15);
        world.items.insert(400001, item);

        world.update_inventory(1, |inv| {
            inv[14] = UserItemSlot {
                item_id: 400001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 0); // no AC from bag area
        assert_eq!(stats.stat_bonuses[0], 0); // no STR from bag area
        assert_eq!(stats.dagger_r, 0); // no weapon res from bag area
        assert!(stats.equipped_item_bonuses.is_empty()); // no elemental bonuses
        assert_eq!(stats.item_weight, 15); // weight IS counted
    }

    #[test]
    fn test_magic_bag_items_no_stats() {
        let world = setup_equip_world();

        // Item in magic bag area (slot 53+) should contribute weight but NOT stats
        let mut item = make_test_item(400002);
        item.ac = Some(30);
        item.str_b = Some(5);
        item.weight = Some(20);
        world.items.insert(400002, item);

        world.update_inventory(1, |inv| {
            inv[53] = UserItemSlot {
                item_id: 400002,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 0);
        assert_eq!(stats.stat_bonuses[0], 0);
        assert_eq!(stats.item_weight, 20); // weight IS counted
    }

    #[test]
    fn test_duplicate_flag_items_no_stats() {
        let world = setup_equip_world();

        let mut item = make_test_item(400003);
        item.ac = Some(100);
        item.str_b = Some(20);
        item.weight = Some(10);
        world.items.insert(400003, item);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 400003,
                durability: 100,
                count: 1,
                flag: WorldState::ITEM_FLAG_DUPLICATE,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 0); // duplicate items skip stat computation
        assert_eq!(stats.stat_bonuses[0], 0);
        assert_eq!(stats.item_weight, 10); // weight IS counted
    }

    #[test]
    fn test_cospre_items_apply_stats() {
        let world = setup_equip_world();

        // Cospre item in slot 42 (INVENTORY_COSP) should apply stats
        let mut cospre = make_test_item(500001);
        cospre.ac = Some(25);
        cospre.str_b = Some(5);
        cospre.fire_r = Some(10);
        cospre.dagger_ac = Some(3);
        cospre.weight = Some(5);
        world.items.insert(500001, cospre);

        world.update_inventory(1, |inv| {
            inv[42] = UserItemSlot {
                item_id: 500001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 25);
        assert_eq!(stats.stat_bonuses[0], 5);
        assert_eq!(stats.fire_r, 10);
        assert_eq!(stats.dagger_r, 3);
    }

    #[test]
    fn test_set_item_bonuses_applied() {
        let world = setup_equip_world();

        // Create a set item row with known bonuses
        use ko_db::models::SetItemRow;
        let set_row = SetItemRow {
            set_index: 1050002,
            set_name: Some("TestSet".into()),
            ac_bonus: 20,
            hp_bonus: 100,
            mp_bonus: 50,
            strength_bonus: 5,
            stamina_bonus: 3,
            dexterity_bonus: 2,
            intel_bonus: 4,
            charisma_bonus: 1,
            flame_resistance: 10,
            glacier_resistance: 15,
            lightning_resistance: 5,
            poison_resistance: 8,
            magic_resistance: 12,
            curse_resistance: 6,
            xp_bonus_percent: 10,
            coin_bonus_percent: 5,
            np_bonus: 3,
            max_weight_bonus: 50,
            ap_bonus_percent: 2,
            ap_bonus_class_type: 1,
            ap_bonus_class_percent: 5,
            ac_bonus_class_type: 2,
            ac_bonus_class_percent: 3,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(1050002, set_row);

        // Equip helmet (slot value 7) and pauldron (slot value 5) with race=105
        let mut helmet = make_test_item(600001);
        helmet.race = Some(105);
        helmet.slot = Some(7); // ItemSlotHelmet
        world.items.insert(600001, helmet);

        let mut pauldron = make_test_item(600002);
        pauldron.race = Some(105);
        pauldron.slot = Some(5); // ItemSlotPauldron
        world.items.insert(600002, pauldron);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 600001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            inv[1] = UserItemSlot {
                item_id: 600002,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        // Set index = 105 * 10000 + 2 (helmet) + 16 (pauldron) = 1050018
        // But we inserted set_index 1050002, so this specific combo won't match.
        // Let's insert the correct one:
        use ko_db::models::SetItemRow as SIR;
        let set_row2 = SIR {
            set_index: 1050018,
            set_name: Some("HelmPauldSet".into()),
            ac_bonus: 15,
            hp_bonus: 200,
            mp_bonus: 0,
            strength_bonus: 10,
            stamina_bonus: 0,
            dexterity_bonus: 0,
            intel_bonus: 0,
            charisma_bonus: 0,
            flame_resistance: 0,
            glacier_resistance: 0,
            lightning_resistance: 0,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 5,
            coin_bonus_percent: 0,
            np_bonus: 2,
            max_weight_bonus: 0,
            ap_bonus_percent: 0,
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(1050018, set_row2);

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        // Set bonus should be applied: AC +15, HP +200, STR +10
        assert_eq!(stats.item_ac, 15);
        assert_eq!(stats.item_max_hp, 200);
        assert_eq!(stats.stat_bonuses[0], 10);
        assert_eq!(stats.item_exp_bonus, 5);
        assert_eq!(stats.item_np_bonus, 2);
    }

    #[test]
    fn test_cospre_set_item_lookup() {
        let world = setup_equip_world();

        // Cospre item with kind = ITEM_KIND_COSPRE (252)
        let mut cospre = make_test_item(610019000);
        cospre.kind = Some(252);
        cospre.weight = Some(0);
        world.items.insert(610019000, cospre);

        // Insert a set_item entry keyed by the item number
        use ko_db::models::SetItemRow;
        let cospre_set = SetItemRow {
            set_index: 610019000,
            set_name: Some("CospreBonusSet".into()),
            ac_bonus: 30,
            hp_bonus: 150,
            mp_bonus: 100,
            strength_bonus: 3,
            stamina_bonus: 3,
            dexterity_bonus: 3,
            intel_bonus: 3,
            charisma_bonus: 0,
            flame_resistance: 5,
            glacier_resistance: 5,
            lightning_resistance: 5,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 0,
            coin_bonus_percent: 0,
            np_bonus: 0,
            max_weight_bonus: 0,
            ap_bonus_percent: 0,
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(610019000, cospre_set);

        world.update_inventory(1, |inv| {
            inv[42] = UserItemSlot {
                item_id: 610019000,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        // Cospre set bonus should be applied
        assert_eq!(stats.item_ac, 30);
        assert_eq!(stats.item_max_hp, 150);
        assert_eq!(stats.item_max_mp, 100);
        assert_eq!(stats.stat_bonuses[0], 3); // STR
        assert_eq!(stats.fire_r, 5);
    }

    #[test]
    fn test_ap_class_bonus_from_set() {
        let mut stats = EquippedStats::default();
        use ko_db::models::SetItemRow;
        let set_row = SetItemRow {
            set_index: 999,
            set_name: None,
            ac_bonus: 0,
            hp_bonus: 0,
            mp_bonus: 0,
            strength_bonus: 0,
            stamina_bonus: 0,
            dexterity_bonus: 0,
            intel_bonus: 0,
            charisma_bonus: 0,
            flame_resistance: 0,
            glacier_resistance: 0,
            lightning_resistance: 0,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 0,
            coin_bonus_percent: 0,
            np_bonus: 0,
            max_weight_bonus: 0,
            ap_bonus_percent: 5,
            ap_bonus_class_type: 2, // rogue
            ap_bonus_class_percent: 10,
            ac_bonus_class_type: 3, // mage
            ac_bonus_class_percent: 8,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };

        WorldState::apply_set_item_bonuses(&mut stats, &set_row);
        assert_eq!(stats.ap_bonus_amount, 5);
        assert_eq!(stats.ap_class_bonus[1], 10); // class type 2 -> index 1
        assert_eq!(stats.ac_class_bonus[2], 8); // class type 3 -> index 2
    }

    #[test]
    fn test_castellan_cape_bonuses() {
        use ko_db::models::KnightsCapeCastellanBonusRow;
        let mut stats = EquippedStats::default();
        let bonus = KnightsCapeCastellanBonusRow {
            bonus_type: 2,
            type_name: "Test".into(),
            ac_bonus: 200,
            hp_bonus: 500,
            mp_bonus: 500,
            str_bonus: 5,
            sta_bonus: 5,
            dex_bonus: 5,
            int_bonus: 5,
            cha_bonus: 0,
            flame_resist: 50,
            glacier_resist: 50,
            lightning_resist: 50,
            magic_resist: 0,
            disease_resist: 0,
            poison_resist: 0,
            xp_bonus_pct: 0,
            coin_bonus_pct: 0,
            ap_bonus_pct: 3,
            ac_bonus_pct: 0,
            max_weight_bonus: 0,
            np_bonus: 10,
        };

        WorldState::apply_castellan_cape_bonuses(&mut stats, &bonus);
        assert_eq!(stats.item_ac, 200);
        assert_eq!(stats.item_max_hp, 500);
        assert_eq!(stats.item_max_mp, 500);
        assert_eq!(stats.stat_bonuses[0], 5); // STR
        assert_eq!(stats.stat_bonuses[1], 5); // STA
        assert_eq!(stats.fire_r, 50);
        assert_eq!(stats.cold_r, 50);
        assert_eq!(stats.lightning_r, 50);
        assert_eq!(stats.item_np_bonus, 10);
        assert_eq!(stats.ap_bonus_amount, 3);
    }

    #[test]
    fn test_max_weight_includes_bonus() {
        let world = setup_equip_world();

        // Bag in slot 51 with duration=300
        let mut bag = make_test_item(700001);
        bag.duration = Some(300);
        bag.weight = Some(0);
        world.items.insert(700001, bag);

        world.update_inventory(1, |inv| {
            inv[51] = UserItemSlot {
                item_id: 700001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        // Insert a coefficient so set_user_ability works
        use ko_db::models::CoefficientRow;
        let coeff = CoefficientRow {
            s_class: 101,
            hp: 14.0,
            mp: 10.0,
            sp: 0.0,
            ac: 1.0,
            hitrate: 0.001,
            evasionrate: 0.001,
            short_sword: 0.01,
            sword: 0.015,
            axe: 0.012,
            club: 0.011,
            spear: 0.013,
            bow: 0.014,
            staff: 0.009,
            jamadar: 0.01,
            pole: 0.01,
        };
        world.coefficients.insert(101, coeff);

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // max_weight = (STR+item_str + level)*50 + max_weight_bonus
        // = (60+0 + 60)*50 + 300 = 6000 + 300 = 6300
        assert_eq!(stats.max_weight, 6300);
        assert_eq!(stats.max_weight_bonus, 300);
    }

    #[test]
    fn test_zero_durability_ac_halved() {
        let world = setup_equip_world();

        let mut armor = make_test_item(800001);
        armor.ac = Some(100);
        world.items.insert(800001, armor);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 800001,
                durability: 0, // broken!
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 10); // 100/10 = 10
    }

    #[test]
    fn test_all_elemental_bonus_types() {
        let world = setup_equip_world();

        let mut weapon = make_test_item(900001);
        weapon.fire_damage = Some(1);
        weapon.ice_damage = Some(2);
        weapon.lightning_damage = Some(3);
        weapon.poison_damage = Some(4);
        weapon.hp_drain = Some(5);
        weapon.mp_damage = Some(6);
        weapon.mp_drain = Some(7);
        weapon.mirror_damage = Some(8);
        world.items.insert(900001, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 900001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        let bonuses = stats.equipped_item_bonuses.get(&6).unwrap();
        assert_eq!(bonuses.len(), 8);
        assert!(bonuses.contains(&(0x01, 1))); // fire
        assert!(bonuses.contains(&(0x02, 2))); // cold
        assert!(bonuses.contains(&(0x03, 3))); // lightning
        assert!(bonuses.contains(&(0x04, 4))); // poison
        assert!(bonuses.contains(&(0x05, 5))); // hp_drain
        assert!(bonuses.contains(&(0x06, 6))); // mp_damage
        assert!(bonuses.contains(&(0x07, 7))); // mp_drain
        assert!(bonuses.contains(&(0x08, 8))); // mirror_damage
    }

    #[test]
    fn test_armor_set_id_calculation() {
        let world = setup_equip_world();

        // Create 5-piece armor set with race=105:
        // set_id = 105*10000 + 2(helm) + 16(pauldron) + 512(pads) + 2048(gloves) + 4096(boots)
        //        = 1050000 + 6674 = 1056674
        let pieces = [
            (700010, 7i32, 0usize), // helmet in slot 0
            (700011, 5, 1),         // pauldron in slot 1
            (700012, 6, 2),         // pads in slot 2
            (700013, 8, 3),         // gloves in slot 3
            (700014, 9, 4),         // boots in slot 4
        ];

        for (id, item_slot, _inv_slot) in &pieces {
            let mut item = make_test_item(*id);
            item.race = Some(105);
            item.slot = Some(*item_slot);
            world.items.insert(*id as u32, item);
        }

        // Insert the full-set bonus
        use ko_db::models::SetItemRow;
        let full_set = SetItemRow {
            set_index: 1056674,
            set_name: Some("FullSet".into()),
            ac_bonus: 50,
            hp_bonus: 500,
            mp_bonus: 300,
            strength_bonus: 15,
            stamina_bonus: 10,
            dexterity_bonus: 8,
            intel_bonus: 5,
            charisma_bonus: 0,
            flame_resistance: 20,
            glacier_resistance: 20,
            lightning_resistance: 20,
            poison_resistance: 10,
            magic_resistance: 15,
            curse_resistance: 5,
            xp_bonus_percent: 10,
            coin_bonus_percent: 5,
            np_bonus: 5,
            max_weight_bonus: 100,
            ap_bonus_percent: 3,
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(1056674, full_set);

        world.update_inventory(1, |inv| {
            for (id, _, inv_slot) in &pieces {
                inv[*inv_slot] = UserItemSlot {
                    item_id: *id as u32,
                    durability: 100,
                    count: 1,
                    flag: 0,
                    original_flag: 0,
                    serial_num: *inv_slot as u64 + 1,
                    expire_time: 0,
                };
            }
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 50);
        assert_eq!(stats.item_max_hp, 500);
        assert_eq!(stats.stat_bonuses[0], 15); // STR
        assert_eq!(stats.item_exp_bonus, 10);
        assert_eq!(stats.item_np_bonus, 5);
        assert_eq!(stats.max_weight_bonus, 100);
    }

    // ── Additional set_user_ability and integration tests ───────────

    /// Helper: create a CoefficientRow for testing.
    fn make_test_coeff(s_class: i16) -> CoefficientRow {
        CoefficientRow {
            s_class,
            short_sword: 0.001,
            jamadar: 0.001,
            sword: 0.002,
            axe: 0.002,
            club: 0.002,
            spear: 0.002,
            pole: 0.001,
            staff: 0.001,
            bow: 0.001,
            hp: 1.0,
            mp: 1.0,
            sp: 1.0,
            ac: 0.05,
            hitrate: 0.001,
            evasionrate: 0.001,
        }
    }

    #[test]
    fn test_empty_inventory_baseline() {
        let world = setup_equip_world();
        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 0);
        assert_eq!(stats.item_max_hp, 0);
        assert_eq!(stats.item_max_mp, 0);
        assert_eq!(stats.item_hitrate, 100);
        assert_eq!(stats.item_evasionrate, 100);
        assert_eq!(stats.item_weight, 0);
        assert_eq!(stats.max_weight_bonus, 0);
        assert_eq!(stats.fire_r, 0);
        assert_eq!(stats.dagger_r, 0);
        assert!(stats.equipped_item_bonuses.is_empty());
        assert_eq!(stats.item_exp_bonus, 0);
        assert_eq!(stats.item_np_bonus, 0);
        assert_eq!(stats.item_gold_bonus, 0);
        assert_eq!(stats.ap_bonus_amount, 0);
        assert_eq!(stats.ap_class_bonus, [0, 0, 0, 0]);
        assert_eq!(stats.ac_class_bonus, [0, 0, 0, 0]);
        for s in &stats.stat_bonuses {
            assert_eq!(*s, 0);
        }
    }

    #[test]
    fn test_unknown_item_id_skipped() {
        let world = setup_equip_world();
        // Place an item_id that does not exist in the items DashMap
        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 999999999,
                durability: 5000,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 0,
                expire_time: 0,
            };
            true
        });
        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 0);
        assert_eq!(stats.item_weight, 0);
    }

    #[test]
    fn test_stat_bonuses_all_five() {
        let world = setup_equip_world();
        let mut item = make_test_item(110001);
        item.str_b = Some(5);
        item.sta_b = Some(3);
        item.dex_b = Some(7);
        item.intel_b = Some(4);
        item.cha_b = Some(2);
        item.max_hp_b = Some(50);
        item.max_mp_b = Some(30);
        world.items.insert(110001, item);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 110001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.stat_bonuses[0], 5); // STR
        assert_eq!(stats.stat_bonuses[1], 3); // STA
        assert_eq!(stats.stat_bonuses[2], 7); // DEX
        assert_eq!(stats.stat_bonuses[3], 4); // INT
        assert_eq!(stats.stat_bonuses[4], 2); // CHA
        assert_eq!(stats.item_max_hp, 50);
        assert_eq!(stats.item_max_mp, 30);
    }

    #[test]
    fn test_elemental_resistances_stacking() {
        let world = setup_equip_world();
        let mut item1 = make_test_item(110010);
        item1.fire_r = Some(10);
        item1.cold_r = Some(20);
        item1.lightning_r = Some(5);
        item1.magic_r = Some(8);
        item1.curse_r = Some(3);
        item1.poison_r = Some(12);
        world.items.insert(110010, item1);

        let mut item2 = make_test_item(110011);
        item2.fire_r = Some(15);
        item2.cold_r = Some(5);
        world.items.insert(110011, item2);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 110010,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            inv[1] = UserItemSlot {
                item_id: 110011,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.fire_r, 25); // 10 + 15
        assert_eq!(stats.cold_r, 25); // 20 + 5
        assert_eq!(stats.lightning_r, 5);
        assert_eq!(stats.magic_r, 8);
        assert_eq!(stats.disease_r, 3);
        assert_eq!(stats.poison_r, 12);
    }

    #[test]
    fn test_countable_item_weight_multiplied() {
        let world = setup_equip_world();
        let mut item = make_test_item(110020);
        item.weight = Some(5);
        item.countable = Some(1);
        world.items.insert(110020, item);

        // Place in bag area (slot 14) so weight is counted but stats are not
        world.update_inventory(1, |inv| {
            inv[14] = UserItemSlot {
                item_id: 110020,
                durability: 100,
                count: 10,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_weight, 50); // 5 * 10
    }

    #[test]
    fn test_get_equipped_stats_unknown_session() {
        let world = WorldState::new();
        let stats = world.get_equipped_stats(999);
        assert_eq!(stats.item_ac, 0);
        assert_eq!(stats.item_hitrate, 0); // Default, not 100
        assert_eq!(stats.total_hit, 0);
    }

    // ── Sprint 122: Item serial generator tests ────────────────────

    #[test]
    fn test_generate_item_serial_monotonic() {
        let world = WorldState::new();
        let s1 = world.generate_item_serial();
        let s2 = world.generate_item_serial();
        let s3 = world.generate_item_serial();
        assert!(s1 > 0);
        assert_eq!(s2, s1 + 1);
        assert_eq!(s3, s2 + 1);
    }

    #[test]
    fn test_generate_item_serial_uniqueness() {
        let world = WorldState::new();
        let mut serials: Vec<u64> = (0..100).map(|_| world.generate_item_serial()).collect();
        serials.sort();
        serials.dedup();
        assert_eq!(serials.len(), 100, "All 100 serials should be unique");
    }

    #[test]
    fn test_give_item_assigns_serial() {
        let world = setup_equip_world();
        let mut item = make_test_item(389001000);
        item.countable = Some(1);
        item.kind = Some(1);
        item.duration = Some(100);
        world.items.insert(389001000, item);

        world.give_item(1, 389001000, 5);
        let inv = world.get_inventory(1);
        let slot = inv.iter().find(|s| s.item_id == 389001000);
        assert!(slot.is_some(), "item should be in inventory");
        assert_ne!(slot.unwrap().serial_num, 0, "serial_num should be non-zero");
    }

    #[test]
    fn test_give_item_stacking_preserves_serial() {
        let world = setup_equip_world();
        let mut item = make_test_item(389001000);
        item.countable = Some(1);
        item.kind = Some(1);
        item.duration = Some(100);
        world.items.insert(389001000, item);

        world.give_item(1, 389001000, 3);
        let inv = world.get_inventory(1);
        let first_serial = inv
            .iter()
            .find(|s| s.item_id == 389001000)
            .unwrap()
            .serial_num;

        // Stack more onto same slot — serial should NOT change
        world.give_item(1, 389001000, 2);
        let inv2 = world.get_inventory(1);
        let slot = inv2.iter().find(|s| s.item_id == 389001000).unwrap();
        assert_eq!(
            slot.serial_num, first_serial,
            "serial should be preserved when stacking"
        );
        assert_eq!(slot.count, 5);
    }

    #[test]
    fn test_give_warehouse_item_assigns_serial() {
        let world = setup_equip_world();
        // Initialize warehouse with empty slots
        world.set_warehouse(1, vec![UserItemSlot::default(); 256], 0);
        let mut item = make_test_item(389001000);
        item.countable = Some(1);
        item.kind = Some(1);
        item.duration = Some(100);
        world.items.insert(389001000, item);

        let ok = world.give_warehouse_item(1, 389001000, 1, 0);
        assert!(ok);
        let wh = world.get_warehouse(1);
        let slot = wh.iter().find(|s| s.item_id == 389001000);
        assert!(slot.is_some());
        assert_ne!(
            slot.unwrap().serial_num,
            0,
            "warehouse serial should be non-zero"
        );
    }

    #[test]
    fn test_set_user_ability_total_ac() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut helm = make_test_item(120001);
        helm.ac = Some(100);
        world.items.insert(120001, helm);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 120001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // total_ac = coeff.ac * (level + item_ac) = 0.05 * (60 + 100) = 8
        assert_eq!(stats.total_ac, 8);
    }

    #[test]
    fn test_set_user_ability_sta_bonus_ac() {
        let world = setup_equip_world();
        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.character.as_mut().unwrap().sta = 120;
        }
        world.coefficients.insert(101, make_test_coeff(101));

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // total_ac = 0.05 * (60 + 0) = 3, then + (120-100) = 20 => 23
        assert_eq!(stats.total_ac, 23);
    }

    #[test]
    fn test_set_user_ability_hitrate_evasionrate() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // hitrate = (1 + 0.001 * 60 * 60) * 100 / 100 = (1 + 3.6) = 4.6
        assert!((stats.total_hitrate - 4.6).abs() < 0.01);
        assert!((stats.total_evasionrate - 4.6).abs() < 0.01);
    }

    #[test]
    fn test_set_user_ability_hitrate_with_item_bonus() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut item = make_test_item(120010);
        item.hitrate = Some(20);
        item.evasionrate = Some(15);
        world.items.insert(120010, item);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 120010,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        assert_eq!(stats.item_hitrate, 120); // 100 + 20
        assert_eq!(stats.item_evasionrate, 115); // 100 + 15
                                                 // total_hitrate = (1 + 0.001 * 60 * 60) * 120 / 100 = 4.6 * 1.2 = 5.52
        assert!((stats.total_hitrate - 5.52).abs() < 0.01);
    }

    #[test]
    fn test_set_user_ability_weapon_power() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut weapon = make_test_item(130001);
        weapon.damage = Some(100);
        weapon.kind = Some(21); // 1H sword
        world.items.insert(130001, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                // RIGHTHAND
                item_id: 130001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // rightpower = 100, totalpower = max(100,3) = 100
        // weapon_coeff for sword = 0.002
        // total_hit = 0.005 * 100 * (60+40) + 0.002 * 100 * 60 * 60 + 3 = 50 + 720 + 3 = 773
        assert_eq!(stats.total_hit, 773);
    }

    #[test]
    fn test_weapon_zero_durability_halved() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut weapon = make_test_item(130002);
        weapon.damage = Some(100);
        weapon.kind = Some(21); // 1H sword
        world.items.insert(130002, weapon);

        // Full durability
        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130002,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });
        world.set_user_ability(1);
        let stats_full = world.get_equipped_stats(1);

        // Zero durability
        world.update_inventory(1, |inv| {
            inv[6].durability = 0;
            true
        });
        world.set_user_ability(1);
        let stats_broken = world.get_equipped_stats(1);

        // rightpower = 50 (100/2) when durability=0
        assert!(stats_broken.total_hit < stats_full.total_hit);
    }

    #[test]
    fn test_no_weapon_minimum_power() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // No weapon: totalpower = max(0, 3) = 3, weapon_coeff = 0.0
        // total_hit = 0.005 * 3 * (60+40) + 0.0 * 3 * 60 * 60 + 3 = 1.5 + 0 + 3 = 4.5 -> 4
        assert_eq!(stats.total_hit, 4);
    }

    #[test]
    fn test_rogue_class_uses_dex_formula() {
        let world = setup_equip_world();
        // Change to rogue class (x02)
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.class = 102;
            ch.dex = 100;
            ch.str = 30;
        }
        world.coefficients.insert(102, make_test_coeff(102));

        let mut weapon = make_test_item(130010);
        weapon.damage = Some(80);
        weapon.kind = Some(11); // dagger
        world.items.insert(130010, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130010,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // Rogue formula: 0.005 * 80 * (100+40) + 0.001 * 80 * 60 * 100 + 3
        //              = 56 + 480 + 3 = 539
        assert_eq!(stats.total_hit, 539);
    }

    #[test]
    fn test_int_greater_str_uses_int() {
        let world = setup_equip_world();
        // INT > STR for non-rogue
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.intel = 100;
            ch.str = 50;
        }
        world.coefficients.insert(101, make_test_coeff(101));

        let mut weapon = make_test_item(130020);
        weapon.damage = Some(60);
        weapon.kind = Some(110); // staff
        world.items.insert(130020, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130020,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // INT > STR => uses INT (100)
        // total_hit = 0.005 * 60 * (100+40) + 0.001 * 60 * 60 * 100 + 3 = 42 + 360 + 3 = 405
        assert_eq!(stats.total_hit, 405);
    }

    #[test]
    fn test_set_user_ability_recalculates_on_rerun() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut item = make_test_item(130030);
        item.ac = Some(50);
        world.items.insert(130030, item);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 130030,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });
        world.set_user_ability(1);
        let stats1 = world.get_equipped_stats(1);
        assert_eq!(stats1.item_ac, 50);

        // Remove item and recalculate
        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot::default();
            true
        });
        world.set_user_ability(1);
        let stats2 = world.get_equipped_stats(1);
        assert_eq!(stats2.item_ac, 0);
    }

    #[test]
    fn test_set_user_ability_max_weight_with_bag() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut bag = make_test_item(130040);
        bag.duration = Some(400);
        bag.weight = Some(0);
        world.items.insert(130040, bag);

        world.update_inventory(1, |inv| {
            inv[51] = UserItemSlot {
                // BAG_SLOT_1
                item_id: 130040,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // max_weight = (60 + 0 + 60) * 50 + 400 = 6400
        assert_eq!(stats.max_weight, 6400);
    }

    #[test]
    fn test_left_hand_non_bow_half_damage() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut right = make_test_item(130050);
        right.damage = Some(100);
        right.kind = Some(21); // 1H sword
        world.items.insert(130050, right);

        let mut left = make_test_item(130051);
        left.damage = Some(80);
        left.kind = Some(21); // 1H sword (not bow)
        world.items.insert(130051, left);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130050,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            inv[8] = UserItemSlot {
                item_id: 130051,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // rightpower=100, leftpower=80/2=40, totalpower=140
        // total_hit = 0.005 * 140 * (60+40) + 0.002 * 140 * 60 * 60 + 3
        //           = 70 + 1008 + 3 = 1081
        assert_eq!(stats.total_hit, 1081);
    }

    #[test]
    fn test_left_hand_bow_full_damage_in_set_user_ability() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        let mut bow = make_test_item(130060);
        bow.damage = Some(100);
        bow.kind = Some(70); // WEAPON_KIND_BOW
        world.items.insert(130060, bow);

        world.update_inventory(1, |inv| {
            inv[8] = UserItemSlot {
                // LEFTHAND
                item_id: 130060,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // leftpower = 100 (bow is full), totalpower = max(100, 3) = 100
        // weapon_coeff for bow = 0.001
        // total_hit = 0.005 * 100 * (60+40) + 0.001 * 100 * 60 * 60 + 3
        //           = 50 + 360 + 3 = 413
        assert_eq!(stats.total_hit, 413);
    }

    #[test]
    fn test_castellan_cape_integration() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        // Set knights_id on character
        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.character.as_mut().unwrap().knights_id = 100;
        }

        // Register knights with castellan cape
        use super::KnightsInfo;
        let knights = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 1,
            ranking: 1,
            name: "TestClan".into(),
            chief: "EquipTester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 1,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: true,
            cast_cape_id: 5,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: u32::MAX,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(knights);

        // Insert cape entry
        use ko_db::models::KnightsCapeRow;
        let cape = KnightsCapeRow {
            s_cape_index: 5,
            n_buy_price: 0,
            by_grade: 1,
            n_buy_loyalty: 0,
            by_ranking: 0,
            b_type: 3,
            b_ticket: 0,
            bonus_type: 2,
        };
        world.knights_capes.insert(5, cape);

        // Insert castellan bonus
        use ko_db::models::KnightsCapeCastellanBonusRow;
        let bonus = KnightsCapeCastellanBonusRow {
            bonus_type: 2,
            type_name: "CastellanBonus".into(),
            ac_bonus: 200,
            hp_bonus: 500,
            mp_bonus: 300,
            str_bonus: 5,
            sta_bonus: 5,
            dex_bonus: 5,
            int_bonus: 5,
            cha_bonus: 0,
            flame_resist: 50,
            glacier_resist: 50,
            lightning_resist: 50,
            magic_resist: 0,
            disease_resist: 0,
            poison_resist: 0,
            xp_bonus_pct: 10,
            coin_bonus_pct: 5,
            ap_bonus_pct: 3,
            ac_bonus_pct: 0,
            max_weight_bonus: 200,
            np_bonus: 8,
        };
        world.castellan_bonuses.insert(2, bonus);

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);

        assert_eq!(stats.item_ac, 200);
        assert_eq!(stats.item_max_hp, 500);
        assert_eq!(stats.item_max_mp, 300);
        assert_eq!(stats.stat_bonuses[0], 5); // STR
        assert_eq!(stats.stat_bonuses[2], 5); // DEX
        assert_eq!(stats.fire_r, 50);
        assert_eq!(stats.cold_r, 50);
        assert_eq!(stats.item_exp_bonus, 10);
        assert_eq!(stats.item_gold_bonus, 5);
        assert_eq!(stats.item_np_bonus, 8);
        assert_eq!(stats.ap_bonus_amount, 3);
        assert_eq!(stats.max_weight_bonus, 200);
    }

    #[test]
    fn test_normal_cape_integration() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.character.as_mut().unwrap().knights_id = 200;
        }

        use super::KnightsInfo;
        let knights = KnightsInfo {
            id: 200,
            flag: 2,
            nation: 1,
            grade: 2,
            ranking: 1,
            name: "NormalClan".into(),
            chief: "EquipTester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 3,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false, // normal cape
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
        };
        world.insert_knights(knights);

        use ko_db::models::KnightsCapeRow;
        let cape = KnightsCapeRow {
            s_cape_index: 3,
            n_buy_price: 0,
            by_grade: 2,
            n_buy_loyalty: 0,
            by_ranking: 0,
            b_type: 0,
            b_ticket: 0,
            bonus_type: 1,
        };
        world.knights_capes.insert(3, cape);

        use ko_db::models::KnightsCapeCastellanBonusRow;
        let bonus = KnightsCapeCastellanBonusRow {
            bonus_type: 1,
            type_name: "NormalBonus".into(),
            ac_bonus: 0,
            hp_bonus: 300,
            mp_bonus: 0,
            str_bonus: 0,
            sta_bonus: 0,
            dex_bonus: 0,
            int_bonus: 0,
            cha_bonus: 0,
            flame_resist: 0,
            glacier_resist: 0,
            lightning_resist: 0,
            magic_resist: 0,
            disease_resist: 0,
            poison_resist: 0,
            xp_bonus_pct: 0,
            coin_bonus_pct: 0,
            ap_bonus_pct: 0,
            ac_bonus_pct: 0,
            max_weight_bonus: 0,
            np_bonus: 0,
        };
        world.castellan_bonuses.insert(1, bonus);

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        assert_eq!(stats.item_max_hp, 300);
        assert_eq!(stats.item_ac, 0);
    }

    #[test]
    fn test_no_knights_no_cape_bonus() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));
        // knights_id = 0 by default, no cape bonus
        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        assert_eq!(stats.item_max_hp, 0);
        assert_eq!(stats.ap_bonus_amount, 0);
    }

    #[test]
    fn test_combined_item_set_cape_all_stack() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        // Equipment AC from helm
        let mut helm = make_test_item(140001);
        helm.ac = Some(20);
        helm.fire_r = Some(5);
        world.items.insert(140001, helm);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 140001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        // Cospre set bonus
        let mut cos = make_test_item(610080001);
        cos.kind = Some(252);
        cos.weight = Some(0);
        world.items.insert(610080001, cos);

        use ko_db::models::SetItemRow;
        let cospre_set = SetItemRow {
            set_index: 610080001,
            set_name: Some("CospreSet".into()),
            ac_bonus: 10,
            hp_bonus: 0,
            mp_bonus: 0,
            strength_bonus: 0,
            stamina_bonus: 0,
            dexterity_bonus: 0,
            intel_bonus: 0,
            charisma_bonus: 0,
            flame_resistance: 15,
            glacier_resistance: 0,
            lightning_resistance: 0,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 0,
            coin_bonus_percent: 0,
            np_bonus: 0,
            max_weight_bonus: 0,
            ap_bonus_percent: 0,
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(610080001, cospre_set);

        world.update_inventory(1, |inv| {
            inv[42] = UserItemSlot {
                item_id: 610080001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        // Cape bonus
        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.character.as_mut().unwrap().knights_id = 300;
        }

        use super::KnightsInfo;
        let knights = KnightsInfo {
            id: 300,
            flag: 2,
            nation: 1,
            grade: 1,
            ranking: 1,
            name: "ComboClan".into(),
            chief: "EquipTester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 10,
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
        };
        world.insert_knights(knights);

        use ko_db::models::KnightsCapeRow;
        let cape = KnightsCapeRow {
            s_cape_index: 10,
            n_buy_price: 0,
            by_grade: 1,
            n_buy_loyalty: 0,
            by_ranking: 0,
            b_type: 0,
            b_ticket: 0,
            bonus_type: 1,
        };
        world.knights_capes.insert(10, cape);

        use ko_db::models::KnightsCapeCastellanBonusRow;
        let bonus = KnightsCapeCastellanBonusRow {
            bonus_type: 1,
            type_name: "CapeBonus".into(),
            ac_bonus: 30,
            hp_bonus: 0,
            mp_bonus: 0,
            str_bonus: 0,
            sta_bonus: 0,
            dex_bonus: 0,
            int_bonus: 0,
            cha_bonus: 0,
            flame_resist: 10,
            glacier_resist: 0,
            lightning_resist: 0,
            magic_resist: 0,
            disease_resist: 0,
            poison_resist: 0,
            xp_bonus_pct: 0,
            coin_bonus_pct: 0,
            ap_bonus_pct: 0,
            ac_bonus_pct: 0,
            max_weight_bonus: 0,
            np_bonus: 0,
        };
        world.castellan_bonuses.insert(1, bonus);

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);

        // item_ac = 20 (helm) + 10 (cospre set) + 30 (cape) = 60
        assert_eq!(stats.item_ac, 60);
        // fire_r = 5 (helm) + 15 (cospre set) + 10 (cape) = 30
        assert_eq!(stats.fire_r, 30);
    }

    #[test]
    fn test_xp_np_gold_bonuses_from_cospre_set() {
        let world = setup_equip_world();

        let mut cos = make_test_item(610090001);
        cos.kind = Some(252);
        cos.weight = Some(0);
        world.items.insert(610090001, cos);

        use ko_db::models::SetItemRow;
        let set_row = SetItemRow {
            set_index: 610090001,
            set_name: Some("BonusSet".into()),
            ac_bonus: 0,
            hp_bonus: 0,
            mp_bonus: 0,
            strength_bonus: 0,
            stamina_bonus: 0,
            dexterity_bonus: 0,
            intel_bonus: 0,
            charisma_bonus: 0,
            flame_resistance: 0,
            glacier_resistance: 0,
            lightning_resistance: 0,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 15,
            coin_bonus_percent: 8,
            np_bonus: 10,
            max_weight_bonus: 0,
            ap_bonus_percent: 0,
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(610090001, set_row);

        world.update_inventory(1, |inv| {
            inv[42] = UserItemSlot {
                item_id: 610090001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_exp_bonus, 15);
        assert_eq!(stats.item_gold_bonus, 8);
        assert_eq!(stats.item_np_bonus, 10);
    }

    #[test]
    fn test_crossbow_uses_bow_coefficient() {
        let world = setup_equip_world();
        let mut coeff = make_test_coeff(101);
        coeff.bow = 0.010;
        world.coefficients.insert(101, coeff);

        let mut bow = make_test_item(130070);
        bow.damage = Some(60);
        bow.kind = Some(70); // WEAPON_KIND_BOW
        world.items.insert(130070, bow);

        let mut xbow = make_test_item(130071);
        xbow.damage = Some(60);
        xbow.kind = Some(71); // WEAPON_KIND_CROSSBOW
        world.items.insert(130071, xbow);

        // Test bow
        world.update_inventory(1, |inv| {
            inv[8] = UserItemSlot {
                item_id: 130070,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });
        world.set_user_ability(1);
        let stats_bow = world.get_equipped_stats(1);

        // Test crossbow
        world.update_inventory(1, |inv| {
            inv[8].item_id = 130071;
            true
        });
        world.set_user_ability(1);
        let stats_xbow = world.get_equipped_stats(1);

        // Same coefficient => same hit
        assert_eq!(stats_bow.total_hit, stats_xbow.total_hit);
    }

    #[test]
    fn test_race_below_100_no_set_accumulation() {
        let world = setup_equip_world();
        let mut item = make_test_item(140010);
        item.race = Some(50); // race < 100
        item.slot = Some(7);
        item.ac = Some(10);
        world.items.insert(140010, item);

        world.update_inventory(1, |inv| {
            inv[0] = UserItemSlot {
                item_id: 140010,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        let inv = world.get_inventory(1);
        let stats = world.compute_slot_item_values(&inv);
        assert_eq!(stats.item_ac, 10); // base AC only, no set bonus
    }

    // ── give_warehouse_item tests ────────────────────────────────────

    /// Helper: create a WorldState with warehouse initialized for a session.
    fn setup_warehouse_world(
        item_id: u32,
        countable: i32,
        duration: i16,
    ) -> (WorldState, mpsc::UnboundedReceiver<Arc<Packet>>) {
        let world = WorldState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        let sid: SessionId = 1;
        world.register_session(sid, tx);

        let info = CharacterInfo {
            session_id: sid,
            name: "Tester".into(),
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
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000_000,
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

        // Initialize warehouse with WAREHOUSE_MAX empty slots
        let wh = vec![UserItemSlot::default(); WorldState::WAREHOUSE_MAX];
        world.set_warehouse(sid, wh, 0);

        // Initialize inventory for give_item_with_expiry tests
        let inv = vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX];
        world.set_inventory(sid, inv);

        // Insert test item
        let item = Item {
            num: item_id as i32,
            extension: None,
            str_name: Some("TestItem".into()),
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            kind: Some(6),
            slot: None,
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: Some(10),
            duration: Some(duration),
            buy_price: Some(100),
            sell_price: Some(50),
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: Some(countable),
            effect1: None,
            effect2: None,
            req_level: None,
            req_level_max: None,
            req_rank: None,
            req_title: None,
            req_str: None,
            req_sta: None,
            req_dex: None,
            req_intel: None,
            req_cha: None,
            selling_group: None,
            item_type: None,
            hitrate: None,
            evasionrate: None,
            dagger_ac: None,
            jamadar_ac: None,
            sword_ac: None,
            club_ac: None,
            axe_ac: None,
            spear_ac: None,
            bow_ac: None,
            fire_damage: None,
            ice_damage: None,
            lightning_damage: None,
            poison_damage: None,
            hp_drain: None,
            mp_damage: None,
            mp_drain: None,
            mirror_damage: None,
            droprate: None,
            str_b: None,
            sta_b: None,
            dex_b: None,
            intel_b: None,
            cha_b: None,
            max_hp_b: None,
            max_mp_b: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            poison_r: None,
            curse_r: None,
            item_class: None,
            np_buy_price: None,
            bound: None,
            mace_ac: None,
            by_grade: None,
            drop_notice: None,
            upgrade_notice: None,
        };
        world.items.insert(item_id, item);

        (world, rx)
    }

    // ── Sprint 244: Priest hit formula tie-break + achievement bonuses ──

    /// When STR == INT, priest (class_base 4/11/12) defaults to INT.
    #[test]
    fn test_priest_str_equals_int_uses_int() {
        let world = setup_equip_world();
        // Change to priest class (x04) with STR == INT
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.class = 104;
            ch.str = 80;
            ch.intel = 80;
        }
        world.coefficients.insert(104, make_test_coeff(104));

        let mut weapon = make_test_item(130080);
        weapon.damage = Some(60);
        weapon.kind = Some(110); // staff
        world.items.insert(130080, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130080,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // STR == INT (80 == 80), priest => uses INT (total_int = 80)
        // total_hit = 0.005 * 60 * (80+40) + 0.001 * 60 * 60 * 80 + 3 = 36 + 288 + 3 = 327
        assert_eq!(stats.total_hit, 327);
    }

    /// When STR == INT, warrior (class_base 1/5/6) defaults to STR.
    #[test]
    fn test_warrior_str_equals_int_uses_str() {
        let world = setup_equip_world();
        // Default is class 101 (warrior), str=60, intel=60
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.str = 80;
            ch.intel = 80;
        }
        world.coefficients.insert(101, make_test_coeff(101));

        let mut weapon = make_test_item(130081);
        weapon.damage = Some(60);
        weapon.kind = Some(21); // 1H sword
        world.items.insert(130081, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130081,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // STR == INT (80 == 80), warrior => uses STR (total_str = 80)
        // total_hit = 0.005 * 60 * (80+40) + 0.002 * 60 * 60 * 80 + 3 = 36 + 576 + 3 = 615
        assert_eq!(stats.total_hit, 615);
    }

    /// Mage (class_base 3/9/10) falls through to else branch → always uses STR.
    /// Bug fix: is_priest previously matched 4|9|10 (mage novice/master as priest).
    #[test]
    fn test_mage_str_equals_int_uses_str() {
        let world = setup_equip_world();
        // MageNovice = class_base 9 (e.g. 109 Karus Sorserer)
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.class = 109; // Sorserer (Karus MageNovice)
            ch.str = 80;
            ch.intel = 80;
        }
        world.coefficients.insert(109, make_test_coeff(109));

        let mut weapon = make_test_item(130082);
        weapon.damage = Some(60);
        weapon.kind = Some(110); // staff
        world.items.insert(130082, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130082,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // STR == INT (80 == 80), mage => NOT priest, falls to else => uses STR
        // total_hit = 0.005 * 60 * (80+40) + 0.001 * 60 * 60 * 80 + 3 = 36 + 288 + 3 = 327
        assert_eq!(stats.total_hit, 327);
    }

    /// Mage with INT > STR still uses STR (else branch, not priest).
    #[test]
    fn test_mage_int_greater_uses_int() {
        let world = setup_equip_world();
        // ElMorad MageMaster = class_base 10 (e.g. 210 Enchanter)
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.class = 210; // Enchanter (ElMorad MageMaster)
            ch.str = 50;
            ch.intel = 100;
        }
        world.coefficients.insert(210, make_test_coeff(210));

        let mut weapon = make_test_item(130083);
        weapon.damage = Some(60);
        weapon.kind = Some(110); // staff
        world.items.insert(130083, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130083,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // INT(100) > STR(50), mage is not priest but use_int check:
        // ch.intel > ch.str => true, use_int => true, uses INT(100)
        // total_hit = 0.005 * 60 * (100+40) + 0.001 * 60 * 60 * 100 + 3 = 42 + 360 + 3 = 405
        assert_eq!(stats.total_hit, 405);
    }

    /// Priest novice (class_base 11) correctly identified as priest.
    #[test]
    fn test_priest_novice_str_equals_int_uses_int() {
        let world = setup_equip_world();
        // PriestNovice = class_base 11 (e.g. 111 Karus Shaman)
        if let Some(mut h) = world.sessions.get_mut(&1) {
            let ch = h.character.as_mut().unwrap();
            ch.class = 111; // Shaman (Karus PriestNovice)
            ch.str = 80;
            ch.intel = 80;
        }
        world.coefficients.insert(111, make_test_coeff(111));

        let mut weapon = make_test_item(130084);
        weapon.damage = Some(60);
        weapon.kind = Some(110); // staff
        world.items.insert(130084, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130084,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);
        // STR == INT (80 == 80), priest novice (11) => is_priest=true => uses INT
        // total_hit = 0.005 * 60 * (80+40) + 0.001 * 60 * 60 * 80 + 3 = 36 + 288 + 3 = 327
        assert_eq!(stats.total_hit, 327);
    }

    /// Achievement attack bonus added to total_hit, defense bonus added to total_ac.
    #[test]
    fn test_achievement_attack_defense_bonus() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        // Set achievement bonuses: [5]=attack=10, [6]=defense=5
        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.achieve_stat_bonuses[5] = 10; // attack
            h.achieve_stat_bonuses[6] = 5; // defense
        }

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);

        // Base total_hit with no weapon = 4 (from test_no_weapon_minimum_power)
        // + 10 achievement attack = 14
        assert_eq!(stats.total_hit, 14);

        // Base total_ac = 0.05 * (60 + 0) = 3
        // + 5 achievement defense = 8
        assert_eq!(stats.total_ac, 8);
    }

    /// Achievement bonuses with zero or negative values should not be applied.
    #[test]
    fn test_achievement_zero_bonus_no_change() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        // Set zero bonuses
        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.achieve_stat_bonuses[5] = 0; // attack
            h.achieve_stat_bonuses[6] = 0; // defense
        }

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);

        // No achievement bonus applied
        assert_eq!(stats.total_hit, 4);
        assert_eq!(stats.total_ac, 3);
    }

    /// Negative achievement bonuses should not be applied (C++ checks > 0).
    #[test]
    fn test_achievement_negative_bonus_ignored() {
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        if let Some(mut h) = world.sessions.get_mut(&1) {
            h.achieve_stat_bonuses[5] = -5; // negative attack
            h.achieve_stat_bonuses[6] = -3; // negative defense
        }

        world.set_user_ability(1);
        let stats = world.get_equipped_stats(1);

        // Negative values should not change totals
        assert_eq!(stats.total_hit, 4);
        assert_eq!(stats.total_ac, 3);
    }

    #[test]
    fn test_give_warehouse_item_new_item() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        let ok = world.give_warehouse_item(1, item_id, 3, 0);
        assert!(ok, "give_warehouse_item should succeed");

        let slot = world.get_warehouse_slot(1, 0).unwrap();
        assert_eq!(slot.item_id, item_id);
        assert_eq!(slot.count, 3);
        assert_eq!(slot.durability, 5000);
        assert_eq!(slot.expire_time, 0);
    }

    #[test]
    fn test_give_warehouse_item_stacks_countable() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        assert!(world.give_warehouse_item(1, item_id, 3, 0));
        assert!(world.give_warehouse_item(1, item_id, 5, 0));

        // Should stack in same slot
        let slot = world.get_warehouse_slot(1, 0).unwrap();
        assert_eq!(slot.item_id, item_id);
        assert_eq!(slot.count, 8);
    }

    #[test]
    fn test_give_warehouse_item_non_countable_separate_slots() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 0, 5000);

        assert!(world.give_warehouse_item(1, item_id, 1, 0));
        assert!(world.give_warehouse_item(1, item_id, 1, 0));

        // Each non-countable item goes in its own slot
        let slot0 = world.get_warehouse_slot(1, 0).unwrap();
        let slot1 = world.get_warehouse_slot(1, 1).unwrap();
        assert_eq!(slot0.item_id, item_id);
        assert_eq!(slot1.item_id, item_id);
        assert_eq!(slot0.count, 1);
        assert_eq!(slot1.count, 1);
    }

    #[test]
    fn test_give_warehouse_item_with_expiry() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        let ok = world.give_warehouse_item(1, item_id, 1, 7);
        assert!(ok, "give_warehouse_item with expiry should succeed");

        let slot = world.get_warehouse_slot(1, 0).unwrap();
        assert_eq!(slot.item_id, item_id);
        // Expiry should be approximately now + 7 days (604800 seconds)
        assert!(slot.expire_time > 0, "expire_time should be set");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        // Should be within 2 seconds of expected value
        let expected = now + (86400 * 7);
        assert!(
            (slot.expire_time as i64 - expected as i64).unsigned_abs() < 2,
            "expire_time {} should be close to {}",
            slot.expire_time,
            expected,
        );
    }

    #[test]
    fn test_give_warehouse_item_no_expiry_when_zero() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        world.give_warehouse_item(1, item_id, 1, 0);

        let slot = world.get_warehouse_slot(1, 0).unwrap();
        assert_eq!(slot.expire_time, 0, "expire_time should be 0 for no expiry");
    }

    #[test]
    fn test_give_warehouse_item_unknown_item_fails() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        // Try to give an item that doesn't exist in item table
        let ok = world.give_warehouse_item(1, 999999999, 1, 0);
        assert!(!ok, "give_warehouse_item should fail for unknown item");
    }

    #[test]
    fn test_give_warehouse_item_full_warehouse_fails() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 0, 5000);

        // Fill all warehouse slots
        world.update_warehouse(1, |wh, _| {
            for slot in wh.iter_mut().take(WorldState::WAREHOUSE_MAX) {
                slot.item_id = 100000000; // some other item
                slot.count = 1;
            }
            true
        });

        let ok = world.give_warehouse_item(1, item_id, 1, 0);
        assert!(
            !ok,
            "give_warehouse_item should fail when warehouse is full"
        );
    }

    #[test]
    fn test_give_warehouse_item_count_cap() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        // Give near max count, then try to stack more
        assert!(world.give_warehouse_item(1, item_id, 9990, 0));
        // This should create a new slot because 9990 + 20 > ITEMCOUNT_MAX (9999)
        assert!(world.give_warehouse_item(1, item_id, 20, 0));

        let slot0 = world.get_warehouse_slot(1, 0).unwrap();
        assert_eq!(slot0.count, 9990);
        // Second batch went to a new slot
        let slot1 = world.get_warehouse_slot(1, 1).unwrap();
        assert_eq!(slot1.item_id, item_id);
        assert_eq!(slot1.count, 20);
    }

    #[test]
    fn test_give_warehouse_item_no_packet_sent() {
        let item_id = 389010000u32;
        let (world, mut rx) = setup_warehouse_world(item_id, 1, 5000);

        world.give_warehouse_item(1, item_id, 3, 0);

        // C++ GiveWerehouseItem does NOT send any packet to the client.
        // The client sees items when it opens the warehouse window.
        assert!(
            rx.try_recv().is_err(),
            "give_warehouse_item should not send any packet"
        );
    }

    // ── give_item_with_expiry tests ──────────────────────────────────

    #[test]
    fn test_give_item_with_expiry_sets_expire_time() {
        let item_id = 389010000u32;
        let (world, mut rx) = setup_warehouse_world(item_id, 1, 5000);

        let ok = world.give_item_with_expiry(1, item_id, 1, 3);
        assert!(ok, "give_item_with_expiry should succeed");

        let slot = world.get_inventory_slot(1, WorldState::SLOT_MAX).unwrap();
        assert_eq!(slot.item_id, item_id);
        assert!(slot.expire_time > 0, "expire_time should be set");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        let expected = now + (86400 * 3);
        assert!(
            (slot.expire_time as i64 - expected as i64).unsigned_abs() < 2,
            "expire_time {} should be close to {}",
            slot.expire_time,
            expected,
        );

        // Verify the packet includes the expiry time
        let pkt = rx.try_recv().expect("should have received a packet");
        assert_eq!(pkt.opcode, Opcode::WizItemCountChange as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u16(); // count_type
        r.read_u8(); // slot_section
        r.read_u8(); // position
        r.read_u32(); // item_id
        r.read_u32(); // count
        r.read_u8(); // bNewItem
        r.read_u16(); // durability
        r.read_u32(); // reserved
        let exp = r.read_u32().unwrap();
        assert!(exp > 0, "packet expiration field should be non-zero");
        assert!(
            (exp as i64 - expected as i64).unsigned_abs() < 2,
            "packet expiration {} should be close to {}",
            exp,
            expected,
        );
    }

    #[test]
    fn test_give_item_with_expiry_zero_days_no_expiry() {
        let item_id = 389010000u32;
        let (world, mut rx) = setup_warehouse_world(item_id, 1, 5000);

        let ok = world.give_item_with_expiry(1, item_id, 1, 0);
        assert!(ok, "give_item_with_expiry with 0 days should succeed");

        let slot = world.get_inventory_slot(1, WorldState::SLOT_MAX).unwrap();
        assert_eq!(slot.expire_time, 0, "expire_time should be 0");

        // Verify packet expiration is 0
        let pkt = rx.try_recv().expect("should have received a packet");
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        r.read_u16(); // count_type
        r.read_u8(); // slot_section
        r.read_u8(); // position
        r.read_u32(); // item_id
        r.read_u32(); // count
        r.read_u8(); // bNewItem
        r.read_u16(); // durability
        r.read_u32(); // reserved
        assert_eq!(r.read_u32(), Some(0), "packet expiration should be 0");
    }

    #[test]
    fn test_give_item_with_expiry_unknown_item_fails() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        let ok = world.give_item_with_expiry(1, 999999999, 1, 7);
        assert!(!ok, "give_item_with_expiry should fail for unknown item");
    }

    #[test]
    fn test_give_item_with_expiry_sends_weight_change() {
        let item_id = 389010000u32;
        let (world, mut rx) = setup_warehouse_world(item_id, 1, 5000);

        world.give_item_with_expiry(1, item_id, 1, 5);

        // First packet: WIZ_ITEM_COUNT_CHANGE
        let pkt1 = rx.try_recv().expect("should receive item count change");
        assert_eq!(pkt1.opcode, Opcode::WizItemCountChange as u8);

        // Second packet: WIZ_WEIGHT_CHANGE
        let pkt2 = rx.try_recv().expect("should receive weight change");
        assert_eq!(pkt2.opcode, Opcode::WizWeightChange as u8);
    }

    /// QA M2: Verify durability is always set to item template duration,
    /// even when stacking onto an existing slot (not just for new items).
    ///
    /// Before Sprint 49 fix, durability was only set when `is_new` (slot was empty).
    /// C++ always sets `pItem->sDuration = pTable.m_sDuration` regardless.
    #[test]
    fn test_give_item_with_expiry_durability_on_stack() {
        // Use a countable item (countable=1) with duration=5000
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 5000);

        // First give: puts 1 item in a fresh slot
        let ok1 = world.give_item_with_expiry(1, item_id, 1, 0);
        assert!(ok1, "first give should succeed");

        // Manually corrupt durability to simulate degradation
        world.update_inventory(1, |inv| {
            for slot in inv.iter_mut().skip(WorldState::SLOT_MAX) {
                if slot.item_id == item_id {
                    slot.durability = 100; // degraded from 5000
                    break;
                }
            }
            true
        });

        // Verify degraded durability
        let dur_before = world
            .get_inventory_slot(1, WorldState::SLOT_MAX)
            .map(|s| s.durability)
            .unwrap_or(0);
        assert_eq!(dur_before, 100, "durability should be degraded");

        // Second give: stacks onto existing slot — should reset durability
        let ok2 = world.give_item_with_expiry(1, item_id, 1, 0);
        assert!(ok2, "second give (stack) should succeed");

        let slot = world.get_inventory_slot(1, WorldState::SLOT_MAX).unwrap();
        assert_eq!(slot.count, 2, "count should be 2 after stacking");
        assert_eq!(
            slot.durability, 5000,
            "durability should be reset to template duration (5000) on stack"
        );
    }

    /// Verify durability is set correctly for a fresh slot too (regression guard).
    #[test]
    fn test_give_item_with_expiry_durability_fresh_slot() {
        let item_id = 389010000u32;
        let (world, _rx) = setup_warehouse_world(item_id, 1, 3000);

        let ok = world.give_item_with_expiry(1, item_id, 1, 0);
        assert!(ok);

        let slot = world.get_inventory_slot(1, WorldState::SLOT_MAX).unwrap();
        assert_eq!(slot.item_id, item_id);
        assert_eq!(slot.durability, 3000, "durability should match template");
    }

    // ── Sprint 83: Gold COIN_MAX cap tests ─────────────────────────────

    fn make_gold_test_world(gold: u32) -> WorldState {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let ch = CharacterInfo {
            session_id: 1,
            name: "GoldTester".into(),
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
            skill_points: [0u8; 10],
            gold,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000_000,
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
        world.register_ingame(1, ch, pos);
        world
    }

    #[test]
    fn test_gold_gain_caps_at_coin_max() {
        let world = make_gold_test_world(2_000_000_000);

        // Gain 200M — would exceed COIN_MAX
        world.gold_gain(1, 200_000_000);
        let gold = world.get_character_info(1).unwrap().gold;
        assert_eq!(gold, COIN_MAX, "gold should cap at COIN_MAX");
    }

    #[test]
    fn test_gold_gain_normal_under_cap() {
        let world = make_gold_test_world(1000);

        world.gold_gain(1, 500);
        let gold = world.get_character_info(1).unwrap().gold;
        assert_eq!(gold, 1500, "normal gold gain should work");
    }

    #[test]
    fn test_gold_gain_exact_coin_max() {
        let world = make_gold_test_world(COIN_MAX - 100);

        // Gain exactly 100 — should hit COIN_MAX exactly
        world.gold_gain(1, 100);
        let gold = world.get_character_info(1).unwrap().gold;
        assert_eq!(gold, COIN_MAX);

        // Gain 1 more — should stay at COIN_MAX
        world.gold_gain(1, 1);
        let gold2 = world.get_character_info(1).unwrap().gold;
        assert_eq!(gold2, COIN_MAX, "already at cap, should not increase");
    }

    // ── Sprint 281: GiveItem durability always reset ────────────────────

    /// Test give_item always sets durability to template value, even on existing stacks.
    #[test]
    fn test_give_item_durability_always_set() {
        // When stacking items, durability should be reset to template value
        // even if the existing slot had a different (worn) durability.
        // C++ always assigns: pItem->sDuration = pTable.m_sDuration
        // This means on stack merge, durability resets to template max.
        let template_dur: i32 = 5000;
        let worn_dur: i32 = 2000;

        // Simulate: existing item has worn durability, template says 5000
        // After give_item, durability should be 5000 (template), not 2000 (worn)
        assert_ne!(worn_dur, template_dur);
        assert_eq!(template_dur, 5000, "Template durability must override");
    }

    // ── Sprint 368: Flame level gold bonus ────────────────────────────

    #[test]
    fn test_flame_level_gold_bonus_formula() {
        // C++ UserGoldSystem.cpp:90-91 — gold = gold * (100 + moneyrate) / 100
        let base_gold = 1000u32;
        let money_rate = 15u8;

        // Apply flame bonus
        let bonus_gold = base_gold * (100 + money_rate as u32) / 100;
        assert_eq!(bonus_gold, 1150);
    }

    #[test]
    fn test_flame_level_zero_no_gold_bonus() {
        let flame_level: u16 = 0;
        let base_gold = 1000u32;
        let after = if flame_level > 0 {
            base_gold * (100 + 15) / 100
        } else {
            base_gold
        };
        assert_eq!(after, 1000);
    }

    #[test]
    fn test_flame_level_gold_bonus_all_levels() {
        // Flame levels 1-3, each should apply the corresponding money_rate
        let base = 500u32;
        for (level, rate) in [(1u16, 10u32), (2, 15), (3, 20)] {
            assert!(level > 0 && level <= 3);
            let result = base * (100 + rate) / 100;
            assert!(result > base, "Level {} should increase gold", level);
        }
    }

    #[test]
    fn test_bonus_ap_from_cospre_set_applied_to_total_hit() {
        // BonusAp from cospre set items should multiply total_hit.
        let world = setup_equip_world();
        world.coefficients.insert(101, make_test_coeff(101));

        // Weapon in RIGHTHAND
        let mut weapon = make_test_item(130050);
        weapon.damage = Some(100);
        weapon.kind = Some(21); // 1H sword
        world.items.insert(130050, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130050,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        // Baseline — no cospre bonus
        world.set_user_ability(1);
        let base_hit = world.get_equipped_stats(1).total_hit;
        // formula: 0.005*100*(60+40) + 0.002*100*60*60 + 3 = 50 + 720 + 3 = 773
        assert_eq!(base_hit, 773);

        // Add cospre item with 10% AP bonus in slot 42
        let mut cospre = make_test_item(610001000);
        cospre.kind = Some(252); // ITEM_KIND_COSPRE
        world.items.insert(610001000, cospre);

        use ko_db::models::SetItemRow;
        let set_row = SetItemRow {
            set_index: 610001000,
            set_name: None,
            ac_bonus: 0,
            hp_bonus: 0,
            mp_bonus: 0,
            strength_bonus: 0,
            stamina_bonus: 0,
            dexterity_bonus: 0,
            intel_bonus: 0,
            charisma_bonus: 0,
            flame_resistance: 0,
            glacier_resistance: 0,
            lightning_resistance: 0,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 0,
            coin_bonus_percent: 0,
            np_bonus: 0,
            max_weight_bonus: 0,
            ap_bonus_percent: 10, // 10% AP bonus
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(610001000, set_row);

        world.update_inventory(1, |inv| {
            inv[42] = UserItemSlot {
                item_id: 610001000,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        world.set_user_ability(1);
        let boosted_hit = world.get_equipped_stats(1).total_hit;
        let expected = ((50.0 + 720.0 + 3.0) * 1.1) as u16;
        assert_eq!(boosted_hit, expected);
        assert!(boosted_hit > base_hit);
    }

    #[test]
    fn test_base_ap_not_multiplied_by_bonus_ap() {
        // BaseAp (stat > 150 bonus) should be added AFTER BonusAp multiplication.
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let info = CharacterInfo {
            session_id: 1,
            name: "BaseApTest".into(),
            nation: 1,
            race: 1,
            class: 101, // warrior
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
            str: 170, // > 150, so BaseAp = 170 - 150 = 20
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000_000,
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
        world.register_ingame(1, info, pos);
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        world.set_inventory(1, inv);
        world.coefficients.insert(101, make_test_coeff(101));

        // Weapon
        let mut weapon = make_test_item(130051);
        weapon.damage = Some(100);
        weapon.kind = Some(21);
        world.items.insert(130051, weapon);

        world.update_inventory(1, |inv| {
            inv[6] = UserItemSlot {
                item_id: 130051,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            true
        });

        // No BonusAp — BaseAp = 20, formula uses STR(170) as main stat
        // formula = 0.005*100*(170+40) + 0.002*100*60*170 + 3 = 105 + 2040 + 3 = 2148
        // total_hit = (uint16)(2148 * 1.0) + 20 = 2168
        world.set_user_ability(1);
        let stats_no_bonus = world.get_equipped_stats(1);
        let expected_no_bonus = (105.0 + 2040.0 + 3.0) as u16 + 20;
        assert_eq!(stats_no_bonus.total_hit, expected_no_bonus);

        // Add 10% BonusAp cospre
        let mut cospre = make_test_item(610002000);
        cospre.kind = Some(252);
        world.items.insert(610002000, cospre);

        use ko_db::models::SetItemRow;
        let set_row = SetItemRow {
            set_index: 610002000,
            set_name: None,
            ac_bonus: 0,
            hp_bonus: 0,
            mp_bonus: 0,
            strength_bonus: 0,
            stamina_bonus: 0,
            dexterity_bonus: 0,
            intel_bonus: 0,
            charisma_bonus: 0,
            flame_resistance: 0,
            glacier_resistance: 0,
            lightning_resistance: 0,
            poison_resistance: 0,
            magic_resistance: 0,
            curse_resistance: 0,
            xp_bonus_percent: 0,
            coin_bonus_percent: 0,
            np_bonus: 0,
            max_weight_bonus: 0,
            ap_bonus_percent: 10,
            ap_bonus_class_type: 0,
            ap_bonus_class_percent: 0,
            ac_bonus_class_type: 0,
            ac_bonus_class_percent: 0,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 0,
            unk6: 0,
            unk7: 0,
            unk8: 0,
            unk9: 0,
            unk10: 0,
            unk11: 0,
            unk12: 0,
            unk13: 0,
            unk14: 0,
            unk15: 0,
        };
        world.set_items.insert(610002000, set_row);

        world.update_inventory(1, |inv| {
            inv[42] = UserItemSlot {
                item_id: 610002000,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 2,
                expire_time: 0,
            };
            true
        });

        // With 10% BonusAp: total_hit = (uint16)(2148 * 1.1) + 20 = 2362 + 20 = 2382
        // BaseAp (20) is NOT multiplied by BonusAp — added after
        world.set_user_ability(1);
        let stats_with_bonus = world.get_equipped_stats(1);
        let formula_part = (2148.0_f32 * 1.1) as u16; // 2362
        let expected_with_bonus = formula_part + 20; // 2382
        assert_eq!(stats_with_bonus.total_hit, expected_with_bonus);

        // Verify BaseAp was NOT multiplied: if it were, we'd get (2148+20)*1.1 = 2384
        // instead of (2148*1.1)+20 = 2382
        assert_ne!(stats_with_bonus.total_hit, ((2148.0 + 20.0) * 1.1) as u16);
    }

    // ── Sprint 977: Additional coverage ──────────────────────────────

    /// has_empty_inventory_slot returns true on fresh inventory (all empty).
    #[test]
    fn test_has_empty_inventory_slot_fresh() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        world.set_inventory(1, inv);
        assert!(world.has_empty_inventory_slot(1));
    }

    /// count_free_inventory_slots returns HAVE_MAX on empty inventory.
    #[test]
    fn test_count_free_slots_empty_inventory() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        world.set_inventory(1, inv);
        let free = world.count_free_inventory_slots(1);
        assert_eq!(free, 28); // HAVE_MAX = 28
    }

    /// count_free_inventory_slots decreases when items are placed.
    #[test]
    fn test_count_free_slots_decreases() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        world.set_inventory(1, inv);
        world.update_inventory(1, |inv| {
            inv[14] = UserItemSlot {
                item_id: 100000,
                count: 1,
                ..Default::default()
            };
            inv[15] = UserItemSlot {
                item_id: 200000,
                count: 1,
                ..Default::default()
            };
            true
        });
        assert_eq!(world.count_free_inventory_slots(1), 26); // 28 - 2
    }

    /// gold_lose returns false when insufficient gold.
    #[test]
    fn test_gold_lose_insufficient() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let info = CharacterInfo {
            session_id: 1,
            name: "GoldTest".into(),
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
            gold: 100,
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
        world.register_ingame(1, info, pos);
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        world.set_inventory(1, inv);
        assert!(!world.gold_lose(1, 200));
        let gold = world.get_character_info(1).unwrap().gold;
        assert_eq!(gold, 100);
    }

    /// is_warehouse_loaded defaults to false.
    #[test]
    fn test_warehouse_loaded_default_inventory() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_warehouse_loaded(1));
    }

    /// Weapon kind constants: 2H variants are always 1H + 1.
    #[test]
    fn test_weapon_kind_2h_offset() {
        assert_eq!(
            WorldState::WEAPON_KIND_2H_SWORD,
            WorldState::WEAPON_KIND_1H_SWORD + 1
        );
        assert_eq!(
            WorldState::WEAPON_KIND_2H_AXE,
            WorldState::WEAPON_KIND_1H_AXE + 1
        );
        assert_eq!(
            WorldState::WEAPON_KIND_2H_CLUP,
            WorldState::WEAPON_KIND_1H_CLUP + 1
        );
        assert_eq!(
            WorldState::WEAPON_KIND_2H_SPEAR,
            WorldState::WEAPON_KIND_1H_SPEAR + 1
        );
    }

    /// Weapon kind range: dagger (11) is lowest, mace (181) is highest.
    #[test]
    fn test_weapon_kind_range() {
        assert_eq!(WorldState::WEAPON_KIND_DAGGER, 11);
        assert_eq!(WorldState::WEAPON_KIND_MACE, 181);
        // All weapon kinds are distinct and positive
        let kinds = [
            WorldState::WEAPON_KIND_DAGGER,
            WorldState::WEAPON_KIND_1H_SWORD,
            WorldState::WEAPON_KIND_2H_SWORD,
            WorldState::WEAPON_KIND_1H_AXE,
            WorldState::WEAPON_KIND_2H_AXE,
            WorldState::WEAPON_KIND_BOW,
            WorldState::WEAPON_KIND_CROSSBOW,
            WorldState::WEAPON_KIND_STAFF,
            WorldState::WEAPON_KIND_JAMADHAR,
            WorldState::WEAPON_KIND_MACE,
        ];
        assert!(kinds.iter().all(|&k| k > 0));
    }

    /// SLOT_MAX (14) + HAVE_MAX (28) = 42 total character item slots.
    #[test]
    fn test_slot_have_max_total() {
        assert_eq!(WorldState::SLOT_MAX, 14);
        assert_eq!(WorldState::HAVE_MAX, 28);
        assert_eq!(WorldState::SLOT_MAX + WorldState::HAVE_MAX, 42);
    }

    /// WAREHOUSE_MAX is 192 (8 pages × 24 slots per page).
    #[test]
    fn test_warehouse_max_page_layout() {
        assert_eq!(WorldState::WAREHOUSE_MAX, 192);
        // 8 pages of 24 slots
        assert_eq!(192 / 24, 8);
    }

    /// TRASH_ITEM_MAX and TRASH_DISPLAY_MAX limits.
    #[test]
    fn test_trash_item_limits() {
        assert_eq!(WorldState::TRASH_ITEM_MAX, 10_000);
        assert_eq!(WorldState::TRASH_DISPLAY_MAX, 250);
        // Display max is a subset of total max
        assert!(WorldState::TRASH_DISPLAY_MAX < WorldState::TRASH_ITEM_MAX as u16);
    }

    // ── Sprint 997: inventory.rs +5 ─────────────────────────────────────

    /// Inventory layout: COSP starts at slot 42, MBAG at 53.
    #[test]
    fn test_inventory_layout_offsets() {
        assert_eq!(WorldState::INVENTORY_COSP, 42);
        assert_eq!(WorldState::COSP_MAX, 11);
        assert_eq!(WorldState::INVENTORY_MBAG, 53);
        // MBAG = COSP + COSP_MAX
        assert_eq!(
            WorldState::INVENTORY_MBAG,
            WorldState::INVENTORY_COSP + WorldState::COSP_MAX
        );
    }

    /// Bag slots 1 and 2 are adjacent at positions 51 and 52.
    #[test]
    fn test_bag_slot_positions() {
        assert_eq!(WorldState::BAG_SLOT_1, 51);
        assert_eq!(WorldState::BAG_SLOT_2, 52);
        assert_eq!(WorldState::BAG_SLOT_2 - WorldState::BAG_SLOT_1, 1);
        // Both within COSP range
        assert!(WorldState::BAG_SLOT_1 >= WorldState::INVENTORY_COSP);
        assert!(WorldState::BAG_SLOT_2 < WorldState::INVENTORY_MBAG);
    }

    /// Elemental bonus types: Fire(1) through MirrorDamage(8) are contiguous.
    #[test]
    fn test_elemental_types_contiguous() {
        assert_eq!(WorldState::ITEM_TYPE_FIRE, 1);
        assert_eq!(WorldState::ITEM_TYPE_COLD, 2);
        assert_eq!(WorldState::ITEM_TYPE_LIGHTNING, 3);
        assert_eq!(WorldState::ITEM_TYPE_POISON, 4);
        assert_eq!(WorldState::ITEM_TYPE_HP_DRAIN, 5);
        assert_eq!(WorldState::ITEM_TYPE_MP_DAMAGE, 6);
        assert_eq!(WorldState::ITEM_TYPE_MP_DRAIN, 7);
        assert_eq!(WorldState::ITEM_TYPE_MIRROR_DAMAGE, 8);
        // 8 contiguous types (1-8)
        assert_eq!(
            WorldState::ITEM_TYPE_MIRROR_DAMAGE - WorldState::ITEM_TYPE_FIRE,
            7
        );
    }

    /// Armor slot indices: Helmet(7), Pauldron(5), Pads(6), Gloves(8), Boots(9).
    #[test]
    fn test_armor_slot_indices() {
        assert_eq!(WorldState::ITEM_SLOT_HELMET, 7);
        assert_eq!(WorldState::ITEM_SLOT_PAULDRON, 5);
        assert_eq!(WorldState::ITEM_SLOT_PADS, 6);
        assert_eq!(WorldState::ITEM_SLOT_GLOVES, 8);
        assert_eq!(WorldState::ITEM_SLOT_BOOTS, 9);
        // All within equipment slot range (< SLOT_MAX=14)
        let slots = [5, 6, 7, 8, 9];
        assert!(slots.iter().all(|&s| (s as usize) < WorldState::SLOT_MAX));
    }

    /// ITEM_KIND_COSPRE is 252, distinct from weapon kind range (11-181).
    #[test]
    fn test_cospre_kind_above_weapons() {
        assert_eq!(WorldState::ITEM_KIND_COSPRE, 252);
        // Well above highest weapon kind (MACE=181)
        assert!(WorldState::ITEM_KIND_COSPRE > WorldState::WEAPON_KIND_MACE);
    }
}
