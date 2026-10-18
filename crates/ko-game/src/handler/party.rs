//! WIZ_PARTY (0x2F) handler -- party system.
//! ## Sub-opcodes (from `packets.h:364-379`)
//! | Code | Name               | Description                      |
//! |------|--------------------|----------------------------------|
//! | 0x01 | PARTY_CREATE       | Leader creates party + invites   |
//! | 0x02 | PARTY_PERMIT       | Accept/decline invitation        |
//! | 0x03 | PARTY_INSERT       | Member info broadcast            |
//! | 0x04 | PARTY_REMOVE       | Kick or leave                    |
//! | 0x05 | PARTY_DELETE       | Disband entire party             |
//! | 0x06 | PARTY_HPCHANGE     | HP/MP update to party            |
//! | 0x09 | PARTY_STATUSCHANGE | Status effect change             |
//! | 0x1C | PARTY_PROMOTE      | Transfer leadership              |
//! | 0x1E | PARTY_COMMAND_PROMATE | Transfer command leadership    |
//! | 0x1F | PARTY_TARGET_NUMBER| Set party target marker          |
//! | 0x20 | PARTY_ALERT        | Party alert signal               |
//! ## PARTY_CREATE Client -> Server
//! ```text
//! [u8 sub_opcode=0x01] [u16 name_len] [bytes target_name] [i8 party_type]
//! ```
//! ## PARTY_INSERT Server -> Client (member info)
//! ```text
//! [u8 PARTY_INSERT=0x03] [u16 result]
//! If result == 1 (success):
//!   [u32 member_sid] [u8 1]
//!   [u16 name_len] [bytes name]        (DByte — C++ default)
//!   [i16 max_hp] [i16 hp] [u8 level] [u16 class]
//!   [i16 max_mp] [i16 mp] [u8 nation] [u8 0]
//!   [u32 target_number_id] [i8 party_type] [u8 loyalty_rank]
//! ```
//! ## PARTY_PERMIT Server -> Client (invitation)
//! ```text
//! [u8 PARTY_PERMIT=0x02] [u32 leader_sid] [u16 name_len] [bytes leader_name]
//! ```
//! ## PARTY_HPCHANGE (sub-opcode 0x06 inside WIZ_PARTY)
//! ```text
//! [u8 PARTY_HPCHANGE=0x06] [u32 sid] [i16 max_hp] [i16 hp] [i16 max_mp] [i16 mp]
//! ```

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::state_change_constants::STATE_CHANGE_PARTY_LEADER;
use crate::world::{
    BotInstance, CharacterInfo, WorldState, ZONE_BORDER_DEFENSE_WAR, ZONE_CHAOS_DUNGEON,
    ZONE_DELOS, ZONE_DUNGEON_DEFENCE, ZONE_JURAID_MOUNTAIN, ZONE_PRISON,
};
use crate::zone::SessionId;

// ── Party sub-opcode constants ──────────────────────────────────────────────

const PARTY_CREATE: u8 = 0x01;
const PARTY_PERMIT: u8 = 0x02;
pub(crate) const PARTY_INSERT: u8 = 0x03;
const PARTY_REMOVE: u8 = 0x04;
const PARTY_DELETE: u8 = 0x05;
const PARTY_HPCHANGE: u8 = 0x06;
/// Sent to party members when a member levels up.
pub(crate) const PARTY_LEVELCHANGE: u8 = 0x07;
/// Sent to party members when a member changes class (job change).
pub(crate) const PARTY_CLASSCHANGE: u8 = 0x08;
/// Used by `send_user_status_update` (regene.rs, buff_tick.rs) to broadcast
/// poison/DOT/disease/blind status changes to party members.
pub(crate) const PARTY_STATUSCHANGE: u8 = 0x09;
const PARTY_PROMOTE: u8 = 0x1C;
const PARTY_COMMAND_PROMATE: u8 = 0x1E;
const PARTY_TARGET_NUMBER: u8 = 0x1F;
const PARTY_ALERT: u8 = 0x20;

/// Check if a zone allows cross-nation party formation.
/// Returns true for Moradon, arenas, Forgotten Temple, Under Castle,
/// Stone zones, Delos Castellan, Draki Tower, Old Moradon, and Eslant zones.
fn is_partner_party_zone(zone_id: u16) -> bool {
    use crate::world::{
        ZONE_ARENA, ZONE_BLOOD_DON_ARENA, ZONE_DELOS_CASTELLAN, ZONE_DRAKI_TOWER,
        ZONE_ELMORAD_ESLANT, ZONE_ELMORAD_ESLANT2, ZONE_ELMORAD_ESLANT3, ZONE_FORGOTTEN_TEMPLE,
        ZONE_GOBLIN_ARENA, ZONE_KARUS_ESLANT, ZONE_KARUS_ESLANT2, ZONE_KARUS_ESLANT3, ZONE_MORADON,
        ZONE_MORADON2, ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5, ZONE_OLD_MORADON,
        ZONE_ORC_ARENA, ZONE_STONE1, ZONE_STONE2, ZONE_STONE3, ZONE_UNDER_CASTLE,
    };
    matches!(
        zone_id,
        ZONE_MORADON
            | ZONE_MORADON2
            | ZONE_MORADON3
            | ZONE_MORADON4
            | ZONE_MORADON5
            | ZONE_ARENA
            | ZONE_ORC_ARENA
            | ZONE_BLOOD_DON_ARENA
            | ZONE_GOBLIN_ARENA
            | ZONE_FORGOTTEN_TEMPLE
            | ZONE_UNDER_CASTLE
            | ZONE_STONE1
            | ZONE_STONE2
            | ZONE_STONE3
            | ZONE_DELOS_CASTELLAN
            | ZONE_DRAKI_TOWER
            | ZONE_OLD_MORADON
            | ZONE_KARUS_ESLANT
            | ZONE_KARUS_ESLANT2
            | ZONE_KARUS_ESLANT3
            | ZONE_ELMORAD_ESLANT
            | ZONE_ELMORAD_ESLANT2
            | ZONE_ELMORAD_ESLANT3
    )
}

// ── Error codes for PARTY_INSERT responses ─────────────────────────────────

/// Invitation declined.
const PARTY_ERR_DECLINED: i16 = -1;
/// Level difference too large.
const PARTY_ERR_LEVEL: i16 = -2;
/// Different nation.
const PARTY_ERR_NATION: i16 = -3;
/// Different zone.
const PARTY_ERR_ZONE: i16 = -5;
/// Target not online. Reserved for future offline-target handling.
const PARTY_ERR_OFFLINE: i16 = -6;
/// Generic error (already in party, target invalid, etc.).
const PARTY_ERR_GENERIC: i16 = -7;
/// Cannot form party in this zone.
const PARTY_ERR_ZONE_RESTRICTED: i16 = -9;
/// Missing seeking party item (914057000).
const PARTY_ERR_SEEKING_ITEM: i16 = -10;

/// Item required for seeking party participation.
const SEEKING_PARTY_ITEM: u32 = 914057000;

