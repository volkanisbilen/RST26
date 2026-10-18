//! WIZ_MAX_HP_CHANGE (0x92) handler — Max HP update + visual effect system.
//!
//! Dual-purpose opcode: sub=2 updates max HP, sub=1 manages visual effects.
//!
//! ## Client RE
//!
//! - sub=2 handler: `0x7C1C60` — reads `[u8=1][i32 padding][i32 max_hp]`,
//!   calls `0x76E330` (SetMaxHp), updates HP bar.
//! - sub=1 handler: `0x7C1D46` → `0x709AF0` — manages visual effects on
//!   `player+0xA84` (buff/debuff visual overlay system).
//!
//! ## S2C Packet Format
//!
//! ```text
//! Sub=2: Max HP Update
//!   [u8 sub=2] [u8 type=1] [i32 padding=0] [i32 max_hp]
//!
//! Sub=1: Visual Effect
//!   [u8 sub=1] [u8 type=1] [i32 owner_id] [u16 effect_id]
//!   [u16 effect_param] [u8 effect_group] [i32 effect_key]
//! ```
//!
//! ## C2S Packets
//!
//! S2C-only — client never sends this opcode.

use ko_protocol::{Opcode, Packet};

// ── S2C Builders ──────────────────────────────────────────────────────

/// Build a max HP update packet (sub=2).
///
/// Client calls `SetMaxHp` and updates the HP bar visual.
///
/// - `max_hp`: New maximum HP value
///
/// Wire: `[u8 sub=2][u8 type=1][i32 pad=0][i32 max_hp]`
pub fn build_max_hp_change(max_hp: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMaxHpChange as u8);
    pkt.write_u8(2); // sub-opcode
    pkt.write_u8(1); // type (must be 1)
    pkt.write_i32(0); // padding
    pkt.write_i32(max_hp);
    pkt
}

/// Build a visual effect packet (sub=1).
///
/// Manages buff/debuff visual overlays on `player+0xA84`.
///
/// - `owner_id`: Entity that owns the effect (stored at +0x168)
/// - `effect_id`: Effect definition lookup key (sign-extended i16)
/// - `effect_param`: Custom param (0 = use default from effect table)
/// - `effect_group`: Duplicate detection group (replaces old in group)
/// - `effect_key`: Direct tree key (0 = auto-search by effect_id)
///
/// Wire: `[u8 sub=1][u8 type=1][i32 owner_id][u16 effect_id]
///        [u16 effect_param][u8 effect_group][i32 effect_key]`
pub fn build_visual_effect(
    owner_id: i32,
    effect_id: u16,
    effect_param: u16,
    effect_group: u8,
    effect_key: i32,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMaxHpChange as u8);
    pkt.write_u8(1); // sub-opcode
    pkt.write_u8(1); // type (must be 1)
    pkt.write_i32(owner_id);
    pkt.write_u16(effect_id);
    pkt.write_u16(effect_param);
    pkt.write_u8(effect_group);
    pkt.write_i32(effect_key);
    pkt
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_build_max_hp_change_opcode() {
        let pkt = build_max_hp_change(5000);
        assert_eq!(pkt.opcode, Opcode::WizMaxHpChange as u8);
    }

    #[test]
    fn test_build_max_hp_change_format() {
        let pkt = build_max_hp_change(5000);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub
        assert_eq!(r.read_u8(), Some(1)); // type
        assert_eq!(r.read_i32(), Some(0)); // padding
        assert_eq!(r.read_i32(), Some(5000)); // max_hp
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_max_hp_change_data_length() {
        // u8 + u8 + i32 + i32 = 10
        assert_eq!(build_max_hp_change(0).data.len(), 10);
    }

    #[test]
    fn test_build_visual_effect_opcode() {
        let pkt = build_visual_effect(1, 0, 0, 0, 0);
        assert_eq!(pkt.opcode, Opcode::WizMaxHpChange as u8);
    }

    #[test]
    fn test_build_visual_effect_format() {
        let pkt = build_visual_effect(10000, 8500, 0, 1, 42);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub
        assert_eq!(r.read_u8(), Some(1)); // type
        assert_eq!(r.read_i32(), Some(10000)); // owner_id
        assert_eq!(r.read_u16(), Some(8500)); // effect_id
        assert_eq!(r.read_u16(), Some(0)); // effect_param
        assert_eq!(r.read_u8(), Some(1)); // effect_group
        assert_eq!(r.read_i32(), Some(42)); // effect_key
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_visual_effect_data_length() {
        // u8 + u8 + i32 + u16 + u16 + u8 + i32 = 15
        assert_eq!(build_visual_effect(0, 0, 0, 0, 0).data.len(), 15);
    }

    #[test]
    fn test_build_max_hp_change_max_value() {
        let pkt = build_max_hp_change(i32::MAX);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_i32();
        assert_eq!(r.read_i32(), Some(i32::MAX));
    }

    // ── Sprint 930: Additional coverage ──────────────────────────────

    /// Opcode value is 0x92.
    #[test]
    fn test_max_hp_change_opcode_value() {
        assert_eq!(Opcode::WizMaxHpChange as u8, 0x92);
    }

    /// Max HP sub-opcode is always 2; visual effect is always 1.
    #[test]
    fn test_max_hp_change_sub_opcodes() {
        let hp_pkt = build_max_hp_change(1000);
        assert_eq!(hp_pkt.data[0], 2, "max hp sub = 2");

        let vfx_pkt = build_visual_effect(1, 100, 0, 0, 0);
        assert_eq!(vfx_pkt.data[0], 1, "visual effect sub = 1");
    }

    /// Max HP with zero value (dead player edge case).
    #[test]
    fn test_max_hp_change_zero_hp() {
        let pkt = build_max_hp_change(0);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_i32();
        assert_eq!(r.read_i32(), Some(0));
    }

    /// Visual effect with all max field values.
    #[test]
    fn test_visual_effect_max_values() {
        let pkt = build_visual_effect(i32::MAX, u16::MAX, u16::MAX, u8::MAX, i32::MAX);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_i32(), Some(i32::MAX));
        assert_eq!(r.read_u16(), Some(u16::MAX));
        assert_eq!(r.read_u16(), Some(u16::MAX));
        assert_eq!(r.read_u8(), Some(u8::MAX));
        assert_eq!(r.read_i32(), Some(i32::MAX));
        assert_eq!(r.remaining(), 0);
    }

    /// Type field is always 1 for both sub-opcodes.
    #[test]
    fn test_max_hp_change_type_always_one() {
        let hp = build_max_hp_change(5000);
        assert_eq!(hp.data[1], 1, "type=1 for max hp");

        let vfx = build_visual_effect(42, 100, 0, 0, 0);
        assert_eq!(vfx.data[1], 1, "type=1 for visual effect");
    }
}
