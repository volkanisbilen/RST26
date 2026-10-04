//! PUS Refund / Item Return handler.
//! `PusRefundPurchase()`, `ItemReturnSendErrorOpcode()`
//! ## Overview
//! After purchasing items from the cash shop, players have a 1-hour window
//! to return unused items for a full refund. The refund map is loaded from
//! DB on game entry and maintained in-memory per session.
//! ## Wire Format
//! All packets use `WIZ_EXT_HOOK (0xE9)` with sub-opcode `PusRefund = 0xD4`.
//! **Sub-opcodes (`enum class pusrefunopcode`):**
//! ```text
//! ireturn          = 0  — client requests item return
//! listsend         = 1  — server sends refund list on game entry
//! itemnotfound     = 2  — error: item not found
//! timeexpired      = 3  — error: refund time expired
//! procestime       = 4  — error: rate limited (5s cooldown)
//! notinventory     = 5  — error: item not in inventory
//! itemused         = 6  — error: item has been used/modified
//! itemreurnsucces  = 7  — item return succeeded
//! listadd          = 8  — add entry to client's refund list
//! ```

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::handler::knight_cash;
use crate::session::ClientSession;
use crate::zone::SessionId;

pub(crate) use super::ext_hook::EXT_SUB_PUS_REFUND;

/// Rate limit between refund attempts (seconds).
const REFUND_COOLDOWN_SECS: u64 = 5;

/// Refund window after purchase (seconds) — 1 hour.
const REFUND_WINDOW_SECS: u64 = 3600;

// ── pusrefunopcode sub-opcodes ──────────────────────────────────────────────

mod subop {
    /// Client requests item return.
    pub const IRETURN: u8 = 0;
    /// Server sends refund list (game entry).
    pub const LISTSEND: u8 = 1;
    /// Error: item not found in refund map.
    pub const ITEMNOTFOUND: u8 = 2;
    /// Error: refund time expired.
    pub const TIMEEXPIRED: u8 = 3;
    /// Error: rate limited (5s cooldown).
    pub const PROCESTIME: u8 = 4;
    /// Error: item not in inventory.
    pub const NOTINVENTORY: u8 = 5;
    /// Error: item has been used or modified.
    pub const ITEMUSED: u8 = 6;
    /// Item return succeeded.
    pub const ITEMRETURNSUCCESS: u8 = 7;
    /// Add entry to client's refund list.
    pub const LISTADD: u8 = 8;
}

/// In-memory refund record for a session.
#[derive(Debug, Clone)]
pub struct PusRefundEntry {
    /// Game item ID.
    pub item_id: u32,
    /// Price paid at time of purchase.
    pub item_price: u32,
    /// Quantity purchased.
    pub item_count: u16,
    /// Item duration (days, 0=permanent).
    pub item_duration: u16,
    /// Unix timestamp when the refund window expires.
    pub expired_time: u64,
    /// Purchase type: 0=KC, 1=TL.
    pub buy_type: u8,
}

// ─────────────────────────────────────────────────────────────────────────────
// Packet Builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build an error response packet for a refund error.
fn build_error_packet(error_code: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PUS_REFUND);
    pkt.write_u8(error_code);
    pkt
}

/// Build the refund list packet sent on game entry.
/// Wire: `[0xE9][0xD4][u8=1 (listsend)][u16 count]([u64 serial][u32 item_id][u32 price][u32 expiry])×N`
fn build_refund_list_packet(entries: &[(u64, &PusRefundEntry)]) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PUS_REFUND);
    pkt.write_u8(subop::LISTSEND);
    pkt.write_u16(entries.len() as u16);
    for (serial, entry) in entries {
        pkt.write_u64(*serial);
        pkt.write_u32(entry.item_id);
        pkt.write_u32(entry.item_price);
        pkt.write_u32(entry.expired_time as u32);
    }
    pkt
}

/// Build a "list add" packet to inform client of a new refundable purchase.
/// Wire: `[sub][u8=8][u64 serial][u32 itemid][u16 count][u32 price][u16 duration][u32 buytime][u8 buytype]`
fn build_listadd_packet(serial: u64, entry: &PusRefundEntry) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PUS_REFUND);
    pkt.write_u8(subop::LISTADD);
    pkt.write_u64(serial);
    pkt.write_u32(entry.item_id);
    pkt.write_u16(entry.item_count);
    pkt.write_u32(entry.item_price);
    pkt.write_u16(entry.item_duration);
    pkt.write_u32(entry.expired_time as u32);
    pkt.write_u8(entry.buy_type);
    pkt
}

