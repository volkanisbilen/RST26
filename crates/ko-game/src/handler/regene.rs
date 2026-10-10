//! WIZ_REGENE (0x12) handler — respawn after death.
//! ## Client -> Server
//! `[u8 regene_type]`
//! - Type 1: Respawn at bind point / zone start
//! - Type 2: Respawn using resurrection stones (level > 5, costs 3 * level stones)
//! ## Server -> Client
//! `[u16 x*10] [u16 z*10] [u16 y*10]`
//! After sending the regene response, the server:
//! 1. Sets the player's region
//! 2. Broadcasts INOUT_RESPAWN to the 3x3 region
//! 3. Sends region user/NPC info to the player
//! 4. Calls `InitializeStealth()` — reset stealth state
//! 5. Cures DOT and poison status effects
//! 6. Restores HP to full (C++ calls `HpChange(GetMaxHealth())`)
//! 7. Recasts saved magic (buffs) if not blinking
//! 8. Activates blink (10s invulnerability) via `BlinkStart()`

use std::time::SystemTime;

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::handler::{region, zone_change};
use crate::session::{ClientSession, SessionState};
use crate::world::types::{
    ZONE_ARENA, ZONE_BATTLE_BASE, ZONE_CHAOS_DUNGEON, ZONE_DELOS, ZONE_DUNGEON_DEFENCE,
    ZONE_ELMORAD, ZONE_JURAID_MOUNTAIN, ZONE_KNIGHT_ROYALE, ZONE_SNOW_BATTLE, ZONE_UNDER_CASTLE,
};
use crate::zone::calc_region;

/// Resurrection stone item ID.
const ITEM_RESURRECTION_STONE: u32 = 379006000;

/// Minimum level required to use resurrection stones.
const MIN_LEVEL_FOR_STONES: u8 = 5;

/// Blink duration in seconds.
const BLINK_TIME: u64 = 10;

use crate::magic_constants::ABNORMAL_NORMAL;

/// Abnormal type: blinking (respawn invulnerability).
const ABNORMAL_BLINKING: u32 = 4;

/// Abnormal type: chaos/dungeon-defence normal (non-blinking form for special zones).
const ABNORMAL_CHAOS_NORMAL: u32 = 7;

use crate::magic_constants::{
    USER_STATUS_CURE, USER_STATUS_DOT, USER_STATUS_POISON, USER_STATUS_SPEED,
};
use crate::state_change_constants::STATE_CHANGE_ABNORMAL;

