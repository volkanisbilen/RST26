//! WIZ_RANK (0x80) handler — Player ranking system.
//! ## Rank Types (`RankTypes` enum in `packets.h:706-711`)
//! | Type | Name                       | Description                        |
//! |------|----------------------------|------------------------------------|
//! | 1    | RANK_TYPE_PK_ZONE          | PK zone daily loyalty ranking      |
//! | 2    | RANK_TYPE_ZONE_BORDER_DEF  | Border Defence War ranking         |
//! | 3    | RANK_TYPE_CHAOS_DUNGEON    | Chaos Dungeon kill/death ranking   |
//! ## Client -> Server (WIZ_RANK 0x80)
//! ```text
//! [u8 rank_type]
//! ```
//! ## Server -> Client (RANK_TYPE_PK_ZONE = 1)
//! ```text
//! [u8 RANK_TYPE_PK_ZONE]
//! For each nation (Karus=0, Elmorad=1):
//!   [u16 count]                             // entries in this nation (max 10)
//!   For each entry:
//!     [string name] [u8 nation]
//!     [u16 knights_id] [u16 mark_version] [string knights_name]
//!     [u32 loyalty_daily]
//!     [u16 loyalty_premium_bonus]           // capped at 999
//!     [i8 loyalty_symbol_rank]
//! [u16 my_rank] [u32 my_loyalty_daily] [u16 my_loyalty_premium_bonus]
//! ```
//! ## Server -> Client (RANK_TYPE_ZONE_BORDER_DEFENSE_WAR = 2)
//! ```text
//! [u8 RANK_TYPE_ZONE_BORDER_DEFENSE_WAR]
//! For each nation (Karus=0, Elmorad=1):
//!   [u16 count]                             // entries (max 8)
//!   For each entry:
//!     [string name] [u8 nation]
//!     [u16 knights_id] [u16 mark_version] [string knights_name]
//!     [u32 user_point]
//! [i64 gained_exp] [i64 premium_gained_exp]
//! ```
//! ## Server -> Client (RANK_TYPE_CHAOS_DUNGEON = 3)
//! ```text
//! [u8 RANK_TYPE_CHAOS_DUNGEON]
//! [u8 count]                                // entries (max 19)
//! For each entry:
//!   [string name] [u16 kill_count] [u16 death_count]
//! [i32 gained_exp] [i32 premium_gained_exp] [u32 1]
//! ```

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::systems::event_room::TempleEventType;
use crate::systems::event_system::{bdw_user_point_exp, chaos_user_exp};
use crate::world::{
    PkZoneRanking, RANK_TYPE_CHAOS_DUNGEON, RANK_TYPE_PK_ZONE, RANK_TYPE_ZONE_BORDER_DEFENSE_WAR,
};

/// Handle incoming WIZ_RANK (0x80) packet.
pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world();

    // Block while ranking update is in progress
    if world.is_ranking_update_in_progress() {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let rank_type = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match rank_type {
        RANK_TYPE_PK_ZONE => {
            // C++ checks isInSpecialEventZone() || isInWarZone() and routes to
            // HandleRankingSpecialEvent which uses zindan_rankings instead.
            let sid = session.session_id();
            let zone_id = world.with_session(sid, |h| h.position.zone_id).unwrap_or(0);
            let is_war_zone = world.get_zone(zone_id).is_some_and(|z| z.is_war_zone());

            if is_war_zone {
                handle_special_event_zone(session).await
            } else {
                handle_pk_zone(session).await
            }
        }
        RANK_TYPE_ZONE_BORDER_DEFENSE_WAR => handle_bdw(session).await,
        RANK_TYPE_CHAOS_DUNGEON => handle_chaos_dungeon(session).await,
        _ => {
            warn!(
                "[{}] WIZ_RANK: unhandled rank_type {}",
                session.addr(),
                rank_type
            );
            Ok(())
        }
    }
}

