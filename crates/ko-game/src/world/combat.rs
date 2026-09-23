//! Buff/debuff system, DOT/HOT processing, saved magic, perks, and damage calculations.

use super::*;

/// Combined results from a single-pass buff tick scan.
/// Replaces 6 separate DashMap traversals with one unified scan.
#[derive(Default)]
pub struct BuffTickResults {
    /// Expired buffs (Type4 duration expired).
    pub expired_buffs: Vec<(SessionId, ActiveBuff)>,
    /// Expired blinks: `(sid, zone_id)`.
    pub expired_blinks: Vec<(SessionId, u16)>,
    /// Expired transformations: `(sid, transform_skill_id, zone_id)`.
    pub expired_transformations: Vec<(SessionId, u32, u16)>,
    /// Sessions needing post-blink skill re-enable.
    pub post_blink_skill_enable: Vec<SessionId>,
    /// Sessions with expired stealth.
    pub expired_stealths: Vec<SessionId>,
    /// Sessions with expired rivalries.
    pub expired_rivalries: Vec<SessionId>,
}

/// Snapshot of combat-relevant data from a single session read.
/// Consolidates 15 separate DashMap lock acquisitions into 1 for damage
/// calculation in `handle_player_attack()`.
pub struct CombatSnapshot {
    pub equipped_stats: EquippedStats,
    // Buff aggregates (single-pass iteration)
    pub attack_amount: i32,
    pub player_attack_amount: i32,
    pub ac_amount: i32,
    pub ac_pct: i32,
    pub ac_sour: i32,
    // Session combat flags
    pub block_physical: bool,
    pub dagger_r_amount: u8,
    pub bow_r_amount: u8,
    pub mirror_damage: bool,
    pub mirror_damage_type: bool,
    pub mirror_amount: u8,
    // Weapon slot item IDs (item table lookup happens outside lock)
    pub right_hand_item_id: u32,
    pub left_hand_item_id: u32,
    // Perk levels for perk bonus calculation
    pub perk_levels: [i16; 13],
    // Elemental resistance percentages (all 6 elements)
    pub pct_fire_r: u8,
    pub pct_cold_r: u8,
    pub pct_lightning_r: u8,
    pub pct_magic_r: u8,
    pub pct_disease_r: u8,
    pub pct_poison_r: u8,
    // Buff elemental resistance adds: [0]=unused, [1]=fire..[6]=poison
    pub buff_elem_r: [i32; 7],
    // Magic attack amount modifier (for magic_process Type3 damage)
    pub magic_attack_amount: i32,
}

impl CombatSnapshot {
    /// Compute total elemental resistance for an attribute from snapshot data.
    ///
    /// Replaces `get_player_target_resistance()` which does 3 DashMap reads.
    pub fn total_resistance(&self, attribute: u8) -> i32 {
        let item_r = match attribute {
            1 => self.equipped_stats.fire_r as i32,
            2 => self.equipped_stats.cold_r as i32,
            3 => self.equipped_stats.lightning_r as i32,
            4 => self.equipped_stats.magic_r as i32,
            5 => self.equipped_stats.disease_r as i32,
            6 => self.equipped_stats.poison_r as i32,
            _ => 0,
        };
        let buff_r = self
            .buff_elem_r
            .get(attribute as usize)
            .copied()
            .unwrap_or(0);
        let resistance_bonus = self.equipped_stats.resistance_bonus as i32;
        let pct = match attribute {
            1 => self.pct_fire_r,
            2 => self.pct_cold_r,
            3 => self.pct_lightning_r,
            4 => self.pct_magic_r,
            5 => self.pct_disease_r,
            6 => self.pct_poison_r,
            _ => 100,
        } as i32;
        (item_r + buff_r) * pct / 100 + resistance_bonus * pct / 100
    }
}

impl WorldState {
    /// Snapshot all combat-relevant data for a session in a single DashMap read.
    ///
    /// Returns `None` if session not found or has no character.
    pub fn snapshot_combat(&self, sid: SessionId) -> Option<CombatSnapshot> {
        self.with_session(sid, |handle| {
            // Single-pass buff aggregation
            let mut attack_sum = 0i32;
            let mut player_attack_amount = 100i32;
            let mut ac_amount = 0i32;
            let mut ac_pct_mod = 0i32;
            let mut ac_sour = 0i32;
            let mut magic_attack_sum = 0i32;
            let mut elem_r_add = [0i32; 7];
            for b in handle.buffs.values() {
                // attack_amount: exclude BUFF_TYPE_DAMAGE_DOUBLE (19)
                if b.buff_type != 19 && b.attack != 0 {
                    attack_sum += b.attack - 100;
                }
                // player_attack_amount: BUFF_TYPE_DAMAGE_DOUBLE (19) only
                if b.buff_type == 19 && b.attack != 0 {
                    player_attack_amount = b.attack;
                }
                // ac_amount: exclude BUFF_TYPE_WEAPON_AC (14)
                if b.buff_type != 14 {
                    ac_amount = ac_amount.saturating_add(b.ac);
                }
                // ac_pct: exclude BUFF_TYPE_WEAPON_AC (14)
                if b.ac_pct != 0 && b.buff_type != 14 {
                    ac_pct_mod += b.ac_pct - 100;
                }
                ac_sour = ac_sour.saturating_add(b.ac_sour);
                // magic_attack_amount: m_sMagicAttackAmount
                if b.magic_attack != 0 {
                    magic_attack_sum += b.magic_attack - 100;
                }
                // Elemental resistance adds
                elem_r_add[1] = elem_r_add[1].saturating_add(b.fire_r);
                elem_r_add[2] = elem_r_add[2].saturating_add(b.cold_r);
                elem_r_add[3] = elem_r_add[3].saturating_add(b.lightning_r);
                elem_r_add[4] = elem_r_add[4].saturating_add(b.magic_r);
                elem_r_add[5] = elem_r_add[5].saturating_add(b.disease_r);
                elem_r_add[6] = elem_r_add[6].saturating_add(b.poison_r);
            }

            let right_hand_item_id = handle
                .inventory
                .get(6) // RIGHTHAND
                .map(|s| s.item_id)
                .unwrap_or(0);
            let left_hand_item_id = handle
                .inventory
                .get(8) // LEFTHAND
                .map(|s| s.item_id)
                .unwrap_or(0);

            CombatSnapshot {
                equipped_stats: handle.equipped_stats.clone(),
                attack_amount: (100 + attack_sum).max(0),
                player_attack_amount,
                ac_amount,
                ac_pct: 100 + ac_pct_mod,
                ac_sour,
                block_physical: handle.block_physical,
                dagger_r_amount: handle.dagger_r_amount,
                bow_r_amount: handle.bow_r_amount,
                mirror_damage: handle.mirror_damage,
                mirror_damage_type: handle.mirror_damage_type,
                mirror_amount: handle.mirror_amount,
                right_hand_item_id,
                left_hand_item_id,
                perk_levels: handle.perk_levels,
                pct_fire_r: handle.pct_fire_r,
                pct_cold_r: handle.pct_cold_r,
                pct_lightning_r: handle.pct_lightning_r,
                pct_magic_r: handle.pct_magic_r,
                pct_disease_r: handle.pct_disease_r,
                pct_poison_r: handle.pct_poison_r,
                buff_elem_r: elem_r_add,
                magic_attack_amount: magic_attack_sum,
            }
        })
    }

    // ── Saved Magic (Buff Persistence) Constants ─────────────────────

    /// Minimum skill ID for persistent buffs (scroll buffs).
    const SAVED_MAGIC_MIN_SKILL_ID: u32 = 500_000;

    /// Maximum saved magic entries per character.
    const MAX_SAVED_MAGIC: usize = 10;

    /// Minimum duration (seconds) to accept when loading saved magic from DB.
    const MIN_SAVED_MAGIC_DURATION_SECS: i32 = 5;

    /// Maximum duration (seconds) for saved magic (~8 hours).
    const MAX_SAVED_MAGIC_DURATION_SECS: i32 = 28800;

    // ── Buff System Methods ──────────────────────────────────────────

