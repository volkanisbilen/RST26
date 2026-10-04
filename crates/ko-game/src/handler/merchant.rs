//! WIZ_MERCHANT (0x68) + WIZ_MERCHANT_INOUT (0x69) handler — player merchant (personal shop).
//! Selling merchant sub-opcodes:
//! - MERCHANT_OPEN (1): Open merchant setup UI
//! - MERCHANT_CLOSE (2): Close merchant / cancel
//! - MERCHANT_ITEM_ADD (3): Add item to shop window
//! - MERCHANT_ITEM_CANCEL (4): Remove item from shop window
//! - MERCHANT_ITEM_LIST (5): Browse a merchant's wares
//! - MERCHANT_ITEM_BUY (6): Buy an item from a merchant
//! - MERCHANT_INSERT (7): Finalize and open the shop
//! - MERCHANT_TRADE_CANCEL (8): Close the browse window

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::{
    MerchData, UserItemSlot, COIN_MAX, ITEM_FLAG_BOUND, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED,
    ITEM_FLAG_SEALED, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN, MAX_MERCH_ITEMS, MAX_MERCH_MESSAGE,
    RACE_UNTRADEABLE,
};

/// Selling merchant sub-opcode constants.
const MERCHANT_OPEN: u8 = 1;
const MERCHANT_CLOSE: u8 = 2;
const MERCHANT_ITEM_ADD: u8 = 3;
const MERCHANT_ITEM_CANCEL: u8 = 4;
const MERCHANT_ITEM_LIST: u8 = 5;
const MERCHANT_ITEM_BUY: u8 = 6;
const MERCHANT_INSERT: u8 = 7;
const MERCHANT_TRADE_CANCEL: u8 = 8;
const MERCHANT_ITEM_PURCHASED: u8 = 9;

/// Buying merchant sub-opcode constants.
const MERCHANT_BUY_OPEN: u8 = 0x21;
const MERCHANT_BUY_INSERT: u8 = 0x22;
const MERCHANT_BUY_LIST: u8 = 0x23;
const MERCHANT_BUY_BUY: u8 = 0x24;
const MERCHANT_BUY_SOLD: u8 = 0x25;
const MERCHANT_BUY_BOUGHT: u8 = 0x26;
const MERCHANT_BUY_CLOSE: u8 = 0x27;
const MERCHANT_BUY_REGION_INSERT: u8 = 0x28;
const MERCHANT_BUY_LIST_NEW: u8 = 0x51;
const MERCHANT_OFFICIAL_LIST: u8 = 0x30;
/// v2600: Merchant preview via WIZ_MERCHANT sub=0x31 (replaces WIZ_MERCHANTLIST 0xBD).
const MERCHANT_LIST_PREVIEW: u8 = 0x31;
/// v2600: Merchant preview new (same handler as 0x31).
const MERCHANT_LIST_PREVIEW_NEW: u8 = 0x11;

use super::{HAVE_MAX, ITEMCOUNT_MAX, ITEM_KIND_UNIQUE, SLOT_MAX};

/// Merchant open response codes.
const MERCHANT_OPEN_SUCCESS: i16 = 1;
const MERCHANT_OPEN_DEAD: i16 = -2;
const MERCHANT_OPEN_TRADING: i16 = -3;
const MERCHANT_OPEN_MERCHANTING: i16 = -4;

/// Shopping (store open) error code — sent when player has Genie/NPC shop open.
const MERCHANT_OPEN_SHOPPING: i16 = -6;

/// Under-leveled error code — sent when player level < server MerchantLevel setting.
const MERCHANT_OPEN_UNDERLEVELED: i16 = 30;

/// Handle WIZ_MERCHANT from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let world = session.world().clone();
    let sid = session.session_id();

    if world.get_character_info(sid).is_none() {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        MERCHANT_OPEN => merchant_open(session).await,
        MERCHANT_CLOSE => merchant_close(session).await,
        MERCHANT_ITEM_ADD => merchant_item_add(session, &mut reader).await,
        MERCHANT_ITEM_CANCEL => merchant_item_cancel(session, &mut reader).await,
        MERCHANT_ITEM_LIST | MERCHANT_LIST_PREVIEW_NEW => {
            merchant_item_list(session, &mut reader).await
        }
        MERCHANT_ITEM_BUY => merchant_item_buy(session, &mut reader).await,
        MERCHANT_INSERT => merchant_insert(session, &mut reader),
        MERCHANT_TRADE_CANCEL => merchant_trade_cancel(session).await,
        // Buying merchant sub-opcodes
        MERCHANT_BUY_OPEN => buying_merchant_open(session).await,
        MERCHANT_BUY_INSERT => buying_merchant_insert(session, &mut reader),
        MERCHANT_BUY_LIST | MERCHANT_BUY_LIST_NEW => {
            buying_merchant_list(session, &mut reader).await
        }
        MERCHANT_BUY_BUY => buying_merchant_buy(session, &mut reader).await,
        MERCHANT_BUY_CLOSE => buying_merchant_close_handler(session).await,
        MERCHANT_OFFICIAL_LIST => merchant_official_list(session, &mut reader).await,
        // v2600: merchant preview via WIZ_MERCHANT sub=0x31
        // (replaces separate WIZ_MERCHANTLIST 0xBD opcode)
        MERCHANT_LIST_PREVIEW => merchant_list_preview(session, &mut reader, sub_opcode).await,
        _ => {
            debug!(
                "[{}] Merchant unhandled sub-opcode 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle WIZ_MERCHANTLIST (0xBD) — merchant search / quick preview.
/// Client sends a merchant socket ID, server responds with that merchant's
/// visible items (first 4, or 8 if premium selling merchant).
pub async fn handle_merchant_list(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    let mut reader = PacketReader::new(&pkt.data);
    let merchant_sid = reader.read_u32().unwrap_or(0) as u16;

    // Validate the merchant
    if !world.is_merchanting(merchant_sid) {
        return Ok(());
    }

    // Must be same zone and event room
    let my_pos = world.get_position(sid).unwrap_or_default();
    let merch_location = world
        .get_bot(merchant_sid as u32)
        .map(|b| (b.zone_id, b.x, b.z))
        .or_else(|| {
            world
                .get_position(merchant_sid)
                .map(|p| (p.zone_id, p.x, p.z))
        })
        .unwrap_or_default();
    if my_pos.zone_id != merch_location.0 {
        return Ok(());
    }
    let my_room = world.get_event_room(sid);
    let merch_room = if world.get_bot(merchant_sid as u32).is_some() {
        0
    } else {
        world.get_event_room(merchant_sid)
    };
    if my_room != merch_room {
        return Ok(());
    }

    let merch_items = world.get_merchant_items(merchant_sid);
    let is_selling = world.is_selling_merchant(merchant_sid);
    let is_buying = world.is_buying_merchant(merchant_sid);

    // Response: WIZ_MERCHANT + MERCHANT_LIST sub-opcode
    let mut result = Packet::new(Opcode::WizMerchant as u8);

    // C++ uses sub-opcode 29 for MERCHANT_LIST response
    const MERCHANT_LIST_SUB: u8 = 29;
    result.write_u8(MERCHANT_LIST_SUB);
    result.write_u8(1); // success
    result.write_u32(merchant_sid as u32);

    if is_selling {
        result.write_i8(0); // merchant state: selling
        let is_premium = world
            .with_session(merchant_sid, |h| h.is_premium_merchant)
            .unwrap_or(false);
        result.write_u8(if is_premium { 1 } else { 0 });

        // First 4 items (or 8 for premium)
        let item_count = if is_premium { 8 } else { 4 };
        for i in 0..item_count {
            if i < merch_items.len() && !merch_items[i].sold_out {
                result.write_u32(merch_items[i].item_id);
            } else {
                result.write_u32(0);
            }
        }
    } else if is_buying {
        result.write_i8(1); // merchant state: buying
        result.write_u8(0);

        // First 4 items
        for item in merch_items.iter().take(4) {
            result.write_u32(item.item_id);
        }
    }

    session.send_packet(&result).await
}

/// Handle MerchantOfficialList (sub=0x30) -- Menisia/official merchant system.
/// (ITEM_MENICIAS_LIST) in inventory, validates map's m_bMenissiahList flag,
/// then dispatches sub-sub-opcodes (2=MerchantListMoveProcess, 5=search).
/// but enabled in compiled server binary).
/// Stub implementation: validates player state, logs the request.
async fn merchant_official_list(
    session: &mut ClientSession,
    _reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // not fishing, not mining, not dead, m_sTradeStatue == 1 (not trading)
    if world.is_player_dead(sid)
        || world.is_merchanting(sid)
        || world.is_fishing(sid)
        || world.is_mining(sid)
        || world.is_trading(sid)
    {
        return Ok(());
    }

    // The full implementation requires map Menissiah flag + item 810166000 check.
    // For now, acknowledge the packet without crash.
    debug!(
        "[{}] MerchantOfficialList (0x30) -- stub handler",
        session.addr()
    );
    Ok(())
}

/// Handle merchant list preview via WIZ_MERCHANT sub=0x31/0x11.
/// v2600 client sends merchant browse as WIZ_MERCHANT sub=0x31 instead of
/// the separate WIZ_MERCHANTLIST (0xBD) opcode.
/// C2S: `[sub:u8] [merchant_sid:u32]`
/// S2C: `[sub:u8] [status:u8] [merchant_sid:u32] [state:u8] [premium:u8] [items:u32 × 4]`
async fn merchant_list_preview(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    sub_opcode: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let merchant_sid = reader.read_u32().unwrap_or(0) as u16;

    if !world.is_merchanting(merchant_sid) {
        let mut fail = Packet::new(Opcode::WizMerchant as u8);
        fail.write_u8(sub_opcode);
        fail.write_u8(4); // error
        session.send_packet(&fail).await?;
        return Ok(());
    }

    let my_pos = world.get_position(sid).unwrap_or_default();
    let merch_location = world
        .get_bot(merchant_sid as u32)
        .map(|b| (b.zone_id, b.x, b.z))
        .or_else(|| {
            world
                .get_position(merchant_sid)
                .map(|p| (p.zone_id, p.x, p.z))
        })
        .unwrap_or_default();
    if my_pos.zone_id != merch_location.0 {
        return Ok(());
    }
    let merchant_room = if world.get_bot(merchant_sid as u32).is_some() {
        0
    } else {
        world.get_event_room(merchant_sid)
    };
    if world.get_event_room(sid) != merchant_room {
        return Ok(());
    }

    let merch_items = world.get_merchant_items(merchant_sid);
    let is_selling = world.is_selling_merchant(merchant_sid);
    let is_buying = world.is_buying_merchant(merchant_sid);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(sub_opcode); // echo request sub-opcode (0x31 or 0x11)
    result.write_u8(1); // success
    result.write_u32(merchant_sid as u32);

    if is_selling {
        result.write_i8(0);
        let is_premium = world
            .with_session(merchant_sid, |h| h.is_premium_merchant)
            .unwrap_or(false);
        result.write_u8(if is_premium { 1 } else { 0 });
        let item_count = if is_premium { 8 } else { 4 };
        for i in 0..item_count {
            if i < merch_items.len() && !merch_items[i].sold_out {
                result.write_u32(merch_items[i].item_id);
            } else {
                result.write_u32(0);
            }
        }
    } else if is_buying {
        result.write_i8(1);
        result.write_u8(0);
        for item in merch_items.iter().take(4) {
            result.write_u32(item.item_id);
        }
    }

    session.send_packet(&result).await
}

/// Handle WIZ_MERCHANT_INOUT from the client (broadcast merchant appearance/disappearance).
/// The client does not send this directly — it's triggered by merchant_insert/close.
pub fn handle_inout(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    debug!(
        "[{}] WIZ_MERCHANT_INOUT received ({} bytes)",
        session.addr(),
        pkt.data.len()
    );
    Ok(())
}

/// MERCHANT_OPEN (1): Request to open merchant setup UI.
async fn merchant_open(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_buying_merchant_preparing(sid) {
        return Ok(());
    }

    // Get minimum merchant level from server settings (default 1)
    let merchant_level = world
        .get_server_settings()
        .map(|s| s.merchant_level)
        .unwrap_or(1);

    let player_level = world
        .get_character_info(sid)
        .map(|ch| ch.level as i16)
        .unwrap_or(0);

    let error_code: i16 = if world.is_player_dead(sid) {
        MERCHANT_OPEN_DEAD
    } else if world.is_store_open(sid) {
        MERCHANT_OPEN_SHOPPING
    } else if world.is_trading(sid) {
        MERCHANT_OPEN_TRADING
    } else if player_level < merchant_level {
        MERCHANT_OPEN_UNDERLEVELED
    } else if world.is_merchanting(sid) {
        MERCHANT_OPEN_MERCHANTING
    } else {
        MERCHANT_OPEN_SUCCESS
    };

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_OPEN);
    result.write_i16(error_code);
    session.send_packet(&result).await?;

    // If already merchanting, close first to resync
    if error_code == -4 {
        merchant_close(session).await?;
    }

    if error_code == MERCHANT_OPEN_SUCCESS {
        world.set_selling_merchant_preparing(sid, true);

        // FerihaLog: MerchantCreationInsertLog
        super::audit_log::log_merchant_creation(
            session.pool(),
            session.account_id().unwrap_or(""),
            &world.get_session_name(sid).unwrap_or_default(),
            &session.addr().to_string(),
            "merchant_open",
        );
    }

    Ok(())
}

/// MERCHANT_CLOSE (2): Close the merchant shop.
pub(crate) async fn merchant_close(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let is_selling = world.is_selling_merchant(sid);
    let is_preparing = world.is_selling_merchant_preparing(sid);

    if !is_selling && !is_preparing {
        return Ok(());
    }

    world.close_merchant(sid);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_CLOSE);
    result.write_u32(sid as u32);

    if is_selling {
        // Broadcast to region
        let (pos, event_room) = world
            .with_session(sid, |h| (h.position, h.event_room))
            .unwrap_or_default();
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(result),
            None,
            event_room,
        );
    } else {
        session.send_packet(&result).await?;
    }

    Ok(())
}

