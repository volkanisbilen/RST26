//! Premium and item time-expiry tick system.
//!   - Calls `UpdateCheckPremiumTime()` and `UpdateCheckItemTime()` every 1 second.
//! In this Rust implementation, we run the check every 10 seconds (reduced
//! frequency is sufficient and avoids per-second overhead).
//! ## Premium Expiry
//! Iterates each player's `premium_map`. If a premium's expiry timestamp is
//! non-zero and less than the current unix time, it is removed. If the active
//! premium was among the expired entries, `premium_in_use` is reset to
//! `NO_PREMIUM`. An updated `WIZ_PREMIUM` info packet is sent to the client.
//! ## Item Expiry
//! Scans all inventory slots (equipment + bag + cospre + magic bags), warehouse,
//! and VIP warehouse for items with `expire_time > 0 && expire_time < now`.
//! Expired items are zeroed out. For inventory/equipment items, a
//! `WIZ_ITEM_COUNT_CHANGE` (SendStackChange) packet is sent per removed item.
//! Warehouse and VIP warehouse items are silently removed (matching C++ behavior).
//! After any inventory removal, `SetUserAbility` + `SendItemWeight` are called.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ko_protocol::{Opcode, Packet};
use tracing::debug;

#[cfg(test)]
use crate::handler::{COSP_MAX, HAVE_MAX};
use crate::handler::{INVENTORY_COSP, INVENTORY_MBAG, INVENTORY_TOTAL, SLOT_MAX};
use crate::world::{UserItemSlot, WorldState};
use crate::zone::SessionId;

/// Expiry check interval in seconds.
/// C++ checks every 1 second per-user in `Update()`. We use 10 seconds to
/// reduce overhead while still catching expirations promptly.
const EXPIRY_TICK_INTERVAL_SECS: u64 = 10;

/// No premium active.
const NO_PREMIUM: u8 = 0;

use crate::handler::premium::SUBOPCODE_PREMIUM_INFO;

/// Start of inventory bag region (equals SLOT_MAX).
const INVENTORY_INVENT: usize = SLOT_MAX;

// ── Item section constants for SendStackChange ──────────────────────────

/// Equipment section.
const ITEM_SECTION_SLOT: u8 = 0;

/// Inventory bag section.
const ITEM_SECTION_INVEN: u8 = 1;

/// Cospre section.
const ITEM_SECTION_COSPRE: u8 = 3;

/// Magic bag section.
const ITEM_SECTION_MBAG: u8 = 4;

// ── Cospre absolute-to-relative index mapping ───────────────────────────
// C++ defines:
//   CWING=42, CHELMET=43, CLEFT=44, CRIGHT=45, CTOP=46, CEMBLEM=47,
//   CFAIRY=48, CTATTOO=49, CTALISMAN=50, CBAG1=51, CBAG2=52
// Relative positions:
//   COSP_WINGS=0, COSP_HELMET=1, COSP_GLOVE=2, COSP_GLOVE2=3,
//   COSP_BREAST=4, COSP_EMBLAM=5, COSP_BAG1=6, COSP_FAIRY=7,
//   COSP_TATTO=8, COSP_TALISMAN=9, COSP_BAG2=10

/// Map an absolute cospre slot index to the relative position used by the client.
fn cospre_relative_pos(absolute_idx: usize) -> Option<u8> {
    match absolute_idx {
        42 => Some(0),  // CWING -> COSP_WINGS
        43 => Some(1),  // CHELMET -> COSP_HELMET
        44 => Some(2),  // CLEFT -> COSP_GLOVE
        45 => Some(3),  // CRIGHT -> COSP_GLOVE2
        46 => Some(4),  // CTOP -> COSP_BREAST
        47 => Some(5),  // CEMBLEM -> COSP_EMBLAM
        48 => Some(7),  // CFAIRY -> COSP_FAIRY
        49 => Some(8),  // CTATTOO -> COSP_TATTO
        50 => Some(9),  // CTALISMAN -> COSP_TALISMAN
        51 => Some(6),  // CBAG1 -> COSP_BAG1
        52 => Some(10), // CBAG2 -> COSP_BAG2
        _ => None,
    }
}

/// Start the premium/item expiry background task.
/// Spawns a tokio task that ticks every 10 seconds and checks all online
/// sessions for expired premiums and items.
pub fn start_expiry_tick_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(EXPIRY_TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            process_expiry_tick(&world);
        }
    })
}

/// Process one expiry tick — check all in-game sessions.
/// Also calls per-player flash/burning time ticks which run on similar
/// intervals (C++ calls these from the per-player `Update()` loop).
fn process_expiry_tick(world: &WorldState) {
    let now = current_unix_time();
    let session_ids = world.get_in_game_session_ids();

    for sid in session_ids {
        check_premium_expiry(world, sid, now);
        check_item_expiry(world, sid, now);
        check_vip_vault_expiry(world, sid, now);
        crate::systems::flash::flash_update_tick(world, sid, now as u64);
        crate::systems::flash::burning_time_tick(world, sid, now as u64);
        crate::handler::genie::check_genie_time_tick(world, sid, now as u64);
        check_return_symbol_expiry(world, sid, now as i64);
    }
}

/// Reset return symbol when its time has expired.
/// ```cpp
/// if (ReturnSymbolisOK > 0 && ReturnSymbolTime < int64(UNIXTIME)) {
///     ReturnSymbolTime = 0;
///     ReturnSymbolisOK = 0;
/// }
/// ```
fn check_return_symbol_expiry(world: &WorldState, sid: SessionId, now: i64) {
    let (ok, time) = world
        .with_session(sid, |h| (h.return_symbol_ok, h.return_symbol_time))
        .unwrap_or((0, 0));
    if ok > 0 && time > 0 && time < now {
        world.update_session(sid, |h| {
            h.return_symbol_ok = 0;
            h.return_symbol_time = 0;
        });
        debug!(
            "[sid={}] Return symbol expired: reset (was ok={}, time={})",
            sid, ok, time
        );
    }
}

