//! WIZ_ALLCHAR_INFO_REQ (0x0C) handler — character list request.
//! ## Wireshark-Verified Format: 0x0C sub=1
//! Original server sends 0x0C sub=1 with 4 character slots.
//! Per slot (character): `[u16 name_len][name][u8 race][u16 class][i16 level]
//!   [u8 face][u32 hair][i16 zone][14×{u32 item_id + u16 dur}][18×0x00]`
//! Empty slot: `[u16 0][114×0x00]` = 116 bytes fixed.
//! Verified from 3 Wireshark captures: 467b, 479b, 490b packet sizes.
//! The previous 0x2F sub=3 approach was INCORRECT — original server never sends 0x2F
//! during character selection.

use ko_db::repositories::account::AccountRepository;
use ko_db::repositories::character::CharacterRepository;
use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};

/// Sub-opcode constants for WIZ_ALLCHAR_INFO_REQ (0x0C).
/// From C++ CUser::AllCharInfo / IDA char_trade.cpp.
const ALLCHAR_INFO_REQ: u8 = 1;
const ALLCHAR_NAME_CHANGE: u8 = 2;
#[allow(dead_code)]
const ALLCHAR_LOCATION_SEND: u8 = 3; // S2C only — server never receives this
const ALLCHAR_LOCATION_RECV: u8 = 4;

/// Visible equipment slots in charsel order
/// The C++ code loops i=0..13, writes only when `i` matches these slots.
/// Output order: HEAD, BREAST, SHOULDER, RIGHTHAND, LEFTHAND, LEG, GLOVE, FOOT.
const CHARSEL_EQUIP_SLOTS: &[i16] = &[1, 4, 5, 6, 8, 10, 12, 13];

/// Trailing zero bytes after 8 equipment entries.
/// Body = header(12) + 8×equip(48) + trailing(54) = 114 bytes.
const TRAILING_ZEROS: usize = 54;

/// Fixed body size (no name): header(12) + 8×6(48) + trailing(54) = 114 bytes.
/// Matches 14-slot format: header(12) + 14×6(84) + 18 trailing = 114.
const EMPTY_SLOT_BODY: usize = 1 + 2 + 2 + 1 + 4 + 2 + 8 * 6 + TRAILING_ZEROS;

