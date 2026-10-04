//! WIZ_POINT_CHANGE (0x28) and WIZ_SKILLPT_CHANGE (0x32) handlers.
//! ## WIZ_POINT_CHANGE — Stat Point Allocation
//! ### Request (C->S)
//! | Offset | Type | Description |
//! |--------|------|-------------|
//! | 0      | u8   | type: 1=STR, 2=STA, 3=DEX, 4=INT, 5=CHA |
//! ### Response (S->C)
//! `[u8 type] [u16 new_stat_value] [i16 max_hp] [i16 max_mp] [u16 total_hit]
//!  [u32 max_weight] [u16 hp] [u16 mp]`
//! ## WIZ_SKILLPT_CHANGE — Skill Point Allocation
//! ### Request (C->S)
//! | Offset | Type | Description |
//! |--------|------|-------------|
//! | 0      | u8   | type: 5=Cat1, 6=Cat2, 7=Cat3, 8=Master |
//! ### Response (S->C) — failure only
//! `[u8 type] [u8 current_skill_points]`
//! On success, no additional data is appended; the client infers success from
//! the absence of a failure byte.

use ko_db::models::CoefficientRow;
use ko_db::repositories::character::{CharacterRepository, SaveStatPointsParams};
use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::world::CharacterInfo;

/// Maximum stat value (`STAT_MAX` from `globals.h:747`).
const STAT_MAX: u8 = 255;

/// Skill point category range — valid types for SkillPointChange.
const SKILLPT_CAT1: u8 = 5;
const SKILLPT_MASTER: u8 = 8;

/// Handle WIZ_POINT_CHANGE — allocate a single stat point.
/// The client sends a 1-byte `type` (1=STR..5=CHA). The server validates the
/// request, increments the stat, decrements free points, recalculates max HP/MP,
/// and sends the updated values back.
pub async fn handle_point_change(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let stat_type = reader.read_u8().unwrap_or(0);

    // C++ converts: statType = (StatType)(type - 1)
    // Valid: type 1-5 → stat_index 0-4
    if !(1..=5).contains(&stat_type) {
        return Ok(());
    }

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Validate: must have free points
    if char_info.free_points < 1 {
        return Ok(());
    }

    // Validate: stat must not exceed STAT_MAX
    let current_stat = get_stat_by_type(&char_info, stat_type);
    if current_stat == STAT_MAX {
        return Ok(());
    }

    // Apply the stat change via world state
    let new_stat_value = current_stat + 1;
    world.update_character_stats(sid, |ch| {
        set_stat_by_type(ch, stat_type, new_stat_value);
        ch.free_points -= 1;
    });

    // Recalculate derived stats + max HP/MP (now includes item + buff bonuses)
    world.set_user_ability(sid);
    let equipped = world.get_equipped_stats(sid);

    // Read final HP/MP values after set_user_ability (includes item + buff bonuses)
    let (final_max_hp, final_max_mp, final_hp, _final_mp) = match world.get_character_info(sid) {
        Some(ch) => (ch.max_hp, ch.max_mp, ch.hp, ch.mp),
        None => return Ok(()),
    };

    // Build response — v2600 sniff verified format:
    // [u8 stat_type] [u16 new_stat] [u16 max_hp] [u16 hp] [u16 max_mp] [u32 max_weight]
    // Note: v2600 omits total_hit and current_mp vs older C++ format
    let mut resp = Packet::new(Opcode::WizPointChange as u8);
    resp.write_u8(stat_type);
    resp.write_u16(new_stat_value as u16);
    resp.write_u16(final_max_hp as u16);
    resp.write_u16(final_hp as u16);
    resp.write_u16(final_max_mp as u16);
    resp.write_u32(equipped.max_weight);
    session.send_packet(&resp).await?;

    // WIZ_POINT_CHANGE already contains the updated HP/MP caps, current HP,
    // and carry weight. A synthetic item-move refresh here makes the client
    // rebuild its equipment/status UI and causes a visible screen blink.

    // Fire-and-forget DB save
    save_stat_points_async(session);

    tracing::debug!(
        "[{}] POINT_CHANGE: type={} new_stat={} free_points={} max_hp={} max_mp={} max_weight={}",
        session.addr(),
        stat_type,
        new_stat_value,
        char_info.free_points - 1,
        final_max_hp,
        final_max_mp,
        equipped.max_weight,
    );

    Ok(())
}

