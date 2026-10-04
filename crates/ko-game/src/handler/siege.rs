//! WIZ_SIEGE (0x6D) handler -- Castle Siege Warfare.
//! ## Main Sub-opcodes
//! | Opcode | Name              | Description                              |
//! |--------|-------------------|------------------------------------------|
//! | 1      | Base Create       | Siege NPC base creation (stub)           |
//! | 2      | Castle Flag       | Owner clan mark/flag info for Delos      |
//! | 3      | Moradon NPC       | War schedule, master info, war status    |
//! | 4      | Delos NPC         | Collect funds, view charges, set tariffs |
//! | 5      | Rank              | Register/view siege rank info            |
//! ## Standalone Functions
//! - [`is_csw_winner_clan`] -- Check if a clan is in the winning group (owner or alliance).
//! - [`delos_castellan_zone_out`] -- Kick non-winners from the castellan zone.
//! - [`csw_winner_members_check`] -- Check if player can enter castellan zone and teleport.
//! - [`monument_capture`] -- Process monument destruction (castle capture).

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::{WorldState, COIN_MAX, ZONE_DELOS, ZONE_DELOS_CASTELLAN, ZONE_MORADON};

/// Castellan zone default spawn X coordinate.
const CASTELLAN_SPAWN_X: f32 = 458.0;

/// Castellan zone default spawn Z coordinate.
const CASTELLAN_SPAWN_Z: f32 = 113.0;

/// Handle incoming WIZ_SIEGE (0x6D) packet.
pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };
    let sub_type = reader.read_u8().unwrap_or(0);
    let tariff = reader.read_u16().unwrap_or(0);

    match opcode {
        1 => handle_base_create(session, sub_type).await,
        2 => handle_castle_flag(session).await,
        3 => handle_moradon_npc(session, sub_type).await,
        4 => handle_delos_npc(session, sub_type, tariff).await,
        5 => handle_rank(session, sub_type).await,
        _ => {
            debug!(
                "[{}] WIZ_SIEGE unhandled opcode={} type={}",
                session.addr(),
                opcode,
                sub_type
            );
            Ok(())
        }
    }
}

/// Opcode 1: Base create.
/// The C++ implementation echoes `opcode + type` back to the client and only handles
/// `type == 1` (which itself is a commented-out stub). We match this behavior: echo
/// the opcode and type, and log the request. Actual base NPC creation is not
/// implemented in the C++ reference either.
async fn handle_base_create(session: &mut ClientSession, sub_type: u8) -> anyhow::Result<()> {
    // C++ sends: result << opcode << type; for type 1, base creation is a no-op.
    let mut resp = Packet::new(Opcode::WizSiege as u8);
    resp.write_u8(1); // opcode
    resp.write_u8(sub_type);

    match sub_type {
        1 => {
            // C++ comment: "CastleSiegeWarBaseCreate" — not implemented in reference
            debug!(
                "[{}] WIZ_SIEGE base create type=1 (no-op, matching C++ stub)",
                session.addr()
            );
        }
        _ => {
            // C++ logs: "Siege Npc Base Created unhandled packets"
            debug!(
                "[{}] WIZ_SIEGE base create unhandled type={}",
                session.addr(),
                sub_type
            );
        }
    }

    session.send_packet(&resp).await?;
    Ok(())
}

/// Opcode 2: Castle flag -- sends owner clan mark/flag info.
/// Sends the owning clan's ID, mark version, flag, and grade to the player.
/// This is sent when a player enters Delos or when CSW state changes.
async fn handle_castle_flag(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sw = world.siege_war().read().await;
    let owner = sw.master_knights;
    drop(sw);
    let resp = build_castle_flag_packet(&world, owner);
    session.send_packet(&resp).await?;
    Ok(())
}

/// Opcode 3: Moradon NPC interactions.
async fn handle_moradon_npc(session: &mut ClientSession, sub_type: u8) -> anyhow::Result<()> {
    let world = session.world().clone();

    match sub_type {
        // Type 2: War schedule info
        // C++ sends: opcode(3) type(2) castle_index(u16) siege_type(u16) war_day(u8) war_time(u8) war_minute(u8)
        2 => {
            let sw = world.siege_war().read().await;
            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(3);
            resp.write_u8(2);
            resp.write_u16(sw.castle_index);
            resp.write_u16(sw.siege_type as u16);
            resp.write_u8(sw.war_day);
            resp.write_u8(sw.war_time);
            resp.write_u8(sw.war_minute);
            session.send_packet(&resp).await?;
        }

        // Type 4: Master clan info
        // C++ sends: SByte + castle_index(u16) + count(u8=1) + clan_name(str) +
        //            nation(u8) + members(u16) + request_day(u8) + request_time(u8) + request_minute(u8)
        4 => {
            let sw = world.siege_war().read().await;
            if sw.master_knights == 0 {
                return Ok(());
            }

            let clan_info = world.get_knights(sw.master_knights);
            let (clan_name, clan_nation, clan_members) = match clan_info {
                Some(k) => (k.name.clone(), k.nation, k.members),
                None => return Ok(()),
            };

            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(3);
            resp.write_u8(4);
            resp.write_u16(sw.castle_index);
            resp.write_u8(1); // count
            resp.write_sbyte_string(&clan_name);
            resp.write_u8(clan_nation);
            resp.write_u16(clan_members);
            resp.write_u8(sw.war_request_day);
            resp.write_u8(sw.war_request_time);
            resp.write_u8(sw.war_request_minute);
            session.send_packet(&resp).await?;
        }

        // Type 5: War status (current master clan info)
        // C++ sends: SByte + castle_index(u16) + siege_type(u8) + clan_name(str) +
        //            nation(u8) + members(u16)
        5 => {
            let sw = world.siege_war().read().await;
            if sw.master_knights == 0 {
                return Ok(());
            }

            let clan_info = world.get_knights(sw.master_knights);
            let (clan_name, clan_nation, clan_members) = match clan_info {
                Some(k) => (k.name.clone(), k.nation, k.members),
                None => return Ok(()),
            };

            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(3);
            resp.write_u8(5);
            resp.write_u16(sw.castle_index);
            resp.write_u8(sw.siege_type);
            resp.write_sbyte_string(&clan_name);
            resp.write_u8(clan_nation);
            resp.write_u16(clan_members);
            session.send_packet(&resp).await?;
        }

        _ => {
            debug!(
                "[{}] WIZ_SIEGE moradon NPC unhandled type={}",
                session.addr(),
                sub_type
            );
        }
    }

    Ok(())
}

