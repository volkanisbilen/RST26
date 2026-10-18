//! WIZ_GENDER_CHANGE (0x8D) handler — race/gender/appearance change.
//! ## Flow
//! 1. Client sends `[u8 sub] [u8 race] [u8 face] [u32 hair]`
//! 2. Server validates: item exists, race matches nation/class, face/hair > 0
//! 3. On success: updates appearance, consumes item, region re-broadcast
//! 4. Response: `[u8 result] [u8 race] [u8 face] [u32 hair] [u8 class]` or `[u8 0]` on fail
//! ## Item Required
//! - `ITEM_GENDER_CHANGE (810594000)`
//! ## Disabled during Cinderella event (checked in dispatch)

#[cfg(test)]
use ko_protocol::PacketReader;
use ko_protocol::{Opcode, Packet};
#[cfg(test)]
use std::sync::Arc;
use tracing::debug;

use crate::session::{ClientSession, SessionState};

// ── Race constants ──────────────────────────────────────────────────────────

#[cfg(test)]
use crate::race_constants::{
    BABARIAN, ELMORAD_MAN, ELMORAD_WOMAN, KARUS_BIG, KARUS_MIDDLE, KARUS_SMALL, KARUS_WOMAN,
    KURIAN, PORUTU,
};

/// Gender Change scroll item ID.
#[cfg(test)]
const ITEM_GENDER_CHANGE: u32 = 810594000;

/// Handle WIZ_GENDER_CHANGE from the client.
/// **v2525 CONFLICT**: Client opcode 0x8D = WizTitle_2 (title sub-system),
/// NOT GenderChange. The v2525 client's handler at `0x99F720` dispatches
/// sub 0/1/2 as title operations. Sending GenderChange S2C packets on 0x8D
/// causes the client to misinterpret the data as title commands.
/// We accept C2S but respond with a WIZ_CHAT notice instead of 0x8D packets.
/// Packet format: `[u8 sub] [u8 new_race] [u8 new_face] [u32 new_hair]`
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // v2525: 0x8D = WizTitle_2 on client side. GenderChange UI is unavailable.
    // Send WIZ_CHAT fallback instead of broken 0x8D S2C packets.
    debug!(
        "[{}] WIZ_GENDER_CHANGE — blocked (v2525 0x8D=WizTitle_2 conflict)",
        session.addr(),
    );

    let mut chat = Packet::new(Opcode::WizChat as u8);
    chat.write_u8(7); // PUBLIC_CHAT
    chat.write_u8(20); // system_msg type
    chat.write_string("Gender Change is not available in this client version.");
    session.send_packet(&chat).await
}

