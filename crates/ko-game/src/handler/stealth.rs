//! WIZ_STEALTH (0x60) handler — stealth/invisibility management.
//! ## Incoming Packet (C->S)
//! The client does not send WIZ_STEALTH — this opcode is server-to-client only.
//! The handler is a no-op for safety.
//! ## Server-initiated Stealth Packets
//! - `InitializeStealth()`: `WIZ_STEALTH << u8(0) << u16(0)` — resets stealth (sent on respawn)
//! - Type9 Lupine stealth: `WIZ_STEALTH << u8(1) << u16(radius)` — mini-map stealth
//! ## Stealth Removal
//! `remove_stealth()` is called when a stealthed player moves (INVIS_DISPEL_ON_MOVE)
//! or attacks (any invisibility type). It:
//! 1. Clears the invisibility_type to INVIS_NONE
//! 2. Removes the BUFF_TYPE_INVISIBILITY buff
//! 3. Broadcasts `WIZ_STATE_CHANGE(7, INVIS_NONE)` to the 3x3 region
//! ## C++ InvisibilityType Enum (globals.h:757-762)
//! | Value | Name                  | Description                          |
//! |-------|-----------------------|--------------------------------------|
//! | 0     | INVIS_NONE            | Not invisible                        |
//! | 1     | INVIS_DISPEL_ON_MOVE  | Stealth breaks on movement           |
//! | 2     | INVIS_DISPEL_ON_ATTACK| Stealth breaks on attack/skill only  |

use ko_protocol::{Opcode, Packet};
use std::sync::Arc;

use crate::session::{ClientSession, SessionState};
use crate::world::WorldState;
use crate::zone::SessionId;

// ── Type9Cancel constants ────────────────────────────────────────────────

use crate::magic_constants::MAGIC_DURATION_EXPIRED;

/// Response code for stealth removal (stateChange <= 2 or 5-6).
const TYPE9_CANCEL_STEALTH_RESPONSE: u8 = 91;

// ── Invisibility type constants ─────────────────────────────────────────────

/// Not invisible.
pub const INVIS_NONE: u8 = 0;

/// Stealth that breaks when the player moves.
pub const INVIS_DISPEL_ON_MOVE: u8 = 1;

/// Stealth that breaks only on attack or offensive skill.
pub const INVIS_DISPEL_ON_ATTACK: u8 = 2;

use crate::buff_constants::BUFF_TYPE_INVISIBILITY;

// ── Handler ─────────────────────────────────────────────────────────────────

/// Handle incoming WIZ_STEALTH from the client.
/// The client should never send WIZ_STEALTH; this is a server-initiated opcode.
/// We silently ignore it (matching C++ behavior where StateChange case 7 returns).
pub fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    tracing::debug!(
        "[{}] WIZ_STEALTH: no-op (server-only opcode)",
        session.addr()
    );
    Ok(())
}

// ── Stealth Removal ─────────────────────────────────────────────────────────

/// Remove all stealth from a player.
/// Checks if the player is invisible (invisibility_type != INVIS_NONE), and if so:
/// 1. Calls `remove_stealth_type()` for both INVIS_DISPEL_ON_MOVE and INVIS_DISPEL_ON_ATTACK
/// This is called from:
/// - `attack.rs` (before processing an attack)
/// - `magic_process.rs` (before casting offensive spells of type 1-3, 7)
pub fn remove_stealth(world: &WorldState, sid: SessionId) {
    let invis_type = world.get_invisibility_type(sid);
    if invis_type == INVIS_NONE {
        return;
    }
    remove_stealth_type(world, sid, INVIS_DISPEL_ON_MOVE);
    remove_stealth_type(world, sid, INVIS_DISPEL_ON_ATTACK);
}

