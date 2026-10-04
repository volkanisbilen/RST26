//! WIZ_NEW_CHAR (0x02) handler — character creation.
//! ## Request (C->S)
//! | Offset | Type   | Description |
//! |--------|--------|-------------|
//! | 0      | u8     | Character index (0-3) |
//! | 1      | string | Character name |
//! | N      | u8     | Race |
//! | N+1    | u16le  | Class |
//! | N+3    | u8     | Face |
//! | N+4    | u32le  | Hair RGB/ID |
//! | N+8    | u8     | STR |
//! | N+9    | u8     | STA |
//! | N+10   | u8     | DEX |
//! | N+11   | u8     | INT |
//! | N+12   | u8     | CHA |
//! ## Response (S->C)
//! v2600 has TWO dispatch paths for opcode 0x02:
//! 1. **Top-level handler** (0xB47BA0, param_4=1): reads `[u8 result]`.
//!    If result != 1 → exits silently. Then calls ALLCHAR_refresh.
//!    C++ server sends `[u8 0]` for success — handler exits, ALLCHAR_refresh runs.
//! 2. **CharSelectRecv sub=0x02** (0x74E7F1): reads `[i32 result] [string name]`.
//!    Only triggered by opcode 0x0C sub=0x02, NOT by opcode 0x02.
//! Strategy: Send opcode 0x0C sub=0x02 for CharSelectRecv feedback,
//! then send ALLCHAR refresh (0x0C sub=0x01) to update character list.

use ko_db::repositories::account::AccountRepository;
use ko_db::repositories::char_creation::CharCreationRepository;
use ko_db::repositories::character::{CharacterRepository, CreateCharParams};
use ko_db::repositories::daily_rank::DailyRankRepository;
use ko_db::repositories::perk::PerkRepository;
use ko_db::repositories::user_data::UserDataRepository;
use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::world::MAX_ID_SIZE;

// New character error codes from C++ reference (globals.h:357-365).
const NEWCHAR_SUCCESS: u8 = 0;
const NEWCHAR_NO_MORE: u8 = 1;
const NEWCHAR_INVALID_DETAILS: u8 = 2;
const NEWCHAR_EXISTS: u8 = 3;
const NEWCHAR_INVALID_NAME: u8 = 4;
const NEWCHAR_STAT_TOO_LOW: u8 = 5;
const NEWCHAR_POINTS_REMAINING: u8 = 6;
const NEWCHAR_INVALID_RACE: u8 = 7;
const NEWCHAR_INVALID_CLASS: u8 = 9;

/// Default spawn zone (Moradon = 21).
const DEFAULT_ZONE: i16 = 21;
/// Default spawn X position (multiplied by 100).
const DEFAULT_PX: i32 = 81600;
/// Default spawn Z position (multiplied by 100).
const DEFAULT_PZ: i32 = 53200;
/// Default spawn Y position (multiplied by 100).
const DEFAULT_PY: i32 = 0;

/// Send NEWCHAR error response (opcode 0x02).
/// v2600 top-level handler (sub_B47BA0, a4=1): reads first byte.
/// If byte != 1 → exits silently (no error display).
/// Error codes are sent as u8: 0=success, 1-11=error.
async fn send_newchar_error(session: &mut ClientSession, error_code: u8) -> anyhow::Result<()> {
    let mut response = Packet::new(Opcode::WizNewChar as u8);
    response.write_u8(error_code);
    session.send_packet(&response).await
}

