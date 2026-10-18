//! Flash Time & Burning Time systems — timed XP/DC/War bonuses.
//! - `CharacterSelectionHandler.cpp:933-977` — `SetFlashTimeNote()`
//! - `User.cpp:1020-1024` — `FlashUpdateTime()` (1-minute tick)
//! - `User.cpp:1158-1172` — `BurningTime()` (hourly flame progression)
//! - `UserLevelExperienceSystem.cpp:443-534` — `ExpFlash/DcFlash/WarFlash/SendFlashNotice`
//! - `PremiumSystem.cpp:32` — flash reset on premium switch
//! ## Flash System Overview
//! Flash items add one stack per successful use, not per monster kill.
//! EXP/DC add 10% up to 100%; WAR adds 1 loyalty up to 10.
//! The flash timer counts down in 1-minute intervals; when it hits 0 the bonus is removed.
//! ## Burning / Flame System
//! Flame level (0-3) increments once per hour while online. Each level adds
//! multipliers from the BURNING_FEATURES table (XP, drop, NP, money bonuses).
//! Flame level persists across short relogs (5-minute grace window in C++).

use ko_protocol::{Opcode, Packet};

use crate::systems::buff_tick::build_buff_expired_packet;
use crate::world::WorldState;
use crate::zone::SessionId;

use crate::buff_constants::BUFF_TYPE_FISHING;

/// Flash type constants matching `m_flashtype` values.
pub const FLASH_TYPE_NONE: u8 = 0;
/// Flash type: EXP bonus (premium 11 or 7).
pub const FLASH_TYPE_EXP: u8 = 1;
/// Flash type: DC/Drop bonus (premium 10 or 7).
pub const FLASH_TYPE_DC: u8 = 2;
/// Flash type: WAR/Loyalty bonus (premium 12 or 7).
pub const FLASH_TYPE_WAR: u8 = 3;

/// Maximum flash stack count.
const MAX_FLASH_COUNT: u8 = 10;

/// Maximum flash EXP/DC bonus percentage.
const MAX_FLASH_EXP_DC_BONUS: u8 = 100;

/// Maximum flash WAR bonus value.
const MAX_FLASH_WAR_BONUS: u8 = 10;

/// Flash check interval: 1 minute (60 seconds).
const PLAYER_FLASH_INTERVAL: u64 = 60;

/// Burning time interval: 1 hour (3600 seconds).
const BURNING_HOUR: u64 = 3600;

/// Maximum flame level.
const MAX_FLAME_LEVEL: u16 = 3;

/// Check if a premium type supports flash bonuses.
/// `bool validprem = GetPremium() >= 10 && GetPremium() <= 12 || GetPremium() == 7;`
pub fn is_flash_premium(premium_type: u8) -> bool {
    (10..=12).contains(&premium_type) || premium_type == 7
}

/// Validate before consuming the item. Recasts do not enter this path.
pub fn use_flash(world: &WorldState, sid: SessionId, kind: i32) -> bool {
    let allowed = world
        .with_session(sid, |h| match kind {
            1 => matches!(h.premium_in_use, 7 | 10) && h.flash_dc_bonus < 100,
            2 => matches!(h.premium_in_use, 7 | 11) && h.flash_exp_bonus < 100,
            3 => matches!(h.premium_in_use, 7 | 12) && h.flash_war_bonus < 10,
            _ => false,
        })
        .unwrap_or(false);
    if !allowed {
        return false;
    }
    match kind {
        1 => dc_flash(world, sid),
        2 => exp_flash(world, sid),
        3 => war_flash(world, sid),
        _ => return false,
    }
    // Set count from the actual bonus, including when changing flash type.
    world.update_session(sid, |h| {
        h.flash_count = match kind {
            1 => h.flash_dc_bonus / 10,
            2 => h.flash_exp_bonus / 10,
            _ => h.flash_war_bonus,
        };
        h.flash_check_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    });
    true
}

/// Apply flash XP bonus on item use (BUFF_TYPE_FISHING with sSpecialAmount == 2).
/// Increments flash_exp_bonus by 10 (max 100), resets DC/WAR bonuses,
/// updates flash_time and flash_count, sets flash_type to EXP.
pub fn exp_flash(world: &WorldState, sid: SessionId) {
    let (premium, current_bonus) = world
        .with_session(sid, |h| (h.premium_in_use, h.flash_exp_bonus))
        .unwrap_or((0, 0));

    // Only premium 11 (EXP_Premium) or 7 (PLATINUM_PREMIUM) can stack EXP flash
    if premium != 11 && premium != 7 {
        return;
    }
    if current_bonus >= MAX_FLASH_EXP_DC_BONUS {
        return;
    }

    // Remove old notice before updating
    send_flash_notice(world, sid, true);

    let flash_time_setting = world.get_flash_time_setting();

    world.update_session(sid, |h| {
        h.flash_war_bonus = 0;
        h.flash_dc_bonus = 0;
        h.flash_exp_bonus = (h.flash_exp_bonus + 10).min(MAX_FLASH_EXP_DC_BONUS);

        if flash_time_setting > 0 && (h.flash_time == 0 || h.flash_type != FLASH_TYPE_EXP) {
            h.flash_time = flash_time_setting;
        }

        if flash_time_setting > 0 && h.flash_count < MAX_FLASH_COUNT {
            h.flash_count += 1;
        }

        h.flash_type = FLASH_TYPE_EXP;
    });

    send_flash_notice(world, sid, false);
}

