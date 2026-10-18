//! WIZ_NATION_TRANSFER (0x82) handler — nation/faction transfer.
//! ## Two-Phase Protocol
//! 1. **Phase 1** (sub=1): Client requests transfer options.
//!    Server responds with character list and their new-nation appearance.
//! 2. **Phase 2** (sub=2): Client confirms selection with race/face/hair choices.
//!    Server updates memory, sends confirmation. Client disconnects itself.
//! ## Item Required
//! - `ITEM_NATION_TRANSFER (810096000)` — consumed on success
//! ## Restrictions
//! - All account characters must not be in a clan
//! - Cannot transfer if character is king

#[cfg(test)]
use ko_db::repositories::character::CharacterRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

// ── Response codes ──────────────────────────────────────────────────────────

/// Open transfer dialog / general error response.
#[cfg(test)]
const NATION_OPEN_BOX: u8 = 2;

// ── Error sub-codes (sent inside NATION_OPEN_BOX response) ──────────────────

/// No error (success).
#[cfg(test)]
const ERR_NONE: u8 = 0;

/// War is currently open — cannot transfer.
#[cfg(test)]
const ERR_WAR_OPEN: u8 = 1;

/// Character is in a clan.
#[cfg(test)]
const ERR_IN_CLAN: u8 = 2;

/// Character is king.
#[cfg(test)]
const ERR_IS_KING: u8 = 3;

/// Player is transformed — cannot transfer.
#[cfg(test)]
const ERR_TRANSFORMED: u8 = 6;

/// No transfer scroll item.
#[cfg(test)]
const ERR_NO_ITEM: u8 = 7;

// ── Item ────────────────────────────────────────────────────────────────────

/// Nation Transfer scroll item ID.
#[cfg(test)]
const ITEM_NATION_TRANSFER: u32 = 810096000;

#[cfg(test)]
use crate::race_constants::{
    BABARIAN, ELMORAD_MAN, ELMORAD_WOMAN, KARUS_BIG, KARUS_MIDDLE, KARUS_SMALL, KARUS_WOMAN,
    KURIAN, PORUTU,
};

/// Handle WIZ_NATION_TRANSFER from the client.
/// **v2525 CONFLICT**: Client opcode 0x82 = WizTitle (title/name-tag system),
/// NOT NationTransfer. The v2525 client's handler at `0x7C9AB0` dispatches
/// sub-opcodes 1-3 as title categories (10 actions each, string resources).
/// Sending NationTransfer S2C packets on 0x82 causes the client to show
/// unrelated title notification strings. The feature is non-functional.
/// We accept C2S but respond with a WIZ_CHAT notice instead of 0x82 packets.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot initiate nation transfer
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    // v2525: 0x82 = WizTitle on client side. NationTransfer UI is unavailable.
    // Send WIZ_CHAT fallback message instead of broken 0x82 S2C packets.
    debug!(
        "[{}] WIZ_NATION_TRANSFER sub={} — blocked (v2525 0x82=WizTitle conflict)",
        session.addr(),
        sub_opcode,
    );

    let mut chat = Packet::new(Opcode::WizChat as u8);
    chat.write_u8(7); // PUBLIC_CHAT (visible in chat window)
    chat.write_u8(20); // system_msg type
    chat.write_string("Nation Transfer is not available in this client version.");
    session.send_packet(&chat).await?;

    Ok(())
}

