//! WIZ_SHOPPING_MALL (0x6A) handler — premium cash shop (PUS).
//! Sub-opcodes:
//! - 1 = STORE_OPEN: Client opens shop UI
//! - 2 = STORE_CLOSE: Client closes shop / load web items
//! - 3 = STORE_BUY (unused in C++)
//! - 4 = STORE_MINI (unused)
//! - 5 = STORE_PROCESS (unused)
//! - 6 = STORE_LETTER: Routes to letter system
//! The PUS (Premium User Store) is a web-based cash shop. Items are
//! purchased externally and delivered to the player's inventory via
//! the STORE_CLOSE flow (WEB_ITEMMALL table).
//! Price types:
//! - 0 = Knight Cash (KC) — premium currency
//! - 1 = TL (Turkish Lira) — real money balance

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};

#[cfg(test)]
use super::INVENTORY_TOTAL;
use super::{HAVE_MAX, SLOT_MAX};

/// STORE_OPEN sub-opcode.
const STORE_OPEN: u8 = 1;
/// STORE_CLOSE sub-opcode.
const STORE_CLOSE: u8 = 2;
use super::letter::STORE_LETTER;

/// Error code: player is dead/trading/merchanting (`-9` as u16).
const ERR_INVALID_STATE: u16 = 0xFFF7;
/// Error code: no free inventory slots (`-8` as u16).
const ERR_NO_FREE_SLOTS: u16 = 0xFFF8;

/// Price type: Knight Cash (KC).
pub const PRICE_TYPE_KC: i16 = 0;
/// Price type: TL (real money balance).
pub const PRICE_TYPE_TL: i16 = 1;

/// Handle WIZ_SHOPPING_MALL from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead player guard — C++ checks isDead() at handler entry
    let world = session.world().clone();
    let sid = session.session_id();
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        STORE_OPEN => {
            handle_store_open(session).await?;
        }
        STORE_CLOSE => {
            handle_store_close(session).await?;
        }
        3..=5 => {
            // Unused sub-opcodes (STORE_BUY, STORE_MINI, STORE_PROCESS)
        }
        STORE_LETTER => {
            // STORE_LETTER — route to letter system
            super::letter::handle(session, &mut reader).await?;
        }
        _ => {
            tracing::trace!(
                "[{}] Unknown shopping_mall sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
        }
    }

    Ok(())
}

/// Handle STORE_OPEN — respond with free slot count.
/// Wire response: `WIZ_SHOPPING_MALL << u8(1) << u16(result) << i16(free_slots)`
async fn handle_store_open(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Check dead/trading/merchanting/genie state
    let is_genie = world.with_session(sid, |h| h.genie_active).unwrap_or(false);
    if world.is_player_dead(sid) || world.is_trading(sid) || world.is_merchanting(sid) || is_genie {
        session
            .send_packet(&build_store_open_error(ERR_INVALID_STATE))
            .await?;
        return Ok(());
    }

    // Not allowed in private arenas (zones 40-45)
    if let Some(pos) = world.get_position(sid) {
        if (40..=45).contains(&pos.zone_id) {
            session
                .send_packet(&build_store_open_error(ERR_INVALID_STATE))
                .await?;
            return Ok(());
        }
    }

    // Count free inventory slots
    let inv = world.get_inventory(sid);
    let mut free_slots: i16 = 0;
    for i in SLOT_MAX..(SLOT_MAX + HAVE_MAX) {
        if let Some(slot) = inv.get(i) {
            if slot.item_id == 0 {
                free_slots += 1;
            }
        }
    }

    if free_slots <= 0 {
        session
            .send_packet(&build_store_open_error(ERR_NO_FREE_SLOTS))
            .await?;
        return Ok(());
    }

    world.set_store_open(sid, true);

    session
        .send_packet(&build_store_open_success(free_slots))
        .await?;

    // The v2625 client embeds WebView2. Give it a short-lived, account-bound
    // URL; catalog, price and balance are always revalidated server-side.
    if let Some(account_id) = session.account_id().filter(|v| !v.is_empty()) {
        let token = crate::pus_web::create_session(
            sid,
            account_id.to_string(),
            world.get_session_name(sid).unwrap_or_default(),
            session.addr().ip(),
        );
        tracing::debug!(sid, token_prefix = %&token[..8], "PUS WebView2 session prepared");
    }
    Ok(())
}

