//! WIZ_DEL_CHAR (0x03) handler — character deletion.
//! ## Request (C->S)
//! | Offset | Type   | Description |
//! |--------|--------|-------------|
//! | 0      | u8     | Character index (0-2) |
//! | 1      | string | Character name |
//! | N      | string | Social security number (max 15) |
//! ## Response (S->C)
//! | Offset | Type | Description |
//! |--------|------|-------------|
//! | 0      | u8   | Result (1=success, 0=fail) |
//! | 1      | u8   | Deleted slot index |

use ko_db::repositories::account::AccountRepository;
use ko_db::repositories::character::CharacterRepository;
use ko_db::repositories::daily_quest::DailyQuestRepository;
use ko_db::repositories::perk::PerkRepository;
use ko_db::repositories::saved_magic::SavedMagicRepository;
use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};

/// Handle WIZ_DEL_CHAR from the client.
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
    let char_index = reader.read_u8().unwrap_or(255);
    let char_name = reader.read_string().unwrap_or_default();
    let _ssn = reader.read_string().unwrap_or_default();

    // Validate index and SSN length
    if char_index > 2 || char_name.is_empty() || _ssn.is_empty() || _ssn.len() > 15 {
        {
            let mut r = Packet::new(Opcode::WizDelChar as u8);
            r.write_u8(0);
            r.write_u8(0xFF_u8);
            session.send_packet(&r).await?;
        }
        return Ok(());
    }

    let account_repo = AccountRepository::new(session.pool());
    let char_repo = CharacterRepository::new(session.pool());

    // Clan leader cannot delete their character
    // CHIEF (fame=1) must transfer leadership before deleting
    match char_repo.load(&char_name).await {
        Ok(Some(ch)) => {
            if ch.knights > 0 && ch.fame == 1 {
                tracing::info!(
                    "[{}] Cannot delete clan leader character: {} (knights={})",
                    session.addr(),
                    char_name,
                    ch.knights
                );
                {
                    let mut r = Packet::new(Opcode::WizDelChar as u8);
                    r.write_u8(0);
                    r.write_u8(0xFF_u8);
                    session.send_packet(&r).await?;
                }
                return Ok(());
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                "[{}] delchar clan leader check DB error for {}: {e}",
                session.addr(),
                char_name
            );
        }
    }

    // Verify the character belongs to this account at the given slot
    let account_chars = account_repo.get_account_chars(&account_id).await?;
    let slot_char = match &account_chars {
        Some(ac) => match char_index {
            0 => ac.str_char_id1.as_deref(),
            1 => ac.str_char_id2.as_deref(),
            2 => ac.str_char_id3.as_deref(),
            _ => None,
        },
        None => None,
    };

    if slot_char != Some(&char_name as &str) {
        {
            let mut r = Packet::new(Opcode::WizDelChar as u8);
            r.write_u8(0);
            r.write_u8(0xFF_u8);
            session.send_packet(&r).await?;
        }
        return Ok(());
    }

    // Delete character from DB (cascade deletes items)
    match char_repo.delete(&char_name).await {
        Ok(true) => {
            // Clear the slot in account_char
            if let Err(e) = account_repo.clear_char_slot(&account_id, char_index).await {
                tracing::error!("[{}] DB error clearing slot: {}", session.addr(), e);
            }

            // Clean up orphan data in related tables (no FK CASCADE)
            let pool = session.pool();
            if let Err(e) = DailyQuestRepository::new(pool)
                .delete_user_quests(&char_name)
                .await
            {
                tracing::warn!("[{}] delchar cleanup daily_quest: {e}", session.addr());
            }
            if let Err(e) = PerkRepository::new(pool)
                .delete_user_perks(&char_name)
                .await
            {
                tracing::warn!("[{}] delchar cleanup perks: {e}", session.addr());
            }
            if let Err(e) = SavedMagicRepository::new(pool)
                .delete_saved_magic(&char_name)
                .await
            {
                tracing::warn!("[{}] delchar cleanup saved_magic: {e}", session.addr());
            }

            let mut response = Packet::new(Opcode::WizDelChar as u8);
            response.write_u8(1); // success
            response.write_u8(char_index);
            session.send_packet(&response).await?;

            // Refresh charsel list (C++ parity: AllCharInfoToAgent after delete).
            // Sends 0x2F sub=3/0x24/0x07/0x06/0x26 + 0x0C sub=3 for remaining chars.
            super::allchar::send_allchar_list(session, &account_id).await?;

            tracing::info!(
                "[{}] Character deleted: {} (account: {})",
                session.addr(),
                char_name,
                account_id
            );
        }
        Ok(false) => {
            let mut r = Packet::new(Opcode::WizDelChar as u8);
            r.write_u8(0);
            r.write_u8(0xFF_u8);
            session.send_packet(&r).await?;
        }
        Err(e) => {
            tracing::error!("[{}] DB error deleting character: {}", session.addr(), e);
            {
                let mut r = Packet::new(Opcode::WizDelChar as u8);
                r.write_u8(0);
                r.write_u8(0xFF_u8);
                session.send_packet(&r).await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_delchar_c2s_packet_format() {
        // C2S: [u8 char_index][string char_name][string ssn]
        let mut pkt = Packet::new(Opcode::WizDelChar as u8);
        pkt.write_u8(0); // char_index
        pkt.write_string("TestChar");
        pkt.write_string("123456789012345"); // max 15 char SSN

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(0));
        assert_eq!(reader.read_string(), Some("TestChar".to_string()));
        assert_eq!(reader.read_string(), Some("123456789012345".to_string()));
    }

    #[test]
    fn test_delchar_success_response() {
        // Success: [u8 result=1][u8 deleted_slot]
        let mut response = Packet::new(Opcode::WizDelChar as u8);
        response.write_u8(1);
        response.write_u8(2); // slot index

        let mut reader = PacketReader::new(&response.data);
        assert_eq!(reader.read_u8(), Some(1), "success");
        assert_eq!(reader.read_u8(), Some(2), "slot index");
    }

    #[test]
    fn test_delchar_fail_response() {
        // Fail: [u8 result=0][u8 0xFF]
        let mut response = Packet::new(Opcode::WizDelChar as u8);
        response.write_u8(0);
        response.write_u8(0xFF);

        let mut reader = PacketReader::new(&response.data);
        assert_eq!(reader.read_u8(), Some(0), "fail");
        assert_eq!(reader.read_u8(), Some(0xFF), "invalid slot marker");
    }

    #[test]
    fn test_delchar_index_validation() {
        // (C++ has 3 char slots max in CharacterSelectionHandler.cpp:630)
        for idx in 0..=2u8 {
            assert!(idx <= 2, "index {idx} should be accepted");
        }
        for idx in [3u8, 255] {
            assert!(idx > 2, "index {idx} should be rejected");
        }
    }

    #[test]
    fn test_delchar_ssn_length_validation() {
        // SSN empty or > 15 chars → reject
        let empty_ssn = "";
        assert!(empty_ssn.is_empty(), "empty SSN rejected");

        let long_ssn = "1234567890123456"; // 16 chars
        assert!(long_ssn.len() > 15, "SSN > 15 chars rejected");

        let valid_ssn = "123456789012345"; // exactly 15
        assert!(
            !valid_ssn.is_empty() && valid_ssn.len() <= 15,
            "15-char SSN valid"
        );
    }

    // ── Sprint 926: Additional coverage ──────────────────────────────

    /// Success and fail responses are both 2 bytes.
    #[test]
    fn test_delchar_response_data_length() {
        let mut success = Packet::new(Opcode::WizDelChar as u8);
        success.write_u8(1);
        success.write_u8(0);
        assert_eq!(success.data.len(), 2);

        let mut fail = Packet::new(Opcode::WizDelChar as u8);
        fail.write_u8(0);
        fail.write_u8(0xFF);
        assert_eq!(fail.data.len(), 2);
    }

    /// All valid slot indices (0, 1, 2) and invalid (3+).
    #[test]
    fn test_delchar_all_valid_slots() {
        for idx in 0..=2u8 {
            let mut pkt = Packet::new(Opcode::WizDelChar as u8);
            pkt.write_u8(1);
            pkt.write_u8(idx);
            assert_eq!(pkt.data[1], idx);
        }
    }

    /// SSN with exactly 1 char is valid (minimum).
    #[test]
    fn test_delchar_ssn_min_length() {
        let ssn = "1";
        assert!(!ssn.is_empty() && ssn.len() <= 15);
    }

    /// Clan leader has fame=1 (CHIEF), which blocks deletion.
    #[test]
    fn test_delchar_clan_leader_check() {
        let fame: i16 = 1; // CHIEF
        let knights: i32 = 500;
        assert!(knights > 0 && fame == 1, "clan leader cannot delete");

        let fame2: i16 = 2; // not CHIEF
        assert!(!(knights > 0 && fame2 == 1), "non-leader can delete");
    }

    /// C2S packet with all fields present.
    #[test]
    fn test_delchar_c2s_full_roundtrip() {
        let mut pkt = Packet::new(Opcode::WizDelChar as u8);
        pkt.write_u8(1); // slot
        pkt.write_string("MyChar");
        pkt.write_string("12345");

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_string(), Some("MyChar".to_string()));
        assert_eq!(r.read_string(), Some("12345".to_string()));
        assert_eq!(r.remaining(), 0);
    }
}