/// Reset VIP storage vault expiration flag when expired.
/// ```cpp
/// if (m_bVIPStorageVaultExpiration && (uint32)UNIXTIME >= m_bVIPStorageVaultExpiration)
///     m_bVIPStorageVaultExpiration = 0;
/// ```
/// When the vault has expired, the flag is set to 0 so subsequent checks
/// see an inactive vault rather than a stale timestamp.
fn check_vip_vault_expiry(world: &WorldState, sid: SessionId, now: u32) {
    let expiry = world.with_session(sid, |h| h.vip_vault_expiry).unwrap_or(0);
    if expiry != 0 && now >= expiry {
        world.update_session(sid, |h| {
            h.vip_vault_expiry = 0;
        });
        debug!("[sid={}] VIP vault expiry: reset (was {})", sid, expiry);
    }
}

/// Check and remove expired premiums for a single session.
fn check_premium_expiry(world: &WorldState, sid: SessionId, now: u32) {
    // Collect expired premium types inside the DashMap lock.
    let mut expired_types: Vec<u8> = Vec::with_capacity(10);
    let mut active_expired = false;

    world.with_session(sid, |h| {
        for (&p_type, &expiry) in &h.premium_map {
            if expiry == 0 || expiry > now {
                continue;
            }
            expired_types.push(p_type);
            if p_type == h.premium_in_use {
                active_expired = true;
            }
        }
    });

    if expired_types.is_empty() {
        return;
    }

    // Remove expired entries and reset active premium if needed.
    world.update_session(sid, |h| {
        for &p_type in &expired_types {
            h.premium_map.remove(&p_type);
        }
        if active_expired {
            h.premium_in_use = NO_PREMIUM;
        }
    });

    // Build and send updated premium info packet.
    let pkt = build_premium_info_packet(world, sid, now);
    world.send_to_session_owned(sid, pkt);

    debug!(
        "[sid={}] premium expiry: removed {} expired type(s)",
        sid,
        expired_types.len()
    );
}

/// Build a `WIZ_PREMIUM` sub-opcode 1 info packet for a given session.
/// Equivalent to `build_premium_info()` in `premium.rs` but works from
/// `&WorldState` + `SessionId` without needing a `ClientSession`.
fn build_premium_info_packet(world: &WorldState, sid: SessionId, now: u32) -> Packet {
    let mut entries: Vec<(u8, u16)> = Vec::with_capacity(10);
    let mut premium_in_use: u8 = NO_PREMIUM;

    world.with_session(sid, |h| {
        premium_in_use = h.premium_in_use;

        for (&p_type, &expiry) in &h.premium_map {
            if expiry == 0 {
                continue;
            }
            let time_rest = expiry.saturating_sub(now);
            let time_show: u16 = if (1..=3600).contains(&time_rest) {
                1
            } else {
                (time_rest / 3600) as u16
            };
            entries.push((p_type, time_show));

            // Auto-select first valid premium if none selected
            if premium_in_use == NO_PREMIUM {
                premium_in_use = p_type;
            }
        }
    });

    // If auto-selected, persist to session.
    let original_in_use = world
        .with_session(sid, |h| h.premium_in_use)
        .unwrap_or(NO_PREMIUM);
    if premium_in_use != original_in_use {
        world.update_session(sid, |h| {
            h.premium_in_use = premium_in_use;
        });
    }

    let mut resp = Packet::new(Opcode::WizPremium as u8);
    resp.write_u8(SUBOPCODE_PREMIUM_INFO);
    resp.write_u8(entries.len() as u8);
    for (p_type, time_show) in &entries {
        resp.write_u8(*p_type);
        resp.write_u16(*time_show);
    }
    resp.write_u8(premium_in_use);
    resp.write_u32(0);
    resp
}

/// Description of a removed expired item slot for packet building.
struct ExpiredItemSlot {
    /// Absolute slot index in the inventory array.
    absolute_idx: usize,
}

/// Check and remove expired items for a single session.
pub fn check_item_expiry(world: &WorldState, sid: SessionId, now: u32) {
    let mut expired_inventory_slots: Vec<ExpiredItemSlot> = Vec::with_capacity(8);
    let mut any_warehouse_expired = false;
    let mut any_vip_expired = false;

    // Phase 1: Scan and clear expired items inside the DashMap lock.
    world.update_session(sid, |h| {
        // ── Inventory (equipment + bag + cospre + magic bags) ─────────
        let inv_len = h.inventory.len().min(INVENTORY_TOTAL);
        for i in 0..inv_len {
            let slot = &h.inventory[i];
            if slot.item_id == 0 {
                continue;
            }
            if slot.expire_time > 0 && slot.expire_time < now {
                // C++ User.cpp:1132-1141 — reset special flags before clearing
                let item_id = slot.item_id;
                if i == 5
                    && matches!(
                        item_id,
                        950_680_000 | 850_680_000 | 510_000_000 | 520_000_000
                    )
                {
                    h.auto_loot = false;
                }
                if i == 48 && item_id == 700_039_768 {
                    h.fairy_check = false;
                }
                expired_inventory_slots.push(ExpiredItemSlot { absolute_idx: i });
                h.inventory[i] = UserItemSlot::default();
            }
        }

        // ── Warehouse (silently remove, no packet) ───────────────────
        for slot in h.warehouse.iter_mut() {
            if slot.item_id == 0 {
                continue;
            }
            if slot.expire_time > 0 && slot.expire_time < now {
                *slot = UserItemSlot::default();
                any_warehouse_expired = true;
            }
        }

        // ── VIP Warehouse (silently remove, no packet) ───────────────
        for slot in h.vip_warehouse.iter_mut() {
            if slot.item_id == 0 {
                continue;
            }
            if slot.expire_time > 0 && slot.expire_time < now {
                *slot = UserItemSlot::default();
                any_vip_expired = true;
            }
        }
    });

    if expired_inventory_slots.is_empty() && !any_warehouse_expired && !any_vip_expired {
        return;
    }

    // Phase 2: Send WIZ_ITEM_EXPIRATION (0x74) UI refresh trigger.
    // IDA verified: v2600 client reads NO payload — bare opcode triggers UI close/refresh.
    if !expired_inventory_slots.is_empty() {
        let notify = Packet::new(Opcode::WizItemExpiration as u8);
        world.send_to_session_owned(sid, notify);
    }

    // Phase 3: Send removal packets for inventory items.
    for expired in &expired_inventory_slots {
        let i = expired.absolute_idx;
        let pkt = build_item_removal_packet(i);
        world.send_to_session_owned(sid, pkt);
    }

    // Phase 3: Recalculate stats and weight if any inventory items were removed.
    if !expired_inventory_slots.is_empty() {
        // Weight notification is integrated into set_user_ability().
        world.set_user_ability(sid);
    }

    let total = expired_inventory_slots.len()
        + if any_warehouse_expired { 1 } else { 0 }
        + if any_vip_expired { 1 } else { 0 };
    debug!(
        "[sid={}] item expiry: {} inventory slot(s), warehouse={}, vip={}",
        sid,
        expired_inventory_slots.len(),
        any_warehouse_expired,
        any_vip_expired,
    );
    let _ = total; // suppress unused warning
}