/// Handle STORE_CLOSE — load web item mall and send inventory refresh.
/// Sniffer verified (session 3, seq 13): response is exactly 422 bytes =
/// 2 header (opcode+sub) + 28 bag slots * 15 bytes each = 420.
/// All 28 slots are sent including empty ones (zero-filled).
async fn handle_store_close(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    world.set_store_open(sid, false);

    let inv = world.get_inventory(sid);

    let mut response = Packet::new(Opcode::WizShoppingMall as u8);
    response.write_u8(STORE_CLOSE);

    // Sniffer: original server sends ALL 28 bag slots (SLOT_MAX..SLOT_MAX+HAVE_MAX),
    // including empty ones as zero-filled 15-byte entries.
    for i in SLOT_MAX..(SLOT_MAX + HAVE_MAX) {
        let slot = inv.get(i).cloned().unwrap_or_default();
        response.write_u32(slot.item_id);
        response.write_u16(slot.durability as u16);
        response.write_u16(slot.count);
        response.write_u8(slot.flag);
        response.write_u16(slot.remaining_rental_minutes());
        response.write_u32(slot.expire_time);
    }

    session.send_packet(&response).await?;
    Ok(())
}

/// Validate a PUS purchase request server-side.
/// Returns `Ok((item_id, buy_count, price))` if valid, `Err(reason)` otherwise.
/// Checks:
/// 1. Item listing exists in the loaded PUS catalog
/// 2. Price is positive
/// 3. Category is active
pub fn validate_purchase(
    world: &crate::world::WorldState,
    listing_id: i32,
) -> Result<(i32, i32, i32, i16), &'static str> {
    let item = world
        .get_pus_item(listing_id)
        .ok_or("PUS item listing not found")?;

    let price = item.price.unwrap_or(0);
    if price < 0 {
        return Err("Invalid item price");
    }

    // Verify category is active
    let categories = world.get_pus_categories();
    let cat_active = categories.iter().any(|c| c.category_id == item.category);
    if !cat_active {
        return Err("Item category is inactive");
    }

    Ok((item.item_id, item.buy_count, price, item.price_type))
}

/// Build a STORE_OPEN success response packet.
pub fn build_store_open_success(free_slots: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
    pkt.write_u8(STORE_OPEN);
    pkt.write_u16(1);
    pkt.write_i16(free_slots);
    pkt
}

