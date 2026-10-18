//! Voice-relay TCP server — the second connection of party voice chat.
//!
//! Wire format recovered from the 2625 client decompile (`sub_EC69A0` inbound
//! dispatcher, `sub_EC66A0` per-frame state machine, `sub_703750` raw sender);
//! see `docs/reverse-engineering/2625/VOICE_CHAT_PROTOCOL.md`.
//!
//! Framing (both directions, no encryption): `AA 55 | u16 len (LE) | payload |
//! 55 AA`. `payload[0]` is the sub-opcode. All integers little-endian, strings
//! are `SByte` (`u8 len` + bytes).
//!
//! Client -> relay:
//! - `01` login: `[SByte name][u32 a][u32 b]` — `a`/`b` are the two opaque u32s the
//!   game server sent in S2C `0xB5/05`; we put the token's low/high halves there.
//! - `04 02` join room: `[u16 room][u64 token][u16 0x0100]`.
//! - `04 01` create room: `[00 01]` — treated like a join of the login's room.
//! - `05` voice data: `[u16 len][opus bytes]`.
//! - `06` mic/speaker state: `[u16 mic][u16 spk]`.
//! - `07` keepalive (1 byte).
//!
//! Relay -> client:
//! - `01` login ack: `[u16 status (1 = ok, 0 = fail)][u16 member_id]`.
//! - `04 02` member joined: `[u8 1][u16 member_id][SByte name]` (also sent for the
//!   joiner itself; the client recognises its own id from the login ack).
//! - `04 03` member left: `[u16 1][u16 member_id][SByte name]`.
//! - `05` voice data: `[u16 sender_id][u16 len][opus bytes]`.
//! - `06` mic state: `[u16 sender_id][u16 mic][u16 spk]`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::handler::voice_chat;

const HEADER: u16 = 0xAA55;
const TAIL: u16 = 0x55AA;
/// Sanity bound against a malformed/hostile length prefix.
const MAX_FRAME: usize = 65_536;

const SUB_LOGIN: u8 = 0x01;
const SUB_ROOM: u8 = 0x04;
const SUB_VOICE_DATA: u8 = 0x05;
const SUB_MIC_STATE: u8 = 0x06;
const SUB_KEEPALIVE: u8 = 0x07;

const ROOM_SUB_CREATE: u8 = 0x01;
const ROOM_SUB_JOIN: u8 = 0x02;
const ROOM_SUB_LEFT: u8 = 0x03;

type RoomId = u16;

/// One relay connection registered in a room.
struct Member {
    conn_id: u64,
    id: u16,
    name: String,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    /// True once the client sent its join/create frame and was announced.
    joined: bool,
}

fn rooms() -> &'static Mutex<HashMap<RoomId, Vec<Member>>> {
    static S: OnceLock<Mutex<HashMap<RoomId, Vec<Member>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_conn_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Start the voice relay listener as a background task. A bind failure is
/// logged, not fatal to the game server as a whole.
pub fn start(bind_addr: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = match TcpListener::bind(&bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                warn!(%bind_addr, error = %e, "voice relay TCP server failed to bind, voice chat is unavailable");
                return;
            }
        };
        info!(address = %bind_addr, "voice relay TCP server listening");
        loop {
            let Ok((stream, peer)) = listener.accept().await else {
                continue;
            };
            debug!(%peer, "voice relay: connection accepted");
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream).await {
                    debug!(%peer, error = %e, "voice relay connection ended");
                }
            });
        }
    })
}

/// Login state of one connection.
struct Login {
    room_id: RoomId,
    token: u64,
}

async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let _ = stream.set_nodelay(true);
    let (mut read_half, write_half) = stream.into_split();
    let conn_id = next_conn_id();
    let mut login: Option<Login> = None;

    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let writer = tokio::spawn(write_loop(write_half, rx));

    let result = read_loop(&mut read_half, conn_id, &mut login, &tx).await;

    if let Some(l) = login {
        leave_room(l.room_id, conn_id);
    }
    drop(tx);
    // Let queued frames (e.g. a failed-login ack) flush before dropping the socket.
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), writer).await;
    result
}

async fn write_loop(mut write_half: OwnedWriteHalf, mut rx: mpsc::UnboundedReceiver<Vec<u8>>) {
    while let Some(frame) = rx.recv().await {
        if write_half.write_all(&frame).await.is_err() {
            break;
        }
    }
}

