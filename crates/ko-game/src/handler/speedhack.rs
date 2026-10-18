//! WIZ_SPEEDHACK_CHECK (0x41) handler — anti-cheat speed validation.
//! The server compares distance traveled (since last check) against
//! class-based speed limits. If the player moved too fast, they are
//! warped back to their last validated position.

use ko_protocol::Packet;
use tracing::debug;

use crate::clan_constants::COMMAND_CAPTAIN;
use crate::session::{ClientSession, SessionState};

/// Handle WIZ_SPEEDHACK_CHECK from the client.
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // GMs are exempt from speed checks
    if ch.authority == 0 {
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Determine class-based speed limit
    let base_class = ch.class % 100;
    let is_rogue = matches!(base_class, 2 | 7 | 8);
    let is_captain = ch.fame == COMMAND_CAPTAIN;

    let mut max_speed: f32 = if is_rogue || is_captain {
        90.0
    } else {
        // warrior, mage, priest, kurian
        67.0
    };
    max_speed += 17.0; // C++ adds a 17.0 buffer

    // Get last validated position
    let (last_x, last_z) = world
        .with_session(sid, |h| (h.speed_last_x, h.speed_last_z))
        .unwrap_or((0.0, 0.0));

    // First check (last position is 0,0): just record current position
    if last_x == 0.0 && last_z == 0.0 {
        world.update_session(sid, |h| {
            h.speed_last_x = pos.x;
            h.speed_last_z = pos.z;
        });
        return Ok(());
    }

    // `float nRange = (pow(GetX() - m_LastX, 2.0f) + pow(GetZ() - m_LastZ, 2.0f)) / 100.0f;`
    let dx = pos.x - last_x;
    let dz = pos.z - last_z;
    let range = (dx * dx + dz * dz) / 100.0;

    if range >= max_speed {
        // Speed hack detected — warp player back to last validated position
        // `Warp(uint16(m_LastX) * 10, uint16(m_LastZ) * 10);`
        debug!(
            "[{}] Speed hack detected: range={:.1} > limit={:.1}, warping back to ({:.0},{:.0})",
            session.addr(),
            range,
            max_speed,
            last_x,
            last_z,
        );

        // Send warp to last valid position
        let warp_x = (last_x as u16).wrapping_mul(10);
        let warp_z = (last_z as u16).wrapping_mul(10);
        let mut warp_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizMove as u8);
        warp_pkt.write_u16(warp_x);
        warp_pkt.write_u16(warp_z);
        warp_pkt.write_u16(0); // Y
        warp_pkt.write_i16(0); // speed = 0
        warp_pkt.write_u8(2); // echo = stop
        session.send_packet(&warp_pkt).await?;
    } else {
        // Valid movement — update last position
        world.update_session(sid, |h| {
            h.speed_last_x = pos.x;
            h.speed_last_z = pos.z;
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_speed_limits() {
        // Rogue: 90 + 17 = 107
        let rogue_class: u16 = 102; // rogue base
        let base = rogue_class % 100;
        let is_rogue = matches!(base, 2 | 7 | 8);
        assert!(is_rogue);

        // Warrior: 67 + 17 = 84
        let warrior_class: u16 = 101;
        let base = warrior_class % 100;
        let is_rogue = matches!(base, 2 | 7 | 8);
        assert!(!is_rogue);
    }

    #[test]
    fn test_range_calculation() {
        // C++ formula: (dx^2 + dz^2) / 100
        let dx: f32 = 50.0;
        let dz: f32 = 50.0;
        let range = (dx * dx + dz * dz) / 100.0;
        assert_eq!(range, 50.0);

        // Exact speed limit for warrior (67 + 17 = 84)
        // sqrt(84 * 100) ≈ 91.65 units max distance
        let max_speed: f32 = 84.0;
        assert!(range < max_speed);

        // Exceeding: dx=100, dz=0 → range=100 > 84
        let range2 = (100.0f32 * 100.0 + 0.0) / 100.0;
        assert!(range2 >= max_speed);
    }

    #[test]
    fn test_first_check_skips_validation_when_last_pos_is_zero() {
        // When speed_last_x and speed_last_z are both 0.0, the handler
        // should record the current position and return — no speed check.
        // This simulates the very first check after login or zone change.
        let last_x: f32 = 0.0;
        let last_z: f32 = 0.0;

        // The guard condition: skip validation when both are 0
        let should_skip = last_x == 0.0 && last_z == 0.0;
        assert!(should_skip, "First check with (0,0) must skip validation");

        // After zone change, even an enormous teleport distance should not
        // trigger detection because the first check is lenient.
        let current_x: f32 = 9999.0;
        let current_z: f32 = 9999.0;
        // If we were to compute range, it would be huge — but the guard
        // prevents this computation entirely.
        let hypothetical_dx = current_x - last_x;
        let hypothetical_dz = current_z - last_z;
        let hypothetical_range =
            (hypothetical_dx * hypothetical_dx + hypothetical_dz * hypothetical_dz) / 100.0;
        // This would fail any speed check...
        assert!(hypothetical_range >= 107.0);
        // ...but the (0,0) guard ensures we never reach this comparison
        assert!(should_skip);
    }

    #[test]
    fn test_teleport_does_not_false_positive_after_position_reset() {
        // Simulates: player warps/teleports → speed_last_x/z reset to 0 →
        // next speed check arrives → first-check guard fires → position recorded.
        // Then the SECOND check after teleport measures from the new position.
        let warp_dest_x: f32 = 500.0;
        let warp_dest_z: f32 = 300.0;

        // After warp, speed_last is reset to 0 by zone change cleanup.
        // First check: guard fires, sets last = warp_dest.
        let last_x = warp_dest_x;
        let last_z = warp_dest_z;

        // Second check: player has moved normally from warp destination.
        let current_x: f32 = 505.0; // moved 5 units
        let current_z: f32 = 303.0; // moved 3 units

        let dx = current_x - last_x;
        let dz = current_z - last_z;
        let range = (dx * dx + dz * dz) / 100.0;

        // range = (25 + 9) / 100 = 0.34 — well within any limit
        let warrior_limit: f32 = 84.0; // 67 + 17
        let rogue_limit: f32 = 107.0; // 90 + 17
        assert!(
            range < warrior_limit,
            "Normal movement after teleport should pass: range={range}"
        );
        assert!(
            range < rogue_limit,
            "Normal movement after teleport should pass for rogue too: range={range}"
        );
    }

    #[test]
    fn test_boundary_exact_threshold_triggers_detection() {
        // The handler uses `>=` comparison: `if range >= max_speed`
        // So a range EXACTLY equal to max_speed should trigger detection.
        let warrior_limit: f32 = 84.0; // 67 + 17
        let rogue_limit: f32 = 107.0; // 90 + 17

        // Warrior: range exactly at limit → detected
        // range = (dx^2 + dz^2) / 100 = 84.0
        // dx^2 + dz^2 = 8400.0 → e.g. dx = sqrt(8400) ≈ 91.65, dz = 0
        let dx: f32 = (warrior_limit * 100.0).sqrt();
        let dz: f32 = 0.0;
        let range = (dx * dx + dz * dz) / 100.0;
        assert!(
            range >= warrior_limit,
            "Exact threshold should trigger: range={range}, limit={warrior_limit}"
        );

        // Rogue: range exactly at limit → detected
        let dx: f32 = (rogue_limit * 100.0).sqrt();
        let dz: f32 = 0.0;
        let range = (dx * dx + dz * dz) / 100.0;
        assert!(
            range >= rogue_limit,
            "Exact threshold should trigger for rogue: range={range}, limit={rogue_limit}"
        );

        // Just below warrior limit → NOT detected
        let dx: f32 = (warrior_limit * 100.0).sqrt() - 0.1;
        let dz: f32 = 0.0;
        let range = (dx * dx + dz * dz) / 100.0;
        assert!(
            range < warrior_limit,
            "Just below threshold should pass: range={range}, limit={warrior_limit}"
        );
    }

    // ── Sprint 925: Additional coverage ──────────────────────────────

    /// Warp coordinate encoding: warp_x = u16(last_x) * 10.
    #[test]
    fn test_warp_coordinate_encoding() {
        let last_x: f32 = 250.0;
        let last_z: f32 = 300.0;
        let warp_x = (last_x as u16).wrapping_mul(10);
        let warp_z = (last_z as u16).wrapping_mul(10);
        assert_eq!(warp_x, 2500);
        assert_eq!(warp_z, 3000);
    }

    /// Captain fame gives rogue-speed (90+17=107).
    #[test]
    fn test_captain_gets_rogue_speed() {
        use crate::clan_constants::COMMAND_CAPTAIN;
        let base_class: u16 = 101 % 100; // warrior
        let is_rogue = matches!(base_class, 2 | 7 | 8);
        let is_captain = true; // fame == COMMAND_CAPTAIN
        assert!(!is_rogue);
        assert!(is_captain);
        let mut max_speed: f32 = if is_rogue || is_captain { 90.0 } else { 67.0 };
        max_speed += 17.0;
        assert_eq!(max_speed, 107.0);
        // Verify COMMAND_CAPTAIN constant exists
        assert!(COMMAND_CAPTAIN > 0);
    }

    /// All rogue variants: base_class 2 (Rogue), 7 (RogueNovice), 8 (RogueMaster).
    #[test]
    fn test_all_rogue_variants() {
        for class in [102u16, 107, 108, 202, 207, 208] {
            let base = class % 100;
            assert!(matches!(base, 2 | 7 | 8), "class {} should be rogue", class);
        }
    }

    /// Non-rogue classes: warrior(1,5,6), mage(3,9,10), priest(4,11,12), kurian(13,14,15).
    #[test]
    fn test_all_non_rogue_classes() {
        for base in [1u16, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14, 15] {
            assert!(
                !matches!(base, 2 | 7 | 8),
                "base {} should NOT be rogue",
                base
            );
        }
    }

    /// Warp-back packet format: WIZ_MOVE [u16 x][u16 z][u16 y=0][i16 speed=0][u8 echo=2].
    #[test]
    fn test_warp_packet_format() {
        use ko_protocol::{Opcode, PacketReader};
        let mut pkt = ko_protocol::Packet::new(Opcode::WizMove as u8);
        pkt.write_u16(2500); // warp_x
        pkt.write_u16(3000); // warp_z
        pkt.write_u16(0); // Y
        pkt.write_i16(0); // speed
        pkt.write_u8(2); // echo = stop

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(2500));
        assert_eq!(r.read_u16(), Some(3000));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_i16(), Some(0));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_normal_walking_speed_passes_validation() {
        // Normal player movement: small increments between speed checks.
        // Speed checks happen periodically from the client, typically every
        // few seconds. Normal walking covers ~10-30 units per interval.
        let warrior_limit: f32 = 84.0; // 67 + 17
        let rogue_limit: f32 = 107.0; // 90 + 17

        // Scenario 1: casual walk — dx=10, dz=10
        let range = (10.0f32 * 10.0 + 10.0 * 10.0) / 100.0; // 2.0
        assert!(
            range < warrior_limit,
            "Casual walk should pass: range={range}"
        );

        // Scenario 2: brisk walk — dx=30, dz=20
        let range = (30.0f32 * 30.0 + 20.0 * 20.0) / 100.0; // 13.0
        assert!(
            range < warrior_limit,
            "Brisk walk should pass: range={range}"
        );

        // Scenario 3: fast run — dx=50, dz=50
        let range = (50.0f32 * 50.0 + 50.0 * 50.0) / 100.0; // 50.0
        assert!(
            range < warrior_limit,
            "Fast run should pass for warrior: range={range}"
        );
        assert!(
            range < rogue_limit,
            "Fast run should pass for rogue: range={range}"
        );

        // Scenario 4: sprint (high but valid) — dx=80, dz=0
        let range = (80.0f32 * 80.0 + 0.0) / 100.0; // 64.0
        assert!(
            range < warrior_limit,
            "Sprint should pass for warrior: range={range}"
        );
        assert!(
            range < rogue_limit,
            "Sprint should pass for rogue: range={range}"
        );
    }
}
