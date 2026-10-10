//! Native event panels used by the verified v2615 client.
//!
//! Opcode families:
//! - `0x9C`: Event hub (`0xF0`) and Roulette (`6=open, 7=spin, 8=reveal, 9=history`)
//! - `0xCC`: Jigsaw selector `1`, Coin selector `2`
//! - `0xCF`: Knight Marble (`1=open, 2=roll, 3..5=panel actions`)

use chrono::{DateTime, Datelike, Utc};
use ko_db::models::native_events::{NativeJigsawState, NativeRoulettePending};
use ko_db::repositories::native_events::NativeEventsRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use rand::Rng;
use tracing::{debug, info, warn};

use crate::handler::knight_cash;
use crate::npc_type_constants::MAX_NPC_RANGE;
use crate::session::{ClientSession, SessionState};

const ROULETTE_FREE_TYPE: i32 = 1;
const ROULETTE_KC_TYPE: i32 = 2;
const ROULETTE_KC_COST: u32 = 350;

const EVENT_HUB_SUB: u8 = 0xF0;
const EVENT_HUB_COIN: u8 = 0;
const EVENT_HUB_ATTENDANCE: u8 = 1;
// The hub advertises Attendance as list id 1, but the v2615 client sends
// selector 4 when that row is clicked.
const EVENT_HUB_ATTENDANCE_SELECT: u8 = 4;
const EVENT_HUB_ROULETTE: u8 = 2;
const EVENT_HUB_JIGSAW: u8 = 3;
const EVENT_HUB_MARBLE: u8 = 5;
// v2615's Moradon Akara instance uses proto 31774 with picture 30001. It
// must remain a nation-3 NPC; nation/group 0 makes the client classify the
// statue as an attack target and suppress WIZ_NPC_EVENT on right-click.
const BOARD_NPC_PROTO_ID: u16 = 31774;
const BOARD_REWARD_ITEM_ID: u32 = 811_084_000;
const BOARD_OPEN_SUB: u8 = 2;
const BOARD_REPLY_SUB: u8 = 3;
// WIZ_SELECT_MSG action flags verified in v2615 sub_7F0770:
// 0x3A -> CUISpecialAuction (Akara Altar), 0x44 -> CUIEventPostUp board 0.
const AKARA_ALTAR_SELECT_FLAG: u8 = 0x3A;
const AKARA_POST_UP_SELECT_FLAG: u8 = 0x44;
const AKARA_AUCTION_LIST_SUB: u8 = 1;
const AKARA_AUCTION_BID_SUB: u8 = 2;
const AKARA_AUCTION_ALT_LIST_SUB: u8 = 4;
const AKARA_AUCTION_CATALOG_SUB: u8 = 7;
pub(crate) const AKARA_ALTAR_EVENT: i32 = 9_317_741;
pub(crate) const AKARA_POST_UP_EVENT: i32 = 9_317_742;
const AKARA_MENU_HEADER_TEXT: i32 = 45_136;
const AKARA_ALTAR_MENU_TEXT: i32 = 45_202;
const AKARA_POST_UP_MENU_TEXT: i32 = 45_236;

// CUIAttendanceCheck does not treat the i32 in each calendar entry as an item
// number.  The v2615 client searches its 28 in-memory slot records by this
// value, then resolves the actual reward through Attendance.tbl.  Daily rows
// are keyed 1..=25 and cumulative rows are keyed 101..=103.
fn attendance_daily_slot_key(day: usize) -> i32 {
    day as i32 + 1
}

fn attendance_cumulative_slot_key(index: usize) -> i32 {
    index as i32 + 101
}

fn character_name(session: &ClientSession) -> Option<String> {
    session
        .world()
        .get_character_info(session.session_id())
        .map(|c| c.name.clone())
        .filter(|v| !v.is_empty())
}

fn native_type(client_type: i32) -> i16 {
    if client_type == ROULETTE_KC_TYPE {
        8
    } else if client_type == ROULETTE_FREE_TYPE {
        7
    } else {
        7
    }
}

fn event_unavailable(opcode: u8, selector: u8) -> Packet {
    let mut pkt = Packet::new(opcode);
    pkt.write_u8(selector);
    pkt.write_i32(0);
    pkt
}

/// Seconds remaining in the current monthly attendance period.
///
/// The v2615 client uses this value for the panel countdown and treats zero as
/// an expired/unavailable calendar. The database progress resets by calendar
/// month, so the wire value must expire at the same boundary.
fn attendance_seconds_remaining(now: DateTime<Utc>) -> i32 {
    let (year, month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };

    let next_period = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date_time| date_time.and_utc().timestamp())
        .unwrap_or_else(|| now.timestamp() + 1);

    (next_period - now.timestamp()).clamp(1, i32::MAX as i64) as i32
}

