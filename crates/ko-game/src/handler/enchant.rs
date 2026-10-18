//! WIZ_ENCHANT (0xCC) handler — weapon/armor + item enchantment system.
//! Dual sub-opcode system: sub=1 for weapon/armor enchant, sub=2 for item enchant.
//! ## Client RE
//! Entry at `0x82FB4D` — reads `[u8 sub]`:
//! - sub=1: Weapon/Armor Enchant panel `[esi+0x678]` → `0xA8D8F0`
//! - sub=2: Item Enchant panel `[esi+0x67C]` → `0xA853D0`
//! Panel-dependent (Group B) — both sub-opcodes null-check panel pointer.
//! ## S2C Packet Formats
//! ### Sub=1 (Weapon/Armor Enchant)
//! ```text
//! first_byte=1: FULL_INIT
//!   [u8 sub=1][u8 first=1][u8 max_star][u8 enchant_count]
//!   [u8 slot_level × 8][u8 slot_unlocked × 9][u8 item_count]
//!   [{u8 type, u32 item_id} × item_count]
//! first_byte=2: RESULT_WITH_INNER
//!   [u8 sub=1][u8 first=2][u8 inner]
//!   inner=1: str 43800, then 9 bools + items (no levels)
//!   inner=2|3: str 12423, inner=4: str 43801, inner=5: str 7434
//! first_byte=3: ENCHANT_RESULT
//!   [u8 sub=1][u8 first=3][u8 result_slot][u8 result_code]
//!   0xFF=cancelled(43802), >=8=max(43803), <8=success(43804)
//! first_byte=4: UNBIND_RESULT
//!   [u8 sub=1][u8 first=4][u8 result]
//!   1=success(43805), 2|5=fail(43802), 3=err(1702), 4=err(16810)
//! first_byte=5: LEVEL_UP_WITH_ANIMATION
//!   [u8 sub=1][u8 first=5][u8 result]
//!   result=1: [u8 new_count][u8 slot_index] → animate(43806)
//!   result=2: fail(43807), result=3: fail(43808)
//! first_byte=6: STAR_LEVEL_UPDATE
//!   [u8 sub=1][u8 first=6][u8 new_star][u8 slot_level × 8]
//!   If new>current: star-up animation (43809)
//! first_byte=8: MAX_ENCHANT_RESULT
//!   [u8 sub=1][u8 first=8][u8 result]
//!   1=success+anim(43803), 2|4=fail(43802), 3=err(1702)
//! ```
//! ### Sub=2 (Item Enchant)
//! ```text
//! first_byte=1: FULL_ITEM_LIST_INIT
//!   [u8 sub=2][u8 first=1][u8 category][u8 item_count]
//!   [{u8 type, u32 item_id} × item_count]
//!   [u8 slot_unlock_count][u8 marker × 4][u8 marker_4]
//! first_byte=2: RESULT_STATUS
//!   [u8 sub=2][u8 first=2][u8 inner]
//!   inner=1: [u8 category], inner=2|3: str 12423, inner=4: str 43858, inner=5: str 7434
//! first_byte=3: ENCHANT_ITEM_RESULT
//!   [u8 sub=2][u8 first=3][u8 inner]
//!   inner=1: [u8 lvl1][u8 lvl2], inner=2: fail(43865)+cooldown=60
//!   inner=3: [i16 val][u8 lvl1][u8 lvl2]
//! first_byte=4: STATUS_RESULT
//!   [u8 sub=2][u8 first=4][u8 result]
//!   2|5: str 43860, 3: str 1702, 4: str 1915
//! first_byte=7: PANEL_CLOSE
//!   [u8 sub=2][u8 first=7]
//! ```
//! ## C2S
//! Single send site at `0x433D45` (script interpreter).
//! Format: `[u8 sub][u8 action][...]` — server responds with S2C above.
//! ## String Table IDs
//! - Sub=1: 43800-43809 (weapon/armor enchant messages)
//! - Sub=2: 43856-43870 (item enchant messages)
//! - Shared: 12423 (generic error), 7434, 1702, 1915

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};

// ── Sub-opcode Constants ────────────────────────────────────────────

/// Sub=1: Weapon/Armor Enchant system.
pub const SUB_WEAPON_ARMOR: u8 = 1;
/// Sub=2: Item Enchant system.
pub const SUB_ITEM: u8 = 2;

// ── Sub=1 First-byte Constants ──────────────────────────────────────

/// Full panel init with all enchant data.
pub const WA_FULL_INIT: u8 = 1;
/// Result with inner sub-dispatch.
pub const WA_RESULT_INNER: u8 = 2;
/// Enchant slot result.
pub const WA_ENCHANT_RESULT: u8 = 3;
/// Unbind result.
pub const WA_UNBIND_RESULT: u8 = 4;
/// Level up with animation.
pub const WA_LEVEL_UP: u8 = 5;
/// Star level update.
pub const WA_STAR_UPDATE: u8 = 6;
/// Max enchant result.
pub const WA_MAX_ENCHANT: u8 = 8;

// ── Sub=2 First-byte Constants ──────────────────────────────────────

/// Full item list init.
pub const ITEM_FULL_INIT: u8 = 1;
/// Result status.
pub const ITEM_RESULT_STATUS: u8 = 2;
/// Enchant item result.
pub const ITEM_ENCHANT_RESULT: u8 = 3;
/// Status result.
pub const ITEM_STATUS_RESULT: u8 = 4;
/// Panel close.
pub const ITEM_PANEL_CLOSE: u8 = 7;

// ── Enchant slot limits ─────────────────────────────────────────────

/// Maximum enchant slots (8 level slots + 9 unlock bools).
const MAX_LEVEL_SLOTS: usize = 8;
/// Maximum unlock bools.
const MAX_UNLOCK_BOOLS: usize = 9;

// ── S2C Builders — Sub=1 (Weapon/Armor Enchant) ────────────────────

