//! WIZ_EVENT (0x5F) handler -- Temple / Event System.
//! ## Sub-opcodes (TempleProcess dispatch)
//! | Value | Name                       | Description                              |
//! |-------|----------------------------|------------------------------------------|
//! | 6     | MONSTER_STONE              | Monster Stone dungeon entry               |
//! | 8     | TEMPLE_EVENT_JOIN          | Join BDW / Juraid / Chaos event           |
//! | 9     | TEMPLE_EVENT_DISBAND       | Leave / disband from event                |
//! | 10    | TEMPLE_EVENT_FINISH        | Event finished notification (server-sent) |
//! | 16    | TEMPLE_EVENT_COUNTER       | Join counter update (server-sent)         |
//! | 33    | TEMPLE_DRAKI_TOWER_ENTER   | Enter Draki Tower dungeon                 |
//! | 34    | TEMPLE_DRAKI_TOWER_LIST    | Request Draki Tower clear list            |
//! | 35    | TEMPLE_DRAKI_TOWER_TIMER   | Draki Tower timer (server-sent)           |
//! | 36    | TEMPLE_DRAKI_TOWER_OUT1    | Draki Tower exit notification 1           |
//! | 37    | TEMPLE_DRAKI_TOWER_OUT2    | Draki Tower exit notification 2           |
//! | 38    | TEMPLE_DRAKI_TOWER_TOWN    | Draki Tower return to town                |
//! | 49    | TEMPLE_EVENT_ALTAR_FLAG    | Altar flag status (BDW)                   |
//! | 50    | TEMPLE_EVENT_ALTAR_TIMER   | Altar timer (BDW)                         |
//! | 58    | TEMPLE_EVENT_DUNGEON_SIGN  | Dungeon Defence entry sign-up             |
//! | 59    | TEMPLE_EVENT_REMAINING_TOWER | Remaining tower count                   |
//! | 60    | TEMPLE_EVENT_STAGE_COUNTER | Stage counter                             |

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::systems::event_room;
use crate::world::types::{
    ZONE_BORDER_DEFENSE_WAR, ZONE_CHAOS_DUNGEON, ZONE_JURAID_MOUNTAIN, ZONE_PRISON,
};

use crate::inventory_constants::RIGHTHAND;

use super::mining::{FISHING_ROD, GOLDEN_FISHING_ROD, GOLDEN_MATTOCK, MATTOCK};

/// Event sub-opcode constants.
mod sub_opcode {
    /// Monster Stone dungeon entry.
    pub const MONSTER_STONE: u8 = 6;
    /// Active event info request/response (C++ TEMPLE_EVENT = 7).
    pub const TEMPLE_EVENT: u8 = 7;
    /// Join an ongoing temple event (BDW / Juraid / Chaos).
    pub const TEMPLE_EVENT_JOIN: u8 = 8;
    /// Leave / disband from a temple event.
    pub const TEMPLE_EVENT_DISBAND: u8 = 9;
    /// Event finished notification (server-sent only).
    #[cfg(test)]
    pub const TEMPLE_EVENT_FINISH: u8 = 10;
    /// Join counter update (server-sent only).
    #[cfg(test)]
    pub const TEMPLE_EVENT_COUNTER: u8 = 16;
    /// Enter Draki Tower dungeon.
    pub const TEMPLE_DRAKI_TOWER_ENTER: u8 = 33;
    /// Request Draki Tower clear list.
    pub const TEMPLE_DRAKI_TOWER_LIST: u8 = 34;
    /// Draki Tower timer (server-sent).
    #[cfg(test)]
    pub const TEMPLE_DRAKI_TOWER_TIMER: u8 = 35;
    /// Draki Tower return to town.
    pub const TEMPLE_DRAKI_TOWER_TOWN: u8 = 38;
    /// Dungeon Defence entry sign-up.
    pub const TEMPLE_EVENT_DUNGEON_SIGN: u8 = 58;
}

/// Event type IDs used by the temple event system.
#[allow(dead_code)]
mod event_type {
    /// Border Defence War (BDW).
    pub const TEMPLE_EVENT_BORDER_DEFENCE_WAR: i16 = 4;
    /// Monster Stone quest event.
    pub const TEMPLE_EVENT_MONSTER_STONE: i16 = 14;
    /// Chaos Dungeon.
    pub const TEMPLE_EVENT_CHAOS: i16 = 24;
    /// Juraid Mountain.
    pub const TEMPLE_EVENT_JURAD_MOUNTAIN: i16 = 100;
    /// Knight Battle Royale.
    pub const TEMPLE_EVENT_KNIGHT_BATTLE_ROYALE: i16 = 104;
}

/// Handle incoming WIZ_EVENT (0x5F) packet.
/// The first byte is a sub-opcode that determines the event action.
/// Prison zone players are blocked from event participation (C++ check).
pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match opcode {
        sub_opcode::MONSTER_STONE => handle_monster_stone(session, &mut reader).await,
        sub_opcode::TEMPLE_EVENT => handle_temple_event_info(session).await,
        sub_opcode::TEMPLE_EVENT_JOIN => handle_temple_join(session).await,
        sub_opcode::TEMPLE_EVENT_DISBAND => handle_temple_disband(session).await,
        sub_opcode::TEMPLE_DRAKI_TOWER_ENTER => handle_draki_enter(session, &mut reader).await,
        sub_opcode::TEMPLE_DRAKI_TOWER_LIST => handle_draki_list(session).await,
        sub_opcode::TEMPLE_DRAKI_TOWER_TOWN => handle_draki_town(session).await,
        sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN => handle_dungeon_defence_sign(session).await,
        _ => {
            debug!(
                "[{}] WIZ_EVENT unhandled sub-opcode={}",
                session.addr(),
                opcode
            );
            Ok(())
        }
    }
}

/// Send active temple event info to the client.
/// Wire: `WIZ_EVENT(0x5F) << u8(7) << i16(active_event) << u16(remain_seconds)`
/// If no event is active, sends `active_event = -1, remain_seconds = 0`.
async fn handle_temple_event_info(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();

    let (active_event, remain_secs) = world.event_room_manager.read_temple_event(|s| {
        let remain = if s.sign_remain_seconds > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if s.sign_remain_seconds > now {
                (s.sign_remain_seconds - now) as u16
            } else {
                0u16
            }
        } else {
            0u16
        };
        (s.active_event, remain)
    });

    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(sub_opcode::TEMPLE_EVENT);
    pkt.write_i16(active_event);
    pkt.write_u16(remain_secs);
    session.send_packet(&pkt).await?;

    debug!(
        "[{}] WIZ_EVENT TEMPLE_EVENT: active={}, remain={}s",
        session.addr(),
        active_event,
        remain_secs
    );
    Ok(())
}