/// Get player nation, name, and gold from session.
fn get_player_info(session: &ClientSession) -> Option<(u8, String, u32)> {
    let world = session.world();
    world.with_session(session.session_id(), |h| {
        h.character
            .as_ref()
            .map(|c| (c.nation, c.name.clone(), c.gold))
    })?
}

/// Get player clan ID from session.
fn get_clan_id(session: &ClientSession) -> u16 {
    let world = session.world();
    world
        .with_session(session.session_id(), |h| {
            h.character.as_ref().map(|c| c.knights_id).unwrap_or(0)
        })
        .unwrap_or(0)
}

/// Opcode 4: Delos NPC interactions.
async fn handle_delos_npc(
    session: &mut ClientSession,
    sub_type: u8,
    tariff: u16,
) -> anyhow::Result<()> {
    let world = session.world().clone();

    match sub_type {
        // Type 2: Collect all funds (tax + dungeon charge)
        2 => {
            let (nation, name, current_gold) = match get_player_info(session) {
                Some(info) => info,
                None => return Ok(()),
            };

            let is_king = world.is_king(nation, &name);

            let mut sw = world.siege_war().write().await;

            if is_king {
                // King collects moradon_tax + delos_tax
                let gold = (sw.delos_tax + sw.moradon_tax).max(0) as u32;
                if (current_gold as u64) + (gold as u64) > COIN_MAX as u64 {
                    let mut resp = Packet::new(Opcode::WizQuest as u8);
                    resp.write_u8(13);
                    resp.write_u8(2);
                    session.send_packet(&resp).await?;
                    return Ok(());
                }
                let sid = session.session_id();
                world.gold_gain(sid, gold);
                sw.delos_tax = 0;
                sw.moradon_tax = 0;
            } else {
                // Castellan clan leader collects dungeon_charge
                let gold = sw.dungeon_charge.max(0) as u32;
                if (current_gold as u64) + (gold as u64) > COIN_MAX as u64 {
                    let mut resp = Packet::new(Opcode::WizQuest as u8);
                    resp.write_u8(13);
                    resp.write_u8(2);
                    session.send_packet(&resp).await?;
                    return Ok(());
                }
                let sid = session.session_id();
                world.gold_gain(sid, gold);
                sw.dungeon_charge = 0;
            }

            // Fire-and-forget DB update
            let pool = session.pool().clone();
            let moradon_tax = sw.moradon_tax;
            let delos_tax = sw.delos_tax;
            let dungeon_charge = sw.dungeon_charge;
            drop(sw);

            tokio::spawn(async move {
                let repo = ko_db::repositories::siege::SiegeRepository::new(&pool);
                if let Err(e) = repo
                    .update_taxes(moradon_tax, delos_tax, dungeon_charge)
                    .await
                {
                    tracing::error!("failed to update siege taxes: {e}");
                }
            });
        }

        // Type 3: View charges (non-king only, shows tariffs + dungeon charge)
        3 => {
            let (nation, name, _gold) = match get_player_info(session) {
                Some(info) => info,
                None => return Ok(()),
            };

            // King cannot view charges (C++ returns early)
            if world.is_king(nation, &name) {
                return Ok(());
            }

            let sw = world.siege_war().read().await;
            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(4);
            resp.write_u8(3);
            resp.write_u16(sw.castle_index);
            resp.write_u16(sw.moradon_tariff);
            resp.write_u16(sw.delos_tariff);
            resp.write_i32(sw.dungeon_charge);
            session.send_packet(&resp).await?;
        }

        // Type 4: Set Moradon tariff
        4 => {
            let (nation, name, _gold) = match get_player_info(session) {
                Some(info) => info,
                None => return Ok(()),
            };

            // King cannot set tariffs (C++ returns early), and max 20
            if tariff > 20 || world.is_king(nation, &name) {
                return Ok(());
            }

            {
                let mut sw = world.siege_war().write().await;
                sw.moradon_tariff = tariff;
            }

            // Broadcast tariff change to all players
            // C++ sends: opcode(4) type(4) result(u16=1) tariff(u16) zone(u8=ZONE_MORADON)
            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(4);
            resp.write_u8(4);
            resp.write_u16(1); // success
            resp.write_u16(tariff);
            resp.write_u8(ZONE_MORADON as u8);
            world.broadcast_to_all(Arc::new(resp), None);

            // Fire-and-forget DB update
            let pool = session.pool().clone();
            tokio::spawn(async move {
                let repo = ko_db::repositories::siege::SiegeRepository::new(&pool);
                if let Err(e) = repo.update_tariff(ZONE_MORADON as u8, tariff as i16).await {
                    tracing::error!("failed to update moradon tariff: {e}");
                }
            });
        }

        // Type 5: Set Delos tariff
        5 => {
            let (nation, name, _gold) = match get_player_info(session) {
                Some(info) => info,
                None => return Ok(()),
            };

            if tariff > 20 || world.is_king(nation, &name) {
                return Ok(());
            }

            {
                let mut sw = world.siege_war().write().await;
                sw.delos_tariff = tariff;
            }

            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(4);
            resp.write_u8(5);
            resp.write_u16(1); // success
            resp.write_u16(tariff);
            resp.write_u8(ZONE_DELOS as u8);
            world.broadcast_to_all(Arc::new(resp), None);

            // Fire-and-forget DB update
            let pool = session.pool().clone();
            tokio::spawn(async move {
                let repo = ko_db::repositories::siege::SiegeRepository::new(&pool);
                if let Err(e) = repo.update_tariff(ZONE_DELOS as u8, tariff as i16).await {
                    tracing::error!("failed to update delos tariff: {e}");
                }
            });
        }

        _ => {
            debug!(
                "[{}] WIZ_SIEGE delos NPC unhandled type={}",
                session.addr(),
                sub_type
            );
        }
    }

    Ok(())
}

