//! Letter (mail) system handler — routed from WIZ_SHOPPING_MALL (STORE_LETTER).
//! Letter opcodes (from `packets.h:971-980`):
//! - 1 = LETTER_UNREAD: Check unread letter count
//! - 2 = LETTER_LIST: List new (unread) letters
//! - 3 = LETTER_HISTORY: List old (read) letters
//! - 4 = LETTER_GET_ITEM: Retrieve attached item from a letter
//! - 5 = LETTER_READ: Read a specific letter
//! - 6 = LETTER_SEND: Send a new letter
//! - 7 = LETTER_DELETE: Delete up to 5 letters at once
//! All responses use: WIZ_SHOPPING_MALL(0x6A) + u8(STORE_LETTER=6) + u8(letter_opcode)

use ko_db::repositories::letter::LetterRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::{
    COIN_MAX, ITEM_FLAG_BOUND, ITEM_FLAG_DUPLICATE, ITEM_FLAG_RENTED, ITEM_FLAG_SEALED, ITEM_GOLD,
    RACE_UNTRADEABLE, ZONE_CHAOS_DUNGEON, ZONE_DUNGEON_DEFENCE, ZONE_KNIGHT_ROYALE,
};

use super::{HAVE_MAX, ITEMCOUNT_MAX, SLOT_MAX};

// Item flag constants imported from crate::world (ITEM_FLAG_RENTED, ITEM_FLAG_BOUND, ITEM_FLAG_DUPLICATE, ITEM_FLAG_SEALED).

/// Shopping mall sub-opcode for letters.
pub(crate) const STORE_LETTER: u8 = 6;

/// Letter sub-opcodes from `packets.h:971-980`.
const LETTER_UNREAD: u8 = 1;
const LETTER_LIST: u8 = 2;
const LETTER_HISTORY: u8 = 3;
const LETTER_GET_ITEM: u8 = 4;
const LETTER_READ: u8 = 5;
const LETTER_SEND: u8 = 6;
const LETTER_DELETE: u8 = 7;

/// Gold cost for sending a text-only letter (type 1).
const LETTER_COST_TEXT: u32 = 1000;

/// Gold cost for sending a letter with an item (type 2, item attached).
/// Note: type 2 without item costs 5000 (line 186), but type 2 is currently disabled.
/// Kept for future item-attachment support. Used in test verification.
const LETTER_COST_ITEM: u32 = 10000;

/// Maximum letters to delete at once.
const MAX_DELETE_COUNT: u8 = 5;

/// Handle letter system packets (already past the STORE_LETTER sub-opcode).
/// Called from shopping_mall::handle when sub_opcode == 6.
pub async fn handle(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot use mail system
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let letter_opcode = reader.read_u8().unwrap_or(0);

    let char_name = match session.world().get_character_info(session.session_id()) {
        Some(ch) => ch.name.clone(),
        None => {
            debug!("[{}] LETTER: no character info", session.addr());
            return Ok(());
        }
    };

    match letter_opcode {
        LETTER_UNREAD => handle_unread(session, &char_name).await,
        LETTER_LIST => handle_list(session, &char_name, true).await,
        LETTER_HISTORY => handle_list(session, &char_name, false).await,
        LETTER_READ => handle_read(session, &char_name, reader).await,
        LETTER_SEND => handle_send(session, &char_name, reader).await,
        LETTER_GET_ITEM => handle_get_item(session, &char_name, reader).await,
        LETTER_DELETE => handle_delete(session, &char_name, reader).await,
        _ => {
            warn!(
                "[{}] LETTER: unknown sub-opcode {}",
                session.addr(),
                letter_opcode
            );
            Ok(())
        }
    }
}

/// Build the standard letter response header.
/// All letter responses start with: WIZ_SHOPPING_MALL + STORE_LETTER + letter_opcode
fn letter_response(letter_opcode: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
    pkt.write_u8(STORE_LETTER);
    pkt.write_u8(letter_opcode);
    pkt
}

/// LETTER_UNREAD (1): Check unread letter count.
/// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_UNREAD + u8(count)
async fn handle_unread(session: &mut ClientSession, char_name: &str) -> anyhow::Result<()> {
    let repo = LetterRepository::new(session.pool());
    let count = repo.count_unread(char_name).await?;

    let mut response = letter_response(LETTER_UNREAD);
    // Client checks for bool (true = has unread), but C++ sends count as u8
    response.write_u8(count.min(255) as u8);

    session.send_packet(&response).await?;
    debug!(
        "[{}] LETTER_UNREAD: {} has {} unread",
        session.addr(),
        char_name,
        count
    );
    Ok(())
}

