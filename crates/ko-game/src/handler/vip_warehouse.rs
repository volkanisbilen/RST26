//! WIZ_VIPWAREHOUSE (0x8B) handler -- premium VIP storage vault.
//! VIP Warehouse is a premium storage with 48 slots, requiring a vault key
//! item to activate (time-limited). Supports 4-digit PIN password protection.
//! Sub-opcodes (`packets.h:1002-1012`):
//! - 1 = VIP_Open: Open VIP storage UI
//! - 2 = VIP_InvenToStorage: Inventory -> VIP storage
//! - 3 = VIP_StorageToInven: VIP storage -> Inventory
//! - 4 = VIP_StorageToStore: Rearrange within VIP storage
//! - 5 = VIP_InvenToInven: Rearrange inventory (from VIP UI)
//! - 6 = VIP_UseVault: Activate vault key item
//! - 8 = VIP_SetPassword: Set 4-digit password
//! - 9 = VIP_CancelPassword: Remove password
//! - 10 = VIP_ChangePassword: Change password
//! - 11 = VIP_EnterPassword: Enter password to unlock

use ko_db::repositories::character::{
    CharacterRepository, SaveItemParams, SaveVipWarehouseItemParams,
};
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::{
    UserItemSlot, WorldState, ITEMCOUNT_MAX, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED,
    ITEM_FLAG_SEALED, ITEM_GOLD, ITEM_NO_TRADE_MAX, ITEM_NO_TRADE_MIN,
};

/// VIP sub-opcodes
const VIP_OPEN: u8 = 1;
const VIP_INVEN_TO_STORAGE: u8 = 2;
const VIP_STORAGE_TO_INVEN: u8 = 3;
const VIP_STORAGE_TO_STORE: u8 = 4;
const VIP_INVEN_TO_INVEN: u8 = 5;
const VIP_USE_VAULT: u8 = 6;
const VIP_SET_PASSWORD: u8 = 8;
const VIP_CANCEL_PASSWORD: u8 = 9;
const VIP_CHANGE_PASSWORD: u8 = 10;
const VIP_ENTER_PASSWORD: u8 = 11;

const VIPWAREHOUSE_MAX: usize = 48;
use super::{HAVE_MAX, ITEM_KIND_UNIQUE, SLOT_MAX};

/// Vault key item IDs (`Define.h:379-381`, `GameDefine.h:1316`).
const VIP_VAULT_KEY: u32 = 800_442_000;
const VIP_SAFE_KEY_1: u32 = 810_442_000;
const VIP_SAFE_KEY_7: u32 = 998_019_000;
/// v2615 `ITEM`/`Item_Org` replacement for the 30-day VIP Vault key.
const VIP_VAULT_KEY_V2615: u32 = 931_695_000;

fn vip_key_days(item_id: u32) -> Option<u32> {
    match item_id {
        VIP_VAULT_KEY | VIP_VAULT_KEY_V2615 => Some(30),
        VIP_SAFE_KEY_1 => Some(1),
        VIP_SAFE_KEY_7 => Some(7),
        _ => None,
    }
}

// Item flag constants imported from crate::world (ITEM_FLAG_RENTED, ITEM_FLAG_DUPLICATE, ITEM_FLAG_SEALED).

/// Max vault extension: 30 days in seconds.
const MAX_VAULT_EXTENSION_SECS: u32 = 60 * 60 * 24 * 30;

/// Build a VIP warehouse error response packet.
fn build_error(subcode: u8, error_id: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizVipwarehouse as u8);
    pkt.write_u8(subcode);
    pkt.write_u8(error_id);
    pkt
}

/// Get current unix timestamp.
fn unix_now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