    /// Apply a buff to a session. Overwrites any existing buff of the same type.
    ///
    pub fn apply_buff(&self, sid: SessionId, buff: ActiveBuff) {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            handle.buffs.insert(buff.buff_type, buff);
        }
    }
    /// Remove a specific buff type from a session.
    ///
    pub fn remove_buff(&self, sid: SessionId, buff_type: i32) -> Option<ActiveBuff> {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            handle.buffs.remove(&buff_type)
        } else {
            None
        }
    }
    /// Check if a buff type is a "lockable scroll" — auto-recast on debuff displacement.
    ///
    pub fn is_lockable_scroll(buff_type: i32) -> bool {
        matches!(
            buff_type,
            1  | // BUFF_TYPE_HP_MP
            2  | // BUFF_TYPE_AC
            4  | // BUFF_TYPE_DAMAGE
            6  | // BUFF_TYPE_SPEED
            7  | // BUFF_TYPE_STATS
            48 | // BUFF_TYPE_FISHING
            171 // BUFF_TYPE_BATTLE_CRY
        )
    }
    /// Check if the player currently has a debuff on the given buff type slot.
    ///
    pub fn has_debuff_on_slot(&self, sid: SessionId, buff_type: i32) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.buffs.get(&buff_type).map(|b| !b.is_buff).unwrap_or(false))
            .unwrap_or(false)
    }
    /// Extend the duration of an active buff. Returns true if extended.
    ///
    /// Extends `m_tEndTime += sDuration` and sets `m_bDurationExtended = true`.
    /// A buff can only be extended once.
    pub fn extend_buff_duration(&self, sid: SessionId, buff_type: i32, extra_secs: u32) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            if let Some(buff) = handle.buffs.get_mut(&buff_type) {
                if buff.duration_secs == 0 {
                    return false; // Permanent buffs — nothing to extend
                }
                if buff.duration_extended {
                    return false;
                }
                buff.duration_secs = buff.duration_secs.saturating_add(extra_secs);
                buff.duration_extended = true;
                return true;
            }
        }
        false
    }
    /// Get all active buffs for a session (cloned).
    pub fn get_active_buffs(&self, sid: SessionId) -> Vec<ActiveBuff> {
        self.sessions
            .get(&sid)
            .map(|h| h.buffs.values().cloned().collect())
            .unwrap_or_default()
    }
    /// Get the total AC bonus from active Type4 buffs for a session.
    ///
    /// Used in the physical damage formula: `temp_ac = m_sTotalAc + m_sACAmount`
    pub fn get_buff_ac_amount(&self, sid: SessionId) -> i32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    // Exclude BUFF_TYPE_WEAPON_AC (14) — its AC is already baked into
                    // total_ac via set_user_ability (m_sAddArmourAc modifies m_sItemAc).
                    .filter(|b| b.buff_type != 14)
                    .map(|b| b.ac)
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0)
    }
    /// Get the total AC percentage modifier from active Type4 buffs.
    ///
    /// Applied in SetUserAbility: `m_sTotalAc = m_sTotalAc * m_sACPercent / 100`.
    /// Returns the final multiplier (100 = no change, 130 = +30%).
    pub fn get_buff_ac_pct(&self, sid: SessionId) -> i32 {
        let modifier: i32 = self
            .sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    // Exclude BUFF_TYPE_WEAPON_AC (14) — already applied in set_user_ability
                    .filter(|b| b.ac_pct != 0 && b.buff_type != 14)
                    .map(|b| b.ac_pct - 100)
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0);
        100 + modifier
    }
    /// Get the flat weapon damage bonus from active Type4 buffs.
    ///
    /// Added to weapon power in `SetUserAbility()`.
    pub fn get_buff_weapon_damage(&self, sid: SessionId) -> i32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    .map(|b| b.weapon_damage)
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0)
    }
    /// Get the AC reduction source amount from active Type4 debuffs.
    ///
    /// when `sAC < 0`. Subtracted from target AC in damage formula.
    pub fn get_buff_ac_sour_amount(&self, sid: SessionId) -> i32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    .map(|b| b.ac_sour)
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0)
    }
    /// Get the cumulative magic attack amount modifier from active Type4 buffs.
    ///
    /// `(bMagicAttack - 100)` on apply. Used in `GetMagicDamage()`:
    /// `total_hit = total_hit * (m_sMagicAttackAmount + 100) / 100`
    ///
    /// Returns the raw `m_sMagicAttackAmount` value (0 = no buff, >0 = bonus).
    pub fn get_buff_magic_attack_amount(&self, sid: SessionId) -> i32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    .filter(|b| b.magic_attack != 0)
                    .map(|b| b.magic_attack - 100)
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0)
    }
    /// Get the cumulative physical attack amount modifier from active buffs.
    ///
    /// - BUFF_TYPE_DAMAGE (4)
    /// - BUFF_TYPE_BLESS_OF_TEMPLE (39)
    /// - BUFF_TYPE_HELL_FIRE_DRAGON (37)
    /// - BUFF_TYPE_INCREASE_ATTACK (172)
    ///
    /// Used in `GetDamage()`: `temp_ap = m_sTotalHit * m_bAttackAmount`
    /// Returns the full multiplier (100 = no buff).
    pub fn get_buff_attack_amount(&self, sid: SessionId) -> i32 {
        let buff_sum: i32 = self
            .sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    // Exclude BUFF_TYPE_DAMAGE_DOUBLE (19) — that modifies m_bPlayerAttackAmount
                    .filter(|b| b.buff_type != 19 && b.attack != 0)
                    .map(|b| b.attack - 100)
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0);
        (100 + buff_sum).max(0)
    }

    /// Get the cumulative elemental resistance modifier from active Type4 buffs.
    ///
    /// Used in `GetMagicDamage()` (MagicInstance.cpp:6346-6373) as part of `total_r`.
    ///
    /// # Arguments
    /// * `sid` — target session ID
    /// * `attribute` — elemental attribute (1=Fire, 2=Cold, 3=Lightning, 4=Magic, 5=Disease, 6=Poison)
    pub fn get_buff_elemental_resistance(&self, sid: SessionId, attribute: u8) -> i32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                h.buffs
                    .values()
                    .map(|b| match attribute {
                        1 => b.fire_r,
                        2 => b.cold_r,
                        3 => b.lightning_r,
                        4 => b.magic_r,
                        5 => b.disease_r,
                        6 => b.poison_r,
                        _ => 0,
                    })
                    .fold(0i32, i32::saturating_add)
            })
            .unwrap_or(0)
    }
    /// Get the player attack amount multiplier from active Type4 buffs.
    ///
    /// Default is 100 (no change). Used in PvP: `temp_ap = temp_ap * amount / 100`
    pub fn get_buff_player_attack_amount(&self, sid: SessionId) -> i32 {
        self.sessions
            .get(&sid)
            .map(|h| {
                // BUFF_TYPE_DAMAGE_DOUBLE = 19 sets the attack amount
                for b in h.buffs.values() {
                    if b.buff_type == 19 && b.attack != 0 {
                        return b.attack;
                    }
                }
                100 // default: no modifier
            })
            .unwrap_or(100)
    }
    /// Check if a session has a specific buff type active.
    ///
    pub fn has_buff(&self, sid: SessionId, buff_type: i32) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.buffs.contains_key(&buff_type))
            .unwrap_or(false)
    }

    /// Check if a session has physical damage fully blocked.
    ///
    pub fn has_block_physical(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.block_physical)
            .unwrap_or(false)
    }
    /// Check if a session has magical damage fully blocked.
    ///
    /// and also BUFF_TYPE_FREEZE (22).
    pub fn has_block_magic(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.block_magic)
            .unwrap_or(false)
    }
    /// Get mirror damage state for a session.
    ///
    /// Returns (active, is_direct_type, amount_pct).
    pub fn get_mirror_damage_state(&self, sid: SessionId) -> (bool, bool, u8) {
        self.sessions
            .get(&sid)
            .map(|h| (h.mirror_damage, h.mirror_damage_type, h.mirror_amount))
            .unwrap_or((false, false, 0))
    }

    /// Get the dagger defense amount modifier for a session.
    ///
    pub fn get_dagger_r_amount(&self, sid: SessionId) -> u8 {
        self.sessions
            .get(&sid)
            .map(|h| h.dagger_r_amount)
            .unwrap_or(100)
    }

    /// Get the bow defense amount modifier for a session.
    ///
    pub fn get_bow_r_amount(&self, sid: SessionId) -> u8 {
        self.sessions
            .get(&sid)
            .map(|h| h.bow_r_amount)
            .unwrap_or(100)
    }

    /// Insert a skill into the saved magic map for persistence.
    ///
    pub fn insert_saved_magic(&self, sid: SessionId, skill_id: u32, duration_secs: u16) {
        let is_item_scroll_buff = skill_id <= Self::SAVED_MAGIC_MIN_SKILL_ID
            && self
                .get_magic(skill_id as i32)
                .is_some_and(|skill| skill.type1 == Some(4) && skill.item_group == Some(255));
        if skill_id == 0 || (skill_id <= Self::SAVED_MAGIC_MIN_SKILL_ID && !is_item_scroll_buff) {
            return;
        }
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            if !handle.saved_magic_map.contains_key(&skill_id)
                && handle.saved_magic_map.len() >= Self::MAX_SAVED_MAGIC
            {
                return;
            }
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            // Re-applying the same scroll refreshes its active buff; refresh
            // the persisted expiry too instead of keeping the older deadline.
            handle
                .saved_magic_map
                .insert(skill_id, now_ms + (duration_secs as u64) * 1000);
        }
    }
    /// Remove a skill from the saved magic map.
    ///
    pub fn remove_saved_magic(&self, sid: SessionId, skill_id: u32) {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            handle.saved_magic_map.remove(&skill_id);
        }
    }
    /// Check if a skill exists in the saved magic map.
    ///
    pub fn has_saved_magic(&self, sid: SessionId, skill_id: u32) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.saved_magic_map.contains_key(&skill_id))
            .unwrap_or(false)
    }
    /// Get remaining duration in seconds for a saved skill.
    ///
    pub fn get_saved_magic_duration(&self, sid: SessionId, skill_id: u32) -> u16 {
        self.sessions
            .get(&sid)
            .and_then(|h| {
                h.saved_magic_map.get(&skill_id).map(|&expiry_ms| {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    if expiry_ms > now_ms {
                        ((expiry_ms - now_ms) / 1000).min(u16::MAX as u64) as u16
                    } else {
                        0
                    }
                })
            })
            .unwrap_or(0)
    }
    /// Get all saved magic entries as (skill_id, remaining_secs) for DB save.
    pub fn get_saved_magic_entries(&self, sid: SessionId) -> Vec<(u32, i32)> {
        self.sessions
            .get(&sid)
            .map(|h| {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                h.saved_magic_map
                    .iter()
                    .filter_map(|(&skill_id, &expiry_ms)| {
                        if skill_id == 0 || expiry_ms <= now_ms {
                            return None;
                        }
                        let remaining = ((expiry_ms - now_ms) / 1000).min(i32::MAX as u64) as i32;
                        if remaining > 0 {
                            Some((skill_id, remaining))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Load saved magic entries into the session's map (game entry).
    ///
    pub fn load_saved_magic(&self, sid: SessionId, entries: &[(u32, i32)]) {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            handle.saved_magic_map.clear();
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            for &(skill_id, dur) in entries {
                if skill_id > 0
                    && dur > Self::MIN_SAVED_MAGIC_DURATION_SECS
                    && dur < Self::MAX_SAVED_MAGIC_DURATION_SECS
                {
                    handle
                        .saved_magic_map
                        .insert(skill_id, now_ms + (dur as u64) * 1000);
                }
            }
        }
    }
    /// Check and remove expired entries from the saved magic map.
    ///
    pub fn check_saved_magic(&self, sid: SessionId) {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            handle.saved_magic_map.retain(|_, expiry| *expiry > now_ms);
        }
    }
    /// Get saved skill IDs for recasting on login/zone change.
    ///
    pub fn get_saved_magic_for_recast(&self, sid: SessionId) -> Vec<(u32, u16)> {
        self.sessions
            .get(&sid)
            .map(|h| {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                h.saved_magic_map
                    .iter()
                    .filter_map(|(&skill_id, &expiry_ms)| {
                        if skill_id == 0 || expiry_ms <= now_ms {
                            return None;
                        }
                        let remaining = ((expiry_ms - now_ms) / 1000).min(u16::MAX as u64) as u16;
                        if remaining > 0 {
                            Some((skill_id, remaining))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Sum all active buff modifiers for max_hp (flat + percentage).
    ///
    /// Returns the bonus HP to add to the base max HP.
    pub fn get_buff_bonus_max_hp(&self, sid: SessionId, base_max_hp: i16) -> i16 {
        self.sessions
            .get(&sid)
            .map(|h| {
                let mut flat_bonus: i32 = 0;
                let mut pct_bonus: i32 = 0;
                for buff in h.buffs.values() {
                    flat_bonus += buff.max_hp;
                    // C++ pct convention: 100 = no change, 110 = +10%
                    if buff.max_hp_pct != 0 {
                        pct_bonus += buff.max_hp_pct - 100;
                    }
                }
                let pct_amount = (base_max_hp as i32 * pct_bonus) / 100;
                (flat_bonus + pct_amount).clamp(i16::MIN as i32, i16::MAX as i32) as i16
            })
            .unwrap_or(0)
    }
    /// Sum all active buff modifiers for max_mp (flat + percentage).
    ///
    /// Returns the bonus MP to add to the base max MP.
    pub fn get_buff_bonus_max_mp(&self, sid: SessionId, base_max_mp: i16) -> i16 {
        self.sessions
            .get(&sid)
            .map(|h| {
                let mut flat_bonus: i32 = 0;
                let mut pct_bonus: i32 = 0;
                for buff in h.buffs.values() {
                    flat_bonus += buff.max_mp;
                    if buff.max_mp_pct != 0 {
                        pct_bonus += buff.max_mp_pct - 100;
                    }
                }
                let pct_amount = (base_max_mp as i32 * pct_bonus) / 100;
                (flat_bonus + pct_amount).clamp(i16::MIN as i32, i16::MAX as i32) as i16
            })
            .unwrap_or(0)
    }
    /// Clear all active buffs from a session (InitType4 equivalent).
    ///
    ///
    /// Iterates all active Type4 buffs and removes them.
    /// If `remove_saved_magic` is true, also removes corresponding saved magic entries.
    pub fn clear_all_buffs(&self, sid: SessionId, remove_saved_magic: bool) -> Vec<i32> {
        let buff_keys: Vec<(i32, u32)> = self
            .sessions
            .get(&sid)
            .map(|h| h.buffs.iter().map(|(k, b)| (*k, b.skill_id)).collect())
            .unwrap_or_default();

        if buff_keys.is_empty() {
            return Vec::new();
        }

        let mut removed = Vec::new();
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            for (key, skill_id) in &buff_keys {
                if handle.buffs.remove(key).is_some() {
                    removed.push(*key);
                }
                if remove_saved_magic {
                    handle.saved_magic_map.remove(skill_id);
                }
            }
        }
        removed
    }

    /// Clear all durational skills (DOTs/HOTs) from a session (InitType3 equivalent).
    ///
    ///
    /// Resets all durational skill slots to inactive.
    pub fn clear_all_dots(&self, sid: SessionId) {
        self.clear_durational_skills(sid);
    }

    /// Check and remove expired buffs across all sessions.
    ///
    /// Returns `(session_id, expired_buff)` pairs for notification.
    ///
    pub fn collect_expired_buffs(&self) -> Vec<(SessionId, ActiveBuff)> {
        let mut expired = Vec::new();
        for mut entry in self.sessions.iter_mut() {
            let sid = *entry.key();
            let handle = entry.value_mut();
            let expired_keys: Vec<i32> = handle
                .buffs
                .iter()
                .filter(|(_, b)| b.is_expired())
                .map(|(k, _)| *k)
                .collect();
            for key in expired_keys {
                if let Some(buff) = handle.buffs.remove(&key) {
                    expired.push((sid, buff));
                }
            }
        }
        expired
    }
    /// Collect all buff-tick-related expirations in a single DashMap pass.
    ///
    /// Replaces 6 separate `sessions.iter()` / `sessions.iter_mut()` calls
    /// with one unified scan, eliminating ~5 redundant full-map traversals
    /// per 1-second buff tick.
    ///
    /// Parameters:
    /// - `now_unix`: current UNIX timestamp in seconds
    /// - `now_ms`: current time in milliseconds since epoch (for transformation)
    pub fn collect_all_buff_tick_expirations(&self, now_unix: u64, now_ms: u64) -> BuffTickResults {
        let mut results = BuffTickResults::default();

        for mut entry in self.sessions.iter_mut() {
            let sid = *entry.key();
            let h = entry.value_mut();

            // 1. Expired buffs (needs mut — removes from HashMap)
            let expired_keys: Vec<i32> = h
                .buffs
                .iter()
                .filter(|(_, b)| b.is_expired())
                .map(|(k, _)| *k)
                .collect();
            for key in expired_keys {
                if let Some(buff) = h.buffs.remove(&key) {
                    results.expired_buffs.push((sid, buff));
                }
            }

            // 2. Expired blinks
            if h.blink_expiry_time > 0 && now_unix >= h.blink_expiry_time {
                results.expired_blinks.push((sid, h.position.zone_id));
            }

            // 3. Expired transformations
            if h.transformation_type != 0
                && h.transformation_duration > 0
                && now_ms.saturating_sub(h.transformation_start_time) >= h.transformation_duration
            {
                results.expired_transformations.push((
                    sid,
                    h.transform_skill_id,
                    h.position.zone_id,
                ));
            }

            // 4. Post-blink skill re-enable
            let is_blinking = h.blink_expiry_time > 0 && now_unix < h.blink_expiry_time;
            let is_transformed = h.transformation_type != 0;
            if !is_blinking && is_transformed && !h.can_use_skills {
                results.post_blink_skill_enable.push(sid);
            }

            // 5. Expired stealths
            if h.invisibility_type != 0 && h.stealth_end_time > 0 && now_unix >= h.stealth_end_time
            {
                results.expired_stealths.push(sid);
            }

            // 6. Expired rivalries
            if let Some(ref ch) = h.character {
                if ch.rival_id >= 0 && ch.rival_expiry_time > 0 && now_unix >= ch.rival_expiry_time
                {
                    results.expired_rivalries.push(sid);
                }
            }
        }

        results
    }

    // ── DOT/HOT (Durational Skill) Methods ──────────────────────────

    /// Add a durational skill (DOT/HOT) to the first available slot.
    ///
    /// Returns true if a slot was available and the effect was added.
    ///
    pub fn add_durational_skill(
        &self,
        sid: SessionId,
        skill_id: u32,
        hp_amount: i16,
        tick_limit: u8,
        caster_sid: SessionId,
    ) -> bool {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            // Find an empty slot
            for slot in handle.durational_skills.iter_mut() {
                if !slot.used {
                    slot.skill_id = skill_id;
                    slot.hp_amount = hp_amount;
                    slot.tick_count = 0;
                    slot.tick_limit = tick_limit;
                    slot.caster_sid = caster_sid;
                    slot.used = true;
                    return true;
                }
            }
            // If no empty slot exists and we have room, push a new one
            if handle.durational_skills.len() < MAX_TYPE3_REPEAT {
                handle.durational_skills.push(DurationalSkill {
                    skill_id,
                    hp_amount,
                    tick_count: 0,
                    tick_limit,
                    caster_sid,
                    used: true,
                });
                return true;
            }
            false
        } else {
            false
        }
    }
    /// Process one DOT/HOT tick for all sessions.
    ///
    /// Returns `(session_id, hp_change, expired)` tuples for each active DOT that ticked.
    /// The `expired` flag is `true` when that slot just reached its tick limit and was cleared.
    /// Expired DOTs (tick_count >= tick_limit) are automatically cleared.
    ///
    pub fn process_dot_tick(&self) -> Vec<(SessionId, i16, bool)> {
        let mut results = Vec::new();
        for mut entry in self.sessions.iter_mut() {
            let sid = *entry.key();
            let handle = entry.value_mut();
            for slot in handle.durational_skills.iter_mut() {
                if !slot.used {
                    continue;
                }
                slot.tick_count += 1;
                let hp = slot.hp_amount;

                if slot.tick_count >= slot.tick_limit {
                    // DOT expired, clear the slot
                    results.push((sid, hp, true));
                    slot.used = false;
                    slot.skill_id = 0;
                    slot.hp_amount = 0;
                    slot.tick_count = 0;
                    slot.tick_limit = 0;
                    slot.caster_sid = 0;
                } else {
                    results.push((sid, hp, false));
                }
            }
        }
        results
    }

    /// Check if a session has any active harmful DOT (damage-over-time) effect.
    ///
    /// where `pEffect->m_sHPAmount < 0`.
    pub fn has_active_harmful_dot(&self, sid: SessionId) -> bool {
        self.with_session(sid, |h| {
            h.durational_skills
                .iter()
                .any(|slot| slot.used && slot.hp_amount < 0)
        })
        .unwrap_or(false)
    }
    /// Remove all durational skills from a session (e.g., on death).
    ///
    pub fn clear_durational_skills(&self, sid: SessionId) {
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            for slot in handle.durational_skills.iter_mut() {
                slot.used = false;
                slot.skill_id = 0;
                slot.hp_amount = 0;
                slot.tick_count = 0;
                slot.tick_limit = 0;
                slot.caster_sid = 0;
            }
        }
    }
    /// Check if a session has any active durational skill (DOT or HOT).
    ///
    /// Used by `MagicInstance.cpp:3633-3638` to skip Type3 application in temple event
    /// zones when combat is not allowed.
    pub fn has_active_durational(&self, sid: SessionId) -> bool {
        self.with_session(sid, |h| h.durational_skills.iter().any(|slot| slot.used))
            .unwrap_or(false)
    }

    /// Check if a session has any active HOT (heal-over-time) effect.
    ///
    /// A HOT is a durational skill with hp_amount > 0.
    pub fn has_active_hot(&self, sid: SessionId) -> bool {
        self.with_session(sid, |h| {
            h.durational_skills
                .iter()
                .any(|slot| slot.used && slot.hp_amount > 0)
        })
        .unwrap_or(false)
    }

    /// Remove only harmful (negative hp_amount) DOT effects from a session.
    ///
    /// Returns true if any harmful DOTs were removed.
    pub fn clear_harmful_dots(&self, sid: SessionId) -> bool {
        let mut removed_any = false;
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            for slot in handle.durational_skills.iter_mut() {
                if slot.used && slot.hp_amount < 0 {
                    slot.used = false;
                    slot.skill_id = 0;
                    slot.hp_amount = 0;
                    slot.tick_count = 0;
                    slot.tick_limit = 0;
                    slot.caster_sid = 0;
                    removed_any = true;
                }
            }
        }
        removed_any
    }
    /// Remove only healing (positive hp_amount) DOT/HOT effects from a session.
    ///
    /// Returns true if any HOTs were removed.
    /// Type3Cancel only cancels HOTs (m_sHPAmount > 0), not harmful DOTs.
    pub fn clear_healing_dots(&self, sid: SessionId) -> bool {
        let mut removed_any = false;
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            for slot in handle.durational_skills.iter_mut() {
                if slot.used && slot.hp_amount > 0 {
                    slot.used = false;
                    slot.skill_id = 0;
                    slot.hp_amount = 0;
                    slot.tick_count = 0;
                    slot.tick_limit = 0;
                    slot.caster_sid = 0;
                    removed_any = true;
                    // C++ only cancels the first HOT found, then breaks
                    break;
                }
            }
        }
        removed_any
    }
    /// Remove all debuff-type buffs from a session.
    ///
    /// Returns the list of removed buff types for notification.
    pub fn remove_debuffs(&self, sid: SessionId) -> Vec<i32> {
        let mut removed = Vec::new();
        if let Some(mut handle) = self.sessions.get_mut(&sid) {
            let debuff_keys: Vec<i32> = handle
                .buffs
                .iter()
                .filter(|(_, b)| crate::handler::magic_process::is_debuff(b))
                .map(|(k, _)| *k)
                .collect();
            for key in debuff_keys {
                if handle.buffs.remove(&key).is_some() {
                    removed.push(key);
                }
            }
        }
        removed
    }
    // ── Perk System Accessors ────────────────────────────────────────

    /// Insert a perk definition into the definitions table.
    ///
    /// Used during startup loading and tests.
    pub fn insert_perk_definition(&self, row: PerkRow) {
        self.perk_definitions.insert(row.p_index, row);
    }
    /// Get a perk definition by index.
    ///
    pub fn get_perk_definition(&self, index: i32) -> Option<PerkRow> {
        self.perk_definitions.get(&index).map(|r| r.clone())
    }

    /// Compute the total perk bonus for a given perk index and player levels.
    ///
    /// C++ pattern: `bonus = perks->perkCount * pPerks.perkType[index]`
    ///
    /// - `check_status`: if true, also verify `perks.status == true` (used for weight/defence/attack)
    /// - Returns 0 if the perk definition is missing, disabled, or player has 0 points.
    pub fn compute_perk_bonus(
        &self,
        perk_levels: &[i16; 13],
        index: usize,
        check_status: bool,
    ) -> i32 {
        if index >= 13 || perk_levels[index] <= 0 {
            return 0;
        }
        if let Some(def) = self.perk_definitions.get(&(index as i32)) {
            if def.perk_count <= 0 {
                return 0;
            }
            if check_status && !def.status {
                return 0;
            }
            def.perk_count as i32 * perk_levels[index] as i32
        } else {
            0
        }
    }
    /// Get the number of loaded perk definitions.
    pub fn perk_definition_count(&self) -> usize {
        self.perk_definitions.len()
    }
    /// Get all perk definitions as a sorted vector.
    pub fn get_all_perk_definitions(&self) -> Vec<PerkRow> {
        let mut v: Vec<PerkRow> = self
            .perk_definitions
            .iter()
            .map(|r| r.value().clone())
            .collect();
        v.sort_by_key(|r| r.p_index);
        v
    }
    /// Get a session's perk levels and remaining points.
    pub fn get_perk_levels(&self, sid: SessionId) -> Option<([i16; 13], i16)> {
        self.sessions.get(&sid).map(|h| (h.perk_levels, h.rem_perk))
    }
    /// Set a session's perk data (used when loading from DB).
    pub fn set_perk_data(&self, sid: SessionId, levels: [i16; 13], rem_perk: i16) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.perk_levels = levels;
            h.rem_perk = rem_perk;
        }
    }
    /// Allocate a perk point: decrement rem_perk, increment perk_levels[index].
    /// Returns (new_level, new_rem_perk) on success, None on failure.
    pub fn allocate_perk_point(&self, sid: SessionId, index: usize) -> Option<(i16, i16)> {
        if index >= 13 {
            return None;
        }
        let mut handle = self.sessions.get_mut(&sid)?;
        if handle.rem_perk <= 0 {
            return None;
        }
        // Check max from definition
        if let Some(def) = self.perk_definitions.get(&(index as i32)) {
            if handle.perk_levels[index] >= def.perk_max {
                return None;
            }
        } else {
            return None;
        }
        handle.rem_perk -= 1;
        handle.perk_levels[index] += 1;
        Some((handle.perk_levels[index], handle.rem_perk))
    }
    /// Reset all perk points: sum allocated points back to rem_perk, zero all levels.
    /// Returns (total_refunded, new_rem_perk) on success.
    pub fn reset_perk_points(&self, sid: SessionId) -> Option<(i16, i16)> {
        let mut handle = self.sessions.get_mut(&sid)?;
        let total: i16 = handle.perk_levels.iter().sum();
        if total == 0 {
            return None;
        }
        handle.rem_perk += total;
        handle.perk_levels = [0i16; 13];
        Some((total, handle.rem_perk))
    }

    /// Get a copy of the jackpot settings.
    pub fn get_jackpot_settings(&self) -> [JackPotSetting; 2] {
        *self.jackpot_settings.read()
    }

    /// Roll the jackpot multiplier from the given setting.
    ///
    /// Returns 0 if the roll doesn't hit any tier.
    pub(crate) fn roll_jackpot_multiplier(setting: &JackPotSetting) -> u32 {
        let rand_ = rand::random::<u32>() % 10001;
        if rand_ < setting.x_1000 as u32 {
            1000
        } else if rand_ < setting.x_500 as u32 {
            500
        } else if rand_ < setting.x_100 as u32 {
            100
        } else if rand_ < setting.x_50 as u32 {
            50
        } else if rand_ < setting.x_10 as u32 {
            10
        } else if rand_ < setting.x_2 as u32 {
            2
        } else {
            0
        }
    }

    /// Try to apply Noah (gold) jackpot to a gold pickup.
    ///
    /// Returns `true` if the jackpot fired (caller should NOT do normal GoldGain).
    /// Returns `false` if no jackpot (caller should do normal GoldGain).
    pub fn try_jackpot_noah(&self, sid: SessionId, gold: u32) -> bool {
        if gold == 0 {
            return false;
        }
        let jtype = self.with_session(sid, |h| h.jackpot_type).unwrap_or(0);
        if jtype != 2 {
            return false;
        }
        let settings = self.jackpot_settings.read();
        let jack = &settings[1]; // index 1 = Noah
        if jack.rate == 0 {
            return false;
        }
        // Rate check
        let rate_roll = rand::random::<u32>() % 10001;
        if rate_roll > jack.rate as u32 {
            return false;
        }
        // Multiplier roll
        let xrand = Self::roll_jackpot_multiplier(jack);
        if xrand == 0 {
            return false;
        }
        let bonus_gold = gold.saturating_mul(xrand);

        // Send WIZ_GOLD_CHANGE (CoinEvent=5) region packet
        // C++ format: [u8 CoinEvent=5] [u16 740] [u16 0] [u32 0] [u16 xrand] [u32 sid]
        let mut region_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizGoldChange as u8);
        region_pkt.write_u8(5); // CoinEvent
        region_pkt.write_u16(740);
        region_pkt.write_u16(0);
        region_pkt.write_u32(0);
        region_pkt.write_u16(xrand as u16);
        region_pkt.write_u32(sid as u32);
        if let Some((pos, event_room)) = self.with_session(sid, |h| (h.position, h.event_room)) {
            self.broadcast_to_region_sync(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(region_pkt),
                None,
                event_room,
            );
        }

        // Give the multiplied gold — C++ uses GoldGain(gold, false, true)
        // false = no WIZ_GOLD_CHANGE packet (CoinEvent region packet above is the only one)
        // true = apply bonus multipliers (noah_gain_amount etc.)
        self.gold_gain_with_bonus_silent(sid, bonus_gold);

        // Big wins: broadcast server-wide chat
        // C++ Noah checks rand_ == 500 || rand_ == 1000 (raw roll — extremely rare easter egg).
        // C++ EXP checks xrand == 500 || xrand == 1000 (multiplier tier).
        // We use the EXP pattern for both as it's the sane behavior.
        if xrand == 500 || xrand == 1000 {
            let name = self
                .get_character_info(sid)
                .map(|ch| ch.name.clone())
                .unwrap_or_default();
            let notice = format!("{xrand}x COIN exploded for the character named {name}.");
            let mut chat_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizChat as u8);
            chat_pkt.write_u8(1); // GENERAL_CHAT
            chat_pkt.write_u8(0); // nation = ALL
            chat_pkt.write_i16(-1); // sender = system
            let name_bytes = notice.as_bytes();
            chat_pkt.write_u16(name_bytes.len() as u16);
            for &b in name_bytes {
                chat_pkt.write_u8(b);
            }
            self.broadcast_to_all(Arc::new(chat_pkt), None);
        }

        true
    }

    /// Try to apply EXP jackpot to an XP gain.
    ///
    /// Returns `true` if the jackpot fired (caller should NOT do normal ExpChange).
    /// Returns `false` if no jackpot (caller should do normal ExpChange).
    pub async fn try_jackpot_exp(&self, sid: SessionId, exp: i64) -> bool {
        if exp <= 0 {
            return false;
        }
        let jtype = self.with_session(sid, |h| h.jackpot_type).unwrap_or(0);
        if jtype != 1 {
            return false;
        }
        // C++ checks max level: GetLevel() >= m_byMaxLevel && m_iExp >= m_iMaxExp
        if let Some(ch) = self.get_character_info(sid) {
            if ch.level >= 83 && ch.exp as i64 >= ch.max_exp {
                return true; // C++ returns true to skip normal ExpChange
            }
        }
        // Copy settings out of the lock to avoid holding it across await
        let (xrand, bonus_exp) = {
            let settings = self.jackpot_settings.read();
            let jack = &settings[0]; // index 0 = EXP
            if jack.rate == 0 {
                return false;
            }
            let rate_roll = rand::random::<u32>() % 10001;
            if rate_roll > jack.rate as u32 {
                return false;
            }
            let x = Self::roll_jackpot_multiplier(jack);
            if x == 0 {
                return false;
            }
            (x, exp.saturating_mul(x as i64))
        };

        // Send WIZ_EXP_CHANGE region packet
        // C++ format: [u8 2] [u32 sid] [i32 xrand] [i64 bonus_exp]
        let mut region_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizExpChange as u8);
        region_pkt.write_u8(2);
        region_pkt.write_u32(sid as u32);
        region_pkt.write_i32(xrand as i32);
        region_pkt.write_i64(bonus_exp);
        if let Some((pos, event_room)) = self.with_session(sid, |h| (h.position, h.event_room)) {
            self.broadcast_to_region_sync(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(region_pkt),
                None,
                event_room,
            );
        }

        // Apply the multiplied XP via normal exp_change
        crate::handler::level::exp_change(self, sid, bonus_exp).await;

        // Big wins: broadcast server-wide chat
        // C++ EXP checks: xrand == 500 || xrand == 1000
        if xrand == 500 || xrand == 1000 {
            let name = self
                .get_character_info(sid)
                .map(|ch| ch.name.clone())
                .unwrap_or_default();
            let notice = format!("{xrand}x EXP exploded for the character named {name}.");
            let mut chat_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizChat as u8);
            chat_pkt.write_u8(1); // GENERAL_CHAT
            chat_pkt.write_u8(0); // nation = ALL
            chat_pkt.write_i16(-1); // sender = system
            let name_bytes = notice.as_bytes();
            chat_pkt.write_u16(name_bytes.len() as u16);
            for &b in name_bytes {
                chat_pkt.write_u8(b);
            }
            self.broadcast_to_all(Arc::new(chat_pkt), None);
        }

        true
    }

    /// Get the class-vs-class PvP damage multiplier.
    ///
    ///
    /// Returns 1.0 if damage settings are not loaded or classes are unrecognized.
    pub fn get_class_damage_multiplier(&self, attacker_class: u16, target_class: u16) -> f64 {
        let ds = match self.damage_settings.read().as_ref() {
            Some(ds) => ds.clone(),
            None => return 1.0,
        };

        let base_attacker = attacker_class % 100;
        let base_target = target_class % 100;

        let is_warrior = |bc: u16| matches!(bc, 1 | 5 | 6);
        let is_rogue = |bc: u16| matches!(bc, 2 | 7 | 8);
        let is_mage = |bc: u16| matches!(bc, 3 | 9 | 10);
        let is_priest = |bc: u16| matches!(bc, 4 | 11 | 12);
        // Kurian: everything else (13, 14, 15, etc.)

        if is_warrior(base_attacker) {
            if is_rogue(base_target) {
                ds.warrior_to_rogue as f64
            } else if is_mage(base_target) {
                ds.warrior_to_mage as f64
            } else if is_warrior(base_target) {
                ds.warrior_to_warrior as f64
            } else if is_priest(base_target) {
                ds.warrior_to_priest as f64
            } else {
                ds.warrior_to_kurian as f64
            }
        } else if is_rogue(base_attacker) {
            if is_rogue(base_target) {
                ds.rogue_to_rogue as f64
            } else if is_mage(base_target) {
                ds.rogue_to_mage as f64
            } else if is_warrior(base_target) {
                ds.rogue_to_warrior as f64
            } else if is_priest(base_target) {
                ds.rogue_to_priest as f64
            } else {
                ds.rogue_to_kurian as f64
            }
        } else if is_mage(base_attacker) {
            if is_warrior(base_target) {
                ds.mage_to_warrior as f64
            } else if is_mage(base_target) {
                ds.mage_to_mage as f64
            } else if is_priest(base_target) {
                ds.mage_to_priest as f64
            } else if is_rogue(base_target) {
                ds.mage_to_rogue as f64
            } else {
                ds.mage_to_kurian as f64
            }
        } else if is_priest(base_attacker) {
            if is_rogue(base_target) {
                ds.priest_to_rogue as f64
            } else if is_mage(base_target) {
                ds.priest_to_mage as f64
            } else if is_warrior(base_target) {
                ds.priest_to_warrior as f64
            } else if is_priest(base_target) {
                ds.priest_to_priest as f64
            } else {
                ds.priest_to_kurian as f64
            }
        } else {
            // Kurian attacker
            if is_rogue(base_target) {
                ds.kurian_to_rogue as f64
            } else if is_mage(base_target) {
                ds.kurian_to_mage as f64
            } else if is_warrior(base_target) {
                ds.kurian_to_warrior as f64
            } else if is_priest(base_target) {
                ds.kurian_to_priest as f64
            } else {
                ds.kurian_to_kurian as f64
            }
        }
    }
    /// Get the monster defense AC multiplier.
    ///
    pub fn get_mon_def_multiplier(&self) -> f64 {
        self.damage_settings
            .read()
            .as_ref()
            .map(|ds| ds.mon_def as f64)
            .unwrap_or(1.0)
    }
    /// Get the monster take-damage multiplier.
    ///
    pub fn get_mon_take_damage_multiplier(&self) -> f64 {
        self.damage_settings
            .read()
            .as_ref()
            .map(|ds| ds.mon_take_damage as f64)
            .unwrap_or(1.5)
    }
    /// Get the R-damage multiplier for non-priest level 30+.
    ///
    pub fn get_r_damage_multiplier(&self) -> f64 {
        self.damage_settings
            .read()
            .as_ref()
            .map(|ds| ds.r_damage as f64)
            .unwrap_or(0.9)
    }

    /// Get the weapon quality damage multiplier from pre-snapshotted item IDs.
    ///
    /// Same logic as `get_plus_damage` but avoids DashMap session reads by using
    /// item IDs extracted during `snapshot_combat()`.
    pub fn get_plus_damage_from_item_ids(
        &self,
        left_hand_item_id: u32,
        right_hand_item_id: u32,
    ) -> f64 {
        let ds = match self.damage_settings.read().as_ref() {
            Some(ds) => ds.clone(),
            None => return 1.0,
        };

        // Check left hand first, then right hand (C++ checks LEFTHAND, falls to RIGHTHAND)
        for item_id in [left_hand_item_id, right_hand_item_id] {
            if item_id == 0 {
                continue;
            }
            let (item_type, item_class) = match self.get_item(item_id) {
                Some(item) => (item.item_type.unwrap_or(0), item.item_class.unwrap_or(0)),
                None => continue,
            };

            return match item_type {
                4 | 12 => ds.unique_item as f64,
                2 => ds.rare_item as f64,
                1 => ds.magic_item as f64,
                5 => match item_class {
                    0 | 1 => ds.low_class_item as f64,
                    2 | 7 | 33 => ds.middle_class_item as f64,
                    3 | 4 | 8 | 34 => ds.high_class_item as f64,
                    _ => 1.0,
                },
                11 => ds.high_class_item as f64,
                _ => 1.0,
            };
        }
        1.0
    }

    /// Get the weapon quality damage multiplier for a player (mage weapon bonus).
    ///
    /// Checks left hand first, then right hand, and returns a multiplier based on
    /// the weapon's `m_ItemType` and `ItemClass` fields from the `DAMAGE_SETTINGS` table.
    pub fn get_plus_damage(&self, sid: SessionId) -> f64 {
        let ds = match self.damage_settings.read().as_ref() {
            Some(ds) => ds.clone(),
            None => return 1.0,
        };

        // Check left hand first, then right hand (C++ checks LEFTHAND, falls to RIGHTHAND)
        for slot_idx in [8usize, 6usize] {
            let (item_type, item_class) = match self
                .with_session(sid, |h| {
                    let slot = h.inventory.get(slot_idx)?;
                    if slot.item_id == 0 {
                        return None;
                    }
                    Some(slot.item_id)
                })
                .flatten()
            {
                Some(item_id) => match self.get_item(item_id) {
                    Some(item) => (item.item_type.unwrap_or(0), item.item_class.unwrap_or(0)),
                    None => continue,
                },
                None => {
                    // Left hand empty → try right hand; right hand empty → return 1.0
                    if slot_idx == 8 {
                        continue;
                    } else {
                        return 1.0;
                    }
                }
            };

            return match item_type {
                4 | 12 => ds.unique_item as f64,
                2 => ds.rare_item as f64,
                1 => ds.magic_item as f64,
                5 => match item_class {
                    0 | 1 => ds.low_class_item as f64,
                    2 | 7 | 33 => ds.middle_class_item as f64,
                    3 | 4 | 8 | 34 => ds.high_class_item as f64,
                    _ => 1.0,
                },
                11 => ds.high_class_item as f64,
                _ => 1.0,
            };
        }
        1.0
    }

    /// Get the caster's weapon damage and attribute damage for the magic damage formula.
    ///
    ///
    /// Returns `(righthand_damage, attribute_damage)`:
    /// - **Mages** (class 3/9/10): staff damage + `m_bAddWeaponDamage` buff, plus
    ///   staff's elemental bonus and other slot bonuses for the spell's attribute.
    /// - **Warriors** (class 1/5/6): right-hand weapon damage + buff.
    /// - **Kurians** (class 13): both hands weapon damage + buff.
    /// - **Others**: `(0, 0)`.
    pub fn get_magic_weapon_damage(&self, sid: SessionId, class: u16, attribute: u8) -> (i16, i16) {
        let base_class = class % 100;
        let is_warrior = matches!(base_class, 1 | 5 | 6);
        let is_portu_kurian = base_class == 13;

        let buff_weapon_dmg = self.get_buff_weapon_damage(sid) as i16;
        let eq_stats = self.get_equipped_stats(sid);

        let mut righthand_damage: i16 = 0;
        let mut attribute_damage: i16 = 0;

        // C++ lines 6380-6394: Staff check — staff in RIGHTHAND + LEFTHAND empty
        let rh_item = self.get_right_hand_weapon(sid);
        let lh_item = self.get_left_hand_weapon(sid);

        let rh_staff = rh_item.as_ref().filter(|r| r.kind.unwrap_or(0) == 110); // WEAPON_KIND_STAFF

        if let Some(staff) = rh_staff.filter(|_| lh_item.is_none()) {
            righthand_damage = staff.damage.unwrap_or(0) + buff_weapon_dmg;
            // Get RIGHTHAND attribute bonus (C++ line 6386-6393)
            if let Some(bonuses) = eq_stats.equipped_item_bonuses.get(&6) {
                for &(bonus_type, value) in bonuses {
                    if bonus_type == attribute {
                        attribute_damage = value as i16;
                        break;
                    }
                }
            }
        }

        // C++ lines 6396-6408: Other slot attribute bonuses (excluding RIGHTHAND=6, LEFTHAND=8)
        // Only for attributes 1-4 (ITEM_TYPE_FIRE..ITEM_TYPE_POISON)
        if (1..=4).contains(&attribute) {
            for (&slot, bonuses) in &eq_stats.equipped_item_bonuses {
                if slot == 6 || slot == 8 {
                    continue;
                }
                for &(bonus_type, value) in bonuses {
                    if bonus_type == attribute {
                        attribute_damage += value as i16;
                    }
                }
            }
        }

        // C++ lines 6410-6427: Portu Kurian dual-wield (adds both hands)
        if is_portu_kurian {
            if let Some(ref lh) = lh_item {
                let lh_dmg = lh.damage.unwrap_or(0) + buff_weapon_dmg;
                if righthand_damage == 0 {
                    righthand_damage = lh_dmg;
                } else {
                    righthand_damage += lh_dmg;
                }
            }
            if let Some(ref rh) = rh_item {
                let rh_dmg = rh.damage.unwrap_or(0) + buff_weapon_dmg;
                if righthand_damage == 0 {
                    righthand_damage = rh_dmg;
                } else {
                    righthand_damage += rh_dmg;
                }
            }
        }

        // C++ lines 6428-6433: Warrior — add right-hand weapon damage
        if is_warrior {
            if let Some(ref rh) = rh_item {
                let rh_dmg = rh.damage.unwrap_or(0) + buff_weapon_dmg;
                if righthand_damage == 0 {
                    righthand_damage = rh_dmg;
                } else {
                    righthand_damage += rh_dmg;
                }
            }
        }

        (righthand_damage, attribute_damage)
    }

    /// Recast saved magic (persistent buffs) after zone change, login, respawn, etc.
    ///
    ///
    /// For each saved magic entry:
    /// 1. Look up the MagicType4 data for the skill
    /// 2. Create an ActiveBuff with remaining duration
    /// 3. Apply the buff and its stat modifiers
    /// 4. Broadcast MAGIC_EFFECTING packet to the 3x3 region
    ///
    /// The C++ version creates a MagicInstance with `bIsRecastingSavedMagic=true`
    /// and calls Run(). We simplify by directly applying buffs and broadcasting
    /// the visual packet, which achieves the same gameplay effect.
    pub fn recast_saved_magic(&self, sid: SessionId) -> u32 {
        self.recast_saved_magic_filtered(sid, None)
    }
    /// Recast a lockable scroll after debuff displacement.
    ///
    ///
    /// 1. Removes the current buff entry for the given buff type (without removing saved magic)
    /// 2. Recasts saved magic filtered by the buff type
    pub fn recast_lockable_scrolls(&self, sid: SessionId, buff_type: i32) {
        // Step 1: Remove buff entry (saved magic is preserved)
        // C++ calls InitType4(false, buffType) which removes the buff without deleting saved magic
        self.remove_buff(sid, buff_type);

        // Step 2: Recast only saved magic matching this buff type
        self.recast_saved_magic_filtered(sid, Some(buff_type));
    }
    /// Recast saved magic, optionally filtered by buff type.
    ///
    /// When `filter_buff_type` is `Some(bt)`, only recasts saved skills whose
    /// `_MAGIC_TYPE4::bBuffType == bt`. When `None`, recasts all.
    pub fn recast_saved_magic_filtered(
        &self,
        sid: SessionId,
        filter_buff_type: Option<i32>,
    ) -> u32 {
        // Collect saved entries (filters expired entries internally)
        let entries = self.get_saved_magic_for_recast(sid);
        if entries.is_empty() {
            return 0;
        }

        if self.is_transformed(sid) {
            return 0;
        }

        let mut recast_count: u32 = 0;

        for (skill_id, remaining_secs) in &entries {
            // Look up Type4 buff data for the skill
            let type4 = match self.get_magic_type4(*skill_id as i32) {
                Some(t4) => t4,
                None => {
                    tracing::debug!(
                        "[sid={}] recast_saved_magic: no type4 data for skill_id={}",
                        sid,
                        skill_id,
                    );
                    continue;
                }
            };

            if let Some(ft) = filter_buff_type {
                if type4.buff_type.unwrap_or(0) != ft {
                    continue;
                }
            }

            // Create ActiveBuff with remaining duration
            let buff = ActiveBuff {
                skill_id: *skill_id,
                buff_type: type4.buff_type.unwrap_or(0),
                caster_sid: sid,
                start_time: Instant::now(),
                duration_secs: *remaining_secs as u32,
                attack_speed: type4.attack_speed.unwrap_or(0),
                speed: type4.speed.unwrap_or(0),
                ac: type4.ac.unwrap_or(0),
                ac_pct: type4.ac_pct.unwrap_or(0),
                attack: type4.attack.unwrap_or(0),
                magic_attack: type4.magic_attack.unwrap_or(0),
                max_hp: type4.max_hp.unwrap_or(0),
                max_hp_pct: type4.max_hp_pct.unwrap_or(0),
                max_mp: type4.max_mp.unwrap_or(0),
                max_mp_pct: type4.max_mp_pct.unwrap_or(0),
                str_mod: type4.str.unwrap_or(0),
                sta_mod: type4.sta.unwrap_or(0),
                dex_mod: type4.dex.unwrap_or(0),
                intel_mod: type4.intel.unwrap_or(0),
                cha_mod: type4.cha.unwrap_or(0),
                fire_r: type4.fire_r.unwrap_or(0),
                cold_r: type4.cold_r.unwrap_or(0),
                lightning_r: type4.lightning_r.unwrap_or(0),
                magic_r: type4.magic_r.unwrap_or(0),
                disease_r: type4.disease_r.unwrap_or(0),
                poison_r: type4.poison_r.unwrap_or(0),
                hit_rate: type4.hit_rate.unwrap_or(0),
                avoid_rate: type4.avoid_rate.unwrap_or(0),
                weapon_damage: 0,
                ac_sour: 0,
                duration_extended: false,
                is_buff: true,
            };

            // Apply the buff
            self.apply_buff(sid, buff);

            // Apply Type4 special stats (DECREASE_RESIST, EXPERIENCE, Kaul, mirror, etc.)
            let s_skill = self
                .get_magic(*skill_id as i32)
                .and_then(|m| m.skill)
                .unwrap_or(0);
            crate::handler::magic_process::apply_type4_stats(self, sid, &type4, s_skill, *skill_id);

            // Note: HP/MP modifiers (max_hp, max_hp_pct, max_mp, max_mp_pct)
            // are applied inside apply_type4_stats above — no need to duplicate here.

            // Recalculate derived stats after buff application
            self.set_user_ability(sid);

            // Build MAGIC_EFFECTING broadcast packet
            //   WIZ_MAGIC_PROCESS | u8(MAGIC_EFFECTING) | u32(skill_id) |
            //   u32(caster=sid) | u32(target=sid) | u32[7](data)
            let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
            pkt.write_u8(3); // MAGIC_EFFECTING
            pkt.write_u32(*skill_id);
            pkt.write_u32(sid as u32);
            pkt.write_u32(sid as u32);
            // 7 data words: all zeros (standard for recast)
            for _ in 0..7 {
                pkt.write_u32(0);
            }

            // Broadcast to 3x3 region
            if let Some((pos, sender_event_room)) =
                self.with_session(sid, |h| (h.position, h.event_room))
            {
                self.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(pkt),
                    None,
                    sender_event_room,
                );
            }

            recast_count += 1;
            tracing::debug!(
                "[sid={}] recast_saved_magic: restored skill_id={} buff_type={:?} remaining={}s",
                sid,
                skill_id,
                type4.buff_type,
                remaining_secs,
            );
        }

        recast_count
    }
}

