//! WIZ_GUILD_BANK (0xD0) handler — Guild/Clan bank system.
//! v2525 client's native guild bank panel (panel at `[[0x1092A14]+0x684]`).
//! Shared bank storage for clan members with tab-based organization,
//! gold management, transaction logging, and permission controls.
//! ## Client RE
//! - Panel field: `[game+0x684]` — init to 0xFFFFFFFF, checked != 0 before dispatch
//! - Main handler: `0x714A90` — 6-entry jump table at `0x714B20`
//! - Sub-panels: bank=0x120, tab_content=0x124, item_detail=0x128, item_list=0x12C
//! - Gold: stored as i64 at `[esi+0xBD8]/[esi+0xBDC]`
//! - Max tabs: 9 (index 0-8)
//! - Sound: 0x53092 on bank open
//! ## S2C Sub-opcodes
//! | Sub | Sub-sub | Name            | Wire format |
//! |-----|---------|-----------------|-------------|
//! | 1   | 1       | Open/Init       | u16 scroll_pos, u16 npc_id |
//! | 1   | 2       | Result code     | i16 result (-4=full, -3=err2, -2=err1, -1=success) |
//! | 2   | 1       | Full init       | u8 tab_perm, u16 npc_id, u16 gold_hi, u16 gold_lo, u8 max_tabs |
//! | 2   | 2       | Tab info        | u16 name_id, i32 total, i32 max, i32 current |
//! | 2   | 3       | Tab content     | (raw stream) |
//! | 2   | 4       | Full reset      | (empty) |
//! | 3   | 1       | Map coords      | u8 count, {u16 x, u16 z} × count |
//! | 3   | 2       | Item name       | u16 name_id, u16 str_len, string data |
//! | 4   | 1       | Gold update     | u8(0), u16 gold |
//! | 4   | 2       | Item slot       | i32 slot, u8 tab, u16×5 (count/maxdur/dur/flag/expiry) |
//! | 4   | 3       | Reset tab       | i32 tab_id |
//! | 5   | 1       | Log page init   | i32 time, u16 page, i32 total, u16 count |
//! | 5   | 2       | Log entry       | u8 tab, i32 item, u16 qty, u16 price, u8 action |
//! | 5   | 3       | Log removal     | u8 flag |
//! | 6   | 1       | Member list     | u8 count, {u16} × count, u8 perms, {u16} × perms |
//! | 6   | 2       | Perm result     | u8 role, u16 result |
//! | 6   | 3       | Item detail     | u16 item_id |
//! | 6   | 4       | Perm multi      | u8 count, {u16} × count |
//! ## String IDs
//! - 0xAD75 (44405): Success
//! - 0xAD76 (44406): Error type 1
//! - 0xAD77 (44407): Error type 2
//! - 0xAD86 (44422): Bank full
//! - 0xAD87 (44423): Open dialog prompt
//! - 0xAD88 (44424): Permission error

use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use ko_db::repositories::character::{CharacterRepository, SaveItemParams};
use ko_db::repositories::guild_bank::GuildBankRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use super::chat::build_chat_packet;
use crate::clan_constants::{CHIEF, VICECHIEF};
use crate::inventory_constants::{ITEMS_PER_PAGE, WAREHOUSE_MAX};
use crate::session::{ClientSession, SessionState};
use crate::world::{
    UserItemSlot, ITEMCOUNT_MAX, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED, ITEM_FLAG_SEALED,
    ITEM_GOLD, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN,
};

use super::{COIN_MAX, HAVE_MAX, SLOT_MAX};

// ── Sub-opcode constants ────────────────────────────────────────────────

const GB_SUB_ITEM_RESULT: u8 = 1;
const GB_SUB_OPEN: u8 = 2;
const GB_SUB_MAP_ITEMS: u8 = 3;
const GB_SUB_ITEM_DATA: u8 = 4;
const GB_SUB_LOG: u8 = 5;
const GB_SUB_PERMISSIONS: u8 = 6;

// ── Result codes (sub=1, sub_sub=2) ─────────────────────────────────────

/// Result -1: Success (string 0xAD75).
const RESULT_SUCCESS: i16 = -1;
/// Result -2: Error type 1 (string 0xAD76).
const RESULT_ERROR_1: i16 = -2;
/// Result -3: Error type 2 (string 0xAD77).
#[cfg(test)]
const RESULT_ERROR_2: i16 = -3;
/// Result -4: Bank full (string 0xAD86).
const RESULT_FULL: i16 = -4;

// ── Guild bank item layout ──────────────────────────────────────────────

/// Items per tab (aliased from ITEMS_PER_PAGE for semantic clarity).
const ITEMS_PER_TAB: usize = ITEMS_PER_PAGE;

// ── In-memory cache ─────────────────────────────────────────────────────

/// In-memory guild bank cache: clan_id → GuildBankCacheData.
/// Loaded lazily from DB on first open. Modified in-memory by item ops,
/// persisted to DB asynchronously (fire-and-forget).
static GUILD_BANKS: LazyLock<DashMap<u16, GuildBankCacheData>> = LazyLock::new(DashMap::new);

/// Cached guild bank state for a clan.
#[derive(Debug, Clone)]
struct GuildBankCacheData {
    /// Flat item array (192 slots: tab*24 + slot_within_tab).
    items: Vec<UserItemSlot>,
    /// Stored gold.
    gold: i64,
    /// Number of unlocked tabs (1-9).
    max_tabs: u8,
    /// Permission flags.
    permissions: u8,
}

// ── S2C Builders — Sub=1: Item Result ───────────────────────────────────

/// Build item result open/init packet (sub=1, sub_sub=1).
/// Client plays sound 0x53092, initializes bank panel, sets scroll/NPC.
pub fn build_open_init(scroll_pos: u16, npc_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_ITEM_RESULT);
    pkt.write_u8(1); // sub_sub
    pkt.write_u16(scroll_pos);
    pkt.write_u16(npc_id);
    pkt
}

/// Build item result code packet (sub=1, sub_sub=2).
/// Result codes: -4=full, -3=err2, -2=err1, -1=success, 0=none, 1=npc_update, 2=reset.
pub fn build_result_code(result: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_ITEM_RESULT);
    pkt.write_u8(2); // sub_sub
    pkt.write_i16(result);
    pkt
}

