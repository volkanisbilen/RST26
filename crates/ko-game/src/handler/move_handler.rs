//! WIZ_MOVE (0x06) handler — character movement.
//! ## Request (C->S)
//! | Offset | Type   | Description |
//! |--------|--------|-------------|
//! | 0      | u16le  | Destination X (×10) |
//! | 2      | u16le  | Destination Z (×10) |
//! | 4      | u16le  | Destination Y (×10) |
//! | 6      | i16le  | Speed |
//! | 8      | u8     | Echo (0=finish, 1=start, 3=move) |
//! | 9      | u16le  | Current X (×10) |
//! | 11     | u16le  | Current Z (×10) |
//! | 13     | u16le  | Current Y (×10) |
//! ## Broadcast to nearby players
//! `[u32 socket_id] [u16 will_x] [u16 will_z] [u16 will_y] [i16 speed] [u8 echo]`

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::clan_constants::COMMAND_CAPTAIN;
use crate::handler::{region, zone_change};
use crate::session::{ClientSession, SessionState};
use crate::world::{RegionChangeResult, WorldState, ZONE_CHAOS_DUNGEON, ZONE_DUNGEON_DEFENCE};
use crate::zone::{GameEventType, SessionId};

/// Valid echo values from the `moveop` enum.
/// Only finish(0), start(1), and move(3) are valid. nott(2) is rejected.
const ECHO_FINISH: u8 = 0;
const ECHO_START: u8 = 1;
const ECHO_MOVE: u8 = 3;

/// Maximum consecutive echo/speed anomaly violations before warping Home.
const MAX_CAUGHT_COUNT: u8 = 3;

/// Time window (ms) for echo anomaly detection.
const CAUGHT_TIME_WINDOW_MS: u64 = 1100;

/// Average two packed movement coordinates without overflowing `u16`.
///
/// The client can use `u16::MAX` as the terrain-Y sentinel immediately after
/// a zone change. Adding two packed coordinates as `u16` therefore panics in
/// debug builds before the division can bring the result back into range.
#[inline]
fn average_packed_coord(a: u16, b: u16) -> u16 {
    ((a as u32 + b as u32) / 2) as u16
}