/// Build a Sub=1 FULL_INIT packet — opens panel with all enchant data.
/// - `max_star_level`: Current star tier (panel+0x328)
/// - `enchant_count`: Number of completed enchants (panel+0x324)
/// - `slot_levels`: 8 slot levels (panel+0x318..0x31F)
/// - `slot_unlocked`: 9 slot unlock bools
/// - `items`: Vec of (type, item_id) enchant material items
/// Wire: `[u8 sub=1][u8 first=1][u8 max_star][u8 enchant_count]`
///       `[u8 × 8 levels][u8 × 9 unlocks][u8 count][{u8,u32} × N]`
pub fn build_wa_full_init(
    max_star_level: u8,
    enchant_count: u8,
    slot_levels: &[u8; MAX_LEVEL_SLOTS],
    slot_unlocked: &[u8; MAX_UNLOCK_BOOLS],
    items: &[(u8, u32)],
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_FULL_INIT);
    pkt.write_u8(max_star_level);
    pkt.write_u8(enchant_count);
    for &level in slot_levels {
        pkt.write_u8(level);
    }
    for &unlocked in slot_unlocked {
        pkt.write_u8(unlocked);
    }
    pkt.write_u8(items.len() as u8);
    for &(item_type, item_id) in items {
        pkt.write_u8(item_type);
        pkt.write_u32(item_id);
    }
    pkt
}

/// Build a Sub=1 FULL_INIT with empty data — no enchant state.
pub fn build_wa_full_init_empty() -> Packet {
    build_wa_full_init(0, 0, &[0; MAX_LEVEL_SLOTS], &[0; MAX_UNLOCK_BOOLS], &[])
}

/// Build a Sub=1 RESULT_WITH_INNER error packet.
/// - `inner`: Result code (1=success+items, 2|3=err 12423, 4=err 43801, 5=err 7434)
/// Wire: `[u8 sub=1][u8 first=2][u8 inner]`
pub fn build_wa_result_error(inner: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_RESULT_INNER);
    pkt.write_u8(inner);
    pkt
}

/// Build a Sub=1 ENCHANT_RESULT packet.
/// - `result_slot`: Slot index (0xFF = cancelled)
/// - `result_code`: 0xFF=cancelled(43802), >=8=max(43803), <8=success(43804)
/// Wire: `[u8 sub=1][u8 first=3][u8 slot][u8 code]`
pub fn build_wa_enchant_result(result_slot: u8, result_code: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_ENCHANT_RESULT);
    pkt.write_u8(result_slot);
    pkt.write_u8(result_code);
    pkt
}

/// Build a Sub=1 UNBIND_RESULT packet.
/// - `result`: 1=success(43805), 2|5=fail(43802), 3=err(1702), 4=err(16810)
/// Wire: `[u8 sub=1][u8 first=4][u8 result]`
pub fn build_wa_unbind_result(result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_UNBIND_RESULT);
    pkt.write_u8(result);
    pkt
}

/// Build a Sub=1 LEVEL_UP success packet with animation.
/// - `new_enchant_count`: Updated enchant count after level-up
/// - `slot_index`: Slot that leveled up (triggers animation)
/// Wire: `[u8 sub=1][u8 first=5][u8 result=1][u8 count][u8 slot]`
pub fn build_wa_level_up_success(new_enchant_count: u8, slot_index: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_LEVEL_UP);
    pkt.write_u8(1); // result = success
    pkt.write_u8(new_enchant_count);
    pkt.write_u8(slot_index);
    pkt
}

/// Build a Sub=1 LEVEL_UP failure packet.
/// - `result`: 2=fail(43807), 3=fail(43808)
/// Wire: `[u8 sub=1][u8 first=5][u8 result]`
pub fn build_wa_level_up_fail(result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_LEVEL_UP);
    pkt.write_u8(result);
    pkt
}

/// Build a Sub=1 STAR_LEVEL_UPDATE packet.
/// - `new_star_level`: New star tier (if > current → animation, str 43809)
/// - `slot_levels`: Updated 8 slot levels
/// Wire: `[u8 sub=1][u8 first=6][u8 star][u8 × 8 levels]`
pub fn build_wa_star_update(new_star_level: u8, slot_levels: &[u8; MAX_LEVEL_SLOTS]) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_STAR_UPDATE);
    pkt.write_u8(new_star_level);
    for &level in slot_levels {
        pkt.write_u8(level);
    }
    pkt
}

/// Build a Sub=1 MAX_ENCHANT_RESULT packet.
/// - `result`: 1=success+anim(43803), 2|4=fail(43802), 3=err(1702)
/// Wire: `[u8 sub=1][u8 first=8][u8 result]`
pub fn build_wa_max_enchant_result(result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_WEAPON_ARMOR);
    pkt.write_u8(WA_MAX_ENCHANT);
    pkt.write_u8(result);
    pkt
}

// ── S2C Builders — Sub=2 (Item Enchant) ─────────────────────────────

/// Build a Sub=2 FULL_ITEM_LIST_INIT packet.
/// - `category`: Item enchant category (panel+0x44C)
/// - `items`: Vec of (type, item_id) items
/// - `slot_unlock_count`: Number of unlocked slots (≥4 triggers sub=1 panel close)
/// - `markers`: 4 slot markers (0xFF=active)
/// - `marker_4`: 5th marker (0xFF=additional active)
/// Wire: `[u8 sub=2][u8 first=1][u8 cat][u8 count][{u8,u32}×N]`
///       `[u8 unlock_count][u8×4 markers][u8 marker_4]`
pub fn build_item_full_init(
    category: u8,
    items: &[(u8, u32)],
    slot_unlock_count: u8,
    markers: &[u8; 4],
    marker_4: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_FULL_INIT);
    pkt.write_u8(category);
    pkt.write_u8(items.len() as u8);
    for &(item_type, item_id) in items {
        pkt.write_u8(item_type);
        pkt.write_u32(item_id);
    }
    pkt.write_u8(slot_unlock_count);
    for &m in markers {
        pkt.write_u8(m);
    }
    pkt.write_u8(marker_4);
    pkt
}