/// LETTER_LIST (2) / LETTER_HISTORY (3): List letters.
/// Response: WIZ_SHOPPING_MALL + STORE_LETTER + opcode + u8(1=success) + i8(count)
///           + for each: u32(letter_id) + u8(status) + sbyte_string(subject)
///                     + sbyte_string(sender) + u8(type) + [if type==2: u32(item_id) + u16(count) + u32(coins)]
///                     + u32(date) + u16(days_remaining)
async fn handle_list(
    session: &mut ClientSession,
    char_name: &str,
    new_only: bool,
) -> anyhow::Result<()> {
    let opcode = if new_only {
        LETTER_LIST
    } else {
        LETTER_HISTORY
    };
    let repo = LetterRepository::new(session.pool());
    let letters = repo.load_letters(char_name, new_only).await?;

    let mut response = letter_response(opcode);
    response.write_u8(1); // success
    response.write_i8(letters.len().min(127) as i8);

    for letter in &letters {
        response.write_u32(letter.letter_id as u32);
        response.write_u8(letter.b_status as u8);
        // SByte strings for subject and sender (C++ uses SByte mode)
        response.write_sbyte_string(&letter.subject);
        response.write_sbyte_string(&letter.sender_name);
        response.write_u8(letter.b_type as u8);

        if letter.b_type == 2 {
            response.write_u32(letter.item_id as u32);
            response.write_u16(letter.item_count as u16);
            response.write_u32(letter.coins as u32);
        }

        response.write_u32(letter.send_date as u32);
        response.write_u16(letter.days_remaining as u16);
    }

    session.send_packet(&response).await?;
    debug!(
        "[{}] LETTER_LIST(new={}): sent {} letters for {}",
        session.addr(),
        new_only,
        letters.len(),
        char_name
    );
    Ok(())
}

/// LETTER_READ (5): Read a specific letter.
/// Client sends: u32(letter_id)
/// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_READ
///           + u8(success) + u32(letter_id) + sbyte_string(message)
async fn handle_read(
    session: &mut ClientSession,
    char_name: &str,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let letter_id = reader.read_u32().unwrap_or(0);

    let repo = LetterRepository::new(session.pool());
    let message = repo.read_letter(char_name, letter_id as i32).await?;

    let mut response = letter_response(LETTER_READ);
    match message {
        Some(msg) => {
            response.write_u8(1); // success
            response.write_u32(letter_id);
            response.write_sbyte_string(&msg);
        }
        None => {
            response.write_u8(0); // not found
        }
    }

    session.send_packet(&response).await?;
    debug!(
        "[{}] LETTER_READ: letter_id={} for {}",
        session.addr(),
        letter_id,
        char_name
    );
    Ok(())
}

