//! Wheel of Fun event handler.
//! ## Overview
//! The Wheel of Fun is a KC-cost gacha spin. The client sends a spin request
//! via `WIZ_EXT_HOOK (0xE9)` with sub-opcode `WheelData = 0xDA`.
//! ## Wire Format
//! **Client → Server (spin request):**
//! ```text
//! WIZ_EXT_HOOK (0xE9) << u8(0xDA)
//! ```
//! **Server → Client (wheel data — sent on game entry):**
//! ```text
//! WIZ_EXT_HOOK (0xE9) << u8(0xDA) << u16(count) << [u32 item_id; count]
//! ```
//! **Server → Client (item notice — sent after successful spin):**
//! ```text
//! WIZ_EXT_HOOK (0xE9) << u8(0xBA) << u8(3) << u32(item_id)
//! ```

use ko_protocol::{Opcode, Packet};
use rand::Rng;
use tracing::{debug, warn};

use crate::handler::knight_cash;
use crate::session::ClientSession;

use super::ext_hook::EXT_SUB_AUTODROP;
pub(crate) use super::ext_hook::EXT_SUB_WHEEL_DATA;

/// KC cost per wheel spin.
const WHEEL_SPIN_COST: i32 = 350;

/// Maximum entries in the random slot array.
const MAX_RAND_SLOTS: usize = 9999;

/// Maximum number of wheel settings entries to send to client.
const MAX_WHEEL_ENTRIES: usize = 25;

// ─────────────────────────────────────────────────────────────────────────────
// Packet Builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build the wheel data packet sent to client on game entry.
/// Wire: `WIZ_EXT_HOOK (0xE9) << u8(0xDA) << u16(count) << [u32 item_id; count]`
pub fn build_wheel_data_packet(item_ids: &[u32]) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_WHEEL_DATA);
    pkt.write_u16(item_ids.len() as u16);
    for &id in item_ids {
        pkt.write_u32(id);
    }
    pkt
}

/// Build the item notice packet (AUTODROP type=3).
/// Wire: `WIZ_EXT_HOOK (0xE9) << u8(0xBA) << u8(3) << u32(item_id)`
fn build_item_notice_packet(item_id: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_AUTODROP);
    pkt.write_u8(3); // type = 3 (wheel of fun)
    pkt.write_u32(item_id);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Send Wheel Data (game entry)
// ─────────────────────────────────────────────────────────────────────────────

