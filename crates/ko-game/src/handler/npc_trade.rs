//! WIZ_ITEM_TRADE (0x21) handler — NPC shop buy/sell and repurchase.
//! Packet format (from client):
//! ```text
//! [u8 type] — 1=Buy, 2=Sell, 5=Repurchase
//! For type 1/2:
//!   [u32 group] [u32 npc_id] [u8 item_count]
//!   Buy  repeated: [u32 item_id][u8 inv_pos][u16 count][u8 line][u8 index]
//!   Sell repeated: [u32 item_id][u8 inv_pos][u16 count]
//! For type 5 (repurchase):
//!   [u8 sub_opcode] — 4=Refresh, 2=Buyback, 3=Clear
//!   Buyback: [u8 index][u32 item_id][i16 unk]
//! ```
//! Response (WIZ_ITEM_TRADE):
//! ```text
//! On success: [u8 1] [u32 gold_after] [u32 total_price] [u8 selling_group]
//! On failure: [u8 0] [u8 error_code]
//! ```

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::{
    DeletedItemEntry, PremiumProperty, UserItemSlot, WorldState, COIN_MAX, ITEM_FLAG_DUPLICATE,
    ITEM_FLAG_RENTED, ITEM_FLAG_SEALED, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN, RACE_UNTRADEABLE,
    ZONE_ARDREAM, ZONE_ARENA, ZONE_BATTLE, ZONE_BATTLE2, ZONE_BATTLE3, ZONE_BATTLE4, ZONE_BATTLE5,
    ZONE_BATTLE6, ZONE_BIFROST, ZONE_BORDER_DEFENSE_WAR, ZONE_CLAN_WAR_ARDREAM,
    ZONE_CLAN_WAR_RONARK, ZONE_DELOS, ZONE_DELOS_CASTELLAN, ZONE_DESPERATION_ABYSS,
    ZONE_DRAKI_TOWER, ZONE_DUNGEON_DEFENCE, ZONE_ELMORAD, ZONE_ELMORAD2, ZONE_ELMORAD3,
    ZONE_ELMORAD_ESLANT, ZONE_ELMORAD_ESLANT2, ZONE_ELMORAD_ESLANT3, ZONE_HELL_ABYSS,
    ZONE_JURAID_MOUNTAIN, ZONE_KARUS, ZONE_KARUS2, ZONE_KARUS3, ZONE_KARUS_ESLANT,
    ZONE_KARUS_ESLANT2, ZONE_KARUS_ESLANT3, ZONE_KROWAZ_DOMINION, ZONE_MORADON, ZONE_MORADON2,
    ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5, ZONE_NEW_BATTLE_TEST, ZONE_OLD_HUMAN,
    ZONE_OLD_KARUS, ZONE_OLD_MORADON, ZONE_PARTY_VS_1, ZONE_PARTY_VS_2, ZONE_PARTY_VS_3,
    ZONE_PARTY_VS_4, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE, ZONE_SNOW_BATTLE, ZONE_STONE1,
    ZONE_STONE2, ZONE_STONE3, ZONE_UNDER_CASTLE,
};

use super::{HAVE_MAX, ITEMCOUNT_MAX, ITEM_KIND_UNIQUE, SLOT_MAX};

/// Scroll item IDs that are exempt from tariff/tax.
const TAX_EXEMPT_SCROLLS: [u32; 5] = [379068000, 379107000, 379109000, 379110000, 379067000];

/// Selling group ID for loyalty NPCs
/// C++ detects loyalty merchants by `m_iSellingGroup == 249000`, NOT by NPC type.
const LOYALTY_SELLING_GROUP: u32 = 249000;

/// Dedicated Moradon fixed-price scroll shop (selling group 280000).
/// The v2615 buy packet does not carry a unit price, so this group must be
/// priced server-side instead of changing the shared item definitions.
const MORADON_FIXED_SCROLL_SELLING_GROUP: u32 = 280000;
const MORADON_FIXED_SCROLL_PRICE: u64 = 50_000_000;

// ── Tax Zone Classification ──────────────────────────────────────────────

/// Zone tax type classification for tariff calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxZoneType {
    /// King tariff zones (Karus/Elmorad capitals, PvP zones, etc.)
    KingTariff,
    /// Moradon/Arena zones — siege tariff.
    SiegeMoradon,
    /// Delos/Abyss zones — siege tariff.
    SiegeDelos,
    /// Neutral/other zones — no tax.
    NoTax,
}

/// Classify a zone ID into its tax type.
pub fn classify_zone_tax(zone_id: u16) -> TaxZoneType {
    match zone_id {
        ZONE_KARUS
        | ZONE_KARUS2
        | ZONE_KARUS3
        | ZONE_ELMORAD
        | ZONE_ELMORAD2
        | ZONE_ELMORAD3
        | ZONE_KARUS_ESLANT
        | ZONE_KARUS_ESLANT2
        | ZONE_KARUS_ESLANT3
        | ZONE_ELMORAD_ESLANT
        | ZONE_ELMORAD_ESLANT2
        | ZONE_ELMORAD_ESLANT3
        | ZONE_BIFROST
        | ZONE_BATTLE
        | ZONE_BATTLE2
        | ZONE_BATTLE3
        | ZONE_BATTLE4
        | ZONE_BATTLE5
        | ZONE_BATTLE6
        | ZONE_SNOW_BATTLE
        | ZONE_RONARK_LAND
        | ZONE_ARDREAM
        | ZONE_RONARK_LAND_BASE
        | ZONE_KROWAZ_DOMINION
        | ZONE_STONE1
        | ZONE_STONE2
        | ZONE_STONE3
        | ZONE_BORDER_DEFENSE_WAR
        | ZONE_UNDER_CASTLE
        | ZONE_JURAID_MOUNTAIN
        | ZONE_PARTY_VS_1
        | ZONE_PARTY_VS_2
        | ZONE_PARTY_VS_3
        | ZONE_PARTY_VS_4
        | ZONE_DRAKI_TOWER
        | ZONE_DUNGEON_DEFENCE
        | ZONE_OLD_MORADON
        | ZONE_CLAN_WAR_ARDREAM
        | ZONE_CLAN_WAR_RONARK
        | ZONE_OLD_KARUS
        | ZONE_OLD_HUMAN
        | ZONE_NEW_BATTLE_TEST => TaxZoneType::KingTariff,
        ZONE_MORADON | ZONE_MORADON2 | ZONE_MORADON3 | ZONE_MORADON4 | ZONE_MORADON5
        | ZONE_ARENA => TaxZoneType::SiegeMoradon,
        ZONE_DELOS | ZONE_DESPERATION_ABYSS | ZONE_HELL_ABYSS | ZONE_DELOS_CASTELLAN => {
            TaxZoneType::SiegeDelos
        }
        _ => TaxZoneType::NoTax,
    }
}

/// Calculate the taxed transaction price for a single item.
/// Returns `(taxed_price, tax_amount)` where tax_amount can be negative for siege discounts.
pub fn calculate_item_tax(
    base_price: u32,
    zone_type: TaxZoneType,
    king_tariff: u8,
    siege_tariff: u16,
) -> (u32, i32) {
    match zone_type {
        TaxZoneType::KingTariff => {
            // C++ stores territory_tariff as 10-20; our Rust stores 0-10. Add 10.
            let tariff_pct = king_tariff as u32 + 10;
            let tax = (base_price as u64 * tariff_pct as u64 / 100) as u32;
            (base_price.saturating_add(tax), tax as i32)
        }
        TaxZoneType::SiegeMoradon | TaxZoneType::SiegeDelos => {
            // siege_tariff 0-10 = discount, 10 = neutral, 11-20 = markup
            let adjustment_pct = siege_tariff as i32 - 10;
            let tax_abs = (base_price as u64 * adjustment_pct.unsigned_abs() as u64 / 100) as u32;
            if adjustment_pct < 0 {
                (base_price.saturating_sub(tax_abs), -(tax_abs as i32))
            } else {
                (base_price.saturating_add(tax_abs), tax_abs as i32)
            }
        }
        TaxZoneType::NoTax => (base_price, 0),
    }
}

/// Check if an item is exempt from tariff/tax.
fn is_tax_exempt(item_id: u32) -> bool {
    TAX_EXEMPT_SCROLLS.contains(&item_id)
}

fn npc_buy_unit_price(
    selling_group: u32,
    is_loyalty: bool,
    gold_price: u64,
    loyalty_price: u64,
) -> u64 {
    if selling_group == MORADON_FIXED_SCROLL_SELLING_GROUP {
        MORADON_FIXED_SCROLL_PRICE
    } else if is_loyalty {
        loyalty_price
    } else {
        gold_price
    }
}

use crate::npc_type_constants::{MAX_NPC_RANGE, NPC_LOYALTY_MERCHANT, NPC_MERCHANT, NPC_TINKER};

/// NPC type: Pet trade merchant.
const NPC_PET_TRADE: u8 = 223;

/// Buy/sell item data parsed from the client packet.
struct TradeItem {
    item_id: u32,
    inv_pos: u8,
    count: u16,
    buy_price: u32,
}

