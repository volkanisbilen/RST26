//! WIZ_SOUND (0xB5) handler — party voice chat control (C2S).
//! ## Background
//! Full wire-protocol findings from static analysis of the client binary live in
//! `docs/reverse-engineering/2625/VOICE_CHAT_PROTOCOL.md`. Summary relevant here:
//! voice chat uses two TCP connections — this (the normal encrypted game socket,
//! carrying only small control messages under 0xB5) and a separate, unencrypted
//! "voice relay" socket (see `crate::systems::voice_relay`) that carries the actual
//! Opus audio frames. This file implements the game-socket half: room creation
//! tied to the player's current party, token issuance, and telling the client
//! where to open its second (relay) connection.
//! ## Sub-opcodes (first byte of the 0xB5 payload)
//! Confirmed by decompiling `CVoiceChatMng`'s packet handlers (`sub_EC7AF0` is the
//! client's S2C dispatcher for this opcode):
//! - S2C `0x02` — room join/create result: `[u16 room_id (0xFFFF = failed)]
//!   [u64 token] [u8 reserved]`. On success the client just stores room_id/token;
//!   it does not echo anything back for this specific message.
//! - S2C `0x05` — per-member roster broadcast: `[SByte name] [u32 member_id]
//!   [SByte str2] [SByte str3] [u32 val1] [u32 val2]`. The trailing two u32 fields
//!   were not pinned down with certainty (see the doc's "Open items"). This
//!   implementation's working assumption: for the joining player's OWN entry,
//!   those two fields double as the voice relay's address (`val1` = IPv4 as u32,
//!   `val2` = port) — this is how the client learns where to open its relay
//!   socket, since no other inbound sub-opcode in the dispatcher reads an
//!   address. Flagged in the doc as needing live-client confirmation.
//!
//! The C2S "please create/join my party's voice room" trigger opcode was NOT
//! directly observed in this pass (only its *response*, S2C 0x02, was). `0x04`
//! is used here as a reasoned placeholder (RE'd outbound sends already use
//! 0x01/0x02/0x03/0x07/0x09 for other purposes — see the doc — leaving 0x04 as
//! the doc's original "room-related" bucket entry least likely to collide).

use std::net::Ipv4Addr;
use std::sync::LazyLock;

use dashmap::DashMap;
use tracing::debug;

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::zone::SessionId;

/// C2S: request a voice room for the sender's current party (assumed — see module docs).
const SUB_JOIN_REQUEST: u8 = 0x04;

/// S2C: room join/create result.
const SUB_ROOM_RESULT: u8 = 0x02;

/// S2C: per-member roster broadcast (doubles as relay address delivery for self).
const SUB_MEMBER_INFO: u8 = 0x05;

/// Sentinel room id the client treats as "join failed".
const ROOM_ID_NONE: u16 = 0xFFFF;

/// A voice-relay login token, tying a raw relay TCP connection back to the
/// game session and party room it was issued for.
struct VoiceToken {
    room_id: u16,
    #[allow(dead_code)] // kept for future per-member relay bookkeeping
    session_id: SessionId,
}

/// Outstanding tokens, keyed by the random 64-bit value handed to the client.
/// Looked up by `crate::systems::voice_relay` when a raw connection logs in.
static TOKENS: LazyLock<DashMap<u64, VoiceToken>> = LazyLock::new(DashMap::new);

/// Issue a fresh voice-relay token for `session_id` joining `room_id`.
/// Collisions are vanishingly unlikely with a 64-bit space but checked anyway.
fn issue_token(session_id: SessionId, room_id: u16) -> u64 {
    loop {
        let token = rand::random::<u64>();
        if token == 0 {
            continue;
        }
        if TOKENS.contains_key(&token) {
            continue;
        }
        TOKENS.insert(
            token,
            VoiceToken {
                room_id,
                session_id,
            },
        );
        return token;
    }
}

/// Validate a token presented by the voice relay's raw-socket login
/// (relay sub-opcode `0x01`). Returns the room id it was issued for.
pub fn validate_token(token: u64) -> Option<u16> {
    TOKENS.get(&token).map(|t| t.room_id)
}

/// The address told to clients for opening their voice-relay connection.
/// Port is read from `VOICE_RELAY_BIND_ADDR`'s port (same variable
/// `crate::systems::voice_relay` binds to); the IP is a separate var since a
/// server normally binds `0.0.0.0` but must advertise its real public IP.
fn relay_public_addr() -> (Ipv4Addr, u16) {
    let bind = std::env::var("VOICE_RELAY_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:15500".into());
    let port = bind
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(15500);
    let ip = std::env::var("VOICE_RELAY_PUBLIC_IP")
        .ok()
        .and_then(|s| s.parse::<Ipv4Addr>().ok())
        .unwrap_or(Ipv4Addr::new(127, 0, 0, 1));
    (ip, port)
}

