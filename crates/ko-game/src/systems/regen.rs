//! HP/MP regeneration tick system.
//! Runs every 5 seconds (`m_bHPIntervalNormal = 5`), iterating all
//! in-game sessions and applying HP/MP regen based on sit/stand state.
//! ## Regen Rules (from C++)
//! - **Snow Battle zone (69)**: `HpChange(5)` flat, return early
//! - **Standing**: MP regen only
//!   - `((level * (1 + level/60) + 1) * 0.2) + 3`
//!   - Mages under 30% MP get 120% MP regen
//! - **Sitting (normal player)**:
//!   - HP: `level * (1 + level/30) + 3`
//!   - MP: `((maxMp * 5) / ((level - 1) + 30)) + 3`
//!   - Mages under 30% MP get 120% MP regen
//! - **Dead**: no regen

use std::sync::Arc;
use std::time::Duration;

use ko_protocol::{Opcode, Packet};

use crate::world::types::{ZONE_PRISON, ZONE_SNOW_BATTLE};
use crate::world::{RegenData, WorldState, USER_DEAD, USER_SITDOWN, USER_STANDING};

/// Regen tick interval in seconds
const REGEN_INTERVAL_SECS: u64 = 5;

/// Start the HP/MP regeneration background task.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_regen_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(REGEN_INTERVAL_SECS));
        loop {
            interval.tick().await;
            process_regen_tick(&world);
        }
    })
}

/// Process one regen tick for all in-game sessions.
fn process_regen_tick(world: &Arc<WorldState>) {
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let data = world.collect_regen_data();
    for rd in data {
        process_session_regen(world, &rd);
        // Training mode: sitting players get periodic XP
        if rd.res_hp_type == USER_SITDOWN {
            training_process(world, &rd, now_unix);
        }
    }
}

/// Check if a class is a mage class (110/210 range).
fn is_mage_class(class: u16) -> bool {
    (110..=115).contains(&class) || (210..=215).contains(&class)
}

/// Calculate the MP regen percent multiplier.
/// ```text
/// if (CheckClass(110, 210) && m_sMp < (30 * m_MaxMp / 100))
///     mpPercent = 120;
/// ```
/// Returns 120 for mages below 30% MP, 100 otherwise.
fn mp_percent(class: u16, mp: i16, max_mp: i16) -> i32 {
    // Character MP fields are i16. 2615 characters can legitimately have
    // enough MP for `30 * max_mp` to overflow i16 in debug builds, killing
    // the entire background regeneration task. Promote before arithmetic.
    if is_mage_class(class) && max_mp > 0 && (mp as i32) < (30 * max_mp as i32 / 100) {
        120
    } else {
        100
    }
}

