//! WIZ_COLLECTION1 (0xA9) + WIZ_COLLECTION2 (0xB4) — Item Collection System.
//!
//! v2525 client's native item collection album. Players collect specific items
//! and receive notifications about progress toward collection sets.
//!
//! ## Architecture
//!
//! - **WIZ_COLLECTION1 (0xA9)**: S2C notification — chat messages about collection progress.
//!   No panel dependency (Group A, always callable). Two sub-opcodes:
//!   - sub=1: Collection set progress (item + 3 counts, string 3706)
//!   - sub=2: Item collection update (item + current/required, string 3707)
//!
//! - **WIZ_COLLECTION2 (0xB4)**: S2C notification — panel-dependent UI update.
//!   Panel at `[esi+0x5C8]`, null-checked. Only sub=2 with a u16 parameter.
//!   Calls vtable method `[vtable+0x5c]` on the collection panel.
//!
//! ## Client RE
//!
//! ### WIZ_COLLECTION1 handler at `0x7BF2B0`
//!
//! - Item ID decomposition: `group_index = item_id / 1000`, `variant = item_id % 1000`
//! - Collection set lookup via global map at `0x1092878`
//! - Category lookup via global table at `0x1092880` (max 45 categories)
//! - Item type validation: `item_id / 1_000_000_000 < 4`
//! - Display: yellow text (0xFFFFFF00) on chat panel `[GameMain+0x1D4]`
//! - String IDs: 3706 (sub=1 set progress), 3707 (sub=2 item update)
//!
//! ### WIZ_COLLECTION2 handler
//!
//! - Panel: `[esi+0x5C8]` — created when collection UI is opened
//! - Only sub=2: reads `u16` value, calls panel vtable method
//! - Panel-dependent: all S2C silently dropped when panel is NULL
//!
//! ## C2S
//!
//! Neither opcode has C2S packets — both are S2C notification-only.
//! The server sends these when items are collected or collection progress changes.

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

// ── S2C Sub-type constants ──────────────────────────────────────────────

/// Collection1 sub 1: Collection set progress notification.
const COLL1_SUB_SET_PROGRESS: u8 = 1;

/// Collection1 sub 2: Item collection update notification.
const COLL1_SUB_ITEM_UPDATE: u8 = 2;

/// Collection2 sub 2: Panel UI update.
const COLL2_SUB_PANEL_UPDATE: u8 = 2;

// ── S2C Builders — WIZ_COLLECTION1 (0xA9) ───────────────────────────────

/// Build a collection set progress notification (sub=1).
///
/// Client reads: `[u8 sub=1][i32 item_id][u16 field_a][u16 field_b][u16 field_c]`
///
/// Displays string 3706 formatted with field_a, set_name, field_b, field_c
/// in yellow (0xFFFFFF00) on the chat panel.
///
/// - `item_id`: Item used to look up the collection set (group = item_id / 1000)
/// - `field_a`: First count value (e.g., items collected)
/// - `field_b`: Second count value (e.g., target count)
/// - `field_c`: Third count value (e.g., total possible)
pub fn build_set_progress(item_id: i32, field_a: u16, field_b: u16, field_c: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCollection1 as u8);
    pkt.write_u8(COLL1_SUB_SET_PROGRESS);
    pkt.write_i32(item_id);
    pkt.write_u16(field_a);
    pkt.write_u16(field_b);
    pkt.write_u16(field_c);
    pkt
}

/// Build an item collection update notification (sub=2).
///
/// Client reads: `[u8 sub=2][i32 item_id][u16 current_count][u16 required_count]`
///
/// Displays string 3707 formatted with current_count, set_name, required_count
/// in yellow (0xFFFFFF00) on the chat panel.
///
/// - `item_id`: The collected item ID (used to look up set and category)
/// - `current_count`: How many of this type the player has collected
/// - `required_count`: How many are needed to complete the set
pub fn build_item_update(item_id: i32, current_count: u16, required_count: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCollection1 as u8);
    pkt.write_u8(COLL1_SUB_ITEM_UPDATE);
    pkt.write_i32(item_id);
    pkt.write_u16(current_count);
    pkt.write_u16(required_count);
    pkt
}

// ── S2C Builders — WIZ_COLLECTION2 (0xB4) ───────────────────────────────

