//! WIZ_TERRAIN_EFFECTS (0x83) — terrain effect notification (server→client only).
//! The server evaluates the player's position in ZONE_BATTLE6 (Oreads) during
//! nation battle and sends a terrain type to the client. The client uses this
//! to display visual effects and apply combat modifiers.
//! ## Terrain Types
//! | Value | Name  | Effect                                         |
//! |-------|-------|------------------------------------------------|
//! | 0     | None  | No terrain effect (clear previous)             |
//! | 1     | Hay   | Boosts fire magic, weakens ice and lightning    |
//! | 2     | Swamp | Boosts all magic, decreases movement speed      |
//! | 3     | Water | Weakens fire magic, boosts ice and lightning    |
//! ## Wire format (Server→Client)
//! `[u8 0x01] [u8 terrain_type]`
//! No client→server handling is needed for this opcode.

use ko_protocol::{Opcode, Packet};

use crate::world::ZONE_BATTLE6;

/// Terrain type: no effect.
pub const TERRAIN_NONE: u8 = 0;
/// Terrain type: hay — boosts fire magic, weakens ice and lightning.
pub const TERRAIN_HAY: u8 = 1;
/// Terrain type: swamp — boosts all magic, decreases movement speed.
pub const TERRAIN_SWAMP: u8 = 2;
/// Terrain type: water/ice — weakens fire magic, boosts ice and lightning.
pub const TERRAIN_WATER: u8 = 3;

/// Build a WIZ_TERRAIN_EFFECTS packet to send to the client.
/// Wire: `[u8 0x01] [u8 terrain_type]`
/// # Arguments
/// * `terrain_type` — One of `TERRAIN_NONE`, `TERRAIN_HAY`, `TERRAIN_SWAMP`, `TERRAIN_WATER`.
pub fn build_terrain_effects_packet(terrain_type: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizTerrainEffects as u8);
    pkt.write_u8(0x01);
    pkt.write_u8(terrain_type);
    pkt
}

