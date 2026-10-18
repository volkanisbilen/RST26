//! WIZ_GENIE (0x97) handler — genie (lamp) system.
//! The genie system provides automated play assistance (auto-loot, auto-potion).
//! Genie time is consumed from a per-character allowance, activated via spirit
//! potion items.
//! ## Top-level commands
//! | Value | Name               | Description                          |
//! |-------|--------------------|--------------------------------------|
//! | 1     | GenieInfoRequest   | Non-attack sub-commands              |
//! | 2     | GenieUpdateRequest | Attack-mode sub-commands (proxied)   |
//! | 25    | GenieNotice        | Walking function toggle notice       |
//! ## GenieInfoRequest sub-commands
//! | Value | Name                  | Description                      |
//! |-------|-----------------------|----------------------------------|
//! | 1     | GenieUseSpiringPotion | Consume genie spirit potion      |
//! | 2     | GenieLoadOptions      | Load saved genie options         |
//! | 3     | GenieSaveOptions      | Save genie options               |
//! | 4     | GenieStartHandle      | Activate genie mode              |
//! | 5     | GenieStopHandle       | Deactivate genie mode            |
//! | 6     | GenieRemainingTime    | Send remaining genie time        |
//! | 7     | GenieActivated        | Region broadcast (active/inactive)|

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::WorldState;
use crate::zone::SessionId;

/// Get current UNIX timestamp in seconds (u32).
pub fn now_secs() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

/// Compute remaining genie seconds from absolute UNIX timestamp.
pub fn genie_remaining_from_abs(abs: u32) -> u32 {
    abs.saturating_sub(now_secs())
}

/// Convert remaining genie seconds to display hours.
/// ```text
/// int hour = int(m_1098GenieTime > UNIXTIME ? m_1098GenieTime - UNIXTIME : 0);
/// if (hour <= 0)   return 0;
/// if (hour < 3600) return 1;
/// double test = hour / static_cast<double>(3600);
/// return (uint16)round(test);
/// ```
pub(crate) fn get_genie_hours_pub(remaining_secs: u32) -> u16 {
    get_genie_hours(remaining_secs)
}

fn get_genie_hours(remaining_secs: u32) -> u16 {
    if remaining_secs == 0 {
        return 0;
    }
    if remaining_secs < 3600 {
        return 1;
    }
    (remaining_secs as f64 / 3600.0).round() as u16
}

// ── Top-level command constants ─────────────────────────────────────────

/// Non-attack sub-command group.
const GENIE_INFO_REQUEST: u8 = 1;
/// Attack-mode sub-command group (move/rotate/attack/magic).
const GENIE_UPDATE_REQUEST: u8 = 2;
/// Walking function toggle notice.
const GENIE_NOTICE: u8 = 25;

// ── GenieAttackHandle sub-commands ──────────────────────────────────────
const GENIE_MOVE: u8 = 1;
const GENIE_ROTATE: u8 = 2;
const GENIE_MAIN_ATTACK: u8 = 3;
const GENIE_MAGIC: u8 = 4;

// ── GenieInfoRequest sub-commands ───────────────────────────────────────

/// Consume genie spirit potion item.
const GENIE_USE_SPIRING_POTION: u8 = 1;
/// Load saved genie options from server.
const GENIE_LOAD_OPTIONS: u8 = 2;
/// Save genie options to server.
const GENIE_SAVE_OPTIONS: u8 = 3;
/// Activate genie mode.
const GENIE_START_HANDLE: u8 = 4;
/// Deactivate genie mode.
const GENIE_STOP_HANDLE: u8 = 5;
/// Send remaining genie time to client.
const GENIE_REMAINING_TIME: u8 = 6;
/// Region broadcast of genie active/inactive state.
const GENIE_ACTIVATED: u8 = 7;

// ── Genie status constants ──────────────────────────────────────────────

/// Status: genie active.  (The original server sent its first start reply with
/// this value as the command byte; it equals `GENIE_INFO_REQUEST`, which is what
/// the single start reply uses now.)
#[allow(dead_code)]
const GENIE_STATUS_ACTIVE: u8 = 1;

/// No skills: two u16 fields and 42 option bytes (v2615 B19360/B1A4B0).
const GENIE_OPTIONS_DEFAULT_SIZE: usize = 46;
/// Full v2615 layout with all 32 skill slots populated.
const GENIE_OPTIONS_MAX_SIZE: usize = 174;

