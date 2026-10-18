//! WIZ_PREMIUM2 (0xAC) — gate keeper tax system.
//! v2525-specific feature — distinct from `WIZ_PREMIUM (0x71)`.
//! Client UI class: `CUIEventPremium`, panel at `[esi+0x5B4]`.
//! ## Client RE Summary
//! ### S2C Dispatch (`0xAA9150`)
//! ```text
//! Sub=1: Init panel — [u8=1][i32 value]
//!   → reset UI, format string 10228 with value, set Text_Title2, show panel
//! Sub=2, second_sub=0: Item+gold dialog — [u8=2][u8=0][i32 item_id][i32 gold_amount]
//!   → look up item name, find "text_gold" UI element, format string 10231, show dialog
//! Sub=2, second_sub=1: Error/info message — [u8=2][u8=1]
//!   → show string 10235 dialog (no additional data)
//! Sub=2, second_sub=2: Error/info message — [u8=2][u8=2]
//!   → show string 10229 dialog (no additional data)
//! ```
//! ### C2S
//! ```text
//! btn_ok click: [u8=2] — player confirms the premium/tax operation
//! ```
//! ### UI Elements
//! - `btn_tax` `[+0x10C]`: transitions to confirmation view
//! - `btn_exit` `[+0x110]`: close panel
//! - `btn_ok` `[+0x114]`: send C2S confirmation
//! - `btn_cancle` `[+0x118]`: reset UI (sic — typo in client)
//! - `Text_Title1` `[+0x120]`: initial title
//! - `Text_Title2` `[+0x124]`: confirmation/detail text
//! ### String IDs
//! - 10228 (0x27F4): sub=1 format string (tax/premium amount)
//! - 10229 (0x27F5): sub=2/second_sub=2 message
//! - 10231 (0x27F7): sub=2/second_sub=0 gold format
//! - 10235 (0x27FB): sub=2/second_sub=1 message

use ko_protocol::Opcode;
use ko_protocol::Packet;
use ko_protocol::PacketReader;
use tracing::debug;

use crate::session::{ClientSession, SessionState};

// ── Sub-opcode constants ────────────────────────────────────────────────

/// Sub=1: Init/show the premium panel with a value.
pub const SUB_INIT: u8 = 1;

/// Sub=2: Response/dialog sub-opcodes.
pub const SUB_RESPONSE: u8 = 2;

// ── Response inner sub-opcodes (second byte when sub=2) ─────────────────

/// second_sub=0: Item + gold dialog.
pub const RESPONSE_ITEM_GOLD: u8 = 0;

/// second_sub=1: Message dialog (string 10235).
pub const RESPONSE_MSG_1: u8 = 1;

/// second_sub=2: Message dialog (string 10229).
pub const RESPONSE_MSG_2: u8 = 2;

// ── S2C Builders ────────────────────────────────────────────────────────

/// Build S2C init panel packet.
/// Wire: `[u8=1][i32 value]`
/// Client formats string 10228 with the value and shows the panel.
pub fn build_init(value: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizPremium2 as u8);
    pkt.write_u8(SUB_INIT);
    pkt.write_i32(value);
    pkt
}

/// Build S2C item + gold dialog packet.
/// Wire: `[u8=2][u8=0][i32 item_id][i32 gold_amount]`
/// Client looks up item name and formats string 10231 with gold.
pub fn build_item_gold(item_id: i32, gold_amount: i32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizPremium2 as u8);
    pkt.write_u8(SUB_RESPONSE);
    pkt.write_u8(RESPONSE_ITEM_GOLD);
    pkt.write_i32(item_id);
    pkt.write_i32(gold_amount);
    pkt
}

/// Build S2C message dialog (string 10235).
/// Wire: `[u8=2][u8=1]`
pub fn build_msg_1() -> Packet {
    let mut pkt = Packet::new(Opcode::WizPremium2 as u8);
    pkt.write_u8(SUB_RESPONSE);
    pkt.write_u8(RESPONSE_MSG_1);
    pkt
}

/// Build S2C message dialog (string 10229).
/// Wire: `[u8=2][u8=2]`
pub fn build_msg_2() -> Packet {
    let mut pkt = Packet::new(Opcode::WizPremium2 as u8);
    pkt.write_u8(SUB_RESPONSE);
    pkt.write_u8(RESPONSE_MSG_2);
    pkt
}

// ── Public API ──────────────────────────────────────────────────────────

/// Initiate a gate keeper tax dialog for the player.
/// Stores the pending tax in session state and sends the init panel.
/// When the player clicks "OK", the C2S handler deducts gold.
/// - `tax_amount`: Gold to charge (displayed in text_id 10228)
pub async fn initiate_gate_tax(session: &mut ClientSession, tax_amount: u32) -> anyhow::Result<()> {
    if tax_amount == 0 {
        return Ok(());
    }

    let sid = session.session_id();
    session.world().update_session(sid, |h| {
        h.pending_gate_tax = tax_amount;
    });

    session.send_packet(&build_init(tax_amount as i32)).await
}

// ── C2S Handler ─────────────────────────────────────────────────────────

/// Handle WIZ_PREMIUM2 (0xAC) C2S — player confirmation.
/// Client sends `[u8=2]` when btn_ok is clicked to confirm premium/tax.
/// Deducts the pending gate tax from the player's gold.
pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let sub = reader.read_u8().unwrap_or(0);
    debug!(
        "[{}] WIZ_PREMIUM2 sub={} ({}B remaining)",
        session.addr(),
        sub,
        reader.remaining()
    );

    match sub {
        SUB_RESPONSE => {
            handle_confirm(session).await?;
        }
        _ => {
            debug!("[{}] WIZ_PREMIUM2 unknown sub={}", session.addr(), sub);
        }
    }

    Ok(())
}