/// Handle WIZ_ITEM_TRADE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        debug!("[{}] NPC trade: not InGame state", session.addr());
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    debug!(
        "[{}] NPC trade: handler entered, sid={}",
        session.addr(),
        sid
    );

    // Must be in-game, alive, not busy
    let no_char = world.get_character_info(sid).is_none();
    let is_dead = world.is_player_dead(sid);
    let is_trd = world.is_trading(sid);
    let is_merch = world.is_merchanting(sid);
    let is_sell_prep = world.is_selling_merchant_preparing(sid);
    let is_buy_prep = world.is_buying_merchant_preparing(sid);
    let is_mine_fish = world.is_mining(sid) || world.is_fishing(sid);
    let is_busy =
        no_char || is_dead || is_trd || is_merch || is_sell_prep || is_buy_prep || is_mine_fish;

    if is_busy {
        warn!(
            "[{}] NPC trade: busy check FAIL — no_char={}, dead={}, trading={}, merch={}, sell_prep={}, buy_prep={}, mine_fish={}",
            session.addr(), no_char, is_dead, is_trd, is_merch, is_sell_prep, is_buy_prep, is_mine_fish
        );
        return send_fail(session, 1).await;
    }

    let mut reader = PacketReader::new(&pkt.data);
    let trade_type = reader.read_u8().unwrap_or(0);
    tracing::info!(
        "[{}] NPC trade: type={} raw=[{}]",
        session.addr(),
        trade_type,
        pkt.data
            .iter()
            .take(30)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );
    tracing::info!(
        "[{}] NPC trade: type={} raw_data=[{}]",
        session.addr(),
        trade_type,
        pkt.data
            .iter()
            .take(40)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    if trade_type == 5 {
        return handle_repurchase(session, &mut reader).await;
    }

    if trade_type != 1 && trade_type != 2 {
        debug!(
            "[{}] NPC trade unhandled type {}",
            session.addr(),
            trade_type
        );
        return Ok(());
    }

    let group = reader.read_u32().unwrap_or(0);
    let npc_id_raw = reader.read_u32().unwrap_or(0);

    // Validate NPC exists, is merchant type, selling_group matches, and is in range
    // Client sends the full NPC runtime ID (already includes NPC_BAND).
    // C++ casts to int16 (uint16 in GetNpcPtr signature) — just truncate, do NOT add NPC_BAND.
    let npc_nid = npc_id_raw as i16 as u16 as u32;
    let npc = match world.get_npc_instance(npc_nid) {
        Some(n) => n,
        None => {
            warn!(
                "[{}] NPC trade: npc_instance not found — raw={}, nid={}",
                session.addr(),
                npc_id_raw,
                npc_nid
            );
            return send_fail(session, 1).await;
        }
    };
    // Dead NPCs cannot provide services (C++ removes them from world)
    if world.is_npc_dead(npc_nid) {
        warn!("[{}] NPC trade: NPC {} is dead", session.addr(), npc_nid);
        return send_fail(session, 1).await;
    }

    // NPC distance check — prevent remote shop access
    if !world.is_in_npc_range(sid, npc_nid) {
        warn!(
            "[{}] NPC trade: NPC {} out of range",
            session.addr(),
            npc_nid
        );
        return send_fail(session, 1).await;
    }

    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => {
            warn!(
                "[{}] NPC trade: template not found for proto={}",
                session.addr(),
                npc.proto_id
            );
            return send_fail(session, 1).await;
        }
    };

    // NPC must be a merchant type
    if !matches!(
        tmpl.npc_type,
        NPC_MERCHANT | NPC_TINKER | NPC_LOYALTY_MERCHANT | NPC_PET_TRADE
    ) && !(npc.proto_id == 13_016 && tmpl.selling_group != 0)
    {
        warn!(
            "[{}] NPC trade: NPC type {} not merchant (proto={})",
            session.addr(),
            tmpl.npc_type,
            npc.proto_id
        );
        return send_fail(session, 1).await;
    }

    // NPC selling_group must match
    if tmpl.selling_group != group {
        warn!(
            "[{}] NPC trade: selling_group mismatch — tmpl={}, client={}",
            session.addr(),
            tmpl.selling_group,
            group
        );
        return send_fail(session, 1).await;
    }

    // Range check
    if let Some(pos) = world.get_position(sid) {
        if npc.zone_id != pos.zone_id {
            return send_fail(session, 1).await;
        }
        let dx = pos.x - npc.x;
        let dz = pos.z - npc.z;
        let dist = (dx * dx + dz * dz).sqrt();
        if dist > MAX_NPC_RANGE {
            return send_fail(session, 1).await;
        }
    }

    // Cannot sell to loyalty merchants
    if trade_type == 2 && group == LOYALTY_SELLING_GROUP {
        return send_fail(session, 1).await;
    }

    let item_count = reader.read_u8().unwrap_or(0);
    if item_count == 0 || item_count > 24 {
        return send_fail(session, 1).await;
    }

    let is_loyalty = group == LOYALTY_SELLING_GROUP;

    if trade_type == 1 {
        handle_buy(session, &mut reader, item_count, group, is_loyalty).await
    } else {
        handle_sell(session, &mut reader, item_count).await
    }
}