pub async fn handle_roulette(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    let pool = session.pool().clone();
    let repo = NativeEventsRepository::new(&pool);
    if sub == EVENT_HUB_SUB {
        return native_event_hub_open(session, &repo).await;
    }
    // CUIEventPostUp is first opened by WIZ_SELECT_MSG flag 0x44. The client
    // then requests board 0 with exactly [sub=2][i32 board_id]. A one-byte
    // sub=2 remains the event hub's Roulette selector, so length separates
    // the two native v2615 contracts without ambiguity.
    if sub == BOARD_OPEN_SUB && pkt.data.len() == 5 {
        let board_id = reader.read_i32().unwrap_or(0);
        if validate_akara_context(session) {
            return open_board_from_npc(session, board_id).await;
        }
        warn!(
            "[{}] ignored Event Post-Up request outside Akara context: board_id={board_id}",
            session.addr()
        );
        return Ok(());
    }
    // The unpacked UIEventPostUp client writes exactly:
    // [sub=3][i32 board_id][u8 text_len][text]. A one-byte sub=3 packet is
    // still the star hub's Jigsaw selector, so length separates them safely.
    if sub == BOARD_REPLY_SUB && pkt.data.len() > 1 {
        return board_reply(session, &repo, &mut reader).await;
    }
    if matches!(
        sub,
        EVENT_HUB_COIN
            | EVENT_HUB_ATTENDANCE
            | EVENT_HUB_ATTENDANCE_SELECT
            | EVENT_HUB_ROULETTE
            | EVENT_HUB_JIGSAW
            | EVENT_HUB_MARBLE
    ) {
        return native_event_hub_select(session, &repo, sub).await;
    }
    if !repo.is_active("roulette").await.unwrap_or(false) {
        let response = event_unavailable(Opcode::WizContinousPacketData as u8, sub);
        session.send_packet(&response).await?;
        return Ok(());
    }
    let Some(name) = character_name(session) else {
        return Ok(());
    };
    match sub {
        6 => roulette_open(session, &repo).await,
        7 => roulette_spin(session, &repo, &name, reader.read_i32().unwrap_or(1)).await,
        8 => roulette_reveal(session, &repo, &name, reader.read_i32().unwrap_or(1)).await,
        9 => roulette_history(session, &repo, &name, reader.read_i32().unwrap_or(1)).await,
        _ => {
            debug!("[{}] native roulette unknown sub={sub}", session.addr());
            Ok(())
        }
    }
}

/// Reply to the star-button confirmation request.
///
/// The v2615 client treats `0xF0` as signed `-16` and dispatches it to
/// `CUIEventWebSelect::ReceiveMessage` (`0xAEB8C0`).  The verified wire format
/// after the subcommand is `i16 count`, followed by `u8 event_id, u8 enabled`
/// pairs. Disabled entries must be omitted: the client only adds pairs whose
/// enabled byte is non-zero to its selection list.
async fn native_event_hub_open(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
) -> anyhow::Result<()> {
    session.world().update_session(session.session_id(), |h| {
        h.native_event_hub_armed = true;
    });
    // The client renders entries in the order supplied by the server.
    let mut active = vec![(EVENT_HUB_COIN, 1u8)]; // proxy entry 0: Daily Quest
    let candidates = [
        (EVENT_HUB_ATTENDANCE, "attendance"),
        (EVENT_HUB_ROULETTE, "roulette"),
        (EVENT_HUB_JIGSAW, "jigsaw"),
        (EVENT_HUB_MARBLE, "marble"),
    ];
    for (event_id, event_key) in candidates {
        if repo.is_active(event_key).await.unwrap_or(false) {
            active.push((event_id, 1u8));
        }
    }

    let out = native_event_hub_packet(&active);
    session.send_packet(&out).await?;
    info!(
        "[{}] native event hub opened: active_ids={:?}",
        session.addr(),
        active.iter().map(|(id, _)| *id).collect::<Vec<_>>()
    );
    Ok(())
}

/// Dispatch a selection made in the star-button event list to the native
/// panel handler that already owns that event's wire contract.
async fn native_event_hub_select(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    event_id: u8,
) -> anyhow::Result<()> {
    let armed = session
        .world()
        .with_session(session.session_id(), |h| h.native_event_hub_armed)
        .unwrap_or(false);
    if !armed {
        debug!(
            "[{}] ignored stale native hub selector id={event_id}",
            session.addr()
        );
        return Ok(());
    }
    session.world().update_session(session.session_id(), |h| {
        h.native_event_hub_armed = false;
    });
    if event_id == EVENT_HUB_COIN {
        super::quest::send_quest_data(session).await?;
        super::daily_quest::daily_quest_send_list(session.world(), session.session_id());
        return super::daily_quest::open_daily_quest_manager(session, 0).await;
    }
    let event_key = match event_id {
        EVENT_HUB_ATTENDANCE | EVENT_HUB_ATTENDANCE_SELECT => "attendance",
        EVENT_HUB_ROULETTE => "roulette",
        EVENT_HUB_JIGSAW => "jigsaw",
        EVENT_HUB_COIN => "coin",
        EVENT_HUB_MARBLE => "marble",
        _ => return Ok(()),
    };

    if !repo.is_active(event_key).await.unwrap_or(false) {
        let response = event_unavailable(Opcode::WizContinousPacketData as u8, event_id);
        session.send_packet(&response).await?;
        return Ok(());
    }

    let result = match event_id {
        EVENT_HUB_ATTENDANCE | EVENT_HUB_ATTENDANCE_SELECT => attendance_open(session).await,
        EVENT_HUB_ROULETTE => roulette_open(session, repo).await,
        EVENT_HUB_JIGSAW => {
            let Some(name) = character_name(session) else {
                return Ok(());
            };
            jigsaw_open(session, repo, &name).await
        }
        EVENT_HUB_COIN => {
            let Some(name) = character_name(session) else {
                return Ok(());
            };
            coin_open(session, repo, &name).await
        }
        EVENT_HUB_MARBLE => {
            let Some(name) = character_name(session) else {
                return Ok(());
            };
            marble_open(session, repo, &name).await
        }
        _ => Ok(()),
    };

    info!(
        "[{}] native event hub selected: id={} key={}",
        session.addr(),
        event_id,
        event_key
    );
    result
}

/// Open the v2615 Event Post-Up board after the Akara menu selection has
/// validated NPC existence, zone, distance and stored event_sid=31774.
pub async fn open_board_from_npc(session: &mut ClientSession, board_id: i32) -> anyhow::Result<()> {
    let Some(name) = character_name(session) else {
        return Ok(());
    };
    let pool = session.pool().clone();
    let repo = NativeEventsRepository::new(&pool);
    let history = repo.board_claim_history(&name).await.unwrap_or_else(|e| {
        warn!("[{}] native board history DB error: {e}", session.addr());
        Vec::new()
    });
    let out = board_open_packet(board_id, &history);
    session.send_packet(&out).await?;
    info!(
        "[{}] native board opened from npc proto={} board_id={} history={}",
        session.addr(),
        BOARD_NPC_PROTO_ID,
        board_id,
        history.len()
    );
    Ok(())
}