/// Original implementation (v2525 conflict blocks this — preserved for future client versions).
#[cfg(test)]
#[allow(dead_code)]
async fn handle_impl(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // State validation
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_store_open(sid)
        || world.is_merchanting(sid)
        || world.is_selling_merchant_preparing(sid)
        || world.is_buying_merchant_preparing(sid)
    {
        return Ok(());
    }
    if world.is_mining(sid) || world.is_fishing(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let _sub = reader.read_u8(); // sub-opcode (dispatched externally)
    let new_race = match reader.read_u8() {
        Some(v) if v > 0 => v,
        _ => {
            send_fail(session).await?;
            return Ok(());
        }
    };
    let new_face = match reader.read_u8() {
        Some(v) if v > 0 => v,
        _ => {
            send_fail(session).await?;
            return Ok(());
        }
    };
    let new_hair = match reader.read_u32() {
        Some(v) if v > 0 => v,
        _ => {
            send_fail(session).await?;
            return Ok(());
        }
    };

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must have gender change scroll
    let has_scroll = world.update_inventory(sid, |inv| {
        for i in 14..42 {
            if let Some(slot) = inv.get(i) {
                if slot.item_id == ITEM_GENDER_CHANGE && slot.count > 0 {
                    return true;
                }
            }
        }
        false
    });
    if !has_scroll {
        send_fail(session).await?;
        return Ok(());
    }

    // Validate race/nation compatibility
    if !validate_race_nation(new_race, ch.nation) {
        send_fail(session).await?;
        return Ok(());
    }

    // Update character appearance
    world.update_character_stats(sid, |c| {
        c.race = new_race;
        c.face = new_face;
        c.hair_rgb = new_hair;
        // If HP < max_hp/2, restore to full
        if c.hp < c.max_hp / 2 {
            c.hp = c.max_hp;
        }
    });

    // Consume gender change scroll
    world.rob_item(sid, ITEM_GENDER_CHANGE, 1);

    // Send success response
    // C++ format: [u8 result=1] [u8 race] [u8 face] [u32 hair] [u8 class]
    let mut result = Packet::new(Opcode::WizGenderChange as u8);
    result.write_u8(1); // success
    result.write_u8(new_race);
    result.write_u8(new_face);
    result.write_u32(new_hair);
    result.write_u8((ch.class & 0xFF) as u8);
    session.send_packet(&result).await?;

    // Region re-broadcast: OUT then WARP so nearby players see updated appearance
    let pos = world.get_position(sid).unwrap_or_default();
    let out_pkt = crate::handler::region::build_user_inout(
        crate::handler::region::INOUT_OUT,
        sid,
        None,
        &pos,
    );
    world.broadcast_to_zone(pos.zone_id, Arc::new(out_pkt), Some(sid));

    let new_ch = world.get_character_info(sid);
    let invis = world.get_invisibility_type(sid);
    let abnormal = world.get_abnormal_type(sid);
    let equip_vis = crate::handler::region::get_equipped_visual(&world, sid);
    let in_pkt = crate::handler::region::build_user_inout_with_invis(
        crate::handler::region::INOUT_WARP,
        sid,
        new_ch.as_ref(),
        &pos,
        invis,
        abnormal,
        &equip_vis,
    );
    world.broadcast_to_zone(pos.zone_id, Arc::new(in_pkt), Some(sid));

    world.clear_all_buffs(sid, false);
    world.set_user_ability(sid);
    world.recast_saved_magic(sid);

    debug!(
        "[{}] WIZ_GENDER_CHANGE: race={} face={} hair={:#X}",
        session.addr(),
        new_race,
        new_face,
        new_hair,
    );

    Ok(())
}

/// Validate that the new race is compatible with the player's nation.
/// ```c++
/// if (gRace < 10 && GetNation() != 1 || (gRace > 10 && GetNation() != 2) || (gRace > 5 && GetNation() == 1))
///     goto fail_return;
/// ```
/// - Karus (nation=1): races 1-4 only (race > 5 rejected — Kurians cannot gender change)
/// - El Morad (nation=2): races 11-14
#[cfg(test)]
fn validate_race_nation(race: u8, nation: u8) -> bool {
    match nation {
        1 => {
            // Karus: valid races are 1-4 only
            // C++ rejects race > 5 for Karus — Kurians (race 6) cannot use gender change
            matches!(race, KARUS_BIG | KARUS_MIDDLE | KARUS_SMALL | KARUS_WOMAN)
        }
        2 => {
            // El Morad: valid races are 11-14
            matches!(race, BABARIAN | ELMORAD_MAN | ELMORAD_WOMAN | PORUTU)
        }
        _ => false,
    }
}

/// Send a gender change failure response.
/// C++ format: `[u8 result=0]`
#[cfg(test)]
#[allow(dead_code)]
async fn send_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
    pkt.write_u8(0); // failure
    session.send_packet(&pkt).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::Packet;

    #[test]
    fn test_race_constants() {
        assert_eq!(KARUS_BIG, 1);
        assert_eq!(KARUS_MIDDLE, 2);
        assert_eq!(KARUS_SMALL, 3);
        assert_eq!(KARUS_WOMAN, 4);
        assert_eq!(KURIAN, 6);
        assert_eq!(BABARIAN, 11);
        assert_eq!(ELMORAD_MAN, 12);
        assert_eq!(ELMORAD_WOMAN, 13);
        assert_eq!(PORUTU, 14);
    }

    #[test]
    fn test_validate_race_nation_karus() {
        // Valid Karus races (1-4 only)
        assert!(validate_race_nation(KARUS_BIG, 1));
        assert!(validate_race_nation(KARUS_MIDDLE, 1));
        assert!(validate_race_nation(KARUS_SMALL, 1));
        assert!(validate_race_nation(KARUS_WOMAN, 1));
        // Kurian (race 6) CANNOT gender change — C++ rejects race > 5 for Karus
        assert!(!validate_race_nation(KURIAN, 1));
        // Invalid: El Morad races in Karus nation
        assert!(!validate_race_nation(BABARIAN, 1));
        assert!(!validate_race_nation(ELMORAD_MAN, 1));
        assert!(!validate_race_nation(ELMORAD_WOMAN, 1));
        // Race 5 is invalid
        assert!(!validate_race_nation(5, 1));
    }

    #[test]
    fn test_validate_race_nation_elmorad() {
        // Valid El Morad races
        assert!(validate_race_nation(BABARIAN, 2));
        assert!(validate_race_nation(ELMORAD_MAN, 2));
        assert!(validate_race_nation(ELMORAD_WOMAN, 2));
        assert!(validate_race_nation(PORUTU, 2));
        // Invalid: Karus races in El Morad nation
        assert!(!validate_race_nation(KARUS_BIG, 2));
        assert!(!validate_race_nation(KARUS_MIDDLE, 2));
    }

    #[test]
    fn test_validate_race_nation_invalid_nation() {
        assert!(!validate_race_nation(KARUS_BIG, 0));
        assert!(!validate_race_nation(KARUS_BIG, 3));
    }

    #[test]
    fn test_gender_change_request_packet() {
        let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
        pkt.write_u8(0); // sub-opcode
        pkt.write_u8(ELMORAD_MAN); // new race
        pkt.write_u8(3); // new face
        pkt.write_u32(0xFF00AA); // new hair

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(ELMORAD_MAN));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(0xFF00AA));
    }

    #[test]
    fn test_gender_change_success_response() {
        let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
        pkt.write_u8(1); // success
        pkt.write_u8(ELMORAD_MAN);
        pkt.write_u8(3); // face
        pkt.write_u32(0xFF00AA); // hair
        pkt.write_u8(102); // class

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(ELMORAD_MAN));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(0xFF00AA));
        assert_eq!(r.read_u8(), Some(102));
    }

    #[test]
    fn test_gender_change_fail_response() {
        let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
        pkt.write_u8(0); // failure
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 0);
    }

    #[test]
    fn test_item_id() {
        assert_eq!(ITEM_GENDER_CHANGE, 810594000);
    }

    // ── Sprint 927: Additional coverage ──────────────────────────────

    /// C2S request: sub(1) + race(1) + face(1) + hair(4) = 7 bytes.
    #[test]
    fn test_gender_change_c2s_data_length() {
        let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
        pkt.write_u8(0);
        pkt.write_u8(ELMORAD_MAN);
        pkt.write_u8(3);
        pkt.write_u32(0xFF00AA);
        assert_eq!(pkt.data.len(), 7);
    }

    /// Success response: result(1) + race(1) + face(1) + hair(4) + class(1) = 8 bytes.
    #[test]
    fn test_gender_change_success_response_length() {
        let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
        pkt.write_u8(1); // success
        pkt.write_u8(ELMORAD_MAN);
        pkt.write_u8(3);
        pkt.write_u32(0xFF00AA);
        pkt.write_u8(102); // class
        assert_eq!(pkt.data.len(), 8);
    }

    /// Kurian (race 6) is blocked for both nations — cannot gender change.
    #[test]
    fn test_gender_change_kurian_blocked() {
        assert!(!validate_race_nation(KURIAN, 1), "Kurian blocked in Karus");
        assert!(
            !validate_race_nation(KURIAN, 2),
            "Kurian blocked in ElMorad"
        );
        assert!(
            !validate_race_nation(KURIAN, 0),
            "Kurian blocked for nation 0"
        );
    }

    /// Boundary race values: 0, 5, 10, 15 are all invalid.
    #[test]
    fn test_gender_change_race_boundary_values() {
        for race in [0u8, 5, 10, 15, 255] {
            assert!(
                !validate_race_nation(race, 1),
                "race {race} invalid for Karus"
            );
            assert!(
                !validate_race_nation(race, 2),
                "race {race} invalid for ElMorad"
            );
        }
    }

    /// Success response roundtrip with all fields.
    #[test]
    fn test_gender_change_success_roundtrip() {
        let mut pkt = Packet::new(Opcode::WizGenderChange as u8);
        pkt.write_u8(1);
        pkt.write_u8(KARUS_WOMAN);
        pkt.write_u8(5);
        pkt.write_u32(0xAABBCC);
        pkt.write_u8(201); // class

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(KARUS_WOMAN));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u32(), Some(0xAABBCC));
        assert_eq!(r.read_u8(), Some(201));
        assert_eq!(r.remaining(), 0);
    }
}