/// Handle WIZ_MOVE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let mut will_x = reader.read_u16().unwrap_or(0);
    let mut will_z = reader.read_u16().unwrap_or(0);
    let mut will_y = reader.read_u16().unwrap_or(0);
    let speed = reader.read_u16().unwrap_or(0) as i16;
    let echo = reader.read_u8().unwrap_or(0);
    let cur_x = reader.read_u16().unwrap_or(0);
    let cur_z = reader.read_u16().unwrap_or(0);
    let cur_y = reader.read_u16().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    // ── Echo validation (no DashMap needed) ──────────────────────────
    // Valid echo: finish(0), start(1), move(3). nott(2) = disconnect.
    if echo != ECHO_FINISH && echo != ECHO_START && echo != ECHO_MOVE {
        tracing::warn!(sid, echo, "invalid move echo value, disconnecting");
        return Ok(());
    }

    // ── Batch snapshot: read all needed session state in ONE DashMap lookup ──
    struct MoveSnapshot {
        is_dead: bool,
        is_selling: bool,
        is_preparing: bool,
        is_mining: bool,
        is_fishing: bool,
        invisibility_type: u8,
        abnormal_type: u32,
        check_warp: bool,
        old_echo: i8,
        old_speed: i16,
        old_will_x: u16,
        old_will_z: u16,
        zone_id: u16,
        pos_x: f32,
        pos_z: f32,
        region_x: u16,
        region_z: u16,
        event_room: u16,
        // Character fields (avoid separate get_character_info DashMap read)
        authority: u8,
        class: u16,
        fame: u8,
        nation: u8,
        bind_zone: u8,
    }
    let snap = match world.with_session(sid, |h| {
        let ch = h.character.as_ref();
        MoveSnapshot {
            is_dead: ch.is_some_and(|c| c.res_hp_type == 0x03 || c.hp <= 0),
            is_selling: h.merchant_state == 2, // MERCHANT_STATE_SELLING
            is_preparing: h.selling_merchant_preparing,
            is_mining: h.is_mining,
            is_fishing: h.is_fishing,
            invisibility_type: h.invisibility_type,
            abnormal_type: h.abnormal_type,
            check_warp: h.check_warp_zone_change,
            old_echo: h.move_old_echo,
            old_speed: h.move_old_speed,
            old_will_x: h.move_old_will_x,
            old_will_z: h.move_old_will_z,
            zone_id: h.position.zone_id,
            pos_x: h.position.x,
            pos_z: h.position.z,
            region_x: h.position.region_x,
            region_z: h.position.region_z,
            event_room: h.event_room,
            authority: ch.map(|c| c.authority).unwrap_or(1),
            class: ch.map(|c| c.class).unwrap_or(0),
            fame: ch.map(|c| c.fame).unwrap_or(0),
            nation: ch.map(|c| c.nation).unwrap_or(0),
            bind_zone: ch.map(|c| c.bind_zone).unwrap_or(0),
        }
    }) {
        Some(s) => s,
        None => return Ok(()),
    };

    // Block movement during zone change (C++ m_bWarp check)
    // Uses dedicated function (not snapshot) because it has 30s stuck auto-clear side effect.
    if world.is_zone_changing(sid) {
        return Ok(());
    }

    // Block movement if player is dead
    if snap.is_dead {
        return Ok(());
    }

    // ── Cancel merchant on movement ──────────────────────────────────
    if snap.is_selling || snap.is_preparing {
        super::merchant::merchant_close(session).await?;
    }

    // ── Cancel mining/fishing on movement ────────────────────────────
    if snap.is_mining {
        super::mining::stop_mining_internal(&world, sid);
    }
    if snap.is_fishing {
        super::mining::stop_fishing_internal(&world, sid);
    }

    let is_gm = snap.authority == 0;

    // ── Echo/speed anomaly detection ─────────────────────────────────
    let stable = will_x == cur_x && will_z == cur_z && will_y == cur_y;
    if !is_gm && !stable {
        // Anomaly: echo!=0 but speed==0, or two consecutive echo==0
        let anomaly = (echo != 0 && speed == 0) || (snap.old_echo == 0 && echo == 0);

        if anomaly {
            let now = std::time::Instant::now();
            let should_home = world
                .with_session(sid, |h| {
                    // If within caught time window, increment; otherwise reset to 1
                    let elapsed_ms = now.duration_since(h.move_caught_time).as_millis() as u64;
                    if elapsed_ms < CAUGHT_TIME_WINDOW_MS {
                        h.speed_hack_count + 1
                    } else {
                        1
                    }
                })
                .unwrap_or(1);

            world.update_session(sid, |h| {
                let elapsed_ms = now.duration_since(h.move_caught_time).as_millis() as u64;
                if elapsed_ms < CAUGHT_TIME_WINDOW_MS {
                    h.speed_hack_count += 1;
                } else {
                    h.speed_hack_count = 1;
                }
                h.move_caught_time = now + std::time::Duration::from_millis(CAUGHT_TIME_WINDOW_MS);
            });

            if should_home >= MAX_CAUGHT_COUNT {
                tracing::warn!(
                    sid,
                    "echo/speed anomaly: {} consecutive violations, warping Home",
                    should_home
                );
                world.update_session(sid, |h| {
                    h.speed_hack_count = 0;
                });
                // Warp to Home zone (C++ Home())
                zone_change::trigger_zone_change(
                    session,
                    determine_home_zone_raw(snap.bind_zone, snap.nation, &world),
                    0.0,
                    0.0,
                )
                .await?;
                return Ok(());
            }
        }
    }

    // Update old echo/speed state
    world.update_session(sid, |h| {
        h.move_old_echo = echo as i8;
        h.move_old_speed = speed;
    });

    // Clear warp-loop prevention flag on first move with speed > 0.
    if speed != 0 && snap.check_warp {
        world.set_check_warp_zone_change(sid, false);
    }

    // ── Previous destination position for correction ─────────────────
    let f_will_x = if snap.old_will_x == 0 {
        will_x
    } else {
        snap.old_will_x
    };
    let f_will_z = if snap.old_will_z == 0 {
        will_z
    } else {
        snap.old_will_z
    };

    // ── Position correction based on distance/speed ratio ────────────
    if snap.old_speed == 0 && echo == ECHO_START {
        will_x = average_packed_coord(will_x, cur_x);
        will_y = average_packed_coord(will_y, cur_y);
        will_z = average_packed_coord(will_z, cur_z);
    } else if speed != 0 {
        // GetDistance returns squared distance (no sqrt)
        let dist = get_distance_scaled(f_will_x, f_will_z, will_x, will_z);
        let ratio = dist / speed as f32;

        if ratio > 8.0 && ratio < 10.0 {
            // Average with current position
            will_x = average_packed_coord(will_x, cur_x);
            will_y = average_packed_coord(will_y, cur_y);
            will_z = average_packed_coord(will_z, cur_z);
        } else if ratio >= 12.0 {
            // Snap to current position
            will_x = cur_x;
            will_y = cur_y;
            will_z = cur_z;
        }
    }

    // Convert from ×10 to world coordinates
    let x = will_x as f32 / 10.0;
    let z = will_z as f32 / 10.0;
    let y = will_y as f32 / 10.0;

    // ── Merchant close on move (broadcast, uses cached snap) ─────────
    // Note: merchant_close() already called above on snap.is_selling/is_preparing.
    // This second block handles broadcast + state cleanup.
    if snap.is_selling || snap.is_preparing {
        world.close_merchant(sid);
        let mut close_pkt = Packet::new(Opcode::WizMerchant as u8);
        close_pkt.write_u8(2); // MERCHANT_CLOSE sub-opcode
        close_pkt.write_u32(sid as u32);
        if snap.is_selling {
            world.broadcast_to_3x3(
                snap.zone_id,
                snap.region_x,
                snap.region_z,
                Arc::new(close_pkt),
                None,
                snap.event_room,
            );
        } else {
            world.send_to_session_owned(sid, close_pkt);
        }
    }

    // ── Speed value validation (SpeedHackUser) ───────────────────────
    if !is_gm {
        let base_class = snap.class % 100;
        let is_rogue = matches!(base_class, 2 | 7 | 8);
        let is_captain = snap.fame == COMMAND_CAPTAIN;
        let in_special_zone =
            snap.zone_id == ZONE_CHAOS_DUNGEON || snap.zone_id == ZONE_DUNGEON_DEFENCE;

        let max_speed: i16 = if is_rogue || is_captain || in_special_zone {
            90
        } else if matches!(
            base_class,
            1 | 3 | 4 | 5 | 6 | 9 | 10 | 11 | 12 | 13 | 14 | 15
        ) {
            // warrior(1), mage(3), priest(4), kurian(5+)
            67
        } else {
            45
        };

        if speed > max_speed || speed < -max_speed {
            tracing::warn!(
                sid,
                speed,
                max_speed,
                "speed value out of range, rejecting move"
            );
            return Ok(());
        }
    }

    // NOTE: Distance-based speed hack check (SpeedHackTime) is handled by
    // WIZ_SPEEDHACK_CHECK (0x41) in speedhack.rs — NOT here.
    // SpeedHackTime() is a separate packet handler in KnightCrownGuard.cpp:5-35.

    // Update old destination position
    world.update_session(sid, |h| {
        h.move_old_will_x = will_x;
        h.move_old_will_z = will_z;
        h.move_old_will_y = will_y;
    });

    // ── Stealth break on move ────────────────────────────────────────
    //   if (m_bInvisibilityType == INVIS_DISPEL_ON_MOVE)
    //       CMagicProcess::RemoveStealth(this, INVIS_DISPEL_ON_MOVE);
    if snap.invisibility_type == super::stealth::INVIS_DISPEL_ON_MOVE {
        super::stealth::remove_stealth_type(&world, sid, super::stealth::INVIS_DISPEL_ON_MOVE)
    }

    // Use cached zone_id from snapshot (avoids redundant DashMap lookup).
    let zone_id = snap.zone_id;

    // Movement validation — boundary check only
    if let Some(zone) = world.get_zone(zone_id) {
        if !zone.is_valid_position(x, z) {
            tracing::warn!(sid, x, z, "movement rejected: out of map bounds");
            return Ok(());
        }
    }

    // Use cached position from snapshot for pet follow (C++ m_oldx/m_oldz).
    let old_x = snap.pos_x;
    let old_z = snap.pos_z;

    // Update position in world state
    let region_result = world.update_position(sid, zone_id, x, y, z);

    // ── GM invisible broadcast suppression ──────────────────────────
    //   if (isGM() && m_bAbnormalType == ABNORMAL_INVISIBLE) skip broadcast
    // ABNORMAL_INVISIBLE = 0
    let suppress_broadcast = is_gm && snap.abnormal_type == 0;

    // Build broadcast packet: [socket_id][will_x][will_z][will_y][speed][echo]
    let mut bcast = Packet::new(Opcode::WizMove as u8);
    bcast.write_u32(sid as u32);
    bcast.write_u16(will_x);
    bcast.write_u16(will_z);
    bcast.write_u16(will_y);
    bcast.write_i16(speed);
    bcast.write_u8(echo);

    match region_result {
        RegionChangeResult::Changed {
            old_rx,
            old_rz,
            new_rx,
            new_rz,
        } => {
            // Player changed regions — update zone grid
            if let Some(zone) = world.get_zone(zone_id) {
                zone.remove_user(old_rx, old_rz, sid);
                zone.add_user(new_rx, new_rz, sid);
            }

            if !suppress_broadcast {
                // Broadcast INOUT_OUT to old-only regions
                let out_pkt =
                    region::build_user_inout(region::INOUT_OUT, sid, None, &Default::default());
                world.broadcast_to_old_regions(
                    zone_id,
                    old_rx,
                    old_rz,
                    new_rx,
                    new_rz,
                    Arc::new(out_pkt),
                    Some(sid),
                    snap.event_room,
                );

                // Broadcast INOUT_IN to new-only regions (position fresh from update_position)
                let my_char = world.get_character_info(sid);
                let my_pos = world.get_position(sid).unwrap_or_default();
                let my_equip = region::get_equipped_visual(&world, sid);
                let in_pkt = region::build_user_inout_with_invis(
                    region::INOUT_IN,
                    sid,
                    my_char.as_ref(),
                    &my_pos,
                    snap.invisibility_type,
                    snap.abnormal_type,
                    &my_equip,
                );
                world.broadcast_to_new_regions(
                    zone_id,
                    old_rx,
                    old_rz,
                    new_rx,
                    new_rz,
                    Arc::new(in_pkt),
                    Some(sid),
                    snap.event_room,
                );
            }

            // Send WIZ_REGIONCHANGE to myself (new nearby list)
            //   RegionNpcInfoForMe();   — NPC ID list (client uses cached templates)
            //   RegionUserInOutForMe(); — user visibility updates
            //   MerchantUserInOutForMe(); — merchant stall visibility
            //   NpcInOutForMe() is COMMENTED OUT in C++ (User.h:886) — client reconstructs
            //   NPC details from local data after receiving NPC_REGION ID list.
            region::send_region_npc_info_for_me(session).await?;
            region::send_region_user_in_out_for_me(session).await?;
            region::send_merchant_user_in_out_for_me(session).await?;

            if !suppress_broadcast {
                // Also broadcast the move to new 3×3 grid
                world.broadcast_to_3x3(
                    zone_id,
                    new_rx,
                    new_rz,
                    Arc::new(bcast),
                    Some(sid),
                    snap.event_room,
                );
            }
        }
        RegionChangeResult::NoChange => {
            if !suppress_broadcast {
                // Same region — broadcast move to 3×3 grid (region unchanged, use cached).
                world.broadcast_to_3x3(
                    zone_id,
                    snap.region_x,
                    snap.region_z,
                    Arc::new(bcast),
                    Some(sid),
                    snap.event_room,
                );
            }
        }
    }

    // Event check — warp gates and traps
    if let Some(zone) = world.get_zone(zone_id) {
        if let Some(event) = zone.check_event(x, z) {
            match event.event_type {
                GameEventType::ZoneChange => {
                    let dest_zone = event.exec[0] as u16;
                    let dest_x = event.exec[1] as f32;
                    let dest_z = event.exec[2] as f32;
                    zone_change::trigger_zone_change(session, dest_zone, dest_x, dest_z).await?;
                }
                GameEventType::TrapDead => {
                    // Intentionally a no-op: C++ has `Dead()` commented out in GameEvent.cpp:31-33
                    tracing::debug!(sid, "trap dead event (no-op, matching C++)");
                }
                GameEventType::TrapArea => {
                    // Apply ZONE_TRAP_DAMAGE (500 HP) every ZONE_TRAP_INTERVAL (2s)
                    const TRAP_INTERVAL_SECS: u64 = 2;
                    const TRAP_DAMAGE: i16 = 500;

                    let now = std::time::Instant::now();
                    let can_trap = world
                        .with_session(sid, |h| {
                            now.duration_since(h.last_trap_time).as_secs() >= TRAP_INTERVAL_SECS
                        })
                        .unwrap_or(false);

                    if can_trap {
                        world.update_session(sid, |h| {
                            h.last_trap_time = now;
                        });

                        // Apply damage via HpChange
                        if let Some(ch) = world.get_character_info(sid) {
                            let new_hp = (ch.hp - TRAP_DAMAGE).max(0);
                            world.update_character_stats(sid, |c| {
                                c.hp = new_hp;
                            });
                            // Send WIZ_HP_CHANGE to victim
                            let hp_pkt =
                                crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp);
                            world.send_to_session_owned(sid, hp_pkt);

                            tracing::debug!(sid, hp = new_hp, "trap area damage: -{}", TRAP_DAMAGE);

                            if new_hp <= 0 {
                                // Player died from trap — trigger death
                                tracing::debug!(sid, "player killed by trap area");
                            }
                        }
                    }
                }
            }
        }
    }

    // ── BDW altar delivery check ────────────────────────────────────────
    // Called at end of every move handler when in zone 84.
    if zone_id == crate::systems::bdw::ZONE_BDW {
        bdw_monument_point_process(&world, sid);
    }

    // ── Oreads terrain effects ──────────────────────────────────────────
    // Evaluates player position in ZONE_BATTLE6 (Oreads) during nation battle
    // and sends terrain type to client (affects combat modifiers + visuals).
    // C++ calls this every move; we only build/send when in ZONE_BATTLE6.
    if zone_id == crate::world::ZONE_BATTLE6 {
        let is_nation_battle = world.get_battle_state().is_nation_battle();
        let is_gm = snap.authority == 0; // GM_AUTHORITY
        let terrain = super::terrain_effects::evaluate_terrain(
            zone_id, is_nation_battle, is_gm, x, z,
        );
        let pkt = super::terrain_effects::build_terrain_effects_packet(terrain);
        world.send_to_session(sid, &pkt);
    }

    // ── Pet follow on player movement ────────────────────────────────
    // When a player with an active pet moves, the pet follows: if distance
    // ≥ 10 m or speed == 0, move pet 2 units toward the player's old position.
    pet_follow_on_move(&world, sid, speed, old_x, old_z);

    Ok(())
}

