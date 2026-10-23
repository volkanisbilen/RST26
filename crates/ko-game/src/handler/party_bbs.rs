//! WIZ_PARTY_BBS (0x4F) handler -- party bulletin board system.
//! An in-memory bulletin board where individual users can register as
//! "seeking a party" and party leaders can post "wanted" notices looking
//! for specific classes.
//! ## Sub-opcodes
//! | Code | Name              | Direction | Description                          |
//! |------|-------------------|-----------|--------------------------------------|
//! | 1    | REGISTER          | C->S      | Register user in seeking party list   |
//! | 2    | DELETE            | C->S      | Remove user from seeking party list   |
//! | 3    | NEEDED            | C->S      | (Unimplemented in C++)               |
//! | 4    | WANTED            | C->S      | Party leader posts wanted notice      |
//! | 5    | CHANGE            | C->S      | Party leader updates wanted notice    |
//! | 6    | PARTY_DELETE      | C->S      | Party leader removes wanted notice    |
//! | 11   | LIST              | C->S      | Query available posts with filters    |
//! ## Constants
//! - `MAX_BBS_PAGE` = 22 entries per page
//! - `MAX_BBS_MESSAGE` = 40 max message length

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::{SeekingPartyUser, WorldState, MAX_BBS_PAGE};
use crate::zone::SessionId;

// ── Sub-opcode constants ────────────────────────────────────────────────────

const PARTY_BBS_REGISTER: u8 = 1;
const PARTY_BBS_DELETE: u8 = 2;
const PARTY_BBS_WANTED: u8 = 4;
const PARTY_BBS_PARTY_CHANGE: u8 = 5;
const PARTY_BBS_PARTY_DELETE: u8 = 6;
const PARTY_BBS_LIST: u8 = 11;

/// Type byte for seeking party packets.
const PARTY_TYPE_SEEKING: u8 = 0;

/// Maximum message length for seeking notes.
const MAX_BBS_MESSAGE: usize = 40;

/// Maximum users for overflow page check.
const MAX_USER: u16 = 3000;