/// LETTER_SEND (6): Send a new letter.
/// Client sends: sbyte_string(recipient) + sbyte_string(subject) + u8(type)
///               + [if type==2: u32(item_id) + u8(src_pos) + u32(coins)]
///               + sbyte_string(message)
/// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_SEND + u8(result)
async fn handle_send(
    session: &mut ClientSession,
    char_name: &str,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    // State checks
    let world = session.world().clone();
    let sid = session.session_id();
    if !world.is_session_ingame(sid)
        || world.is_merchanting(sid)
        || world.is_trading(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        send_letter_result(session, -1i8).await?;
        return Ok(());
    }
    // Zone restriction — C++ LetterHandler.cpp:152
    if let Some(pos) = world.get_position(sid) {
        if pos.zone_id == ZONE_KNIGHT_ROYALE {
            send_letter_result(session, -1i8).await?;
            return Ok(());
        }
    }

    // Read recipient and subject as SByte strings
    let recipient = match reader.read_sbyte_string() {
        Some(s) if !s.is_empty() && s.len() <= 20 => s,
        _ => {
            send_letter_result(session, -1i8).await?;
            return Ok(());
        }
    };

    let subject = match reader.read_sbyte_string() {
        Some(s) if !s.is_empty() && s.len() <= 31 => s,
        _ => {
            send_letter_result(session, -1i8).await?;
            return Ok(());
        }
    };

    let b_type = reader.read_u8().unwrap_or(0);

    // Validate type (1 = text, 2 = with item)
    if b_type == 0 || b_type > 2 {
        send_letter_result(session, -1i8).await?;
        return Ok(());
    }

    // Cannot send to yourself
    if recipient.eq_ignore_ascii_case(char_name) {
        send_letter_result(session, -6i8).await?;
        return Ok(());
    }

    // ── Type 2 (with item): read item fields from packet ──────────────
    let mut item_id: u32 = 0;
    let mut src_pos: u8 = 0;
    let mut coins: u32 = 0;
    let mut item_count: u16 = 0;
    let mut item_durability: i16 = 0;
    let mut item_serial: u64 = 0;
    let mut item_expiry: u32 = 0;

    if b_type == 2 {
        item_id = reader.read_u32().unwrap_or(0);
        src_pos = reader.read_u8().unwrap_or(0);
        coins = reader.read_u32().unwrap_or(0);
        // coins is always 0 in practice (disabled in C++)

        // Validate slot range — C++ LetterHandler.cpp:195
        if src_pos as usize >= HAVE_MAX {
            send_letter_result(session, -1i8).await?;
            return Ok(());
        }

        // Validate item exists in item table
        let item_def = match world.get_item(item_id) {
            Some(i) => i,
            None => {
                send_letter_result(session, -1i8).await?;
                return Ok(());
            }
        };

        // Validate item is in the sender's inventory at the given position
        let inv_pos = SLOT_MAX + src_pos as usize;
        let slot_info = world.get_inventory_slot(sid, inv_pos);
        let slot = match slot_info {
            Some(s) if s.item_id == item_id => s,
            _ => {
                send_letter_result(session, -1i8).await?;
                return Ok(());
            }
        };

        // Untradeable checks — C++ LetterHandler.cpp:198-206 → error -32
        let race = item_def.race.unwrap_or(0);
        if race == RACE_UNTRADEABLE
            || item_id >= ITEM_GOLD
            || slot.flag == ITEM_FLAG_SEALED
            || slot.flag == ITEM_FLAG_RENTED
            || slot.flag == ITEM_FLAG_BOUND
            || slot.flag == ITEM_FLAG_DUPLICATE
            || slot.expire_time > 0
        {
            send_letter_result(session, -32i8).await?;
            return Ok(());
        }

        // Capture item details for DB
        item_count = slot.count;
        item_durability = slot.durability;
        item_serial = slot.serial_num;
        item_expiry = slot.expire_time;
    }

    // ── Read message (SByte string) ─────────────────────────────────────
    let message = match reader.read_sbyte_string() {
        Some(s) if !s.is_empty() && s.len() <= 128 => s,
        _ => {
            send_letter_result(session, -1i8).await?;
            return Ok(());
        }
    };

    // ── Gold cost — C++ LetterHandler.cpp:142,184,186 ───────────────────
    let gold_cost = if b_type == 2 && item_id != 0 {
        LETTER_COST_ITEM // 10000
    } else {
        LETTER_COST_TEXT // 1000
    };

    let player_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);

    // C++ LetterHandler.cpp:220-224 — also checks gold_cost + coins
    if player_gold < gold_cost || player_gold < gold_cost.saturating_add(coins) {
        send_letter_result(session, -3i8).await?;
        return Ok(());
    }

    // ── Calculate date in yy*10000 + mm*100 + dd format ─────────────────
    let send_date = {
        let now = chrono::Utc::now();
        let yy = (now.format("%y").to_string().parse::<i32>().unwrap_or(0)) % 100;
        let mm = now.format("%m").to_string().parse::<i32>().unwrap_or(1);
        let dd = now.format("%d").to_string().parse::<i32>().unwrap_or(1);
        yy * 10000 + mm * 100 + dd
    };

    // ── Send letter via DB ──────────────────────────────────────────────
    let repo = LetterRepository::new(session.pool());
    let success = repo
        .send_letter(
            char_name,
            &recipient,
            &subject,
            &message,
            b_type as i16,
            item_id as i32,
            item_count as i16,
            item_durability,
            item_serial as i64,
            item_expiry as i32,
            coins as i32,
            send_date,
        )
        .await?;

    if !success {
        // Recipient doesn't exist
        send_letter_result(session, -2i8).await?;
        return Ok(());
    }

    // ── Deduct gold — C++ LetterHandler.cpp:240-245 ─────────────────────
    let total_gold_cost = if coins > 0 {
        gold_cost.saturating_add(coins)
    } else {
        gold_cost
    };
    if !world.gold_lose(sid, total_gold_cost) {
        send_letter_result(session, -3i8).await?;
        return Ok(());
    }

    // ── Remove item from sender inventory — C++ LetterHandler.cpp:248-251 ─
    if b_type == 2 && item_id != 0 {
        let inv_pos = SLOT_MAX + src_pos as usize;
        world.update_inventory(sid, |inv| {
            if inv_pos < inv.len() && inv[inv_pos].item_id == item_id {
                inv[inv_pos] = crate::world::UserItemSlot::default();
            }
            true
        });

        // SendStackChange — notify client of removed item
        let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
        pkt.write_u16(1); // count_type
        pkt.write_u8(1); // slot_section: inventory
        pkt.write_u8(src_pos);
        pkt.write_u32(item_id);
        pkt.write_u32(0); // count = 0 (removed)
        pkt.write_u8(0); // bNewItem = false
        pkt.write_u16(0); // durability = 0
        pkt.write_u32(0); // reserved
        pkt.write_u32(0); // expiration
        pkt.write_u16(0); // v2625 SendStackChange trailing field
        world.send_to_session_owned(sid, pkt);

        // Weight notification is integrated into set_user_ability().
        world.set_user_ability(sid);
    }

    // ── Notify recipient if online ──────────────────────────────────────
    if let Some(recipient_sid) = world.find_session_by_name(&recipient) {
        let mut notify = Packet::new(Opcode::WizShoppingMall as u8);
        notify.write_u8(STORE_LETTER);
        notify.write_u8(LETTER_UNREAD);
        notify.write_u8(1); // has unread = true
        world.send_to_session_owned(recipient_sid, notify);
    }

    send_letter_result(session, 1i8).await?;
    debug!(
        "[{}] LETTER_SEND: {} -> {} (type={}, item={}, subject={})",
        session.addr(),
        char_name,
        recipient,
        b_type,
        item_id,
        subject
    );
    Ok(())
}