/// Phase 1: Client requests nation transfer options.
/// Validates eligibility and responds with character list showing
/// what each character would look like after transfer.
#[cfg(test)]
#[allow(dead_code)]
async fn handle_request(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Check: has nation transfer scroll
    if !has_item(&world, sid, ITEM_NATION_TRANSFER) {
        send_error(session, ERR_NO_ITEM).await?;
        return Ok(());
    }

    // Check: not transformed
    if world.is_transformed(sid) {
        send_error(session, ERR_TRANSFORMED).await?;
        return Ok(());
    }

    // Check: war not open
    if world.is_war_open() {
        send_error(session, ERR_WAR_OPEN).await?;
        return Ok(());
    }

    // Check: ALL account characters must not be in a clan and none can be king.
    let account_err = check_all_account_chars(session, &world, ch.nation).await?;
    if account_err != ERR_NONE {
        send_error(session, account_err).await?;
        return Ok(());
    }

    // Compute what this character would look like after transfer
    let new_nation = if ch.nation == 1 { 2u8 } else { 1u8 };
    let new_race = convert_race(ch.race, ch.class, ch.nation);
    let new_class = if ch.nation == 1 {
        ch.class + 100 // Karus→Elmorad: class +100
    } else {
        ch.class.saturating_sub(100) // Elmorad→Karus: class -100
    };

    if new_race == 0 {
        send_error(session, ERR_NONE).await?;
        return Ok(());
    }

    // Build response: character list with converted appearance
    //      [u16 charnum] [string name] [u8 new_race] [u8 new_nation] [u16 new_class] [u8 face] [u32 hair]
    let mut result = Packet::new(Opcode::WizNationTransfer as u8);
    result.write_u8(NATION_OPEN_BOX);
    result.write_u8(ERR_NONE);
    result.write_u8(1); // 1 character
    result.write_u16(sid);
    result.write_string(&ch.name);
    result.write_u8(new_race);
    result.write_u8(new_nation);
    result.write_u16(new_class);
    result.write_u8(ch.face);
    result.write_u32(ch.hair_rgb);
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_NATION_TRANSFER: request — nation {} → {}, race {} → {}",
        session.addr(),
        ch.nation,
        new_nation,
        ch.race,
        new_race,
    );

    Ok(())
}

/// Phase 2: Client confirms nation transfer with appearance choices.
/// Validates, updates memory, consumes item. Client disconnects itself.
#[cfg(test)]
#[allow(dead_code)]
async fn handle_confirm(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Re-validate all conditions
    if !has_item(&world, sid, ITEM_NATION_TRANSFER) {
        send_error(session, ERR_NO_ITEM).await?;
        return Ok(());
    }

    if world.is_war_open() {
        send_error(session, ERR_WAR_OPEN).await?;
        return Ok(());
    }

    if world.is_transformed(sid) {
        send_error(session, ERR_TRANSFORMED).await?;
        return Ok(());
    }

    // Re-validate: ALL account characters not in clan and none is king
    let account_err = check_all_account_chars(session, &world, ch.nation).await?;
    if account_err != ERR_NONE {
        send_error(session, account_err).await?;
        return Ok(());
    }

    // Read confirmation data
    let flag1 = reader.read_u8().unwrap_or(0);
    let flag2 = reader.read_u8().unwrap_or(0);
    let char_count = reader.read_u8().unwrap_or(0);

    if char_count == 0 || char_count > 4 {
        send_error(session, ERR_NONE).await?;
        return Ok(());
    }

    // Read first character's data
    let _char_num = reader.read_u16().unwrap_or(0);
    let char_name = match reader.read_string() {
        Some(s) => s,
        None => {
            send_error(session, ERR_NONE).await?;
            return Ok(());
        }
    };
    let new_race = reader.read_u8().unwrap_or(0);
    let new_face = reader.read_u8().unwrap_or(0);
    let new_hair = reader.read_u32().unwrap_or(0);

    // Validate character name matches
    if !char_name.eq_ignore_ascii_case(&ch.name) {
        send_error(session, ERR_NONE).await?;
        return Ok(());
    }

    // Validate race is a valid conversion target
    if new_race == 0 || !is_valid_target_race(new_race, ch.nation) {
        send_error(session, ERR_NONE).await?;
        return Ok(());
    }

    let new_nation = if ch.nation == 1 { 2u8 } else { 1u8 };
    let new_class = if ch.nation == 1 {
        ch.class + 100
    } else {
        ch.class.saturating_sub(100)
    };

    // Consume nation transfer scroll
    world.rob_item(sid, ITEM_NATION_TRANSFER, 1);

    // Update character in memory
    world.update_character_stats(sid, |c| {
        c.nation = new_nation;
        c.race = new_race;
        c.class = new_class;
        c.face = new_face;
        c.hair_rgb = new_hair;
        c.rank = 0;
        c.title = 0;
    });

    // Send success response (echo back the data)
    let mut result = Packet::new(Opcode::WizNationTransfer as u8);
    result.write_u8(NATION_OPEN_BOX);
    result.write_u8(flag1);
    result.write_u8(flag2);
    result.write_u8(1);
    result.write_u16(sid);
    result.write_string(&char_name);
    result.write_u8(new_race);
    result.write_u8(new_face);
    result.write_u32(new_hair);
    session.send_packet(&result).await?;

    world.clear_all_buffs(sid, false);
    world.set_user_ability(sid);
    world.recast_saved_magic(sid);

    debug!(
        "[{}] WIZ_NATION_TRANSFER: confirmed — {} nation {} → {}, race {} → {}",
        session.addr(),
        char_name,
        ch.nation,
        new_nation,
        ch.race,
        new_race,
    );

    // FerihaLog: NationTransferInsertLog
    super::audit_log::log_nation_transfer(
        session.pool(),
        session.account_id().unwrap_or(""),
        &char_name,
        ch.nation,
        new_nation,
    );

    // C++ disconnects the player after successful transfer.
    // The client handles disconnect itself after receiving the response.

    Ok(())
}

