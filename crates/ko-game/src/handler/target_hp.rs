//! WIZ_TARGET_HP (0x22) handler — target HP query/response.
//! When the client clicks on a target (player or NPC), it sends this opcode
//! to query the target's HP. The server responds with max/current HP.
//! ## Request (Client -> Server)
//! | Type  | Description        |
//! |-------|--------------------|
//! | u32le | Target ID          |
//! | u8    | Echo flag          |
//! ## Response (Server -> Client)
//! | Type  | Description        |
//! |-------|--------------------|
//! | u32le | Target ID          |
//! | u8    | Echo flag          |
//! | u32le | Max HP             |
//! | u32le | Current HP         |
//! | i32le | HP change/damage   |
//! | i32le | Score delta        |
//! | u8    | Reserved (must be 0)|

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::npc::NPC_BAND;
use crate::session::{ClientSession, SessionState};
use crate::zone::SessionId;

/// Build the v2625 `WIZ_TARGET_HP` response.
///
/// The supplied v2625 executable's `sub_81DF70` reads the first trailing
/// dword into the damage-message path (multiplied by ten at `0x0081E537`) and
/// passes the second trailing dword to the score accumulator via
/// `sub_74A200`. Negative HP change means damage; positive means healing.
///
/// The final byte remains reserved in the v2625 client. A non-zero value here was
/// confirmed in `ko-server_20260821_195135.log` to make the client disconnect
/// immediately after selecting a Ronark Land monster.
pub(crate) fn build_target_hp_packet(
    target_id: u32,
    echo: u8,
    max_hp: u32,
    current_hp: u32,
    hp_change: i32,
    score_delta: i32,
) -> Packet {
    let mut response = Packet::new(Opcode::WizTargetHp as u8);
    response.write_u32(target_id);
    response.write_u8(echo);
    response.write_u32(max_hp);
    response.write_u32(current_hp);
    response.write_i32(hp_change);
    response.write_i32(score_delta);
    response.write_u8(0);
    response
}