/// Send wheel data to the client on game entry.
/// Loads wheel settings from the world's cached table, builds the item list,
/// and sends it. Skips if settings are empty or exceed 25 entries.
pub async fn send_wheel_data(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();

    let settings = world.get_wheel_of_fun_settings();
    if settings.is_empty() || settings.len() > MAX_WHEEL_ENTRIES {
        return Ok(());
    }

    let item_ids: Vec<u32> = settings.iter().map(|s| s.item_id as u32).collect();
    let pkt = build_wheel_data_packet(&item_ids);
    session.send_packet(&pkt).await?;

    debug!(
        "[{}] SendWheelData: {} entries",
        session.addr(),
        item_ids.len()
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handle Spin Request
// ─────────────────────────────────────────────────────────────────────────────

/// Handle a Wheel of Fun spin request from the client.
/// 1. Check in-game, not trading/merchanting
/// 2. Check KC >= 350
/// 3. Build weighted random array from settings
/// 4. Pick random slot → item
/// 5. Check free inventory slots (>= 2)
/// 6. Deduct 350 KC
/// 7. Give item
/// 8. Send item notice
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // Guard: in-game, not trading/merchanting
    let is_valid = world
        .with_session(sid, |h| h.character.is_some())
        .unwrap_or(false);
    if !is_valid || world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }

    // Guard: sufficient KC
    let current_kc = world.get_knight_cash(sid);
    if (current_kc as i32) < WHEEL_SPIN_COST {
        debug!(
            "[{}] WheelOfFun: insufficient KC ({} < {})",
            session.addr(),
            current_kc,
            WHEEL_SPIN_COST
        );
        return Ok(());
    }

    // Load wheel settings
    let settings = world.get_wheel_of_fun_settings();
    if settings.is_empty() {
        return Ok(());
    }

    // Build weighted random array and pick a winner
    let (item_id, item_count) = {
        let mut rand_array: Vec<usize> = Vec::with_capacity(MAX_RAND_SLOTS);
        for (i, setting) in settings.iter().enumerate() {
            let slot_count = (setting.drop_rate / 5) as usize;
            for _ in 0..slot_count {
                if rand_array.len() >= MAX_RAND_SLOTS {
                    break;
                }
                rand_array.push(i);
            }
            if rand_array.len() >= MAX_RAND_SLOTS {
                break;
            }
        }

        if rand_array.is_empty() {
            return Ok(());
        }

        // Pick random slot (scope rng before any await)
        let mut rng = rand::thread_rng();
        let rand_idx = rng.gen_range(0..rand_array.len());
        let setting_idx = rand_array[rand_idx];

        let setting = match settings.get(setting_idx) {
            Some(s) => s,
            None => return Ok(()),
        };

        (
            setting.item_id as u32,
            setting.item_count.clamp(0, u16::MAX as i32) as u16,
        )
    };

    // Guard: free inventory slots (>= 2)
    let free_slots = world.count_free_slots(sid);
    if free_slots <= 1 {
        debug!(
            "[{}] WheelOfFun: not enough free slots ({})",
            session.addr(),
            free_slots
        );
        let notice = crate::systems::timed_notice::build_notice_packet(
            7,
            "[Wheel] You need at least 2 free inventory slots.",
        );
        session.send_packet(&notice).await?;
        return Ok(());
    }

    // Deduct KC
    if !knight_cash::deduct_kc(session, WHEEL_SPIN_COST).await? {
        warn!("[{}] WheelOfFun: KC deduction failed", session.addr());
        return Ok(());
    }

    // Give item. The 350 KC are already gone at this point, so a failure must give them back
    // (an item missing from the item table used to eat the spin without any result).
    if !world.give_item(sid, item_id, item_count) {
        warn!(
            "[{}] WheelOfFun: give_item failed for item_id={}, refunding {} KC",
            session.addr(),
            item_id,
            WHEEL_SPIN_COST
        );
        knight_cash::cash_gain(&world, session.pool(), sid, WHEEL_SPIN_COST as u32);
        let notice = crate::systems::timed_notice::build_notice_packet(
            7,
            "[Wheel] The prize could not be given; your Knight Cash was refunded.",
        );
        session.send_packet(&notice).await?;
        return Ok(());
    }

    // Send item notice to self
    let notice = build_item_notice_packet(item_id);
    session.send_packet(&notice).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_msg = format!("[Wheel] You won item {}! (x{})", item_id, item_count);
    let chat_pkt = crate::systems::timed_notice::build_notice_packet(7, &chat_msg);
    session.send_packet(&chat_pkt).await?;

    debug!(
        "[{}] WheelOfFun spin: won item_id={} count={}",
        session.addr(),
        item_id,
        item_count
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_wheel_data_packet() {
        let items = vec![100001u32, 100002, 100003];
        let pkt = build_wheel_data_packet(&items);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        // data: [0xDA, count_lo, count_hi, item1 x4, item2 x4, item3 x4]
        assert_eq!(pkt.data[0], EXT_SUB_WHEEL_DATA);
        assert_eq!(pkt.data[1], 3); // count lo
        assert_eq!(pkt.data[2], 0); // count hi
                                    // item1 = 100001 = 0x000186A1 little-endian
        let id1 = u32::from_le_bytes([pkt.data[3], pkt.data[4], pkt.data[5], pkt.data[6]]);
        assert_eq!(id1, 100001);
    }

    #[test]
    fn test_build_item_notice_packet() {
        let pkt = build_item_notice_packet(900001);
        assert_eq!(pkt.data[0], EXT_SUB_AUTODROP);
        assert_eq!(pkt.data[1], 3); // type
        let id = u32::from_le_bytes([pkt.data[2], pkt.data[3], pkt.data[4], pkt.data[5]]);
        assert_eq!(id, 900001);
    }

    #[test]
    fn test_build_wheel_data_empty() {
        let pkt = build_wheel_data_packet(&[]);
        assert_eq!(pkt.data[0], EXT_SUB_WHEEL_DATA);
        assert_eq!(pkt.data[1], 0); // count = 0
        assert_eq!(pkt.data[2], 0);
        assert_eq!(pkt.data.len(), 3);
    }

    #[test]
    fn test_max_rand_slots_constant() {
        assert_eq!(MAX_RAND_SLOTS, 9999);
    }

    #[test]
    fn test_wheel_spin_cost_constant() {
        assert_eq!(WHEEL_SPIN_COST, 350);
    }

    // ── Sprint 923: Additional coverage ──────────────────────────────

    /// Wheel data packet length: sub(1) + count(2) + items(N*4).
    #[test]
    fn test_wheel_data_packet_length() {
        let items: Vec<u32> = (1..=10).collect();
        let pkt = build_wheel_data_packet(&items);
        // 1 + 2 + 10*4 = 43
        assert_eq!(pkt.data.len(), 43);
    }

    /// Item notice data length: sub(1) + type(1) + item_id(4) = 6.
    #[test]
    fn test_item_notice_data_length() {
        let pkt = build_item_notice_packet(123456);
        assert_eq!(pkt.data.len(), 6);
    }

    /// MAX_WHEEL_ENTRIES = 25
    #[test]
    fn test_max_wheel_entries_constant() {
        assert_eq!(MAX_WHEEL_ENTRIES, 25);
    }

    /// Wheel data with max 25 entries.
    #[test]
    fn test_wheel_data_max_entries() {
        let items: Vec<u32> = (100001..=100025).collect();
        assert_eq!(items.len(), 25);
        let pkt = build_wheel_data_packet(&items);
        // count field
        assert_eq!(pkt.data[1], 25);
        assert_eq!(pkt.data[2], 0);
        // 1 + 2 + 25*4 = 103
        assert_eq!(pkt.data.len(), 103);
    }

    /// Wheel data roundtrip: write then read back all item IDs.
    #[test]
    fn test_wheel_data_roundtrip() {
        use ko_protocol::PacketReader;
        let items = vec![500001u32, 500002, 500003, 500004, 500005];
        let pkt = build_wheel_data_packet(&items);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_WHEEL_DATA));
        assert_eq!(r.read_u16(), Some(5));
        for &expected in &items {
            assert_eq!(r.read_u32(), Some(expected));
        }
        assert_eq!(r.remaining(), 0);
    }
}
