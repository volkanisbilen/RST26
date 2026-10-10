//! WIZ_REQ_NPCIN (0x1D) handler — respond with NPC info for requested IDs.
//! When the client receives a NPC region list (WIZ_NPC_REGION), it requests
//! detailed info for each NPC it doesn't know about via this opcode.
//! ## Request (Client -> Server)
//! | Type  | Description                  |
//! |-------|------------------------------|
//! | u16le | Requested NPC count          |
//! | u32le | NPC ID (repeated count×)     |
//! ## Response (Server -> Client) — compressed if > 500 bytes
//! | Type  | Description                  |
//! |-------|------------------------------|
//! | u16le | Actual NPC count returned    |
//! Per NPC:
//! | Type  | Description                  |
//! |-------|------------------------------|
//! | u32le | NPC ID                       |
//! | ...   | GetNpcInfo data (variable)   |
//! Note: GetNpcInfo is 43 bytes for default/type-191 NPCs, but variable-length
//! for type-15 NPCs (barracks/pets include string fields). The compression
//! wrapper computes the uncompressed size dynamically, so variable-length
//! packets are handled correctly.

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::npc::{
    build_npc_inout, is_native_moraranker_template, write_npc_info_base, NpcInstance, NpcTemplate,
    NPC_BAND, NPC_IN,
};
use crate::session::ClientSession;

/// Maximum NPCs per response (C++ MAX_SEND_NPCID).
const MAX_SEND_NPCID: u16 = 100;

/// Upper bound for valid NPC IDs (C++ INVALID_BAND).
const INVALID_BAND: u32 = 60000;