/// Build a Sub=2 FULL_ITEM_LIST_INIT with empty data.
pub fn build_item_full_init_empty() -> Packet {
    build_item_full_init(0, &[], 0, &[0; 4], 0)
}

/// Build a Sub=2 RESULT_STATUS packet.
/// - `inner`: 1=refresh(+category), 2|3=err 12423, 4=err 43858, 5=err 7434
/// Wire: `[u8 sub=2][u8 first=2][u8 inner]`
pub fn build_item_result_error(inner: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_RESULT_STATUS);
    pkt.write_u8(inner);
    pkt
}

/// Build a Sub=2 RESULT_STATUS with category refresh (inner=1).
/// Wire: `[u8 sub=2][u8 first=2][u8 inner=1][u8 category]`
pub fn build_item_result_refresh(category: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_RESULT_STATUS);
    pkt.write_u8(1); // inner=refresh
    pkt.write_u8(category);
    pkt
}

/// Build a Sub=2 ENCHANT_ITEM_RESULT — inner=1 success.
/// Client reads 2 level bytes and updates display.
/// Wire: `[u8 sub=2][u8 first=3][u8 inner=1][u8 level_1][u8 level_2]`
pub fn build_item_enchant_success(level_1: u8, level_2: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_ENCHANT_RESULT);
    pkt.write_u8(1); // inner=success
    pkt.write_u8(level_1);
    pkt.write_u8(level_2);
    pkt
}

/// Build a Sub=2 ENCHANT_ITEM_RESULT — inner=2 failure.
/// Client shows string 43865 and sets 60s cooldown.
/// Wire: `[u8 sub=2][u8 first=3][u8 inner=2]`
pub fn build_item_enchant_fail() -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_ENCHANT_RESULT);
    pkt.write_u8(2); // inner=fail
    pkt
}

/// Build a Sub=2 ENCHANT_ITEM_RESULT — inner=3 special result.
/// Client reads i16 value + 2 level bytes.
/// Wire: `[u8 sub=2][u8 first=3][u8 inner=3][i16 value][u8 level_1][u8 level_2]`
pub fn build_item_enchant_special(value: i16, level_1: u8, level_2: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_ENCHANT_RESULT);
    pkt.write_u8(3); // inner=special
    pkt.write_i16(value);
    pkt.write_u8(level_1);
    pkt.write_u8(level_2);
    pkt
}

/// Build a Sub=2 STATUS_RESULT packet.
/// - `result`: 2|5=str 43860, 3=str 1702, 4=str 1915
/// Wire: `[u8 sub=2][u8 first=4][u8 result]`
pub fn build_item_status_result(result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_STATUS_RESULT);
    pkt.write_u8(result);
    pkt
}

/// Build a Sub=2 PANEL_CLOSE packet.
/// Wire: `[u8 sub=2][u8 first=7]`
pub fn build_item_panel_close() -> Packet {
    let mut pkt = Packet::new(Opcode::WizEnchant as u8);
    pkt.write_u8(SUB_ITEM);
    pkt.write_u8(ITEM_PANEL_CLOSE);
    pkt
}

// ── C2S Handler ─────────────────────────────────────────────────────

/// Handle WIZ_ENCHANT (0xCC) from the client.
/// C2S format: `[u8 sub][u8 action][...]`
/// Single send site via script interpreter at `0x433D45`.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);

    match sub {
        SUB_WEAPON_ARMOR => handle_weapon_armor(session, &mut reader).await,
        SUB_ITEM => handle_item_enchant(session, &mut reader).await,
        _ => {
            debug!(
                "[{}] WIZ_ENCHANT unknown sub={} ({}B)",
                session.addr(),
                sub,
                reader.remaining()
            );
            Ok(())
        }
    }
}

// ── C2S Action Constants ──────────────────────────────────────────────

/// Open/init panel request (both sub=1 and sub=2).
const C2S_ACTION_OPEN: u8 = 1;
/// Enchant a slot / perform enchant operation.
const C2S_ACTION_ENCHANT: u8 = 2;
/// Unbind an enchanted slot.
const C2S_ACTION_UNBIND: u8 = 3;
/// Level-up: upgrade enchant tier.
const C2S_ACTION_LEVEL_UP: u8 = 4;
/// Max enchant: set all slots to maximum.
const C2S_ACTION_MAX_ENCHANT: u8 = 5;

/// Maximum slot level before considered "max" (S2C: code >= 8 = max).
const MAX_SLOT_LEVEL: u8 = 8;
/// Number of completed enchants needed per level-up.
const ENCHANTS_PER_LEVEL_UP: u8 = 8;
/// Star tier thresholds: min average level across slots for each star tier.
const STAR_THRESHOLDS: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
/// Item enchant success rate (out of 10000). 7000 = 70%.
const ENCHANT_SUCCESS_RATE: u32 = 7000;
/// Item enchant special result rate (out of 10000). 500 = 5% of all rolls.
const ENCHANT_SPECIAL_RATE: u32 = 500;
/// Item enchant cooldown after failure. Client shows 60s timer (string 43865).
const ENCHANT_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(60);

// ── C2S Sub-handlers ─────────────────────────────────────────────────

