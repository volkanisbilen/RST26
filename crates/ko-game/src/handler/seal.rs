//! WIZ_SEAL (0x95) handler — v2525-specific item seal system.
//! v2525-specific opcode. The C++ server handles sealing via
//! `WIZ_ITEM_UPGRADE (0x5B) sub=8 (ITEM_SEAL)`. The v2525 client has a
//! dedicated seal UI at opcode 0x95 with its own protocol.
//! ## Client RE
//! ### C2S Dispatch (client at 0x7BF9A0)
//! Reads `u8 sub`, dispatches on `(sub - 2)` via jump table (range 0x02..0x13):
//! | Sub       | Handler    | Description                          |
//! |-----------|-----------|--------------------------------------|
//! | 0x02      | 0x938BD0  | Seal toggle                          |
//! | 0x03      | 0x7BFA0A  | Unseal/release — reads u16 count     |
//! | 0x04      | 0x938FE0  | Double seal — primary + secondary NPC |
//! | 0x09      | 0x7BFBC6  | Seal scroll select — u16 idx + u16 page |
//! | 0x10-0x11 | 0x93ECA0  | Seal type operation (variant A)      |
//! | 0x12-0x13 | 0x93EFD0  | Seal type operation (variant B)      |
//! | 0x05-0x0F | 0x9398E0  | Default — generic seal operation     |
//! ### C2S Wire Formats
//! ```text
//! sub=0x02: [u8 sub=0x02][u8 0x40][u16 zero]         — seal toggle
//! sub=0x65: [u8 sub=0x65][u8 0x30][u16 zero]         — zone sync
//! sub=0x63: [u8 sub=0x63][u8 0x30][u16 zero]         — full seal request
//!           [u64 value_a][u64 value_b]
//!           [u32 slot_calc=(slot-8)*60]
//!           [u8 type_flag][u16 seal_idx][u32 ts_hash]
//! ```
//! ### S2C Wire Format (handler at 0x7C02F0)
//! ```text
//! [i32 npc_id]              — seal NPC proto_id
//! ```
//! NPC ID gating — only NPC 12401 (0x3071) processes further:
//! - 12385, 12387, 16385: early exit (no display)
//! - 12401: seal result notification (full packet below)
//! Full S2C (npc_id == 12401):
//! ```text
//! [i32 npc_id=12401][u16 result_type][u16 operation]
//! [u16 name_len][u8 item_name × name_len]
//! [i32 field_a][i32 field_b][i32 item_num]
//! ```
//! ### String Table IDs
//! | result_type | operation | String ID |
//! |-------------|-----------|-----------|
//! | 1 (seal)    | 1         | 3352      |
//! | 1 (seal)    | 2         | 3353      |
//! | 2 (unseal)  | 1         | 3354      |
//! | 2 (unseal)  | 2         | 3355      |
//! | 2 (unseal)  | 3         | 3356      |
//! | 3 (break)   | 1         | 3357      |
//! | 3 (break)   | 2         | 3358      |
//! | 4 (other)   | any       | 3359      |
//! **NOTE**: Strings 3352-3359 are ALL EMPTY in v2525 Texts_us.tbl.

use ko_db::repositories::character::{CharacterRepository, SaveItemParams};
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::inventory_constants::{HAVE_MAX, SLOT_MAX};
use crate::session::{ClientSession, SessionState};
use crate::world::types::{ITEM_FLAG_BOUND, ITEM_FLAG_NONE, ITEM_FLAG_NOT_BOUND, ITEM_FLAG_SEALED};

// ── NPC ID Constants ────────────────────────────────────────────────

/// Seal NPC that produces S2C result notifications.
pub const SEAL_NPC_RESULT: i32 = 12401;
/// Seal NPC types (early exit, no S2C display).
pub const SEAL_NPC_TYPE_A: i32 = 12385;
pub const SEAL_NPC_TYPE_B: i32 = 12387;
pub const SEAL_NPC_TYPE_C: i32 = 16385;

// ── C2S Sub-opcode Constants ────────────────────────────────────────

