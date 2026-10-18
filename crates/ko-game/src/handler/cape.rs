//! WIZ_CAPE (0x70) handler — cape customization and purchase.
//! Handles cape purchase (normal and ticket/castellan), colour painting,
//! and cape updates for clans and alliances.
//! Packet format (incoming):
//!   opcode(u8) + cape_id(i16) + r(u8) + g(u8) + b(u8)
//! opcode 0 = normal purchase (gold + clan points)
//! opcode 1 = ticket purchase (castellan cape, requires item 914006000)

use std::sync::Arc;

use ko_db::repositories::knights::KnightsRepository;
use ko_db::repositories::knights_cape::KnightsCapeRepository;
use ko_protocol::Packet;
use tracing::debug;

use crate::clan_constants::{CHIEF, CLAN_TYPE_ACCREDITED5};
use crate::session::{ClientSession, SessionState};

/// WIZ_CAPE opcode byte.
const WIZ_CAPE: u8 = 0x70;
/// WIZ_KNIGHTS_PROCESS opcode byte.
const WIZ_KNIGHTS_PROCESS: u8 = 0x3C;

use super::knights::KNIGHTS_UPDATE;

/// King cape IDs — cannot be purchased.
const KING_CAPE_IDS: [i16; 3] = [97, 98, 99];

/// Castellan ticket item ID.
const CASTELLAN_TICKET_ITEM: u32 = 914006000;

/// Paint cost in clan points (non-1098 version).
const PAINT_COST_CLAN_POINTS: u32 = 36000;

use super::{HAVE_MAX, SLOT_MAX};

/// Send a cape error response to the client.
fn send_cape_fail(error_code: i16) -> Packet {
    let mut pkt = Packet::new(WIZ_CAPE);
    pkt.write_i16(error_code);
    pkt
}

/// Check if the player's inventory contains at least one of `item_id`.
fn has_item_in_inventory(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    item_id: u32,
) -> bool {
    world.update_inventory(sid, |inv| {
        for i in SLOT_MAX..(SLOT_MAX + HAVE_MAX) {
            if let Some(slot) = inv.get(i) {
                if slot.item_id == item_id && slot.count > 0 {
                    return true;
                }
            }
        }
        false
    })
}

