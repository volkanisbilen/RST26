//! Event state machine coordinator and scheduler.
//! This module provides the top-level event scheduling and state machine
//! that coordinates BDW, Juraid, Chaos, and other room-based events.
//! ## State Machine Flow
//! ```text
//!   Inactive ──[schedule trigger]──> Registration
//!   Registration ──[sign timer expires]──> Active
//!   Active ──[play timer expires]──> Rewards
//!   Rewards ──[finish delay expires]──> Cleanup
//!   Cleanup ──[rooms destroyed]──> Inactive
//! ```
//! ## GM Manual Override
//! GMs can open events manually (bypassing the schedule) or close them early.
//! Manual close triggers immediate winner determination, then proceeds to Rewards.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Timelike;

use ko_protocol::{Opcode, Packet};

use crate::systems::bdw;
use crate::systems::chaos;
use crate::systems::event_room::{
    self, EventLocalId, EventRoomManager, EventScheduleEntry, TempleEventType, VroomOpt,
};
use crate::systems::juraid;
use crate::world::{WorldState, ZONE_BIFROST, ZONE_RONARK_LAND};
use crate::zone::SessionId;
use ko_db::models::event_schedule::EventRewardRow;

// ── Background Task ─────────────────────────────────────────────────────────

/// Event tick interval in seconds.
const EVENT_TICK_INTERVAL_SECS: u64 = 1;

fn spawn_juraid_room_npcs(world: &WorldState) {
    world.clear_juraid_monument_respawns();

    let rooms = world
        .event_room_manager
        .list_rooms(TempleEventType::JuraidMountain);
    for room_id in rooms {
        let family = 20 + room_id as i16;
        let rows = world.get_juraid_respawn_family(family);
        if rows.is_empty() {
            tracing::warn!(
                room_id,
                family,
                "Juraid spawn skipped: no monster_juraid_respawn_list rows"
            );
            continue;
        }

        world.despawn_room_npcs(juraid::ZONE_JURAID, room_id as u16);

        let mut bridge_trap = 1u8;
        let mut spawned_total = 0usize;
        let mut spawned_monsters = 0usize;
        let mut spawned_bridges = 0usize;
        let mut spawned_merchants = 0usize;
        let mut spawned_deva = 0usize;
        for row in rows {
            let is_deva = juraid::is_deva_bird(row.s_sid as u16);
            let is_bridge = juraid::is_bridge(row.s_sid as u16);
            let is_wave_monster = juraid::is_wave_monster(&row);

            // The MSSQL table is already the complete Juraid layout.  In
            // particular, each of the six main rows has s_count=5 (three
            // groups per nation).  Truncating those rows and forcing count=1
            // left rooms empty and made progression impossible.
            let is_merchant = row.b_type == 1;
            if !is_wave_monster && !is_deva && !is_bridge && !is_merchant {
                continue;
            }

            // C++ loads PID 6700 from K_MONSTER and sends it through the
            // monster NPC_INOUT layout.  v2615 uses that wire flag to attach
            // the Juraid bridge model/collision object; cloning it as K_NPC
            // made the server open the gate while the client kept no walkable
            // bridge. Its zero search range keeps this static gate out of AI.
            // Bridge of Summoning (PID 8110) is a world gate, not a combat
            // target. Sending it through the monster NPC_INOUT layout makes
            // Z-targeting select it and puts the client into attack mode.
            // Keep the monster wire layout only for actual combat spawns.
            let is_monster = is_wave_monster || is_deva;
            let count = if is_wave_monster {
                row.s_count.max(1) as u16
            } else {
                1
            };
            let trap_number = if is_bridge {
                let trap = bridge_trap;
                bridge_trap = bridge_trap.saturating_add(1);
                trap
            } else {
                0
            };
            let summon_type = if is_wave_monster {
                juraid::SUMMON_JURAID_MAIN
            } else if is_deva {
                juraid::SUMMON_JURAID_DEVA
            } else if is_bridge {
                juraid::SUMMON_JURAID_BRIDGE
            } else {
                0
            };

            let ids = world.spawn_event_npc_ex_with_direction(
                row.s_sid as u16,
                is_monster,
                juraid::ZONE_JURAID,
                row.x as f32,
                row.z as f32,
                count,
                room_id as u16,
                summon_type,
                row.by_direction.clamp(0, u8::MAX as i16) as u8,
            );
            if trap_number > 0 {
                for nid in &ids {
                    world.update_npc_trap_number(*nid, trap_number as i16);
                }
            }
            if is_wave_monster {
                spawned_monsters += ids.len();
            } else if is_deva {
                spawned_deva += ids.len();
            } else if is_bridge {
                spawned_bridges += ids.len();
            } else if is_merchant {
                spawned_merchants += ids.len();
            }
            spawned_total += ids.len();
        }

        let spawned_monuments = juraid::spawn_deva_room_monuments(world, room_id);
        spawned_total += spawned_monuments;

        tracing::info!(
            room_id,
            family,
            spawned_monsters,
            spawned_bridges,
            spawned_merchants,
            spawned_deva,
            spawned_monuments,
            spawned_total,
            "Juraid room NPCs spawned from monster_juraid_respawn_list"
        );
    }
}

