//! WIZ_CLAN_WAREHOUSE (0xD1) handler — clan (shared) warehouse.
//! Sub-opcodes (`ClanBankOpcodes` in `packets.h:716-723`):
//! - 1 = ClanBankOpen: Load + send all 192 clan warehouse slots
//! - 2 = ClanBankInput: Move item from inventory -> clan warehouse
//! - 3 = ClanBankOutput: Move item from clan warehouse -> inventory
//! - 4 = ClanBankMove: Rearrange items within clan warehouse
//! - 5 = ClanBankInventoryMove: Rearrange items within inventory (while bank open)
//! Access rules:
//! - Open: any clan member
//! - Input (deposit): any clan member
//! - Output (withdraw): clan leader or assistant only
//! - Move: clan leader or assistant only
//! - InventoryMove: clan leader or assistant only

use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use ko_db::repositories::character::{CharacterRepository, SaveItemParams};
use ko_db::repositories::clan_warehouse::{ClanWarehouseRepository, SaveClanWarehouseItemParams};
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use super::chat::build_chat_packet;
use crate::session::{ClientSession, SessionState};
use crate::world::{
    UserItemSlot, ITEMCOUNT_MAX, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED, ITEM_FLAG_SEALED,
    ITEM_GOLD, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN,
};

/// Clan warehouse sub-opcodes (`ClanBankOpcodes` in `packets.h:716-723`).
const CLAN_BANK_OPEN: u8 = 0x01;
const CLAN_BANK_INPUT: u8 = 0x02;
const CLAN_BANK_OUTPUT: u8 = 0x03;
const CLAN_BANK_MOVE: u8 = 0x04;
const CLAN_BANK_INVENTORY_MOVE: u8 = 0x05;

use super::{COIN_MAX, HAVE_MAX, SLOT_MAX};
use crate::inventory_constants::{ITEMS_PER_PAGE, WAREHOUSE_MAX};

// Item flag constants imported from crate::world (ITEM_FLAG_RENTED, ITEM_FLAG_DUPLICATE, ITEM_FLAG_SEALED).

use crate::clan_constants::{CHIEF, VICECHIEF};

/// In-memory clan warehouse cache: clan_id -> (items, gold).
/// Shared across all sessions. Loaded lazily from DB on first ClanBankOpen.
/// Modified in-memory by Input/Output/Move, persisted to DB asynchronously.
static CLAN_WAREHOUSES: LazyLock<DashMap<u16, ClanWarehouseData>> = LazyLock::new(DashMap::new);

/// Cached clan warehouse state.
#[derive(Debug, Clone)]
struct ClanWarehouseData {
    items: Vec<UserItemSlot>,
    gold: u32,
}

/// Build a clan warehouse response packet.
fn build_result(sub_opcode: u8, result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizClanWarehouse as u8);
    pkt.write_u8(sub_opcode);
    pkt.write_u8(result);
    pkt
}

/// The classic clan warehouse needs the player to stand next to a warehouse NPC; the native
/// window opened from the clan window button has no NPC.
fn npc_in_range(session: &ClientSession, npc_id: u16) -> bool {
    session.clan_wh_native()
        || session
            .world()
            .is_in_npc_range(session.session_id(), npc_id as u32)
}

/// Clan premium is required for the clan warehouse.
fn clan_has_premium(world: &crate::world::WorldState, sid: u16) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    world
        .get_character_info(sid)
        .filter(|c| c.knights_id > 0)
        .and_then(|c| world.get_knights(c.knights_id))
        .map(|k| k.premium_time > now || k.premium_in_use > 0)
        .unwrap_or(false)
}

/// "Clan warehouse" button of the clan window (hook packet): open the native warehouse window
/// showing the clan warehouse. Needs a clan and an active clan premium.
pub async fn open_native(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    if world.is_player_dead(sid) || world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }
    let in_clan = world
        .get_character_info(sid)
        .map(|c| c.knights_id > 0)
        .unwrap_or(false);
    if !in_clan {
        return session
            .send_packet(&crate::systems::timed_notice::build_notice_packet(
                8,
                "Clan kasasi icin bir clana uye olmalisin.",
            ))
            .await;
    }
    if !clan_has_premium(&world, sid) {
        return session
            .send_packet(&crate::systems::timed_notice::build_notice_packet(
                8,
                "Clan kasasi icin Clan Premium gerekli.",
            ))
            .await;
    }
    session.set_clan_wh_native(true);
    let mut open = Packet::new(Opcode::WizClanWarehouse as u8);
    open.data.push(CLAN_BANK_OPEN);
    open.data.extend_from_slice(&0u16.to_le_bytes());
    warn!("[{}] clan warehouse opened in the native window", session.addr());
    handle(session, open).await
}

/// A packet of the native warehouse window while it shows the clan warehouse:
/// `[op][u32 npc][...]` becomes the clan format `[op][u16 npc][...]`.
pub async fn handle_native(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut data = pkt.data;
    if data.len() >= 5 {
        data.drain(3..5); // u32 npc id -> u16
    } else if data.len() == 1 {
        data.extend_from_slice(&0u16.to_le_bytes());
    }
    let mut out = Packet::new(Opcode::WizClanWarehouse as u8);
    out.data = data;
    handle(session, out).await
}

