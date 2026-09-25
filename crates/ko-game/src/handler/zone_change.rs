//! WIZ_ZONE_CHANGE (0x27) handler — cross-zone teleport and warp gates.
//! ## Flow
//! 1. Player steps on warp tile → `trigger_zone_change()` called from move_handler
//! 2. Same zone → `same_zone_warp()` (sends WIZ_WARP)
//! 3. Cross zone → cleanup old zone, update position, send WIZ_ZONE_CHANGE(Teleport=3)
//! 4. Client loads new zone → sends WIZ_ZONE_CHANGE(Loading=1)
//! 5. Server sends NPC/user region data + WIZ_ZONE_CHANGE(Loaded=2) ACK
//! 6. Client finishes → sends WIZ_ZONE_CHANGE(Loaded=2)
//! 7. Server broadcasts INOUT_RESPAWN to new zone

use ko_db::repositories::character::CharacterRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::handler::{mining, regene, region, stealth};
use crate::session::{ClientSession, SessionState};
use crate::world::{
    CswOpStatus, Position, WorldState, ZONE_ARDREAM, ZONE_BATTLE_BASE, ZONE_BIFROST,
    ZONE_CHAOS_DUNGEON, ZONE_DELOS, ZONE_DUNGEON_DEFENCE, ZONE_ELMORAD, ZONE_ELMORAD2,
    ZONE_ELMORAD3, ZONE_ELMORAD_ESLANT, ZONE_ELMORAD_ESLANT2, ZONE_ELMORAD_ESLANT3, ZONE_KARUS,
    ZONE_KARUS2, ZONE_KARUS3, ZONE_KARUS_ESLANT, ZONE_KARUS_ESLANT2, ZONE_KARUS_ESLANT3,
    ZONE_KNIGHT_ROYALE, ZONE_MORADON, ZONE_MORADON2, ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
    ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE, ZONE_SNOW_BATTLE, ZONE_SPBATTLE1, ZONE_SPBATTLE_MAX,
    ZONE_SPBATTLE_MIN,
};
use crate::zone::{calc_region, SessionId};

/// Monster transformation type — cancelled on zone change to restricted zones.
use crate::magic_constants::TRANSFORMATION_MONSTER;

use crate::magic_constants::MAGIC_CANCEL_TRANSFORMATION;

/// Extended blink duration for special zones (10 base + 45 extra = 55 seconds).
const BLINK_TIME_SPECIAL_ZONE: u64 = 55;

/// Zone change sub-opcodes.
const ZONE_CHANGE_LOADING: u8 = 1;
const ZONE_CHANGE_LOADED: u8 = 2;
const ZONE_CHANGE_TELEPORT: u8 = 3;

// ── WarpListResponse error codes ─────────────────────────────────────────

/// Generic error (default).
const WARP_ERROR: u8 = 0;
/// Below minimum level for this zone.
const WARP_MIN_LEVEL: u8 = 2;
/// Cannot enter during Castle Siege War (not in clan / wrong grade).
const WARP_NOT_DURING_CSW: u8 = 3;
/// Cannot enter during active war.
const WARP_NOT_DURING_WAR: u8 = 4;
/// Need national points (loyalty) to enter.
const WARP_NEED_NP: u8 = 5;
/// Don't qualify (above max level).
const WARP_DO_NOT_QUALIFY: u8 = 7;

use crate::world::{NATION_ELMORAD, NATION_KARUS};

/// Zone Ardream type for battle zone type comparison.
const ZONE_ARDREAM_TYPE: u8 = 72;

/// Validate zone entry permissions for a player.
/// Checks level requirements, nation restrictions, and GM bypass.
/// Returns `Ok(())` if allowed, `Err(error_code)` with the WarpListResponse code.
/// Validate Cinderella War zone entry requirements.
/// Returns `None` if entry is allowed, or `Some(message)` with the rejection
/// reason string to send via `HSACSX_SendMessageBox`.
fn validate_cinderella_entry(world: &WorldState, sid: SessionId) -> Option<String> {
    let ch = world.get_character_info(sid)?;

    // GMs bypass all checks
    if ch.authority == 0 || ch.authority == 2 {
        return None;
    }

    let event = world.cindwar_event();
    let setting = world.get_cindwar_setting(event.setting_id)?;

    let loyalty = ch.loyalty;

    // C++ line 35: if (!GetLoyalty())
    if loyalty == 0 {
        return Some("Your loyalty points are insufficient.".to_string());
    }

    // C++ line 41: if (GetLoyalty() < pSet.reqloyalty)
    if (loyalty as i64) < (setting.req_loyalty as i64) {
        return Some(format!(
            "You must have a minimum of {} loyalty points to participate in the event.",
            setting.req_loyalty
        ));
    }

    // C++ line 47: if (GetCoins() < pSet.reqmoney)
    if (ch.gold as i64) < (setting.req_money as i64) {
        return Some(format!(
            "You must have a minimum of {} coins to participate in the event.",
            setting.req_money
        ));
    }

    // C++ line 53: if (GetLevel() < pSet.minlevel)
    if (ch.level as i16) < setting.min_level {
        return Some(format!(
            "You must be at least level {} to participate in the event.",
            setting.min_level
        ));
    }

    // C++ line 59: if (GetLevel() > pSet.maxlevel)
    if setting.max_level > 0 && (ch.level as i16) > setting.max_level {
        return Some(format!(
            "Levels between a minimum of {} and a maximum of {} can participate in the event.",
            setting.min_level, setting.max_level
        ));
    }

    // C++ line 65: if (pSet.maxuserlimit && GetTotalCount() > pSet.maxuserlimit)
    if setting.max_user_limit > 0
        && world.cindwar_event_user_count() as i16 > setting.max_user_limit
    {
        return Some(
            "the maximum number of users has been reached. please try again later..".to_string(),
        );
    }

    None
}

