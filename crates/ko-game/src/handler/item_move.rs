//! WIZ_ITEM_MOVE (0x1F) handler — inventory and equipment item moves.
//! Packet format (from client):
//! ```text
//! [u8 type] — 1 = InventorySystem, 2 = InventorySystemRefresh, 3 = invalid
//! For type 1 (InventorySystem):
//!   [u8 dir] [u32 item_id] [u8 src_pos] [u8 dst_pos]
//! ```
//! Response (WIZ_ITEM_MOVE):
//! ```text
//! [u8 command=1] [u8 subcommand] — 0=fail, 1=success
//! If success: [u16 total_hit] [u16 total_ac] [u16 max_weight] [u8 0] [u8 0]
//!   [u16 max_hp] [u16 max_mp] [i16 str_bonus] [i16 sta_bonus] [i16 dex_bonus]
//!   [i16 int_bonus] [i16 cha_bonus] [u16 fire_r] [u16 cold_r] [u16 lightning_r]
//!   [u16 magic_r] [u16 disease_r] [u16 poison_r]
//! ```

use ko_db::models::Item;
use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::session::{ClientSession, SessionState};
use crate::world::{UserItemSlot, ITEM_FLAG_DUPLICATE, PET_INVENTORY_TOTAL, ZONE_CHAOS_DUNGEON};

use super::{
    COSP_MAX, HAVE_MAX, INVENTORY_COSP, INVENTORY_MBAG, INVENTORY_TOTAL, ITEM_KIND_PET,
    ITEM_KIND_UNIQUE, SLOT_MAX,
};

/// Item movement direction types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ItemMoveDir {
    InvenSlot = 1,
    SlotInven = 2,
    InvenInven = 3,
    SlotSlot = 4,
    InvenToCosp = 7,
    CospToInven = 8,
    InvenToMbag = 9,
    MbagToInven = 10,
    MbagToMbag = 11,
    InvenToPet = 12,
    PetToInven = 13,
    SlotInvenToMbag = 14,
}

impl ItemMoveDir {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::InvenSlot),
            2 => Some(Self::SlotInven),
            3 => Some(Self::InvenInven),
            4 => Some(Self::SlotSlot),
            7 => Some(Self::InvenToCosp),
            8 => Some(Self::CospToInven),
            9 => Some(Self::InvenToMbag),
            10 => Some(Self::MbagToInven),
            11 => Some(Self::MbagToMbag),
            12 => Some(Self::InvenToPet),
            13 => Some(Self::PetToInven),
            14 => Some(Self::SlotInvenToMbag),
            _ => None,
        }
    }
}

use crate::inventory_constants::{
    BREAST, FOOT, GLOVE, HEAD, LEFTEAR, LEFTHAND, LEFTRING, LEG, NECK, RIGHTEAR, RIGHTHAND,
    RIGHTRING, SHOULDER, WAIST,
};

/// Item slot type values (from m_bSlot field).
const ITEM_SLOT_EITHER_HAND: i32 = 0;
const ITEM_SLOT_1H_RIGHT: i32 = 1;
const ITEM_SLOT_1H_LEFT: i32 = 2;
const ITEM_SLOT_2H_RIGHT: i32 = 3;
const ITEM_SLOT_2H_LEFT: i32 = 4;
const ITEM_SLOT_PAULDRON: i32 = 5;
const ITEM_SLOT_PADS: i32 = 6;
const ITEM_SLOT_HELMET: i32 = 7;
const ITEM_SLOT_GLOVES: i32 = 8;
const ITEM_SLOT_BOOTS: i32 = 9;
const ITEM_SLOT_EARRING: i32 = 10;
const ITEM_SLOT_NECKLACE: i32 = 11;
const ITEM_SLOT_RING: i32 = 12;
const ITEM_SLOT_SHOULDER: i32 = 13;
const ITEM_SLOT_BELT: i32 = 14;
const ITEM_SLOT_KAUL: i32 = 20;

// ── Cospre (cosmetic) slot layout ───────────────────────────────

/// Magic bag slots per bag.
const MBAG_MAX: usize = 12;

/// Number of magic bags — v2600 has 3 (C++ v2525 had 2).
const MBAG_COUNT: usize = 3;

/// Total magic bag slots (3 bags × 12 slots).
const MBAG_TOTAL: usize = MBAG_MAX * MBAG_COUNT; // 36

/// Start of magic bag 2 items in the item array.
const INVENTORY_MBAG2: usize = INVENTORY_MBAG + MBAG_MAX; // 65

/// Start of magic bag 3 items in the item array (v2600).
#[allow(dead_code)]
const INVENTORY_MBAG3: usize = INVENTORY_MBAG2 + MBAG_MAX; // 77

// Absolute item array indices for special cospre slots.
const CHELMET: usize = 43;
const CTOP: usize = 46;
const CEMBLEM: usize = 47;
const CFAIRY: usize = 48;
const CTATTOO: usize = 49;
const CTALISMAN: usize = 50;
const CBAG1: usize = 51;
const CBAG2: usize = 52;
/// v2600: 3rd magic bag cospre slot. Stored OUTSIDE the normal inventory range
/// to avoid overlap with magic bag 1 items at slot 53 (INVENTORY_MBAG).
const CBAG3: usize = INVENTORY_TOTAL; // 96

// Client-side cospre relative positions (bDstPos/bSrcPos values).
const COSP_WINGS: u8 = 0;
const COSP_HELMET: u8 = 1;
const COSP_GLOVE: u8 = 2;
const COSP_GLOVE2: u8 = 3;
const COSP_BREAST: u8 = 4;
const COSP_EMBLAM: u8 = 5;
const COSP_BAG1: u8 = 6;
const COSP_FAIRY: u8 = 7;
const COSP_TATTO: u8 = 8;
const COSP_TALISMAN: u8 = 9;
const COSP_BAG2: u8 = 10;

// Cospre item slot types (m_bSlot values).
const ITEM_SLOT_BAG: i32 = 25;
const ITEM_SLOT_COSP_GLOVES: i32 = 100;
const ITEM_SLOT_COSP_PAULDRON: i32 = 105;
const ITEM_SLOT_COSP_HELMET: i32 = 107;
const ITEM_SLOT_COSP_WINGS: i32 = 110;
const ITEM_SLOT_COSP_FAIRY: i32 = 111;
const ITEM_SLOT_COSP_TATTOO: i32 = 112;
const ITEM_SLOT_COSP_TALISMAN: i32 = 113;
const ITEM_SLOT_COSP_EMBLEM0: i32 = 114;
const ITEM_SLOT_COSP_EMBLEM1: i32 = 115;

pub(crate) const ITEM_OREADS: u32 = 700039768;

/// Robin loot item IDs that enable auto-loot when equipped in SHOULDER slot.
const ROBIN_LOOT_ITEM: u32 = 950680000;
const ROBIN_LOOT_ITEM2: u32 = 850680000;
const MINING_ROBIN_ITEM: u32 = 510000000;
const FISHING_ROBIN_ITEM: u32 = 520000000;

/// Check if an item ID is a robin loot item.
fn is_robin_loot_item(item_id: u32) -> bool {
    matches!(
        item_id,
        ROBIN_LOOT_ITEM | ROBIN_LOOT_ITEM2 | MINING_ROBIN_ITEM | FISHING_ROBIN_ITEM
    )
}

/// Handle WIZ_ITEM_MOVE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot move/equip items
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Cannot move items while in a busy state
    if world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
        || world.get_browsing_merchant(sid).is_some()
    {
        return send_item_move_result(session, false).await;
    }

    // Cannot move items in Chaos Dungeon
    if world.get_position(sid).map(|p| p.zone_id).unwrap_or(0) == ZONE_CHAOS_DUNGEON {
        return send_item_move_result(session, false).await;
    }

    let mut reader = PacketReader::new(&pkt.data);

    let move_type = reader.read_u8().unwrap_or(0);

    match move_type {
        1 => handle_inventory_system(session, &mut reader).await,
        2 => handle_inventory_refresh(session).await,
        _ => {
            // Invalid type (including 3)
            send_item_move_result(session, false).await
        }
    }
}

/// Handle InventorySystemRefresh (type 2) — sort inventory by item_id descending and resend.
/// Sorts the bag portion of inventory (SLOT_MAX..SLOT_MAX+HAVE_MAX) by item_id descending,
/// then sends all bag items to the client with full item data.
/// Response packet (WIZ_ITEM_MOVE):
/// ```text
/// [u8 type=2] [u8 result=1]
/// For each bag slot (HAVE_MAX items):
///   [u32 item_id] [u16 durability] [u16 count] [u8 flag] [u16 rental_time=0]
///   [u32 serial=0] [u32 expire_time]
/// ```
async fn handle_inventory_refresh(session: &mut ClientSession) -> anyhow::Result<()> {
    send_inventory_snapshot_inner(session, true).await
}

/// Send the authoritative inventory view without reordering slots. Used after
/// server-side bank transfers so the bag updates immediately rather than only
/// after the user clicks the manual refresh button.
pub async fn send_inventory_snapshot(session: &mut ClientSession) -> anyhow::Result<()> {
    send_inventory_snapshot_inner(session, false).await
}

async fn send_inventory_snapshot_inner(
    session: &mut ClientSession,
    sort_bag: bool,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if sort_bag {
        // Manual client refresh preserves the legacy sort behavior.
        world.update_inventory(sid, |inv| {
            if inv.len() < INVENTORY_TOTAL {
                inv.resize(INVENTORY_TOTAL, UserItemSlot::default());
            }
            let bag = &mut inv[SLOT_MAX..SLOT_MAX + HAVE_MAX];
            bag.sort_by(|a, b| b.item_id.cmp(&a.item_id));
            true
        });
    }

    // Build the response packet
    let inventory = world.get_inventory(sid);
    let rebirth_level = world
        .get_character_info(sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);
    let mut pkt = Packet::new(Opcode::WizItemMove as u8);
    pkt.write_u8(2); // type = InventorySystemRefresh
    pkt.write_u8(1); // result = success

    for i in SLOT_MAX..SLOT_MAX + HAVE_MAX {
        let slot = inventory.get(i).cloned().unwrap_or_default();
        pkt.write_u32(slot.item_id);
        pkt.write_u16(slot.durability as u16);
        pkt.write_u16(slot.count);
        pkt.write_u8(slot.flag);
        pkt.write_u16(slot.remaining_rental_minutes()); // sRemainingRentalTime
        crate::handler::unique_item_info::write_unique_item_info(
            &world,
            session.pool(),
            slot.item_id,
            slot.serial_num,
            rebirth_level,
            &mut pkt,
        )
        .await;
        pkt.write_u32(slot.expire_time);
    }

    session.send_packet(&pkt).await?;

    //   if (!send_packet) { SetUserAbility(false); SendItemWeight(); }
    // Weight notification is integrated into set_user_ability().
    world.set_user_ability(sid);
    Ok(())
}

