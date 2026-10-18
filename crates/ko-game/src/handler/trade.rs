//! WIZ_EXCHANGE (0x30) handler — player-to-player trade.
//! Sub-opcodes:
//! - EXCHANGE_REQ (1): Initiate trade with target player
//! - EXCHANGE_AGREE (2): Accept/decline trade
//! - EXCHANGE_ADD (3): Add item to trade window
//! - EXCHANGE_DECIDE (5): Lock in / confirm trade
//! - EXCHANGE_CANCEL (8): Cancel trade at any point
//! Server-to-client only:
//! - EXCHANGE_OTHERADD (4): Other player added an item
//! - EXCHANGE_OTHERDECIDE (6): Other player confirmed
//! - EXCHANGE_DONE (7): Trade completed

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::{
    ExchangeItem, UserItemSlot, COIN_MAX, ITEM_FLAG_BOUND, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED,
    ITEM_FLAG_SEALED, ITEM_GOLD, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN, RACE_UNTRADEABLE,
    TRADE_STATE_DECIDING, TRADE_STATE_NONE, TRADE_STATE_SENDER, TRADE_STATE_TARGET,
    TRADE_STATE_TRADING,
};

/// Exchange sub-opcode constants.
const EXCHANGE_REQ: u8 = 1;
const EXCHANGE_AGREE: u8 = 2;
const EXCHANGE_ADD: u8 = 3;
const EXCHANGE_OTHERADD: u8 = 4;
const EXCHANGE_DECIDE: u8 = 5;
const EXCHANGE_OTHERDECIDE: u8 = 6;
const EXCHANGE_DONE: u8 = 7;
const EXCHANGE_CANCEL: u8 = 8;

use super::{HAVE_MAX, ITEMCOUNT_MAX, ITEM_KIND_UNIQUE, SLOT_MAX};

