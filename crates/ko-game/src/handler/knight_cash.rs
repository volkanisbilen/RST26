//! Knight Cash (KC) balance system.
//! -  — `GiveBalance()`
//! -  — Load/Update methods
//! -  — `UpdateAccountKnightCash`
//! ## Overview
//! The KC system manages two in-memory balance counters per session:
//! - `knight_cash` — Knight Cash (KC) premium currency (DB: `tb_user.cash_point`)
//! - `tl_balance`  — TL real-money balance (DB: `tb_user.bonus_cash_point`)
//! The client is notified of balance changes via `WIZ_EXT_HOOK (0xE9)` (0xE9) with
//! sub-opcode `KCUPDATE = 0xB9`:
//! ```text
//! WIZ_EXT_HOOK (0xE9) << u8(0xB9) << u32(knight_cash) << u32(tl_balance)
//! ```
//! This opcode is guarded behind `#if(__VERSION < 2369)` in C++, so it applies
//! to the legacy client version this server targets.
//! ## PPCard / Prepaid Card (WIZ_EDIT_BOX sub-opcode 4)
//! PPCard redemption uses `WIZ_EDIT_BOX` (0x59) opcode and is handled
//! separately in `edit_box.rs`.  On success `GiveBalance()` in C++ calls
//! `UpdateAccountKnightCash` which:
//! 1. Adds cash/TL amounts to in-memory counters.
//! 2. Persists to DB via `UPDATE_BALANCE` stored procedure.
//! 3. Sends `KCUPDATE` packet to client.
//! ## WIZ_EXT_HOOK (0xE9) + KCUPDATE (0xB9) — client query / gift purchase
//! The client can send `WIZ_EXT_HOOK (0xE9)` with sub-opcode `0xB9` to query its
//! current balance (e.g. after a web-shop purchase, gift send, etc.).
//! Wire layout (client → server):
//! ```text
//! WIZ_EXT_HOOK (0xE9) << u8(0xB9)
//! ```
//! Server response (same opcode):
//! ```text
//! WIZ_EXT_HOOK (0xE9) << u8(0xB9) << u32(knight_cash) << u32(tl_balance)
//! ```

use ko_db::repositories::cash_shop::CashShopRepository;
use ko_protocol::{Opcode, Packet};
use tracing::{debug, warn};

use crate::session::ClientSession;

pub(crate) use super::ext_hook::{EXT_SUB_CASHCHANGE, EXT_SUB_KCUPDATE};

// ─────────────────────────────────────────────────────────────────────────────
// Packet Builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build a WIZ_CHAT PUBLIC_CHAT fallback for KC balance notification.
/// v2525 vanilla client drops both WIZ_EXT_HOOK (0xE9) and WIZ_ADD_MSG (0xDB)
/// — both are outside the GameMain dispatch range (0x06-0xD7). This chat
/// packet (WIZ_CHAT 0x12, type 7 PUBLIC_CHAT) is the only reliable way to
/// show KC balance to vanilla v2525 clients.
pub fn build_kc_chat_packet(knight_cash: u32, tl_balance: u32) -> Packet {
    let msg = format!("[KC] Balance: {} KC, {} TL", knight_cash, tl_balance);
    crate::systems::timed_notice::build_notice_packet(7, &msg)
}

/// Build the `KCUPDATE` packet sent to the client.
/// ```cpp
/// Packet newpkt(WIZ_EXT_HOOK (0xE9), uint8(ExtSub::KCUPDATE));
/// newpkt << m_nKnightCash << m_nTLBalance;
/// Send(&newpkt);
/// ```
/// Wire: `WIZ_EXT_HOOK (0xE9)(0xE9) << u8(0xB9) << u32(knight_cash) << u32(tl_balance)`
pub fn build_kcupdate_packet(knight_cash: u32, tl_balance: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_KCUPDATE);
    pkt.write_u32(knight_cash);
    pkt.write_u32(tl_balance);
    pkt
}

/// Build the `CASHCHANGE` packet — same wire format as `KCUPDATE`, different sub-opcode.
/// ```cpp
/// result << uint8(ExtSub::CASHCHANGE) << uint32(m_nKnightCash) << uint32(m_nTLBalance);
/// ```
/// Wire: `WIZ_EXT_HOOK (0xE9)(0xE9) << u8(0xA9) << u32(knight_cash) << u32(tl_balance)`
pub fn build_cashchange_packet(knight_cash: u32, tl_balance: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_CASHCHANGE);
    pkt.write_u32(knight_cash);
    pkt.write_u32(tl_balance);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Load from DB (called on game entry)