/// Apply regen logic for a single session.
fn process_session_regen(world: &WorldState, rd: &RegenData) {
    // Dead players don't regen
    if rd.res_hp_type == USER_DEAD || rd.hp <= 0 {
        return;
    }

    //   if (!isBlinking()) { HPTimeChange(); SpTimeChange(); }
    // Blinking players do NOT receive normal HP/MP regen.
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if rd.blink_expiry_time > 0 && now_unix < rd.blink_expiry_time {
        return;
    }

    let level = rd.level as f64;
    let mut hp_change: i32 = 0;
    let mut mp_change: i32 = 0;

    if rd.zone_id == ZONE_SNOW_BATTLE {
        if rd.hp < rd.max_hp {
            hp_change = 5;
        }
        // Apply HP change for Snow Battle and return
        if hp_change > 0 {
            let new_hp = (rd.hp as i32 + hp_change).min(rd.max_hp as i32) as i16;
            if new_hp != rd.hp {
                world.update_session_hp(rd.session_id, new_hp);
                let pkt = build_hp_change_packet(rd.max_hp, new_hp);
                world.send_to_session_owned(rd.session_id, pkt);
            }
        }
        return;
    }

    let mp_pct = mp_percent(rd.class, rd.mp, rd.max_mp);

    match rd.res_hp_type {
        USER_STANDING => {
            // Standing: MP regen only
            if rd.mp < rd.max_mp {
                let base = ((level * (1.0 + level / 60.0) + 1.0) * 0.2 + 3.0) as i32;
                mp_change = base * mp_pct / 100;
            }
        }
        USER_SITDOWN => {
            // Sitting players, including GMs, use the normal gradual regen
            // formula. Authority must not change combat-resource timing.
            if rd.hp < rd.max_hp {
                hp_change = (level * (1.0 + level / 30.0)) as i32 + 3;
            }
            //   if (GetZoneID() == ZONE_PRISON && GetLevel() > 1)
            //       MSpChange(+(m_MaxMp * 5 / 100));
            //   else normal formula
            if rd.mp < rd.max_mp {
                if rd.zone_id == ZONE_PRISON && rd.level > 1 {
                    mp_change = (rd.max_mp as i32) * 5 / 100;
                } else {
                    let divisor = (rd.level as i32 - 1).max(0) + 30;
                    let base = ((rd.max_mp as i32) * 5 / divisor) + 3;
                    mp_change = base * mp_pct / 100;
                }
            }
        }
        _ => {
            // Other states (mining, flashing, etc.) — no regen for now
        }
    }

    // Apply HP change (undead: regen becomes damage)
    if hp_change > 0 {
        let effective = if rd.is_undead { -hp_change } else { hp_change };
        let new_hp = if effective > 0 {
            (rd.hp as i32 + effective).min(rd.max_hp as i32) as i16
        } else {
            (rd.hp as i32 + effective).max(0) as i16
        };
        if new_hp != rd.hp {
            world.update_session_hp(rd.session_id, new_hp);
            let pkt = build_hp_change_packet(rd.max_hp, new_hp);
            world.send_to_session_owned(rd.session_id, pkt);
            crate::handler::party::broadcast_party_hp(world, rd.session_id);
        }
    }

    // Apply MP change
    if mp_change > 0 {
        let new_mp = (rd.mp as i32 + mp_change).min(rd.max_mp as i32) as i16;
        if new_mp != rd.mp {
            world.update_session_mp(rd.session_id, new_mp);
            let pkt = build_mp_change_packet(rd.max_mp, new_mp);
            world.send_to_session_owned(rd.session_id, pkt);
        }
    }
}

/// Build a WIZ_HP_CHANGE (0x17) packet with a specific attacker ID.
/// ```text
/// result << m_MaxHp << m_sHp << uint32(tid);
/// ```
/// Wire format: `[i16 max_hp] [i16 current_hp] [u32 attacker_id]`
pub fn build_hp_change_packet_with_attacker(
    max_hp: i16,
    current_hp: i16,
    attacker_id: u32,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizHpChange as u8);
    pkt.write_i16(max_hp);
    pkt.write_i16(current_hp);
    pkt.write_u32(attacker_id);
    pkt
}

/// Build a WIZ_HP_CHANGE (0x17) packet.
/// ```text
/// result << m_MaxHp << m_sHp << uint32(tid);
/// ```
/// Wire format: `[i16 max_hp] [i16 current_hp] [u32 attacker_id]`
/// When regen (no attacker), attacker_id = 0xFFFFFFFF (-1).
/// v2600 PCAP verified: original server sends 0xFFFFFFFF (not 0x0000FFFF).
pub fn build_hp_change_packet(max_hp: i16, current_hp: i16) -> Packet {
    build_hp_change_packet_with_attacker(max_hp, current_hp, 0xFFFF_FFFF)
}

/// Build a WIZ_MSP_CHANGE (0x18) packet.
/// ```text
/// result << m_MaxMp << m_sMp;
/// ```
/// Wire format: `[i16 max_mp] [i16 current_mp]`
pub fn build_mp_change_packet(max_mp: i16, current_mp: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMspChange as u8);
    pkt.write_i16(max_mp);
    pkt.write_i16(current_mp);
    pkt
}

/// Calculate standing MP regen amount (before mage percent).
/// C++ formula: `((level * (1 + level / 60.0) + 1) * 0.2) + 3`
pub fn calc_standing_mp_regen(level: u8) -> i32 {
    let lvl = level as f64;
    ((lvl * (1.0 + lvl / 60.0) + 1.0) * 0.2 + 3.0) as i32
}