/// Open Akara's selection menu from a real NPC interaction packet
/// (WIZ_CLIENT_EVENT/WIZ_NPC_EVENT). WIZ_TARGET_HP is deliberately not an
/// interaction trigger: the client emits it for ordinary left-click target
/// selection too.
pub async fn try_open_akara_menu_from_target(
    session: &mut ClientSession,
    target_nid: u32,
) -> anyhow::Result<bool> {
    let world = session.world().clone();
    let sid = session.session_id();

    let Some(npc) = world.get_npc_instance(target_nid) else {
        return Ok(false);
    };
    if npc.proto_id != BOARD_NPC_PROTO_ID || world.is_npc_dead(target_nid) {
        return Ok(false);
    }

    let Some(pos) = world.get_position(sid) else {
        return Ok(false);
    };
    if npc.zone_id != pos.zone_id {
        return Ok(false);
    }

    let dx = pos.x - npc.x;
    let dz = pos.z - npc.z;
    if (dx * dx + dz * dz).sqrt() > MAX_NPC_RANGE {
        return Ok(false);
    }

    let menu_already_open = world
        .with_session(sid, |state| {
            state.event_nid == target_nid as i16
                && state.event_sid == BOARD_NPC_PROTO_ID as i16
                && state.select_msg_events[0] == AKARA_ALTAR_EVENT
                && state.select_msg_events[1] == AKARA_POST_UP_EVENT
        })
        .unwrap_or(false);
    if menu_already_open {
        return Ok(true);
    }

    world.update_session(sid, |state| {
        state.event_nid = target_nid as i16;
        state.event_sid = BOARD_NPC_PROTO_ID as i16;
        state.akara_altar_armed = false;
    });

    let mut button_texts = [-1; 12];
    button_texts[0] = AKARA_ALTAR_MENU_TEXT;
    button_texts[1] = AKARA_POST_UP_MENU_TEXT;
    let mut button_events = [-1; 12];
    button_events[0] = AKARA_ALTAR_EVENT;
    button_events[1] = AKARA_POST_UP_EVENT;
    super::select_msg::send_select_msg(
        &world,
        sid,
        3,
        -1,
        AKARA_MENU_HEADER_TEXT,
        &button_texts,
        &button_events,
        "31774_Akara.lua",
    );
    info!(
        "[{}] Akara menu opened: nid={} proto={}",
        session.addr(),
        target_nid,
        BOARD_NPC_PROTO_ID
    );
    Ok(true)
}