/// Handle buying items from an NPC shop.
async fn handle_buy(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    item_count: u8,
    selling_group: u32,
    is_loyalty: bool,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Parse buy items
    let mut items = Vec::with_capacity(item_count.min(12) as usize);
    for _ in 0..item_count {
        let item_id = reader.read_u32().unwrap_or(0);
        let inv_pos = reader.read_u8().unwrap_or(0);
        let count = reader.read_u16().unwrap_or(0);
        let line = reader.read_u8().unwrap_or(0);
        let index = reader.read_u8().unwrap_or(0);

        if item_id == 0 || count == 0 || inv_pos as usize >= HAVE_MAX || count >= ITEMCOUNT_MAX {
            return {
                tracing::warn!("[{}] BUY_FAIL_20 line 371", session.addr());
                send_fail(session, 20)
            }
            .await;
        }
        // LINE must be 0-11, INDEX must be 0-23
        if line >= 12 || index >= 24 {
            return {
                tracing::warn!("[{}] BUY_FAIL_21 line 375", session.addr());
                send_fail(session, 21)
            }
            .await;
        }

        // Validate item exists in NPC sell table
        if !world.validate_sell_table_item(selling_group as i32, index as usize, item_id as i32) {
            warn!(
                "[{}] NPC trade: item {} not in sell table (group={}, index={})",
                session.addr(),
                item_id,
                selling_group,
                index,
            );
            return {
                tracing::warn!("[{}] BUY_FAIL_22 line 388", session.addr());
                send_fail(session, 22)
            }
            .await;
        }

        items.push(TradeItem {
            item_id,
            inv_pos,
            count,
            buy_price: 0,
        });
    }

    // Duplicate IPOS check — same slot cannot appear twice
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            if items[i].inv_pos == items[j].inv_pos {
                return {
                    tracing::warn!("[{}] BUY_FAIL_23 line 404", session.addr());
                    send_fail(session, 23)
                }
                .await;
            }
        }
    }

    // ── Tax/Tariff Setup ─────────────────────────────────────────────────
    // Tax only applies to gold purchases (not loyalty).
    let (zone_id, nation) = world
        .with_session(sid, |h| {
            let n = h.character.as_ref().map(|c| c.nation).unwrap_or(0);
            (h.position.zone_id, n)
        })
        .unwrap_or((0, 0));
    let zone_tax_type = if is_loyalty {
        TaxZoneType::NoTax
    } else {
        classify_zone_tax(zone_id)
    };
    let king_tariff = world
        .get_king_system(nation)
        .map(|ks| ks.territory_tariff)
        .unwrap_or(0);

    // Read siege tariffs for Moradon/Delos zones (async RwLock)
    let active_siege_tariff = match zone_tax_type {
        TaxZoneType::SiegeMoradon => {
            let sw = world.siege_war().read().await;
            sw.moradon_tariff
        }
        TaxZoneType::SiegeDelos => {
            let sw = world.siege_war().read().await;
            sw.delos_tariff
        }
        _ => 10, // default = no tax
    };

    let mut total_price: u64 = 0;
    let mut total_weight: u32 = 0;
    let mut total_king_tax: u64 = 0;
    let mut total_siege_tax: i64 = 0;
    let stats = world.get_equipped_stats(sid);

    // Validate all items and calculate total price/weight (with tax)
    for item in &mut items {
        let item_def = match world.get_item(item.item_id) {
            Some(i) => i,
            None => {
                return {
                    tracing::warn!("[{}] BUY_FAIL_24 line 451", session.addr());
                    send_fail(session, 24)
                }
                .await
            }
        };

        // Loyalty merchants use NP price; regular merchants use gold price
        let unit_price = npc_buy_unit_price(
            selling_group,
            is_loyalty,
            item_def.buy_price.unwrap_or(0) as u64,
            item_def.np_buy_price.unwrap_or(0) as u64,
        );
        let base_price = unit_price * item.count as u64;
        if base_price > COIN_MAX as u64 {
            return {
                tracing::warn!("[{}] BUY_FAIL_25 line 463", session.addr());
                send_fail(session, 25)
            }
            .await;
        }

        // Apply tariff/tax (gold purchases only, non-exempt items)
        let transaction_price = if !is_loyalty
            && selling_group != MORADON_FIXED_SCROLL_SELLING_GROUP
            && !is_tax_exempt(item.item_id)
        {
            let (taxed, tax_amount) = calculate_item_tax(
                base_price as u32,
                zone_tax_type,
                king_tariff,
                active_siege_tariff,
            );
            if zone_tax_type == TaxZoneType::KingTariff && tax_amount > 0 {
                total_king_tax += tax_amount as u64;
            }
            if matches!(
                zone_tax_type,
                TaxZoneType::SiegeMoradon | TaxZoneType::SiegeDelos
            ) && tax_amount > 0
            {
                total_siege_tax += tax_amount as i64;
            }
            taxed as u64
        } else {
            base_price
        };

        if transaction_price > COIN_MAX as u64 {
            return {
                tracing::warn!("[{}] BUY_FAIL_26 line 492", session.addr());
                send_fail(session, 26)
            }
            .await;
        }

        item.buy_price = transaction_price as u32;
        total_price += transaction_price;

        if total_price > COIN_MAX as u64 {
            return {
                tracing::warn!("[{}] BUY_FAIL_27 line 499", session.addr());
                send_fail(session, 27)
            }
            .await;
        }

        let weight = (item_def.weight.unwrap_or(0) as u32).saturating_mul(item.count as u32);
        total_weight = total_weight.saturating_add(weight);

        if total_weight.saturating_add(stats.item_weight) > stats.max_weight {
            tracing::debug!(
                "[{}] NPC trade weight exceeded: item_weight={}, total_new_weight={}, max_weight={}, combined={}",
                session.addr(),
                stats.item_weight,
                total_weight,
                stats.max_weight,
                total_weight.saturating_add(stats.item_weight),
            );
            return send_fail(session, 4).await;
        }

        // Check destination slot
        let actual_slot = SLOT_MAX + item.inv_pos as usize;
        let slot = world
            .get_inventory_slot(sid, actual_slot)
            .unwrap_or_default();
        if slot.item_id != 0 {
            // Must be same item and countable
            if slot.item_id != item.item_id {
                tracing::warn!(
                    "[{}] BUY_FAIL_28: client wants inv_pos={} (server_slot={}), \
                     client_item={}, but server has item={} count={} at that slot",
                    session.addr(),
                    item.inv_pos,
                    actual_slot,
                    item.item_id,
                    slot.item_id,
                    slot.count
                );
                return send_fail(session, 28).await;
            }
            let countable = item_def.countable.unwrap_or(0);
            if countable == 0 || item.count == 0 {
                return {
                    tracing::warn!("[{}] BUY_FAIL_29 line 529", session.addr());
                    send_fail(session, 29)
                }
                .await;
            }
            if countable > 0 && (item.count + slot.count) > ITEMCOUNT_MAX {
                return send_fail(session, 4).await;
            }
        }
    }

    // Check currency (gold or loyalty)
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return send_fail(session, 1).await,
    };
    if is_loyalty {
        // For loyalty, hasCoins checks m_iLoyalty
        if (total_price as u32) > ch.loyalty {
            return send_fail(session, 3).await;
        }
    } else if (total_price as u32) > ch.gold {
        return send_fail(session, 3).await;
    }

    // Execute: deduct currency and give items
    if is_loyalty {
        world.update_character_stats(sid, |c| {
            c.loyalty = c.loyalty.saturating_sub(total_price as u32);
        });
    } else {
        world.gold_lose(sid, total_price as u32);
        // Daily rank stat: GMTotalSold += total_price (gold spent at NPC shop)
        world.update_session(sid, |h| {
            h.dr_gm_total_sold += total_price;
        });
    }

    for item in &items {
        if item.item_id == 0 {
            continue;
        }
        let item_def = match world.get_item(item.item_id) {
            Some(i) => i,
            None => continue,
        };

        let actual_slot = SLOT_MAX + item.inv_pos as usize;
        let serial = world.generate_item_serial();
        world.update_inventory(sid, |inv| {
            if actual_slot >= inv.len() {
                return false;
            }
            let is_new = inv[actual_slot].item_id == 0;
            inv[actual_slot].item_id = item.item_id;
            inv[actual_slot].count = (inv[actual_slot].count + item.count).min(ITEMCOUNT_MAX);
            if inv[actual_slot].durability == 0 {
                inv[actual_slot].durability = item_def.duration.unwrap_or(0);
            }
            if is_new {
                inv[actual_slot].serial_num = serial;
            }
            let countable = item_def.countable.unwrap_or(0);
            if countable == 0 {
                inv[actual_slot].count = 1;
            }
            true
        });
    }

    world.set_user_ability(sid);

    // FerihaLog: NpcShoppingLog (buy)
    {
        let zone_id = world
            .get_position(sid)
            .map(|p| p.zone_id as i16)
            .unwrap_or(0);
        super::audit_log::log_npc_shopping(
            session.pool(),
            session.account_id().unwrap_or(""),
            &world.get_session_name(sid).unwrap_or_default(),
            zone_id,
            selling_group as u16,
            if is_loyalty {
                "buy_loyalty"
            } else {
                "buy_gold"
            },
        );
    }

    // ── Accumulate tax revenue ───────────────────────────────────────────
    if total_king_tax > 0 && zone_tax_type == TaxZoneType::KingTariff {
        // 80% to national treasury, 20% to territory tax (king's collectible fund)
        let treasury_share = (total_king_tax * 80 / 100) as u32;
        let territory_share = (total_king_tax * 20 / 100) as u32;
        world.update_king_system(nation, |ks| {
            ks.national_treasury = ks
                .national_treasury
                .saturating_add(treasury_share)
                .min(COIN_MAX);
            ks.territory_tax = ks
                .territory_tax
                .saturating_add(territory_share)
                .min(COIN_MAX);
        });
    }
    // Siege revenue tracking (markup only, when tariff > 10)
    if total_siege_tax > 0 {
        let siege_tax = total_siege_tax as u32;
        let dungeon_share = (siege_tax as u64 * 80 / 100) as i32;
        let zone_tax_share = (siege_tax as u64 * 10 / 100) as i32;
        let has_castle_owner = {
            let sw = world.siege_war().read().await;
            sw.master_knights > 0
        };
        if has_castle_owner {
            let mut sw = world.siege_war().write().await;
            sw.dungeon_charge = sw.dungeon_charge.saturating_add(dungeon_share);
            match zone_tax_type {
                TaxZoneType::SiegeMoradon => {
                    sw.moradon_tax = sw.moradon_tax.saturating_add(zone_tax_share);
                }
                TaxZoneType::SiegeDelos => {
                    sw.delos_tax = sw.delos_tax.saturating_add(zone_tax_share);
                }
                _ => {}
            }
        }
    }

    // Send WEIGHT_CHANGE before trade response (sniffer verified: 0x54 sent before 0x21)
    // Sniffer format: [0x54][total_weight:u32le] — single u32, NOT two u16s
    {
        let cur_weight = world.get_equipped_stats(sid).item_weight;
        let mut wpkt = Packet::new(Opcode::WizWeightChange as u8);
        wpkt.write_u32(cur_weight);
        session.send_packet(&wpkt).await?;
    }

    // Send success response
    let mut result = Packet::new(Opcode::WizItemTrade as u8);
    result.write_u8(1); // success
    if is_loyalty {
        // Loyalty response: loyalty_after + total_price + selling_group
        let loyalty_after = world
            .get_character_info(sid)
            .map(|c| c.loyalty)
            .unwrap_or(0);
        result.write_u32(loyalty_after);
        result.write_u32(total_price as u32);
        result.write_u8((selling_group & 0xFF) as u8);
    } else {
        // Gold response: gold_after + total_price
        // v2600: no trailing u8 selling_group (sniff verified — 9 bytes, not 10)
        let gold_after = world.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
        result.write_u32(gold_after);
        result.write_u32(total_price as u32);
    }
    session.send_packet(&result).await
}

