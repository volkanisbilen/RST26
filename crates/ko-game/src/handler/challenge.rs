//! WIZ_CHALLENGE (0x75) handler — PVP duel and CVC (clan vs clan) arena.
//! Sub-opcodes:
//! - CHALLENGE_PVP_REQUEST (1): Challenger requests a 1v1 duel
//! - CHALLENGE_PVP_CANCEL (2): Challenger cancels the request
//! - CHALLENGE_PVP_ACCEPT (3): Challengee accepts the duel
//! - CHALLENGE_PVP_REJECT (4): Challengee rejects the duel
//! - CHALLENGE_PVP_REQ_SENT (5): Server -> challenger confirmation
//! - CHALLENGE_CVC_REQUEST (6): Clan leader requests CVC
//! - CHALLENGE_CVC_CANCEL (7): Clan leader cancels CVC request
//! - CHALLENGE_CVC_ACCEPT (8): Target clan leader accepts CVC
//! - CHALLENGE_CVC_REJECT (9): Target clan leader rejects CVC
//! - CHALLENGE_CVC_REQ_SENT (10): Server -> requester confirmation
//! - CHALLENGE_GENERIC_ERROR (11): Generic error response
//! - CHALLENGE_ZONE_ERROR (12): Zone restriction error
//! - CHALLENGE_CLAN_ERROR (13): Clan requirement error

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::ZONE_ARENA;
use crate::zone::SessionId;

/// Challenge sub-opcode constants.
const CHALLENGE_PVP_REQUEST: u8 = 1;
const CHALLENGE_PVP_CANCEL: u8 = 2;
const CHALLENGE_PVP_ACCEPT: u8 = 3;
const CHALLENGE_PVP_REJECT: u8 = 4;
const CHALLENGE_PVP_REQ_SENT: u8 = 5;
const CHALLENGE_CVC_REQUEST: u8 = 6;
const CHALLENGE_CVC_CANCEL: u8 = 7;
const CHALLENGE_CVC_ACCEPT: u8 = 8;
const CHALLENGE_CVC_REJECT: u8 = 9;
const CHALLENGE_CVC_REQ_SENT: u8 = 10;
const CHALLENGE_GENERIC_ERROR: u8 = 11;
const CHALLENGE_ZONE_ERROR: u8 = 12;
const CHALLENGE_CLAN_ERROR: u8 = 13;

/// Arena zone coordinates for PVP duel.
const PVP_ARENA_ACCEPTER_X: f32 = 135.0;
const PVP_ARENA_ACCEPTER_Z: f32 = 115.0;
const PVP_ARENA_REQUESTER_X: f32 = 120.0;
const PVP_ARENA_REQUESTER_Z: f32 = 115.0;

/// Arena zone coordinates for CVC (clan1 and clan2).
const CVC_ARENA_CLAN1_X: f32 = 128.0;
const CVC_ARENA_CLAN1_Z: f32 = 125.0;
const CVC_ARENA_CLAN2_X: f32 = 135.0;
const CVC_ARENA_CLAN2_Z: f32 = 120.0;