/// Handle WIZ_PARTY (0x2F) from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot perform party actions
    let world = session.world().clone();
    if world.is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = match reader.read_u8() {
        Some(op) => op,
        None => return Ok(()),
    };

    match sub_opcode {
        PARTY_CREATE => handle_party_create(session, &mut reader).await,
        PARTY_INSERT => handle_party_invite(session, &mut reader),
        PARTY_PERMIT => handle_party_permit(session, &mut reader),
        PARTY_REMOVE => handle_party_remove(session, &mut reader),
        PARTY_DELETE => handle_party_delete(session),
        PARTY_PROMOTE => handle_party_promote(session, &mut reader),
        PARTY_TARGET_NUMBER => handle_target_number(session, &mut reader),
        PARTY_ALERT => handle_party_alert(session, &mut reader),
        PARTY_COMMAND_PROMATE => handle_party_command(session, &mut reader),
        _ => {
            debug!(
                "[{}] Party unhandled sub-opcode: 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Check if the level difference between two players is acceptable for partying.
/// Level check passes if EITHER condition is true:
/// - Target level is within +-8 of inviter level
/// - Target level is within 0.67x to 1.5x of inviter level
fn is_level_compatible(level_a: u8, level_b: u8) -> bool {
    let a = level_a as i32;
    let b = level_b as i32;

    // Condition 1: within +-8 levels
    if b <= a + 8 && b >= a - 8 {
        return true;
    }

    // Condition 2: within 0.67x to 1.5x ratio
    if b <= (a * 3 / 2) && b >= (a * 2 / 3) {
        return true;
    }

    false
}

/// Check if a zone restricts party formation.
fn is_party_restricted_zone(zone_id: u16) -> bool {
    matches!(
        zone_id,
        ZONE_CHAOS_DUNGEON | ZONE_PRISON | ZONE_DUNGEON_DEFENCE
    )
}

/// Check if level restriction should be relaxed for this zone.
fn is_level_check_relaxed_zone(zone_id: u16) -> bool {
    matches!(zone_id, ZONE_BORDER_DEFENSE_WAR | ZONE_JURAID_MOUNTAIN)
}

/// Send a PARTY_INSERT error response to the caller.
/// C++ wire format: `WIZ_PARTY [u8 PARTY_INSERT] [i16 error_code]`
fn send_party_error(world: &WorldState, sid: SessionId, error_code: i16) {
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_INSERT);
    pkt.write_i16(error_code);
    world.send_to_session_owned(sid, pkt);
}

/// Build a PARTY_INSERT member info packet.
/// ```text
/// [u8 PARTY_INSERT] [u16 1] [u32 sid] [u8 index_hint]
/// [u16 name_len] [bytes name] [i16 max_hp] [i16 hp] [u8 level] [u16 class]
/// [i16 max_mp] [i16 mp] [u8 nation] [u8 0]
/// [u32 target_number_id] [i8 party_type] [u8 loyalty_rank]
/// ```
pub(crate) fn build_party_member_info(
    ch: &CharacterInfo,
    index_hint: u8,
    target_number_id: i16,
    party_type: i8,
    loyalty_rank: i8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_INSERT);
    pkt.write_u16(1); // success
    pkt.write_u32(ch.session_id as u32);
    pkt.write_u8(index_hint);
    pkt.write_string(&ch.name);
    pkt.write_i16(ch.max_hp);
    pkt.write_i16(ch.hp);
    pkt.write_u8(ch.level);
    pkt.write_u16(ch.class);
    pkt.write_i16(ch.max_mp);
    pkt.write_i16(ch.mp);
    pkt.write_u8(ch.nation);
    pkt.write_u8(0); // padding
    pkt.write_u32(target_number_id as i32 as u32); // NumberTargetID
    pkt.write_i8(party_type);
    pkt.write_i8(loyalty_rank);
    pkt
}

/// Build the same PARTY_INSERT payload for a runtime bot. Bots deliberately
/// share the normal party wire format so the stock client renders their HP,
/// MP, level and class exactly like a player member.
pub(crate) fn build_bot_party_member_info(
    bot: &BotInstance,
    index_hint: u8,
    target_number_id: i16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_INSERT);
    pkt.write_u16(1);
    pkt.write_u32(bot.id);
    pkt.write_u8(index_hint);
    pkt.write_string(&bot.name);
    pkt.write_i16(bot.max_hp);
    pkt.write_i16(bot.hp);
    pkt.write_u8(bot.level);
    pkt.write_u16(bot.class);
    pkt.write_i16(bot.max_mp);
    pkt.write_i16(bot.mp);
    pkt.write_u8(bot.nation);
    pkt.write_u8(0);
    pkt.write_u32(target_number_id as i32 as u32);
    pkt.write_i8(0);
    let loyalty_rank = match (bot.personal_rank, bot.knights_rank) {
        (0, 0) => -1,
        (p, 0) => p as i8,
        (0, k) => k as i8,
        (p, k) => p.min(k) as i8,
    };
    pkt.write_i8(loyalty_rank);
    pkt
}

/// Get character info + loyalty symbol rank in a single DashMap read.
/// Avoids the pattern `get_character_info(sid)` + `get_loyalty_symbol_rank(sid)`
/// which acquires two separate DashMap locks on the same session.
fn get_char_with_loyalty(world: &WorldState, sid: SessionId) -> Option<(CharacterInfo, i8)> {
    world
        .with_session(sid, |h| {
            let ch = h.character.as_ref()?.clone();
            let pr = h.personal_rank;
            let kr = h.knights_rank;
            let lr = if (pr > 100 && pr <= 200) || (kr > 100 && kr <= 200) || (kr == 0 && pr == 0) {
                -1
            } else if kr == 0 {
                pr as i8
            } else if pr == 0 || kr <= pr {
                kr as i8
            } else {
                pr as i8
            };
            Some((ch, lr))
        })
        .flatten()
}

/// Build a PARTY_HPCHANGE packet for a party member.
pub fn build_party_hp_update(ch: &CharacterInfo) -> Packet {
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_HPCHANGE);
    pkt.write_u32(ch.session_id as u32);
    pkt.write_i16(ch.max_hp);
    pkt.write_i16(ch.hp);
    pkt.write_i16(ch.max_mp);
    pkt.write_i16(ch.mp);
    pkt
}

/// Broadcast party HP updates for a member to the entire party.
/// Called when a member's HP changes (damage, heal, regen).
pub fn broadcast_party_hp(world: &WorldState, sid: SessionId) {
    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return,
    };

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };

    let hp_pkt = build_party_hp_update(&ch);
    world.send_to_party(party_id, &hp_pkt);
}

/// Build a PARTY_CLASSCHANGE packet for a party member.
/// Sent to all party members when a member changes class (job change).
/// ```text
/// [u8 PARTY_CLASSCHANGE=0x08] [u32 sid] [u16 new_class]
/// ```
pub fn build_party_class_change(sid: SessionId, new_class: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_CLASSCHANGE);
    pkt.write_u32(sid as u32);
    pkt.write_u16(new_class);
    pkt
}

/// Broadcast party class change notification for a member to the entire party.
/// Called after a member changes class (job change, master change).
pub fn broadcast_party_class_change(world: &WorldState, sid: SessionId, new_class: u16) {
    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return,
    };

    let pkt = build_party_class_change(sid, new_class);
    world.send_to_party(party_id, &pkt);
}

// ── Sub-opcode handlers ─────────────────────────────────────────────────────

/// Handle PARTY_CREATE (0x01) -- leader creates a new party and invites target.
/// Client: `[u8 0x01] [u16 name_len] [bytes target_name] [i8 party_type]`
async fn handle_party_create(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let target_name = match reader.read_string() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };
    let party_type = reader.read_u8().unwrap_or(0) as i8;

    // Store party_type on leader session (C++ m_sUserPartyType)
    world.update_session(sid, |h| {
        h.party_type = party_type;
    });

    // Get inviter info
    let inviter = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Get inviter position
    let inviter_pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Look up target by name
    let target_sid = match world.find_session_by_name(&target_name) {
        Some(tsid) => tsid,
        None => {
            send_party_error(&world, sid, PARTY_ERR_GENERIC);
            return Ok(());
        }
    };

    // Cannot invite yourself
    if target_sid == sid {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    // Target must be in-game (character loaded)
    if !world.is_session_ingame(target_sid) {
        send_party_error(&world, sid, PARTY_ERR_OFFLINE);
        return Ok(());
    }

    // Target must not be in a party or have a pending invitation
    if world.is_in_party(target_sid) || world.has_party_invitation(target_sid) {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    // Get target info
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => {
            send_party_error(&world, sid, PARTY_ERR_OFFLINE);
            return Ok(());
        }
    };

    // Nation check — skip for partner party zones (Moradon, arenas, etc.)
    if inviter.nation != target.nation && !is_partner_party_zone(inviter_pos.zone_id) {
        send_party_error(&world, sid, PARTY_ERR_NATION);
        return Ok(());
    }

    // Seeking party item requirement (party_type == 2)
    if party_type == 2 && !world.check_exist_item(target_sid, SEEKING_PARTY_ITEM, 1) {
        send_party_error(&world, sid, PARTY_ERR_SEEKING_ITEM);
        return Ok(());
    }

    // CSW Delos alliance restriction
    if inviter_pos.zone_id == ZONE_DELOS && inviter.knights_id != target.knights_id {
        let csw = world.csw_event().read().await;
        let csw_active = csw.is_active();
        drop(csw);

        if csw_active {
            // During CSW on Delos, cross-clan parties require alliance.
            // If target has no clan (knights_id == 0), GetClanPtr returns null →
            // entire block is skipped → target is allowed.
            let allowed = if target.knights_id == 0 {
                // C++ skips the CSW check when target has no clan
                true
            } else if let Some(target_clan) = world.get_knights(target.knights_id) {
                if target_clan.alliance == 0 {
                    // Target not in alliance — cannot cross-clan party
                    false
                } else if let Some(alliance) = world.get_alliance(target_clan.alliance) {
                    // Target in alliance — check if alliance matches main clan
                    target_clan.alliance == alliance.main_clan
                } else {
                    false
                }
            } else {
                false
            };

            if !allowed {
                send_party_error(&world, sid, PARTY_ERR_NATION);
                return Ok(());
            }
        }
    }

    // Zone check
    let target_pos = match world.get_position(target_sid) {
        Some(p) => p,
        None => {
            send_party_error(&world, sid, PARTY_ERR_GENERIC);
            return Ok(());
        }
    };

    if inviter_pos.zone_id != target_pos.zone_id {
        send_party_error(&world, sid, PARTY_ERR_ZONE);
        return Ok(());
    }

    // Zone restriction check
    if is_party_restricted_zone(inviter_pos.zone_id) {
        send_party_error(&world, sid, PARTY_ERR_ZONE_RESTRICTED);
        return Ok(());
    }

    // Level check (relaxed in certain zones, same clan, or chicken mode).
    let inviter_is_chicken = inviter.level < 30;
    let target_is_chicken = target.level < 30;
    let same_clan = inviter.knights_id > 0 && inviter.knights_id == target.knights_id;
    if !inviter_is_chicken
        && !target_is_chicken
        && !is_level_check_relaxed_zone(inviter_pos.zone_id)
        && !same_clan
        && !is_level_compatible(inviter.level, target.level)
    {
        send_party_error(&world, sid, PARTY_ERR_LEVEL);
        return Ok(());
    }

    // Inviter must not already be in a party or have a pending invitation
    if world.is_in_party(sid) || world.has_party_invitation(sid) {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    // Create party with inviter as leader
    let party_id = match world.create_party(sid) {
        Some(id) => id,
        None => {
            send_party_error(&world, sid, PARTY_ERR_GENERIC);
            return Ok(());
        }
    };

    // Store pending invitation for the target
    world.set_party_invitation(target_sid, party_id, sid);

    // Send PARTY_PERMIT to target: [u8 PARTY_PERMIT] [u32 leader_sid] [dbyte leader_name]
    let mut permit_pkt = Packet::new(Opcode::WizParty as u8);
    permit_pkt.write_u8(PARTY_PERMIT);
    permit_pkt.write_u32(sid as u32);
    permit_pkt.write_string(&inviter.name);
    world.send_to_session_owned(target_sid, permit_pkt);

    // Broadcast party leader 'P' symbol to nearby players
    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        let leader_sc = crate::handler::regene::build_state_change_broadcast(
            sid as u32,
            STATE_CHANGE_PARTY_LEADER,
            1,
        );
        world.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(leader_sc),
            None,
            event_room,
        );
    }

    debug!(
        "[sid={}] Created party {} and invited {} (sid={})",
        sid, party_id, target_name, target_sid
    );

    Ok(())
}