/// Handle WIZ_NEW_CHAR from the client.
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

    // Parse request fields
    let char_index = reader.read_u8().unwrap_or(255);
    let char_name = reader.read_string().unwrap_or_default();
    let race = reader.read_u8().unwrap_or(0);
    let class = reader.read_u16().unwrap_or(0);
    let face = reader.read_u8().unwrap_or(0);
    let hair = reader.read_u32().unwrap_or(0);
    let str_val = reader.read_u8().unwrap_or(0);
    let sta_val = reader.read_u8().unwrap_or(0);
    let dex_val = reader.read_u8().unwrap_or(0);
    let int_val = reader.read_u8().unwrap_or(0);
    let cha_val = reader.read_u8().unwrap_or(0);

    // Validation — matches C++ order (CharacterSelectionHandler.cpp:411-426)
    let error = validate_new_char(
        char_index, &char_name, race, class, str_val, sta_val, dex_val, int_val, cha_val,
    );

    if error != NEWCHAR_SUCCESS {
        send_newchar_error(session, error).await?;
        return Ok(());
    }

    let account_repo = AccountRepository::new(session.pool());
    let char_repo = CharacterRepository::new(session.pool());

    // Check if character name already exists
    match char_repo.name_exists(&char_name).await {
        Ok(true) => {
            send_newchar_error(session, NEWCHAR_EXISTS).await?;
            return Ok(());
        }
        Err(e) => {
            tracing::error!("[{}] DB error checking name: {}", session.addr(), e);
            send_newchar_error(session, NEWCHAR_INVALID_DETAILS).await?;
            return Ok(());
        }
        _ => {}
    }

    // Get the nation from account_char
    let nation = match account_repo.get_account_chars(&account_id).await? {
        Some(ac) => ac.b_nation,
        None => {
            send_newchar_error(session, NEWCHAR_INVALID_DETAILS).await?;
            return Ok(());
        }
    };

    // Create character in database
    let params = CreateCharParams {
        char_id: &char_name,
        nation,
        race: race as i16,
        class: class as i16,
        face: face as i16,
        hair: hair as i32,
        strong: str_val as i16,
        sta: sta_val as i16,
        dex: dex_val as i16,
        intel: int_val as i16,
        cha: cha_val as i16,
        zone: DEFAULT_ZONE,
        px: DEFAULT_PX,
        pz: DEFAULT_PZ,
        py: DEFAULT_PY,
    };

    match char_repo.create(&params).await {
        Ok(()) => {
            // Assign character to account slot
            if let Err(e) = account_repo
                .set_char_slot(&account_id, char_index, &char_name)
                .await
            {
                tracing::error!("[{}] DB error setting char slot: {}", session.addr(), e);
            }

            let char_creation_repo = CharCreationRepository::new(session.pool());
            let beginner_type = match char_creation_repo.load_beginner_type(1).await {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(
                        "[{}] Could not load beginner type for new character '{}': {}",
                        session.addr(),
                        char_name,
                        error
                    );
                    1
                }
            };

            // Apply level-specific equipment when configured; otherwise use legacy class set.
            // The create_new_char_set table uses base class values (1-4, 13),
            // not the full class value (101, 102, 201, etc.).
            let class_type = (class % 100) as i16;
            let equipment = match char_creation_repo
                .load_starting_equipment(class_type, beginner_type)
                .await
            {
                Ok(items) => items,
                Err(error) => {
                    tracing::error!(
                        "[{}] Could not load starting items for '{}' (class={}, beginner_type={}): {}",
                        session.addr(),
                        char_name,
                        class_type,
                        beginner_type,
                        error
                    );
                    Vec::new()
                }
            };
            tracing::info!(
                "[{}] New character '{}' uses starting items: class={}, beginner_type={}, item_slots={}",
                session.addr(),
                char_name,
                class_type,
                beginner_type,
                equipment.len()
            );
            if !equipment.is_empty() {
                if let Err(e) = char_repo
                    .apply_starting_equipment(&char_name, &equipment)
                    .await
                {
                    tracing::error!(
                        "[{}] DB error applying starting equipment: {}",
                        session.addr(),
                        e
                    );
                }
            }

            // C++ parity: BEGINNER_SETTINGS selects CREATE_NEW_CHAR_VALUE.job_type.
            let world = session.world();
            if let Some(stats) = world.get_starting_stats(class_type, beginner_type) {
                if let Err(e) = char_repo.apply_starting_stats(&char_name, &stats).await {
                    tracing::error!(
                        "[{}] DB error applying starting stats: {}",
                        session.addr(),
                        e
                    );
                }
            }

            // ── Initialize per-character tables (C++ CREATE_NEW_CHAR SP parity) ──
            // These INSERTs ensure downstream systems (daily rank, genie, perks)
            // find pre-existing rows on first login.

            // daily_rank — empty rank row
            let daily_rank_repo = DailyRankRepository::new(session.pool());
            if let Err(e) = daily_rank_repo.init_for_new_char(&char_name).await {
                tracing::error!(
                    "[{}] DB error creating daily_rank for {}: {}",
                    session.addr(),
                    char_name,
                    e
                );
            }
            // user_daily_rank_stats — empty stats row
            if let Err(e) = daily_rank_repo.init_stats_for_new_char(&char_name).await {
                tracing::error!(
                    "[{}] DB error creating user_daily_rank_stats for {}: {}",
                    session.addr(),
                    char_name,
                    e
                );
            }

            // user_genie_data — genie row with optional initial hours from server_settings.
            let give_genie_hour = session
                .world()
                .get_server_settings()
                .map(|s| s.give_genie_hour)
                .unwrap_or(0);
            let initial_genie_time = if give_genie_hour > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i32;
                now.saturating_add(give_genie_hour as i32 * 3600)
            } else {
                0
            };
            let user_data_repo = UserDataRepository::new(session.pool());
            if let Err(e) = user_data_repo
                .save_genie_data(&char_name, initial_genie_time, &[0u8], 0)
                .await
            {
                tracing::error!(
                    "[{}] DB error creating user_genie_data for {}: {}",
                    session.addr(),
                    char_name,
                    e
                );
            }

            // user_perks — empty perk row (all 0)
            let perk_repo = PerkRepository::new(session.pool());
            if let Err(e) = perk_repo.save_user_perks(&char_name, &[0i16; 13], 0).await {
                tracing::error!(
                    "[{}] DB error creating user_perks for {}: {}",
                    session.addr(),
                    char_name,
                    e
                );
            }

            // user_soul_data — empty soul row (all 0, v2525)
            let soul_repo = ko_db::repositories::soul::SoulRepository::new(session.pool());
            let empty_cats: [[i16; 4]; 8] = [
                [0, 0, 0, 0],
                [1, 0, 0, 0],
                [2, 0, 0, 0],
                [3, 0, 0, 0],
                [4, 0, 0, 0],
                [5, 0, 0, 0],
                [6, 0, 0, 0],
                [7, 0, 0, 0],
            ];
            let empty_slots: [[i16; 2]; 5] = [[0, 0], [1, 0], [2, 0], [3, 0], [4, 0]];
            if let Err(e) = soul_repo.save(&char_name, &empty_cats, &empty_slots).await {
                tracing::error!(
                    "[{}] DB error creating user_soul_data for {}: {}",
                    session.addr(),
                    char_name,
                    e
                );
            }

            // Auto-grant premium from server_settings.
            //   if (pServerSetting.premiumID && pServerSetting.premiumTime)
            //     GivePremium((uint8)premiumID, premiumTime, true);
            let (prem_id, prem_time) = session
                .world()
                .get_server_settings()
                .map(|s| (s.premium_id, s.premium_time))
                .unwrap_or((0, 0));
            if prem_id > 0 && prem_time > 0 {
                super::premium::give_premium(session, prem_id as u8, prem_time as u16, true);
            }

            // Send error code 0 (success) via opcode 0x02 — C++ parity.
            // Then resend full character list via sub=0x03 per character.
            send_newchar_error(session, NEWCHAR_SUCCESS).await?;
            super::allchar::send_allchar_list(session, &account_id).await?;

            tracing::info!(
                "[{}] Character created: {} (account: {})",
                session.addr(),
                char_name,
                account_id
            );
        }
        Err(e) => {
            tracing::error!("[{}] DB error creating character: {}", session.addr(), e);
            send_newchar_error(session, NEWCHAR_INVALID_DETAILS).await?;
        }
    }

    Ok(())
}

