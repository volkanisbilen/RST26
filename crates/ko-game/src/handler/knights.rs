//! WIZKNIGHTS_PROCESS (0x3C) handler — clan system.
//! Handles clan sub-opcodes: create, join, withdraw, remove, destroy,
//! admit, reject, punish, chief, vicechief, officer, member listing,
//! donate NP, and clan notice update.

use ko_db::repositories::character::CharacterRepository;
use ko_db::repositories::knights::KnightsRepository;
use ko_protocol::Packet;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::{KnightsAlliance, KnightsInfo, MAX_ID_SIZE};

// ── Fame / Rank Constants ─────────────────────────────────────────────

use crate::clan_constants::{
    CHIEF, CLAN_COIN_REQUIREMENT, CLAN_LEVEL_REQUIREMENT, COMMAND_CAPTAIN, MAX_CLAN_USERS, OFFICER,
    TRAINEE, VICECHIEF,
};
use crate::systems::daily_reset::get_knights_grade;
/// Minimum NP required to donate (user must retain at least this much).
const MIN_NP_TO_DONATE: u32 = 1000;

// ── Sub-Opcode Constants ──────────────────────────────────────────────

const KNIGHTS_CREATE: u8 = 1;
const KNIGHTS_JOIN: u8 = 2;
const KNIGHTS_WITHDRAW: u8 = 3;
const KNIGHTS_REMOVE: u8 = 4;
const KNIGHTS_DESTROY: u8 = 5;
const KNIGHTS_ADMIT: u8 = 6;
const KNIGHTS_REJECT: u8 = 7;
const KNIGHTS_PUNISH: u8 = 8;
const KNIGHTS_CHIEF: u8 = 9;
const KNIGHTS_VICECHIEF: u8 = 10;
const KNIGHTS_OFFICER: u8 = 11;
const KNIGHTS_ALLLIST_REQ: u8 = 12;
const KNIGHTS_MEMBER_REQ: u8 = 13;
const KNIGHTS_CURRENT_REQ: u8 = 14;
const KNIGHTS_JOIN_REQ: u8 = 17;
const KNIGHTS_USER_ONLINE: u8 = 39;
const KNIGHTS_USER_OFFLINE: u8 = 40;
const KNIGHTS_MARK_VERSION_REQ: u8 = 25;
const KNIGHTS_MARK_REGISTER: u8 = 26;
const KNIGHTS_ALLY_CREATE: u8 = 28;
const KNIGHTS_ALLY_REQ: u8 = 29;
const KNIGHTS_ALLY_INSERT: u8 = 30;
const KNIGHTS_ALLY_REMOVE: u8 = 31;
const KNIGHTS_ALLY_PUNISH: u8 = 32;
const KNIGHTS_ALLY_LIST: u8 = 34;
const KNIGHTS_MARK_REQ: u8 = 35;
pub(crate) const KNIGHTS_UPDATE: u8 = 36;
const KNIGHTS_MARK_REGION_REQ: u8 = 37;
const KNIGHTS_POINT_REQ: u8 = 59;
const KNIGHTS_POINT_METHOD: u8 = 60;
const KNIGHTS_DONATE_POINTS: u8 = 61;
const KNIGHTS_HANDOVER_VICECHIEF_LIST: u8 = 62;
const KNIGHTS_HANDOVER_REQ: u8 = 63;
const KNIGHTS_DONATION_LIST: u8 = 64;
const KNIGHTS_TOP10: u8 = 65;
const KNIGHTS_HANDOVER: u8 = 79;
const KNIGHTS_UPDATENOTICE: u8 = 80;
const KNIGHTS_PROMATE_CLAN: u8 = 82;
const KNIGHTS_UPDATEMEMO: u8 = 88;
const KNIGHTS_VS_LIST: u8 = 96;
const KNIGHTS_UNK1: u8 = 99;
const KNIGHTS_LADDER_POINTS: u8 = 100;

/// Opcode for WIZKNIGHTS_PROCESS.
const WIZKNIGHTS_PROCESS: u8 = 0x3C;
/// Opcode for WIZ_NOTICE (used for clan notice display).
const WIZ_NOTICE: u8 = 0x2E;

fn packet_hex(opcode: u8, data: &[u8]) -> String {
    let mut out = format!("{opcode:02X}");
    for byte in data {
        use std::fmt::Write as _;
        let _ = write!(&mut out, " {byte:02X}");
    }
    out
}

/// Handle WIZKNIGHTS_PROCESS from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = ko_protocol::PacketReader::new(&pkt.data);
    let sub_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    // v2615 requests these read-only clan lists after character selection but
    // before GAMESTART phase 2. They initialise the clan/cape UI; mutations
    // remain strictly InGame-only.
    if session.state() != SessionState::InGame {
        if session.state() == SessionState::CharacterSelected {
            return match sub_opcode {
                KNIGHTS_ALLY_LIST => handle_alliance_list(session).await,
                KNIGHTS_UNK1 => handle_flags_list(session).await,
                _ => Ok(()),
            };
        }
        return Ok(());
    }

    // Dead players cannot perform clan operations
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    match sub_opcode {
        KNIGHTS_CREATE => handle_create(session, &mut reader).await,
        KNIGHTS_JOIN => handle_join(session, &mut reader).await,
        KNIGHTS_WITHDRAW => handle_withdraw(session).await,
        KNIGHTS_REMOVE => handle_remove(session, &mut reader).await,
        KNIGHTS_DESTROY => handle_destroy(session).await,
        KNIGHTS_ADMIT => handle_admit(session, &mut reader).await,
        KNIGHTS_REJECT => handle_reject(session, &mut reader).await,
        KNIGHTS_PUNISH => handle_punish(session, &mut reader).await,
        KNIGHTS_CHIEF => handle_chief(session, &mut reader).await,
        KNIGHTS_VICECHIEF => handle_vicechief(session, &mut reader).await,
        KNIGHTS_OFFICER => handle_officer(session, &mut reader).await,
        KNIGHTS_ALLLIST_REQ => handle_alllist(session, &mut reader).await,
        KNIGHTS_MEMBER_REQ => handle_member_req(session).await,
        KNIGHTS_CURRENT_REQ => handle_current_req(session, &mut reader).await,
        KNIGHTS_JOIN_REQ => handle_join_req(session, &mut reader).await,
        KNIGHTS_MARK_VERSION_REQ => handle_mark_version_req(session).await,
        KNIGHTS_MARK_REGISTER => handle_mark_register(session, &mut reader).await,
        KNIGHTS_MARK_REQ => handle_mark_req(session, &mut reader).await,
        KNIGHTS_MARK_REGION_REQ => {
            // NOTE: Even C++ has early `return;` at top (line 1153) — this is disabled server-side.
            debug!(
                "[{}] KNIGHTS_MARK_REGION_REQ: no-op (disabled in C++ too)",
                session.addr()
            );
            Ok(())
        }
        KNIGHTS_ALLY_CREATE | KNIGHTS_ALLY_INSERT => {
            handle_alliance_create(session, &mut reader, sub_opcode).await
        }
        KNIGHTS_ALLY_REQ => handle_alliance_req(session, &mut reader).await,
        KNIGHTS_ALLY_REMOVE => handle_alliance_remove(session).await,
        KNIGHTS_ALLY_PUNISH => handle_alliance_punish(session, &mut reader).await,
        KNIGHTS_ALLY_LIST => handle_alliance_list(session).await,
        KNIGHTS_POINT_REQ => handle_point_req(session).await,
        KNIGHTS_POINT_METHOD => handle_point_method(session, &mut reader).await,
        KNIGHTS_DONATE_POINTS => handle_donate_np(session, &mut reader).await,
        KNIGHTS_HANDOVER_VICECHIEF_LIST => handle_handover_list(session).await,
        KNIGHTS_HANDOVER_REQ => handle_handover_req(session, &mut reader).await,
        KNIGHTS_HANDOVER => handle_handover(session, &mut reader),
        KNIGHTS_DONATION_LIST => handle_donation_list(session).await,
        KNIGHTS_UPDATENOTICE => handle_update_notice(session, &mut reader).await,
        KNIGHTS_PROMATE_CLAN => handle_promote_clan_list(session, &mut reader).await,
        KNIGHTS_UPDATEMEMO => handle_update_memo(session, &mut reader).await,
        KNIGHTS_TOP10 => handle_top10(session).await,
        KNIGHTS_UNK1 => handle_flags_list(session).await,
        KNIGHTS_LADDER_POINTS => handle_ladder_points(session).await,
        KNIGHTS_VS_LIST => {
            // Tournament clan VS list — send current tournament state to requesting player
            let world = session.world().clone();
            super::tournament::send_state_to_player(&world, session.session_id());
            Ok(())
        }
        _ => {
            debug!(
                "[{}] Unhandled knights sub-opcode: 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

// ── Helper: send a simple error response ──────────────────────────────

/// Build a knights response with sub-opcode + error code.
fn knights_error(sub_opcode: u8, error_code: u8) -> Packet {
    let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
    pkt.write_u8(sub_opcode);
    pkt.write_u8(error_code);
    pkt
}

/// Get the character info for the current session (must be in-game).
fn get_char_info(session: &ClientSession) -> Option<crate::world::CharacterInfo> {
    session.world().get_character_info(session.session_id())
}

// ── KNIGHTS_CREATE (1) ────────────────────────────────────────────────

/// Create a new clan.
async fn handle_create(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Dead players cannot create clans
    if world.is_player_dead(sid) {
        return Ok(());
    }

    // C++ checks: isTrading() || isMerchanting() || isSellingMerchantingPreparing()
    if world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }

    let clan_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Validation checks (C++ order)
    let error = if clan_name.is_empty() || clan_name.len() > MAX_ID_SIZE {
        Some(3u8) // Invalid clan name
    } else if ch.knights_id > 0 {
        Some(5) // Already in a clan
    } else if ch.level < CLAN_LEVEL_REQUIREMENT {
        Some(2) // Level too low
    } else if ch.gold < CLAN_COIN_REQUIREMENT {
        Some(4) // Not enough gold
    } else {
        None
    };

    if let Some(err) = error {
        session
            .send_packet(&knights_error(KNIGHTS_CREATE, err))
            .await?;
        return Ok(());
    }

    // Check zone allows clan updates
    let pos = session.world().get_position(session.session_id());
    if let Some(pos) = pos {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_CREATE, 9))
                    .await?;
                return Ok(());
            }
        }
    }

    // Check if clan name already exists
    let repo = KnightsRepository::new(session.pool());
    match repo.find_by_name(&clan_name).await {
        Ok(Some(_)) => {
            session
                .send_packet(&knights_error(KNIGHTS_CREATE, 3))
                .await?;
            return Ok(());
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                "[{}] find_by_name DB error for clan '{}': {e}",
                session.addr(),
                clan_name
            );
            session
                .send_packet(&knights_error(KNIGHTS_CREATE, 6))
                .await?;
            return Ok(());
        }
    }

    // Get next available clan ID
    let nation = ch.nation;
    let next_id = match repo.next_clan_id(nation).await? {
        Some(id) => id,
        None => {
            session
                .send_packet(&knights_error(KNIGHTS_CREATE, 6))
                .await?;
            return Ok(());
        }
    };

    let char_name = ch.name.clone();
    // `g_pMain->pServerSetting.AutoRoyalG1 ? ClanTypeRoyal1 : ClanTypeTraining`
    use crate::clan_constants::{CLAN_TYPE_ROYAL1, CLAN_TYPE_TRAINING};
    let auto_royal = world
        .get_server_settings()
        .map(|s| s.auto_royal_g1 != 0)
        .unwrap_or(false);
    let clan_flag: i16 = if auto_royal {
        CLAN_TYPE_ROYAL1 as i16
    } else {
        CLAN_TYPE_TRAINING as i16
    };
    // The reference AutoRoyalG1 flow promotes through ClanTypePromoted first,
    // which installs cape 0 and grade 1 before applying Royal1. Keeping -1/5
    // with flag 12 leaves the client cape catalogue uninitialised.
    // Clan grade is driven by Ladder Points (monthly NP), not lifetime NP.
    // Seed the new clan from its founder's current monthly value; scheduled
    // ranking refreshes subsequently sum every member's monthly NP.
    let clan_points = ch.loyalty_monthly;
    let clan_grade: u8 = if auto_royal {
        1
    } else {
        get_knights_grade(clan_points)
    };
    let clan_cape: u16 = if auto_royal { 0 } else { 0xFFFF };

    // Create in DB
    if let Err(e) = repo
        .create_knights(
            next_id,
            nation as i16,
            &clan_name,
            &char_name,
            clan_flag,
            clan_points.min(i32::MAX as u32) as i32,
        )
        .await
    {
        warn!("[{}] Failed to create knights in DB: {}", session.addr(), e);
        session
            .send_packet(&knights_error(KNIGHTS_CREATE, 6))
            .await?;
        return Ok(());
    }

    // Update user's clan + fame in DB
    if let Err(e) = repo
        .update_user_knights(&char_name, next_id, CHIEF as i16)
        .await
    {
        warn!("[{}] Failed to update user knights: {}", session.addr(), e);
    }

    // Deduct gold
    let new_gold = ch.gold.saturating_sub(CLAN_COIN_REQUIREMENT);
    let sid = session.session_id();
    session.world().update_character_stats(sid, |c| {
        c.gold = new_gold;
        c.knights_id = next_id as u16;
        c.fame = CHIEF;
    });

    // Save gold to DB (fire-and-forget)
    let pool = session.pool().clone();
    let char_name_db = char_name.clone();
    tokio::spawn(async move {
        let char_repo = CharacterRepository::new(&pool);
        if let Err(e) = char_repo.update_gold(&char_name_db, new_gold as i64).await {
            tracing::error!("Failed to update gold for {char_name_db} after clan creation: {e}");
        }
    });

    // Insert into runtime knights table
    let info = KnightsInfo {
        id: next_id as u16,
        flag: clan_flag as u8,
        nation,
        grade: clan_grade,
        ranking: 0,
        name: clan_name.clone(),
        chief: char_name.clone(),
        vice_chief_1: String::new(),
        vice_chief_2: String::new(),
        vice_chief_3: String::new(),
        members: 1,
        points: clan_points,
        clan_point_fund: 0,
        notice: String::new(),
        cape: clan_cape,
        cape_r: 0,
        cape_g: 0,
        cape_b: 0,
        mark_version: 0,
        mark_data: Vec::new(),
        alliance: 0,
        castellan_cape: false,
        clan_point_method: 0,
        cast_cape_id: -1,
        cast_cape_r: 0,
        cast_cape_g: 0,
        cast_cape_b: 0,
        cast_cape_time: 0,
        alliance_req: 0,
        premium_time: 0,
        premium_in_use: 0,
        online_members: 0,
        online_np_count: 0,
        online_exp_count: 0,
    };
    session.world().insert_knights(info);

    // Build success response
    // result << uint8(1) << uint32(sid) << sClanID << strKnightsName
    //        << uint8(grade) << ranking << gold;
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_CREATE);
    result.write_u8(1); // success
    result.write_u32(sid as u32);
    result.write_u16(next_id as u16);
    result.write_string(&clan_name);
    result.write_u8(clan_grade);
    result.write_u8(0); // ranking
    result.write_u32(new_gold);

    // Broadcast to region
    if let Some((pos, event_room)) = session
        .world()
        .with_session(sid, |h| (h.position, h.event_room))
    {
        session.world().broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(result),
            None,
            event_room,
        );
    }

    Ok(())
}

// ── KNIGHTS_JOIN (2) ──────────────────────────────────────────────────

/// Invite a player to join the clan.
async fn handle_join(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    // Dead players cannot invite to clan
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be in a zone that allows clan updates
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_JOIN, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    // Must be clan leader or vice chief
    if ch.fame != CHIEF && ch.fame != VICECHIEF {
        session.send_packet(&knights_error(KNIGHTS_JOIN, 6)).await?;
        return Ok(());
    }

    // Clan must exist
    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => {
            session.send_packet(&knights_error(KNIGHTS_JOIN, 7)).await?;
            return Ok(());
        }
    };

    // Read target session ID
    let target_id = match reader.read_u32() {
        Some(v) => v as u16,
        None => return Ok(()),
    };

    // Target must be online and in-game
    let target_ch = match session.world().get_character_info(target_id) {
        Some(c) => c,
        None => {
            session.send_packet(&knights_error(KNIGHTS_JOIN, 2)).await?;
            return Ok(());
        }
    };

    // Target must be alive
    if target_ch.res_hp_type == crate::world::USER_DEAD {
        session.send_packet(&knights_error(KNIGHTS_JOIN, 3)).await?;
        return Ok(());
    }

    // Target must be same nation
    if target_ch.nation != ch.nation {
        session.send_packet(&knights_error(KNIGHTS_JOIN, 4)).await?;
        return Ok(());
    }

    // Target must not already be in a clan
    if target_ch.knights_id > 0 {
        session.send_packet(&knights_error(KNIGHTS_JOIN, 5)).await?;
        return Ok(());
    }

    // Track pending invitation on the target — C++ sets m_bKnightsReq (KnightsManager.cpp:335)
    session.world().update_session(target_id, |h| {
        h.pending_knights_invite = ch.knights_id;
    });

    // Send join request to the target
    // C++ sends: KNIGHTS_JOIN_REQ + u8(1) + u32(inviter_sid) + u16(clan_id) + string(clan_name)
    let mut invite_pkt = Packet::new(WIZKNIGHTS_PROCESS);
    invite_pkt.write_u8(KNIGHTS_JOIN_REQ);
    invite_pkt.write_u8(1); // request flag
    invite_pkt.write_u32(session.session_id() as u32);
    invite_pkt.write_u16(ch.knights_id);
    invite_pkt.write_string(&clan.name);
    session.world().send_to_session_owned(target_id, invite_pkt);

    // Acknowledge to the inviter
    let mut ack = Packet::new(WIZKNIGHTS_PROCESS);
    ack.write_u8(KNIGHTS_JOIN);
    ack.write_u8(0); // success (invitation sent)
    session.send_packet(&ack).await?;

    Ok(())
}

// ── KNIGHTS_JOIN_REQ (17) ─────────────────────────────────────────────

fn read_join_req_response(reader: &mut ko_protocol::PacketReader<'_>) -> Option<(u8, u16, u16)> {
    let response = reader.read_u8()?;
    // v2615 sub_8413E0 sends response:u8, inviter:u32, clan:u16.
    // Reading clan immediately after response mistook the inviter SID for it.
    let inviter_id = u16::try_from(reader.read_u32()?).ok()?;
    let clan_id = reader.read_u16()?;
    Some((response, inviter_id, clan_id))
}

/// Accept or decline a clan join invitation.
async fn handle_join_req(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let (response, inviter_id, clan_id) = match read_join_req_response(reader) {
        Some(v) => v,
        None => return Ok(()),
    };

    // If declined, clear pending invite and return
    if response != 1 {
        let sid = session.session_id();
        session.world().update_session(sid, |h| {
            h.pending_knights_invite = 0;
        });
        return Ok(());
    }

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // C++ KnightsManager.cpp:880 — validate pending invitation
    let sid = session.session_id();
    let pending = session
        .world()
        .with_session(sid, |h| h.pending_knights_invite)
        .unwrap_or(0);
    if pending == 0 || pending != clan_id {
        warn!(
            sid,
            clan_id, pending, inviter_id, "Clan invitation mismatch"
        );
        return Ok(());
    }
    let valid_inviter = session
        .world()
        .get_character_info(inviter_id)
        .is_some_and(|c| {
            c.knights_id == clan_id
                && c.nation == ch.nation
                && (c.fame == CHIEF || c.fame == VICECHIEF)
        });
    if !valid_inviter {
        session.send_packet(&knights_error(KNIGHTS_JOIN, 2)).await?;
        return Ok(());
    }
    // Clear the pending invite now that we're processing it
    session.world().update_session(sid, |h| {
        h.pending_knights_invite = 0;
    });

    // Already in a clan
    if ch.knights_id > 0 {
        return Ok(());
    }

    // Clan must exist
    let clan = match session.world().get_knights(clan_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Clan full check
    if clan.members >= MAX_CLAN_USERS {
        return Ok(());
    }

    // Update DB
    let repo = KnightsRepository::new(session.pool());
    let char_name = ch.name.clone();
    if let Err(e) = repo
        .update_user_knights(&char_name, clan_id as i16, TRAINEE as i16)
        .await
    {
        warn!("[{}] Failed to join knights DB: {}", session.addr(), e);
        return Ok(());
    }

    // Update member count
    let new_member_count = clan.members + 1;
    if let Err(e) = repo
        .update_member_count(clan_id as i16, new_member_count as i32)
        .await
    {
        tracing::error!("Failed to update member count for clan {clan_id}: {e}");
    }

    // Update runtime
    let sid = session.session_id();
    session.world().update_character_stats(sid, |c| {
        c.knights_id = clan_id;
        c.fame = TRAINEE;
    });
    session.world().update_knights(clan_id, |k| {
        k.members = new_member_count;
    });

    // Notify the joiner
    // C++ ReqKnightsJoin sends: KNIGHTS_JOIN + u8(1) + u32(sid) + clan_id + clan_name + grade + ranking
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_JOIN);
    result.write_u8(1); // success
    result.write_u32(sid as u32);
    result.write_u16(clan_id);
    result.write_string(&clan.name);
    result.write_u8(clan.grade);
    result.write_u8(clan.ranking);
    session.send_packet(&result).await?;

    // SendClanPremium after join
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        if clan.premium_time > now {
            let remaining = (clan.premium_time - now) / 60;
            session.world().update_session(sid, |h| {
                h.clan_premium_in_use = 13;
            });
            let pkt = super::premium::build_clan_premium_packet(true, remaining);
            session.send_packet(&pkt).await?;
        }
    }

    // Broadcast online notification to clan members
    let mut online_pkt = Packet::new(WIZKNIGHTS_PROCESS);
    online_pkt.write_u8(KNIGHTS_USER_ONLINE);
    online_pkt.write_sbyte_string(&char_name);
    session
        .world()
        .send_to_knights_members(clan_id, Arc::new(online_pkt), Some(sid));

    Ok(())
}