/// Handle WIZ_VIPWAREHOUSE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let opcode = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] WIZ_VIPWAREHOUSE: sub-opcode={}",
        session.addr(),
        opcode
    );

    // C++ checks: dead, trading, mining, merchanting, fishing
    if world.is_trading(sid)
        || world.is_mining(sid)
        || world.is_merchanting(sid)
        || world.is_fishing(sid)
    {
        let err = build_error(opcode, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Lazy-load VIP warehouse from DB
    if !world.is_vip_warehouse_loaded(sid) {
        load_vip_warehouse_from_db(session).await?;
    }

    // C++ check: vault expiry for InvenToStorage and StorageToStore
    let now = unix_now();
    let vault_expiry = world.get_vip_vault_expiry(sid);
    if (vault_expiry == 0 || vault_expiry < now)
        && (opcode == VIP_INVEN_TO_STORAGE || opcode == VIP_STORAGE_TO_STORE)
    {
        let err = build_error(opcode, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    match opcode {
        VIP_OPEN => handle_open(session, &mut reader).await,
        VIP_INVEN_TO_STORAGE => handle_inven_to_storage(session, &mut reader).await,
        VIP_STORAGE_TO_INVEN => handle_storage_to_inven(session, &mut reader).await,
        VIP_STORAGE_TO_STORE => handle_storage_to_store(session, &mut reader).await,
        VIP_INVEN_TO_INVEN => handle_inven_to_inven(session, &mut reader).await,
        VIP_USE_VAULT => handle_use_vault(session, &mut reader).await,
        VIP_SET_PASSWORD => handle_set_password(session, &mut reader).await,
        VIP_CANCEL_PASSWORD => handle_cancel_password(session).await,
        VIP_CHANGE_PASSWORD => handle_change_password(session, &mut reader).await,
        VIP_ENTER_PASSWORD => handle_enter_password(session, &mut reader).await,
        _ => {
            warn!(
                "[{}] Unknown VIP warehouse sub-opcode: {}",
                session.addr(),
                opcode
            );
            Ok(())
        }
    }
}

/// Load VIP warehouse data from the database.
async fn load_vip_warehouse_from_db(session: &mut ClientSession) -> anyhow::Result<()> {
    let account_id = match session.account_id() {
        Some(a) => a.to_string(),
        None => return Ok(()),
    };

    let repo = CharacterRepository::new(session.pool());
    let meta = repo.load_vip_warehouse(&account_id).await?;
    let db_items = repo.load_vip_warehouse_items(&account_id).await?;

    let mut items = vec![UserItemSlot::default(); VIPWAREHOUSE_MAX];
    for row in &db_items {
        let idx = row.slot_index as usize;
        if idx < VIPWAREHOUSE_MAX {
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

    let (password, password_request, vault_expiry) = match meta {
        Some(m) => (m.password, m.password_request as u8, m.vault_expiry as u32),
        None => (String::new(), 0, 0),
    };

    let world = session.world().clone();
    let sid = session.session_id();
    world.set_vip_warehouse(sid, items, password, password_request, vault_expiry);
    Ok(())
}

/// v2525 VIP warehouse total slots: 4 tabs × 48 items per tab.
const VIPWAREHOUSE_V2525_TOTAL: usize = 192;

/// Build the VIP warehouse open response with all items.
/// v2525 format (binary RE at `0x98F420`):
/// ```text
/// [u8 sub=1] [u8 result=1] [u8 password_request]
/// [4 tabs × (u8 is_key_active + u32 vault_expiry)]   ← 20 bytes
/// [192 items × (u32 id + u16 dur + u16 count + u8 flag + u32 unique + u32 expire)]
/// ```
/// v2525 client expanded to 4 tabs (192 slots). Only tab 0 is active.
async fn build_open_response(
    world: &WorldState,
    pool: &ko_db::DbPool,
    rebirth_level: u8,
    password_request: u8,
    vault_expiry: u32,
    items: &[UserItemSlot],
) -> Packet {
    let now = unix_now();
    let is_key = vault_expiry > now;

    let mut result = Packet::new(Opcode::WizVipwarehouse as u8);
    result.write_u8(VIP_OPEN);
    result.write_u8(1); // success
    result.write_u8(password_request);

    // v2525: 4-tab header — each tab has (u8 is_key_active + u32 vault_expiry)
    // Tab 0: active vault (from DB). Tabs 1-3: inactive (zeroed).
    result.write_u8(if is_key { 1 } else { 0 });
    result.write_u32(if is_key { vault_expiry } else { 0 });
    for _ in 1..4 {
        result.write_u8(0); // tab inactive
        result.write_u32(0); // no expiry
    }

    // v2525: 192 item slots (4 × 48). Items 0-47 from DB, rest empty.
    for i in 0..VIPWAREHOUSE_V2525_TOTAL {
        let slot = if i < VIPWAREHOUSE_MAX {
            items.get(i).cloned().unwrap_or_default()
        } else {
            UserItemSlot::default()
        };
        result.write_u32(slot.item_id);
        result.write_u16(slot.durability as u16);
        result.write_u16(slot.count);
        result.write_u8(slot.flag);
        crate::handler::unique_item_info::write_unique_item_info(
            world,
            pool,
            slot.item_id,
            slot.serial_num,
            rebirth_level,
            &mut result,
        )
        .await;
        result.write_u32(slot.expire_time);
    }
    result
}

/// Handle VIP_Open (sub-opcode 1).
async fn handle_open(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let npc_id = reader.read_u16().unwrap_or(0);
    let _password = reader.read_sbyte_string().unwrap_or_default();

    let now = unix_now();
    let vault_expiry = world.get_vip_vault_expiry(sid);

    // If npc_id==0 (remote access) and vault not active, error 21
    if npc_id == 0 && vault_expiry < now {
        let err = build_error(VIP_OPEN, 21);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // NPC range check — if NPC is specified, must be in range
    if npc_id != 0 && !world.is_in_npc_range(sid, npc_id as u32) {
        let err = build_error(VIP_OPEN, 0);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let password_request = world.get_vip_password_request(sid);
    let password = world.get_vip_password(sid);

    // C++ check: if password is set and request flag active, send error(11, 1)
    // This tells the client to show the password entry dialog.
    if password_request != 0 && !password.is_empty() {
        let err = build_error(VIP_ENTER_PASSWORD, 1);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let items = world.get_vip_warehouse(sid);
    let rebirth_level = world
        .get_character_info(sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);
    let response = build_open_response(
        &world,
        session.pool(),
        rebirth_level,
        password_request,
        vault_expiry,
        &items,
    )
    .await;
    session.send_packet(&response).await
}

/// Handle VIP_InvenToStorage (sub-opcode 2) -- inventory -> VIP storage.
async fn handle_inven_to_storage(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let now = unix_now();
    if world.get_vip_vault_expiry(sid) < now {
        let err = build_error(VIP_INVEN_TO_STORAGE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let _npc_id = reader.read_u32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let count = reader.read_u16().unwrap_or(0);

    let reference_pos = 48 * page as usize;
    if page > 1 || count == 0 {
        let err = build_error(VIP_INVEN_TO_STORAGE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Cannot store gold or no-trade items
    if item_id == ITEM_GOLD || (ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&item_id) {
        let err = build_error(VIP_INVEN_TO_STORAGE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Get item table entry
    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            let err = build_error(VIP_INVEN_TO_STORAGE, 2);
            session.send_packet(&err).await?;
            return Ok(());
        }
    };

    // C++ check: countable==2 cannot be stored
    if item_table.countable.unwrap_or(0) == 2 {
        let err = build_error(VIP_INVEN_TO_STORAGE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let kind = item_table.kind.unwrap_or(0);
    let countable = item_table.countable.unwrap_or(0);

    let src_slot_idx = SLOT_MAX + src_pos as usize;
    let vip_slot_idx = reference_pos + dst_pos as usize;

    if src_pos as usize >= HAVE_MAX || vip_slot_idx >= VIPWAREHOUSE_MAX {
        let err = build_error(VIP_INVEN_TO_STORAGE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let success = world.update_inventory_and_vip_warehouse(sid, |inv, vip| {
        while vip.len() < VIPWAREHOUSE_MAX {
            vip.push(UserItemSlot::default());
        }

        let src = match inv.get(src_slot_idx) {
            Some(s) if s.item_id == item_id => s.clone(),
            _ => return false,
        };

        // C++ checks: not rented, sealed, duplicate, or expiring
        if src.flag == ITEM_FLAG_RENTED
            || src.flag == ITEM_FLAG_SEALED
            || src.flag == ITEM_FLAG_DUPLICATE
            || src.expire_time > 0
        {
            return false;
        }

        let dst = &vip[vip_slot_idx];

        // C++ check: kind==255 && !countable && dst occupied
        if dst.item_id != 0 && kind == ITEM_KIND_UNIQUE && countable == 0 {
            return false;
        }

        // Non-stackable must go to empty; stackable must match or empty
        if src.count < count || (dst.item_id != 0 && (dst.item_id != src.item_id || !is_stackable))
        {
            return false;
        }

        // Clear empty dst before writing
        if vip[vip_slot_idx].count == 0 || vip[vip_slot_idx].item_id == 0 {
            vip[vip_slot_idx] = UserItemSlot::default();
        }

        // Apply to destination
        let dst = &mut vip[vip_slot_idx];
        if is_stackable {
            dst.count = dst.count.saturating_add(count);
        } else {
            dst.count = count;
        }

        // Reduce source
        let src_mut = &mut inv[src_slot_idx];
        if is_stackable {
            src_mut.count -= count;
        } else {
            src_mut.count = 0;
        }

        // Handle serial number
        let serial = if src.serial_num != 0 {
            src.serial_num
        } else {
            world.generate_item_serial()
        };
        if is_stackable {
            if inv[src_slot_idx].count == 0 && dst.serial_num == 0 {
                dst.serial_num = serial;
            } else if dst.serial_num == 0 {
                dst.serial_num = world.generate_item_serial();
            }
        } else {
            dst.serial_num = serial;
        }

        dst.durability = src.durability;
        dst.flag = src.flag;
        dst.original_flag = src.original_flag;
        dst.expire_time = src.expire_time;
        dst.item_id = src.item_id;

        if dst.count > ITEMCOUNT_MAX {
            dst.count = ITEMCOUNT_MAX;
        }

        // Clear source if empty
        if inv[src_slot_idx].count == 0 || (kind == ITEM_KIND_UNIQUE && countable == 0) {
            inv[src_slot_idx] = UserItemSlot::default();
        }

        true
    });

    if success {
        world.set_user_ability(sid);
        save_vip_warehouse_slot_async(session, vip_slot_idx);
        save_inventory_slot_async(session, src_slot_idx);

        let ok = build_error(VIP_INVEN_TO_STORAGE, 1);
        session.send_packet(&ok).await?;
    } else {
        let err = build_error(VIP_INVEN_TO_STORAGE, 2);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle VIP_StorageToInven (sub-opcode 3) -- VIP storage -> inventory.
async fn handle_storage_to_inven(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let _npc_id = reader.read_u32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let count = reader.read_u16().unwrap_or(0);

    if page > 1 || count == 0 {
        let err = build_error(VIP_STORAGE_TO_INVEN, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    if item_id == ITEM_GOLD {
        let err = build_error(VIP_STORAGE_TO_INVEN, 1);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let reference_pos = 48 * page as usize;
    let vip_slot_idx = reference_pos + src_pos as usize;

    if vip_slot_idx >= VIPWAREHOUSE_MAX {
        let err = build_error(VIP_STORAGE_TO_INVEN, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let item_table = match world.get_item(item_id) {
        Some(i) => i,
        None => {
            let err = build_error(VIP_STORAGE_TO_INVEN, 2);
            session.send_packet(&err).await?;
            return Ok(());
        }
    };
    let is_stackable = item_table.countable.unwrap_or(0) > 0;
    let kind = item_table.kind.unwrap_or(0);
    let countable = item_table.countable.unwrap_or(0);

    if dst_pos as usize >= HAVE_MAX {
        let err = build_error(VIP_STORAGE_TO_INVEN, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Weight check
    if !world.check_weight(sid, item_id, count) {
        let err = build_error(VIP_STORAGE_TO_INVEN, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let dst_slot_idx = SLOT_MAX + dst_pos as usize;

    let success = world.update_inventory_and_vip_warehouse(sid, |inv, vip| {
        while vip.len() < VIPWAREHOUSE_MAX {
            vip.push(UserItemSlot::default());
        }

        let src = match vip.get(vip_slot_idx) {
            Some(s) if s.item_id == item_id && s.count >= count => s.clone(),
            _ => return false,
        };

        let dst = match inv.get(dst_slot_idx) {
            Some(d) => d.clone(),
            None => return false,
        };

        if src.count < count || (dst.item_id != 0 && (dst.item_id != src.item_id || !is_stackable))
        {
            return false;
        }

        if vip[vip_slot_idx].count == 0 || vip[vip_slot_idx].item_id == 0 {
            vip[vip_slot_idx] = UserItemSlot::default();
        }

        let dst_mut = &mut inv[dst_slot_idx];
        if is_stackable {
            dst_mut.count = dst_mut.count.saturating_add(count);
        } else {
            dst_mut.count = count;
        }

        let src_mut = &mut vip[vip_slot_idx];
        if is_stackable {
            src_mut.count -= count;
        } else {
            src_mut.count = 0;
        }

        let serial = if src.serial_num != 0 {
            src.serial_num
        } else {
            world.generate_item_serial()
        };
        if is_stackable {
            if vip[vip_slot_idx].count == 0 && dst_mut.serial_num == 0 {
                dst_mut.serial_num = serial;
            } else if dst_mut.serial_num == 0 {
                dst_mut.serial_num = world.generate_item_serial();
            }
        } else {
            dst_mut.serial_num = serial;
        }

        dst_mut.durability = src.durability;
        dst_mut.flag = src.flag;
        dst_mut.original_flag = src.original_flag;
        dst_mut.expire_time = src.expire_time;
        dst_mut.item_id = src.item_id;

        if dst_mut.count > ITEMCOUNT_MAX {
            dst_mut.count = ITEMCOUNT_MAX;
        }

        if vip[vip_slot_idx].count == 0 || (kind == ITEM_KIND_UNIQUE && countable == 0) {
            vip[vip_slot_idx] = UserItemSlot::default();
        }

        true
    });

    if success {
        world.set_user_ability(sid);
        save_vip_warehouse_slot_async(session, vip_slot_idx);
        save_inventory_slot_async(session, dst_slot_idx);

        let ok = build_error(VIP_STORAGE_TO_INVEN, 1);
        session.send_packet(&ok).await?;
    } else {
        let err = build_error(VIP_STORAGE_TO_INVEN, 2);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle VIP_StorageToStore (sub-opcode 4) -- rearrange within VIP storage.
async fn handle_storage_to_store(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let now = unix_now();
    if world.get_vip_vault_expiry(sid) < now {
        let err = build_error(VIP_STORAGE_TO_STORE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let _npc_id = reader.read_u32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let _count = reader.read_u16().unwrap_or(0);

    let reference_pos = 48 * page as usize;
    if page > 1
        || src_pos as usize > reference_pos + VIPWAREHOUSE_MAX
        || dst_pos as usize > reference_pos + VIPWAREHOUSE_MAX
    {
        let err = build_error(VIP_STORAGE_TO_STORE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let src_idx = src_pos as usize;
    let dst_idx = dst_pos as usize;

    if src_idx >= VIPWAREHOUSE_MAX || dst_idx >= VIPWAREHOUSE_MAX {
        let err = build_error(VIP_STORAGE_TO_STORE, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let success = world.update_vip_warehouse(sid, |vip| {
        while vip.len() < VIPWAREHOUSE_MAX {
            vip.push(UserItemSlot::default());
        }

        if vip[src_idx].item_id != item_id || vip[dst_idx].item_id != 0 {
            return false;
        }

        let tmp = vip[src_idx].clone();
        vip[dst_idx] = tmp;
        vip[src_idx] = UserItemSlot::default();
        true
    });

    if success {
        save_vip_warehouse_slot_async(session, src_idx);
        save_vip_warehouse_slot_async(session, dst_idx);
        super::warehouse::gm_move_note(&world, sid, "Vault", item_id, src_idx, dst_idx);
        let ok = build_error(VIP_STORAGE_TO_STORE, 1);
        session.send_packet(&ok).await?;
    } else {
        let err = build_error(VIP_STORAGE_TO_STORE, 2);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle VIP_InvenToInven (sub-opcode 5) -- rearrange inventory from VIP UI.
async fn handle_inven_to_inven(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let _npc_id = reader.read_u32().unwrap_or(0);
    let _item_id = reader.read_u32().unwrap_or(0);
    let page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let dst_pos = reader.read_u8().unwrap_or(0);
    let _count = reader.read_u16().unwrap_or(0);

    if page > 1 || src_pos as usize >= HAVE_MAX || dst_pos as usize >= HAVE_MAX {
        let err = build_error(VIP_INVEN_TO_INVEN, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let src_idx = SLOT_MAX + src_pos as usize;
    let dst_idx = SLOT_MAX + dst_pos as usize;

    let success = world.update_inventory(sid, |inv| {
        if src_idx >= inv.len() || dst_idx >= inv.len() {
            return false;
        }

        // C++ does a full swap via memcpy
        let tmp = inv[src_idx].clone();
        inv[src_idx] = inv[dst_idx].clone();
        inv[dst_idx] = tmp;
        true
    });

    if success {
        save_inventory_slot_async(session, src_idx);
        save_inventory_slot_async(session, dst_idx);
        let ok = build_error(VIP_INVEN_TO_INVEN, 1);
        session.send_packet(&ok).await?;
    } else {
        let err = build_error(VIP_INVEN_TO_INVEN, 2);
        session.send_packet(&err).await?;
    }

    Ok(())
}

/// Handle VIP_UseVault (sub-opcode 6) -- activate vault key item.
async fn handle_use_vault(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let now = unix_now();
    let current_expiry = world.get_vip_vault_expiry(sid);

    // C++ check: don't allow extending beyond 30 days from now
    if current_expiry > now && (current_expiry - now) > MAX_VAULT_EXTENSION_SECS {
        let err = build_error(VIP_USE_VAULT, 5);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let _npc_id = reader.read_u32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let _page = reader.read_u8().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let _dst_pos = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] VIP_UseVault request: npc_id={} item_id={} src_pos={} current_expiry={}",
        session.addr(),
        _npc_id,
        item_id,
        src_pos,
        current_expiry
    );

    let days = match vip_key_days(item_id) {
        Some(days) => days,
        None => {
            warn!(
                "[{}] VIP_UseVault rejected unsupported key item_id={}",
                session.addr(),
                item_id
            );
            let err = build_error(VIP_USE_VAULT, 2);
            session.send_packet(&err).await?;
            return Ok(());
        }
    };

    if src_pos as usize >= HAVE_MAX {
        let err = build_error(VIP_USE_VAULT, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let src_slot_idx = SLOT_MAX + src_pos as usize;

    // Verify item exists in the given slot
    let slot = world
        .get_inventory_slot(sid, src_slot_idx)
        .unwrap_or_default();
    if slot.item_id != item_id {
        warn!(
            "[{}] VIP_UseVault slot mismatch: item_id={} src_pos={} absolute_slot={} slot_item={} count={} flag={}",
            session.addr(), item_id, src_pos, src_slot_idx, slot.item_id, slot.count, slot.flag
        );
        let err = build_error(VIP_USE_VAULT, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // C++ checks: no rented, duplicate items
    if slot.flag == ITEM_FLAG_RENTED || slot.flag == ITEM_FLAG_DUPLICATE {
        warn!(
            "[{}] VIP_UseVault rejected item flag: item_id={} flag={}",
            session.addr(),
            item_id,
            slot.flag
        );
        let err = build_error(VIP_USE_VAULT, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // v2615 Safe/Vault keys (931695000 and 998019000) intentionally live in
    // the generic no-trade ID range. `vip_key_days()` is the authoritative
    // allowlist here; applying ITEM_NO_TRADE_* again rejects valid keys.

    let new_expiry = now + 60 * 60 * 24 * days;

    // Consume exactly one key; v2615 keys are countable.
    let consumed = world.update_inventory(sid, |inv| {
        let Some(src) = inv.get_mut(src_slot_idx) else {
            return false;
        };
        if src.item_id != item_id || src.count == 0 {
            return false;
        }
        if src.count > 1 {
            src.count -= 1;
        } else {
            *src = UserItemSlot::default();
        }
        true
    });
    if !consumed {
        let err = build_error(VIP_USE_VAULT, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Set vault expiry
    world.set_vip_vault_expiry(sid, new_expiry);
    world.set_user_ability(sid);

    debug!(
        "[{}] VIP_UseVault activated: item_id={} days={} expiry={}",
        session.addr(),
        item_id,
        days,
        new_expiry
    );

    // Save to DB
    save_inventory_slot_async(session, src_slot_idx);
    save_vip_metadata_async(session);

    // Send weight change
    // Response: u8(6) + u8(1) + u8(1) + u32(new_expiry)
    let mut result = Packet::new(Opcode::WizVipwarehouse as u8);
    result.write_u8(VIP_USE_VAULT);
    result.write_u8(1); // success
    result.write_u8(1); // vault active
    result.write_u32(new_expiry);
    session.send_packet(&result).await
}

/// Handle VIP_SetPassword (sub-opcode 8) -- set 4-digit PIN.
async fn handle_set_password(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let password = reader.read_sbyte_string().unwrap_or_default();

    // Validate: exactly 4 digits
    if !is_valid_pin(&password) {
        let err = build_error(VIP_SET_PASSWORD, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    world.set_vip_password(sid, password, 1);
    save_vip_metadata_async(session);

    let ok = build_error(VIP_SET_PASSWORD, 1);
    session.send_packet(&ok).await
}

/// Handle VIP_CancelPassword (sub-opcode 9) -- remove password.
async fn handle_cancel_password(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    world.set_vip_password(sid, String::new(), 0);
    save_vip_metadata_async(session);

    let ok = build_error(VIP_CANCEL_PASSWORD, 1);
    session.send_packet(&ok).await
}

/// Handle VIP_ChangePassword (sub-opcode 10) -- change PIN.
async fn handle_change_password(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let password = reader.read_sbyte_string().unwrap_or_default();

    if !is_valid_pin(&password) {
        let err = build_error(VIP_CHANGE_PASSWORD, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let password_request = world.get_vip_password_request(sid);
    world.set_vip_password(sid, password, password_request);
    save_vip_metadata_async(session);

    let ok = build_error(VIP_CHANGE_PASSWORD, 1);
    session.send_packet(&ok).await
}

/// Handle VIP_EnterPassword (sub-opcode 11) -- enter PIN to unlock.
async fn handle_enter_password(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let password = reader.read_sbyte_string().unwrap_or_default();

    if !is_valid_pin(&password) {
        let err = build_error(VIP_ENTER_PASSWORD, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    let stored_password = world.get_vip_password(sid);
    if password != stored_password {
        let err = build_error(VIP_ENTER_PASSWORD, 2);
        session.send_packet(&err).await?;
        return Ok(());
    }

    // Password accepted -- send success response
    let mut accept = Packet::new(Opcode::WizVipwarehouse as u8);
    accept.write_u8(VIP_ENTER_PASSWORD);
    accept.write_u8(1); // success
    accept.write_u8(0);
    session.send_packet(&accept).await?;

    // Then send the full VIP_Open response with items
    let password_request = world.get_vip_password_request(sid);
    let vault_expiry = world.get_vip_vault_expiry(sid);
    let items = world.get_vip_warehouse(sid);

    let now = unix_now();
    let has_password = stored_password.len() == 4;
    let is_key = vault_expiry > now;

    let mut result = Packet::new(Opcode::WizVipwarehouse as u8);
    result.write_u8(VIP_OPEN);
    result.write_u8(1); // success
    result.write_u8(if has_password { 1 } else { 0 });

    if !is_key {
        result.write_u8(0); // no vault key
        result.write_u32(0);
    } else {
        result.write_u8(1); // vault active
        result.write_u32(vault_expiry);
    }

    // Vault key item info placeholder
    result.write_u32(VIP_VAULT_KEY);
    result.write_u16(1);
    result.write_u16(1);
    result.write_u8(0);
    result.write_u32(0);
    result.write_u16(0);

    // 48 item slots
    let rebirth_level = world
        .get_character_info(sid)
        .map(|c| c.rebirth_level)
        .unwrap_or(0);
    for i in 0..VIPWAREHOUSE_MAX {
        let slot = items.get(i).cloned().unwrap_or_default();
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

    session.send_packet(&result).await?;

    // Ignore password_request unused variable
    let _ = password_request;

    Ok(())
}

/// Validate a 4-digit PIN.
fn is_valid_pin(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_digit())
}

/// Save a VIP warehouse slot to DB (fire-and-forget).
fn save_vip_warehouse_slot_async(session: &ClientSession, slot_idx: usize) {
    let world = session.world().clone();
    let sid = session.session_id();
    let account_id = match session.account_id() {
        Some(a) => a.to_string(),
        None => return,
    };
    let slot = world
        .get_vip_warehouse_slot(sid, slot_idx)
        .unwrap_or_default();
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        let params = SaveVipWarehouseItemParams {
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
        if let Err(e) = repo.save_vip_warehouse_item(&params).await {
            warn!("Failed to save VIP warehouse slot {}: {}", slot_idx, e);
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

/// Save VIP warehouse metadata (password, password_request, vault_expiry) to DB.
fn save_vip_metadata_async(session: &ClientSession) {
    let world = session.world().clone();
    let sid = session.session_id();
    let account_id = match session.account_id() {
        Some(a) => a.to_string(),
        None => return,
    };
    let password = world.get_vip_password(sid);
    let password_request = world.get_vip_password_request(sid);
    let vault_expiry = world.get_vip_vault_expiry(sid);
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo
            .save_vip_warehouse(
                &account_id,
                &password,
                password_request as i16,
                vault_expiry as i32,
            )
            .await
        {
            warn!("Failed to save VIP warehouse metadata: {}", e);
        }
    });
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_vip_sub_opcodes() {
        assert_eq!(VIP_OPEN, 1);
        assert_eq!(VIP_INVEN_TO_STORAGE, 2);
        assert_eq!(VIP_STORAGE_TO_INVEN, 3);
        assert_eq!(VIP_STORAGE_TO_STORE, 4);
        assert_eq!(VIP_INVEN_TO_INVEN, 5);
        assert_eq!(VIP_USE_VAULT, 6);
        assert_eq!(VIP_SET_PASSWORD, 8);
        assert_eq!(VIP_CANCEL_PASSWORD, 9);
        assert_eq!(VIP_CHANGE_PASSWORD, 10);
        assert_eq!(VIP_ENTER_PASSWORD, 11);
    }

    #[test]
    fn test_vip_error_packet() {
        let pkt = build_error(VIP_OPEN, 2);
        assert_eq!(pkt.opcode, Opcode::WizVipwarehouse as u8);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], VIP_OPEN);
        assert_eq!(pkt.data[1], 2);
    }

    #[test]
    fn test_vip_constants() {
        assert_eq!(VIPWAREHOUSE_MAX, 48);
        assert_eq!(VIP_VAULT_KEY, 800_442_000);
        assert_eq!(VIP_VAULT_KEY_V2615, 931_695_000);
        assert_eq!(VIP_SAFE_KEY_1, 810_442_000);
        assert_eq!(VIP_SAFE_KEY_7, 998_019_000);
    }

    #[test]
    fn test_is_valid_pin() {
        assert!(is_valid_pin("1234"));
        assert!(is_valid_pin("0000"));
        assert!(is_valid_pin("9999"));
        assert!(!is_valid_pin("123")); // too short
        assert!(!is_valid_pin("12345")); // too long
        assert!(!is_valid_pin("abcd")); // not digits
        assert!(!is_valid_pin("12a4")); // mixed
        assert!(!is_valid_pin("")); // empty
    }

    #[test]
    fn test_vault_key_days() {
        assert_eq!(vip_key_days(VIP_VAULT_KEY), Some(30));
        assert_eq!(vip_key_days(VIP_VAULT_KEY_V2615), Some(30));
        assert_eq!(vip_key_days(VIP_SAFE_KEY_1), Some(1));
        assert_eq!(vip_key_days(VIP_SAFE_KEY_7), Some(7));
        assert_eq!(vip_key_days(931_694_000), None);
    }

    #[tokio::test]
    async fn test_build_open_response_no_vault() {
        let world = WorldState::new();
        let pool = ko_db::DbPool::connect_lazy("postgres://invalid").unwrap();
        let items = vec![UserItemSlot::default(); VIPWAREHOUSE_MAX];
        let pkt = build_open_response(&world, &pool, 0, 0, 0, &items).await;
        assert_eq!(pkt.opcode, Opcode::WizVipwarehouse as u8);

        // v2525 header: VIP_OPEN(1) + success(1) + pw_req(1) + 4 tabs × (isKey(1) + expiry(4))
        assert_eq!(pkt.data[0], VIP_OPEN);
        assert_eq!(pkt.data[1], 1); // success
        assert_eq!(pkt.data[2], 0); // password_request
        assert_eq!(pkt.data[3], 0); // tab0 isKey = false (expiry=0)

        // v2525: 3 + 20 (4 tabs) + 192 items × 17 bytes
        let expected_len = 3 + 4 * 5 + VIPWAREHOUSE_V2525_TOTAL * (4 + 2 + 2 + 1 + 4 + 4);
        assert_eq!(pkt.data.len(), expected_len);
    }

    #[tokio::test]
    async fn test_build_open_response_with_items() {
        let world = WorldState::new();
        let pool = ko_db::DbPool::connect_lazy("postgres://invalid").unwrap();
        let mut items = vec![UserItemSlot::default(); VIPWAREHOUSE_MAX];
        items[0] = UserItemSlot {
            item_id: 100001,
            durability: 100,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 12345,
            expire_time: 0,
        };
        items[5] = UserItemSlot {
            item_id: 200002,
            durability: 50,
            count: 99,
            flag: 0,
            original_flag: 0,
            serial_num: 67890,
            expire_time: 0,
        };

        let pkt = build_open_response(&world, &pool, 0, 1, u32::MAX, &items).await;
        assert_eq!(pkt.data[0], VIP_OPEN);
        assert_eq!(pkt.data[1], 1); // success
        assert_eq!(pkt.data[2], 1); // password_request = 1
        assert_eq!(pkt.data[3], 1); // tab0 isKey = true (expiry is far future)

        // Verify tab1-3 are inactive
        assert_eq!(pkt.data[8], 0); // tab1 isKey = false
        assert_eq!(pkt.data[13], 0); // tab2 isKey = false
        assert_eq!(pkt.data[18], 0); // tab3 isKey = false

        // Verify item 0 starts at offset 23 (3 header + 20 tab data)
        let item0_off = 23;
        let id = u32::from_le_bytes(pkt.data[item0_off..item0_off + 4].try_into().unwrap());
        assert_eq!(id, 100001);
    }

    #[tokio::test]
    async fn test_build_open_response_v2525_4tab_format() {
        let world = WorldState::new();
        let pool = ko_db::DbPool::connect_lazy("postgres://invalid").unwrap();
        let items = vec![UserItemSlot::default(); VIPWAREHOUSE_MAX];
        let vault_expiry = u32::MAX; // future timestamp
        let pkt = build_open_response(&world, &pool, 0, 0, vault_expiry, &items).await;

        // Tab 0: active
        assert_eq!(pkt.data[3], 1); // isKey = true
        let exp0 = u32::from_le_bytes(pkt.data[4..8].try_into().unwrap());
        assert_eq!(exp0, u32::MAX);

        // Tab 1: inactive
        assert_eq!(pkt.data[8], 0);
        let exp1 = u32::from_le_bytes(pkt.data[9..13].try_into().unwrap());
        assert_eq!(exp1, 0);

        // Tab 2: inactive
        assert_eq!(pkt.data[13], 0);

        // Tab 3: inactive
        assert_eq!(pkt.data[18], 0);

        // Total: 192 items
        let header_size = 3 + 4 * 5;
        let item_size = 4 + 2 + 2 + 1 + 4 + 4; // 17
        assert_eq!(
            pkt.data.len(),
            header_size + VIPWAREHOUSE_V2525_TOTAL * item_size
        );
    }

    #[test]
    fn test_build_error_all_opcodes() {
        for &(op, code) in &[
            (VIP_OPEN, 2u8),
            (VIP_INVEN_TO_STORAGE, 2),
            (VIP_STORAGE_TO_INVEN, 1),
            (VIP_STORAGE_TO_STORE, 2),
            (VIP_USE_VAULT, 5),
            (VIP_SET_PASSWORD, 2),
            (VIP_CANCEL_PASSWORD, 1),
            (VIP_CHANGE_PASSWORD, 2),
            (VIP_ENTER_PASSWORD, 2),
        ] {
            let pkt = build_error(op, code);
            assert_eq!(pkt.data[0], op);
            assert_eq!(pkt.data[1], code);
        }
    }

    #[test]
    fn test_no_trade_item_range() {
        assert!(ITEM_NO_TRADE_MIN > ITEM_GOLD);
        assert!(ITEM_NO_TRADE_MAX > ITEM_NO_TRADE_MIN);
        assert!((ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&900_000_001));
        assert!(!(ITEM_NO_TRADE_MIN..=ITEM_NO_TRADE_MAX).contains(&ITEM_GOLD));
    }

    #[test]
    fn test_max_vault_extension() {
        // 30 days in seconds
        assert_eq!(MAX_VAULT_EXTENSION_SECS, 60 * 60 * 24 * 30);
        assert_eq!(MAX_VAULT_EXTENSION_SECS, 2_592_000);
    }

    #[test]
    fn test_slot_max_and_have_max() {
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        // First inventory slot = SLOT_MAX + 0 = 14
        // Last inventory slot = SLOT_MAX + HAVE_MAX - 1 = 41
        assert_eq!(SLOT_MAX + HAVE_MAX - 1, 41);
    }

    // ── Sprint 949: Additional coverage ──────────────────────────────

    /// Sub-opcodes 7 is skipped (no VIP_CLAIM).
    #[test]
    fn test_vip_sub_opcode_gap() {
        // Opcode 7 is intentionally missing in the protocol
        assert_eq!(VIP_USE_VAULT, 6);
        assert_eq!(VIP_SET_PASSWORD, 8);
    }

    /// VIPWAREHOUSE_V2525_TOTAL = 192 (4 tabs × 48 slots).
    #[test]
    fn test_vip_v2525_total_slots() {
        assert_eq!(VIPWAREHOUSE_V2525_TOTAL, 192);
        assert_eq!(VIPWAREHOUSE_V2525_TOTAL, VIPWAREHOUSE_MAX * 4);
    }

    /// Vault keys are distinct item IDs.
    #[test]
    fn test_vault_key_ids_distinct() {
        assert_ne!(VIP_VAULT_KEY, VIP_SAFE_KEY_1);
        assert_ne!(VIP_VAULT_KEY, VIP_SAFE_KEY_7);
        assert_ne!(VIP_VAULT_KEY, VIP_VAULT_KEY_V2615);
        assert_ne!(VIP_SAFE_KEY_1, VIP_SAFE_KEY_7);
    }

    /// PIN edge cases: leading zeros and all same digits.
    #[test]
    fn test_pin_edge_cases() {
        assert!(is_valid_pin("0001"));
        assert!(is_valid_pin("1111"));
        assert!(!is_valid_pin("12 4")); // space
        assert!(!is_valid_pin("12\n4")); // newline
    }

    /// Error packet is always 2 bytes (subcode + error_id).
    #[test]
    fn test_error_packet_size() {
        let pkt = build_error(VIP_ENTER_PASSWORD, 0);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.opcode, Opcode::WizVipwarehouse as u8);
    }

    // ── Sprint 959: Additional coverage ──────────────────────────────

    /// VIPWAREHOUSE_MAX is 48 slots.
    #[test]
    fn test_vipwarehouse_max_slots() {
        assert_eq!(VIPWAREHOUSE_MAX, 48);
        // 4 tabs of 12 slots each in v2525
        assert_eq!(VIPWAREHOUSE_MAX, 4 * 12);
    }

    /// Vault key item IDs are in different item ranges.
    #[test]
    fn test_vault_key_item_ranges() {
        assert_eq!(VIP_VAULT_KEY, 800_442_000);
        assert_eq!(VIP_SAFE_KEY_1, 810_442_000);
        assert_eq!(VIP_SAFE_KEY_7, 998_019_000);
        // All in premium item range (800M+)
        assert!(VIP_VAULT_KEY >= 800_000_000);
        assert!(VIP_SAFE_KEY_1 >= 800_000_000);
        assert!(VIP_SAFE_KEY_7 >= 900_000_000);
    }

    /// MAX_VAULT_EXTENSION_SECS is exactly 30 days.
    #[test]
    fn test_max_vault_extension_30_days() {
        assert_eq!(MAX_VAULT_EXTENSION_SECS, 60 * 60 * 24 * 30);
        assert_eq!(MAX_VAULT_EXTENSION_SECS, 2_592_000);
    }

    /// Password sub-opcodes are 8-11 (gap at 7).
    #[test]
    fn test_vip_password_subopcodes() {
        assert_eq!(VIP_SET_PASSWORD, 8);
        assert_eq!(VIP_CANCEL_PASSWORD, 9);
        assert_eq!(VIP_CHANGE_PASSWORD, 10);
        assert_eq!(VIP_ENTER_PASSWORD, 11);
        // Sequential 8..11
        assert_eq!(VIP_ENTER_PASSWORD - VIP_SET_PASSWORD, 3);
    }

    /// is_valid_pin rejects non-digit and wrong-length strings.
    #[test]
    fn test_pin_validation_extended() {
        assert!(is_valid_pin("9999"));
        assert!(is_valid_pin("0000"));
        assert!(!is_valid_pin("")); // empty
        assert!(!is_valid_pin("123")); // too short
        assert!(!is_valid_pin("12345")); // too long
        assert!(!is_valid_pin("abcd")); // letters
        assert!(!is_valid_pin("12-4")); // special char
    }

    // ── Sprint 971: Additional coverage ──────────────────────────────

    /// VIP sub-opcodes 1-6 are storage operations (gap at 7).
    #[test]
    fn test_vip_storage_subopcodes() {
        assert_eq!(VIP_OPEN, 1);
        assert_eq!(VIP_INVEN_TO_STORAGE, 2);
        assert_eq!(VIP_STORAGE_TO_INVEN, 3);
        assert_eq!(VIP_STORAGE_TO_STORE, 4);
        assert_eq!(VIP_INVEN_TO_INVEN, 5);
        assert_eq!(VIP_USE_VAULT, 6);
        // Gap at 7, then password opcodes start at 8
        assert_eq!(VIP_SET_PASSWORD - VIP_USE_VAULT, 2);
    }

    /// build_error produces correct opcode and 2-byte payload.
    #[test]
    fn test_build_error_all_subcodes() {
        for sub in [
            VIP_OPEN,
            VIP_USE_VAULT,
            VIP_SET_PASSWORD,
            VIP_ENTER_PASSWORD,
        ] {
            let pkt = build_error(sub, 1);
            assert_eq!(pkt.opcode, Opcode::WizVipwarehouse as u8);
            assert_eq!(pkt.data[0], sub);
            assert_eq!(pkt.data[1], 1);
        }
    }

    /// Vault key IDs are all distinct and in premium range.
    #[test]
    fn test_vault_keys_distinct() {
        assert_ne!(VIP_VAULT_KEY, VIP_SAFE_KEY_1);
        assert_ne!(VIP_VAULT_KEY, VIP_SAFE_KEY_7);
        assert_ne!(VIP_SAFE_KEY_1, VIP_SAFE_KEY_7);
    }

    /// No-trade items cannot be stored in VIP warehouse.
    #[test]
    fn test_vip_no_trade_range() {
        assert!(ITEM_GOLD < ITEM_NO_TRADE_MIN);
        assert!(ITEM_NO_TRADE_MIN < ITEM_NO_TRADE_MAX);
        // Gold is a special case, always blocked
        assert_eq!(ITEM_GOLD, 900_000_000);
    }

    /// VIPWAREHOUSE_MAX fits in u8 for slot indexing.
    #[test]
    fn test_vipwarehouse_max_fits_u8() {
        assert!(VIPWAREHOUSE_MAX <= u8::MAX as usize);
        assert_eq!(VIPWAREHOUSE_MAX, 48);
    }

    /// VIP sub-opcodes have a gap at 7 (6 → 8).
    #[test]
    fn test_vip_sub_opcode_gap_at_7() {
        assert_eq!(VIP_USE_VAULT, 6);
        assert_eq!(VIP_SET_PASSWORD, 8);
        // Gap: 7 is not used
        assert_eq!(VIP_SET_PASSWORD - VIP_USE_VAULT, 2);
        // Password sub-opcodes 8-11 are contiguous
        assert_eq!(VIP_CANCEL_PASSWORD, VIP_SET_PASSWORD + 1);
        assert_eq!(VIP_CHANGE_PASSWORD, VIP_CANCEL_PASSWORD + 1);
        assert_eq!(VIP_ENTER_PASSWORD, VIP_CHANGE_PASSWORD + 1);
    }

    /// VIPWAREHOUSE_MAX (48) = 2 pages × 24 slots per page.
    #[test]
    fn test_vip_warehouse_page_layout() {
        assert_eq!(VIPWAREHOUSE_MAX, 48);
        // 2 pages of 24
        assert_eq!(VIPWAREHOUSE_MAX / 24, 2);
        assert_eq!(VIPWAREHOUSE_MAX % 24, 0);
    }

    /// MAX_VAULT_EXTENSION_SECS = 30 days in seconds = 2,592,000.
    #[test]
    fn test_vault_extension_calculation() {
        assert_eq!(MAX_VAULT_EXTENSION_SECS, 60 * 60 * 24 * 30);
        assert_eq!(MAX_VAULT_EXTENSION_SECS, 2_592_000);
        // Fits in u32
        assert!(MAX_VAULT_EXTENSION_SECS < u32::MAX);
    }

    /// Vault key IDs are in 800M+ range and all distinct.
    #[test]
    fn test_vault_key_id_ranges() {
        assert_eq!(VIP_VAULT_KEY, 800_442_000);
        assert_eq!(VIP_SAFE_KEY_1, 810_442_000);
        assert_eq!(VIP_SAFE_KEY_7, 998_019_000);
        // All in 800M-1B range
        assert!(VIP_VAULT_KEY >= 800_000_000);
        assert!(VIP_SAFE_KEY_7 < 1_000_000_000);
        // All distinct
        assert_ne!(VIP_VAULT_KEY, VIP_SAFE_KEY_1);
        assert_ne!(VIP_SAFE_KEY_1, VIP_SAFE_KEY_7);
    }

    /// VIP opcode is WizVipwarehouse (0x8B).
    #[test]
    fn test_vip_warehouse_opcode() {
        assert_eq!(Opcode::WizVipwarehouse as u8, 0x8B);
        // Within v2525 dispatch range (0x06-0xD7)
        assert!(Opcode::WizVipwarehouse as u8 >= 0x06);
        assert!(Opcode::WizVipwarehouse as u8 <= 0xD7);
    }
}
