//! Equipment View — WIZ_USER_INFORMATIN (0x98) sub-opcode 5.
//!
//! The right-click context menu on another player's (or a GM-spawned bot's —
//! see `systems/bot_ai.rs::write_bot_user_info`, which shares the player
//! INOUT wire format) model offers "Equipment View" alongside "Request a
//! party", "Report", "User trade", "Whisper", "Add friend", "User
//! Information" and "Duel". This answers that request with the target's 17
//! visual equipment slots (8 equipped + 9 cosplay — see
//! `handler/region.rs::VISUAL_SLOT_ORDER`/`get_equipped_visual`).
//!
//! ## Reverse-engineering confidence
//! See `docs/reverse-engineering/2625/KNIGHTONLINE_2625_PROTOCOL_AUDIT.md`
//! for the full writeup. Short version: the C2S trigger (sub-opcode 5, `u16`
//! target id) is an assumption inherited from code that already existed in
//! `user_info.rs` before this pass (originally named
//! `handle_get_user_info_view`) — this pass could not independently
//! reconfirm it against the client binary. The S2C reply shape below
//! (symmetric with the sub 1/3 list request/response pair elsewhere in
//! `user_info.rs`) is this pass's best-effort placeholder, **not** a
//! confirmed wire format: direct inspection of the client's packet-receive
//! dispatcher for opcode 0x98 (decompiled function at file offset
//! `0x7DA4F0`) shows real cases only for sub-opcodes `{1, 2, 3, 4, 6}` — sub
//! 5 falls through to the default/no-op branch, so a real v2625 client is
//! likely to silently drop this exact reply. See the audit doc's "Open
//! items" for the `sub=6`/`UIOtherUserState.cpp` lead that should be tried
//! first in any follow-up pass.

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::npc::NPC_BAND;
use crate::session::ClientSession;

use super::region::get_equipped_visual;

/// Sub-opcode shared by the client's request and this handler's reply
/// (mirrors the symmetric sub 1/3 list request/response pair in
/// `user_info.rs`).
pub const SUB_EQUIPMENT_VIEW: u8 = 5;

/// Handle a WIZ_USER_INFORMATIN sub=5 (Equipment View) request.
///
/// Wire (Client -> Server): `[u8 sub=5] [u16 target_id]`
/// `target_id` is a session id for a player target, or a `BotId` for a
/// GM-spawned bot — the same id space `attack.rs` already distinguishes via
/// `tid < NPC_BAND && world.get_bot(tid).is_none()`.
pub async fn handle(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let Some(target_id) = reader.read_u16() else {
        return Ok(());
    };
    let world = session.world().clone();
    let tid = target_id as u32;

    let response = if tid < NPC_BAND && world.get_bot(tid).is_none() {
        let target_sid = target_id;
        let Some(target) = world.get_character_info(target_sid) else {
            return Ok(());
        };
        // GMs are invisible to non-GMs — same rule UserInfoDetail (sub 2)
        // already applies to the bottom-left-panel inspect path.
        if let Some(my_ch) = world.get_character_info(session.session_id()) {
            if my_ch.authority != 0 && target.authority == 0 {
                return Ok(());
            }
        }
        let equip = get_equipped_visual(&world, target_sid);
        build_response(
            target_id,
            &target.name,
            target.nation,
            target.race,
            target.class,
            target.level,
            &equip,
        )
    } else {
        let Some(bot) = world.get_bot(tid) else {
            return Ok(());
        };
        build_response(
            target_id,
            &bot.name,
            bot.nation,
            bot.race,
            bot.class,
            bot.level,
            &bot.equip_visual,
        )
    };

    session.send_packet(&response).await?;
    Ok(())
}

/// Build the sub=5 reply: target id + name/nation/race/class/level header,
/// followed by the 17 visual equipment slots in `VISUAL_SLOT_ORDER` (8
/// equipped + 9 cosplay), each `[u32 item_id][u16 duration][u8 flag]` — the
/// same per-slot encoding `region.rs::build_user_inout_with_clan` already
/// uses for the broadcast INOUT packet.
fn build_response(
    target_id: u16,
    name: &str,
    nation: u8,
    race: u8,
    class: u16,
    level: u8,
    equip_visual: &[(u32, i16, u8); 17],
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
    pkt.write_u8(SUB_EQUIPMENT_VIEW);
    pkt.write_u16(target_id);
    pkt.write_sbyte_string(name);
    pkt.write_u8(nation);
    pkt.write_u8(race);
    pkt.write_u16(class);
    pkt.write_u8(level);
    for &(item_id, dur, flag) in equip_visual {
        pkt.write_u32(item_id);
        pkt.write_u16(dur as u16);
        pkt.write_u8(flag);
    }
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_opcode_constant() {
        assert_eq!(SUB_EQUIPMENT_VIEW, 5);
    }

    #[test]
    fn test_build_response_header_and_opcode() {
        let equip = [(0u32, 0i16, 0u8); 17];
        let pkt = build_response(42, "TestKnight", 1, 2, 107, 60, &equip);
        assert_eq!(pkt.opcode, Opcode::WizUserInfo as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_EQUIPMENT_VIEW));
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.read_sbyte_string(), Some("TestKnight".to_string()));
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u8(), Some(2)); // race
        assert_eq!(r.read_u16(), Some(107)); // class
        assert_eq!(r.read_u8(), Some(60)); // level
    }

    #[test]
    fn test_build_response_equip_slots_round_trip() {
        let mut equip = [(0u32, 0i16, 0u8); 17];
        // Slot 2 is HEAD in VISUAL_SLOT_ORDER; slot 13 is CEMBLEM.
        equip[2] = (123456, 1000, 0);
        equip[13] = (931677910, -1, 1);

        let pkt = build_response(7, "X", 1, 1, 0, 1, &equip);
        let mut r = PacketReader::new(&pkt.data);
        // Skip header: sub(1) + target_id(2) + sbyte "X"(2) + nation/race(2) + class(2) + level(1)
        r.read_u8();
        r.read_u16();
        r.read_sbyte_string();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        r.read_u8();

        for i in 0..17 {
            let item_id = r.read_u32().unwrap();
            let dur = r.read_u16().unwrap();
            let flag = r.read_u8().unwrap();
            let (exp_id, exp_dur, exp_flag) = equip[i];
            assert_eq!(item_id, exp_id, "slot {i} item id mismatch");
            assert_eq!(dur, exp_dur as u16, "slot {i} duration mismatch");
            assert_eq!(flag, exp_flag, "slot {i} flag mismatch");
        }
    }

    #[test]
    fn test_empty_request_does_not_panic() {
        // Zero-length payload (missing target id) must not panic the reader.
        let empty: [u8; 0] = [];
        let mut r = PacketReader::new(&empty);
        assert_eq!(r.read_u16(), None);
    }
}