pub async fn handle_akara_menu_event(
    session: &mut ClientSession,
    event: i32,
) -> anyhow::Result<bool> {
    if !validate_akara_context(session) {
        warn!(
            "[{}] ignored stale Akara menu event={event}",
            session.addr()
        );
        return Ok(true);
    }

    match event {
        AKARA_ALTAR_EVENT => {
            // v2615 sub_7F0770 case 0x3A calls sub_DD0D00, which initializes
            // and shows CUISpecialAuction in Akara Altar mode. 0x5B belongs to
            // the UI's later button-action switch and cannot open the panel.
            let world = session.world().clone();
            let sid = session.session_id();
            let empty = [-1; 12];
            super::select_msg::send_select_msg(
                &world,
                sid,
                AKARA_ALTAR_SELECT_FLAG,
                -1,
                -1,
                &empty,
                &empty,
                "31774_Akara.lua",
            );
            session
                .world()
                .update_session(session.session_id(), |state| {
                    state.akara_altar_armed = true;
                });
            info!(
                "[{}] native Akara Altar UIF dispatched: select_flag=0x{:02X}",
                session.addr(),
                AKARA_ALTAR_SELECT_FLAG
            );
        }
        AKARA_POST_UP_EVENT => {
            // v2615 sub_7F0770 case 0x44 shows CUIEventPostUp for board 0 and
            // makes the client send [0x9C][2][i32 0]. The board payload must
            // be returned only after that request, while the UIF is alive.
            let world = session.world().clone();
            let sid = session.session_id();
            let empty = [-1; 12];
            super::select_msg::send_select_msg(
                &world,
                sid,
                AKARA_POST_UP_SELECT_FLAG,
                -1,
                -1,
                &empty,
                &empty,
                "31774_Akara.lua",
            );
            session
                .world()
                .update_session(session.session_id(), |state| {
                    state.akara_altar_armed = false;
                });
            info!(
                "[{}] native I Love Knight Online UIF dispatched: select_flag=0x{:02X}",
                session.addr(),
                AKARA_POST_UP_SELECT_FLAG
            );
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Handle CUISpecialAuction's 0xC3 traffic while the server-validated Akara
/// panel is active. Returns false for ordinary costume packets.
pub async fn try_handle_akara_altar(
    session: &mut ClientSession,
    pkt: &Packet,
) -> anyhow::Result<bool> {
    let armed = session
        .world()
        .with_session(session.session_id(), |h| h.akara_altar_armed)
        .unwrap_or(false);
    if !armed {
        return Ok(false);
    }
    if !validate_akara_context(session) {
        session.world().update_session(session.session_id(), |h| {
            h.akara_altar_armed = false;
        });
        return Ok(false);
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    match sub {
        AKARA_AUCTION_LIST_SUB | AKARA_AUCTION_ALT_LIST_SUB => {
            let pool = session.pool().clone();
            let repo = NativeEventsRepository::new(&pool);
            let rows = repo.akara_auctions().await.unwrap_or_else(|e| {
                warn!("[{}] Akara auction list DB error: {e}", session.addr());
                Vec::new()
            });
            // The opening request is sub=1, whose response contract is a
            // schedule-page delta backed by Special_Auction.tbl. Our altar
            // catalogue is server-owned, so return the dynamic sub=4 list
            // contract that accepts arbitrary valid item IDs and opens mode 2.
            let response_sub = AKARA_AUCTION_ALT_LIST_SUB;
            let out = akara_auction_list_packet(response_sub, &rows);
            session.send_packet(&out).await?;
            info!(
                "[{}] Akara auction list: sub={} rows={}",
                session.addr(),
                response_sub,
                rows.len()
            );
        }
        AKARA_AUCTION_BID_SUB => {
            handle_akara_bid(session, &mut reader).await?;
        }
        AKARA_AUCTION_CATALOG_SUB => {
            // sub=7 is the third altar catalogue. A zero count is a complete
            // v2615 response and, unlike the costume handler, stays inside
            // CUISpecialAuction.
            let mut out = Packet::new(Opcode::WizCostume as u8);
            out.write_u8(AKARA_AUCTION_CATALOG_SUB);
            out.write_u16(0);
            session.send_packet(&out).await?;
        }
        _ => {
            debug!(
                "[{}] Akara auction unknown sub={} bytes={}",
                session.addr(),
                sub,
                pkt.data.len()
            );
        }
    }
    Ok(true)
}

async fn handle_akara_bid(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    // v2615 sub_DCD290:
    // [u8 slot][i32 item][u16 ext][u8 digits]
    // digits * [u32 integrity_tag][u8 digit]
    // [i64 displayed_bid][i64 offered_bid].
    let slot = reader.read_u8().unwrap_or(u8::MAX);
    let item_id = reader.read_i32().unwrap_or(0);
    let _item_ext = reader.read_u16().unwrap_or(0);
    let digit_count = reader.read_u8().unwrap_or(0).min(10);
    for _ in 0..digit_count {
        let _integrity_tag = reader.read_u32().unwrap_or(0);
        let _digit = reader.read_u8().unwrap_or(0);
    }
    let displayed_bid = reader.read_i64().unwrap_or(0);
    let offered_bid = reader.read_i64().unwrap_or(0);

    let mut result = -2i16;
    let Some(name) = character_name(session) else {
        return Ok(());
    };
    if slot < 16 && item_id > 0 && offered_bid > displayed_bid && offered_bid <= u32::MAX as i64 {
        let has_gold = session
            .world()
            .get_character_info(session.session_id())
            .is_some_and(|ch| ch.gold >= offered_bid as u32);
        if has_gold {
            let pool = session.pool().clone();
            let repo = NativeEventsRepository::new(&pool);
            if repo
                .place_akara_bid(slot as i16, item_id, &name, offered_bid)
                .await?
                .is_some()
            {
                if session
                    .world()
                    .gold_lose(session.session_id(), offered_bid as u32)
                {
                    result = 1;
                } else {
                    result = -3;
                }
            } else {
                result = -3;
            }
        } else {
            result = -4;
        }
    }

    let mut out = Packet::new(Opcode::WizCostume as u8);
    out.write_u8(AKARA_AUCTION_BID_SUB);
    out.write_i16(result);
    out.write_i16(0);
    session.send_packet(&out).await?;
    info!(
        "[{}] Akara auction bid: character={} slot={} item={} displayed={} offered={} result={}",
        session.addr(),
        name,
        slot,
        item_id,
        displayed_bid,
        offered_bid,
        result
    );
    Ok(())
}

fn akara_auction_list_packet(sub: u8, rows: &[(i16, i32, i16, i64, i64, i32, i32)]) -> Packet {
    let mut out = Packet::new(Opcode::WizCostume as u8);
    out.write_u8(sub);
    out.write_u16(1); // load result
    out.write_u8(rows.len().min(16) as u8);
    for &(slot, item_id, item_ext, current_bid, _increment, ends_at, bid_count) in
        rows.iter().take(16)
    {
        // Exact 26-byte row consumed by v2615 sub_DCDE70.
        out.write_i32(slot as i32 + 1); // auction row id
        out.write_u8((bid_count > 0) as u8);
        out.write_u8(slot as u8);
        out.write_i32(item_id);
        out.write_u16(item_ext as u16);
        out.write_i32(ends_at);
        out.write_u8(0);
        out.write_i64(current_bid);
        out.write_u8(1); // active altar item
    }
    out
}

fn validate_akara_context(session: &ClientSession) -> bool {
    let world = session.world();
    let sid = session.session_id();
    let Some((event_nid, event_sid)) =
        world.with_session(sid, |state| (state.event_nid, state.event_sid))
    else {
        return false;
    };
    if event_nid <= 0 || event_sid != BOARD_NPC_PROTO_ID as i16 {
        return false;
    }
    let Some(npc) = world.get_npc_instance(event_nid as u32) else {
        return false;
    };
    let Some(pos) = world.get_position(sid) else {
        return false;
    };
    if npc.proto_id != BOARD_NPC_PROTO_ID
        || npc.zone_id != pos.zone_id
        || world.is_npc_dead(event_nid as u32)
    {
        return false;
    }
    let dx = pos.x - npc.x;
    let dz = pos.z - npc.z;
    (dx * dx + dz * dz).sqrt() <= MAX_NPC_RANGE
}

async fn board_reply(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let board_id = reader.read_i32().unwrap_or(0);
    let answer = reader.read_sbyte_string().unwrap_or_default();
    let normalized = answer.split_whitespace().collect::<Vec<_>>().join(" ");
    let world = session.world();
    let sid = session.session_id();
    let context = world.with_session(sid, |h| (h.event_nid, h.event_sid));
    let npc_ok = context.is_some_and(|(event_nid, event_sid)| {
        if event_nid <= 0 || event_sid != BOARD_NPC_PROTO_ID as i16 {
            return false;
        }
        let Some(npc) = world.get_npc_instance(event_nid as u32) else {
            return false;
        };
        let Some(pos) = world.get_position(sid) else {
            return false;
        };
        if npc.proto_id != BOARD_NPC_PROTO_ID
            || npc.zone_id != pos.zone_id
            || world.is_npc_dead(event_nid as u32)
        {
            return false;
        }
        let dx = pos.x - npc.x;
        let dz = pos.z - npc.z;
        (dx * dx + dz * dz).sqrt() <= MAX_NPC_RANGE
    });
    let correct = normalized.eq_ignore_ascii_case("I Love Knight Online");
    let Some(name) = character_name(session) else {
        return Ok(());
    };

    let mut result = 0i32;
    if npc_ok && correct {
        match repo.reserve_board_claim(&name).await? {
            Some(claim_id) => {
                if session
                    .world()
                    .give_item(session.session_id(), BOARD_REWARD_ITEM_ID, 1)
                {
                    result = 1;
                } else {
                    repo.cancel_board_claim(claim_id, &name).await?;
                    result = 5; // inventory full
                }
            }
            None => result = 101, // weekly limit
        }
    }

    let history = if result == 1 {
        repo.board_claim_history(&name).await.unwrap_or_else(|e| {
            warn!("[{}] native board refresh DB error: {e}", session.addr());
            Vec::new()
        })
    } else {
        Vec::new()
    };
    let out = board_reply_packet(board_id, result, &history);
    session.send_packet(&out).await?;
    info!(
        "[{}] native board reply: character={} board_id={} npc_ok={} correct={} result={}",
        session.addr(),
        name,
        board_id,
        npc_ok,
        correct,
        result
    );
    Ok(())
}

fn board_open_packet(board_id: i32, history: &[(String, i32, i32)]) -> Packet {
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    // v2615 CUIEventPostUp dispatches sub=2 to sub_ADAC20 (load/open) and
    // sub=3 to sub_ADAF40 (reply result). The open body begins with the board
    // id and result, followed by four i32 header fields and an i16 row count.
    // A zero-row payload is a complete, valid panel-open response.
    out.write_u8(BOARD_OPEN_SUB);
    out.write_i32(board_id);
    // sub_ADAC20 calls the body parser only when this value is non-zero.
    out.write_i32(1); // load result: success
    write_board_body(&mut out, history);
    out
}

fn write_board_body(out: &mut Packet, history: &[(String, i32, i32)]) {
    out.write_i32(0); // event/start header
    out.write_i32(0); // event/end header
    out.write_i32(0); // display/start timestamp
    out.write_i32(0); // display/end timestamp
    out.write_i16(history.len().min(100) as i16);
    // v2615 sub_AD9FB0 consumes exactly three SByte strings and one Unix
    // timestamp for every history row.
    for (character, item_id, claimed_at) in history.iter().take(100) {
        out.write_sbyte_string(character);
        out.write_sbyte_string("I Love Knight Online");
        out.write_sbyte_string(&item_id.to_string());
        out.write_i32(*claimed_at);
    }
}

fn board_reply_packet(board_id: i32, result: i32, history: &[(String, i32, i32)]) -> Packet {
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(BOARD_REPLY_SUB);
    out.write_i32(board_id);
    out.write_i32(result);
    // sub_ADAF40 dispatches result=1 into the same AD9FB0 body parser used by
    // the open response. Appending the refreshed rows makes the successful
    // reply update immediately instead of clearing the panel state.
    if result == 1 {
        write_board_body(&mut out, history);
    }
    out
}

/// Open the v2615 native Attendance panel.
///
/// The unpacked client dispatches this UI through WIZ_CONTINOUS_PACKET_DATA
/// (0x9C), not WIZ_ATTENDANCE (0xB7):
///
/// `[0x9C][outer=4][i32 error][i32 result][calendar state...]`
///
/// `outer=4` selects `CUIAttendanceCheck` at UI-manager offset `+0x66C`;
/// `error=0, result=1` loads the complete calendar and shows it.
async fn attendance_open(session: &mut ClientSession) -> anyhow::Result<()> {
    const TOTAL_DAYS: usize = 25;

    // The native v2615 panel does not send the legacy WIZ_ATTENDANCE claim
    // request. Claim today's next sequential reward on open; the shared claim
    // routine enforces one reward per calendar day and inventory capacity.
    let granted = crate::handler::attendance::claim_for_native_open(session).await?;

    let pool = session.pool().clone();
    let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);
    let rewards = repo.load_all().await.unwrap_or_else(|e| {
        warn!(
            "[{}] native attendance load_all DB error: {e}",
            session.addr()
        );
        Vec::new()
    });

    let Some(name) = character_name(session) else {
        return Ok(());
    };
    let progress = repo.load_user_progress(&name).await.unwrap_or_else(|e| {
        warn!(
            "[{}] native attendance load_user_progress DB error: {e}",
            session.addr()
        );
        Vec::new()
    });
    let cumulative = repo.load_cumulative().await.unwrap_or_else(|e| {
        warn!(
            "[{}] native attendance load_cumulative DB error: {e}",
            session.addr()
        );
        None
    });

    let mut claimed = [false; TOTAL_DAYS];
    for row in &progress {
        let index = row.day_index as usize;
        if index < TOTAL_DAYS {
            claimed[index] = row.claimed;
        }
    }

    let next_claimable = claimed.iter().position(|value| !*value);

    let now = chrono::Utc::now();
    let seconds_remaining = attendance_seconds_remaining(now);

    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(EVENT_HUB_ATTENDANCE_SELECT); // outer selector: CUIAttendanceCheck
    out.write_i32(0); // error
    out.write_i32(1); // result: load/show panel
    out.write_i32(now.timestamp().clamp(0, i32::MAX as i64) as i32);
    out.write_i16(TOTAL_DAYS as i16);

    for day in 0..TOTAL_DAYS {
        let has_reward = rewards
            .iter()
            .find(|row| row.day_index as usize == day)
            .is_some_and(|row| row.item_id > 0);
        out.write_i32(if has_reward {
            attendance_daily_slot_key(day)
        } else {
            0
        });
        // Native CUIAttendanceCheck hides the reward group when state is 0.
        // State 3 keeps a future/locked reward visible but inactive.
        let state = if claimed[day] {
            1
        } else if Some(day) == next_claimable {
            3
        } else {
            3
        };
        out.write_u8(state);
    }

    let cumulative_items = cumulative
        .map(|row| {
            [
                row.item1.unwrap_or(0),
                row.item2.unwrap_or(0),
                row.item3.unwrap_or(0),
            ]
        })
        .unwrap_or([0; 3]);
    out.write_i16(cumulative_items.len() as i16);
    for (index, item_id) in cumulative_items.into_iter().enumerate() {
        out.write_i32(if item_id > 0 {
            attendance_cumulative_slot_key(index)
        } else {
            0
        });
        let milestone = [7usize, 14, 21][index];
        let completed = claimed.iter().take(milestone).all(|value| *value);
        let state = if completed {
            1
        } else if claimed.iter().filter(|value| **value).count() + 1 == milestone {
            3
        } else {
            3
        };
        out.write_u8(state);
    }
    out.write_i32(seconds_remaining);

    session.send_packet(&out).await?;
    info!(
        "[{}] native attendance opened: rewards={} claimed={} granted_today={} seconds_remaining={}",
        session.addr(),
        rewards.len(),
        claimed.iter().filter(|value| **value).count(),
        granted,
        seconds_remaining
    );
    Ok(())
}

fn native_event_hub_packet(active: &[(u8, u8)]) -> Packet {
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(EVENT_HUB_SUB);
    out.write_i16(active.len().min(i16::MAX as usize) as i16);
    for &(event_id, enabled) in active.iter().take(i16::MAX as usize) {
        out.write_u8(event_id);
        out.write_u8(enabled);
    }
    out
}

async fn roulette_open(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
) -> anyhow::Result<()> {
    let free = repo.roulette_rewards(7).await.unwrap_or_default();
    let paid = repo.roulette_rewards(8).await.unwrap_or_default();
    if free.is_empty() || paid.is_empty() {
        let response = event_unavailable(Opcode::WizContinousPacketData as u8, 6);
        session.send_packet(&response).await?;
        return Ok(());
    }
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(6);
    out.write_i32(1);
    for reward in [&free[0], &paid[0]] {
        out.write_i32(reward.item_id);
        out.write_i32(reward.item_count as i32);
        out.write_i32(0);
    }
    session.send_packet(&out).await?;
    Ok(())
}

async fn roulette_spin(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
    client_type: i32,
) -> anyhow::Result<()> {
    let kind = native_type(client_type);
    let rewards = repo.roulette_rewards(kind).await?;
    let total_weight: i32 = rewards.iter().map(|r| r.weight.max(1)).sum();
    if rewards.is_empty() || total_weight <= 0 {
        return Ok(());
    }
    if repo
        .roulette_pending(name)
        .await?
        .and_then(|p| p.pending_item_id)
        .is_some()
    {
        let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
        out.write_u8(7);
        out.write_i32(client_type);
        out.write_i32(2);
        session.send_packet(&out).await?;
        return Ok(());
    }
    if client_type == ROULETTE_KC_TYPE
        && !knight_cash::cash_lose(
            session.world(),
            &session.pool().clone(),
            session.session_id(),
            ROULETTE_KC_COST,
        )
    {
        let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
        out.write_u8(7);
        out.write_i32(client_type);
        out.write_i32(20);
        session.send_packet(&out).await?;
        return Ok(());
    }
    let mut roll = rand::thread_rng().gen_range(0..total_weight);
    let mut selected = &rewards[0];
    for reward in &rewards {
        roll -= reward.weight.max(1);
        if roll < 0 {
            selected = reward;
            break;
        }
    }
    if !repo.reserve_roulette_result(name, selected).await? {
        return Ok(());
    }
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(7);
    out.write_i32(client_type);
    out.write_i32(1);
    session.send_packet(&out).await?;
    info!(
        "native roulette reserved: character={name} type={kind} slot={}",
        selected.slot
    );
    Ok(())
}

async fn roulette_reveal(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
    client_type: i32,
) -> anyhow::Result<()> {
    let pending = repo.roulette_pending(name).await?;
    let Some(p) = pending.filter(|p| p.pending_item_id.is_some()) else {
        let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
        out.write_u8(8);
        out.write_i32(client_type);
        out.write_i32(3);
        session.send_packet(&out).await?;
        return Ok(());
    };
    let item_id = p.pending_item_id.unwrap_or(0);
    let count = p.pending_item_count.unwrap_or(1).max(1) as u16;
    if !session
        .world()
        .check_weight(session.session_id(), item_id as u32, count)
    {
        let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
        out.write_u8(8);
        out.write_i32(client_type);
        out.write_i32(21);
        session.send_packet(&out).await?;
        return Ok(());
    }
    let completed = repo.complete_roulette(name).await?;
    if completed.as_ref().and_then(|v| v.pending_item_id).is_none() {
        return Ok(());
    }
    if !session
        .world()
        .give_item(session.session_id(), item_id as u32, count)
    {
        warn!("native roulette give_item failed: {name} item={item_id}");
        return Ok(());
    }
    send_roulette_result(session, client_type, &p).await
}

async fn send_roulette_result(
    session: &mut ClientSession,
    client_type: i32,
    p: &NativeRoulettePending,
) -> anyhow::Result<()> {
    let item = p.pending_item_id.unwrap_or(0);
    let count = p.pending_item_count.unwrap_or(1) as i32;
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(8);
    out.write_i32(client_type);
    out.write_i32(1);
    out.write_i32(p.pending_slot.unwrap_or(0) as i32 + 1);
    out.write_i32(item);
    out.write_i32(count);
    out.write_i32(item);
    out.write_i32(count);
    out.write_i32(0);
    out.write_i32(0);
    out.write_i32(0);
    out.write_i32(0);
    session.send_packet(&out).await?;
    Ok(())
}

async fn roulette_history(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
    client_type: i32,
) -> anyhow::Result<()> {
    let rows = repo.roulette_history(name).await?;
    let mut out = Packet::new(Opcode::WizContinousPacketData as u8);
    out.write_u8(9);
    out.write_i32(client_type);
    out.write_i32(1);
    out.write_i32(rows.len().min(20) as i32);
    for row in rows.iter().take(20) {
        out.write_i32(row.item_id);
        out.write_i32(row.item_count as i32);
        out.write_i32(row.roulette_type as i32);
    }
    session.send_packet(&out).await?;
    Ok(())
}

pub async fn handle_jigsaw_coin(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    let pool = session.pool().clone();
    let repo = NativeEventsRepository::new(&pool);
    let Some(name) = character_name(session) else {
        return Ok(());
    };
    match sub {
        1 if repo.is_active("jigsaw").await.unwrap_or(false) => {
            jigsaw_open(session, &repo, &name).await
        }
        4 | 8 => {
            if repo.is_active("jigsaw").await.unwrap_or(false) {
                jigsaw_open(session, &repo, &name).await
            } else if repo.is_active("coin").await.unwrap_or(false) {
                coin_open(session, &repo, &name).await
            } else {
                Ok(())
            }
        }
        5 if repo.is_active("jigsaw").await.unwrap_or(false) => {
            jigsaw_piece(session, &repo, &name, reader.read_u8().unwrap_or(255)).await
        }
        2 if repo.is_active("jigsaw").await.unwrap_or(false) => {
            jigsaw_claim(session, &repo, &name, reader.read_u8().unwrap_or(255)).await
        }
        2 if repo.is_active("coin").await.unwrap_or(false) => {
            coin_action(session, &repo, &name, reader.read_u8().unwrap_or(0)).await
        }
        _ => {
            debug!("[{}] native 0xCC ignored sub={sub}", session.addr());
            Ok(())
        }
    }
}

async fn jigsaw_open(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
) -> anyhow::Result<()> {
    let state = repo.jigsaw_state(name).await?;
    let mut out = Packet::new(Opcode::WizEnchant as u8);
    out.write_u8(1);
    out.write_u8(1);
    out.write_u8(
        state
            .piece_counts
            .iter()
            .map(|v| *v as i32)
            .sum::<i32>()
            .min(255) as u8,
    );
    out.write_u8(8);
    for i in 0..8 {
        out.write_u8(
            state
                .piece_counts
                .get(i)
                .copied()
                .unwrap_or(0)
                .clamp(0, 255) as u8,
        );
    }
    for i in 0..9 {
        out.write_u8(state.reward_claimed.get(i).copied().unwrap_or(false) as u8);
    }
    out.write_u8(0);
    session.send_packet(&out).await?;
    Ok(())
}

async fn jigsaw_piece(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
    piece: u8,
) -> anyhow::Result<()> {
    let Some(_state) = repo.add_jigsaw_piece(name, piece as i16).await? else {
        let mut out = Packet::new(Opcode::WizEnchant as u8);
        out.write_u8(1);
        out.write_u8(4);
        out.write_u8(2);
        session.send_packet(&out).await?;
        return Ok(());
    };
    let mut out = Packet::new(Opcode::WizEnchant as u8);
    out.write_u8(1);
    out.write_u8(3);
    out.write_u8(piece);
    out.write_u8(0);
    session.send_packet(&out).await?;
    Ok(())
}

async fn jigsaw_claim(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
    index: u8,
) -> anyhow::Result<()> {
    let grant = repo.claim_jigsaw(name, index as i16).await?;
    let success = if let Some(g) = grant {
        session.world().give_item(
            session.session_id(),
            g.item_id as u32,
            g.item_count.max(1) as u16,
        )
    } else {
        false
    };
    let state = repo.jigsaw_state(name).await.unwrap_or(NativeJigsawState {
        piece_counts: vec![0; 8],
        reward_claimed: vec![false; 9],
    });
    let mut out = Packet::new(Opcode::WizEnchant as u8);
    out.write_u8(1);
    out.write_u8(2);
    out.write_u8(if success { 1 } else { 4 });
    if success {
        for i in 0..9 {
            out.write_u8(state.reward_claimed.get(i).copied().unwrap_or(false) as u8);
        }
    }
    session.send_packet(&out).await?;
    Ok(())
}

async fn coin_open(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
) -> anyhow::Result<()> {
    let _state = repo.coin_state(name).await?;
    let mut out = Packet::new(Opcode::WizEnchant as u8);
    out.write_u8(2);
    out.write_u8(1);
    out.write_u8(1);
    out.write_u8(1);
    session.send_packet(&out).await?;
    Ok(())
}

async fn coin_action(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
    action: u8,
) -> anyhow::Result<()> {
    let event_type = action.clamp(1, 5);
    let state = repo
        .add_coin_point(name)
        .await?
        .unwrap_or(repo.coin_state(name).await?);
    if state.points >= 8 {
        if let Some(grant) = repo.claim_coin(name, (event_type - 1) as i16).await? {
            let _ = session.world().give_item(
                session.session_id(),
                grant.item_id as u32,
                grant.item_count.max(1) as u16,
            );
        }
    }
    let mut out = Packet::new(Opcode::WizEnchant as u8);
    out.write_u8(2);
    out.write_u8(3);
    out.write_u8(1);
    out.write_u8(event_type);
    out.write_u8(state.points.clamp(0, 8) as u8);
    session.send_packet(&out).await?;
    Ok(())
}

pub async fn handle_marble(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = PacketReader::new(&pkt.data);
    let sub = reader.read_u8().unwrap_or(0);
    let pool = session.pool().clone();
    let repo = NativeEventsRepository::new(&pool);
    if !repo.is_active("marble").await.unwrap_or(false) {
        return Ok(());
    }
    let Some(name) = character_name(session) else {
        return Ok(());
    };
    match sub {
        1 => marble_open(session, &repo, &name).await,
        2 => marble_roll(session, &repo, &name).await,
        3 | 4 | 5 => marble_open(session, &repo, &name).await,
        _ => Ok(()),
    }
}

async fn marble_open(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
) -> anyhow::Result<()> {
    let state = repo.marble_state(name).await?;
    let mut out = Packet::new(Opcode::WizAbility as u8);
    out.write_u8(1);
    out.write_u8(1);
    out.write_u8(1);
    out.write_u8(state.position as u8);
    out.write_u8(0);
    out.write_u8(state.rolls_today.clamp(0, 255) as u8);
    out.write_u8(state.laps.clamp(0, 255) as u8);
    out.write_u64(0);
    session.send_packet(&out).await?;
    Ok(())
}

async fn marble_roll(
    session: &mut ClientSession,
    repo: &NativeEventsRepository<'_>,
    name: &str,
) -> anyhow::Result<()> {
    let die = rand::thread_rng().gen_range(1..=6) as i16;
    let Some((state, tile)) = repo.roll_marble(name, die).await? else {
        let mut out = Packet::new(Opcode::WizAbility as u8);
        out.write_u8(2);
        out.write_u8(3);
        session.send_packet(&out).await?;
        return Ok(());
    };
    if tile.item_id > 0 && tile.item_count > 0 {
        let _ = session.world().give_item(
            session.session_id(),
            tile.item_id as u32,
            tile.item_count as u16,
        );
    }
    let mut ack = Packet::new(Opcode::WizAbility as u8);
    ack.write_u8(2);
    ack.write_u8(1);
    session.send_packet(&ack).await?;
    let mut out = Packet::new(Opcode::WizAbility as u8);
    out.write_u8(6);
    out.write_u8(1);
    out.write_u8(die as u8);
    out.write_u8(1);
    out.write_u8(state.position as u8);
    out.write_u8(tile.tile_type as u8);
    out.write_u8(state.rolls_today.clamp(0, 255) as u8);
    out.write_u8(0);
    out.write_u8(0);
    out.write_u8(0);
    out.write_u8(0);
    session.send_packet(&out).await?;
    info!(
        "native marble roll: character={name} die={die} position={}",
        state.position
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_roulette_types_map_to_tbl_groups() {
        assert_eq!(native_type(ROULETTE_FREE_TYPE), 7);
        assert_eq!(native_type(ROULETTE_KC_TYPE), 8);
    }

    #[test]
    fn unavailable_packet_uses_requested_sub() {
        let packet = event_unavailable(0x9C, 6);
        assert_eq!(packet.data, vec![6, 0, 0, 0, 0]);
    }

    #[test]
    fn event_hub_packet_matches_v2615_signed_f0_contract() {
        let packet = native_event_hub_packet(&[
            (EVENT_HUB_ATTENDANCE, 1),
            (EVENT_HUB_ROULETTE, 1),
            (EVENT_HUB_JIGSAW, 1),
            (EVENT_HUB_MARBLE, 1),
        ]);
        assert_eq!(packet.opcode, Opcode::WizContinousPacketData as u8);
        assert_eq!(packet.data, vec![0xF0, 4, 0, 1, 1, 2, 1, 3, 1, 5, 1]);
    }

    #[test]
    fn board_packets_match_unpacked_v2615_open_and_reply_dispatch() {
        let open = board_open_packet(7, &[]);
        assert_eq!(
            open.data,
            vec![
                2, // load/open dispatch
                7, 0, 0, 0, // requested board id
                1, 0, 0, 0, // success
                0, 0, 0, 0, // event/start header
                0, 0, 0, 0, // event/end header
                0, 0, 0, 0, // display/start timestamp
                0, 0, 0, 0, // display/end timestamp
                0, 0, // row count
            ]
        );

        let reply = board_reply_packet(7, 1, &[]);
        assert_eq!(reply.data.len(), 9 + 18);
        assert_eq!(&reply.data[..9], &[3, 7, 0, 0, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn board_history_rows_match_three_strings_and_timestamp() {
        let open = board_open_packet(0, &[("a".to_string(), 811_084_000, 1_700_000_000)]);
        assert_eq!(&open.data[25..27], &[1, 0]);
        assert_eq!(
            &open.data[27..],
            &[
                1, b'a', 20, b'I', b' ', b'L', b'o', b'v', b'e', b' ', b'K', b'n', b'i', b'g',
                b'h', b't', b' ', b'O', b'n', b'l', b'i', b'n', b'e', 9, b'8', b'1', b'1', b'0',
                b'8', b'4', b'0', b'0', b'0', 0, 241, 83, 101,
            ]
        );
    }

    #[test]
    fn akara_auction_list_row_matches_v2615_26_byte_contract() {
        let packet = akara_auction_list_packet(
            AKARA_AUCTION_ALT_LIST_SUB,
            &[(0, 810_889_000, 0, 1_000_000, 100_000, 1_700_000_000, 0)],
        );
        assert_eq!(packet.opcode, Opcode::WizCostume as u8);
        assert_eq!(packet.data[0], AKARA_AUCTION_ALT_LIST_SUB);
        assert_eq!(&packet.data[1..3], &[1, 0]);
        assert_eq!(packet.data[3], 1);
        assert_eq!(packet.data.len(), 4 + 26);
    }

    #[test]
    fn akara_select_flags_match_v2615_select_msg_dispatch() {
        assert_eq!(AKARA_ALTAR_SELECT_FLAG, 0x3A);
        assert_eq!(AKARA_POST_UP_SELECT_FLAG, 0x44);
    }

    #[test]
    fn attendance_wire_keys_match_v2615_calendar_lookup() {
        assert_eq!(attendance_daily_slot_key(0), 1);
        assert_eq!(attendance_daily_slot_key(24), 25);
        assert_eq!(attendance_cumulative_slot_key(0), 101);
        assert_eq!(attendance_cumulative_slot_key(2), 103);
    }
}