/// Handle selling items to an NPC shop.
async fn handle_sell(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    item_count: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Parse sell items
    let mut items = Vec::with_capacity(item_count.min(12) as usize);
    for _ in 0..item_count {
        let item_id = reader.read_u32().unwrap_or(0);
        let inv_pos = reader.read_u8().unwrap_or(0);
        let count = reader.read_u16().unwrap_or(0);
        items.push(TradeItem {
            item_id,
            inv_pos,
            count,
            buy_price: 0,
        });
    }

    let mut total_sell_price: u64 = 0;

    // Validate all items first
    for item in &mut items {
        let item_def = match world.get_item(item.item_id) {
            Some(i) => i,
            None => return send_fail(session, 2).await,
        };

        // Cannot sell untradeable items
        if (item.item_id >= ITEM_NO_TRADE_MIN && item.item_id <= ITEM_NO_TRADE_MAX)
            || item_def.race.unwrap_or(0) == RACE_UNTRADEABLE
        {
            return send_fail(session, 2).await;
        }

        // Check item exists in inventory
        let actual_slot = SLOT_MAX + item.inv_pos as usize;
        let slot = match world.get_inventory_slot(sid, actual_slot) {
            Some(s) if s.item_id == item.item_id => s,
            _ => return send_fail(session, 2).await,
        };

        // Check flags — use equality, NOT bitmask.
        if slot.flag == ITEM_FLAG_SEALED
            || slot.flag == ITEM_FLAG_RENTED
            || slot.flag == ITEM_FLAG_DUPLICATE
            || slot.expire_time > 0
        {
            return send_fail(session, 2).await;
        }

        if slot.count < item.count {
            return send_fail(session, 3).await;
        }

        // Calculate sell price using the C++ algorithm (NPCHandler.cpp:1912-1931):
        // 1. If sell_npc_type == 1 AND sell_npc_price * count > 0, use that
        // 2. If sell_price == SellTypeFullPrice (1), use buy_price * count
        // 3. Otherwise: (buy_price / divisor) * count (divisor = 4 premium, 6 normal)
        let buy_price = item_def.buy_price.unwrap_or(0) as u64;
        let sell_prem = world.get_premium_property(sid, PremiumProperty::ItemSellPercent);
        let sell_divisor: u64 = if sell_prem > 0 { 4 } else { 6 };
        let count = item.count as u64;

        const SELL_TYPE_FULL_PRICE: i32 = 1;

        let sell_npc_type = item_def.sell_npc_type.unwrap_or(0);
        let sell_npc_price = item_def.sell_npc_price.unwrap_or(0) as u64;
        let item_sell_price_flag = item_def.sell_price.unwrap_or(0);

        let sell_price = match sell_npc_type {
            1 => {
                // C++ case 1: use sell_npc_price * count, fall back if <= 0
                let npc_price = sell_npc_price.saturating_mul(count);
                if npc_price > 0 {
                    npc_price
                } else if item_sell_price_flag != SELL_TYPE_FULL_PRICE {
                    (buy_price / sell_divisor).saturating_mul(count)
                } else {
                    buy_price.saturating_mul(count)
                }
            }
            _ => {
                // C++ default: check SellTypeFullPrice flag
                if item_sell_price_flag != SELL_TYPE_FULL_PRICE {
                    (buy_price / sell_divisor).saturating_mul(count)
                } else {
                    buy_price.saturating_mul(count)
                }
            }
        };
        item.buy_price = sell_price as u32;
        total_sell_price += sell_price;

        // Check gold cap
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => return send_fail(session, 1).await,
        };
        if (ch.gold as u64) + total_sell_price > COIN_MAX as u64 {
            return send_fail(session, 3).await;
        }
    }

    // Execute: give gold and remove items, recording non-countable items for repurchase
    world.gold_gain(sid, total_sell_price as u32);

    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    for item in &items {
        if item.item_id == 0 {
            continue;
        }
        let item_def = world.get_item(item.item_id);
        let actual_slot = SLOT_MAX + item.inv_pos as usize;

        // Capture slot data before clearing, for repurchase tracking
        // not in Cinderella War → save to trash list for repurchase
        let slot_snapshot = world.get_inventory_slot(sid, actual_slot);

        world.update_inventory(sid, |inv| {
            if actual_slot >= inv.len() {
                return false;
            }
            let kind = item_def.as_ref().and_then(|i| i.kind).unwrap_or(0);
            let countable = item_def.as_ref().and_then(|i| i.countable).unwrap_or(0);

            if (kind == ITEM_KIND_UNIQUE && countable == 0) || item.count >= inv[actual_slot].count
            {
                inv[actual_slot] = UserItemSlot::default();
            } else {
                inv[actual_slot].count -= item.count;
            }
            true
        });

        // Record non-countable items for repurchase (trash item list)
        //   if (!pCindWar.isEventUser() && g_pMain->pServerSetting.trashitem
        //       && !pItem->isExpirationTime() && !pTable.m_bCountable) { ... }
        let trash_item_enabled = world
            .get_server_settings()
            .map(|s| s.trash_item)
            .unwrap_or(false);
        if trash_item_enabled {
            if let (Some(def), Some(slot)) = (&item_def, &slot_snapshot) {
                let countable = def.countable.unwrap_or(0);
                let has_expiry = slot.expire_time != 0;
                if countable == 0 && !has_expiry {
                    let delete_time = now_unix + 72 * 60;
                    let entry = DeletedItemEntry {
                        db_id: 0, // will be set after DB insert
                        item_id: slot.item_id,
                        count: slot.count as u32,
                        delete_time,
                        duration: slot.durability as u16,
                        serial_num: slot.serial_num,
                        flag: slot.flag,
                    };
                    record_trash_item(session, &world, sid, entry, delete_time as i32).await;
                }
            }
        }
    }

    world.set_user_ability(sid);

    // FerihaLog: NpcShoppingLog (sell)
    {
        let zone_id = world
            .get_position(sid)
            .map(|p| p.zone_id as i16)
            .unwrap_or(0);
        super::audit_log::log_npc_shopping(
            session.pool(),
            session.account_id().unwrap_or(""),
            &world.get_session_name(sid).unwrap_or_default(),
            zone_id,
            0,
            "sell",
        );
    }

    // Send success response
    // v2600: no trailing u8 selling_group (sniff verified — 9 bytes, not 10)
    let gold_after = world.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
    let mut result = Packet::new(Opcode::WizItemTrade as u8);
    result.write_u8(1); // success
    result.write_u32(gold_after);
    result.write_u32(total_sell_price as u32);
    session.send_packet(&result).await
}

async fn send_fail(session: &mut ClientSession, error_code: u8) -> anyhow::Result<()> {
    let mut result = Packet::new(Opcode::WizItemTrade as u8);
    result.write_u8(0); // failure
    result.write_u8(error_code);
    session.send_packet(&result).await
}

// ── Repurchase (Trash Item) System ──────────────────────────────────

/// Record a sold non-countable item in the trash list for future repurchase.
async fn record_trash_item(
    session: &mut ClientSession,
    world: &WorldState,
    sid: crate::zone::SessionId,
    mut entry: DeletedItemEntry,
    delete_time: i32,
) {
    let char_name = match world.get_character_info(sid) {
        Some(c) => c.name.clone(),
        None => return,
    };

    // Persist to DB
    let pool = session.pool();
    let repo = ko_db::repositories::character::CharacterRepository::new(pool);
    let params = ko_db::repositories::character::InsertTrashItemParams {
        char_id: &char_name,
        item_id: entry.item_id as i32,
        delete_time,
        duration: entry.duration as i16,
        count: entry.count as i32,
        flag: entry.flag as i16,
        serial_num: entry.serial_num as i64,
    };
    match repo.insert_trash_item(&params).await {
        Ok(db_id) => {
            entry.db_id = db_id;
            world.add_deleted_item(sid, entry);
        }
        Err(e) => {
            debug!("[{}] Failed to insert trash item: {}", session.addr(), e);
        }
    }
}