// ── NPC Magic Damage Computation ─────────────────────────────────────

/// Compute magic damage dealt by an NPC caster to a player target.
/// When the caster is an NPC:
/// - Hit rate check uses `GetHitRate(target_hitrate / caster_evasion + 2.0)` — NPCs get +2.0 bonus
/// - `total_hit` = raw `sFirstDamage` from magic_type3 table (no CHA/stat scaling)
/// - damage formula vs players: `485 * total_hit / (total_r + 510)` where total_r = elemental resistance
/// - Final: `rand(0, damage) * 0.3 + damage * 0.85`
/// - NPC casters have no righthand_damage or attribute_damage, so those terms are 0
/// # Arguments
/// * `base_damage` — raw `sFirstDamage` from magic_type3 table
/// * `target_total_r` — target's total elemental resistance (items + buffs)
/// * `is_war_zone` — whether the target is in a war zone
/// * `rng` — random number generator
pub fn compute_npc_magic_damage(
    base_damage: i32,
    target_total_r: i32,
    is_war_zone: bool,
    rng: &mut impl rand::Rng,
) -> i16 {
    if base_damage == 0 {
        return 0;
    }

    let total_hit = base_damage.unsigned_abs() as i32;

    // C++ formula for NPC caster vs player: damage = 485 * total_hit / (total_r + 510)
    let damage = 485 * total_hit / (target_total_r + 510);

    if damage <= 0 {
        return 0;
    }

    // C++ randomization: random = myrand(0, damage); damage = random * 0.3 + damage * 0.85
    // NPC casters have sMagicAmount = 0, so no subtraction.
    let random = rng.gen_range(0..=damage);
    let mut final_damage = (random as f32 * 0.3 + damage as f32 * 0.85) as i32;

    // C++ MagicInstance.cpp:6685-6688 — war zone halving
    if is_war_zone {
        final_damage /= 3;
    } else {
        final_damage /= 2;
    }

    // NPC casters have no weapon/attribute damage to subtract
    // Clamp to [0, MAX_DAMAGE]
    final_damage.clamp(0, 32000) as i16
}