/// Handle WIZ_REQ_NPCIN from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let mut npc_count = reader.read_u16().unwrap_or(0);

    if npc_count > 1000 {
        npc_count = 1000;
    }

    let world = session.world().clone();
    let my_sid = session.session_id();
    let my_zone = world.get_position(my_sid).map(|p| p.zone_id).unwrap_or(0);

    let mut result = Packet::new(Opcode::WizReqNpcIn as u8);
    let mut npc_packet_count: u16 = 0;
    let mut native_ranker_inouts: Vec<(Arc<NpcInstance>, Arc<NpcTemplate>)> = Vec::new();

    // Reserve space for NPC count (will overwrite later)
    result.write_u16(0);

    for _ in 0..npc_count {
        let npc_id = match reader.read_u32() {
            Some(id) => id,
            None => break,
        };

        // Validate NPC ID range
        if !(NPC_BAND..=INVALID_BAND).contains(&npc_id) {
            continue;
        }

        // Look up NPC instance
        let instance = match world.get_npc_instance(npc_id) {
            Some(n) => n,
            None => continue,
        };

        // Zone match check
        if instance.zone_id != my_zone {
            continue;
        }

        // Dead NPC check
        if world.is_npc_dead(npc_id) {
            continue;
        }

        // Look up template
        let template = match world.get_npc_template(instance.proto_id, instance.is_monster) {
            Some(t) => t,
            None => continue,
        };

        // Familiars keep their own (transformed) model/size
        let template = world.npc_visual_template(&instance, &template);

        // Write per-NPC data: [u32 npcId] [GetNpcInfo]
        result.write_u32(npc_id);
        write_npc_info_base(&mut result, &instance, &template);
        if is_native_moraranker_template(&template) {
            native_ranker_inouts.push((instance.clone(), template.clone()));
        }

        npc_packet_count += 1;

        if npc_packet_count >= MAX_SEND_NPCID {
            break;
        }

        // Packet size limit (60KB)
        if result.data.len() >= 60000 {
            break;
        }
    }

    if npc_packet_count > 0 {
        // Overwrite the count at offset 0
        let count_bytes = npc_packet_count.to_le_bytes();
        result.data[0] = count_bytes[0];
        result.data[1] = count_bytes[1];

        // Send compressed
        let to_send = match result.to_compressed() {
            Some(compressed) => compressed,
            None => result,
        };
        session.send_packet(&to_send).await?;

        for (instance, template) in native_ranker_inouts {
            let pkt = build_npc_inout(NPC_IN, &instance, &template);
            session.send_packet(&pkt).await?;
        }
    }

    tracing::debug!(
        "[{}] REQ_NPCIN: requested={}, returned={}",
        session.addr(),
        npc_count,
        npc_packet_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_req_npcin_c2s_packet_format() {
        // C2S: [u16 count][u32 npc_id × count]
        let mut pkt = Packet::new(Opcode::WizReqNpcIn as u8);
        pkt.write_u16(3);
        pkt.write_u32(10001);
        pkt.write_u32(10002);
        pkt.write_u32(10003);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u16(), Some(3));
        assert_eq!(reader.read_u32(), Some(10001));
        assert_eq!(reader.read_u32(), Some(10002));
        assert_eq!(reader.read_u32(), Some(10003));
    }

    #[test]
    fn test_req_npcin_constants() {
        assert_eq!(MAX_SEND_NPCID, 100);
        assert_eq!(INVALID_BAND, 60000);
        assert_eq!(NPC_BAND, 10000);
    }

    #[test]
    fn test_req_npcin_npc_id_validation() {
        // Valid range: NPC_BAND(10000)..=INVALID_BAND(60000)
        assert!(!(NPC_BAND..=INVALID_BAND).contains(&0), "0 invalid");
        assert!(!(NPC_BAND..=INVALID_BAND).contains(&9999), "below band");
        assert!(
            (NPC_BAND..=INVALID_BAND).contains(&10000),
            "NPC_BAND boundary"
        );
        assert!((NPC_BAND..=INVALID_BAND).contains(&30000), "mid-range");
        assert!(
            (NPC_BAND..=INVALID_BAND).contains(&60000),
            "INVALID_BAND boundary"
        );
        assert!(
            !(NPC_BAND..=INVALID_BAND).contains(&60001),
            "above invalid band"
        );
    }

    #[test]
    fn test_req_npcin_count_clamped_to_1000() {
        // Handler clamps count to 1000 if > 1000
        let mut count: u16 = 5000;
        if count > 1000 {
            count = 1000;
        }
        assert_eq!(count, 1000);

        // Normal count passes through
        let mut count2: u16 = 50;
        if count2 > 1000 {
            count2 = 1000;
        }
        assert_eq!(count2, 50);
    }

    #[test]
    fn test_req_npcin_response_count_overwrite() {
        // Response uses a reserved u16 at offset 0, overwritten after populating NPCs
        let mut result = Packet::new(Opcode::WizReqNpcIn as u8);
        result.write_u16(0); // placeholder

        // Simulate adding some NPC data
        let actual_count: u16 = 42;
        let count_bytes = actual_count.to_le_bytes();
        result.data[0] = count_bytes[0];
        result.data[1] = count_bytes[1];

        let mut reader = PacketReader::new(&result.data);
        assert_eq!(reader.read_u16(), Some(42), "overwritten count");
    }

    // ── Sprint 926: Additional coverage ──────────────────────────────

    /// C2S data length: count(2) + N * npc_id(4).
    #[test]
    fn test_req_npcin_c2s_data_length() {
        let mut pkt = Packet::new(Opcode::WizReqNpcIn as u8);
        pkt.write_u16(5);
        for i in 0..5u32 {
            pkt.write_u32(10000 + i);
        }
        assert_eq!(pkt.data.len(), 22); // 2 + 5*4
    }

    /// Empty request (count=0) produces no NPC data.
    #[test]
    fn test_req_npcin_empty_request() {
        let mut pkt = Packet::new(Opcode::WizReqNpcIn as u8);
        pkt.write_u16(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// NPC_BAND (10000) is the first valid NPC ID.
    #[test]
    fn test_req_npcin_npc_band_is_inclusive() {
        assert!((NPC_BAND..=INVALID_BAND).contains(&NPC_BAND));
        assert!(!(NPC_BAND..=INVALID_BAND).contains(&(NPC_BAND - 1)));
    }

    /// MAX_SEND_NPCID caps at 100 NPCs per response.
    #[test]
    fn test_req_npcin_max_send_cap() {
        let mut count: u16 = 0;
        for _ in 0..200 {
            count += 1;
            if count >= MAX_SEND_NPCID {
                break;
            }
        }
        assert_eq!(count, MAX_SEND_NPCID);
    }

    /// Response header reserves 2 bytes for count.
    #[test]
    fn test_req_npcin_response_header_size() {
        let mut result = Packet::new(Opcode::WizReqNpcIn as u8);
        result.write_u16(0); // reserved count
        assert_eq!(result.data.len(), 2);
    }
}