/// Handle PK zone ranking request (RANK_TYPE_PK_ZONE = 1).
/// Sends top 10 players per nation for the requester's current zone,
/// sorted by daily loyalty descending. Includes the requester's own rank.
async fn handle_pk_zone(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Get requester's info
    let (nation, zone_id, my_loyalty_daily, my_loyalty_premium) =
        match world.with_session(sid, |h| {
            h.character.as_ref().map(|c| {
                (
                    c.nation,
                    h.position.zone_id,
                    h.pk_loyalty_daily,
                    h.pk_loyalty_premium_bonus,
                )
            })
        }) {
            Some(Some(v)) => v,
            _ => return Ok(()),
        };

    // Gather sorted rankings per nation from pk_zone_rankings
    let rankings: [Vec<PkZoneRanking>; 2] = [
        world.pk_zone_get_sorted(0, zone_id),
        world.pk_zone_get_sorted(1, zone_id),
    ];

    let result = build_pk_ranking_packet(
        session,
        sid,
        nation,
        my_loyalty_daily,
        my_loyalty_premium,
        &rankings,
        zone_id,
        true,
        false, // PK zone: write each entry's own nation
    );
    session.send_packet(&result).await?;

    debug!("[{}] WIZ_RANK PK_ZONE: zone={}", session.addr(), zone_id,);

    Ok(())
}

/// Handle special event zone (Zindan War) ranking request.
/// Uses zindan_rankings instead of pk_zone_rankings, but same packet format.
async fn handle_special_event_zone(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    let (nation, zone_id, my_loyalty_daily, my_loyalty_premium) =
        match world.with_session(sid, |h| {
            h.character.as_ref().map(|c| {
                (
                    c.nation,
                    h.position.zone_id,
                    h.pk_loyalty_daily,
                    h.pk_loyalty_premium_bonus,
                )
            })
        }) {
            Some(Some(v)) => v,
            _ => return Ok(()),
        };

    let rankings: [Vec<PkZoneRanking>; 2] = [
        world.zindan_get_sorted(0, zone_id),
        world.zindan_get_sorted(1, zone_id),
    ];

    // C++ BUG: HandleRankingSpecialEvent writes the REQUESTER's nation for
    // every entry (line 315: `result << pUser->GetName() << GetNation()`),
    // not the entry's own nation. We replicate this for client parity.
    let result = build_pk_ranking_packet(
        session,
        sid,
        nation,
        my_loyalty_daily,
        my_loyalty_premium,
        &rankings,
        zone_id,
        false,
        true, // Special event: write requester's nation (C++ parity)
    );
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_RANK SPECIAL_EVENT: zone={}",
        session.addr(),
        zone_id,
    );

    Ok(())
}

