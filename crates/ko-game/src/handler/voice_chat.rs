//! WIZ_SOUND (0xB5) handler — party voice chat control (game socket half).
//!
//! Protocol recovered from the 2625 client decompile (`CVoiceChatMng`,
//! `sub_EC7AF0` S2C dispatcher, `sub_EC66A0` per-frame state machine,
//! `sub_EC9590` relay connect, `sub_704DA0` TCP connect); see
//! `docs/reverse-engineering/2625/VOICE_CHAT_PROTOCOL.md`.
//!
//! Flow (client view):
//! 1. Voice window open / Reconnect: C2S `[05]`.
//! 2. Server: S2C `[02][u16 room][u64 token][u8 0]` (room/token globals), then
//!    S2C `[05][SByte host][u32 port][SByte ""][SByte ""][u32 a][u32 b]`.
//!    `host` MUST be a dotted-IP (or DNS) string: the client passes it to
//!    `inet_addr`/`gethostbyname` and `connect()`s to `host:port` — anything else
//!    makes the connect fail and the UI shows "Failed". `a`/`b` are opaque and are
//!    echoed back verbatim in the relay login frame; we put the token's low/high
//!    halves there so the relay can authenticate the login.
//! 3. Client connects to the relay (see `crate::systems::voice_relay`).
//! 4. If the client has no room yet it sends C2S `[06]` (re-request room): we
//!    answer with S2C `[02]` again.
//!
//! Other C2S subs (`01` live-on-relay confirm, `02` self-join ack, `03`/`09`
//! acks, `07` volume) carry nothing the server needs and are ignored.

use std::net::Ipv4Addr;
use std::sync::LazyLock;

use dashmap::DashMap;
use tracing::debug;

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};
use crate::zone::SessionId;

/// C2S: legacy guess for "request a voice room", still accepted as an alias.
const SUB_JOIN_REQUEST: u8 = 0x04;

/// C2S: the real "open voice / Reconnect" request sent by the client (`sub_EC4800`).
const SUB_OPEN_REQUEST: u8 = 0x05;

/// C2S: "I have no room/token yet" re-request sent from the client tick (`sub_EC66A0`).
const SUB_ROOM_REQUEST: u8 = 0x06;

/// S2C: room join/create result.
const SUB_ROOM_RESULT: u8 = 0x02;

/// S2C: relay connection info (`host`, `port`, two opaque u32s echoed in the relay login).
const SUB_RELAY_INFO: u8 = 0x05;

/// Sentinel room id the client treats as "join failed".
const ROOM_ID_NONE: u16 = 0xFFFF;

/// A voice-relay login token, tying a raw relay TCP connection back to the
/// game session and party room it was issued for.
struct VoiceToken {
    room_id: u16,
    session_id: SessionId,
    name: String,
}

/// What a valid token resolves to for the relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    pub room_id: u16,
    pub name: String,
}

/// Outstanding tokens, keyed by the random 64-bit value handed to the client.
/// Looked up by `crate::systems::voice_relay` when a raw connection logs in.
static TOKENS: LazyLock<DashMap<u64, VoiceToken>> = LazyLock::new(DashMap::new);

/// Return the token for `session_id` in `room_id`, issuing a fresh one when the
/// session has none (or its old one belongs to another room). Reusing keeps a
/// relay connection that already logged in valid across client re-requests.
fn issue_token(session_id: SessionId, room_id: u16, name: &str) -> u64 {
    if let Some(existing) = TOKENS
        .iter()
        .find(|e| e.value().session_id == session_id && e.value().room_id == room_id)
        .map(|e| *e.key())
    {
        return existing;
    }
    TOKENS.retain(|_, v| v.session_id != session_id);
    loop {
        let token = rand::random::<u64>();
        if token == 0 || token == u64::MAX || TOKENS.contains_key(&token) {
            continue;
        }
        TOKENS.insert(
            token,
            VoiceToken {
                room_id,
                session_id,
                name: name.to_string(),
            },
        );
        return token;
    }
}