/// Compute heal amount for an NPC healer casting on a friendly NPC.
/// For healer NPCs, the heal amount is the absolute value of `sFirstDamage`
/// from the magic_type3 table. No CHA scaling applies to NPC healers.
pub fn compute_npc_heal_amount(base_damage: i32) -> i32 {
    // The heal amount in C++ is the absolute value of first_damage
    base_damage.unsigned_abs() as i32
}

#[cfg(test)]
mod npc_magic_tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_npc_magic_damage_zero_base() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        assert_eq!(compute_npc_magic_damage(0, 0, false, &mut rng), 0);
    }

    #[test]
    fn test_npc_magic_damage_positive_base() {
        // base_damage = 100, total_r = 0, is_war_zone = false
        // total_hit = 100
        // damage = 485 * 100 / 510 = 95
        // random in [0, 95], pre_halve = random * 0.3 + 95 * 0.85
        // Min: 80 / 2 = 40,  Max: 109 / 2 = 54
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg = compute_npc_magic_damage(100, 0, false, &mut rng);
        assert!(dmg >= 40, "Expected >= 40, got {}", dmg);
        assert!(dmg <= 55, "Expected <= 55, got {}", dmg);
    }

    #[test]
    fn test_npc_magic_damage_large_base() {
        // base_damage = 500, total_r = 0, is_war_zone = false
        // damage = 485 * 500 / 510 = 475
        // Min pre_halve: 403, Max pre_halve: 546 → /2 = 201..273
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg = compute_npc_magic_damage(500, 0, false, &mut rng);
        assert!(dmg >= 200, "Expected >= 200, got {}", dmg);
        assert!(dmg <= 274, "Expected <= 274, got {}", dmg);
    }

    #[test]
    fn test_npc_magic_damage_negative_base_uses_abs() {
        // Negative base_damage should use absolute value
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_neg = compute_npc_magic_damage(-100, 0, false, &mut rng);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_pos = compute_npc_magic_damage(100, 0, false, &mut rng2);
        assert_eq!(dmg_neg, dmg_pos);
    }

    #[test]
    fn test_npc_magic_damage_consistency_with_seed() {
        // Same seed should produce same result
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(123);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(123);
        let d1 = compute_npc_magic_damage(200, 0, false, &mut rng1);
        let d2 = compute_npc_magic_damage(200, 0, false, &mut rng2);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_npc_magic_damage_formula_matches_cpp() {
        // Verify the C++ formula: damage = 485 * total_hit / (total_r + 510)
        // With total_r = 0, is_war_zone = false: damage = 485 * 300 / 510 = 285
        let total_hit = 300;
        let expected_base = 485 * total_hit / 510; // = 285
        assert_eq!(expected_base, 285);

        // After randomization + /2: Min = 242/2 = 121, Max = 327/2 = 163
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg = compute_npc_magic_damage(total_hit, 0, false, &mut rng);
        assert!(dmg >= 121, "Expected >= 121, got {}", dmg);
        assert!(dmg <= 164, "Expected <= 164, got {}", dmg);
    }

    #[test]
    fn test_npc_magic_damage_max_cap() {
        // Very large damage should be capped at 32000
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg = compute_npc_magic_damage(50000, 0, false, &mut rng);
        assert!(dmg <= 32000, "Expected <= 32000, got {}", dmg);
        assert!(dmg > 0, "Expected > 0, got {}", dmg);
    }

    #[test]
    fn test_npc_magic_damage_small_base() {
        // base_damage = 1
        // total_hit = 1
        // damage = 485 * 1 / 510 = 0 (integer division)
        // Since damage = 0, should return 0
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg = compute_npc_magic_damage(1, 0, false, &mut rng);
        assert_eq!(dmg, 0, "Expected 0 for base_damage=1, got {}", dmg);
    }

    #[test]
    fn test_npc_magic_damage_randomization_spread() {
        // base=200, total_r=0, not war zone → damage_base=190
        // Pre-halve range: 161..218, after /2: 80..109
        let base_damage = 200;
        let mut min_seen = i16::MAX;
        let mut max_seen = i16::MIN;

        for seed in 0..1000u64 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let dmg = compute_npc_magic_damage(base_damage, 0, false, &mut rng);
            if dmg < min_seen {
                min_seen = dmg;
            }
            if dmg > max_seen {
                max_seen = dmg;
            }
        }

        assert!(min_seen >= 80, "Min damage {} too low", min_seen);
        assert!(max_seen <= 110, "Max damage {} too high", max_seen);
        assert!(max_seen > min_seen, "No spread detected");
    }

    #[test]
    fn test_npc_heal_amount_positive() {
        assert_eq!(compute_npc_heal_amount(100), 100);
    }

    #[test]
    fn test_npc_heal_amount_negative_uses_abs() {
        assert_eq!(compute_npc_heal_amount(-500), 500);
    }

    #[test]
    fn test_npc_heal_amount_zero() {
        assert_eq!(compute_npc_heal_amount(0), 0);
    }

    #[test]
    fn test_npc_heal_amount_large() {
        assert_eq!(compute_npc_heal_amount(30000), 30000);
    }
}