/// Apply flash DC/Drop bonus on item use (BUFF_TYPE_FISHING with sSpecialAmount == 1).
pub fn dc_flash(world: &WorldState, sid: SessionId) {
    let (premium, current_bonus) = world
        .with_session(sid, |h| (h.premium_in_use, h.flash_dc_bonus))
        .unwrap_or((0, 0));

    if premium != 10 && premium != 7 {
        return;
    }
    if current_bonus >= MAX_FLASH_EXP_DC_BONUS {
        return;
    }

    send_flash_notice(world, sid, true);

    let flash_time_setting = world.get_flash_time_setting();

    world.update_session(sid, |h| {
        h.flash_exp_bonus = 0;
        h.flash_war_bonus = 0;
        h.flash_dc_bonus = (h.flash_dc_bonus + 10).min(MAX_FLASH_EXP_DC_BONUS);

        if flash_time_setting > 0 && (h.flash_time == 0 || h.flash_type != FLASH_TYPE_DC) {
            h.flash_time = flash_time_setting;
        }

        if flash_time_setting > 0 && h.flash_count < MAX_FLASH_COUNT {
            h.flash_count += 1;
        }

        h.flash_type = FLASH_TYPE_DC;
    });

    send_flash_notice(world, sid, false);
}

/// Apply flash WAR/Loyalty bonus on item use (BUFF_TYPE_FISHING with sSpecialAmount == 3).
pub fn war_flash(world: &WorldState, sid: SessionId) {
    let (premium, current_bonus) = world
        .with_session(sid, |h| (h.premium_in_use, h.flash_war_bonus))
        .unwrap_or((0, 0));

    if premium != 12 && premium != 7 {
        return;
    }
    if current_bonus >= MAX_FLASH_WAR_BONUS {
        return;
    }

    send_flash_notice(world, sid, true);

    let flash_time_setting = world.get_flash_time_setting();

    world.update_session(sid, |h| {
        h.flash_exp_bonus = 0;
        h.flash_dc_bonus = 0;
        h.flash_war_bonus = (h.flash_war_bonus + 1).min(MAX_FLASH_WAR_BONUS);

        if flash_time_setting > 0 && (h.flash_time == 0 || h.flash_type != FLASH_TYPE_WAR) {
            h.flash_time = flash_time_setting;
        }

        if flash_time_setting > 0 && h.flash_count < MAX_FLASH_COUNT {
            h.flash_count += 1;
        }

        h.flash_type = FLASH_TYPE_WAR;
    });

    send_flash_notice(world, sid, false);
}

/// Restore flash bonuses from DB-persisted flash state on login.
/// Called during GameStart phase 2 if `flash_time > 0 && GetPremium()`.
pub fn set_flash_time_note(world: &WorldState, sid: SessionId) {
    let (flash_time, flash_count, flash_type, premium) = match world.with_session(sid, |h| {
        (h.flash_time, h.flash_count, h.flash_type, h.premium_in_use)
    }) {
        Some(v) => v,
        None => return,
    };

    if flash_time == 0 || !is_flash_premium(premium) {
        return;
    }

    let count = flash_count.min(MAX_FLASH_COUNT);

    world.update_session(sid, |h| {
        if premium == 10 || flash_type == 2 {
            h.flash_dc_bonus = count * 10;
        } else if premium == 11 || flash_type == 1 {
            h.flash_exp_bonus = count * 10;
        } else if premium == 12 || flash_type == 3 {
            h.flash_war_bonus = count;
        }
    });

    send_flash_notice(world, sid, false);
}

/// Remove all flash bonuses (called on premium switch or flash time expiry).
/// Order matters: C++ zeros flash_time/count/type first, then sends notice
/// (while bonus values are still set so the notice packet is non-empty),
/// then zeros the bonus values.
pub fn remove_flash_bonuses(world: &WorldState, sid: SessionId) {
    let has_bonus = world
        .with_session(sid, |h| {
            h.flash_exp_bonus > 0 || h.flash_dc_bonus > 0 || h.flash_war_bonus > 0
        })
        .unwrap_or(false);

    if !has_bonus {
        return;
    }

    // Step 1: Zero flash_time, flash_count, flash_type (C++ line 937-939)
    world.update_session(sid, |h| {
        h.flash_time = 0;
        h.flash_count = 0;
        h.flash_type = 0;
    });

    // Step 2: Send removal notice while bonus values are still set (C++ line 940)
    send_flash_notice(world, sid, true);

    // Step 3: Zero the bonus values (C++ lines 941-943)
    world.update_session(sid, |h| {
        h.flash_exp_bonus = 0;
        h.flash_dc_bonus = 0;
        h.flash_war_bonus = 0;
    });

    // Step 4: Remove the fishing buff (C++ line 944)
    //   CMagicProcess::RemoveType4Buff(BUFF_TYPE_FISHING, this, true);
    remove_fishing_buff(world, sid);
}