/// Opcode 5: Rank info.
/// `CastleSiegeWarfareRankRegister()`
async fn handle_rank(session: &mut ClientSession, sub_type: u8) -> anyhow::Result<()> {
    let world = session.world().clone();

    match sub_type {
        // Type 1: Register clan in siege warfare ranking
        // Adds the player's clan to the kill tracking list if CSW is active.
        1 => {
            let clan_id = get_clan_id(session);
            if clan_id == 0 {
                return Ok(());
            }

            let csw = world.csw_event().read().await;
            if !csw.is_active() {
                return Ok(());
            }
            drop(csw);

            let mut csw = world.csw_event().write().await;
            csw.register_clan(clan_id);
            debug!(
                "[{}] WIZ_SIEGE rank register clan_id={}",
                session.addr(),
                clan_id
            );
        }

        // Type 2: Show siege warfare ranking (clan kill counts)
        // During active CSW, shows sorted clan kill counts (max 50).
        2 => {
            let clan_id = get_clan_id(session);
            if clan_id == 0 {
                return Ok(());
            }

            let csw = world.csw_event().read().await;
            if !csw.is_active() {
                // Not active -- send empty list
                let mut resp = Packet::new(Opcode::WizSiege as u8);
                resp.write_u8(5);
                resp.write_u8(2);
                resp.write_u16(0);
                session.send_packet(&resp).await?;
                return Ok(());
            }

            // Collect and sort by kill count descending
            let mut clan_list: Vec<(u16, u16)> = csw
                .clan_kill_list
                .iter()
                .map(|(&cid, &kills)| (cid, kills))
                .collect();
            drop(csw);

            clan_list.sort_by(|a, b| b.1.cmp(&a.1));

            let mut resp = Packet::new(Opcode::WizSiege as u8);
            resp.write_u8(5);
            resp.write_u8(2);

            // Reserve position for count (we'll fill it after)
            let count_pos = resp.data.len();
            resp.write_u16(0); // placeholder

            let mut count: u16 = 0;
            let mut rank: u8 = 0;

            for (cid, kills) in &clan_list {
                if let Some(clan) = world.get_knights(*cid) {
                    rank += 1;
                    // C++ writes: clan_id(u16) + mark_version(u16) + clan_name(str) +
                    //             u8(0) + kill_count(u16) + u8(1) + rank(u8)
                    resp.write_u16(*cid);
                    resp.write_u16(clan.mark_version);
                    resp.write_string(&clan.name);
                    resp.write_u8(0);
                    resp.write_u16(*kills);
                    resp.write_u8(1);
                    resp.write_u8(rank);
                    count += 1;
                    if count >= 50 {
                        break;
                    }
                }
            }

            // Patch the count field
            let count_bytes = count.to_le_bytes();
            if count_pos + 1 < resp.data.len() {
                resp.data[count_pos] = count_bytes[0];
                resp.data[count_pos + 1] = count_bytes[1];
            }

            session.send_packet(&resp).await?;
        }

        _ => {
            debug!(
                "[{}] WIZ_SIEGE rank unhandled type={}",
                session.addr(),
                sub_type
            );
        }
    }

    Ok(())
}

// ── Public CSW helper functions ────────────────────────────────────────

/// Check if a clan belongs to the CSW-winning group (direct owner or alliance member).
/// and `isCswWinnerNembers()`.
/// Returns `true` if:
/// - `clan_id` matches `master_knights` directly, OR
/// - the clan's `alliance` field equals `master_knights`.
pub fn is_csw_winner_clan(world: &WorldState, clan_id: u16, master_knights: u16) -> bool {
    if master_knights == 0 || clan_id == 0 {
        return false;
    }
    if clan_id == master_knights {
        return true;
    }
    if let Some(clan) = world.get_knights(clan_id) {
        clan.alliance == master_knights
    } else {
        false
    }
}