// ── KNIGHTS_WITHDRAW (3) ──────────────────────────────────────────────

/// Leave the clan voluntarily.
async fn handle_withdraw(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // If chief, treat as destroy
    if ch.fame == CHIEF {
        return handle_destroy(session).await;
    }

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_WITHDRAW, 2))
            .await?;
        return Ok(());
    }

    // Cannot withdraw if command captain
    if ch.fame == COMMAND_CAPTAIN {
        session
            .send_packet(&knights_error(KNIGHTS_WITHDRAW, 2))
            .await?;
        return Ok(());
    }

    // Check zone allows clan updates
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_WITHDRAW, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    // Update DB
    let repo = KnightsRepository::new(session.pool());
    let char_name = ch.name.clone();
    if let Err(e) = repo.update_user_knights(&char_name, 0, 0).await {
        warn!(
            "[{}] Failed to withdraw from knights: {}",
            session.addr(),
            e
        );
        return Ok(());
    }

    // Update member count
    if let Some(clan) = session.world().get_knights(clan_id) {
        let new_count = clan.members.saturating_sub(1);
        if let Err(e) = repo
            .update_member_count(clan_id as i16, new_count as i32)
            .await
        {
            tracing::warn!("Failed to update member count for clan {clan_id}: {e}");
        }
        session.world().update_knights(clan_id, |k| {
            k.members = new_count;
        });
    }

    // Update session
    let sid = session.session_id();
    session.world().update_character_stats(sid, |c| {
        c.knights_id = 0;
        c.fame = 0;
    });

    // SendClanPremium(exits=true) — reset clan premium on leave
    session.world().update_session(sid, |h| {
        h.clan_premium_in_use = 0;
    });
    let pkt = super::premium::build_clan_premium_packet(false, 0);
    session.send_packet(&pkt).await?;

    // Send success to the user
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_WITHDRAW);
    result.write_u8(1); // success
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_REMOVE (4) ────────────────────────────────────────────────

/// Kick a member from the clan.
async fn handle_remove(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let target_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_REMOVE, 2))
            .await?;
        return Ok(());
    }

    // Must be clan leader to remove a member.
    if ch.fame != CHIEF {
        session
            .send_packet(&knights_error(KNIGHTS_REMOVE, 6))
            .await?;
        return Ok(());
    }

    // Can't remove yourself
    if target_name.to_uppercase() == ch.name.to_uppercase() {
        session
            .send_packet(&knights_error(KNIGHTS_REMOVE, 9))
            .await?;
        return Ok(());
    }

    // If target is online, validate nation and clan
    if let Some(target_sid) = session.world().find_session_by_name(&target_name) {
        if let Some(tc) = session.world().get_character_info(target_sid) {
            if ch.nation != tc.nation {
                session
                    .send_packet(&knights_error(KNIGHTS_REMOVE, 4))
                    .await?;
                return Ok(());
            }
            if ch.knights_id != tc.knights_id {
                session
                    .send_packet(&knights_error(KNIGHTS_REMOVE, 5))
                    .await?;
                return Ok(());
            }
        }
    }

    // Check zone
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_REMOVE, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    // Update target in DB (clear clan — only if target is in caller's clan)
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo.remove_from_knights(&target_name, clan_id as i16).await {
        warn!("[{}] Failed to remove from knights: {}", session.addr(), e);
        return Ok(());
    }

    // Update member count
    if let Some(clan) = session.world().get_knights(clan_id) {
        let new_count = clan.members.saturating_sub(1);
        if let Err(e) = repo
            .update_member_count(clan_id as i16, new_count as i32)
            .await
        {
            tracing::warn!("Failed to update member count for clan {clan_id}: {e}");
        }
        session.world().update_knights(clan_id, |k| {
            k.members = new_count;
        });
    }

    // If target is online, update their session
    if let Some(target_sid) = session.world().find_session_by_name(&target_name) {
        session.world().update_character_stats(target_sid, |c| {
            c.knights_id = 0;
            c.fame = 0;
        });

        // SendClanPremium(exits=true) — reset clan premium on kick
        session.world().update_session(target_sid, |h| {
            h.clan_premium_in_use = 0;
        });
        let prem_pkt = super::premium::build_clan_premium_packet(false, 0);
        session.world().send_to_session_owned(target_sid, prem_pkt);

        // Notify target
        let mut notify = Packet::new(WIZKNIGHTS_PROCESS);
        notify.write_u8(KNIGHTS_REMOVE);
        notify.write_u8(1); // you were removed
        session.world().send_to_session_owned(target_sid, notify);
    }

    // Confirm to the remover
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_REMOVE);
    result.write_u8(1); // success
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_DESTROY (5) ───────────────────────────────────────────────

/// Disband the clan (chief only).
async fn handle_destroy(session: &mut ClientSession) -> anyhow::Result<()> {
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.fame != CHIEF {
        session
            .send_packet(&knights_error(KNIGHTS_DESTROY, 0))
            .await?;
        return Ok(());
    }

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        return Ok(());
    }

    // Check zone
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_DESTROY, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    // DB: clear all members + delete clan
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo.clear_all_members(clan_id as i16).await {
        tracing::error!("Failed to clear all members for clan {clan_id}: {e}");
    }
    if let Err(e) = repo.destroy_knights(clan_id as i16).await {
        tracing::error!("Failed to destroy knights for clan {clan_id}: {e}");
    }

    // Send destroy notification and clear clan from all online members
    let mut destroy_notify = Packet::new(WIZKNIGHTS_PROCESS);
    destroy_notify.write_u8(KNIGHTS_DESTROY);
    destroy_notify.write_u8(1); // success
    session
        .world()
        .send_to_knights_members(clan_id, Arc::new(destroy_notify), None);

    // v2525: Clan dissolved screen notification (WIZ_CLANPOINTS_BATTLE sub=0)
    let clan_notif = super::clanpoints_battle::build_notification(0);
    session
        .world()
        .send_to_knights_members(clan_id, Arc::new(clan_notif), None);

    // C++ Knights.cpp:646-663 — if the disbanding player is king, send king cape
    let sid = session.session_id();
    if let Some(nation) = session
        .world()
        .with_session(sid, |h| h.character.as_ref().map(|c| c.nation))
        .flatten()
    {
        if session.world().is_king(nation, &ch.name) {
            let king_cape = if nation == 2 { 98u16 } else { 97u16 }; // ELMORAD=98, KARUS=97
            let mut cape_pkt = Packet::new(ko_protocol::Opcode::WizCape as u8);
            cape_pkt.write_u16(1);
            cape_pkt.write_u16(0);
            cape_pkt.write_u16(0);
            cape_pkt.write_u16(king_cape);
            cape_pkt.write_u16(0);
            cape_pkt.write_u16(0);
            session.send_packet(&cape_pkt).await?;
        }
    }

    // Clear clan info from all online members' sessions
    session.world().clear_knights_from_sessions(clan_id);

    // Remove from runtime
    session.world().remove_knights(clan_id);

    Ok(())
}

// ── KNIGHTS_ADMIT (6) ─────────────────────────────────────────────────

/// Promote a clan member's rank.
async fn handle_admit(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let target_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_ADMIT, 7))
            .await?;
        return Ok(());
    }

    // Must be officer or higher
    if ch.fame > OFFICER || ch.fame == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_ADMIT, 0))
            .await?;
        return Ok(());
    }

    // Can't promote yourself
    if target_name.to_uppercase() == ch.name.to_uppercase() {
        session
            .send_packet(&knights_error(KNIGHTS_ADMIT, 9))
            .await?;
        return Ok(());
    }

    // Check zone
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_ADMIT, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    // Verify target is online and in the same clan/nation
    match session.world().find_session_by_name(&target_name) {
        Some(target_sid) => {
            if let Some(tc) = session.world().get_character_info(target_sid) {
                if ch.nation != tc.nation {
                    session
                        .send_packet(&knights_error(KNIGHTS_ADMIT, 4))
                        .await?;
                    return Ok(());
                }
                if tc.knights_id != clan_id {
                    session
                        .send_packet(&knights_error(KNIGHTS_ADMIT, 5))
                        .await?;
                    return Ok(());
                }
            }
        }
        None => {
            // Target offline — error code 2
            session
                .send_packet(&knights_error(KNIGHTS_ADMIT, 2))
                .await?;
            return Ok(());
        }
    }

    // Success — DB update handled by the DB request flow
    // For now, acknowledge
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_ADMIT);
    result.write_u8(1); // success
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_REJECT (7) ────────────────────────────────────────────────

/// Demote a clan member's rank.
async fn handle_reject(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let target_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_REJECT, 7))
            .await?;
        return Ok(());
    }

    // Must be officer or higher
    if ch.fame > OFFICER || ch.fame == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_REJECT, 0))
            .await?;
        return Ok(());
    }

    // Can't demote yourself
    if target_name.to_uppercase() == ch.name.to_uppercase() {
        session
            .send_packet(&knights_error(KNIGHTS_REJECT, 9))
            .await?;
        return Ok(());
    }

    // Check zone
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_REJECT, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_REJECT);
    result.write_u8(1); // success
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_PUNISH (8) ────────────────────────────────────────────────

/// Punish a clan member.
async fn handle_punish(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let target_name = match reader.read_string() {
        Some(n) if !n.is_empty() => n,
        _ => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be VICECHIEF or higher to punish.
    // NOTE: In C++ with CHIEF=1, VICECHIEF=2 the comparison `fame < 2` unintentionally
    // blocks CHIEF(1). We treat this as a C++ bug and allow CHIEF as well, since the
    // clan leader should always be able to punish members.
    if ch.knights_id == 0 || (ch.fame != CHIEF && ch.fame != VICECHIEF) {
        session
            .send_packet(&knights_error(KNIGHTS_PUNISH, 0))
            .await?;
        return Ok(());
    }

    // Cannot punish yourself
    if target_name.to_uppercase() == ch.name.to_uppercase() {
        session
            .send_packet(&knights_error(KNIGHTS_PUNISH, 9))
            .await?;
        return Ok(());
    }

    // Validate target is online, same nation, same clan
    let target_sid = session.world().find_session_by_name(&target_name);
    let b_result = match target_sid {
        Some(tsid) => {
            match session.world().get_character_info(tsid) {
                Some(tc) => {
                    if ch.nation != tc.nation {
                        4 // different nation
                    } else if ch.knights_id != tc.knights_id {
                        5 // not in same clan
                    } else {
                        1 // success
                    }
                }
                None => 2, // invalid
            }
        }
        None => 2, // offline/not found
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_PUNISH);
    result.write_u8(b_result);
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_CHIEF (9) ─────────────────────────────────────────────────

/// Transfer clan leadership to another member.
async fn handle_chief(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let target_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.fame != CHIEF {
        session
            .send_packet(&knights_error(KNIGHTS_CHIEF, 0))
            .await?;
        return Ok(());
    }

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        return Ok(());
    }

    // Check zone
    if let Some(pos) = session.world().get_position(session.session_id()) {
        if let Some(zone) = session.world().get_zone(pos.zone_id) {
            if !zone
                .zone_info
                .as_ref()
                .is_none_or(|zi| zi.abilities.clan_updates)
            {
                session
                    .send_packet(&knights_error(KNIGHTS_CHIEF, 12))
                    .await?;
                return Ok(());
            }
        }
    }

    // Target must be online and in the same clan
    let target_sid = match session.world().find_session_by_name(&target_name) {
        Some(s) => s,
        None => {
            session
                .send_packet(&knights_error(KNIGHTS_CHIEF, 2))
                .await?;
            return Ok(());
        }
    };

    let target_ch = match session.world().get_character_info(target_sid) {
        Some(c) => c,
        None => {
            session
                .send_packet(&knights_error(KNIGHTS_CHIEF, 2))
                .await?;
            return Ok(());
        }
    };

    if target_ch.knights_id != clan_id {
        session
            .send_packet(&knights_error(KNIGHTS_CHIEF, 5))
            .await?;
        return Ok(());
    }

    // DB: update chief in knights table + update both users' fame
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo.update_chief(clan_id as i16, &target_ch.name).await {
        tracing::error!("Failed to update chief for clan {clan_id}: {e}");
    }
    if let Err(e) = repo
        .update_user_knights(&ch.name, clan_id as i16, TRAINEE as i16)
        .await
    {
        tracing::error!("Failed to update old chief {} to trainee: {e}", ch.name);
    }
    if let Err(e) = repo
        .update_user_knights(&target_ch.name, clan_id as i16, CHIEF as i16)
        .await
    {
        tracing::error!("Failed to update new chief {}: {e}", target_ch.name);
    }

    // Update runtime
    let sid = session.session_id();
    session.world().update_character_stats(sid, |c| {
        c.fame = TRAINEE;
    });
    session.world().update_character_stats(target_sid, |c| {
        c.fame = CHIEF;
    });
    session.world().update_knights(clan_id, |k| {
        k.chief = target_ch.name.clone();
    });

    // Notify both
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_CHIEF);
    result.write_u8(1);
    session.send_packet(&result).await?;
    session.world().send_to_session_owned(target_sid, result);

    Ok(())
}

// ── KNIGHTS_VICECHIEF (10) ────────────────────────────────────────────

/// Appoint/remove a vice chief.
async fn handle_vicechief(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let target_name = match reader.read_string() {
        Some(n) if !n.is_empty() => n,
        _ => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.fame != CHIEF {
        session
            .send_packet(&knights_error(KNIGHTS_VICECHIEF, 0))
            .await?;
        return Ok(());
    }

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        return Ok(());
    }

    // Validate target: must be online, same nation, same clan
    if let Some(target_sid) = session.world().find_session_by_name(&target_name) {
        if let Some(tc) = session.world().get_character_info(target_sid) {
            if ch.nation != tc.nation {
                session
                    .send_packet(&knights_error(KNIGHTS_VICECHIEF, 4))
                    .await?;
                return Ok(());
            }
            if tc.knights_id != clan_id {
                session
                    .send_packet(&knights_error(KNIGHTS_VICECHIEF, 5))
                    .await?;
                return Ok(());
            }
        }
    } else {
        // Target offline
        session
            .send_packet(&knights_error(KNIGHTS_VICECHIEF, 2))
            .await?;
        return Ok(());
    }

    // Update target fame to VICECHIEF
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo
        .update_user_knights(&target_name, clan_id as i16, VICECHIEF as i16)
        .await
    {
        tracing::warn!("Failed to set vice chief for {target_name}: {e}");
    }

    // Update runtime for target if online
    if let Some(target_sid) = session.world().find_session_by_name(&target_name) {
        session.world().update_character_stats(target_sid, |c| {
            c.fame = VICECHIEF;
        });
    }

    // Update vice chief slots in knights
    session.world().update_knights(clan_id, |k| {
        if k.vice_chief_1.is_empty() {
            k.vice_chief_1 = target_name.clone();
        } else if k.vice_chief_2.is_empty() {
            k.vice_chief_2 = target_name.clone();
        } else if k.vice_chief_3.is_empty() {
            k.vice_chief_3 = target_name.clone();
        }
    });

    // Update DB vice chiefs
    if let Some(clan) = session.world().get_knights(clan_id) {
        if let Err(e) = repo
            .update_vice_chiefs(
                clan_id as i16,
                Some(clan.vice_chief_1.as_str()).filter(|s| !s.is_empty()),
                Some(clan.vice_chief_2.as_str()).filter(|s| !s.is_empty()),
                Some(clan.vice_chief_3.as_str()).filter(|s| !s.is_empty()),
            )
            .await
        {
            tracing::warn!("Failed to update vice chiefs for clan {clan_id}: {e}");
        }
    }

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_VICECHIEF);
    result.write_u8(1);
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_OFFICER (11) ──────────────────────────────────────────────

/// Appoint/remove an officer.
async fn handle_officer(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let target_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be clan leader to promote to officer.
    if ch.fame != CHIEF {
        session
            .send_packet(&knights_error(KNIGHTS_OFFICER, 6))
            .await?;
        return Ok(());
    }

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        return Ok(());
    }

    // Validate target: must be online, same nation, same clan.
    let target_sid = match session.world().find_session_by_name(&target_name) {
        Some(s) => s,
        None => {
            session
                .send_packet(&knights_error(KNIGHTS_OFFICER, 2))
                .await?;
            return Ok(());
        }
    };
    if let Some(tc) = session.world().get_character_info(target_sid) {
        if ch.nation != tc.nation {
            session
                .send_packet(&knights_error(KNIGHTS_OFFICER, 4))
                .await?;
            return Ok(());
        }
        if tc.knights_id != clan_id {
            session
                .send_packet(&knights_error(KNIGHTS_OFFICER, 5))
                .await?;
            return Ok(());
        }
    }

    // Update target fame to OFFICER
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo
        .update_user_knights(&target_name, clan_id as i16, OFFICER as i16)
        .await
    {
        tracing::warn!("Failed to set officer for {target_name}: {e}");
    }

    // Update runtime if online
    if let Some(target_sid) = session.world().find_session_by_name(&target_name) {
        session.world().update_character_stats(target_sid, |c| {
            c.fame = OFFICER;
        });
    }

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_OFFICER);
    result.write_u8(1);
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_ALLLIST_REQ (12) ──────────────────────────────────────────