// ─────────────────────────────────────────────────────────────────────────────

/// Load KC and TL balances from DB and store in session state.
/// Called during `SelectCharID` (character selection → game entry).
/// On success, updates `session.world.knight_cash` and `tl_balance`.
/// On DB error, balances remain at 0 (safe — client won't see incorrect value).
pub async fn load_kc_balances(session: &mut ClientSession) {
    let account_id = match session.account_id() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return,
    };

    let pool = session.pool().clone();
    let repo = CashShopRepository::new(&pool);

    match repo.load_kc_balances(&account_id).await {
        Ok((kc, tl)) => {
            let kc_u32 = kc.max(0) as u32;
            let tl_u32 = tl.max(0) as u32;
            session
                .world()
                .set_kc_balance(session.session_id(), kc_u32, tl_u32);
            debug!(
                "[{}] KC balances loaded: KC={} TL={}",
                session.addr(),
                kc_u32,
                tl_u32
            );
        }
        Err(e) => {
            warn!(
                "[{}] Failed to load KC balances for {}: {}",
                session.addr(),
                account_id,
                e
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handle WIZ_EXT_HOOK (0xE9) KCUPDATE (0xB9) from client
// ─────────────────────────────────────────────────────────────────────────────

/// Handle `WIZ_EXT_HOOK (0xE9)` sub-opcode `KCUPDATE (0xB9)` from client.
/// The client sends this after opening the cash shop or after a web purchase
/// to request the current KC/TL balance.
/// Response: `WIZ_EXT_HOOK (0xE9) << u8(0xB9) << u32(kc) << u32(tl)`
pub async fn handle_kcupdate_query(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    let kc = world.get_knight_cash(sid);
    let tl = world.get_tl_balance(sid);

    let pkt = build_kcupdate_packet(kc, tl);
    session.send_packet(&pkt).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(kc, tl);
    session.send_packet(&chat_pkt).await?;

    debug!("[{}] KC query: KC={} TL={}", session.addr(), kc, tl);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// GiveBalance — add KC/TL and persist to DB
// ─────────────────────────────────────────────────────────────────────────────

/// Add KC and/or TL to a player's balance and persist to DB.
/// and `DatabaseThread.cpp:485-527` (`UpdateAccountKnightCash` handler).
/// - Updates in-memory balance on the session.
/// - Persists to DB via `update_kc_balances()`.
/// - Sends `KCUPDATE` packet to the client.
pub async fn give_balance(
    session: &mut ClientSession,
    cash_amount: i32,
    tl_amount: i32,
) -> anyhow::Result<()> {
    if cash_amount == 0 && tl_amount == 0 {
        return Ok(());
    }

    let account_id = match session.account_id() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Ok(()),
    };

    let sid = session.session_id();
    let world = session.world().clone();

    // Update in-memory balances
    let (new_kc, new_tl) = world
        .with_session(sid, |h| {
            let kc = (h.knight_cash as i64 + cash_amount as i64).clamp(0, u32::MAX as i64) as u32;
            let tl = (h.tl_balance as i64 + tl_amount as i64).clamp(0, u32::MAX as i64) as u32;
            (kc, tl)
        })
        .unwrap_or((0, 0));

    world.set_kc_balance(sid, new_kc, new_tl);

    // Persist to DB
    let pool = session.pool().clone();
    let acct = account_id.clone();
    tokio::spawn(async move {
        let repo = CashShopRepository::new(&pool);
        if let Err(e) = repo
            .update_kc_balances(&acct, new_kc as i32, new_tl as i32)
            .await
        {
            warn!("Failed to persist KC balances for {}: {}", acct, e);
        }
    });

    // esn redeems write their own ledger row (with the code)
    if !super::pus_codes::in_esn() {
        super::pus_codes::ledger(session.pool(), &account_id, &world.get_session_name(sid).unwrap_or_default(), cash_amount as i64, tl_amount as i64, new_kc, new_tl, "give", "");
    }
    // Send KCUPDATE packet to client
    let pkt = build_kcupdate_packet(new_kc, new_tl);
    session.send_packet(&pkt).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(new_kc, new_tl);
    session.send_packet(&chat_pkt).await?;

    // FerihaLog: PusShoppingInsertLog (refund/gift)
    {
        let zone_id = world
            .get_position(sid)
            .map(|p| p.zone_id as i16)
            .unwrap_or(0);
        super::audit_log::log_pus_shopping(
            session.pool(),
            &account_id,
            &world.get_session_name(sid).unwrap_or_default(),
            &session.addr().to_string(),
            zone_id,
            0,
            0,
            (cash_amount + tl_amount) as u32,
        );
    }

    debug!(
        "[{}] GiveBalance: +KC={} +TL={} → KC={} TL={}",
        session.addr(),
        cash_amount,
        tl_amount,
        new_kc,
        new_tl
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Deduct KC for a purchase
// ─────────────────────────────────────────────────────────────────────────────

/// Deduct KC from a player's balance after a successful purchase.
/// followed by `UpdateKnightCash` DB call and `CASHCHANGE` packet send.
/// Returns `true` if sufficient balance and deduction succeeded.
/// Returns `false` if insufficient KC balance (purchase should be rejected).
pub async fn deduct_kc(session: &mut ClientSession, amount: i32) -> anyhow::Result<bool> {
    if amount <= 0 {
        return Ok(true);
    }

    let sid = session.session_id();
    let world = session.world().clone();

    // Check balance
    let current_kc = world.get_knight_cash(sid) as i32;
    if current_kc < amount {
        debug!(
            "[{}] KC deduct failed: have={} need={}",
            session.addr(),
            current_kc,
            amount
        );
        return Ok(false);
    }

    let new_kc = (current_kc - amount).max(0) as u32;
    let new_tl = world.get_tl_balance(sid);

    world.set_kc_balance(sid, new_kc, new_tl);

    // Persist to DB
    let pool = session.pool().clone();
    let account_id = session.account_id().unwrap_or("").to_string();
    let acct = account_id.clone();
    tokio::spawn(async move {
        if acct.is_empty() {
            return;
        }
        let repo = CashShopRepository::new(&pool);
        if let Err(e) = repo
            .update_kc_balances(&acct, new_kc as i32, new_tl as i32)
            .await
        {
            warn!("Failed to persist KC deduction for {}: {}", acct, e);
        }
    });

    super::pus_codes::ledger(session.pool(), &account_id, &world.get_session_name(sid).unwrap_or_default(), -(amount as i64), 0, new_kc, new_tl, "buy_kc", "");
    // Send CASHCHANGE packet to client
    let pkt = build_cashchange_packet(new_kc, new_tl);
    session.send_packet(&pkt).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(new_kc, new_tl);
    session.send_packet(&chat_pkt).await?;

    // FerihaLog: PusShoppingInsertLog (purchase)
    {
        let zone_id = world
            .get_position(sid)
            .map(|p| p.zone_id as i16)
            .unwrap_or(0);
        super::audit_log::log_pus_shopping(
            session.pool(),
            &account_id,
            &world.get_session_name(sid).unwrap_or_default(),
            &session.addr().to_string(),
            zone_id,
            0,
            0,
            amount as u32,
        );
    }

    debug!(
        "[{}] KC deducted: -{} → KC={} TL={}",
        session.addr(),
        amount,
        new_kc,
        new_tl
    );
    Ok(true)
}

/// Deduct TL from a player's balance after a successful TL-priced purchase.
/// Returns `true` if sufficient balance and deduction succeeded.
/// Returns `false` if insufficient TL balance.
pub async fn deduct_tl(session: &mut ClientSession, amount: i32) -> anyhow::Result<bool> {
    if amount <= 0 {
        return Ok(true);
    }

    let sid = session.session_id();
    let world = session.world().clone();

    let current_tl = world.get_tl_balance(sid) as i32;
    if current_tl < amount {
        debug!(
            "[{}] TL deduct failed: have={} need={}",
            session.addr(),
            current_tl,
            amount
        );
        return Ok(false);
    }

    let new_kc = world.get_knight_cash(sid);
    let new_tl = (current_tl - amount).max(0) as u32;

    world.set_kc_balance(sid, new_kc, new_tl);

    // Persist to DB
    let pool = session.pool().clone();
    let account_id = session.account_id().unwrap_or("").to_string();
    let acct = account_id.clone();
    tokio::spawn(async move {
        if acct.is_empty() {
            return;
        }
        let repo = CashShopRepository::new(&pool);
        if let Err(e) = repo
            .update_kc_balances(&acct, new_kc as i32, new_tl as i32)
            .await
        {
            warn!("Failed to persist TL deduction for {}: {}", acct, e);
        }
    });

    super::pus_codes::ledger(session.pool(), &account_id, &world.get_session_name(sid).unwrap_or_default(), 0, -(amount as i64), new_kc, new_tl, "buy_tl", "");
    // Send CASHCHANGE packet to client
    let pkt = build_cashchange_packet(new_kc, new_tl);
    session.send_packet(&pkt).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(new_kc, new_tl);
    session.send_packet(&chat_pkt).await?;

    debug!(
        "[{}] TL deducted: -{} → KC={} TL={}",
        session.addr(),
        amount,
        new_kc,
        new_tl
    );
    Ok(true)
}

// ─────────────────────────────────────────────────────────────────────────────
// World-level CashLose / CashGain — for merchant & cross-session KC transfers
// ─────────────────────────────────────────────────────────────────────────────

/// Deduct KC from a player using WorldState (no `ClientSession` required).
/// - Checks balance, subtracts, sends `CASHCHANGE` packet, queues DB save.
/// Returns `false` if the player has insufficient KC.
pub fn cash_lose(
    world: &crate::world::WorldState,
    pool: &ko_db::DbPool,
    sid: crate::zone::SessionId,
    amount: u32,
) -> bool {
    if amount == 0 {
        return true;
    }
    let kc = world.get_knight_cash(sid);
    if kc < amount {
        return false;
    }
    let new_kc = kc - amount;
    let tl = world.get_tl_balance(sid);
    world.set_kc_balance(sid, new_kc, tl);

    // Send CASHCHANGE to client
    let pkt = build_cashchange_packet(new_kc, tl);
    world.send_to_session_owned(sid, pkt);
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(new_kc, tl);
    world.send_to_session_owned(sid, chat_pkt);

    // DB persist async
    let account_id = world
        .with_session(sid, |h| h.account_id.clone())
        .unwrap_or_default();
    if !account_id.is_empty() {
        let pool = pool.clone();
        tokio::spawn(async move {
            let repo = CashShopRepository::new(&pool);
            if let Err(e) = repo
                .update_kc_balances(&account_id, new_kc as i32, tl as i32)
                .await
            {
                warn!("Failed to persist KC loss for {}: {}", account_id, e);
            }
        });
    }
    true
}

/// Add KC to a player using WorldState (no `ClientSession` required).
/// - Adds KC, sends `CASHCHANGE` packet, queues DB save.
pub fn cash_gain(
    world: &crate::world::WorldState,
    pool: &ko_db::DbPool,
    sid: crate::zone::SessionId,
    amount: u32,
) {
    if amount == 0 {
        return;
    }
    let kc = world.get_knight_cash(sid);
    let new_kc = kc.saturating_add(amount);
    let tl = world.get_tl_balance(sid);
    world.set_kc_balance(sid, new_kc, tl);

    // Send CASHCHANGE to client
    let pkt = build_cashchange_packet(new_kc, tl);
    world.send_to_session_owned(sid, pkt);
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(new_kc, tl);
    world.send_to_session_owned(sid, chat_pkt);

    // DB persist async
    let account_id = world
        .with_session(sid, |h| h.account_id.clone())
        .unwrap_or_default();
    if !account_id.is_empty() {
        let pool = pool.clone();
        tokio::spawn(async move {
            let repo = CashShopRepository::new(&pool);
            if let Err(e) = repo
                .update_kc_balances(&account_id, new_kc as i32, tl as i32)
                .await
            {
                warn!("Failed to persist KC gain for {}: {}", account_id, e);
            }
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GM command helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Send a `KCUPDATE` packet for a given session without changing balances.
/// Used by GM `/showkc` style commands to display current KC balance.
pub fn send_kcupdate(world: &crate::world::WorldState, sid: crate::zone::SessionId) {
    let kc = world.get_knight_cash(sid);
    let tl = world.get_tl_balance(sid);
    let pkt = build_kcupdate_packet(kc, tl);
    world.send_to_session_owned(sid, pkt);
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = build_kc_chat_packet(kc, tl);
    world.send_to_session_owned(sid, chat_pkt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── Packet format tests ────────────────────────────────────────────────

    #[test]
    fn test_kc_chat_fallback_packet() {
        // WIZ_CHAT PUBLIC_CHAT fallback for vanilla v2525 client
        let pkt = build_kc_chat_packet(1500, 200);
        assert_eq!(pkt.opcode, Opcode::WizChat as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(7)); // PUBLIC_CHAT
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u32(), Some(0xFFFFFFFF)); // sender_id
        let _ = r.read_sbyte_string(); // empty name
        let msg = r.read_string();
        assert!(msg.unwrap().contains("1500 KC"));
    }

    #[test]
    fn test_kcupdate_packet_format() {
        // Packet newpkt(WIZ_EXT_HOOK (0xE9), uint8(ExtSub::KCUPDATE));
        // newpkt << m_nKnightCash << m_nTLBalance;
        let pkt = build_kcupdate_packet(1000, 250);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_KCUPDATE)); // sub-opcode 0xB9
        assert_eq!(r.read_u32(), Some(1000)); // knight_cash
        assert_eq!(r.read_u32(), Some(250)); // tl_balance
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_cashchange_packet_format() {
        // result << uint8(ExtSub::CASHCHANGE) << uint32(m_nKnightCash) << uint32(m_nTLBalance);
        let pkt = build_cashchange_packet(500, 100);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASHCHANGE)); // sub-opcode 0xA9
        assert_eq!(r.read_u32(), Some(500)); // knight_cash
        assert_eq!(r.read_u32(), Some(100)); // tl_balance
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_kcupdate_zero_balances() {
        let pkt = build_kcupdate_packet(0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_KCUPDATE));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_kcupdate_max_values() {
        // u32::MAX for both fields
        let pkt = build_kcupdate_packet(u32::MAX, u32::MAX);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_KCUPDATE));
        assert_eq!(r.read_u32(), Some(u32::MAX));
        assert_eq!(r.read_u32(), Some(u32::MAX));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_kcupdate_packet_size() {
        // sub-opcode (1) + kc (4) + tl (4) = 9 bytes of data
        let pkt = build_kcupdate_packet(1234, 5678);
        assert_eq!(pkt.data.len(), 9);
    }

    #[test]
    fn test_cashchange_packet_size() {
        let pkt = build_cashchange_packet(1234, 5678);
        assert_eq!(pkt.data.len(), 9);
    }

    #[test]
    fn test_ext_sub_kcupdate_constant() {
        assert_eq!(EXT_SUB_KCUPDATE, 0xB9);
    }

    #[test]
    fn test_ext_sub_cashchange_constant() {
        assert_eq!(EXT_SUB_CASHCHANGE, 0xA9);
    }

    // ── WorldState KC balance accessor tests ──────────────────────────────

    #[test]
    fn test_world_kc_balance_default_zero() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        assert_eq!(world.get_knight_cash(1), 0);
        assert_eq!(world.get_tl_balance(1), 0);
    }

    #[test]
    fn test_world_set_kc_balance() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_kc_balance(1, 3000, 125);
        assert_eq!(world.get_knight_cash(1), 3000);
        assert_eq!(world.get_tl_balance(1), 125);
    }

    #[test]
    fn test_world_kc_nonexistent_session() {
        use crate::world::WorldState;

        let world = WorldState::new();
        // Session 99 doesn't exist — should return 0 safely
        assert_eq!(world.get_knight_cash(99), 0);
        assert_eq!(world.get_tl_balance(99), 0);
    }

    #[test]
    fn test_world_kc_update_replaces_old_value() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        world.set_kc_balance(1, 500, 50);
        assert_eq!(world.get_knight_cash(1), 500);

        // Update again
        world.set_kc_balance(1, 1000, 200);
        assert_eq!(world.get_knight_cash(1), 1000);
        assert_eq!(world.get_tl_balance(1), 200);
    }

    // ── Give balance / deduct logic tests (pure math) ─────────────────────

    #[test]
    fn test_give_balance_math_kc_only() {
        // Simulate: 500 KC + give 200 KC = 700 KC
        let kc: i64 = 500;
        let tl: i64 = 0;
        let delta_kc: i32 = 200;
        let delta_tl: i32 = 0;

        let new_kc = (kc + delta_kc as i64).clamp(0, u32::MAX as i64) as u32;
        let new_tl = (tl + delta_tl as i64).clamp(0, u32::MAX as i64) as u32;

        assert_eq!(new_kc, 700);
        assert_eq!(new_tl, 0);
    }

    #[test]
    fn test_give_balance_math_tl_only() {
        let kc: i64 = 100;
        let tl: i64 = 50;
        let delta_kc: i32 = 0;
        let delta_tl: i32 = 100;

        let new_kc = (kc + delta_kc as i64).clamp(0, u32::MAX as i64) as u32;
        let new_tl = (tl + delta_tl as i64).clamp(0, u32::MAX as i64) as u32;

        assert_eq!(new_kc, 100);
        assert_eq!(new_tl, 150);
    }

    #[test]
    fn test_give_balance_math_both() {
        let kc: i64 = 300;
        let tl: i64 = 25;
        let delta_kc: i32 = 500;
        let delta_tl: i32 = 50;

        let new_kc = (kc + delta_kc as i64).clamp(0, u32::MAX as i64) as u32;
        let new_tl = (tl + delta_tl as i64).clamp(0, u32::MAX as i64) as u32;

        assert_eq!(new_kc, 800);
        assert_eq!(new_tl, 75);
    }

    #[test]
    fn test_give_balance_math_negative_clamped() {
        // C++ allows subtracting but we clamp at 0
        let kc: i64 = 100;
        let tl: i64 = 10;
        let delta_kc: i32 = -200; // overshoot
        let delta_tl: i32 = -20; // overshoot

        let new_kc = (kc + delta_kc as i64).clamp(0, u32::MAX as i64) as u32;
        let new_tl = (tl + delta_tl as i64).clamp(0, u32::MAX as i64) as u32;

        assert_eq!(new_kc, 0); // clamped
        assert_eq!(new_tl, 0); // clamped
    }

    #[test]
    fn test_deduct_kc_sufficient_balance() {
        // Player has 1000 KC, deduct 499
        let current_kc = 1000i32;
        let amount = 499i32;
        assert!(current_kc >= amount);
        let new_kc = (current_kc - amount).max(0) as u32;
        assert_eq!(new_kc, 501);
    }

    #[test]
    fn test_deduct_kc_exact_balance() {
        let current_kc = 499i32;
        let amount = 499i32;
        assert!(current_kc >= amount);
        let new_kc = (current_kc - amount).max(0) as u32;
        assert_eq!(new_kc, 0);
    }

    #[test]
    fn test_deduct_kc_insufficient_balance() {
        let current_kc = 100i32;
        let amount = 499i32;
        assert!(current_kc < amount); // should be rejected
    }

    #[test]
    fn test_deduct_tl_sufficient_balance() {
        let current_tl = 125i32;
        let amount = 100i32;
        assert!(current_tl >= amount);
        let new_tl = (current_tl - amount).max(0) as u32;
        assert_eq!(new_tl, 25);
    }

    #[test]
    fn test_deduct_tl_insufficient_balance() {
        let current_tl = 50i32;
        let amount = 100i32;
        assert!(current_tl < amount); // should be rejected
    }

    // ── Packet little-endian tests ─────────────────────────────────────────

    #[test]
    fn test_kcupdate_little_endian_encoding() {
        // KC=0x00000064 (100), TL=0x0000000A (10) in little-endian
        let pkt = build_kcupdate_packet(100, 10);
        // bytes 1-4 should be [100,0,0,0] (LE u32)
        assert_eq!(pkt.data[1], 100);
        assert_eq!(pkt.data[2], 0);
        assert_eq!(pkt.data[3], 0);
        assert_eq!(pkt.data[4], 0);
        // bytes 5-8 should be [10,0,0,0] (LE u32)
        assert_eq!(pkt.data[5], 10);
        assert_eq!(pkt.data[6], 0);
        assert_eq!(pkt.data[7], 0);
        assert_eq!(pkt.data[8], 0);
    }

    #[test]
    fn test_kcupdate_large_value_little_endian() {
        // KC=5000 = 0x00001388, TL=200 = 0xC8
        let pkt = build_kcupdate_packet(5000, 200);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0xB9));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.read_u32(), Some(200));
    }

    // ── send_kcupdate world helper test ────────────────────────────────────

    #[test]
    fn test_send_kcupdate_from_world_state() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 2500, 75);

        send_kcupdate(&world, 1);

        // Verify the packet was sent
        let pkt = rx.try_recv().expect("packet should be queued");
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_KCUPDATE));
        assert_eq!(r.read_u32(), Some(2500));
        assert_eq!(r.read_u32(), Some(75));
    }

    #[test]
    fn test_send_kcupdate_chat_fallback() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 2500, 75);

        send_kcupdate(&world, 1);

        // First packet: ext_hook KCUPDATE
        let pkt1 = rx.try_recv().expect("ext_hook packet");
        assert_eq!(pkt1.opcode, Opcode::EXT_HOOK_S2C);
        // Second packet: WIZ_CHAT PUBLIC_CHAT fallback
        let pkt2 = rx.try_recv().expect("chat fallback packet");
        assert_eq!(pkt2.opcode, Opcode::WizChat as u8);
        let mut r = PacketReader::new(&pkt2.data);
        assert_eq!(r.read_u8(), Some(7)); // PUBLIC_CHAT
    }

    // ── cash_lose / cash_gain world-level helper tests ──────────────────

    #[tokio::test]
    async fn test_cash_lose_sufficient_balance() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 1000, 50);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        let result = cash_lose(&world, &pool, 1, 300);
        assert!(result);
        assert_eq!(world.get_knight_cash(1), 700);
        assert_eq!(world.get_tl_balance(1), 50); // TL unchanged

        // Verify CASHCHANGE packet sent
        let pkt = rx.try_recv().expect("CASHCHANGE packet should be sent");
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASHCHANGE));
        assert_eq!(r.read_u32(), Some(700));
        assert_eq!(r.read_u32(), Some(50));
    }

    #[tokio::test]
    async fn test_cash_lose_insufficient_balance() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 100, 0);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        let result = cash_lose(&world, &pool, 1, 200);
        assert!(!result);
        assert_eq!(world.get_knight_cash(1), 100); // unchanged
    }

    #[tokio::test]
    async fn test_cash_lose_exact_balance() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 500, 25);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        let result = cash_lose(&world, &pool, 1, 500);
        assert!(result);
        assert_eq!(world.get_knight_cash(1), 0);
        assert_eq!(world.get_tl_balance(1), 25);
    }

    #[tokio::test]
    async fn test_cash_lose_zero_amount() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 1000, 0);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        let result = cash_lose(&world, &pool, 1, 0);
        assert!(result);
        assert_eq!(world.get_knight_cash(1), 1000); // unchanged
    }

    #[tokio::test]
    async fn test_cash_gain_basic() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 500, 10);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        cash_gain(&world, &pool, 1, 300);
        assert_eq!(world.get_knight_cash(1), 800);
        assert_eq!(world.get_tl_balance(1), 10); // TL unchanged

        // Verify CASHCHANGE packet sent
        let pkt = rx.try_recv().expect("CASHCHANGE packet should be sent");
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASHCHANGE));
        assert_eq!(r.read_u32(), Some(800));
        assert_eq!(r.read_u32(), Some(10));
    }

    #[tokio::test]
    async fn test_cash_gain_zero_amount() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, 500, 0);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        cash_gain(&world, &pool, 1, 0);
        assert_eq!(world.get_knight_cash(1), 500); // unchanged

        // No packet should be sent for zero gain
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_cash_gain_saturating_add() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.set_kc_balance(1, u32::MAX - 10, 0);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        cash_gain(&world, &pool, 1, 100);
        assert_eq!(world.get_knight_cash(1), u32::MAX); // capped
    }

    #[tokio::test]
    async fn test_cash_lose_then_gain_roundtrip() {
        // Simulates a merchant KC transaction: buyer loses, seller gains
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1); // buyer
        world.register_session(2, tx2); // seller
        world.set_kc_balance(1, 1000, 0);
        world.set_kc_balance(2, 200, 0);

        let pool = ko_db::DbPool::connect_lazy("postgres://test").unwrap();
        let price = 350u32;

        // Buyer pays
        assert!(cash_lose(&world, &pool, 1, price));
        // Seller receives
        cash_gain(&world, &pool, 2, price);

        assert_eq!(world.get_knight_cash(1), 650); // 1000 - 350
        assert_eq!(world.get_knight_cash(2), 550); // 200 + 350
    }
}