/// Build a `WIZ_ITEM_COUNT_CHANGE` (SendStackChange) packet for item removal.
/// When removing an expired item, all fields are zeroed:
/// `SendStackChange(0, 0, 0, pos, true, 0, section)`
fn build_item_removal_packet(absolute_idx: usize) -> Packet {
    let (section, pos) = slot_to_section_and_pos(absolute_idx);

    let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
    pkt.write_u16(1); // count_type: always 1
    pkt.write_u8(section); // slot section
    pkt.write_u8(pos); // position within section
    pkt.write_u32(0); // item_id = 0 (removed)
    pkt.write_u32(0); // count = 0
    pkt.write_u8(100); // bNewItem = true (100)
    pkt.write_u16(0); // durability = 0
    pkt.write_u32(0); // reserved
    pkt.write_u32(0); // expiration = 0
    pkt.write_u16(0); // v2625 SendStackChange trailing field
    pkt
}

/// Map an absolute inventory index to (section, relative_pos) for `SendStackChange`.
fn slot_to_section_and_pos(idx: usize) -> (u8, u8) {
    if idx < SLOT_MAX {
        // Equipment slot: section=0, pos=idx
        (ITEM_SECTION_SLOT, idx as u8)
    } else if idx < INVENTORY_COSP {
        // Inventory bag: section=1, pos=idx-SLOT_MAX
        (ITEM_SECTION_INVEN, (idx - INVENTORY_INVENT) as u8)
    } else if idx < INVENTORY_MBAG {
        // Cospre slot: section=3, pos=cospre_relative_pos()
        let rel = cospre_relative_pos(idx).unwrap_or((idx - INVENTORY_COSP) as u8);
        (ITEM_SECTION_COSPRE, rel)
    } else {
        // Magic bag slot: section=4, pos=idx-INVENTORY_MBAG
        (ITEM_SECTION_MBAG, (idx - INVENTORY_MBAG) as u8)
    }
}