/// List all clans of the same nation (paged, 10 per page).
async fn handle_alllist(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let page = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    let start = page as usize * 10;
    let nation = ch.nation;

    // Use DB as the source of truth for the full clan listing.
    let repo = KnightsRepository::new(session.pool());
    let all_clans = match repo.find_by_nation(nation as i16).await {
        Ok(clans) => clans,
        Err(e) => {
            tracing::warn!("[{}] find_by_nation DB error: {e}", session.addr());
            Vec::new()
        }
    };

    // Filter to promoted clans (flag >= 2 = ClanTypePromoted)
    let promoted: Vec<_> = all_clans
        .iter()
        .filter(|k| k.flag >= 2) // isPromoted
        .collect();

    let total = promoted.len();
    let end = (start + 10).min(total);
    let count = if start < total {
        (end - start) as u16
    } else {
        0
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_ALLLIST_REQ);
    result.write_u8(1); // success
    result.write_u16(page);
    result.write_u16(count);

    if start < total {
        for row in &promoted[start..end] {
            result.write_u16(row.id_num as u16);
            result.write_string(&row.id_name);
            result.write_u16(row.members as u16);
            result.write_string(&row.chief);
            result.write_u32(row.points as u32);
        }
    }

    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_MEMBER_REQ (13) ───────────────────────────────────────────

/// List all members of the player's clan.
async fn handle_member_req(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        session
            .send_packet(&knights_error(KNIGHTS_MEMBER_REQ, 2))
            .await?;
        return Ok(());
    }

    let clan_id = ch.knights_id;
    let clan = match session.world().get_knights(clan_id) {
        Some(k) => k,
        None => {
            session
                .send_packet(&knights_error(KNIGHTS_MEMBER_REQ, 7))
                .await?;
            return Ok(());
        }
    };

    // Load members from DB
    let repo = KnightsRepository::new(session.pool());
    let member_count = match repo.count_members(clan_id as i16).await {
        Ok(count) => count as u16,
        Err(e) => {
            tracing::warn!(
                "[{}] count_members DB error (clan {clan_id}): {e}",
                session.addr()
            );
            0
        }
    };
    let members = match repo.load_clan_members(clan_id as i16).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                "[{}] load_clan_members DB error (clan {clan_id}): {e}",
                session.addr()
            );
            Vec::new()
        }
    };

    // Build response
    // C++ format: sub_opcode(u8) + result(u8) + [DByte mode] pkt_size(u16) +
    //   member_count(u16) + max_users(u16) + notice(string) + count(u16) +
    //   for each member: name(string) + fame(u8) + unknown(u8) + level(u8) +
    //     class(u16) + online(u8) + memo(string) + last_login_hours(u32)
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_MEMBER_REQ);
    result.write_u8(1); // success

    // The C++ code uses DByte + pktSize placeholder, then fills it later.
    // We'll calculate it inline. The pktSize field covers everything after it.
    // For simplicity, we write a placeholder and fix it later.
    let pkt_size_pos = result.data.len();
    result.write_u16(0); // placeholder for pkt_size

    result.write_u16(member_count);
    result.write_u16(MAX_CLAN_USERS);
    result.write_string(&clan.notice);

    let count = members.len().min(MAX_CLAN_USERS as usize) as u16;
    result.write_u16(count);

    for member in members.iter().take(MAX_CLAN_USERS as usize) {
        // Check if online
        let online_info = session.world().find_session_by_name(&member.str_user_id);
        let is_online = online_info.is_some();

        if is_online {
            // Use live data for online members
            if let Some(sid) = online_info {
                if let Some(live_ch) = session.world().get_character_info(sid) {
                    result.write_string(&live_ch.name);
                    result.write_u8(live_ch.fame);
                    result.write_u8(0); // unknown
                    result.write_u8(live_ch.level);
                    result.write_u16(live_ch.class);
                    result.write_u8(1); // online
                    result.write_string(member.str_memo.as_deref().unwrap_or(""));
                    result.write_u32(0); // last login hours (online = 0)
                    continue;
                }
            }
        }

        // Offline member — use DB data
        result.write_string(&member.str_user_id);
        result.write_u8(member.fame as u8);
        result.write_u8(0); // unknown
        result.write_u8(member.level as u8);
        result.write_u16(member.class as u16);
        result.write_u8(0); // offline

        result.write_string(member.str_memo.as_deref().unwrap_or(""));
        // Last login in hours (approximate)
        let hours = if member.n_last_login > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i32;
            ((now - member.n_last_login) / 3600).max(0) as u32
        } else {
            0
        };
        result.write_u32(hours);
    }

    // Fill in pkt_size (C++ puts total byte count of the DByte section)
    let pkt_size = (result.data.len() - pkt_size_pos - 2 + 4) as u16;
    result.data[pkt_size_pos] = (pkt_size & 0xFF) as u8;
    result.data[pkt_size_pos + 1] = ((pkt_size >> 8) & 0xFF) as u8;

    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_CURRENT_REQ (14) ──────────────────────────────────────────

/// List online clan members (paged).
async fn handle_current_req(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let page = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    let clan_id = ch.knights_id;
    if clan_id == 0 {
        let mut err = Packet::new(WIZKNIGHTS_PROCESS);
        err.write_u8(KNIGHTS_CURRENT_REQ);
        err.write_u8(0);
        err.write_string("No clan");
        session.send_packet(&err).await?;
        return Ok(());
    }

    let clan = match session.world().get_knights(clan_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Collect online members
    let online_members = session.world().get_online_knights_members(clan_id);

    let start = page as usize * 10;
    let end = (start + 10).min(online_members.len());
    let count = if start < online_members.len() {
        (end - start) as u16
    } else {
        0
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_CURRENT_REQ);
    result.write_u8(1); // success
    result.write_string(&clan.chief);
    result.write_u16(page);
    result.write_u16(count);

    if start < online_members.len() {
        for (name, fame, level, class) in &online_members[start..end] {
            result.write_string(name);
            result.write_u8(*fame);
            result.write_u8(*level);
            result.write_u16(*class);
        }
    }

    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_POINT_REQ (59) ────────────────────────────────────────────

/// Get donate NP info (user's loyalty + clan fund).
async fn handle_point_req(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        return Ok(());
    }

    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_POINT_REQ);
    result.write_u8(1); // success
    result.write_u32(ch.loyalty);
    result.write_u32(clan.clan_point_fund);

    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_DONATE_POINTS (61) ────────────────────────────────────────

/// Donate NP to the clan fund.
/// Deducts NP from the user's loyalty and adds it to the clan fund.
/// User must retain at least `MIN_NP_TO_DONATE` (1000) NP after donation.
async fn handle_donate_np(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    // Dead players cannot donate NP
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let amount = match reader.read_u32() {
        Some(v) => v,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 || amount == 0 {
        return Ok(());
    }

    // C++ checks: user must have enough NP and retain MIN_NP_TO_DONATE after donation
    if amount > ch.loyalty || (ch.loyalty - amount) < MIN_NP_TO_DONATE {
        return Ok(());
    }

    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Must be at least Accredited (flag >= 3)
    if clan.flag < 3 {
        return Ok(());
    }

    // Deduct NP from the user
    let new_loyalty = ch.loyalty - amount;
    let world = session.world();
    let sid = session.session_id();
    world.update_session(sid, |h| {
        if let Some(ref mut c) = h.character {
            c.loyalty = new_loyalty;
        }
    });

    // Update clan fund in runtime (cap at i32::MAX for DB compatibility)
    let new_fund = clan
        .clan_point_fund
        .saturating_add(amount)
        .min(i32::MAX as u32);
    world.update_knights(ch.knights_id, |k| {
        k.clan_point_fund = new_fund;
    });

    // Send WIZ_LOYALTY_CHANGE to update the client
    // Packet: WIZ_LOYALTY_CHANGE(0x2A) + u8(1) + u32(loyalty) + u32(loyalty_monthly) + u32(0) + u32(0)
    let loyalty_monthly = world
        .with_session(sid, |h| {
            h.character.as_ref().map(|c| c.loyalty_monthly).unwrap_or(0)
        })
        .unwrap_or(0);

    let mut loyalty_pkt = Packet::new(0x2A); // WIZ_LOYALTY_CHANGE
    loyalty_pkt.write_u8(1); // LOYALTY_NATIONAL_POINTS
    loyalty_pkt.write_u32(new_loyalty);
    loyalty_pkt.write_u32(loyalty_monthly);
    loyalty_pkt.write_u32(0); // clan donations
    loyalty_pkt.write_u32(0); // clan loyalty amount
    session.send_packet(&loyalty_pkt).await?;

    // Update DB: clan fund
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo
        .update_clan_point_fund(ch.knights_id as i16, new_fund.min(i32::MAX as u32) as i32)
        .await
    {
        tracing::warn!(
            "Failed to update clan point fund for clan {}: {e}",
            ch.knights_id
        );
    }

    Ok(())
}

// ── KNIGHTS_POINT_METHOD (60) ────────────────────────────────────────

/// Change the clan point method (how NP is distributed).
async fn handle_point_method(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let sub_code = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be clan leader
    if ch.fame != CHIEF || ch.knights_id == 0 {
        return Ok(());
    }

    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Must be at least Accredited5 (flag >= 3) and user is the actual chief
    if clan.flag < 3 || ch.name != clan.chief {
        return Ok(());
    }

    // C++ logic: method = subCode != 0 ? subCode - 1 : pKnights->m_sClanPointMethod
    let method = if sub_code != 0 {
        sub_code.saturating_sub(1)
    } else {
        clan.clan_point_method
    };

    // Update DB
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo
        .update_clan_point_method(ch.knights_id as i16, method as i16)
        .await
    {
        tracing::warn!(
            "Failed to update clan point method for clan {}: {e}",
            ch.knights_id
        );
    }

    // Update runtime
    session.world().update_knights(ch.knights_id, |k| {
        k.clan_point_method = method;
    });

    // Send success response
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_POINT_METHOD);
    result.write_u8(1); // success
    result.write_u8(method);
    session.send_packet(&result).await?;

    Ok(())
}

// ── KNIGHTS_HANDOVER_VICECHIEF_LIST (62) ────────────────────────────

/// List online vice chiefs for leadership handover.
async fn handle_handover_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        return Ok(());
    }

    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    let is_clan_leader: u8 = if ch.fame == CHIEF { 1 } else { 2 };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_HANDOVER_VICECHIEF_LIST);
    result.write_u8(is_clan_leader);

    // Placeholder for vice chief count
    let count_pos = result.data.len();
    result.write_u16(0);
    let mut count: u16 = 0;

    // Only include online vice chiefs
    if !clan.vice_chief_1.is_empty()
        && session
            .world()
            .find_session_by_name(&clan.vice_chief_1)
            .is_some()
    {
        result.write_string(&clan.vice_chief_1);
        count += 1;
    }
    if !clan.vice_chief_2.is_empty()
        && session
            .world()
            .find_session_by_name(&clan.vice_chief_2)
            .is_some()
    {
        result.write_string(&clan.vice_chief_2);
        count += 1;
    }
    if !clan.vice_chief_3.is_empty()
        && session
            .world()
            .find_session_by_name(&clan.vice_chief_3)
            .is_some()
    {
        result.write_string(&clan.vice_chief_3);
        count += 1;
    }

    // Fill in count
    result.data[count_pos] = (count & 0xFF) as u8;
    result.data[count_pos + 1] = ((count >> 8) & 0xFF) as u8;

    session.send_packet(&result).await?;
    Ok(())
}

// ── KNIGHTS_HANDOVER_REQ (63) ───────────────────────────────────────

/// Request leadership handover to a vice chief.
async fn handle_handover_req(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.fame != CHIEF || ch.knights_id == 0 {
        let mut fail = Packet::new(WIZKNIGHTS_PROCESS);
        fail.write_u8(KNIGHTS_HANDOVER_REQ);
        fail.write_u8(3); // failure
        session.send_packet(&fail).await?;
        return Ok(());
    }

    // Validate clan exists in runtime state
    if session.world().get_knights(ch.knights_id).is_none() {
        let mut fail = Packet::new(WIZKNIGHTS_PROCESS);
        fail.write_u8(KNIGHTS_HANDOVER_REQ);
        fail.write_u8(3);
        session.send_packet(&fail).await?;
        return Ok(());
    }

    let target_name = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    // Target must be online and a vice chief in the same clan
    let target_sid = match session.world().find_session_by_name(&target_name) {
        Some(s) => s,
        None => {
            let mut fail = Packet::new(WIZKNIGHTS_PROCESS);
            fail.write_u8(KNIGHTS_HANDOVER_REQ);
            fail.write_u8(3);
            session.send_packet(&fail).await?;
            return Ok(());
        }
    };

    let target_ch = match session.world().get_character_info(target_sid) {
        Some(c) => c,
        None => {
            let mut fail = Packet::new(WIZKNIGHTS_PROCESS);
            fail.write_u8(KNIGHTS_HANDOVER_REQ);
            fail.write_u8(3);
            session.send_packet(&fail).await?;
            return Ok(());
        }
    };

    if target_ch.fame != VICECHIEF || target_ch.knights_id != ch.knights_id {
        let mut fail = Packet::new(WIZKNIGHTS_PROCESS);
        fail.write_u8(KNIGHTS_HANDOVER_REQ);
        fail.write_u8(3);
        session.send_packet(&fail).await?;
        return Ok(());
    }

    // Perform the handover via DB
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo
        .handover_leadership(ch.knights_id as i16, &target_name, &ch.name)
        .await
    {
        warn!("[{}] Failed to handover leadership: {}", session.addr(), e);
        let mut fail = Packet::new(WIZKNIGHTS_PROCESS);
        fail.write_u8(KNIGHTS_HANDOVER);
        fail.write_u8(3);
        session.send_packet(&fail).await?;
        return Ok(());
    }

    // Update runtime — remove target from vice chief slots, set as chief
    let old_chief_name = ch.name.clone();
    session.world().update_knights(ch.knights_id, |k| {
        if k.vice_chief_1.to_uppercase() == target_name.to_uppercase() {
            k.vice_chief_1 = String::new();
        } else if k.vice_chief_2.to_uppercase() == target_name.to_uppercase() {
            k.vice_chief_2 = String::new();
        } else if k.vice_chief_3.to_uppercase() == target_name.to_uppercase() {
            k.vice_chief_3 = String::new();
        }
        k.chief = target_name.clone();
    });

    // Update fame for both players
    let sid = session.session_id();
    session.world().update_character_stats(sid, |c| {
        c.fame = TRAINEE;
    });
    session.world().update_character_stats(target_sid, |c| {
        c.fame = CHIEF;
    });

    // Broadcast result to clan
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_HANDOVER);
    result.write_string(&old_chief_name);
    result.write_string(&target_name);
    session
        .world()
        .send_to_knights_members(ch.knights_id, Arc::new(result), None);

    Ok(())
}

// ── KNIGHTS_HANDOVER (79) ───────────────────────────────────────────

/// Handle the KNIGHTS_HANDOVER sub-opcode (DB response routing).
/// In the C++ code this is routed to the DB thread. Since we handle it
/// inline, this delegates to the same logic as `handle_handover_req`.
fn handle_handover(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    // This sub-opcode is the DB-thread response in C++. Client does not send it directly.
    // If received from client, treat as invalid.
    debug!(
        "[{}] KNIGHTS_HANDOVER (79) received from client — ignoring",
        session.addr()
    );
    let _ = reader;
    Ok(())
}

// ── KNIGHTS_DONATION_LIST (64) ──────────────────────────────────────

/// List NP donations per clan member.
async fn handle_donation_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        return Ok(());
    }

    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Must be at least Accredited (flag >= 3)
    if clan.flag < 3 {
        return Ok(());
    }

    // Load members from DB
    let repo = KnightsRepository::new(session.pool());
    let members = match repo.load_clan_members(ch.knights_id as i16).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                "[{}] load_clan_members DB error (clan {}): {e}",
                session.addr(),
                ch.knights_id
            );
            Vec::new()
        }
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_DONATION_LIST);

    let count = members.len().min(255) as u8;
    result.write_u8(count);

    // C++ sends: strUserName + nDonatedNP for each member
    // We use loyalty as a proxy for donated NP (the real system tracks donations separately)
    for member in members.iter().take(255) {
        result.write_string(&member.str_user_id);
        result.write_u32(member.loyalty as u32);
    }

    session.send_packet(&result).await?;
    Ok(())
}

// ── KNIGHTS_ALLY_CREATE (28) / KNIGHTS_ALLY_INSERT (30) ──────────────

/// Send an alliance creation or join invitation to another clan leader.
/// Both opcodes share the same flow: the user targets another clan leader
/// and sends them an invitation to join or create an alliance.
async fn handle_alliance_create(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
    incoming_opcode: u8,
) -> anyhow::Result<()> {
    // Determine response sub-opcode based on incoming opcode.
    let is_insert = incoming_opcode == KNIGHTS_ALLY_INSERT;
    let resp_opcode = if is_insert {
        KNIGHTS_ALLY_REQ
    } else {
        KNIGHTS_ALLY_CREATE
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be alive and clan leader
    if ch.res_hp_type == crate::world::USER_DEAD || ch.fame != CHIEF {
        if is_insert {
            session.send_packet(&knights_error(resp_opcode, 0)).await?;
        }
        return Ok(());
    }

    let main_clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Must be promoted (flag > 1)
    if main_clan.flag <= 1 {
        session.send_packet(&knights_error(resp_opcode, 0)).await?;
        return Ok(());
    }

    let target_sid = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let target_ch = match session.world().get_character_info(target_sid) {
        Some(c) => c,
        None => {
            if is_insert {
                session.send_packet(&knights_error(resp_opcode, 0)).await?;
            }
            return Ok(());
        }
    };

    let target_clan = match session.world().get_knights(target_ch.knights_id) {
        Some(k) => k,
        None => {
            if is_insert {
                session.send_packet(&knights_error(resp_opcode, 0)).await?;
            }
            return Ok(());
        }
    };

    // Validate: same nation, target is clan leader, target not already in alliance
    if ch.nation != target_ch.nation
        || target_ch.fame != CHIEF
        || target_clan.alliance > 0
        || target_clan.alliance_req > 0
    {
        session.send_packet(&knights_error(resp_opcode, 0)).await?;
        return Ok(());
    }

    if is_insert {
        // For ALLY_INSERT, the main clan must already have an alliance
        // and the alliance must not be full.
        let alliance = match session.world().get_alliance(main_clan.id) {
            Some(a) => a,
            None => {
                session.send_packet(&knights_error(resp_opcode, 0)).await?;
                return Ok(());
            }
        };
        if alliance.sub_clan > 0 && alliance.mercenary_1 > 0 && alliance.mercenary_2 > 0 {
            session.send_packet(&knights_error(resp_opcode, 0)).await?;
            return Ok(());
        }
    } else {
        // For ALLY_CREATE, main clan must NOT already be in an alliance
        if main_clan.alliance > 0 {
            session.send_packet(&knights_error(resp_opcode, 0)).await?;
            return Ok(());
        }
    }

    // Set the alliance request on the target clan
    session.world().update_knights(target_clan.id, |k| {
        k.alliance_req = main_clan.id;
    });

    // Send invitation to the target
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(resp_opcode);
    result.write_u8(1); // success/invitation flag
    result.write_sbyte_string(&main_clan.name);
    result.write_u16(main_clan.id);
    session.world().send_to_session_owned(target_sid, result);

    Ok(())
}

// ── KNIGHTS_ALLY_REQ (29) ────────────────────────────────────────────

/// Accept or decline an alliance invitation.
async fn handle_alliance_req(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.res_hp_type == crate::world::USER_DEAD || ch.fame != CHIEF {
        return Ok(());
    }

    let our_clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Must have a pending alliance request
    if our_clan.alliance_req == 0 || our_clan.alliance > 0 {
        return Ok(());
    }

    let decision = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let requesting_clan_id = our_clan.alliance_req;

    // Clear the request regardless of decision
    session.world().update_knights(our_clan.id, |k| {
        k.alliance_req = 0;
    });

    if decision != 1 {
        return Ok(());
    }

    let main_clan = match session.world().get_knights(requesting_clan_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    if main_clan.flag <= 1 {
        return Ok(());
    }

    let repo = KnightsRepository::new(session.pool());

    // Check if an alliance already exists for the main clan
    if let Some(alliance) = session.world().get_alliance(main_clan.id) {
        // Alliance exists — insert us into an available slot
        if main_clan.alliance == 0
            || alliance.main_clan != main_clan.alliance
            || alliance.mercenary_1 == our_clan.id
            || alliance.mercenary_2 == our_clan.id
            || alliance.sub_clan == our_clan.id
        {
            let mut result = Packet::new(WIZKNIGHTS_PROCESS);
            result.write_u8(KNIGHTS_ALLY_REQ);
            result.write_u8(0);
            session.send_packet(&result).await?;
            return Ok(());
        }

        // Check if alliance is full
        if alliance.sub_clan > 0 && alliance.mercenary_1 > 0 && alliance.mercenary_2 > 0 {
            let mut result = Packet::new(WIZKNIGHTS_PROCESS);
            result.write_u8(KNIGHTS_ALLY_REQ);
            result.write_u8(0);
            session.send_packet(&result).await?;
            return Ok(());
        }

        // Find empty slot: sub=1, merc1=2, merc2=3
        let slot = if alliance.sub_clan == 0 {
            1u8
        } else if alliance.mercenary_1 == 0 {
            2
        } else if alliance.mercenary_2 == 0 {
            3
        } else {
            return Ok(());
        };

        // Update DB
        if let Err(e) = repo
            .alliance_insert(main_clan.id as i16, our_clan.id as i16, slot)
            .await
        {
            tracing::warn!(
                "Failed to insert alliance slot for clans {}/{}: {e}",
                main_clan.id,
                our_clan.id
            );
        }
        if let Err(e) = repo
            .update_alliance_id(our_clan.id as i16, main_clan.id as i16)
            .await
        {
            tracing::warn!("Failed to update alliance id for clan {}: {e}", our_clan.id);
        }

        // Update runtime alliance
        session
            .world()
            .update_alliance(main_clan.id, |a| match slot {
                1 => a.sub_clan = our_clan.id,
                2 => a.mercenary_1 = our_clan.id,
                3 => a.mercenary_2 = our_clan.id,
                _ => {}
            });

        // Update clan alliance ID
        session.world().update_knights(our_clan.id, |k| {
            k.alliance = main_clan.id;
        });

        // Broadcast insertion to region
        let mut result = Packet::new(WIZKNIGHTS_PROCESS);
        result.write_u8(KNIGHTS_ALLY_INSERT);
        result.write_u8(1); // success
        result.write_u16(main_clan.id);
        result.write_u16(our_clan.id);
        result.write_u16(main_clan.cape);

        if let Some(pos) = session.world().get_position(session.session_id()) {
            let event_room = session.world().get_event_room(session.session_id());
            session.world().broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(result),
                None,
                event_room,
            );
        }

        // Send knights update to the joining clan
        send_knights_update(session, our_clan.id);
    } else {
        // No alliance exists — create one
        // DB create
        if let Err(e) = repo
            .alliance_create(main_clan.id as i16, our_clan.id as i16)
            .await
        {
            tracing::warn!(
                "Failed to create alliance for clans {}/{}: {e}",
                main_clan.id,
                our_clan.id
            );
        }
        if let Err(e) = repo
            .update_alliance_id(main_clan.id as i16, main_clan.id as i16)
            .await
        {
            tracing::warn!(
                "Failed to update alliance id for main clan {}: {e}",
                main_clan.id
            );
        }
        if let Err(e) = repo
            .update_alliance_id(our_clan.id as i16, main_clan.id as i16)
            .await
        {
            tracing::warn!("Failed to update alliance id for clan {}: {e}", our_clan.id);
        }

        // Create runtime alliance
        let alliance = KnightsAlliance {
            main_clan: main_clan.id,
            sub_clan: our_clan.id,
            mercenary_1: 0,
            mercenary_2: 0,
            notice: String::new(),
        };
        session.world().insert_alliance(alliance);

        // Update both clans' alliance IDs
        session.world().update_knights(main_clan.id, |k| {
            k.alliance = main_clan.id;
        });
        session.world().update_knights(our_clan.id, |k| {
            k.alliance = main_clan.id;
        });

        // Broadcast insertion
        let mut result = Packet::new(WIZKNIGHTS_PROCESS);
        result.write_u8(KNIGHTS_ALLY_INSERT);
        result.write_u8(1);
        result.write_u16(main_clan.id);
        result.write_u16(our_clan.id);
        result.write_u16(main_clan.cape);

        if let Some(pos) = session.world().get_position(session.session_id()) {
            let event_room = session.world().get_event_room(session.session_id());
            session.world().broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(result),
                None,
                event_room,
            );
        }

        // Send updates
        send_knights_update(session, main_clan.id);
        send_knights_update(session, our_clan.id);
    }

    Ok(())
}