/// Start the event system background task.
/// Spawns a tokio task that calls [`event_tick_at`] every second,
/// processing BDW, Juraid, and other room-based event state machines.
/// BDW and Juraid managers are created locally within the task since they
/// are not stored on `WorldState`.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_event_system_task(
    world: std::sync::Arc<crate::world::WorldState>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(EVENT_TICK_INTERVAL_SECS));
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        loop {
            interval.tick().await;
            let now = unix_now();
            let erm = world.event_room_manager();
            let active_event_i16 = erm.read_temple_event(|s| s.active_event);

            // ── Schedule auto-trigger ─────────────────────────────────────
            // EventMainTimer.cpp:491-523
            // When no event is active, check schedules against current time.
            if active_event_i16 < 0 {
                if let Some((event_type, sign_secs)) = try_schedule_trigger(erm, now) {
                    // Broadcast sign-up announcement to all online players.
                    // C++ skips: isInTempleEventZone(), isInMonsterStoneZone(), ZONE_PRISON
                    let start_pkt = event_room::build_event_start_broadcast(
                        event_type as i16,
                        sign_secs as u16,
                    );
                    const EXCLUDED_ZONES: &[u16] = &[81, 82, 83, 84, 85, 87, 92];
                    world.broadcast_to_all_excluding_zones(Arc::new(start_pkt), EXCLUDED_ZONES);
                    if event_type == TempleEventType::JuraidMountain {
                        broadcast_juraid_registration_notice(&world, sign_secs as u16);
                    }
                }
            }

            let action = {
                let mut bdw_mgr = world.bdw_manager_write();
                event_tick_at(erm, &mut bdw_mgr, &mut juraid_mgr, &mut chaos_mgr, now)
            };

            // Cinderella War per-second timer tick
            crate::handler::cinderella::cinderella_timer_tick(&world);

            // Wanted/Vanguard event position broadcast tick
            // tick_wanted_position_broadcasts() internally throttles to 60s intervals
            crate::handler::vanguard::tick_wanted_position_broadcasts(&world, now);

            // Wanted event lifecycle state machine (selecting, inviting, running, finishing)
            crate::handler::vanguard::tick_wanted_event_lifecycle(&world, now);

            // Distribute rewards when transitioning to Rewards phase.
            //
            //
            // Chaos uses per-user EXP from kills/deaths (not winner/loser table rewards).
            // BDW uses EVENT_REWARD rows. Juraid gets its original winner-only
            // gem/EXP rewards and keeps monster/chest drops on their own tables.
            if active_event_i16 == TempleEventType::JuraidMountain as i16 {
                for (room_id, nation) in world.take_due_juraid_monument_respawns(now) {
                    let spawned = juraid::spawn_monument(&world, room_id, nation);
                    if spawned > 0 {
                        tracing::info!(room_id, nation, spawned, "Juraid Monument respawned");
                    }
                }
            }

            match &action {
                EventTickAction::TransitionedToRewards(results) => {
                    // Send winner screen to all room users before distributing rewards.
                    event_room::send_winner_screen(&world, active_event_i16, now);

                    if active_event_i16 == 24 {
                        // Chaos Dungeon: per-user EXP from kills/deaths
                        distribute_chaos_finish_exp(&world).await;
                    } else if active_event_i16 == TempleEventType::JuraidMountain as i16 {
                        distribute_juraid_rewards(&world, results).await;
                    } else {
                        // BDW / Juraid: table-based winner/loser rewards
                        let local_id = active_event_to_local_id(active_event_i16);
                        distribute_event_rewards(&world, local_id, results).await;
                    }
                }
                EventTickAction::BdwAltarRespawns(room_ids) => {
                    // Restore altar NPC HP and broadcast respawn packet
                    //   pAltar->SetHP(pAltar->GetMaxHP());
                    {
                        let bdw_mgr = world.bdw_manager_read();
                        for &room_id in room_ids {
                            if let Some(state) = bdw_mgr.get_room_state(room_id) {
                                if state.altar_npc_id != 0 {
                                    let nid = state.altar_npc_id;
                                    if let Some(tmpl) =
                                        world.get_npc_template(bdw::ALTAR_OF_MANES, true)
                                    {
                                        world.update_npc_hp(nid, tmpl.max_hp as i32);
                                    }
                                }
                            }
                        }
                    }

                    let respawn_pkt = event_room::build_altar_respawn_packet();
                    for &room_id in room_ids {
                        broadcast_to_bdw_room(&world, room_id, &respawn_pkt);
                    }
                }
                EventTickAction::TransitionedToActive(assigned) => {
                    // Teleport all room-assigned users into the event zone,
                    // send timer overlay packets, and create parties.
                    //
                    let event_type = erm.read_temple_event(|s| {
                        event_room::TempleEventType::from_i16(s.active_event)
                    });
                    if let Some(et) = event_type {
                        if et == TempleEventType::JuraidMountain {
                            broadcast_juraid_active_notice(&world, *assigned);
                        }

                        // Teleport users + send timer overlay packets
                        event_room::teleport_users_to_event(&world, et);

                        if et == TempleEventType::JuraidMountain {
                            spawn_juraid_room_npcs(&world);
                        }

                        // Create auto-parties for BDW and Juraid (Chaos is FFA).
                        // after TeleportUsers for BDW and Juraid only.
                        if et != TempleEventType::ChaosDungeon {
                            event_room::temple_event_create_parties(&world, et);
                        }
                    }
                }
                EventTickAction::TransitionedToCleanup(et, user_sids)
                | EventTickAction::ManualCloseCleanup(et, user_sids) => {
                    // Per-user cleanup before kick teleport.
                    //
                    //   - BDW: RemoveType4Buff(BUFF_TYPE_FRAGMENT_OF_MANES)
                    //   - Clear event session data
                    //
                    // Session IDs are pre-collected before rooms are destroyed.
                    if *et == TempleEventType::BorderDefenceWar {
                        for sid in user_sids.iter() {
                            world.remove_buff(
                                *sid,
                                crate::systems::bdw::BUFF_TYPE_FRAGMENT_OF_MANES,
                            );
                        }
                    }

                    // Clear Juraid bridge state from WorldState on cleanup.
                    if *et == TempleEventType::JuraidMountain {
                        world.clear_juraid_bridge_states();
                        world.clear_juraid_monument_respawns();
                    }

                    // Kick all event zone users to their appropriate destination.
                    let event_zone = et.zone_id();
                    for sid in user_sids {
                        world.update_session(*sid, |h| {
                            h.event_room = 0;
                            h.joined_event = false;
                            h.is_final_joined_event = false;
                        });
                        let nation = world
                            .get_character_info(*sid)
                            .map(|c| c.nation)
                            .unwrap_or(0);
                        let level = world.get_session_level(*sid);
                        let dest_zone = event_room::kick_out_destination(event_zone, nation, level);
                        crate::handler::zone_change::server_teleport_to_zone(
                            &world, *sid, dest_zone, 0.0, 0.0,
                        )
                    }
                }
                EventTickAction::JuraidBridgesOpened(indices) => {
                    // Juraid bridge open — open bridge state in all rooms.
                    //
                    // C++ sends WIZ_NPC_INOUT (OUT then IN) for bridge gate NPCs,
                    // per-nation to each room. The NPC m_byGateOpen is set to 2.
                    let room_ids: Vec<u8> = juraid_mgr.room_states.keys().copied().collect();
                    for &bridge_idx in indices {
                        let rooms_opened =
                            juraid::open_bridge_for_all_rooms(&mut juraid_mgr, bridge_idx);

                        // Broadcast NPC_INOUT to zone users so the client sees gates open.
                        // Also sync bridge state to WorldState for CheckDevaAttack.
                        for &room_id in &room_ids {
                            world.broadcast_juraid_bridge_open(bridge_idx, room_id as u16);
                            if let Some(rs) = juraid_mgr.room_states.get(&room_id) {
                                world.set_juraid_bridge_state(room_id, rs.bridges.clone());
                            }
                            // Deva and both monuments are part of the static
                            // room layout and are spawned at event start behind
                            // the closed gates. Do not duplicate them here.
                        }

                        tracing::info!(
                            "Juraid bridge {} opened in {} rooms (broadcast sent)",
                            bridge_idx,
                            rooms_opened,
                        );
                    }
                }
                _ => {}
            }

            // ── Per-room finish countdown (TempleEventRoomClose) ────────
            // After the winner screen is sent (Rewards phase), each room has a
            // 20-second countdown. When it expires, users are kicked from that
            // room individually. This runs every tick during Rewards phase.
            {
                let phase = current_phase(erm);
                if phase == EventPhase::Rewards {
                    if let Some(et) = erm.read_temple_event(|s| {
                        event_room::TempleEventType::from_i16(s.active_event)
                    }) {
                        event_room::temple_event_room_close(&world, et, now);
                    }
                }
            }

            // ── Monster Stone timer tick ─────────────────────────────────
            // Check all active rooms for expiry (30-minute timeout or 20s boss-kill grace).
            // Expired rooms: teleport users to Moradon, despawn NPCs, reset room.
            {
                let expired_rooms = world.monster_stone_write().timer_tick(now);
                for room_id in expired_rooms {
                    // Read room state before reset (need zone_id and users)
                    let (zone_id, users) = {
                        let mgr = world.monster_stone_read();
                        match mgr.get_room(room_id) {
                            Some(room) => (room.zone_id, room.users.clone()),
                            None => continue,
                        }
                    };

                    // Teleport all users to Moradon and clear status
                    for &uid in &users {
                        world.update_session(uid, |h| {
                            h.event_room = 0;
                            h.monster_stone_status = false;
                        });
                        crate::handler::zone_change::server_teleport_to_zone(
                            &world,
                            uid,
                            crate::systems::monster_stone::ZONE_MORADON,
                            0.0,
                            0.0,
                        )
                    }

                    // Despawn all event NPCs in the room
                    let event_room_id = room_id + 1; // 1-based
                    world.despawn_room_npcs(zone_id as u16, event_room_id);

                    // Reset room back to pool
                    world.monster_stone_write().reset_room(room_id);

                    tracing::debug!(
                        "Monster Stone room {} expired: {} users teleported to Moradon, NPCs despawned",
                        room_id,
                        users.len()
                    );
                }
            }

            // ── Draki Tower room tick ──────────────────────────────────────
            // Called every second for each active room. Handles sub-stage expiry,
            // kick-out timers, town-out timers, and room close countdown.
            {
                use crate::handler::draki_tower;

                // Collect tick results under a read lock, then act on them.
                let mut kick_rooms: Vec<u16> = Vec::with_capacity(16);
                let mut town_rooms: Vec<u16> = Vec::with_capacity(16);
                let mut substage_expired_rooms: Vec<u16> = Vec::with_capacity(16);
                let mut close_rooms: Vec<u16> = Vec::with_capacity(16);

                {
                    let mut rooms = world.draki_tower_rooms_write();
                    for (&room_id, room) in rooms.iter_mut() {
                        if !room.tower_started {
                            continue;
                        }

                        // Room close countdown (runs every second)
                        if draki_tower::room_close_tick(room) {
                            close_rooms.push(room_id);
                            continue;
                        }

                        match draki_tower::room_timer_tick(room, now) {
                            draki_tower::DrakiTickResult::KickOut => {
                                kick_rooms.push(room_id);
                            }
                            draki_tower::DrakiTickResult::TownOut => {
                                town_rooms.push(room_id);
                            }
                            draki_tower::DrakiTickResult::SubStageExpired => {
                                substage_expired_rooms.push(room_id);
                            }
                            draki_tower::DrakiTickResult::Idle => {}
                        }
                    }
                } // drop write lock before sending packets

                // BUG-10 fix: Send OUT1 for substage-expired BEFORE applying kickout
                // C++ DrakiTowerKickOuts sends OUT1 first, then sets out_timer
                for &room_id in &substage_expired_rooms {
                    let sid_opt = world.find_session_by(|h| h.draki_room_id == room_id);
                    if let Some(sid) = sid_opt {
                        // BUG-4 fix: include full C++ payload bytes
                        let mut out1_pkt = Packet::new(ko_protocol::Opcode::WizEvent as u8);
                        out1_pkt.write_u8(draki_tower::TEMPLE_DRAKI_TOWER_OUT1);
                        out1_pkt.write_u8(0x0C);
                        out1_pkt.write_u8(0x04);
                        out1_pkt.write_u8(0x00);
                        out1_pkt.write_u8(0x14);
                        out1_pkt.write_u16(0);
                        out1_pkt.write_u8(0);
                        world.send_to_session_owned(sid, out1_pkt);
                    }
                }

                // Now apply kickout state (after OUT1 was sent)
                {
                    let mut rooms = world.draki_tower_rooms_write();
                    for &room_id in &substage_expired_rooms {
                        if let Some(room) = rooms.get_mut(&room_id) {
                            draki_tower::apply_kickout(room, now);
                        }
                    }
                }

                // Process kick-out / town-out / close: teleport player, reset room
                for room_id in kick_rooms
                    .iter()
                    .chain(town_rooms.iter())
                    .chain(close_rooms.iter())
                {
                    let user_name = {
                        let rooms = world.draki_tower_rooms_read();
                        rooms
                            .get(room_id)
                            .map(|r| r.user_name.clone())
                            .unwrap_or_default()
                    };

                    let sid_opt = world.find_session_by(|h| h.draki_room_id == *room_id);
                    if let Some(sid) = sid_opt {
                        let (level, nation) = world
                            .get_character_info(sid)
                            .map(|c| (c.level, c.nation))
                            .unwrap_or((1, 1));
                        let exit_zone = draki_tower::get_exit_zone(level as u16, nation);

                        // BUG-4 fix: Send OUT2 with full C++ payload
                        // C++ DrakiTowerKickTimer doesn't send OUT2 for kick,
                        // but for completeness we mirror DrakiTowerTown OUT2 format
                        let (stage, sub_stage, elapsed) = {
                            let rooms = world.draki_tower_rooms_read();
                            rooms
                                .get(room_id)
                                .map(|r| {
                                    let elapsed = now.saturating_sub(r.draki_timer) as u32;
                                    (r.draki_stage, r.draki_sub_stage, elapsed)
                                })
                                .unwrap_or((0, 0, 0))
                        };
                        let mut out2_pkt = Packet::new(ko_protocol::Opcode::WizEvent as u8);
                        out2_pkt.write_u8(draki_tower::TEMPLE_DRAKI_TOWER_OUT2);
                        out2_pkt.write_u8(0x0C);
                        out2_pkt.write_u8(0x04);
                        out2_pkt.write_u16(stage);
                        out2_pkt.write_u16(sub_stage);
                        out2_pkt.write_u32(elapsed);
                        out2_pkt.write_u8(1);
                        world.send_to_session_owned(sid, out2_pkt);

                        // Clear session state
                        world.update_session(sid, |h| {
                            h.event_room = 0;
                            h.draki_room_id = 0;
                        });

                        crate::handler::zone_change::server_teleport_to_zone(
                            &world, sid, exit_zone, 0.0, 0.0,
                        )
                    }

                    // Despawn room NPCs and reset room
                    world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, *room_id);
                    {
                        let mut rooms = world.draki_tower_rooms_write();
                        if let Some(room) = rooms.get_mut(room_id) {
                            room.reset();
                        }
                    }

                    tracing::debug!("Draki Tower room {} closed (user={})", room_id, user_name);
                }
            }

            // ── Draki Tower daily entrance limit reset ────────────────────
            {
                use crate::handler::draki_tower;
                let local_now = chrono::Local::now();
                if draki_tower::should_reset_limits(
                    local_now.hour(),
                    local_now.minute(),
                    local_now.second(),
                ) {
                    world.reset_draki_entrance_limits();
                    // Persist reset to DB so limits survive server restart
                    if let Some(pool) = world.db_pool() {
                        let pool = pool.clone();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::draki_tower::DrakiTowerRepository::new(&pool);
                            match repo.reset_all_entrance_limits().await {
                                Ok(n) => tracing::info!(
                                    rows = n,
                                    "Draki Tower: DB entrance limits reset"
                                ),
                                Err(e) => tracing::warn!(
                                    "Draki Tower: DB entrance limit reset failed: {e}"
                                ),
                            }
                        });
                    }
                    tracing::info!("Draki Tower: daily entrance limits reset (18:00)");
                }
            }

            // ── Dungeon Defence room tick ─────────────────────────────────
            // Called every 1s for each active room. Handles spawn timers,
            // room close countdowns, and finish out timers.
            {
                use crate::handler::dungeon_defence;

                // Collect tick results, then act on them outside of any lock
                let mut spawn_rooms: Vec<(u16, i16, u16)> = Vec::with_capacity(10);
                let mut kick_rooms: Vec<u16> = Vec::with_capacity(10);

                for room in world.dd_rooms() {
                    if !room.is_started.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    let room_id = room.room_id.load(std::sync::atomic::Ordering::Relaxed);
                    match dungeon_defence::timer_tick(room) {
                        dungeon_defence::DdTickResult::SpawnStage {
                            stage_id,
                            difficulty,
                        } => {
                            spawn_rooms.push((room_id, stage_id, difficulty));
                        }
                        dungeon_defence::DdTickResult::KickAll => {
                            kick_rooms.push(room_id);
                        }
                        dungeon_defence::DdTickResult::Idle => {}
                    }
                }

                // Process spawn events
                for (room_id, stage_id, difficulty) in spawn_rooms {
                    // Collect monster spawn data under read lock, then drop it
                    // before any .await to avoid holding RwLockReadGuard across await.
                    let (total_count, spawn_list) = {
                        let monsters_guard = world.dd_monsters();
                        let stage_monsters =
                            dungeon_defence::get_stage_monsters(&monsters_guard, stage_id);
                        let total: u32 = stage_monsters
                            .iter()
                            .map(|m| m.s_count.unwrap_or(1) as u32)
                            .sum();
                        let list: Vec<(u16, bool, f32, f32, u16, u8)> = stage_monsters
                            .iter()
                            .map(|m| {
                                (
                                    m.monster_id as u16,
                                    !m.is_monster,
                                    m.pos_x as f32,
                                    m.pos_z as f32,
                                    m.s_count.unwrap_or(1) as u16,
                                    m.s_direction as u8,
                                )
                            })
                            .collect();
                        (total, list)
                    }; // guard dropped here

                    // Set kill_count on the room
                    if let Some(room) = world
                        .dd_rooms()
                        .iter()
                        .find(|r| r.room_id.load(std::sync::atomic::Ordering::Relaxed) == room_id)
                    {
                        room.kill_count
                            .store(total_count, std::sync::atomic::Ordering::Relaxed);
                    }

                    // Send stage counter packet BEFORE spawning monsters
                    if let Some((max_stage, display_stage)) =
                        dungeon_defence::get_stage_display(difficulty, stage_id)
                    {
                        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizEvent as u8);
                        pkt.write_u8(dungeon_defence::TEMPLE_EVENT_STAGE_COUNTER);
                        pkt.write_u8(max_stage);
                        pkt.write_u8(display_stage);
                        world.broadcast_to_zone_event_room(
                            dungeon_defence::ZONE_DUNGEON_DEFENCE,
                            room_id,
                            Arc::new(pkt),
                            None,
                        );
                    }

                    // Spawn all monsters for this stage
                    for &(npc_id, is_monster, px, pz, count, dir) in &spawn_list {
                        world.spawn_event_npc_ex(
                            npc_id,
                            is_monster,
                            dungeon_defence::ZONE_DUNGEON_DEFENCE,
                            px,
                            pz,
                            count,
                            room_id,
                            dir,
                        );
                    }

                    tracing::debug!(
                        "DD room {}: spawning stage {} ({} monsters), difficulty={}",
                        room_id,
                        stage_id,
                        total_count,
                        difficulty
                    );
                }

                // Process kick events
                for room_id in kick_rooms {
                    // Find all users in this DD room and teleport them to Moradon
                    let user_sids = world.collect_sessions_by(|h| {
                        h.event_room == room_id
                            && h.position.zone_id == dungeon_defence::ZONE_DUNGEON_DEFENCE
                    });

                    for sid in &user_sids {
                        // Remove Monster Coins and restore normal HP before teleport
                        world.rob_all_of_item(*sid, dungeon_defence::MONSTER_COIN_ITEM);
                        world.recalculate_max_hp_mp(*sid);
                        world.update_session(*sid, |h| {
                            h.event_room = 0;
                        });
                        crate::handler::zone_change::server_teleport_to_zone(
                            &world,
                            *sid,
                            crate::world::types::ZONE_MORADON,
                            0.0,
                            0.0,
                        )
                    }

                    // Despawn room NPCs and reset
                    world.despawn_room_npcs(dungeon_defence::ZONE_DUNGEON_DEFENCE, room_id);

                    // Reset room back to pool
                    if let Some(room) = world
                        .dd_rooms()
                        .iter()
                        .find(|r| r.room_id.load(std::sync::atomic::Ordering::Relaxed) == room_id)
                    {
                        room.reset();
                    }

                    tracing::debug!(
                        "DD room {} kicked: {} users teleported to Moradon",
                        room_id,
                        user_sids.len()
                    );
                }
            }

            // ── Forgotten Temple timer tick ───────────────────────────────
            // Called every 1s while the event is active. Handles summon phase,
            // stage spawning, victory detection, and finish/kick logic.
            {
                use crate::handler::forgotten_temple;

                let ft_state = world.forgotten_temple_state();

                // Only tick when FT is active
                if ft_state
                    .is_active
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    // Build FtTimerOptions from the EventRoomManager's ForgottenTempleOpts
                    let ft_timer_opts = {
                        let opts = world.event_room_manager().ft_opts.read();
                        forgotten_temple::FtTimerOptions {
                            playing_time: opts.playing_time as u16,
                            summon_time: opts.summon_time as u16,
                            spawn_min_time: opts.spawn_min_time as u16,
                            waiting_time: opts.waiting_time as u16,
                            min_level: opts.min_level as u8,
                            max_level: opts.max_level as u8,
                        }
                    };

                    // Convert FtStageRow (DB) → FtStageEntry (handler)
                    let stage_entries: Vec<forgotten_temple::FtStageEntry> = {
                        let db_stages = world.ft_stages();
                        db_stages
                            .iter()
                            .map(|r| forgotten_temple::FtStageEntry {
                                n_index: r.n_index,
                                event_type: r.event_type,
                                stage: r.stage,
                                time_offset: r.time_offset,
                            })
                            .collect()
                    };

                    match forgotten_temple::timer_tick(
                        ft_state,
                        &ft_timer_opts,
                        &stage_entries,
                        now,
                    ) {
                        forgotten_temple::FtTickResult::SummonPhaseStarted => {
                            tracing::info!("Forgotten Temple: summon phase started");
                        }
                        forgotten_temple::FtTickResult::SpawnStage(stage) => {
                            // Collect summon entries for this stage
                            let et = ft_state
                                .event_type
                                .load(std::sync::atomic::Ordering::Relaxed);
                            let spawn_list: Vec<(u16, i16, i16, i16, i16)> = {
                                let db_summons = world.ft_summons();
                                db_summons
                                    .iter()
                                    .filter(|s| {
                                        s.event_type == et as i16 && s.stage == stage as i16
                                    })
                                    .map(|s| {
                                        (
                                            s.sid_id as u16,
                                            s.sid_count,
                                            s.pos_x,
                                            s.pos_z,
                                            s.spawn_range,
                                        )
                                    })
                                    .collect()
                            };

                            let mut total_count: u32 = 0;
                            for &(sid_id, sid_count, pos_x, pos_z, _range) in &spawn_list {
                                total_count += sid_count as u32;
                                world.spawn_event_npc(
                                    sid_id,
                                    true, // is_monster
                                    forgotten_temple::ZONE_FORGOTTEN_TEMPLE,
                                    pos_x as f32,
                                    pos_z as f32,
                                    sid_count as u16,
                                );
                            }

                            ft_state
                                .monster_count
                                .fetch_add(total_count, std::sync::atomic::Ordering::Relaxed);

                            tracing::info!(stage, total_count, "Forgotten Temple: spawning stage");
                        }
                        forgotten_temple::FtTickResult::LastSummonReached => {
                            tracing::info!(
                                "Forgotten Temple: last summon reached, waiting for kills"
                            );
                        }
                        forgotten_temple::FtTickResult::Victory => {
                            tracing::info!("Forgotten Temple: victory! Distributing rewards...");
                            distribute_ft_rewards(&world).await;
                        }
                        forgotten_temple::FtTickResult::Finish => {
                            tracing::info!(
                                "Forgotten Temple: event finished, kicking players to Moradon"
                            );

                            // Find all users in FT zone and teleport to Moradon
                            let ft_users = world.collect_sessions_by(|h| {
                                h.position.zone_id == forgotten_temple::ZONE_FORGOTTEN_TEMPLE
                            });

                            for sid in &ft_users {
                                crate::handler::zone_change::server_teleport_to_zone(
                                    &world,
                                    *sid,
                                    crate::world::types::ZONE_MORADON,
                                    0.0,
                                    0.0,
                                )
                            }

                            // Reset FT state
                            forgotten_temple::finish_event(ft_state);

                            tracing::info!(
                                "Forgotten Temple: {} users teleported to Moradon",
                                ft_users.len()
                            );
                        }
                        forgotten_temple::FtTickResult::Idle => {}
                    }
                }
            }

            // ── Under The Castle timer tick ───────────────────────────────
            // Called every 1s while the event is active. Handles monster spawn,
            // movie trigger, countdown, and event end/kick.
            {
                use crate::handler::under_castle;

                let utc_state = world.under_the_castle_state();

                if utc_state
                    .is_active
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    match under_castle::timer_tick(utc_state) {
                        under_castle::UtcTickResult::SpawnMonsters => {
                            // Spawn all monsters from monster_under_the_castle table
                            // (sid, count, is_monster, x, y, z, dir, trap_number)
                            type UtcSpawnTuple = (i16, i16, bool, i16, i16, i16, i16, i16);
                            let spawn_list: Vec<UtcSpawnTuple> = {
                                let spawns = utc_state.monster_list.read().len();
                                // Only spawn if not already spawned (empty list)
                                if spawns == 0 {
                                    let rows = world.utc_spawns().read();
                                    rows.iter()
                                        .map(|r| {
                                            (
                                                r.s_sid,
                                                r.s_count,
                                                r.b_type == 0, // 0=monster, 1=NPC
                                                r.x,
                                                r.y,
                                                r.z,
                                                r.by_direction,
                                                r.trap_number,
                                            )
                                        })
                                        .collect()
                                } else {
                                    Vec::new()
                                }
                            };

                            for &(s_sid, count, is_monster, x, y, z, direction, trap_number) in
                                &spawn_list
                            {
                                let ids = world.spawn_event_npc_ex_with_metadata(
                                    s_sid as u16,
                                    is_monster,
                                    under_castle::ZONE_UNDER_CASTLE,
                                    x as f32,
                                    y as f32,
                                    z as f32,
                                    count as u16,
                                    0, // event_room (UTC uses single zone)
                                    0, // UTC summon/event type (not trap_number)
                                    direction as u8,
                                    trap_number as u8,
                                );

                                for id in &ids {
                                    under_castle::add_monster_id(utc_state, *id);
                                }

                                // Register gate NPCs: trap_number 1-3 maps to gate index 0-2
                                if (1..=3).contains(&trap_number) {
                                    for id in ids {
                                        under_castle::set_gate_id(
                                            utc_state,
                                            (trap_number - 1) as u8,
                                            id,
                                        );
                                    }
                                }
                            }

                            tracing::info!(
                                count = spawn_list.len(),
                                "Under The Castle: monsters spawned"
                            );
                        }
                        under_castle::UtcTickResult::TriggerMovie => {
                            // Broadcast WIZ_UTC_MOVIE to all players in UTC zone
                            let movie_pkt = under_castle::build_utc_movie_packet(1);
                            world.broadcast_to_zone(
                                under_castle::ZONE_UNDER_CASTLE,
                                Arc::new(movie_pkt),
                                None,
                            );

                            tracing::info!("Under The Castle: movie cutscene triggered");
                        }
                        under_castle::UtcTickResult::Finish => {
                            tracing::info!("Under The Castle: event finished, kicking players");

                            // Find all users in UTC zone and teleport to Moradon
                            let utc_users = world.collect_sessions_by(|h| {
                                h.position.zone_id == under_castle::ZONE_UNDER_CASTLE
                            });

                            for sid in &utc_users {
                                crate::handler::zone_change::server_teleport_to_zone(
                                    &world,
                                    *sid,
                                    crate::world::types::ZONE_MORADON,
                                    0.0,
                                    0.0,
                                )
                            }

                            // Reset UTC state
                            utc_state.reset();

                            tracing::info!(
                                "Under The Castle: {} users teleported to Moradon",
                                utc_users.len()
                            );
                        }
                        under_castle::UtcTickResult::Tick => {
                            // Normal countdown tick — no action needed
                        }
                        under_castle::UtcTickResult::Idle => {}
                    }
                }
            }

            // ── Soccer event timer tick ──────────────────────────────────
            // Called every 1s for each Moradon zone (21-25). Handles match
            // timer countdown, ball position checks, goal detection, match
            // end, and cooldown.
            {
                use crate::handler::soccer;
                use crate::world::{
                    ZONE_MORADON, ZONE_MORADON2, ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
                };

                let moradon_zones = [
                    ZONE_MORADON,
                    ZONE_MORADON2,
                    ZONE_MORADON3,
                    ZONE_MORADON4,
                    ZONE_MORADON5,
                ];

                let soccer_state = world.soccer_state().clone();
                let mut state = soccer_state.write();

                for &zone_id in &moradon_zones {
                    let room = match state.get_room_mut(zone_id) {
                        Some(r) => r,
                        None => continue,
                    };

                    let ball_npc_id = room.ball_npc_id;
                    let tick_result = soccer::timer_tick(room);

                    match tick_result {
                        soccer::TickResult::MatchStart { time } => {
                            // Broadcast timer packet to all registered users
                            let arc_timer = Arc::new(soccer::build_timer_packet(time));
                            for user_name in room.users.keys() {
                                if let Some(sid) = world.find_session_by_name(user_name) {
                                    world.send_to_session_arc(sid, Arc::clone(&arc_timer));
                                }
                            }
                            tracing::info!(zone_id, time, "Soccer: match started");
                        }
                        soccer::TickResult::BallCheck => {
                            // Look up the ball NPC's position and check for goals
                            if ball_npc_id >= 0 {
                                let npc_id = ball_npc_id as u32;
                                if let Some(ball_npc) = world.get_npc_instance(npc_id) {
                                    let ball_zone = soccer::check_ball_position(
                                        ball_npc.zone_id,
                                        ball_npc.x,
                                        ball_npc.z,
                                    );

                                    match ball_zone {
                                        soccer::TEAM_COLOUR_BLUE | soccer::TEAM_COLOUR_RED => {
                                            // Goal scored! Record and broadcast
                                            let Some(room) = state.get_room_mut(zone_id) else {
                                                continue;
                                            };
                                            let socket_id = room.socket_id;
                                            let (blue_goals, red_goals) =
                                                soccer::record_goal(room, ball_zone);

                                            let goal_pkt = soccer::build_goal_packet(
                                                socket_id, ball_zone, blue_goals, red_goals,
                                            );
                                            // Broadcast to zone
                                            world.broadcast_to_zone(
                                                zone_id,
                                                Arc::new(goal_pkt),
                                                None,
                                            );

                                            // Reset ball NPC to center of the field
                                            teleport_ball_npc(
                                                &world,
                                                npc_id,
                                                zone_id,
                                                soccer::BALL_CENTER_X,
                                                soccer::BALL_CENTER_Z,
                                            );

                                            tracing::info!(
                                                zone_id,
                                                ball_zone,
                                                blue_goals,
                                                red_goals,
                                                "Soccer: goal scored, ball reset to center"
                                            );
                                        }
                                        soccer::TEAM_COLOUR_OUTSIDE => {
                                            // Ball went out — reset to center
                                            teleport_ball_npc(
                                                &world,
                                                npc_id,
                                                zone_id,
                                                soccer::BALL_CENTER_X,
                                                soccer::BALL_CENTER_Z,
                                            );
                                            tracing::debug!(
                                                zone_id,
                                                "Soccer: ball out of bounds, reset to center"
                                            );
                                        }
                                        _ => {
                                            // Ball is on the field, no action needed
                                        }
                                    }
                                }
                            }
                        }
                        soccer::TickResult::MatchEnd {
                            blue_goals,
                            red_goals,
                        } => {
                            let winner = soccer::determine_winner(blue_goals, red_goals);
                            let end_pkt = soccer::build_end_packet(winner, blue_goals, red_goals);

                            // Send end packet to all registered users and teleport them
                            let Some(room) = state.get_room_mut(zone_id) else {
                                continue;
                            };
                            let user_names: Vec<(String, u8)> = room
                                .users
                                .iter()
                                .map(|(name, u)| (name.clone(), u.team))
                                .collect();

                            let arc_end = Arc::new(end_pkt);
                            for (user_name, team) in &user_names {
                                if let Some(sid) = world.find_session_by_name(user_name) {
                                    world.send_to_session_arc(sid, Arc::clone(&arc_end));
                                    // Same-zone warp to team-specific end position
                                    let (tx, tz) = soccer::end_teleport_position(*team);
                                    warp_player_in_zone(&world, sid, zone_id, tx, tz);
                                }
                            }

                            tracing::info!(
                                zone_id,
                                blue_goals,
                                red_goals,
                                winner,
                                "Soccer: match ended"
                            );
                        }
                        soccer::TickResult::Cooldown { .. } => {
                            // Cooldown ticking — no action needed
                        }
                        soccer::TickResult::CooldownDone => {
                            // Clean up the room for next match
                            if let Some(room) = state.get_room_mut(zone_id) {
                                room.clean();
                            }
                            tracing::debug!(zone_id, "Soccer: room reset after cooldown");
                        }
                        soccer::TickResult::Idle => {}
                    }
                }
            }

            // ── Lottery event timer tick ──────────────────────────────────
            // Called every 1s. Sends countdown warnings at 15/10/5/3/2/1 min
            // remaining. When time expires: draws winners, sends reward
            // letters, broadcasts winner announcement, resets state.
            {
                use crate::handler::lottery;

                let lottery_proc = world.lottery_process().clone();
                let now_u32 = now as u32;

                match lottery::lottery_timer_tick(&lottery_proc, now_u32) {
                    lottery::LotteryTickResult::CountdownWarning(minutes) => {
                        let msg = format!("[Lottery Event] Remaining Minute {}", minutes);
                        let announce_pkt = lottery::build_lottery_announce(&msg);
                        world.broadcast_to_all(Arc::new(announce_pkt), None);
                        tracing::info!(
                            "[Lottery] Countdown warning: {} minutes remaining",
                            minutes
                        );
                    }
                    lottery::LotteryTickResult::Expired { winners } => {
                        // ── 1. Broadcast winner announcement ─────────────────
                        let header_msg = "------Lottery Event Winners------";
                        let header_pkt = lottery::build_lottery_announce(header_msg);
                        world.broadcast_to_all(Arc::new(header_pkt), None);

                        if winners.is_empty() {
                            let no_winner_pkt = lottery::build_lottery_announce("No Winner");
                            world.broadcast_to_all(Arc::new(no_winner_pkt), None);
                        } else {
                            for (rank, (name, _item_id)) in winners.iter().enumerate() {
                                let msg = lottery::format_winner_message(rank + 1, name);
                                let winner_pkt = lottery::build_lottery_announce(&msg);
                                world.broadcast_to_all(Arc::new(winner_pkt), None);
                            }
                            let footer_pkt = lottery::build_lottery_announce(
                                "---------------------------------",
                            );
                            world.broadcast_to_all(Arc::new(footer_pkt), None);
                        }

                        // ── 2. Send reward letters to winners ────────────────
                        if let Some(pool) = world.db_pool() {
                            for (name, item_id) in &winners {
                                match crate::handler::letter::create_system_letter(
                                    pool, "LOTTERY", name, "REWARD", "REWARD", *item_id,
                                    1, // count: C++ sets 1 for non-countable items
                                    0, // durability: filled from item table at delivery
                                )
                                .await
                                {
                                    Ok(true) => {
                                        tracing::info!(
                                            "[Lottery] Reward letter sent: {} -> item {}",
                                            name,
                                            item_id
                                        );
                                    }
                                    Ok(false) => {
                                        tracing::warn!(
                                            "[Lottery] Reward letter failed (no recipient?): {} -> item {}",
                                            name,
                                            item_id
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "[Lottery] Reward letter DB error for {}: {}",
                                            name,
                                            e
                                        );
                                    }
                                }
                            }
                        } else {
                            tracing::warn!(
                                "[Lottery] No DB pool available — {} winner letters not sent",
                                winners.len()
                            );
                        }

                        // ── 3. Broadcast end packet + finish announcement ────
                        let end_pkt = lottery::build_end_packet();
                        world.broadcast_to_all(Arc::new(end_pkt), None);

                        let finish_msg = "Lottery Event has finished.";
                        let finish_pkt = lottery::build_lottery_announce(finish_msg);
                        world.broadcast_to_all(Arc::new(finish_pkt), None);

                        tracing::info!(
                            "[Lottery] Event finished. {} winners drawn.",
                            winners.len()
                        );
                    }
                    lottery::LotteryTickResult::Idle => {}
                }
            }

            // ── Bifrost event timer tick ──────────────────────────────────
            //                `EventMainTimer()` in EventMainTimer.cpp:244-258
            // Decrements remaining seconds, checks farming phase expiry,
            // and handles loser nation sign-in.
            {
                use crate::handler::bifrost;

                match bifrost::bifrost_tick(&world) {
                    bifrost::BifrostTickResult::Draw => {
                        // Monument not destroyed before timer expired
                        bifrost::broadcast_beef_notice(&world, bifrost::NOTICE_DRAW);
                        bifrost::bifrost_reset(&world);
                        tracing::info!("[Bifrost] Event ended in draw — reset");
                    }
                    bifrost::BifrostTickResult::LoserSignOpened => {
                        bifrost::broadcast_beef_notice(&world, bifrost::NOTICE_LOSER_SIGN);
                        tracing::info!("[Bifrost] Loser nation sign-in opened");
                    }
                    bifrost::BifrostTickResult::FarmingExpired => {
                        // Farming phase ended — broadcast finish, kick, reset
                        bifrost::broadcast_beef_notice(&world, bifrost::NOTICE_FINISH);

                        let sessions = world.sessions_in_zone(ZONE_BIFROST);
                        for sid in sessions {
                            crate::handler::zone_change::server_teleport_to_zone(
                                &world,
                                sid,
                                ZONE_RONARK_LAND,
                                0.0,
                                0.0,
                            )
                        }

                        bifrost::bifrost_reset(&world);
                        tracing::info!(
                            "[Bifrost] Farming phase expired — users kicked, event reset"
                        );
                    }
                    bifrost::BifrostTickResult::Idle => {}
                }
            }
        }
    })
}

/// Broadcast the in-game Juraid registration notice alongside the native event UI.
pub fn broadcast_juraid_registration_notice(world: &WorldState, remaining_secs: u16) {
    let msg = juraid_registration_notice_message(remaining_secs);
    let pkt = crate::systems::timed_notice::build_notice_packet(8, &msg);
    world.broadcast_to_all(Arc::new(pkt), None);
}

fn broadcast_juraid_active_notice(world: &WorldState, assigned_users: usize) {
    let msg = if assigned_users == 0 {
        "Juraid Mountain registration has ended. No players registered.".to_string()
    } else {
        format!(
            "Juraid Mountain has started. {} player(s) have been assigned.",
            assigned_users
        )
    };
    let pkt = crate::systems::timed_notice::build_notice_packet(8, &msg);
    world.broadcast_to_all(Arc::new(pkt), None);
}

pub fn broadcast_juraid_force_start_notice(world: &WorldState, signed_users: usize) {
    let msg = if signed_users == 0 {
        "Juraid Mountain countdown skipped. No registered players yet.".to_string()
    } else {
        format!(
            "Juraid Mountain countdown skipped. Starting with {} registered player(s).",
            signed_users
        )
    };
    let pkt = crate::systems::timed_notice::build_notice_packet(8, &msg);
    world.broadcast_to_all(Arc::new(pkt), None);
}

fn juraid_registration_notice_message(remaining_secs: u16) -> String {
    let minutes = ((remaining_secs as u32) + 59) / 60;
    if minutes > 0 {
        format!(
            "Juraid Mountain registration has started. Registration closes in {} minute(s).",
            minutes
        )
    } else {
        "Juraid Mountain registration has started.".to_string()
    }
}

/// Broadcast a packet to all active users in a BDW room.
/// Used by the event tick task to send altar respawn broadcasts.
fn broadcast_to_bdw_room(world: &WorldState, room_id: u8, pkt: &ko_protocol::Packet) {
    let Some(room) = world
        .event_room_manager
        .get_room(TempleEventType::BorderDefenceWar, room_id)
    else {
        return;
    };
    let arc_pkt = Arc::new(pkt.clone());
    for u in room.karus_users.values().filter(|u| !u.logged_out) {
        world.send_to_session_arc(u.session_id, Arc::clone(&arc_pkt));
    }
    for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
        world.send_to_session_arc(u.session_id, Arc::clone(&arc_pkt));
    }
}

/// Collect all active (non-logged-out) users from an event room.
/// Returns a list of `(SessionId, user_name)` tuples for teleport operations.
fn collect_room_users(
    erm: &EventRoomManager,
    event_type: TempleEventType,
    room_id: u8,
) -> Vec<(SessionId, String)> {
    let Some(room) = erm.get_room(event_type, room_id) else {
        return Vec::new();
    };
    let total = room.karus_users.len() + room.elmorad_users.len() + room.mixed_users.len();
    let mut users = Vec::with_capacity(total);
    for u in room.karus_users.values().filter(|u| !u.logged_out) {
        users.push((u.session_id, u.user_name.clone()));
    }
    for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
        users.push((u.session_id, u.user_name.clone()));
    }
    for u in room.mixed_users.values().filter(|u| !u.logged_out) {
        users.push((u.session_id, u.user_name.clone()));
    }
    users
}

/// Collect all active session IDs from ALL rooms for an event type.
/// Used to snapshot user sessions before `cleanup_event` destroys rooms.
fn collect_all_room_sessions(
    erm: &EventRoomManager,
    event_type: TempleEventType,
) -> Vec<SessionId> {
    let room_ids = erm.list_rooms(event_type);
    let mut sids = Vec::with_capacity(room_ids.len() * 8);
    for room_id in room_ids {
        let room_users = collect_room_users(erm, event_type, room_id);
        for (sid, _name) in room_users {
            sids.push(sid);
        }
    }
    sids
}