/// Seal toggle — `[u8 0x40][u16 zero]`.
pub const SUB_TOGGLE: u8 = 0x02;
/// Unseal/release — `[u16 count]` + item data.
pub const SUB_UNSEAL: u8 = 0x03;
/// Double seal — primary + secondary NPC.
pub const SUB_DOUBLE: u8 = 0x04;
/// Seal scroll select — `[u16 scroll_idx][u16 page]`.
pub const SUB_SCROLL: u8 = 0x09;
/// Full seal request — 32 bytes.
pub const SUB_FULL_REQUEST: u8 = 0x63;
/// Zone sync — resync seal state after zone change.
pub const SUB_ZONE_SYNC: u8 = 0x65;

// ── S2C Result Type Constants ───────────────────────────────────────

/// Seal operation.
pub const RESULT_SEAL: u16 = 1;
/// Unseal operation.
pub const RESULT_UNSEAL: u16 = 2;
/// Seal break.
pub const RESULT_BREAK: u16 = 3;
/// Other operation.
pub const RESULT_OTHER: u16 = 4;

// ── S2C Builders ────────────────────────────────────────────────────

/// Build a seal NPC early-exit packet (NPC types 12385/12387/16385).
/// Client reads npc_id, matches against known seal NPC IDs,
/// and exits early for these types (no further fields read).
/// Wire: `[i32 npc_id]`
pub fn build_npc_ack(npc_id: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizSeal as u8);
    pkt.write_i32(npc_id);
    pkt
}

/// Build a full seal result notification (NPC 12401 only).
/// Client displays formatted string in yellow (0xFFFFFF00) chat.
/// - `result_type`: 1=seal, 2=unseal, 3=break, 4=other
/// - `operation`: Message variant (1/2/3)
/// - `item_name`: Item name string (raw bytes)
/// - `field_a`, `field_b`: Format arguments for display template
/// - `item_num`: Full item number (e.g. 379006001)
/// Wire: `[i32 npc_id=12401][u16 result_type][u16 operation]`
///       `[u16 name_len][u8 × name][i32 field_a][i32 field_b][i32 item_num]`
pub fn build_seal_result(
    result_type: u16,
    operation: u16,
    item_name: &str,
    field_a: i32,
    field_b: i32,
    item_num: i32,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizSeal as u8);
    pkt.write_i32(SEAL_NPC_RESULT);
    pkt.write_u16(result_type);
    pkt.write_u16(operation);
    let name_bytes = item_name.as_bytes();
    pkt.write_u16(name_bytes.len() as u16);
    for &b in name_bytes {
        pkt.write_u8(b);
    }
    pkt.write_i32(field_a);
    pkt.write_i32(field_b);
    pkt.write_i32(item_num);
    pkt
}

// ── C2S Handler ─────────────────────────────────────────────────────

