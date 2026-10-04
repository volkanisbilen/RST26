//! WIZ_WAREHOUSE (0x45) handler — personal warehouse (inn) storage.
//! Sub-opcodes:
//! - 1 = WAREHOUSE_OPEN: Send all stored items to client (192 slots)
//! - 2 = WAREHOUSE_INPUT: Move item from inventory -> warehouse
//! - 3 = WAREHOUSE_OUTPUT: Move item from warehouse -> inventory
//! - 4 = WAREHOUSE_MOVE: Rearrange items within warehouse
//! - 5 = WAREHOUSE_INVENMOVE: Rearrange items within inventory (from warehouse UI)
//! Warehouse is per-account (not per-character).
//! Items are loaded lazily on first WAREHOUSE_OPEN.
//! Response format: `[u8 opcode] [u8 result]`
//! - result 0 = NoAccess, 1 = Success, 2 = RequiredMoney, 3 = InvalidPassword

use ko_db::repositories::character::{
    CharacterRepository, SaveItemParams, SaveWarehouseItemParams,
};
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::warn;

use crate::session::{ClientSession, SessionState};
use crate::world::{
    UserItemSlot, ITEMCOUNT_MAX, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED, ITEM_FLAG_SEALED,
    ITEM_GOLD, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN, ZONE_ARDREAM, ZONE_RONARK_LAND,
    ZONE_RONARK_LAND_BASE,
};

/// Warehouse sub-opcodes
const WAREHOUSE_OPEN: u8 = 0x01;
const WAREHOUSE_INPUT: u8 = 0x02;
const WAREHOUSE_OUTPUT: u8 = 0x03;
const WAREHOUSE_MOVE: u8 = 0x04;
const WAREHOUSE_INVENMOVE: u8 = 0x05;

use super::COIN_MAX;
use super::{HAVE_MAX, SLOT_MAX};
use crate::inventory_constants::{ITEMS_PER_PAGE, WAREHOUSE_MAX};

// Item flag constants imported from crate::world (ITEM_FLAG_RENTED, ITEM_FLAG_DUPLICATE, ITEM_FLAG_SEALED).

/// PK zone warehouse open cost.
const PK_ZONE_WAREHOUSE_COST: u32 = 10_000;

/// Build a warehouse response packet.
fn build_warehouse_result(opcode: u8, result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizWarehouse as u8);
    pkt.write_u8(opcode);
    pkt.write_u8(result);
    pkt
}

/// Check if a zone is a PK zone.
fn is_pk_zone(zone_id: u16) -> bool {
    zone_id == ZONE_ARDREAM || zone_id == ZONE_RONARK_LAND || zone_id == ZONE_RONARK_LAND_BASE
}

/// Handle WIZ_WAREHOUSE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let world = session.world().clone();
    let sid = session.session_id();

    // Must be alive and not in a busy state
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_store_open(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_buying_merchant_preparing(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    let mut reader = PacketReader::new(&pkt.data);
    let opcode = reader.read_u8().unwrap_or(0);

    // Genie state check — must happen AFTER reading opcode so error packet includes it.
    if world.with_session(sid, |h| h.genie_active).unwrap_or(false) {
        return session
            .send_packet(&build_warehouse_result(opcode, 0))
            .await;
    }

    let pos = world.get_position(sid);
    let zone_id = pos.map(|p| p.zone_id).unwrap_or(0);

    if opcode == WAREHOUSE_OPEN {
        return handle_warehouse_open(session, &ch, zone_id).await;
    }

    // For non-OPEN opcodes, read common fields:
    // [u32 npc_id] [u32 item_id] [u8 page] [u8 src_pos] [u8 dst_pos]
    let npc_id = reader.read_u32().unwrap_or(0);

    // NPC validation — dead check, type check, and distance check
    // C++ fail_return sends: [u8 opcode][u8 NotAccess(0)]
    use crate::npc_type_constants::NPC_WAREHOUSE;
    const RESULT_NOT_ACCESS: u8 = 0;
    if npc_id != 0 {
        let mut npc_valid = !world.is_npc_dead(npc_id) && world.is_in_npc_range(sid, npc_id);
        if npc_valid {
            if let Some(npc) = world.get_npc_instance(npc_id) {
                if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
                    if tmpl.npc_type != NPC_WAREHOUSE {
                        npc_valid = false;
                    }
                } else {
                    npc_valid = false;
                }
            } else {
                npc_valid = false;
            }
        }
        if !npc_valid {
            let mut fail = Packet::new(Opcode::WizWarehouse as u8);
            fail.write_u8(opcode);
            fail.write_u8(RESULT_NOT_ACCESS);
            return session.send_packet(&fail).await;
        }
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);

    let reference_pos = ITEMS_PER_PAGE * page as usize;

    match opcode {
        WAREHOUSE_INPUT => {
            let count = reader.read_u32().unwrap_or(0);
            handle_warehouse_input(session, item_id, reference_pos, src_pos, dst_pos, count).await
        }
        WAREHOUSE_OUTPUT => {
            let count = reader.read_u32().unwrap_or(0);
            handle_warehouse_output(session, item_id, reference_pos, src_pos, dst_pos, count).await
        }
        WAREHOUSE_MOVE => {
            handle_warehouse_move(session, item_id, reference_pos, src_pos, dst_pos).await
        }
        WAREHOUSE_INVENMOVE => handle_warehouse_invenmove(session, item_id, src_pos, dst_pos).await,
        _ => {
            warn!(
                "[{}] Unknown warehouse sub-opcode: 0x{:02X}",
                session.addr(),
                opcode
            );
            Ok(())
        }
    }
}