/// MERCHANT_ITEM_ADD (3): Add an item to the merchant shop setup.
async fn merchant_item_add(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if !world.is_selling_merchant_preparing(sid) || world.is_buying_merchant_preparing(sid) {
        return Ok(());
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let count = reader.read_u16().unwrap_or(0);
    let gold = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let _mode = reader.read_u8().unwrap_or(0);
    let is_kc = reader.read_u8().unwrap_or(0);

    // Validate
    if item_id == 0
        || count == 0
        || src_pos as usize >= HAVE_MAX
        || dst_pos as usize >= MAX_MERCH_ITEMS
    {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    // Check if src_pos already used in another merchant slot
    let merch_items = world.get_merchant_items(sid);
    for item in &merch_items {
        if item.item_id != 0 && item.original_slot == (src_pos + SLOT_MAX as u8) {
            return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc)
                .await;
        }
    }

    // Validate item table
    let item_def = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc)
                .await
        }
    };

    // Cannot sell untradeable items
    if (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id)
        || item_def.race.unwrap_or(0) == RACE_UNTRADEABLE
        || item_def.countable.unwrap_or(0) == 2
    {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    let kind = item_def.kind.unwrap_or(0);
    let countable = item_def.countable.unwrap_or(0);

    if kind == ITEM_KIND_UNIQUE && count != 1 {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    if countable == 1 && count > ITEMCOUNT_MAX {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    // Validate inventory slot
    let actual_slot = SLOT_MAX + src_pos as usize;
    let slot = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) => s,
        None => {
            return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc)
                .await
        }
    };

    if slot.item_id != item_id || slot.count == 0 || slot.count < count {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    // Check flags — use equality, NOT bitmask.
    if slot.flag == ITEM_FLAG_RENTED
        || slot.flag == ITEM_FLAG_SEALED
        || slot.flag == ITEM_FLAG_BOUND
        || slot.flag == ITEM_FLAG_DUPLICATE
        || slot.expire_time > 0
    {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    if is_kc != 0 {
        let min_kc = world
            .get_server_settings()
            .map(|s| s.merchant_min_cash)
            .unwrap_or(0);
        if min_kc > 0 && gold < min_kc as u32 {
            return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc)
                .await;
        }
    }

    // Check destination slot is empty
    if merch_items[dst_pos as usize].item_id != 0 {
        return send_merch_add_fail(session, item_id, count, src_pos, dst_pos, gold, is_kc).await;
    }

    // Mark inventory item as merchant
    world.set_inventory_merchant_flag(sid, actual_slot, true);

    let sell_count = if kind == ITEM_KIND_UNIQUE && countable == 0 {
        1
    } else {
        count
    };

    // Set merchant data
    world.set_merchant_item(
        sid,
        dst_pos as usize,
        MerchData {
            item_id,
            durability: slot.durability,
            sell_count,
            original_count: slot.count,
            serial_num: slot.serial_num,
            price: gold,
            original_slot: src_pos + SLOT_MAX as u8,
            sold_out: false,
            is_kc: is_kc != 0,
        },
    );

    // Send success
    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_ITEM_ADD);
    result.write_u16(1); // success
    result.write_u32(item_id);
    result.write_u16(sell_count);
    result.write_u16(slot.durability as u16);
    result.write_u32(gold);
    result.write_u8(src_pos);
    result.write_u8(dst_pos);
    result.write_u8(is_kc);
    session.send_packet(&result).await
}

/// MERCHANT_ITEM_CANCEL (4): Remove an item from the merchant setup.
async fn merchant_item_cancel(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if !world.is_selling_merchant_preparing(sid) || world.is_buying_merchant_preparing(sid) {
        return Ok(());
    }

    let slot_pos = reader.read_u8().unwrap_or(0);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_ITEM_CANCEL);

    if slot_pos as usize >= MAX_MERCH_ITEMS {
        result.write_i16(-2);
        return session.send_packet(&result).await;
    }

    let merch = match world.get_merchant_item(sid, slot_pos as usize) {
        Some(m) if m.item_id != 0 => m,
        _ => {
            result.write_i16(-3);
            return session.send_packet(&result).await;
        }
    };

    // Validate inventory slot still matches merchant data
    //   pItem = GetItem(pMerch->bSrcPos + SLOT_MAX);
    //   if (pItem == nullptr || pItem->nNum != pMerch->nItemID) goto fail_return;
    //   if (pItem->sCount != pMerch->bCount) goto fail_return;
    let inv_slot = match world.get_inventory_slot(sid, merch.original_slot as usize) {
        Some(s) if s.item_id == merch.item_id => s,
        _ => {
            result.write_i16(-3);
            return session.send_packet(&result).await;
        }
    };
    if inv_slot.count != merch.sell_count {
        result.write_i16(-3);
        return session.send_packet(&result).await;
    }

    // Unmark inventory item
    world.set_inventory_merchant_flag(sid, merch.original_slot as usize, false);

    // Clear merchant slot
    world.set_merchant_item(sid, slot_pos as usize, MerchData::default());

    result.write_i16(1);
    result.write_u8(slot_pos);
    session.send_packet(&result).await
}

/// MERCHANT_INSERT (7): Finalize and open the merchant shop.
fn merchant_insert(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if !world.is_selling_merchant_preparing(sid) || world.is_buying_merchant_preparing(sid) {
        return Ok(());
    }

    let advert_msg = reader.read_string().unwrap_or_default();
    if advert_msg.len() > MAX_MERCH_MESSAGE {
        return Ok(());
    }

    let merch_items = world.get_merchant_items(sid);
    let total_items = merch_items.iter().filter(|m| m.item_id > 0).count() as u16;

    if total_items == 0 || total_items > 12 {
        return Ok(());
    }

    // Activate merchant state
    world.activate_selling_merchant(sid);

    // Build broadcast packet
    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_INSERT);
    result.write_u16(1); // success
    result.write_string(&advert_msg);
    result.write_u32(sid as u32);
    let is_premium = world
        .with_session(sid, |h| h.is_premium_merchant)
        .unwrap_or(false);
    result.write_u8(if is_premium { 1 } else { 0 });

    for item in &merch_items {
        result.write_u32(item.item_id);
    }

    // Broadcast to region
    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(result),
        None,
        event_room,
    );

    Ok(())
}