/// Build the success packet after a successful item return.
/// Wire: `[sub][u8=7][u64 serial][u32 item_id]`
fn build_success_packet(serial: u64, item_id: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PUS_REFUND);
    pkt.write_u8(subop::ITEMRETURNSUCCESS);
    pkt.write_u64(serial);
    pkt.write_u32(item_id);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the current UNIX timestamp in seconds.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─────────────────────────────────────────────────────────────────────────────
// Load & Send Refund List (game entry)
// ─────────────────────────────────────────────────────────────────────────────

/// Load refund records from DB and store in session state, then send list to client.
/// Called during game entry (after character selection).
pub async fn load_and_send_refund_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let account_id = match session.account_id() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Ok(()),
    };

    let pool = session.pool().clone();
    let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&pool);

    let rows = match repo.load_account_purchases(&account_id).await {
        Ok(r) => r,
        Err(e) => {
            warn!(
                "[{}] Failed to load PUS refund for {}: {}",
                session.addr(),
                account_id,
                e
            );
            return Ok(());
        }
    };

    if rows.is_empty() {
        return Ok(());
    }

    let now = unix_now();
    let mut refund_map: HashMap<u64, PusRefundEntry> = HashMap::new();
    let mut valid_entries: Vec<(u64, PusRefundEntry)> = Vec::new();

    for row in &rows {
        let serial = row.mserial as u64;
        let buying_time = row.buying_time as u64;
        let expired_time = buying_time + REFUND_WINDOW_SECS;

        // Skip expired entries
        if now > expired_time {
            continue;
        }

        let entry = PusRefundEntry {
            item_id: row.item_id as u32,
            item_price: row.item_price as u32,
            item_count: row.item_count as u16,
            item_duration: row.item_duration as u16,
            expired_time,
            buy_type: row.buy_type as u8,
        };

        refund_map.insert(serial, entry.clone());
        valid_entries.push((serial, entry));
    }

    // Store in session state
    let sid = session.session_id();
    let world = session.world().clone();
    world.update_session(sid, |h| {
        h.pus_refund_map = refund_map;
    });

    // Send list to client
    if !valid_entries.is_empty() {
        let list_refs: Vec<(u64, &PusRefundEntry)> =
            valid_entries.iter().map(|(s, e)| (*s, e)).collect();
        let pkt = build_refund_list_packet(&list_refs);
        session.send_packet(&pkt).await?;
        debug!(
            "[{}] PusRefundSendList: {} entries",
            session.addr(),
            valid_entries.len()
        );
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handle Client Request
// ─────────────────────────────────────────────────────────────────────────────

/// Handle `WIZ_EXT_HOOK (0xE9)` sub-opcode `PusRefund (0xD4)` from client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0xFF);

    match sub {
        subop::IRETURN => handle_item_return(session, &mut reader).await,
        _ => {
            debug!(
                "[{}] PusRefund: unhandled sub-opcode {}",
                session.addr(),
                sub
            );
            Ok(())
        }
    }
}