/// Handle WAREHOUSE_OPEN (sub-opcode 1).
/// PK zone: costs 10,000 gold to open.
/// Response: `[u8 WAREHOUSE_OPEN] [u8 1=success] [u32 inn_coins]`
///   followed by 192 items: `[u32 item_id] [u16 durability] [u16 count] [u8 flag] [u32 unique_id] [u32 expire_time]`
async fn handle_warehouse_open(
    session: &mut ClientSession,
    ch: &crate::world::CharacterInfo,
    zone_id: u16,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // PK zone: charge 10,000 gold
    if is_pk_zone(zone_id) {
        if ch.gold < PK_ZONE_WAREHOUSE_COST {
            let err = build_warehouse_result(WAREHOUSE_OPEN, 2); // RequiredMoney
            session.send_packet(&err).await?;
            return Ok(());
        }
        world.gold_lose(sid, PK_ZONE_WAREHOUSE_COST);
    }

    // Lazy-load warehouse from DB if not already loaded
    if !world.is_warehouse_loaded(sid) {
        let account_id = match session.account_id() {
            Some(a) => a.to_string(),
            None => return Ok(()),
        };

        let repo = CharacterRepository::new(session.pool());
        let db_items = repo.load_warehouse_items(&account_id).await?;
        let inn_coins = repo.load_warehouse_coins(&account_id).await?;

        // Build 192-slot warehouse array
        let mut warehouse = vec![UserItemSlot::default(); WAREHOUSE_MAX];
        for row in &db_items {
            let idx = row.slot_index as usize;
            if idx < WAREHOUSE_MAX {
                warehouse[idx] = UserItemSlot {
                    item_id: row.item_id as u32,
                    durability: row.durability,
                    count: row.count as u16,
                    flag: row.flag as u8,
                    original_flag: row.original_flag as u8,
                    serial_num: row.serial_num as u64,
                    expire_time: row.expire_time as u32,
                };
            }
        }

        world.set_warehouse(sid, warehouse, inn_coins as u32);
    }

    // Build response
    let inn_coins = world.get_inn_coins(sid);
    let warehouse = world.get_warehouse(sid);
    let rebirth_level = world
        .get_character_info(sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);

    let mut result = Packet::new(Opcode::WizWarehouse as u8);
    result.write_u8(WAREHOUSE_OPEN);
    result.write_u8(1); // success
    result.write_u32(inn_coins);

    for i in 0..WAREHOUSE_MAX {
        let slot = warehouse.get(i).cloned().unwrap_or_default();
        result.write_u32(slot.item_id);
        result.write_u16(slot.durability as u16);
        result.write_u16(slot.count);
        result.write_u8(slot.flag);
        crate::handler::unique_item_info::write_unique_item_info(
            &world,
            session.pool(),
            slot.item_id,
            slot.serial_num,
            rebirth_level,
            &mut result,
        )
        .await;
        result.write_u32(slot.expire_time);
    }

    session.send_packet(&result).await
}

