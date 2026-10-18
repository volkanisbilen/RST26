//! WIZ_MOVING_TOWER (0x84) handler -- siege tower boarding and dismounting.
//! ## Sub-commands
//! | Cmd | Direction     | Description                                      |
//! |-----|---------------|--------------------------------------------------|
//! | 1   | Client→Server | Board siege tower (ZONE_DELOS, siege war active)  |
//! | 2   | Client→Server | Dismount siege tower (warp to position)           |
//! | 16  | Client→Server | Mount NPC tower (ZONE_BATTLE6, war open)          |
//! | 17  | Client→Server | Dismount NPC tower                                |
//! ## Wire format
//! **Client→Server (cmd 1):** `[u8 command=1]`
//! **Client→Server (cmd 2):** `[u8 command=2] [u16 pos_x] [u16 pos_z]`
//! **Client→Server (cmd 16):** `[u8 command=16] [u16 npc_id]`
//! **Client→Server (cmd 17):** `[u8 command=17]`
//! **Server→Client (cmd 1):** `[u8 command=1] [u8 success=1]`
//! **Server→Client (cmd 2):** `[u8 command=2] [u8 success=1]`
//! **Server→Client (cmd 16):** `[u8 command=16] [u8 success=1] [u32 user_id] [u32 npc_id] [u16 sx] [u16 sz] [u16 sy]`
//! **Server→Client (cmd 17):** `[u8 command=17] [u8 success=1] [u32 user_id]`

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::handler::region;
use crate::session::{ClientSession, SessionState};
use crate::world::{ZONE_BATTLE6, ZONE_DELOS};

use crate::state_change_constants::{STATE_CHANGE_ABNORMAL, STATE_CHANGE_TEAM_COLOUR};

/// Boarding transformation value (player becomes siege tower).
const ABNORMAL_BOARDING: u32 = 8;

use crate::magic_constants::ABNORMAL_NORMAL;

/// NPC tower type (type 191 in npc_template.npc_type).
const NPC_TYPE_TOWER: u8 = 191;

/// Maximum squared distance for NPC tower interaction.
const MAX_NPC_RANGE_SQUARED: f32 = 121.0;

/// NPC transformation value for mounting tower (450018 from C++).
const TOWER_TRANSFORM_ID: u32 = 450018;

/// Sub-command: board a siege tower (player transformation) in ZONE_DELOS.
const CMD_BOARD_SIEGE: u8 = 1;
/// Sub-command: dismount a siege tower and warp to a position.
const CMD_DISMOUNT_SIEGE: u8 = 2;
/// Sub-command: mount an NPC tower in ZONE_BATTLE6.
const CMD_MOUNT_NPC_TOWER: u8 = 16;
/// Sub-command: dismount an NPC tower.
const CMD_DISMOUNT_NPC_TOWER: u8 = 17;

/// Handle incoming WIZ_MOVING_TOWER (0x84) packet.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot operate towers/siege equipment
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let command = match reader.read_u8() {
        Some(c) => c,
        None => return Ok(()),
    };

    match command {
        CMD_BOARD_SIEGE => handle_board_siege(session).await,
        CMD_DISMOUNT_SIEGE => {
            let pos_x = reader.read_u16().unwrap_or(0);
            let pos_z = reader.read_u16().unwrap_or(0);
            handle_dismount_siege(session, pos_x, pos_z).await
        }
        CMD_MOUNT_NPC_TOWER => {
            let npc_id = reader.read_u16().unwrap_or(0);
            handle_mount_npc_tower(session, npc_id).await
        }
        CMD_DISMOUNT_NPC_TOWER => handle_dismount_npc_tower(session).await,
        _ => {
            warn!(
                "[{}] WIZ_MOVING_TOWER: unhandled sub-command {}",
                session.addr(),
                command
            );
            Ok(())
        }
    }
}