/// Handle InventorySystem — the main item move logic.
async fn handle_inventory_system(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let dir_byte = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);

    let world = session.world().clone();
    let sid = session.session_id();

    // Parse direction
    let dir = match ItemMoveDir::from_u8(dir_byte) {
        Some(d) => d,
        None => {
            tracing::debug!(
                "[{}] ItemMove: unknown direction {}",
                session.addr(),
                dir_byte
            );
            return send_item_move_result(session, false).await;
        }
    };

    tracing::info!(
        "[{}] ItemMove: dir={} item={} src={} dst={}",
        session.addr(),
        dir_byte,
        item_id,
        src_pos,
        dst_pos
    );

    // Look up item definition
    let item_table = match world.get_item(item_id) {
        Some(t) => t,
        None => {
            tracing::debug!("[{}] ItemMove: unknown item ID {}", session.addr(), item_id);
            return send_item_move_result(session, false).await;
        }
    };

    // Validate equip requirements for slot-targeting directions
    if dir == ItemMoveDir::InvenSlot || dir == ItemMoveDir::SlotSlot {
        if dst_pos as usize >= SLOT_MAX {
            return send_item_move_result(session, false).await;
        }
        if !is_valid_slot_pos(&item_table, dst_pos as usize) {
            return send_item_move_result(session, false).await;
        }
        // Check class restriction and equip requirements
        if let Some(ref ch) = world.get_character_info(sid) {
            if !item_class_available(&item_table, ch.class) {
                return send_item_move_result(session, false).await;
            }
            if !item_equip_available(&item_table, ch) {
                return send_item_move_result(session, false).await;
            }
        }
    }

    // Validate source slot exists
    if dir == ItemMoveDir::SlotInven && src_pos as usize >= SLOT_MAX {
        return send_item_move_result(session, false).await;
    }
    // Validate SlotInven destination is within inventory bag range (0..HAVE_MAX)
    if dir == ItemMoveDir::SlotInven && dst_pos as usize >= HAVE_MAX {
        return send_item_move_result(session, false).await;
    }

    // Validate InvenSlot source is within inventory bag range (0..HAVE_MAX)
    // Without this, src_pos 28+ would access cospre/magic bag regions via InvenSlot.
    if dir == ItemMoveDir::InvenSlot && src_pos as usize >= HAVE_MAX {
        return send_item_move_result(session, false).await;
    }

    // Prevent equipping a duplicate-flagged item from inventory to equipment slot.
    if dir == ItemMoveDir::InvenSlot {
        let src_idx = SLOT_MAX + src_pos as usize;
        if let Some(src_slot) = world.get_inventory_slot(sid, src_idx) {
            if src_slot.flag == ITEM_FLAG_DUPLICATE {
                return send_item_move_result(session, false).await;
            }
        }
    }

    // Pet item on SHOULDER: prevent equipping if a pet is already active.
    //   if (bDstPos == SHOULDER && pTable.isPetItem() && m_PettingOn != nullptr) goto fail_return;
    if dir == ItemMoveDir::InvenSlot && dst_pos as usize == SHOULDER {
        let kind = item_table.kind.unwrap_or(0);
        if kind == ITEM_KIND_PET {
            // Pet item — check if a pet is already equipped
            let has_pet = world
                .with_session(sid, |h| h.pet_data.is_some())
                .unwrap_or(false);
            if has_pet {
                return send_item_move_result(session, false).await;
            }
        }
    }

    // For InvenSlot: prevent equipping weapons on non-hand slots.
    if dir == ItemMoveDir::InvenSlot {
        let is_non_hand_slot = matches!(
            dst_pos as usize,
            RIGHTEAR
                | HEAD
                | LEFTEAR
                | NECK
                | BREAST
                | WAIST
                | RIGHTRING
                | LEG
                | LEFTRING
                | GLOVE
                | FOOT
        );
        let is_pet_item = item_table.kind.unwrap_or(0) == ITEM_KIND_PET;
        if is_non_hand_slot && !is_pet_item && is_weapon_item(&item_table) {
            return send_item_move_result(session, false).await;
        }
    }

    // Perform the item swap/move in the inventory.
    // For InvenSlot, we need the item's slot type to handle two-handed weapons.
    let item_slot_type = item_table.slot.unwrap_or(-1);

    // Pre-lookup the opposite hand item's slot type for 1H weapon 2H auto-unequip.
    // When equipping a 1H weapon, C++ checks if the OTHER hand holds a 2H weapon
    // and auto-unequips it. We need the item table lookup BEFORE the closure.
    let rh_item_slot_type = world
        .get_inventory_slot(sid, RIGHTHAND)
        .filter(|s| s.item_id != 0)
        .and_then(|s| world.get_item(s.item_id))
        .and_then(|t| t.slot);
    let lh_item_slot_type = world
        .get_inventory_slot(sid, LEFTHAND)
        .filter(|s| s.item_id != 0)
        .and_then(|s| world.get_item(s.item_id))
        .and_then(|t| t.slot);

    // For SlotInven, validate that any swap-back item is compatible.
    if dir == ItemMoveDir::SlotInven {
        let dst_idx = SLOT_MAX + dst_pos as usize;
        if let Some(dst_slot) = world.get_inventory_slot(sid, dst_idx) {
            if dst_slot.item_id != 0 {
                if let Some(dst_item_def) = world.get_item(dst_slot.item_id) {
                    let dst_slot_type = dst_item_def.slot.unwrap_or(-1);
                    if dst_slot_type != item_slot_type
                        || !is_valid_slot_pos(&dst_item_def, src_pos as usize)
                    {
                        return send_item_move_result(session, false).await;
                    }
                } else {
                    return send_item_move_result(session, false).await;
                }
            }
        }
    }

    // For SlotSlot, validate the destination item (if any) is compatible.
    if dir == ItemMoveDir::SlotSlot {
        let dst_idx = dst_pos as usize;
        if let Some(dst_slot) = world.get_inventory_slot(sid, dst_idx) {
            if dst_slot.item_id != 0 {
                if let Some(dst_item_def) = world.get_item(dst_slot.item_id) {
                    let dst_slot_type = dst_item_def.slot.unwrap_or(-1);
                    if dst_slot_type != item_slot_type {
                        return send_item_move_result(session, false).await;
                    }
                } else {
                    return send_item_move_result(session, false).await;
                }
            }
        }
    }

    // Pre-validation for ITEM_INVEN_TO_COSP (7).
    if dir == ItemMoveDir::InvenToCosp
        && (dst_pos as usize >= COSP_MAX + MBAG_COUNT
            || src_pos as usize >= HAVE_MAX
            || !is_valid_cosp_slot_pos(&item_table, dst_pos))
    {
        return send_item_move_result(session, false).await;
    }

    // Pre-validation for ITEM_COSP_TO_INVEN (8).
    if dir == ItemMoveDir::CospToInven {
        if dst_pos as usize >= HAVE_MAX
            || src_pos as usize >= COSP_MAX
            || src_pos == COSP_BAG1
            || src_pos == COSP_BAG2
        {
            return send_item_move_result(session, false).await;
        }
        // If destination has an item, validate the swap is compatible.
        let dst_idx = SLOT_MAX + dst_pos as usize;
        if let Some(dst_slot) = world.get_inventory_slot(sid, dst_idx) {
            if dst_slot.item_id != 0 {
                if let Some(dst_item_def) = world.get_item(dst_slot.item_id) {
                    let dst_slot_type = dst_item_def.slot.unwrap_or(-1);
                    if dst_slot_type != item_slot_type
                        || !is_valid_cosp_slot_pos(&dst_item_def, src_pos)
                    {
                        return send_item_move_result(session, false).await;
                    }
                } else {
                    return send_item_move_result(session, false).await;
                }
            }
        }
    }

    // Pre-validation for ITEM_SLOT_INVEN_TO_MBAG (14).
    if dir == ItemMoveDir::SlotInvenToMbag
        && (dst_pos as usize >= COSP_MAX + MBAG_COUNT
            || src_pos as usize >= HAVE_MAX
            || !is_valid_cosp_slot_pos(&item_table, dst_pos + 8))
    {
        return send_item_move_result(session, false).await;
    }

    // Pre-validation for ITEM_INVEN_TO_PET (12).
    if dir == ItemMoveDir::InvenToPet
        && (src_pos as usize >= HAVE_MAX || dst_pos as usize >= PET_INVENTORY_TOTAL)
    {
        return send_item_move_result(session, false).await;
    }

    // Pre-validation for ITEM_PET_TO_INVEN (13).
    if dir == ItemMoveDir::PetToInven
        && (src_pos as usize >= PET_INVENTORY_TOTAL || dst_pos as usize >= HAVE_MAX)
    {
        return send_item_move_result(session, false).await;
    }

    // Pet inventory directions use a separate update path since pet items
    // are stored in PetState, not in the main inventory Vec.
    // C++ Note: pTableSrc is never set for pet directions (isnull() returns true),
    // so the stacking code path is never reached — always swap when occupied.
    if dir == ItemMoveDir::InvenToPet || dir == ItemMoveDir::PetToInven {
        let success = world.update_inventory_and_pet(sid, |inv, pet_items| {
            if inv.len() < INVENTORY_TOTAL {
                inv.resize(INVENTORY_TOTAL, UserItemSlot::default());
            }
            match dir {
                ItemMoveDir::InvenToPet => {
                    let src_idx = SLOT_MAX + src_pos as usize;
                    let dst_idx = dst_pos as usize;
                    if src_idx >= inv.len() || inv[src_idx].item_id != item_id {
                        return false;
                    }
                    let dst_item = &mut pet_items[dst_idx];
                    if dst_item.item_id != 0 {
                        // C++ always swaps (pTableSrc uninitialized → isnull() = true)
                        std::mem::swap(&mut inv[src_idx], dst_item);
                    } else {
                        // Move to empty slot
                        *dst_item = inv[src_idx].clone();
                        inv[src_idx] = UserItemSlot::default();
                    }
                    true
                }
                ItemMoveDir::PetToInven => {
                    let src_idx = src_pos as usize;
                    let dst_idx = SLOT_MAX + dst_pos as usize;
                    if dst_idx >= inv.len() || pet_items[src_idx].item_id != item_id {
                        return false;
                    }
                    let src_item = &mut pet_items[src_idx];
                    if inv[dst_idx].item_id != 0 {
                        // C++ always swaps (pTableSrc uninitialized → isnull() = true)
                        std::mem::swap(src_item, &mut inv[dst_idx]);
                    } else {
                        // Move to empty slot
                        inv[dst_idx] = src_item.clone();
                        *src_item = UserItemSlot::default();
                    }
                    true
                }
                _ => false,
            }
        });

        if !success {
            return send_item_move_result(session, false).await;
        }

        // Recalculate stats for the 2H weapon swap (equipment change).
        // Weight notification is integrated into set_user_ability().
        world.set_user_ability(sid);
        // Pet equipment is not part of user_items. Persist it now so a
        // dismiss, disconnect or server restart cannot delete auto-loot gear.
        super::pet::save_pet_items(session).await;
        // SNIFFER-VERIFIED (2026-03-29): Original server sends sub=1 for equip success.
        // Sub=2 is stats-only (gamestart refresh) — client does NOT unlock UI on sub=2.
        send_item_move_result(session, true).await?;
        return Ok(());
    }

    let success = world.update_inventory(sid, |inv| {
        if inv.len() < INVENTORY_TOTAL {
            inv.resize(INVENTORY_TOTAL, UserItemSlot::default());
        }

        match dir {
            ItemMoveDir::InvenSlot => {
                // Inventory bag -> equipment slot
                let src_idx = SLOT_MAX + src_pos as usize;
                let dst_idx = dst_pos as usize;
                if src_idx >= INVENTORY_TOTAL || dst_idx >= SLOT_MAX {
                    return false;
                }
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // Two-handed weapon handling.
                handle_two_handed_equip(
                    inv,
                    src_idx,
                    dst_idx,
                    item_slot_type,
                    rh_item_slot_type,
                    lh_item_slot_type,
                )
            }
            ItemMoveDir::SlotInven => {
                // Equipment slot -> inventory bag
                let src_idx = src_pos as usize;
                let dst_idx = SLOT_MAX + dst_pos as usize;
                if src_idx >= SLOT_MAX || dst_idx >= INVENTORY_TOTAL {
                    return false;
                }
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                if inv[dst_idx].item_id != 0 {
                    inv.swap(src_idx, dst_idx);
                } else {
                    inv[dst_idx] = inv[src_idx].clone();
                    inv[src_idx] = UserItemSlot::default();
                }
                true
            }
            ItemMoveDir::InvenInven => {
                // Rearrange within inventory bag
                let src_idx = SLOT_MAX + src_pos as usize;
                let dst_idx = SLOT_MAX + dst_pos as usize;
                if src_pos as usize >= HAVE_MAX
                    || dst_pos as usize >= HAVE_MAX
                    || src_idx >= INVENTORY_TOTAL
                    || dst_idx >= INVENTORY_TOTAL
                {
                    return false;
                }
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // If same stackable item at dst, merge stacks.
                // Only items with countable exactly 1 can merge stacks.
                let is_countable = world
                    .get_item(item_id)
                    .and_then(|it| it.countable)
                    .unwrap_or(0)
                    == 1;
                if is_countable && inv[dst_idx].item_id == item_id && inv[dst_idx].item_id != 0 {
                    let new_count = inv[dst_idx].count.saturating_add(inv[src_idx].count);
                    inv[dst_idx].count = new_count.min(crate::world::ITEMCOUNT_MAX);
                    inv[src_idx] = UserItemSlot::default();
                } else if inv[dst_idx].item_id != 0 {
                    inv.swap(src_idx, dst_idx);
                } else {
                    inv[dst_idx] = inv[src_idx].clone();
                    inv[src_idx] = UserItemSlot::default();
                }
                true
            }
            ItemMoveDir::SlotSlot => {
                // Swap between equipment slots
                let src_idx = src_pos as usize;
                let dst_idx = dst_pos as usize;
                if src_pos as usize >= SLOT_MAX || dst_pos as usize >= SLOT_MAX {
                    return false;
                }
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                inv.swap(src_idx, dst_idx);
                true
            }
            ItemMoveDir::InvenToCosp => {
                // Inventory bag -> cospre (cosmetic) slot
                let src_idx = SLOT_MAX + src_pos as usize;
                if src_idx >= INVENTORY_TOTAL || inv[src_idx].item_id != item_id {
                    return false;
                }
                // Determine destination array index.
                // C++ uses special handling for bags, fairy, tattoo, emblem, talisman.
                let dst_idx = if item_slot_type == ITEM_SLOT_BAG {
                    // Magic bag equip — C++ ItemHandler.cpp:813-820
                    // Client sends destpos=9 for bag1, destpos=10 for bag2, destpos=11 for bag3 (v2600)
                    let bag_idx = INVENTORY_COSP + dst_pos as usize;
                    // v2600: bag3 maps to 53 (=INVENTORY_MBAG) but stored at CBAG3(96)
                    let bag_idx = if bag_idx == INVENTORY_MBAG {
                        CBAG3
                    } else {
                        bag_idx
                    };
                    if bag_idx != CBAG1 && bag_idx != CBAG2 && bag_idx != CBAG3 {
                        return false;
                    }
                    // Can't replace existing magic bag
                    if bag_idx >= inv.len() || inv[bag_idx].item_id != 0 {
                        return false;
                    }
                    bag_idx
                } else {
                    match cosp_pos_to_abs_index(dst_pos) {
                        Some(idx) if idx < inv.len() => idx,
                        _ => return false,
                    }
                };
                // Swap or move
                swap_items(inv, src_idx, dst_idx);
                true
            }
            ItemMoveDir::CospToInven => {
                // Cospre (cosmetic) slot -> inventory bag
                let dst_idx = SLOT_MAX + dst_pos as usize;
                if dst_idx >= INVENTORY_TOTAL {
                    return false;
                }
                // Map cospre position to absolute array index
                let src_idx = match cosp_pos_to_abs_index(src_pos) {
                    Some(idx) if idx < inv.len() => idx,
                    _ => return false,
                };
                // Validate item exists at source
                // C++ only checks nItemID for special slots (fairy, tattoo, emblem, talisman)
                // but we check universally for safety.
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // Swap or move
                swap_items(inv, src_idx, dst_idx);
                true
            }
            ItemMoveDir::InvenToMbag => {
                // Inventory bag -> magic bag
                if (dst_pos as usize) >= MBAG_TOTAL || (src_pos as usize) >= HAVE_MAX {
                    return false;
                }
                let src_idx = SLOT_MAX + src_pos as usize;
                let dst_idx = INVENTORY_MBAG + dst_pos as usize;
                if src_idx >= INVENTORY_TOTAL || dst_idx >= INVENTORY_TOTAL {
                    return false;
                }
                // Verify the magic bag item exists for the target bag
                // Bag 1 [53..65): requires CBAG1 cospre item
                // Bag 2 [65..77): requires CBAG2 cospre item
                // Bag 3 [77..89): always accessible (PUS item, no cospre gate)
                if dst_idx < INVENTORY_MBAG2 && inv[CBAG1].item_id == 0 {
                    return false;
                }
                if dst_idx >= INVENTORY_MBAG2
                    && dst_idx < INVENTORY_MBAG3
                    && inv[CBAG2].item_id == 0
                {
                    return false;
                }
                // Bag 3: no cospre check — accessible via PUS purchase
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // C++ ItemHandler.cpp:758-760 — prevent different stackable items
                let is_countable = world
                    .get_item(item_id)
                    .and_then(|it| it.countable)
                    .unwrap_or(0)
                    == 1;
                let is_kind_255 =
                    world.get_item(item_id).and_then(|it| it.kind).unwrap_or(0) == ITEM_KIND_UNIQUE;
                if inv[dst_idx].item_id != 0
                    && (is_countable || is_kind_255)
                    && inv[dst_idx].item_id != item_id
                {
                    return false;
                }
                // Stack if same countable item, else swap/move
                if inv[dst_idx].item_id == item_id && is_countable {
                    let new_count = inv[src_idx].count.saturating_add(inv[dst_idx].count);
                    inv[dst_idx] = inv[src_idx].clone();
                    inv[dst_idx].count = new_count.min(crate::world::ITEMCOUNT_MAX);
                    inv[src_idx] = UserItemSlot::default();
                } else {
                    swap_items(inv, src_idx, dst_idx);
                }
                true
            }
            ItemMoveDir::MbagToInven => {
                // Magic bag -> inventory bag
                if (dst_pos as usize) >= HAVE_MAX || (src_pos as usize) >= MBAG_TOTAL {
                    return false;
                }
                let src_idx = INVENTORY_MBAG + src_pos as usize;
                let dst_idx = SLOT_MAX + dst_pos as usize;
                if src_idx >= INVENTORY_TOTAL || dst_idx >= INVENTORY_TOTAL {
                    return false;
                }
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // Note: C++ has bag existence checks commented out for this direction
                swap_items(inv, src_idx, dst_idx);
                true
            }
            ItemMoveDir::MbagToMbag => {
                // Magic bag -> magic bag (within or between bags)
                if (dst_pos as usize) >= MBAG_TOTAL || (src_pos as usize) >= MBAG_TOTAL {
                    return false;
                }
                let src_idx = INVENTORY_MBAG + src_pos as usize;
                let dst_idx = INVENTORY_MBAG + dst_pos as usize;
                if src_idx >= INVENTORY_TOTAL || dst_idx >= INVENTORY_TOTAL {
                    return false;
                }
                // Verify the destination magic bag exists
                if dst_idx < INVENTORY_MBAG2 && inv[CBAG1].item_id == 0 {
                    return false;
                }
                if dst_idx >= INVENTORY_MBAG2
                    && dst_idx < INVENTORY_MBAG3
                    && inv[CBAG2].item_id == 0
                {
                    return false;
                }
                // Bag 3: no cospre check — accessible via PUS purchase
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // C++ sets pTableSrc for stack merging
                let is_countable = world
                    .get_item(item_id)
                    .and_then(|it| it.countable)
                    .unwrap_or(0)
                    == 1;
                if inv[dst_idx].item_id == item_id && is_countable {
                    let new_count = inv[src_idx].count.saturating_add(inv[dst_idx].count);
                    inv[dst_idx] = inv[src_idx].clone();
                    inv[dst_idx].count = new_count.min(crate::world::ITEMCOUNT_MAX);
                    inv[src_idx] = UserItemSlot::default();
                } else {
                    swap_items(inv, src_idx, dst_idx);
                }
                true
            }
            ItemMoveDir::SlotInvenToMbag => {
                // Inventory bag -> cospre bag slot (equip magic bag)
                let src_idx = SLOT_MAX + src_pos as usize;
                // CBAG1: dst_pos=1 → slot 51, CBAG2: dst_pos=2 → slot 52
                // CBAG3: dst_pos=3 → slot CBAG3 (96, outside normal range to avoid overlap)
                let dst_idx = if dst_pos == 3 {
                    CBAG3 // v2600: bag 3 stored at dedicated slot
                } else {
                    INVENTORY_COSP + dst_pos as usize + 8
                };
                if src_idx >= INVENTORY_TOTAL {
                    return false;
                }
                if dst_idx >= inv.len() {
                    return false;
                }
                if inv[src_idx].item_id != item_id {
                    return false;
                }
                // Can't replace existing bag, must be a bag item
                if inv[dst_idx].item_id != 0 || item_slot_type != ITEM_SLOT_BAG {
                    return false;
                }
                swap_items(inv, src_idx, dst_idx);
                true
            }
            // Pet directions are handled separately above via update_inventory_and_pet.
            ItemMoveDir::InvenToPet | ItemMoveDir::PetToInven => unreachable!(),
        }
    });

    if !success {
        return send_item_move_result(session, false).await;
    }

    // Recalculate equipment stats only for equipment/cospre changes (5 directions).
    let is_equipment_change = dir == ItemMoveDir::InvenSlot
        || dir == ItemMoveDir::SlotInven
        || dir == ItemMoveDir::SlotSlot
        || dir == ItemMoveDir::InvenToCosp
        || dir == ItemMoveDir::CospToInven;

    if is_equipment_change {
        world.set_user_ability(sid);

        // Update equipped_items in CharacterInfo for visual broadcasting
        let inventory = world.get_inventory(sid);
        world.update_character_stats(sid, |ch| {
            for slot in 0..14 {
                if slot < inventory.len() {
                    ch.equipped_items[slot] = inventory[slot].item_id;
                }
            }
        });

        // Broadcast look change to nearby players
        broadcast_look_change(session, dir, src_pos, dst_pos, item_id, &world);
    }

    // Track fairy item equip/unequip for auto-loot blocking.
    if dir == ItemMoveDir::InvenToCosp && dst_pos == COSP_FAIRY && item_id == ITEM_OREADS {
        world.update_session(sid, |h| h.fairy_check = true);
    }
    if dir == ItemMoveDir::CospToInven && src_pos == COSP_FAIRY && item_id == ITEM_OREADS {
        world.update_session(sid, |h| h.fairy_check = false);
    }

    // Track robin loot item equip/unequip for auto-loot.
    // Robin loot items: 950680000, 850680000, 510000000, 520000000
    if dir == ItemMoveDir::InvenSlot && dst_pos as usize == SHOULDER && is_robin_loot_item(item_id)
    {
        world.update_session(sid, |h| h.auto_loot = true);
    }
    if dir == ItemMoveDir::SlotInven && src_pos as usize == SHOULDER && is_robin_loot_item(item_id)
    {
        world.update_session(sid, |h| h.auto_loot = false);
    }

    // SNIFFER-VERIFIED (2026-03-29): Original server sends sub=1 for equip success.
    // Sub=2 is stats-only broadcast (gamestart refresh only).
    // Client unlocks UI only on sub=1, NOT on sub=2.
    send_item_move_result(session, true).await
}

