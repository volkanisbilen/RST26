//! WIZ_ROOM_PACKET (0x61) handler -- Room/instance system.
//!   `#define WIZ_ROOM_PACKETPROCESS 0x61`
//! The room system manages instanced content like battle royale rooms,
//! event instances, and party dungeon rooms. Players can create, join,
//! leave, and list available rooms.
//! IDA analysis (sub_634B90): The client-side dispatch for 0x61 triggers
//! `sub_62FB10(a2, 3, 0)` which builds a C2S packet `[0xD0][6][2][3]`.
//! The room packet process acts as a coordinator for the room UI.
//! ## Sub-opcodes
//! | Code | Name       | Direction | Description              |
//! |------|------------|-----------|--------------------------|
//! | 0x01 | CREATE     | C2S       | Create a new room        |
//! | 0x02 | JOIN       | C2S       | Join an existing room    |
//! | 0x03 | LEAVE      | C2S       | Leave current room       |
//! | 0x04 | LIST       | C2S/S2C   | List available rooms     |
//! | 0x05 | INFO       | S2C       | Room info update         |
//! | 0x06 | READY      | C2S       | Toggle ready state       |
//! | 0x07 | START      | S2C       | Room start notification  |
//! ## Server -> Client (LIST response)
//! ```text
//! [u8 sub=4] [u8 room_count] [per_room: [u16 room_id] [sbyte_string name]
//!  [u8 player_count] [u8 max_players] [u8 status]]
//! ```

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

/// Room sub-opcode constants.
const ROOM_CREATE: u8 = 0x01;
const ROOM_JOIN: u8 = 0x02;
const ROOM_LEAVE: u8 = 0x03;
const ROOM_LIST: u8 = 0x04;
const ROOM_INFO: u8 = 0x05;
const ROOM_READY: u8 = 0x06;
const ROOM_START: u8 = 0x07;

/// Room result codes.
const ROOM_RESULT_SUCCESS: u8 = 1;
const ROOM_RESULT_FAIL: u8 = 0;
#[allow(dead_code)]
const ROOM_RESULT_FULL: u8 = 2;
const ROOM_RESULT_NOT_FOUND: u8 = 3;
#[allow(dead_code)]
const ROOM_RESULT_ALREADY_IN: u8 = 4;

/// Room status codes.
#[allow(dead_code)]
const ROOM_STATUS_WAITING: u8 = 0;
#[allow(dead_code)]
const ROOM_STATUS_PLAYING: u8 = 1;
#[allow(dead_code)]
const ROOM_STATUS_CLOSED: u8 = 2;

/// Handle WIZ_ROOM_PACKET (0x61) -- room/instance operations.
/// Routes to sub-handlers based on the sub-opcode byte.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = match reader.read_u8() {
        Some(op) => op,
        None => return Ok(()),
    };

    match sub_opcode {
        ROOM_CREATE => handle_create(session, &mut reader).await,
        ROOM_JOIN => handle_join(session, &mut reader).await,
        ROOM_LEAVE => handle_leave(session).await,
        ROOM_LIST => handle_list(session).await,
        ROOM_READY => handle_ready(session).await,
        _ => {
            debug!(
                "[{}] WIZ_ROOM_PACKET: unknown sub-opcode 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle ROOM_CREATE (0x01) -- create a new room.
/// C2S: `[u8 sub=1] [sbyte_string name] [u8 max_players] [u8 room_type]`
/// S2C: `[u8 sub=1] [u8 result] [u16 room_id]`
async fn handle_create(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let name = reader.read_sbyte_string().unwrap_or_default();
    let max_players = reader.read_u8().unwrap_or(8);
    let room_type = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] ROOM_CREATE: name='{}', max={}, type={}",
        session.addr(),
        name,
        max_players,
        room_type
    );

    // Stub response: room creation not yet backed by world state
    let response = build_create_result(ROOM_RESULT_FAIL, 0);
    session.send_packet(&response).await?;
    Ok(())
}

/// Handle ROOM_JOIN (0x02) -- join an existing room.
/// C2S: `[u8 sub=2] [u16 room_id]`
/// S2C: `[u8 sub=2] [u8 result]`
async fn handle_join(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let room_id = reader.read_u16().unwrap_or(0);

    debug!("[{}] ROOM_JOIN: room_id={}", session.addr(), room_id);

    let response = build_result_packet(ROOM_JOIN, ROOM_RESULT_NOT_FOUND);
    session.send_packet(&response).await?;
    Ok(())
}

/// Handle ROOM_LEAVE (0x03) -- leave current room.
/// C2S: `[u8 sub=3]`
/// S2C: `[u8 sub=3] [u8 result]`
async fn handle_leave(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] ROOM_LEAVE", session.addr());

    let response = build_result_packet(ROOM_LEAVE, ROOM_RESULT_SUCCESS);
    session.send_packet(&response).await?;
    Ok(())
}

/// Handle ROOM_LIST (0x04) -- list available rooms.
/// C2S: `[u8 sub=4] [u8 room_type]`
/// S2C: `[u8 sub=4] [u8 count=0]` (empty list for now)
async fn handle_list(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] ROOM_LIST", session.addr());

    let response = build_empty_list_packet();
    session.send_packet(&response).await?;
    Ok(())
}

/// Handle ROOM_READY (0x06) -- toggle ready state.
/// C2S: `[u8 sub=6]`
/// S2C: `[u8 sub=6] [u8 result]`
async fn handle_ready(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] ROOM_READY", session.addr());

    let response = build_result_packet(ROOM_READY, ROOM_RESULT_SUCCESS);
    session.send_packet(&response).await?;
    Ok(())
}