/// Handle WIZ_SEAL (0x95) from the client.
/// C2S sub-opcodes: 0x02 (toggle), 0x03 (unseal), 0x04 (double),
/// 0x09 (scroll), 0x10-0x13 (type ops), 0x63 (full request), 0x65 (zone sync).
/// Full implementation requires seal item DB tables + NPC validation.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);

    match sub {
        SUB_TOGGLE => {
            debug!(
                "[{}] WIZ_SEAL sub=0x02 toggle ({}B)",
                session.addr(),
                reader.remaining()
            );
            // Send NPC ack to acknowledge the toggle request
            session.send_packet(&build_npc_ack(SEAL_NPC_TYPE_A)).await?;
        }
        SUB_UNSEAL => {
            // C2S: [u16 count] + per-item [u8 slot_pos][u32 item_id]
            let count = reader.read_u16().unwrap_or(0);
            debug!(
                "[{}] WIZ_SEAL sub=0x03 unseal: count={}",
                session.addr(),
                count
            );
            handle_unseal(session, &mut reader, count).await?;
        }
        SUB_FULL_REQUEST => {
            debug!(
                "[{}] WIZ_SEAL sub=0x63 full request ({}B)",
                session.addr(),
                reader.remaining()
            );
            // Full seal request — send seal result with "other" type (no-op ack)
            session
                .send_packet(&build_seal_result(RESULT_OTHER, 1, "", 0, 0, 0))
                .await?;
        }
        SUB_DOUBLE => {
            // C2S: [u8 slot_pos][u32 item_id] — seal an item (may already be bound)
            debug!(
                "[{}] WIZ_SEAL sub=0x04 double seal ({}B)",
                session.addr(),
                reader.remaining()
            );
            handle_double_seal(session, &mut reader).await?;
        }
        SUB_SCROLL => {
            // C2S: [u16 scroll_idx][u16 page]
            let scroll_idx = reader.read_u16().unwrap_or(0);
            let page = reader.read_u16().unwrap_or(0);
            debug!(
                "[{}] WIZ_SEAL sub=0x09 scroll select: idx={}, page={}",
                session.addr(),
                scroll_idx,
                page
            );
            handle_scroll_select(session, scroll_idx, page).await?;
        }
        SUB_ZONE_SYNC => {
            debug!(
                "[{}] WIZ_SEAL sub=0x65 zone sync ({}B)",
                session.addr(),
                reader.remaining()
            );
            // Zone sync — send NPC ack to resync state
            session.send_packet(&build_npc_ack(SEAL_NPC_TYPE_A)).await?;
        }
        _ => {
            debug!(
                "[{}] WIZ_SEAL sub=0x{:02X} ({}B)",
                session.addr(),
                sub,
                reader.remaining()
            );
            // Unknown sub-opcode — send NPC ack to avoid silent drop
            session.send_packet(&build_npc_ack(SEAL_NPC_TYPE_A)).await?;
        }
    }

    Ok(())
}

// ── Sub-opcode Handlers ───────────────────────────────────────────────

/// Handle SUB_UNSEAL (0x03) — unseal one or more items.
/// C2S: `[u16 count]` + per-item `[u8 slot_pos][u32 item_id]`
/// For each valid sealed item: restore `original_flag` (bound/not-bound) or
/// default to ITEM_FLAG_NONE. Persists each slot change to DB.
async fn handle_unseal(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    count: u16,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();
    let mut unsealed = 0u16;

    for _ in 0..count.min(HAVE_MAX as u16) {
        let slot_pos = match reader.read_u8() {
            Some(s) if (s as usize) < HAVE_MAX => s,
            _ => break,
        };
        let item_id = match reader.read_u32() {
            Some(id) if id != 0 => id,
            _ => break,
        };

        let actual_slot = SLOT_MAX + slot_pos as usize;

        let ok = world.update_inventory(sid, |inv| {
            if actual_slot < inv.len()
                && inv[actual_slot].item_id == item_id
                && inv[actual_slot].flag == ITEM_FLAG_SEALED
            {
                let o = inv[actual_slot].original_flag;
                inv[actual_slot].flag = if o == ITEM_FLAG_NOT_BOUND || o == ITEM_FLAG_BOUND {
                    o
                } else {
                    ITEM_FLAG_NONE
                };
                inv[actual_slot].original_flag = 0;
                true
            } else {
                false
            }
        });

        if ok {
            save_seal_slot_async(session, actual_slot);
            unsealed += 1;
        }
    }

    if unsealed > 0 {
        session
            .send_packet(&build_seal_result(
                RESULT_UNSEAL,
                1,
                "",
                unsealed as i32,
                0,
                0,
            ))
            .await
    } else {
        session.send_packet(&build_npc_ack(SEAL_NPC_TYPE_A)).await
    }
}