/// Board a siege tower (cmd=1).
/// Validates: in clan, zone == ZONE_DELOS, siege war open.
async fn handle_board_siege(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        debug!("[{}] WIZ_MOVING_TOWER cmd=1: not in clan", session.addr());
        return Ok(());
    }

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    if pos.zone_id != ZONE_DELOS {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=1: wrong zone {} (need ZONE_DELOS={})",
            session.addr(),
            pos.zone_id,
            ZONE_DELOS
        );
        return Ok(());
    }

    // Check siege war is active
    let csw = world.csw_event().read().await;
    if !csw.is_war_active() {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=1: siege war not active",
            session.addr()
        );
        return Ok(());
    }
    drop(csw);

    // Broadcast type-11 state change (team colour = BOARDING) to 3x3 region.
    let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
    state_pkt.write_u32(sid as u32);
    state_pkt.write_u8(STATE_CHANGE_TEAM_COLOUR);
    state_pkt.write_u32(ABNORMAL_BOARDING);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(state_pkt),
        None,
        event_room,
    );

    let out_pkt = region::build_user_inout(region::INOUT_OUT, sid, None, &pos);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(out_pkt),
        Some(sid),
        event_room,
    );

    session.send_packet(&build_board_siege_response()).await?;

    debug!(
        "[{}] WIZ_MOVING_TOWER cmd=1: boarded siege tower",
        session.addr()
    );
    Ok(())
}

/// Dismount a siege tower (cmd=2), warp to the given position.
/// Validates: in clan, zone == ZONE_DELOS, siege war open.
async fn handle_dismount_siege(
    session: &mut ClientSession,
    pos_x: u16,
    pos_z: u16,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        return Ok(());
    }

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    if pos.zone_id != ZONE_DELOS {
        return Ok(());
    }

    let csw = world.csw_event().read().await;
    if !csw.is_war_active() {
        return Ok(());
    }
    drop(csw);

    // Broadcast type-11 state change (restore normal appearance) to 3x3 region.
    let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
    state_pkt.write_u32(sid as u32);
    state_pkt.write_u8(STATE_CHANGE_TEAM_COLOUR);
    state_pkt.write_u32(ABNORMAL_NORMAL);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(state_pkt),
        None,
        event_room,
    );

    let warp_x = pos_x as f32 / 10.0;
    let warp_z = pos_z as f32 / 10.0;
    world.update_position(sid, pos.zone_id, warp_x, pos.y, warp_z);

    // Re-send INOUT_IN so the player reappears at the new position.
    let new_pos = world.get_position(sid).unwrap_or(pos);
    let new_ch = world.get_character_info(sid);
    let invis = world.get_invisibility_type(sid);
    let abnormal = world.get_abnormal_type(sid);
    let equip_vis = region::get_equipped_visual(&world, sid);
    let in_pkt = region::build_user_inout_with_invis(
        region::INOUT_IN,
        sid,
        new_ch.as_ref(),
        &new_pos,
        invis,
        abnormal,
        &equip_vis,
    );
    world.broadcast_to_3x3(
        new_pos.zone_id,
        new_pos.region_x,
        new_pos.region_z,
        Arc::new(in_pkt),
        Some(sid),
        event_room,
    );

    session
        .send_packet(&build_dismount_siege_response())
        .await?;

    debug!(
        "[{}] WIZ_MOVING_TOWER cmd=2: dismounted siege tower at ({}, {})",
        session.addr(),
        pos_x,
        pos_z
    );
    Ok(())
}