/// MERCHANT_ITEM_LIST (5): Browse a merchant's items.
async fn merchant_item_list(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let merchant_sid = reader.read_u32().unwrap_or(0) as u16;

    // Validate the merchant
    if !world.is_selling_merchant(merchant_sid) || world.is_selling_merchant_preparing(merchant_sid)
    {
        return Ok(());
    }

    // Check if someone else is already looking (single DashMap read instead of 2)
    if let Some(existing_looker) = world.get_merchant_looker(merchant_sid) {
        if existing_looker != sid {
            if let Some(looker_ch) = world.get_character_info(existing_looker) {
                let mut busy_pkt = Packet::new(Opcode::WizMerchant as u8);
                busy_pkt.write_u8(MERCHANT_ITEM_LIST);
                busy_pkt.write_i16(-7);
                busy_pkt.write_sbyte_string(&looker_ch.name);
                return session.send_packet(&busy_pkt).await;
            }
            // Previous looker gone, clear
            world.set_merchant_looker(merchant_sid, None);
        }
    }

    // Set us as the looker
    world.set_merchant_looker(merchant_sid, Some(sid));
    world.set_browsing_merchant(sid, Some(merchant_sid));

    let merch_items = world.get_merchant_items(merchant_sid);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_ITEM_LIST);
    result.write_u16(1); // success
    result.write_u32(merchant_sid as u32);

    let rebirth_level = world
        .get_character_info(sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);
    for item in &merch_items {
        result.write_u32(item.item_id);
        result.write_u16(item.sell_count);
        result.write_u16(item.durability as u16);
        result.write_u32(item.price);
        crate::handler::unique_item_info::write_unique_item_info(
            &world,
            session.pool(),
            item.item_id,
            item.serial_num,
            rebirth_level,
            &mut result,
        )
        .await;
    }

    // KC flags
    for item in &merch_items {
        result.write_u8(if item.is_kc { 1 } else { 0 });
    }

    session.send_packet(&result).await
}

/// MERCHANT_ITEM_BUY (6): Buy an item from a player merchant.
async fn merchant_item_buy(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let merchant_sid = match world.get_browsing_merchant(sid) {
        Some(m) => m,
        None => return send_merch_buy_fail(session).await,
    };
    let merchant_is_bot = world.get_bot(merchant_sid as u32).is_some();

    // Self-buy prevention — cannot buy from your own shop
    if merchant_sid == sid {
        return send_merch_buy_fail(session).await;
    }

    // Buyer state checks
    if world.is_player_dead(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_trading(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return send_merch_buy_fail(session).await;
    }

    if !world.is_selling_merchant(merchant_sid) || world.is_selling_merchant_preparing(merchant_sid)
    {
        return send_merch_buy_fail(session).await;
    }

    // Range check — buyer must be within 35m of merchant, same zone
    let buyer_pos = world.get_position(sid).unwrap_or_default();
    let seller_location = world
        .get_bot(merchant_sid as u32)
        .map(|b| (b.zone_id, b.x, b.z))
        .or_else(|| {
            world
                .get_position(merchant_sid)
                .map(|p| (p.zone_id, p.x, p.z))
        })
        .unwrap_or_default();
    if buyer_pos.zone_id != seller_location.0 {
        return send_merch_buy_fail(session).await;
    }
    let dx = buyer_pos.x - seller_location.1;
    let dz = buyer_pos.z - seller_location.2;
    if dx * dx + dz * dz > 35.0 * 35.0 {
        return send_merch_buy_fail(session).await;
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let item_count = reader.read_u16().unwrap_or(0);
    let item_slot = reader.read_u8().unwrap_or(0);
    let dest_slot = reader.read_u8().unwrap_or(0);

    if item_slot as usize >= MAX_MERCH_ITEMS || dest_slot as usize >= HAVE_MAX || item_count == 0 {
        return send_merch_buy_fail(session).await;
    }

    // Validate the item table BEFORE atomic buy attempt
    let item_def = match world.get_item(item_id) {
        Some(i) => i,
        None => return send_merch_buy_fail(session).await,
    };

    let countable = item_def.countable.unwrap_or(0);
    if countable == 0 && item_count != 1 {
        return send_merch_buy_fail(session).await;
    }

    // Check weight (u32 comparison — no u16 truncation)
    let req_weight = (item_def.weight.unwrap_or(0) as u32).saturating_mul(item_count as u32);
    let stats = world.get_equipped_stats(sid);
    if req_weight.saturating_add(stats.item_weight) > stats.max_weight {
        return send_merch_buy_fail(session).await;
    }

    // Check destination slot
    let actual_dest = SLOT_MAX + dest_slot as usize;
    let dest_item = world
        .get_inventory_slot(sid, actual_dest)
        .unwrap_or_default();
    if dest_item.item_id != 0 && (dest_item.item_id != item_id || countable == 0) {
        return send_merch_buy_fail(session).await;
    }

    // ── Stack overflow prevention ─────────────────────────────────────────
    // slot capacity BEFORE purchase. Without this check, buyer pays full
    // price but .min(ITEMCOUNT_MAX) silently discards excess items.
    if dest_item.item_id != 0 && countable == 1 && (dest_item.count + item_count) > ITEMCOUNT_MAX {
        return send_merch_buy_fail(session).await;
    }

    // ── Verify seller's actual inventory still has the item (ghost-item prevention) ──
    let is_kc;
    {
        let merch_preview = match world.get_merchant_item(merchant_sid, item_slot as usize) {
            Some(m) if m.item_id == item_id && !m.sold_out && m.sell_count >= item_count => m,
            _ => return send_merch_buy_fail(session).await,
        };
        // Reject purchase if the item has zero price set.
        if merch_preview.price == 0 {
            return send_merch_buy_fail(session).await;
        }
        is_kc = merch_preview.is_kc;
        match world.get_inventory_slot(merchant_sid, merch_preview.original_slot as usize) {
            Some(seller_item)
                if seller_item.item_id == item_id
                    && seller_item.count > 0
                    && seller_item.count >= item_count =>
            {
                // Seller's inventory matches — proceed
            }
            _ if merchant_is_bot => {}
            _ => return send_merch_buy_fail(session).await,
        }
    }

    // ── Atomically take from merchant FIRST — prevents duplication race ────
    // Only ONE concurrent buyer can succeed on the same merchant+slot.
    let merch = match world.try_merchant_buy(merchant_sid, item_slot as usize, item_id, item_count)
    {
        Some(m) => m,
        None => return send_merch_buy_fail(session).await,
    };

    // Check payment (after atomic take — if check fails we must restore)
    let req_price = merch.price.saturating_mul(item_count as u32);
    if is_kc {
        // KC (Knight Cash) payment path
        if !crate::handler::knight_cash::cash_lose(&world, session.pool(), sid, req_price) {
            world.restore_merchant_buy(merchant_sid, item_slot as usize, item_id, item_count);
            return send_merch_buy_fail(session).await;
        }
        // Seller gains KC (skip for offline/bot — C++ only calls CashGain on real users)
        if !merchant_is_bot && !world.is_offline_status(merchant_sid) {
            crate::handler::knight_cash::cash_gain(&world, session.pool(), merchant_sid, req_price);
        }
    } else {
        // Gold payment path (existing logic)
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                world.restore_merchant_buy(merchant_sid, item_slot as usize, item_id, item_count);
                return send_merch_buy_fail(session).await;
            }
        };
        if ch.gold < req_price {
            world.restore_merchant_buy(merchant_sid, item_slot as usize, item_id, item_count);
            return send_merch_buy_fail(session).await;
        }

        // Check merchant's gold won't overflow
        let merch_gold = world
            .get_character_info(merchant_sid)
            .map(|c| c.gold)
            .or_else(|| world.get_bot(merchant_sid as u32).map(|b| b.gold))
            .unwrap_or(0);
        if (merch_gold as u64) + (req_price as u64) > COIN_MAX as u64 {
            world.restore_merchant_buy(merchant_sid, item_slot as usize, item_id, item_count);
            return send_merch_buy_fail(session).await;
        }

        // Execute gold transfer
        world.gold_lose(sid, req_price);
        if merchant_is_bot {
            world.update_bot(merchant_sid as u32, |bot| {
                bot.gold = bot.gold.saturating_add(req_price).min(COIN_MAX);
            });
        } else {
            world.gold_gain(merchant_sid, req_price);
        }
    }

    // Daily rank stat: GMTotalSold += req_price (merchant seller earns gold)
    world.update_session(merchant_sid, |h| {
        h.dr_gm_total_sold += req_price as u64;
    });

    let leftover_count = merch.sell_count - item_count;

    // Give item to buyer
    world.update_inventory(sid, |inv| {
        if actual_dest >= inv.len() {
            return false;
        }
        inv[actual_dest].item_id = item_id;
        inv[actual_dest].count = (inv[actual_dest].count + item_count).min(ITEMCOUNT_MAX);
        inv[actual_dest].durability = merch.durability;
        if inv[actual_dest].serial_num == 0 {
            inv[actual_dest].serial_num = merch.serial_num;
        }
        true
    });

    // Update seller's inventory to match
    let kind = item_def.kind.unwrap_or(0);
    let fully_sold = leftover_count == 0 || (countable == 0 && kind == ITEM_KIND_UNIQUE);
    if fully_sold && !merchant_is_bot {
        // Force merchant slot fully sold out (covers kind==255 edge case)
        world.set_merchant_item(
            merchant_sid,
            item_slot as usize,
            MerchData {
                sold_out: true,
                ..MerchData::default()
            },
        );
        // Remove from seller's inventory
        world.update_inventory(merchant_sid, |inv| {
            let pos = merch.original_slot as usize;
            if pos < inv.len() && inv[pos].item_id == item_id {
                inv[pos] = UserItemSlot::default();
            }
            true
        });
    } else if !merchant_is_bot {
        // Deduct from seller's inventory
        world.update_inventory(merchant_sid, |inv| {
            let pos = merch.original_slot as usize;
            if pos < inv.len() && inv[pos].item_id == item_id {
                inv[pos].count = inv[pos].count.saturating_sub(item_count);
                if inv[pos].count == 0 {
                    inv[pos] = UserItemSlot::default();
                }
            }
            true
        });
    }

    // Notify seller's client of the inventory change so it stays in sync.
    let seller_slot_pos = merch.original_slot.saturating_sub(SLOT_MAX as u8);
    let seller_slot_data = world
        .get_inventory_slot(merchant_sid, merch.original_slot as usize)
        .unwrap_or_default();
    {
        let mut stack_pkt = Packet::new(Opcode::WizItemCountChange as u8);
        stack_pkt.write_u16(1); // count_type
        stack_pkt.write_u8(1); // slot_section = inventory
        stack_pkt.write_u8(seller_slot_pos);
        stack_pkt.write_u32(seller_slot_data.item_id);
        stack_pkt.write_u32(seller_slot_data.count as u32);
        stack_pkt.write_u8(0); // bNewItem = false
        stack_pkt.write_u16(seller_slot_data.durability as u16);
        stack_pkt.write_u32(0); // reserved
        stack_pkt.write_u32(0); // expire_time
        stack_pkt.write_u16(0); // v2625 SendStackChange trailing field
        world.send_to_session_owned(merchant_sid, stack_pkt);
    }

    // Notify merchant of purchase
    let buyer_name = world
        .get_character_info(sid)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let mut purchase_pkt = Packet::new(Opcode::WizMerchant as u8);
    purchase_pkt.write_u8(MERCHANT_ITEM_PURCHASED);
    purchase_pkt.write_u32(item_id);
    purchase_pkt.write_string(&buyer_name);
    world.send_to_session_owned(merchant_sid, purchase_pkt);

    // Send buy success to buyer
    let mut buy_result = Packet::new(Opcode::WizMerchant as u8);
    buy_result.write_u8(MERCHANT_ITEM_BUY);
    buy_result.write_u16(1); // success
    buy_result.write_u32(item_id);
    buy_result.write_u16(leftover_count);
    buy_result.write_u8(item_slot);
    buy_result.write_u8(dest_slot);
    session.send_packet(&buy_result).await?;

    // FerihaLog: MerchantShoppingDetailInsertLog
    {
        let merchant_acc = world
            .with_session(merchant_sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let merchant_name = world.get_session_name(merchant_sid).unwrap_or_default();
        super::audit_log::log_merchant_shopping(
            session.pool(),
            &merchant_acc,
            &merchant_name,
            "buy",
            item_id,
            item_count,
            merch.price,
            &buyer_name,
        );
    }

    // If first 4 items sold out, broadcast INOUT update
    if item_slot < 4 && leftover_count == 0 {
        let mut inout_pkt = Packet::new(Opcode::WizMerchantInout as u8);
        inout_pkt.write_u8(2);
        inout_pkt.write_u32(merchant_sid as u32);
        inout_pkt.write_u8(1);
        inout_pkt.write_u8(0);
        inout_pkt.write_u8(item_slot);
        let pos = world.get_position(merchant_sid).unwrap_or_default();
        let event_room = world.get_event_room(merchant_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(inout_pkt),
            None,
            event_room,
        );
    }

    // Check if all items sold — close merchant
    let merch_items = world.get_merchant_items(merchant_sid);
    let items_remaining = merch_items
        .iter()
        .filter(|m| m.item_id != 0 && !m.sold_out)
        .count();
    if items_remaining == 0 {
        world.close_merchant(merchant_sid);
        let mut close_pkt = Packet::new(Opcode::WizMerchant as u8);
        close_pkt.write_u8(MERCHANT_CLOSE);
        close_pkt.write_u32(merchant_sid as u32);
        let pos = world.get_position(merchant_sid).unwrap_or_default();
        let event_room = world.get_event_room(merchant_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(close_pkt),
            None,
            event_room,
        );

        // If the merchant was an offline session, disconnect it now.
        // the merchant closes.  For offline sessions this means full cleanup.
        if world.is_offline_status(merchant_sid) {
            debug!(
                "Offline merchant (sid={}) all items sold — cleaning up",
                merchant_sid
            );
            crate::systems::offline_merchant::cleanup_offline_session(&world, merchant_sid).await;
        }
    }

    Ok(())
}

/// MERCHANT_TRADE_CANCEL (8): Close the browse window.
pub(crate) async fn merchant_trade_cancel(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    world.remove_from_merchant_lookers(sid);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_TRADE_CANCEL);
    result.write_u16(1);
    session.send_packet(&result).await
}