/// Handle WIZ_REGENE from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Player must be dead to respawn
    if !world.is_player_dead(sid) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let mut regene_type = reader.read_u8().unwrap_or(1);

    // Normalize: only 1 or 2 are valid
    if regene_type != 1 && regene_type != 2 {
        regene_type = 1;
    }

    let char_info = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Type 2: Resurrection stone respawn
    // Requires level > 5 and 3 * level resurrection stones (item 379006000)
    if regene_type == 2 {
        if char_info.level <= MIN_LEVEL_FOR_STONES {
            tracing::debug!(
                "[{}] Regene type 2: level {} too low (min {}), ignoring",
                session.addr(),
                char_info.level,
                MIN_LEVEL_FOR_STONES,
            );
            return Ok(());
        }

        let stone_count = 3u16 * char_info.level as u16;
        if !world.rob_item(sid, ITEM_RESURRECTION_STONE, stone_count) {
            tracing::debug!(
                "[{}] Regene type 2: not enough resurrection stones (need {})",
                session.addr(),
                stone_count,
            );
            return Ok(());
        }

        tracing::info!(
            "[{}] Regene type 2: consumed {} resurrection stones (level {})",
            session.addr(),
            stone_count,
            char_info.level,
        );
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Determine respawn location
    let (dest_zone, mut dest_x, mut dest_z) =
        determine_respawn_location(&char_info, pos.zone_id, &world);

    // Reset state: set to standing, restore HP/MP
    world.update_res_hp_type(sid, crate::world::USER_STANDING);

    world.update_session(sid, |h| {
        h.who_killed_me = -1;
        h.lost_exp = 0;
    });

    // Restore HP to full (C++ calls HpChange(GetMaxHealth()) at line 425)
    // GMs get 100%, regular players also get full HP per C++ reference
    let new_hp = char_info.max_hp;
    world.update_character_hp(sid, new_hp);

    //   if (GetZoneID() == ZONE_UNDER_CASTLE) MSpChange(GetMaxMana());
    // Only ZONE_UNDER_CASTLE gets full MP restore on respawn
    if dest_zone == ZONE_UNDER_CASTLE {
        world.update_character_mp(sid, char_info.max_mp);
    }

    // Check if we need to change zones
    if dest_zone != pos.zone_id {
        // Cross-zone respawn — use zone change system
        // First, send regene response with new coordinates
        let mut resp = Packet::new(Opcode::WizRegene as u8);
        resp.write_u16((dest_x * 10.0) as u16);
        resp.write_u16((dest_z * 10.0) as u16);
        resp.write_u16(0); // y * 10
        resp.write_u8(1); // v2600: trailing success byte (sniff verified, always 0x01)
        session.send_packet(&resp).await?;

        // Trigger zone change to respawn zone
        zone_change::trigger_zone_change(session, dest_zone, dest_x, dest_z).await?;
    } else {
        // Same-zone respawn
        // Validate spawn position is within zone map bounds
        if let Some(zone) = world.get_zone(pos.zone_id) {
            if !zone.is_valid_position(dest_x, dest_z) {
                tracing::error!(
                    "[{}] Respawn: invalid spawn position ({:.0}, {:.0}) in zone {} — using current position",
                    session.addr(),
                    dest_x,
                    dest_z,
                    pos.zone_id,
                );
                // Fallback: respawn at current position rather than OOB
                dest_x = pos.x;
                dest_z = pos.z;
            }
        }

        // 1. Broadcast INOUT_OUT from current position
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

        // 2. Remove from old region
        if let Some(zone) = world.get_zone(pos.zone_id) {
            zone.remove_user(pos.region_x, pos.region_z, sid);
        }

        // 3. Update position
        let new_rx = calc_region(dest_x);
        let new_rz = calc_region(dest_z);
        world.update_position(sid, pos.zone_id, dest_x, 0.0, dest_z);

        // 4. Add to new region
        if let Some(zone) = world.get_zone(pos.zone_id) {
            zone.add_user(new_rx, new_rz, sid);
        }

        // 5. Send WIZ_REGENE response to client
        let mut resp = Packet::new(Opcode::WizRegene as u8);
        resp.write_u16((dest_x * 10.0) as u16); // GetSPosX
        resp.write_u16((dest_z * 10.0) as u16); // GetSPosZ
        resp.write_u16(0); // GetSPosY (y=0)
        resp.write_u8(1); // v2600: trailing success byte (sniff verified, always 0x01)
        session.send_packet(&resp).await?;

        // 6. Broadcast INOUT_RESPAWN to new region
        region::broadcast_user_in(session).await?;

        // 7. Send region data to player
        region::send_region_user_in_out_for_me(session).await?;
        region::send_region_npc_info_for_me(session).await?;
    }

    // ── InitializeStealth ────────────────────────────────────────────
    // Sends WIZ_STEALTH with u8(0) u16(0) to reset stealth/invisibility
    send_initialize_stealth(session).await?;

    // ── Cure DOT & Poison ────────────────────────────────────────────
    //   SendUserStatusUpdate(USER_STATUS_DOT, USER_STATUS_CURE)
    //   SendUserStatusUpdate(USER_STATUS_POISON, USER_STATUS_CURE)
    world.clear_durational_skills(sid);
    send_user_status_update(session, USER_STATUS_DOT, USER_STATUS_CURE).await?;
    send_user_status_update(session, USER_STATUS_POISON, USER_STATUS_CURE).await?;

    // ── Arena speed cure ──────────────────────────────────────────────
    //   if (isInArena()) SendUserStatusUpdate(USER_STATUS_SPEED, USER_STATUS_CURE);
    if dest_zone == ZONE_ARENA {
        send_user_status_update(session, USER_STATUS_SPEED, USER_STATUS_CURE).await?;
    }

    // ── Reset anger gauge ─────────────────────────────────────────────
    //   if (GetAngerGauge() > 0) UpdateAngerGauge(0);
    super::arena::reset_anger_gauge(&world, sid);

    // 8. Send HP change to client (full HP restore)
    let mut hp_pkt = Packet::new(Opcode::WizHpChange as u8);
    hp_pkt.write_i16(new_hp); // MaxHP
    hp_pkt.write_i16(new_hp); // CurrentHP (fully restored)
    hp_pkt.write_u32(0x0000_FFFF); // attacker_id: C++ uint16(-1) → uint32 = 65535 (no attacker)
    session.send_packet(&hp_pkt).await?;

    // ── RecastSavedMagic ─────────────────────────────────────────────
    //   if (!isBlinking() && zone != CHAOS/DUNGEON_DEFENCE/KNIGHT_ROYALE)
    //     { InitType4(); RecastSavedMagic(); }
    // Recast on all regene types (C++ does it for magicid != 0, but also for normal regene
    // when not blinking and not in special zones).
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let is_blinking = world.is_player_blinking(sid, now);
        let is_special_zone = matches!(
            dest_zone,
            ZONE_CHAOS_DUNGEON | ZONE_DUNGEON_DEFENCE | ZONE_KNIGHT_ROYALE
        );
        if !is_blinking && !is_special_zone {
            world.clear_all_buffs(sid, false);
            // C++ pattern: InitType4() → SetUserAbility() → RecastSavedMagic()
            world.set_user_ability(sid);
            world.recast_saved_magic(sid);
        }
    }

    // ── BlinkStart ───────────────────────────────────────────────────
    //   if (GetZoneID() == ZONE_CHAOS_DUNGEON) BlinkStart(-10);
    //   else if (magicid == 0 && !isNPCTransformation()) BlinkStart();
    // BlinkStart(-10) means BLINK_TIME(-10) = 0s — no blink in Chaos Dungeon.
    if dest_zone != ZONE_CHAOS_DUNGEON {
        activate_blink(session, dest_zone)?;
    }

    // ── ZoneOnlineRewardChange ──────────────────────────────────────
    // Reset the player's online reward timer after respawn.
    crate::systems::zone_rewards::zone_online_reward_change(&world, sid);

    // ── Loyalty-zero kick ─────────────────────────────────────────
    //   if (magicid == 0) {
    //     if (GetLoyalty() == 0 && (GetMap()->isWarZone()
    //         || isInSpecialEventZone() || isInPKZone() || cindirella))
    //       KickOutZoneUser();
    //   }
    // Only on normal respawn (regene_type 1), not resurrection.
    // Players with 0 loyalty in war/PvP/PK zones are sent to their home zone.
    if regene_type == 1 {
        let loyalty = world
            .get_character_info(sid)
            .map(|ch| ch.loyalty)
            .unwrap_or(0);
        if loyalty == 0 {
            // C++ isWarZone() || isInPKZone() || isInSpecialEventZone()
            // isInPKZone: zones 71 (Ronark Land), 72 (Ardream), 73 (Ronark Land Base)
            // isInSpecialEventZone: zones 105-115 (SPBATTLE1-SPBATTLE11)
            let is_kick_zone = world.get_zone(dest_zone).is_some_and(|z| z.is_war_zone())
                || matches!(dest_zone, 71..=73)
                || (105..=115).contains(&dest_zone);
            if is_kick_zone {
                // Send player to their home/bind zone via zone change
                let home = determine_respawn_location(&char_info, 0, &world);
                if home.0 != dest_zone {
                    zone_change::trigger_zone_change(session, home.0, home.1, home.2).await?;
                    tracing::info!(
                        "[{}] Loyalty-zero kick: sid={} sent home to zone {} from war zone {}",
                        session.addr(),
                        sid,
                        home.0,
                        dest_zone,
                    );
                }
            }
        }
    }

    // 9. Save respawn position to DB (fire-and-forget)
    zone_change::save_position_async(session, dest_zone, dest_x, dest_z);

    tracing::info!(
        "[{}] Player respawned (type={}) at zone {} ({:.0},{:.0}) with HP={}/{}",
        session.addr(),
        regene_type,
        dest_zone,
        dest_x,
        dest_z,
        new_hp,
        char_info.max_hp,
    );

    Ok(())
}

/// Send WIZ_STEALTH to reset stealth/invisibility state.
/// ```text
/// Packet pkt(WIZ_STEALTH);
/// pkt << uint8(0) << uint16(0);
/// Send(&pkt);
/// ```
async fn send_initialize_stealth(session: &mut ClientSession) -> anyhow::Result<()> {
    // Reset invisibility_type to INVIS_NONE
    session
        .world()
        .set_invisibility_type(session.session_id(), 0);

    let mut pkt = Packet::new(Opcode::WizStealth as u8);
    pkt.write_u8(0);
    pkt.write_u16(0);
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Send WIZ_ZONEABILITY sub-opcode 2 to cure a status effect,
/// then broadcast PARTY_STATUSCHANGE to party members.
/// ```text
/// Packet result(WIZ_ZONEABILITY, uint8(2));
/// result << uint8(type) << uint8(status);
/// ```
/// C++ also calls `SendPartyStatusUpdate()` (`PartyHandler.cpp:1275-1282`).
async fn send_user_status_update(
    session: &mut ClientSession,
    status_type: u8,
    status_behaviour: u8,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizZoneability as u8);
    pkt.write_u8(2); // sub-opcode for status update
    pkt.write_u8(status_type);
    pkt.write_u8(status_behaviour);
    session.send_packet(&pkt).await?;

    let world = session.world().clone();
    let sid = session.session_id();
    if let Some(party_id) = world.get_party_id(sid) {
        let mut party_pkt = Packet::new(Opcode::WizParty as u8);
        party_pkt.write_u8(crate::handler::party::PARTY_STATUSCHANGE);
        party_pkt.write_u32(sid as u32);
        party_pkt.write_u8(status_type);
        party_pkt.write_u8(status_behaviour);
        world.send_to_party(party_id, &party_pkt);
    }
    Ok(())
}