/// Handle WIZ_PARTY_BBS (0x4F) from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    // C++ reads Type byte first (must be 0 = PARTY_TYPE_SEEKING)
    let pkt_type = match reader.read_u8() {
        Some(t) => t,
        None => return Ok(()),
    };
    if pkt_type != PARTY_TYPE_SEEKING {
        return Ok(());
    }

    let sub_opcode = match reader.read_u8() {
        Some(op) => op,
        None => return Ok(()),
    };

    // C++ checks: isSellingMerchantingPreparing()
    let world = session.world().clone();
    let sid = session.session_id();
    if world.is_selling_merchant_preparing(sid) || world.is_selling_merchant(sid) {
        return Ok(());
    }

    match sub_opcode {
        PARTY_BBS_REGISTER => handle_register(session, &world, sid, &mut reader).await,
        PARTY_BBS_DELETE => handle_delete(session, &world, sid, &mut reader).await,
        PARTY_BBS_WANTED => handle_wanted(session, &world, sid, &mut reader).await,
        PARTY_BBS_PARTY_CHANGE => handle_change(session, &world, sid, &mut reader),
        PARTY_BBS_PARTY_DELETE => handle_party_delete(session, &world, sid, &mut reader).await,
        PARTY_BBS_LIST => handle_list(session, &world, sid, &mut reader),
        _ => {
            debug!(
                "[{}] Party BBS unhandled sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle PARTY_BBS_REGISTER (1) -- individual user registers as seeking party.
/// Packet: `[SByte] [message:string]`
async fn handle_register(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let seeking_msg = reader.read_sbyte_string().unwrap_or_default();
    let seeking_msg = truncate_message(&seeking_msg);

    let ch = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    let party_id = ch.party_id.unwrap_or(0);
    let is_leader = if party_id > 0 {
        world
            .get_party(party_id)
            .map(|p| p.is_leader(sid))
            .unwrap_or(false)
    } else {
        false
    };

    let entry = SeekingPartyUser {
        sid,
        class: ch.class,
        is_party_leader: u8::from(is_leader),
        level: ch.level as i16,
        zone: pos.zone_id as u8,
        seeking_note: seeking_msg,
        name: ch.name.clone(),
        nation: ch.nation,
        party_id,
        seek_type: 0,
        login_type: 0,
    };

    world.register_seeking_party(entry);

    // StateChangeServerDirect(2, 2) -- seeking a party
    broadcast_state_change(session, world, sid, 2, 2).await?;

    // Send the list response (page 0, no filters)
    send_party_bbs_list(world, sid, 0, 0, 0, 0);

    debug!("[{}] PARTY_BBS_REGISTER: {}", session.addr(), ch.name);
    Ok(())
}

/// Handle PARTY_BBS_DELETE (2) -- individual user removes themselves from list.
/// Packet: (no payload)
async fn handle_delete(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    _reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    world.remove_seeking_party(sid);

    // StateChangeServerDirect(2, 1) -- not looking for a party
    broadcast_state_change(session, world, sid, 2, 1).await?;

    // Send the list response (page 0, no filters)
    send_party_bbs_list(world, sid, 0, 0, 0, 0);

    debug!("[{}] PARTY_BBS_DELETE", session.addr());
    Ok(())
}

/// Handle PARTY_BBS_WANTED (4) -- party leader posts wanted notice.
/// Packet: `[DByte] [wanted_class:u16] [page_index:u16] [message:string]`
async fn handle_wanted(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let ch = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };
    let party_id = match ch.party_id {
        Some(id) => id,
        None => return Ok(()),
    };
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };
    if !party.is_leader(sid) {
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    let wanted_class = reader.read_u16().unwrap_or(0);
    let _page_index = reader.read_u16().unwrap_or(0);
    let wanted_msg = reader.read_string().unwrap_or_default();
    let wanted_msg = truncate_message(&wanted_msg);

    let entry = SeekingPartyUser {
        sid,
        class: wanted_class,
        is_party_leader: 1,
        level: ch.level as i16,
        zone: pos.zone_id as u8,
        seeking_note: wanted_msg,
        name: ch.name.clone(),
        nation: ch.nation,
        party_id,
        seek_type: 0,
        login_type: 0,
    };

    world.register_seeking_party(entry);

    // StateChangeServerDirect(2, 3) -- looking for party members
    broadcast_state_change(session, world, sid, 2, 3).await?;

    // Send the list response (page 0, no filters)
    send_party_bbs_list(world, sid, 0, 0, 0, 0);

    debug!(
        "[{}] PARTY_BBS_WANTED: class={}",
        session.addr(),
        wanted_class
    );
    Ok(())
}

/// Handle PARTY_BBS_PARTY_CHANGE (5) -- party leader updates wanted notice.
/// Packet: `[DByte] [wanted_class:u16] [page_index:u16] [message:string]`
fn handle_change(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let ch = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };
    let party_id = match ch.party_id {
        Some(id) => id,
        None => return Ok(()),
    };
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };
    if !party.is_leader(sid) {
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    let wanted_class = reader.read_u16().unwrap_or(0);
    let _page_index = reader.read_u16().unwrap_or(0);
    let wanted_msg = reader.read_string().unwrap_or_default();
    let wanted_msg = truncate_message(&wanted_msg);

    let entry = SeekingPartyUser {
        sid,
        class: wanted_class,
        is_party_leader: 1,
        level: ch.level as i16,
        zone: pos.zone_id as u8,
        seeking_note: wanted_msg,
        name: ch.name.clone(),
        nation: ch.nation,
        party_id,
        seek_type: 0,
        login_type: 0,
    };

    world.register_seeking_party(entry);

    // Send the list response (page 0, no filters)
    send_party_bbs_list(world, sid, 0, 0, 0, 0);

    debug!(
        "[{}] PARTY_BBS_PARTY_CHANGE: class={}",
        session.addr(),
        wanted_class
    );
    Ok(())
}

/// Handle PARTY_BBS_PARTY_DELETE (6) -- party leader removes wanted notice.
/// Packet: (no payload)
async fn handle_party_delete(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    _reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let ch = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };
    let party_id = match ch.party_id {
        Some(id) => id,
        None => return Ok(()),
    };
    let party = match world.get_party(party_id) {
        Some(p) => p,
        None => return Ok(()),
    };
    if !party.is_leader(sid) {
        return Ok(());
    }

    world.remove_seeking_party(sid);

    // StateChangeServerDirect(2, 1) -- not looking for a party
    broadcast_state_change(session, world, sid, 2, 1).await?;

    // Send the list response (page 0, no filters)
    send_party_bbs_list(world, sid, 0, 0, 0, 0);

    debug!("[{}] PARTY_BBS_PARTY_DELETE", session.addr());
    Ok(())
}

