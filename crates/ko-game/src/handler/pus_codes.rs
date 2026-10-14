//! Power Up Store ESN codes (KC / TL e-pins) and the KC/TL ledger.
//!
//! Codes are generated in the admin panel (`pus_code`, `pus_code_batch`); a player types one into the Power
//! Up Store "Use ESN" box. Every attempt is logged (`pus_code_attempt`), every balance change goes to
//! `kc_ledger`, so the panel can show which account used which code.
//!
//! Client -> server: `[0xE9][0xA8][4][string code]`   Server -> client: `[0xE9][0xDC][u8 result][u32 kc][u32 tl]`
//! result: 0 ok, 1 invalid, 2 already used, 3 disabled / expired, 4 too many wrong tries, 5 busy

use ko_protocol::{Packet, PacketReader};
use tracing::warn;

use crate::session::ClientSession;
use ko_db::DbPool;

pub const EXT_SUB_PUS_ESN: u8 = 0xDC;

const MAX_FAILS: u32 = 5;
const FAIL_WINDOW_MIN: i32 = 10;

/// Append a ledger row (fire and forget).
pub fn ledger(pool: &DbPool, account: &str, char_name: &str, kc_delta: i64, tl_delta: i64, kc_after: u32, tl_after: u32, reason: &str, reference: &str) {
    if account.is_empty() || (kc_delta == 0 && tl_delta == 0) {
        return;
    }
    let pool = pool.clone();
    let (account, char_name, reason, reference) = (account.to_string(), char_name.to_string(), reason.to_string(), reference.to_string());
    tokio::spawn(async move {
        let r = sqlx::query(
            "INSERT INTO kc_ledger(account,char_name,kc_delta,tl_delta,kc_after,tl_after,reason,ref) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(account)
        .bind(char_name)
        .bind(kc_delta as i32)
        .bind(tl_delta as i32)
        .bind(kc_after as i64)
        .bind(tl_after as i64)
        .bind(reason)
        .bind(reference)
        .execute(&pool)
        .await;
        if let Err(e) = r {
            warn!("kc_ledger insert failed: {e}");
        }
    });
}

tokio::task_local! {
    static ESN: ();
}

/// True while an ESN redeem credits the balance (it writes its own ledger row).
pub fn in_esn() -> bool {
    ESN.try_with(|_| ()).is_ok()
}

fn normalize(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
}

/// Canonical stored form: `XXXX-XXXX-XXXX-XXXX`.
fn canonical(raw: &str) -> String {
    let n = normalize(raw);
    n.as_bytes().chunks(4).map(|c| String::from_utf8_lossy(c).to_string()).collect::<Vec<_>>().join("-")
}

fn reply(result: u8, kc: u32, tl: u32) -> Packet {
    let mut p = Packet::new(ko_protocol::Opcode::EXT_HOOK_S2C);
    p.write_u8(EXT_SUB_PUS_ESN);
    p.write_u8(result);
    p.write_u32(kc);
    p.write_u32(tl);
    p
}

async fn log_attempt(pool: &DbPool, account: &str, ch: &str, ip: &str, code: &str, result: &str) {
    let _ = sqlx::query("INSERT INTO pus_code_attempt(account,char_name,ip,code,result) VALUES($1,$2,$3,$4,$5)")
        .bind(account)
        .bind(ch)
        .bind(ip)
        .bind(code.chars().take(40).collect::<String>())
        .bind(result)
        .execute(pool)
        .await;
}

/// PUS op 4: redeem a code.
pub async fn handle_redeem(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    let raw = PacketReader::new(data).read_string().unwrap_or_default();
    let world = session.world().clone();
    let sid = session.session_id();
    let account = session.account_id().unwrap_or("").to_string();
    if account.is_empty() {
        return Ok(());
    }
    let ch = world.get_session_name(sid).unwrap_or_default();
    let ip = session.addr().ip().to_string();
    let pool = session.pool().clone();
    let (kc0, tl0) = world.with_session(sid, |h| (h.knight_cash, h.tl_balance)).unwrap_or((0, 0));

    let code = canonical(&raw);
    if normalize(&raw).len() < 8 || normalize(&raw).len() > 20 {
        log_attempt(&pool, &account, &ch, &ip, &raw, "invalid").await;
        session.send_packet(&reply(1, kc0, tl0)).await?;
        return Ok(());
    }

    // brute force guard: recent wrong tries of this account
    let fails: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pus_code_attempt WHERE account=$1 AND result IN ('invalid','used','disabled') AND at > NOW() - make_interval(mins => $2)",
    )
    .bind(&account)
    .bind(FAIL_WINDOW_MIN)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);
    if fails >= MAX_FAILS as i64 {
        log_attempt(&pool, &account, &ch, &ip, &raw, "blocked").await;
        session.send_packet(&reply(4, kc0, tl0)).await?;
        return Ok(());
    }

    // atomic claim: only one redeem can ever flip used_at
    let claimed: Option<(i32, i32)> = sqlx::query_as(
        "UPDATE pus_code SET used_at=NOW(), used_account=$2, used_char=$3, used_ip=$4 \
         WHERE code=$1 AND used_at IS NULL AND NOT disabled AND (expires_at IS NULL OR expires_at > NOW()) \
         RETURNING kc, tl",
    )
    .bind(&code)
    .bind(&account)
    .bind(&ch)
    .bind(&ip)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let (kc, tl) = match claimed {
        Some(v) => v,
        None => {
            let state: Option<(bool, bool, bool)> = sqlx::query_as(
                "SELECT used_at IS NOT NULL, disabled, (expires_at IS NOT NULL AND expires_at <= NOW()) FROM pus_code WHERE code=$1",
            )
            .bind(&code)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
            let (res, text) = match state {
                None => (1u8, "invalid"),
                Some((true, _, _)) => (2, "used"),
                Some(_) => (3, "disabled"),
            };
            log_attempt(&pool, &account, &ch, &ip, &raw, text).await;
            session.send_packet(&reply(res, kc0, tl0)).await?;
            return Ok(());
        }
    };

    ESN.scope((), crate::handler::knight_cash::give_balance(session, kc, tl)).await?;
    let (kc1, tl1) = world.with_session(sid, |h| (h.knight_cash, h.tl_balance)).unwrap_or((kc0, tl0));
    ledger(&pool, &account, &ch, kc as i64, tl as i64, kc1, tl1, "esn", &code);
    log_attempt(&pool, &account, &ch, &ip, &raw, "ok").await;
    warn!("ESN {code} redeemed by {account}/{ch}: +{kc} KC +{tl} TL");
    session.send_packet(&reply(0, kc1, tl1)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_form() {
        assert_eq!(canonical("abcd efgh-ijkl_mnop"), "ABCD-EFGH-IJKL-MNOP");
        assert_eq!(canonical("abc"), "ABC");
    }
}
