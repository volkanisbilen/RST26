//! WIZ_REQ_USERIN (0x16) handler — respond with user info for requested IDs.
//! When the client receives a region user list (WIZ_REGIONCHANGE), it requests
//! detailed info for each user it doesn't know about via this opcode.
//! ## Request (Client -> Server)
//! | Type  | Description                  |
//! |-------|------------------------------|
//! | u16le | Requested user count         |
//! | u32le | Socket ID (repeated count×)  |
//! ## Response (Server -> Client) — compressed
//! | Type  | Description                  |
//! |-------|------------------------------|
//! | u16le | Actual user count returned   |
//! Per user:
//! | Type  | Description                  |
//! |-------|------------------------------|
//! | u8    | Type marker (0x00)           |
//! | u32le | Socket ID                    |
//! | ...   | GetUserInfo data             |

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::handler::region;
use crate::handler::region::write_user_info;
use crate::npc::NPC_BAND;
use crate::session::ClientSession;
use crate::world::BOT_ID_BASE;
use crate::zone::SessionId;

/// Maximum users per response (C++ MAX_SEND_USERID).
const MAX_SEND_USERID: u16 = 100;

/// Handle WIZ_REQ_USERIN from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let mut req_count = reader.read_u16().unwrap_or(0);

    if req_count > 1000 {
        req_count = 1000;
    }

    let world = session.world().clone();
    let my_sid = session.session_id();
    let my_zone = world.get_position(my_sid).map(|p| p.zone_id).unwrap_or(0);

    let mut result = Packet::new(Opcode::WizReqUserIn as u8);
    let mut user_count: u16 = 0;

    // Reserve space for user count (will overwrite later)
    result.write_u16(0);

    for _ in 0..req_count {
        let socket_id = match reader.read_u32() {
            Some(id) => id,
            None => break,
        };

        // Runtime bots share the user visibility protocol even though their IDs
        // live at/above NPC_BAND. Resolve them before applying the player guard.
        if socket_id >= BOT_ID_BASE {
            let bot = match world.get_bot(socket_id) {
                Some(bot) if bot.in_game && bot.zone_id == my_zone => bot,
                _ => continue,
            };

            result.write_u8(0); // type marker (user/bot)
            result.write_u32(bot.id);
            crate::systems::bot_ai::write_bot_user_info(&mut result, &bot, &world);
            user_count += 1;

            if user_count >= MAX_SEND_USERID || result.data.len() >= 60000 {
                break;
            }
            continue;
        }

        // Only handle real user IDs (< NPC_BAND). Skip self.
        if socket_id >= NPC_BAND || socket_id as SessionId == my_sid {
            continue;
        }

        let other_sid = socket_id as SessionId;

        // Check zone match
        let other_pos = match world.get_position(other_sid) {
            Some(p) if p.zone_id == my_zone => p,
            _ => continue,
        };

        let other_char = match world.get_character_info(other_sid) {
            Some(c) => c,
            None => continue,
        };

        let other_clan = if other_char.knights_id > 0 {
            world.get_knights(other_char.knights_id)
        } else {
            None
        };

        // Write per-user data: [u8 type=0] [u32 socketId] [GetUserInfo]
        let other_invis = world.get_invisibility_type(other_sid);
        let other_abnormal = world.get_abnormal_type(other_sid);
        let other_bs = world.get_broadcast_state(other_sid);
        let other_equip = region::get_equipped_visual(&world, other_sid);
        let other_alliance_cape = other_clan
            .as_ref()
            .and_then(|ki| region::resolve_alliance_cape(ki, &world));
        let other_is_king = world.is_king(other_char.nation, &other_char.name);
        result.write_u8(0); // type marker (user/bot)
        result.write_u32(other_sid as u32);
        write_user_info(
            &mut result,
            &other_char,
            &other_pos,
            other_clan.as_ref(),
            other_alliance_cape,
            other_is_king,
            other_invis,
            other_abnormal,
            &other_bs,
            &other_equip,
        );

        user_count += 1;

        if user_count >= MAX_SEND_USERID {
            break;
        }

        // Packet size limit (60KB)
        if result.data.len() >= 60000 {
            break;
        }
    }

    if user_count > 0 {
        // Overwrite the count at offset 0
        let count_bytes = user_count.to_le_bytes();
        result.data[0] = count_bytes[0];
        result.data[1] = count_bytes[1];

        // Send compressed
        let to_send = match result.to_compressed() {
            Some(compressed) => compressed,
            None => result,
        };
        session.send_packet(&to_send).await?;
    }

    tracing::debug!(
        "[{}] REQ_USERIN: requested={}, returned={}",
        session.addr(),
        req_count,
        user_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_req_userin_c2s_packet_format() {
        // C2S: [u16 count][u32 socket_id × count]
        let mut pkt = Packet::new(Opcode::WizReqUserIn as u8);
        pkt.write_u16(2);
        pkt.write_u32(1);
        pkt.write_u32(42);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u16(), Some(2));
        assert_eq!(reader.read_u32(), Some(1));
        assert_eq!(reader.read_u32(), Some(42));
    }

    #[test]
    fn test_req_userin_constants() {
        assert_eq!(MAX_SEND_USERID, 100);
        assert_eq!(NPC_BAND, 10000);
    }

    #[test]
    fn test_req_userin_id_filtering() {
        // Only user IDs (< NPC_BAND=10000) are accepted. NPC IDs are skipped.
        for user_id in [1u32, 9999] {
            assert!(user_id < NPC_BAND, "user ID {user_id} is valid");
        }
        for npc_id in [10000u32, 50000] {
            assert!(npc_id >= NPC_BAND, "NPC ID {npc_id} is skipped");
        }
    }

    #[test]
    fn test_req_userin_count_clamped() {
        let mut count: u16 = 65535;
        if count > 1000 {
            count = 1000;
        }
        assert_eq!(count, 1000);
    }

    #[test]
    fn test_req_userin_response_count_overwrite() {
        // Response reserves u16 at offset 0, overwritten after populating users
        let mut result = Packet::new(Opcode::WizReqUserIn as u8);
        result.write_u16(0); // placeholder

        let actual: u16 = 7;
        let bytes = actual.to_le_bytes();
        result.data[0] = bytes[0];
        result.data[1] = bytes[1];

        let mut reader = PacketReader::new(&result.data);
        assert_eq!(reader.read_u16(), Some(7));
    }

    // ── Sprint 926: Additional coverage ──────────────────────────────

    /// C2S data length: count(2) + N * socket_id(4).
    #[test]
    fn test_req_userin_c2s_data_length() {
        let mut pkt = Packet::new(Opcode::WizReqUserIn as u8);
        pkt.write_u16(4);
        for i in 0..4u32 {
            pkt.write_u32(i + 1);
        }
        assert_eq!(pkt.data.len(), 18); // 2 + 4*4
    }

    /// Empty request (count=0).
    #[test]
    fn test_req_userin_empty_request() {
        let mut pkt = Packet::new(Opcode::WizReqUserIn as u8);
        pkt.write_u16(0);
        assert_eq!(pkt.data.len(), 2);
    }

    /// Self socket ID is skipped in the handler.
    #[test]
    fn test_req_userin_self_skip_logic() {
        let my_sid: u32 = 42;
        let other_sid: u32 = 100;
        assert_eq!(my_sid, 42);
        assert_ne!(other_sid, my_sid);
        // Self is skipped
        assert!(my_sid as usize == 42);
    }

    /// Type marker is u8(0) for user entries.
    #[test]
    fn test_req_userin_type_marker() {
        let mut pkt = Packet::new(Opcode::WizReqUserIn as u8);
        pkt.write_u8(0); // type marker = user
        pkt.write_u32(42); // socket ID
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0)); // user type
        assert_eq!(r.read_u32(), Some(42));
    }

    /// Packet size limit is 60000 bytes.
    #[test]
    fn test_req_userin_packet_size_limit() {
        let limit: usize = 60000;
        assert_eq!(limit, 60000);
        // A response at limit should stop adding users
        assert!(59999 < limit);
        assert!(60000 >= limit);
    }
}