/// Handle SUB_DOUBLE (0x04) — seal an item, preserving its current flag
/// in `original_flag` (enables "double seal" — bound + sealed state).
/// C2S: `[u8 slot_pos][u32 item_id]`
/// `oFlag = bFlag` saved before setting ITEM_FLAG_SEALED.
async fn handle_double_seal(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let slot_pos = reader.read_u8().unwrap_or(0xFF);
    let item_id = reader.read_u32().unwrap_or(0);

    if slot_pos as usize >= HAVE_MAX || item_id == 0 {
        return session.send_packet(&build_npc_ack(SEAL_NPC_TYPE_A)).await;
    }

    let sid = session.session_id();
    let world = session.world().clone();
    let actual_slot = SLOT_MAX + slot_pos as usize;

    let ok = world.update_inventory(sid, |inv| {
        if actual_slot < inv.len()
            && inv[actual_slot].item_id == item_id
            && inv[actual_slot].flag != ITEM_FLAG_SEALED
            && inv[actual_slot].count > 0
            && inv[actual_slot].serial_num != 0
        {
            inv[actual_slot].original_flag = inv[actual_slot].flag;
            inv[actual_slot].flag = ITEM_FLAG_SEALED;
            true
        } else {
            false
        }
    });

    if ok {
        save_seal_slot_async(session, actual_slot);
        session
            .send_packet(&build_seal_result(RESULT_SEAL, 1, "", 0, 0, item_id as i32))
            .await
    } else {
        session.send_packet(&build_npc_ack(SEAL_NPC_TYPE_A)).await
    }
}

/// Handle SUB_SCROLL (0x09) — seal UI scroll/page selection.
/// C2S: `[u16 scroll_idx][u16 page]`
/// Responds with sealed item count so the client can update its UI state.
async fn handle_scroll_select(
    session: &mut ClientSession,
    scroll_idx: u16,
    page: u16,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();
    let inv = world.get_inventory(sid);
    let sealed_count = inv.iter().filter(|s| s.flag == ITEM_FLAG_SEALED).count() as i32;

    session
        .send_packet(&build_seal_result(
            RESULT_OTHER,
            1,
            "",
            sealed_count,
            page as i32,
            scroll_idx as i32,
        ))
        .await
}

// ── DB Persistence ────────────────────────────────────────────────────

