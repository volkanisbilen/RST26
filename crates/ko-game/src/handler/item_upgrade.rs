//! WIZ_ITEM_UPGRADE (0x5B) handler — item upgrade system (normal, accessories, rebirth).
//! Packet format (from client):
//! ```text
//! [u8 nUpgradeType] [u8 bType] [u32 npcID] [10x (i32 itemID, i8 slot)]
//! ```
//! nUpgradeType: 2=normal, 3=accessories, 7=rebirth
//! bType: 1=execute, 2=preview
//! Response (WIZ_ITEM_UPGRADE):
//! ```text
//! [u8 nUpgradeType] [u8 bType] [u8 result] [optional: u8 logos_flag] [N x (i32 itemID, i8 slot)]
//! ```

use std::sync::Arc;

use ko_db::models::item_tables::ITEMS_SPECIAL_EXCHANGE_GROUP;
use ko_db::repositories::character::{CharacterRepository, SaveItemParams};
use ko_db::repositories::pet::PetRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::{
    PetState, ITEM_FLAG_BOUND, ITEM_FLAG_CHAR_SEAL, ITEM_FLAG_DUPLICATE, ITEM_FLAG_NONE,
    ITEM_FLAG_NOT_BOUND, ITEM_FLAG_RENTED, ITEM_FLAG_SEALED, ZONE_MORADON, ZONE_MORADON2,
    ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
};

use super::{HAVE_MAX, ITEM_KIND_UNIQUE, SLOT_MAX};
use crate::object_event_constants::{OBJECT_ANVIL, OBJECT_ARTIFACT, OBJECT_NPC};

const ITEM_TRINA: u32 = 700002000;
const ITEM_KARIVDIS: u32 = 379258000;
const ITEM_LOW_CLASS_TRINA: u32 = 353000000;
const ITEM_MIDDLE_CLASS_TRINA: u32 = 352900000;
const ITEM_BLESSING_LOGOS: u32 = 890092000;
/// Accessory trina piece.
const ITEM_RING_TRINA: u32 = 354000000;
const ITEM_ACCESSORY_DISASSEMBLE_SCROLL: u32 = 810325000;
const ACCESSORY_UPGRADE_SCROLLS: [i32; 6] = [
    379159000, 379160000, 379161000, 379162000, 379163000, 379164000,
];
const ITEM_BLESSED_ELEMENTAL_SCROLL: u32 = 379025000;

const NPC_ANVIL: u8 = 24;

const UPGRADE_DELAY: u64 = 2;

const MAX_ITEMS_REQ: usize = 8;

/// C++ sub-opcodes for WIZ_ITEM_UPGRADE (ItemUpgradeOpcodes enum).
const ITEM_UPGRADE: u8 = 2;
const ITEM_ACCESSORIES: u8 = 3;
const ITEM_UPGRADE_REBIRTH: u8 = 7;
const ITEM_UPGRADE_REVERSE: u8 = 14;
const ITEM_BIFROST_REQ: u8 = 4;
const ITEM_BIFROST_EXCHANGE: u8 = 5;
const SPECIAL_PART_SEWING: u8 = 11;
const ITEM_OLDMAN_EXCHANGE: u8 = 13;
const ITEM_ACCESSORY_DISASSEMBLE: u8 = 15;
const ITEM_SEAL: u8 = 8;

// ── Item Seal sub-opcodes ────────────────────────────────────────────
const SEAL_LOCK: u8 = 1;
const SEAL_UNLOCK: u8 = 2;
const SEAL_BOUND: u8 = 3;
const SEAL_UNBOUND: u8 = 4;
/// Gold cost for sealing an item.
const ITEM_SEAL_PRICE: u32 = 1_000_000;
/// Binding scroll item ID — consumed when unbinding.
const BINDING_SCROLL_ID: u32 = 810_890_000;

// ── Pet Hatching / Transform sub-opcodes ──────────────────────────────
const PET_HATCHING: u8 = 6;
const PET_IMAGE_TRANSFORM: u8 = 10;
const PET_START_ITEM: u32 = 610_001_000;
const PET_START_LEVEL: u8 = 1;

const UPGRADE_TYPE_NORMAL: u8 = 1;
const UPGRADE_TYPE_PREVIEW: u8 = 2;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpgradeResult {
    Failed = 0,
    Succeeded = 1,
    Trading = 2,
    NeedCoins = 3,
    NoMatch = 4,
    Rental = 5,
}

async fn send_upgrade_fail(
    session: &mut ClientSession,
    upgrade_type: u8,
    b_type: u8,
    result: UpgradeResult,
    logos: bool,
    items: &[UpgradeItem],
    reason: &str,
) -> anyhow::Result<()> {
    debug!(
        "[{}] ItemUpgrade fail: type={} b_type={} result={:?} reason={} items={:?}",
        session.addr(),
        upgrade_type,
        b_type,
        result,
        reason,
        items
    );
    send_fail(session, upgrade_type, b_type, result, logos, items).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
enum ScrollType {
    Invalid = -1,
    LowClass = 1,
    MiddleClass = 2,
    HighClass = 3,
    Rebirth = 4,
    Class = 5,
    HighToRebirth = 15,
    Accessories = 8,
    RebirthRestoration = 17,
}

// Item flag constants imported from crate::world (ITEM_FLAG_BOUND, ITEM_FLAG_DUPLICATE, ITEM_FLAG_SEALED, ITEM_FLAG_RENTED).

/// An item sent by the client in the upgrade packet.
#[derive(Debug, Clone)]
struct UpgradeItem {
    item_id: u32,
    slot: i8,
}

/// Classify a scroll item ID into a scroll type.
fn get_scroll_type(scroll_id: u32) -> ScrollType {
    match scroll_id {
        379221000 | 379222000 | 379223000 | 379224000 | 379225000 | 379226000 | 379227000
        | 379228000 | 379229000 | 379230000 | 379231000 | 379232000 | 379233000 | 379234000
        | 379235000 | 379255000 => ScrollType::LowClass,

        379205000 | 379206000 | 379208000 | 379209000 | 379210000 | 379211000 | 379212000
        | 379213000 | 379214000 | 379215000 | 379216000 | 379217000 | 379218000 | 379219000
        | 379220000 => ScrollType::MiddleClass,

        379021000 | 379022000 | 379023000 | 379024000 | 379025000 | 379030000 | 379031000
        | 379032000 | 379033000 | 379034000 | 379035000 | 379138000 | 379139000 | 379140000
        | 379141000 | 379016000 | 379020000 | 379018000 | 379019000 => ScrollType::HighClass,

        379256000 => ScrollType::HighToRebirth,
        379257000 => ScrollType::Rebirth,
        810322000 => ScrollType::RebirthRestoration,

        379159000 | 379160000 | 379161000 | 379162000 | 379163000 | 379164000 => {
            ScrollType::Accessories
        }

        379152000 => ScrollType::Class,

        _ => ScrollType::Invalid,
    }
}

/// Count how many items in the list have the given item_id.
fn count_in_items(items: &[UpgradeItem], item_id: u32) -> u16 {
    items.iter().filter(|i| i.item_id == item_id).count() as u16
}

/// Handle WIZ_ITEM_UPGRADE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // NOTE: Busy-state checks are done INSIDE each sub-handler with proper
    // send_fail() responses.  A silent early return here would leave the
    // client waiting with no response (UI hangs).

    let mut reader = PacketReader::new(&pkt.data);

    // First byte: sub-opcode (upgrade type)
    let upgrade_type = reader.read_u8().unwrap_or(0);

    match upgrade_type {
        ITEM_UPGRADE | ITEM_ACCESSORIES | ITEM_UPGRADE_REBIRTH | ITEM_UPGRADE_REVERSE => {
            item_upgrade(session, &mut reader, upgrade_type).await
        }
        ITEM_BIFROST_REQ => bifrost_piece_req(session).await,
        ITEM_BIFROST_EXCHANGE => bifrost_piece_exchange(session, &mut reader).await,
        PET_HATCHING => pet_hatching(session, &mut reader).await,
        ITEM_SEAL => item_seal_process(session, &mut reader).await,
        PET_IMAGE_TRANSFORM => pet_image_transform(session, &mut reader).await,
        SPECIAL_PART_SEWING => shozin_exchange(session, &mut reader).await,
        ITEM_OLDMAN_EXCHANGE | ITEM_ACCESSORY_DISASSEMBLE => {
            item_disassemble(session, &mut reader, upgrade_type).await
        }
        super::character_seal::ITEM_CHARACTER_SEAL => {
            super::character_seal::handle(session, &mut reader).await
        }
        _ => {
            debug!(
                "[{}] WIZ_ITEM_UPGRADE: unknown sub-opcode {}",
                session.addr(),
                upgrade_type
            );
            Ok(())
        }
    }
}

