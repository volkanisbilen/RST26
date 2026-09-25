//! WIZ_REBIRTH (0xD3) handler — Rebirth system.
//!
//! v2525 client's rebirth (reincarnation) system. Allows players to reset
//! and re-level with bonuses.
//!
//! ## Protocol Reference
//!
//! `RebirthBas @ 0x140284b40`:
//! - Requires: GetExpPercent() >= 100, gold >= 100M, loyalty >= 10K
//! - On fail: SendHelpDescription with error messages
//! - On success: sends [0xE9][0xF2][0x01] (ext_hook) — v2525 uses native 0xD3
//!
//! ## Client RE
//!
//! - Handler: `0x70D920` — 4 sub-opcodes
//! - Panel: `[esi+0x69C]` — Group B (panel-dependent for some operations)
//! - Sound: `0x53021` on activation
//! - String IDs: `0xAECE` (activation), `0xAED2` (completion, yellow notice)
//!
//! ## S2C Packet Format
//!
//! ```text
//! [u8 sub] — sub-opcode:
//!   sub=1: Activation
//!     (no additional fields)
//!   sub=2: Result/Cancel
//!     [u8 result] — result code (clears active/init flags)
//!   sub=3: Progress Update
//!     [i32 level_offset] — subtracted from 0x708 for position calc
//!     [i32 current]      — progress bar current value
//!     [i32 max]          — progress bar maximum value
//!   sub=4: Completion
//!     [i32 rebirth_level] — new rebirth level (shown in yellow notice)
//! ```
//!
//! ## Rebirth Process
//!
//! 1. Player talks to Rebirth NPC → Lua `RebirthBas()` shows menu
//! 2. Client sends C2S [0xD3][sub=1] to request rebirth
//! 3. Server validates: EXP >= 100%, Gold >= 100M, Loyalty >= 10K
//! 4. On success: increment rebirth_level, snapshot stats, reset level to 1
//! 5. Send S2C [0xD3][sub=1] (activate) + [0xD3][sub=4] (complete)
//! 6. Persist to DB

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};

// ── Constants ────────────────────────────────────────────────────────

/// Minimum gold required for rebirth (100M coins).
const REBIRTH_GOLD_COST: u32 = 100_000_000;

/// Minimum loyalty (NP) required for rebirth (10K).
const REBIRTH_LOYALTY_COST: u32 = 10_000;

/// EXP percent threshold (100% = full level).
const REBIRTH_EXP_PERCENT: i32 = 100;

// ── Sub-opcode constants ──────────────────────────────────────────────

/// Activation — plays sound, shows "Rebirth activated".
pub const SUB_ACTIVATE: u8 = 1;

/// Cancel/result — clears rebirth state.
pub const SUB_RESULT: u8 = 2;

/// Progress — updates progress bar (level_offset, current, max).
pub const SUB_PROGRESS: u8 = 3;

/// Completion — shows "Rebirth Lv X" in yellow.
pub const SUB_COMPLETE: u8 = 4;

// ── S2C Builders ──────────────────────────────────────────────────────

/// Build a rebirth activation packet (sub=1).
///
/// Client plays sound `0x53021`, sets active flag, shows string `0xAECE`,
/// and updates the rebirth button state.
///
/// Wire: `[u8 sub=1]`
pub fn build_activate() -> Packet {
    let mut pkt = Packet::new(Opcode::WizRebirth as u8);
    pkt.write_u8(SUB_ACTIVATE);
    pkt
}

/// Build a rebirth result/cancel packet (sub=2).
///
/// Client clears active/init flags and reads result byte.
///
/// - `result`: Result code displayed to the player
///
/// Wire: `[u8 sub=2][u8 result]`
pub fn build_result(result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRebirth as u8);
    pkt.write_u8(SUB_RESULT);
    pkt.write_u8(result);
    pkt
}

/// Build a rebirth progress update packet (sub=3).
///
/// Client updates the progress bar display.
///
/// - `level_offset`: Subtracted from `0x708` for position calculation
/// - `current`: Progress bar current value
/// - `max`: Progress bar maximum value
///
/// Wire: `[u8 sub=3][i32 level_offset][i32 current][i32 max]`
pub fn build_progress(level_offset: i32, current: i32, max: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRebirth as u8);
    pkt.write_u8(SUB_PROGRESS);
    pkt.write_i32(level_offset);
    pkt.write_i32(current);
    pkt.write_i32(max);
    pkt
}

/// Build a rebirth completion packet (sub=4).
///
/// Client formats string `0xAED2` with rebirth_level and shows it
/// via the notice panel in yellow (`0xFFFFFF00`).
///
/// - `rebirth_level`: The new rebirth level achieved
///
/// Wire: `[u8 sub=4][i32 rebirth_level]`
pub fn build_complete(rebirth_level: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRebirth as u8);
    pkt.write_u8(SUB_COMPLETE);
    pkt.write_i32(rebirth_level);
    pkt
}

