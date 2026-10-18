//! Canonical inventory slot and size constants.
//! All inventory-related magic numbers live here. Every module that needs
//! equipment slot indices or inventory/warehouse sizes should import from
//! this module rather than defining its own copies.

// ── Inventory sizes ──────────────────────────────────────────────────────

/// Number of equipment slots
pub const SLOT_MAX: usize = 14;

/// Number of inventory bag slots
pub const HAVE_MAX: usize = 28;

/// Number of cospre slot positions
/// Note: v2600 CBAG3 uses slot 53 (=INVENTORY_MBAG) which overlaps with magic bag start.
/// The overlap is handled specially in MyInfo Phase 4 and item_move handler.
pub const COSP_MAX: usize = 11;

/// Start of cospre slots in the item array
pub const INVENTORY_COSP: usize = SLOT_MAX + HAVE_MAX;

/// Start of magic bag 1 region
pub const INVENTORY_MBAG: usize = INVENTORY_COSP + COSP_MAX;

/// Number of magic bag slots per bag
pub const MBAG_MAX: usize = 12;

/// Number of magic bags — v2600 has 3 (C++ v2525 had 2).
pub const MBAG_COUNT: usize = 3;

/// Number of knight royale equipment slots (sniffer: 7 extra after magic bags).
pub const KNIGHT_ROYALE_MAX: usize = 7;

/// Total inventory size — **sniffer verified: orijinal sunucu 96 item gönderiyor.**
/// Layout: equip(14) + bag(28) + cospre(11) + mbag(3×12=36) + knight_royale(7) = 96.
/// Sniffer session 45, seq 10: LZF decompress → 96 items × 19 bytes + header + footer.
pub const INVENTORY_TOTAL: usize = INVENTORY_MBAG + MBAG_MAX * MBAG_COUNT + KNIGHT_ROYALE_MAX;

/// Maximum warehouse slots (`WAREHOUSE_MAX = 192`, 8 pages * 24).
pub const WAREHOUSE_MAX: usize = 192;

/// Items per warehouse/bank page/tab
pub const ITEMS_PER_PAGE: usize = 24;

/// Maximum gold cap
/// Type is u32 to match the `gold` field in CharacterInfo. Callers that need
/// overflow-safe arithmetic should cast to u64 before comparison.
pub const COIN_MAX: u32 = 2_100_000_000;

// NOTE: MAX_ITEM_COUNT (9999) consolidated → canonical `ITEMCOUNT_MAX` in world/types.rs

// ── Equipment slot indices ───────────────────────────────────────────────

/// Right earring slot
pub const RIGHTEAR: usize = 0;

/// Helmet slot
pub const HEAD: usize = 1;

/// Left earring slot
pub const LEFTEAR: usize = 2;

/// Necklace slot
pub const NECK: usize = 3;

/// Chest armour slot
pub const BREAST: usize = 4;

/// Shoulder / pauldron slot
pub const SHOULDER: usize = 5;

/// Right-hand weapon slot
pub const RIGHTHAND: usize = 6;

/// Belt slot
pub const WAIST: usize = 7;

/// Left-hand / shield slot
pub const LEFTHAND: usize = 8;

/// Right ring slot
pub const RIGHTRING: usize = 9;

/// Leg armour slot
pub const LEG: usize = 10;

/// Left ring slot
pub const LEFTRING: usize = 11;

/// Glove slot
pub const GLOVE: usize = 12;

/// Boot slot
pub const FOOT: usize = 13;

// ── Item kind constants ────────────────────────────────────────────────

/// Unique / non-stackable item kind
/// Items with this kind cannot stack, cannot merge into occupied slots,
/// and always move as single units. Used in trade, merchant, warehouse,
/// and NPC shop validation.
pub const ITEM_KIND_UNIQUE: i32 = 255;

/// Pet item kind (`isPetItem()` → `m_bKind == 151`).
pub const ITEM_KIND_PET: i32 = 151;

/// Cospre (costume) item kind
pub const ITEM_KIND_COSPRE: i32 = 252;

// ── Weapon kind constants ──────────────────────────────────────────────

pub const WEAPON_KIND_DAGGER: i32 = 11;
pub const WEAPON_KIND_1H_SWORD: i32 = 21;
pub const WEAPON_KIND_2H_SWORD: i32 = 22;
pub const WEAPON_KIND_1H_AXE: i32 = 31;
pub const WEAPON_KIND_2H_AXE: i32 = 32;
pub const WEAPON_KIND_1H_CLUB: i32 = 41;
pub const WEAPON_KIND_2H_CLUB: i32 = 42;
pub const WEAPON_KIND_1H_SPEAR: i32 = 51;
pub const WEAPON_KIND_2H_SPEAR: i32 = 52;
/// Shield kind
pub const WEAPON_KIND_SHIELD: i32 = 60;
pub const WEAPON_KIND_BOW: i32 = 70;
pub const WEAPON_KIND_CROSSBOW: i32 = 71;
pub const WEAPON_KIND_STAFF: i32 = 110;
pub const WEAPON_KIND_JAMADAR: i32 = 140;
pub const WEAPON_KIND_MACE: i32 = 181;