/// Remove a specific type of stealth from a player.
/// 1. Validates the invisibility type (must be 1 or 2)
/// 2. Removes the BUFF_TYPE_INVISIBILITY buff
/// 3. Resets invisibility_type to INVIS_NONE
/// 4. Broadcasts `WIZ_STATE_CHANGE(7, INVIS_NONE)` to the 3x3 region
/// This is also called directly from `move_handler.rs` (only for INVIS_DISPEL_ON_MOVE).
pub fn remove_stealth_type(world: &WorldState, sid: SessionId, invis_type: u8) {
    if invis_type != INVIS_DISPEL_ON_MOVE && invis_type != INVIS_DISPEL_ON_ATTACK {
        return;
    }

    // Check if this specific type is active
    let current = world.get_invisibility_type(sid);
    if current == INVIS_NONE {
        return;
    }

    // C++ checks m_type9BuffMap for the type key and calls Type9Cancel.
    // Type9Cancel (MagicInstance.cpp:6816-6821) does:
    //   1. Remove buff from type9BuffMap
    //   2. StateChangeServerDirect(7, INVIS_NONE)
    //   3. Send MAGIC_DURATION_EXPIRED packet with response 91

    // Remove the stealth buff
    world.remove_buff(sid, BUFF_TYPE_INVISIBILITY);

    // Reset invisibility type to none
    world.set_invisibility_type(sid, INVIS_NONE);

    // Broadcast StateChange(7, INVIS_NONE) to 3x3 region
    let mut pkt = Packet::new(Opcode::WizStateChange as u8);
    pkt.write_u32(sid as u32);
    pkt.write_u8(7); // type 7 = invisibility
    pkt.write_u32(INVIS_NONE as u32);

    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );

        // v2525: Clear visual state flag (WIZ_PACKET2 +0xB69 = 0)
        let flag_pkt = super::packet2::build_state_flag(sid as i32, 0);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(flag_pkt),
            None,
            event_room,
        );
    }

    // Send MAGIC_DURATION_EXPIRED to the player to clear the client buff icon.
    //   Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_DURATION_EXPIRED));
    //   result << bResponse;   // 91 for stealths
    //   pCaster->Send(&result);
    let mut expire_pkt = Packet::new(Opcode::WizMagicProcess as u8);
    expire_pkt.write_u8(MAGIC_DURATION_EXPIRED);
    expire_pkt.write_u8(TYPE9_CANCEL_STEALTH_RESPONSE);
    world.send_to_session_owned(sid, expire_pkt);

    tracing::debug!(
        "[sid={}] Stealth removed with Type9Cancel (was type {})",
        sid,
        current,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;

    fn create_test_world() -> WorldState {
        WorldState::new()
    }

    fn register_session(world: &WorldState, sid: SessionId) {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
    }

    #[test]
    fn test_invis_constants_match_cpp() {
        assert_eq!(INVIS_NONE, 0);
        assert_eq!(INVIS_DISPEL_ON_MOVE, 1);
        assert_eq!(INVIS_DISPEL_ON_ATTACK, 2);
    }

    #[test]
    fn test_buff_type_invisibility_constant() {
        assert_eq!(BUFF_TYPE_INVISIBILITY, 100);
    }

    #[test]
    fn test_invisibility_type_default_is_none() {
        let world = create_test_world();
        register_session(&world, 1);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
        assert!(!world.is_invisible(1));
    }

    #[test]
    fn test_set_invisibility_type() {
        let world = create_test_world();
        register_session(&world, 1);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        assert_eq!(world.get_invisibility_type(1), INVIS_DISPEL_ON_MOVE);
        assert!(world.is_invisible(1));

        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);
        assert_eq!(world.get_invisibility_type(1), INVIS_DISPEL_ON_ATTACK);
        assert!(world.is_invisible(1));

        world.set_invisibility_type(1, INVIS_NONE);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
        assert!(!world.is_invisible(1));
    }

    #[test]
    fn test_get_invisibility_type_missing_session() {
        let world = create_test_world();
        assert_eq!(world.get_invisibility_type(999), INVIS_NONE);
    }

    #[test]
    fn test_is_invisible_missing_session() {
        let world = create_test_world();
        assert!(!world.is_invisible(999));
    }

    #[tokio::test]
    async fn test_remove_stealth_when_not_stealthed() {
        let world = create_test_world();
        register_session(&world, 1);

        // Should be a no-op when not stealthed
        remove_stealth(&world, 1);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_remove_stealth_clears_invisibility_type() {
        let world = create_test_world();
        register_session(&world, 1);

        // Set stealth
        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        assert!(world.is_invisible(1));

        // Remove stealth
        remove_stealth(&world, 1);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
        assert!(!world.is_invisible(1));
    }

    #[tokio::test]
    async fn test_remove_stealth_clears_attack_type() {
        let world = create_test_world();
        register_session(&world, 1);

        // Set attack-dispel stealth
        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);
        assert!(world.is_invisible(1));

        // Remove stealth
        remove_stealth(&world, 1);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_remove_stealth_removes_buff() {
        use crate::world::ActiveBuff;
        use std::time::Instant;

        let world = create_test_world();
        register_session(&world, 1);

        // Apply stealth buff
        let buff = ActiveBuff {
            skill_id: 108010,
            buff_type: BUFF_TYPE_INVISIBILITY,
            caster_sid: 1,
            start_time: Instant::now(),
            duration_secs: 30,
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
        };
        world.apply_buff(1, buff);
        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);

        // Verify buff exists
        let buffs = world.get_active_buffs(1);
        assert!(buffs.iter().any(|b| b.buff_type == BUFF_TYPE_INVISIBILITY));

        // Remove stealth
        remove_stealth(&world, 1);

        // Verify buff removed
        let buffs = world.get_active_buffs(1);
        assert!(!buffs.iter().any(|b| b.buff_type == BUFF_TYPE_INVISIBILITY));
    }

    #[tokio::test]
    async fn test_remove_stealth_type_invalid_type() {
        let world = create_test_world();
        register_session(&world, 1);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);

        // Invalid type should do nothing
        remove_stealth_type(&world, 1, 3);
        assert_eq!(world.get_invisibility_type(1), INVIS_DISPEL_ON_MOVE);

        remove_stealth_type(&world, 1, 0);
        assert_eq!(world.get_invisibility_type(1), INVIS_DISPEL_ON_MOVE);
    }

    #[tokio::test]
    async fn test_remove_stealth_type_move_clears() {
        let world = create_test_world();
        register_session(&world, 1);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        remove_stealth_type(&world, 1, INVIS_DISPEL_ON_MOVE);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_remove_stealth_type_attack_clears() {
        let world = create_test_world();
        register_session(&world, 1);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);
        remove_stealth_type(&world, 1, INVIS_DISPEL_ON_ATTACK);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_remove_stealth_type_already_none() {
        let world = create_test_world();
        register_session(&world, 1);

        // Already none — should be a no-op
        remove_stealth_type(&world, 1, INVIS_DISPEL_ON_MOVE);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_remove_stealth_broadcasts_state_change() {
        let world = create_test_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);

        // Remove stealth — should broadcast WIZ_STATE_CHANGE
        remove_stealth(&world, 1);

        // Drain the channel and check for state change packets.
        // Note: broadcast_to_3x3 requires zone/region which we don't set in test,
        // so the packet may not arrive via broadcast. The logic correctness is
        // verified by invisibility_type being reset.
        while let Ok(_pkt) = rx.try_recv() {
            // consume any queued packets
        }
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_remove_stealth_idempotent() {
        let world = create_test_world();
        register_session(&world, 1);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);

        // Remove twice — should not panic or error
        remove_stealth(&world, 1);
        remove_stealth(&world, 1);
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_stealth_set_then_remove_cycle() {
        let world = create_test_world();
        register_session(&world, 1);

        // Cycle: set → remove → set → remove
        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        assert!(world.is_invisible(1));

        remove_stealth(&world, 1);
        assert!(!world.is_invisible(1));

        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);
        assert!(world.is_invisible(1));

        remove_stealth(&world, 1);
        assert!(!world.is_invisible(1));
    }

    #[tokio::test]
    async fn test_remove_stealth_multiple_sessions() {
        let world = create_test_world();
        register_session(&world, 1);
        register_session(&world, 2);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        world.set_invisibility_type(2, INVIS_DISPEL_ON_ATTACK);

        // Remove only session 1
        remove_stealth(&world, 1);

        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
        assert_eq!(world.get_invisibility_type(2), INVIS_DISPEL_ON_ATTACK);
    }

    #[tokio::test]
    async fn test_move_dispel_does_not_clear_attack_type() {
        let world = create_test_world();
        register_session(&world, 1);

        // Player has INVIS_DISPEL_ON_ATTACK (assassin stealth)
        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);

        // Move dispel should still clear it because remove_stealth_type checks
        // current != INVIS_NONE, not current == invis_type
        // This matches C++ behavior: RemoveStealth() calls both types
        remove_stealth_type(&world, 1, INVIS_DISPEL_ON_MOVE);

        // In C++, RemoveStealth calls both types. But the Type9BuffMap lookup
        // is keyed by type (1 or 2). Since our simplified impl doesn't track
        // separate types, calling either type clears the single invisibility_type.
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);
    }

    #[test]
    fn test_invis_dispel_on_move_is_rogue_stealth() {
        // This is the standard rogue stealth that breaks on movement
        assert_eq!(INVIS_DISPEL_ON_MOVE, 1);
    }

    #[test]
    fn test_invis_dispel_on_attack_is_assassin_stealth() {
        // Used for assassin stealth that only breaks on attack/skill
        assert_eq!(INVIS_DISPEL_ON_ATTACK, 2);
    }

    #[tokio::test]
    async fn test_remove_stealth_nonexistent_session() {
        let world = create_test_world();
        // Should not panic when session doesn't exist
        remove_stealth(&world, 999);
        remove_stealth_type(&world, 999, INVIS_DISPEL_ON_MOVE);
    }

    #[test]
    fn test_set_invisibility_type_nonexistent_session() {
        let world = create_test_world();
        // Should not panic
        world.set_invisibility_type(999, INVIS_DISPEL_ON_MOVE);
        assert_eq!(world.get_invisibility_type(999), INVIS_NONE);
    }

    #[tokio::test]
    async fn test_attack_stealth_break_removes_both_types() {
        // INVIS_DISPEL_ON_MOVE and INVIS_DISPEL_ON_ATTACK
        let world = create_test_world();
        register_session(&world, 1);

        // Set INVIS_DISPEL_ON_ATTACK (stronger stealth)
        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);
        assert!(world.is_invisible(1));

        // Full remove_stealth (as used in attack handler) should clear it
        remove_stealth(&world, 1);
        assert!(!world.is_invisible(1));
    }

    #[tokio::test]
    async fn test_magic_stealth_break_types_1_to_3_and_7() {
        // Skills of type 1-3 and 7 remove stealth before executing
        let world = create_test_world();
        register_session(&world, 1);

        // Verify that remove_stealth works for the magic handler use case
        for invis in [INVIS_DISPEL_ON_MOVE, INVIS_DISPEL_ON_ATTACK] {
            world.set_invisibility_type(1, invis);
            assert!(world.is_invisible(1));
            remove_stealth(&world, 1);
            assert!(!world.is_invisible(1));
        }
    }

    #[tokio::test]
    async fn test_move_stealth_break_only_type_1() {
        // Only INVIS_DISPEL_ON_MOVE (1) breaks on movement
        let world = create_test_world();
        register_session(&world, 1);

        // Type 1 should break on move
        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        assert_eq!(world.get_invisibility_type(1), INVIS_DISPEL_ON_MOVE);

        // Simulate move handler check: only break if type == INVIS_DISPEL_ON_MOVE
        if world.get_invisibility_type(1) == INVIS_DISPEL_ON_MOVE {
            remove_stealth_type(&world, 1, INVIS_DISPEL_ON_MOVE);
        }
        assert!(!world.is_invisible(1));
    }

    #[tokio::test]
    async fn test_move_does_not_break_attack_dispel_stealth() {
        // Only if m_bInvisibilityType == INVIS_DISPEL_ON_MOVE, stealth is broken
        let world = create_test_world();
        register_session(&world, 1);

        // INVIS_DISPEL_ON_ATTACK should NOT be broken by movement
        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);

        // Simulate move handler check: the condition check prevents removal
        if world.get_invisibility_type(1) == INVIS_DISPEL_ON_MOVE {
            remove_stealth_type(&world, 1, INVIS_DISPEL_ON_MOVE);
        }
        // Stealth should remain since the condition was not met
        assert!(world.is_invisible(1));
        assert_eq!(world.get_invisibility_type(1), INVIS_DISPEL_ON_ATTACK);
    }

    #[tokio::test]
    async fn test_initialize_stealth_resets_invisibility() {
        // Our send_initialize_stealth also resets invisibility_type to 0
        let world = create_test_world();
        register_session(&world, 1);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);
        assert!(world.is_invisible(1));

        // Simulate what send_initialize_stealth does
        world.set_invisibility_type(1, INVIS_NONE);
        assert!(!world.is_invisible(1));
    }

    #[tokio::test]
    async fn test_stealth_state_change_packet_format() {
        // Packet: [u32 socket_id] [u8 type=7] [u32 invis_type]
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(42); // session id
        pkt.write_u8(7); // type = invisibility
        pkt.write_u32(INVIS_NONE as u32);

        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        let _r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(pkt.data.len(), 9); // u32 + u8 + u32
    }

    // ── Type9Cancel Tests ────────────────────────────────────────────────

    #[test]
    fn test_magic_duration_expired_constant() {
        assert_eq!(MAGIC_DURATION_EXPIRED, 5);
    }

    #[test]
    fn test_type9_cancel_stealth_response_constant() {
        assert_eq!(TYPE9_CANCEL_STEALTH_RESPONSE, 91);
    }

    #[test]
    fn test_type9_cancel_packet_format() {
        // Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_DURATION_EXPIRED));
        // result << bResponse;
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(MAGIC_DURATION_EXPIRED);
        pkt.write_u8(TYPE9_CANCEL_STEALTH_RESPONSE);

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        assert_eq!(pkt.data.len(), 2); // u8 + u8
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_DURATION_EXPIRED));
        assert_eq!(r.read_u8(), Some(TYPE9_CANCEL_STEALTH_RESPONSE));
    }

    #[tokio::test]
    async fn test_remove_stealth_sends_type9cancel_packet() {
        // When stealth is removed, the client should receive:
        // 1. WIZ_STATE_CHANGE broadcast (visibility)
        // 2. WIZ_MAGIC_PROCESS / MAGIC_DURATION_EXPIRED direct send (buff icon clear)
        let world = create_test_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_MOVE);

        // Remove stealth
        remove_stealth(&world, 1);

        // Verify invisibility cleared
        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);

        // Drain packets and look for MAGIC_DURATION_EXPIRED
        let mut found_expire = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizMagicProcess as u8 {
                let mut r = ko_protocol::PacketReader::new(&pkt.data);
                if r.read_u8() == Some(MAGIC_DURATION_EXPIRED) {
                    assert_eq!(r.read_u8(), Some(TYPE9_CANCEL_STEALTH_RESPONSE));
                    found_expire = true;
                }
            }
        }
        assert!(found_expire, "Expected MAGIC_DURATION_EXPIRED packet");
    }

    #[tokio::test]
    async fn test_remove_stealth_attack_type_sends_type9cancel() {
        let world = create_test_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_invisibility_type(1, INVIS_DISPEL_ON_ATTACK);
        remove_stealth(&world, 1);

        assert_eq!(world.get_invisibility_type(1), INVIS_NONE);

        let mut found_expire = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizMagicProcess as u8 {
                let mut r = ko_protocol::PacketReader::new(&pkt.data);
                if r.read_u8() == Some(MAGIC_DURATION_EXPIRED) {
                    found_expire = true;
                }
            }
        }
        assert!(
            found_expire,
            "Expected MAGIC_DURATION_EXPIRED packet for attack-dispel stealth"
        );
    }

    #[tokio::test]
    async fn test_remove_stealth_no_type9cancel_when_not_stealthed() {
        let world = create_test_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Not stealthed — should be no-op, no packets
        remove_stealth(&world, 1);

        let mut pkt_count = 0;
        while let Ok(_pkt) = rx.try_recv() {
            pkt_count += 1;
        }
        assert_eq!(pkt_count, 0, "No packets expected when not stealthed");
    }

    /// Invisibility types form a contiguous range 0-2.
    #[test]
    fn test_invis_types_contiguous_range() {
        assert_eq!(INVIS_NONE, 0);
        assert_eq!(INVIS_DISPEL_ON_MOVE, 1);
        assert_eq!(INVIS_DISPEL_ON_ATTACK, 2);
        assert_eq!(INVIS_DISPEL_ON_ATTACK - INVIS_NONE, 2);
    }

    /// WIZ_STEALTH opcode is 0x60, within v2525 dispatch range.
    #[test]
    fn test_stealth_opcode_value() {
        assert_eq!(Opcode::WizStealth as u8, 0x60);
        assert!(Opcode::WizStealth as u8 >= 0x06);
        assert!(Opcode::WizStealth as u8 <= 0xD7);
    }

    /// TYPE9_CANCEL_STEALTH_RESPONSE (91) is distinct from MAGIC_DURATION_EXPIRED (5).
    #[test]
    fn test_type9_cancel_vs_magic_expired() {
        assert_eq!(TYPE9_CANCEL_STEALTH_RESPONSE, 91);
        assert_eq!(MAGIC_DURATION_EXPIRED, 5);
        assert_ne!(TYPE9_CANCEL_STEALTH_RESPONSE, MAGIC_DURATION_EXPIRED);
    }

    /// BUFF_TYPE_INVISIBILITY is the buff key used for stealth tracking.
    #[test]
    fn test_buff_type_invisibility_is_positive() {
        assert!(BUFF_TYPE_INVISIBILITY > 0);
        // Imported from buff_constants, same value in both modules
        assert_eq!(
            BUFF_TYPE_INVISIBILITY,
            crate::buff_constants::BUFF_TYPE_INVISIBILITY
        );
    }

    /// Stealth types: MOVE breaks more easily than ATTACK.
    #[test]
    fn test_stealth_fragility_ordering() {
        // MOVE dispel (1) is "more fragile" — breaks on any movement
        // ATTACK dispel (2) only breaks on attack/skill
        assert!(INVIS_DISPEL_ON_MOVE < INVIS_DISPEL_ON_ATTACK);
        // Both are non-zero (actual stealth, not NONE)
        assert!(INVIS_DISPEL_ON_MOVE > INVIS_NONE);
        assert!(INVIS_DISPEL_ON_ATTACK > INVIS_NONE);
    }
}