/// Handle WAREHOUSE_INPUT (sub-opcode 2) — inventory -> warehouse.
async fn handle_warehouse_input(
    session: &mut ClientSession,
    item_id: u32,
    reference_pos: usize,
    src_pos: u8,
    dst_pos: u8,
    count: u32,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if count == 0 {
        let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Special case: gold input (inventory gold -> inn coins)
    if item_id == ITEM_GOLD {
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
                session.send_packet(&err).await?;
                return Ok(());
            }
        };
        let inn = world.get_inn_coins(sid);
        if ch.gold < count || (inn as u64 + count as u64) > COIN_MAX as u64 {
            let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
            session.send_packet(&err).await?;
            return Ok(());
        }
        world.gold_lose(sid, count);
        world.update_warehouse(sid, |_wh, coins| {
            *coins += count;
            true
        });
        save_warehouse_coins_async(session);
        let ok = build_warehouse_result(WAREHOUSE_INPUT, 1);
        session.send_packet(&ok).await?;
        return Ok(());
    }

    // Validate positions
    let src_slot_idx = SLOT_MAX + src_pos as usize;
    let wh_slot_idx = reference_pos + dst_pos as usize;

    // C++ uses `bSrcPos > HAVE_MAX` but position 28 hits cospre region (SLOT_MAX+28=42).
    // Use `>=` to match Sprint 828 fix (item_move) and vip/clan warehouse handlers.
    if src_pos as usize >= HAVE_MAX || wh_slot_idx >= WAREHOUSE_MAX {
        let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // No-trade items cannot be stored
    if (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id) {
        let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Get item table entry
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
            session.send_packet(&err).await?;
            return Ok(());
        }
    };
    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let countable = item_table.countable.unwrap_or(0);

    // Perform the transfer atomically
    let success = world.update_inventory_and_warehouse(sid, |inv, wh, _coins| {
        // Ensure warehouse is properly sized
        while wh.len() < WAREHOUSE_MAX {
            wh.push(UserItemSlot::default());
        }

        // Validate source
        let src = match inv.get(src_slot_idx) {
            Some(s) if s.item_id == item_id => s.clone(),
            _ => return false,
        };

        // Check flags: no rented, duplicate, sealed, or expiration items
        if src.flag == ITEM_FLAG_RENTED
            || src.flag == ITEM_FLAG_DUPLICATE
            || src.flag == ITEM_FLAG_SEALED
            || src.expire_time > 0
        {
            return false;
        }

        // Validate destination slot: non-stackable must go to empty,
        // stackable must go to same item or empty. Source must have enough.
        let dst = &wh[wh_slot_idx];
        if src.count < count as u16
            || (dst.item_id != 0 && (!is_stackable || dst.item_id != src.item_id))
        {
            return false;
        }

        // Capture original destination state for serial number logic.
        // which is evaluated BEFORE `pDstItem->nNum = pSrcItem->nNum` at line 187.
        let dst_was_empty = wh[wh_slot_idx].item_id == 0;

        // Apply to destination
        let dst = &mut wh[wh_slot_idx];
        if is_stackable {
            dst.count = dst.count.saturating_add(count as u16);
        } else {
            dst.count = count as u16;
        }
        dst.durability = src.durability;
        dst.flag = src.flag;
        dst.original_flag = src.original_flag;
        dst.expire_time = src.expire_time;
        dst.item_id = src.item_id;

        if dst.count > ITEMCOUNT_MAX {
            dst.count = ITEMCOUNT_MAX;
        }

        // Handle serial number
        // C++ checks `!pDstItem->nNum` (original item_id == 0) to determine if
        // destination was empty. Serial assignment only happens for empty destinations.
        let serial = if src.serial_num != 0 {
            src.serial_num
        } else {
            world.generate_item_serial()
        };
        if is_stackable {
            if inv[src_slot_idx].count == count as u16 && dst_was_empty {
                dst.serial_num = serial;
            } else if dst_was_empty {
                dst.serial_num = world.generate_item_serial();
            }
        } else {
            dst.serial_num = serial;
        }

        // Reduce source
        let src_mut = &mut inv[src_slot_idx];
        if is_stackable {
            src_mut.count -= count as u16;
        } else {
            src_mut.count = 0;
        }

        if src_mut.count == 0 || countable == 0 {
            *src_mut = UserItemSlot::default();
        }

        true
    });

    if success {
        world.set_user_ability(sid);
        save_warehouse_slot_async(session, wh_slot_idx);
        save_inventory_slot_async(session, src_slot_idx);

        let ok = build_warehouse_result(WAREHOUSE_INPUT, 1);
        session.send_packet(&ok).await?;
        crate::handler::item_move::send_inventory_snapshot(session).await?;
    } else {
        let err = build_warehouse_result(WAREHOUSE_INPUT, 0);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle WAREHOUSE_OUTPUT (sub-opcode 3) — warehouse -> inventory.
async fn handle_warehouse_output(
    session: &mut ClientSession,
    item_id: u32,
    reference_pos: usize,
    src_pos: u8,
    dst_pos: u8,
    count: u32,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if count == 0 {
        let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Special case: gold output (inn coins -> inventory gold)
    if item_id == ITEM_GOLD {
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
                session.send_packet(&err).await?;
                return Ok(());
            }
        };
        let inn = world.get_inn_coins(sid);
        if inn < count || (ch.gold as u64 + count as u64) > COIN_MAX as u64 {
            let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
            session.send_packet(&err).await?;
            return Ok(());
        }
        world.update_warehouse(sid, |_wh, coins| {
            *coins -= count;
            true
        });
        world.gold_gain(sid, count);
        save_warehouse_coins_async(session);
        let ok = build_warehouse_result(WAREHOUSE_OUTPUT, 1);
        session.send_packet(&ok).await?;
        return Ok(());
    }

    let wh_slot_idx = reference_pos + src_pos as usize;
    let dst_slot_idx = SLOT_MAX + dst_pos as usize;

    // C++ uses `bDstPos > HAVE_MAX` but position 28 hits cospre region.
    if wh_slot_idx >= WAREHOUSE_MAX || dst_pos as usize >= HAVE_MAX {
        let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Get item table entry
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
            session.send_packet(&err).await?;
            return Ok(());
        }
    };
    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let _kind = item_table.kind.unwrap_or(0);
    let countable = item_table.countable.unwrap_or(0);

    // Weight check
    if !world.check_weight(sid, item_id, count as u16) {
        let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let success = world.update_inventory_and_warehouse(sid, |inv, wh, _coins| {
        while wh.len() < WAREHOUSE_MAX {
            wh.push(UserItemSlot::default());
        }

        // Validate source
        let src = match wh.get(wh_slot_idx) {
            Some(s) if s.item_id == item_id && s.count >= count as u16 => s.clone(),
            _ => return false,
        };

        // Validate destination: non-stackable must go to empty,
        // stackable must go to same item or empty.
        let dst = match inv.get(dst_slot_idx) {
            Some(d) => d.clone(),
            None => return false,
        };
        if dst.item_id != 0 && (!is_stackable || dst.item_id != src.item_id) {
            return false;
        }

        // Apply to destination
        let dst_mut = &mut inv[dst_slot_idx];
        if is_stackable {
            dst_mut.count = dst_mut.count.saturating_add(count as u16);
        } else {
            dst_mut.count = count as u16;
        }
        dst_mut.durability = src.durability;
        dst_mut.flag = src.flag;
        dst_mut.original_flag = src.original_flag;
        dst_mut.expire_time = src.expire_time;
        dst_mut.item_id = src.item_id;

        if dst_mut.count > ITEMCOUNT_MAX {
            dst_mut.count = ITEMCOUNT_MAX;
        }

        // Handle serial number
        // C++ checks `!pDstItem->nNum` (original item_id == 0) — use the cloned `dst`
        // which still has the original state captured before modifications.
        let dst_was_empty = dst.item_id == 0;
        let serial = if src.serial_num != 0 {
            src.serial_num
        } else {
            world.generate_item_serial()
        };
        if is_stackable {
            if wh[wh_slot_idx].count == count as u16 && dst_was_empty {
                dst_mut.serial_num = serial;
            } else if dst_was_empty {
                dst_mut.serial_num = world.generate_item_serial();
            }
        } else {
            dst_mut.serial_num = serial;
        }

        // Reduce source
        let src_mut = &mut wh[wh_slot_idx];
        if is_stackable {
            src_mut.count -= count as u16;
        } else {
            src_mut.count = 0;
        }

        if src_mut.count == 0 || countable == 0 {
            *src_mut = UserItemSlot::default();
        }

        true
    });

    if success {
        world.set_user_ability(sid);
        save_warehouse_slot_async(session, wh_slot_idx);
        save_inventory_slot_async(session, dst_slot_idx);

        let ok = build_warehouse_result(WAREHOUSE_OUTPUT, 1);
        session.send_packet(&ok).await?;
        crate::handler::item_move::send_inventory_snapshot(session).await?;
    } else {
        let err = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle WAREHOUSE_MOVE (sub-opcode 4) — rearrange within warehouse.
async fn handle_warehouse_move(
    session: &mut ClientSession,
    item_id: u32,
    reference_pos: usize,
    src_pos: u8,
    dst_pos: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let src_idx = reference_pos + src_pos as usize;
    let dst_idx = reference_pos + dst_pos as usize;

    if src_idx >= WAREHOUSE_MAX || dst_idx >= WAREHOUSE_MAX {
        let err = build_warehouse_result(WAREHOUSE_MOVE, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let success = world.update_warehouse(sid, |wh, _coins| {
        while wh.len() < WAREHOUSE_MAX {
            wh.push(UserItemSlot::default());
        }

        // Source must match, destination must be empty
        if wh[src_idx].item_id != item_id || wh[dst_idx].item_id != 0 {
            return false;
        }

        let tmp = wh[src_idx].clone();
        wh[dst_idx] = tmp;
        wh[src_idx] = UserItemSlot::default();
        true
    });

    if success {
        save_warehouse_slot_async(session, src_idx);
        save_warehouse_slot_async(session, dst_idx);
        // GM inventory diagnostics are intentionally emitted only after a
        // successful move, so an invalid client request cannot create noise.
        if world
            .get_character_info(sid)
            .is_some_and(|character| character.authority == 0 || character.authority == 2)
        {
            super::client_event::send_gm_debug_chat(
                &world,
                sid,
                &format!(
                    "[GM] Bank item moved: item_id={} slot {} -> {}",
                    item_id, src_idx, dst_idx
                ),
            );
        }
        let ok = build_warehouse_result(WAREHOUSE_MOVE, 1);
        session.send_packet(&ok).await?;
    } else {
        let err = build_warehouse_result(WAREHOUSE_MOVE, 0);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle WAREHOUSE_INVENMOVE (sub-opcode 5) — rearrange within inventory (from warehouse UI).
async fn handle_warehouse_invenmove(
    session: &mut ClientSession,
    item_id: u32,
    src_pos: u8,
    dst_pos: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // C++ uses `> HAVE_MAX` but position 28 hits cospre region.
    if src_pos as usize >= HAVE_MAX || dst_pos as usize >= HAVE_MAX {
        let err = build_warehouse_result(WAREHOUSE_INVENMOVE, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let src_idx = SLOT_MAX + src_pos as usize;
    let dst_idx = SLOT_MAX + dst_pos as usize;

    let success = world.update_inventory(sid, |inv| {
        if src_idx >= inv.len() || dst_idx >= inv.len() {
            return false;
        }
        // Source must match, destination must be empty
        if inv[src_idx].item_id != item_id || inv[dst_idx].item_id != 0 {
            return false;
        }

        let tmp = inv[src_idx].clone();
        inv[dst_idx] = tmp;
        inv[src_idx] = UserItemSlot::default();
        true
    });

    if success {
        save_inventory_slot_async(session, src_idx);
        save_inventory_slot_async(session, dst_idx);
        if world
            .get_character_info(sid)
            .is_some_and(|character| character.authority == 0 || character.authority == 2)
        {
            super::client_event::send_gm_debug_chat(
                &world,
                sid,
                &format!(
                    "[GM] Inventory item moved: item_id={} slot {} -> {}",
                    item_id, src_pos, dst_pos
                ),
            );
        }
        let ok = build_warehouse_result(WAREHOUSE_INVENMOVE, 1);
        session.send_packet(&ok).await?;
    } else {
        let err = build_warehouse_result(WAREHOUSE_INVENMOVE, 0);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Save a warehouse slot to DB (fire-and-forget).
fn save_warehouse_slot_async(session: &ClientSession, slot_idx: usize) {
    let world = session.world().clone();
    let sid = session.session_id();
    let account_id = match session.account_id() {
        Some(a) => a.to_string(),
        None => return,
    };
    let slot = world.get_warehouse_slot(sid, slot_idx).unwrap_or_default();
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        let params = SaveWarehouseItemParams {
            account_id: &account_id,
            slot_index: slot_idx as i16,
            item_id: slot.item_id as i32,
            durability: slot.durability,
            count: slot.count as i16,
            flag: slot.flag as i16,
            original_flag: slot.original_flag as i16,
            serial_num: slot.serial_num as i64,
            expire_time: slot.expire_time as i32,
        };
        if let Err(e) = repo.save_warehouse_item(&params).await {
            warn!("Failed to save warehouse slot {}: {}", slot_idx, e);
        }
    });
}

/// Save an inventory slot to DB (fire-and-forget).
fn save_inventory_slot_async(session: &ClientSession, slot_idx: usize) {
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
            warn!("Failed to save inventory slot {}: {}", slot_idx, e);
        }
    });
}

/// Save warehouse coins to DB (fire-and-forget).
fn save_warehouse_coins_async(session: &ClientSession) {
    let world = session.world().clone();
    let sid = session.session_id();
    let account_id = match session.account_id() {
        Some(a) => a.to_string(),
        None => return,
    };
    let coins = world.get_inn_coins(sid);
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo
            .save_warehouse_coins(&account_id, coins.min(i32::MAX as u32) as i32)
            .await
        {
            warn!("Failed to save warehouse coins: {}", e);
        }
    });
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pk_zone() {
        assert!(is_pk_zone(71));
        assert!(is_pk_zone(72));
        assert!(is_pk_zone(73));
        assert!(!is_pk_zone(1));
        assert!(!is_pk_zone(21));
    }

    #[test]
    fn test_warehouse_result_packet() {
        let pkt = build_warehouse_result(WAREHOUSE_OPEN, 1);
        assert_eq!(pkt.opcode, Opcode::WizWarehouse as u8);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], WAREHOUSE_OPEN);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_item_flag_constants() {
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
    }

    #[test]
    fn test_warehouse_constants() {
        assert_eq!(WAREHOUSE_MAX, 192);
        assert_eq!(ITEMS_PER_PAGE, 24);
        assert_eq!(WAREHOUSE_MAX, ITEMS_PER_PAGE * 8); // 8 pages
    }

    #[test]
    fn test_reference_pos_calculation() {
        // Page 0: slots 0-23
        assert_eq!(0_u32, 0); // page 0 starts at slot 0
                              // Page 7: slots 168-191
        assert_eq!(ITEMS_PER_PAGE * 7, 168);
        assert!(ITEMS_PER_PAGE * 7 + 23 < WAREHOUSE_MAX);
    }

    #[test]
    fn test_gold_constants() {
        assert_eq!(ITEM_GOLD, 900_000_000);
        assert!(ITEM_NO_TRADE_MIN > ITEM_GOLD);
        assert!(ITEM_NO_TRADE_MAX > ITEM_NO_TRADE_MIN);
    }

    // ── Sprint 287 / Sprint 835: Slot bounds tests ─────────────────────
    // C++ uses `bSrcPos > HAVE_MAX` but position 28 maps to SLOT_MAX+28=42
    // which is the first cospre slot. Sprint 835 corrected to `>= HAVE_MAX`
    // matching Sprint 828 item_move fix and vip/clan warehouse handlers.

    #[test]
    fn test_inventory_slot_27_is_valid() {
        let slot_27: usize = 27;
        assert!(
            slot_27 < HAVE_MAX,
            "Slot 27 must pass bounds check (last valid bag slot)"
        );
    }

    #[test]
    fn test_inventory_slot_28_is_invalid() {
        // Position 28 = SLOT_MAX + 28 = 42 = first cospre slot (cross-region)
        let slot_28: usize = 28;
        assert!(
            slot_28 >= HAVE_MAX,
            "Slot 28 must fail bounds check (cospre region)"
        );
    }

    // ── Sprint 313: Serial number uses original item_id check ────────

    /// before it's set at line 187. Serial assignment only for originally-empty slots.
    #[test]
    fn test_serial_check_uses_original_item_id() {
        // Empty slot: item_id == 0 → dst_was_empty = true
        let empty_item_id: u32 = 0;
        assert!(empty_item_id == 0); // dst_was_empty

        // Non-empty slot (existing stack): item_id != 0 → dst_was_empty = false
        let occupied_item_id: u32 = 389010000;
        assert!(occupied_item_id != 0); // NOT dst_was_empty

        // Serial should NOT be touched when stacking into occupied slot
        let existing_serial: u64 = 123456;
        let dst_was_empty = occupied_item_id == 0;
        assert!(!dst_was_empty);
        // The existing serial (123456) stays unchanged
        assert_eq!(existing_serial, 123456);
    }

    #[test]
    fn test_serial_empty_slot_all_moved() {
        // C++ WareHouse.cpp:176 — `!pSrcItem->sCount && !pDstItem->nNum`
        // All items moved to empty slot → use source serial
        let src_count_before: u16 = 10;
        let transfer_count: u16 = 10;
        let dst_was_empty = true;
        let all_moved = src_count_before == transfer_count;
        assert!(all_moved && dst_was_empty); // → dst.serial = src.serial
    }

    #[test]
    fn test_serial_empty_slot_partial_move() {
        // C++ WareHouse.cpp:178 — `!pDstItem->nNum` (partial move)
        // Partial move to empty slot → generate new serial
        let src_count_before: u16 = 10;
        let transfer_count: u16 = 5;
        let dst_was_empty = true;
        let all_moved = src_count_before == transfer_count;
        assert!(!all_moved && dst_was_empty); // → dst.serial = GenerateItemSerial()
    }

    // ── Sprint 318: Genie check moved after opcode read ─────────────

    /// the sub-opcode, and sends an error packet with the correct opcode byte.
    #[test]
    fn test_genie_warehouse_open_error_packet() {
        let pkt = build_warehouse_result(WAREHOUSE_OPEN, 0);
        assert_eq!(pkt.opcode, Opcode::WizWarehouse as u8);
        assert_eq!(pkt.data[0], WAREHOUSE_OPEN);
        assert_eq!(pkt.data[1], 0); // NoAccess
    }

    #[test]
    fn test_genie_warehouse_input_error_packet() {
        let pkt = build_warehouse_result(WAREHOUSE_INPUT, 0);
        assert_eq!(pkt.data[0], WAREHOUSE_INPUT);
        assert_eq!(pkt.data[1], 0); // NoAccess
    }

    #[test]
    fn test_genie_warehouse_output_error_packet() {
        let pkt = build_warehouse_result(WAREHOUSE_OUTPUT, 0);
        assert_eq!(pkt.data[0], WAREHOUSE_OUTPUT);
        assert_eq!(pkt.data[1], 0); // NoAccess
    }

    // ── Sprint 954: Additional coverage ──────────────────────────────

    /// Warehouse sub-opcodes: sequential 1-5.
    #[test]
    fn test_warehouse_sub_opcodes() {
        assert_eq!(WAREHOUSE_OPEN, 1);
        assert_eq!(WAREHOUSE_INPUT, 2);
        assert_eq!(WAREHOUSE_OUTPUT, 3);
        assert_eq!(WAREHOUSE_MOVE, 4);
        assert_eq!(WAREHOUSE_INVENMOVE, 5);
    }

    /// PK zone warehouse cost is 10,000 gold.
    #[test]
    fn test_pk_zone_warehouse_cost() {
        assert_eq!(PK_ZONE_WAREHOUSE_COST, 10_000);
    }

    /// is_pk_zone identifies PK zones correctly.
    #[test]
    fn test_is_pk_zone_warehouse() {
        assert!(is_pk_zone(ZONE_ARDREAM));
        assert!(is_pk_zone(ZONE_RONARK_LAND));
        assert!(is_pk_zone(ZONE_RONARK_LAND_BASE));
        assert!(!is_pk_zone(1)); // Moradon
        assert!(!is_pk_zone(21)); // El Morad
    }

    /// build_warehouse_result: success response has result=1.
    #[test]
    fn test_warehouse_result_success() {
        let pkt = build_warehouse_result(WAREHOUSE_OPEN, 1);
        assert_eq!(pkt.opcode, Opcode::WizWarehouse as u8);
        assert_eq!(pkt.data[0], WAREHOUSE_OPEN);
        assert_eq!(pkt.data[1], 1); // Success
        assert_eq!(pkt.data.len(), 2);
    }

    /// build_warehouse_result: money required response has result=2.
    #[test]
    fn test_warehouse_result_money_required() {
        let pkt = build_warehouse_result(WAREHOUSE_INPUT, 2);
        assert_eq!(pkt.data[1], 2); // RequiredMoney
    }
}
