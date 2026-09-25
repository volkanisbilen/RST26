//! Session management, character info, position tracking, and broadcasting.

use super::*;

impl WorldState {
    /// Allocate a unique session ID, skipping 0 (sentinel value for "no session").
    ///
    /// Also registers the session in the rate limiter for flood protection.
    pub fn allocate_session_id(&self) -> SessionId {
        loop {
            let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
            if id != 0 {
                self.rate_limiter.register_session(id);
                return id;
            }
        }
    }
    /// Update character stats for a session via a closure.
    ///
    /// The closure receives a mutable reference to the `CharacterInfo` and can
    /// modify any fields. Commonly used after stat/skill point allocation.
    pub fn update_character_stats(&self, id: SessionId, updater: impl FnOnce(&mut CharacterInfo)) {
        let vitals_changed = if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                let before = (ch.hp, ch.max_hp, ch.mp, ch.max_mp);
                updater(ch);
                before != (ch.hp, ch.max_hp, ch.mp, ch.max_mp)
            } else {
                false
            }
        } else {
            false
        };
        // Drop the session-map guard before party lookup/broadcast. Updating
        // vitals through this common path must refresh both HP and MP bars.
        if vitals_changed {
            crate::handler::party::broadcast_party_hp(self, id);
        }
    }
    /// Recalculate max HP/MP using the coefficient formula and update the session.
    ///
    /// When `iFlag=1`, the zone-based override is skipped and the normal
    /// coefficient formula is used. This restores normal HP after leaving
    /// zones that cap HP (DD zone 89, Chaos zone 85).
    pub fn recalculate_max_hp_mp(&self, sid: SessionId) {
        let ch = match self.get_character_info(sid) {
            Some(c) => c,
            None => return,
        };
        let coeff = self.get_coefficient(ch.class);
        let abilities = crate::handler::stats::recalculate_abilities(&ch, coeff.as_ref());
        self.update_character_stats(sid, |c| {
            c.max_hp = abilities.max_hp;
            c.max_mp = abilities.max_mp;
            if c.hp > c.max_hp {
                c.hp = c.max_hp;
            }
            if c.mp > c.max_mp {
                c.mp = c.max_mp;
            }
        });
    }

    /// Get all **alive** NPC IDs in a 3×3 region grid, filtered by event room.
    ///
    ///
    /// Only returns NPCs whose `event_room` matches the requesting player's
    /// event room. This provides instance-level isolation in event zones.
    pub fn get_nearby_npc_ids(
        &self,
        zone_id: u16,
        rx: u16,
        rz: u16,
        event_room: u16,
    ) -> Vec<NpcId> {
        let zone = match self.get_zone(zone_id) {
            Some(z) => z,
            None => return Vec::new(),
        };
        zone.get_npcs_in_3x3(rx, rz)
            .into_iter()
            .filter(|&nid| !self.is_npc_dead(nid))
            .filter(|&nid| {
                self.get_npc_instance(nid)
                    .map(|inst| inst.event_room == event_room)
                    .unwrap_or(true)
            })
            .collect()
    }
    /// Register a session with an outbound channel (pre-game, no character info yet).
    pub fn register_session(&self, id: SessionId, tx: mpsc::UnboundedSender<Arc<Packet>>) {
        self.sessions.insert(
            id,
            SessionHandle {
                tx,
                pvp_serial_kill_count: 0,
                pvp_total_kill_count: 0,
                pvp_last_kill_time: 0,
                character: None,
                position: Position::default(),
                direction: 0,
                last_response_time: Instant::now(),
                zone_changing: false,
                zone_change_started_at: Instant::now(),
                check_warp_zone_change: false,
                private_chat_target: None,
                target_id: 0,
                pending_knights_invite: 0,
                pending_gate_tax: 0,
                buffs: HashMap::new(),
                saved_magic_map: HashMap::new(),
                durational_skills: Vec::new(),
                inventory: Vec::new(),
                equipped_stats: EquippedStats::default(),
                warehouse: Vec::new(),
                inn_coins: 0,
                warehouse_loaded: false,
                vip_warehouse: Vec::new(),
                vip_password: String::new(),
                vip_password_request: 0,
                vip_vault_expiry: 0,
                vip_warehouse_loaded: false,
                store_open: false,
                trade_state: TRADE_STATE_NONE,
                exchange_user: None,
                exchange_items: Vec::new(),
                is_request_sender: false,
                merchant_state: MERCHANT_STATE_NONE,
                selling_merchant_preparing: false,
                buying_merchant_preparing: false,
                merchant_items: Default::default(),
                merchant_looker: None,
                browsing_merchant: None,
                quests: HashMap::new(),
                daily_quests: HashMap::new(),
                event_nid: -1,
                event_sid: -1,
                native_event_hub_armed: false,
                akara_altar_armed: false,
                quest_helper_id: 0,
                by_selected_reward: -1,
                select_msg_flag: 0,
                select_msg_events: [-1; 12],
                pk_loyalty_daily: 0,
                pk_loyalty_premium_bonus: 0,
                personal_rank: 0,
                knights_rank: 0,
                achieve_map: HashMap::new(),
                achieve_summary: AchieveSummary::default(),
                achieve_login_time: 0,
                achieve_timed: HashMap::new(),
                achieve_challenge_active: false,
                achieve_stat_bonuses: [0i16; 7],
                requesting_challenge: 0,
                challenge_requested: 0,
                challenge_user: -1,
                lost_exp: 0,
                who_killed_me: -1,
                is_muted: false,
                attack_disabled_until: 0,
                last_chat_time: Instant::now(),
                chat_flood_count: 0,
                last_town_time: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1200))
                    .unwrap_or(Instant::now()),
                gm_send_pm_id: 0xFFFF,
                gm_send_pm_time: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(601))
                    .unwrap_or(Instant::now()),
                is_mining: false,
                is_fishing: false,
                auto_mining_time: 0,
                auto_fishing_time: 0,
                beef_exchange_time: Instant::now(),
                event_room: 0,
                need_party: 0,
                party_leader: 0,
                party_type: 0,
                monster_stone_status: false,
                draki_entrance_limit: 3,
                draki_room_id: 0,
                joined_event: false,
                is_final_joined_event: false,
                last_attack_time: None,
                skill_cooldowns: HashMap::new(),
                magic_type_cooldowns: HashMap::new(),
                cast_skill_id: 0,
                cast_x: 0.0,
                cast_z: 0.0,
                cast_failed: false,
                last_type2_cast_time: 0,
                last_type2_skill_id: 0,
                reflect_armor_type: 0,
                dagger_r_amount: 100,
                bow_r_amount: 100,
                mirror_damage: false,
                mirror_damage_type: false,
                mirror_amount: 0,
                last_mining_attempt: Instant::now(),
                last_upgrade_time: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(5))
                    .unwrap_or(Instant::now()),
                upgrade_count: 0,
                last_potion_time: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(3))
                    .unwrap_or(Instant::now()),
                last_target_number_time: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1))
                    .unwrap_or(Instant::now()),
                team_colour: 0, // TeamColourNone
                last_trap_time: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(10))
                    .unwrap_or(Instant::now()),
                speed_last_x: 0.0,
                speed_last_z: 0.0,
                speed_hack_count: 0,
                move_old_echo: -1,
                move_old_speed: 0,
                move_caught_time: Instant::now(),
                move_old_will_x: 0,
                move_old_will_z: 0,
                move_old_will_y: 0,
                pet_data: None,
                last_pet_decay_time: 0,
                is_hiding_cospre: false,
                is_hiding_helmet: false,
                fairy_check: false,
                auto_loot: false,
                is_wanted: false,
                wanted_expiry_time: 0,
                account_id: String::new(),
                premium_map: HashMap::new(),
                premium_in_use: 0,
                clan_premium_in_use: 0,
                switch_premium_count: 0,
                account_status: 0,
                deleted_items: Vec::new(),
                delete_item_list: HashMap::new(),
                chat_room_index: 0,
                block_private_chat: false,
                perk_levels: [0i16; 13],
                rem_perk: 0,
                soul_categories: [
                    [0, 0, 0, 0],
                    [1, 0, 0, 0],
                    [2, 0, 0, 0],
                    [3, 0, 0, 0],
                    [4, 0, 0, 0],
                    [5, 0, 0, 0],
                    [6, 0, 0, 0],
                    [7, 0, 0, 0],
                ],
                soul_slots: [[0, 0], [1, 0], [2, 0], [3, 0], [4, 0]],
                soul_loaded: false,
                seal_max_tier: 0,
                seal_selected_slot: 0,
                seal_status: 1,
                seal_upgrade_count: 0,
                seal_current_level: 0,
                seal_elapsed_time: 0.0,
                seal_loaded: false,
                costume_active_type: 0,
                costume_item_id: 0,
                costume_item_param: 0,
                costume_scale_raw: 0,
                costume_color_index: 0,
                costume_expiry_time: 0,
                costume_loaded: false,
                enchant_max_star: 0,
                enchant_count: 0,
                enchant_slot_levels: [0; 8],
                enchant_slot_unlocked: [0; 9],
                enchant_item_category: 0,
                enchant_item_slot_unlock: 0,
                enchant_item_markers: [0; 5],
                enchant_loaded: false,
                enchant_item_last_fail: None,
                watched_upgrade_item: 0,
                tower_owner_id: -1,
                invisibility_type: 0,
                stealth_end_time: 0,
                blink_expiry_time: 0,
                can_use_skills: true,
                can_use_potions: true,
                is_kaul: false,
                is_undead: false,
                abnormal_type: 1,     // ABNORMAL_NORMAL
                old_abnormal_type: 1, // ABNORMAL_NORMAL
                is_blinded: false,
                block_physical: false,
                block_magic: false,
                is_devil: false,
                size_effect: 0,
                can_teleport: true,
                can_stealth: true,
                block_curses: false,
                reflect_curses: false,
                instant_cast: false,
                drop_scroll_amount: 0,
                weapons_disabled: false,
                mana_absorb: 0,
                absorb_count: 0,
                magic_damage_reduction: 100,
                pct_fire_r: 100,
                pct_cold_r: 100,
                pct_lightning_r: 100,
                pct_magic_r: 100,
                pct_disease_r: 100,
                pct_poison_r: 100,
                exp_gain_buff11: 0,
                exp_gain_buff33: 0,
                skill_np_bonus_33: 0,
                skill_np_bonus_42: 0,
                jackpot_type: 0,
                np_gain_amount: 100,
                noah_gain_amount: 100,
                is_premium_merchant: false,
                weight_buff_amount: 100,
                transformation_type: 0,
                transform_id: 0,
                transform_skill_id: 0,
                transformation_start_time: 0,
                transformation_duration: 0,
                pvp_kill_count: 0,
                zone_online_reward_timers: Vec::new(),
                online_cash_next_time: 0,
                genie_active: false,
                genie_time_abs: 0,
                genie_loaded: false,
                genie_check_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                genie_options: Vec::new(),
                total_training_exp: 0,
                last_training_time: 0,
                flash_time: 0,
                flash_count: 0,
                flash_type: 0,
                flash_exp_bonus: 0,
                flash_dc_bonus: 0,
                flash_war_bonus: 0,
                flash_check_time: 0,
                flame_level: 0,
                flame_time: 0,
                is_offline: false,
                offline_type: OfflineCharacterType::default(),
                offline_remaining_minutes: 0,
                offline_next_check: None,
                knight_cash: 0,
                tl_balance: 0,
                cr_kill_counts: [0u16; 3],
                cr_check_finish: false,
                tagname: String::new(),
                tagname_rgb: 0,
                pus_refund_map: std::collections::HashMap::new(),
                pus_refund_last_time: 0,
                return_symbol_ok: 0,
                return_symbol_time: 0,
                ppcard_cooldown: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(40))
                    .unwrap_or(Instant::now()),
                ext_last_heartbeat: 0,
                ext_last_support: 0,
                ext_last_seen: 0,
                temp_items_sent: false,
                chest_block_items: Vec::new(),
                dr_gm_total_sold: 0,
                dr_mh_total_kill: 0,
                dr_sh_total_exchange: 0,
                dr_cw_counter_win: 0,
                dr_up_counter_bles: 0,
            },
        );
    }
    /// Update a session with character info and initial position (game entry).
    pub fn register_ingame(&self, id: SessionId, character: CharacterInfo, position: Position) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let old_zone = handle.position.zone_id;
            let new_zone = position.zone_id;
            // Add to name index
            let name_lower = character.name.to_lowercase();
            handle.character = Some(character);
            handle.position = position;
            self.name_to_session.insert(name_lower, id);
            self.online_count.fetch_add(1, Ordering::Relaxed);
            if old_zone != new_zone {
                if old_zone != 0 {
                    if let Some(entry) = self.zone_session_index.get(&old_zone) {
                        entry.value().write().remove(&id);
                    }
                }
                if new_zone != 0 {
                    self.zone_session_index
                        .entry(new_zone)
                        .or_insert_with(|| parking_lot::RwLock::new(HashSet::new()))
                        .write()
                        .insert(id);
                }
            }
        }
    }
    /// Update the last-response timestamp for a session (monotonic clock).
    ///
    /// Called on each successful packet receive to track activity.
    ///
    pub fn touch_session(&self, id: SessionId) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.last_response_time = Instant::now();
        }
    }
    /// Remove a session from the world.
    pub fn unregister_session(&self, id: SessionId) {
        // Remove from per-zone session index before removing from sessions map.
        if let Some((_, handle)) = self.sessions.remove(&id) {
            let zone_id = handle.position.zone_id;
            if zone_id != 0 {
                if let Some(entry) = self.zone_session_index.get(&zone_id) {
                    entry.value().write().remove(&id);
                }
            }
            // Remove from name index and decrement online count
            if let Some(ref ch) = handle.character {
                self.name_to_session.remove(&ch.name.to_lowercase());
                self.online_count.fetch_sub(1, Ordering::Relaxed);
            }
        }
        self.rate_limiter.unregister_session(id);
    }
    /// Set the zone_changing flag for a session.
    ///
    pub fn set_zone_changing(&self, id: SessionId, value: bool) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.zone_changing = value;
            if value {
                handle.zone_change_started_at = Instant::now();
            }
        }
    }
    /// Check if a session is currently changing zones.
    ///
    /// Safety: auto-clears the flag after 30 seconds to prevent permanent stuck
    /// state if the client never sends the zone-loaded acknowledgement.
    pub fn is_zone_changing(&self, id: SessionId) -> bool {
        if let Some(mut h) = self.sessions.get_mut(&id) {
            if h.zone_changing && h.zone_change_started_at.elapsed().as_secs() >= 30 {
                tracing::warn!("zone_changing stuck for sid={} (>30s) — auto-clearing", id);
                h.zone_changing = false;
                return false;
            }
            h.zone_changing
        } else {
            false
        }
    }
    /// Set the warp-loop prevention flag.
    ///
    pub fn set_check_warp_zone_change(&self, id: SessionId, value: bool) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.check_warp_zone_change = value;
        }
    }
    /// Check the warp-loop prevention flag.
    pub fn is_check_warp_zone_change(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.check_warp_zone_change)
            .unwrap_or(false)
    }
    /// Set the store_open flag (shopping mall UI open).
    ///
    pub fn set_store_open(&self, id: SessionId, value: bool) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.store_open = value;
        }
    }
    /// Check if the shopping mall UI is currently open.
    pub fn is_store_open(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.store_open)
            .unwrap_or(false)
    }
    /// Check if attack is disabled (GM ban-attack).
    ///
    /// Returns true if `attack_disabled_until` is u32::MAX (permanent) or > current UNIX time.
    pub fn is_attack_disabled(&self, id: SessionId) -> bool {
        let status = self
            .sessions
            .get(&id)
            .map(|h| h.attack_disabled_until)
            .unwrap_or(0);
        if status == 0 {
            return false;
        }
        if status == u32::MAX {
            return true; // permanent
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        status > now
    }
    /// Read session data via a closure.
    ///
    /// Returns `None` if the session does not exist, otherwise returns
    /// `Some(R)` where `R` is the closure's return value.
    pub fn with_session<R>(&self, id: SessionId, f: impl FnOnce(&SessionHandle) -> R) -> Option<R> {
        self.sessions.get(&id).map(|h| f(&h))
    }
    /// Mutably update session data via a closure.
    pub fn update_session(&self, id: SessionId, f: impl FnOnce(&mut SessionHandle)) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let old_zone = handle.position.zone_id;
            f(&mut handle);
            let new_zone = handle.position.zone_id;
            if old_zone != new_zone {
                if old_zone != 0 {
                    if let Some(entry) = self.zone_session_index.get(&old_zone) {
                        entry.value().write().remove(&id);
                    }
                }
                if new_zone != 0 {
                    self.zone_session_index
                        .entry(new_zone)
                        .or_insert_with(|| parking_lot::RwLock::new(HashSet::new()))
                        .write()
                        .insert(id);
                }
            }
        }
    }
    /// Get character info for a session.
    pub fn get_character_info(&self, id: SessionId) -> Option<CharacterInfo> {
        self.sessions.get(&id).and_then(|h| h.character.clone())
    }
    /// Check if a session is a GM (authority == 0).
    ///
    /// Peeks through DashMap ref without cloning CharacterInfo.
    pub fn is_gm(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .and_then(|h| h.character.as_ref().map(|c| c.authority == 0))
            .unwrap_or(false)
    }

    /// Check if a session is in-game (has character info loaded).
    ///
    pub fn is_session_ingame(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.character.is_some())
            .unwrap_or(false)
    }
    /// Get position for a session.
    pub fn get_position(&self, id: SessionId) -> Option<Position> {
        self.sessions.get(&id).map(|h| h.position)
    }
    /// Get the saved cast position if it matches the given skill ID.
    ///
    /// Returns `Some((x, z))` if the player's saved cast_skill_id matches,
    /// or `None` if it doesn't match or the session doesn't exist.
    ///
    pub fn get_cast_position(&self, id: SessionId, skill_id: u32) -> Option<(f32, f32)> {
        self.sessions.get(&id).and_then(|h| {
            if h.cast_skill_id == skill_id {
                Some((h.cast_x, h.cast_z))
            } else {
                None
            }
        })
    }
    /// Check if a player is currently blinking (respawn invulnerability).
    ///
    ///   `return m_bAbnormalType == ABNORMAL_BLINKING;`
    ///
    /// While blinking, the player is invulnerable to NPC attacks and invisible
    /// to NPC AI targeting. Blink expires after `BLINK_TIME` (10 seconds).
    pub fn is_player_blinking(&self, id: SessionId, now_unix: u64) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.blink_expiry_time > 0 && now_unix < h.blink_expiry_time)
            .unwrap_or(false)
    }

    /// Collect all sessions whose blink has expired.
    ///
    /// Called from the periodic tick to find sessions that need blink cleared.
    ///
    /// Returns a list of `(session_id, zone_id)` for expired blink sessions.
    pub fn collect_expired_blinks(&self, now_unix: u64) -> Vec<(SessionId, u16)> {
        let mut expired = Vec::with_capacity(8);
        for entry in self.sessions.iter() {
            let h = entry.value();
            if h.blink_expiry_time > 0 && now_unix >= h.blink_expiry_time {
                expired.push((*entry.key(), h.position.zone_id));
            }
        }
        expired
    }

    /// Clear blink state for a session (set expiry to 0).
    ///
    /// Resets `m_tBlinkExpiryTime`, `m_bRegeneType`, and `m_bCanUseSkills`.
    pub fn clear_blink(&self, id: SessionId) {
        self.update_session(id, |h| {
            h.blink_expiry_time = 0;
            h.can_use_skills = true;
        });
    }

    /// Set invisibility type for a session.
    ///
    /// Called by `StateChangeServerDirect(7, ...)` to update the stealth type.
    pub fn set_invisibility_type(&self, id: SessionId, invis_type: u8) {
        self.update_session(id, |h| {
            h.invisibility_type = invis_type;
        });
    }

    /// Get invisibility type for a session.
    ///
    /// Returns 0 (INVIS_NONE) if session not found.
    pub fn get_invisibility_type(&self, id: SessionId) -> u8 {
        self.sessions
            .get(&id)
            .map(|h| h.invisibility_type)
            .unwrap_or(0)
    }

    /// Check if a player is currently invisible (stealthed).
    ///
    pub fn is_invisible(&self, id: SessionId) -> bool {
        self.get_invisibility_type(id) != 0
    }

    /// Get abnormal type for a session.
    ///
    /// Returns 1 (ABNORMAL_NORMAL) if session not found.
    pub fn get_abnormal_type(&self, id: SessionId) -> u32 {
        self.sessions.get(&id).map(|h| h.abnormal_type).unwrap_or(1)
    }

    /// Build `BroadcastState` from the session's current runtime state.
    ///
    pub fn get_broadcast_state(&self, id: SessionId) -> BroadcastState {
        self.sessions
            .get(&id)
            .map(|h| BroadcastState {
                need_party: h.need_party,
                party_leader: h.party_leader,
                is_devil: if h.is_devil { 1 } else { 0 },
                team_colour: h.team_colour,
                direction: h.direction as u16,
                is_hiding_helmet: if h.is_hiding_helmet { 1 } else { 0 },
                is_hiding_cospre: if h.is_hiding_cospre { 1 } else { 0 },
                knights_rank: if h.knights_rank == 0 {
                    -1
                } else {
                    h.knights_rank as i8
                },
                personal_rank: if h.personal_rank == 0 {
                    -1
                } else {
                    h.personal_rank as i8
                },
                is_in_genie: if h.genie_active { 1 } else { 0 },
                return_symbol_ok: h.return_symbol_ok,
            })
            .unwrap_or_default()
    }

    // ── Transformation (Type 6) helpers ─────────────────────────────

    /// Check if a player is currently transformed.
    ///
    ///   `return m_transformationType != TransformationNone;`
    pub fn is_transformed(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.transformation_type != 0)
            .unwrap_or(false)
    }

    /// Set transformation state for a session.
    ///
    /// Stores transform type, visual ID, skill ID, start time, and duration.
    pub fn set_transformation(
        &self,
        id: SessionId,
        transformation_type: u8,
        transform_id: u16,
        skill_id: u32,
        start_time_ms: u64,
        duration_ms: u64,
    ) {
        self.update_session(id, |h| {
            h.transformation_type = transformation_type;
            h.transform_id = transform_id;
            h.transform_skill_id = skill_id;
            h.transformation_start_time = start_time_ms;
            h.transformation_duration = duration_ms;
            h.can_use_skills = true;
        });
    }

    /// Clear transformation state (Type6Cancel).
    ///
    /// Resets transformation type, visual ID, skill ID, and timing.
    pub fn clear_transformation(&self, id: SessionId) {
        self.update_session(id, |h| {
            h.transformation_type = 0;
            h.transform_id = 0;
            h.transform_skill_id = 0;
            h.transformation_start_time = 0;
            h.transformation_duration = 0;
        });
    }

    /// Collect all sessions whose transformation has expired.
    ///
    /// Called from the periodic tick every ~700ms.
    /// Checks: `UNIXTIME2 - m_tTransformationStartTime >= m_sTransformationDuration`
    ///
    /// Returns `(session_id, transform_skill_id, zone_id)`.
    pub fn collect_expired_transformations(&self, now_ms: u64) -> Vec<(SessionId, u32, u16)> {
        let mut expired = Vec::with_capacity(4);
        for entry in self.sessions.iter() {
            let h = entry.value();
            if h.transformation_type != 0
                && h.transformation_duration > 0
                && now_ms.saturating_sub(h.transformation_start_time) >= h.transformation_duration
            {
                expired.push((*entry.key(), h.transform_skill_id, h.position.zone_id));
            }
        }
        expired
    }

    /// Check if a player can use skills.
    ///
    /// Returns false during blink invulnerability.
    pub fn can_use_skills(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.can_use_skills)
            .unwrap_or(true)
    }

    /// Check whether a player can use potions (false during BUFF_TYPE_NO_POTIONS).
    pub fn can_use_potions(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.can_use_potions)
            .unwrap_or(true)
    }

    /// Check whether a player is undead (BUFF_TYPE_UNDEAD active).
    ///
    pub fn is_undead(&self, id: SessionId) -> bool {
        self.sessions.get(&id).map(|h| h.is_undead).unwrap_or(false)
    }

    /// Check whether a player is blinded (UNSIGHT/BLIND/DISABLE_TARGETING active).
    ///
    pub fn is_blinded(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.is_blinded)
            .unwrap_or(false)
    }

    /// Check whether physical damage is fully blocked for a player.
    ///
    pub fn is_block_physical(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.block_physical)
            .unwrap_or(false)
    }

    /// Check whether magical damage is fully blocked for a player.
    ///
    pub fn is_block_magic(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.block_magic)
            .unwrap_or(false)
    }

    /// Check whether a player is in Devil transformation.
    ///
    pub fn is_devil(&self, id: SessionId) -> bool {
        self.sessions.get(&id).map(|h| h.is_devil).unwrap_or(false)
    }

    /// Check whether a player can teleport (not blocked by NO_RECALL debuff).
    ///
    pub fn can_teleport(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.can_teleport)
            .unwrap_or(true)
    }

    /// Check whether a player can use stealth (not blocked by PROHIBIT_INVIS debuff).
    ///
    pub fn can_stealth(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.can_stealth)
            .unwrap_or(true)
    }

    /// Collect sessions where blink has ended, player is still transformed,
    /// but `can_use_skills` is false (needs re-enabling).
    ///
    ///   `if (!isBlinking() && isTransformed() && m_bCanUseSkills == false) m_bCanUseSkills = true;`
    pub fn collect_post_blink_skill_enable(&self, now_unix: u64) -> Vec<SessionId> {
        let mut result = Vec::with_capacity(8);
        for entry in self.sessions.iter() {
            let h = entry.value();
            let is_blinking = h.blink_expiry_time > 0 && now_unix < h.blink_expiry_time;
            let is_transformed = h.transformation_type != 0;
            if !is_blinking && is_transformed && !h.can_use_skills {
                result.push(*entry.key());
            }
        }
        result
    }

    /// Get the number of online (in-game) sessions.
    ///
    pub fn online_count(&self) -> usize {
        self.online_count.load(Ordering::Relaxed) as usize
    }

    /// Get all session IDs that have an active character (in-game players).
    ///
    /// Used by the periodic character save task to iterate all online players
    /// and persist their stats/position to DB.
    ///
    pub fn get_in_game_session_ids(&self) -> Vec<SessionId> {
        self.sessions
            .iter()
            .filter(|e| e.value().character.is_some())
            .map(|e| *e.key())
            .collect()
    }
    /// Check if a player is dead (hp <= 0 or res_hp_type == USER_DEAD).
    ///
    pub fn is_player_dead(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| {
                h.character
                    .as_ref()
                    .map(|ch| ch.res_hp_type == USER_DEAD || ch.hp <= 0)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
    /// Update a player's current HP in CharacterInfo.
    pub fn update_character_hp(&self, id: SessionId, hp: i16) {
        let changed = if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                let changed = ch.hp != hp;
                ch.hp = hp;
                changed
            } else {
                false
            }
        } else { false };
        if changed {
            crate::handler::party::broadcast_party_hp(self, id);
        }
    }
    /// Update a player's current MP in CharacterInfo.
    pub fn update_character_mp(&self, id: SessionId, mp: i16) {
        let changed = if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                let changed = ch.mp != mp;
                ch.mp = mp;
                changed
            } else {
                false
            }
        } else { false };
        if changed {
            crate::handler::party::broadcast_party_hp(self, id);
        }
    }
    /// Update a player's current SP in CharacterInfo.
    ///
    pub fn update_character_sp(&self, id: SessionId, sp: i16) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                ch.sp = sp;
            }
        }
    }
    /// Update a player's res_hp_type (sit/stand/dead state).
    ///
    pub fn update_res_hp_type(&self, id: SessionId, res_hp_type: u8) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                ch.res_hp_type = res_hp_type;
            }
        }
    }
    /// Remove a player's rivalry state, resetting rival_id to -1.
    ///
    /// Sends WIZ_PVP(PVPRemoveRival=2) to notify the client.
    pub fn remove_rival(&self, id: SessionId) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                ch.rival_id = -1;
                ch.rival_expiry_time = 0;
            }
        }
        // Notify client: WIZ_PVP(PVPRemoveRival=2)
        let pkt = crate::handler::arena::build_remove_rival_packet();
        self.send_to_session_owned(id, pkt);
    }

    /// Assign a rival to a player (victim sets killer as rival for 5 minutes).
    ///
    /// Sends WIZ_PVP(PVPAssignRival=1) to the victim's client.
    ///
    /// Packet format: [u8:1] [u32:rival_session_id] [u16:1] [u16:1] [u32:coins] [u32:loyalty] [sbyte:clan_name] [sbyte:rival_name]
    pub fn set_rival(&self, victim_id: SessionId, killer_id: SessionId, now_secs: u64) {
        let already_has_rival = self
            .with_session(victim_id, |h| {
                h.character
                    .as_ref()
                    .is_some_and(|ch| ch.rival_id >= 0 && now_secs < ch.rival_expiry_time)
            })
            .unwrap_or(false);
        if already_has_rival {
            return;
        }

        // Gather killer info for the packet
        let killer_info = self.with_session(killer_id, |h| {
            h.character
                .as_ref()
                .map(|ch| (ch.gold, ch.loyalty, ch.knights_id, ch.name.clone()))
        });
        let (killer_gold, killer_loyalty, killer_clan_id, killer_name) = match killer_info.flatten()
        {
            Some(info) => info,
            None => return,
        };

        // Look up clan name
        let clan_name = self
            .knights
            .get(&killer_clan_id)
            .map(|k| k.value().name.clone())
            .unwrap_or_default();

        // Update victim's rival state
        if let Some(mut handle) = self.sessions.get_mut(&victim_id) {
            if let Some(ref mut ch) = handle.character {
                ch.rival_id = killer_id as i16;
                ch.rival_expiry_time = now_secs + 300; // RIVALRY_DURATION = 300s
            }
        }

        // Build WIZ_PVP(PVPAssignRival=1) packet and send to victim
        let clan_opt = if clan_name.is_empty() {
            None
        } else {
            Some(clan_name.as_str())
        };
        let pkt = crate::handler::arena::build_assign_rival_packet(
            killer_id as u32,
            killer_gold,
            killer_loyalty,
            clan_opt,
            &killer_name,
        );
        self.send_to_session_owned(victim_id, pkt);
    }

    /// Update a player's anger gauge and send WIZ_PVP(PVPUpdateHelmet/PVPResetHelmet)
    /// to the player's own client.
    ///
    /// Sub-opcodes: PVPUpdateHelmet=5 when gauge > 0, PVPResetHelmet=6 when gauge == 0.
    /// Note: C++ CUser version calls `Send(&result)` (self only), NOT SendToRegion.
    pub fn update_anger_gauge(&self, id: SessionId, new_gauge: u8) {
        use crate::handler::arena::MAX_ANGER_GAUGE;
        let clamped = new_gauge.min(MAX_ANGER_GAUGE);

        if let Some(mut handle) = self.sessions.get_mut(&id) {
            if let Some(ref mut ch) = handle.character {
                ch.anger_gauge = clamped;
            }
        }

        // Build WIZ_PVP packet — send only to the player whose gauge changed
        let pkt = if clamped == 0 {
            crate::handler::arena::build_reset_helmet_packet()
        } else {
            crate::handler::arena::build_update_helmet_packet(clamped)
        };
        self.send_to_session_owned(id, pkt);
    }

    /// Check and expire rivalry for a player whose expiry time has passed.
    ///
    /// Called on the server tick — if `hasRival() && hasRivalryExpired()` → `RemoveRival()`.
    pub fn check_rivalry_expiry(&self, id: SessionId, now_secs: u64) {
        let should_remove = self
            .with_session(id, |h| {
                h.character
                    .as_ref()
                    .is_some_and(|ch| ch.rival_id >= 0 && now_secs >= ch.rival_expiry_time)
            })
            .unwrap_or(false);

        if should_remove {
            self.remove_rival(id);
        }
    }
    /// Collect a snapshot of all in-game sessions for regen tick processing.
    ///
    /// Returns lightweight `RegenData` copies so the regen system can work
    /// without holding DashMap references.
    pub fn collect_regen_data(&self) -> Vec<RegenData> {
        let mut data = Vec::with_capacity(self.online_count());
        for entry in self.sessions.iter() {
            let handle = entry.value();
            if let Some(ref ch) = handle.character {
                data.push(RegenData {
                    session_id: *entry.key(),
                    level: ch.level,
                    hp: ch.hp,
                    max_hp: ch.max_hp,
                    mp: ch.mp,
                    max_mp: ch.max_mp,
                    res_hp_type: ch.res_hp_type,
                    authority: ch.authority,
                    zone_id: handle.position.zone_id,
                    class: ch.class,
                    sp: ch.sp,
                    max_sp: ch.max_sp,
                    pro_skill4: ch.skill_points[8],
                    blink_expiry_time: handle.blink_expiry_time,
                    is_undead: handle.is_undead,
                    last_training_time: handle.last_training_time,
                    total_training_exp: handle.total_training_exp,
                });
            }
        }
        data
    }
    /// Collect regen data for Kurian class sessions only.
    ///
    /// Filters during collection to avoid creating RegenData for non-Kurian
    /// sessions (~87% of players). Used by the SP regen tick.
    pub fn collect_kurian_regen_data(&self) -> Vec<RegenData> {
        let mut data = Vec::with_capacity(8);
        for entry in self.sessions.iter() {
            let handle = entry.value();
            if let Some(ref ch) = handle.character {
                if ch.class % 100 >= 13 && ch.class % 100 <= 15 {
                    data.push(RegenData {
                        session_id: *entry.key(),
                        level: ch.level,
                        hp: ch.hp,
                        max_hp: ch.max_hp,
                        mp: ch.mp,
                        max_mp: ch.max_mp,
                        res_hp_type: ch.res_hp_type,
                        authority: ch.authority,
                        zone_id: handle.position.zone_id,
                        class: ch.class,
                        sp: ch.sp,
                        max_sp: ch.max_sp,
                        pro_skill4: ch.skill_points[8],
                        blink_expiry_time: handle.blink_expiry_time,
                        is_undead: handle.is_undead,
                        last_training_time: handle.last_training_time,
                        total_training_exp: handle.total_training_exp,
                    });
                }
            }
        }
        data
    }

    /// Update a session's HP (alias for `update_character_hp`).
    pub fn update_session_hp(&self, id: SessionId, hp: i16) {
        self.update_character_hp(id, hp);
    }
    /// Update a session's MP (alias for `update_character_mp`).
    pub fn update_session_mp(&self, id: SessionId, mp: i16) {
        self.update_character_mp(id, mp);
    }
    /// Update a session's position and detect region change.
    ///
    pub fn update_position(
        &self,
        id: SessionId,
        zone_id: u16,
        x: f32,
        y: f32,
        z: f32,
    ) -> RegionChangeResult {
        let new_rx = calc_region(x);
        let new_rz = calc_region(z);

        if let Some(mut handle) = self.sessions.get_mut(&id) {
            let old_rx = handle.position.region_x;
            let old_rz = handle.position.region_z;
            let old_zone = handle.position.zone_id;

            handle.position.x = x;
            handle.position.y = y;
            handle.position.z = z;
            handle.position.zone_id = zone_id;
            handle.position.region_x = new_rx;
            handle.position.region_z = new_rz;

            // Update per-zone session index on zone change.
            if old_zone != zone_id {
                if old_zone != 0 {
                    if let Some(entry) = self.zone_session_index.get(&old_zone) {
                        entry.value().write().remove(&id);
                    }
                }
                if zone_id != 0 {
                    self.zone_session_index
                        .entry(zone_id)
                        .or_insert_with(|| parking_lot::RwLock::new(HashSet::new()))
                        .write()
                        .insert(id);
                }
            }

            if old_rx != new_rx || old_rz != new_rz {
                RegionChangeResult::Changed {
                    old_rx,
                    old_rz,
                    new_rx,
                    new_rz,
                }
            } else {
                RegionChangeResult::NoChange
            }
        } else {
            RegionChangeResult::NoChange
        }
    }
    /// Send a packet to a specific session.
    pub fn send_to_session(&self, id: SessionId, packet: &Packet) {
        if let Some(handle) = self.sessions.get(&id) {
            let _ = handle.tx.send(Arc::new(packet.clone()));
        }
    }

    /// Send a packet to a specific session, taking ownership (avoids clone).
    pub fn send_to_session_owned(&self, id: SessionId, packet: Packet) {
        if let Some(handle) = self.sessions.get(&id) {
            let _ = handle.tx.send(Arc::new(packet));
        }
    }

    /// Send a pre-wrapped Arc<Packet> to a specific session (zero-copy).
    ///
    /// Use this in loops where the same packet is sent to multiple recipients
    /// to avoid per-recipient cloning. Wrap the packet in `Arc::new()` once,
    /// then call this with `Arc::clone()` for each recipient.
    pub fn send_to_session_arc(&self, id: SessionId, packet: Arc<Packet>) {
        if let Some(handle) = self.sessions.get(&id) {
            let _ = handle.tx.send(packet);
        }
    }

    /// Get the current number of connected sessions (including pre-game).
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
    /// Broadcast a packet to all sessions in the 3×3 region grid, filtered by event room.
    ///
    ///
    /// Event room isolation: only recipients whose `event_room` matches `sender_event_room`
    /// will receive the packet. In non-event zones all players have `event_room=0`, so the
    /// filter is effectively a no-op. In instanced event zones (BDW, Chaos, Juraid,
    /// Monster Stone, Draki Tower) this prevents cross-room packet leaks.
    pub fn broadcast_to_3x3(
        &self,
        zone_id: u16,
        rx: u16,
        rz: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
        sender_event_room: u16,
    ) {
        let zone = match self.get_zone(zone_id) {
            Some(z) => z,
            None => return,
        };

        let users = zone.get_users_in_3x3(rx, rz);
        for user_id in users {
            if except == Some(user_id) {
                continue;
            }
            // Event room filter — C++ User.cpp:2088
            if let Some(h) = self.sessions.get(&user_id) {
                if h.character.is_some() && h.event_room == sender_event_room {
                    let _ = h.tx.send(Arc::clone(&packet));
                }
            }
        }
    }
    /// Get all session IDs in the 3×3 region grid, filtered by event room.
    ///
    ///
    /// Only returns sessions whose `event_room` matches `sender_event_room`.
    /// This provides instance-level isolation: players in BDW room 1 cannot
    /// see players in BDW room 2. In non-event zones (event_room=0), all
    /// normal players (also event_room=0) match as expected.
    pub fn get_nearby_session_ids(
        &self,
        zone_id: u16,
        rx: u16,
        rz: u16,
        except: Option<SessionId>,
        sender_event_room: u16,
    ) -> Vec<SessionId> {
        let zone = match self.get_zone(zone_id) {
            Some(z) => z,
            None => return Vec::new(),
        };

        let users = zone.get_users_in_3x3(rx, rz);
        users
            .into_iter()
            .filter(|id| except != Some(*id))
            .filter(|id| {
                self.sessions
                    .get(id)
                    .map(|h| h.character.is_some() && h.event_room == sender_event_room)
                    .unwrap_or(false)
            })
            .collect()
    }
    /// Broadcast a packet to all in-game sessions on the server.
    ///
    /// Get a reference to the shared game time/weather state.
    pub fn game_time_weather(&self) -> &Arc<GameTimeWeather> {
        &self.game_time_weather
    }
    /// Get a reference to the Lua quest scripting engine.
    ///
    pub fn lua_engine(&self) -> &Arc<crate::lua_engine::LuaEngine> {
        &self.lua_engine
    }
    /// Get the shared database connection pool (if available).
    ///
    /// Returns `None` in test contexts where no DB is configured.
    pub fn db_pool(&self) -> Option<&ko_db::DbPool> {
        self.db_pool.as_ref()
    }

    /// Get a reference to the rate limiter for flood protection.
    pub fn rate_limiter(&self) -> &crate::rate_limiter::RateLimiter {
        &self.rate_limiter
    }
    pub fn broadcast_to_all(&self, packet: Arc<Packet>, except: Option<SessionId>) {
        for entry in self.sessions.iter() {
            let sid = *entry.key();
            if except == Some(sid) {
                continue;
            }
            let handle = entry.value();
            if handle.character.is_some() {
                let _ = handle.tx.send(Arc::clone(&packet));
            }
        }
    }
    /// Return a list of all in-game (character loaded) session IDs.
    ///
    pub fn all_ingame_session_ids(&self) -> Vec<u16> {
        self.sessions
            .iter()
            .filter(|e| e.value().character.is_some())
            .map(|e| *e.key())
            .collect()
    }

    /// Broadcast a packet to all in-game sessions whose zone is NOT in the exclusion set.
    ///
    /// — skips `isInTempleEventZone()`, `isInMonsterStoneZone()`, `ZONE_PRISON`.
    pub fn broadcast_to_all_excluding_zones(&self, packet: Arc<Packet>, excluded_zones: &[u16]) {
        for entry in self.sessions.iter() {
            let handle = entry.value();
            if handle.character.is_some() && !excluded_zones.contains(&handle.position.zone_id) {
                let _ = handle.tx.send(Arc::clone(&packet));
            }
        }
    }

    /// Broadcast a packet to all in-game sessions in the 3x3 region grid (synchronous).
    ///
    /// This sync version uses SessionHandle position data instead of zone region
    /// locks, suitable for use in non-async Lua bindings.
    pub fn broadcast_to_region_sync(
        &self,
        zone_id: u16,
        rx: u16,
        rz: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
        sender_event_room: u16,
    ) {
        let rx_min = rx.saturating_sub(1);
        let rx_max = rx.saturating_add(1);
        let rz_min = rz.saturating_sub(1);
        let rz_max = rz.saturating_add(1);

        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if except == Some(sid) {
                    continue;
                }
                if let Some(handle) = self.sessions.get(&sid) {
                    if handle.character.is_some()
                        && handle.position.region_x >= rx_min
                        && handle.position.region_x <= rx_max
                        && handle.position.region_z >= rz_min
                        && handle.position.region_z <= rz_max
                        && handle.event_room == sender_event_room
                    {
                        let _ = handle.tx.send(Arc::clone(&packet));
                    }
                }
            }
        }
    }
    /// Broadcast a packet to all in-game sessions in a specific zone.
    ///
    pub fn broadcast_to_zone(&self, zone_id: u16, packet: Arc<Packet>, except: Option<SessionId>) {
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if except == Some(sid) {
                    continue;
                }
                if let Some(handle) = self.sessions.get(&sid) {
                    if handle.character.is_some() {
                        let _ = handle.tx.send(Arc::clone(&packet));
                    }
                }
            }
        }
    }
    /// Collect tag name entries for players in a 3×3 region grid.
    ///
    /// who have non-empty, non-"-" tag names.
    ///
    /// Returns Vec of (char_name, tag_name, r, g, b).
    pub fn collect_session_tags(
        &self,
        zone_id: u16,
        rx: u16,
        rz: u16,
        except: Option<SessionId>,
    ) -> Vec<(String, String, u8, u8, u8)> {
        let rx_min = rx.saturating_sub(1);
        let rx_max = rx.saturating_add(1);
        let rz_min = rz.saturating_sub(1);
        let rz_max = rz.saturating_add(1);
        let mut entries = Vec::with_capacity(16);

        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if except == Some(sid) {
                    continue;
                }
                if let Some(h) = self.sessions.get(&sid) {
                    if h.character.is_none()
                        || h.position.region_x < rx_min
                        || h.position.region_x > rx_max
                        || h.position.region_z < rz_min
                        || h.position.region_z > rz_max
                    {
                        continue;
                    }
                    if h.tagname.is_empty() || h.tagname == "-" {
                        continue;
                    }
                    let name = h
                        .character
                        .as_ref()
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    let rgb = h.tagname_rgb;
                    let r = (rgb & 0xFF) as u8;
                    let g = ((rgb >> 8) & 0xFF) as u8;
                    let b = ((rgb >> 16) & 0xFF) as u8;
                    entries.push((name, h.tagname.clone(), r, g, b));
                }
            }
        }
        entries
    }

    /// Broadcast a packet to all in-game sessions in a zone AND event_room.
    ///
    /// Used by Dungeon Defence timer/stage counter packets.
    pub fn broadcast_to_zone_event_room(
        &self,
        zone_id: u16,
        event_room: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
    ) {
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if except == Some(sid) {
                    continue;
                }
                if let Some(handle) = self.sessions.get(&sid) {
                    if handle.character.is_some() && handle.event_room == event_room {
                        let _ = handle.tx.send(Arc::clone(&packet));
                    }
                }
            }
        }
    }
    /// Broadcast to all players in a zone + event_room that match a nation filter.
    ///
    /// Used for Juraid bridge NPC broadcasts (per-nation).
    pub fn broadcast_to_zone_event_room_nation(
        &self,
        zone_id: u16,
        event_room: u16,
        nation: u8,
        packet: Arc<Packet>,
    ) {
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if let Some(handle) = self.sessions.get(&sid) {
                    if handle.character.is_some() && handle.event_room == event_room {
                        // nation 0 = ALL nations
                        if nation == 0
                            || handle
                                .character
                                .as_ref()
                                .map(|c| c.nation == nation)
                                .unwrap_or(false)
                        {
                            let _ = handle.tx.send(Arc::clone(&packet));
                        }
                    }
                }
            }
        }
    }
    /// Send the native v2615 death notice and the killer's narration update.
    ///
    /// `WIZ_CHAT/DEATH_NOTICE` is rendered as the chat-bar death line and
    /// minimap marker. `WIZ_KILLASSIST` drives `re_killcount.uif`,
    /// `killnameall.dxt` and `killnamebackgall.dxt`.
    #[allow(clippy::too_many_arguments)]
    pub fn send_death_notice_to_zone(
        &self,
        zone_id: u16,
        killer_sid: SessionId,
        victim_sid: SessionId,
        killer_name: &str,
        victim_name: &str,
        _killer_party_id: Option<u16>,
        victim_x: u16,
        victim_z: u16,
    ) {
        let killer_nation = self
            .get_character_info(killer_sid)
            .map(|ch| ch.nation)
            .or_else(|| self.get_bot(killer_sid as u32).map(|bot| bot.nation))
            .unwrap_or(0);
        let victim_nation = self
            .get_character_info(victim_sid)
            .map(|ch| ch.nation)
            .or_else(|| self.get_bot(victim_sid as u32).map(|bot| bot.nation))
            .unwrap_or(0);
        let death_notice = crate::handler::chat::build_death_notice_packet(
            killer_nation,
            victim_nation,
            0,
            killer_sid as u32,
            killer_name,
            victim_sid as u32,
            victim_name,
            victim_x,
            victim_z,
        );
        self.broadcast_to_zone(zone_id, Arc::new(death_notice), None);

        // Runtime bots have no client connection. A real player killer receives
        // the exact CUser::KA_KillUpdate payload used by the 2615 client UI.
        if (killer_sid as u32) < crate::world::BOT_ID_BASE {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let mut narration_state = None;
            self.update_session(killer_sid, |handle| {
                if handle.pvp_last_kill_time > 0
                    && now.saturating_sub(handle.pvp_last_kill_time) > 15 * 60
                {
                    handle.pvp_serial_kill_count = 0;
                    handle.pvp_total_kill_count = 0;
                }
                handle.pvp_serial_kill_count = handle.pvp_serial_kill_count.saturating_add(1);
                handle.pvp_total_kill_count = handle.pvp_total_kill_count.saturating_add(1);
                handle.pvp_last_kill_time = now;
                narration_state = Some((
                    handle.pvp_serial_kill_count.min(u16::MAX as u32) as u16,
                    handle.pvp_total_kill_count,
                ));
            });
            if let Some((stage, total_kills)) = narration_state {
                self.send_to_session_owned(
                    killer_sid,
                    crate::handler::dead::build_kill_narration_packet(
                        killer_sid as u32,
                        killer_name,
                        killer_nation,
                        stage,
                    ),
                );
                if total_kills % 10 == 0 {
                    let killer_nation = self
                        .get_character_info(killer_sid)
                        .map(|ch| ch.nation)
                        .unwrap_or(0);
                    let total_packet = crate::handler::dead::build_kill_total_packet(
                        killer_sid as u32,
                        killer_name,
                        killer_nation,
                        total_kills.min(u16::MAX as u32) as u16,
                    );
                    if let Some(party_id) = self.get_party_id(killer_sid) {
                        self.send_to_party(party_id, &total_packet);
                    } else {
                        self.send_to_session_owned(killer_sid, total_packet);
                    }
                }
            }
        }

        if (victim_sid as u32) < crate::world::BOT_ID_BASE {
            self.update_session(victim_sid, |handle| {
                handle.pvp_serial_kill_count = 0;
            });
        }
    }

    /// Collect all in-game session IDs.
    ///
    /// Used by GM `+resetloyalty` to iterate all online players.
    pub fn all_session_ids(&self) -> Vec<SessionId> {
        let mut result = Vec::with_capacity(self.online_count());
        for entry in self.sessions.iter() {
            if entry.value().character.is_some() {
                result.push(*entry.key());
            }
        }
        result
    }

    /// Collect all in-game session IDs in a given zone.
    ///
    /// Used by GM `+tp_all` to enumerate players for mass teleport.
    pub fn sessions_in_zone(&self, zone_id: u16) -> Vec<SessionId> {
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let ids = index_entry.value().read();
            let mut result = Vec::with_capacity(ids.len());
            for &sid in ids.iter() {
                if let Some(handle) = self.sessions.get(&sid) {
                    if handle.character.is_some() {
                        result.push(sid);
                    }
                }
            }
            result
        } else {
            Vec::new()
        }
    }
    /// Broadcast a packet to all in-game sessions of a specific nation.
    ///
    pub fn broadcast_to_nation(&self, nation: u8, packet: Arc<Packet>, except: Option<SessionId>) {
        for entry in self.sessions.iter() {
            let sid = *entry.key();
            if except == Some(sid) {
                continue;
            }
            let handle = entry.value();
            if let Some(ref ch) = handle.character {
                if ch.nation == nation {
                    let _ = handle.tx.send(Arc::clone(&packet));
                }
            }
        }
    }
    /// Broadcast a packet to all in-game sessions with level ≤ max_level.
    ///
    pub fn broadcast_to_max_level(&self, max_level: u8, packet: Arc<Packet>) {
        for entry in self.sessions.iter() {
            let handle = entry.value();
            if let Some(ref ch) = handle.character {
                if ch.level <= max_level {
                    let _ = handle.tx.send(Arc::clone(&packet));
                }
            }
        }
    }

    /// Broadcast a packet to class-matched, party-less players in a zone.
    ///
    ///
    /// `class_bitmask`: bit0=warrior, bit1=rogue, bit2=mage, bit3=priest, bit4(10)=kurian
    pub fn broadcast_to_zone_matched_class(
        &self,
        zone_id: u16,
        nation: u8,
        event_room: u16,
        class_bitmask: u8,
        packet: Arc<Packet>,
        except: Option<SessionId>,
    ) {
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if except == Some(sid) {
                    continue;
                }
                if let Some(handle) = self.sessions.get(&sid) {
                    let Some(ref ch) = handle.character else {
                        continue;
                    };
                    // Must match event room
                    if handle.event_room != event_room {
                        continue;
                    }
                    // Must NOT be in a party
                    if ch.party_id.is_some() {
                        continue;
                    }
                    // Nation check (Moradon 21-25 allows cross-nation)
                    let is_moradon = (21..=25).contains(&zone_id);
                    if !is_moradon && ch.nation != nation {
                        continue;
                    }
                    // Class bitmask check (matches C++ FundamentalMethods.cpp:414-418)
                    let class = ch.class;
                    let w =
                        class_bitmask & 1 != 0 && crate::handler::quest::job_group_check(class, 1);
                    let r =
                        class_bitmask & 2 != 0 && crate::handler::quest::job_group_check(class, 2);
                    let m =
                        class_bitmask & 4 != 0 && crate::handler::quest::job_group_check(class, 3);
                    let p =
                        class_bitmask & 8 != 0 && crate::handler::quest::job_group_check(class, 4);
                    let k = class_bitmask & 10 != 0
                        && crate::handler::quest::job_group_check(class, 13);
                    if w || r || m || p || k {
                        let _ = handle.tx.send(Arc::clone(&packet));
                    }
                }
            }
        }
    }

    /// Find a session by account ID (case-insensitive).
    ///
    ///
    /// Used by the login handler to detect and kick duplicate logins.
    pub fn find_session_by_account(&self, account: &str) -> Option<SessionId> {
        let acct_lower = account.to_lowercase();
        for entry in self.sessions.iter() {
            if entry.value().account_id.to_lowercase() == acct_lower {
                return Some(*entry.key());
            }
        }
        None
    }

    /// Send a WIZ_MYINFO packet with info_type=0x07 to display a kick/disconnect
    /// reason dialog on the client before the connection is closed.
    ///
    /// The client at handler 0xE6AC3F reads a reason string from this packet,
    /// looks up string resource 0xAFF3, and shows a MessageBox with the reason.
    ///
    /// Packet format: `[u16 info_type=7][u32 0][u8 0][sbyte reason_string]`
    pub fn send_kick_reason(&self, id: SessionId, reason: &str) {
        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizMyInfo as u8);
        pkt.write_u16(0x0007); // info_type = disconnect reason
        pkt.write_u32(0); // socket_id (unused by this handler)
        pkt.write_u8(0); // char_result
        pkt.write_sbyte_string(reason);
        self.send_to_session_owned(id, pkt);
    }

    /// Kick a session for duplicate login — remove from world so the old
    /// session's writer/reader tasks detect the closed channel and shut down.
    ///
    ///
    /// We remove the session from the DashMap which drops the `tx` sender,
    /// then re-insert a tombstone-free entry. The old session's reader/writer
    /// tasks will detect the broken channel and exit, but since the session
    /// is already removed, the cleanup is handled here inline.
    pub async fn kick_session_for_duplicate(&self, id: SessionId) {
        // 0a. Send kick reason to client before disconnecting
        self.send_kick_reason(id, "Another client has logged in with this account.");

        // 0b. Save character data before kicking — prevents data loss on duplicate login
        if let Some(pool) = self.db_pool() {
            crate::systems::character_save::save_single_character_sync(
                self,
                pool,
                id,
                "Duplicate login save",
            )
            .await;
        }

        // 1. Get position for zone region cleanup before removing
        let pos = self.sessions.get(&id).map(|h| h.position);

        // 2. Party cleanup — remove from party so members see correct state
        self.cleanup_party_on_disconnect(id);

        // 3. Trade cleanup — cancel active exchange if any
        if self.is_trading(id) {
            let partner_sid = self.get_exchange_user(id);
            self.reset_trade(id);
            if let Some(partner) = partner_sid {
                self.reset_trade(partner);
                let mut cancel_pkt =
                    ko_protocol::Packet::new(ko_protocol::Opcode::WizExchange as u8);
                cancel_pkt.write_u8(0x08); // EXCHANGE_CANCEL
                self.send_to_session_owned(partner, cancel_pkt);
            }
        }

        // 4. Merchant cleanup — close stall if active
        if self.is_merchanting(id) {
            self.close_merchant(id);
        }

        // 5. Zone region cleanup (must happen while we still know the position)
        if let Some(pos) = pos {
            if let Some(zone) = self.get_zone(pos.zone_id) {
                zone.remove_user(pos.region_x, pos.region_z, id);
            }
        }

        // 6. Remove from zone session index before removing from sessions.
        if let Some(pos) = pos {
            if pos.zone_id != 0 {
                if let Some(entry) = self.zone_session_index.get(&pos.zone_id) {
                    entry.value().write().remove(&id);
                }
            }
        }

        // 7. Remove from sessions DashMap — drops tx, closing the channel.
        // The old session's writer task will detect the closed receiver and exit.
        self.sessions.remove(&id);
    }

    /// Try to acquire the login lock for an account (case-insensitive).
    ///
    /// Returns `true` if the lock was acquired (no other login is in progress).
    /// Returns `false` if another login for this account is already being processed.
    /// The caller MUST call `release_login_lock()` when the login flow completes.
    pub fn try_acquire_login_lock(&self, account: &str) -> bool {
        let key = account.to_lowercase();
        // Atomic check-and-insert via DashMap entry API (holds shard lock).
        match self.login_in_progress.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(_) => false,
            dashmap::mapref::entry::Entry::Vacant(v) => {
                v.insert(());
                true
            }
        }
    }

    /// Release the login lock for an account after the login flow completes.
    pub fn release_login_lock(&self, account: &str) {
        self.login_in_progress.remove(&account.to_lowercase());
    }

    /// Update the name-to-session index when a character is renamed.
    pub fn update_name_index(&self, old_name: &str, new_name: &str, id: SessionId) {
        self.name_to_session.remove(&old_name.to_lowercase());
        self.name_to_session.insert(new_name.to_lowercase(), id);
    }

    /// Find a session by character name (case-insensitive).
    /// O(1) lookup via name_to_session index.
    ///
    pub fn find_session_by_name(&self, name: &str) -> Option<SessionId> {
        self.name_to_session
            .get(&name.to_lowercase())
            .map(|entry| *entry.value())
    }
    /// Find a session matching a predicate on `SessionHandle`.
    ///
    /// Scans all active sessions and returns the first match.
    pub fn find_session_by<F>(&self, predicate: F) -> Option<SessionId>
    where
        F: Fn(&super::types::SessionHandle) -> bool,
    {
        for entry in self.sessions.iter() {
            if predicate(entry.value()) {
                return Some(*entry.key());
            }
        }
        None
    }
    /// Collect all session IDs matching a predicate.
    ///
    /// Like `find_session_by` but returns ALL matches instead of just the first.
    /// Used by DD timer to find all users in a specific event room.
    pub fn collect_sessions_by<F>(&self, predicate: F) -> Vec<SessionId>
    where
        F: Fn(&super::types::SessionHandle) -> bool,
    {
        let mut result = Vec::with_capacity(32);
        for entry in self.sessions.iter() {
            if predicate(entry.value()) {
                result.push(*entry.key());
            }
        }
        result
    }

    /// Set the private chat target for a session.
    ///
    pub fn set_private_chat_target(&self, id: SessionId, target: Option<SessionId>) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.private_chat_target = target;
        }
    }
    /// Get the private chat target for a session.
    pub fn get_private_chat_target(&self, id: SessionId) -> Option<SessionId> {
        self.sessions.get(&id).and_then(|h| h.private_chat_target)
    }
    /// Check if a session is blocking private messages.
    ///
    pub fn is_blocking_private_chat(&self, id: SessionId) -> bool {
        self.sessions
            .get(&id)
            .map(|h| h.block_private_chat)
            .unwrap_or(false)
    }
    /// Set the PM block flag for a session.
    ///
    pub fn set_block_private_chat(&self, id: SessionId, block: bool) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.block_private_chat = block;
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn broadcast_to_old_regions(
        &self,
        zone_id: u16,
        old_rx: u16,
        old_rz: u16,
        new_rx: u16,
        new_rz: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
        sender_event_room: u16,
    ) {
        let zone = match self.get_zone(zone_id) {
            Some(z) => z,
            None => return,
        };

        // Compute the set difference: cells in old 3x3 but not in new 3x3
        let old_cells = get_3x3_cells(old_rx, old_rz, zone.max_region_x, zone.max_region_z);
        let new_cells = get_3x3_cells(new_rx, new_rz, zone.max_region_x, zone.max_region_z);
        let old_only: Vec<_> = old_cells.difference(&new_cells).copied().collect();

        for (cx, cz) in old_only {
            if let Some(region) = zone.get_region(cx, cz) {
                let guard = region.users.read();
                for &user_id in guard.iter() {
                    if except == Some(user_id) {
                        continue;
                    }
                    // Event room filter
                    if let Some(h) = self.sessions.get(&user_id) {
                        if h.character.is_some() && h.event_room == sender_event_room {
                            let _ = h.tx.send(Arc::clone(&packet));
                        }
                    }
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn broadcast_to_new_regions(
        &self,
        zone_id: u16,
        old_rx: u16,
        old_rz: u16,
        new_rx: u16,
        new_rz: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
        sender_event_room: u16,
    ) {
        let zone = match self.get_zone(zone_id) {
            Some(z) => z,
            None => return,
        };

        let old_cells = get_3x3_cells(old_rx, old_rz, zone.max_region_x, zone.max_region_z);
        let new_cells = get_3x3_cells(new_rx, new_rz, zone.max_region_x, zone.max_region_z);
        let new_only: Vec<_> = new_cells.difference(&old_cells).copied().collect();

        for (cx, cz) in new_only {
            if let Some(region) = zone.get_region(cx, cz) {
                let guard = region.users.read();
                for &user_id in guard.iter() {
                    if except == Some(user_id) {
                        continue;
                    }
                    // Event room filter
                    if let Some(h) = self.sessions.get(&user_id) {
                        if h.character.is_some() && h.event_room == sender_event_room {
                            let _ = h.tx.send(Arc::clone(&packet));
                        }
                    }
                }
            }
        }
    }
    /// Check if a player is a clan leader.
    pub fn is_session_clan_leader(&self, sid: SessionId) -> bool {
        self.with_session(sid, |h| {
            if let Some(c) = &h.character {
                c.fame == 1 && c.knights_id > 0
            } else {
                false
            }
        })
        .unwrap_or(false)
    }
    /// Get session's clan ID.
    pub fn get_session_clan_id(&self, sid: SessionId) -> u16 {
        self.with_session(sid, |h| {
            h.character.as_ref().map(|c| c.knights_id).unwrap_or(0)
        })
        .unwrap_or(0)
    }
    /// Get session's character name.
    pub fn get_session_name(&self, sid: SessionId) -> Option<String> {
        self.with_session(sid, |h| h.character.as_ref().map(|c| c.name.clone()))
            .flatten()
    }
    /// Get session's level.
    pub fn get_session_level(&self, sid: SessionId) -> u8 {
        self.with_session(sid, |h| h.character.as_ref().map(|c| c.level).unwrap_or(0))
            .unwrap_or(0)
    }
    // ── Challenge (Duel) System Accessors ──────────────────────────────

    /// Get challenge state for a session.
    ///
    /// Returns `(requesting_challenge, challenge_requested, challenge_user)`.
    pub fn get_challenge_state(&self, sid: SessionId) -> (u8, u8, i16) {
        self.with_session(sid, |h| {
            (
                h.requesting_challenge,
                h.challenge_requested,
                h.challenge_user,
            )
        })
        .unwrap_or((0, 0, -1))
    }
    /// Collect online clan members in a specific zone eligible for CVC arena warp.
    ///
    /// Returns `(session_id, nation)` for each eligible member.
    ///
    /// Skips members in temple event zones (`isInTempleEventZone()`).
    pub fn get_cvc_eligible_clan_members(
        &self,
        clan_id: u16,
        origin_zone: u16,
    ) -> Vec<(SessionId, u8)> {
        let mut result = Vec::new();
        if let Some(index_entry) = self.zone_session_index.get(&origin_zone) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if let Some(h) = self.sessions.get(&sid) {
                    let ch = match &h.character {
                        Some(c) => c,
                        None => continue,
                    };
                    if ch.knights_id != clan_id {
                        continue;
                    }
                    // C++ first checks: isInGame() && !isInTempleEventZone()
                    if crate::systems::event_room::is_in_temple_event_zone(h.position.zone_id) {
                        continue;
                    }
                    if h.merchant_state != MERCHANT_STATE_NONE
                        || h.is_mining
                        || h.is_fishing
                        || h.requesting_challenge != 0
                        || h.challenge_user >= 0
                        || h.trade_state != TRADE_STATE_NONE
                        || ch.party_id.is_some()
                    {
                        continue;
                    }
                    result.push((sid, ch.nation));
                }
            }
        }
        result
    }

    /// Check if two sessions are in the same event room.
    ///
    ///
    /// Returns `true` if:
    /// - Both sessions have `event_room > 0` AND their values are equal
    /// - Both sessions have `event_room == 0` (neither is in an event room)
    ///
    /// Returns `false` if one is in an event room and the other is not,
    /// or if they are in different event rooms.
    pub fn is_same_event_room(&self, sid_a: SessionId, sid_b: SessionId) -> bool {
        let room_a = self.with_session(sid_a, |h| h.event_room).unwrap_or(0);
        let room_b = self.with_session(sid_b, |h| h.event_room).unwrap_or(0);
        room_a == room_b
    }

    /// Get the event room ID for a session (0 = not in any event room).
    ///
    pub fn get_event_room(&self, id: SessionId) -> u16 {
        self.with_session(id, |h| h.event_room).unwrap_or(0)
    }

    /// Get the Monster Stone activation status for a session.
    ///
    pub fn get_monster_stone_status(&self, id: SessionId) -> bool {
        self.with_session(id, |h| h.monster_stone_status)
            .unwrap_or(false)
    }

    /// Set the Monster Stone activation status for a session.
    ///
    pub fn set_monster_stone_status(&self, id: SessionId, status: bool) {
        self.update_session(id, |h| {
            h.monster_stone_status = status;
        });
    }

    /// Get the tower owner NPC ID for a session (-1 = not mounted).
    ///
    pub fn get_tower_owner_id(&self, id: SessionId) -> i32 {
        self.with_session(id, |h| h.tower_owner_id).unwrap_or(-1)
    }

    /// Set the tower owner NPC ID for a session.
    ///
    pub fn set_tower_owner_id(&self, id: SessionId, npc_id: i32) {
        self.update_session(id, |h| {
            h.tower_owner_id = npc_id;
        });
    }

    // ── Pet Decay System ──────────────────────────────────────────────

    /// Collect a snapshot of all sessions with active pets for decay processing.
    ///
    /// Only includes sessions that have an active pet (pet_data.is_some()).
    pub fn collect_pet_decay_data(&self, now_unix: u64) -> Vec<PetDecayData> {
        let mut data = Vec::with_capacity(16);
        for entry in self.sessions.iter() {
            let handle = entry.value();
            if handle.character.is_none() {
                continue;
            }
            if let Some(ref pet) = handle.pet_data {
                // Only include if decay interval has elapsed
                if handle.last_pet_decay_time + PET_DECAY_INTERVAL_SECS < now_unix {
                    data.push(PetDecayData {
                        session_id: *entry.key(),
                        satisfaction: pet.satisfaction,
                        last_decay_time: handle.last_pet_decay_time,
                        pet_nid: pet.nid,
                        pet_index: pet.index,
                    });
                }
            }
        }
        data
    }

    /// Apply pet satisfaction decay for a session and return the new satisfaction.
    ///
    /// Returns `Some(new_satisfaction)` if the pet is still alive, `None` if it died.
    ///
    pub fn apply_pet_decay(&self, sid: SessionId, amount: i16, now_unix: u64) -> Option<i16> {
        let mut result = None;
        self.update_session(sid, |h| {
            h.last_pet_decay_time = now_unix;
            if let Some(ref mut pet) = h.pet_data {
                pet.satisfaction = (pet.satisfaction + amount).clamp(0, 10000);
                if pet.satisfaction <= 0 {
                    // Pet dies — remove pet data
                    h.pet_data = None;
                    result = None;
                } else {
                    result = Some(pet.satisfaction);
                }
            }
        });
        result
    }

    /// Get pet data for building death/satisfaction packets.
    ///
    /// Returns `(pet_nid, pet_index)` if the session has an active pet.
    pub fn get_pet_packet_data(&self, sid: SessionId) -> Option<(u16, u32, i16)> {
        self.with_session(sid, |h| {
            h.pet_data
                .as_ref()
                .map(|p| (p.nid, p.index, p.satisfaction))
        })
        .flatten()
    }

    /// Find the owner session of a pet NPC by its runtime NPC ID.
    ///
    /// Iterates sessions to locate the one whose `pet_data.nid` matches.
    /// Returns the owner's session ID if found.
    pub fn find_pet_owner_by_nid(&self, pet_npc_id: u16) -> Option<SessionId> {
        if pet_npc_id == 0 {
            return None;
        }
        for entry in self.sessions.iter() {
            if let Some(ref pet) = entry.value().pet_data {
                if pet.nid == pet_npc_id {
                    return Some(*entry.key());
                }
            }
        }
        None
    }

    /// Collect all sessions whose pet is currently in family-attack mode.
    ///
    /// Returns a snapshot of the pet attack state for each qualifying session.
    /// Used by `pet_attack_tick` to process pet auto-attacks without holding
    /// DashMap references.
    ///
    pub fn collect_pet_attack_data(&self) -> Vec<PetAttackData> {
        let mut data = Vec::with_capacity(16);
        for entry in self.sessions.iter() {
            let handle = entry.value();
            if handle.character.is_none() {
                continue;
            }
            if let Some(ref pet) = handle.pet_data {
                if !pet.attack_started || pet.attack_target_id < 0 {
                    continue;
                }
                let sid = *entry.key();
                let zone_id = self.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
                let is_dead = self.is_player_dead(sid);
                data.push(PetAttackData {
                    session_id: sid,
                    pet_nid: pet.nid,
                    target_npc_id: pet.attack_target_id as u32,
                    owner_zone_id: zone_id,
                    owner_dead: is_dead,
                });
            }
        }
        data
    }

    /// Collect session IDs that have zone online reward timers initialised.
    ///
    /// Used by the background zone online reward tick to find eligible players.
    pub fn collect_zone_online_reward_session_ids(&self) -> Vec<SessionId> {
        let mut ids = Vec::with_capacity(16);
        for entry in self.sessions.iter() {
            let h = entry.value();
            if h.character.is_some() && !h.zone_online_reward_timers.is_empty() {
                ids.push(*entry.key());
            }
        }
        ids
    }

    // ── Knight Cash (KC) / TL Balance ────────────────────────────────────

    /// Get Knight Cash balance for a session.
    ///
    pub fn get_knight_cash(&self, id: SessionId) -> u32 {
        self.with_session(id, |h| h.knight_cash).unwrap_or(0)
    }

    /// Get TL balance for a session.
    ///
    pub fn get_tl_balance(&self, id: SessionId) -> u32 {
        self.with_session(id, |h| h.tl_balance).unwrap_or(0)
    }

    /// Set Knight Cash and TL balances for a session.
    ///
    /// Called after loading from DB on gamestart, or after a successful purchase.
    pub fn set_kc_balance(&self, id: SessionId, knight_cash: u32, tl_balance: u32) {
        self.update_session(id, |h| {
            h.knight_cash = knight_cash;
            h.tl_balance = tl_balance;
        });
    }

    // ── Zone-Nation Broadcasting ───────────────────────────────────────

    /// Broadcast a packet to all in-game sessions in a specific zone that match
    /// the given nation.
    ///
    /// Used by the wanted event to send position updates to the enemy nation.
    pub fn broadcast_to_zone_nation(
        &self,
        zone_id: u16,
        nation: u8,
        packet: Arc<Packet>,
        except: Option<SessionId>,
    ) {
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if except == Some(sid) {
                    continue;
                }
                if let Some(handle) = self.sessions.get(&sid) {
                    if let Some(ref ch) = handle.character {
                        if ch.nation == nation {
                            let _ = handle.tx.send(Arc::clone(&packet));
                        }
                    }
                }
            }
        }
    }

    // ── Wanted Event Helpers ──────────────────────────────────────────

    /// Access the wanted event rooms (read-only lock handle).
    ///
    pub fn wanted_rooms(
        &self,
    ) -> &parking_lot::RwLock<[crate::world::WantedEventRoom; crate::world::MAX_WANTED_ROOMS]> {
        &self.wanted_rooms
    }

    /// Collect wanted players in a given zone for position broadcast.
    ///
    /// Returns `Vec<(session_id, nation, x, z, name)>` for wanted players that
    /// are alive and in the specified PK zone.
    ///
    pub fn collect_wanted_players_in_zone(
        &self,
        zone_id: u16,
    ) -> Vec<(SessionId, u8, u16, u16, String)> {
        let mut result = Vec::new();
        if let Some(index_entry) = self.zone_session_index.get(&zone_id) {
            let session_ids: Vec<SessionId> = index_entry.value().read().iter().copied().collect();
            for sid in session_ids {
                if let Some(handle) = self.sessions.get(&sid) {
                    if let Some(ref ch) = handle.character {
                        if handle.is_wanted && ch.res_hp_type != USER_DEAD && ch.hp > 0 {
                            result.push((
                                sid,
                                ch.nation,
                                handle.position.x as u16,
                                handle.position.z as u16,
                                ch.name.clone(),
                            ));
                        }
                    }
                }
            }
        }
        result
    }

    /// Reset all online players' Draki Tower entrance limits.
    ///
    pub fn reset_draki_entrance_limits(&self) {
        for mut entry in self.sessions.iter_mut() {
            entry.value_mut().draki_entrance_limit =
                crate::handler::draki_tower::MAX_ENTRANCE_LIMIT;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ── Sprint 58: Session lookup tests ─────────────────────────────────

    #[test]
    fn test_find_session_by_account_found() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.account_id = "TestUser".to_string();
        });

        let result = world.find_session_by_account("TestUser");
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_find_session_by_account_not_found() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.account_id = "OtherUser".to_string();
        });

        let result = world.find_session_by_account("UnknownUser");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_session_by_account_case_insensitive() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.account_id = "TestAcct".to_string();
        });

        // Search with different case
        let result = world.find_session_by_account("testacct");
        assert_eq!(result, Some(1));
    }

    #[tokio::test]
    async fn test_kick_session_removes_from_world() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Verify session exists
        assert_eq!(world.sessions.len(), 1);

        // Kick the session
        world.kick_session_for_duplicate(1).await;

        // Verify session was removed
        assert_eq!(world.sessions.len(), 0);
    }

    #[test]
    fn test_login_lock_acquire_and_release() {
        let world = WorldState::new();

        // First acquire succeeds
        assert!(world.try_acquire_login_lock("TestAccount"));
        // Second acquire for same account (case-insensitive) fails
        assert!(!world.try_acquire_login_lock("testaccount"));
        assert!(!world.try_acquire_login_lock("TESTACCOUNT"));

        // Different account succeeds
        assert!(world.try_acquire_login_lock("OtherAccount"));

        // Release first account
        world.release_login_lock("TestAccount");
        // Now re-acquire succeeds
        assert!(world.try_acquire_login_lock("testaccount"));

        // Cleanup
        world.release_login_lock("testaccount");
        world.release_login_lock("OtherAccount");
        assert_eq!(world.login_in_progress.len(), 0);
    }

    // ── Sprint 77: Session ID zero-skip test ────────────────────────────

    #[test]
    fn test_allocate_session_id_never_returns_zero() {
        let world = WorldState::new();

        // Allocate many session IDs — none should be 0
        for _ in 0..1000 {
            let id = world.allocate_session_id();
            assert_ne!(id, 0, "allocate_session_id must never return 0");
        }
    }

    #[test]
    fn test_allocate_session_id_skips_zero_on_wrap() {
        let world = WorldState::new();
        // Set counter to u16::MAX so next fetch_add wraps to 0
        world.next_session_id.store(u16::MAX, Ordering::Relaxed);

        // First call: fetch_add returns u16::MAX (valid), counter wraps to 0
        let id1 = world.allocate_session_id();
        assert_eq!(id1, u16::MAX);

        // Second call: fetch_add returns 0 (skipped), then returns 1
        let id2 = world.allocate_session_id();
        assert_ne!(id2, 0, "must skip 0 on wrap-around");
        assert_eq!(id2, 1);
    }

    // ── Sprint 196: event_room tracking tests ────────────────────────────

    #[test]
    fn test_event_room_default_zero() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let room = world.get_event_room(1);
        assert_eq!(room, 0, "Default event_room should be 0");
    }

    #[test]
    fn test_event_room_set_and_get() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.update_session(1, |h| {
            h.event_room = 42;
        });

        assert_eq!(world.get_event_room(1), 42);
    }

    #[test]
    fn test_is_same_event_room_both_zero() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Both in event_room 0 (no event room) — same
        assert!(world.is_same_event_room(1, 2));
    }

    #[test]
    fn test_is_same_event_room_both_same_nonzero() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| h.event_room = 5);
        world.update_session(2, |h| h.event_room = 5);

        assert!(world.is_same_event_room(1, 2));
    }

    #[test]
    fn test_is_same_event_room_different_rooms() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| h.event_room = 3);
        world.update_session(2, |h| h.event_room = 7);

        assert!(!world.is_same_event_room(1, 2));
    }

    #[test]
    fn test_is_same_event_room_one_zero_one_nonzero() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| h.event_room = 0);
        world.update_session(2, |h| h.event_room = 5);

        assert!(!world.is_same_event_room(1, 2));
    }

    #[test]
    fn test_event_room_clear_on_exit() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.update_session(1, |h| h.event_room = 10);
        assert_eq!(world.get_event_room(1), 10);

        // Simulate exit: clear event_room
        world.update_session(1, |h| h.event_room = 0);
        assert_eq!(world.get_event_room(1), 0);
    }

    #[test]
    fn test_get_event_room_nonexistent_session() {
        let world = WorldState::new();
        // Session 999 doesn't exist
        assert_eq!(world.get_event_room(999), 0);
    }

    #[test]
    fn test_is_same_event_room_nonexistent_session() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Session 2 doesn't exist — both default to 0, so "same"
        assert!(world.is_same_event_room(1, 2));
    }

    #[tokio::test]
    async fn test_get_nearby_session_ids_filters_by_event_room() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        // All have character info (required for get_nearby_session_ids)
        let make_char = |sid| {
            let ch = crate::world::CharacterInfo {
                session_id: sid,
                name: format!("P{}", sid),
                nation: 1,
                race: 1,
                class: 101,
                level: 60,
                face: 1,
                hair_rgb: 0,
                rank: 0,
                title: 0,
                max_hp: 5000,
                hp: 5000,
                max_mp: 3000,
                mp: 3000,
                max_sp: 0,
                sp: 0,
                equipped_items: [0u32; 14],
                bind_zone: 0,
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
                max_exp: 100_000,
                exp_seal_status: false,
                sealed_exp: 0,
                item_weight: 0,
                max_weight: 1000,
                res_hp_type: 1,
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
            ch
        };

        world.update_session(1, |h| {
            h.character = Some(make_char(1));
            h.event_room = 3; // Room 3
        });
        world.update_session(2, |h| {
            h.character = Some(make_char(2));
            h.event_room = 3; // Room 3 (same)
        });
        world.update_session(3, |h| {
            h.character = Some(make_char(3));
            h.event_room = 5; // Room 5 (different)
        });

        // Place all in the same zone/region
        world.update_position(1, 84, 500.0, 0.0, 500.0);
        world.update_position(2, 84, 500.0, 0.0, 500.0);
        world.update_position(3, 84, 500.0, 0.0, 500.0);

        // Add to zone region grid
        if let Some(zone) = world.get_zone(84) {
            let rx = crate::zone::calc_region(500.0);
            let rz = crate::zone::calc_region(500.0);
            zone.add_user(rx, rz, 1);
            zone.add_user(rx, rz, 2);
            zone.add_user(rx, rz, 3);

            // Query from sid=1 (event_room=3): should see sid=2 (room 3) but not sid=3 (room 5)
            let nearby = world.get_nearby_session_ids(84, rx, rz, Some(1), 3);
            assert!(nearby.contains(&2), "sid=2 (room 3) should be visible");
            assert!(
                !nearby.contains(&3),
                "sid=3 (room 5) should be filtered out"
            );
            assert!(
                !nearby.contains(&1),
                "sid=1 (self) should be excluded via except"
            );
        }
    }

    #[tokio::test]
    async fn test_get_nearby_session_ids_room_zero_matches_zero() {
        // In non-event zones, all players have event_room=0, so all match.
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let make_char = |sid| crate::world::CharacterInfo {
            session_id: sid,
            name: format!("P{}", sid),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0u32; 14],
            bind_zone: 0,
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
            max_exp: 100_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
            res_hp_type: 1,
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

        // Both in normal zone with event_room=0
        world.update_session(1, |h| {
            h.character = Some(make_char(1));
            h.event_room = 0;
        });
        world.update_session(2, |h| {
            h.character = Some(make_char(2));
            h.event_room = 0;
        });

        world.update_position(1, 21, 500.0, 0.0, 500.0);
        world.update_position(2, 21, 500.0, 0.0, 500.0);

        if let Some(zone) = world.get_zone(21) {
            let rx = crate::zone::calc_region(500.0);
            let rz = crate::zone::calc_region(500.0);
            zone.add_user(rx, rz, 1);
            zone.add_user(rx, rz, 2);

            let nearby = world.get_nearby_session_ids(21, rx, rz, Some(1), 0);
            assert!(
                nearby.contains(&2),
                "Normal zone: room=0 should match room=0"
            );
        }
    }

    // ── Sprint 286: Potion cooldown tests ───────────────────────────────

    #[test]
    fn test_potion_cooldown_constant_matches_cpp() {
        let cooldown_ms: u128 = 2400;
        assert_eq!(
            cooldown_ms, 2400,
            "Potion cooldown must be 2400ms per C++ PLAYER_POTION_REQUEST_INTERVAL"
        );
    }

    #[test]
    fn test_potion_cooldown_field_initialized_in_past() {
        // SessionHandle.last_potion_time is initialized 3s in the past,
        // so the first potion use is always allowed.
        let init_offset = std::time::Duration::from_secs(3);
        let cooldown = std::time::Duration::from_millis(2400);
        assert!(
            init_offset > cooldown,
            "Initial 3s offset must exceed 2400ms cooldown"
        );
    }

    // ── Sprint 592: zone_changing safety timeout tests ─────────────────

    #[test]
    fn test_zone_changing_flag_basic() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        assert!(!world.is_zone_changing(1));
        world.set_zone_changing(1, true);
        assert!(world.is_zone_changing(1));
        world.set_zone_changing(1, false);
        assert!(!world.is_zone_changing(1));
    }

    #[test]
    fn test_zone_changing_timeout_auto_clear() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Manually set zone_changing=true with a timestamp 31s in the past
        world.update_session(1, |h| {
            h.zone_changing = true;
            h.zone_change_started_at = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(31))
                .unwrap_or(std::time::Instant::now());
        });

        // is_zone_changing should auto-clear and return false after 30s
        let started_at = world.with_session(1, |h| h.zone_change_started_at).unwrap();
        if started_at.elapsed().as_secs() >= 30 {
            assert!(
                !world.is_zone_changing(1),
                "zone_changing should auto-clear after 30s"
            );
            // Verify it actually cleared the flag
            let raw = world.with_session(1, |h| h.zone_changing).unwrap();
            assert!(!raw);
        }
    }

    // ── Sprint 957: Additional coverage ──────────────────────────────

    /// is_gm returns false for non-existent session.
    #[test]
    fn test_is_gm_nonexistent() {
        let world = WorldState::new();
        assert!(!world.is_gm(999));
    }

    /// is_player_dead returns false for non-existent session.
    #[test]
    fn test_is_player_dead_nonexistent() {
        let world = WorldState::new();
        assert!(!world.is_player_dead(999));
    }

    /// session_count and online_count start at zero.
    #[test]
    fn test_session_and_online_counts_initial() {
        let world = WorldState::new();
        assert_eq!(world.session_count(), 0);
        assert_eq!(world.online_count(), 0);
    }

    /// store_open defaults to false and can be toggled.
    #[test]
    fn test_store_open_toggle() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        assert!(!world.is_store_open(1));
        world.set_store_open(1, true);
        assert!(world.is_store_open(1));
        world.set_store_open(1, false);
        assert!(!world.is_store_open(1));
    }

    /// get_character_info returns None when no character is registered.
    #[test]
    fn test_get_character_info_no_character() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(world.get_character_info(1).is_none());
    }

    // ── Sprint 963: Additional coverage ──────────────────────────────

    /// Invisibility defaults to 0 (visible) and can be set/cleared.
    #[test]
    fn test_invisibility_toggle() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert_eq!(world.get_invisibility_type(1), 0);
        assert!(!world.is_invisible(1));
        world.set_invisibility_type(1, 5);
        assert_eq!(world.get_invisibility_type(1), 5);
        assert!(world.is_invisible(1));
    }

    /// is_transformed defaults to false.
    #[test]
    fn test_is_transformed_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_transformed(1));
        assert!(!world.is_transformed(999)); // nonexistent
    }

    /// Combat flags default to safe values (not blocked, not blinded).
    #[test]
    fn test_combat_flags_defaults() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_blinded(1));
        assert!(!world.is_block_physical(1));
        assert!(!world.is_block_magic(1));
        assert!(!world.is_undead(1));
        assert!(!world.is_devil(1));
    }

    /// can_use_skills and can_use_potions default to true.
    #[test]
    fn test_skill_potion_defaults() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(world.can_use_skills(1));
        assert!(world.can_use_potions(1));
    }

    /// get_abnormal_type defaults to 1 (ABNORMAL_NORMAL).
    #[test]
    fn test_abnormal_type_default() {
        let world = WorldState::new();
        assert_eq!(world.get_abnormal_type(999), 1); // nonexistent → default
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert_eq!(world.get_abnormal_type(1), 1);
    }

    // ── Sprint 967: Additional coverage ──────────────────────────────

    /// zone_changing flag set/clear and auto-clear after timeout.
    #[test]
    fn test_zone_changing_flag() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_zone_changing(1));
        world.set_zone_changing(1, true);
        assert!(world.is_zone_changing(1));
        world.set_zone_changing(1, false);
        assert!(!world.is_zone_changing(1));
    }

    /// check_warp_zone_change flag defaults to false.
    #[test]
    fn test_warp_zone_change_flag() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_check_warp_zone_change(1));
        world.set_check_warp_zone_change(1, true);
        assert!(world.is_check_warp_zone_change(1));
        world.set_check_warp_zone_change(1, false);
        assert!(!world.is_check_warp_zone_change(1));
    }

    /// attack_disabled defaults to false (not disabled).
    #[test]
    fn test_attack_disabled_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_attack_disabled(1));
        // nonexistent session
        assert!(!world.is_attack_disabled(999));
    }

    /// is_player_blinking returns false when blink_expiry_time is 0.
    #[test]
    fn test_blinking_default_false() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!world.is_player_blinking(1, now));
    }

    /// is_session_ingame returns false before register_ingame.
    #[test]
    fn test_is_session_ingame() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_session_ingame(1));
        // nonexistent
        assert!(!world.is_session_ingame(999));
    }

    // ── Sprint 972: Additional coverage ──────────────────────────────

    /// is_undead, is_blinded, is_devil default to false.
    #[test]
    fn test_status_flags_default_false() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_undead(1));
        assert!(!world.is_blinded(1));
        assert!(!world.is_devil(1));
        // nonexistent returns false
        assert!(!world.is_undead(999));
        assert!(!world.is_blinded(999));
        assert!(!world.is_devil(999));
    }

    /// can_teleport and can_stealth default to true.
    #[test]
    fn test_teleport_stealth_defaults_true() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(world.can_teleport(1));
        assert!(world.can_stealth(1));
        // nonexistent also returns true (safe default)
        assert!(world.can_teleport(999));
        assert!(world.can_stealth(999));
    }

    /// block_physical and block_magic default to false.
    #[test]
    fn test_block_physical_magic_defaults() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_block_physical(1));
        assert!(!world.is_block_magic(1));
        // nonexistent
        assert!(!world.is_block_physical(999));
        assert!(!world.is_block_magic(999));
    }

    /// with_session returns None for missing sessions, Some for existing.
    #[test]
    fn test_with_session_some_none() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let result = world.with_session(1, |h| h.zone_changing);
        assert_eq!(result, Some(false));
        let missing = world.with_session(999, |h| h.zone_changing);
        assert!(missing.is_none());
    }

    /// online_count starts at 0 and reflects registered sessions.
    #[test]
    fn test_online_count_tracks_sessions() {
        let world = WorldState::new();
        assert_eq!(world.online_count(), 0);
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        // register_session adds to sessions map but online_count counts in-game only
        // so it should still be 0 (no character set)
        assert_eq!(world.online_count(), 0);
    }

    /// set_invisibility_type / get_invisibility_type / is_invisible round-trip.
    #[test]
    fn test_invisibility_type_set_get() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        // Default is 0 (INVIS_NONE)
        assert_eq!(world.get_invisibility_type(1), 0);
        assert!(!world.is_invisible(1));
        // Set stealth type 1
        world.set_invisibility_type(1, 1);
        assert_eq!(world.get_invisibility_type(1), 1);
        assert!(world.is_invisible(1));
        // Nonexistent session returns 0
        assert_eq!(world.get_invisibility_type(999), 0);
        assert!(!world.is_invisible(999));
    }

    /// clear_blink resets blink_expiry_time and re-enables skills.
    #[test]
    fn test_clear_blink_resets_state() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        // Set blink state manually
        world.update_session(1, |h| {
            h.blink_expiry_time = 99999;
            h.can_use_skills = false;
        });
        // Verify blinking
        assert!(world.is_player_blinking(1, 50000));
        assert!(!world.can_use_skills(1));
        // Clear blink
        world.clear_blink(1);
        assert!(!world.is_player_blinking(1, 50000));
        assert!(world.can_use_skills(1));
    }

    /// get_cast_position returns Some only when skill_id matches.
    #[test]
    fn test_cast_position_skill_match() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.cast_skill_id = 500100;
            h.cast_x = 100.5;
            h.cast_z = 200.5;
        });
        // Matching skill_id → Some
        let pos = world.get_cast_position(1, 500100);
        assert_eq!(pos, Some((100.5, 200.5)));
        // Non-matching skill_id → None
        assert!(world.get_cast_position(1, 999999).is_none());
        // Nonexistent session → None
        assert!(world.get_cast_position(999, 500100).is_none());
    }

    /// session_count tracks pre-game sessions (before character load).
    #[test]
    fn test_session_count_vs_online_count() {
        let world = WorldState::new();
        assert_eq!(world.session_count(), 0);
        assert_eq!(world.online_count(), 0);
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        // session_count includes pre-game, online_count does not
        assert_eq!(world.session_count(), 2);
        assert_eq!(world.online_count(), 0);
    }

    /// get_abnormal_type defaults to 1 (ABNORMAL_NORMAL).
    #[test]
    fn test_abnormal_type_default_value() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert_eq!(world.get_abnormal_type(1), 1);
        // Nonexistent session also returns 1
        assert_eq!(world.get_abnormal_type(999), 1);
    }

    /// set_transformation / is_transformed / clear_transformation lifecycle.
    #[test]
    fn test_transformation_lifecycle() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_transformed(1));
        world.set_transformation(1, 1, 500, 600100, 10000, 30000);
        assert!(world.is_transformed(1));
        // Verify fields via with_session
        let (t_type, t_id) = world
            .with_session(1, |h| (h.transformation_type, h.transform_id))
            .unwrap();
        assert_eq!(t_type, 1);
        assert_eq!(t_id, 500);
        // Clear
        world.clear_transformation(1);
        assert!(!world.is_transformed(1));
    }

    /// collect_expired_blinks finds expired blink sessions.
    #[test]
    fn test_collect_expired_blinks() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        // Session 1: blink expires at 100, session 2: blink expires at 200
        world.update_session(1, |h| {
            h.blink_expiry_time = 100;
        });
        world.update_session(2, |h| {
            h.blink_expiry_time = 200;
        });
        // At time 150: session 1 expired, session 2 still active
        let expired = world.collect_expired_blinks(150);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, 1);
    }

    /// can_use_potions defaults to true, returns true for nonexistent.
    #[test]
    fn test_can_use_potions_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(world.can_use_potions(1));
        // Disable potions
        world.update_session(1, |h| {
            h.can_use_potions = false;
        });
        assert!(!world.can_use_potions(1));
        // Nonexistent → true (safe default)
        assert!(world.can_use_potions(999));
    }

    /// unregister_session removes session and decrements online_count if ingame.
    #[test]
    fn test_unregister_removes_session() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert_eq!(world.session_count(), 1);
        world.unregister_session(1);
        assert_eq!(world.session_count(), 0);
        // with_session returns None after unregister
        assert!(world.with_session(1, |_| true).is_none());
    }

    /// touch_session updates last_response_time to now.
    #[test]
    fn test_touch_session_updates_time() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let before = world.with_session(1, |h| h.last_response_time).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        world.touch_session(1);
        let after = world.with_session(1, |h| h.last_response_time).unwrap();
        assert!(after >= before);
    }

    /// update_session closure modifies session fields.
    #[test]
    fn test_update_session_closure() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.is_mining = true;
            h.is_fishing = true;
        });
        let (mining, fishing) = world
            .with_session(1, |h| (h.is_mining, h.is_fishing))
            .unwrap();
        assert!(mining);
        assert!(fishing);
    }

    /// set_check_warp_zone_change round-trip.
    #[test]
    fn test_warp_zone_change_set_get() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_check_warp_zone_change(1));
        world.set_check_warp_zone_change(1, true);
        assert!(world.is_check_warp_zone_change(1));
        world.set_check_warp_zone_change(1, false);
        assert!(!world.is_check_warp_zone_change(1));
        // Nonexistent returns false
        assert!(!world.is_check_warp_zone_change(999));
    }

    /// collect_expired_transformations finds sessions past duration.
    #[test]
    fn test_collect_expired_transformations() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        // Session 1: transform started at 1000, duration 500 → expires at 1500
        world.set_transformation(1, 1, 100, 600100, 1000, 500);
        // Session 2: transform started at 1000, duration 5000 → expires at 6000
        world.set_transformation(2, 1, 200, 600200, 1000, 5000);
        // At time 2000: session 1 expired (2000-1000=1000 >= 500), session 2 active
        let expired = world.collect_expired_transformations(2000);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, 1); // session_id
        assert_eq!(expired[0].1, 600100); // skill_id
    }

    /// find_session_by_account is case-insensitive.
    #[test]
    fn test_find_session_by_account_case() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.account_id = "TestAccount".to_string();
        });
        assert_eq!(world.find_session_by_account("testaccount"), Some(1));
        assert_eq!(world.find_session_by_account("TESTACCOUNT"), Some(1));
        assert!(world.find_session_by_account("other").is_none());
    }

    // ── Sprint 999: Additional coverage ──────────────────────────────

    /// Session sentinel defaults: gm_send_pm_id=0xFFFF, event_nid/sid=-1, by_selected_reward=-1.
    #[test]
    fn test_session_sentinel_defaults() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let (pm_id, event_nid, event_sid, reward) = world
            .with_session(1, |h| {
                (
                    h.gm_send_pm_id,
                    h.event_nid,
                    h.event_sid,
                    h.by_selected_reward,
                )
            })
            .unwrap();
        assert_eq!(pm_id, 0xFFFF);
        assert_eq!(event_nid, -1);
        assert_eq!(event_sid, -1);
        assert_eq!(reward, -1);
    }

    /// Draki entrance limit defaults to 3 (MAX_ENTRANCE_LIMIT).
    #[test]
    fn test_session_draki_entrance_limit_default() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let limit = world.with_session(1, |h| h.draki_entrance_limit).unwrap();
        assert_eq!(limit, 3);
    }

    /// All elemental resists default to 100 (100% = no reduction).
    #[test]
    fn test_session_elemental_resist_defaults() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let resists = world
            .with_session(1, |h| {
                (
                    h.pct_fire_r,
                    h.pct_cold_r,
                    h.pct_lightning_r,
                    h.pct_magic_r,
                    h.pct_disease_r,
                    h.pct_poison_r,
                )
            })
            .unwrap();
        assert_eq!(resists, (100, 100, 100, 100, 100, 100));
    }

    /// Buff multiplier defaults: np/noah/weight gain all 100, magic_damage_reduction 100.
    #[test]
    fn test_session_buff_multiplier_defaults() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let (np, noah, weight, mdr) = world
            .with_session(1, |h| {
                (
                    h.np_gain_amount,
                    h.noah_gain_amount,
                    h.weight_buff_amount,
                    h.magic_damage_reduction,
                )
            })
            .unwrap();
        assert_eq!(np, 100);
        assert_eq!(noah, 100);
        assert_eq!(weight, 100);
        assert_eq!(mdr, 100);
    }

    /// Ranged amounts default to 100 and soul_categories follow sequential pattern.
    #[test]
    fn test_session_ranged_amounts_and_soul_layout() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let (dagger, bow, soul_cats) = world
            .with_session(1, |h| {
                (h.dagger_r_amount, h.bow_r_amount, h.soul_categories)
            })
            .unwrap();
        assert_eq!(dagger, 100);
        assert_eq!(bow, 100);
        // Soul categories: 8 entries, first element = index (0-7), rest zeros
        for i in 0..8 {
            assert_eq!(
                soul_cats[i][0], i as i16,
                "soul_categories[{}][0] should be {}",
                i, i
            );
            assert_eq!(soul_cats[i][1], 0i16);
            assert_eq!(soul_cats[i][2], 0i16);
            assert_eq!(soul_cats[i][3], 0i16);
        }
    }
}