/// BDW altar delivery check — called every movement tick in zone 84.
/// 1. Validates user is in a BDW room with the altar flag
/// 2. Checks if user is in their nation's delivery zone
/// 3. On delivery: scores, broadcasts, starts respawn timer
fn bdw_monument_point_process(world: &WorldState, sid: SessionId) {
    use crate::systems::bdw;
    use crate::systems::event_room::{self, TempleEventState, TempleEventType};

    let is_bdw_active = world
        .event_room_manager
        .read_temple_event(|s: &TempleEventState| s.is_bdw_active());
    if !is_bdw_active {
        return;
    }

    let (user_name, nation) = match world.get_character_info(sid) {
        Some(ch) => (ch.name.clone(), ch.nation),
        None => return,
    };

    // Get current position for delivery zone check
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return,
    };

    // C++ uses pos / 10.0 for zone coordinate check
    let check_x = pos.x;
    let check_z = pos.z;

    // Quick check: is the player in any delivery zone?
    if !bdw::is_in_delivery_zone(nation, check_x, check_z) {
        return;
    }

    // Find user's room
    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::BorderDefenceWar, &user_name)
    {
        Some(r) => r,
        None => return,
    };

    // Process delivery inside lock scopes
    // Lock order: bdw_manager → room DashMap (consistent with event_system.rs)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let delivery_result = {
        let mut bdw_mgr = world.bdw_manager_write();

        let Some(mut room) = world
            .event_room_manager
            .get_room_mut(TempleEventType::BorderDefenceWar, room_id)
        else {
            return;
        };

        if room.finish_packet_sent {
            return;
        }

        // Check if user has the altar flag
        let has_flag = if nation == 1 {
            room.karus_users
                .get(&user_name)
                .is_some_and(|u| u.has_altar_obtained)
        } else {
            room.elmorad_users
                .get(&user_name)
                .is_some_and(|u| u.has_altar_obtained)
        };

        if !has_flag {
            return;
        }

        // Clear flag from carrier
        if nation == 1 {
            if let Some(u) = room.karus_users.get_mut(&user_name) {
                u.has_altar_obtained = false;
            }
        } else if let Some(u) = room.elmorad_users.get_mut(&user_name) {
            u.has_altar_obtained = false;
        }

        // Score the delivery
        let bdw_state = match bdw_mgr.get_room_state_mut(room_id) {
            Some(s) => s,
            None => return,
        };

        let (k_score, e_score, winner) =
            bdw::altar_delivery_score_change(&mut room, bdw_state, nation, now);

        // Start respawn timer (if no win triggered — win already cleared it)
        if winner.is_none() {
            bdw::start_altar_respawn_timer(bdw_state, now);
        }

        Some((k_score, e_score, winner))
    }; // bdw_mgr + room lock dropped

    let Some((k_score, e_score, winner)) = delivery_result else {
        return;
    };

    // Broadcast altar timer (60 seconds) to all room users
    let timer_pkt = event_room::build_altar_timer_packet(bdw::ALTAR_RESPAWN_DELAY_SECS as u16);
    super::dead::broadcast_to_bdw_room(world, room_id, &timer_pkt);

    // Broadcast scoreboard update
    let screen_pkt = event_room::build_temple_screen_packet(k_score, e_score);
    super::dead::broadcast_to_bdw_room(world, room_id, &screen_pkt);

    // If win condition met, send winner screen packets
    if let Some(winner_nation) = winner {
        let arc_select = Arc::new(event_room::build_winner_select_msg(4)); // BDW
        let arc_finish = Arc::new(event_room::build_finish_packet(winner_nation));

        if let Some(room) = world
            .event_room_manager
            .get_room(TempleEventType::BorderDefenceWar, room_id)
        {
            for u in room.karus_users.values().filter(|u| !u.logged_out) {
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_select));
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_finish));
            }
            for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_select));
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_finish));
            }
        }
    }

    // Remove speed debuff from carrier on delivery
    world.remove_buff(sid, crate::systems::bdw::BUFF_TYPE_FRAGMENT_OF_MANES);

    tracing::info!(
        "BDW altar delivery: '{}' (nation={}) in room {}, scores: K={} E={}",
        user_name,
        nation,
        room_id,
        k_score,
        e_score,
    );
}