/// Handle WIZ_CLAN_WAREHOUSE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Must be alive
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    // Genie state check — genie blocks clan warehouse.
    if world.with_session(sid, |h| h.genie_active).unwrap_or(false) {
        return session.send_packet(&build_result(sub_opcode, 0)).await;
    }

    // Clan premium gate
    //   if (g_pMain->pServerSetting.ClanBankWithPremium) {
    //     if (!isInClan() || !sClanPremStatus) → fail;
    //   }
    let clan_bank_premium = world
        .get_server_settings()
        .map(|s| s.clan_bank_premium != 0)
        .unwrap_or(false);
    if clan_bank_premium || session.clan_wh_native() {
        let clan_id = world.get_character_info(sid).and_then(|ch| {
            if ch.knights_id > 0 {
                Some(ch.knights_id)
            } else {
                None
            }
        });
        let has_premium = clan_id
            .and_then(|cid| world.get_knights(cid))
            .map(|k| k.premium_in_use > 0 || k.premium_time > now_secs())
            .unwrap_or(false);
        if !has_premium {
            return session.send_packet(&build_result(sub_opcode, 0)).await;
        }
    }

    match sub_opcode {
        CLAN_BANK_OPEN => handle_open(session, &mut reader).await,
        CLAN_BANK_INPUT => handle_input(session, &mut reader).await,
        CLAN_BANK_OUTPUT => handle_output(session, &mut reader).await,
        CLAN_BANK_MOVE => handle_move(session, &mut reader).await,
        CLAN_BANK_INVENTORY_MOVE => handle_inventory_move(session, &mut reader).await,
        _ => {
            warn!(
                "[{}] Unknown clan warehouse sub-opcode: 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

fn now_secs() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

/// Common validation: must be in-game, not dead, not trading, not merchanting,
/// not mining, not fishing.
fn validate_basic_state(session: &ClientSession) -> bool {
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return false;
    }
    true
}

/// Validate that player is in a clan and get their clan_id and fame.
/// Returns None if validation fails.
fn get_clan_info(session: &ClientSession) -> Option<(u16, u8)> {
    let world = session.world().clone();
    let sid = session.session_id();
    let ch = world.get_character_info(sid)?;
    if ch.knights_id == 0 {
        return None;
    }
    Some((ch.knights_id, ch.fame))
}

/// Check if the player is a clan leader or assistant (vice chief).
fn is_leader_or_assistant(fame: u8) -> bool {
    fame == CHIEF || fame == VICECHIEF
}

/// Load clan warehouse from DB into cache if not already loaded.
async fn ensure_loaded(session: &ClientSession, clan_id: u16) -> anyhow::Result<()> {
    if CLAN_WAREHOUSES.contains_key(&clan_id) {
        return Ok(());
    }

    let pool = session.pool().clone();
    let repo = ClanWarehouseRepository::new(&pool);
    let db_items = repo.load_items(clan_id as i16).await?;
    let gold = repo.load_gold(clan_id as i16).await?;

    let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];
    for row in &db_items {
        let idx = row.slot_index as usize;
        if idx < WAREHOUSE_MAX {
            items[idx] = UserItemSlot {
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

    CLAN_WAREHOUSES.insert(
        clan_id,
        ClanWarehouseData {
            items,
            gold: gold.max(0) as u32,
        },
    );

    Ok(())
}

/// Handle ClanBankOpen (sub-opcode 1).
/// Packet in: `[u8 sub=1] [u16 npc_id]`
/// Packet out (success): `[u8 sub=1] [u8 1] [u32 clan_gold]` + 192 items
/// Packet out (fail): `[u8 sub=1] [u8 0]`
async fn handle_open(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let npc_id = reader.read_u16().unwrap_or(0);

    // NPC range check — prevent remote clan warehouse access
    if !npc_in_range(session, npc_id) {
        return session.send_packet(&build_result(CLAN_BANK_OPEN, 0)).await;
    }

    if !validate_basic_state(session) {
        session
            .send_packet(&build_result(CLAN_BANK_OPEN, 0))
            .await?;
        return Ok(());
    }

    let (clan_id, _fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_OPEN, 0))
                .await?;
            return Ok(());
        }
    };

    // Verify clan exists in world state
    let world = session.world().clone();
    if world.get_knights(clan_id).is_none() {
        session
            .send_packet(&build_result(CLAN_BANK_OPEN, 0))
            .await?;
        return Ok(());
    }

    // Load from DB if needed
    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!("[{}] Failed to load clan warehouse: {}", session.addr(), e);
        session
            .send_packet(&build_result(CLAN_BANK_OPEN, 0))
            .await?;
        return Ok(());
    }

    // Read cached data
    let data = match CLAN_WAREHOUSES.get(&clan_id) {
        Some(d) => d.clone(),
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_OPEN, 0))
                .await?;
            return Ok(());
        }
    };

    // Build response: [u8 sub=1] [u8 result=1] [u32 gold] + 192 items
    let mut result = Packet::new(Opcode::WizClanWarehouse as u8);
    result.write_u8(CLAN_BANK_OPEN);
    result.write_u8(1); // success
    result.write_u32(data.gold);

    // C++ ClanBank.cpp:98-108 — write each item slot
    for i in 0..WAREHOUSE_MAX {
        let slot = data.items.get(i).cloned().unwrap_or_default();
        // Skip expired items
        let slot = if slot.expire_time != 0
            && slot.expire_time
                < std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as u32
        {
            UserItemSlot::default()
        } else {
            slot
        };
        result.write_u32(slot.item_id);
        result.write_u16(slot.durability as u16);
        result.write_u16(slot.count);
        result.write_u8(slot.flag);
        result.write_u32(0x00); // unique ID (pet/cypher ring — not used)
        result.write_u32(slot.expire_time);
    }

    session.send_packet(&result).await?;
    debug!(
        "[{}] Clan warehouse opened for clan {}",
        session.addr(),
        clan_id
    );
    Ok(())
}

