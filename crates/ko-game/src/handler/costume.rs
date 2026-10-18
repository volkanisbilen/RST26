//! WIZ_COSTUME (0xC3) handler — costume appearance system.
//!
//! v2525 client's native costume panel (panel at `[esi+0x63C]`).
//! Manages costume equip, dye colors, 3D model attachments.
//!
//! ## Client RE
//!
//! - Panel object: `[esi+0x63C]` — created on UI open, null-checked before dispatch
//! - Main handler: `0xD850E0` — sub-opcode switch (sub 1-7)
//! - Busy flag: `[this+0x440]` set at entry, cleared at exit
//! - C2S: sub=1 (list), sub=2 (equip), sub=3 (select), sub=4 (clear), sub=6 (attach), sub=7 (dye)
//! - S2C: sub=1 (status), sub=2 (equip result), sub=3 (model change), sub=4 (clear attachments),
//!   sub=5 (store/buy), sub=6 (attach result), sub=7 (dye catalog)
//!
//! ## Dye Colors (14 hardcoded)
//!
//! | Idx | Name       | ARGB       |
//! |-----|------------|------------|
//! |  0  | AliceBlue  | 0xFFF0F8FF |
//! |  1  | Aquamarine | 0xFF7FFFD4 |
//! |  2  | Coral      | 0xFFFF7F50 |
//! |  3  | Khaki      | 0xFFF0E68C |
//! |  4  | Crimson    | 0xFFDC143C |
//! |  5  | Cyan       | 0xFF00FFFF |
//! |  6  | DeepPink   | 0xFFFF1493 |
//! |  7  | IndianRed  | 0xFFF15F5F |
//! |  8  | Gold       | 0xFFFFD700 |
//! |  9  | GreenYellw | 0xFFADFF2F |
//! | 10  | BurlyWood  | 0xFFDEB887 |
//! | 11  | Yellow     | 0xFFFFFF00 |
//! | 12  | PowderBlue | 0xFFB2EBF4 |
//! | 13  | White      | 0xFFFFFFFF |

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use ko_db::repositories::costume::CostumeRepository;

// ── S2C Sub-type constants ────────────────────────────────────────────────

/// Sub 1: Costume status — `[u16 active_type][optional item data]`.
const COSTUME_SUB_STATUS: u8 = 1;

/// Sub 2: Equip result — `[u16 result_code][u16 detail_code]`.
const COSTUME_SUB_EQUIP_RESULT: u8 = 2;

/// Sub 3: Model change result — `[u16 count1][u16 count2][conditional]`.
#[cfg(test)]
const COSTUME_SUB_MODEL_CHANGE: u8 = 3;

/// Sub 4: Clear attachments — `[u16 param_id]`.
const COSTUME_SUB_CLEAR: u8 = 4;

/// Sub 5: Store/buy result — `[u8 mode][u8 action][conditional]`.
#[cfg(test)]
const COSTUME_SUB_STORE: u8 = 5;

/// Sub 6: Attachment equip result — `[u16 result_code]`.
const COSTUME_SUB_ATTACH_RESULT: u8 = 6;

/// Sub 7: Dye color catalog — `[u16 color_count][nested items]`.
const COSTUME_SUB_DYE_CATALOG: u8 = 7;

// ── Active type for Sub 1 ─────────────────────────────────────────────────

/// active_type=0: empty — shows string 0xAABB.
const STATUS_EMPTY: u16 = 0;

/// active_type=2: equipped — shows 0xAABA, client requests list.
const STATUS_EQUIPPED: u16 = 2;

// ── Result codes ──────────────────────────────────────────────────────────

/// Equip/attach/model success.
const RESULT_SUCCESS: i16 = 1;

/// Generic error — string 0x407A.
const RESULT_ERROR: i16 = 0;

// ── Dye color table (14 entries, client jump table at 0xD87C34) ───────────