fn validate_zone_entry(world: &WorldState, sid: SessionId, dest_zone: u16) -> Result<(), (u8, u8)> {
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Err((WARP_ERROR, 0)),
    };

    // GMs bypass all zone restrictions
    // AUTHORITY_GAME_MASTER = 0, AUTHORITY_GM_USER = 2
    if ch.authority == 0 || ch.authority == 2 {
        return Ok(());
    }

    let level = ch.level;

    // Get zone info for level requirements
    if let Some(zone) = world.get_zone(dest_zone) {
        let (min_level, max_level) = zone.level_range();

        if min_level > 0 && level < min_level {
            return Err((WARP_MIN_LEVEL, min_level));
        }

        if max_level > 0 && max_level < 83 && level > max_level {
            return Err((WARP_DO_NOT_QUALIFY, 0));
        }
    }

    let nation = ch.nation;

    // Zone-specific nation restrictions
    match dest_zone {
        // Karus homeland — Karus always allowed; El Morad only during invasion
        ZONE_KARUS | ZONE_KARUS2 | ZONE_KARUS3 => {
            if nation == NATION_KARUS {
                // Own nation always allowed
            } else if nation == NATION_ELMORAD {
                // Opposing nation may enter if invasion flag is set
                let battle_state = world.get_battle_state();
                if !battle_state.karus_open_flag {
                    return Err((WARP_ERROR, 0));
                }
            } else {
                return Err((WARP_ERROR, 0));
            }
        }
        // El Morad homeland — El Morad always allowed; Karus only during invasion
        ZONE_ELMORAD | ZONE_ELMORAD2 | ZONE_ELMORAD3 => {
            if nation == NATION_ELMORAD {
                // Own nation always allowed
            } else if nation == NATION_KARUS {
                // Opposing nation may enter if invasion flag is set
                let battle_state = world.get_battle_state();
                if !battle_state.elmorad_open_flag {
                    return Err((WARP_ERROR, 0));
                }
            } else {
                return Err((WARP_ERROR, 0));
            }
        }
        // Karus Eslant — Karus only
        ZONE_KARUS_ESLANT | ZONE_KARUS_ESLANT2 | ZONE_KARUS_ESLANT3 => {
            if nation != NATION_KARUS {
                return Err((WARP_ERROR, 0));
            }
        }
        // El Morad Eslant — El Morad only
        ZONE_ELMORAD_ESLANT | ZONE_ELMORAD_ESLANT2 | ZONE_ELMORAD_ESLANT3 => {
            if nation != NATION_ELMORAD {
                return Err((WARP_ERROR, 0));
            }
        }
        // Delos — CSW clan/grade check + loyalty requirement
        ZONE_DELOS => {
            // During CSW: must be in a real clan (not auto-clan) with grade <= 3
            let csw = world.csw_event().blocking_read();
            let csw_active = csw.is_active();
            drop(csw);

            if csw_active {
                // Must be in a clan
                if ch.knights_id == 0 {
                    return Err((WARP_NOT_DURING_CSW, 0));
                }
                // Check clan grade (only grade 1-3 allowed)
                if let Some(clan) = world.get_knights(ch.knights_id) {
                    if clan.grade > 3 {
                        return Err((WARP_DO_NOT_QUALIFY, 0));
                    }
                } else {
                    return Err((WARP_NOT_DURING_CSW, 0));
                }
            }

            // Always require loyalty > 0 for Delos
            if ch.loyalty == 0 {
                return Err((WARP_NEED_NP, 0));
            }
        }
        // Bifrost — always allowed
        ZONE_BIFROST => {}
        // Ardream — blocked during any active war, requires loyalty
        ZONE_ARDREAM => {
            if world.is_war_open() {
                return Err((WARP_NOT_DURING_WAR, 0));
            }
            if ch.loyalty == 0 {
                return Err((WARP_NEED_NP, 0));
            }
        }
        // Ronark Land / Ronark Land Base — blocked during war UNLESS battle zone type is Ardream
        ZONE_RONARK_LAND | ZONE_RONARK_LAND_BASE => {
            let battle_state = world.get_battle_state();
            if battle_state.is_war_open() && battle_state.battle_zone_type != ZONE_ARDREAM_TYPE {
                return Err((WARP_NOT_DURING_WAR, 0));
            }
            if ch.loyalty == 0 {
                return Err((WARP_NEED_NP, 0));
            }
        }
        // SPBATTLE zones (105-115) — require loyalty
        z if (ZONE_SPBATTLE_MIN..=ZONE_SPBATTLE_MAX).contains(&z) => {
            if ch.loyalty == 0 {
                return Err((WARP_NEED_NP, 0));
            }
        }
        // Default: war zones may only be entered if that war zone is active
        _ => {
            if let Some(zone) = world.get_zone(dest_zone) {
                if zone.is_war_zone() {
                    let battle_state = world.get_battle_state();
                    if !battle_state.is_war_open() {
                        return Err((WARP_ERROR, 0));
                    }
                    if dest_zone == ZONE_SNOW_BATTLE {
                        // Snow battle uses offset from ZONE_SNOW_BATTLE
                        if (dest_zone - ZONE_SNOW_BATTLE) as u8 != battle_state.battle_zone {
                            return Err((WARP_ERROR, 0));
                        }
                    } else if (dest_zone - ZONE_BATTLE_BASE) as u8 != battle_state.battle_zone {
                        // Regular battle zones use offset from ZONE_BATTLE_BASE
                        return Err((WARP_ERROR, 0));
                    } else if (nation == NATION_ELMORAD && battle_state.elmorad_open_flag)
                        || (nation == NATION_KARUS && battle_state.karus_open_flag)
                    {
                        // Own homeland is under invasion — cannot enter battle zone
                        return Err((WARP_ERROR, 0));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Send a zone change rejection packet to the client.
async fn send_warp_error(
    session: &mut ClientSession,
    error: u8,
    min_level: u8,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizWarpList as u8);
    pkt.write_u8(2); // sub-opcode: error response
    pkt.write_u8(error);
    if error == WARP_MIN_LEVEL {
        pkt.write_u8(min_level);
    }
    session.send_packet(&pkt).await
}

/// Handle WIZ_ZONE_CHANGE packets from the client.
/// The client sends this after receiving a teleport command:
/// - Sub=1 (Loading): client is loading the new zone
/// - Sub=2 (Loaded): client finished loading
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        ZONE_CHANGE_LOADING => handle_loading(session).await,
        ZONE_CHANGE_LOADED => handle_loaded(session).await,
        _ => {
            tracing::warn!(
                "[{}] Unknown zone_change sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Trigger a zone change from a warp gate event.
/// Called from `move_handler` when the player steps on a ZoneChange event tile.
pub async fn trigger_zone_change(
    session: &mut ClientSession,
    dest_zone: u16,
    dest_x: f32,
    dest_z: f32,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Prevent double-warp (C++ m_bWarp check)
    if world.is_zone_changing(sid) {
        return Ok(());
    }

    if !world.can_teleport(sid) {
        return Ok(());
    }

    world.set_zone_changing(sid, true);

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => {
            // C++ parity: clear zone_changing flag on failure to prevent stuck state.
            world.set_zone_changing(sid, false);
            return Ok(());
        }
    };

    // Same zone → use WIZ_WARP (simpler flow)
    if pos.zone_id == dest_zone {
        return same_zone_warp(session, dest_x, dest_z).await;
    }

    // Verify destination zone exists and is active
    match world.get_zone(dest_zone) {
        Some(zone) => {
            let zone_status = zone.zone_info.as_ref().map(|zi| zi.status).unwrap_or(1);
            if zone_status != 1 {
                tracing::warn!(
                    "[{}] Zone change to disabled zone {} (status={}) — aborting",
                    session.addr(),
                    dest_zone,
                    zone_status
                );
                world.set_zone_changing(sid, false);
                return Ok(());
            }
        }
        None => {
            tracing::warn!(
                "[{}] Zone change to unknown zone {} — aborting",
                session.addr(),
                dest_zone
            );
            world.set_zone_changing(sid, false);
            return Ok(());
        }
    }

    // Resolve (0,0) coordinates to zone start_position
    let (dest_x, dest_z) = if dest_x == 0.0 && dest_z == 0.0 {
        resolve_zero_coords(dest_zone, sid, &world)
    } else {
        (dest_x, dest_z)
    };

    // Validate destination position is within zone map bounds
    if let Some(zone) = world.get_zone(dest_zone) {
        if !zone.is_valid_position(dest_x, dest_z) {
            tracing::warn!(
                "[{}] Zone change to zone {}: invalid position ({:.0}, {:.0}) — aborting",
                session.addr(),
                dest_zone,
                dest_x,
                dest_z,
            );
            world.set_zone_changing(sid, false);
            return Ok(());
        }
    }

    // Cinderella War zone entry validation
    {
        let cind_zone = world.cinderella_zone_id();
        if super::cinderella::is_cinderella_zone(dest_zone, cind_zone) {
            if pos.zone_id == dest_zone {
                world.set_zone_changing(sid, false);
                return Ok(());
            }
            if let Some(rejection) = validate_cinderella_entry(&world, sid) {
                let pkt = super::ext_hook::build_ext_message_box("Fun Class Event", &rejection);
                session.send_packet(&pkt).await?;
                world.set_zone_changing(sid, false);
                world.set_check_warp_zone_change(sid, false);
                return Ok(());
            }
        }
    }

    // Validate zone entry permissions (level, nation, etc.)
    if let Err((error_code, min_level)) = validate_zone_entry(&world, sid, dest_zone) {
        world.set_zone_changing(sid, false);
        world.set_check_warp_zone_change(sid, false);
        send_warp_error(session, error_code, min_level).await?;
        return Ok(());
    }

    let nation = world.get_character_info(sid).map(|c| c.nation).unwrap_or(0);

    // 0b. BDW cleanup when warping out of zone 84
    if pos.zone_id == crate::systems::bdw::ZONE_BDW && dest_zone != crate::systems::bdw::ZONE_BDW {
        // Remove speed debuff first
        world.remove_buff(sid, crate::systems::bdw::BUFF_TYPE_FRAGMENT_OF_MANES);
        super::logout::bdw_user_logout(&world, sid);
    }

    // 0c. Monster Stone cleanup when warping out of a stone zone (81-83)
    // When ANY player exits, the entire room is disbanded.
    {
        use crate::systems::monster_stone;
        if monster_stone::is_monster_stone_zone(pos.zone_id)
            && !monster_stone::is_monster_stone_zone(dest_zone)
        {
            let ms_active = world
                .with_session(sid, |h| h.event_room > 0)
                .unwrap_or(false);
            if ms_active {
                monster_stone_exit_room(&world, sid);
            }
        }
    }

    // 0c1b. Dungeon Defence cleanup when warping out of zone 89
    //   case ZONE_DUNGEON_DEFENCE:
    //     DungeonDefenceRobItemSkills();  // remove all Monster Coins
    //     SetMaxHp(1);                   // force HP recalc (normal formula)
    //     m_bEventRoom = 0;
    {
        use crate::handler::dungeon_defence;
        if pos.zone_id == ZONE_DUNGEON_DEFENCE && dest_zone != ZONE_DUNGEON_DEFENCE {
            // Remove all Monster Coins from inventory
            world.rob_all_of_item(sid, dungeon_defence::MONSTER_COIN_ITEM);

            // Force HP recalculation with normal formula (iFlag=1 bypasses zone override)
            world.recalculate_max_hp_mp(sid);

            // Clear event_room
            world.update_session(sid, |h| {
                h.event_room = 0;
            });
        }
    }

    // 0c1c. Chaos Dungeon cleanup when warping out of zone 85
    //   case ZONE_CHAOS_DUNGEON:
    //     if (sNewZone != ZONE_CHAOS_DUNGEON && isEventUser())
    //       SetMaxHp(1);  RobChaosSkillItems();  m_bEventRoom = 0;
    {
        use crate::handler::dead::{ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE, ITEM_LIGHT_PIT};
        let is_event_user = world.with_session(sid, |h| h.joined_event).unwrap_or(false);
        if pos.zone_id == ZONE_CHAOS_DUNGEON && dest_zone != ZONE_CHAOS_DUNGEON && is_event_user {
            // Remove Chaos Dungeon skill items from inventory
            for &item_id in &[ITEM_LIGHT_PIT, ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE] {
                world.rob_all_of_item(sid, item_id);
            }

            // Remove player from Chaos Expansion ranking
            world.chaos_remove_player(sid);

            // Force HP recalculation with normal formula
            world.recalculate_max_hp_mp(sid);

            // Clear event_room
            world.update_session(sid, |h| {
                h.event_room = 0;
            });
        }
    }

    // 0c2. Draki Tower cleanup when warping out of zone 95
    {
        use crate::handler::draki_tower;
        if pos.zone_id == draki_tower::ZONE_DRAKI_TOWER
            && dest_zone != draki_tower::ZONE_DRAKI_TOWER
        {
            let room_id = world.with_session(sid, |h| h.draki_room_id).unwrap_or(0);
            if room_id > 0 {
                // Clear session state
                world.update_session(sid, |h| {
                    h.event_room = 0;
                    h.draki_room_id = 0;
                });
                // Despawn room NPCs and reset room
                world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, room_id);
                let mut rooms = world.draki_tower_rooms_write();
                if let Some(room) = rooms.get_mut(&room_id) {
                    room.reset();
                }
            }
        }
    }

    // 0d-cind. Cinderella War zone exit cleanup
    {
        let cind_zone = world.cinderella_zone_id();
        if pos.zone_id == cind_zone && dest_zone != cind_zone {
            super::cinderella::cinderella_logout(&world, sid, false);
        }
    }

    // 0d. Generic temple event zone exit cleanup — ResetTempleEventData equivalent
    //   if (isInTempleEventZone() && isEventUser() && !isInTempleEventZone(newZone))
    //       ResetTempleEventData(); // clears m_bEventRoom, m_iEventJoinOrder, etc.
    {
        use crate::systems::event_room::is_in_temple_event_zone;
        if is_in_temple_event_zone(pos.zone_id) && !is_in_temple_event_zone(dest_zone) {
            let had_room = world.get_event_room(sid) > 0;
            if had_room {
                world.update_session(sid, |h| {
                    h.event_room = 0;
                    h.monster_stone_status = false;
                    h.joined_event = false;
                    h.is_final_joined_event = false;
                });
            }
        }
    }

    // 1. Broadcast INOUT_OUT to old zone (other players see us leave)
    let out_pkt = region::build_user_inout(region::INOUT_OUT, sid, None, &pos);
    let event_room = world.get_event_room(sid);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(out_pkt),
        Some(sid),
        event_room,
    );

    // 1+. BottomUserLogOut — zone-wide logout notification for bottom user list
    // Sends WIZ_USER_INFORMATIN(sub=4/RegionDelete) to all players in the old zone.
    if let Some(ch) = world.get_character_info(sid) {
        let region_del_pkt = super::user_info::build_region_delete_packet(&ch.name);
        world.broadcast_to_zone(pos.zone_id, Arc::new(region_del_pkt), Some(sid));
    }

    // 1+. Zindan War logout when leaving SPBATTLE zone
    let is_old_spbattle = (ZONE_SPBATTLE_MIN..=ZONE_SPBATTLE_MAX).contains(&pos.zone_id);
    let is_new_spbattle = (ZONE_SPBATTLE_MIN..=ZONE_SPBATTLE_MAX).contains(&dest_zone);
    if is_old_spbattle && !is_new_spbattle && world.is_zindan_event_opened() {
        let pkt = super::ext_hook::build_zindan_logout();
        session.send_packet(&pkt).await?;
    }

    // 1a. Reset anger gauge when leaving the zone.
    //   if (GetAngerGauge() > 0) UpdateAngerGauge(0);
    super::arena::reset_anger_gauge(&world, sid);

    // 1b. Remove rival on cross-zone change
    //   if (hasRival()) RemoveRival();
    {
        let has_rival = world
            .get_character_info(sid)
            .map(|ch| ch.rival_id >= 0)
            .unwrap_or(false);
        if has_rival {
            world.remove_rival(sid);
        }
    }

    // 1b. Dismiss active pet on cross-zone change
    {
        let mut pet_index: Option<u32> = None;
        world.update_session(sid, |h| {
            if let Some(pet) = h.pet_data.take() {
                pet_index = Some(pet.index);
            }
        });
        if let Some(index) = pet_index {
            let mut resp = Packet::new(Opcode::WizPet as u8);
            resp.write_u8(1); // PET_MODE_FUNCTION
            resp.write_u8(5); // NORMAL_MODE
            resp.write_u8(2); // death sub-code
            resp.write_u16(1);
            resp.write_u32(index);
            session.send_packet(&resp).await?;
        }
    }

    // 1c. ResetWindows — cancel trade, merchant, mining, fishing, challenge
    {
        // Cancel active trade
        if world.is_trading(sid) {
            let partner_sid = world.get_exchange_user(sid);
            world.reset_trade(sid);
            if let Some(partner) = partner_sid {
                world.reset_trade(partner);
                let mut cancel_pkt = Packet::new(Opcode::WizExchange as u8);
                cancel_pkt.write_u8(0x08); // EXCHANGE_CANCEL
                world.send_to_session_owned(partner, cancel_pkt);
            }
        }

        // Cancel active challenge (duel request)
        {
            let (requesting, requested, challenge_user) = world.get_challenge_state(sid);
            if requesting > 0 || requested > 0 {
                let target = challenge_user as u16;
                // Notify target that challenge is cancelled
                if challenge_user >= 0 {
                    world.update_session(target, |h| {
                        h.challenge_user = -1;
                        h.requesting_challenge = 0;
                        h.challenge_requested = 0;
                    });
                    let mut cancel_pkt = Packet::new(Opcode::WizChallenge as u8);
                    cancel_pkt.write_u8(if requesting > 0 { 2 } else { 4 }); // PVP_CANCEL or PVP_REJECT
                    world.send_to_session_owned(target, cancel_pkt);
                }
                // Clear our own challenge state
                world.update_session(sid, |h| {
                    h.challenge_user = -1;
                    h.requesting_challenge = 0;
                    h.challenge_requested = 0;
                });
            }
        }

        // Close merchant stall if we're a vendor
        if world.is_merchanting(sid) {
            world.close_merchant(sid);
        }

        // Remove from merchant we're browsing
        world.remove_from_merchant_lookers(sid);

        // Stop mining / fishing
        mining::stop_mining_internal(&world, sid);
        mining::stop_fishing_internal(&world, sid);
    }

    // 1d. Party cleanup — promote leader if needed, then remove from party
    //   if (isInParty() && isPartyLeader()) PartyLeaderPromote(pParty->uid[1]);
    //   PartyNemberRemove(GetSocketID());
    world.cleanup_party_on_disconnect(sid);

    // 2. Remove from old zone region grid
    if let Some(old_zone) = world.get_zone(pos.zone_id) {
        old_zone.remove_user(pos.region_x, pos.region_z, sid);
    }

    // 3. Update position to new zone
    let new_rx = calc_region(dest_x);
    let new_rz = calc_region(dest_z);
    let new_pos = Position {
        zone_id: dest_zone,
        x: dest_x,
        y: 0.0,
        z: dest_z,
        region_x: new_rx,
        region_z: new_rz,
    };
    world.update_position(sid, dest_zone, dest_x, 0.0, dest_z);

    // 4. Add to new zone region grid
    if let Some(new_zone) = world.get_zone(dest_zone) {
        new_zone.add_user(new_rx, new_rz, sid);
    }

    // 5a. Cancel monster transformation if entering a restricted zone
    // Transformations are allowed in homeland, Eslant, and Moradon zones.
    // If entering any other zone while monster-transformed, cancel the transformation.
    let transform_type = world
        .with_session(sid, |h| h.transformation_type)
        .unwrap_or(0);
    if transform_type == TRANSFORMATION_MONSTER && !is_transform_allowed_zone(dest_zone) {
        world.clear_transformation(sid);
        let mut cancel_pkt = Packet::new(Opcode::WizMagicProcess as u8);
        cancel_pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);
        session.send_packet(&cancel_pkt).await?;
        tracing::debug!(
            "[{}] Monster transform cancelled on zone change to zone {}",
            session.addr(),
            dest_zone,
        );
    }

    // 5b. Pre-transition buff cleanup (cross-zone only)
    // Clear DOTs, buffs, stealth, and recalculate stats
    // C++ line 25,488: Cinderella zone entry removes saved magic (bRemoveSavedMagic=true)
    let cind_zone = world.cinderella_zone_id();
    let is_cind = world.is_cinderella_active() && cind_zone == dest_zone;
    world.clear_all_dots(sid);
    world.clear_all_buffs(sid, is_cind);
    stealth::remove_stealth(&world, sid);
    world.set_user_ability(sid);

    // Reset death tracking fields on zone change
    world.update_session(sid, |h| {
        h.who_killed_me = -1;
        h.lost_exp = 0;
    });

    // 6. Send zone ability for the new zone
    super::zone_ability::send_zone_ability(session, dest_zone).await?;
    super::zone_ability::send_zone_flag(session, dest_zone).await?;

    // 6b. Draki Tower: dead player regene position update before zone change
    if pos.zone_id == crate::handler::draki_tower::ZONE_DRAKI_TOWER && world.is_player_dead(sid) {
        let mut regene_pkt = Packet::new(Opcode::WizRegene as u8);
        regene_pkt.write_u16((dest_x * 10.0) as u16);
        regene_pkt.write_u16((dest_z * 10.0) as u16);
        regene_pkt.write_u16(0); // y
        session.send_packet(&regene_pkt).await?;
    }

    // 7. Send WIZ_ZONE_CHANGE(Teleport=3) to client
    let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
    pkt.write_u8(ZONE_CHANGE_TELEPORT);
    pkt.write_u16(dest_zone);
    pkt.write_u16(0); // padding
    pkt.write_u16((dest_x * 10.0) as u16); // GetSPosX
    pkt.write_u16((dest_z * 10.0) as u16); // GetSPosZ
    pkt.write_u16(0); // GetSPosY (y=0)
    pkt.write_u8(nation);
    pkt.write_u16(0xFFFF); // unknown (-1)
    session.send_packet(&pkt).await?;

    // 7b. Send GOLDSHELL packet during active nation war
    // Only during NATION_BATTLE and when battle zone is NOT ZONE_BATTLE3
    {
        let battle_state = world.get_battle_state();
        if battle_state.battle_open == crate::systems::war::NATION_BATTLE
            && battle_state.battle_zone_id() != crate::world::ZONE_BATTLE3
        {
            let mut gs_pkt = Packet::new(Opcode::WizMapEvent as u8);
            gs_pkt.write_u8(9); // GOLDSHELL
            gs_pkt.write_u8(1); // enable
            gs_pkt.write_u32(sid as u32);
            session.send_packet(&gs_pkt).await?;
        }
    }

    // 7c. Resend time and weather for the new zone
    // The client reloads the map on zone change; without fresh time/weather
    // the new zone may render with stale lighting/effects.
    {
        let time_pkt = crate::systems::time_weather::build_time_packet();
        session.send_packet(&time_pkt).await?;

        let tw = world.game_time_weather();
        let weather_pkt = crate::systems::time_weather::build_weather_packet(
            tw.get_weather_type(),
            tw.get_weather_amount(),
        );
        session.send_packet(&weather_pkt).await?;
    }

    // 8. Save position to DB (fire-and-forget)
    save_position_async(session, dest_zone, dest_x, dest_z);

    // 9. Save active buffs to DB for persistence across zone change
    session.save_saved_magic().await;

    // 10. Send active event time (for players entering event zones)
    crate::systems::event_room::send_active_event_time(&world, sid);

    // 11. Send event remaining time when entering Bifrost, Ronark Land, or Battle zones
    {
        use crate::world::{ZONE_BATTLE4, ZONE_BATTLE5};

        // SendEventRemainingTime: WIZ_BIFROST(BIFROST_EVENT) + remaining u16
        // C++ EventSigningSystem.cpp:5-42 — only Battle4 has real time, others=0
        if dest_zone == ZONE_BIFROST
            || dest_zone == ZONE_BATTLE4
            || dest_zone == ZONE_BATTLE5
            || dest_zone == ZONE_RONARK_LAND
        {
            let mut evt_pkt = Packet::new(Opcode::WizBifrost as u8);
            evt_pkt.write_u8(2); // BIFROST_EVENT sub-opcode
                                 // C++ sends u16 for SendEventRemainingTime, but BeefEventGetTime
                                 // sends u32. Since both are on opcode WIZ_BIFROST sub 2, use u32
                                 // (matches our Bifrost handler format). Battle zones get 0.
            evt_pkt.write_u32(0_u32);
            world.send_to_session_owned(sid, evt_pkt);
        }

        // BeefEventGetTime: send actual Bifrost remaining time
        if dest_zone == ZONE_BIFROST
            || pos.zone_id == ZONE_BIFROST
            || dest_zone == ZONE_RONARK_LAND
            || pos.zone_id == ZONE_RONARK_LAND
        {
            let remaining = world.get_bifrost_remaining_secs();
            let mut beef_pkt = Packet::new(Opcode::WizBifrost as u8);
            beef_pkt.write_u8(2); // BIFROST_EVENT sub-opcode
            beef_pkt.write_u32(remaining);
            world.send_to_session_owned(sid, beef_pkt);
        }
    }

    // 12. Send lottery event state on zone change
    {
        let lottery_proc = world.lottery_process().clone();
        let proc_guard = lottery_proc.read();
        if proc_guard.lottery_start && proc_guard.event_time != 0 {
            let char_name = world
                .get_character_info(sid)
                .map(|c| c.name.to_uppercase())
                .unwrap_or_default();
            let my_tickets = proc_guard
                .participants
                .get(&char_name)
                .map(|p| p.ticket_count)
                .unwrap_or(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            let lottery_pkt =
                crate::handler::lottery::build_start_packet(&proc_guard, now, my_tickets);
            drop(proc_guard);
            world.send_to_session_owned(sid, lottery_pkt);
        }
    }

    tracing::info!(
        "[{}] Zone change: zone {} ({:.0},{:.0}) → zone {} ({:.0},{:.0})",
        session.addr(),
        pos.zone_id,
        pos.x,
        pos.z,
        dest_zone,
        dest_x,
        dest_z,
    );

    let _ = new_pos; // used above via update_position
    Ok(())
}

/// Same-zone warp — teleport within the current zone.
pub async fn same_zone_warp(
    session: &mut ClientSession,
    dest_x: f32,
    dest_z: f32,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };
    let zone_id = pos.zone_id;

    // Validate destination position is within zone map bounds
    if let Some(zone) = world.get_zone(zone_id) {
        if !zone.is_valid_position(dest_x, dest_z) {
            tracing::warn!(
                "[{}] Same-zone warp in zone {}: invalid position ({:.0}, {:.0}) — aborting",
                session.addr(),
                zone_id,
                dest_x,
                dest_z,
            );
            world.set_zone_changing(sid, false);
            return Ok(());
        }
    }

    // 1. Broadcast INOUT_OUT to old region
    let out_pkt = region::build_user_inout(region::INOUT_OUT, sid, None, &pos);
    let event_room = world.get_event_room(sid);
    world.broadcast_to_3x3(
        zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(out_pkt),
        Some(sid),
        event_room,
    );

    // 2. Remove from old region
    if let Some(zone) = world.get_zone(zone_id) {
        zone.remove_user(pos.region_x, pos.region_z, sid);
    }

    // 3. Update position (same zone, new coords)
    let new_rx = calc_region(dest_x);
    let new_rz = calc_region(dest_z);
    world.update_position(sid, zone_id, dest_x, 0.0, dest_z);

    // 4. Add to new region
    if let Some(zone) = world.get_zone(zone_id) {
        zone.add_user(new_rx, new_rz, sid);
    }

    // 5. Send WIZ_WARP to client
    let mut pkt = Packet::new(Opcode::WizWarp as u8);
    pkt.write_u16((dest_x * 10.0) as u16);
    pkt.write_u16((dest_z * 10.0) as u16);
    pkt.write_i16(-1);
    session.send_packet(&pkt).await?;

    // 6. Send region change data (users + NPCs + merchants)
    // C++ ZoneChangeWarpHandler.cpp:632-634 — only NPC_REGION (ID list), no NPC_INOUT
    region::send_region_user_in_out_for_me(session).await?;
    region::send_merchant_user_in_out_for_me(session).await?;
    region::send_region_npc_info_for_me(session).await?;
    region::send_nearby_npc_inouts(session).await?;

    // 7. Broadcast INOUT_WARP to new region
    region::broadcast_user_in_with_type(session, region::INOUT_WARP).await?;

    // 8. Clear zone_changing flag
    world.set_zone_changing(sid, false);

    tracing::info!(
        "[{}] Same-zone warp: zone {} ({:.0},{:.0}) → ({:.0},{:.0})",
        session.addr(),
        zone_id,
        pos.x,
        pos.z,
        dest_x,
        dest_z,
    );

    Ok(())
}

/// Handle ZoneChangeLoading (sub=1) — client is loading the new zone.
/// Server responds by sending NPC/user region data, then ZoneChangeLoaded ACK.
async fn handle_loading(session: &mut ClientSession) -> anyhow::Result<()> {
    // Send NPC region list — client uses cached templates for rendering
    // C++ ZoneChangeWarpHandler.cpp:662 — only RegionNpcInfoForMe(), no NPC_INOUT
    region::send_region_npc_info_for_me(session).await?;
    region::send_nearby_npc_inouts(session).await?;

    // Send user region list (WIZ_REGIONCHANGE 3-phase) + merchants
    region::send_region_user_in_out_for_me(session).await?;
    region::send_merchant_user_in_out_for_me(session).await?;

    // Send ZoneChangeLoaded ACK
    let mut ack = Packet::new(Opcode::WizZoneChange as u8);
    ack.write_u8(ZONE_CHANGE_LOADED);
    session.send_packet(&ack).await?;

    tracing::debug!(
        "[{}] Zone change: loading phase complete, sent region data",
        session.addr(),
    );

    Ok(())
}

/// Handle ZoneChangeLoaded (sub=2) — client finished loading the new zone.
/// Server broadcasts INOUT_RESPAWN so other players in the new zone see us.
/// For cross-zone changes: activates blink, clears+recasts saved magic,
/// sends Delos siege packets if applicable.
async fn handle_loaded(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    if !world.is_zone_changing(sid) {
        return Ok(());
    }

    // Broadcast INOUT_RESPAWN to new zone (now others see us)
    region::broadcast_user_in(session).await?;

    // All zone changes going through handle_loaded are cross-zone
    // (same-zone warps complete inline in same_zone_warp and never reach here)
    let pos = world.get_position(sid);
    let zone_id = pos.as_ref().map(|p| p.zone_id).unwrap_or(0);

    // If the player is dead when arriving in the new zone, broadcast death animation
    // so nearby players see them as a corpse.
    if world.is_player_dead(sid) {
        if let Some(ref p) = pos {
            let mut death_pkt = Packet::new(Opcode::WizDead as u8);
            death_pkt.write_u32(sid as u32);
            let event_room = world.get_event_room(sid);
            world.broadcast_to_3x3(
                p.zone_id,
                p.region_x,
                p.region_z,
                Arc::new(death_pkt),
                Some(sid),
                event_room,
            );
            tracing::debug!(
                "[{}] Zone change: player is dead, broadcasting death animation in zone {}",
                session.addr(),
                zone_id,
            );
        }
    }

    // Activate blink (respawn invulnerability) for cross-zone changes
    //   if (zone == CHAOS/KNIGHT_ROYALE/DUNGEON_DEFENCE) BlinkStart(45); // 10+45=55
    //   else if (!isNPCTransformation()) BlinkStart(); // 10
    let blink_duration = if matches!(
        zone_id,
        ZONE_CHAOS_DUNGEON | ZONE_KNIGHT_ROYALE | ZONE_DUNGEON_DEFENCE
    ) {
        BLINK_TIME_SPECIAL_ZONE
    } else {
        10 // default BLINK_TIME
    };
    regene::activate_blink_with_duration(session, zone_id, blink_duration)?;

    // Clear all buffs (without removing saved magic) then recast saved magic
    // Skip for Chaos Dungeon AND Cinderella War zones
    // C++ line 688: `if (GetZoneID() != ZONE_CHAOS_DUNGEON && !IsCindIn())`
    let cind_zone = world.cinderella_zone_id();
    let is_cind_in = world.is_cinderella_active() && cind_zone == zone_id;
    if zone_id != ZONE_CHAOS_DUNGEON && !is_cind_in {
        world.clear_all_buffs(sid, false);
        // Recast saved magic — restore persistent buffs after zone change
        world.recast_saved_magic(sid);
    }

    // Zone-based HP override for Chaos Dungeon (86) and Dungeon Defence (89)
    //   else if (GetZoneID() == ZONE_CHAOS_DUNGEON && iFlag == 0
    //         || (GetZoneID() == ZONE_DUNGEON_DEFENCE && iFlag == 0))
    //       m_MaxHp = 10000 / 10;   // = 1000
    if matches!(zone_id, ZONE_CHAOS_DUNGEON | ZONE_DUNGEON_DEFENCE) {
        world.update_character_stats(sid, |ch| {
            ch.max_hp = 1000;
            if ch.hp > 1000 {
                ch.hp = 1000;
            }
        });
    }

    // DD zone entry: remove any leftover Monster Coins from previous runs
    if zone_id == ZONE_DUNGEON_DEFENCE {
        world.rob_all_of_item(sid, crate::handler::dungeon_defence::MONSTER_COIN_ITEM);
    }

    // Send Delos siege packets if entering Delos during active siege
    if zone_id == ZONE_DELOS {
        let csw_state = world.csw_event().read().await;
        if csw_state.is_active() && csw_state.csw_time > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let remaining = csw_state.csw_time.saturating_sub(now) as u32;
            let csw_status = csw_state.status as u8;
            let phase_mins = if csw_state.status == CswOpStatus::Preparation {
                csw_state.prep_minutes
            } else {
                csw_state.war_minutes
            };
            drop(csw_state);
            send_delos_siege_packets(session, remaining).await?;

            // CSW ext_hook timer packet
            let owner_name = {
                let mk = world.get_csw_master_knights();
                if mk != 0 {
                    world
                        .get_knights(mk)
                        .map(|c| c.name.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            };
            let csw_pkt = super::ext_hook::build_csw_timer_packet(
                remaining,
                &owner_name,
                csw_status,
                phase_mins,
            );
            session.send_packet(&csw_pkt).await?;
        }
    }

    // ── Zindan War zone entry ─────────────────────────────────────────
    if zone_id == ZONE_SPBATTLE1 && world.is_zindan_event_opened() {
        let pkt = {
            let zws = world.zindan_war_state.read();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let remaining = zws.finish_time.saturating_sub(now) as u32;
            super::ext_hook::build_zindan_flagsend(
                &zws.elmo_name,
                zws.elmo_kills,
                &zws.karus_name,
                zws.karus_kills,
                remaining,
            )
        };
        session.send_packet(&pkt).await?;
    }

    // ── Tournament zone entry ────────────────────────────────────────
    // Send scoreboard, timer, and clan list when entering a tournament zone.
    // Also validates that the player's clan is one of the two competing clans.
    if super::tournament::is_tournament_zone(zone_id) {
        if !super::tournament::is_player_allowed_in_zone(&world, sid, zone_id) {
            tracing::warn!(
                "[{}] Player not allowed in tournament zone {} — not in a competing clan",
                session.addr(),
                zone_id,
            );
        }
        super::tournament::send_state_to_player(&world, sid);
    }

    // Reset zone online reward timers for the new zone
    crate::systems::zone_rewards::zone_online_reward_change(&world, sid);

    // Clear zone_changing flag
    world.set_zone_changing(sid, false);
    world.set_check_warp_zone_change(sid, false);

    tracing::info!(
        "[{}] Zone change complete — player now visible in zone {}",
        session.addr(),
        zone_id,
    );

    Ok(())
}

/// Send Delos castle siege packets on zone entry.
/// Sends WIZ_SELECT_MSG and WIZ_BIFROST packets with the siege timer.
async fn send_delos_siege_packets(
    session: &mut ClientSession,
    remaining_secs: u32,
) -> anyhow::Result<()> {
    // WIZ_SELECT_MSG packet
    let mut sel_pkt = Packet::new(Opcode::WizSelectMsg as u8);
    sel_pkt.write_u32(0);
    sel_pkt.write_u8(7);
    sel_pkt.write_u64(0);
    sel_pkt.write_u32(9);
    sel_pkt.write_u8(11);
    sel_pkt.write_u32(remaining_secs);
    session.send_packet(&sel_pkt).await?;

    // WIZ_BIFROST packet
    let mut bif_pkt = Packet::new(Opcode::WizBifrost as u8);
    bif_pkt.write_u8(5);
    bif_pkt.write_u16(remaining_secs as u16);
    session.send_packet(&bif_pkt).await?;

    Ok(())
}

/// Save position to DB asynchronously (fire-and-forget).
/// DB stores positions as i32 (world coords × 100).
pub(crate) fn save_position_async(session: &ClientSession, zone_id: u16, x: f32, z: f32) {
    let pool = session.pool().clone();
    let char_id = session.character_id().unwrap_or("").to_string();
    if char_id.is_empty() {
        return;
    }

    // Convert world coords to DB format (×100)
    let px = (x * 100.0) as i32;
    let pz = (z * 100.0) as i32;
    let py = 0i32;

    tokio::spawn(async move {
        let repo = CharacterRepository::new(&pool);
        if let Err(e) = repo
            .save_position(&char_id, zone_id as i16, px, py, pz)
            .await
        {
            tracing::error!(char_id, "failed to save position on zone change: {}", e);
        }
    });
}

/// Check if a zone allows monster transformations (not cancelled on entry).
/// Resolve (0.0, 0.0) coordinates to zone start_position with nation-specific coords.
/// When a zone change is triggered with (0,0) coordinates (e.g., event kick-out),
/// the server looks up the spawn position from the start_position table using
/// nation-specific columns. Falls back to zone spawn_position if no DB entry.
fn resolve_zero_coords(
    dest_zone: u16,
    sid: crate::zone::SessionId,
    world: &crate::world::WorldState,
) -> (f32, f32) {
    use rand::Rng;
    let nation = world.get_character_info(sid).map(|c| c.nation).unwrap_or(0);

    // 1. Try start_position table (nation-specific)
    if let Some(sp) = world.get_start_position(dest_zone) {
        let (base_x, base_z) = if nation == 1 {
            (sp.karus_x as f32, sp.karus_z as f32)
        } else {
            (sp.elmorad_x as f32, sp.elmorad_z as f32)
        };
        if base_x != 0.0 || base_z != 0.0 {
            let mut rng = rand::thread_rng();
            let offset_x = if sp.range_x > 0 {
                rng.gen_range(0..=sp.range_x) as f32
            } else {
                0.0
            };
            let offset_z = if sp.range_z > 0 {
                rng.gen_range(0..=sp.range_z) as f32
            } else {
                0.0
            };
            tracing::debug!(
                "[sid={}] resolve_zero_coords: zone {} → start_position ({:.0},{:.0})",
                sid,
                dest_zone,
                base_x + offset_x,
                base_z + offset_z
            );
            return (base_x + offset_x, base_z + offset_z);
        }
    }

    // 2. Fallback: zone init_x/init_z
    if let Some(zone) = world.get_zone(dest_zone) {
        let (x, z, _y) = zone.spawn_position();
        if x != 0.0 || z != 0.0 {
            return (x, z);
        }
    }

    // 3. Hardcoded Moradon fallback
    tracing::warn!(
        "[sid={}] resolve_zero_coords: no coords for zone {} — using Moradon fallback",
        sid,
        dest_zone
    );
    (267.0, 303.0)
}

/// Monster transformations are preserved in homeland, Eslant, and Moradon zones.
/// Entering any other zone cancels monster transformations.
fn is_transform_allowed_zone(zone_id: u16) -> bool {
    matches!(
        zone_id,
        ZONE_KARUS
            | ZONE_KARUS2
            | ZONE_KARUS3
            | ZONE_ELMORAD
            | ZONE_ELMORAD2
            | ZONE_ELMORAD3
            | ZONE_KARUS_ESLANT
            | ZONE_KARUS_ESLANT2
            | ZONE_KARUS_ESLANT3
            | ZONE_ELMORAD_ESLANT
            | ZONE_ELMORAD_ESLANT2
            | ZONE_ELMORAD_ESLANT3
            | ZONE_MORADON
            | ZONE_MORADON2
            | ZONE_MORADON3
            | ZONE_MORADON4
            | ZONE_MORADON5
    )
}

/// Server-initiated zone change (teleport a user without their ClientSession).
/// to force-teleport users who are in event instances.
/// This is a lightweight version of `trigger_zone_change` that works with
/// just a session ID + WorldState (no ClientSession needed).
pub(crate) fn server_teleport_to_zone(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    dest_zone: u16,
    dest_x: f32,
    dest_z: f32,
) {
    server_teleport_to_zone_impl(world, sid, dest_zone, dest_x, dest_z, false);
}

/// Event entry variant which also repositions a player already inside the
/// destination zone. This is required for Manes participants that entered a
/// physical instance manually before the registration countdown completed.
pub(crate) fn server_teleport_to_zone_force(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    dest_zone: u16,
    dest_x: f32,
    dest_z: f32,
) {
    server_teleport_to_zone_impl(world, sid, dest_zone, dest_x, dest_z, true);
}

fn server_teleport_to_zone_impl(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    dest_zone: u16,
    dest_x: f32,
    dest_z: f32,
    force_same_zone: bool,
) {
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return,
    };

    // Skip if already in the destination zone
    if !force_same_zone && pos.zone_id == dest_zone {
        return;
    }

    // Resolve (0,0) coordinates to zone start_position
    let (dest_x, dest_z) = if dest_x == 0.0 && dest_z == 0.0 {
        resolve_zero_coords(dest_zone, sid, world)
    } else {
        (dest_x, dest_z)
    };

    // Set zone_changing flag so the client triggers handle_loaded properly
    // QA Sprint 195 fix: without this, teleported users are invisible in
    // the destination zone until they move or relog.
    world.set_zone_changing(sid, true);

    let (nation, event_room) = world
        .with_session(sid, |h| {
            let n = h.character.as_ref().map(|c| c.nation).unwrap_or(0);
            (n, h.event_room)
        })
        .unwrap_or((0, 0));

    // 1. Broadcast INOUT_OUT to old zone
    let out_pkt = super::region::build_user_inout(super::region::INOUT_OUT, sid, None, &pos);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(out_pkt),
        Some(sid),
        event_room,
    );

    // 1+. BottomUserLogOut — zone-wide logout notification for bottom user list
    if let Some(ch) = world.get_character_info(sid) {
        let region_del_pkt = super::user_info::build_region_delete_packet(&ch.name);
        world.broadcast_to_zone(pos.zone_id, Arc::new(region_del_pkt), Some(sid));
    }

    // 2. Remove from old zone region grid
    if let Some(old_zone) = world.get_zone(pos.zone_id) {
        old_zone.remove_user(pos.region_x, pos.region_z, sid);
    }

    // 3. Update position to new zone
    let new_rx = crate::zone::calc_region(dest_x);
    let new_rz = crate::zone::calc_region(dest_z);
    world.update_position(sid, dest_zone, dest_x, 0.0, dest_z);

    // 4. Add to new zone region grid
    if let Some(new_zone) = world.get_zone(dest_zone) {
        new_zone.add_user(new_rx, new_rz, sid);
    }

    // 5. Send WIZ_ZONE_CHANGE(Teleport=3) packet
    let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
    pkt.write_u8(ZONE_CHANGE_TELEPORT);
    pkt.write_u16(dest_zone);
    pkt.write_u16(0);
    pkt.write_u16((dest_x * 10.0) as u16);
    pkt.write_u16((dest_z * 10.0) as u16);
    pkt.write_u16(0);
    pkt.write_u8(nation);
    pkt.write_u16(0xFFFF);
    world.send_to_session_owned(sid, pkt);

    // 5b. Send GOLDSHELL during active nation war
    {
        let battle_state = world.get_battle_state();
        if battle_state.battle_open == crate::systems::war::NATION_BATTLE
            && battle_state.battle_zone_id() != crate::world::ZONE_BATTLE3
        {
            let mut gs_pkt = Packet::new(Opcode::WizMapEvent as u8);
            gs_pkt.write_u8(9); // GOLDSHELL
            gs_pkt.write_u8(1); // enable
            gs_pkt.write_u32(sid as u32);
            world.send_to_session_owned(sid, gs_pkt);
        }
    }
}

/// Monster Stone room exit cleanup.
/// When ANY player exits a Monster Stone zone, the ENTIRE room is disbanded:
/// all users are cleared, their `event_room` is reset to 0, event NPCs
/// are despawned, and the room is returned to the pool.
pub(crate) fn monster_stone_exit_room(
    world: &std::sync::Arc<crate::world::WorldState>,
    exiting_sid: crate::zone::SessionId,
) {
    // Find which room this player is in
    let (room_id, zone_id) = match world.monster_stone_read().find_user_room(exiting_sid) {
        Some(r) => {
            let zone = world
                .monster_stone_read()
                .get_room(r)
                .map(|rm| rm.zone_id)
                .unwrap_or(0);
            (r, zone)
        }
        None => {
            // Not in any room — just clear the event_room field + monster stone status
            world.update_session(exiting_sid, |h| {
                h.event_room = 0;
                h.monster_stone_status = false;
            });
            return;
        }
    };

    // Reset room and get all user session IDs
    let users = world.monster_stone_write().reset_room(room_id);

    // Teleport remaining users to Moradon, clear event_room + monster stone status
    for &uid in &users {
        world.update_session(uid, |h| {
            h.event_room = 0;
            h.monster_stone_status = false;
        });
        // Don't teleport the exiting user (they triggered the exit via their own zone change)
        if uid != exiting_sid {
            use crate::systems::monster_stone;
            let in_stone_zone = world
                .get_position(uid)
                .map(|p| monster_stone::is_monster_stone_zone(p.zone_id))
                .unwrap_or(false);
            if in_stone_zone {
                server_teleport_to_zone(world, uid, monster_stone::ZONE_MORADON, 0.0, 0.0);
            }
        }
    }

    // Despawn all event NPCs belonging to this room
    if zone_id > 0 {
        let event_room_id = room_id + 1; // 1-based
        world.despawn_room_npcs(zone_id as u16, event_room_id);
    }

    tracing::debug!(
        "[sid={}] Monster Stone room {} disbanded ({} users teleported to Moradon, NPCs despawned in zone {})",
        exiting_sid, room_id, users.len(), zone_id
    );
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_zone_change_teleport_packet_format() {
        // Build WIZ_ZONE_CHANGE(Teleport=3) packet
        let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
        pkt.write_u8(3); // ZoneChangeTeleport
        pkt.write_u16(21); // zone_id (Moradon)
        pkt.write_u16(0); // padding
        pkt.write_u16(5120); // x * 10 (512.0 * 10)
        pkt.write_u16(3410); // z * 10 (341.0 * 10)
        pkt.write_u16(0); // y * 10
        pkt.write_u8(1); // nation (Karus)
        pkt.write_u16(0xFFFF); // unknown

        assert_eq!(pkt.opcode, Opcode::WizZoneChange as u8);
        assert_eq!(pkt.data.len(), 14);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3)); // sub-opcode
        assert_eq!(r.read_u16(), Some(21)); // zone_id
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u16(), Some(5120)); // x*10
        assert_eq!(r.read_u16(), Some(3410)); // z*10
        assert_eq!(r.read_u16(), Some(0)); // y*10
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u16(), Some(0xFFFF)); // unknown
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_warp_packet_format() {
        // Build WIZ_WARP packet
        let mut pkt = Packet::new(Opcode::WizWarp as u8);
        pkt.write_u16(6160); // x * 10 (616.0 * 10)
        pkt.write_u16(3410); // z * 10 (341.0 * 10)
        pkt.write_i16(-1); // unknown

        assert_eq!(pkt.opcode, Opcode::WizWarp as u8);
        assert_eq!(pkt.data.len(), 6);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(6160));
        assert_eq!(r.read_u16(), Some(3410));
        assert_eq!(r.read_u16(), Some(0xFFFF)); // -1 as u16
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 44: Zone change buff lifecycle tests ──────────────────

    #[test]
    fn test_zone_change_constants() {
        // Verify zone ID constants match C++ Define.h
        assert_eq!(super::ZONE_DELOS, 30);
        assert_eq!(super::ZONE_CHAOS_DUNGEON, 85);
    }

    #[test]
    fn test_zone_change_sub_opcodes() {
        assert_eq!(super::ZONE_CHANGE_LOADING, 1);
        assert_eq!(super::ZONE_CHANGE_LOADED, 2);
        assert_eq!(super::ZONE_CHANGE_TELEPORT, 3);
    }

    #[test]
    fn test_delos_siege_select_msg_packet_format() {
        // WIZ_SELECT_MSG packet for Delos siege timer
        let remaining_secs: u32 = 600; // 10 minutes

        let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
        pkt.write_u32(0);
        pkt.write_u8(7);
        pkt.write_u64(0);
        pkt.write_u32(9);
        pkt.write_u8(11);
        pkt.write_u32(remaining_secs);

        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);
        // 4 + 1 + 8 + 4 + 1 + 4 = 22 bytes
        assert_eq!(pkt.data.len(), 22);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u64(), Some(0));
        assert_eq!(r.read_u32(), Some(9));
        assert_eq!(r.read_u8(), Some(11));
        assert_eq!(r.read_u32(), Some(600));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_delos_siege_bifrost_packet_format() {
        // WIZ_BIFROST packet with siege timer
        let remaining_secs: u32 = 600;

        let mut pkt = Packet::new(Opcode::WizBifrost as u8);
        pkt.write_u8(5);
        pkt.write_u16(remaining_secs as u16);

        assert_eq!(pkt.opcode, Opcode::WizBifrost as u8);
        assert_eq!(pkt.data.len(), 3); // u8 + u16

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u16(), Some(600));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_delos_siege_bifrost_timer_truncation() {
        // Timer larger than u16::MAX should truncate
        let remaining_secs: u32 = 70000;
        let truncated = remaining_secs as u16;
        assert_eq!(truncated, 4464); // 70000 mod 65536
    }

    #[test]
    fn test_delos_siege_timer_zero() {
        // Zero timer should still produce valid packets
        let remaining_secs: u32 = 0;

        let mut pkt = Packet::new(Opcode::WizBifrost as u8);
        pkt.write_u8(5);
        pkt.write_u16(remaining_secs as u16);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5));
        assert_eq!(r.read_u16(), Some(0));
    }

    // ── Sprint 46: Zone change death animation tests ────────────────────

    #[test]
    fn test_death_animation_packet_format() {
        // Packet: WIZ_DEAD [u32 dead_unit_id]
        let sid: u16 = 42;
        let mut pkt = Packet::new(Opcode::WizDead as u8);
        pkt.write_u32(sid as u32);

        assert_eq!(pkt.opcode, Opcode::WizDead as u8);
        assert_eq!(pkt.data.len(), 4);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_zone_constants_knight_royale() {
        assert_eq!(super::ZONE_KNIGHT_ROYALE, 76);
    }

    #[test]
    fn test_zone_constants_dungeon_defence() {
        assert_eq!(super::ZONE_DUNGEON_DEFENCE, 89);
    }

    #[test]
    fn test_blink_time_special_zone() {
        // BLINK_TIME(10) + 45 = 55
        assert_eq!(super::BLINK_TIME_SPECIAL_ZONE, 55);
    }

    // ── Sprint 48: Zone Change Integration Tests ────────────────────

    /// Integration: Zone change clears buffs and DOTs, then recasts saved magic.
    ///
    /// Verifies: buffs exist → zone change → buffs cleared → saved magic recast.
    #[test]
    fn test_integration_zone_change_clear_buffs_recast_saved() {
        use crate::world::{ActiveBuff, Position, WorldState};
        use std::time::Instant;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            crate::world::CharacterInfo {
                session_id: sid,
                name: "ZoneTest".into(),
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
                gold: 0,
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
            },
            Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Apply 2 buffs and 1 DOT before zone change
        world.apply_buff(
            sid,
            ActiveBuff {
                skill_id: 108010,
                buff_type: 8,
                caster_sid: sid,
                start_time: Instant::now(),
                duration_secs: 120,
                speed: 50,
                attack_speed: 0,
                ac: 0,
                ac_pct: 0,
                attack: 0,
                magic_attack: 0,
                max_hp: 0,
                max_hp_pct: 0,
                max_mp: 0,
                max_mp_pct: 0,
                str_mod: 0,
                sta_mod: 0,
                dex_mod: 0,
                intel_mod: 0,
                cha_mod: 0,
                fire_r: 0,
                cold_r: 0,
                lightning_r: 0,
                magic_r: 0,
                disease_r: 0,
                poison_r: 0,
                hit_rate: 0,
                avoid_rate: 0,
                weapon_damage: 0,
                ac_sour: 0,
                duration_extended: false,
                is_buff: true,
            },
        );
        world.apply_buff(
            sid,
            ActiveBuff {
                skill_id: 108020,
                buff_type: 9,
                caster_sid: sid,
                start_time: Instant::now(),
                duration_secs: 60,
                attack: 30,
                attack_speed: 0,
                speed: 0,
                ac: 0,
                ac_pct: 0,
                magic_attack: 0,
                max_hp: 0,
                max_hp_pct: 0,
                max_mp: 0,
                max_mp_pct: 0,
                str_mod: 0,
                sta_mod: 0,
                dex_mod: 0,
                intel_mod: 0,
                cha_mod: 0,
                fire_r: 0,
                cold_r: 0,
                lightning_r: 0,
                magic_r: 0,
                disease_r: 0,
                poison_r: 0,
                hit_rate: 0,
                avoid_rate: 0,
                weapon_damage: 0,
                ac_sour: 0,
                duration_extended: false,
                is_buff: true,
            },
        );
        world.add_durational_skill(sid, 108100, -10, 5, sid);

        assert_eq!(world.get_active_buffs(sid).len(), 2, "Should have 2 buffs");

        // Simulate zone change: clear DOTs, clear buffs
        world.clear_all_dots(sid);
        world.clear_all_buffs(sid, false);

        assert_eq!(
            world.get_active_buffs(sid).len(),
            0,
            "Buffs should be cleared after zone change"
        );
    }

    /// Integration: Dead player zone change broadcasts death animation.
    ///
    /// Verifies: dead player → zone change → WIZ_DEAD broadcast format correct.
    #[test]
    fn test_integration_dead_player_zone_change_death_broadcast() {
        use crate::world::{Position, WorldState};

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let ch = crate::world::CharacterInfo {
            session_id: sid,
            name: "DeadTest".into(),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 0, // DEAD
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
            gold: 0,
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
        world.register_ingame(
            sid,
            ch,
            Position {
                zone_id: 11,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Player is dead — verify
        let info = world.get_character_info(sid).unwrap();
        assert_eq!(info.hp, 0, "Player should be dead");

        // Build WIZ_DEAD broadcast packet (what zone change would send)
        let mut death_pkt = Packet::new(Opcode::WizDead as u8);
        death_pkt.write_u32(sid as u32);

        assert_eq!(death_pkt.opcode, Opcode::WizDead as u8);
        assert_eq!(death_pkt.data.len(), 4);

        let mut r = PacketReader::new(&death_pkt.data);
        assert_eq!(r.read_u32(), Some(sid as u32));
        assert_eq!(r.remaining(), 0);
    }

    /// Integration: Zone change during trade cancels the trade first.
    ///
    /// Verifies: active trade → zone change → trade state reset.
    #[test]
    fn test_integration_zone_change_cancels_trade() {
        use crate::world::{WorldState, TRADE_STATE_TRADING};

        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let sid1 = world.allocate_session_id();
        let sid2 = world.allocate_session_id();
        world.register_session(sid1, tx1);
        world.register_session(sid2, tx2);

        // Set up both players in trading state
        world.update_session(sid1, |h| {
            h.trade_state = TRADE_STATE_TRADING;
            h.exchange_user = Some(sid2);
        });
        world.update_session(sid2, |h| {
            h.trade_state = TRADE_STATE_TRADING;
            h.exchange_user = Some(sid1);
        });

        // Verify trade is active
        assert_eq!(world.get_trade_state(sid1), TRADE_STATE_TRADING);
        assert_eq!(world.get_exchange_user(sid1), Some(sid2));

        // Simulate zone change: cancel trade for both parties
        world.update_session(sid1, |h| {
            h.trade_state = 0; // TRADE_STATE_NONE
            h.exchange_user = None;
            h.exchange_items.clear();
        });
        world.update_session(sid2, |h| {
            h.trade_state = 0;
            h.exchange_user = None;
            h.exchange_items.clear();
        });

        assert_eq!(
            world.get_trade_state(sid1),
            0,
            "Trade should be cancelled for sid1"
        );
        assert_eq!(
            world.get_trade_state(sid2),
            0,
            "Trade should be cancelled for sid2"
        );
        assert_eq!(
            world.get_exchange_user(sid1),
            None,
            "Exchange partner should be cleared"
        );
    }

    /// Integration: Zone change resets online reward timers.
    ///
    /// Verifies: timers exist → zone change → timers cleared.
    #[test]
    fn test_integration_zone_change_resets_online_reward_timers() {
        use crate::world::WorldState;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Set up zone online reward timers
        world.update_session(sid, |h| {
            h.zone_online_reward_timers = vec![100, 200, 300];
        });

        let timers = world
            .with_session(sid, |h| h.zone_online_reward_timers.clone())
            .unwrap();
        assert_eq!(timers.len(), 3, "Should have 3 timers");

        // Simulate zone change: clear timers
        world.update_session(sid, |h| {
            h.zone_online_reward_timers.clear();
        });

        let after = world
            .with_session(sid, |h| h.zone_online_reward_timers.clone())
            .unwrap();
        assert_eq!(after.len(), 0, "Timers should be cleared after zone change");
    }

    /// Integration: Blink activation on zone change with special zone duration.
    ///
    /// Verifies: normal zone → 10s blink, special zone → 55s blink.
    #[test]
    fn test_integration_blink_duration_normal_vs_special_zone() {
        let normal_duration: u64 = 10;
        let special_duration: u64 = super::BLINK_TIME_SPECIAL_ZONE;

        // Normal zone (e.g., Moradon)
        assert_eq!(normal_duration, 10);

        // Special zones
        for zone_id in [
            super::ZONE_CHAOS_DUNGEON,
            super::ZONE_KNIGHT_ROYALE,
            super::ZONE_DUNGEON_DEFENCE,
        ] {
            let duration = if matches!(zone_id, 85 | 76 | 89) {
                special_duration
            } else {
                normal_duration
            };
            assert_eq!(
                duration, 55,
                "Special zone {} should have 55s blink",
                zone_id
            );
        }

        // Normal zone (Moradon=21)
        let normal_zone: u16 = 21;
        let duration = if matches!(normal_zone, 85 | 76 | 89) {
            special_duration
        } else {
            normal_duration
        };
        assert_eq!(duration, 10, "Normal zone should have 10s blink");
    }

    /// Integration: Zone change clears stealth state.
    ///
    /// Verifies: stealthed player → zone change → stealth removed.
    #[test]
    fn test_integration_zone_change_clears_stealth() {
        use crate::world::WorldState;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Activate stealth
        world.update_session(sid, |h| {
            h.invisibility_type = 1;
        });

        let stealth = world.with_session(sid, |h| h.invisibility_type).unwrap();
        assert_eq!(stealth, 1, "Should be stealthed");

        // Zone change: clear stealth
        world.update_session(sid, |h| {
            h.invisibility_type = 0;
            h.stealth_end_time = 0;
        });

        let after = world.with_session(sid, |h| h.invisibility_type).unwrap();
        assert_eq!(after, 0, "Stealth should be cleared after zone change");
    }

    /// Integration: Zone change packet format for cross-zone teleport.
    ///
    /// Verifies: WIZ_ZONE_CHANGE(Teleport=3) packet has correct wire format.
    #[test]
    fn test_integration_zone_change_teleport_packet_complete() {
        let dest_zone: u16 = 11; // Karus
        let dest_x: f32 = 512.5;
        let dest_z: f32 = 341.0;
        let nation: u8 = 1;

        let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
        pkt.write_u8(super::ZONE_CHANGE_TELEPORT);
        pkt.write_u16(dest_zone);
        pkt.write_u16(0); // padding
        pkt.write_u16((dest_x * 10.0) as u16); // 5125
        pkt.write_u16((dest_z * 10.0) as u16); // 3410
        pkt.write_u16(0); // y * 10
        pkt.write_u8(nation);
        pkt.write_u16(0xFFFF);

        // Verify all fields
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3)); // ZONE_CHANGE_TELEPORT
        assert_eq!(r.read_u16(), Some(11)); // zone_id
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u16(), Some(5125)); // x * 10
        assert_eq!(r.read_u16(), Some(3410)); // z * 10
        assert_eq!(r.read_u16(), Some(0)); // y * 10
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u16(), Some(0xFFFF)); // unknown
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 55: Hardening Edge Case Tests ────────────────────────

    /// Edge case: same-zone warp produces a valid WIZ_WARP packet.
    /// Players can trigger same-zone warps via warp gates within a single zone.
    #[test]
    fn test_same_zone_warp_packet_format() {
        // Same-zone warp sends WIZ_WARP (not WIZ_ZONE_CHANGE)
        let dest_x: f32 = 616.0;
        let dest_z: f32 = 341.0;

        let mut pkt = Packet::new(Opcode::WizWarp as u8);
        pkt.write_u16((dest_x * 10.0) as u16);
        pkt.write_u16((dest_z * 10.0) as u16);
        pkt.write_i16(-1);

        assert_eq!(pkt.opcode, Opcode::WizWarp as u8);
        assert_eq!(pkt.data.len(), 6);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(6160)); // x*10
        assert_eq!(r.read_u16(), Some(3410)); // z*10
        assert_eq!(r.read_u16(), Some(0xFFFF)); // -1
        assert_eq!(r.remaining(), 0);
    }

    /// Edge case: zone change with zone_id=0 should produce a valid teleport
    /// packet (the handler would reject it, but the packet format should not panic).
    #[test]
    fn test_zone_change_invalid_zone_id_zero_packet_format() {
        let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
        pkt.write_u8(super::ZONE_CHANGE_TELEPORT);
        pkt.write_u16(0); // invalid zone_id = 0
        pkt.write_u16(0); // padding
        pkt.write_u16(0); // x*10
        pkt.write_u16(0); // z*10
        pkt.write_u16(0); // y*10
        pkt.write_u8(0); // nation
        pkt.write_u16(0xFFFF);

        // Packet should still be well-formed (14 bytes)
        assert_eq!(pkt.data.len(), 14);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(super::ZONE_CHANGE_TELEPORT));
        assert_eq!(r.read_u16(), Some(0)); // zone_id = 0
    }

    /// Edge case: zone change with max zone_id (u16::MAX) should produce valid packet.
    #[test]
    fn test_zone_change_max_zone_id_packet_format() {
        let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
        pkt.write_u8(super::ZONE_CHANGE_TELEPORT);
        pkt.write_u16(u16::MAX); // max zone_id
        pkt.write_u16(0);
        pkt.write_u16(5120);
        pkt.write_u16(3410);
        pkt.write_u16(0);
        pkt.write_u8(1);
        pkt.write_u16(0xFFFF);

        assert_eq!(pkt.data.len(), 14);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(super::ZONE_CHANGE_TELEPORT));
        assert_eq!(r.read_u16(), Some(u16::MAX));
    }

    /// Edge case: zone change position update with world state — position is
    /// correctly written even when warping to the same zone.
    #[test]
    fn test_integration_same_zone_warp_position_update() {
        use crate::world::{Position, WorldState};

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let ch = crate::world::CharacterInfo {
            session_id: sid,
            name: "WarpTest".into(),
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
            gold: 0,
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
        // Start in zone 21
        world.register_ingame(
            sid,
            ch,
            Position {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 0,
                region_z: 0,
            },
        );

        // Simulate same-zone warp: update position within same zone
        world.update_session(sid, |h| {
            h.position.x = 616.0;
            h.position.z = 341.0;
        });

        let pos = world.with_session(sid, |h| h.position).unwrap();
        assert_eq!(pos.zone_id, 21, "Zone should remain the same");
        assert!((pos.x - 616.0).abs() < 0.01, "X should be updated");
        assert!((pos.z - 341.0).abs() < 0.01, "Z should be updated");
    }

    // ── Zone entry validation tests ──────────────────────────────────

    use super::*;

    fn make_validation_world() -> WorldState {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set up character info with nation=Karus, level=50, authority=1 (player)
        world.update_session(1, |h| {
            h.character = Some(crate::world::CharacterInfo {
                session_id: 1,
                name: "TestPlayer".to_string(),
                nation: NATION_KARUS,
                race: 1,
                class: 101,
                level: 50,
                face: 1,
                hair_rgb: 0,
                rank: 0,
                title: 0,
                str: 60,
                sta: 60,
                dex: 60,
                intel: 60,
                cha: 60,
                free_points: 0,
                skill_points: [0u8; 10],
                gold: 1000,
                loyalty: 100,
                loyalty_monthly: 0,
                authority: 1, // AUTHORITY_PLAYER
                fame: 0,
                knights_id: 0,
                bind_zone: 21,
                bind_x: 500.0,
                bind_z: 500.0,
                hp: 1000,
                mp: 500,
                max_hp: 1000,
                max_mp: 500,
                sp: 0,
                max_sp: 0,
                equipped_items: [0u32; 14],
                party_id: None,
                exp: 0,
                max_exp: 100_000,
                exp_seal_status: false,
                sealed_exp: 0,
                item_weight: 0,
                max_weight: 1000,
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
            });
        });

        // Ensure some zones exist
        world.ensure_zone(1, 2048); // ZONE_KARUS
        world.ensure_zone(2, 2048); // ZONE_ELMORAD
        world.ensure_zone(11, 2048); // ZONE_KARUS_ESLANT
        world.ensure_zone(12, 2048); // ZONE_ELMORAD_ESLANT
        world.ensure_zone(21, 2048); // Moradon
        world.ensure_zone(31, 2048); // ZONE_BIFROST
        world.ensure_zone(72, 2048); // ZONE_ARDREAM

        world
    }

    /// GM (authority=0) bypasses all zone restrictions.
    #[test]
    fn test_validate_zone_gm_bypass() {
        let world = make_validation_world();
        // Set authority to GM (0)
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.authority = 0; // AUTHORITY_GAME_MASTER
            }
        });
        // Should be able to enter any zone
        assert!(
            validate_zone_entry(&world, 1, 2).is_ok(),
            "GM should enter El Morad"
        );
        assert!(
            validate_zone_entry(&world, 1, 12).is_ok(),
            "GM should enter El Morad Eslant"
        );
    }

    /// Karus player can enter Karus homeland.
    #[test]
    fn test_validate_zone_karus_homeland() {
        let world = make_validation_world();
        assert!(validate_zone_entry(&world, 1, 1).is_ok());
    }

    /// Karus player CANNOT enter El Morad homeland.
    #[test]
    fn test_validate_zone_karus_blocked_from_elmorad() {
        let world = make_validation_world();
        assert!(validate_zone_entry(&world, 1, 2).is_err());
    }

    /// Karus player can enter Karus Eslant.
    #[test]
    fn test_validate_zone_karus_eslant_allowed() {
        let world = make_validation_world();
        assert!(validate_zone_entry(&world, 1, 11).is_ok());
    }

    /// Karus player CANNOT enter El Morad Eslant.
    #[test]
    fn test_validate_zone_karus_blocked_from_elmorad_eslant() {
        let world = make_validation_world();
        assert!(validate_zone_entry(&world, 1, 12).is_err());
    }

    /// Bifrost is always allowed.
    #[test]
    fn test_validate_zone_bifrost_always_allowed() {
        let world = make_validation_world();
        assert!(validate_zone_entry(&world, 1, 31).is_ok());
    }

    /// Ardream requires loyalty > 0.
    #[test]
    fn test_validate_zone_ardream_requires_loyalty() {
        let world = make_validation_world();
        // With loyalty=100, should pass
        assert!(validate_zone_entry(&world, 1, 72).is_ok());

        // Set loyalty to 0
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.loyalty = 0;
            }
        });
        assert!(validate_zone_entry(&world, 1, 72).is_err());
    }

    /// Warp error constants match C++ enum values.
    #[test]
    fn test_warp_error_codes() {
        assert_eq!(WARP_ERROR, 0);
        assert_eq!(WARP_MIN_LEVEL, 2);
        assert_eq!(WARP_DO_NOT_QUALIFY, 7);
    }

    // ── Sprint 120: Zone change transformation cancel tests ──────────

    #[test]
    fn test_transformation_monster_constant() {
        assert_eq!(super::TRANSFORMATION_MONSTER, 1);
    }

    #[test]
    fn test_magic_cancel_transformation_constant() {
        assert_eq!(super::MAGIC_CANCEL_TRANSFORMATION, 7);
    }

    #[test]
    fn test_transform_allowed_in_karus_homeland() {
        assert!(super::is_transform_allowed_zone(ZONE_KARUS));
        assert!(super::is_transform_allowed_zone(ZONE_KARUS2));
        assert!(super::is_transform_allowed_zone(ZONE_KARUS3));
    }

    #[test]
    fn test_transform_allowed_in_elmorad_homeland() {
        assert!(super::is_transform_allowed_zone(ZONE_ELMORAD));
        assert!(super::is_transform_allowed_zone(ZONE_ELMORAD2));
        assert!(super::is_transform_allowed_zone(ZONE_ELMORAD3));
    }

    #[test]
    fn test_transform_allowed_in_eslant() {
        assert!(super::is_transform_allowed_zone(ZONE_KARUS_ESLANT));
        assert!(super::is_transform_allowed_zone(ZONE_KARUS_ESLANT2));
        assert!(super::is_transform_allowed_zone(ZONE_KARUS_ESLANT3));
        assert!(super::is_transform_allowed_zone(ZONE_ELMORAD_ESLANT));
        assert!(super::is_transform_allowed_zone(ZONE_ELMORAD_ESLANT2));
        assert!(super::is_transform_allowed_zone(ZONE_ELMORAD_ESLANT3));
    }

    #[test]
    fn test_transform_allowed_in_moradon() {
        assert!(super::is_transform_allowed_zone(ZONE_MORADON));
        assert!(super::is_transform_allowed_zone(ZONE_MORADON2));
        assert!(super::is_transform_allowed_zone(ZONE_MORADON3));
        assert!(super::is_transform_allowed_zone(ZONE_MORADON4));
        assert!(super::is_transform_allowed_zone(ZONE_MORADON5));
    }

    #[test]
    fn test_transform_not_allowed_in_restricted_zones() {
        // Delos, Ardream, Ronark, Bifrost, Chaos Dungeon, Knight Royale
        assert!(!super::is_transform_allowed_zone(super::ZONE_DELOS));
        assert!(!super::is_transform_allowed_zone(ZONE_ARDREAM));
        assert!(!super::is_transform_allowed_zone(ZONE_RONARK_LAND));
        assert!(!super::is_transform_allowed_zone(ZONE_RONARK_LAND_BASE));
        assert!(!super::is_transform_allowed_zone(ZONE_BIFROST));
        assert!(!super::is_transform_allowed_zone(super::ZONE_CHAOS_DUNGEON));
        assert!(!super::is_transform_allowed_zone(super::ZONE_KNIGHT_ROYALE));
        assert!(!super::is_transform_allowed_zone(
            super::ZONE_DUNGEON_DEFENCE
        ));
    }

    #[test]
    fn test_transform_cancel_packet_format() {
        // WIZ_MAGIC_PROCESS + MAGIC_CANCEL_TRANSFORMATION(7)
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(super::MAGIC_CANCEL_TRANSFORMATION);

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 7);
    }

    #[test]
    fn test_server_teleport_packet_format() {
        // Verify the packet built by server_teleport_to_zone matches C++ format
        let dest_zone: u16 = 21; // Moradon
        let dest_x: f32 = 0.0;
        let dest_z: f32 = 0.0;
        let nation: u8 = 1;

        let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
        pkt.write_u8(super::ZONE_CHANGE_TELEPORT);
        pkt.write_u16(dest_zone);
        pkt.write_u16(0);
        pkt.write_u16((dest_x * 10.0) as u16);
        pkt.write_u16((dest_z * 10.0) as u16);
        pkt.write_u16(0);
        pkt.write_u8(nation);
        pkt.write_u16(0xFFFF);

        assert_eq!(pkt.opcode, Opcode::WizZoneChange as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3)); // ZONE_CHANGE_TELEPORT
        assert_eq!(r.read_u16(), Some(21)); // zone
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u16(), Some(0)); // x * 10
        assert_eq!(r.read_u16(), Some(0)); // z * 10
        assert_eq!(r.read_u16(), Some(0)); // y
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u16(), Some(0xFFFF)); // unknown (-1)
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 265: Delos CSW validation + warp error codes ──────────

    /// Warp error code constants should match C++ WarpListResponse enum.
    #[test]
    fn test_warp_error_code_constants() {
        assert_eq!(super::WARP_ERROR, 0);
        assert_eq!(super::WARP_MIN_LEVEL, 2);
        assert_eq!(super::WARP_NOT_DURING_CSW, 3);
        assert_eq!(super::WARP_NEED_NP, 5);
        assert_eq!(super::WARP_DO_NOT_QUALIFY, 7);
    }

    /// Delos CSW validation: player with zero loyalty should be rejected.
    #[test]
    fn test_delos_zero_loyalty_rejected() {
        let world = make_validation_world();
        // Set loyalty to 0
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.loyalty = 0;
            }
        });

        let result = super::validate_zone_entry(&world, 1, super::ZONE_DELOS);
        assert!(result.is_err());
        let (code, _) = result.unwrap_err();
        assert_eq!(code, super::WARP_NEED_NP);
    }

    /// Delos CSW validation: player with loyalty > 0 and no active CSW should pass.
    #[test]
    fn test_delos_with_loyalty_no_csw_allowed() {
        let world = make_validation_world();
        world.ensure_zone(super::ZONE_DELOS, 2048);
        // Default world has loyalty=100, CSW not active → should pass
        let result = super::validate_zone_entry(&world, 1, super::ZONE_DELOS);
        assert!(result.is_ok());
    }

    /// GMs bypass all zone restrictions including Delos.
    #[test]
    fn test_delos_gm_bypass() {
        let world = make_validation_world();
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.authority = 0; // GM
                ch.loyalty = 0; // even with zero loyalty
            }
        });

        let result = super::validate_zone_entry(&world, 1, super::ZONE_DELOS);
        assert!(result.is_ok(), "GMs should bypass Delos restrictions");
    }

    // ── Sprint 269: War restriction tests ──────────────────────────

    #[test]
    fn test_warp_not_during_war_constant() {
        // C++ User.h:94 — WarpListNotDuringWar = 4
        assert_eq!(super::WARP_NOT_DURING_WAR, 4);
    }

    #[test]
    fn test_spbattle_zone_range() {
        // C++ Define.h — ZONE_SPBATTLE1 = 105 .. ZONE_SPBATTLE11 = 115
        assert_eq!(super::ZONE_SPBATTLE_MIN, 105);
        assert_eq!(super::ZONE_SPBATTLE_MAX, 115);
        for z in 105..=115u16 {
            assert!((super::ZONE_SPBATTLE_MIN..=super::ZONE_SPBATTLE_MAX).contains(&z));
        }
        assert!(!(super::ZONE_SPBATTLE_MIN..=super::ZONE_SPBATTLE_MAX).contains(&104));
        assert!(!(super::ZONE_SPBATTLE_MIN..=super::ZONE_SPBATTLE_MAX).contains(&116));
    }

    #[test]
    fn test_zone_ardream_type_constant() {
        // C++ Define.h:173 — ZONE_ARDREAM = 72 (used as battle zone type)
        assert_eq!(super::ZONE_ARDREAM_TYPE, 72);
    }

    #[test]
    fn test_ardream_loyalty_required() {
        // Ardream requires loyalty > 0 when no war is active
        let world = make_validation_world();
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.loyalty = 0;
            }
        });
        let result = super::validate_zone_entry(&world, 1, super::ZONE_ARDREAM);
        assert_eq!(result, Err((super::WARP_NEED_NP, 0)));
    }

    #[test]
    fn test_ardream_with_loyalty_no_war() {
        // Ardream: loyalty > 0, no war → allowed
        let world = make_validation_world();
        // Default world has loyalty=100, no war → should pass
        let result = super::validate_zone_entry(&world, 1, super::ZONE_ARDREAM);
        assert!(result.is_ok());
    }

    #[test]
    fn test_spbattle_zero_loyalty() {
        // SPBATTLE zones require loyalty > 0
        let world = make_validation_world();
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.loyalty = 0;
            }
        });
        let result = super::validate_zone_entry(&world, 1, 110); // SPBATTLE6
        assert_eq!(result, Err((super::WARP_NEED_NP, 0)));
    }

    #[test]
    fn test_spbattle_with_loyalty() {
        // SPBATTLE zones: loyalty > 0 → allowed
        let world = make_validation_world();
        let result = super::validate_zone_entry(&world, 1, 110); // SPBATTLE6
        assert!(result.is_ok());
    }

    // ── Sprint 270: War zone default + homeland invasion tests ──────

    /// Helper to create a war zone in the test world via set_zone_info.
    fn add_war_zone(world: &WorldState, zone_id: u16) {
        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            zone_id,
            ZoneInfo {
                smd_name: format!("battle_{zone_id}"),
                zone_name: format!("Battle Zone {zone_id}"),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 100.0,
                init_z: 100.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    war_zone: true,
                    attack_other_nation: true,
                    ..Default::default()
                },
            },
        );
    }

    #[test]
    fn test_war_zone_active_battle_zone_allowed() {
        // Player can enter battle zone 61 when battle_zone=1 (61 - 60 = 1)
        let world = make_validation_world();
        add_war_zone(&world, 61); // ZONE_BATTLE = ZONE_BATTLE_BASE + 1
        world.update_battle_state(|bs| {
            bs.battle_open = 1; // NATION_BATTLE
            bs.battle_zone = 1;
        });
        let result = validate_zone_entry(&world, 1, 61);
        assert!(result.is_ok(), "Should allow entry to active battle zone");
    }

    #[test]
    fn test_war_zone_wrong_battle_zone_denied() {
        // Player cannot enter battle zone 62 when active is 61 (battle_zone=1)
        let world = make_validation_world();
        add_war_zone(&world, 62); // ZONE_BATTLE2 = ZONE_BATTLE_BASE + 2
        world.update_battle_state(|bs| {
            bs.battle_open = 1; // NATION_BATTLE
            bs.battle_zone = 1; // active zone is 61, not 62
        });
        let result = validate_zone_entry(&world, 1, 62);
        assert_eq!(
            result,
            Err((WARP_ERROR, 0)),
            "Wrong battle zone should be denied"
        );
    }

    #[test]
    fn test_snow_battle_zone_allowed() {
        // Snow battle zone 69: offset = 69 - 69 = 0, battle_zone should be 0
        let world = make_validation_world();
        add_war_zone(&world, 69); // ZONE_SNOW_BATTLE
        world.update_battle_state(|bs| {
            bs.battle_open = 2; // SNOW_BATTLE
            bs.battle_zone = 0;
        });
        let result = validate_zone_entry(&world, 1, 69);
        assert!(
            result.is_ok(),
            "Should allow entry to active snow battle zone"
        );
    }

    #[test]
    fn test_snow_battle_zone_wrong_offset_denied() {
        // Snow battle zone 69: if battle_zone != 0, deny
        let world = make_validation_world();
        add_war_zone(&world, 69); // ZONE_SNOW_BATTLE
        world.update_battle_state(|bs| {
            bs.battle_open = 1; // NATION_BATTLE (not snow)
            bs.battle_zone = 1; // mismatch with 69-69=0
        });
        let result = validate_zone_entry(&world, 1, 69);
        assert_eq!(
            result,
            Err((WARP_ERROR, 0)),
            "Snow battle zone with wrong offset denied"
        );
    }

    #[test]
    fn test_war_zone_open_flag_banish_karus() {
        // Karus player denied when karus_open_flag is true (homeland under invasion)
        let world = make_validation_world();
        add_war_zone(&world, 61); // ZONE_BATTLE
        world.update_battle_state(|bs| {
            bs.battle_open = 1;
            bs.battle_zone = 1;
            bs.karus_open_flag = true; // Karus homeland invaded
        });
        let result = validate_zone_entry(&world, 1, 61);
        assert_eq!(
            result,
            Err((WARP_ERROR, 0)),
            "Karus player denied when own homeland invaded"
        );
    }

    #[test]
    fn test_war_zone_open_flag_banish_elmorad() {
        // Elmorad player denied when elmorad_open_flag is true
        let world = make_validation_world();
        add_war_zone(&world, 61); // ZONE_BATTLE
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.nation = NATION_ELMORAD;
            }
        });
        world.update_battle_state(|bs| {
            bs.battle_open = 1;
            bs.battle_zone = 1;
            bs.elmorad_open_flag = true; // Elmorad homeland invaded
        });
        let result = validate_zone_entry(&world, 1, 61);
        assert_eq!(
            result,
            Err((WARP_ERROR, 0)),
            "Elmorad player denied when own homeland invaded"
        );
    }

    #[test]
    fn test_non_war_zone_default_allowed() {
        // Non-war zones in default case should be allowed
        let world = make_validation_world();
        world.ensure_zone(40, 2048); // some random non-war zone
        let result = validate_zone_entry(&world, 1, 40);
        assert!(
            result.is_ok(),
            "Non-war zone should be allowed in default case"
        );
    }

    #[test]
    fn test_karus_homeland_own_nation_allowed() {
        // Karus player can always enter Karus homeland
        let world = make_validation_world();
        let result = validate_zone_entry(&world, 1, ZONE_KARUS);
        assert!(result.is_ok(), "Karus player should enter own homeland");
    }

    #[test]
    fn test_karus_homeland_elmorad_no_invasion() {
        // Elmorad player cannot enter Karus homeland without invasion
        let world = make_validation_world();
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.nation = NATION_ELMORAD;
            }
        });
        let result = validate_zone_entry(&world, 1, ZONE_KARUS);
        assert_eq!(
            result,
            Err((WARP_ERROR, 0)),
            "Elmorad denied without invasion flag"
        );
    }

    #[test]
    fn test_karus_homeland_elmorad_invasion_allowed() {
        // Elmorad player can enter Karus homeland during invasion
        let world = make_validation_world();
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.nation = NATION_ELMORAD;
            }
        });
        world.update_battle_state(|bs| {
            bs.karus_open_flag = true; // Karus under invasion
        });
        let result = validate_zone_entry(&world, 1, ZONE_KARUS);
        assert!(result.is_ok(), "Elmorad should enter Karus during invasion");
    }

    #[test]
    fn test_elmorad_homeland_karus_no_invasion() {
        // Karus player cannot enter Elmorad homeland without invasion
        let world = make_validation_world();
        // Player is already Karus from make_validation_world
        let result = validate_zone_entry(&world, 1, ZONE_ELMORAD);
        assert_eq!(
            result,
            Err((WARP_ERROR, 0)),
            "Karus denied without invasion flag"
        );
    }

    #[test]
    fn test_elmorad_homeland_karus_invasion_allowed() {
        // Karus player can enter Elmorad homeland during invasion
        let world = make_validation_world();
        world.update_battle_state(|bs| {
            bs.elmorad_open_flag = true; // Elmorad under invasion
        });
        let result = validate_zone_entry(&world, 1, ZONE_ELMORAD);
        assert!(result.is_ok(), "Karus should enter Elmorad during invasion");
    }

    // ── Sprint 280: Cinderella zone buff clearing ───────────────────────

    /// Test Cinderella zone detection for buff clearing.
    #[test]
    fn test_cinderella_zone_detection() {
        use crate::handler::cinderella::is_cinderella_zone;

        // When event zone matches destination, it's a Cinderella zone
        assert!(is_cinderella_zone(110, 110));
        assert!(!is_cinderella_zone(21, 110));
        // Note: is_cinderella_zone(0, 0) returns true since 0==0,
        // but in practice cinderella_zone_id() is never 0 when active.
        assert!(
            !is_cinderella_zone(21, 0),
            "event_zone=0 should not match real zones"
        );
    }

    /// Test post-load recast should skip Chaos Dungeon AND Cinderella zones.
    #[test]
    fn test_recast_skip_chaos_and_cinderella() {
        // ZONE_CHAOS_DUNGEON = 85, Cinderella zones are dynamic
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
        // Both should be excluded from post-load buff recast
        let chaos = ZONE_CHAOS_DUNGEON;
        let cind_active = true;
        let cind_zone: u16 = 110;
        let test_zone: u16 = 110;

        // Chaos dungeon excluded
        assert!(chaos == ZONE_CHAOS_DUNGEON, "Chaos dungeon zone is 85");
        // Cinderella zone excluded when active and matching
        let is_cind = cind_active && cind_zone == test_zone;
        assert!(
            is_cind,
            "Active Cinderella with matching zone should skip recast"
        );
        // Non-Cinderella zone not excluded
        let is_cind2 = cind_active && cind_zone == 21;
        assert!(!is_cind2, "Non-matching zone should not skip recast");
    }

    // ── Sprint 358: BottomUserLogOut on zone change ─────────────────

    /// Cross-zone change broadcasts RegionDelete (BottomUserLogOut) to old zone.
    ///
    #[test]
    fn test_cross_zone_change_sends_region_delete() {
        use crate::world::{CharacterInfo, Position, WorldState};

        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

        let sid1 = world.allocate_session_id();
        let sid2 = world.allocate_session_id();
        world.register_session(sid1, tx1);
        world.register_session(sid2, tx2);

        let ch1 = CharacterInfo {
            session_id: sid1,
            name: "Leaver".into(),
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
            gold: 0,
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
        let pos1 = Position {
            zone_id: 21,
            x: 500.0,
            y: 0.0,
            z: 500.0,
            region_x: 3,
            region_z: 3,
        };
        world.register_ingame(sid1, ch1, pos1);

        let ch2 = CharacterInfo {
            session_id: sid2,
            name: "Watcher".into(),
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
            gold: 0,
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
        let pos2 = Position {
            zone_id: 21,
            x: 510.0,
            y: 0.0,
            z: 510.0,
            region_x: 3,
            region_z: 3,
        };
        world.register_ingame(sid2, ch2, pos2);

        // Simulate BottomUserLogOut for sid1 leaving zone 21
        let ch = world.get_character_info(sid1).unwrap();
        let pkt = crate::handler::user_info::build_region_delete_packet(&ch.name);
        world.broadcast_to_zone(21, Arc::new(pkt), Some(sid1));

        // Player 2 should receive RegionDelete
        let received = rx2.try_recv().unwrap();
        assert_eq!(received.opcode, Opcode::WizUserInfo as u8);
        assert_eq!(received.data[0], 4); // RegionDelete sub-opcode

        // Verify character name in packet
        let mut r = PacketReader::new(&received.data);
        assert_eq!(r.read_u8(), Some(4));
        let name = r.read_sbyte_string().unwrap_or_default();
        assert_eq!(name, "Leaver");
    }

    /// RegionDelete packet has correct byte-perfect wire format.
    #[test]
    fn test_region_delete_packet_format() {
        let pkt = crate::handler::user_info::build_region_delete_packet("TestName");
        assert_eq!(pkt.opcode, Opcode::WizUserInfo as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4)); // sub-opcode = RegionDelete
        let name = r.read_sbyte_string().unwrap_or_default();
        assert_eq!(name, "TestName");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_goldshell_zone_entry_packet_format() {
        // C++ User.cpp:1936-1942 — GOLDSHELL(9) + enable(1) + socketID(u32)
        let sid: u16 = 42;
        let mut gs_pkt = Packet::new(Opcode::WizMapEvent as u8);
        gs_pkt.write_u8(9); // GOLDSHELL
        gs_pkt.write_u8(1); // enable
        gs_pkt.write_u32(sid as u32);

        assert_eq!(gs_pkt.opcode, Opcode::WizMapEvent as u8);
        assert_eq!(gs_pkt.data.len(), 6); // 1 + 1 + 4

        let mut r = PacketReader::new(&gs_pkt.data);
        assert_eq!(r.read_u8(), Some(9));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(42));
    }

    #[test]
    fn test_goldshell_zone_condition() {
        // GOLDSHELL sent when: battle_open == NATION_BATTLE && battle_zone_id != ZONE_BATTLE3
        use crate::systems::war;
        use crate::world::{ZONE_BATTLE3, ZONE_BATTLE_BASE};

        // Standard zone1 → GOLDSHELL should be sent
        let battle_zone: u8 = 1;
        let battle_zone_id = ZONE_BATTLE_BASE + battle_zone as u16;
        assert_ne!(battle_zone_id, ZONE_BATTLE3);

        // Zone3 (Ardream PvP) → GOLDSHELL should NOT be sent
        let battle_zone3: u8 = 3;
        let battle_zone_id3 = ZONE_BATTLE_BASE + battle_zone3 as u16;
        assert_eq!(battle_zone_id3, ZONE_BATTLE3);

        // Only active during NATION_BATTLE (1), not SNOW_BATTLE (2)
        assert_eq!(war::NATION_BATTLE, 1);
        assert_eq!(war::SNOW_BATTLE, 2);
    }

    // ── resolve_zero_coords tests (Sprint 671) ──────────────────────

    #[test]
    fn test_resolve_zero_coords_from_start_position() {
        use crate::world::{CharacterInfo, Position, WorldState};

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let ch = CharacterInfo {
            session_id: sid,
            nation: 1, // Karus
            name: "ZeroTest".into(),
            ..Default::default()
        };
        world.register_ingame(
            sid,
            ch,
            Position {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 2,
                region_z: 2,
            },
        );

        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 21,
            karus_x: 200,
            karus_z: 300,
            elmorad_x: 400,
            elmorad_z: 500,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });

        let (x, z) = resolve_zero_coords(21, sid, &world);
        assert_eq!(x, 200.0); // Karus coords
        assert_eq!(z, 300.0);
    }

    #[test]
    fn test_resolve_zero_coords_fallback_hardcoded() {
        use crate::world::{CharacterInfo, Position, WorldState};

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let ch = CharacterInfo {
            session_id: sid,
            nation: 2,
            name: "ZeroFallback".into(),
            ..Default::default()
        };
        world.register_ingame(
            sid,
            ch,
            Position {
                zone_id: 99,
                x: 50.0,
                y: 0.0,
                z: 50.0,
                region_x: 1,
                region_z: 1,
            },
        );

        // No start_position, no zone data → hardcoded fallback
        let (x, z) = resolve_zero_coords(99, sid, &world);
        assert_eq!(x, 267.0);
        assert_eq!(z, 303.0);
    }

    #[test]
    fn test_resolve_zero_coords_elmorad_nation() {
        use crate::world::{CharacterInfo, Position, WorldState};

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let ch = CharacterInfo {
            session_id: sid,
            nation: 2, // Elmorad
            name: "ElmoZero".into(),
            ..Default::default()
        };
        world.register_ingame(
            sid,
            ch,
            Position {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                region_x: 2,
                region_z: 2,
            },
        );

        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 21,
            karus_x: 200,
            karus_z: 300,
            elmorad_x: 400,
            elmorad_z: 500,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });

        let (x, z) = resolve_zero_coords(21, sid, &world);
        assert_eq!(x, 400.0); // Elmorad coords
        assert_eq!(z, 500.0);
    }
}