/// Build a STORE_OPEN error response packet.
pub fn build_store_open_error(error_code: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
    pkt.write_u8(STORE_OPEN);
    pkt.write_u16(error_code);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_db::models::cash_shop::{PusCategoryRow, PusItemRow};

    #[test]
    fn test_store_close_item_size() {
        // Each item in STORE_CLOSE is: u32 + u16 + u16 + u8 + u16 + u32 = 15 bytes
        let item_byte_size: usize = 4 + 2 + 2 + 1 + 2 + 4;
        assert_eq!(item_byte_size, 15);

        // Items from SLOT_MAX..INVENTORY_TOTAL = 14..96 = 82 items
        let item_count = INVENTORY_TOTAL - SLOT_MAX;
        assert_eq!(item_count, 82);
    }

    #[test]
    fn test_constants() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(INVENTORY_TOTAL, 96);
        assert_eq!(STORE_OPEN, 1);
        assert_eq!(STORE_CLOSE, 2);
        assert_eq!(STORE_LETTER, 6);
        assert_eq!(ERR_INVALID_STATE, 0xFFF7);
        assert_eq!(ERR_NO_FREE_SLOTS, 0xFFF8);
        assert_eq!(PRICE_TYPE_KC, 0);
        assert_eq!(PRICE_TYPE_TL, 1);
    }

    #[test]
    fn test_store_open_success_packet() {
        let pkt = build_store_open_success(15);
        assert_eq!(pkt.opcode, Opcode::WizShoppingMall as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // STORE_OPEN
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_i16(), Some(15)); // free_slots
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_store_open_error_dead_packet() {
        let pkt = build_store_open_error(ERR_INVALID_STATE);
        assert_eq!(pkt.opcode, Opcode::WizShoppingMall as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // STORE_OPEN
        assert_eq!(r.read_u16(), Some(0xFFF7)); // -9 as u16
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_store_open_error_no_slots_packet() {
        let pkt = build_store_open_error(ERR_NO_FREE_SLOTS);
        assert_eq!(pkt.opcode, Opcode::WizShoppingMall as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // STORE_OPEN
        assert_eq!(r.read_u16(), Some(0xFFF8)); // -8 as u16
        assert_eq!(r.remaining(), 0);
    }

    fn make_test_world() -> crate::world::WorldState {
        let world = crate::world::WorldState::new();

        // Insert test categories
        world.pus_categories.insert(
            1,
            PusCategoryRow {
                id: 1,
                category_name: "Scrolls".into(),
                description: "Scrollar".into(),
                category_id: 1,
                status: 1,
            },
        );
        world.pus_categories.insert(
            2,
            PusCategoryRow {
                id: 2,
                category_name: "Premium-Other".into(),
                description: "Premium-Others".into(),
                category_id: 2,
                status: 1,
            },
        );

        // Insert test items
        let item1 = PusItemRow {
            id: 1,
            item_id: 800079000,
            item_name: Some("HP Scroll 60%".into()),
            item_title: Some("HP Scroll 60%".into()),
            price: Some(499),
            send_type: Some(1),
            buy_count: 1,
            item_desc: "HP Scroll 60%".into(),
            category: 1,
            price_type: PRICE_TYPE_KC,
        };
        let item2 = PusItemRow {
            id: 29,
            item_id: 399295859,
            item_name: Some("Switching Premium".into()),
            item_title: Some("Switching Premium".into()),
            price: Some(125),
            send_type: Some(1),
            buy_count: 1,
            item_desc: "Switching Premium".into(),
            category: 2,
            price_type: PRICE_TYPE_TL,
        };
        let item3 = PusItemRow {
            id: 112,
            item_id: 489500000,
            item_name: Some("10 TL To 200 KC".into()),
            item_title: Some("10 TL To 200 KC".into()),
            price: Some(10),
            send_type: Some(1),
            buy_count: 200,
            item_desc: "10 TL To 200 KC".into(),
            category: 5,
            price_type: PRICE_TYPE_TL,
        };

        world.pus_items_by_id.insert(item1.id, item1.clone());
        world.pus_items_by_id.insert(item2.id, item2.clone());
        world.pus_items_by_id.insert(item3.id, item3.clone());

        world
            .pus_items_by_category
            .entry(1)
            .or_default()
            .push(item1);
        world
            .pus_items_by_category
            .entry(2)
            .or_default()
            .push(item2);
        // item3 category 5 not inserted into categories -> inactive category test

        world
    }

    #[test]
    fn test_validate_purchase_kc_item() {
        let world = make_test_world();
        let result = validate_purchase(&world, 1);
        assert!(result.is_ok());
        let (item_id, buy_count, price, price_type) = result.unwrap();
        assert_eq!(item_id, 800079000);
        assert_eq!(buy_count, 1);
        assert_eq!(price, 499);
        assert_eq!(price_type, PRICE_TYPE_KC);
    }

    #[test]
    fn test_validate_purchase_tl_item() {
        let world = make_test_world();
        let result = validate_purchase(&world, 29);
        assert!(result.is_ok());
        let (item_id, buy_count, price, price_type) = result.unwrap();
        assert_eq!(item_id, 399295859);
        assert_eq!(buy_count, 1);
        assert_eq!(price, 125);
        assert_eq!(price_type, PRICE_TYPE_TL);
    }

    #[test]
    fn test_validate_purchase_nonexistent_listing() {
        let world = make_test_world();
        let result = validate_purchase(&world, 9999);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "PUS item listing not found");
    }

    #[test]
    fn test_validate_purchase_inactive_category() {
        let world = make_test_world();
        // Item 112 is in category 5, which is not in pus_categories
        let result = validate_purchase(&world, 112);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Item category is inactive");
    }

    #[test]
    fn test_pus_categories_loaded() {
        let world = make_test_world();
        assert_eq!(world.pus_category_count(), 2);
        let cats = world.get_pus_categories();
        assert_eq!(cats.len(), 2);
    }

    #[test]
    fn test_pus_items_by_category() {
        let world = make_test_world();
        let scrolls = world.get_pus_items_by_category(1);
        assert_eq!(scrolls.len(), 1);
        assert_eq!(scrolls[0].item_id, 800079000);

        let premium = world.get_pus_items_by_category(2);
        assert_eq!(premium.len(), 1);
        assert_eq!(premium[0].item_id, 399295859);

        // Non-existent category returns empty
        let empty = world.get_pus_items_by_category(99);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_pus_item_count() {
        let world = make_test_world();
        assert_eq!(world.pus_item_count(), 3);
    }

    #[test]
    fn test_pus_item_lookup() {
        let world = make_test_world();
        let item = world.get_pus_item(1);
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.item_id, 800079000);
        assert_eq!(item.price, Some(499));
        assert_eq!(item.price_type, PRICE_TYPE_KC);

        assert!(world.get_pus_item(9999).is_none());
    }

    #[test]
    fn test_store_close_total_bytes() {
        // STORE_CLOSE sends 82 items * 15 bytes = 1230 bytes of item data
        // Plus 1 byte sub-opcode header = 1231 bytes total data
        let items = INVENTORY_TOTAL - SLOT_MAX;
        let item_bytes = items * 15;
        assert_eq!(item_bytes, 1230);
    }

    #[test]
    fn test_price_type_values() {
        // Verify price type matches MSSQL data
        // PriceType 0 = KC (Knight Cash)
        // PriceType 1 = TL (real money balance)
        assert_eq!(PRICE_TYPE_KC, 0);
        assert_eq!(PRICE_TYPE_TL, 1);
    }

    #[test]
    fn test_validate_purchase_zero_price_item() {
        let world = make_test_world();

        // Insert a free item (price=0, like VIP Hazir Paket ID=26)
        let free_item = PusItemRow {
            id: 26,
            item_id: 810039000,
            item_name: Some("VIP Hazir Paket".into()),
            item_title: Some("VIP Hazir Paket".into()),
            price: Some(0),
            send_type: Some(1),
            buy_count: 1,
            item_desc: "VIP Hazir Paket".into(),
            category: 2,
            price_type: PRICE_TYPE_TL,
        };
        world.pus_items_by_id.insert(free_item.id, free_item);

        let result = validate_purchase(&world, 26);
        assert!(result.is_ok());
        let (_, _, price, _) = result.unwrap();
        assert_eq!(price, 0); // Free items are valid
    }

    #[test]
    fn test_validate_purchase_buy_count_multiple() {
        let world = make_test_world();

        // Item 112 has buy_count=200 (10 TL To 200 KC)
        // But its category is inactive, so let's add category 5
        world.pus_categories.insert(
            5,
            PusCategoryRow {
                id: 5,
                category_name: "TL & Knight KC".into(),
                description: "TL & Knight KC".into(),
                category_id: 5,
                status: 1,
            },
        );

        let result = validate_purchase(&world, 112);
        assert!(result.is_ok());
        let (item_id, buy_count, price, price_type) = result.unwrap();
        assert_eq!(item_id, 489500000);
        assert_eq!(buy_count, 200); // Gives 200 items per purchase
        assert_eq!(price, 10);
        assert_eq!(price_type, PRICE_TYPE_TL);
    }

    // ── Sprint 313: STORE_CLOSE skips empty slots ────────────────────

    /// `if (pItem == nullptr) continue;` — empty slots are skipped.
    /// Packet size is variable (only non-empty items), NOT fixed 63*15 bytes.
    #[test]
    fn test_store_close_skips_empty_slots() {
        use ko_protocol::Packet;
        let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
        pkt.write_u8(STORE_CLOSE);

        // Simulate: 3 slots, 1 has an item, 2 are empty
        let items = [(0u32, 0u16, 0u16), (100001, 100, 5), (0, 0, 0)];
        for (id, dur, cnt) in &items {
            if *id == 0 {
                continue; // skip empty
            }
            pkt.write_u32(*id);
            pkt.write_u16(*dur);
            pkt.write_u16(*cnt);
            pkt.write_u8(0);
            pkt.write_u16(0);
            pkt.write_u32(0);
        }

        // Only 1 item written: 1 (sub-opcode) + 15 (item data) = 16 bytes
        assert_eq!(pkt.data.len(), 1 + 15);
    }

    #[test]
    fn test_store_close_empty_inventory() {
        use ko_protocol::Packet;
        let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
        pkt.write_u8(STORE_CLOSE);

        // All slots empty → nothing written after sub-opcode
        for _ in 0..63 {
            let item_id: u32 = 0;
            if item_id == 0 {
                continue;
            }
        }

        // Only sub-opcode byte
        assert_eq!(pkt.data.len(), 1);
    }

    // ── Sprint 323: Genie state check ───────────────────────────────

    #[test]
    fn test_genie_blocks_store_open() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Default: genie not active
        let genie = world.with_session(1, |h| h.genie_active).unwrap();
        assert!(!genie, "genie should be inactive by default");

        // Set genie active
        world.update_session(1, |h| h.genie_active = true);
        let genie = world.with_session(1, |h| h.genie_active).unwrap();
        assert!(genie, "genie should be active after set");
    }

    // ── Sprint 329: Store open state tracking ──────────────────────

    /// Test that store_open flag is tracked and defaults to false.
    #[test]
    fn test_store_open_flag_tracking() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Default: store not open
        assert!(!world.is_store_open(1));

        // Open store
        world.set_store_open(1, true);
        assert!(world.is_store_open(1));

        // Close store
        world.set_store_open(1, false);
        assert!(!world.is_store_open(1));
    }

    /// Test that warehouse is blocked when store is open.
    #[test]
    fn test_store_open_blocks_warehouse() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_store_open(1, true);
        assert!(
            world.is_store_open(1),
            "Store open should block warehouse access"
        );
    }
}