/// 14 costume dye colors (ARGB) matching the client's hardcoded palette.
pub const DYE_COLORS: [u32; 14] = [
    0xFFF0F8FF, // 0: AliceBlue
    0xFF7FFFD4, // 1: Aquamarine
    0xFFFF7F50, // 2: Coral
    0xFFF0E68C, // 3: Khaki
    0xFFDC143C, // 4: Crimson
    0xFF00FFFF, // 5: Cyan
    0xFFFF1493, // 6: DeepPink
    0xFFF15F5F, // 7: IndianRed (custom)
    0xFFFFD700, // 8: Gold
    0xFFADFF2F, // 9: GreenYellow
    0xFFDEB887, // 10: BurlyWood
    0xFFFFFF00, // 11: Yellow
    0xFFB2EBF4, // 12: PowderBlue (custom)
    0xFFFFFFFF, // 13: White
];

// ── S2C Packet Builders ──────────────────────────────────────────────────

/// Build a Sub 1 (costume status) empty response — "no costumes".
///
/// Client RE: `0xD8514D` — active_type=0 shows string 0xAABB (no costume).
///
/// Wire: `[0xC3][0x01][u16 active_type=0]`
pub fn build_status_empty() -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_STATUS);
    pkt.write_u16(STATUS_EMPTY);
    pkt
}

/// Build a Sub 1 (costume status) with active costume data.
///
/// Client RE: `0xD8514D` — type 1/2/3 reads:
/// `[i32 expiry_countdown][i32 item_param][u8 skip][i32 scale_raw][u8 color_index]`
///
/// Wire: `[0xC3][0x01][u16 type][i32 expiry][i32 item_param][u8 0][i32 scale][u8 color]`
pub fn build_status_active(
    active_type: u16,
    expiry_countdown: i32,
    item_param: i32,
    scale_raw: i32,
    color_index: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_STATUS);
    pkt.write_u16(active_type);
    pkt.write_i32(expiry_countdown);
    pkt.write_i32(item_param);
    pkt.write_u8(0); // unknown byte (skipped by client)
    pkt.write_i32(scale_raw);
    pkt.write_u8(color_index.min(13)); // clamp to valid range
    pkt
}

/// Build a Sub 2 (equip result) packet.
///
/// Client RE: `0xD87311` — result_code dispatch (jump table at 0xD87C6C).
/// Success=1, error codes 0 to -9.
///
/// Wire: `[0xC3][0x02][u16 result_code][u16 detail_code]`
fn build_equip_result(result_code: i16, detail_code: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_EQUIP_RESULT);
    pkt.write_u16(result_code as u16);
    pkt.write_u16(detail_code as u16);
    pkt
}

/// Build a Sub 4 (clear attachments) packet.
///
/// Client RE: `0xD85CAC` — iterates attachment vectors at `[this+0x398]`
/// and `[this+0x3A4]`, destroying each 3D model object.
///
/// Wire: `[0xC3][0x04][u16 param_id]`
fn build_clear_attachments(param_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_CLEAR);
    pkt.write_u16(param_id);
    pkt
}

/// Build a Sub 6 (attachment equip result) packet.
///
/// Client RE: `0xD876FC` — result_code dispatch (jump table at 0xD87C98).
/// Success=1, error codes 0 to -6.
///
/// Wire: `[0xC3][0x06][u16 result_code]`
fn build_attach_result(result_code: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_ATTACH_RESULT);
    pkt.write_u16(result_code as u16);
    pkt
}

/// Build a Sub 7 (dye catalog) empty response — no items in any color slot.
///
/// Client RE: `0xD866C2` — outer loop: `u16 color_count` colors,
/// inner loop: 8 items per color (hardcoded). Each item: `i32 + i64 + u8`.
/// color_count=0 means no dye catalog available.
///
/// Wire: `[0xC3][0x07][u16 color_count=0]`
fn build_dye_catalog_empty() -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_DYE_CATALOG);
    pkt.write_u16(0); // no colors
    pkt
}