/// Process an item return request.
async fn handle_item_return(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let serial = reader.read_u64().unwrap_or(0);
    let _slot_id = reader.read_u8().unwrap_or(0);

    let sid = session.session_id();
    let world = session.world().clone();
    let now = unix_now();

    // Rate limit check (5s cooldown)
    let last_refund_time = world
        .with_session(sid, |h| h.pus_refund_last_time)
        .unwrap_or(0);
    if last_refund_time + REFUND_COOLDOWN_SECS > now {
        let pkt = build_error_packet(subop::PROCESTIME);
        session.send_packet(&pkt).await?;
        return Ok(());
    }

    // Update cooldown timer
    world.update_session(sid, |h| {
        h.pus_refund_last_time = now;
    });

    // Look up serial in refund map
    let entry = world.with_session(sid, |h| h.pus_refund_map.get(&serial).cloned());
    let entry = match entry {
        Some(Some(e)) => e,
        _ => {
            let pkt = build_error_packet(subop::ITEMNOTFOUND);
            session.send_packet(&pkt).await?;
            return Ok(());
        }
    };

    // Check expiry
    if now > entry.expired_time {
        let pkt = build_error_packet(subop::TIMEEXPIRED);
        session.send_packet(&pkt).await?;
        return Ok(());
    }

    // Find item in inventory by matching item_id
    let item_id = entry.item_id;
    let found_slot = world.with_session(sid, |h| {
        // Search bag slots (SLOT_MAX to SLOT_MAX+HAVE_MAX)
        for (i, slot) in h.inventory.iter().enumerate() {
            if slot.item_id == item_id && slot.count > 0 {
                return Some(i);
            }
        }
        None
    });

    let slot_idx = match found_slot {
        Some(Some(idx)) => idx,
        _ => {
            let pkt = build_error_packet(subop::NOTINVENTORY);
            session.send_packet(&pkt).await?;
            return Ok(());
        }
    };

    // Check item count matches (item hasn't been partially used)
    let count_matches = world
        .with_session(sid, |h| {
            if let Some(slot) = h.inventory.get(slot_idx) {
                slot.count == entry.item_count
            } else {
                false
            }
        })
        .unwrap_or(false);

    if !count_matches {
        let pkt = build_error_packet(subop::ITEMUSED);
        session.send_packet(&pkt).await?;
        return Ok(());
    }

    // Clear item from inventory and send stack change to sync client UI
    world.update_session(sid, |h| {
        if let Some(slot) = h.inventory.get_mut(slot_idx) {
            slot.item_id = 0;
            slot.count = 0;
            slot.durability = 0;
        }
    });

    // SendStackChange — notify client to update inventory slot
    {
        let mut sc_pkt = Packet::new(Opcode::WizItemCountChange as u8);
        sc_pkt.write_u16(1); // count_type
        sc_pkt.write_u8(1); // slot_section = 1 (inventory)
        sc_pkt.write_u8(slot_idx as u8); // pos relative to SLOT_MAX
        sc_pkt.write_u32(0); // item_id = 0 (cleared)
        sc_pkt.write_u32(0); // count = 0
        sc_pkt.write_u8(0); // bNewItem = false
        sc_pkt.write_u16(0); // durability = 0
        sc_pkt.write_u32(0); // reserved
        sc_pkt.write_u32(0); // expire_time = 0
        sc_pkt.write_u16(0); // v2625 SendStackChange trailing field
        session.send_packet(&sc_pkt).await?;
    }

    // Refund the price: KC if buy_type=0, TL if buy_type=1
    let kc_amount = if entry.buy_type == 0 {
        entry.item_price as i32
    } else {
        0
    };
    let tl_amount = if entry.buy_type == 1 {
        entry.item_price as i32
    } else {
        0
    };
    knight_cash::give_balance(session, kc_amount, tl_amount).await?;

    // Remove from in-memory refund map
    world.update_session(sid, |h| {
        h.pus_refund_map.remove(&serial);
    });

    // Delete from DB
    let pool = session.pool().clone();
    let account_id = session.account_id().unwrap_or("").to_string();
    tokio::spawn(async move {
        let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&pool);
        if let Err(e) = repo.delete_purchase(&account_id, serial as i64).await {
            warn!("Failed to delete PUS refund (serial={}): {}", serial, e);
        }
    });

    // Send success to client
    let pkt = build_success_packet(serial, item_id);
    session.send_packet(&pkt).await?;

    debug!(
        "[{}] PusRefund: returned item_id={} serial={} refund=KC:{}/TL:{}",
        session.addr(),
        item_id,
        serial,
        kc_amount,
        tl_amount
    );
    Ok(())
}