/// Process player's tax confirmation — deduct gold and respond.
async fn handle_confirm(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // Read pending tax, then clear it.
    let tax = world.with_session(sid, |h| h.pending_gate_tax).unwrap_or(0);
    if tax > 0 {
        world.update_session(sid, |h| {
            h.pending_gate_tax = 0;
        });
    }

    if tax == 0 {
        // No pending tax — send error message (string 10235).
        debug!(
            "[{}] WIZ_PREMIUM2 confirm with no pending tax",
            session.addr()
        );
        return session.send_packet(&build_msg_1()).await;
    }

    // Check if player has enough gold.
    let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);

    if gold < tax {
        // Not enough gold — send error message (string 10229).
        debug!(
            "[{}] WIZ_PREMIUM2 insufficient gold: have={}, need={}",
            session.addr(),
            gold,
            tax
        );
        return session.send_packet(&build_msg_2()).await;
    }

    // Deduct gold.
    world.update_session(sid, |h| {
        if let Some(ref mut ch) = h.character {
            ch.gold = ch.gold.saturating_sub(tax);
        }
    });

    let new_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);

    // Send gold change to client.
    let mut gold_pkt = Packet::new(Opcode::WizGoldChange as u8);
    gold_pkt.write_u8(2); // GoldLose
    gold_pkt.write_u32(tax);
    gold_pkt.write_u32(new_gold);
    session.send_packet(&gold_pkt).await?;

    debug!(
        "[{}] WIZ_PREMIUM2 tax={} deducted, new_gold={}",
        session.addr(),
        tax,
        new_gold
    );

    // Send success message (string 10235).
    session.send_packet(&build_msg_1()).await
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_wire_format() {
        let pkt = build_init(50000);
        assert_eq!(pkt.opcode, Opcode::WizPremium2 as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_INIT)); // sub=1
        assert_eq!(r.read_i32(), Some(50000)); // value
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_init_data_length() {
        // u8(1) + i32(4) = 5 bytes
        assert_eq!(build_init(0).data.len(), 5);
    }

    #[test]
    fn test_item_gold_wire_format() {
        let pkt = build_item_gold(389010000, 100_000);
        assert_eq!(pkt.opcode, Opcode::WizPremium2 as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESPONSE)); // sub=2
        assert_eq!(r.read_u8(), Some(RESPONSE_ITEM_GOLD)); // second_sub=0
        assert_eq!(r.read_i32(), Some(389010000)); // item_id
        assert_eq!(r.read_i32(), Some(100_000)); // gold_amount
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_item_gold_data_length() {
        // u8(1) + u8(1) + i32(4) + i32(4) = 10 bytes
        assert_eq!(build_item_gold(0, 0).data.len(), 10);
    }

    #[test]
    fn test_msg_1_wire_format() {
        let pkt = build_msg_1();
        assert_eq!(pkt.opcode, Opcode::WizPremium2 as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESPONSE)); // sub=2
        assert_eq!(r.read_u8(), Some(RESPONSE_MSG_1)); // second_sub=1
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_msg_2_wire_format() {
        let pkt = build_msg_2();
        assert_eq!(pkt.opcode, Opcode::WizPremium2 as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESPONSE)); // sub=2
        assert_eq!(r.read_u8(), Some(RESPONSE_MSG_2)); // second_sub=2
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_msg_data_lengths() {
        // u8(1) + u8(1) = 2 bytes each
        assert_eq!(build_msg_1().data.len(), 2);
        assert_eq!(build_msg_2().data.len(), 2);
    }

    #[test]
    fn test_all_opcodes_correct() {
        assert_eq!(build_init(0).opcode, 0xAC);
        assert_eq!(build_item_gold(0, 0).opcode, 0xAC);
        assert_eq!(build_msg_1().opcode, 0xAC);
        assert_eq!(build_msg_2().opcode, 0xAC);
    }

    #[test]
    fn test_negative_value() {
        let pkt = build_init(-100);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_i32(), Some(-100));
    }

    // ── C2S confirmation tests ──────────────────────────────────────────

    #[test]
    fn test_c2s_confirm_format() {
        // Client sends [u8=2] on btn_ok click
        let mut pkt = Packet::new(Opcode::WizPremium2 as u8);
        pkt.write_u8(SUB_RESPONSE);
        assert_eq!(pkt.data.len(), 1);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_RESPONSE));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_sub_constants() {
        assert_eq!(SUB_INIT, 1);
        assert_eq!(SUB_RESPONSE, 2);
        assert_eq!(RESPONSE_ITEM_GOLD, 0);
        assert_eq!(RESPONSE_MSG_1, 1);
        assert_eq!(RESPONSE_MSG_2, 2);
    }

    #[test]
    fn test_msg_1_is_string_10235() {
        // build_msg_1 → [u8=2][u8=1] → client shows string 10235
        let pkt = build_msg_1();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(1)); // RESPONSE_MSG_1 → string 10235
    }

    #[test]
    fn test_msg_2_is_string_10229() {
        // build_msg_2 → [u8=2][u8=2] → client shows string 10229
        let pkt = build_msg_2();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(2)); // RESPONSE_MSG_2 → string 10229
    }

    #[test]
    fn test_init_for_gate_tax() {
        // initiate_gate_tax sends build_init(tax) to show panel
        let tax: u32 = 5000;
        let pkt = build_init(tax as i32);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(SUB_INIT));
        assert_eq!(r.read_i32(), Some(5000));
    }
}