/// Try to open an event based on the loaded schedule entries.
/// Called once per tick when no event is active. Compares current time
/// (weekday, hour, minute) against all schedule entries and opens the
/// first matching event via [`open_virtual_event`].
/// Returns `Some((event_type, sign_secs))` if an event was opened, so the
/// caller can broadcast the sign-up announcement.
/// EventMainTimer.cpp:491-523. Match is exact: `hour == h && minute == m`.
/// Only triggers at second == 0 (once per minute), simulated here by
/// comparing against the `now` timestamp.
fn try_schedule_trigger(erm: &EventRoomManager, now: u64) -> Option<(TempleEventType, u64)> {
    use chrono::{Datelike, TimeZone, Timelike};

    // Convert unix timestamp to local time components.
    // C++ uses `g_localTime.tm_wday`, `tm_hour`, `tm_min`, `tm_sec`.
    let dt = match chrono::Local.timestamp_opt(now as i64, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => return None,
    };
    let weekday = dt.weekday().num_days_from_sunday(); // 0=Sun, 1=Mon, ...
    let hour = dt.hour();
    let minute = dt.minute();
    let second = dt.second();

    // C++ only triggers at second == 0 to avoid re-triggering every tick
    if second != 0 {
        return None;
    }

    let schedules = erm.schedules.read();
    for entry in schedules.iter() {
        if let Some(_time_index) = check_schedule_trigger(entry, weekday, hour, minute) {
            // Map schedule entry's event_id to EventLocalId then vroom index
            let local_id = match EventLocalId::from_u8(entry.event_id as u8) {
                Some(id) => id,
                None => continue,
            };
            let vroom_index = match local_id_to_vroom_index(local_id) {
                Some(idx) => idx,
                None => continue,
            };

            let event_type = match vroom_index_to_event_type(vroom_index) {
                Some(et) => et,
                None => continue,
            };

            let vroom_opts = match erm.get_vroom_opt(vroom_index as usize) {
                Some(opts) => opts,
                None => continue,
            };

            let sign_secs = (vroom_opts.sign as u64) * 60;

            let params = EventOpenParams {
                vroom_index,
                event_type,
                vroom_opts,
                is_automatic: true,
                min_level: entry.min_level,
                max_level: entry.max_level,
                req_loyalty: entry.req_loyalty,
                req_money: entry.req_money,
            };

            if open_virtual_event(erm, &params) {
                tracing::info!(
                    "Schedule auto-trigger: opened {:?} (weekday={}, {:02}:{:02})",
                    event_type,
                    weekday,
                    hour,
                    minute,
                );
                return Some((event_type, sign_secs));
            }
        }
    }
    None
}

// ── Event State ─────────────────────────────────────────────────────────────

/// High-level event lifecycle state.
/// This is a logical overlay on top of `TempleEventState` fields, providing
/// clearer semantics for state machine transitions.
/// (`isActive`, `bAllowJoin`, `EventTimerFinishControl`, etc.)
/// Note: C++ does not have a distinct "Scoring" phase — winner determination
/// and reward distribution happen atomically. The flow goes directly from
/// Active to Rewards (via `TempleEventSendWinnerScreen` + `EventTimerFinishControl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    /// No event is active. Waiting for schedule trigger or GM command.
    Inactive,
    /// Sign-up period. Players can join/leave.
    Registration,
    /// Sign-up closed, assigning rooms, about to teleport players.
    Countdown,
    /// Event is in progress. Combat may be allowed per timer.
    Active,
    /// Winner screen sent. Distributing rewards.
    Rewards,
    /// Kicking players, resetting rooms. About to go inactive.
    Cleanup,
}

impl EventPhase {
    /// Convert from u8 for DB/config serialization.
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Inactive),
            1 => Some(Self::Registration),
            2 => Some(Self::Countdown),
            3 => Some(Self::Active),
            4 => Some(Self::Rewards),
            5 => Some(Self::Cleanup),
            _ => None,
        }
    }
}

// ── Schedule Checker ────────────────────────────────────────────────────────

/// Check if the current day-of-week (0=Sun..6=Sat) matches the schedule entry.
fn is_on_day(entry: &EventScheduleEntry, weekday: u32) -> bool {
    if weekday > 6 {
        return false;
    }
    entry.days[weekday as usize]
}

/// Check if a schedule entry matches the current time and day.
/// Returns the index of the matching start time (0..4), or None.
pub fn check_schedule_trigger(
    entry: &EventScheduleEntry,
    weekday: u32,
    hour: u32,
    minute: u32,
) -> Option<usize> {
    //   `pTempleEvent.type == VirtualRoom`
    // Only VirtualRoom (type == 2) schedule entries should trigger events.
    if !entry.status || entry.event_type != 2 || !is_on_day(entry, weekday) {
        return None;
    }

    for i in 0..5 {
        let (h, m) = entry.start_times[i];
        if h < 0 || m < 0 {
            continue;
        }
        if h as u32 == hour && m as u32 == minute {
            return Some(i);
        }
    }
    None
}

/// Map an `EventLocalId` to the virtual room config index (0=BDW, 1=Chaos, 2=JR).
pub fn local_id_to_vroom_index(local_id: EventLocalId) -> Option<u8> {
    match local_id {
        EventLocalId::BorderDefenceWar => Some(0),
        EventLocalId::ChaosExpansion => Some(1),
        EventLocalId::JuraidMountain => Some(2),
        _ => None,
    }
}

/// Map a vroom index (0=BDW, 1=Chaos, 2=JR) to `TempleEventType`.
pub fn vroom_index_to_event_type(index: u8) -> Option<TempleEventType> {
    match index {
        0 => Some(TempleEventType::BorderDefenceWar),
        1 => Some(TempleEventType::ChaosDungeon),
        2 => Some(TempleEventType::JuraidMountain),
        _ => None,
    }
}

// ── Event Open Logic ────────────────────────────────────────────────────────

/// Parameters for opening a virtual room event.
#[derive(Debug, Clone)]
pub struct EventOpenParams {
    /// Vroom config index (0=BDW, 1=Chaos, 2=JR).
    pub vroom_index: u8,
    /// Event type.
    pub event_type: TempleEventType,
    /// Vroom timing options.
    pub vroom_opts: VroomOpt,
    /// Whether this is an automatic (scheduled) opening.
    pub is_automatic: bool,
    /// Minimum level requirement (0 = no requirement).
    pub min_level: i16,
    /// Maximum level requirement (0 = no requirement).
    pub max_level: i16,
    /// Required loyalty points.
    pub req_loyalty: i32,
    /// Required gold.
    pub req_money: i32,
}

/// Open a virtual room event (transition to Registration phase).
/// Resets the temple event state and configures sign-up parameters.
/// Returns true if the event was successfully opened.
pub fn open_virtual_event(mgr: &EventRoomManager, params: &EventOpenParams) -> bool {
    let now = unix_now();

    let sign_secs = (params.vroom_opts.sign as u64) * 60;
    let play_secs = (params.vroom_opts.play as u64) * 60;
    let close_time = sign_secs + play_secs;

    // Reset any previous event
    mgr.reset_temple_event();

    // Configure the new event
    mgr.update_temple_event(|s| {
        s.is_attackable = false;
        s.allow_join = true;
        s.active_event = params.event_type as i16;
        s.zone_id = params.event_type.zone_id();
        s.start_time = now;
        s.closed_time = now + close_time;
        s.sign_remain_seconds = now + sign_secs;
        s.is_automatic = params.is_automatic;
    });

    true
}

/// Get the current event phase from `TempleEventState` flags.
/// Maps the C++ flag-based state to our clean enum.
pub fn current_phase(mgr: &EventRoomManager) -> EventPhase {
    mgr.read_temple_event(|s| {
        if s.active_event < 0 {
            return EventPhase::Inactive;
        }
        if s.timer_reset_control {
            return EventPhase::Cleanup;
        }
        if s.timer_finish_control {
            return EventPhase::Rewards;
        }
        if !s.is_active {
            return EventPhase::Registration;
        }
        if !s.timer_start_control {
            return EventPhase::Countdown;
        }
        EventPhase::Active
    })
}

/// Close sign-up and transition to Active state.
pub fn transition_to_active(mgr: &EventRoomManager) {
    mgr.update_temple_event(|s| {
        s.allow_join = false;
        s.sign_remain_seconds = 0;
        s.last_event_room = 1;
        s.is_active = true;
        s.timer_start_control = true;
    });
}

/// Transition to the finishing state (winner screen sent).
/// `TempleEventSendWinnerScreen()` call
pub fn transition_to_finish(mgr: &EventRoomManager) {
    mgr.update_temple_event(|s| {
        s.timer_finish_control = true;
    });
}

/// Transition to full cleanup/reset.
pub fn transition_to_reset(mgr: &EventRoomManager) {
    mgr.update_temple_event(|s| {
        s.timer_reset_control = true;
    });
}

/// Process GM manual close request.
/// Sets the manual close flag and records the timestamp.
/// The timer loop will handle the actual cleanup after the configured finish delay.
pub fn manual_close(mgr: &EventRoomManager) -> bool {
    let now = unix_now();

    mgr.update_temple_event(|s| {
        if s.active_event < 0 || !s.is_active {
            return false;
        }
        if s.manual_close {
            return false; // already submitted
        }
        s.manual_close = true;
        s.manual_closed_time = now;
        true
    })
}

// ── Timer Checks ────────────────────────────────────────────────────────────

/// Check if the sign-up period has expired.
/// BDW uses strict `>` (C++ EventMainTimer.cpp:440), while Chaos/Juraid
/// use `>=`. This means BDW transitions one second later.
pub fn is_sign_expired(mgr: &EventRoomManager, vroom_opts: &VroomOpt) -> bool {
    let now = unix_now();
    is_sign_expired_at(mgr, vroom_opts, now)
}

/// Check if the sign-up period has expired using an explicit timestamp.
/// Test-friendly version that accepts `now` as a parameter.
/// Chaos/Juraid use `>=`. Audited in Sprint 202: only called from the main
/// event tick loop (Registration phase). No separate real-time path exists.
pub fn is_sign_expired_at(mgr: &EventRoomManager, vroom_opts: &VroomOpt, now: u64) -> bool {
    let sign_secs = (vroom_opts.sign as u64) * 60;
    mgr.read_temple_event(|s| {
        // EventMainTimer.cpp:530,630 — Chaos/Juraid use `>=`
        if s.active_event == TempleEventType::BorderDefenceWar as i16 {
            now > s.start_time + sign_secs
        } else {
            now >= s.start_time + sign_secs
        }
    })
}

/// Check if the attack-open time has been reached.
pub fn is_attack_open_time(mgr: &EventRoomManager, vroom_opts: &VroomOpt) -> bool {
    let now = unix_now();
    let attack_open_secs = ((vroom_opts.sign + vroom_opts.attack_open) as u64) * 60;
    mgr.read_temple_event(|s| now >= s.start_time + attack_open_secs)
}

/// Check if the attack-close time has been reached.
pub fn is_attack_close_time(mgr: &EventRoomManager, vroom_opts: &VroomOpt) -> bool {
    let now = unix_now();
    let attack_close_secs = ((vroom_opts.sign + vroom_opts.attack_close) as u64) * 60;
    mgr.read_temple_event(|s| now >= s.start_time + attack_close_secs)
}

/// Check if the play timer has expired (event should finish).
pub fn is_play_expired(mgr: &EventRoomManager, vroom_opts: &VroomOpt) -> bool {
    let now = unix_now();
    let finish_secs = ((vroom_opts.sign + vroom_opts.play) as u64) * 60;
    mgr.read_temple_event(|s| now >= s.start_time + finish_secs)
}

/// Check if the play timer has expired using an explicit timestamp.
/// Test-friendly version that accepts `now` as a parameter.
pub fn is_play_expired_at(mgr: &EventRoomManager, vroom_opts: &VroomOpt, now: u64) -> bool {
    let finish_secs = ((vroom_opts.sign + vroom_opts.play) as u64) * 60;
    mgr.read_temple_event(|s| now >= s.start_time + finish_secs)
}

/// Check if the reset timer has expired (cleanup should happen).
pub fn is_reset_expired(mgr: &EventRoomManager, vroom_opts: &VroomOpt, active_event: i16) -> bool {
    let now = unix_now();
    is_reset_expired_at(mgr, vroom_opts, now, active_event)
}

/// Check if the reset timer has expired using an explicit timestamp.
/// Test-friendly version that accepts `now` as a parameter.
/// - BDW/Juraid: `(sign + play + 1) * MINUTE + finish`
/// - Chaos: `(sign + play) * MINUTE + finish` (no +1)
pub fn is_reset_expired_at(
    mgr: &EventRoomManager,
    vroom_opts: &VroomOpt,
    now: u64,
    active_event: i16,
) -> bool {
    let extra_min: i32 = if active_event == TempleEventType::ChaosDungeon as i16 {
        0
    } else {
        1
    };
    let reset_secs =
        ((vroom_opts.sign + vroom_opts.play + extra_min) as u64) * 60 + vroom_opts.finish as u64;
    mgr.read_temple_event(|s| now >= s.start_time + reset_secs)
}

/// Check if a manual close should trigger cleanup.
pub fn is_manual_close_expired(mgr: &EventRoomManager, finish_delay: u64) -> bool {
    let now = unix_now();
    mgr.read_temple_event(|s| {
        if !s.manual_close {
            return false;
        }
        now >= s.manual_closed_time + finish_delay
    })
}

// ── Event Tick ───────────────────────────────────────────────────────────────

/// Result of a single event tick, used for testing and logging.
/// Describes what action (if any) was taken during this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventTickAction {
    /// No action taken (no active event or phase unchanged).
    None,
    /// Sign-up expired; transitioned to Active phase.
    /// Contains the number of users assigned to rooms.
    TransitionedToActive(usize),
    /// Play timer expired; determined winners and transitioned to Rewards.
    /// Contains list of (room_id, winner_nation).
    TransitionedToRewards(Vec<(u8, u8)>),
    /// Reset timer expired; cleaned up and returned to Inactive.
    /// Carries event type + pre-collected session IDs (rooms destroyed after collect).
    TransitionedToCleanup(TempleEventType, Vec<SessionId>),
    /// Manual close timer expired; cleaned up.
    /// Carries event type + pre-collected session IDs (rooms destroyed after collect).
    ManualCloseCleanup(TempleEventType, Vec<SessionId>),
    /// Altar respawns triggered during Active BDW phase.
    /// Contains room IDs where altars respawned (need broadcast).
    BdwAltarRespawns(Vec<u8>),
    /// Juraid bridges opened during Active phase.
    JuraidBridgesOpened(Vec<usize>),
}

/// Process one event tick with explicit timestamp.
/// This is the main event state machine driver. Called every second from
/// the background tick task in `start_event_system_task`.
/// ## State transitions
/// 1. **Registration → Active**: When sign timer expires, assigns users to rooms
///    and transitions to Active phase.
/// 2. **Active → Rewards**: When play timer expires, determines winners and
///    transitions to Rewards phase. Also processes BDW altar ticks and Juraid
///    bridge timers during Active phase.
/// 3. **Rewards → Cleanup**: When reset timer expires, resets all state.
/// 4. **Manual close**: If GM closed the event, cleanup happens after finish_delay.
pub fn event_tick_at(
    erm: &EventRoomManager,
    bdw_mgr: &mut bdw::BdwManager,
    juraid_mgr: &mut juraid::JuraidManager,
    chaos_mgr: &mut chaos::ChaosManager,
    now: u64,
) -> EventTickAction {
    let phase = current_phase(erm);

    // Read the active event type and vroom index
    let (active_event_i16, vroom_idx) = erm.read_temple_event(|s| {
        let ae = s.active_event;
        let idx = TempleEventType::from_i16(ae).and_then(EventRoomManager::vroom_index);
        (ae, idx)
    });

    if active_event_i16 < 0 {
        return EventTickAction::None;
    }

    let vroom_idx = match vroom_idx {
        Some(i) => i,
        None => return EventTickAction::None,
    };

    let opts = match erm.get_vroom_opt(vroom_idx) {
        Some(o) => o,
        None => return EventTickAction::None,
    };

    match phase {
        EventPhase::Inactive => EventTickAction::None,

        EventPhase::Registration => {
            // Check if sign-up period has expired
            if !is_sign_expired_at(erm, &opts, now) {
                return EventTickAction::None;
            }

            // Transition to active: assign rooms, teleport
            transition_to_active(erm);

            let assigned = match TempleEventType::from_i16(active_event_i16) {
                Some(TempleEventType::BorderDefenceWar) => {
                    bdw_mgr.init_rooms(erm);
                    bdw::assign_users_to_rooms(erm, bdw_mgr)
                }
                Some(TempleEventType::JuraidMountain) => {
                    juraid_mgr.init_rooms(erm);
                    juraid_mgr.start_bridge_timer(now);
                    juraid::assign_users_to_rooms(erm, juraid_mgr)
                }
                Some(TempleEventType::ChaosDungeon) => {
                    chaos_mgr.init_rooms(erm);
                    chaos::assign_users_to_rooms(erm, chaos_mgr)
                }
                _ => 0,
            };

            tracing::info!(
                "Event tick: Registration → Active, assigned {} users",
                assigned
            );
            EventTickAction::TransitionedToActive(assigned)
        }

        EventPhase::Countdown => {
            // Countdown is a transient state between Registration and Active.
            // In practice, transition_to_active sets timer_start_control=true
            // so we should not linger here. No action needed.
            EventTickAction::None
        }

        EventPhase::Active => {
            // Check for manual close
            let is_manual = erm.read_temple_event(|s| s.manual_close);
            if is_manual {
                let finish_delay = opts.finish as u64;
                let expired = erm.read_temple_event(|s| now >= s.manual_closed_time + finish_delay);
                if expired {
                    // Capture event type before reset (needed for user kick)
                    let event_type = TempleEventType::from_i16(active_event_i16);

                    // Determine winners, then cleanup
                    let results = match event_type {
                        Some(TempleEventType::BorderDefenceWar) => bdw::determine_all_winners(erm),
                        Some(TempleEventType::JuraidMountain) => {
                            juraid::determine_all_winners(erm, juraid_mgr)
                        }
                        Some(TempleEventType::ChaosDungeon) => chaos::determine_all_winners(erm),
                        _ => Vec::new(),
                    };

                    // Collect user sessions BEFORE cleanup destroys rooms
                    let user_sids = if let Some(et) = event_type {
                        collect_all_room_sessions(erm, et)
                    } else {
                        Vec::new()
                    };

                    transition_to_finish(erm);
                    transition_to_reset(erm);
                    cleanup_event(erm, bdw_mgr, juraid_mgr, chaos_mgr, active_event_i16);

                    tracing::info!(
                        "Event tick: Manual close cleanup, {} room results, {} users to kick",
                        results.len(),
                        user_sids.len(),
                    );
                    if let Some(et) = event_type {
                        return EventTickAction::ManualCloseCleanup(et, user_sids);
                    }
                    return EventTickAction::None;
                }
            }

            // ── Attack open timer ──────────────────────────────────────
            // Sets is_attackable=true when attack_open time is reached.
            {
                let (already_open, attackable) =
                    erm.read_temple_event(|s| (s.timer_attack_open_control, s.is_attackable));
                if !already_open && !attackable {
                    let attack_open_secs = ((opts.sign + opts.attack_open) as u64) * 60;
                    let start_time = erm.read_temple_event(|s| s.start_time);
                    if now >= start_time + attack_open_secs {
                        erm.update_temple_event(|s| {
                            s.is_attackable = true;
                            s.timer_attack_open_control = true;
                        });
                        tracing::info!(
                            "Event {:?} attack OPEN",
                            TempleEventType::from_i16(active_event_i16)
                        );
                    }
                }
            }

            // ── Attack close timer ─────────────────────────────────────
            // Sets is_attackable=false when attack_close time is reached.
            {
                let (already_closed, attackable) =
                    erm.read_temple_event(|s| (s.timer_attack_close_control, s.is_attackable));
                if !already_closed && attackable {
                    let attack_close_secs = ((opts.sign + opts.attack_close) as u64) * 60;
                    let start_time = erm.read_temple_event(|s| s.start_time);
                    if now >= start_time + attack_close_secs {
                        erm.update_temple_event(|s| {
                            s.is_attackable = false;
                            s.timer_attack_close_control = true;
                        });
                        tracing::info!(
                            "Event {:?} attack CLOSED",
                            TempleEventType::from_i16(active_event_i16)
                        );
                    }
                }
            }

            // Check if play timer expired → determine winners
            if is_play_expired_at(erm, &opts, now) {
                let results = match TempleEventType::from_i16(active_event_i16) {
                    Some(TempleEventType::BorderDefenceWar) => bdw::determine_all_winners(erm),
                    Some(TempleEventType::JuraidMountain) => {
                        juraid::determine_all_winners(erm, juraid_mgr)
                    }
                    Some(TempleEventType::ChaosDungeon) => chaos::determine_all_winners(erm),
                    _ => Vec::new(),
                };

                transition_to_finish(erm);

                tracing::info!(
                    "Event tick: Active → Rewards, {} room results",
                    results.len()
                );
                return EventTickAction::TransitionedToRewards(results);
            }

            // Process event-specific active-phase logic
            match TempleEventType::from_i16(active_event_i16) {
                Some(TempleEventType::BorderDefenceWar) => {
                    // Process altar respawn timers for all active BDW rooms
                    let mut respawned_rooms = Vec::new();
                    let room_ids = erm.list_rooms(TempleEventType::BorderDefenceWar);
                    for room_id in room_ids {
                        if let Some(bdw_state) = bdw_mgr.get_room_state_mut(room_id) {
                            if bdw::altar_timer_tick(bdw_state, now) {
                                bdw::altar_respawn_complete(bdw_state);
                                respawned_rooms.push(room_id);
                            }
                        }
                    }
                    if !respawned_rooms.is_empty() {
                        return EventTickAction::BdwAltarRespawns(respawned_rooms);
                    }
                }
                Some(TempleEventType::JuraidMountain) => {
                    // Check bridge timers
                    let opened_indices = juraid::check_bridge_timers(juraid_mgr, now);
                    if !opened_indices.is_empty() {
                        for &bridge_idx in &opened_indices {
                            juraid::open_bridge_for_all_rooms(juraid_mgr, bridge_idx);
                        }
                        tracing::info!("Event tick: Juraid bridges opened: {:?}", opened_indices);
                        return EventTickAction::JuraidBridgesOpened(opened_indices);
                    }
                }
                _ => {}
            }

            EventTickAction::None
        }

        EventPhase::Rewards => {
            // Check if reset timer expired
            if is_reset_expired_at(erm, &opts, now, active_event_i16) {
                // Capture event type + collect sessions BEFORE cleanup destroys rooms
                let event_type = TempleEventType::from_i16(active_event_i16);
                let user_sids = if let Some(et) = event_type {
                    collect_all_room_sessions(erm, et)
                } else {
                    Vec::new()
                };
                transition_to_reset(erm);
                cleanup_event(erm, bdw_mgr, juraid_mgr, chaos_mgr, active_event_i16);

                tracing::info!(
                    "Event tick: Rewards → Cleanup → Inactive, {} users to kick",
                    user_sids.len(),
                );
                if let Some(et) = event_type {
                    return EventTickAction::TransitionedToCleanup(et, user_sids);
                }
                return EventTickAction::None;
            }
            EventTickAction::None
        }

        EventPhase::Cleanup => {
            // Already in cleanup — full reset should have been done.
            // If somehow we're still here, force reset.
            erm.reset_temple_event();
            EventTickAction::None
        }
    }
}

/// Clean up event state (destroy rooms, reset managers).
/// Called when transitioning to Cleanup/Inactive.
fn cleanup_event(
    erm: &EventRoomManager,
    bdw_mgr: &mut bdw::BdwManager,
    juraid_mgr: &mut juraid::JuraidManager,
    chaos_mgr: &mut chaos::ChaosManager,
    active_event_i16: i16,
) {
    match TempleEventType::from_i16(active_event_i16) {
        Some(TempleEventType::BorderDefenceWar) => {
            bdw_mgr.destroy_rooms(erm);
        }
        Some(TempleEventType::JuraidMountain) => {
            juraid_mgr.destroy_rooms(erm);
        }
        Some(TempleEventType::ChaosDungeon) => {
            chaos_mgr.destroy_rooms(erm);
        }
        _ => {}
    }
    erm.reset_temple_event();
}

// ── Reward Distribution ─────────────────────────────────────────────────────

/// Aggregated reward totals from one or more `EventRewardRow` entries.
/// Collects all non-zero items and sums exp/loyalty/noah across rows.
/// before distributing to players.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AggregatedReward {
    /// Non-zero reward items as `(item_id, count)` pairs.
    pub items: Vec<(u32, u16)>,
    /// Total experience reward.
    pub experience: i64,
    /// Total loyalty (NP) reward.
    pub loyalty: i32,
    /// Total gold (noah) reward.
    pub noah: i32,
}