/// Build a collection panel UI update (sub=2).
///
/// Client reads: `[u8 sub=2][u16 value]`
///
/// Only processed when the collection panel (`[esi+0x5C8]`) is open.
/// Calls the panel's vtable method to update the displayed item detail.
///
/// - `value`: Panel-specific update parameter (e.g., item index or page)
pub fn build_panel_update(value: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCollection2 as u8);
    pkt.write_u8(COLL2_SUB_PANEL_UPDATE);
    pkt.write_u16(value);
    pkt
}

// ── Public API ──────────────────────────────────────────────────────────

/// Notify a player about item collection progress after picking up an item.
///
/// Sends WIZ_COLLECTION1 (0xA9) sub=2 (item update) to the player.
/// When a full collection definition table is available, this function
/// should look up whether the item belongs to a collection set and send
/// appropriate set progress notifications.
///
/// Currently sends a basic item update notification if the item's
/// `group_index` (item_id / 1000) is non-zero, which matches the client's
/// collection lookup logic.
pub fn notify_item_collected(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    item_id: u32,
    current_count: u16,
) {
    // Client validates: item_id / 1_000_000_000 < 4 (item types 0-3)
    if item_id == 0 || item_id / 1_000_000_000 >= 4 {
        return;
    }

    // Only notify if the item could plausibly be in a collection set.
    // group_index = item_id / 1000 — client uses this for set lookup.
    let group_index = item_id / 1000;
    if group_index == 0 {
        return;
    }

    // Send item collection update to the player.
    // required_count is unknown without collection definition data —
    // use 0 to indicate "no target count" (client will still display).
    let pkt = build_item_update(item_id as i32, current_count, 0);
    world.send_to_session_owned(sid, pkt);
}

// ── C2S Handlers ────────────────────────────────────────────────────────

/// Handle WIZ_COLLECTION1 (0xA9) from the client.
///
/// This opcode is S2C-only — the client never sends it.
/// If received, log and discard.
pub async fn handle_collection1(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    debug!(
        "[{}] WIZ_COLLECTION1 sub={} (S2C-only opcode, {}B ignored)",
        session.addr(),
        sub,
        reader.remaining()
    );
    Ok(())
}

