//! Voice-relay TCP server — the second connection described in
//! `docs/reverse-engineering/2625/VOICE_CHAT_PROTOCOL.md`. Separate from the
//! main game server's listener (`crate::server::GameServer`): this is a raw,
//! unencrypted socket that only relays opaque voice/control bytes between the
//! members of a room, keyed by the room id (== party id) that the game-socket
//! handler (`crate::handler::voice_chat`) already assigned when it issued the
//! connecting client's token.
//!
//! Framing (both directions): `AA55 (u16 BE) | len (u16 LE) | payload (len
//! bytes) | 55AA (u16 BE)` — the same CAPISocket-style framing the launcher
//! patch TCP server already speaks (see `crates/ko-admin/src/patch_tcp.rs`).
//! `payload[0]` is the sub-opcode:
//! - `0x01` login: `[u64 token]` — the token issued over the game socket.
//! - `0x04` room create/join: accepted and ignored. Redundant with the
//!   game-socket side — a connection's room is already fixed by its token.
//! - `0x05` voice data: opaque Opus frame records, relayed byte-for-byte.
//! - `0x06` mic/speaker state: opaque, relayed byte-for-byte so peers' UIs
//!   can show who's muted/talking.
//! - `0x07` keepalive: liveness only, never relayed.
//!
//! The exact login/room payload shapes were not confirmed against a live
//! client capture in this pass (see the doc's "Open items") — the `0x01`
//! body is assumed to carry the same 64-bit token
//! `crate::handler::voice_chat` already hands out on the game socket.

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
/// Generous cap for one relay frame (a batch of 10ms Opus records); just a
/// sanity bound against a malformed/hostile length prefix.
const MAX_FRAME: usize = 65_536;

const SUB_LOGIN: u8 = 0x01;
const SUB_ROOM: u8 = 0x04;
const SUB_VOICE_DATA: u8 = 0x05;
const SUB_MIC_STATE: u8 = 0x06;
const SUB_KEEPALIVE: u8 = 0x07;

type RoomId = u16;

/// One logged-in relay connection, as seen by the rest of its room.
struct Member {
    conn_id: u64,
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

/// Rooms currently open on the relay, keyed by room id (== party id).
/// Deliberately simple — a shared map of member lists, matching the scope of
/// a first real implementation rather than a general pub/sub system.
fn rooms() -> &'static Mutex<HashMap<RoomId, Vec<Member>>> {
    static S: OnceLock<Mutex<HashMap<RoomId, Vec<Member>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_conn_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Start the voice relay listener as a background task. A bind failure is
/// logged, not fatal to the game server as a whole — voice chat simply stays
/// unavailable, matching how the launcher patch TCP server degrades.
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
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream).await {
                    debug!(%peer, error = %e, "voice relay connection ended");
                }
            });
        }
    })
}