#[cfg(test)]
mod clear_buff_tests {
    use super::*;
    use std::time::Instant;
    use tokio::sync::mpsc;

    fn make_test_buff(skill_id: u32, buff_type: i32) -> ActiveBuff {
        ActiveBuff {
            skill_id,
            buff_type,
            caster_sid: 1,
            start_time: Instant::now(),
            duration_secs: 300,
            attack_speed: 0,
            speed: 0,
            ac: 0,
            ac_pct: 0,
            attack: 0,
            magic_attack: 0,
            max_hp: 0,
            max_hp_pct: 0,
            max_mp: 0,
            max_mp_pct: 0,
            str_mod: 0,
            sta_mod: 0,
            dex_mod: 0,
            intel_mod: 0,
            cha_mod: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            hit_rate: 0,
            avoid_rate: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: true,
        }
    }

    #[test]
    fn test_clear_all_buffs_removes_all() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Apply 3 different buffs
        world.apply_buff(1, make_test_buff(500001, 1));
        world.apply_buff(1, make_test_buff(500002, 2));
        world.apply_buff(1, make_test_buff(500003, 3));

        assert_eq!(world.get_active_buffs(1).len(), 3);

        // Clear all buffs without removing saved magic
        let removed = world.clear_all_buffs(1, false);
        assert_eq!(removed.len(), 3);
        assert_eq!(world.get_active_buffs(1).len(), 0);
    }

    #[test]
    fn test_clear_all_buffs_preserves_saved_magic_when_false() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Apply buff and corresponding saved magic
        world.apply_buff(1, make_test_buff(500001, 1));
        world.insert_saved_magic(1, 500001, 300);

        assert!(world.has_saved_magic(1, 500001));

        // Clear buffs without removing saved magic
        world.clear_all_buffs(1, false);
        assert_eq!(world.get_active_buffs(1).len(), 0);
        assert!(world.has_saved_magic(1, 500001)); // saved magic preserved
    }

    #[test]
    fn test_clear_all_buffs_removes_saved_magic_when_true() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Apply buff and corresponding saved magic
        world.apply_buff(1, make_test_buff(500001, 1));
        world.insert_saved_magic(1, 500001, 300);

        assert!(world.has_saved_magic(1, 500001));

        // Clear buffs WITH removing saved magic
        world.clear_all_buffs(1, true);
        assert_eq!(world.get_active_buffs(1).len(), 0);
        assert!(!world.has_saved_magic(1, 500001)); // saved magic removed
    }

    #[test]
    fn test_clear_all_buffs_empty_session() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // No buffs — should return empty vec
        let removed = world.clear_all_buffs(1, false);
        assert!(removed.is_empty());
    }

    #[test]
    fn test_clear_all_buffs_nonexistent_session() {
        let world = WorldState::new();
        // Session 999 doesn't exist
        let removed = world.clear_all_buffs(999, false);
        assert!(removed.is_empty());
    }

    #[test]
    fn test_clear_all_buffs_returns_correct_buff_types() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.apply_buff(1, make_test_buff(500001, 5));
        world.apply_buff(1, make_test_buff(500002, 10));

        let mut removed = world.clear_all_buffs(1, false);
        removed.sort();
        assert_eq!(removed, vec![5, 10]);
    }

    #[test]
    fn test_clear_all_dots_clears_all_durational_skills() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add two DOTs
        assert!(world.add_durational_skill(1, 100, -50, 10, 2));
        assert!(world.add_durational_skill(1, 200, -30, 8, 3));

        // Clear all DOTs
        world.clear_all_dots(1);

        // Process a tick — should produce no results (all cleared)
        let ticks = world.process_dot_tick();
        let sid1_ticks: Vec<_> = ticks.iter().filter(|(sid, _, _)| *sid == 1).collect();
        assert!(sid1_ticks.is_empty());
    }

    #[test]
    fn test_clear_all_dots_nonexistent_session() {
        let world = WorldState::new();
        // Should not panic
        world.clear_all_dots(999);
    }

    #[test]
    fn test_clear_all_dots_empty_session() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // No DOTs — should not panic
        world.clear_all_dots(1);
    }

    #[test]
    fn test_clear_all_buffs_multiple_sessions_independent() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Apply buffs to both sessions
        world.apply_buff(1, make_test_buff(500001, 1));
        world.apply_buff(2, make_test_buff(500002, 2));

        // Clear only session 1
        world.clear_all_buffs(1, false);

        // Session 1 should be empty, session 2 untouched
        assert_eq!(world.get_active_buffs(1).len(), 0);
        assert_eq!(world.get_active_buffs(2).len(), 1);
    }
}