/// Handle weapon/armor enchant (sub=1) C2S actions.
/// action=1: Open panel → send full_init with session state.
/// action=2: Enchant slot → read `[u8 slot_index]`, upgrade level.
/// action=3: Unbind slot → read `[u8 slot_index]`, reset level to 0.
/// action=4: Level up → upgrade enchant count, unlock next slot.
/// action=5: Max enchant → set all unlocked slots to max level.
/// C2S format is assumed (Binary/ has no reference). All actions
/// log raw remaining bytes for in-game verification.
async fn handle_weapon_armor(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let action = reader.read_u8().unwrap_or(0);
    let remaining_bytes = reader.read_remaining();

    debug!(
        "[{}] WIZ_ENCHANT sub=1 action={} remaining={:02X?}",
        session.addr(),
        action,
        remaining_bytes
    );

    match action {
        C2S_ACTION_OPEN => send_wa_full_init(session).await,
        C2S_ACTION_ENCHANT => {
            // Enchant a specific slot — C2S assumed: [u8 slot_index]
            let slot_index = remaining_bytes.first().copied().unwrap_or(0xFF);
            handle_wa_enchant_slot(session, slot_index).await
        }
        C2S_ACTION_UNBIND => {
            // Unbind a slot — C2S assumed: [u8 slot_index]
            let slot_index = remaining_bytes.first().copied().unwrap_or(0xFF);
            handle_wa_unbind(session, slot_index).await
        }
        C2S_ACTION_LEVEL_UP => {
            // Level up — C2S assumed: [u8 slot_index] or bare
            let slot_index = remaining_bytes.first().copied().unwrap_or(0);
            handle_wa_level_up(session, slot_index).await
        }
        C2S_ACTION_MAX_ENCHANT => {
            // Max enchant all slots
            handle_wa_max_enchant(session).await
        }
        _ => {
            warn!(
                "[{}] WIZ_ENCHANT sub=1 unknown action={} data={:02X?}",
                session.addr(),
                action,
                remaining_bytes
            );
            session.send_packet(&build_wa_result_error(2)).await
        }
    }
}

/// Handle item enchant (sub=2) C2S actions.
/// action=1: Open panel → send full init with session state.
/// action=2: Select category / enchant item.
/// action=3: Perform item enchant.
/// C2S format is assumed (Binary/ has no reference).
async fn handle_item_enchant(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let action = reader.read_u8().unwrap_or(0);
    let remaining_bytes = reader.read_remaining();

    debug!(
        "[{}] WIZ_ENCHANT sub=2 action={} remaining={:02X?}",
        session.addr(),
        action,
        remaining_bytes
    );

    match action {
        C2S_ACTION_OPEN => send_item_full_init(session).await,
        C2S_ACTION_ENCHANT => {
            // Category select — [u8 category]
            let category = remaining_bytes.first().copied().unwrap_or(0);
            handle_item_category_select(session, category).await
        }
        C2S_ACTION_UNBIND => {
            // Toggle marker slot — [u8 marker_slot]
            let marker_slot = remaining_bytes.first().copied().unwrap_or(0);
            handle_item_marker_op(session, marker_slot).await
        }
        C2S_ACTION_LEVEL_UP => {
            // Execute enchant with current markers
            handle_item_execute_enchant(session).await
        }
        _ => {
            warn!(
                "[{}] WIZ_ENCHANT sub=2 unknown action={} data={:02X?}",
                session.addr(),
                action,
                remaining_bytes
            );
            session.send_packet(&build_item_panel_close()).await
        }
    }
}

// ── Sub=1 Operation Handlers ────────────────────────────────────────

/// Send weapon/armor enchant full init with current session state.
async fn send_wa_full_init(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let pkt = session
        .world()
        .with_session(sid, |h| {
            if h.enchant_loaded {
                build_wa_full_init(
                    h.enchant_max_star,
                    h.enchant_count,
                    &h.enchant_slot_levels,
                    &h.enchant_slot_unlocked,
                    &[], // inventory scan for enchant materials — deferred
                )
            } else {
                build_wa_full_init_empty()
            }
        })
        .unwrap_or_else(build_wa_full_init_empty);
    session.send_packet(&pkt).await
}

/// Enchant a specific slot — increment its level.
/// Validates slot index (0-7), checks slot is unlocked, checks not already max.
/// On success: increments level, updates star tier if needed, async DB save.
async fn handle_wa_enchant_slot(session: &mut ClientSession, slot_index: u8) -> anyhow::Result<()> {
    if slot_index >= MAX_LEVEL_SLOTS as u8 {
        // Invalid slot → cancelled
        return session
            .send_packet(&build_wa_enchant_result(0xFF, 0xFF))
            .await;
    }

    let sid = session.session_id();
    let idx = slot_index as usize;

    // Read + validate + update in one atomic operation
    let mut result_slot = 0xFF_u8;
    let mut result_code = 0xFF_u8;
    let mut changed = false;

    session.world().update_session(sid, |h| {
        if !h.enchant_loaded {
            return; // not loaded → cancelled
        }
        // Check slot is unlocked
        if h.enchant_slot_unlocked[idx] == 0 {
            result_slot = slot_index;
            return; // locked → cancelled
        }
        let current_level = h.enchant_slot_levels[idx];
        if current_level >= MAX_SLOT_LEVEL {
            result_slot = slot_index;
            result_code = MAX_SLOT_LEVEL;
            return; // already max
        }
        // Success — increment level
        let new_level = current_level + 1;
        h.enchant_slot_levels[idx] = new_level;
        h.enchant_count = h.enchant_count.saturating_add(1);

        // Check star tier update
        let min_level = h.enchant_slot_levels.iter().copied().min().unwrap_or(0);
        let new_star = STAR_THRESHOLDS
            .iter()
            .rposition(|&threshold| min_level >= threshold)
            .unwrap_or(0) as u8;
        h.enchant_max_star = new_star;

        result_slot = slot_index;
        result_code = new_level;
        changed = true;
    });

    // Send enchant result
    let pkt = build_wa_enchant_result(result_slot, result_code);
    session.send_packet(&pkt).await?;

    // If star tier changed, send star update
    if changed {
        let star_pkt = session.world().with_session(sid, |h| {
            build_wa_star_update(h.enchant_max_star, &h.enchant_slot_levels)
        });
        if let Some(star_pkt) = star_pkt {
            session.send_packet(&star_pkt).await?;
        }
        save_enchant_async(session);
    }

    Ok(())
}