async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let (mut read_half, write_half) = stream.into_split();
    let conn_id = next_conn_id();
    let mut joined_room: Option<RoomId> = None;

    // Relayed bytes from other room members are queued here and written by a
    // dedicated task, so one slow peer can't stall our read loop.
    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let writer = tokio::spawn(write_loop(write_half, rx));

    let result = read_loop(&mut read_half, conn_id, &mut joined_room, &tx).await;

    if let Some(room_id) = joined_room {
        leave_room(room_id, conn_id);
    }
    drop(tx);
    writer.abort();
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
    joined_room: &mut Option<RoomId>,
    tx: &mpsc::UnboundedSender<Vec<u8>>,
) -> anyhow::Result<()> {
    loop {
        let Some(payload) = read_frame(read_half).await? else {
            return Ok(());
        };
        if payload.is_empty() {
            continue;
        }
        let sub_opcode = payload[0];
        match sub_opcode {
            SUB_LOGIN => {
                let Some(token) = payload
                    .get(1..9)
                    .and_then(|b| <[u8; 8]>::try_from(b).ok())
                    .map(u64::from_le_bytes)
                else {
                    debug!(conn_id, "voice relay: malformed login frame, closing");
                    return Ok(());
                };
                match voice_chat::validate_token(token) {
                    Some(room_id) => {
                        join_room(room_id, conn_id, tx.clone());
                        *joined_room = Some(room_id);
                        debug!(conn_id, room_id, "voice relay: connection logged in");
                    }
                    None => {
                        debug!(conn_id, "voice relay: invalid token, closing connection");
                        return Ok(());
                    }
                }
            }
            SUB_ROOM => {
                debug!(
                    conn_id,
                    "voice relay: room create/join (already fixed by login token), ignoring"
                );
            }
            SUB_VOICE_DATA | SUB_MIC_STATE => {
                let Some(room_id) = *joined_room else {
                    debug!(conn_id, "voice relay: data before login, ignoring");
                    continue;
                };
                relay_to_room(room_id, conn_id, &payload);
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

fn join_room(room_id: RoomId, conn_id: u64, tx: mpsc::UnboundedSender<Vec<u8>>) {
    rooms()
        .lock()
        .entry(room_id)
        .or_default()
        .push(Member { conn_id, tx });
}

fn leave_room(room_id: RoomId, conn_id: u64) {
    let mut rooms = rooms().lock();
    if let Some(members) = rooms.get_mut(&room_id) {
        members.retain(|m| m.conn_id != conn_id);
        if members.is_empty() {
            rooms.remove(&room_id);
        }
    }
}

/// Forward `payload` (already including its sub-opcode byte) to every OTHER
/// connection in `room_id`, re-wrapped in the same AA55/55AA framing. A
/// connection never receives its own audio/state back.
fn relay_to_room(room_id: RoomId, sender_conn_id: u64, payload: &[u8]) {
    let rooms = rooms().lock();
    let Some(members) = rooms.get(&room_id) else {
        return;
    };
    if members.len() <= 1 {
        return;
    }
    let frame = build_frame(payload);
    for member in members {
        if member.conn_id == sender_conn_id {
            continue;
        }
        let _ = member.tx.send(frame.clone());
    }
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

/// Wrap `payload` in the `AA55 | len | payload | 55AA` frame for sending.
fn build_frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 6);
    out.extend_from_slice(&HEADER.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&TAIL.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_parse_frame_roundtrip() {
        let payload = vec![SUB_VOICE_DATA, 1, 2, 3, 4];
        let frame = build_frame(&payload);
        assert_eq!(&frame[0..2], &HEADER.to_be_bytes());
        assert_eq!(u16::from_le_bytes([frame[2], frame[3]]), payload.len() as u16);
        assert_eq!(&frame[4..4 + payload.len()], payload.as_slice());
        assert_eq!(&frame[frame.len() - 2..], &TAIL.to_be_bytes());
    }

    #[tokio::test]
    async fn test_relay_sends_to_others_not_self() {
        let room_id: RoomId = 9001;
        let (tx_a, mut rx_a) = mpsc::unbounded_channel::<Vec<u8>>();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel::<Vec<u8>>();
        let conn_a = next_conn_id();
        let conn_b = next_conn_id();
        join_room(room_id, conn_a, tx_a);
        join_room(room_id, conn_b, tx_b);

        let voice_payload = vec![SUB_VOICE_DATA, 0xAA, 0xBB];
        relay_to_room(room_id, conn_a, &voice_payload);

        // The sender (conn_a) must not receive its own audio back.
        assert!(rx_a.try_recv().is_err());

        // Every other member (conn_b) must receive it, correctly framed.
        let frame = rx_b.try_recv().expect("conn_b should receive the relay");
        assert_eq!(frame, build_frame(&voice_payload));

        leave_room(room_id, conn_a);
        leave_room(room_id, conn_b);
    }

    #[tokio::test]
    async fn test_relay_with_single_member_sends_nothing() {
        let room_id: RoomId = 9002;
        let (tx_a, mut rx_a) = mpsc::unbounded_channel::<Vec<u8>>();
        let conn_a = next_conn_id();
        join_room(room_id, conn_a, tx_a);

        relay_to_room(room_id, conn_a, &[SUB_VOICE_DATA, 1]);
        assert!(rx_a.try_recv().is_err());

        leave_room(room_id, conn_a);
    }

    #[tokio::test]
    async fn test_leave_room_removes_member_and_empties_room() {
        let room_id: RoomId = 9003;
        let (tx_a, _rx_a) = mpsc::unbounded_channel::<Vec<u8>>();
        let conn_a = next_conn_id();
        join_room(room_id, conn_a, tx_a);
        assert!(rooms().lock().contains_key(&room_id));

        leave_room(room_id, conn_a);
        assert!(!rooms().lock().contains_key(&room_id));
    }

    #[tokio::test]
    async fn test_relay_to_unknown_room_is_a_no_op() {
        // Must not panic when the room doesn't exist (e.g. everyone already left).
        relay_to_room(9999, 1, &[SUB_VOICE_DATA]);
    }

    #[test]
    fn test_sub_opcode_values_match_doc() {
        assert_eq!(SUB_LOGIN, 0x01);
        assert_eq!(SUB_ROOM, 0x04);
        assert_eq!(SUB_VOICE_DATA, 0x05);
        assert_eq!(SUB_MIC_STATE, 0x06);
        assert_eq!(SUB_KEEPALIVE, 0x07);
    }

    #[test]
    fn test_header_and_tail_constants() {
        assert_eq!(HEADER, 0xAA55);
        assert_eq!(TAIL, 0x55AA);
    }
}