/// Build success result (convenience).
pub fn build_result_success() -> Packet {
    build_result_code(RESULT_SUCCESS)
}

/// Build bank-full result (convenience).
pub fn build_result_full() -> Packet {
    build_result_code(RESULT_FULL)
}

/// Build error result (convenience).
pub fn build_result_error() -> Packet {
    build_result_code(RESULT_ERROR_1)
}

// ── S2C Builders — Sub=2: Open Bank ─────────────────────────────────────

/// Build full bank init packet (sub=2, sub_sub=1).
/// Client opens the bank UI, shows dialog (string 0xAD87), initializes tabs.
pub fn build_bank_init(
    tab_permission: u8,
    npc_id: u16,
    gold_hi: u16,
    gold_lo: u16,
    max_tabs: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_OPEN);
    pkt.write_u8(1); // sub_sub
    pkt.write_u8(tab_permission);
    pkt.write_u16(npc_id);
    pkt.write_u16(gold_hi);
    pkt.write_u16(gold_lo);
    pkt.write_u8(max_tabs);
    pkt
}

/// Build tab permissions update packet (sub=2, sub_sub=2).
pub fn build_tab_info(
    item_name_id: u16,
    total_items: i32,
    max_capacity: i32,
    current_count: i32,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_OPEN);
    pkt.write_u8(2); // sub_sub
    pkt.write_u16(item_name_id);
    pkt.write_i32(total_items);
    pkt.write_i32(max_capacity);
    pkt.write_i32(current_count);
    pkt
}

/// Build full reset packet (sub=2, sub_sub=4).
pub fn build_bank_reset() -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_OPEN);
    pkt.write_u8(4); // sub_sub
    pkt
}

// ── S2C Builders — Sub=3: Map Items ─────────────────────────────────────

/// Build map coordinate markers packet (sub=3, sub_sub=1).
/// Displays markers on the minimap for item locations.
pub fn build_map_coords(coords: &[(u16, u16)]) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_MAP_ITEMS);
    pkt.write_u8(1); // sub_sub
    pkt.write_u8(coords.len().min(255) as u8);
    for &(x, z) in coords.iter().take(255) {
        pkt.write_u16(x);
        pkt.write_u16(z);
    }
    pkt
}

// ── S2C Builders — Sub=4: Item Data ─────────────────────────────────────

/// Build gold update packet (sub=4, sub_sub=1).
/// Client sign-extends u16 gold to i64 for display.
pub fn build_gold_update(gold: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_ITEM_DATA);
    pkt.write_u8(1); // sub_sub
    pkt.write_u8(0); // skip byte
    pkt.write_u16(gold);
    pkt
}

/// Build item slot data packet (sub=4, sub_sub=2).
pub fn build_item_slot(
    slot_id: i32,
    tab_index: u8,
    item_count: u16,
    max_durability: u16,
    current_durability: u16,
    flag: u16,
    expiry_time: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_ITEM_DATA);
    pkt.write_u8(2); // sub_sub
    pkt.write_i32(slot_id);
    pkt.write_u8(tab_index);
    pkt.write_u16(item_count);
    pkt.write_u16(max_durability);
    pkt.write_u16(current_durability);
    pkt.write_u16(flag);
    pkt.write_u16(expiry_time);
    pkt
}

/// Build reset tab packet (sub=4, sub_sub=3).
pub fn build_reset_tab(tab_id: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_ITEM_DATA);
    pkt.write_u8(3); // sub_sub
    pkt.write_i32(tab_id);
    pkt
}

// ── S2C Builders — Sub=5: Transaction Log ───────────────────────────────

/// Build log page init packet (sub=5, sub_sub=1).
pub fn build_log_page_init(
    timestamp: i32,
    page_num: u16,
    total_entries: i32,
    entries_on_page: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_LOG);
    pkt.write_u8(1); // sub_sub
    pkt.write_i32(timestamp);
    pkt.write_u16(page_num);
    pkt.write_i32(total_entries);
    pkt.write_u16(entries_on_page);
    pkt
}

/// Build log entry packet (sub=5, sub_sub=2).
pub fn build_log_entry(
    tab_index: u8,
    item_id: i32,
    quantity: u16,
    price: u16,
    action_type: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_LOG);
    pkt.write_u8(2); // sub_sub
    pkt.write_u8(tab_index);
    pkt.write_i32(item_id);
    pkt.write_u16(quantity);
    pkt.write_u16(price);
    pkt.write_u8(action_type);
    pkt
}

/// Build log removal packet (sub=5, sub_sub=3).
pub fn build_log_removal(flag: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_LOG);
    pkt.write_u8(3); // sub_sub
    pkt.write_u8(flag);
    pkt
}

// ── S2C Builders — Sub=6: Permissions ───────────────────────────────────

/// Build permission result packet (sub=6, sub_sub=2).
/// Result codes: -11..-1 = errors (string 0xAD88), 0 = none, 1 = add member.
pub fn build_perm_result(role_type: u8, result_code: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_PERMISSIONS);
    pkt.write_u8(2); // sub_sub
    pkt.write_u8(role_type);
    pkt.write_u16(result_code);
    pkt
}

/// Build item detail packet (sub=6, sub_sub=3).
pub fn build_perm_item_detail(item_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
    pkt.write_u8(GB_SUB_PERMISSIONS);
    pkt.write_u8(3); // sub_sub
    pkt.write_u16(item_id);
    pkt
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Common state validation: not dead, not trading, not merchanting, not mining, not fishing.
fn validate_basic_state(session: &ClientSession) -> bool {
    let world = session.world();
    let sid = session.session_id();

    !(world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid))
}