/// Handle ClanBankInput (sub-opcode 2) — inventory -> clan warehouse.
/// Packet in: `[u8 sub=2] [u16 npc_id] [u32 item_id] [u8 page] [u8 src_pos] [u8 dst_pos] [u32 count]`
async fn handle_input(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    let (clan_id, _fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_INPUT, 0))
                .await?;
            return Ok(());
        }
    };

    // Verify clan exists
    let world = session.world().clone();
    if world.get_knights(clan_id).is_none() {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    let npc_id = reader.read_u16().unwrap_or(0);

    // NPC range check — prevent remote clan warehouse deposit
    if !npc_in_range(session, npc_id) {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let count = reader.read_u32().unwrap_or(0);

    if count == 0 {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    // Ensure clan warehouse is loaded
    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!("[{}] Failed to load clan warehouse: {}", session.addr(), e);
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    let sid = session.session_id();

    // Special case: gold deposit
    if item_id == ITEM_GOLD {
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                session
                    .send_packet(&build_result(CLAN_BANK_INPUT, 0))
                    .await?;
                return Ok(());
            }
        };

        let mut success = false;
        if let Some(mut data) = CLAN_WAREHOUSES.get_mut(&clan_id) {
            if ch.gold >= count && (data.gold as u64 + count as u64) <= COIN_MAX as u64 {
                data.gold += count;
                success = true;
            }
        }

        if success {
            world.gold_lose(sid, count);
            save_clan_gold_async(session, clan_id);
            session
                .send_packet(&build_result(CLAN_BANK_INPUT, 1))
                .await?;
        } else {
            session
                .send_packet(&build_result(CLAN_BANK_INPUT, 0))
                .await?;
        }
        return Ok(());
    }

    // Validate item table
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_INPUT, 0))
                .await?;
            return Ok(());
        }
    };

    let reference_pos = ITEMS_PER_PAGE * page as usize;
    let wh_slot_idx = reference_pos + dst_pos as usize;

    // Bounds check
    if src_pos as usize >= HAVE_MAX || wh_slot_idx >= WAREHOUSE_MAX {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    // No-trade items cannot be stored
    if (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id) {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let countable = item_table.countable.unwrap_or(0);
    let src_slot_idx = SLOT_MAX + src_pos as usize;

    // Read source item from inventory
    let src_item = match world.get_inventory_slot(sid, src_slot_idx) {
        Some(s) if s.item_id == item_id => s,
        _ => {
            session
                .send_packet(&build_result(CLAN_BANK_INPUT, 0))
                .await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:221-225 — check flags: no rented, duplicate, sealed, or expiration
    if src_item.flag == ITEM_FLAG_RENTED
        || src_item.flag == ITEM_FLAG_SEALED
        || src_item.expire_time > 0
    {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }
    if src_item.flag == ITEM_FLAG_DUPLICATE {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 2))
            .await?;
        return Ok(());
    }

    if src_item.count < count as u16 {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    // Atomically update clan warehouse + player inventory
    let mut wh_success = false;
    let mut final_wh_slot = UserItemSlot::default();

    if let Some(mut data) = CLAN_WAREHOUSES.get_mut(&clan_id) {
        while data.items.len() < WAREHOUSE_MAX {
            data.items.push(UserItemSlot::default());
        }

        let dst = &data.items[wh_slot_idx];
        // Validate destination: non-stackable must go to empty slot,
        // stackable must go to same item or empty slot
        if dst.item_id != 0 && (!is_stackable || dst.item_id != src_item.item_id) {
            // Slot occupied
        } else {
            let dst = &mut data.items[wh_slot_idx];

            if is_stackable {
                dst.count = dst.count.saturating_add(count as u16);
            } else {
                dst.count = count as u16;
            }

            // Transfer properties
            dst.durability = src_item.durability;
            dst.flag = src_item.flag;
            dst.original_flag = src_item.original_flag;
            dst.expire_time = src_item.expire_time;
            dst.item_id = src_item.item_id;

            if dst.count > ITEMCOUNT_MAX {
                dst.count = ITEMCOUNT_MAX;
            }

            // Handle serial number
            let serial = if src_item.serial_num != 0 {
                src_item.serial_num
            } else {
                world.generate_item_serial()
            };
            if is_stackable {
                if src_item.count == count as u16 && dst.serial_num == 0 {
                    dst.serial_num = serial;
                } else if dst.serial_num == 0 {
                    dst.serial_num = world.generate_item_serial();
                }
            } else {
                dst.serial_num = serial;
            }

            final_wh_slot = dst.clone();
            wh_success = true;
        }
    }

    if !wh_success {
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    // Update inventory: reduce source
    let inv_success = world.update_inventory(sid, |inv| {
        if src_slot_idx >= inv.len() {
            return false;
        }
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

    if !inv_success {
        // Rollback clan warehouse change
        if let Some(mut data) = CLAN_WAREHOUSES.get_mut(&clan_id) {
            data.items[wh_slot_idx] = UserItemSlot::default();
        }
        session
            .send_packet(&build_result(CLAN_BANK_INPUT, 0))
            .await?;
        return Ok(());
    }

    world.set_user_ability(sid);
    save_clan_wh_slot_async(session, clan_id, wh_slot_idx, final_wh_slot);
    save_inventory_slot_async(session, src_slot_idx);

    // Send clan warehouse deposit notification to all clan members
    send_clan_bank_notice(&world, clan_id, sid, item_id, count, true);

    // FerihaLog: ClanBankInsertLog (deposit)
    {
        let clan_name = world
            .get_knights(clan_id)
            .map(|k| k.name.clone())
            .unwrap_or_default();
        super::audit_log::log_clan_bank(
            session.pool(),
            session.account_id().unwrap_or(""),
            &world.get_session_name(sid).unwrap_or_default(),
            "deposit",
            clan_id,
            &clan_name,
            item_id,
            count,
            0,
        );
    }

    session
        .send_packet(&build_result(CLAN_BANK_INPUT, 1))
        .await?;
    debug!(
        "[{}] Clan warehouse input: item={}, count={}, slot={}",
        session.addr(),
        item_id,
        count,
        wh_slot_idx
    );
    Ok(())
}

/// Handle ClanBankOutput (sub-opcode 3) — clan warehouse -> inventory.
/// Packet in: `[u8 sub=3] [u16 npc_id] [u32 item_id] [u8 page] [u8 src_pos] [u8 dst_pos] [u32 count]`
async fn handle_output(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    let (clan_id, fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
                .await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:369 — only leader or assistant can withdraw
    if !is_leader_or_assistant(fame) {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    let world = session.world().clone();
    if world.get_knights(clan_id).is_none() {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    let npc_id = reader.read_u16().unwrap_or(0);

    // NPC range check — prevent remote clan warehouse withdrawal
    if !npc_in_range(session, npc_id) {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let count = reader.read_u32().unwrap_or(0);

    if count == 0 {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!("[{}] Failed to load clan warehouse: {}", session.addr(), e);
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    let sid = session.session_id();

    // Special case: gold withdrawal
    if item_id == ITEM_GOLD {
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                session
                    .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
                    .await?;
                return Ok(());
            }
        };

        let mut success = false;
        if let Some(mut data) = CLAN_WAREHOUSES.get_mut(&clan_id) {
            if data.gold >= count && (ch.gold as u64 + count as u64) <= COIN_MAX as u64 {
                data.gold -= count;
                success = true;
            }
        }

        if success {
            world.gold_gain(sid, count);
            save_clan_gold_async(session, clan_id);
            session
                .send_packet(&build_result(CLAN_BANK_OUTPUT, 1))
                .await?;
        } else {
            session
                .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
                .await?;
        }
        return Ok(());
    }

    // Validate item table
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
                .await?;
            return Ok(());
        }
    };

    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let countable = item_table.countable.unwrap_or(0);

    // C++ ClanBank.cpp:430-445 — weight check
    if !world.check_weight(sid, item_id, count as u16) {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 3))
            .await?;
        return Ok(());
    }

    let reference_pos = ITEMS_PER_PAGE * page as usize;
    let wh_slot_idx = reference_pos + src_pos as usize;
    let dst_slot_idx = SLOT_MAX + dst_pos as usize;

    if wh_slot_idx >= WAREHOUSE_MAX || dst_pos as usize >= HAVE_MAX {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    // Read source from clan warehouse cache
    let src_item = match CLAN_WAREHOUSES.get(&clan_id) {
        Some(data) => {
            let slot = data.items.get(wh_slot_idx).cloned().unwrap_or_default();
            if slot.item_id != item_id || slot.count < count as u16 {
                session
                    .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
                    .await?;
                return Ok(());
            }
            if slot.flag == ITEM_FLAG_DUPLICATE {
                session
                    .send_packet(&build_result(CLAN_BANK_OUTPUT, 2))
                    .await?;
                return Ok(());
            }
            slot
        }
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
                .await?;
            return Ok(());
        }
    };

    // Update inventory: add to destination
    let inv_success = world.update_inventory(sid, |inv| {
        if dst_slot_idx >= inv.len() {
            return false;
        }
        let dst = &inv[dst_slot_idx];
        if dst.item_id != 0 && (!is_stackable || dst.item_id != src_item.item_id) {
            return false;
        }

        let dst = &mut inv[dst_slot_idx];
        if is_stackable {
            dst.count = dst.count.saturating_add(count as u16);
        } else {
            dst.count = count as u16;
        }
        dst.durability = src_item.durability;
        dst.flag = src_item.flag;
        dst.original_flag = src_item.original_flag;
        dst.expire_time = src_item.expire_time;
        dst.item_id = src_item.item_id;

        if dst.count > ITEMCOUNT_MAX {
            dst.count = ITEMCOUNT_MAX;
        }

        // Handle serial number
        let serial = if src_item.serial_num != 0 {
            src_item.serial_num
        } else {
            world.generate_item_serial()
        };
        if is_stackable {
            if src_item.count == count as u16 && dst.serial_num == 0 {
                dst.serial_num = serial;
            } else if dst.serial_num == 0 {
                dst.serial_num = world.generate_item_serial();
            }
        } else {
            dst.serial_num = serial;
        }

        true
    });

    if !inv_success {
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    // Reduce clan warehouse source — re-validate under write lock to prevent
    // TOCTOU race if two clan members withdraw from the same slot concurrently.
    let mut final_wh_slot = UserItemSlot::default();
    let wh_deduct_ok = if let Some(mut data) = CLAN_WAREHOUSES.get_mut(&clan_id) {
        let src_mut = &mut data.items[wh_slot_idx];
        // Re-validate: another withdrawal may have changed the slot since our read.
        if src_mut.item_id != item_id || src_mut.count < count as u16 {
            false
        } else {
            if is_stackable {
                src_mut.count = src_mut.count.saturating_sub(count as u16);
            } else {
                src_mut.count = 0;
            }
            if src_mut.count == 0 || countable == 0 {
                *src_mut = UserItemSlot::default();
            }
            final_wh_slot = data.items[wh_slot_idx].clone();
            true
        }
    } else {
        false
    };

    // If warehouse deduction failed (race condition), rollback the inventory change.
    if !wh_deduct_ok {
        world.update_inventory(sid, |inv| {
            if dst_slot_idx < inv.len() {
                inv[dst_slot_idx] = UserItemSlot::default();
            }
            true
        });
        session
            .send_packet(&build_result(CLAN_BANK_OUTPUT, 0))
            .await?;
        return Ok(());
    }

    world.set_user_ability(sid);
    save_clan_wh_slot_async(session, clan_id, wh_slot_idx, final_wh_slot);
    save_inventory_slot_async(session, dst_slot_idx);

    // Send clan warehouse withdraw notification to all clan members
    send_clan_bank_notice(&world, clan_id, sid, item_id, count, false);

    // FerihaLog: ClanBankInsertLog (withdraw)
    {
        let clan_name = world
            .get_knights(clan_id)
            .map(|k| k.name.clone())
            .unwrap_or_default();
        super::audit_log::log_clan_bank(
            session.pool(),
            session.account_id().unwrap_or(""),
            &world.get_session_name(sid).unwrap_or_default(),
            "withdraw",
            clan_id,
            &clan_name,
            item_id,
            count,
            0,
        );
    }

    session
        .send_packet(&build_result(CLAN_BANK_OUTPUT, 1))
        .await?;
    debug!(
        "[{}] Clan warehouse output: item={}, count={}, slot={}",
        session.addr(),
        item_id,
        count,
        wh_slot_idx
    );
    Ok(())
}

/// Handle ClanBankMove (sub-opcode 4) — rearrange within clan warehouse.
/// Packet in: `[u8 sub=4] [u16 npc_id] [u32 item_id] [u8 page] [u8 src_pos] [u8 dst_pos]`
async fn handle_move(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
        return Ok(());
    }

    let (clan_id, fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_MOVE, 0))
                .await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:610 — leader or assistant only
    if !is_leader_or_assistant(fame) {
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
        return Ok(());
    }

    let world = session.world().clone();
    if world.get_knights(clan_id).is_none() {
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
        return Ok(());
    }

    let npc_id = reader.read_u16().unwrap_or(0);

    // NPC range check — prevent remote clan warehouse move
    if !npc_in_range(session, npc_id) {
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
        return Ok(());
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!("[{}] Failed to load clan warehouse: {}", session.addr(), e);
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
        return Ok(());
    }

    let reference_pos = ITEMS_PER_PAGE * page as usize;
    let src_idx = reference_pos + src_pos as usize;
    let dst_idx = reference_pos + dst_pos as usize;

    if src_idx >= WAREHOUSE_MAX || dst_idx >= WAREHOUSE_MAX {
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
        return Ok(());
    }

    let mut success = false;
    let mut final_src = UserItemSlot::default();
    let mut final_dst = UserItemSlot::default();

    if let Some(mut data) = CLAN_WAREHOUSES.get_mut(&clan_id) {
        while data.items.len() < WAREHOUSE_MAX {
            data.items.push(UserItemSlot::default());
        }

        let src = &data.items[src_idx];
        let dst = &data.items[dst_idx];

        // Source must match item_id, destination must be empty
        if src.item_id == item_id && dst.item_id == 0 {
            // Check for duplicate flags
            if src.flag == ITEM_FLAG_DUPLICATE || dst.flag == ITEM_FLAG_DUPLICATE {
                session
                    .send_packet(&build_result(CLAN_BANK_MOVE, 2))
                    .await?;
                return Ok(());
            }

            let tmp = data.items[src_idx].clone();
            data.items[dst_idx] = tmp;
            data.items[src_idx] = UserItemSlot::default();
            final_src = data.items[src_idx].clone();
            final_dst = data.items[dst_idx].clone();
            success = true;
        }
    }

    if success {
        save_clan_wh_slot_async(session, clan_id, src_idx, final_src);
        save_clan_wh_slot_async(session, clan_id, dst_idx, final_dst);
        super::warehouse::gm_move_note(session.world(), session.session_id(), "Clan bank", item_id, src_idx, dst_idx);
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 1))
            .await?;
        debug!(
            "[{}] Clan warehouse move: item={}, {} -> {}",
            session.addr(),
            item_id,
            src_idx,
            dst_idx
        );
    } else {
        session
            .send_packet(&build_result(CLAN_BANK_MOVE, 0))
            .await?;
    }

    Ok(())
}

/// Handle ClanBankInventoryMove (sub-opcode 5) — rearrange items within
/// inventory while clan warehouse UI is open.
/// Packet in: `[u8 sub=5] [u16 npc_id] [u32 item_id] [u8 page] [u8 src_pos] [u8 dst_pos]`
async fn handle_inventory_move(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 0))
            .await?;
        return Ok(());
    }

    let (clan_id, fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session
                .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 0))
                .await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:713 — leader or assistant only
    if !is_leader_or_assistant(fame) {
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 0))
            .await?;
        return Ok(());
    }

    let world = session.world().clone();
    if world.get_knights(clan_id).is_none() {
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 0))
            .await?;
        return Ok(());
    }

    let npc_id = reader.read_u16().unwrap_or(0);

    // NPC range check — prevent remote clan warehouse inventory move
    if !npc_in_range(session, npc_id) {
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 0))
            .await?;
        return Ok(());
    }

    let item_id = reader.read_u32().unwrap_or(0);
    let _page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);

    if src_pos as usize >= HAVE_MAX || dst_pos as usize >= HAVE_MAX {
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 0))
            .await?;
        return Ok(());
    }

    let sid = session.session_id();
    let src_idx = SLOT_MAX + src_pos as usize;
    let dst_idx = SLOT_MAX + dst_pos as usize;

    // C++ ClanBank.cpp:756-767 — source must match, check for duplicates, then swap
    let mut result_code: u8 = 0;
    let success = world.update_inventory(sid, |inv| {
        if src_idx >= inv.len() || dst_idx >= inv.len() {
            return false;
        }
        if inv[src_idx].item_id != item_id {
            return false;
        }
        if inv[src_idx].flag == ITEM_FLAG_DUPLICATE || inv[dst_idx].flag == ITEM_FLAG_DUPLICATE {
            result_code = 2;
            return false;
        }

        // Swap the two slots
        let tmp = inv[dst_idx].clone();
        inv[dst_idx] = inv[src_idx].clone();
        inv[src_idx] = tmp;
        true
    });

    if success {
        save_inventory_slot_async(session, src_idx);
        save_inventory_slot_async(session, dst_idx);
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, 1))
            .await?;
    } else {
        let code = if result_code != 0 { result_code } else { 0 };
        session
            .send_packet(&build_result(CLAN_BANK_INVENTORY_MOVE, code))
            .await?;
    }

    Ok(())
}

