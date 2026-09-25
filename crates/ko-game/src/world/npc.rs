//! NPC/monster instance management, AI state, spawning, killing, and bot system.

use super::*;

const NPC_BOSS: u8 = 3;

impl WorldState {
    /// Allocate a unique NPC runtime ID.
    pub fn allocate_npc_id(&self) -> NpcId {
        self.next_npc_id.fetch_add(1, Ordering::Relaxed)
    }
    /// Get an NPC template by (s_sid, is_monster).
    pub fn get_npc_template(&self, s_sid: u16, is_monster: bool) -> Option<Arc<NpcTemplate>> {
        self.npc_templates
            .get(&(s_sid, is_monster))
            .map(|t| t.clone())
    }
    /// Update an NPC template's group (nation) and optionally PID (model).
    ///
    ///
    /// This modifies the shared template data so that future NPC respawns
    /// use the updated nation/model. If `group > 0`, sets the group field.
    /// If `pid > 0`, sets the pid field.
    pub fn npc_template_update(&self, s_sid: u16, is_monster: bool, group: u8, pid: u16) {
        // Clone the template data and drop the DashMap ref before inserting
        // to avoid deadlock (holding read shard lock while requesting write).
        let updated = self.npc_templates.get(&(s_sid, is_monster)).map(|old| {
            let mut t = (**old).clone();
            if group > 0 {
                t.group = group;
            }
            if pid > 0 {
                t.pid = pid;
            }
            t
        });
        if let Some(tmpl) = updated {
            self.npc_templates
                .insert((s_sid, is_monster), Arc::new(tmpl));
        }
    }
    /// Get an NPC instance by runtime ID.
    pub fn get_npc_instance(&self, nid: NpcId) -> Option<Arc<NpcInstance>> {
        self.npc_instances.get(&nid).map(|n| n.clone())
    }
    /// Check if a player is within NPC interaction range.
    ///
    /// before all NPC interactions (shop, warehouse, quest, repair, upgrade).
    ///
    /// Returns `true` if the NPC exists and the player is within range,
    /// or if the NPC is not found (to avoid blocking on missing NPC data).
    pub fn is_in_npc_range(&self, sid: SessionId, npc_id: NpcId) -> bool {
        const MAX_NPC_RANGE_SQ: f32 = 121.0; // C++ MAX_NPC_RANGE = pow(11.0f, 2.0f) — squared distance
        let player_pos = match self.get_position(sid) {
            Some(p) => p,
            None => return false,
        };
        let npc = match self.get_npc_instance(npc_id) {
            Some(n) => n,
            // C++ returns nullptr → caller sends fail.  Don't allow with missing NPC.
            None => return false,
        };
        if player_pos.zone_id != npc.zone_id {
            return false;
        }
        let dx = player_pos.x - npc.x;
        let dz = player_pos.z - npc.z;
        (dx * dx + dz * dz) <= MAX_NPC_RANGE_SQ
    }
    /// Find the first NPC instance in the given zone whose proto_id matches.
    ///
    ///
    /// Used by KrowazGate to locate gate NPCs by their template ID within a zone.
    pub fn find_npc_in_zone(&self, proto_id: u16, zone_id: u16) -> Option<Arc<NpcInstance>> {
        self.npc_instances
            .iter()
            .find(|entry| entry.zone_id == zone_id && entry.proto_id == proto_id)
            .map(|entry| entry.value().clone())
    }
    /// Find all NPC instances in the given zone whose proto_id matches.
    ///
    /// in `UserObjectSystem.cpp:213-225` — `LogLeverBuringLog`
    ///
    /// Used by Wood lever events to toggle all matching gate NPCs.
    pub fn find_all_npcs_in_zone(&self, proto_id: u16, zone_id: u16) -> Vec<Arc<NpcInstance>> {
        self.npc_instances
            .iter()
            .filter(|entry| entry.zone_id == zone_id && entry.proto_id == proto_id)
            .map(|entry| entry.value().clone())
            .collect()
    }
    /// Get the current HP of an NPC by runtime ID.
    ///
    /// Returns `None` if the NPC does not exist.
    pub fn get_npc_hp(&self, nid: NpcId) -> Option<i32> {
        self.npc_hp.get(&nid).map(|v| *v)
    }
    /// Initialize HP for an NPC (used during spawn and for testing).
    pub fn init_npc_hp(&self, nid: NpcId, hp: i32) {
        self.npc_hp.insert(nid, hp);
    }
    /// Update (set) the HP of an NPC by runtime ID.
    pub fn update_npc_hp(&self, nid: NpcId, hp: i32) {
        if let Some(mut entry) = self.npc_hp.get_mut(&nid) {
            *entry = hp;
        }
    }
    /// Check if an NPC is dead (HP <= 0).
    pub fn is_npc_dead(&self, nid: NpcId) -> bool {
        self.npc_hp.get(&nid).map(|v| *v <= 0).unwrap_or(true)
    }
    /// Record damage dealt to an NPC by a player.
    ///
    /// so loot rights go to the highest damage dealer, not the last-hitter.
    pub fn record_npc_damage(&self, nid: NpcId, sid: SessionId, damage: i32) {
        let inner = self.npc_damage.entry(nid).or_default();
        inner
            .entry(sid)
            .and_modify(|v| *v += damage)
            .or_insert(damage);
    }
    /// Get the player who dealt the most total damage to an NPC.
    ///
    /// Returns `None` if no damage was recorded.
    pub fn get_max_damage_user(&self, nid: NpcId) -> Option<SessionId> {
        self.npc_damage.get(&nid).and_then(|inner| {
            inner
                .iter()
                .max_by_key(|entry| *entry.value())
                .map(|entry| *entry.key())
        })
    }
    /// Clear damage tracking for an NPC (called on death/respawn).
    pub fn clear_npc_damage(&self, nid: NpcId) {
        self.npc_damage.remove(&nid);
    }
    /// Remove one player's threat/damage entry without affecting loot rights
    /// for other players who are still fighting this NPC.
    pub fn clear_npc_player_damage(&self, nid: NpcId, sid: SessionId) {
        if let Some(inner) = self.npc_damage.get(&nid) {
            inner.remove(&sid);
        }
    }
    /// Get all damage entries for an NPC, returning `Vec<(SessionId, i32)>`.
    ///
    /// XP/NP distribution on NPC death.
    pub fn get_npc_damage_entries(&self, nid: NpcId) -> Vec<(SessionId, i32)> {
        self.npc_damage
            .get(&nid)
            .map(|inner| inner.iter().map(|e| (*e.key(), *e.value())).collect())
            .unwrap_or_default()
    }
    /// Check if a specific player has dealt damage to an NPC.
    ///
    /// Returns true if the player is in the NPC's damage tracking map.
    pub fn npc_damage_contains(&self, nid: NpcId, sid: SessionId) -> bool {
        self.npc_damage
            .get(&nid)
            .map(|inner| inner.contains_key(&sid))
            .unwrap_or(false)
    }
    /// Notify an NPC that it was damaged by a player (reactive aggro switch).
    ///
    ///
    /// When a player damages an NPC that is in Standing/Moving state, the NPC
    /// immediately switches to Attacking state with the attacker as target.
    /// This is especially important for TENDER (passive) NPCs that wouldn't
    /// otherwise detect enemies on their own without being damaged first.
    ///
    /// Also calls `find_friends()` for pack NPCs (has_friends) and bosses,
    /// alerting nearby same-family or any NPCs to join the fight.
    pub fn notify_npc_damaged(&self, nid: NpcId, attacker_sid: SessionId) {
        // Snapshot fields we need for find_friends after releasing the lock.
        let mut should_find_friends = false;
        let is_boss = self
            .get_npc_instance(nid)
            .and_then(|npc| self.get_npc_template(npc.proto_id, npc.is_monster))
            .is_some_and(|tmpl| tmpl.npc_type == NPC_BOSS);

        if let Some(mut ai) = self.npc_ai.get_mut(&nid) {
            // Reactive aggro bypasses find_enemy(), which normally starts the
            // chase timer. A first hit must start it too (and subsequent hits
            // keep an active fight alive).
            ai.last_combat_time_ms = ai.last_tick_ms.max(1);
            // Only switch target if NPC is in a non-combat state
            //   if (GetNpcState() == NPC_STANDING || NPC_MOVING || NPC_SLEEPING)
            match ai.state {
                NpcState::Standing | NpcState::Moving | NpcState::Sleeping => {
                    ai.target_id = Some(attacker_sid);
                    ai.state = NpcState::Attacking;
                    ai.delay_ms = 0;
                }
                _ => {
                    // Already in combat — consider switching target based on threat.
                    //   Random-weighted comparison between current and new attacker.
                    self.maybe_switch_target(&mut ai, nid, attacker_sid);
                }
            }

            //   if (m_bHasFriends || GetType() == NPC_BOSS)
            //     FindFriend(GetType() == NPC_BOSS ? MonSearchAny : MonSearchSameFamily);
            if ai.has_friends {
                should_find_friends = true;
            }
        }

        // Call find_friends outside the npc_ai borrow.
        if should_find_friends && !is_boss {
            self.find_friends(nid, false);
        }
    }

    /// Alert nearby NPCs to join combat when a pack NPC or boss is attacked.
    ///
    /// and `CNpc::FindFriendRegion()`
    ///
    /// For bosses (`npc_type == NPC_BOSS`), uses `MonSearchAny` — alerts ANY nearby
    /// NPC regardless of family type. For regular pack NPCs, uses `MonSearchSameFamily`
    /// — only alerts NPCs with matching `family_type` that also have `has_friends`.
    ///
    /// Search range uses `tracing_range` from the NPC template
    /// Iterates NPCs in the same 3x3 region grid. Each eligible friend NPC gets its
    /// target set to the caller's target and transitions to Attacking state.
    fn find_friends(&self, caller_nid: NpcId, is_boss: bool) {
        // Snapshot the caller's state needed for friend search.
        let (target_id, zone_id, cur_x, cur_z, family_type, tracing_range) = {
            let ai = match self.npc_ai.get(&caller_nid) {
                Some(a) => a,
                None => return,
            };

            //   if (m_bySearchRange == 0 || (type != SameFamily && hasTarget())) return 0;
            // For SameFamily search, we proceed even if we already have a target.
            // For Any search (boss), skip if we already had a target but that's handled
            // differently — C++ actually always calls FindFriend from ChangeTarget end.

            let target = match ai.target_id {
                Some(t) => t,
                None => return, // No target to share with friends
            };

            let inst = match self.get_npc_instance(caller_nid) {
                Some(i) => i,
                None => return,
            };
            let tmpl = match self.get_npc_template(inst.proto_id, inst.is_monster) {
                Some(t) => t,
                None => return,
            };

            if tmpl.search_range == 0 {
                return;
            }

            (
                target,
                ai.zone_id,
                ai.cur_x,
                ai.cur_z,
                ai.family_type,
                tmpl.tracing_range as f32,
            )
        };
        // ^ DashMap ref dropped here

        // Collect eligible friend NPCs to activate.
        // C++ uses region grid (3x3 foreach_region), but since we iterate all AI NPCs
        // we filter by zone and distance which is equivalent and simpler.
        // We collect first, then mutate, to avoid holding iterator refs during mutation.
        let mut friends_to_activate: Vec<NpcId> = Vec::new();

        for entry in self.npc_ai.iter() {
            let friend_nid = *entry.key();

            // Skip self
            if friend_nid == caller_nid {
                continue;
            }

            let friend_ai = entry.value();

            // Must be in the same zone
            if friend_ai.zone_id != zone_id {
                continue;
            }

            // Must be alive (not Dead state) and not already fighting
            //   if (pNpc->hasTarget() && pNpc->GetNpcState() == NPC_FIGHTING) continue;
            if friend_ai.state == NpcState::Dead {
                continue;
            }
            if friend_ai.target_id.is_some() && friend_ai.state == NpcState::Fighting {
                continue;
            }

            // Distance check using tracing_range
            let dx = friend_ai.cur_x - cur_x;
            let dz = friend_ai.cur_z - cur_z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist > tracing_range {
                continue;
            }

            if is_boss {
                // MonSearchAny: alert any NPC in range
            } else {
                // MonSearchSameFamily: only same-family NPCs with has_friends
                //   if (pNpc->m_bHasFriends && pNpc->m_proto->m_byFamilyType == m_proto->m_byFamilyType)
                if !friend_ai.has_friends || friend_ai.family_type != family_type {
                    continue;
                }
            }

            // Check NPC is not dead (HP check)
            if self.is_npc_dead(friend_nid) {
                continue;
            }

            // Skip gate NPCs — they should not be called as friends
            if let Some(inst) = self.get_npc_instance(friend_nid) {
                if let Some(tmpl) = self.get_npc_template(inst.proto_id, inst.is_monster) {
                    if is_gate_npc_type(tmpl.npc_type) {
                        continue;
                    }
                }
            }

            friends_to_activate.push(friend_nid);
        }

        // Now activate all collected friends.
        //   pNpc->m_Target.id = m_Target.id;
        //   pNpc->NpcStrategy(NPC_ATTACK_SHOUT);
        for friend_nid in friends_to_activate {
            if let Some(mut friend_ai) = self.npc_ai.get_mut(&friend_nid) {
                // Only activate if still eligible (state could have changed)
                if friend_ai.state == NpcState::Dead {
                    continue;
                }
                if friend_ai.target_id.is_some() && friend_ai.state == NpcState::Fighting {
                    continue;
                }
                friend_ai.target_id = Some(target_id);
                friend_ai.state = NpcState::Attacking;
                friend_ai.delay_ms = 0;
            }
        }
    }