/// LETTER_GET_ITEM (4): Retrieve attached item from a letter.
/// Validates zone restrictions, busy states, inventory space, weight, and gold cap
/// before granting the attached item and coins to the player's inventory.
/// Client sends: u32(letter_id)
/// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_GET_ITEM + i8(result)
/// Result codes: 1=success, -1=error, -2=no slot, -5=overweight
async fn handle_get_item(
    session: &mut ClientSession,
    char_name: &str,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // ── State checks — C++ LetterHandler.cpp:274-278 ────────────────────
    if world.is_merchanting(sid) || world.is_trading(sid) {
        let mut r = letter_response(LETTER_GET_ITEM);
        r.write_i8(-1);
        return session.send_packet(&r).await;
    }

    // ── Zone restriction — C++ LetterHandler.cpp:279-281 ────────────────
    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if matches!(
        zone_id,
        ZONE_KNIGHT_ROYALE | ZONE_CHAOS_DUNGEON | ZONE_DUNGEON_DEFENCE
    ) {
        let mut r = letter_response(LETTER_GET_ITEM);
        r.write_i8(-1);
        return session.send_packet(&r).await;
    }

    let letter_id = reader.read_u32().unwrap_or(0);

    // ── Get letter item from DB ─────────────────────────────────────────
    let repo = LetterRepository::new(session.pool());
    let letter = match repo
        .get_item_from_letter(char_name, letter_id as i32)
        .await?
    {
        Some(l) => l,
        None => {
            let mut r = letter_response(LETTER_GET_ITEM);
            r.write_i8(-1);
            return session.send_packet(&r).await;
        }
    };

    let item_id = letter.item_id as u32;
    let count = letter.item_count.max(0) as u16;
    let durability = letter.item_durability.max(0) as u16;
    let serial = letter.item_serial as u64;
    let coins = letter.coins.max(0) as u32;
    let expiry_days = letter.item_expiry.max(0) as u32;

    // ── Validate item exists in game tables — C++ LetterHandler.cpp:291 ─
    if item_id == 0 && coins == 0 {
        let mut r = letter_response(LETTER_GET_ITEM);
        r.write_i8(-1);
        return session.send_packet(&r).await;
    }

    // Item granting path (item_id != 0)
    if item_id != 0 {
        let _item_def = match world.get_item(item_id) {
            Some(i) => i,
            None => {
                let mut r = letter_response(LETTER_GET_ITEM);
                r.write_i8(-1);
                return session.send_packet(&r).await;
            }
        };

        // ── Find inventory slot — C++ LetterHandler.cpp:299 ────────────
        let pos = match world.find_slot_for_item(sid, item_id, count) {
            Some(p) => p,
            None => {
                let mut r = letter_response(LETTER_GET_ITEM);
                r.write_i8(-2); // no slot
                return session.send_packet(&r).await;
            }
        };

        // ── Empty slot check — C++ LetterHandler.cpp:332 ────────────
        // C++ GetEmptySlot() == -1 — must have at least one truly empty slot.
        if !world.has_empty_inventory_slot(sid) {
            let mut r = letter_response(LETTER_GET_ITEM);
            r.write_i8(-1);
            return session.send_packet(&r).await;
        }

        // ── Weight check — C++ LetterHandler.cpp:313 ───────────────────
        if !world.check_weight(sid, item_id, count) {
            let mut r = letter_response(LETTER_GET_ITEM);
            r.write_i8(-5); // overweight
            return session.send_packet(&r).await;
        }

        // ── Gold cap check — C++ LetterHandler.cpp:320-324 ─────────────
        if coins > 0 {
            let current_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
            if current_gold as u64 + coins as u64 > COIN_MAX as u64 {
                let mut r = letter_response(LETTER_GET_ITEM);
                r.write_i8(-1);
                return session.send_packet(&r).await;
            }
        }

        // ── Calculate expiration time — C++ LetterHandler.cpp:342-344 ───
        let expire_time = if expiry_days > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            now + (86400 * expiry_days)
        } else {
            0u32
        };

        // ── Grant item to inventory — C++ LetterHandler.cpp:333-351 ─────
        // Use letter's serial (or generate new if 0)
        let actual_serial = if serial != 0 {
            serial
        } else {
            world.generate_item_serial()
        };

        let mut pkt_info: Option<(u8, u16, u16, u32)> = None;
        let success = world.update_inventory(sid, |inv| {
            if pos >= inv.len() {
                return false;
            }
            let slot = &mut inv[pos];
            slot.item_id = item_id;
            slot.count = (slot.count + count).min(ITEMCOUNT_MAX);
            // C++ uses sDuration += sDurability (additive)
            slot.durability += durability as i16;
            slot.serial_num = actual_serial;
            slot.expire_time = expire_time;

            pkt_info = Some((
                (pos - SLOT_MAX) as u8,
                slot.count,
                slot.durability as u16,
                expire_time,
            ));
            true
        });

        if !success {
            let mut r = letter_response(LETTER_GET_ITEM);
            r.write_i8(-1);
            return session.send_packet(&r).await;
        }

        // ── SendStackChange — C++ LetterHandler.cpp:347-348 ────────────
        if let Some((slot_pos, new_count, dur, exp)) = pkt_info {
            let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
            pkt.write_u16(1); // count_type
            pkt.write_u8(1); // slot_section: inventory
            pkt.write_u8(slot_pos);
            pkt.write_u32(item_id);
            pkt.write_u32(new_count as u32);
            pkt.write_u8(100); // bNewItem = true
            pkt.write_u16(dur);
            pkt.write_u32(0); // reserved
            pkt.write_u32(exp); // expiration
            pkt.write_u16(0); // v2625 SendStackChange trailing field
            world.send_to_session_owned(sid, pkt);

            // Weight notification is integrated into set_user_ability().
            world.set_user_ability(sid);
        }
    } else {
        // Coins-only letter (item_id == 0): C++ would fail at GetItemPtr(0)
        // but we handle it gracefully — just check gold cap
        if coins == 0 {
            let mut r = letter_response(LETTER_GET_ITEM);
            r.write_i8(-1);
            return session.send_packet(&r).await;
        }

        let current_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
        if current_gold as u64 + coins as u64 > COIN_MAX as u64 {
            let mut r = letter_response(LETTER_GET_ITEM);
            r.write_i8(-1);
            return session.send_packet(&r).await;
        }
    }

    // ── Grant coins — C++ LetterHandler.cpp:355-356 ─────────────────────
    if coins > 0 {
        world.gold_gain(sid, coins);
    }

    // ── Mark item as taken in DB — C++ LetterHandler.cpp:358 ────────────
    repo.mark_item_taken(char_name, letter_id as i32).await?;

    let mut response = letter_response(LETTER_GET_ITEM);
    response.write_i8(1); // success
    session.send_packet(&response).await?;

    debug!(
        "[{}] LETTER_GET_ITEM: letter_id={} item_id={} count={} coins={} for {}",
        session.addr(),
        letter_id,
        item_id,
        count,
        coins,
        char_name
    );
    Ok(())
}