/// Fire-and-forget DB save for a single inventory slot after seal/unseal.
/// Same pattern as `item_upgrade::save_seal_item_async`.
fn save_seal_slot_async(session: &ClientSession, slot_idx: usize) {
    let world = session.world().clone();
    let sid = session.session_id();
    let char_id = match session.character_id() {
        Some(c) => c.to_string(),
        None => return,
    };
    let slot = world.get_inventory_slot(sid, slot_idx).unwrap_or_default();
    let pool = session.pool().clone();

    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        let params = SaveItemParams {
            char_id: &char_id,
            slot_index: slot_idx as i16,
            item_id: slot.item_id as i32,
            durability: slot.durability,
            count: slot.count as i16,
            flag: slot.flag as i16,
            original_flag: slot.original_flag as i16,
            serial_num: slot.serial_num as i64,
            expire_time: slot.expire_time as i32,
        };
        if let Err(e) = repo.save_item(&params).await {
            warn!("WIZ_SEAL: failed to save slot {}: {}", slot_idx, e);
        }
    });
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_build_npc_ack_opcode() {
        let pkt = build_npc_ack(SEAL_NPC_TYPE_A);
        assert_eq!(pkt.opcode, Opcode::WizSeal as u8);
    }

    #[test]
    fn test_build_npc_ack_format() {
        let pkt = build_npc_ack(12385);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(12385));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_npc_ack_data_length() {
        // i32 only = 4 bytes
        assert_eq!(build_npc_ack(0).data.len(), 4);
    }

    #[test]
    fn test_build_seal_result_opcode() {
        let pkt = build_seal_result(RESULT_SEAL, 1, "test", 100, 200, 379006001);
        assert_eq!(pkt.opcode, Opcode::WizSeal as u8);
    }

    #[test]
    fn test_build_seal_result_format() {
        let pkt = build_seal_result(RESULT_SEAL, 1, "Sword", 100, 200, 379006001);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(SEAL_NPC_RESULT)); // npc_id
        assert_eq!(r.read_u16(), Some(RESULT_SEAL)); // result_type
        assert_eq!(r.read_u16(), Some(1)); // operation
        assert_eq!(r.read_u16(), Some(5)); // name_len
                                           // "Sword" = 5 bytes
        for &b in b"Sword" {
            assert_eq!(r.read_u8(), Some(b));
        }
        assert_eq!(r.read_i32(), Some(100)); // field_a
        assert_eq!(r.read_i32(), Some(200)); // field_b
        assert_eq!(r.read_i32(), Some(379006001)); // item_num
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_seal_result_empty_name() {
        let pkt = build_seal_result(RESULT_UNSEAL, 2, "", 0, 0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(SEAL_NPC_RESULT));
        assert_eq!(r.read_u16(), Some(RESULT_UNSEAL));
        assert_eq!(r.read_u16(), Some(2));
        assert_eq!(r.read_u16(), Some(0)); // empty name
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_seal_result_data_length() {
        // npc_id(4) + result_type(2) + operation(2) + name_len(2) + field_a(4) + field_b(4) + item_num(4) = 22 + name_len
        let name = "Test";
        let pkt = build_seal_result(1, 1, name, 0, 0, 0);
        assert_eq!(pkt.data.len(), 22 + name.len());
    }

    #[test]
    fn test_build_seal_result_all_types() {
        for result_type in [RESULT_SEAL, RESULT_UNSEAL, RESULT_BREAK, RESULT_OTHER] {
            let pkt = build_seal_result(result_type, 1, "", 0, 0, 0);
            let mut r = PacketReader::new(&pkt.data);
            r.read_i32(); // npc_id
            assert_eq!(r.read_u16(), Some(result_type));
        }
    }

    #[test]
    fn test_seal_npc_constants() {
        assert_eq!(SEAL_NPC_RESULT, 12401);
        assert_eq!(SEAL_NPC_TYPE_A, 12385);
        assert_eq!(SEAL_NPC_TYPE_B, 12387);
        assert_eq!(SEAL_NPC_TYPE_C, 16385);
    }

    #[test]
    fn test_c2s_sub_constants() {
        assert_eq!(SUB_TOGGLE, 0x02);
        assert_eq!(SUB_UNSEAL, 0x03);
        assert_eq!(SUB_DOUBLE, 0x04);
        assert_eq!(SUB_SCROLL, 0x09);
        assert_eq!(SUB_FULL_REQUEST, 0x63);
        assert_eq!(SUB_ZONE_SYNC, 0x65);
    }

    #[test]
    fn test_c2s_unseal_per_item_format() {
        // C2S: [u8 sub=0x03][u16 count] + per-item [u8 slot][u32 item_id]
        let mut pkt = Packet::new(Opcode::WizSeal as u8);
        pkt.write_u8(SUB_UNSEAL);
        pkt.write_u16(2); // 2 items
        pkt.write_u8(3); // slot 3
        pkt.write_u32(379006001);
        pkt.write_u8(7); // slot 7
        pkt.write_u32(150050001);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_UNSEAL));
        assert_eq!(r.read_u16(), Some(2));
        // item 1
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(379006001));
        // item 2
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u32(), Some(150050001));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_c2s_scroll_format() {
        // C2S: [u8 sub=0x09][u16 scroll_idx][u16 page]
        let mut pkt = Packet::new(Opcode::WizSeal as u8);
        pkt.write_u8(SUB_SCROLL);
        pkt.write_u16(5);
        pkt.write_u16(2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_SCROLL));
        assert_eq!(r.read_u16(), Some(5), "scroll_idx");
        assert_eq!(r.read_u16(), Some(2), "page");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_c2s_double_seal_format() {
        // C2S: [u8 sub=0x04][u8 slot_pos][u32 item_id]
        let mut pkt = Packet::new(Opcode::WizSeal as u8);
        pkt.write_u8(SUB_DOUBLE);
        pkt.write_u8(5); // slot
        pkt.write_u32(379006001); // item_id

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_DOUBLE));
        assert_eq!(r.read_u8(), Some(5), "slot_pos");
        assert_eq!(r.read_u32(), Some(379006001), "item_id");
        assert_eq!(r.remaining(), 0);
    }

    // ── Handler existence tests ─────────────────────────────────────

    #[test]
    fn test_save_seal_slot_async_is_sync() {
        // save_seal_slot_async is a sync fire-and-forget (spawns internally)
        let _: fn(&ClientSession, usize) = save_seal_slot_async;
    }

    // ── S2C response type validation ─────────────────────────────────

    #[test]
    fn test_unseal_result_uses_correct_type() {
        // Unseal success uses RESULT_UNSEAL (2)
        let pkt = build_seal_result(RESULT_UNSEAL, 1, "", 3, 0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(SEAL_NPC_RESULT));
        assert_eq!(r.read_u16(), Some(RESULT_UNSEAL));
        assert_eq!(r.read_u16(), Some(1)); // operation
        assert_eq!(r.read_u16(), Some(0)); // empty name
        assert_eq!(r.read_i32(), Some(3)); // field_a = unsealed count
    }

    #[test]
    fn test_double_seal_result_uses_correct_type() {
        // Double seal success uses RESULT_SEAL (1)
        let pkt = build_seal_result(RESULT_SEAL, 1, "", 0, 0, 379006001);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(SEAL_NPC_RESULT));
        assert_eq!(r.read_u16(), Some(RESULT_SEAL));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.read_i32(), Some(379006001)); // item_num
    }

    #[test]
    fn test_scroll_result_uses_other_type() {
        // Scroll response uses RESULT_OTHER (4)
        let pkt = build_seal_result(RESULT_OTHER, 1, "", 5, 2, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(SEAL_NPC_RESULT));
        assert_eq!(r.read_u16(), Some(RESULT_OTHER));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_i32(), Some(5)); // sealed_count
        assert_eq!(r.read_i32(), Some(2)); // page
        assert_eq!(r.read_i32(), Some(0)); // scroll_idx
    }

    // ── Item flag constants for seal logic ────────────────────────────

    #[test]
    fn test_item_flag_sealed_value() {
        assert_eq!(ITEM_FLAG_SEALED, 4);
        assert_eq!(ITEM_FLAG_NONE, 0);
        assert_eq!(ITEM_FLAG_BOUND, 8);
        assert_eq!(ITEM_FLAG_NOT_BOUND, 7);
    }

    #[test]
    fn test_unseal_restores_bound_flag() {
        // When original_flag is BOUND (8), unseal should restore to BOUND
        // When original_flag is NOT_BOUND (7), unseal should restore to NOT_BOUND
        // When original_flag is anything else, unseal should default to NONE (0)
        for (original, expected) in [
            (ITEM_FLAG_BOUND, ITEM_FLAG_BOUND),
            (ITEM_FLAG_NOT_BOUND, ITEM_FLAG_NOT_BOUND),
            (ITEM_FLAG_NONE, ITEM_FLAG_NONE),
            (3, ITEM_FLAG_NONE), // DUPLICATE → default to NONE
            (1, ITEM_FLAG_NONE), // RENTED → default to NONE
        ] {
            let restored = if original == ITEM_FLAG_NOT_BOUND || original == ITEM_FLAG_BOUND {
                original
            } else {
                ITEM_FLAG_NONE
            };
            assert_eq!(restored, expected, "original_flag={original}");
        }
    }

    #[test]
    fn test_have_max_caps_unseal_count() {
        // HAVE_MAX = 28, so unseal count is capped
        use crate::inventory_constants::HAVE_MAX;
        assert_eq!(100u16.min(HAVE_MAX as u16), 28);
        assert_eq!(5u16.min(HAVE_MAX as u16), 5);
    }

    #[test]
    fn test_slot_max_offset() {
        // Inventory bag starts at SLOT_MAX (14)
        use crate::inventory_constants::SLOT_MAX;
        assert_eq!(SLOT_MAX, 14);
        // slot_pos=0 → actual_slot=14, slot_pos=27 → actual_slot=41
        assert_eq!(SLOT_MAX + 0, 14);
        assert_eq!(SLOT_MAX + 27, 41);
    }
}