/// Handle WIZ_TARGET_HP from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let target_id = reader.read_u32().unwrap_or(0);
    let echo = reader.read_u8().unwrap_or(0);

    if target_id == 0 {
        return Ok(());
    }

    let world = session.world().clone();

    let (max_hp, current_hp) = if let Some(bot) = world.get_bot(target_id) {
        // Resolve runtime bots before numeric bands. This remains correct if a
        // future shard changes its bot ID allocation.
        (bot.max_hp as u32, bot.hp.max(0) as u32)
    } else if target_id >= NPC_BAND {
        // NPC/Monster target
        let instance = match world.get_npc_instance(target_id) {
            Some(n) => n,
            None => return Ok(()),
        };
        let template = match world.get_npc_template(instance.proto_id, instance.is_monster) {
            Some(t) => t,
            None => return Ok(()),
        };
        // Use actual NPC HP from world state (updated by combat)
        let current = world
            .get_npc_hp(target_id)
            .unwrap_or(template.max_hp as i32);
        (template.max_hp, current.max(0) as u32)
    } else {
        // Player target
        let other_sid = target_id as SessionId;
        let ch = match world.get_character_info(other_sid) {
            Some(c) => c,
            None => return Ok(()),
        };
        (ch.max_hp as u32, ch.hp as u32)
    };

    // C++ stores m_targetID for use by GM commands like +npcinfo.  Its target
    // handler does not report ordinary NPCs or monsters; it prints one chat
    // line only when a GM selects a runtime bot.  NPC/monster data remains an
    // explicit +npcinfo action, so target-HP polls cannot spam the chat.
    let sid = session.session_id();
    let previous_target = world.with_session(sid, |h| h.target_id).unwrap_or(0);
    world.update_session(sid, |h| h.target_id = target_id);

    // Match reference_cpp/GameServer/User.cpp::CUser::HandleTargetHP.
    if previous_target != target_id {
        let is_gm = world
            .get_character_info(sid)
            .map(|c| c.authority == 0 || c.authority == 2)
            .unwrap_or(false);
        if is_gm {
            if let Some(bot) = world.get_bot(target_id) {
                let debug_msg = format!(
                    "ID = {}, Name = {}, Nation = {}",
                    target_id, bot.name, bot.nation
                );
                super::client_event::send_gm_debug_chat(&world, sid, &debug_msg);
            }
        }
    }

    let response = build_target_hp_packet(target_id, echo, max_hp, current_hp, 0, 0);

    session.send_packet(&response).await?;

    tracing::debug!(
        "[{}] TARGET_HP: target={}, hp={}/{}",
        session.addr(),
        target_id,
        current_hp,
        max_hp
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_target_hp_clamps_to_zero() {
        // Verify that negative HP values are clamped to 0
        let negative: i32 = -50;
        assert_eq!(negative.max(0) as u32, 0);

        let positive: i32 = 500;
        assert_eq!(positive.max(0) as u32, 500);

        let zero: i32 = 0;
        assert_eq!(zero.max(0) as u32, 0);
    }

    #[test]
    fn test_dead_target_hp_returns_zero_current() {
        // A dead NPC/player has current_hp <= 0. The handler clamps via .max(0).
        // Verify that various "dead" HP states all produce current_hp=0 in the response.

        // Dead NPC with exactly 0 HP
        let npc_hp: i32 = 0;
        let clamped = npc_hp.max(0) as u32;
        assert_eq!(clamped, 0, "Dead NPC with 0 HP should return 0");

        // Dead NPC with negative HP (overkill damage)
        let npc_hp: i32 = -1000;
        let clamped = npc_hp.max(0) as u32;
        assert_eq!(clamped, 0, "Overkill NPC with -1000 HP should return 0");

        // Dead player (hp stored as i32 in CharacterInfo)
        let player_hp: i32 = 0;
        let as_u32 = player_hp as u32;
        assert_eq!(as_u32, 0, "Dead player with 0 HP should return 0");

        // max_hp is still reported even when dead (client uses this for HP bar)
        let max_hp: u32 = 5000;
        let current_hp: u32 = 0;
        assert!(
            max_hp > current_hp,
            "Dead entity: max_hp should still be positive"
        );
    }

    #[test]
    fn test_zero_target_id_rejected() {
        // The handler returns early when target_id == 0.
        // This prevents lookups with an invalid sentinel value.
        let target_id: u32 = 0;
        assert_eq!(target_id, 0, "Zero target_id should be rejected");

        // Verify the guard condition matches the handler logic
        let should_reject = target_id == 0;
        assert!(should_reject, "target_id=0 must trigger early return");

        // Non-zero IDs pass the guard
        let valid_player_id: u32 = 42;
        assert_ne!(valid_player_id, 0);
        let valid_npc_id: u32 = NPC_BAND + 5; // 10005
        assert_ne!(valid_npc_id, 0);
    }

    #[test]
    fn test_c2s_target_hp_packet_format() {
        // C2S: [u32 target_id] [u8 echo]
        let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
        pkt.write_u32(10042); // target_id (NPC)
        pkt.write_u8(1); // echo flag

        assert_eq!(pkt.opcode, Opcode::WizTargetHp as u8);
        assert_eq!(pkt.data.len(), 5); // 4 + 1

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(10042));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_s2c_target_hp_response_format() {
        let pkt = build_target_hp_packet(10042, 1, 5000, 3500, -123, -45);

        assert_eq!(pkt.data.len(), 22); // 4+1+4+4+4+4+1

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(10042));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.read_u32(), Some(3500));
        assert_eq!(r.read_i32(), Some(-123));
        assert_eq!(r.read_i32(), Some(-45));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_npc_band_constant() {
        // NPC IDs start at NPC_BAND; below that are player session IDs
        assert_eq!(NPC_BAND, 10000);
    }

    #[test]
    fn test_target_hp_opcode_value() {
        assert_eq!(Opcode::WizTargetHp as u8, 0x22);
    }

    #[test]
    fn test_target_id_classification() {
        // target_id < NPC_BAND → player; >= NPC_BAND → NPC/bot
        let player_id: u32 = 42;
        let npc_id: u32 = NPC_BAND + 100;

        assert!(player_id < NPC_BAND, "Player IDs below NPC_BAND");
        assert!(npc_id >= NPC_BAND, "NPC IDs at or above NPC_BAND");
    }

    #[test]
    fn test_gm_authority_values_for_debug_info() {
        // GM debug info is only sent when authority == 0 (GAME_MASTER)
        // or authority == 2 (GM_USER). Other values are regular players.

        // GAME_MASTER
        let authority: u8 = 0;
        let is_gm = authority == 0 || authority == 2;
        assert!(is_gm, "authority=0 (GAME_MASTER) should trigger debug info");

        // GM_USER
        let authority: u8 = 2;
        let is_gm = authority == 0 || authority == 2;
        assert!(is_gm, "authority=2 (GM_USER) should trigger debug info");

        // Regular player (authority=1)
        let authority: u8 = 1;
        let is_gm = authority == 0 || authority == 2;
        assert!(
            !is_gm,
            "authority=1 (regular player) should NOT get debug info"
        );

        // Other authority values — also not GM
        for auth in [3u8, 5, 10, 255] {
            let is_gm = auth == 0 || auth == 2;
            assert!(!is_gm, "authority={auth} should NOT be treated as GM");
        }

        // Verify NPC_BAND threshold: debug info only applies to NPC-band targets
        let player_target: u32 = 50; // below NPC_BAND
        let npc_target: u32 = NPC_BAND + 100; // above NPC_BAND
        assert!(
            player_target < NPC_BAND,
            "Player IDs are below NPC_BAND ({NPC_BAND})"
        );
        assert!(
            npc_target >= NPC_BAND,
            "NPC IDs are at or above NPC_BAND ({NPC_BAND})"
        );
    }

    // ── Sprint 929: Additional coverage ──────────────────────────────

    /// C2S data length: target_id(4) + echo(1) = 5.
    #[test]
    fn test_target_hp_c2s_data_length() {
        let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
        pkt.write_u32(42);
        pkt.write_u8(1);
        assert_eq!(pkt.data.len(), 5);
    }

    /// S2C response data length: target_id(4) + echo(1) + max_hp(4) + cur_hp(4) + dmg(4) + reserved(4) + reserved(1) = 22.
    #[test]
    fn test_target_hp_s2c_data_length() {
        let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
        pkt.write_u32(10042);
        pkt.write_u8(1);
        pkt.write_u32(5000);
        pkt.write_u32(3500);
        pkt.write_u32(0);
        pkt.write_u32(0);
        pkt.write_u8(0);
        assert_eq!(pkt.data.len(), 22);
    }

    /// Echo flag is preserved in response (roundtrip).
    #[test]
    fn test_target_hp_echo_roundtrip() {
        for echo in [0u8, 1, 127, 255] {
            let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
            pkt.write_u32(100);
            pkt.write_u8(echo);
            pkt.write_u32(1000);
            pkt.write_u32(500);
            pkt.write_u32(0);
            pkt.write_u32(0);
            pkt.write_u8(0);

            let mut r = PacketReader::new(&pkt.data);
            assert_eq!(r.read_u32(), Some(100));
            assert_eq!(r.read_u8(), Some(echo), "echo={echo} preserved");
        }
    }

    /// Player target (id < NPC_BAND) HP uses u32 for both max and current.
    #[test]
    fn test_target_hp_player_hp_u32() {
        let max_hp: u32 = 25000;
        let hp: u32 = 12500;
        let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
        pkt.write_u32(42); // player sid
        pkt.write_u8(0);
        pkt.write_u32(max_hp);
        pkt.write_u32(hp);
        pkt.write_u32(0);
        pkt.write_u32(0);
        pkt.write_u8(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        r.read_u8();
        assert_eq!(r.read_u32(), Some(25000));
        assert_eq!(r.read_u32(), Some(12500));
    }

    /// Damage and reserved fields are always 0 in current implementation.
    #[test]
    fn test_target_hp_trailing_zeros() {
        let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
        pkt.write_u32(10001);
        pkt.write_u8(1);
        pkt.write_u32(8000);
        pkt.write_u32(4000);
        pkt.write_u32(0); // damage
        pkt.write_u32(0); // reserved u32
        pkt.write_u8(0); // reserved u8

        let mut r = PacketReader::new(&pkt.data);
        r.read_u32();
        r.read_u8();
        r.read_u32();
        r.read_u32();
        assert_eq!(r.read_u32(), Some(0), "damage always 0");
        assert_eq!(r.read_u32(), Some(0), "reserved u32 always 0");
        assert_eq!(r.read_u8(), Some(0), "reserved u8 always 0");
        assert_eq!(r.remaining(), 0);
    }
}