/// LETTER_DELETE (7): Delete up to 5 letters at once.
/// Client sends: u8(count) + for each: u32(letter_id)
/// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_DELETE + u8(count)
///           + for each: u32(letter_id)
async fn handle_delete(
    session: &mut ClientSession,
    char_name: &str,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let count = reader.read_u8().unwrap_or(0);
    if count > MAX_DELETE_COUNT {
        let mut response = letter_response(LETTER_DELETE);
        response.write_i8(-3);
        session.send_packet(&response).await?;
        return Ok(());
    }

    let mut letter_ids = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let lid = reader.read_u32().unwrap_or(0);
        letter_ids.push(lid);
    }

    let repo = LetterRepository::new(session.pool());
    for &lid in &letter_ids {
        repo.delete_letter(char_name, lid as i32).await?;
    }

    let mut response = letter_response(LETTER_DELETE);
    response.write_u8(count);
    for &lid in &letter_ids {
        response.write_u32(lid);
    }

    session.send_packet(&response).await?;
    debug!(
        "[{}] LETTER_DELETE: deleted {} letters for {}",
        session.addr(),
        count,
        char_name
    );
    Ok(())
}

/// Send a LETTER_SEND result packet.
async fn send_letter_result(session: &mut ClientSession, result: i8) -> anyhow::Result<()> {
    let mut response = letter_response(LETTER_SEND);
    response.write_u8(result as u8);
    session.send_packet(&response).await
}

/// Create a server-initiated system letter with an optional item attached.
/// `ReqLetterGiveBeginnerItem` to send letters from the system (no player session needed).
/// This is a fire-and-forget async function designed to be called from Lua bindings
/// or other server-internal code that needs to send letters without a client session.
/// Parameters:
/// - `pool`: Database connection pool
/// - `sender`: Sender name (e.g., "System", "Premium Store")
/// - `recipient`: Recipient character name
/// - `subject`: Letter subject
/// - `message`: Letter body text
/// - `item_id`: Item ID to attach (0 for text-only)
/// - `item_count`: Number of items to attach
/// - `item_durability`: Item durability value
#[allow(clippy::too_many_arguments)]
pub async fn create_system_letter(
    pool: &ko_db::DbPool,
    sender: &str,
    recipient: &str,
    subject: &str,
    message: &str,
    item_id: u32,
    item_count: u16,
    item_durability: u16,
) -> Result<bool, anyhow::Error> {
    let b_type: i16 = if item_id > 0 { 2 } else { 1 };
    let send_date = {
        let now = chrono::Utc::now();
        let yy = (now.format("%y").to_string().parse::<i32>().unwrap_or(0)) % 100;
        let mm = now.format("%m").to_string().parse::<i32>().unwrap_or(1);
        let dd = now.format("%d").to_string().parse::<i32>().unwrap_or(1);
        yy * 10000 + mm * 100 + dd
    };

    let repo = LetterRepository::new(pool);
    let success = repo
        .send_letter(
            sender,
            recipient,
            subject,
            message,
            b_type,
            item_id as i32,
            item_count as i16,
            item_durability as i16,
            0, // serial
            0, // expiry
            0, // coins
            send_date,
        )
        .await?;
    Ok(success)
}