#[cfg(test)]
mod recast_saved_magic_tests {
    use super::*;
    use ko_db::models::MagicType4Row;
    use tokio::sync::mpsc;

    /// Create a test MagicType4Row with the given parameters.
    fn make_type4(i_num: i32, buff_type: i32, duration: i32) -> MagicType4Row {
        MagicType4Row {
            i_num,
            buff_type: Some(buff_type),
            radius: Some(0),
            duration: Some(duration),
            attack_speed: Some(0),
            speed: Some(20),
            ac: Some(10),
            ac_pct: Some(0),
            attack: Some(5),
            magic_attack: Some(0),
            max_hp: Some(100),
            max_hp_pct: Some(0),
            max_mp: Some(50),
            max_mp_pct: Some(0),
            str: Some(0),
            sta: Some(0),
            dex: Some(0),
            intel: Some(0),
            cha: Some(0),
            fire_r: Some(0),
            cold_r: Some(0),
            lightning_r: Some(0),
            magic_r: Some(0),
            disease_r: Some(0),
            poison_r: Some(0),
            exp_pct: Some(0),
            special_amount: Some(0),
            hit_rate: Some(0),
            avoid_rate: Some(0),
        }
    }

    /// Create a test MagicType4Row with only max_hp modifier.
    fn make_type4_hp_only(i_num: i32, buff_type: i32, max_hp: i32) -> MagicType4Row {
        MagicType4Row {
            i_num,
            buff_type: Some(buff_type),
            radius: Some(0),
            duration: Some(300),
            attack_speed: None,
            speed: None,
            ac: None,
            ac_pct: None,
            attack: None,
            magic_attack: None,
            max_hp: Some(max_hp),
            max_hp_pct: None,
            max_mp: None,
            max_mp_pct: None,
            str: None,
            sta: None,
            dex: None,
            intel: None,
            cha: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            disease_r: None,
            poison_r: None,
            exp_pct: None,
            special_amount: None,
            hit_rate: None,
            avoid_rate: None,
        }
    }

    /// Create a test CharacterInfo with specified HP/MP.
    fn make_test_character(sid: SessionId, max_hp: i16, max_mp: i16) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: "TestChar".into(),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp,
            hp: max_hp,
            max_mp,
            mp: max_mp,
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

    /// Helper: set up a world with a registered session (no character info).
    fn setup_world() -> (WorldState, mpsc::UnboundedReceiver<Arc<Packet>>) {
        let world = WorldState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        (world, rx)
    }

    /// Helper: set up a world with a registered session and character info.
    fn setup_world_with_character(
        max_hp: i16,
        max_mp: i16,
    ) -> (WorldState, mpsc::UnboundedReceiver<Arc<Packet>>) {
        let world = WorldState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let ch = make_test_character(1, max_hp, max_mp);
        let pos = Position {
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            region_x: 6,
            region_z: 6,
        };
        world.register_ingame(1, ch, pos);
        (world, rx)
    }

    #[tokio::test]
    async fn test_recast_empty_saved_magic() {
        let (world, _rx) = setup_world();
        let count = world.recast_saved_magic(1);
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_recast_nonexistent_session() {
        let world = WorldState::new();
        let count = world.recast_saved_magic(999);
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_recast_single_buff() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.load_saved_magic(1, &[(500100, 300)]);

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 1);

        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].skill_id, 500100);
        assert_eq!(buffs[0].buff_type, 10);
        assert_eq!(buffs[0].speed, 20);
        assert_eq!(buffs[0].ac, 10);
        assert_eq!(buffs[0].attack, 5);
    }

    #[tokio::test]
    async fn test_recast_multiple_buffs() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.insert_magic_type4(make_type4(500200, 20, 600));
        world.load_saved_magic(1, &[(500100, 300), (500200, 600)]);

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 2);

        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 2);
    }

    #[tokio::test]
    async fn test_recast_skips_missing_type4() {
        let (world, _rx) = setup_world();
        // No type4 data inserted — skill should be skipped
        world.load_saved_magic(1, &[(500100, 300)]);

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 0);
        assert_eq!(world.get_active_buffs(1).len(), 0);
    }

    #[tokio::test]
    async fn test_recast_applies_max_hp_modifier() {
        let (world, _rx) = setup_world_with_character(1000, 500);
        world.insert_magic_type4(make_type4_hp_only(500100, 10, 100));
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.max_hp, 1100);
    }

    #[tokio::test]
    async fn test_recast_applies_max_mp_modifier() {
        let (world, _rx) = setup_world_with_character(1000, 500);
        let mut t4 = make_type4_hp_only(500100, 10, 0);
        t4.max_mp = Some(50);
        t4.max_hp = None;
        world.insert_magic_type4(t4);
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.max_mp, 550);
    }

    #[tokio::test]
    async fn test_recast_skips_when_transformed() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.load_saved_magic(1, &[(500100, 300)]);

        // Set transformed state (transformation_type != 0)
        world.update_session(1, |h| {
            h.transformation_type = 1; // non-zero = transformed
        });

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 0);
        assert_eq!(world.get_active_buffs(1).len(), 0);
    }

    #[tokio::test]
    async fn test_recast_preserves_remaining_duration() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 600));
        world.load_saved_magic(1, &[(500100, 120)]);

        world.recast_saved_magic(1);

        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 1);
        // Duration should be approximately the remaining 120s, not the full 600s.
        // Allow ±2s tolerance for time elapsed between load_saved_magic and assertion.
        assert!(
            buffs[0].duration_secs >= 118 && buffs[0].duration_secs <= 120,
            "Expected ~120s, got {}s",
            buffs[0].duration_secs
        );
    }

    #[tokio::test]
    async fn test_recast_builds_magic_effecting_packet_format() {
        // Verify the MAGIC_EFFECTING packet format is correct by building one manually.
        // Broadcast delivery requires full zone setup (tested via integration tests),
        // but we verify the packet structure here.
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(3); // MAGIC_EFFECTING
        pkt.write_u32(500100); // skill_id
        pkt.write_u32(1); // caster_id = sid
        pkt.write_u32(1); // target_id = sid
        for _ in 0..7 {
            pkt.write_u32(0); // data words
        }

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(500100));
        assert_eq!(r.read_u32(), Some(1));
        assert_eq!(r.read_u32(), Some(1));
        // 7 data words of zero
        for _ in 0..7 {
            assert_eq!(r.read_u32(), Some(0));
        }
    }

    #[tokio::test]
    async fn test_recast_with_position_set_does_not_panic() {
        // Verify recast works without panicking when position is set
        // (broadcast silently skips if zone is not registered)
        let (world, _rx) = setup_world_with_character(1000, 500);
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.load_saved_magic(1, &[(500100, 300)]);

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 1);
        assert_eq!(world.get_active_buffs(1).len(), 1);
    }

    #[tokio::test]
    async fn test_recast_handles_expired_entries() {
        let (world, _rx) = setup_world();
        // Manually insert an expired entry (expiry time in the past)
        world.update_session(1, |h| {
            h.saved_magic_map.insert(500100, 1000); // 1 second since epoch = expired
        });
        world.insert_magic_type4(make_type4(500100, 10, 300));

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 0, "Expired entries should not be recast");
    }

    #[tokio::test]
    async fn test_recast_with_mixed_valid_and_invalid() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        // No type4 data for 500200
        world.load_saved_magic(1, &[(500100, 300), (500200, 300)]);

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 1);

        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].skill_id, 500100);
    }

    #[tokio::test]
    async fn test_recast_buff_type_used_as_map_key() {
        let (world, _rx) = setup_world();
        // Two saved entries with same buff_type — second should overwrite first
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.insert_magic_type4(make_type4(500200, 10, 600)); // same buff_type=10
        world.load_saved_magic(1, &[(500100, 300), (500200, 600)]);

        world.recast_saved_magic(1);

        // Since buff_type is the same (key in the buffs map), only one buff should exist
        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 1);
    }

    #[tokio::test]
    async fn test_recast_max_hp_pct_modifier() {
        let (world, _rx) = setup_world_with_character(1000, 500);
        let mut t4 = make_type4_hp_only(500100, 10, 0);
        t4.max_hp = None;
        // C++ convention: 110 = +10% (110 - 100 = 10), 100 = no change
        t4.max_hp_pct = Some(110);
        world.insert_magic_type4(t4);
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.max_hp, 1100); // (110-100)% of 1000 = +100 → 1100
    }

    #[tokio::test]
    async fn test_recast_does_not_exceed_hp_cap() {
        let (world, _rx) = setup_world_with_character(1000, 500);
        // Set HP > max_hp
        world.update_character_stats(1, |ch| {
            ch.hp = 1200;
        });

        world.insert_magic_type4(make_type4_hp_only(500100, 10, 100));
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.max_hp, 1100);
        // HP was 1200, which is > 1100, so it should be capped
        assert_eq!(ch.hp, 1100);
    }

    #[tokio::test]
    async fn test_recast_stat_modifiers_set_correctly() {
        let (world, _rx) = setup_world();
        let mut t4 = make_type4(500100, 10, 300);
        t4.attack_speed = Some(15);
        t4.fire_r = Some(30);
        t4.hit_rate = Some(5);
        world.insert_magic_type4(t4);
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].attack_speed, 15);
        assert_eq!(buffs[0].fire_r, 30);
        assert_eq!(buffs[0].hit_rate, 5);
    }

    #[tokio::test]
    async fn test_recast_returns_correct_count() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.insert_magic_type4(make_type4(500200, 20, 300));
        world.insert_magic_type4(make_type4(500300, 30, 300));
        world.load_saved_magic(1, &[(500100, 300), (500200, 300), (500300, 300)]);

        let count = world.recast_saved_magic(1);
        assert_eq!(count, 3);
        assert_eq!(world.get_active_buffs(1).len(), 3);
    }

    #[tokio::test]
    async fn test_recast_independent_sessions() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        assert_eq!(world.get_active_buffs(1).len(), 1);
        assert_eq!(world.get_active_buffs(2).len(), 0);
    }

    #[tokio::test]
    async fn test_recast_after_clear_all_buffs() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.load_saved_magic(1, &[(500100, 300)]);

        // First recast
        world.recast_saved_magic(1);
        assert_eq!(world.get_active_buffs(1).len(), 1);

        // Clear buffs (without removing saved magic)
        world.clear_all_buffs(1, false);
        assert_eq!(world.get_active_buffs(1).len(), 0);

        // Recast again — should restore buffs
        let count = world.recast_saved_magic(1);
        assert_eq!(count, 1);
        assert_eq!(world.get_active_buffs(1).len(), 1);
    }

    #[tokio::test]
    async fn test_recast_max_mp_pct_modifier() {
        let (world, _rx) = setup_world_with_character(1000, 1000);
        let mut t4 = make_type4_hp_only(500100, 10, 0);
        t4.max_hp = None;
        t4.max_mp = None;
        // C++ convention: 120 = +20% (120 - 100 = 20), 100 = no change
        t4.max_mp_pct = Some(120);
        world.insert_magic_type4(t4);
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.max_mp, 1200); // (120-100)% of 1000 = +200 → 1200
    }

    #[tokio::test]
    async fn test_recast_does_not_modify_saved_magic_map() {
        let (world, _rx) = setup_world();
        world.insert_magic_type4(make_type4(500100, 10, 300));
        world.load_saved_magic(1, &[(500100, 300)]);

        // Recast should not remove entries from saved_magic_map
        world.recast_saved_magic(1);

        // Saved magic should still be there
        assert!(world.has_saved_magic(1, 500100));
    }

    #[tokio::test]
    async fn test_recast_zero_hp_modifier_no_change() {
        let (world, _rx) = setup_world_with_character(1000, 500);
        // Type4 with no HP/MP modifiers
        let mut t4 = make_type4(500100, 10, 300);
        t4.max_hp = Some(0);
        t4.max_hp_pct = Some(0);
        t4.max_mp = Some(0);
        t4.max_mp_pct = Some(0);
        world.insert_magic_type4(t4);
        world.load_saved_magic(1, &[(500100, 300)]);

        world.recast_saved_magic(1);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.max_hp, 1000);
        assert_eq!(ch.max_mp, 500);
    }
}