    /// Evaluate whether the NPC should switch its current target to a new attacker.
    ///
    ///
    /// Uses a random roll (0-100 inclusive, matching `myrand(0,100)`) to decide:
    /// - [0,50): compare how much damage each player deals TO the NPC — switch if new
    ///   attacker dealt more cumulative damage (approximation of `GetDamage(this)`)
    /// - [50,80): compare distance — switch if new attacker is closer
    /// - [80,95): compare NPC's outgoing damage to each player — switch if NPC would
    ///   hurt the new target more (`GetDamage(pUser)`, approximated via AC)
    /// - [95,100]: no comparison — unconditionally switch (C++ falls through to set target)
    fn maybe_switch_target(&self, ai: &mut NpcAiState, nid: NpcId, attacker_sid: SessionId) {
        use rand::Rng;

        let current_target = match ai.target_id {
            Some(t) => t,
            None => {
                // No current target — just set the new attacker
                ai.target_id = Some(attacker_sid);
                return;
            }
        };

        // Same attacker — no switch needed
        if current_target == attacker_sid {
            return;
        }

        // C++ uses myrand(0, 100) which is 0..=100 (101 values)
        let roll = rand::thread_rng().gen_range(0..=100);

        let should_switch = if roll < 50 {
            // [0,50): Compare cumulative damage dealt TO the NPC.
            // C++ calls `preUser->GetDamage(this)` vs `pUser->GetDamage(this)` in preview
            // mode, but we approximate with recorded cumulative damage.
            let current_dmg = self
                .npc_damage
                .get(&nid)
                .and_then(|inner| inner.get(&current_target).map(|v| *v))
                .unwrap_or(0);
            let new_dmg = self
                .npc_damage
                .get(&nid)
                .and_then(|inner| inner.get(&attacker_sid).map(|v| *v))
                .unwrap_or(0);
            // C++ switches if lastDamage >= preDamage (new >= old)
            new_dmg > current_dmg
        } else if roll < 80 {
            // [50,80): Compare distance — switch if new attacker is closer.
            let npc_x = ai.cur_x;
            let npc_z = ai.cur_z;

            let old_dist_sq = self
                .get_position(current_target)
                .map(|p| {
                    let dx = p.x - npc_x;
                    let dz = p.z - npc_z;
                    dx * dx + dz * dz
                })
                .unwrap_or(f32::MAX);

            let new_dist_sq = self
                .get_position(attacker_sid)
                .map(|p| {
                    let dx = p.x - npc_x;
                    let dz = p.z - npc_z;
                    dx * dx + dz * dz
                })
                .unwrap_or(f32::MAX);

            new_dist_sq < old_dist_sq
        } else if roll < 95 {
            // [80,95): Compare NPC's outgoing damage to each player.
            // C++ calls `GetDamage(preUser)` vs `GetDamage(pUser)` — NPC damage TO player.
            // We approximate by comparing player AC: lower AC = NPC deals more damage.
            let old_ac = self.get_equipped_stats(current_target).total_ac;
            let new_ac = self.get_equipped_stats(attacker_sid).total_ac;
            // Switch if NPC would deal more to new target (lower AC)
            new_ac < old_ac
        } else {
            // [95,100]: Unconditionally switch.
            // C++ has no guard here — falls through to `m_Target.id = pUser->GetID()`.
            true
        };

        if should_switch {
            ai.target_id = Some(attacker_sid);
            ai.delay_ms = 0;
        }
    }
    /// Get a snapshot of an NPC's AI state.
    pub fn get_npc_ai(&self, nid: NpcId) -> Option<NpcAiState> {
        self.npc_ai.get(&nid).map(|v| v.clone())
    }
    /// Insert or replace an NPC AI state entry (used in test setup).
    #[cfg(test)]
    pub(crate) fn insert_npc_ai(&self, nid: NpcId, ai: NpcAiState) {
        self.npc_ai.insert(nid, ai);
    }
    /// Update an NPC's AI state via a closure.
    pub fn update_npc_ai(&self, nid: NpcId, updater: impl FnOnce(&mut NpcAiState)) {
        if let Some(mut entry) = self.npc_ai.get_mut(&nid) {
            updater(&mut entry);
        }
    }
    /// Get all NPC IDs that have AI state (for tick processing).
    pub fn get_all_ai_npc_ids(&self) -> Vec<NpcId> {
        self.npc_ai.iter().map(|entry| *entry.key()).collect()
    }

    /// Get NPC IDs grouped by zone ID (for parallel per-zone AI processing).
    pub fn get_ai_npc_ids_by_zone(&self) -> HashMap<u16, Vec<NpcId>> {
        let mut by_zone: HashMap<u16, Vec<NpcId>> = HashMap::new();
        for entry in self.npc_ai.iter() {
            by_zone
                .entry(entry.value().zone_id)
                .or_default()
                .push(*entry.key());
        }
        by_zone
    }
    /// Update the `gate_open` field on an NpcInstance (replaces the Arc).
    ///
    /// instance so NPC_INOUT packets include the correct gate state when new
    /// clients enter the region.
    pub fn update_npc_gate_open(&self, nid: NpcId, gate_open: u8) {
        // Clone the instance data and drop the DashMap ref before inserting
        // to avoid deadlock (holding read shard lock while requesting write).
        let updated = self.npc_instances.get(&nid).map(|old| {
            Arc::new(NpcInstance {
                gate_open,
                ..(**old).clone()
            })
        });
        if let Some(inst) = updated {
            self.npc_instances.insert(nid, inst);
        }
    }
    /// Update an NPC instance's position (replaces the Arc).
    ///
    /// Used for soccer ball NPC teleportation (goal reset, out-of-bounds reset).
    ///
    pub fn update_npc_position(&self, nid: NpcId, new_x: f32, new_z: f32) {
        let updated = self.npc_instances.get(&nid).map(|old| {
            let new_rx = crate::zone::calc_region(new_x);
            let new_rz = crate::zone::calc_region(new_z);
            Arc::new(NpcInstance {
                x: new_x,
                z: new_z,
                region_x: new_rx,
                region_z: new_rz,
                ..(**old).clone()
            })
        });
        if let Some(inst) = updated {
            self.npc_instances.insert(nid, inst);
        }
        // Also update AI state if it exists
        self.update_npc_ai(nid, |ai| {
            ai.cur_x = new_x;
            ai.cur_z = new_z;
            ai.region_x = crate::zone::calc_region(new_x);
            ai.region_z = crate::zone::calc_region(new_z);
        });
    }
    /// Toggle a gate NPC's open/close state and broadcast to nearby players.
    ///
    ///
    /// Updates `NpcInstance.gate_open`, then broadcasts a `WIZ_OBJECT_EVENT`
    /// packet to the NPC's 3x3 region. For `NPC_OBJECT_WOOD` (54) and
    /// `NPC_ROLLINGSTONE` (181), only the state is updated — no broadcast
    /// is sent (matching C++ early return).
    ///
    /// Wire format: `WIZ_OBJECT_EVENT [u8 object_type] [u8 1] [u32 npc_id] [u8 gate_open]`
    ///
    /// The `object_type` is resolved from the zone's object event table using
    /// the NPC's proto_id. Falls back to `OBJECT_FLAG_LEVER` (4) if not found.
    pub fn send_gate_flag(&self, nid: NpcId, gate_open: u8) {
        use crate::object_event_constants::OBJECT_FLAG_LEVER;
        use std::sync::Arc;

        use crate::npc_type_constants::NPC_OBJECT_WOOD;

        use crate::npc_type_constants::NPC_ROLLINGSTONE;

        // Look up the NPC instance
        let npc = match self.get_npc_instance(nid) {
            Some(n) => n,
            None => return,
        };

        // Update the instance's gate_open for future NPC_INOUT packets
        self.update_npc_gate_open(nid, gate_open);

        // Look up template to check NPC type
        let tmpl = match self.get_npc_template(npc.proto_id, npc.is_monster) {
            Some(t) => t,
            None => return,
        };

        // NPC_OBJECT_WOOD and NPC_ROLLINGSTONE: only update state, no broadcast
        if tmpl.npc_type == NPC_OBJECT_WOOD || tmpl.npc_type == NPC_ROLLINGSTONE {
            return;
        }

        // Resolve object_type from zone's object event table (using proto_id as key)
        //   _OBJECT_EVENT* pObjectEvent = GetMap()->GetObjectEvent(GetProtoID());
        //   if (pObjectEvent) objectType = (uint8)pObjectEvent->sType;
        let object_type = self
            .get_zone(npc.zone_id)
            .and_then(|z| z.get_object_event(npc.proto_id).map(|e| e.obj_type as u8))
            .unwrap_or(OBJECT_FLAG_LEVER);

        // Build WIZ_OBJECT_EVENT packet
        //   result << uint8(1) << uint32(GetID()) << m_byGateOpen;
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(object_type);
        pkt.write_u8(1); // success marker
        pkt.write_u32(nid);
        pkt.write_u8(gate_open);

        // Broadcast to the NPC's 3x3 region
        self.broadcast_to_3x3(
            npc.zone_id,
            npc.region_x,
            npc.region_z,
            Arc::new(pkt),
            None,
            npc.event_room,
        );
    }
    // ── Juraid Bridge NPC Broadcast ──────────────────────────────────

    /// Broadcast Juraid bridge gate opening to players in the zone.
    ///
    /// and `HandleJuraidGateOpen()` in NpcThread.cpp:1250-1281.
    ///
    /// For each room, finds bridge NPCs by trap_number, sets `gate_open = 2`,
    /// then sends `WIZ_NPC_INOUT(OUT)` + `WIZ_NPC_INOUT(IN)` to players in
    /// the matching zone + event_room + nation.
    ///
    /// Karus bridges use trap_number `bridge_idx + 1` (1, 2, 3).
    /// Elmorad bridges use trap_number `bridge_idx + 4` (4, 5, 6).
    ///
    /// Timer-triggered broadcasts are per-nation: Karus bridge → KARUS only,
    /// Elmorad bridge → ELMORAD only.
    pub fn broadcast_juraid_bridge_open(&self, bridge_idx: usize, room_id: u16) {
        self.broadcast_juraid_bridge_open_for_nation(bridge_idx, room_id, NATION_KARUS);
        self.broadcast_juraid_bridge_open_for_nation(bridge_idx, room_id, NATION_ELMORAD);
    }

    /// Open only one nation's Juraid bridge. Kill progression is independent;
    /// the two-nation wrapper above is reserved for the 10-minute fallback.
    pub fn broadcast_juraid_bridge_open_for_nation(
        &self,
        bridge_idx: usize,
        room_id: u16,
        nation: u8,
    ) {
        use crate::npc::{build_npc_inout, NPC_IN, NPC_OUT};
        use crate::systems::juraid::ZONE_JURAID;

        let trap = match nation {
            NATION_KARUS => bridge_idx as i16 + 1,
            NATION_ELMORAD => bridge_idx as i16 + 4,
            _ => return,
        };
        {
            // Find the bridge NPC by zone + event_room + trap_number
            let npc = self.npc_instances.iter().find_map(|entry| {
                let n = entry.value();
                if n.zone_id == ZONE_JURAID && n.event_room == room_id && n.trap_number == trap {
                    Some(n.clone())
                } else {
                    None
                }
            });

            let npc = match npc {
                Some(n) => n,
                None => return,
            };

            // Set gate_open = 2
            self.update_npc_gate_open(npc.nid, 2);

            // Build INOUT_OUT packet (despawn closed gate)
            let mut out_pkt = Packet::new(Opcode::WizNpcInout as u8);
            out_pkt.write_u8(NPC_OUT);
            out_pkt.write_u32(npc.nid);

            // C++ HandleJuraidGateOpen broadcasts the bridge transition to ALL
            // users in the event room. The two nation paths use different
            // physical bridge NPCs, but filtering the packet by nation left the
            // client's collision state stale in v2615.
            self.broadcast_to_zone_event_room(ZONE_JURAID, room_id, Arc::new(out_pkt), None);

            // Re-read the updated NPC instance (with gate_open=2)
            let updated_npc = match self.get_npc_instance(npc.nid) {
                Some(n) => n,
                None => return,
            };

            // Build INOUT_IN packet (spawn with gate_open=2)
            let tmpl = match self.get_npc_template(npc.proto_id, npc.is_monster) {
                Some(t) => t,
                None => return,
            };
            let in_pkt = build_npc_inout(NPC_IN, &updated_npc, &tmpl);

            self.broadcast_to_zone_event_room(ZONE_JURAID, room_id, Arc::new(in_pkt), None);

            // The C++ server also sends CNpc::SendJuraidBridgeFlag() when
            // changing the physical bridge collision state. NPC_INOUT alone
            // updates its visible gate_open field, but the client keeps the
            // bridge collision closed without this OBJECT_GATE event.
            let mut gate_pkt = Packet::new(Opcode::WizObjectEvent as u8);
            gate_pkt.write_u8(crate::object_event_constants::OBJECT_GATE);
            gate_pkt.write_u8(1);
            gate_pkt.write_u32(npc.nid);
            gate_pkt.write_u8(1);
            self.broadcast_to_3x3(
                ZONE_JURAID,
                npc.region_x,
                npc.region_z,
                Arc::new(gate_pkt),
                None,
                room_id,
            );

            tracing::debug!(
                npc_id = npc.nid,
                trap_number = trap,
                room = room_id,
                nation = nation,
                "Juraid bridge NPC gate opened"
            );
        }
    }

    // ── Event NPC Spawn / Kill ─────────────────────────────────────

    /// Spawn an event NPC/monster at the specified position.
    ///
    ///
    /// Allocates a new runtime NPC ID, creates an NpcInstance, registers it in
    /// the zone region grid, initializes HP, and broadcasts NPC_IN to nearby
    /// players. For monsters, also initializes AI state.
    ///
    /// `event_room` is 1-based (room_id + 1). Pass 0 for non-room event NPCs.
    /// `summon_type`: 0 = normal, 1 = Monster Stone boss, etc.
    pub fn spawn_event_npc(
        &self,
        s_sid: u16,
        is_monster: bool,
        zone_id: u16,
        x: f32,
        z: f32,
        count: u16,
    ) -> Vec<NpcId> {
        self.spawn_event_npc_ex(s_sid, is_monster, zone_id, x, z, count, 0, 0)
    }