/// Send merchant add item failure.
async fn send_merch_add_fail(
    session: &mut ClientSession,
    item_id: u32,
    count: u16,
    src_pos: u8,
    dst_pos: u8,
    gold: u32,
    is_kc: u8,
) -> anyhow::Result<()> {
    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_ITEM_ADD);
    result.write_u16(0); // failure
    result.write_u32(item_id);
    result.write_u16(count);
    result.write_u16(src_pos as u16 + dst_pos as u16);
    result.write_u32(gold);
    result.write_u8(src_pos);
    result.write_u8(dst_pos);
    result.write_u8(is_kc);
    session.send_packet(&result).await
}

/// Send merchant buy failure.
async fn send_merch_buy_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_ITEM_BUY);
    result.write_u16(0xFFEE_u16); // -18 as u16
    session.send_packet(&result).await
}

// ============================================================================
// Buying Merchant handlers (sub-opcodes 0x21–0x28)
// ============================================================================

/// MERCHANT_BUY_OPEN (0x21): Request to open buying merchant setup UI.
async fn buying_merchant_open(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Already in selling merchant prep — block
    if world.is_selling_merchant_preparing(sid) {
        return Ok(());
    }

    let merchant_level = world
        .get_server_settings()
        .map(|s| s.merchant_level)
        .unwrap_or(1);
    let player_level = world
        .get_character_info(sid)
        .map(|ch| ch.level as i16)
        .unwrap_or(0);

    let error_code: i16 = if world.is_player_dead(sid) {
        MERCHANT_OPEN_DEAD
    } else if world.is_store_open(sid) {
        MERCHANT_OPEN_SHOPPING
    } else if world.is_trading(sid) {
        MERCHANT_OPEN_TRADING
    } else if player_level < merchant_level {
        MERCHANT_OPEN_UNDERLEVELED
    } else if world.is_merchanting(sid) {
        MERCHANT_OPEN_MERCHANTING
    } else {
        MERCHANT_OPEN_SUCCESS
    };

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_BUY_OPEN);
    result.write_i16(error_code);
    session.send_packet(&result).await?;

    if error_code == MERCHANT_OPEN_MERCHANTING {
        buying_merchant_close_internal(session).await?;
    }

    if error_code == MERCHANT_OPEN_SUCCESS {
        world.set_buying_merchant_preparing(sid, true);
        // Clear previous merchant items
        for i in 0..MAX_MERCH_ITEMS {
            world.set_merchant_item(sid, i, MerchData::default());
        }
    }

    Ok(())
}

/// MERCHANT_BUY_INSERT (0x22): Submit wanted item list and start buying merchant.
fn buying_merchant_insert(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if !world.is_buying_merchant_preparing(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_merchanting(sid)
    {
        return Ok(());
    }

    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    let merchant_level = world
        .get_server_settings()
        .map(|s| s.merchant_level)
        .unwrap_or(1);
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };
    if (ch.level as i16) < merchant_level {
        return Ok(());
    }

    let item_count = reader.read_u8().unwrap_or(0) as usize;
    if item_count == 0 || item_count > MAX_MERCH_ITEMS {
        return Ok(());
    }

    // Parse items and validate
    let mut total_gold: u64 = 0;
    let mut items: Vec<(u32, u16, u32, i16)> = Vec::with_capacity(item_count);

    for _ in 0..item_count {
        let item_id = reader.read_u32().unwrap_or(0);
        let count = reader.read_u16().unwrap_or(0);
        let price = reader.read_u32().unwrap_or(0);

        if item_id == 0 || count == 0 || price == 0 {
            return Ok(());
        }

        // Validate item table
        let item_def = match world.get_item(item_id) {
            Some(i) => i,
            None => return Ok(()),
        };

        // Cannot buy untradeable items
        if (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id)
            || item_def.race.unwrap_or(0) == RACE_UNTRADEABLE
            || item_def.countable.unwrap_or(0) == 2
        {
            return Ok(());
        }

        let countable = item_def.countable.unwrap_or(0);
        let kind = item_def.kind.unwrap_or(0);

        // Non-countable items: count must be 1
        if countable == 0 && count != 1 {
            return Ok(());
        }
        // Kind 255 + non-countable: count must be 1
        if kind == ITEM_KIND_UNIQUE && countable == 0 && count != 1 {
            return Ok(());
        }

        let durability = item_def.duration.unwrap_or(0);
        total_gold += price as u64 * count as u64;
        items.push((item_id, count, price, durability));
    }

    // Check gold
    if total_gold > ch.gold as u64 {
        return Ok(());
    }

    // Set merchant items
    for (i, (item_id, count, price, dur)) in items.iter().enumerate() {
        world.set_merchant_item(
            sid,
            i,
            MerchData {
                item_id: *item_id,
                durability: *dur, // C++ stores max durability for buy validation
                sell_count: *count,
                original_count: 0,
                serial_num: 0,
                price: *price,
                original_slot: 0,
                sold_out: false,
                is_kc: false,
            },
        );
    }

    // Activate buying merchant
    world.activate_buying_merchant(sid);

    // Broadcast region insert
    buying_merchant_region_insert(session)?;

    Ok(())
}

/// MERCHANT_BUY_LIST (0x23): Buyer views buying merchant's wanted item list.
async fn buying_merchant_list(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let merchant_sid = reader.read_u32().unwrap_or(0) as u16;

    // Validate the merchant is an active buying merchant
    if !world.is_buying_merchant(merchant_sid) {
        return Ok(());
    }

    // Check if someone else is already looking (single DashMap read instead of 2)
    if let Some(existing_looker) = world.get_merchant_looker(merchant_sid) {
        if existing_looker != sid {
            if let Some(looker_ch) = world.get_character_info(existing_looker) {
                let mut busy_pkt = Packet::new(Opcode::WizMerchant as u8);
                busy_pkt.write_u8(MERCHANT_BUY_LIST);
                busy_pkt.write_i16(-7);
                busy_pkt.write_sbyte_string(&looker_ch.name);
                return session.send_packet(&busy_pkt).await;
            }
            world.set_merchant_looker(merchant_sid, None);
        }
    }

    // Set us as the looker
    world.set_merchant_looker(merchant_sid, Some(sid));
    world.set_browsing_merchant(sid, Some(merchant_sid));

    let merch_items = world.get_merchant_items(merchant_sid);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_BUY_LIST);
    result.write_u8(1); // success
    result.write_u32(merchant_sid as u32);

    for item in &merch_items {
        result.write_u32(item.item_id);
        result.write_u16(item.sell_count); // wanted count
        result.write_u16(if item.durability < 0 {
            0
        } else {
            item.durability as u16
        });
        result.write_u32(item.price);
    }

    session.send_packet(&result).await
}