/// Unbind a slot — reset its level to 0.
async fn handle_wa_unbind(session: &mut ClientSession, slot_index: u8) -> anyhow::Result<()> {
    if slot_index >= MAX_LEVEL_SLOTS as u8 {
        return session
            .send_packet(&build_wa_unbind_result(2)) // fail
            .await;
    }

    let sid = session.session_id();
    let idx = slot_index as usize;

    let mut success = false;

    session.world().update_session(sid, |h| {
        if !h.enchant_loaded || h.enchant_slot_unlocked[idx] == 0 {
            return;
        }
        if h.enchant_slot_levels[idx] == 0 {
            return; // nothing to unbind
        }
        h.enchant_slot_levels[idx] = 0;

        // Recalculate star tier
        let min_level = h.enchant_slot_levels.iter().copied().min().unwrap_or(0);
        h.enchant_max_star = STAR_THRESHOLDS
            .iter()
            .rposition(|&t| min_level >= t)
            .unwrap_or(0) as u8;
        success = true;
    });

    let result = if success { 1 } else { 2 }; // 1=success, 2=fail
    session.send_packet(&build_wa_unbind_result(result)).await?;

    if success {
        save_enchant_async(session);
    }
    Ok(())
}

/// Level up — consume enchant count to unlock next slot.
/// When `enchant_count >= ENCHANTS_PER_LEVEL_UP`, the next locked slot
/// gets unlocked and enchant_count resets.
async fn handle_wa_level_up(session: &mut ClientSession, _slot_index: u8) -> anyhow::Result<()> {
    let sid = session.session_id();

    let mut result_code = 2_u8;
    let mut new_count = 0_u8;
    let mut unlocked_slot = 0_u8;

    session.world().update_session(sid, |h| {
        if !h.enchant_loaded {
            result_code = 2; // fail
            return;
        }
        if h.enchant_count < ENCHANTS_PER_LEVEL_UP {
            result_code = 3; // not enough enchants (str 43808)
            return;
        }
        // Find next locked slot to unlock
        let unlock_pos = h.enchant_slot_unlocked.iter().position(|&v| v == 0);
        match unlock_pos {
            Some(idx) if idx < MAX_UNLOCK_BOOLS => {
                h.enchant_slot_unlocked[idx] = 1;
                h.enchant_count = h.enchant_count.saturating_sub(ENCHANTS_PER_LEVEL_UP);
                result_code = 1; // success
                new_count = h.enchant_count;
                unlocked_slot = idx as u8;
            }
            _ => {
                result_code = 2; // all slots already unlocked → fail (str 43807)
            }
        }
    });

    let pkt = if result_code == 1 {
        save_enchant_async(session);
        build_wa_level_up_success(new_count, unlocked_slot)
    } else {
        build_wa_level_up_fail(result_code)
    };
    session.send_packet(&pkt).await
}

/// Max enchant — set all unlocked slots to maximum level.
async fn handle_wa_max_enchant(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();

    let mut success = false;

    session.world().update_session(sid, |h| {
        if !h.enchant_loaded {
            return;
        }
        let mut any_changed = false;
        for i in 0..MAX_LEVEL_SLOTS {
            if h.enchant_slot_unlocked[i] != 0 && h.enchant_slot_levels[i] < MAX_SLOT_LEVEL {
                h.enchant_slot_levels[i] = MAX_SLOT_LEVEL;
                any_changed = true;
            }
        }
        if any_changed {
            // Update star to max since all slots are now max
            let min_level = h.enchant_slot_levels.iter().copied().min().unwrap_or(0);
            h.enchant_max_star = STAR_THRESHOLDS
                .iter()
                .rposition(|&t| min_level >= t)
                .unwrap_or(0) as u8;
        }
        success = any_changed;
    });

    let result = if success { 1 } else { 2 }; // 1=success+anim, 2=fail
    session
        .send_packet(&build_wa_max_enchant_result(result))
        .await?;

    if success {
        // Also send star update with new levels
        let star_pkt = session.world().with_session(sid, |h| {
            build_wa_star_update(h.enchant_max_star, &h.enchant_slot_levels)
        });
        if let Some(star_pkt) = star_pkt {
            session.send_packet(&star_pkt).await?;
        }
        save_enchant_async(session);
    }
    Ok(())
}

// ── Sub=2 Operation Handlers ────────────────────────────────────────

/// Send item enchant full init with current session state.
async fn send_item_full_init(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let pkt = session
        .world()
        .with_session(sid, |h| {
            if h.enchant_loaded {
                let markers_4: [u8; 4] = [
                    h.enchant_item_markers[0],
                    h.enchant_item_markers[1],
                    h.enchant_item_markers[2],
                    h.enchant_item_markers[3],
                ];
                build_item_full_init(
                    h.enchant_item_category,
                    &[], // inventory scan for enchant materials — deferred
                    h.enchant_item_slot_unlock,
                    &markers_4,
                    h.enchant_item_markers[4],
                )
            } else {
                build_item_full_init_empty()
            }
        })
        .unwrap_or_else(build_item_full_init_empty);
    session.send_packet(&pkt).await
}

/// Item enchant — select display category.
async fn handle_item_category_select(
    session: &mut ClientSession,
    category: u8,
) -> anyhow::Result<()> {
    let sid = session.session_id();

    let mut updated = false;

    session.world().update_session(sid, |h| {
        if !h.enchant_loaded {
            return;
        }
        h.enchant_item_category = category;
        updated = true;
    });

    if updated {
        save_enchant_async(session);
        session
            .send_packet(&build_item_result_refresh(category))
            .await
    } else {
        session.send_packet(&build_item_result_error(2)).await
    }
}