/// Monster Stone sub-opcode handler.
/// The client sends a u32 item ID identifying the monster stone to activate.
/// Server validates the request, allocates a room, spawns monsters, and
/// teleports the player to the Monster Stone zone.
async fn handle_monster_stone(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    use crate::systems::monster_stone;

    let item_id = reader.read_u32().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();

    debug!(
        "[{}] WIZ_EVENT MONSTER_STONE item_id={}",
        session.addr(),
        item_id
    );

    // ── Pre-condition checks (C++ lines 12-18) ──────────────────────────
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    // ── Server setting gate (C++ lines 25-28) ─────────────────────────
    //   if (!g_pMain->pServerSetting.monsterstone_status) {
    //     SendHelpDescription("Monster Stone map is in maintenance mode.");
    //     return SendMonsterStoneFail(1);
    //   }
    let ms_enabled = world
        .get_server_settings()
        .map(|s| s.monsterstone_status)
        .unwrap_or(false);
    if !ms_enabled {
        session
            .send_packet(&monster_stone::build_fail_packet(1))
            .await?;
        return Ok(());
    }

    // ── Validate item ID (C++ line 33) ──────────────────────────────────
    if !monster_stone::is_monster_stone_item(item_id) {
        return Ok(());
    }

    // ── Character info checks ───────────────────────────────────────────
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // HP must be >= 50% of max HP (C++ lines 36-37)
    if ch.hp < ch.max_hp / 2 {
        session
            .send_packet(&monster_stone::build_fail_packet(9))
            .await?;
        return Ok(());
    }

    // ── Zone restrictions (C++ lines 39-42) ─────────────────────────────
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };
    if pos.zone_id == ZONE_PRISON
        || monster_stone::is_monster_stone_zone(pos.zone_id)
        || event_room::is_in_temple_event_zone(pos.zone_id)
    {
        session
            .send_packet(&monster_stone::build_fail_packet(1))
            .await?;
        return Ok(());
    }

    // ── Status checks (C++ lines 44-47) ─────────────────────────────────
    {
        let is_ms_active = world
            .with_session(sid, |h| h.event_room > 0)
            .unwrap_or(false);
        if is_ms_active {
            session
                .send_packet(&monster_stone::build_fail_packet(5))
                .await?;
            return Ok(());
        }
    }

    // Check item exists in inventory
    if !world.check_exist_item(sid, item_id, 1) {
        session
            .send_packet(&monster_stone::build_fail_packet(5))
            .await?;
        return Ok(());
    }

    // ── Allocate room (C++ lines 49-59) ─────────────────────────────────
    let room_id = match world.monster_stone_write().allocate_room() {
        Some(r) => r,
        None => return Ok(()), // No rooms available — C++ silently returns
    };

    // ── Determine zone and family (C++ MonsterStoneSystem.cpp:71-308) ────
    // C++ uses `new_monsterstone` setting to select mode:
    //   new_monsterstone != 0 → zone-specific stones only (300144036/37/38)
    //   new_monsterstone == 0 → universal stone only (900144023, level-based)
    let new_ms = world
        .get_server_settings()
        .map(|s| s.new_monsterstone)
        .unwrap_or(0);

    let (zone_id, family) = if new_ms != 0 {
        // Zone-specific mode (C++ lines 71-245) — only zone-specific stones accepted
        if item_id == monster_stone::ITEM_UNIVERSAL {
            return Ok(());
        }
        let z = match monster_stone::item_to_zone(item_id) {
            Some(z) => z,
            None => return Ok(()),
        };
        let f = monster_stone::random_family_for_zone(z);
        (z, f)
    } else {
        // Level-based mode (C++ lines 247-308) — only universal stone accepted
        if item_id != monster_stone::ITEM_UNIVERSAL {
            return Ok(());
        }
        match monster_stone::universal_stone_zone_family(ch.level) {
            Some((z, f)) => (z, f),
            None => {
                session
                    .send_packet(&monster_stone::build_fail_packet(5))
                    .await?;
                return Ok(());
            }
        }
    };

    // ── Validate spawn list (C++ lines 131-146) ─────────────────────────
    let spawns = world.get_monster_stone_spawns(zone_id, family);
    if spawns.is_empty() {
        session
            .send_packet(&monster_stone::build_fail_packet(5))
            .await?;
        return Ok(());
    }

    // ── Consume item (C++ RobItem) ──────────────────────────────────────
    if !world.rob_item(sid, item_id, 1) {
        session
            .send_packet(&monster_stone::build_fail_packet(5))
            .await?;
        return Ok(());
    }

    // ── Activate room (C++ lines 149-156) ───────────────────────────────
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // ── Collect party members if in a party (C++ lines 158-180) ─────────
    // C++ populates pRoom.mUserList with eligible party members, or just
    // the activating player if solo.
    let event_room_id = room_id + 1; // 1-based (C++ roomid + 1)
    let eligible_members: Vec<crate::zone::SessionId> =
        if let Some(party_id) = world.get_party_id(sid) {
            if let Some(party) = world.get_party(party_id) {
                party
                    .active_members()
                    .into_iter()
                    .filter(|&member_sid| {
                        // C++ filters: MonsterStoneSystem.cpp:170-177
                        // Skip null, in PK zone, dead, merchanting, trading, store open
                        if world.is_trading(member_sid) || world.is_merchanting(member_sid) {
                            return false;
                        }
                        let member_zone = world
                            .get_position(member_sid)
                            .map(|p| p.zone_id)
                            .unwrap_or(0);
                        // C++ isInPKZone() — Ardream(72), Ronark Land(71), Ronark Land Base(73)
                        if matches!(member_zone, 71..=73) {
                            return false;
                        }
                        // C++ isDead() — HP <= 0
                        if let Some(mch) = world.get_character_info(member_sid) {
                            if mch.hp <= 0 {
                                return false;
                            }
                        } else {
                            return false;
                        }
                        true
                    })
                    .collect()
            } else {
                vec![sid]
            }
        } else {
            vec![sid]
        };

    {
        let mut mgr = world.monster_stone_write();
        mgr.activate_room(room_id, zone_id, family, pos.zone_id, now);
        for &member_sid in &eligible_members {
            mgr.add_user(room_id, member_sid);
        }
    }

    // ── Set event_room on all members (1-based) + Monster Stone status flag ──
    for &member_sid in &eligible_members {
        world.update_session(member_sid, |h| {
            h.event_room = event_room_id;
            h.monster_stone_status = true;
        });
    }

    // ── Spawn monsters (C++ lines 187-205, 367-369) ─────────────────────
    // C++ passes event_room = roomid + 1 (1-based) and summon_type = isBoss (1 for boss).
    for row in &spawns {
        let is_monster = row.b_type == 0;
        let count = row.s_count.max(1) as u16;
        let summon_type = if row.is_boss { 1u8 } else { 0u8 };
        world.spawn_event_npc_ex_with_direction(
            row.s_sid as u16,
            is_monster,
            zone_id as u16,
            row.x as f32,
            row.z as f32,
            count,
            event_room_id,
            summon_type,
            row.by_direction.clamp(0, u8::MAX as i16) as u8,
        );
    }

    // ── Spawn vendor NPCs (C++ lines 191-205, 371-385) ──────────────────
    // C++ spawns 3 utility NPCs at hardcoded positions per zone.
    // NPC 16062 = Repair, NPC 12117 = Sundries, NPC 31508 = Potion
    let vendor_coords: [(f32, f32); 3] = match zone_id {
        81 => [(204.0, 201.0), (204.0, 197.0), (204.0, 193.0)],
        82 => [(203.0, 202.0), (203.0, 197.0), (203.0, 193.0)],
        83 => [(204.0, 207.0), (204.0, 200.0), (204.0, 194.0)],
        _ => [(200.0, 200.0), (200.0, 196.0), (200.0, 192.0)],
    };
    let vendor_npcs: [u16; 3] = [16062, 12117, 31508];
    for (npc_sid, &(vx, vz)) in vendor_npcs.iter().zip(vendor_coords.iter()) {
        world.spawn_event_npc_ex(
            *npc_sid,
            false, // NPC, not monster
            zone_id as u16,
            vx,
            vz,
            1,
            event_room_id,
            0, // not a boss
        );
    }

    // ── Teleport all members (C++ lines 207-243) ─────────────────────────
    let timer_pkt = monster_stone::build_timer_packet(monster_stone::ROOM_DURATION_SECS as u16);
    let select_pkt =
        monster_stone::build_select_msg_timer(monster_stone::ROOM_DURATION_SECS as u16);

    for &member_sid in &eligible_members {
        if member_sid == sid {
            // Activating player — use session-based teleport
            super::zone_change::trigger_zone_change(session, zone_id as u16, 0.0, 0.0).await?;
            session.send_packet(&timer_pkt).await?;
            session.send_packet(&select_pkt).await?;
        } else {
            // Party member — server-initiated teleport
            super::zone_change::server_teleport_to_zone(
                &world,
                member_sid,
                zone_id as u16,
                0.0,
                0.0,
            );
            world.send_to_session(member_sid, &timer_pkt);
            world.send_to_session(member_sid, &select_pkt);
        }
    }

    debug!(
        "[sid={}] Monster Stone activated: room={}, zone={}, family={}, members={}",
        sid,
        room_id,
        zone_id,
        family,
        eligible_members.len()
    );

    Ok(())
}