/// Handle WIZ_CAPE from the client.
/// Full validation flow:
/// 1. Parse opcode (0=normal, 1=ticket), cape_id, RGB
/// 2. State checks (in game, alive, clan leader, not busy)
/// 3. Clan checks (promoted, alliance rules)
/// 4. Cape table lookup and validation (grade, ranking, king capes blocked)
/// 5. Ticket validation (castellan cape type 3, item 914006000)
/// 6. Cost calculation (gold + clan points + paint cost)
/// 7. Apply cape change, deduct costs, save to DB
/// 8. Send success response + KNIGHTS_UPDATE broadcast
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(&pkt.data);

    let opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };
    let cape_id = match reader.read_u16() {
        Some(v) => v as i16,
        None => return Ok(()),
    };
    let r = reader.read_u8().unwrap_or(0);
    let g = reader.read_u8().unwrap_or(0);
    let b = reader.read_u8().unwrap_or(0);

    // Only opcode 0 (normal) and 1 (ticket) are valid
    if opcode != 0 && opcode != 1 {
        session.send_packet(&send_cape_fail(-1)).await?;
        return Ok(());
    }

    let ch = match session.world().get_character_info(session.session_id()) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Solo Cape designs are bought by the character, not the clan
    let is_leader = ch.fame == CHIEF && ch.knights_id != 0;
    if super::solo_cape::try_buy(session, cape_id, r, g, b, is_leader).await? {
        return Ok(());
    }

    // Must be: in game, alive, clan leader, not busy, in a clan
    let sid = session.session_id();
    let world = session.world();
    if ch.res_hp_type == crate::world::USER_DEAD
        || ch.fame != CHIEF
        || ch.knights_id == 0
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        session.send_packet(&send_cape_fail(-1)).await?;
        return Ok(());
    }

    let mut clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => {
            session.send_packet(&send_cape_fail(-2)).await?;
            return Ok(());
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    // Normalize an expired runtime flag before validating a new purchase.
    // Visual packets already check the timestamp; the purchase and bonus paths
    // must make the same decision.
    if clan.castellan_cape && clan.cast_cape_time < now {
        session.world().update_knights(ch.knights_id, |k| {
            k.castellan_cape = false;
        });
        clan.castellan_cape = false;
    }

    // Must be promoted (flag >= 2)
    if clan.flag < 2 {
        session.send_packet(&send_cape_fail(-1)).await?;
        return Ok(());
    }

    // If in an alliance, only main or sub alliance can change capes.
    // Sub alliance can only change colour (cape_id must be < 0).
    if clan.alliance > 0 {
        if let Some(alliance) = session.world().get_alliance(clan.alliance) {
            if clan.id != alliance.main_clan && clan.id != alliance.sub_clan {
                session.send_packet(&send_cape_fail(-1)).await?;
                return Ok(());
            }
            if clan.id == alliance.sub_clan && cape_id >= 0 {
                session.send_packet(&send_cape_fail(-1)).await?;
                return Ok(());
            }
        }
    }

    let mut req_coins: u32 = 0;
    let mut req_clan_points: u32 = 0;
    let mut is_castellan_cape = false;

    // Cape table validation (when selecting a new cape)
    if cape_id >= 0 {
        let cape_def = match session.world().get_knights_cape(cape_id) {
            Some(c) => c,
            None => {
                session.send_packet(&send_cape_fail(-5)).await?;
                return Ok(());
            }
        };

        // King capes cannot be purchased
        if KING_CAPE_IDS.contains(&cape_id) {
            session.send_packet(&send_cape_fail(-5)).await?;
            return Ok(());
        }

        // Type 3 = castellan cape
        if cape_def.b_type == 3 {
            is_castellan_cape = true;
        }

        // Ticket opcode requires cape type 3 and b_ticket == 1
        if opcode == 1 && (cape_def.b_ticket != 1 || cape_def.b_type != 3) {
            session.send_packet(&send_cape_fail(-1)).await?;
            return Ok(());
        }

        // Ticket purchase requires the ticket item
        if opcode == 1
            && !has_item_in_inventory(session.world(), session.session_id(), CASTELLAN_TICKET_ITEM)
        {
            session.send_packet(&send_cape_fail(-10)).await?;
            return Ok(());
        }

        // If clan already has castellan cape, can only change to another castellan cape
        if clan.castellan_cape && cape_def.b_type != 3 {
            session.send_packet(&send_cape_fail(-10)).await?;
            return Ok(());
        }

        // Ticket purchase: clan grade must be <= 3
        if opcode == 1 && clan.grade > 3 {
            session.send_packet(&send_cape_fail(-6)).await?;
            return Ok(());
        }

        // Normal purchase: grade and ranking check
        if opcode == 0
            && ((cape_def.by_grade > 0 && clan.grade > cape_def.by_grade as u8)
                || (clan.flag as i16) < cape_def.by_ranking)
        {
            session.send_packet(&send_cape_fail(-6)).await?;
            return Ok(());
        }

        // Normal purchase: gold check
        if opcode == 0 && cape_def.n_buy_price > 0 && ch.gold < cape_def.n_buy_price as u32 {
            session.send_packet(&send_cape_fail(-7)).await?;
            return Ok(());
        }

        req_coins = cape_def.n_buy_price as u32;
        req_clan_points = cape_def.n_buy_loyalty as u32;
    }

    // Paint (colour) cost
    let applying_paint = r != 0 || g != 0 || b != 0;
    if applying_paint {
        // v2615/reference rule: cape painting is unlocked by clan type, not
        // the computed NP grade. Promoted clans (flag=2) may buy an eligible
        // cape but cannot paint it until Accredited5 (flag=3).
        if clan.flag < CLAN_TYPE_ACCREDITED5 {
            session.send_packet(&send_cape_fail(-1)).await?;
            return Ok(());
        }
        req_clan_points += PAINT_COST_CLAN_POINTS;
    }

    // Final gold check
    if opcode == 0 && ch.gold < req_coins {
        session.send_packet(&send_cape_fail(-7)).await?;
        return Ok(());
    }

    // Final clan points check
    if opcode == 0 && req_clan_points > 0 && clan.clan_point_fund < req_clan_points {
        session.send_packet(&send_cape_fail(-9)).await?;
        return Ok(());
    }

    // ── Consume resources ──────────────────────────────────────────────

    // Remove ticket item if ticket purchase
    if cape_id >= 0
        && opcode == 1
        && !session
            .world()
            .rob_item(session.session_id(), CASTELLAN_TICKET_ITEM, 1)
    {
        return Ok(());
    }

    // Deduct gold
    if opcode == 0 && req_coins > 0 && !session.world().gold_lose(session.session_id(), req_coins) {
        return Ok(());
    }

    // ── Apply cape change ──────────────────────────────────────────────

    if cape_id >= 0 {
        if is_castellan_cape {
            let cape_duration_days: u32 = if opcode == 1 { 14 } else { 15 };
            let expiry = now + 60 * 60 * 24 * cape_duration_days;
            session.world().update_knights(ch.knights_id, |k| {
                k.cast_cape_id = cape_id;
                k.castellan_cape = true;
                k.cast_cape_time = expiry;
            });
        } else {
            session.world().update_knights(ch.knights_id, |k| {
                k.cape = cape_id as u16;
            });
        }
    }

    // Deduct clan points
    // Ticket/castellan requests never spend the clan fund in the reference
    // flow. This matters when a ticket request also contains an RGB colour.
    if opcode == 0 && req_clan_points > 0 {
        let new_fund = clan.clan_point_fund.saturating_sub(req_clan_points);
        session.world().update_knights(ch.knights_id, |k| {
            k.clan_point_fund = new_fund;
        });
        let repo = KnightsRepository::new(session.pool());
        if let Err(e) = repo
            .update_clan_point_fund(ch.knights_id as i16, new_fund.min(i32::MAX as u32) as i32)
            .await
        {
            tracing::warn!("Failed to update clan point fund for cape: {e}");
        }
    }

    // Apply paint colours
    let final_r = if applying_paint { r } else { 0 };
    let final_g = if applying_paint { g } else { 0 };
    let final_b = if applying_paint { b } else { 0 };

    let updated_clan = {
        let is_cast = session
            .world()
            .get_knights(ch.knights_id)
            .map(|k| k.castellan_cape)
            .unwrap_or(false);

        if is_cast {
            session.world().update_knights(ch.knights_id, |k| {
                k.cast_cape_r = final_r;
                k.cast_cape_g = final_g;
                k.cast_cape_b = final_b;
            });
        } else {
            session.world().update_knights(ch.knights_id, |k| {
                k.cape_r = final_r;
                k.cape_g = final_g;
                k.cape_b = final_b;
            });
        }
        session
            .world()
            .get_knights(ch.knights_id)
            .unwrap_or(clan.clone())
    };

    // ── Save to DB ─────────────────────────────────────────────────────
    let cape_repo = KnightsCapeRepository::new(session.pool());
    if updated_clan.castellan_cape {
        if let Err(e) = cape_repo
            .save_castellan_cape(
                ch.knights_id as i16,
                updated_clan.cast_cape_id,
                updated_clan.cast_cape_r as i16,
                updated_clan.cast_cape_g as i16,
                updated_clan.cast_cape_b as i16,
                updated_clan.cast_cape_time.min(i32::MAX as u32) as i32,
            )
            .await
        {
            tracing::warn!(
                "Failed to save castellan cape for clan {}: {e}",
                ch.knights_id
            );
        }
    } else if let Err(e) = cape_repo
        .save_cape(
            ch.knights_id as i16,
            updated_clan.cape as i16,
            updated_clan.cape_r as i16,
            updated_clan.cape_g as i16,
            updated_clan.cape_b as i16,
        )
        .await
    {
        tracing::warn!("Failed to save cape for clan {}: {e}", ch.knights_id);
    }

    // ── Build success response ─────────────────────────────────────────
    let mut result = Packet::new(WIZ_CAPE);
    result.write_u16(1); // success
    result.write_u16(clan.alliance);
    result.write_u16(ch.knights_id);

    // King path: uint16(king_cape_id) + uint32(0) — NO trailing u8(0)
    // Non-king path: cape_data + u8(R) + u8(G) + u8(B) + uint8(0) trailing
    let is_king = session.world().is_king(ch.nation, &ch.name);
    if is_king {
        // KNIGHTS_HUMAN_KING_CAPE = 98, KNIGHTS_KARUS_KING_CAPE = 97
        let king_cape_id: i16 = if ch.nation == 2 { 98 } else { 97 };
        result.write_i16(king_cape_id);
        result.write_u32(0); // king capes have no custom colors — no trailing byte
    } else if updated_clan.castellan_cape {
        result.write_i16(updated_clan.cast_cape_id);
        result.write_u8(updated_clan.cast_cape_r);
        result.write_u8(updated_clan.cast_cape_g);
        result.write_u8(updated_clan.cast_cape_b);
        result.write_u8(0); // C++ Reference: KnightCape.cpp:163
    } else {
        result.write_u16(updated_clan.cape);
        result.write_u8(updated_clan.cape_r);
        result.write_u8(updated_clan.cape_g);
        result.write_u8(updated_clan.cape_b);
        result.write_u8(0); // C++ Reference: KnightCape.cpp:163
    }

    session.send_packet(&result).await?;

    // ── Send KNIGHTS_UPDATE to all clan members ────────────────────────
    let mut update_pkt = Packet::new(WIZ_KNIGHTS_PROCESS);
    update_pkt.write_u8(KNIGHTS_UPDATE);
    update_pkt.write_u16(updated_clan.id);
    update_pkt.write_u8(updated_clan.flag);
    if updated_clan.castellan_cape {
        update_pkt.write_i16(updated_clan.cast_cape_id);
        update_pkt.write_u8(updated_clan.cast_cape_r);
        update_pkt.write_u8(updated_clan.cast_cape_g);
        update_pkt.write_u8(updated_clan.cast_cape_b);
    } else {
        update_pkt.write_u16(updated_clan.cape);
        update_pkt.write_u8(updated_clan.cape_r);
        update_pkt.write_u8(updated_clan.cape_g);
        update_pkt.write_u8(updated_clan.cape_b);
    }
    update_pkt.write_u8(0);
    session
        .world()
        .send_to_knights_members(ch.knights_id, Arc::new(update_pkt), None);

    // Cape bonuses are part of SetUserAbility in both implementations. Apply
    // or remove them immediately for every online clan member instead of
    // waiting for relog/equipment movement.
    for member_sid in session
        .world()
        .get_online_knights_session_ids(ch.knights_id)
    {
        session.world().set_user_ability(member_sid);
    }

    // If in alliance and is alliance leader, update all alliance clans.
    if clan.alliance > 0 && clan.alliance == clan.id {
        if let Some(alliance) = session.world().get_alliance(clan.alliance) {
            if alliance.sub_clan > 0 {
                if let Some(sub_clan) = session.world().get_knights(alliance.sub_clan) {
                    let mut sub_pkt = Packet::new(WIZ_KNIGHTS_PROCESS);
                    sub_pkt.write_u8(KNIGHTS_UPDATE);
                    sub_pkt.write_u16(sub_clan.id);
                    sub_pkt.write_u8(sub_clan.flag);
                    if updated_clan.castellan_cape {
                        sub_pkt.write_i16(updated_clan.cast_cape_id);
                    } else {
                        sub_pkt.write_u16(updated_clan.cape);
                    }
                    sub_pkt.write_u8(sub_clan.cape_r);
                    sub_pkt.write_u8(sub_clan.cape_g);
                    sub_pkt.write_u8(sub_clan.cape_b);
                    sub_pkt.write_u8(0);
                    session.world().send_to_knights_members(
                        alliance.sub_clan,
                        Arc::new(sub_pkt),
                        None,
                    );
                }
            }
            for &merc_id in &[alliance.mercenary_1, alliance.mercenary_2] {
                if merc_id == 0 {
                    continue;
                }
                if let Some(merc_clan) = session.world().get_knights(merc_id) {
                    let mut merc_pkt = Packet::new(WIZ_KNIGHTS_PROCESS);
                    merc_pkt.write_u8(KNIGHTS_UPDATE);
                    merc_pkt.write_u16(merc_clan.id);
                    merc_pkt.write_u8(merc_clan.flag);
                    if updated_clan.castellan_cape {
                        merc_pkt.write_i16(updated_clan.cast_cape_id);
                    } else {
                        merc_pkt.write_u16(updated_clan.cape);
                    }
                    merc_pkt.write_u32(0); // no colors for mercenary
                    session
                        .world()
                        .send_to_knights_members(merc_id, Arc::new(merc_pkt), None);
                }
            }
        }
    }

    debug!(
        "[{}] Cape changed: clan={}, cape={}, castellan={}, rgb=({},{},{})",
        session.addr(),
        ch.knights_id,
        cape_id,
        is_castellan_cape,
        final_r,
        final_g,
        final_b
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_cape_fail_packet() {
        let pkt = send_cape_fail(-1);
        assert_eq!(pkt.opcode, WIZ_CAPE);
        assert_eq!(pkt.data.len(), 2);
        // -1 as i16 little-endian = 0xFF, 0xFF
        assert_eq!(pkt.data[0], 0xFF);
        assert_eq!(pkt.data[1], 0xFF);
    }

    #[test]
    fn test_cape_fail_specific_codes() {
        // -2 = clan not found
        let pkt = send_cape_fail(-2);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], 0xFE);
        assert_eq!(pkt.data[1], 0xFF);

        // -5 = cape not found / king cape
        let pkt = send_cape_fail(-5);
        assert_eq!(pkt.data[0], 0xFB);
        assert_eq!(pkt.data[1], 0xFF);

        // -6 = grade/ranking insufficient
        let pkt = send_cape_fail(-6);
        assert_eq!(pkt.data[0], 0xFA);
        assert_eq!(pkt.data[1], 0xFF);

        // -7 = insufficient gold
        let pkt = send_cape_fail(-7);
        assert_eq!(pkt.data[0], 0xF9);
        assert_eq!(pkt.data[1], 0xFF);

        // -9 = insufficient clan points
        let pkt = send_cape_fail(-9);
        assert_eq!(pkt.data[0], 0xF7);
        assert_eq!(pkt.data[1], 0xFF);

        // -10 = ticket/castellan error
        let pkt = send_cape_fail(-10);
        assert_eq!(pkt.data[0], 0xF6);
        assert_eq!(pkt.data[1], 0xFF);
    }

    #[test]
    fn test_king_cape_ids_blocked() {
        assert!(KING_CAPE_IDS.contains(&97));
        assert!(KING_CAPE_IDS.contains(&98));
        assert!(KING_CAPE_IDS.contains(&99));
        assert!(!KING_CAPE_IDS.contains(&1));
        assert!(!KING_CAPE_IDS.contains(&0));
    }

    #[test]
    fn test_castellan_ticket_item() {
        assert_eq!(CASTELLAN_TICKET_ITEM, 914006000);
    }

    #[test]
    fn test_paint_cost() {
        assert_eq!(PAINT_COST_CLAN_POINTS, 36000);
    }

    #[test]
    fn test_success_packet_structure() {
        // Simulate building a success response
        let mut result = Packet::new(WIZ_CAPE);
        result.write_u16(1); // success marker
        result.write_u16(0); // alliance_id
        result.write_u16(100); // clan_id
        result.write_u16(5); // cape_id
        result.write_u8(255); // R
        result.write_u8(128); // G
        result.write_u8(64); // B
        result.write_u8(0); // trailer

        assert_eq!(result.opcode, WIZ_CAPE);
        // 2 + 2 + 2 + 2 + 1 + 1 + 1 + 1 = 12 bytes
        assert_eq!(result.data.len(), 12);

        // Verify success marker
        assert_eq!(result.data[0], 1);
        assert_eq!(result.data[1], 0);

        // Verify clan_id (100 LE)
        assert_eq!(result.data[4], 100);
        assert_eq!(result.data[5], 0);

        // Verify cape_id (5 LE)
        assert_eq!(result.data[6], 5);
        assert_eq!(result.data[7], 0);

        // Verify RGB
        assert_eq!(result.data[8], 255);
        assert_eq!(result.data[9], 128);
        assert_eq!(result.data[10], 64);

        // Verify trailer
        assert_eq!(result.data[11], 0);
    }

    #[test]
    fn test_castellan_success_packet_structure() {
        // Simulate building a castellan cape success response
        let mut result = Packet::new(WIZ_CAPE);
        result.write_u16(1); // success marker
        result.write_u16(500); // alliance_id
        result.write_u16(100); // clan_id
        result.write_i16(42); // cast_cape_id
        result.write_u8(0); // R
        result.write_u8(0); // G
        result.write_u8(0); // B
        result.write_u8(0); // trailer

        assert_eq!(result.opcode, WIZ_CAPE);
        assert_eq!(result.data.len(), 12);

        // Verify alliance_id (500 LE)
        assert_eq!(result.data[2], 0xF4);
        assert_eq!(result.data[3], 0x01);

        // Verify cast_cape_id (42 LE)
        assert_eq!(result.data[6], 42);
        assert_eq!(result.data[7], 0);
    }

    #[test]
    fn test_knights_update_packet_structure() {
        let mut update_pkt = Packet::new(WIZ_KNIGHTS_PROCESS);
        update_pkt.write_u8(KNIGHTS_UPDATE);
        update_pkt.write_u16(100); // clan_id
        update_pkt.write_u8(3); // flag
        update_pkt.write_u16(10); // cape_id
        update_pkt.write_u8(200); // R
        update_pkt.write_u8(100); // G
        update_pkt.write_u8(50); // B
        update_pkt.write_u8(0); // trailer

        assert_eq!(update_pkt.opcode, WIZ_KNIGHTS_PROCESS);
        // 1 + 2 + 1 + 2 + 1 + 1 + 1 + 1 = 10 bytes
        assert_eq!(update_pkt.data.len(), 10);

        // Verify sub-opcode
        assert_eq!(update_pkt.data[0], KNIGHTS_UPDATE);

        // Verify clan_id (100 LE)
        assert_eq!(update_pkt.data[1], 100);
        assert_eq!(update_pkt.data[2], 0);
    }

    #[test]
    fn test_mercenary_update_packet_no_colors() {
        let mut merc_pkt = Packet::new(WIZ_KNIGHTS_PROCESS);
        merc_pkt.write_u8(KNIGHTS_UPDATE);
        merc_pkt.write_u16(200); // merc clan id
        merc_pkt.write_u8(2); // flag
        merc_pkt.write_u16(10); // cape_id (from main clan)
        merc_pkt.write_u32(0); // no colors for mercenary

        assert_eq!(merc_pkt.opcode, WIZ_KNIGHTS_PROCESS);
        // 1 + 2 + 1 + 2 + 4 = 10 bytes
        assert_eq!(merc_pkt.data.len(), 10);

        // Verify zero colours
        assert_eq!(merc_pkt.data[6], 0);
        assert_eq!(merc_pkt.data[7], 0);
        assert_eq!(merc_pkt.data[8], 0);
        assert_eq!(merc_pkt.data[9], 0);
    }

    #[test]
    fn test_all_error_codes() {
        let codes = [-1, -2, -5, -6, -7, -9, -10];
        for code in &codes {
            let pkt = send_cape_fail(*code);
            assert_eq!(pkt.opcode, WIZ_CAPE);
            assert_eq!(pkt.data.len(), 2);
            let val = i16::from_le_bytes([pkt.data[0], pkt.data[1]]);
            assert_eq!(val, *code);
        }
    }

    // ── Sprint 311: King cape response ─────────────────────────────────

    /// King players get king cape IDs (97=Karus, 98=Elmorad) instead of clan cape.
    #[test]
    fn test_king_cape_ids() {
        // Elmorad king = 98, Karus king = 97
        let nation_elmo: u8 = 2;
        let nation_karus: u8 = 1;
        let elmo_king_cape: i16 = if nation_elmo == 2 { 98 } else { 97 };
        let karus_king_cape: i16 = if nation_karus == 2 { 98 } else { 97 };
        assert_eq!(elmo_king_cape, 98);
        assert_eq!(karus_king_cape, 97);
    }

    #[test]
    fn test_king_cape_packet_format() {
        // King cape: [i16 king_cape_id] [u32(0)] — NO trailing u8(0)
        let mut result = Packet::new(WIZ_CAPE);
        result.write_u16(1); // success
        result.write_u16(0); // alliance
        result.write_u16(100); // clan_id

        // King: write i16 + u32(0) — no trailing byte
        let king_cape_id: i16 = 98;
        result.write_i16(king_cape_id);
        result.write_u32(0);

        // Total: 2 + 2 + 2 + 2 + 4 = 12 bytes (no trailing u8)
        assert_eq!(result.data.len(), 12);

        let mut r = PacketReader::new(&result.data);
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_u16(), Some(0)); // alliance
        assert_eq!(r.read_u16(), Some(100)); // clan
        assert_eq!(r.read_i16(), Some(98)); // king cape
        assert_eq!(r.read_u32(), Some(0)); // no colors — replaces R/G/B+trailer
    }

    #[test]
    fn test_normal_cape_has_trailing_byte() {
        // Normal cape: [u16 cape] [u8 R] [u8 G] [u8 B] [u8(0)]
        let mut result = Packet::new(WIZ_CAPE);
        result.write_u16(1); // success
        result.write_u16(0); // alliance
        result.write_u16(100); // clan_id
        result.write_u16(5); // cape
        result.write_u8(255); // R
        result.write_u8(128); // G
        result.write_u8(0); // B
        result.write_u8(0); // trailing byte

        // Total: 2 + 2 + 2 + 2 + 1 + 1 + 1 + 1 = 12 bytes
        assert_eq!(result.data.len(), 12);

        let mut r = PacketReader::new(&result.data);
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.read_u16(), Some(5)); // cape
        assert_eq!(r.read_u8(), Some(255)); // R
        assert_eq!(r.read_u8(), Some(128)); // G
        assert_eq!(r.read_u8(), Some(0)); // B
        assert_eq!(r.read_u8(), Some(0)); // trailing
    }

    // ── Sprint 956: Additional coverage ──────────────────────────────

    /// King cape IDs are 97-99 and all distinct.
    #[test]
    fn test_king_cape_id_values() {
        assert_eq!(KING_CAPE_IDS, [97, 98, 99]);
        assert_eq!(KING_CAPE_IDS.len(), 3);
        // All within valid i16 cape range
        for &id in &KING_CAPE_IDS {
            assert!(id > 0 && id < 100);
        }
    }

    /// Castellan ticket item ID matches expected value.
    #[test]
    fn test_castellan_ticket_item_id() {
        assert_eq!(CASTELLAN_TICKET_ITEM, 914006000);
        // Item ID is in the 9-prefix range (cash shop / special items)
        assert!(CASTELLAN_TICKET_ITEM >= 900000000);
    }

    /// Paint cost in clan points.
    #[test]
    fn test_paint_cost_clan_points() {
        assert_eq!(PAINT_COST_CLAN_POINTS, 36000);
        // Significant cost — more than a casual amount
        assert!(PAINT_COST_CLAN_POINTS > 10000);
    }

    /// WIZ_CAPE and WIZ_KNIGHTS_PROCESS opcode values.
    #[test]
    fn test_cape_opcode_values() {
        assert_eq!(WIZ_CAPE, 0x70);
        assert_eq!(WIZ_KNIGHTS_PROCESS, 0x3C);
        assert_ne!(WIZ_CAPE, WIZ_KNIGHTS_PROCESS);
    }

    /// send_cape_fail produces a 2-byte payload with the error code.
    #[test]
    fn test_send_cape_fail_positive_code() {
        // Positive error code (edge case)
        let pkt = send_cape_fail(1);
        assert_eq!(pkt.opcode, WIZ_CAPE);
        assert_eq!(pkt.data.len(), 2);
        let val = i16::from_le_bytes([pkt.data[0], pkt.data[1]]);
        assert_eq!(val, 1);

        // Zero error code
        let pkt0 = send_cape_fail(0);
        let val0 = i16::from_le_bytes([pkt0.data[0], pkt0.data[1]]);
        assert_eq!(val0, 0);
    }
}