/// Execute item enchant using current markers.
/// Rolls for success/fail/special outcome.
/// - Success (inner=1): markers remain, updated levels sent
/// - Fail (inner=2): markers reset, 60s cooldown, client shows string 43865
/// - Special (inner=3): bonus value + levels, rare outcome
/// Success rate: 70% base. Special rate: 5% of successes.
async fn handle_item_execute_enchant(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();

    // Read enchant state snapshot
    let state = session
        .world()
        .with_session(sid, |h| {
            if !h.enchant_loaded {
                return None;
            }
            Some((h.enchant_item_markers, h.enchant_item_last_fail))
        })
        .flatten();

    let (markers, last_fail) = match state {
        Some(s) => s,
        None => return session.send_packet(&build_item_result_error(2)).await,
    };

    // Check 60s cooldown after last failure
    if let Some(fail_time) = last_fail {
        if fail_time.elapsed() < ENCHANT_COOLDOWN {
            debug!(
                "[{}] WIZ_ENCHANT sub=2 execute: cooldown active ({:.0}s remaining)",
                session.addr(),
                (ENCHANT_COOLDOWN - fail_time.elapsed()).as_secs_f32()
            );
            return session.send_packet(&build_item_enchant_fail()).await;
        }
    }

    // Require at least 1 active marker
    let active_count = markers.iter().filter(|&&m| m == 0xFF).count();
    if active_count == 0 {
        return session.send_packet(&build_item_result_error(2)).await;
    }

    // Roll for result
    let roll = rand::random::<u32>() % 10000;
    let lvl1 = markers[0];
    let lvl2 = markers[1];

    if roll >= ENCHANT_SUCCESS_RATE {
        // FAIL — reset all markers, set cooldown
        session.world().update_session(sid, |h| {
            h.enchant_item_markers = [0; 5];
            h.enchant_item_last_fail = Some(std::time::Instant::now());
        });
        save_enchant_async(session);
        debug!(
            "[{}] WIZ_ENCHANT sub=2 execute: FAIL (roll={}, rate={})",
            session.addr(),
            roll,
            ENCHANT_SUCCESS_RATE
        );
        session.send_packet(&build_item_enchant_fail()).await
    } else if roll < ENCHANT_SPECIAL_RATE {
        // SPECIAL — rare bonus result (within the success range)
        let bonus_value = (active_count as i16) * 10;
        save_enchant_async(session);
        debug!(
            "[{}] WIZ_ENCHANT sub=2 execute: SPECIAL (roll={}, bonus={})",
            session.addr(),
            roll,
            bonus_value
        );
        session
            .send_packet(&build_item_enchant_special(bonus_value, lvl1, lvl2))
            .await
    } else {
        // SUCCESS — markers stay as-is
        save_enchant_async(session);
        debug!(
            "[{}] WIZ_ENCHANT sub=2 execute: SUCCESS (roll={})",
            session.addr(),
            roll
        );
        session
            .send_packet(&build_item_enchant_success(lvl1, lvl2))
            .await
    }
}

/// Item marker operation — toggle a marker slot.
async fn handle_item_marker_op(session: &mut ClientSession, marker_slot: u8) -> anyhow::Result<()> {
    let sid = session.session_id();

    if marker_slot >= 5 {
        return session.send_packet(&build_item_status_result(2)).await;
    }

    let mut lvl1 = 0_u8;
    let mut lvl2 = 0_u8;
    let mut toggled = false;

    session.world().update_session(sid, |h| {
        if !h.enchant_loaded {
            return;
        }
        let idx = marker_slot as usize;
        // Toggle marker: 0 → 0xFF, 0xFF → 0
        h.enchant_item_markers[idx] = if h.enchant_item_markers[idx] == 0 {
            0xFF
        } else {
            0
        };
        // Return updated levels for the result packet
        lvl1 = h.enchant_item_markers[0];
        lvl2 = h.enchant_item_markers[1];
        toggled = true;
    });

    if toggled {
        save_enchant_async(session);
        session
            .send_packet(&build_item_enchant_success(lvl1, lvl2))
            .await
    } else {
        session.send_packet(&build_item_status_result(2)).await
    }
}

// ── Async DB Persistence ────────────────────────────────────────────