// v2615 B19360 writes, B1A4B0 reads: u16 count, count*u32 skills,
// u16(42), 42 option bytes. Never truncate this variable-size structure.
fn valid_options(data: &[u8]) -> bool {
    if data.len() < 4 || data.len() > GENIE_OPTIONS_MAX_SIZE {
        return false;
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let offset = 2 + count * 4;
    count <= 32 && data.len() == offset + 44 && data[offset..offset + 2] == [42, 0]
}

/// The JstKO client saves the game's own options padded to 180 bytes followed by its own 220 byte block
/// (genie leader / go range / party code / mob list): 400 bytes in total (`DLL genie_hg.cpp`).
const HG_NATIVE_LEN: usize = 180;
const HG_BLOCK_LEN: usize = 220;
const HG_BLOB_LEN: usize = HG_NATIVE_LEN + HG_BLOCK_LEN;
/// Hook sub-opcode of the genie channel (`E9 ED`), op 4 carries the JstKO block to the client.
const EXT_SUB_HGGENIE: u8 = 0xED;

/// Length of the game's own option structure at the start of `data` (None when it is not one).
fn native_len(data: &[u8]) -> Option<usize> {
    if data.len() < 4 {
        return None;
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let offset = 2 + count * 4;
    (count <= 32 && data.len() >= offset + 44 && data[offset..offset + 2] == [42, 0]).then_some(offset + 44)
}

/// A saved blob: the game's own options alone, or the 400 byte JstKO blob.
fn valid_payload(data: &[u8]) -> bool {
    valid_options(data) || (data.len() == HG_BLOB_LEN && native_len(&data[..HG_NATIVE_LEN]).is_some())
}

fn default_options() -> Vec<u8> {
    let mut options = vec![0; GENIE_OPTIONS_DEFAULT_SIZE];
    options[2] = 42;
    // Defaults initialized by CUIGenie_Main at B18E1D and B1A4E6.
    for (index, value) in [
        (2, 30),
        (3, 10),
        (4, 30),
        (6, 30),
        (8, 80),
        (25, 3),
        (27, 2),
    ] {
        options[4 + index] = value;
    }
    options
}

/// Convert absolute genie timestamp to DB i32 for storage.
/// C++ and in-memory both use absolute UNIX timestamp (`uint32`).
/// DB column is INTEGER (i32). If expired (abs <= now), stores 0.
pub fn genie_abs_to_db(abs: u32) -> i32 {
    if abs == 0 {
        return 0;
    }
    // If already expired, store 0 to avoid stale data
    if abs <= now_secs() {
        return 0;
    }
    abs as i32
}

/// Largest genie allowance a character can hold at once, in hours.
///
/// The client keeps the remaining time as a `u16` hour count (use-spirit reply,
/// start/stop replies, remaining-time tick) and the database column is an `i32`
/// UNIX timestamp, so an unbounded stack of vouchers/potions would eventually
/// wrap the stored value and wipe the whole allowance.  60 000 h is about 6.8
/// years: far beyond any real use and still inside both limits.
pub const GENIE_MAX_HOURS: u32 = 60_000;

/// Absolute expiry after adding `hours` to an allowance expiring at `abs`.
///
/// Time that is still running is kept (`max(abs, now) + hours`); `None` when
/// the result would exceed [`GENIE_MAX_HOURS`] of remaining time or the `i32`
/// range of the database column.
pub fn genie_abs_after(abs: u32, now: u32, hours: u32) -> Option<u32> {
    let new_abs = u64::from(abs.max(now)) + u64::from(hours) * 3600;
    let remaining = new_abs.saturating_sub(u64::from(now));
    if remaining > u64::from(GENIE_MAX_HOURS) * 3600 || new_abs > i32::MAX as u64 {
        return None;
    }
    Some(new_abs as u32)
}

/// Add `hours` of genie time to a session (spirit potion, vouchers, GM command,
/// Lua `GenieExchange`).  Atomic on the session entry.  Returns the new absolute
/// expiry, or `None` when the session is gone or the cap would be exceeded.
pub fn grant_genie_hours(world: &WorldState, sid: SessionId, hours: u32) -> Option<u32> {
    let now = now_secs();
    let mut granted = None;
    world.update_session(sid, |h| {
        if let Some(abs) = genie_abs_after(h.genie_time_abs, now, hours) {
            h.genie_time_abs = abs;
            granted = Some(abs);
        }
    });
    granted
}

/// Whether the connected client reads the `GenieActivated` session id as a
/// `u32` (v2625 `sub_B20AB0` / live `0xB214F0`) instead of the `u16` the
/// v2615 client and the original server used.
fn client_uses_u32_genie_id(world: &WorldState) -> bool {
    world
        .get_server_settings()
        .map(|s| genie_id_is_u32(s.game_version))
        .unwrap_or(false)
}

/// `GenieActivated` id width for a given `server_settings.game_version`.
fn genie_id_is_u32(game_version: i16) -> bool {
    game_version >= 2625
}

/// `GenieActivated` region packet: `[01][07][id][active]`.
fn build_activated_packet(sid: SessionId, active: bool, u32_id: bool) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGenie as u8);
    pkt.write_u8(GENIE_INFO_REQUEST);
    pkt.write_u8(GENIE_ACTIVATED);
    if u32_id {
        pkt.write_u32(u32::from(sid));
    } else {
        pkt.write_u16(sid);
    }
    pkt.write_u8(u8::from(active));
    pkt
}

/// `[01][sub][u16 1][u16 hours]` reply shared by GenieStart (sub 4) and GenieStop (sub 5).
fn build_state_reply(sub: u8, hours: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGenie as u8);
    pkt.write_u8(GENIE_INFO_REQUEST);
    pkt.write_u8(sub);
    pkt.write_u16(1);
    pkt.write_u16(hours);
    pkt
}

/// Tell the 3x3 region that `sid` entered/left genie mode and refresh its
/// appearance (the `isInGenie()` marker is part of the UserInOut block).
fn broadcast_genie_state(world: &WorldState, sid: SessionId, active: bool) {
    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        let pkt = build_activated_packet(sid, active, client_uses_u32_genie_id(world));
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }
    super::region::broadcast_user_appearance_update(world, sid);
}

/// Activate genie mode for a session that already passed the start checks.
/// The start reply goes only to the owner; the region gets the marker.
pub(crate) fn activate_genie(world: &WorldState, sid: SessionId, remaining_secs: u32) {
    world.update_session(sid, |h| h.genie_active = true);
    world.send_to_session_owned(
        sid,
        build_state_reply(GENIE_START_HANDLE, get_genie_hours(remaining_secs)),
    );
    broadcast_genie_state(world, sid, true);
}

/// Clear genie mode atomically.  Returns `true` when it was active (so only one
/// of several racing stop paths — client stop, expiry tick, respawn, GM — does
/// the follow-up work).
fn take_genie_active(world: &WorldState, sid: SessionId) -> bool {
    let mut was_active = false;
    world.update_session(sid, |h| {
        was_active = h.genie_active;
        h.genie_active = false;
    });
    was_active
}

/// Deactivate genie mode from server-side code (expiry tick, respawn, GM):
/// owner gets the stop reply, the region the marker change.  `false` when the
/// genie was not active.
pub(crate) fn deactivate_genie(world: &WorldState, sid: SessionId) -> bool {
    if !take_genie_active(world, sid) {
        return false;
    }
    let abs = world.with_session(sid, |h| h.genie_time_abs).unwrap_or(0);
    let hours = get_genie_hours(genie_remaining_from_abs(abs));
    world.send_to_session_owned(sid, build_state_reply(GENIE_STOP_HANDLE, hours));
    broadcast_genie_state(world, sid, false);
    true
}