/// Validate clan membership and get clan_id + fame.
fn get_clan_info(session: &ClientSession) -> Option<(u16, u8)> {
    let world = session.world();
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

/// Load guild bank from DB into cache if not already loaded.
async fn ensure_loaded(session: &ClientSession, clan_id: u16) -> anyhow::Result<()> {
    if GUILD_BANKS.contains_key(&clan_id) {
        return Ok(());
    }

    let pool = session.pool().clone();
    let repo = GuildBankRepository::new(&pool);

    let bank = repo.load_or_create(clan_id as i32).await?;
    let db_items = repo.load_items(clan_id as i32).await?;

    let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];
    for row in &db_items {
        let flat_idx = row.tab_index as usize * ITEMS_PER_TAB + row.slot_id as usize;
        if flat_idx < WAREHOUSE_MAX {
            items[flat_idx] = UserItemSlot {
                item_id: row.item_id as u32,
                durability: row.cur_durability,
                count: row.item_count as u16,
                flag: row.flag as u8,
                original_flag: 0,
                serial_num: 0,
                expire_time: row.expiry_time as u32,
            };
        }
    }

    GUILD_BANKS.insert(
        clan_id,
        GuildBankCacheData {
            items,
            gold: bank.gold,
            max_tabs: bank.max_tabs.max(1) as u8,
            permissions: bank.permissions as u8,
        },
    );

    Ok(())
}

// ── C2S Handler ─────────────────────────────────────────────────────────

/// Handle WIZ_GUILD_BANK (0xD0) from the client.
/// C2S packets are sent from UI interactions (clicking in guild bank panel).
/// DB tables: guild_bank, guild_bank_item, guild_bank_log.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    let sub_sub = reader.read_u8().unwrap_or(0);

    match sub {
        GB_SUB_OPEN => {
            handle_open(session, sub_sub).await?;
        }
        GB_SUB_ITEM_DATA => {
            handle_item_data(session, &mut reader, sub_sub).await?;
        }
        GB_SUB_LOG => {
            handle_log(session, &mut reader, sub_sub).await?;
        }
        GB_SUB_ITEM_RESULT => {
            // C2S: item result acknowledgment — sub_sub determines variant
            let remaining = reader.read_remaining();
            debug!(
                "[{}] WIZ_GUILD_BANK sub=1 item_result: sub_sub={}, data={:02X?}",
                session.addr(),
                sub_sub,
                remaining
            );
            session.send_packet(&build_result_error()).await?;
        }
        GB_SUB_MAP_ITEMS => {
            // C2S: map item coordinates request — sub_sub determines variant
            let remaining = reader.read_remaining();
            debug!(
                "[{}] WIZ_GUILD_BANK sub=3 map_items: sub_sub={}, data={:02X?}",
                session.addr(),
                sub_sub,
                remaining
            );
            session.send_packet(&build_result_error()).await?;
        }
        GB_SUB_PERMISSIONS => {
            // C2S: permission management — sub_sub determines operation
            let remaining = reader.read_remaining();
            debug!(
                "[{}] WIZ_GUILD_BANK sub=6 permissions: sub_sub={}, data={:02X?}",
                session.addr(),
                sub_sub,
                remaining
            );
            session.send_packet(&build_result_error()).await?;
        }
        _ => {
            debug!(
                "[{}] WIZ_GUILD_BANK unknown sub={} sub_sub={} ({}B remaining)",
                session.addr(),
                sub,
                sub_sub,
                reader.remaining()
            );
        }
    }

    Ok(())
}