/// Flash timer tick — called every minute per player from the update loop.
/// ```c++
/// if (m_flashtime > 0 && m_flashchecktime + PLAYER_FLASH_INTERVAL < UNIXTIME) {
///     m_flashchecktime = UNIXTIME;
///     FlashUpdateTime(--m_flashtime);
/// }
/// ```
pub fn flash_update_tick(world: &WorldState, sid: SessionId, now: u64) {
    let (flash_time, flash_check_time) =
        match world.with_session(sid, |h| (h.flash_time, h.flash_check_time)) {
            Some(v) => v,
            None => return,
        };

    if flash_time == 0 {
        return;
    }

    if flash_check_time + PLAYER_FLASH_INTERVAL >= now {
        return;
    }

    // Decrement flash time
    let new_flash_time = flash_time.saturating_sub(1);
    world.update_session(sid, |h| {
        h.flash_check_time = now;
        h.flash_time = new_flash_time;
    });

    // C++ FlashUpdateTime: remove old notice, if time == 0 clear bonuses
    send_flash_notice(world, sid, true);
    if new_flash_time == 0 {
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 0;
            h.flash_dc_bonus = 0;
            h.flash_war_bonus = 0;
        });
        //   CMagicProcess::RemoveType4Buff(BUFF_TYPE_FISHING, this, true);
        remove_fishing_buff(world, sid);
    }
    send_flash_notice(world, sid, false);
}

/// Burning time tick — called every update cycle per player.
/// If `flame_time > 0` and current time >= `flame_time`, increment flame_level (max 3)
/// and set next flame time to +1 hour.
pub fn burning_time_tick(world: &WorldState, sid: SessionId, now: u64) {
    let (flame_time, flame_level) = match world.with_session(sid, |h| (h.flame_time, h.flame_level))
    {
        Some(v) => v,
        None => return,
    };

    if flame_time == 0 || flame_level >= MAX_FLAME_LEVEL || now < flame_time {
        return;
    }

    world.update_session(sid, |h| {
        h.flame_level += 1;
        h.flame_time = now + BURNING_HOUR;
    });
}

/// Start burning timer on zone entry / game entry.
/// Sets the initial flame_time to `now + 1 hour` if not already active.
pub fn start_burning_timer(world: &WorldState, sid: SessionId, now: u64) {
    let flame_time = world.with_session(sid, |h| h.flame_time).unwrap_or(0);
    if flame_time == 0 {
        world.update_session(sid, |h| {
            h.flame_time = now + BURNING_HOUR;
        });
    }
}

/// Send flash notice to client (add or remove).
/// Wire format:
/// ```text
/// WIZ_NOTICE << DByte << u8(4) << u8(is_remove ? 2 : 1) << header_str << desc_str
/// ```
pub fn send_flash_notice(world: &WorldState, sid: SessionId, is_remove: bool) {
    let (flash_exp, flash_dc, flash_war, flash_time) = match world.with_session(sid, |h| {
        (
            h.flash_exp_bonus,
            h.flash_dc_bonus,
            h.flash_war_bonus,
            h.flash_time,
        )
    }) {
        Some(v) => v,
        None => return,
    };

    // Build header string
    let header = if flash_exp > 0 {
        format!("Exp +{}%", flash_exp)
    } else if flash_dc > 0 {
        format!("Item Drop +{}%", flash_dc)
    } else if flash_war > 0 {
        format!("Cont +{}", flash_war)
    } else {
        return; // No bonus active, nothing to send
    };

    let description = if !header.is_empty() && flash_time > 0 {
        format!("{} .remaining time {}", header, flash_time)
    } else {
        header.clone()
    };

    // WIZ_NOTICE — C++ uses DByte() but our write_string already uses u16 length prefix
    let mut pkt = Packet::new(Opcode::WizNotice as u8);
    pkt.write_u8(4); // notice type = flash
    pkt.write_u8(if is_remove { 2 } else { 1 });
    pkt.write_string(&header);
    pkt.write_string(&description);

    world.send_to_session_owned(sid, pkt);
}

/// Get flash EXP bonus for a session (0-100 percentage).
pub fn get_flash_exp_bonus(world: &WorldState, sid: SessionId) -> u8 {
    world.with_session(sid, |h| h.flash_exp_bonus).unwrap_or(0)
}

/// Get flash DC bonus for a session (0-100 percentage).
pub fn get_flash_dc_bonus(world: &WorldState, sid: SessionId) -> u8 {
    world.with_session(sid, |h| h.flash_dc_bonus).unwrap_or(0)
}