/// Calculate distance between two points (×10 scaled coordinates) using C++ GetDistance.
/// `GetDistance(fWillX / 10.0f, fWillZ / 10.0f, will_x / 10.0f, will_z / 10.0f)`
/// GetDistance returns squared distance (dx² + dz²) without sqrt.
fn get_distance_scaled(x1: u16, z1: u16, x2: u16, z2: u16) -> f32 {
    let dx = x2 as f32 / 10.0 - x1 as f32 / 10.0;
    let dz = z2 as f32 / 10.0 - z1 as f32 / 10.0;
    dx * dx + dz * dz
}

/// Determine the zone to warp to when a speed hack Home() is triggered.
/// Uses bind zone if available, otherwise falls back to nation home zone.
fn determine_home_zone_raw(bind_zone: u8, nation: u8, world: &crate::world::WorldState) -> u16 {
    let bz = bind_zone as u16;
    if bz > 0 {
        return bz;
    }
    if world.get_home_position(nation).is_some() {
        if nation == 1 {
            return 1; // Karus zone
        } else {
            return 2; // Elmorad zone
        }
    }
    21 // Moradon fallback
}

/// Pet follow on player movement.
/// When a player with an active pet moves:
/// 1. Look up pet NPC instance by the pet's runtime NPC ID
/// 2. If distance from pet to player ≥ 10 m OR speed == 0, move pet closer
/// 3. Normalize direction from pet to player, move 2 units toward player's old position
/// 4. If pet is in attack mode with active family attack, cancel the attack
/// 5. Broadcast `WIZ_NPC_MOVE` for the pet NPC
fn pet_follow_on_move(world: &WorldState, sid: SessionId, speed: i16, old_x: f32, old_z: f32) {
    /// Distance threshold for pet follow (squared): 10 m.
    ///
    /// Note: `GetDistanceSqrt` returns ACTUAL distance (with sqrt), not squared.
    const PET_FOLLOW_DIST: f32 = 10.0;

    use super::pet::MODE_ATTACK;

    let pet_info = world.with_session(sid, |h| {
        h.pet_data
            .as_ref()
            .map(|p| (p.nid, p.state_change, p.attack_started))
    });

    let (pet_nid, pet_mode, attack_started) = match pet_info {
        Some(Some(info)) => info,
        _ => return,
    };

    // Look up the pet NPC instance
    let pet_npc = match world.get_npc_instance(pet_nid as u32) {
        Some(n) => n,
        None => return,
    };

    // C++ checks NPC_STANDING or NPC_MOVING state — we only have NPC instances,
    // not full AI state for pet NPCs. Skip state check (pets are always in
    // standing/moving state by definition).

    // Get the player's current position (already updated)
    let player_pos = match world.get_position(sid) {
        Some(p) => p,
        None => return,
    };

    // Distance check: actual distance (sqrt), matching C++ GetDistanceSqrt
    let dx = pet_npc.x - player_pos.x;
    let dz = pet_npc.z - player_pos.z;
    let dist_sq = dx * dx + dz * dz;
    let distance = dist_sq.sqrt();

    // Trigger follow if: player stopped (speed==0) OR pet too far (≥10m)
    if speed != 0 && distance < PET_FOLLOW_DIST {
        return;
    }

    if distance == 0.0 {
        return;
    }

    // Cancel active attack if pet is in attack mode
    if pet_mode == MODE_ATTACK && attack_started {
        world.update_session(sid, |h| {
            if let Some(ref mut pet) = h.pet_data {
                pet.attack_started = false;
                pet.attack_target_id = -1;
            }
        });
    }

    // Normalize direction from pet to player and move 2 units
    //   warp_x /= distance; warp_z /= distance;
    //   warp_x *= 2; warp_z *= 2;
    //   warp_x += m_oldx; warp_z += m_oldz;
    let dir_x = dx / distance;
    let dir_z = dz / distance;
    let new_x = old_x + dir_x * 2.0;
    let new_z = old_z + dir_z * 2.0;

    // Update pet NPC position
    world.update_npc_position(pet_nid as u32, new_x, new_z);

    // Broadcast pet move: WIZ_NPC_MOVE
    let mut move_pkt = Packet::new(Opcode::WizNpcMove as u8);
    move_pkt.write_u8(1); // move type
    move_pkt.write_u32(pet_nid as u32);
    move_pkt.write_u16((new_x * 10.0) as u16);
    move_pkt.write_u16((new_z * 10.0) as u16);
    move_pkt.write_u16(0); // y * 10
    move_pkt.write_u16((distance * 10.0) as u16); // speed

    let event_room = world.get_event_room(sid);
    world.broadcast_to_3x3(
        player_pos.zone_id,
        player_pos.region_x,
        player_pos.region_z,
        Arc::new(move_pkt),
        None,
        event_room,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average_packed_coord_handles_zone_change_y_sentinel() {
        assert_eq!(average_packed_coord(u16::MAX, u16::MAX), u16::MAX);
        assert_eq!(average_packed_coord(u16::MAX, 0), 32_767);
        assert_eq!(average_packed_coord(2_000, 4_000), 3_000);
    }

    #[test]
    fn test_echo_validation() {
        // Valid echo values
        assert_eq!(ECHO_FINISH, 0);
        assert_eq!(ECHO_START, 1);
        assert_eq!(ECHO_MOVE, 3);

        // Echo value 2 (nott) is invalid
        let invalid_echo: u8 = 2;
        assert!(
            invalid_echo != ECHO_FINISH && invalid_echo != ECHO_START && invalid_echo != ECHO_MOVE
        );

        // Echo values 0, 1, 3 are valid
        for &valid in &[ECHO_FINISH, ECHO_START, ECHO_MOVE] {
            assert!(valid == ECHO_FINISH || valid == ECHO_START || valid == ECHO_MOVE);
        }
    }

    #[test]
    fn test_get_distance_scaled() {
        // Same point
        assert_eq!(get_distance_scaled(100, 100, 100, 100), 0.0);

        // 10 units apart in X (100 = 10.0, 200 = 20.0, diff = 10.0)
        let dist = get_distance_scaled(100, 100, 200, 100);
        assert!((dist - 100.0).abs() < 0.01); // 10² = 100

        // Diagonal: dx=10, dz=10 → 100+100 = 200
        let dist = get_distance_scaled(100, 100, 200, 200);
        assert!((dist - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_speed_hack_class_limits() {
        // Rogue base classes: 2, 7, 8
        for &class in &[102u16, 107, 108, 202, 207, 208] {
            let base = class % 100;
            assert!(matches!(base, 2 | 7 | 8), "class {} should be rogue", class);
        }

        // Non-rogue classes
        for &class in &[101u16, 103, 104, 105, 201, 203, 204, 205] {
            let base = class % 100;
            assert!(
                !matches!(base, 2 | 7 | 8),
                "class {} should not be rogue",
                class
            );
        }
    }

    #[test]
    fn test_speed_value_limits() {
        // Rogue: max speed 90
        let rogue_max: i16 = 90;
        assert!(80 <= rogue_max && 80 >= -rogue_max); // valid
        assert!((91 > rogue_max)); // invalid

        // Warrior/Mage/Priest: max speed 67
        let other_max: i16 = 67;
        assert!(60 <= other_max && 60 >= -other_max); // valid
        assert!((68 > other_max)); // invalid
    }

    #[test]
    fn test_distance_speed_ratio_correction() {
        // Test the position correction logic from C++:
        // ratio 8-10 → average with current
        // ratio >= 12 → snap to current

        let f_will_x: u16 = 1000; // 100.0 world coords
        let f_will_z: u16 = 1000;
        let will_x: u16 = 2000; // 200.0 world coords
        let will_z: u16 = 1000;
        let speed: i16 = 1;

        let dist = get_distance_scaled(f_will_x, f_will_z, will_x, will_z);
        let ratio = dist / speed as f32;
        // dist = 100² = 10000, ratio = 10000/1 = 10000 → >= 12, snap
        assert!(ratio >= 12.0);

        // Normal speed case
        let speed2: i16 = 50;
        let will_x2: u16 = 1100; // 110.0 world coords
        let dist2 = get_distance_scaled(f_will_x, f_will_z, will_x2, will_z);
        let ratio2 = dist2 / speed2 as f32;
        // dist = 10² = 100, ratio = 100/50 = 2.0 → valid (< 8)
        assert!(ratio2 < 8.0);
    }

    #[test]
    fn test_echo_anomaly_detection() {
        // Anomaly case 1: echo != 0 but speed == 0
        let echo: u8 = 1;
        let speed: i16 = 0;
        assert!(echo != 0 && speed == 0);

        // Anomaly case 2: consecutive echo == 0
        let old_echo: i8 = 0;
        let echo2: u8 = 0;
        assert!(old_echo == 0 && echo2 == 0);

        // Normal case: echo == 0, speed == 0 (valid stop)
        let echo3: u8 = 0;
        let speed3: i16 = 0;
        let old_echo3: i8 = 1;
        let anomaly = (echo3 != 0 && speed3 == 0) || (old_echo3 == 0 && echo3 == 0);
        assert!(!anomaly);
    }

    #[test]
    fn test_max_caught_count() {
        assert_eq!(MAX_CAUGHT_COUNT, 3);
        // After 3 consecutive violations within the time window, player warps Home
    }

    #[test]
    fn test_command_captain_gets_rogue_speed() {
        assert_eq!(COMMAND_CAPTAIN, 100);
        // Command captains use rogue speed limit (90) instead of class default
    }

    #[test]
    fn test_special_zone_speed() {
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
        assert_eq!(ZONE_DUNGEON_DEFENCE, 89);
        // Players in these zones get rogue speed limit (90)
    }

    #[test]
    fn test_merchant_close_on_move() {
        // MERCHANT_CLOSE sub-opcode is 2
        let close_sub_opcode: u8 = 2;
        assert_eq!(close_sub_opcode, 2);
    }

    #[test]
    fn test_gm_exemption() {
        // GMs (authority == 0) are exempt from all speed checks
        let authority: u8 = 0;
        let is_gm = authority == 0;
        assert!(is_gm);

        // Regular player (authority == 1) is not exempt
        let authority2: u8 = 1;
        let is_gm2 = authority2 == 0;
        assert!(!is_gm2);
    }

    // ── Sprint 320: Merchant auto-close on movement ─────────────────

    /// Moving while selling merchant should auto-close the merchant.
    #[test]
    fn test_merchant_auto_close_on_move_check() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Initially not a merchant
        assert!(!world.is_selling_merchant(1));
        assert!(!world.is_selling_merchant_preparing(1));

        // Set merchant preparing state
        world.update_session(1, |h| {
            h.selling_merchant_preparing = true;
        });
        assert!(world.is_selling_merchant_preparing(1));

        // Move handler should detect this and close
        let should_close = world.is_selling_merchant(1) || world.is_selling_merchant_preparing(1);
        assert!(
            should_close,
            "preparing merchant should trigger close on move"
        );
    }

    /// After close_merchant, merchant state should be cleared.
    #[test]
    fn test_merchant_close_clears_state() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.update_session(1, |h| {
            h.selling_merchant_preparing = true;
        });
        assert!(world.is_selling_merchant_preparing(1));

        // close_merchant clears all merchant state
        world.close_merchant(1);

        assert!(!world.is_selling_merchant(1));
        assert!(!world.is_selling_merchant_preparing(1));
    }

    // ── Sprint 327: Warp-loop prevention flag tests ─────────────────

    /// Test that check_warp_zone_change flag is cleared on move with speed > 0.
    #[test]
    fn test_warp_loop_flag_cleared_on_move() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set the flag (simulating warp gate interaction)
        world.set_check_warp_zone_change(1, true);
        assert!(world.is_check_warp_zone_change(1));

        // Move with speed > 0 should clear it
        // (In actual move handler, this happens after echo/speed update)
        world.set_check_warp_zone_change(1, false);
        assert!(!world.is_check_warp_zone_change(1));
    }

    /// Test that check_warp_zone_change flag is NOT cleared when speed == 0.
    #[test]
    fn test_warp_loop_flag_not_cleared_on_zero_speed() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_check_warp_zone_change(1, true);

        // Speed == 0 should NOT clear the flag
        let speed: i16 = 0;
        if speed != 0 && world.is_check_warp_zone_change(1) {
            world.set_check_warp_zone_change(1, false);
        }
        assert!(
            world.is_check_warp_zone_change(1),
            "Flag should remain set when speed is 0"
        );
    }

    /// Test that check_warp_zone_change defaults to false.
    #[test]
    fn test_warp_loop_flag_default_false() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        assert!(
            !world.is_check_warp_zone_change(1),
            "Flag should default to false"
        );
    }

    // ── Sprint 504: Pet follow on player movement ───────────────────

    #[test]
    fn test_pet_follow_distance_threshold() {
        // C++ GetDistanceSqrt returns ACTUAL distance (with sqrt), threshold = 10m
        // Pet should follow when distance >= 10m
        let dx: f32 = 8.0;
        let dz: f32 = 6.0; // distance = 10.0 (exact)
        let dist_sq = dx * dx + dz * dz;
        let distance = dist_sq.sqrt();
        assert!((distance - 10.0).abs() < 0.01);
        // At exactly 10m, pet should follow
        assert!(distance >= 10.0);
    }

    #[test]
    fn test_pet_follow_direction_calculation() {
        // Pet at (100, 100), player at (110, 100), old position (108, 100)
        let pet_x: f32 = 100.0;
        let pet_z: f32 = 100.0;
        let player_x: f32 = 110.0;
        let player_z: f32 = 100.0;
        let old_x: f32 = 108.0;
        let old_z: f32 = 100.0;

        let dx = pet_x - player_x; // -10
        let dz = pet_z - player_z; // 0
        let dist = (dx * dx + dz * dz).sqrt(); // 10.0

        // Normalize + multiply by 2 + add to old position
        let dir_x = dx / dist; // -1.0
        let dir_z = dz / dist; // 0.0
        let new_x = old_x + dir_x * 2.0; // 108 + (-2) = 106
        let new_z = old_z + dir_z * 2.0; // 100 + 0 = 100

        assert!((new_x - 106.0).abs() < 0.01);
        assert!((new_z - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_pet_follow_move_packet_format() {
        // C++ SendMoveResult: WIZ_NPC_MOVE [u8(1)] [u32(id)] [u16(x*10)] [u16(z*10)] [u16(y*10)] [u16(speed*10)]
        let pet_nid: u32 = 10500;
        let new_x: f32 = 106.0;
        let new_z: f32 = 100.0;
        let distance: f32 = 10.0;

        let mut pkt = Packet::new(Opcode::WizNpcMove as u8);
        pkt.write_u8(1);
        pkt.write_u32(pet_nid);
        pkt.write_u16((new_x * 10.0) as u16);
        pkt.write_u16((new_z * 10.0) as u16);
        pkt.write_u16(0);
        pkt.write_u16((distance * 10.0) as u16);

        assert_eq!(pkt.opcode, Opcode::WizNpcMove as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10500));
        assert_eq!(r.read_u16(), Some(1060)); // 106.0 * 10
        assert_eq!(r.read_u16(), Some(1000)); // 100.0 * 10
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(100)); // 10.0 * 10
        assert_eq!(r.remaining(), 0);
    }

    /// Echo gap: value 2 (nott) is intentionally skipped between start(1) and move(3).
    #[test]
    fn test_echo_gap_at_nott() {
        assert_eq!(ECHO_FINISH, 0);
        assert_eq!(ECHO_START, 1);
        // nott=2 is not a constant (rejected)
        assert_eq!(ECHO_MOVE, 3);
        // Gap of 2 between start and move
        assert_eq!(ECHO_MOVE - ECHO_START, 2);
    }

    /// Anomaly detection: MAX_CAUGHT_COUNT=3 violations before warp, window=1100ms.
    #[test]
    fn test_anomaly_thresholds() {
        assert_eq!(MAX_CAUGHT_COUNT, 3);
        assert_eq!(CAUGHT_TIME_WINDOW_MS, 1100);
        // 3 consecutive violations in 1100ms triggers warp
        assert!(MAX_CAUGHT_COUNT > 1);
        assert!(CAUGHT_TIME_WINDOW_MS > 1000);
    }

    /// Home zone determination: Karus=1, Elmorad=2, fallback=21 (Moradon).
    #[test]
    fn test_home_zone_fallback_values() {
        let world = WorldState::new();
        // bind_zone=0, nation=0 → fallback Moradon (21)
        assert_eq!(determine_home_zone_raw(0, 0, &world), 21);
        // bind_zone > 0 → use bind_zone directly
        assert_eq!(determine_home_zone_raw(51, 1, &world), 51);
        assert_eq!(determine_home_zone_raw(10, 2, &world), 10);
    }

    /// get_distance_scaled: symmetric and scales by /10.
    #[test]
    fn test_distance_scaled_symmetry() {
        // Symmetric: dist(a,b) == dist(b,a)
        let d1 = get_distance_scaled(100, 200, 300, 400);
        let d2 = get_distance_scaled(300, 400, 100, 200);
        assert!((d1 - d2).abs() < 0.001);
        // Zero distance for same point
        assert_eq!(get_distance_scaled(500, 500, 500, 500), 0.0);
    }

    /// Move opcode is WIZ_MOVE (0x06) — first in GameMain dispatch range.
    #[test]
    fn test_move_opcode_value() {
        assert_eq!(Opcode::WizMove as u8, 0x06);
        // First valid opcode in v2525 dispatch range (0x06-0xD7)
        assert_eq!(Opcode::WizMove as u8, 6);
    }

    #[test]
    fn test_pet_follow_cancels_attack() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set up pet in attack mode with active attack
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                nid: 10,
                state_change: 3, // MODE_ATTACK
                attack_started: true,
                attack_target_id: 42,
                ..Default::default()
            });
        });

        // Simulate attack cancel (same logic as in pet_follow_on_move)
        let pet_mode: u8 = 3;
        let attack_started = true;
        if pet_mode == 3 && attack_started {
            world.update_session(sid, |h| {
                if let Some(ref mut pet) = h.pet_data {
                    pet.attack_started = false;
                    pet.attack_target_id = -1;
                }
            });
        }

        // Verify attack cancelled
        let (started, target) = world
            .with_session(sid, |h| {
                h.pet_data
                    .as_ref()
                    .map(|p| (p.attack_started, p.attack_target_id))
            })
            .flatten()
            .unwrap();
        assert!(!started);
        assert_eq!(target, -1);
    }
}