/// Validate new character parameters.
/// Returns NEWCHAR_SUCCESS (0) if valid, or an error code.
/// Validation order matches C++ exactly:
/// 1. char_index > 3
/// 2. total > 300 OR race/class combo invalid → INVALID_DETAILS
/// 3. class not in whitelist → INVALID_CLASS
/// 4. race not in whitelist → INVALID_RACE
/// 5. total < 300 → POINTS_REMAINING
/// 6. any stat < 50 → STAT_TOO_LOW
/// 7. name empty/too long → INVALID_NAME
#[allow(clippy::too_many_arguments)]
fn validate_new_char(
    char_index: u8,
    char_name: &str,
    race: u8,
    class: u16,
    str_val: u8,
    sta_val: u8,
    dex_val: u8,
    int_val: u8,
    cha_val: u8,
) -> u8 {
    // C++ line 411
    if char_index > 3 {
        return NEWCHAR_NO_MORE;
    }

    let total = str_val as u16 + sta_val as u16 + dex_val as u16 + int_val as u16 + cha_val as u16;

    // C++ line 413-415: coefficient null check skipped (handled by class whitelist),
    // total > 300 OR race/class combo invalid
    if total > 300 || !new_char_valid(race, class) {
        return NEWCHAR_INVALID_DETAILS;
    }

    // C++ line 417
    if !new_char_class_valid(class) {
        return NEWCHAR_INVALID_CLASS;
    }

    // C++ line 419
    if !new_char_race_valid(race) {
        return NEWCHAR_INVALID_RACE;
    }

    // C++ line 421
    if total < 300 {
        return NEWCHAR_POINTS_REMAINING;
    }

    // C++ line 423
    if str_val < 50 || sta_val < 50 || dex_val < 50 || int_val < 50 || cha_val < 50 {
        return NEWCHAR_STAT_TOO_LOW;
    }

    // C++ line 425 — name validation uses string_is_valid equivalent
    if char_name.is_empty()
        || char_name.len() > MAX_ID_SIZE
        || !char_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return NEWCHAR_INVALID_NAME;
    }

    NEWCHAR_SUCCESS
}