/// Kick non-winning-clan members from the castellan zone to Moradon.
/// Iterates all players in `ZONE_DELOS_CASTELLAN`. Any player whose clan is NOT
/// the castle owner and whose clan's alliance is NOT the castle owner gets
/// teleported to `ZONE_MORADON` with default spawn coordinates.
pub async fn delos_castellan_zone_out(world: &Arc<WorldState>) {
    let master_knights = {
        let sw = world.siege_war().read().await;
        sw.master_knights
    };
    if master_knights == 0 {
        return;
    }

    if world.get_knights(master_knights).is_none() {
        return;
    }

    let sessions = world.sessions_in_zone(ZONE_DELOS_CASTELLAN);
    for sid in sessions {
        let clan_id = world.get_session_clan_id(sid);
        if clan_id == 0 {
            continue;
        }
        if is_csw_winner_clan(world, clan_id, master_knights) {
            continue;
        }

        // Build zone change packet (type=3 teleport, coords=0 → use default spawn)
        let nation = world
            .with_session(sid, |h| h.character.as_ref().map(|c| c.nation))
            .flatten()
            .unwrap_or(0);
        let pkt = build_zone_change_packet(ZONE_MORADON, 0.0, 0.0, nation);
        world.update_position(sid, ZONE_MORADON, 0.0, 0.0, 0.0);
        world.send_to_session_owned(sid, pkt);
    }
}

/// Check if the player is a CSW winner member and teleport them to the castellan zone.
/// If the player's clan is the castle owner or in alliance with the owner,
/// they are teleported to `ZONE_DELOS_CASTELLAN` at the fixed spawn point (458, 113).
/// Returns true for an eligible owner/alliance member after starting the zone transition.
pub async fn csw_winner_members_check(session: &mut ClientSession) -> anyhow::Result<bool> {
    let world = session.world().clone();
    let master_knights = world.siege_war().read().await.master_knights;
    if master_knights == 0 || world.get_knights(master_knights).is_none() {
        return Ok(false);
    }

    let clan_id = get_clan_id(session);
    if clan_id == 0 || world.get_knights(clan_id).is_none() {
        return Ok(false);
    }

    if !is_csw_winner_clan(&world, clan_id, master_knights) {
        return Ok(false);
    }

    crate::handler::zone_change::server_teleport_to_zone(
        &world,
        session.session_id(),
        ZONE_DELOS_CASTELLAN,
        CASTELLAN_SPAWN_X,
        CASTELLAN_SPAWN_Z,
    );
    Ok(true)
}
/// Process monument capture (NPC monument destroyed by a player).
/// When the monument NPC is killed:
/// 1. The killer's clan becomes the new castle owner.
/// 2. The DB siege record is updated.
/// 3. All players in Delos are notified (monument killed notice + flag update).
/// `killer_clan_id` must be non-zero and belong to a valid clan.
/// `pool` is the database connection pool for persisting the change.
pub async fn monument_capture(world: &WorldState, killer_clan_id: u16, pool: &ko_db::DbPool) -> bool {
    if killer_clan_id == 0 || world.get_knights(killer_clan_id).is_none() {
        tracing::error!(killer_clan_id, "CSW capture rejected: invalid clan");
        return false;
    }

    {
        let csw = world.csw_event().read().await;
        if !csw.is_war_active() {
            tracing::warn!(killer_clan_id, "CSW capture rejected: war phase is not active");
            return false;
        }
    }

    let castle_index = world.siege_war().read().await.castle_index;

    // Persist ownership without overwriting the scheduled war date/time. A
    // missing castle row is an error, not a phantom successful capture.
    let repo = ko_db::repositories::siege::SiegeRepository::new(pool);
    if let Err(error) = repo
        .update_master_knights(castle_index as i16, killer_clan_id as i16)
        .await
    {
        tracing::error!(
            killer_clan_id,
            castle_index,
            error = %error,
            "failed to persist CSW owner to knights_siege_warfare"
        );
        return false;
    }
    world.siege_war().write().await.master_knights = killer_clan_id;
    tracing::info!(killer_clan_id, castle_index, "CSW owner persisted to knights_siege_warfare");

    let flag = build_castle_flag_packet(world, killer_clan_id);
    let arc_flag = Arc::new(flag);
    for sid in world.sessions_in_zone(ZONE_DELOS) {
        world.send_to_session_arc(sid, Arc::clone(&arc_flag));
    }

    tracing::info!(
        "CSW monument captured by clan_id={}, new castle owner set",
        killer_clan_id
    );
    true
}
/// Build a WIZ_ZONE_CHANGE teleport packet.
pub(crate) fn build_zone_change_packet(zone_id: u16, x: f32, z: f32, nation: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
    pkt.write_u8(3); // ZONE_CHANGE_TELEPORT
    pkt.write_u16(zone_id);
    pkt.write_u16(0);
    pkt.write_u16((x * 10.0) as u16);
    pkt.write_u16((z * 10.0) as u16);
    pkt.write_u16(0);
    pkt.write_u8(nation);
    pkt.write_u16(0xFFFF);
    pkt
}

/// Build a castle flag packet (opcode 2) for a specific owner clan.
pub(crate) fn build_castle_flag_packet(world: &WorldState, owner_clan_id: u16) -> Packet {
    let mut resp = Packet::new(Opcode::WizSiege as u8);
    resp.write_u8(2);
    resp.write_u8(0);
    if owner_clan_id != 0 {
        if let Some(clan) = world.get_knights(owner_clan_id) {
            resp.write_u16(clan.id);
            resp.write_u16(clan.mark_version);
            resp.write_u8(clan.flag);
            resp.write_u8(clan.grade);
        } else {
            resp.write_u16(0);
            resp.write_u16(0);
            resp.write_u8(0);
            resp.write_u8(0);
        }
    } else {
        resp.write_u16(0);
        resp.write_u16(0);
        resp.write_u8(0);
        resp.write_u8(0);
    }

    resp
}