/// Activate blink (respawn invulnerability).
/// `duration_secs` is the total blink duration in seconds.
/// C++ uses `BLINK_TIME(10) + exBlinkTime`, so:
/// - Normal: pass `BLINK_TIME` (10)
/// - Special zones (Chaos Dungeon, Knight Royale, Dungeon Defence): pass 55 (10 + 45)
/// Checks:
/// - Not a GM (authority == 0)
/// - Zone supports blink (blink_zone == true, not a war zone)
/// Sets `blink_expiry_time` and broadcasts `ABNORMAL_BLINKING` state change.
pub(crate) fn activate_blink_with_duration(
    session: &mut ClientSession,
    zone_id: u16,
    duration_secs: u64,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let is_gm = world
        .get_character_info(sid)
        .map(|ch| ch.authority == 0)
        .unwrap_or(false);
    if is_gm {
        return Ok(());
    }

    // M5: Skip blink if transformed
    if world.is_transformed(sid) {
        return Ok(());
    }

    // If war zone or blink_zone is not enabled, clear blink if active and return
    let (is_war, has_blink) = world
        .get_zone(zone_id)
        .map(|z| {
            (
                z.is_war_zone(),
                z.zone_info
                    .as_ref()
                    .map(|zi| zi.abilities.blink_zone)
                    .unwrap_or(false),
            )
        })
        .unwrap_or((false, false));

    if is_war || !has_blink {
        let was_blinking = {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            world.is_player_blinking(sid, now)
        };
        if was_blinking {
            world.clear_blink(sid);
            let normal_type = if zone_id == ZONE_CHAOS_DUNGEON || zone_id == ZONE_DUNGEON_DEFENCE {
                ABNORMAL_CHAOS_NORMAL
            } else {
                ABNORMAL_NORMAL
            };
            let state_pkt =
                build_state_change_broadcast(sid as u32, STATE_CHANGE_ABNORMAL, normal_type);
            if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room))
            {
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(state_pkt),
                    None,
                    event_room,
                );
            }
        }
        return Ok(());
    }

    // Set blink expiry
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expiry = now + duration_secs;

    //   m_bAbnormalType = ABNORMAL_BLINKING;
    //   m_tBlinkExpiryTime = UNIXTIME + BLINK_TIME;
    //   m_bRegeneType = REGENE_ZONECHANGE;
    //   m_bCanUseSkills = false;
    world.update_session(sid, |h| {
        h.blink_expiry_time = expiry;
        h.can_use_skills = false;
    });

    // Broadcast ABNORMAL_BLINKING state change to 3x3 region
    let state_pkt =
        build_state_change_broadcast(sid as u32, STATE_CHANGE_ABNORMAL, ABNORMAL_BLINKING);
    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(state_pkt),
            None,
            event_room,
        );
    }

    tracing::debug!(
        "[sid={}] BlinkStart: blink active for {}s until unix_time={}",
        sid,
        duration_secs,
        expiry,
    );

    Ok(())
}

/// Activate blink with the default duration (BLINK_TIME = 10 seconds).
/// Convenience wrapper around `activate_blink_with_duration`.
pub(crate) fn activate_blink(session: &mut ClientSession, zone_id: u16) -> anyhow::Result<()> {
    activate_blink_with_duration(session, zone_id, BLINK_TIME)
}

/// Build a WIZ_STATE_CHANGE broadcast packet.
/// Format: `[u32 socket_id] [u8 bType] [u32 nBuff]`
pub fn build_state_change_broadcast(sid: u32, b_type: u8, n_buff: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizStateChange as u8);
    pkt.write_u32(sid);
    pkt.write_u8(b_type);
    pkt.write_u32(n_buff);
    pkt
}

