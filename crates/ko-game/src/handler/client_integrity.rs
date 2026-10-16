//! Client integrity handshake (hook `WIZ_EXT_HOOK` 0xE9 / sub `INTEGRITY` 0xF4).
//!
//! Only the real `d3d9.dll` proxy knows the shared key below, so a repacked or home-made
//! client can't answer the challenge. The DLL also watches for modules loaded into the
//! process after startup (injected cheats/overlays) and reports that in the same reply.
//!
//! Flow, right after the player enters the game (end of `gamestart::handle`):
//! - S2C `[1][u8 nonce_len][nonce bytes]` — a fresh random nonce, one per login.
//! - C2S `[2][32 bytes HMAC-SHA256(KEY, nonce)][u8 tamper (1 = unknown module seen)]`
//!
//! A wrong/missing MAC, a tamper flag, or no reply within [`HANDSHAKE_TIMEOUT_SECS`]
//! disconnects the session. [`tick`] (called once a second from the event loop) kicks
//! sessions that never answered.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use hmac::{Hmac, Mac};
use ko_protocol::{Opcode, Packet, PacketReader};
use rand::RngCore;
use sha2::Sha256;
use tracing::warn;

use crate::session::ClientSession;
use crate::world::WorldState;
use crate::zone::SessionId;

pub const EXT_SUB_INTEGRITY: u8 = 0xF4;

/// Shared secret baked into both the server and the DLL (`client_integrity.cpp`). Rotate this
/// (and the client's copy) in a release if it ever leaks — it's the only thing an unofficial
/// client would need to fake the handshake.
const KEY: &[u8] = b"JstKO-2625-client-integrity-v1-7f3c9a1e";

const HANDSHAKE_TIMEOUT_SECS: u64 = 15;

/// Pending challenges: session -> (nonce sent, deadline).
static PENDING: LazyLock<DashMap<SessionId, (Vec<u8>, Instant)>> = LazyLock::new(DashMap::new);

/// Sends the challenge; called once, right after game entry.
pub async fn send_challenge(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce);
    PENDING.insert(
        sid,
        (nonce.to_vec(), Instant::now() + Duration::from_secs(HANDSHAKE_TIMEOUT_SECS)),
    );
    let mut pkt = Packet::new(Opcode::WizExtHook as u8);
    pkt.write_u8(EXT_SUB_INTEGRITY);
    pkt.write_u8(1);
    pkt.write_u8(nonce.len() as u8);
    pkt.write_bytes(&nonce);
    session.send_packet(&pkt).await
}

/// C2S handler. Returning `Err` here ends the session (see `session::run`'s dispatch loop).
/// `pkt.data` already has the sub-opcode byte stripped (see `repack_ext_data`).
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let sid = session.session_id();
    let mut r = PacketReader::new(&pkt.data);
    let op = r.read_u8().unwrap_or(0);
    if op == 3 {
        // Async alert: module_guard.cpp saw a module get injected after startup, any time during play.
        warn!(
            "[integrity] sid={} reports an unknown module loaded into the game process — disconnecting",
            sid
        );
        anyhow::bail!("client integrity: unknown module loaded");
    }
    if op != 2 {
        return Ok(());
    }
    let Some((_, (nonce, _))) = PENDING.remove(&sid) else {
        // Already handled (verified, timed out, or never challenged) — ignore.
        return Ok(());
    };
    let rest = r.read_remaining();
    if rest.len() < 33 {
        anyhow::bail!("client integrity: malformed reply");
    }
    let mac = &rest[..32];
    let tamper = rest[32] != 0;

    let mut expected = Hmac::<Sha256>::new_from_slice(KEY).expect("HMAC accepts any key length");
    expected.update(&nonce);
    let ok = expected.verify_slice(mac).is_ok();

    if tamper {
        warn!(
            "[integrity] sid={} reports an unknown module loaded into the game process — disconnecting",
            sid
        );
        anyhow::bail!("client integrity: unknown module loaded");
    }
    if !ok {
        warn!(
            "[integrity] sid={} failed the client handshake (wrong/missing key) — disconnecting",
            sid
        );
        anyhow::bail!("client integrity: handshake mismatch");
    }
    Ok(())
}

/// Called once a second from the event loop: kicks sessions that never answered the challenge.
pub async fn tick(world: &WorldState) {
    let now = Instant::now();
    let expired: Vec<SessionId> = PENDING
        .iter()
        .filter(|e| now > e.value().1)
        .map(|e| *e.key())
        .collect();
    for sid in expired {
        PENDING.remove(&sid);
        warn!("[integrity] sid={} never answered the client handshake — kicking", sid);
        world.kick_session(sid, "Client verification failed.").await;
    }
}