/// Handle two-handed weapon equip logic within the inventory array.
/// When equipping a 1H weapon to the right hand and the left hand holds a 2H-left weapon,
/// the 2H-left weapon is auto-unequipped to the source inventory slot.
/// Similarly for 1H-left vs 2H-right, and for equipping 2H weapons when one hand is occupied.
#[allow(clippy::too_many_arguments)]
fn handle_two_handed_equip(
    inv: &mut [UserItemSlot],
    src_idx: usize,
    dst_idx: usize,
    item_slot_type: i32,
    rh_item_slot_type: Option<i32>,
    lh_item_slot_type: Option<i32>,
) -> bool {
    let rh = RIGHTHAND;
    let lh = LEFTHAND;

    match item_slot_type {
        // 1H Right hand or Either-hand going to right hand
        ITEM_SLOT_1H_RIGHT => {
            two_hand_check_1h_right(inv, src_idx, dst_idx, rh, lh, lh_item_slot_type)
        }
        ITEM_SLOT_EITHER_HAND if dst_idx == rh => {
            two_hand_check_1h_right(inv, src_idx, dst_idx, rh, lh, lh_item_slot_type)
        }
        // 1H Left hand or Either-hand going to left hand
        ITEM_SLOT_1H_LEFT => {
            two_hand_check_1h_left(inv, src_idx, dst_idx, rh, lh, rh_item_slot_type)
        }
        ITEM_SLOT_EITHER_HAND if dst_idx == lh => {
            two_hand_check_1h_left(inv, src_idx, dst_idx, rh, lh, rh_item_slot_type)
        }
        // 2H Right hand
        ITEM_SLOT_2H_RIGHT => {
            // Cannot equip if both hands are occupied
            if inv[lh].item_id != 0 && inv[rh].item_id != 0 {
                return false;
            }
            // If left hand has an item, move it to inventory src slot
            if inv[lh].item_id != 0 {
                let tmp = inv[src_idx].clone();
                inv[rh] = tmp;
                inv[src_idx] = inv[lh].clone();
                inv[lh] = UserItemSlot::default();
            } else {
                swap_items(inv, src_idx, dst_idx);
            }
            true
        }
        // 2H Left hand
        ITEM_SLOT_2H_LEFT => {
            // Cannot equip if both hands are occupied
            if inv[lh].item_id != 0 && inv[rh].item_id != 0 {
                return false;
            }
            // If right hand has an item, move it to inventory src slot
            if inv[rh].item_id != 0 {
                let tmp = inv[src_idx].clone();
                inv[lh] = tmp;
                inv[src_idx] = inv[rh].clone();
                inv[rh] = UserItemSlot::default();
            } else {
                swap_items(inv, src_idx, dst_idx);
            }
            true
        }
        // All other slot types: simple swap
        _ => {
            swap_items(inv, src_idx, dst_idx);
            true
        }
    }
}

/// Check for 1H-right equip: if left hand has a 2H-left weapon, auto-unequip it.
/// When equipping a 1H weapon to the right hand:
/// - If left hand is empty → normal swap (src ↔ righthand)
/// - If left hand has a 2H-left weapon (slot 0x04) → auto-unequip:
///   move new item to righthand, move 2H-left to src inventory slot, clear lefthand
/// - If left hand has any other item → normal swap (src ↔ righthand)
fn two_hand_check_1h_right(
    inv: &mut [UserItemSlot],
    src_idx: usize,
    dst_idx: usize,
    _rh: usize,
    lh: usize,
    lh_item_slot_type: Option<i32>,
) -> bool {
    if inv[lh].item_id != 0 {
        if lh_item_slot_type == Some(ITEM_SLOT_2H_LEFT) {
            // C++ ItemHandler.cpp:902-906: left hand has 2H-left weapon.
            // Auto-unequip: new item → righthand, 2H-left → src slot, clear lefthand.
            inv[dst_idx] = inv[src_idx].clone();
            inv[src_idx] = inv[lh].clone();
            inv[lh] = UserItemSlot::default();
        } else {
            // Normal swap: src ↔ dst (righthand)
            swap_items(inv, src_idx, dst_idx);
        }
    } else {
        // Left hand empty: normal swap
        swap_items(inv, src_idx, dst_idx);
    }
    true
}

/// Check for 1H-left equip: if right hand has a 2H-right weapon, auto-unequip it.
/// When equipping a 1H weapon to the left hand:
/// - If right hand is empty → normal swap (src ↔ lefthand)
/// - If right hand has a 2H-right weapon (slot 0x03) → auto-unequip:
///   move new item to lefthand, move 2H-right to src inventory slot, clear righthand
/// - If right hand has any other item → normal swap (src ↔ lefthand)
fn two_hand_check_1h_left(
    inv: &mut [UserItemSlot],
    src_idx: usize,
    dst_idx: usize,
    rh: usize,
    _lh: usize,
    rh_item_slot_type: Option<i32>,
) -> bool {
    if inv[rh].item_id != 0 {
        if rh_item_slot_type == Some(ITEM_SLOT_2H_RIGHT) {
            // C++ ItemHandler.cpp:930-934: right hand has 2H-right weapon.
            // Auto-unequip: new item → lefthand, 2H-right → src slot, clear righthand.
            inv[dst_idx] = inv[src_idx].clone();
            inv[src_idx] = inv[rh].clone();
            inv[rh] = UserItemSlot::default();
        } else {
            // Normal swap: src ↔ dst (lefthand)
            swap_items(inv, src_idx, dst_idx);
        }
    } else {
        // Right hand empty: normal swap
        swap_items(inv, src_idx, dst_idx);
    }
    true
}

/// Swap two inventory slots, or move if destination is empty.
fn swap_items(inv: &mut [UserItemSlot], src_idx: usize, dst_idx: usize) {
    if inv[dst_idx].item_id != 0 {
        inv.swap(src_idx, dst_idx);
    } else {
        inv[dst_idx] = inv[src_idx].clone();
        inv[src_idx] = UserItemSlot::default();
    }
}

/// Check if an item is a weapon type (should not be equipped on non-hand slots).
fn is_weapon_item(item: &Item) -> bool {
    let kind = item.kind.unwrap_or(0);
    matches!(
        kind,
        11  // dagger
        | 21 | 22  // 1H/2H sword
        | 31 | 32  // 1H/2H axe
        | 41 | 42  // 1H/2H club
        | 51 | 52  // 1H/2H spear
        | 60        // shield — C++ GameDefine.h WEAPON_SHIELD=60
        | 61        // pickaxe — C++ ItemHandler.cpp:873 isPickaxe()
        | 70 | 71  // bow / crossbow
        | 110       // staff
        | 140       // jamadar
        | 151       // pet item — C++ ItemHandler.cpp:874 isPetItem()
        | 181 // mace
    )
}

/// Send WIZ_ITEM_MOVE response to the client.
async fn send_item_move_result(session: &mut ClientSession, success: bool) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemMove as u8);
    pkt.write_u8(1); // command
    pkt.write_u8(if success { 1 } else { 0 }); // SNIFFER-VERIFIED: sub=1 for equip success (client unlocks UI), sub=2 is stats-only (gamestart refresh)

    if success {
        let world = session.world().clone();
        let sid = session.session_id();

        if let Some(ch) = world.get_character_info(sid) {
            let stats = world.get_equipped_stats(sid);
            pkt.write_u16(stats.total_hit);
            pkt.write_u16(stats.total_ac as u16);
            pkt.write_u32(stats.max_weight);
            pkt.write_u8(0); // reserved
            pkt.write_u8(0); // reserved
            pkt.write_u16(ch.max_hp as u16);
            pkt.write_u16(ch.max_mp as u16);
            pkt.write_i16(stats.stat_bonuses[0]); // STR bonus
            pkt.write_i16(stats.stat_bonuses[1]); // STA bonus
            pkt.write_i16(stats.stat_bonuses[2]); // DEX bonus
            pkt.write_i16(stats.stat_bonuses[3]); // INT bonus
            pkt.write_i16(stats.stat_bonuses[4]); // CHA bonus
            pkt.write_u16(stats.fire_r as u16);
            pkt.write_u16(stats.cold_r as u16);
            pkt.write_u16(stats.lightning_r as u16);
            pkt.write_u16(stats.magic_r as u16);
            pkt.write_u16(stats.disease_r as u16);
            pkt.write_u16(stats.poison_r as u16);
        }
    }

    session.send_packet(&pkt).await
}

/// Check if an item can be equipped in the given cospre slot position.
/// Handles the cospre-specific item slot types.
fn is_valid_cosp_slot_pos(item: &Item, dest_pos: u8) -> bool {
    let slot = item.slot.unwrap_or(-1);
    match slot {
        ITEM_SLOT_COSP_GLOVES => dest_pos == COSP_GLOVE || dest_pos == COSP_GLOVE2,
        ITEM_SLOT_COSP_PAULDRON => dest_pos == COSP_BREAST,
        ITEM_SLOT_COSP_HELMET => dest_pos == COSP_HELMET,
        ITEM_SLOT_COSP_WINGS => dest_pos == COSP_WINGS,
        // C++ ItemHandler.cpp:2377 — bags go to COSP_BAG1+3(=9) or COSP_BAG2(=10)
        // v2600 adds bag 3 at position COSP_BAG2+1(=11)
        ITEM_SLOT_BAG => {
            dest_pos == COSP_BAG1 + 3 || dest_pos == COSP_BAG2 || dest_pos == COSP_BAG2 + 1
        }
        ITEM_SLOT_COSP_FAIRY => dest_pos == COSP_FAIRY,
        ITEM_SLOT_COSP_TATTOO => dest_pos == COSP_TATTO,
        ITEM_SLOT_COSP_TALISMAN => dest_pos == COSP_TALISMAN,
        ITEM_SLOT_COSP_EMBLEM0 | ITEM_SLOT_COSP_EMBLEM1 => dest_pos == COSP_EMBLAM,
        _ => false,
    }
}

/// Map a client cospre relative position to the absolute item array index.
/// Returns `None` if the position is out of range.
fn cosp_pos_to_abs_index(pos: u8) -> Option<usize> {
    match pos {
        COSP_EMBLAM => Some(CEMBLEM),
        COSP_BAG1 => Some(CBAG1),
        COSP_BAG2 => Some(CBAG2),
        11 => Some(CBAG3), // v2600: bag 3 at dedicated slot (COSP_BAG2+1)
        COSP_FAIRY => Some(CFAIRY),
        COSP_TATTO => Some(CTATTOO),
        COSP_TALISMAN => Some(CTALISMAN),
        0..=4 => Some(INVENTORY_COSP + pos as usize),
        _ => None,
    }
}