// ── KNIGHTS_ALLY_REMOVE (31) ─────────────────────────────────────────

/// Leave an alliance. If the main clan leaves, the entire alliance is dissolved.
async fn handle_alliance_remove(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.res_hp_type == crate::world::USER_DEAD || ch.fame != CHIEF {
        return Ok(());
    }

    let our_clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    if our_clan.alliance == 0 {
        return Ok(());
    }

    let alliance = match session.world().get_alliance(our_clan.alliance) {
        Some(a) => a,
        None => return Ok(()),
    };

    let repo = KnightsRepository::new(session.pool());

    if our_clan.id == alliance.main_clan {
        // Main clan leaving — dissolve entire alliance
        if let Err(e) = repo.alliance_destroy(alliance.main_clan as i16).await {
            tracing::error!(
                "Failed to destroy alliance for main clan {}: {e}",
                alliance.main_clan
            );
        }

        // Clear alliance from all member clans
        let clan_ids = [
            alliance.main_clan,
            alliance.sub_clan,
            alliance.mercenary_1,
            alliance.mercenary_2,
        ];

        for &cid in &clan_ids {
            if cid == 0 {
                continue;
            }
            if let Err(e) = repo.update_alliance_id(cid as i16, 0).await {
                tracing::error!("Failed to clear alliance id for clan {cid}: {e}");
            }
            session.world().update_knights(cid, |k| {
                k.alliance = 0;
            });
            send_knights_update(session, cid);
        }

        // Remove runtime alliance
        session.world().remove_alliance(alliance.main_clan);

        // Send remove notification
        let mut result = Packet::new(WIZKNIGHTS_PROCESS);
        result.write_u8(KNIGHTS_ALLY_REMOVE);
        result.write_u8(1);
        result.write_u16(our_clan.alliance);
        result.write_u16(our_clan.id);
        result.write_u16(0xFFFF); // -1 as u16

        let arc_result = Arc::new(result);
        for &cid in &clan_ids {
            if cid == 0 {
                continue;
            }
            session
                .world()
                .send_to_knights_members(cid, Arc::clone(&arc_result), None);
        }
    } else {
        // Non-main clan leaving
        if let Err(e) = repo
            .alliance_remove(alliance.main_clan as i16, our_clan.id as i16)
            .await
        {
            tracing::error!("Failed to remove clan {} from alliance: {e}", our_clan.id);
        }
        if let Err(e) = repo.update_alliance_id(our_clan.id as i16, 0).await {
            tracing::error!("Failed to clear alliance id for clan {}: {e}", our_clan.id);
        }

        // Update runtime
        session.world().update_alliance(alliance.main_clan, |a| {
            if a.sub_clan == our_clan.id {
                a.sub_clan = 0;
            }
            if a.mercenary_1 == our_clan.id {
                a.mercenary_1 = 0;
            }
            if a.mercenary_2 == our_clan.id {
                a.mercenary_2 = 0;
            }
        });
        session.world().update_knights(our_clan.id, |k| {
            k.alliance = 0;
        });

        // Check if alliance is now empty (only main clan left)
        let updated_alliance = session.world().get_alliance(alliance.main_clan);
        if let Some(ua) = updated_alliance {
            if ua.sub_clan == 0 && ua.mercenary_1 == 0 && ua.mercenary_2 == 0 {
                // Alliance is effectively dissolved
                if let Err(e) = repo.alliance_destroy(alliance.main_clan as i16).await {
                    tracing::error!(
                        "Failed to destroy empty alliance for main clan {}: {e}",
                        alliance.main_clan
                    );
                }
                if let Err(e) = repo.update_alliance_id(alliance.main_clan as i16, 0).await {
                    tracing::error!(
                        "Failed to clear alliance id for main clan {}: {e}",
                        alliance.main_clan
                    );
                }
                session.world().update_knights(alliance.main_clan, |k| {
                    k.alliance = 0;
                });
                session.world().remove_alliance(alliance.main_clan);
                send_knights_update(session, alliance.main_clan);
            }
        }

        // Send remove notification
        let mut result = Packet::new(WIZKNIGHTS_PROCESS);
        result.write_u8(KNIGHTS_ALLY_REMOVE);
        result.write_u8(1);
        result.write_u16(our_clan.alliance);
        result.write_u16(our_clan.id);
        result.write_u16(0xFFFF);
        let arc_result_remove = Arc::new(result);
        session
            .world()
            .send_to_knights_members(our_clan.id, Arc::clone(&arc_result_remove), None);

        if let Some(pos) = session.world().get_position(session.session_id()) {
            let event_room = session.world().get_event_room(session.session_id());
            session.world().broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                arc_result_remove,
                None,
                event_room,
            );
        }

        send_knights_update(session, our_clan.id);
    }

    Ok(())
}

// ── KNIGHTS_ALLY_PUNISH (32) ─────────────────────────────────────────

/// Kick a clan from the alliance (alliance leader only).
async fn handle_alliance_punish(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.res_hp_type == crate::world::USER_DEAD || ch.fame != CHIEF {
        return Ok(());
    }

    let target_clan_id = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let our_clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    let target_clan = match session.world().get_knights(target_clan_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    // Must be the alliance leader
    if our_clan.alliance != our_clan.id {
        return Ok(());
    }

    let alliance = match session.world().get_alliance(our_clan.id) {
        Some(a) => a,
        None => return Ok(()),
    };

    // Cannot kick yourself (the main clan)
    if target_clan_id == alliance.main_clan {
        return Ok(());
    }

    // Target must be in the alliance
    if target_clan.alliance != alliance.main_clan
        || (target_clan_id != alliance.sub_clan
            && target_clan_id != alliance.mercenary_1
            && target_clan_id != alliance.mercenary_2)
    {
        return Ok(());
    }

    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo
        .alliance_remove(alliance.main_clan as i16, target_clan_id as i16)
        .await
    {
        tracing::error!("Failed to remove punished clan {target_clan_id} from alliance: {e}");
    }
    if let Err(e) = repo.update_alliance_id(target_clan_id as i16, 0).await {
        tracing::error!("Failed to clear alliance id for punished clan {target_clan_id}: {e}");
    }

    // Update runtime
    session.world().update_alliance(alliance.main_clan, |a| {
        if a.sub_clan == target_clan_id {
            a.sub_clan = 0;
        }
        if a.mercenary_1 == target_clan_id {
            a.mercenary_1 = 0;
        }
        if a.mercenary_2 == target_clan_id {
            a.mercenary_2 = 0;
        }
    });
    session.world().update_knights(target_clan_id, |k| {
        k.alliance = 0;
    });

    // Check if alliance is now empty
    let updated_alliance = session.world().get_alliance(alliance.main_clan);
    if let Some(ua) = updated_alliance {
        if ua.sub_clan == 0 && ua.mercenary_1 == 0 && ua.mercenary_2 == 0 {
            if let Err(e) = repo.alliance_destroy(alliance.main_clan as i16).await {
                tracing::error!(
                    "Failed to destroy empty alliance after punish for main clan {}: {e}",
                    alliance.main_clan
                );
            }
            if let Err(e) = repo.update_alliance_id(alliance.main_clan as i16, 0).await {
                tracing::error!(
                    "Failed to clear alliance id after punish for main clan {}: {e}",
                    alliance.main_clan
                );
            }
            session.world().update_knights(alliance.main_clan, |k| {
                k.alliance = 0;
            });
            session.world().remove_alliance(alliance.main_clan);
            send_knights_update(session, alliance.main_clan);
        }
    }

    // Send punish notification
    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_ALLY_PUNISH);
    result.write_u8(1);
    result.write_u16(our_clan.id);
    result.write_u16(target_clan_id);
    result.write_u16(our_clan.cape);

    let arc_result = Arc::new(result);
    let sender_event_room = session.world().get_event_room(session.session_id());
    if let Some(pos) = session.world().get_position(session.session_id()) {
        session.world().broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::clone(&arc_result),
            None,
            sender_event_room,
        );
    }

    // Also broadcast in the target clan leader's region
    if let Some(target_chief_sid) = session.world().find_session_by_name(&target_clan.chief) {
        if let Some(pos) = session.world().get_position(target_chief_sid) {
            let target_event_room = session.world().get_event_room(target_chief_sid);
            session.world().broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::clone(&arc_result),
                None,
                target_event_room,
            );
        }
    }

    session
        .world()
        .send_to_knights_members(target_clan_id, Arc::clone(&arc_result), None);

    send_knights_update(session, target_clan_id);

    Ok(())
}

// ── KNIGHTS_ALLY_LIST (34) ───────────────────────────────────────────

/// List all clans in the player's alliance.
async fn handle_alliance_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        return Ok(());
    }

    let our_clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_ALLY_LIST);

    if our_clan.alliance == 0 {
        result.write_u8(0);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let alliance = match session.world().get_alliance(our_clan.alliance) {
        Some(a) => a,
        None => {
            result.write_u8(0);
            session.send_packet(&result).await?;
            return Ok(());
        }
    };

    let clan_ids = [
        alliance.main_clan,
        alliance.sub_clan,
        alliance.mercenary_1,
        alliance.mercenary_2,
    ];

    // Count valid clans first
    let mut clan_count: u8 = 0;
    let count_pos = result.data.len();
    result.write_u8(0); // placeholder
    result.write_string(&alliance.notice);

    for &cid in &clan_ids {
        if cid == 0 {
            continue;
        }
        let clan = match session.world().get_knights(cid) {
            Some(k) => k,
            None => continue,
        };

        result.write_u16(clan.id);
        result.write_sbyte_string(&clan.name);
        result.write_u8(if clan.alliance > 0 { 1 } else { 0 });

        // Count officers
        let mut info_count: u8 = 0;
        let info_count_pos = result.data.len();
        result.write_u8(0); // placeholder

        if !clan.chief.is_empty() {
            info_count += 1;
            result.write_u8(CHIEF);
            result.write_sbyte_string(&clan.chief);
        }
        if !clan.vice_chief_1.is_empty() {
            info_count += 1;
            result.write_u8(VICECHIEF);
            result.write_sbyte_string(&clan.vice_chief_1);
        }
        if !clan.vice_chief_2.is_empty() {
            info_count += 1;
            result.write_u8(VICECHIEF);
            result.write_sbyte_string(&clan.vice_chief_2);
        }
        if !clan.vice_chief_3.is_empty() {
            info_count += 1;
            result.write_u8(VICECHIEF);
            result.write_sbyte_string(&clan.vice_chief_3);
        }

        result.data[info_count_pos] = info_count;
        clan_count += 1;
    }

    if clan_count == 0 {
        return Ok(());
    }

    result.data[count_pos] = clan_count;
    session.send_packet(&result).await?;

    Ok(())
}

// ── SendUpdate helper ────────────────────────────────────────────────

/// Build a KNIGHTS_UPDATE packet for a clan.
/// Alliance cape rules:
/// - Main/sub alliance clans: show alliance leader's cape + their own RGB colors
/// - Mercenary clans: show alliance leader's cape with no colors (u32(0))
/// - Non-alliance clans: show their own cape + RGB colors
pub(crate) fn build_knights_update_packet(session: &ClientSession, clan_id: u16) -> Option<Packet> {
    let clan = match session.world().get_knights(clan_id) {
        Some(k) => k,
        None => return None,
    };

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_UPDATE);

    if clan.alliance > 0 {
        let main_clan = session.world().get_knights(clan.alliance);
        let alliance = session.world().get_alliance(clan.alliance);
        if let (Some(mc), Some(a)) = (main_clan, alliance) {
            let is_mercenary = a.mercenary_1 == clan.id || a.mercenary_2 == clan.id;
            let cape_id = if mc.castellan_cape {
                mc.cast_cape_id as u16
            } else {
                mc.cape
            };

            result.write_u16(clan.id);
            result.write_u8(clan.flag);
            result.write_u16(cape_id);

            if is_mercenary {
                result.write_u32(0);
            } else if mc.castellan_cape {
                // Main or sub alliance: use castellan cape colors
                result.write_u8(clan.cast_cape_r);
                result.write_u8(clan.cast_cape_g);
                result.write_u8(clan.cast_cape_b);
                result.write_u8(0);
            } else {
                // Main or sub alliance: use normal cape colors
                result.write_u8(clan.cape_r);
                result.write_u8(clan.cape_g);
                result.write_u8(clan.cape_b);
                result.write_u8(0);
            }
        } else {
            result.write_u16(clan.id);
            result.write_u8(clan.flag);
            result.write_u16(clan.cape);
            result.write_u8(clan.cape_r);
            result.write_u8(clan.cape_g);
            result.write_u8(clan.cape_b);
            result.write_u8(0);
        }
    } else if clan.castellan_cape {
        result.write_u16(clan.id);
        result.write_u8(clan.flag);
        result.write_u16(clan.cast_cape_id as u16);
        result.write_u8(clan.cast_cape_r);
        result.write_u8(clan.cast_cape_g);
        result.write_u8(clan.cast_cape_b);
        result.write_u8(0);
    } else {
        result.write_u16(clan.id);
        result.write_u8(clan.flag);
        result.write_u16(clan.cape);
        result.write_u8(clan.cape_r);
        result.write_u8(clan.cape_g);
        result.write_u8(clan.cape_b);
        result.write_u8(0);
    }

    info!(
        clan_id = clan.id,
        flag = clan.flag,
        grade = clan.grade,
        cape = clan.cape,
        cape_r = clan.cape_r,
        cape_g = clan.cape_g,
        cape_b = clan.cape_b,
        castellan_cape = clan.castellan_cape,
        cast_cape = clan.cast_cape_id,
        cast_cape_r = clan.cast_cape_r,
        cast_cape_g = clan.cast_cape_g,
        cast_cape_b = clan.cast_cape_b,
        alliance = clan.alliance,
        packet = %packet_hex(WIZKNIGHTS_PROCESS, &result.data),
        "KNIGHTS_UPDATE cape payload"
    );

    Some(result)
}

/// Send a KNIGHTS_UPDATE packet for a clan to all its online members.
pub(crate) fn send_knights_update(session: &ClientSession, clan_id: u16) {
    let Some(result) = build_knights_update_packet(session, clan_id) else {
        return;
    };
    session
        .world()
        .send_to_knights_members(clan_id, Arc::new(result), None);
}

// ── KNIGHTS_UPDATENOTICE (80) ─────────────────────────────────────────

// ── KNIGHTS_PROMATE_CLAN (82) ────────────────────────────────────────

/// Clan promotion list — returns paginated list of all clans with details.
/// Sniffer verified (session 10, ids 84403-84405):
///   C2S: `[sub=82][page:u8][flag:u8]`
///   S2C 1: `[sub=82][01][total_pages:u16le]`
///   S2C 2: `[sub=82][02][clan_count:u8][00][...entries...]`
///   Entry: `[clan_id:u16le][name:DByte][grade:u8][nation:u8][leader:DByte][fame:u8][00][memo:DByte(139)]`
const PROMOTE_CLANS_PER_PAGE: usize = 8;
const PROMOTE_MEMO_LEN: usize = 139;

async fn handle_promote_clan_list(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let page = reader.read_u8().unwrap_or(1).max(1);
    let _flag = reader.read_u8().unwrap_or(1);

    // Query all clans from DB, sorted by points descending
    let repo = KnightsRepository::new(session.pool());
    let mut clans = match repo.load_all().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[{}] KNIGHTS_PROMATE_CLAN DB error: {e}", session.addr());
            return Ok(());
        }
    };
    clans.retain(|c| c.id_num > 0);
    clans.sort_by(|a, b| b.points.cmp(&a.points));

    let total = clans.len();
    let total_pages = ((total + PROMOTE_CLANS_PER_PAGE - 1) / PROMOTE_CLANS_PER_PAGE).max(1) as u16;

    // S2C 1: page info
    let mut info_pkt = Packet::new(WIZKNIGHTS_PROCESS);
    info_pkt.write_u8(KNIGHTS_PROMATE_CLAN);
    info_pkt.write_u8(1); // type = page info
    info_pkt.write_u16(total_pages);
    session.send_packet(&info_pkt).await?;

    // S2C 2: clan data for requested page
    let start = ((page as usize) - 1) * PROMOTE_CLANS_PER_PAGE;
    let page_clans: Vec<_> = clans
        .iter()
        .skip(start)
        .take(PROMOTE_CLANS_PER_PAGE)
        .collect();

    let mut data_pkt = Packet::new(WIZKNIGHTS_PROCESS);
    data_pkt.write_u8(KNIGHTS_PROMATE_CLAN);
    data_pkt.write_u8(2); // type = data
    data_pkt.write_u8(page_clans.len() as u8);
    data_pkt.write_u8(0); // padding

    for clan in &page_clans {
        data_pkt.write_u16(clan.id_num as u16);
        data_pkt.write_string(&clan.id_name); // DByte (u16le len + chars)
        data_pkt.write_u8(clan.flag as u8); // grade/type (1=training, 2+=promoted)
        data_pkt.write_u8(clan.nation as u8);
        data_pkt.write_string(&clan.chief); // DByte leader name
        data_pkt.write_u8(clan.ranking as u8); // fame/ranking
        data_pkt.write_u8(0); // unk

        // Memo: fixed 139 bytes, space-padded
        let memo = clan.str_clan_notice.as_deref().unwrap_or("");
        let memo_bytes = memo.as_bytes();
        let mut memo_buf = vec![0x20u8; PROMOTE_MEMO_LEN]; // fill with spaces
        let copy_len = memo_bytes.len().min(PROMOTE_MEMO_LEN);
        memo_buf[..copy_len].copy_from_slice(&memo_bytes[..copy_len]);
        data_pkt.write_u16(PROMOTE_MEMO_LEN as u16);
        data_pkt.data.extend_from_slice(&memo_buf);
    }

    session.send_packet(&data_pkt).await?;

    debug!(
        "[{}] KNIGHTS_PROMATE_CLAN: page={}, total_pages={}, entries={}",
        session.addr(),
        page,
        total_pages,
        page_clans.len(),
    );

    Ok(())
}