/// Build a PK zone / special event ranking response packet.
/// Shared logic between `handle_pk_zone` and `handle_special_event_zone`.
/// The packet format is identical — only the data source differs.
/// When `use_requester_nation` is true, every entry's nation byte is the
/// requester's nation instead of the entry's own (C++ parity for special event).
fn build_pk_ranking_packet(
    session: &ClientSession,
    sid: crate::zone::SessionId,
    nation: u8,
    my_loyalty_daily: u32,
    my_loyalty_premium: u16,
    rankings: &[Vec<PkZoneRanking>; 2],
    zone_id: u16,
    include_runtime_bots: bool,
    use_requester_nation: bool,
) -> Packet {
    let world = session.world();
    let mut result = Packet::new(Opcode::WizRank as u8);
    result.write_u8(RANK_TYPE_PK_ZONE);

    let mut my_rank: u16 = 0;
    let mut my_rank_found = false;
    let mut nation_totals = [0usize; 2];

    for nation_idx in 0..2u8 {
        let sorted = &rankings[nation_idx as usize];
        let mut display_entries: Vec<(
            Option<crate::zone::SessionId>,
            String,
            u8,
            u16,
            u32,
            u16,
            i8,
        )> = Vec::new();

        for entry in sorted {
            let entry_data = world.with_session(entry.session_id, |h| {
                h.character
                    .as_ref()
                    .map(|c| (c.name.clone(), c.nation, c.knights_id))
            });
            if let Some(Some((name, entry_nation, knights_id))) = entry_data {
                display_entries.push((
                    Some(entry.session_id),
                    name,
                    entry_nation,
                    knights_id,
                    entry.loyalty_daily,
                    entry.loyalty_premium_bonus.min(999),
                    world.get_loyalty_symbol_rank(entry.session_id),
                ));
            }
        }

        if include_runtime_bots {
            for bot in world
                .get_bots_in_zone_live(zone_id)
                .into_iter()
                .filter(|bot| bot.nation == nation_idx + 1)
            {
                let symbol_rank = match (bot.personal_rank, bot.knights_rank) {
                    (0, 0) => -1,
                    (p, 0) => p as i8,
                    (0, k) => k as i8,
                    (p, k) => p.min(k) as i8,
                };
                display_entries.push((
                    None,
                    bot.name,
                    bot.nation,
                    bot.knights_id,
                    bot.loyalty_daily,
                    0,
                    symbol_rank,
                ));
            }
        }

        display_entries.sort_by(|a, b| b.4.cmp(&a.4).then_with(|| a.1.cmp(&b.1)));
        nation_totals[nation_idx as usize] = display_entries.len();

        let count_offset = result.wpos();
        result.write_u16(0);

        let mut count: u16 = 0;

        for (entry_sid, name, entry_nation, knights_id, loyalty, premium, symbol_rank) in
            display_entries
        {
            if !my_rank_found && (nation_idx + 1) == nation {
                my_rank += 1;
                if entry_sid == Some(sid) {
                    my_rank_found = true;
                }
            }

            if count >= 10 {
                if my_rank_found || (nation_idx + 1) != nation {
                    break;
                }
                continue;
            }

            result.write_string(&name);
            // C++ PK zone writes entry's own nation; special event writes requester's.
            result.write_u8(if use_requester_nation {
                nation
            } else {
                entry_nation
            });

            if knights_id == 0 || knights_id == 0xFFFF {
                result.write_u16(0);
                result.write_u16(0);
                result.write_string("");
            } else {
                match world.get_knights(knights_id) {
                    Some(ki) => {
                        result.write_u16(ki.id);
                        result.write_u16(ki.mark_version);
                        result.write_string(&ki.name);
                    }
                    None => {
                        result.write_u16(0);
                        result.write_u16(0);
                        result.write_string("");
                    }
                }
            }

            result.write_u32(loyalty);
            result.write_u16(premium);
            result.write_i8(symbol_rank);

            count += 1;
        }

        result.put_u16_at(count_offset, count);
    }

    // If player is ranked > 10 and total > 9, multiply total by CzRank.
    if my_rank > 10 {
        let my_nation_total = nation_totals[(nation.saturating_sub(1)) as usize] as u16;
        if my_nation_total > 9 {
            let cz_rank = world.rank_bug.read().cz_rank as u16;
            if cz_rank > 0 {
                my_rank = my_nation_total.saturating_mul(cz_rank);
            }
        }
    }

    result.write_u16(my_rank);
    result.write_u32(my_loyalty_daily);
    result.write_u16(my_loyalty_premium);

    result
}