/// Convert race for nation transfer.
/// Returns the default target race (0 if invalid).
#[cfg(test)]
fn convert_race(current_race: u8, class: u16, nation: u8) -> u8 {
    if nation == 1 {
        // Karus → El Morad
        match current_race {
            KARUS_BIG => BABARIAN,
            KARUS_MIDDLE => ELMORAD_MAN,
            KARUS_SMALL => ELMORAD_MAN,
            KARUS_WOMAN => ELMORAD_WOMAN,
            KURIAN => PORUTU,
            _ => 0,
        }
    } else {
        // El Morad → Karus
        let class_type = (class % 100) as u8;
        match current_race {
            BABARIAN => KARUS_BIG,
            ELMORAD_MAN => match class_type {
                1 | 5 | 6 => KARUS_BIG,
                2 | 7 | 8 => KARUS_MIDDLE,
                3 | 9 | 10 => KARUS_SMALL,
                4 | 11 | 12 => KARUS_MIDDLE,
                13..=15 => KURIAN,
                _ => KARUS_MIDDLE,
            },
            ELMORAD_WOMAN => match class_type {
                1 | 5 | 6 => KARUS_BIG,
                2 | 7 | 8 => KARUS_MIDDLE,
                3 | 9 | 10 => KARUS_WOMAN,
                4 | 11 | 12 => KARUS_WOMAN,
                13..=15 => KURIAN,
                _ => KARUS_WOMAN,
            },
            PORUTU => KURIAN,
            _ => 0,
        }
    }
}

/// Check if the target race is valid for the player's current nation.
#[cfg(test)]
fn is_valid_target_race(target_race: u8, current_nation: u8) -> bool {
    if current_nation == 1 {
        // Karus → El Morad: target must be El Morad race
        matches!(target_race, BABARIAN | ELMORAD_MAN | ELMORAD_WOMAN | PORUTU)
    } else {
        // El Morad → Karus: target must be Karus race
        matches!(
            target_race,
            KARUS_BIG | KARUS_MIDDLE | KARUS_SMALL | KARUS_WOMAN | KURIAN
        )
    }
}

/// Check if player has a specific item in inventory.
#[cfg(test)]
#[allow(dead_code)]
fn has_item(world: &crate::world::WorldState, sid: crate::zone::SessionId, item_id: u32) -> bool {
    world.update_inventory(sid, |inv| {
        for i in 14..42 {
            if let Some(slot) = inv.get(i) {
                if slot.item_id == item_id && slot.count > 0 {
                    return true;
                }
            }
        }
        false
    })
}