/// Write the genie expiry to the database.  Only when the stored row was
/// loaded for this session (a failed load must never be overwritten with a
/// guess) and always from the live session value.
async fn save_genie_time(world: &WorldState, pool: &ko_db::DbPool, sid: SessionId, label: &str) {
    let Some((abs, options, loaded)) =
        world.with_session(sid, |h| (h.genie_time_abs, h.genie_options.clone(), h.genie_loaded))
    else {
        return;
    };
    if !loaded {
        return;
    }
    let Some(name) = world.get_character_info(sid).map(|c| c.name.clone()) else {
        return;
    };
    let repo = ko_db::repositories::user_data::UserDataRepository::new(pool);
    if let Err(e) = repo
        .save_genie_data(&name, genie_abs_to_db(abs), &options, 0)
        .await
    {
        warn!("{}: failed to save genie time for {}: {}", label, name, e);
    }
}

/// Fire-and-forget [`save_genie_time`] for code that has no session handle
/// (GM command, Lua).  No-op without a database (tests).
pub(crate) fn persist_genie_time(world: &Arc<WorldState>, sid: SessionId, label: &'static str) {
    let Some(pool) = world.db_pool().cloned() else {
        return;
    };
    // Lua scripts may run on a plain thread; without a runtime the periodic save covers it.
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let world = Arc::clone(world);
    runtime.spawn(async move {
        save_genie_time(&world, &pool, sid, label).await;
    });
}

/// Reply code for a refused spirit-potion use: the client prints "Fail"
/// (`sub_B1F8C0`, hours 0xFFFB / 0xFFFA).
const GENIE_USE_FAIL: u16 = 0xFFFB;

/// `[01][01][u16]` use-spirit reply: remaining hours, or [`GENIE_USE_FAIL`].
fn build_use_reply(value: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizGenie as u8);
    pkt.write_u8(GENIE_USE_SPIRING_POTION);
    pkt.write_u8(GENIE_USE_SPIRING_POTION);
    pkt.write_u16(value);
    pkt
}

/// Party-heal percentage the DLL starts with (`genie_hg.cpp` Settings::partyHeal).
const HG_DEFAULT_PARTY_HEAL: u8 = 50;

/// Block for a character that has none stored yet: signature `HG`, version 1,
/// party heal at its default.  Without any block the DLL never marks its
/// settings as loaded (`g_loaded`) and drops every edit of the JstKO controls
/// (PT code, go-to-range, auto party, leader target) until the game's own save
/// happens to run.
fn default_hg_block() -> Vec<u8> {
    let mut block = vec![0u8; HG_BLOCK_LEN];
    block[0] = b'H';
    block[1] = b'G';
    block[2] = 1;
    block[6] = HG_DEFAULT_PARTY_HEAL;
    block
}

/// Handle WIZ_GENIE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    // A dead character may still stop its genie, load/save its options: the
    // client stops the bot when it dies and the server must not keep the
    // "in genie" flag (warehouse/trade/shop lock and the marker over the head).
    let dead = session.world().is_player_dead(session.session_id());

    let mut r = PacketReader::new(&pkt.data);
    let command = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    debug!(
        "[{}] WIZ_GENIE: command={} payload_len={}",
        session.addr(),
        command,
        pkt.data.len()
    );

    match command {
        GENIE_INFO_REQUEST => handle_info_request(session, &mut r, dead).await,
        GENIE_UPDATE_REQUEST => {
            if dead {
                // Corpses neither walk nor fight.
                return Ok(());
            }
            // Attack-mode commands (move/rotate/attack/magic) are proxied
            // through the genie system. The inner packet must be extracted
            // and dispatched to the appropriate handler.
            //
            // ```c++
            // uint8 command = pkt.read<uint8>();
            // if (UNIXTIME > m_1098GenieTime) return SendGenieStop(true);
            // switch (command) {
            //   case GenieMove:       MoveProcess(pkt); break;
            //   case GenieRotate:     Rotate(pkt); break;
            //   case GenieMainAttack: Attack(pkt); break;
            //   case GenieMagic:      CMagicProcess::MagicPacket(pkt, this); break;
            // }
            // ```
            let sid = session.session_id();
            let abs = session
                .world()
                .with_session(sid, |h| h.genie_time_abs)
                .unwrap_or(0);
            let remaining = genie_remaining_from_abs(abs);
            if remaining == 0 {
                // Genie expired — auto-stop
                handle_genie_stop(session).await?;
                return Ok(());
            }

            // Read inner command and dispatch to appropriate handler
            let inner_command = r.read_u8().unwrap_or(0);
            let inner_data = r.read_remaining().to_vec();

            match inner_command {
                GENIE_MOVE => {
                    let inner_pkt = Packet::with_data(Opcode::WizMove as u8, inner_data);
                    super::move_handler::handle(session, inner_pkt).await?;
                }
                GENIE_ROTATE => {
                    let inner_pkt = Packet::with_data(Opcode::WizRotate as u8, inner_data);
                    super::rotate::handle(session, inner_pkt)?;
                }
                GENIE_MAIN_ATTACK => {
                    let inner_pkt = Packet::with_data(Opcode::WizAttack as u8, inner_data);
                    super::attack::handle(session, inner_pkt).await?;
                }
                GENIE_MAGIC => {
                    let inner_pkt = Packet::with_data(Opcode::WizMagicProcess as u8, inner_data);
                    super::magic_process::handle(session, inner_pkt).await?;
                }
                _ => {
                    debug!(
                        "[{}] WIZ_GENIE: unknown attack handle command={}",
                        session.addr(),
                        inner_command
                    );
                }
            }
            Ok(())
        }
        GENIE_NOTICE => {
            // Walking function toggle — informational only
            let _status = r.read_u8().unwrap_or(0);
            debug!("[{}] WIZ_GENIE: Notice status={}", session.addr(), _status);
            Ok(())
        }
        _ => {
            debug!(
                "[{}] WIZ_GENIE: unhandled command={}",
                session.addr(),
                command
            );
            Ok(())
        }
    }
}