/// Handle Border Defence War ranking request (RANK_TYPE_ZONE_BORDER_DEFENSE_WAR = 2).
/// Reads real ranking data from the EventRoom. Each nation's users are sorted by
/// `bdw_points` descending and up to 8 entries are written per nation. The
/// requester's own accumulated EXP preview (display-only) is appended.
async fn handle_bdw(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Get requester's character info
    let (char_name, requester_nation, requester_level, my_bdw_points) =
        match world.with_session(sid, |h| {
            h.character
                .as_ref()
                .map(|c| (c.name.clone(), c.nation, c.level, 0u32))
        }) {
            Some(Some(v)) => v,
            _ => return Ok(()),
        };

    // Check BDW is active
    let is_active = world
        .event_room_manager()
        .read_temple_event(|te| te.is_bdw_active());
    if !is_active {
        // Send empty response when event is not active
        let mut result = Packet::new(Opcode::WizRank as u8);
        result.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);
        result.write_u16(0);
        result.write_u16(0);
        result.write_u64(0);
        result.write_u64(0);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let erm = world.event_room_manager();

    // Find requester's room
    let room_id = match erm.find_user_room(TempleEventType::BorderDefenceWar, &char_name) {
        Some((rid, _)) => rid,
        None => {
            // User not in any BDW room — send empty
            let mut result = Packet::new(Opcode::WizRank as u8);
            result.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);
            result.write_u16(0);
            result.write_u16(0);
            result.write_u64(0);
            result.write_u64(0);
            session.send_packet(&result).await?;
            return Ok(());
        }
    };

    // Collect user data from room (two nations)
    // Struct: (name, nation, session_id, bdw_points)
    let (karus_entries, elmo_entries, my_points) = {
        let Some(room) = erm.get_room(TempleEventType::BorderDefenceWar, room_id) else {
            let mut result = Packet::new(Opcode::WizRank as u8);
            result.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);
            result.write_u16(0);
            result.write_u16(0);
            result.write_u64(0);
            result.write_u64(0);
            session.send_packet(&result).await?;
            return Ok(());
        };

        let mut karus: Vec<(String, crate::zone::SessionId, u32)> = Vec::with_capacity(8);
        let mut elmo: Vec<(String, crate::zone::SessionId, u32)> = Vec::with_capacity(8);
        let mut my_pts = my_bdw_points;

        for user in room.karus_users.values() {
            if !user.user_name.is_empty() && !user.logged_out {
                karus.push((user.user_name.clone(), user.session_id, user.bdw_points));
            }
            if user.user_name == char_name {
                my_pts = user.bdw_points;
            }
        }
        for user in room.elmorad_users.values() {
            if !user.user_name.is_empty() && !user.logged_out {
                elmo.push((user.user_name.clone(), user.session_id, user.bdw_points));
            }
            if user.user_name == char_name {
                my_pts = user.bdw_points;
            }
        }

        // Sort by bdw_points descending
        karus.sort_by(|a, b| b.2.cmp(&a.2));
        elmo.sort_by(|a, b| b.2.cmp(&a.2));

        (karus, elmo, my_pts)
    }; // room lock dropped

    let mut result = Packet::new(Opcode::WizRank as u8);
    result.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);

    // Write entries for each nation (max 8 per nation)
    for entries in [&karus_entries, &elmo_entries] {
        let count_offset = result.wpos();
        result.write_u16(0); // placeholder

        let mut count: u16 = 0;
        for (name, entry_sid, bdw_points) in entries.iter() {
            if count >= 8 {
                break;
            }

            // Verify user is still in-game
            let user_data =
                world.with_session(*entry_sid, |h| h.character.as_ref().map(|c| c.knights_id));

            let knights_id = match user_data {
                Some(Some(kid)) => kid,
                _ => continue,
            };

            result.write_string(name);
            // C++ BUG: writes requester's nation for all entries (line 408)
            result.write_u8(requester_nation);

            if knights_id == 0 || knights_id == 0xFFFF {
                result.write_u16(0);
                result.write_u16(0);
                result.write_string("");
            } else {
                match world.get_knights(knights_id) {
                    Some(ki) => {
                        result.write_u16(ki.id);
                        result.write_u16(ki.mark_version);
                        result.write_string(&ki.name);
                    }
                    None => {
                        result.write_u16(0);
                        result.write_u16(0);
                        result.write_string("");
                    }
                }
            }

            result.write_u32(*bdw_points);
            count += 1;
        }

        result.put_u16_at(count_offset, count);
    }

    // Requester's own EXP preview (display-only)
    let (gained_exp, premium_gained_exp) = bdw_user_point_exp(requester_level, my_points);
    result.write_i64(gained_exp);
    result.write_i64(premium_gained_exp);

    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_RANK BDW: room={}, my_points={}, exp={}/{}",
        session.addr(),
        room_id,
        my_points,
        gained_exp,
        premium_gained_exp,
    );

    Ok(())
}