/// Build a Sub 7 (dye catalog) with item data.
///
/// Client RE: `0xD866C2` — per color (up to 14):
///   per item (8 each): `[i32 item_id][i64 extra_data][u8 slot_type]`
///   slot_type: 3=time-based display, 4=model-based display.
///
/// Wire: `[0xC3][0x07][u16 color_count]{ { [i32 id][i64 extra][u8 slot] }×8 }×color_count`
#[cfg(test)]
fn build_dye_catalog(colors: &[[(i32, i64, u8); 8]]) -> Packet {
    let mut pkt = Packet::new(Opcode::WizCostume as u8);
    pkt.write_u8(COSTUME_SUB_DYE_CATALOG);
    let count = colors.len().min(14) as u16;
    pkt.write_u16(count);
    for color_items in colors.iter().take(14) {
        for &(item_id, extra, slot_type) in color_items {
            pkt.write_i32(item_id);
            // i64 as two i32 words (lo, hi)
            pkt.write_i32(extra as i32);
            pkt.write_i32((extra >> 32) as i32);
            pkt.write_u8(slot_type);
        }
    }
    pkt
}

// ── C2S Handler ──────────────────────────────────────────────────────────

/// Handle WIZ_COSTUME (0xC3) from the client.
///
/// C2S sub-opcodes:
/// - sub=1: Request costume status/list (no payload)
/// - sub=2: Confirm/apply costume (slot + item_id + param + attachments)
/// - sub=3: Select/equip specific (timestamp + slot_type + body_type + item_id + param + extra + color)
/// - sub=4: Request clear (no payload)
/// - sub=6: Use/apply attachment (timestamp + slot_type + body_type + item_id + param)
/// - sub=7: Request dye catalog (no payload)
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // v2615 routes CUISpecialAuction through the same 0xC3 opcode as the
    // costume panel. Akara's server-validated panel context must get first
    // refusal; otherwise sub 1/2/4/7 receive valid costume replies and the
    // altar opens with an empty list and non-working bid button.
    if super::native_events::try_handle_akara_altar(session, &pkt).await? {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);

    match sub {
        1 => handle_status_request(session).await,
        2 => handle_equip_request(session, &mut reader).await,
        3 => handle_select_request(session, &mut reader).await,
        4 => handle_clear_request(session).await,
        6 => handle_attach_request(session, &mut reader).await,
        7 => handle_dye_request(session).await,
        _ => {
            debug!(
                "[{}] WIZ_COSTUME unknown C2S sub={} ({}B)",
                session.addr(),
                sub,
                reader.remaining()
            );
            Ok(())
        }
    }
}

/// Handle C2S sub=1: Request costume status.
///
/// Sends costume data from session (loaded at gamestart from DB).
async fn handle_status_request(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] WIZ_COSTUME sub=1 status request", session.addr());

    let world = session.world();
    let sid = session.session_id();
    let costume_data = world
        .with_session(sid, |h| {
            if h.costume_loaded && h.costume_active_type > 0 {
                Some((
                    h.costume_active_type,
                    h.costume_item_param,
                    h.costume_scale_raw,
                    h.costume_color_index,
                    h.costume_expiry_time,
                ))
            } else {
                None
            }
        })
        .flatten();

    let pkt = match costume_data {
        Some((active_type, item_param, scale_raw, color_index, expiry_time)) => {
            // Compute countdown from absolute expiry
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let countdown = if expiry_time > now {
                (expiry_time - now) as i32
            } else if expiry_time == 0 {
                0 // No expiry set
            } else {
                -1 // Expired
            };
            build_status_active(active_type, countdown, item_param, scale_raw, color_index)
        }
        None => build_status_empty(),
    };
    session.send_packet(&pkt).await
}