/// Mount an NPC tower (cmd=16) in ZONE_BATTLE6.
/// Validates: no existing tower, zone == ZONE_BATTLE6, war open, NPC type 191.
async fn handle_mount_npc_tower(session: &mut ClientSession, npc_id: u16) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    if pos.zone_id != ZONE_BATTLE6 {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=16: wrong zone {} (need ZONE_BATTLE6={})",
            session.addr(),
            pos.zone_id,
            ZONE_BATTLE6
        );
        return Ok(());
    }

    // isWarOpen() checks m_byBattleOpen >= NATION_BATTLE (nation war, not CSW).
    if !world.is_war_open() {
        debug!("[{}] WIZ_MOVING_TOWER cmd=16: war not open", session.addr());
        return Ok(());
    }

    if world.get_tower_owner_id(sid) != -1 {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=16: already mounted on tower",
            session.addr()
        );
        return Ok(());
    }

    // Client sends the full NPC runtime ID (already includes NPC_BAND). No addition needed.
    let npc_nid = npc_id as u32;
    let npc = match world.get_npc_instance(npc_nid) {
        Some(n) => n,
        None => {
            debug!(
                "[{}] WIZ_MOVING_TOWER cmd=16: NPC {} not found",
                session.addr(),
                npc_nid
            );
            return Ok(());
        }
    };

    // C++ GetNpcPtr returns null for dead NPCs — explicit check needed
    if world.is_npc_dead(npc_nid) {
        return Ok(());
    }

    // Verify NPC is in the same zone
    if npc.zone_id != pos.zone_id {
        return Ok(());
    }

    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => return Ok(()),
    };
    if tmpl.npc_type != NPC_TYPE_TOWER {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=16: NPC {} is type {} (need {})",
            session.addr(),
            npc_nid,
            tmpl.npc_type,
            NPC_TYPE_TOWER
        );
        return Ok(());
    }

    let is_owned = world
        .get_npc_ai(npc_nid)
        .map(|ai| ai.is_tower_owner)
        .unwrap_or(false);
    if is_owned {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=16: NPC {} already owned",
            session.addr(),
            npc_nid
        );
        return Ok(());
    }

    // C++ Unit::isInRange uses GetDistance() which returns dx²+dz² (no sqrt).
    let dx = pos.x - npc.x;
    let dz = pos.z - npc.z;
    let dist_sq = dx * dx + dz * dz;
    if dist_sq > MAX_NPC_RANGE_SQUARED {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=16: NPC {} out of range (dist²={:.0} > {:.0})",
            session.addr(),
            npc_nid,
            dist_sq,
            MAX_NPC_RANGE_SQUARED
        );
        return Ok(());
    }

    // StateChange(NPC_HIDE) calls SendInOut(INOUT_OUT) — broadcasts NPC disappear to region.
    let hide_pkt = crate::npc::build_npc_inout(crate::npc::NPC_OUT, &npc, &tmpl);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(hide_pkt),
        None,
        event_room,
    );

    // Mark NPC as owned: pNpc->m_isTowerOwner = true;
    world.update_npc_ai(npc_nid, |ai| {
        ai.is_tower_owner = true;
    });

    world.set_tower_owner_id(sid, npc_nid as i32);

    world.update_position(sid, pos.zone_id, npc.x, npc.y, npc.z);

    let new_pos = world.get_position(sid).unwrap_or(pos);
    let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
    state_pkt.write_u32(sid as u32);
    state_pkt.write_u8(STATE_CHANGE_ABNORMAL);
    state_pkt.write_u32(TOWER_TRANSFORM_ID);
    world.broadcast_to_3x3(
        new_pos.zone_id,
        new_pos.region_x,
        new_pos.region_z,
        Arc::new(state_pkt),
        None,
        event_room,
    );

    let spawn_x = (npc.x * 10.0) as u16;
    let spawn_z = (npc.z * 10.0) as u16;
    let spawn_y = (npc.y * 10.0) as u16;

    session
        .send_packet(&build_mount_npc_tower_response(
            sid as u32, npc_nid, spawn_x, spawn_z, spawn_y,
        ))
        .await?;

    debug!(
        "[{}] WIZ_MOVING_TOWER cmd=16: mounted NPC tower {} (spawn {},{},{})",
        session.addr(),
        npc_nid,
        spawn_x,
        spawn_z,
        spawn_y
    );
    Ok(())
}

/// Dismount an NPC tower (cmd=17).
/// Validates: has tower, zone == ZONE_BATTLE6, war open.
async fn handle_dismount_npc_tower(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    if pos.zone_id != ZONE_BATTLE6 {
        return Ok(());
    }

    if !world.is_war_open() {
        return Ok(());
    }

    let tower_npc_id = world.get_tower_owner_id(sid);
    if tower_npc_id == -1 {
        debug!(
            "[{}] WIZ_MOVING_TOWER cmd=17: not mounted on a tower",
            session.addr()
        );
        return Ok(());
    }

    let npc_nid = tower_npc_id as u32;

    // Validate the NPC still exists, is type 191, and is actually tower-owned.
    let npc = match world.get_npc_instance(npc_nid) {
        Some(n) => n,
        None => return Ok(()),
    };
    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) if t.npc_type == NPC_TYPE_TOWER => t,
        _ => return Ok(()),
    };

    let is_owned = world
        .get_npc_ai(npc_nid)
        .map(|ai| ai.is_tower_owner)
        .unwrap_or(false);
    if !is_owned {
        return Ok(());
    }

    // StateChange(NPC_SHOW) calls SendInOut(INOUT_IN) — broadcasts NPC reappear to region.
    let show_pkt = crate::npc::build_npc_inout(crate::npc::NPC_IN, &npc, &tmpl);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(show_pkt),
        None,
        event_room,
    );

    world.update_npc_ai(npc_nid, |ai| {
        ai.is_tower_owner = false;
    });

    world.set_tower_owner_id(sid, -1);

    //   AbnormalType abtype = ABNORMAL_NORMAL;
    //   if (isGM() && m_bAbnormalType == ABNORMAL_INVISIBLE) abtype = ABNORMAL_INVISIBLE;
    //   StateChangeServerDirect(3, abtype);
    let ch = world.get_character_info(sid);
    let is_gm = ch.as_ref().map(|c| c.authority == 0).unwrap_or(false);
    let abnormal_value = if is_gm {
        // ABNORMAL_INVISIBLE = 0 in C++ GameDefine.h:1396
        0u32 // Preserve GM invisibility
    } else {
        ABNORMAL_NORMAL
    };

    let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
    state_pkt.write_u32(sid as u32);
    state_pkt.write_u8(STATE_CHANGE_ABNORMAL);
    state_pkt.write_u32(abnormal_value);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(state_pkt),
        None,
        event_room,
    );

    world.clear_all_buffs(sid, false);
    world.set_user_ability(sid);
    world.recast_saved_magic(sid);

    session
        .send_packet(&build_dismount_npc_tower_response(sid as u32))
        .await?;

    debug!(
        "[{}] WIZ_MOVING_TOWER cmd=17: dismounted NPC tower {}",
        session.addr(),
        npc_nid
    );
    Ok(())
}