/// Build a simple result packet for room operations.
/// Format: `[u8 sub_opcode] [u8 result]`
fn build_result_packet(sub_opcode: u8, result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRoomPacketProcess as u8);
    pkt.write_u8(sub_opcode);
    pkt.write_u8(result);
    pkt
}

/// Build create room result with room ID.
/// Format: `[u8 sub=1] [u8 result] [u16 room_id]`
fn build_create_result(result: u8, room_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRoomPacketProcess as u8);
    pkt.write_u8(ROOM_CREATE);
    pkt.write_u8(result);
    pkt.write_u16(room_id);
    pkt
}

/// Build empty room list response.
/// Format: `[u8 sub=4] [u8 count=0]`
fn build_empty_list_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::WizRoomPacketProcess as u8);
    pkt.write_u8(ROOM_LIST);
    pkt.write_u8(0); // count = 0
    pkt
}

/// Build room info S2C packet.
/// Format: `[u8 sub=5] [u16 room_id] [sbyte_string name] [u8 player_count]
///          [u8 max_players] [u8 status]`
pub fn build_room_info_packet(
    room_id: u16,
    name: &str,
    player_count: u8,
    max_players: u8,
    status: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRoomPacketProcess as u8);
    pkt.write_u8(ROOM_INFO);
    pkt.write_u16(room_id);
    pkt.write_sbyte_string(name);
    pkt.write_u8(player_count);
    pkt.write_u8(max_players);
    pkt.write_u8(status);
    pkt
}

/// Build room start notification S2C packet.
/// Format: `[u8 sub=7] [u16 room_id]`
pub fn build_room_start_packet(room_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRoomPacketProcess as u8);
    pkt.write_u8(ROOM_START);
    pkt.write_u16(room_id);
    pkt
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, PacketReader};

    use super::*;

    #[test]
    fn test_room_packet_opcode_value() {
        assert_eq!(Opcode::WizRoomPacketProcess as u8, 0x61);
    }

    #[test]
    fn test_room_packet_opcode_from_byte() {
        assert_eq!(Opcode::from_byte(0x61), Some(Opcode::WizRoomPacketProcess));
    }

    #[test]
    fn test_room_sub_opcodes() {
        assert_eq!(ROOM_CREATE, 0x01);
        assert_eq!(ROOM_JOIN, 0x02);
        assert_eq!(ROOM_LEAVE, 0x03);
        assert_eq!(ROOM_LIST, 0x04);
        assert_eq!(ROOM_INFO, 0x05);
        assert_eq!(ROOM_READY, 0x06);
        assert_eq!(ROOM_START, 0x07);
    }

    #[test]
    fn test_room_result_codes() {
        assert_eq!(ROOM_RESULT_SUCCESS, 1);
        assert_eq!(ROOM_RESULT_FAIL, 0);
        assert_eq!(ROOM_RESULT_FULL, 2);
        assert_eq!(ROOM_RESULT_NOT_FOUND, 3);
        assert_eq!(ROOM_RESULT_ALREADY_IN, 4);
    }

    #[test]
    fn test_room_status_codes() {
        assert_eq!(ROOM_STATUS_WAITING, 0);
        assert_eq!(ROOM_STATUS_PLAYING, 1);
        assert_eq!(ROOM_STATUS_CLOSED, 2);
    }

    #[test]
    fn test_result_packet_format() {
        let pkt = build_result_packet(ROOM_JOIN, ROOM_RESULT_SUCCESS);
        assert_eq!(pkt.opcode, Opcode::WizRoomPacketProcess as u8);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(ROOM_JOIN));
        assert_eq!(reader.read_u8(), Some(ROOM_RESULT_SUCCESS));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_create_result_packet_format() {
        let pkt = build_create_result(ROOM_RESULT_SUCCESS, 42);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(ROOM_CREATE));
        assert_eq!(reader.read_u8(), Some(ROOM_RESULT_SUCCESS));
        assert_eq!(reader.read_u16(), Some(42));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_empty_list_packet_format() {
        let pkt = build_empty_list_packet();
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(ROOM_LIST));
        assert_eq!(reader.read_u8(), Some(0));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_room_info_packet_format() {
        let pkt = build_room_info_packet(100, "Test Room", 3, 8, ROOM_STATUS_WAITING);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(ROOM_INFO));
        assert_eq!(reader.read_u16(), Some(100));
        assert_eq!(reader.read_sbyte_string().as_deref(), Some("Test Room"));
        assert_eq!(reader.read_u8(), Some(3));
        assert_eq!(reader.read_u8(), Some(8));
        assert_eq!(reader.read_u8(), Some(ROOM_STATUS_WAITING));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_room_start_packet_format() {
        let pkt = build_room_start_packet(42);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(ROOM_START));
        assert_eq!(reader.read_u16(), Some(42));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_result_packet_data_length() {
        let pkt = build_result_packet(0x02, 0x01);
        assert_eq!(pkt.data.len(), 2);
    }

    #[test]
    fn test_create_result_data_length() {
        let pkt = build_create_result(1, 100);
        // sub(1) + result(1) + room_id(2) = 4
        assert_eq!(pkt.data.len(), 4);
    }

    #[test]
    fn test_room_start_data_length() {
        let pkt = build_room_start_packet(1);
        // sub(1) + room_id(2) = 3
        assert_eq!(pkt.data.len(), 3);
    }

    #[test]
    fn test_result_packet_all_sub_opcodes() {
        for sub in [ROOM_CREATE, ROOM_JOIN, ROOM_LEAVE, ROOM_LIST, ROOM_READY] {
            let pkt = build_result_packet(sub, ROOM_RESULT_SUCCESS);
            let mut r = PacketReader::new(&pkt.data);
            assert_eq!(r.read_u8(), Some(sub));
            assert_eq!(r.read_u8(), Some(ROOM_RESULT_SUCCESS));
        }
    }
}
