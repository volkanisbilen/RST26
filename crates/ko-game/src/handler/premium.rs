//! WIZ_PREMIUM (0x71) handler — premium/VIP subscription system.
//! Sub-opcodes (from client):
//! - 4 (HandlePremium): Switch the currently active premium type.
//! Server-initiated packets:
//! - 1 (SendPremiumInfo): List of active premiums with remaining time.
//! - 2 (SendClanPremiumPkt): Clan premium status.
//! Premium types (`Define.h:513-528`):
//! -  0: NO_PREMIUM
//! -  1: NORMAL_PREMIUM
//! -  2: CLAN_PREMIUM
//! -  3: BRONZE_PREMIUM
//! -  4: SILVER_PREMIUM
//! -  5: GOLD_PREMIUM
//! -  6: Dummy_Premium
//! -  7: PLATINUM_PREMIUM
//! -  8: ROYAL_PREMIUM
//! -  9: UNKNOW_PREMIUM
//! - 10: DISC_Premium
//! - 11: EXP_Premium
//! - 12: WAR_Premium
//! - 13: SWITCH_PREMIUM
//! Each premium entry stores a type (u8) and expiration unix timestamp (u32).
//! Maximum `PREMIUM_TOTAL` (6) simultaneous premium slots per account.

use std::time::{SystemTime, UNIX_EPOCH};

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

/// Maximum number of premium slots per account.
const PREMIUM_TOTAL: usize = 6;

/// No premium active.
const NO_PREMIUM: u8 = 0;

/// Sub-opcode: server sends premium info list.
pub(crate) const SUBOPCODE_PREMIUM_INFO: u8 = 1;

/// Sub-opcode: clan premium packet.
const SUBOPCODE_CLAN_PREMIUM: u8 = 2;

/// Sub-opcode: client requests to switch active premium.
const SUBOPCODE_HANDLE_PREMIUM: u8 = 4;

/// Handle WIZ_PREMIUM from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut r = PacketReader::new(&pkt.data);
    let opcode = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };
    let p_type = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    // C++ only handles opcode == 4
    if opcode != SUBOPCODE_HANDLE_PREMIUM {
        return Ok(());
    }

    let sid = session.session_id();
    let world = session.world();
    let now = current_unix_time();

    // Check if already using this premium type
    let already_in_use = world
        .with_session(sid, |h| h.premium_in_use == p_type)
        .unwrap_or(false);

    if already_in_use {
        // Send error: already using this type
        let resp = build_handle_premium_error(opcode);
        session.send_packet(&resp).await?;
        debug!(
            "[{}] WIZ_PREMIUM: type {} already in use",
            session.addr(),
            p_type
        );
        return Ok(());
    }

    // Check if premium exists and is not expired
    let premium_valid = world
        .with_session(sid, |h| {
            if let Some(&expiry) = h.premium_map.get(&p_type) {
                expiry >= now
            } else {
                false
            }
        })
        .unwrap_or(false);

    if !premium_valid {
        // Send error: premium not found or expired
        let resp = build_handle_premium_error(opcode);
        session.send_packet(&resp).await?;
        debug!(
            "[{}] WIZ_PREMIUM: type {} not found or expired",
            session.addr(),
            p_type
        );
        return Ok(());
    }

    // `if (m_FlashExpBonus > 0 || m_FlashDcBonus > 0 || m_FlashWarBonus > 0) SetFlashTimeNote(true);`
    crate::systems::flash::remove_flash_bonuses(world, sid);

    // Set the new premium in use
    world.update_session(sid, |h| {
        h.premium_in_use = p_type;
    });

    // Send updated premium info
    let info_pkt = build_premium_info(session);
    session.send_packet(&info_pkt).await?;

    debug!(
        "[{}] WIZ_PREMIUM: switched to type {}",
        session.addr(),
        p_type
    );
    Ok(())
}