/// Sub-commands a dead character may still send: stopping the genie and the
/// settings round-trip.  Using a potion or starting the genie need a live body.
fn allowed_while_dead(sub_command: u8) -> bool {
    matches!(
        sub_command,
        GENIE_STOP_HANDLE | GENIE_LOAD_OPTIONS | GENIE_SAVE_OPTIONS
    )
}

/// Handle GenieInfoRequest sub-commands.
async fn handle_info_request(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
    dead: bool,
) -> anyhow::Result<()> {
    let sub_command = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    if dead && !allowed_while_dead(sub_command) {
        return Ok(());
    }

    debug!(
        "[{}] WIZ_GENIE: InfoRequest sub={} remaining_bytes={}",
        session.addr(),
        sub_command,
        r.remaining()
    );

    match sub_command {
        GENIE_USE_SPIRING_POTION => handle_genie_use_spirit(session, r).await,
        GENIE_LOAD_OPTIONS => handle_load_options(session).await,
        GENIE_SAVE_OPTIONS => handle_save_options(session, r).await,
        GENIE_START_HANDLE => handle_genie_start(session).await,
        GENIE_STOP_HANDLE => handle_genie_stop(session).await,
        _ => {
            debug!(
                "[{}] WIZ_GENIE: InfoRequest unhandled sub={}",
                session.addr(),
                sub_command
            );
            Ok(())
        }
    }
}

/// Valid genie spirit potion item IDs.
const GENIE_ITEM_IDS: [u32; 3] = [810305000, 810378000, 900772000];

/// Default genie duration (hours) granted per spirit potion.
const GENIE_HOURS_PER_POTION: u32 = 360;

/// Handle genie spirit potion consumption.
/// 1. Read item_id from packet
/// 2. Validate it's a valid genie item
/// 3. Remove from inventory
/// 4. Extend genie time by 360 hours
/// 5. Send remaining time to client
async fn handle_genie_use_spirit(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let item_id = match r.read_u32() {
        Some(v) => v,
        None => return Ok(()),
    };

    let sid = session.session_id();
    let world = session.world().clone();

    // Validate item is a known genie spirit potion
    if !GENIE_ITEM_IDS.contains(&item_id) {
        session.send_packet(&build_use_reply(GENIE_USE_FAIL)).await?;
        return Ok(());
    }

    // if (isTrading() || isMerchanting() || isMining() || isFishing()) return;
    if world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        session.send_packet(&build_use_reply(GENIE_USE_FAIL)).await?;
        return Ok(());
    }

    // The stored allowance must have been read for this session, otherwise the
    // new total would be computed from 0 and later overwrite the real row.
    let loaded = world.with_session(sid, |h| h.genie_loaded).unwrap_or(false);

    // Check item exists in inventory and that the extra time still fits the cap
    // (checked before the potion is taken so a refused use costs nothing).
    let fits = world
        .with_session(sid, |h| {
            genie_abs_after(h.genie_time_abs, now_secs(), GENIE_HOURS_PER_POTION).is_some()
        })
        .unwrap_or(false);
    if !loaded || !fits || !world.check_exist_item(sid, item_id, 1) {
        session.send_packet(&build_use_reply(GENIE_USE_FAIL)).await?;
        return Ok(());
    }

    // Remove the item from inventory (rob_item sends WIZ_ITEM_COUNT_CHANGE).
    // Only a successful removal grants time.
    if !world.rob_item(sid, item_id, 1) {
        session.send_packet(&build_use_reply(GENIE_USE_FAIL)).await?;
        return Ok(());
    }

    // Extend genie time by 360 hours (= 360 * 3600 seconds)
    // Absolute timestamp: if expired, start from now; if active, extend deadline.
    let Some(genie_abs) = grant_genie_hours(&world, sid, GENIE_HOURS_PER_POTION) else {
        // Lost a race with another grant that used up the headroom: give the potion back.
        world.give_item(sid, item_id, 1);
        session.send_packet(&build_use_reply(GENIE_USE_FAIL)).await?;
        return Ok(());
    };

    // Send response:
    //   Packet result(WIZ_GENIE, uint8(GenieUseSpiringPotion));
    //   result << uint8(GenieUseSpiringPotion) << GetGenieTime();
    // Wire: [u8(1)] [u8(1)] [u16 hours]
    let remaining = genie_remaining_from_abs(genie_abs);
    let hours = get_genie_hours(remaining);
    session.send_packet(&build_use_reply(hours)).await?;

    // Persist genie time to DB after potion use to prevent data loss.  Awaited
    // (not spawned) so two potions used back to back are written in order.
    let pool = session.pool().clone();
    save_genie_time(&world, &pool, sid, "GenieUseSpirit").await;

    debug!(
        "[{}] WIZ_GENIE: Used spirit potion item={}, genie_abs={}, remaining={}",
        session.addr(),
        item_id,
        genie_abs,
        remaining
    );
    Ok(())
}

/// Load and send genie options to the client.
/// Sends a blob of saved genie configuration bytes.
pub(crate) async fn handle_load_options(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world();

    // Get saved options from session data (or default zeros)
    let saved = world
        .with_session(sid, |h| h.genie_options.clone())
        .filter(|saved| valid_payload(saved));
    let (options, hg_block): (Vec<u8>, Option<Vec<u8>>) = match saved {
        Some(blob) if blob.len() == HG_BLOB_LEN => {
            let n = native_len(&blob[..HG_NATIVE_LEN]).unwrap_or(0);
            (blob[..n].to_vec(), Some(blob[HG_NATIVE_LEN..].to_vec()))
        }
        // Nothing (or only the game's own part) stored: the DLL still needs a block,
        // otherwise it never treats its settings as loaded and loses edits.
        Some(blob) => (blob, Some(default_hg_block())),
        None => (default_options(), Some(default_hg_block())),
    };

    let mut resp = Packet::new(Opcode::WizGenie as u8);
    resp.write_u8(GENIE_INFO_REQUEST);
    resp.write_u8(GENIE_LOAD_OPTIONS);
    resp.data.extend_from_slice(&options);
    session.send_packet(&resp).await?;
    if let Some(block) = hg_block {
        let mut hook = Packet::new(Opcode::EXT_HOOK_S2C);
        hook.write_u8(EXT_SUB_HGGENIE);
        hook.write_u8(4);
        hook.data.extend_from_slice(&block);
        session.send_packet(&hook).await?;
    }

    tracing::info!(
        "[{}] WIZ_GENIE: LoadOptions sent ({} bytes)",
        session.addr(),
        options.len()
    );
    Ok(())
}