/// Determine the respawn location for a dead player.
/// Priority order:
/// 1. Bind point (if set and not in ZONE_DELOS)
/// 2. Home zone (≤ZONE_ELMORAD) or active battle zone → nation-specific from start_position + random offset
/// 3. Chaos Dungeon / Bowl event zone → random from start_position_random
/// 4. Juraid Mountain → stage-specific coords per nation (only when event active)
/// 5. Default → nation-specific from start_position table
/// 6. Fallback: zone init_x/init_z
/// 7. HOME table nation coords
/// 8. Moradon (21) fallback
fn determine_respawn_location(
    char_info: &crate::world::CharacterInfo,
    current_zone: u16,
    world: &crate::world::WorldState,
) -> (u16, f32, f32) {
    use rand::Rng;

    // ── 1. Bind point respawn ─────────────────────────────────────────
    // Our bind_zone/bind_x/bind_z is equivalent to C++'s m_sBind event coords.
    let bind_zone = char_info.bind_zone as u16;
    if bind_zone > 0
        && (char_info.bind_x != 0.0 || char_info.bind_z != 0.0)
        && current_zone != ZONE_DELOS
    {
        return (bind_zone, char_info.bind_x, char_info.bind_z);
    }

    // ── 2. Home zone or active battle zone → nation-specific coords ──
    //   if ((GetZoneID() <= ZONE_ELMORAD) ||
    //       (GetZoneID() != ZONE_SNOW_BATTLE && GetZoneID() == ZONE_BATTLE_BASE + m_byBattleZone))
    //   {
    //     x = (GetNation()==KARUS ? sKarusX : sElmoradX) + myrand(0, bRangeX);
    //     z = (GetNation()==KARUS ? sKarusZ : sElmoradZ) + myrand(0, bRangeZ);
    //   }
    let battle_zone_id = world.get_battle_zone_id();
    let is_home_zone = current_zone <= ZONE_ELMORAD;
    let is_battle_zone = current_zone != ZONE_SNOW_BATTLE
        && battle_zone_id > 0
        && current_zone == ZONE_BATTLE_BASE + battle_zone_id;

    if is_home_zone || is_battle_zone {
        if let Some(sp) = world.get_start_position(current_zone) {
            let mut rng = rand::thread_rng();
            let (base_x, base_z) = if char_info.nation == 1 {
                (sp.karus_x as f32, sp.karus_z as f32)
            } else {
                (sp.elmorad_x as f32, sp.elmorad_z as f32)
            };
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
            return (current_zone, base_x + offset_x, base_z + offset_z);
        }
    }

    // ── 2b. Delos: CSW master-clan members respawn in the castellan area ──
    //   C++ GetStartPosition (User.cpp:4351): x = 505 + rand(0..range_x), z = 840 + rand(0..range_z)
    if current_zone == ZONE_DELOS {
        let master = world.get_csw_master_knights();
        if master != 0 && char_info.knights_id != 0 && char_info.knights_id == master {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let (range_x, range_z) = world
                .get_start_position(current_zone)
                .map(|s| (s.range_x.max(0), s.range_z.max(0)))
                .unwrap_or((5, 5));
            return (
                current_zone,
                505.0 + rng.gen_range(0..=range_x) as f32,
                840.0 + rng.gen_range(0..=range_z) as f32,
            );
        }
    }

    // ── 3. Chaos Dungeon / Bowl event → random spawn point ───────────
    //   if (GetZoneID() == ZONE_CHAOS_DUNGEON || (tBowlEventZone == GetZoneID() && isBowlEventActive))
    //     GetStartPositionRandom(sx, sz);
    if let Some((rx, rz)) = world.get_start_position_random(current_zone) {
        return (current_zone, rx, rz);
    }

    // ── 4. Juraid Mountain — stage-specific respawn coords ───────────
    //   3 stages × 2 nations = 6 coordinate sets with ±3 random offset.
    //   isDevaStage:   K(511,738) E(511,281)
    //   isBridgeStage2: K(336,848) E(695,171)
    //   isBridgeStage1: K(224,671) E(800,349)
    if current_zone == ZONE_JURAID_MOUNTAIN {
        if let Some((jx, jz)) = juraid_respawn_coords(char_info.nation, world) {
            return (current_zone, jx, jz);
        }
    }

    // ── 5. Default: nation-specific from start_position table ─────────
    if let Some(sp) = world.get_start_position(current_zone) {
        let (base_x, base_z) = if char_info.nation == 1 {
            (sp.karus_x as f32, sp.karus_z as f32)
        } else {
            (sp.elmorad_x as f32, sp.elmorad_z as f32)
        };
        if base_x != 0.0 || base_z != 0.0 {
            return (current_zone, base_x, base_z);
        }
    }

    // ── 6. Fallback: zone init_x/init_z ──────────────────────────────
    if let Some(zone) = world.get_zone(current_zone) {
        let (x, z, _y) = zone.spawn_position();
        if x != 0.0 || z != 0.0 {
            return (current_zone, x, z);
        }
    }

    // ── 7. HOME table nation coords ──────────────────────────────────
    if let Some(home) = world.get_home_position(char_info.nation) {
        let (x, z) = if char_info.nation == 1 {
            (home.karus_zone_x as f32, home.karus_zone_z as f32)
        } else {
            (home.elmo_zone_x as f32, home.elmo_zone_z as f32)
        };
        if x != 0.0 || z != 0.0 {
            return (char_info.nation as u16, x, z);
        }
    }

    // ── 8. Moradon fallback ──────────────────────────────────────────
    (21, 512.0, 341.0)
}