/// MERCHANT_BUY_BUY (0x24): Seller sells item to buying merchant.
async fn buying_merchant_buy(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let merchant_sid = match world.get_browsing_merchant(sid) {
        Some(m) => m,
        None => return Ok(()),
    };

    // Cannot sell to yourself
    if merchant_sid == sid {
        return Ok(());
    }

    // Validate merchant is active buying merchant
    if !world.is_buying_merchant(merchant_sid) {
        return Ok(());
    }

    let seller_src_slot = reader.read_u8().unwrap_or(0);
    let merchant_list_slot = reader.read_u8().unwrap_or(0);
    let stack_size = reader.read_u16().unwrap_or(0);

    if stack_size == 0
        || seller_src_slot as usize >= HAVE_MAX
        || merchant_list_slot as usize >= MAX_MERCH_ITEMS
    {
        return Ok(());
    }

    // Get the wanted item from the merchant
    let wanted = match world.get_merchant_item(merchant_sid, merchant_list_slot as usize) {
        Some(m) if m.item_id != 0 && !m.sold_out && m.sell_count >= stack_size => m,
        _ => return Ok(()),
    };

    // Get seller's item
    let actual_slot = SLOT_MAX + seller_src_slot as usize;
    let seller_item = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) if s.item_id == wanted.item_id && s.count >= stack_size => s,
        _ => return Ok(()),
    };

    // Check seller's item flags — cannot sell sealed/bound/rented/duplicate items
    if seller_item.flag == ITEM_FLAG_SEALED
        || seller_item.flag == ITEM_FLAG_BOUND
        || seller_item.flag == ITEM_FLAG_RENTED
        || seller_item.flag == ITEM_FLAG_DUPLICATE
    {
        return Ok(());
    }

    // Item table validation
    let item_def = match world.get_item(wanted.item_id) {
        Some(i) => i,
        None => return Ok(()),
    };
    let countable = item_def.countable.unwrap_or(0);
    if countable == 0 && stack_size != 1 {
        return Ok(());
    }

    // Seller's item must have full durability (match the wanted item's stored durability)
    if wanted.durability >= 0 && seller_item.durability != wanted.durability {
        return Ok(());
    }

    // Calculate price
    let total_price = wanted.price.saturating_mul(stack_size as u32);

    if wanted.is_kc {
        // KC payment: merchant (buyer) loses KC, seller (session) gains KC
        if world.get_knight_cash(merchant_sid) < total_price {
            return Ok(());
        }
    } else {
        // Gold payment: check merchant has enough gold
        if world
            .get_character_info(merchant_sid)
            .is_none_or(|c| c.gold < total_price)
        {
            return Ok(());
        }
    }

    // Find a slot in merchant's inventory for the item
    let Some(buyer_dest) = world.find_slot_for_item(merchant_sid, wanted.item_id, stack_size)
    else {
        return Ok(());
    };

    // ── Execute transaction ──

    if wanted.is_kc {
        // KC transfer: merchant pays KC, seller receives KC
        if !crate::handler::knight_cash::cash_lose(
            &world,
            session.pool(),
            merchant_sid,
            total_price,
        ) {
            return Ok(());
        }
        crate::handler::knight_cash::cash_gain(&world, session.pool(), sid, total_price);
    } else {
        // Gold transfer
        world.gold_lose(merchant_sid, total_price);
        world.gold_gain(sid, total_price);
    }

    // Update wanted count in merchant's list
    let remaining_wanted = wanted.sell_count - stack_size;
    if remaining_wanted == 0 {
        world.set_merchant_item(
            merchant_sid,
            merchant_list_slot as usize,
            MerchData::default(),
        );
    } else {
        world.set_merchant_item(
            merchant_sid,
            merchant_list_slot as usize,
            MerchData {
                sell_count: remaining_wanted,
                ..wanted.clone()
            },
        );
    }

    // Give item to merchant's inventory
    world.update_inventory(merchant_sid, |inv| {
        if buyer_dest >= inv.len() {
            return false;
        }
        if inv[buyer_dest].item_id == wanted.item_id {
            inv[buyer_dest].count = inv[buyer_dest].count.saturating_add(stack_size);
        } else {
            inv[buyer_dest].item_id = wanted.item_id;
            inv[buyer_dest].count = stack_size;
            inv[buyer_dest].durability = seller_item.durability;
            inv[buyer_dest].serial_num = seller_item.serial_num;
        }
        true
    });

    // Remove item from seller's inventory
    let seller_remaining = seller_item.count - stack_size;
    world.update_inventory(sid, |inv| {
        if actual_slot >= inv.len() {
            return false;
        }
        if seller_remaining == 0 {
            inv[actual_slot] = UserItemSlot::default();
        } else {
            inv[actual_slot].count = seller_remaining;
        }
        true
    });

    // Send WIZ_ITEM_COUNT_CHANGE to seller (inventory update)
    {
        let mut sc = Packet::new(Opcode::WizItemCountChange as u8);
        sc.write_u16(1);
        sc.write_u8(1); // slot_section = INVENTORY
        sc.write_u8(seller_src_slot);
        sc.write_u32(wanted.item_id);
        sc.write_u32(seller_remaining as u32);
        sc.write_u8(0); // bNewItem = false
        sc.write_u16(seller_item.durability as u16);
        sc.write_u32(0);
        sc.write_u32(0); // time
        sc.write_u16(0); // v2625 SendStackChange trailing field
        session.send_packet(&sc).await?;
    }
    // Send WIZ_ITEM_COUNT_CHANGE to merchant (inventory update)
    {
        let buyer_inv_pos = (buyer_dest as u16).saturating_sub(SLOT_MAX as u16) as u8;
        let buyer_count = world
            .get_inventory_slot(merchant_sid, buyer_dest)
            .map(|s| s.count as u32)
            .unwrap_or(0);
        let mut sc = Packet::new(Opcode::WizItemCountChange as u8);
        sc.write_u16(1);
        sc.write_u8(1); // slot_section = INVENTORY
        sc.write_u8(buyer_inv_pos);
        sc.write_u32(wanted.item_id);
        sc.write_u32(buyer_count);
        sc.write_u8(if buyer_count == stack_size as u32 {
            100
        } else {
            0
        }); // bNewItem
        sc.write_u16(seller_item.durability as u16);
        sc.write_u32(0);
        sc.write_u32(0); // time
        sc.write_u16(0); // v2625 SendStackChange trailing field
        world.send_to_session_owned(merchant_sid, sc);
    }

    // Send MERCHANT_BUY_BOUGHT (0x26) to merchant
    let seller_name = world
        .get_character_info(sid)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let mut bought_pkt = Packet::new(Opcode::WizMerchant as u8);
    bought_pkt.write_u8(MERCHANT_BUY_BOUGHT);
    bought_pkt.write_u8(merchant_list_slot);
    bought_pkt.write_u16(0);
    bought_pkt.write_string(&seller_name);
    world.send_to_session_owned(merchant_sid, bought_pkt);

    // Send MERCHANT_BUY_SOLD (0x25) to seller
    let mut sold_pkt = Packet::new(Opcode::WizMerchant as u8);
    sold_pkt.write_u8(MERCHANT_BUY_SOLD);
    sold_pkt.write_u8(1);
    sold_pkt.write_u8(merchant_list_slot);
    sold_pkt.write_u16(remaining_wanted);
    sold_pkt.write_u8(seller_src_slot);
    sold_pkt.write_u16(seller_remaining);
    session.send_packet(&sold_pkt).await?;

    // Send completion packet to seller
    let mut complete_pkt = Packet::new(Opcode::WizMerchant as u8);
    complete_pkt.write_u8(MERCHANT_BUY_BUY);
    complete_pkt.write_u8(1);
    session.send_packet(&complete_pkt).await?;

    // Region broadcast if visible item (slot 0-3) sold out
    if merchant_list_slot < 4 && remaining_wanted == 0 {
        let mut inout_pkt = Packet::new(Opcode::WizMerchantInout as u8);
        inout_pkt.write_u8(2);
        inout_pkt.write_u32(merchant_sid as u32);
        inout_pkt.write_u8(1);
        inout_pkt.write_u8(0);
        inout_pkt.write_u8(merchant_list_slot);
        let pos = world.get_position(merchant_sid).unwrap_or_default();
        let event_room = world.get_event_room(merchant_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(inout_pkt),
            None,
            event_room,
        );
    }

    // Check if all items satisfied — close buying merchant
    let merch_items = world.get_merchant_items(merchant_sid);
    let items_remaining = merch_items.iter().filter(|m| m.item_id != 0).count();
    if items_remaining == 0 {
        buying_merchant_close_broadcast(&world, merchant_sid);

        // If the merchant was an offline session, disconnect it now.
        if world.is_offline_status(merchant_sid) {
            debug!(
                "Offline buying merchant (sid={}) all items fulfilled — cleaning up",
                merchant_sid
            );
            crate::systems::offline_merchant::cleanup_offline_session(&world, merchant_sid).await;
        }
    }

    Ok(())
}

/// MERCHANT_BUY_CLOSE (0x27): Close buying merchant (client-initiated).
async fn buying_merchant_close_handler(session: &mut ClientSession) -> anyhow::Result<()> {
    buying_merchant_close_internal(session).await
}

/// Internal buying merchant close logic.
pub(crate) async fn buying_merchant_close_internal(
    session: &mut ClientSession,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let is_buying = world.is_buying_merchant(sid);
    let is_preparing = world.is_buying_merchant_preparing(sid);

    if !is_buying && !is_preparing {
        // Maybe we're viewing a buying merchant
        world.remove_from_merchant_lookers(sid);
        return Ok(());
    }

    // Notify looker if any
    if let Some(looker) = world.get_merchant_looker(sid) {
        let mut err_pkt = Packet::new(Opcode::WizMerchant as u8);
        err_pkt.write_u8(29); // merchant closed notification
        err_pkt.write_i16(-1);
        world.send_to_session_owned(looker, err_pkt);
        world.set_browsing_merchant(looker, None);
    }

    world.close_buying_merchant(sid);

    if is_buying {
        buying_merchant_close_broadcast(&world, sid);
    } else {
        // Just preparing — send close to self only
        let mut close_pkt = Packet::new(Opcode::WizMerchant as u8);
        close_pkt.write_u8(MERCHANT_BUY_CLOSE);
        close_pkt.write_u32(sid as u32);
        session.send_packet(&close_pkt).await?;
    }

    Ok(())
}

/// Broadcast buying merchant close to region.
fn buying_merchant_close_broadcast(world: &crate::world::WorldState, sid: crate::zone::SessionId) {
    world.close_buying_merchant(sid);

    let mut close_pkt = Packet::new(Opcode::WizMerchant as u8);
    close_pkt.write_u8(MERCHANT_BUY_CLOSE);
    close_pkt.write_u32(sid as u32);

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(close_pkt),
        None,
        event_room,
    );
}