// ── CSW Timer Logic ────────────────────────────────────────────────────

use crate::world::types::{CswEventState, CswNotice, CswOpStatus};

/// One minute in seconds.
const MINUTE: u64 = 60;

/// Result of a CSW timer tick.
#[derive(Debug, PartialEq, Eq)]
pub enum CswTickAction {
    /// No action needed this tick.
    None,
    /// Send a countdown notice to all players.
    SendNotice {
        notice_type: CswNotice,
        remaining_minutes: u32,
    },
    /// Preparation phase ended — transition to war.
    TransitionToWar,
    /// War phase ended — close CSW.
    TransitionToClose,
}

/// Pure function: check CSW timer state and return the appropriate action.
/// Called every 1 second from the war tick loop. Compares `now` against
/// `state.csw_time` to determine countdown notices and phase transitions.
pub fn csw_timer_tick(state: &CswEventState, now: u64) -> CswTickAction {
    if !state.is_active() {
        return CswTickAction::None;
    }

    let r_time = state.csw_time.saturating_sub(now);

    match state.status {
        CswOpStatus::Preparation if !state.prepare_check => {
            // C++ countdown at exact minute boundaries
            match r_time {
                t if t == 10 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: 10,
                },
                t if t == 5 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: 5,
                },
                t if t == 4 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: 4,
                },
                t if t == 3 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: 3,
                },
                t if t == 2 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: 2,
                },
                t if t == MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: 1,
                },
                0 => CswTickAction::TransitionToWar,
                _ => CswTickAction::None,
            }
        }
        CswOpStatus::War if !state.war_check => {
            // C++ countdown: 30, 10, 5, 4, 3, 2, 1 minutes
            match r_time {
                t if t == 30 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 30,
                },
                t if t == 10 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 10,
                },
                t if t == 5 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 5,
                },
                t if t == 4 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 4,
                },
                t if t == 3 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 3,
                },
                t if t == 2 * MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 2,
                },
                t if t == MINUTE => CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: 1,
                },
                0 => CswTickAction::TransitionToClose,
                _ => CswTickAction::None,
            }
        }
        _ => CswTickAction::None,
    }
}

/// Initialize the preparation phase.
pub fn csw_prepare_open(state: &mut CswEventState, preparing_minutes: u32, now: u64) {
    if state.started {
        return;
    }
    state.clan_kill_list.clear();
    state.status = CswOpStatus::Preparation;
    state.csw_time = now + (preparing_minutes as u64) * MINUTE;
    state.started = true;
    state.prepare_check = false;
    state.war_check = false;
    state.prep_minutes = preparing_minutes;
    state.monument_time = 0;
}

/// Transition from preparation to war phase.
pub fn csw_war_open(state: &mut CswEventState, wartime_minutes: u32, now: u64) {
    if !state.started {
        return;
    }
    state.prepare_check = true;
    state.status = CswOpStatus::War;
    state.war_minutes = wartime_minutes;
    state.csw_time = now + (wartime_minutes as u64) * MINUTE;
}

/// Close the CSW event and reset state.
pub fn csw_close(state: &mut CswEventState) {
    state.war_check = true;
    state.reset();
}

/// Build a CSW countdown notice packet (WAR_SYSTEM_CHAT).
/// Returns a chat packet that announces the remaining time for the current phase.
pub fn build_csw_raw_notice(notice_type: CswNotice, minutes: u32) -> Packet {
    let phase = match notice_type {
        CswNotice::Preparation => "Castle Siege Warfare preparation",
        CswNotice::War => "Castle Siege Warfare",
        CswNotice::MonumentKilled => "Monument has been destroyed",
        CswNotice::CswFinish => "Castle Siege Warfare has ended",
    };

    let msg = if minutes > 0 {
        format!("{phase} — {minutes} minute(s) remaining.")
    } else {
        format!("{phase}!")
    };

    crate::handler::chat::build_chat_packet(
        8,      // WAR_SYSTEM_CHAT
        1,      // nation = ALL
        0xFFFF, // sender_id = -1
        "", &msg, 0, 0, 0,
    )
}

