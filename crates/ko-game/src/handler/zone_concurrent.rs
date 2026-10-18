//! WIZ_ZONE_CONCURRENT (0x4D) handler -- Battle zone user counts.
//! This packet provides concurrent user counts for battle zones.
//! The client uses it to display zone population in the battle zone
//! selection UI, helping players choose which zone to enter.
//! ## Client -> Server
//! Empty (or ignored) -- the client triggers this when opening the
//! battle zone selection dialog.
//! ## Server -> Client
//! ```text
//! [u8 zone_count] [per_zone: [u16 zone_id] [u16 user_count]] ...
//! ```
//! IDA analysis (sub_D7D2B0): The client iterates over zone entries,
//! reads zone info and builds a concatenated string for the zone
//! selection UI. The format matches the C++ pattern of listing
//! battle zone populations.

use ko_protocol::{Opcode, Packet};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::types::{
    ZONE_ARDREAM, ZONE_BATTLE, ZONE_BATTLE2, ZONE_BATTLE3, ZONE_BATTLE4, ZONE_BATTLE5,
    ZONE_BATTLE6, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE,
};

/// Battle zone IDs for which we report concurrent user counts.
const BATTLE_ZONES: &[u16] = &[
    ZONE_BATTLE,
    ZONE_BATTLE2,
    ZONE_BATTLE3,
    ZONE_BATTLE4,
    ZONE_BATTLE5,
    ZONE_BATTLE6,
    ZONE_RONARK_LAND,
    ZONE_ARDREAM,
    ZONE_RONARK_LAND_BASE,
];

/// Handle WIZ_ZONE_CONCURRENT (0x4D) -- send battle zone user counts.
/// When the client opens the battle zone selection UI, it sends this
/// opcode. The server responds with user counts per battle zone.
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();

    let pkt = build_zone_concurrent_packet(&world);
    session.send_packet(&pkt).await?;

    debug!(
        "[{}] WIZ_ZONE_CONCURRENT: sent battle zone counts",
        session.addr()
    );
    Ok(())
}

/// Build WIZ_ZONE_CONCURRENT S2C packet.
/// Format: `[u8 zone_count] [per_zone: [u16 zone_id] [u16 user_count]]`
fn build_zone_concurrent_packet(world: &crate::world::WorldState) -> Packet {
    let mut pkt = Packet::new(Opcode::WizZoneConcurrent as u8);

    let count = BATTLE_ZONES.len() as u8;
    pkt.write_u8(count);

    for &zone_id in BATTLE_ZONES {
        let user_count = world.zone_player_count(zone_id);
        pkt.write_u16(zone_id);
        pkt.write_u16(user_count);
    }

    pkt
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, PacketReader};

    use super::*;

    #[test]
    fn test_zone_concurrent_opcode_value() {
        assert_eq!(Opcode::WizZoneConcurrent as u8, 0x4D);
    }

    #[test]
    fn test_zone_concurrent_opcode_from_byte() {
        assert_eq!(Opcode::from_byte(0x4D), Some(Opcode::WizZoneConcurrent));
    }

    #[test]
    fn test_battle_zones_count() {
        assert_eq!(BATTLE_ZONES.len(), 9);
    }

    #[test]
    fn test_battle_zones_contains_expected() {
        assert!(BATTLE_ZONES.contains(&ZONE_BATTLE));
        assert!(BATTLE_ZONES.contains(&ZONE_RONARK_LAND));
        assert!(BATTLE_ZONES.contains(&ZONE_ARDREAM));
    }

    #[test]
    fn test_zone_concurrent_packet_format_manual() {
        // Build a manual packet and verify format
        let mut pkt = Packet::new(Opcode::WizZoneConcurrent as u8);
        pkt.write_u8(2); // 2 zones
        pkt.write_u16(61); // zone_id
        pkt.write_u16(50); // user_count
        pkt.write_u16(62); // zone_id
        pkt.write_u16(30); // user_count

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(2));
        assert_eq!(reader.read_u16(), Some(61));
        assert_eq!(reader.read_u16(), Some(50));
        assert_eq!(reader.read_u16(), Some(62));
        assert_eq!(reader.read_u16(), Some(30));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_zone_concurrent_data_length() {
        // u8(1) + 2 * (u16 + u16)(4) = 1 + 8 = 9
        let mut pkt = Packet::new(Opcode::WizZoneConcurrent as u8);
        pkt.write_u8(2);
        pkt.write_u16(61);
        pkt.write_u16(50);
        pkt.write_u16(62);
        pkt.write_u16(30);
        assert_eq!(pkt.data.len(), 9);
    }

    #[test]
    fn test_zone_concurrent_zero_users() {
        let mut pkt = Packet::new(Opcode::WizZoneConcurrent as u8);
        pkt.write_u8(1);
        pkt.write_u16(ZONE_BATTLE);
        pkt.write_u16(0);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(1));
        assert_eq!(reader.read_u16(), Some(ZONE_BATTLE));
        assert_eq!(reader.read_u16(), Some(0));
    }

    #[test]
    fn test_zone_concurrent_all_battle_zones() {
        // Verify all 9 battle zones produce valid packet
        let mut pkt = Packet::new(Opcode::WizZoneConcurrent as u8);
        pkt.write_u8(BATTLE_ZONES.len() as u8);
        for &zone in BATTLE_ZONES {
            pkt.write_u16(zone);
            pkt.write_u16(100);
        }
        // u8(1) + 9 * 4 = 37
        assert_eq!(pkt.data.len(), 1 + 9 * 4);
    }
}