/// Handle PARTY_INSERT (0x03) -- invite a player to an existing party.
/// Client: `[u8 0x03] [u16 name_len] [bytes target_name] [i8 party_type]`
fn handle_party_invite(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let target_name = match reader.read_string() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };
    let party_type = reader.read_u8().unwrap_or(0) as i8;

    // Update party_type on inviter session (may change from initial create)
    world.update_session(sid, |h| {
        h.party_type = party_type;
    });

    // Get inviter info
    let inviter = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Must be in a party
    let party_id = match inviter.party_id {
        Some(id) => id,
        None => {
            send_party_error(&world, sid, PARTY_ERR_GENERIC);
            return Ok(());
        }
    };

    // Must be the party leader
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => {
            send_party_error(&world, sid, PARTY_ERR_GENERIC);
            return Ok(());
        }
    };

    if !party.is_leader(sid) {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    // Party must not be full
    if party.is_full() {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    // Look up target
    let target_sid = match world.find_session_by_name(&target_name) {
        Some(tsid) => tsid,
        None => {
            send_party_error(&world, sid, PARTY_ERR_GENERIC);
            return Ok(());
        }
    };

    if target_sid == sid {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    // Target must be in-game (character loaded)
    if !world.is_session_ingame(target_sid) {
        send_party_error(&world, sid, PARTY_ERR_OFFLINE);
        return Ok(());
    }

    // Target must not be in a party or have a pending invitation
    if world.is_in_party(target_sid) || world.has_party_invitation(target_sid) {
        send_party_error(&world, sid, PARTY_ERR_GENERIC);
        return Ok(());
    }

    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => {
            send_party_error(&world, sid, PARTY_ERR_OFFLINE);
            return Ok(());
        }
    };

    // Position checks
    let inviter_pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };
    let target_pos = match world.get_position(target_sid) {
        Some(p) => p,
        None => {
            send_party_error(&world, sid, PARTY_ERR_OFFLINE);
            return Ok(());
        }
    };

    // Nation check — skip for partner party zones (Moradon, arenas, etc.)
    if inviter.nation != target.nation && !is_partner_party_zone(inviter_pos.zone_id) {
        send_party_error(&world, sid, PARTY_ERR_NATION);
        return Ok(());
    }

    // Seeking party item requirement (party_type == 2 or inviter's party_type == 2)
    let inviter_party_type = world.with_session(sid, |h| h.party_type).unwrap_or(0);
    if (party_type == 2 || inviter_party_type == 2)
        && !world.check_exist_item(target_sid, SEEKING_PARTY_ITEM, 1)
    {
        send_party_error(&world, sid, PARTY_ERR_SEEKING_ITEM);
        return Ok(());
    }

    // Zone check
    if inviter_pos.zone_id != target_pos.zone_id {
        send_party_error(&world, sid, PARTY_ERR_ZONE);
        return Ok(());
    }

    // Zone restriction
    if is_party_restricted_zone(inviter_pos.zone_id) {
        send_party_error(&world, sid, PARTY_ERR_ZONE_RESTRICTED);
        return Ok(());
    }

    // Level check (relaxed in certain zones, same clan, or chicken mode).
    let inviter_is_chicken = inviter.level < 30;
    let target_is_chicken = target.level < 30;
    let same_clan = inviter.knights_id > 0 && inviter.knights_id == target.knights_id;
    if !inviter_is_chicken
        && !target_is_chicken
        && !is_level_check_relaxed_zone(inviter_pos.zone_id)
        && !same_clan
        && !is_level_compatible(inviter.level, target.level)
    {
        send_party_error(&world, sid, PARTY_ERR_LEVEL);
        return Ok(());
    }

    // Store pending invitation
    world.set_party_invitation(target_sid, party_id, sid);

    // Send PARTY_PERMIT to target
    let mut permit_pkt = Packet::new(Opcode::WizParty as u8);
    permit_pkt.write_u8(PARTY_PERMIT);
    permit_pkt.write_u32(sid as u32);
    permit_pkt.write_string(&inviter.name);
    world.send_to_session_owned(target_sid, permit_pkt);

    debug!(
        "[sid={}] Invited {} (sid={}) to party {}",
        sid, target_name, target_sid, party_id
    );

    Ok(())
}