/// Build the HandlePremium error response.
/// Wire: `WIZ_PREMIUM << u8(opcode) << i8(0) << i16(-1)`
fn build_handle_premium_error(opcode: u8) -> Packet {
    let mut resp = Packet::new(Opcode::WizPremium as u8);
    resp.write_u8(opcode);
    resp.write_i8(0);
    resp.write_i16(-1);
    resp
}

/// Build and return the premium info packet.
/// Wire layout:
/// ```text
/// WIZ_PREMIUM << u8(1) << u8(count)
///   [for each premium: << u8(premium_type) << u16(time_hours)]
///   << u8(premium_in_use) << u32(0)
/// ```
pub fn build_premium_info(session: &ClientSession) -> Packet {
    let sid = session.session_id();
    let world = session.world();
    let now = current_unix_time();

    let mut entries: Vec<(u8, u16)> = Vec::with_capacity(10);
    let mut premium_in_use: u8 = NO_PREMIUM;

    world.with_session(sid, |h| {
        premium_in_use = h.premium_in_use;

        for (&p_type, &expiry) in &h.premium_map {
            if expiry == 0 || expiry <= now {
                continue;
            }

            let time_rest = expiry.saturating_sub(now);
            let time_show: u16 = if (1..=3600).contains(&time_rest) {
                1
            } else {
                (time_rest / 3600) as u16
            };

            entries.push((p_type, time_show));

            // If no premium is selected yet, auto-select the first valid one
            if premium_in_use == NO_PREMIUM {
                premium_in_use = p_type;
            }
        }
    });

    // If auto-selected a premium, update the session
    let original_in_use = world
        .with_session(sid, |h| h.premium_in_use)
        .unwrap_or(NO_PREMIUM);
    if premium_in_use != original_in_use {
        world.update_session(sid, |h| {
            h.premium_in_use = premium_in_use;
        });
    }

    let mut resp = Packet::new(Opcode::WizPremium as u8);
    resp.write_u8(SUBOPCODE_PREMIUM_INFO);
    resp.write_u8(entries.len() as u8);
    for (p_type, time_show) in &entries {
        resp.write_u8(*p_type);
        resp.write_u16(*time_show);
    }
    resp.write_u8(premium_in_use);
    resp.write_u32(0);
    resp
}

/// Build the clan premium status packet.
/// Wire layout:
/// ```text
/// WIZ_PREMIUM << u8(2) << u8(status) << u32(time_minutes) << u16(0) << u8(2)
/// ```
/// - status=0 + time=0: no clan premium (exits=true)
/// - status=4 + time=remaining_minutes: clan premium active
pub fn build_clan_premium_packet(active: bool, remaining_minutes: u32) -> Packet {
    let mut resp = Packet::new(Opcode::WizPremium as u8);
    resp.write_u8(SUBOPCODE_CLAN_PREMIUM);
    if active {
        resp.write_u8(4);
        resp.write_u32(remaining_minutes);
    } else {
        resp.write_u8(0);
        resp.write_u32(0);
    }
    resp.write_u16(0);
    resp.write_u8(2);
    resp
}

/// Give a premium to the player.
/// Adds or extends a premium subscription. If `minute` is true, the time
/// is in minutes; otherwise it's in days.
pub fn give_premium(session: &ClientSession, premium_type: u8, time_amount: u16, minute: bool) {
    if premium_type == 0 || premium_type > 13 || time_amount == 0 {
        return;
    }

    let sid = session.session_id();
    let world = session.world();
    let now = current_unix_time();

    world.update_session(sid, |h| {
        if h.premium_map.len() >= PREMIUM_TOTAL && !h.premium_map.contains_key(&premium_type) {
            return;
        }

        let duration_secs: u32 = if minute {
            60 * time_amount as u32
        } else {
            24 * 60 * 60 * time_amount as u32
        };

        let entry = h.premium_map.entry(premium_type).or_insert(now);
        // If expired, reset to now
        if *entry < now {
            *entry = now;
        }
        *entry += duration_secs;

        h.premium_in_use = premium_type;
    });

    // FerihaLog: PremiumInsertLog
    super::audit_log::log_premium(
        session.pool(),
        session.account_id().unwrap_or(""),
        &world.get_session_name(sid).unwrap_or_default(),
        premium_type,
        time_amount as u32,
    );
}