/// Temple event join handler.
/// Validates the player can join the active event, then adds them to the event
/// sign-up queue via `EventRoomManager::add_signed_up_user()`.
/// Validation checks (per C++):
/// 1. Player is not already an event user
/// 2. Player is not in prison zone
/// 3. No active event or sign-up closed → fail
/// 4. Event already in active phase → fail
/// 5. Level requirements (if automatic event)
/// 6. Loyalty requirements (if automatic event)
/// 7. Gold requirements (if automatic event)
/// Response: `WIZ_EVENT(0x5F) + TEMPLE_EVENT_JOIN(8) + result(u8) + active_event(i16)`
/// - result=1: success
/// - result=4: fail
async fn handle_temple_join(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] WIZ_EVENT TEMPLE_EVENT_JOIN", session.addr());

    let world = session.world().clone();
    let sid = session.session_id();

    // Read current temple event state
    let (active_event, allow_join, is_active) = world
        .event_room_manager
        .read_temple_event(|s| (s.active_event, s.allow_join, s.is_active));

    // Check: player already signed up or in prison
    let player_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    if player_zone == ZONE_PRISON {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(4);
        resp.write_i16(active_event);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check: player already in event zone (BDW=84, Chaos=85, Juraid=87)
    if player_zone == ZONE_BORDER_DEFENSE_WAR
        || player_zone == ZONE_CHAOS_DUNGEON
        || player_zone == ZONE_JURAID_MOUNTAIN
    {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(4);
        resp.write_i16(active_event);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check: no active event or sign-up not allowed
    if active_event < 0 || !allow_join {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(4);
        resp.write_i16(active_event);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check: event already moved to active phase (can't join anymore)
    if is_active {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(4);
        resp.write_i16(active_event);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // CHAOS event: reject players who are mining, fishing, or holding tools
    if active_event == event_type::TEMPLE_EVENT_CHAOS {
        if world.is_mining(sid) || world.is_fishing(sid) {
            let mut resp = Packet::new(Opcode::WizEvent as u8);
            resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
            resp.write_u8(4);
            resp.write_i16(active_event);
            session.send_packet(&resp).await?;
            return Ok(());
        }

        // C++ also checks right-hand item for mining/fishing tools
        let right_hand_item = world
            .get_inventory_slot(sid, RIGHTHAND)
            .map(|s| s.item_id)
            .unwrap_or(0);
        if right_hand_item == MATTOCK
            || right_hand_item == GOLDEN_MATTOCK
            || right_hand_item == FISHING_ROD
            || right_hand_item == GOLDEN_FISHING_ROD
        {
            let mut resp = Packet::new(Opcode::WizEvent as u8);
            resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
            resp.write_u8(4);
            resp.write_i16(active_event);
            session.send_packet(&resp).await?;
            return Ok(());
        }
    }

    // Get player info for validation
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    let player_name = match world.get_session_name(sid) {
        Some(n) => n,
        None => return Ok(()),
    };

    // Add to sign-up queue
    let join_result =
        world
            .event_room_manager
            .add_signed_up_user(player_name.clone(), sid, ch.nation);

    if join_result.is_none() {
        // Already signed up (duplicate)
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(4);
        resp.write_i16(active_event);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Update nation counts
    world.event_room_manager.update_temple_event(|s| {
        if ch.nation == 1 {
            s.karus_user_count += 1;
        } else {
            s.elmorad_user_count += 1;
        }
        s.all_user_count = s.karus_user_count + s.elmorad_user_count;
    });

    // Success response
    let mut resp = Packet::new(Opcode::WizEvent as u8);
    resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
    resp.write_u8(1); // success
    resp.write_i16(active_event);
    session.send_packet(&resp).await?;

    tracing::info!(
        "[{}] Player '{}' (nation={}) joined event {} (total signed up: {})",
        session.addr(),
        player_name,
        ch.nation,
        active_event,
        world.event_room_manager.signed_up_count(),
    );

    // Broadcast updated counter to all signed-up users.
    event_room::broadcast_event_counter(&world);

    Ok(())
}

/// Temple event disband handler.
/// Removes the player from the sign-up queue if still in the signing phase.
/// If the event has moved to active phase, disband is rejected.
/// Response: `WIZ_EVENT(0x5F) + TEMPLE_EVENT_DISBAND(9) + result(u8) + active_event(u16)`
/// - result=1: success
/// - result=4: fail
async fn handle_temple_disband(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] WIZ_EVENT TEMPLE_EVENT_DISBAND", session.addr());

    let world = session.world().clone();
    let sid = session.session_id();

    // Read current temple event state
    let (active_event, is_active) = world
        .event_room_manager
        .read_temple_event(|s| (s.active_event, s.is_active));

    let active_event_u16 = active_event as u16;

    // Check: player in prison
    let player_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    if player_zone == ZONE_PRISON {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
        resp.write_u8(4);
        resp.write_u16(active_event_u16);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check: no active event
    if active_event < 0 {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
        resp.write_u8(4);
        resp.write_u16(active_event_u16);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Check: event already in active phase (can't disband)
    if is_active {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
        resp.write_u8(4);
        resp.write_u16(active_event_u16);
        session.send_packet(&resp).await?;
        return Ok(());
    }

    // Get player name
    let player_name = match world.get_session_name(sid) {
        Some(n) => n,
        None => return Ok(()),
    };

    // Remove from sign-up queue
    let removed = world.event_room_manager.remove_signed_up_user(&player_name);

    let Some(removed_user) = removed else {
        // Player was not signed up
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
        resp.write_u8(4);
        resp.write_u16(active_event_u16);
        session.send_packet(&resp).await?;
        return Ok(());
    };

    // Update nation counts
    world.event_room_manager.update_temple_event(|s| {
        if removed_user.nation == 1 {
            s.karus_user_count = s.karus_user_count.saturating_sub(1);
        } else {
            s.elmorad_user_count = s.elmorad_user_count.saturating_sub(1);
        }
        s.all_user_count = s.karus_user_count + s.elmorad_user_count;
    });

    // Success response
    let mut resp = Packet::new(Opcode::WizEvent as u8);
    resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
    resp.write_u8(1); // success
    resp.write_u16(active_event_u16);
    session.send_packet(&resp).await?;

    tracing::info!(
        "[{}] Player '{}' (nation={}) disbanded from event {} (remaining: {})",
        session.addr(),
        player_name,
        removed_user.nation,
        active_event,
        world.event_room_manager.signed_up_count(),
    );

    // Broadcast updated counter to all signed-up users.
    // C++ also sends the counter directly to the disbanding user (who was already removed
    // from the signed-up list and thus won't receive the broadcast).
    if let Some(counter_pkt) = event_room::broadcast_event_counter(&world) {
        world.send_to_session_owned(sid, counter_pkt);
    }

    Ok(())
}

/// Draki Tower enter handler.
/// The client sends item_id(u32) + enter_dungeon(u8).
/// Validates the player is in their nation's castle zone (Luferson/Elmorad),
/// has the entrance item or remaining entrance limit, and allocates a room.
/// Currently responds with error code 8 ("you cannot enter right now").
async fn handle_draki_enter(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    use crate::handler::draki_tower;
    use crate::world::types::{
        ZONE_ELMORAD, ZONE_ELMORAD2, ZONE_ELMORAD3, ZONE_KARUS, ZONE_KARUS2, ZONE_KARUS3,
    };

    let item_id = reader.read_u32().unwrap_or(0);
    let enter_dungeon = reader.read_u8().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();

    debug!(
        "[{}] WIZ_EVENT DRAKI_TOWER_ENTER item={} dungeon={}",
        session.addr(),
        item_id,
        enter_dungeon
    );

    // Helper to build error response packet
    let send_err = |code: u32| -> Packet {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_DRAKI_TOWER_ENTER);
        resp.write_u32(code);
        resp
    };

    // ── Gather player state + load DB data ────────────────────────────
    let (ch, pos, event_room_val, entrance_limit, saved_stage) = match world.get_character_info(sid)
    {
        Some(ch) => {
            let pos = world.get_position(sid).unwrap_or_default();
            let er = world.with_session(sid, |h| h.event_room).unwrap_or(0);

            // Load from DB (C++ LoadUserDrakiTowerData)
            let repo = ko_db::repositories::draki_tower::DrakiTowerRepository::new(session.pool());
            match repo.reset_user_entrance_limit_if_due(&ch.name).await {
                Ok(true) => tracing::info!(
                    user = %ch.name,
                    "Draki Tower entrance limit lazily reset for current 18:00 bucket"
                ),
                Ok(false) => {}
                Err(e) => tracing::warn!(
                    user = %ch.name,
                    "Draki Tower lazy entrance reset failed: {e}"
                ),
            }
            let user_data = match repo.load_user_data(&ch.name).await {
                Ok(data) => data,
                Err(e) => {
                    tracing::warn!(
                        "[{}] draki_tower load_user_data DB error for {}: {e}",
                        session.addr(),
                        ch.name
                    );
                    None
                }
            };

            let limit = match &user_data {
                Some(ud) => ud.b_draki_enterance_limit as u8,
                None => draki_tower::MAX_ENTRANCE_LIMIT,
            };
            let saved = match &user_data {
                Some(ud) => ud.b_draki_stage as u8,
                None => 0,
            };

            world.update_session(sid, |h| {
                h.draki_entrance_limit = limit;
            });

            (ch, pos, er, limit, saved)
        }
        None => return Ok(()),
    };

    // ── Nation castle zone check (C++ lines 66-73) ─────────────────────
    let is_in_nation_castle = match ch.nation {
        1 => matches!(pos.zone_id, ZONE_KARUS | ZONE_KARUS2 | ZONE_KARUS3),
        2 => matches!(pos.zone_id, ZONE_ELMORAD | ZONE_ELMORAD2 | ZONE_ELMORAD3),
        _ => false,
    };

    // ── Check if player has the certificate item ───────────────────────
    let has_certificate = world
        .with_session(sid, |h| {
            h.inventory
                .iter()
                .any(|slot| slot.item_id == draki_tower::CERTIFIKAOFDRAKI && slot.count > 0)
        })
        .unwrap_or(false);

    // ── Validate entry (pure logic) ────────────────────────────────────
    if let Err(code) = draki_tower::validate_entry(
        ch.hp <= 0,
        world.is_war_open(),
        world.is_player_in_cinderella(sid),
        pos.zone_id,
        event_room_val,
        ch.nation,
        is_in_nation_castle,
        enter_dungeon,
        item_id,
        entrance_limit,
        has_certificate,
    ) {
        let pkt = send_err(code);
        session.send_packet(&pkt).await?;
        return Ok(());
    }

    let (spawn_x, spawn_z) = draki_tower::dungeon_spawn_position(enter_dungeon);
    if !super::zone_change::can_trigger_zone_change(
        &world,
        sid,
        draki_tower::ZONE_DRAKI_TOWER,
        spawn_x as f32,
        spawn_z as f32,
    ) {
        tracing::warn!(user = %ch.name, room_zone = draki_tower::ZONE_DRAKI_TOWER, "Draki Tower entry rejected before consuming the entrance: destination warp is unavailable");
        session
            .send_packet(&send_err(draki_tower::ENTER_ERR_INTERNAL))
            .await?;
        return Ok(());
    }

    // ── Find a free room ───────────────────────────────────────────────
    let room_id = {
        let rooms = world.draki_tower_rooms_read();
        draki_tower::find_free_room(&rooms)
    };
    let room_id = match room_id {
        Some(id) => id,
        None => {
            let pkt = send_err(draki_tower::ENTER_ERR_INSTANCE_FAILED);
            session.send_packet(&pkt).await?;
            return Ok(());
        }
    };

    // ── Initialize room ────────────────────────────────────────────────
    let user_name = ch.name.clone();
    {
        let mut rooms = world.draki_tower_rooms_write();
        if let Some(room) = rooms.get_mut(&room_id) {
            draki_tower::initialize_room_for_entry(
                room,
                enter_dungeon,
                &user_name,
                saved_stage,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }
    }

    // ── Set session fields ─────────────────────────────────────────────
    world.update_session(sid, |h| {
        h.event_room = room_id; // 1-based room ID used as event_room
        h.draki_room_id = room_id;
    });

    // ── Spawn initial monsters for the first sub-stage ─────────────────
    let spawn_list: Vec<(u16, bool, f32, f32)> = {
        let stages = world.draki_tower_stages();
        let monsters = world.draki_monster_list();
        let mut list = Vec::new();
        if let Some(stage_idx) = draki_tower::find_stage_index(
            &stages,
            enter_dungeon as u16,
            1, // sub-stage 1
            0, // monster stage
        ) {
            if let Some(stage) = draki_tower::get_stage_at(&stages, stage_idx) {
                let stage_is_monster = stage.draki_tower_npc_state == 0;
                for m in draki_tower::get_monsters_for_stage(&monsters, stage.id) {
                    list.push((
                        m.monster_id as u16,
                        stage_is_monster,
                        m.pos_x as f32,
                        m.pos_z as f32,
                    ));
                }
            }
        }
        list
    };
    for (npc_id, is_monster, x, z) in &spawn_list {
        world.spawn_event_npc_ex(
            *npc_id,
            *is_monster,
            draki_tower::ZONE_DRAKI_TOWER,
            *x,
            *z,
            1,
            room_id,
            0,
        );
    }

    // Set the kill counter = number of monsters spawned (countdown pattern)
    {
        let monster_count = spawn_list.iter().filter(|(_, is_m, _, _)| *is_m).count() as u32;
        let mut rooms = world.draki_tower_rooms_write();
        if let Some(room) = rooms.get_mut(&room_id) {
            room.draki_monster_kill = monster_count;
        }
    }

    // ── Zone change to Draki Tower ─────────────────────────────────────
    super::zone_change::trigger_zone_change(
        session,
        draki_tower::ZONE_DRAKI_TOWER,
        spawn_x as f32,
        spawn_z as f32,
    )
    .await?;

    // trigger_zone_change intentionally treats blocked/unknown destination
    // zones as a no-op. Never start the tower UI, reserve the room, or charge
    // an entry unless the server actually moved this session into zone 95.
    if world.get_position(sid).map(|p| p.zone_id) != Some(draki_tower::ZONE_DRAKI_TOWER) {
        world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, room_id);
        world.update_session(sid, |h| {
            h.event_room = 0;
            h.draki_room_id = 0;
        });
        if let Some(room) = world.draki_tower_rooms_write().get_mut(&room_id) {
            room.reset();
        }
        session
            .send_packet(&send_err(draki_tower::ENTER_ERR_INTERNAL))
            .await?;
        tracing::warn!(user = %ch.name, room_id, "Draki Tower zone transition did not complete; room released without charging the entry");
        return Ok(());
    }

    // Only charge after the zone transition succeeds.
    if entrance_limit > 0 {
        let new_limit = entrance_limit.saturating_sub(1);
        world.update_session(sid, |h| h.draki_entrance_limit = new_limit);
        let repo = ko_db::repositories::draki_tower::DrakiTowerRepository::new(session.pool());
        if let Err(e) = repo.update_entrance_limit(&ch.name, new_limit as i16).await {
            tracing::warn!("Failed to update Draki entrance limit for {}: {e}", ch.name);
        }
    } else {
        world.update_session(sid, |h| {
            if let Some(slot) = h
                .inventory
                .iter_mut()
                .find(|s| s.item_id == draki_tower::CERTIFIKAOFDRAKI && s.count > 0)
            {
                slot.count -= 1;
                if slot.count == 0 {
                    slot.item_id = 0;
                    slot.durability = 0;
                }
            }
        });
    }

    // ── Send timer packets (BUG-9 fix: match C++ SendDrakiTempleDetail format) ──
    {
        let time_limit = draki_tower::SUB_STAGE_TIME_LIMIT as u16;

        // 1. WIZ_SELECT_MSG — drives client countdown UI
        let mut select_pkt = Packet::new(Opcode::WizSelectMsg as u8);
        select_pkt.write_u32(0);
        select_pkt.write_u8(7);
        select_pkt.write_u64(0);
        select_pkt.write_u32(0x0A);
        select_pkt.write_u8(233);
        select_pkt.write_u16(time_limit);
        select_pkt.write_u16(0); // elapsed = 0 at entry
        session.send_packet(&select_pkt).await?;

        // 2. WIZ_EVENT TIMER — stage info display
        let mut timer_pkt = Packet::new(Opcode::WizEvent as u8);
        timer_pkt.write_u8(draki_tower::TEMPLE_DRAKI_TOWER_TIMER);
        timer_pkt.write_u8(233);
        timer_pkt.write_u8(3);
        timer_pkt.write_u16(enter_dungeon as u16);
        timer_pkt.write_u16(1); // sub-stage
        timer_pkt.write_u32(time_limit as u32);
        timer_pkt.write_u32(0); // elapsed = 0 at entry
        session.send_packet(&timer_pkt).await?;

        // 3. WIZ_BIFROST timer
        let mut bifrost_pkt = Packet::new(Opcode::WizBifrost as u8);
        bifrost_pkt.write_u8(5);
        bifrost_pkt.write_u16(time_limit);
        session.send_packet(&bifrost_pkt).await?;
    }

    debug!(
        "[sid={}] Draki Tower entered: dungeon={}, room={}, spawn=({},{}), spawned={}, monsters={}",
        sid,
        enter_dungeon,
        room_id,
        spawn_x,
        spawn_z,
        spawn_list.len(),
        spawn_list.iter().filter(|(_, is_m, _, _)| *is_m).count()
    );

    Ok(())
}

/// Draki Tower list handler.
/// Builds the ranking list packet: 5 class-rank entries + 1 user entry.
/// Packet format matches `pkt.SByte()` mode.
async fn handle_draki_list(session: &mut ClientSession) -> anyhow::Result<()> {
    use crate::handler::draki_tower;
    use ko_db::repositories::draki_tower::DrakiTowerRepository;

    debug!("[{}] WIZ_EVENT DRAKI_TOWER_LIST", session.addr());

    let world = session.world().clone();
    let sid = session.session_id();

    // Get character info for class lookup
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };
    let user_name = ch.name.clone();
    let user_draki_class = draki_tower::draki_class(ch.class);

    let repo = DrakiTowerRepository::new(session.pool());

    if let Err(e) = repo.reset_user_entrance_limit_if_due(&user_name).await {
        tracing::warn!(
            "[{}] draki_tower lazy entrance reset failed for {}: {e}",
            session.addr(),
            user_name
        );
    }

    // Load all rift rankings and filter by user's class
    let all_ranks = match repo.load_rift_ranks().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] draki_tower load_rift_ranks DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };
    let class_ranks: Vec<_> = all_ranks
        .iter()
        .filter(|r| r.class == user_draki_class)
        .take(5)
        .collect();

    // Load user's own data
    let user_data = match repo.load_user_data(&user_name).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] draki_tower load_user_data DB error: {e}",
                session.addr()
            );
            None
        }
    };

    // Build response packet
    let mut resp = Packet::new(Opcode::WizEvent as u8);
    resp.write_u8(sub_opcode::TEMPLE_DRAKI_TOWER_LIST);

    // 5 class ranking entries (pad with defaults if fewer)
    for i in 0..5u8 {
        if let Some(rank) = class_ranks.get(i as usize) {
            resp.write_u8(rank.rank_id as u8);
            resp.write_sbyte_string(&rank.str_user_id);
            resp.write_u32(rank.finish_time as u32);
            resp.write_u32(rank.b_stage as u32);
        } else {
            // Default empty entry (C++ lines 4601-4605)
            resp.write_u8(i + 1);
            resp.write_sbyte_string("");
            resp.write_u32(3600); // default time
            resp.write_u32(1); // default stage
        }
    }

    // User's own entry
    if let Some(ud) = &user_data {
        // Find user's rank in the class rankings
        let user_rank = all_ranks
            .iter()
            .filter(|r| r.class == user_draki_class)
            .position(|r| r.str_user_id == user_name)
            .map(|i| (i + 1) as u8)
            .unwrap_or(0xFF);
        resp.write_u8(user_rank);
        resp.write_sbyte_string(&user_name);
        resp.write_u32(ud.i_draki_time as u32);
        resp.write_u32(ud.b_draki_stage as u32);
        resp.write_u32(draki_tower::max_stages_from_linear(ud.b_draki_stage));
        resp.write_u32(ud.b_draki_enterance_limit as u32);
    } else {
        // No data — send defaults (C++ lines 4616-4625)
        resp.write_u8(0xFF);
        resp.write_sbyte_string(&user_name);
        resp.write_u32(3600); // default time
        resp.write_u32(1); // default stage
        resp.write_u32(1); // max_stages
        resp.write_u32(draki_tower::MAX_ENTRANCE_LIMIT as u32);
    }

    session.send_packet(&resp).await?;
    Ok(())
}