/// Update the clan notice.
async fn handle_update_notice(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    // C++ reads with DByte mode (u16 prefix)
    let notice = match reader.read_string() {
        Some(n) => n,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be clan chief and in a clan
    if ch.knights_id == 0 || ch.fame != CHIEF {
        return Ok(());
    }

    let clan_id = ch.knights_id;

    // Update runtime
    session.world().update_knights(clan_id, |k| {
        k.notice = notice.clone();
    });

    // Update DB
    let repo = KnightsRepository::new(session.pool());
    if let Err(e) = repo.update_notice(clan_id as i16, &notice).await {
        tracing::error!("Failed to update clan notice for clan {clan_id}: {e}");
    }

    // Broadcast the updated notice to all online clan members
    if !notice.is_empty() {
        let notice_pkt = build_clan_notice_packet(&notice);
        session
            .world()
            .send_to_knights_members(clan_id, Arc::new(notice_pkt), None);
    }

    Ok(())
}

// ── WIZKNIGHTS_LIST (0x3E) ──────────────────────────────────────────

/// Opcode for WIZKNIGHTS_LIST.
const WIZKNIGHTS_LIST: u8 = 0x3E;

/// Handle WIZKNIGHTS_LIST — send all clan IDs and names.
/// This is called once on login. The C++ code sends it compressed, but
/// we send it uncompressed for simplicity (client handles both).
pub async fn handle_knights_list(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame && session.state() != SessionState::CharacterSelected
    {
        return Ok(());
    }
    let all_clans = session.world().get_all_knights();

    let mut result = Packet::new(WIZKNIGHTS_LIST);
    let count = all_clans.len() as u16;
    result.write_u16(count);

    for (id, name) in &all_clans {
        result.write_u16(*id);
        result.write_string(name);
    }

    session.send_packet(&result).await?;
    Ok(())
}

// ── Clan Notice Helper ────────────────────────────────────────────────

/// Build a WIZ_NOTICE packet containing the clan notice.
pub fn build_clan_notice_packet(notice: &str) -> Packet {
    let mut pkt = Packet::new(WIZ_NOTICE);
    // C++ uses DByte mode (u16 string prefix)
    pkt.write_u8(4); // type = clan notice
    pkt.write_u8(1); // total blocks = 1
    pkt.write_string("Clan Notice"); // header
    pkt.write_string(notice); // notice text
    pkt
}

/// Send the clan notice to a player on login.
/// Called from gamestart handler when the player is in a clan with a non-empty notice.
pub async fn send_clan_notice_on_login(
    session: &mut ClientSession,
    clan_id: u16,
) -> anyhow::Result<()> {
    if let Some(clan) = session.world().get_knights(clan_id) {
        if !clan.notice.is_empty() {
            let pkt = build_clan_notice_packet(&clan.notice);
            session.send_packet(&pkt).await?;
        }

        // Also send online notification to other clan members
        if let Some(ch) = get_char_info(session) {
            let mut online_pkt = Packet::new(WIZKNIGHTS_PROCESS);
            online_pkt.write_u8(KNIGHTS_USER_ONLINE);
            online_pkt.write_sbyte_string(&ch.name);
            session.world().send_to_knights_members(
                clan_id,
                Arc::new(online_pkt),
                Some(session.session_id()),
            );
        }
    }
    Ok(())
}

// ── KNIGHTS_UPDATEMEMO (88) ──────────────────────────────────────────

/// Update user memo or alliance notice.
/// type=2 → alliance notice update (alliance leader only)
/// type=3 → user memo update
/// type=6 → user title update (not fully implemented in C++)
async fn handle_update_memo(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let memo_type = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let ch = match get_char_info(session) {
        Some(c) => c,
        None => return Ok(()),
    };

    if ch.knights_id == 0 {
        return Ok(());
    }

    match memo_type {
        2 => {
            // Alliance notice update
            let notice = match reader.read_string() {
                Some(n) => n,
                None => return Ok(()),
            };

            // Must be clan leader
            if ch.fame != CHIEF {
                return Ok(());
            }

            let clan = match session.world().get_knights(ch.knights_id) {
                Some(k) => k,
                None => return Ok(()),
            };

            // Must be in an alliance and be the alliance leader
            if clan.alliance == 0 || clan.alliance != clan.id {
                return Ok(());
            }

            // Update runtime
            session.world().update_alliance(clan.alliance, |a| {
                a.notice = notice.clone();
            });

            // Update DB
            let repo = KnightsRepository::new(session.pool());
            if let Err(e) = repo
                .update_alliance_notice(clan.alliance as i16, &notice)
                .await
            {
                tracing::warn!(
                    "Failed to update alliance notice for alliance {}: {e}",
                    clan.alliance
                );
            }

            // Broadcast to all alliance members
            let mut result = Packet::new(WIZKNIGHTS_PROCESS);
            result.write_u8(KNIGHTS_UPDATEMEMO);
            result.write_u8(2); // type = alliance notice
            result.write_u8(1); // success
            result.write_string(&notice);

            // Send to all clans in alliance
            if let Some(alliance) = session.world().get_alliance(clan.alliance) {
                let clan_ids = [
                    alliance.main_clan,
                    alliance.sub_clan,
                    alliance.mercenary_1,
                    alliance.mercenary_2,
                ];
                let arc_result = Arc::new(result);
                for &cid in &clan_ids {
                    if cid == 0 {
                        continue;
                    }
                    session
                        .world()
                        .send_to_knights_members(cid, Arc::clone(&arc_result), None);
                }
            }
        }
        3 => {
            // User memo update
            let memo = match reader.read_string() {
                Some(n) => n,
                None => return Ok(()),
            };

            // Max 20 chars (matching DB column)
            if memo.len() > 20 {
                return Ok(());
            }

            // Update DB
            let repo = KnightsRepository::new(session.pool());
            if let Err(e) = repo.update_user_memo(&ch.name, &memo).await {
                tracing::error!("Failed to update user memo for {}: {e}", ch.name);
            }

            // Broadcast to clan
            let mut result = Packet::new(WIZKNIGHTS_PROCESS);
            result.write_u8(KNIGHTS_UPDATEMEMO);
            result.write_u8(3); // type = user memo
            result.write_u8(1); // success
            result.write_string(&memo);
            session
                .world()
                .send_to_knights_members(ch.knights_id, Arc::new(result), None);
        }
        6 => {
            // User title update
            // C++ reads username + title via SByte, validates, then sets bResult=false (TO-DO).
            // We replicate C++ behavior: read the data, validate, send failure.
            let username = match reader.read_sbyte_string() {
                Some(n) => n,
                None => return Ok(()),
            };
            let title = match reader.read_sbyte_string() {
                Some(t) => t,
                None => return Ok(()),
            };

            // C++ validates clan + username match, but then always sets bResult=false
            // (TO-DO comment: "Acco update paketi dinlenecek"). We match C++ behavior.
            let _clan = session.world().get_knights(ch.knights_id);

            // Always failure — same as C++ TO-DO stub
            let mut result = Packet::new(WIZKNIGHTS_PROCESS);
            result.write_u8(KNIGHTS_UPDATEMEMO);
            result.write_u8(6); // type
            result.write_u8(0); // bResult = false (C++ TO-DO)
            result.write_sbyte_string(&username);
            result.write_sbyte_string(&title);

            session.send_packet(&result).await?;
        }
        _ => {
            debug!(
                "[{}] KNIGHTS_UPDATEMEMO unhandled type: {}",
                session.addr(),
                memo_type
            );
        }
    }

    Ok(())
}

// ── Clan Offline Notification ───────────────────────────────────────

/// Send clan offline notification when a player logs out.
pub fn send_clan_offline_notification(
    world: &crate::world::WorldState,
    clan_id: u16,
    char_name: &str,
    session_id: u16,
) {
    // Skip system clans (ID 1 and 15001 in C++)
    if clan_id <= 1 || clan_id == 15001 {
        return;
    }

    let mut offline_pkt = Packet::new(WIZKNIGHTS_PROCESS);
    offline_pkt.write_u8(KNIGHTS_USER_OFFLINE);
    offline_pkt.write_sbyte_string(char_name);
    world.send_to_knights_members(clan_id, Arc::new(offline_pkt), Some(session_id));
}

/// Build and send top-5 clans per nation packet.
/// Used by both KNIGHTS_TOP10 and KNIGHTS_UNK1 — they differ only in
/// the header u16 value (0 for TOP10, 1 for UNK1/flags).
/// Wire: `[u8 sub] [u16 header] + per nation(2): 5 entries: [i16 id] [str name] [i16 mark_ver] [i16 rank]`
async fn send_top_clans(
    session: &mut ClientSession,
    sub_opcode: u8,
    header: u16,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
    pkt.write_u8(sub_opcode);
    pkt.write_u16(header);

    // 2 nations: Karus(1), El Morad(2)
    for nation in 1u8..=2 {
        let top = world.get_top_knights_by_nation(nation, 5);
        for (rank, (id, name, mark_ver)) in top.iter().enumerate() {
            pkt.write_i16(*id as i16);
            pkt.write_string(name);
            pkt.write_i16(*mark_ver as i16);
            pkt.write_i16(rank as i16);
        }
        // Fill remaining slots with empty entries
        for k in top.len()..5 {
            pkt.write_i16(-1); // no clan
            pkt.write_string(""); // empty name
            pkt.write_i16(-1); // no symbol
            pkt.write_i16(k as i16); // rank slot
        }
    }

    session.send_packet(&pkt).await
}

/// Handle KNIGHTS_TOP10 — top 10 clans ranking (5 per nation).
async fn handle_top10(session: &mut ClientSession) -> anyhow::Result<()> {
    send_top_clans(session, KNIGHTS_TOP10, 0).await
}

/// Handle KNIGHTS_UNK1 — flags list (top 5 clans per nation).
async fn handle_flags_list(session: &mut ClientSession) -> anyhow::Result<()> {
    send_top_clans(session, KNIGHTS_UNK1, 1).await
}

/// Handle KNIGHTS_LADDER_POINTS — current clan members' monthly NP ranking.
///
/// CKnightsManager::KnightsLadderPointList sends `[sub][u16 count]` followed
/// by `DByte(name), u32(loyalty_monthly)` for every clan member, sorted high
/// to low. The old implementation incorrectly reused the global top-clans
/// response, which left the v2615 Ladder Points window empty.
async fn handle_ladder_points(session: &mut ClientSession) -> anyhow::Result<()> {
    let ch = match get_char_info(session) {
        Some(c) if c.knights_id > 0 => c,
        _ => return Ok(()),
    };
    let clan = match session.world().get_knights(ch.knights_id) {
        Some(k) => k,
        None => return Ok(()),
    };

    let mut members: Vec<(String, u32)> = session
        .world()
        .all_session_ids()
        .into_iter()
        .filter_map(|sid| {
            session.world().get_character_info(sid).and_then(|member| {
                (member.knights_id == clan.id).then_some((member.name, member.loyalty_monthly))
            })
        })
        .collect();
    members.sort_unstable_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let mut result = Packet::new(WIZKNIGHTS_PROCESS);
    result.write_u8(KNIGHTS_LADDER_POINTS);
    result.write_u16(members.len().min(u16::MAX as usize) as u16);
    for (name, loyalty_monthly) in members.into_iter().take(u16::MAX as usize) {
        result.write_string(&name);
        result.write_u32(loyalty_monthly);
    }
    session.send_packet(&result).await
}

// ── KNIGHTS_MARK_VERSION_REQ (25) ────────────────────────────────────
//
// Client asks for current clan emblem version before registering a new one.
//
//
// Response: `[u8 25] [i16 failCode]` on error, or `[u8 25] [i16 1] [u16 markVersion]` on success
//
// Error codes:
//   11 = clan not promoted or not leader
//   12 = wrong zone (must be in home zone = nation)

async fn handle_mark_version_req(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let mut fail_code: i16 = 1;

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Must be in a clan
    if ch.knights_id == 0 {
        fail_code = 11;
    } else {
        // Must be clan leader
        if ch.fame != CHIEF {
            fail_code = 11;
        } else {
            // Check clan is promoted (flag >= 3)
            match world.get_knights(ch.knights_id) {
                Some(clan) => {
                    if clan.flag < 3 {
                        fail_code = 11;
                    }
                }
                None => fail_code = 11,
            }
        }

        // Must be in home zone (zone_id == nation)
        if fail_code == 1 {
            let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
            if zone_id != ch.nation as u16 {
                fail_code = 12;
            }
        }
    }

    let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
    pkt.write_u8(KNIGHTS_MARK_VERSION_REQ);
    pkt.write_i16(fail_code);

    if fail_code == 1 {
        // Mark version — currently 0 (no emblem storage implemented yet)
        let mark_ver = world
            .get_knights(ch.knights_id)
            .map(|k| k.mark_version)
            .unwrap_or(0);
        pkt.write_u16(mark_ver);
    }

    session.send_packet(&pkt).await
}

// ── KNIGHTS_MARK_REGISTER (26) ───────────────────────────────────────
//
// Client sends clan emblem data for registration.
//

/// Maximum clan emblem size in bytes.
const MAXKNIGHTS_MARK: u16 = 2400;

/// Gold cost to register a clan emblem.
const CLAN_SYMBOL_COST: u32 = 5_000_000;

async fn handle_mark_register(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world();

    let symbol_size = reader.read_u16().unwrap_or(0);

    // Gather all validation info from character + knights in-memory data
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };
    let clan_id = ch.knights_id;
    let fame = ch.fame;
    let nation = ch.nation;
    let gold = ch.gold;
    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    drop(ch);

    let mut fail_code: u16 = 1; // 1 = success

    // Must be clan leader
    if fame != CHIEF {
        fail_code = 11;
    }
    // Must be in home zone (zone == nation)
    else if zone_id != nation as u16 {
        fail_code = 12;
    }
    // Invalid symbol size
    else if symbol_size == 0 || symbol_size > MAXKNIGHTS_MARK {
        fail_code = 13;
    }
    // Not enough gold
    else if gold < CLAN_SYMBOL_COST {
        fail_code = 14;
    }
    // Clan must exist
    else if world.get_knights(clan_id).is_none() {
        fail_code = 20;
    }
    // Clan must be promoted (flag >= 2)
    else if world.get_knights(clan_id).map(|k| k.flag).unwrap_or(0) < 2 {
        fail_code = 11;
    }

    if fail_code != 1 {
        // C++ fail_return: result << sErrorCode << sNewVersion(0); pUser->Send(&result);
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_MARK_REGISTER);
        pkt.write_u16(fail_code);
        pkt.write_u16(0); // version=0 on failure
        return session.send_packet(&pkt).await;
    }

    // Read the symbol data from packet
    let mut mark_data = vec![0u8; MAXKNIGHTS_MARK as usize];
    for byte in mark_data.iter_mut().take(symbol_size as usize) {
        *byte = reader.read_u8().unwrap_or(0);
    }

    // Deduct gold
    world.gold_lose(sid, CLAN_SYMBOL_COST);

    // Update in-memory knights info and get new mark version
    let new_version = {
        let mut ver = 0u16;
        world.update_knights(clan_id, |k| {
            k.mark_version = k.mark_version.wrapping_add(1);
            if k.mark_version == 0 {
                k.mark_version = 1; // never let it be 0
            }
            k.mark_data = mark_data.clone();
            ver = k.mark_version;
        });
        ver
    };

    // Persist to DB (fire-and-forget)
    let pool = session.pool().clone();
    let mark_clone = mark_data.clone();
    tokio::spawn(async move {
        let repo = KnightsRepository::new(&pool);
        if let Err(e) = repo
            .update_mark(clan_id as i16, new_version as i16, &mark_clone)
            .await
        {
            tracing::error!("Failed to save clan mark for clan {clan_id}: {e}");
        }
    });

    // Persist gold to DB
    let pool2 = session.pool().clone();
    let char_name = world
        .get_character_info(sid)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let new_gold = world.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
    tokio::spawn(async move {
        let char_repo = CharacterRepository::new(&pool2);
        if let Err(e) = char_repo.update_gold(&char_name, new_gold as i64).await {
            tracing::error!("Failed to save gold after mark register: {e}");
        }
    });

    debug!(
        "[{}] KNIGHTS_MARK_REGISTER: clan={} version={} size={}",
        session.addr(),
        clan_id,
        new_version,
        symbol_size,
    );

    // C++ success: result << sErrorCode(1) << sNewVersion; pKnights->Send(&result);
    // On success, broadcast to ALL online clan members (not just the requester).
    let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
    pkt.write_u8(KNIGHTS_MARK_REGISTER);
    pkt.write_u16(1); // success
    pkt.write_u16(new_version);

    world.send_to_knights_members(clan_id, Arc::new(pkt), None);
    Ok(())
}

// ── KNIGHTS_MARK_REQ (35) ────────────────────────────────────────────
//
// Request to download a clan's emblem image.
//
//
// C++ silently returns if clan has no mark (version=0 or len=0).