/// Calculate sitting HP regen amount.
/// C++ formula: `level * (1 + level / 30.0) + 3`
pub fn calc_sitting_hp_regen(level: u8) -> i32 {
    let lvl = level as f64;
    (lvl * (1.0 + lvl / 30.0)) as i32 + 3
}

/// Calculate sitting MP regen amount (before mage percent).
/// C++ formula: `((maxMp * 5) / ((level - 1) + 30)) + 3`
pub fn calc_sitting_mp_regen(level: u8, max_mp: i16) -> i32 {
    let divisor = (level as i32 - 1).max(0) + 30;
    ((max_mp as i32) * 5 / divisor) + 3
}

/// Training interval in seconds.
const PLAYER_TRAINING_INTERVAL: u64 = 15;

/// Minimum level for training mode XP.
const TRAINING_MIN_LEVEL: u8 = 10;

/// Calculate training XP reward for a given level.
fn training_xp_for_level(level: u8) -> u32 {
    match level {
        10..=20 => 50,
        21..=40 => 200,
        41..=60 => 1000,
        61..=70 => 2000,
        71..=80 => 2500,
        _ if level > 80 => 5000,
        _ => 0,
    }
}

/// Process training mode for a sitting player.
/// When a player is sitting (USER_SITDOWN) and level >= 10, they receive
/// periodic XP rewards every 15 seconds. A counter (`m_iTotalTrainingExp`)
/// accumulates the total training XP earned and is sent to the client
/// via WIZ_MINING packet with sub-opcodes (18, 3).
fn training_process(world: &Arc<WorldState>, rd: &RegenData, now_unix: u64) {
    if rd.level < TRAINING_MIN_LEVEL {
        return;
    }

    let last_time = rd.last_training_time;
    let total_exp = rd.total_training_exp;

    // Initialize timer on first call
    if last_time == 0 {
        tracing::debug!(
            "[sid={}] Training init: level={}, setting last_training_time={}",
            rd.session_id,
            rd.level,
            now_unix,
        );
        world.update_session(rd.session_id, |h| {
            h.last_training_time = now_unix;
        });
        return;
    }

    // Check interval
    if last_time + PLAYER_TRAINING_INTERVAL > now_unix {
        return;
    }

    let xp = training_xp_for_level(rd.level);
    let new_total = total_exp + xp;

    // Single update: timer + exp accumulation in one lock
    world.update_session(rd.session_id, |h| {
        h.last_training_time = now_unix;
        if xp > 0 {
            h.total_training_exp = new_total;
        }
    });

    if xp == 0 {
        return;
    }

    // Give XP via async task (handles bonuses + level-up + DB save)
    let w = Arc::clone(world);
    let sid = rd.session_id;
    tokio::spawn(async move {
        crate::handler::level::exp_change_with_bonus(&w, sid, xp as i64, true).await;
    });

    // v2600 PCAP verified: training counter uses WIZ_MINING (0x86), sub=0x12, sub2=0x03.
    // Original server sends [0x86][0x12][0x03][total_training_exp:u32le] every 15s.
    // Counter increments by 50 per tick (50, 100, 150, ...).
    {
        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizMining as u8);
        pkt.write_u8(0x12); // sub: training counter (18 decimal)
        pkt.write_u8(0x03); // sub2: counter update
        pkt.write_u32(new_total);
        world.send_to_session_owned(rd.session_id, pkt);
    }

    tracing::debug!(
        "[sid={}] Training tick: level={}, xp={}, new_total={}",
        rd.session_id,
        rd.level,
        xp,
        new_total,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Regen formula tests ─────────────────────────────────────────

    #[test]
    fn test_standing_mp_regen_level_1() {
        // level=1: ((1 * (1 + 1/60) + 1) * 0.2) + 3
        // = ((1 * 1.0167 + 1) * 0.2) + 3
        // = (2.0167 * 0.2) + 3
        // = 0.4033 + 3 = 3.4033 -> truncated to 3
        let mp = calc_standing_mp_regen(1);
        assert_eq!(mp, 3);
    }

    #[test]
    fn test_standing_mp_regen_level_60() {
        // level=60: ((60 * (1 + 60/60) + 1) * 0.2) + 3
        // = ((60 * 2 + 1) * 0.2) + 3
        // = (121 * 0.2) + 3
        // = 24.2 + 3 = 27.2 -> 27
        let mp = calc_standing_mp_regen(60);
        assert_eq!(mp, 27);
    }

    #[test]
    fn test_standing_mp_regen_level_83() {
        // level=83: ((83 * (1 + 83/60) + 1) * 0.2) + 3
        // = ((83 * 2.383 + 1) * 0.2) + 3
        // = ((197.833 + 1) * 0.2) + 3
        // = (198.833 * 0.2) + 3
        // = 39.766 + 3 = 42.766 -> 42
        let mp = calc_standing_mp_regen(83);
        assert_eq!(mp, 42);
    }

    #[test]
    fn test_sitting_hp_regen_level_1() {
        // level=1: (1 * (1 + 1/30)) + 3 = (1 * 1.033) + 3 = 1.033 -> 1 + 3 = 4
        let hp = calc_sitting_hp_regen(1);
        assert_eq!(hp, 4);
    }

    #[test]
    fn test_sitting_hp_regen_level_60() {
        // level=60: (60 * (1 + 60/30)) + 3 = (60 * 3) + 3 = 180 + 3 = 183
        let hp = calc_sitting_hp_regen(60);
        assert_eq!(hp, 183);
    }

    #[test]
    fn test_sitting_hp_regen_level_83() {
        // level=83: (83 * (1 + 83/30)) + 3 = (83 * 3.766) + 3 = 312.611 -> 312 + 3 = 315
        let hp = calc_sitting_hp_regen(83);
        assert_eq!(hp, 315);
    }

    #[test]
    fn test_sitting_mp_regen_level_60_max_mp_500() {
        // level=60, maxMp=500: ((500 * 5) / ((60-1) + 30)) + 3
        // = (2500 / 89) + 3 = 28 + 3 = 31
        let mp = calc_sitting_mp_regen(60, 500);
        assert_eq!(mp, 31);
    }

    #[test]
    fn test_sitting_mp_regen_level_1_max_mp_100() {
        // level=1, maxMp=100: ((100 * 5) / ((1-1) + 30)) + 3
        // = (500 / 30) + 3 = 16 + 3 = 19
        let mp = calc_sitting_mp_regen(1, 100);
        assert_eq!(mp, 19);
    }

    // ── Mage MP percent tests ───────────────────────────────────────

    #[test]
    fn test_mp_percent_non_mage() {
        // Warrior class 101 should always get 100%
        assert_eq!(mp_percent(101, 50, 500), 100);
        assert_eq!(mp_percent(101, 10, 500), 100);
    }

    #[test]
    fn test_mp_percent_mage_above_30() {
        // Mage at 50% MP -> 100%
        assert_eq!(mp_percent(110, 250, 500), 100);
    }

    #[test]
    fn test_mp_percent_mage_below_30() {
        // Mage at 20% MP -> 120%
        assert_eq!(mp_percent(110, 100, 500), 120);
        // El Morad mage at 10% -> 120%
        assert_eq!(mp_percent(210, 50, 500), 120);
    }

    #[test]
    fn test_mp_percent_mage_at_boundary() {
        // Mage at exactly 30% -> not below, so 100%
        assert_eq!(mp_percent(110, 150, 500), 100);
        // Mage at 29% (149/500) -> below 30%, so 120%
        assert_eq!(mp_percent(110, 149, 500), 120);
    }

    #[test]
    fn test_mp_percent_high_mp_does_not_overflow() {
        assert_eq!(mp_percent(110, 800, 3_000), 120);
        assert_eq!(mp_percent(110, 900, 3_000), 100);
    }

    #[test]
    fn test_is_mage_class() {
        assert!(is_mage_class(110));
        assert!(is_mage_class(115));
        assert!(is_mage_class(210));
        assert!(is_mage_class(215));
        assert!(!is_mage_class(101));
        assert!(!is_mage_class(201));
        assert!(!is_mage_class(116));
    }

    // ── ZONE_PRISON MP regen tests ──────────────────────────────────

    #[test]
    fn test_zone_prison_constant() {
        assert_eq!(ZONE_PRISON, 92);
    }

    #[test]
    fn test_prison_mp_regen_formula() {
        // MSpChange(+(m_MaxMp * 5 / 100))
        // 5% of max MP for sitting in prison
        let max_mp: i16 = 500;
        let prison_regen = (max_mp as i32) * 5 / 100;
        assert_eq!(prison_regen, 25);

        let max_mp2: i16 = 1000;
        let prison_regen2 = (max_mp2 as i32) * 5 / 100;
        assert_eq!(prison_regen2, 50);
    }

    #[test]
    fn test_prison_mp_regen_small_mp() {
        // Small MP pool: 19 * 5 / 100 = 0
        let max_mp: i16 = 19;
        let prison_regen = (max_mp as i32) * 5 / 100;
        assert_eq!(prison_regen, 0);

        // 20 * 5 / 100 = 1
        let max_mp2: i16 = 20;
        let prison_regen2 = (max_mp2 as i32) * 5 / 100;
        assert_eq!(prison_regen2, 1);
    }

    // ── Blink regen guard tests ─────────────────────────────────────

    #[test]
    fn test_blink_guard_skips_regen() {
        // A blinking player should NOT receive HP/MP regen.
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let rd = RegenData {
            session_id: 1,
            level: 60,
            hp: 500,
            max_hp: 1000,
            mp: 200,
            max_mp: 500,
            res_hp_type: USER_SITDOWN,
            authority: 1,
            zone_id: 21, // Moradon
            class: 101,
            sp: 0,
            max_sp: 0,
            pro_skill4: 0,
            blink_expiry_time: now_unix + 10, // blinking for 10 more seconds
            is_undead: false,
            last_training_time: 0,
            total_training_exp: 0,
        };

        // Verify the blink guard condition matches
        assert!(rd.blink_expiry_time > 0 && now_unix < rd.blink_expiry_time);
    }

    #[test]
    fn test_no_blink_allows_regen() {
        // When blink_expiry_time is 0 (not blinking), regen should proceed.
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let rd = RegenData {
            session_id: 1,
            level: 60,
            hp: 500,
            max_hp: 1000,
            mp: 200,
            max_mp: 500,
            res_hp_type: USER_SITDOWN,
            authority: 1,
            zone_id: 21,
            class: 101,
            sp: 0,
            max_sp: 0,
            pro_skill4: 0,
            blink_expiry_time: 0, // not blinking
            is_undead: false,
            last_training_time: 0,
            total_training_exp: 0,
        };

        // Verify blink guard does NOT trigger
        assert!(!(rd.blink_expiry_time > 0 && now_unix < rd.blink_expiry_time));
    }

    #[test]
    fn test_expired_blink_allows_regen() {
        // When blink has expired, regen should proceed.
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let rd = RegenData {
            session_id: 1,
            level: 60,
            hp: 500,
            max_hp: 1000,
            mp: 200,
            max_mp: 500,
            res_hp_type: USER_SITDOWN,
            authority: 1,
            zone_id: 21,
            class: 101,
            sp: 0,
            max_sp: 0,
            pro_skill4: 0,
            blink_expiry_time: now_unix - 5, // expired 5 seconds ago
            is_undead: false,
            last_training_time: 0,
            total_training_exp: 0,
        };

        // Verify blink guard does NOT trigger for expired blink
        assert!(!(rd.blink_expiry_time > 0 && now_unix < rd.blink_expiry_time));
    }

    // ── Packet format tests ─────────────────────────────────────────

    #[test]
    fn test_hp_change_packet_format() {
        let pkt = build_hp_change_packet(1000, 500);
        assert_eq!(pkt.opcode, Opcode::WizHpChange as u8);
        assert_eq!(pkt.data.len(), 8); // i16 + i16 + u32 = 2 + 2 + 4
                                       // max_hp = 1000 (0x03E8) little-endian
        assert_eq!(pkt.data[0], 0xE8);
        assert_eq!(pkt.data[1], 0x03);
        // current_hp = 500 (0x01F4) little-endian
        assert_eq!(pkt.data[2], 0xF4);
        assert_eq!(pkt.data[3], 0x01);
        // attacker_id = 0xFFFFFFFF (-1, v2600 PCAP verified)
        assert_eq!(pkt.data[4], 0xFF);
        assert_eq!(pkt.data[5], 0xFF);
        assert_eq!(pkt.data[6], 0xFF);
        assert_eq!(pkt.data[7], 0xFF);
    }

    #[test]
    fn test_mp_change_packet_format() {
        let pkt = build_mp_change_packet(800, 300);
        assert_eq!(pkt.opcode, Opcode::WizMspChange as u8);
        assert_eq!(pkt.data.len(), 4); // i16 + i16 = 2 + 2
                                       // max_mp = 800 (0x0320) little-endian
        assert_eq!(pkt.data[0], 0x20);
        assert_eq!(pkt.data[1], 0x03);
        // current_mp = 300 (0x012C) little-endian
        assert_eq!(pkt.data[2], 0x2C);
        assert_eq!(pkt.data[3], 0x01);
    }

    #[test]
    fn test_hp_change_packet_roundtrip() {
        use ko_protocol::PacketReader;
        let pkt = build_hp_change_packet(5000, 2500);
        let mut reader = PacketReader::new(&pkt.data);
        let max_hp = reader.read_u16().map(|v| v as i16).unwrap();
        let current_hp = reader.read_u16().map(|v| v as i16).unwrap();
        let attacker_id = reader.read_u32().unwrap();
        assert_eq!(max_hp, 5000);
        assert_eq!(current_hp, 2500);
        assert_eq!(attacker_id, 0xFFFF_FFFF);
    }

    #[test]
    fn test_mp_change_packet_roundtrip() {
        use ko_protocol::PacketReader;
        let pkt = build_mp_change_packet(3000, 1500);
        let mut reader = PacketReader::new(&pkt.data);
        let max_mp = reader.read_u16().map(|v| v as i16).unwrap();
        let current_mp = reader.read_u16().map(|v| v as i16).unwrap();
        assert_eq!(max_mp, 3000);
        assert_eq!(current_mp, 1500);
    }

    #[test]
    fn test_hp_change_packet_with_attacker_roundtrip() {
        use ko_protocol::PacketReader;
        let pkt = build_hp_change_packet_with_attacker(2000, 750, 42);
        assert_eq!(pkt.opcode, Opcode::WizHpChange as u8);
        assert_eq!(pkt.data.len(), 8);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u16().map(|v| v as i16).unwrap(), 2000);
        assert_eq!(reader.read_u16().map(|v| v as i16).unwrap(), 750);
        assert_eq!(reader.read_u32().unwrap(), 42);
    }

    // ── Training mode XP tests ──────────────────────────────────────

    #[test]
    fn test_training_xp_by_level() {
        assert_eq!(training_xp_for_level(5), 0); // below 10
        assert_eq!(training_xp_for_level(9), 0); // below 10
        assert_eq!(training_xp_for_level(10), 50); // 10-20
        assert_eq!(training_xp_for_level(15), 50);
        assert_eq!(training_xp_for_level(20), 50);
        assert_eq!(training_xp_for_level(21), 200); // 21-40
        assert_eq!(training_xp_for_level(30), 200);
        assert_eq!(training_xp_for_level(40), 200);
        assert_eq!(training_xp_for_level(41), 1000); // 41-60
        assert_eq!(training_xp_for_level(50), 1000);
        assert_eq!(training_xp_for_level(60), 1000);
        assert_eq!(training_xp_for_level(61), 2000); // 61-70
        assert_eq!(training_xp_for_level(70), 2000);
        assert_eq!(training_xp_for_level(71), 2500); // 71-80
        assert_eq!(training_xp_for_level(80), 2500);
        assert_eq!(training_xp_for_level(81), 5000); // 80+
        assert_eq!(training_xp_for_level(83), 5000);
    }

    #[test]
    fn test_training_interval() {
        assert_eq!(PLAYER_TRAINING_INTERVAL, 15);
    }

    #[test]
    fn test_training_min_level() {
        assert_eq!(TRAINING_MIN_LEVEL, 10);
    }

    // NOTE: Training packet format test removed — v2525 client drops WIZ_MINING sub=18
    // (jump table only handles sub 1-9). CUITraining has zero references in v2525 binary.
    // Server-side training XP grant via exp_change_with_bonus still works.
}