/// Handle WIZ_ALLCHAR_INFO_REQ from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::LoggedIn && session.state() != SessionState::NationSelected
    {
        return Ok(());
    }

    let account_id = match session.account_id() {
        Some(id) => id.to_string(),
        None => return Ok(()),
    };

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        ALLCHAR_INFO_REQ => send_allchar_list(session, &account_id).await,
        ALLCHAR_NAME_CHANGE => handle_name_change(session, &account_id, &mut reader).await,
        ALLCHAR_LOCATION_RECV => handle_slot_reorder(session, &account_id, &mut reader).await,
        _ => {
            tracing::debug!(
                "[{}] Unhandled allchar sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Send the character selection list via 0x0C sub=1.
/// ## Wire Format (Wireshark verified)
/// ```text
/// [0x0C] [sub=1] [result=1]
/// [4 × slot]
/// ```
/// Per slot (character present):
/// ```text
/// [u16 name_len][name][u8 race][u16 class][i16 level]
/// [u8 face][u32 hair][i16 zone]
/// [14 × {u32 item_id, u16 durability}]
/// [18 × 0x00]
/// ```
/// Empty slot: `[u16 0][114 × 0x00]` = 116 bytes fixed.
pub async fn send_allchar_list(
    session: &mut ClientSession,
    account_id: &str,
) -> anyhow::Result<()> {
    let pool = session.pool().clone();
    let account_repo = AccountRepository::new(&pool);
    let char_repo = CharacterRepository::new(&pool);

    let account_chars = account_repo.get_account_chars(account_id).await?;

    let char_ids: [Option<String>; 4] = match &account_chars {
        Some(ac) => [
            ac.str_char_id1.clone(),
            ac.str_char_id2.clone(),
            ac.str_char_id3.clone(),
            ac.str_char_id4.clone(),
        ],
        None => [None, None, None, None],
    };

    // Batch-load all characters for this account (1 query).
    let all_chars = char_repo.load_all_for_account(account_id).await?;

    // Collect active character names for batch item query.
    let active_names: Vec<&str> = char_ids
        .iter()
        .filter_map(|c| c.as_deref().filter(|n| !n.is_empty()))
        .collect();

    // Batch-load equipped items (slot < 14) for all characters (1 query).
    let all_items = if active_names.is_empty() {
        Vec::new()
    } else {
        char_repo
            .load_equipped_items_batch(&active_names)
            .await
            .unwrap_or_default()
    };

    // Build 0x0C sub=1 response.
    let mut response = Packet::new(Opcode::WizAllcharInfoReq as u8);
    response.write_u8(1); // sub = 1
    response.write_u8(1); // result = success

    for char_id in &char_ids {
        match char_id {
            Some(name) if !name.is_empty() => {
                let user_data = all_chars.iter().find(|ud| ud.str_user_id == *name);

                match user_data {
                    Some(ud) => {
                        write_character_slot(&mut response, ud, name, &all_items);
                    }
                    None => {
                        write_empty_slot(&mut response);
                    }
                }
            }
            _ => {
                write_empty_slot(&mut response);
            }
        }
    }

    session.send_packet(&response).await?;

    tracing::debug!(
        "[{}] Sent allchar list (0x0C sub=1) data_len={}",
        session.addr(),
        response.data.len(),
    );
    Ok(())
}

/// Handle sub=2: character name change.
/// ## Wire Format
/// C2S: `[0x0C][sub=2][charRanking:u16][oldName:str][newName:str]`
/// S2C: `[0x0C][sub=2][result:u8]`
///   result: 3=success, 2=name taken, 0=error
async fn handle_name_change(
    session: &mut ClientSession,
    account_id: &str,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let _char_ranking = reader.read_u16().unwrap_or(0);
    let old_name = match reader.read_string() {
        Some(s) => s,
        None => return Ok(()),
    };
    let new_name = match reader.read_string() {
        Some(s) => s,
        None => return Ok(()),
    };

    tracing::info!(
        "[{}] Name change request: '{}' -> '{}'",
        session.addr(),
        old_name,
        new_name
    );

    let pool = session.pool().clone();
    let char_repo = CharacterRepository::new(&pool);
    let result = char_repo
        .rename_character(account_id, &old_name, &new_name)
        .await
        .unwrap_or(0);

    let mut response = Packet::new(Opcode::WizAllcharInfoReq as u8);
    response.write_u8(ALLCHAR_NAME_CHANGE);
    response.write_u8(result);
    session.send_packet(&response).await?;

    // On success, refresh the character list so the client sees the new name.
    if result == 3 {
        send_allchar_list(session, account_id).await?;
    }
    Ok(())
}

/// Handle sub=4: character slot reorder.
/// ## Wire Format
/// C2S: `[0x0C][sub=4][rank1:u8][rank2:u8][rank3:u8][rank4:u8]`
/// S2C: `[0x0C][sub=1]...` (refreshed char list)
/// The client sends the desired slot ordering as 4 rank bytes (0-3).
/// Each byte indicates which current slot goes into position N.
async fn handle_slot_reorder(
    session: &mut ClientSession,
    account_id: &str,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let ranks: [u8; 4] = [
        reader.read_u8().unwrap_or(0),
        reader.read_u8().unwrap_or(1),
        reader.read_u8().unwrap_or(2),
        reader.read_u8().unwrap_or(3),
    ];

    // Validate: each rank must be 0-3 and unique.
    let mut seen = [false; 4];
    for &r in &ranks {
        if r >= 4 || seen[r as usize] {
            tracing::warn!("[{}] Invalid slot reorder: {:?}", session.addr(), ranks);
            return Ok(());
        }
        seen[r as usize] = true;
    }

    let pool = session.pool().clone();
    let account_repo = AccountRepository::new(&pool);
    let account_chars = account_repo.get_account_chars(account_id).await?;

    if let Some(ac) = account_chars {
        let old_slots = [
            ac.str_char_id1,
            ac.str_char_id2,
            ac.str_char_id3,
            ac.str_char_id4,
        ];

        // Rearrange: new_slot[i] = old_slots[ranks[i]]
        let new_slots: [Option<String>; 4] = [
            old_slots[ranks[0] as usize].clone(),
            old_slots[ranks[1] as usize].clone(),
            old_slots[ranks[2] as usize].clone(),
            old_slots[ranks[3] as usize].clone(),
        ];

        account_repo
            .reorder_char_slots(
                account_id,
                [
                    new_slots[0].as_deref(),
                    new_slots[1].as_deref(),
                    new_slots[2].as_deref(),
                    new_slots[3].as_deref(),
                ],
            )
            .await?;

        tracing::info!("[{}] Slot reorder: {:?}", session.addr(), ranks);
    }

    // Refresh the character list after reorder.
    send_allchar_list(session, account_id).await
}

/// Write a populated character slot to the packet.
/// Format: `[string name][u8 race][u16 class][i16 level][u8 face]`
///         `[u32 hair][i16 zone][14×{u32 item_id, u16 dur}][18×0x00]`
fn write_character_slot(
    pkt: &mut Packet,
    ud: &ko_db::models::UserData,
    name: &str,
    all_items: &[ko_db::models::UserItem],
) {
    pkt.write_string(name);
    pkt.write_u8(ud.race as u8);
    pkt.write_u16(ud.class as u16);
    pkt.write_i16(ud.level);
    pkt.write_u8(ud.face as u8);
    pkt.write_u32(ud.hair_rgb as u32);
    pkt.write_i16(ud.zone);

    // 8 visible equipment slots only (C++ loops 0..14, writes only visible).
    // Order: HEAD(1), BREAST(4), SHOULDER(5), RIGHTHAND(6),
    //        LEFTHAND(8), LEG(10), GLOVE(12), FOOT(13).
    for &slot in CHARSEL_EQUIP_SLOTS {
        if let Some(item) = all_items
            .iter()
            .find(|i| i.str_user_id == name && i.slot_index == slot)
        {
            pkt.write_u32(item.item_id as u32);
            pkt.write_u16(item.durability as u16);
        } else {
            pkt.write_u32(0);
            pkt.write_u16(0);
        }
    }

    // 54 trailing bytes. TEMPORARY DIAGNOSTIC: byte i = 100 + i, so the second number the character select shows
    // ("level/x") tells which byte the client reads as the rebirth level; then that byte is set for real.
    for i in 0..TRAILING_ZEROS {
        pkt.write_u8(100 + i as u8);
    }
}

/// Write an empty character slot: `[u16 0][114×0x00]`.
fn write_empty_slot(pkt: &mut Packet) {
    pkt.write_i16(0); // name_len = 0
    for _ in 0..EMPTY_SLOT_BODY {
        pkt.write_u8(0);
    }
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use super::*;

    #[test]
    fn test_allchar_c2s_packet_format() {
        let mut pkt = Packet::new(Opcode::WizAllcharInfoReq as u8);
        pkt.write_u8(1);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(1));
    }

    #[test]
    fn test_empty_slot_size() {
        let mut pkt = Packet::new(0x0C);
        write_empty_slot(&mut pkt);
        // u16(0) + 114 zeros = 116
        assert_eq!(pkt.data.len(), 2 + EMPTY_SLOT_BODY);
        assert_eq!(pkt.data.len(), 116);
    }

    #[test]
    fn test_empty_slot_all_zeros() {
        let mut pkt = Packet::new(0x0C);
        write_empty_slot(&mut pkt);
        assert!(pkt.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_all_empty_response() {
        let mut pkt = Packet::new(Opcode::WizAllcharInfoReq as u8);
        pkt.write_u8(1);
        pkt.write_u8(1);
        for _ in 0..4 {
            write_empty_slot(&mut pkt);
        }
        // sub(1) + result(1) + 4 × 116 = 466
        assert_eq!(pkt.data.len(), 466);
    }

    #[test]
    fn test_slot_roundtrip() {
        let mut pkt = Packet::new(0x0C);
        pkt.write_string("TestHero");
        pkt.write_u8(1); // race
        pkt.write_u16(101); // class
        pkt.write_i16(60); // level
        pkt.write_u8(3); // face
        pkt.write_u32(0x00FF8800); // hair
        pkt.write_i16(21); // zone
                           // 8 visible equip slots + 54 trailing
        for _ in 0..8 {
            pkt.write_u32(0);
            pkt.write_u16(0);
        }
        for _ in 0..TRAILING_ZEROS {
            pkt.write_u8(0);
        }

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_string(), Some("TestHero".to_string()));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(101));
        assert_eq!(r.read_i16(), Some(60));
        assert_eq!(r.read_u8(), Some(3)); // face
        assert_eq!(r.read_u32(), Some(0x00FF8800));
        assert_eq!(r.read_i16(), Some(21));
        for _ in 0..8 {
            assert_eq!(r.read_u32(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
        }
        for _ in 0..TRAILING_ZEROS {
            assert_eq!(r.read_u8(), Some(0));
        }
        assert_eq!(r.read_u8(), None);
    }

    #[test]
    fn test_constants() {
        assert_eq!(CHARSEL_EQUIP_SLOTS.len(), 8);
        assert_eq!(CHARSEL_EQUIP_SLOTS, &[1, 4, 5, 6, 8, 10, 12, 13]);
        assert_eq!(TRAILING_ZEROS, 54);
        assert_eq!(EMPTY_SLOT_BODY, 114);
    }

    #[test]
    fn test_sub_opcode_constants() {
        assert_eq!(ALLCHAR_INFO_REQ, 1);
        assert_eq!(ALLCHAR_NAME_CHANGE, 2);
        assert_eq!(ALLCHAR_LOCATION_SEND, 3);
        assert_eq!(ALLCHAR_LOCATION_RECV, 4);
    }

    #[test]
    fn test_name_change_c2s_format() {
        // C2S: [0x0C][sub=2][charRanking:u16][oldName:str][newName:str]
        let mut pkt = Packet::new(Opcode::WizAllcharInfoReq as u8);
        pkt.write_u8(ALLCHAR_NAME_CHANGE);
        pkt.write_u16(0); // charRanking
        pkt.write_string("OldHero");
        pkt.write_string("NewHero");

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ALLCHAR_NAME_CHANGE));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_string(), Some("OldHero".to_string()));
        assert_eq!(r.read_string(), Some("NewHero".to_string()));
        assert_eq!(r.read_u8(), None); // consumed all
    }

    #[test]
    fn test_name_change_s2c_format() {
        // S2C: [0x0C][sub=2][result:u8]
        let mut pkt = Packet::new(Opcode::WizAllcharInfoReq as u8);
        pkt.write_u8(ALLCHAR_NAME_CHANGE);
        pkt.write_u8(3); // success

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ALLCHAR_NAME_CHANGE));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), None);
    }

    #[test]
    fn test_slot_reorder_c2s_format() {
        // C2S: [0x0C][sub=4][rank1:u8][rank2:u8][rank3:u8][rank4:u8]
        let mut pkt = Packet::new(Opcode::WizAllcharInfoReq as u8);
        pkt.write_u8(ALLCHAR_LOCATION_RECV);
        pkt.write_u8(2); // slot 2 -> position 0
        pkt.write_u8(0); // slot 0 -> position 1
        pkt.write_u8(1); // slot 1 -> position 2
        pkt.write_u8(3); // slot 3 -> position 3

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(ALLCHAR_LOCATION_RECV));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), None);
    }
}