#[cfg(test)]
mod tests {
    use super::*;

    /// Inventory size layout — sniffer verified: 96 items.
    #[test]
    fn test_inventory_size_layout() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(COSP_MAX, 11);
        assert_eq!(MBAG_MAX, 12);
        assert_eq!(MBAG_COUNT, 3);
        assert_eq!(KNIGHT_ROYALE_MAX, 7);
        assert_eq!(INVENTORY_COSP, SLOT_MAX + HAVE_MAX); // 42
        assert_eq!(INVENTORY_MBAG, INVENTORY_COSP + COSP_MAX); // 53
        assert_eq!(INVENTORY_TOTAL, 96); // sniffer verified: 14+28+11+36+7
    }

    /// Equipment slot indices cover 0-13 (SLOT_MAX=14).
    #[test]
    fn test_equipment_slot_range() {
        assert_eq!(RIGHTEAR, 0);
        assert_eq!(HEAD, 1);
        assert_eq!(LEFTEAR, 2);
        assert_eq!(NECK, 3);
        assert_eq!(BREAST, 4);
        assert_eq!(SHOULDER, 5);
        assert_eq!(RIGHTHAND, 6);
        assert_eq!(WAIST, 7);
        assert_eq!(LEFTHAND, 8);
        assert_eq!(RIGHTRING, 9);
        assert_eq!(LEG, 10);
        assert_eq!(LEFTRING, 11);
        assert_eq!(GLOVE, 12);
        assert_eq!(FOOT, 13);
        assert!(FOOT < SLOT_MAX);
    }

    /// Weapon kind constants: 1H and 2H variants differ by 1.
    #[test]
    fn test_weapon_kind_pairs() {
        assert_eq!(WEAPON_KIND_1H_SWORD, 21);
        assert_eq!(WEAPON_KIND_2H_SWORD, 22);
        assert_eq!(WEAPON_KIND_1H_AXE, 31);
        assert_eq!(WEAPON_KIND_2H_AXE, 32);
        assert_eq!(WEAPON_KIND_1H_CLUB, 41);
        assert_eq!(WEAPON_KIND_2H_CLUB, 42);
        assert_eq!(WEAPON_KIND_1H_SPEAR, 51);
        assert_eq!(WEAPON_KIND_2H_SPEAR, 52);
    }

    /// COIN_MAX matches C++ value (2.1 billion).
    #[test]
    fn test_coin_max() {
        assert_eq!(COIN_MAX, 2_100_000_000);
        assert!(COIN_MAX < u32::MAX);
    }

    /// Warehouse: 192 slots = 8 pages * 24 items.
    #[test]
    fn test_warehouse_layout() {
        assert_eq!(WAREHOUSE_MAX, 192);
        assert_eq!(ITEMS_PER_PAGE, 24);
        assert_eq!(WAREHOUSE_MAX, 8 * ITEMS_PER_PAGE);
    }

    // ── Sprint 938: Additional coverage ──────────────────────────────

    /// Item kind constants: unique, pet, cospre.
    #[test]
    fn test_item_kind_values() {
        assert_eq!(ITEM_KIND_UNIQUE, 255);
        assert_eq!(ITEM_KIND_PET, 151);
        assert_eq!(ITEM_KIND_COSPRE, 252);
    }

    /// Cospre and magic bag offsets are derived from prior constants.
    #[test]
    fn test_derived_offsets() {
        assert_eq!(INVENTORY_COSP, 42); // 14 + 28
        assert_eq!(INVENTORY_MBAG, 53); // 42 + 11
    }

    /// Left/right equipment pairs are symmetric.
    #[test]
    fn test_equipment_pairs() {
        // Ears: 0 and 2
        assert_eq!(RIGHTEAR, 0);
        assert_eq!(LEFTEAR, 2);
        // Rings: 9 and 11
        assert_eq!(RIGHTRING, 9);
        assert_eq!(LEFTRING, 11);
        // Hands: 6 and 8
        assert_eq!(RIGHTHAND, 6);
        assert_eq!(LEFTHAND, 8);
    }

    /// INVENTORY_TOTAL equals sum of all sections (sniffer verified: 96).
    #[test]
    fn test_total_equals_sum() {
        // sniffer: 14 + 28 + 11 + 36 + 7 = 96
        assert_eq!(
            INVENTORY_TOTAL,
            SLOT_MAX + HAVE_MAX + COSP_MAX + MBAG_MAX * MBAG_COUNT + KNIGHT_ROYALE_MAX
        );
    }

    /// Shield kind is 60, separate from weapon ranges.
    #[test]
    fn test_shield_and_ranged_kinds() {
        assert_eq!(WEAPON_KIND_SHIELD, 60);
        assert_eq!(WEAPON_KIND_BOW, 70);
        assert_eq!(WEAPON_KIND_CROSSBOW, 71);
        assert_eq!(WEAPON_KIND_DAGGER, 11);
    }
}