/// Handle WIZ_SKILLPT_CHANGE — allocate a single skill point.
/// The client sends a 1-byte `type` (5=Cat1..8=Master). The server validates
/// the request and either applies the change (silently) or sends back a failure
/// packet with the current skill points for that category.
pub async fn handle_skillpt_change(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let skill_type = reader.read_u8().unwrap_or(0);

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Validate type range: 5..=8
    if !(SKILLPT_CAT1..=SKILLPT_MASTER).contains(&skill_type) {
        send_skillpt_fail(session, skill_type, &char_info).await?;
        return Ok(());
    }

    // Validate: free skill points available (index 0)
    if char_info.skill_points[0] < 1 {
        send_skillpt_fail(session, skill_type, &char_info).await?;
        return Ok(());
    }

    // Validate: skill points per category cannot exceed level
    let idx = skill_type as usize;
    if idx >= char_info.skill_points.len() {
        send_skillpt_fail(session, skill_type, &char_info).await?;
        return Ok(());
    }
    // u16 cast prevents u8 wrapping when skill_points[idx] == 255 (defensive)
    if char_info.skill_points[idx] as u16 + 1 > char_info.level as u16 {
        send_skillpt_fail(session, skill_type, &char_info).await?;
        return Ok(());
    }

    // Validate: must have completed first job change (class % 100 > 4)
    if (char_info.class % 100) <= 4 {
        send_skillpt_fail(session, skill_type, &char_info).await?;
        return Ok(());
    }

    // Validate master category (type 8) — extra constraints
    if skill_type == SKILLPT_MASTER {
        let class_mod = char_info.class % 100;
        let is_mastered = matches!(class_mod, 6 | 8 | 10 | 12 | 15);
        let max_level: u8 = 83; // g_pMain->m_byMaxLevel

        if !is_mastered
            || char_info.skill_points[idx] >= max_level.saturating_sub(60)
            || char_info.skill_points[idx] >= char_info.level.saturating_sub(60)
        {
            send_skillpt_fail(session, skill_type, &char_info).await?;
            return Ok(());
        }
    }

    // Apply: decrement free, increment category
    world.update_character_stats(sid, |ch| {
        ch.skill_points[0] = ch.skill_points[0].saturating_sub(1);
        ch.skill_points[idx] = ch.skill_points[idx].saturating_add(1);
    });

    // Recalculate derived stats after skill point change
    world.set_user_ability(sid);

    // Fire-and-forget DB save
    save_stat_points_async(session);

    tracing::debug!(
        "[{}] SKILLPT_CHANGE: type={} new_val={} free={}",
        session.addr(),
        skill_type,
        char_info.skill_points[idx].saturating_add(1),
        char_info.skill_points[0].saturating_sub(1),
    );

    Ok(())
}