/// Retry a failed options write a few times.  A retry only runs while the
/// session still holds exactly these options, so it can never overwrite a newer
/// save that succeeded in the meantime.
fn spawn_options_retry(
    world: Arc<WorldState>,
    pool: ko_db::DbPool,
    sid: SessionId,
    char_name: String,
    options: Vec<u8>,
) {
    tokio::spawn(async move {
        for delay_secs in [2u64, 10, 30] {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            let current = world.with_session(sid, |h| h.genie_options == options);
            if current != Some(true) {
                return;
            }
            let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
            match repo.save_genie_options(&char_name, &options).await {
                Ok(()) => return,
                Err(e) => warn!("WIZ_GENIE: retry saving options for {}: {}", char_name, e),
            }
        }
    });
}

/// Save genie options from the client.
/// Reads the options blob from the packet and stores it in the session.
async fn handle_save_options(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // Reject incomplete payloads without destroying the last complete settings.
    let payload = r.read_remaining();
    if !valid_payload(payload) {
        warn!(
            "WIZ_GENIE: rejected malformed options: sid={}, bytes={}",
            sid,
            payload.len()
        );
        return Ok(());
    }
    let options = payload.to_vec();

    // `genie_loaded` is deliberately left alone: it only says "the expiry in
    // memory was read from the database".  Setting it here (as an earlier
    // version did) let the periodic save write expiry 0 over the real row
    // whenever the login-time load had failed and the user then saved options.
    world.update_session(sid, |h| {
        h.genie_options = options.clone();
    });

    // Persist immediately. Disconnect/periodic saves remain as a safeguard,
    // but the client expects a settings change to survive a relog right away.
    // A database error must not end the session (a handler error closes the
    // connection): log it and retry in the background.
    if let Some(char_name) = world.get_character_info(sid).map(|c| c.name.clone()) {
        let pool = session.pool().clone();
        let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
        if let Err(e) = repo.save_genie_options(&char_name, &options).await {
            warn!(
                "WIZ_GENIE: saving options for {} failed ({}); retrying in background",
                char_name, e
            );
            spawn_options_retry(world.clone(), pool, sid, char_name, options.clone());
        }
    }

    tracing::info!(
        "[{}] WIZ_GENIE: SaveOptions ({} bytes)",
        session.addr(),
        options.len()
    );
    Ok(())
}

/// Activate the genie.
/// Sends the genie activation packet with remaining time.
async fn handle_genie_start(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    // Premium requirement check — server setting `LootandGeniePremium`.
    let requires_premium = world
        .get_server_settings()
        .map(|s| s.loot_genie_premium != 0)
        .unwrap_or(false);
    let (has_premium, abs) = world
        .with_session(sid, |h| (h.premium_in_use > 0, h.genie_time_abs))
        .unwrap_or((false, 0));
    if requires_premium && !has_premium {
        warn!(
            "[{}] WIZ_GENIE: Start rejected — premium required but inactive",
            session.addr()
        );
        return Ok(());
    }

    // Time check — genie must have remaining time.
    let remaining = genie_remaining_from_abs(abs);
    if remaining == 0 {
        warn!(
            "[{}] WIZ_GENIE: Start rejected — no remaining time (abs={})",
            session.addr(),
            abs
        );
        return Ok(());
    }

    // Set genie active, answer the owner (one start reply: the original server
    // sent the identical packet twice, which makes the client run its start
    // routine, effect and chat line twice) and tell the region.  `isInGenie()`
    // is also carried in the UserInOut appearance block, which
    // `broadcast_genie_state` refreshes as well.
    activate_genie(&world, sid, remaining);

    debug!(
        "[{}] WIZ_GENIE: Started (remaining={})",
        session.addr(),
        remaining
    );
    Ok(())
}

/// Deactivate the genie.
pub(crate) async fn handle_genie_stop(session: &mut ClientSession) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();

    let abs = world.with_session(sid, |h| h.genie_time_abs).unwrap_or(0);
    let remaining = genie_remaining_from_abs(abs);
    let hours = get_genie_hours(remaining);

    // Atomic test-and-clear: whichever stop path gets here first does the work.
    if !take_genie_active(&world, sid) {
        return Ok(());
    }

    session
        .send_packet(&build_state_reply(GENIE_STOP_HANDLE, hours))
        .await?;
    broadcast_genie_state(&world, sid, false);

    // Persist genie time to DB on stop to prevent data loss on crash.
    let pool = session.pool().clone();
    save_genie_time(&world, &pool, sid, "GenieStop").await;

    debug!(
        "[{}] WIZ_GENIE: Stopped (remaining={})",
        session.addr(),
        remaining
    );
    Ok(())
}

/// Genie time check interval in seconds (1 minute).
const PLAYER_GENIE_INTERVAL: u64 = 60;