/// Handle WIZ_SOUND (0xB5) sent by the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = match reader.read_u8() {
        Some(op) => op,
        None => return Ok(()),
    };

    match sub_opcode {
        // Live trace: the 2625 client sends 0x05 (1-byte payload) when the voice window opens and
        // on its Reconnect button; 0x04 was only the original guess. Both ask for a room.
        SUB_JOIN_REQUEST | SUB_MEMBER_INFO => handle_join_request(session).await,
        _ => {
            debug!(
                "[{}] WIZ_SOUND (0xB5): unhandled inbound sub-opcode 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle a voice-room join/create request, tied to the sender's current party.
/// Players not in a party cannot open voice chat — there is no room to join.
async fn handle_join_request(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let room_id = match world.get_party_id(sid) {
        Some(pid) => pid,
        None => {
            debug!(
                "[{}] Voice chat join request: sid={} not in a party, rejecting",
                session.addr(),
                sid
            );
            session
                .send_packet(&build_room_result(ROOM_ID_NONE, 0))
                .await?;
            return Ok(());
        }
    };

    let token = issue_token(sid, room_id);
    session
        .send_packet(&build_room_result(room_id, token))
        .await?;

    let name = world.get_session_name(sid).unwrap_or_default();
    let (relay_ip, relay_port) = relay_public_addr();
    session
        .send_packet(&build_member_info(&name, sid as u32, relay_ip, relay_port))
        .await?;

    tracing::info!(
        "[{}] Voice chat: sid={} ('{}') joined voice room (party_id={}), relay={}:{}",
        session.addr(),
        sid,
        name,
        room_id,
        relay_ip,
        relay_port,
    );
    Ok(())
}

/// Build the S2C room join/create result: `[u16 room_id][u64 token][u8 reserved]`.
/// `room_id == 0xFFFF` tells the client the request failed; `token` is only
/// meaningful on success.
fn build_room_result(room_id: u16, token: u64) -> Packet {
    let mut pkt = Packet::new(Opcode::WizSound as u8);
    pkt.write_u8(SUB_ROOM_RESULT);
    pkt.write_u16(room_id);
    pkt.write_u64(token);
    pkt.write_u8(0); // reserved — client reads and discards this byte
    pkt
}

/// Build the S2C member roster entry used here to also deliver the relay address.
/// `[SByte name][u32 member_id][SByte ""][SByte ""][u32 relay_ip][u32 relay_port]`.
fn build_member_info(name: &str, member_id: u32, relay_ip: Ipv4Addr, relay_port: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizSound as u8);
    pkt.write_u8(SUB_MEMBER_INFO);
    pkt.write_sbyte_string(name);
    pkt.write_u32(member_id);
    pkt.write_sbyte_string("");
    pkt.write_sbyte_string("");
    pkt.write_u32(u32::from(relay_ip));
    pkt.write_u32(relay_port as u32);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_opcode_values() {
        assert_eq!(SUB_JOIN_REQUEST, 0x04);
        assert_eq!(SUB_ROOM_RESULT, 0x02);
        assert_eq!(SUB_MEMBER_INFO, 0x05);
    }

    #[test]
    fn test_dispatch_reads_sub_opcode_first() {
        // Mirrors `handle()`'s own parsing without needing a live ClientSession.
        let mut pkt = Packet::new(Opcode::WizSound as u8);
        pkt.write_u8(SUB_JOIN_REQUEST);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_JOIN_REQUEST));
    }

    #[test]
    fn test_dispatch_empty_payload_is_ignored_not_panicked() {
        let pkt = Packet::new(Opcode::WizSound as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), None);
    }

    #[test]
    fn test_room_result_success_format() {
        let pkt = build_room_result(42, 0xDEAD_BEEF_0011_2233);
        assert_eq!(pkt.opcode, Opcode::WizSound as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_ROOM_RESULT));
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.read_u64(), Some(0xDEAD_BEEF_0011_2233));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_room_result_failure_format() {
        let pkt = build_room_result(ROOM_ID_NONE, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_ROOM_RESULT));
        assert_eq!(r.read_u16(), Some(0xFFFF));
    }

    #[test]
    fn test_member_info_format() {
        let ip = Ipv4Addr::new(203, 0, 113, 7);
        let pkt = build_member_info("Knight", 1234, ip, 15500);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_MEMBER_INFO));
        assert_eq!(r.read_sbyte_string().as_deref(), Some("Knight"));
        assert_eq!(r.read_u32(), Some(1234));
        assert_eq!(r.read_sbyte_string().as_deref(), Some(""));
        assert_eq!(r.read_sbyte_string().as_deref(), Some(""));
        assert_eq!(r.read_u32(), Some(u32::from(ip)));
        assert_eq!(r.read_u32(), Some(15500));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_issue_token_is_nonzero_and_unique() {
        let t1 = issue_token(1, 10);
        let t2 = issue_token(2, 10);
        assert_ne!(t1, 0);
        assert_ne!(t2, 0);
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_issue_and_validate_token_roundtrip() {
        let token = issue_token(5, 99);
        assert_eq!(validate_token(token), Some(99));
    }

    #[test]
    fn test_validate_unknown_token_is_none() {
        assert_eq!(validate_token(0xFFFF_FFFF_FFFF_FFFF), None);
    }

    #[test]
    fn test_relay_public_addr_defaults() {
        // No env vars set in the default test environment: falls back to
        // loopback + the documented default port.
        std::env::remove_var("VOICE_RELAY_BIND_ADDR");
        std::env::remove_var("VOICE_RELAY_PUBLIC_IP");
        let (ip, port) = relay_public_addr();
        assert_eq!(ip, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(port, 15500);
    }
}