/// Fire-and-forget save of enchant state to DB.
fn save_enchant_async(session: &ClientSession) {
    let Some(char_id) = session.character_id() else {
        return;
    };
    let name = char_id.to_string();
    let pool = session.pool().clone();
    let world = session.world();
    let sid = session.session_id();

    let data = world
        .with_session(sid, |h| {
            if h.enchant_loaded {
                Some((
                    h.enchant_max_star,
                    h.enchant_count,
                    h.enchant_slot_levels,
                    h.enchant_slot_unlocked,
                    h.enchant_item_category,
                    h.enchant_item_slot_unlock,
                    h.enchant_item_markers,
                ))
            } else {
                None
            }
        })
        .flatten();

    let Some((max_star, enc_count, levels, unlocked, item_cat, item_unlock, markers)) = data else {
        return;
    };

    tokio::spawn(async move {
        let repo = ko_db::repositories::enchant::EnchantRepository::new(&pool);
        if let Err(e) = repo
            .save(
                &name,
                max_star as i16,
                enc_count as i16,
                &levels,
                &unlocked,
                item_cat as i16,
                item_unlock as i16,
                &markers,
            )
            .await
        {
            warn!("Failed to save enchant for {}: {}", name, e);
        }
    });
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    // ── Sub=1 Weapon/Armor Enchant ──────────────────────────────────

    #[test]
    fn test_wa_full_init_empty() {
        let pkt = build_wa_full_init_empty();
        assert_eq!(pkt.opcode, Opcode::WizEnchant as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_WEAPON_ARMOR));
        assert_eq!(r.read_u8(), Some(WA_FULL_INIT));
        assert_eq!(r.read_u8(), Some(0)); // max_star
        assert_eq!(r.read_u8(), Some(0)); // enchant_count
        for _ in 0..8 {
            assert_eq!(r.read_u8(), Some(0)); // slot levels
        }
        for _ in 0..9 {
            assert_eq!(r.read_u8(), Some(0)); // slot unlocked
        }
        assert_eq!(r.read_u8(), Some(0)); // item count
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_full_init_with_items() {
        let levels = [1, 2, 3, 0, 0, 0, 0, 0];
        let unlocks = [1, 1, 1, 0, 0, 0, 0, 0, 0];
        let items = vec![(1u8, 910001000u32), (2, 910002000)];
        let pkt = build_wa_full_init(5, 3, &levels, &unlocks, &items);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub
        assert_eq!(r.read_u8(), Some(1)); // first
        assert_eq!(r.read_u8(), Some(5)); // max_star
        assert_eq!(r.read_u8(), Some(3)); // enchant_count
                                          // 8 slot levels
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(3));
        for _ in 3..8 {
            assert_eq!(r.read_u8(), Some(0));
        }
        // 9 unlock bools
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        for _ in 3..9 {
            assert_eq!(r.read_u8(), Some(0));
        }
        // items
        assert_eq!(r.read_u8(), Some(2)); // count
        assert_eq!(r.read_u8(), Some(1)); // type
        assert_eq!(r.read_u32(), Some(910001000)); // id
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u32(), Some(910002000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_full_init_empty_data_length() {
        // sub(1) + first(1) + star(1) + count(1) + 8 levels + 9 unlocks + item_count(1) = 22
        assert_eq!(build_wa_full_init_empty().data.len(), 22);
    }

    #[test]
    fn test_wa_result_error() {
        let pkt = build_wa_result_error(2); // generic error
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub
        assert_eq!(r.read_u8(), Some(2)); // first=RESULT_WITH_INNER
        assert_eq!(r.read_u8(), Some(2)); // inner
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_enchant_result() {
        let pkt = build_wa_enchant_result(3, 0xFF); // slot 3, cancelled
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(3)); // first=ENCHANT_RESULT
        assert_eq!(r.read_u8(), Some(3)); // slot
        assert_eq!(r.read_u8(), Some(0xFF)); // cancelled
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_unbind_result() {
        let pkt = build_wa_unbind_result(1); // success
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(4)); // first=UNBIND_RESULT
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_level_up_success() {
        let pkt = build_wa_level_up_success(5, 2);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(5)); // first=LEVEL_UP
        assert_eq!(r.read_u8(), Some(1)); // result=success
        assert_eq!(r.read_u8(), Some(5)); // new_count
        assert_eq!(r.read_u8(), Some(2)); // slot_index
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_level_up_fail() {
        let pkt = build_wa_level_up_fail(2);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u8(), Some(2)); // fail
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_star_update() {
        let levels = [3, 3, 3, 3, 0, 0, 0, 0];
        let pkt = build_wa_star_update(4, &levels);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(6)); // first=STAR_UPDATE
        assert_eq!(r.read_u8(), Some(4)); // new star
        for level in &levels {
            assert_eq!(r.read_u8(), Some(*level));
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_wa_star_update_data_length() {
        // sub(1) + first(1) + star(1) + 8 levels = 11
        assert_eq!(build_wa_star_update(0, &[0; 8]).data.len(), 11);
    }

    #[test]
    fn test_wa_max_enchant_result() {
        let pkt = build_wa_max_enchant_result(1); // success
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(8)); // first=MAX_ENCHANT
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.remaining(), 0);
    }

    // ── Sub=2 Item Enchant ──────────────────────────────────────────

    #[test]
    fn test_item_full_init_empty() {
        let pkt = build_item_full_init_empty();
        assert_eq!(pkt.opcode, Opcode::WizEnchant as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub
        assert_eq!(r.read_u8(), Some(1)); // first
        assert_eq!(r.read_u8(), Some(0)); // category
        assert_eq!(r.read_u8(), Some(0)); // item_count
        assert_eq!(r.read_u8(), Some(0)); // slot_unlock_count
        for _ in 0..4 {
            assert_eq!(r.read_u8(), Some(0)); // markers
        }
        assert_eq!(r.read_u8(), Some(0)); // marker_4
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_full_init_with_items() {
        let items = vec![(3u8, 800001000u32)];
        let markers = [0xFF, 0, 0xFF, 0];
        let pkt = build_item_full_init(2, &items, 1, &markers, 0xFF);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub
        assert_eq!(r.read_u8(), Some(1)); // first
        assert_eq!(r.read_u8(), Some(2)); // category
        assert_eq!(r.read_u8(), Some(1)); // count
        assert_eq!(r.read_u8(), Some(3)); // type
        assert_eq!(r.read_u32(), Some(800001000)); // id
        assert_eq!(r.read_u8(), Some(1)); // slot_unlock
        assert_eq!(r.read_u8(), Some(0xFF)); // marker 0
        assert_eq!(r.read_u8(), Some(0)); // marker 1
        assert_eq!(r.read_u8(), Some(0xFF)); // marker 2
        assert_eq!(r.read_u8(), Some(0)); // marker 3
        assert_eq!(r.read_u8(), Some(0xFF)); // marker_4
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_full_init_empty_data_length() {
        // sub(1) + first(1) + cat(1) + count(1) + unlock(1) + 4 markers + marker_4(1) = 10
        assert_eq!(build_item_full_init_empty().data.len(), 10);
    }

    #[test]
    fn test_item_result_error() {
        let pkt = build_item_result_error(4); // err 43858
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(2)); // RESULT_STATUS
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_result_refresh() {
        let pkt = build_item_result_refresh(3);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(2)); // RESULT_STATUS
        assert_eq!(r.read_u8(), Some(1)); // inner=refresh
        assert_eq!(r.read_u8(), Some(3)); // category
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_status_result() {
        let pkt = build_item_status_result(3); // str 1702
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(4)); // STATUS_RESULT
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_panel_close() {
        let pkt = build_item_panel_close();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(7)); // PANEL_CLOSE
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_panel_close_data_length() {
        // sub(1) + first(1) = 2
        assert_eq!(build_item_panel_close().data.len(), 2);
    }

    // ── Sub=2 Item Enchant Result (first_byte=3) ─────────────────

    #[test]
    fn test_item_enchant_success() {
        let pkt = build_item_enchant_success(5, 3);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub=ITEM
        assert_eq!(r.read_u8(), Some(3)); // first=ENCHANT_ITEM_RESULT
        assert_eq!(r.read_u8(), Some(1)); // inner=success
        assert_eq!(r.read_u8(), Some(5)); // level_1
        assert_eq!(r.read_u8(), Some(3)); // level_2
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_enchant_success_data_length() {
        // sub(1) + first(1) + inner(1) + lvl1(1) + lvl2(1) = 5
        assert_eq!(build_item_enchant_success(0, 0).data.len(), 5);
    }

    #[test]
    fn test_item_enchant_fail() {
        let pkt = build_item_enchant_fail();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub
        assert_eq!(r.read_u8(), Some(3)); // first
        assert_eq!(r.read_u8(), Some(2)); // inner=fail
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_enchant_fail_data_length() {
        // sub(1) + first(1) + inner(1) = 3
        assert_eq!(build_item_enchant_fail().data.len(), 3);
    }

    #[test]
    fn test_item_enchant_special() {
        let pkt = build_item_enchant_special(500, 7, 4);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub
        assert_eq!(r.read_u8(), Some(3)); // first
        assert_eq!(r.read_u8(), Some(3)); // inner=special
        assert_eq!(r.read_i16(), Some(500)); // value
        assert_eq!(r.read_u8(), Some(7)); // level_1
        assert_eq!(r.read_u8(), Some(4)); // level_2
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_enchant_special_data_length() {
        // sub(1) + first(1) + inner(1) + i16(2) + lvl1(1) + lvl2(1) = 7
        assert_eq!(build_item_enchant_special(0, 0, 0).data.len(), 7);
    }

    // ── Opcode correctness ──────────────────────────────────────────

    #[test]
    fn test_all_builders_have_correct_opcode() {
        assert_eq!(build_wa_full_init_empty().opcode, 0xCC);
        assert_eq!(build_wa_result_error(2).opcode, 0xCC);
        assert_eq!(build_wa_enchant_result(0, 0).opcode, 0xCC);
        assert_eq!(build_wa_unbind_result(1).opcode, 0xCC);
        assert_eq!(build_wa_level_up_success(0, 0).opcode, 0xCC);
        assert_eq!(build_wa_level_up_fail(2).opcode, 0xCC);
        assert_eq!(build_wa_star_update(0, &[0; 8]).opcode, 0xCC);
        assert_eq!(build_wa_max_enchant_result(1).opcode, 0xCC);
        assert_eq!(build_item_full_init_empty().opcode, 0xCC);
        assert_eq!(build_item_result_error(2).opcode, 0xCC);
        assert_eq!(build_item_result_refresh(0).opcode, 0xCC);
        assert_eq!(build_item_enchant_success(0, 0).opcode, 0xCC);
        assert_eq!(build_item_enchant_fail().opcode, 0xCC);
        assert_eq!(build_item_enchant_special(0, 0, 0).opcode, 0xCC);
        assert_eq!(build_item_status_result(2).opcode, 0xCC);
        assert_eq!(build_item_panel_close().opcode, 0xCC);
    }

    // ── Data length regression tests ────────────────────────────────

    #[test]
    fn test_wa_error_data_lengths() {
        // sub(1) + first(1) + inner(1) = 3
        assert_eq!(build_wa_result_error(2).data.len(), 3);
        assert_eq!(build_wa_unbind_result(1).data.len(), 3);
        assert_eq!(build_wa_max_enchant_result(1).data.len(), 3);
        assert_eq!(build_wa_level_up_fail(2).data.len(), 3);
    }

    #[test]
    fn test_wa_enchant_result_data_length() {
        // sub(1) + first(1) + slot(1) + code(1) = 4
        assert_eq!(build_wa_enchant_result(0, 0).data.len(), 4);
    }

    #[test]
    fn test_wa_level_up_success_data_length() {
        // sub(1) + first(1) + result(1) + count(1) + slot(1) = 5
        assert_eq!(build_wa_level_up_success(0, 0).data.len(), 5);
    }

    // ── Enchant execution constants ─────────────────────────────────

    #[test]
    fn test_enchant_success_rate_in_range() {
        assert!(ENCHANT_SUCCESS_RATE <= 10000);
        assert!(ENCHANT_SUCCESS_RATE > 0);
    }

    #[test]
    fn test_enchant_special_rate_less_than_success() {
        assert!(ENCHANT_SPECIAL_RATE < ENCHANT_SUCCESS_RATE);
    }

    #[test]
    fn test_enchant_cooldown_is_60s() {
        assert_eq!(ENCHANT_COOLDOWN, std::time::Duration::from_secs(60));
    }

    #[test]
    fn test_enchant_fail_triggers_cooldown_packet() {
        // build_item_enchant_fail sends inner=2 which triggers 60s cooldown on client
        let pkt = build_item_enchant_fail();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_ITEM));
        assert_eq!(r.read_u8(), Some(ITEM_ENCHANT_RESULT));
        assert_eq!(r.read_u8(), Some(2)); // inner=2 → fail + 60s cooldown
    }

    #[test]
    fn test_enchant_special_has_value_and_levels() {
        let pkt = build_item_enchant_special(30, 0xFF, 0xFF);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_ITEM));
        assert_eq!(r.read_u8(), Some(ITEM_ENCHANT_RESULT));
        assert_eq!(r.read_u8(), Some(3)); // inner=3 → special
        assert_eq!(r.read_i16(), Some(30)); // bonus value
        assert_eq!(r.read_u8(), Some(0xFF)); // lvl1 (marker)
        assert_eq!(r.read_u8(), Some(0xFF)); // lvl2 (marker)
    }
}