// ── C2S Handler ───────────────────────────────────────────────────────

/// Handle WIZ_REBIRTH (0xD3) from the client.
///
/// C2S sub=1 is the rebirth request from the panel/NPC dialog.
/// Server validates requirements and processes rebirth on success.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);

    match sub {
        SUB_ACTIVATE => handle_rebirth_request(session).await,
        _ => {
            debug!(
                "[{}] WIZ_REBIRTH unhandled sub={} ({}B remaining)",
                session.addr(),
                sub,
                reader.remaining()
            );
            Ok(())
        }
    }
}

/// Process a rebirth request (C2S sub=1).
///
/// Binary/ Reference (RebirthBas @ 0x140284b40):
/// 1. Check EXP percent >= 100%
/// 2. Check gold >= 100,000,000
/// 3. Check loyalty >= 10,000
/// 4. On success: send activation + complete packets
async fn handle_rebirth_request(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // ── 1. Gather player state ──
    let (
        exp,
        level,
        gold,
        loyalty,
        rebirth_level,
        char_name,
        str_val,
        sta_val,
        dex_val,
        intel_val,
        cha_val,
    ) = match world.with_session(sid, |h| {
        h.character.as_ref().map(|ch| {
            (
                ch.exp,
                ch.level,
                ch.gold,
                ch.loyalty,
                ch.rebirth_level,
                ch.name.clone(),
                ch.str,
                ch.sta,
                ch.dex,
                ch.intel,
                ch.cha,
            )
        })
    }) {
        Some(Some(data)) => data,
        _ => return Ok(()),
    };

    // Use the same level-up table predicate as the Lua NPC quest flow. The
    // cached max_exp may be stale after rebirth/level changes, and percentage
    // rounding can falsely reject a character at the level cap.
    let required_exp = world.get_exp_by_level(level, rebirth_level);
    let exp_percent = if required_exp > 0 && exp == required_exp as u64 {
        100
    } else {
        0
    };

    // ── 2. Validate requirements (Binary/ RebirthBas parity) ──

    if exp_percent < REBIRTH_EXP_PERCENT {
        warn!(
            "[{}] Rebirth FAIL: EXP {}% < {}%",
            session.addr(),
            exp_percent,
            REBIRTH_EXP_PERCENT
        );
        session.send_packet(&build_result(0)).await?;
        return Ok(());
    }

    if gold < REBIRTH_GOLD_COST {
        warn!(
            "[{}] Rebirth FAIL: gold {} < {}",
            session.addr(),
            gold,
            REBIRTH_GOLD_COST
        );
        session.send_packet(&build_result(0)).await?;
        return Ok(());
    }

    if loyalty < REBIRTH_LOYALTY_COST {
        warn!(
            "[{}] Rebirth FAIL: loyalty {} < {}",
            session.addr(),
            loyalty,
            REBIRTH_LOYALTY_COST
        );
        session.send_packet(&build_result(0)).await?;
        return Ok(());
    }

    // ── 3. Process rebirth ──
    let new_rebirth_level = rebirth_level.saturating_add(1);

    // Snapshot current stats as rebirth bonus stats
    let reb_str = str_val;
    let reb_sta = sta_val;
    let reb_dex = dex_val;
    let reb_intel = intel_val;
    let reb_cha = cha_val;

    // Deduct gold and loyalty, increment rebirth_level, reset EXP
    world.update_character_stats(sid, |ch| {
        ch.gold = ch.gold.saturating_sub(REBIRTH_GOLD_COST);
        ch.loyalty = ch.loyalty.saturating_sub(REBIRTH_LOYALTY_COST);
        ch.rebirth_level = new_rebirth_level;
        ch.reb_str = reb_str;
        ch.reb_sta = reb_sta;
        ch.reb_dex = reb_dex;
        ch.reb_intel = reb_intel;
        ch.reb_cha = reb_cha;
        // Reset EXP to 0 (keep level — rebirth doesn't reset level in v2525)
        ch.exp = 0;
    });

    // ── 4. Send S2C packets ──
    // Activation packet (plays sound, shows "Rebirth activated")
    session.send_packet(&build_activate()).await?;

    // Complete packet (shows "Rebirth Lv X" in yellow)
    session
        .send_packet(&build_complete(new_rebirth_level as i32))
        .await?;

    // ── 5. Persist to DB ──
    if let Some(pool) = world.db_pool() {
        let pool = pool.clone();
        let name = char_name.clone();
        let rl = new_rebirth_level as i16;
        tokio::spawn(async move {
            let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
            if let Err(e) = repo
                .save_rebirth(
                    &name,
                    rl,
                    reb_str as i16,
                    reb_sta as i16,
                    reb_dex as i16,
                    reb_intel as i16,
                    reb_cha as i16,
                    0, // reset EXP
                )
                .await
            {
                tracing::error!("Rebirth DB save failed for {}: {}", name, e);
            }
        });
    }

    warn!(
        "[{}] Rebirth SUCCESS: {} → rebirth_level={}, cost=100M gold + 10K NP",
        session.addr(),
        char_name,
        new_rebirth_level
    );

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── Activate (sub=1) ──────────────────────────────────────────────

    #[test]
    fn test_build_activate_opcode() {
        let pkt = build_activate();
        assert_eq!(pkt.opcode, Opcode::WizRebirth as u8);
    }

    #[test]
    fn test_build_activate_format() {
        let pkt = build_activate();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_ACTIVATE));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_activate_data_length() {
        // u8 sub = 1
        assert_eq!(build_activate().data.len(), 1);
    }

    // ── Result (sub=2) ────────────────────────────────────────────────

    #[test]
    fn test_build_result_opcode() {
        let pkt = build_result(0);
        assert_eq!(pkt.opcode, Opcode::WizRebirth as u8);
    }

    #[test]
    fn test_build_result_format() {
        let pkt = build_result(3);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESULT));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_result_data_length() {
        // u8 sub + u8 result = 2
        assert_eq!(build_result(0).data.len(), 2);
    }

    #[test]
    fn test_build_result_zero() {
        let pkt = build_result(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESULT));
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_build_result_max() {
        let pkt = build_result(u8::MAX);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESULT));
        assert_eq!(r.read_u8(), Some(u8::MAX));
    }

    // ── Progress (sub=3) ──────────────────────────────────────────────

    #[test]
    fn test_build_progress_opcode() {
        let pkt = build_progress(0, 0, 0);
        assert_eq!(pkt.opcode, Opcode::WizRebirth as u8);
    }

    #[test]
    fn test_build_progress_format() {
        let pkt = build_progress(100, 500, 1000);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_PROGRESS));
        assert_eq!(r.read_i32(), Some(100)); // level_offset
        assert_eq!(r.read_i32(), Some(500)); // current
        assert_eq!(r.read_i32(), Some(1000)); // max
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_progress_data_length() {
        // u8 sub + i32×3 = 1 + 12 = 13
        assert_eq!(build_progress(0, 0, 0).data.len(), 13);
    }

    #[test]
    fn test_build_progress_negative_values() {
        let pkt = build_progress(-1, -100, i32::MIN);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_PROGRESS));
        assert_eq!(r.read_i32(), Some(-1));
        assert_eq!(r.read_i32(), Some(-100));
        assert_eq!(r.read_i32(), Some(i32::MIN));
    }

    // ── Complete (sub=4) ──────────────────────────────────────────────

    #[test]
    fn test_build_complete_opcode() {
        let pkt = build_complete(0);
        assert_eq!(pkt.opcode, Opcode::WizRebirth as u8);
    }

    #[test]
    fn test_build_complete_format() {
        let pkt = build_complete(5);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_COMPLETE));
        assert_eq!(r.read_i32(), Some(5)); // rebirth_level
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_complete_data_length() {
        // u8 sub + i32 level = 1 + 4 = 5
        assert_eq!(build_complete(0).data.len(), 5);
    }

    #[test]
    fn test_build_complete_max_level() {
        let pkt = build_complete(i32::MAX);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_COMPLETE));
        assert_eq!(r.read_i32(), Some(i32::MAX));
    }

    // ── Sub-opcode constants ──────────────────────────────────────────

    #[test]
    fn test_sub_opcode_values() {
        assert_eq!(SUB_ACTIVATE, 1);
        assert_eq!(SUB_RESULT, 2);
        assert_eq!(SUB_PROGRESS, 3);
        assert_eq!(SUB_COMPLETE, 4);
    }

    // ── All builders consistent ───────────────────────────────────────

    #[test]
    fn test_all_builders_same_opcode() {
        assert_eq!(build_activate().opcode, Opcode::WizRebirth as u8);
        assert_eq!(build_result(0).opcode, Opcode::WizRebirth as u8);
        assert_eq!(build_progress(0, 0, 0).opcode, Opcode::WizRebirth as u8);
        assert_eq!(build_complete(0).opcode, Opcode::WizRebirth as u8);
    }

    // ── Constants ─────────────────────────────────────────────────────

    #[test]
    fn test_rebirth_constants() {
        assert_eq!(REBIRTH_GOLD_COST, 100_000_000);
        assert_eq!(REBIRTH_LOYALTY_COST, 10_000);
        assert_eq!(REBIRTH_EXP_PERCENT, 100);
    }
}