/// Get flash WAR bonus for a session (0-10).
pub fn get_flash_war_bonus(world: &WorldState, sid: SessionId) -> u8 {
    world.with_session(sid, |h| h.flash_war_bonus).unwrap_or(0)
}

/// Get the current flame level for a session (0-3).
pub fn get_flame_level(world: &WorldState, sid: SessionId) -> u16 {
    world.with_session(sid, |h| h.flame_level).unwrap_or(0)
}

/// Remove the BUFF_TYPE_FISHING buff when flash bonuses expire.
/// Removes the fishing buff from the session's buff map and sends a
/// `MAGIC_DURATION_EXPIRED` packet to the client so the buff icon disappears.
fn remove_fishing_buff(world: &WorldState, sid: SessionId) {
    if world.remove_buff(sid, BUFF_TYPE_FISHING).is_some() {
        let expired_pkt = build_buff_expired_packet(BUFF_TYPE_FISHING as u8);
        world.send_to_session_owned(sid, expired_pkt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{BurningFeatureRates, WorldState};
    use ko_protocol::PacketReader;
    use tokio::sync::mpsc;

    fn setup_world() -> (WorldState, SessionId) {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        (world, 1)
    }

    fn setup_world_with_premium(premium: u8) -> (WorldState, SessionId) {
        let (world, sid) = setup_world();
        world.update_session(sid, |h| {
            h.premium_in_use = premium;
        });
        (world, sid)
    }

    // ── is_flash_premium tests ──────────────────────────────────────

    #[test]
    fn test_is_flash_premium_valid() {
        assert!(is_flash_premium(7)); // PLATINUM
        assert!(is_flash_premium(10)); // DISC
        assert!(is_flash_premium(11)); // EXP
        assert!(is_flash_premium(12)); // WAR
    }

    #[test]
    fn test_is_flash_premium_invalid() {
        assert!(!is_flash_premium(0));
        assert!(!is_flash_premium(1));
        assert!(!is_flash_premium(5));
        assert!(!is_flash_premium(8));
        assert!(!is_flash_premium(9));
        assert!(!is_flash_premium(13));
    }

    // ── exp_flash tests ─────────────────────────────────────────────

    #[test]
    fn test_use_flash_ten_stacks_then_rejects_without_mutation() {
        for (premium, kind) in [(10, 1), (11, 2), (12, 3)] {
            let (world, sid) = setup_world_with_premium(premium);
            for count in 1..=10 {
                assert!(use_flash(&world, sid, kind));
                assert_eq!(world.with_session(sid, |h| h.flash_count), Some(count));
                let bonus = match kind {
                    1 => get_flash_dc_bonus(&world, sid),
                    2 => get_flash_exp_bonus(&world, sid),
                    _ => get_flash_war_bonus(&world, sid),
                };
                assert_eq!(bonus, if kind == 3 { count } else { count * 10 });
            }
            assert!(!use_flash(&world, sid, kind));
            assert_eq!(world.with_session(sid, |h| h.flash_count), Some(10));
        }
    }

    #[test]
    fn test_use_flash_rejects_wrong_premium_and_unknown_kind() {
        let (world, sid) = setup_world_with_premium(11);
        assert!(!use_flash(&world, sid, 1));
        assert!(!use_flash(&world, sid, 3));
        assert!(!use_flash(&world, sid, 4));
        assert_eq!(world.with_session(sid, |h| h.flash_count), Some(0));
    }

    #[test]
    fn test_use_flash_switch_resets_count_to_actual_bonus() {
        let (world, sid) = setup_world_with_premium(7);
        for _ in 0..5 {
            assert!(use_flash(&world, sid, 2));
        }
        assert!(use_flash(&world, sid, 1));
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
        assert_eq!(get_flash_dc_bonus(&world, sid), 10);
        assert_eq!(world.with_session(sid, |h| h.flash_count), Some(1));
    }

    #[test]
    fn test_exp_flash_stacks() {
        let (world, sid) = setup_world_with_premium(11);
        exp_flash(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 10);
        exp_flash(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 20);
    }

    #[test]
    fn test_exp_flash_resets_other_bonuses() {
        let (world, sid) = setup_world_with_premium(7);
        // First give DC bonus
        dc_flash(&world, sid);
        assert_eq!(get_flash_dc_bonus(&world, sid), 10);
        // Now exp flash should reset DC
        world.update_session(sid, |h| h.premium_in_use = 11);
        exp_flash(&world, sid);
        assert_eq!(get_flash_dc_bonus(&world, sid), 0);
        assert_eq!(get_flash_exp_bonus(&world, sid), 10);
    }

    #[test]
    fn test_exp_flash_caps_at_100() {
        let (world, sid) = setup_world_with_premium(11);
        for _ in 0..15 {
            exp_flash(&world, sid);
        }
        assert_eq!(get_flash_exp_bonus(&world, sid), 100);
    }

    #[test]
    fn test_exp_flash_wrong_premium() {
        let (world, sid) = setup_world_with_premium(10); // DC premium
        exp_flash(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
    }

    #[test]
    fn test_exp_flash_platinum_premium() {
        let (world, sid) = setup_world_with_premium(7); // Platinum
        exp_flash(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 10);
    }

    // ── dc_flash tests ──────────────────────────────────────────────

    #[test]
    fn test_dc_flash_stacks() {
        let (world, sid) = setup_world_with_premium(10);
        dc_flash(&world, sid);
        assert_eq!(get_flash_dc_bonus(&world, sid), 10);
        dc_flash(&world, sid);
        assert_eq!(get_flash_dc_bonus(&world, sid), 20);
    }

    #[test]
    fn test_dc_flash_caps_at_100() {
        let (world, sid) = setup_world_with_premium(10);
        for _ in 0..15 {
            dc_flash(&world, sid);
        }
        assert_eq!(get_flash_dc_bonus(&world, sid), 100);
    }

    #[test]
    fn test_dc_flash_wrong_premium() {
        let (world, sid) = setup_world_with_premium(11); // EXP premium
        dc_flash(&world, sid);
        assert_eq!(get_flash_dc_bonus(&world, sid), 0);
    }

    // ── war_flash tests ─────────────────────────────────────────────

    #[test]
    fn test_war_flash_stacks() {
        let (world, sid) = setup_world_with_premium(12);
        war_flash(&world, sid);
        assert_eq!(get_flash_war_bonus(&world, sid), 1);
        war_flash(&world, sid);
        assert_eq!(get_flash_war_bonus(&world, sid), 2);
    }

    #[test]
    fn test_war_flash_caps_at_10() {
        let (world, sid) = setup_world_with_premium(12);
        for _ in 0..15 {
            war_flash(&world, sid);
        }
        assert_eq!(get_flash_war_bonus(&world, sid), 10);
    }

    #[test]
    fn test_war_flash_wrong_premium() {
        let (world, sid) = setup_world_with_premium(11); // EXP premium
        war_flash(&world, sid);
        assert_eq!(get_flash_war_bonus(&world, sid), 0);
    }

    // ── set_flash_time_note tests ───────────────────────────────────

    #[test]
    fn test_set_flash_time_note_exp() {
        let (world, sid) = setup_world_with_premium(11);
        world.update_session(sid, |h| {
            h.flash_time = 100;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_EXP;
        });
        set_flash_time_note(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 50); // 5 * 10 = 50
    }

    #[test]
    fn test_set_flash_time_note_dc() {
        let (world, sid) = setup_world_with_premium(10);
        world.update_session(sid, |h| {
            h.flash_time = 100;
            h.flash_count = 3;
            h.flash_type = FLASH_TYPE_DC;
        });
        set_flash_time_note(&world, sid);
        assert_eq!(get_flash_dc_bonus(&world, sid), 30); // 3 * 10 = 30
    }

    #[test]
    fn test_set_flash_time_note_war() {
        let (world, sid) = setup_world_with_premium(12);
        world.update_session(sid, |h| {
            h.flash_time = 100;
            h.flash_count = 7;
            h.flash_type = FLASH_TYPE_WAR;
        });
        set_flash_time_note(&world, sid);
        assert_eq!(get_flash_war_bonus(&world, sid), 7);
    }

    #[test]
    fn test_set_flash_time_note_no_flash_time() {
        let (world, sid) = setup_world_with_premium(11);
        world.update_session(sid, |h| {
            h.flash_time = 0;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_EXP;
        });
        set_flash_time_note(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
    }

    #[test]
    fn test_set_flash_time_note_invalid_premium() {
        let (world, sid) = setup_world_with_premium(5); // Gold premium
        world.update_session(sid, |h| {
            h.flash_time = 100;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_EXP;
        });
        set_flash_time_note(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
    }

    // ── remove_flash_bonuses tests ──────────────────────────────────

    #[test]
    fn test_remove_flash_bonuses() {
        let (world, sid) = setup_world_with_premium(11);
        exp_flash(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 10);
        remove_flash_bonuses(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(0);
        assert_eq!(ft, 0);
    }

    // ── flash_update_tick tests ─────────────────────────────────────

    #[test]
    fn test_flash_update_tick_decrements() {
        let (world, sid) = setup_world_with_premium(11);
        world.update_session(sid, |h| {
            h.flash_time = 5;
            h.flash_exp_bonus = 50;
            h.flash_check_time = 100;
        });
        // Tick at time 161 (>= 100 + 60)
        flash_update_tick(&world, sid, 161);
        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(0);
        assert_eq!(ft, 4);
    }

    #[test]
    fn test_flash_update_tick_too_early() {
        let (world, sid) = setup_world_with_premium(11);
        world.update_session(sid, |h| {
            h.flash_time = 5;
            h.flash_exp_bonus = 50;
            h.flash_check_time = 100;
        });
        // Tick at time 150 (< 100 + 60)
        flash_update_tick(&world, sid, 150);
        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(0);
        assert_eq!(ft, 5); // unchanged
    }

    #[test]
    fn test_flash_update_tick_expiry_clears_bonuses() {
        let (world, sid) = setup_world_with_premium(11);
        world.update_session(sid, |h| {
            h.flash_time = 1;
            h.flash_exp_bonus = 50;
            h.flash_check_time = 0;
        });
        flash_update_tick(&world, sid, 61);
        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(0);
        assert_eq!(ft, 0);
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
    }

    // ── burning_time_tick tests ─────────────────────────────────────

    #[test]
    fn test_burning_time_tick_increments() {
        let (world, sid) = setup_world();
        world.update_session(sid, |h| {
            h.flame_level = 0;
            h.flame_time = 1000;
        });
        burning_time_tick(&world, sid, 1001);
        assert_eq!(get_flame_level(&world, sid), 1);
        // Next flame time should be 1001 + 3600
        let ft = world.with_session(sid, |h| h.flame_time).unwrap_or(0);
        assert_eq!(ft, 1001 + 3600);
    }

    #[test]
    fn test_burning_time_tick_max_level() {
        let (world, sid) = setup_world();
        world.update_session(sid, |h| {
            h.flame_level = 3;
            h.flame_time = 1000;
        });
        burning_time_tick(&world, sid, 2000);
        assert_eq!(get_flame_level(&world, sid), 3); // no change
    }

    #[test]
    fn test_burning_time_tick_not_ready() {
        let (world, sid) = setup_world();
        world.update_session(sid, |h| {
            h.flame_level = 1;
            h.flame_time = 2000;
        });
        burning_time_tick(&world, sid, 1000); // too early
        assert_eq!(get_flame_level(&world, sid), 1); // no change
    }

    #[test]
    fn test_burning_time_tick_inactive() {
        let (world, sid) = setup_world();
        world.update_session(sid, |h| {
            h.flame_level = 0;
            h.flame_time = 0; // not active
        });
        burning_time_tick(&world, sid, 5000);
        assert_eq!(get_flame_level(&world, sid), 0); // no change
    }

    // ── start_burning_timer tests ───────────────────────────────────

    #[test]
    fn test_start_burning_timer() {
        let (world, sid) = setup_world();
        start_burning_timer(&world, sid, 1000);
        let ft = world.with_session(sid, |h| h.flame_time).unwrap_or(0);
        assert_eq!(ft, 1000 + 3600);
    }

    #[test]
    fn test_start_burning_timer_already_active() {
        let (world, sid) = setup_world();
        world.update_session(sid, |h| {
            h.flame_time = 5000;
        });
        start_burning_timer(&world, sid, 1000);
        let ft = world.with_session(sid, |h| h.flame_time).unwrap_or(0);
        assert_eq!(ft, 5000); // unchanged
    }

    // ── send_flash_notice packet format tests ───────────────────────

    #[test]
    fn test_flash_notice_packet_exp() {
        // Manually build what send_flash_notice would build
        let header = format!("Exp +{}%", 50);
        let desc = format!("{} .remaining time {}", header, 120);

        let mut pkt = Packet::new(Opcode::WizNotice as u8);
        pkt.write_u8(4);
        pkt.write_u8(1); // not remove
        pkt.write_string(&header);
        pkt.write_string(&desc);

        assert_eq!(pkt.opcode, Opcode::WizNotice as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4)); // notice type
        assert_eq!(r.read_u8(), Some(1)); // add (not remove)
    }

    #[test]
    fn test_flash_notice_packet_remove() {
        let header = "Exp +50%";
        let mut pkt = Packet::new(Opcode::WizNotice as u8);
        pkt.write_u8(4);
        pkt.write_u8(2); // remove
        pkt.write_string(header);
        pkt.write_string(header);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_u8(), Some(2)); // remove flag
    }

    // ── Flash count capping test ────────────────────────────────────

    #[test]
    fn test_flash_count_caps_at_10() {
        let (world, sid) = setup_world_with_premium(11);
        for _ in 0..15 {
            exp_flash(&world, sid);
        }
        let count = world.with_session(sid, |h| h.flash_count).unwrap_or(0);
        assert_eq!(count, 10);
    }

    // ── Flash type switching test ───────────────────────────────────

    #[test]
    fn test_flash_type_switching() {
        let (world, sid) = setup_world_with_premium(7); // Platinum (can use all)
        exp_flash(&world, sid);
        let ft = world.with_session(sid, |h| h.flash_type).unwrap_or(0);
        assert_eq!(ft, FLASH_TYPE_EXP);

        dc_flash(&world, sid);
        let ft = world.with_session(sid, |h| h.flash_type).unwrap_or(0);
        assert_eq!(ft, FLASH_TYPE_DC);

        war_flash(&world, sid);
        let ft = world.with_session(sid, |h| h.flash_type).unwrap_or(0);
        assert_eq!(ft, FLASH_TYPE_WAR);
    }

    // ── Burning features accessor test ──────────────────────────────

    #[test]
    fn test_burning_feature_accessor() {
        let world = WorldState::new();
        // Set level 1 burning features
        {
            let mut features = world.burning_features.write();
            features[0] = BurningFeatureRates {
                np_rate: 10,
                money_rate: 15,
                exp_rate: 20,
                drop_rate: 5,
            };
        }
        let feat = world.get_burning_feature(1).unwrap();
        assert_eq!(feat.exp_rate, 20);
        assert_eq!(feat.drop_rate, 5);
        assert!(world.get_burning_feature(0).is_none());
        assert!(world.get_burning_feature(4).is_none());
    }

    // ── H1 fix: remove_flash_bonuses notice ordering tests ──────────

    #[test]
    fn test_remove_flash_bonuses_sends_notice_with_nonzero_bonus() {
        // H1 fix: verify that remove_flash_bonuses sends the notice
        // BEFORE zeroing bonus values, so the client receives the removal.
        let (world, sid) = setup_world_with_premium(11);
        let (tx, mut rx) = mpsc::unbounded_channel();
        // Replace the session's tx to capture packets
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 50;
            h.flash_time = 100;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_EXP;
        });
        // Re-register with a new tx to capture packets
        world.register_session(sid, tx);
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 50;
            h.flash_time = 100;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_EXP;
        });

        remove_flash_bonuses(&world, sid);

        // After removal, all values should be zero
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(1);
        assert_eq!(ft, 0);

        // Check that we received at least one WizNotice packet (the removal notice)
        let mut found_notice = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizNotice as u8 {
                let mut r = PacketReader::new(&pkt.data);
                let notice_type = r.read_u8();
                let remove_flag = r.read_u8();
                if notice_type == Some(4) && remove_flag == Some(2) {
                    found_notice = true;
                }
            }
        }
        assert!(found_notice, "Expected WizNotice removal packet (flag=2)");
    }

    #[test]
    fn test_remove_flash_bonuses_all_fields_zeroed() {
        // After removal, ALL flash fields should be zero
        let (world, sid) = setup_world_with_premium(7); // Platinum
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 30;
            h.flash_dc_bonus = 0;
            h.flash_war_bonus = 0;
            h.flash_time = 50;
            h.flash_count = 3;
            h.flash_type = FLASH_TYPE_EXP;
        });
        remove_flash_bonuses(&world, sid);
        let (ft, fc, ftype, exp, dc, war) = world
            .with_session(sid, |h| {
                (
                    h.flash_time,
                    h.flash_count,
                    h.flash_type,
                    h.flash_exp_bonus,
                    h.flash_dc_bonus,
                    h.flash_war_bonus,
                )
            })
            .unwrap();
        assert_eq!(ft, 0);
        assert_eq!(fc, 0);
        assert_eq!(ftype, 0);
        assert_eq!(exp, 0);
        assert_eq!(dc, 0);
        assert_eq!(war, 0);
    }

    #[test]
    fn test_remove_flash_bonuses_dc_sends_notice() {
        let (world, sid) = setup_world_with_premium(10);
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.update_session(sid, |h| {
            h.flash_dc_bonus = 40;
            h.flash_time = 80;
            h.flash_count = 4;
            h.flash_type = FLASH_TYPE_DC;
        });

        remove_flash_bonuses(&world, sid);

        let mut found_notice = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizNotice as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(4) && r.read_u8() == Some(2) {
                    found_notice = true;
                }
            }
        }
        assert!(found_notice, "Expected DC flash removal notice");
    }

    #[test]
    fn test_remove_flash_bonuses_war_sends_notice() {
        let (world, sid) = setup_world_with_premium(12);
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.update_session(sid, |h| {
            h.flash_war_bonus = 5;
            h.flash_time = 60;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_WAR;
        });

        remove_flash_bonuses(&world, sid);

        let mut found_notice = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizNotice as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(4) && r.read_u8() == Some(2) {
                    found_notice = true;
                }
            }
        }
        assert!(found_notice, "Expected WAR flash removal notice");
    }

    #[test]
    fn test_remove_flash_bonuses_no_notice_when_no_bonus() {
        // When no bonus is active, remove_flash_bonuses should be a no-op
        let (world, sid) = setup_world_with_premium(11);
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);
        // All bonuses are 0 by default

        remove_flash_bonuses(&world, sid);

        let pkt_count = std::iter::from_fn(|| rx.try_recv().ok()).count();
        assert_eq!(pkt_count, 0, "No packets expected when no bonus active");
    }

    // ── M1 fix: remove fishing buff on flash removal tests ──────────

    fn make_fishing_buff() -> crate::world::ActiveBuff {
        use std::time::Instant;
        crate::world::ActiveBuff {
            skill_id: 500000,
            buff_type: BUFF_TYPE_FISHING,
            caster_sid: 1,
            start_time: Instant::now(),
            duration_secs: 600,
            attack_speed: 0,
            speed: 0,
            ac: 0,
            ac_pct: 0,
            attack: 0,
            magic_attack: 0,
            max_hp: 0,
            max_hp_pct: 0,
            max_mp: 0,
            max_mp_pct: 0,
            str_mod: 0,
            sta_mod: 0,
            dex_mod: 0,
            intel_mod: 0,
            cha_mod: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            hit_rate: 0,
            avoid_rate: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: true,
        }
    }

    #[test]
    fn test_remove_flash_bonuses_removes_fishing_buff() {
        //   CMagicProcess::RemoveType4Buff(BUFF_TYPE_FISHING, this, true);
        let (world, sid) = setup_world_with_premium(11);
        // Add a fishing buff
        world.apply_buff(sid, make_fishing_buff());
        let _ = world.remove_buff(sid, BUFF_TYPE_FISHING);
        // Re-add (remove_buff consumed it)
        world.apply_buff(sid, make_fishing_buff());

        // Set flash bonuses
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 50;
            h.flash_time = 100;
            h.flash_count = 5;
            h.flash_type = FLASH_TYPE_EXP;
        });

        remove_flash_bonuses(&world, sid);

        // Fishing buff should be removed
        let buffs = world.get_active_buffs(sid);
        let fishing = buffs.iter().find(|b| b.buff_type == BUFF_TYPE_FISHING);
        assert!(
            fishing.is_none(),
            "Fishing buff should be removed after flash expiry"
        );
    }

    #[test]
    fn test_remove_flash_bonuses_sends_fishing_expired_packet() {
        // Verify MAGIC_DURATION_EXPIRED packet for BUFF_TYPE_FISHING is sent
        let (world, sid) = setup_world_with_premium(11);
        world.apply_buff(sid, make_fishing_buff());
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 30;
            h.flash_time = 60;
            h.flash_count = 3;
            h.flash_type = FLASH_TYPE_EXP;
        });

        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);
        // Re-set after register
        world.apply_buff(sid, make_fishing_buff());
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 30;
            h.flash_time = 60;
            h.flash_count = 3;
            h.flash_type = FLASH_TYPE_EXP;
        });

        remove_flash_bonuses(&world, sid);

        let mut found_expired = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizMagicProcess as u8 {
                let mut r = PacketReader::new(&pkt.data);
                let sub_op = r.read_u8();
                let btype = r.read_u8();
                if sub_op == Some(5) && btype == Some(BUFF_TYPE_FISHING as u8) {
                    found_expired = true;
                }
            }
        }
        assert!(
            found_expired,
            "Expected MAGIC_DURATION_EXPIRED packet for BUFF_TYPE_FISHING (48)"
        );
    }

    #[test]
    fn test_remove_flash_bonuses_no_fishing_buff_no_crash() {
        // When no fishing buff exists, remove should be a no-op (no crash)
        let (world, sid) = setup_world_with_premium(11);
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 20;
            h.flash_time = 30;
            h.flash_count = 2;
            h.flash_type = FLASH_TYPE_EXP;
        });

        // Should not panic even without a fishing buff
        remove_flash_bonuses(&world, sid);
        assert_eq!(get_flash_exp_bonus(&world, sid), 0);
    }

    #[test]
    fn test_flash_update_tick_removes_fishing_buff_at_zero() {
        // When flash_time ticks to 0, fishing buff should be removed
        let (world, sid) = setup_world_with_premium(11);
        world.apply_buff(sid, make_fishing_buff());
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 10;
            h.flash_time = 1; // Will tick to 0
            h.flash_check_time = 0;
            h.flash_count = 1;
            h.flash_type = FLASH_TYPE_EXP;
        });

        // Tick past the interval
        flash_update_tick(&world, sid, PLAYER_FLASH_INTERVAL + 1);

        // Flash time should be 0
        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(1);
        assert_eq!(ft, 0);

        // Fishing buff should be removed
        let buffs = world.get_active_buffs(sid);
        let fishing = buffs.iter().find(|b| b.buff_type == BUFF_TYPE_FISHING);
        assert!(
            fishing.is_none(),
            "Fishing buff should be removed when flash_time hits 0"
        );
    }

    #[test]
    fn test_flash_update_tick_keeps_fishing_buff_when_not_zero() {
        // When flash_time ticks down but doesn't reach 0, fishing buff stays
        let (world, sid) = setup_world_with_premium(11);
        world.apply_buff(sid, make_fishing_buff());
        world.update_session(sid, |h| {
            h.flash_exp_bonus = 10;
            h.flash_time = 5; // Will tick to 4 (not 0)
            h.flash_check_time = 0;
            h.flash_count = 1;
            h.flash_type = FLASH_TYPE_EXP;
        });

        flash_update_tick(&world, sid, PLAYER_FLASH_INTERVAL + 1);

        let ft = world.with_session(sid, |h| h.flash_time).unwrap_or(0);
        assert_eq!(ft, 4);

        // Fishing buff should still be present
        let buffs = world.get_active_buffs(sid);
        let fishing = buffs.iter().find(|b| b.buff_type == BUFF_TYPE_FISHING);
        assert!(
            fishing.is_some(),
            "Fishing buff should remain when flash_time > 0"
        );
    }
}