/// Handle C2S sub=2: Confirm/apply costume.
///
/// Wire: `[u8 slot][i32 item_id][u16 item_param][u8 has_attachments][...]`
///
/// Stores costume data in session and persists to DB.
/// Client validates costume ownership; server stores what client reports.
async fn handle_equip_request(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let _slot = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_i32().unwrap_or(0);
    let item_param = reader.read_u16().unwrap_or(0);
    let _has_attachments = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] WIZ_COSTUME sub=2 equip: item={} param={}",
        session.addr(),
        item_id,
        item_param,
    );

    if item_id == 0 {
        return session
            .send_packet(&build_equip_result(RESULT_ERROR, 0))
            .await;
    }

    // Update session state
    let world = session.world();
    let sid = session.session_id();
    world.update_session(sid, |h| {
        h.costume_active_type = STATUS_EQUIPPED;
        h.costume_item_id = item_id;
        h.costume_item_param = item_param as i32;
    });

    // Fire-and-forget DB save
    save_costume_async(session);

    let result = build_equip_result(RESULT_SUCCESS, 0);
    session.send_packet(&result).await
}

/// Handle C2S sub=3: Select/equip specific costume.
///
/// Wire: `[i32 timestamp][u8 slot_type][u8 body_type][i32 item_id][u16 param][i32 extra][u8 color]`
///
/// Stores full costume state including color and scale in session + DB.
async fn handle_select_request(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let _timestamp = reader.read_i32().unwrap_or(0);
    let _slot_type = reader.read_u8().unwrap_or(0);
    let _body_type = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_i32().unwrap_or(0);
    let param = reader.read_u16().unwrap_or(0);
    let scale_raw = reader.read_i32().unwrap_or(0);
    let color = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] WIZ_COSTUME sub=3 select: item={} param={} scale={} color={}",
        session.addr(),
        item_id,
        param,
        scale_raw,
        color,
    );

    if item_id == 0 {
        return session
            .send_packet(&build_equip_result(RESULT_ERROR, 0))
            .await;
    }

    // Validate color index (0-13)
    let valid_color = color.min(13);

    // Update session state
    let world = session.world();
    let sid = session.session_id();
    world.update_session(sid, |h| {
        h.costume_active_type = STATUS_EQUIPPED;
        h.costume_item_id = item_id;
        h.costume_item_param = param as i32;
        h.costume_scale_raw = scale_raw;
        h.costume_color_index = valid_color;
    });

    // Fire-and-forget DB save
    save_costume_async(session);

    let result = build_equip_result(RESULT_SUCCESS, 0);
    session.send_packet(&result).await
}

/// Handle C2S sub=4: Request clear costume.
///
/// No payload. Clears any equipped costume and resets session state.
async fn handle_clear_request(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] WIZ_COSTUME sub=4 clear request", session.addr());

    // Reset costume session state
    let world = session.world();
    let sid = session.session_id();
    world.update_session(sid, |h| {
        h.costume_active_type = 0;
        h.costume_item_id = 0;
        h.costume_item_param = 0;
        h.costume_scale_raw = 0;
        h.costume_color_index = 0;
        h.costume_expiry_time = 0;
    });

    // Persist cleared state to DB
    save_costume_async(session);

    // Send clear attachments (param=0 → reset)
    let clear = build_clear_attachments(0);
    session.send_packet(&clear).await?;

    // Send empty status
    let status = build_status_empty();
    session.send_packet(&status).await
}

/// Handle C2S sub=6: Use/apply attachment.
///
/// Wire: `[i32 timestamp][u8 slot_type][u8 body_type][i32 item_id][u16 param]`
///
/// Accepts attachment and acknowledges success.
async fn handle_attach_request(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let _timestamp = reader.read_i32().unwrap_or(0);
    let _slot_type = reader.read_u8().unwrap_or(0);
    let _body_type = reader.read_u8().unwrap_or(0);
    let item_id = reader.read_i32().unwrap_or(0);
    let param = reader.read_u16().unwrap_or(0);

    debug!(
        "[{}] WIZ_COSTUME sub=6 attach: item={} param={}",
        session.addr(),
        item_id,
        param,
    );

    if item_id == 0 {
        return session
            .send_packet(&build_attach_result(RESULT_ERROR))
            .await;
    }

    // Attachments are a visual sub-feature of the costume.
    // Accept and acknowledge — the session costume state tracks the main costume.
    let result = build_attach_result(RESULT_SUCCESS);
    session.send_packet(&result).await
}