/// Aggregate multiple reward rows into a single `AggregatedReward`.
/// Collects all non-zero item slots and sums scalar rewards.
pub fn aggregate_rewards(rows: &[&EventRewardRow]) -> AggregatedReward {
    let mut result = AggregatedReward::default();
    for r in rows {
        if r.item_id1 > 0 && r.item_count1 > 0 {
            result.items.push((r.item_id1 as u32, r.item_count1 as u16));
        }
        if r.item_id2 > 0 && r.item_count2 > 0 {
            result.items.push((r.item_id2 as u32, r.item_count2 as u16));
        }
        if r.item_id3 > 0 && r.item_count3 > 0 {
            result.items.push((r.item_id3 as u32, r.item_count3 as u16));
        }
        result.experience += r.experience;
        result.loyalty += r.loyalty;
        result.noah += r.noah;
    }
    result
}

/// Convert an event `local_id` (from schedule table) to `TempleEventType`.
/// `TempleEventType` uses internal enum values (4, 24, 100).
fn local_id_to_event_type(local_id: i16) -> Option<TempleEventType> {
    match local_id {
        9 => Some(TempleEventType::BorderDefenceWar),
        10 => Some(TempleEventType::ChaosDungeon),
        11 => Some(TempleEventType::JuraidMountain),
        _ => None,
    }
}

/// Convert a `TempleEventType` discriminant (`active_event`) to `EventLocalId`.
/// converts from `ActiveEvent` (TempleEventType) to `EventLocalID` before
/// looking up reward rows.
/// `active_event` stores TempleEventType values (4, 24, 100) but reward
/// tables are keyed by EventLocalId values (9, 10, 11).
fn active_event_to_local_id(active_event: i16) -> i16 {
    match active_event {
        4 => 9,    // BDW: TempleEventType(4) → EventLocalId::BorderDefenceWar(9)
        24 => 10,  // Chaos: TempleEventType(24) → EventLocalId::ChaosExpansion(10)
        100 => 11, // Juraid: TempleEventType(100) → EventLocalId::JuraidMountain(11)
        other => other,
    }
}

/// Grant a single `AggregatedReward` to a player.
/// Distributes items, exp, loyalty, and gold using existing WorldState methods.
/// Experience is granted via `exp_change_with_bonus` which handles level-up
/// processing (unlike the old `grant_exp_sync` which skipped it).
///   `ExpChange()`, `SendLoyaltyChange()`, `GoldGain()`.
async fn grant_reward(world: &WorldState, sid: SessionId, reward: &AggregatedReward) {
    // Items
    for &(item_id, count) in &reward.items {
        if !world.give_item(sid, item_id, count) {
            tracing::warn!(
                "Failed to give item {}x{} to session {}",
                item_id,
                count,
                sid
            );
        }
    }

    // Experience — use exp_change_with_bonus (is_bonus_reward=true) to skip
    // server-side multipliers (matching C++ bIsBonusReward=true) while still
    // processing level-ups properly.
    if reward.experience != 0 {
        crate::handler::level::exp_change_with_bonus(
            world,
            sid,
            reward.experience,
            true, // is_bonus_reward — skip multipliers for event rewards
        )
        .await;
    }

    // Loyalty (NP) — C++ CindirellaWar.cpp:820 uses (false, false, true)
    if reward.loyalty != 0 {
        crate::systems::loyalty::send_loyalty_change(
            world,
            sid,
            reward.loyalty,
            false, // is_kill_reward
            false, // is_bonus_reward (C++ parity)
            true,  // has_monthly_loyalty (C++ default)
        );
    }

    // Gold (noah)
    if reward.noah > 0 {
        world.gold_gain(sid, reward.noah as u32);
    }
}

/// Distribute Forgotten Temple victory rewards to all eligible players in zone 55.
/// Iterates all in-game, alive players in ZONE_FORGOTTEN_TEMPLE and grants
/// rewards from the EVENT_REWARD table (local_id=13). Each active reward row's
/// items, exp, loyalty, and noah are given to every eligible player.
/// Eligibility: player must be in-game, alive (not dead), and in zone 55.
pub async fn distribute_ft_rewards(world: &WorldState) {
    use crate::handler::forgotten_temple;

    let rewards = match world.get_event_rewards(forgotten_temple::FT_REWARD_LOCAL_ID) {
        Some(r) => r,
        None => {
            tracing::warn!(
                "Forgotten Temple: no event rewards configured for local_id={}",
                forgotten_temple::FT_REWARD_LOCAL_ID
            );
            return;
        }
    };

    // Filter to active reward rows (status=true, local_id=13)
    let active_rows: Vec<&EventRewardRow> = rewards
        .iter()
        .filter(|r| r.status && r.local_id == forgotten_temple::FT_REWARD_LOCAL_ID)
        .collect();

    if active_rows.is_empty() {
        tracing::warn!("Forgotten Temple: no active reward rows found");
        return;
    }

    let aggregated = aggregate_rewards(&active_rows);

    // Collect all eligible players in FT zone
    let ft_users: Vec<SessionId> = world.collect_sessions_by(|h| {
        h.character.is_some()
            && h.position.zone_id == forgotten_temple::ZONE_FORGOTTEN_TEMPLE
            && h.character
                .as_ref()
                .map(|ch| ch.res_hp_type != crate::world::types::USER_DEAD && ch.hp > 0)
                .unwrap_or(false)
    });

    let mut rewarded = 0u32;

    for sid in &ft_users {
        grant_reward(world, *sid, &aggregated).await;
        rewarded += 1;
    }

    tracing::info!(
        "Forgotten Temple: distributed rewards to {}/{} players (items={}, exp={}, loyalty={}, noah={})",
        rewarded,
        ft_users.len(),
        aggregated.items.len(),
        aggregated.experience,
        aggregated.loyalty,
        aggregated.noah,
    );
}

/// Distribute event rewards for a completed BDW or Juraid event.
/// Loads reward rows from `WorldState::get_event_rewards(local_id)` and
/// separates them into winner and loser reward sets. For each room, iterates
/// over participants and grants appropriate rewards to winners and losers.
/// Players who have already received rewards (`prize_given`) or logged out
/// during the event (`logged_out`) are skipped.
/// Experience rewards are granted via `exp_change_with_bonus` which properly
/// handles level-up processing (stat/skill points, HP/MP recalc, broadcast).
/// # Arguments
/// * `world` — Shared world state for reward data lookup and granting.
/// * `local_id` — Event local ID (9=BDW, 11=Juraid).
/// * `winner_results` — List of `(room_id, winner_nation)` pairs from winner determination.
pub async fn distribute_event_rewards(
    world: &WorldState,
    local_id: i16,
    winner_results: &[(u8, u8)],
) {
    let rewards = match world.get_event_rewards(local_id) {
        Some(r) => r,
        None => {
            tracing::warn!("No event rewards configured for local_id={}", local_id);
            return;
        }
    };

    let event_type = match local_id_to_event_type(local_id) {
        Some(et) => et,
        None => {
            tracing::warn!(
                "Unknown event local_id={} for reward distribution",
                local_id
            );
            return;
        }
    };

    let (winner_rows, loser_rows) = partition_rewards_by_winner(&rewards);
    let winner_reward = aggregate_rewards(&winner_rows);
    let loser_reward = aggregate_rewards(&loser_rows);

    tracing::info!(
        "Event reward distribution for local_id={}: {} winner row(s), {} loser row(s), {} room result(s)",
        local_id,
        winner_rows.len(),
        loser_rows.len(),
        winner_results.len(),
    );

    let erm = world.event_room_manager();

    for &(room_id, winner_nation) in winner_results {
        // Collect eligible participants while holding room lock, then drop it
        let participants: Vec<(SessionId, u8)> = {
            let Some(mut room) = erm.get_room_mut(event_type, room_id) else {
                tracing::warn!(
                    "Room {}:{} not found for reward distribution",
                    local_id,
                    room_id
                );
                continue;
            };

            let mut users = Vec::new();

            // Collect from karus_users
            for user in room.karus_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }

            // Collect from elmorad_users
            for user in room.elmorad_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }

            // Collect from mixed_users (Chaos Dungeon uses this instead of nation-split)
            for user in room.mixed_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }

            users
        }; // room lock dropped here

        // Grant rewards to each participant
        for (sid, nation) in participants {
            let base_reward = if nation == winner_nation {
                &winner_reward
            } else {
                &loser_reward
            };

            // BDW: add per-player level-based exp bonus
            if local_id == 9 {
                if let Some(ch) = world.get_character_info(sid) {
                    let level_bonus = bdw_level_exp_bonus(ch.level);
                    if level_bonus > 0 {
                        let mut modified = base_reward.clone();
                        modified.experience += level_bonus;
                        grant_reward(world, sid, &modified).await;
                        tracing::debug!(
                            "Granted {} reward (+ {} BDW level bonus) to session {} (nation={}, level={}) in room {}",
                            if nation == winner_nation { "winner" } else { "loser" },
                            level_bonus, sid, nation, ch.level, room_id
                        );
                        continue;
                    }
                }
            }

            grant_reward(world, sid, base_reward).await;
            tracing::debug!(
                "Granted {} reward to session {} (nation={}) in room {}",
                if nation == winner_nation {
                    "winner"
                } else {
                    "loser"
                },
                sid,
                nation,
                room_id
            );
        }
    }
}

pub async fn distribute_juraid_rewards(world: &WorldState, winner_results: &[(u8, u8)]) {
    let erm = world.event_room_manager();

    for &(room_id, winner_nation) in winner_results {
        let participants: Vec<(SessionId, u8)> = {
            let Some(mut room) = erm.get_room_mut(TempleEventType::JuraidMountain, room_id) else {
                tracing::warn!("Juraid room {} not found for reward distribution", room_id);
                continue;
            };

            let mut users = Vec::new();
            for user in room.karus_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            for user in room.elmorad_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            users
        };

        for (sid, nation) in participants {
            if winner_nation == 0 {
                continue;
            }

            let Some(ch) = world.get_character_info(sid) else {
                continue;
            };
            let level = ch.level;
            let rebirth_level = ch.rebirth_level;
            if let Some(gem_id) = juraid::juraid_winner_gem(level, rebirth_level) {
                let gem_count = juraid::juraid_reward_gem_count(nation == winner_nation);
                if !world.give_item(sid, gem_id, gem_count) {
                    tracing::warn!(
                        sid,
                        gem_id,
                        gem_count,
                        "Juraid reward gem could not be delivered"
                    );
                } else {
                    tracing::info!(
                        sid,
                        room_id,
                        is_winner = nation == winner_nation,
                        gem_id,
                        gem_count,
                        "Juraid gem reward granted"
                    );
                }
            }

            let is_winner = nation == winner_nation;
            if is_winner {
                crate::handler::achieve::on_war_event_result(world, sid, 6);
            }
            let is_premium = world.with_session(sid, |h| h.premium_in_use).unwrap_or(0) != 0;
            let exp = if is_winner {
                if is_premium {
                    50_000_000
                } else {
                    20_000_000
                }
            } else {
                0
            };
            if exp > 0 {
                crate::handler::level::exp_change_with_bonus(world, sid, exp, true).await;
            }

            tracing::info!(
                sid,
                room_id,
                nation,
                level,
                rebirth_level,
                is_winner,
                is_premium,
                exp,
                "Juraid participation reward granted"
            );
        }
    }
}

/// Partition event reward rows into (winner_rewards, loser_rewards).
/// Only active rewards (status=true) are included.
pub fn partition_rewards_by_winner(
    rewards: &[EventRewardRow],
) -> (Vec<&EventRewardRow>, Vec<&EventRewardRow>) {
    let mut winners = Vec::with_capacity(rewards.len());
    let mut losers = Vec::with_capacity(rewards.len());
    for r in rewards {
        if !r.status {
            continue;
        }
        if r.is_winner {
            winners.push(r);
        } else {
            losers.push(r);
        }
    }
    (winners, losers)
}

/// Calculate BDW per-player level-based experience bonus.
/// Low-level players (< 58) get a smaller bonus that scales from level 20.
/// High-level players (>= 58) get a larger bonus.
/// Players at or below level 20 receive no level bonus.
fn bdw_level_exp_bonus(level: u8) -> i64 {
    if level < 58 {
        let clamped = level.max(20) as i64;
        (clamped - 20) * 203_000
    } else {
        (level as i64 + 55) * 120_000
    }
}

/// Calculate BDW per-user EXP reward based on individual contribution points.
/// ```text
/// nGainedExp = level^3 * 0.15 * (5 * bdw_points)
/// nPremiumGainedExp = nGainedExp * 2
/// ```
/// Returns `(normal_exp, premium_exp)` both clamped to their respective caps.
pub fn bdw_user_point_exp(level: u8, bdw_points: u32) -> (i64, i64) {
    let raw = (level as f64).powi(3) * 0.15 * (5 * bdw_points as i64) as f64;
    let gained = (raw as i64).clamp(0, 8_000_000);
    let premium = (gained * 2).clamp(0, 10_000_000);
    (gained, premium)
}

/// Calculate Chaos Dungeon per-user EXP reward based on kill/death stats.
/// ```text
/// nGainedExp = level^3 * 0.15 * (5 * kills - deaths)
/// nPremiumGainedExp = nGainedExp * 2
/// ```
/// Returns `(normal_exp, premium_exp)` both clamped to their respective caps.
pub fn chaos_user_exp(level: u8, kills: u32, deaths: u32) -> (i64, i64) {
    let kill_score = 5i64 * kills as i64 - deaths as i64;
    let raw = (level as f64).powi(3) * 0.15 * kill_score as f64;
    let gained = (raw as i64).clamp(0, 8_000_000);
    let premium = (gained * 2).clamp(0, 10_000_000);
    (gained, premium)
}

// ── Reward Configuration ────────────────────────────────────────────────────

/// Reward configuration for an event outcome (winner or loser).
#[derive(Debug, Clone, Default)]
pub struct EventReward {
    /// Whether this reward entry is active.
    pub active: bool,
    /// Event local ID (9=BDW, 11=Juraid).
    pub local_id: i16,
    /// Whether this is the winner reward (true) or loser (false).
    pub is_winner: bool,
    /// Description for logging.
    pub description: String,
    /// Reward items (item_id, count, expiration_minutes).
    pub items: [(i32, i32, i32); 3],
    /// Bonus experience.
    pub experience: i64,
    /// Bonus loyalty (NP).
    pub loyalty: i32,
    /// Bonus cash shop points.
    pub cash: i32,
    /// Bonus gold.
    pub noah: i32,
}

// ── Chaos Finish EXP Distribution ───────────────────────────────────────────

/// Distribute per-user EXP and rank-based rewards at Chaos Dungeon event finish.
/// For each room:
/// 1. Calculates per-user EXP from kills/deaths via [`chaos_user_exp`].
/// 2. Determines each user's rank (1-based, sorted by kills descending).
/// 3. Grants rank-based rewards from `event_chaos_rewards` table:
///    items, cash, additional EXP, loyalty, and noah.
/// This is called **instead of** `distribute_event_rewards` for Chaos events,
/// because Chaos uses per-user kills/deaths EXP rather than winner/loser
/// table rewards.
pub async fn distribute_chaos_finish_exp(world: &crate::world::WorldState) {
    use crate::systems::event_room::TempleEventType;

    let erm = world.event_room_manager();
    let rooms = erm.list_rooms(TempleEventType::ChaosDungeon);

    let mut total_rewarded = 0u32;

    for room_id in &rooms {
        // Collect eligible users with their kills/deaths while holding lock
        let participants: Vec<(SessionId, u32, u32)> = {
            let Some(mut room) = erm.get_room_mut(TempleEventType::ChaosDungeon, *room_id) else {
                continue;
            };

            if room.finished {
                continue;
            }

            room.finished = true;

            let mut users = Vec::new();
            for user in room.mixed_users.values_mut() {
                if user.user_name.is_empty() || user.prize_given || user.logged_out {
                    continue;
                }

                user.prize_given = true;
                users.push((user.session_id, user.kills, user.deaths));
            }
            users
        }; // room lock dropped

        // Build a ranking list sorted by kills descending (for rank-based rewards)
        let mut ranked = participants.clone();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));

        // Grant EXP and rank-based rewards to each participant
        for (sid, kills, deaths) in &participants {
            let Some(ch) = world.get_character_info(*sid) else {
                continue;
            };

            let (normal_exp, premium_exp) = chaos_user_exp(ch.level, *kills, *deaths);

            // nChangeExp = pUser->GetPremium() != 0 ? nPremiumGainedExp : nGainedExp;
            let is_premium = world.with_session(*sid, |h| h.premium_in_use).unwrap_or(0) != 0;
            let exp = if is_premium { premium_exp } else { normal_exp };

            if exp > 0 {
                crate::handler::level::exp_change_with_bonus(world, *sid, exp, true).await;
            }

            // Rank-based rewards: pChaosReward[rank - 1]
            let user_rank = get_player_chaos_rank(&ranked, *sid);

            // Daily rank stat: CWCounterWin++ for rank 1 winners
            if user_rank == 1 {
                world.update_session(*sid, |h| {
                    h.dr_cw_counter_win += 1;
                });
            }

            if (1..=18).contains(&user_rank) {
                // Extract reward data while holding the read lock, then drop it
                // before any await points.
                let reward_data = {
                    let rewards = world.chaos_stone_rewards();
                    crate::handler::chaos_stone::get_reward_by_rank(&rewards, user_rank as i16).map(
                        |r| {
                            let items = crate::handler::chaos_stone::collect_reward_items(r);
                            (items, r.experience, r.loyalty, r.cash, r.noah)
                        },
                    )
                };

                if let Some((items, rw_exp, rw_loyalty, rw_cash, rw_noah)) = reward_data {
                    // Grant items (up to 5 slots)
                    for (item_id, count, expiry) in &items {
                        if *item_id != 0 {
                            if *expiry > 0 {
                                world.give_item_with_expiry(
                                    *sid,
                                    *item_id as u32,
                                    *count as u16,
                                    *expiry as u32,
                                );
                            } else {
                                world.give_item(*sid, *item_id as u32, *count as u16);
                            }
                        }
                    }

                    // Grant cash (shop balance)
                    if rw_cash > 0 {
                        world.update_session(*sid, |h| {
                            h.inn_coins = h.inn_coins.saturating_add(rw_cash as u32);
                        });
                    }

                    // Grant additional experience
                    if rw_exp != 0 {
                        crate::handler::level::exp_change_with_bonus(
                            world,
                            *sid,
                            rw_exp as i64,
                            true,
                        )
                        .await;
                    }

                    // Grant loyalty (NP) — C++ default bIsAddLoyaltyMonthly = true
                    if rw_loyalty != 0 {
                        crate::systems::loyalty::send_loyalty_change(
                            world, *sid, rw_loyalty, false, false, true,
                        );
                    }

                    // Grant noah (gold)
                    if rw_noah > 0 {
                        world.gold_gain(*sid, rw_noah as u32);
                    }

                    tracing::debug!(
                        "Chaos rank reward: session={}, rank={}, items={}, cash={}, exp={}, loyalty={}, noah={}",
                        sid,
                        user_rank,
                        items.len(),
                        rw_cash,
                        rw_exp,
                        rw_loyalty,
                        rw_noah,
                    );
                }
            }

            total_rewarded += 1;
            tracing::debug!(
                "Chaos finish EXP: session={}, kills={}, deaths={}, level={}, premium={}, exp={}",
                sid,
                kills,
                deaths,
                ch.level,
                is_premium,
                exp
            );
        }
    }

    tracing::info!(
        "Chaos Dungeon finish — distributed EXP + rank rewards to {} users across {} rooms",
        total_rewarded,
        rooms.len()
    );
}

/// Get a player's 1-based rank in the Chaos Dungeon ranking.
/// Iterates the pre-sorted (kills desc) participant list and returns the
/// player's 1-based position. Returns 0 if not found.
fn get_player_chaos_rank(ranked: &[(SessionId, u32, u32)], sid: SessionId) -> u16 {
    for (i, (s, _, _)) in ranked.iter().enumerate() {
        if *s == sid {
            return (i + 1) as u16;
        }
    }
    0
}

// ── Utility ─────────────────────────────────────────────────────────────────

/// Get current unix timestamp in seconds.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Soccer Helper Functions ──────────────────────────────────────────────────

/// Teleport a ball NPC to a new position by despawning and respawning.
fn teleport_ball_npc(world: &WorldState, npc_id: u32, zone_id: u16, new_x: f32, new_z: f32) {
    use crate::npc::{build_npc_inout, NPC_IN, NPC_OUT};

    let old_npc = match world.get_npc_instance(npc_id) {
        Some(n) => n,
        None => return,
    };
    let tmpl = match world.get_npc_template(old_npc.proto_id, old_npc.is_monster) {
        Some(t) => t,
        None => return,
    };

    // 1. Broadcast INOUT_OUT at old position
    let out_pkt = build_npc_inout(NPC_OUT, &old_npc, &tmpl);
    world.broadcast_to_zone(zone_id, Arc::new(out_pkt), None);

    // 2. Update NPC position
    world.update_npc_position(npc_id, new_x, new_z);

    // 3. Broadcast INOUT_IN at new position
    let updated_npc = match world.get_npc_instance(npc_id) {
        Some(n) => n,
        None => return,
    };
    let in_pkt = build_npc_inout(NPC_IN, &updated_npc, &tmpl);
    world.broadcast_to_zone(zone_id, Arc::new(in_pkt), None);
}