/// Handle Chaos Dungeon ranking request (RANK_TYPE_CHAOS_DUNGEON = 3).
/// Reads real ranking data from the EventRoom. All users in the same room are
/// sorted by kills descending (deaths ascending as tiebreaker) and up to 19
/// entries are written. The requester's own EXP preview is appended.
async fn handle_chaos_dungeon(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Get requester's character info
    let (char_name, requester_level) = match world.with_session(sid, |h| {
        h.character.as_ref().map(|c| (c.name.clone(), c.level))
    }) {
        Some(Some(v)) => v,
        _ => return Ok(()),
    };

    // Check Chaos is active
    let is_active = world
        .event_room_manager()
        .read_temple_event(|te| te.is_chaos_active());
    if !is_active {
        let mut result = Packet::new(Opcode::WizRank as u8);
        result.write_u8(RANK_TYPE_CHAOS_DUNGEON);
        result.write_u8(0);
        result.write_i32(0);
        result.write_i32(0);
        result.write_u32(1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let erm = world.event_room_manager();

    // Find requester's room
    let room_id = match erm.find_user_room(TempleEventType::ChaosDungeon, &char_name) {
        Some((rid, _)) => rid,
        None => {
            let mut result = Packet::new(Opcode::WizRank as u8);
            result.write_u8(RANK_TYPE_CHAOS_DUNGEON);
            result.write_u8(0);
            result.write_i32(0);
            result.write_i32(0);
            result.write_u32(1);
            session.send_packet(&result).await?;
            return Ok(());
        }
    };

    // Collect user data from room
    // Struct: (name, session_id, kills, deaths)
    let (mut entries, my_kills, my_deaths) = {
        let Some(room) = erm.get_room(TempleEventType::ChaosDungeon, room_id) else {
            let mut result = Packet::new(Opcode::WizRank as u8);
            result.write_u8(RANK_TYPE_CHAOS_DUNGEON);
            result.write_u8(0);
            result.write_i32(0);
            result.write_i32(0);
            result.write_u32(1);
            session.send_packet(&result).await?;
            return Ok(());
        };

        let mut data: Vec<(String, crate::zone::SessionId, u32, u32)> = Vec::with_capacity(24);
        let mut mk = 0u32;
        let mut md = 0u32;

        for user in room.mixed_users.values() {
            if user.user_name.is_empty() || user.logged_out {
                continue;
            }
            data.push((
                user.user_name.clone(),
                user.session_id,
                user.kills,
                user.deaths,
            ));
            if user.user_name == char_name {
                mk = user.kills;
                md = user.deaths;
            }
        }

        (data, mk, md)
    }; // room lock dropped

    // Sort by kills descending, then deaths ascending as tiebreaker
    entries.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.3.cmp(&b.3)));

    let mut result = Packet::new(Opcode::WizRank as u8);
    result.write_u8(RANK_TYPE_CHAOS_DUNGEON);

    let count_offset = result.wpos();
    result.write_u8(0); // placeholder

    let mut count: u8 = 0;
    for (name, entry_sid, kills, deaths) in &entries {
        if count > 18 {
            break;
        }

        // Verify user is still in-game
        if world
            .with_session(*entry_sid, |h| h.character.is_some())
            .unwrap_or(false)
        {
            result.write_string(name);
            result.write_u16(*kills as u16);
            result.write_u16(*deaths as u16);
            count += 1;
        }
    }

    // Backpatch count
    result.data[count_offset] = count;

    // Requester's own EXP preview (display-only)
    let (gained_exp, premium_gained_exp) = chaos_user_exp(requester_level, my_kills, my_deaths);
    result.write_i32(gained_exp as i32);
    result.write_i32(premium_gained_exp as i32);
    result.write_u32(1); // trailing constant

    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_RANK CHAOS: room={}, entries={}, my_kills={}, my_deaths={}, exp={}/{}",
        session.addr(),
        room_id,
        count,
        my_kills,
        my_deaths,
        gained_exp,
        premium_gained_exp,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::Packet;

    #[test]
    fn test_pk_zone_ranking_packet_format() {
        // Build a minimal PK zone ranking response with 0 entries per nation
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_PK_ZONE);

        // Karus: 0 entries
        let karus_offset = pkt.wpos();
        pkt.write_u16(0);
        pkt.put_u16_at(karus_offset, 0);

        // El Morad: 0 entries
        let elmo_offset = pkt.wpos();
        pkt.write_u16(0);
        pkt.put_u16_at(elmo_offset, 0);

        // My rank info
        pkt.write_u16(0); // my_rank
        pkt.write_u32(0); // my_loyalty_daily
        pkt.write_u16(0); // my_loyalty_premium_bonus

        assert_eq!(pkt.opcode, 0x80);
        // Data: [1] + [0,0] + [0,0] + [0,0] + [0,0,0,0] + [0,0] = 1+2+2+2+4+2 = 13
        assert_eq!(pkt.data.len(), 13);
        assert_eq!(pkt.data[0], RANK_TYPE_PK_ZONE);
    }

    #[test]
    fn test_pk_zone_ranking_with_one_entry() {
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_PK_ZONE);

        // Karus: 1 entry
        let karus_offset = pkt.wpos();
        pkt.write_u16(0); // placeholder

        // Entry: name="TestUser", nation=1, no clan, loyalty=500, premium=10, rank=-1
        pkt.write_string("TestUser");
        pkt.write_u8(1); // nation
        pkt.write_u16(0); // no knights_id
        pkt.write_u16(0); // no mark_version
        pkt.write_string(""); // empty knights name
        pkt.write_u32(500); // loyalty_daily
        pkt.write_u16(10); // loyalty_premium_bonus
        pkt.write_i8(-1); // loyalty_symbol_rank

        // Backpatch count
        pkt.put_u16_at(karus_offset, 1);

        // El Morad: 0 entries
        pkt.write_u16(0);

        // My rank info
        pkt.write_u16(1); // rank 1
        pkt.write_u32(500);
        pkt.write_u16(10);

        // Verify the backpatched count
        assert_eq!(pkt.data[1], 1); // count low byte
        assert_eq!(pkt.data[2], 0); // count high byte

        // Verify opcode
        assert_eq!(pkt.opcode, 0x80);
    }

    #[test]
    fn test_bdw_ranking_empty_response() {
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);
        pkt.write_u16(0); // Karus
        pkt.write_u16(0); // El Morad
        pkt.write_u64(0); // gained_exp
        pkt.write_u64(0); // premium_gained_exp

        assert_eq!(pkt.opcode, 0x80);
        // Data: [2] + [0,0] + [0,0] + 8 + 8 = 1+2+2+8+8 = 21
        assert_eq!(pkt.data.len(), 21);
        assert_eq!(pkt.data[0], RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);
    }

    #[test]
    fn test_chaos_dungeon_ranking_empty_response() {
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_CHAOS_DUNGEON);
        pkt.write_u8(0); // count
        pkt.write_i32(0); // gained_exp
        pkt.write_i32(0); // premium_gained_exp
        pkt.write_u32(1); // trailing constant

        assert_eq!(pkt.opcode, 0x80);
        // Data: [3] + [0] + 4 + 4 + 4 = 1+1+4+4+4 = 14
        assert_eq!(pkt.data.len(), 14);
        assert_eq!(pkt.data[0], RANK_TYPE_CHAOS_DUNGEON);
    }

    #[test]
    fn test_premium_bonus_cap() {
        // Verify premium bonus is capped at 999 (C++ code caps at 999)
        let value: u16 = 1500;
        let capped = if value > 999 { 999 } else { value };
        assert_eq!(capped, 999);

        let value2: u16 = 500;
        let capped2 = if value2 > 999 { 999 } else { value2 };
        assert_eq!(capped2, 500);
    }

    #[test]
    fn test_special_event_zone_uses_pk_zone_opcode() {
        // Both normal PK zone and special event zone use RANK_TYPE_PK_ZONE (1)
        // as the rank type in the response packet, per C++ code.
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_PK_ZONE);
        pkt.write_u16(0); // Karus count
        pkt.write_u16(0); // El Morad count
        pkt.write_u16(0); // my_rank
        pkt.write_u32(0); // my_loyalty_daily
        pkt.write_u16(0); // my_loyalty_premium_bonus

        assert_eq!(pkt.opcode, 0x80);
        assert_eq!(pkt.data[0], RANK_TYPE_PK_ZONE);
    }

    #[test]
    fn test_rank_type_constants_match_cpp() {
        assert_eq!(RANK_TYPE_PK_ZONE, 1);
        assert_eq!(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR, 2);
        assert_eq!(RANK_TYPE_CHAOS_DUNGEON, 3);
    }

    #[test]
    fn test_bdw_ranking_with_entries_packet_format() {
        // Build a BDW ranking response with 1 Karus entry and 0 El Morad entries
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);

        // Karus: 1 entry
        let karus_offset = pkt.wpos();
        pkt.write_u16(0); // placeholder

        // Entry: name="Warrior1", nation=1 (requester's), no clan, 5 bdw_points
        pkt.write_string("Warrior1");
        pkt.write_u8(1); // C++ BUG: always writes requester's nation
        pkt.write_u16(0); // no knights
        pkt.write_u16(0);
        pkt.write_string("");
        pkt.write_u32(5); // bdw_points

        pkt.put_u16_at(karus_offset, 1);

        // El Morad: 0 entries
        pkt.write_u16(0);

        // EXP preview (level=50, bdw_points=5)
        let (exp, prem) = bdw_user_point_exp(50, 5);
        pkt.write_i64(exp);
        pkt.write_i64(prem);

        assert_eq!(pkt.opcode, 0x80);
        assert_eq!(pkt.data[0], RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);

        // Verify backpatched Karus count = 1
        assert_eq!(pkt.data[1], 1); // count low byte
        assert_eq!(pkt.data[2], 0); // count high byte

        // EXP formula: 50^3 * 0.15 * (5 * 5) = 125000 * 0.15 * 25 = 468750
        assert_eq!(exp, 468_750);
        assert_eq!(prem, 937_500);
    }

    #[test]
    fn test_bdw_ranking_sorts_by_points_desc() {
        // Verify sorting order: highest bdw_points first
        let mut entries: Vec<(&str, u32)> = vec![
            ("Player1", 3),
            ("Player2", 10),
            ("Player3", 1),
            ("Player4", 7),
        ];
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        assert_eq!(entries[0].0, "Player2"); // 10 points
        assert_eq!(entries[1].0, "Player4"); // 7 points
        assert_eq!(entries[2].0, "Player1"); // 3 points
        assert_eq!(entries[3].0, "Player3"); // 1 point
    }

    #[test]
    fn test_bdw_ranking_max_8_per_nation() {
        // Verify that at most 8 entries are written per nation
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);

        let karus_offset = pkt.wpos();
        pkt.write_u16(0);

        let mut count: u16 = 0;
        for i in 0..12u32 {
            if count >= 8 {
                break;
            }
            pkt.write_string(&format!("Player{}", i));
            pkt.write_u8(1);
            pkt.write_u16(0);
            pkt.write_u16(0);
            pkt.write_string("");
            pkt.write_u32(100 - i);
            count += 1;
        }
        pkt.put_u16_at(karus_offset, count);

        assert_eq!(count, 8);
    }

    #[test]
    fn test_chaos_ranking_with_entries_packet_format() {
        // Build a Chaos ranking response with 2 entries
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_CHAOS_DUNGEON);

        let count_offset = pkt.wpos();
        pkt.write_u8(0); // placeholder

        // Entry 1: "Killer1", 5 kills, 1 death
        pkt.write_string("Killer1");
        pkt.write_u16(5);
        pkt.write_u16(1);

        // Entry 2: "Killer2", 3 kills, 2 deaths
        pkt.write_string("Killer2");
        pkt.write_u16(3);
        pkt.write_u16(2);

        pkt.data[count_offset] = 2;

        // EXP preview (level=60, kills=5, deaths=1)
        let (exp, prem) = chaos_user_exp(60, 5, 1);
        pkt.write_i32(exp as i32);
        pkt.write_i32(prem as i32);
        pkt.write_u32(1);

        assert_eq!(pkt.opcode, 0x80);
        assert_eq!(pkt.data[0], RANK_TYPE_CHAOS_DUNGEON);
        assert_eq!(pkt.data[count_offset], 2);

        // EXP formula: 60^3 * 0.15 * (5*5 - 1) = 216000 * 0.15 * 24 = 777600
        assert_eq!(exp, 777_600);
        assert_eq!(prem, 1_555_200);
    }

    #[test]
    fn test_chaos_ranking_sorts_kills_desc_deaths_asc() {
        // Verify sorting: kills descending, deaths ascending as tiebreaker
        let mut entries: Vec<(&str, u32, u32)> = vec![
            ("A", 5, 3), // 5 kills, 3 deaths
            ("B", 5, 1), // 5 kills, 1 death (should rank higher — fewer deaths)
            ("C", 3, 0), // 3 kills, 0 deaths
            ("D", 8, 5), // 8 kills — top
        ];
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));

        assert_eq!(entries[0].0, "D"); // 8 kills
        assert_eq!(entries[1].0, "B"); // 5 kills, 1 death
        assert_eq!(entries[2].0, "A"); // 5 kills, 3 deaths
        assert_eq!(entries[3].0, "C"); // 3 kills
    }

    #[test]
    fn test_chaos_ranking_max_19_entries() {
        // C++ allows up to 19 entries (sCount > 18 → break)
        let mut pkt = Packet::new(Opcode::WizRank as u8);
        pkt.write_u8(RANK_TYPE_CHAOS_DUNGEON);

        let count_offset = pkt.wpos();
        pkt.write_u8(0);

        let mut count: u8 = 0;
        for i in 0..25u16 {
            if count > 18 {
                break;
            }
            pkt.write_string(&format!("P{}", i));
            pkt.write_u16(50 - i);
            pkt.write_u16(i);
            count += 1;
        }
        pkt.data[count_offset] = count;

        // Should cap at 19 entries (0..18 inclusive)
        assert_eq!(count, 19);
    }

    #[test]
    fn test_bdw_exp_preview_display_only() {
        // Verify EXP preview uses bdw_user_point_exp (same formula as C++ NewRankingSystem.cpp:426)
        // Level 80, 20 bdw_points → 80^3 * 0.15 * (5 * 20) = 512000 * 0.15 * 100 = 7680000
        let (exp, prem) = bdw_user_point_exp(80, 20);
        assert_eq!(exp, 7_680_000);
        assert_eq!(prem, 10_000_000); // 15360000 capped to 10M
    }

    #[test]
    fn test_chaos_exp_preview_negative_clamped() {
        // If deaths > 5*kills, EXP should be 0
        let (exp, prem) = chaos_user_exp(50, 1, 10);
        // kill_score = 5*1 - 10 = -5 → negative → clamped to 0
        assert_eq!(exp, 0);
        assert_eq!(prem, 0);
    }

    // ── Sprint 977: Additional coverage ──────────────────────────────

    /// RANK_TYPE constants are 1, 2, 3 — sequential and distinct.
    #[test]
    fn test_rank_type_sequential() {
        assert_eq!(RANK_TYPE_PK_ZONE, 1);
        assert_eq!(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR, 2);
        assert_eq!(RANK_TYPE_CHAOS_DUNGEON, 3);
        assert_eq!(RANK_TYPE_CHAOS_DUNGEON - RANK_TYPE_PK_ZONE, 2);
    }

    /// WIZ_RANK opcode is 0x80.
    #[test]
    fn test_wiz_rank_opcode() {
        assert_eq!(Opcode::WizRank as u8, 0x80);
    }

    /// bdw_user_point_exp scales with level cubed.
    #[test]
    fn test_bdw_exp_level_scaling() {
        let (exp_low, _) = bdw_user_point_exp(50, 10);
        let (exp_high, _) = bdw_user_point_exp(80, 10);
        // Higher level → more exp
        assert!(exp_high > exp_low);
    }

    /// chaos_user_exp with equal kills and deaths still yields positive exp.
    #[test]
    fn test_chaos_exp_equal_kills_deaths() {
        let (exp, _) = chaos_user_exp(60, 5, 5);
        // kill_score = 5*5 - 5 = 20 → positive
        assert!(exp > 0);
    }

    /// bdw_user_point_exp with 0 points yields 0 exp.
    #[test]
    fn test_bdw_exp_zero_points() {
        let (exp, prem) = bdw_user_point_exp(80, 0);
        assert_eq!(exp, 0);
        assert_eq!(prem, 0);
    }
}