/// Save a clan warehouse slot to DB (fire-and-forget).
fn save_clan_wh_slot_async(
    session: &ClientSession,
    clan_id: u16,
    slot_idx: usize,
    slot: UserItemSlot,
) {
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = ClanWarehouseRepository::new(&pool);
        let params = SaveClanWarehouseItemParams {
            clan_id: clan_id as i16,
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
            warn!("Failed to save clan warehouse slot {}: {}", slot_idx, e);
        }
    });
}

/// Save clan warehouse gold to DB (fire-and-forget).
fn save_clan_gold_async(session: &ClientSession, clan_id: u16) {
    let gold = CLAN_WAREHOUSES.get(&clan_id).map(|d| d.gold).unwrap_or(0);
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = ClanWarehouseRepository::new(&pool);
        if let Err(e) = repo
            .save_gold(clan_id as i16, gold.min(i32::MAX as u32) as i32)
            .await
        {
            warn!("Failed to save clan warehouse gold: {}", e);
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

/// Clear clan warehouse cache for a given clan (e.g., on clan disband).
pub fn evict_cache(clan_id: u16) {
    CLAN_WAREHOUSES.remove(&clan_id);
}

/// Send clan warehouse deposit/withdraw notification to all clan members.
/// Uses `ChatType::Gm` (12) for the notification, which displays as a system message.
fn send_clan_bank_notice(
    world: &std::sync::Arc<crate::world::WorldState>,
    clan_id: u16,
    sid: u16,
    item_id: u32,
    count: u32,
    is_deposit: bool,
) {
    let player_name = world.get_session_name(sid).unwrap_or_default();
    let item_name = world
        .get_item(item_id)
        .and_then(|item| item.str_name.clone())
        .unwrap_or_else(|| format!("Item#{}", item_id));

    let message = if is_deposit {
        if count > 1 {
            format!(
                "### {} Clan Bankasina {} adet {} birakti. ###",
                player_name, count, item_name
            )
        } else {
            format!(
                "### {} Clan Bankasina {} Birakti. ###",
                player_name, item_name
            )
        }
    } else if count > 1 {
        format!(
            "### {} Clan Bankasindan {} adet {} aldi. ###",
            player_name, count, item_name
        )
    } else {
        format!(
            "### {} Clan Bankasindan {} aldi. ###",
            player_name, item_name
        )
    };

    // GM_CHAT type=12, sender_id=0 (system), nation=0
    let pkt = build_chat_packet(12, 0, 0, "", &message, 0, 0, 0);
    world.send_to_knights_members(clan_id, Arc::new(pkt), None);
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_opcode_values() {
        assert_eq!(CLAN_BANK_OPEN, 0x01);
        assert_eq!(CLAN_BANK_INPUT, 0x02);
        assert_eq!(CLAN_BANK_OUTPUT, 0x03);
        assert_eq!(CLAN_BANK_MOVE, 0x04);
        assert_eq!(CLAN_BANK_INVENTORY_MOVE, 0x05);
    }

    #[test]
    fn test_constants() {
        assert_eq!(WAREHOUSE_MAX, 192);
        assert_eq!(ITEMS_PER_PAGE, 24);
        assert_eq!(WAREHOUSE_MAX, ITEMS_PER_PAGE * 8); // 8 pages
        assert_eq!(ITEM_GOLD, 900_000_000);
        assert!(ITEM_NO_TRADE_MIN > ITEM_GOLD);
        assert!(ITEM_NO_TRADE_MAX > ITEM_NO_TRADE_MIN);
        assert_eq!(COIN_MAX, 2_100_000_000);
    }

    #[test]
    fn test_is_leader_or_assistant() {
        assert!(is_leader_or_assistant(CHIEF));
        assert!(is_leader_or_assistant(VICECHIEF));
        assert!(!is_leader_or_assistant(0)); // no clan
        assert!(!is_leader_or_assistant(3)); // command captain
        assert!(!is_leader_or_assistant(4)); // officer
        assert!(!is_leader_or_assistant(5)); // trainee
    }

    #[test]
    fn test_build_result_packet() {
        let pkt = build_result(CLAN_BANK_OPEN, 1);
        assert_eq!(pkt.opcode, Opcode::WizClanWarehouse as u8);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], CLAN_BANK_OPEN);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_build_result_fail() {
        let pkt = build_result(CLAN_BANK_INPUT, 0);
        assert_eq!(pkt.opcode, Opcode::WizClanWarehouse as u8);
        assert_eq!(pkt.data[0], CLAN_BANK_INPUT);
        assert_eq!(pkt.data[1], 0);
    }

    #[test]
    fn test_build_result_duplicate() {
        let pkt = build_result(CLAN_BANK_OUTPUT, 2);
        assert_eq!(pkt.data[1], 2); // duplicate error
    }

    #[test]
    fn test_build_result_weight() {
        let pkt = build_result(CLAN_BANK_OUTPUT, 3);
        assert_eq!(pkt.data[1], 3); // weight error
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
    fn test_item_flags() {
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
    }

    #[test]
    fn test_clan_warehouse_data_local() {
        // Test ClanWarehouseData operations without touching the global LazyLock
        let data = ClanWarehouseData {
            items: vec![UserItemSlot::default(); WAREHOUSE_MAX],
            gold: 12345,
        };
        assert_eq!(data.gold, 12345);
        assert_eq!(data.items.len(), WAREHOUSE_MAX);
        assert_eq!(data.items[0].item_id, 0);
    }

    #[test]
    fn test_clan_warehouse_item_deposit_logic() {
        // Test deposit logic: set item in slot, verify fields
        let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];

        items[0] = UserItemSlot {
            item_id: 100001,
            durability: 50,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 42,
            expire_time: 0,
        };

        assert_eq!(items[0].item_id, 100001);
        assert_eq!(items[0].count, 1);
        assert_eq!(items[0].durability, 50);
        assert_eq!(items[0].serial_num, 42);
        assert_eq!(items[1].item_id, 0); // empty
    }

    #[test]
    fn test_clan_warehouse_gold_deposit() {
        let mut data = ClanWarehouseData {
            items: vec![UserItemSlot::default(); WAREHOUSE_MAX],
            gold: 1000,
        };

        // Deposit 500 gold
        data.gold += 500;
        assert_eq!(data.gold, 1500);

        // Withdraw 300 gold
        data.gold -= 300;
        assert_eq!(data.gold, 1200);
    }

    #[test]
    fn test_clan_warehouse_move_logic() {
        let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];
        items[5] = UserItemSlot {
            item_id: 200001,
            durability: 100,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 99,
            expire_time: 0,
        };

        // Move from slot 5 to slot 10
        assert_eq!(items[5].item_id, 200001);
        assert_eq!(items[10].item_id, 0);

        let tmp = items[5].clone();
        items[10] = tmp;
        items[5] = UserItemSlot::default();

        assert_eq!(items[5].item_id, 0);
        assert_eq!(items[10].item_id, 200001);
        assert_eq!(items[10].serial_num, 99);
    }

    #[test]
    fn test_clan_warehouse_inventory_swap_logic() {
        // Test the swap logic used in inventory_move
        let mut src = UserItemSlot {
            item_id: 300001,
            durability: 75,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 55,
            expire_time: 0,
        };
        let mut dst = UserItemSlot::default();

        // Swap
        let tmp = dst.clone();
        dst = src.clone();
        src = tmp;

        assert_eq!(src.item_id, 0); // now empty
        assert_eq!(dst.item_id, 300001);
        assert_eq!(dst.serial_num, 55);
    }

    #[test]
    fn test_no_trade_range() {
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&900_000_001));
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&999_999_999));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&900_000_000));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&1_000_000_000));
    }

    #[test]
    fn test_open_response_packet_format() {
        // Verify the expected packet layout for ClanBankOpen success response
        let mut result = Packet::new(Opcode::WizClanWarehouse as u8);
        result.write_u8(CLAN_BANK_OPEN);
        result.write_u8(1); // success
        result.write_u32(5000); // clan gold

        // Each item slot: u32 item_id + u16 dur + u16 count + u8 flag + u32 unique + u32 expire = 17 bytes
        for _ in 0..WAREHOUSE_MAX {
            result.write_u32(0); // item_id
            result.write_u16(0); // durability
            result.write_u16(0); // count
            result.write_u8(0); // flag
            result.write_u32(0); // unique_id
            result.write_u32(0); // expire_time
        }

        assert_eq!(result.opcode, Opcode::WizClanWarehouse as u8);
        // 1 (sub) + 1 (result) + 4 (gold) + 192 * 17 (items) = 3270
        assert_eq!(result.data.len(), 6 + 192 * 17);
    }

    #[test]
    fn test_open_fail_response_format() {
        let pkt = build_result(CLAN_BANK_OPEN, 0);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], CLAN_BANK_OPEN);
        assert_eq!(pkt.data[1], 0);
    }

    #[test]
    fn test_stackable_deposit_to_existing_stack() {
        // Verify stackable item logic: adding count to existing slot
        let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];
        items[3] = UserItemSlot {
            item_id: 389010, // arrow
            durability: 0,
            count: 500,
            flag: 0,
            original_flag: 0,
            serial_num: 10,
            expire_time: 0,
        };

        // Deposit 300 more
        let is_stackable = true;
        let src_item_id = 389010u32;
        let count = 300u16;

        let dst = &mut items[3];
        assert_eq!(dst.item_id, src_item_id);
        if is_stackable {
            dst.count += count;
        }
        assert_eq!(dst.count, 800);
    }

    #[test]
    fn test_non_stackable_reject_occupied_slot() {
        let items = vec![
            UserItemSlot {
                item_id: 150001,
                durability: 100,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
            1
        ];

        let is_stackable = false;
        let dst = &items[0];
        // Non-stackable: destination must be empty
        let can_deposit = is_stackable || dst.item_id == 0;
        assert!(!can_deposit);
    }

    #[test]
    fn test_gold_overflow_check() {
        let gold: u32 = 2_000_000_000;
        let deposit: u32 = 200_000_000;
        let fits = (gold as u64 + deposit as u64) <= COIN_MAX as u64;
        assert!(!fits); // 2.2B > 2.1B limit
    }

    #[test]
    fn test_gold_within_limit() {
        let gold: u32 = 1_000_000_000;
        let deposit: u32 = 500_000_000;
        let fits = (gold as u64 + deposit as u64) <= COIN_MAX as u64;
        assert!(fits); // 1.5B <= 2.1B
    }

    #[test]
    fn test_expired_item_filter() {
        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        let expired = UserItemSlot {
            item_id: 100001,
            durability: 50,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 1,
            expire_time: now_ts - 3600, // expired 1 hour ago
        };

        let not_expired = UserItemSlot {
            item_id: 100002,
            durability: 50,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 2,
            expire_time: now_ts + 3600, // expires in 1 hour
        };

        let no_expiry = UserItemSlot {
            item_id: 100003,
            durability: 50,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 3,
            expire_time: 0, // never expires
        };

        // Expired items should be filtered (as in handle_open)
        assert!(expired.expire_time != 0 && expired.expire_time < now_ts);
        assert!(!(not_expired.expire_time != 0 && not_expired.expire_time < now_ts));
        assert!(!(no_expiry.expire_time != 0 && no_expiry.expire_time < now_ts));
    }

    #[test]
    fn test_clan_warehouse_data_clone() {
        let data = ClanWarehouseData {
            items: vec![
                UserItemSlot {
                    item_id: 100001,
                    durability: 50,
                    count: 1,
                    flag: 0,
                    original_flag: 0,
                    serial_num: 42,
                    expire_time: 0,
                },
                UserItemSlot::default(),
            ],
            gold: 5000,
        };

        let cloned = data.clone();
        assert_eq!(cloned.gold, 5000);
        assert_eq!(cloned.items[0].item_id, 100001);
        assert_eq!(cloned.items[1].item_id, 0);
    }

    // ── Sprint 965: Additional coverage ──────────────────────────────

    /// Clan bank sub-opcodes are sequential 1-5.
    #[test]
    fn test_clan_bank_sub_opcodes_sequential() {
        assert_eq!(CLAN_BANK_OPEN, 1);
        assert_eq!(CLAN_BANK_INPUT, 2);
        assert_eq!(CLAN_BANK_OUTPUT, 3);
        assert_eq!(CLAN_BANK_MOVE, 4);
        assert_eq!(CLAN_BANK_INVENTORY_MOVE, 5);
    }

    /// Leader/assistant check uses CHIEF=1 and VICECHIEF=2.
    #[test]
    fn test_leader_or_assistant_roles() {
        assert!(is_leader_or_assistant(CHIEF));
        assert!(is_leader_or_assistant(VICECHIEF));
        // Regular member (fame=0) should not pass
        assert!(!is_leader_or_assistant(0));
        // Other values should not pass
        assert!(!is_leader_or_assistant(3));
        assert!(!is_leader_or_assistant(255));
    }

    /// No-trade items cannot be deposited (range check).
    #[test]
    fn test_no_trade_item_range() {
        assert_eq!(ITEM_NO_TRADE_MIN, 900_000_001);
        assert_eq!(ITEM_NO_TRADE_MAX, 999_999_999);
        // ITEM_GOLD is below the range
        assert!(ITEM_GOLD < ITEM_NO_TRADE_MIN);
        // Verify the range is contiguous
        assert!(ITEM_NO_TRADE_MAX > ITEM_NO_TRADE_MIN);
    }

    /// Item flag constants used in deposit validation.
    #[test]
    fn test_item_flag_constants() {
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
        // All flags are distinct
        assert_ne!(ITEM_FLAG_RENTED, ITEM_FLAG_DUPLICATE);
        assert_ne!(ITEM_FLAG_RENTED, ITEM_FLAG_SEALED);
        assert_ne!(ITEM_FLAG_DUPLICATE, ITEM_FLAG_SEALED);
    }

    /// build_result produces correct sub-opcode + result byte layout.
    #[test]
    fn test_build_result_packet_layout() {
        let pkt = build_result(CLAN_BANK_OPEN, 1);
        assert_eq!(pkt.data[0], CLAN_BANK_OPEN);
        assert_eq!(pkt.data[1], 1);
        assert_eq!(pkt.data.len(), 2);
    }

    /// Clan bank sub-opcodes 1-5 are contiguous with no gaps.
    #[test]
    fn test_clan_bank_subopcodes_no_gap() {
        let ops = [
            CLAN_BANK_OPEN,
            CLAN_BANK_INPUT,
            CLAN_BANK_OUTPUT,
            CLAN_BANK_MOVE,
            CLAN_BANK_INVENTORY_MOVE,
        ];
        for i in 0..ops.len() - 1 {
            assert_eq!(
                ops[i + 1] - ops[i],
                1,
                "gap between sub-opcode {} and {}",
                ops[i],
                ops[i + 1]
            );
        }
    }

    /// WAREHOUSE_MAX (192) = ITEMS_PER_PAGE (24) × 8 pages.
    #[test]
    fn test_warehouse_page_structure() {
        assert_eq!(WAREHOUSE_MAX, 192);
        assert_eq!(ITEMS_PER_PAGE, 24);
        assert_eq!(WAREHOUSE_MAX / ITEMS_PER_PAGE, 8);
        assert_eq!(WAREHOUSE_MAX % ITEMS_PER_PAGE, 0);
    }

    /// COIN_MAX is 2.1 billion — gold cap applies to clan warehouse too.
    #[test]
    fn test_clan_gold_cap() {
        assert_eq!(COIN_MAX, 2_100_000_000);
        // Fits in u32 but below u32::MAX (4.29B)
        assert!(COIN_MAX < u32::MAX);
        assert!(COIN_MAX > 2_000_000_000);
    }

    /// Only CHIEF and VICECHIEF can withdraw/move (leader_or_assistant check).
    #[test]
    fn test_clan_warehouse_authority_roles() {
        assert_eq!(CHIEF, 1);
        assert_eq!(VICECHIEF, 2);
        // Both are less than TRAINEE (5) — higher authority = lower number
        assert!(CHIEF < VICECHIEF);
        // Only these two roles have full access
        let authorized = [CHIEF, VICECHIEF];
        assert_eq!(authorized.len(), 2);
    }

    /// ITEMCOUNT_MAX (9999) limits stack size in clan warehouse deposits.
    #[test]
    fn test_itemcount_max_stack_limit() {
        assert_eq!(ITEMCOUNT_MAX, 9999);
        // Fits in u16
        assert!(ITEMCOUNT_MAX <= u16::MAX);
        // Can't exceed 4 digits (display constraint)
        assert!(ITEMCOUNT_MAX < 10_000);
    }
}
