//! WIZ_OBJECT_EVENT object type constants.
//! These values are used as the first byte of WIZ_OBJECT_EVENT packets
//! to identify the type of map object that triggered the event.

/// Bind point object (save/set bind location).
pub const OBJECT_BIND: u8 = 0;

/// Gate object (standard gate).
pub const OBJECT_GATE: u8 = 1;

/// Gate variant 2 (secondary gate type).
pub const OBJECT_GATE2: u8 = 2;

/// Gate lever (mechanism to open/close gates).
pub const OBJECT_GATE_LEVER: u8 = 3;

/// Flag lever (flag/banner mechanism, default fallback type).
pub const OBJECT_FLAG_LEVER: u8 = 4;

/// Warp gate (teleport gate for zone/area transitions).
pub const OBJECT_WARP_GATE: u8 = 5;

/// Wall object (destructible/movable wall).
pub const OBJECT_WALL: u8 = 6;

/// Remove bind (clear bind point).
pub const OBJECT_REMOVE_BIND: u8 = 7;

/// Anvil (item upgrade station).
pub const OBJECT_ANVIL: u8 = 8;

/// Artifact (Bifrost artifact, castle siege artifact).
pub const OBJECT_ARTIFACT: u8 = 9;

/// NPC visual effect (ShowNpcEffect).
pub const OBJECT_NPC: u8 = 11;

/// Krowaz gate (dungeon gate requiring keys).
pub const OBJECT_KROWAZ_GATE: u8 = 12;

/// Poison gas object.
pub const OBJECT_POISON_GAS: u8 = 13;

/// Wood/burning log object.
pub const OBJECT_WOOD: u8 = 14;

/// Wood lever (mechanism for wood objects).
pub const OBJECT_WOOD_LEVER: u8 = 15;

/// Visual effect object (client-only rendering).
pub const OBJECT_EFFECT: u8 = 50;

#[cfg(test)]
mod tests {
    use super::*;

    /// Sequential range 0-9 for core object types.
    #[test]
    fn test_core_object_types_sequential() {
        assert_eq!(OBJECT_BIND, 0);
        assert_eq!(OBJECT_GATE, 1);
        assert_eq!(OBJECT_GATE2, 2);
        assert_eq!(OBJECT_GATE_LEVER, 3);
        assert_eq!(OBJECT_FLAG_LEVER, 4);
        assert_eq!(OBJECT_WARP_GATE, 5);
        assert_eq!(OBJECT_WALL, 6);
        assert_eq!(OBJECT_REMOVE_BIND, 7);
        assert_eq!(OBJECT_ANVIL, 8);
        assert_eq!(OBJECT_ARTIFACT, 9);
    }

    /// Extended object types (11+).
    #[test]
    fn test_extended_object_types() {
        assert_eq!(OBJECT_NPC, 11);
        assert_eq!(OBJECT_KROWAZ_GATE, 12);
        assert_eq!(OBJECT_POISON_GAS, 13);
        assert_eq!(OBJECT_WOOD, 14);
        assert_eq!(OBJECT_WOOD_LEVER, 15);
    }

    /// OBJECT_EFFECT is at 50 (gap from 15).
    #[test]
    fn test_effect_type_at_50() {
        assert_eq!(OBJECT_EFFECT, 50);
        assert!(OBJECT_EFFECT > OBJECT_WOOD_LEVER);
    }

    /// Bind/unbind pair: 0 and 7.
    #[test]
    fn test_bind_unbind_pair() {
        assert_eq!(OBJECT_BIND, 0);
        assert_eq!(OBJECT_REMOVE_BIND, 7);
    }

    /// Gate-related types are distinct.
    #[test]
    fn test_gate_types_distinct() {
        let gates = [
            OBJECT_GATE,
            OBJECT_GATE2,
            OBJECT_GATE_LEVER,
            OBJECT_WARP_GATE,
            OBJECT_KROWAZ_GATE,
        ];
        for i in 0..gates.len() {
            for j in (i + 1)..gates.len() {
                assert_ne!(gates[i], gates[j]);
            }
        }
    }

    // ── Sprint 939: Additional coverage ──────────────────────────────

    /// Lever types: GATE_LEVER(3), FLAG_LEVER(4), WOOD_LEVER(15).
    #[test]
    fn test_lever_types() {
        assert_eq!(OBJECT_GATE_LEVER, 3);
        assert_eq!(OBJECT_FLAG_LEVER, 4);
        assert_eq!(OBJECT_WOOD_LEVER, 15);
    }

    /// Destructible objects: WALL(6), WOOD(14).
    #[test]
    fn test_destructible_objects() {
        assert_eq!(OBJECT_WALL, 6);
        assert_eq!(OBJECT_WOOD, 14);
    }

    /// Poison gas and anvil types.
    #[test]
    fn test_special_objects() {
        assert_eq!(OBJECT_POISON_GAS, 13);
        assert_eq!(OBJECT_ANVIL, 8);
        assert_eq!(OBJECT_ARTIFACT, 9);
    }

    /// No gap in 0-9 range (10 values exist).
    #[test]
    fn test_no_gap_in_core_range() {
        let core = [
            OBJECT_BIND,
            OBJECT_GATE,
            OBJECT_GATE2,
            OBJECT_GATE_LEVER,
            OBJECT_FLAG_LEVER,
            OBJECT_WARP_GATE,
            OBJECT_WALL,
            OBJECT_REMOVE_BIND,
            OBJECT_ANVIL,
            OBJECT_ARTIFACT,
        ];
        assert_eq!(core.len(), 10);
        for (i, val) in core.iter().enumerate() {
            assert_eq!(*val, i as u8);
        }
    }

    /// NPC type (11) skips value 10.
    #[test]
    fn test_npc_skips_10() {
        assert_eq!(OBJECT_NPC, 11);
        assert!(OBJECT_NPC > OBJECT_ARTIFACT + 1);
    }
}