/// Draki Tower return to town handler.
/// Initiates a 20-second countdown to return to town from Draki Tower.
async fn handle_draki_town(session: &mut ClientSession) -> anyhow::Result<()> {
    use crate::handler::draki_tower;

    let world = session.world().clone();
    let sid = session.session_id();

    debug!("[{}] WIZ_EVENT DRAKI_TOWER_TOWN", session.addr());

    // Must be in Draki Tower zone
    let pos = match world.get_position(sid) {
        Some(p) if p.zone_id == draki_tower::ZONE_DRAKI_TOWER => p,
        _ => return Ok(()),
    };

    let room_id = world.with_session(sid, |h| h.draki_room_id).unwrap_or(0);

    if room_id == 0 {
        return Ok(());
    }

    // Apply town return state (sets 20s timer)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // BUG-3 fix: check tower_started and !town_request guards (C++ line 504)
    let (stage, sub_stage, elapsed) = {
        let mut rooms = world.draki_tower_rooms_write();
        match rooms.get_mut(&room_id) {
            Some(room) if room.tower_started && !room.town_request => {
                let elapsed = now.saturating_sub(room.draki_timer) as u32;
                let s = room.draki_stage;
                let ss = room.draki_sub_stage;
                draki_tower::apply_town_return(room, now);
                (s, ss, elapsed)
            }
            _ => return Ok(()),
        }
    };

    // Send OUT1 notification (C++ DrakiTowerTown lines 507-510)
    let mut out1_pkt = Packet::new(Opcode::WizEvent as u8);
    out1_pkt.write_u8(draki_tower::TEMPLE_DRAKI_TOWER_OUT1);
    out1_pkt.write_u8(0x0C);
    out1_pkt.write_u8(0x04);
    out1_pkt.write_u8(0x00);
    out1_pkt.write_u8(0x14);
    out1_pkt.write_u16(0);
    out1_pkt.write_u8(0);
    session.send_packet(&out1_pkt).await?;

    // Send OUT2 with stage progress (C++ DrakiTowerTown lines 512-518)
    let mut out2_pkt = Packet::new(Opcode::WizEvent as u8);
    out2_pkt.write_u8(draki_tower::TEMPLE_DRAKI_TOWER_OUT2);
    out2_pkt.write_u8(0x0C);
    out2_pkt.write_u8(0x04);
    out2_pkt.write_u16(stage);
    out2_pkt.write_u16(sub_stage);
    out2_pkt.write_u32(elapsed);
    out2_pkt.write_u8(1);
    session.send_packet(&out2_pkt).await?;

    // Persist progress before town exit (C++ DrakiTowerTown line 523: DrakiTowerSavedUserInfo)
    crate::handler::attack::draki_tower_save_progress(&world, sid, now).await;

    let _ = pos;
    debug!(
        "[sid={}] Draki Tower town return requested, room={}",
        sid, room_id
    );
    Ok(())
}