/// Validate a token presented by the relay login. Returns the room id and the
/// character name it was issued for.
pub fn validate_token(token: u64) -> Option<TokenInfo> {
    TOKENS.get(&token).map(|t| TokenInfo {
        room_id: t.room_id,
        name: t.name.clone(),
    })
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
        SUB_JOIN_REQUEST | SUB_OPEN_REQUEST => handle_join_request(session, true).await,
        SUB_ROOM_REQUEST => handle_join_request(session, false).await,
        _ => {
            debug!(
                "[{}] WIZ_SOUND (0xB5): ignored inbound sub-opcode 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle a voice-room join/create request, tied to the sender's current party.
/// Players not in a party cannot open voice chat — there is no room to join.
/// `send_relay` is false for the `[06]` re-request, which only wants the room result.
async fn handle_join_request(session: &mut ClientSession, send_relay: bool) -> anyhow::Result<()> {
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

    let name = world.get_session_name(sid).unwrap_or_default();
    let token = issue_token(sid, room_id, &name);
    session
        .send_packet(&build_room_result(room_id, token))
        .await?;

    let (relay_ip, relay_port) = relay_public_addr();
    if send_relay {
        session
            .send_packet(&build_relay_info(relay_ip, relay_port, token))
            .await?;
    }

    tracing::info!(
        "[{}] Voice chat: sid={} ('{}') joined voice room (party_id={}), relay={}:{} (relay_info={})",
        session.addr(),
        sid,
        name,
        room_id,
        relay_ip,
        relay_port,
        send_relay,
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

/// Build the S2C relay-connection packet:
/// `[05][SByte host][u32 port][SByte ""][SByte ""][u32 token_lo][u32 token_hi]`.
/// `host` is the dotted-quad string the client feeds to `inet_addr`.
fn build_relay_info(relay_ip: Ipv4Addr, relay_port: u16, token: u64) -> Packet {
    let mut pkt = Packet::new(Opcode::WizSound as u8);
    pkt.write_u8(SUB_RELAY_INFO);
    pkt.write_sbyte_string(&relay_ip.to_string());
    pkt.write_u32(relay_port as u32);
    pkt.write_sbyte_string("");
    pkt.write_sbyte_string("");
    pkt.write_u32(token as u32);
    pkt.write_u32((token >> 32) as u32);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_opcode_values() {
        assert_eq!(SUB_JOIN_REQUEST, 0x04);
        assert_eq!(SUB_OPEN_REQUEST, 0x05);
        assert_eq!(SUB_ROOM_REQUEST, 0x06);
        assert_eq!(SUB_ROOM_RESULT, 0x02);
        assert_eq!(SUB_RELAY_INFO, 0x05);
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
    fn test_relay_info_format() {
        let ip = Ipv4Addr::new(203, 0, 113, 7);
        let token = 0x1122_3344_5566_7788u64;
        let pkt = build_relay_info(ip, 15500, token);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RELAY_INFO));
        // The client connect()s to this string via inet_addr: must be dotted-quad.
        assert_eq!(r.read_sbyte_string().as_deref(), Some("203.0.113.7"));
        assert_eq!(r.read_u32(), Some(15500));
        assert_eq!(r.read_sbyte_string().as_deref(), Some(""));
        assert_eq!(r.read_sbyte_string().as_deref(), Some(""));
        assert_eq!(r.read_u32(), Some(0x5566_7788));
        assert_eq!(r.read_u32(), Some(0x1122_3344));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_issue_token_is_nonzero_and_unique() {
        let t1 = issue_token(1001, 10, "A");
        let t2 = issue_token(1002, 10, "B");
        assert_ne!(t1, 0);
        assert_ne!(t2, 0);
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_issue_and_validate_token_roundtrip() {
        let token = issue_token(1005, 99, "Knight");
        assert_eq!(
            validate_token(token),
            Some(TokenInfo {
                room_id: 99,
                name: "Knight".into()
            })
        );
    }

    #[test]
    fn test_issue_token_reuses_for_same_session_and_room() {
        let t1 = issue_token(1077, 12, "X");
        assert_eq!(issue_token(1077, 12, "X"), t1);
        let t2 = issue_token(1077, 13, "X");
        assert_ne!(t1, t2);
        assert_eq!(validate_token(t1), None);
    }

    #[test]
    fn test_validate_unknown_token_is_none() {
        assert_eq!(validate_token(0xFFFF_FFFF_FFFF_FFFF), None);
    }

    #[test]
    fn test_relay_public_addr_defaults() {
        std::env::remove_var("VOICE_RELAY_BIND_ADDR");
        std::env::remove_var("VOICE_RELAY_PUBLIC_IP");
        let (ip, port) = relay_public_addr();
        assert_eq!(ip, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(port, 15500);
    }
}