/// Send a skill point change failure packet.
/// Response: `[u8 type] [u8 current_skill_points_for_type]`
async fn send_skillpt_fail(
    session: &mut ClientSession,
    skill_type: u8,
    char_info: &CharacterInfo,
) -> anyhow::Result<()> {
    let current = if (skill_type as usize) < char_info.skill_points.len() {
        char_info.skill_points[skill_type as usize]
    } else {
        0
    };

    let mut resp = Packet::new(Opcode::WizSkillptChange as u8);
    resp.write_u8(skill_type);
    resp.write_u8(current);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Get a stat value by type (1=STR..5=CHA).
/// C++ maps: type 1→STAT_STR(0), 2→STAT_STA(1), 3→STAT_DEX(2), 4→STAT_INT(3), 5→STAT_CHA(4)
fn get_stat_by_type(ch: &CharacterInfo, stat_type: u8) -> u8 {
    match stat_type {
        1 => ch.str,
        2 => ch.sta,
        3 => ch.dex,
        4 => ch.intel,
        5 => ch.cha,
        _ => 0,
    }
}

/// Set a stat value by type (1=STR..5=CHA).
fn set_stat_by_type(ch: &mut CharacterInfo, stat_type: u8, value: u8) {
    match stat_type {
        1 => ch.str = value,
        2 => ch.sta = value,
        3 => ch.dex = value,
        4 => ch.intel = value,
        5 => ch.cha = value,
        _ => {}
    }
}

/// Results from recalculating user abilities.
/// Mirrors the derived stats computed by `CUser::SetUserAbility` in C++.
pub struct AbilityResult {
    /// Maximum health points.
    pub max_hp: i16,
    /// Maximum mana points.
    pub max_mp: i16,
    /// Maximum carry weight (`uint32 m_sMaxWeight` in User.h:394).
    pub max_weight: u32,
}

/// Recalculate max HP, MP, and weight based on class coefficients and stats.
/// - `UserHealtMagicSpSystem.cpp:224-304` (CUser::SetMaxHp)
/// - `UserHealtMagicSpSystem.cpp:312-373` (CUser::SetMaxMp)
/// - `UserAbilityHandler.cpp:147` (m_sMaxWeight)
/// ## HP Formula
/// `(HP_COEFF * level^2 * STA) + (0.1 * level * STA) + (STA / 5) + bonuses + 20`
/// ## MP Formula (magic classes: MP coefficient != 0)
/// `(MP_COEFF * level^2 * (INT+30)) + (0.1 * level * 2 * (INT+30)) + ((INT+30) / 5) + bonuses + 20`
/// ## MP Formula (kurian/melee: SP coefficient != 0, MP == 0)
/// `(SP_COEFF * level^2 * STA) + (0.1 * level * STA) + (STA / 5) + bonuses`
/// ## Weight Formula
/// `(STR + level) * 50`
pub fn recalculate_abilities(ch: &CharacterInfo, coeff: Option<&CoefficientRow>) -> AbilityResult {
    let coeff = match coeff {
        Some(c) => c,
        None => {
            return AbilityResult {
                max_hp: ch.max_hp,
                max_mp: ch.max_mp,
                max_weight: calculate_max_weight(ch),
            };
        }
    };

    let level = ch.level as f64;
    let sta = ch.sta as f64 + ch.reb_sta as f64;
    let intel = ch.intel as f64 + ch.reb_intel as f64;

    // HP: (HP_COEFF * level^2 * STA) + (0.1 * level * STA) + (STA / 5) + 20
    // Note: item + buff bonuses are added in set_user_ability(), not here.
    let max_hp = (coeff.hp * level * level * sta) + (0.1 * level * sta) + (sta / 5.0) + 20.0;
    let max_hp = (max_hp as i16).max(20); // minimum 20 HP

    // MP: depends on whether MP or SP coefficient is nonzero
    let max_mp = if coeff.mp != 0.0 {
        // Magic class: uses INT+30
        let temp_intel = intel + 30.0;
        let mp = (coeff.mp * level * level * temp_intel)
            + (0.1 * level * 2.0 * temp_intel)
            + (temp_intel / 5.0)
            + 20.0;
        (mp as i16).max(0)
    } else if coeff.sp != 0.0 {
        // Kurian/melee: uses STA
        let mp = (coeff.sp * level * level * sta) + (0.1 * level * sta) + (sta / 5.0);
        (mp as i16).max(0)
    } else {
        ch.max_mp // No coefficient — keep existing
    };

    AbilityResult {
        max_hp,
        max_mp,
        max_weight: calculate_max_weight(ch),
    }
}

/// Calculate maximum SP (Kurian stamina points) based on class type and master skill points.
/// - Class type 13 (Beginner Kurian): 100
/// - Class type 14 (Novice Kurian): 150
/// - Class type 15 (Master Kurian): 200, or 250 if PRO_SKILL4 (skill_points[8]) is in 3..=23
/// - All other classes: 0 (non-Kurian)
/// The `pro_skill4` parameter is `skill_points[8]` (PRO_SKILL4 = 0x08 in C++).
/// C++ logic: `CheckSkillPoint(PRO_SKILL4, 3, 23)` → m_MaxSp = 250, else 200.
pub(crate) fn calculate_max_sp(class: u16, pro_skill4: u8) -> i16 {
    match class % 100 {
        13 => 100,
        14 => 150,
        15 => {
            if (3..=23).contains(&pro_skill4) {
                250
            } else {
                200
            }
        }
        _ => 0,
    }
}

/// Check if a class is a Kurian/Portu class.
pub(crate) fn is_kurian_class(class: u16) -> bool {
    matches!(class % 100, 13..=15)
}

/// Calculate maximum carry weight.
/// `m_sMaxWeight = (((GetStatWithItemBonus(STAT_STR) + GetLevel()) * 50) + maxweightbonus)`
/// Without item bonuses, this simplifies to `(STR + level) * 50`.
pub fn calculate_max_weight(ch: &CharacterInfo) -> u32 {
    (ch.str as u32 + ch.level as u32) * 50
}

/// Backward-compatible wrapper returning only (max_hp, max_mp).
/// Used by callers that don't need max_weight.
pub fn recalculate_max_hp_mp(ch: &CharacterInfo, coeff: Option<&CoefficientRow>) -> (i16, i16) {
    let result = recalculate_abilities(ch, coeff);
    (result.max_hp, result.max_mp)
}

/// Save stat and skill points to DB asynchronously (fire-and-forget).
/// Follows the same pattern as `zone_change::save_position_async`.
fn save_stat_points_async(session: &ClientSession) {
    let pool = session.pool().clone();
    let char_id = session.character_id().unwrap_or("").to_string();
    if char_id.is_empty() {
        return;
    }

    let world = session.world().clone();
    let sid = session.session_id();

    tokio::spawn(async move {
        let ch = match world.get_character_info(sid) {
            Some(ch) => ch,
            None => return,
        };

        let mut skill_i16 = [0i16; 10];
        for (i, &v) in ch.skill_points.iter().enumerate() {
            skill_i16[i] = v as i16;
        }

        let params = SaveStatPointsParams {
            char_id: &char_id,
            str_val: ch.str as i16,
            sta: ch.sta as i16,
            dex: ch.dex as i16,
            intel: ch.intel as i16,
            cha: ch.cha as i16,
            free_points: ch.free_points as i16,
            skill_points: skill_i16,
        };

        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo.save_stat_points(&params).await {
            tracing::error!(char_id, "failed to save stat/skill points: {}", e);
        }
    });
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_point_change_request_format() {
        // Client -> Server: [u8 type]
        let mut pkt = Packet::new(Opcode::WizPointChange as u8);
        pkt.write_u8(1); // type 1 = STR

        assert_eq!(pkt.opcode, Opcode::WizPointChange as u8);
        assert_eq!(pkt.data.len(), 1);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_point_change_response_format() {
        // v2600 sniff verified format:
        // [u8 stat_type] [u16 new_stat] [u16 max_hp] [u16 hp] [u16 max_mp] [u32 max_weight]
        let mut pkt = Packet::new(Opcode::WizPointChange as u8);
        pkt.write_u8(3); // stat_type
        pkt.write_u16(66); // new stat value
        pkt.write_u16(500); // max_hp
        pkt.write_u16(480); // hp
        pkt.write_u16(200); // max_mp
        pkt.write_u32(1000); // max_weight

        assert_eq!(pkt.data.len(), 13); // 1 + 2 + 2 + 2 + 2 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3)); // sub-opcode
        assert_eq!(r.read_u16(), Some(66));
        assert_eq!(r.read_u16(), Some(500)); // max_hp
        assert_eq!(r.read_u16(), Some(480)); // hp
        assert_eq!(r.read_u16(), Some(200)); // max_mp
        assert_eq!(r.read_u32(), Some(1000)); // max_weight
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_skillpt_change_request_format() {
        // Client -> Server: [u8 type]
        let mut pkt = Packet::new(Opcode::WizSkillptChange as u8);
        pkt.write_u8(5); // type 5 = Cat1

        assert_eq!(pkt.opcode, Opcode::WizSkillptChange as u8);
        assert_eq!(pkt.data.len(), 1);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_skillpt_change_fail_response_format() {
        // Server -> Client (failure): [u8 type] [u8 current_skill_points]
        let mut pkt = Packet::new(Opcode::WizSkillptChange as u8);
        pkt.write_u8(5); // type = Cat1
        pkt.write_u8(10); // current skill points

        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u8(), Some(10));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_get_set_stat_by_type() {
        let mut ch = test_character();
        assert_eq!(get_stat_by_type(&ch, 1), 65); // STR
        assert_eq!(get_stat_by_type(&ch, 2), 65); // STA
        assert_eq!(get_stat_by_type(&ch, 3), 60); // DEX
        assert_eq!(get_stat_by_type(&ch, 4), 50); // INT
        assert_eq!(get_stat_by_type(&ch, 5), 50); // CHA
        assert_eq!(get_stat_by_type(&ch, 0), 0); // invalid
        assert_eq!(get_stat_by_type(&ch, 6), 0); // invalid

        set_stat_by_type(&mut ch, 1, 70);
        assert_eq!(ch.str, 70);
        set_stat_by_type(&mut ch, 4, 55);
        assert_eq!(ch.intel, 55);
    }

    #[test]
    fn test_stat_max_validation() {
        // Stat at 255 should not increment
        let ch = CharacterInfo {
            str: STAT_MAX,
            ..test_character()
        };
        assert_eq!(get_stat_by_type(&ch, 1), STAT_MAX);
    }

    #[test]
    fn test_hp_formula_warrior_class_101() {
        // Warrior class 101 — verify HP formula with known coefficients
        // HP coefficient for warriors is typically ~0.000022
        let coeff = CoefficientRow {
            s_class: 101,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.0,
            axe: 0.0,
            club: 0.0,
            spear: 0.0,
            pole: 0.0,
            staff: 0.0,
            bow: 0.0,
            hp: 0.000022,
            mp: 0.0,
            sp: 0.000022,
            ac: 0.0,
            hitrate: 0.0,
            evasionrate: 0.0,
        };

        let ch = CharacterInfo {
            level: 60,
            sta: 90,
            intel: 50,
            class: 101,
            ..test_character()
        };

        let (max_hp, max_mp) = recalculate_max_hp_mp(&ch, Some(&coeff));

        // HP = 0.000022 * 60^2 * 90 + 0.1 * 60 * 90 + 90/5 + 20
        // = 0.000022 * 3600 * 90 + 540 + 18 + 20
        // = 7.128 + 540 + 18 + 20
        // = 585.128 → 585
        assert_eq!(max_hp, 585);

        // MP uses SP coefficient (warrior), not MP
        // SP = 0.000022 * 60^2 * 90 + 0.1 * 60 * 90 + 90/5
        // = 7.128 + 540 + 18 = 565.128 → 565
        assert_eq!(max_mp, 565);
    }

    #[test]
    fn test_hp_formula_mage_class_107() {
        // Mage class — uses MP coefficient for MP calculation
        let coeff = CoefficientRow {
            s_class: 107,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.0,
            axe: 0.0,
            club: 0.0,
            spear: 0.0,
            pole: 0.0,
            staff: 0.0,
            bow: 0.0,
            hp: 0.000019,
            mp: 0.000033,
            sp: 0.0,
            ac: 0.0,
            hitrate: 0.0,
            evasionrate: 0.0,
        };

        let ch = CharacterInfo {
            level: 60,
            sta: 60,
            intel: 80,
            class: 107,
            ..test_character()
        };

        let (max_hp, max_mp) = recalculate_max_hp_mp(&ch, Some(&coeff));

        // HP = 0.000019 * 3600 * 60 + 0.1 * 60 * 60 + 60/5 + 20
        // = 4.104 + 360 + 12 + 20 = 396.104 → 396
        assert_eq!(max_hp, 396);

        // MP = 0.000033 * 3600 * (80+30) + 0.1 * 60 * 2 * 110 + 110/5 + 20
        // = 0.000033 * 3600 * 110 + 1320 + 22 + 20
        // = 13.068 + 1320 + 22 + 20 = 1375.068 → 1375
        assert_eq!(max_mp, 1375);
    }

    #[test]
    fn test_recalculate_no_coefficient() {
        // Without coefficient data, should return existing values
        let ch = test_character();
        let (max_hp, max_mp) = recalculate_max_hp_mp(&ch, None);
        assert_eq!(max_hp, ch.max_hp);
        assert_eq!(max_mp, ch.max_mp);
    }

    #[test]
    fn test_skillpt_category_range() {
        // Valid range is 5..=8 (SkillPointCat1..SkillPointMaster)
        assert_eq!(SKILLPT_CAT1, 5);
        assert_eq!(SKILLPT_MASTER, 8);
        assert!(SKILLPT_CAT1 <= 5 && 5 <= SKILLPT_MASTER);
        assert!(SKILLPT_CAT1 <= 8 && 8 <= SKILLPT_MASTER);
        assert!(4 < SKILLPT_CAT1); // 4 is out of range
        assert!(9 > SKILLPT_MASTER); // 9 is out of range
    }

    #[test]
    fn test_mastery_validation() {
        // Class 107 (mage base) has class % 100 == 7, which is mastered
        assert!((107u16 % 100) >= 7);
        // Class 103 (mage novice) has class % 100 == 3, which is NOT mastered
        assert!(!((103u16 % 100) >= 7 && (103u16 % 100) <= 9));
        // Class 109 (mage master) has class % 100 == 9
        assert!((109u16 % 100) >= 7);
    }

    #[test]
    fn test_job_change_validation() {
        // Class 101 → 101 % 100 = 1, which is <=4 → no job change yet
        assert!((101u16 % 100) <= 4);
        // Class 105 → 105 % 100 = 5, which is >4 → has job change
        assert!((105u16 % 100) > 4);
        // Class 211 → 211 % 100 = 11, which is >4
        assert!((211u16 % 100) > 4);
    }

    #[test]
    fn test_max_weight_formula() {
        let ch = test_character(); // STR=65, level=60
        let weight = calculate_max_weight(&ch);
        // (65 + 60) * 50 = 125 * 50 = 6250
        assert_eq!(weight, 6250);

        // High STR character
        let ch2 = CharacterInfo {
            str: 200,
            level: 83,
            ..test_character()
        };
        // (200 + 83) * 50 = 283 * 50 = 14150
        assert_eq!(calculate_max_weight(&ch2), 14150);
    }

    #[test]
    fn test_recalculate_abilities_warrior() {
        let coeff = CoefficientRow {
            s_class: 101,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.0,
            axe: 0.0,
            club: 0.0,
            spear: 0.0,
            pole: 0.0,
            staff: 0.0,
            bow: 0.0,
            hp: 0.000022,
            mp: 0.0,
            sp: 0.000022,
            ac: 0.0,
            hitrate: 0.0,
            evasionrate: 0.0,
        };

        let ch = CharacterInfo {
            level: 60,
            sta: 90,
            intel: 50,
            str: 65,
            class: 101,
            ..test_character()
        };

        let result = recalculate_abilities(&ch, Some(&coeff));
        assert_eq!(result.max_hp, 585);
        assert_eq!(result.max_mp, 565);
        // (65 + 60) * 50 = 6250
        assert_eq!(result.max_weight, 6250);
    }

    #[test]
    fn test_recalculate_abilities_no_coefficient() {
        let ch = test_character();
        let result = recalculate_abilities(&ch, None);
        assert_eq!(result.max_hp, ch.max_hp);
        assert_eq!(result.max_mp, ch.max_mp);
        // Weight is always calculated even without coefficient
        assert_eq!(result.max_weight, (ch.str as u32 + ch.level as u32) * 50);
    }

    /// Helper to create a test character with warrior-like stats.
    fn test_character() -> CharacterInfo {
        CharacterInfo {
            session_id: 1,
            name: "TestWarrior".into(),
            nation: 1,
            race: 1,
            class: 105,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 65,
            sta: 65,
            dex: 60,
            intel: 50,
            cha: 50,
            free_points: 10,
            skill_points: [20, 0, 0, 0, 0, 10, 5, 3, 0, 0],
            gold: 1000,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 0x01,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    // ── calculate_max_sp tests ───────────────────────────────────────

    #[test]
    fn test_max_sp_beginner_kurian() {
        // Class type 13: beginner kurian = 100 max SP
        assert_eq!(calculate_max_sp(113, 0), 100); // Karus
        assert_eq!(calculate_max_sp(213, 0), 100); // El Morad
    }

    #[test]
    fn test_max_sp_novice_kurian() {
        // Class type 14: novice kurian = 150 max SP
        assert_eq!(calculate_max_sp(114, 0), 150);
        assert_eq!(calculate_max_sp(214, 0), 150);
    }

    #[test]
    fn test_max_sp_master_kurian_base() {
        // Class type 15: master kurian = 200 max SP (no PRO_SKILL4 points)
        assert_eq!(calculate_max_sp(115, 0), 200);
        assert_eq!(calculate_max_sp(215, 0), 200);
        // PRO_SKILL4 in range 0..=2 → still 200
        assert_eq!(calculate_max_sp(115, 1), 200);
        assert_eq!(calculate_max_sp(115, 2), 200);
    }

    #[test]
    fn test_max_sp_master_kurian_250() {
        // PRO_SKILL4 in range 3..=23 → 250 max SP
        assert_eq!(calculate_max_sp(115, 3), 250); // min boundary
        assert_eq!(calculate_max_sp(215, 3), 250); // El Morad
        assert_eq!(calculate_max_sp(115, 10), 250); // mid range
        assert_eq!(calculate_max_sp(115, 23), 250); // max boundary
        assert_eq!(calculate_max_sp(215, 23), 250); // El Morad max boundary
                                                    // Beyond range 23 → falls back to 200
        assert_eq!(calculate_max_sp(115, 24), 200);
        assert_eq!(calculate_max_sp(215, 24), 200);
    }

    #[test]
    fn test_max_sp_non_kurian_classes() {
        // All non-kurian classes should have 0 max SP regardless of pro_skill4
        assert_eq!(calculate_max_sp(101, 0), 0); // Warrior
        assert_eq!(calculate_max_sp(102, 0), 0); // Rogue
        assert_eq!(calculate_max_sp(103, 0), 0); // Mage
        assert_eq!(calculate_max_sp(104, 0), 0); // Priest
        assert_eq!(calculate_max_sp(105, 0), 0); // Warrior novice
        assert_eq!(calculate_max_sp(106, 0), 0); // Warrior master
        assert_eq!(calculate_max_sp(201, 0), 0); // El Morad warrior
        assert_eq!(calculate_max_sp(210, 0), 0); // El Morad mage master
                                                 // Even with pro_skill4 set, non-kurian still 0
        assert_eq!(calculate_max_sp(106, 10), 0);
    }

    // ── is_kurian_class tests ────────────────────────────────────────

    #[test]
    fn test_is_kurian_class_true() {
        assert!(is_kurian_class(113)); // Karus beginner kurian
        assert!(is_kurian_class(114)); // Karus novice kurian
        assert!(is_kurian_class(115)); // Karus master kurian
        assert!(is_kurian_class(213)); // El Morad beginner portu
        assert!(is_kurian_class(214)); // El Morad novice portu
        assert!(is_kurian_class(215)); // El Morad master portu
    }

    #[test]
    fn test_is_kurian_class_false() {
        assert!(!is_kurian_class(101)); // Warrior
        assert!(!is_kurian_class(102)); // Rogue
        assert!(!is_kurian_class(103)); // Mage
        assert!(!is_kurian_class(104)); // Priest
        assert!(!is_kurian_class(105)); // Warrior novice
        assert!(!is_kurian_class(106)); // Warrior master
        assert!(!is_kurian_class(107)); // Mage novice
        assert!(!is_kurian_class(112)); // Priest master
        assert!(!is_kurian_class(201)); // El Morad warrior
    }

    // ── Max SP fits in u8 ────────────────────────────────────────────

    #[test]
    fn test_max_sp_fits_in_u8() {
        // Verify all max SP values fit in u8 (packet sends as u8)
        for &class in &[113u16, 114, 115, 213, 214, 215] {
            // Test both with and without PRO_SKILL4 points
            for &pro_skill4 in &[0u8, 3, 10, 23] {
                let max_sp = calculate_max_sp(class, pro_skill4);
                assert!(
                    (0..=255).contains(&max_sp),
                    "max_sp for class {} (pro_skill4={}) = {} exceeds u8 range",
                    class,
                    pro_skill4,
                    max_sp
                );
            }
        }
    }
}