/// Check ALL account characters for clan membership and king status.
/// and `NationTransferHandler.cpp:437-513` (ReqSendNationTransfer)
/// C++ calls `GetAllCharID()` to load all 4 character IDs from `account_char`,
/// then for each non-empty character:
/// - checks against king name → `ERR_IS_KING` (3)
/// - loads character data and checks `sClanID > 0` → `ERR_IN_CLAN` (2)
/// Returns `ERR_NONE` (0) if all characters pass.
#[cfg(test)]
#[allow(dead_code)]
async fn check_all_account_chars(
    session: &ClientSession,
    world: &crate::world::WorldState,
    nation: u8,
) -> anyhow::Result<u8> {
    let account_id = match session.account_id() {
        Some(id) => id,
        None => return Ok(ERR_NONE),
    };

    let char_repo = CharacterRepository::new(session.pool());
    let all_chars = char_repo.load_all_for_account(account_id).await?;

    // Check king status first (C++ checks king before clan)
    for ch in &all_chars {
        if world.is_king(nation, &ch.str_user_id) {
            debug!(
                "[{}] Nation transfer blocked: {} is king (nation={})",
                session.addr(),
                ch.str_user_id,
                nation,
            );
            return Ok(ERR_IS_KING);
        }
    }

    // Check clan membership
    for ch in &all_chars {
        if ch.knights > 0 {
            debug!(
                "[{}] Nation transfer blocked: {} is in clan (knights={})",
                session.addr(),
                ch.str_user_id,
                ch.knights,
            );
            return Ok(ERR_IN_CLAN);
        }
    }

    Ok(ERR_NONE)
}