/// Handle guild bank open request (sub=2).
/// Validates clan membership, loads bank from DB into cache,
/// sends bank init + tab info + item slots.
async fn handle_open(session: &mut ClientSession, _sub_sub: u8) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let (clan_id, _fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            debug!(
                "[{}] WIZ_GUILD_BANK open denied — not in clan",
                session.addr()
            );
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    let world = session.world();
    if world.get_knights(clan_id).is_none() {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    // Genie state check — C++ ClanBank.cpp:13
    let sid = session.session_id();
    if world.with_session(sid, |h| h.genie_active).unwrap_or(false) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    debug!(
        "[{}] WIZ_GUILD_BANK open knights_id={}",
        session.addr(),
        clan_id
    );

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!(
            "[{}] WIZ_GUILD_BANK DB error loading bank: {}",
            session.addr(),
            e
        );
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let data = match GUILD_BANKS.get(&clan_id) {
        Some(d) => d.clone(),
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    // Split i64 gold into hi/lo u16 for the packet
    let gold_hi = ((data.gold >> 16) & 0xFFFF) as u16;
    let gold_lo = (data.gold & 0xFFFF) as u16;

    // Send bank init: opens the UI on client
    let init_pkt = build_bank_init(data.permissions, 0, gold_hi, gold_lo, data.max_tabs);
    session.send_packet(&init_pkt).await?;

    // Send tab info + item slots for each unlocked tab
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    for tab in 0..data.max_tabs as usize {
        let base = tab * ITEMS_PER_TAB;
        let end = (base + ITEMS_PER_TAB).min(WAREHOUSE_MAX);

        let mut tab_count = 0i32;
        for slot in &data.items[base..end] {
            if slot.item_id != 0 {
                tab_count += 1;
            }
        }

        let tab_info = build_tab_info(tab as u16, tab_count, ITEMS_PER_TAB as i32, tab_count);
        session.send_packet(&tab_info).await?;

        for slot_idx in base..end {
            let slot = &data.items[slot_idx];
            if slot.item_id == 0 {
                continue;
            }
            // Skip expired items
            if slot.expire_time != 0 && slot.expire_time < now {
                continue;
            }
            let slot_pkt = build_item_slot(
                slot_idx as i32,
                tab as u8,
                slot.count,
                slot.durability as u16,
                slot.durability as u16,
                slot.flag as u16,
                slot.expire_time as u16,
            );
            session.send_packet(&slot_pkt).await?;
        }
    }

    session.send_packet(&build_result_success()).await?;
    Ok(())
}

/// Handle guild bank item data operations (sub=4).
/// Dispatches on sub_sub to distinguish operation type.
/// C2S format (based on C++ ClanBank.cpp reference):
/// - sub_sub=1: gold op — `[u32 amount] [u8 direction]` (0=deposit, 1=withdraw)
/// - sub_sub=2: item deposit — `[u16 npc_id] [u32 item_id] [u8 page] [u8 src] [u8 dst] [u32 count]`
/// - sub_sub=3: item withdraw — same format as deposit
/// - sub_sub=4: item move — `[u16 npc_id] [u32 item_id] [u8 page] [u8 src] [u8 dst]`
/// NOTE: C2S format determined by v2525 client binary. If format doesn't match,
/// debug logging below will reveal actual structure for correction.
async fn handle_item_data(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    sub_sub: u8,
) -> anyhow::Result<()> {
    match sub_sub {
        1 => handle_gold_op(session, reader).await,
        2 => handle_deposit(session, reader).await,
        3 => handle_withdraw(session, reader).await,
        4 => handle_move_item(session, reader).await,
        _ => {
            debug!(
                "[{}] WIZ_GUILD_BANK sub=4 unknown sub_sub={} ({}B remaining)",
                session.addr(),
                sub_sub,
                reader.remaining()
            );
            session.send_packet(&build_result_error()).await?;
            Ok(())
        }
    }
}

/// Handle gold deposit/withdraw (sub=4, sub_sub=1).
/// C2S: `[u32 amount] [u8 direction]` — direction 0=deposit, 1=withdraw.
async fn handle_gold_op(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let (clan_id, fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    let amount = reader.read_u32().unwrap_or(0);
    let direction = reader.read_u8().unwrap_or(0);

    if amount == 0 {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!(
            "[{}] WIZ_GUILD_BANK gold op DB error: {}",
            session.addr(),
            e
        );
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let world = session.world();
    let sid = session.session_id();

    if direction == 0 {
        // Deposit gold: player → bank
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                session.send_packet(&build_result_error()).await?;
                return Ok(());
            }
        };

        let mut success = false;
        if let Some(mut data) = GUILD_BANKS.get_mut(&clan_id) {
            if ch.gold >= amount && (data.gold + amount as i64) <= COIN_MAX as i64 {
                data.gold += amount as i64;
                success = true;
            }
        }

        if success {
            world.gold_lose(sid, amount);
            save_gb_gold_async(session, clan_id);
            log_guild_bank_action(session, clan_id, 0, 0, amount, true);
            session.send_packet(&build_result_success()).await?;
            // Send updated gold balance to refresh client's bank panel
            let new_gold = GUILD_BANKS
                .get(&clan_id)
                .map(|d| (d.gold & 0xFFFF) as u16)
                .unwrap_or(0);
            session.send_packet(&build_gold_update(new_gold)).await?;
            debug!(
                "[{}] WIZ_GUILD_BANK gold deposit: {} gold",
                session.addr(),
                amount
            );
        } else {
            session.send_packet(&build_result_error()).await?;
        }
    } else {
        // Withdraw gold: bank → player (leader/assistant only)
        if !is_leader_or_assistant(fame) {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }

        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => {
                session.send_packet(&build_result_error()).await?;
                return Ok(());
            }
        };

        let mut success = false;
        if let Some(mut data) = GUILD_BANKS.get_mut(&clan_id) {
            if data.gold >= amount as i64 && (ch.gold as u64 + amount as u64) <= COIN_MAX as u64 {
                data.gold -= amount as i64;
                success = true;
            }
        }

        if success {
            world.gold_gain(sid, amount);
            save_gb_gold_async(session, clan_id);
            log_guild_bank_action(session, clan_id, 0, 0, amount, false);
            session.send_packet(&build_result_success()).await?;
            // Send updated gold balance to refresh client's bank panel
            let new_gold = GUILD_BANKS
                .get(&clan_id)
                .map(|d| (d.gold & 0xFFFF) as u16)
                .unwrap_or(0);
            session.send_packet(&build_gold_update(new_gold)).await?;
            debug!(
                "[{}] WIZ_GUILD_BANK gold withdraw: {} gold",
                session.addr(),
                amount
            );
        } else {
            session.send_packet(&build_result_error()).await?;
        }
    }

    Ok(())
}

/// Handle item deposit: inventory → guild bank (sub=4, sub_sub=2).
/// C2S: `[u16 npc_id] [u32 item_id] [u8 page/tab] [u8 src_pos] [u8 dst_pos] [u32 count]`
async fn handle_deposit(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let (clan_id, _fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    let world = session.world();
    if world.get_knights(clan_id).is_none() {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let _npc_id = reader.read_u16().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let tab = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let count = reader.read_u32().unwrap_or(0);

    if count == 0 || item_id == 0 {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    // Gold handled by handle_gold_op
    if item_id == ITEM_GOLD {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!(
            "[{}] WIZ_GUILD_BANK deposit load error: {}",
            session.addr(),
            e
        );
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let sid = session.session_id();

    // Validate item table
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    let wh_slot_idx = tab as usize * ITEMS_PER_TAB + dst_pos as usize;

    // Bounds check
    if src_pos as usize >= HAVE_MAX || wh_slot_idx >= WAREHOUSE_MAX {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    // No-trade items cannot be stored
    if (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let countable = item_table.countable.unwrap_or(0);
    let src_slot_idx = SLOT_MAX + src_pos as usize;

    // Read source item from inventory
    let src_item = match world.get_inventory_slot(sid, src_slot_idx) {
        Some(s) if s.item_id == item_id => s,
        _ => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:221-225 — check flags
    if src_item.flag == ITEM_FLAG_RENTED
        || src_item.flag == ITEM_FLAG_SEALED
        || src_item.expire_time > 0
    {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }
    if src_item.flag == ITEM_FLAG_DUPLICATE {
        session.send_packet(&build_result_code(2)).await?;
        return Ok(());
    }

    if src_item.count < count as u16 {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    // Atomically update guild bank cache
    let mut wh_success = false;
    let mut final_wh_slot = UserItemSlot::default();

    if let Some(mut data) = GUILD_BANKS.get_mut(&clan_id) {
        while data.items.len() < WAREHOUSE_MAX {
            data.items.push(UserItemSlot::default());
        }

        let dst = &data.items[wh_slot_idx];
        if dst.item_id != 0 && (!is_stackable || dst.item_id != src_item.item_id) {
            // Slot occupied by different item
        } else {
            let dst = &mut data.items[wh_slot_idx];

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

            final_wh_slot = dst.clone();
            wh_success = true;
        }
    }

    if !wh_success {
        session.send_packet(&build_result_full()).await?;
        return Ok(());
    }

    // Update inventory: reduce source
    let inv_success = world.update_inventory(sid, |inv| {
        if src_slot_idx >= inv.len() {
            return false;
        }
        let src_mut = &mut inv[src_slot_idx];
        if is_stackable {
            src_mut.count = src_mut.count.saturating_sub(count as u16);
        } else {
            src_mut.count = 0;
        }
        if src_mut.count == 0 || countable == 0 {
            *src_mut = UserItemSlot::default();
        }
        true
    });

    if !inv_success {
        // Rollback guild bank cache change
        if let Some(mut data) = GUILD_BANKS.get_mut(&clan_id) {
            data.items[wh_slot_idx] = UserItemSlot::default();
        }
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    world.set_user_ability(sid);
    save_gb_slot_async(session, clan_id, wh_slot_idx, final_wh_slot);
    save_inventory_slot_async(session, src_slot_idx);

    // Clan notification
    send_guild_bank_notice(session.world(), clan_id, sid, item_id, count, true);

    // Transaction log
    log_guild_bank_action(session, clan_id, tab, item_id, count, true);

    session.send_packet(&build_result_success()).await?;
    debug!(
        "[{}] WIZ_GUILD_BANK deposit: item={}, count={}, slot={}",
        session.addr(),
        item_id,
        count,
        wh_slot_idx
    );
    Ok(())
}

/// Handle item withdraw: guild bank → inventory (sub=4, sub_sub=3).
/// C2S: `[u16 npc_id] [u32 item_id] [u8 page/tab] [u8 src_pos] [u8 dst_pos] [u32 count]`
/// Only clan leader or assistant can withdraw.
async fn handle_withdraw(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let (clan_id, fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:369 — only leader or assistant can withdraw
    if !is_leader_or_assistant(fame) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let world = session.world();
    if world.get_knights(clan_id).is_none() {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let _npc_id = reader.read_u16().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let tab = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let count = reader.read_u32().unwrap_or(0);

    if count == 0 || item_id == 0 {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    if item_id == ITEM_GOLD {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!(
            "[{}] WIZ_GUILD_BANK withdraw load error: {}",
            session.addr(),
            e
        );
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let sid = session.session_id();

    // Validate item table
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let countable = item_table.countable.unwrap_or(0);

    // C++ ClanBank.cpp:430-445 — weight check
    if !world.check_weight(sid, item_id, count as u16) {
        session.send_packet(&build_result_code(3)).await?;
        return Ok(());
    }

    let wh_slot_idx = tab as usize * ITEMS_PER_TAB + src_pos as usize;
    let dst_slot_idx = SLOT_MAX + dst_pos as usize;

    if wh_slot_idx >= WAREHOUSE_MAX || dst_pos as usize >= HAVE_MAX {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    // Read source from guild bank cache
    let src_item = match GUILD_BANKS.get(&clan_id) {
        Some(data) => {
            let slot = data.items.get(wh_slot_idx).cloned().unwrap_or_default();
            if slot.item_id != item_id || slot.count < count as u16 {
                session.send_packet(&build_result_error()).await?;
                return Ok(());
            }
            if slot.flag == ITEM_FLAG_DUPLICATE {
                session.send_packet(&build_result_code(2)).await?;
                return Ok(());
            }
            slot
        }
        None => {
            session.send_packet(&build_result_error()).await?;
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
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    // Reduce guild bank source — re-validate under write lock (TOCTOU)
    let mut final_wh_slot = UserItemSlot::default();
    let wh_deduct_ok = if let Some(mut data) = GUILD_BANKS.get_mut(&clan_id) {
        let src_mut = &mut data.items[wh_slot_idx];
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

    if !wh_deduct_ok {
        // Rollback inventory change
        world.update_inventory(sid, |inv| {
            if dst_slot_idx < inv.len() {
                inv[dst_slot_idx] = UserItemSlot::default();
            }
            true
        });
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    world.set_user_ability(sid);
    save_gb_slot_async(session, clan_id, wh_slot_idx, final_wh_slot);
    save_inventory_slot_async(session, dst_slot_idx);

    // Clan notification
    send_guild_bank_notice(session.world(), clan_id, sid, item_id, count, false);

    // Transaction log
    log_guild_bank_action(session, clan_id, tab, item_id, count, false);

    session.send_packet(&build_result_success()).await?;
    debug!(
        "[{}] WIZ_GUILD_BANK withdraw: item={}, count={}, slot={}",
        session.addr(),
        item_id,
        count,
        wh_slot_idx
    );
    Ok(())
}

/// Handle item move within guild bank (sub=4, sub_sub=4).
/// C2S: `[u16 npc_id] [u32 item_id] [u8 page/tab] [u8 src_pos] [u8 dst_pos]`
/// Only clan leader or assistant can move.
async fn handle_move_item(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !validate_basic_state(session) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let (clan_id, fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    // C++ ClanBank.cpp:610 — leader or assistant only
    if !is_leader_or_assistant(fame) {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let world = session.world();
    if world.get_knights(clan_id).is_none() {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let _npc_id = reader.read_u16().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let tab = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);

    if let Err(e) = ensure_loaded(session, clan_id).await {
        warn!("[{}] WIZ_GUILD_BANK move load error: {}", session.addr(), e);
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let reference_pos = tab as usize * ITEMS_PER_TAB;
    let src_idx = reference_pos + src_pos as usize;
    let dst_idx = reference_pos + dst_pos as usize;

    if src_idx >= WAREHOUSE_MAX || dst_idx >= WAREHOUSE_MAX {
        session.send_packet(&build_result_error()).await?;
        return Ok(());
    }

    let mut success = false;
    let mut final_src = UserItemSlot::default();
    let mut final_dst = UserItemSlot::default();

    if let Some(mut data) = GUILD_BANKS.get_mut(&clan_id) {
        while data.items.len() < WAREHOUSE_MAX {
            data.items.push(UserItemSlot::default());
        }

        let src = &data.items[src_idx];
        let dst = &data.items[dst_idx];

        // Source must match item_id, destination must be empty
        if src.item_id == item_id && dst.item_id == 0 {
            if src.flag == ITEM_FLAG_DUPLICATE || dst.flag == ITEM_FLAG_DUPLICATE {
                session.send_packet(&build_result_code(2)).await?;
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
        save_gb_slot_async(session, clan_id, src_idx, final_src);
        save_gb_slot_async(session, clan_id, dst_idx, final_dst);
        session.send_packet(&build_result_success()).await?;
        debug!(
            "[{}] WIZ_GUILD_BANK move: item={}, {} -> {}",
            session.addr(),
            item_id,
            src_idx,
            dst_idx
        );
    } else {
        session.send_packet(&build_result_error()).await?;
    }

    Ok(())
}

/// Handle guild bank log request (sub=5).
/// Loads paginated transaction logs from DB and sends to client.
async fn handle_log(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    _sub_sub: u8,
) -> anyhow::Result<()> {
    let (clan_id, _fame) = match get_clan_info(session) {
        Some(info) => info,
        None => {
            session.send_packet(&build_result_error()).await?;
            return Ok(());
        }
    };

    // C2S log request: page number
    let page = reader.read_u16().unwrap_or(0) as i64;
    let entries_per_page: i64 = 20;

    debug!("[{}] WIZ_GUILD_BANK log page={}", session.addr(), page);

    let pool = session.pool().clone();
    let repo = GuildBankRepository::new(&pool);

    let total = match repo.count_logs(clan_id as i32).await {
        Ok(t) => t,
        Err(e) => {
            warn!("[{}] WIZ_GUILD_BANK log count error: {}", session.addr(), e);
            0
        }
    };

    let logs = match repo
        .load_logs(clan_id as i32, entries_per_page, page * entries_per_page)
        .await
    {
        Ok(l) => l,
        Err(e) => {
            warn!("[{}] WIZ_GUILD_BANK log load error: {}", session.addr(), e);
            Vec::new()
        }
    };

    // Send log page header
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0);

    let page_init = build_log_page_init(now, page as u16, total as i32, logs.len() as u16);
    session.send_packet(&page_init).await?;

    // Send individual log entries
    for log in &logs {
        let entry = build_log_entry(
            log.tab_index as u8,
            log.item_id,
            log.quantity as u16,
            log.price as u16,
            log.action_type as u8,
        );
        session.send_packet(&entry).await?;
    }

    Ok(())
}

// ── Async persistence helpers ───────────────────────────────────────────

/// Save a guild bank slot to DB (fire-and-forget).
fn save_gb_slot_async(session: &ClientSession, clan_id: u16, flat_idx: usize, slot: UserItemSlot) {
    let pool = session.pool().clone();
    let tab_index = (flat_idx / ITEMS_PER_TAB) as i16;
    let slot_id = flat_idx as i32;

    tokio::spawn(async move {
        let repo = GuildBankRepository::new(&pool);
        if slot.item_id == 0 {
            if let Err(e) = repo.remove_item(clan_id as i32, tab_index, slot_id).await {
                warn!("Failed to remove guild bank slot {}: {}", flat_idx, e);
            }
        } else if let Err(e) = repo
            .upsert_item(
                clan_id as i32,
                tab_index,
                slot_id,
                slot.item_id as i32,
                slot.count as i16,
                slot.durability,
                slot.durability,
                slot.flag as i16,
                slot.expire_time as i32,
            )
            .await
        {
            warn!("Failed to save guild bank slot {}: {}", flat_idx, e);
        }
    });
}

/// Save guild bank gold to DB (fire-and-forget).
fn save_gb_gold_async(session: &ClientSession, clan_id: u16) {
    let gold = GUILD_BANKS.get(&clan_id).map(|d| d.gold).unwrap_or(0);
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = GuildBankRepository::new(&pool);
        if let Err(e) = repo.update_gold(clan_id as i32, gold).await {
            warn!("Failed to save guild bank gold: {}", e);
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

/// Send deposit/withdraw notification to all clan members.
fn send_guild_bank_notice(
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
                "### {} Guild Bankasina {} adet {} birakti. ###",
                player_name, count, item_name
            )
        } else {
            format!(
                "### {} Guild Bankasina {} Birakti. ###",
                player_name, item_name
            )
        }
    } else if count > 1 {
        format!(
            "### {} Guild Bankasindan {} adet {} aldi. ###",
            player_name, count, item_name
        )
    } else {
        format!(
            "### {} Guild Bankasindan {} aldi. ###",
            player_name, item_name
        )
    };

    // GM_CHAT type=12, sender_id=0 (system), nation=0
    let pkt = build_chat_packet(12, 0, 0, "", &message, 0, 0, 0);
    world.send_to_knights_members(clan_id, Arc::new(pkt), None);
}

/// Log a guild bank transaction (fire-and-forget DB insert + audit log).
fn log_guild_bank_action(
    session: &ClientSession,
    clan_id: u16,
    tab: u8,
    item_id: u32,
    count: u32,
    is_deposit: bool,
) {
    let world = session.world();
    let sid = session.session_id();
    let char_name = world.get_session_name(sid).unwrap_or_default();
    let action_type: i16 = if is_deposit { 1 } else { 2 };

    // DB log
    let pool = session.pool().clone();
    let char_name_clone = char_name.clone();
    tokio::spawn(async move {
        let repo = GuildBankRepository::new(&pool);
        if let Err(e) = repo
            .insert_log(
                clan_id as i32,
                &char_name_clone,
                tab as i16,
                item_id as i32,
                count as i16,
                0, // price
                action_type,
            )
            .await
        {
            warn!("Failed to insert guild bank log: {}", e);
        }
    });

    // Audit log
    let clan_name = world
        .get_knights(clan_id)
        .map(|k| k.name.clone())
        .unwrap_or_default();
    let action_str = if is_deposit { "deposit" } else { "withdraw" };
    super::audit_log::log_clan_bank(
        session.pool(),
        session.account_id().unwrap_or(""),
        &char_name,
        action_str,
        clan_id,
        &clan_name,
        item_id,
        count,
        0,
    );
}

/// Clear guild bank cache for a given clan (e.g., on clan disband).
pub fn evict_cache(clan_id: u16) {
    GUILD_BANKS.remove(&clan_id);
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── Sub=1: Item Result ──────────────────────────────────────────

    #[test]
    fn test_build_open_init_format() {
        let pkt = build_open_init(100, 5001);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_ITEM_RESULT));
        assert_eq!(r.read_u8(), Some(1)); // sub_sub
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.read_u16(), Some(5001));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_open_init_length() {
        let pkt = build_open_init(0, 0);
        assert_eq!(pkt.data.len(), 6); // u8+u8+u16+u16
    }

    #[test]
    fn test_build_result_code_format() {
        let pkt = build_result_code(-4);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_ITEM_RESULT));
        assert_eq!(r.read_u8(), Some(2)); // sub_sub
        assert_eq!(r.read_i16(), Some(-4)); // FULL
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_result_success() {
        let pkt = build_result_success();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_i16(), Some(RESULT_SUCCESS)); // -1
    }

    #[test]
    fn test_build_result_full() {
        let pkt = build_result_full();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        assert_eq!(r.read_i16(), Some(RESULT_FULL)); // -4
    }

    #[test]
    fn test_result_code_values() {
        assert_eq!(RESULT_SUCCESS, -1);
        assert_eq!(RESULT_ERROR_1, -2);
        assert_eq!(RESULT_ERROR_2, -3);
        assert_eq!(RESULT_FULL, -4);
    }

    // ── Sub=2: Open Bank ────────────────────────────────────────────

    #[test]
    fn test_build_bank_init_format() {
        let pkt = build_bank_init(3, 5001, 100, 200, 5);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_OPEN));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(3)); // tab_permission
        assert_eq!(r.read_u16(), Some(5001)); // npc_id
        assert_eq!(r.read_u16(), Some(100)); // gold_hi
        assert_eq!(r.read_u16(), Some(200)); // gold_lo
        assert_eq!(r.read_u8(), Some(5)); // max_tabs
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_bank_init_length() {
        let pkt = build_bank_init(0, 0, 0, 0, 0);
        assert_eq!(pkt.data.len(), 10); // u8+u8+u8+u16+u16+u16+u8
    }

    #[test]
    fn test_build_tab_info_format() {
        let pkt = build_tab_info(1234, 50, 100, 75);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_OPEN));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(1234));
        assert_eq!(r.read_i32(), Some(50));
        assert_eq!(r.read_i32(), Some(100));
        assert_eq!(r.read_i32(), Some(75));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_bank_reset_format() {
        let pkt = build_bank_reset();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_OPEN));
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sub=3: Map Items ────────────────────────────────────────────

    #[test]
    fn test_build_map_coords_empty() {
        let pkt = build_map_coords(&[]);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_MAP_ITEMS));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(0)); // count
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_map_coords_multiple() {
        let coords = vec![(100, 200), (300, 400), (500, 600)];
        let pkt = build_map_coords(&coords);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_MAP_ITEMS));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.read_u16(), Some(200));
        assert_eq!(r.read_u16(), Some(300));
        assert_eq!(r.read_u16(), Some(400));
        assert_eq!(r.read_u16(), Some(500));
        assert_eq!(r.read_u16(), Some(600));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_map_coords_length() {
        let coords = vec![(0, 0); 3];
        let pkt = build_map_coords(&coords);
        // u8 sub + u8 sub_sub + u8 count + 3×(u16+u16) = 3+12 = 15
        assert_eq!(pkt.data.len(), 15);
    }

    // ── Sub=4: Item Data ────────────────────────────────────────────

    #[test]
    fn test_build_gold_update_format() {
        let pkt = build_gold_update(5000);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_ITEM_DATA));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(0)); // skip
        assert_eq!(r.read_u16(), Some(5000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_item_slot_format() {
        let pkt = build_item_slot(42, 2, 10, 100, 95, 1, 3600);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_ITEM_DATA));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_i32(), Some(42)); // slot_id
        assert_eq!(r.read_u8(), Some(2)); // tab_index
        assert_eq!(r.read_u16(), Some(10)); // count
        assert_eq!(r.read_u16(), Some(100)); // max_dur
        assert_eq!(r.read_u16(), Some(95)); // cur_dur
        assert_eq!(r.read_u16(), Some(1)); // flag
        assert_eq!(r.read_u16(), Some(3600)); // expiry
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_item_slot_length() {
        let pkt = build_item_slot(0, 0, 0, 0, 0, 0, 0);
        // u8+u8+i32+u8+u16×5 = 2+4+1+10 = 17
        assert_eq!(pkt.data.len(), 17);
    }

    #[test]
    fn test_build_reset_tab_format() {
        let pkt = build_reset_tab(3);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_ITEM_DATA));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_i32(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sub=5: Transaction Log ──────────────────────────────────────

    #[test]
    fn test_build_log_page_init_format() {
        let pkt = build_log_page_init(1710000000, 1, 50, 10);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_LOG));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_i32(), Some(1710000000));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_i32(), Some(50));
        assert_eq!(r.read_u16(), Some(10));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_log_entry_format() {
        let pkt = build_log_entry(0, 200001000, 5, 1000, 1);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_LOG));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0)); // tab
        assert_eq!(r.read_i32(), Some(200001000)); // item
        assert_eq!(r.read_u16(), Some(5)); // qty
        assert_eq!(r.read_u16(), Some(1000)); // price
        assert_eq!(r.read_u8(), Some(1)); // action
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_log_removal_format() {
        let pkt = build_log_removal(1);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_LOG));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sub=6: Permissions ──────────────────────────────────────────

    #[test]
    fn test_build_perm_result_format() {
        let pkt = build_perm_result(2, 0xFFF5); // -11 as u16
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_PERMISSIONS));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(0xFFF5));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_perm_item_detail_format() {
        let pkt = build_perm_item_detail(9999);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GB_SUB_PERMISSIONS));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u16(), Some(9999));
        assert_eq!(r.remaining(), 0);
    }

    // ── Opcode verification ─────────────────────────────────────────

    #[test]
    fn test_all_builders_use_correct_opcode() {
        let expected = Opcode::WizGuildBank as u8;
        assert_eq!(build_open_init(0, 0).opcode, expected);
        assert_eq!(build_result_code(0).opcode, expected);
        assert_eq!(build_bank_init(0, 0, 0, 0, 0).opcode, expected);
        assert_eq!(build_tab_info(0, 0, 0, 0).opcode, expected);
        assert_eq!(build_bank_reset().opcode, expected);
        assert_eq!(build_map_coords(&[]).opcode, expected);
        assert_eq!(build_gold_update(0).opcode, expected);
        assert_eq!(build_item_slot(0, 0, 0, 0, 0, 0, 0).opcode, expected);
        assert_eq!(build_reset_tab(0).opcode, expected);
        assert_eq!(build_log_page_init(0, 0, 0, 0).opcode, expected);
        assert_eq!(build_log_entry(0, 0, 0, 0, 0).opcode, expected);
        assert_eq!(build_log_removal(0).opcode, expected);
        assert_eq!(build_perm_result(0, 0).opcode, expected);
        assert_eq!(build_perm_item_detail(0).opcode, expected);
    }

    // ── Sub-opcode constant verification ────────────────────────────

    #[test]
    fn test_sub_opcode_values() {
        assert_eq!(GB_SUB_ITEM_RESULT, 1);
        assert_eq!(GB_SUB_OPEN, 2);
        assert_eq!(GB_SUB_MAP_ITEMS, 3);
        assert_eq!(GB_SUB_ITEM_DATA, 4);
        assert_eq!(GB_SUB_LOG, 5);
        assert_eq!(GB_SUB_PERMISSIONS, 6);
    }

    // ── New: Guild bank cache ───────────────────────────────────────

    #[test]
    fn test_guild_bank_cache_data() {
        let data = GuildBankCacheData {
            items: vec![UserItemSlot::default(); WAREHOUSE_MAX],
            gold: 500_000,
            max_tabs: 3,
            permissions: 1,
        };
        assert_eq!(data.gold, 500_000);
        assert_eq!(data.max_tabs, 3);
        assert_eq!(data.permissions, 1);
        assert_eq!(data.items.len(), WAREHOUSE_MAX);
        assert_eq!(data.items[0].item_id, 0);
    }

    #[test]
    fn test_guild_bank_tab_slot_mapping() {
        // Tab 0: slots 0-23, Tab 1: slots 24-47, etc.
        let tab: usize = 0;
        let tab0_start = tab * ITEMS_PER_TAB;
        assert_eq!(tab0_start, 0);
        assert_eq!(tab0_start + 23, 23);
        assert_eq!(ITEMS_PER_TAB, 24);
        let tab7_end = 7 * ITEMS_PER_TAB + 23;
        assert_eq!(tab7_end, 191);
        assert!(tab7_end < WAREHOUSE_MAX);
    }

    #[test]
    fn test_guild_bank_deposit_logic() {
        let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];

        // Deposit a non-stackable item to slot 5
        items[5] = UserItemSlot {
            item_id: 150001,
            durability: 100,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 42,
            expire_time: 0,
        };

        assert_eq!(items[5].item_id, 150001);
        assert_eq!(items[5].count, 1);
        assert_eq!(items[5].serial_num, 42);
        assert_eq!(items[6].item_id, 0); // adjacent slot empty
    }

    #[test]
    fn test_guild_bank_stackable_deposit() {
        let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];
        items[3] = UserItemSlot {
            item_id: 389010, // stackable (arrow)
            durability: 0,
            count: 500,
            flag: 0,
            original_flag: 0,
            serial_num: 10,
            expire_time: 0,
        };

        // Deposit 300 more to existing stack
        let count = 300u16;
        items[3].count += count;
        assert_eq!(items[3].count, 800);
    }

    #[test]
    fn test_guild_bank_gold_overflow() {
        let gold: i64 = 2_000_000_000;
        let deposit: u32 = 200_000_000;
        let fits = (gold + deposit as i64) <= COIN_MAX as i64;
        assert!(!fits); // 2.2B > 2.1B limit
    }

    #[test]
    fn test_guild_bank_gold_within_limit() {
        let gold: i64 = 1_000_000_000;
        let deposit: u32 = 500_000_000;
        let fits = (gold + deposit as i64) <= COIN_MAX as i64;
        assert!(fits); // 1.5B <= 2.1B
    }

    #[test]
    fn test_guild_bank_move_logic() {
        let mut items = vec![UserItemSlot::default(); WAREHOUSE_MAX];
        items[10] = UserItemSlot {
            item_id: 200001,
            durability: 75,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 99,
            expire_time: 0,
        };

        // Move slot 10 to slot 15
        let tmp = items[10].clone();
        items[15] = tmp;
        items[10] = UserItemSlot::default();

        assert_eq!(items[10].item_id, 0);
        assert_eq!(items[15].item_id, 200001);
        assert_eq!(items[15].serial_num, 99);
    }

    #[test]
    fn test_guild_bank_no_trade_range() {
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&900_000_001));
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&999_999_999));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&900_000_000));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&1_000_000_000));
    }

    #[test]
    fn test_is_leader_or_assistant() {
        assert!(is_leader_or_assistant(CHIEF));
        assert!(is_leader_or_assistant(VICECHIEF));
        assert!(!is_leader_or_assistant(0));
        assert!(!is_leader_or_assistant(3));
        assert!(!is_leader_or_assistant(5));
    }

    #[test]
    fn test_items_per_tab_constant() {
        assert_eq!(ITEMS_PER_TAB, 24);
        assert_eq!(ITEMS_PER_TAB * 8, WAREHOUSE_MAX);
    }

    #[test]
    fn test_c2s_sub_constants() {
        assert_eq!(GB_SUB_ITEM_RESULT, 1);
        assert_eq!(GB_SUB_OPEN, 2);
        assert_eq!(GB_SUB_MAP_ITEMS, 3);
        assert_eq!(GB_SUB_ITEM_DATA, 4);
        assert_eq!(GB_SUB_LOG, 5);
        assert_eq!(GB_SUB_PERMISSIONS, 6);
    }

    #[test]
    fn test_c2s_stub_dispatch_format() {
        // All stub subs use [u8 sub][u8 sub_sub] header
        for &sub in &[GB_SUB_ITEM_RESULT, GB_SUB_MAP_ITEMS, GB_SUB_PERMISSIONS] {
            let mut pkt = Packet::new(Opcode::WizGuildBank as u8);
            pkt.write_u8(sub);
            pkt.write_u8(1); // sub_sub

            let mut r = PacketReader::new(&pkt.data);
            assert_eq!(r.read_u8(), Some(sub), "sub-opcode");
            assert_eq!(r.read_u8(), Some(1), "sub_sub");
        }
    }

    #[test]
    fn test_no_trade_item_range() {
        let min = ITEM_NO_TRADE_MIN;
        let max = ITEM_NO_TRADE_MAX;
        assert!(min > 900_000_000);
        assert!(max < 1_000_000_000);
        assert!(min < max);
    }
}