/// Handle PARTY_BBS_LIST (11) -- query the seeking party list.
/// Packet: `[page:u16] [type_filter:u8] [location_filter:u8] [level_filter:u8]`
fn handle_list(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let page_index = reader.read_u16().unwrap_or(0);
    let type_filter = reader.read_u8().unwrap_or(0);
    let location_filter = reader.read_u8().unwrap_or(0);
    let level_filter = reader.read_u8().unwrap_or(0);

    send_party_bbs_list(
        world,
        sid,
        page_index,
        type_filter,
        location_filter,
        level_filter,
    );

    debug!(
        "[{}] PARTY_BBS_LIST: page={}, type={}, loc={}, lvl={}",
        session.addr(),
        page_index,
        type_filter,
        location_filter,
        level_filter
    );
    Ok(())
}

/// Build and send the party BBS list response packet.
/// Response format:
/// ```text
/// [WIZ_PARTY_BBS] [type=0] [sub=11] [status=1] [page:u16] [count:u16]
/// For each entry (up to MAX_BBS_PAGE=22):
///   [nation:u8] [seek_type:u8] [name:DByte_string] [class:u16]
///   [padding:u16=0] [level:u16] [role:u8]
///   [message:SByte_string] [zone:u8] [party_members:u8]
/// Remaining empty slots: 10 zero bytes each
/// [current_page:u16] [total_pages:u16]
/// ```
fn send_party_bbs_list(
    world: &WorldState,
    sid: SessionId,
    page_index: u16,
    type_filter: u8,
    location_filter: u8,
    level_filter: u8,
) {
    let start_offset = page_index as usize * MAX_BBS_PAGE;

    // Overflow check (C++ checks start_counter >= MAX_USER)
    if start_offset >= MAX_USER as usize {
        let mut result = Packet::new(Opcode::WizPartyBbs as u8);
        result.write_u8(PARTY_TYPE_SEEKING);
        result.write_u8(PARTY_BBS_LIST);
        result.write_u8(0); // status = fail
        world.send_to_session_owned(sid, result);
        return;
    }

    let mut all_entries = world.get_seeking_party_list();
    // Every free field bot of the viewer's nation is listed as a player looking for a party, so it can be
    // invited from this window like anybody else (the invite itself goes through the normal party UI by name).
    let viewer_nation = world.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
    all_entries.extend(crate::systems::field_bots::seeking_entries(world, viewer_nation));

    // Build the header with a placeholder count
    // [type=0][sub=11][status=1][page:u16][count:u16]
    let mut result = Packet::new(Opcode::WizPartyBbs as u8);
    result.write_u8(PARTY_TYPE_SEEKING);
    result.write_u8(PARTY_BBS_LIST);
    result.write_u8(1); // status = success
    result.write_u16(page_index);
    let count_offset = result.data.len();
    result.write_u16(0); // placeholder for bbs_count

    let mut global_index: usize = 0;
    let mut bbs_count: u16 = 0;
    let total_filtered =
        count_filtered_entries(&all_entries, type_filter, location_filter, level_filter);

    for entry in &all_entries {
        if !passes_filters(entry, type_filter, location_filter, level_filter) {
            continue;
        }

        // Pagination: skip entries before the current page
        if global_index < start_offset {
            global_index += 1;
            continue;
        }
        global_index += 1;

        if bbs_count >= MAX_BBS_PAGE as u16 {
            break;
        }

        let class = entry.class;
        let party_members: u8 = if entry.is_party_leader == 1 {
            world.get_party_member_count(entry.party_id)
        } else {
            0
        };
        let role: u8 = if entry.is_party_leader == 1 { 3 } else { 2 };

        // C++ wire format per entry:
        // result.DByte();
        // result << nation:u8 << seek_type:u8 << name:string << class:u16
        //     << padding:u16(0) << level:u16 << role:u8;
        // result.SByte();
        // result << message:string << zone:u8 << party_members:u8;
        result.write_u8(entry.nation);
        result.write_u8(entry.seek_type);
        result.write_string(&entry.name);
        result.write_u16(class);
        result.write_u16(0); // padding
        result.write_u16(entry.level as u16);
        result.write_u8(role);
        result.write_sbyte_string(&entry.seeking_note);
        result.write_u8(entry.zone);
        result.write_u8(party_members);

        bbs_count += 1;
    }

    // Fill remaining empty slots with zero padding (10 bytes each)
    for _ in bbs_count as usize..MAX_BBS_PAGE {
        result.write_u16(0);
        result.write_u16(0);
        result.write_u16(0);
        result.write_u8(0);
        result.write_u8(0);
        result.write_u8(0);
        result.write_u16(0);
        result.write_u8(0);
    }

    // Calculate total pages
    let mut total_pages = total_filtered / MAX_BBS_PAGE;
    if !total_filtered.is_multiple_of(MAX_BBS_PAGE) {
        total_pages += 1;
    }

    result.write_u16(page_index);
    result.write_u16(total_pages as u16);

    // Patch the count at the placeholder offset
    result.put_u16_at(count_offset, bbs_count);

    world.send_to_session_owned(sid, result);
}