async fn read_loop(
    read_half: &mut OwnedReadHalf,
    conn_id: u64,
    login: &mut Option<Login>,
    tx: &mpsc::UnboundedSender<Vec<u8>>,
) -> anyhow::Result<()> {
    loop {
        let Some(payload) = read_frame(read_half).await? else {
            return Ok(());
        };
        if payload.is_empty() {
            continue;
        }
        match payload[0] {
            SUB_LOGIN => {
                let Some((name, token)) = parse_login(&payload) else {
                    debug!(conn_id, "voice relay: malformed login frame, closing");
                    return Ok(());
                };
                let Some(info) = voice_chat::validate_token(token) else {
                    debug!(conn_id, client_name = %name, "voice relay: invalid token, rejecting login");
                    let _ = tx.send(build_frame(&build_login_ack(None)));
                    return Ok(());
                };
                if login.is_some() {
                    debug!(conn_id, "voice relay: duplicate login ignored");
                    continue;
                }
                let member_id = register_member(info.room_id, conn_id, &info.name, tx.clone());
                let _ = tx.send(build_frame(&build_login_ack(Some(member_id))));
                *login = Some(Login {
                    room_id: info.room_id,
                    token,
                });
                info!(
                    conn_id,
                    room_id = info.room_id,
                    member_id,
                    name = %info.name,
                    "voice relay: login ok"
                );
            }
            SUB_ROOM => {
                let Some(l) = login.as_ref() else {
                    debug!(conn_id, "voice relay: room frame before login, ignoring");
                    continue;
                };
                let room_sub = payload.get(1).copied().unwrap_or(0);
                let ok = match room_sub {
                    ROOM_SUB_JOIN => parse_join(&payload)
                        .map(|(room, token)| room == l.room_id && token == l.token)
                        .unwrap_or(false),
                    // Create request (`04 01 00 01`): the room is fixed by the party,
                    // so treat it as a join of the login's room.
                    ROOM_SUB_CREATE => true,
                    _ => {
                        debug!(conn_id, room_sub, "voice relay: unknown room sub, ignoring");
                        continue;
                    }
                };
                if ok {
                    announce_join(l.room_id, conn_id);
                    info!(conn_id, room_id = l.room_id, "voice relay: member joined room");
                } else {
                    debug!(conn_id, "voice relay: join with wrong room/token, ignoring");
                }
            }
            SUB_VOICE_DATA => {
                if let Some(l) = login.as_ref() {
                    relay_voice(l.room_id, conn_id, &payload);
                }
            }
            SUB_MIC_STATE => {
                if let Some(l) = login.as_ref() {
                    relay_mic_state(l.room_id, conn_id, &payload);
                }
            }
            SUB_KEEPALIVE => {
                debug!(conn_id, "voice relay: keepalive");
            }
            other => {
                debug!(
                    conn_id,
                    sub_opcode = other,
                    "voice relay: unknown sub-opcode, ignoring"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Frame parsing / building (pure, unit-tested)
// ---------------------------------------------------------------------------

/// Parse `[01][SByte name][u32 a][u32 b]` into `(name, token)`; `token = a | b << 32`.
fn parse_login(payload: &[u8]) -> Option<(String, u64)> {
    let len = *payload.get(1)? as usize;
    let name = payload.get(2..2 + len)?;
    let rest = payload.get(2 + len..2 + len + 8)?;
    let a = u32::from_le_bytes(rest[0..4].try_into().ok()?);
    let b = u32::from_le_bytes(rest[4..8].try_into().ok()?);
    Some((
        String::from_utf8_lossy(name).into_owned(),
        (a as u64) | ((b as u64) << 32),
    ))
}

/// Parse `[04][02][u16 room][u64 token][u16 ..]` into `(room, token)`.
fn parse_join(payload: &[u8]) -> Option<(u16, u64)> {
    let room = u16::from_le_bytes(payload.get(2..4)?.try_into().ok()?);
    let token = u64::from_le_bytes(payload.get(4..12)?.try_into().ok()?);
    Some((room, token))
}

/// `[01][u16 status][u16 member_id]` — `None` builds the failure ack.
fn build_login_ack(member_id: Option<u16>) -> Vec<u8> {
    let mut p = vec![SUB_LOGIN];
    match member_id {
        Some(id) => {
            p.extend_from_slice(&1u16.to_le_bytes());
            p.extend_from_slice(&id.to_le_bytes());
        }
        None => {
            p.extend_from_slice(&0u16.to_le_bytes());
            p.extend_from_slice(&0xFFFFu16.to_le_bytes());
        }
    }
    p
}

fn push_sbyte(p: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    let n = b.len().min(255);
    p.push(n as u8);
    p.extend_from_slice(&b[..n]);
}

/// `[04][02][u8 1][u16 member_id][SByte name]`.
fn build_member_joined(id: u16, name: &str) -> Vec<u8> {
    let mut p = vec![SUB_ROOM, ROOM_SUB_JOIN, 1];
    p.extend_from_slice(&id.to_le_bytes());
    push_sbyte(&mut p, name);
    p
}

/// `[04][03][u16 1][u16 member_id][SByte name]`.
fn build_member_left(id: u16, name: &str) -> Vec<u8> {
    let mut p = vec![SUB_ROOM, ROOM_SUB_LEFT];
    p.extend_from_slice(&1u16.to_le_bytes());
    p.extend_from_slice(&id.to_le_bytes());
    push_sbyte(&mut p, name);
    p
}

/// Client voice frame `[05][u16 len][data]` -> relayed `[05][u16 sender][u16 len][data]`.
fn build_voice_out(sender: u16, payload: &[u8]) -> Option<Vec<u8>> {
    let len = u16::from_le_bytes(payload.get(1..3)?.try_into().ok()?) as usize;
    let data = payload.get(3..)?;
    let data = &data[..len.min(data.len())];
    let mut p = vec![SUB_VOICE_DATA];
    p.extend_from_slice(&sender.to_le_bytes());
    p.extend_from_slice(&(data.len() as u16).to_le_bytes());
    p.extend_from_slice(data);
    Some(p)
}

/// Client mic state `[06][u16 mic][u16 spk]` -> `[06][u16 sender][u16 mic][u16 spk]`.
fn build_mic_out(sender: u16, payload: &[u8]) -> Option<Vec<u8>> {
    let state = payload.get(1..5)?;
    let mut p = vec![SUB_MIC_STATE];
    p.extend_from_slice(&sender.to_le_bytes());
    p.extend_from_slice(state);
    Some(p)
}

/// Wrap `payload` in the `AA55 | len | payload | 55AA` frame for sending.
fn build_frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 6);
    out.extend_from_slice(&HEADER.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&TAIL.to_be_bytes());
    out
}

/// Read one `AA55 | len | payload | 55AA` frame. `Ok(None)` means the peer
/// closed the connection cleanly.
async fn read_frame(stream: &mut OwnedReadHalf) -> anyhow::Result<Option<Vec<u8>>> {
    let mut hdr = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut hdr).await {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e.into());
    }
    let header = u16::from_be_bytes([hdr[0], hdr[1]]);
    if header != HEADER {
        anyhow::bail!("bad voice relay frame header 0x{header:04X}");
    }
    let len = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
    if len == 0 || len > MAX_FRAME {
        anyhow::bail!("bad voice relay frame length {len}");
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;
    let mut tail = [0u8; 2];
    stream.read_exact(&mut tail).await?;
    if u16::from_be_bytes(tail) != TAIL {
        anyhow::bail!("bad voice relay frame tail");
    }
    Ok(Some(payload))
}

// ---------------------------------------------------------------------------
// Room registry
// ---------------------------------------------------------------------------

/// Add a connection to `room_id` and return its room-unique member id (>= 1).
/// A stale connection with the same character name is evicted first (reconnect).
fn register_member(
    room_id: RoomId,
    conn_id: u64,
    name: &str,
    tx: mpsc::UnboundedSender<Vec<u8>>,
) -> u16 {
    let mut rooms = rooms().lock();
    let members = rooms.entry(room_id).or_default();
    if let Some(pos) = members.iter().position(|m| m.name == name) {
        let old = members.remove(pos);
        if old.joined {
            let frame = build_frame(&build_member_left(old.id, &old.name));
            for m in members.iter().filter(|m| m.joined) {
                let _ = m.tx.send(frame.clone());
            }
        }
    }
    let id = (1u16..)
        .find(|id| members.iter().all(|m| m.id != *id))
        .unwrap_or(1);
    members.push(Member {
        conn_id,
        id,
        name: name.to_string(),
        tx,
        joined: false,
    });
    id
}

/// Mark `conn_id` joined: tell it about itself and every already-joined member,
/// and tell the already-joined members about it.
fn announce_join(room_id: RoomId, conn_id: u64) {
    let mut rooms = rooms().lock();
    let Some(members) = rooms.get_mut(&room_id) else {
        return;
    };
    let Some(me) = members.iter().position(|m| m.conn_id == conn_id) else {
        return;
    };
    members[me].joined = true;
    let my_frame = build_frame(&build_member_joined(members[me].id, &members[me].name));
    let _ = members[me].tx.send(my_frame.clone());
    for (i, other) in members.iter().enumerate() {
        if i == me || !other.joined {
            continue;
        }
        let _ = other.tx.send(my_frame.clone());
        let _ = members[me]
            .tx
            .send(build_frame(&build_member_joined(other.id, &other.name)));
    }
}

fn leave_room(room_id: RoomId, conn_id: u64) {
    let mut rooms = rooms().lock();
    if let Some(members) = rooms.get_mut(&room_id) {
        if let Some(pos) = members.iter().position(|m| m.conn_id == conn_id) {
            let gone = members.remove(pos);
            if gone.joined {
                let frame = build_frame(&build_member_left(gone.id, &gone.name));
                for m in members.iter().filter(|m| m.joined) {
                    let _ = m.tx.send(frame.clone());
                }
            }
        }
        if members.is_empty() {
            rooms.remove(&room_id);
        }
    }
}

/// Send `build(sender_id)` to every other joined member of the room.
fn fanout(
    room_id: RoomId,
    sender_conn_id: u64,
    build: impl Fn(u16) -> Option<Vec<u8>>,
) {
    let rooms = rooms().lock();
    let Some(members) = rooms.get(&room_id) else {
        return;
    };
    let Some(sender) = members
        .iter()
        .find(|m| m.conn_id == sender_conn_id && m.joined)
    else {
        return;
    };
    let Some(payload) = build(sender.id) else {
        return;
    };
    let frame = build_frame(&payload);
    for m in members {
        if m.conn_id != sender_conn_id && m.joined {
            let _ = m.tx.send(frame.clone());
        }
    }
}

fn relay_voice(room_id: RoomId, sender_conn_id: u64, payload: &[u8]) {
    fanout(room_id, sender_conn_id, |id| build_voice_out(id, payload));
}

fn relay_mic_state(room_id: RoomId, sender_conn_id: u64, payload: &[u8]) {
    fanout(room_id, sender_conn_id, |id| build_mic_out(id, payload));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(room: RoomId, name: &str) -> (u64, u16, mpsc::UnboundedReceiver<Vec<u8>>) {
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let conn = next_conn_id();
        let id = register_member(room, conn, name, tx);
        (conn, id, rx)
    }

    fn drain(rx: &mut mpsc::UnboundedReceiver<Vec<u8>>) -> Vec<Vec<u8>> {
        let mut v = Vec::new();
        while let Ok(f) = rx.try_recv() {
            v.push(f);
        }
        v
    }

    #[test]
    fn test_frame_layout() {
        let payload = vec![SUB_VOICE_DATA, 1, 2, 3, 4];
        let frame = build_frame(&payload);
        assert_eq!(&frame[0..2], &[0xAA, 0x55]);
        assert_eq!(u16::from_le_bytes([frame[2], frame[3]]), 5);
        assert_eq!(&frame[4..9], payload.as_slice());
        assert_eq!(&frame[9..], &[0x55, 0xAA]);
    }

    #[test]
    fn test_parse_login() {
        // [01][len 4]["Test"][u32 lo][u32 hi]
        let mut p = vec![SUB_LOGIN, 4];
        p.extend_from_slice(b"Test");
        p.extend_from_slice(&0x5566_7788u32.to_le_bytes());
        p.extend_from_slice(&0x1122_3344u32.to_le_bytes());
        assert_eq!(
            parse_login(&p),
            Some(("Test".to_string(), 0x1122_3344_5566_7788))
        );
        assert_eq!(parse_login(&p[..p.len() - 1]), None);
    }

    #[test]
    fn test_parse_join_matches_client_layout() {
        // Client: Dst=0x0204 (bytes 04 02), u16 room, u64 token, u16 256 -> 14 bytes.
        let mut p = vec![0x04, 0x02];
        p.extend_from_slice(&77u16.to_le_bytes());
        p.extend_from_slice(&0xAABB_CCDD_EEFF_0011u64.to_le_bytes());
        p.extend_from_slice(&256u16.to_le_bytes());
        assert_eq!(p.len(), 14);
        assert_eq!(parse_join(&p), Some((77, 0xAABB_CCDD_EEFF_0011)));
    }

    #[test]
    fn test_login_ack_layout() {
        assert_eq!(build_login_ack(Some(3)), vec![0x01, 1, 0, 3, 0]);
        assert_eq!(&build_login_ack(None)[..3], &[0x01, 0, 0]);
    }

    #[test]
    fn test_member_joined_and_left_layout() {
        assert_eq!(
            build_member_joined(2, "Ab"),
            vec![0x04, 0x02, 1, 2, 0, 2, b'A', b'b']
        );
        assert_eq!(
            build_member_left(2, "Ab"),
            vec![0x04, 0x03, 1, 0, 2, 0, 2, b'A', b'b']
        );
    }

    #[test]
    fn test_voice_out_inserts_sender_id() {
        let mut p = vec![SUB_VOICE_DATA];
        p.extend_from_slice(&3u16.to_le_bytes());
        p.extend_from_slice(&[9, 8, 7]);
        let out = build_voice_out(5, &p).unwrap();
        assert_eq!(out, vec![0x05, 5, 0, 3, 0, 9, 8, 7]);
        // Truncated payload: length clamps to what is present.
        assert_eq!(
            build_voice_out(5, &[0x05, 9, 0, 1, 2]).unwrap(),
            vec![0x05, 5, 0, 2, 0, 1, 2]
        );
    }

    #[test]
    fn test_mic_out_inserts_sender_id() {
        assert_eq!(
            build_mic_out(4, &[0x06, 1, 0, 2, 0]).unwrap(),
            vec![0x06, 4, 0, 1, 0, 2, 0]
        );
        assert_eq!(build_mic_out(4, &[0x06, 1]), None);
    }

    #[tokio::test]
    async fn test_join_announces_and_voice_reaches_only_joined_others() {
        let room: RoomId = 9001;
        let (ca, ia, mut rx_a) = member(room, "Alice");
        let (cb, ib, mut rx_b) = member(room, "Bob");
        assert_ne!(ia, ib);

        // Alice joins alone: sees herself.
        announce_join(room, ca);
        assert_eq!(
            drain(&mut rx_a),
            vec![build_frame(&build_member_joined(ia, "Alice"))]
        );

        // Bob has not joined yet: Alice's voice must not reach him.
        relay_voice(room, ca, &[0x05, 1, 0, 0xAA]);
        assert!(drain(&mut rx_b).is_empty());

        // Bob joins: he gets himself + Alice, Alice gets Bob.
        announce_join(room, cb);
        let b_frames = drain(&mut rx_b);
        assert!(b_frames.contains(&build_frame(&build_member_joined(ib, "Bob"))));
        assert!(b_frames.contains(&build_frame(&build_member_joined(ia, "Alice"))));
        assert_eq!(
            drain(&mut rx_a),
            vec![build_frame(&build_member_joined(ib, "Bob"))]
        );

        // Voice: Bob hears Alice (with her id), Alice does not hear herself.
        relay_voice(room, ca, &[0x05, 1, 0, 0xAA]);
        assert!(drain(&mut rx_a).is_empty());
        assert_eq!(
            drain(&mut rx_b),
            vec![build_frame(&[0x05, ia as u8, 0, 1, 0, 0xAA])]
        );

        // Leaving notifies the remaining member and empties the room.
        leave_room(room, ca);
        assert_eq!(
            drain(&mut rx_b),
            vec![build_frame(&build_member_left(ia, "Alice"))]
        );
        leave_room(room, cb);
        assert!(!rooms().lock().contains_key(&room));
    }

    #[tokio::test]
    async fn test_reconnect_evicts_same_name() {
        let room: RoomId = 9002;
        let (ca, _ia, _rx_a) = member(room, "Alice");
        announce_join(room, ca);
        let (cb, _ib, _rx_b) = member(room, "Alice");
        let rooms_guard = rooms().lock();
        let members = rooms_guard.get(&room).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].conn_id, cb);
        drop(rooms_guard);
        leave_room(room, cb);
    }

    #[tokio::test]
    async fn test_relay_to_unknown_room_is_a_no_op() {
        relay_voice(9999, 1, &[SUB_VOICE_DATA, 0, 0]);
    }

    #[test]
    fn test_header_and_tail_constants() {
        assert_eq!(HEADER, 0xAA55);
        assert_eq!(TAIL, 0x55AA);
    }
}