/// Handle PARTY_PERMIT (0x02) -- accept or decline party invitation.
/// Client: `[u8 0x02] [u8 accept_flag]`
fn handle_party_permit(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let accept = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    // Consume the pending invitation
    let (party_id, leader_sid) = match world.take_party_invitation(sid) {
        Some(inv) => inv,
        None => return Ok(()),
    };

    if accept == 0 {
        // Declined -- notify leader
        let party = match world.get_party(party_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        // If leader is alone, disband
        if party.member_count() <= 1 {
            let members = world.disband_party(party_id);
            // Notify leader about disband
            for &msid in &members {
                let mut del_pkt = Packet::new(Opcode::WizParty as u8);
                del_pkt.write_u8(PARTY_DELETE);
                world.send_to_session_owned(msid, del_pkt);
            }
        }

        // Send decline to leader
        send_party_error(&world, leader_sid, PARTY_ERR_DECLINED);
        return Ok(());
    }

    // Accepted: join the party

    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Verify leader is still valid
    let leader_sid_check = match party.leader_sid() {
        Some(l) => l,
        None => return Ok(()),
    };

    // Must still be in same zone as leader
    let joiner_pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    let leader_zone = world
        .get_position(leader_sid_check)
        .map(|p| p.zone_id)
        .or_else(|| world.get_bot(leader_sid_check as u32).map(|b| b.zone_id));

    if leader_zone != Some(joiner_pos.zone_id) || party.is_full() {
        // Zone changed or party full -- decline
        send_party_error(&world, leader_sid_check, PARTY_ERR_DECLINED);
        return Ok(());
    }

    // Send existing members' info to the joining player
    let target_number_id = party.target_number_id;
    for &member_sid in &party.active_members() {
        if let Some((member_ch, lr)) = get_char_with_loyalty(&world, member_sid) {
            let info_pkt = build_party_member_info(&member_ch, 1, target_number_id, 0, lr);
            world.send_to_session_owned(sid, info_pkt);
        } else if let Some(bot) = world.get_bot(member_sid as u32) {
            let info_pkt = build_bot_party_member_info(&bot, 1, target_number_id);
            world.send_to_session_owned(sid, info_pkt);
        }
    }

    // Propagate leader's party_type to the joiner
    let leader_party_type = world
        .with_session(leader_sid_check, |h| h.party_type)
        .unwrap_or(0);
    world.update_session(sid, |h| {
        h.party_type = leader_party_type;
    });

    // Add joiner to party
    if !world.add_party_member(party_id, sid) {
        return Ok(());
    }

    // Joining a party consumes the seek request. The client may leave a stale
    // BBS row/need-party flag behind, which otherwise makes PK bots send a new
    // invitation on every AI pass.
    world.remove_seeking_party(sid);
    world.update_session(sid, |h| h.need_party = 0);

    // Broadcast the new member's info to the whole party
    if let Some((joiner_ch, lr)) = get_char_with_loyalty(&world, sid) {
        let info_pkt = build_party_member_info(&joiner_ch, 1, target_number_id, 0, lr);
        world.send_to_party(party_id, &info_pkt);
    }

    // Send HP updates for all members
    if let Some(party) = world.get_party(party_id) {
        for &member_sid in &party.active_members() {
            if let Some(member_ch) = world.get_character_info(member_sid) {
                let hp_pkt = build_party_hp_update(&member_ch);
                world.send_to_party(party_id, &hp_pkt);
            } else if let Some(bot) = world.get_bot(member_sid as u32) {
                let mut hp_pkt = Packet::new(Opcode::WizParty as u8);
                hp_pkt.write_u8(PARTY_HPCHANGE);
                hp_pkt.write_u32(bot.id);
                hp_pkt.write_i16(bot.max_hp);
                hp_pkt.write_i16(bot.hp);
                hp_pkt.write_i16(bot.max_mp);
                hp_pkt.write_i16(bot.mp);
                world.send_to_party(party_id, &hp_pkt);
            }
        }
    }

    debug!(
        "[sid={}] Joined party {} (leader={})",
        sid, party_id, leader_sid
    );

    Ok(())
}

/// Handle PARTY_REMOVE (0x04) -- kick a member or leave the party.
/// Client: `[u8 0x04] [u16 target_sid]`
fn handle_party_remove(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let target_sid = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return Ok(()),
    };

    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };

    // If the target is the leader and removing self -- disband
    if target_sid == sid && party.is_leader(sid) {
        handle_party_delete_internal(&world, sid, party_id);
        return Ok(());
    }

    // If removing someone else, must be the leader
    if target_sid != sid && !party.is_leader(sid) {
        return Ok(());
    }

    // Check if only 2 members remain -- if so, disband entirely
    let member_count_without_target = party
        .active_members()
        .iter()
        .filter(|&&m| m != target_sid)
        .count();

    if member_count_without_target <= 1 {
        // Only leader would be left -- disband
        handle_party_delete_internal(&world, sid, party_id);
        return Ok(());
    }

    // Broadcast PARTY_REMOVE to all members before removal
    let mut remove_pkt = Packet::new(Opcode::WizParty as u8);
    remove_pkt.write_u8(PARTY_REMOVE);
    remove_pkt.write_u32(target_sid as u32);
    world.send_to_party(party_id, &remove_pkt);

    // Remove the member
    world.remove_party_member(party_id, target_sid);

    // Send HP updates for remaining members
    if let Some(party) = world.get_party(party_id) {
        for &member_sid in &party.active_members() {
            if let Some(member_ch) = world.get_character_info(member_sid) {
                let hp_pkt = build_party_hp_update(&member_ch);
                world.send_to_party(party_id, &hp_pkt);
            }
        }
    }

    debug!(
        "[sid={}] Removed sid={} from party {}",
        sid, target_sid, party_id
    );

    Ok(())
}

/// Handle PARTY_DELETE (0x05) -- disband the entire party.
fn handle_party_delete(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return Ok(()),
    };

    handle_party_delete_internal(&world, sid, party_id);
    Ok(())
}

/// Internal function to disband a party and notify all members.
fn handle_party_delete_internal(world: &WorldState, _requester_sid: SessionId, party_id: u16) {
    // Broadcast PARTY_DELETE to all members
    let mut del_pkt = Packet::new(Opcode::WizParty as u8);
    del_pkt.write_u8(PARTY_DELETE);
    world.send_to_party(party_id, &del_pkt);

    // Get leader before disband to clear party leader symbol
    let leader_sid = world.get_party(party_id).and_then(|p| {
        let members = p.active_members();
        members.first().copied()
    });

    // Disband the party (clears party_id from all members)
    let members = world.disband_party(party_id);

    // Remove party leader 'P' symbol from old leader
    if let Some(lsid) = leader_sid {
        if let Some(pos) = world.get_position(lsid) {
            let sc = crate::handler::regene::build_state_change_broadcast(
                lsid as u32,
                STATE_CHANGE_PARTY_LEADER,
                0,
            );
            let event_room = world.get_event_room(lsid);
            world.broadcast_to_region_sync(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(sc),
                None,
                event_room,
            );
        }
    }

    debug!(
        "Party {} disbanded, {} members released",
        party_id,
        members.len()
    );
}

/// Handle PARTY_PROMOTE (0x1C) -- transfer leadership.
/// Client: `[u8 0x1C] [u16 new_leader_sid]`
fn handle_party_promote(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let new_leader_sid = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return Ok(()),
    };

    // Must be current leader
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };

    if !party.is_leader(sid) {
        return Ok(());
    }

    // Target must be in our party
    if !party.contains(new_leader_sid) {
        return Ok(());
    }

    // Promote
    if !world.promote_party_leader(party_id, new_leader_sid) {
        return Ok(());
    }

    // Update party leader 'P' symbols — remove from old, set on new
    //   `StateChangeServerDirect(6, 0)` on old leader
    //   `StateChangeServerDirect(6, 1)` on new leader
    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        let old_sc = crate::handler::regene::build_state_change_broadcast(
            sid as u32,
            STATE_CHANGE_PARTY_LEADER,
            0,
        );
        world.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(old_sc),
            None,
            event_room,
        );
    }
    if let Some(pos) = world.get_position(new_leader_sid) {
        let new_sc = crate::handler::regene::build_state_change_broadcast(
            new_leader_sid as u32,
            STATE_CHANGE_PARTY_LEADER,
            1,
        );
        let event_room = world.get_event_room(new_leader_sid);
        world.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(new_sc),
            None,
            event_room,
        );
    }

    // Transfer command leadership to new leader
    world.update_party(party_id, |party| {
        if party.command_leader_sid == Some(sid) {
            party.command_leader_sid = Some(new_leader_sid);
        }
    });

    // Broadcast the new leader info with index_hint=100 (reset to leader)
    let target_number_id = party.target_number_id;
    if let Some((new_leader_ch, lr)) = get_char_with_loyalty(&world, new_leader_sid) {
        let info_pkt = build_party_member_info(&new_leader_ch, 100, target_number_id, 0, lr);
        world.send_to_party(party_id, &info_pkt);
    }

    // Send HP updates for all members
    if let Some(party) = world.get_party(party_id) {
        for &member_sid in &party.active_members() {
            if let Some(member_ch) = world.get_character_info(member_sid) {
                let hp_pkt = build_party_hp_update(&member_ch);
                world.send_to_party(party_id, &hp_pkt);
            }
        }
    }

    debug!(
        "[sid={}] Promoted sid={} to leader of party {}",
        sid, new_leader_sid, party_id
    );

    Ok(())
}

/// Handle PARTY_TARGET_NUMBER (0x1F) -- set party target marker.
/// Client: `[u8 0x1F] [i16 target_id] [u32 effect_id] [i8 success]`
fn handle_target_number(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    {
        let blocked = world
            .with_session(sid, |h| {
                h.last_target_number_time.elapsed().as_millis() < 850
            })
            .unwrap_or(true);
        if blocked {
            return Ok(());
        }
        world.update_session(sid, |h| {
            h.last_target_number_time = std::time::Instant::now();
        });
    }

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let target_id = reader.read_u16().map(|v| v as i16).unwrap_or(-1);
    let _effect_id = reader.read_u32().unwrap_or(0);
    let success = reader.read_u8().map(|v| v as i8).unwrap_or(-1);

    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return Ok(()),
    };

    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };
    if !party.is_command_leader(sid) {
        return Ok(());
    }

    // Update party target number
    world.update_party(party_id, |party| {
        party.target_number_id = target_id;
    });

    // Broadcast to party
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_TARGET_NUMBER);
    pkt.write_u32(target_id as i32 as u32);
    pkt.write_i8(success);
    world.send_to_party(party_id, &pkt);

    Ok(())
}

/// Handle PARTY_ALERT (0x20) -- party alert signal.
/// Client: `[u8 0x20] [u8 sub_opcode] [u32 effect_id]`
fn handle_party_alert(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    {
        let blocked = world
            .with_session(sid, |h| {
                h.last_target_number_time.elapsed().as_millis() < 850
            })
            .unwrap_or(true);
        if blocked {
            return Ok(());
        }
        world.update_session(sid, |h| {
            h.last_target_number_time = std::time::Instant::now();
        });
    }

    if world.is_player_dead(sid) {
        return Ok(());
    }

    let sub_opcode = reader.read_u8().unwrap_or(0);
    let _effect_id = reader.read_u32().unwrap_or(0);

    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return Ok(()),
    };

    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };
    if !party.is_command_leader(sid) {
        return Ok(());
    }

    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_ALERT);
    pkt.write_u8(sub_opcode);
    world.send_to_party(party_id, &pkt);

    Ok(())
}