/// Fire-and-forget save costume data to DB.
///
/// Reads current costume state from session and persists via CostumeRepository.
fn save_costume_async(session: &ClientSession) {
    let Some(char_id) = session.character_id() else {
        return;
    };
    let name = char_id.to_string();
    let pool = session.pool().clone();
    let world = session.world();
    let sid = session.session_id();

    let data = world
        .with_session(sid, |h| {
            if h.costume_loaded {
                Some((
                    h.costume_active_type,
                    h.costume_item_id,
                    h.costume_item_param,
                    h.costume_scale_raw,
                    h.costume_color_index,
                    h.costume_expiry_time,
                ))
            } else {
                None
            }
        })
        .flatten();

    let Some((active_type, item_id, item_param, scale_raw, color_index, expiry_time)) = data else {
        return;
    };

    tokio::spawn(async move {
        let repo = CostumeRepository::new(&pool);
        if let Err(e) = repo
            .save(
                &name,
                active_type as i16,
                item_id,
                item_param,
                scale_raw,
                color_index as i16,
                expiry_time,
            )
            .await
        {
            tracing::warn!("Failed to save costume for {}: {}", name, e);
        }
    });
}

/// Handle C2S sub=7: Request dye color catalog.
///
/// No costume items — send empty dye catalog.
async fn handle_dye_request(session: &mut ClientSession) -> anyhow::Result<()> {
    debug!("[{}] WIZ_COSTUME sub=7 dye catalog request", session.addr());

    let pkt = build_dye_catalog_empty();
    session.send_packet(&pkt).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, PacketReader};

    #[test]
    fn test_costume_opcode_value() {
        assert_eq!(Opcode::WizCostume as u8, 0xC3);
        assert_eq!(Opcode::from_byte(0xC3), Some(Opcode::WizCostume));
    }

    // ── Sub 1: Status builders ────────────────────────────────────────

    #[test]
    fn test_build_status_empty() {
        let pkt = build_status_empty();
        assert_eq!(pkt.opcode, 0xC3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_STATUS)); // sub=1
        assert_eq!(r.read_u16(), Some(STATUS_EMPTY)); // type=0
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_status_empty_data_length() {
        // u8(sub) + u16(type) = 3 bytes
        assert_eq!(build_status_empty().data.len(), 3);
    }

    #[test]
    fn test_build_status_active() {
        let pkt = build_status_active(STATUS_EQUIPPED, 3600, 12345, 100, 5);
        assert_eq!(pkt.opcode, 0xC3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_STATUS));
        assert_eq!(r.read_u16(), Some(STATUS_EQUIPPED)); // type=2
        assert_eq!(r.read_i32(), Some(3600)); // expiry
        assert_eq!(r.read_i32(), Some(12345)); // item_param
        assert_eq!(r.read_u8(), Some(0)); // skipped byte
        assert_eq!(r.read_i32(), Some(100)); // scale_raw
        assert_eq!(r.read_u8(), Some(5)); // color_index (Cyan)
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_status_active_data_length() {
        // u8 + u16 + i32 + i32 + u8 + i32 + u8 = 1+2+4+4+1+4+1 = 17
        let pkt = build_status_active(1, 0, 0, 0, 0);
        assert_eq!(pkt.data.len(), 17);
    }

    #[test]
    fn test_build_status_color_clamp() {
        // color_index >= 14 should be clamped to 13
        let pkt = build_status_active(1, 0, 0, 0, 20);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8(); // sub
        r.read_u16(); // type
        r.read_i32(); // expiry
        r.read_i32(); // item_param
        r.read_u8(); // skip
        r.read_i32(); // scale
        assert_eq!(r.read_u8(), Some(13)); // clamped to max
    }

    // ── Sub 2: Equip result ───────────────────────────────────────────

    #[test]
    fn test_build_equip_result_success() {
        let pkt = build_equip_result(RESULT_SUCCESS, 0);
        assert_eq!(pkt.opcode, 0xC3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_EQUIP_RESULT));
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_u16(), Some(0)); // detail
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_equip_result_error() {
        let pkt = build_equip_result(RESULT_ERROR, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_EQUIP_RESULT));
        assert_eq!(r.read_u16(), Some(0)); // error
        assert_eq!(r.read_u16(), Some(0));
    }

    #[test]
    fn test_build_equip_result_negative() {
        // Negative result codes use i16 → u16 encoding
        let pkt = build_equip_result(-3, 0);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        let code_raw = r.read_u16().unwrap();
        assert_eq!(code_raw as i16, -3);
    }

    // ── Sub 4: Clear attachments ──────────────────────────────────────

    #[test]
    fn test_build_clear_attachments() {
        let pkt = build_clear_attachments(42);
        assert_eq!(pkt.opcode, 0xC3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_CLEAR));
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sub 6: Attachment result ──────────────────────────────────────

    #[test]
    fn test_build_attach_result_success() {
        let pkt = build_attach_result(RESULT_SUCCESS);
        assert_eq!(pkt.opcode, 0xC3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_ATTACH_RESULT));
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_attach_result_error() {
        let pkt = build_attach_result(RESULT_ERROR);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_ATTACH_RESULT));
        assert_eq!(r.read_u16(), Some(0)); // error
    }

    // ── Sub 7: Dye catalog ───────────────────────────────────────────

    #[test]
    fn test_build_dye_catalog_empty() {
        let pkt = build_dye_catalog_empty();
        assert_eq!(pkt.opcode, 0xC3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_DYE_CATALOG));
        assert_eq!(r.read_u16(), Some(0)); // no colors
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_dye_catalog_one_color() {
        // 1 color with 8 empty items
        let items: [(i32, i64, u8); 8] = [(0, 0, 0); 8];
        let pkt = build_dye_catalog(&[items]);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COSTUME_SUB_DYE_CATALOG));
        assert_eq!(r.read_u16(), Some(1)); // 1 color
                                           // 8 items × (i32 + i32_lo + i32_hi + u8) = 8 × 13 = 104 bytes
        for _ in 0..8 {
            assert_eq!(r.read_i32(), Some(0)); // item_id
            assert_eq!(r.read_i32(), Some(0)); // extra_lo
            assert_eq!(r.read_i32(), Some(0)); // extra_hi
            assert_eq!(r.read_u8(), Some(0)); // slot_type
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_dye_catalog_with_item() {
        let mut items = [(0i32, 0i64, 0u8); 8];
        items[0] = (910252000, 1_700_000_000i64, 3); // time-based display

        let pkt = build_dye_catalog(&[items]);
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8(); // sub
        r.read_u16(); // count
                      // First item
        assert_eq!(r.read_i32(), Some(910252000)); // item_id
        let lo = r.read_i32().unwrap();
        let hi = r.read_i32().unwrap();
        let extra = (lo as i64) | ((hi as i64) << 32);
        assert_eq!(extra, 1_700_000_000);
        assert_eq!(r.read_u8(), Some(3)); // slot_type = time-based
    }

    // ── C2S format tests ─────────────────────────────────────────────

    #[test]
    fn test_c2s_status_request() {
        // Client sends: [0xC3][0x01]
        let mut pkt = Packet::new(Opcode::WizCostume as u8);
        pkt.write_u8(1);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 1);
    }

    #[test]
    fn test_c2s_equip_request() {
        // [0xC3][0x02][u8 slot][i32 item_id][u16 param][u8 attachments]
        let mut pkt = Packet::new(Opcode::WizCostume as u8);
        pkt.write_u8(2);
        pkt.write_u8(1); // slot
        pkt.write_i32(910252000); // item_id
        pkt.write_u16(100); // param
        pkt.write_u8(0); // no attachments

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_i32(), Some(910252000));
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_c2s_select_request() {
        // [0xC3][0x03][i32 ts][u8 slot_type][u8 body_type][i32 item_id][u16 param][i32 extra][u8 color]
        let mut pkt = Packet::new(Opcode::WizCostume as u8);
        pkt.write_u8(3);
        pkt.write_i32(1000); // timestamp
        pkt.write_u8(15); // slot_type (maps to slot 15-14=1)
        pkt.write_u8(2); // body_type
        pkt.write_i32(700085000); // item_id
        pkt.write_u16(50); // param
        pkt.write_i32(0); // extra
        pkt.write_u8(8); // color = Gold

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_i32(), Some(1000));
        assert_eq!(r.read_u8(), Some(15));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_i32(), Some(700085000));
        assert_eq!(r.read_u16(), Some(50));
        assert_eq!(r.read_i32(), Some(0));
        assert_eq!(r.read_u8(), Some(8));
    }

    #[test]
    fn test_c2s_clear_request() {
        // [0xC3][0x04] — no payload
        let mut pkt = Packet::new(Opcode::WizCostume as u8);
        pkt.write_u8(4);
        assert_eq!(pkt.data.len(), 1);
    }

    #[test]
    fn test_c2s_attach_request() {
        // [0xC3][0x06][i32 ts][u8 slot_type][u8 body_type][i32 item_id][u16 param]
        let mut pkt = Packet::new(Opcode::WizCostume as u8);
        pkt.write_u8(6);
        pkt.write_i32(2000);
        pkt.write_u8(16); // slot_type
        pkt.write_u8(1); // body_type
        pkt.write_i32(811095000); // item_id
        pkt.write_u16(75);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(6));
        assert_eq!(r.read_i32(), Some(2000));
        assert_eq!(r.read_u8(), Some(16));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_i32(), Some(811095000));
        assert_eq!(r.read_u16(), Some(75));
    }

    #[test]
    fn test_c2s_dye_request() {
        // [0xC3][0x07] — no payload
        let mut pkt = Packet::new(Opcode::WizCostume as u8);
        pkt.write_u8(7);
        assert_eq!(pkt.data.len(), 1);
    }

    // ── Dye color table ──────────────────────────────────────────────

    #[test]
    fn test_dye_color_count() {
        assert_eq!(DYE_COLORS.len(), 14);
    }

    #[test]
    fn test_dye_color_values() {
        assert_eq!(DYE_COLORS[0], 0xFFF0F8FF); // AliceBlue
        assert_eq!(DYE_COLORS[4], 0xFFDC143C); // Crimson
        assert_eq!(DYE_COLORS[8], 0xFFFFD700); // Gold
        assert_eq!(DYE_COLORS[13], 0xFFFFFFFF); // White
    }

    #[test]
    fn test_dye_colors_all_opaque() {
        // All colors should have alpha = 0xFF
        for (i, &color) in DYE_COLORS.iter().enumerate() {
            assert_eq!(
                color >> 24,
                0xFF,
                "Color {} not fully opaque: 0x{:08X}",
                i,
                color
            );
        }
    }

    // ── Sub-type constants ───────────────────────────────────────────

    #[test]
    fn test_sub_type_constants() {
        assert_eq!(COSTUME_SUB_STATUS, 1);
        assert_eq!(COSTUME_SUB_EQUIP_RESULT, 2);
        assert_eq!(COSTUME_SUB_MODEL_CHANGE, 3);
        assert_eq!(COSTUME_SUB_CLEAR, 4);
        assert_eq!(COSTUME_SUB_STORE, 5);
        assert_eq!(COSTUME_SUB_ATTACH_RESULT, 6);
        assert_eq!(COSTUME_SUB_DYE_CATALOG, 7);
    }
}