/// Check if a seeking party entry passes all given filters.
fn passes_filters(
    entry: &SeekingPartyUser,
    type_filter: u8,
    location_filter: u8,
    level_filter: u8,
) -> bool {
    if entry.login_type == 2 {
        return false;
    }
    if type_filter == 2 && entry.is_party_leader == 1 {
        return false;
    }
    if type_filter == 3 && entry.is_party_leader == 0 {
        return false;
    }
    if location_filter > 0 && location_filter != entry.zone {
        return false;
    }
    matches_level_filter(level_filter, entry.level)
}

/// Count the total number of entries matching the given filters.
fn count_filtered_entries(
    entries: &[SeekingPartyUser],
    type_filter: u8,
    location_filter: u8,
    level_filter: u8,
) -> usize {
    entries
        .iter()
        .filter(|e| passes_filters(e, type_filter, location_filter, level_filter))
        .count()
}

/// Check if a level matches the given level filter bracket.
/// Level brackets:
/// - 0: all levels
/// - 1: 1-10
/// - 2: 11-20
/// - 3: 21-30
/// - 4: 31-40
/// - 5: 41-50
/// - 6: 51-60
/// - 7: 61-70
/// - 8: 71-80
/// - 9: 81+
fn matches_level_filter(filter: u8, level: i16) -> bool {
    match filter {
        0 => true,
        1 => level <= 11,
        2 => (11..=20).contains(&level),
        3 => (21..=30).contains(&level),
        4 => (31..=40).contains(&level),
        5 => (41..=50).contains(&level),
        6 => (51..=60).contains(&level),
        7 => (61..=70).contains(&level),
        8 => (71..=80).contains(&level),
        9 => level >= 81,
        _ => true,
    }
}

/// Broadcast a WIZ_STATE_CHANGE to the 3x3 region grid around a player.
/// Used to announce party-seeking status changes:
/// - `(2, 1)` = not looking
/// - `(2, 2)` = seeking a party (individual)
/// - `(2, 3)` = party leader seeking members
async fn broadcast_state_change(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    b_type: u8,
    n_buff: u32,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizStateChange as u8);
    pkt.write_u32(sid as u32);
    pkt.write_u8(b_type);
    pkt.write_u32(n_buff);

    let arc_pkt = Arc::new(pkt);
    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::clone(&arc_pkt),
            Some(sid),
            event_room,
        );
    }
    // Also send to self
    session.send_packet(&arc_pkt).await?;
    Ok(())
}

/// Truncate a message to MAX_BBS_MESSAGE characters.
fn truncate_message(msg: &str) -> String {
    if msg.len() > MAX_BBS_MESSAGE {
        msg[..MAX_BBS_MESSAGE].to_string()
    } else {
        msg.to_string()
    }
}