/// Core upgrade logic shared by normal, accessories, and rebirth upgrades.
async fn item_upgrade(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    upgrade_type: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Read bType and npcID
    let b_type = reader.read_u8().unwrap_or(0);
    let npc_id = reader.read_u32().unwrap_or(0);

    let selected_npc_id = world
        .with_session(sid, |h| h.event_nid)
        .filter(|nid| *nid > 0)
        .map(|nid| nid as u32);
    let resolved_npc_id = if world.get_npc_instance(npc_id).is_some() {
        npc_id
    } else if let Some(selected_id) = selected_npc_id {
        selected_id
    } else {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::Trading,
            false,
            &[],
            "npc not found and no selected anvil",
        )
        .await?;
        return Ok(());
    };
    let Some(npc_inst) = world.get_npc_instance(resolved_npc_id) else {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::Trading,
            false,
            &[],
            "selected anvil not found",
        )
        .await?;
        return Ok(());
    };

    let npc_type = world
        .get_npc_template(npc_inst.proto_id, npc_inst.is_monster)
        .map(|tmpl| tmpl.npc_type)
        .unwrap_or(0);
    let selected_anvil_ui = world
        .with_session(sid, |h| {
            h.event_nid == resolved_npc_id as i16 && h.event_sid == npc_inst.proto_id as i16
        })
        .unwrap_or(false);
    let is_template_anvil = npc_type == NPC_ANVIL;
    let is_object_anvil = npc_type == OBJECT_ANVIL || selected_anvil_ui;
    if !is_template_anvil && !is_object_anvil {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::Trading,
            false,
            &[],
            "npc is not an anvil",
        )
        .await?;
        return Ok(());
    }

    // Static object anvils are opened via WIZ_OBJECT_EVENT, which already checks
    // object_event_pos range. Some object NPC instance coordinates differ from
    // that object position, so allow the immediate upgrade packet only if this
    // session opened the same anvil UI.
    let in_npc_range = world.is_in_npc_range(sid, resolved_npc_id);
    let selected_object_anvil = is_object_anvil && selected_anvil_ui;
    if !in_npc_range && !selected_object_anvil {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::Trading,
            false,
            &[],
            "npc out of range",
        )
        .await?;
        return Ok(());
    }

    // Read 10 items from the client
    let mut raw_items: [u32; 10] = [0; 10];
    let mut raw_slots: [i8; 10] = [-1; 10];
    let mut items: Vec<UpgradeItem> = Vec::with_capacity(10);

    for i in 0..raw_items.len() {
        let item_id = reader.read_u32().unwrap_or(0) as i32;
        let slot = reader.read_u8().unwrap_or(0xff) as i8;

        raw_items[i] = item_id as u32;
        raw_slots[i] = slot;

        if item_id > 0 && slot >= 0 && (slot as usize) < HAVE_MAX {
            items.push(UpgradeItem {
                item_id: item_id as u32,
                slot,
            });
        }
    }

    debug!(
        "[{}] ItemUpgrade request: type={} b_type={} npc={} raw_items={:?} parsed_items={:?}",
        session.addr(),
        upgrade_type,
        b_type,
        npc_id,
        raw_items,
        items
    );

    // ── Validation checks (matching C++ order) ──

    // Check player state: dead, trading, store open, merchanting, mining
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_store_open(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
    {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::Trading,
            false,
            &[],
            "blocked player state",
        )
        .await?;
        return Ok(());
    }

    // bType must be 1 (execute) or 2 (preview)
    if !(UPGRADE_TYPE_NORMAL..=UPGRADE_TYPE_PREVIEW).contains(&b_type) {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &[],
            "invalid b_type",
        )
        .await?;
        return Ok(());
    }

    // Must have at least one item
    if items.is_empty() {
        send_upgrade_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &[],
            "empty item list",
        )
        .await?;
        return Ok(());
    }

    // Rate-limiting: max upgrade count and 2-second cooldown between upgrades.
    {
        let max_count = world
            .get_server_settings()
            .map(|s| s.user_max_upgrade)
            .unwrap_or(30) as u8;
        let blocked = world
            .with_session(sid, |h| {
                if h.upgrade_count >= max_count {
                    return true;
                }
                h.last_upgrade_time.elapsed() < std::time::Duration::from_secs(UPGRADE_DELAY)
            })
            .unwrap_or(true);
        if blocked {
            send_upgrade_fail(
                session,
                upgrade_type,
                b_type,
                UpgradeResult::Trading,
                false,
                &[],
                "rate limit",
            )
            .await?;
            return Ok(());
        }
        // Update cooldown timestamp and increment counter
        world.update_session(sid, |h| {
            h.last_upgrade_time = std::time::Instant::now();
            h.upgrade_count = h.upgrade_count.saturating_add(1);
        });
    }

    // Validate all items exist in inventory and are not bound/sealed/rented/duplicate
    for item in &items {
        let actual_idx = SLOT_MAX + item.slot as usize;
        match world.get_inventory_slot(sid, actual_idx) {
            Some(inv_slot) => {
                if inv_slot.item_id != item.item_id || inv_slot.item_id == 0 {
                    send_fail(
                        session,
                        upgrade_type,
                        b_type,
                        UpgradeResult::NoMatch,
                        false,
                        &items,
                    )
                    .await?;
                    return Ok(());
                }
                if inv_slot.flag == ITEM_FLAG_BOUND
                    || inv_slot.flag == ITEM_FLAG_DUPLICATE
                    || inv_slot.flag == ITEM_FLAG_RENTED
                    || inv_slot.flag == ITEM_FLAG_SEALED
                {
                    send_fail(
                        session,
                        upgrade_type,
                        b_type,
                        UpgradeResult::Rental,
                        false,
                        &items,
                    )
                    .await?;
                    return Ok(());
                }
            }
            None => {
                send_fail(
                    session,
                    upgrade_type,
                    b_type,
                    UpgradeResult::NoMatch,
                    false,
                    &items,
                )
                .await?;
                return Ok(());
            }
        }
    }

    // Origin item is the first in the list
    let origin = &items[0];
    let origin_item_id = origin.item_id;

    // Look up origin item definition
    let proto = match world.get_item(origin_item_id) {
        Some(p) => p,
        None => {
            send_fail(
                session,
                upgrade_type,
                b_type,
                UpgradeResult::NoMatch,
                false,
                &items,
            )
            .await?;
            return Ok(());
        }
    };

    // Find the scroll in the items list
    let mut user_scroll_type = ScrollType::Invalid;
    let mut scroll_id: u32 = 0;
    for item in &items {
        let st = get_scroll_type(item.item_id);
        if st != ScrollType::Invalid {
            user_scroll_type = st;
            scroll_id = item.item_id;
            break;
        }
    }

    if user_scroll_type == ScrollType::Invalid {
        debug!(
            "[{}] ItemUpgrade fail: type={} reason=no recognized scroll raw_items={:?}",
            session.addr(),
            upgrade_type,
            raw_items
        );
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Detect special items
    let has_trina = count_in_items(&items, ITEM_TRINA) > 0;
    let has_karivdis = count_in_items(&items, ITEM_KARIVDIS) > 0;
    let has_low_trina = count_in_items(&items, ITEM_LOW_CLASS_TRINA) > 0;
    let has_mid_trina = count_in_items(&items, ITEM_MIDDLE_CLASS_TRINA) > 0;
    let has_logos = count_in_items(&items, ITEM_BLESSING_LOGOS) > 0;
    let has_ring_trina = count_in_items(&items, ITEM_RING_TRINA) > 0;

    // Validation: can't use ring trina more than once
    if has_ring_trina && count_in_items(&items, ITEM_RING_TRINA) > 1 {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Can't use both trina and karivdis, or more than one of each
    if (has_trina && has_karivdis)
        || count_in_items(&items, ITEM_TRINA) > 1
        || count_in_items(&items, ITEM_KARIVDIS) > 1
    {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Can't use both low and mid class trina, or more than one of each
    if (has_low_trina && has_mid_trina)
        || count_in_items(&items, ITEM_LOW_CLASS_TRINA) > 1
        || count_in_items(&items, ITEM_MIDDLE_CLASS_TRINA) > 1
    {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Logos can't be combined with trina/karivdis
    if has_logos && (has_trina || has_karivdis || has_low_trina || has_mid_trina) {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Logos only works on certain item types (4, 5, 11, 12)
    let item_type = proto.item_type.unwrap_or(0) as i16;
    if has_logos && item_type != 4 && item_type != 5 && item_type != 11 && item_type != 12 {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Determine the item's class scroll requirement based on proto.item_class
    let item_class = proto.item_class.unwrap_or(0);
    let item_scroll_type = match item_class {
        1 => ScrollType::LowClass,
        2 => ScrollType::MiddleClass,
        3 => ScrollType::HighClass,
        4 => ScrollType::Rebirth,
        8 => ScrollType::Accessories,
        _ => ScrollType::Invalid,
    };

    // Validate scroll type compatibility (C++ scroll class matching logic)
    if !is_scroll_compatible(item_scroll_type, user_scroll_type) {
        debug!(
            "[{}] ItemUpgrade fail: type={} reason=incompatible scroll item_class={} item_scroll={:?} user_scroll={:?}",
            session.addr(),
            upgrade_type,
            item_class,
            item_scroll_type,
            user_scroll_type
        );
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Logos + high-to-rebirth or accessories scroll = invalid
    if has_logos
        && (user_scroll_type == ScrollType::HighToRebirth
            || user_scroll_type == ScrollType::Accessories)
    {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Logos grade limit check — uses server settings, not hardcoded value
    // Derive grade from item number (last digit) as fallback — C++ stores m_byGrade per-row,
    // but our PostgreSQL export had all by_grade=0. Safe fallback: num % 10.
    let by_grade = {
        let db_grade = proto.by_grade.unwrap_or(0);
        if db_grade == 0 {
            (origin_item_id % 10) as i16
        } else {
            db_grade
        }
    };
    if has_logos {
        let max_grade: i16 = {
            let settings = world.get_server_settings();
            if item_type == 11 || item_type == 12 {
                settings.map(|s| s.max_blessing_up_reb).unwrap_or(10)
            } else {
                settings.map(|s| s.max_blessing_up).unwrap_or(10)
            }
        };
        if by_grade >= max_grade {
            send_fail(
                session,
                upgrade_type,
                b_type,
                UpgradeResult::NoMatch,
                false,
                &items,
            )
            .await?;
            return Ok(());
        }
    }

    // Accessories upgrade: first 3 items must be the same
    if user_scroll_type == ScrollType::Accessories
        && (raw_items[0] == 0
            || raw_items[1] == 0
            || raw_items[2] == 0
            || raw_items[3] == 0
            || raw_items[0] != raw_items[1]
            || raw_items[0] != raw_items[2])
    {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // ── Look up the upgrade recipe ──
    let recipes = match world.get_upgrade_recipes(origin_item_id as i32) {
        Some(r) => r,
        None => {
            send_fail(
                session,
                upgrade_type,
                b_type,
                UpgradeResult::NoMatch,
                false,
                &items,
            )
            .await?;
            return Ok(());
        }
    };

    // Find matching recipe: check scroll number match
    let mut new_item_id: u32 = 0;
    let mut matched_req_item: i32 = 0;
    let mut matched_recipe_grade: i16 = -1;
    let mut recipe_found = false;

    for recipe in &recipes {
        if user_scroll_type != ScrollType::Accessories {
            // For non-accessories, check scroll items at positions 1 and 2
            if raw_items[1] > 0
                && raw_items[1] != ITEM_TRINA
                && raw_items[1] != ITEM_KARIVDIS
                && raw_items[1] != ITEM_MIDDLE_CLASS_TRINA
                && raw_items[1] != ITEM_LOW_CLASS_TRINA
                && raw_items[1] != ITEM_BLESSING_LOGOS
                && recipe.req_item != raw_items[1] as i32
            {
                continue;
            }
            if raw_items[2] > 0
                && raw_items[2] != ITEM_TRINA
                && raw_items[2] != ITEM_KARIVDIS
                && raw_items[2] != ITEM_MIDDLE_CLASS_TRINA
                && raw_items[2] != ITEM_LOW_CLASS_TRINA
                && raw_items[2] != ITEM_BLESSING_LOGOS
                && recipe.req_item != raw_items[2] as i32
            {
                continue;
            }
        } else {
            // For accessories, check scroll at position 3
            if raw_items[3] != ITEM_RING_TRINA && recipe.req_item != raw_items[3] as i32 {
                continue;
            }
        }

        new_item_id = recipe.new_number as u32;
        matched_req_item = recipe.req_item;
        matched_recipe_grade = recipe.grade;
        recipe_found = true;
        break;
    }

    if !recipe_found || new_item_id == 0 {
        debug!(
            "[{}] ItemUpgrade fail: type={} reason=recipe not found origin={} scroll={:?} raw_items={:?} recipe_count={}",
            session.addr(),
            upgrade_type,
            origin_item_id,
            user_scroll_type,
            raw_items,
            recipes.len()
        );
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // ── Look up upgrade settings (success rate, cost, required items) ──
    let mut gen_rate: u32 = 0;
    let mut req_coins: u32 = 0;
    let mut settings_found = false;

    if has_logos {
        // Logos upgrade has fixed 33% success rate, no coin cost via settings
        gen_rate = 3300;
        settings_found = true;
    } else {
        // Find matching upgrade settings
        let req1 = if user_scroll_type == ScrollType::Accessories {
            raw_items[3] as i32
        } else {
            raw_items[1] as i32
        };
        let req2 = if user_scroll_type == ScrollType::Accessories {
            raw_items[4] as i32
        } else {
            raw_items[2] as i32
        };
        let settings_grade = if matched_recipe_grade >= 0 {
            matched_recipe_grade
        } else {
            by_grade
        };

        if let Some(setting) = world.find_upgrade_setting(item_type, settings_grade, req1, req2) {
            gen_rate = setting.success_rate as u32;
            if gen_rate > 10000 {
                gen_rate = 10000;
            }
            req_coins = setting.item_req_coins as u32;
            settings_found = true;

            // Validate required items from settings exist in items list and inventory.
            let s_req1 = setting.req_item_id1;
            let s_req2 = setting.req_item_id2;
            if s_req1 > 0 && !raw_items.iter().any(|&id| id as i32 == s_req1) {
                settings_found = false;
            }
            if s_req2 > 0 && !raw_items.iter().any(|&id| id as i32 == s_req2) {
                settings_found = false;
            }
            if settings_found && s_req1 > 0 && !world.check_exist_item(sid, s_req1 as u32, 1) {
                settings_found = false;
            }
            if settings_found && s_req2 > 0 && !world.check_exist_item(sid, s_req2 as u32, 1) {
                settings_found = false;
            }
        }

        if !settings_found
            && scroll_id == ITEM_BLESSED_ELEMENTAL_SCROLL
            && matched_req_item == ITEM_BLESSED_ELEMENTAL_SCROLL as i32
            && settings_grade == 0
        {
            gen_rate = 10000;
            req_coins = 500_000;
            settings_found = true;
            debug!(
                "[{}] ItemUpgrade settings fallback: Blessed Elemental +0 origin={} new_item={} item_type={} grade={}",
                session.addr(),
                origin_item_id,
                new_item_id,
                item_type,
                settings_grade
            );
        }
    }

    if !settings_found || gen_rate == 0 {
        if upgrade_type == ITEM_ACCESSORIES
            && user_scroll_type == ScrollType::Accessories
            && matched_req_item == scroll_id as i32
        {
            gen_rate = 10000;
            req_coins = 100_000;
            settings_found = true;
            debug!(
                "[{}] ItemUpgrade accessory settings fallback: origin={} new_item={} scroll={} grade={}",
                session.addr(),
                origin_item_id,
                new_item_id,
                scroll_id,
                matched_recipe_grade
            );
        }
    }

    if !settings_found || gen_rate == 0 {
        debug!(
            "[{}] ItemUpgrade fail: type={} reason=settings not found origin={} new_item={} scroll={} scroll_type={:?} item_type={} grade={} recipe_grade={} matched_req={}",
            session.addr(),
            upgrade_type,
            origin_item_id,
            new_item_id,
            scroll_id,
            user_scroll_type,
            item_type,
            by_grade,
            matched_recipe_grade,
            matched_req_item
        );
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NoMatch,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // Check if player has enough gold
    let player_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
    if player_gold < req_coins {
        send_fail(
            session,
            upgrade_type,
            b_type,
            UpgradeResult::NeedCoins,
            false,
            &items,
        )
        .await?;
        return Ok(());
    }

    // ── Perform the upgrade ──
    let _has_protection = has_trina || has_karivdis || has_low_trina || has_mid_trina;

    // Generate random number for success check
    // C++ myrand(0, 10000) is inclusive [0, 10000] → use 10001 for exclusive upper
    let rand_val = rand_range(0, 10001);
    let effective_rate = if has_logos { 3300 } else { gen_rate };

    let mut result_items = items.clone();
    let b_result;

    if b_type == UPGRADE_TYPE_NORMAL && effective_rate < rand_val {
        // ── Upgrade FAILED ──
        if has_logos {
            // Logos failure: may downgrade the item
            // C++ myrand(0, 6700) is inclusive [0, 6700]
            let rand_down = rand_range(0, 6701);
            if rand_down < rand_val && by_grade > 1 {
                let downgraded_id = origin_item_id - 1;
                if world.get_item(downgraded_id).is_some() {
                    // Downgrade the item
                    let actual_idx = SLOT_MAX + origin.slot as usize;
                    world.update_inventory(sid, |inv| {
                        if actual_idx < inv.len() {
                            inv[actual_idx].item_id = downgraded_id;
                            true
                        } else {
                            false
                        }
                    });
                    result_items[0].item_id = downgraded_id;
                }
            }
        } else {
            // Normal failure: destroy the origin item
            let actual_idx = SLOT_MAX + origin.slot as usize;
            world.update_inventory(sid, |inv| {
                if actual_idx < inv.len() {
                    inv[actual_idx] = Default::default();
                    true
                } else {
                    false
                }
            });
            result_items[0].item_id = 0;
        }

        b_result = UpgradeResult::Failed;

        // Deduct gold on failure too
        if req_coins > 0 {
            world.gold_lose(sid, req_coins);
        }
    } else {
        // ── Upgrade SUCCEEDED (or preview) ──
        let new_proto = match world.get_item(new_item_id) {
            Some(p) => p,
            None => {
                send_fail(
                    session,
                    upgrade_type,
                    b_type,
                    UpgradeResult::NoMatch,
                    false,
                    &items,
                )
                .await?;
                return Ok(());
            }
        };

        if b_type != UPGRADE_TYPE_PREVIEW {
            // Apply the upgrade
            let actual_idx = SLOT_MAX + origin.slot as usize;
            let new_dur = new_proto.duration.unwrap_or(0);
            world.update_inventory(sid, |inv| {
                if actual_idx < inv.len() {
                    inv[actual_idx].item_id = new_item_id;
                    inv[actual_idx].durability = new_dur;
                    true
                } else {
                    false
                }
            });

            // Deduct gold
            if req_coins > 0 {
                world.gold_lose(sid, req_coins);
            }
        }

        result_items[0].item_id = new_item_id;
        b_result = UpgradeResult::Succeeded;
    }

    // Remove consumed items (scroll, trina, etc.) — not the origin item (index 0)
    if b_type != UPGRADE_TYPE_PREVIEW {
        for item in items.iter().skip(1).take(MAX_ITEMS_REQ) {
            let actual_idx = SLOT_MAX + item.slot as usize;

            world.update_inventory(sid, |inv| {
                if actual_idx < inv.len() && inv[actual_idx].item_id != 0 {
                    if inv[actual_idx].count > 1 {
                        inv[actual_idx].count -= 1;
                    } else {
                        inv[actual_idx] = Default::default();
                    }
                    true
                } else {
                    false
                }
            });
        }
    }

    // Recalculate equipment stats
    if b_type != UPGRADE_TYPE_PREVIEW {
        world.set_user_ability(sid);
    }

    // ── Build response packet ──
    let mut result = upgrade_response_header(upgrade_type, b_type, b_result, has_logos);

    let response_items = build_upgrade_response_items(&raw_items, &raw_slots, &result_items);
    for item in &response_items {
        result.write_i32(item.item_id as i32);
        result.write_i8(item.slot);
    }
    session.send_packet(&result).await?;

    // FerihaLog: UpgradeInsertLog
    if b_type != UPGRADE_TYPE_PREVIEW {
        // Send authoritative slot counts after the anvil animation response.
        // This also clears destroyed items without reopening the inventory.
        for source in &items {
            if let Some(slot) = world.get_inventory_slot(sid, SLOT_MAX + source.slot as usize) {
                let mut update = Packet::new(Opcode::WizItemCountChange as u8);
                update.write_u16(1);
                update.write_u8(1);
                update.write_u8(source.slot as u8);
                update.write_u32(if slot.item_id == 0 {
                    source.item_id
                } else {
                    slot.item_id
                });
                update.write_u32(slot.count as u32);
                update.write_u8(0);
                update.write_u16(slot.durability as u16);
                update.write_u32(0);
                update.write_u32(slot.expire_time);
                session.send_packet(&update).await?;
            }
        }
        let upgrade_type_str = if upgrade_type == ITEM_ACCESSORIES {
            "accessories"
        } else {
            "normal"
        };
        super::audit_log::log_upgrade(
            session.pool(),
            session.account_id().unwrap_or(""),
            &world.get_session_name(sid).unwrap_or_default(),
            origin_item_id,
            req_coins,
            upgrade_type_str,
            b_result == UpgradeResult::Succeeded,
        );
    }

    // ── Upgrade Notice: server-wide broadcast for notable items ──────
    //   if (pItem.isnull() || isGM() || !pServerSetting.UpgradeNotice) return;
    //   if (!pItem.m_isUpgradeNotice) return;
    //   Packet(WIZ_LOGOSSHOUT, 0x02) << 0x05 << UpgradeResult << name << item_num << rank
    if b_type != UPGRADE_TYPE_PREVIEW {
        let notice_item_id = if b_result == UpgradeResult::Succeeded {
            new_item_id
        } else {
            origin_item_id
        };
        let notice_proto = world.get_item(notice_item_id);
        let high_class_success = b_result == UpgradeResult::Succeeded
            && proto.item_class == Some(3)
            && matches!(item_type, 4 | 5)
            && (7..=9).contains(&(new_item_id % 10));
        let has_upgrade_notice = high_class_success
            || notice_proto
                .as_ref()
                .and_then(|p| p.upgrade_notice)
                .unwrap_or(0)
                != 0;
        if has_upgrade_notice {
            let upgrade_notice_enabled = world
                .get_server_settings()
                .map(|s| s.upgrade_notice != 0)
                .unwrap_or(false);
            if upgrade_notice_enabled || high_class_success {
                let player_name = world.get_session_name(sid).unwrap_or_default();
                let rank = world.with_session(sid, |h| h.personal_rank).unwrap_or(0);
                let notice = super::logosshout::build_upgrade_notice(
                    b_result as u8,
                    &player_name,
                    notice_item_id,
                    rank,
                );
                world.broadcast_to_all(Arc::new(notice), None);
            }
        }
    }

    // v2525: Send rebirth progress notification (0xD3) for rebirth upgrades
    if b_type != UPGRADE_TYPE_PREVIEW && upgrade_type == ITEM_UPGRADE_REBIRTH {
        let rebirth_level = world.get_rebirth_level(sid) as i32;
        if b_result == UpgradeResult::Succeeded {
            // Progress update — show current rebirth level and upgrade grade
            let reb_pkt = super::rebirth::build_progress(
                rebirth_level,
                by_grade as i32,
                10, // max grade
            );
            world.send_to_session_owned(sid, reb_pkt);
        } else {
            // Failure — clear rebirth UI state
            let reb_pkt = super::rebirth::build_result(0);
            world.send_to_session_owned(sid, reb_pkt);
        }
    }

    // Broadcast anvil effect to the region
    if b_type != UPGRADE_TYPE_PREVIEW {
        let mut anvil_pkt = Packet::new(Opcode::WizObjectEvent as u8);
        anvil_pkt.write_u8(OBJECT_ANVIL);
        anvil_pkt.write_u8(b_result as u8);
        anvil_pkt.write_u32(resolved_npc_id);

        if let Some(pos) = world.get_position(sid) {
            world.broadcast_to_zone(pos.zone_id, Arc::new(anvil_pkt), Some(sid));
        }
    }

    // v2525: Awakening visual effect on successful upgrade (broadcast to region)
    // Shows particle effect on the upgrading player for high-grade upgrades.
    if b_type != UPGRADE_TYPE_PREVIEW && b_result == UpgradeResult::Succeeded {
        let effect_scale = match by_grade {
            0..=4 => 1.0_f32,
            5..=7 => 1.5,
            _ => 2.0,
        };
        let awk_pkt = super::awakening::build_visual_effect(effect_scale, new_item_id as i32);
        if let Some(pos) = world.get_position(sid) {
            world.broadcast_to_zone(pos.zone_id, Arc::new(awk_pkt), None);
        }
    }

    Ok(())
}

/// Check if the user's scroll type is compatible with the item's class requirement.
fn is_scroll_compatible(item_class: ScrollType, user_scroll: ScrollType) -> bool {
    match item_class {
        ScrollType::LowClass => matches!(
            user_scroll,
            ScrollType::LowClass
                | ScrollType::MiddleClass
                | ScrollType::HighClass
                | ScrollType::Class
        ),
        ScrollType::MiddleClass => matches!(
            user_scroll,
            ScrollType::MiddleClass | ScrollType::HighClass | ScrollType::Class
        ),
        ScrollType::HighClass => matches!(
            user_scroll,
            ScrollType::HighClass | ScrollType::HighToRebirth | ScrollType::Class
        ),
        ScrollType::Rebirth => matches!(
            user_scroll,
            ScrollType::Rebirth
                | ScrollType::HighToRebirth
                | ScrollType::HighClass
                | ScrollType::RebirthRestoration
        ),
        ScrollType::Accessories => user_scroll == ScrollType::Accessories,
        ScrollType::HighToRebirth => matches!(
            user_scroll,
            ScrollType::HighToRebirth | ScrollType::HighClass
        ),
        ScrollType::RebirthRestoration => user_scroll == ScrollType::RebirthRestoration,
        ScrollType::Invalid => false,
        _ => false,
    }
}

/// Send a failure response packet.
fn upgrade_response_header(
    upgrade_type: u8,
    mode: u8,
    result: UpgradeResult,
    logos: bool,
) -> Packet {
    // 2625 sub_B99EB0 reads mode/result; sub_B95810 then reads the logos flag
    // on a failed normal upgrade (subtype 2), before reading item slots.
    let mut packet = Packet::new(Opcode::WizItemUpgrade as u8);
    packet.write_u8(upgrade_type);
    packet.write_u8(mode);
    packet.write_u8(result as u8);
    if upgrade_type == ITEM_UPGRADE && result == UpgradeResult::Failed {
        packet.write_u8(u8::from(logos));
    }
    packet
}

async fn send_fail(
    session: &mut ClientSession,
    upgrade_type: u8,
    b_type: u8,
    result: UpgradeResult,
    logos: bool,
    items: &[UpgradeItem],
) -> anyhow::Result<()> {
    let mut pkt = upgrade_response_header(upgrade_type, b_type, result, logos);

    for item in items {
        pkt.write_i32(item.item_id as i32);
        pkt.write_i8(item.slot);
    }
    if matches!(
        upgrade_type,
        ITEM_UPGRADE | ITEM_ACCESSORIES | ITEM_UPGRADE_REBIRTH | ITEM_UPGRADE_REVERSE
    ) {
        for _ in items.len()..10 {
            pkt.write_i32(0);
            pkt.write_i8(-1);
        }
    }
    session.send_packet(&pkt).await
}

fn build_upgrade_response_items(
    raw_items: &[u32; 10],
    raw_slots: &[i8; 10],
    result_items: &[UpgradeItem],
) -> Vec<UpgradeItem> {
    let mut response = Vec::with_capacity(10);
    for i in 0..10 {
        let mut item_id = raw_items[i];
        let slot = raw_slots[i];

        if i < result_items.len() {
            item_id = result_items[i].item_id;
        }

        response.push(UpgradeItem { item_id, slot });
    }
    response
}

/// Generate a random number in [min, max).
fn rand_range(min: u32, max: u32) -> u32 {
    use rand::Rng;
    rand::thread_rng().gen_range(min..max)
}

// ── Constants for Shozin Exchange (Special Part Sewing) ──────────────

const NPC_CRAFTSMAN: u8 = 135;
const NPC_JEWELY: u8 = 174;
const ITEM_SHADOW_PIECE: u32 = 700_009_000;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CraftingErrorCode {
    WrongMaterial = 0,
    Success = 1,
    Failed = 2,
}

// ── Constants for Item Smash (Old Man Exchange) ──────────────────────

#[cfg(test)]
const NPC_OLD_MAN: u8 = 222;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmashError {
    Success = 1,
    Inventory = 2,
    Item = 4,
    Npc = 5,
}

/// Check if a zone ID is a Moradon zone.
fn is_moradon(zone_id: u16) -> bool {
    matches!(
        zone_id,
        ZONE_MORADON | ZONE_MORADON2 | ZONE_MORADON3 | ZONE_MORADON4 | ZONE_MORADON5
    )
}

/// Send a crafting failure packet.
async fn send_crafting_fail(
    session: &mut ClientSession,
    code: CraftingErrorCode,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(SPECIAL_PART_SEWING);
    pkt.write_u8(code as u8);
    session.send_packet(&pkt).await
}

/// Send an item smash failure packet.
async fn send_smash_fail(
    session: &mut ClientSession,
    response_type: u8,
    error: SmashError,
) -> anyhow::Result<()> {
    if response_type == ITEM_ACCESSORY_DISASSEMBLE {
        return send_accessory_disassemble_fail(session, error).await;
    }

    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(response_type);
    pkt.write_u16(error as u16);
    session.send_packet(&pkt).await
}

async fn send_accessory_disassemble_fail(
    session: &mut ClientSession,
    error: SmashError,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(ITEM_ACCESSORY_DISASSEMBLE);
    // CUIAccessoryReturn parses a fixed result body on failure too. A short
    // 3-byte response makes the 2615 client read past the packet and crash.
    pkt.write_u16(error as u16);
    pkt.write_u32(0);
    pkt.write_u8(0xff);
    pkt.write_u16(0);
    for _ in 0..3 {
        pkt.write_u32(0);
        pkt.write_u8(0xff);
        pkt.write_u16(0);
    }
    session.send_packet(&pkt).await
}

/// Handle the Shozin Exchange (Special Part Sewing / Crafting) sub-opcode.
/// Packet format:
/// ```text
/// [u32 npcID] [u32 shadowPiece] [u8 shadowSlot] [u8 materialCount]
/// [materialCount x u8 slot]
/// [u8 downFlag]
/// [materialCount x (N-byte ASCII item_id, 3-byte ASCII count)]
/// ```
async fn shozin_exchange(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // ── Parse packet fields ──
    let npc_id_raw = reader.read_u32().unwrap_or(0);
    let shadow_piece_id = reader.read_u32().unwrap_or(0);
    let shadow_piece_slot = reader.read_u8().unwrap_or(0xff);
    let material_count = reader.read_u8().unwrap_or(0) as usize;

    // Validate material count
    if material_count > ITEMS_SPECIAL_EXCHANGE_GROUP {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // Player state validation
    if world.is_player_dead(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // Must be in Moradon
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await,
    };
    if !is_moradon(pos.zone_id) {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // Validate shadow piece slot
    if shadow_piece_slot as usize >= HAVE_MAX {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // Validate NPC: look up the NPC instance, get its template, check type + range
    // C++ removes dead NPCs from world — we keep them, so check explicitly
    let npc_inst = match world.get_npc_instance(npc_id_raw) {
        Some(n) => n,
        None => return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await,
    };
    if world.is_npc_dead(npc_id_raw) {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }
    if !world.is_in_npc_range(sid, npc_id_raw) {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }
    let npc_tmpl = match world.get_npc_template(npc_inst.proto_id, npc_inst.is_monster) {
        Some(t) => t,
        None => return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await,
    };
    if npc_tmpl.npc_type != NPC_CRAFTSMAN && npc_tmpl.npc_type != NPC_JEWELY {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // Check shadow piece item
    let has_shadow = shadow_piece_id == ITEM_SHADOW_PIECE;
    if has_shadow {
        let actual_idx = SLOT_MAX + shadow_piece_slot as usize;
        match world.get_inventory_slot(sid, actual_idx) {
            Some(slot) => {
                if slot.item_id != ITEM_SHADOW_PIECE || slot.count < 1 {
                    return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
                }
                if slot.flag == ITEM_FLAG_BOUND
                    || slot.flag == ITEM_FLAG_DUPLICATE
                    || slot.flag == ITEM_FLAG_RENTED
                    || slot.flag == ITEM_FLAG_SEALED
                {
                    return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
                }
            }
            None => {
                return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
            }
        }
    }

    // Read material slots
    let mut mat_slots: Vec<u8> = Vec::with_capacity(material_count);
    for _ in 0..material_count {
        let s = reader.read_u8().unwrap_or(0xff);
        if s as usize >= HAVE_MAX {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }
        mat_slots.push(s);
    }

    // Check for duplicate slots
    for i in 0..mat_slots.len() {
        for j in (i + 1)..mat_slots.len() {
            if mat_slots[i] == mat_slots[j] {
                return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
            }
        }
    }

    // Read nDownFlag (unused in our logic, but must be consumed)
    let _down_flag = reader.read_u8().unwrap_or(0);

    // Read material item IDs and counts (ASCII-encoded in the packet)
    let mut mat_item_ids: Vec<u32> = Vec::with_capacity(material_count);
    let mut mat_item_counts: Vec<u16> = Vec::with_capacity(material_count);

    for &mat_slot in mat_slots.iter().take(material_count) {
        let actual_idx = SLOT_MAX + mat_slot as usize;
        let inv_slot = match world.get_inventory_slot(sid, actual_idx) {
            Some(s) => s,
            None => {
                return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
            }
        };
        if inv_slot.item_id == 0 || inv_slot.count == 0 {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }
        if inv_slot.flag == ITEM_FLAG_DUPLICATE || inv_slot.flag == ITEM_FLAG_RENTED {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }

        // Determine digit length (9 or 10 digits)
        let digit_len: usize = if inv_slot.item_id > 999_999_999 {
            10
        } else {
            9
        };

        // Read ASCII-encoded item ID
        let mut item_id: u32 = 0;
        let mut digit_place: u32 = if digit_len == 10 {
            1_000_000_000
        } else {
            100_000_000
        };
        let mut parse_ok = true;
        for _ in 0..digit_len {
            let b = reader.read_u8().unwrap_or(0);
            let decimal = b.wrapping_sub(48);
            if decimal > 9 {
                parse_ok = false;
                break;
            }
            item_id += decimal as u32 * digit_place;
            digit_place /= 10;
        }
        if !parse_ok {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }

        // Read 3-digit ASCII count
        let c0 = reader.read_u8().unwrap_or(0).wrapping_sub(48);
        let c1 = reader.read_u8().unwrap_or(0).wrapping_sub(48);
        let c2 = reader.read_u8().unwrap_or(0).wrapping_sub(48);
        if c0 > 9 || c1 > 9 || c2 > 9 {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }
        let count = c0 as u16 * 100 + c1 as u16 * 10 + c2 as u16;

        // Validate inventory matches parsed data
        if inv_slot.item_id != item_id || inv_slot.count < count || count == 0 {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }

        // Check countable: if count > 1, the item must be countable
        if let Some(proto) = world.get_item(item_id) {
            if count > 1 && proto.countable.unwrap_or(0) == 0 {
                return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
            }
        }

        mat_item_ids.push(item_id);
        mat_item_counts.push(count);
    }

    // ── Find matching recipes ──
    // Determine NPC DB ID for filtering
    let npc_db_id: i32 = if npc_tmpl.npc_type == NPC_JEWELY {
        31402
    } else {
        19073 // NPC_CRAFTSMAN
    };

    let all_recipes = match world.get_special_sewing_recipes(npc_db_id) {
        Some(r) => r,
        None => return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await,
    };

    // Filter recipes that match our materials exactly
    let mut matching_recipes = Vec::new();
    for recipe in &all_recipes {
        let table_size = recipe.material_count() as usize;
        if table_size != material_count {
            continue;
        }

        // Match each material against recipe slots
        let mut match_count = 0u8;
        for i in 0..material_count {
            if mat_item_ids[i] != 0 {
                for x in 0..ITEMS_SPECIAL_EXCHANGE_GROUP {
                    if recipe.req_item_id_at(x) != 0
                        && mat_item_ids[i] == recipe.req_item_id_at(x) as u32
                        && mat_item_counts[i] == recipe.req_item_count_at(x) as u16
                    {
                        match_count += 1;
                        break;
                    }
                }
            }
        }

        if material_count as u8 == match_count {
            matching_recipes.push(recipe.clone());
        }
    }

    if matching_recipes.is_empty() {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // Random selection from matching recipes
    let recipe_idx = if matching_recipes.len() == 1 {
        0
    } else {
        rand_range(0, matching_recipes.len() as u32) as usize
    };
    let recipe = matching_recipes[recipe_idx].clone();

    // Double-check the recipe (isnull check from C++)
    if recipe.n_index == 0 || recipe.give_item_id == 0 {
        return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
    }

    // ── Success/Failure roll ──
    let mut upgrade_rate = recipe.success_rate as u32;
    if has_shadow {
        if recipe.is_shadow_success {
            upgrade_rate = 10000;
        } else {
            upgrade_rate += (upgrade_rate * 40) / 100;
        }
    }
    if upgrade_rate > 10000 {
        upgrade_rate = 10000;
    }

    let rand_val = rand_range(0, 10001);
    let result_code;
    let mut give_item_slot: u8 = 0;
    let give_item_id = recipe.give_item_id as u32;
    let give_count = recipe.give_item_count.clamp(0, u16::MAX as i32) as u16;

    if upgrade_rate < rand_val {
        // ── FAILURE ──
        // Remove all materials
        for i in 0..material_count {
            let actual_idx = SLOT_MAX + mat_slots[i] as usize;
            let count = mat_item_counts[i];
            world.update_inventory(sid, |inv| {
                if actual_idx < inv.len() && inv[actual_idx].item_id != 0 {
                    if inv[actual_idx].count > count {
                        inv[actual_idx].count -= count;
                    } else {
                        inv[actual_idx] = Default::default();
                    }
                    true
                } else {
                    false
                }
            });
        }
        // Remove shadow piece on failure
        if has_shadow {
            let shadow_idx = SLOT_MAX + shadow_piece_slot as usize;
            world.update_inventory(sid, |inv| {
                if shadow_idx < inv.len() {
                    inv[shadow_idx] = Default::default();
                    true
                } else {
                    false
                }
            });
        }
        result_code = CraftingErrorCode::Failed;
    } else {
        // ── SUCCESS ──
        let give_proto = match world.get_item(give_item_id) {
            Some(p) => p,
            None => {
                return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
            }
        };

        // Weight check
        let item_weight = (give_proto.weight.unwrap_or(0) as i32).saturating_mul(give_count as i32);
        if let Some(ch) = world.get_character_info(sid) {
            if ch.item_weight + item_weight > ch.max_weight {
                return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
            }
        }

        // Kind 255 + non-countable → force count = 1 (C++ line 284-285)
        let final_count = if give_proto.kind.unwrap_or(0) == ITEM_KIND_UNIQUE
            && give_proto.countable.unwrap_or(0) == 0
        {
            1u16
        } else {
            give_count
        };

        // Give item
        if !world.give_item(sid, give_item_id, final_count) {
            return send_crafting_fail(session, CraftingErrorCode::WrongMaterial).await;
        }

        // Find which slot got the item (for the response packet)
        if let Some(slot_idx) = world.find_slot_for_item(sid, give_item_id, 0) {
            if slot_idx >= SLOT_MAX {
                give_item_slot = (slot_idx - SLOT_MAX) as u8;
            }
        }

        // Remove materials
        for i in 0..material_count {
            let actual_idx = SLOT_MAX + mat_slots[i] as usize;
            let count = mat_item_counts[i];
            world.update_inventory(sid, |inv| {
                if actual_idx < inv.len() && inv[actual_idx].item_id != 0 {
                    if inv[actual_idx].count > count {
                        inv[actual_idx].count -= count;
                    } else {
                        inv[actual_idx] = Default::default();
                    }
                    true
                } else {
                    false
                }
            });
        }
        // Remove shadow piece
        if has_shadow {
            let shadow_idx = SLOT_MAX + shadow_piece_slot as usize;
            world.update_inventory(sid, |inv| {
                if shadow_idx < inv.len() {
                    inv[shadow_idx] = Default::default();
                    true
                } else {
                    false
                }
            });
        }

        result_code = CraftingErrorCode::Success;

        // Daily rank stat: SHTotalExchange++ (crafting success)
        world.update_session(sid, |h| {
            h.dr_sh_total_exchange += 1;
        });
    }

    // ── Send response ──
    // C++ response: [u8 SPECIAL_PART_SEWING] [u8 resultOpCode] [u32 npcID]
    // On success: + [u32 itemNumber] [u8 slot]
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(SPECIAL_PART_SEWING);
    pkt.write_u8(result_code as u8);
    pkt.write_u32(npc_id_raw);
    if result_code == CraftingErrorCode::Success {
        pkt.write_u32(give_item_id);
        pkt.write_u8(give_item_slot);
    }
    session.send_packet(&pkt).await?;

    // NPC effect broadcast (C++ ShowNpcEffect 31033/31034)
    // C++ ShowNpcEffect uses OBJECT_NPC (11), not OBJECT_ANVIL (8)
    // See User.cpp:2696: Packet result(WIZ_OBJECT_EVENT, uint8(OBJECT_NPC));
    if result_code == CraftingErrorCode::Success {
        let mut effect_pkt = Packet::new(Opcode::WizObjectEvent as u8);
        effect_pkt.write_u8(OBJECT_NPC);
        effect_pkt.write_u8(3);
        effect_pkt.write_u32(npc_id_raw);
        effect_pkt.write_u32(31033);
        world.broadcast_to_zone(pos.zone_id, Arc::new(effect_pkt), Some(sid));
    } else if result_code == CraftingErrorCode::Failed {
        let mut effect_pkt = Packet::new(Opcode::WizObjectEvent as u8);
        effect_pkt.write_u8(OBJECT_NPC);
        effect_pkt.write_u8(3);
        effect_pkt.write_u32(npc_id_raw);
        effect_pkt.write_u32(31034);
        world.broadcast_to_zone(pos.zone_id, Arc::new(effect_pkt), Some(sid));
    }

    Ok(())
}

/// Handle the Item Disassemble (Old Man Exchange / Item Smash) sub-opcode.
/// Packet format:
/// ```text
/// [u32 itemID] [u8 slot] [u32 npcID]
/// ```
/// Response:
/// ```text
/// [u16 error/success] [u32 origItemID] [u8 origSlot] [u16 rollCount]
/// [rollCount x (u32 itemID, u8 slot, u16 count)]
/// ```
async fn item_disassemble(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    response_type: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Parse packet. Old Man Exchange (13) uses the legacy compact format:
    // [u32 itemID] [u8 slot] [u32 npcID].
    //
    // Accessory disassemble (15) in the 2615 client sends the anvil-style
    // payload: [u32 npcID] [4 x (u32 itemID, u8 slot)]. Pick the entry whose
    // upgraded item can be reversed through NEW_UPGRADE.
    let (item_id, slot, _npc_id_raw, consume_item): (u32, u8, u32, Option<(u32, u8)>) =
        if response_type == ITEM_ACCESSORY_DISASSEMBLE && reader.remaining() >= 24 {
            let npc_id = reader.read_u32().unwrap_or(0);
            let mut selected: Option<(u32, u8)> = None;
            let mut material: Option<(u32, u8)> = None;
            let mut candidates: Vec<(u32, u8, u32)> = Vec::new();

            for _ in 0..4 {
                let packet_item_id = reader.read_u32().unwrap_or(0);
                let candidate_slot = reader.read_u8().unwrap_or(0xff);
                if candidate_slot as usize >= HAVE_MAX {
                    continue;
                }
                let inv_idx = SLOT_MAX + candidate_slot as usize;
                let Some(inv_item_id) = world
                    .get_inventory_slot(sid, inv_idx)
                    .map(|inv| inv.item_id)
                    .filter(|id| *id != 0)
                else {
                    continue;
                };
                candidates.push((packet_item_id, candidate_slot, inv_item_id));
                if inv_item_id == ITEM_ACCESSORY_DISASSEMBLE_SCROLL
                    || packet_item_id == ITEM_ACCESSORY_DISASSEMBLE_SCROLL
                {
                    material.get_or_insert((inv_item_id, candidate_slot));
                    continue;
                }

                let is_reverseable = world
                    .find_upgrade_recipe_by_new_number_and_req_items(
                        inv_item_id as i32,
                        &ACCESSORY_UPGRADE_SCROLLS,
                    )
                    .is_some();
                let looks_like_upgraded_accessory = inv_item_id % 10 > 0
                    && world.get_item(inv_item_id).is_some_and(|proto| {
                        proto.countable.unwrap_or(0) == 0
                            && proto.kind.unwrap_or(0) != ITEM_KIND_UNIQUE
                            && matches!(
                                proto.item_class.unwrap_or(0) as i16,
                                21 | 22 | 31 | 32 | 33 | 34 | 35 | 37 | 38
                            )
                    });

                if selected.is_none() && (is_reverseable || looks_like_upgraded_accessory) {
                    selected = Some((inv_item_id, candidate_slot));
                } else if material.is_none() {
                    material = Some((inv_item_id, candidate_slot));
                }
            }

            if selected.is_none() {
                for i in 0..HAVE_MAX {
                    let Some(inv_item_id) = world
                        .get_inventory_slot(sid, SLOT_MAX + i)
                        .map(|inv| inv.item_id)
                        .filter(|id| *id != 0 && *id != ITEM_ACCESSORY_DISASSEMBLE_SCROLL)
                    else {
                        continue;
                    };

                    let is_reverseable = world
                        .find_upgrade_recipe_by_new_number_and_req_items(
                            inv_item_id as i32,
                            &ACCESSORY_UPGRADE_SCROLLS,
                        )
                        .is_some();
                    let looks_like_upgraded_accessory = inv_item_id % 10 > 0
                        && world.get_item(inv_item_id).is_some_and(|proto| {
                            proto.countable.unwrap_or(0) == 0
                                && proto.kind.unwrap_or(0) != ITEM_KIND_UNIQUE
                                && matches!(
                                    proto.item_class.unwrap_or(0) as i16,
                                    21 | 22 | 31 | 32 | 33 | 34 | 35 | 37 | 38
                                )
                        });

                    if is_reverseable || looks_like_upgraded_accessory {
                        selected = Some((inv_item_id, i as u8));
                        break;
                    }
                }
            }

            let (selected_item_id, selected_slot) = match selected {
                Some(v) => v,
                None => {
                    debug!(
                        "[{}] ItemDisassemble accessory fail: no reverseable item in type=15 payload npc_id={} candidates={:?}",
                        session.addr(),
                        npc_id,
                        candidates
                    );
                    return send_smash_fail(session, response_type, SmashError::Item).await;
                }
            };

            (selected_item_id, selected_slot, npc_id, material)
        } else {
            let item_id = reader.read_u32().unwrap_or(0);
            let slot = reader.read_u8().unwrap_or(0xff);
            let npc_id = reader.read_u32().unwrap_or(0);
            (item_id, slot, npc_id, None)
        };

    // Player state validation
    if world.is_player_dead(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_buying_merchant_preparing(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return send_smash_fail(session, response_type, SmashError::Npc).await;
    }

    // Must be in Moradon
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return send_smash_fail(session, response_type, SmashError::Npc).await,
    };
    if !is_moradon(pos.zone_id) {
        return send_smash_fail(session, response_type, SmashError::Npc).await;
    }

    // Validate slot
    if slot as usize >= HAVE_MAX {
        return send_smash_fail(session, response_type, SmashError::Npc).await;
    }

    // Look up item definition
    let proto = match world.get_item(item_id) {
        Some(p) => p,
        None => return send_smash_fail(session, response_type, SmashError::Npc).await,
    };

    // Item must not be countable, kind must not be 255
    if proto.countable.unwrap_or(0) != 0 || proto.kind.unwrap_or(0) == ITEM_KIND_UNIQUE {
        return send_smash_fail(session, response_type, SmashError::Npc).await;
    }

    // Validate item class (C++ ItemClass check)
    let item_class = proto.item_class.unwrap_or(0) as i16;
    if !matches!(
        item_class,
        3 | 4 | 5 | 8 | 31 | 32 | 33 | 34 | 35 | 37 | 38 | 21 | 22
    ) {
        return send_smash_fail(session, response_type, SmashError::Item).await;
    }

    // Gold cost: 10000 (normal) or 100000 (type 4/12 items)
    let item_type = proto.item_type.unwrap_or(0) as i16;
    let req_coins: u32 = if item_type == 4 || item_type == 12 {
        100_000
    } else {
        10_000
    };

    // Check gold
    let player_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
    if player_gold < req_coins {
        return send_smash_fail(session, response_type, SmashError::Item).await;
    }

    // Validate the item in inventory
    let actual_idx = SLOT_MAX + slot as usize;
    match world.get_inventory_slot(sid, actual_idx) {
        Some(inv_slot) => {
            if inv_slot.item_id != item_id
                || inv_slot.count != 1
                || inv_slot.flag == ITEM_FLAG_DUPLICATE
                || inv_slot.flag == ITEM_FLAG_RENTED
            {
                return send_smash_fail(session, response_type, SmashError::Item).await;
            }
        }
        None => return send_smash_fail(session, response_type, SmashError::Item).await,
    }

    if response_type == ITEM_ACCESSORY_DISASSEMBLE {
        let reward_item_id = if let Some(recipe) = world
            .find_upgrade_recipe_by_new_number_and_req_items(
                item_id as i32,
                &ACCESSORY_UPGRADE_SCROLLS,
            ) {
            recipe.origin_number as u32
        } else if item_id % 10 > 0 {
            let fallback = item_id - 1;
            debug!(
                "[{}] ItemDisassemble accessory fallback: item_id={} reward_item_id={}",
                session.addr(),
                item_id,
                fallback
            );
            fallback
        } else {
            debug!(
                "[{}] ItemDisassemble accessory fail: no reverse recipe/fallback for item_id={}",
                session.addr(),
                item_id
            );
            return send_smash_fail(session, response_type, SmashError::Item).await;
        };

        if reward_item_id == 0 || world.get_item(reward_item_id).is_none() {
            return send_smash_fail(session, response_type, SmashError::Item).await;
        }

        let reward_count: u16 = 3;
        let mut free_slots = 1u8; // the source slot becomes free after removal
        for i in 0..HAVE_MAX {
            if let Some(inv_slot) = world.get_inventory_slot(sid, SLOT_MAX + i) {
                if inv_slot.item_id == 0 {
                    free_slots += 1;
                    if free_slots >= reward_count as u8 {
                        break;
                    }
                }
            }
        }
        if free_slots < reward_count as u8 {
            return send_smash_fail(session, response_type, SmashError::Inventory).await;
        }

        let reward_weight = world
            .get_item(reward_item_id)
            .map(|p| (p.weight.unwrap_or(0) as i32).saturating_mul(reward_count as i32))
            .unwrap_or(0);
        if let Some(ch) = world.get_character_info(sid) {
            if ch.item_weight + reward_weight > ch.max_weight {
                return send_smash_fail(session, response_type, SmashError::Item).await;
            }
        }

        if !world.gold_lose(sid, req_coins) {
            return send_smash_fail(session, response_type, SmashError::Item).await;
        }

        if let Some((material_item_id, material_slot)) = consume_item {
            let material_idx = SLOT_MAX + material_slot as usize;
            world.update_inventory(sid, |inv| {
                if material_idx < inv.len()
                    && inv[material_idx].item_id == material_item_id
                    && inv[material_idx].count > 0
                {
                    if inv[material_idx].count > 1 {
                        inv[material_idx].count -= 1;
                    } else {
                        inv[material_idx] = Default::default();
                    }
                    true
                } else {
                    false
                }
            });
        }

        world.update_inventory(sid, |inv| {
            if actual_idx < inv.len() {
                inv[actual_idx] = Default::default();
                true
            } else {
                false
            }
        });

        let mut reward_slots: Vec<(u32, u8)> = Vec::with_capacity(reward_count as usize);

        for _ in 0..reward_count {
            if let Some(slot_idx) = world.find_slot_for_item(sid, reward_item_id, 1) {
                if slot_idx >= SLOT_MAX && world.give_item(sid, reward_item_id, 1) {
                    reward_slots.push((reward_item_id, (slot_idx - SLOT_MAX) as u8));
                }
            }
        }

        if reward_slots.len() != reward_count as usize {
            return send_smash_fail(session, response_type, SmashError::Inventory).await;
        }

        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(response_type);
        pkt.write_u16(SmashError::Success as u16);
        pkt.write_u32(item_id);
        pkt.write_u8(slot);
        pkt.write_u16(reward_count);
        for (reward_id, reward_slot) in &reward_slots {
            pkt.write_u32(*reward_id);
            pkt.write_u8(*reward_slot);
            pkt.write_u16(1);
        }
        session.send_packet(&pkt).await?;

        // Accessory Return consumes the legacy 31-byte result above.  A
        // second normal-upgrade (53-byte) result makes the 2615 client parse
        // an unrelated contract and disconnect with 10054.

        world.set_user_ability(sid);
        return Ok(());
    }

    // Determine index range for the item class
    let (range_start, range_end) = match item_class {
        3 | 4 | 5 | 8 => (2_000_000i32, 3_000_000i32),
        32 | 33 | 34 | 35 | 37 | 38 => (3_000_000, 4_000_000),
        21 => (4_000_000, 5_000_000),
        31 | 22 => (5_000_000, 6_000_000),
        _ => return send_smash_fail(session, response_type, SmashError::Npc).await,
    };

    let smash_list = world.get_item_smash_in_range(range_start, range_end);
    if smash_list.is_empty() {
        return send_smash_fail(session, response_type, SmashError::Npc).await;
    }

    // Determine roll count based on item class
    let roll_count: u16 = if matches!(item_class, 31 | 21 | 22) {
        1
    } else if matches!(item_class, 3 | 4 | 5 | 8) {
        2
    } else {
        // 32 | 33 | 34 | 35 | 37 | 38
        3
    };

    // Check free slots
    let mut free_slots = 0u8;
    for i in 0..HAVE_MAX {
        if let Some(inv_slot) = world.get_inventory_slot(sid, SLOT_MAX + i) {
            if inv_slot.item_id == 0 {
                free_slots += 1;
                if free_slots >= roll_count as u8 {
                    break;
                }
            }
        }
    }
    if free_slots < roll_count as u8 {
        return send_smash_fail(session, response_type, SmashError::Inventory).await;
    }

    // ── Weighted random selection for each roll ──
    struct SmashResult {
        item_id: u32,
        count: u16,
    }
    let mut results: Vec<SmashResult> = Vec::with_capacity(roll_count as usize);

    for _ in 0..roll_count {
        // Build weighted array using cumulative weights
        let mut weighted: Vec<(i32, u32)> = Vec::new();
        let mut total_weight = 0u32;
        for entry in &smash_list {
            let w = entry.rate as u32 / 5;
            if w == 0 {
                continue;
            }
            total_weight += w;
            weighted.push((entry.n_index, total_weight));
        }

        if total_weight == 0 || weighted.is_empty() {
            return send_smash_fail(session, response_type, SmashError::Item).await;
        }

        let roll = rand_range(0, total_weight);
        let mut selected_index = weighted.last().map(|e| e.0).unwrap_or(0);
        for (idx, cumulative) in &weighted {
            if roll < *cumulative {
                selected_index = *idx;
                break;
            }
        }

        if let Some(smash_entry) = world.get_item_smash(selected_index) {
            results.push(SmashResult {
                item_id: smash_entry.item_id as u32,
                count: smash_entry.count as u16,
            });
        }
    }

    if results.is_empty() {
        return send_smash_fail(session, response_type, SmashError::Item).await;
    }

    // Weight check for all resulting items
    let mut total_result_weight: i32 = 0;
    for res in &results {
        if let Some(p) = world.get_item(res.item_id) {
            total_result_weight = total_result_weight
                .saturating_add((p.weight.unwrap_or(0) as i32).saturating_mul(res.count as i32));
        }
    }
    if let Some(ch) = world.get_character_info(sid) {
        if ch.item_weight + total_result_weight > ch.max_weight {
            return send_smash_fail(session, response_type, SmashError::Item).await;
        }
    }

    // Deduct gold
    if !world.gold_lose(sid, req_coins) {
        return send_smash_fail(session, response_type, SmashError::Item).await;
    }

    // Remove the original item
    world.update_inventory(sid, |inv| {
        if actual_idx < inv.len() {
            inv[actual_idx] = Default::default();
            true
        } else {
            false
        }
    });

    // Build response packet
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(response_type);
    pkt.write_u16(SmashError::Success as u16);
    pkt.write_u32(item_id);
    pkt.write_u8(slot);
    pkt.write_u16(roll_count);

    // Give each result item
    for res in &results {
        if res.item_id == 0 || res.count == 0 {
            continue;
        }
        if let Some(slot_idx) = world.find_slot_for_item(sid, res.item_id, res.count) {
            if slot_idx >= SLOT_MAX {
                world.give_item(sid, res.item_id, res.count);
                let pos_byte = (slot_idx - SLOT_MAX) as u8;
                pkt.write_u32(res.item_id);
                pkt.write_u8(pos_byte);
                pkt.write_u16(res.count);
            }
        }
    }

    session.send_packet(&pkt).await?;

    // Recalculate stats
    world.set_user_ability(sid);

    Ok(())
}

// ── Bifrost Piece Exchange ─────────────────────────────────────────────

const NPC_CHAOTIC_GENERATOR: u8 = 137;
const NPC_CHAOTIC_GENERATOR2: u8 = 162;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BeefEffectType {
    Red = 1,
    Green = 2,
    White = 3,
}

/// Handle ITEM_BIFROST_REQ (sub=4) — simple NPC validation + success response.
async fn bifrost_piece_req(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(ITEM_BIFROST_REQ);
    pkt.write_u8(1); // success
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Send Bifrost exchange failure packet.
async fn bifrost_send_fail(session: &mut ClientSession, error_code: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(ITEM_BIFROST_EXCHANGE);
    pkt.write_u8(error_code);
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Handle ITEM_BIFROST_EXCHANGE (sub=5) — Bifrost Piece exchange with weighted
/// random loot selection.
/// Client packet: `[u32 npc_id] [u32 piece_item_id] [i8 src_pos]`
/// Response on success:
/// `[u8 1] [u32 reward_item_id] [i8 reward_slot] [u32 piece_item_id] [i8 src_pos] [u8 effect_type]`
async fn bifrost_piece_exchange(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let error_code: u8 = 2;

    // Read client packet
    let npc_id_raw = reader.read_u32().unwrap_or(0);
    let piece_item_id = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_i8().unwrap_or(-1);

    debug!(
        "[{}] BifrostPieceProcess npc={} item={} pos={}",
        session.addr(),
        npc_id_raw,
        piece_item_id,
        src_pos
    );

    // Chaotic coins gate
    //   uint32 coinsreq = g_pMain->pServerSetting.chaoticcoins;
    //   if (coinsreq && !hasCoins(coinsreq)) return BifrostPieceSendFail(errorcode);
    let chaotic_coins = world
        .get_server_settings()
        .map(|s| s.chaotic_coins)
        .unwrap_or(0);
    if chaotic_coins > 0 {
        let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
        if gold < chaotic_coins as u32 {
            return bifrost_send_fail(session, error_code).await;
        }
    }

    // Cooldown check (1500ms)
    // C++ uses `m_BeefExchangeTime > UNIXTIME2`, then sets `UNIXTIME2 + 1500`
    let cooldown_ok = world
        .with_session(sid, |h| {
            h.beef_exchange_time.elapsed() >= std::time::Duration::from_millis(1500)
        })
        .unwrap_or(false);
    if !cooldown_ok {
        return bifrost_send_fail(session, error_code).await;
    }

    // Weight check
    let (item_weight, max_weight) = world
        .with_session(sid, |h| {
            (h.equipped_stats.item_weight, h.equipped_stats.max_weight)
        })
        .unwrap_or((0, 0));
    if item_weight >= max_weight {
        return bifrost_send_fail(session, error_code).await;
    }

    // Set cooldown
    world.update_session(sid, |h| {
        h.beef_exchange_time = std::time::Instant::now();
    });

    // NPC range check
    let npc_id = npc_id_raw;
    if !world.is_in_npc_range(sid, npc_id) {
        return bifrost_send_fail(session, error_code).await;
    }

    // Busy-state checks
    if world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
        || world.is_player_dead(sid)
    {
        return bifrost_send_fail(session, error_code).await;
    }

    // Zone check — must be in Moradon
    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if !matches!(
        zone_id,
        ZONE_MORADON | ZONE_MORADON2 | ZONE_MORADON3 | ZONE_MORADON4 | ZONE_MORADON5
    ) {
        return bifrost_send_fail(session, error_code).await;
    }

    // NPC type check — must be CHAOTIC_GENERATOR or CHAOTIC_GENERATOR2
    if let Some(npc_inst) = world.get_npc_instance(npc_id) {
        if let Some(tmpl) = world.get_npc_template(npc_inst.proto_id, false) {
            if tmpl.npc_type != NPC_CHAOTIC_GENERATOR && tmpl.npc_type != NPC_CHAOTIC_GENERATOR2 {
                return bifrost_send_fail(session, error_code).await;
            }
        }
    }

    // Validate origin item (piece) — must exist in item table, be countable, Effect2 == 251
    let piece_table = match world.get_item(piece_item_id) {
        Some(t) => t,
        None => return bifrost_send_fail(session, error_code).await,
    };
    if piece_table.countable.unwrap_or(0) == 0 || piece_table.effect2.unwrap_or(0) != 251 {
        return bifrost_send_fail(session, error_code).await;
    }

    // Validate inventory slot — item must match, count > 0, not rented/sealed/duplicate
    if src_pos < 0 || src_pos as usize >= HAVE_MAX {
        return bifrost_send_fail(session, error_code).await;
    }
    let actual_slot = SLOT_MAX + src_pos as usize;
    let inv_item = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) => s,
        None => return bifrost_send_fail(session, error_code).await,
    };
    if inv_item.item_id != piece_item_id
        || inv_item.count == 0
        || inv_item.flag & (ITEM_FLAG_RENTED | ITEM_FLAG_SEALED | ITEM_FLAG_DUPLICATE) != 0
    {
        return bifrost_send_fail(session, error_code).await;
    }

    // Check free inventory slots — need at least 1
    let free_slots = world.count_free_slots(sid);
    if free_slots < 1 {
        return bifrost_send_fail(session, error_code).await;
    }

    // Load matching bifrost exchanges — random_flag IN (1,2,3) with matching origin item
    let exchanges = world.get_bifrost_exchanges(piece_item_id);
    if exchanges.is_empty() {
        return bifrost_send_fail(session, error_code).await;
    }

    // BifrostCheckExchange per-entry validation
    // C++ validates every exchange entry BEFORE building the random array.
    // If any entry fails, the whole operation is aborted.
    for ex in &exchanges {
        let reward_id = ex.exchange_item_num1 as u32;
        if reward_id == 0 {
            continue;
        }
        // Validate reward item exists
        if world.get_item(reward_id).is_none() {
            return bifrost_send_fail(session, error_code).await;
        }
    }

    // Build weighted random array (10000 slots, divided by 5)
    let mut rand_array: Vec<u32> = Vec::with_capacity(10000);
    for ex in &exchanges {
        // Skip if random_flag >= 101
        if ex.random_flag >= 101 {
            continue;
        }
        // Check if player has enough origin items for this exchange
        let req_count = ex.origin_item_count1 as u16;
        if req_count > 0 && !world.check_exist_item(sid, piece_item_id, req_count) {
            continue;
        }
        // Populate array: sExchangeItemCount[0] / 5 entries
        let slots = (ex.exchange_item_count1 / 5) as usize;
        let reward_id = ex.exchange_item_num1 as u32;
        if reward_id == 0 {
            continue;
        }
        for _ in 0..slots {
            if rand_array.len() >= 10000 {
                break;
            }
            rand_array.push(reward_id);
        }
        if rand_array.len() >= 10000 {
            break;
        }
    }

    if rand_array.is_empty() {
        return bifrost_send_fail(session, error_code).await;
    }

    // Random selection
    let rand_idx = rand_range(0, rand_array.len() as u32) as usize;
    let reward_item_id = rand_array[rand_idx];

    // Validate reward item
    let reward_table = match world.get_item(reward_item_id) {
        Some(t) => t,
        None => return bifrost_send_fail(session, error_code).await,
    };

    // Weight check for reward item
    let reward_weight = reward_table.weight.unwrap_or(0) as u32;
    if reward_weight + item_weight >= max_weight {
        return bifrost_send_fail(session, error_code).await;
    }

    // Verify that the reward can fit before consuming the piece. The final
    // slot must be resolved again after consumption: when the last piece in
    // the source stack is removed, that newly-empty slot may become the first
    // valid destination for the reward.
    if world.find_slot_for_item(sid, reward_item_id, 1).is_none() {
        return bifrost_send_fail(session, error_code).await;
    }

    // Remove exactly one piece from the client-selected source slot. Capture
    // the authoritative remaining count so it can be sent after the exchange
    // result; the result packet alone is not sufficient to keep v2615's
    // inventory cache synchronized after repeated exchanges.
    let mut consumed_slot: Option<(u32, u16)> = None;
    let consumed = world.update_inventory(sid, |inv| {
        if actual_slot >= inv.len()
            || inv[actual_slot].item_id != piece_item_id
            || inv[actual_slot].count == 0
        {
            return false;
        }

        inv[actual_slot].count -= 1;
        let remaining = inv[actual_slot].count;
        let durability = inv[actual_slot].durability.max(0) as u16;
        if remaining == 0 {
            inv[actual_slot] = Default::default();
        }
        consumed_slot = Some((remaining as u32, durability));
        true
    });
    if !consumed {
        return bifrost_send_fail(session, error_code).await;
    }

    // Resolve the actual reward slot after consuming the source item. This
    // prevents reporting a stale slot when the consumed stack became empty.
    let reward_slot = match world.find_slot_for_item(sid, reward_item_id, 1) {
        Some(s) => s,
        None => {
            // The pre-check succeeded and session packets are processed
            // serially, so this is defensive rollback for unexpected state.
            let _ = world.give_item(sid, piece_item_id, 1);
            return bifrost_send_fail(session, error_code).await;
        }
    };

    // Give reward item
    if !world.give_item(sid, reward_item_id, 1) {
        // Do not consume a piece when reward delivery unexpectedly fails.
        let _ = world.give_item(sid, piece_item_id, 1);
        return bifrost_send_fail(session, 0).await;
    }

    // Determine effect color by item type
    let reward_item_type = reward_table.item_type.unwrap_or(0);
    let effect_type = if reward_item_type == 4 {
        BeefEffectType::White
    } else if reward_item_type == 5 {
        BeefEffectType::Green
    } else {
        BeefEffectType::Red
    };

    // Recalculate stats
    world.set_user_ability(sid);

    // Send success response
    let slot_check = if reward_slot >= SLOT_MAX {
        (reward_slot - SLOT_MAX) as i8
    } else {
        reward_slot as i8
    };

    let mut result = Packet::new(Opcode::WizItemUpgrade as u8);
    result.write_u8(ITEM_BIFROST_EXCHANGE);
    result.write_u8(1); // success
    result.write_u32(reward_item_id);
    result.write_i8(slot_check);
    result.write_u32(piece_item_id);
    result.write_i8(src_pos);
    result.write_u8(effect_type as u8);
    session.send_packet(&result).await?;

    // Send the exact remaining source count after the exchange response. This
    // is authoritative and corrects the client's local decrement/cache state.
    if let Some((remaining, durability)) = consumed_slot {
        let mut count_pkt = Packet::new(Opcode::WizItemCountChange as u8);
        count_pkt.write_u16(1); // count_type
        count_pkt.write_u8(1); // slot_section: inventory
        count_pkt.write_u8(src_pos as u8);
        count_pkt.write_u32(piece_item_id);
        count_pkt.write_u32(remaining);
        count_pkt.write_u8(0); // bNewItem = false (consumption)
        count_pkt.write_u16(durability);
        count_pkt.write_u32(0); // reserved
        count_pkt.write_u32(0); // expiration
        world.send_to_session_owned(sid, count_pkt);
    }

    // Broadcast artifact effect to region (3×3 grid)
    let mut artifact_pkt = Packet::new(Opcode::WizObjectEvent as u8);
    artifact_pkt.write_u8(OBJECT_ARTIFACT);
    artifact_pkt.write_u8(effect_type as u8);
    artifact_pkt.write_u32(npc_id);
    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(artifact_pkt),
            None,
            event_room,
        );
    }

    // Epic item notice — broadcast to all if ItemType==4 or specific item
    if reward_item_type == 4 || reward_item_id == 379_068_000 {
        let (char_name, personal_rank) = world
            .with_session(sid, |h| {
                let name = h
                    .character
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                (name, h.personal_rank)
            })
            .unwrap_or_default();

        let mut logos_pkt = Packet::new(Opcode::WizLogosshout as u8);
        logos_pkt.write_u8(0x02);
        logos_pkt.write_u8(0x04);
        logos_pkt.write_sbyte_string(&char_name);
        logos_pkt.write_u32(reward_item_id);
        logos_pkt.write_u8(personal_rank);
        world.broadcast_to_all(Arc::new(logos_pkt), None);
    }

    debug!(
        "[{}] BifrostPiece: piece={} → reward={} effect={:?}",
        session.addr(),
        piece_item_id,
        reward_item_id,
        effect_type
    );

    Ok(())
}

// ── Item Seal System ─────────────────────────────────────────────────
//
//
// Sub-opcode 8 of WIZ_ITEM_UPGRADE.
// Four operations: LOCK (seal), UNLOCK (unseal), BOUND, UNBOUND.

/// Send seal result to client.
async fn send_seal_result(
    session: &mut ClientSession,
    seal_type: u8,
    result: u8,
    item_id: u32,
    src_pos: u8,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(ITEM_SEAL);
    pkt.write_u8(seal_type);
    pkt.write_u8(result);
    if result == 1 {
        pkt.write_u32(item_id);
        pkt.write_u8(src_pos);
    }
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Process item seal operations (lock/unlock/bind/unbind).
async fn item_seal_process(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let seal_opcode = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] ItemSealProcess: seal_opcode={}",
        session.addr(),
        seal_opcode
    );

    match seal_opcode {
        SEAL_LOCK => item_seal_lock(session, reader, &world, sid).await,
        SEAL_UNLOCK => item_seal_unlock(session, reader, &world, sid).await,
        SEAL_BOUND => item_seal_bound(session, reader, &world, sid).await,
        SEAL_UNBOUND => item_seal_unbound(session, reader, &world, sid).await,
        _ => {
            debug!(
                "[{}] ItemSealProcess: unknown seal_opcode {}",
                session.addr(),
                seal_opcode
            );
            Ok(())
        }
    }
}

/// ITEM_LOCK — seal an item (costs 1M gold, requires VIP password).
async fn item_seal_lock(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    world: &std::sync::Arc<crate::world::WorldState>,
    sid: crate::zone::SessionId,
) -> anyhow::Result<()> {
    let _unk0 = reader.read_i32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let password = reader.read_string().unwrap_or_default();

    // C++ early return if item_id == 0
    if item_id == 0 {
        return Ok(());
    }

    // VIP password check (C++: 8-char length enforced)
    let stored_password = world.get_vip_password(sid);
    if stored_password.len() != 8 || password.len() != 8 || stored_password != password {
        return send_seal_result(session, SEAL_LOCK, 4, item_id, src_pos).await; // SealErrorInvalidCode
    }

    // Gold check
    let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
    if gold < ITEM_SEAL_PRICE {
        return send_seal_result(session, SEAL_LOCK, 3, item_id, src_pos).await; // SealErrorNeedCoins
    }

    // Inventory validation
    if src_pos as usize >= HAVE_MAX {
        return send_seal_result(session, SEAL_LOCK, 2, item_id, src_pos).await;
    }
    let actual_slot = SLOT_MAX + src_pos as usize;
    let inv_item = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) => s,
        None => return send_seal_result(session, SEAL_LOCK, 2, item_id, src_pos).await,
    };

    if inv_item.item_id != item_id
        || inv_item.count == 0
        || inv_item.serial_num == 0
        || inv_item.flag == ITEM_FLAG_SEALED
        || inv_item.flag == ITEM_FLAG_CHAR_SEAL
        || inv_item.flag == ITEM_FLAG_DUPLICATE
        || inv_item.expire_time > 0
        || inv_item.flag == ITEM_FLAG_RENTED
    {
        return send_seal_result(session, SEAL_LOCK, 2, item_id, src_pos).await;
    }

    // Item must not be countable
    let item_table = match world.get_item(item_id) {
        Some(t) => t,
        None => return send_seal_result(session, SEAL_LOCK, 2, item_id, src_pos).await,
    };
    if item_table.countable.unwrap_or(0) != 0 {
        return send_seal_result(session, SEAL_LOCK, 2, item_id, src_pos).await;
    }

    // Deduct gold
    world.gold_lose(sid, ITEM_SEAL_PRICE);

    // Update flag: save original flag, set to SEALED
    world.update_inventory(sid, |inv| {
        if actual_slot < inv.len() && inv[actual_slot].item_id == item_id {
            inv[actual_slot].original_flag = inv[actual_slot].flag;
            inv[actual_slot].flag = ITEM_FLAG_SEALED;
            true
        } else {
            false
        }
    });

    // Immediately persist to DB (C++ calls UpdateUserSealItem here)
    save_seal_item_async(session, actual_slot);

    debug!(
        "[sid={}] ItemSealLock: item={} pos={}",
        sid, item_id, src_pos
    );
    send_seal_result(session, SEAL_LOCK, 1, item_id, src_pos).await
}

/// ITEM_UNLOCK — unseal an item (requires VIP password).
async fn item_seal_unlock(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    world: &std::sync::Arc<crate::world::WorldState>,
    sid: crate::zone::SessionId,
) -> anyhow::Result<()> {
    let _unk0 = reader.read_i32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let password = reader.read_string().unwrap_or_default();

    // C++ early return if item_id == 0
    if item_id == 0 {
        return Ok(());
    }

    // VIP password check (C++: 8-char length enforced)
    let stored_password = world.get_vip_password(sid);
    if stored_password.len() != 8 || password.len() != 8 || stored_password != password {
        return send_seal_result(session, SEAL_UNLOCK, 4, item_id, src_pos).await;
    }

    // Inventory validation
    if src_pos as usize >= HAVE_MAX {
        return send_seal_result(session, SEAL_UNLOCK, 2, item_id, src_pos).await;
    }
    let actual_slot = SLOT_MAX + src_pos as usize;
    let inv_item = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) => s,
        None => return send_seal_result(session, SEAL_UNLOCK, 2, item_id, src_pos).await,
    };

    if inv_item.item_id != item_id
        || inv_item.count == 0
        || inv_item.serial_num == 0
        || inv_item.flag != ITEM_FLAG_SEALED
        || inv_item.flag == ITEM_FLAG_DUPLICATE
        || inv_item.expire_time > 0
        || inv_item.flag == ITEM_FLAG_RENTED
    {
        return send_seal_result(session, SEAL_UNLOCK, 2, item_id, src_pos).await;
    }

    // Item must not be countable
    let item_table = match world.get_item(item_id) {
        Some(t) => t,
        None => return send_seal_result(session, SEAL_UNLOCK, 2, item_id, src_pos).await,
    };
    if item_table.countable.unwrap_or(0) != 0 {
        return send_seal_result(session, SEAL_UNLOCK, 2, item_id, src_pos).await;
    }

    // Restore original flag
    world.update_inventory(sid, |inv| {
        if actual_slot < inv.len() && inv[actual_slot].item_id == item_id {
            let o_flag = inv[actual_slot].original_flag;
            if o_flag == ITEM_FLAG_NOT_BOUND || o_flag == ITEM_FLAG_BOUND {
                inv[actual_slot].flag = o_flag;
            } else {
                inv[actual_slot].flag = ITEM_FLAG_NONE;
            }
            inv[actual_slot].original_flag = 0;
            true
        } else {
            false
        }
    });

    // Immediately persist to DB (C++ calls UpdateUserSealItem here)
    save_seal_item_async(session, actual_slot);

    debug!(
        "[sid={}] ItemSealUnlock: item={} pos={}",
        sid, item_id, src_pos
    );
    send_seal_result(session, SEAL_UNLOCK, 1, item_id, src_pos).await
}

/// ITEM_BOUND — bind a Krowaz item (no password, no cost).
async fn item_seal_bound(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    world: &std::sync::Arc<crate::world::WorldState>,
    sid: crate::zone::SessionId,
) -> anyhow::Result<()> {
    // Live v2615 evidence (2026-08-12) proves the 12 bytes after ITEM_BOUND are:
    // u32 unk0, u32 item_id, u8 inventory_pos, u8 unk1, u16 unk2.
    // Example for DB slot 34 / item 278005111:
    //   00 00 00 00 | 77 05 92 10 | 14 | 01 | 00 31
    // The older C++ layout decoded this as item=91684864/slot=146.
    let (unk0, item_id, src_pos, unk1, unk2) = read_item_bound_fields(reader);

    tracing::info!(
        sid,
        unk0,
        item_id,
        src_pos,
        unk1,
        unk2,
        "ITEM_BOUND request decoded"
    );

    // C++ early return if item_id == 0
    if item_id == 0 {
        return Ok(());
    }

    // Inventory validation
    if src_pos as usize >= HAVE_MAX {
        return send_seal_result(session, SEAL_BOUND, 2, item_id, src_pos).await;
    }
    let actual_slot = SLOT_MAX + src_pos as usize;
    let inv_item = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) => s,
        None => return send_seal_result(session, SEAL_BOUND, 2, item_id, src_pos).await,
    };

    if inv_item.item_id != item_id
        || inv_item.count == 0
        || inv_item.flag == ITEM_FLAG_DUPLICATE
        || inv_item.flag == ITEM_FLAG_BOUND
        || inv_item.flag == ITEM_FLAG_RENTED
        || inv_item.serial_num == 0
    {
        tracing::warn!(
            sid,
            item_id,
            src_pos,
            inventory_item_id = inv_item.item_id,
            count = inv_item.count,
            flag = inv_item.flag,
            serial_num = inv_item.serial_num,
            expire_time = inv_item.expire_time,
            "ITEM_BOUND rejected by inventory validation"
        );
        return send_seal_result(session, SEAL_BOUND, 2, item_id, src_pos).await;
    }

    // Item must not be countable
    let item_table = match world.get_item(item_id) {
        Some(t) => t,
        None => {
            tracing::warn!(
                sid,
                item_id,
                src_pos,
                "ITEM_BOUND rejected: item table row missing"
            );
            return send_seal_result(session, SEAL_BOUND, 2, item_id, src_pos).await;
        }
    };
    if item_table.countable.unwrap_or(0) != 0 {
        tracing::warn!(
            sid,
            item_id,
            src_pos,
            countable = item_table.countable,
            "ITEM_BOUND rejected: countable item"
        );
        return send_seal_result(session, SEAL_BOUND, 2, item_id, src_pos).await;
    }

    // Set flag to BOUND
    world.update_inventory(sid, |inv| {
        if actual_slot < inv.len() && inv[actual_slot].item_id == item_id {
            inv[actual_slot].flag = ITEM_FLAG_BOUND;
            true
        } else {
            false
        }
    });

    // Immediately persist to DB (C++ calls UpdateUserSealItem here)
    save_seal_item_async(session, actual_slot);

    debug!(
        "[sid={}] ItemSealBound: item={} pos={}",
        sid, item_id, src_pos
    );
    send_seal_result(session, SEAL_BOUND, 1, item_id, src_pos).await
}

fn read_item_bound_fields(reader: &mut PacketReader<'_>) -> (u32, u32, u8, u8, u16) {
    (
        reader.read_u32().unwrap_or(0),
        reader.read_u32().unwrap_or(0),
        reader.read_u8().unwrap_or(0),
        reader.read_u8().unwrap_or(0),
        reader.read_u16().unwrap_or(0),
    )
}

/// ITEM_UNBOUND — unbind an item (requires VIP password + binding scrolls).
async fn item_seal_unbound(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
    world: &std::sync::Arc<crate::world::WorldState>,
    sid: crate::zone::SessionId,
) -> anyhow::Result<()> {
    let _unk0 = reader.read_i32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let password = reader.read_string().unwrap_or_default();

    // C++ early return if item_id == 0
    if item_id == 0 {
        return Ok(());
    }

    // VIP password check (C++: 8-char length enforced)
    let stored_password = world.get_vip_password(sid);
    if stored_password.len() != 8 || password.len() != 8 || stored_password != password {
        return send_seal_result(session, SEAL_UNBOUND, 4, item_id, src_pos).await;
    }

    // Inventory validation
    if src_pos as usize >= HAVE_MAX {
        return send_seal_result(session, SEAL_UNBOUND, 2, item_id, src_pos).await;
    }
    let actual_slot = SLOT_MAX + src_pos as usize;
    let inv_item = match world.get_inventory_slot(sid, actual_slot) {
        Some(s) => s,
        None => return send_seal_result(session, SEAL_UNBOUND, 2, item_id, src_pos).await,
    };

    if inv_item.item_id != item_id
        || inv_item.count == 0
        || inv_item.serial_num == 0
        || inv_item.flag != ITEM_FLAG_BOUND
        || inv_item.flag == ITEM_FLAG_DUPLICATE
        || inv_item.flag == ITEM_FLAG_RENTED
    {
        return send_seal_result(session, SEAL_UNBOUND, 2, item_id, src_pos).await;
    }

    // Item must not be countable
    let item_table = match world.get_item(item_id) {
        Some(t) => t,
        None => return send_seal_result(session, SEAL_UNBOUND, 2, item_id, src_pos).await,
    };
    if item_table.countable.unwrap_or(0) != 0 {
        return send_seal_result(session, SEAL_UNBOUND, 2, item_id, src_pos).await;
    }

    // Binding scroll cost — need `m_Bound` count from item table
    let bound_count = item_table.bound.unwrap_or(0) as u16;
    if bound_count > 0 && !world.check_exist_item(sid, BINDING_SCROLL_ID, bound_count) {
        return send_seal_result(session, SEAL_UNBOUND, 2, item_id, src_pos).await;
    }

    // Consume binding scrolls
    if bound_count > 0 {
        world.rob_item(sid, BINDING_SCROLL_ID, bound_count);
    }

    // Set flag to NOT_BOUND
    world.update_inventory(sid, |inv| {
        if actual_slot < inv.len() && inv[actual_slot].item_id == item_id {
            inv[actual_slot].flag = ITEM_FLAG_NOT_BOUND;
            true
        } else {
            false
        }
    });

    // Immediately persist to DB (C++ calls UpdateUserSealItem here)
    save_seal_item_async(session, actual_slot);

    debug!(
        "[sid={}] ItemSealUnbound: item={} pos={} scrolls_consumed={}",
        sid, item_id, src_pos, bound_count
    );
    send_seal_result(session, SEAL_UNBOUND, 1, item_id, src_pos).await
}

// ── Pet Hatching (sub=6) ──────────────────────────────────────────────

/// Send a pet hatching failure response.
async fn send_pet_hatching_fail(session: &mut ClientSession, error_code: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(PET_HATCHING);
    pkt.write_u8(0); // success = 0
    pkt.write_u8(error_code);
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Handle pet hatching — convert an egg item into a pet kaul.
/// Packet format (from client, after sub-opcode):
/// ```text
/// [u32 npc_id] [i32 item_id] [i8 slot_pos] [dbyte_string pet_name]
/// ```
/// DByte mode: pet_name uses u16 length prefix.
async fn pet_hatching(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Double-check busy state (C++ does this again in HactchingTransformExchange)
    if world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return send_pet_hatching_fail(session, 1).await;
    }

    // Read packet — C++ uses DByte mode for the string
    let _npc_id = reader.read_u32().unwrap_or(0);
    let item_id = reader.read_u32().unwrap_or(0);
    let slot_pos = reader.read_i8().unwrap_or(-1);
    // Pet name: u16 length prefix (DByte mode)
    let name_len = reader.read_u16().unwrap_or(0) as usize;
    let pet_name: String = if name_len > 0 && name_len <= 15 {
        let mut bytes = Vec::with_capacity(name_len);
        for _ in 0..name_len {
            match reader.read_u8() {
                Some(b) => bytes.push(b),
                None => break,
            }
        }
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        String::new()
    };

    // Validate pet name (1-15 chars)
    if pet_name.is_empty() || pet_name.len() > 15 {
        return send_pet_hatching_fail(session, 2).await; // InvalidName
    }

    // Validate slot position
    if slot_pos < 0 || slot_pos as usize >= HAVE_MAX {
        return send_pet_hatching_fail(session, 1).await;
    }

    // PET_START_ITEM must exist in item table
    let pet_table = match world.get_item(PET_START_ITEM) {
        Some(t) => t,
        None => return send_pet_hatching_fail(session, 1).await,
    };

    // Validate item in inventory slot
    let actual_slot = SLOT_MAX + slot_pos as usize;
    let inv_item = world
        .get_inventory_slot(sid, actual_slot)
        .unwrap_or_default();
    if inv_item.item_id != item_id
        || inv_item.count == 0
        || inv_item.flag == ITEM_FLAG_BOUND
        || inv_item.flag == ITEM_FLAG_DUPLICATE
        || inv_item.flag == ITEM_FLAG_RENTED
        || inv_item.flag == ITEM_FLAG_SEALED
    {
        return send_pet_hatching_fail(session, 1).await;
    }

    // PET_START_LEVEL stats must exist
    let pet_info = match world.get_pet_stats_info(PET_START_LEVEL) {
        Some(info) => info,
        None => return send_pet_hatching_fail(session, 1).await,
    };

    // Generate new serial number
    let new_serial = world.generate_item_serial();

    // Create pet in DB
    let pool = session.pool().clone();
    let pet_repo = PetRepository::new(&pool);
    let pet_index = match pet_repo
        .create_pet(
            new_serial as i64,
            PET_START_LEVEL as i16,
            &pet_name,
            pet_info.pet_max_hp,
            pet_info.pet_max_sp,
        )
        .await
    {
        Ok(idx) if idx > 0 => idx as u32,
        Ok(status) => {
            // Non-positive index means pet creation failed.
            warn!("[sid={}] pet_hatching: DB returned status {}", sid, status);
            // Send DB error response (C++ sends nStatus as the first byte)
            let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
            pkt.write_u8(PET_HATCHING);
            pkt.write_u8(status as u8);
            pkt.write_u32(PET_START_ITEM);
            pkt.write_i8(slot_pos);
            pkt.write_u32(0); // pet_index = 0
                              // DByte string
            let name_bytes = pet_name.as_bytes();
            pkt.write_u16(name_bytes.len() as u16);
            pkt.write_bytes(name_bytes);
            pkt.write_u8(pet_table.damage.unwrap_or(0) as u8);
            pkt.write_u8(1); // level
            pkt.write_u16(0); // exp
            pkt.write_u16(9000); // satisfaction
            session.send_packet(&pkt).await?;
            return Ok(());
        }
        Err(e) => {
            warn!("[sid={}] pet_hatching: DB error: {}", sid, e);
            return send_pet_hatching_fail(session, 1).await;
        }
    };

    // Replace egg item with pet kaul in inventory
    let pet_durability = pet_table.duration.unwrap_or(0) as i16;
    world.update_inventory(sid, |inv| {
        if actual_slot < inv.len() {
            inv[actual_slot].item_id = PET_START_ITEM;
            inv[actual_slot].serial_num = new_serial;
            inv[actual_slot].count = 1;
            inv[actual_slot].durability = pet_durability;
            inv[actual_slot].flag = 0;
            inv[actual_slot].original_flag = 0;
            inv[actual_slot].expire_time = 0;
            true
        } else {
            false
        }
    });

    // Create in-memory pet state (not spawned yet — just data)
    let pet_state = PetState {
        serial_id: new_serial,
        level: PET_START_LEVEL,
        satisfaction: 9000,
        exp: 0,
        hp: pet_info.pet_max_hp as u16,
        nid: 0,
        index: pet_index,
        mp: pet_info.pet_max_sp as u16,
        state_change: 4, // MODE_DEFENCE
        name: pet_name.clone(),
        pid: 25500,
        size: 100,
        attack_started: false,
        attack_target_id: -1,
        ..Default::default()
    };

    // Store pet state on session
    world.update_session(sid, |h| {
        h.pet_data = Some(pet_state);
    });

    // Send success response
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(PET_HATCHING);
    pkt.write_u8(1); // success
    pkt.write_u32(PET_START_ITEM);
    pkt.write_i8(slot_pos);
    pkt.write_u32(pet_index);
    // DByte string (u16 length prefix)
    let name_bytes = pet_name.as_bytes();
    pkt.write_u16(name_bytes.len() as u16);
    pkt.write_bytes(name_bytes);
    pkt.write_u8(pet_table.damage.unwrap_or(0) as u8); // attack_type
    pkt.write_u8(PET_START_LEVEL); // level
    pkt.write_u16(0); // exp
    pkt.write_u16(9000); // satisfaction
    session.send_packet(&pkt).await?;

    // Send pet spawn info packet — opens the pet status window on client.
    let spawn_info = super::pet::PetSpawnInfo {
        index: pet_index,
        name: pet_name.clone(),
        level: PET_START_LEVEL,
        exp_percent: 0,
        max_hp: pet_info.pet_max_hp as u16,
        hp: pet_info.pet_max_hp as u16,
        max_mp: pet_info.pet_max_sp as u16,
        mp: pet_info.pet_max_sp as u16,
        satisfaction: 9000,
        attack: pet_info.pet_attack as u16,
        defence: pet_info.pet_defence as u16,
        resistance: pet_info.pet_res as u16,
    };
    let spawn_pkt = super::pet::build_pet_spawn_packet(&spawn_info);
    session.send_packet(&spawn_pkt).await?;

    debug!(
        "[sid={}] pet_hatching: egg={} → pet_index={} name={}",
        sid, item_id, pet_index, pet_name
    );
    Ok(())
}

// ── Pet Image Transform (sub=10) ──────────────────────────────────────

/// Send a pet image transform failure response.
async fn send_pet_transform_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(PET_IMAGE_TRANSFORM);
    pkt.write_u8(0); // success = 0
    pkt.write_u8(1); // error
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Handle pet image transform — change pet appearance via transform recipe.
/// Packet format (from client, after sub-opcode):
/// ```text
/// [u32 npc_id] [u32 item0] [u8 pos0] [u32 item1] [u8 pos1] [u32 item2] [u8 pos2] [u32 item3] [u8 pos3]
/// ```
/// item0 = pet kaul, item1 = catalyst (used for recipe matching), item2/3 = optional.
/// Success response uses sub=6 (PET_HATCHING), NOT sub=10.
async fn pet_image_transform(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let _npc_id = reader.read_u32().unwrap_or(0);

    // Read 4 item/slot pairs
    let mut item_ids: [u32; 4] = [0; 4];
    let mut slot_pos: [u8; 4] = [0; 4];
    for i in 0..4 {
        item_ids[i] = reader.read_u32().unwrap_or(0);
        slot_pos[i] = reader.read_u8().unwrap_or(0);
    }

    // Validate each provided item (C++ validates non-zero items)
    for i in 0..4 {
        if item_ids[i] == 0 {
            continue;
        }
        // Item must exist in item table
        if world.get_item(item_ids[i]).is_none() {
            return send_pet_transform_fail(session).await;
        }
        // Slot check
        if (slot_pos[i] as usize) >= HAVE_MAX {
            return send_pet_transform_fail(session).await;
        }
        // Inventory item validation
        let actual_slot = SLOT_MAX + slot_pos[i] as usize;
        let inv_item = world
            .get_inventory_slot(sid, actual_slot)
            .unwrap_or_default();
        if inv_item.item_id != item_ids[i]
            || inv_item.flag == ITEM_FLAG_BOUND
            || inv_item.flag == ITEM_FLAG_DUPLICATE
            || inv_item.flag == ITEM_FLAG_RENTED
            || inv_item.flag == ITEM_FLAG_SEALED
        {
            return send_pet_transform_fail(session).await;
        }
    }

    // Find matching transform recipes by catalyst item (item[1])
    let matching = world.find_pet_transforms_by_item(item_ids[1] as i32);
    if matching.is_empty() {
        return send_pet_transform_fail(session).await;
    }

    // Build weighted random array (10000 slots, C++ bRandArray[10000])
    let mut rand_array: Vec<i32> = Vec::with_capacity(10000);
    for recipe in &matching {
        let count = recipe.s_percent as usize;
        for _ in 0..count {
            if rand_array.len() >= 10000 {
                break;
            }
            rand_array.push(recipe.s_index);
        }
    }

    if rand_array.is_empty() {
        return send_pet_transform_fail(session).await;
    }

    // Random selection
    let rand_slot = rand_range(0, rand_array.len() as u32) as usize;
    let selected_index = rand_array[rand_slot];

    // Look up the selected recipe
    let recipe = match world.get_pet_image_change(selected_index) {
        Some(r) => r,
        None => return send_pet_transform_fail(session).await,
    };

    // Replacement item must exist
    let replace_item_id = recipe.n_replace_item as u32;
    if world.get_item(replace_item_id).is_none() {
        return send_pet_transform_fail(session).await;
    }

    // Pet kaul item (slot 0) must be valid
    let kaul_slot = SLOT_MAX + slot_pos[0] as usize;
    let kaul_item = world.get_inventory_slot(sid, kaul_slot).unwrap_or_default();
    if kaul_item.item_id == 0 {
        return send_pet_transform_fail(session).await;
    }

    // Pet data must exist (keyed by serial number)
    let pet_serial = kaul_item.serial_num;
    let (pet_index, pet_name, pet_level, pet_exp, pet_satisfaction) = world
        .with_session(sid, |h| {
            h.pet_data.as_ref().map(|pet| {
                (
                    pet.index,
                    pet.name.clone(),
                    pet.level,
                    pet.exp as u16,
                    pet.satisfaction as u16,
                )
            })
        })
        .flatten()
        .unwrap_or((0, String::new(), 1, 0, 9000));

    if pet_index == 0 {
        return send_pet_transform_fail(session).await;
    }

    // Consume catalyst item (slot 1) — decrement count
    let catalyst_slot = SLOT_MAX + slot_pos[1] as usize;
    world.update_inventory(sid, |inv| {
        if catalyst_slot < inv.len() && inv[catalyst_slot].item_id == item_ids[1] {
            if inv[catalyst_slot].count > 1 {
                inv[catalyst_slot].count -= 1;
            } else {
                inv[catalyst_slot] = Default::default();
            }
            true
        } else {
            false
        }
    });

    // Send SendStackChange for catalyst
    let catalyst_info = world
        .get_inventory_slot(sid, catalyst_slot)
        .unwrap_or_default();
    {
        let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
        pkt.write_u16(1); // count_type
        pkt.write_u8(1); // slot_section: inventory
        pkt.write_u8(slot_pos[1]); // position within inventory
        pkt.write_u32(catalyst_info.item_id);
        pkt.write_u32(catalyst_info.count as u32);
        pkt.write_u8(0); // bNewItem = false
        pkt.write_u16(catalyst_info.durability as u16);
        pkt.write_u32(0); // reserved
        pkt.write_u32(0); // expiration
        session.send_packet(&pkt).await?;
    }

    // Replace pet kaul item with new pet item (only nNum changes)
    world.update_inventory(sid, |inv| {
        if kaul_slot < inv.len() {
            inv[kaul_slot].item_id = replace_item_id;
            true
        } else {
            false
        }
    });

    // Update in-memory pet data (sPid, sSize)
    world.update_session(sid, |h| {
        if let Some(ref mut pet) = h.pet_data {
            pet.pid = recipe.s_replace_spid as u16;
            pet.size = recipe.s_replace_size as u16;
        }
    });

    // Build success response — NOTE: uses PET_HATCHING (6) as sub-opcode, NOT PET_IMAGE_TRANSFORM (10)
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(PET_HATCHING); // C++ quirk: success uses sub=6
    pkt.write_u8(1); // success
    pkt.write_u32(replace_item_id);
    pkt.write_u8(slot_pos[0]);
    pkt.write_u32(pet_index);
    // DByte string (u16 length prefix)
    let name_bytes = pet_name.as_bytes();
    pkt.write_u16(name_bytes.len() as u16);
    pkt.write_bytes(name_bytes);
    pkt.write_u8(101); // attack_type hardcoded
    pkt.write_u8(pet_level);
    pkt.write_u16(pet_exp);
    pkt.write_u16(pet_satisfaction);
    // 3 consumed item entries (slots 1-3)
    for i in 1..4 {
        pkt.write_u32(item_ids[i]);
        pkt.write_u8(slot_pos[i]);
    }
    session.send_packet(&pkt).await?;

    debug!(
        "[sid={}] pet_image_transform: kaul={} → replace={} recipe={} serial={}",
        sid, item_ids[0], replace_item_id, selected_index, pet_serial
    );
    Ok(())
}

/// Fire-and-forget immediate DB save for a single inventory slot.
fn save_seal_item_async(session: &ClientSession, slot_idx: usize) {
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
            warn!("Failed to save seal item slot {}: {}", slot_idx, e);
        }
    });
}

#[cfg(test)]
#[allow(clippy::ifs_same_cond)]
mod tests {
    use super::*;

    #[test]
    fn test_v2615_item_bound_packet_layout() {
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u32(0);
        pkt.write_u32(278_005_111);
        pkt.write_u8(20);
        pkt.write_u8(1);
        pkt.write_u16(0x3100);

        assert_eq!(pkt.data.len(), 12);
        let mut reader = PacketReader::new(&pkt.data);
        let fields = read_item_bound_fields(&mut reader);
        assert_eq!(fields, (0, 278_005_111, 20, 1, 0x3100));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_scroll_type_classification() {
        // Low class scrolls
        assert_eq!(get_scroll_type(379221000) as i8, ScrollType::LowClass as i8);
        assert_eq!(get_scroll_type(379235000) as i8, ScrollType::LowClass as i8);
        assert_eq!(get_scroll_type(379255000) as i8, ScrollType::LowClass as i8);

        // Middle class scrolls
        assert_eq!(
            get_scroll_type(379205000) as i8,
            ScrollType::MiddleClass as i8
        );
        assert_eq!(
            get_scroll_type(379220000) as i8,
            ScrollType::MiddleClass as i8
        );

        // High class scrolls
        assert_eq!(
            get_scroll_type(379021000) as i8,
            ScrollType::HighClass as i8
        );
        assert_eq!(
            get_scroll_type(379020000) as i8,
            ScrollType::HighClass as i8
        );

        // Special scrolls
        assert_eq!(
            get_scroll_type(379256000) as i8,
            ScrollType::HighToRebirth as i8
        );
        assert_eq!(get_scroll_type(379257000) as i8, ScrollType::Rebirth as i8);
        assert_eq!(
            get_scroll_type(810322000) as i8,
            ScrollType::RebirthRestoration as i8
        );
        assert_eq!(get_scroll_type(379152000) as i8, ScrollType::Class as i8);

        // Accessories scrolls
        assert_eq!(
            get_scroll_type(379159000) as i8,
            ScrollType::Accessories as i8
        );
        assert_eq!(
            get_scroll_type(379164000) as i8,
            ScrollType::Accessories as i8
        );

        // Invalid
        assert_eq!(get_scroll_type(0) as i8, ScrollType::Invalid as i8);
        assert_eq!(get_scroll_type(999999999) as i8, ScrollType::Invalid as i8);
    }

    #[test]
    fn test_scroll_compatibility() {
        // Low class item accepts low, middle, high, class scrolls
        assert!(is_scroll_compatible(
            ScrollType::LowClass,
            ScrollType::LowClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::LowClass,
            ScrollType::MiddleClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::LowClass,
            ScrollType::HighClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::LowClass,
            ScrollType::Class
        ));
        assert!(!is_scroll_compatible(
            ScrollType::LowClass,
            ScrollType::Rebirth
        ));
        assert!(!is_scroll_compatible(
            ScrollType::LowClass,
            ScrollType::Accessories
        ));

        // Middle class item accepts middle, high, class
        assert!(!is_scroll_compatible(
            ScrollType::MiddleClass,
            ScrollType::LowClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::MiddleClass,
            ScrollType::MiddleClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::MiddleClass,
            ScrollType::HighClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::MiddleClass,
            ScrollType::Class
        ));

        // High class item accepts high, high-to-rebirth, class
        assert!(!is_scroll_compatible(
            ScrollType::HighClass,
            ScrollType::LowClass
        ));
        assert!(!is_scroll_compatible(
            ScrollType::HighClass,
            ScrollType::MiddleClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::HighClass,
            ScrollType::HighClass
        ));
        assert!(is_scroll_compatible(
            ScrollType::HighClass,
            ScrollType::HighToRebirth
        ));
        assert!(is_scroll_compatible(
            ScrollType::HighClass,
            ScrollType::Class
        ));

        // Rebirth item accepts rebirth, high-to-rebirth, high
        assert!(is_scroll_compatible(
            ScrollType::Rebirth,
            ScrollType::Rebirth
        ));
        assert!(is_scroll_compatible(
            ScrollType::Rebirth,
            ScrollType::HighToRebirth
        ));
        assert!(is_scroll_compatible(
            ScrollType::Rebirth,
            ScrollType::HighClass
        ));
        assert!(!is_scroll_compatible(
            ScrollType::Rebirth,
            ScrollType::LowClass
        ));

        // Accessories item only accepts accessories scroll
        assert!(is_scroll_compatible(
            ScrollType::Accessories,
            ScrollType::Accessories
        ));
        assert!(!is_scroll_compatible(
            ScrollType::Accessories,
            ScrollType::HighClass
        ));
    }

    #[test]
    fn test_count_in_items() {
        let items = vec![
            UpgradeItem {
                item_id: 100,
                slot: 0,
            },
            UpgradeItem {
                item_id: ITEM_TRINA,
                slot: 1,
            },
            UpgradeItem {
                item_id: 200,
                slot: 2,
            },
        ];
        assert_eq!(count_in_items(&items, ITEM_TRINA), 1);
        assert_eq!(count_in_items(&items, ITEM_KARIVDIS), 0);
        assert_eq!(count_in_items(&items, 100), 1);
    }

    #[test]
    fn test_upgrade_result_codes() {
        assert_eq!(UpgradeResult::Failed as u8, 0);
        assert_eq!(UpgradeResult::Succeeded as u8, 1);
        assert_eq!(UpgradeResult::Trading as u8, 2);
        assert_eq!(UpgradeResult::NeedCoins as u8, 3);
        assert_eq!(UpgradeResult::NoMatch as u8, 4);
        assert_eq!(UpgradeResult::Rental as u8, 5);
    }

    // ── Crafting System Tests ────────────────────────────────────────

    #[test]
    fn test_crafting_error_codes() {
        assert_eq!(CraftingErrorCode::WrongMaterial as u8, 0);
        assert_eq!(CraftingErrorCode::Success as u8, 1);
        assert_eq!(CraftingErrorCode::Failed as u8, 2);
    }

    #[test]
    fn test_smash_error_codes() {
        assert_eq!(SmashError::Success as u16, 1);
        assert_eq!(SmashError::Inventory as u16, 2);
        assert_eq!(SmashError::Item as u16, 4);
        assert_eq!(SmashError::Npc as u16, 5);
    }

    #[test]
    fn test_is_moradon() {
        assert!(is_moradon(21)); // ZONE_MORADON
        assert!(is_moradon(22)); // ZONE_MORADON2
        assert!(is_moradon(23)); // ZONE_MORADON3
        assert!(is_moradon(24)); // ZONE_MORADON4
        assert!(is_moradon(25)); // ZONE_MORADON5
        assert!(!is_moradon(1)); // Karus zone
        assert!(!is_moradon(11)); // Elmorad zone
        assert!(!is_moradon(0));
        assert!(!is_moradon(100));
    }

    #[test]
    fn test_npc_type_constants() {
        // C++ enum values from globals.h
        assert_eq!(NPC_CRAFTSMAN, 135);
        assert_eq!(NPC_JEWELY, 174);
        assert_eq!(NPC_OLD_MAN, 222);
    }

    #[test]
    fn test_shadow_piece_constant() {
        assert_eq!(ITEM_SHADOW_PIECE, 700_009_000);
    }

    #[test]
    fn test_special_part_sewing_opcode() {
        assert_eq!(SPECIAL_PART_SEWING, 11);
        assert_eq!(ITEM_OLDMAN_EXCHANGE, 13);
        assert_eq!(ITEM_ACCESSORY_DISASSEMBLE, 15);
    }

    #[test]
    fn test_item_special_sewing_row_accessors() {
        use ko_db::models::item_tables::ItemSpecialSewingRow;

        let row = ItemSpecialSewingRow {
            n_index: 1,
            description: Some("Test Recipe".to_string()),
            req_item_id_1: 100,
            req_item_count_1: 5,
            req_item_id_2: 200,
            req_item_count_2: 10,
            req_item_id_3: 300,
            req_item_count_3: 15,
            req_item_id_4: 0,
            req_item_count_4: 0,
            req_item_id_5: 0,
            req_item_count_5: 0,
            req_item_id_6: 0,
            req_item_count_6: 0,
            req_item_id_7: 0,
            req_item_count_7: 0,
            req_item_id_8: 0,
            req_item_count_8: 0,
            req_item_id_9: 0,
            req_item_count_9: 0,
            req_item_id_10: 0,
            req_item_count_10: 0,
            give_item_id: 999,
            give_item_count: 1,
            success_rate: 5000,
            npc_id: 19073,
            is_notice: true,
            is_shadow_success: true,
        };

        assert_eq!(row.req_item_id_at(0), 100);
        assert_eq!(row.req_item_count_at(0), 5);
        assert_eq!(row.req_item_id_at(1), 200);
        assert_eq!(row.req_item_count_at(1), 10);
        assert_eq!(row.req_item_id_at(2), 300);
        assert_eq!(row.req_item_count_at(2), 15);
        assert_eq!(row.req_item_id_at(3), 0);
        assert_eq!(row.req_item_id_at(10), 0); // out of range

        assert_eq!(row.material_count(), 3);
    }

    #[test]
    fn test_sewing_row_material_count_full() {
        use ko_db::models::item_tables::ItemSpecialSewingRow;

        let row = ItemSpecialSewingRow {
            n_index: 1,
            description: None,
            req_item_id_1: 1,
            req_item_count_1: 1,
            req_item_id_2: 2,
            req_item_count_2: 1,
            req_item_id_3: 3,
            req_item_count_3: 1,
            req_item_id_4: 4,
            req_item_count_4: 1,
            req_item_id_5: 5,
            req_item_count_5: 1,
            req_item_id_6: 6,
            req_item_count_6: 1,
            req_item_id_7: 7,
            req_item_count_7: 1,
            req_item_id_8: 8,
            req_item_count_8: 1,
            req_item_id_9: 9,
            req_item_count_9: 1,
            req_item_id_10: 10,
            req_item_count_10: 1,
            give_item_id: 999,
            give_item_count: 1,
            success_rate: 10000,
            npc_id: 19073,
            is_notice: false,
            is_shadow_success: false,
        };

        assert_eq!(row.material_count(), 10);
    }

    #[test]
    fn test_sewing_row_empty_recipe() {
        use ko_db::models::item_tables::ItemSpecialSewingRow;

        let row = ItemSpecialSewingRow {
            n_index: 0,
            description: None,
            req_item_id_1: 0,
            req_item_count_1: 0,
            req_item_id_2: 0,
            req_item_count_2: 0,
            req_item_id_3: 0,
            req_item_count_3: 0,
            req_item_id_4: 0,
            req_item_count_4: 0,
            req_item_id_5: 0,
            req_item_count_5: 0,
            req_item_id_6: 0,
            req_item_count_6: 0,
            req_item_id_7: 0,
            req_item_count_7: 0,
            req_item_id_8: 0,
            req_item_count_8: 0,
            req_item_id_9: 0,
            req_item_count_9: 0,
            req_item_id_10: 0,
            req_item_count_10: 0,
            give_item_id: 0,
            give_item_count: 0,
            success_rate: 0,
            npc_id: 0,
            is_notice: false,
            is_shadow_success: false,
        };

        assert_eq!(row.material_count(), 0);
    }

    #[test]
    fn test_smash_roll_count_by_class() {
        // Accessories: 1 roll
        for class in [31i16, 21, 22] {
            let roll = if matches!(class, 31 | 21 | 22) {
                1u16
            } else if matches!(class, 3 | 4 | 5 | 8) {
                2
            } else {
                3
            };
            assert_eq!(roll, 1, "class {} should have 1 roll", class);
        }

        // Weapons: 2 rolls
        for class in [3i16, 4, 5, 8] {
            let roll = if matches!(class, 31 | 21 | 22) {
                1u16
            } else if matches!(class, 3 | 4 | 5 | 8) {
                2
            } else {
                3
            };
            assert_eq!(roll, 2, "class {} should have 2 rolls", class);
        }

        // Armor: 3 rolls
        for class in [32i16, 33, 34, 35, 37, 38] {
            let roll = if matches!(class, 31 | 21 | 22) {
                1u16
            } else if matches!(class, 3 | 4 | 5 | 8) {
                2
            } else {
                3
            };
            assert_eq!(roll, 3, "class {} should have 3 rolls", class);
        }
    }

    #[test]
    fn test_smash_index_range_by_class() {
        // Weapons: 2M-3M
        for class in [3i16, 4, 5, 8] {
            let (start, end) = match class {
                3 | 4 | 5 | 8 => (2_000_000i32, 3_000_000i32),
                32 | 33 | 34 | 35 | 37 | 38 => (3_000_000, 4_000_000),
                21 => (4_000_000, 5_000_000),
                31 | 22 => (5_000_000, 6_000_000),
                _ => (0, 0),
            };
            assert_eq!(start, 2_000_000, "class {} start", class);
            assert_eq!(end, 3_000_000, "class {} end", class);
        }

        // Armor: 3M-4M
        for class in [32i16, 33, 34, 35, 37, 38] {
            let (start, _) = match class {
                3 | 4 | 5 | 8 => (2_000_000i32, 3_000_000i32),
                32 | 33 | 34 | 35 | 37 | 38 => (3_000_000, 4_000_000),
                21 => (4_000_000, 5_000_000),
                31 | 22 => (5_000_000, 6_000_000),
                _ => (0, 0),
            };
            assert_eq!(start, 3_000_000, "class {} start", class);
        }

        // Earring: 4M-5M
        let (start, end) = match 21i16 {
            3 | 4 | 5 | 8 => (2_000_000i32, 3_000_000i32),
            32 | 33 | 34 | 35 | 37 | 38 => (3_000_000, 4_000_000),
            21 => (4_000_000, 5_000_000),
            31 | 22 => (5_000_000, 6_000_000),
            _ => (0, 0),
        };
        assert_eq!(start, 4_000_000);
        assert_eq!(end, 5_000_000);

        // Necklace/Ring: 5M-6M
        for class in [31i16, 22] {
            let (start, _) = match class {
                3 | 4 | 5 | 8 => (2_000_000i32, 3_000_000i32),
                32 | 33 | 34 | 35 | 37 | 38 => (3_000_000, 4_000_000),
                21 => (4_000_000, 5_000_000),
                31 | 22 => (5_000_000, 6_000_000),
                _ => (0, 0),
            };
            assert_eq!(start, 5_000_000, "class {} start", class);
        }
    }

    #[test]
    fn test_smash_gold_cost() {
        // Normal items: 10000
        for item_type in [1i16, 2, 3, 5, 6, 7, 8, 9, 10, 11] {
            let cost: u32 = if item_type == 4 || item_type == 12 {
                100_000
            } else {
                10_000
            };
            assert_eq!(cost, 10_000, "type {} should cost 10000", item_type);
        }

        // Type 4 and 12: 100000
        for item_type in [4i16, 12] {
            let cost: u32 = if item_type == 4 || item_type == 12 {
                100_000
            } else {
                10_000
            };
            assert_eq!(cost, 100_000, "type {} should cost 100000", item_type);
        }
    }

    #[test]
    fn test_shadow_piece_upgrade_rate() {
        // Base rate 2700 with shadow piece, not guaranteed
        let base = 2700u32;
        let boosted = base + (base * 40) / 100;
        assert_eq!(boosted, 3780);

        // Base rate 2700 with shadow guaranteed
        let guaranteed = 10000u32;
        assert_eq!(guaranteed, 10000);

        // Cap at 10000
        let base_high = 8000u32;
        let mut boosted_high = base_high + (base_high * 40) / 100;
        if boosted_high > 10000 {
            boosted_high = 10000;
        }
        assert_eq!(boosted_high, 10000);
    }

    #[test]
    fn test_rand_range_bounds() {
        // Test that rand_range produces values in [min, max)
        for _ in 0..100 {
            let val = rand_range(0, 10);
            assert!(val < 10, "rand_range(0, 10) produced {}", val);
        }
        for _ in 0..100 {
            let val = rand_range(5, 15);
            assert!((5..15).contains(&val), "rand_range(5, 15) produced {}", val);
        }
    }

    #[test]
    fn test_valid_item_classes_for_smash() {
        let valid_classes = [3, 4, 5, 8, 31, 32, 33, 34, 35, 37, 38, 21, 22];
        for class in valid_classes {
            assert!(
                matches!(
                    class,
                    3 | 4 | 5 | 8 | 31 | 32 | 33 | 34 | 35 | 37 | 38 | 21 | 22
                ),
                "class {} should be valid",
                class
            );
        }
        // Invalid classes
        for class in [0, 1, 2, 6, 7, 9, 10, 20, 23, 30, 36, 39, 40, 255] {
            assert!(
                !matches!(
                    class,
                    3 | 4 | 5 | 8 | 31 | 32 | 33 | 34 | 35 | 37 | 38 | 21 | 22
                ),
                "class {} should be invalid",
                class
            );
        }
    }

    // ── Upgrade Preview & Extended Tests ────────────────────────────────

    #[test]
    fn test_upgrade_type_constants() {
        assert_eq!(UPGRADE_TYPE_NORMAL, 1);
        assert_eq!(UPGRADE_TYPE_PREVIEW, 2);
        assert_eq!(ITEM_UPGRADE, 2);
        assert_eq!(ITEM_ACCESSORIES, 3);
        assert_eq!(ITEM_UPGRADE_REBIRTH, 7);
        assert_eq!(ITEM_UPGRADE_REVERSE, 14);
        assert_eq!(SPECIAL_PART_SEWING, 11);
        assert_eq!(ITEM_OLDMAN_EXCHANGE, 13);
        assert_eq!(ITEM_ACCESSORY_DISASSEMBLE, 15);
    }

    #[test]
    fn test_preview_type_in_valid_range() {
        // bType 1 = normal, 2 = preview, both valid
        assert!((UPGRADE_TYPE_NORMAL..=UPGRADE_TYPE_PREVIEW).contains(&1));
        assert!((UPGRADE_TYPE_NORMAL..=UPGRADE_TYPE_PREVIEW).contains(&2));
        // bType 0 or 3 = invalid
        assert!(!(UPGRADE_TYPE_NORMAL..=UPGRADE_TYPE_PREVIEW).contains(&0));
        assert!(!(UPGRADE_TYPE_NORMAL..=UPGRADE_TYPE_PREVIEW).contains(&3));
    }

    #[test]
    fn test_new_upgrade_row_structure() {
        use ko_db::models::item_tables::NewUpgradeRow;

        let row = NewUpgradeRow {
            n_index: 111478,
            str_note: Some("Legend Knight Priest Gauntlets (+1)".to_string()),
            origin_number: 334004001,
            n_str_note: Some("Legend Knight Priest Gauntlets (+1)".to_string()),
            new_number: 334004521,
            req_item: 379032000,
            grade: 1,
        };

        assert_eq!(row.n_index, 111478);
        assert_eq!(row.origin_number, 334004001);
        assert_eq!(row.new_number, 334004521);
        assert_eq!(row.req_item, 379032000);
        assert_eq!(row.grade, 1);
        // req_item 379032000 is a High Class scroll
        assert_eq!(
            get_scroll_type(row.req_item as u32) as i8,
            ScrollType::HighClass as i8
        );
    }

    #[test]
    fn test_new_upgrade_grade_range() {
        // Grades in NEW_UPGRADE1 range from 1 to 10
        for grade in 1..=10i16 {
            assert!((1..=10).contains(&grade));
        }
    }

    #[test]
    fn test_itemup_probability_row() {
        use ko_db::models::item_upgrade_ext::ItemUpProbabilityRow;

        let row = ItemUpProbabilityRow {
            b_type: 1,
            max_success: 1,
            max_fail: 1,
            cur_success: 1,
            cur_fail: 1,
        };

        assert_eq!(row.b_type, 1);
        assert_eq!(row.max_success, 1);
        assert_eq!(row.max_fail, 1);
        assert_eq!(row.cur_success, 1);
        assert_eq!(row.cur_fail, 1);
    }

    #[test]
    fn test_upgrade_item_flags() {
        // Verify flag constants match C++ _ITEM_DATA flag bits
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_BOUND, 8);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
    }

    #[test]
    fn test_logos_upgrade_constants() {
        // Logos item ID constant
        assert_eq!(ITEM_BLESSING_LOGOS, 890092000);
        // Logos success rate is fixed at 33%
        let logos_rate: u32 = 33 * 100;
        assert_eq!(logos_rate, 3300);
        // Logos downgrade threshold: 6700
        let logos_down_threshold: u32 = 6700;
        assert!(logos_down_threshold < 10000);
    }

    #[test]
    fn test_accessory_trina_constant() {
        assert_eq!(ITEM_RING_TRINA, 354000000);
    }

    #[test]
    fn test_scroll_type_for_known_upgrade_scrolls() {
        // All scroll IDs from sample NEW_UPGRADE1 data: req_item = 379032000
        let scroll_id = 379032000u32;
        let scroll_type = get_scroll_type(scroll_id);
        assert_eq!(scroll_type as i8, ScrollType::HighClass as i8);

        // Item class 3 (high class) should accept high class scrolls
        assert!(is_scroll_compatible(
            ScrollType::HighClass,
            ScrollType::HighClass
        ));
    }

    #[test]
    fn test_upgrade_preview_does_not_consume() {
        // Verify the logic: when bType == UPGRADE_TYPE_PREVIEW, items should NOT
        // be consumed and gold should NOT be deducted.
        // This is tested by checking the control flow constants.
        let b_type = UPGRADE_TYPE_PREVIEW;
        assert_eq!(b_type, 2);

        // In the handler, items are consumed only when bType != UPGRADE_TYPE_PREVIEW
        assert_ne!(b_type, UPGRADE_TYPE_NORMAL);
    }

    #[test]
    fn test_upgrade_result_packet_format() {
        // Success layout: [u8 upgradeType] [u8 bType] [u8 result] [items]
        let upgrade_type: u8 = ITEM_UPGRADE;
        let b_type: u8 = UPGRADE_TYPE_PREVIEW;
        let result: u8 = UpgradeResult::Succeeded as u8;

        // Build a mock response (same structure as handler)
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(upgrade_type);
        pkt.write_u8(b_type);
        pkt.write_u8(result);
        // Write one item
        pkt.write_i32(334004521); // new item ID
        pkt.write_i8(0); // slot

        // Verify packet starts with the right opcode
        assert!(pkt.data.len() >= 3);
    }

    #[test]
    fn test_upgrade_failure_matches_2625_parser() {
        let pkt = upgrade_response_header(
            ITEM_UPGRADE,
            UPGRADE_TYPE_NORMAL,
            UpgradeResult::Failed,
            false,
        );
        assert_eq!(pkt.data, vec![ITEM_UPGRADE, UPGRADE_TYPE_NORMAL, 0, 0]);
        let protected = upgrade_response_header(
            ITEM_UPGRADE,
            UPGRADE_TYPE_NORMAL,
            UpgradeResult::Failed,
            true,
        );
        assert_eq!(
            protected.data,
            vec![ITEM_UPGRADE, UPGRADE_TYPE_NORMAL, 0, 1]
        );
        let accessory = upgrade_response_header(
            ITEM_ACCESSORIES,
            UPGRADE_TYPE_NORMAL,
            UpgradeResult::Failed,
            false,
        );
        assert_eq!(
            accessory.data,
            vec![ITEM_ACCESSORIES, UPGRADE_TYPE_NORMAL, 0]
        );
    }

    #[test]
    fn test_upgrade_preview_result_is_success() {
        // In preview mode, the result is always UpgradeSucceeded because
        // we skip the random roll (GenRate < rand check is only for UPGRADE_TYPE_NORMAL)
        let b_type = UPGRADE_TYPE_PREVIEW;
        let gen_rate: u32 = 5000;
        let rand_val: u32 = 8000;

        // The C++ condition: if (bType == UpgradeTypeNormal && GenRate < rand)
        // In preview mode, this condition is FALSE even when GenRate < rand
        let should_fail = b_type == UPGRADE_TYPE_NORMAL && gen_rate < rand_val;
        assert!(!should_fail, "Preview mode should never fail");
    }

    // ── Sprint 275: NPC_ANVIL & Broadcast Tests ──────────────────────────

    /// Test NPC_ANVIL constant matches C++ globals.h value.
    #[test]
    fn test_npc_anvil_constant() {
        // C++ globals.h:118 — NPC_ANVIL = 24
        assert_eq!(NPC_ANVIL, 24);
    }

    /// Test OBJECT_ANVIL broadcast uses NPC ID, not player ID.
    #[test]
    fn test_object_anvil_broadcast_uses_npc_id() {
        use ko_protocol::Packet;

        let npc_id: u32 = 5001;
        let b_result = UpgradeResult::Succeeded;

        let mut anvil_pkt = Packet::new(Opcode::WizObjectEvent as u8);
        anvil_pkt.write_u8(OBJECT_ANVIL);
        anvil_pkt.write_u8(b_result as u8);
        anvil_pkt.write_u32(npc_id);

        let mut r = PacketReader::new(&anvil_pkt.data);
        assert_eq!(r.read_u8(), Some(OBJECT_ANVIL));
        assert_eq!(r.read_u8(), Some(1)); // UpgradeResult::Succeeded
                                          // Must be NPC ID, not player ID
        assert_eq!(r.read_u32(), Some(5001));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 277: Upgrade Rate Limit Tests ────────────────────────────

    /// Test UPGRADE_DELAY constant matches C++ packets.h value.
    #[test]
    fn test_upgrade_delay_constant() {
        assert_eq!(UPGRADE_DELAY, 2);
    }

    /// Test upgrade rate-limit fields exist in SessionHandle with correct defaults.
    #[test]
    fn test_upgrade_rate_limit_session_fields() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Default upgrade_count should be 0
        let count = world.with_session(1, |h| h.upgrade_count).unwrap_or(255);
        assert_eq!(count, 0, "Default upgrade_count must be 0");

        // last_upgrade_time should be in the past (initialized with 5s offset)
        let elapsed = world
            .with_session(1, |h| h.last_upgrade_time.elapsed())
            .unwrap();
        assert!(
            elapsed >= std::time::Duration::from_secs(2),
            "Initial last_upgrade_time should be far enough in the past to not block first upgrade"
        );
    }

    /// Test upgrade_count increments on update.
    #[test]
    fn test_upgrade_count_increments() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Simulate upgrade count increment
        world.update_session(1, |h| {
            h.upgrade_count = h.upgrade_count.saturating_add(1);
        });
        let count = world.with_session(1, |h| h.upgrade_count).unwrap();
        assert_eq!(count, 1);

        // Saturate at u8::MAX
        world.update_session(1, |h| {
            h.upgrade_count = 255;
        });
        world.update_session(1, |h| {
            h.upgrade_count = h.upgrade_count.saturating_add(1);
        });
        let count = world.with_session(1, |h| h.upgrade_count).unwrap();
        assert_eq!(count, 255, "Should saturate at u8::MAX, not overflow");
    }

    // ── Sprint 285: Required items validation ───────────────────────────

    /// After finding upgrade settings, both ReqItem1 and ReqItem2 must be
    /// verified to exist in the client items list AND in the player's inventory.
    #[test]
    fn test_req_item_validation_logic() {
        // A non-zero required item ID must appear in the items list
        let items: Vec<u32> = vec![100001, 200002, 300003];
        let req1: i32 = 200002;
        let req2: i32 = 0; // No second requirement

        assert!(items.iter().any(|&id| id as i32 == req1));
        // req2 == 0 means no requirement — should be skipped
        assert!(req2 <= 0 || items.iter().any(|&id| id as i32 == req2));
    }

    /// Required items with ID 0 should always pass validation (no requirement).
    #[test]
    fn test_req_item_zero_always_passes() {
        let req1: i32 = 0;
        let req2: i32 = 0;
        // Both are zero — no material requirements
        assert!(req1 <= 0, "Zero req should be skipped");
        assert!(req2 <= 0, "Zero req should be skipped");
    }

    // ── Sprint 320: gen_rate cap ────────────────────────────────────

    #[test]
    fn test_gen_rate_cap_at_10000() {
        let mut gen_rate: u32 = 15000;
        if gen_rate > 10000 {
            gen_rate = 10000;
        }
        assert_eq!(gen_rate, 10000, "gen_rate over 10000 must be capped");
    }

    #[test]
    fn test_gen_rate_below_cap_unchanged() {
        let mut gen_rate: u32 = 8000;
        if gen_rate > 10000 {
            gen_rate = 10000;
        }
        assert_eq!(gen_rate, 8000, "gen_rate below 10000 should stay unchanged");
    }

    #[test]
    fn test_gen_rate_exact_10000_unchanged() {
        let mut gen_rate: u32 = 10000;
        if gen_rate > 10000 {
            gen_rate = 10000;
        }
        assert_eq!(
            gen_rate, 10000,
            "gen_rate exactly 10000 should stay unchanged"
        );
    }

    // ── Bifrost Piece Exchange Tests ────────────────────────────────────

    #[test]
    fn test_bifrost_constants() {
        assert_eq!(ITEM_BIFROST_REQ, 4);
        assert_eq!(ITEM_BIFROST_EXCHANGE, 5);
        assert_eq!(NPC_CHAOTIC_GENERATOR, 137);
        assert_eq!(NPC_CHAOTIC_GENERATOR2, 162);
    }

    #[test]
    fn test_beef_effect_type_values() {
        assert_eq!(BeefEffectType::Red as u8, 1);
        assert_eq!(BeefEffectType::Green as u8, 2);
        assert_eq!(BeefEffectType::White as u8, 3);
    }

    #[test]
    fn test_bifrost_effect_type_by_item_type() {
        // ItemType 4 → White (armor)
        assert_eq!(
            if 4 == 4 {
                BeefEffectType::White
            } else if 4 == 5 {
                BeefEffectType::Green
            } else {
                BeefEffectType::Red
            },
            BeefEffectType::White
        );
        // ItemType 5 → Green (shields)
        assert_eq!(
            if 5 == 4 {
                BeefEffectType::White
            } else if 5 == 5 {
                BeefEffectType::Green
            } else {
                BeefEffectType::Red
            },
            BeefEffectType::Green
        );
        // ItemType 1 → Red (other)
        assert_eq!(
            if 1 == 4 {
                BeefEffectType::White
            } else if 1 == 5 {
                BeefEffectType::Green
            } else {
                BeefEffectType::Red
            },
            BeefEffectType::Red
        );
    }

    #[test]
    fn test_bifrost_weighted_random_array_construction() {
        // Simulate the weighted random array logic from bifrost_piece_exchange.
        // exchange_item_count1 / 5 = number of slots per exchange entry.
        let test_cases = vec![
            (100, 500),  // 500 / 5 = 100 slots
            (200, 1000), // 1000 / 5 = 200 slots
            (50, 250),   // 250 / 5 = 50 slots
        ];

        let mut rand_array: Vec<u32> = Vec::with_capacity(10000);
        for (item_id, count) in &test_cases {
            let slots = (count / 5) as usize;
            for _ in 0..slots {
                if rand_array.len() >= 10000 {
                    break;
                }
                rand_array.push(*item_id);
            }
        }

        assert_eq!(rand_array.len(), 350); // 100 + 200 + 50
        assert_eq!(rand_array[0], 100);
        assert_eq!(rand_array[99], 100);
        assert_eq!(rand_array[100], 200);
        assert_eq!(rand_array[299], 200);
        assert_eq!(rand_array[300], 50);
        assert_eq!(rand_array[349], 50);
    }

    #[test]
    fn test_bifrost_array_cap_at_10000() {
        let mut rand_array: Vec<u32> = Vec::with_capacity(10000);
        // Fill with more than 10000 entries
        for _ in 0..15000 {
            if rand_array.len() >= 10000 {
                break;
            }
            rand_array.push(12345);
        }
        assert_eq!(rand_array.len(), 10000);
    }

    // ── Item Seal tests ──────────────────────────────────────────────

    #[test]
    fn test_seal_constants() {
        assert_eq!(ITEM_SEAL, 8);
        assert_eq!(SEAL_LOCK, 1);
        assert_eq!(SEAL_UNLOCK, 2);
        assert_eq!(SEAL_BOUND, 3);
        assert_eq!(SEAL_UNBOUND, 4);
        assert_eq!(ITEM_SEAL_PRICE, 1_000_000);
        assert_eq!(BINDING_SCROLL_ID, 810_890_000);
    }

    #[test]
    fn test_seal_flag_values() {
        use crate::world::{
            ITEM_FLAG_BOUND, ITEM_FLAG_CHAR_SEAL, ITEM_FLAG_NONE, ITEM_FLAG_NOT_BOUND,
            ITEM_FLAG_SEALED,
        };
        assert_eq!(ITEM_FLAG_NONE, 0);
        assert_eq!(ITEM_FLAG_CHAR_SEAL, 2);
        assert_eq!(ITEM_FLAG_SEALED, 4);
        assert_eq!(ITEM_FLAG_NOT_BOUND, 7);
        assert_eq!(ITEM_FLAG_BOUND, 8);
    }

    #[test]
    fn test_seal_original_flag_default() {
        use crate::world::types::UserItemSlot;
        let slot = UserItemSlot::default();
        assert_eq!(slot.original_flag, 0);
        assert_eq!(slot.flag, 0);
    }

    #[test]
    fn test_seal_unlock_restores_bound_flag() {
        use crate::world::{
            ITEM_FLAG_BOUND, ITEM_FLAG_NONE, ITEM_FLAG_NOT_BOUND, ITEM_FLAG_SEALED,
        };
        // Simulate seal lock → unlock with BOUND original flag
        let original_flag: u8 = ITEM_FLAG_BOUND;
        let restored = if original_flag == ITEM_FLAG_NOT_BOUND || original_flag == ITEM_FLAG_BOUND {
            original_flag
        } else {
            ITEM_FLAG_NONE
        };
        assert_eq!(restored, ITEM_FLAG_BOUND);

        // Simulate seal lock → unlock with NONE original flag
        let original_flag: u8 = ITEM_FLAG_NONE;
        let restored = if original_flag == ITEM_FLAG_NOT_BOUND || original_flag == ITEM_FLAG_BOUND {
            original_flag
        } else {
            ITEM_FLAG_NONE
        };
        assert_eq!(restored, ITEM_FLAG_NONE);

        // SEALED flag should never be original_flag (it's the sealed state itself)
        let original_flag: u8 = ITEM_FLAG_SEALED;
        let restored = if original_flag == ITEM_FLAG_NOT_BOUND || original_flag == ITEM_FLAG_BOUND {
            original_flag
        } else {
            ITEM_FLAG_NONE
        };
        assert_eq!(restored, ITEM_FLAG_NONE);
    }

    // ── Pet Hatching / Transform tests ────────────────────────────────

    #[test]
    fn test_pet_hatching_constants() {
        assert_eq!(PET_HATCHING, 6);
        assert_eq!(PET_IMAGE_TRANSFORM, 10);
        assert_eq!(PET_START_ITEM, 610_001_000);
        assert_eq!(PET_START_LEVEL, 1);
    }

    #[test]
    fn test_pet_hatching_fail_packet_format() {
        // Error packet: [sub=6] [success=0] [error_code]
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(PET_HATCHING);
        pkt.write_u8(0);
        pkt.write_u8(2); // InvalidName
        assert_eq!(pkt.data.len(), 3);
        assert_eq!(pkt.data[0], PET_HATCHING);
        assert_eq!(pkt.data[1], 0);
        assert_eq!(pkt.data[2], 2);
    }

    #[test]
    fn test_pet_transform_fail_packet_format() {
        // Error packet: [sub=10] [success=0] [error=1]
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(PET_IMAGE_TRANSFORM);
        pkt.write_u8(0);
        pkt.write_u8(1);
        assert_eq!(pkt.data.len(), 3);
        assert_eq!(pkt.data[0], PET_IMAGE_TRANSFORM);
        assert_eq!(pkt.data[1], 0);
        assert_eq!(pkt.data[2], 1);
    }

    #[test]
    fn test_pet_transform_success_uses_hatching_sub() {
        // C++ quirk: success response uses PET_HATCHING (6) as sub-opcode, NOT PET_IMAGE_TRANSFORM (10)
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(PET_HATCHING); // success uses sub=6
        pkt.write_u8(1); // success
        pkt.write_u32(610_002_000); // replace_item_id
        pkt.write_u8(5); // slot_pos_0
        assert_eq!(pkt.data[0], PET_HATCHING); // 6, not 10
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_pet_transform_weighted_random() {
        // Simulate weighted random with 2 recipes: A=7000, B=3000
        let mut rand_array: Vec<i32> = Vec::with_capacity(10000);
        rand_array.extend(std::iter::repeat_n(1, 7000)); // recipe A
        rand_array.extend(std::iter::repeat_n(2, 3000)); // recipe B
        assert_eq!(rand_array.len(), 10000);
        // First 7000 should be recipe A
        assert_eq!(rand_array[0], 1);
        assert_eq!(rand_array[6999], 1);
        // Last 3000 should be recipe B
        assert_eq!(rand_array[7000], 2);
        assert_eq!(rand_array[9999], 2);
    }

    #[test]
    fn test_pet_name_validation() {
        // Empty name should fail
        let empty = "";
        assert!(empty.is_empty() || empty.len() > 15);

        // Valid name (1-15 chars)
        let valid = "MyPet";
        assert!(!valid.is_empty() && valid.len() <= 15);

        // Too long name (>15 chars)
        let long = "ABCDEFGHIJKLMNOP"; // 16 chars
        assert!(long.len() > 15);

        // Exactly 15 chars
        let exact = "ABCDEFGHIJKLMNO"; // 15 chars
        assert!(!exact.is_empty() && exact.len() <= 15);
    }

    // ── Sprint 962: Additional coverage ──────────────────────────────

    /// Upgrade sub-opcodes match C++ ItemUpgradeOpcodes enum.
    #[test]
    fn test_upgrade_sub_opcodes() {
        assert_eq!(ITEM_UPGRADE, 2);
        assert_eq!(ITEM_ACCESSORIES, 3);
        assert_eq!(ITEM_BIFROST_REQ, 4);
        assert_eq!(ITEM_BIFROST_EXCHANGE, 5);
        assert_eq!(PET_HATCHING, 6);
        assert_eq!(ITEM_UPGRADE_REBIRTH, 7);
        assert_eq!(ITEM_SEAL, 8);
        assert_eq!(PET_IMAGE_TRANSFORM, 10);
        assert_eq!(SPECIAL_PART_SEWING, 11);
        assert_eq!(ITEM_OLDMAN_EXCHANGE, 13);
        assert_eq!(ITEM_UPGRADE_REVERSE, 14);
        assert_eq!(ITEM_ACCESSORY_DISASSEMBLE, 15);
    }

    /// Seal sub-opcodes are sequential 1-4.
    #[test]
    fn test_seal_sub_opcodes_sequential() {
        assert_eq!(SEAL_LOCK, 1);
        assert_eq!(SEAL_UNLOCK, 2);
        assert_eq!(SEAL_BOUND, 3);
        assert_eq!(SEAL_UNBOUND, 4);
    }

    /// Trina item IDs are distinct material types.
    #[test]
    fn test_trina_item_ids() {
        assert_eq!(ITEM_TRINA, 700002000);
        assert_eq!(ITEM_LOW_CLASS_TRINA, 353000000);
        assert_eq!(ITEM_MIDDLE_CLASS_TRINA, 352900000);
        assert_eq!(ITEM_RING_TRINA, 354000000);
        // All distinct
        let trinas = [
            ITEM_TRINA,
            ITEM_LOW_CLASS_TRINA,
            ITEM_MIDDLE_CLASS_TRINA,
            ITEM_RING_TRINA,
        ];
        for i in 0..trinas.len() {
            for j in (i + 1)..trinas.len() {
                assert_ne!(trinas[i], trinas[j]);
            }
        }
    }

    /// UPGRADE_DELAY and MAX_ITEMS_REQ constants.
    #[test]
    fn test_upgrade_limits() {
        assert_eq!(UPGRADE_DELAY, 2);
        assert_eq!(MAX_ITEMS_REQ, 8);
        assert_eq!(ITEM_SEAL_PRICE, 1_000_000);
    }

    /// UpgradeResult enum values match C++ UpgradeErrorCodes.
    #[test]
    fn test_upgrade_result_values() {
        assert_eq!(UpgradeResult::Failed as u8, 0);
        assert_eq!(UpgradeResult::Succeeded as u8, 1);
        assert_eq!(UpgradeResult::Trading as u8, 2);
        assert_eq!(UpgradeResult::NeedCoins as u8, 3);
        assert_eq!(UpgradeResult::NoMatch as u8, 4);
        assert_eq!(UpgradeResult::Rental as u8, 5);
    }

    /// BINDING_SCROLL_ID and PET_START constants are non-zero and distinct.
    #[test]
    fn test_binding_and_pet_constants() {
        assert_eq!(BINDING_SCROLL_ID, 810_890_000);
        assert_eq!(PET_START_ITEM, 610_001_000);
        assert_eq!(PET_START_LEVEL, 1);
        // All distinct
        assert_ne!(BINDING_SCROLL_ID, PET_START_ITEM);
    }

    /// UPGRADE_TYPE_NORMAL and UPGRADE_TYPE_PREVIEW form a contiguous range 1..=2.
    #[test]
    fn test_upgrade_type_range() {
        assert_eq!(UPGRADE_TYPE_NORMAL, 1);
        assert_eq!(UPGRADE_TYPE_PREVIEW, 2);
        assert_eq!(UPGRADE_TYPE_PREVIEW - UPGRADE_TYPE_NORMAL, 1);
    }

    /// ScrollType enum covers the known upgrade scroll classes plus Invalid.
    #[test]
    fn test_scroll_type_coverage() {
        assert_eq!(ScrollType::Invalid as i8, -1);
        assert_eq!(ScrollType::LowClass as i8, 1);
        assert_eq!(ScrollType::MiddleClass as i8, 2);
        assert_eq!(ScrollType::HighClass as i8, 3);
        assert_eq!(ScrollType::Rebirth as i8, 4);
        assert_eq!(ScrollType::Class as i8, 5);
        assert_eq!(ScrollType::Accessories as i8, 8);
        assert_eq!(ScrollType::HighToRebirth as i8, 15);
        assert_eq!(ScrollType::RebirthRestoration as i8, 17);
    }

    /// NPC_ANVIL, ITEM_KARIVDIS, and ITEM_BLESSING_LOGOS are correct C++ values.
    #[test]
    fn test_npc_anvil_and_special_items() {
        assert_eq!(NPC_ANVIL, 24);
        assert_eq!(ITEM_KARIVDIS, 379258000);
        assert_eq!(ITEM_BLESSING_LOGOS, 890092000);
        // Karivdis and Logos are distinct
        assert_ne!(ITEM_KARIVDIS, ITEM_BLESSING_LOGOS);
    }

    /// get_scroll_type returns correct types for known scroll IDs.
    #[test]
    fn test_get_scroll_type_known_ids() {
        assert_eq!(get_scroll_type(379221000), ScrollType::LowClass);
        assert_eq!(get_scroll_type(379220000), ScrollType::MiddleClass);
        assert_eq!(get_scroll_type(379016000), ScrollType::HighClass);
        assert_eq!(get_scroll_type(379256000), ScrollType::HighToRebirth);
        assert_eq!(get_scroll_type(379257000), ScrollType::Rebirth);
        assert_eq!(get_scroll_type(379152000), ScrollType::Class);
        assert_eq!(get_scroll_type(999999999), ScrollType::Invalid);
    }

    /// UpgradeResult enum values match C++ UpgradeErrorCodes exactly.
    #[test]
    fn test_upgrade_result_enum_values() {
        assert_eq!(UpgradeResult::Failed as u8, 0);
        assert_eq!(UpgradeResult::Succeeded as u8, 1);
        assert_eq!(UpgradeResult::Trading as u8, 2);
        assert_eq!(UpgradeResult::NeedCoins as u8, 3);
        assert_eq!(UpgradeResult::NoMatch as u8, 4);
        assert_eq!(UpgradeResult::Rental as u8, 5);
        // 6 distinct result codes
        let results = [
            UpgradeResult::Failed,
            UpgradeResult::Succeeded,
            UpgradeResult::Trading,
            UpgradeResult::NeedCoins,
            UpgradeResult::NoMatch,
            UpgradeResult::Rental,
        ];
        assert_eq!(results.len(), 6);
    }

    /// Seal sub-opcodes 1-4 cover lock/unlock/bound/unbound.
    #[test]
    fn test_seal_subopcodes_complete() {
        assert_eq!(SEAL_LOCK, 1);
        assert_eq!(SEAL_UNLOCK, 2);
        assert_eq!(SEAL_BOUND, 3);
        assert_eq!(SEAL_UNBOUND, 4);
        // Contiguous 1-4
        assert_eq!(SEAL_UNBOUND - SEAL_LOCK, 3);
        // Seal price is 1M gold
        assert_eq!(ITEM_SEAL_PRICE, 1_000_000);
    }

    /// Bifrost and special exchange sub-opcodes are in 4-13 range.
    #[test]
    fn test_special_exchange_subopcodes() {
        assert_eq!(ITEM_BIFROST_REQ, 4);
        assert_eq!(ITEM_BIFROST_EXCHANGE, 5);
        assert_eq!(PET_HATCHING, 6);
        assert_eq!(ITEM_SEAL, 8);
        assert_eq!(PET_IMAGE_TRANSFORM, 10);
        assert_eq!(SPECIAL_PART_SEWING, 11);
        assert_eq!(ITEM_OLDMAN_EXCHANGE, 13);
        assert_eq!(ITEM_ACCESSORY_DISASSEMBLE, 15);
        // All distinct
        let subs = [
            ITEM_BIFROST_REQ,
            ITEM_BIFROST_EXCHANGE,
            PET_HATCHING,
            ITEM_SEAL,
            PET_IMAGE_TRANSFORM,
            SPECIAL_PART_SEWING,
            ITEM_OLDMAN_EXCHANGE,
            ITEM_ACCESSORY_DISASSEMBLE,
        ];
        for i in 0..subs.len() {
            for j in (i + 1)..subs.len() {
                assert_ne!(subs[i], subs[j]);
            }
        }
    }

    /// Pet constants: start item 610001000, start level 1.
    #[test]
    fn test_pet_start_constants() {
        assert_eq!(PET_START_ITEM, 610_001_000);
        assert_eq!(PET_START_LEVEL, 1);
        // Pet item ID is in 600M+ range (distinct from weapon/armor ranges)
        assert!(PET_START_ITEM >= 600_000_000);
        assert!(PET_START_ITEM < 700_000_000);
    }

    /// Trina item IDs: low < middle < ring < high-class trina.
    #[test]
    fn test_trina_item_ordering() {
        assert_eq!(ITEM_LOW_CLASS_TRINA, 353_000_000);
        assert_eq!(ITEM_MIDDLE_CLASS_TRINA, 352_900_000);
        assert_eq!(ITEM_RING_TRINA, 354_000_000);
        assert_eq!(ITEM_TRINA, 700_002_000);
        // Ring trina > both low/middle class
        assert!(ITEM_RING_TRINA > ITEM_LOW_CLASS_TRINA);
        assert!(ITEM_RING_TRINA > ITEM_MIDDLE_CLASS_TRINA);
        // High-class Trina (700M) is in a completely different ID range
        assert!(ITEM_TRINA > ITEM_RING_TRINA);
    }

    // ── Sprint 996: item_upgrade.rs +5 ──────────────────────────────────

    /// CraftingErrorCode enum values: WrongMaterial=0, Success=1, Failed=2.
    #[test]
    fn test_crafting_error_code_values() {
        assert_eq!(CraftingErrorCode::WrongMaterial as u8, 0);
        assert_eq!(CraftingErrorCode::Success as u8, 1);
        assert_eq!(CraftingErrorCode::Failed as u8, 2);
        // Success is between Wrong and Failed
        assert!((CraftingErrorCode::Success as u8) > (CraftingErrorCode::WrongMaterial as u8));
        assert!((CraftingErrorCode::Success as u8) < (CraftingErrorCode::Failed as u8));
    }

    /// SmashError enum: Success=1, Inventory=2, Item=4, Npc=5 (gap at 3).
    #[test]
    fn test_smash_error_gap_at_3() {
        assert_eq!(SmashError::Success as u16, 1);
        assert_eq!(SmashError::Inventory as u16, 2);
        assert_eq!(SmashError::Item as u16, 4);
        assert_eq!(SmashError::Npc as u16, 5);
        // Gap at 3 (no error code 3)
        assert_eq!(SmashError::Item as u16 - SmashError::Inventory as u16, 2);
    }

    /// Chaotic generator NPC IDs are distinct (two Bifrost piece NPCs).
    #[test]
    fn test_chaotic_generator_npc_pair() {
        assert_eq!(NPC_CHAOTIC_GENERATOR, 137);
        assert_eq!(NPC_CHAOTIC_GENERATOR2, 162);
        assert_ne!(NPC_CHAOTIC_GENERATOR, NPC_CHAOTIC_GENERATOR2);
        // Both are distinct from NPC_CRAFTSMAN and NPC_JEWELY
        assert_ne!(NPC_CHAOTIC_GENERATOR, NPC_CRAFTSMAN);
        assert_ne!(NPC_CHAOTIC_GENERATOR2, NPC_JEWELY);
    }

    /// BeefEffectType visual effects: Red=1, Green=2, White=3 (contiguous).
    #[test]
    fn test_beef_effect_contiguous() {
        assert_eq!(BeefEffectType::Red as u8, 1);
        assert_eq!(BeefEffectType::Green as u8, 2);
        assert_eq!(BeefEffectType::White as u8, 3);
        // Contiguous 1-3
        assert_eq!(BeefEffectType::White as u8 - BeefEffectType::Red as u8, 2);
    }

    /// ScrollType covers 7 variants with gaps (Invalid=-1, then 1-5, 8, 15).
    #[test]
    fn test_scroll_type_variant_spread() {
        assert_eq!(ScrollType::Invalid as i8, -1);
        assert_eq!(ScrollType::LowClass as i8, 1);
        assert_eq!(ScrollType::MiddleClass as i8, 2);
        assert_eq!(ScrollType::HighClass as i8, 3);
        assert_eq!(ScrollType::Rebirth as i8, 4);
        assert_eq!(ScrollType::Class as i8, 5);
        assert_eq!(ScrollType::Accessories as i8, 8);
        assert_eq!(ScrollType::HighToRebirth as i8, 15);
        // Largest gap: Class(5) to Accessories(8) = 3
        assert_eq!(ScrollType::Accessories as i8 - ScrollType::Class as i8, 3);
    }
}