/// Register a new refundable purchase (called after a successful shop purchase).
pub fn register_purchase(
    world: &crate::world::WorldState,
    sid: SessionId,
    serial: u64,
    entry: PusRefundEntry,
) {
    // Send listadd to client
    let pkt = build_listadd_packet(serial, &entry);
    world.send_to_session_owned(sid, pkt);

    // Store in session
    world.update_session(sid, |h| {
        h.pus_refund_map.insert(serial, entry);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_error_packet() {
        let pkt = build_error_packet(subop::ITEMNOTFOUND);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        assert_eq!(pkt.data[0], EXT_SUB_PUS_REFUND);
        assert_eq!(pkt.data[1], subop::ITEMNOTFOUND);
    }

    #[test]
    fn test_build_success_packet() {
        let pkt = build_success_packet(123456, 900001);
        assert_eq!(pkt.data[0], EXT_SUB_PUS_REFUND);
        assert_eq!(pkt.data[1], subop::ITEMRETURNSUCCESS);
        let serial = u64::from_le_bytes([
            pkt.data[2],
            pkt.data[3],
            pkt.data[4],
            pkt.data[5],
            pkt.data[6],
            pkt.data[7],
            pkt.data[8],
            pkt.data[9],
        ]);
        assert_eq!(serial, 123456);
        // C++ parity: item_id follows serial
        let item_id = u32::from_le_bytes([pkt.data[10], pkt.data[11], pkt.data[12], pkt.data[13]]);
        assert_eq!(item_id, 900001);
    }

    #[test]
    fn test_build_refund_list_empty() {
        let pkt = build_refund_list_packet(&[]);
        assert_eq!(pkt.data[0], EXT_SUB_PUS_REFUND);
        assert_eq!(pkt.data[1], subop::LISTSEND);
        let count = u16::from_le_bytes([pkt.data[2], pkt.data[3]]);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_build_refund_list_one_entry() {
        let entry = PusRefundEntry {
            item_id: 100001,
            item_price: 350,
            item_count: 1,
            item_duration: 0,
            expired_time: 1700000000,
            buy_type: 0,
        };
        let pkt = build_refund_list_packet(&[(999, &entry)]);
        assert_eq!(pkt.data[1], subop::LISTSEND);
        let count = u16::from_le_bytes([pkt.data[2], pkt.data[3]]);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_build_listadd_packet() {
        let entry = PusRefundEntry {
            item_id: 200001,
            item_price: 500,
            item_count: 3,
            item_duration: 7,
            expired_time: 1700003600,
            buy_type: 1,
        };
        let pkt = build_listadd_packet(42, &entry);
        assert_eq!(pkt.data[0], EXT_SUB_PUS_REFUND);
        assert_eq!(pkt.data[1], subop::LISTADD);
        // Verify buy_type is the last byte (C++ parity)
        let last_byte = *pkt.data.last().unwrap();
        assert_eq!(last_byte, 1); // buy_type = TL
    }

    #[test]
    fn test_refund_cooldown() {
        assert_eq!(REFUND_COOLDOWN_SECS, 5);
    }

    #[test]
    fn test_refund_window() {
        assert_eq!(REFUND_WINDOW_SECS, 3600);
    }

    #[test]
    fn test_subop_values() {
        assert_eq!(subop::IRETURN, 0);
        assert_eq!(subop::LISTSEND, 1);
        assert_eq!(subop::ITEMNOTFOUND, 2);
        assert_eq!(subop::TIMEEXPIRED, 3);
        assert_eq!(subop::PROCESTIME, 4);
        assert_eq!(subop::NOTINVENTORY, 5);
        assert_eq!(subop::ITEMUSED, 6);
        assert_eq!(subop::ITEMRETURNSUCCESS, 7);
        assert_eq!(subop::LISTADD, 8);
    }

    // ── Sprint 924: Additional coverage ──────────────────────────────

    /// Error packet is always sub(1) + error_code(1) = 2 bytes.
    #[test]
    fn test_error_packet_data_length() {
        for code in [
            subop::ITEMNOTFOUND,
            subop::TIMEEXPIRED,
            subop::PROCESTIME,
            subop::NOTINVENTORY,
            subop::ITEMUSED,
        ] {
            let pkt = build_error_packet(code);
            assert_eq!(pkt.data.len(), 2);
        }
    }

    /// Success packet: sub(1) + success(1) + serial(8) + item_id(4) = 14 bytes.
    #[test]
    fn test_success_packet_data_length() {
        let pkt = build_success_packet(999, 800001);
        assert_eq!(pkt.data.len(), 14);
    }

    /// Refund list: header(4) + N * entry(20).
    /// Entry = serial(8) + item_id(4) + price(4) + expiry(4) = 20 bytes.
    #[test]
    fn test_refund_list_entry_size() {
        let e1 = PusRefundEntry {
            item_id: 100,
            item_price: 200,
            item_count: 1,
            item_duration: 0,
            expired_time: 1700000000,
            buy_type: 0,
        };
        let e2 = PusRefundEntry {
            item_id: 300,
            item_price: 400,
            item_count: 2,
            item_duration: 7,
            expired_time: 1700003600,
            buy_type: 1,
        };
        let pkt = build_refund_list_packet(&[(1, &e1), (2, &e2)]);
        // header: sub(1) + listsend(1) + count(2) = 4, entries: 2*20 = 40
        assert_eq!(pkt.data.len(), 44);
    }

    /// Listadd packet: sub(1)+listadd(1)+serial(8)+item_id(4)+count(2)+price(4)+duration(2)+expiry(4)+buy_type(1) = 27.
    #[test]
    fn test_listadd_packet_data_length() {
        let entry = PusRefundEntry {
            item_id: 500,
            item_price: 350,
            item_count: 1,
            item_duration: 0,
            expired_time: 1700000000,
            buy_type: 0,
        };
        let pkt = build_listadd_packet(12345, &entry);
        assert_eq!(pkt.data.len(), 27);
    }

    /// Item return C2S format: [u8 sub=0][u64 serial][u8 slot_id].
    #[test]
    fn test_ireturn_c2s_format() {
        use ko_protocol::PacketReader;
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_PUS_REFUND);
        pkt.write_u8(subop::IRETURN);
        pkt.write_u64(9876543210);
        pkt.write_u8(5); // slot_id

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_PUS_REFUND));
        assert_eq!(r.read_u8(), Some(0)); // IRETURN
        assert_eq!(r.read_u64(), Some(9876543210));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.remaining(), 0);
    }
}