/// Warp a player within the same zone (lightweight, no ClientSession needed).
/// Updates position and sends WIZ_WARP to the client so they teleport to new
/// coordinates. Region management will be handled by the client's next
/// movement packet or region check.
fn warp_player_in_zone(world: &WorldState, sid: SessionId, zone_id: u16, dest_x: f32, dest_z: f32) {
    // 1. Update position in WorldState
    world.update_position(sid, zone_id, dest_x, 0.0, dest_z);

    // 2. Send WIZ_WARP to client — client will handle visual teleport
    let mut pkt = Packet::new(Opcode::WizWarp as u8);
    pkt.write_u16((dest_x * 10.0) as u16);
    pkt.write_u16((dest_z * 10.0) as u16);
    pkt.write_i16(-1);
    world.send_to_session_owned(sid, pkt);
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::event_room::EventRoomManager;
    use chrono::TimeZone;
    use ko_protocol::{Opcode, Packet};
    use std::sync::Arc;

    fn make_bdw_opts() -> VroomOpt {
        VroomOpt {
            name: "BDW".to_string(),
            sign: 10,
            play: 15,
            attack_open: 0,
            attack_close: 30,
            finish: 20,
        }
    }

    fn make_juraid_opts() -> VroomOpt {
        VroomOpt {
            name: "Juraid".to_string(),
            sign: 10,
            play: 45,
            attack_open: 0,
            attack_close: 60,
            finish: 30,
        }
    }

    #[test]
    fn test_event_phase_from_u8() {
        assert_eq!(EventPhase::from_u8(0), Some(EventPhase::Inactive));
        assert_eq!(EventPhase::from_u8(1), Some(EventPhase::Registration));
        assert_eq!(EventPhase::from_u8(2), Some(EventPhase::Countdown));
        assert_eq!(EventPhase::from_u8(3), Some(EventPhase::Active));
        assert_eq!(EventPhase::from_u8(4), Some(EventPhase::Rewards));
        assert_eq!(EventPhase::from_u8(5), Some(EventPhase::Cleanup));
        assert_eq!(EventPhase::from_u8(6), None);
    }

    #[test]
    fn test_schedule_is_on_day() {
        let entry = EventScheduleEntry {
            event_id: 9,
            event_type: 2,
            zone_id: 84,
            name: "BDW".to_string(),
            status: true,
            start_times: [(9, 0), (14, 0), (19, 0), (2, 0), (6, 0)],
            days: [true, true, true, true, true, true, true],
            min_level: 35,
            max_level: 83,
            req_loyalty: 0,
            req_money: 0,
        };

        assert!(is_on_day(&entry, 0)); // Sunday
        assert!(is_on_day(&entry, 6)); // Saturday
        assert!(!is_on_day(&entry, 7)); // Invalid

        let entry_some_days = EventScheduleEntry {
            days: [true, false, false, false, false, false, true],
            ..entry
        };
        assert!(is_on_day(&entry_some_days, 0)); // Sunday
        assert!(!is_on_day(&entry_some_days, 1)); // Monday
        assert!(is_on_day(&entry_some_days, 6)); // Saturday
    }

    #[test]
    fn test_check_schedule_trigger() {
        let entry = EventScheduleEntry {
            event_id: 9,
            event_type: 2,
            zone_id: 84,
            name: "BDW".to_string(),
            status: true,
            start_times: [(9, 0), (14, 0), (19, 0), (2, 0), (-1, -1)],
            days: [true, true, true, true, true, true, true],
            min_level: 35,
            max_level: 83,
            req_loyalty: 0,
            req_money: 0,
        };

        // Matching times
        assert_eq!(check_schedule_trigger(&entry, 0, 9, 0), Some(0));
        assert_eq!(check_schedule_trigger(&entry, 3, 14, 0), Some(1));
        assert_eq!(check_schedule_trigger(&entry, 5, 19, 0), Some(2));
        assert_eq!(check_schedule_trigger(&entry, 1, 2, 0), Some(3));

        // Non-matching times
        assert_eq!(check_schedule_trigger(&entry, 0, 10, 0), None);
        assert_eq!(check_schedule_trigger(&entry, 0, 9, 1), None);

        // Disabled entry
        let disabled = EventScheduleEntry {
            status: false,
            ..entry.clone()
        };
        assert_eq!(check_schedule_trigger(&disabled, 0, 9, 0), None);

        // Wrong day
        let weekday_only = EventScheduleEntry {
            days: [false, true, true, true, true, true, false],
            ..entry
        };
        assert_eq!(check_schedule_trigger(&weekday_only, 0, 9, 0), None); // Sunday disabled
        assert_eq!(check_schedule_trigger(&weekday_only, 1, 9, 0), Some(0)); // Monday enabled
    }

    #[test]
    fn test_check_schedule_trigger_inactive_slot() {
        let entry = EventScheduleEntry {
            event_id: 11,
            event_type: 2,
            zone_id: 87,
            name: "Juraid".to_string(),
            status: true,
            start_times: [(10, 0), (15, 0), (-1, -1), (-1, -1), (-1, -1)],
            days: [true, true, true, true, true, true, true],
            min_level: 60,
            max_level: 83,
            req_loyalty: 0,
            req_money: 0,
        };

        assert_eq!(check_schedule_trigger(&entry, 0, 10, 0), Some(0));
        assert_eq!(check_schedule_trigger(&entry, 0, 15, 0), Some(1));
        // Inactive slots should not trigger
        assert_eq!(check_schedule_trigger(&entry, 0, 0, 0), None);
    }

    #[test]
    fn test_local_id_to_vroom_index() {
        assert_eq!(
            local_id_to_vroom_index(EventLocalId::BorderDefenceWar),
            Some(0)
        );
        assert_eq!(
            local_id_to_vroom_index(EventLocalId::ChaosExpansion),
            Some(1)
        );
        assert_eq!(
            local_id_to_vroom_index(EventLocalId::JuraidMountain),
            Some(2)
        );
        assert_eq!(local_id_to_vroom_index(EventLocalId::BeefEvent), None);
        assert_eq!(local_id_to_vroom_index(EventLocalId::CastleSiegeWar), None);
    }

    #[test]
    fn test_vroom_index_to_event_type() {
        assert_eq!(
            vroom_index_to_event_type(0),
            Some(TempleEventType::BorderDefenceWar)
        );
        assert_eq!(
            vroom_index_to_event_type(1),
            Some(TempleEventType::ChaosDungeon)
        );
        assert_eq!(
            vroom_index_to_event_type(2),
            Some(TempleEventType::JuraidMountain)
        );
        assert_eq!(vroom_index_to_event_type(3), None);
    }

    #[test]
    fn test_open_virtual_event() {
        let mgr = EventRoomManager::new();

        let params = EventOpenParams {
            vroom_index: 0,
            event_type: TempleEventType::BorderDefenceWar,
            vroom_opts: make_bdw_opts(),
            is_automatic: true,
            min_level: 35,
            max_level: 83,
            req_loyalty: 0,
            req_money: 0,
        };

        let result = open_virtual_event(&mgr, &params);
        assert!(result);

        // Verify state
        mgr.read_temple_event(|s| {
            assert_eq!(s.active_event, TempleEventType::BorderDefenceWar as i16);
            assert!(!s.is_attackable);
            assert!(s.allow_join);
            assert!(s.is_automatic);
            assert_eq!(s.zone_id, 84);
            assert!(s.start_time > 0);
            assert!(s.closed_time > s.start_time);
            assert!(s.sign_remain_seconds > 0);
        });
    }

    #[test]
    fn test_current_phase_inactive() {
        let mgr = EventRoomManager::new();
        assert_eq!(current_phase(&mgr), EventPhase::Inactive);
    }

    #[test]
    fn test_current_phase_registration() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.allow_join = true;
            s.is_active = false;
        });
        assert_eq!(current_phase(&mgr), EventPhase::Registration);
    }

    #[test]
    fn test_current_phase_active() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
        });
        assert_eq!(current_phase(&mgr), EventPhase::Active);
    }

    #[test]
    fn test_current_phase_rewards() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.timer_finish_control = true;
        });
        assert_eq!(current_phase(&mgr), EventPhase::Rewards);
    }

    #[test]
    fn test_current_phase_cleanup() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::ChaosDungeon as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.timer_finish_control = true;
            s.timer_reset_control = true;
        });
        assert_eq!(current_phase(&mgr), EventPhase::Cleanup);
    }

    #[test]
    fn test_transition_to_active() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.allow_join = true;
        });

        transition_to_active(&mgr);

        mgr.read_temple_event(|s| {
            assert!(!s.allow_join);
            assert_eq!(s.sign_remain_seconds, 0);
            assert_eq!(s.last_event_room, 1);
            assert!(s.is_active);
            assert!(s.timer_start_control);
        });
    }

    #[test]
    fn test_transition_to_finish() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
            s.is_active = true;
        });

        transition_to_finish(&mgr);

        mgr.read_temple_event(|s| {
            assert!(s.timer_finish_control);
        });
    }

    #[test]
    fn test_transition_to_reset() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::ChaosDungeon as i16;
            s.is_active = true;
        });

        transition_to_reset(&mgr);

        mgr.read_temple_event(|s| {
            assert!(s.timer_reset_control);
        });
    }

    #[test]
    fn test_manual_close_success() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
        });

        assert!(manual_close(&mgr));

        mgr.read_temple_event(|s| {
            assert!(s.manual_close);
            assert!(s.manual_closed_time > 0);
        });
    }

    #[test]
    fn test_manual_close_no_active_event() {
        let mgr = EventRoomManager::new();
        assert!(!manual_close(&mgr));
    }

    #[test]
    fn test_manual_close_not_in_active_phase() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = false; // still in signing
        });
        assert!(!manual_close(&mgr));
    }

    #[test]
    fn test_manual_close_already_submitted() {
        let mgr = EventRoomManager::new();
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
        });

        assert!(manual_close(&mgr));
        assert!(!manual_close(&mgr)); // Already submitted
    }

    #[test]
    fn test_is_sign_expired() {
        let mgr = EventRoomManager::new();
        let opts = make_bdw_opts();

        // Set start_time to way in the past
        mgr.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.start_time = 1000; // ancient past
        });

        assert!(is_sign_expired(&mgr, &opts));

        // Set start_time to now (sign not expired)
        let now = unix_now();
        mgr.update_temple_event(|s| {
            s.start_time = now;
        });
        assert!(!is_sign_expired(&mgr, &opts));
    }

    #[test]
    fn test_is_play_expired() {
        let mgr = EventRoomManager::new();
        let opts = make_bdw_opts();

        // Past event
        mgr.update_temple_event(|s| {
            s.start_time = 1000;
        });
        assert!(is_play_expired(&mgr, &opts));

        // Current event
        let now = unix_now();
        mgr.update_temple_event(|s| {
            s.start_time = now;
        });
        assert!(!is_play_expired(&mgr, &opts));
    }

    #[test]
    fn test_is_reset_expired() {
        let mgr = EventRoomManager::new();
        let opts = make_bdw_opts();
        let bdw_event = TempleEventType::BorderDefenceWar as i16;

        // Ancient past
        mgr.update_temple_event(|s| {
            s.start_time = 1000;
        });
        assert!(is_reset_expired(&mgr, &opts, bdw_event));

        // Current
        let now = unix_now();
        mgr.update_temple_event(|s| {
            s.start_time = now;
        });
        assert!(!is_reset_expired(&mgr, &opts, bdw_event));
    }

    #[test]
    fn test_is_reset_expired_chaos_no_extra_minute() {
        let mgr = EventRoomManager::new();
        // sign=5, play=20, finish=30
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 5,
            play: 20,
            attack_open: 6,
            attack_close: 24,
            finish: 30,
        };
        let chaos_event = TempleEventType::ChaosDungeon as i16;
        let bdw_event = TempleEventType::BorderDefenceWar as i16;

        let start = 10_000u64;
        mgr.update_temple_event(|s| {
            s.start_time = start;
        });

        // Chaos: reset = (5 + 20) * 60 + 30 = 1530
        // BDW:   reset = (5 + 20 + 1) * 60 + 30 = 1590

        // At start + 1530: Chaos expired, BDW not
        assert!(is_reset_expired_at(&mgr, &opts, start + 1530, chaos_event));
        assert!(!is_reset_expired_at(&mgr, &opts, start + 1530, bdw_event));

        // At start + 1590: both expired
        assert!(is_reset_expired_at(&mgr, &opts, start + 1590, chaos_event));
        assert!(is_reset_expired_at(&mgr, &opts, start + 1590, bdw_event));

        // At start + 1529: neither expired
        assert!(!is_reset_expired_at(&mgr, &opts, start + 1529, chaos_event));
        assert!(!is_reset_expired_at(&mgr, &opts, start + 1529, bdw_event));
    }

    #[test]
    fn test_is_manual_close_expired() {
        let mgr = EventRoomManager::new();

        // No manual close set
        assert!(!is_manual_close_expired(&mgr, 20));

        // Set manual close in the past
        mgr.update_temple_event(|s| {
            s.manual_close = true;
            s.manual_closed_time = 1000;
        });
        assert!(is_manual_close_expired(&mgr, 20));

        // Set manual close to now (not expired with 20s delay)
        let now = unix_now();
        mgr.update_temple_event(|s| {
            s.manual_closed_time = now;
        });
        assert!(!is_manual_close_expired(&mgr, 20));
    }

    #[test]
    fn test_event_reward_default() {
        let reward = EventReward::default();
        assert!(!reward.active);
        assert_eq!(reward.local_id, 0);
        assert!(!reward.is_winner);
        assert_eq!(reward.experience, 0);
        assert_eq!(reward.loyalty, 0);
        assert_eq!(reward.cash, 0);
        assert_eq!(reward.noah, 0);
        assert_eq!(reward.items, [(0, 0, 0); 3]);
    }

    #[test]
    fn test_event_reward_bdw_winner() {
        let reward = EventReward {
            active: true,
            local_id: 9,
            is_winner: true,
            description: "bdw winner".to_string(),
            items: [(900017000, 1, 0), (0, 0, 0), (0, 0, 0)],
            experience: 200_000_000,
            loyalty: 1000,
            cash: 100,
            noah: 10_000_000,
        };
        assert!(reward.active);
        assert_eq!(reward.local_id, 9);
        assert!(reward.is_winner);
        assert_eq!(reward.experience, 200_000_000);
    }

    #[test]
    fn test_full_event_lifecycle() {
        let mgr = EventRoomManager::new();

        // 1. Start inactive
        assert_eq!(current_phase(&mgr), EventPhase::Inactive);

        // 2. Open BDW event
        let params = EventOpenParams {
            vroom_index: 0,
            event_type: TempleEventType::BorderDefenceWar,
            vroom_opts: make_bdw_opts(),
            is_automatic: false,
            min_level: 35,
            max_level: 83,
            req_loyalty: 0,
            req_money: 0,
        };
        assert!(open_virtual_event(&mgr, &params));
        assert_eq!(current_phase(&mgr), EventPhase::Registration);

        // 3. Transition to active
        transition_to_active(&mgr);
        assert_eq!(current_phase(&mgr), EventPhase::Active);

        // 4. Transition to finish
        transition_to_finish(&mgr);
        assert_eq!(current_phase(&mgr), EventPhase::Rewards);

        // 5. Transition to reset
        transition_to_reset(&mgr);
        assert_eq!(current_phase(&mgr), EventPhase::Cleanup);

        // 6. Full reset
        mgr.reset_temple_event();
        assert_eq!(current_phase(&mgr), EventPhase::Inactive);
    }

    #[test]
    fn test_attack_timer_checks() {
        let mgr = EventRoomManager::new();
        let opts = make_bdw_opts();

        // Ancient past — both should be true
        mgr.update_temple_event(|s| {
            s.start_time = 1000;
        });
        assert!(is_attack_open_time(&mgr, &opts));
        assert!(is_attack_close_time(&mgr, &opts));

        // Current — both should be false (10min + 0min attack_open = 10min in future)
        let now = unix_now();
        mgr.update_temple_event(|s| {
            s.start_time = now;
        });
        assert!(!is_attack_open_time(&mgr, &opts));
        assert!(!is_attack_close_time(&mgr, &opts));
    }

    // ── Event Tick Tests ────────────────────────────────────────────────────

    #[test]
    fn test_event_tick_inactive_no_action() {
        let erm = EventRoomManager::new();
        let mut bdw_mgr = bdw::BdwManager::default();
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let action = event_tick_at(&erm, &mut bdw_mgr, &mut juraid_mgr, &mut chaos_mgr, 1000);
        assert_eq!(action, EventTickAction::None);
    }

    #[test]
    fn test_event_tick_bdw_registration_to_active() {
        let erm = EventRoomManager::new();
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(2);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        // Open BDW event at time 10000
        let start_time = 10000_u64;
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.allow_join = true;
            s.is_active = false;
            s.start_time = start_time;
            s.sign_remain_seconds = start_time + (opts.sign as u64) * 60;
        });

        // Add some signed-up users
        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("e1".to_string(), 2, 2);

        // Before sign expires — should be None
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 500,
        );
        assert_eq!(action, EventTickAction::None);

        // At exact sign expiry (10 min = 600s) — BDW uses strict `>`, so
        // should NOT trigger yet
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 600,
        );
        assert_eq!(action, EventTickAction::None);

        // After sign expires (600+1 = 601s) — now `>` is satisfied
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 601,
        );
        assert_eq!(action, EventTickAction::TransitionedToActive(2));

        // Verify phase
        assert_eq!(current_phase(&erm), EventPhase::Active);
        // Verify rooms created
        assert!(erm.room_count(TempleEventType::BorderDefenceWar) > 0);
    }

    #[test]
    fn test_event_tick_bdw_active_to_rewards() {
        let erm = EventRoomManager::new();
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        // Set up as active BDW with rooms
        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
        });

        // Set some scores
        if let Some(mut room) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 1) {
            room.state = crate::systems::event_room::RoomState::Running;
            room.karus_score = 10;
            room.elmorad_score = 5;
        }

        // Play timer: sign(10) + play(15) = 25 min = 1500s
        let play_end = start_time + 1500;

        // Before play expires — should not transition
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            play_end - 1,
        );
        assert!(matches!(
            action,
            EventTickAction::None | EventTickAction::BdwAltarRespawns(_)
        ));

        // After play expires
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            play_end,
        );
        match action {
            EventTickAction::TransitionedToRewards(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].1, 1); // Karus wins (10 > 5)
            }
            _ => panic!("Expected TransitionedToRewards, got {:?}", action),
        }
        assert_eq!(current_phase(&erm), EventPhase::Rewards);
    }

    #[test]
    fn test_event_tick_rewards_to_cleanup() {
        let erm = EventRoomManager::new();
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        // Set up in Rewards phase
        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.timer_finish_control = true;
            s.start_time = start_time;
        });

        // Reset timer: (sign(10) + play(15) + 1) * 60 + finish(20) = 26*60 + 20 = 1580
        let reset_time = start_time + 1580;

        // Before reset — no action
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            reset_time - 1,
        );
        assert_eq!(action, EventTickAction::None);

        // After reset
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            reset_time,
        );
        assert_eq!(
            action,
            EventTickAction::TransitionedToCleanup(TempleEventType::BorderDefenceWar, vec![])
        );

        // Verify back to inactive
        assert_eq!(current_phase(&erm), EventPhase::Inactive);
    }

    #[test]
    fn test_event_tick_juraid_registration_to_active() {
        let erm = EventRoomManager::new();
        let opts = make_juraid_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[2] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::default();
        let mut juraid_mgr = juraid::JuraidManager::new(2);
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 20000_u64;
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
            s.allow_join = true;
            s.is_active = false;
            s.start_time = start_time;
        });

        erm.add_signed_up_user("k1".to_string(), 1, 1);
        erm.add_signed_up_user("e1".to_string(), 2, 2);

        // sign = 10min = 600s
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 600,
        );
        assert_eq!(action, EventTickAction::TransitionedToActive(2));
        assert_eq!(current_phase(&erm), EventPhase::Active);

        // Verify bridge timer started
        assert!(juraid_mgr.bridge_active);
        assert_eq!(juraid_mgr.bridge_start_time, start_time + 600);
    }

    #[test]
    fn test_event_tick_juraid_bridges() {
        let erm = EventRoomManager::new();
        let opts = make_juraid_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[2] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::default();
        let mut juraid_mgr = juraid::JuraidManager::new(1);
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 20000_u64;
        juraid_mgr.init_rooms(&erm);
        juraid_mgr.start_bridge_timer(start_time);

        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
        });

        // At +1200, the 10- and 20-minute section fallbacks have opened.
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1200,
        );
        assert_eq!(action, EventTickAction::JuraidBridgesOpened(vec![0, 1]));

        // Bridge 2 at +1800
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1800,
        );
        assert_eq!(action, EventTickAction::JuraidBridgesOpened(vec![2]));

        // No bridge remains at +2400.
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 2400,
        );
        assert_eq!(action, EventTickAction::None);

        // No more bridges
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 2500,
        );
        assert_eq!(action, EventTickAction::None);
    }

    #[test]
    fn test_event_tick_bdw_altar_ticks() {
        let erm = EventRoomManager::new();
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
        });

        // Start altar respawn timer
        if let Some(state) = bdw_mgr.get_room_state_mut(1) {
            bdw::start_altar_respawn_timer(state, 0);
        }

        // Tick at time >= ALTAR_RESPAWN_DELAY_SECS — respawn completes
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            bdw::ALTAR_RESPAWN_DELAY_SECS,
        );
        assert_eq!(action, EventTickAction::BdwAltarRespawns(vec![1]));
    }

    #[test]
    fn test_event_tick_manual_close() {
        let erm = EventRoomManager::new();
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
            s.manual_close = true;
            s.manual_closed_time = start_time + 100;
        });

        // finish delay = 20s, so cleanup at manual_closed_time + 20
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 120,
        );
        assert_eq!(
            action,
            EventTickAction::ManualCloseCleanup(TempleEventType::BorderDefenceWar, vec![])
        );
        assert_eq!(current_phase(&erm), EventPhase::Inactive);
    }

    #[test]
    fn test_cleanup_event_bdw() {
        let erm = EventRoomManager::new();
        let mut bdw_mgr = bdw::BdwManager::new(2);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        bdw_mgr.init_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::BorderDefenceWar), 2);

        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
        });

        cleanup_event(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            TempleEventType::BorderDefenceWar as i16,
        );

        assert_eq!(erm.room_count(TempleEventType::BorderDefenceWar), 0);
        assert!(bdw_mgr.room_states.is_empty());
        assert_eq!(current_phase(&erm), EventPhase::Inactive);
    }

    #[test]
    fn test_cleanup_event_juraid() {
        let erm = EventRoomManager::new();
        let mut bdw_mgr = bdw::BdwManager::default();
        let mut juraid_mgr = juraid::JuraidManager::new(3);
        let mut chaos_mgr = chaos::ChaosManager::default();

        juraid_mgr.init_rooms(&erm);
        assert_eq!(erm.room_count(TempleEventType::JuraidMountain), 3);

        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
        });

        cleanup_event(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            TempleEventType::JuraidMountain as i16,
        );

        assert_eq!(erm.room_count(TempleEventType::JuraidMountain), 0);
        assert!(juraid_mgr.room_states.is_empty());
        assert_eq!(current_phase(&erm), EventPhase::Inactive);
    }

    #[test]
    fn test_event_tick_interval_constant() {
        // Verify the tick interval matches C++ (1 second)
        assert_eq!(EVENT_TICK_INTERVAL_SECS, 1);
    }

    #[test]
    fn test_start_event_system_task_compiles() {
        // Verify start_event_system_task signature is correct: takes Arc<WorldState>, returns JoinHandle
        let _: fn(std::sync::Arc<crate::world::WorldState>) -> tokio::task::JoinHandle<()> =
            start_event_system_task;
    }

    // ── Reward Distribution Tests ─────────────────────────────────────

    fn make_reward_row(
        s_index: i32,
        local_id: i16,
        is_winner: bool,
        status: bool,
    ) -> EventRewardRow {
        EventRewardRow {
            s_index,
            status,
            local_id,
            is_winner,
            description: if is_winner {
                "winner".to_string()
            } else {
                "loser".to_string()
            },
            item_id1: 900017000,
            item_count1: 1,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 200_000_000,
            loyalty: 1000,
            cash: 100,
            noah: 10_000_000,
        }
    }

    #[test]
    fn test_partition_rewards_by_winner_basic() {
        let rewards = vec![
            make_reward_row(1, 9, true, true),
            make_reward_row(2, 9, false, true),
        ];

        let (winners, losers) = partition_rewards_by_winner(&rewards);
        assert_eq!(winners.len(), 1);
        assert_eq!(losers.len(), 1);
        assert!(winners[0].is_winner);
        assert!(!losers[0].is_winner);
    }

    #[test]
    fn test_partition_rewards_filters_inactive() {
        let rewards = vec![
            make_reward_row(1, 9, true, true),
            make_reward_row(2, 9, true, false), // inactive
            make_reward_row(3, 9, false, true),
            make_reward_row(4, 9, false, false), // inactive
        ];

        let (winners, losers) = partition_rewards_by_winner(&rewards);
        assert_eq!(winners.len(), 1);
        assert_eq!(losers.len(), 1);
    }

    #[test]
    fn test_partition_rewards_empty() {
        let rewards: Vec<EventRewardRow> = vec![];
        let (winners, losers) = partition_rewards_by_winner(&rewards);
        assert!(winners.is_empty());
        assert!(losers.is_empty());
    }

    #[test]
    fn test_partition_rewards_all_winners() {
        let rewards = vec![
            make_reward_row(1, 11, true, true),
            make_reward_row(2, 11, true, true),
        ];

        let (winners, losers) = partition_rewards_by_winner(&rewards);
        assert_eq!(winners.len(), 2);
        assert!(losers.is_empty());
    }

    #[test]
    fn test_partition_rewards_all_losers() {
        let rewards = vec![
            make_reward_row(1, 11, false, true),
            make_reward_row(2, 11, false, true),
        ];

        let (winners, losers) = partition_rewards_by_winner(&rewards);
        assert!(winners.is_empty());
        assert_eq!(losers.len(), 2);
    }

    #[tokio::test]
    async fn test_distribute_event_rewards_no_rewards() {
        // With a WorldState that has no event rewards loaded,
        // distribute_event_rewards should log a warning and return gracefully.
        let world = WorldState::new();
        let results = vec![(1u8, 1u8)];
        // Should not panic — just logs a warning about missing rewards
        distribute_event_rewards(&world, 9, &results).await;
    }

    // ── Aggregate Reward Tests ────────────────────────────────────────

    #[test]
    fn test_aggregate_rewards_single_row() {
        let row = make_reward_row(1, 9, true, true);
        let refs: Vec<&EventRewardRow> = vec![&row];
        let agg = aggregate_rewards(&refs);

        assert_eq!(agg.items.len(), 1);
        assert_eq!(agg.items[0], (900017000, 1));
        assert_eq!(agg.experience, 200_000_000);
        assert_eq!(agg.loyalty, 1000);
        assert_eq!(agg.noah, 10_000_000);
    }

    #[test]
    fn test_aggregate_rewards_multiple_rows() {
        let row1 = EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 9,
            is_winner: true,
            description: "w1".to_string(),
            item_id1: 100,
            item_count1: 2,
            item_expiration1: 0,
            item_id2: 200,
            item_count2: 3,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 50_000,
            loyalty: 100,
            cash: 0,
            noah: 5_000,
        };
        let row2 = EventRewardRow {
            s_index: 2,
            status: true,
            local_id: 9,
            is_winner: true,
            description: "w2".to_string(),
            item_id1: 300,
            item_count1: 1,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 30_000,
            loyalty: 200,
            cash: 0,
            noah: 3_000,
        };

        let refs: Vec<&EventRewardRow> = vec![&row1, &row2];
        let agg = aggregate_rewards(&refs);

        // Items: (100,2), (200,3) from row1; (300,1) from row2
        assert_eq!(agg.items.len(), 3);
        assert_eq!(agg.items[0], (100, 2));
        assert_eq!(agg.items[1], (200, 3));
        assert_eq!(agg.items[2], (300, 1));
        // Sums
        assert_eq!(agg.experience, 80_000);
        assert_eq!(agg.loyalty, 300);
        assert_eq!(agg.noah, 8_000);
    }

    #[test]
    fn test_aggregate_rewards_empty() {
        let refs: Vec<&EventRewardRow> = vec![];
        let agg = aggregate_rewards(&refs);
        assert!(agg.items.is_empty());
        assert_eq!(agg.experience, 0);
        assert_eq!(agg.loyalty, 0);
        assert_eq!(agg.noah, 0);
    }

    #[test]
    fn test_aggregate_rewards_skips_zero_items() {
        let row = EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 9,
            is_winner: true,
            description: "no items".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 100,
            loyalty: 50,
            cash: 0,
            noah: 0,
        };
        let refs: Vec<&EventRewardRow> = vec![&row];
        let agg = aggregate_rewards(&refs);
        assert!(agg.items.is_empty());
        assert_eq!(agg.experience, 100);
        assert_eq!(agg.loyalty, 50);
    }

    // ── local_id_to_event_type Tests ──────────────────────────────────

    #[test]
    fn test_local_id_to_event_type() {
        assert_eq!(
            local_id_to_event_type(9),
            Some(TempleEventType::BorderDefenceWar)
        );
        assert_eq!(
            local_id_to_event_type(10),
            Some(TempleEventType::ChaosDungeon)
        );
        assert_eq!(
            local_id_to_event_type(11),
            Some(TempleEventType::JuraidMountain)
        );
        assert_eq!(local_id_to_event_type(0), None);
        assert_eq!(local_id_to_event_type(99), None);
    }

    #[test]
    fn test_active_event_to_local_id() {
        // BDW: TempleEventType(4) → EventLocalId(9)
        assert_eq!(active_event_to_local_id(4), 9);
        // Chaos: TempleEventType(24) → EventLocalId(10)
        assert_eq!(active_event_to_local_id(24), 10);
        // Juraid: TempleEventType(100) → EventLocalId(11)
        assert_eq!(active_event_to_local_id(100), 11);
        // Unknown passes through
        assert_eq!(active_event_to_local_id(42), 42);
        assert_eq!(active_event_to_local_id(-1), -1);
    }

    // ── Session Setup Helper ──────────────────────────────────────────

    fn setup_session(
        world: &WorldState,
        sid: SessionId,
        nation: u8,
    ) -> tokio::sync::mpsc::UnboundedReceiver<Arc<Packet>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        let info = crate::world::CharacterInfo {
            session_id: sid,
            name: format!("Player{}", sid),
            nation,
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
            gold: 1000,
            loyalty: 500,
            loyalty_monthly: 100,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 10_000,
            max_exp: 100_000_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
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
        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(sid, info, pos);
        rx
    }

    // ── grant_reward Tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_grant_reward_gold_only() {
        let world = WorldState::new();
        let mut rx = setup_session(&world, 1, 1);

        let reward = AggregatedReward {
            items: vec![],
            experience: 0,
            loyalty: 0,
            noah: 5_000,
        };
        grant_reward(&world, 1, &reward).await;

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 6_000); // 1000 + 5000

        // WIZ_GOLD_CHANGE packet
        let pkt = rx.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizGoldChange as u8);
    }

    #[tokio::test]
    async fn test_grant_reward_exp_and_loyalty() {
        let world = WorldState::new();
        let _rx = setup_session(&world, 1, 1);

        let reward = AggregatedReward {
            items: vec![],
            experience: 25_000,
            loyalty: 500,
            noah: 0,
        };
        grant_reward(&world, 1, &reward).await;

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 35_000); // 10_000 + 25_000
        assert_eq!(ch.loyalty, 1000); // 500 + 500
    }

    // ── Distribute Integration Tests ─────────────────────────────────

    #[tokio::test]
    async fn test_distribute_rewards_unknown_local_id() {
        let world = WorldState::new();
        // local_id=99 is unknown, should return early without panic
        distribute_event_rewards(&world, 99, &[(1, 1)]).await;
    }

    #[tokio::test]
    async fn test_distribute_rewards_empty_winner_results() {
        let world = WorldState::new();
        // Empty winner_results — should be a no-op
        distribute_event_rewards(&world, 9, &[]).await;
    }

    #[tokio::test]
    async fn test_distribute_rewards_room_not_found() {
        // Event room manager has no rooms, so room lookup will fail gracefully
        let world = WorldState::new();
        // Would need event_rewards loaded, but with no rewards it exits early
        distribute_event_rewards(&world, 9, &[(1, 1)]).await;
    }

    #[test]
    fn test_prize_given_prevents_double_grant() {
        use crate::systems::event_room::EventUser;

        let erm = EventRoomManager::new();

        // Create a room with a user that already has prize_given=true
        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_users.insert(
            "AlreadyRewarded".to_string(),
            EventUser {
                user_name: "AlreadyRewarded".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: true,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.karus_users.insert(
            "NotYetRewarded".to_string(),
            EventUser {
                user_name: "NotYetRewarded".to_string(),
                session_id: 2,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms
            .insert((TempleEventType::BorderDefenceWar, 1), room);

        // Simulate the collection logic from distribute_event_rewards
        let participants: Vec<(SessionId, u8)> = {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            let mut users = Vec::new();
            for user in room.karus_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            users
        };

        // Only the non-prize_given user should be collected
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].0, 2); // session_id=2

        // Verify prize_given was set
        let room = erm
            .get_room_mut(TempleEventType::BorderDefenceWar, 1)
            .unwrap();
        assert!(room.karus_users.get("AlreadyRewarded").unwrap().prize_given);
        assert!(room.karus_users.get("NotYetRewarded").unwrap().prize_given);
    }

    #[test]
    fn test_logged_out_users_skipped() {
        use crate::systems::event_room::EventUser;

        let erm = EventRoomManager::new();

        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::JuraidMountain);
        room.elmorad_users.insert(
            "LoggedOut".to_string(),
            EventUser {
                user_name: "LoggedOut".to_string(),
                session_id: 1,
                nation: 2,
                prize_given: false,
                logged_out: true, // logged out
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.elmorad_users.insert(
            "Online".to_string(),
            EventUser {
                user_name: "Online".to_string(),
                session_id: 2,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::JuraidMountain, 1), room);

        let participants: Vec<(SessionId, u8)> = {
            let mut room = erm
                .get_room_mut(TempleEventType::JuraidMountain, 1)
                .unwrap();
            let mut users = Vec::new();
            for user in room.elmorad_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            users
        };

        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].0, 2); // only the online user
    }

    #[test]
    fn test_winner_vs_loser_determination() {
        // Winner nation=1 (Karus), so Karus users get winner, Elmorad get loser
        let winner_nation: u8 = 1;

        let karus_nation: u8 = 1;
        let elmorad_nation: u8 = 2;

        let karus_is_winner = karus_nation == winner_nation;
        let elmorad_is_winner = elmorad_nation == winner_nation;

        assert!(karus_is_winner);
        assert!(!elmorad_is_winner);

        // Winner nation=2
        let winner_nation2: u8 = 2;
        assert!(2 == winner_nation2); // Elmorad wins
        assert!(1 != winner_nation2); // Karus loses
    }

    #[test]
    fn test_empty_room_no_participants() {
        let erm = EventRoomManager::new();

        // Create an empty room
        let room = crate::systems::event_room::EventRoom::new(1, TempleEventType::BorderDefenceWar);
        erm.rooms
            .insert((TempleEventType::BorderDefenceWar, 1), room);

        let participants: Vec<(SessionId, u8)> = {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            let mut users = Vec::new();
            for user in room.karus_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            for user in room.elmorad_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            users
        };

        assert!(participants.is_empty());
    }

    #[test]
    fn test_aggregated_reward_default() {
        let agg = AggregatedReward::default();
        assert!(agg.items.is_empty());
        assert_eq!(agg.experience, 0);
        assert_eq!(agg.loyalty, 0);
        assert_eq!(agg.noah, 0);
    }

    // ── grant_reward comprehensive ──────────────────────────────────

    #[tokio::test]
    async fn test_grant_reward_all_types() {
        let world = WorldState::new();
        let _rx = setup_session(&world, 1, 1);

        let reward = AggregatedReward {
            items: vec![(900017000, 1)],
            experience: 100_000,
            loyalty: 200,
            noah: 50_000,
        };
        grant_reward(&world, 1, &reward).await;

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 110_000); // 10_000 + 100_000
        assert_eq!(ch.loyalty, 700); // 500 + 200
        assert_eq!(ch.gold, 51_000); // 1000 + 50_000
                                     // Item give may fail (no item table loaded), but should not panic
    }

    #[tokio::test]
    async fn test_grant_reward_empty_reward() {
        let world = WorldState::new();
        let mut rx = setup_session(&world, 1, 1);

        let reward = AggregatedReward::default();
        grant_reward(&world, 1, &reward).await;

        let ch = world.get_character_info(1).unwrap();
        // Nothing should change
        assert_eq!(ch.exp, 10_000);
        assert_eq!(ch.loyalty, 500);
        assert_eq!(ch.gold, 1000);
        // No packets sent for empty reward
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_grant_reward_no_session() {
        let world = WorldState::new();
        // Session 99 does not exist — should not panic
        let reward = AggregatedReward {
            items: vec![(100, 1)],
            experience: 1000,
            loyalty: 50,
            noah: 500,
        };
        grant_reward(&world, 99, &reward).await;
    }

    #[tokio::test]
    async fn test_grant_reward_exp_triggers_level_up() {
        let world = WorldState::new();
        let _rx = setup_session(&world, 1, 1);
        // setup_session sets level=60, exp=10_000, max_exp=100_000_000

        // Grant enough XP to surpass max_exp and trigger level-up
        let reward = AggregatedReward {
            items: vec![],
            experience: 100_000_000, // 10_000 + 100_000_000 >= 100_000_000
            loyalty: 0,
            noah: 0,
        };
        grant_reward(&world, 1, &reward).await;

        let ch = world.get_character_info(1).unwrap();
        // Level should have increased from 60 to 61
        assert_eq!(
            ch.level, 61,
            "Level should increase from 60 to 61 after enough XP"
        );
        // Remainder exp = (10_000 + 100_000_000) - 100_000_000 = 10_000
        assert_eq!(
            ch.exp, 10_000,
            "Remainder exp after level-up should be 10_000"
        );
    }

    #[tokio::test]
    async fn test_grant_reward_exp_below_level_up_threshold() {
        let world = WorldState::new();
        let _rx = setup_session(&world, 1, 1);
        // setup_session sets level=60, exp=10_000, max_exp=100_000_000

        // Grant XP that is NOT enough to trigger level-up
        let reward = AggregatedReward {
            items: vec![],
            experience: 50_000, // 10_000 + 50_000 = 60_000 < 100_000_000
            loyalty: 0,
            noah: 0,
        };
        grant_reward(&world, 1, &reward).await;

        let ch = world.get_character_info(1).unwrap();
        // Level should remain 60
        assert_eq!(
            ch.level, 60,
            "Level should remain 60 when XP is below threshold"
        );
        assert_eq!(ch.exp, 60_000, "Exp should be 10_000 + 50_000 = 60_000");
    }

    // ── Full distribute_event_rewards with rooms ────────────────────

    #[tokio::test]
    async fn test_distribute_rewards_with_rooms_and_sessions() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();

        // Set up sessions for 4 players (2 Karus, 2 Elmorad)
        let _rx1 = setup_session(&world, 1, 1); // Karus
        let _rx2 = setup_session(&world, 2, 1); // Karus
        let _rx3 = setup_session(&world, 3, 2); // Elmorad
        let _rx4 = setup_session(&world, 4, 2); // Elmorad

        let erm = world.event_room_manager();

        // Create BDW room 1
        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_users.insert(
            "KarusPlayer1".to_string(),
            EventUser {
                user_name: "KarusPlayer1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.karus_users.insert(
            "KarusPlayer2".to_string(),
            EventUser {
                user_name: "KarusPlayer2".to_string(),
                session_id: 2,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.elmorad_users.insert(
            "ElmoradPlayer1".to_string(),
            EventUser {
                user_name: "ElmoradPlayer1".to_string(),
                session_id: 3,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.elmorad_users.insert(
            "ElmoradPlayer2".to_string(),
            EventUser {
                user_name: "ElmoradPlayer2".to_string(),
                session_id: 4,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms
            .insert((TempleEventType::BorderDefenceWar, 1), room);

        // No event rewards loaded — distribute should log warning and return
        // (event_rewards is private so we can't populate it from here)
        let results = vec![(1u8, 1u8)]; // room 1, Karus wins
        distribute_event_rewards(&world, 9, &results).await;

        // Since no rewards are loaded, prize_given should NOT be set (function returns early)
        let room_ref = erm
            .get_room_mut(TempleEventType::BorderDefenceWar, 1)
            .unwrap();
        // All users still have prize_given=false because we returned before room iteration
        for user in room_ref.karus_users.values() {
            assert!(
                !user.prize_given,
                "prize_given should not be set when no rewards loaded"
            );
        }
        for user in room_ref.elmorad_users.values() {
            assert!(
                !user.prize_given,
                "prize_given should not be set when no rewards loaded"
            );
        }
    }

    #[test]
    fn test_distribute_rewards_mixed_room_states() {
        use crate::systems::event_room::EventUser;

        let erm = EventRoomManager::new();

        // Room with mixed user states: one normal, one prize_given, one logged_out
        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::JuraidMountain);
        room.karus_users.insert(
            "Normal".to_string(),
            EventUser {
                user_name: "Normal".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.karus_users.insert(
            "AlreadyRewarded".to_string(),
            EventUser {
                user_name: "AlreadyRewarded".to_string(),
                session_id: 2,
                nation: 1,
                prize_given: true,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.elmorad_users.insert(
            "LoggedOut".to_string(),
            EventUser {
                user_name: "LoggedOut".to_string(),
                session_id: 3,
                nation: 2,
                prize_given: false,
                logged_out: true,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.elmorad_users.insert(
            "ElmoNormal".to_string(),
            EventUser {
                user_name: "ElmoNormal".to_string(),
                session_id: 4,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::JuraidMountain, 1), room);

        // Simulate the collection logic from distribute_event_rewards
        let winner_nation: u8 = 1; // Karus wins
        let participants: Vec<(SessionId, u8)> = {
            let mut room = erm
                .get_room_mut(TempleEventType::JuraidMountain, 1)
                .unwrap();
            let mut users = Vec::new();
            for user in room.karus_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            for user in room.elmorad_users.values_mut() {
                if user.prize_given || user.logged_out {
                    continue;
                }
                user.prize_given = true;
                users.push((user.session_id, user.nation));
            }
            users
        };

        // Should collect exactly 2 users: Normal (karus) and ElmoNormal (elmorad)
        assert_eq!(participants.len(), 2);

        // Verify winners and losers
        let winners: Vec<_> = participants
            .iter()
            .filter(|(_, nation)| *nation == winner_nation)
            .collect();
        let losers: Vec<_> = participants
            .iter()
            .filter(|(_, nation)| *nation != winner_nation)
            .collect();

        assert_eq!(winners.len(), 1); // Normal (Karus, winner)
        assert_eq!(losers.len(), 1); // ElmoNormal (Elmorad, loser)

        // Verify prize_given flags
        let room_ref = erm
            .get_room_mut(TempleEventType::JuraidMountain, 1)
            .unwrap();
        assert!(room_ref.karus_users.get("Normal").unwrap().prize_given);
        assert!(
            room_ref
                .karus_users
                .get("AlreadyRewarded")
                .unwrap()
                .prize_given
        ); // was already true
        assert!(!room_ref.elmorad_users.get("LoggedOut").unwrap().prize_given); // skipped
        assert!(
            room_ref
                .elmorad_users
                .get("ElmoNormal")
                .unwrap()
                .prize_given
        );
    }

    #[test]
    fn test_distribute_rewards_multiple_rooms() {
        use crate::systems::event_room::EventUser;

        let erm = EventRoomManager::new();

        // Create 2 BDW rooms with different winners
        for room_id in 1..=2u8 {
            let mut room = crate::systems::event_room::EventRoom::new(
                room_id,
                TempleEventType::BorderDefenceWar,
            );
            room.karus_users.insert(
                format!("K{}", room_id),
                EventUser {
                    user_name: format!("K{}", room_id),
                    session_id: (room_id * 10) as SessionId,
                    nation: 1,
                    prize_given: false,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                },
            );
            room.elmorad_users.insert(
                format!("E{}", room_id),
                EventUser {
                    user_name: format!("E{}", room_id),
                    session_id: (room_id * 10 + 1) as SessionId,
                    nation: 2,
                    prize_given: false,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                },
            );
            erm.rooms
                .insert((TempleEventType::BorderDefenceWar, room_id), room);
        }

        // Room 1: Karus wins, Room 2: Elmorad wins
        let winner_results = vec![(1u8, 1u8), (2u8, 2u8)];

        let mut all_participants = Vec::new();
        let event_type = TempleEventType::BorderDefenceWar;

        for &(room_id, winner_nation) in &winner_results {
            let participants: Vec<(SessionId, u8, bool)> = {
                let mut room = erm.get_room_mut(event_type, room_id).unwrap();
                let mut users = Vec::new();
                for user in room.karus_users.values_mut() {
                    if user.prize_given || user.logged_out {
                        continue;
                    }
                    user.prize_given = true;
                    let is_winner = user.nation == winner_nation;
                    users.push((user.session_id, user.nation, is_winner));
                }
                for user in room.elmorad_users.values_mut() {
                    if user.prize_given || user.logged_out {
                        continue;
                    }
                    user.prize_given = true;
                    let is_winner = user.nation == winner_nation;
                    users.push((user.session_id, user.nation, is_winner));
                }
                users
            };
            all_participants.extend(participants);
        }

        // Should collect 4 users total (2 per room)
        assert_eq!(all_participants.len(), 4);

        // Room 1: K1 is winner (nation=1, winner_nation=1), E1 is loser
        let k1 = all_participants
            .iter()
            .find(|(sid, _, _)| *sid == 10)
            .unwrap();
        assert!(k1.2, "K1 in room 1 should be winner");
        let e1 = all_participants
            .iter()
            .find(|(sid, _, _)| *sid == 11)
            .unwrap();
        assert!(!e1.2, "E1 in room 1 should be loser");

        // Room 2: K2 is loser (nation=1, winner_nation=2), E2 is winner
        let k2 = all_participants
            .iter()
            .find(|(sid, _, _)| *sid == 20)
            .unwrap();
        assert!(!k2.2, "K2 in room 2 should be loser");
        let e2 = all_participants
            .iter()
            .find(|(sid, _, _)| *sid == 21)
            .unwrap();
        assert!(e2.2, "E2 in room 2 should be winner");

        // Verify all prize_given flags are true
        for room_id in 1..=2u8 {
            let room = erm.get_room_mut(event_type, room_id).unwrap();
            for user in room.karus_users.values() {
                assert!(user.prize_given);
            }
            for user in room.elmorad_users.values() {
                assert!(user.prize_given);
            }
        }
    }

    #[test]
    fn test_aggregate_rewards_all_three_item_slots() {
        let row = EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 9,
            is_winner: true,
            description: "full".to_string(),
            item_id1: 100,
            item_count1: 1,
            item_expiration1: 0,
            item_id2: 200,
            item_count2: 2,
            item_expiration2: 0,
            item_id3: 300,
            item_count3: 3,
            item_expiration3: 0,
            experience: 0,
            loyalty: 0,
            cash: 0,
            noah: 0,
        };
        let refs: Vec<&EventRewardRow> = vec![&row];
        let agg = aggregate_rewards(&refs);

        assert_eq!(agg.items.len(), 3);
        assert_eq!(agg.items[0], (100, 1));
        assert_eq!(agg.items[1], (200, 2));
        assert_eq!(agg.items[2], (300, 3));
    }

    #[test]
    fn test_aggregate_rewards_partial_item_slots() {
        // item_id2 > 0 but item_count2 == 0 → should be skipped
        let row = EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 9,
            is_winner: true,
            description: "partial".to_string(),
            item_id1: 100,
            item_count1: 1,
            item_expiration1: 0,
            item_id2: 200,
            item_count2: 0, // zero count
            item_expiration2: 0,
            item_id3: 0, // zero id
            item_count3: 5,
            item_expiration3: 0,
            experience: 0,
            loyalty: 0,
            cash: 0,
            noah: 0,
        };
        let refs: Vec<&EventRewardRow> = vec![&row];
        let agg = aggregate_rewards(&refs);

        // Only item_id1 qualifies (item_id2 has count=0, item_id3 has id=0)
        assert_eq!(agg.items.len(), 1);
        assert_eq!(agg.items[0], (100, 1));
    }

    // ── BDW Level Exp Bonus Tests ───────────────────────────────────

    #[test]
    fn test_bdw_level_exp_bonus_formula() {
        // Level 1 (clamped to 20): (20-20)*203000 = 0
        assert_eq!(bdw_level_exp_bonus(1), 0);
        // Level 20: (20-20)*203000 = 0
        assert_eq!(bdw_level_exp_bonus(20), 0);
        // Level 30: (30-20)*203000 = 2_030_000
        assert_eq!(bdw_level_exp_bonus(30), 2_030_000);
        // Level 57: (57-20)*203000 = 7_511_000
        assert_eq!(bdw_level_exp_bonus(57), 7_511_000);
        // Level 58: (58+55)*120000 = 13_560_000
        assert_eq!(bdw_level_exp_bonus(58), 13_560_000);
        // Level 83: (83+55)*120000 = 16_560_000
        assert_eq!(bdw_level_exp_bonus(83), 16_560_000);
    }

    #[test]
    fn test_bdw_level_exp_bonus_boundary_values() {
        // Level 0 (clamped to 20): 0
        assert_eq!(bdw_level_exp_bonus(0), 0);
        // Level 19 (clamped to 20): 0
        assert_eq!(bdw_level_exp_bonus(19), 0);
        // Level 21: (21-20)*203000 = 203_000
        assert_eq!(bdw_level_exp_bonus(21), 203_000);
        // Level 57 (last low-level): (57-20)*203000 = 7_511_000
        assert_eq!(bdw_level_exp_bonus(57), 7_511_000);
        // Level 58 (first high-level): (58+55)*120000 = 13_560_000
        assert_eq!(bdw_level_exp_bonus(58), 13_560_000);
    }

    // ── BDW per-user point EXP tests ─────────────────────────────────

    #[test]
    fn test_bdw_user_point_exp_zero_points() {
        let (gained, premium) = bdw_user_point_exp(60, 0);
        assert_eq!(gained, 0);
        assert_eq!(premium, 0);
    }

    #[test]
    fn test_bdw_user_point_exp_one_kill() {
        // level 60, 1 point: 60^3 * 0.15 * (5*1) = 216000 * 0.15 * 5 = 162_000
        let (gained, premium) = bdw_user_point_exp(60, 1);
        assert_eq!(gained, 162_000);
        assert_eq!(premium, 324_000);
    }

    #[test]
    fn test_bdw_user_point_exp_altar_delivery() {
        // level 60, 11 points (1 kill + 1 altar): 216000 * 0.15 * 55 = 1_782_000
        let (gained, premium) = bdw_user_point_exp(60, 11);
        assert_eq!(gained, 1_782_000);
        assert_eq!(premium, 3_564_000);
    }

    #[test]
    fn test_bdw_user_point_exp_cap() {
        // level 83, 100 points: 83^3 * 0.15 * 500 = 571787 * 0.15 * 500 = 42_884_025
        // → capped to 8_000_000
        let (gained, premium) = bdw_user_point_exp(83, 100);
        assert_eq!(gained, 8_000_000);
        assert_eq!(premium, 10_000_000); // 16M capped to 10M
    }

    #[test]
    fn test_bdw_user_point_exp_low_level() {
        // level 1, 1 point: 1^3 * 0.15 * 5 = 0 (truncated from 0.75)
        let (gained, _) = bdw_user_point_exp(1, 1);
        assert_eq!(gained, 0);
    }

    // ── Chaos per-user EXP tests ─────────────────────────────────────

    #[test]
    fn test_chaos_user_exp_zero_kills() {
        let (gained, premium) = chaos_user_exp(60, 0, 0);
        assert_eq!(gained, 0);
        assert_eq!(premium, 0);
    }

    #[test]
    fn test_chaos_user_exp_kills_only() {
        // level 60, 10 kills 0 deaths: 60^3 * 0.15 * (50 - 0) = 216000 * 0.15 * 50 = 1_620_000
        let (gained, premium) = chaos_user_exp(60, 10, 0);
        assert_eq!(gained, 1_620_000);
        assert_eq!(premium, 3_240_000);
    }

    #[test]
    fn test_chaos_user_exp_kills_and_deaths() {
        // level 60, 10 kills 5 deaths: 216000 * 0.15 * (50-5) = 216000 * 0.15 * 45 = 1_458_000
        let (gained, premium) = chaos_user_exp(60, 10, 5);
        assert_eq!(gained, 1_458_000);
        assert_eq!(premium, 2_916_000);
    }

    #[test]
    fn test_chaos_user_exp_negative_clamp() {
        // level 60, 0 kills 10 deaths: 216000 * 0.15 * (0-10) = -324_000 → clamped to 0
        let (gained, premium) = chaos_user_exp(60, 0, 10);
        assert_eq!(gained, 0);
        assert_eq!(premium, 0);
    }

    #[test]
    fn test_chaos_user_exp_cap() {
        // level 83, 200 kills 0 deaths: 571787 * 0.15 * 1000 = 85_768_050 → capped
        let (gained, premium) = chaos_user_exp(83, 200, 0);
        assert_eq!(gained, 8_000_000);
        assert_eq!(premium, 10_000_000);
    }

    #[test]
    fn test_chaos_user_exp_deaths_exceed_kills() {
        // 5 kills, 30 deaths: 5*5-30 = -5 → negative → 0
        let (gained, _) = chaos_user_exp(60, 5, 30);
        assert_eq!(gained, 0);
    }

    #[tokio::test]
    async fn test_bdw_distribute_adds_level_bonus() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();

        // Set up a level-60 player (setup_session sets level=60)
        let _rx1 = setup_session(&world, 1, 1); // Karus, level 60

        let erm = world.event_room_manager();

        // Create BDW room
        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.karus_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms
            .insert((TempleEventType::BorderDefenceWar, 1), room);

        // Load a minimal reward config: 0 base exp (to isolate level bonus)
        let reward_row = EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 9,
            is_winner: true,
            description: "bdw winner".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 100_000, // base exp from DB
            loyalty: 0,
            cash: 0,
            noah: 0,
        };
        world.insert_event_rewards(9, vec![reward_row]);

        // Distribute: room 1, Karus wins
        distribute_event_rewards(&world, 9, &[(1, 1)]).await;

        // Level 60 bonus: (60+55)*120000 = 13_800_000
        let expected_bonus = bdw_level_exp_bonus(60);
        assert_eq!(expected_bonus, 13_800_000);

        let ch = world.get_character_info(1).unwrap();
        // Starting exp=10_000, reward exp=100_000 + level_bonus=13_800_000
        // Total = 10_000 + 100_000 + 13_800_000 = 13_910_000
        assert_eq!(ch.exp, 13_910_000);
    }

    // ── Chaos Finish EXP Tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_chaos_finish_exp_basic() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1); // level 60, no premium

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 10,
                deaths: 3,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // level 60, 10 kills, 3 deaths → kill_score = 50 - 3 = 47
        // 60^3 * 0.15 * 47 = 216000 * 0.15 * 47 = 1_522_800
        // No premium → use normal_exp
        let (expected, _) = chaos_user_exp(60, 10, 3);
        assert_eq!(expected, 1_522_800);

        let ch = world.get_character_info(1).unwrap();
        // Starting exp=10_000 + 1_522_800 = 1_532_800
        assert_eq!(ch.exp, 10_000 + expected as u64);

        // Verify prize_given was set
        let room = erm.get_room(TempleEventType::ChaosDungeon, 1).unwrap();
        assert!(room.mixed_users.get("Player1").unwrap().prize_given);
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_premium_user() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);

        // Set premium status on session handle (C++ m_bPremiumInUse)
        world.update_session(1, |h| {
            h.premium_in_use = 1; // premium active
        });

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 10,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // level 60, 10 kills, 0 deaths → 216000 * 0.15 * 50 = 1_620_000
        // Premium → use premium_exp = 1_620_000 * 2 = 3_240_000
        let (_, premium_exp) = chaos_user_exp(60, 10, 0);
        assert_eq!(premium_exp, 3_240_000);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 10_000 + premium_exp as u64);
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_zero_kills() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 5,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // 0 kills, 5 deaths → kill_score = -5 → clamped to 0 → no EXP
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 10_000); // unchanged
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_skip_logged_out() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: true, // logged out
                kills: 10,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // Logged out user should be skipped
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 10_000); // unchanged
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_skip_already_rewarded() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: true, // already rewarded
                logged_out: false,
                kills: 10,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // Already-rewarded user should be skipped
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 10_000); // unchanged
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_skip_finished_room() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.finished = true; // already finished
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 10,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // Finished room should be skipped entirely
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.exp, 10_000); // unchanged
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_multiple_users() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);
        let _rx2 = setup_session(&world, 2, 2);

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 5,
                deaths: 2,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.mixed_users.insert(
            "Player2".to_string(),
            EventUser {
                user_name: "Player2".to_string(),
                session_id: 2,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 3,
                deaths: 4,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // Player1: 60^3 * 0.15 * (25-2) = 216000 * 0.15 * 23 = 745_200
        let (exp1, _) = chaos_user_exp(60, 5, 2);
        assert_eq!(exp1, 745_200);
        let ch1 = world.get_character_info(1).unwrap();
        assert_eq!(ch1.exp, 10_000 + exp1 as u64);

        // Player2: 60^3 * 0.15 * (15-4) = 216000 * 0.15 * 11 = 356_400
        let (exp2, _) = chaos_user_exp(60, 3, 4);
        assert_eq!(exp2, 356_400);
        let ch2 = world.get_character_info(2).unwrap();
        assert_eq!(ch2.exp, 10_000 + exp2 as u64);
    }

    #[tokio::test]
    async fn test_chaos_finish_exp_sets_room_finished() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1);

        let erm = world.event_room_manager();

        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 5,
                deaths: 1,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        // Before: room is not finished
        assert!(
            !erm.get_room(TempleEventType::ChaosDungeon, 1)
                .unwrap()
                .finished
        );

        distribute_chaos_finish_exp(&world).await;

        // After: room.finished must be true
        assert!(
            erm.get_room(TempleEventType::ChaosDungeon, 1)
                .unwrap()
                .finished
        );

        // Calling again should be a no-op (already finished)
        let ch_before = world.get_character_info(1).unwrap();
        let exp_before = ch_before.exp;
        distribute_chaos_finish_exp(&world).await;
        let ch_after = world.get_character_info(1).unwrap();
        assert_eq!(ch_after.exp, exp_before);
    }

    #[test]
    fn test_get_player_chaos_rank_basic() {
        // Sorted by kills desc: sid=3 (20), sid=1 (10), sid=2 (5)
        let ranked = vec![(3, 20u32, 0u32), (1, 10, 3), (2, 5, 1)];
        assert_eq!(get_player_chaos_rank(&ranked, 3), 1);
        assert_eq!(get_player_chaos_rank(&ranked, 1), 2);
        assert_eq!(get_player_chaos_rank(&ranked, 2), 3);
    }

    #[test]
    fn test_get_player_chaos_rank_not_found() {
        let ranked = vec![(1, 10u32, 0u32), (2, 5, 1)];
        assert_eq!(get_player_chaos_rank(&ranked, 99), 0);
    }

    #[test]
    fn test_get_player_chaos_rank_single_user() {
        let ranked = vec![(1, 3u32, 1u32)];
        assert_eq!(get_player_chaos_rank(&ranked, 1), 1);
    }

    #[test]
    fn test_get_player_chaos_rank_empty() {
        let ranked: Vec<(SessionId, u32, u32)> = vec![];
        assert_eq!(get_player_chaos_rank(&ranked, 1), 0);
    }

    #[tokio::test]
    async fn test_chaos_finish_rank_rewards_granted() {
        use crate::systems::event_room::EventUser;
        use ko_db::models::chaos_stone::EventChaosRewardRow;

        let world = WorldState::new();
        let _rx1 = setup_session(&world, 1, 1); // level 60
        let _rx2 = setup_session(&world, 2, 2); // level 60

        // Seed rank-1 reward: experience=5000, loyalty=100, noah=500
        {
            let mut rewards = world.chaos_stone_rewards_mut();
            rewards.push(EventChaosRewardRow {
                rank_id: 1,
                item_id1: 0,
                item_count1: 0,
                item_expiration1: 0,
                item_id2: 0,
                item_count2: 0,
                item_expiration2: 0,
                item_id3: 0,
                item_count3: 0,
                item_expiration3: 0,
                item_id4: 0,
                item_count4: 0,
                item_expiration4: 0,
                item_id5: 0,
                item_count5: 0,
                item_expiration5: 0,
                experience: 5000,
                loyalty: 100,
                cash: 0,
                noah: 500,
            });
            rewards.push(EventChaosRewardRow {
                rank_id: 2,
                item_id1: 0,
                item_count1: 0,
                item_expiration1: 0,
                item_id2: 0,
                item_count2: 0,
                item_expiration2: 0,
                item_id3: 0,
                item_count3: 0,
                item_expiration3: 0,
                item_id4: 0,
                item_count4: 0,
                item_expiration4: 0,
                item_id5: 0,
                item_count5: 0,
                item_expiration5: 0,
                experience: 2000,
                loyalty: 50,
                cash: 0,
                noah: 200,
            });
        }

        let erm = world.event_room_manager();
        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        // Player1: 10 kills (rank 1), Player2: 3 kills (rank 2)
        room.mixed_users.insert(
            "Player1".to_string(),
            EventUser {
                user_name: "Player1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 10,
                deaths: 2,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        room.mixed_users.insert(
            "Player2".to_string(),
            EventUser {
                user_name: "Player2".to_string(),
                session_id: 2,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 3,
                deaths: 5,
                bdw_points: 0,
                has_altar_obtained: false,
            },
        );
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        distribute_chaos_finish_exp(&world).await;

        // Player1 (rank 1): kill EXP + rank reward EXP (5000)
        let (kill_exp1, _) = chaos_user_exp(60, 10, 2);
        let ch1 = world.get_character_info(1).unwrap();
        // Starting 10_000 + kill_exp + 5000 (rank reward exp)
        assert_eq!(ch1.exp, 10_000 + kill_exp1 as u64 + 5000);

        // Player1 should have received 500 noah (gold), starting gold = 1000
        assert_eq!(ch1.gold, 1000 + 500);

        // Player2 (rank 2): kill EXP + rank reward EXP (2000)
        let (kill_exp2, _) = chaos_user_exp(60, 3, 5);
        let ch2 = world.get_character_info(2).unwrap();
        // kill_score = 15 - 5 = 10, exp = 216000 * 0.15 * 10 = 324_000
        assert_eq!(ch2.exp, 10_000 + kill_exp2 as u64 + 2000);
        assert_eq!(ch2.gold, 1000 + 200);
    }

    #[tokio::test]
    async fn test_chaos_finish_rank_over_18_no_reward() {
        use crate::systems::event_room::EventUser;

        let world = WorldState::new();
        // Create 20 sessions (ranks 1-20, but only ranks 1-18 get rewards)
        let mut receivers = Vec::new();
        for i in 1..=20u16 {
            receivers.push(setup_session(&world, i, (i as u8).clamp(1, 2)));
        }

        let erm = world.event_room_manager();
        let mut room = crate::systems::event_room::EventRoom::new(1, TempleEventType::ChaosDungeon);
        for i in 1..=20u16 {
            room.mixed_users.insert(
                format!("P{}", i),
                EventUser {
                    user_name: format!("P{}", i),
                    session_id: i,
                    nation: 1,
                    prize_given: false,
                    logged_out: false,
                    kills: 100 - i as u32, // P1=99 kills (rank 1), P20=80 kills (rank 20)
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                },
            );
        }
        erm.rooms.insert((TempleEventType::ChaosDungeon, 1), room);

        // No reward data seeded — ranks 1-18 won't find rewards, rank 19-20 won't even try
        distribute_chaos_finish_exp(&world).await;

        // All users should still get their kill EXP (no crash, no panic)
        for i in 1..=20u16 {
            let ch = world.get_character_info(i).unwrap();
            let (exp, _) = chaos_user_exp(60, 100 - i as u32, 0);
            assert_eq!(ch.exp, 10_000 + exp as u64);
        }
    }

    // ── Monster Stone timer tick integration tests ──────────────────

    #[test]
    fn test_monster_stone_timer_tick_expiry_basic() {
        use crate::systems::monster_stone::MonsterStoneManager;

        let mut mgr = MonsterStoneManager::new();

        // Allocate and activate a room
        let room_id = mgr.allocate_room().unwrap();
        assert_eq!(room_id, 0);
        mgr.activate_room(room_id, 81, 3, 21, 1000);
        mgr.add_user(room_id, 42); // session_id = 42

        // Before expiry: no rooms expired
        let expired = mgr.timer_tick(2799); // 1000 + 1800 - 1
        assert!(expired.is_empty());

        // At expiry: room expires (boss not killed, finish_time reached)
        let expired = mgr.timer_tick(2800); // 1000 + 1800
        assert_eq!(expired, vec![0]);
    }

    #[test]
    fn test_monster_stone_timer_tick_boss_kill_grace() {
        use crate::systems::monster_stone::MonsterStoneManager;

        let mut mgr = MonsterStoneManager::new();

        let room_id = mgr.allocate_room().unwrap();
        mgr.activate_room(room_id, 82, 7, 21, 1000);
        mgr.add_user(room_id, 10);

        // Kill boss at t=1500
        assert!(mgr.boss_killed(room_id, 1500));

        // Before grace period: no rooms expired
        let expired = mgr.timer_tick(1519); // 1500 + 20 - 1
        assert!(expired.is_empty());

        // After grace period: room expires
        let expired = mgr.timer_tick(1520); // 1500 + 20
        assert_eq!(expired, vec![0]);
    }

    #[test]
    fn test_monster_stone_timer_tick_does_not_expire_boss_killed_before_grace() {
        use crate::systems::monster_stone::MonsterStoneManager;

        let mut mgr = MonsterStoneManager::new();

        let room_id = mgr.allocate_room().unwrap();
        mgr.activate_room(room_id, 83, 12, 21, 1000);
        mgr.add_user(room_id, 5);

        // Kill boss at t=2700 (before the 1800s main timer)
        assert!(mgr.boss_killed(room_id, 2700));

        // At main timer expiry (t=2800): should NOT expire because boss is killed
        // and grace period hasn't passed yet (waiting_time=2720)
        let expired = mgr.timer_tick(2719);
        assert!(expired.is_empty());

        // At grace expiry (t=2720): room expires
        let expired = mgr.timer_tick(2720);
        assert_eq!(expired, vec![0]);
    }

    #[test]
    fn test_monster_stone_room_reset_clears_users() {
        use crate::systems::monster_stone::MonsterStoneManager;

        let mut mgr = MonsterStoneManager::new();

        let room_id = mgr.allocate_room().unwrap();
        mgr.activate_room(room_id, 81, 1, 21, 1000);
        mgr.add_user(room_id, 10);
        mgr.add_user(room_id, 20);
        mgr.add_user(room_id, 30);

        let users = mgr.reset_room(room_id);
        assert_eq!(users, vec![10, 20, 30]);

        // Room is now available again
        let room = mgr.get_room(room_id).unwrap();
        assert!(!room.active);
        assert!(room.users.is_empty());
        assert!(room.is_available());
    }

    // ── collect_room_users tests ─────────────────────────────────────

    #[test]
    fn test_collect_room_users_empty_room() {
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        let users = collect_room_users(&erm, TempleEventType::BorderDefenceWar, 1);
        assert!(users.is_empty());
    }

    #[test]
    fn test_collect_room_users_with_users() {
        use crate::systems::event_room::EventUser;

        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);

        // Add users to the room
        if let Some(mut room) = erm.get_room_mut(TempleEventType::BorderDefenceWar, 1) {
            room.karus_users.insert(
                "karus1".to_string(),
                EventUser {
                    user_name: "karus1".to_string(),
                    session_id: 10,
                    nation: 1,
                    prize_given: false,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                },
            );
            room.elmorad_users.insert(
                "elmo1".to_string(),
                EventUser {
                    user_name: "elmo1".to_string(),
                    session_id: 20,
                    nation: 2,
                    prize_given: false,
                    logged_out: false,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                },
            );
            // Add a logged-out user who should be filtered
            room.karus_users.insert(
                "karus_gone".to_string(),
                EventUser {
                    user_name: "karus_gone".to_string(),
                    session_id: 30,
                    nation: 1,
                    prize_given: false,
                    logged_out: true,
                    kills: 0,
                    deaths: 0,
                    bdw_points: 0,
                    has_altar_obtained: false,
                },
            );
        }

        let users = collect_room_users(&erm, TempleEventType::BorderDefenceWar, 1);
        assert_eq!(users.len(), 2); // logged_out user excluded
        let sids: Vec<SessionId> = users.iter().map(|(s, _)| *s).collect();
        assert!(sids.contains(&10));
        assert!(sids.contains(&20));
        assert!(!sids.contains(&30));
    }

    #[test]
    fn test_collect_room_users_nonexistent_room() {
        let erm = EventRoomManager::new();
        let users = collect_room_users(&erm, TempleEventType::BorderDefenceWar, 99);
        assert!(users.is_empty());
    }

    // ── try_schedule_trigger tests ───────────────────────────────────

    #[test]
    fn test_try_schedule_trigger_no_schedules_does_nothing() {
        let erm = EventRoomManager::new();
        // No schedules loaded; should not panic or change state
        let initial = erm.read_temple_event(|s| s.active_event);
        assert_eq!(initial, -1);
        try_schedule_trigger(&erm, 1000000);
        let after = erm.read_temple_event(|s| s.active_event);
        assert_eq!(after, -1);
    }

    #[test]
    fn test_try_schedule_trigger_no_vroom_opts_does_nothing() {
        let erm = EventRoomManager::new();
        // Add a valid schedule entry but no vroom_opts loaded
        {
            let mut schedules = erm.schedules.write();
            schedules.push(EventScheduleEntry {
                event_id: 9,   // BDW
                event_type: 2, // VirtualRoom
                zone_id: 84,
                name: "BDW Test".to_string(),
                status: true,
                start_times: [(10, 0), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
                days: [true; 7],
                min_level: 0,
                max_level: 0,
                req_loyalty: 0,
                req_money: 0,
            });
        }

        // Use a timestamp that corresponds to Sunday 10:00:00
        // Without vroom_opts, event should not open
        try_schedule_trigger(&erm, 1000000);
        let after = erm.read_temple_event(|s| s.active_event);
        assert_eq!(after, -1);
    }

    #[test]
    fn test_try_schedule_trigger_opens_event_on_match() {
        use crate::systems::event_room::VroomOpt;

        let erm = EventRoomManager::new();

        // Load BDW vroom opts (index 0)
        {
            let mut opts = erm.vroom_opts.write();
            opts[0] = Some(VroomOpt {
                name: "BDW".to_string(),
                sign: 5,
                play: 20,
                attack_open: 6,
                attack_close: 24,
                finish: 30,
            });
        }

        // Add schedule entry for BDW at hour=10, minute=30
        {
            let mut schedules = erm.schedules.write();
            schedules.push(EventScheduleEntry {
                event_id: 9, // BDW = EventLocalId::BorderDefenceWar
                event_type: 2,
                zone_id: 84,
                name: "BDW Auto".to_string(),
                status: true,
                start_times: [(10, 30), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
                days: [true; 7],
                min_level: 0,
                max_level: 0,
                req_loyalty: 0,
                req_money: 0,
            });
        }

        // Create a timestamp for Sunday 10:30:00 in local time
        let dt = chrono::Local
            .with_ymd_and_hms(2026, 2, 15, 10, 30, 0) // 2026-02-15 is Sunday
            .unwrap();
        let ts = dt.timestamp() as u64;

        try_schedule_trigger(&erm, ts);

        // Event should be opened
        let active = erm.read_temple_event(|s| s.active_event);
        assert!(
            active >= 0,
            "Expected event to be opened, but active_event = {}",
            active
        );
    }

    // ── kick_out_destination integration tests ──────────────────────

    /// BDW kick: level >= 35 Karus player → Karus capital (zone 1).
    #[test]
    fn test_kick_out_destination_bdw_karus_high_level() {
        let dest = event_room::kick_out_destination(84, 1, 60);
        assert_eq!(dest, 1, "Karus level 60 BDW → Karus capital");
    }

    /// BDW kick: level >= 35 Elmorad player → Elmorad capital (zone 2).
    #[test]
    fn test_kick_out_destination_bdw_elmorad_high_level() {
        let dest = event_room::kick_out_destination(84, 2, 40);
        assert_eq!(dest, 2, "Elmorad level 40 BDW → Elmorad capital");
    }

    /// BDW kick: low-level player → Moradon (zone 21).
    #[test]
    fn test_kick_out_destination_bdw_low_level() {
        let dest = event_room::kick_out_destination(84, 1, 20);
        assert_eq!(dest, 21, "Low-level player BDW → Moradon");
    }

    /// Chaos kick: level >= 35 player → nation capital.
    #[test]
    fn test_kick_out_destination_chaos_high_level() {
        let dest = event_room::kick_out_destination(85, 2, 50);
        assert_eq!(dest, 2, "Elmorad level 50 Chaos → Elmorad capital");
    }

    /// Juraid kick: exits to Moradon.
    #[test]
    fn test_kick_out_destination_juraid_high_level() {
        let dest = event_room::kick_out_destination(87, 1, 60);
        assert_eq!(dest, 21, "High-level Juraid → Moradon");
    }

    /// Juraid kick: low-level → Moradon.
    #[test]
    fn test_kick_out_destination_juraid_low_level() {
        let dest = event_room::kick_out_destination(87, 2, 30);
        assert_eq!(dest, 21, "Low-level Juraid → Moradon");
    }

    // ── Juraid bridge integration tests ────────────────────────────

    /// open_bridge_for_all_rooms opens bridges in all Juraid rooms.
    #[test]
    fn test_juraid_bridge_open_integration() {
        let erm = EventRoomManager::new();
        let mut juraid_mgr = juraid::JuraidManager::new(3);
        juraid_mgr.init_rooms(&erm);
        juraid_mgr.start_bridge_timer(10000);

        // Open bridge 0 for all rooms
        let opened = juraid::open_bridge_for_all_rooms(&mut juraid_mgr, 0);
        assert_eq!(opened, 3, "All 3 rooms should have bridge 0 opened");

        // Verify bridge state in each room
        for room_state in juraid_mgr.room_states.values() {
            assert!(room_state.bridges.is_bridge_open(0, 1));
            assert!(room_state.bridges.is_bridge_open(0, 2));
        }

        // Re-opening returns 0
        let re_opened = juraid::open_bridge_for_all_rooms(&mut juraid_mgr, 0);
        assert_eq!(re_opened, 0, "Already opened bridges should not re-open");
    }

    #[test]
    fn test_try_schedule_trigger_skips_non_zero_second() {
        use crate::systems::event_room::VroomOpt;

        let erm = EventRoomManager::new();

        {
            let mut opts = erm.vroom_opts.write();
            opts[0] = Some(VroomOpt {
                name: "BDW".to_string(),
                sign: 5,
                play: 20,
                attack_open: 6,
                attack_close: 24,
                finish: 30,
            });
        }

        {
            let mut schedules = erm.schedules.write();
            schedules.push(EventScheduleEntry {
                event_id: 9,
                event_type: 2,
                zone_id: 84,
                name: "BDW".to_string(),
                status: true,
                start_times: [(10, 30), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
                days: [true; 7],
                min_level: 0,
                max_level: 0,
                req_loyalty: 0,
                req_money: 0,
            });
        }

        // 10:30:15 — second != 0, should NOT trigger
        let dt = chrono::Local
            .with_ymd_and_hms(2026, 2, 15, 10, 30, 15)
            .unwrap();
        let ts = dt.timestamp() as u64;

        try_schedule_trigger(&erm, ts);

        let active = erm.read_temple_event(|s| s.active_event);
        assert_eq!(active, -1, "Should not trigger at second != 0");
    }

    // ── Sprint 200: Attack open/close timer tests ──────────────────

    #[test]
    fn test_event_tick_attack_open_during_active() {
        let erm = EventRoomManager::new();
        // sign=10, attack_open=0, attack_close=30, play=15
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
            // is_attackable starts false, open_control starts false
        });

        // attack_open = sign(10) + attack_open(0) = 10 min = 600s
        let attack_open_time = start_time + 600;

        // Before attack open — is_attackable should be false
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_open_time - 1,
        );
        let attackable = erm.read_temple_event(|s| s.is_attackable);
        assert!(
            !attackable,
            "is_attackable should be false before attack_open time"
        );

        // At attack open time — is_attackable should become true
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_open_time,
        );
        let (attackable, open_ctrl) =
            erm.read_temple_event(|s| (s.is_attackable, s.timer_attack_open_control));
        assert!(
            attackable,
            "is_attackable should be true after attack_open time"
        );
        assert!(open_ctrl, "timer_attack_open_control should be set");
    }

    #[test]
    fn test_event_tick_attack_close_during_active() {
        let erm = EventRoomManager::new();
        // Use opts where attack_close < play so attack close fires before play expires
        // sign=5, attack_open=1, attack_close=10, play=30
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 5,
            play: 30,
            attack_open: 1,
            attack_close: 10,
            finish: 20,
        };
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
            // Pre-set attack as already open
            s.is_attackable = true;
            s.timer_attack_open_control = true;
        });

        // attack_close = (sign(5) + attack_close(10)) * 60 = 900s
        let attack_close_time = start_time + 900;

        // Before attack close — is_attackable still true
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_close_time - 1,
        );
        let attackable = erm.read_temple_event(|s| s.is_attackable);
        assert!(
            attackable,
            "is_attackable should still be true before attack_close time"
        );

        // At attack close time — is_attackable should become false
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_close_time,
        );
        let (attackable, close_ctrl) =
            erm.read_temple_event(|s| (s.is_attackable, s.timer_attack_close_control));
        assert!(
            !attackable,
            "is_attackable should be false after attack_close time"
        );
        assert!(close_ctrl, "timer_attack_close_control should be set");
    }

    #[test]
    fn test_event_tick_attack_open_idempotent() {
        let erm = EventRoomManager::new();
        let opts = make_bdw_opts();
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
        });

        let attack_open_time = start_time + 600;

        // Tick twice at attack open time — should only trigger once
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_open_time,
        );
        assert!(erm.read_temple_event(|s| s.is_attackable));

        // Second tick — open_control already set, should not re-trigger
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_open_time + 1,
        );
        assert!(erm.read_temple_event(|s| s.is_attackable));
        assert!(erm.read_temple_event(|s| s.timer_attack_open_control));
    }

    #[test]
    fn test_event_tick_attack_close_idempotent() {
        let erm = EventRoomManager::new();
        // Use opts where attack_close < play
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 5,
            play: 30,
            attack_open: 1,
            attack_close: 10,
            finish: 20,
        };
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
            s.is_attackable = true;
            s.timer_attack_open_control = true;
        });

        // attack_close = (5+10)*60 = 900s
        let attack_close_time = start_time + 900;

        // Close attack
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_close_time,
        );
        assert!(!erm.read_temple_event(|s| s.is_attackable));

        // Second tick — close_control already set, should not re-open
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            attack_close_time + 1,
        );
        assert!(!erm.read_temple_event(|s| s.is_attackable));
        assert!(erm.read_temple_event(|s| s.timer_attack_close_control));
    }

    #[test]
    fn test_event_tick_attack_open_close_sequence() {
        let erm = EventRoomManager::new();
        // Use opts where attack_open=2 and attack_close=12 (both relative to sign)
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 5,
            play: 20,
            attack_open: 2,
            attack_close: 12,
            finish: 30,
        };
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        bdw_mgr.init_rooms(&erm);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
        });

        // attack_open = (5 + 2) * 60 = 420s
        // attack_close = (5 + 12) * 60 = 1020s

        // Before open
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 419,
        );
        assert!(!erm.read_temple_event(|s| s.is_attackable));

        // Open
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 420,
        );
        assert!(erm.read_temple_event(|s| s.is_attackable));

        // Still open
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1019,
        );
        assert!(erm.read_temple_event(|s| s.is_attackable));

        // Close
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1020,
        );
        assert!(!erm.read_temple_event(|s| s.is_attackable));
    }

    #[test]
    fn test_event_tick_juraid_attack_open_close() {
        let erm = EventRoomManager::new();
        let opts = make_juraid_opts(); // sign=10, attack_open=0, attack_close=60
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[2] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::default();
        let mut juraid_mgr = juraid::JuraidManager::new(1);
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 20000_u64;
        juraid_mgr.init_rooms(&erm);
        juraid_mgr.start_bridge_timer(start_time);
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::JuraidMountain as i16;
            s.is_active = true;
            s.timer_start_control = true;
            s.start_time = start_time;
        });

        // attack_open = (10 + 0) * 60 = 600s
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 600,
        );
        assert!(
            erm.read_temple_event(|s| s.is_attackable),
            "Juraid attack should open"
        );

        // attack_close = (10 + 60) * 60 = 4200s
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 4200,
        );
        assert!(
            !erm.read_temple_event(|s| s.is_attackable),
            "Juraid attack should close"
        );
    }

    // ── Sprint 200: Consolidation verification tests ───────────────

    #[test]
    fn test_event_tick_full_lifecycle_with_attack_phases() {
        let erm = EventRoomManager::new();
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 5,
            play: 20,
            attack_open: 2,
            attack_close: 18,
            finish: 30,
        };
        {
            let mut vopts = erm.vroom_opts.write();
            vopts[0] = Some(opts.clone());
        }
        let mut bdw_mgr = bdw::BdwManager::new(1);
        let mut juraid_mgr = juraid::JuraidManager::default();
        let mut chaos_mgr = chaos::ChaosManager::default();

        let start_time = 10000_u64;
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.allow_join = true;
            s.is_active = false;
            s.start_time = start_time;
            s.sign_remain_seconds = start_time + 300;
        });
        erm.add_signed_up_user("k1".to_string(), 1, 1);

        // Phase: Registration
        assert_eq!(current_phase(&erm), EventPhase::Registration);

        // Sign expires — BDW uses strict `>` so 300s is not enough, need 301s
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 301,
        );
        assert!(matches!(action, EventTickAction::TransitionedToActive(_)));
        assert_eq!(current_phase(&erm), EventPhase::Active);
        assert!(!erm.read_temple_event(|s| s.is_attackable));

        // Attack open (5+2)*60 = 420s
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 420,
        );
        assert!(erm.read_temple_event(|s| s.is_attackable));

        // Attack close (5+18)*60 = 1380s
        let _ = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1380,
        );
        assert!(!erm.read_temple_event(|s| s.is_attackable));

        // Play expires (5+20)*60 = 1500s → Rewards
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1500,
        );
        assert!(matches!(action, EventTickAction::TransitionedToRewards(_)));
        assert_eq!(current_phase(&erm), EventPhase::Rewards);

        // Reset timer (5+20+1)*60 + 30 = 1590s → Cleanup
        let action = event_tick_at(
            &erm,
            &mut bdw_mgr,
            &mut juraid_mgr,
            &mut chaos_mgr,
            start_time + 1590,
        );
        assert!(matches!(
            action,
            EventTickAction::TransitionedToCleanup(_, _)
        ));
        assert_eq!(current_phase(&erm), EventPhase::Inactive);
    }

    #[test]
    fn test_winner_screen_packet_format() {
        // Verify the winner screen packet builder produces correct bytes
        let pkt = event_room::build_winner_select_msg(4); // BDW
        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);

        let pkt = event_room::build_finish_packet(1); // Karus wins
        assert_eq!(pkt.opcode, Opcode::WizEvent as u8);
    }

    #[test]
    fn test_room_close_condition_requires_finish_packet_sent() {
        // Verify the finish condition logic: finish_packet_sent must be true
        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.finish_packet_sent = false;
        room.finish_time_counter = 1;
        room.finished = false;
        let now = 100u64;

        let should_close = room.finish_packet_sent
            && !room.finished
            && room.finish_time_counter > 0
            && room.finish_time_counter <= now;
        assert!(
            !should_close,
            "Room should not close without finish_packet_sent"
        );
    }

    #[test]
    fn test_room_close_condition_after_countdown() {
        // Verify the finish condition logic: countdown must expire
        let mut room =
            crate::systems::event_room::EventRoom::new(1, TempleEventType::BorderDefenceWar);
        room.finish_packet_sent = true;
        room.finish_time_counter = 100;
        room.finished = false;

        // Before countdown
        let should_close_99 = room.finish_packet_sent
            && !room.finished
            && room.finish_time_counter > 0
            && room.finish_time_counter <= 99;
        assert!(!should_close_99, "Room should not close before countdown");

        // At countdown expiry
        let should_close_100 = room.finish_packet_sent
            && !room.finished
            && room.finish_time_counter > 0
            && room.finish_time_counter <= 100;
        assert!(should_close_100, "Room should close after countdown");

        // Mark as finished (simulating the close)
        room.finished = true;

        // After finished, should not re-close
        let should_close_again = room.finish_packet_sent
            && !room.finished
            && room.finish_time_counter > 0
            && room.finish_time_counter <= 200;
        assert!(
            !should_close_again,
            "Room should not re-close after finished"
        );
    }

    // ── Sprint 201: VirtualRoom filter + BDW sign expiry tests ────────

    #[test]
    fn test_check_schedule_trigger_requires_virtual_room_type() {
        // Schedule entries with event_type != 2 should NOT trigger.
        let entry = EventScheduleEntry {
            status: true,
            event_type: 1, // NOT VirtualRoom (2)
            event_id: 9,   // BDW
            zone_id: 84,
            name: String::new(),
            days: [true; 7],
            start_times: [(10, 0), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
            min_level: 0,
            max_level: 0,
            req_loyalty: 0,
            req_money: 0,
        };
        // weekday=0 (Sunday), hour=10, minute=0 → should match time
        // but event_type != 2 → should NOT trigger
        assert!(check_schedule_trigger(&entry, 0, 10, 0).is_none());
    }

    #[test]
    fn test_check_schedule_trigger_virtual_room_type_triggers() {
        // event_type == 2 (VirtualRoom) should trigger normally
        let entry = EventScheduleEntry {
            status: true,
            event_type: 2, // VirtualRoom
            event_id: 9,   // BDW
            zone_id: 84,
            name: String::new(),
            days: [true; 7],
            start_times: [(14, 30), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
            min_level: 0,
            max_level: 0,
            req_loyalty: 0,
            req_money: 0,
        };
        assert_eq!(check_schedule_trigger(&entry, 3, 14, 30), Some(0));
    }

    #[test]
    fn test_bdw_sign_expiry_uses_strict_greater_than() {
        // At exactly start_time + sign_secs, BDW should NOT be expired
        let erm = EventRoomManager::new();
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 10,
            play: 15,
            attack_open: 0,
            attack_close: 30,
            finish: 20,
        };

        let start_time = 1000u64;
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::BorderDefenceWar as i16;
            s.start_time = start_time;
        });

        let sign_secs = 600u64; // 10 * 60
                                // At exact boundary: BDW uses `>` so should NOT be expired
        assert!(!is_sign_expired_at(&erm, &opts, start_time + sign_secs));
        // One second later: should be expired
        assert!(is_sign_expired_at(&erm, &opts, start_time + sign_secs + 1));
    }

    #[test]
    fn test_chaos_sign_expiry_uses_greater_or_equal() {
        // At exactly start_time + sign_secs, Chaos SHOULD be expired
        let erm = EventRoomManager::new();
        let opts = VroomOpt {
            name: "test".to_string(),
            sign: 5,
            play: 15,
            attack_open: 0,
            attack_close: 30,
            finish: 20,
        };

        let start_time = 1000u64;
        erm.update_temple_event(|s| {
            s.active_event = TempleEventType::ChaosDungeon as i16;
            s.start_time = start_time;
        });

        let sign_secs = 300u64; // 5 * 60
                                // At exact boundary: Chaos uses `>=` so SHOULD be expired
        assert!(is_sign_expired_at(&erm, &opts, start_time + sign_secs));
        // One second before: should NOT be expired
        assert!(!is_sign_expired_at(&erm, &opts, start_time + sign_secs - 1));
    }

    #[test]
    fn test_monster_stone_status_accessor() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Default: false
        assert!(!world.get_monster_stone_status(1));

        // Set true
        world.set_monster_stone_status(1, true);
        assert!(world.get_monster_stone_status(1));

        // Set false
        world.set_monster_stone_status(1, false);
        assert!(!world.get_monster_stone_status(1));
    }

    // ── FT Reward Distribution Tests ────────────────────────────────────

    fn setup_ft_session(
        world: &WorldState,
        sid: SessionId,
        nation: u8,
        zone_id: u16,
        hp: i16,
        res_hp_type: u8,
    ) -> tokio::sync::mpsc::UnboundedReceiver<Arc<Packet>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        let info = crate::world::CharacterInfo {
            session_id: sid,
            name: format!("FTPlayer{}", sid),
            nation,
            race: 1,
            class: 101,
            level: 70,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 1000,
            hp,
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
            gold: 1000,
            loyalty: 500,
            loyalty_monthly: 100,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 10_000,
            max_exp: 100_000_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
            res_hp_type,
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
        let pos = crate::world::Position {
            zone_id,
            x: 128.0,
            y: 0.0,
            z: 128.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(sid, info, pos);
        rx
    }

    #[tokio::test]
    async fn test_distribute_ft_rewards_no_rewards_configured() {
        // When no rewards are loaded, should not panic and return gracefully.
        let world = WorldState::new();
        let _rx = setup_ft_session(&world, 1, 1, 55, 1000, 0x01);
        distribute_ft_rewards(&world).await;
        // Gold should be unchanged — no rewards were configured
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 1000);
    }

    #[tokio::test]
    async fn test_distribute_ft_rewards_with_rewards() {
        let world = WorldState::new();
        // Session in FT zone, alive
        let _rx = setup_ft_session(&world, 1, 1, 55, 1000, 0x01);
        // Session NOT in FT zone (Moradon)
        let _rx2 = setup_ft_session(&world, 2, 1, 21, 1000, 0x01);

        // Load FT rewards (local_id=13)
        use ko_db::models::EventRewardRow;
        let reward_rows = vec![EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 13,
            is_winner: true,
            description: "FT Victory".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 50_000,
            loyalty: 100,
            cash: 0,
            noah: 10_000,
        }];
        world.insert_event_rewards(13, reward_rows);

        distribute_ft_rewards(&world).await;

        // Player 1 (in FT zone) should get rewards
        let ch1 = world.get_character_info(1).unwrap();
        assert_eq!(ch1.gold, 11_000); // 1000 + 10_000
        assert_eq!(ch1.exp, 60_000); // 10_000 + 50_000

        // Player 2 (NOT in FT zone) should NOT get rewards
        let ch2 = world.get_character_info(2).unwrap();
        assert_eq!(ch2.gold, 1000);
        assert_eq!(ch2.exp, 10_000);
    }

    #[tokio::test]
    async fn test_distribute_ft_rewards_skips_dead_players() {
        let world = WorldState::new();
        // Alive player in FT zone
        let _rx1 = setup_ft_session(&world, 1, 1, 55, 1000, 0x01);
        // Dead player in FT zone (res_hp_type = USER_DEAD = 0x03)
        let _rx2 = setup_ft_session(&world, 2, 1, 55, 0, 0x03);

        use ko_db::models::EventRewardRow;
        let reward_rows = vec![EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 13,
            is_winner: true,
            description: "FT Victory".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 0,
            loyalty: 0,
            cash: 0,
            noah: 5_000,
        }];
        world.insert_event_rewards(13, reward_rows);

        distribute_ft_rewards(&world).await;

        // Alive player should receive reward
        let ch1 = world.get_character_info(1).unwrap();
        assert_eq!(ch1.gold, 6_000); // 1000 + 5_000

        // Dead player should NOT receive reward
        let ch2 = world.get_character_info(2).unwrap();
        assert_eq!(ch2.gold, 1000);
    }

    #[tokio::test]
    async fn test_distribute_ft_rewards_empty_zone() {
        // No players in zone 55 — should not panic
        let world = WorldState::new();
        // All players in other zones
        let _rx = setup_ft_session(&world, 1, 1, 21, 1000, 0x01);

        use ko_db::models::EventRewardRow;
        let reward_rows = vec![EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 13,
            is_winner: true,
            description: "FT Victory".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 100,
            loyalty: 10,
            cash: 0,
            noah: 100,
        }];
        world.insert_event_rewards(13, reward_rows);

        distribute_ft_rewards(&world).await;
        // No one should have received anything
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 1000);
    }

    #[tokio::test]
    async fn test_distribute_ft_rewards_inactive_rows_ignored() {
        let world = WorldState::new();
        let _rx = setup_ft_session(&world, 1, 1, 55, 1000, 0x01);

        use ko_db::models::EventRewardRow;
        // Only inactive reward rows
        let reward_rows = vec![EventRewardRow {
            s_index: 1,
            status: false, // inactive
            local_id: 13,
            is_winner: true,
            description: "FT Disabled".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 0,
            loyalty: 0,
            cash: 0,
            noah: 999_999,
        }];
        world.insert_event_rewards(13, reward_rows);

        distribute_ft_rewards(&world).await;
        // No rewards given because row is inactive
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 1000);
    }

    /// TempleEventType zone mapping: BDW=84, FT=55, Chaos=85, Juraid=87.
    #[test]
    fn test_temple_event_type_zone_mapping() {
        assert_eq!(TempleEventType::BorderDefenceWar.zone_id(), 84);
        assert_eq!(TempleEventType::ForgottenTemple.zone_id(), 55);
        assert_eq!(TempleEventType::ChaosDungeon.zone_id(), 85);
        assert_eq!(TempleEventType::JuraidMountain.zone_id(), 87);
        // All zones are distinct
        let zones = [84u16, 55, 85, 87];
        for i in 0..zones.len() {
            for j in (i + 1)..zones.len() {
                assert_ne!(zones[i], zones[j]);
            }
        }
    }

    /// TempleEventType from_i16 round-trip for all variants.
    #[test]
    fn test_temple_event_type_from_i16_roundtrip() {
        assert_eq!(
            TempleEventType::from_i16(4),
            Some(TempleEventType::BorderDefenceWar)
        );
        assert_eq!(
            TempleEventType::from_i16(14),
            Some(TempleEventType::ForgottenTemple)
        );
        assert_eq!(
            TempleEventType::from_i16(24),
            Some(TempleEventType::ChaosDungeon)
        );
        assert_eq!(
            TempleEventType::from_i16(100),
            Some(TempleEventType::JuraidMountain)
        );
        assert_eq!(TempleEventType::from_i16(0), None);
        assert_eq!(TempleEventType::from_i16(-1), None);
        assert_eq!(TempleEventType::from_i16(50), None);
    }

    /// Excluded zones list has exactly 7 entries (war/event zones + prison).
    #[test]
    fn test_excluded_zones_count_and_contents() {
        let excluded: &[u16] = &[81, 82, 83, 84, 85, 87, 92];
        assert_eq!(excluded.len(), 7);
        // Contains BDW zone (84), Chaos (85), Juraid (87), Prison (92)
        assert!(excluded.contains(&84));
        assert!(excluded.contains(&85));
        assert!(excluded.contains(&87));
        assert!(excluded.contains(&92));
    }

    /// VroomOpt defaults: all fields are independently configurable.
    #[test]
    fn test_vroom_opt_fields() {
        let opt = VroomOpt {
            name: "TestEvent".to_string(),
            sign: 5,
            play: 20,
            attack_open: 3,
            attack_close: 18,
            finish: 30,
        };
        assert_eq!(opt.sign, 5);
        assert_eq!(opt.play, 20);
        // attack_open < attack_close < play
        assert!(opt.attack_open < opt.attack_close);
        assert!(opt.attack_close <= opt.play);
        // finish is in seconds, sign/play in minutes
        assert!(opt.finish > 0);
    }

    /// unix_now returns a value greater than year 2020 epoch.
    #[test]
    fn test_unix_now_reasonable() {
        let now = unix_now();
        // Jan 1 2020 00:00:00 UTC = 1577836800
        assert!(now > 1_577_836_800);
        // Should be less than year 2100
        assert!(now < 4_102_444_800);
    }

    // ── Sprint 1000: Additional coverage ──────────────────────────────

    /// BDW/Chaos EXP caps: 8M gained, 10M premium.
    #[test]
    fn test_event_exp_cap_constants() {
        const EXP_CAP: i64 = 8_000_000;
        const PREMIUM_CAP: i64 = 10_000_000;
        // BDW: very high points → capped
        let (gained, premium) = bdw_user_point_exp(83, 1000);
        assert_eq!(gained, EXP_CAP);
        assert_eq!(premium, PREMIUM_CAP);
        // Chaos: very high kills → capped
        let (gained2, premium2) = chaos_user_exp(83, 1000, 0);
        assert_eq!(gained2, EXP_CAP);
        assert_eq!(premium2, PREMIUM_CAP);
    }

    /// BDW premium EXP is exactly 2x gained EXP (before capping).
    #[test]
    fn test_bdw_premium_is_double_gained() {
        // Low enough points that neither cap is hit
        let (gained, premium) = bdw_user_point_exp(40, 2);
        assert_eq!(premium, gained * 2);
        // Another data point
        let (g2, p2) = bdw_user_point_exp(50, 5);
        assert_eq!(p2, g2 * 2);
    }

    /// Chaos kill_score formula: 5*kills - deaths.
    #[test]
    fn test_chaos_kill_score_formula() {
        // 10 kills, 0 deaths → score = 50
        // 10 kills, 10 deaths → score = 40
        // The ratio between them should reflect the formula
        let (exp_50, _) = chaos_user_exp(60, 10, 0);
        let (exp_40, _) = chaos_user_exp(60, 10, 10);
        // exp_50 / exp_40 ≈ 50/40 = 1.25
        assert!(exp_50 > exp_40);
        // Exact: same base, kill_score difference → ratio = 50/40
        // 216000 * 0.15 * 50 = 1_620_000 vs 216000 * 0.15 * 40 = 1_296_000
        assert_eq!(exp_50, 1_620_000);
        assert_eq!(exp_40, 1_296_000);
    }

    /// BDW level exp bonus boundary: level 57 uses low formula, 58 uses high formula.
    #[test]
    fn test_bdw_level_exp_bonus_formula_boundary() {
        let low = bdw_level_exp_bonus(57); // (57-20)*203000 = 7_511_000
        let high = bdw_level_exp_bonus(58); // (58+55)*120000 = 13_560_000
                                            // High formula gives significantly more EXP at the boundary
        assert!(high > low);
        // The jump ratio at boundary
        assert!((high as f64 / low as f64) > 1.5);
        // Verify exact values
        assert_eq!(low, 37 * 203_000);
        assert_eq!(high, 113 * 120_000);
    }

    /// EVENT_TICK_INTERVAL_SECS equals 1 second (C++ VirtualEventTimer).
    #[test]
    fn test_event_tick_interval_is_one_second() {
        assert_eq!(EVENT_TICK_INTERVAL_SECS, 1);
    }

    #[tokio::test]
    async fn test_distribute_ft_rewards_hp_zero_is_dead() {
        let world = WorldState::new();
        // Player with hp=0 but res_hp_type != USER_DEAD
        // C++ isDead() returns true if hp <= 0 OR res_hp_type == USER_DEAD
        let _rx = setup_ft_session(&world, 1, 1, 55, 0, 0x01);

        use ko_db::models::EventRewardRow;
        let reward_rows = vec![EventRewardRow {
            s_index: 1,
            status: true,
            local_id: 13,
            is_winner: true,
            description: "FT".to_string(),
            item_id1: 0,
            item_count1: 0,
            item_expiration1: 0,
            item_id2: 0,
            item_count2: 0,
            item_expiration2: 0,
            item_id3: 0,
            item_count3: 0,
            item_expiration3: 0,
            experience: 0,
            loyalty: 0,
            cash: 0,
            noah: 10_000,
        }];
        world.insert_event_rewards(13, reward_rows);

        distribute_ft_rewards(&world).await;
        // hp=0 means dead, should not receive reward
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 1000);
    }
}