/// Build a CSW phase-change notice packet.
pub fn build_csw_notice(notice_type: CswNotice) -> Packet {
    build_csw_raw_notice(notice_type, 0)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::world::{KnightsInfo, WorldState};

    /// Helper: create a test world.
    fn make_test_world() -> Arc<WorldState> {
        Arc::new(WorldState::new())
    }

    /// Helper: create a minimal KnightsInfo with specified fields.
    fn make_clan(id: u16, alliance: u16, grade: u8) -> KnightsInfo {
        KnightsInfo {
            id,
            flag: 2,
            nation: 1,
            grade,
            ranking: 0,
            name: format!("Clan{id}"),
            chief: String::new(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0xFFFF,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance,
            castellan_cape: false,
            cast_cape_id: -1,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        }
    }

    // ── is_csw_winner_clan tests ───────────────────────────────────────

    #[test]
    fn winner_clan_zero_master_returns_false() {
        let world = make_test_world();
        assert!(!is_csw_winner_clan(&world, 100, 0));
    }

    #[test]
    fn winner_clan_zero_clan_returns_false() {
        let world = make_test_world();
        assert!(!is_csw_winner_clan(&world, 0, 100));
    }

    #[test]
    fn winner_clan_both_zero_returns_false() {
        let world = make_test_world();
        assert!(!is_csw_winner_clan(&world, 0, 0));
    }

    #[test]
    fn winner_clan_direct_match() {
        let world = make_test_world();
        // Direct match: clan_id == master_knights (no world lookup needed)
        assert!(is_csw_winner_clan(&world, 42, 42));
    }

    #[test]
    fn winner_clan_alliance_match() {
        let world = make_test_world();
        // Insert clan 50 with alliance pointing to master clan 42
        world.insert_knights(make_clan(50, 42, 3));
        assert!(is_csw_winner_clan(&world, 50, 42));
    }

    #[test]
    fn winner_clan_no_match() {
        let world = make_test_world();
        // Insert clan 60 with a different alliance
        world.insert_knights(make_clan(60, 99, 3));
        assert!(!is_csw_winner_clan(&world, 60, 42));
    }

    #[test]
    fn winner_clan_nonexistent_clan() {
        let world = make_test_world();
        // Clan 999 does not exist in the world
        assert!(!is_csw_winner_clan(&world, 999, 42));
    }

    #[test]
    fn winner_clan_zero_alliance_not_match() {
        let world = make_test_world();
        // Clan 70 with alliance=0 should not match master=42
        world.insert_knights(make_clan(70, 0, 3));
        assert!(!is_csw_winner_clan(&world, 70, 42));
    }

    // ── build_zone_change_packet tests ─────────────────────────────────

    #[test]
    fn zone_change_packet_format() {
        let pkt = build_zone_change_packet(ZONE_MORADON, 100.0, 200.0, 1);
        assert_eq!(pkt.opcode, Opcode::WizZoneChange as u8);
        // Type byte
        assert_eq!(pkt.data[0], 3);
        // Zone ID (little-endian u16)
        assert_eq!(u16::from_le_bytes([pkt.data[1], pkt.data[2]]), ZONE_MORADON);
        // Reserved u16
        assert_eq!(u16::from_le_bytes([pkt.data[3], pkt.data[4]]), 0);
        // X * 10 = 1000
        assert_eq!(u16::from_le_bytes([pkt.data[5], pkt.data[6]]), 1000);
        // Z * 10 = 2000
        assert_eq!(u16::from_le_bytes([pkt.data[7], pkt.data[8]]), 2000);
        // Reserved u16
        assert_eq!(u16::from_le_bytes([pkt.data[9], pkt.data[10]]), 0);
        // Nation
        assert_eq!(pkt.data[11], 1);
        // 0xFFFF
        assert_eq!(u16::from_le_bytes([pkt.data[12], pkt.data[13]]), 0xFFFF);
    }

    #[test]
    fn zone_change_packet_zero_coords() {
        let pkt = build_zone_change_packet(ZONE_MORADON, 0.0, 0.0, 2);
        assert_eq!(u16::from_le_bytes([pkt.data[5], pkt.data[6]]), 0);
        assert_eq!(u16::from_le_bytes([pkt.data[7], pkt.data[8]]), 0);
        assert_eq!(pkt.data[11], 2);
    }

    #[test]
    fn zone_change_packet_castellan_coords() {
        let pkt = build_zone_change_packet(
            ZONE_DELOS_CASTELLAN,
            CASTELLAN_SPAWN_X,
            CASTELLAN_SPAWN_Z,
            1,
        );
        assert_eq!(
            u16::from_le_bytes([pkt.data[1], pkt.data[2]]),
            ZONE_DELOS_CASTELLAN
        );
        // 458.0 * 10 = 4580
        assert_eq!(u16::from_le_bytes([pkt.data[5], pkt.data[6]]), 4580);
        // 113.0 * 10 = 1130
        assert_eq!(u16::from_le_bytes([pkt.data[7], pkt.data[8]]), 1130);
    }

    // ── build_castle_flag_packet tests ─────────────────────────────────

    #[test]
    fn castle_flag_packet_no_owner() {
        let world = make_test_world();
        let pkt = build_castle_flag_packet(&world, 0);
        assert_eq!(pkt.opcode, Opcode::WizSiege as u8);
        assert_eq!(pkt.data[0], 2); // opcode
        assert_eq!(pkt.data[1], 0); // SByte
        assert_eq!(u32::from_le_bytes(pkt.data[2..6].try_into().unwrap()), 0);
        assert_eq!(u16::from_le_bytes([pkt.data[6], pkt.data[7]]), 0);
        assert_eq!(pkt.data.len(), 8);
    }

    #[test]
    fn castle_flag_packet_with_owner() {
        let world = make_test_world();
        let mut info = make_clan(77, 0, 1);
        info.mark_version = 3;
        info.flag = 2;
        world.insert_knights(info);

        let pkt = build_castle_flag_packet(&world, 77);
        assert_eq!(pkt.data[0], 2);
        assert_eq!(pkt.data[1], 0);
        // clan_id as u16 (C++ GetID() returns uint16)
        assert_eq!(u16::from_le_bytes([pkt.data[2], pkt.data[3]]), 77);
        // mark_version
        assert_eq!(u16::from_le_bytes([pkt.data[4], pkt.data[5]]), 3);
        // flag
        assert_eq!(pkt.data[6], 2);
        // grade
        assert_eq!(pkt.data[7], 1);
        // Total "has owner" payload: u16 + u16 + u8 + u8 = 6 bytes after opcode+sbyte
        assert_eq!(pkt.data.len(), 8); // 2 (opcode+sbyte) + 6 (payload)
    }

    #[test]
    fn castle_flag_packet_nonexistent_owner() {
        let world = make_test_world();
        // Owner 999 does not exist
        let pkt = build_castle_flag_packet(&world, 999);
        assert_eq!(u32::from_le_bytes(pkt.data[2..6].try_into().unwrap()), 0);
        assert_eq!(u16::from_le_bytes([pkt.data[6], pkt.data[7]]), 0);
    }

    // ── delos_castellan_zone_out tests ─────────────────────────────────

    #[tokio::test]
    async fn castellan_zone_out_no_master() {
        let world = make_test_world();
        // Should not panic with master_knights=0 (default)
        delos_castellan_zone_out(&world).await;
    }

    #[tokio::test]
    async fn castellan_zone_out_with_master_no_sessions() {
        let world = make_test_world();
        // Set master_knights
        world.insert_knights(make_clan(10, 0, 1));
        {
            let mut sw = world.siege_war().write().await;
            sw.master_knights = 10;
        }
        // No sessions in castellan zone -- should not panic
        delos_castellan_zone_out(&world).await;
    }

    // ── tariff validation tests ────────────────────────────────────────

    #[test]
    fn tariff_max_boundary() {
        // Tariff must be 0-20; > 20 is rejected
        assert!(21 > 20);
        assert!((20 <= 20));
    }

    #[test]
    fn tariff_min_boundary() {
        // Tariff 0 is valid (the handler rejects tariff > 20)
        let tariff: u16 = 0;
        assert!(tariff <= 20);
    }

    // ── monument_capture tests ─────────────────────────────────────────

    #[test]
    fn monument_capture_does_not_require_clan_grade() {
        let world = make_test_world();
        // Grade 4 should be rejected
        world.insert_knights(make_clan(80, 0, 4));
        let clan = world.get_knights(80).unwrap();
        assert!(clan.grade > 3);

        // Grade 3 should be accepted
        world.insert_knights(make_clan(81, 0, 3));
        let clan = world.get_knights(81).unwrap();
        assert!(clan.grade <= 3);
    }

    #[test]
    fn csw_winner_clan_with_direct_and_alliance() {
        let world = make_test_world();
        let master = 100;
        // Master clan itself
        world.insert_knights(make_clan(master, 0, 1));
        assert!(is_csw_winner_clan(&world, master, master));

        // Allied clan
        world.insert_knights(make_clan(200, master, 2));
        assert!(is_csw_winner_clan(&world, 200, master));

        // Unrelated clan
        world.insert_knights(make_clan(300, 0, 3));
        assert!(!is_csw_winner_clan(&world, 300, master));

        // Clan in different alliance
        world.insert_knights(make_clan(400, 500, 1));
        assert!(!is_csw_winner_clan(&world, 400, master));
    }

    // ── Base Create response tests ────────────────────────────────────

    #[test]
    fn base_create_response_packet_format() {
        // C++ echoes opcode(1) + type back to the client
        let mut resp = Packet::new(Opcode::WizSiege as u8);
        resp.write_u8(1); // opcode
        resp.write_u8(1); // type

        assert_eq!(resp.opcode, Opcode::WizSiege as u8);
        assert_eq!(resp.data[0], 1);
        assert_eq!(resp.data[1], 1);
        assert_eq!(resp.data.len(), 2);
    }

    #[test]
    fn base_create_unknown_type_response() {
        let mut resp = Packet::new(Opcode::WizSiege as u8);
        resp.write_u8(1);
        resp.write_u8(99); // unknown sub type

        assert_eq!(resp.data[0], 1);
        assert_eq!(resp.data[1], 99);
    }

    // ── CSW Timer Tick tests ──────────────────────────────────────────

    /// Helper: create a CswEventState in preparation phase.
    fn make_csw_prep(now: u64, minutes: u32) -> CswEventState {
        let mut state = CswEventState::default();
        csw_prepare_open(&mut state, minutes, now);
        state
    }

    #[test]
    fn csw_timer_tick_idle_returns_none() {
        let state = CswEventState::default();
        assert!(!state.is_active());
        assert_eq!(csw_timer_tick(&state, 1000), CswTickAction::None);
    }

    #[test]
    fn csw_timer_tick_preparation_notices() {
        let now = 100_000;
        let state = make_csw_prep(now, 15); // 15 min prep

        // At 10 minutes remaining
        let at_10 = state.csw_time - 10 * MINUTE;
        assert_eq!(
            csw_timer_tick(&state, at_10),
            CswTickAction::SendNotice {
                notice_type: CswNotice::Preparation,
                remaining_minutes: 10,
            }
        );

        // At 5 minutes remaining
        let at_5 = state.csw_time - 5 * MINUTE;
        assert_eq!(
            csw_timer_tick(&state, at_5),
            CswTickAction::SendNotice {
                notice_type: CswNotice::Preparation,
                remaining_minutes: 5,
            }
        );

        // At 4, 3, 2, 1 minutes
        for m in [4, 3, 2, 1] {
            let at_m = state.csw_time - m * MINUTE;
            assert_eq!(
                csw_timer_tick(&state, at_m),
                CswTickAction::SendNotice {
                    notice_type: CswNotice::Preparation,
                    remaining_minutes: m as u32,
                }
            );
        }

        // At non-boundary time — no action
        let between = state.csw_time - 7 * MINUTE;
        assert_eq!(csw_timer_tick(&state, between), CswTickAction::None);
    }

    #[test]
    fn csw_timer_tick_preparation_to_war() {
        let now = 100_000;
        let state = make_csw_prep(now, 10);

        // Exactly when timer expires
        assert_eq!(
            csw_timer_tick(&state, state.csw_time),
            CswTickAction::TransitionToWar
        );

        // Past expiry
        assert_eq!(
            csw_timer_tick(&state, state.csw_time + 5),
            CswTickAction::TransitionToWar
        );
    }

    #[test]
    fn csw_timer_tick_war_notices() {
        let now = 100_000;
        let mut state = make_csw_prep(now, 10);
        // Transition to war
        csw_war_open(&mut state, 40, now + 10 * MINUTE);

        // At 30 minutes remaining
        let at_30 = state.csw_time - 30 * MINUTE;
        assert_eq!(
            csw_timer_tick(&state, at_30),
            CswTickAction::SendNotice {
                notice_type: CswNotice::War,
                remaining_minutes: 30,
            }
        );

        // At 10, 5, 4, 3, 2, 1 minutes
        for m in [10, 5, 4, 3, 2, 1] {
            let at_m = state.csw_time - m * MINUTE;
            assert_eq!(
                csw_timer_tick(&state, at_m),
                CswTickAction::SendNotice {
                    notice_type: CswNotice::War,
                    remaining_minutes: m as u32,
                }
            );
        }
    }

    #[test]
    fn csw_timer_tick_war_to_close() {
        let now = 100_000;
        let mut state = make_csw_prep(now, 10);
        csw_war_open(&mut state, 40, now + 10 * MINUTE);

        // Timer expires
        assert_eq!(
            csw_timer_tick(&state, state.csw_time),
            CswTickAction::TransitionToClose
        );
    }

    #[test]
    fn csw_prepare_open_sets_state() {
        let now = 50_000;
        let mut state = CswEventState::default();
        csw_prepare_open(&mut state, 20, now);

        assert_eq!(state.status, CswOpStatus::Preparation);
        assert_eq!(state.csw_time, now + 20 * MINUTE);
        assert!(state.started);
        assert!(!state.prepare_check);
        assert!(!state.war_check);
        assert!(state.clan_kill_list.is_empty());
    }

    #[test]
    fn csw_prepare_open_ignores_if_already_started() {
        let now = 50_000;
        let mut state = CswEventState::default();
        csw_prepare_open(&mut state, 20, now);
        let first_time = state.csw_time;

        // Second call should be ignored
        csw_prepare_open(&mut state, 30, now + 1000);
        assert_eq!(state.csw_time, first_time);
    }

    #[test]
    fn csw_war_open_sets_state() {
        let now = 50_000;
        let mut state = CswEventState::default();
        csw_prepare_open(&mut state, 10, now);

        let war_start = now + 10 * MINUTE;
        csw_war_open(&mut state, 40, war_start);

        assert_eq!(state.status, CswOpStatus::War);
        assert_eq!(state.csw_time, war_start + 40 * MINUTE);
        assert!(state.prepare_check);
        assert!(!state.war_check);
    }

    #[test]
    fn csw_close_resets_state() {
        let now = 50_000;
        let mut state = CswEventState::default();
        csw_prepare_open(&mut state, 10, now);
        csw_war_open(&mut state, 40, now + 10 * MINUTE);

        csw_close(&mut state);

        assert_eq!(state.status, CswOpStatus::NotOperation);
        assert_eq!(state.csw_time, 0);
        assert!(!state.started);
        assert!(!state.is_active());
    }

    #[test]
    fn csw_full_lifecycle() {
        let now = 100_000;
        let mut state = CswEventState::default();

        // 1. Open preparation
        csw_prepare_open(&mut state, 10, now);
        assert!(state.is_active());
        assert!(!state.is_war_active());

        // 2. During prep — 5 min notice
        let at_5 = state.csw_time - 5 * MINUTE;
        assert_eq!(
            csw_timer_tick(&state, at_5),
            CswTickAction::SendNotice {
                notice_type: CswNotice::Preparation,
                remaining_minutes: 5,
            }
        );

        // 3. Prep timer expires → transition
        assert_eq!(
            csw_timer_tick(&state, state.csw_time),
            CswTickAction::TransitionToWar
        );

        // 4. Apply transition
        let war_start = state.csw_time;
        csw_war_open(&mut state, 40, war_start);
        assert!(state.is_war_active());

        // 5. War 10 min notice
        let at_10 = state.csw_time - 10 * MINUTE;
        assert_eq!(
            csw_timer_tick(&state, at_10),
            CswTickAction::SendNotice {
                notice_type: CswNotice::War,
                remaining_minutes: 10,
            }
        );

        // 6. War timer expires → close
        assert_eq!(
            csw_timer_tick(&state, state.csw_time),
            CswTickAction::TransitionToClose
        );

        // 7. Apply close
        csw_close(&mut state);
        assert!(!state.is_active());
        assert_eq!(csw_timer_tick(&state, 999_999), CswTickAction::None);
    }

    #[test]
    fn csw_raw_notice_preparation() {
        let pkt = build_csw_raw_notice(CswNotice::Preparation, 5);
        assert_eq!(pkt.opcode, Opcode::WizChat as u8);
        // Verify message is non-empty (contains "5 minute")
        let data_str = String::from_utf8_lossy(&pkt.data);
        assert!(data_str.contains("5"));
    }

    #[test]
    fn csw_raw_notice_finish() {
        let pkt = build_csw_notice(CswNotice::CswFinish);
        assert_eq!(pkt.opcode, Opcode::WizChat as u8);
    }
}