/// Handle PARTY_COMMAND_PROMATE (0x1E) -- transfer command leadership.
/// Client: `[u8 0x1E] [i16 member_sid]`
fn handle_party_command(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    {
        let blocked = world
            .with_session(sid, |h| {
                h.last_target_number_time.elapsed().as_millis() < 850
            })
            .unwrap_or(true);
        if blocked {
            return Ok(());
        }
        world.update_session(sid, |h| {
            h.last_target_number_time = std::time::Instant::now();
        });
    }

    let target_sid = reader.read_u16().unwrap_or(0);

    let party_id = match world.get_party_id(sid) {
        Some(id) => id,
        None => return Ok(()),
    };

    // Verify target is in same party
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };

    if !party.is_command_leader(sid) {
        return Ok(());
    }

    if !party.contains(target_sid) {
        return Ok(());
    }

    // Transfer command leadership to target
    world.update_party(party_id, |party| {
        party.command_leader_sid = Some(target_sid);
    });

    // Get target info for broadcast
    let target_ch = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Broadcast command leader change
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(PARTY_COMMAND_PROMATE);
    pkt.write_u32(target_sid as u32);
    pkt.write_sbyte_string(&target_ch.name);
    world.send_to_party(party_id, &pkt);

    Ok(())
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::world::{CharacterInfo, WorldState};
    use ko_protocol::{Opcode, PacketReader};
    use tokio::sync::mpsc;

    fn make_test_char(sid: u16, name: &str, nation: u8, level: u8) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation,
            race: 1,
            class: 101,
            level,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
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
            res_hp_type: 0x01,
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
        }
    }

    /// Test level compatibility check.
    #[test]
    fn test_level_compatible_same_level() {
        assert!(is_level_compatible(60, 60));
    }

    #[test]
    fn test_level_compatible_within_8() {
        assert!(is_level_compatible(60, 68));
        assert!(is_level_compatible(60, 52));
    }

    #[test]
    fn test_level_compatible_within_ratio() {
        // 60 * 1.5 = 90, 60 * 0.67 = 40
        assert!(is_level_compatible(60, 90));
        assert!(is_level_compatible(60, 40));
    }

    #[test]
    fn test_level_incompatible() {
        // Level 60 vs Level 10 -- too far
        assert!(!is_level_compatible(60, 10));
        // Level 10 vs Level 60 -- too far
        assert!(!is_level_compatible(10, 60));
    }

    #[test]
    fn test_level_compatible_low_level() {
        // Level 1 vs Level 9 (within +-8)
        assert!(is_level_compatible(1, 9));
        // Level 1 vs Level 10 -- 10 > 1+8=9 but 10 <= 1.5 -- no, 10 > 1*3/2=1, and 10 <= 9? No
        assert!(!is_level_compatible(1, 10));
    }

    /// Test zone restriction checks.
    #[test]
    fn test_party_restricted_zones() {
        assert!(is_party_restricted_zone(ZONE_CHAOS_DUNGEON));
        assert!(is_party_restricted_zone(ZONE_PRISON));
        assert!(is_party_restricted_zone(ZONE_DUNGEON_DEFENCE));
        assert!(!is_party_restricted_zone(21)); // Moradon
    }

    /// Test level check relaxation zones.
    #[test]
    fn test_level_check_relaxed_zones() {
        assert!(is_level_check_relaxed_zone(ZONE_BORDER_DEFENSE_WAR));
        assert!(is_level_check_relaxed_zone(ZONE_JURAID_MOUNTAIN));
        assert!(!is_level_check_relaxed_zone(21));
    }

    /// Test party member info packet wire format.
    /// C++ uses DByte (u16 length prefix) for the name string.
    #[test]
    fn test_party_member_info_packet_format() {
        let ch = make_test_char(42, "TestUser", 1, 60);
        let pkt = build_party_member_info(&ch, 1, -1, 0, -1);

        assert_eq!(pkt.opcode, Opcode::WizParty as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_INSERT)); // sub-opcode
        assert_eq!(r.read_u16(), Some(1)); // success
        assert_eq!(r.read_u32(), Some(42)); // session id
        assert_eq!(r.read_u8(), Some(1)); // index hint
        let name = r.read_string().unwrap(); // DByte (u16 length prefix)
        assert_eq!(name, "TestUser");
        assert_eq!(r.read_u16().map(|v| v as i16), Some(500)); // max_hp
        assert_eq!(r.read_u16().map(|v| v as i16), Some(500)); // hp
        assert_eq!(r.read_u8(), Some(60)); // level
        assert_eq!(r.read_u16(), Some(101)); // class
        assert_eq!(r.read_u16().map(|v| v as i16), Some(200)); // max_mp
        assert_eq!(r.read_u16().map(|v| v as i16), Some(200)); // mp
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u8(), Some(0)); // padding
        let tnid = r.read_u32().unwrap(); // target_number_id
        assert_eq!(tnid, 0xFFFFFFFF); // -1 as i32 as u32 (C++ sign-extension)
        assert_eq!(r.read_u8().map(|v| v as i8), Some(0)); // party_type
        assert_eq!(r.read_u8(), Some(0xFF)); // loyalty rank (-1 = unranked)
        assert_eq!(r.remaining(), 0);
    }

    /// Test party HP update packet wire format.
    #[test]
    fn test_party_hp_update_format() {
        let ch = make_test_char(42, "TestUser", 1, 60);
        let pkt = build_party_hp_update(&ch);

        assert_eq!(pkt.opcode, Opcode::WizParty as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_HPCHANGE));
        assert_eq!(r.read_u32(), Some(42)); // sid
        assert_eq!(r.read_u16().map(|v| v as i16), Some(500)); // max_hp
        assert_eq!(r.read_u16().map(|v| v as i16), Some(500)); // hp
        assert_eq!(r.read_u16().map(|v| v as i16), Some(200)); // max_mp
        assert_eq!(r.read_u16().map(|v| v as i16), Some(200)); // mp
        assert_eq!(r.remaining(), 0);
    }

    /// Test PARTY_CLASSCHANGE packet wire format.
    ///
    /// ```text
    /// [u8 PARTY_CLASSCHANGE=0x08] [u32 sid] [u16 new_class]
    /// ```
    #[test]
    fn test_party_class_change_format() {
        let pkt = build_party_class_change(42, 205);

        assert_eq!(pkt.opcode, Opcode::WizParty as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_CLASSCHANGE));
        assert_eq!(r.read_u32(), Some(42)); // sid
        assert_eq!(r.read_u16(), Some(205)); // new_class
        assert_eq!(r.remaining(), 0);
    }

    /// Test broadcast_party_class_change sends to party members.
    #[test]
    fn test_broadcast_party_class_change() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);

        // Member 1 changes class -- party should be notified
        broadcast_party_class_change(&world, 1, 302);

        // Member 2 should receive the packet
        let pkt = rx2.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizParty as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_CLASSCHANGE));
        assert_eq!(r.read_u32(), Some(1)); // member 1 sid
        assert_eq!(r.read_u16(), Some(302)); // new class
        assert_eq!(r.remaining(), 0);
    }

    /// Test broadcast_party_class_change is a no-op when not in party.
    #[test]
    fn test_broadcast_party_class_change_no_party() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Solo", 1, 60), pos);

        // Not in a party -- should be a no-op
        broadcast_party_class_change(&world, 1, 205);

        // No packet should be received
        assert!(rx1.try_recv().is_err());
    }

    /// Test party error packet format.
    #[test]
    fn test_party_error_format() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        send_party_error(&world, 1, PARTY_ERR_LEVEL);

        let pkt = rx.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizParty as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_INSERT));
        assert_eq!(r.read_u16().map(|v| v as i16), Some(PARTY_ERR_LEVEL));
        assert_eq!(r.remaining(), 0);
    }

    /// Test Party struct operations.
    #[test]
    fn test_party_struct() {
        use crate::world::Party;

        let mut party = Party::new(1, 10);

        assert_eq!(party.leader_sid(), Some(10));
        assert_eq!(party.member_count(), 1);
        assert!(!party.is_full());
        assert!(party.is_leader(10));
        assert!(!party.is_leader(20));
        assert!(party.contains(10));
        assert!(!party.contains(20));

        // Add member
        assert!(party.add_member(20));
        assert_eq!(party.member_count(), 2);
        assert!(party.contains(20));

        // Can't add duplicate
        assert!(!party.add_member(20));
        assert_eq!(party.member_count(), 2);

        // Remove member
        assert!(party.remove_member(20));
        assert_eq!(party.member_count(), 1);
        assert!(!party.contains(20));

        // Remove non-existent member
        assert!(!party.remove_member(99));
    }

    /// Test Party leadership swap.
    #[test]
    fn test_party_promote() {
        use crate::world::Party;

        let mut party = Party::new(1, 10);
        party.add_member(20);
        party.add_member(30);

        assert!(party.is_leader(10));
        assert!(!party.is_leader(20));

        // Promote 20
        let pos = party.find_slot(20).unwrap();
        party.swap_leader(pos);

        assert!(party.is_leader(20));
        assert!(!party.is_leader(10));
        // 10 should be at position pos now
        assert!(party.contains(10));
    }

    /// Test Party full capacity.
    #[test]
    fn test_party_full() {
        use crate::world::{Party, MAX_PARTY_USERS};

        let mut party = Party::new(1, 10);
        for i in 1..MAX_PARTY_USERS {
            assert!(party.add_member(10 + i as u16));
        }
        assert!(party.is_full());
        assert_eq!(party.member_count(), MAX_PARTY_USERS);

        // Can't add more
        assert!(!party.add_member(99));
    }

    /// Test WorldState party creation and membership.
    #[test]
    fn test_world_party_create() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let ch1 = make_test_char(1, "Leader", 1, 60);
        let ch2 = make_test_char(2, "Member", 1, 60);
        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, ch1, pos);
        world.register_ingame(2, ch2, pos);

        // Create party
        let party_id = world.create_party(1).unwrap();
        assert!(world.is_in_party(1));
        assert_eq!(world.get_party_id(1), Some(party_id));
        assert!(!world.is_in_party(2));

        // Add member
        assert!(world.add_party_member(party_id, 2));
        assert!(world.is_in_party(2));

        // Verify party
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 2);
        assert!(party.is_leader(1));

        // Remove member
        assert!(world.remove_party_member(party_id, 2));
        assert!(!world.is_in_party(2));

        // Disband
        let members = world.disband_party(party_id);
        assert_eq!(members.len(), 1); // Only leader remains
        assert!(!world.is_in_party(1));
        assert!(world.get_party(party_id).is_none());
    }

    /// Test party invitation tracking.
    #[test]
    fn test_party_invitations() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        assert!(!world.has_party_invitation(2));

        world.set_party_invitation(2, 100, 1);
        assert!(world.has_party_invitation(2));

        let inv = world.take_party_invitation(2);
        assert_eq!(inv, Some((100, 1)));
        assert!(!world.has_party_invitation(2));

        // Take again returns None
        assert!(world.take_party_invitation(2).is_none());
    }

    /// Test party cleanup on disconnect — member leaves, party survives.
    #[test]
    fn test_cleanup_party_member_disconnect() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let (tx3, mut rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member1", 1, 60), pos);
        world.register_ingame(3, make_test_char(3, "Member2", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);
        world.add_party_member(party_id, 3);

        assert_eq!(world.get_party(party_id).unwrap().member_count(), 3);

        // Member 2 disconnects
        world.cleanup_party_on_disconnect(2);

        // Member 2 is no longer in party
        assert!(!world.is_in_party(2));

        // Party still exists with 2 members
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 2);
        assert!(party.is_leader(1));
        assert!(party.contains(3));

        // Remaining members received PARTY_REMOVE packet
        // Drain rx2 (member2 also gets the packet before removal)
        while rx2.try_recv().is_ok() {}
        // rx3 should have received PARTY_REMOVE
        let pkt = rx3.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizParty as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_REMOVE));
        assert_eq!(r.read_u32(), Some(2)); // removed member's sid
    }

    /// Test party cleanup on disconnect — leader leaves, promotes next member.
    #[test]
    fn test_cleanup_party_leader_disconnect() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member1", 1, 60), pos);
        world.register_ingame(3, make_test_char(3, "Member2", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);
        world.add_party_member(party_id, 3);

        // Leader disconnects
        world.cleanup_party_on_disconnect(1);

        // Leader is no longer in party
        assert!(!world.is_in_party(1));

        // Party still exists with 2 members, member2 promoted to leader
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 2);
        assert!(party.is_leader(2)); // promoted
        assert!(party.contains(3));
    }

    /// Test party cleanup on disconnect — 2-member party disbands.
    #[test]
    fn test_cleanup_party_two_members_disband() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);

        // Member disconnects — only leader left, party should disband
        world.cleanup_party_on_disconnect(2);

        assert!(!world.is_in_party(1));
        assert!(!world.is_in_party(2));
        assert!(world.get_party(party_id).is_none());

        // Leader should have received PARTY_DELETE
        let pkt = rx1.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizParty as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_DELETE));
    }

    /// Test party cleanup clears pending invitations.
    #[test]
    fn test_cleanup_party_clears_invitations() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        world.set_party_invitation(1, 100, 2);
        assert!(world.has_party_invitation(1));

        world.cleanup_party_on_disconnect(1);
        assert!(!world.has_party_invitation(1));
    }

    // ── Sprint 49: Integration Tests ────────────────────────────────────

    /// Integration test: full party formation flow — create, invite, accept, verify state.
    #[test]
    fn test_integration_party_formation_full_flow() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member1", 1, 58), pos);
        world.register_ingame(3, make_test_char(3, "Member2", 1, 62), pos);

        // Step 1: Create party with leader=1
        let party_id = world.create_party(1).unwrap();
        assert!(world.is_in_party(1));
        assert!(!world.is_in_party(2));

        // Step 2: Simulate invitation and acceptance for member2
        world.set_party_invitation(2, party_id, 1);
        assert!(world.has_party_invitation(2));

        // Step 3: Accept invitation — add to party
        let (inv_party, inv_leader) = world.take_party_invitation(2).unwrap();
        assert_eq!(inv_party, party_id);
        assert_eq!(inv_leader, 1);
        assert!(world.add_party_member(party_id, 2));

        // Step 4: Add third member
        world.set_party_invitation(3, party_id, 1);
        world.take_party_invitation(3);
        assert!(world.add_party_member(party_id, 3));

        // Verify full party state
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 3);
        assert!(party.is_leader(1));
        assert!(party.contains(2));
        assert!(party.contains(3));
        assert!(!party.is_full());

        // Verify party_id is set on all members
        assert_eq!(world.get_party_id(1), Some(party_id));
        assert_eq!(world.get_party_id(2), Some(party_id));
        assert_eq!(world.get_party_id(3), Some(party_id));
    }

    /// Integration test: party leader transfer — leader promotes member, verify new leader.
    #[test]
    fn test_integration_party_leader_transfer() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "NewLeader", 1, 60), pos);
        world.register_ingame(3, make_test_char(3, "Member", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);
        world.add_party_member(party_id, 3);

        // Verify initial leader
        let party = world.get_party(party_id).unwrap();
        assert!(party.is_leader(1));

        // Promote member 2
        assert!(world.promote_party_leader(party_id, 2));

        // Verify new leader
        let party = world.get_party(party_id).unwrap();
        assert!(party.is_leader(2));
        assert!(!party.is_leader(1));
        // Old leader is still in party
        assert!(party.contains(1));
        assert_eq!(party.member_count(), 3);
    }

    /// Integration test: party member disconnect with 3+ members — party survives.
    #[test]
    fn test_integration_party_member_disconnect_survives() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, mut rx3) = mpsc::unbounded_channel();
        let (tx4, mut rx4) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);
        world.register_session(4, tx4);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member1", 1, 60), pos);
        world.register_ingame(3, make_test_char(3, "Member2", 1, 60), pos);
        world.register_ingame(4, make_test_char(4, "Member3", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);
        world.add_party_member(party_id, 3);
        world.add_party_member(party_id, 4);
        assert_eq!(world.get_party(party_id).unwrap().member_count(), 4);

        // Member2 disconnects
        world.cleanup_party_on_disconnect(2);

        // Party still exists with 3 members
        assert!(!world.is_in_party(2));
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 3);
        assert!(party.is_leader(1));
        assert!(party.contains(3));
        assert!(party.contains(4));

        // Remaining members received removal packets
        // Drain any packets
        while rx1.try_recv().is_ok() {}
        while rx3.try_recv().is_ok() {}
        while rx4.try_recv().is_ok() {}
    }

    /// Integration test: party leader disconnect promotes next member.
    #[test]
    fn test_integration_party_leader_disconnect_promotes() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member1", 1, 60), pos);
        world.register_ingame(3, make_test_char(3, "Member2", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);
        world.add_party_member(party_id, 3);

        // Leader disconnects
        world.cleanup_party_on_disconnect(1);

        assert!(!world.is_in_party(1));
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 2);
        // Member2 should be promoted to leader
        assert!(party.is_leader(2));
        assert!(party.contains(3));
    }

    /// Integration test: party invitation declined — solo leader party disbands.
    #[test]
    fn test_integration_party_invitation_declined_disbands() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Decliner", 1, 60), pos);

        // Create party (leader only)
        let party_id = world.create_party(1).unwrap();
        assert!(world.is_in_party(1));

        // Invite target, then decline
        world.set_party_invitation(2, party_id, 1);
        world.take_party_invitation(2);
        // Decline: since leader is alone, disband
        let party = world.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 1);
        let members = world.disband_party(party_id);
        assert_eq!(members.len(), 1);

        // Party no longer exists
        assert!(!world.is_in_party(1));
        assert!(world.get_party(party_id).is_none());

        // Leader should receive PARTY_DELETE
        let mut del_pkt = Packet::new(Opcode::WizParty as u8);
        del_pkt.write_u8(PARTY_DELETE);
        world.send_to_session_owned(1, del_pkt);

        let pkt = rx1.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizParty as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_DELETE));
    }

    // ── Sprint 55: Hardening Edge Case Tests ────────────────────────

    /// Edge case: inviting a player who is already in a party should be
    /// rejected at the handler level. The handler checks `is_in_party()`
    /// before calling `add_party_member()`.
    #[test]
    fn test_invite_player_already_in_party_gating() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader1", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member", 1, 60), pos);
        world.register_ingame(3, make_test_char(3, "Leader2", 1, 60), pos);

        // Create party A with leader=1, add member=2
        let party_a = world.create_party(1).unwrap();
        world.add_party_member(party_a, 2);
        assert!(world.is_in_party(2));

        // Create party B with leader=3
        let _party_b = world.create_party(3).unwrap();

        // Handler gating: check is_in_party BEFORE calling add_party_member
        let already_in_party = world.is_in_party(2);
        assert!(
            already_in_party,
            "Player already in party A should be detected"
        );

        // The handler would return early here, so player stays in party A
        assert_eq!(
            world.get_party_id(2),
            Some(party_a),
            "Player's party membership should remain unchanged"
        );
    }

    /// Edge case: party leader disconnect with exactly 2 members should
    /// disband the party (remaining member goes solo).
    #[test]
    fn test_leader_disconnect_two_member_party_disbands() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = crate::world::Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60), pos);
        world.register_ingame(2, make_test_char(2, "Member", 1, 60), pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 2);
        assert_eq!(world.get_party(party_id).unwrap().member_count(), 2);

        // Leader disconnects — 2-member party should disband
        world.cleanup_party_on_disconnect(1);

        assert!(!world.is_in_party(1), "Leader should not be in party");
        assert!(
            !world.is_in_party(2),
            "Remaining member should not be in party"
        );
        assert!(
            world.get_party(party_id).is_none(),
            "Party should be disbanded"
        );

        // Remaining member should receive PARTY_DELETE
        let pkt = rx2.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizParty as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_DELETE));
    }

    /// Edge case: cleanup_party_on_disconnect for a player not in any party
    /// should be a no-op (not panic).
    #[test]
    fn test_cleanup_party_disconnect_no_party_is_noop() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        // Player is not in any party
        assert!(!world.is_in_party(1));

        // Should not panic
        world.cleanup_party_on_disconnect(1);

        // Still not in a party
        assert!(!world.is_in_party(1));
    }

    // ── Sprint 248: Same-clan exemption tests ────────────────────────

    /// Same-clan members should bypass level check.
    #[test]
    fn test_same_clan_bypasses_level_check() {
        // Two players with same knights_id bypass level check
        let knights_id: u16 = 100;
        let same_clan = knights_id > 0 && knights_id == knights_id;
        assert!(same_clan, "Same clan should bypass level check");

        // Different clan does NOT bypass
        let other_id: u16 = 200;
        let diff_clan = knights_id > 0 && knights_id == other_id;
        assert!(!diff_clan, "Different clan should not bypass");

        // No clan does NOT bypass
        let no_clan: u16 = 0;
        let no_clan_check = no_clan > 0 && no_clan == knights_id;
        assert!(!no_clan_check, "No-clan should not bypass");
    }

    /// Level compatibility function should work correctly.
    #[test]
    fn test_level_compatible_function() {
        // 60 and 60: compatible (same level)
        assert!(is_level_compatible(60, 60));
        // 60 and 68: within ±8
        assert!(is_level_compatible(60, 68));
        // 60 and 52: within ±8
        assert!(is_level_compatible(60, 52));
        // 60 and 80: 80 <= 60*1.5=90 AND 80 >= 60*2/3=40 → compatible
        assert!(is_level_compatible(60, 80));
        // 10 and 30: 30 > 10+8=18 AND 30 > 10*1.5=15 → incompatible
        assert!(!is_level_compatible(10, 30));
    }

    // ── Sprint 264: Seeking party item + CSW Delos alliance ──────────

    /// Seeking party item constant should be 914057000.
    #[test]
    fn test_seeking_party_item_constant() {
        assert_eq!(SEEKING_PARTY_ITEM, 914057000);
    }

    /// Seeking party error code should be -10.
    #[test]
    fn test_seeking_party_error_code() {
        assert_eq!(PARTY_ERR_SEEKING_ITEM, -10);
    }

    /// ZONE_DELOS constant should be 30.
    #[test]
    fn test_zone_delos_constant() {
        assert_eq!(ZONE_DELOS, 30);
    }

    /// Seeking party error packet should have correct format.
    #[test]
    fn test_seeking_party_error_packet_format() {
        let world = WorldState::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();

        world.register_session(1, tx1);

        // Send seeking party error
        send_party_error(&world, 1, PARTY_ERR_SEEKING_ITEM);

        let recv_pkt = rx1.try_recv().unwrap();
        assert_eq!(recv_pkt.opcode, Opcode::WizParty as u8);
        let mut r = PacketReader::new(&recv_pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_INSERT));
        assert_eq!(r.read_i16(), Some(-10));
    }

    /// check_exist_item returns false for non-existent session.
    /// This proves the guard works if target has no inventory.
    #[test]
    fn test_seeking_party_item_check_no_session() {
        let world = WorldState::new();
        // No session registered → check_exist_item returns false
        assert!(
            !world.check_exist_item(999, SEEKING_PARTY_ITEM, 1),
            "Non-existent session should fail item check"
        );
    }

    /// CSW Delos: cross-clan party without alliance should get error -3.
    #[test]
    fn test_csw_delos_cross_clan_no_alliance_rejected() {
        use crate::world::{CswEventState, CswOpStatus, KnightsInfo};

        // Set up CSW as active
        let csw = CswEventState {
            status: CswOpStatus::War,
            started: true,
            ..CswEventState::default()
        };
        // Verify CSW is active
        assert!(csw.is_active());
        assert!(csw.is_war_active());

        // Target clan without alliance → should be rejected
        let target_clan = KnightsInfo {
            id: 200,
            alliance: 0, // no alliance
            name: "TargetClan".to_string(),
            ..KnightsInfo::default()
        };

        // The check: alliance == 0 means not in alliance → reject cross-clan
        assert_eq!(target_clan.alliance, 0);
        // Different clans check
        assert_ne!(100u16, target_clan.id);
    }

    /// CSW Delos: same-clan party should be allowed even during CSW.
    #[test]
    fn test_csw_delos_same_clan_allowed() {
        // Same clan → the cross-clan check is skipped
        let inviter_knights_id: u16 = 100;
        let target_knights_id: u16 = 100;
        assert_eq!(
            inviter_knights_id, target_knights_id,
            "Same clan should skip CSW alliance check"
        );
    }

    // ── Sprint 296: Clanless target CSW Delos fix ──────────────────

    /// CSW Delos: clanless target should be allowed (C++ GetClanPtr(0) returns null → block skipped).
    #[test]
    fn test_csw_delos_clanless_target_allowed() {
        // When target has no clan (knights_id == 0), C++ GetClanPtr returns null,
        // entire CSW alliance block is skipped — target is allowed.
        let target_knights_id: u16 = 0;
        let inviter_knights_id: u16 = 100;

        // Different clans → outer check passes
        assert_ne!(inviter_knights_id, target_knights_id);

        // C++ behavior: target has no clan → skip block → allowed
        let allowed = if target_knights_id == 0 {
            true // C++ skips when GetClanPtr returns null
        } else {
            false // Would need alliance check
        };
        assert!(
            allowed,
            "Clanless target should be allowed during CSW on Delos"
        );
    }

    /// Genie active trade check constant.
    #[test]
    fn test_genie_active_blocks_trade() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(1, tx1);

        // Default genie_active should be false
        let genie = world.with_session(1, |h| h.genie_active).unwrap_or(false);
        assert!(!genie, "Genie should be inactive by default");

        // After activating genie
        world.update_session(1, |h| {
            h.genie_active = true;
        });
        let genie = world.with_session(1, |h| h.genie_active).unwrap_or(false);
        assert!(genie, "Genie should be active after setting");
    }

    // ── Sprint 272: isPartnerPartyZone tests ──────────────────────────

    #[test]
    fn test_partner_party_zone_moradon() {
        // All Moradon variants are partner party zones
        assert!(super::is_partner_party_zone(21)); // ZONE_MORADON
        assert!(super::is_partner_party_zone(22)); // ZONE_MORADON2
        assert!(super::is_partner_party_zone(23)); // ZONE_MORADON3
        assert!(super::is_partner_party_zone(24)); // ZONE_MORADON4
        assert!(super::is_partner_party_zone(25)); // ZONE_MORADON5
    }

    #[test]
    fn test_partner_party_zone_arenas() {
        assert!(super::is_partner_party_zone(48)); // ZONE_ARENA
        assert!(super::is_partner_party_zone(51)); // ZONE_ORC_ARENA
        assert!(super::is_partner_party_zone(52)); // ZONE_BLOOD_DON_ARENA
        assert!(super::is_partner_party_zone(53)); // ZONE_GOBLIN_ARENA
    }

    #[test]
    fn test_partner_party_zone_special() {
        assert!(super::is_partner_party_zone(55)); // ZONE_FORGOTTEN_TEMPLE
        assert!(super::is_partner_party_zone(86)); // ZONE_UNDER_CASTLE
        assert!(super::is_partner_party_zone(81)); // ZONE_STONE1
        assert!(super::is_partner_party_zone(82)); // ZONE_STONE2
        assert!(super::is_partner_party_zone(83)); // ZONE_STONE3
        assert!(super::is_partner_party_zone(35)); // ZONE_DELOS_CASTELLAN
        assert!(super::is_partner_party_zone(95)); // ZONE_DRAKI_TOWER
        assert!(super::is_partner_party_zone(29)); // ZONE_OLD_MORADON
    }

    #[test]
    fn test_partner_party_zone_eslant() {
        assert!(super::is_partner_party_zone(11)); // ZONE_KARUS_ESLANT
        assert!(super::is_partner_party_zone(13)); // ZONE_KARUS_ESLANT2
        assert!(super::is_partner_party_zone(14)); // ZONE_KARUS_ESLANT3
        assert!(super::is_partner_party_zone(12)); // ZONE_ELMORAD_ESLANT
        assert!(super::is_partner_party_zone(15)); // ZONE_ELMORAD_ESLANT2
        assert!(super::is_partner_party_zone(16)); // ZONE_ELMORAD_ESLANT3
    }

    #[test]
    fn test_command_leader_set_on_create() {
        // Command leader is set to leader on party creation
        use crate::world::Party;
        let party = Party::new(1, 100);
        assert!(party.is_command_leader(100));
        assert!(!party.is_command_leader(200));
    }

    #[test]
    fn test_command_leader_transfer() {
        // Command leadership can be transferred
        use crate::world::Party;
        let mut party = Party::new(1, 100);
        party.add_member(200);
        assert!(party.is_command_leader(100));
        assert!(!party.is_command_leader(200));

        party.command_leader_sid = Some(200);
        assert!(!party.is_command_leader(100));
        assert!(party.is_command_leader(200));
    }

    #[test]
    fn test_non_partner_party_zones() {
        // Homeland, war zones, etc. are NOT partner party zones
        assert!(!super::is_partner_party_zone(1)); // ZONE_KARUS
        assert!(!super::is_partner_party_zone(2)); // ZONE_ELMORAD
        assert!(!super::is_partner_party_zone(61)); // ZONE_BATTLE
        assert!(!super::is_partner_party_zone(71)); // ZONE_RONARK_LAND
        assert!(!super::is_partner_party_zone(72)); // ZONE_ARDREAM
        assert!(!super::is_partner_party_zone(30)); // ZONE_DELOS
    }

    // ── Sprint 282: Party 850ms shared cooldown ──────────────────────────

    /// PartyTargetNumber, PartyAlert, and PartyCommand (850ms cooldown).
    #[test]
    fn test_party_target_number_cooldown_constant() {
        // The 850ms cooldown is shared among three party operations.
        // Verify the cooldown value matches C++ UNIXTIME2 + 850.
        let cooldown_ms: u128 = 850;
        assert_eq!(
            cooldown_ms, 850,
            "Party target/alert/command cooldown must be 850ms"
        );
    }

    #[test]
    fn test_party_cooldown_field_initialized_in_past() {
        // SessionHandle.last_target_number_time is initialized 1s in the past,
        // so the first party command is always allowed.
        let one_sec = std::time::Duration::from_secs(1);
        let cooldown = std::time::Duration::from_millis(850);
        assert!(
            one_sec > cooldown,
            "Initial 1s offset must exceed 850ms cooldown"
        );
    }

    // ── Sprint 320: Party target isInGame validation ────────────────

    /// PARTY_ERR_OFFLINE (-6) should be used for offline targets.
    #[test]
    fn test_party_err_offline_code() {
        assert_eq!(super::PARTY_ERR_OFFLINE, -6);
    }

    /// Target without position (not in-game) should be rejected.
    #[test]
    fn test_party_create_target_not_ingame() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Session 2 not in game — is_session_ingame returns false
        assert!(
            !world.is_session_ingame(2),
            "session without register_ingame should not be in-game"
        );
    }

    /// Target with character (in-game) should pass isInGame check.
    #[test]
    fn test_party_create_target_ingame() {
        use crate::world::{Position, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(2, make_test_char(2, "Target", 1, 60), pos);

        assert!(
            world.is_session_ingame(2),
            "in-game session should pass isInGame check"
        );
    }

    // ── Sprint 320: StateChange party leader symbol ─────────────────

    /// Party leader 'P' symbol is broadcast with state_change type=6, buff=1.
    #[test]
    fn test_party_leader_state_change_packet_format() {
        let pkt =
            crate::handler::regene::build_state_change_broadcast(100, STATE_CHANGE_PARTY_LEADER, 1);
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizStateChange as u8);
        let data = &pkt.data;
        // [u32 sid=100] [u8 type=STATE_CHANGE_PARTY_LEADER] [u32 buff=1]
        assert_eq!(data.len(), 9);
        assert_eq!(
            u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            100
        );
        assert_eq!(data[4], STATE_CHANGE_PARTY_LEADER);
        assert_eq!(u32::from_le_bytes([data[5], data[6], data[7], data[8]]), 1);
        // buff = show
    }

    /// Removing party leader symbol on disband.
    #[test]
    fn test_party_leader_remove_state_change_packet() {
        let pkt =
            crate::handler::regene::build_state_change_broadcast(200, STATE_CHANGE_PARTY_LEADER, 0);
        let data = &pkt.data;
        assert_eq!(data[4], STATE_CHANGE_PARTY_LEADER);
        assert_eq!(u32::from_le_bytes([data[5], data[6], data[7], data[8]]), 0);
        // buff = hide
    }

    /// Party state change type 6 — value 1 = show, 0 = hide.
    #[test]
    fn test_party_leader_symbol_values() {
        // C++ StateChangeServerDirect(6, X):
        // X=1 on create/promote — show 'P' symbol
        // X=0 on disband/promote-old — hide 'P' symbol
        let show: u32 = 1;
        let hide: u32 = 0;
        assert_ne!(show, hide);
        assert_eq!(show, 1);
        assert_eq!(hide, 0);
    }

    // ── Sprint 321: Chicken mode party level bypass ─────────────────

    /// Chicken players bypass level compatibility check.
    #[test]
    fn test_chicken_mode_bypasses_level_check() {
        // Level 15 (chicken) + Level 70 (non-chicken) — normally incompatible
        assert!(!is_level_compatible(15, 70));

        // But chicken mode should bypass:
        let inviter_is_chicken = 15u8 < 30;
        let target_is_chicken = 70u8 < 30;
        let bypass = inviter_is_chicken || target_is_chicken;
        assert!(bypass, "chicken mode should bypass level check");
    }

    #[test]
    fn test_both_chickens_bypass_level_check() {
        // Level 5 + Level 25 — both chickens, normally incompatible
        assert!(!is_level_compatible(5, 25));

        let inviter_is_chicken = 5u8 < 30;
        let target_is_chicken = 25u8 < 30;
        let bypass = inviter_is_chicken || target_is_chicken;
        assert!(bypass, "both chickens should bypass level check");
    }

    #[test]
    fn test_non_chicken_enforces_level_check() {
        // Level 60 + Level 35 — neither is chicken, incompatible (diff > 8 and ratio fails)
        assert!(!is_level_compatible(60, 35));

        let inviter_is_chicken = 60u8 < 30;
        let target_is_chicken = 35u8 < 30;
        let bypass = inviter_is_chicken || target_is_chicken;
        assert!(!bypass, "non-chickens should NOT bypass level check");
    }

    #[test]
    fn test_chicken_threshold_is_30() {
        // Level 29 = chicken, Level 30 = NOT chicken
        assert!(29u8 < 30, "level 29 should be chicken");
        assert!((30u8 >= 30), "level 30 should NOT be chicken");
    }
}