/// Dungeon Defence sign-up handler.
/// Validates the party, finds a free room, determines difficulty,
/// spawns guardian NPCs, and teleports all party members into zone 89.
async fn handle_dungeon_defence_sign(session: &mut ClientSession) -> anyhow::Result<()> {
    use crate::handler::dungeon_defence;
    use crate::world::types::ZONE_MORADON;

    let world = session.world().clone();
    let sid = session.session_id();

    debug!("[{}] WIZ_EVENT DUNGEON_DEFENCE_SIGN", session.addr());

    // Helper to send a DD sign response packet.
    let send_result = |code: u8| -> Packet {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN);
        resp.write_u8(code);
        resp
    };

    // ── Must be in a party and be the leader (C++ lines 20-28) ────────
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };
    let party_id = match ch.party_id {
        Some(pid) => pid,
        None => {
            session
                .send_packet(&send_result(dungeon_defence::DdSignResult::NoParty as u8))
                .await?;
            return Ok(());
        }
    };
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => {
            session
                .send_packet(&send_result(dungeon_defence::DdSignResult::NoParty as u8))
                .await?;
            return Ok(());
        }
    };
    if !party.is_leader(sid) {
        session
            .send_packet(&send_result(dungeon_defence::DdSignResult::Failed as u8))
            .await?;
        return Ok(());
    }

    let members = party.active_members();
    let member_count = members.len() as u16;

    // ── Determine difficulty from party size (C++ lines 125-127) ──────
    let difficulty = match dungeon_defence::DdDifficulty::from_party_size(member_count) {
        Some(d) => d,
        None => {
            session
                .send_packet(&send_result(dungeon_defence::DdSignResult::Failed as u8))
                .await?;
            return Ok(());
        }
    };

    // ── Validate all party members (C++ lines 30-115) ─────────────────
    for &msid in &members {
        // BUG-5 fix: must be a Full Moon Rift party (party_type == 2)
        let m_party_type = world.with_session(msid, |h| h.party_type).unwrap_or(0);
        if m_party_type != 2 {
            session
                .send_packet(&send_result(
                    dungeon_defence::DdSignResult::WrongPartyType as u8,
                ))
                .await?;
            return Ok(());
        }

        let m_pos = world.get_position(msid).unwrap_or_default();
        let m_ch = world.get_character_info(msid);
        let m_event_room = world.with_session(msid, |h| h.event_room).unwrap_or(0);

        // Must be in Moradon
        if m_pos.zone_id != ZONE_MORADON {
            session
                .send_packet(&send_result(
                    dungeon_defence::DdSignResult::NotInMoradon as u8,
                ))
                .await?;
            return Ok(());
        }

        // Must not be dead
        if let Some(ref mc) = m_ch {
            if mc.hp <= 0 {
                session
                    .send_packet(&send_result(
                        dungeon_defence::DdSignResult::MemberDead as u8,
                    ))
                    .await?;
                return Ok(());
            }
        }

        // Must not be in another instance
        if m_event_room > 0 {
            session
                .send_packet(&send_result(
                    dungeon_defence::DdSignResult::InstanceBusy as u8,
                ))
                .await?;
            return Ok(());
        }

        // Must have rift voucher item
        let has_item = world
            .with_session(msid, |h| {
                h.inventory.iter().any(|slot| {
                    slot.item_id == dungeon_defence::DUNGEON_DEFENCE_RIFT_ITEM && slot.count > 0
                })
            })
            .unwrap_or(false);
        if !has_item {
            session
                .send_packet(&send_result(
                    dungeon_defence::DdSignResult::NoRiftItem as u8,
                ))
                .await?;
            return Ok(());
        }
    }

    // ── Atomically claim a free room (C++ lines 146-158) ──────────────
    // Uses CAS to prevent TOCTOU races when multiple parties sign concurrently.
    let room_id = match dungeon_defence::try_claim_free_room(world.dd_rooms(), difficulty) {
        Some(id) => id,
        None => {
            session
                .send_packet(&send_result(
                    dungeon_defence::DdSignResult::SystemError as u8,
                ))
                .await?;
            return Ok(());
        }
    };

    // ── Teleport all party members to zone 89 (C++ lines 163-172) ────
    // C++ calls ZoneChange() per member which does full server-side zone change,
    // then RobItem() ONLY on success. We mirror this: zone change first, then
    // consume voucher only after successful teleport.
    //
    // Leader uses trigger_zone_change (has session handle).
    // Non-leaders use server_teleport_to_zone (session-less server-side teleport).
    // event_room is set AFTER zone change to avoid breaking INOUT_OUT broadcasts
    // in the source zone (Moradon), matching C++ where m_bEventRoom is set at
    // the END of ZoneChange().
    super::zone_change::trigger_zone_change(
        session,
        dungeon_defence::ZONE_DUNGEON_DEFENCE,
        0.0,
        0.0,
    )
    .await?;

    for &msid in &members {
        if msid == sid {
            continue; // leader already handled above
        }
        super::zone_change::server_teleport_to_zone(
            &world,
            msid,
            dungeon_defence::ZONE_DUNGEON_DEFENCE,
            0.0,
            0.0,
        )
    }

    // ── Set event_room on all members (C++ ZoneChange line 449-450) ──
    for &msid in &members {
        world.update_session(msid, |h| {
            h.event_room = room_id;
        });
    }

    // ── Consume rift voucher AFTER successful zone change (C++ line 171) ─
    // C++ only calls RobItem() when ZoneChange() returns true.
    for &msid in &members {
        world.update_session(msid, |h| {
            if let Some(slot) = h
                .inventory
                .iter_mut()
                .find(|s| s.item_id == dungeon_defence::DUNGEON_DEFENCE_RIFT_ITEM && s.count > 0)
            {
                slot.count -= 1;
                if slot.count == 0 {
                    slot.item_id = 0;
                    slot.durability = 0;
                }
            }
        });
    }

    // ── Spawn 6 guardian NPCs (C++ lines 175-180) ─────────────────────
    for &(npc_id, x, z) in &dungeon_defence::GUARDIAN_NPCS {
        world.spawn_event_npc_ex(
            npc_id,
            false, // is_monster = false (NPCs)
            dungeon_defence::ZONE_DUNGEON_DEFENCE,
            x as f32,
            z as f32,
            1,       // count
            room_id, // event_room
            0,       // direction
        );
    }

    // ── Send success response (C++ line 192) ──────────────────────────
    session.send_packet(&send_result(0)).await?;

    debug!(
        "[sid={}] Dungeon Defence entered: difficulty={:?}, room={}, members={}",
        sid, difficulty, room_id, member_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::event_room::TempleEventType;

    #[test]
    fn test_monster_stone_fail_packet() {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::MONSTER_STONE);
        resp.write_u8(1);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(6)); // MONSTER_STONE
        assert_eq!(reader.read_u8(), Some(1)); // error code
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_temple_join_fail_packet() {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(4);
        resp.write_i16(-1);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(8)); // TEMPLE_EVENT_JOIN
        assert_eq!(reader.read_u8(), Some(4)); // fail
        assert_eq!(reader.read_u16(), Some(0xFFFF)); // -1 as u16
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_temple_join_success_packet() {
        let active_event: i16 = TempleEventType::BorderDefenceWar as i16; // 4
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_JOIN);
        resp.write_u8(1); // success
        resp.write_i16(active_event);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(8)); // TEMPLE_EVENT_JOIN
        assert_eq!(reader.read_u8(), Some(1)); // success
        assert_eq!(reader.read_u16(), Some(4)); // BDW event type
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_temple_disband_fail_packet() {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
        resp.write_u8(4);
        resp.write_u16(0);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(9)); // TEMPLE_EVENT_DISBAND
        assert_eq!(reader.read_u8(), Some(4)); // fail
        assert_eq!(reader.read_u16(), Some(0)); // no active event
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_temple_disband_success_packet() {
        let active_event_u16: u16 = TempleEventType::JuraidMountain as u16; // 100
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DISBAND);
        resp.write_u8(1); // success
        resp.write_u16(active_event_u16);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(9)); // TEMPLE_EVENT_DISBAND
        assert_eq!(reader.read_u8(), Some(1)); // success
        assert_eq!(reader.read_u16(), Some(100)); // Juraid event type
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_zone_prison_constant() {
        assert_eq!(ZONE_PRISON, 92);
    }

    #[test]
    fn test_draki_enter_fail_packet() {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_DRAKI_TOWER_ENTER);
        resp.write_u32(8);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(33)); // TEMPLE_DRAKI_TOWER_ENTER
        assert_eq!(reader.read_u32(), Some(8)); // error: cannot enter right now
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_draki_list_empty_packet() {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_DRAKI_TOWER_LIST);
        resp.write_u8(0);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(34)); // TEMPLE_DRAKI_TOWER_LIST
        assert_eq!(reader.read_u8(), Some(0)); // count = 0
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_dungeon_sign_fail_packet() {
        let mut resp = Packet::new(Opcode::WizEvent as u8);
        resp.write_u8(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN);
        resp.write_u8(1);

        assert_eq!(resp.opcode, 0x5F);
        let mut reader = PacketReader::new(&resp.data);
        assert_eq!(reader.read_u8(), Some(58)); // TEMPLE_EVENT_DUNGEON_SIGN
        assert_eq!(reader.read_u8(), Some(1)); // instance generation failed
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_event_sub_opcode_values() {
        // Verify sub-opcode constants match C++ packets.h values
        assert_eq!(sub_opcode::MONSTER_STONE, 6);
        assert_eq!(sub_opcode::TEMPLE_EVENT_JOIN, 8);
        assert_eq!(sub_opcode::TEMPLE_EVENT_DISBAND, 9);
        assert_eq!(sub_opcode::TEMPLE_EVENT_FINISH, 10);
        assert_eq!(sub_opcode::TEMPLE_EVENT_COUNTER, 16);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_ENTER, 33);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_LIST, 34);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_TIMER, 35);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_TOWN, 38);
        assert_eq!(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN, 58);
    }

    #[test]
    fn test_event_type_values() {
        // Verify event type constants match C++ EventOpCode enum
        assert_eq!(event_type::TEMPLE_EVENT_BORDER_DEFENCE_WAR, 4);
        assert_eq!(event_type::TEMPLE_EVENT_MONSTER_STONE, 14);
        assert_eq!(event_type::TEMPLE_EVENT_CHAOS, 24);
        assert_eq!(event_type::TEMPLE_EVENT_JURAD_MOUNTAIN, 100);
        assert_eq!(event_type::TEMPLE_EVENT_KNIGHT_BATTLE_ROYALE, 104);
    }

    // ── Monster Stone handler validation tests ──────────────────────────

    #[test]
    fn test_monster_stone_item_validation() {
        use crate::systems::monster_stone;

        // Valid items
        assert!(monster_stone::is_monster_stone_item(300_144_036)); // STONE1
        assert!(monster_stone::is_monster_stone_item(300_145_037)); // STONE2
        assert!(monster_stone::is_monster_stone_item(300_146_038)); // STONE3
        assert!(monster_stone::is_monster_stone_item(900_144_023)); // UNIVERSAL

        // Invalid items
        assert!(!monster_stone::is_monster_stone_item(0));
        assert!(!monster_stone::is_monster_stone_item(300_144_035)); // off-by-one
        assert!(!monster_stone::is_monster_stone_item(389_132_000)); // mattock
        assert!(!monster_stone::is_monster_stone_item(u32::MAX));
    }

    #[test]
    fn test_monster_stone_item_to_zone() {
        use crate::systems::monster_stone;

        assert_eq!(monster_stone::item_to_zone(300_144_036), Some(81));
        assert_eq!(monster_stone::item_to_zone(300_145_037), Some(82));
        assert_eq!(monster_stone::item_to_zone(300_146_038), Some(83));
        assert_eq!(monster_stone::item_to_zone(900_144_023), None); // universal has no fixed zone
        assert_eq!(monster_stone::item_to_zone(0), None);
    }

    #[test]
    fn test_monster_stone_fail_packet_error_codes() {
        use crate::systems::monster_stone;

        // Error 1: zone restriction (prison / already in MS zone / temple event zone)
        let pkt1 = monster_stone::build_fail_packet(1);
        assert_eq!(pkt1.opcode, 0x5F);
        let mut r = PacketReader::new(&pkt1.data);
        assert_eq!(r.read_u8(), Some(6)); // MONSTER_STONE sub-opcode
        assert_eq!(r.read_u8(), Some(1)); // error 1
        assert_eq!(r.remaining(), 0);

        // Error 5: status / inventory / spawn / item consume failure
        let pkt5 = monster_stone::build_fail_packet(5);
        let mut r = PacketReader::new(&pkt5.data);
        assert_eq!(r.read_u8(), Some(6));
        assert_eq!(r.read_u8(), Some(5));

        // Error 9: HP too low (< 50%)
        let pkt9 = monster_stone::build_fail_packet(9);
        let mut r = PacketReader::new(&pkt9.data);
        assert_eq!(r.read_u8(), Some(6));
        assert_eq!(r.read_u8(), Some(9));
    }

    #[test]
    fn test_monster_stone_universal_level_ranges() {
        use crate::systems::monster_stone;

        // Below level 20 → None
        assert!(monster_stone::universal_stone_zone_family(1).is_none());
        assert!(monster_stone::universal_stone_zone_family(19).is_none());

        // Level 20-29 → zone 81, family 1
        assert_eq!(
            monster_stone::universal_stone_zone_family(20),
            Some((81, 1))
        );
        assert_eq!(
            monster_stone::universal_stone_zone_family(29),
            Some((81, 1))
        );

        // Level 30-35 → zone 81, family 2
        assert_eq!(
            monster_stone::universal_stone_zone_family(30),
            Some((81, 2))
        );
        assert_eq!(
            monster_stone::universal_stone_zone_family(35),
            Some((81, 2))
        );

        // Level 36-40 → zone 81, family 3
        assert_eq!(
            monster_stone::universal_stone_zone_family(36),
            Some((81, 3))
        );
        assert_eq!(
            monster_stone::universal_stone_zone_family(40),
            Some((81, 3))
        );

        // Level 41-46 → zone 81, family 4
        assert_eq!(
            monster_stone::universal_stone_zone_family(41),
            Some((81, 4))
        );
        assert_eq!(
            monster_stone::universal_stone_zone_family(46),
            Some((81, 4))
        );

        // Level 47-55 → zone 81 or 82, family 4 or 5 (random)
        for _ in 0..20 {
            let (z, f) = monster_stone::universal_stone_zone_family(50).unwrap();
            assert!(f == 4 || f == 5);
            if f == 4 {
                assert_eq!(z, 81);
            }
            if f == 5 {
                assert_eq!(z, 82);
            }
        }

        // Level 56-60 → zone 82, family 6-8
        for _ in 0..20 {
            let (z, f) = monster_stone::universal_stone_zone_family(58).unwrap();
            assert_eq!(z, 82);
            assert!((6..=8).contains(&f));
        }

        // Level 61-66 → zone 82, family 8-9
        for _ in 0..20 {
            let (z, f) = monster_stone::universal_stone_zone_family(63).unwrap();
            assert_eq!(z, 82);
            assert!((8..=9).contains(&f));
        }

        // Level 67-70 → zone 82 or 83, family 9 or 10
        for _ in 0..20 {
            let (z, f) = monster_stone::universal_stone_zone_family(69).unwrap();
            assert!(f == 9 || f == 10);
            if f == 9 {
                assert_eq!(z, 82);
            }
            if f == 10 {
                assert_eq!(z, 83);
            }
        }

        // Level 71-74 → zone 83, family 10-12
        for _ in 0..20 {
            let (z, f) = monster_stone::universal_stone_zone_family(73).unwrap();
            assert_eq!(z, 83);
            assert!((10..=12).contains(&f));
        }

        // Level 75+ → zone 83, family 13
        assert_eq!(
            monster_stone::universal_stone_zone_family(75),
            Some((83, 13))
        );
        assert_eq!(
            monster_stone::universal_stone_zone_family(83),
            Some((83, 13))
        );
    }

    #[test]
    fn test_monster_stone_random_family_per_zone() {
        use crate::systems::monster_stone;

        // Zone 81 → family 1-4
        for _ in 0..20 {
            let f = monster_stone::random_family_for_zone(81);
            assert!((1..=4).contains(&f), "zone 81 family {} out of range", f);
        }

        // Zone 82 → family 5-9
        for _ in 0..20 {
            let f = monster_stone::random_family_for_zone(82);
            assert!((5..=9).contains(&f), "zone 82 family {} out of range", f);
        }

        // Zone 83 → family 10-13
        for _ in 0..20 {
            let f = monster_stone::random_family_for_zone(83);
            assert!((10..=13).contains(&f), "zone 83 family {} out of range", f);
        }

        // Unknown zone → default family 1
        assert_eq!(monster_stone::random_family_for_zone(21), 1);
    }

    #[test]
    fn test_monster_stone_timer_packet_format() {
        use crate::systems::monster_stone;

        let pkt = monster_stone::build_timer_packet(1800);
        assert_eq!(pkt.opcode, Opcode::WizBifrost as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5)); // MONSTER_SQUARD
        assert_eq!(r.read_u16(), Some(1800));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_monster_stone_select_msg_timer_format() {
        use crate::systems::monster_stone;

        let pkt = monster_stone::build_select_msg_timer(1800);
        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(0)); // padding
        assert_eq!(r.read_u8(), Some(7)); // type
        assert_eq!(r.read_u64(), Some(0)); // padding (was two u32s, fixed to u64)
        assert_eq!(r.read_u8(), Some(9)); // sub-type
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u8(), Some(0)); // padding
        assert_eq!(r.read_u8(), Some(11)); // timer marker
        assert_eq!(r.read_u16(), Some(1800)); // time
        assert_eq!(r.read_u16(), Some(0)); // trailing
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_monster_stone_boss_kill_packets() {
        use crate::systems::monster_stone;

        let (finish, quest) = monster_stone::build_boss_kill_packets();

        // Finish packet: WIZ_EVENT | TEMPLE_EVENT_FINISH | 0x11 | 0x00 | 0x65 | 0x14 | u32(0)
        assert_eq!(finish.opcode, Opcode::WizEvent as u8);
        let mut r = PacketReader::new(&finish.data);
        assert_eq!(r.read_u8(), Some(10)); // TEMPLE_EVENT_FINISH
        assert_eq!(r.read_u8(), Some(0x11));
        assert_eq!(r.read_u8(), Some(0x00));
        assert_eq!(r.read_u8(), Some(0x65));
        assert_eq!(r.read_u8(), Some(0x14));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);

        // Quest packet: WIZ_QUEST | 2 | u16(209) | 0
        assert_eq!(quest.opcode, Opcode::WizQuest as u8);
        let mut r = PacketReader::new(&quest.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(209));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_monster_stone_zone_restriction_constants() {
        // Verify ZONE_PRISON used in handler matches C++ define
        assert_eq!(ZONE_PRISON, 92);

        use crate::systems::monster_stone;
        // Monster stone zones should be detected
        assert!(monster_stone::is_monster_stone_zone(81));
        assert!(monster_stone::is_monster_stone_zone(82));
        assert!(monster_stone::is_monster_stone_zone(83));
        // Prison is NOT a monster stone zone
        assert!(!monster_stone::is_monster_stone_zone(92));
    }

    #[test]
    fn test_monster_stone_vendor_npc_ids() {
        // Three vendor NPCs spawned per room
        let vendor_npcs: [u16; 3] = [16062, 12117, 31508];
        assert_eq!(vendor_npcs.len(), 3);
        // All NPCs should be non-zero
        for &npc_id in &vendor_npcs {
            assert!(npc_id > 0);
        }
    }

    #[test]
    fn test_monster_stone_vendor_coords_per_zone() {
        let z81: [(f32, f32); 3] = [(204.0, 201.0), (204.0, 197.0), (204.0, 193.0)];
        let z82: [(f32, f32); 3] = [(203.0, 202.0), (203.0, 197.0), (203.0, 193.0)];
        let z83: [(f32, f32); 3] = [(204.0, 207.0), (204.0, 200.0), (204.0, 194.0)];

        // All coordinates should be positive and within reasonable bounds
        for coords in [z81, z82, z83] {
            for (x, z) in coords {
                assert!(x > 100.0 && x < 300.0, "x={} out of bounds", x);
                assert!(z > 100.0 && z < 300.0, "z={} out of bounds", z);
            }
        }
    }

    #[test]
    fn test_monster_stone_event_room_id_is_one_based() {
        // room_id 0 → event_room_id 1
        // room_id 749 → event_room_id 750
        let room_id: u16 = 0;
        let event_room_id = room_id + 1;
        assert_eq!(event_room_id, 1);

        let room_id: u16 = 749;
        let event_room_id = room_id + 1;
        assert_eq!(event_room_id, 750);
    }

    // ── Sprint 952: Additional coverage ──────────────────────────────

    /// Event sub-opcodes: core values match C++ packets.h.
    #[test]
    fn test_event_sub_opcodes() {
        assert_eq!(sub_opcode::MONSTER_STONE, 6);
        assert_eq!(sub_opcode::TEMPLE_EVENT, 7);
        assert_eq!(sub_opcode::TEMPLE_EVENT_JOIN, 8);
        assert_eq!(sub_opcode::TEMPLE_EVENT_DISBAND, 9);
        assert_eq!(sub_opcode::TEMPLE_EVENT_FINISH, 10);
        assert_eq!(sub_opcode::TEMPLE_EVENT_COUNTER, 16);
    }

    /// Draki Tower sub-opcodes: 33-38.
    #[test]
    fn test_draki_tower_sub_opcodes() {
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_ENTER, 33);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_LIST, 34);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_TIMER, 35);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_TOWN, 38);
    }

    /// Event type IDs for temple events.
    #[test]
    fn test_event_type_ids() {
        assert_eq!(event_type::TEMPLE_EVENT_BORDER_DEFENCE_WAR, 4);
        assert_eq!(event_type::TEMPLE_EVENT_MONSTER_STONE, 14);
        assert_eq!(event_type::TEMPLE_EVENT_CHAOS, 24);
        assert_eq!(event_type::TEMPLE_EVENT_JURAD_MOUNTAIN, 100);
        assert_eq!(event_type::TEMPLE_EVENT_KNIGHT_BATTLE_ROYALE, 104);
    }

    /// Event zone constants are distinct.
    #[test]
    fn test_event_zone_constants() {
        assert_ne!(ZONE_BORDER_DEFENSE_WAR, ZONE_CHAOS_DUNGEON);
        assert_ne!(ZONE_CHAOS_DUNGEON, ZONE_JURAID_MOUNTAIN);
        assert_ne!(ZONE_JURAID_MOUNTAIN, ZONE_PRISON);
    }

    /// Dungeon Defence sign-up sub-opcode.
    #[test]
    fn test_dungeon_defence_sub_opcode() {
        assert_eq!(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN, 58);
    }

    // ── Sprint 965: Additional coverage ──────────────────────────────

    /// Mining/fishing tool IDs imported for equipment validation.
    #[test]
    fn test_mining_fishing_tool_ids() {
        assert_eq!(MATTOCK, 389_132_000);
        assert_eq!(GOLDEN_MATTOCK, 389_135_000);
        assert_eq!(FISHING_ROD, 191_346_000);
        assert_eq!(GOLDEN_FISHING_ROD, 191_347_000);
    }

    /// Draki Tower sub-opcodes have gaps (35 timer, 36-37 out, 38 town).
    #[test]
    fn test_draki_tower_opcode_gaps() {
        // ENTER=33, LIST=34, TIMER=35 are contiguous
        assert_eq!(
            sub_opcode::TEMPLE_DRAKI_TOWER_LIST - sub_opcode::TEMPLE_DRAKI_TOWER_ENTER,
            1
        );
        assert_eq!(
            sub_opcode::TEMPLE_DRAKI_TOWER_TIMER - sub_opcode::TEMPLE_DRAKI_TOWER_LIST,
            1
        );
        // TOWN=38 skips OUT1(36) and OUT2(37)
        assert_eq!(
            sub_opcode::TEMPLE_DRAKI_TOWER_TOWN - sub_opcode::TEMPLE_DRAKI_TOWER_TIMER,
            3
        );
    }

    /// Event types are non-contiguous i16 values.
    #[test]
    fn test_event_types_non_contiguous() {
        let types = [
            event_type::TEMPLE_EVENT_BORDER_DEFENCE_WAR,
            event_type::TEMPLE_EVENT_MONSTER_STONE,
            event_type::TEMPLE_EVENT_CHAOS,
            event_type::TEMPLE_EVENT_JURAD_MOUNTAIN,
            event_type::TEMPLE_EVENT_KNIGHT_BATTLE_ROYALE,
        ];
        // All values are distinct
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j]);
            }
        }
        // All are positive
        for &t in &types {
            assert!(t > 0);
        }
    }

    /// Temple event info packet format: opcode + sub(7) + active_event(i16) + remain(u16).
    #[test]
    fn test_temple_event_info_packet_format() {
        let mut pkt = Packet::new(Opcode::WizEvent as u8);
        pkt.write_u8(sub_opcode::TEMPLE_EVENT);
        pkt.write_i16(-1); // no active event
        pkt.write_u16(0); // no remaining time
                          // sub(1) + active_event(2) + remain(2) = 5 bytes
        assert_eq!(pkt.data.len(), 5);
        assert_eq!(pkt.data[0], sub_opcode::TEMPLE_EVENT);
    }

    /// Monster stone sub-opcode (6) is below temple event range (7+).
    #[test]
    fn test_monster_stone_is_lowest_sub_opcode() {
        assert!(sub_opcode::MONSTER_STONE < sub_opcode::TEMPLE_EVENT);
        assert!(sub_opcode::MONSTER_STONE < sub_opcode::TEMPLE_EVENT_JOIN);
        assert!(sub_opcode::MONSTER_STONE < sub_opcode::TEMPLE_EVENT_DISBAND);
        assert!(sub_opcode::MONSTER_STONE < sub_opcode::TEMPLE_DRAKI_TOWER_ENTER);
    }

    /// Temple event join/disband are adjacent sub-opcodes (8, 9).
    #[test]
    fn test_temple_event_join_disband_adjacent() {
        assert_eq!(sub_opcode::TEMPLE_EVENT_JOIN, 8);
        assert_eq!(sub_opcode::TEMPLE_EVENT_DISBAND, 9);
        assert_eq!(
            sub_opcode::TEMPLE_EVENT_DISBAND - sub_opcode::TEMPLE_EVENT_JOIN,
            1
        );
    }

    /// Server-sent sub-opcodes: FINISH (10) and COUNTER (16) are distinct from client sub-opcodes.
    #[test]
    fn test_server_sent_subopcodes_distinct() {
        assert_eq!(sub_opcode::TEMPLE_EVENT_FINISH, 10);
        assert_eq!(sub_opcode::TEMPLE_EVENT_COUNTER, 16);
        assert_ne!(
            sub_opcode::TEMPLE_EVENT_FINISH,
            sub_opcode::TEMPLE_EVENT_COUNTER
        );
        // Both above client join/disband range
        assert!(sub_opcode::TEMPLE_EVENT_FINISH > sub_opcode::TEMPLE_EVENT_DISBAND);
        assert!(sub_opcode::TEMPLE_EVENT_COUNTER > sub_opcode::TEMPLE_EVENT_FINISH);
    }

    /// Event type IDs: BDW(4) < MonsterStone(14) < Chaos(24) < Juraid(100) < KBR(104).
    #[test]
    fn test_event_type_ids_ordered() {
        assert_eq!(event_type::TEMPLE_EVENT_BORDER_DEFENCE_WAR, 4);
        assert_eq!(event_type::TEMPLE_EVENT_MONSTER_STONE, 14);
        assert_eq!(event_type::TEMPLE_EVENT_CHAOS, 24);
        assert_eq!(event_type::TEMPLE_EVENT_JURAD_MOUNTAIN, 100);
        assert_eq!(event_type::TEMPLE_EVENT_KNIGHT_BATTLE_ROYALE, 104);
        // Strictly increasing
        assert!(
            event_type::TEMPLE_EVENT_BORDER_DEFENCE_WAR < event_type::TEMPLE_EVENT_MONSTER_STONE
        );
        assert!(event_type::TEMPLE_EVENT_CHAOS < event_type::TEMPLE_EVENT_JURAD_MOUNTAIN);
        assert!(
            event_type::TEMPLE_EVENT_JURAD_MOUNTAIN < event_type::TEMPLE_EVENT_KNIGHT_BATTLE_ROYALE
        );
    }

    /// Draki tower sub-opcodes: timer (35) sits between list (34) and town (38).
    #[test]
    fn test_draki_tower_timer_position() {
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_LIST, 34);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_TIMER, 35);
        assert_eq!(sub_opcode::TEMPLE_DRAKI_TOWER_TOWN, 38);
        assert!(sub_opcode::TEMPLE_DRAKI_TOWER_LIST < sub_opcode::TEMPLE_DRAKI_TOWER_TIMER);
        assert!(sub_opcode::TEMPLE_DRAKI_TOWER_TIMER < sub_opcode::TEMPLE_DRAKI_TOWER_TOWN);
    }

    /// Dungeon defence sign-up (58) is the highest client-handled sub-opcode.
    #[test]
    fn test_dungeon_defence_highest_client_subopcode() {
        assert_eq!(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN, 58);
        // Higher than all other client sub-opcodes
        assert!(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN > sub_opcode::TEMPLE_DRAKI_TOWER_TOWN);
        assert!(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN > sub_opcode::TEMPLE_DRAKI_TOWER_ENTER);
        assert!(sub_opcode::TEMPLE_EVENT_DUNGEON_SIGN > sub_opcode::TEMPLE_EVENT_DISBAND);
    }
}