/// Evaluate terrain type based on player position in ZONE_BATTLE6 (Oreads).
/// Returns `TERRAIN_NONE` if:
/// - Zone is not ZONE_BATTLE6
/// - Nation battle is not active
/// - Player is a GM
/// Otherwise checks rectangular terrain zones:
/// - HAY (1): 5 areas — boosts fire, weakens ice/lightning
/// - Swamp (2): 1 area — boosts all magic, slows movement
/// - Water (3): 1 area — weakens fire, boosts ice/lightning
pub fn evaluate_terrain(
    zone_id: u16,
    is_nation_battle: bool,
    is_gm: bool,
    pos_x: f32,
    pos_z: f32,
) -> u8 {
    // if (g_pMain->m_byBattleOpen != NATION_BATTLE || GetZoneID() != ZONE_BATTLE6 || isGM())
    if !is_nation_battle || zone_id != ZONE_BATTLE6 || is_gm {
        return TERRAIN_NONE;
    }

    // Helper: check if position is within a rectangular area
    let in_rect = |x_min: f32, x_max: f32, z_min: f32, z_max: f32| -> bool {
        (x_min..=x_max).contains(&pos_x) && (z_min..=z_max).contains(&pos_z)
    };

    if in_rect(531.0, 713.0, 447.0, 690.0)    // Area 1
        || in_rect(375.0, 527.0, 392.0, 612.0) // Area 2
        || in_rect(285.0, 423.0, 344.0, 569.0) // Area 3
        || in_rect(263.0, 319.0, 461.0, 537.0) // Area 4
        || in_rect(591.0, 669.0, 416.0, 493.0)
    // Area 5
    {
        return TERRAIN_HAY;
    }

    if in_rect(619.0, 984.0, 714.0, 970.0) {
        return TERRAIN_SWAMP;
    }

    if in_rect(138.0, 373.0, 62.0, 306.0) {
        return TERRAIN_WATER;
    }

    TERRAIN_NONE
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_terrain_none_packet() {
        let pkt = build_terrain_effects_packet(TERRAIN_NONE);
        assert_eq!(pkt.opcode, Opcode::WizTerrainEffects as u8);
        assert_eq!(pkt.opcode, 0x83);
        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x01));
        assert_eq!(r.read_u8(), Some(TERRAIN_NONE));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_terrain_hay_packet() {
        let pkt = build_terrain_effects_packet(TERRAIN_HAY);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x01));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_terrain_swamp_packet() {
        let pkt = build_terrain_effects_packet(TERRAIN_SWAMP);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x01));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_terrain_water_packet() {
        let pkt = build_terrain_effects_packet(TERRAIN_WATER);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x01));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_terrain_constants() {
        assert_eq!(TERRAIN_NONE, 0);
        assert_eq!(TERRAIN_HAY, 1);
        assert_eq!(TERRAIN_SWAMP, 2);
        assert_eq!(TERRAIN_WATER, 3);
    }

    // ── evaluate_terrain tests ──────────────────────────────────────

    #[test]
    fn test_evaluate_terrain_wrong_zone_returns_none() {
        // Not in ZONE_BATTLE6 — always NONE regardless of position
        assert_eq!(evaluate_terrain(1, true, false, 500.0, 500.0), TERRAIN_NONE);
    }

    #[test]
    fn test_evaluate_terrain_no_nation_battle_returns_none() {
        // In ZONE_BATTLE6 but nation battle not active
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, false, false, 500.0, 500.0),
            TERRAIN_NONE
        );
    }

    #[test]
    fn test_evaluate_terrain_gm_returns_none() {
        // GM in ZONE_BATTLE6 during nation battle — still NONE
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, true, 500.0, 500.0),
            TERRAIN_NONE
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_area1() {
        // Center of HAY Area 1: x=531-713, z=447-690
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 620.0, 550.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_area2() {
        // Center of HAY Area 2: x=375-527, z=392-612
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 450.0, 500.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_area3() {
        // Center of HAY Area 3: x=285-423, z=344-569
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 350.0, 450.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_area4() {
        // Center of HAY Area 4: x=263-319, z=461-537
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 290.0, 500.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_area5() {
        // Center of HAY Area 5: x=591-669, z=416-493
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 630.0, 450.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_swamp() {
        // Center of Swamp: x=619-984, z=714-970
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 800.0, 850.0),
            TERRAIN_SWAMP
        );
    }

    #[test]
    fn test_evaluate_terrain_water() {
        // Center of Water: x=138-373, z=62-306
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 250.0, 180.0),
            TERRAIN_WATER
        );
    }

    #[test]
    fn test_evaluate_terrain_none_in_battle6() {
        // In ZONE_BATTLE6 during nation battle, but outside all terrain zones
        // Position (50, 50) is outside all defined areas
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 50.0, 50.0),
            TERRAIN_NONE
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_boundary_min() {
        // HAY Area 1 lower-left corner (boundary inclusive)
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 531.0, 447.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_boundary_max() {
        // HAY Area 1 upper-right corner (boundary inclusive)
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 713.0, 690.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_swamp_boundary() {
        // Swamp boundary corners
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 619.0, 714.0),
            TERRAIN_SWAMP
        );
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 984.0, 970.0),
            TERRAIN_SWAMP
        );
    }

    #[test]
    fn test_evaluate_terrain_water_boundary() {
        // Water boundary corners
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 138.0, 62.0),
            TERRAIN_WATER
        );
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 373.0, 306.0),
            TERRAIN_WATER
        );
    }

    #[test]
    fn test_evaluate_terrain_just_outside_hay_area1() {
        // Just outside HAY Area 1 boundary
        assert_ne!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 530.0, 550.0),
            TERRAIN_HAY
        );
        assert_ne!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 714.0, 550.0),
            TERRAIN_HAY
        );
    }

    #[test]
    fn test_evaluate_terrain_hay_priority_over_swamp() {
        // HAY areas are checked before swamp — verify priority in overlap region
        // There's no actual overlap in C++ data, but test that HAY is checked first
        // Position in HAY Area 1 that doesn't overlap with swamp
        assert_eq!(
            evaluate_terrain(ZONE_BATTLE6, true, false, 650.0, 500.0),
            TERRAIN_HAY
        );
    }
}