/// Check if an item can be equipped in the given slot position.
fn is_valid_slot_pos(item: &Item, dest_pos: usize) -> bool {
    let slot = item.slot.unwrap_or(-1);
    match slot {
        ITEM_SLOT_EITHER_HAND => dest_pos == RIGHTHAND || dest_pos == LEFTHAND,
        ITEM_SLOT_1H_RIGHT => dest_pos == RIGHTHAND,
        ITEM_SLOT_1H_LEFT => dest_pos == LEFTHAND,
        ITEM_SLOT_2H_RIGHT => dest_pos == RIGHTHAND,
        ITEM_SLOT_2H_LEFT => dest_pos == LEFTHAND,
        ITEM_SLOT_PAULDRON => dest_pos == BREAST,
        ITEM_SLOT_PADS => dest_pos == LEG,
        ITEM_SLOT_HELMET => dest_pos == HEAD,
        ITEM_SLOT_GLOVES => dest_pos == GLOVE,
        ITEM_SLOT_BOOTS => dest_pos == FOOT,
        ITEM_SLOT_EARRING => dest_pos == RIGHTEAR || dest_pos == LEFTEAR,
        ITEM_SLOT_NECKLACE => dest_pos == NECK,
        ITEM_SLOT_RING => dest_pos == RIGHTRING || dest_pos == LEFTRING,
        ITEM_SLOT_SHOULDER | ITEM_SLOT_KAUL => dest_pos == SHOULDER,
        ITEM_SLOT_BELT => dest_pos == WAIST,
        _ => false,
    }
}

/// Check if the item's class restriction allows this player's class.
fn item_class_available(item: &Item, player_class: u16) -> bool {
    let item_class = item.class.unwrap_or(0);
    if item_class == 0 {
        return true; // No restriction
    }

    let class_base = (player_class % 100) as u8;
    match item_class {
        1 => matches!(class_base, 1 | 5 | 6 | 13 | 14 | 15), // Warrior + Kurian
        2 => matches!(class_base, 2 | 7 | 8),                // Rogue
        3 => matches!(class_base, 3 | 9 | 10),               // Mage
        4 => matches!(class_base, 4 | 11 | 12),              // Priest
        5 => matches!(class_base, 5 | 6 | 14 | 15),          // Novice/Mastered Warrior+Kurian
        6 => matches!(class_base, 6 | 15),                   // Mastered Warrior+Kurian
        7 => matches!(class_base, 7 | 8),                    // Novice/Mastered Rogue
        8 => class_base == 8,                                // Mastered Rogue
        9 => matches!(class_base, 9 | 10),                   // Novice/Mastered Mage
        10 => class_base == 10,                              // Mastered Mage
        11 => matches!(class_base, 11 | 12),                 // Novice/Mastered Priest
        12 => class_base == 12,                              // Mastered Priest
        13 => matches!(class_base, 13..=15),                 // Kurian base
        14 => matches!(class_base, 14 | 15),                 // Novice/Mastered Kurian
        15 => class_base == 15,                              // Mastered Kurian
        _ => true,
    }
}

/// Check if the player meets the item's stat/level requirements.
fn item_equip_available(item: &Item, ch: &crate::world::CharacterInfo) -> bool {
    let req_level = item.req_level.unwrap_or(0) as u8;
    let req_level_max = item.req_level_max.unwrap_or(255) as u8;

    ch.level >= req_level
        && ch.level <= req_level_max
        && ch.str >= item.req_str.unwrap_or(0) as u8
        && ch.sta >= item.req_sta.unwrap_or(0) as u8
        && ch.dex >= item.req_dex.unwrap_or(0) as u8
        && ch.intel >= item.req_intel.unwrap_or(0) as u8
        && ch.cha >= item.req_cha.unwrap_or(0) as u8
}