/// Build a WIZ_MOVING_TOWER response for board siege tower (cmd=1).
pub fn build_board_siege_response() -> Packet {
    let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
    pkt.write_u8(CMD_BOARD_SIEGE);
    pkt.write_u8(1);
    pkt
}

/// Build a WIZ_MOVING_TOWER response for dismount siege tower (cmd=2).
pub fn build_dismount_siege_response() -> Packet {
    let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
    pkt.write_u8(CMD_DISMOUNT_SIEGE);
    pkt.write_u8(1);
    pkt
}

/// Build a WIZ_MOVING_TOWER response for mount NPC tower (cmd=16).
/// Wire: `[u8 16] [u8 1] [u32 user_id] [u32 npc_id] [u16 sx] [u16 sz] [u16 sy]`
pub fn build_mount_npc_tower_response(
    user_id: u32,
    npc_id: u32,
    spawn_x: u16,
    spawn_z: u16,
    spawn_y: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
    pkt.write_u8(CMD_MOUNT_NPC_TOWER);
    pkt.write_u8(1);
    pkt.write_u32(user_id);
    pkt.write_u32(npc_id);
    pkt.write_u16(spawn_x);
    pkt.write_u16(spawn_z);
    pkt.write_u16(spawn_y);
    pkt
}

/// Build a WIZ_MOVING_TOWER response for dismount NPC tower (cmd=17).
/// Wire: `[u8 17] [u8 1] [u32 user_id]`
pub fn build_dismount_npc_tower_response(user_id: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
    pkt.write_u8(CMD_DISMOUNT_NPC_TOWER);
    pkt.write_u8(1);
    pkt.write_u32(user_id);
    pkt
}