/// Validate race is in the allowed set.
/// Valid races: 1-4 (Karus), 6 (Kurian), 11-14 (El Morad)
fn new_char_race_valid(race: u8) -> bool {
    matches!(race, 1 | 2 | 3 | 4 | 6 | 11 | 12 | 13 | 14)
}

/// Validate class is in the allowed set.
/// Valid classes: 101-115 (Karus side), 201-215 (El Morad side)
fn new_char_class_valid(class: u16) -> bool {
    matches!(
        class,
        101 | 102
            | 103
            | 104
            | 105
            | 106
            | 107
            | 108
            | 109
            | 110
            | 111
            | 112
            | 113
            | 114
            | 115
            | 201
            | 202
            | 203
            | 204
            | 205
            | 206
            | 207
            | 208
            | 209
            | 210
            | 211
            | 212
            | 213
            | 214
            | 215
    )
}

/// Validate race/class combination is compatible.
/// Each race can only start with specific base classes.
fn new_char_valid(race: u8, class: u16) -> bool {
    match race {
        1 => class == 101,                     // Arch Tuarek → Warrior
        2 => class == 102 || class == 104,     // Tuarek → Rogue or Priest
        3 => class == 103,                     // Wrinkle Tuarek → Mage
        4 => class == 103 || class == 104,     // Puri Tuarek → Mage or Priest
        6 => class == 113,                     // Kurian → Kurian
        11 => class == 201,                    // Barbarian → Warrior
        12 | 13 => matches!(class, 201..=204), // El Morad M/F → any base class
        14 => class == 213,                    // Porutu → Kurian
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;
    use ko_db::models::char_creation::{CreateNewCharSetRow, CreateNewCharValueRow};

    #[test]
    fn test_validate_new_char_success() {
        // Arch Tuarek (race=1) + Warrior (class=101), stats=300
        let result = validate_new_char(0, "TestChar", 1, 101, 60, 60, 60, 60, 60);
        assert_eq!(result, NEWCHAR_SUCCESS);
    }

    #[test]
    fn test_validate_new_char_exact_300() {
        let result = validate_new_char(0, "TestChar", 11, 201, 50, 50, 50, 50, 100);
        assert_eq!(result, NEWCHAR_SUCCESS);
    }

    #[test]
    fn test_validate_new_char_index_too_high() {
        let result = validate_new_char(4, "TestChar", 1, 101, 60, 60, 60, 60, 60);
        assert_eq!(result, NEWCHAR_NO_MORE);
    }

    #[test]
    fn test_validate_new_char_empty_name() {
        let result = validate_new_char(0, "", 1, 101, 60, 60, 60, 60, 60);
        assert_eq!(result, NEWCHAR_INVALID_NAME);
    }

    #[test]
    fn test_validate_new_char_name_too_long() {
        let long_name = "a".repeat(22);
        let result = validate_new_char(0, &long_name, 1, 101, 60, 60, 60, 60, 60);
        assert_eq!(result, NEWCHAR_INVALID_NAME);
    }

    #[test]
    fn test_validate_new_char_special_chars_rejected() {
        // Names with spaces, hyphens, or special chars must be rejected
        assert_eq!(
            validate_new_char(0, "Test Char", 1, 101, 60, 60, 60, 60, 60),
            NEWCHAR_INVALID_NAME
        );
        assert_eq!(
            validate_new_char(0, "Test-Char", 1, 101, 60, 60, 60, 60, 60),
            NEWCHAR_INVALID_NAME
        );
        assert_eq!(
            validate_new_char(0, "Test!@#", 1, 101, 60, 60, 60, 60, 60),
            NEWCHAR_INVALID_NAME
        );
        // Underscore allowed per C++ string_is_valid
        assert_eq!(
            validate_new_char(0, "Test_Char", 1, 101, 60, 60, 60, 60, 60),
            NEWCHAR_SUCCESS
        );
        // Alphanumeric names pass
        assert_eq!(
            validate_new_char(0, "TestChar123", 1, 101, 60, 60, 60, 60, 60),
            NEWCHAR_SUCCESS
        );
    }

    #[test]
    fn test_validate_new_char_stat_too_low() {
        let result = validate_new_char(0, "TestChar", 1, 101, 49, 60, 60, 60, 71);
        assert_eq!(result, NEWCHAR_STAT_TOO_LOW);
    }

    #[test]
    fn test_validate_new_char_points_remaining() {
        let result = validate_new_char(0, "TestChar", 1, 101, 50, 50, 50, 50, 50);
        assert_eq!(result, NEWCHAR_POINTS_REMAINING);
    }

    #[test]
    fn test_validate_new_char_over_300() {
        let result = validate_new_char(0, "TestChar", 1, 101, 60, 61, 60, 60, 60);
        assert_eq!(result, NEWCHAR_INVALID_DETAILS);
    }

    // ── Race/Class Validation Tests ─────────────────────────────────

    #[test]
    fn test_new_char_race_valid() {
        // Karus races: 1-4, 6
        assert!(new_char_race_valid(1));
        assert!(new_char_race_valid(2));
        assert!(new_char_race_valid(3));
        assert!(new_char_race_valid(4));
        assert!(new_char_race_valid(6));
        // El Morad races: 11-14
        assert!(new_char_race_valid(11));
        assert!(new_char_race_valid(12));
        assert!(new_char_race_valid(13));
        assert!(new_char_race_valid(14));
        // Invalid races
        assert!(!new_char_race_valid(0));
        assert!(!new_char_race_valid(5));
        assert!(!new_char_race_valid(7));
        assert!(!new_char_race_valid(10));
        assert!(!new_char_race_valid(15));
    }

    #[test]
    fn test_new_char_class_valid() {
        // Karus classes: 101-115
        for c in [
            101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115,
        ] {
            assert!(new_char_class_valid(c), "class {} should be valid", c);
        }
        // El Morad classes: 201-215
        for c in [
            201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215,
        ] {
            assert!(new_char_class_valid(c), "class {} should be valid", c);
        }
        // Invalid classes
        assert!(!new_char_class_valid(0));
        assert!(!new_char_class_valid(100));
        assert!(!new_char_class_valid(116));
        assert!(!new_char_class_valid(200));
        assert!(!new_char_class_valid(216));
        assert!(!new_char_class_valid(300));
    }

    #[test]
    fn test_new_char_valid_karus_combos() {
        // C++ CharacterSelectionHandler.cpp:526-571
        // Race 1 (Arch Tuarek) → only class 101 (Warrior)
        assert!(new_char_valid(1, 101));
        assert!(!new_char_valid(1, 102));
        assert!(!new_char_valid(1, 103));
        assert!(!new_char_valid(1, 104));

        // Race 2 (Tuarek) → class 102 (Rogue) or 104 (Priest)
        assert!(new_char_valid(2, 102));
        assert!(new_char_valid(2, 104));
        assert!(!new_char_valid(2, 101));
        assert!(!new_char_valid(2, 103));

        // Race 3 (Wrinkle Tuarek) → only class 103 (Mage)
        assert!(new_char_valid(3, 103));
        assert!(!new_char_valid(3, 101));

        // Race 4 (Puri Tuarek) → class 103 (Mage) or 104 (Priest)
        assert!(new_char_valid(4, 103));
        assert!(new_char_valid(4, 104));
        assert!(!new_char_valid(4, 101));

        // Race 6 (Kurian) → only class 113
        assert!(new_char_valid(6, 113));
        assert!(!new_char_valid(6, 101));
    }

    #[test]
    fn test_new_char_valid_elmorad_combos() {
        // Race 11 (Barbarian) → only class 201 (Warrior)
        assert!(new_char_valid(11, 201));
        assert!(!new_char_valid(11, 202));

        // Race 12 (El Morad Male) → classes 201-204
        assert!(new_char_valid(12, 201));
        assert!(new_char_valid(12, 202));
        assert!(new_char_valid(12, 203));
        assert!(new_char_valid(12, 204));
        assert!(!new_char_valid(12, 213));

        // Race 13 (El Morad Female) → classes 201-204
        assert!(new_char_valid(13, 201));
        assert!(new_char_valid(13, 202));
        assert!(new_char_valid(13, 203));
        assert!(new_char_valid(13, 204));

        // Race 14 (Porutu) → only class 213
        assert!(new_char_valid(14, 213));
        assert!(!new_char_valid(14, 201));
    }

    #[test]
    fn test_new_char_valid_cross_nation_rejected() {
        // Karus race with El Morad class
        assert!(!new_char_valid(1, 201));
        assert!(!new_char_valid(2, 202));
        // El Morad race with Karus class
        assert!(!new_char_valid(11, 101));
        assert!(!new_char_valid(12, 102));
    }

    #[test]
    fn test_validate_invalid_race_returns_error() {
        // Race 5 is invalid → new_char_valid fails → INVALID_DETAILS
        let result = validate_new_char(0, "TestChar", 5, 101, 60, 60, 60, 60, 60);
        assert_eq!(result, NEWCHAR_INVALID_DETAILS);
    }

    #[test]
    fn test_validate_invalid_class_returns_error() {
        // Race 1 can only be class 101; class 102 fails combo → INVALID_DETAILS
        let result = validate_new_char(0, "TestChar", 1, 102, 60, 60, 60, 60, 60);
        assert_eq!(result, NEWCHAR_INVALID_DETAILS);
    }

    #[test]
    fn test_error_code_constants() {
        assert_eq!(NEWCHAR_SUCCESS, 0);
        assert_eq!(NEWCHAR_NO_MORE, 1);
        assert_eq!(NEWCHAR_INVALID_DETAILS, 2);
        assert_eq!(NEWCHAR_EXISTS, 3);
        assert_eq!(NEWCHAR_INVALID_NAME, 4);
        assert_eq!(NEWCHAR_STAT_TOO_LOW, 5);
        assert_eq!(NEWCHAR_POINTS_REMAINING, 6);
        assert_eq!(NEWCHAR_INVALID_RACE, 7);
        assert_eq!(NEWCHAR_INVALID_CLASS, 9);
    }

    #[test]
    fn test_world_starting_equipment_empty_when_no_data() {
        let world = WorldState::new();
        let equip = world.get_starting_equipment(1);
        assert!(equip.is_empty());
    }

    #[test]
    fn test_world_starting_stats_none_when_no_data() {
        let world = WorldState::new();
        let stats = world.get_starting_stats(1, 0);
        assert!(stats.is_none());
    }

    #[test]
    fn test_world_starting_equipment_returns_data() {
        let world = WorldState::new();
        let row = CreateNewCharSetRow {
            id: 1,
            class_type: 1,
            slot_id: 0,
            item_id: 1310515313,
            item_duration: 1,
            item_count: 1,
            item_flag: 0,
            item_expire_time: 0,
        };
        world.insert_test_new_char_set(1, row);
        let equip = world.get_starting_equipment(1);
        assert_eq!(equip.len(), 1);
        assert_eq!(equip[0].item_id, 1310515313);
        assert_eq!(equip[0].slot_id, 0);
    }

    #[test]
    fn test_world_starting_stats_returns_data() {
        let world = WorldState::new();
        let row = CreateNewCharValueRow {
            n_index: 1,
            class_type: 1,
            job_type: 0,
            level: 83,
            exp: 0,
            strength: 0,
            health: 0,
            dexterity: 0,
            intelligence: 0,
            magic_power: 0,
            free_points: 292,
            skill_point_free: 148,
            skill_point_cat1: 0,
            skill_point_cat2: 0,
            skill_point_cat3: 0,
            skill_point_master: 0,
            gold: 100000000,
        };
        world.insert_test_new_char_value((1, 0), row);
        let stats = world.get_starting_stats(1, 0);
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.level, 83);
        assert_eq!(s.gold, 100000000);
        assert_eq!(s.free_points, 292);
        assert_eq!(s.skill_point_free, 148);
    }

    #[test]
    fn test_world_starting_stats_different_job_types() {
        let world = WorldState::new();
        let base = CreateNewCharValueRow {
            n_index: 1,
            class_type: 2,
            job_type: 0,
            level: 83,
            exp: 0,
            strength: 0,
            health: 0,
            dexterity: 0,
            intelligence: 0,
            magic_power: 0,
            free_points: 292,
            skill_point_free: 148,
            skill_point_cat1: 0,
            skill_point_cat2: 0,
            skill_point_cat3: 0,
            skill_point_master: 0,
            gold: 100000000,
        };
        let first_change = CreateNewCharValueRow {
            n_index: 7,
            class_type: 2,
            job_type: 1,
            level: 59,
            exp: 0,
            strength: 0,
            health: 0,
            dexterity: 0,
            intelligence: 0,
            magic_power: 0,
            free_points: 0,
            skill_point_free: 0,
            skill_point_cat1: 0,
            skill_point_cat2: 0,
            skill_point_cat3: 0,
            skill_point_master: 0,
            gold: 1000000,
        };
        world.insert_test_new_char_value((2, 0), base);
        world.insert_test_new_char_value((2, 1), first_change);

        let base_stats = world.get_starting_stats(2, 0).unwrap();
        assert_eq!(base_stats.level, 83);
        assert_eq!(base_stats.gold, 100000000);

        let first_stats = world.get_starting_stats(2, 1).unwrap();
        assert_eq!(first_stats.level, 59);
        assert_eq!(first_stats.gold, 1000000);

        // Job type 2 not inserted, should be None
        assert!(world.get_starting_stats(2, 2).is_none());
    }

    #[test]
    fn test_world_starting_equipment_multiple_classes() {
        let world = WorldState::new();
        // Class 1 (Warrior)
        world.insert_test_new_char_set(
            1,
            CreateNewCharSetRow {
                id: 1,
                class_type: 1,
                slot_id: 0,
                item_id: 111111,
                item_duration: 1,
                item_count: 1,
                item_flag: 0,
                item_expire_time: 0,
            },
        );
        // Class 3 (Mage)
        world.insert_test_new_char_set(
            3,
            CreateNewCharSetRow {
                id: 151,
                class_type: 3,
                slot_id: 0,
                item_id: 333333,
                item_duration: 1,
                item_count: 1,
                item_flag: 0,
                item_expire_time: 0,
            },
        );

        let warrior_equip = world.get_starting_equipment(1);
        assert_eq!(warrior_equip.len(), 1);
        assert_eq!(warrior_equip[0].item_id, 111111);

        let mage_equip = world.get_starting_equipment(3);
        assert_eq!(mage_equip.len(), 1);
        assert_eq!(mage_equip[0].item_id, 333333);

        // Class 13 (Kurian) not inserted
        let kurian_equip = world.get_starting_equipment(13);
        assert!(kurian_equip.is_empty());
    }

    #[test]
    fn test_world_starting_equipment_preserves_slot_order() {
        let world = WorldState::new();
        let items = vec![
            CreateNewCharSetRow {
                id: 1,
                class_type: 4,
                slot_id: 0,
                item_id: 100,
                item_duration: 1,
                item_count: 1,
                item_flag: 0,
                item_expire_time: 0,
            },
            CreateNewCharSetRow {
                id: 2,
                class_type: 4,
                slot_id: 1,
                item_id: 200,
                item_duration: 2800,
                item_count: 1,
                item_flag: 0,
                item_expire_time: 0,
            },
            CreateNewCharSetRow {
                id: 3,
                class_type: 4,
                slot_id: 6,
                item_id: 300,
                item_duration: 14000,
                item_count: 1,
                item_flag: 0,
                item_expire_time: 0,
            },
        ];
        for item in items {
            world.insert_test_new_char_set(4, item);
        }
        let equip = world.get_starting_equipment(4);
        assert_eq!(equip.len(), 3);
        assert_eq!(equip[0].slot_id, 0);
        assert_eq!(equip[1].slot_id, 1);
        assert_eq!(equip[2].slot_id, 6);
    }

    #[test]
    fn test_create_new_char_set_row_fields() {
        let row = CreateNewCharSetRow {
            id: 42,
            class_type: 1,
            slot_id: 41,
            item_id: 1113329037,
            item_duration: 1,
            item_count: 1,
            item_flag: 0,
            item_expire_time: 0,
        };
        assert_eq!(row.id, 42);
        assert_eq!(row.class_type, 1);
        assert_eq!(row.slot_id, 41);
        assert_eq!(row.item_id, 1113329037);
    }

    #[test]
    fn test_create_new_char_value_row_fields() {
        let row = CreateNewCharValueRow {
            n_index: 25,
            class_type: 13,
            job_type: 4,
            level: 80,
            exp: 0,
            strength: 0,
            health: 0,
            dexterity: 0,
            intelligence: 0,
            magic_power: 0,
            free_points: 277,
            skill_point_free: 142,
            skill_point_cat1: 0,
            skill_point_cat2: 0,
            skill_point_cat3: 0,
            skill_point_master: 0,
            gold: 1000000000,
        };
        assert_eq!(row.n_index, 25);
        assert_eq!(row.class_type, 13);
        assert_eq!(row.job_type, 4);
        assert_eq!(row.level, 80);
        assert_eq!(row.gold, 1000000000);
        assert_eq!(row.free_points, 277);
        assert_eq!(row.skill_point_free, 142);
    }
}