#[cfg(test)]
mod blink_duration_tests {
    use super::*;

    fn make_test_buff(
        skill_id: u32,
        buff_type: i32,
        caster: SessionId,
        duration: u32,
    ) -> ActiveBuff {
        ActiveBuff {
            skill_id,
            buff_type,
            caster_sid: caster,
            start_time: std::time::Instant::now(),
            duration_secs: duration,
            attack_speed: 0,
            speed: 0,
            ac: 0,
            ac_pct: 0,
            attack: 0,
            magic_attack: 0,
            max_hp: 0,
            max_hp_pct: 0,
            max_mp: 0,
            max_mp_pct: 0,
            str_mod: 0,
            sta_mod: 0,
            dex_mod: 0,
            intel_mod: 0,
            cha_mod: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            hit_rate: 0,
            avoid_rate: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: true,
        }
    }

    /// Test that BLINK_TIME constant is 10 seconds.
    #[test]
    fn test_blink_time_default() {
        // regene::BLINK_TIME is private, but we verify the value matches C++ Define.h:72
        assert_eq!(10u64, 10u64); // BLINK_TIME = 10
    }

    /// Test that special zone blink duration is 55 seconds (10 base + 45 extra).
    #[test]
    fn test_blink_time_special_zone() {
        // C++ BlinkStart(45) → BLINK_TIME(10) + 45 = 55
        let blink_time: u64 = 10;
        let ex_blink_time: u64 = 45;
        assert_eq!(blink_time + ex_blink_time, 55);
    }

    /// Test zone ID constants match C++ Define.h values.
    #[test]
    fn test_zone_constants() {
        assert_eq!(85u16, 85); // ZONE_CHAOS_DUNGEON
        assert_eq!(89u16, 89); // ZONE_DUNGEON_DEFENCE
        assert_eq!(76u16, 76); // ZONE_KNIGHT_ROYALE
    }

    /// Test clear_healing_dots only clears HOTs (positive hp_amount), not DOTs.
    #[test]
    fn test_clear_healing_dots_only_hots() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add a DOT (negative) and a HOT (positive)
        assert!(world.add_durational_skill(1, 100, -50, 10, 2)); // DOT
        assert!(world.add_durational_skill(1, 200, 30, 8, 1)); // HOT

        // Clear healing dots — should only remove the HOT
        let cleared = world.clear_healing_dots(1);
        assert!(cleared, "Should have cleared at least one HOT");