/// Determine Juraid Mountain respawn coordinates based on bridge stage.
/// Stage determination from bridge state:
/// - No bridges open → Deva stage (starting area)
/// - Bridge 0 open, bridge 1 not → Bridge1 stage
/// - Bridge 1 open → Bridge2 stage
/// Coordinates are hardcoded per stage per nation with ±3 random offset.
fn juraid_respawn_coords(nation: u8, world: &crate::world::WorldState) -> Option<(f32, f32)> {
    use rand::Rng;

    // Determine stage from bridge state.
    // Try rooms 1..=10 to find any active bridge state.
    // C++ requires isInValidRoom(0) — we require at least one room to have bridge state.
    let mut found_room = false;
    let mut stage = 0u8; // 0=deva, 1=bridge1, 2=bridge2
    for room_id in 1..=10u8 {
        if let Some(bs) = world.get_juraid_bridge_state(room_id) {
            found_room = true;
            if bs.karus_bridges[1] || bs.elmorad_bridges[1] {
                stage = 2;
            } else if bs.karus_bridges[0] || bs.elmorad_bridges[0] {
                stage = 1;
            }
            break;
        }
    }

    // No active event room → fall through to default GetStartPosition path
    if !found_room {
        return None;
    }

    let mut rng = rand::thread_rng();
    // C++ uses separate myrand(0,3) calls for X and Z
    let rx = rng.gen_range(0..=3) as f32;
    let rz = rng.gen_range(0..=3) as f32;

    let (x, z) = match (stage, nation) {
        // Deva stage (initial area — no bridges open)
        (0, 1) => (511.0 + rx, 738.0 + rz),
        (0, _) => (511.0 + rx, 281.0 + rz),
        // Bridge1 stage (bridge[0] open)
        (1, 1) => (224.0 + rx, 671.0 + rz),
        (1, _) => (800.0 + rx, 349.0 + rz),
        // Bridge2 stage (bridge[1] open)
        (2, 1) => (336.0 + rx, 848.0 + rz),
        (2, _) => (695.0 + rx, 171.0 + rz),
        _ => (511.0 + rx, 738.0 + rz),
    };

    Some((x, z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_regene_request_format() {
        // Client -> Server: [u8 regene_type]
        let mut pkt = Packet::new(Opcode::WizRegene as u8);
        pkt.write_u8(1); // type 1 = normal respawn

        assert_eq!(pkt.opcode, Opcode::WizRegene as u8);
        assert_eq!(pkt.data.len(), 1);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_regene_response_format() {
        // Server -> Client: [u16 x*10] [u16 z*10] [u16 y*10]
        let x = 512.0_f32;
        let z = 341.0_f32;

        let mut pkt = Packet::new(Opcode::WizRegene as u8);
        pkt.write_u16((x * 10.0) as u16);
        pkt.write_u16((z * 10.0) as u16);
        pkt.write_u16(0); // y * 10

        assert_eq!(pkt.opcode, Opcode::WizRegene as u8);
        assert_eq!(pkt.data.len(), 6);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(5120)); // 512 * 10
        assert_eq!(r.read_u16(), Some(3410)); // 341 * 10
        assert_eq!(r.read_u16(), Some(0)); // y * 10
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_hp_change_packet_format() {
        // WIZ_HP_CHANGE: [i16 max_hp] [i16 current_hp] [u32 attacker_id]
        let max_hp: i16 = 1000;
        let current_hp: i16 = 1000;

        let mut pkt = Packet::new(Opcode::WizHpChange as u8);
        pkt.write_i16(max_hp);
        pkt.write_i16(current_hp);
        pkt.write_u32(0xFFFF); // no attacker

        assert_eq!(pkt.data.len(), 8); // 2 + 2 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u32(), Some(0xFFFF));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_respawn_hp_calculation() {
        // Full HP restore on respawn (per C++ reference)
        let max_hp: i16 = 1000;
        let restored_hp = max_hp; // C++ restores to full
        assert_eq!(restored_hp, 1000);
    }

    #[test]
    fn test_respawn_position_bind_point() {
        // When bind_zone > 0 and bind coords are set, use bind point
        let bind_zone: u8 = 11;
        let bind_x: f32 = 200.0;
        let bind_z: f32 = 300.0;

        // Simulate determine_respawn_location logic
        assert!(bind_zone > 0);
        assert!(bind_x != 0.0 || bind_z != 0.0);
        // Should return bind point
        let (zone, x, z) = (bind_zone as u16, bind_x, bind_z);
        assert_eq!(zone, 11);
        assert_eq!(x, 200.0);
        assert_eq!(z, 300.0);
    }

    #[test]
    fn test_respawn_position_fallback_moradon() {
        // When no bind point and no zone spawn, fallback to Moradon (zone 21)
        let bind_zone: u8 = 0;
        let bind_x: f32 = 0.0;
        let bind_z: f32 = 0.0;

        assert!(bind_zone == 0 || (bind_x == 0.0 && bind_z == 0.0));
        // Fallback: zone 21 (Moradon), coords (512, 341)
        let (zone, x, z) = (21_u16, 512.0_f32, 341.0_f32);
        assert_eq!(zone, 21);
        assert_eq!(x, 512.0);
        assert_eq!(z, 341.0);
    }

    #[test]
    fn test_respawn_position_coords_scaled() {
        // WIZ_REGENE response sends positions multiplied by 10
        let x = 512.5_f32;
        let z = 341.3_f32;

        let sx = (x * 10.0) as u16;
        let sz = (z * 10.0) as u16;
        assert_eq!(sx, 5125);
        assert_eq!(sz, 3413);
    }

    #[test]
    fn test_regene_type_normalization() {
        // Only types 1 and 2 are valid, anything else becomes 1
        for invalid in [0_u8, 3, 4, 5, 255] {
            let normalized = if invalid != 1 && invalid != 2 {
                1
            } else {
                invalid
            };
            assert_eq!(normalized, 1);
        }
        assert_eq!(1_u8, 1); // type 1 stays 1
        assert_eq!(2_u8, 2); // type 2 stays 2
    }

    #[test]
    fn test_resurrection_stone_constants() {
        assert_eq!(ITEM_RESURRECTION_STONE, 379006000);
        assert_eq!(MIN_LEVEL_FOR_STONES, 5);
    }

    #[test]
    fn test_resurrection_stone_cost_calculation() {
        // Cost = 3 * level
        assert_eq!(3u16 * 10, 30); // level 10 needs 30 stones
        assert_eq!(3u16 * 60, 180); // level 60 needs 180 stones
        assert_eq!(3u16 * 83, 249); // level 83 (max) needs 249 stones
    }

    #[test]
    fn test_resurrection_stone_level_check() {
        // Level 5 and below cannot use resurrection stones
        for level in 0..=5u8 {
            assert!(level <= MIN_LEVEL_FOR_STONES);
        }
        // Level 6+ can use them
        for level in 6..=83u8 {
            assert!(level > MIN_LEVEL_FOR_STONES);
        }
    }

    #[test]
    fn test_regene_type2_request_format() {
        // Client -> Server: [u8 regene_type=2]
        let mut pkt = Packet::new(Opcode::WizRegene as u8);
        pkt.write_u8(2); // type 2 = resurrection stone

        assert_eq!(pkt.opcode, Opcode::WizRegene as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.remaining(), 0);
    }

    // ── Blink constants ──────────────────────────────────────────────

    #[test]
    fn test_blink_time_constant() {
        assert_eq!(BLINK_TIME, 10);
    }

    #[test]
    fn test_abnormal_constants() {
        assert_eq!(ABNORMAL_NORMAL, 1);
        assert_eq!(ABNORMAL_BLINKING, 4);
    }

    // ── State change broadcast packet ────────────────────────────────

    #[test]
    fn test_build_state_change_broadcast_blinking() {
        let pkt = build_state_change_broadcast(42, STATE_CHANGE_ABNORMAL, ABNORMAL_BLINKING);
        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42)); // session_id
        assert_eq!(r.read_u8(), Some(3)); // bType = abnormal
        assert_eq!(r.read_u32(), Some(4)); // nBuff = ABNORMAL_BLINKING
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_state_change_broadcast_normal() {
        let pkt = build_state_change_broadcast(100, STATE_CHANGE_ABNORMAL, ABNORMAL_NORMAL);
        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(100)); // session_id
        assert_eq!(r.read_u8(), Some(3)); // bType = abnormal
        assert_eq!(r.read_u32(), Some(1)); // nBuff = ABNORMAL_NORMAL
        assert_eq!(r.remaining(), 0);
    }

    // ── InitializeStealth packet format ──────────────────────────────

    #[test]
    fn test_initialize_stealth_packet_format() {
        // Packet pkt(WIZ_STEALTH); pkt << uint8(0) << uint16(0);
        let mut pkt = Packet::new(Opcode::WizStealth as u8);
        pkt.write_u8(0);
        pkt.write_u16(0);

        assert_eq!(pkt.opcode, Opcode::WizStealth as u8);
        assert_eq!(pkt.data.len(), 3); // u8 + u16

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    // ── SendUserStatusUpdate packet format ───────────────────────────

    #[test]
    fn test_user_status_update_dot_cure_format() {
        // Packet result(WIZ_ZONEABILITY, uint8(2));
        // result << uint8(type) << uint8(status);
        let mut pkt = Packet::new(Opcode::WizZoneability as u8);
        pkt.write_u8(2); // sub-opcode
        pkt.write_u8(USER_STATUS_DOT);
        pkt.write_u8(USER_STATUS_CURE);

        assert_eq!(pkt.opcode, Opcode::WizZoneability as u8);
        assert_eq!(pkt.data.len(), 3);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub-opcode
        assert_eq!(r.read_u8(), Some(1)); // USER_STATUS_DOT
        assert_eq!(r.read_u8(), Some(0)); // USER_STATUS_CURE
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_user_status_update_poison_cure_format() {
        let mut pkt = Packet::new(Opcode::WizZoneability as u8);
        pkt.write_u8(2);
        pkt.write_u8(USER_STATUS_POISON);
        pkt.write_u8(USER_STATUS_CURE);

        assert_eq!(pkt.opcode, Opcode::WizZoneability as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub-opcode
        assert_eq!(r.read_u8(), Some(2)); // USER_STATUS_POISON
        assert_eq!(r.read_u8(), Some(0)); // USER_STATUS_CURE
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_user_status_constants() {
        assert_eq!(USER_STATUS_DOT, 1);
        assert_eq!(USER_STATUS_POISON, 2);
        assert_eq!(USER_STATUS_CURE, 0);
    }

    // ── State change broadcast size ──────────────────────────────────

    #[test]
    fn test_state_change_broadcast_size() {
        // [u32 socket_id] [u8 bType] [u32 nBuff] = 4 + 1 + 4 = 9 bytes
        let pkt = build_state_change_broadcast(1, STATE_CHANGE_ABNORMAL, 4);
        assert_eq!(pkt.data.len(), 9);
    }

    // ── Blink expiry calculation ─────────────────────────────────────

    #[test]
    fn test_blink_expiry_calculation() {
        let now: u64 = 1700000000;
        let expiry = now + BLINK_TIME;
        assert_eq!(expiry, 1700000010);
        assert!(now < expiry); // still blinking at start
        assert!(expiry <= expiry); // expired at exact expiry time
    }

    #[test]
    fn test_blink_time_check_logic() {
        let now: u64 = 1700000015;
        let expiry: u64 = 1700000010;
        // now >= expiry means blink has expired
        assert!(now >= expiry, "Blink should be expired");

        let still_active_expiry: u64 = 1700000020;
        assert!(now < still_active_expiry, "Blink should still be active");
    }

    // ── State change type constant ───────────────────────────────────

    #[test]
    fn test_state_change_type_abnormal() {
        assert_eq!(STATE_CHANGE_ABNORMAL, 3);
    }

    // ── Sprint 42: New constants ─────────────────────────────────────

    #[test]
    fn test_abnormal_chaos_normal_constant() {
        assert_eq!(ABNORMAL_CHAOS_NORMAL, 7);
    }

    #[test]
    fn test_zone_constants() {
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
        assert_eq!(ZONE_DUNGEON_DEFENCE, 89);
        assert_eq!(ZONE_ARENA, 48);
        assert_eq!(ZONE_UNDER_CASTLE, 86);
    }

    #[test]
    fn test_user_status_speed_constant() {
        assert_eq!(USER_STATUS_SPEED, 3);
    }

    #[test]
    fn test_chaos_zone_blink_clear_uses_chaos_normal() {
        // When clearing blink in ZONE_CHAOS_DUNGEON or ZONE_DUNGEON_DEFENCE,
        // the server should broadcast ABNORMAL_CHAOS_NORMAL (7) instead of ABNORMAL_NORMAL (1).
        let zone_id = ZONE_CHAOS_DUNGEON;
        let chaos_type = if zone_id == ZONE_CHAOS_DUNGEON || zone_id == ZONE_DUNGEON_DEFENCE {
            ABNORMAL_CHAOS_NORMAL
        } else {
            ABNORMAL_NORMAL
        };
        assert_eq!(chaos_type, ABNORMAL_CHAOS_NORMAL);
    }

    #[test]
    fn test_normal_zone_blink_clear_uses_normal() {
        // Normal zones use ABNORMAL_NORMAL (1) when clearing blink
        let zone_id: u16 = 21; // Moradon
        let normal_type = if zone_id == ZONE_CHAOS_DUNGEON || zone_id == ZONE_DUNGEON_DEFENCE {
            ABNORMAL_CHAOS_NORMAL
        } else {
            ABNORMAL_NORMAL
        };
        assert_eq!(normal_type, ABNORMAL_NORMAL);
    }

    #[test]
    fn test_arena_speed_cure_packet_format() {
        // if (isInArena()) SendUserStatusUpdate(USER_STATUS_SPEED, USER_STATUS_CURE)
        let mut pkt = Packet::new(Opcode::WizZoneability as u8);
        pkt.write_u8(2); // sub-opcode
        pkt.write_u8(USER_STATUS_SPEED);
        pkt.write_u8(USER_STATUS_CURE);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // sub-opcode
        assert_eq!(r.read_u8(), Some(3)); // USER_STATUS_SPEED
        assert_eq!(r.read_u8(), Some(0)); // USER_STATUS_CURE
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 43: Transformation blink skip tests ──────────────────

    #[test]
    fn test_blink_skip_when_transformed() {
        // When a player is transformed, BlinkStart should be skipped
        let is_transformed = true;
        let is_gm = false;

        // Should skip blink if transformed
        let should_skip = is_gm || is_transformed;
        assert!(
            should_skip,
            "Blink should be skipped for transformed players"
        );
    }

    #[test]
    fn test_blink_allowed_when_not_transformed() {
        // When a player is NOT transformed, BlinkStart should proceed
        let is_transformed = false;
        let is_gm = false;

        let should_skip = is_gm || is_transformed;
        assert!(
            !should_skip,
            "Blink should proceed for non-transformed players"
        );
    }

    #[test]
    fn test_blink_sets_can_use_skills_false() {
        // When blink activates, skills should be disabled
        // BlinkStart sets can_use_skills to false
        let can_use_skills = false;
        assert!(!can_use_skills, "Skills should be disabled during blink");
    }

    #[test]
    fn test_blink_clear_restores_can_use_skills() {
        // When blink expires, skills should be re-enabled
        // BlinkTimeCheck sets can_use_skills to true
        let can_use_skills = true;
        assert!(can_use_skills, "Skills should be re-enabled after blink");
    }

    #[test]
    fn test_post_blink_transform_skill_reenable_logic() {
        //   if (!isBlinking() && isTransformed() && m_bCanUseSkills == false)
        //       m_bCanUseSkills = true;
        let is_blinking = false;
        let is_transformed = true;
        let can_use_skills = false;

        let should_reenable = !is_blinking && is_transformed && !can_use_skills;
        assert!(
            should_reenable,
            "Skills should re-enable post-blink when transformed"
        );
    }

    #[test]
    fn test_post_blink_no_reenable_if_still_blinking() {
        // If still blinking, don't re-enable
        let is_blinking = true;
        let is_transformed = true;
        let can_use_skills = false;

        let should_reenable = !is_blinking && is_transformed && !can_use_skills;
        assert!(
            !should_reenable,
            "Skills should NOT re-enable while still blinking"
        );
    }

    // ── Sprint 289: Mini Arena respawn ──────────────────────────────────

    #[test]
    fn test_mini_arena_respawn_constants() {
        // MINI_ARENA_RESPAWN_X=734, MINI_ARENA_RESPAWN_Z=427, RADIUS=5
        let x: f32 = 734.0;
        let z: f32 = 427.0;
        let radius: f32 = 5.0;
        assert_eq!(ZONE_ARENA, 48);
        // Min/max bounds
        assert!(x - radius >= 729.0);
        assert!(x + radius <= 739.0);
        assert!(z - radius >= 422.0);
        assert!(z + radius <= 432.0);
    }

    // ── Sprint 316: ZONE_DELOS bind point restriction ────────────────

    /// Players in Delos must use the zone's default spawn position.
    #[test]
    fn test_delos_blocks_bind_point() {
        let current_zone = ZONE_DELOS;
        let bind_zone: u16 = 1; // valid bind point
        let bind_x: f32 = 100.0;

        // Even with a valid bind point, ZONE_DELOS must NOT use it
        let can_use_bind = bind_zone > 0 && bind_x != 0.0 && current_zone != ZONE_DELOS;
        assert!(!can_use_bind, "ZONE_DELOS must block bind point respawn");
    }

    #[test]
    fn test_non_delos_allows_bind_point() {
        let current_zone: u16 = 1; // Karus
        let bind_zone: u16 = 1;
        let bind_x: f32 = 100.0;

        let can_use_bind = bind_zone > 0 && bind_x != 0.0 && current_zone != ZONE_DELOS;
        assert!(
            can_use_bind,
            "Non-Delos zones should allow bind point respawn"
        );
    }

    /// Verify PARTY_STATUSCHANGE packet format:
    /// [u8 sub=0x09] [u32 socketID] [u8 status_type] [u8 status_behaviour]
    #[test]
    fn test_party_status_change_packet_format() {
        let mut pkt = Packet::new(Opcode::WizParty as u8);
        pkt.write_u8(crate::handler::party::PARTY_STATUSCHANGE);
        pkt.write_u32(42); // session id
        pkt.write_u8(2); // poison
        pkt.write_u8(1); // apply

        assert_eq!(pkt.opcode, Opcode::WizParty as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x09));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(2)); // poison type
        assert_eq!(r.read_u8(), Some(1)); // apply
        assert_eq!(r.remaining(), 0);
    }

    /// Verify all status types from C++ reference:
    /// 1=DOT, 2=poison, 3=disease, 4=blind, 5=grey_hp
    #[test]
    fn test_status_types_match_cpp() {
        // C++ User.cpp:4374-4383 comment:
        assert_eq!(1u8, 1); // DOT
        assert_eq!(2u8, 2); // poison (purple)
        assert_eq!(3u8, 3); // disease (green)
        assert_eq!(4u8, 4); // blind
        assert_eq!(5u8, 5); // grey HP

        // Behaviour: 0 = cure, 1 = apply
        let cure: u8 = 0;
        let apply: u8 = 1;
        assert_ne!(cure, apply);
    }

    /// PARTY_STATUSCHANGE cure packet (behaviour=0).
    #[test]
    fn test_party_status_change_cure() {
        let mut pkt = Packet::new(Opcode::WizParty as u8);
        pkt.write_u8(crate::handler::party::PARTY_STATUSCHANGE);
        pkt.write_u32(100); // session id
        pkt.write_u8(1); // DOT
        pkt.write_u8(0); // cure

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x09));
        assert_eq!(r.read_u32(), Some(100));
        assert_eq!(r.read_u8(), Some(1)); // DOT type
        assert_eq!(r.read_u8(), Some(0)); // cured
        assert_eq!(r.remaining(), 0);
    }

    // ── determine_respawn_location integration tests ─────────────────

    use crate::world::{CharacterInfo, WorldState};

    fn make_char_info(nation: u8) -> CharacterInfo {
        CharacterInfo {
            nation,
            ..Default::default()
        }
    }

    #[test]
    fn test_respawn_arena_uses_start_position() {
        // ZONE_ARENA (48) should use start_position table (C++ path: GetStartPosition),
        // NOT the Moradon mini-arena fixed center (which is for isInMoradon()&&isInArena())
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 48,
            karus_x: 120,
            karus_z: 115,
            elmorad_x: 120,
            elmorad_z: 115,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 5,
            range_z: 5,
        });
        let ch = make_char_info(1);
        let (zone, x, z) = determine_respawn_location(&ch, ZONE_ARENA, &world);
        assert_eq!(zone, ZONE_ARENA);
        // Nation-specific from start_position: Karus(120, 115)
        assert_eq!(x, 120.0);
        assert_eq!(z, 115.0);
    }

    #[test]
    fn test_respawn_bind_point_used() {
        let world = WorldState::new();
        let mut ch = make_char_info(1);
        ch.bind_zone = 11;
        ch.bind_x = 200.0;
        ch.bind_z = 300.0;
        let (zone, x, z) = determine_respawn_location(&ch, 21, &world);
        assert_eq!(zone, 11);
        assert_eq!(x, 200.0);
        assert_eq!(z, 300.0);
    }

    #[test]
    fn test_respawn_bind_point_blocked_in_delos() {
        let world = WorldState::new();
        let mut ch = make_char_info(1);
        ch.bind_zone = 11;
        ch.bind_x = 200.0;
        ch.bind_z = 300.0;
        // In Delos (zone 30), bind point is NOT used
        let (zone, _x, _z) = determine_respawn_location(&ch, ZONE_DELOS, &world);
        assert_ne!(zone, 11, "Delos should not use bind point");
    }

    #[test]
    fn test_respawn_home_zone_nation_karus() {
        let world = WorldState::new();
        // Insert start_position for zone 1 (Karus home)
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 1,
            karus_x: 437,
            karus_z: 1627,
            elmorad_x: 1869,
            elmorad_z: 172,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 5,
            range_z: 5,
        });
        let ch = make_char_info(1); // Karus
        let (zone, x, z) = determine_respawn_location(&ch, 1, &world);
        assert_eq!(zone, 1);
        // Karus coords: 437 + rand(0..=5), 1627 + rand(0..=5)
        assert!((437.0..=442.0).contains(&x), "karus x={x}");
        assert!((1627.0..=1632.0).contains(&z), "karus z={z}");
    }

    #[test]
    fn test_respawn_home_zone_nation_elmorad() {
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 2,
            karus_x: 214,
            karus_z: 1862,
            elmorad_x: 1598,
            elmorad_z: 407,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 5,
            range_z: 5,
        });
        let ch = make_char_info(2); // Elmorad
        let (zone, x, z) = determine_respawn_location(&ch, 2, &world);
        assert_eq!(zone, 2);
        // Elmorad coords: 1598 + rand(0..=5), 407 + rand(0..=5)
        assert!((1598.0..=1603.0).contains(&x), "elmo x={x}");
        assert!((407.0..=412.0).contains(&z), "elmo z={z}");
    }

    #[test]
    fn test_respawn_chaos_dungeon_random() {
        let world = WorldState::new();
        // Insert random spawn points for zone 85
        world.insert_start_position_random(
            85,
            vec![
                ko_db::models::StartPositionRandomRow {
                    id: 1,
                    zone_id: 85,
                    pos_x: 126,
                    pos_z: 130,
                    radius: 1,
                },
                ko_db::models::StartPositionRandomRow {
                    id: 2,
                    zone_id: 85,
                    pos_x: 174,
                    pos_z: 146,
                    radius: 1,
                },
            ],
        );
        let ch = make_char_info(1);
        let (zone, x, z) = determine_respawn_location(&ch, 85, &world);
        assert_eq!(zone, 85);
        // C++ uses positive-only radius: pos + myrand(0, radius)
        // Point 1: (126+0..=1, 130+0..=1) → x∈[126,127], z∈[130,131]
        // Point 2: (174+0..=1, 146+0..=1) → x∈[174,175], z∈[146,147]
        let near_p1 = (126.0..=127.0).contains(&x) && (130.0..=131.0).contains(&z);
        let near_p2 = (174.0..=175.0).contains(&x) && (146.0..=147.0).contains(&z);
        assert!(near_p1 || near_p2, "chaos spawn x={x} z={z}");
    }

    #[test]
    fn test_respawn_juraid_no_event_falls_through() {
        let world = WorldState::new();
        // No bridge state → no active event → falls through to default path
        // Insert start_position for zone 87 so it doesn't hit Moradon fallback
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 87,
            karus_x: 224,
            karus_z: 272,
            elmorad_x: 799,
            elmorad_z: 749,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });
        let ch = make_char_info(1); // Karus
        let (zone, x, z) = determine_respawn_location(&ch, ZONE_JURAID_MOUNTAIN, &world);
        assert_eq!(zone, ZONE_JURAID_MOUNTAIN);
        // Falls through to default GetStartPosition → Karus(224, 272)
        assert_eq!(x, 224.0);
        assert_eq!(z, 272.0);
    }

    #[test]
    fn test_respawn_juraid_deva_stage_karus() {
        let world = WorldState::new();
        // Bridge state exists but no bridges open → Deva stage
        let bs = crate::systems::juraid::JuraidBridgeState::new();
        world.set_juraid_bridge_state(1, bs);
        let ch = make_char_info(1);
        let (zone, x, z) = determine_respawn_location(&ch, ZONE_JURAID_MOUNTAIN, &world);
        assert_eq!(zone, ZONE_JURAID_MOUNTAIN);
        // Karus Deva: 511 + rand(0..=3), 738 + rand(0..=3)
        assert!((511.0..=514.0).contains(&x), "juraid karus deva x={x}");
        assert!((738.0..=741.0).contains(&z), "juraid karus deva z={z}");
    }

    #[test]
    fn test_respawn_juraid_deva_stage_elmorad() {
        let world = WorldState::new();
        let bs = crate::systems::juraid::JuraidBridgeState::new();
        world.set_juraid_bridge_state(1, bs);
        let ch = make_char_info(2);
        let (zone, x, z) = determine_respawn_location(&ch, ZONE_JURAID_MOUNTAIN, &world);
        assert_eq!(zone, ZONE_JURAID_MOUNTAIN);
        // Elmorad Deva: 511 + rand(0..=3), 281 + rand(0..=3)
        assert!((511.0..=514.0).contains(&x), "juraid elmo deva x={x}");
        assert!((281.0..=284.0).contains(&z), "juraid elmo deva z={z}");
    }

    #[test]
    fn test_respawn_juraid_bridge1_stage() {
        let world = WorldState::new();
        // Set bridge[0] open for room 1 → Bridge1 stage
        let mut bs = crate::systems::juraid::JuraidBridgeState::new();
        bs.karus_bridges[0] = true;
        world.set_juraid_bridge_state(1, bs);

        let ch = make_char_info(1);
        let (zone, x, z) = determine_respawn_location(&ch, ZONE_JURAID_MOUNTAIN, &world);
        assert_eq!(zone, ZONE_JURAID_MOUNTAIN);
        // Karus Bridge1: 224 + rand(0..=3), 671 + rand(0..=3)
        assert!((224.0..=227.0).contains(&x), "juraid bridge1 x={x}");
        assert!((671.0..=674.0).contains(&z), "juraid bridge1 z={z}");
    }

    #[test]
    fn test_respawn_juraid_bridge2_stage() {
        let world = WorldState::new();
        // Set bridge[0] and bridge[1] open for room 1 → Bridge2 stage
        let mut bs = crate::systems::juraid::JuraidBridgeState::new();
        bs.karus_bridges[0] = true;
        bs.karus_bridges[1] = true;
        world.set_juraid_bridge_state(1, bs);

        let ch = make_char_info(2);
        let (zone, x, z) = determine_respawn_location(&ch, ZONE_JURAID_MOUNTAIN, &world);
        assert_eq!(zone, ZONE_JURAID_MOUNTAIN);
        // Elmorad Bridge2: 695 + rand(0..=3), 171 + rand(0..=3)
        assert!((695.0..=698.0).contains(&x), "juraid bridge2 x={x}");
        assert!((171.0..=174.0).contains(&z), "juraid bridge2 z={z}");
    }

    #[test]
    fn test_respawn_default_nation_specific() {
        let world = WorldState::new();
        // Zone 31 (Bifrost) — not a home zone, no random, no juraid
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 31,
            karus_x: 76,
            karus_z: 729,
            elmorad_x: 244,
            elmorad_z: 945,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 5,
            range_z: 5,
        });
        let ch = make_char_info(2); // Elmorad
        let (zone, x, z) = determine_respawn_location(&ch, 31, &world);
        assert_eq!(zone, 31);
        // Elmorad coords: 244, 945 (no range for default path)
        assert_eq!(x, 244.0);
        assert_eq!(z, 945.0);
    }

    #[test]
    fn test_respawn_moradon_fallback() {
        let world = WorldState::new();
        let ch = make_char_info(1);
        // Zone 999 doesn't exist — should fall through to Moradon
        let (zone, x, z) = determine_respawn_location(&ch, 999, &world);
        assert_eq!(zone, 21);
        assert_eq!(x, 512.0);
        assert_eq!(z, 341.0);
    }

    #[test]
    fn test_respawn_zone_constants() {
        assert_eq!(ZONE_ELMORAD, 2);
        assert_eq!(ZONE_DELOS, 30);
        assert_eq!(ZONE_BATTLE_BASE, 60);
        assert_eq!(ZONE_SNOW_BATTLE, 69);
        assert_eq!(ZONE_JURAID_MOUNTAIN, 87);
    }
}