/// Handle WIZ_CHALLENGE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Must be alive
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        CHALLENGE_PVP_REQUEST => handle_pvp_request(session, &mut reader).await,
        CHALLENGE_PVP_ACCEPT => handle_pvp_accept(session).await,
        CHALLENGE_CVC_REQUEST => handle_cvc_request(session, &mut reader).await,
        CHALLENGE_CVC_ACCEPT => handle_cvc_accept(session).await,
        CHALLENGE_PVP_CANCEL | CHALLENGE_CVC_CANCEL => handle_cancelled(session, sub_opcode).await,
        CHALLENGE_PVP_REJECT | CHALLENGE_CVC_REJECT => handle_rejected(session, sub_opcode).await,
        _ => {
            debug!(
                "[{}] Challenge unhandled sub-opcode {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Build a WIZ_CHALLENGE error packet.
fn build_error(error_code: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizChallenge as u8);
    pkt.write_u8(error_code);
    pkt
}

/// Check if a zone is a nation PVP zone (can attack other nation but not same).
fn is_nation_pvp_zone(world: &crate::world::WorldState, zone_id: u16) -> bool {
    world
        .get_zone(zone_id)
        .is_some_and(|z| z.can_attack_other_nation() && !z.can_attack_same_nation())
}

/// Send a zone change teleport packet to a remote session.
/// Used when we cannot call trigger_zone_change directly (no &mut ClientSession).
fn send_zone_change(
    world: &crate::world::WorldState,
    target_sid: SessionId,
    zone_id: u16,
    x: f32,
    z: f32,
    nation: u8,
) {
    world.update_position(target_sid, zone_id, x, 0.0, z);

    let mut zpkt = Packet::new(Opcode::WizZoneChange as u8);
    zpkt.write_u8(3); // ZONE_CHANGE_TELEPORT
    zpkt.write_u16(zone_id);
    zpkt.write_u16(0);
    zpkt.write_u16((x * 10.0) as u16);
    zpkt.write_u16((z * 10.0) as u16);
    zpkt.write_u16(0);
    zpkt.write_u8(nation);
    zpkt.write_u16(0xFFFF);

    world.send_to_session_owned(target_sid, zpkt);
}

/// CHALLENGE_PVP_REQUEST (1): Request a 1v1 duel.
async fn handle_pvp_request(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Validate self: not already in challenge, not in arena, not in party/trading/merchanting
    let (self_requesting, self_requested, _, self_zone_id) = match world.with_session(sid, |h| {
        (
            h.requesting_challenge,
            h.challenge_requested,
            h.challenge_user,
            h.position.zone_id,
        )
    }) {
        Some(v) => v,
        None => return Ok(()),
    };

    if self_requesting != 0
        || self_requested != 0
        || self_zone_id == ZONE_ARENA
        || world.is_in_party(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
    {
        session
            .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
            .await?;
        return Ok(());
    }

    // Zone restriction: no challenges in nation PVP zones, Delos, or war zones
    if is_nation_pvp_zone(&world, self_zone_id)
        || self_zone_id == crate::world::ZONE_DELOS
        || world
            .get_zone(self_zone_id)
            .is_some_and(|z| z.is_war_zone())
    {
        session
            .send_packet(&build_error(CHALLENGE_ZONE_ERROR))
            .await?;
        return Ok(());
    }

    // Read target name (SByte string)
    let target_name = match reader.read_sbyte_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let target_sid = match world.find_session_by_name(&target_name) {
        Some(id) => id,
        None => {
            session
                .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
                .await?;
            return Ok(());
        }
    };

    // Validate target: in-game, alive, not busy, same zone
    let target_ok = world
        .with_session(target_sid, |h| {
            let ch = match &h.character {
                Some(c) => c,
                None => return false,
            };
            ch.res_hp_type != crate::world::USER_DEAD
                && h.requesting_challenge == 0
                && h.challenge_requested == 0
                && h.position.zone_id == self_zone_id
        })
        .unwrap_or(false);

    if !target_ok
        || world.is_in_party(target_sid)
        || world.is_trading(target_sid)
        || world.is_merchanting(target_sid)
    {
        session
            .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
            .await?;
        return Ok(());
    }

    // Get self name for response packets
    let self_name = match world.get_character_info(sid) {
        Some(ch) => ch.name,
        None => return Ok(()),
    };

    // Set challenge state on both players
    world.update_session(sid, |h| {
        h.requesting_challenge = CHALLENGE_PVP_CANCEL;
        h.challenge_user = target_sid as i16;
    });
    world.update_session(target_sid, |h| {
        h.challenge_requested = CHALLENGE_PVP_REJECT;
        h.challenge_user = sid as i16;
    });

    // Send request to target: [PVP_REQUEST] [challenger_name]
    let mut to_target = Packet::new(Opcode::WizChallenge as u8);
    to_target.write_u8(CHALLENGE_PVP_REQUEST);
    to_target.write_sbyte_string(&self_name);
    world.send_to_session_owned(target_sid, to_target);

    // Send confirmation to requester: [PVP_REQ_SENT] [target_name]
    let mut to_self = Packet::new(Opcode::WizChallenge as u8);
    to_self.write_u8(CHALLENGE_PVP_REQ_SENT);
    to_self.write_sbyte_string(&target_name);
    session.send_packet(&to_self).await?;

    debug!(
        "[{}] Challenge PVP request: {} -> {}",
        session.addr(),
        self_name,
        target_name,
    );

    Ok(())
}

/// CHALLENGE_PVP_ACCEPT (3): Accept a PVP duel request.
async fn handle_pvp_accept(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Must have a pending challenge request
    let (_, challenge_requested, challenge_user) = world.get_challenge_state(sid);

    if challenge_requested == 0 {
        return Ok(());
    }

    let requester_sid = challenge_user as SessionId;

    // Clear challenge state on self
    world.update_session(sid, |h| {
        h.challenge_user = -1;
        h.challenge_requested = 0;
    });

    // Get the requester's nation
    let requester_nation = match world.get_character_info(requester_sid) {
        Some(ch) => ch.nation,
        None => {
            session
                .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
                .await?;
            return Ok(());
        }
    };

    // Clear challenge state on requester
    world.update_session(requester_sid, |h| {
        h.challenge_user = -1;
        h.requesting_challenge = 0;
    });

    // Teleport both players to arena
    // Accepter goes to (135, 115), requester goes to (120, 115)
    use crate::handler::zone_change;
    zone_change::trigger_zone_change(
        session,
        ZONE_ARENA,
        PVP_ARENA_ACCEPTER_X,
        PVP_ARENA_ACCEPTER_Z,
    )
    .await?;

    send_zone_change(
        &world,
        requester_sid,
        ZONE_ARENA,
        PVP_ARENA_REQUESTER_X,
        PVP_ARENA_REQUESTER_Z,
        requester_nation,
    );

    debug!(
        "[{}] Challenge PVP accept: arena zone change triggered",
        session.addr(),
    );

    Ok(())
}

/// CHALLENGE_CVC_REQUEST (6): Request a clan vs clan battle.
async fn handle_cvc_request(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Validate self state
    let (self_requesting, self_requested, _, self_zone_id) = match world.with_session(sid, |h| {
        (
            h.requesting_challenge,
            h.challenge_requested,
            h.challenge_user,
            h.position.zone_id,
        )
    }) {
        Some(v) => v,
        None => return Ok(()),
    };

    if self_requesting != 0
        || self_requested != 0
        || self_zone_id == ZONE_ARENA
        || world.is_in_party(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
    {
        session
            .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
            .await?;
        return Ok(());
    }

    // Must be clan leader
    if !world.is_session_clan_leader(sid) {
        session
            .send_packet(&build_error(CHALLENGE_CLAN_ERROR))
            .await?;
        return Ok(());
    }

    // Zone restriction
    if is_nation_pvp_zone(&world, self_zone_id)
        || self_zone_id == crate::world::ZONE_DELOS
        || world
            .get_zone(self_zone_id)
            .is_some_and(|z| z.is_war_zone())
    {
        session
            .send_packet(&build_error(CHALLENGE_ZONE_ERROR))
            .await?;
        return Ok(());
    }

    // Read target name
    let target_name = match reader.read_sbyte_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let target_sid = match world.find_session_by_name(&target_name) {
        Some(id) => id,
        None => {
            session
                .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
                .await?;
            return Ok(());
        }
    };

    // Validate target
    let target_ok = world
        .with_session(target_sid, |h| {
            let ch = match &h.character {
                Some(c) => c,
                None => return false,
            };
            ch.res_hp_type != crate::world::USER_DEAD
                && h.requesting_challenge == 0
                && h.challenge_requested == 0
                && h.position.zone_id == self_zone_id
        })
        .unwrap_or(false);

    if !target_ok
        || world.is_in_party(target_sid)
        || world.is_trading(target_sid)
        || world.is_merchanting(target_sid)
    {
        session
            .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
            .await?;
        return Ok(());
    }

    // Target must also be a clan leader
    if !world.is_session_clan_leader(target_sid) {
        session
            .send_packet(&build_error(CHALLENGE_CLAN_ERROR))
            .await?;
        return Ok(());
    }

    // Get self name
    let self_name = match world.get_character_info(sid) {
        Some(ch) => ch.name,
        None => return Ok(()),
    };

    // Set challenge state
    world.update_session(sid, |h| {
        h.requesting_challenge = CHALLENGE_CVC_CANCEL;
        h.challenge_user = target_sid as i16;
    });
    world.update_session(target_sid, |h| {
        h.challenge_requested = CHALLENGE_CVC_REJECT;
        h.challenge_user = sid as i16;
    });

    // Send request to target
    let mut to_target = Packet::new(Opcode::WizChallenge as u8);
    to_target.write_u8(CHALLENGE_CVC_REQUEST);
    to_target.write_sbyte_string(&self_name);
    world.send_to_session_owned(target_sid, to_target);

    // Send confirmation to requester
    let mut to_self = Packet::new(Opcode::WizChallenge as u8);
    to_self.write_u8(CHALLENGE_CVC_REQ_SENT);
    to_self.write_sbyte_string(&target_name);
    session.send_packet(&to_self).await?;

    debug!(
        "[{}] Challenge CVC request: {} -> {}",
        session.addr(),
        self_name,
        target_name,
    );

    Ok(())
}

/// CHALLENGE_CVC_ACCEPT (8): Accept a clan vs clan battle.
async fn handle_cvc_accept(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Must have a pending CVC challenge
    let (_, challenge_requested, challenge_user) = world.get_challenge_state(sid);

    if challenge_requested == 0 {
        return Ok(());
    }

    let self_clan_id = world.get_session_clan_id(sid);
    if self_clan_id == 0 {
        return Ok(());
    }

    // Clear challenge state on self FIRST (matching C++ order)
    world.update_session(sid, |h| {
        h.challenge_user = -1;
        h.challenge_requested = 0;
    });

    let requester_sid = challenge_user as SessionId;

    let requester_clan_id = world.get_session_clan_id(requester_sid);
    if requester_clan_id == 0 {
        session
            .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
            .await?;
        return Ok(());
    }

    // Clear challenge state on requester
    world.update_session(requester_sid, |h| {
        h.challenge_user = -1;
        h.requesting_challenge = 0;
    });

    // Get the zone both clan leaders are in (for member eligibility check)
    let origin_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    // Teleport eligible members from both clans to arena
    // Clan 1 (accepter's clan) -> (128, 125)
    let clan1_members = world.get_cvc_eligible_clan_members(self_clan_id, origin_zone);
    for (member_sid, nation) in clan1_members {
        send_zone_change(
            &world,
            member_sid,
            ZONE_ARENA,
            CVC_ARENA_CLAN1_X,
            CVC_ARENA_CLAN1_Z,
            nation,
        );
    }

    // Clan 2 (requester's clan) -> (135, 120)
    let clan2_members = world.get_cvc_eligible_clan_members(requester_clan_id, origin_zone);
    for (member_sid, nation) in clan2_members {
        send_zone_change(
            &world,
            member_sid,
            ZONE_ARENA,
            CVC_ARENA_CLAN2_X,
            CVC_ARENA_CLAN2_Z,
            nation,
        );
    }

    debug!(
        "[{}] Challenge CVC accept: clans {} vs {} to arena",
        session.addr(),
        self_clan_id,
        requester_clan_id,
    );

    Ok(())
}

/// CHALLENGE_PVP_CANCEL (2) / CHALLENGE_CVC_CANCEL (7): Challenger cancels the request.
async fn handle_cancelled(session: &mut ClientSession, opcode: u8) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (requesting, _, challenge_user) = world.get_challenge_state(sid);

    // Must be the one who initiated the challenge
    if requesting == 0 {
        return Ok(());
    }

    let target_sid = challenge_user as SessionId;

    // Check target still has us as their challenge partner
    let (_, _, target_challenge_user) = world.get_challenge_state(target_sid);
    let target_matches = target_challenge_user == sid as i16;

    if !target_matches {
        session
            .send_packet(&build_error(CHALLENGE_GENERIC_ERROR))
            .await?;
    } else {
        // Clear target's challenge state and notify them
        world.update_session(target_sid, |h| {
            h.challenge_user = -1;
            h.challenge_requested = 0;
        });

        let mut result = Packet::new(Opcode::WizChallenge as u8);
        result.write_u8(opcode);
        world.send_to_session_owned(target_sid, result);
    }

    // Clear our challenge state
    world.update_session(sid, |h| {
        h.challenge_user = -1;
        h.requesting_challenge = 0;
    });

    debug!("[{}] Challenge cancel (sub={})", session.addr(), opcode);

    Ok(())
}

/// CHALLENGE_PVP_REJECT (4) / CHALLENGE_CVC_REJECT (9): Challengee rejects the request.
async fn handle_rejected(session: &mut ClientSession, opcode: u8) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (_, requested, challenge_user) = world.get_challenge_state(sid);

    // Must be the one who was challenged
    if requested == 0 {
        return Ok(());
    }

    let requester_sid = challenge_user as SessionId;

    // Build the result packet
    let mut result = Packet::new(Opcode::WizChallenge as u8);

    // Check requester still has us as their challenge partner
    let (_, _, req_challenge_user) = world.get_challenge_state(requester_sid);
    let requester_matches = req_challenge_user == sid as i16;

    if !requester_matches {
        result.write_u8(CHALLENGE_GENERIC_ERROR);
    } else {
        // Clear requester's challenge state and notify them
        world.update_session(requester_sid, |h| {
            h.challenge_user = -1;
            h.requesting_challenge = 0;
        });

        result.write_u8(opcode);
        world.send_to_session(requester_sid, &result);
    }

    // Clear our challenge state
    world.update_session(sid, |h| {
        h.challenge_user = -1;
        h.challenge_requested = 0;
    });

    // C++ sends result to self regardless
    session.send_packet(&result).await?;

    debug!("[{}] Challenge reject (sub={})", session.addr(), opcode);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::Packet;

    #[test]
    fn test_challenge_pvp_request_packet_format() {
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_PVP_REQUEST);
        pkt.write_sbyte_string("TestPlayer");

        assert_eq!(pkt.opcode, Opcode::WizChallenge as u8);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_PVP_REQUEST);
        assert_eq!(reader.read_sbyte_string().unwrap(), "TestPlayer");
    }

    #[test]
    fn test_challenge_pvp_req_sent_packet_format() {
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_PVP_REQ_SENT);
        pkt.write_sbyte_string("TargetPlayer");

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_PVP_REQ_SENT);
        assert_eq!(reader.read_sbyte_string().unwrap(), "TargetPlayer");
    }

    #[test]
    fn test_challenge_cvc_request_packet_format() {
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_CVC_REQUEST);
        pkt.write_sbyte_string("ClanLeader");

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CVC_REQUEST);
        assert_eq!(reader.read_sbyte_string().unwrap(), "ClanLeader");
    }

    #[test]
    fn test_challenge_cvc_req_sent_packet_format() {
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_CVC_REQ_SENT);
        pkt.write_sbyte_string("TargetLeader");

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CVC_REQ_SENT);
        assert_eq!(reader.read_sbyte_string().unwrap(), "TargetLeader");
    }

    #[test]
    fn test_challenge_error_packet_format() {
        let pkt = build_error(CHALLENGE_GENERIC_ERROR);
        assert_eq!(pkt.opcode, Opcode::WizChallenge as u8);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_GENERIC_ERROR);
    }

    #[test]
    fn test_challenge_zone_error_packet_format() {
        let pkt = build_error(CHALLENGE_ZONE_ERROR);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_ZONE_ERROR);
    }

    #[test]
    fn test_challenge_clan_error_packet_format() {
        let pkt = build_error(CHALLENGE_CLAN_ERROR);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CLAN_ERROR);
    }

    #[test]
    fn test_challenge_cancel_packet_format() {
        // PVP cancel
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_PVP_CANCEL);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_PVP_CANCEL);

        // CVC cancel
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_CVC_CANCEL);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CVC_CANCEL);
    }

    #[test]
    fn test_challenge_reject_packet_format() {
        // PVP reject
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_PVP_REJECT);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_PVP_REJECT);

        // CVC reject
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_CVC_REJECT);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CVC_REJECT);
    }

    #[test]
    fn test_challenge_accept_packet_format() {
        // PVP accept (no data besides sub-opcode)
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_PVP_ACCEPT);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_PVP_ACCEPT);

        // CVC accept
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_CVC_ACCEPT);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CVC_ACCEPT);
    }

    #[test]
    fn test_sub_opcode_constants() {
        assert_eq!(CHALLENGE_PVP_REQUEST, 1);
        assert_eq!(CHALLENGE_PVP_CANCEL, 2);
        assert_eq!(CHALLENGE_PVP_ACCEPT, 3);
        assert_eq!(CHALLENGE_PVP_REJECT, 4);
        assert_eq!(CHALLENGE_PVP_REQ_SENT, 5);
        assert_eq!(CHALLENGE_CVC_REQUEST, 6);
        assert_eq!(CHALLENGE_CVC_CANCEL, 7);
        assert_eq!(CHALLENGE_CVC_ACCEPT, 8);
        assert_eq!(CHALLENGE_CVC_REJECT, 9);
        assert_eq!(CHALLENGE_CVC_REQ_SENT, 10);
        assert_eq!(CHALLENGE_GENERIC_ERROR, 11);
        assert_eq!(CHALLENGE_ZONE_ERROR, 12);
        assert_eq!(CHALLENGE_CLAN_ERROR, 13);
    }

    #[test]
    fn test_arena_zone_constant() {
        assert_eq!(ZONE_ARENA, 48);
    }

    /// Verify PVP arena spawn coordinates match C++ constants.
    ///
    #[test]
    fn test_pvp_arena_coordinates() {
        assert_eq!(PVP_ARENA_ACCEPTER_X, 135.0);
        assert_eq!(PVP_ARENA_ACCEPTER_Z, 115.0);
        assert_eq!(PVP_ARENA_REQUESTER_X, 120.0);
        assert_eq!(PVP_ARENA_REQUESTER_Z, 115.0);
    }

    /// Verify CVC arena spawn coordinates match C++ constants.
    ///
    #[test]
    fn test_cvc_arena_coordinates() {
        assert_eq!(CVC_ARENA_CLAN1_X, 128.0);
        assert_eq!(CVC_ARENA_CLAN1_Z, 125.0);
        assert_eq!(CVC_ARENA_CLAN2_X, 135.0);
        assert_eq!(CVC_ARENA_CLAN2_Z, 120.0);
    }

    /// Zone change packet sent to remote session must match the expected wire format.
    ///
    /// Format: [opcode=WizZoneChange] [u8:3] [u16:zone] [u16:0] [u16:x*10] [u16:z*10] [u16:0] [u8:nation] [u16:0xFFFF]
    #[test]
    fn test_send_zone_change_packet_format() {
        // Simulate the packet that send_zone_change would create
        let zone_id: u16 = ZONE_ARENA;
        let x: f32 = 135.0;
        let z: f32 = 115.0;
        let nation: u8 = 1;

        let mut zpkt = Packet::new(Opcode::WizZoneChange as u8);
        zpkt.write_u8(3); // ZONE_CHANGE_TELEPORT
        zpkt.write_u16(zone_id);
        zpkt.write_u16(0);
        zpkt.write_u16((x * 10.0) as u16);
        zpkt.write_u16((z * 10.0) as u16);
        zpkt.write_u16(0);
        zpkt.write_u8(nation);
        zpkt.write_u16(0xFFFF);

        assert_eq!(zpkt.opcode, Opcode::WizZoneChange as u8);
        let mut reader = PacketReader::new(&zpkt.data);
        assert_eq!(reader.read_u8().unwrap(), 3);
        assert_eq!(reader.read_u16().unwrap(), 48);
        assert_eq!(reader.read_u16().unwrap(), 0);
        assert_eq!(reader.read_u16().unwrap(), 1350); // 135.0 * 10
        assert_eq!(reader.read_u16().unwrap(), 1150); // 115.0 * 10
        assert_eq!(reader.read_u16().unwrap(), 0);
        assert_eq!(reader.read_u8().unwrap(), 1);
        assert_eq!(reader.read_u16().unwrap(), 0xFFFF);
    }

    /// All error codes produce a valid 1-byte payload packet.
    #[test]
    fn test_all_error_codes_roundtrip() {
        for code in [
            CHALLENGE_GENERIC_ERROR,
            CHALLENGE_ZONE_ERROR,
            CHALLENGE_CLAN_ERROR,
        ] {
            let pkt = build_error(code);
            assert_eq!(pkt.opcode, Opcode::WizChallenge as u8);
            assert_eq!(pkt.data.len(), 1);
            assert_eq!(pkt.data[0], code);
        }
    }

    /// PVP request+sent packet pair must have SByte string (u8 length prefix).
    #[test]
    fn test_pvp_sbyte_string_encoding() {
        let name = "KnightPlayer";
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_PVP_REQUEST);
        pkt.write_sbyte_string(name);

        // Expected layout: [1(sub)] [12(strlen as u8)] [12 bytes name]
        assert_eq!(pkt.data.len(), 1 + 1 + name.len());

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), 1);
        let decoded = reader.read_sbyte_string().unwrap();
        assert_eq!(decoded, name);
    }

    /// CVC request+sent packet pair must have SByte string.
    #[test]
    fn test_cvc_sbyte_string_encoding() {
        let name = "ClanChief";
        let mut pkt = Packet::new(Opcode::WizChallenge as u8);
        pkt.write_u8(CHALLENGE_CVC_REQUEST);
        pkt.write_sbyte_string(name);

        assert_eq!(pkt.data.len(), 1 + 1 + name.len());

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), CHALLENGE_CVC_REQUEST);
        let decoded = reader.read_sbyte_string().unwrap();
        assert_eq!(decoded, name);
    }

    /// Verify that the cancel sub-opcodes match the requesting_challenge values
    /// that are set during request (C++ pattern: cancel opcode == requesting_challenge).
    #[test]
    fn test_challenge_state_opcode_symmetry() {
        // PVP: requesting_challenge is set to CHALLENGE_PVP_CANCEL (2)
        // and challengee's challenge_requested is set to CHALLENGE_PVP_REJECT (4)
        assert_eq!(CHALLENGE_PVP_CANCEL, 2);
        assert_eq!(CHALLENGE_PVP_REJECT, 4);

        // CVC: requesting_challenge is set to CHALLENGE_CVC_CANCEL (7)
        // and challengee's challenge_requested is set to CHALLENGE_CVC_REJECT (9)
        assert_eq!(CHALLENGE_CVC_CANCEL, 7);
        assert_eq!(CHALLENGE_CVC_REJECT, 9);
    }
}