/// Get current unix timestamp as u32.
fn current_unix_time() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;
    use ko_protocol::PacketReader;
    use tokio::sync::mpsc;

    /// Helper: create a WorldState with a registered session.
    fn setup_world() -> (WorldState, SessionId) {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        (world, 1)
    }

    /// Helper: create a WorldState with a registered session that collects sent packets.
    fn setup_world_with_rx() -> (WorldState, SessionId, mpsc::UnboundedReceiver<Arc<Packet>>) {
        let world = WorldState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        (world, 1, rx)
    }

    // ── Premium Expiry Tests ────────────────────────────────────────────

    #[test]
    fn test_check_premium_expiry_no_premiums() {
        let (world, sid) = setup_world();
        // No premiums at all — should be a no-op
        check_premium_expiry(&world, sid, 1000);
        let in_use = world.with_session(sid, |h| h.premium_in_use).unwrap();
        assert_eq!(in_use, NO_PREMIUM);
    }

    #[test]
    fn test_check_premium_expiry_none_expired() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        // Add premiums that haven't expired yet
        world.update_session(sid, |h| {
            h.premium_map.insert(3, now + 3600); // expires in 1 hour
            h.premium_map.insert(5, now + 7200); // expires in 2 hours
            h.premium_in_use = 3;
        });

        check_premium_expiry(&world, sid, now);

        // Nothing should have changed
        let (map_len, in_use) = world
            .with_session(sid, |h| (h.premium_map.len(), h.premium_in_use))
            .unwrap();
        assert_eq!(map_len, 2);
        assert_eq!(in_use, 3);
    }

    #[test]
    fn test_check_premium_expiry_one_expired() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        // Add two premiums: one expired, one still valid
        world.update_session(sid, |h| {
            h.premium_map.insert(3, now - 100); // expired 100s ago
            h.premium_map.insert(5, now + 7200); // valid for 2 hours
            h.premium_in_use = 5;
        });

        check_premium_expiry(&world, sid, now);

        let (map_len, in_use, has_3, has_5) = world
            .with_session(sid, |h| {
                (
                    h.premium_map.len(),
                    h.premium_in_use,
                    h.premium_map.contains_key(&3),
                    h.premium_map.contains_key(&5),
                )
            })
            .unwrap();
        assert_eq!(map_len, 1);
        assert!(!has_3, "expired premium type 3 should be removed");
        assert!(has_5, "valid premium type 5 should remain");
        assert_eq!(in_use, 5, "active premium should remain unchanged");

        // Should have received a WIZ_PREMIUM packet
        let pkt = rx.try_recv().expect("should receive premium info packet");
        assert_eq!(pkt.opcode, Opcode::WizPremium as u8);
    }

    #[test]
    fn test_check_premium_expiry_active_premium_expired() {
        let (world, sid, _rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        // The active premium is the one that expires
        world.update_session(sid, |h| {
            h.premium_map.insert(3, now - 50); // expired
            h.premium_in_use = 3;
        });

        check_premium_expiry(&world, sid, now);

        let (map_len, in_use) = world
            .with_session(sid, |h| (h.premium_map.len(), h.premium_in_use))
            .unwrap();
        assert_eq!(map_len, 0, "expired premium should be removed");
        assert_eq!(
            in_use, NO_PREMIUM,
            "active premium should reset to NO_PREMIUM"
        );
    }

    #[test]
    fn test_check_premium_expiry_all_expired() {
        let (world, sid, _rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        world.update_session(sid, |h| {
            h.premium_map.insert(1, now - 1000);
            h.premium_map.insert(3, now - 500);
            h.premium_map.insert(5, now - 100);
            h.premium_in_use = 3;
        });

        check_premium_expiry(&world, sid, now);

        let (map_len, in_use) = world
            .with_session(sid, |h| (h.premium_map.len(), h.premium_in_use))
            .unwrap();
        assert_eq!(map_len, 0);
        assert_eq!(in_use, NO_PREMIUM);
    }

    #[test]
    fn test_check_premium_expiry_zero_time_not_expired() {
        let (world, sid) = setup_world();
        // A premium with expiry_time=0 should NOT be treated as expired
        world.update_session(sid, |h| {
            h.premium_map.insert(3, 0);
            h.premium_in_use = 3;
        });

        check_premium_expiry(&world, sid, 1_700_000_000);

        let map_len = world.with_session(sid, |h| h.premium_map.len()).unwrap();
        assert_eq!(map_len, 1, "premium with time=0 should not be removed");
    }

    // ── Premium Info Packet Tests ───────────────────────────────────────

    #[test]
    fn test_build_premium_info_packet_empty() {
        let (world, sid) = setup_world();
        let pkt = build_premium_info_packet(&world, sid, 1_700_000_000);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUBOPCODE_PREMIUM_INFO));
        assert_eq!(r.read_u8(), Some(0)); // count = 0
        assert_eq!(r.read_u8(), Some(NO_PREMIUM)); // premium_in_use
        assert_eq!(r.read_u32(), Some(0)); // trailing zero
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_premium_info_packet_with_entries() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        world.update_session(sid, |h| {
            h.premium_map.insert(3, now + 86400); // 24 hours
            h.premium_in_use = 3;
        });

        let pkt = build_premium_info_packet(&world, sid, now);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUBOPCODE_PREMIUM_INFO));
        assert_eq!(r.read_u8(), Some(1)); // count = 1
        assert_eq!(r.read_u8(), Some(3)); // premium_type
        assert_eq!(r.read_u16(), Some(24)); // 24 hours
        assert_eq!(r.read_u8(), Some(3)); // premium_in_use
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_premium_info_packet_auto_selects_first() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        // No premium_in_use but there's a valid premium in the map
        world.update_session(sid, |h| {
            h.premium_map.insert(5, now + 7200);
            h.premium_in_use = NO_PREMIUM;
        });

        let pkt = build_premium_info_packet(&world, sid, now);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUBOPCODE_PREMIUM_INFO));
        assert_eq!(r.read_u8(), Some(1)); // count
        assert_eq!(r.read_u8(), Some(5)); // type
        assert_eq!(r.read_u16(), Some(2)); // 2 hours
        assert_eq!(r.read_u8(), Some(5)); // auto-selected
        assert_eq!(r.read_u32(), Some(0));

        // Verify it was persisted
        let in_use = world.with_session(sid, |h| h.premium_in_use).unwrap();
        assert_eq!(in_use, 5);
    }

    // ── Item Expiry Tests ───────────────────────────────────────────────

    #[test]
    fn test_check_item_expiry_no_items() {
        let (world, sid) = setup_world();
        // Empty inventory — should be a no-op
        check_item_expiry(&world, sid, 1_700_000_000);
    }

    #[test]
    fn test_check_item_expiry_no_expiring_items() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        // Items with expire_time=0 (no expiry) should not be removed
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[SLOT_MAX].item_id = 100001;
        inv[SLOT_MAX].count = 1;
        inv[SLOT_MAX].durability = 1000;
        inv[SLOT_MAX].expire_time = 0;
        world.set_inventory(sid, inv);

        check_item_expiry(&world, sid, now);

        let slot = world.get_inventory_slot(sid, SLOT_MAX).unwrap();
        assert_eq!(slot.item_id, 100001, "non-expiring item should remain");
    }

    #[test]
    fn test_check_item_expiry_future_expiry_not_removed() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[SLOT_MAX].item_id = 100001;
        inv[SLOT_MAX].count = 1;
        inv[SLOT_MAX].expire_time = now + 3600; // expires in 1 hour
        world.set_inventory(sid, inv);

        check_item_expiry(&world, sid, now);

        let slot = world.get_inventory_slot(sid, SLOT_MAX).unwrap();
        assert_eq!(slot.item_id, 100001, "future-expiry item should remain");
    }

    #[test]
    fn test_check_item_expiry_expired_inventory_item() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Inventory bag slot 0 (absolute index 14)
        inv[SLOT_MAX].item_id = 200001;
        inv[SLOT_MAX].count = 5;
        inv[SLOT_MAX].durability = 500;
        inv[SLOT_MAX].expire_time = now - 100; // expired 100s ago
        world.set_inventory(sid, inv);

        check_item_expiry(&world, sid, now);

        // Slot should be cleared
        let slot = world.get_inventory_slot(sid, SLOT_MAX).unwrap();
        assert_eq!(slot.item_id, 0, "expired item should be cleared");
        assert_eq!(slot.count, 0);
        assert_eq!(slot.expire_time, 0);

        // Should receive WIZ_ITEM_EXPIRATION (0x74) bare opcode first
        let exp_pkt = rx.try_recv().expect("should receive expiration notify");
        assert_eq!(exp_pkt.opcode, Opcode::WizItemExpiration as u8);
        assert_eq!(exp_pkt.data.len(), 0, "0x74 has no payload (IDA verified)");

        // Then WIZ_ITEM_COUNT_CHANGE packet
        let pkt = rx.try_recv().expect("should receive item removal packet");
        assert_eq!(pkt.opcode, Opcode::WizItemCountChange as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1)); // count_type
        assert_eq!(r.read_u8(), Some(ITEM_SECTION_INVEN)); // section
        assert_eq!(r.read_u8(), Some(0)); // pos (first inventory bag slot)
        assert_eq!(r.read_u32(), Some(0)); // item_id = 0
        assert_eq!(r.read_u32(), Some(0)); // count = 0
        assert_eq!(r.read_u8(), Some(100)); // bNewItem = true
        assert_eq!(r.read_u16(), Some(0)); // durability = 0
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(0)); // expiration = 0
                                           // v2600: no trailing u16 padding (sniff verified)
    }

    #[test]
    fn test_check_item_expiry_expired_equipment_slot() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Equipment slot 6 (right hand)
        inv[6].item_id = 300001;
        inv[6].count = 1;
        inv[6].durability = 2000;
        inv[6].expire_time = now - 500;
        world.set_inventory(sid, inv);

        check_item_expiry(&world, sid, now);

        let slot = world.get_inventory_slot(sid, 6).unwrap();
        assert_eq!(slot.item_id, 0, "expired equipment should be cleared");

        // Drain the 0x74 expiration notify first
        let exp_pkt = rx.try_recv().expect("should receive 0x74 notify");
        assert_eq!(exp_pkt.opcode, Opcode::WizItemExpiration as u8);

        // Check the removal packet uses ITEM_SECTION_SLOT
        let pkt = rx.try_recv().expect("should receive removal packet");
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(ITEM_SECTION_SLOT)); // equipment section
        assert_eq!(r.read_u8(), Some(6)); // equipment slot index
    }

    #[test]
    fn test_check_item_expiry_warehouse_silent_removal() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        // Set up warehouse with an expired item
        world.update_session(sid, |h| {
            h.warehouse = vec![UserItemSlot::default(); WorldState::WAREHOUSE_MAX];
            h.warehouse[0].item_id = 400001;
            h.warehouse[0].count = 1;
            h.warehouse[0].expire_time = now - 200;
        });

        check_item_expiry(&world, sid, now);

        // Warehouse item should be cleared
        let cleared = world
            .with_session(sid, |h| h.warehouse[0].item_id == 0)
            .unwrap();
        assert!(cleared, "expired warehouse item should be cleared");

        // No packet should be sent for warehouse items (C++ behavior: silent removal)
        // We might receive a weight packet if inventory items were also expired.
        // Since no inventory items expired, there should be no packets.
        assert!(
            rx.try_recv().is_err(),
            "no packets should be sent for warehouse-only expiry"
        );
    }

    #[test]
    fn test_check_item_expiry_vip_warehouse_silent_removal() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        world.update_session(sid, |h| {
            h.vip_warehouse = vec![UserItemSlot::default(); 48];
            h.vip_warehouse[5].item_id = 500001;
            h.vip_warehouse[5].count = 1;
            h.vip_warehouse[5].expire_time = now - 10;
        });

        check_item_expiry(&world, sid, now);

        let cleared = world
            .with_session(sid, |h| h.vip_warehouse[5].item_id == 0)
            .unwrap();
        assert!(cleared, "expired VIP warehouse item should be cleared");

        assert!(
            rx.try_recv().is_err(),
            "no packets for VIP warehouse-only expiry"
        );
    }

    #[test]
    fn test_check_item_expiry_multiple_items() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Two expired inventory items and one non-expired
        inv[SLOT_MAX].item_id = 100001;
        inv[SLOT_MAX].expire_time = now - 100;
        inv[SLOT_MAX].count = 1;

        inv[SLOT_MAX + 1].item_id = 100002;
        inv[SLOT_MAX + 1].expire_time = now - 50;
        inv[SLOT_MAX + 1].count = 2;

        inv[SLOT_MAX + 2].item_id = 100003;
        inv[SLOT_MAX + 2].expire_time = now + 3600; // still valid
        inv[SLOT_MAX + 2].count = 3;
        world.set_inventory(sid, inv);

        check_item_expiry(&world, sid, now);

        // First two should be cleared, third should remain
        assert_eq!(world.get_inventory_slot(sid, SLOT_MAX).unwrap().item_id, 0);
        assert_eq!(
            world.get_inventory_slot(sid, SLOT_MAX + 1).unwrap().item_id,
            0
        );
        assert_eq!(
            world.get_inventory_slot(sid, SLOT_MAX + 2).unwrap().item_id,
            100003
        );

        // Drain 0x74 expiration notify
        let exp_pkt = rx.try_recv().expect("0x74 notify");
        assert_eq!(exp_pkt.opcode, Opcode::WizItemExpiration as u8);

        // Should receive 2 removal packets
        let pkt1 = rx.try_recv().expect("first removal packet");
        assert_eq!(pkt1.opcode, Opcode::WizItemCountChange as u8);
        let pkt2 = rx.try_recv().expect("second removal packet");
        assert_eq!(pkt2.opcode, Opcode::WizItemCountChange as u8);
    }

    // ── Slot Section Mapping Tests ──────────────────────────────────────

    #[test]
    fn test_slot_to_section_equipment() {
        for i in 0..SLOT_MAX {
            let (section, pos) = slot_to_section_and_pos(i);
            assert_eq!(section, ITEM_SECTION_SLOT);
            assert_eq!(pos, i as u8);
        }
    }

    #[test]
    fn test_slot_to_section_inventory_bag() {
        for i in INVENTORY_INVENT..(INVENTORY_INVENT + HAVE_MAX) {
            let (section, pos) = slot_to_section_and_pos(i);
            assert_eq!(section, ITEM_SECTION_INVEN);
            assert_eq!(pos, (i - INVENTORY_INVENT) as u8);
        }
    }

    #[test]
    fn test_slot_to_section_cospre() {
        // CWING=42 -> COSP_WINGS=0
        assert_eq!(slot_to_section_and_pos(42), (ITEM_SECTION_COSPRE, 0));
        // CHELMET=43 -> COSP_HELMET=1
        assert_eq!(slot_to_section_and_pos(43), (ITEM_SECTION_COSPRE, 1));
        // CLEFT=44 -> COSP_GLOVE=2
        assert_eq!(slot_to_section_and_pos(44), (ITEM_SECTION_COSPRE, 2));
        // CRIGHT=45 -> COSP_GLOVE2=3
        assert_eq!(slot_to_section_and_pos(45), (ITEM_SECTION_COSPRE, 3));
        // CTOP=46 -> COSP_BREAST=4
        assert_eq!(slot_to_section_and_pos(46), (ITEM_SECTION_COSPRE, 4));
        // CEMBLEM=47 -> COSP_EMBLAM=5
        assert_eq!(slot_to_section_and_pos(47), (ITEM_SECTION_COSPRE, 5));
        // CFAIRY=48 -> COSP_FAIRY=7
        assert_eq!(slot_to_section_and_pos(48), (ITEM_SECTION_COSPRE, 7));
        // CTATTOO=49 -> COSP_TATTO=8
        assert_eq!(slot_to_section_and_pos(49), (ITEM_SECTION_COSPRE, 8));
        // CTALISMAN=50 -> COSP_TALISMAN=9
        assert_eq!(slot_to_section_and_pos(50), (ITEM_SECTION_COSPRE, 9));
        // CBAG1=51 -> COSP_BAG1=6
        assert_eq!(slot_to_section_and_pos(51), (ITEM_SECTION_COSPRE, 6));
        // CBAG2=52 -> COSP_BAG2=10
        assert_eq!(slot_to_section_and_pos(52), (ITEM_SECTION_COSPRE, 10));
    }

    #[test]
    fn test_slot_to_section_magic_bag() {
        // Magic bag slots start at INVENTORY_MBAG=53
        assert_eq!(slot_to_section_and_pos(53), (ITEM_SECTION_MBAG, 0));
        assert_eq!(slot_to_section_and_pos(64), (ITEM_SECTION_MBAG, 11));
        assert_eq!(slot_to_section_and_pos(65), (ITEM_SECTION_MBAG, 12));
        assert_eq!(slot_to_section_and_pos(76), (ITEM_SECTION_MBAG, 23));
    }

    // ── Cospre Relative Position Tests ──────────────────────────────────

    #[test]
    fn test_cospre_relative_pos_all_slots() {
        assert_eq!(cospre_relative_pos(42), Some(0)); // CWING
        assert_eq!(cospre_relative_pos(43), Some(1)); // CHELMET
        assert_eq!(cospre_relative_pos(44), Some(2)); // CLEFT
        assert_eq!(cospre_relative_pos(45), Some(3)); // CRIGHT
        assert_eq!(cospre_relative_pos(46), Some(4)); // CTOP
        assert_eq!(cospre_relative_pos(47), Some(5)); // CEMBLEM
        assert_eq!(cospre_relative_pos(48), Some(7)); // CFAIRY
        assert_eq!(cospre_relative_pos(49), Some(8)); // CTATTOO
        assert_eq!(cospre_relative_pos(50), Some(9)); // CTALISMAN
        assert_eq!(cospre_relative_pos(51), Some(6)); // CBAG1
        assert_eq!(cospre_relative_pos(52), Some(10)); // CBAG2
    }

    #[test]
    fn test_cospre_relative_pos_out_of_range() {
        assert_eq!(cospre_relative_pos(41), None);
        assert_eq!(cospre_relative_pos(53), None);
        assert_eq!(cospre_relative_pos(0), None);
    }

    // ── Item Removal Packet Tests ───────────────────────────────────────

    #[test]
    fn test_build_item_removal_packet_equipment() {
        let pkt = build_item_removal_packet(6);
        assert_eq!(pkt.opcode, Opcode::WizItemCountChange as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(ITEM_SECTION_SLOT));
        assert_eq!(r.read_u8(), Some(6));
        assert_eq!(r.read_u32(), Some(0)); // item_id
        assert_eq!(r.read_u32(), Some(0)); // count
        assert_eq!(r.read_u8(), Some(100)); // bNewItem
        assert_eq!(r.read_u16(), Some(0)); // durability
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(0)); // expiration
        assert_eq!(r.remaining(), 0); // v2600: no trailing padding
    }

    #[test]
    fn test_build_item_removal_packet_inventory() {
        let pkt = build_item_removal_packet(SLOT_MAX + 5);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(ITEM_SECTION_INVEN));
        assert_eq!(r.read_u8(), Some(5));
    }

    #[test]
    fn test_build_item_removal_packet_cospre() {
        let pkt = build_item_removal_packet(48); // CFAIRY
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(ITEM_SECTION_COSPRE));
        assert_eq!(r.read_u8(), Some(7)); // COSP_FAIRY
    }

    #[test]
    fn test_build_item_removal_packet_magic_bag() {
        let pkt = build_item_removal_packet(INVENTORY_MBAG + 3);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(ITEM_SECTION_MBAG));
        assert_eq!(r.read_u8(), Some(3));
    }

    // ── Constants Sanity Tests ──────────────────────────────────────────

    #[test]
    fn test_inventory_layout_constants() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(INVENTORY_INVENT, 14);
        assert_eq!(INVENTORY_COSP, 42);
        assert_eq!(COSP_MAX, 11);
        assert_eq!(INVENTORY_MBAG, 53);
        // v2600: 3 magic bags (14+28+11+36=89)
        assert_eq!(INVENTORY_TOTAL, 96);
    }

    #[test]
    fn test_item_section_constants() {
        assert_eq!(ITEM_SECTION_SLOT, 0);
        assert_eq!(ITEM_SECTION_INVEN, 1);
        assert_eq!(ITEM_SECTION_COSPRE, 3);
        assert_eq!(ITEM_SECTION_MBAG, 4);
    }

    #[test]
    fn test_expiry_interval() {
        assert_eq!(EXPIRY_TICK_INTERVAL_SECS, 10);
    }

    // ── Edge Case Tests ─────────────────────────────────────────────────

    #[test]
    fn test_check_item_expiry_exact_boundary() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // expire_time == now: the C++ check is `< UNIXTIME`, so exact equality
        // means NOT expired
        inv[SLOT_MAX].item_id = 100001;
        inv[SLOT_MAX].count = 1;
        inv[SLOT_MAX].expire_time = now;
        world.set_inventory(sid, inv);

        check_item_expiry(&world, sid, now);

        let slot = world.get_inventory_slot(sid, SLOT_MAX).unwrap();
        assert_eq!(
            slot.item_id, 100001,
            "item at exact boundary (expire_time == now) should NOT be removed"
        );
    }

    #[test]
    fn test_check_premium_expiry_exact_boundary() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        // expire_time == now: C++ check is `> UNIXTIME`, so exact equality
        // means expired (not > now)
        world.update_session(sid, |h| {
            h.premium_map.insert(3, now);
            h.premium_in_use = 3;
        });

        check_premium_expiry(&world, sid, now);

        // C++ code: `pPremium->iPremiumTime > (uint32)UNIXTIME` continues (skips).
        // So if iPremiumTime == UNIXTIME, it does NOT skip — it's treated as expired.
        let map_len = world.with_session(sid, |h| h.premium_map.len()).unwrap();
        assert_eq!(map_len, 0, "premium at exact boundary should be expired");
    }

    #[test]
    fn test_check_item_expiry_mixed_inventory_and_warehouse() {
        let (world, sid, mut rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;

        // Expired inventory item
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[SLOT_MAX].item_id = 100001;
        inv[SLOT_MAX].count = 1;
        inv[SLOT_MAX].expire_time = now - 10;
        world.set_inventory(sid, inv);

        // Expired warehouse item
        world.update_session(sid, |h| {
            h.warehouse = vec![UserItemSlot::default(); WorldState::WAREHOUSE_MAX];
            h.warehouse[0].item_id = 200001;
            h.warehouse[0].count = 1;
            h.warehouse[0].expire_time = now - 20;
        });

        check_item_expiry(&world, sid, now);

        // Both should be cleared
        assert_eq!(world.get_inventory_slot(sid, SLOT_MAX).unwrap().item_id, 0);
        let wh_cleared = world
            .with_session(sid, |h| h.warehouse[0].item_id == 0)
            .unwrap();
        assert!(wh_cleared);

        // Drain 0x74 expiration notify
        let exp_pkt = rx.try_recv().expect("0x74 notify");
        assert_eq!(exp_pkt.opcode, Opcode::WizItemExpiration as u8);

        // Should receive: 1 removal packet
        let pkt1 = rx.try_recv().expect("removal packet");
        assert_eq!(pkt1.opcode, Opcode::WizItemCountChange as u8);
    }

    #[test]
    fn test_process_expiry_tick_empty_world() {
        let world = WorldState::new();
        // Should not panic with no sessions
        process_expiry_tick(&world);
    }

    // ── Touch Session / Last Response Time Tests ────────────────────────

    #[test]
    fn test_touch_session_updates_last_response_time() {
        let (world, sid) = setup_world();

        // Get initial timestamp
        let t1 = world.with_session(sid, |h| h.last_response_time).unwrap();

        // Small sleep to ensure monotonic clock advances
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Touch session
        world.touch_session(sid);

        // Verify timestamp advanced
        let t2 = world.with_session(sid, |h| h.last_response_time).unwrap();
        assert!(t2 > t1, "last_response_time should advance after touch");
    }

    #[test]
    fn test_touch_session_nonexistent_no_panic() {
        let world = WorldState::new();
        // Touching a non-existent session should be a no-op
        world.touch_session(999);
    }

    #[test]
    fn test_last_response_time_initialized_on_register() {
        let world = WorldState::new();
        let before = std::time::Instant::now();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(42, tx);
        let after = std::time::Instant::now();

        let t = world.with_session(42, |h| h.last_response_time).unwrap();
        assert!(
            t >= before && t <= after,
            "last_response_time should be initialized to ~now on register"
        );
    }

    // ── VIP Vault Expiry Reset Tests ────────────────────────────────────

    #[test]
    fn test_vip_vault_expiry_reset_when_expired() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        // Set an expired vault
        world.update_session(sid, |h| {
            h.vip_vault_expiry = now - 100; // expired 100s ago
        });

        check_vip_vault_expiry(&world, sid, now);

        let expiry = world.with_session(sid, |h| h.vip_vault_expiry).unwrap();
        assert_eq!(expiry, 0, "expired VIP vault should be reset to 0");
    }

    #[test]
    fn test_vip_vault_expiry_not_reset_when_active() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        let future = now + 3600;
        world.update_session(sid, |h| {
            h.vip_vault_expiry = future;
        });

        check_vip_vault_expiry(&world, sid, now);

        let expiry = world.with_session(sid, |h| h.vip_vault_expiry).unwrap();
        assert_eq!(expiry, future, "active VIP vault should not be reset");
    }

    #[test]
    fn test_vip_vault_expiry_zero_stays_zero() {
        let (world, sid) = setup_world();
        // vip_vault_expiry=0 means no vault, should not be touched
        check_vip_vault_expiry(&world, sid, 1_700_000_000);

        let expiry = world.with_session(sid, |h| h.vip_vault_expiry).unwrap();
        assert_eq!(expiry, 0, "zero expiry should stay zero");
    }

    #[test]
    fn test_vip_vault_expiry_exact_boundary() {
        let (world, sid) = setup_world();
        let now = 1_700_000_000u32;
        world.update_session(sid, |h| {
            h.vip_vault_expiry = now;
        });

        check_vip_vault_expiry(&world, sid, now);

        let expiry = world.with_session(sid, |h| h.vip_vault_expiry).unwrap();
        assert_eq!(
            expiry, 0,
            "vault at exact boundary (expiry == now) should be reset"
        );
    }

    // ── Return Symbol Expiry Tests ─────────────────────────────────

    #[test]
    fn test_return_symbol_expired() {
        let (world, sid) = setup_world();
        let now: i64 = 1_700_000_000;
        world.update_session(sid, |h| {
            h.return_symbol_ok = 1;
            h.return_symbol_time = now - 3600; // expired 1h ago
        });
        check_return_symbol_expiry(&world, sid, now);
        let (ok, time) = world
            .with_session(sid, |h| (h.return_symbol_ok, h.return_symbol_time))
            .unwrap();
        assert_eq!(ok, 0);
        assert_eq!(time, 0);
    }

    #[test]
    fn test_return_symbol_still_active() {
        let (world, sid) = setup_world();
        let now: i64 = 1_700_000_000;
        world.update_session(sid, |h| {
            h.return_symbol_ok = 1;
            h.return_symbol_time = now + 86400; // expires in 24h
        });
        check_return_symbol_expiry(&world, sid, now);
        let (ok, time) = world
            .with_session(sid, |h| (h.return_symbol_ok, h.return_symbol_time))
            .unwrap();
        assert_eq!(ok, 1);
        assert_eq!(time, now + 86400);
    }

    #[test]
    fn test_return_symbol_inactive_stays_zero() {
        let (world, sid) = setup_world();
        let now: i64 = 1_700_000_000;
        // Default: return_symbol_ok=0, return_symbol_time=0
        check_return_symbol_expiry(&world, sid, now);
        let ok = world.with_session(sid, |h| h.return_symbol_ok).unwrap();
        assert_eq!(ok, 0);
    }

    // ── Item Expiry Special Flag Reset Tests ────────────────────────────

    #[test]
    fn test_check_item_expiry_robin_loot_resets_auto_loot() {
        // C++ User.cpp:1132-1137 — when robin loot item expires from SHOULDER (5),
        // m_bAutoLoot must be reset to false.
        let robin_ids: [u32; 4] = [950_680_000, 850_680_000, 510_000_000, 520_000_000];
        for &robin_id in &robin_ids {
            let (world, sid, _rx) = setup_world_with_rx();
            let now = 1_700_000_000u32;
            let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
            inv[5].item_id = robin_id;
            inv[5].count = 1;
            inv[5].durability = 100;
            inv[5].expire_time = now - 10; // expired
            world.set_inventory(sid, inv);
            world.update_session(sid, |h| {
                h.auto_loot = true;
            });

            check_item_expiry(&world, sid, now);

            let (slot_cleared, auto_loot) = world
                .with_session(sid, |h| (h.inventory[5].item_id == 0, h.auto_loot))
                .unwrap();
            assert!(
                slot_cleared,
                "robin loot item {} should be cleared",
                robin_id
            );
            assert!(
                !auto_loot,
                "auto_loot should be reset to false when robin loot {} expires",
                robin_id
            );
        }
    }

    #[test]
    fn test_check_item_expiry_oreads_resets_fairy_check() {
        // C++ User.cpp:1139-1141 — when ITEM_OREADS expires from CFAIRY (48),
        // m_bFairyCheck must be reset to false.
        let (world, sid, _rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[48].item_id = 700_039_768; // ITEM_OREADS
        inv[48].count = 1;
        inv[48].durability = 100;
        inv[48].expire_time = now - 10; // expired
        world.set_inventory(sid, inv);
        world.update_session(sid, |h| {
            h.fairy_check = true;
        });

        check_item_expiry(&world, sid, now);

        let (slot_cleared, fairy_check) = world
            .with_session(sid, |h| (h.inventory[48].item_id == 0, h.fairy_check))
            .unwrap();
        assert!(slot_cleared, "ITEM_OREADS should be cleared");
        assert!(
            !fairy_check,
            "fairy_check should be reset to false when ITEM_OREADS expires"
        );
    }

    #[test]
    fn test_check_item_expiry_non_robin_shoulder_no_reset() {
        // A non-robin item expiring from SHOULDER should NOT reset auto_loot.
        let (world, sid, _rx) = setup_world_with_rx();
        let now = 1_700_000_000u32;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[5].item_id = 999_999; // some other item
        inv[5].count = 1;
        inv[5].expire_time = now - 10;
        world.set_inventory(sid, inv);
        world.update_session(sid, |h| {
            h.auto_loot = true;
        });

        check_item_expiry(&world, sid, now);

        let auto_loot = world.with_session(sid, |h| h.auto_loot).unwrap();
        assert!(
            auto_loot,
            "auto_loot should remain true for non-robin items"
        );
    }
}
