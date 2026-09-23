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

/// Status: genie active.
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

/// Handle WIZ_GENIE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

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
        GENIE_INFO_REQUEST => handle_info_request(session, &mut r).await,
        GENIE_UPDATE_REQUEST => {
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

/// Handle GenieInfoRequest sub-commands.
async fn handle_info_request(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let sub_command = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

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

    // Validate item is a known genie spirit potion
    if !GENIE_ITEM_IDS.contains(&item_id) {
        return Ok(());
    }

    let sid = session.session_id();
    let world = session.world().clone();

    // if (isTrading() || isMerchanting() || isMining() || isFishing()) return;
    if world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    // Check item exists in inventory
    if !world.check_exist_item(sid, item_id, 1) {
        return Ok(());
    }

    // Remove the item from inventory (rob_item now sends WIZ_ITEM_COUNT_CHANGE)
    world.rob_item(sid, item_id, 1);

    // Extend genie time by 360 hours (= 360 * 3600 seconds)
    // Absolute timestamp: if expired, start from now; if active, extend deadline.
    let duration_secs = GENIE_HOURS_PER_POTION * 3600;
    let now = now_secs();
    world.update_session(sid, |h| {
        h.genie_time_abs = h.genie_time_abs.max(now) + duration_secs;
    });

    // Send response:
    //   Packet result(WIZ_GENIE, uint8(GenieUseSpiringPotion));
    //   result << uint8(GenieUseSpiringPotion) << GetGenieTime();
    // Wire: [u8(1)] [u8(1)] [u16 hours]
    let genie_abs = world.with_session(sid, |h| h.genie_time_abs).unwrap_or(0);
    let remaining = genie_remaining_from_abs(genie_abs);
    let hours = get_genie_hours(remaining);
    let mut resp = Packet::new(Opcode::WizGenie as u8);
    resp.write_u8(GENIE_USE_SPIRING_POTION);
    resp.write_u8(GENIE_USE_SPIRING_POTION);
    resp.write_u16(hours);
    session.send_packet(&resp).await?;

    // Persist genie time to DB after potion use to prevent data loss.
    {
        let pool = session.pool().clone();
        let char_name = world
            .get_character_info(sid)
            .map(|ch| ch.name.clone())
            .unwrap_or_default();
        if !char_name.is_empty() {
            let genie_options = world
                .with_session(sid, |h| h.genie_options.clone())
                .unwrap_or_default();
            let db_val = genie_abs_to_db(genie_abs);
            tokio::spawn(async move {
                let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
                if let Err(e) = repo
                    .save_genie_data(&char_name, db_val, &genie_options, 0)
                    .await
                {
                    tracing::warn!("GenieUseSpirit: failed to save genie data: {}", e);
                }
            });
        }
    }

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
    let options = world
        .with_session(sid, |h| h.genie_options.clone())
        .filter(|saved| valid_options(saved))
        .unwrap_or_else(default_options);

    let mut resp = Packet::new(Opcode::WizGenie as u8);
    resp.write_u8(GENIE_INFO_REQUEST);
    resp.write_u8(GENIE_LOAD_OPTIONS);
    resp.data.extend_from_slice(&options);
    session.send_packet(&resp).await?;

    tracing::info!(
        "[{}] WIZ_GENIE: LoadOptions sent ({} bytes)",
        session.addr(),
        options.len()
    );
    Ok(())
}

/// Save genie options from the client.
/// Reads the options blob from the packet and stores it in the session.
async fn handle_save_options(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world();

    // Reject incomplete payloads without destroying the last complete settings.
    let payload = r.read_remaining();
    if !valid_options(payload) {
        warn!(
            "WIZ_GENIE: rejected malformed options: sid={}, bytes={}",
            sid,
            payload.len()
        );
        return Ok(());
    }
    let options = payload.to_vec();

    world.update_session(sid, |h| {
        h.genie_options = options.clone();
        h.genie_loaded = true;
    });

    // Persist immediately. Disconnect/periodic saves remain as a safeguard,
    // but the client expects a settings change to survive a relog right away.
    if let Some(char_name) = world.get_character_info(sid).map(|c| c.name.clone()) {
        let pool = session.pool().clone();
        let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
        repo.save_genie_options(&char_name, &options).await?;
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

    // Set genie active
    world.update_session(sid, |h| {
        h.genie_active = true;
    });

    let hours = get_genie_hours(remaining);

    let mut resp = Packet::new(Opcode::WizGenie as u8);
    resp.write_u8(GENIE_STATUS_ACTIVE);
    resp.write_u8(4);
    resp.write_u16(1);
    resp.write_u16(hours);
    session.send_packet(&resp).await?;

    // Also send the start confirmation
    let mut start_pkt = Packet::new(Opcode::WizGenie as u8);
    start_pkt.write_u8(GENIE_INFO_REQUEST);
    start_pkt.write_u8(GENIE_START_HANDLE);
    start_pkt.write_u16(1);
    start_pkt.write_u16(hours);
    session.send_packet(&start_pkt).await?;

    // Broadcast genie activated to region
    let mut region_pkt = Packet::new(Opcode::WizGenie as u8);
    region_pkt.write_u8(GENIE_INFO_REQUEST);
    region_pkt.write_u8(GENIE_ACTIVATED);
    // Native C++ serialises GetID() as uint16 here.
    region_pkt.write_u16(sid);
    region_pkt.write_u8(1); // active

    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(region_pkt),
            None,
            event_room,
        );
    }

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

    let (was_active, abs) = world
        .with_session(sid, |h| (h.genie_active, h.genie_time_abs))
        .unwrap_or((false, 0));

    if !was_active {
        return Ok(());
    }

    world.update_session(sid, |h| {
        h.genie_active = false;
    });
    let remaining = genie_remaining_from_abs(abs);
    let hours = get_genie_hours(remaining);

    let mut resp = Packet::new(Opcode::WizGenie as u8);
    resp.write_u8(GENIE_INFO_REQUEST);
    resp.write_u8(GENIE_STOP_HANDLE);
    resp.write_u16(1);
    resp.write_u16(hours);
    session.send_packet(&resp).await?;

    // Broadcast genie deactivated to region
    let mut region_pkt = Packet::new(Opcode::WizGenie as u8);
    region_pkt.write_u8(GENIE_INFO_REQUEST);
    region_pkt.write_u8(GENIE_ACTIVATED);
    region_pkt.write_u16(sid);
    region_pkt.write_u8(0); // inactive

    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(region_pkt),
            None,
            event_room,
        );
    }

    // Persist genie time to DB on stop to prevent data loss on crash.
    {
        let pool = session.pool().clone();
        let char_name = world
            .get_character_info(sid)
            .map(|ch| ch.name.clone())
            .unwrap_or_default();
        if !char_name.is_empty() {
            let genie_options = world
                .with_session(sid, |h| h.genie_options.clone())
                .unwrap_or_default();
            let db_val = genie_abs_to_db(abs);
            tokio::spawn(async move {
                let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
                if let Err(e) = repo
                    .save_genie_data(&char_name, db_val, &genie_options, 0)
                    .await
                {
                    tracing::warn!("GenieStop: failed to save genie data: {}", e);
                }
            });
        }
    }

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
        // Already expired or never had time — check if we need to auto-stop
        let was_active = world.with_session(sid, |h| h.genie_active).unwrap_or(false);
        if was_active {
            world.update_session(sid, |h| {
                h.genie_active = false;
            });

            // Send stop response
            let mut stop_pkt = Packet::new(Opcode::WizGenie as u8);
            stop_pkt.write_u8(GENIE_INFO_REQUEST);
            stop_pkt.write_u8(GENIE_STOP_HANDLE);
            stop_pkt.write_u16(1);
            stop_pkt.write_u16(0);
            world.send_to_session_owned(sid, stop_pkt);

            // Broadcast genie deactivated to region
            let mut region_pkt = Packet::new(Opcode::WizGenie as u8);
            region_pkt.write_u8(GENIE_INFO_REQUEST);
            region_pkt.write_u8(GENIE_ACTIVATED);
            region_pkt.write_u16(sid);
            region_pkt.write_u8(0);
            if let Some(pos) = world.get_position(sid) {
                world.broadcast_to_zone(pos.zone_id, Arc::new(region_pkt), None);
            }
        }
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

    /// Genie attack sub-commands: MOVE=1, ROTATE=2, MAIN_ATTACK=3, MAGIC=4.
    #[test]
    fn test_genie_attack_sub_commands() {
        assert_eq!(GENIE_MOVE, 1);
        assert_eq!(GENIE_ROTATE, 2);
        assert_eq!(GENIE_MAIN_ATTACK, 3);
        assert_eq!(GENIE_MAGIC, 4);
    }
}