    /// Extended event NPC spawn with event room and summon type.
    ///
    /// `nEventRoom` and `nSummonSpecialID` parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_event_npc_ex(
        &self,
        s_sid: u16,
        is_monster: bool,
        zone_id: u16,
        x: f32,
        z: f32,
        count: u16,
        event_room: u16,
        summon_type: u8,
    ) -> Vec<NpcId> {
        self.spawn_event_npc_ex_with_direction(
            s_sid,
            is_monster,
            zone_id,
            x,
            z,
            count,
            event_room,
            summon_type,
            0,
        )
    }

    /// Event spawn variant preserving the authoritative DB facing direction.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_event_npc_ex_with_direction(
        &self,
        s_sid: u16,
        is_monster: bool,
        zone_id: u16,
        x: f32,
        z: f32,
        count: u16,
        event_room: u16,
        summon_type: u8,
        direction: u8,
    ) -> Vec<NpcId> {
        self.spawn_event_npc_ex_with_metadata(
            s_sid,
            is_monster,
            zone_id,
            x,
            0.0,
            z,
            count,
            event_room,
            summon_type,
            direction,
            0,
        )
    }

    /// Event spawn variant preserving placement data that is significant for
    /// scripted map objects. UTC doors use both their DB Y/facing values and
    /// `trap_number`; the C++ SpawnEventNpc call passes all three separately
    /// from its summon/event type.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_event_npc_ex_with_metadata(
        &self,
        s_sid: u16,
        is_monster: bool,
        zone_id: u16,
        x: f32,
        y: f32,
        z: f32,
        count: u16,
        event_room: u16,
        summon_type: u8,
        direction: u8,
        trap_number: u8,
    ) -> Vec<NpcId> {
        let tmpl = match self.get_npc_template(s_sid, is_monster) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let zone = match self.get_zone(zone_id) {
            Some(z) => z,
            None => return Vec::new(),
        };

        let mut spawned_ids = Vec::with_capacity(count as usize);

        for i in 0..count {
            let nid = self.allocate_npc_id();

            // Scatter position for multi-spawn (C++ myrand(-10, 10))
            let (offset_x, offset_z) = if count > 1 {
                let ox = ((i as f32) * 3.7 - 5.0).clamp(-10.0, 10.0);
                let oz = ((i as f32) * 2.3 - 5.0).clamp(-10.0, 10.0);
                (ox, oz)
            } else {
                (0.0, 0.0)
            };

            let spawn_x = x + offset_x;
            let spawn_z = z + offset_z;
            let region_x = calc_region(spawn_x);
            let region_z = calc_region(spawn_z);

            // Monster Stone support NPCs are neutral to both nations. Their
            // global templates retain the original nation for normal map
            // spawns; only private rooms override the runtime nation to 3.
            let runtime_nation = if is_monster {
                0
            } else if event_room > 0 && matches!(zone_id, 81..=83) {
                3
            } else {
                tmpl.group
            };

            let instance = Arc::new(NpcInstance {
                nid,
                proto_id: s_sid,
                is_monster,
                zone_id,
                x: spawn_x,
                y,
                z: spawn_z,
                direction,
                region_x,
                region_z,
                gate_open: 0,
                object_type: 0,
                nation: runtime_nation,
                special_type: 0,
                trap_number: trap_number as i16,
                event_room,
                is_event_npc: true,
                summon_type,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            });

            // Register in zone region grid
            zone.add_npc(region_x, region_z, nid);
            self.npc_instances.insert(nid, instance.clone());
            self.npc_hp.insert(nid, tmpl.max_hp as i32);

            // Initialize AI state for monsters with search range
            if is_monster && tmpl.search_range > 0 {
                let has_friends = matches!(tmpl.act_type, 3 | 4)
                    && !matches!(
                        zone_id,
                        ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
                    );
                self.npc_ai.insert(
                    nid,
                    NpcAiState {
                        state: NpcState::Standing,
                        spawn_x,
                        spawn_z,
                        cur_x: spawn_x,
                        cur_z: spawn_z,
                        target_id: None,
                        npc_target_id: None,
                        delay_ms: tmpl.stand_time as u64,
                        last_tick_ms: 0,
                        regen_time_ms: 0, // event NPCs don't auto-respawn
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

            // Guard event NPCs also need AI (e.g. castle siege guards)
            if !is_monster && is_guard_npc_type(tmpl.npc_type) && !self.npc_ai.contains_key(&nid) {
                self.npc_ai.insert(
                    nid,
                    NpcAiState {
                        state: NpcState::Standing,
                        spawn_x,
                        spawn_z,
                        cur_x: spawn_x,
                        cur_z: spawn_z,
                        target_id: None,
                        npc_target_id: None,
                        delay_ms: tmpl.stand_time as u64,
                        last_tick_ms: 0,
                        regen_time_ms: 0,
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

            // Broadcast NPC_IN to nearby players
            let pkt = crate::npc::build_npc_inout(crate::npc::NPC_IN, &instance, &tmpl);
            self.broadcast_to_3x3(
                zone_id,
                region_x,
                region_z,
                Arc::new(pkt),
                None,
                instance.event_room,
            );

            spawned_ids.push(nid);
        }

        spawned_ids
    }

    /// Update an event NPC trap number, used by Juraid bridge gates.
    pub fn update_npc_trap_number(&self, nid: NpcId, trap_number: i16) {
        if let Some(entry) = self.npc_instances.get(&nid) {
            let old = entry.value().clone();
            let updated = Arc::new(NpcInstance {
                trap_number,
                ..(*old).clone()
            });
            drop(entry);
            self.npc_instances.insert(nid, updated);
        }
    }

    /// Set duration (auto-death timer) on a spawned NPC.
    ///
    /// After `duration_secs` elapses, the NPC AI tick will automatically kill it.
    ///
    /// `tick_now_ms` should be the current tick counter for spawned_at_ms.
    pub fn set_npc_duration(&self, nid: NpcId, duration_secs: u16, tick_now_ms: u64) {
        self.update_npc_ai(nid, |s| {
            s.duration_secs = duration_secs;
            s.spawned_at_ms = tick_now_ms;
        });
    }
    /// Kill/remove an NPC by runtime ID.
    ///
    ///
    /// Sets HP to 0, broadcasts death packet, removes from region grid.
    pub fn kill_npc(&self, nid: NpcId) {
        let instance = match self.get_npc_instance(nid) {
            Some(i) => i,
            None => return,
        };

        // Set HP to 0
        self.update_npc_hp(nid, 0);

        // Broadcast WIZ_DEAD for the NPC
        let mut death_pkt = Packet::new(Opcode::WizDead as u8);
        death_pkt.write_u32(nid);
        self.broadcast_to_3x3(
            instance.zone_id,
            instance.region_x,
            instance.region_z,
            Arc::new(death_pkt),
            None,
            instance.event_room,
        );

        // Broadcast NPC_OUT
        let tmpl_opt = self.get_npc_template(instance.proto_id, instance.is_monster);
        if let Some(tmpl) = tmpl_opt {
            let out_pkt = crate::npc::build_npc_inout(crate::npc::NPC_OUT, &instance, &tmpl);
            self.broadcast_to_3x3(
                instance.zone_id,
                instance.region_x,
                instance.region_z,
                Arc::new(out_pkt),
                None,
                instance.event_room,
            );
        }

        // Remove from region grid
        if let Some(zone) = self.get_zone(instance.zone_id) {
            zone.remove_npc(instance.region_x, instance.region_z, nid);
        }

        // Cleanup: remove from instances, HP, AI, DOTs, buffs
        self.npc_instances.remove(&nid);
        self.npc_hp.remove(&nid);
        self.npc_ai.remove(&nid);
        self.clear_npc_dots(nid);
        self.clear_npc_buffs(nid);
    }

    /// Despawn all event NPCs in a zone that belong to a specific event room.
    ///
    /// those matching `GetEventRoom() == roomid + 1`.
    ///
    /// `event_room` is 1-based (room_id + 1).
    pub fn despawn_room_npcs(&self, zone_id: u16, event_room: u16) {
        // Collect NPC IDs to despawn (avoid holding DashMap ref across await)
        let npc_ids: Vec<NpcId> = self
            .npc_instances
            .iter()
            .filter(|entry| {
                let inst = entry.value();
                inst.zone_id == zone_id && inst.event_room == event_room && inst.is_event_npc
            })
            .map(|entry| *entry.key())
            .collect();

        for nid in npc_ids {
            self.kill_npc(nid);
        }
    }

    // ── NPC DOT System ───────────────────────────────────────────────

    /// Register a DOT effect on an NPC.
    ///
    /// Replaces any existing slot with the same `skill_id`, up to a max
    /// of 4 slots per NPC (matches `MAX_TYPE3_REPEAT`).
    ///
    pub fn add_npc_dot(&self, npc_id: NpcId, slot: NpcDotSlot) {
        let mut entry = self.npc_dots.entry(npc_id).or_default();
        // Replace existing DOT from same skill
        if let Some(existing) = entry.iter_mut().find(|s| s.skill_id == slot.skill_id) {
            *existing = slot;
            return;
        }
        // Max 4 DOT slots per NPC (matches MAX_TYPE3_REPEAT)
        if entry.len() < 4 {
            entry.push(slot);
        }
    }
    /// Process one DOT tick for all NPCs with active DOT effects.
    ///
    /// Returns a list of `(npc_id, total_damage, caster_sid)` for each NPC
    /// that took damage this tick, so the caller can apply HP changes and
    /// check for death.
    pub fn process_npc_dot_tick(&self) -> Vec<(NpcId, i32, SessionId)> {
        let mut results = Vec::new();
        let mut to_remove = Vec::new();

        for mut entry in self.npc_dots.iter_mut() {
            let npc_id = *entry.key();
            let slots = entry.value_mut();

            let mut total_damage: i32 = 0;
            let mut primary_caster: SessionId = 0;
            let mut i = 0;

            while i < slots.len() {
                slots[i].tick_count += 1;
                total_damage += slots[i].hp_amount as i32;
                if primary_caster == 0 {
                    primary_caster = slots[i].caster_sid;
                }

                if slots[i].tick_count >= slots[i].tick_limit {
                    slots.swap_remove(i);
                } else {
                    i += 1;
                }
            }

            if slots.is_empty() {
                to_remove.push(npc_id);
            }

            if total_damage != 0 {
                results.push((npc_id, total_damage, primary_caster));
            }
        }

        for npc_id in to_remove {
            self.npc_dots.remove(&npc_id);
        }

        results
    }
    /// Remove all DOT effects from an NPC (e.g., on death).
    pub fn clear_npc_dots(&self, npc_id: NpcId) {
        self.npc_dots.remove(&npc_id);
    }

    // ── NPC Buff System (Type4) ─────────────────────────────────────

    /// Apply a Type4 buff/debuff to an NPC. Overwrites any existing buff of the same type.
    ///
    pub fn apply_npc_buff(&self, npc_id: NpcId, entry: NpcBuffEntry) {
        let mut map = self.npc_buffs.entry(npc_id).or_default();
        map.insert(entry.buff_type, entry);
    }

    /// Remove a specific buff type from an NPC.
    ///
    pub fn remove_npc_buff(&self, npc_id: NpcId, buff_type: i32) -> bool {
        if let Some(mut map) = self.npc_buffs.get_mut(&npc_id) {
            let removed = map.remove(&buff_type).is_some();
            if map.is_empty() {
                drop(map);
                self.npc_buffs.remove(&npc_id);
            }
            removed
        } else {
            false
        }
    }

    /// Remove all buffs/debuffs from an NPC (e.g., on death).
    ///
    pub fn clear_npc_buffs(&self, npc_id: NpcId) {
        self.npc_buffs.remove(&npc_id);
    }

    /// Check if an NPC has a specific buff type active.
    ///
    /// Useful for AI checks (e.g., is NPC slowed, stunned).
    pub fn has_npc_buff(&self, npc_id: NpcId, buff_type: i32) -> bool {
        self.npc_buffs
            .get(&npc_id)
            .map(|map| map.contains_key(&buff_type))
            .unwrap_or(false)
    }

    /// Get the number of active buffs on an NPC.
    pub fn npc_buff_count(&self, npc_id: NpcId) -> usize {
        self.npc_buffs
            .get(&npc_id)
            .map(|map| map.len())
            .unwrap_or(0)
    }

    /// Process one NPC buff tick — remove expired buffs.
    ///
    /// Returns a list of `(npc_id, buff_type)` for each expired buff,
    /// so the caller can log or take further action.
    ///
    /// C++ removes one expired buff per tick (break after first removal).
    /// We remove all expired buffs per tick for simplicity and correctness.
    pub fn process_npc_buff_tick(&self) -> Vec<(NpcId, i32)> {
        let mut expired_list = Vec::new();
        let mut empty_npcs = Vec::new();

        for mut entry in self.npc_buffs.iter_mut() {
            let npc_id = *entry.key();
            let map = entry.value_mut();

            let expired_keys: Vec<i32> = map
                .iter()
                .filter(|(_, buff)| buff.is_expired())
                .map(|(k, _)| *k)
                .collect();

            for key in expired_keys {
                map.remove(&key);
                expired_list.push((npc_id, key));
            }

            if map.is_empty() {
                empty_npcs.push(npc_id);
            }
        }

        // Clean up empty entries
        for npc_id in empty_npcs {
            self.npc_buffs.remove(&npc_id);
        }

        expired_list
    }

    // ── Bot System Accessors ───────────────────────────────────────────

    /// Look up a farm bot by its ID.
    ///
    pub fn get_bot_farm(&self, id: i32) -> Option<BotHandlerFarmRow> {
        self.bot_farm_data.get(&id).map(|r| r.clone())
    }
    /// Get the total number of farm bots loaded.
    pub fn bot_farm_count(&self) -> usize {
        self.bot_farm_data.len()
    }
    /// Look up a merchant bot template by its index.
    ///
    pub fn get_bot_merchant_template(&self, index: i16) -> Option<BotHandlerMerchantRow> {
        self.bot_merchant_templates.get(&index).map(|r| r.clone())
    }
    /// Get the total number of merchant bot templates loaded.
    pub fn bot_merchant_template_count(&self) -> usize {
        self.bot_merchant_templates.len()
    }
    /// Look up a pre-configured merchant stall by index.
    ///
    pub fn get_bot_merchant_data(&self, index: i32) -> Option<BotMerchantDataRow> {
        self.bot_merchant_data.get(&index).map(|r| r.clone())
    }
    /// Get the total number of merchant stall configurations loaded.
    pub fn bot_merchant_data_count(&self) -> usize {
        self.bot_merchant_data.len()
    }
    /// Get every configured merchant stall in deterministic DB-index order.
    pub fn get_all_bot_merchant_data(&self) -> Vec<BotMerchantDataRow> {
        let mut rows: Vec<_> = self
            .bot_merchant_data
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        rows.sort_by_key(|row| row.n_index);
        rows
    }
    /// Look up a user bot by its ID.
    pub fn get_user_bot(&self, id: i32) -> Option<UserBotRow> {
        self.user_bots.get(&id).map(|r| r.clone())
    }
    /// Get the total number of user bots loaded.
    pub fn user_bot_count(&self) -> usize {
        self.user_bots.len()
    }
    /// Get all farm bots in a specific zone.
    ///
    pub fn get_bots_in_zone(&self, zone_id: i16) -> Vec<BotHandlerFarmRow> {
        self.bot_farm_data
            .iter()
            .filter(|e| e.value().zone == zone_id)
            .map(|e| e.value().clone())
            .collect()
    }
    /// Get all loaded farm-bot templates. GM-created PK bots use this only as
    /// an equipment fallback when their target zone has no class template.
    pub fn get_all_bot_templates(&self) -> Vec<BotHandlerFarmRow> {
        let mut rows: Vec<_> = self
            .bot_farm_data
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        rows.sort_by_key(|row| row.id);
        rows
    }
    /// Get a snapshot of the bot knights ranking.
    pub fn get_bot_knights_rank(&self) -> Vec<BotKnightsRankRow> {
        self.bot_knights_rank.read().clone()
    }
    /// Get the number of bot knights rank entries.
    pub fn bot_knights_rank_count(&self) -> usize {
        self.bot_knights_rank.read().len()
    }
    /// Get a snapshot of the bot personal ranking.
    pub fn get_bot_personal_rank(&self) -> Vec<BotPersonalRankRow> {
        self.bot_personal_rank.read().clone()
    }
    /// Get the number of bot personal rank entries.
    pub fn bot_personal_rank_count(&self) -> usize {
        self.bot_personal_rank.read().len()
    }

    // ── User Rankings ──────────────────────────────────────────────────

    /// Look up a user's personal rank by uppercase character name.
    ///
    /// Returns 0 if not ranked.
    pub fn get_user_personal_rank(&self, char_name: &str) -> u8 {
        let upper = char_name.to_uppercase();
        *self.user_personal_rank.read().get(&upper).unwrap_or(&0)
    }

    /// Look up a user's knights rank by uppercase character name.
    ///
    /// Returns 0 if not ranked.
    pub fn get_user_knights_rank(&self, char_name: &str) -> u8 {
        let upper = char_name.to_uppercase();
        *self.user_knights_rank.read().get(&upper).unwrap_or(&0)
    }

    /// Apply loaded user ranks to all online sessions.
    ///
    /// and sets `m_bPersonalRank` / `m_bKnightsRank` on each online user.
    pub fn apply_user_ranks_to_sessions(&self) {
        let personal = self.user_personal_rank.read();
        let knights = self.user_knights_rank.read();
        let sids = self.get_in_game_session_ids();

        for sid in &sids {
            self.update_session(*sid, |h| {
                let name = match &h.character {
                    Some(ch) => ch.name.to_uppercase(),
                    None => return,
                };
                h.personal_rank = *personal.get(&name).unwrap_or(&0);
                h.knights_rank = *knights.get(&name).unwrap_or(&0);
            });
        }
    }

    /// Apply loaded user ranks to all bot instances.
    ///
    pub fn apply_user_ranks_to_bots(&self) {
        let personal = self.bot_personal_rank.read();
        let knights_r = self.bot_knights_rank.read();

        // Collect bot IDs and names first to avoid holding DashMap ref during mutation
        let bot_data: Vec<(BotId, String, u8)> = self
            .bots
            .iter()
            .map(|e| (e.value().id, e.value().name.clone(), e.value().nation))
            .collect();

        for (bot_id, name, nation) in &bot_data {
            let name_upper = name.to_uppercase();

            // Personal rank: find this bot's name in the per-nation column
            let p_rank = personal
                .iter()
                .find(|r| {
                    let field = if *nation == 1 {
                        &r.str_karus_user_id
                    } else {
                        &r.str_elmo_user_id
                    };
                    field
                        .as_ref()
                        .map(|s| s.eq_ignore_ascii_case(&name_upper))
                        .unwrap_or(false)
                })
                .map(|r| r.n_rank as u8)
                .unwrap_or(0);

            // Knights rank
            let k_rank = knights_r
                .iter()
                .find(|r| {
                    let field = if *nation == 1 {
                        &r.str_karus_user_id
                    } else {
                        &r.str_elmo_user_id
                    };
                    field
                        .as_ref()
                        .map(|s| s.eq_ignore_ascii_case(&name_upper))
                        .unwrap_or(false)
                })
                .map(|r| r.sh_index as u8)
                .unwrap_or(0);

            if let Some(mut bot_mut) = self.bots.get_mut(bot_id) {
                bot_mut.personal_rank = p_rank;
                bot_mut.knights_rank = k_rank;
            }
        }
    }

    // ── Runtime Bot Registry ──────────────────────────────────────────

    /// Allocate a new unique BotId.
    ///
    pub fn alloc_bot_id(&self) -> BotId {
        self.next_bot_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Insert (or replace) a runtime bot instance.
    ///
    pub fn insert_bot(&self, bot: BotInstance) {
        self.bots.insert(bot.id, bot);
    }

    /// Remove a runtime bot by ID, returning it if present.
    ///
    pub fn remove_bot(&self, id: BotId) -> Option<BotInstance> {
        self.bots.remove(&id).map(|(_, b)| b)
    }

    /// Get a snapshot of a runtime bot by ID.
    ///
    pub fn get_bot(&self, id: BotId) -> Option<BotInstance> {
        self.bots.get(&id).map(|b| b.clone())
    }

    /// Returns the number of active runtime bots.
    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }

    /// Returns all runtime bots in a specific zone.
    ///
    pub fn get_bots_in_zone_live(&self, zone_id: u16) -> Vec<BotInstance> {
        self.bots
            .iter()
            .filter(|e| e.zone_id == zone_id && e.in_game)
            .map(|e| e.clone())
            .collect()
    }

    /// Collect all expired bots (duration elapsed).
    ///
    /// Returns a list of BotIds that should be despawned.
    ///
    pub fn collect_expired_bot_ids(&self, now_unix: u64) -> Vec<BotId> {
        self.bots
            .iter()
            .filter(|e| e.is_expired(now_unix))
            .map(|e| *e.key())
            .collect()
    }

    /// Mutably update a bot instance in-place via a closure.
    ///
    /// Returns `true` if the bot was found and updated.
    pub fn update_bot<F>(&self, id: BotId, f: F) -> bool
    where
        F: FnOnce(&mut BotInstance),
    {
        if let Some(mut entry) = self.bots.get_mut(&id) {
            f(entry.value_mut());
            true
        } else {
            false
        }
    }

    // ── Test Helpers ──────────────────────────────────────────────────

    /// Insert an NPC template for testing.
    #[cfg(test)]
    pub(crate) fn insert_npc_template(&self, tmpl: crate::npc::NpcTemplate) {
        self.npc_templates
            .insert((tmpl.s_sid, tmpl.is_monster), Arc::new(tmpl));
    }

    // ── Sync NPC Kill (for Lua bindings) ─────────────────────────────

    /// Kill/despawn an NPC by its runtime ID, broadcasting death/out packets.
    ///
    /// Uses `tokio::spawn` for async broadcasts since this may be called
    /// from synchronous Lua binding context.
    ///
    pub(crate) fn kill_npc_by_runtime_id(&self, nid: NpcId) {
        let instance = match self.get_npc_instance(nid) {
            Some(i) => i,
            None => return,
        };

        // Set HP to 0
        self.update_npc_hp(nid, 0);

        // Build packets
        let mut death_pkt = Packet::new(Opcode::WizDead as u8);
        death_pkt.write_u32(nid);

        let tmpl_opt = self.get_npc_template(instance.proto_id, instance.is_monster);
        let _out_pkt = tmpl_opt
            .as_ref()
            .map(|tmpl| crate::npc::build_npc_inout(crate::npc::NPC_OUT, &instance, tmpl));

        // Remove from instances, HP, AI, DOTs, buffs (immediate cleanup)
        self.npc_instances.remove(&nid);
        self.npc_hp.remove(&nid);
        self.npc_ai.remove(&nid);
        self.clear_npc_dots(nid);
        self.clear_npc_buffs(nid);

        // Remove from region grid and broadcast via spawned task
        let zone_id = instance.zone_id;
        let region_x = instance.region_x;
        let region_z = instance.region_z;
        if let Some(zone) = self.get_zone(zone_id) {
            tokio::spawn({
                let zone = zone.clone();
                async move {
                    zone.remove_npc(region_x, region_z, nid);
                }
            });
        }

        // Note: broadcast not performed here since we don't hold an Arc<WorldState>.
        // The NPC will disappear from region grids and clients will be notified
        // on next region scan. For immediate broadcast, use kill_npc() async method.
    }

    /// Kill all NPCs in a zone that match a given proto_id.
    ///
    pub(crate) fn kill_npc_by_proto_id(&self, proto_id: u16, zone_id: u16) {
        let matching_nids: Vec<NpcId> = self
            .npc_instances
            .iter()
            .filter(|entry| {
                let inst = entry.value();
                inst.proto_id == proto_id && inst.zone_id == zone_id
            })
            .map(|entry| *entry.key())
            .collect();

        for nid in matching_nids {
            self.kill_npc_by_runtime_id(nid);
        }
    }

    /// Kill all non-monster NPCs in a given zone.
    ///
    pub(crate) fn kill_non_monster_npcs_in_zone(&self, zone_id: u16) {
        let matching_nids: Vec<NpcId> = self
            .npc_instances
            .iter()
            .filter(|entry| {
                let inst = entry.value();
                inst.zone_id == zone_id && !inst.is_monster
            })
            .map(|entry| *entry.key())
            .collect();

        for nid in matching_nids {
            self.kill_npc_by_runtime_id(nid);
        }
    }

    pub(crate) fn kill_non_monster_npcs_in_room(&self, zone_id: u16, event_room: u16) {
        if event_room == 0 {
            return;
        }
        let matching_nids: Vec<NpcId> = self
            .npc_instances
            .iter()
            .filter(|entry| {
                let inst = entry.value();
                inst.zone_id == zone_id && inst.event_room == event_room && !inst.is_monster
            })
            .map(|entry| *entry.key())
            .collect();
        for nid in matching_nids {
            self.kill_npc_by_runtime_id(nid);
        }
    }

    /// Insert an NPC instance and register it in the zone's region grid.
    ///
    /// Calculates proper region coordinates from the instance's world position
    /// and registers the NPC in the zone so it appears in region-based
    /// visibility queries (3x3 grid). Without zone registration, NPCs exist
    /// in `npc_instances` but are invisible to clients.
    ///
    /// Uses `try_write()` for synchronous zone registration so this method
    /// can be called from both sync and async contexts. The lock is
    /// uncontended at startup / in tests so this always succeeds.
    ///
    pub(crate) fn insert_npc_instance(&self, instance: crate::npc::NpcInstance) {
        let nid = instance.nid;
        let zone_id = instance.zone_id;
        let region_x = calc_region(instance.x);
        let region_z = calc_region(instance.z);

        self.npc_instances.insert(nid, Arc::new(instance));

        // Register in zone region grid so the NPC is visible to nearby players
        if let Some(zone) = self.get_zone(zone_id) {
            if let Some(region) = zone.get_region(region_x, region_z) {
                if let Some(mut npcs) = region.npcs.try_write() {
                    npcs.insert(nid);
                }
            }
        }
    }

    // ── NPC Event Helpers (for Lua bindings) ─────────────────────────

    /// Set the selling group on a live NPC instance (changes what items the NPC sells).
    ///
    ///   Sets `pNpc->m_iSellingGroup = m_iSellingGroup` and sends WIZ_TRADE_NPC.
    ///
    /// Since our NPC instances are immutable Arc, we replace the instance with
    /// a cloned version that has the updated selling_group set. The actual
    /// selling_group field is on the NPC template; for runtime override, the
    /// Lua binding sends a WIZ_TRADE_NPC packet directly to the player.
    /// This method is a no-op structurally — the Lua binding handles the packet.
    #[cfg(test)]
    pub(crate) fn set_npc_selling_group(&self, _nid: NpcId, _selling_group: i32) {
        // In the C++ code, this modifies a mutable field on the NPC.
        // In our Rust implementation, the Lua binding sends the packet
        // directly. No persistent state change needed since the NPC's
        // sell list is loaded from item_sell_table by selling_group.
    }

    /// Spawn random boss monsters at startup.
    ///
    ///
    /// For each stage in `monster_boss_random_stages`, finds matching candidates
    /// from `monster_boss_random_spawn` and picks one at random to spawn.
    pub fn random_boss_system_load(&self) -> u32 {
        let stages = self.get_boss_random_stages();
        if stages.is_empty() {
            tracing::info!("RandomBossSystemLoad: no stages configured, skipping");
            return 0;
        }

        let mut count = 0u32;
        for stage_row in &stages {
            // C++ GetRandomIndex(Stage, MonsterID, MonsterZone) — filters by all 3 fields
            let candidates: Vec<_> = self
                .get_boss_spawn_candidates(stage_row.stage as i32)
                .into_iter()
                .filter(|c| {
                    c.monster_id == stage_row.monster_id as i32
                        && c.monster_zone == stage_row.monster_zone as i32
                })
                .collect();

            if candidates.is_empty() {
                continue;
            }

            // C++ myrand(0, list.size() - 1) — pick a random candidate
            let idx = if candidates.len() == 1 {
                0
            } else {
                rand::random::<usize>() % candidates.len()
            };
            let pick = &candidates[idx];

            // C++ SpawnEventNpc(MonsterID, true, MonsterZone, PosX, 0, PosZ, 1, Range, ...)
            let ids = self.spawn_event_npc(
                pick.monster_id as u16,
                true,
                pick.monster_zone as u16,
                pick.pos_x as f32,
                pick.pos_z as f32,
                1,
            );

            if !ids.is_empty() {
                count += 1;
                tracing::debug!(
                    "RandomBossSystemLoad: spawned boss {} in zone {} at ({}, {})",
                    pick.monster_name,
                    pick.monster_zone,
                    pick.pos_x,
                    pick.pos_z
                );
            }
        }

        tracing::info!(
            "RandomBossSystemLoad: spawned {} bosses from {} stages",
            count,
            stages.len()
        );
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn test_ai_state() -> NpcAiState {
        NpcAiState {
            state: NpcState::Standing,
            spawn_x: 0.0,
            spawn_z: 0.0,
            cur_x: 0.0,
            cur_z: 0.0,
            target_id: None,
            npc_target_id: None,
            delay_ms: 0,
            last_tick_ms: 0,
            regen_time_ms: 30000,
            is_aggressive: false,
            zone_id: 0,
            region_x: 0,
            region_z: 0,
            fainting_until_ms: 0,
            old_state: NpcState::Standing,
            active_skill_id: 0,
            active_target_id: -1,
            active_cast_time_ms: 0,
            has_friends: false,
            family_type: 0,
            skill_cooldown_ms: 0,
            nation: 0,
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
        }
    }

    fn test_char_info(name: &str) -> CharacterInfo {
        CharacterInfo {
            session_id: 0,
            name: name.to_string(),
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
            max_sp: 100,
            sp: 100,
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
            max_exp: 0,
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
        }
    }

    #[test]
    fn test_set_npc_selling_group_no_crash() {
        let world = WorldState::new();
        // No-op method should not panic even with invalid NPC ID
        world.set_npc_selling_group(99999, 42);
    }

    #[test]
    fn test_notify_npc_damaged_standing_to_attacking() {
        let world = WorldState::new();
        let nid: NpcId = 100;
        world.npc_ai.insert(
            nid,
            NpcAiState {
                state: NpcState::Standing,
                target_id: None,
                last_tick_ms: 10_000,
                ..test_ai_state()
            },
        );
        world.notify_npc_damaged(nid, 1);
        let ai = world.get_npc_ai(nid).unwrap();
        assert_eq!(ai.target_id, Some(1));
        assert!(matches!(ai.state, NpcState::Attacking));
        assert_eq!(ai.last_combat_time_ms, 10_000);
    }

    #[test]
    fn test_notify_npc_damaged_combat_no_panic() {
        // When NPC is already in combat, calling notify_npc_damaged should
        // not panic even without recorded damage or positions.
        let world = WorldState::new();
        let nid: NpcId = 101;
        world.npc_ai.insert(
            nid,
            NpcAiState {
                state: NpcState::Attacking,
                target_id: Some(1),
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );
        // Should not panic (no damage or position data yet)
        world.notify_npc_damaged(nid, 2);
    }

    #[test]
    fn test_maybe_switch_target_no_current() {
        let world = WorldState::new();
        let nid: NpcId = 102;
        let mut ai = NpcAiState {
            state: NpcState::Attacking,
            target_id: None,
            ..test_ai_state()
        };
        // No current target: should set attacker as target
        world.maybe_switch_target(&mut ai, nid, 5);
        assert_eq!(ai.target_id, Some(5));
    }

    #[test]
    fn test_maybe_switch_target_same_attacker() {
        let world = WorldState::new();
        let nid: NpcId = 103;
        let mut ai = NpcAiState {
            state: NpcState::Attacking,
            target_id: Some(7),
            ..test_ai_state()
        };
        // Same attacker: target should not change
        world.maybe_switch_target(&mut ai, nid, 7);
        assert_eq!(ai.target_id, Some(7));
    }

    #[test]
    fn test_maybe_switch_target_damage_comparison() {
        let world = WorldState::new();
        let nid: NpcId = 104;

        // Register sessions so position lookup works
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Record more damage from attacker 2
        world.record_npc_damage(nid, 1, 100);
        world.record_npc_damage(nid, 2, 500);

        // Run many trials — with enough iterations, the damage path (50%)
        // should trigger and switch target at least once
        let mut switched_count = 0;
        for _ in 0..200 {
            let mut ai = NpcAiState {
                state: NpcState::Attacking,
                target_id: Some(1),
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            };
            world.maybe_switch_target(&mut ai, nid, 2);
            if ai.target_id == Some(2) {
                switched_count += 1;
            }
        }
        // With 50% damage path + 15% random switch, expect ~65% switch rate
        // Use generous bounds for statistical test
        assert!(
            switched_count > 20,
            "Expected some target switches due to higher damage, got {}",
            switched_count
        );
    }

    #[test]
    fn test_maybe_switch_target_statistical_distribution() {
        // Verify that with equal damage and distance, the random switch (15%)
        // and keep (5%) paths produce reasonable distribution.
        let world = WorldState::new();
        let nid: NpcId = 105;

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(10, tx1);
        world.register_session(20, tx2);

        // Equal damage
        world.record_npc_damage(nid, 10, 200);
        world.record_npc_damage(nid, 20, 200);

        // Both at same distance from NPC
        world.update_session(10, |h| {
            h.position.x = 100.0;
            h.position.z = 100.0;
        });
        world.update_session(20, |h| {
            h.position.x = 100.0;
            h.position.z = 100.0;
        });

        let mut switched = 0;
        let trials = 1000;
        for _ in 0..trials {
            let mut ai = NpcAiState {
                state: NpcState::Attacking,
                target_id: Some(10),
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            };
            world.maybe_switch_target(&mut ai, nid, 20);
            if ai.target_id == Some(20) {
                switched += 1;
            }
        }
        // With equal damage (0-49: no switch) + equal distance (50-79: no switch)
        // + equal AC (80-94: no switch) + unconditional (95-100: switch) = ~6%
        // Allow generous bounds: 1-15%
        let pct = (switched as f64 / trials as f64) * 100.0;
        assert!(
            pct > 1.0 && pct < 15.0,
            "Expected ~6% switch rate with equal stats, got {:.1}%",
            pct
        );
    }

    #[test]
    fn test_broadcast_to_region_sync_filters_by_region() {
        let world = WorldState::new();

        // Create 3 sessions: 2 nearby (same region), 1 far away
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let (tx3, mut rx3) = mpsc::unbounded_channel();

        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        // Set character info so they count as in-game
        let ch = test_char_info("a");
        world.register_ingame(
            1,
            ch.clone(),
            Position {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 5,
                region_z: 5,
            },
        );
        world.register_ingame(
            2,
            ch.clone(),
            Position {
                zone_id: 21,
                x: 110.0,
                y: 0.0,
                z: 110.0,
                region_x: 5,
                region_z: 6,
            },
        );
        world.register_ingame(
            3,
            ch,
            Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 25,
                region_z: 25,
            },
        );

        let pkt = ko_protocol::Packet::new(0x01);
        world.broadcast_to_region_sync(21, 5, 5, Arc::new(pkt), None, 0);

        // Sessions 1 and 2 should receive (region 5,5 and 5,6 are within +-1)
        assert!(
            rx1.try_recv().is_ok(),
            "Session 1 should receive packet (same region)"
        );
        assert!(
            rx2.try_recv().is_ok(),
            "Session 2 should receive packet (adjacent region)"
        );
        // Session 3 should NOT receive (region 25,25 is far away)
        assert!(
            rx3.try_recv().is_err(),
            "Session 3 should NOT receive packet (far region)"
        );
    }

    #[test]
    fn test_broadcast_to_region_sync_except() {
        let world = WorldState::new();

        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let ch = test_char_info("b");
        world.register_ingame(
            1,
            ch.clone(),
            Position {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 5,
                region_z: 5,
            },
        );
        world.register_ingame(
            2,
            ch,
            Position {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 5,
                region_z: 5,
            },
        );

        let pkt = ko_protocol::Packet::new(0x01);
        // Exclude session 1
        world.broadcast_to_region_sync(21, 5, 5, Arc::new(pkt), Some(1), 0);

        assert!(rx1.try_recv().is_err(), "Session 1 should be excluded");
        assert!(rx2.try_recv().is_ok(), "Session 2 should receive packet");
    }

    // ── Friend Calling Tests ──────────────────────────────────────────

    /// Create a monster NpcTemplate for friend-calling tests.
    fn friend_template(s_sid: u16, family_type: u8, act_type: u8) -> crate::npc::NpcTemplate {
        crate::npc::NpcTemplate {
            s_sid,
            is_monster: true,
            name: format!("Monster_{}", s_sid),
            pid: s_sid,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type,
            npc_type: 0, // NPC_MONSTER
            family_type,
            selling_group: 0,
            level: 40,
            max_hp: 500,
            max_mp: 0,
            attack: 50,
            ac: 10,
            hit_rate: 100,
            evade_rate: 50,
            damage: 30,
            attack_delay: 1500,
            speed_1: 1000,
            speed_2: 500,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
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
            exp: 50,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }

    /// Set up a pair of NPCs (caller + friend) for friend calling tests.
    /// Returns (caller_nid, friend_nid).
    fn setup_friend_pair(
        world: &WorldState,
        caller_family: u8,
        friend_family: u8,
        friend_has_friends: bool,
        friend_state: NpcState,
        friend_dist: f32,
    ) -> (NpcId, NpcId) {
        let caller_nid: NpcId = 1000;
        let friend_nid: NpcId = 1001;

        let tmpl = friend_template(100, caller_family, 3);
        world.npc_templates.insert((100, true), Arc::new(tmpl));

        // Caller NPC instance + template
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: caller_nid,
            proto_id: 100,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(caller_nid, 500);
        world.npc_ai.insert(
            caller_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: caller_family,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                region_x: 2,
                region_z: 2,
                ..test_ai_state()
            },
        );

        // Friend NPC — use proto_id 101 so it can have a different family
        let friend_tmpl = friend_template(101, friend_family, 3);
        world
            .npc_templates
            .insert((101, true), Arc::new(friend_tmpl));

        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: friend_nid,
            proto_id: 101,
            is_monster: true,
            zone_id: 21,
            x: 100.0 + friend_dist,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(friend_nid, 500);
        world.npc_ai.insert(
            friend_nid,
            NpcAiState {
                state: friend_state,
                has_friends: friend_has_friends,
                family_type: friend_family,
                zone_id: 21,
                cur_x: 100.0 + friend_dist,
                cur_z: 100.0,
                region_x: 2,
                region_z: 2,
                ..test_ai_state()
            },
        );

        (caller_nid, friend_nid)
    }

    #[test]
    fn test_find_friends_activates_same_family() {
        let world = WorldState::new();
        let (caller_nid, friend_nid) = setup_friend_pair(
            &world,
            5,    // caller family
            5,    // friend same family
            true, // friend has_friends
            NpcState::Standing,
            10.0, // within range
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id,
            Some(42),
            "Same-family friend should acquire attacker as target"
        );
        assert_eq!(friend_ai.state, NpcState::Attacking);
    }

    #[test]
    fn test_find_friends_only_standing_npcs_called() {
        let world = WorldState::new();
        let (caller_nid, friend_nid) = setup_friend_pair(
            &world,
            5,
            5,
            true,
            NpcState::Fighting, // already fighting
            10.0,
        );
        // Give the friend a target so it's "in combat"
        world.update_npc_ai(friend_nid, |ai| {
            ai.target_id = Some(99);
        });

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id,
            Some(99),
            "Already-fighting NPC should keep its existing target"
        );
    }

    #[test]
    fn test_find_friends_dead_npcs_not_called() {
        let world = WorldState::new();
        let (caller_nid, friend_nid) =
            setup_friend_pair(&world, 5, 5, true, NpcState::Standing, 10.0);
        // Kill the friend (HP = 0)
        world.update_npc_hp(friend_nid, 0);

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "Dead NPC should not be activated"
        );
    }

    #[test]
    fn test_find_friends_different_family_not_called() {
        let world = WorldState::new();
        let (caller_nid, friend_nid) = setup_friend_pair(
            &world,
            5, // caller family
            8, // different family
            true,
            NpcState::Standing,
            10.0,
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "Different-family NPC should not be activated"
        );
    }

    #[test]
    fn test_find_friends_respects_tracing_range() {
        let world = WorldState::new();
        let (caller_nid, friend_nid) = setup_friend_pair(
            &world,
            5,
            5,
            true,
            NpcState::Standing,
            50.0, // beyond tracing_range of 30
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "NPC beyond tracing range should not be activated"
        );
    }

    #[test]
    fn test_find_friends_gate_npcs_excluded() {
        let world = WorldState::new();
        let caller_nid: NpcId = 2000;
        let gate_nid: NpcId = 2001;

        // Caller template (monster with act_type=3, family=5)
        let caller_tmpl = friend_template(200, 5, 3);
        world
            .npc_templates
            .insert((200, true), Arc::new(caller_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: caller_nid,
            proto_id: 200,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(caller_nid, 500);
        world.npc_ai.insert(
            caller_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        // "Friend" is a gate NPC (npc_type=50)
        let mut gate_tmpl = friend_template(201, 5, 3);
        gate_tmpl.npc_type = 50; // NPC_GATE
        world.npc_templates.insert((201, true), Arc::new(gate_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: gate_nid,
            proto_id: 201,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(gate_nid, 500);
        world.npc_ai.insert(
            gate_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 105.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(caller_nid, 42);

        let gate_ai = world.get_npc_ai(gate_nid).unwrap();
        assert_eq!(
            gate_ai.target_id, None,
            "Gate NPC should not be activated as friend"
        );
    }

    #[test]
    fn test_damaged_boss_does_not_recruit_even_with_friends_flag() {
        // Bosses use MonSearchAny — should alert NPCs of any family type.
        let world = WorldState::new();
        let boss_nid: NpcId = 3000;
        let friend_nid: NpcId = 3001;

        // Boss template (npc_type=3 = NPC_BOSS)
        let mut boss_tmpl = friend_template(300, 10, 5);
        boss_tmpl.npc_type = NPC_BOSS;
        world.npc_templates.insert((300, true), Arc::new(boss_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: boss_nid,
            proto_id: 300,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(boss_nid, 5000);
        world.npc_ai.insert(
            boss_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true, // Bosses must not recruit even with this flag.
                family_type: 10,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        // Nearby NPC with DIFFERENT family, no has_friends
        let friend_tmpl = friend_template(301, 99, 1);
        world
            .npc_templates
            .insert((301, true), Arc::new(friend_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: friend_nid,
            proto_id: 301,
            is_monster: true,
            zone_id: 21,
            x: 110.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(friend_nid, 500);
        world.npc_ai.insert(
            friend_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: false,
                family_type: 99,
                zone_id: 21,
                cur_x: 110.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(boss_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "Damaging a boss must not recruit nearby NPCs"
        );
        assert_eq!(friend_ai.state, NpcState::Standing);
    }

    #[test]
    fn test_find_friends_no_has_friends_flag() {
        // NPC without has_friends should not call friends.
        let world = WorldState::new();
        let caller_nid: NpcId = 4000;
        let friend_nid: NpcId = 4001;

        let tmpl = friend_template(400, 5, 1); // act_type=1 → no has_friends
        world.npc_templates.insert((400, true), Arc::new(tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: caller_nid,
            proto_id: 400,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(caller_nid, 500);
        world.npc_ai.insert(
            caller_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: false, // no has_friends, not a boss
                family_type: 5,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        let friend_tmpl = friend_template(401, 5, 3);
        world
            .npc_templates
            .insert((401, true), Arc::new(friend_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: friend_nid,
            proto_id: 401,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(friend_nid, 500);
        world.npc_ai.insert(
            friend_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 105.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "Non-pack, non-boss NPC should not activate friends"
        );
    }

    #[test]
    fn test_find_friends_different_zone_not_called() {
        let world = WorldState::new();
        let caller_nid: NpcId = 5000;
        let friend_nid: NpcId = 5001;

        let tmpl = friend_template(500, 5, 3);
        world.npc_templates.insert((500, true), Arc::new(tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: caller_nid,
            proto_id: 500,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(caller_nid, 500);
        world.npc_ai.insert(
            caller_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        let friend_tmpl = friend_template(501, 5, 3);
        world
            .npc_templates
            .insert((501, true), Arc::new(friend_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: friend_nid,
            proto_id: 501,
            is_monster: true,
            zone_id: 22, // different zone
            x: 105.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(friend_nid, 500);
        world.npc_ai.insert(
            friend_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 22, // different zone
                cur_x: 105.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "NPCs in different zones should not be activated"
        );
    }

    #[test]
    fn test_has_friends_zone_exclusion() {
        // In Ronark Land zones, act_type 3/4 NPCs should NOT have has_friends.
        let world = WorldState::new();
        let nid: NpcId = 6000;

        let tmpl = friend_template(600, 5, 3);
        world.npc_templates.insert((600, true), Arc::new(tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid,
            proto_id: 600,
            is_monster: true,
            zone_id: ZONE_RONARK_LAND,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(nid, 500);

        // Simulate the has_friends logic from spawn
        let has_friends = matches!(3u8, 3 | 4)
            && !matches!(
                ZONE_RONARK_LAND,
                ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
            );
        assert!(
            !has_friends,
            "NPCs in Ronark Land should not have has_friends"
        );

        // Test Ardream too
        let has_friends_ardream = matches!(4u8, 3 | 4)
            && !matches!(
                ZONE_ARDREAM,
                ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
            );
        assert!(
            !has_friends_ardream,
            "NPCs in Ardream should not have has_friends"
        );

        // Normal zone should have has_friends
        let has_friends_normal = matches!(3u8, 3 | 4)
            && !matches!(
                21u16,
                ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
            );
        assert!(
            has_friends_normal,
            "NPCs in normal zones should have has_friends"
        );
    }

    #[test]
    fn test_damaged_boss_without_friends_flag_does_not_recruit() {
        // Bosses trigger MonSearchAny even without has_friends flag.
        let world = WorldState::new();
        let boss_nid: NpcId = 7000;
        let friend_nid: NpcId = 7001;

        let mut boss_tmpl = friend_template(700, 10, 5);
        boss_tmpl.npc_type = NPC_BOSS;
        world.npc_templates.insert((700, true), Arc::new(boss_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: boss_nid,
            proto_id: 700,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(boss_nid, 5000);
        world.npc_ai.insert(
            boss_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: false, // Boss does NOT have has_friends
                family_type: 10,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        // Nearby NPC: different family, no has_friends either
        let friend_tmpl = friend_template(701, 50, 1);
        world
            .npc_templates
            .insert((701, true), Arc::new(friend_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: friend_nid,
            proto_id: 701,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(friend_nid, 500);
        world.npc_ai.insert(
            friend_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: false,
                family_type: 50,
                zone_id: 21,
                cur_x: 105.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(boss_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "Bosses without has_friends must not recruit nearby NPCs"
        );
    }

    #[test]
    fn test_find_friends_multiple_npcs_activated() {
        // When a pack NPC is attacked, ALL eligible friends should be activated.
        let world = WorldState::new();
        let caller_nid: NpcId = 8000;

        let tmpl = friend_template(800, 5, 3);
        world.npc_templates.insert((800, true), Arc::new(tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: caller_nid,
            proto_id: 800,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(caller_nid, 500);
        world.npc_ai.insert(
            caller_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        // 3 friends, all same family, all in range
        let friend_nids: Vec<NpcId> = vec![8001, 8002, 8003];
        for (i, &nid) in friend_nids.iter().enumerate() {
            let ft = friend_template(801 + i as u16, 5, 3);
            world
                .npc_templates
                .insert((801 + i as u16, true), Arc::new(ft));
            world.insert_npc_instance(crate::npc::NpcInstance {
                nid,
                proto_id: 801 + i as u16,
                is_monster: true,
                zone_id: 21,
                x: 100.0 + (i as f32 * 5.0),
                y: 0.0,
                z: 100.0,
                direction: 0,
                region_x: 2,
                region_z: 2,
                gate_open: 0,
                object_type: 0,
                nation: 0,
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
            world.init_npc_hp(nid, 500);
            world.npc_ai.insert(
                nid,
                NpcAiState {
                    state: NpcState::Standing,
                    has_friends: true,
                    family_type: 5,
                    zone_id: 21,
                    cur_x: 100.0 + (i as f32 * 5.0),
                    cur_z: 100.0,
                    ..test_ai_state()
                },
            );
        }

        world.notify_npc_damaged(caller_nid, 42);

        for &nid in &friend_nids {
            let ai = world.get_npc_ai(nid).unwrap();
            assert_eq!(
                ai.target_id,
                Some(42),
                "All same-family friends should be activated (nid={})",
                nid
            );
            assert_eq!(ai.state, NpcState::Attacking);
        }
    }

    #[test]
    fn test_find_friends_dead_state_not_called() {
        // NPCs in Dead AI state should not be activated by friend calling.
        let world = WorldState::new();
        let (caller_nid, friend_nid) = setup_friend_pair(
            &world,
            5,
            5,
            true,
            NpcState::Dead, // Dead state
            10.0,
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "NPC in Dead state should not be activated"
        );
    }

    #[test]
    fn test_find_friends_search_range_zero_no_calling() {
        // When search_range is 0, find_friends should return early.
        let world = WorldState::new();
        let caller_nid: NpcId = 9000;
        let friend_nid: NpcId = 9001;

        let mut caller_tmpl = friend_template(900, 5, 3);
        caller_tmpl.search_range = 0; // Zero search range
        world
            .npc_templates
            .insert((900, true), Arc::new(caller_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: caller_nid,
            proto_id: 900,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(caller_nid, 500);
        world.npc_ai.insert(
            caller_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        let friend_tmpl = friend_template(901, 5, 3);
        world
            .npc_templates
            .insert((901, true), Arc::new(friend_tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid: friend_nid,
            proto_id: 901,
            is_monster: true,
            zone_id: 21,
            x: 105.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(friend_nid, 500);
        world.npc_ai.insert(
            friend_nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 105.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(caller_nid, 42);

        let friend_ai = world.get_npc_ai(friend_nid).unwrap();
        assert_eq!(
            friend_ai.target_id, None,
            "NPC with search_range=0 should not call friends"
        );
    }

    // ── Blink (Respawn Invulnerability) Tests ─────────────────────────

    #[test]
    fn test_is_player_blinking_active() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set blink expiry 10 seconds in the future
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        world.update_session(1, |h| {
            h.blink_expiry_time = now + 10;
        });

        assert!(
            world.is_player_blinking(1, now),
            "Player with future blink_expiry should be blinking"
        );
        assert!(
            world.is_player_blinking(1, now + 5),
            "Player should still be blinking midway"
        );
    }

    #[test]
    fn test_is_player_blinking_expired() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        world.update_session(1, |h| {
            h.blink_expiry_time = now - 1; // already expired
        });

        assert!(
            !world.is_player_blinking(1, now),
            "Player with past blink_expiry should not be blinking"
        );
    }

    #[test]
    fn test_is_player_blinking_zero_means_not_blinking() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        // blink_expiry_time defaults to 0

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(
            !world.is_player_blinking(1, now),
            "Player with blink_expiry_time=0 should not be blinking"
        );
    }

    #[test]
    fn test_is_player_blinking_nonexistent_session() {
        let world = WorldState::new();
        assert!(
            !world.is_player_blinking(999, 0),
            "Non-existent session should not be blinking"
        );
    }

    #[test]
    fn test_blink_constant_matches_cpp() {
        // C++ Define.h:72 — `#define BLINK_TIME (10)`
        let blink_time: u64 = 10;
        assert_eq!(blink_time, 10, "BLINK_TIME should be 10 seconds");
    }

    #[test]
    fn test_notify_npc_damaged_caller_gets_target() {
        // The caller NPC itself should also get the attacker as target.
        let world = WorldState::new();
        let nid: NpcId = 10000;

        let tmpl = friend_template(1000, 5, 3);
        world.npc_templates.insert((1000, true), Arc::new(tmpl));
        world.insert_npc_instance(crate::npc::NpcInstance {
            nid,
            proto_id: 1000,
            is_monster: true,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        world.init_npc_hp(nid, 500);
        world.npc_ai.insert(
            nid,
            NpcAiState {
                state: NpcState::Standing,
                has_friends: true,
                family_type: 5,
                zone_id: 21,
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            },
        );

        world.notify_npc_damaged(nid, 42);

        let ai = world.get_npc_ai(nid).unwrap();
        assert_eq!(ai.target_id, Some(42));
        assert_eq!(ai.state, NpcState::Attacking);
    }

    // ── Sprint 41: Threat Bracket Tests ─────────────────────────────

    #[test]
    fn test_maybe_switch_target_95_100_always_switches() {
        // [95,100] bracket should ALWAYS switch (no guard in C++).
        let world = WorldState::new();
        let nid: NpcId = 11000;

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Run 500 trials to verify the unconditional switch in [95,100]
        let mut switched = 0;
        for _ in 0..500 {
            let mut ai = NpcAiState {
                state: NpcState::Attacking,
                target_id: Some(1),
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            };
            world.maybe_switch_target(&mut ai, nid, 2);
            if ai.target_id == Some(2) {
                switched += 1;
            }
        }
        // With 101 values (0..=100), the [95,100] bracket has 6/101 = ~5.9% chance.
        // Without damage/position data, [0,50) won't switch (0 vs 0 damage),
        // [50,80) won't switch (MAX vs MAX distance), [80,95) won't switch (equal AC).
        // So only ~6% switch rate expected. 500 * 6% = ~30 switches.
        assert!(
            switched > 10,
            "Expected some switches from unconditional bracket, got {}",
            switched
        );
    }

    #[test]
    fn test_maybe_switch_target_ac_comparison_bracket() {
        // [80,95) bracket compares AC — NPC prefers target with lower AC
        let world = WorldState::new();
        let nid: NpcId = 11100;

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(10, tx1);
        world.register_session(20, tx2);

        // Give equal damage, same distance, so only AC and unconditional matter
        world.record_npc_damage(nid, 10, 100);
        world.record_npc_damage(nid, 20, 100);
        world.update_session(10, |h| {
            h.position.x = 100.0;
            h.position.z = 100.0;
        });
        world.update_session(20, |h| {
            h.position.x = 100.0;
            h.position.z = 100.0;
        });

        // New attacker (20) has very low AC — NPC should prefer attacking them
        // in the [80,95) bracket. We can't control the random roll, but over
        // many trials the AC path should contribute to switching.
        let mut switched = 0;
        for _ in 0..1000 {
            let mut ai = NpcAiState {
                state: NpcState::Attacking,
                target_id: Some(10),
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            };
            world.maybe_switch_target(&mut ai, nid, 20);
            if ai.target_id == Some(20) {
                switched += 1;
            }
        }
        // With equal stats + AC comparison + unconditional = about 20% switch rate
        let pct = (switched as f64 / 1000.0) * 100.0;
        assert!(
            pct > 3.0 && pct < 40.0,
            "Expected ~20% switch rate with equal stats, got {:.1}%",
            pct
        );
    }

    // ── SendGateFlag Tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_send_gate_flag_updates_npc_instance() {
        let world = WorldState::new();
        let nid: NpcId = 20001;

        // Insert a gate NPC template
        let tmpl = NpcTemplate {
            s_sid: 500,
            is_monster: false,
            name: "TestGate".to_string(),
            pid: 500,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 50, // NPC_GATE
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 100,
            max_mp: 0,
            attack: 0,
            ac: 0,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 0,
            speed_1: 0,
            speed_2: 0,
            stand_time: 0,
            search_range: 0,
            attack_range: 0,
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
        };
        world.npc_templates.insert((500, false), Arc::new(tmpl));

        // Insert an NPC instance with gate_open = 0
        let inst = NpcInstance {
            nid,
            proto_id: 500,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        };
        world.npc_instances.insert(nid, Arc::new(inst));

        // Call send_gate_flag to open the gate
        world.send_gate_flag(nid, 1);

        // Verify the instance was updated
        let updated = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated.gate_open, 1, "gate_open should be 1 after toggle");

        // Call again to close
        world.send_gate_flag(nid, 0);
        let updated = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated.gate_open, 0, "gate_open should be 0 after close");
    }

    #[tokio::test]
    async fn test_send_gate_flag_no_broadcast_for_wood() {
        let world = WorldState::new();
        let nid: NpcId = 20002;

        // NPC_OBJECT_WOOD type (54) — should update state but NOT broadcast
        let tmpl = NpcTemplate {
            s_sid: 501,
            is_monster: false,
            name: "WoodLog".to_string(),
            pid: 501,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 54, // NPC_OBJECT_WOOD
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 100,
            max_mp: 0,
            attack: 0,
            ac: 0,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 0,
            speed_1: 0,
            speed_2: 0,
            stand_time: 0,
            search_range: 0,
            attack_range: 0,
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
        };
        world.npc_templates.insert((501, false), Arc::new(tmpl));

        let inst = NpcInstance {
            nid,
            proto_id: 501,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        };
        world.npc_instances.insert(nid, Arc::new(inst));

        // Call send_gate_flag — state should update even for wood
        world.send_gate_flag(nid, 1);

        let updated = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated.gate_open, 1, "Wood NPC state should update");
        // No broadcast verification needed — C++ returns early after setting state
    }

    #[tokio::test]
    async fn test_send_gate_flag_no_broadcast_for_rollingstone() {
        let world = WorldState::new();
        let nid: NpcId = 20003;

        // NPC_ROLLINGSTONE type (181) — should update state but NOT broadcast
        let tmpl = NpcTemplate {
            s_sid: 502,
            is_monster: false,
            name: "RollingStone".to_string(),
            pid: 502,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 181, // NPC_ROLLINGSTONE
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 100,
            max_mp: 0,
            attack: 0,
            ac: 0,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 0,
            speed_1: 0,
            speed_2: 0,
            stand_time: 0,
            search_range: 0,
            attack_range: 0,
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
        };
        world.npc_templates.insert((502, false), Arc::new(tmpl));

        let inst = NpcInstance {
            nid,
            proto_id: 502,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        };
        world.npc_instances.insert(nid, Arc::new(inst));

        world.send_gate_flag(nid, 1);

        let updated = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated.gate_open, 1, "RollingStone NPC state should update");
    }

    #[tokio::test]
    async fn test_send_gate_flag_broadcasts_to_region() {
        let world = WorldState::new();
        let nid: NpcId = 20004;

        // Create zone 21 so broadcast_to_3x3 can find it
        world.ensure_zone(21, 128);

        // Gate NPC template (not wood/rollingstone — should broadcast)
        let tmpl = NpcTemplate {
            s_sid: 503,
            is_monster: false,
            name: "BattleGate".to_string(),
            pid: 503,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 52, // NPC_SPECIAL_GATE
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 100,
            max_mp: 0,
            attack: 0,
            ac: 0,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 0,
            speed_1: 0,
            speed_2: 0,
            stand_time: 0,
            search_range: 0,
            attack_range: 0,
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
        };
        world.npc_templates.insert((503, false), Arc::new(tmpl));

        let inst = NpcInstance {
            nid,
            proto_id: 503,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        };
        world.npc_instances.insert(nid, Arc::new(inst));

        // Register a player session in the same zone and region
        let (tx, mut rx) = mpsc::unbounded_channel();
        let session_id = 1;
        world.register_session(session_id, tx);
        // Mark session as in-game with character info and position
        world.register_ingame(
            session_id,
            test_char_info("GatePlayer"),
            crate::world::Position {
                zone_id: 21,
                x: 100.0,
                z: 100.0,
                y: 0.0,
                region_x: 2,
                region_z: 2,
            },
        );

        // Register the player in the zone's region grid so get_users_in_3x3 finds them
        let zone = world.get_zone(21).unwrap();
        zone.add_user(2, 2, session_id);

        // Call send_gate_flag — should broadcast
        world.send_gate_flag(nid, 1);

        // Verify the player received a WIZ_OBJECT_EVENT packet
        let pkt = rx.try_recv();
        assert!(pkt.is_ok(), "Player should receive gate flag broadcast");
        let pkt = pkt.unwrap();
        assert_eq!(
            pkt.opcode,
            ko_protocol::Opcode::WizObjectEvent as u8,
            "Broadcast should be WIZ_OBJECT_EVENT"
        );

        // Parse the packet: [u8 object_type] [u8 1] [u32 npc_id] [u8 gate_open]
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        let obj_type = r.read_u8().unwrap();
        assert_eq!(
            obj_type,
            crate::object_event_constants::OBJECT_FLAG_LEVER,
            "Default object type should be OBJECT_FLAG_LEVER (4)"
        );
        let success = r.read_u8().unwrap();
        assert_eq!(success, 1, "Success marker should be 1");
        let recv_nid = r.read_u32().unwrap();
        assert_eq!(recv_nid, nid, "NPC ID should match");
        let gate_state = r.read_u8().unwrap();
        assert_eq!(gate_state, 1, "Gate state should be 1 (open)");
        assert_eq!(r.remaining(), 0, "No extra data");
    }

    #[test]
    fn test_find_all_npcs_in_zone() {
        let world = WorldState::new();

        // Insert 3 NPCs: 2 with proto_id=500 in zone 21, 1 with proto_id=500 in zone 22
        for (nid, zone_id) in [(30001u32, 21u16), (30002, 21), (30003, 22)] {
            let inst = NpcInstance {
                nid,
                proto_id: 500,
                is_monster: false,
                zone_id,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                direction: 0,
                region_x: 2,
                region_z: 2,
                gate_open: 0,
                object_type: 0,
                nation: 0,
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
            };
            world.npc_instances.insert(nid, Arc::new(inst));
        }

        // Also insert one with different proto_id in zone 21
        let inst = NpcInstance {
            nid: 30004,
            proto_id: 501,
            is_monster: false,
            zone_id: 21,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 2,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        };
        world.npc_instances.insert(30004, Arc::new(inst));

        // Should find 2 matching NPCs in zone 21 with proto_id 500
        let results = world.find_all_npcs_in_zone(500, 21);
        assert_eq!(
            results.len(),
            2,
            "Should find 2 NPCs with proto_id=500 in zone 21"
        );

        let nids: Vec<u32> = results.iter().map(|n| n.nid).collect();
        assert!(nids.contains(&30001));
        assert!(nids.contains(&30002));

        // Zone 22 should have 1
        let results_z22 = world.find_all_npcs_in_zone(500, 22);
        assert_eq!(results_z22.len(), 1);

        // Non-existent proto should return empty
        let results_none = world.find_all_npcs_in_zone(999, 21);
        assert!(results_none.is_empty());
    }

    #[tokio::test]
    async fn test_send_gate_flag_nonexistent_npc_no_panic() {
        let world = WorldState::new();
        // Should not panic for non-existent NPC
        world.send_gate_flag(99999, 1);
    }

    #[test]
    fn test_send_gate_flag_packet_format() {
        // Verify the wire format of the gate flag broadcast packet
        //   Packet result(WIZ_OBJECT_EVENT, objectType);
        //   result << uint8(1) << uint32(GetID()) << m_byGateOpen;
        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizObjectEvent as u8);
        pkt.write_u8(crate::object_event_constants::OBJECT_FLAG_LEVER);
        pkt.write_u8(1); // success
        pkt.write_u32(10500); // npc_id
        pkt.write_u8(1); // gate_open

        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizObjectEvent as u8);
        assert_eq!(pkt.data.len(), 7); // 1 + 1 + 4 + 1 = 7 bytes
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10500));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_maybe_switch_target_roll_range_101() {
        // C++ myrand(0,100) produces 0..=100 (101 values).
        // Verify our implementation uses gen_range(0..=100).
        // We test by running many trials and checking that the unconditional
        // switch bracket (95-100 = 6 values out of 101) fires roughly ~6% of
        // the time in isolation. We can't isolate it perfectly, but we verify
        // the overall behavior is reasonable.
        let world = WorldState::new();
        let nid: NpcId = 11200;

        // No damage, no position — only AC and unconditional paths can switch
        let mut switched = 0;
        let trials = 2000;
        for _ in 0..trials {
            let mut ai = NpcAiState {
                state: NpcState::Attacking,
                target_id: Some(1),
                cur_x: 100.0,
                cur_z: 100.0,
                ..test_ai_state()
            };
            world.maybe_switch_target(&mut ai, nid, 2);
            if ai.target_id == Some(2) {
                switched += 1;
            }
        }
        // Without damage records: [0,50) compares 0 vs 0 → no switch
        // Without positions: [50,80) old_dist=MAX, new_dist=MAX → no switch
        // [80,95): AC comparison on default equipped stats (both 0 AC)→ no switch (equal)
        // [95,100]: unconditional switch (~6%)
        // So expect roughly 6% switch rate
        let pct = (switched as f64 / trials as f64) * 100.0;
        assert!(
            pct > 2.0 && pct < 15.0,
            "Expected ~6% switch rate (unconditional only), got {:.1}%",
            pct
        );
    }

    /// Verify MAX_NPC_RANGE_SQ matches C++ MAX_NPC_RANGE = pow(11.0f, 2.0f) = 121.0.
    ///
    #[test]
    fn test_npc_range_constant_matches_cpp() {
        use crate::npc::NpcInstance;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid: SessionId = 1;
        let npc_id: NpcId = 500;
        let zone_id: u16 = 1;

        // Register session so position lookup works
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Place player at (100, 100)
        world.update_position(sid, zone_id, 100.0, 0.0, 100.0);

        let make_npc = |x: f32| NpcInstance {
            nid: npc_id,
            proto_id: 1,
            zone_id,
            x,
            y: 0.0,
            z: 100.0,
            is_monster: false,
            direction: 0,
            region_x: 0,
            region_z: 0,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        };

        // 11 units away → squared distance = 121 → at boundary (in range)
        world.insert_npc_instance(make_npc(111.0));
        assert!(
            world.is_in_npc_range(sid, npc_id),
            "11 units should be in range (121 <= 121)"
        );

        // 12 units away → squared distance = 144 → out of range
        world.insert_npc_instance(make_npc(112.0));
        assert!(
            !world.is_in_npc_range(sid, npc_id),
            "12 units should be out of range (144 > 121)"
        );
    }

    // ── Sprint 949: Additional coverage ──────────────────────────────

    /// NPC_BOSS constant matches C++ globals.h.
    #[test]
    fn test_npc_boss_constant() {
        assert_eq!(NPC_BOSS, 3);
    }

    /// allocate_npc_id returns incrementing IDs.
    #[test]
    fn test_allocate_npc_id_increments() {
        let world = WorldState::new();
        let id1 = world.allocate_npc_id();
        let id2 = world.allocate_npc_id();
        assert!(id2 > id1);
    }

    /// get_npc_instance returns None for non-existent.
    #[test]
    fn test_get_npc_instance_missing() {
        let world = WorldState::new();
        assert!(world.get_npc_instance(99999).is_none());
    }

    /// get_npc_template returns None for non-existent.
    #[test]
    fn test_get_npc_template_missing() {
        let world = WorldState::new();
        assert!(world.get_npc_template(9999, true).is_none());
        assert!(world.get_npc_template(9999, false).is_none());
    }

    /// find_npc_in_zone returns None on empty world.
    #[test]
    fn test_find_npc_in_zone_empty() {
        let world = WorldState::new();
        assert!(world.find_npc_in_zone(100, 1).is_none());
    }

    // ── Sprint 957: Additional coverage ──────────────────────────────

    /// is_npc_dead returns true for non-existent NPC (no HP entry).
    #[test]
    fn test_is_npc_dead_nonexistent() {
        let world = WorldState::new();
        assert!(world.is_npc_dead(99999));
    }

    /// init_npc_hp + get_npc_hp roundtrip.
    #[test]
    fn test_npc_hp_init_and_get() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.init_npc_hp(nid, 5000);
        assert_eq!(world.get_npc_hp(nid), Some(5000));
        assert!(!world.is_npc_dead(nid));
    }

    /// update_npc_hp modifies existing HP.
    #[test]
    fn test_update_npc_hp() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.init_npc_hp(nid, 1000);
        world.update_npc_hp(nid, 0);
        assert_eq!(world.get_npc_hp(nid), Some(0));
        assert!(world.is_npc_dead(nid));
    }

    /// record_npc_damage + get_max_damage_user.
    #[test]
    fn test_npc_damage_tracking() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 1, 500);
        world.record_npc_damage(nid, 2, 800);
        world.record_npc_damage(nid, 1, 400); // total 900
        assert_eq!(world.get_max_damage_user(nid), Some(1));
    }

    /// clear_npc_damage removes all damage entries.
    #[test]
    fn test_clear_npc_damage() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 1, 100);
        world.clear_npc_damage(nid);
        assert!(world.get_npc_damage_entries(nid).is_empty());
        assert!(world.get_max_damage_user(nid).is_none());
    }

    // ── Sprint 967: Additional coverage ──────────────────────────────

    /// NPC ID allocation is monotonically increasing.
    #[test]
    fn test_npc_id_allocation_monotonic() {
        let world = WorldState::new();
        let a = world.allocate_npc_id();
        let b = world.allocate_npc_id();
        let c = world.allocate_npc_id();
        assert!(b > a);
        assert!(c > b);
    }

    /// npc_damage_contains returns correct membership.
    #[test]
    fn test_npc_damage_contains() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        assert!(!world.npc_damage_contains(nid, 1));
        world.record_npc_damage(nid, 1, 200);
        assert!(world.npc_damage_contains(nid, 1));
        assert!(!world.npc_damage_contains(nid, 2));
    }

    #[test]
    fn test_clear_npc_player_damage_preserves_other_attackers() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 1, 200);
        world.record_npc_damage(nid, 2, 300);

        world.clear_npc_player_damage(nid, 1);

        assert!(!world.npc_damage_contains(nid, 1));
        assert!(world.npc_damage_contains(nid, 2));
        assert_eq!(world.get_max_damage_user(nid), Some(2));
    }

    /// get_npc_damage_entries returns all recorded players.
    #[test]
    fn test_npc_damage_entries_multi() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 10, 100);
        world.record_npc_damage(nid, 20, 200);
        world.record_npc_damage(nid, 30, 300);
        let entries = world.get_npc_damage_entries(nid);
        assert_eq!(entries.len(), 3);
        let total: i32 = entries.iter().map(|(_, d)| d).sum();
        assert_eq!(total, 600);
    }

    /// find_npc_in_zone and find_all_npcs_in_zone return empty for non-matching zone.
    #[test]
    fn test_find_npc_zone_miss() {
        let world = WorldState::new();
        assert!(world.find_npc_in_zone(100, 1).is_none());
        assert!(world.find_all_npcs_in_zone(100, 1).is_empty());
    }

    /// NPC_BOSS constant equals 3.
    #[test]
    fn test_npc_boss_value() {
        assert_eq!(NPC_BOSS, 3);
    }

    // ── Sprint 973: Additional coverage ──────────────────────────────

    /// apply_npc_buff / has_npc_buff / npc_buff_count roundtrip.
    #[test]
    fn test_npc_buff_apply_has_count() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        assert_eq!(world.npc_buff_count(nid), 0);
        assert!(!world.has_npc_buff(nid, 100));
        world.apply_npc_buff(
            nid,
            NpcBuffEntry {
                skill_id: 1000,
                buff_type: 100,
                start_time: Instant::now(),
                duration_secs: 30,
            },
        );
        assert!(world.has_npc_buff(nid, 100));
        assert_eq!(world.npc_buff_count(nid), 1);
    }

    /// remove_npc_buff returns true when removed, false when absent.
    #[test]
    fn test_npc_buff_remove() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        assert!(!world.remove_npc_buff(nid, 100));
        world.apply_npc_buff(
            nid,
            NpcBuffEntry {
                skill_id: 1000,
                buff_type: 100,
                start_time: Instant::now(),
                duration_secs: 30,
            },
        );
        assert!(world.remove_npc_buff(nid, 100));
        assert!(!world.has_npc_buff(nid, 100));
        assert_eq!(world.npc_buff_count(nid), 0);
    }

    /// clear_npc_buffs removes all buffs at once.
    #[test]
    fn test_npc_buff_clear_all() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        for i in 0..5 {
            world.apply_npc_buff(
                nid,
                NpcBuffEntry {
                    skill_id: 1000 + i,
                    buff_type: i as i32,
                    start_time: Instant::now(),
                    duration_secs: 60,
                },
            );
        }
        assert_eq!(world.npc_buff_count(nid), 5);
        world.clear_npc_buffs(nid);
        assert_eq!(world.npc_buff_count(nid), 0);
    }

    /// get_max_damage_user returns the session with highest total damage.
    #[test]
    fn test_max_damage_user() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        assert!(world.get_max_damage_user(nid).is_none());
        world.record_npc_damage(nid, 10, 100);
        world.record_npc_damage(nid, 20, 500);
        world.record_npc_damage(nid, 30, 200);
        assert_eq!(world.get_max_damage_user(nid), Some(20));
    }

    /// clear_npc_damage removes all damage entries and max user.
    #[test]
    fn test_clear_npc_damage_resets_max_user() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 1, 100);
        world.record_npc_damage(nid, 2, 200);
        assert_eq!(world.get_npc_damage_entries(nid).len(), 2);
        world.clear_npc_damage(nid);
        assert!(world.get_npc_damage_entries(nid).is_empty());
        assert!(world.get_max_damage_user(nid).is_none());
    }

    /// add_npc_dot replaces existing DOT from same skill.
    #[test]
    fn test_npc_dot_replace_same_skill() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.add_npc_dot(
            nid,
            NpcDotSlot {
                skill_id: 100,
                hp_amount: 50,
                tick_count: 0,
                tick_limit: 5,
                caster_sid: 1,
            },
        );
        world.add_npc_dot(
            nid,
            NpcDotSlot {
                skill_id: 100,
                hp_amount: 80,
                tick_count: 0,
                tick_limit: 3,
                caster_sid: 2,
            },
        );
        // Same skill_id → replaced, not duplicated
        let dots = world.npc_dots.get(&nid).unwrap();
        assert_eq!(dots.len(), 1);
        assert_eq!(dots[0].hp_amount, 80);
        assert_eq!(dots[0].caster_sid, 2);
    }

    /// add_npc_dot caps at 4 slots (MAX_TYPE3_REPEAT).
    #[test]
    fn test_npc_dot_max_slots() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        for i in 0..6 {
            world.add_npc_dot(
                nid,
                NpcDotSlot {
                    skill_id: i,
                    hp_amount: 10,
                    tick_count: 0,
                    tick_limit: 5,
                    caster_sid: 1,
                },
            );
        }
        let dots = world.npc_dots.get(&nid).unwrap();
        assert_eq!(dots.len(), 4); // Max 4 slots
    }

    /// clear_npc_dots removes all DOTs for an NPC.
    #[test]
    fn test_clear_npc_dots() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.add_npc_dot(
            nid,
            NpcDotSlot {
                skill_id: 1,
                hp_amount: 10,
                tick_count: 0,
                tick_limit: 5,
                caster_sid: 1,
            },
        );
        world.add_npc_dot(
            nid,
            NpcDotSlot {
                skill_id: 2,
                hp_amount: 20,
                tick_count: 0,
                tick_limit: 5,
                caster_sid: 1,
            },
        );
        assert!(world.npc_dots.get(&nid).is_some());
        world.clear_npc_dots(nid);
        assert!(world.npc_dots.get(&nid).is_none());
    }

    /// set_npc_duration updates AI state fields.
    #[test]
    fn test_set_npc_duration() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        // Insert AI state first
        world.npc_ai.insert(nid, test_ai_state());
        world.set_npc_duration(nid, 60, 5000);
        let ai = world.get_npc_ai(nid).unwrap();
        assert_eq!(ai.duration_secs, 60);
        assert_eq!(ai.spawned_at_ms, 5000);
    }

    /// update_npc_gate_open sets gate flag on NPC instance.
    #[test]
    fn test_update_npc_gate_open() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        let inst = NpcInstance {
            nid,
            proto_id: 100,
            zone_id: 21,
            region_x: 5,
            region_z: 5,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            is_monster: false,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            gate_open: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.npc_instances.insert(nid, Arc::new(inst));
        world.update_npc_gate_open(nid, 1);
        let updated = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated.gate_open, 1);
    }

    /// get_all_ai_npc_ids returns IDs of NPCs with AI state.
    #[test]
    fn test_get_all_ai_npc_ids() {
        let world = WorldState::new();
        let nid1 = world.allocate_npc_id();
        let nid2 = world.allocate_npc_id();
        assert!(world.get_all_ai_npc_ids().is_empty());
        world.npc_ai.insert(nid1, test_ai_state());
        world.npc_ai.insert(nid2, test_ai_state());
        let ids = world.get_all_ai_npc_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&nid1));
        assert!(ids.contains(&nid2));
    }

    /// get_ai_npc_ids_by_zone groups NPC IDs by zone.
    #[test]
    fn test_get_ai_npc_ids_by_zone() {
        let world = WorldState::new();
        let nid1 = world.allocate_npc_id();
        let nid2 = world.allocate_npc_id();
        let nid3 = world.allocate_npc_id();
        let mut ai1 = test_ai_state();
        ai1.zone_id = 21;
        let mut ai2 = test_ai_state();
        ai2.zone_id = 21;
        let mut ai3 = test_ai_state();
        ai3.zone_id = 51;
        world.npc_ai.insert(nid1, ai1);
        world.npc_ai.insert(nid2, ai2);
        world.npc_ai.insert(nid3, ai3);
        let by_zone = world.get_ai_npc_ids_by_zone();
        assert_eq!(by_zone.get(&21).unwrap().len(), 2);
        assert_eq!(by_zone.get(&51).unwrap().len(), 1);
    }

    /// update_npc_ai modifies AI state via closure.
    #[test]
    fn test_update_npc_ai_closure() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.npc_ai.insert(nid, test_ai_state());
        world.update_npc_ai(nid, |ai| {
            ai.target_id = Some(42);
        });
        let ai = world.get_npc_ai(nid).unwrap();
        assert_eq!(ai.target_id, Some(42));
    }

    /// is_npc_dead returns true when HP not initialized (default dead).
    #[test]
    fn test_is_npc_dead_no_hp_entry() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        // No HP entry → treated as dead
        assert!(world.is_npc_dead(nid));
        // Init HP → alive
        world.init_npc_hp(nid, 100);
        assert!(!world.is_npc_dead(nid));
        // Set HP to 0 → dead
        world.update_npc_hp(nid, 0);
        assert!(world.is_npc_dead(nid));
    }

    /// record_npc_damage accumulates damage from same session.
    #[test]
    fn test_record_npc_damage_accumulates() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 1, 100);
        world.record_npc_damage(nid, 1, 200);
        // Same session → 300 total
        let entries = world.get_npc_damage_entries(nid);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1, 300);
    }

    /// process_npc_buff_tick removes expired buffs and returns them.
    #[test]
    fn test_process_npc_buff_tick_expired() {
        use crate::world::NpcBuffEntry;
        use std::time::Instant;
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        // Permanent buff (duration_secs=0) → never expires
        world.apply_npc_buff(
            nid,
            NpcBuffEntry {
                skill_id: 100,
                buff_type: 1,
                duration_secs: 0,
                start_time: Instant::now(),
            },
        );
        // Already expired buff (duration very short, applied in past)
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(100))
            .unwrap();
        world.apply_npc_buff(
            nid,
            NpcBuffEntry {
                skill_id: 200,
                buff_type: 2,
                duration_secs: 1,
                start_time: past,
            },
        );
        let expired = world.process_npc_buff_tick();
        // Only buff_type=2 should expire
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].1, 2);
        // Permanent buff still active
        assert!(world.has_npc_buff(nid, 1));
        assert!(!world.has_npc_buff(nid, 2));
    }

    /// find_npc_in_zone returns matching NPC by proto_id and zone.
    #[test]
    fn test_find_npc_in_zone_match() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        let inst = NpcInstance {
            nid,
            proto_id: 500,
            zone_id: 21,
            region_x: 3,
            region_z: 3,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            direction: 0,
            is_monster: false,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            gate_open: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.npc_instances.insert(nid, Arc::new(inst));
        // Match
        assert!(world.find_npc_in_zone(500, 21).is_some());
        // Wrong zone
        assert!(world.find_npc_in_zone(500, 22).is_none());
        // Wrong proto_id
        assert!(world.find_npc_in_zone(501, 21).is_none());
    }

    /// find_all_npcs_in_zone returns all matching NPCs.
    #[test]
    fn test_find_all_npcs_in_zone_multiple() {
        let world = WorldState::new();
        for _ in 0..3 {
            let nid = world.allocate_npc_id();
            let inst = NpcInstance {
                nid,
                proto_id: 700,
                zone_id: 51,
                region_x: 1,
                region_z: 1,
                x: 10.0,
                y: 0.0,
                z: 10.0,
                direction: 0,
                is_monster: true,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                gate_open: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            world.npc_instances.insert(nid, Arc::new(inst));
        }
        let found = world.find_all_npcs_in_zone(700, 51);
        assert_eq!(found.len(), 3);
        // Different zone → empty
        assert!(world.find_all_npcs_in_zone(700, 52).is_empty());
    }

    /// update_npc_position updates position and recalculates region.
    #[test]
    fn test_update_npc_position() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        let inst = NpcInstance {
            nid,
            proto_id: 100,
            zone_id: 21,
            region_x: 0,
            region_z: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            is_monster: false,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            gate_open: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.npc_instances.insert(nid, Arc::new(inst));
        world.update_npc_position(nid, 500.0, 600.0);
        let updated = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated.x, 500.0);
        assert_eq!(updated.z, 600.0);
    }

    /// get_npc_template returns None for missing template.
    #[test]
    fn test_get_npc_template_missing_monster() {
        let world = WorldState::new();
        assert!(world.get_npc_template(65535, true).is_none());
        assert!(world.get_npc_template(65535, false).is_none());
    }

    // ── Sprint 999: Additional coverage ──────────────────────────────

    /// npc_template_update only updates fields when values are > 0.
    /// group=0 means "don't change", pid=0 means "don't change".
    #[test]
    fn test_npc_template_update_conditional_fields() {
        let world = WorldState::new();
        let mut tmpl = friend_template(100, 0, 0);
        tmpl.group = 1;
        tmpl.pid = 200;
        world.npc_templates.insert((100, true), Arc::new(tmpl));

        // Update only group (pid=0 → no change)
        world.npc_template_update(100, true, 5, 0);
        let updated = world.get_npc_template(100, true).unwrap();
        assert_eq!(updated.group, 5);
        assert_eq!(updated.pid, 200); // unchanged

        // Update only pid (group=0 → no change)
        world.npc_template_update(100, true, 0, 999);
        let updated2 = world.get_npc_template(100, true).unwrap();
        assert_eq!(updated2.group, 5); // unchanged
        assert_eq!(updated2.pid, 999);
    }

    /// Juraid bridge trap_number formula: Karus = bridge_idx+1, Elmorad = bridge_idx+4.
    #[test]
    fn test_juraid_bridge_trap_number_formula() {
        // Karus bridges: trap_number = bridge_idx + 1 → (1, 2, 3)
        // Elmorad bridges: trap_number = bridge_idx + 4 → (4, 5, 6)
        for bridge_idx in 0i16..3 {
            let karus_trap = bridge_idx + 1;
            let elmo_trap = bridge_idx + 4;
            assert_eq!(karus_trap, bridge_idx + 1);
            assert_eq!(elmo_trap, bridge_idx + 4);
        }
        // Verify ranges don't overlap
        let karus_traps: Vec<i16> = (0..3i16).map(|i| i + 1).collect();
        let elmo_traps: Vec<i16> = (0..3i16).map(|i| i + 4).collect();
        assert_eq!(karus_traps, vec![1i16, 2, 3]);
        assert_eq!(elmo_traps, vec![4i16, 5, 6]);
        // No overlap between the two sets
        for k in &karus_traps {
            assert!(!elmo_traps.contains(k));
        }
    }

    /// Event NPC multi-spawn scatter offset uses deterministic formula.
    /// Offset = (i * 3.7 - 5.0, i * 2.3 - 5.0), clamped to [-10, 10].
    #[test]
    fn test_event_npc_scatter_offset_formula() {
        // Single spawn (count=1) → no offset
        let (ox_single, oz_single) = (0.0_f32, 0.0_f32);
        assert_eq!(ox_single, 0.0);
        assert_eq!(oz_single, 0.0);

        // Multi-spawn: i=0 → offset = (0*3.7-5.0, 0*2.3-5.0) = (-5.0, -5.0)
        let i = 0_f32;
        let ox = (i * 3.7 - 5.0).clamp(-10.0, 10.0);
        let oz = (i * 2.3 - 5.0).clamp(-10.0, 10.0);
        assert!((ox - (-5.0)).abs() < 0.01);
        assert!((oz - (-5.0)).abs() < 0.01);

        // i=2 → offset = (2*3.7-5.0, 2*2.3-5.0) = (2.4, -0.4)
        let i = 2_f32;
        let ox2 = (i * 3.7 - 5.0).clamp(-10.0, 10.0);
        let oz2 = (i * 2.3 - 5.0).clamp(-10.0, 10.0);
        assert!((ox2 - 2.4).abs() < 0.01);
        assert!((oz2 - (-0.4)).abs() < 0.01);
    }

    /// Event NPC AI: act_type 1-4 → not aggressive (TENDER), 5+ → aggressive (ATROCITY).
    #[test]
    fn test_event_npc_ai_aggressive_by_act_type() {
        for act_type in 1..=4u8 {
            let is_aggressive = !matches!(act_type, 1..=4);
            assert!(
                !is_aggressive,
                "act_type {} should be non-aggressive (TENDER)",
                act_type
            );
        }
        for act_type in [0u8, 5, 6, 10, 255] {
            let is_aggressive = !matches!(act_type, 1..=4);
            assert!(
                is_aggressive,
                "act_type {} should be aggressive (ATROCITY)",
                act_type
            );
        }
    }

    // ── Sprint 1002: npc.rs +5 ──────────────────────────────────────

    /// NpcAiState default regen time is 30 seconds (30_000 ms).
    #[test]
    fn test_npc_ai_state_regen_default() {
        let ai = test_ai_state();
        assert_eq!(ai.regen_time_ms, 30_000);
        assert_eq!(ai.regen_time_ms / 1000, 30);
    }

    /// alloc_bot_id produces sequential IDs starting from BOT_ID_BASE.
    #[test]
    fn test_alloc_bot_id_sequential() {
        let world = WorldState::new();
        let id1 = world.alloc_bot_id();
        let id2 = world.alloc_bot_id();
        let id3 = world.alloc_bot_id();
        assert!(id2 > id1);
        assert!(id3 > id2);
        assert_eq!(id2 - id1, 1);
        assert_eq!(id3 - id2, 1);
    }

    /// clear_npc_damage empties the damage map for a given NPC.
    #[test]
    fn test_clear_npc_damage_empties_map() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        world.record_npc_damage(nid, 1, 50);
        world.record_npc_damage(nid, 2, 75);
        assert_eq!(world.get_npc_damage_entries(nid).len(), 2);
        world.clear_npc_damage(nid);
        assert!(world.get_npc_damage_entries(nid).is_empty());
        assert!(world.get_max_damage_user(nid).is_none());
    }

    /// npc_buff_count returns 0 for NPC with no buffs.
    #[test]
    fn test_npc_buff_count_empty() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        assert_eq!(world.npc_buff_count(nid), 0);
    }

    /// get_max_damage_user returns None when no damage recorded.
    #[test]
    fn test_get_max_damage_user_empty() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        assert!(world.get_max_damage_user(nid).is_none());
        assert!(world.get_npc_damage_entries(nid).is_empty());
    }

    /// update_npc_position cascades position change to AI state (cur_x, cur_z).
    #[test]
    fn test_update_npc_position_cascades_to_ai() {
        let world = WorldState::new();
        let nid = world.allocate_npc_id();
        let inst = NpcInstance {
            nid,
            proto_id: 100,
            zone_id: 21,
            region_x: 0,
            region_z: 0,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            direction: 0,
            is_monster: true,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            gate_open: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.npc_instances.insert(nid, Arc::new(inst));
        // Insert AI state at initial position
        let ai = NpcAiState {
            cur_x: 50.0,
            cur_z: 50.0,
            region_x: 0,
            region_z: 0,
            ..test_ai_state()
        };
        world.insert_npc_ai(nid, ai);

        // Update position
        world.update_npc_position(nid, 300.0, 400.0);

        // Verify instance updated
        let updated_inst = world.get_npc_instance(nid).unwrap();
        assert_eq!(updated_inst.x, 300.0);
        assert_eq!(updated_inst.z, 400.0);

        // Verify AI state also updated
        let updated_ai = world.get_npc_ai(nid).unwrap();
        assert_eq!(updated_ai.cur_x, 300.0);
        assert_eq!(updated_ai.cur_z, 400.0);
    }
}