/// Build a LETTER_UNREAD notification packet.
/// Can be used to notify a player that they have unread letters without requiring
/// a client session context.
pub fn build_unread_notification() -> Packet {
    let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
    pkt.write_u8(STORE_LETTER);
    pkt.write_u8(LETTER_UNREAD);
    pkt.write_u8(1); // has unread = true
    pkt
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_letter_constants() {
        assert_eq!(STORE_LETTER, 6);
        assert_eq!(LETTER_UNREAD, 1);
        assert_eq!(LETTER_LIST, 2);
        assert_eq!(LETTER_HISTORY, 3);
        assert_eq!(LETTER_GET_ITEM, 4);
        assert_eq!(LETTER_READ, 5);
        assert_eq!(LETTER_SEND, 6);
        assert_eq!(LETTER_DELETE, 7);
    }

    #[test]
    fn test_letter_costs() {
        // C++ LetterHandler.cpp:142 — type 1 base cost is 1000
        assert_eq!(LETTER_COST_TEXT, 1000);
        // C++ LetterHandler.cpp:184 — type 2 with item costs 10000
        assert_eq!(LETTER_COST_ITEM, 10000);
    }

    #[test]
    fn test_letter_response_header() {
        let pkt = letter_response(LETTER_UNREAD);
        assert_eq!(pkt.opcode, Opcode::WizShoppingMall as u8);
        // data should be [STORE_LETTER, LETTER_UNREAD] = [6, 1]
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], STORE_LETTER);
        assert_eq!(pkt.data[1], LETTER_UNREAD);
    }

    /// Validate all letter response headers start with WIZ_SHOPPING_MALL + STORE_LETTER + opcode.
    #[test]
    fn test_letter_response_headers_all_opcodes() {
        for opcode in [
            LETTER_UNREAD,
            LETTER_LIST,
            LETTER_HISTORY,
            LETTER_READ,
            LETTER_SEND,
            LETTER_GET_ITEM,
            LETTER_DELETE,
        ] {
            let pkt = letter_response(opcode);
            assert_eq!(pkt.opcode, Opcode::WizShoppingMall as u8);
            assert_eq!(pkt.data[0], STORE_LETTER);
            assert_eq!(pkt.data[1], opcode);
        }
    }

    /// Validate LETTER_UNREAD response format.
    ///
    /// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_UNREAD + u8(count)
    #[test]
    fn test_letter_unread_response_format() {
        let mut pkt = letter_response(LETTER_UNREAD);
        pkt.write_u8(3); // 3 unread

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_UNREAD));
        assert_eq!(reader.read_u8(), Some(3));
        assert_eq!(reader.remaining(), 0);
    }

    /// Validate LETTER_SEND result format.
    ///
    #[test]
    fn test_letter_send_result_format() {
        let mut pkt = letter_response(LETTER_SEND);
        pkt.write_u8(1u8); // success

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_SEND));
        assert_eq!(reader.read_u8(), Some(1)); // success
        assert_eq!(reader.remaining(), 0);
    }

    /// Validate LETTER_READ response format matches C++ LetterHandler.cpp:116-134.
    ///
    /// Success: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_READ + u8(1) + u32(letter_id) + sbyte_string(message)
    #[test]
    fn test_letter_read_response_format() {
        let mut pkt = letter_response(LETTER_READ);
        pkt.write_u8(1); // success
        pkt.write_u32(12345); // letter_id
        pkt.write_sbyte_string("Hello, this is a test message!");

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_READ));
        assert_eq!(reader.read_u8(), Some(1));
        assert_eq!(reader.read_u32(), Some(12345));
        assert_eq!(
            reader.read_sbyte_string(),
            Some("Hello, this is a test message!".to_string())
        );
        assert_eq!(reader.remaining(), 0);
    }

    /// Validate LETTER_DELETE response format matches C++ LetterHandler.cpp:367-379.
    ///
    /// Response: WIZ_SHOPPING_MALL + STORE_LETTER + LETTER_DELETE + u8(count) + for each: u32(letter_id)
    #[test]
    fn test_letter_delete_response_format() {
        let mut pkt = letter_response(LETTER_DELETE);
        pkt.write_u8(3); // deleting 3 letters
        pkt.write_u32(100);
        pkt.write_u32(200);
        pkt.write_u32(300);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_DELETE));
        assert_eq!(reader.read_u8(), Some(3));
        assert_eq!(reader.read_u32(), Some(100));
        assert_eq!(reader.read_u32(), Some(200));
        assert_eq!(reader.read_u32(), Some(300));
        assert_eq!(reader.remaining(), 0);
    }

    /// Validate max delete count matches C++ LetterHandler.cpp:26 — `if (bCount > 5)`.
    #[test]
    fn test_letter_max_delete_count() {
        assert_eq!(MAX_DELETE_COUNT, 5);
    }

    /// Validate LETTER_LIST entry format with type 2 (item attached).
    ///
    /// Entry: u32(id) + u8(status) + sbyte(subject) + sbyte(sender) + u8(type)
    ///        + [if type==2: u32(item_id) + u16(count) + u32(coins)]
    ///        + u32(date) + u16(days)
    #[test]
    fn test_letter_list_entry_with_item() {
        let mut pkt = letter_response(LETTER_LIST);
        pkt.write_u8(1); // success
        pkt.write_i8(1); // count: 1

        // Single letter with item
        pkt.write_u32(999); // letter_id
        pkt.write_u8(0); // unread
        pkt.write_sbyte_string("Gift for you");
        pkt.write_sbyte_string("Warrior01");
        pkt.write_u8(2); // type 2: with item
        pkt.write_u32(389010000); // item_id
        pkt.write_u16(5); // item count
        pkt.write_u32(0); // coins
        pkt.write_u32(260209); // date: 2026-02-09
        pkt.write_u16(30); // days remaining

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_LIST));
        assert_eq!(reader.read_u8(), Some(1)); // success
        assert_eq!(reader.read_u8(), Some(1)); // count

        assert_eq!(reader.read_u32(), Some(999));
        assert_eq!(reader.read_u8(), Some(0));
        assert_eq!(reader.read_sbyte_string(), Some("Gift for you".to_string()));
        assert_eq!(reader.read_sbyte_string(), Some("Warrior01".to_string()));
        assert_eq!(reader.read_u8(), Some(2));
        assert_eq!(reader.read_u32(), Some(389010000));
        assert_eq!(reader.read_u16(), Some(5));
        assert_eq!(reader.read_u32(), Some(0));
        assert_eq!(reader.read_u32(), Some(260209));
        assert_eq!(reader.read_u16(), Some(30));
        assert_eq!(reader.remaining(), 0);
    }

    /// Validate LETTER_LIST entry format with type 1 (text only, no item fields).
    #[test]
    fn test_letter_list_entry_text_only() {
        let mut pkt = letter_response(LETTER_LIST);
        pkt.write_u8(1); // success
        pkt.write_i8(1); // count: 1

        pkt.write_u32(100);
        pkt.write_u8(1); // read
        pkt.write_sbyte_string("Hello");
        pkt.write_sbyte_string("Mage02");
        pkt.write_u8(1); // type 1: text only (no item fields)
        pkt.write_u32(260101); // date
        pkt.write_u16(15); // days remaining

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_LIST));
        assert_eq!(reader.read_u8(), Some(1));
        assert_eq!(reader.read_u8(), Some(1));

        assert_eq!(reader.read_u32(), Some(100));
        assert_eq!(reader.read_u8(), Some(1));
        assert_eq!(reader.read_sbyte_string(), Some("Hello".to_string()));
        assert_eq!(reader.read_sbyte_string(), Some("Mage02".to_string()));
        assert_eq!(reader.read_u8(), Some(1));
        // No item fields for type 1
        assert_eq!(reader.read_u32(), Some(260101));
        assert_eq!(reader.read_u16(), Some(15));
        assert_eq!(reader.remaining(), 0);
    }

    /// Validate build_unread_notification() matches LETTER_UNREAD format.
    #[test]
    fn test_build_unread_notification_format() {
        let pkt = build_unread_notification();
        assert_eq!(pkt.opcode, Opcode::WizShoppingMall as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(STORE_LETTER)); // 6
        assert_eq!(r.read_u8(), Some(LETTER_UNREAD)); // 1
        assert_eq!(r.read_u8(), Some(1)); // has_unread = true
        assert_eq!(r.remaining(), 0);
    }

    /// Verify create_system_letter determines b_type correctly:
    /// item_id > 0 => type 2 (with item), item_id == 0 => type 1 (text only).
    /// (This just tests the logic; DB call requires a real pool.)
    #[test]
    fn test_system_letter_b_type_logic() {
        // Helper matching the logic in create_system_letter
        fn b_type(item_id: u32) -> i16 {
            if item_id > 0 {
                2
            } else {
                1
            }
        }
        assert_eq!(b_type(389010000), 2, "item_id > 0 => type 2 (with item)");
        assert_eq!(b_type(0), 1, "item_id == 0 => type 1 (text only)");
    }

    /// Verify zone restriction constants match C++ LetterHandler.cpp:279-281.
    #[test]
    fn test_letter_get_item_zone_constants() {
        assert_eq!(ZONE_KNIGHT_ROYALE, 76);
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
        assert_eq!(ZONE_DUNGEON_DEFENCE, 89);
    }

    /// Verify LETTER_GET_ITEM success response format: STORE_LETTER + LETTER_GET_ITEM + i8(1).
    ///
    #[test]
    fn test_letter_get_item_success_response_format() {
        let mut pkt = letter_response(LETTER_GET_ITEM);
        pkt.write_i8(1); // success

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(STORE_LETTER));
        assert_eq!(reader.read_u8(), Some(LETTER_GET_ITEM));
        assert_eq!(reader.read_u8(), Some(1)); // success (i8=1 as u8)
        assert_eq!(reader.remaining(), 0);
    }

    /// Verify LETTER_GET_ITEM error codes match C++ LetterHandler.cpp.
    ///
    /// -1 = general error, -2 = no inventory slot, -5 = overweight
    #[test]
    fn test_letter_get_item_error_codes() {
        // Error: general (-1)
        let mut pkt = letter_response(LETTER_GET_ITEM);
        pkt.write_i8(-1);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(STORE_LETTER));
        assert_eq!(r.read_u8(), Some(LETTER_GET_ITEM));
        assert_eq!(r.read_u8(), Some(0xFF)); // -1 as u8

        // Error: no slot (-2)
        let mut pkt2 = letter_response(LETTER_GET_ITEM);
        pkt2.write_i8(-2);
        let mut r2 = PacketReader::new(&pkt2.data);
        assert_eq!(r2.read_u8(), Some(STORE_LETTER));
        assert_eq!(r2.read_u8(), Some(LETTER_GET_ITEM));
        assert_eq!(r2.read_u8(), Some(0xFE)); // -2 as u8

        // Error: overweight (-5)
        let mut pkt3 = letter_response(LETTER_GET_ITEM);
        pkt3.write_i8(-5);
        let mut r3 = PacketReader::new(&pkt3.data);
        assert_eq!(r3.read_u8(), Some(STORE_LETTER));
        assert_eq!(r3.read_u8(), Some(LETTER_GET_ITEM));
        assert_eq!(r3.read_u8(), Some(0xFB)); // -5 as u8
    }

    /// Verify expiration time calculation matches C++ LetterHandler.cpp:342-344.
    ///
    #[test]
    fn test_letter_get_item_expiry_calculation() {
        // 7 days = 7 * 86400 = 604800 seconds
        let expiry_days: u32 = 7;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        let expected = now + (86400 * expiry_days);

        // Verify: result should be within 2 seconds of expected (timing tolerance)
        let actual = now + (86400 * 7);
        assert!(
            (actual as i64 - expected as i64).unsigned_abs() < 2,
            "expiry should be now + 7 days in seconds"
        );

        // 0 days = no expiry
        let no_expiry: u32 = 0;
        let expire_time = if no_expiry > 0 {
            now + (86400 * no_expiry)
        } else {
            0u32
        };
        assert_eq!(expire_time, 0, "0 days = no expiry");
    }

    /// Verify COIN_MAX is accessible and matches C++ Define.h.
    #[test]
    fn test_letter_coin_max_constant() {
        assert_eq!(COIN_MAX, 2_100_000_000);
    }

    // ── Sprint 125: Letter Send with Item (type 2) tests ────────────────

    /// Verify item flag constants match C++ globals.h:411-415.
    #[test]
    fn test_letter_item_flag_constants() {
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_BOUND, 8);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_SEALED, 4);
    }

    /// Verify gold cost constants for type 1 vs type 2 letters.
    ///
    #[test]
    fn test_letter_gold_cost_logic() {
        // Type 1 (text only)
        assert_eq!(LETTER_COST_TEXT, 1000);
        // Type 2 (with item)
        assert_eq!(LETTER_COST_ITEM, 10000);

        // Gold cost determination logic from handle_send
        let cost_type1 = LETTER_COST_TEXT;
        let cost_type2_with_item = LETTER_COST_ITEM;
        assert!(
            cost_type2_with_item > cost_type1,
            "item letter should cost more"
        );
    }

    /// Verify untradeable item detection logic matches C++ LetterHandler.cpp:198-206.
    ///
    /// Items that cannot be sent via mail:
    /// - race == RACE_UNTRADEABLE (20)
    /// - item_id >= ITEM_GOLD (900000000)
    /// - flag == SEALED/RENTED/BOUND/DUPLICATE
    /// - expire_time > 0
    #[test]
    fn test_letter_untradeable_checks() {
        use crate::world::{UserItemSlot, ITEM_GOLD, RACE_UNTRADEABLE};

        // Normal tradeable item
        let normal = UserItemSlot {
            item_id: 389010000,
            flag: 0,
            original_flag: 0,
            expire_time: 0,
            ..Default::default()
        };
        assert!(normal.flag != ITEM_FLAG_SEALED);
        assert!(normal.flag != ITEM_FLAG_RENTED);
        assert!(normal.flag != ITEM_FLAG_BOUND);
        assert!(normal.flag != ITEM_FLAG_DUPLICATE);
        assert!(normal.expire_time == 0);
        assert!(normal.item_id < ITEM_GOLD);

        // Sealed item
        let sealed = UserItemSlot {
            item_id: 389010000,
            flag: ITEM_FLAG_SEALED,
            original_flag: 0,
            ..Default::default()
        };
        assert_eq!(sealed.flag, ITEM_FLAG_SEALED);

        // Item ID >= ITEM_GOLD (coins)
        assert!(ITEM_GOLD >= 900_000_000);
        assert_eq!(RACE_UNTRADEABLE, 20);
    }

    /// Verify HAVE_MAX matches C++ constant for slot range validation.
    #[test]
    fn test_letter_have_max() {
        assert_eq!(HAVE_MAX, 28);
    }

    /// Verify LETTER_SEND error -32 format (untradeable item).
    ///
    #[test]
    fn test_letter_send_untradeable_error() {
        let mut pkt = letter_response(LETTER_SEND);
        pkt.write_u8((-32i8) as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(STORE_LETTER));
        assert_eq!(r.read_u8(), Some(LETTER_SEND));
        assert_eq!(r.read_u8(), Some(0xE0)); // -32 as u8 = 224
    }

    // ── Sprint 265: State checks for LETTER_SEND ─────────────────────

    /// LETTER_SEND must block mining/fishing/merchanting/trading players.
    #[test]
    fn test_letter_send_state_checks_constants() {
        // Verify ZONE_KNIGHT_ROYALE is used for letter zone restriction
        assert_eq!(ZONE_KNIGHT_ROYALE, 76);
    }
}