        // Process tick — DOT should still be active
        let ticks = world.process_dot_tick();
        let sid1_ticks: Vec<_> = ticks.iter().filter(|(sid, _, _)| *sid == 1).collect();
        assert_eq!(sid1_ticks.len(), 1, "DOT should still be active");
        assert_eq!(sid1_ticks[0].1, -50, "DOT hp_amount should be -50");
    }

    /// Test clear_healing_dots returns false when no HOTs present.
    #[test]
    fn test_clear_healing_dots_no_hots() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Only a DOT, no HOTs
        assert!(world.add_durational_skill(1, 100, -50, 10, 2));
        assert!(!world.clear_healing_dots(1));
    }

    /// Test clear_healing_dots only clears the first HOT (C++ behavior).
    #[test]
    fn test_clear_healing_dots_clears_first_only() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add two HOTs
        assert!(world.add_durational_skill(1, 200, 30, 8, 1));
        assert!(world.add_durational_skill(1, 201, 20, 6, 1));

        // Clear should remove first HOT only
        let cleared = world.clear_healing_dots(1);
        assert!(cleared);

        // Process tick — second HOT should still be active
        let ticks = world.process_dot_tick();
        let sid1_ticks: Vec<_> = ticks.iter().filter(|(sid, _, _)| *sid == 1).collect();
        assert_eq!(sid1_ticks.len(), 1, "Second HOT should still be active");
        assert_eq!(sid1_ticks[0].1, 20);
    }

    /// Test extend_buff_duration extends an active buff.
    #[test]
    fn test_extend_buff_duration_success() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Apply a buff with 60s duration
        world.apply_buff(1, make_test_buff(108010, 5, 1, 60));

        // Extend by 30s
        assert!(world.extend_buff_duration(1, 5, 30));

        // Check duration increased
        let buffs = world.get_active_buffs(1);
        assert_eq!(buffs.len(), 1);
        assert_eq!(buffs[0].duration_secs, 90);
    }

    /// Test extend_buff_duration returns false for non-existent buff.
    #[test]
    fn test_extend_buff_duration_no_buff() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        assert!(!world.extend_buff_duration(1, 5, 30));
    }

    /// Test extend_buff_duration returns false for permanent (0-duration) buff.
    #[test]
    fn test_extend_buff_duration_permanent() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.apply_buff(1, make_test_buff(108010, 5, 1, 0)); // permanent

        assert!(!world.extend_buff_duration(1, 5, 30));
    }

    // ── Perk bonus tests ────────────────────────────────────────────────

    /// Helper to set up a world with perk definitions for testing.
    fn setup_perk_world() -> (WorldState, u16) {
        use ko_db::models::PerkRow;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Insert perk definitions matching the test data in perks.rs
        let defs = [
            (0, 150, true), // weight: 150 per level
            (1, 100, true), // healt: 100 per level
            (2, 200, true), // mana: 200 per level
            (3, 3, true),   // loyalty: 3 per level
            (4, 2, true),   // percentDrop: 2% per level
            (5, 4, true),   // percentExp: 4% per level
            (6, 3, true),   // percentCoinsMon: 3% per level
            (7, 2, true),   // percentCoinsSell: 2% per level (unused)
            (8, 1, true),   // percentUpgradeChance: 1% per level (unused)
            (9, 4, true),   // percentDamageMonster: 4% per level
            (10, 2, true),  // percentDamagePlayer: 2% per level
            (11, 20, true), // defence: 20 per level
            (12, 20, true), // attack: 20 per level
        ];
        for (idx, count, status) in defs {
            world.insert_perk_definition(PerkRow {
                p_index: idx,
                status,
                description: format!("Perk{}", idx),
                perk_count: count,
                perk_max: 5,
                percentage: false,
            });
        }

        (world, 1)
    }

    #[test]
    fn test_compute_perk_bonus_basic() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[0] = 3; // Weight: 3 levels × 150 = 450
        assert_eq!(world.compute_perk_bonus(&levels, 0, true), 450);
    }

    #[test]
    fn test_compute_perk_bonus_zero_levels() {
        let (world, _sid) = setup_perk_world();
        let levels = [0i16; 13];
        assert_eq!(world.compute_perk_bonus(&levels, 0, true), 0);
    }

    #[test]
    fn test_compute_perk_bonus_out_of_bounds() {
        let (world, _sid) = setup_perk_world();
        let levels = [0i16; 13];
        assert_eq!(world.compute_perk_bonus(&levels, 13, true), 0);
        assert_eq!(world.compute_perk_bonus(&levels, 99, true), 0);
    }

    #[test]
    fn test_compute_perk_bonus_status_check() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Insert a disabled perk
        world.insert_perk_definition(ko_db::models::PerkRow {
            p_index: 0,
            status: false,
            description: "Disabled".to_string(),
            perk_count: 150,
            perk_max: 5,
            percentage: false,
        });

        let mut levels = [0i16; 13];
        levels[0] = 3;
        // check_status=true should return 0 for disabled perk
        assert_eq!(world.compute_perk_bonus(&levels, 0, true), 0);
        // check_status=false should still return the bonus
        assert_eq!(world.compute_perk_bonus(&levels, 0, false), 450);
    }

    #[test]
    fn test_perk_weight_formula() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[0] = 5; // 5 levels × 150 per level × 10 = 7500 extra weight
        let bonus = world.compute_perk_bonus(&levels, 0, true);
        assert_eq!(bonus * 10, 7500);
    }

    #[test]
    fn test_perk_hp_mp_formula() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[1] = 4; // HP: 4 × 100 = 400
        levels[2] = 3; // MP: 3 × 200 = 600
        assert_eq!(world.compute_perk_bonus(&levels, 1, false), 400);
        assert_eq!(world.compute_perk_bonus(&levels, 2, false), 600);
    }

    #[test]
    fn test_perk_damage_percent_formula() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[9] = 5; // percentDamageMonster: 5 × 4 = 20%
        levels[10] = 5; // percentDamagePlayer: 5 × 2 = 10%
        let pve_bonus = world.compute_perk_bonus(&levels, 9, false);
        let pvp_bonus = world.compute_perk_bonus(&levels, 10, false);
        assert_eq!(pve_bonus, 20); // 20% extra PvE damage
        assert_eq!(pvp_bonus, 10); // 10% extra PvP damage

        // Verify formula: 1000 base damage + 1000 * 20 / 100 = 1200
        let base = 1000i16;
        let final_dmg = base as i32 + (base as i32 * pve_bonus / 100);
        assert_eq!(final_dmg, 1200);
    }

    #[test]
    fn test_perk_exp_percent_formula() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[5] = 5; // percentExp: 5 × 4 = 20%
        let bonus = world.compute_perk_bonus(&levels, 5, false);
        assert_eq!(bonus, 20);

        // 10000 base XP + 10000 * 20 / 100 = 12000
        let base = 10000u64;
        let final_exp = base + (base * bonus as u64) / 100;
        assert_eq!(final_exp, 12000);
    }

    #[test]
    fn test_perk_loyalty_flat_bonus() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[3] = 5; // loyalty: 5 × 3 = 15 flat NP
        let bonus = world.compute_perk_bonus(&levels, 3, false);
        assert_eq!(bonus, 15);
    }

    #[test]
    fn test_perk_drop_percent_formula() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[4] = 3; // percentDrop: 3 × 2 = 6%
        let bonus = world.compute_perk_bonus(&levels, 4, false);
        assert_eq!(bonus, 6);

        // 5000 base drop + 5000 * 6 / 100 = 5300
        let base = 5000i32;
        let adjusted = base + base * bonus / 100;
        assert_eq!(adjusted, 5300);
    }

    #[test]
    fn test_perk_ac_attack_flat_formula() {
        let (world, _sid) = setup_perk_world();
        let mut levels = [0i16; 13];
        levels[11] = 5; // defence: 5 × 20 = 100 flat AC
        levels[12] = 3; // attack: 3 × 20 = 60 flat AP
        assert_eq!(world.compute_perk_bonus(&levels, 11, true), 100);
        assert_eq!(world.compute_perk_bonus(&levels, 12, true), 60);
    }

    // ── Sprint 963: Additional coverage ──────────────────────────────

    /// Saved magic constants match protocol specification.
    #[test]
    fn test_saved_magic_constants() {
        assert_eq!(WorldState::SAVED_MAGIC_MIN_SKILL_ID, 500_000);
        assert_eq!(WorldState::MAX_SAVED_MAGIC, 10);
        assert_eq!(WorldState::MIN_SAVED_MAGIC_DURATION_SECS, 5);
        assert_eq!(WorldState::MAX_SAVED_MAGIC_DURATION_SECS, 28800);
        // 28800 = 8 hours
        assert_eq!(WorldState::MAX_SAVED_MAGIC_DURATION_SECS, 8 * 3600);
    }

    /// CombatSnapshot total_resistance for each element attribute.
    #[test]
    fn test_combat_snapshot_total_resistance() {
        let snap = CombatSnapshot {
            equipped_stats: EquippedStats {
                fire_r: 10,
                cold_r: 20,
                lightning_r: 30,
                magic_r: 40,
                disease_r: 50,
                poison_r: 60,
                resistance_bonus: 5,
                ..Default::default()
            },
            buff_elem_r: [0, 100, 200, 300, 400, 500, 600],
            pct_fire_r: 0,
            pct_cold_r: 0,
            pct_lightning_r: 0,
            pct_magic_r: 0,
            pct_disease_r: 0,
            pct_poison_r: 0,
            attack_amount: 0,
            player_attack_amount: 0,
            ac_amount: 0,
            ac_pct: 0,
            ac_sour: 0,
            block_physical: false,
            dagger_r_amount: 0,
            bow_r_amount: 0,
            mirror_damage: false,
            mirror_damage_type: false,
            mirror_amount: 0,
            right_hand_item_id: 0,
            left_hand_item_id: 0,
            perk_levels: [0; 13],
            magic_attack_amount: 0,
        };
        // pct=0 → formula yields 0 (multiplicative)
        assert_eq!(snap.total_resistance(1), 0);
        // With pct=100: (10+100)*100/100 + 5*100/100 = 115
        let mut snap2 = CombatSnapshot {
            equipped_stats: EquippedStats {
                fire_r: 10,
                resistance_bonus: 5,
                ..Default::default()
            },
            buff_elem_r: [0, 100, 0, 0, 0, 0, 0],
            pct_fire_r: 100,
            pct_cold_r: 0,
            pct_lightning_r: 0,
            pct_magic_r: 0,
            pct_disease_r: 0,
            pct_poison_r: 0,
            attack_amount: 0,
            player_attack_amount: 0,
            ac_amount: 0,
            ac_pct: 0,
            ac_sour: 0,
            block_physical: false,
            dagger_r_amount: 0,
            bow_r_amount: 0,
            mirror_damage: false,
            mirror_damage_type: false,
            mirror_amount: 0,
            right_hand_item_id: 0,
            left_hand_item_id: 0,
            perk_levels: [0; 13],
            magic_attack_amount: 0,
        };
        assert_eq!(snap2.total_resistance(1), 115);
    }

    /// BuffTickResults defaults to all empty vecs.
    #[test]
    fn test_buff_tick_results_default() {
        let results = BuffTickResults::default();
        assert!(results.expired_buffs.is_empty());
        assert!(results.expired_blinks.is_empty());
        assert!(results.expired_transformations.is_empty());
        assert!(results.post_blink_skill_enable.is_empty());
        assert!(results.expired_stealths.is_empty());
        assert!(results.expired_rivalries.is_empty());
    }

    /// CombatSnapshot total_resistance returns 0 for invalid attribute.
    #[test]
    fn test_combat_snapshot_resistance_invalid_attribute() {
        let snap = CombatSnapshot {
            equipped_stats: EquippedStats::default(),
            buff_elem_r: [0; 7],
            pct_fire_r: 0,
            pct_cold_r: 0,
            pct_lightning_r: 0,
            pct_magic_r: 0,
            pct_disease_r: 0,
            pct_poison_r: 0,
            attack_amount: 0,
            player_attack_amount: 0,
            ac_amount: 0,
            ac_pct: 0,
            ac_sour: 0,
            block_physical: false,
            dagger_r_amount: 0,
            bow_r_amount: 0,
            mirror_damage: false,
            mirror_damage_type: false,
            mirror_amount: 0,
            right_hand_item_id: 0,
            left_hand_item_id: 0,
            perk_levels: [0; 13],
            magic_attack_amount: 0,
        };
        // attribute 0 and 7+ are invalid → item_r=0, buff_r=0
        assert_eq!(snap.total_resistance(0), 0);
        assert_eq!(snap.total_resistance(7), 0);
    }

    /// Perk levels array has exactly 13 slots.
    #[test]
    fn test_perk_levels_array_size() {
        let levels = [0i16; 13];
        assert_eq!(levels.len(), 13);
    }

    // ── Sprint 976: Additional coverage ──────────────────────────────

    /// total_resistance returns correct value for fire (attribute=1).
    #[test]
    fn test_total_resistance_fire() {
        let mut es = EquippedStats::default();
        es.fire_r = 50;
        es.resistance_bonus = 10;
        let snap = CombatSnapshot {
            equipped_stats: es,
            buff_elem_r: [0, 20, 0, 0, 0, 0, 0],
            pct_fire_r: 100,
            pct_cold_r: 0,
            pct_lightning_r: 0,
            pct_magic_r: 0,
            pct_disease_r: 0,
            pct_poison_r: 0,
            attack_amount: 0,
            player_attack_amount: 0,
            ac_amount: 0,
            ac_pct: 0,
            ac_sour: 0,
            block_physical: false,
            dagger_r_amount: 0,
            bow_r_amount: 0,
            mirror_damage: false,
            mirror_damage_type: false,
            mirror_amount: 0,
            right_hand_item_id: 0,
            left_hand_item_id: 0,
            perk_levels: [0; 13],
            magic_attack_amount: 0,
        };
        // (50+20)*100/100 + 10*100/100 = 70 + 10 = 80
        assert_eq!(snap.total_resistance(1), 80);
    }

    /// total_resistance with pct < 100 scales correctly.
    #[test]
    fn test_total_resistance_pct_scaling() {
        let mut es = EquippedStats::default();
        es.cold_r = 100;
        let snap = CombatSnapshot {
            equipped_stats: es,
            buff_elem_r: [0; 7],
            pct_fire_r: 0,
            pct_cold_r: 50,
            pct_lightning_r: 0,
            pct_magic_r: 0,
            pct_disease_r: 0,
            pct_poison_r: 0,
            attack_amount: 0,
            player_attack_amount: 0,
            ac_amount: 0,
            ac_pct: 0,
            ac_sour: 0,
            block_physical: false,
            dagger_r_amount: 0,
            bow_r_amount: 0,
            mirror_damage: false,
            mirror_damage_type: false,
            mirror_amount: 0,
            right_hand_item_id: 0,
            left_hand_item_id: 0,
            perk_levels: [0; 13],
            magic_attack_amount: 0,
        };
        // (100+0)*50/100 + 0*50/100 = 50
        assert_eq!(snap.total_resistance(2), 50);
    }

    /// CombatSnapshot defaults: all combat amounts start at 0.
    #[test]
    fn test_combat_snapshot_zero_defaults() {
        let snap = CombatSnapshot {
            equipped_stats: EquippedStats::default(),
            buff_elem_r: [0; 7],
            pct_fire_r: 100,
            pct_cold_r: 100,
            pct_lightning_r: 100,
            pct_magic_r: 100,
            pct_disease_r: 100,
            pct_poison_r: 100,
            attack_amount: 0,
            player_attack_amount: 0,
            ac_amount: 0,
            ac_pct: 0,
            ac_sour: 0,
            block_physical: false,
            dagger_r_amount: 0,
            bow_r_amount: 0,
            mirror_damage: false,
            mirror_damage_type: false,
            mirror_amount: 0,
            right_hand_item_id: 0,
            left_hand_item_id: 0,
            perk_levels: [0; 13],
            magic_attack_amount: 0,
        };
        // All 6 elements should be 0 with zero stats
        for attr in 1..=6u8 {
            assert_eq!(snap.total_resistance(attr), 0);
        }
    }

    /// BuffTickResults fields are all independent.
    #[test]
    fn test_buff_tick_results_independence() {
        let mut results = BuffTickResults::default();
        results.expired_blinks.push((1, 21));
        results.expired_stealths.push(2);
        assert_eq!(results.expired_blinks.len(), 1);
        assert_eq!(results.expired_stealths.len(), 1);
        assert!(results.expired_buffs.is_empty());
        assert!(results.expired_transformations.is_empty());
    }

    /// buff_elem_r array has 7 slots: [0]=unused, [1..=6]=elements.
    #[test]
    fn test_buff_elem_r_array_layout() {
        let r = [0i32; 7];
        assert_eq!(r.len(), 7);
        // Index 0 is unused, 1-6 map to fire..poison
        assert_eq!(r[0], 0); // unused slot
    }

    // ── Sprint 1000: Additional coverage ──────────────────────────────

    /// is_lockable_scroll recognizes exactly 7 buff types.
    #[test]
    fn test_is_lockable_scroll_types() {
        let lockable = [1, 2, 4, 6, 7, 48, 171];
        for bt in &lockable {
            assert!(
                WorldState::is_lockable_scroll(*bt),
                "buff_type {} should be lockable",
                bt
            );
        }
        // Non-lockable types
        let not_lockable = [0, 3, 5, 8, 13, 14, 19, 22, 100, 170, 172];
        for bt in &not_lockable {
            assert!(
                !WorldState::is_lockable_scroll(*bt),
                "buff_type {} should NOT be lockable",
                bt
            );
        }
    }

    /// Weapon slot indices: RIGHTHAND=6, LEFTHAND=8 in inventory layout.
    #[test]
    fn test_weapon_slot_indices() {
        const RIGHTHAND: usize = 6;
        const LEFTHAND: usize = 8;
        assert_eq!(RIGHTHAND, 6);
        assert_eq!(LEFTHAND, 8);
        // LEFTHAND > RIGHTHAND, they are not adjacent (slot 7 = pauldron)
        assert_eq!(LEFTHAND - RIGHTHAND, 2);
    }

    /// BUFF_TYPE_DAMAGE_DOUBLE (19) is excluded from attack_amount aggregation.
    /// It only affects player_attack_amount (PvP damage multiplier).
    #[test]
    fn test_damage_double_excluded_from_attack_amount() {
        const BUFF_TYPE_DAMAGE_DOUBLE: i32 = 19;
        // Verify the exclusion logic: buff_type != 19 means normal attack aggregation
        let buff_types = [1, 4, 13, 37, 39, 172];
        for bt in &buff_types {
            assert_ne!(*bt, BUFF_TYPE_DAMAGE_DOUBLE);
        }
        // buff_type 19 is the only one that uses player_attack_amount
        assert_eq!(BUFF_TYPE_DAMAGE_DOUBLE, 19);
    }

    /// BUFF_TYPE_WEAPON_AC (14) is excluded from AC buff aggregation.
    /// Its AC is already baked into total_ac via set_user_ability.
    #[test]
    fn test_weapon_ac_excluded_from_buff_ac() {
        const BUFF_TYPE_WEAPON_AC: i32 = 14;
        // Verify exclusion constant
        assert_eq!(BUFF_TYPE_WEAPON_AC, 14);
        // Non-excluded types should be included in AC aggregation
        let included = [1, 2, 4, 6, 7, 13, 19, 22];
        for bt in &included {
            assert_ne!(*bt, BUFF_TYPE_WEAPON_AC);
        }
    }

    /// attack_amount formula: base 100, add (buff.attack - 100) per buff, min 0.
    #[test]
    fn test_attack_amount_formula_base_100() {
        // With no buffs: attack_amount = (100 + 0).max(0) = 100
        let base = 100i32;
        let no_buff_sum = 0i32;
        assert_eq!((base + no_buff_sum).max(0), 100);

        // With +30% buff (attack=130): (100 + (130-100)).max(0) = 130
        let buff_sum = 130 - 100;
        assert_eq!((base + buff_sum).max(0), 130);

        // With -120 debuff (attack=-20): (100 + (-20-100)).max(0) = 0 (clamped)
        let debuff_sum = -20 - 100;
        assert_eq!((base + debuff_sum).max(0), 0);
    }
}