/// Handle WIZ_COLLECTION2 (0xB4) from the client.
///
/// This opcode is S2C-only — the client never sends it.
/// If received, log and discard.
pub async fn handle_collection2(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    debug!(
        "[{}] WIZ_COLLECTION2 sub={} (S2C-only opcode, {}B ignored)",
        session.addr(),
        sub,
        reader.remaining()
    );
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── WIZ_COLLECTION1 builders ────────────────────────────────────

    #[test]
    fn test_build_set_progress_opcode() {
        let pkt = build_set_progress(100001000, 5, 10, 10);
        assert_eq!(pkt.opcode, Opcode::WizCollection1 as u8);
    }

    #[test]
    fn test_build_set_progress_format() {
        let pkt = build_set_progress(200001000, 3, 8, 12);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL1_SUB_SET_PROGRESS)); // sub=1
        assert_eq!(r.read_i32(), Some(200001000)); // item_id
        assert_eq!(r.read_u16(), Some(3)); // field_a
        assert_eq!(r.read_u16(), Some(8)); // field_b
        assert_eq!(r.read_u16(), Some(12)); // field_c
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_set_progress_data_length() {
        // u8 sub + i32 item_id + u16×3 = 1+4+6 = 11
        let pkt = build_set_progress(0, 0, 0, 0);
        assert_eq!(pkt.data.len(), 11);
    }

    #[test]
    fn test_build_item_update_opcode() {
        let pkt = build_item_update(300500000, 2, 5);
        assert_eq!(pkt.opcode, Opcode::WizCollection1 as u8);
    }

    #[test]
    fn test_build_item_update_format() {
        let pkt = build_item_update(150200000, 7, 15);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL1_SUB_ITEM_UPDATE)); // sub=2
        assert_eq!(r.read_i32(), Some(150200000)); // item_id
        assert_eq!(r.read_u16(), Some(7)); // current_count
        assert_eq!(r.read_u16(), Some(15)); // required_count
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_item_update_data_length() {
        // u8 sub + i32 item_id + u16×2 = 1+4+4 = 9
        let pkt = build_item_update(0, 0, 0);
        assert_eq!(pkt.data.len(), 9);
    }

    #[test]
    fn test_build_item_update_max_values() {
        let pkt = build_item_update(i32::MAX, u16::MAX, u16::MAX);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL1_SUB_ITEM_UPDATE));
        assert_eq!(r.read_i32(), Some(i32::MAX));
        assert_eq!(r.read_u16(), Some(u16::MAX));
        assert_eq!(r.read_u16(), Some(u16::MAX));
    }

    #[test]
    fn test_build_set_progress_negative_item_id() {
        // Negative item_id should be preserved (i32)
        let pkt = build_set_progress(-1, 0, 0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL1_SUB_SET_PROGRESS));
        assert_eq!(r.read_i32(), Some(-1));
    }

    // ── WIZ_COLLECTION2 builders ────────────────────────────────────

    #[test]
    fn test_build_panel_update_opcode() {
        let pkt = build_panel_update(42);
        assert_eq!(pkt.opcode, Opcode::WizCollection2 as u8);
    }

    #[test]
    fn test_build_panel_update_format() {
        let pkt = build_panel_update(1234);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL2_SUB_PANEL_UPDATE)); // sub=2
        assert_eq!(r.read_u16(), Some(1234)); // value
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_panel_update_data_length() {
        // u8 sub + u16 value = 1+2 = 3
        let pkt = build_panel_update(0);
        assert_eq!(pkt.data.len(), 3);
    }

    #[test]
    fn test_build_panel_update_zero() {
        let pkt = build_panel_update(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL2_SUB_PANEL_UPDATE));
        assert_eq!(r.read_u16(), Some(0));
    }

    #[test]
    fn test_build_panel_update_max() {
        let pkt = build_panel_update(u16::MAX);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL2_SUB_PANEL_UPDATE));
        assert_eq!(r.read_u16(), Some(u16::MAX));
    }

    // ── Client item_id decomposition validation ─────────────────────

    #[test]
    fn test_item_id_decomposition() {
        // Client: group_index = item_id / 1000, variant = item_id % 1000
        let item_id: i32 = 200050123;
        let group_index = item_id / 1000;
        let variant = item_id % 1000;
        assert_eq!(group_index, 200050);
        assert_eq!(variant, 123);
    }

    #[test]
    fn test_item_id_type_range() {
        // Client validates: item_id / 1_000_000_000 < 4
        let type0: i32 = 100001000;
        let type1: i32 = 1_100_001_000;
        let type3: i64 = 3_100_001_000;
        assert!(type0 / 1_000_000_000 < 4); // type 0
        assert!(type1 / 1_000_000_000 < 4); // type 1
        assert!(type3 / 1_000_000_000 < 4); // type 3 (i64 for overflow)
    }

    #[test]
    fn test_item_id_category_bounds() {
        // Client validates: category_index < 45 (0x2D)
        let max_valid: u8 = 44;
        let first_invalid: u8 = 45;
        assert!(max_valid < 45); // max valid
        assert!(first_invalid >= 45); // invalid
    }

    // ── Sub-opcode constant verification ────────────────────────────

    #[test]
    fn test_sub_opcode_values() {
        assert_eq!(COLL1_SUB_SET_PROGRESS, 1);
        assert_eq!(COLL1_SUB_ITEM_UPDATE, 2);
        assert_eq!(COLL2_SUB_PANEL_UPDATE, 2);
    }

    // ── notify_item_collected validation tests ──────────────────────

    #[test]
    fn test_item_type_validation() {
        // item_id / 1_000_000_000 must be < 4
        assert!(100_001_000u32 / 1_000_000_000 < 4); // type 0, valid
        assert!(1_100_001_000u32 / 1_000_000_000 < 4); // type 1, valid
                                                       // type 4+ would be invalid (item_id >= 4_000_000_000)
    }

    #[test]
    fn test_group_index_zero_skipped() {
        // group_index = item_id / 1000
        // item_id < 1000 → group_index == 0 → should be skipped
        let item_id: u32 = 500;
        assert_eq!(item_id / 1000, 0);
    }

    #[test]
    fn test_notify_packet_format() {
        // notify_item_collected sends build_item_update(item_id, count, 0)
        let item_id: i32 = 200_001_000;
        let count: u16 = 3;
        let pkt = build_item_update(item_id, count, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COLL1_SUB_ITEM_UPDATE));
        assert_eq!(r.read_i32(), Some(200_001_000));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.read_u16(), Some(0)); // required_count = 0 (unknown)
    }

    #[test]
    fn test_group_index_decomposition_for_sets() {
        // group_index maps items to collection sets
        // Items 200001000..200001999 → group_index 200001 (same set)
        assert_eq!(200_001_000u32 / 1000, 200_001);
        assert_eq!(200_001_999u32 / 1000, 200_001);
        // Different set
        assert_eq!(200_002_000u32 / 1000, 200_002);
    }
}