/// Build a WIZ_MOVING_TOWER death dismount notification (cmd=17).
/// Sent when a player dies while mounted on an NPC tower.
/// Wire: `[u8 17] [u8 1] [u32 user_id] [u32 npc_id]`
pub fn build_tower_death_dismount(user_id: u32, npc_id: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
    pkt.write_u8(CMD_DISMOUNT_NPC_TOWER);
    pkt.write_u8(1);
    pkt.write_u32(user_id);
    pkt.write_u32(npc_id);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, PacketReader};

    #[test]
    fn test_board_siege_response_format() {
        let pkt = build_board_siege_response();
        assert_eq!(pkt.opcode, Opcode::WizMovingTower as u8);
        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CMD_BOARD_SIEGE));
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_dismount_siege_response_format() {
        let pkt = build_dismount_siege_response();
        assert_eq!(pkt.opcode, Opcode::WizMovingTower as u8);
        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CMD_DISMOUNT_SIEGE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_mount_npc_tower_response_format() {
        let pkt = build_mount_npc_tower_response(42, 100, 500, 600, 10);
        assert_eq!(pkt.opcode, Opcode::WizMovingTower as u8);
        // u8 cmd + u8 success + u32 user + u32 npc + u16 sx + u16 sz + u16 sy = 16 bytes
        assert_eq!(pkt.data.len(), 16);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CMD_MOUNT_NPC_TOWER));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(42)); // user_id
        assert_eq!(r.read_u32(), Some(100)); // npc_id
        assert_eq!(r.read_u16(), Some(500)); // spawn_x
        assert_eq!(r.read_u16(), Some(600)); // spawn_z
        assert_eq!(r.read_u16(), Some(10)); // spawn_y
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_dismount_npc_tower_response_format() {
        let pkt = build_dismount_npc_tower_response(42);
        assert_eq!(pkt.opcode, Opcode::WizMovingTower as u8);
        // u8 cmd + u8 success + u32 user = 6 bytes
        assert_eq!(pkt.data.len(), 6);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CMD_DISMOUNT_NPC_TOWER));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_tower_death_dismount_format() {
        let pkt = build_tower_death_dismount(42, 100);
        assert_eq!(pkt.opcode, Opcode::WizMovingTower as u8);
        // u8 cmd + u8 success + u32 user + u32 npc = 10 bytes
        assert_eq!(pkt.data.len(), 10);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(CMD_DISMOUNT_NPC_TOWER));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u32(), Some(100));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_sub_command_constants() {
        assert_eq!(CMD_BOARD_SIEGE, 1);
        assert_eq!(CMD_DISMOUNT_SIEGE, 2);
        assert_eq!(CMD_MOUNT_NPC_TOWER, 16);
        assert_eq!(CMD_DISMOUNT_NPC_TOWER, 17);
    }

    #[test]
    fn test_client_request_parsing_cmd1() {
        // Client sends: [u8 command=1]
        let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
        pkt.write_u8(CMD_BOARD_SIEGE);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_client_request_parsing_cmd2() {
        // Client sends: [u8 command=2] [u16 pos_x] [u16 pos_z]
        let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
        pkt.write_u8(CMD_DISMOUNT_SIEGE);
        pkt.write_u16(1500);
        pkt.write_u16(2000);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(1500));
        assert_eq!(r.read_u16(), Some(2000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_client_request_parsing_cmd16() {
        // Client sends: [u8 command=16] [u16 npc_id]
        let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
        pkt.write_u8(CMD_MOUNT_NPC_TOWER);
        pkt.write_u16(350);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(16));
        assert_eq!(r.read_u16(), Some(350));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_client_request_parsing_cmd17() {
        // Client sends: [u8 command=17]
        let mut pkt = Packet::new(Opcode::WizMovingTower as u8);
        pkt.write_u8(CMD_DISMOUNT_NPC_TOWER);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(17));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_board_siege_state_change_packet_format() {
        // StateChangeServerDirect(11, BOARDING=8) broadcast format:
        // [u32 socket_id] [u8 type=11] [u32 value=8]
        let sid: u32 = 42;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(sid);
        pkt.write_u8(STATE_CHANGE_TEAM_COLOUR);
        pkt.write_u32(ABNORMAL_BOARDING);

        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        assert_eq!(pkt.data.len(), 9); // 4 + 1 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(11)); // STATE_CHANGE_TEAM_COLOUR
        assert_eq!(r.read_u32(), Some(8)); // ABNORMAL_BOARDING
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_dismount_siege_state_change_packet_format() {
        // StateChangeServerDirect(11, ABNORMAL_NORMAL=1) broadcast format
        let sid: u32 = 42;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(sid);
        pkt.write_u8(STATE_CHANGE_TEAM_COLOUR);
        pkt.write_u32(ABNORMAL_NORMAL);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(11));
        assert_eq!(r.read_u32(), Some(1)); // ABNORMAL_NORMAL
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_mount_npc_tower_state_change_packet_format() {
        // StateChangeServerDirect(3, 450018) broadcast format
        let sid: u32 = 42;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(sid);
        pkt.write_u8(STATE_CHANGE_ABNORMAL);
        pkt.write_u32(TOWER_TRANSFORM_ID);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(3)); // STATE_CHANGE_ABNORMAL
        assert_eq!(r.read_u32(), Some(450018)); // TOWER_TRANSFORM_ID
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_dismount_npc_tower_state_change_packet_format() {
        // StateChangeServerDirect(3, ABNORMAL_NORMAL=1) broadcast format
        let sid: u32 = 42;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(sid);
        pkt.write_u8(STATE_CHANGE_ABNORMAL);
        pkt.write_u32(ABNORMAL_NORMAL);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(1)); // ABNORMAL_NORMAL
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_npc_type_tower_constant() {
        assert_eq!(NPC_TYPE_TOWER, 191);
    }

    #[test]
    fn test_tower_transform_id_constant() {
        // C++ TowerTransformationProcess.cpp:73 uses literal 450018
        assert_eq!(TOWER_TRANSFORM_ID, 450018);
    }

    #[test]
    fn test_abnormal_constants() {
        // C++ GameDefine.h values
        assert_eq!(ABNORMAL_BOARDING, 8);
        assert_eq!(ABNORMAL_NORMAL, 1);
    }

    #[test]
    fn test_spawn_position_encoding() {
        // C++ GetSPosX() = GetX() * 10
        let npc_x: f32 = 123.5;
        let npc_z: f32 = 456.7;
        let npc_y: f32 = 0.0;
        let sx = (npc_x * 10.0) as u16;
        let sz = (npc_z * 10.0) as u16;
        let sy = (npc_y * 10.0) as u16;
        assert_eq!(sx, 1235);
        assert_eq!(sz, 4567);
        assert_eq!(sy, 0);
    }

    #[test]
    fn test_warp_position_decoding() {
        // Client sends pos_x/pos_z multiplied by 10, server divides by 10
        let pos_x: u16 = 1500;
        let pos_z: u16 = 2000;
        let warp_x = pos_x as f32 / 10.0;
        let warp_z = pos_z as f32 / 10.0;
        assert!((warp_x - 150.0).abs() < f32::EPSILON);
        assert!((warp_z - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_range_check_squared_distance() {
        // C++ MAX_NPC_RANGE = 121.0f = 11² (squared distance, no sqrt).
        // Player at (100, 100), NPC at (110, 100) → dist² = 100 → in range.
        let dx: f32 = 100.0 - 110.0;
        let dz: f32 = 100.0 - 100.0;
        let dist_sq = dx * dx + dz * dz;
        assert_eq!(dist_sq, 100.0);
        assert!(dist_sq <= MAX_NPC_RANGE_SQUARED); // 100 <= 121

        // Player at (100, 100), NPC at (111, 100) → dist² = 121 → at boundary.
        let dx2: f32 = 100.0 - 111.0;
        let dist_sq2 = dx2 * dx2;
        assert_eq!(dist_sq2, 121.0);
        assert!(dist_sq2 <= MAX_NPC_RANGE_SQUARED); // 121 <= 121

        // Player at (100, 100), NPC at (112, 100) → dist² = 144 → out of range.
        let dx3: f32 = 100.0 - 112.0;
        let dist_sq3 = dx3 * dx3;
        assert_eq!(dist_sq3, 144.0);
        assert!(dist_sq3 > MAX_NPC_RANGE_SQUARED); // 144 > 121
    }

    #[test]
    fn test_range_check_diagonal() {
        // Diagonal: player at (0,0), NPC at (8,8) → dist² = 128 → out of range.
        let dx: f32 = 8.0;
        let dz: f32 = 8.0;
        let dist_sq = dx * dx + dz * dz;
        assert_eq!(dist_sq, 128.0);
        assert!(dist_sq > MAX_NPC_RANGE_SQUARED); // 128 > 121

        // Diagonal: player at (0,0), NPC at (7,7) → dist² = 98 → in range.
        let dx2: f32 = 7.0;
        let dz2: f32 = 7.0;
        let dist_sq2 = dx2 * dx2 + dz2 * dz2;
        assert_eq!(dist_sq2, 98.0);
        assert!(dist_sq2 <= MAX_NPC_RANGE_SQUARED); // 98 <= 121
    }

    #[test]
    fn test_max_npc_range_matches_cpp() {
        // C++ Define.h: MAX_NPC_RANGE = 121.0f = pow(11.0f, 2.0f)
        assert_eq!(MAX_NPC_RANGE_SQUARED, 121.0);
        assert_eq!(MAX_NPC_RANGE_SQUARED, 11.0f32.powi(2));
    }

    #[test]
    fn test_gm_invisibility_preservation_on_dismount() {
        // ABNORMAL_INVISIBLE = 0
        // For GM: StateChangeServerDirect(3, 0) preserves invisibility.
        let abnormal_invisible: u32 = 0;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(42); // sid
        pkt.write_u8(STATE_CHANGE_ABNORMAL);
        pkt.write_u32(abnormal_invisible);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(0)); // ABNORMAL_INVISIBLE
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_non_gm_normal_restore_on_dismount() {
        // Non-GM player: StateChangeServerDirect(3, ABNORMAL_NORMAL=1)
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(42); // sid
        pkt.write_u8(STATE_CHANGE_ABNORMAL);
        pkt.write_u32(ABNORMAL_NORMAL);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(1)); // ABNORMAL_NORMAL
        assert_eq!(r.remaining(), 0);
    }
}
