//! Party, knights (clan), alliance, chat rooms, king system, ranking, and seeking party.

use std::sync::Arc;

use super::*;

const KNIGHTS_CLAN_BONUS: u8 = 98;

impl WorldState {
    // ── Knights (Clan) Accessors ──────────────────────────────────────

    /// Look up a clan by ID.
    ///
    pub fn get_knights(&self, clan_id: u16) -> Option<KnightsInfo> {
        self.knights.get(&clan_id).map(|r| r.clone())
    }
    /// Get all clan IDs and names (for WIZ_KNIGHTS_LIST).
    ///
    pub fn get_all_knights(&self) -> Vec<(u16, String)> {
        self.knights
            .iter()
            .map(|r| (r.id, r.name.clone()))
            .collect()
    }
    /// Get all clan IDs from the runtime table.
    ///
    /// Used by the periodic knights save task to iterate all clans.
    pub fn get_all_knights_ids(&self) -> Vec<u16> {
        self.knights.iter().map(|r| *r.key()).collect()
    }

    /// Insert or replace a clan in the runtime table.
    pub fn insert_knights(&self, info: KnightsInfo) {
        self.knights.insert(info.id, info);
    }
    /// Remove a clan from the runtime table (disband).
    pub fn remove_knights(&self, clan_id: u16) -> Option<KnightsInfo> {
        self.knights.remove(&clan_id).map(|(_, v)| v)
    }
    /// Find a clan by name (case-sensitive, linear scan).
    ///
    /// Used by the tournament system to look up clans by name string.
    ///
    pub fn find_knights_by_name(&self, name: &str) -> Option<KnightsInfo> {
        self.knights
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.clone())
    }

    // ── Tournament System Accessors ───────────────────────────────────────

    /// Insert or replace a tournament arena entry.
    ///
    pub fn insert_tournament(&self, state: crate::handler::tournament::TournamentState) {
        self.tournament_registry.insert(state.zone_id, state);
    }

    /// Remove a tournament arena entry by zone ID.
    ///
    pub fn remove_tournament(&self, zone_id: u16) {
        self.tournament_registry.remove(&zone_id);
    }

    /// Mutate the tournament state for a zone via a closure (if it exists).
    ///
    pub fn with_tournament(
        &self,
        zone_id: u16,
        f: impl FnOnce(&mut crate::handler::tournament::TournamentState),
    ) {
        if let Some(mut entry) = self.tournament_registry.get_mut(&zone_id) {
            f(&mut entry);
        }
    }

    /// Get a snapshot (clone) of the tournament state for a zone.
    ///
    /// Returns `None` if no tournament is active for that zone.
    pub fn with_tournament_snapshot(
        &self,
        zone_id: u16,
    ) -> Option<crate::handler::tournament::TournamentState> {
        self.tournament_registry.get(&zone_id).map(|r| r.clone())
    }
    /// Get top N clans for a nation, sorted by points (descending).
    ///
    /// Returns up to `limit` clans with (id, name, mark_version) for the given nation.
    pub fn get_top_knights_by_nation(&self, nation: u8, limit: usize) -> Vec<(u16, String, u16)> {
        let mut clans: Vec<(u32, u16, String, u16)> = self
            .knights
            .iter()
            .filter(|r| r.nation == nation && !r.name.is_empty())
            .map(|r| (r.points, r.id, r.name.clone(), r.mark_version))
            .collect();
        clans.sort_by(|a, b| b.0.cmp(&a.0)); // sort by points DESC
        clans
            .into_iter()
            .take(limit)
            .map(|(_, id, name, mv)| (id, name, mv))
            .collect()
    }
    /// Update a clan's data via a closure.
    pub fn update_knights(&self, clan_id: u16, updater: impl FnOnce(&mut KnightsInfo)) {
        if let Some(mut entry) = self.knights.get_mut(&clan_id) {
            updater(&mut entry);
        }
    }
    /// Check if a clan name already exists (case-insensitive).
    ///
    pub fn knights_name_exists(&self, name: &str) -> bool {
        let upper = name.to_uppercase();
        for entry in self.knights.iter() {
            if entry.value().name.to_uppercase() == upper {
                return true;
            }
        }
        false
    }
    /// Send a packet to all online members of a clan.
    ///
    pub fn send_to_knights_members(
        &self,
        clan_id: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
    ) {
        for entry in self.sessions.iter() {
            let sid = *entry.key();
            if except == Some(sid) {
                continue;
            }
            if let Some(ref ch) = entry.value().character {
                if ch.knights_id == clan_id {
                    let _ = entry.value().tx.send(Arc::clone(&packet));
                }
            }
        }
    }
    /// Collect online clan members for listing.
    ///
    /// Returns `(name, fame, level, class)` for each online member in the clan.
    pub fn get_online_knights_members(&self, clan_id: u16) -> Vec<(String, u8, u8, u16)> {
        let mut members = Vec::new();
        for entry in self.sessions.iter() {
            if let Some(ref ch) = entry.value().character {
                if ch.knights_id == clan_id {
                    members.push((ch.name.clone(), ch.fame, ch.level, ch.class));
                }
            }
        }
        members
    }
    /// Get all online session IDs that belong to a clan.
    ///
    pub fn get_online_knights_session_ids(&self, clan_id: u16) -> Vec<SessionId> {
        let mut sids = Vec::new();
        for entry in self.sessions.iter() {
            if let Some(ref ch) = entry.value().character {
                if ch.knights_id == clan_id {
                    sids.push(*entry.key());
                }
            }
        }
        sids
    }
    /// Clear the clan from all online sessions (used when a clan is disbanded).
    ///
    pub fn clear_knights_from_sessions(&self, clan_id: u16) {
        for mut entry in self.sessions.iter_mut() {
            if let Some(ref mut ch) = entry.value_mut().character {
                if ch.knights_id == clan_id {
                    ch.knights_id = 0;
                    ch.fame = 0;
                }
            }
        }
    }
    // ── Alliance Accessors ────────────────────────────────────────────

    /// Look up an alliance by its main clan ID.
    ///
    pub fn get_alliance(&self, main_clan_id: u16) -> Option<KnightsAlliance> {
        self.alliances.get(&main_clan_id).map(|r| r.clone())
    }
    /// Insert or replace an alliance in the runtime table.
    pub fn insert_alliance(&self, alliance: KnightsAlliance) {
        self.alliances.insert(alliance.main_clan, alliance);
    }
    /// Remove an alliance from the runtime table.
    pub fn remove_alliance(&self, main_clan_id: u16) -> Option<KnightsAlliance> {
        self.alliances.remove(&main_clan_id).map(|(_, v)| v)
    }
    /// Update an alliance's data via a closure.
    pub fn update_alliance(&self, main_clan_id: u16, updater: impl FnOnce(&mut KnightsAlliance)) {
        if let Some(mut entry) = self.alliances.get_mut(&main_clan_id) {
            updater(&mut entry);
        }
    }
    /// Send a packet to all online members of all clans in an alliance.
    ///
    pub fn send_to_alliance_members(
        &self,
        alliance_id: u16,
        packet: Arc<Packet>,
        except: Option<SessionId>,
    ) {
        let alliance = match self.get_alliance(alliance_id) {
            Some(a) => a,
            None => return,
        };

        let clan_ids = [
            alliance.main_clan,
            alliance.sub_clan,
            alliance.mercenary_1,
            alliance.mercenary_2,
        ];

        for &clan_id in &clan_ids {
            if clan_id == 0 {
                continue;
            }
            self.send_to_knights_members(clan_id, Arc::clone(&packet), except);
        }
    }
    // ── Party System Methods ─────────────────────────────────────────

    /// Create a new party with the given leader. Returns the party ID.
    ///
    pub fn create_party(&self, leader_sid: SessionId) -> Option<u16> {
        let party_id = self.next_party_id.fetch_add(1, Ordering::Relaxed);
        if party_id == 0 {
            // Skip 0, it's used as "no party"
            return self.create_party(leader_sid);
        }
        let party = Party::new(party_id, leader_sid);
        self.parties.insert(party_id, party);
        // Set party_id on the leader's CharacterInfo
        if let Some(mut handle) = self.sessions.get_mut(&leader_sid) {
            if let Some(ref mut ch) = handle.character {
                ch.party_id = Some(party_id);
            }
        }
        Some(party_id)
    }
    /// Get a clone of a party by ID.
    pub fn get_party(&self, party_id: u16) -> Option<Party> {
        self.parties.get(&party_id).map(|p| p.clone())
    }
    /// Update a party with a closure. Returns true if the party was found.
    pub fn update_party<F: FnOnce(&mut Party)>(&self, party_id: u16, f: F) -> bool {
        if let Some(mut party) = self.parties.get_mut(&party_id) {
            f(&mut party);
            true
        } else {
            false
        }
    }
    /// Add a member to a party. Returns true on success.
    ///
    pub fn add_party_member(&self, party_id: u16, sid: SessionId) -> bool {
        let added = if let Some(mut party) = self.parties.get_mut(&party_id) {
            party.add_member(sid)
        } else {
            return false;
        };
        if added {
            if let Some(mut handle) = self.sessions.get_mut(&sid) {
                if let Some(ref mut ch) = handle.character {
                    ch.party_id = Some(party_id);
                }
            }
        }
        added
    }
    /// Remove a member from a party. Returns true if found and removed.
    ///
    pub fn remove_party_member(&self, party_id: u16, sid: SessionId) -> bool {
        let removed = if let Some(mut party) = self.parties.get_mut(&party_id) {
            party.remove_member(sid)
        } else {
            return false;
        };
        if removed {
            if let Some(mut handle) = self.sessions.get_mut(&sid) {
                handle.party_type = 0; // Reset party type on leave
                if let Some(ref mut ch) = handle.character {
                    ch.party_id = None;
                }
            }
        }
        removed
    }
    /// Disband an entire party, clearing party_id on all members.
    /// Returns the list of member session IDs that were in the party.
    ///
    pub fn disband_party(&self, party_id: u16) -> Vec<SessionId> {
        let members = if let Some((_, party)) = self.parties.remove(&party_id) {
            party.active_members()
        } else {
            return Vec::new();
        };
        for &sid in &members {
            if let Some(mut handle) = self.sessions.get_mut(&sid) {
                handle.party_type = 0; // Reset party type on disband
                if let Some(ref mut ch) = handle.character {
                    ch.party_id = None;
                }
            }
        }
        members
    }
    /// Promote a new leader in a party.
    ///
    pub fn promote_party_leader(&self, party_id: u16, new_leader_sid: SessionId) -> bool {
        if let Some(mut party) = self.parties.get_mut(&party_id) {
            if let Some(pos) = party.find_slot(new_leader_sid) {
                party.swap_leader(pos);
                return true;
            }
        }
        false
    }
    /// Store a pending party invitation for `invitee_sid`.
    pub fn set_party_invitation(
        &self,
        invitee_sid: SessionId,
        party_id: u16,
        inviter_sid: SessionId,
    ) {
        self.party_invitations
            .insert(invitee_sid, (party_id, inviter_sid));
    }
    /// Take (consume) a pending party invitation for `invitee_sid`.
    pub fn take_party_invitation(&self, invitee_sid: SessionId) -> Option<(u16, SessionId)> {
        self.party_invitations.remove(&invitee_sid).map(|(_, v)| v)
    }
    /// Check if a player has a pending party invitation.
    pub fn has_party_invitation(&self, sid: SessionId) -> bool {
        self.party_invitations.contains_key(&sid)
    }
    /// Send a packet to all members of a party.
    ///
    /// Clones the packet once and shares via Arc to avoid per-recipient cloning.
    pub fn send_to_party(&self, party_id: u16, packet: &Packet) {
        if let Some(party) = self.parties.get(&party_id) {
            let arc_pkt = Arc::new(packet.clone());
            for sid in party.active_members() {
                if let Some(handle) = self.sessions.get(&sid) {
                    let _ = handle.tx.send(Arc::clone(&arc_pkt));
                }
            }
        }
    }
    /// Get the party ID for a real player or runtime bot.
    ///
    /// Real players cache the ID on `CharacterInfo`. Runtime bots have no
    /// session handle, so their membership is resolved from the party roster.
    pub fn get_party_id(&self, sid: SessionId) -> Option<u16> {
        if let Some(handle) = self.sessions.get(&sid) {
            if let Some(party_id) = handle.character.as_ref().and_then(|ch| ch.party_id) {
                // CharacterInfo can briefly retain a stale party ID during
                // roster changes. Trust the live roster only when it contains
                // this session; otherwise resolve from the party maps below.
                if self
                    .parties
                    .get(&party_id)
                    .is_some_and(|party| party.contains(sid))
                {
                    return Some(party_id);
                }
            }
        }

        self.parties
            .iter()
            .find_map(|entry| entry.value().find_slot(sid).map(|_| *entry.key()))
    }
    /// Check if a session is currently in a party.
    pub fn is_in_party(&self, sid: SessionId) -> bool {
        self.get_party_id(sid).is_some()
    }
    /// Clean up party state when a player disconnects.
    ///
    ///
    /// If the disconnecting player is the party leader, promote the next member
    /// before removal. If only one member remains after removal, disband entirely.
    /// Also clears any pending party invitation.
    pub fn cleanup_party_on_disconnect(&self, sid: SessionId) {
        // Clear any pending invitation
        self.party_invitations.remove(&sid);

        let party_id = match self.get_party_id(sid) {
            Some(id) => id,
            None => return,
        };

        let party = match self.get_party(party_id) {
            Some(p) => p,
            None => return,
        };

        // If the disconnecting player is the leader, promote the next member first
        if party.is_leader(sid) {
            // Find first non-leader member
            if let Some(&Some(next_sid)) = party.members.iter().skip(1).find(|m| m.is_some()) {
                self.promote_party_leader(party_id, next_sid);
            }
        }

        // Count how many members will remain after removal
        let remaining = party.active_members().iter().filter(|&&m| m != sid).count();

        if remaining <= 1 {
            // Only one member left — disband the party
            // Broadcast PARTY_DELETE first
            let mut del_pkt = Packet::new(Opcode::WizParty as u8);
            del_pkt.write_u8(0x05); // PARTY_DELETE
            self.send_to_party(party_id, &del_pkt);
            self.disband_party(party_id);
        } else {
            // Broadcast PARTY_REMOVE to remaining members
            let mut remove_pkt = Packet::new(Opcode::WizParty as u8);
            remove_pkt.write_u8(0x04); // PARTY_REMOVE
            remove_pkt.write_u32(sid as u32);
            self.send_to_party(party_id, &remove_pkt);
            self.remove_party_member(party_id, sid);
        }
    }
    // ── Chat Room Methods ────────────────────────────────────────────

    /// Get the chat room index for a session.
    pub fn get_chat_room_index(&self, id: SessionId) -> u16 {
        self.sessions
            .get(&id)
            .map(|h| h.chat_room_index)
            .unwrap_or(0)
    }
    /// Set the chat room index for a session.
    pub fn set_chat_room_index(&self, id: SessionId, room_index: u16) {
        if let Some(mut handle) = self.sessions.get_mut(&id) {
            handle.chat_room_index = room_index;
        }
    }
    /// Create a new chat room and return its index, or None on failure.
    ///
    pub fn create_chat_room(
        &self,
        name: String,
        administrator: String,
        password: String,
        nation: u8,
        max_users: u16,
    ) -> Option<u16> {
        // Check for duplicate room name
        for entry in self.chat_rooms.iter() {
            if entry.value().name.eq_ignore_ascii_case(&name) {
                return None;
            }
        }

        let index = self.next_chat_room_id.fetch_add(1, Ordering::Relaxed);

        let mut room = ChatRoom {
            index,
            name,
            administrator: administrator.clone(),
            password,
            nation,
            max_users,
            current_users: 0,
            members: HashMap::new(),
            next_member_id: 0,
        };

        room.add_user(&administrator);
        self.chat_rooms.insert(index, room);
        Some(index)
    }
    /// Get a chat room by index (immutable reference).
    pub fn get_chat_room(
        &self,
        index: u16,
    ) -> Option<dashmap::mapref::one::Ref<'_, u16, ChatRoom>> {
        self.chat_rooms.get(&index)
    }
    /// Get a mutable reference to a chat room by index.
    pub fn get_chat_room_mut(
        &self,
        index: u16,
    ) -> Option<dashmap::mapref::one::RefMut<'_, u16, ChatRoom>> {
        self.chat_rooms.get_mut(&index)
    }
    /// Remove a chat room entirely.
    pub fn remove_chat_room(&self, index: u16) {
        self.chat_rooms.remove(&index);
    }
    /// Send a packet to all members of a chat room.
    ///
    pub fn send_to_chat_room(&self, room_index: u16, packet: &Packet) {
        let member_names: Vec<String> = {
            match self.chat_rooms.get(&room_index) {
                Some(room) => room.members.values().cloned().collect(),
                None => return,
            }
        };

        let arc_pkt = Arc::new(packet.clone());
        for name in &member_names {
            if let Some(sid) = self.find_session_by_name(name) {
                if let Some(handle) = self.sessions.get(&sid) {
                    let _ = handle.tx.send(Arc::clone(&arc_pkt));
                }
            }
        }
    }
    /// Collect all chat rooms for listing.
    ///
    pub fn list_chat_rooms(&self) -> Vec<(u16, String, bool, u8, u16, u16)> {
        let mut rooms = Vec::new();
        for entry in self.chat_rooms.iter() {
            let room = entry.value();
            rooms.push((
                room.index,
                room.name.clone(),
                room.has_password(),
                room.nation,
                room.current_users,
                room.max_users,
            ));
        }
        rooms
    }
    // ── King System Methods ────────────────────────────────────────────

    /// Get a clone of the king system state for a nation.
    ///
    pub fn get_king_system(&self, nation: u8) -> Option<KingSystem> {
        self.king_systems.get(&nation).map(|r| r.clone())
    }
    /// Check if a character name is the king of a given nation.
    ///
    pub fn is_king(&self, nation: u8, name: &str) -> bool {
        self.king_systems
            .get(&nation)
            .is_some_and(|ks| !ks.king_name.is_empty() && ks.king_name.eq_ignore_ascii_case(name))
    }
    /// Update the king system state for a nation (in-memory).
    pub fn update_king_system<F>(&self, nation: u8, f: F)
    where
        F: FnOnce(&mut KingSystem),
    {
        if let Some(mut entry) = self.king_systems.get_mut(&nation) {
            f(entry.value_mut());
        }
    }
    // ── Siege Warfare Accessors ─────────────────────────────────────────

    /// Get read access to the siege warfare state.
    ///
    pub fn siege_war(&self) -> &tokio::sync::RwLock<SiegeWarfare> {
        &self.siege_war
    }
    /// Access the CSW runtime event state (lifecycle, timers, clan kill list).
    ///
    pub fn csw_event(&self) -> &tokio::sync::RwLock<CswEventState> {
        &self.csw_event
    }
    /// Check if the Cinderella War event is currently active.
    ///
    pub fn is_cinderella_active(&self) -> bool {
        self.cindwar_active
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Check if Zindan War (special event) is currently opened/active.
    ///
    pub fn is_zindan_event_opened(&self) -> bool {
        self.zindan_event_opened
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Set the Zindan War (special event) opened state.
    ///
    pub fn set_zindan_event_opened(&self, opened: bool) {
        self.zindan_event_opened
            .store(opened, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the zone ID of the active Cinderella War event.
    ///
    pub fn cinderella_zone_id(&self) -> u16 {
        self.cindwar_zone_id
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Check if a player is a Cinderella War event participant in the event zone.
    ///
    ///
    /// Returns true if ALL of: event is active, player is in event user set,
    /// and player is currently in the Cinderella zone.
    pub fn is_player_in_cinderella(&self, sid: SessionId) -> bool {
        if !self.is_cinderella_active() {
            return false;
        }
        if !self.cindwar_event_users.contains_key(&sid) {
            return false;
        }
        let cind_zone = self.cinderella_zone_id();
        if cind_zone == 0 {
            return false;
        }
        self.get_position(sid)
            .is_some_and(|pos| pos.zone_id == cind_zone)
    }

    /// Add a player to the Cinderella War event user set.
    pub fn add_cinderella_user(&self, sid: SessionId) {
        self.cindwar_event_users.insert(sid, ());
    }

    /// Remove a player from the Cinderella War event user set.
    pub fn remove_cinderella_user(&self, sid: SessionId) {
        self.cindwar_event_users.remove(&sid);
    }

    /// Set the Cinderella War event active state and zone.
    pub fn set_cinderella_active(&self, active: bool, zone_id: u16) {
        self.cindwar_active
            .store(active, std::sync::atomic::Ordering::Relaxed);
        self.cindwar_zone_id
            .store(zone_id, std::sync::atomic::Ordering::Relaxed);
        if !active {
            self.cindwar_event_users.clear();
        }
    }

    // ── Cinderella War Per-Player + Event State Accessors ──────────

    /// Get a read lock on the global Cinderella event state.
    pub fn cindwar_event(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, crate::handler::cinderella::CindirellaEventState> {
        self.cindwar_event_state.read()
    }

    /// Get a write lock on the global Cinderella event state.
    pub fn cindwar_event_mut(
        &self,
    ) -> parking_lot::RwLockWriteGuard<'_, crate::handler::cinderella::CindirellaEventState> {
        self.cindwar_event_state.write()
    }

    /// Get a copy of a player's Cinderella state.
    pub fn get_cindwar_player(
        &self,
        sid: SessionId,
    ) -> Option<crate::handler::cinderella::CindirellaPlayerState> {
        self.cindwar_player_states.get(&sid).map(|r| r.clone())
    }

    /// Insert or update a player's Cinderella state.
    pub fn set_cindwar_player(
        &self,
        sid: SessionId,
        state: crate::handler::cinderella::CindirellaPlayerState,
    ) {
        self.cindwar_player_states.insert(sid, state);
    }

    /// Update a player's Cinderella state in-place.
    pub fn update_cindwar_player(
        &self,
        sid: SessionId,
        f: impl FnOnce(&mut crate::handler::cinderella::CindirellaPlayerState),
    ) {
        if let Some(mut entry) = self.cindwar_player_states.get_mut(&sid) {
            f(entry.value_mut());
        }
    }

    /// Remove a player's Cinderella state.
    pub fn remove_cindwar_player(
        &self,
        sid: SessionId,
    ) -> Option<crate::handler::cinderella::CindirellaPlayerState> {
        self.cindwar_player_states.remove(&sid).map(|(_, v)| v)
    }

    /// Get all session IDs in the Cinderella event.
    pub fn cindwar_all_users(&self) -> Vec<SessionId> {
        self.cindwar_event_users.iter().map(|e| *e.key()).collect()
    }

    /// Get the count of users in the Cinderella event.
    pub fn cindwar_event_user_count(&self) -> usize {
        self.cindwar_event_users.len()
    }

    /// Get the cindwar setting for a specific setting_id.
    pub fn get_cindwar_setting(
        &self,
        setting_id: u8,
    ) -> Option<ko_db::models::cinderella::CindwarSettingRow> {
        self.cindwar_settings
            .read()
            .iter()
            .find(|s| s.setting_id == setting_id as i16)
            .cloned()
    }

    /// Get cindwar items for a specific tier and class.
    pub fn get_cindwar_items_for_class(
        &self,
        tier: i16,
        class: i16,
    ) -> Vec<ko_db::models::cinderella::CindwarItemRow> {
        self.cindwar_items
            .read()
            .iter()
            .filter(|item| item.tier == tier && item.class == class)
            .cloned()
            .collect()
    }

    /// Get cindwar stat preset for a setting_id and class.
    pub fn get_cindwar_stat_preset(
        &self,
        setting_id: i16,
        class: i16,
    ) -> Option<ko_db::models::cinderella::CindwarStatRow> {
        self.cindwar_stats
            .read()
            .iter()
            .find(|s| s.setting_id == setting_id && s.class == class)
            .cloned()
    }

    /// Get cindwar reward for a rank.
    pub fn get_cindwar_reward(
        &self,
        rank: i16,
    ) -> Option<ko_db::models::cinderella::CindwarRewardRow> {
        self.cindwar_rewards
            .read()
            .iter()
            .find(|r| r.rank_id == rank)
            .cloned()
    }

    /// Check whether a player can enter the Delos zone during active CSW.
    ///
    /// Requires: in a real clan (not auto-clan), clan grade <= 3, and loyalty > 0.
    pub fn can_enter_delos(&self, clan_id: u16, loyalty: u32) -> bool {
        if clan_id == 0 || loyalty == 0 {
            return false;
        }
        if let Some(clan) = self.get_knights(clan_id) {
            clan.grade <= 3
        } else {
            false
        }
    }
    /// Get the top 10 ranked clans for a nation, sorted by points descending.
    ///
    /// to find top 10 clan leaders who become senators.
    pub fn get_top_ranked_clans(&self, nation: u8, limit: usize) -> Vec<(u16, String)> {
        let mut clans: Vec<(u16, String, u32)> = Vec::new();
        for entry in self.knights.iter() {
            let k = entry.value();
            if k.nation == nation && !k.chief.is_empty() && k.id != 0 {
                clans.push((k.id, k.chief.clone(), k.points));
            }
        }
        // Sort by points descending
        clans.sort_by(|a, b| b.2.cmp(&a.2));
        clans
            .into_iter()
            .take(limit)
            .map(|(id, chief, _)| (id, chief))
            .collect()
    }
    // ── Ranking System Methods ──────────────────────────────────────────

    /// Check if a ranking update/reset is in progress.
    pub fn is_ranking_update_in_progress(&self) -> bool {
        self.ranking_update_in_progress.load(Ordering::Relaxed)
    }
    /// Clear all BDW rankings.
    pub fn clear_bdw_rankings(&self) {
        self.bdw_rankings[0].clear();
        self.bdw_rankings[1].clear();
    }
    /// Clear all Chaos Expansion rankings.
    pub fn clear_chaos_rankings(&self) {
        self.chaos_rankings.clear();
    }
    /// Clear all Zindan War rankings.
    pub fn clear_zindan_rankings(&self) {
        self.zindan_rankings[0].clear();
        self.zindan_rankings[1].clear();
    }
    /// Get the loyalty symbol rank for a player.
    ///
    pub fn get_loyalty_symbol_rank(&self, sid: SessionId) -> i8 {
        self.with_session(sid, |h| {
            let pr = h.personal_rank;
            let kr = h.knights_rank;
            if (pr > 100 && pr <= 200) || (kr > 100 && kr <= 200) {
                return -1;
            }
            if kr == 0 && pr == 0 {
                return -1;
            }
            if kr == 0 {
                return pr as i8;
            }
            if pr == 0 {
                return kr as i8;
            }
            if kr <= pr {
                kr as i8
            } else {
                pr as i8
            }
        })
        .unwrap_or(-1)
    }
    // ── Party BBS (Seeking Party) Methods ────────────────────────────

    /// Register or update a seeking-party entry for a user.
    ///
    /// If an entry with the same `sid` already exists, updates the note.
    /// Otherwise creates a new entry.
    ///
    pub fn register_seeking_party(&self, entry: SeekingPartyUser) {
        let mut list = self.seeking_party.write();
        if let Some(existing) = list.iter_mut().find(|e| e.sid == entry.sid) {
            existing.seeking_note = entry.seeking_note;
            existing.class = entry.class;
            existing.level = entry.level;
            existing.zone = entry.zone;
            existing.is_party_leader = entry.is_party_leader;
            existing.party_id = entry.party_id;
        } else {
            list.push(entry);
        }
    }
    /// Remove a seeking-party entry by session ID.
    ///
    pub fn remove_seeking_party(&self, sid: SessionId) {
        let mut list = self.seeking_party.write();
        list.retain(|e| e.sid != sid);
    }
    /// Get a snapshot of the seeking-party list for iteration.
    ///
    pub fn get_seeking_party_list(&self) -> Vec<SeekingPartyUser> {
        self.seeking_party.read().clone()
    }
    /// Get the number of party members for a given party ID.
    ///
    pub fn get_party_member_count(&self, party_id: u16) -> u8 {
        self.parties
            .get(&party_id)
            .map(|p| p.member_count() as u8)
            .unwrap_or(0)
    }
    /// Get the WantedMessage from a party (stored on the seeking entry).
    /// Returns None if the party is not found in the seeking list.
    pub fn get_party_wanted_info(&self, party_id: u16) -> Option<(u16, String)> {
        let list = self.seeking_party.read();
        list.iter()
            .find(|e| e.party_id == party_id && e.is_party_leader == 1)
            .map(|e| (e.class, e.seeking_note.clone()))
    }

    /// Broadcast a merchant wind notice to all players in a zone.
    ///
    ///
    /// Sends a scrolling merchant notice to all players in the zone.
    ///
    /// Original C++ used WIZ_ADD_MSG (0xDB), but that opcode is outside the
    /// v2525 GameMain dispatch range (0x06-0xD7) and is silently dropped.
    /// Uses WIZ_CHAT with WAR_SYSTEM_CHAT channel as a v2525-compatible
    /// alternative for scrolling zone-wide announcements.
    pub fn send_merchant_wind_notice(
        &self,
        zone_id: u16,
        name: &str,
        message: &str,
        x: u16,
        z: u16,
    ) {
        use ko_protocol::{Opcode, Packet};

        let txt = format!("{} : {}(Location:{},{})", name, message, x, z);

        // Use WIZ_CHAT (0x10) with WAR_SYSTEM_CHAT type (7) as v2525-compatible
        // scrolling notification. WAR_SYSTEM_CHAT displays as a system-style
        // announcement that all players in the zone can see.
        let mut pkt = Packet::new(Opcode::WizChat as u8);
        pkt.write_u8(7); // ChatType::WarSystem (WAR_SYSTEM_CHAT)
        pkt.write_u8(0); // nation (0 = all)
                         // Write formatted text as Latin-1 encoded string
        pkt.write_string(&txt);

        self.broadcast_to_zone(zone_id, Arc::new(pkt), None);
    }

    // ── GM Online List ────────────────────────────────────────────────

    /// Add a GM to the online list (called on gamestart phase 2).
    ///
    pub fn gm_list_add(&self, name: &str) {
        let mut list = self.gm_list.write();
        if !list.iter().any(|n| n == name) {
            list.push(name.to_string());
        }
    }

    /// Remove a GM from the online list (called on logout/disconnect).
    ///
    pub fn gm_list_remove(&self, name: &str) {
        let mut list = self.gm_list.write();
        list.retain(|n| n != name);
    }

    /// Build a WIZ_NOTICE(5) packet containing the current GM online list.
    ///
    ///
    /// Wire: `[u8 sub=5] [u8 count] [for each: u16le len + string bytes]`
    ///
    /// C++ uses `DByte()` mode which means strings use u16 length prefix.
    pub fn build_gm_list_packet(&self) -> Packet {
        let list = self.gm_list.read();
        let mut pkt = Packet::new(Opcode::WizNotice as u8);
        pkt.write_u8(5); // sub-opcode for GM list
        let count_pos = pkt.wpos();
        pkt.write_u8(0); // count placeholder
        for name in list.iter() {
            pkt.write_string(name); // u16-prefix string (DByte mode)
        }
        pkt.put_u8_at(count_pos, list.len() as u8);
        pkt
    }

    // ── Knights Clan Buff Update ──────────────────────────────────────

    /// Update clan online member count and broadcast bonus info to all clan members.
    ///
    ///
    ///
    /// - `sign=true` (login): increment online member count
    /// - `sign=false` (logout): decrement online member count
    ///
    /// Calculates NP bonus (`online_np_count`) and EXP bonus (`online_exp_count`)
    /// based on online member count, then broadcasts `WIZ_KNIGHTS_PROCESS(KNIGHTS_CLAN_BONUS)`
    /// to all online clan members.
    pub fn knights_clan_buff_update(&self, clan_id: u16, sign: bool, session_id: SessionId) {
        if clan_id == 0 {
            return;
        }

        let mut entry = match self.knights.get_mut(&clan_id) {
            Some(e) => e,
            None => {
                // Clan not found — send empty response
                let mut pkt = Packet::new(Opcode::WizKnightsProcess as u8);
                pkt.write_u8(KNIGHTS_CLAN_BONUS);
                pkt.write_u16(0);
                self.send_to_session_owned(session_id, pkt);
                return;
            }
        };

        let info = entry.value_mut();

        // Update online member count
        if sign {
            info.online_members = info.online_members.saturating_add(1);
        } else {
            info.online_members = info.online_members.saturating_sub(1);
        }

        // Cap at MAX_CLAN_USERS
        use crate::clan_constants::MAX_CLAN_USERS;
        if info.online_members > MAX_CLAN_USERS {
            info.online_members = MAX_CLAN_USERS;
        }

        // Calculate NP bonus: ceil(online_members * 10 / 100), max 5
        if info.online_members >= 5 {
            info.online_np_count = ((info.online_members as f64 * 10.0) / 100.0).ceil() as u16;
        } else {
            info.online_np_count = 0;
        }

        // Calculate EXP bonus: 15 + online_members, max 65
        if info.online_members >= 5 {
            info.online_exp_count = 15 + info.online_members;
        } else {
            info.online_exp_count = 0;
        }

        if info.online_exp_count > 65 {
            info.online_exp_count = 65;
        }
        if info.online_np_count > 5 {
            info.online_np_count = 5;
        }

        let online_members = info.online_members;
        drop(entry); // Release DashMap lock before broadcasting

        // Broadcast to all clan members
        let mut pkt = Packet::new(Opcode::WizKnightsProcess as u8);
        pkt.write_u8(KNIGHTS_CLAN_BONUS);
        pkt.write_u16(online_members);
        self.send_to_knights_members(clan_id, Arc::new(pkt), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn make_world() -> WorldState {
        WorldState::new()
    }

    fn register_session(world: &WorldState, sid: SessionId) {
        let (tx, _rx) = mpsc::unbounded_channel::<Arc<Packet>>();
        world.register_session(sid, tx);
    }

    fn make_test_clan(id: u16, nation: u8, name: &str, points: u32) -> KnightsInfo {
        KnightsInfo {
            id,
            flag: 0,
            nation,
            grade: 2,
            ranking: 0,
            name: name.to_string(),
            chief: String::new(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0xFFFF,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: -1,
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
        }
    }

    // ── Knights CRUD ────────────────────────────────────────────────

    #[test]
    fn test_knights_insert_and_get() {
        let world = make_world();
        let clan = make_test_clan(100, 1, "TestClan", 500);
        world.insert_knights(clan);

        let got = world.get_knights(100).unwrap();
        assert_eq!(got.id, 100);
        assert_eq!(got.name, "TestClan");
        assert_eq!(got.points, 500);
    }

    #[test]
    fn test_knights_remove() {
        let world = make_world();
        world.insert_knights(make_test_clan(100, 1, "TestClan", 0));
        let removed = world.remove_knights(100);
        assert!(removed.is_some());
        assert!(world.get_knights(100).is_none());
    }

    #[test]
    fn test_knights_get_nonexistent() {
        let world = make_world();
        assert!(world.get_knights(999).is_none());
    }

    #[test]
    fn test_knights_find_by_name() {
        let world = make_world();
        world.insert_knights(make_test_clan(100, 1, "Alpha", 0));
        world.insert_knights(make_test_clan(200, 2, "Beta", 0));

        let found = world.find_knights_by_name("Beta").unwrap();
        assert_eq!(found.id, 200);
        assert!(world.find_knights_by_name("Gamma").is_none());
    }

    #[test]
    fn test_knights_name_exists_case_insensitive() {
        let world = make_world();
        world.insert_knights(make_test_clan(100, 1, "TestClan", 0));
        assert!(world.knights_name_exists("TESTCLAN"));
        assert!(world.knights_name_exists("testclan"));
        assert!(!world.knights_name_exists("OtherClan"));
    }

    #[test]
    fn test_knights_update() {
        let world = make_world();
        world.insert_knights(make_test_clan(100, 1, "TestClan", 100));
        world.update_knights(100, |k| k.points = 999);
        assert_eq!(world.get_knights(100).unwrap().points, 999);
    }

    #[test]
    fn test_get_all_knights() {
        let world = make_world();
        world.insert_knights(make_test_clan(100, 1, "A", 0));
        world.insert_knights(make_test_clan(200, 2, "B", 0));

        let all = world.get_all_knights();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_top_knights_by_nation() {
        let world = make_world();
        world.insert_knights(make_test_clan(1, 1, "Low", 100));
        world.insert_knights(make_test_clan(2, 1, "Mid", 500));
        world.insert_knights(make_test_clan(3, 1, "High", 1000));
        world.insert_knights(make_test_clan(4, 2, "Other", 2000));

        let top = world.get_top_knights_by_nation(1, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, 3); // High (1000) first
        assert_eq!(top[1].0, 2); // Mid (500) second
    }

    // ── Alliance ────────────────────────────────────────────────────

    #[test]
    fn test_alliance_crud() {
        let world = make_world();
        let alliance = KnightsAlliance {
            main_clan: 100,
            sub_clan: 200,
            mercenary_1: 300,
            mercenary_2: 0,
            notice: String::new(),
        };
        world.insert_alliance(alliance);

        let got = world.get_alliance(100).unwrap();
        assert_eq!(got.main_clan, 100);
        assert_eq!(got.sub_clan, 200);

        world.update_alliance(100, |a| a.mercenary_2 = 400);
        assert_eq!(world.get_alliance(100).unwrap().mercenary_2, 400);

        world.remove_alliance(100);
        assert!(world.get_alliance(100).is_none());
    }

    // ── Party System ────────────────────────────────────────────────

    #[test]
    fn test_create_party() {
        let world = make_world();
        register_session(&world, 100);

        let party_id = world.create_party(100).unwrap();
        assert!(party_id > 0);

        let party = world.get_party(party_id).unwrap();
        assert!(party.is_leader(100));
    }

    #[test]
    fn test_add_and_remove_party_member() {
        let world = make_world();
        register_session(&world, 100);
        register_session(&world, 200);

        let pid = world.create_party(100).unwrap();
        assert!(world.add_party_member(pid, 200));

        let party = world.get_party(pid).unwrap();
        assert_eq!(party.active_members().len(), 2);

        assert!(world.remove_party_member(pid, 200));
        let party = world.get_party(pid).unwrap();
        assert_eq!(party.active_members().len(), 1);
    }

    #[test]
    fn test_disband_party() {
        let world = make_world();
        register_session(&world, 100);
        register_session(&world, 200);

        let pid = world.create_party(100).unwrap();
        world.add_party_member(pid, 200);

        let members = world.disband_party(pid);
        assert_eq!(members.len(), 2);
        assert!(world.get_party(pid).is_none());
    }

    #[test]
    fn test_party_invitation() {
        let world = make_world();
        world.set_party_invitation(200, 1, 100);
        assert!(world.has_party_invitation(200));

        let (pid, inviter) = world.take_party_invitation(200).unwrap();
        assert_eq!(pid, 1);
        assert_eq!(inviter, 100);
        assert!(!world.has_party_invitation(200));
    }

    // ── Chat Room ───────────────────────────────────────────────────

    #[test]
    fn test_chat_room_index() {
        let world = make_world();
        register_session(&world, 100);
        assert_eq!(world.get_chat_room_index(100), 0);

        world.set_chat_room_index(100, 42);
        assert_eq!(world.get_chat_room_index(100), 42);
    }

    #[test]
    fn test_create_chat_room() {
        let world = make_world();
        let idx = world.create_chat_room(
            "General".to_string(),
            "Admin".to_string(),
            String::new(),
            1,
            50,
        );
        assert!(idx.is_some());

        // Duplicate name should fail
        let dup = world.create_chat_room(
            "general".to_string(), // case-insensitive
            "Admin".to_string(),
            String::new(),
            1,
            50,
        );
        assert!(dup.is_none());
    }

    // ── Sprint 961: Additional coverage ──────────────────────────────

    /// KNIGHTS_CLAN_BONUS constant matches C++ packets.h.
    #[test]
    fn test_knights_clan_bonus_constant() {
        assert_eq!(KNIGHTS_CLAN_BONUS, 98);
    }

    /// get_party returns None for non-existent party.
    #[test]
    fn test_get_party_nonexistent() {
        let world = make_world();
        assert!(world.get_party(9999).is_none());
    }

    /// is_in_party returns false when not in a party.
    #[test]
    fn test_is_in_party_false() {
        let world = make_world();
        register_session(&world, 1);
        assert!(!world.is_in_party(1));
    }

    /// promote_party_leader changes the leader.
    #[test]
    fn test_promote_party_leader() {
        let world = make_world();
        register_session(&world, 1);
        register_session(&world, 2);
        let pid = world.create_party(1).unwrap();
        world.add_party_member(pid, 2);
        assert!(world.promote_party_leader(pid, 2));
        let party = world.get_party(pid).unwrap();
        assert_eq!(party.leader_sid(), Some(2));
    }

    /// list_chat_rooms returns empty on fresh world.
    #[test]
    fn test_list_chat_rooms_empty() {
        let world = make_world();
        assert!(world.list_chat_rooms().is_empty());
    }

    // ── Sprint 975: Additional coverage ──────────────────────────────

    /// has_party_invitation returns false when no invitation set.
    #[test]
    fn test_has_party_invitation_default_false() {
        let world = make_world();
        register_session(&world, 1);
        assert!(!world.has_party_invitation(1));
    }

    /// get_all_knights_ids returns empty on fresh world.
    #[test]
    fn test_get_all_knights_ids_empty() {
        let world = make_world();
        assert!(world.get_all_knights_ids().is_empty());
    }

    /// get_party_id returns None when not in a party.
    #[test]
    fn test_get_party_id_none() {
        let world = make_world();
        register_session(&world, 1);
        assert!(world.get_party_id(1).is_none());
    }

    /// get_chat_room_index defaults to 0 and can be set.
    #[test]
    fn test_chat_room_index_default_and_set() {
        let world = make_world();
        register_session(&world, 1);
        assert_eq!(world.get_chat_room_index(1), 0);
        world.set_chat_room_index(1, 42);
        assert_eq!(world.get_chat_room_index(1), 42);
    }

    /// get_online_knights_session_ids returns empty for unknown clan.
    #[test]
    fn test_online_knights_session_ids_empty() {
        let world = make_world();
        assert!(world.get_online_knights_session_ids(9999).is_empty());
    }
}