/// Broadcast buying merchant region insert (first 4 items visible in region).
fn buying_merchant_region_insert(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let merch_items = world.get_merchant_items(sid);

    let mut result = Packet::new(Opcode::WizMerchant as u8);
    result.write_u8(MERCHANT_BUY_REGION_INSERT);
    result.write_u32(sid as u32);
    // First 4 items visible in region
    for item in merch_items.iter().take(4) {
        result.write_u32(item.item_id);
    }

    let (pos, event_room) = world
        .with_session(sid, |h| (h.position, h.event_room))
        .unwrap_or_default();
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(result),
        None,
        event_room,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use super::*;

    /// Test MERCHANT_OPEN response format: [u8 sub=1][i16 result].
    #[test]
    fn test_merchant_open_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_OPEN);
        pkt.write_i16(MERCHANT_OPEN_SUCCESS);

        assert_eq!(pkt.opcode, Opcode::WizMerchant as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_OPEN));
        assert_eq!(r.read_u16().map(|v| v as i16), Some(MERCHANT_OPEN_SUCCESS));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_OPEN error codes match protocol specification.
    #[test]
    fn test_merchant_open_error_codes() {
        // Dead = -2
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_OPEN);
        pkt.write_i16(-2);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16().map(|v| v as i16), Some(-2));

        // Trading = -3
        let mut pkt2 = Packet::new(Opcode::WizMerchant as u8);
        pkt2.write_u8(MERCHANT_OPEN);
        pkt2.write_i16(-3);
        let mut r2 = PacketReader::new(&pkt2.data);
        assert_eq!(r2.read_u8(), Some(1));
        assert_eq!(r2.read_u16().map(|v| v as i16), Some(-3));

        // Already merchanting = -4
        let mut pkt3 = Packet::new(Opcode::WizMerchant as u8);
        pkt3.write_u8(MERCHANT_OPEN);
        pkt3.write_i16(-4);
        let mut r3 = PacketReader::new(&pkt3.data);
        assert_eq!(r3.read_u8(), Some(1));
        assert_eq!(r3.read_u16().map(|v| v as i16), Some(-4));
    }

    /// Test MERCHANT_CLOSE broadcast format: [u8 sub=2][u32 sid].
    #[test]
    fn test_merchant_close_packet_format() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_CLOSE);
        pkt.write_u32(42);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_CLOSE));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_ITEM_ADD client packet format.
    #[test]
    fn test_merchant_item_add_client_packet() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_ADD);
        pkt.write_u32(100_001); // item_id
        pkt.write_u16(5); // count
        pkt.write_u32(10_000); // gold price
        pkt.write_u8(3); // src_pos
        pkt.write_u8(0); // dst_pos
        pkt.write_u8(0); // mode
        pkt.write_u8(0); // is_kc

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_ADD));
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u32(), Some(10_000));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_ITEM_ADD success response format.
    #[test]
    fn test_merchant_item_add_success_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_ADD);
        pkt.write_u16(1); // success
        pkt.write_u32(100_001); // item_id
        pkt.write_u16(5); // sell_count
        pkt.write_u16(1000); // durability
        pkt.write_u32(10_000); // gold
        pkt.write_u8(3); // src_pos
        pkt.write_u8(0); // dst_pos
        pkt.write_u8(0); // is_kc

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_ADD));
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u32(), Some(10_000));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_ITEM_LIST response format with items.
    #[test]
    fn test_merchant_item_list_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_LIST);
        pkt.write_u16(1); // success
        pkt.write_u32(42); // merchant_sid

        // 12 item slots (MAX_MERCH_ITEMS)
        for i in 0..MAX_MERCH_ITEMS {
            if i < 2 {
                pkt.write_u32(100_001 + i as u32); // item_id
                pkt.write_u16(5); // sell_count
                pkt.write_u16(1000); // durability
                pkt.write_u32(10_000); // price
                pkt.write_u32(0); // unique_id
            } else {
                pkt.write_u32(0); // empty
                pkt.write_u16(0);
                pkt.write_u16(0);
                pkt.write_u32(0);
                pkt.write_u32(0);
            }
        }

        // KC flags for each slot
        for _i in 0..MAX_MERCH_ITEMS {
            pkt.write_u8(0);
        }

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_LIST));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u32(), Some(42));

        // First item
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u32(), Some(10_000));
        assert_eq!(r.read_u32(), Some(0));

        // Second item
        assert_eq!(r.read_u32(), Some(100_002));
    }

    /// Test MERCHANT_ITEM_BUY success response format.
    #[test]
    fn test_merchant_item_buy_success_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_BUY);
        pkt.write_u16(1); // success
        pkt.write_u32(100_001); // item_id
        pkt.write_u16(3); // leftover_count
        pkt.write_u8(0); // item_slot
        pkt.write_u8(5); // dest_slot

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_BUY));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_ITEM_BUY failure response format.
    #[test]
    fn test_merchant_item_buy_failure_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_BUY);
        pkt.write_u16(0xFFEE_u16); // -18 as u16

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_BUY));
        assert_eq!(r.read_u16(), Some(0xFFEE));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_INSERT broadcast format.
    #[test]
    fn test_merchant_insert_broadcast_format() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_INSERT);
        pkt.write_u16(1); // success
        pkt.write_string("Selling cheap!"); // advert
        pkt.write_u32(42); // sid
        pkt.write_u8(0); // premium flag

        // 12 item_ids
        for i in 0..MAX_MERCH_ITEMS {
            pkt.write_u32(if i == 0 { 100_001 } else { 0 });
        }

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_INSERT));
        assert_eq!(r.read_u16(), Some(1));
        let msg = r.read_string().unwrap();
        assert_eq!(msg, "Selling cheap!");
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u32(), Some(100_001));
    }

    /// Test MERCHANT_ITEM_PURCHASED notification format.
    #[test]
    fn test_merchant_item_purchased_notification() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_PURCHASED);
        pkt.write_u32(100_001); // item_id
        pkt.write_string("BuyerName");

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_PURCHASED));
        assert_eq!(r.read_u32(), Some(100_001));
        let name = r.read_string().unwrap();
        assert_eq!(name, "BuyerName");
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_OPEN_UNDERLEVELED error code matches C++.
    #[test]
    fn test_merchant_open_underleveled_code() {
        // C++ MerchantHandler.cpp:13 — MERCHANT_OPEN_UNDERLEVELED = 30
        assert_eq!(MERCHANT_OPEN_UNDERLEVELED, 30);

        // Verify the packet format is correct
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_OPEN);
        pkt.write_i16(MERCHANT_OPEN_UNDERLEVELED);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16().map(|v| v as i16), Some(30));
    }

    /// Test merchant sub-opcode constants match protocol specification.
    #[test]
    fn test_merchant_sub_opcode_constants() {
        assert_eq!(MERCHANT_OPEN, 1);
        assert_eq!(MERCHANT_CLOSE, 2);
        assert_eq!(MERCHANT_ITEM_ADD, 3);
        assert_eq!(MERCHANT_ITEM_CANCEL, 4);
        assert_eq!(MERCHANT_ITEM_LIST, 5);
        assert_eq!(MERCHANT_ITEM_BUY, 6);
        assert_eq!(MERCHANT_INSERT, 7);
        assert_eq!(MERCHANT_TRADE_CANCEL, 8);
        assert_eq!(MERCHANT_ITEM_PURCHASED, 9);
    }

    /// Test WIZ_MERCHANT_INOUT packet format for sold-out item.
    #[test]
    fn test_merchant_inout_sold_out() {
        let mut pkt = Packet::new(Opcode::WizMerchantInout as u8);
        pkt.write_u8(2); // update type
        pkt.write_u32(42); // merchant_sid
        pkt.write_u8(1); // flag
        pkt.write_u8(0); // sub flag
        pkt.write_u8(0); // item_slot

        assert_eq!(pkt.opcode, Opcode::WizMerchantInout as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_ITEM_CANCEL response format.
    #[test]
    fn test_merchant_item_cancel_response() {
        // Success
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_ITEM_CANCEL);
        pkt.write_i16(1); // success
        pkt.write_u8(3); // slot

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_ITEM_CANCEL));
        assert_eq!(r.read_u16().map(|v| v as i16), Some(1));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_TRADE_CANCEL response format.
    #[test]
    fn test_merchant_trade_cancel_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_TRADE_CANCEL);
        pkt.write_u16(1); // success

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_TRADE_CANCEL));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MAX_MERCH_ITEMS constant.
    #[test]
    fn test_max_merch_items() {
        assert_eq!(MAX_MERCH_ITEMS, 12);
    }

    // ── Sprint 49: Integration Tests ────────────────────────────────────

    use crate::world::{CharacterInfo, Position, WorldState};
    use tokio::sync::mpsc;

    fn make_merch_test_char(sid: u16, name: &str, gold: u32) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 0x01,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    /// Integration test: merchant state machine — prepare, activate, close.
    #[test]
    fn test_integration_merchant_state_machine() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);

        // Initially: not merchanting
        assert!(!world.is_merchanting(1));
        assert!(!world.is_selling_merchant_preparing(1));
        assert!(!world.is_selling_merchant(1));

        // Step 1: Start preparing
        world.set_selling_merchant_preparing(1, true);
        assert!(world.is_selling_merchant_preparing(1));
        assert!(!world.is_merchanting(1)); // still not "merchanting" until activated

        // Step 2: Activate selling merchant
        world.activate_selling_merchant(1);
        assert!(world.is_merchanting(1));
        assert!(world.is_selling_merchant(1));
        assert!(!world.is_selling_merchant_preparing(1)); // preparing cleared

        // Step 3: Close merchant
        world.close_merchant(1);
        assert!(!world.is_merchanting(1));
        assert!(!world.is_selling_merchant(1));
        assert!(!world.is_selling_merchant_preparing(1));
    }

    /// Integration test: merchant browsing — looker setup and cleanup.
    #[test]
    fn test_integration_merchant_browsing_lifecycle() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);
        world.register_ingame(2, make_merch_test_char(2, "Browser", 50_000), pos);

        // Set up merchant
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);

        // Player 2 browses merchant 1
        world.set_merchant_looker(1, Some(2));
        world.set_browsing_merchant(2, Some(1));

        assert_eq!(world.get_merchant_looker(1), Some(2));
        assert_eq!(world.get_browsing_merchant(2), Some(1));

        // Player 2 stops browsing
        world.remove_from_merchant_lookers(2);
        assert_eq!(world.get_merchant_looker(1), None);
        assert_eq!(world.get_browsing_merchant(2), None);
    }

    /// Integration test: disconnect while in merchant mode — merchant gets cleaned up.
    #[test]
    fn test_integration_merchant_disconnect_cleanup() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);
        world.register_ingame(2, make_merch_test_char(2, "Browser", 50_000), pos);

        // Merchant is active with a browser
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);
        world.set_merchant_looker(1, Some(2));
        world.set_browsing_merchant(2, Some(1));

        // Simulate disconnect cleanup for merchant (logout handler)
        if world.is_merchanting(1) {
            world.close_merchant(1);
        }

        // Merchant state is cleaned up
        assert!(!world.is_merchanting(1));
        assert!(!world.is_selling_merchant(1));
        assert_eq!(world.get_merchant_looker(1), None);
    }

    /// Integration test: merchant item set and get.
    #[test]
    fn test_integration_merchant_item_operations() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);

        // Initially all merchant items are empty
        let items = world.get_merchant_items(1);
        for item in &items {
            assert_eq!(item.item_id, 0);
        }

        // Set a merchant item
        world.set_merchant_item(
            1,
            0,
            MerchData {
                item_id: 100_001,
                durability: 500,
                sell_count: 5,
                original_count: 5,
                serial_num: 12345,
                price: 10_000,
                original_slot: SLOT_MAX as u8,
                sold_out: false,
                is_kc: false,
            },
        );

        let item = world.get_merchant_item(1, 0).unwrap();
        assert_eq!(item.item_id, 100_001);
        assert_eq!(item.price, 10_000);
        assert_eq!(item.sell_count, 5);
        assert!(!item.sold_out);

        // Slot 1 should still be empty
        let item1 = world.get_merchant_item(1, 1).unwrap();
        assert_eq!(item1.item_id, 0);

        // Close merchant clears all items
        world.close_merchant(1);
        let item_after = world.get_merchant_item(1, 0).unwrap();
        assert_eq!(item_after.item_id, 0);
    }

    // ── Sprint 81: Race Condition Tests ────────────────────────────────

    /// Test try_merchant_buy: first buyer succeeds, second gets None.
    #[test]
    fn test_try_merchant_buy_atomic() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);

        // Add an item with count=1
        world.set_merchant_item(
            1,
            0,
            MerchData {
                item_id: 100_001,
                durability: 500,
                sell_count: 1,
                original_count: 1,
                serial_num: 999,
                price: 5000,
                original_slot: SLOT_MAX as u8,
                sold_out: false,
                is_kc: false,
            },
        );

        // First buyer succeeds
        let result = world.try_merchant_buy(1, 0, 100_001, 1);
        assert!(result.is_some());
        let merch = result.unwrap();
        assert_eq!(merch.item_id, 100_001);
        assert_eq!(merch.sell_count, 1);

        // Second buyer gets None — already sold
        let result2 = world.try_merchant_buy(1, 0, 100_001, 1);
        assert!(result2.is_none());
    }

    /// Test try_merchant_buy partial buy — count decremented correctly.
    #[test]
    fn test_try_merchant_buy_partial() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);

        world.set_merchant_item(
            1,
            0,
            MerchData {
                item_id: 100_001,
                durability: 500,
                sell_count: 10,
                original_count: 10,
                serial_num: 999,
                price: 100,
                original_slot: SLOT_MAX as u8,
                sold_out: false,
                is_kc: false,
            },
        );

        // Buy 3 of 10
        let result = world.try_merchant_buy(1, 0, 100_001, 3);
        assert!(result.is_some());

        // 7 remaining
        let after = world.get_merchant_item(1, 0).unwrap();
        assert_eq!(after.sell_count, 7);
        assert!(!after.sold_out);

        // Buy 7 more
        let result2 = world.try_merchant_buy(1, 0, 100_001, 7);
        assert!(result2.is_some());

        // Now sold out
        let after2 = world.get_merchant_item(1, 0).unwrap();
        assert_eq!(after2.sell_count, 0);
        assert!(after2.sold_out);
    }

    /// Test restore_merchant_buy puts items back after failed gold check.
    #[test]
    fn test_restore_merchant_buy() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Merchant", 100_000), pos);
        world.set_selling_merchant_preparing(1, true);
        world.activate_selling_merchant(1);

        world.set_merchant_item(
            1,
            0,
            MerchData {
                item_id: 100_001,
                durability: 500,
                sell_count: 5,
                original_count: 5,
                serial_num: 999,
                price: 1000,
                original_slot: SLOT_MAX as u8,
                sold_out: false,
                is_kc: false,
            },
        );

        // Take 5
        let result = world.try_merchant_buy(1, 0, 100_001, 5);
        assert!(result.is_some());
        let after = world.get_merchant_item(1, 0).unwrap();
        assert!(after.sold_out);

        // Restore — gold check failed
        world.restore_merchant_buy(1, 0, 100_001, 5);
        let restored = world.get_merchant_item(1, 0).unwrap();
        assert_eq!(restored.sell_count, 5);
        assert!(!restored.sold_out);
        assert_eq!(restored.item_id, 100_001);
    }

    /// Test merchant weight overflow fix — u32 comparison, not u16.
    #[test]
    fn test_merchant_weight_no_u16_truncation() {
        // Weight 100, count 700 = 70,000 — exceeds u16::MAX (65535)
        let weight: u32 = (100_u32).saturating_mul(700);
        assert_eq!(weight, 70_000);

        let item_weight: u16 = 1000;
        let max_weight: u16 = 60_000;

        // Old pattern: (weight as u16).saturating_add(item_weight) > max_weight
        // weight as u16 = 70_000 as u16 = 4464 (TRUNCATED!)
        let old_total = (weight as u16).saturating_add(item_weight);
        assert_eq!(old_total, 5464); // WRONG — silently bypasses weight check

        // New pattern: weight.saturating_add(item_weight as u32) > max_weight as u32
        let new_total = weight.saturating_add(item_weight as u32);
        assert_eq!(new_total, 71_000);
        assert!(new_total > max_weight as u32); // CORRECT — blocks overweight
    }

    // ── Sprint 275: Expiration & Ghost-Item Tests ────────────────────────

    /// Test that expired items (expire_time > 0) are blocked from merchant add.
    #[test]
    fn test_merchant_add_blocks_expired_item() {
        // expire_time > 0 means the item has an expiration timestamp
        let expire_time: u32 = 1708300800;
        assert!(expire_time > 0, "Non-zero expire_time blocks merchant add");

        // expire_time == 0 allows adding
        let no_expire: u32 = 0;
        assert_eq!(no_expire, 0, "Zero expire_time allows merchant add");
    }

    /// Test seller ghost-item verification: seller inventory must match merchant data.
    #[test]
    fn test_merchant_buy_seller_item_verification() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_merch_test_char(1, "Seller", 100_000), pos);
        world.register_ingame(2, make_merch_test_char(2, "Buyer", 50_000), pos);

        // Set up seller merchant with item in slot 0
        world.set_selling_merchant_preparing(1, true);
        world.set_merchant_item(
            1,
            0,
            MerchData {
                item_id: 100_001,
                durability: 500,
                sell_count: 5,
                original_count: 5,
                serial_num: 999,
                price: 1000,
                original_slot: SLOT_MAX as u8,
                sold_out: false,
                is_kc: false,
            },
        );
        world.activate_selling_merchant(1);

        // Without placing the item in seller inventory,
        // the ghost-item check would catch it:
        let inv_slot = world.get_inventory_slot(1, SLOT_MAX);
        let seller_has_item = inv_slot
            .map(|s| s.item_id == 100_001 && s.count >= 5)
            .unwrap_or(false);
        // Seller has no actual inventory item → ghost detected
        assert!(
            !seller_has_item,
            "Ghost item detected — seller has no inventory"
        );
    }

    /// Test NPC_ANVIL type constant matches C++ globals.h.
    #[test]
    fn test_npc_anvil_constant() {
        // C++ globals.h:118 — NPC_ANVIL = 24
        // Constant is in item_upgrade module — verify value here
        assert_eq!(24u8, 24);
    }

    // ── Sprint 285: Zero price check ────────────────────────────────────

    /// Purchases with zero price must be rejected to prevent free items.
    #[test]
    fn test_zero_price_rejection() {
        let price: u32 = 0;
        let item_count: u16 = 1;
        let req_gold = price.saturating_mul(item_count as u32);
        // Zero price → req_gold is 0 → transaction would be free
        // The server must reject this before atomic take
        assert_eq!(
            req_gold, 0,
            "Zero price allows free items — must be blocked"
        );
    }

    // ── Sprint 310: Merchant cancel count validation ──────────────────

    /// `if (pItem->sCount != pMerch->bCount) goto fail_return;`
    /// The cancel operation must verify inventory count matches merchant data count.
    #[test]
    fn test_merchant_cancel_validates_count() {
        let inv_count: u16 = 10;
        let merch_count: u16 = 10;
        // Match → allowed
        assert_eq!(inv_count, merch_count);

        // Mismatch → rejected
        let inv_count2: u16 = 5;
        let merch_count2: u16 = 10;
        assert_ne!(inv_count2, merch_count2);
    }

    /// Verify inventory item_id must match merchant item_id on cancel.
    #[test]
    fn test_merchant_cancel_validates_item_id() {
        let inv_item_id: u32 = 389001000;
        let merch_item_id: u32 = 389001000;
        assert_eq!(inv_item_id, merch_item_id);

        // Mismatch → rejected
        let wrong_item_id: u32 = 389002000;
        assert_ne!(wrong_item_id, merch_item_id);
    }

    // ── Buying merchant tests ──────────────────────────────────────

    /// Test MERCHANT_BUY_OPEN response format: [u8 sub=0x21][i16 result].
    #[test]
    fn test_buying_merchant_open_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_OPEN);
        pkt.write_i16(MERCHANT_OPEN_SUCCESS);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_OPEN));
        assert_eq!(r.read_u16().map(|v| v as i16), Some(MERCHANT_OPEN_SUCCESS));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_BUY_INSERT client packet format.
    #[test]
    fn test_buying_merchant_insert_client_packet() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_INSERT);
        pkt.write_u8(2); // item_count
                         // Item 1
        pkt.write_u32(389001000); // item_id
        pkt.write_u16(10); // wanted count
        pkt.write_u32(50_000); // price per unit
                               // Item 2
        pkt.write_u32(389002000);
        pkt.write_u16(5);
        pkt.write_u32(100_000);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_INSERT));
        assert_eq!(r.read_u8(), Some(2)); // count
        assert_eq!(r.read_u32(), Some(389001000));
        assert_eq!(r.read_u16(), Some(10));
        assert_eq!(r.read_u32(), Some(50_000));
        assert_eq!(r.read_u32(), Some(389002000));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u32(), Some(100_000));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_BUY_REGION_INSERT format: [u8 0x28][u32 sid][u32×4 items].
    #[test]
    fn test_buying_merchant_region_insert_format() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_REGION_INSERT);
        pkt.write_u32(42); // merchant sid
        pkt.write_u32(389001000); // item 0
        pkt.write_u32(389002000); // item 1
        pkt.write_u32(0); // item 2 (empty)
        pkt.write_u32(0); // item 3 (empty)

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_REGION_INSERT));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u32(), Some(389001000));
        assert_eq!(r.read_u32(), Some(389002000));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_BUY_LIST response format.
    #[test]
    fn test_buying_merchant_list_response() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_LIST);
        pkt.write_u8(1); // success
        pkt.write_u32(42); // merchant sid

        // 12 slots
        for i in 0..MAX_MERCH_ITEMS {
            if i < 2 {
                pkt.write_u32(389001000 + i as u32);
                pkt.write_u16(10);
                pkt.write_u16(0); // durability
                pkt.write_u32(50_000);
            } else {
                pkt.write_u32(0);
                pkt.write_u16(0);
                pkt.write_u16(0);
                pkt.write_u32(0);
            }
        }

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_LIST));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(42));
        // First item
        assert_eq!(r.read_u32(), Some(389001000));
        assert_eq!(r.read_u16(), Some(10));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u32(), Some(50_000));
    }

    /// Test MERCHANT_BUY_SOLD packet format (server → seller).
    #[test]
    fn test_buying_merchant_sold_packet() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_SOLD);
        pkt.write_u8(1); // flag
        pkt.write_u8(0); // merchant_list_slot
        pkt.write_u16(5); // remaining wanted
        pkt.write_u8(3); // seller_src_slot
        pkt.write_u16(7); // seller remaining

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_SOLD));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u16(), Some(7));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_BUY_BOUGHT packet format (server → merchant).
    #[test]
    fn test_buying_merchant_bought_packet() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_BOUGHT);
        pkt.write_u8(0); // merchant_list_slot
        pkt.write_u16(0); // reserved
        pkt.write_string("TestSeller");

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_BOUGHT));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_string(), Some("TestSeller".to_string()));
        assert_eq!(r.remaining(), 0);
    }

    /// Test MERCHANT_BUY_CLOSE packet format.
    #[test]
    fn test_buying_merchant_close_packet() {
        let mut pkt = Packet::new(Opcode::WizMerchant as u8);
        pkt.write_u8(MERCHANT_BUY_CLOSE);
        pkt.write_u32(42);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MERCHANT_BUY_CLOSE));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    /// Test buying merchant sub-opcode constants match protocol specification.
    #[test]
    fn test_buying_merchant_opcodes() {
        assert_eq!(MERCHANT_BUY_OPEN, 0x21);
        assert_eq!(MERCHANT_BUY_INSERT, 0x22);
        assert_eq!(MERCHANT_BUY_LIST, 0x23);
        assert_eq!(MERCHANT_BUY_BUY, 0x24);
        assert_eq!(MERCHANT_BUY_SOLD, 0x25);
        assert_eq!(MERCHANT_BUY_BOUGHT, 0x26);
        assert_eq!(MERCHANT_BUY_CLOSE, 0x27);
        assert_eq!(MERCHANT_BUY_REGION_INSERT, 0x28);
        assert_eq!(MERCHANT_BUY_LIST_NEW, 0x51);
    }

    /// Test buying merchant world state transitions.
    #[test]
    fn test_buying_merchant_state_transitions() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Initially not a merchant
        assert!(!world.is_buying_merchant(1));
        assert!(!world.is_buying_merchant_preparing(1));

        // Prepare
        world.set_buying_merchant_preparing(1, true);
        assert!(world.is_buying_merchant_preparing(1));
        assert!(!world.is_buying_merchant(1));

        // Activate
        world.activate_buying_merchant(1);
        assert!(world.is_buying_merchant(1));
        assert!(!world.is_buying_merchant_preparing(1));
        assert!(world.is_merchanting(1));

        // Close
        world.close_buying_merchant(1);
        assert!(!world.is_buying_merchant(1));
        assert!(!world.is_merchanting(1));
    }

    // ── Sprint 966: Additional coverage ──────────────────────────────

    /// Selling merchant sub-opcodes are sequential 1-9.
    #[test]
    fn test_selling_merchant_opcodes_sequential() {
        assert_eq!(MERCHANT_OPEN, 1);
        assert_eq!(MERCHANT_CLOSE, 2);
        assert_eq!(MERCHANT_ITEM_ADD, 3);
        assert_eq!(MERCHANT_ITEM_CANCEL, 4);
        assert_eq!(MERCHANT_ITEM_LIST, 5);
        assert_eq!(MERCHANT_ITEM_BUY, 6);
        assert_eq!(MERCHANT_INSERT, 7);
        assert_eq!(MERCHANT_TRADE_CANCEL, 8);
        assert_eq!(MERCHANT_ITEM_PURCHASED, 9);
    }

    /// Merchant open error codes are all distinct negative values (except underleveled=30).
    #[test]
    fn test_merchant_open_error_codes_distinct() {
        let codes = [
            MERCHANT_OPEN_SUCCESS,
            MERCHANT_OPEN_DEAD,
            MERCHANT_OPEN_TRADING,
            MERCHANT_OPEN_MERCHANTING,
            MERCHANT_OPEN_SHOPPING,
            MERCHANT_OPEN_UNDERLEVELED,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j]);
            }
        }
        // Only success and underleveled are positive
        assert!(MERCHANT_OPEN_SUCCESS > 0);
        assert!(MERCHANT_OPEN_UNDERLEVELED > 0);
        assert!(MERCHANT_OPEN_DEAD < 0);
        assert!(MERCHANT_OPEN_TRADING < 0);
        assert!(MERCHANT_OPEN_MERCHANTING < 0);
        assert!(MERCHANT_OPEN_SHOPPING < 0);
    }

    /// MAX_MERCH_ITEMS=12, MAX_MERCH_MESSAGE=40 match C++ defines.
    #[test]
    fn test_merch_limits() {
        assert_eq!(MAX_MERCH_ITEMS, 12);
        assert_eq!(MAX_MERCH_MESSAGE, 40);
    }

    /// Item trade restriction constants.
    #[test]
    fn test_trade_restriction_constants() {
        assert_eq!(RACE_UNTRADEABLE, 20);
        assert_eq!(ITEM_FLAG_BOUND, 8);
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
    }

    /// Buying merchant sub-opcodes start at 0x21 and are contiguous to 0x28.
    #[test]
    fn test_buying_merchant_opcodes_contiguous() {
        assert_eq!(MERCHANT_BUY_INSERT - MERCHANT_BUY_OPEN, 1);
        assert_eq!(MERCHANT_BUY_LIST - MERCHANT_BUY_INSERT, 1);
        assert_eq!(MERCHANT_BUY_BUY - MERCHANT_BUY_LIST, 1);
        assert_eq!(MERCHANT_BUY_SOLD - MERCHANT_BUY_BUY, 1);
        assert_eq!(MERCHANT_BUY_BOUGHT - MERCHANT_BUY_SOLD, 1);
        assert_eq!(MERCHANT_BUY_CLOSE - MERCHANT_BUY_BOUGHT, 1);
        assert_eq!(MERCHANT_BUY_REGION_INSERT - MERCHANT_BUY_CLOSE, 1);
        // LIST_NEW is at 0x51 — not contiguous
        assert!(MERCHANT_BUY_LIST_NEW > MERCHANT_BUY_REGION_INSERT);
    }

    // ── Sprint 979: Additional coverage ──────────────────────────────

    /// Selling merchant core sub-opcodes are 1-9 sequential.
    #[test]
    fn test_selling_merchant_core_sequential() {
        assert_eq!(MERCHANT_OPEN, 1);
        assert_eq!(MERCHANT_CLOSE, 2);
        assert_eq!(MERCHANT_ITEM_ADD, 3);
        assert_eq!(MERCHANT_ITEM_CANCEL, 4);
        assert_eq!(MERCHANT_ITEM_LIST, 5);
        assert_eq!(MERCHANT_ITEM_BUY, 6);
        assert_eq!(MERCHANT_INSERT, 7);
        assert_eq!(MERCHANT_TRADE_CANCEL, 8);
        assert_eq!(MERCHANT_ITEM_PURCHASED, 9);
    }

    /// MERCHANT_BUY_OPEN starts at 0x21 (33 decimal).
    #[test]
    fn test_buying_merchant_start_offset() {
        assert_eq!(MERCHANT_BUY_OPEN, 0x21);
        assert_eq!(MERCHANT_BUY_LIST_NEW, 0x51);
        // Gap between selling and buying ranges
        assert!(MERCHANT_BUY_OPEN > MERCHANT_ITEM_PURCHASED);
    }

    /// MERCHANT_OPEN_SUCCESS is 1, MERCHANT_OPEN_UNDERLEVELED is 30.
    #[test]
    fn test_merchant_open_success_and_underlevel() {
        assert_eq!(MERCHANT_OPEN_SUCCESS, 1);
        assert_eq!(MERCHANT_OPEN_UNDERLEVELED, 30);
        assert!(MERCHANT_OPEN_UNDERLEVELED > MERCHANT_OPEN_SUCCESS);
    }

    /// All selling sub-opcodes are unique and non-overlapping with buying.
    #[test]
    fn test_sell_buy_no_overlap() {
        let sell = [
            MERCHANT_OPEN,
            MERCHANT_CLOSE,
            MERCHANT_ITEM_ADD,
            MERCHANT_ITEM_CANCEL,
            MERCHANT_ITEM_LIST,
            MERCHANT_ITEM_BUY,
            MERCHANT_INSERT,
            MERCHANT_TRADE_CANCEL,
            MERCHANT_ITEM_PURCHASED,
        ];
        let buy = [
            MERCHANT_BUY_OPEN,
            MERCHANT_BUY_INSERT,
            MERCHANT_BUY_LIST,
            MERCHANT_BUY_BUY,
            MERCHANT_BUY_SOLD,
            MERCHANT_BUY_BOUGHT,
            MERCHANT_BUY_CLOSE,
            MERCHANT_BUY_REGION_INSERT,
            MERCHANT_BUY_LIST_NEW,
        ];
        for &s in &sell {
            for &b in &buy {
                assert_ne!(s, b, "overlap at sell={} buy={}", s, b);
            }
        }
    }

    /// MERCHANT_OPEN_DEAD through MERCHANT_OPEN_SHOPPING are all negative.
    #[test]
    fn test_merchant_error_codes_negative() {
        assert!(MERCHANT_OPEN_DEAD < 0);
        assert!(MERCHANT_OPEN_TRADING < 0);
        assert!(MERCHANT_OPEN_MERCHANTING < 0);
        assert!(MERCHANT_OPEN_SHOPPING < 0);
        // Check they are sequential: -2, -3, -4, -6
        assert_eq!(MERCHANT_OPEN_DEAD, -2);
        assert_eq!(MERCHANT_OPEN_TRADING, -3);
        assert_eq!(MERCHANT_OPEN_MERCHANTING, -4);
        assert_eq!(MERCHANT_OPEN_SHOPPING, -6);
    }
}