/// Send a NATION_OPEN_BOX error response.
/// Format: `[0x82] [u8 NATION_OPEN_BOX=2] [u8 error_code]`
#[cfg(test)]
#[allow(dead_code)]
async fn send_error(session: &mut ClientSession, error: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
    pkt.write_u8(NATION_OPEN_BOX);
    pkt.write_u8(error);
    session.send_packet(&pkt).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::Packet;

    #[test]
    fn test_constants() {
        assert_eq!(NATION_OPEN_BOX, 2);
        assert_eq!(ITEM_NATION_TRANSFER, 810096000);
    }

    /// Verify error codes match C++ NationTransferHandler.cpp values.
    #[test]
    fn test_error_codes_match_cpp() {
        assert_eq!(ERR_NONE, 0);
        assert_eq!(ERR_WAR_OPEN, 1);
        assert_eq!(ERR_IN_CLAN, 2);
        assert_eq!(ERR_IS_KING, 3);
        assert_eq!(ERR_TRANSFORMED, 6);
        assert_eq!(ERR_NO_ITEM, 7);
    }

    #[test]
    fn test_convert_race_karus_to_elmorad() {
        assert_eq!(convert_race(KARUS_BIG, 101, 1), BABARIAN);
        assert_eq!(convert_race(KARUS_MIDDLE, 102, 1), ELMORAD_MAN);
        assert_eq!(convert_race(KARUS_SMALL, 103, 1), ELMORAD_MAN);
        assert_eq!(convert_race(KARUS_WOMAN, 104, 1), ELMORAD_WOMAN);
        assert_eq!(convert_race(KURIAN, 113, 1), PORUTU);
        // Invalid race
        assert_eq!(convert_race(5, 101, 1), 0);
        assert_eq!(convert_race(0, 101, 1), 0);
    }

    #[test]
    fn test_convert_race_elmorad_man_to_karus() {
        assert_eq!(convert_race(ELMORAD_MAN, 201, 2), KARUS_BIG); // Warrior
        assert_eq!(convert_race(ELMORAD_MAN, 205, 2), KARUS_BIG); // Blade
        assert_eq!(convert_race(ELMORAD_MAN, 202, 2), KARUS_MIDDLE); // Rogue
        assert_eq!(convert_race(ELMORAD_MAN, 207, 2), KARUS_MIDDLE); // Ranger
        assert_eq!(convert_race(ELMORAD_MAN, 203, 2), KARUS_SMALL); // Mage
        assert_eq!(convert_race(ELMORAD_MAN, 204, 2), KARUS_MIDDLE); // Priest
        assert_eq!(convert_race(ELMORAD_MAN, 213, 2), KURIAN); // Kurian
    }

    #[test]
    fn test_convert_race_elmorad_woman_to_karus() {
        assert_eq!(convert_race(ELMORAD_WOMAN, 201, 2), KARUS_BIG); // Warrior
        assert_eq!(convert_race(ELMORAD_WOMAN, 203, 2), KARUS_WOMAN); // Mage
        assert_eq!(convert_race(ELMORAD_WOMAN, 204, 2), KARUS_WOMAN); // Priest
        assert_eq!(convert_race(ELMORAD_WOMAN, 202, 2), KARUS_MIDDLE); // Rogue
    }

    #[test]
    fn test_convert_race_special() {
        assert_eq!(convert_race(BABARIAN, 201, 2), KARUS_BIG);
        assert_eq!(convert_race(PORUTU, 213, 2), KURIAN);
    }

    #[test]
    fn test_is_valid_target_race() {
        // Karus → El Morad
        assert!(is_valid_target_race(BABARIAN, 1));
        assert!(is_valid_target_race(ELMORAD_MAN, 1));
        assert!(is_valid_target_race(ELMORAD_WOMAN, 1));
        assert!(is_valid_target_race(PORUTU, 1));
        assert!(!is_valid_target_race(KARUS_BIG, 1));

        // El Morad → Karus
        assert!(is_valid_target_race(KARUS_BIG, 2));
        assert!(is_valid_target_race(KARUS_MIDDLE, 2));
        assert!(is_valid_target_race(KURIAN, 2));
        assert!(!is_valid_target_race(BABARIAN, 2));
    }

    #[test]
    fn test_request_packet() {
        let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
        pkt.write_u8(1); // sub=1

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
    }

    #[test]
    fn test_error_response() {
        let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
        pkt.write_u8(NATION_OPEN_BOX);
        pkt.write_u8(ERR_IN_CLAN);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(NATION_OPEN_BOX));
        assert_eq!(r.read_u8(), Some(ERR_IN_CLAN));
    }

    #[test]
    fn test_character_list_response() {
        let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
        pkt.write_u8(NATION_OPEN_BOX);
        pkt.write_u8(ERR_NONE);
        pkt.write_u8(1); // char count
        pkt.write_u16(42);
        pkt.write_string("TestPlayer");
        pkt.write_u8(BABARIAN);
        pkt.write_u8(2); // nation
        pkt.write_u16(201); // class
        pkt.write_u8(3); // face
        pkt.write_u32(0xFF00AA); // hair

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(NATION_OPEN_BOX));
        assert_eq!(r.read_u8(), Some(ERR_NONE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.read_string(), Some("TestPlayer".to_string()));
        assert_eq!(r.read_u8(), Some(BABARIAN));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(201));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(0xFF00AA));
    }

    #[test]
    fn test_class_conversion() {
        // Karus → El Morad: +100
        assert_eq!(101u16 + 100, 201);
        assert_eq!(113u16 + 100, 213);
        // El Morad → Karus: -100
        assert_eq!(201u16.saturating_sub(100), 101);
        assert_eq!(213u16.saturating_sub(100), 113);
    }

    // ── Sprint 964: Additional coverage ──────────────────────────────

    /// ITEM_NATION_TRANSFER matches C++ Define.h.
    #[test]
    fn test_nation_transfer_item_id() {
        assert_eq!(ITEM_NATION_TRANSFER, 810096000);
        assert!(ITEM_NATION_TRANSFER >= 800_000_000);
    }

    /// Error codes are distinct and cover all validation paths.
    #[test]
    fn test_error_codes_distinct() {
        let codes = [
            ERR_NONE,
            ERR_WAR_OPEN,
            ERR_IN_CLAN,
            ERR_IS_KING,
            ERR_TRANSFORMED,
            ERR_NO_ITEM,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j]);
            }
        }
    }

    /// NATION_OPEN_BOX response code is 2.
    #[test]
    fn test_nation_open_box_value() {
        assert_eq!(NATION_OPEN_BOX, 2);
    }

    /// Race constants are sequential for each nation group.
    #[test]
    fn test_race_constants_values() {
        // Karus races
        assert_eq!(KARUS_BIG, 1);
        assert_eq!(KARUS_MIDDLE, 2);
        assert_eq!(KARUS_SMALL, 3);
        assert_eq!(KARUS_WOMAN, 4);
        // Elmorad races
        assert_eq!(ELMORAD_MAN, 12);
        assert_eq!(ELMORAD_WOMAN, 13);
    }

    /// Kurian and special races.
    #[test]
    fn test_special_race_constants() {
        assert_eq!(BABARIAN, 11);
        assert_eq!(PORUTU, 14);
        assert_eq!(KURIAN, 6);
    }
}