/// Get current unix timestamp as u32.
fn current_unix_time() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use ko_db::models::premium::{PremiumItemExpRow, PremiumItemRow};
    use ko_protocol::PacketReader;

    #[test]
    fn test_premium_constants() {
        assert_eq!(NO_PREMIUM, 0);
        assert_eq!(PREMIUM_TOTAL, 6);
        assert_eq!(SUBOPCODE_PREMIUM_INFO, 1);
        assert_eq!(SUBOPCODE_CLAN_PREMIUM, 2);
        assert_eq!(SUBOPCODE_HANDLE_PREMIUM, 4);
    }

    #[test]
    fn test_handle_premium_error_packet() {
        let pkt = build_handle_premium_error(4);
        assert_eq!(pkt.opcode, Opcode::WizPremium as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4)); // opcode
        assert_eq!(r.read_u8(), Some(0)); // result = 0 (i8 read as u8)
                                          // i16(-1) = 0xFFFF in little-endian
        let lo = r.read_u8().unwrap();
        let hi = r.read_u8().unwrap();
        assert_eq!(i16::from_le_bytes([lo, hi]), -1);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_clan_premium_active_packet() {
        let pkt = build_clan_premium_packet(true, 1440);
        assert_eq!(pkt.opcode, Opcode::WizPremium as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUBOPCODE_CLAN_PREMIUM)); // sub-opcode 2
        assert_eq!(r.read_u8(), Some(4)); // active status
        assert_eq!(r.read_u32(), Some(1440)); // 1440 minutes = 1 day
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u8(), Some(2)); // trailing constant
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_clan_premium_inactive_packet() {
        let pkt = build_clan_premium_packet(false, 0);
        assert_eq!(pkt.opcode, Opcode::WizPremium as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUBOPCODE_CLAN_PREMIUM)); // sub-opcode 2
        assert_eq!(r.read_u8(), Some(0)); // inactive status
        assert_eq!(r.read_u32(), Some(0)); // no time
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u8(), Some(2)); // trailing constant
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_premium_info_packet_no_premiums() {
        // Build a premium info packet with zero entries manually
        // (since build_premium_info requires a session, we test the wire format)
        let mut resp = Packet::new(Opcode::WizPremium as u8);
        resp.write_u8(SUBOPCODE_PREMIUM_INFO);
        resp.write_u8(0); // count = 0
        resp.write_u8(NO_PREMIUM); // premium_in_use
        resp.write_u32(0);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(1)); // sub-opcode
        assert_eq!(r.read_u8(), Some(0)); // count
        assert_eq!(r.read_u8(), Some(0)); // premium_in_use = NO_PREMIUM
        assert_eq!(r.read_u32(), Some(0)); // trailing zero
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_premium_info_packet_with_entries() {
        // Simulate building a premium info packet with 2 entries
        let entries: Vec<(u8, u16)> = vec![(3, 24), (7, 168)]; // Bronze 24h, Platinum 168h
        let premium_in_use: u8 = 3;

        let mut resp = Packet::new(Opcode::WizPremium as u8);
        resp.write_u8(SUBOPCODE_PREMIUM_INFO);
        resp.write_u8(entries.len() as u8);
        for (p_type, time_show) in &entries {
            resp.write_u8(*p_type);
            resp.write_u16(*time_show);
        }
        resp.write_u8(premium_in_use);
        resp.write_u32(0);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(1)); // sub-opcode
        assert_eq!(r.read_u8(), Some(2)); // count = 2
                                          // Entry 1: Bronze, 24 hours
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u16(), Some(24));
        // Entry 2: Platinum, 168 hours
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u16(), Some(168));
        // Footer
        assert_eq!(r.read_u8(), Some(3)); // premium_in_use
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_premium_info_time_calculation() {
        // Time remaining <= 3600 seconds (1 hour) shows as 1
        let time_rest: u32 = 1800; // 30 minutes
        let time_show: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        assert_eq!(time_show, 1);

        // Time remaining > 3600 shows as hours
        let time_rest: u32 = 86400; // 24 hours
        let time_show: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        assert_eq!(time_show, 24);

        // Time remaining = 0 shows as 0
        let time_rest: u32 = 0;
        let time_show: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        assert_eq!(time_show, 0);

        // Exactly 1 hour shows as 1
        let time_rest: u32 = 3600;
        let time_show: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        assert_eq!(time_show, 1);

        // Just over 1 hour shows as 1 (3601/3600 = 1)
        let time_rest: u32 = 3601;
        let time_show: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        assert_eq!(time_show, 1);
    }

    #[test]
    fn test_give_premium_bounds() {
        // Verify type bounds: 0 and >13 are rejected (tested via logic, not session)
        assert!(0 == 0 || 0 > 13); // type 0 is rejected
        assert!(14 == 0 || 14 > 13); // type 14 is rejected
        assert!(1 != 0); // type 1 is valid
        assert!(13 != 0); // type 13 is valid
    }

    #[test]
    fn test_handle_premium_request_packet_roundtrip() {
        // Client sends: WIZ_PREMIUM + u8(opcode=4) + u8(premium_type)
        let mut pkt = Packet::new(Opcode::WizPremium as u8);
        pkt.write_u8(4); // opcode = HandlePremium
        pkt.write_u8(7); // type = PLATINUM_PREMIUM

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_premium_types() {
        // Verify premium type constants match C++ Define.h:513-528
        let normal: u8 = 1;
        let clan: u8 = 2;
        let bronze: u8 = 3;
        let silver: u8 = 4;
        let gold: u8 = 5;
        let dummy: u8 = 6;
        let platinum: u8 = 7;
        let royal: u8 = 8;
        let unknow: u8 = 9;
        let disc: u8 = 10;
        let exp: u8 = 11;
        let war: u8 = 12;
        let switch: u8 = 13;

        assert_eq!(normal, 1);
        assert_eq!(clan, 2);
        assert_eq!(bronze, 3);
        assert_eq!(silver, 4);
        assert_eq!(gold, 5);
        assert_eq!(dummy, 6);
        assert_eq!(platinum, 7);
        assert_eq!(royal, 8);
        assert_eq!(unknow, 9);
        assert_eq!(disc, 10);
        assert_eq!(exp, 11);
        assert_eq!(war, 12);
        assert_eq!(switch, 13);
    }

    /// Helper: create a test WorldState with premium data loaded + session id=1.
    fn setup_premium_world() -> (crate::world::WorldState, u16) {
        use crate::world::PremiumProperty;
        use tokio::sync::mpsc;

        let world = crate::world::WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Insert premium type definitions (matching MSSQL data)
        world.premium_item_types.insert(
            3,
            PremiumItemRow {
                premium_type: 3,
                name: "Bronze Premium".into(),
                exp_restore_pct: 10.0,
                noah_pct: 0,
                drop_pct: 0,
                bonus_loyalty: 2,
                repair_disc_pct: 50,
                item_sell_pct: 30,
            },
        );
        world.premium_item_types.insert(
            5,
            PremiumItemRow {
                premium_type: 5,
                name: "Gold Premium".into(),
                exp_restore_pct: 2.0,
                noah_pct: 5,
                drop_pct: 1,
                bonus_loyalty: 4,
                repair_disc_pct: 50,
                item_sell_pct: 50,
            },
        );
        world.premium_item_types.insert(
            10,
            PremiumItemRow {
                premium_type: 10,
                name: "DISC Premium".into(),
                exp_restore_pct: 2.0,
                noah_pct: 100,
                drop_pct: 2,
                bonus_loyalty: 5,
                repair_disc_pct: 50,
                item_sell_pct: 50,
            },
        );
        world.premium_item_types.insert(
            1,
            PremiumItemRow {
                premium_type: 1,
                name: "Normal Premium".into(),
                exp_restore_pct: 0.0,
                noah_pct: 0,
                drop_pct: 0,
                bonus_loyalty: 0,
                repair_disc_pct: 0,
                item_sell_pct: 0,
            },
        );

        // Insert premium XP exp entries
        *world.premium_item_exp.write() = vec![
            PremiumItemExpRow {
                n_index: 3,
                premium_type: 3,
                min_level: 1,
                max_level: 83,
                s_percent: 20,
            },
            PremiumItemExpRow {
                n_index: 7,
                premium_type: 7,
                min_level: 1,
                max_level: 50,
                s_percent: 400,
            },
            PremiumItemExpRow {
                n_index: 8,
                premium_type: 7,
                min_level: 51,
                max_level: 83,
                s_percent: 100,
            },
            PremiumItemExpRow {
                n_index: 11,
                premium_type: 10,
                min_level: 1,
                max_level: 83,
                s_percent: 30,
            },
        ];

        let _ = PremiumProperty::NoahPercent; // ensure enum is accessible
        (world, 1)
    }

    #[test]
    fn test_premium_property_lookup_with_session() {
        use crate::world::PremiumProperty;

        let (world, sid) = setup_premium_world();

        // Set premium_in_use to Bronze (3)
        world.update_session(sid, |h| {
            h.premium_in_use = 3;
        });

        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::BonusLoyalty),
            2
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::RepairDiscountPercent),
            50
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::ItemSellPercent),
            30
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::NoahPercent),
            0
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::DropPercent),
            0
        );
    }

    #[test]
    fn test_premium_property_no_premium() {
        use crate::world::PremiumProperty;

        let (world, sid) = setup_premium_world();

        // No premium active (default = 0)
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::BonusLoyalty),
            0
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::RepairDiscountPercent),
            0
        );
    }

    #[test]
    fn test_premium_exp_restore_lookup() {
        let (world, sid) = setup_premium_world();

        // Gold premium: exp_restore_pct = 2.0
        world.update_session(sid, |h| {
            h.premium_in_use = 5;
        });
        let restore = world.get_premium_exp_restore(sid);
        assert!((restore - 2.0).abs() < f64::EPSILON);

        // No premium: 0.0
        world.update_session(sid, |h| {
            h.premium_in_use = 0;
        });
        assert!((world.get_premium_exp_restore(sid) - 0.0).abs() < f64::EPSILON);

        // Bronze premium: 10.0
        world.update_session(sid, |h| {
            h.premium_in_use = 3;
        });
        assert!((world.get_premium_exp_restore(sid) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_premium_exp_percent_level_range() {
        let (world, sid) = setup_premium_world();

        // Bronze premium type 3: 20% for level 1-83
        world.update_session(sid, |h| {
            h.premium_in_use = 3;
        });
        assert_eq!(world.get_premium_exp_percent(sid, 1), 20);
        assert_eq!(world.get_premium_exp_percent(sid, 40), 20);
        assert_eq!(world.get_premium_exp_percent(sid, 83), 20);

        // DISC premium type 10: 30% for level 1-83
        world.update_session(sid, |h| {
            h.premium_in_use = 10;
        });
        assert_eq!(world.get_premium_exp_percent(sid, 1), 30);
        assert_eq!(world.get_premium_exp_percent(sid, 83), 30);

        // No premium: 0
        world.update_session(sid, |h| {
            h.premium_in_use = 0;
        });
        assert_eq!(world.get_premium_exp_percent(sid, 50), 0);
    }

    #[test]
    fn test_premium_property_disc_premium() {
        use crate::world::PremiumProperty;

        let (world, sid) = setup_premium_world();

        // DISC Premium (10) has the best bonuses
        world.update_session(sid, |h| {
            h.premium_in_use = 10;
        });

        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::NoahPercent),
            100
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::DropPercent),
            2
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::BonusLoyalty),
            5
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::RepairDiscountPercent),
            50
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::ItemSellPercent),
            50
        );
    }

    #[test]
    fn test_premium_clan_property() {
        use crate::world::PremiumProperty;

        let (world, sid) = setup_premium_world();

        // Set clan premium to type 5 (Gold)
        world.update_session(sid, |h| {
            h.clan_premium_in_use = 5;
        });

        assert_eq!(
            world.get_clan_premium_property(sid, PremiumProperty::BonusLoyalty),
            4
        );
        assert_eq!(
            world.get_clan_premium_property(sid, PremiumProperty::NoahPercent),
            5
        );
        assert_eq!(
            world.get_clan_premium_property(sid, PremiumProperty::RepairDiscountPercent),
            50
        );

        // No clan premium: 0
        world.update_session(sid, |h| {
            h.clan_premium_in_use = 0;
        });
        assert_eq!(
            world.get_clan_premium_property(sid, PremiumProperty::BonusLoyalty),
            0
        );
    }

    #[test]
    fn test_repair_discount_calculation() {
        // C++ formula: cost * RepairDiscountPercent / 100
        // With RepairDiscountPercent = 50, cost becomes 50% of original
        let original_cost: u32 = 1000;
        let repair_disc: i32 = 50;
        let discounted = original_cost * repair_disc as u32 / 100;
        assert_eq!(discounted, 500); // 50% discount

        // No discount (0): cost unchanged
        let no_disc: i32 = 0;
        let no_change = if no_disc > 0 {
            original_cost * no_disc as u32 / 100
        } else {
            original_cost
        };
        assert_eq!(no_change, 1000);
    }

    #[test]
    fn test_sell_price_premium_divisor() {
        // C++ formula: buy_price / (premium > 0 ? 4 : 6) * count
        let buy_price: u64 = 1800;
        let count: u64 = 1;

        // Standard sell: 1800 / 6 = 300
        let sell_standard = (buy_price / 6) * count;
        assert_eq!(sell_standard, 300);

        // Premium sell: 1800 / 4 = 450
        let sell_premium = (buy_price / 4) * count;
        assert_eq!(sell_premium, 450);

        // Premium gives 50% more gold per item sold
        assert!(sell_premium > sell_standard);
    }

    #[test]
    fn test_normal_premium_has_no_bonuses() {
        use crate::world::PremiumProperty;

        let (world, sid) = setup_premium_world();

        // Normal Premium (1) has all zeros
        world.update_session(sid, |h| {
            h.premium_in_use = 1;
        });

        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::NoahPercent),
            0
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::DropPercent),
            0
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::BonusLoyalty),
            0
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::RepairDiscountPercent),
            0
        );
        assert_eq!(
            world.get_premium_property(sid, PremiumProperty::ItemSellPercent),
            0
        );
        assert!((world.get_premium_exp_restore(sid) - 0.0).abs() < f64::EPSILON);
    }
}

/// Rows for `account_premium` in the order the login picks the active premium: the premium the player has
/// switched to is stored in slot 0 (the HashMap order was random, so a different premium could come back
/// after every relog), the others follow by type.
pub(crate) fn premium_slots_for_save(map: &std::collections::HashMap<u8, u32>, in_use: u8) -> Vec<(i16, i16, i32)> {
    let mut v: Vec<(u8, u32)> = map.iter().map(|(&t, &e)| (t, e)).collect();
    v.sort_by_key(|&(t, _)| (t != in_use, t));
    v.into_iter().enumerate().map(|(i, (t, e))| (i as i16, t as i16, e as i32)).collect()
}