/// Remove a player from the seeking party list on logout.
/// Called from the disconnect cleanup path.
pub fn cleanup_on_disconnect(world: &WorldState, sid: SessionId) {
    world.remove_seeking_party(sid);
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    use super::*;
    use crate::world::{CharacterInfo, Position, WorldState};
    use ko_protocol::PacketReader;
    use tokio::sync::mpsc;

    fn make_test_char(sid: u16, name: &str, nation: u8, level: u8, class: u16) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation,
            race: 1,
            class,
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

    fn make_test_pos(zone_id: u16) -> Position {
        Position {
            zone_id,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        }
    }

    fn make_seeking_entry(
        sid: u16,
        name: &str,
        nation: u8,
        level: i16,
        class: u16,
        zone: u8,
        is_leader: bool,
        party_id: u16,
        msg: &str,
    ) -> SeekingPartyUser {
        SeekingPartyUser {
            sid,
            class,
            is_party_leader: u8::from(is_leader),
            level,
            zone,
            seeking_note: msg.to_string(),
            name: name.to_string(),
            nation,
            party_id,
            seek_type: 0,
            login_type: 0,
        }
    }

    // ── Level filter tests ──────────────────────────────────────────────

    #[test]
    fn test_level_filter_all() {
        assert!(matches_level_filter(0, 1));
        assert!(matches_level_filter(0, 83));
    }

    #[test]
    fn test_level_filter_bracket_1() {
        assert!(matches_level_filter(1, 1));
        assert!(matches_level_filter(1, 10));
        assert!(matches_level_filter(1, 11)); // C++ bracket 1 accepts up to 11
        assert!(!matches_level_filter(1, 12));
    }

    #[test]
    fn test_level_filter_bracket_2() {
        assert!(!matches_level_filter(2, 10));
        assert!(matches_level_filter(2, 11));
        assert!(matches_level_filter(2, 20));
        assert!(!matches_level_filter(2, 21));
    }

    #[test]
    fn test_level_filter_bracket_mid() {
        // Bracket 5 = 41-50
        assert!(matches_level_filter(5, 41));
        assert!(matches_level_filter(5, 50));
        assert!(!matches_level_filter(5, 40));
        assert!(!matches_level_filter(5, 51));
    }

    #[test]
    fn test_level_filter_bracket_9() {
        assert!(!matches_level_filter(9, 80));
        assert!(matches_level_filter(9, 81));
        assert!(matches_level_filter(9, 83));
    }

    #[test]
    fn test_level_filter_unknown() {
        // Unknown filter values pass all levels
        assert!(matches_level_filter(10, 50));
        assert!(matches_level_filter(255, 1));
    }

    // ── Seeking party storage tests ─────────────────────────────────────

    #[test]
    fn test_register_and_remove_seeking() {
        let world = WorldState::new();
        let entry = make_seeking_entry(1, "TestUser", 1, 60, 101, 21, false, 0, "LFG");

        world.register_seeking_party(entry);
        let list = world.get_seeking_party_list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "TestUser");

        world.remove_seeking_party(1);
        let list = world.get_seeking_party_list();
        assert!(list.is_empty());
    }

    #[test]
    fn test_register_update_existing() {
        let world = WorldState::new();
        let entry1 = make_seeking_entry(1, "TestUser", 1, 60, 101, 21, false, 0, "LFG");
        world.register_seeking_party(entry1);

        let entry2 = make_seeking_entry(1, "TestUser", 1, 60, 101, 21, false, 0, "Updated msg");
        world.register_seeking_party(entry2);

        let list = world.get_seeking_party_list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].seeking_note, "Updated msg");
    }

    #[test]
    fn test_remove_nonexistent() {
        let world = WorldState::new();
        world.remove_seeking_party(999);
        assert!(world.get_seeking_party_list().is_empty());
    }

    #[test]
    fn test_multiple_entries() {
        let world = WorldState::new();
        for i in 1..=5u16 {
            let entry =
                make_seeking_entry(i, &format!("User{}", i), 1, 60, 101, 21, false, 0, "LFG");
            world.register_seeking_party(entry);
        }
        assert_eq!(world.get_seeking_party_list().len(), 5);

        world.remove_seeking_party(3);
        let list = world.get_seeking_party_list();
        assert_eq!(list.len(), 4);
        assert!(list.iter().all(|e| e.sid != 3));
    }

    // ── Filter tests ────────────────────────────────────────────────────

    #[test]
    fn test_passes_filters_no_filter() {
        let entry = make_seeking_entry(1, "A", 1, 30, 101, 21, false, 0, "");
        assert!(passes_filters(&entry, 0, 0, 0));
    }

    #[test]
    fn test_passes_filters_hidden() {
        let mut entry = make_seeking_entry(1, "A", 1, 30, 101, 21, false, 0, "");
        entry.login_type = 2;
        assert!(!passes_filters(&entry, 0, 0, 0));
    }

    #[test]
    fn test_passes_filters_type_filter_2() {
        let leader = make_seeking_entry(1, "A", 1, 30, 101, 21, true, 100, "");
        let solo = make_seeking_entry(2, "B", 1, 30, 101, 21, false, 0, "");
        // type_filter=2: hide leaders
        assert!(!passes_filters(&leader, 2, 0, 0));
        assert!(passes_filters(&solo, 2, 0, 0));
    }

    #[test]
    fn test_passes_filters_type_filter_3() {
        let leader = make_seeking_entry(1, "A", 1, 30, 101, 21, true, 100, "");
        let solo = make_seeking_entry(2, "B", 1, 30, 101, 21, false, 0, "");
        // type_filter=3: hide non-leaders
        assert!(passes_filters(&leader, 3, 0, 0));
        assert!(!passes_filters(&solo, 3, 0, 0));
    }

    #[test]
    fn test_passes_filters_location() {
        let entry = make_seeking_entry(1, "A", 1, 30, 101, 21, false, 0, "");
        assert!(passes_filters(&entry, 0, 21, 0));
        assert!(!passes_filters(&entry, 0, 11, 0));
        assert!(passes_filters(&entry, 0, 0, 0)); // 0 = all locations
    }

    #[test]
    fn test_passes_filters_level() {
        let entry = make_seeking_entry(1, "A", 1, 25, 101, 21, false, 0, "");
        assert!(passes_filters(&entry, 0, 0, 3)); // 21-30
        assert!(!passes_filters(&entry, 0, 0, 4)); // 31-40
    }

    #[test]
    fn test_count_filtered_entries() {
        let entries = vec![
            make_seeking_entry(1, "A", 1, 30, 101, 21, false, 0, ""),
            make_seeking_entry(2, "B", 1, 60, 201, 21, true, 100, ""),
            make_seeking_entry(3, "C", 2, 45, 103, 11, false, 0, ""),
        ];
        assert_eq!(count_filtered_entries(&entries, 0, 0, 0), 3);
        assert_eq!(count_filtered_entries(&entries, 2, 0, 0), 2); // hide leaders
        assert_eq!(count_filtered_entries(&entries, 3, 0, 0), 1); // only leaders
        assert_eq!(count_filtered_entries(&entries, 0, 21, 0), 2); // zone 21 only
        assert_eq!(count_filtered_entries(&entries, 0, 0, 3), 1); // level 21-30
        assert_eq!(count_filtered_entries(&entries, 0, 0, 6), 1); // level 51-60
    }

    // ── Message truncation tests ────────────────────────────────────────

    #[test]
    fn test_truncate_short_message() {
        assert_eq!(truncate_message("Hello"), "Hello");
    }

    #[test]
    fn test_truncate_long_message() {
        let long_msg = "A".repeat(50);
        assert_eq!(truncate_message(&long_msg).len(), MAX_BBS_MESSAGE);
    }

    #[test]
    fn test_truncate_exact_limit() {
        let msg = "A".repeat(MAX_BBS_MESSAGE);
        assert_eq!(truncate_message(&msg).len(), MAX_BBS_MESSAGE);
    }

    // ── List response packet format tests ───────────────────────────────

    #[test]
    fn test_list_empty_response() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "Test", 1, 60, 101), make_test_pos(21));

        send_party_bbs_list(&world, 1, 0, 0, 0, 0);

        let pkt = rx.try_recv().unwrap();
        assert_eq!(pkt.opcode, Opcode::WizPartyBbs as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_TYPE_SEEKING)); // type
        assert_eq!(r.read_u8(), Some(PARTY_BBS_LIST)); // sub
        assert_eq!(r.read_u8(), Some(1)); // status
        assert_eq!(r.read_u16(), Some(0)); // page
        assert_eq!(r.read_u16(), Some(0)); // count = 0

        // 22 empty slots x 10 bytes = 220 bytes of padding
        for _ in 0..MAX_BBS_PAGE {
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
        }

        assert_eq!(r.read_u16(), Some(0)); // current page
        assert_eq!(r.read_u16(), Some(0)); // total pages (0 entries = 0 pages)
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_list_single_entry_wire_format() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(
            1,
            make_test_char(1, "Player1", 1, 60, 103),
            make_test_pos(21),
        );

        let entry = make_seeking_entry(10, "Seeker", 1, 45, 202, 21, false, 0, "Need party");
        world.register_seeking_party(entry);

        send_party_bbs_list(&world, 1, 0, 0, 0, 0);

        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);

        // Header
        assert_eq!(r.read_u8(), Some(PARTY_TYPE_SEEKING));
        assert_eq!(r.read_u8(), Some(PARTY_BBS_LIST));
        assert_eq!(r.read_u8(), Some(1)); // status
        assert_eq!(r.read_u16(), Some(0)); // page
        assert_eq!(r.read_u16(), Some(1)); // count = 1

        // Entry 1
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u8(), Some(0)); // seek_type
        let name = r.read_string().unwrap();
        assert_eq!(name, "Seeker");
        assert_eq!(r.read_u16(), Some(202)); // class
        assert_eq!(r.read_u16(), Some(0)); // padding
        assert_eq!(r.read_u16(), Some(45)); // level
        assert_eq!(r.read_u8(), Some(2)); // role = 2 (not leader)
        let msg = r.read_sbyte_string().unwrap();
        assert_eq!(msg, "Need party");
        assert_eq!(r.read_u8(), Some(21)); // zone
        assert_eq!(r.read_u8(), Some(0)); // party_members = 0

        // 21 empty slots
        for _ in 0..21 {
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
            assert_eq!(r.read_u16(), Some(0));
            assert_eq!(r.read_u8(), Some(0));
        }

        assert_eq!(r.read_u16(), Some(0)); // current page
        assert_eq!(r.read_u16(), Some(1)); // total pages
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_list_leader_entry_shows_party_members() {
        let world = WorldState::new();
        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();
        let (tx3, mut rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = make_test_pos(21);
        world.register_ingame(1, make_test_char(1, "Leader", 1, 60, 101), pos);
        world.register_ingame(2, make_test_char(2, "Member", 1, 60, 101), pos);
        world.register_ingame(3, make_test_char(3, "Viewer", 1, 60, 101), pos);

        let pid = world.create_party(1).unwrap();
        world.add_party_member(pid, 2);

        let entry = make_seeking_entry(1, "Leader", 1, 60, 300, 21, true, pid, "LFM healer");
        world.register_seeking_party(entry);

        send_party_bbs_list(&world, 3, 0, 0, 0, 0);

        let pkt = rx3.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);

        // Skip header
        r.read_u8(); // type
        r.read_u8(); // sub
        r.read_u8(); // status
        r.read_u16(); // page
        assert_eq!(r.read_u16(), Some(1)); // count = 1

        // Entry
        assert_eq!(r.read_u8(), Some(1)); // nation
        r.read_u8(); // seek_type
        let name = r.read_string().unwrap();
        assert_eq!(name, "Leader");
        assert_eq!(r.read_u16(), Some(300)); // wanted class
        r.read_u16(); // padding
        assert_eq!(r.read_u16(), Some(60)); // level
        assert_eq!(r.read_u8(), Some(3)); // role = 3 (leader)
        let msg = r.read_sbyte_string().unwrap();
        assert_eq!(msg, "LFM healer");
        assert_eq!(r.read_u8(), Some(21)); // zone
        assert_eq!(r.read_u8(), Some(2)); // party_members = 2 (leader + 1 member)
    }

    #[test]
    fn test_list_pagination() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "Test", 1, 60, 101), make_test_pos(21));

        // Add 25 entries (more than one page)
        for i in 0..25u16 {
            let entry = make_seeking_entry(
                100 + i,
                &format!("User{}", i),
                1,
                30,
                101,
                21,
                false,
                0,
                "LFG",
            );
            world.register_seeking_party(entry);
        }

        // Page 0 should have 22 entries
        send_party_bbs_list(&world, 1, 0, 0, 0, 0);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8(); // type
        r.read_u8(); // sub
        r.read_u8(); // status
        r.read_u16(); // page
        assert_eq!(r.read_u16(), Some(22)); // count

        // Page 1 should have 3 entries
        send_party_bbs_list(&world, 1, 1, 0, 0, 0);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(3)); // count = 3 remaining
    }

    #[test]
    fn test_list_pagination_total_pages() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "T", 1, 60, 101), make_test_pos(21));

        for i in 0..45u16 {
            let entry =
                make_seeking_entry(100 + i, &format!("U{}", i), 1, 30, 101, 21, false, 0, "");
            world.register_seeking_party(entry);
        }

        // 45 entries / 22 per page = 3 pages (22 + 22 + 1)
        send_party_bbs_list(&world, 1, 0, 0, 0, 0);
        let pkt = rx.try_recv().unwrap();
        let data = &pkt.data;
        // Total pages is at the end: last 2 bytes before the 2 current_page bytes
        let len = data.len();
        let total_pages = u16::from_le_bytes([data[len - 2], data[len - 1]]);
        assert_eq!(total_pages, 3);
    }

    #[test]
    fn test_list_type_filter() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "T", 1, 60, 101), make_test_pos(21));

        let e1 = make_seeking_entry(10, "Solo", 1, 30, 101, 21, false, 0, "LFG");
        let e2 = make_seeking_entry(20, "Leader", 1, 60, 201, 21, true, 100, "LFM");
        world.register_seeking_party(e1);
        world.register_seeking_party(e2);

        // type_filter=2: only non-leaders
        send_party_bbs_list(&world, 1, 0, 2, 0, 0);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(1));

        // type_filter=3: only leaders
        send_party_bbs_list(&world, 1, 0, 3, 0, 0);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(1));
    }

    #[test]
    fn test_list_location_filter() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "T", 1, 60, 101), make_test_pos(21));

        let e1 = make_seeking_entry(10, "A", 1, 30, 101, 21, false, 0, "");
        let e2 = make_seeking_entry(20, "B", 1, 60, 101, 11, false, 0, "");
        world.register_seeking_party(e1);
        world.register_seeking_party(e2);

        send_party_bbs_list(&world, 1, 0, 0, 21, 0);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(1)); // only zone 21
    }

    #[test]
    fn test_list_level_filter() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "T", 1, 60, 101), make_test_pos(21));

        let e1 = make_seeking_entry(10, "Low", 1, 15, 101, 21, false, 0, "");
        let e2 = make_seeking_entry(20, "Mid", 1, 45, 101, 21, false, 0, "");
        let e3 = make_seeking_entry(30, "High", 1, 82, 101, 21, false, 0, "");
        world.register_seeking_party(e1);
        world.register_seeking_party(e2);
        world.register_seeking_party(e3);

        // Level bracket 2 = 11-20
        send_party_bbs_list(&world, 1, 0, 0, 0, 2);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(1)); // only "Low" (level 15)

        // Level bracket 9 = 81+
        send_party_bbs_list(&world, 1, 0, 0, 0, 9);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(1)); // only "High" (level 82)
    }

    #[test]
    fn test_list_overflow_page() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "T", 1, 60, 101), make_test_pos(21));

        let overflow_page = (MAX_USER / MAX_BBS_PAGE as u16) + 1;
        send_party_bbs_list(&world, 1, overflow_page, 0, 0, 0);

        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PARTY_TYPE_SEEKING));
        assert_eq!(r.read_u8(), Some(PARTY_BBS_LIST));
        assert_eq!(r.read_u8(), Some(0)); // status = fail
    }

    #[test]
    fn test_cleanup_on_disconnect() {
        let world = WorldState::new();
        let entry = make_seeking_entry(1, "User1", 1, 60, 101, 21, false, 0, "LFG");
        world.register_seeking_party(entry);
        assert_eq!(world.get_seeking_party_list().len(), 1);

        cleanup_on_disconnect(&world, 1);
        assert!(world.get_seeking_party_list().is_empty());
    }

    #[test]
    fn test_party_member_count() {
        let world = WorldState::new();
        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_ingame(1, make_test_char(1, "A", 1, 60, 101), make_test_pos(21));
        world.register_ingame(2, make_test_char(2, "B", 1, 60, 101), make_test_pos(21));

        let pid = world.create_party(1).unwrap();
        world.add_party_member(pid, 2);
        assert_eq!(world.get_party_member_count(pid), 2);
        assert_eq!(world.get_party_member_count(9999), 0);
    }

    #[test]
    fn test_combined_filters() {
        let world = WorldState::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.register_ingame(1, make_test_char(1, "T", 1, 60, 101), make_test_pos(21));

        // Zone 21, level 25, non-leader
        let e1 = make_seeking_entry(10, "A", 1, 25, 101, 21, false, 0, "");
        // Zone 21, level 65, leader
        let e2 = make_seeking_entry(20, "B", 1, 65, 201, 21, true, 100, "");
        // Zone 11, level 25, non-leader
        let e3 = make_seeking_entry(30, "C", 1, 25, 101, 11, false, 0, "");
        world.register_seeking_party(e1);
        world.register_seeking_party(e2);
        world.register_seeking_party(e3);

        // type_filter=2 (no leaders) + zone 21 + level bracket 3 (21-30)
        send_party_bbs_list(&world, 1, 0, 2, 21, 3);
        let pkt = rx.try_recv().unwrap();
        let mut r = PacketReader::new(&pkt.data);
        r.read_u8();
        r.read_u8();
        r.read_u8();
        r.read_u16();
        assert_eq!(r.read_u16(), Some(1)); // only entry A matches
    }
}