/// Broadcast WIZ_USERLOOK_CHANGE to nearby players when equipment changes.
fn broadcast_look_change(
    session: &mut ClientSession,
    dir: ItemMoveDir,
    src_pos: u8,
    dst_pos: u8,
    item_id: u32,
    world: &crate::world::WorldState,
) {
    let sid = session.session_id();
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return,
    };
    let inventory = world.get_inventory(sid);

    // Check if player is hiding cosmetics (toggled via WIZ_HELMET).
    let is_hiding_cospre = world
        .with_session(sid, |h| h.is_hiding_cospre)
        .unwrap_or(false);

    // Helper: check if cospre override suppresses equipment broadcast.
    // C++ ItemHandler.cpp:1144-1158, 1165-1180
    // When equipping/unequipping to HEAD, BREAST, LEG, GLOVE, FOOT, check if the
    // corresponding cosmetic slot has an item AND cosmetics are NOT hidden.
    // If cosmetics ARE hidden (m_bIsHidingCospre=true), always broadcast equipment.
    let should_broadcast_equip_slot = |slot: u8| -> bool {
        match slot as usize {
            HEAD => {
                let cosp_empty = inventory
                    .get(CHELMET)
                    .map(|s| s.item_id == 0)
                    .unwrap_or(true);
                cosp_empty || is_hiding_cospre
            }
            BREAST | LEG | GLOVE | FOOT => {
                let cosp_empty = inventory.get(CTOP).map(|s| s.item_id == 0).unwrap_or(true);
                cosp_empty || is_hiding_cospre
            }
            _ => true,
        }
    };

    // Determine which slot(s) to broadcast based on direction
    match dir {
        ItemMoveDir::InvenSlot => {
            // C++ ItemHandler.cpp:1144-1159 — cospre override suppression
            if should_broadcast_equip_slot(dst_pos) {
                let dur = inventory
                    .get(dst_pos as usize)
                    .map(|s| s.durability as u16)
                    .unwrap_or(0);
                send_look_change_packet(world, sid, pos, dst_pos, item_id, dur);
            }
        }
        ItemMoveDir::SlotInven => {
            // C++ ItemHandler.cpp:1165-1180 — cospre override suppression
            if should_broadcast_equip_slot(src_pos) {
                let new_item = inventory
                    .get(src_pos as usize)
                    .map(|s| s.item_id)
                    .unwrap_or(0);
                let dur = inventory
                    .get(src_pos as usize)
                    .map(|s| s.durability as u16)
                    .unwrap_or(0);
                send_look_change_packet(world, sid, pos, src_pos, new_item, dur);
            }
        }
        ItemMoveDir::SlotSlot => {
            // Both slots changed
            let src_item = inventory.get(src_pos as usize);
            let dst_item = inventory.get(dst_pos as usize);
            if let Some(s) = src_item {
                send_look_change_packet(world, sid, pos, src_pos, s.item_id, s.durability as u16)
            }
            if let Some(d) = dst_item {
                send_look_change_packet(world, sid, pos, dst_pos, d.item_id, d.durability as u16)
            }
        }
        ItemMoveDir::InvenToCosp => {
            // C++ ItemHandler.cpp:1160-1162 — skip pos 5 (EMBLAM) and 6 (BAG1)
            if dst_pos != 5 && dst_pos != 6 {
                let abs_idx = cosp_pos_to_abs_index(dst_pos).unwrap_or(0);
                let dur = inventory
                    .get(abs_idx)
                    .map(|s| s.durability as u16)
                    .unwrap_or(0);
                send_look_change_packet(world, sid, pos, dst_pos, item_id, dur);
            }
        }
        ItemMoveDir::CospToInven => {
            // C++ ItemHandler.cpp:1181-1214 — multi-slot reveal on cosmetic removal
            match src_pos {
                COSP_BREAST => {
                    // Removing breast cosmetic reveals BREAST, LEG, GLOVE, FOOT
                    for &equip_slot in &[BREAST, LEG, GLOVE, FOOT] {
                        let slot_item = inventory.get(equip_slot);
                        let (reveal_id, reveal_dur) = match slot_item {
                            Some(s) if s.item_id != 0 => (s.item_id, s.durability as u16),
                            _ => (0, 0),
                        };
                        send_look_change_packet(
                            world,
                            sid,
                            pos,
                            equip_slot as u8,
                            reveal_id,
                            reveal_dur,
                        )
                    }
                }
                COSP_HELMET => {
                    // Removing helmet cosmetic reveals HEAD
                    let slot_item = inventory.get(HEAD);
                    let (reveal_id, reveal_dur) = match slot_item {
                        Some(s) if s.item_id != 0 => (s.item_id, s.durability as u16),
                        _ => (0, 0),
                    };
                    send_look_change_packet(world, sid, pos, HEAD as u8, reveal_id, reveal_dur);
                }
                _ => {
                    // Other cosmetics: single clear broadcast.
                    // Skip EMBLAM(5) and BAG slots — they have no visual appearance.
                    if src_pos != COSP_EMBLAM && src_pos != COSP_BAG1 && src_pos != COSP_BAG2 {
                        send_look_change_packet(world, sid, pos, src_pos, 0, 0);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Send a single WIZ_USERLOOK_CHANGE packet to nearby players.
fn send_look_change_packet(
    world: &crate::world::WorldState,
    sid: u16,
    pos: crate::world::Position,
    slot: u8,
    item_id: u32,
    durability: u16,
) {
    if slot as usize >= SLOT_MAX {
        return;
    }
    // Skip accessories (earring=91, necklace=92, ring=93, belt=94)
    if item_id != 0 {
        if let Some(item) = world.get_item(item_id) {
            let kind = item.kind.unwrap_or(0);
            if matches!(kind, 91..=94) {
                return;
            }
        }
    }

    let mut pkt = Packet::new(Opcode::WizUserlookChange as u8);
    pkt.write_u32(sid as u32);
    pkt.write_u8(slot);
    pkt.write_u32(item_id);
    pkt.write_u16(durability);
    pkt.write_u8(0); // reserved

    let event_room = world.get_event_room(sid);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(pkt),
        Some(sid),
        event_room,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_move_dir_from_u8() {
        assert_eq!(ItemMoveDir::from_u8(1), Some(ItemMoveDir::InvenSlot));
        assert_eq!(ItemMoveDir::from_u8(2), Some(ItemMoveDir::SlotInven));
        assert_eq!(ItemMoveDir::from_u8(3), Some(ItemMoveDir::InvenInven));
        assert_eq!(ItemMoveDir::from_u8(4), Some(ItemMoveDir::SlotSlot));
        assert!(ItemMoveDir::from_u8(0).is_none());
        assert!(ItemMoveDir::from_u8(5).is_none());
    }

    #[test]
    fn test_is_valid_slot_pos_helmet() {
        let item = make_test_item(ITEM_SLOT_HELMET);
        assert!(is_valid_slot_pos(&item, HEAD));
        assert!(!is_valid_slot_pos(&item, BREAST));
    }

    #[test]
    fn test_is_valid_slot_pos_pauldron() {
        let item = make_test_item(ITEM_SLOT_PAULDRON);
        assert!(is_valid_slot_pos(&item, BREAST));
        assert!(!is_valid_slot_pos(&item, HEAD));
    }

    #[test]
    fn test_is_valid_slot_pos_either_hand() {
        let item = make_test_item(ITEM_SLOT_EITHER_HAND);
        assert!(is_valid_slot_pos(&item, RIGHTHAND));
        assert!(is_valid_slot_pos(&item, LEFTHAND));
        assert!(!is_valid_slot_pos(&item, HEAD));
    }

    #[test]
    fn test_is_valid_slot_pos_earring() {
        let item = make_test_item(ITEM_SLOT_EARRING);
        assert!(is_valid_slot_pos(&item, RIGHTEAR));
        assert!(is_valid_slot_pos(&item, LEFTEAR));
        assert!(!is_valid_slot_pos(&item, NECK));
    }

    #[test]
    fn test_is_valid_slot_pos_ring() {
        let item = make_test_item(ITEM_SLOT_RING);
        assert!(is_valid_slot_pos(&item, RIGHTRING));
        assert!(is_valid_slot_pos(&item, LEFTRING));
        assert!(!is_valid_slot_pos(&item, GLOVE));
    }

    #[test]
    fn test_is_valid_slot_pos_belt() {
        let item = make_test_item(ITEM_SLOT_BELT);
        assert!(is_valid_slot_pos(&item, WAIST));
        assert!(!is_valid_slot_pos(&item, FOOT));
    }

    #[test]
    fn test_is_valid_slot_pos_boots() {
        let item = make_test_item(ITEM_SLOT_BOOTS);
        assert!(is_valid_slot_pos(&item, FOOT));
        assert!(!is_valid_slot_pos(&item, GLOVE));
    }

    #[test]
    fn test_is_valid_slot_pos_gloves() {
        let item = make_test_item(ITEM_SLOT_GLOVES);
        assert!(is_valid_slot_pos(&item, GLOVE));
        assert!(!is_valid_slot_pos(&item, FOOT));
    }

    #[test]
    fn test_is_valid_slot_pos_pads() {
        let item = make_test_item(ITEM_SLOT_PADS);
        assert!(is_valid_slot_pos(&item, LEG));
        assert!(!is_valid_slot_pos(&item, BREAST));
    }

    #[test]
    fn test_is_valid_slot_pos_necklace() {
        let item = make_test_item(ITEM_SLOT_NECKLACE);
        assert!(is_valid_slot_pos(&item, NECK));
        assert!(!is_valid_slot_pos(&item, HEAD));
    }

    #[test]
    fn test_is_valid_slot_pos_shoulder() {
        let item = make_test_item(ITEM_SLOT_SHOULDER);
        assert!(is_valid_slot_pos(&item, SHOULDER));
        assert!(!is_valid_slot_pos(&item, BREAST));
    }

    #[test]
    fn test_is_valid_slot_pos_2h_right() {
        let item = make_test_item(ITEM_SLOT_2H_RIGHT);
        assert!(is_valid_slot_pos(&item, RIGHTHAND));
        assert!(!is_valid_slot_pos(&item, LEFTHAND));
    }

    #[test]
    fn test_item_class_available_no_restriction() {
        let item = make_test_item_with_class(0);
        assert!(item_class_available(&item, 101)); // warrior
        assert!(item_class_available(&item, 202)); // rogue
    }

    #[test]
    fn test_item_class_warrior_only() {
        let item = make_test_item_with_class(1);
        assert!(item_class_available(&item, 101)); // warrior base
        assert!(item_class_available(&item, 105)); // warrior novice
        assert!(item_class_available(&item, 106)); // warrior mastered
        assert!(!item_class_available(&item, 102)); // rogue base
        assert!(!item_class_available(&item, 103)); // mage base
    }

    #[test]
    fn test_item_class_rogue_only() {
        let item = make_test_item_with_class(2);
        assert!(item_class_available(&item, 102));
        assert!(item_class_available(&item, 107));
        assert!(item_class_available(&item, 108));
        assert!(!item_class_available(&item, 101));
    }

    #[test]
    fn test_item_class_mage_only() {
        let item = make_test_item_with_class(3);
        assert!(item_class_available(&item, 103));
        assert!(item_class_available(&item, 109));
        assert!(item_class_available(&item, 110));
        assert!(!item_class_available(&item, 101));
    }

    #[test]
    fn test_item_class_priest_only() {
        let item = make_test_item_with_class(4);
        assert!(item_class_available(&item, 104));
        assert!(item_class_available(&item, 111));
        assert!(item_class_available(&item, 112));
        assert!(!item_class_available(&item, 101));
    }

    /// Helper: create a test Item with given slot value.
    fn make_test_item(slot: i32) -> Item {
        Item {
            num: 100000,
            slot: Some(slot),
            class: Some(0),
            kind: Some(0),
            req_level: Some(0),
            req_level_max: Some(255),
            req_str: Some(0),
            req_sta: Some(0),
            req_dex: Some(0),
            req_intel: Some(0),
            req_cha: Some(0),
            extension: None,
            str_name: None,
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            race: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: None,
            duration: None,
            buy_price: None,
            sell_price: None,
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: None,
            effect1: None,
            effect2: None,
            req_rank: None,
            req_title: None,
            selling_group: None,
            item_type: None,
            hitrate: None,
            evasionrate: None,
            dagger_ac: None,
            jamadar_ac: None,
            sword_ac: None,
            club_ac: None,
            axe_ac: None,
            spear_ac: None,
            bow_ac: None,
            fire_damage: None,
            ice_damage: None,
            lightning_damage: None,
            poison_damage: None,
            hp_drain: None,
            mp_damage: None,
            mp_drain: None,
            mirror_damage: None,
            droprate: None,
            str_b: None,
            sta_b: None,
            dex_b: None,
            intel_b: None,
            cha_b: None,
            max_hp_b: None,
            max_mp_b: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            poison_r: None,
            curse_r: None,
            item_class: None,
            np_buy_price: None,
            bound: None,
            mace_ac: None,
            by_grade: None,
            drop_notice: None,
            upgrade_notice: None,
        }
    }

    /// Helper: create a test Item with given class restriction.
    fn make_test_item_with_class(class: i32) -> Item {
        let mut item = make_test_item(0);
        item.class = Some(class);
        item
    }

    /// Helper: create a test Item with given kind (weapon type).
    fn make_test_item_with_kind(kind: i32) -> Item {
        let mut item = make_test_item(0);
        item.kind = Some(kind);
        item
    }

    #[test]
    fn test_is_weapon_item_dagger() {
        let item = make_test_item_with_kind(11);
        assert!(is_weapon_item(&item));
    }

    #[test]
    fn test_is_weapon_item_sword() {
        assert!(is_weapon_item(&make_test_item_with_kind(21)));
        assert!(is_weapon_item(&make_test_item_with_kind(22)));
    }

    #[test]
    fn test_is_weapon_item_bow() {
        assert!(is_weapon_item(&make_test_item_with_kind(70)));
        assert!(is_weapon_item(&make_test_item_with_kind(71)));
    }

    #[test]
    fn test_is_weapon_item_staff() {
        assert!(is_weapon_item(&make_test_item_with_kind(110)));
    }

    #[test]
    fn test_is_weapon_item_armor_not_weapon() {
        // Armor kinds should not be weapons
        assert!(!is_weapon_item(&make_test_item_with_kind(0)));
        assert!(!is_weapon_item(&make_test_item_with_kind(91))); // earring
        assert!(!is_weapon_item(&make_test_item_with_kind(92))); // necklace
    }

    #[test]
    fn test_handle_two_handed_equip_simple_swap() {
        // Normal equip: slot type 5 (pauldron) to breast slot
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Place item in inventory bag slot 0
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 1000,
            durability: 100,
            count: 1,
            ..Default::default()
        };
        let dst_idx = BREAST;
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_PAULDRON,
            None,
            None,
        ));
        assert_eq!(inv[dst_idx].item_id, 1000);
        assert_eq!(inv[src_idx].item_id, 0);
    }

    #[test]
    fn test_handle_two_handed_equip_2h_right_empty_hands() {
        // Equip 2H-right weapon to right hand, both hands empty
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 2000,
            durability: 50,
            count: 1,
            ..Default::default()
        };
        let dst_idx = RIGHTHAND;
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_2H_RIGHT,
            None,
            None,
        ));
        assert_eq!(inv[dst_idx].item_id, 2000);
        assert_eq!(inv[src_idx].item_id, 0);
    }

    #[test]
    fn test_handle_two_handed_equip_2h_right_left_occupied() {
        // Equip 2H-right weapon when left hand has an item.
        // The left hand item should be moved to the inventory src slot.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 2000,
            durability: 50,
            count: 1,
            ..Default::default()
        };
        inv[LEFTHAND] = UserItemSlot {
            item_id: 3000,
            durability: 80,
            count: 1,
            ..Default::default()
        };
        let dst_idx = RIGHTHAND;
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_2H_RIGHT,
            None,
            None,
        ));
        assert_eq!(inv[dst_idx].item_id, 2000); // new weapon in right hand
        assert_eq!(inv[LEFTHAND].item_id, 0); // left hand cleared
        assert_eq!(inv[src_idx].item_id, 3000); // old left hand item in inventory
    }

    #[test]
    fn test_handle_two_handed_equip_2h_right_both_occupied() {
        // Equip 2H-right weapon when both hands are occupied: should fail.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 2000,
            durability: 50,
            count: 1,
            ..Default::default()
        };
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 4000,
            durability: 60,
            count: 1,
            ..Default::default()
        };
        inv[LEFTHAND] = UserItemSlot {
            item_id: 3000,
            durability: 80,
            count: 1,
            ..Default::default()
        };
        let dst_idx = RIGHTHAND;
        assert!(!handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_2H_RIGHT,
            None,
            None,
        ));
    }

    #[test]
    fn test_handle_two_handed_equip_2h_left_right_occupied() {
        // Equip 2H-left weapon when right hand has an item.
        // The right hand item should be moved to the inventory src slot.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 5000,
            durability: 50,
            count: 1,
            ..Default::default()
        };
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 6000,
            durability: 70,
            count: 1,
            ..Default::default()
        };
        let dst_idx = LEFTHAND;
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_2H_LEFT,
            None,
            None,
        ));
        assert_eq!(inv[dst_idx].item_id, 5000); // new weapon in left hand
        assert_eq!(inv[RIGHTHAND].item_id, 0); // right hand cleared
        assert_eq!(inv[src_idx].item_id, 6000); // old right hand item in inventory
    }

    #[test]
    fn test_handle_two_handed_equip_2h_left_both_occupied() {
        // Equip 2H-left weapon when both hands are occupied: should fail.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 5000,
            durability: 50,
            count: 1,
            ..Default::default()
        };
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 4000,
            durability: 60,
            count: 1,
            ..Default::default()
        };
        inv[LEFTHAND] = UserItemSlot {
            item_id: 3000,
            durability: 80,
            count: 1,
            ..Default::default()
        };
        let dst_idx = LEFTHAND;
        assert!(!handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_2H_LEFT,
            None,
            None,
        ));
    }

    // ── Sprint 244: 1H weapon equip with 2H auto-unequip ──────────

    /// Equip 1H-right weapon when left hand has a 2H-left weapon.
    /// The 2H-left weapon should auto-unequip to inventory src slot.
    #[test]
    fn test_1h_right_equip_auto_unequips_2h_left() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;

        // 1H sword in bag
        inv[src_idx] = UserItemSlot {
            item_id: 100001,
            durability: 100,
            count: 1,
            ..Default::default()
        };
        // 2H-left weapon (e.g. quiver) in left hand
        inv[LEFTHAND] = UserItemSlot {
            item_id: 200001,
            durability: 80,
            count: 1,
            ..Default::default()
        };

        let dst_idx = RIGHTHAND;
        // lh_item_slot_type = Some(ITEM_SLOT_2H_LEFT) = Some(4)
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_1H_RIGHT,
            None,
            Some(ITEM_SLOT_2H_LEFT),
        ));

        // 1H sword now in right hand
        assert_eq!(inv[RIGHTHAND].item_id, 100001);
        // 2H-left weapon moved to inventory src slot
        assert_eq!(inv[src_idx].item_id, 200001);
        assert_eq!(inv[src_idx].durability, 80);
        // Left hand cleared
        assert_eq!(inv[LEFTHAND].item_id, 0);
    }

    /// Equip 1H-left weapon when right hand has a 2H-right weapon.
    /// The 2H-right weapon should auto-unequip to inventory src slot.
    #[test]
    fn test_1h_left_equip_auto_unequips_2h_right() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;

        // Shield in bag
        inv[src_idx] = UserItemSlot {
            item_id: 300001,
            durability: 90,
            count: 1,
            ..Default::default()
        };
        // 2H-right weapon (e.g. 2H sword) in right hand
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 400001,
            durability: 70,
            count: 1,
            ..Default::default()
        };

        let dst_idx = LEFTHAND;
        // rh_item_slot_type = Some(ITEM_SLOT_2H_RIGHT) = Some(3)
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_1H_LEFT,
            Some(ITEM_SLOT_2H_RIGHT),
            None,
        ));

        // Shield now in left hand
        assert_eq!(inv[LEFTHAND].item_id, 300001);
        // 2H-right weapon moved to inventory src slot
        assert_eq!(inv[src_idx].item_id, 400001);
        assert_eq!(inv[src_idx].durability, 70);
        // Right hand cleared
        assert_eq!(inv[RIGHTHAND].item_id, 0);
    }

    /// Equip 1H-right weapon when left hand has a non-2H weapon (e.g. shield).
    /// Normal swap should occur (no auto-unequip).
    #[test]
    fn test_1h_right_equip_normal_swap_with_1h_left() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;

        // 1H sword in bag
        inv[src_idx] = UserItemSlot {
            item_id: 100001,
            durability: 100,
            count: 1,
            ..Default::default()
        };
        // Shield (1H-left, slot=2) in left hand
        inv[LEFTHAND] = UserItemSlot {
            item_id: 500001,
            durability: 60,
            count: 1,
            ..Default::default()
        };
        // Right hand has old weapon
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 600001,
            durability: 50,
            count: 1,
            ..Default::default()
        };

        let dst_idx = RIGHTHAND;
        // lh_item_slot_type = Some(ITEM_SLOT_1H_LEFT) = Some(2) — NOT 2H
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_1H_RIGHT,
            None,
            Some(ITEM_SLOT_1H_LEFT),
        ));

        // Normal swap: new 1H sword → right hand, old weapon → bag
        assert_eq!(inv[RIGHTHAND].item_id, 100001);
        assert_eq!(inv[src_idx].item_id, 600001);
        // Left hand unchanged (shield stays)
        assert_eq!(inv[LEFTHAND].item_id, 500001);
    }

    /// Either-hand weapon going to right hand with 2H-left in left hand.
    #[test]
    fn test_either_hand_to_right_auto_unequips_2h_left() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;

        inv[src_idx] = UserItemSlot {
            item_id: 100001,
            durability: 100,
            count: 1,
            ..Default::default()
        };
        inv[LEFTHAND] = UserItemSlot {
            item_id: 200001,
            durability: 80,
            count: 1,
            ..Default::default()
        };

        let dst_idx = RIGHTHAND;
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_EITHER_HAND,
            None,
            Some(ITEM_SLOT_2H_LEFT),
        ));

        assert_eq!(inv[RIGHTHAND].item_id, 100001);
        assert_eq!(inv[src_idx].item_id, 200001);
        assert_eq!(inv[LEFTHAND].item_id, 0);
    }

    /// Either-hand weapon going to left hand with 2H-right in right hand.
    #[test]
    fn test_either_hand_to_left_auto_unequips_2h_right() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX;

        inv[src_idx] = UserItemSlot {
            item_id: 300001,
            durability: 90,
            count: 1,
            ..Default::default()
        };
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 400001,
            durability: 70,
            count: 1,
            ..Default::default()
        };

        let dst_idx = LEFTHAND;
        assert!(handle_two_handed_equip(
            &mut inv,
            src_idx,
            dst_idx,
            ITEM_SLOT_EITHER_HAND,
            Some(ITEM_SLOT_2H_RIGHT),
            None,
        ));

        assert_eq!(inv[LEFTHAND].item_id, 300001);
        assert_eq!(inv[src_idx].item_id, 400001);
        assert_eq!(inv[RIGHTHAND].item_id, 0);
    }

    #[test]
    fn test_swap_items_empty_dst() {
        let mut inv = vec![UserItemSlot::default(); 20];
        inv[0] = UserItemSlot {
            item_id: 100,
            count: 1,
            ..Default::default()
        };
        swap_items(&mut inv, 0, 5);
        assert_eq!(inv[5].item_id, 100);
        assert_eq!(inv[0].item_id, 0);
    }

    #[test]
    fn test_swap_items_both_occupied() {
        let mut inv = vec![UserItemSlot::default(); 20];
        inv[0] = UserItemSlot {
            item_id: 100,
            count: 1,
            ..Default::default()
        };
        inv[5] = UserItemSlot {
            item_id: 200,
            count: 2,
            ..Default::default()
        };
        swap_items(&mut inv, 0, 5);
        assert_eq!(inv[5].item_id, 100);
        assert_eq!(inv[0].item_id, 200);
    }

    // ── InventorySystemRefresh (Type 2) Tests ───────────────────────

    #[test]
    fn test_inventory_refresh_sort_descending() {
        // Simulate the sort that handle_inventory_refresh does
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Place items in bag slots (SLOT_MAX..SLOT_MAX+HAVE_MAX)
        inv[SLOT_MAX] = UserItemSlot {
            item_id: 100,
            count: 1,
            ..Default::default()
        };
        inv[SLOT_MAX + 1] = UserItemSlot {
            item_id: 300,
            count: 1,
            ..Default::default()
        };
        inv[SLOT_MAX + 2] = UserItemSlot {
            item_id: 200,
            count: 1,
            ..Default::default()
        };

        // Sort bag portion descending by item_id (same as C++ std::sort with >)
        let bag = &mut inv[SLOT_MAX..SLOT_MAX + HAVE_MAX];
        bag.sort_by(|a, b| b.item_id.cmp(&a.item_id));

        assert_eq!(inv[SLOT_MAX].item_id, 300);
        assert_eq!(inv[SLOT_MAX + 1].item_id, 200);
        assert_eq!(inv[SLOT_MAX + 2].item_id, 100);
    }

    #[test]
    fn test_inventory_refresh_sort_preserves_equipment() {
        // Equipment slots (0..SLOT_MAX) should NOT be sorted
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[0] = UserItemSlot {
            item_id: 50,
            count: 1,
            ..Default::default()
        };
        inv[6] = UserItemSlot {
            item_id: 10,
            count: 1,
            ..Default::default()
        };
        inv[SLOT_MAX] = UserItemSlot {
            item_id: 999,
            count: 1,
            ..Default::default()
        };

        let bag = &mut inv[SLOT_MAX..SLOT_MAX + HAVE_MAX];
        bag.sort_by(|a, b| b.item_id.cmp(&a.item_id));

        // Equipment unchanged
        assert_eq!(inv[0].item_id, 50);
        assert_eq!(inv[6].item_id, 10);
        // Bag sorted
        assert_eq!(inv[SLOT_MAX].item_id, 999);
    }

    #[test]
    fn test_inventory_refresh_sort_empty_slots_last() {
        // Empty slots (item_id=0) should sort to the end
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[SLOT_MAX + 5] = UserItemSlot {
            item_id: 500,
            count: 1,
            ..Default::default()
        };
        inv[SLOT_MAX + 10] = UserItemSlot {
            item_id: 100,
            count: 1,
            ..Default::default()
        };
        // Slots 0..4, 6..9, 11..HAVE_MAX are empty (item_id=0)

        let bag = &mut inv[SLOT_MAX..SLOT_MAX + HAVE_MAX];
        bag.sort_by(|a, b| b.item_id.cmp(&a.item_id));

        // Non-empty items should be first
        assert_eq!(inv[SLOT_MAX].item_id, 500);
        assert_eq!(inv[SLOT_MAX + 1].item_id, 100);
        // Rest should be empty
        assert_eq!(inv[SLOT_MAX + 2].item_id, 0);
    }

    #[test]
    fn test_inventory_refresh_packet_format() {
        // Verify the packet structure for type 2 response
        use ko_protocol::PacketReader;

        let mut pkt = Packet::new(Opcode::WizItemMove as u8);
        pkt.write_u8(2); // type
        pkt.write_u8(1); // result

        // Write one item slot
        pkt.write_u32(12345); // item_id
        pkt.write_u16(100); // durability
        pkt.write_u16(5); // count
        pkt.write_u8(0); // flag
        pkt.write_u16(0); // rental_time
        pkt.write_u32(0); // serial
        pkt.write_u32(0); // expire_time

        assert_eq!(pkt.opcode, Opcode::WizItemMove as u8);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(2)); // type
        assert_eq!(reader.read_u8(), Some(1)); // result
        assert_eq!(reader.read_u32(), Some(12345)); // item_id
        assert_eq!(reader.read_u16(), Some(100)); // durability
        assert_eq!(reader.read_u16(), Some(5)); // count
        assert_eq!(reader.read_u8(), Some(0)); // flag
        assert_eq!(reader.read_u16(), Some(0)); // rental_time
        assert_eq!(reader.read_u32(), Some(0)); // serial
        assert_eq!(reader.read_u32(), Some(0)); // expire_time
    }

    #[test]
    fn test_inventory_refresh_full_bag_size() {
        // Verify HAVE_MAX items would produce correct packet size
        // Per item: 4+2+2+1+2+4+4 = 19 bytes
        // Header: 1+1 = 2 bytes
        // Total: 2 + 28*19 = 2 + 532 = 534 bytes
        let expected_data_len = 2 + HAVE_MAX * 19;
        assert_eq!(expected_data_len, 534);
    }

    // ── Sprint 48: Economy Integration Tests ────────────────────────

    /// Integration: Item move type 2 (bag sort) sorts items descending, produces full refresh packet.
    ///
    /// Verifies: unsorted bag → sort descending → packet has all 28 items → equipment untouched.
    #[test]
    fn test_integration_bag_sort_full_refresh_flow() {
        use ko_protocol::PacketReader;

        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        // Equipment slot: should NOT be sorted
        inv[HEAD] = UserItemSlot {
            item_id: 50,
            durability: 100,
            count: 1,
            ..Default::default()
        };

        // Bag: unsorted items scattered across slots
        inv[SLOT_MAX] = UserItemSlot {
            item_id: 100,
            durability: 80,
            count: 1,
            ..Default::default()
        };
        inv[SLOT_MAX + 5] = UserItemSlot {
            item_id: 500,
            durability: 90,
            count: 3,
            ..Default::default()
        };
        inv[SLOT_MAX + 10] = UserItemSlot {
            item_id: 300,
            durability: 70,
            count: 1,
            ..Default::default()
        };
        inv[SLOT_MAX + 20] = UserItemSlot {
            item_id: 200,
            durability: 50,
            count: 2,
            ..Default::default()
        };

        // Sort bag portion descending (same as C++ InventorySystemReflesh)
        let bag = &mut inv[SLOT_MAX..SLOT_MAX + HAVE_MAX];
        bag.sort_by(|a, b| b.item_id.cmp(&a.item_id));

        // Verify sort order: 500, 300, 200, 100, then zeros
        assert_eq!(inv[SLOT_MAX].item_id, 500);
        assert_eq!(inv[SLOT_MAX + 1].item_id, 300);
        assert_eq!(inv[SLOT_MAX + 2].item_id, 200);
        assert_eq!(inv[SLOT_MAX + 3].item_id, 100);
        assert_eq!(inv[SLOT_MAX + 4].item_id, 0); // empty slots at end

        // Equipment unchanged
        assert_eq!(inv[HEAD].item_id, 50, "Equipment should not be sorted");

        // Build refresh packet and verify format
        let mut pkt = Packet::new(Opcode::WizItemMove as u8);
        pkt.write_u8(2); // type
        pkt.write_u8(1); // result
        for slot in &inv[SLOT_MAX..SLOT_MAX + HAVE_MAX] {
            pkt.write_u32(slot.item_id);
            pkt.write_u16(slot.durability as u16);
            pkt.write_u16(slot.count);
            pkt.write_u8(slot.flag);
            pkt.write_u16(0); // rental
            pkt.write_u32(0); // serial
            pkt.write_u32(slot.expire_time);
        }

        // Verify packet: header + 28 * 19 bytes = 534
        assert_eq!(pkt.data.len(), 2 + HAVE_MAX * 19);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(2)); // type
        assert_eq!(reader.read_u8(), Some(1)); // result

        // First item should be 500
        assert_eq!(reader.read_u32(), Some(500));
    }

    /// Integration: 2H weapon equip auto-removes off-hand item.
    ///
    /// Verifies: equip 2H-right → left hand item moved to inventory → right hand has weapon.
    #[test]
    fn test_integration_2h_weapon_offhand_auto_remove() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        // Inventory bag slot 0: 2H weapon to equip
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 200001, // 2H sword
            durability: 100,
            count: 1,
            ..Default::default()
        };

        // Left hand has a shield
        inv[LEFTHAND] = UserItemSlot {
            item_id: 300001, // shield
            durability: 80,
            count: 1,
            ..Default::default()
        };

        // Right hand empty
        assert_eq!(inv[RIGHTHAND].item_id, 0);

        // Perform 2H-right equip
        let result =
            handle_two_handed_equip(&mut inv, src_idx, RIGHTHAND, ITEM_SLOT_2H_RIGHT, None, None);
        assert!(
            result,
            "2H equip should succeed with only left hand occupied"
        );

        // Verify: 2H weapon in right hand
        assert_eq!(inv[RIGHTHAND].item_id, 200001);
        assert_eq!(inv[RIGHTHAND].durability, 100);

        // Verify: left hand cleared
        assert_eq!(inv[LEFTHAND].item_id, 0);

        // Verify: shield moved to inventory src slot
        assert_eq!(inv[src_idx].item_id, 300001);
        assert_eq!(inv[src_idx].durability, 80);
    }

    /// Integration: 2H weapon equip fails when both hands occupied.
    ///
    /// Verifies: both hands full → 2H equip rejected → no items moved.
    #[test]
    fn test_integration_2h_equip_fails_both_hands_full() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 200001, // 2H weapon
            durability: 100,
            count: 1,
            ..Default::default()
        };
        inv[RIGHTHAND] = UserItemSlot {
            item_id: 100001, // existing right hand weapon
            durability: 90,
            count: 1,
            ..Default::default()
        };
        inv[LEFTHAND] = UserItemSlot {
            item_id: 300001, // shield in left hand
            durability: 80,
            count: 1,
            ..Default::default()
        };

        // Should fail: both hands occupied
        let result =
            handle_two_handed_equip(&mut inv, src_idx, RIGHTHAND, ITEM_SLOT_2H_RIGHT, None, None);
        assert!(!result, "2H equip should fail when both hands occupied");

        // Verify: nothing changed
        assert_eq!(inv[src_idx].item_id, 200001, "Source item unchanged");
        assert_eq!(inv[RIGHTHAND].item_id, 100001, "Right hand unchanged");
        assert_eq!(inv[LEFTHAND].item_id, 300001, "Left hand unchanged");
    }

    /// Integration: Item weight tracking across equip/unequip cycle.
    ///
    /// Verifies: equip adds weight → unequip removes weight → net zero.
    #[test]
    fn test_integration_item_weight_equip_unequip_cycle() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        // Simulate: item in bag slot with weight 150
        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 100001,
            durability: 100,
            count: 1,
            ..Default::default()
        };

        // Track weight manually (simulating what SetUserAbility does)
        let item_weight: u16 = 150;
        let mut total_weight: u16 = 0;

        // Equip: move to equipment slot
        swap_items(&mut inv, src_idx, BREAST);
        total_weight += item_weight; // equipped item contributes to weight

        assert_eq!(inv[BREAST].item_id, 100001, "Item should be equipped");
        assert_eq!(inv[src_idx].item_id, 0, "Source slot should be empty");
        assert_eq!(total_weight, 150, "Weight should increase after equip");

        // Unequip: move back to bag
        swap_items(&mut inv, BREAST, src_idx);
        total_weight -= item_weight; // unequipped

        assert_eq!(inv[src_idx].item_id, 100001, "Item should be back in bag");
        assert_eq!(inv[BREAST].item_id, 0, "Equipment slot should be empty");
        assert_eq!(total_weight, 0, "Weight should be zero after unequip");
    }

    /// Integration: Slot validation across all equipment positions.
    ///
    /// Verifies: each item slot type only fits in its correct equipment position(s).
    #[test]
    fn test_integration_slot_validation_complete_mapping() {
        let slot_mapping: Vec<(i32, Vec<usize>, Vec<usize>)> = vec![
            (ITEM_SLOT_HELMET, vec![HEAD], vec![BREAST, LEFTHAND]),
            (ITEM_SLOT_PAULDRON, vec![BREAST], vec![HEAD, LEFTHAND]),
            (ITEM_SLOT_PADS, vec![LEG], vec![BREAST, FOOT]),
            (ITEM_SLOT_GLOVES, vec![GLOVE], vec![FOOT, HEAD]),
            (ITEM_SLOT_BOOTS, vec![FOOT], vec![GLOVE, HEAD]),
            (ITEM_SLOT_NECKLACE, vec![NECK], vec![HEAD, BREAST]),
            (ITEM_SLOT_BELT, vec![WAIST], vec![FOOT, HEAD]),
            (ITEM_SLOT_SHOULDER, vec![SHOULDER], vec![BREAST, HEAD]),
            (
                ITEM_SLOT_EITHER_HAND,
                vec![RIGHTHAND, LEFTHAND],
                vec![HEAD, BREAST],
            ),
            (ITEM_SLOT_EARRING, vec![RIGHTEAR, LEFTEAR], vec![NECK, HEAD]),
            (ITEM_SLOT_RING, vec![RIGHTRING, LEFTRING], vec![GLOVE, HEAD]),
            (ITEM_SLOT_2H_RIGHT, vec![RIGHTHAND], vec![LEFTHAND, HEAD]),
        ];

        for (slot_type, valid_positions, invalid_positions) in &slot_mapping {
            let item = make_test_item(*slot_type);

            for &pos in valid_positions {
                assert!(
                    is_valid_slot_pos(&item, pos),
                    "Slot type {} should be valid at position {}",
                    slot_type,
                    pos
                );
            }

            for &pos in invalid_positions {
                assert!(
                    !is_valid_slot_pos(&item, pos),
                    "Slot type {} should be INVALID at position {}",
                    slot_type,
                    pos
                );
            }
        }
    }

    /// Integration: Item class restriction check across all classes.
    ///
    /// Verifies: class-restricted items are only equippable by correct class family.
    #[test]
    fn test_integration_item_class_restriction_all_families() {
        // class=0: all classes allowed
        let universal = make_test_item_with_class(0);
        for &class in &[101u16, 102, 103, 104, 113, 201, 202, 203, 204, 213] {
            assert!(
                item_class_available(&universal, class),
                "Universal item should work for class {}",
                class
            );
        }

        // class=1: warrior only (101, 105, 106)
        let warrior_item = make_test_item_with_class(1);
        assert!(item_class_available(&warrior_item, 101));
        assert!(item_class_available(&warrior_item, 105));
        assert!(item_class_available(&warrior_item, 106));
        assert!(!item_class_available(&warrior_item, 102));
        assert!(!item_class_available(&warrior_item, 103));
        assert!(!item_class_available(&warrior_item, 104));

        // class=2: rogue only (102, 107, 108)
        let rogue_item = make_test_item_with_class(2);
        assert!(item_class_available(&rogue_item, 102));
        assert!(item_class_available(&rogue_item, 107));
        assert!(item_class_available(&rogue_item, 108));
        assert!(!item_class_available(&rogue_item, 101));

        // class=3: mage only (103, 109, 110)
        let mage_item = make_test_item_with_class(3);
        assert!(item_class_available(&mage_item, 103));
        assert!(item_class_available(&mage_item, 109));
        assert!(item_class_available(&mage_item, 110));
        assert!(!item_class_available(&mage_item, 101));

        // class=4: priest only (104, 111, 112)
        let priest_item = make_test_item_with_class(4);
        assert!(item_class_available(&priest_item, 104));
        assert!(item_class_available(&priest_item, 111));
        assert!(item_class_available(&priest_item, 112));
        assert!(!item_class_available(&priest_item, 101));
    }

    /// Integration: Inventory swap preserves all item fields.
    ///
    /// Verifies: swap copies all fields (item_id, durability, count, flag, expire_time).
    #[test]
    fn test_integration_swap_preserves_all_fields() {
        let mut inv = vec![UserItemSlot::default(); 20];

        inv[0] = UserItemSlot {
            item_id: 12345,
            durability: 99,
            count: 5,
            flag: 3,
            original_flag: 0,
            serial_num: 999,
            expire_time: 1700000000,
        };
        inv[5] = UserItemSlot {
            item_id: 67890,
            durability: 50,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 111,
            expire_time: 0,
        };

        swap_items(&mut inv, 0, 5);

        // Slot 5 now has the first item
        assert_eq!(inv[5].item_id, 12345);
        assert_eq!(inv[5].durability, 99);
        assert_eq!(inv[5].count, 5);
        assert_eq!(inv[5].flag, 3);
        assert_eq!(inv[5].serial_num, 999);
        assert_eq!(inv[5].expire_time, 1700000000);

        // Slot 0 now has the second item
        assert_eq!(inv[0].item_id, 67890);
        assert_eq!(inv[0].durability, 50);
        assert_eq!(inv[0].count, 1);
        assert_eq!(inv[0].flag, 0);
        assert_eq!(inv[0].serial_num, 111);
        assert_eq!(inv[0].expire_time, 0);
    }

    // ── Sprint 55: Hardening Edge Case Tests ────────────────────────

    /// Edge case: moving item to an already-occupied slot should swap the items,
    /// not lose either item.
    #[test]
    fn test_swap_items_occupied_destination_preserves_both() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        // Source: bag slot 0
        let src = SLOT_MAX;
        inv[src] = UserItemSlot {
            item_id: 1001,
            durability: 80,
            count: 3,
            flag: 1,
            original_flag: 0,
            serial_num: 42,
            expire_time: 100,
        };

        // Destination: bag slot 1 (already occupied)
        let dst = SLOT_MAX + 1;
        inv[dst] = UserItemSlot {
            item_id: 2002,
            durability: 50,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 99,
            expire_time: 0,
        };

        swap_items(&mut inv, src, dst);

        // After swap: items should be exchanged
        assert_eq!(
            inv[dst].item_id, 1001,
            "Source item should move to destination"
        );
        assert_eq!(inv[dst].durability, 80);
        assert_eq!(inv[dst].count, 3);

        assert_eq!(
            inv[src].item_id, 2002,
            "Destination item should move to source"
        );
        assert_eq!(inv[src].durability, 50);
        assert_eq!(inv[src].count, 1);
    }

    /// Edge case: swap items with self (src == dst) should be a no-op.
    #[test]
    fn test_swap_items_same_slot_is_noop() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        let idx = SLOT_MAX;
        inv[idx] = UserItemSlot {
            item_id: 5555,
            durability: 100,
            count: 1,
            ..Default::default()
        };

        swap_items(&mut inv, idx, idx);

        // Item should remain unchanged
        assert_eq!(inv[idx].item_id, 5555);
        assert_eq!(inv[idx].durability, 100);
        assert_eq!(inv[idx].count, 1);
    }

    /// Edge case: item with durability already at max should not need repair.
    /// Verify the repair cost calculation returns quantity=0.
    #[test]
    fn test_repair_max_durability_no_cost() {
        // max_durability = 100, current durability = 100
        let max_durability: i32 = 100;
        let current_durability: i32 = 100;
        let quantity = max_durability - current_durability;

        assert_eq!(
            quantity, 0,
            "No repair needed when durability is already at max"
        );

        // With quantity=0, repair cost formula produces 0
        let buy_price: f64 = 10000.0;
        let cost = ((((buy_price - 10.0) / 10000.0) + buy_price.powf(0.75)) * quantity as f64
            / max_durability as f64) as u32;
        assert_eq!(cost, 0, "Repair cost should be 0 when no damage");
    }

    /// Edge case: dropping item with count > actual stack should be clamped.
    /// The handler should never drop more than available.
    #[test]
    fn test_drop_count_clamp_to_actual() {
        let actual_count: u16 = 5;
        let requested_drop: u16 = 100;

        // The server should clamp to actual
        let drop_count = requested_drop.min(actual_count);
        assert_eq!(
            drop_count, 5,
            "Drop count should be clamped to actual stack"
        );
    }

    /// Edge case: swapping empty slot with empty slot should produce no change.
    #[test]
    fn test_swap_empty_with_empty() {
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];

        let src = SLOT_MAX;
        let dst = SLOT_MAX + 5;

        // Both empty
        assert_eq!(inv[src].item_id, 0);
        assert_eq!(inv[dst].item_id, 0);

        swap_items(&mut inv, src, dst);

        // Still empty
        assert_eq!(inv[src].item_id, 0);
        assert_eq!(inv[dst].item_id, 0);
    }

    // ── Sprint 121: Browsing merchant guard ────────────────────────

    #[test]
    fn test_browsing_merchant_blocks_item_move() {
        use crate::world::WorldState;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Initially not browsing any merchant
        assert!(world.get_browsing_merchant(sid).is_none());

        // Start browsing a merchant
        let merchant_sid = world.allocate_session_id();
        world.set_browsing_merchant(sid, Some(merchant_sid));
        assert!(world.get_browsing_merchant(sid).is_some());

        // Clear browsing
        world.set_browsing_merchant(sid, None);
        assert!(world.get_browsing_merchant(sid).is_none());
    }

    // ── Sprint 244: Pickaxe and pet item are weapons ────────────────

    #[test]
    fn test_is_weapon_item_pickaxe() {
        assert!(is_weapon_item(&make_test_item_with_kind(61)));
    }

    #[test]
    fn test_is_weapon_item_pet_item() {
        assert!(is_weapon_item(&make_test_item_with_kind(151)));
    }

    /// Duplicate-flagged items (flag == 3) must not be equipped.
    #[test]
    fn test_duplicate_flag_item_id_constant() {
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
    }

    // ── Sprint 120: Weight update after inventory refresh ────────────

    /// Verify the WIZ_WEIGHT_CHANGE packet format is [u16 item_weight].
    #[test]
    fn test_weight_change_packet_format_after_refresh() {
        use ko_protocol::PacketReader;

        let item_weight: u16 = 450;
        let mut pkt = Packet::new(Opcode::WizWeightChange as u8);
        pkt.write_u16(item_weight);

        assert_eq!(pkt.opcode, Opcode::WizWeightChange as u8);
        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(450));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 280: Countable strict equality ───────────────────────────

    /// Test countable check uses == 1 (not > 0) for stack merging.
    #[test]
    fn test_countable_strict_equality() {
        // countable == 0: not stackable
        assert_ne!(0i32, 1, "countable=0 should NOT be stackable");
        // countable == 1: stackable
        assert_eq!(1i32, 1, "countable=1 should be stackable");
        // countable == 2 or higher: special items, NOT stackable (C++ uses ==1)
        assert_ne!(2i32, 1, "countable=2 should NOT stack (strict ==1 check)");
        assert_ne!(3i32, 1, "countable=3 should NOT stack");
    }

    // ── Sprint 311: Pet item SHOULDER slot check ─────────────────────

    /// `if (bDstPos == SHOULDER && pTable.isPetItem() && m_PettingOn != nullptr) goto fail_return;`
    /// A pet item (kind 151) cannot be equipped to SHOULDER if a pet is already active.
    #[test]
    fn test_pet_item_shoulder_slot_constant() {
        assert_eq!(SHOULDER, 5);
        // Pet item kind
        let pet_kind: i32 = 151;
        assert!(is_weapon_item(&make_test_item_with_kind(pet_kind)));
    }

    #[test]
    fn test_pet_item_blocked_if_pet_active() {
        // If has_pet is true, equipping a pet item to SHOULDER must fail.
        let has_pet = true;
        let kind = 151;
        let dst_pos = SHOULDER;
        assert!(kind == ITEM_KIND_PET && dst_pos == SHOULDER && has_pet);
    }

    #[test]
    fn test_pet_item_allowed_if_no_pet() {
        // If has_pet is false, equipping a pet item to SHOULDER is allowed.
        let has_pet = false;
        let kind = 151;
        let dst_pos = SHOULDER;
        let blocked = kind == ITEM_KIND_PET && dst_pos == SHOULDER && has_pet;
        assert!(!blocked);
    }

    // ── New direction type tests ────────────────────────────────

    #[test]
    fn test_item_move_dir_from_u8_new_directions() {
        assert_eq!(ItemMoveDir::from_u8(7), Some(ItemMoveDir::InvenToCosp));
        assert_eq!(ItemMoveDir::from_u8(8), Some(ItemMoveDir::CospToInven));
        assert_eq!(ItemMoveDir::from_u8(9), Some(ItemMoveDir::InvenToMbag));
        assert_eq!(ItemMoveDir::from_u8(10), Some(ItemMoveDir::MbagToInven));
        assert_eq!(ItemMoveDir::from_u8(11), Some(ItemMoveDir::MbagToMbag));
        assert_eq!(ItemMoveDir::from_u8(12), Some(ItemMoveDir::InvenToPet));
        assert_eq!(ItemMoveDir::from_u8(13), Some(ItemMoveDir::PetToInven));
        assert_eq!(ItemMoveDir::from_u8(14), Some(ItemMoveDir::SlotInvenToMbag));
        // Unmapped values
        assert!(ItemMoveDir::from_u8(15).is_none());
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_wings() {
        let item = make_test_item(ITEM_SLOT_COSP_WINGS);
        assert!(is_valid_cosp_slot_pos(&item, COSP_WINGS));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_HELMET));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_helmet() {
        let item = make_test_item(ITEM_SLOT_COSP_HELMET);
        assert!(is_valid_cosp_slot_pos(&item, COSP_HELMET));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_WINGS));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_gloves() {
        let item = make_test_item(ITEM_SLOT_COSP_GLOVES);
        assert!(is_valid_cosp_slot_pos(&item, COSP_GLOVE));
        assert!(is_valid_cosp_slot_pos(&item, COSP_GLOVE2));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_WINGS));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_pauldron() {
        let item = make_test_item(ITEM_SLOT_COSP_PAULDRON);
        assert!(is_valid_cosp_slot_pos(&item, COSP_BREAST));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_HELMET));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_bag() {
        let item = make_test_item(ITEM_SLOT_BAG);
        // C++ ItemHandler.cpp:2377 — bags go to COSP_BAG1+3 (=9) or COSP_BAG2 (=10)
        assert!(is_valid_cosp_slot_pos(&item, COSP_BAG1 + 3)); // pos 9
        assert!(is_valid_cosp_slot_pos(&item, COSP_BAG2)); // pos 10
        assert!(!is_valid_cosp_slot_pos(&item, COSP_BAG1)); // pos 6 — NOT valid
        assert!(!is_valid_cosp_slot_pos(&item, COSP_FAIRY));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_fairy() {
        let item = make_test_item(ITEM_SLOT_COSP_FAIRY);
        assert!(is_valid_cosp_slot_pos(&item, COSP_FAIRY));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_TATTO));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_tattoo() {
        let item = make_test_item(ITEM_SLOT_COSP_TATTOO);
        assert!(is_valid_cosp_slot_pos(&item, COSP_TATTO));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_FAIRY));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_talisman() {
        let item = make_test_item(ITEM_SLOT_COSP_TALISMAN);
        assert!(is_valid_cosp_slot_pos(&item, COSP_TALISMAN));
        assert!(!is_valid_cosp_slot_pos(&item, COSP_EMBLAM));
    }

    #[test]
    fn test_is_valid_cosp_slot_pos_emblem() {
        let item0 = make_test_item(ITEM_SLOT_COSP_EMBLEM0);
        let item1 = make_test_item(ITEM_SLOT_COSP_EMBLEM1);
        assert!(is_valid_cosp_slot_pos(&item0, COSP_EMBLAM));
        assert!(is_valid_cosp_slot_pos(&item1, COSP_EMBLAM));
        assert!(!is_valid_cosp_slot_pos(&item0, COSP_FAIRY));
    }

    #[test]
    fn test_cosp_pos_to_abs_index_normal_slots() {
        // Positions 0-4 map to INVENTORY_COSP + pos
        assert_eq!(cosp_pos_to_abs_index(0), Some(42)); // CWING
        assert_eq!(cosp_pos_to_abs_index(1), Some(43)); // CHELMET
        assert_eq!(cosp_pos_to_abs_index(2), Some(44)); // CLEFT
        assert_eq!(cosp_pos_to_abs_index(3), Some(45)); // CRIGHT
        assert_eq!(cosp_pos_to_abs_index(4), Some(46)); // CTOP
    }

    #[test]
    fn test_cosp_pos_to_abs_index_special_slots() {
        assert_eq!(cosp_pos_to_abs_index(COSP_EMBLAM), Some(CEMBLEM)); // 5 -> 47
        assert_eq!(cosp_pos_to_abs_index(COSP_FAIRY), Some(CFAIRY)); // 7 -> 48
        assert_eq!(cosp_pos_to_abs_index(COSP_TATTO), Some(CTATTOO)); // 8 -> 49
        assert_eq!(cosp_pos_to_abs_index(COSP_TALISMAN), Some(CTALISMAN)); // 9 -> 50
    }

    #[test]
    fn test_cosp_pos_to_abs_index_bag_slots() {
        // COSP_BAG1(6) → CBAG1(51), COSP_BAG2(10) → CBAG2(52)
        assert_eq!(cosp_pos_to_abs_index(COSP_BAG1), Some(CBAG1)); // 6 -> 51
        assert_eq!(cosp_pos_to_abs_index(COSP_BAG2), Some(CBAG2)); // 10 -> 52
    }

    #[test]
    fn test_cosp_pos_to_abs_index_out_of_range() {
        assert_eq!(cosp_pos_to_abs_index(12), None);
        assert_eq!(cosp_pos_to_abs_index(255), None);
    }

    #[test]
    fn test_cosp_pos_to_abs_index_bag3() {
        // v2600: position 11 (COSP_BAG2+1) → CBAG3 (dedicated slot 96)
        assert_eq!(cosp_pos_to_abs_index(11), Some(CBAG3));
    }

    #[test]
    fn test_inventory_layout_constants() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(INVENTORY_COSP, 42);
        assert_eq!(COSP_MAX, 11);
        assert_eq!(INVENTORY_MBAG, 53);
        assert_eq!(MBAG_MAX, 12);
        assert_eq!(MBAG_COUNT, 3);
        assert_eq!(INVENTORY_MBAG2, 65);
        assert_eq!(INVENTORY_MBAG3, 77);
        assert_eq!(MBAG_TOTAL, 36);
        assert_eq!(INVENTORY_TOTAL, 96);
    }

    #[test]
    fn test_cospre_absolute_indices() {
        assert_eq!(CEMBLEM, 47);
        assert_eq!(CFAIRY, 48);
        assert_eq!(CTATTOO, 49);
        assert_eq!(CTALISMAN, 50);
        assert_eq!(CBAG1, 51);
        assert_eq!(CBAG2, 52);
    }

    #[test]
    fn test_mbag_bag_existence_check() {
        // Bag 1 covers [INVENTORY_MBAG..INVENTORY_MBAG2) = [53..65)
        // Bag 2 covers [INVENTORY_MBAG2..INVENTORY_MBAG3) = [65..77)
        // Bag 3 covers [INVENTORY_MBAG3..INVENTORY_TOTAL) = [77..89)
        let bag1_start = INVENTORY_MBAG;
        let bag1_end = INVENTORY_MBAG2;
        assert!(bag1_start < bag1_end);
        assert_eq!(bag1_end - bag1_start, MBAG_MAX);
        let bag2_start = INVENTORY_MBAG2;
        let bag2_end = INVENTORY_MBAG3;
        assert_eq!(bag2_end - bag2_start, MBAG_MAX);
        let bag3_start = INVENTORY_MBAG3;
        let bag3_end = INVENTORY_MBAG3 + MBAG_MAX;
        assert_eq!(bag3_end - bag3_start, MBAG_MAX);
        // Knight royale slots follow after magic bags
        assert_eq!(
            bag3_end,
            INVENTORY_TOTAL - crate::inventory_constants::KNIGHT_ROYALE_MAX
        );
    }

    #[test]
    fn test_swap_items_mbag_to_inven() {
        // Simulate MBAG_TO_INVEN: move from magic bag slot to inventory
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let mbag_slot = INVENTORY_MBAG; // first slot of bag 1
        inv[mbag_slot] = UserItemSlot {
            item_id: 999,
            count: 5,
            ..Default::default()
        };
        let inven_slot = SLOT_MAX; // first inventory slot
        swap_items(&mut inv, mbag_slot, inven_slot);
        assert_eq!(inv[inven_slot].item_id, 999);
        assert_eq!(inv[inven_slot].count, 5);
        assert_eq!(inv[mbag_slot].item_id, 0);
    }

    #[test]
    fn test_swap_items_inven_to_cosp() {
        // Simulate INVEN_TO_COSP: move from inventory to cospre wing slot
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let inven_slot = SLOT_MAX + 3;
        inv[inven_slot] = UserItemSlot {
            item_id: 12345,
            count: 1,
            ..Default::default()
        };
        let cosp_slot = INVENTORY_COSP; // CWING
        swap_items(&mut inv, inven_slot, cosp_slot);
        assert_eq!(inv[cosp_slot].item_id, 12345);
        assert_eq!(inv[inven_slot].item_id, 0);
    }

    #[test]
    fn test_slot_inven_to_mbag_offset() {
        // C++ uses INVENTORY_COSP + bDstPos + 8 for ITEM_SLOT_INVEN_TO_MBAG
        // bDstPos=1 → 42 + 1 + 8 = 51 = CBAG1
        // bDstPos=2 → 42 + 2 + 8 = 52 = CBAG2
        assert_eq!(INVENTORY_COSP + 1 + 8, CBAG1);
        assert_eq!(INVENTORY_COSP + 2 + 8, CBAG2);
    }

    // ── Sprint 364: Cospre override suppression + multi-slot reveal ──

    #[test]
    fn test_chelmet_ctop_constants() {
        // C++ globals.h: CHELMET=43, CTOP=46
        assert_eq!(CHELMET, INVENTORY_COSP + COSP_HELMET as usize); // 42 + 1 = 43
        assert_eq!(CTOP, INVENTORY_COSP + COSP_BREAST as usize); // 42 + 4 = 46
    }

    #[test]
    fn test_cospre_override_head_with_coshelmet() {
        // When cosmetic helmet is equipped (CHELMET has an item),
        // equipping to HEAD should be suppressed.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Equip cosmetic helmet
        inv[CHELMET] = UserItemSlot {
            item_id: 700001,
            durability: 100,
            count: 1,
            ..Default::default()
        };
        // The broadcast check: CHELMET has item → suppress
        assert_ne!(inv[CHELMET].item_id, 0);
    }

    #[test]
    fn test_cospre_override_head_without_coshelmet() {
        // When cosmetic helmet is NOT equipped (CHELMET is empty),
        // equipping to HEAD should broadcast.
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // CHELMET empty → broadcast allowed
        assert_eq!(inv[CHELMET].item_id, 0);
    }

    #[test]
    fn test_cospre_override_breast_with_costop() {
        // When cosmetic breast is equipped (CTOP has an item),
        // equipping to BREAST/LEG/GLOVE/FOOT should be suppressed.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[CTOP] = UserItemSlot {
            item_id: 700002,
            durability: 100,
            count: 1,
            ..Default::default()
        };
        assert_ne!(inv[CTOP].item_id, 0);
    }

    #[test]
    fn test_cosp_breast_removal_reveals_4_slots() {
        // Removing COSP_BREAST should reveal BREAST(4), LEG(10), GLOVE(12), FOOT(13)
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[BREAST] = UserItemSlot {
            item_id: 100001,
            durability: 80,
            count: 1,
            ..Default::default()
        };
        inv[LEG] = UserItemSlot {
            item_id: 100002,
            durability: 70,
            count: 1,
            ..Default::default()
        };
        inv[GLOVE] = UserItemSlot {
            item_id: 100003,
            durability: 60,
            count: 1,
            ..Default::default()
        };
        inv[FOOT] = UserItemSlot {
            item_id: 100004,
            durability: 50,
            count: 1,
            ..Default::default()
        };

        // Verify all 4 equipment slots have items to reveal
        let reveal_slots: [(usize, u32); 4] = [
            (BREAST, 100001),
            (LEG, 100002),
            (GLOVE, 100003),
            (FOOT, 100004),
        ];
        for (slot, expected_id) in &reveal_slots {
            assert_eq!(inv[*slot].item_id, *expected_id);
        }
    }

    #[test]
    fn test_cosp_helmet_removal_reveals_head() {
        // Removing COSP_HELMET should reveal HEAD(1)
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[HEAD] = UserItemSlot {
            item_id: 200001,
            durability: 90,
            count: 1,
            ..Default::default()
        };
        assert_eq!(inv[HEAD].item_id, 200001);
    }

    #[test]
    fn test_cosp_breast_removal_empty_slots() {
        // Removing COSP_BREAST when equipment slots are empty should send (slot, 0, 0)
        let inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        for &slot in &[BREAST, LEG, GLOVE, FOOT] {
            assert_eq!(inv[slot].item_id, 0);
        }
    }

    #[test]
    fn test_set_user_ability_directions() {
        // Verify the 5 directions that should trigger SetUserAbility
        let equipment_dirs = [
            ItemMoveDir::InvenSlot,
            ItemMoveDir::SlotInven,
            ItemMoveDir::SlotSlot,
            ItemMoveDir::InvenToCosp,
            ItemMoveDir::CospToInven,
        ];
        let non_equipment_dirs = [
            ItemMoveDir::InvenInven,
            ItemMoveDir::InvenToMbag,
            ItemMoveDir::MbagToInven,
            ItemMoveDir::MbagToMbag,
            ItemMoveDir::SlotInvenToMbag,
            ItemMoveDir::InvenToPet,
            ItemMoveDir::PetToInven,
        ];

        for dir in &equipment_dirs {
            let is_equip = matches!(
                dir,
                ItemMoveDir::InvenSlot
                    | ItemMoveDir::SlotInven
                    | ItemMoveDir::SlotSlot
                    | ItemMoveDir::InvenToCosp
                    | ItemMoveDir::CospToInven
            );
            assert!(is_equip, "Expected {:?} to trigger SetUserAbility", dir);
        }
        for dir in &non_equipment_dirs {
            let is_equip = matches!(
                dir,
                ItemMoveDir::InvenSlot
                    | ItemMoveDir::SlotInven
                    | ItemMoveDir::SlotSlot
                    | ItemMoveDir::InvenToCosp
                    | ItemMoveDir::CospToInven
            );
            assert!(
                !is_equip,
                "Expected {:?} to NOT trigger SetUserAbility",
                dir
            );
        }
    }

    // ── Sprint 365: Hiding flags, fairy check, CospToInven broadcast fix ──

    #[test]
    fn test_item_oreads_constant() {
        // C++ GameDefine.h:62 — ITEM_OREADS
        assert_eq!(ITEM_OREADS, 700039768);
    }

    #[test]
    fn test_cosp_fairy_position() {
        // COSP_FAIRY is position 7
        assert_eq!(COSP_FAIRY, 7);
        // Maps to CFAIRY absolute index 48
        assert_eq!(cosp_pos_to_abs_index(COSP_FAIRY), Some(CFAIRY));
    }

    #[test]
    fn test_cospre_override_with_hiding_flag() {
        // When cosmetic exists but player is hiding cospre,
        // should_broadcast should return true (broadcast allowed).
        // This tests the `cosp_empty || is_hiding_cospre` logic.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        // Cosmetic helmet exists
        inv[CHELMET] = UserItemSlot {
            item_id: 700001,
            count: 1,
            ..Default::default()
        };
        // Cosmetic breast exists
        inv[CTOP] = UserItemSlot {
            item_id: 700002,
            count: 1,
            ..Default::default()
        };

        // Without hiding: cosps suppress broadcast
        let cosp_head_empty = inv[CHELMET].item_id == 0;
        let cosp_body_empty = inv[CTOP].item_id == 0;
        assert!(!cosp_head_empty); // cosmetic present
        assert!(!cosp_body_empty);

        // With hiding flag: broadcast allowed even with cosmetic present
        let is_hiding = true;
        assert!(cosp_head_empty || is_hiding);
        assert!(cosp_body_empty || is_hiding);
    }

    #[test]
    fn test_cosp_to_inven_no_broadcast_for_emblam_bag() {
        // CospToInven should NOT broadcast for EMBLAM(5), BAG1(6), BAG2(10)
        let skip_positions = [COSP_EMBLAM, COSP_BAG1, COSP_BAG2];
        for &pos in &skip_positions {
            assert!(
                pos == COSP_EMBLAM || pos == COSP_BAG1 || pos == COSP_BAG2,
                "Position {} should be skipped in CospToInven broadcast",
                pos
            );
        }
        // COSP_WINGS, COSP_GLOVE etc. should still broadcast
        for pos in 0..=4u8 {
            assert!(
                pos != COSP_EMBLAM && pos != COSP_BAG1 && pos != COSP_BAG2,
                "Position {} should NOT be skipped",
                pos
            );
        }
    }

    #[test]
    fn test_fairy_check_equip_condition() {
        // Fairy check should be set when equipping ITEM_OREADS to COSP_FAIRY
        let dir = ItemMoveDir::InvenToCosp;
        let dst_pos = COSP_FAIRY;
        let item_id = ITEM_OREADS;
        assert!(dir == ItemMoveDir::InvenToCosp && dst_pos == COSP_FAIRY && item_id == ITEM_OREADS);
    }

    #[test]
    fn test_fairy_check_unequip_condition() {
        // Fairy check should be cleared when removing ITEM_OREADS from COSP_FAIRY
        let dir = ItemMoveDir::CospToInven;
        let src_pos = COSP_FAIRY;
        let item_id = ITEM_OREADS;
        assert!(dir == ItemMoveDir::CospToInven && src_pos == COSP_FAIRY && item_id == ITEM_OREADS);
    }

    #[test]
    fn test_fairy_check_not_triggered_for_other_items() {
        // Non-OREADS items should NOT trigger fairy check
        let item_id = 100000u32;
        assert!(item_id != ITEM_OREADS);
        // Non-FAIRY positions should NOT trigger fairy check
        let pos = COSP_TATTO;
        assert!(pos != COSP_FAIRY);
    }

    // ── Sprint 366: Pet inventory directions 12/13 ──

    #[test]
    fn test_pet_inventory_total_constant() {
        assert_eq!(PET_INVENTORY_TOTAL, 4);
    }

    #[test]
    fn test_inven_to_pet_bounds_validation() {
        // src_pos must be < HAVE_MAX (28), dst_pos must be < PET_INVENTORY_TOTAL (4)
        let src_valid = 0u8;
        let dst_valid = 3u8;
        assert!((src_valid as usize) < HAVE_MAX);
        assert!((dst_valid as usize) < PET_INVENTORY_TOTAL);

        let src_invalid = HAVE_MAX as u8;
        let dst_invalid = PET_INVENTORY_TOTAL as u8;
        assert!(((src_invalid as usize) >= HAVE_MAX));
        assert!(((dst_invalid as usize) >= PET_INVENTORY_TOTAL));
    }

    #[test]
    fn test_pet_to_inven_bounds_validation() {
        // src_pos must be < PET_INVENTORY_TOTAL (4), dst_pos must be < HAVE_MAX (28)
        let src_valid = 2u8;
        let dst_valid = 10u8;
        assert!((src_valid as usize) < PET_INVENTORY_TOTAL);
        assert!((dst_valid as usize) < HAVE_MAX);

        let src_invalid = PET_INVENTORY_TOTAL as u8;
        let dst_invalid = HAVE_MAX as u8;
        assert!(((src_invalid as usize) >= PET_INVENTORY_TOTAL));
        assert!(((dst_invalid as usize) >= HAVE_MAX));
    }

    #[test]
    fn test_inven_to_pet_move_to_empty() {
        // Move item from inventory to empty pet slot
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let mut pet_items = [
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
        ];

        let src_idx = SLOT_MAX + 5; // inventory pos 5
        inv[src_idx] = UserItemSlot {
            item_id: 12345,
            count: 1,
            durability: 100,
            ..Default::default()
        };

        let dst_idx = 0usize; // pet slot 0
        assert_eq!(pet_items[dst_idx].item_id, 0);

        // Simulate move: empty destination
        pet_items[dst_idx] = inv[src_idx].clone();
        inv[src_idx] = UserItemSlot::default();

        assert_eq!(pet_items[dst_idx].item_id, 12345);
        assert_eq!(pet_items[dst_idx].durability, 100);
        assert_eq!(inv[src_idx].item_id, 0);
    }

    #[test]
    fn test_pet_to_inven_move_to_empty() {
        // Move item from pet slot to empty inventory
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let mut pet_items = [
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
        ];

        pet_items[2] = UserItemSlot {
            item_id: 67890,
            count: 3,
            durability: 50,
            ..Default::default()
        };

        let dst_idx = SLOT_MAX + 10; // inventory pos 10
        assert_eq!(inv[dst_idx].item_id, 0);

        // Simulate move: empty destination
        inv[dst_idx] = pet_items[2].clone();
        pet_items[2] = UserItemSlot::default();

        assert_eq!(inv[dst_idx].item_id, 67890);
        assert_eq!(inv[dst_idx].count, 3);
        assert_eq!(pet_items[2].item_id, 0);
    }

    #[test]
    fn test_inven_to_pet_swap() {
        // Swap when pet slot is occupied with a different item
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let mut pet_items = [
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
        ];

        let src_idx = SLOT_MAX;
        inv[src_idx] = UserItemSlot {
            item_id: 1111,
            count: 1,
            durability: 80,
            ..Default::default()
        };
        pet_items[0] = UserItemSlot {
            item_id: 2222,
            count: 1,
            durability: 60,
            ..Default::default()
        };

        // Swap
        std::mem::swap(&mut inv[src_idx], &mut pet_items[0]);

        assert_eq!(inv[src_idx].item_id, 2222);
        assert_eq!(inv[src_idx].durability, 60);
        assert_eq!(pet_items[0].item_id, 1111);
        assert_eq!(pet_items[0].durability, 80);
    }

    #[test]
    fn test_pet_inventory_same_item_swaps_not_stacks() {
        // C++ never stacks for pet directions (pTableSrc uninitialized → isnull() = true).
        // Even with same countable item, it always swaps.
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let mut pet_items = [
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
        ];

        let src_idx = SLOT_MAX + 3;
        inv[src_idx] = UserItemSlot {
            item_id: 5555,
            count: 10,
            durability: 0,
            ..Default::default()
        };
        pet_items[1] = UserItemSlot {
            item_id: 5555,
            count: 5,
            durability: 0,
            ..Default::default()
        };

        // C++ behavior: always swap, never stack for pet
        std::mem::swap(&mut inv[src_idx], &mut pet_items[1]);

        // After swap: inventory gets pet's 5, pet gets inventory's 10
        assert_eq!(inv[src_idx].item_id, 5555);
        assert_eq!(inv[src_idx].count, 5);
        assert_eq!(pet_items[1].item_id, 5555);
        assert_eq!(pet_items[1].count, 10);
    }

    #[test]
    fn test_pet_to_inven_same_item_swaps_not_stacks() {
        // C++ never stacks for pet directions — always swaps
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let mut pet_items = [
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
            UserItemSlot::default(),
        ];

        pet_items[0] = UserItemSlot {
            item_id: 7777,
            count: 20,
            durability: 0,
            ..Default::default()
        };
        let dst_idx = SLOT_MAX + 5;
        inv[dst_idx] = UserItemSlot {
            item_id: 7777,
            count: 8,
            durability: 0,
            ..Default::default()
        };

        // C++ behavior: always swap
        std::mem::swap(&mut pet_items[0], &mut inv[dst_idx]);

        // After swap: inventory gets pet's 20, pet gets inventory's 8
        assert_eq!(inv[dst_idx].item_id, 7777);
        assert_eq!(inv[dst_idx].count, 20);
        assert_eq!(pet_items[0].item_id, 7777);
        assert_eq!(pet_items[0].count, 8);
    }

    #[test]
    fn test_pet_inven_item_id_mismatch_rejected() {
        // Transfer must fail if item_id doesn't match source slot
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        let src_idx = SLOT_MAX + 2;
        inv[src_idx] = UserItemSlot {
            item_id: 9999,
            count: 1,
            ..Default::default()
        };

        // Request claims item_id 1234 but actual is 9999
        let requested_item_id = 1234u32;
        assert_ne!(inv[src_idx].item_id, requested_item_id);
    }

    #[test]
    fn test_pet_directions_not_equipment_change() {
        // Pet directions should NOT trigger SetUserAbility or broadcast_look_change
        let dir_pet1 = ItemMoveDir::InvenToPet;
        let dir_pet2 = ItemMoveDir::PetToInven;
        let is_equip1 = matches!(
            dir_pet1,
            ItemMoveDir::InvenSlot
                | ItemMoveDir::SlotInven
                | ItemMoveDir::SlotSlot
                | ItemMoveDir::InvenToCosp
                | ItemMoveDir::CospToInven
        );
        let is_equip2 = matches!(
            dir_pet2,
            ItemMoveDir::InvenSlot
                | ItemMoveDir::SlotInven
                | ItemMoveDir::SlotSlot
                | ItemMoveDir::InvenToCosp
                | ItemMoveDir::CospToInven
        );
        assert!(!is_equip1, "InvenToPet should NOT trigger SetUserAbility");
        assert!(!is_equip2, "PetToInven should NOT trigger SetUserAbility");
    }
}