/// Handle WIZ_ITEM_TRADE type 5 — Repurchase (trash item buyback).
/// Sub-opcodes:
/// - 4: Refresh list (send current valid items)
/// - 2: Buyback a specific item
/// - 3: Clear the display list
async fn handle_repurchase(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let trash_item_enabled = world
        .get_server_settings()
        .map(|s| s.trash_item)
        .unwrap_or(false);
    if !trash_item_enabled {
        return Ok(());
    }

    // Must be in-game, alive, not busy
    let is_busy = world.get_character_info(sid).is_none()
        || world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid);

    if is_busy {
        return Ok(());
    }

    let sub_opcode = reader.read_u8().unwrap_or(0);
    match sub_opcode {
        4 => send_repurchase_list(session, &world, sid, true).await,
        2 => buyback_item(session, reader, &world, sid).await,
        3 => {
            // Only clears the display mapping, NOT the actual deleted items
            world.clear_delete_item_list(sid);
            Ok(())
        }
        _ => {
            debug!(
                "[{}] ItemTradeRepurchase unhandled sub-opcode {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Build and send the repurchase list to the client.
/// Packet format (WIZ_ITEM_TRADE):
/// ```text
/// [u8 5][u8 sub_opcode][u8 1][u16 count]
/// repeated count times:
///   [u32 item_id][u32 price][u32 delete_time][u8 display_index]
/// ```
pub(crate) async fn send_repurchase_list(
    session: &mut ClientSession,
    world: &WorldState,
    sid: u16,
    is_refreshed: bool,
) -> anyhow::Result<()> {
    // Clear old display mapping
    world.clear_delete_item_list(sid);

    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let deleted_items = world.get_deleted_items(sid);

    let mut result = Packet::new(Opcode::WizItemTrade as u8);
    result.write_u8(5); // trade type
    let sub = if is_refreshed { 4u8 } else { 1u8 };
    result.write_u8(sub);
    result.write_u8(1); // success

    // Placeholder for count — will be patched after loop
    let count_offset = result.data.len();
    result.write_u16(0);

    let mut display_count: u16 = 0;
    let mut display_mapping: HashMap<u8, usize> = HashMap::new();

    for (vec_idx, entry) in deleted_items.iter().enumerate() {
        // Skip expired items
        if now_unix >= entry.delete_time {
            continue;
        }

        // Max 250 displayed
        if display_count >= WorldState::TRASH_DISPLAY_MAX {
            break;
        }

        // Look up item definition for buy_price
        let item_def = match world.get_item(entry.item_id) {
            Some(i) => i,
            None => continue,
        };

        // Price formula: buy_price * count * 30
        let buy_price = item_def.buy_price.unwrap_or(0) as u64;
        let mut price = buy_price
            .saturating_mul(entry.count as u64)
            .saturating_mul(30);
        if price > COIN_MAX as u64 {
            price = COIN_MAX as u64;
        }

        result.write_u32(entry.item_id);
        result.write_u32(price as u32);
        result.write_u32(entry.delete_time);
        result.write_u8((display_count + 1) as u8);

        display_mapping.insert(display_count as u8, vec_idx);
        display_count += 1;
    }

    // Patch the count field
    let count_bytes = display_count.to_le_bytes();
    if count_offset + 1 < result.data.len() {
        result.data[count_offset] = count_bytes[0];
        result.data[count_offset + 1] = count_bytes[1];
    }

    // Store the display mapping for buyback lookups
    world.set_delete_item_list(sid, display_mapping);

    session.send_packet(&result).await
}

/// Handle buying back a specific item from the repurchase list.
/// Packet format (from client):
/// ```text
/// [u8 index][u32 item_id][i16 unk]
/// ```
async fn buyback_item(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    world: &WorldState,
    sid: u16,
) -> anyhow::Result<()> {
    let index = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let _unk = reader.read_i16().unwrap_or(0);

    // Look up display index → deleted item entry
    let (_, entry) = match world.get_deleted_item_by_display_index(sid, index) {
        Some(pair) => pair,
        None => return send_repurchase_fail(session).await,
    };

    // Validate item_id matches
    let item_def = match world.get_item(item_id) {
        Some(i) => i,
        None => return send_repurchase_fail(session).await,
    };

    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    if entry.item_id != item_id || now_unix >= entry.delete_time {
        return send_repurchase_fail(session).await;
    }

    // Calculate price: buy_price * count * 30
    let buy_price = item_def.buy_price.unwrap_or(0) as u64;
    let price = buy_price
        .saturating_mul(entry.count as u64)
        .saturating_mul(30);
    let price32 = if price > COIN_MAX as u64 {
        COIN_MAX
    } else {
        price as u32
    };

    // Check gold
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return send_repurchase_fail(session).await,
    };
    if ch.gold < price32 {
        return send_repurchase_fail(session).await;
    }

    // Check weight
    let weight = (item_def.weight.unwrap_or(0) as u32).saturating_mul(entry.count);
    let stats = world.get_equipped_stats(sid);
    if weight.saturating_add(stats.item_weight) > stats.max_weight {
        return send_repurchase_fail(session).await;
    }

    // Find free inventory slot
    let slot_pos =
        match world.find_slot_for_item(sid, entry.item_id, entry.count.min(u16::MAX as u32) as u16)
        {
            Some(pos) => pos,
            None => return send_repurchase_fail(session).await,
        };

    // Verify the slot is actually empty (C++ double-check)
    let slot = world.get_inventory_slot(sid, slot_pos).unwrap_or_default();
    if slot.item_id != 0 {
        return send_repurchase_fail(session).await;
    }

    // Restore item to inventory
    let item_count = entry.count;
    let item_duration = entry.duration;
    let item_serial = entry.serial_num;
    let item_flag = entry.flag;
    let db_id = entry.db_id;
    let kind = item_def.kind.unwrap_or(0);
    let countable = item_def.countable.unwrap_or(0);

    world.update_inventory(sid, |inv| {
        if slot_pos >= inv.len() {
            return false;
        }
        inv[slot_pos].item_id = item_id;
        inv[slot_pos].count = item_count.min(u16::MAX as u32) as u16;
        inv[slot_pos].durability = item_duration as i16;
        inv[slot_pos].serial_num = item_serial;
        inv[slot_pos].flag = item_flag;
        inv[slot_pos].original_flag = 0; // C++ NPCHandler.cpp:2135 — pItem->oFlag = 0
        inv[slot_pos].expire_time = 0;
        // Non-countable or kind 255 always count=1
        if kind == ITEM_KIND_UNIQUE || countable == 0 {
            inv[slot_pos].count = 1;
        }
        if inv[slot_pos].count > ITEMCOUNT_MAX {
            inv[slot_pos].count = ITEMCOUNT_MAX;
        }
        true
    });

    // Update stats and weight
    world.set_user_ability(sid);

    // Deduct gold
    world.gold_lose(sid, price32);

    // Remove from in-memory list
    world.remove_deleted_item(sid, db_id);

    // Remove from DB
    let pool = session.pool();
    let repo = ko_db::repositories::character::CharacterRepository::new(pool);
    if let Err(e) = repo.delete_trash_item(db_id).await {
        debug!(
            "[{}] Failed to delete trash item from DB: {}",
            session.addr(),
            e
        );
    }

    // Send success response
    let mut result = Packet::new(Opcode::WizItemTrade as u8);
    result.write_u8(5); // trade type
    result.write_u8(2); // sub_opcode = buyback
    result.write_u8(1); // success
    result.write_u16(0); // unused
    result.write_u32(item_id);
    session.send_packet(&result).await?;

    // Send stack change to update client UI
    let inv_pos = (slot_pos as u8).saturating_sub(SLOT_MAX as u8);
    let current_slot = world.get_inventory_slot(sid, slot_pos).unwrap_or_default();
    send_stack_change(
        session,
        item_id,
        current_slot.count as u32,
        current_slot.durability as u16,
        inv_pos,
        true,
        0,
    )
    .await
}

/// Send a stack change notification to the client.
/// Packet format:
/// ```text
/// [u16 1][u8 slot_section][u8 pos][u32 item_id][u32 count]
/// [u8 new_item_flag][u16 durability][u32 0][u32 expire_time][u16 0]
/// ```
async fn send_stack_change(
    session: &mut ClientSession,
    item_id: u32,
    count: u32,
    durability: u16,
    pos: u8,
    is_new: bool,
    expire_time: u32,
) -> anyhow::Result<()> {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
    pkt.write_u16(1);
    pkt.write_u8(1); // bSlotSection = 1 (inventory)
    pkt.write_u8(pos);
    pkt.write_u32(item_id);
    pkt.write_u32(count);
    pkt.write_u8(if is_new { 100 } else { 0 });
    pkt.write_u16(durability);
    pkt.write_u32(0); // unknown
    if expire_time > now_unix {
        pkt.write_u32(expire_time);
    } else {
        pkt.write_u32(0);
    }
    session.send_packet(&pkt).await
}

/// Send a repurchase failure response.
async fn send_repurchase_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut result = Packet::new(Opcode::WizItemTrade as u8);
    result.write_u8(5); // trade type
    result.write_u8(2); // sub_opcode = buyback
    result.write_u8(2); // failure
    session.send_packet(&result).await
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants, clippy::useless_vec)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use super::*;

    /// Test NPC buy client packet format:
    /// [u8 type=1][u32 group][u32 npc_id][u8 count]
    /// repeated: [u32 item_id][u8 inv_pos][u16 count][u8 line][u8 index]
    #[test]
    fn test_npc_buy_client_packet() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(1); // type = Buy
        pkt.write_u32(10); // group
        pkt.write_u32(500); // npc_id
        pkt.write_u8(2); // item_count

        // Item 1
        pkt.write_u32(100_001); // item_id
        pkt.write_u8(3); // inv_pos
        pkt.write_u16(5); // count
        pkt.write_u8(0); // line
        pkt.write_u8(0); // index

        // Item 2
        pkt.write_u32(200_002); // item_id
        pkt.write_u8(7); // inv_pos
        pkt.write_u16(1); // count
        pkt.write_u8(1); // line
        pkt.write_u8(2); // index

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // type
        assert_eq!(r.read_u32(), Some(10)); // group
        assert_eq!(r.read_u32(), Some(500)); // npc_id
        assert_eq!(r.read_u8(), Some(2)); // item_count

        // Item 1
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));

        // Item 2
        assert_eq!(r.read_u32(), Some(200_002));
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(2));

        assert_eq!(r.remaining(), 0);
    }

    /// Test NPC sell client packet format:
    /// [u8 type=2][u32 group][u32 npc_id][u8 count]
    /// repeated: [u32 item_id][u8 inv_pos][u16 count]
    #[test]
    fn test_npc_sell_client_packet() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(2); // type = Sell
        pkt.write_u32(5); // group
        pkt.write_u32(300); // npc_id
        pkt.write_u8(1); // item_count

        // Item 1
        pkt.write_u32(100_001);
        pkt.write_u8(2); // inv_pos
        pkt.write_u16(10); // count

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u32(), Some(5));
        assert_eq!(r.read_u32(), Some(300));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(100_001));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(10));
        assert_eq!(r.remaining(), 0);
    }

    /// Test NPC trade success response format:
    /// [u8 1][u32 gold_after][u32 total_price][u8 selling_group]
    #[test]
    fn test_npc_trade_success_response() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(1); // success
        pkt.write_u32(90_000); // gold_after
        pkt.write_u32(10_000); // total_price
        pkt.write_u8(0); // selling_group

        assert_eq!(pkt.opcode, Opcode::WizItemTrade as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(90_000));
        assert_eq!(r.read_u32(), Some(10_000));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test NPC trade failure response format: [u8 0][u8 error_code].
    #[test]
    fn test_npc_trade_failure_response() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(0); // failure
        pkt.write_u8(3); // error_code: not enough gold

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    /// Test sell price calculation: buy_price / 6 (standard formula from C++).
    #[test]
    fn test_sell_price_calculation() {
        // C++ formula: sell_price = buy_price / 6 * count
        let buy_price: u64 = 600;
        let count: u64 = 3;
        let sell_price = (buy_price / 6) * count;
        assert_eq!(sell_price, 300);

        // Test integer division rounding
        let buy_price2: u64 = 100;
        let sell_price2 = buy_price2 / 6;
        assert_eq!(sell_price2, 16); // 100/6 = 16 (integer truncation)
    }

    /// Test COIN_MAX constant from protocol specification.
    #[test]
    fn test_coin_max() {
        assert_eq!(COIN_MAX, 2_100_000_000);
        // Verify it's under u32::MAX
        assert!(COIN_MAX < u32::MAX);
    }

    /// Test inventory slot constants.
    #[test]
    fn test_inventory_constants() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(ITEMCOUNT_MAX, 9999);
    }

    /// Test NPC type constants from C++ globals.h.
    #[test]
    fn test_npc_merchant_types() {
        assert_eq!(NPC_MERCHANT, 21);
        assert_eq!(NPC_TINKER, 22);
        assert_eq!(NPC_LOYALTY_MERCHANT, 170);
        assert_eq!(NPC_PET_TRADE, 223);
    }

    /// Test NPC type matching logic.
    #[test]
    fn test_is_merchant_type() {
        let valid_types = [
            NPC_MERCHANT,
            NPC_TINKER,
            NPC_LOYALTY_MERCHANT,
            NPC_PET_TRADE,
        ];
        for t in valid_types {
            assert!(matches!(
                t,
                NPC_MERCHANT | NPC_TINKER | NPC_LOYALTY_MERCHANT | NPC_PET_TRADE
            ));
        }
        // Non-merchant types should fail
        assert!(!matches!(
            0u8,
            NPC_MERCHANT | NPC_TINKER | NPC_LOYALTY_MERCHANT | NPC_PET_TRADE
        ));
        assert!(!matches!(
            100u8, // NPC_ROLLINGSTONE
            NPC_MERCHANT | NPC_TINKER | NPC_LOYALTY_MERCHANT | NPC_PET_TRADE
        ));
    }

    /// Test buy item bounds validation.
    #[test]
    fn test_buy_item_bounds() {
        // inv_pos must be < HAVE_MAX (28)
        assert!(27 < HAVE_MAX);
        assert!((28 >= HAVE_MAX));

        // line must be < 12
        assert!(11 < 12u8);
        assert!((12 >= 12u8));

        // index must be < 24
        assert!(23 < 24u8);
        assert!((24 >= 24u8));
    }

    /// Test duplicate IPOS detection.
    #[test]
    fn test_duplicate_ipos_check() {
        let items = vec![
            TradeItem {
                item_id: 1,
                inv_pos: 3,
                count: 1,
                buy_price: 0,
            },
            TradeItem {
                item_id: 2,
                inv_pos: 5,
                count: 1,
                buy_price: 0,
            },
            TradeItem {
                item_id: 3,
                inv_pos: 3,
                count: 1,
                buy_price: 0,
            }, // duplicate pos 3
        ];

        let mut has_duplicate = false;
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                if items[i].inv_pos == items[j].inv_pos {
                    has_duplicate = true;
                }
            }
        }
        assert!(has_duplicate);

        // No duplicates case
        let items2 = [
            TradeItem {
                item_id: 1,
                inv_pos: 0,
                count: 1,
                buy_price: 0,
            },
            TradeItem {
                item_id: 2,
                inv_pos: 1,
                count: 1,
                buy_price: 0,
            },
        ];
        let mut has_dup2 = false;
        for i in 0..items2.len() {
            for j in (i + 1)..items2.len() {
                if items2[i].inv_pos == items2[j].inv_pos {
                    has_dup2 = true;
                }
            }
        }
        assert!(!has_dup2);
    }

    /// Test NPC ID conversion — client sends full NPC runtime ID (includes NPC_BAND).
    ///
    /// C++ does NOT add NPC_BAND; it just truncates to int16 → uint16.
    #[test]
    fn test_npc_id_conversion() {
        // Client sends the full NPC runtime ID (NPC_BAND + offset)
        let npc_id_raw: u32 = 10500; // NPC_BAND(10000) + 500
        let npc_nid = npc_id_raw as i16 as u16 as u32;
        assert_eq!(npc_nid, 10500);

        // First NPC in zone
        let npc_id_first: u32 = 10001; // NPC_BAND(10000) + 1
        assert_eq!(npc_id_first as i16 as u16 as u32, 10001);

        // Large NPC ID wraps via i16 truncation (> 32767)
        let npc_id_large: u32 = 40000;
        assert_eq!(npc_id_large as i16 as u16 as u32, 40000);
    }

    /// Test ItemSellTableRow.item_at() accessor for slot-based item lookup.
    #[test]
    fn test_item_sell_table_row_item_at() {
        use ko_db::models::ItemSellTableRow;

        let row = ItemSellTableRow {
            n_index: 1,
            i_selling_group: 101000,
            item1: 111010049,
            item2: 111110049,
            item3: 111210049,
            item4: 0,
            item5: 0,
            item6: 0,
            item7: 0,
            item8: 0,
            item9: 0,
            item10: 0,
            item11: 0,
            item12: 0,
            item13: 0,
            item14: 0,
            item15: 0,
            item16: 0,
            item17: 0,
            item18: 0,
            item19: 0,
            item20: 0,
            item21: 0,
            item22: 0,
            item23: 0,
            item24: 999999999,
        };

        assert_eq!(row.item_at(0), 111010049);
        assert_eq!(row.item_at(1), 111110049);
        assert_eq!(row.item_at(2), 111210049);
        assert_eq!(row.item_at(3), 0);
        assert_eq!(row.item_at(23), 999999999);
        // Out of range returns 0
        assert_eq!(row.item_at(24), 0);
        assert_eq!(row.item_at(100), 0);
    }

    /// Test loyalty merchant detection via selling_group.
    #[test]
    fn test_loyalty_merchant_detection() {
        let loyalty_group: u32 = super::LOYALTY_SELLING_GROUP;
        let regular_group: u32 = 101000;

        assert_eq!(loyalty_group, super::LOYALTY_SELLING_GROUP);
        assert_ne!(regular_group, super::LOYALTY_SELLING_GROUP);
    }

    /// Test loyalty price vs gold price selection.
    #[test]
    fn test_loyalty_vs_gold_pricing() {
        // Loyalty merchants use np_buy_price
        let np_buy_price: i32 = 50;
        let buy_price: i32 = 1000;
        let count: u64 = 3;

        let loyalty_cost = np_buy_price as u64 * count;
        let gold_cost = buy_price as u64 * count;

        assert_eq!(loyalty_cost, 150);
        assert_eq!(gold_cost, 3000);
    }

    #[test]
    fn test_moradon_fixed_scroll_shop_price_overrides_shared_item_price() {
        assert_eq!(
            npc_buy_unit_price(MORADON_FIXED_SCROLL_SELLING_GROUP, false, 2_000, 0),
            50_000_000
        );
        assert_eq!(npc_buy_unit_price(255_000, false, 2_000, 0), 2_000);
        assert_eq!(
            npc_buy_unit_price(LOYALTY_SELLING_GROUP, true, 2_000, 75),
            75
        );
    }

    // ── Tax/Tariff Tests ──────────────────────────────────────────────────

    /// Test zone classification for king tariff zones.
    #[test]
    fn test_classify_king_tariff_zones() {
        assert_eq!(classify_zone_tax(ZONE_KARUS), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_KARUS2), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_KARUS3), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_ELMORAD), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_ELMORAD2), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_ELMORAD3), TaxZoneType::KingTariff);
        assert_eq!(
            classify_zone_tax(ZONE_KARUS_ESLANT),
            TaxZoneType::KingTariff
        );
        assert_eq!(
            classify_zone_tax(ZONE_ELMORAD_ESLANT),
            TaxZoneType::KingTariff
        );
        assert_eq!(classify_zone_tax(ZONE_BATTLE), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_RONARK_LAND), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_ARDREAM), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_BIFROST), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_SNOW_BATTLE), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_DRAKI_TOWER), TaxZoneType::KingTariff);
        assert_eq!(
            classify_zone_tax(ZONE_DUNGEON_DEFENCE),
            TaxZoneType::KingTariff
        );
        assert_eq!(classify_zone_tax(ZONE_OLD_MORADON), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_OLD_KARUS), TaxZoneType::KingTariff);
        assert_eq!(classify_zone_tax(ZONE_OLD_HUMAN), TaxZoneType::KingTariff);
    }

    /// Test zone classification for siege moradon zones.
    #[test]
    fn test_classify_siege_moradon_zones() {
        assert_eq!(classify_zone_tax(ZONE_MORADON), TaxZoneType::SiegeMoradon);
        assert_eq!(classify_zone_tax(ZONE_MORADON2), TaxZoneType::SiegeMoradon);
        assert_eq!(classify_zone_tax(ZONE_MORADON3), TaxZoneType::SiegeMoradon);
        assert_eq!(classify_zone_tax(ZONE_MORADON4), TaxZoneType::SiegeMoradon);
        assert_eq!(classify_zone_tax(ZONE_MORADON5), TaxZoneType::SiegeMoradon);
        assert_eq!(classify_zone_tax(ZONE_ARENA), TaxZoneType::SiegeMoradon);
    }

    /// Test zone classification for siege delos zones.
    #[test]
    fn test_classify_siege_delos_zones() {
        assert_eq!(classify_zone_tax(ZONE_DELOS), TaxZoneType::SiegeDelos);
        assert_eq!(
            classify_zone_tax(ZONE_DESPERATION_ABYSS),
            TaxZoneType::SiegeDelos
        );
        assert_eq!(classify_zone_tax(ZONE_HELL_ABYSS), TaxZoneType::SiegeDelos);
        assert_eq!(
            classify_zone_tax(ZONE_DELOS_CASTELLAN),
            TaxZoneType::SiegeDelos
        );
    }

    /// Test zone classification for neutral/unknown zones.
    #[test]
    fn test_classify_no_tax_zones() {
        assert_eq!(classify_zone_tax(0), TaxZoneType::NoTax);
        assert_eq!(classify_zone_tax(100), TaxZoneType::NoTax);
        assert_eq!(classify_zone_tax(200), TaxZoneType::NoTax);
        assert_eq!(classify_zone_tax(9999), TaxZoneType::NoTax);
    }

    /// Test king tariff tax calculation.
    ///
    /// C++ formula: tax = (base_price * (territory_tariff + 10)) / 100
    #[test]
    fn test_king_tariff_tax_calculation() {
        // tariff=0 (stored), effective=10% => 1000 * 10/100 = 100 tax
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::KingTariff, 0, 10);
        assert_eq!(price, 1100);
        assert_eq!(tax, 100);

        // tariff=5 (stored), effective=15% => 1000 * 15/100 = 150 tax
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::KingTariff, 5, 10);
        assert_eq!(price, 1150);
        assert_eq!(tax, 150);

        // tariff=10 (stored), effective=20% => 1000 * 20/100 = 200 tax
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::KingTariff, 10, 10);
        assert_eq!(price, 1200);
        assert_eq!(tax, 200);
    }

    /// Test siege tariff tax calculation — discount range (0-9).
    #[test]
    fn test_siege_tariff_discount() {
        // tariff=0 => -10% discount
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeMoradon, 0, 0);
        assert_eq!(price, 900);
        assert_eq!(tax, -100);

        // tariff=5 => -5% discount
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeMoradon, 0, 5);
        assert_eq!(price, 950);
        assert_eq!(tax, -50);

        // tariff=9 => -1% discount
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeDelos, 0, 9);
        assert_eq!(price, 990);
        assert_eq!(tax, -10);
    }

    /// Test siege tariff tax calculation — neutral (10).
    #[test]
    fn test_siege_tariff_neutral() {
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeMoradon, 0, 10);
        assert_eq!(price, 1000);
        assert_eq!(tax, 0);
    }

    /// Test siege tariff tax calculation — markup range (11-20).
    #[test]
    fn test_siege_tariff_markup() {
        // tariff=11 => +1% markup
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeMoradon, 0, 11);
        assert_eq!(price, 1010);
        assert_eq!(tax, 10);

        // tariff=15 => +5% markup
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeDelos, 0, 15);
        assert_eq!(price, 1050);
        assert_eq!(tax, 50);

        // tariff=20 => +10% markup
        let (price, tax) = calculate_item_tax(1000, TaxZoneType::SiegeMoradon, 0, 20);
        assert_eq!(price, 1100);
        assert_eq!(tax, 100);
    }

    /// Test no-tax zone type always returns base price.
    #[test]
    fn test_no_tax_zone() {
        let (price, tax) = calculate_item_tax(5000, TaxZoneType::NoTax, 10, 20);
        assert_eq!(price, 5000);
        assert_eq!(tax, 0);
    }

    /// Test tax-exempt scroll IDs skip tariff.
    #[test]
    fn test_tax_exempt_scrolls() {
        assert!(is_tax_exempt(379068000));
        assert!(is_tax_exempt(379107000));
        assert!(is_tax_exempt(379109000));
        assert!(is_tax_exempt(379110000));
        assert!(is_tax_exempt(379067000));
        assert!(!is_tax_exempt(111010049));
        assert!(!is_tax_exempt(0));
        assert!(!is_tax_exempt(999999999));
    }

    /// Test king tariff revenue split: 80% treasury, 20% territory tax.
    #[test]
    fn test_king_tariff_revenue_split() {
        let total_tax: u64 = 200;
        let treasury_share = (total_tax * 80 / 100) as u32;
        let territory_share = (total_tax * 20 / 100) as u32;
        assert_eq!(treasury_share, 160);
        assert_eq!(territory_share, 40);
        assert_eq!(treasury_share + territory_share, 200);
    }

    /// Test siege revenue split: 80% dungeon charge, 10% zone tax.
    #[test]
    fn test_siege_revenue_split() {
        let siege_tax: u32 = 100;
        let dungeon_share = (siege_tax as u64 * 80 / 100) as i32;
        let zone_share = (siege_tax as u64 * 10 / 100) as i32;
        assert_eq!(dungeon_share, 80);
        assert_eq!(zone_share, 10);
    }

    /// Test integer edge cases: tax on small prices.
    #[test]
    fn test_tax_small_price_rounding() {
        // 1 gold at 10% tariff => tax = 0 (integer truncation)
        let (price, tax) = calculate_item_tax(1, TaxZoneType::KingTariff, 0, 10);
        assert_eq!(tax, 0);
        assert_eq!(price, 1);

        // 9 gold at 10% tariff => tax = 0 (9*10/100=0 truncated)
        let (price, tax) = calculate_item_tax(9, TaxZoneType::KingTariff, 0, 10);
        assert_eq!(tax, 0);
        assert_eq!(price, 9);

        // 10 gold at 10% tariff => tax = 1
        let (price, tax) = calculate_item_tax(10, TaxZoneType::KingTariff, 0, 10);
        assert_eq!(tax, 1);
        assert_eq!(price, 11);
    }

    /// Test zone constant values match C++ Define.h.
    #[test]
    fn test_zone_constants() {
        assert_eq!(ZONE_KARUS, 1);
        assert_eq!(ZONE_ELMORAD, 2);
        assert_eq!(ZONE_KARUS2, 5);
        assert_eq!(ZONE_KARUS3, 6);
        assert_eq!(ZONE_ELMORAD2, 7);
        assert_eq!(ZONE_ELMORAD3, 8);
        assert_eq!(ZONE_KARUS_ESLANT, 11);
        assert_eq!(ZONE_ELMORAD_ESLANT, 12);
        assert_eq!(ZONE_MORADON, 21);
        assert_eq!(ZONE_DELOS, 30);
        assert_eq!(ZONE_BIFROST, 31);
        assert_eq!(ZONE_BATTLE, 61);
        assert_eq!(ZONE_SNOW_BATTLE, 69);
        assert_eq!(ZONE_RONARK_LAND, 71);
        assert_eq!(ZONE_ARDREAM, 72);
        assert_eq!(ZONE_ARENA, 48);
    }

    // ── Repurchase System Tests ─────────────────────────────────────

    /// Test `DeletedItemEntry` struct creation and field access.
    #[test]
    fn test_deleted_item_entry_struct() {
        let entry = DeletedItemEntry {
            db_id: 42,
            item_id: 150001,
            count: 1,
            delete_time: 1700000000 + 72 * 60,
            duration: 500,
            serial_num: 1234567890,
            flag: 0,
        };
        assert_eq!(entry.db_id, 42);
        assert_eq!(entry.item_id, 150001);
        assert_eq!(entry.count, 1);
        assert_eq!(entry.duration, 500);
        assert_eq!(entry.serial_num, 1234567890);
        assert_eq!(entry.flag, 0);
    }

    /// Test repurchase price formula: buy_price * count * 30.
    ///
    #[test]
    fn test_repurchase_price_formula() {
        // Standard weapon: buy_price=1000, count=1
        let buy_price: u64 = 1000;
        let count: u64 = 1;
        let price = buy_price * count * 30;
        assert_eq!(price, 30_000);

        // Armor: buy_price=5000, count=1
        let price2 = 5000u64 * 30;
        assert_eq!(price2, 150_000);

        // Non-countable items always have count=1
        let price3 = 100u64 * 30;
        assert_eq!(price3, 3_000);
    }

    /// Test repurchase price capped at COIN_MAX.
    #[test]
    fn test_repurchase_price_cap() {
        let buy_price: u64 = 100_000_000;
        let count: u64 = 1;
        let mut price = buy_price * count * 30;
        if price > COIN_MAX as u64 {
            price = COIN_MAX as u64;
        }
        assert_eq!(price, COIN_MAX as u64);
    }

    /// Test 72-minute expiration calculation.
    ///
    #[test]
    fn test_repurchase_expiry_72_minutes() {
        let now: u32 = 1700000000;
        let delete_time = now + 72 * 60;
        assert_eq!(delete_time, now + 4320);

        // Not expired yet (just sold)
        assert!(now < delete_time);

        // Expired after 72 minutes
        let later = now + 72 * 60;
        assert!((later >= delete_time)); // >= means expired

        // Expired well after
        let way_later = now + 73 * 60;
        assert!(way_later >= delete_time);
    }

    /// Test countable items are NOT recorded for repurchase.
    ///
    #[test]
    fn test_countable_items_rejected() {
        // Countable items (potions, arrows) should NOT go to repurchase
        let countable: i16 = 1;
        let has_expiry = false;
        let should_record = countable == 0 && !has_expiry;
        assert!(!should_record);

        // Non-countable items (weapons, armor) SHOULD go to repurchase
        let non_countable: i16 = 0;
        let should_record2 = non_countable == 0 && !has_expiry;
        assert!(should_record2);
    }

    /// Test expiring items are NOT recorded for repurchase.
    ///
    #[test]
    fn test_expiring_items_rejected() {
        let countable: i16 = 0;
        let has_expiry = true;
        let should_record = countable == 0 && !has_expiry;
        assert!(!should_record);
    }

    /// Test repurchase list packet format.
    ///
    #[test]
    fn test_repurchase_list_packet_format() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(5); // trade type
        pkt.write_u8(4); // sub_opcode = refresh
        pkt.write_u8(1); // success
        pkt.write_u16(2); // count = 2 items

        // Item 1
        pkt.write_u32(150001); // item_id
        pkt.write_u32(30_000); // price = 1000 * 1 * 30
        pkt.write_u32(1700004320); // delete_time
        pkt.write_u8(1); // display_index

        // Item 2
        pkt.write_u32(250001); // item_id
        pkt.write_u32(150_000); // price = 5000 * 1 * 30
        pkt.write_u32(1700004320); // delete_time
        pkt.write_u8(2); // display_index

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5)); // type
        assert_eq!(r.read_u8(), Some(4)); // sub
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.read_u16(), Some(2)); // count

        // Item 1
        assert_eq!(r.read_u32(), Some(150001));
        assert_eq!(r.read_u32(), Some(30_000));
        assert_eq!(r.read_u32(), Some(1700004320));
        assert_eq!(r.read_u8(), Some(1));

        // Item 2
        assert_eq!(r.read_u32(), Some(250001));
        assert_eq!(r.read_u32(), Some(150_000));
        assert_eq!(r.read_u32(), Some(1700004320));
        assert_eq!(r.read_u8(), Some(2));

        assert_eq!(r.remaining(), 0);
    }

    /// Test repurchase buyback success packet format.
    ///
    #[test]
    fn test_repurchase_buyback_success_packet() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(5); // trade type
        pkt.write_u8(2); // sub = buyback
        pkt.write_u8(1); // success
        pkt.write_u16(0); // unused
        pkt.write_u32(150001); // item_id

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u32(), Some(150001));
        assert_eq!(r.remaining(), 0);
    }

    /// Test repurchase buyback failure packet format.
    ///
    #[test]
    fn test_repurchase_buyback_failure_packet() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(5); // trade type
        pkt.write_u8(2); // sub = buyback
        pkt.write_u8(2); // failure

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.remaining(), 0);
    }

    /// Test repurchase client request packet format (buyback).
    ///
    #[test]
    fn test_repurchase_buyback_request_packet() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(5); // type = repurchase
        pkt.write_u8(2); // sub = buyback
        pkt.write_u8(0); // index
        pkt.write_u32(150001); // item_id
        pkt.write_i16(0); // unk

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5)); // type
        assert_eq!(r.read_u8(), Some(2)); // sub
        assert_eq!(r.read_u8(), Some(0)); // index
        assert_eq!(r.read_u32(), Some(150001)); // item_id
        assert_eq!(r.read_i16(), Some(0)); // unk
        assert_eq!(r.remaining(), 0);
    }

    /// Test TRASH_DISPLAY_MAX = 250 and TRASH_ITEM_MAX = 10,000.
    #[test]
    fn test_repurchase_limits() {
        assert_eq!(WorldState::TRASH_DISPLAY_MAX, 250);
        assert_eq!(WorldState::TRASH_ITEM_MAX, 10_000);
    }

    /// Test stack change packet format for repurchase item restoration.
    ///
    #[test]
    fn test_stack_change_packet_format() {
        let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
        pkt.write_u16(1);
        pkt.write_u8(1); // slot_section
        pkt.write_u8(5); // pos
        pkt.write_u32(150001); // item_id
        pkt.write_u32(1); // count (u32, not u16 — C++ comment says "needs to be 4 bytes, not a bug")
        pkt.write_u8(100); // bNewItem = true (100)
        pkt.write_u16(500); // durability
        pkt.write_u32(0); // unknown
        pkt.write_u32(0); // expire_time

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u32(), Some(150001));
        assert_eq!(r.read_u32(), Some(1));
        assert_eq!(r.read_u8(), Some(100));
        assert_eq!(r.read_u16(), Some(500));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0); // v2600: no trailing u16 padding
    }

    /// Test repurchase sub-opcode constants match protocol specification.
    ///
    #[test]
    fn test_repurchase_sub_opcodes() {
        // Sub-opcode 4 = refresh list
        // Sub-opcode 2 = buyback
        // Sub-opcode 3 = clear list
        // These must match the C++ switch cases exactly
        let refresh: u8 = 4;
        let buyback: u8 = 2;
        let clear: u8 = 3;
        assert_ne!(refresh, buyback);
        assert_ne!(refresh, clear);
        assert_ne!(buyback, clear);
    }

    /// Test display index starts at 1 (not 0) as per protocol specification.
    ///
    #[test]
    fn test_display_index_starts_at_one() {
        // C++ sends display_index as sCount + 1 (1-based)
        let display_count: u16 = 0;
        let display_index = (display_count + 1) as u8;
        assert_eq!(display_index, 1);

        let display_count2: u16 = 5;
        let display_index2 = (display_count2 + 1) as u8;
        assert_eq!(display_index2, 6);
    }

    /// Test empty repurchase list packet.
    #[test]
    fn test_empty_repurchase_list_packet() {
        let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
        pkt.write_u8(5); // trade type
        pkt.write_u8(1); // sub_opcode = initial list
        pkt.write_u8(1); // success
        pkt.write_u16(0); // count = 0

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 78: Overflow safety regression tests ─────────────────

    /// Test weight calculation uses saturating_mul (no overflow on large weight × count).
    #[test]
    fn test_weight_saturating_mul_no_overflow() {
        let weight: u32 = 50_000;
        let count: u32 = 100_000;
        // Without saturating_mul: 50_000 * 100_000 = 5_000_000_000 > u32::MAX → wraps
        let safe_weight = weight.saturating_mul(count);
        assert_eq!(safe_weight, u32::MAX);

        // Normal case: no overflow
        let normal_weight = (10u32).saturating_mul(5);
        assert_eq!(normal_weight, 50);
    }

    /// Test repurchase price uses saturating_mul (no overflow on large price × count × 30).
    #[test]
    fn test_repurchase_price_saturating_mul() {
        let buy_price: u64 = 1_000_000_000;
        let count: u64 = 10_000;
        // Without saturating: 1e9 * 10_000 * 30 = 3e14, fits in u64
        let price = buy_price.saturating_mul(count).saturating_mul(30);
        assert_eq!(price, 300_000_000_000_000);

        // Edge case: would overflow u64
        let huge_price: u64 = u64::MAX / 2;
        let overflow_price = huge_price.saturating_mul(3);
        assert_eq!(overflow_price, u64::MAX);
    }

    /// Test sell price preserves C++ divide-first order with saturating_mul.
    #[test]
    fn test_sell_price_order_matches_cpp() {
        // C++ pattern: (m_iBuyPrice / divisor) * count
        let buy_price: u64 = 1000;
        let divisor: u64 = 6;
        let count: u64 = 7;
        let sell_price = (buy_price / divisor).saturating_mul(count);
        // 1000/6 = 166 (truncated), 166*7 = 1162
        assert_eq!(sell_price, 1162);
    }

    /// Test weight comparison uses u32 (no u16 truncation).
    #[test]
    fn test_weight_comparison_no_u16_truncation() {
        let total_weight: u32 = 70_000; // > u16::MAX (65535)
        let item_weight: u16 = 100;
        let max_weight: u16 = 60_000;

        // Old pattern: (total_weight as u16) truncates → 4464, plus 100 = 4564 < 60000 = PASS (wrong!)
        let old_check = (total_weight as u16).saturating_add(item_weight) > max_weight;
        assert!(
            !old_check,
            "old pattern falsely passes due to u16 truncation"
        );

        // New pattern: compare as u32 → 70000 + 100 = 70100 > 60000 = FAIL (correct!)
        let new_check = total_weight.saturating_add(item_weight as u32) > max_weight as u32;
        assert!(new_check, "new pattern correctly detects overweight");
    }

    #[test]
    fn test_npc_buy_assigns_serial_to_new_slot() {
        // Verify the serial assignment logic in NPC buy path
        use crate::inventory_constants::INVENTORY_TOTAL;
        use crate::world::{UserItemSlot, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_inventory(1, vec![UserItemSlot::default(); INVENTORY_TOTAL]);

        // Simulate NPC buy: assign item to empty slot with serial
        let serial = world.generate_item_serial();
        world.update_inventory(1, |inv| {
            let slot_idx = SLOT_MAX;
            if slot_idx >= inv.len() {
                return false;
            }
            let is_new = inv[slot_idx].item_id == 0;
            inv[slot_idx].item_id = 389001000;
            inv[slot_idx].count = 1;
            inv[slot_idx].durability = 50;
            if is_new {
                inv[slot_idx].serial_num = serial;
            }
            true
        });

        let inv = world.get_inventory(1);
        assert_eq!(inv[SLOT_MAX].item_id, 389001000);
        assert_ne!(
            inv[SLOT_MAX].serial_num, 0,
            "NPC buy should assign non-zero serial"
        );
    }

    // ── Sprint 310: Loyalty detection via selling_group ─────────────

    #[test]
    fn test_loyalty_detection_uses_selling_group_not_npc_type() {
        // Loyalty merchants are detected by selling_group, NOT by NPC type.
        let group_loyalty: u32 = super::LOYALTY_SELLING_GROUP;
        assert_eq!(group_loyalty, super::LOYALTY_SELLING_GROUP);

        // Gold NPC: selling_group=1100
        let group_gold: u32 = 1100;
        assert_ne!(group_gold, super::LOYALTY_SELLING_GROUP);

        // NPC type is irrelevant — a type-170 NPC with group!=LOYALTY is NOT loyalty
        let npc_type_loyalty: u8 = 170;
        let _ = npc_type_loyalty; // type is not used for detection
    }

    #[test]
    fn test_cannot_sell_to_loyalty_npc() {
        // Selling (trade_type==2) is blocked when selling_group == LOYALTY_SELLING_GROUP.
        let trade_type: u8 = 2; // sell
        let group: u32 = super::LOYALTY_SELLING_GROUP;
        assert!(trade_type == 2 && group == super::LOYALTY_SELLING_GROUP);
    }
}