/// Periodic genie time check — called from the expiry tick loop.
/// ```c++
/// if (GetGenieTime() > 0 && m_tGenieTimeNormal + PLAYER_GENIE_INTERVAL < UNIXTIME) {
///     m_tGenieTimeNormal = UNIXTIME;
///     CheckGenieTime();
/// }
/// ```
/// `CheckGenieTime()` sends remaining hours to client and stops genie if expired.
pub fn check_genie_time_tick(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    now: u64,
) {
    let (genie_abs, genie_check_time) =
        match world.with_session(sid, |h| (h.genie_time_abs, h.genie_check_time)) {
            Some(v) => v,
            None => return,
        };

    // C++ checks only `GetGenieTime() > 0` — NOT `m_bGenieActive`.
    // Time passes naturally via absolute timestamp — no manual decrement needed.
    let remaining = genie_abs.saturating_sub(now as u32);
    if genie_abs == 0 || remaining == 0 {
        // Already expired or never had time — stop a still-running genie.
        // The stop reply carries 0 hours (the client then walks home), the
        // marker change goes to the player's own region only.
        deactivate_genie(world, sid);
        return;
    }

    if genie_check_time + PLAYER_GENIE_INTERVAL >= now {
        return;
    }

    // Update check time
    world.update_session(sid, |h| {
        h.genie_check_time = now;
    });

    // Send remaining time to client
    let hours = get_genie_hours(remaining);
    let mut pkt = Packet::new(Opcode::WizGenie as u8);
    pkt.write_u8(GENIE_INFO_REQUEST);
    pkt.write_u8(GENIE_REMAINING_TIME);
    pkt.write_u16(hours);
    world.send_to_session_owned(sid, pkt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_genie_constants() {
        assert_eq!(GENIE_INFO_REQUEST, 1);
        assert_eq!(GENIE_UPDATE_REQUEST, 2);
        assert_eq!(GENIE_NOTICE, 25);
        assert_eq!(GENIE_USE_SPIRING_POTION, 1);
        assert_eq!(GENIE_LOAD_OPTIONS, 2);
        assert_eq!(GENIE_SAVE_OPTIONS, 3);
        assert_eq!(GENIE_START_HANDLE, 4);
        assert_eq!(GENIE_STOP_HANDLE, 5);
        assert_eq!(GENIE_REMAINING_TIME, 6);
        assert_eq!(GENIE_ACTIVATED, 7);
        assert_eq!(GENIE_STATUS_ACTIVE, 1); // C++ GenieStatusActive = 1
        assert_eq!(GENIE_OPTIONS_DEFAULT_SIZE, 46);
        assert_eq!(GENIE_OPTIONS_MAX_SIZE, 174);
    }

    #[test]
    fn test_genie_start_packet_format() {
        // GenieStatusActive packet — time is u16 hours, not u32 seconds
        let hours = get_genie_hours(3600); // 1 hour remaining → 1
        let mut resp = Packet::new(Opcode::WizGenie as u8);
        resp.write_u8(GENIE_STATUS_ACTIVE);
        resp.write_u8(4);
        resp.write_u16(1);
        resp.write_u16(hours);

        assert_eq!(resp.opcode, Opcode::WizGenie as u8);
        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(GENIE_STATUS_ACTIVE));
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(1)); // 1 hour
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_genie_stop_packet_format() {
        let hours = get_genie_hours(1800); // 30 min → 1 (< 3600 returns 1)
        let mut resp = Packet::new(Opcode::WizGenie as u8);
        resp.write_u8(GENIE_INFO_REQUEST);
        resp.write_u8(GENIE_STOP_HANDLE);
        resp.write_u16(1);
        resp.write_u16(hours);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(GENIE_INFO_REQUEST));
        assert_eq!(r.read_u8(), Some(GENIE_STOP_HANDLE));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(1)); // < 1 hour → 1
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_genie_activated_region_broadcast() {
        // Genie activated broadcast
        let mut pkt = Packet::new(Opcode::WizGenie as u8);
        pkt.write_u8(GENIE_INFO_REQUEST);
        pkt.write_u8(GENIE_ACTIVATED);
        pkt.write_u16(42); // native C++ GetID() is uint16
        pkt.write_u8(1); // active

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(GENIE_INFO_REQUEST));
        assert_eq!(r.read_u8(), Some(GENIE_ACTIVATED));
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);

        // Deactivated
        let mut pkt2 = Packet::new(Opcode::WizGenie as u8);
        pkt2.write_u8(GENIE_INFO_REQUEST);
        pkt2.write_u8(GENIE_ACTIVATED);
        pkt2.write_u16(42);
        pkt2.write_u8(0); // inactive

        let mut r2 = PacketReader::new(&pkt2.data);
        assert_eq!(r2.read_u8(), Some(GENIE_INFO_REQUEST));
        assert_eq!(r2.read_u8(), Some(GENIE_ACTIVATED));
        assert_eq!(r2.read_u16(), Some(42));
        assert_eq!(r2.read_u8(), Some(0));
        assert_eq!(r2.remaining(), 0);
    }

    #[test]
    fn test_genie_load_options_packet() {
        let mut resp = Packet::new(Opcode::WizGenie as u8);
        resp.write_u8(GENIE_INFO_REQUEST);
        resp.write_u8(GENIE_LOAD_OPTIONS);
        let options = vec![0u8; GENIE_OPTIONS_DEFAULT_SIZE];
        resp.data.extend_from_slice(&options);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(GENIE_INFO_REQUEST));
        assert_eq!(r.read_u8(), Some(GENIE_LOAD_OPTIONS));
        assert_eq!(r.remaining(), GENIE_OPTIONS_DEFAULT_SIZE);
    }

    #[test]
    fn test_genie_remaining_time_packet() {
        let hours = get_genie_hours(7200); // 2 hours → 2
        let mut resp = Packet::new(Opcode::WizGenie as u8);
        resp.write_u8(GENIE_INFO_REQUEST);
        resp.write_u8(GENIE_REMAINING_TIME);
        resp.write_u16(hours);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(GENIE_INFO_REQUEST));
        assert_eq!(r.read_u8(), Some(GENIE_REMAINING_TIME));
        assert_eq!(r.read_u16(), Some(2)); // 2 hours
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_get_genie_hours() {
        assert_eq!(get_genie_hours(0), 0); // 0 seconds → 0 hours
        assert_eq!(get_genie_hours(100), 1); // < 3600 → 1
        assert_eq!(get_genie_hours(3599), 1); // < 3600 → 1
        assert_eq!(get_genie_hours(3600), 1); // exactly 1 hour
        assert_eq!(get_genie_hours(7200), 2); // 2 hours
        assert_eq!(get_genie_hours(5400), 2); // 1.5 hours → round to 2
        assert_eq!(get_genie_hours(1_296_000), 360); // 360 hours (15 days)
    }

    #[test]
    fn test_genie_use_spirit_response_format() {
        //   Packet result(WIZ_GENIE, uint8(GenieUseSpiringPotion));
        //   result << uint8(GenieUseSpiringPotion) << GetGenieTime();
        // Wire: [u8(1)] [u8(1)] [u16 hours]
        let hours: u16 = 360; // 15 days
        let mut resp = Packet::new(Opcode::WizGenie as u8);
        resp.write_u8(GENIE_USE_SPIRING_POTION);
        resp.write_u8(GENIE_USE_SPIRING_POTION);
        resp.write_u16(hours);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(1)); // sub-command
        assert_eq!(r.read_u8(), Some(1)); // repeated
        assert_eq!(r.read_u16(), Some(360)); // hours
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_genie_abs_to_db_zero() {
        assert_eq!(genie_abs_to_db(0), 0);
    }

    #[test]
    fn test_genie_abs_to_db_expired() {
        // Expired timestamp (in the past) → 0
        assert_eq!(genie_abs_to_db(1), 0);
    }

    #[test]
    fn test_genie_abs_to_db_future() {
        let future = now_secs() + 3600;
        let db_val = genie_abs_to_db(future);
        assert_eq!(db_val, future as i32);
    }

    #[test]
    fn test_genie_remaining_from_abs() {
        let now = now_secs();
        assert_eq!(genie_remaining_from_abs(0), 0);
        assert_eq!(genie_remaining_from_abs(1), 0); // expired
        let future = now + 7200;
        let remaining = genie_remaining_from_abs(future);
        // Should be approximately 7200 (within 2 seconds)
        assert!((7198..=7202).contains(&remaining));
    }

    // ── Sprint 953: Additional coverage ──────────────────────────────

    /// Genie spirit potion item IDs.
    #[test]
    fn test_genie_item_ids() {
        assert_eq!(GENIE_ITEM_IDS.len(), 3);
        assert_eq!(GENIE_ITEM_IDS[0], 810305000);
        assert_eq!(GENIE_ITEM_IDS[1], 810378000);
        assert_eq!(GENIE_ITEM_IDS[2], 900772000);
    }

    /// Genie hours per potion is 360 (15 days).
    #[test]
    fn test_genie_hours_per_potion() {
        assert_eq!(GENIE_HOURS_PER_POTION, 360);
        assert_eq!(GENIE_HOURS_PER_POTION * 3600, 1_296_000); // in seconds
    }

    /// Every legal v2615 skill count survives without truncation.
    #[test]
    fn test_genie_options_size() {
        assert_eq!(GENIE_OPTIONS_DEFAULT_SIZE, 46);
        assert_eq!(GENIE_OPTIONS_MAX_SIZE, 174);
        assert!(valid_options(&default_options()));
        for count in 0u16..=32 {
            let mut blob = count.to_le_bytes().to_vec();
            for slot in 0..count {
                blob.extend_from_slice(&(100001u32 + u32::from(slot) * 1000000).to_le_bytes());
            }
            blob.extend_from_slice(&default_options()[2..]);
            assert!(valid_options(&blob));
            assert!(!valid_options(&blob[..blob.len() - 1]));
        }
        assert!(!valid_options(&[0; 62]));
    }

    /// get_genie_hours: 0→0, <3600→1, 3600→1, 7200→2.
    #[test]
    fn test_get_genie_hours_boundary() {
        assert_eq!(get_genie_hours(0), 0);
        assert_eq!(get_genie_hours(1), 1);
        assert_eq!(get_genie_hours(3599), 1);
        assert_eq!(get_genie_hours(3600), 1);
        assert_eq!(get_genie_hours(7200), 2);
    }

    // ── Review fixes ─────────────────────────────────────────────────

    fn drain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Arc<Packet>>) -> Vec<Arc<Packet>> {
        let mut out = Vec::new();
        while let Ok(p) = rx.try_recv() {
            out.push(p);
        }
        out
    }

    fn world_with_session() -> (
        Arc<WorldState>,
        tokio::sync::mpsc::UnboundedReceiver<Arc<Packet>>,
    ) {
        let world = Arc::new(WorldState::new());
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        (world, rx)
    }

    #[test]
    fn genie_abs_after_adds_to_running_time_or_now() {
        let now = 1_790_000_000u32;
        assert_eq!(genie_abs_after(0, now, 360), Some(now + 360 * 3600));
        assert_eq!(genie_abs_after(now - 5, now, 1), Some(now + 3600));
        let running = now + 7200;
        assert_eq!(genie_abs_after(running, now, 2), Some(running + 7200));
        assert_eq!(genie_abs_after(0, now, 0), Some(now));
    }

    #[test]
    fn genie_abs_after_enforces_cap_and_never_wraps() {
        let now = 1_790_000_000u32;
        assert!(genie_abs_after(0, now, GENIE_MAX_HOURS).is_some());
        assert!(genie_abs_after(0, now, GENIE_MAX_HOURS + 1).is_none());
        // 300 potions would have wrapped the i32 DB column before the cap existed.
        let mut abs = 0;
        let mut taken = 0;
        while let Some(next) = genie_abs_after(abs, now, GENIE_HOURS_PER_POTION) {
            abs = next;
            taken += 1;
            assert!(abs <= i32::MAX as u32);
            assert!(get_genie_hours(abs - now) as u32 <= GENIE_MAX_HOURS);
        }
        assert!(taken > 100 && taken < 300, "taken={taken}");
        // u32 arithmetic of the old Lua/GM code wrapped here.
        assert!(genie_abs_after(0, now, u32::MAX).is_none());
    }

    #[test]
    fn grant_genie_hours_updates_session_and_respects_cap() {
        let (world, _rx) = world_with_session();
        let abs = grant_genie_hours(&world, 1, 24).expect("fits");
        assert_eq!(world.with_session(1, |h| h.genie_time_abs), Some(abs));
        assert!((24 * 3600 - 2..=24 * 3600 + 2).contains(&genie_remaining_from_abs(abs)));
        assert!(grant_genie_hours(&world, 1, GENIE_MAX_HOURS).is_none());
        assert_eq!(world.with_session(1, |h| h.genie_time_abs), Some(abs));
        assert!(grant_genie_hours(&world, 99, 1).is_none());
    }

    #[test]
    fn activated_packet_width_follows_client_version() {
        // The v2625 client (sub_B20AB0, live 0xB214F0) reads [u32 id][u8 flag].
        let wide = build_activated_packet(0x1234, true, true);
        assert_eq!(wide.data, vec![1, 7, 0x34, 0x12, 0, 0, 1]);
        // The v2615 client / original server use a u16 id.
        let narrow = build_activated_packet(0x1234, false, false);
        assert_eq!(narrow.data, vec![1, 7, 0x34, 0x12, 0]);
        assert!(genie_id_is_u32(2625) && genie_id_is_u32(2700));
        assert!(!genie_id_is_u32(2615));
    }

    #[test]
    fn activate_sends_one_start_reply_and_deactivate_one_stop_reply() {
        let (world, mut rx) = world_with_session();
        let abs = grant_genie_hours(&world, 1, 3).unwrap();
        drain(&mut rx);

        activate_genie(&world, 1, genie_remaining_from_abs(abs));
        assert_eq!(world.with_session(1, |h| h.genie_active), Some(true));
        let starts: Vec<_> = drain(&mut rx)
            .into_iter()
            .filter(|p| p.opcode == Opcode::WizGenie as u8 && p.data[..2] == [1, 4])
            .collect();
        assert_eq!(starts.len(), 1, "the start reply must not be duplicated");
        assert_eq!(starts[0].data, vec![1, 4, 1, 0, 3, 0]);

        assert!(deactivate_genie(&world, 1));
        assert_eq!(world.with_session(1, |h| h.genie_active), Some(false));
        let stops: Vec<_> = drain(&mut rx)
            .into_iter()
            .filter(|p| p.opcode == Opcode::WizGenie as u8 && p.data[..2] == [1, 5])
            .collect();
        assert_eq!(stops.len(), 1);
        assert_eq!(stops[0].data, vec![1, 5, 1, 0, 3, 0]);

        // Second stop (client + tick + respawn racing) is a no-op.
        assert!(!deactivate_genie(&world, 1));
        assert!(drain(&mut rx).is_empty());
    }

    #[test]
    fn deactivate_after_expiry_reports_zero_hours() {
        let (world, mut rx) = world_with_session();
        world.update_session(1, |h| {
            h.genie_active = true;
            h.genie_time_abs = 5; // long expired
        });
        assert!(deactivate_genie(&world, 1));
        let stop = drain(&mut rx)
            .into_iter()
            .find(|p| p.data.len() >= 2 && p.data[..2] == [1, 5])
            .unwrap();
        assert_eq!(stop.data, vec![1, 5, 1, 0, 0, 0]);
    }

    #[test]
    fn expiry_tick_stops_an_active_expired_genie_once() {
        let (world, mut rx) = world_with_session();
        world.update_session(1, |h| {
            h.genie_active = true;
            h.genie_time_abs = 5;
        });
        check_genie_time_tick(&world, 1, u64::from(now_secs()));
        assert_eq!(world.with_session(1, |h| h.genie_active), Some(false));
        assert_eq!(
            drain(&mut rx)
                .iter()
                .filter(|p| p.data.len() >= 2 && p.data[..2] == [1, 5])
                .count(),
            1
        );
        check_genie_time_tick(&world, 1, u64::from(now_secs()));
        assert!(drain(&mut rx).is_empty());
    }

    #[test]
    fn dead_players_may_only_stop_and_exchange_settings() {
        assert!(allowed_while_dead(GENIE_STOP_HANDLE));
        assert!(allowed_while_dead(GENIE_LOAD_OPTIONS));
        assert!(allowed_while_dead(GENIE_SAVE_OPTIONS));
        assert!(!allowed_while_dead(GENIE_START_HANDLE));
        assert!(!allowed_while_dead(GENIE_USE_SPIRING_POTION));
    }

    #[test]
    fn default_hg_block_matches_the_dll_layout() {
        let b = default_hg_block();
        assert_eq!(b.len(), HG_BLOCK_LEN);
        assert_eq!(&b[..3], &[b'H', b'G', 1]);
        assert_eq!(b[6], 50); // party heal %
        assert!(b[7..].iter().all(|&x| x == 0)); // empty PT code, no monsters
        // The DLL wants opcode + sub + op + block (3 + HG_BLOCK bytes on the wire).
        let mut hook = Packet::new(Opcode::EXT_HOOK_S2C);
        hook.write_u8(EXT_SUB_HGGENIE);
        hook.write_u8(4);
        hook.data.extend_from_slice(&b);
        assert_eq!(hook.data.len(), 2 + HG_BLOCK_LEN);
    }

    #[test]
    fn use_reply_encodes_fail_code_the_client_prints_as_fail() {
        assert_eq!(build_use_reply(GENIE_USE_FAIL).data, vec![1, 1, 0xFB, 0xFF]);
        assert_eq!(build_use_reply(360).data, vec![1, 1, 0x68, 0x01]);
    }

    /// Genie attack sub-commands: MOVE=1, ROTATE=2, MAIN_ATTACK=3, MAGIC=4.
    #[test]
    fn test_genie_attack_sub_commands() {
        assert_eq!(GENIE_MOVE, 1);
        assert_eq!(GENIE_ROTATE, 2);
        assert_eq!(GENIE_MAIN_ATTACK, 3);
        assert_eq!(GENIE_MAGIC, 4);
    }
}