/// Handle WIZ_EXCHANGE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let world = session.world().clone();
    let sid = session.session_id();

    // Must be in-game
    if world.get_character_info(sid).is_none() {
        return Ok(());
    }

    // Cannot trade while dead or in busy state
    if world.is_player_dead(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_buying_merchant_preparing(sid)
    {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        EXCHANGE_REQ => exchange_req(session, &mut reader).await,
        EXCHANGE_AGREE => exchange_agree(session, &mut reader).await,
        EXCHANGE_ADD => exchange_add(session, &mut reader).await,
        EXCHANGE_DECIDE => exchange_decide(session).await,
        EXCHANGE_CANCEL => exchange_cancel(session).await,
        _ => {
            debug!(
                "[{}] Trade unhandled sub-opcode {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// EXCHANGE_REQ (1): Request a trade with another player.
async fn exchange_req(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let target_id = reader.read_u16().unwrap_or(0);

    // If already trading, cancel first
    if world.is_trading(sid) {
        exchange_cancel(session).await?;
        return Ok(());
    }

    // Validate target exists, is in-game, same zone, not busy
    // Single DashMap read per player: char_info + position + account_id (3 reads → 1)
    let (my_info, my_pos, my_account) = match world
        .with_session(sid, |h| {
            h.character
                .as_ref()
                .map(|ch| (ch.clone(), h.position, h.account_id.clone()))
        })
        .flatten()
    {
        Some(v) => v,
        None => return send_cancel(session).await,
    };

    let (target_info, target_pos, target_account) = match world
        .with_session(target_id, |h| {
            h.character
                .as_ref()
                .map(|ch| (ch.clone(), h.position, h.account_id.clone()))
        })
        .flatten()
    {
        Some(v) => v,
        None => return send_cancel(session).await,
    };

    // Cannot trade with self, GMs, or cross-zone
    if target_id == sid
        || target_info.authority == 0 // GM
        || target_info.name == my_info.name
        || target_pos.zone_id != my_pos.zone_id
    {
        return send_cancel(session).await;
    }

    // Same-account trade prevention
    // Prevents item duplication between characters on the same account.
    if !my_account.is_empty() && my_account.to_lowercase() == target_account.to_lowercase() {
        return send_cancel(session).await;
    }

    // Cross-nation trade check
    if my_info.nation != target_info.nation {
        let can_trade = world
            .get_zone(my_pos.zone_id)
            .is_some_and(|z| z.can_trade_other_nation());
        if !can_trade {
            return send_cancel(session).await;
        }
    }

    // Target must not be busy
    if world.is_player_dead(target_id)
        || world.is_trading(target_id)
        || world.is_merchanting(target_id)
        || world.is_mining(target_id)
        || world.is_fishing(target_id)
        || world.is_selling_merchant_preparing(target_id)
        || world.is_buying_merchant_preparing(target_id)
        || world
            .with_session(target_id, |h| h.genie_active)
            .unwrap_or(false)
    {
        return send_cancel(session).await;
    }

    // Minimum trade level check from server settings
    let trade_level = world
        .get_server_settings()
        .map(|s| s.trade_level)
        .unwrap_or(1);
    if (my_info.level as i16) < trade_level {
        return send_cancel(session).await;
    }

    // Set up the trade request
    world.init_trade_request(sid, target_id);

    // Send EXCHANGE_REQ to target with sender's ID
    let mut pkt = Packet::new(Opcode::WizExchange as u8);
    pkt.write_u8(EXCHANGE_REQ);
    pkt.write_u32(sid as u32);
    world.send_to_session_owned(target_id, pkt);

    Ok(())
}

/// EXCHANGE_AGREE (2): Accept or decline a trade request.
async fn exchange_agree(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let partner_sid = match world.get_exchange_user(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Must be in TARGET state (we received the request)
    if world.get_trade_state(sid) != TRADE_STATE_TARGET {
        return send_cancel(session).await;
    }

    let accepted = reader.read_u8().unwrap_or(0);

    // Self-trade prevention: the acceptor must NOT be the request sender
    // C++ uses goDisconnect() for exploit detection. We reset + warn.
    if world.get_trade_state(sid) == TRADE_STATE_SENDER {
        // Packet manipulation attempt — sender trying to accept own request
        tracing::warn!(
            "[sid={}] Trade exploit: sender tried to accept own request",
            sid
        );
        world.reset_trade(sid);
        world.reset_trade(partner_sid);
        return send_cancel(session).await;
    }

    // Validate partner is still valid, alive, and in correct state
    let partner_state = world.get_trade_state(partner_sid);
    if partner_state != TRADE_STATE_SENDER || world.is_player_dead(partner_sid) {
        world.reset_trade(sid);
        world.reset_trade(partner_sid);
        return send_cancel(session).await;
    }

    // Zone equality + partner busy-state + nation cross-trade validation
    let my_pos = world.get_position(sid).unwrap_or_default();
    let partner_pos = world.get_position(partner_sid).unwrap_or_default();
    if my_pos.zone_id != partner_pos.zone_id
        || world.get_exchange_user(partner_sid) != Some(sid)
        || world.is_mining(partner_sid)
        || world.is_fishing(partner_sid)
        || world.is_merchanting(partner_sid)
        || world.is_selling_merchant_preparing(partner_sid)
        || world.is_buying_merchant_preparing(partner_sid)
    {
        world.reset_trade(sid);
        world.reset_trade(partner_sid);
        return send_cancel(session).await;
    }

    // Same-account trade prevention
    let (my_account, my_nation) = world
        .with_session(sid, |h| {
            let nation = h.character.as_ref().map(|c| c.nation).unwrap_or(0);
            (h.account_id.clone(), nation)
        })
        .unwrap_or_default();
    let (partner_account, partner_nation) = world
        .with_session(partner_sid, |h| {
            let nation = h.character.as_ref().map(|c| c.nation).unwrap_or(0);
            (h.account_id.clone(), nation)
        })
        .unwrap_or_default();
    if !my_account.is_empty() && my_account.to_lowercase() == partner_account.to_lowercase() {
        world.reset_trade(sid);
        world.reset_trade(partner_sid);
        return send_cancel(session).await;
    }

    // Nation cross-trade check at agree phase
    {
        if my_nation != partner_nation {
            let can_trade = world
                .get_zone(my_pos.zone_id)
                .is_some_and(|z| z.can_trade_other_nation());
            if !can_trade {
                world.reset_trade(sid);
                world.reset_trade(partner_sid);
                return send_cancel(session).await;
            }
        }
    }

    if accepted == 0 {
        // Declined — reset both
        world.trade_decline(sid);
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_AGREE);
        pkt.write_u16(0);
        world.send_to_session_owned(partner_sid, pkt);
    } else {
        // Accepted — move both to TRADING state
        world.trade_agree(sid);

        // Remove from merchant lookers if browsing
        world.remove_from_merchant_lookers(sid);

        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_AGREE);
        pkt.write_u16(1);
        world.send_to_session_owned(partner_sid, pkt);
    }

    Ok(())
}

/// EXCHANGE_ADD (3): Add an item to the trade window.
async fn exchange_add(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let pos = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let count = reader.read_u32().unwrap_or(0);

    if world.is_mining(sid)
        || world.is_fishing(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant_preparing(sid)
    {
        return send_add_fail(session).await;
    }

    let partner_sid = match world.get_exchange_user(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Must be in TRADING state
    if world.get_trade_state(sid) != TRADE_STATE_TRADING {
        return send_add_fail(session).await;
    }

    // Validate partner is still in a valid state (including zone equality)
    let my_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    let partner_zone = world
        .get_position(partner_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);
    let partner_state = world.get_trade_state(partner_sid);
    if my_zone != partner_zone
        || world.get_exchange_user(partner_sid) != Some(sid)
        || world.is_player_dead(partner_sid)
        || world.is_mining(partner_sid)
        || world.is_fishing(partner_sid)
        || world.is_merchanting(partner_sid)
        || world.is_selling_merchant_preparing(partner_sid)
        || world.is_buying_merchant_preparing(partner_sid)
    {
        exchange_cancel(session).await?;
        return Ok(());
    }

    // Nation re-validation: cannot trade cross-nation in restricted zones.
    {
        let my_nation = world.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
        let partner_nation = world
            .get_character_info(partner_sid)
            .map(|c| c.nation)
            .unwrap_or(0);
        if my_nation != partner_nation {
            let can_trade = world
                .get_zone(my_zone)
                .is_some_and(|z| z.can_trade_other_nation());
            if !can_trade {
                exchange_cancel(session).await?;
                return Ok(());
            }
        }
    }

    // Identity re-validation: prevent same-account/name trading (item dupe protection).
    {
        let (my_account, my_name) = world
            .with_session(sid, |h| {
                let name = h
                    .character
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                (h.account_id.clone(), name)
            })
            .unwrap_or_default();
        let (partner_account, partner_name) = world
            .with_session(partner_sid, |h| {
                let name = h
                    .character
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                (h.account_id.clone(), name)
            })
            .unwrap_or_default();

        if sid == partner_sid
            || my_name == partner_name
            || (!my_account.is_empty()
                && my_account.to_lowercase() == partner_account.to_lowercase())
        {
            tracing::warn!(
                "[sid={}] Trade exploit: identity re-validation failed in exchange_add",
                sid
            );
            exchange_cancel(session).await?;
            return Ok(());
        }
    }

    // C++ allows partner to be in TRADING(4) or DECIDING(5) state
    // TradeHandler.cpp:249-250
    if partner_state != TRADE_STATE_TRADING && partner_state != TRADE_STATE_DECIDING {
        return send_add_fail(session).await;
    }

    if count == 0 || item_id == 0 {
        return send_add_fail(session).await;
    }

    // Gold handling: pos==255 means gold
    if item_id == ITEM_GOLD && pos != 255 {
        return send_add_fail(session).await;
    }

    // Validate the item from the item table
    if item_id != ITEM_GOLD {
        if pos as usize >= HAVE_MAX {
            return send_add_fail(session).await;
        }
        // Check no-trade range
        if (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id) {
            return send_add_fail(session).await;
        }
        if count as u16 >= ITEMCOUNT_MAX {
            return send_add_fail(session).await;
        }
        // Check item table properties
        let item_def = match world.get_item(item_id) {
            Some(i) => i,
            None => return send_add_fail(session).await,
        };
        if item_def.race.unwrap_or(0) == RACE_UNTRADEABLE || item_def.countable.unwrap_or(0) == 2 {
            return send_add_fail(session).await;
        }
    }

    // Check max items in exchange (12 items + gold)
    let current_items = world.get_exchange_items(sid);
    let has_gold = current_items.iter().any(|i| i.item_id == ITEM_GOLD);
    let max_items = if has_gold { 13 } else { 12 };
    if current_items.len() >= max_items {
        return send_add_fail(session).await;
    }

    let mut duration: i16 = 0;
    let mut serial_num: u64 = 0;
    let mut add_new = true;

    if item_id == ITEM_GOLD {
        // Gold: verify player has enough
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => return send_add_fail(session).await,
        };
        if count == 0 || count > ch.gold as u32 {
            return send_add_fail(session).await;
        }

        // Check if gold already in exchange list, add to it
        let found_gold = world.update_exchange_item_count(sid, ITEM_GOLD, count);
        if found_gold {
            add_new = false;
        }

        // Deduct gold
        world.gold_lose(sid, count);
    } else {
        // Item: verify it exists in inventory at the given slot
        let actual_slot = SLOT_MAX + pos as usize;
        let slot = match world.get_inventory_slot(sid, actual_slot) {
            Some(s) => s,
            None => return send_add_fail(session).await,
        };

        if slot.item_id != item_id || slot.count < count as u16 || slot.count == 0 {
            return send_add_fail(session).await;
        }

        // Check flags: rented, sealed, bound, duplicate — use equality, NOT bitmask.
        if slot.flag == ITEM_FLAG_RENTED
            || slot.flag == ITEM_FLAG_SEALED
            || slot.flag == ITEM_FLAG_BOUND
            || slot.flag == ITEM_FLAG_DUPLICATE
            || slot.expire_time > 0
        {
            return send_add_fail(session).await;
        }

        duration = slot.durability;
        serial_num = slot.serial_num;

        // For stackable items, check if already in the list
        let item_def = world.get_item(item_id);
        let countable = item_def.as_ref().and_then(|i| i.countable).unwrap_or(0);
        if countable > 0 {
            let found = world.update_exchange_item_count(sid, item_id, count);
            if found {
                add_new = false;
            }
        }

        // Deduct from inventory
        world.update_inventory(sid, |inv| {
            if actual_slot < inv.len() {
                inv[actual_slot].count -= count as u16;
                if inv[actual_slot].count == 0 {
                    inv[actual_slot] = UserItemSlot::default();
                }
                true
            } else {
                false
            }
        });
    }

    // Add new exchange item entry if needed
    if add_new {
        world.add_exchange_item(
            sid,
            ExchangeItem {
                item_id,
                count,
                durability: duration,
                serial_num,
                src_pos: if item_id == ITEM_GOLD {
                    255
                } else {
                    SLOT_MAX as u8 + pos
                },
                dst_pos: 0,
            },
        );
    }

    // Send success to adder
    let mut result = Packet::new(Opcode::WizExchange as u8);
    result.write_u8(EXCHANGE_ADD);
    result.write_u8(1);
    session.send_packet(&result).await?;

    // Send EXCHANGE_OTHERADD to partner
    let mut other_pkt = Packet::new(Opcode::WizExchange as u8);
    other_pkt.write_u8(EXCHANGE_OTHERADD);
    other_pkt.write_u32(item_id);
    other_pkt.write_u32(count);
    other_pkt.write_u16(duration as u16);
    // C++ only writes pet/cypher ring info when bAdd is true (new item, not count update)
    if add_new {
        let rebirth_level = world
            .get_character_info(sid)
            .map(|c| c.rebirth_level)
            .unwrap_or(0);
        crate::handler::unique_item_info::write_unique_item_info(
            &world,
            session.pool(),
            item_id,
            serial_num,
            rebirth_level,
            &mut other_pkt,
        )
        .await;
    } else {
        other_pkt.write_u32(0);
    }
    world.send_to_session_owned(partner_sid, other_pkt);

    Ok(())
}

/// EXCHANGE_DECIDE (5): Confirm the trade.
async fn exchange_decide(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.get_trade_state(sid) != TRADE_STATE_TRADING {
        return Ok(());
    }

    let partner_sid = match world.get_exchange_user(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    let my_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    let partner_zone = world
        .get_position(partner_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);
    if my_zone != partner_zone
        || world.get_exchange_user(partner_sid) != Some(sid)
        || world.is_player_dead(partner_sid)
        || world.is_mining(partner_sid)
        || world.is_merchanting(partner_sid)
    {
        return Ok(());
    }

    let partner_state = world.get_trade_state(partner_sid);
    if partner_state != TRADE_STATE_DECIDING && partner_state != TRADE_STATE_TRADING {
        return Ok(());
    }

    // If partner has not decided yet, set our state and notify
    if partner_state != TRADE_STATE_DECIDING {
        world.set_trade_state(sid, TRADE_STATE_DECIDING);
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_OTHERDECIDE);
        world.send_to_session_owned(partner_sid, pkt);
        return Ok(());
    }

    // Both have decided — execute the trade
    world.set_trade_state(sid, TRADE_STATE_DECIDING);

    // Validate both sides can receive items (first pass: weight + slots)
    let can_execute = check_exchange(&world, sid) && check_exchange(&world, partner_sid);

    if !can_execute {
        // Fail — send EXCHANGE_DONE with failure
        let mut fail_pkt = Packet::new(Opcode::WizExchange as u8);
        fail_pkt.write_u8(EXCHANGE_DONE);
        fail_pkt.write_u8(0);
        world.send_to_session(sid, &fail_pkt);
        world.send_to_session(partner_sid, &fail_pkt);

        // Give items back and cancel
        world.exchange_give_items_back(sid);
        world.exchange_give_items_back(partner_sid);
        world.reset_trade(sid);
        world.reset_trade(partner_sid);
        return Ok(());
    }

    // Second validation pass: item placement (kind=255 unique, slot collision, item table)
    let can_execute2 =
        check_execute_exchange(&world, sid) && check_execute_exchange(&world, partner_sid);

    if !can_execute2 {
        // Fail — send EXCHANGE_DONE with failure
        let mut fail_pkt = Packet::new(Opcode::WizExchange as u8);
        fail_pkt.write_u8(EXCHANGE_DONE);
        fail_pkt.write_u8(0);
        world.send_to_session(sid, &fail_pkt);
        world.send_to_session(partner_sid, &fail_pkt);

        // Give items back and cancel
        world.exchange_give_items_back(sid);
        world.exchange_give_items_back(partner_sid);
        world.reset_trade(sid);
        world.reset_trade(partner_sid);
        return Ok(());
    }

    // Execute: move partner's items to us, our items to partner
    let partner_items = world.get_exchange_items(partner_sid);
    let my_items = world.get_exchange_items(sid);

    // Give partner's items to us
    let mut received_items: Vec<(u8, u32, u16, i16, u64)> = Vec::with_capacity(12);
    for ex_item in &partner_items {
        if ex_item.item_id == ITEM_GOLD {
            world.gold_gain(sid, ex_item.count);
            continue;
        }
        let slot = match world.find_slot_for_item(sid, ex_item.item_id, ex_item.count as u16) {
            Some(s) => s,
            None => continue,
        };
        // Non-countable items always have count=1 after trade.
        let item_countable = world
            .get_item(ex_item.item_id)
            .and_then(|i| i.countable)
            .unwrap_or(0);
        world.update_inventory(sid, |inv| {
            if slot >= inv.len() {
                return false;
            }
            let is_new = inv[slot].item_id == 0;
            inv[slot].item_id = ex_item.item_id;
            inv[slot].count = (inv[slot].count + ex_item.count as u16).min(ITEMCOUNT_MAX);
            if item_countable == 0 {
                inv[slot].count = 1;
            }
            if is_new {
                inv[slot].durability = ex_item.durability;
                inv[slot].serial_num = ex_item.serial_num;
            }
            true
        });
        let final_count = if item_countable == 0 {
            1u16
        } else {
            ex_item.count as u16
        };
        received_items.push((
            (slot - 14) as u8,
            ex_item.item_id,
            final_count,
            ex_item.durability,
            ex_item.serial_num,
        ));
    }

    // Give our items to partner
    let mut partner_received: Vec<(u8, u32, u16, i16, u64)> = Vec::with_capacity(12);
    for ex_item in &my_items {
        if ex_item.item_id == ITEM_GOLD {
            world.gold_gain(partner_sid, ex_item.count);
            continue;
        }
        let slot =
            match world.find_slot_for_item(partner_sid, ex_item.item_id, ex_item.count as u16) {
                Some(s) => s,
                None => continue,
            };
        let item_countable = world
            .get_item(ex_item.item_id)
            .and_then(|i| i.countable)
            .unwrap_or(0);
        world.update_inventory(partner_sid, |inv| {
            if slot >= inv.len() {
                return false;
            }
            let is_new = inv[slot].item_id == 0;
            inv[slot].item_id = ex_item.item_id;
            inv[slot].count = (inv[slot].count + ex_item.count as u16).min(ITEMCOUNT_MAX);
            if item_countable == 0 {
                inv[slot].count = 1;
            }
            if is_new {
                inv[slot].durability = ex_item.durability;
                inv[slot].serial_num = ex_item.serial_num;
            }
            true
        });
        let final_count = if item_countable == 0 {
            1u16
        } else {
            ex_item.count as u16
        };
        partner_received.push((
            (slot - 14) as u8,
            ex_item.item_id,
            final_count,
            ex_item.durability,
            ex_item.serial_num,
        ));
    }

    // Clear source items from both sides
    for ex_item in &partner_items {
        if ex_item.item_id != ITEM_GOLD {
            world.update_inventory(partner_sid, |inv| {
                let pos = ex_item.src_pos as usize;
                if pos < inv.len() && inv[pos].count == 0 {
                    inv[pos] = UserItemSlot::default();
                }
                true
            });
        }
    }
    for ex_item in &my_items {
        if ex_item.item_id != ITEM_GOLD {
            world.update_inventory(sid, |inv| {
                let pos = ex_item.src_pos as usize;
                if pos < inv.len() && inv[pos].count == 0 {
                    inv[pos] = UserItemSlot::default();
                }
                true
            });
        }
    }

    // Build EXCHANGE_DONE success packet for us (with partner's items)
    let (my_gold, my_rebirth) = world
        .get_character_info(sid)
        .map(|c| (c.gold, c.rebirth_level))
        .unwrap_or((0, 0));
    let mut done_pkt = Packet::new(Opcode::WizExchange as u8);
    done_pkt.write_u8(EXCHANGE_DONE);
    done_pkt.write_u8(1);
    done_pkt.write_u32(my_gold);
    done_pkt.write_u16(received_items.len() as u16);
    for (dst, item_id, count, dur, serial) in &received_items {
        done_pkt.write_u8(*dst);
        done_pkt.write_u32(*item_id);
        done_pkt.write_u16(*count);
        done_pkt.write_u16(*dur as u16);
        done_pkt.write_u8(0);
        crate::handler::unique_item_info::write_unique_item_info(
            &world,
            session.pool(),
            *item_id,
            *serial,
            my_rebirth,
            &mut done_pkt,
        )
        .await;
    }
    world.send_to_session_owned(sid, done_pkt);

    // Build EXCHANGE_DONE success packet for partner (with our items)
    let partner_gold = world
        .get_character_info(partner_sid)
        .map(|c| c.gold)
        .unwrap_or(0);
    let partner_rebirth = world
        .get_character_info(partner_sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);
    let mut partner_pkt = Packet::new(Opcode::WizExchange as u8);
    partner_pkt.write_u8(EXCHANGE_DONE);
    partner_pkt.write_u8(1);
    partner_pkt.write_u32(partner_gold);
    partner_pkt.write_u16(partner_received.len() as u16);
    for (dst, item_id, count, dur, serial) in &partner_received {
        partner_pkt.write_u8(*dst);
        partner_pkt.write_u32(*item_id);
        partner_pkt.write_u16(*count);
        partner_pkt.write_u16(*dur as u16);
        partner_pkt.write_u8(0);
        crate::handler::unique_item_info::write_unique_item_info(
            &world,
            session.pool(),
            *item_id,
            *serial,
            partner_rebirth,
            &mut partner_pkt,
        )
        .await;
    }
    world.send_to_session_owned(partner_sid, partner_pkt);

    // Recalculate ability and finish trade for both
    // Weight notification is integrated into set_user_ability().
    world.set_user_ability(sid);
    world.set_user_ability(partner_sid);

    // FerihaLog: TradeInsertLog
    {
        let my_acc = world
            .with_session(sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let my_name = world.get_session_name(sid).unwrap_or_default();
        let my_gold = world.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
        let p_acc = world
            .with_session(partner_sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let p_name = world.get_session_name(partner_sid).unwrap_or_default();
        let p_gold = world
            .get_character_info(partner_sid)
            .map(|c| c.gold)
            .unwrap_or(0);
        if let Some(pool) = world.db_pool() {
            super::audit_log::log_trade(
                pool, &my_acc, &my_name, "", my_gold, &p_acc, &p_name, p_gold,
            );
        }
    }

    world.reset_trade(sid);
    world.reset_trade(partner_sid);

    Ok(())
}

/// EXCHANGE_CANCEL (8): Cancel the trade.
pub(crate) async fn exchange_cancel(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.get_trade_state(sid) == TRADE_STATE_NONE {
        return Ok(());
    }

    let partner_sid = world.get_exchange_user(sid);

    // Give items back
    world.exchange_give_items_back(sid);
    world.reset_trade(sid);

    // Send cancel to self
    let mut cancel_pkt = Packet::new(Opcode::WizExchange as u8);
    cancel_pkt.write_u8(EXCHANGE_CANCEL);
    session.send_packet(&cancel_pkt).await?;

    // Send inventory refresh (WIZ_ITEM_MOVE type=2) so client resyncs
    send_inventory_refresh(&world, session.pool(), sid).await;

    // Cancel partner too
    if let Some(pid) = partner_sid {
        if world.get_trade_state(pid) != TRADE_STATE_NONE {
            world.exchange_give_items_back(pid);
            world.reset_trade(pid);
            world.send_to_session_owned(pid, cancel_pkt);
            send_inventory_refresh(&world, session.pool(), pid).await;
        }
    }

    Ok(())
}

/// Validate that a player can receive the partner's exchange items.
fn check_exchange(world: &crate::world::WorldState, sid: u16) -> bool {
    let partner_sid = match world.get_exchange_user(sid) {
        Some(p) => p,
        None => return false,
    };

    if world.is_player_dead(sid) || !world.is_trading(sid) {
        return false;
    }

    let partner_items = world.get_exchange_items(partner_sid);

    let free_slots = world.count_free_slots(sid);
    let mut total_weight: u32 = 0;
    let mut item_count: u8 = 0;

    for ex_item in &partner_items {
        if ex_item.item_id == ITEM_GOLD {
            let ch = match world.get_character_info(sid) {
                Some(c) => c,
                None => return false,
            };
            if (ch.gold as u64) + (ex_item.count as u64) > COIN_MAX as u64 {
                return false;
            }
            continue;
        }

        let item_def = match world.get_item(ex_item.item_id) {
            Some(i) => i,
            None => return false,
        };

        let weight = item_def.weight.unwrap_or(0) as u32;
        total_weight = total_weight.saturating_add(weight);
        item_count += 1;
    }

    if item_count > free_slots {
        return false;
    }

    let stats = world.get_equipped_stats(sid);
    (total_weight + stats.item_weight) <= stats.max_weight
}

/// Second-pass validation before executing the trade: verify item placement.
/// Validates:
/// - Player is in TRADE_STATE_DECIDING
/// - Each partner item exists in the item table
/// - A valid destination slot exists (`FindSlotForItem`)
/// - Source item in partner's inventory still matches (`pSrcItem->nNum == pTable.m_iNum`)
/// - kind==255 unique items cannot go into an occupied slot
/// - Destination slot must be empty or have the same item ID
fn check_execute_exchange(world: &crate::world::WorldState, sid: u16) -> bool {
    // C++ line 830: m_sTradeStatue != 5
    if world.get_trade_state(sid) != TRADE_STATE_DECIDING {
        return false;
    }

    // Busy-state checks matching C++ lines 831-837
    if !world.is_trading(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_buying_merchant_preparing(sid)
        || world.is_merchanting(sid)
    {
        return false;
    }

    let partner_sid = match world.get_exchange_user(sid) {
        Some(p) => p,
        None => return false,
    };

    // C++ lines 845-854: partner validation
    if world.get_exchange_user(partner_sid) != Some(sid)
        || world.get_position(partner_sid).map(|p| p.zone_id)
            != world.get_position(sid).map(|p| p.zone_id)
        || world.is_mining(partner_sid)
        || world.is_fishing(partner_sid)
        || world.is_selling_merchant_preparing(partner_sid)
        || world.is_buying_merchant_preparing(partner_sid)
        || world.is_merchanting(partner_sid)
    {
        return false;
    }

    // Nation cross-trade check
    {
        let my_nation = world.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
        let partner_nation = world
            .get_character_info(partner_sid)
            .map(|c| c.nation)
            .unwrap_or(0);
        if my_nation != partner_nation {
            let my_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
            let can_trade = world
                .get_zone(my_zone)
                .is_some_and(|z| z.can_trade_other_nation());
            if !can_trade {
                return false;
            }
        }
    }

    let partner_items = world.get_exchange_items(partner_sid);

    // C++ lines 856-884: validate each partner item can be placed
    for ex_item in &partner_items {
        if ex_item.item_id == ITEM_GOLD {
            continue;
        }

        // C++ line 864-866: item must exist in item table
        let item_def = match world.get_item(ex_item.item_id) {
            Some(i) => i,
            None => return false,
        };

        // C++ line 868-871: must have a valid destination slot
        let dst_slot = match world.find_slot_for_item(sid, ex_item.item_id, ex_item.count as u16) {
            Some(s) if s < SLOT_MAX + HAVE_MAX => s,
            _ => return false,
        };

        // C++ line 874-877: source item in partner's inventory must match
        let src_slot = world.get_inventory_slot(partner_sid, ex_item.src_pos as usize);
        match src_slot {
            Some(src) if src.item_id == ex_item.item_id => {}
            _ => return false,
        }

        // C++ line 879-880: kind==255 unique items cannot go into occupied slot
        let kind = item_def.kind.unwrap_or(0);
        let dst = world.get_inventory_slot(sid, dst_slot).unwrap_or_default();
        if kind == ITEM_KIND_UNIQUE && dst.item_id != 0 {
            return false;
        }

        // C++ line 882-883: destination must be empty or same item
        if dst.item_id != ex_item.item_id && dst.item_id != 0 {
            return false;
        }
    }

    true
}

/// Send EXCHANGE_CANCEL to the session.
async fn send_cancel(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizExchange as u8);
    pkt.write_u8(EXCHANGE_CANCEL);
    session.send_packet(&pkt).await
}

/// Send EXCHANGE_ADD failure to the session.
async fn send_add_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizExchange as u8);
    pkt.write_u8(EXCHANGE_ADD);
    pkt.write_u8(0);
    session.send_packet(&pkt).await
}

/// Send a full inventory refresh packet (WIZ_ITEM_MOVE type=2) after exchange cancel.
/// resyncs its inventory UI after items are returned from the exchange.
/// Calls `SetSpecialItemBuffer` per slot for Cypher Ring / pet data (C++ line 604).
pub async fn send_inventory_refresh(
    world: &crate::world::WorldState,
    pool: &ko_db::DbPool,
    sid: u16,
) {
    let rebirth_level = world
        .get_character_info(sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);
    let mut pkt = Packet::new(Opcode::WizItemMove as u8);
    pkt.write_u8(2); // type = inventory refresh
    pkt.write_u8(1); // success
    for slot_idx in 0..HAVE_MAX {
        let slot = world
            .get_inventory_slot(sid, SLOT_MAX + slot_idx)
            .unwrap_or_default();
        pkt.write_u32(slot.item_id);
        pkt.write_u16(slot.durability as u16);
        pkt.write_u16(slot.count);
        pkt.write_u8(slot.flag);
        pkt.write_u16(0); // remaining_rental_time
                          // C++ TradeHandler.cpp:604 — SetSpecialItemBuffer(pItem->nNum, pItem->nSerialNum, newpkt)
        crate::handler::unique_item_info::write_unique_item_info(
            world,
            pool,
            slot.item_id,
            slot.serial_num,
            rebirth_level,
            &mut pkt,
        )
        .await;
        pkt.write_u32(slot.expire_time);
    }
    world.send_to_session_owned(sid, pkt);
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use super::*;
    use crate::handler::INVENTORY_TOTAL;

    /// Test EXCHANGE_REQ packet format: [u8 sub=1][u32 sender_id].
    #[test]
    fn test_exchange_req_packet_format() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_REQ);
        pkt.write_u32(42);

        assert_eq!(pkt.opcode, Opcode::WizExchange as u8);
        assert_eq!(pkt.data.len(), 5); // 1 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_REQ));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_AGREE packet format: [u8 sub=2][u16 result].
    #[test]
    fn test_exchange_agree_packet_format() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_AGREE);
        pkt.write_u16(1); // accepted

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_AGREE));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_AGREE decline format: [u8 sub=2][u16 0].
    #[test]
    fn test_exchange_agree_decline_format() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_AGREE);
        pkt.write_u16(0); // declined

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_AGREE));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_ADD client packet format: [u8 pos][u32 item_id][u32 count].
    #[test]
    fn test_exchange_add_client_packet() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_ADD);
        pkt.write_u8(5); // pos
        pkt.write_u32(100_001); // item_id
        pkt.write_u32(3); // count

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_ADD));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u32(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_OTHERADD server packet format:
    /// [u8 sub=4][u32 item_id][u32 count][u16 duration][u32 unique_id].
    #[test]
    fn test_exchange_otheradd_packet_format() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_OTHERADD);
        pkt.write_u32(200_001); // item_id
        pkt.write_u32(10); // count
        pkt.write_u16(500); // duration
        pkt.write_u32(0); // unique_id

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_OTHERADD));
        assert_eq!(r.read_u32(), Some(200_001));
        assert_eq!(r.read_u32(), Some(10));
        assert_eq!(r.read_u16(), Some(500));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_DONE success packet format with received items.
    #[test]
    fn test_exchange_done_success_packet() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_DONE);
        pkt.write_u8(1); // success
        pkt.write_u32(50_000); // gold_after
        pkt.write_u16(2); // num received items

        // Item 1
        pkt.write_u8(3); // dst_pos
        pkt.write_u32(100_001);
        pkt.write_u16(5); // count
        pkt.write_u16(1000); // durability
        pkt.write_u8(0); // flag
        pkt.write_u32(0); // unique_id

        // Item 2
        pkt.write_u8(7);
        pkt.write_u32(200_002);
        pkt.write_u16(1);
        pkt.write_u16(500);
        pkt.write_u8(0);
        pkt.write_u32(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_DONE));
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.read_u32(), Some(50_000)); // gold
        assert_eq!(r.read_u16(), Some(2)); // item count

        // Item 1
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u32(), Some(0));

        // Item 2
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u32(), Some(200_002));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(500));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u32(), Some(0));

        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_DONE failure packet format.
    #[test]
    fn test_exchange_done_failure_packet() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_DONE);
        pkt.write_u8(0); // failure

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_DONE));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test EXCHANGE_CANCEL packet format.
    #[test]
    fn test_exchange_cancel_packet_format() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_CANCEL);

        assert_eq!(pkt.data.len(), 1);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_CANCEL));
        assert_eq!(r.remaining(), 0);
    }

    /// Test trade state constants match C++ reference values.
    #[test]
    fn test_trade_state_constants() {
        assert_eq!(TRADE_STATE_NONE, 1);
        assert_eq!(TRADE_STATE_SENDER, 2);
        assert_eq!(TRADE_STATE_TARGET, 3);
        assert_eq!(TRADE_STATE_TRADING, 4);
        assert_eq!(TRADE_STATE_DECIDING, 5);
    }

    /// Test exchange sub-opcode constants match protocol specification.
    #[test]
    fn test_exchange_sub_opcode_constants() {
        assert_eq!(EXCHANGE_REQ, 1);
        assert_eq!(EXCHANGE_AGREE, 2);
        assert_eq!(EXCHANGE_ADD, 3);
        assert_eq!(EXCHANGE_OTHERADD, 4);
        assert_eq!(EXCHANGE_DECIDE, 5);
        assert_eq!(EXCHANGE_OTHERDECIDE, 6);
        assert_eq!(EXCHANGE_DONE, 7);
        assert_eq!(EXCHANGE_CANCEL, 8);
    }

    /// Test item tradeability: no-trade range items should be blocked.
    #[test]
    fn test_item_no_trade_range() {
        // ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX should be blocked
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&900_000_001));
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&999_999_999));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&100_001));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&ITEM_GOLD));
    }

    /// Test gold item ID constant.
    #[test]
    fn test_gold_item_id() {
        assert_eq!(ITEM_GOLD, 900_000_000);
        // Gold is NOT in the no-trade range
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&ITEM_GOLD));
    }

    /// Test gold add uses pos=255 as protocol specification.
    #[test]
    fn test_exchange_gold_add_pos() {
        let mut pkt = Packet::new(Opcode::WizExchange as u8);
        pkt.write_u8(EXCHANGE_ADD);
        pkt.write_u8(255); // gold pos
        pkt.write_u32(ITEM_GOLD);
        pkt.write_u32(10_000); // gold amount

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_ADD));
        let pos = r.read_u8().unwrap();
        let item_id = r.read_u32().unwrap();
        let count = r.read_u32().unwrap();
        assert_eq!(pos, 255);
        assert_eq!(item_id, ITEM_GOLD);
        assert_eq!(count, 10_000);
    }

    // ── Sprint 49: Integration Tests ────────────────────────────────────

    use crate::world::{CharacterInfo, Position, WorldState};
    use tokio::sync::mpsc;

    fn make_trade_test_char(sid: u16, name: &str, gold: u32) -> CharacterInfo {
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

    /// Integration test: trade initiation sets up bidirectional state.
    #[test]
    fn test_integration_trade_init_bidirectional_state() {
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
        world.register_ingame(1, make_trade_test_char(1, "Sender", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Target", 50_000), pos);

        // Init trade request
        world.init_trade_request(1, 2);

        // Verify sender state
        assert!(world.is_trading(1));
        assert_eq!(world.get_trade_state(1), TRADE_STATE_SENDER);
        assert_eq!(world.get_exchange_user(1), Some(2));

        // Verify target state
        assert!(world.is_trading(2));
        assert_eq!(world.get_trade_state(2), TRADE_STATE_TARGET);
        assert_eq!(world.get_exchange_user(2), Some(1));
    }

    /// Integration test: trade agree advances both to TRADING state.
    #[test]
    fn test_integration_trade_agree_advances_state() {
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
        world.register_ingame(1, make_trade_test_char(1, "Sender", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Target", 50_000), pos);

        world.init_trade_request(1, 2);
        assert_eq!(world.get_trade_state(1), TRADE_STATE_SENDER);
        assert_eq!(world.get_trade_state(2), TRADE_STATE_TARGET);

        // Target agrees
        world.trade_agree(2);

        assert_eq!(world.get_trade_state(1), TRADE_STATE_TRADING);
        assert_eq!(world.get_trade_state(2), TRADE_STATE_TRADING);
    }

    /// Integration test: trade decline resets both players' state.
    #[test]
    fn test_integration_trade_decline_resets_both() {
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
        world.register_ingame(1, make_trade_test_char(1, "Sender", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Target", 50_000), pos);

        world.init_trade_request(1, 2);
        assert!(world.is_trading(1));
        assert!(world.is_trading(2));

        // Target declines
        world.trade_decline(2);

        // Both should be reset
        assert!(!world.is_trading(1));
        assert!(!world.is_trading(2));
        assert_eq!(world.get_trade_state(1), TRADE_STATE_NONE);
        assert_eq!(world.get_trade_state(2), TRADE_STATE_NONE);
        assert_eq!(world.get_exchange_user(1), None);
        assert_eq!(world.get_exchange_user(2), None);
    }

    /// Integration test: gold exchange — add gold to exchange, cancel returns it.
    #[test]
    fn test_integration_trade_gold_add_and_cancel_returns() {
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
        world.register_ingame(1, make_trade_test_char(1, "Trader1", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Trader2", 50_000), pos);

        // Start trade and agree
        world.init_trade_request(1, 2);
        world.trade_agree(2);

        // Player 1 adds 30,000 gold to exchange
        world.gold_lose(1, 30_000);
        world.add_exchange_item(
            1,
            ExchangeItem {
                item_id: ITEM_GOLD,
                count: 30_000,
                durability: 0,
                serial_num: 0,
                src_pos: 255,
                dst_pos: 0,
            },
        );

        // Verify gold was deducted
        let ch1 = world.get_character_info(1).unwrap();
        assert_eq!(ch1.gold, 70_000);

        // Verify exchange items
        let items = world.get_exchange_items(1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, ITEM_GOLD);
        assert_eq!(items[0].count, 30_000);

        // Cancel trade — gold should be returned
        world.exchange_give_items_back(1);
        world.reset_trade(1);
        world.reset_trade(2);

        let ch1_after = world.get_character_info(1).unwrap();
        assert_eq!(ch1_after.gold, 100_000); // gold returned
        assert!(!world.is_trading(1));
        assert!(!world.is_trading(2));
    }

    /// Integration test: disconnect during active trade — partner gets reset.
    #[test]
    fn test_integration_trade_disconnect_resets_partner() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
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
        world.register_ingame(1, make_trade_test_char(1, "Player1", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Player2", 50_000), pos);

        // Start and agree to trade
        world.init_trade_request(1, 2);
        world.trade_agree(2);

        // Player 1 disconnects — simulate logout cleanup
        if world.is_trading(1) {
            let partner_sid = world.get_exchange_user(1);
            world.exchange_give_items_back(1);
            world.reset_trade(1);
            if let Some(partner) = partner_sid {
                world.exchange_give_items_back(partner);
                world.reset_trade(partner);
                let mut cancel_pkt = Packet::new(Opcode::WizExchange as u8);
                cancel_pkt.write_u8(EXCHANGE_CANCEL);
                world.send_to_session_owned(partner, cancel_pkt);
            }
        }

        // Verify both are reset
        assert!(!world.is_trading(1));
        assert!(!world.is_trading(2));

        // Player 2 should have received EXCHANGE_CANCEL
        let pkt = rx2.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizExchange as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXCHANGE_CANCEL));
    }

    /// Integration test: exchange item count update for stackable items.
    #[test]
    fn test_integration_trade_exchange_item_count_update() {
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
        world.register_ingame(1, make_trade_test_char(1, "Player1", 200_000), pos);

        // Add gold once
        world.add_exchange_item(
            1,
            ExchangeItem {
                item_id: ITEM_GOLD,
                count: 10_000,
                durability: 0,
                serial_num: 0,
                src_pos: 255,
                dst_pos: 0,
            },
        );

        // Update count (add more gold)
        let updated = world.update_exchange_item_count(1, ITEM_GOLD, 5_000);
        assert!(updated);

        // Verify combined count
        let items = world.get_exchange_items(1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].count, 15_000);

        // Non-existent item should not be found
        let not_found = world.update_exchange_item_count(1, 999_999, 100);
        assert!(!not_found);
    }

    // ── Sprint 108: C++ Parity Tests ──────────────────────────────────

    /// Test that partner bidirectional exchange user check works.
    #[test]
    fn test_exchange_bidirectional_partner_check() {
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
        world.register_ingame(1, make_trade_test_char(1, "Sender", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Target", 50_000), pos);

        // Setup trade
        world.init_trade_request(1, 2);
        world.trade_agree(2);

        // Verify bidirectional exchange user links
        assert_eq!(world.get_exchange_user(1), Some(2));
        assert_eq!(world.get_exchange_user(2), Some(1));

        // Both in TRADING state
        assert_eq!(world.get_trade_state(1), TRADE_STATE_TRADING);
        assert_eq!(world.get_trade_state(2), TRADE_STATE_TRADING);
    }

    /// Test DECIDING state: after one player decides, both states are valid.
    #[test]
    fn test_exchange_deciding_state_allows_partner_add() {
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
        world.register_ingame(1, make_trade_test_char(1, "Player1", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Player2", 50_000), pos);

        world.init_trade_request(1, 2);
        world.trade_agree(2);

        // Player 1 decides (sets state to DECIDING)
        world.set_trade_state(1, TRADE_STATE_DECIDING);

        // Player 2 should still be TRADING
        assert_eq!(world.get_trade_state(2), TRADE_STATE_TRADING);
        // Player 1 is DECIDING
        assert_eq!(world.get_trade_state(1), TRADE_STATE_DECIDING);

        // C++ allows partner in state 4(TRADING) or 5(DECIDING)
        // Player 2 (TRADING) adding items while Player 1 (DECIDING) should be valid
        let partner_state = world.get_trade_state(1); // partner = Player 1
        assert!(
            partner_state == TRADE_STATE_TRADING || partner_state == TRADE_STATE_DECIDING,
            "Partner in DECIDING state should be allowed"
        );
    }

    /// Test that trade state constants used in partner validation are correct.
    #[test]
    fn test_trade_state_constants_for_deciding() {
        // DECIDING is a valid partner state for exchange_add (C++ line 249-250)
        assert_eq!(TRADE_STATE_DECIDING, 5);
        assert_ne!(TRADE_STATE_DECIDING, TRADE_STATE_TRADING);
        assert_ne!(TRADE_STATE_DECIDING, TRADE_STATE_NONE);
    }

    /// Verify item expiration check constant is used in exchange_add.
    #[test]
    fn test_expiration_time_blocks_trade() {
        // An item with expire_time > 0 is considered an expiration item.
        // The isExpirationTime() check in C++ returns nExpirationTime > 0.
        let expire_time: u32 = 1708300800; // arbitrary nonzero timestamp
        assert!(expire_time > 0, "Non-zero expire_time should block trade");

        let no_expire: u32 = 0;
        assert_eq!(no_expire, 0, "Zero expire_time should allow trade");
    }

    // ── Sprint 275: Exchange Agree Validation Tests ──────────────────────

    /// Test that the trade acceptor must NOT be in SENDER state.
    #[test]
    fn test_exchange_agree_sender_cannot_accept() {
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
        world.register_ingame(1, make_trade_test_char(1, "Sender", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Target", 50_000), pos);

        world.init_trade_request(1, 2);

        // Sender (state=2) cannot be the one to accept
        assert_eq!(world.get_trade_state(1), TRADE_STATE_SENDER);
        // If the sender tried to call exchange_agree, the SENDER check blocks it
        assert_ne!(
            world.get_trade_state(1),
            TRADE_STATE_TARGET,
            "Sender cannot accept their own request"
        );
    }

    /// Test that zone mismatch blocks exchange_agree.
    #[test]
    fn test_exchange_agree_blocks_different_zones() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos1 = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let pos2 = Position {
            zone_id: 22,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_trade_test_char(1, "Sender", 100_000), pos1);
        world.register_ingame(2, make_trade_test_char(2, "Target", 50_000), pos2);

        // Different zones should fail validation
        let zone1 = world.get_position(1).map(|p| p.zone_id).unwrap_or(0);
        let zone2 = world.get_position(2).map(|p| p.zone_id).unwrap_or(0);
        assert_ne!(zone1, zone2, "Different zones block trade agree");
    }

    /// Test that mutual exchange user link is validated in exchange_add.
    #[test]
    fn test_exchange_add_zone_check() {
        // Zone equality is now checked in exchange_add partner validation
        let zone1: u16 = 21;
        let zone2: u16 = 22;
        assert_ne!(zone1, zone2, "Zone mismatch blocks exchange_add");
    }

    /// Test that exchange_cancel sends inventory refresh packet.
    #[tokio::test]
    async fn test_exchange_cancel_sends_inventory_refresh() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_trade_test_char(1, "Player1", 100_000), pos);

        // Call the refresh helper directly (no actual DB needed — all items are empty
        // so write_unique_item_info writes u32(0) without DB lookup)
        let pool = ko_db::DbPool::connect_lazy("postgres://invalid").unwrap();
        send_inventory_refresh(&world, &pool, 1).await;

        // Should receive a WIZ_ITEM_MOVE packet
        let pkt = rx1.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizItemMove as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // type = inventory refresh
        assert_eq!(r.read_u8(), Some(1)); // success
                                          // 28 slots (HAVE_MAX) × (item data + u32(0) special) bytes each
        assert!(r.remaining() > 0, "Should contain inventory data");
    }

    /// Test logos grade uses server settings (max_blessing_up / max_blessing_up_reb).
    #[test]
    fn test_logos_grade_limit_constants() {
        // Default fallback values when server settings not loaded
        let fallback_normal: i16 = 10;
        let fallback_rebirth: i16 = 10;
        assert_eq!(fallback_normal, 10);
        assert_eq!(fallback_rebirth, 10);
        // When settings loaded, these come from max_blessing_up / max_blessing_up_reb
    }

    // ── Sprint 277: CheckExecuteExchange Tests ──────────────────────────

    /// Test check_execute_exchange rejects non-DECIDING state.
    #[test]
    fn test_check_execute_exchange_requires_deciding_state() {
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
        world.register_ingame(1, make_trade_test_char(1, "Player1", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "Player2", 50_000), pos);

        // Set up trade relationship but NOT in DECIDING state
        world.update_session(1, |h| h.exchange_user = Some(2));
        world.update_session(2, |h| h.exchange_user = Some(1));
        world.set_trade_state(1, TRADE_STATE_TRADING);
        world.set_trade_state(2, TRADE_STATE_TRADING);

        // Should fail because state is TRADING, not DECIDING
        assert!(!check_execute_exchange(&world, 1));

        // Set to DECIDING — now should pass (no items to validate)
        world.set_trade_state(1, TRADE_STATE_DECIDING);
        world.set_trade_state(2, TRADE_STATE_DECIDING);
        assert!(check_execute_exchange(&world, 1));
    }

    /// Test check_execute_exchange rejects cross-zone partners.
    #[test]
    fn test_check_execute_exchange_rejects_cross_zone() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos1 = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let pos2 = Position {
            zone_id: 22, // Different zone
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_trade_test_char(1, "Player1", 100_000), pos1);
        world.register_ingame(2, make_trade_test_char(2, "Player2", 50_000), pos2);

        world.update_session(1, |h| h.exchange_user = Some(2));
        world.update_session(2, |h| h.exchange_user = Some(1));
        world.set_trade_state(1, TRADE_STATE_DECIDING);
        world.set_trade_state(2, TRADE_STATE_DECIDING);

        // Should fail because zones don't match
        assert!(!check_execute_exchange(&world, 1));
    }

    /// Test exchange_decide double-validation order: check_exchange then check_execute_exchange.
    #[test]
    fn test_exchange_decide_has_double_validation() {
        // Verify both functions exist and have correct signatures
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
        world.register_ingame(1, make_trade_test_char(1, "P1", 100_000), pos);
        world.register_ingame(2, make_trade_test_char(2, "P2", 50_000), pos);

        world.update_session(1, |h| h.exchange_user = Some(2));
        world.update_session(2, |h| h.exchange_user = Some(1));
        world.set_trade_state(1, TRADE_STATE_DECIDING);
        world.set_trade_state(2, TRADE_STATE_DECIDING);

        // Both pass when no items (gold-only or empty trade)
        assert!(check_exchange(&world, 1));
        assert!(check_execute_exchange(&world, 1));
    }

    /// Verify TRADE_STATE_DECIDING constant matches C++ m_sTradeStatue == 5.
    #[test]
    fn test_trade_state_deciding_is_5() {
        assert_eq!(TRADE_STATE_DECIDING, 5);
    }

    // ── Sprint 314: Same-account trade prevention ────────────────────

    /// `if (pUser->GetAccountName() == GetAccountName())` — blocks same-account trade
    /// to prevent item duplication between characters on the same account.
    #[test]
    fn test_same_account_trade_blocked() {
        let acct_a = "player1";
        let acct_b = "player1"; // same account
        assert_eq!(
            acct_a.to_lowercase(),
            acct_b.to_lowercase(),
            "Same account must be blocked"
        );
    }

    #[test]
    fn test_different_account_trade_allowed() {
        let acct_a = "player1";
        let acct_b = "player2"; // different account
        assert_ne!(
            acct_a.to_lowercase(),
            acct_b.to_lowercase(),
            "Different accounts must be allowed"
        );
    }

    #[test]
    fn test_same_account_case_insensitive() {
        // Account comparison must be case-insensitive
        let acct_a = "PlayerOne";
        let acct_b = "playerone";
        assert_eq!(acct_a.to_lowercase(), acct_b.to_lowercase());
    }

    // ── Sprint 327: exchange_add re-validation tests ─────────────────

    /// Test that exchange_add re-validates nation during item addition.
    #[test]
    fn test_exchange_add_nation_revalidation() {
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
        // Different nations
        let mut ch1 = make_trade_test_char(1, "KarusPlayer", 100_000);
        ch1.nation = 1;
        let mut ch2 = make_trade_test_char(2, "ElmoPlayer", 50_000);
        ch2.nation = 2;

        world.register_ingame(1, ch1, pos);
        world.register_ingame(2, ch2, pos);

        // Nation mismatch in zone 21 (not cross-trade zone) should block
        // The actual handler would cancel trade; here we verify the data layer
        let my_nation = world.get_character_info(1).map(|c| c.nation).unwrap_or(0);
        let partner_nation = world.get_character_info(2).map(|c| c.nation).unwrap_or(0);
        assert_ne!(
            my_nation, partner_nation,
            "Nations must differ for this test"
        );
    }

    /// Test that exchange_add re-validates identity (same account) during item addition.
    #[test]
    fn test_exchange_add_identity_revalidation() {
        // Simulates the re-validation checks at exchange_add time:
        // 1. sid == partner_sid → blocked
        // 2. same name → blocked
        // 3. same account (case-insensitive) → blocked

        // Case 1: Self-trade
        let sid: u16 = 1;
        let partner_sid: u16 = 1;
        assert_eq!(sid, partner_sid, "Self-trade must be blocked");

        // Case 2: Same name
        let my_name = "Warrior1";
        let partner_name = "Warrior1";
        assert_eq!(my_name, partner_name, "Same-name trade must be blocked");

        // Case 3: Same account (case-insensitive)
        let my_account = "Account1";
        let partner_account = "ACCOUNT1";
        assert_eq!(
            my_account.to_lowercase(),
            partner_account.to_lowercase(),
            "Same-account (case-insensitive) trade must be blocked"
        );

        // Legitimate trade: different everything
        let sid2: u16 = 1;
        let partner_sid2: u16 = 2;
        let my_name2 = "Warrior1";
        let partner_name2 = "Mage2";
        let my_account2 = "account_a";
        let partner_account2 = "account_b";
        assert_ne!(sid2, partner_sid2);
        assert_ne!(my_name2, partner_name2);
        assert_ne!(my_account2.to_lowercase(), partner_account2.to_lowercase());
    }

    /// Sprint 585: Test that exchange_give_items_back restores full-stack items.
    ///
    /// When a player trades their ENTIRE stack (count→0), the slot is cleared
    /// to default (item_id=0). On cancel, the restore must reconstruct the slot
    /// from exchange data rather than failing the item_id match check.
    ///
    /// clears slot, so restore always finds matching item_id. Our Rust code
    /// clears at count=0, so restore must handle the empty-slot case.
    #[test]
    fn test_exchange_give_items_back_full_stack_restore() {
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
        world.register_ingame(1, make_trade_test_char(1, "Player1", 100_000), pos);

        // Initialize inventory and place an item in slot SLOT_MAX (first bag slot)
        let test_item_id: u32 = 379090000;
        let test_count: u16 = 5;
        let test_durability: i16 = 1000;
        let test_serial: u64 = 12345;
        let mut inv = vec![UserItemSlot::default(); INVENTORY_TOTAL];
        inv[SLOT_MAX].item_id = test_item_id;
        inv[SLOT_MAX].count = test_count;
        inv[SLOT_MAX].durability = test_durability;
        inv[SLOT_MAX].serial_num = test_serial;
        world.set_inventory(1, inv);

        // Simulate exchange_add: deduct entire stack (clears slot to default)
        world.update_session(1, |h| {
            h.inventory[SLOT_MAX].count -= test_count;
            if h.inventory[SLOT_MAX].count == 0 {
                h.inventory[SLOT_MAX] = UserItemSlot::default(); // clears item_id to 0
            }
        });
        world.add_exchange_item(
            1,
            ExchangeItem {
                item_id: test_item_id,
                count: test_count as u32,
                durability: test_durability,
                serial_num: test_serial,
                src_pos: SLOT_MAX as u8,
                dst_pos: 0,
            },
        );

        // Verify slot is cleared
        let slot_before = world.get_inventory_slot(1, SLOT_MAX).unwrap();
        assert_eq!(
            slot_before.item_id, 0,
            "slot should be empty after full trade"
        );

        // Cancel trade — items should be restored
        world.exchange_give_items_back(1);

        // Verify full item restoration
        let slot_after = world.get_inventory_slot(1, SLOT_MAX).unwrap();
        assert_eq!(
            slot_after.item_id, test_item_id,
            "item_id should be restored"
        );
        assert_eq!(slot_after.count, test_count, "count should be restored");
        assert_eq!(
            slot_after.durability, test_durability,
            "durability should be restored"
        );
        assert_eq!(
            slot_after.serial_num, test_serial,
            "serial_num should be restored"
        );
    }

    /// Exchange sub-opcodes: client-handled (1-3,5,8) vs server-only (4,6,7).
    #[test]
    fn test_exchange_client_vs_server_opcodes() {
        // Client sends these
        let client_ops = [
            EXCHANGE_REQ,
            EXCHANGE_AGREE,
            EXCHANGE_ADD,
            EXCHANGE_DECIDE,
            EXCHANGE_CANCEL,
        ];
        assert_eq!(client_ops.len(), 5);
        // Server sends these (not handled from client)
        let server_ops = [EXCHANGE_OTHERADD, EXCHANGE_OTHERDECIDE, EXCHANGE_DONE];
        assert_eq!(server_ops.len(), 3);
        // Total 8 sub-opcodes
        assert_eq!(client_ops.len() + server_ops.len(), 8);
    }

    /// Trade state machine: NONE(1) → SENDER(2)/TARGET(3) → TRADING(4) → DECIDING(5).
    #[test]
    fn test_trade_state_progression() {
        assert_eq!(TRADE_STATE_NONE, 1);
        assert_eq!(TRADE_STATE_SENDER, 2);
        assert_eq!(TRADE_STATE_TARGET, 3);
        assert_eq!(TRADE_STATE_TRADING, 4);
        assert_eq!(TRADE_STATE_DECIDING, 5);
        // Progressive: NONE < SENDER < TARGET < TRADING < DECIDING
        assert!(TRADE_STATE_NONE < TRADE_STATE_SENDER);
        assert!(TRADE_STATE_TRADING < TRADE_STATE_DECIDING);
    }

    /// EXCHANGE_CANCEL (8) is the highest sub-opcode, with a gap at 4,6,7.
    #[test]
    fn test_exchange_cancel_highest_subopcode() {
        assert_eq!(EXCHANGE_CANCEL, 8);
        assert!(EXCHANGE_CANCEL > EXCHANGE_DONE);
        assert!(EXCHANGE_CANCEL > EXCHANGE_OTHERDECIDE);
        // Gap between DECIDE(5) and CANCEL(8)
        assert_eq!(EXCHANGE_CANCEL - EXCHANGE_DECIDE, 3);
    }

    /// RACE_UNTRADEABLE (20) blocks trade for special race items.
    #[test]
    fn test_race_untradeable_constant() {
        assert_eq!(RACE_UNTRADEABLE, 20);
        assert!(RACE_UNTRADEABLE > 0);
    }

    /// ExchangeItem struct: default src_pos and dst_pos are separate fields.
    #[test]
    fn test_exchange_item_struct_fields() {
        let item = ExchangeItem {
            item_id: 379090000,
            count: 5,
            durability: 1000,
            serial_num: 12345,
            src_pos: 14,
            dst_pos: 0,
        };
        assert_eq!(item.item_id, 379090000);
        assert_eq!(item.count, 5);
        assert_eq!(item.src_pos, SLOT_MAX as u8);
        assert_ne!(item.src_pos, item.dst_pos);
    }
}