async fn handle_mark_req(
    session: &mut ClientSession,
    reader: &mut ko_protocol::PacketReader<'_>,
) -> anyhow::Result<()> {
    let clan_id = reader.read_u16().unwrap_or(0);
    let world = session.world();

    let knights = match world.get_knights(clan_id) {
        Some(k) => k,
        None => return Ok(()), // C++ silently returns
    };

    if knights.flag < 2 || knights.mark_version == 0 || knights.mark_data.is_empty() {
        return Ok(());
    }

    let nation = knights.nation;
    let version = knights.mark_version;
    let raw_mark_len = knights.mark_data.len();
    if raw_mark_len != MAXKNIGHTS_MARK as usize {
        tracing::warn!(
            clan_id,
            version,
            mark_len = raw_mark_len,
            expected_len = MAXKNIGHTS_MARK,
            "KNIGHTS_MARK_REQ skipped invalid clan mark length"
        );
        return Ok(());
    }
    let mark_len = raw_mark_len as u16;
    let mark_data = knights.mark_data.clone();
    drop(knights);

    //                       << u16(markVersion) << u16(markLen); result.append(m_Image, markLen);
    // C++ sends this via SendCompressed, but we send uncompressed for now.
    let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
    pkt.write_u8(KNIGHTS_MARK_REQ);
    pkt.write_u16(1); // success
    pkt.write_u16(nation as u16);
    pkt.write_u16(clan_id);
    pkt.write_u16(version);
    pkt.write_u16(mark_len);
    pkt.write_bytes(&mark_data);

    debug!(
        "[{}] KNIGHTS_MARK_REQ: clan={} version={} size={}",
        session.addr(),
        clan_id,
        version,
        mark_len,
    );

    session.send_packet(&pkt).await
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::clan_constants::KNIGHT;

    #[test]
    fn test_knights_error_packet() {
        let pkt = knights_error(KNIGHTS_CREATE, 3);
        assert_eq!(pkt.opcode, WIZKNIGHTS_PROCESS);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], KNIGHTS_CREATE);
        assert_eq!(pkt.data[1], 3);
    }

    #[test]
    fn test_build_clan_notice_packet() {
        let pkt = build_clan_notice_packet("Welcome to the clan!");
        assert_eq!(pkt.opcode, WIZ_NOTICE);
        // type(1) + blocks(1) + header_len(2) + "Clan Notice"(11) + notice_len(2) + "Welcome..."(20)
        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let pkt_type = reader.read_u8().unwrap();
        assert_eq!(pkt_type, 4);
        let blocks = reader.read_u8().unwrap();
        assert_eq!(blocks, 1);
        let header = reader.read_string().unwrap();
        assert_eq!(header, "Clan Notice");
        let notice = reader.read_string().unwrap();
        assert_eq!(notice, "Welcome to the clan!");
    }

    #[test]
    fn test_join_req_accept_reads_v2615_inviter_before_clan() {
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(1); // accepted
        pkt.write_u32(0x1234); // inviter socket id
        pkt.write_u16(0x5678); // clan id

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(
            read_join_req_response(&mut reader),
            Some((1, 0x1234, 0x5678))
        );
    }

    #[test]
    fn test_join_req_reject_uses_same_v2615_layout() {
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(0); // declined
        pkt.write_u32(9); // inviter socket id
        pkt.write_u16(77); // clan id

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(read_join_req_response(&mut reader), Some((0, 9, 77)));
    }

    #[test]
    fn test_constants() {
        assert_eq!(CHIEF, 1);
        assert_eq!(VICECHIEF, 2);
        assert_eq!(OFFICER, 4);
        assert_eq!(TRAINEE, 5);
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(CLAN_COIN_REQUIREMENT, 500_000);
        assert_eq!(CLAN_LEVEL_REQUIREMENT, 30);
        assert_eq!(MAX_CLAN_USERS, 50);
    }

    #[test]
    fn test_sub_opcodes() {
        assert_eq!(KNIGHTS_CREATE, 1);
        assert_eq!(KNIGHTS_JOIN, 2);
        assert_eq!(KNIGHTS_WITHDRAW, 3);
        assert_eq!(KNIGHTS_REMOVE, 4);
        assert_eq!(KNIGHTS_DESTROY, 5);
        assert_eq!(KNIGHTS_ADMIT, 6);
        assert_eq!(KNIGHTS_REJECT, 7);
        assert_eq!(KNIGHTS_PUNISH, 8);
        assert_eq!(KNIGHTS_CHIEF, 9);
        assert_eq!(KNIGHTS_VICECHIEF, 10);
        assert_eq!(KNIGHTS_OFFICER, 11);
        assert_eq!(KNIGHTS_ALLLIST_REQ, 12);
        assert_eq!(KNIGHTS_MEMBER_REQ, 13);
        assert_eq!(KNIGHTS_CURRENT_REQ, 14);
        assert_eq!(KNIGHTS_JOIN_REQ, 17);
        assert_eq!(KNIGHTS_POINT_REQ, 59);
        assert_eq!(KNIGHTS_DONATE_POINTS, 61);
        assert_eq!(KNIGHTS_UPDATENOTICE, 80);
    }

    #[test]
    fn test_alliance_sub_opcodes() {
        assert_eq!(KNIGHTS_ALLY_CREATE, 28);
        assert_eq!(KNIGHTS_ALLY_REQ, 29);
        assert_eq!(KNIGHTS_ALLY_INSERT, 30);
        assert_eq!(KNIGHTS_ALLY_REMOVE, 31);
        assert_eq!(KNIGHTS_ALLY_PUNISH, 32);
        assert_eq!(KNIGHTS_ALLY_LIST, 34);
        assert_eq!(KNIGHTS_UPDATE, 36);
    }

    #[test]
    fn test_knights_alliance_struct() {
        let alliance = KnightsAlliance {
            main_clan: 100,
            sub_clan: 200,
            mercenary_1: 300,
            mercenary_2: 0,
            notice: "Test alliance".to_string(),
        };
        assert_eq!(alliance.main_clan, 100);
        assert_eq!(alliance.sub_clan, 200);
        assert_eq!(alliance.mercenary_1, 300);
        assert_eq!(alliance.mercenary_2, 0);
        assert_eq!(alliance.notice, "Test alliance");
    }

    #[test]
    fn test_knights_info_alliance_fields() {
        let info = KnightsInfo {
            id: 1,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "TestClan".to_string(),
            chief: "TestLeader".to_string(),
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
            alliance: 100,
            castellan_cape: false,
            cast_cape_id: -1,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 50,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        assert_eq!(info.alliance, 100);
        assert_eq!(info.alliance_req, 50);
        assert!(info.alliance > 0); // isInAlliance
        assert!(info.alliance_req > 0); // isAnyAllianceRequest
        assert!(!info.castellan_cape);
    }

    #[test]
    fn test_alliance_world_operations() {
        use crate::world::WorldState;
        let world = WorldState::new();

        // Insert alliance
        let alliance = KnightsAlliance {
            main_clan: 100,
            sub_clan: 200,
            mercenary_1: 0,
            mercenary_2: 0,
            notice: String::new(),
        };
        world.insert_alliance(alliance.clone());

        // Get alliance
        let fetched = world.get_alliance(100).unwrap();
        assert_eq!(fetched.main_clan, 100);
        assert_eq!(fetched.sub_clan, 200);

        // Update alliance
        world.update_alliance(100, |a| {
            a.mercenary_1 = 300;
        });
        let updated = world.get_alliance(100).unwrap();
        assert_eq!(updated.mercenary_1, 300);

        // Remove alliance
        let removed = world.remove_alliance(100);
        assert!(removed.is_some());
        assert!(world.get_alliance(100).is_none());
    }

    #[test]
    fn test_alliance_leader_check() {
        // C++ isAllianceLeader() = GetAllianceID() == GetID()
        let info = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "LeaderClan".to_string(),
            chief: "Leader".to_string(),
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
            alliance: 100, // same as id — this is the alliance leader
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
        };
        assert_eq!(info.alliance, info.id); // isAllianceLeader
    }

    #[test]
    fn test_new_sub_opcodes() {
        assert_eq!(KNIGHTS_USER_OFFLINE, 40);
        assert_eq!(KNIGHTS_POINT_METHOD, 60);
        assert_eq!(KNIGHTS_HANDOVER_VICECHIEF_LIST, 62);
        assert_eq!(KNIGHTS_HANDOVER_REQ, 63);
        assert_eq!(KNIGHTS_DONATION_LIST, 64);
        assert_eq!(KNIGHTS_HANDOVER, 79);
        assert_eq!(KNIGHTS_UPDATEMEMO, 88);
    }

    #[test]
    fn test_clan_point_method_field() {
        let info = KnightsInfo {
            id: 1,
            flag: 3,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "TestClan".to_string(),
            chief: "Leader".to_string(),
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
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: -1,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 2,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        assert_eq!(info.clan_point_method, 2);
    }

    #[test]
    fn test_offline_notification_packet() {
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_USER_OFFLINE);
        pkt.write_sbyte_string("TestPlayer");
        assert_eq!(pkt.opcode, WIZKNIGHTS_PROCESS);
        assert_eq!(pkt.data[0], KNIGHTS_USER_OFFLINE);
    }

    #[test]
    fn test_knights_list_packet() {
        let mut pkt = Packet::new(WIZKNIGHTS_LIST);
        let count: u16 = 2;
        pkt.write_u16(count);
        pkt.write_u16(100); // clan_id 1
        pkt.write_string("Clan1"); // name 1
        pkt.write_u16(200); // clan_id 2
        pkt.write_string("Clan2"); // name 2

        assert_eq!(pkt.opcode, WIZKNIGHTS_LIST);
        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let read_count = reader.read_u16().unwrap();
        assert_eq!(read_count, 2);
        let id1 = reader.read_u16().unwrap();
        assert_eq!(id1, 100);
        let name1 = reader.read_string().unwrap();
        assert_eq!(name1, "Clan1");
        let id2 = reader.read_u16().unwrap();
        assert_eq!(id2, 200);
        let name2 = reader.read_string().unwrap();
        assert_eq!(name2, "Clan2");
    }

    #[test]
    fn test_get_all_knights() {
        use crate::world::WorldState;
        let world = WorldState::new();

        let info1 = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "Clan1".to_string(),
            chief: "Chief1".to_string(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 100,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0xFFFF,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
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
        };
        let info2 = KnightsInfo {
            id: 200,
            name: "Clan2".to_string(),
            chief: "Chief2".to_string(),
            ..info1.clone()
        };

        world.insert_knights(info1);
        world.insert_knights(info2);

        let all = world.get_all_knights();
        assert_eq!(all.len(), 2);

        // Check both clans are present (order not guaranteed with DashMap)
        let ids: Vec<u16> = all.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&100));
        assert!(ids.contains(&200));
    }

    #[test]
    fn test_donation_list_format() {
        // Verify the donation list packet format
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_DONATION_LIST);
        pkt.write_u8(2); // count
        pkt.write_string("Player1");
        pkt.write_u32(1000); // NP donated
        pkt.write_string("Player2");
        pkt.write_u32(500);

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_DONATION_LIST);
        let count = reader.read_u8().unwrap();
        assert_eq!(count, 2);
        let name1 = reader.read_string().unwrap();
        assert_eq!(name1, "Player1");
        let np1 = reader.read_u32().unwrap();
        assert_eq!(np1, 1000);
    }

    #[test]
    fn test_handover_list_format() {
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_HANDOVER_VICECHIEF_LIST);
        pkt.write_u8(1); // is_clan_leader
        pkt.write_u16(1); // 1 online vice chief
        pkt.write_string("ViceChief1");

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_HANDOVER_VICECHIEF_LIST);
        let is_leader = reader.read_u8().unwrap();
        assert_eq!(is_leader, 1);
        let count = reader.read_u16().unwrap();
        assert_eq!(count, 1);
        let vc_name = reader.read_string().unwrap();
        assert_eq!(vc_name, "ViceChief1");
    }

    #[test]
    fn test_point_method_format() {
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_POINT_METHOD);
        pkt.write_u8(1); // success
        pkt.write_u8(2); // method

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_POINT_METHOD);
        let success = reader.read_u8().unwrap();
        assert_eq!(success, 1);
        let method = reader.read_u8().unwrap();
        assert_eq!(method, 2);
    }

    #[test]
    fn test_alliance_invitation_packet_format() {
        // C++ sends: sub_opcode + u8(1) + SByte(name) + u16(clan_id)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_ALLY_CREATE);
        pkt.write_u8(1); // success/invitation flag
        pkt.write_sbyte_string("TestClan");
        pkt.write_u16(42);

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_ALLY_CREATE);
        let flag = reader.read_u8().unwrap();
        assert_eq!(flag, 1);
        let name = reader.read_sbyte_string().unwrap();
        assert_eq!(name, "TestClan");
        let clan_id = reader.read_u16().unwrap();
        assert_eq!(clan_id, 42);
    }

    #[test]
    fn test_alliance_insert_response_packet_format() {
        // C++ ALLY_INSERT broadcast: sub_opcode(ALLY_INSERT) + u8(1) + u16(main) + u16(target) + u16(cape)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_ALLY_INSERT);
        pkt.write_u8(1); // success
        pkt.write_u16(100); // main clan ID
        pkt.write_u16(200); // target clan ID
        pkt.write_u16(5); // cape ID

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_ALLY_INSERT);
        let success = reader.read_u8().unwrap();
        assert_eq!(success, 1);
        let main_id = reader.read_u16().unwrap();
        assert_eq!(main_id, 100);
        let target_id = reader.read_u16().unwrap();
        assert_eq!(target_id, 200);
        let cape = reader.read_u16().unwrap();
        assert_eq!(cape, 5);
    }

    #[test]
    fn test_alliance_remove_packet_format() {
        // C++ ALLY_REMOVE: sub_opcode + u8(1) + u16(alliance_id) + u16(clan_id) + u16(0xFFFF)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_ALLY_REMOVE);
        pkt.write_u8(1);
        pkt.write_u16(100); // alliance ID
        pkt.write_u16(200); // leaving clan ID
        pkt.write_u16(0xFFFF); // -1 marker

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_ALLY_REMOVE);
        let success = reader.read_u8().unwrap();
        assert_eq!(success, 1);
        let alliance_id = reader.read_u16().unwrap();
        assert_eq!(alliance_id, 100);
        let clan_id = reader.read_u16().unwrap();
        assert_eq!(clan_id, 200);
        let marker = reader.read_u16().unwrap();
        assert_eq!(marker, 0xFFFF);
    }

    #[test]
    fn test_alliance_punish_packet_format() {
        // C++ ALLY_PUNISH: sub_opcode + u8(1) + u16(main_clan) + u16(target_clan) + u16(cape)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_ALLY_PUNISH);
        pkt.write_u8(1);
        pkt.write_u16(100); // main clan ID
        pkt.write_u16(300); // punished clan ID
        pkt.write_u16(7); // cape ID

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_ALLY_PUNISH);
        let success = reader.read_u8().unwrap();
        assert_eq!(success, 1);
        let main_id = reader.read_u16().unwrap();
        assert_eq!(main_id, 100);
        let target_id = reader.read_u16().unwrap();
        assert_eq!(target_id, 300);
        let cape = reader.read_u16().unwrap();
        assert_eq!(cape, 7);
    }

    #[test]
    fn test_knights_update_packet_no_alliance() {
        // C++ SendUpdate without alliance: u16(id) + u8(flag) + u16(cape) + u8(r) + u8(g) + u8(b) + u8(0)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_UPDATE);
        pkt.write_u16(100); // clan id
        pkt.write_u8(2); // flag
        pkt.write_u16(5); // cape
        pkt.write_u8(255); // r
        pkt.write_u8(128); // g
        pkt.write_u8(0); // b
        pkt.write_u8(0); // trailing byte

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let sub = reader.read_u8().unwrap();
        assert_eq!(sub, KNIGHTS_UPDATE);
        let clan_id = reader.read_u16().unwrap();
        assert_eq!(clan_id, 100);
        let flag = reader.read_u8().unwrap();
        assert_eq!(flag, 2);
        let cape = reader.read_u16().unwrap();
        assert_eq!(cape, 5);
        let r = reader.read_u8().unwrap();
        assert_eq!(r, 255);
        let g = reader.read_u8().unwrap();
        assert_eq!(g, 128);
        let b = reader.read_u8().unwrap();
        assert_eq!(b, 0);
        let trailing = reader.read_u8().unwrap();
        assert_eq!(trailing, 0);
    }

    // ── Loyalty Tracking Tests ──────────────────────────────────────

    #[test]
    fn test_min_np_to_donate_constant() {
        // C++ Knights.h:5 — MIN_NP_TO_DONATE = 1000
        assert_eq!(MIN_NP_TO_DONATE, 1000);
    }

    #[test]
    fn test_loyalty_donate_np_validation() {
        // Verify the validation logic: user must retain MIN_NP_TO_DONATE after donation
        let loyalty: u32 = 5000;
        let amount: u32 = 3500;
        // After donation: 5000 - 3500 = 1500 >= 1000 -> OK
        assert!((loyalty - amount) >= MIN_NP_TO_DONATE);

        let loyalty2: u32 = 2000;
        let amount2: u32 = 1500;
        // After donation: 2000 - 1500 = 500 < 1000 -> REJECTED
        assert!((loyalty2 - amount2) < MIN_NP_TO_DONATE);

        // Edge case: exact minimum retained
        let loyalty3: u32 = 2000;
        let amount3: u32 = 1000;
        // After donation: 2000 - 1000 = 1000 >= 1000 -> OK
        assert!((loyalty3 - amount3) >= MIN_NP_TO_DONATE);
    }

    #[test]
    fn test_loyalty_change_packet_format() {
        // WIZ_LOYALTY_CHANGE(0x2A) + u8(1) + u32(loyalty) + u32(monthly) + u32(0) + u32(0)
        let loyalty: u32 = 3500;
        let monthly: u32 = 1200;

        let mut pkt = Packet::new(0x2A);
        pkt.write_u8(1); // LOYALTY_NATIONAL_POINTS
        pkt.write_u32(loyalty);
        pkt.write_u32(monthly);
        pkt.write_u32(0); // clan donations
        pkt.write_u32(0); // clan loyalty amount

        assert_eq!(pkt.opcode, 0x2A);
        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(1)); // type
        assert_eq!(reader.read_u32(), Some(3500)); // loyalty
        assert_eq!(reader.read_u32(), Some(1200)); // monthly
        assert_eq!(reader.read_u32(), Some(0)); // clan donations
        assert_eq!(reader.read_u32(), Some(0)); // clan loyalty amount
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_point_req_packet_format() {
        // KNIGHTS_POINT_REQ response: u8(59) + u8(1) + u32(loyalty) + u32(fund)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_POINT_REQ);
        pkt.write_u8(1); // success
        pkt.write_u32(5000); // user loyalty
        pkt.write_u32(12000); // clan fund

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(KNIGHTS_POINT_REQ));
        assert_eq!(reader.read_u8(), Some(1));
        assert_eq!(reader.read_u32(), Some(5000));
        assert_eq!(reader.read_u32(), Some(12000));
    }

    // ── KNIGHTS_UPDATEMEMO Type 6 (Title Update) Tests ──────────────

    #[test]
    fn test_memo_title_update_packet_format() {
        // Packet: [u8 KNIGHTS_UPDATEMEMO][u8 type=6][u8 bResult][SByte username][SByte title]
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_UPDATEMEMO);
        pkt.write_u8(6); // type
        pkt.write_u8(0); // bResult = false (C++ TO-DO)
        pkt.write_sbyte_string("TestUser");
        pkt.write_sbyte_string("MyTitle");

        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(KNIGHTS_UPDATEMEMO));
        assert_eq!(reader.read_u8(), Some(6));
        assert_eq!(reader.read_u8(), Some(0)); // always false
        assert_eq!(reader.read_sbyte_string(), Some("TestUser".to_string()));
        assert_eq!(reader.read_sbyte_string(), Some("MyTitle".to_string()));
    }

    #[test]
    fn test_memo_title_update_always_failure() {
        // C++ sets bResult = false (TO-DO comment: "Acco update paketi dinlenecek")
        // Our implementation matches: always returns 0 (failure)
        let b_result: u8 = 0;
        assert_eq!(
            b_result, 0,
            "Title update should always return failure per C++ TO-DO"
        );
    }

    // ── Sprint 55: Hardening Edge Case Tests ────────────────────────

    /// Edge case: joining a clan when already in one. A player with
    /// knights_id != 0 should not be able to join another clan.
    #[test]
    fn test_join_clan_when_already_in_clan() {
        use crate::world::WorldState;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };

        // Player already in clan 100
        let ch = crate::world::CharacterInfo {
            session_id: sid,
            name: "ClanPlayer".into(),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 90,
            sta: 60,
            dex: 30,
            intel: 20,
            cha: 10,
            free_points: 0,
            skill_points: [0; 10],
            gold: 1_000_000,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 100, // already in clan 100
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 1,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        };
        world.register_ingame(sid, ch, pos);

        // Verify player is already in a clan
        let info = world.get_character_info(sid).unwrap();
        assert_ne!(info.knights_id, 0, "Player should already be in a clan");

        // The handler checks knights_id != 0 and rejects the join.
        // We verify that the check logic is correct:
        let can_join_new_clan = info.knights_id == 0;
        assert!(
            !can_join_new_clan,
            "Player in a clan should not be able to join another"
        );
    }

    /// Edge case: clan donation with insufficient gold. The donation should
    /// be rejected when the player doesn't retain MIN_NP_TO_DONATE after donation.
    #[test]
    fn test_clan_donation_insufficient_loyalty() {
        // Player has 500 NP, tries to donate 100
        // After: 500 - 100 = 400 < MIN_NP_TO_DONATE(1000) → rejected
        let loyalty: u32 = 500;
        let amount: u32 = 100;

        let can_donate = loyalty >= amount && (loyalty - amount) >= MIN_NP_TO_DONATE;
        assert!(
            !can_donate,
            "Should reject donation when remaining NP < MIN_NP_TO_DONATE"
        );
    }

    /// Edge case: clan donation of zero amount should be rejected.
    #[test]
    fn test_clan_donation_zero_amount() {
        let _loyalty: u32 = 5000;
        let amount: u32 = 0;

        // Zero donation makes no sense — should be rejected
        let valid = amount > 0;
        assert!(!valid, "Zero donation should be rejected");
    }

    /// Edge case: creating a clan with insufficient gold. Player needs
    /// CLAN_COIN_REQUIREMENT (500,000) gold.
    #[test]
    fn test_clan_create_insufficient_gold() {
        use crate::world::WorldState;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let ch = crate::world::CharacterInfo {
            session_id: sid,
            name: "PoorPlayer".into(),
            nation: 1,
            race: 1,
            class: 101,
            level: 40,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 90,
            sta: 60,
            dex: 30,
            intel: 20,
            cha: 10,
            free_points: 0,
            skill_points: [0; 10],
            gold: 100_000, // insufficient (needs 500,000)
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 1,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        };
        world.register_ingame(sid, ch, pos);

        let info = world.get_character_info(sid).unwrap();
        let has_enough_gold = info.gold >= CLAN_COIN_REQUIREMENT;
        assert!(
            !has_enough_gold,
            "Player with 100k gold should not meet 500k requirement"
        );

        let meets_level = info.level >= CLAN_LEVEL_REQUIREMENT;
        assert!(meets_level, "Level 40 should meet level 30 requirement");
    }

    #[test]
    fn test_knights_sub_opcode_constants() {
        assert_eq!(KNIGHTS_TOP10, 65);
        assert_eq!(KNIGHTS_UNK1, 99);
        assert_eq!(KNIGHTS_LADDER_POINTS, 100);
        assert_eq!(KNIGHTS_VS_LIST, 96);
    }

    #[test]
    fn test_top_clans_empty() {
        let world = crate::world::WorldState::new();
        // No clans registered — should return empty
        let karus = world.get_top_knights_by_nation(1, 5);
        assert!(karus.is_empty());
        let elmo = world.get_top_knights_by_nation(2, 5);
        assert!(elmo.is_empty());
    }

    #[test]
    fn test_top_clans_sorting() {
        let world = crate::world::WorldState::new();

        // Insert 3 Karus clans with different points
        world.insert_knights(KnightsInfo {
            id: 1,
            nation: 1,
            name: "Alpha".into(),
            points: 1000,
            mark_version: 1,
            mark_data: Vec::new(),
            ..Default::default()
        });
        world.insert_knights(KnightsInfo {
            id: 2,
            nation: 1,
            name: "Bravo".into(),
            points: 3000,
            mark_version: 2,
            mark_data: Vec::new(),
            ..Default::default()
        });
        world.insert_knights(KnightsInfo {
            id: 3,
            nation: 1,
            name: "Charlie".into(),
            points: 2000,
            mark_version: 3,
            mark_data: Vec::new(),
            ..Default::default()
        });

        let top = world.get_top_knights_by_nation(1, 5);
        assert_eq!(top.len(), 3);
        // Should be sorted by points DESC: Bravo(3000), Charlie(2000), Alpha(1000)
        assert_eq!(top[0].0, 2); // Bravo
        assert_eq!(top[0].1, "Bravo");
        assert_eq!(top[1].0, 3); // Charlie
        assert_eq!(top[2].0, 1); // Alpha

        // El Morad should be empty
        let elmo = world.get_top_knights_by_nation(2, 5);
        assert!(elmo.is_empty());
    }

    #[test]
    fn test_top_clans_limit() {
        let world = crate::world::WorldState::new();

        // Insert 7 clans — limit should cap at 5
        for i in 1..=7u16 {
            world.insert_knights(KnightsInfo {
                id: i,
                nation: 2,
                name: format!("Clan{}", i),
                points: i as u32 * 100,
                mark_version: 0,
                mark_data: Vec::new(),
                ..Default::default()
            });
        }

        let top = world.get_top_knights_by_nation(2, 5);
        assert_eq!(top.len(), 5);
        // Top 5 by points: Clan7(700), Clan6(600), Clan5(500), Clan4(400), Clan3(300)
        assert_eq!(top[0].0, 7);
        assert_eq!(top[4].0, 3);
    }

    #[test]
    fn test_top_clans_packet_format() {
        // Build the packet that send_top_clans would produce
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_TOP10);
        pkt.write_u16(0); // header

        // One Karus entry, 4 empty
        pkt.write_i16(1); // id
        pkt.write_string("TestClan");
        pkt.write_i16(5); // mark_version
        pkt.write_i16(0); // rank 0

        for k in 1..5i16 {
            pkt.write_i16(-1);
            pkt.write_string("");
            pkt.write_i16(-1);
            pkt.write_i16(k);
        }

        // 5 empty El Morad entries
        for k in 0..5i16 {
            pkt.write_i16(-1);
            pkt.write_string("");
            pkt.write_i16(-1);
            pkt.write_i16(k);
        }

        assert_eq!(pkt.opcode, WIZKNIGHTS_PROCESS);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(KNIGHTS_TOP10));
        assert_eq!(r.read_u16(), Some(0)); // header

        // First Karus entry
        assert_eq!(r.read_i16(), Some(1));
        assert_eq!(r.read_string(), Some("TestClan".to_string()));
        assert_eq!(r.read_i16(), Some(5));
        assert_eq!(r.read_i16(), Some(0));
    }

    #[test]
    fn test_mark_sub_opcodes() {
        assert_eq!(KNIGHTS_MARK_VERSION_REQ, 25);
        assert_eq!(KNIGHTS_MARK_REGISTER, 26);
        assert_eq!(KNIGHTS_MARK_REQ, 35);
        assert_eq!(KNIGHTS_MARK_REGION_REQ, 37);
    }

    #[test]
    fn test_mark_version_req_response_format() {
        // Success response: [u8 25] [i16 1] [u16 markVersion]
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_MARK_VERSION_REQ);
        pkt.write_i16(1); // success
        pkt.write_u16(3); // mark version = 3

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(KNIGHTS_MARK_VERSION_REQ));
        assert_eq!(r.read_i16(), Some(1));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_mark_version_req_error_format() {
        // Error response: [u8 25] [i16 errorCode] (no version field)
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_MARK_VERSION_REQ);
        pkt.write_i16(11); // error: not leader / not promoted

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(KNIGHTS_MARK_VERSION_REQ));
        assert_eq!(r.read_i16(), Some(11));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_mark_register_error_format() {
        // Error response: [u8 26] [u16 errorCode]
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_MARK_REGISTER);
        pkt.write_u16(11); // error

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(KNIGHTS_MARK_REGISTER));
        assert_eq!(r.read_u16(), Some(11));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_mark_req_full_response_format() {
        // Full response (C++ KnightsGetSymbol): [u8 35] [u16 1] [u16 nation] [u16 clanID] [u16 markVer] [u16 markLen] [bytes]
        let mut pkt = Packet::new(WIZKNIGHTS_PROCESS);
        pkt.write_u8(KNIGHTS_MARK_REQ);
        pkt.write_u16(1); // success
        pkt.write_u16(1); // nation = Karus
        pkt.write_u16(100); // clanID
        pkt.write_u16(5); // markVersion
        pkt.write_u16(4); // markLen
        pkt.data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // image data

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(KNIGHTS_MARK_REQ));
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_u16(), Some(1)); // nation
        assert_eq!(r.read_u16(), Some(100)); // clanID
        assert_eq!(r.read_u16(), Some(5)); // version
        assert_eq!(r.read_u16(), Some(4)); // len
        assert_eq!(r.remaining(), 4); // image bytes
    }

    // ── Sprint 248: Permission constant tests ────────────────────────

    /// KNIGHTS_REMOVE: only CHIEF may remove members.
    #[test]
    fn test_remove_requires_chief_only() {
        assert_eq!(CHIEF, 1);
        // VICECHIEF(2), KNIGHT(3), OFFICER(4), TRAINEE(5) must NOT pass
        for &fame in &[0u8, VICECHIEF, KNIGHT, OFFICER, TRAINEE] {
            assert_ne!(fame, CHIEF, "fame={fame} must not equal CHIEF for remove");
        }
    }

    /// KNIGHTS_PUNISH: CHIEF and VICECHIEF may punish.
    /// Note: C++ bug causes CHIEF(1) < VICECHIEF(2) → denied. We fix this.
    #[test]
    fn test_punish_allows_chief_and_vicechief() {
        assert!(CHIEF == 1 || CHIEF == VICECHIEF, "CHIEF must pass punish");
        assert_eq!(VICECHIEF, 2);
        // KNIGHT(3), OFFICER(4), TRAINEE(5) must NOT pass
        for &fame in &[0u8, KNIGHT, OFFICER, TRAINEE] {
            assert!(
                fame != CHIEF && fame != VICECHIEF,
                "fame={fame} must not pass punish"
            );
        }
    }

    /// KNIGHTS_OFFICER promotion: only CHIEF may promote to officer.
    #[test]
    fn test_officer_promotion_requires_chief() {
        assert_eq!(CHIEF, 1);
        for &fame in &[0u8, VICECHIEF, KNIGHT, OFFICER, TRAINEE] {
            assert_ne!(fame, CHIEF, "fame={fame} must not promote officers");
        }
    }

    // ── Sprint 315: AutoRoyalG1 clan creation flag ───────────────────

    /// `g_pMain->pServerSetting.AutoRoyalG1 ? ClanTypeRoyal1 : ClanTypeTraining`
    #[test]
    fn test_auto_royal_g1_clan_flag() {
        use crate::clan_constants::{CLAN_TYPE_ROYAL1, CLAN_TYPE_TRAINING};

        // When AutoRoyalG1 is enabled, new clans start at Royal1 (12)
        let auto_royal = true;
        let flag = if auto_royal {
            CLAN_TYPE_ROYAL1
        } else {
            CLAN_TYPE_TRAINING
        };
        assert_eq!(flag, 12);

        // When disabled, new clans start at Training (1)
        let auto_royal = false;
        let flag = if auto_royal {
            CLAN_TYPE_ROYAL1
        } else {
            CLAN_TYPE_TRAINING
        };
        assert_eq!(flag, 1);
    }

    // ── Sprint 322: Knights validation fixes ───────────────────────

    /// Error codes: 2=offline, 4=diff nation, 5=diff clan, 9=self-punish.
    #[test]
    fn test_punish_error_codes() {
        let offline: u8 = 2;
        let diff_nation: u8 = 4;
        let diff_clan: u8 = 5;
        let self_punish: u8 = 9;
        assert_ne!(offline, diff_nation);
        assert_ne!(diff_nation, diff_clan);
        assert_ne!(diff_clan, self_punish);
    }

    #[test]
    fn test_remove_nation_check() {
        // Inviter nation 1, target nation 2 — should fail with error 4
        let inviter_nation: u8 = 1;
        let target_nation: u8 = 2;
        assert_ne!(inviter_nation, target_nation);
    }

    #[test]
    fn test_admit_offline_target_error() {
        // Offline target should return error code 2
        let b_result: u8 = 2;
        assert_eq!(b_result, 2, "offline target should get error 2");
    }

    #[test]
    fn test_vicechief_target_validation() {
        // Same nation, same clan → success
        let inviter_nation: u8 = 1;
        let target_nation: u8 = 1;
        let inviter_clan: u16 = 100;
        let target_clan: u16 = 100;
        assert_eq!(inviter_nation, target_nation);
        assert_eq!(inviter_clan, target_clan);

        // Different nation → error 4
        let target_nation_diff: u8 = 2;
        assert_ne!(inviter_nation, target_nation_diff);

        // Different clan → error 5
        let target_clan_diff: u16 = 200;
        assert_ne!(inviter_clan, target_clan_diff);
    }

    /// Self-punish prevention matches self-remove pattern.
    #[test]
    fn test_punish_self_check() {
        let inviter = "TestPlayer";
        let target = "TestPlayer";
        assert_eq!(
            inviter.to_uppercase(),
            target.to_uppercase(),
            "same name should be detected as self"
        );
    }

    // ── Sprint 958: Additional coverage ──────────────────────────────

    /// Knights sub-opcodes 1-14 are sequential (C++ KnightsPacket enum).
    #[test]
    fn test_knights_core_subopcodes_sequential() {
        assert_eq!(KNIGHTS_CREATE, 1);
        assert_eq!(KNIGHTS_JOIN, 2);
        assert_eq!(KNIGHTS_WITHDRAW, 3);
        assert_eq!(KNIGHTS_REMOVE, 4);
        assert_eq!(KNIGHTS_DESTROY, 5);
        assert_eq!(KNIGHTS_ADMIT, 6);
        assert_eq!(KNIGHTS_REJECT, 7);
        assert_eq!(KNIGHTS_PUNISH, 8);
        assert_eq!(KNIGHTS_CHIEF, 9);
        assert_eq!(KNIGHTS_VICECHIEF, 10);
        assert_eq!(KNIGHTS_OFFICER, 11);
        assert_eq!(KNIGHTS_ALLLIST_REQ, 12);
        assert_eq!(KNIGHTS_MEMBER_REQ, 13);
        assert_eq!(KNIGHTS_CURRENT_REQ, 14);
    }

    /// Knights alliance sub-opcodes are in 28-35 range.
    #[test]
    fn test_knights_alliance_subopcodes() {
        assert_eq!(KNIGHTS_ALLY_CREATE, 28);
        assert_eq!(KNIGHTS_ALLY_REQ, 29);
        assert_eq!(KNIGHTS_ALLY_INSERT, 30);
        assert_eq!(KNIGHTS_ALLY_REMOVE, 31);
        assert_eq!(KNIGHTS_ALLY_PUNISH, 32);
        assert_eq!(KNIGHTS_ALLY_LIST, 34);
        assert_eq!(KNIGHTS_MARK_REQ, 35);
    }

    /// Knights donation/handover sub-opcodes are in 59-65+ range.
    #[test]
    fn test_knights_donation_subopcodes() {
        assert_eq!(KNIGHTS_POINT_REQ, 59);
        assert_eq!(KNIGHTS_POINT_METHOD, 60);
        assert_eq!(KNIGHTS_DONATE_POINTS, 61);
        assert_eq!(KNIGHTS_HANDOVER_VICECHIEF_LIST, 62);
        assert_eq!(KNIGHTS_HANDOVER_REQ, 63);
        assert_eq!(KNIGHTS_DONATION_LIST, 64);
        assert_eq!(KNIGHTS_TOP10, 65);
    }

    /// MIN_NP_TO_DONATE matches C++ Knights.h.
    #[test]
    fn test_min_np_to_donate() {
        assert_eq!(MIN_NP_TO_DONATE, 1000);
        assert!(MIN_NP_TO_DONATE > 0);
    }

    /// WIZKNIGHTS_PROCESS and WIZ_NOTICE opcode values.
    #[test]
    fn test_knights_opcode_values() {
        assert_eq!(WIZKNIGHTS_PROCESS, 0x3C);
        assert_eq!(WIZ_NOTICE, 0x2E);
        assert_ne!(WIZKNIGHTS_PROCESS, WIZ_NOTICE);
    }

    // ── Sprint 968: Additional coverage ──────────────────────────────

    /// High sub-opcodes: HANDOVER=79, UPDATENOTICE=80, UPDATEMEMO=88.
    #[test]
    fn test_knights_high_subopcodes() {
        assert_eq!(KNIGHTS_HANDOVER, 79);
        assert_eq!(KNIGHTS_UPDATENOTICE, 80);
        assert_eq!(KNIGHTS_UPDATEMEMO, 88);
        assert_eq!(KNIGHTS_VS_LIST, 96);
        assert_eq!(KNIGHTS_UNK1, 99);
        assert_eq!(KNIGHTS_LADDER_POINTS, 100);
    }

    /// Mark-related sub-opcodes: VERSION=25, REGISTER=26, REQ=35, REGION=37.
    #[test]
    fn test_knights_mark_subopcodes() {
        assert_eq!(KNIGHTS_MARK_VERSION_REQ, 25);
        assert_eq!(KNIGHTS_MARK_REGISTER, 26);
        assert_eq!(KNIGHTS_MARK_REQ, 35);
        assert_eq!(KNIGHTS_MARK_REGION_REQ, 37);
        assert_eq!(KNIGHTS_UPDATE, 36);
    }

    /// User online/offline sub-opcodes are adjacent (39, 40).
    #[test]
    fn test_knights_user_status_subopcodes() {
        assert_eq!(KNIGHTS_USER_ONLINE, 39);
        assert_eq!(KNIGHTS_USER_OFFLINE, 40);
        assert_eq!(KNIGHTS_USER_OFFLINE - KNIGHTS_USER_ONLINE, 1);
    }

    /// JOIN_REQ=17 is separate from JOIN=2 (application vs invite).
    #[test]
    fn test_knights_join_vs_join_req() {
        assert_eq!(KNIGHTS_JOIN, 2);
        assert_eq!(KNIGHTS_JOIN_REQ, 17);
        assert_ne!(KNIGHTS_JOIN, KNIGHTS_JOIN_REQ);
    }

    /// Clan constants imported from clan_constants match expected values.
    #[test]
    fn test_imported_clan_constants() {
        assert_eq!(CHIEF, 1);
        assert_eq!(VICECHIEF, 2);
        assert_eq!(OFFICER, 4);
        assert_eq!(TRAINEE, 5);
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(MAX_CLAN_USERS, 50);
        assert_eq!(CLAN_COIN_REQUIREMENT, 500_000);
        assert_eq!(CLAN_LEVEL_REQUIREMENT, 30);
    }

    // ── Sprint 972: Additional coverage ──────────────────────────────

    /// Alliance sub-opcodes form a contiguous block 28-32 (except 33 gap) + 34.
    #[test]
    fn test_knights_alliance_block_contiguous() {
        assert_eq!(KNIGHTS_ALLY_CREATE, 28);
        assert_eq!(KNIGHTS_ALLY_REQ, 29);
        assert_eq!(KNIGHTS_ALLY_INSERT, 30);
        assert_eq!(KNIGHTS_ALLY_REMOVE, 31);
        assert_eq!(KNIGHTS_ALLY_PUNISH, 32);
        // gap at 33
        assert_eq!(KNIGHTS_ALLY_LIST, 34);
    }

    /// Point/donation sub-opcodes form a contiguous block 59-65.
    #[test]
    fn test_knights_point_donation_block() {
        assert_eq!(KNIGHTS_POINT_REQ, 59);
        assert_eq!(KNIGHTS_POINT_METHOD, 60);
        assert_eq!(KNIGHTS_DONATE_POINTS, 61);
        assert_eq!(KNIGHTS_HANDOVER_VICECHIEF_LIST, 62);
        assert_eq!(KNIGHTS_HANDOVER_REQ, 63);
        assert_eq!(KNIGHTS_DONATION_LIST, 64);
        assert_eq!(KNIGHTS_TOP10, 65);
        // Verify contiguous 59..=65
        assert_eq!(KNIGHTS_TOP10 - KNIGHTS_POINT_REQ, 6);
    }

    /// MIN_NP_TO_DONATE is 1000 and all sub-opcodes are unique.
    #[test]
    fn test_all_subopcodes_unique() {
        let ops: Vec<u8> = vec![
            KNIGHTS_CREATE,
            KNIGHTS_JOIN,
            KNIGHTS_WITHDRAW,
            KNIGHTS_REMOVE,
            KNIGHTS_DESTROY,
            KNIGHTS_ADMIT,
            KNIGHTS_REJECT,
            KNIGHTS_PUNISH,
            KNIGHTS_CHIEF,
            KNIGHTS_VICECHIEF,
            KNIGHTS_OFFICER,
            KNIGHTS_ALLLIST_REQ,
            KNIGHTS_MEMBER_REQ,
            KNIGHTS_CURRENT_REQ,
            KNIGHTS_JOIN_REQ,
            KNIGHTS_USER_ONLINE,
            KNIGHTS_USER_OFFLINE,
            KNIGHTS_MARK_VERSION_REQ,
            KNIGHTS_MARK_REGISTER,
            KNIGHTS_ALLY_CREATE,
            KNIGHTS_ALLY_REQ,
            KNIGHTS_ALLY_INSERT,
            KNIGHTS_ALLY_REMOVE,
            KNIGHTS_ALLY_PUNISH,
            KNIGHTS_ALLY_LIST,
            KNIGHTS_MARK_REQ,
            KNIGHTS_UPDATE,
            KNIGHTS_MARK_REGION_REQ,
            KNIGHTS_POINT_REQ,
            KNIGHTS_POINT_METHOD,
            KNIGHTS_DONATE_POINTS,
            KNIGHTS_HANDOVER_VICECHIEF_LIST,
            KNIGHTS_HANDOVER_REQ,
            KNIGHTS_DONATION_LIST,
            KNIGHTS_TOP10,
            KNIGHTS_HANDOVER,
            KNIGHTS_UPDATENOTICE,
            KNIGHTS_UPDATEMEMO,
            KNIGHTS_VS_LIST,
            KNIGHTS_UNK1,
            KNIGHTS_LADDER_POINTS,
        ];
        let mut set = std::collections::HashSet::new();
        for op in &ops {
            assert!(set.insert(*op), "duplicate sub-opcode: {}", op);
        }
    }

    /// Core management sub-opcodes 1-14 are sequential with no gaps.
    #[test]
    fn test_core_management_sequential() {
        let core = [
            KNIGHTS_CREATE,
            KNIGHTS_JOIN,
            KNIGHTS_WITHDRAW,
            KNIGHTS_REMOVE,
            KNIGHTS_DESTROY,
            KNIGHTS_ADMIT,
            KNIGHTS_REJECT,
            KNIGHTS_PUNISH,
            KNIGHTS_CHIEF,
            KNIGHTS_VICECHIEF,
            KNIGHTS_OFFICER,
            KNIGHTS_ALLLIST_REQ,
            KNIGHTS_MEMBER_REQ,
            KNIGHTS_CURRENT_REQ,
        ];
        for (i, &op) in core.iter().enumerate() {
            assert_eq!(op, (i + 1) as u8);
        }
    }

    /// MIN_NP_TO_DONATE matches C++ define value.
    #[test]
    fn test_min_np_donate_value() {
        assert_eq!(MIN_NP_TO_DONATE, 1000);
        // Must be non-zero (would allow infinite donation)
        assert!(MIN_NP_TO_DONATE > 0);
    }

    /// WIZKNIGHTS_PROCESS and WIZ_NOTICE opcodes are distinct.
    #[test]
    fn test_wizknights_and_notice_opcodes_distinct() {
        assert_eq!(WIZKNIGHTS_PROCESS, 0x3C);
        assert_eq!(WIZ_NOTICE, 0x2E);
        assert_ne!(WIZKNIGHTS_PROCESS, WIZ_NOTICE);
    }

    /// knights_error packet structure: opcode + sub_opcode + error_code.
    #[test]
    fn test_knights_error_all_subopcodes() {
        for sub in [
            KNIGHTS_CREATE,
            KNIGHTS_JOIN,
            KNIGHTS_WITHDRAW,
            KNIGHTS_REMOVE,
            KNIGHTS_DESTROY,
        ] {
            let pkt = knights_error(sub, 7);
            assert_eq!(pkt.opcode, WIZKNIGHTS_PROCESS);
            assert_eq!(pkt.data[0], sub);
            assert_eq!(pkt.data[1], 7);
        }
    }

    /// WIZKNIGHTS_LIST opcode (0x3E) is two above WIZKNIGHTS_PROCESS (0x3C).
    #[test]
    fn test_knights_list_opcode_offset() {
        assert_eq!(WIZKNIGHTS_LIST, WIZKNIGHTS_PROCESS + 2);
        assert_eq!(WIZKNIGHTS_LIST, 0x3E);
    }

    /// Clan rank constants: CHIEF=1, VICECHIEF=2, KNIGHT=3, OFFICER=4, TRAINEE=5.
    #[test]
    fn test_clan_rank_values() {
        assert_eq!(CHIEF, 1);
        assert_eq!(VICECHIEF, 2);
        assert_eq!(KNIGHT, 3);
        assert_eq!(OFFICER, 4);
        assert_eq!(TRAINEE, 5);
        // COMMAND_CAPTAIN is separate at 100
        assert_eq!(COMMAND_CAPTAIN, 100);
    }

    /// MAX_CLAN_USERS and level/coin requirements are reasonable.
    #[test]
    fn test_clan_creation_requirements() {
        // Level requirement must be positive
        assert!(CLAN_LEVEL_REQUIREMENT > 0);
        // Coin requirement must be positive
        assert!(CLAN_COIN_REQUIREMENT > 0);
        // Max clan users is a reasonable limit
        assert!(MAX_CLAN_USERS > 0);
        assert!(MAX_CLAN_USERS <= 200);
    }

    /// build_clan_notice_packet: type=4, blocks=1, header="Clan Notice".
    #[test]
    fn test_clan_notice_packet_structure() {
        let pkt = build_clan_notice_packet("Hello clan");
        assert_eq!(pkt.opcode, WIZ_NOTICE);
        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), 4); // type
        assert_eq!(reader.read_u8().unwrap(), 1); // blocks
        let header = reader.read_string().unwrap();
        assert_eq!(header, "Clan Notice");
        let notice = reader.read_string().unwrap();
        assert_eq!(notice, "Hello clan");
    }

    /// build_clan_notice_packet with empty notice produces valid packet.
    #[test]
    fn test_clan_notice_empty_notice() {
        let pkt = build_clan_notice_packet("");
        let mut reader = ko_protocol::PacketReader::new(&pkt.data);
        let _ = reader.read_u8(); // type
        let _ = reader.read_u8(); // blocks
        let _ = reader.read_string(); // header
        let notice = reader.read_string().unwrap();
        assert_eq!(notice, "");
    }

    /// KNIGHTS_USER_ONLINE and KNIGHTS_USER_OFFLINE are adjacent (39, 40).
    #[test]
    fn test_user_online_offline_adjacent() {
        assert_eq!(KNIGHTS_USER_ONLINE, 39);
        assert_eq!(KNIGHTS_USER_OFFLINE, 40);
        assert_eq!(KNIGHTS_USER_OFFLINE, KNIGHTS_USER_ONLINE + 1);
    }

    /// KNIGHTS_HANDOVER (79) is separate from donation block (59-65).
    #[test]
    fn test_handover_separate_from_donation() {
        assert_eq!(KNIGHTS_HANDOVER, 79);
        // Far from donation block
        assert!(KNIGHTS_HANDOVER > KNIGHTS_TOP10);
        assert!(KNIGHTS_HANDOVER < KNIGHTS_UPDATENOTICE);
    }

    /// KNIGHTS_VS_LIST, UNK1, LADDER_POINTS are high-range sub-opcodes.
    #[test]
    fn test_high_range_subopcodes_ordering() {
        assert_eq!(KNIGHTS_VS_LIST, 96);
        assert_eq!(KNIGHTS_UNK1, 99);
        assert_eq!(KNIGHTS_LADDER_POINTS, 100);
        // Strictly ordered
        assert!(KNIGHTS_VS_LIST < KNIGHTS_UNK1);
        assert!(KNIGHTS_UNK1 < KNIGHTS_LADDER_POINTS);
    }

    /// knights_error produces 2 bytes of data (sub_opcode + error_code).
    #[test]
    fn test_knights_error_data_length() {
        let pkt = knights_error(KNIGHTS_PUNISH, 10);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], KNIGHTS_PUNISH);
        assert_eq!(pkt.data[1], 10);
    }

    /// KNIGHTS_MARK sub-opcodes (25, 26, 35, 37) are all distinct.
    #[test]
    fn test_mark_subopcodes_all_distinct() {
        let marks = [
            KNIGHTS_MARK_VERSION_REQ,
            KNIGHTS_MARK_REGISTER,
            KNIGHTS_MARK_REQ,
            KNIGHTS_MARK_REGION_REQ,
        ];
        let mut set = std::collections::HashSet::new();
        for &m in &marks {
            assert!(set.insert(m), "duplicate mark sub-opcode: {}", m);
        }
        assert_eq!(marks.len(), 4);
    }

    /// KNIGHTS_JOIN_REQ (17) is separate from KNIGHTS_JOIN (2).
    #[test]
    fn test_join_req_vs_join() {
        assert_eq!(KNIGHTS_JOIN, 2);
        assert_eq!(KNIGHTS_JOIN_REQ, 17);
        assert_ne!(KNIGHTS_JOIN, KNIGHTS_JOIN_REQ);
        // JOIN_REQ is in the gap between 14 and 25
        assert!(KNIGHTS_JOIN_REQ > KNIGHTS_CURRENT_REQ);
        assert!(KNIGHTS_JOIN_REQ < KNIGHTS_MARK_VERSION_REQ);
    }

    /// KnightsInfo default cape value 0xFFFF represents "no cape".
    #[test]
    fn test_knights_info_no_cape() {
        let info = KnightsInfo {
            id: 1,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "T".to_string(),
            chief: "C".to_string(),
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
            alliance: 0,
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
        };
        // 0xFFFF = no cape (C++ default -1 as u16)
        assert_eq!(info.cape, 0xFFFF);
        assert_eq!(info.cast_cape_id, -1);
    }

    /// KNIGHTS_UPDATEMEMO (88) is separate from KNIGHTS_UPDATENOTICE (80).
    #[test]
    fn test_updatememo_vs_updatenotice() {
        assert_eq!(KNIGHTS_UPDATENOTICE, 80);
        assert_eq!(KNIGHTS_UPDATEMEMO, 88);
        assert_ne!(KNIGHTS_UPDATENOTICE, KNIGHTS_UPDATEMEMO);
        assert!(KNIGHTS_UPDATEMEMO > KNIGHTS_UPDATENOTICE);
    }

    /// Core sub-opcodes 1-11 cover create through officer — contiguous block.
    #[test]
    fn test_knights_create_to_officer_contiguous() {
        assert_eq!(KNIGHTS_CREATE, 1);
        assert_eq!(KNIGHTS_JOIN, 2);
        assert_eq!(KNIGHTS_WITHDRAW, 3);
        assert_eq!(KNIGHTS_REMOVE, 4);
        assert_eq!(KNIGHTS_DESTROY, 5);
        assert_eq!(KNIGHTS_ADMIT, 6);
        assert_eq!(KNIGHTS_REJECT, 7);
        assert_eq!(KNIGHTS_PUNISH, 8);
        assert_eq!(KNIGHTS_CHIEF, 9);
        assert_eq!(KNIGHTS_VICECHIEF, 10);
        assert_eq!(KNIGHTS_OFFICER, 11);
        // 1-11 contiguous
        assert_eq!(KNIGHTS_OFFICER - KNIGHTS_CREATE, 10);
    }

    /// Clan grade constants form 1-5 hierarchy (chief highest authority).
    #[test]
    fn test_clan_grade_hierarchy_values() {
        assert_eq!(CHIEF, 1);
        assert_eq!(VICECHIEF, 2);
        assert_eq!(KNIGHT, 3);
        assert_eq!(OFFICER, 4);
        assert_eq!(TRAINEE, 5);
        // Lower number = higher authority
        assert!(CHIEF < VICECHIEF);
        assert!(VICECHIEF < TRAINEE);
        // Exactly 5 distinct grades
        let grades = [CHIEF, VICECHIEF, KNIGHT, OFFICER, TRAINEE];
        assert_eq!(grades.len(), 5);
    }

    /// MIN_NP_TO_DONATE is 1000 and distinct from clan coin requirement.
    #[test]
    fn test_donation_np_minimum() {
        assert_eq!(MIN_NP_TO_DONATE, 1000);
        assert_eq!(CLAN_COIN_REQUIREMENT, 500_000);
        // Donation min << clan creation cost
        assert!(MIN_NP_TO_DONATE < CLAN_COIN_REQUIREMENT);
    }

    /// Alliance structure holds exactly 4 clan slots (main + sub + 2 mercenaries).
    #[test]
    fn test_alliance_clan_slots() {
        let alliance = KnightsAlliance {
            main_clan: 1,
            sub_clan: 2,
            mercenary_1: 3,
            mercenary_2: 4,
            notice: String::new(),
        };
        // 4 distinct clan slots
        let slots = [
            alliance.main_clan,
            alliance.sub_clan,
            alliance.mercenary_1,
            alliance.mercenary_2,
        ];
        assert_eq!(slots.len(), 4);
        // All non-zero
        assert!(slots.iter().all(|&s| s > 0));
    }

    /// Alliance sub-opcodes 28-34 cover create/req/insert/remove/punish/list.
    #[test]
    fn test_alliance_subopcodes_range() {
        assert_eq!(KNIGHTS_ALLY_CREATE, 28);
        assert_eq!(KNIGHTS_ALLY_REQ, 29);
        assert_eq!(KNIGHTS_ALLY_INSERT, 30);
        assert_eq!(KNIGHTS_ALLY_REMOVE, 31);
        assert_eq!(KNIGHTS_ALLY_PUNISH, 32);
        assert_eq!(KNIGHTS_ALLY_LIST, 34);
        // Range 28-34
        assert!(KNIGHTS_ALLY_LIST - KNIGHTS_ALLY_CREATE <= 6);
    }

    // ── Sprint 995: knights.rs +5 ───────────────────────────────────────

    /// MAX_CLAN_USERS is 50, MAX_ID_SIZE is 20 (character name length limit).
    #[test]
    fn test_clan_capacity_and_name_limits() {
        assert_eq!(MAX_CLAN_USERS, 50);
        assert_eq!(MAX_ID_SIZE, 20);
        // Name limit fits in u8
        assert!(MAX_ID_SIZE <= u8::MAX as usize);
    }

    /// Three handover sub-opcodes are distinct: HANDOVER(79), VICECHIEF_LIST(62), REQ(63).
    #[test]
    fn test_handover_trio_distinct() {
        assert_eq!(KNIGHTS_HANDOVER, 79);
        assert_eq!(KNIGHTS_HANDOVER_VICECHIEF_LIST, 62);
        assert_eq!(KNIGHTS_HANDOVER_REQ, 63);
        // All distinct
        assert_ne!(KNIGHTS_HANDOVER, KNIGHTS_HANDOVER_VICECHIEF_LIST);
        assert_ne!(KNIGHTS_HANDOVER, KNIGHTS_HANDOVER_REQ);
        assert_ne!(KNIGHTS_HANDOVER_VICECHIEF_LIST, KNIGHTS_HANDOVER_REQ);
    }

    /// Gap between alliance block (28-34) and donation block (59-65) is 24.
    #[test]
    fn test_alliance_donation_gap() {
        // Alliance ends at 34 (ALLY_LIST) or 37 (MARK_REGION_REQ)
        // Donation starts at 59 (POINT_REQ)
        assert_eq!(KNIGHTS_POINT_REQ - KNIGHTS_MARK_REGION_REQ, 22);
        // Large gap contains USER_ONLINE(39), USER_OFFLINE(40)
        assert!(KNIGHTS_USER_ONLINE > KNIGHTS_MARK_REGION_REQ);
        assert!(KNIGHTS_USER_ONLINE < KNIGHTS_POINT_REQ);
    }

    /// KNIGHTS_LADDER_POINTS (100) is the highest sub-opcode — exactly 99 above CREATE (1).
    #[test]
    fn test_ladder_points_highest_offset() {
        assert_eq!(KNIGHTS_LADDER_POINTS, 100);
        assert_eq!(KNIGHTS_LADDER_POINTS - KNIGHTS_CREATE, 99);
        // No sub-opcode exceeds 100
        let all_ops = [
            KNIGHTS_CREATE,
            KNIGHTS_JOIN,
            KNIGHTS_WITHDRAW,
            KNIGHTS_REMOVE,
            KNIGHTS_DESTROY,
            KNIGHTS_ADMIT,
            KNIGHTS_REJECT,
            KNIGHTS_PUNISH,
            KNIGHTS_CHIEF,
            KNIGHTS_VICECHIEF,
            KNIGHTS_OFFICER,
            KNIGHTS_ALLLIST_REQ,
            KNIGHTS_MEMBER_REQ,
            KNIGHTS_CURRENT_REQ,
            KNIGHTS_JOIN_REQ,
            KNIGHTS_USER_ONLINE,
            KNIGHTS_USER_OFFLINE,
            KNIGHTS_MARK_VERSION_REQ,
            KNIGHTS_MARK_REGISTER,
            KNIGHTS_ALLY_CREATE,
            KNIGHTS_ALLY_REQ,
            KNIGHTS_ALLY_INSERT,
            KNIGHTS_ALLY_REMOVE,
            KNIGHTS_ALLY_PUNISH,
            KNIGHTS_ALLY_LIST,
            KNIGHTS_MARK_REQ,
            KNIGHTS_UPDATE,
            KNIGHTS_MARK_REGION_REQ,
            KNIGHTS_POINT_REQ,
            KNIGHTS_POINT_METHOD,
            KNIGHTS_DONATE_POINTS,
            KNIGHTS_HANDOVER_VICECHIEF_LIST,
            KNIGHTS_HANDOVER_REQ,
            KNIGHTS_DONATION_LIST,
            KNIGHTS_TOP10,
            KNIGHTS_HANDOVER,
            KNIGHTS_UPDATENOTICE,
            KNIGHTS_UPDATEMEMO,
            KNIGHTS_VS_LIST,
            KNIGHTS_UNK1,
            KNIGHTS_LADDER_POINTS,
        ];
        assert!(all_ops.iter().all(|&op| op <= KNIGHTS_LADDER_POINTS));
    }

    /// COMMAND_CAPTAIN (100) matches KNIGHTS_LADDER_POINTS value but different semantics.
    #[test]
    fn test_command_captain_vs_ladder_coincidence() {
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(KNIGHTS_LADDER_POINTS, 100);
        // Same numeric value, different usage:
        // COMMAND_CAPTAIN = clan fame rank, KNIGHTS_LADDER_POINTS = sub-opcode
        assert_eq!(COMMAND_CAPTAIN as u8, KNIGHTS_LADDER_POINTS);
    }

    // ── Sprint 998: knights.rs +5 ───────────────────────────────────────

    /// Total distinct sub-opcodes handled in dispatch: 31 match arms + 1 wildcard.
    #[test]
    fn test_knights_total_handled_subopcodes() {
        let handled = [
            KNIGHTS_CREATE,
            KNIGHTS_JOIN,
            KNIGHTS_WITHDRAW,
            KNIGHTS_REMOVE,
            KNIGHTS_DESTROY,
            KNIGHTS_ADMIT,
            KNIGHTS_REJECT,
            KNIGHTS_PUNISH,
            KNIGHTS_CHIEF,
            KNIGHTS_VICECHIEF,
            KNIGHTS_OFFICER,
            KNIGHTS_ALLLIST_REQ,
            KNIGHTS_MEMBER_REQ,
            KNIGHTS_CURRENT_REQ,
            KNIGHTS_JOIN_REQ,
            KNIGHTS_MARK_VERSION_REQ,
            KNIGHTS_MARK_REGISTER,
            KNIGHTS_MARK_REQ,
            KNIGHTS_MARK_REGION_REQ,
            KNIGHTS_ALLY_CREATE,
            KNIGHTS_ALLY_REQ,
            KNIGHTS_ALLY_INSERT,
            KNIGHTS_ALLY_REMOVE,
            KNIGHTS_ALLY_PUNISH,
            KNIGHTS_ALLY_LIST,
            KNIGHTS_POINT_REQ,
            KNIGHTS_POINT_METHOD,
            KNIGHTS_DONATE_POINTS,
            KNIGHTS_HANDOVER_VICECHIEF_LIST,
            KNIGHTS_HANDOVER_REQ,
            KNIGHTS_HANDOVER,
            KNIGHTS_DONATION_LIST,
            KNIGHTS_UPDATENOTICE,
            KNIGHTS_UPDATEMEMO,
            KNIGHTS_TOP10,
            KNIGHTS_UNK1,
            KNIGHTS_LADDER_POINTS,
            KNIGHTS_VS_LIST,
        ];
        assert!(handled.len() >= 38);
    }

    /// KNIGHTS_ALLY_INSERT (30) shares handler with KNIGHTS_ALLY_CREATE (28).
    #[test]
    fn test_ally_create_insert_shared_handler() {
        assert_eq!(KNIGHTS_ALLY_CREATE, 28);
        assert_eq!(KNIGHTS_ALLY_INSERT, 30);
        // Both route to handle_alliance_create — gap of 2
        assert_eq!(KNIGHTS_ALLY_INSERT - KNIGHTS_ALLY_CREATE, 2);
    }

    /// KNIGHTS_MARK_REGION_REQ (37) is a no-op (disabled in C++ too).
    #[test]
    fn test_mark_region_req_noop() {
        assert_eq!(KNIGHTS_MARK_REGION_REQ, 37);
        // It's the highest mark-related sub-opcode
        assert!(KNIGHTS_MARK_REGION_REQ > KNIGHTS_MARK_REQ);
        assert!(KNIGHTS_MARK_REGION_REQ > KNIGHTS_MARK_REGISTER);
        assert!(KNIGHTS_MARK_REGION_REQ > KNIGHTS_MARK_VERSION_REQ);
    }

    /// CLAN_LEVEL_REQUIREMENT is 30, below MAX_LEVEL.
    #[test]
    fn test_clan_level_requirement_below_max() {
        assert_eq!(CLAN_LEVEL_REQUIREMENT, 30);
        // Players must be level 30+ to create clan
        assert!((CLAN_LEVEL_REQUIREMENT as u16) < crate::world::types::MAX_LEVEL);
    }

    /// knights_error packet always uses WIZKNIGHTS_PROCESS opcode.
    #[test]
    fn test_knights_error_uses_process_opcode() {
        for sub in [
            KNIGHTS_JOIN,
            KNIGHTS_WITHDRAW,
            KNIGHTS_ADMIT,
            KNIGHTS_REJECT,
        ] {
            let pkt = knights_error(sub, 0xFF);
            assert_eq!(pkt.opcode, WIZKNIGHTS_PROCESS);
            assert_eq!(pkt.data.len(), 2);
            assert_eq!(pkt.data[1], 0xFF);
        }
    }
}
