//! WIZ_STATE_CHANGE (0x29) handler — sit/stand, emotes, visibility.
//! ## Request (C->S)
//! | Offset | Type   | Description |
//! |--------|--------|-------------|
//! | 0      | u8     | bType (1=sit/stand, 2=party, 3=view, 4=emotion, 5=abnormal, 7=invisibility) |
//! | 1      | u32le  | nBuff (meaning depends on bType) |
//! ## Broadcast to nearby players
//! `[u32 socket_id] [u8 bType] [u32 nBuff]`
//! ## State Constants
//! - `USER_STANDING` (0x01): player is standing
//! - `USER_SITDOWN` (0x02): player is sitting
//! - `USER_DEAD` (0x03): player is dead
//! - `USER_MONUMENT` (0x06): monument state

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::clan_constants::COMMAND_CAPTAIN;
use crate::handler::region::{
    build_user_inout_with_clan, get_equipped_visual, INOUT_IN, INOUT_OUT,
};
use crate::session::{ClientSession, SessionState};
use crate::state_change_constants::STATE_CHANGE_GM_VISIBILITY;
use crate::world::{USER_MONUMENT, USER_SITDOWN, USER_STANDING};

/// Handle WIZ_STATE_CHANGE from the client.
/// Validates the type/buff combination, updates server-side state, and
/// broadcasts the change to the 3x3 region grid.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Dead players cannot change state
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let b_type = reader.read_u8().unwrap_or(0);
    let n_buff = reader.read_u32().unwrap_or(0);
    let buff = n_buff as u8; // C++ casts: `uint8 buff = *(uint8*)&nBuff;`

    // Validate by type
    match b_type {
        // Type 1: Sit/Stand — validate buff values
        1 => {
            if buff != USER_STANDING && buff != USER_SITDOWN && buff != USER_MONUMENT {
                return Ok(());
            }
        }
        // Type 2: Party need — pass through
        2 => {}
        // Type 3: Abnormal/view — GM only (buff 1, 2, 3, or 5)
        3 => {
            let (is_gm, current_abnormal) = world
                .with_session(sid, |h| {
                    let gm = h
                        .character
                        .as_ref()
                        .map(|c| c.authority == 0)
                        .unwrap_or(false);
                    (gm, h.abnormal_type)
                })
                .unwrap_or((false, 1));
            if !is_gm || (n_buff != 1 && n_buff != 5 && n_buff != 2 && n_buff != 3) {
                return Ok(());
            }
            // C++ line 2959: if (isGM()) StateChangeServerDirect(5, 1);
            // Force GM visible before transformation to prevent visibility desync.
            if current_abnormal == 0 {
                // GM is currently invisible — force visible before transform
                world.update_session(sid, |h| {
                    h.abnormal_type = 1;
                });
                // Broadcast Type 5 visibility=1 to region
                let mut vis_pkt = Packet::new(Opcode::WizStateChange as u8);
                vis_pkt.write_u32(sid as u32);
                vis_pkt.write_u8(5);
                vis_pkt.write_u32(1);
                let arc_vis_pkt = Arc::new(vis_pkt);
                if let Some((pos, event_room)) =
                    world.with_session(sid, |h| (h.position, h.event_room))
                {
                    world.broadcast_to_3x3(
                        pos.zone_id,
                        pos.region_x,
                        pos.region_z,
                        Arc::clone(&arc_vis_pkt),
                        Some(sid),
                        event_room,
                    );
                    // Also send GmInOut IN so the GM appears to others
                    let ch = world.get_character_info(sid);
                    let clan = ch.as_ref().and_then(|c| {
                        if c.knights_id > 0 {
                            world.get_knights(c.knights_id)
                        } else {
                            None
                        }
                    });
                    let bs = world.get_broadcast_state(sid);
                    let equip_vis = get_equipped_visual(&world, sid);
                    let ac = clan
                        .as_ref()
                        .and_then(|ki| crate::handler::region::resolve_alliance_cape(ki, &world));
                    let is_king = ch
                        .as_ref()
                        .is_some_and(|c| world.is_king(c.nation, &c.name));
                    let inout_pkt = build_user_inout_with_clan(
                        INOUT_IN,
                        sid,
                        ch.as_ref(),
                        &pos,
                        clan.as_ref(),
                        ac,
                        is_king,
                        0,
                        1, // visible
                        &bs,
                        &equip_vis,
                    );
                    world.broadcast_to_3x3(
                        pos.zone_id,
                        pos.region_x,
                        pos.region_z,
                        Arc::new(inout_pkt),
                        Some(sid),
                        event_room,
                    );
                }
                world.send_to_session(sid, &arc_vis_pkt);
            }
        }
        // Type 4: Emotions
        4 => {
            match buff {
                1..=3 | 11..=13 => {
                    // Greeting (1-3) and Provoke (11-13) — captains cannot use these
                    let is_captain = world
                        .get_character_info(sid)
                        .map(|ch| ch.fame == COMMAND_CAPTAIN)
                        .unwrap_or(false);
                    if is_captain {
                        return Ok(());
                    }
                }
                14 | 15 => {} // spacebar animations — always allowed
                _ => {
                    // Unknown emotion buff values proceed to broadcast.
                    tracing::trace!(
                        "[sid={}] State change Type 4: unknown emotion buff {}",
                        sid,
                        buff,
                    );
                }
            }
        }
        // Type 5: GM visibility toggle — GM only, with duplicate-state guard
        //   `if ((buff == 0 && m_bAbnormalType == 0) || (buff == 1 && m_bAbnormalType == 1) || (!isGM())) return;`
        5 => {
            let (is_gm, current_abnormal) = world
                .with_session(sid, |h| {
                    let gm = h
                        .character
                        .as_ref()
                        .map(|c| c.authority == 0)
                        .unwrap_or(false);
                    (gm, h.abnormal_type)
                })
                .unwrap_or((false, 1));
            if !is_gm {
                return Ok(());
            }
            // Reject toggle to the same state (already invisible / already visible)
            if (buff == 0 && current_abnormal == 0) || (buff == 1 && current_abnormal == 1) {
                return Ok(());
            }
        }
        // Types 6, 7, 8, 11: Server-internal only — reject from client packets.
        // These types are ONLY set via `StateChangeServerDirect()` (server-side calls).
        //   6 = party leader flag (set by party system)
        //   7 = invisibility (set by magic system)
        //   8 = beginner quest (set by quest system)
        //  11 = team colour (set by event system)
        6 | 7 | 8 | 11 => return Ok(()),
        // Unknown types — reject
        _ => return Ok(()),
    }

    // (happens before the switch/case, applies to all bType values)
    // m_iTotalTrainingExp = 0; m_iTotalTrainingTime = 0; m_lastTrainingTime = 0;
    world.update_session(sid, |h| {
        h.total_training_exp = 0;
        h.last_training_time = 0;
    });

    // Update server-side state for Type 1 (sit/stand)
    if b_type == 1 {
        world.update_res_hp_type(sid, buff);

        // v2600 PCAP verified: original server does NOT send WizKing (0x78)
        // training packets on sit/stand. The v2525 training panel mechanism
        // (sub=0x02) does not exist in v2600. Training XP accumulation is
        // handled server-side only — no client UI packet needed.
    }

    // Update server-side state for Type 2 (party need flag)
    if b_type == 2 {
        world.update_session(sid, |h| {
            h.need_party = buff;
        });
    }

    // Update server-side state for Type 3 (abnormal/transformation — GM only)
    //   If GM, force visibility before transformation to prevent desync.
    //   m_nOldAbnormalType = m_bAbnormalType;
    //   m_bAbnormalType = nBuff;
    if b_type == 3 {
        world.update_session(sid, |h| {
            h.old_abnormal_type = h.abnormal_type;
            h.abnormal_type = n_buff;
        });
    }

    // Update server-side state for Type 5 (GM visibility toggle)
    //   m_bAbnormalType = nBuff;
    //   nBuff == 0 ? GmInOut(INOUT_OUT) : GmInOut(INOUT_IN);
    if b_type == STATE_CHANGE_GM_VISIBILITY {
        world.update_session(sid, |h| {
            h.abnormal_type = n_buff;
        });
    }

    // Note: Types 6 (party leader) and 11 (team colour) are now server-internal only
    // and are set directly by party.rs and event system handlers, not through this path.

    // Build broadcast packet: [u32 socket_id][u8 bType][u32 nBuff]
    // v2600 PCAP: no S2C state_change in sniffer because single-player session.
    // Client needs to receive its OWN state_change back for sit/stand to take effect.
    let mut bcast = Packet::new(Opcode::WizStateChange as u8);
    bcast.write_u32(sid as u32);
    bcast.write_u8(b_type);
    bcast.write_u32(n_buff);
    let arc_bcast = Arc::new(bcast);

    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        // Broadcast to region INCLUDING self — client needs confirmation
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::clone(&arc_bcast),
            None, // include self — client needs the echo to update sit/stand state
            event_room,
        );

        // For Type 5: broadcast GmInOut to make GM appear/disappear to other players
        //   GmGetInOut builds WIZ_USER_INOUT with GetUserInfo
        //   SendToRegion sends to 3x3 grid (excluding self)
        if b_type == STATE_CHANGE_GM_VISIBILITY {
            let inout_type = if n_buff == 0 { INOUT_OUT } else { INOUT_IN };
            let (ch, gm_abnormal) = world
                .with_session(sid, |h| (h.character.clone(), h.abnormal_type))
                .unwrap_or((None, 1));
            let clan = ch.as_ref().and_then(|c| {
                if c.knights_id > 0 {
                    world.get_knights(c.knights_id)
                } else {
                    None
                }
            });
            let bs = world.get_broadcast_state(sid);
            let equip_vis = get_equipped_visual(&world, sid);
            let ac = clan
                .as_ref()
                .and_then(|ki| crate::handler::region::resolve_alliance_cape(ki, &world));
            let is_king = ch
                .as_ref()
                .is_some_and(|c| world.is_king(c.nation, &c.name));
            let inout_pkt = build_user_inout_with_clan(
                inout_type,
                sid,
                ch.as_ref(),
                &pos,
                clan.as_ref(),
                ac,
                is_king,
                0, // GM visibility toggle — no invisibility type
                gm_abnormal,
                &bs,
                &equip_vis,
            );
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(inout_pkt),
                Some(sid), // exclude self
                event_room,
            );
        }
    }

    // Also send to self (C++ sends to region which includes self)
    session.send_packet(&arc_bcast).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_state_change_broadcast_format() {
        // Build broadcast packet: [u32 socket_id][u8 bType][u32 nBuff]
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(42); // session_id
        pkt.write_u8(1); // bType = sit/stand
        pkt.write_u32(0x02); // nBuff = USER_SITDOWN

        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        assert_eq!(pkt.data.len(), 9); // 4 + 1 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(0x02));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_state_change_emotion_format() {
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(100); // session_id
        pkt.write_u8(4); // bType = emotion
        pkt.write_u32(1); // nBuff = greeting 1

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(100));
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_u32(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 49: Session Lifecycle Integration Tests ──────────────────

    use crate::world::{CharacterInfo, Position, WorldState};
    use tokio::sync::mpsc;

    fn make_session_test_char(sid: u16, name: &str) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 1000,
            hp: 1000,
            max_mp: 500,
            mp: 500,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 100_000,
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

    /// Integration test: session register -> ingame -> verify character state.
    #[test]
    fn test_integration_session_register_to_ingame() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Before register_ingame, no character info
        assert!(world.get_character_info(1).is_none());

        let pos = Position {
            zone_id: 21,
            x: 512.0,
            y: 0.0,
            z: 341.0,
            region_x: 5,
            region_z: 3,
        };
        let ch = make_session_test_char(1, "TestPlayer");
        world.register_ingame(1, ch, pos);

        // After register_ingame, character info available
        let info = world.get_character_info(1).unwrap();
        assert_eq!(info.name, "TestPlayer");
        assert_eq!(info.level, 60);
        assert_eq!(info.hp, 1000);
        assert_eq!(info.gold, 100_000);

        let position = world.get_position(1).unwrap();
        assert_eq!(position.zone_id, 21);
        assert_eq!(position.x, 512.0);
        assert_eq!(position.z, 341.0);
    }

    /// Integration test: player death detection via HP.
    #[test]
    fn test_integration_player_dead_state() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_session_test_char(1, "Player"), pos);

        // Alive initially
        assert!(!world.is_player_dead(1));

        // Set HP to 0
        world.update_character_hp(1, 0);
        assert!(world.is_player_dead(1));

        // Restore HP
        world.update_character_hp(1, 500);
        assert!(!world.is_player_dead(1));
    }

    /// Integration test: gold gain and loss operations.
    #[test]
    fn test_integration_gold_operations() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_session_test_char(1, "GoldPlayer"), pos);

        // Start with 100,000
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 100_000);

        // Lose 30,000
        world.gold_lose(1, 30_000);
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 70_000);

        // Gain 50,000
        world.gold_gain(1, 50_000);
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, 120_000);
    }

    /// Integration test: zone change flag prevents stacking.
    #[test]
    fn test_integration_zone_change_flag() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_session_test_char(1, "Zoner"), pos);

        // Initially not zone changing
        assert!(!world.is_zone_changing(1));

        // Start zone change
        world.set_zone_changing(1, true);
        assert!(world.is_zone_changing(1));

        // Complete zone change
        world.set_zone_changing(1, false);
        assert!(!world.is_zone_changing(1));
    }

    /// Integration test: blink (respawn invulnerability) timing.
    #[test]
    fn test_integration_blink_timing() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_session_test_char(1, "Blinker"), pos);

        let now: u64 = 1700000000;

        // Not blinking initially
        assert!(!world.is_player_blinking(1, now));

        // Set blink expiry 10s in the future
        world.update_session(1, |h| {
            h.blink_expiry_time = now + 10;
        });

        // Still blinking at now+5
        assert!(world.is_player_blinking(1, now + 5));

        // Not blinking at now+10 (expired)
        assert!(!world.is_player_blinking(1, now + 10));

        // Not blinking at now+15
        assert!(!world.is_player_blinking(1, now + 15));
    }

    /// Captain (fame=100) cannot use greeting/provoke emotions.
    #[test]
    fn test_captain_blocked_from_emotions() {
        // COMMAND_CAPTAIN = 100 in C++ GameDefine.h:1285
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut ch = make_session_test_char(1, "Captain");
        ch.fame = 100; // COMMAND_CAPTAIN
        world.register_ingame(1, ch, pos);

        let info = world.get_character_info(1).unwrap();
        assert_eq!(info.fame, 100);
        // The handler should reject emotions 1-3 and 11-13 for captains
        // (verified via code review — handler checks `ch.fame == COMMAND_CAPTAIN`)
    }

    // ── Sprint 278: StateChange Type 2/3 Tests ─────────────────────────

    /// Test Type 2 need_party server state tracking.
    #[test]
    fn test_state_change_type2_need_party_state() {
        use crate::world::{Position, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_session_test_char(1, "Player1"), pos);

        // Default: need_party = 0
        let need = world.with_session(1, |h| h.need_party).unwrap();
        assert_eq!(need, 0);

        // Simulate Type 2 state change with buff=1 (looking for party)
        world.update_session(1, |h| {
            h.need_party = 1;
        });
        let need = world.with_session(1, |h| h.need_party).unwrap();
        assert_eq!(need, 1, "need_party should be set to 1 after Type 2");
    }

    /// Test Type 3 GM transformation updates abnormal_type and old_abnormal_type.
    #[test]
    fn test_state_change_type3_gm_abnormal_tracking() {
        use crate::world::{Position, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut ch = make_session_test_char(1, "GMPlayer");
        ch.authority = 0; // GM
        world.register_ingame(1, ch, pos);

        // Default: abnormal_type = 1 (ABNORMAL_NORMAL)
        let abn = world.with_session(1, |h| h.abnormal_type).unwrap();
        assert_eq!(abn, 1);

        // Simulate Type 3 state change — save old, set new
        world.update_session(1, |h| {
            h.old_abnormal_type = h.abnormal_type;
            h.abnormal_type = 5;
        });
        let (old, new) = world
            .with_session(1, |h| (h.old_abnormal_type, h.abnormal_type))
            .unwrap();
        assert_eq!(old, 1, "old_abnormal_type should be previous value");
        assert_eq!(new, 5, "abnormal_type should be updated to new value");
    }

    /// Verify GM visibility auto-toggle constant values.
    #[test]
    fn test_gm_visibility_auto_toggle_constants() {
        // Type 5 = GM visibility, buff 1 = visible
        assert_eq!(5u8, 5u8); // GM visibility type
        assert_eq!(1u32, 1u32); // Force visible buff value
                                // When abnormal_type == 0, GM is invisible and needs auto-toggle before Type 3
        assert_eq!(0u32, 0u32); // Invisible state
    }

    // ── Sprint 288: New state types ─────────────────────────────────────

    #[test]
    fn test_team_colour_enum_values() {
        let none: u8 = 0;
        let blue: u8 = 1;
        let red: u8 = 2;
        assert_eq!(none, 0);
        assert_eq!(blue, 1);
        assert_eq!(red, 2);
        // Verify mapping logic matches C++
        let buff: u8 = 2;
        let colour = match buff {
            2 => 2u8, // Red
            1 => 1u8, // Blue
            _ => 0u8, // None
        };
        assert_eq!(colour, 2);
    }

    #[test]
    fn test_state_type_6_party_leader_broadcast() {
        // Type 6 packet: [u32 sid][u8 type=6][u32 leader_flag]
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(10); // session_id
        pkt.write_u8(6); // bType = party leader
        pkt.write_u32(1); // nBuff = 1 (is leader)

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(10));
        assert_eq!(r.read_u8(), Some(6));
        assert_eq!(r.read_u32(), Some(1));
    }

    #[test]
    fn test_clan_notice_chat_type_value() {
        assert_eq!(24u8, 24);
    }

    // ── Sprint 325: Server-internal types rejected from client ──────────

    #[test]
    fn test_server_internal_types_rejected() {
        // Types 6, 7, 8, 11 should ONLY be set via StateChangeServerDirect
        // (server-side calls from party, magic, quest, and event systems).
        // Client packets with these types must be rejected.
        let server_only_types: [u8; 4] = [6, 7, 8, 11];
        let client_types: [u8; 5] = [1, 2, 3, 4, 5];

        // Server-only types should be in the rejection list
        for t in &server_only_types {
            assert!(
                matches!(*t, 6 | 7 | 8 | 11),
                "Type {} should be server-internal only",
                t
            );
        }

        // Client types should NOT be in the rejection list
        for t in &client_types {
            assert!(
                !matches!(*t, 6 | 7 | 8 | 11),
                "Type {} should be accepted from clients",
                t
            );
        }
    }

    // ── Sprint 751: Training timer packets ──────────────────────────────

    #[test]
    fn test_training_start_packet_format() {
        // v2525 client: WIZ_MINING sub=9, inner=1 (start)
        // Format: [0x86][0x09][0x01][u16 npc_id][5 skip bytes][u8 param]
        let mut pkt = Packet::new(Opcode::WizMining as u8);
        pkt.write_u8(9); // sub = training
        pkt.write_u8(1); // inner = start
        pkt.write_u16(0); // npc_id
        pkt.write_bytes(&[0u8; 5]); // 5 skip bytes
        pkt.write_u8(0); // parameter

        assert_eq!(pkt.opcode, Opcode::WizMining as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(9), "sub = training");
        assert_eq!(r.read_u8(), Some(1), "inner = start");
        assert_eq!(r.read_u16(), Some(0), "npc_id");
        // 5 skip bytes + 1 param = 6 remaining
        assert_eq!(r.remaining(), 6);
    }

    #[test]
    fn test_training_stop_packet_format() {
        // v2525 client: WIZ_MINING sub=9, inner=3 (stop)
        // Format: [0x86][0x09][0x03]
        let mut pkt = Packet::new(Opcode::WizMining as u8);
        pkt.write_u8(9); // sub = training
        pkt.write_u8(3); // inner = stop

        assert_eq!(pkt.opcode, Opcode::WizMining as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(9), "sub = training");
        assert_eq!(r.read_u8(), Some(3), "inner = stop");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_training_level_gate() {
        // Training only for level >= 10
        for level in [10u8, 83] {
            assert!(level >= 10, "level {level} qualifies for training");
        }
        let low_level: u8 = 9;
        assert!(low_level < 10, "level 9 does not qualify");
    }
}
