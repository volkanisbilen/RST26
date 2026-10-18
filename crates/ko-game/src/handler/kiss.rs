//! WIZ_KISS (0x66) handler -- Kiss emote / NPC interaction animation.
//! Triggered when a player interacts with a "kiss" NPC event. The server
//! broadcasts the kiss animation to nearby players.
//! ## Client -> Server
//! Empty -- the client triggers this via NPC event system (m_sEventNid).
//! ## Server -> Client (broadcast)
//! ```text
//! [u32 player_id] [i16 event_npc_id]
//! ```
//! The client uses this to play the kiss animation effect between the
//! player and the referenced NPC.

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

/// Handle WIZ_KISS (0x66) -- kiss emote via NPC event.
/// The client sends this when interacting with a kiss-event NPC.
/// Server broadcasts `[u32 player_id] [i16 event_npc_id]` to the region.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Get character info for player ID
    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // C2S may carry a target NPC id, or it may come from session event state.
    // C++ uses m_sEventNid which is set during NPC interaction.
    // If the client sends a target_npc_id in the packet, use it; otherwise default to 0.
    let mut reader = PacketReader::new(&pkt.data);
    let target_npc_id = reader.read_i16().unwrap_or(0);

    // Build S2C broadcast packet
    let broadcast = build_kiss_packet(char_info.session_id as u32, target_npc_id);

    // Get region info for broadcasting
    let region_info = world.with_session(sid, |h| {
        (
            h.position.zone_id,
            h.position.region_x,
            h.position.region_z,
            h.event_room,
        )
    });

    if let Some((zone_id, rx, rz, event_room)) = region_info {
        world.broadcast_to_region_sync(zone_id, rx, rz, Arc::new(broadcast), None, event_room);
    }

    debug!(
        "[{}] WIZ_KISS: player_id={}, npc_id={}",
        session.addr(),
        char_info.session_id,
        target_npc_id
    );
    Ok(())
}

/// Build WIZ_KISS S2C packet.
/// Format: `[u32 player_id] [i16 event_npc_id]`
fn build_kiss_packet(player_id: u32, event_npc_id: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizKiss as u8);
    pkt.write_u32(player_id);
    pkt.write_i16(event_npc_id);
    pkt
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, PacketReader};

    use super::*;

    #[test]
    fn test_kiss_opcode_value() {
        assert_eq!(Opcode::WizKiss as u8, 0x66);
    }

    #[test]
    fn test_kiss_packet_format() {
        let pkt = build_kiss_packet(12345, 100);
        assert_eq!(pkt.opcode, Opcode::WizKiss as u8);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(12345));
        assert_eq!(reader.read_i16(), Some(100));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_kiss_packet_negative_npc_id() {
        let pkt = build_kiss_packet(1, -1);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(1));
        assert_eq!(reader.read_i16(), Some(-1));
    }

    #[test]
    fn test_kiss_packet_data_length() {
        // u32(4) + i16(2) = 6 bytes
        let pkt = build_kiss_packet(0, 0);
        assert_eq!(pkt.data.len(), 6);
    }

    #[test]
    fn test_kiss_opcode_from_byte() {
        assert_eq!(Opcode::from_byte(0x66), Some(Opcode::WizKiss));
    }

    #[test]
    fn test_kiss_packet_roundtrip_various() {
        for (pid, nid) in [(0u32, 0i16), (u32::MAX, i16::MAX), (500, -500)] {
            let pkt = build_kiss_packet(pid, nid);
            let mut r = PacketReader::new(&pkt.data);
            assert_eq!(r.read_u32(), Some(pid));
            assert_eq!(r.read_i16(), Some(nid));
            assert_eq!(r.remaining(), 0);
        }
    }
}
