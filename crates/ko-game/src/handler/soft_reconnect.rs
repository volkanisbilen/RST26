//! Soft re-connect (client proxy `reconnect.cpp`) - WIZ_EXT_HOOK 0xE9, sub 0xEF.
//!
//! When a character enters the game the server hands out a one-shot token:
//!   S->C `[0x01][16 token][str account][str character]`
//! If the connection drops, the client reconnects, passes the version check and, instead of the
//! character select, sends
//!   C->S `[0x02][16 token][str account]`
//! The server validates and consumes the token, logs the account in, selects the stored
//! character (the normal WIZ_SEL_CHAR path) and, once the character is in the world, answers
//!   S->C `[0x03][u8 ok]`
//! A failed resume is answered with `[0x03][0]` so the client falls back to the hard re-connect.
//!
//! The token is not consumed by a resume attempt (a second drop in the middle of an attempt can retry
//! with it; it is replaced when the character is in the world again) and it is mirrored into the table
//! `soft_reconnect_token`, so a server restart does not force players into the hard (process restart)
//! re-connect.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
use ko_protocol::{Opcode, Packet, PacketReader};
use rand::RngCore;

use crate::session::{ClientSession, SessionState};
use crate::world::WorldState;
use crate::zone::SessionId;

pub(crate) const EXT_SUB_RECONNECT: u8 = 0xEF;
const TOKEN_TTL: Duration = Duration::from_secs(15 * 60);

struct Entry {
    account: String,
    character: String,
    expires: Instant,
}

fn tokens() -> &'static DashMap<[u8; 16], Entry> {
    static T: OnceLock<DashMap<[u8; 16], Entry>> = OnceLock::new();
    T.get_or_init(DashMap::new)
}

fn resumed() -> &'static DashSet<SessionId> {
    static R: OnceLock<DashSet<SessionId>> = OnceLock::new();
    R.get_or_init(DashSet::new)
}

fn base(op: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_RECONNECT);
    pkt.write_u8(op);
    pkt
}

fn ack(ok: bool) -> Packet {
    let mut pkt = base(3);
    pkt.write_u8(u8::from(ok));
    pkt
}

/// Called once the character is fully in the world: hand out a fresh token and, for a resumed
/// session, confirm the resume.
pub fn on_game_entered(world: &WorldState, sid: SessionId) {
    let Some(ch) = world.get_character_info(sid) else {
        return;
    };
    let account = world.with_session(sid, |h| h.account_id.clone()).unwrap_or_default();
    if account.is_empty() {
        return;
    }
    let now = Instant::now();
    tokens().retain(|_, e| e.expires > now);
    // one live token per account
    tokens().retain(|_, e| e.account != account);
    let mut token = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut token);
    tokens().insert(
        token,
        Entry { account: account.clone(), character: ch.name.clone(), expires: now + TOKEN_TTL },
    );
    if let Some(pool) = world.db_pool().cloned() {
        let (acc, chr) = (account.clone(), ch.name.clone());
        tokio::spawn(async move {
            let _ = sqlx::query("DELETE FROM soft_reconnect_token WHERE account = $1 OR expires_at < now()")
                .bind(&acc)
                .execute(&pool)
                .await;
            let _ = sqlx::query(
                "INSERT INTO soft_reconnect_token (token, account, character, expires_at) VALUES ($1, $2, $3, now() + interval '15 minutes')",
            )
            .bind(&token[..])
            .bind(&acc)
            .bind(&chr)
            .execute(&pool)
            .await;
        });
    }
    if resumed().remove(&sid).is_some() {
        world.send_to_session_owned(sid, ack(true));
    }
    let mut pkt = base(1);
    for b in token {
        pkt.write_u8(b);
    }
    pkt.write_string(&account);
    pkt.write_string(&ch.name);
    world.send_to_session_owned(sid, pkt);
}

/// C->S dispatcher; `pkt.data` has the 0xEF byte already stripped.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut r = PacketReader::new(&pkt.data);
    if r.read_u8() != Some(2) {
        return Ok(());
    }
    handle_resume(session, &mut r).await
}

async fn handle_resume(session: &mut ClientSession, r: &mut PacketReader<'_>) -> anyhow::Result<()> {
    if session.state() != SessionState::VersionChecked || session.account_id().is_some() {
        return Ok(());
    }
    let mut token = [0u8; 16];
    for b in token.iter_mut() {
        *b = r.read_u8().unwrap_or(0);
    }
    let account = r.read_string().unwrap_or_default();

    let world = session.world().clone();
    let mut entry = tokens()
        .get(&token)
        .map(|e| Entry { account: e.account.clone(), character: e.character.clone(), expires: e.expires })
        .filter(|e| e.expires > Instant::now() && e.account == account);
    if entry.is_none() {
        // not in memory (server restarted since the token was issued): the table has it
        if let Some(pool) = world.db_pool() {
            let row: Option<(String, String)> = sqlx::query_as(
                "SELECT account, character FROM soft_reconnect_token WHERE token = $1 AND expires_at > now()",
            )
            .bind(&token[..])
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
            entry = row
                .filter(|(a, _)| *a == account)
                .map(|(a, c)| Entry { account: a, character: c, expires: Instant::now() + TOKEN_TTL });
        }
    }
    let Some(entry) = entry else {
        tracing::info!("[{}] soft reconnect refused (unknown / expired token) for '{}'", session.addr(), account);
        session.send_packet(&ack(false)).await?;
        return Ok(());
    };

    let sid = session.session_id();

    // The old connection may still be registered: same handling as a duplicate login.
    if let Some(old_sid) = world.find_session_by_account(&entry.account) {
        if old_sid != sid {
            if world.is_offline_status(old_sid) {
                crate::systems::offline_merchant::cleanup_offline_session(&world, old_sid).await;
            } else {
                world.kick_session_for_duplicate(old_sid).await;
            }
        }
    }

    session.set_account_id(entry.account.clone());
    session.set_state(SessionState::LoggedIn);
    world.update_session(sid, |h| h.account_id = entry.account.clone());
    resumed().insert(sid);
    tracing::info!("[{}] soft reconnect: {} resumes {}", session.addr(), entry.account, entry.character);

    // continue as if the client had selected the character
    let mut sel = Packet::new(Opcode::WizSelChar as u8);
    sel.write_string(&entry.account);
    sel.write_string(&entry.character);
    sel.write_u8(1);
    super::selchar::handle(session, sel).await
}
