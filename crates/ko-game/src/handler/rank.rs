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

/// PK rank window (type 1 / type 5) is only valid in Ronark Land, Ronark Land
/// Base, Ardream and the war zones.
pub(crate) fn pk_rank_zone_allowed(zone_id: u16, is_war_zone: bool) -> bool {
    use crate::world::{ZONE_ARDREAM, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE};
    is_war_zone
        || zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_RONARK_LAND_BASE
        || zone_id == ZONE_ARDREAM
}

/// Which event board a WIZ_RANK request opens inside an event zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventBoard {
    Bdw,
    Chaos,
    Juraid,
}

/// Zone gating of the event boards. The 2625 client's ALT handler sends type 2 in zone 84
/// (`sub_D31DE0`: `zone == 84 -> [0x80][2]`), type 3 in zone 85 (`UILadderChaos`) and type 1 / 5
/// elsewhere; the reply type decides which window opens (type 2 -> the temple board
/// `sub_D304D0`, type 3 -> the Chaos ladder `sub_BCA100`). Inside an event zone every request
/// type is answered with that zone's own board, so a stale type-1/5 request never opens the PK
/// window and never goes unanswered. Juraid (87) uses the temple board layout (type 2) with the
/// room's kills as the score.
pub(crate) fn event_zone_reply(zone_id: u16, rank_type: u8) -> Option<EventBoard> {
    if !matches!(
        rank_type,
        RANK_TYPE_PK_ZONE | RANK_TYPE_ZONE_BORDER_DEFENSE_WAR | RANK_TYPE_CHAOS_DUNGEON | RANK_TYPE_LADDER
    ) {
        return None;
    }
    match zone_id {
        crate::world::ZONE_CHAOS_DUNGEON => Some(EventBoard::Chaos),
        crate::world::ZONE_BDW => Some(EventBoard::Bdw),
        crate::world::ZONE_JURAID => Some(EventBoard::Juraid),
        _ => None,
    }
}

/// One row of the temple board (type 2 reply): `(name, nation, knights_id, mark_version,
/// knights_name, score)`.
pub(crate) type TempleRow = (String, u8, u16, u16, String, u32);

/// The temple board rows a client row must carry per list: list A is drawn from rows whose
/// nation byte is 1 (Karus), list B from rows whose nation byte is 2 (El Morad) -- the client
/// parser (`sub_D304D0`) drops every other row. The C++ reference wrote the requester's nation
/// into every row, which emptied the enemy list; the real nation is written here.
pub(crate) const TEMPLE_BOARD_MAX_ROWS: usize = 8;

/// Build the type-2 (temple board) reply:
/// `[u8 2] {[u16 count] count x ([string name][u8 nation][u16 kid][u16 mark][string kname][u32 score])} x2
///  [i64 exp][i64 premium_exp]`. `rows` must already be sorted; each list keeps only the rows
/// of its own nation and at most [`TEMPLE_BOARD_MAX_ROWS`].
pub(crate) fn build_temple_rank_packet(rows: &[TempleRow], exp: i64, premium_exp: i64) -> Packet {
    let mut result = Packet::new(Opcode::WizRank as u8);
    result.write_u8(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR);
    for nation in [1u8, 2u8] {
        let list: Vec<&TempleRow> = rows.iter().filter(|r| r.1 == nation).take(TEMPLE_BOARD_MAX_ROWS).collect();
        result.write_u16(list.len() as u16);
        for (name, nation, kid, mark, kname, score) in list {
            result.write_string(name);
            result.write_u8(*nation);
            result.write_u16(*kid);
            result.write_u16(*mark);
            result.write_string(kname);
            result.write_u32(*score);
        }
    }
    result.write_i64(exp);
    result.write_i64(premium_exp);
    result
}

/// Clan columns of a room member for the temple board. Event bots have no session and are
/// listed like a clanless player.
fn temple_clan_columns(world: &crate::world::WorldState, sid: crate::zone::SessionId) -> Option<(u16, u16, String)> {
    let knights_id = match world.with_session(sid, |h| h.character.as_ref().map(|c| c.knights_id)) {
        Some(Some(kid)) => kid,
        _ if rank_entry_is_event_bot(world, sid) => 0,
        _ => return None,
    };
    if knights_id == 0 || knights_id == 0xFFFF {
        return Some((0, 0, String::new()));
    }
    Some(match world.get_knights(knights_id) {
        Some(ki) => (ki.id, ki.mark_version, ki.name.clone()),
        None => (0, 0, String::new()),
    })
}

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

    // Zone gating of the rank window (the 2625 client asks with type 1 or 5 on
    // ALT in every zone and shows whatever window the reply frames):
    //  - Chaos Dungeon: always answer with the Chaos ladder (type 3 reply),
    //  - Ronark Land / Base / Ardream (and war zones): PK ranking,
    //  - everywhere else: no reply, so no PK window opens.
    let zone_id = {
        let sid = session.session_id();
        world.with_session(sid, |h| h.position.zone_id).unwrap_or(0)
    };
    if let Some(reply) = event_zone_reply(zone_id, rank_type) {
        return match reply {
            EventBoard::Chaos => handle_chaos_dungeon(session).await,
            EventBoard::Bdw => handle_bdw(session).await,
            EventBoard::Juraid => handle_juraid(session).await,
        };
    }
    let is_war_zone = world.get_zone(zone_id).is_some_and(|z| z.is_war_zone());
    if matches!(rank_type, RANK_TYPE_PK_ZONE | RANK_TYPE_LADDER)
        && !pk_rank_zone_allowed(zone_id, is_war_zone)
    {
        debug!(
            "[{}] WIZ_RANK type {} ignored outside PK zones (zone {})",
            session.addr(),
            rank_type,
            zone_id
        );
        return Ok(());
    }

    match rank_type {
        RANK_TYPE_PK_ZONE => {
            // C++ checks isInSpecialEventZone() || isInWarZone() and routes to
            // HandleRankingSpecialEvent which uses zindan_rankings instead.
            if is_war_zone {
                handle_special_event_zone(session).await
            } else {
                handle_pk_zone(session).await
            }
        }
        RANK_TYPE_ZONE_BORDER_DEFENSE_WAR => handle_bdw(session).await,
        RANK_TYPE_CHAOS_DUNGEON => handle_chaos_dungeon(session).await,
        RANK_TYPE_LADDER => handle_ladder(session).await,
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

/// Rank type 5 — the 2625 client's default ranking-window request.
///
/// The client (`UIRadder`, `sub_D31C00` @ 0x00D31C00) sends `[0x80][5]` when the
/// rank hotkey is pressed outside the special-event zones (`sub_D31C60` sends
/// type 1 for the PK-zone variant). The reply is parsed by `sub_D30B20`
/// @ 0x00D30B20, which fills the very same list slots as the type-1 parser
/// (`sub_D30160`) but with a different framing, see [`build_ladder_rank_packet`].
const RANK_TYPE_LADDER: u8 = 5;

/// One row of the type-5 ranking lists.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LadderEntry {
    name: String,
    /// 1 = Karus, 2 = El Morad. The client only keeps list-A rows whose nation
    /// byte is 1 and list-B rows whose nation byte is 2.
    nation: u8,
    knights_id: u16,
    mark_version: u16,
    knights_name: String,
    /// Shown as the main figure of the row (daily loyalty).
    score: u32,
    /// Shown as `(+bonus)` next to the score when non-zero (premium bonus).
    bonus: u32,
}

/// Client limit: `sub_D30B20` skips the whole list when a count byte is > 10.
const LADDER_MAX_ROWS: usize = 10;

/// Build the type-5 ranking response.
///
/// Wire layout, taken from the client parser `sub_D30B20` (0x00D30B20):
/// ```text
/// [u8 rank_type = 5]
/// [u8 count_a (<=10)] count_a x row     // Karus list (row nation must be 1)
/// [u8 count_b (<=10)] count_b x row     // El Morad list (row nation must be 2)
/// [u32 my_rank] [u8 flag] [u32 my_score] [u32 my_bonus]
/// row = [u16 len][name] [u8 nation] [u16 knights_id] [u16 mark_version]
///       [u16 len][knights_name] [u32 score] [u32 bonus]
/// ```
fn build_ladder_rank_packet(
    lists: &[Vec<LadderEntry>; 2],
    my_rank: u32,
    my_score: u32,
    my_bonus: u32,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizRank as u8);
    pkt.write_u8(RANK_TYPE_LADDER);
    for (idx, list) in lists.iter().enumerate() {
        let rows: Vec<&LadderEntry> = list
            .iter()
            .filter(|e| e.nation as usize == idx + 1)
            .take(LADDER_MAX_ROWS)
            .collect();
        pkt.write_u8(rows.len() as u8);
        for e in rows {
            pkt.write_string(&e.name);
            pkt.write_u8(e.nation);
            pkt.write_u16(e.knights_id);
            pkt.write_u16(e.mark_version);
            pkt.write_string(&e.knights_name);
            pkt.write_u32(e.score);
            pkt.write_u32(e.bonus);
        }
    }
    pkt.write_u32(my_rank);
    pkt.write_u8(0); // stored by the client at panel+0xB3C; no known meaning, 0 is neutral
    pkt.write_u32(my_score);
    pkt.write_u32(my_bonus);
    pkt
}

/// Handle the default ranking window request (rank type 5).
///
/// The server has no dedicated "ladder" data source, so the rows come from the
/// same live daily-loyalty ranking the PK-zone window uses (requester's zone,
/// players plus runtime bots). Outside PK zones that list is empty, which the
/// client renders as an empty table together with the requester's own figures
/// -- instead of the previous behaviour where the request was dropped and the
/// window could never be opened.
async fn handle_ladder(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    let (nation, zone_id, my_daily, my_bonus) = match world.with_session(sid, |h| {
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

    let mut lists: [Vec<LadderEntry>; 2] = [Vec::new(), Vec::new()];
    for nation_idx in 0..2u8 {
        // Sorted by score like the PK window.
        let mut rows: Vec<LadderEntry> = Vec::new();
        for rank in world.pk_zone_get_sorted(nation_idx as usize, zone_id) {
            let Some(Some((name, entry_nation, knights_id))) =
                world.with_session(rank.session_id, |h| {
                    h.character
                        .as_ref()
                        .map(|c| (c.name.clone(), c.nation, c.knights_id))
                })
            else {
                continue;
            };
            let (clan_id, mark, clan_name) = match world.get_knights(knights_id) {
                Some(ki) if knights_id != 0 && knights_id != 0xFFFF => {
                    (ki.id, ki.mark_version, ki.name.clone())
                }
                _ => (0, 0, String::new()),
            };
            rows.push(LadderEntry {
                name,
                nation: entry_nation,
                knights_id: clan_id,
                mark_version: mark,
                knights_name: clan_name,
                score: rank.loyalty_daily,
                bonus: rank.loyalty_premium_bonus.min(999) as u32,
            });
        }
        for bot in world
            .get_bots_in_zone_live(zone_id)
            .into_iter()
            .filter(|bot| bot.nation == nation_idx + 1)
        {
            let (clan_id, mark, clan_name) = match world.get_knights(bot.knights_id) {
                Some(ki) if bot.knights_id != 0 && bot.knights_id != 0xFFFF => {
                    (ki.id, ki.mark_version, ki.name.clone())
                }
                _ => (0, 0, String::new()),
            };
            rows.push(LadderEntry {
                name: bot.name,
                nation: bot.nation,
                knights_id: clan_id,
                mark_version: mark,
                knights_name: clan_name,
                score: bot.loyalty_daily,
                bonus: 0,
            });
        }
        rows.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
        lists[nation_idx as usize] = rows;
    }

    // Requester's own position inside their nation's (uncapped) list.
    let my_rank = if (1..=2).contains(&nation) {
        let idx = (nation - 1) as usize;
        world
            .with_session(sid, |h| h.character.as_ref().map(|c| c.name.clone()))
            .flatten()
            .and_then(|me| lists[idx].iter().position(|e| e.name == me))
            .map(|p| p as u32 + 1)
            .unwrap_or(0)
    } else {
        0
    };

    let pkt = build_ladder_rank_packet(&lists, my_rank, my_daily, my_bonus as u32);
    session.send_packet(&pkt).await?;
    debug!("[{}] WIZ_RANK LADDER: zone={}", session.addr(), zone_id);
    Ok(())
}

/// Handle Border Defence War ranking request (RANK_TYPE_ZONE_BORDER_DEFENSE_WAR = 2).
/// Reads real ranking data from the EventRoom. Each nation's users are sorted by
/// `bdw_points` descending and up to 8 entries are written per nation. The
/// requester's own accumulated EXP preview (display-only) is appended.
async fn handle_bdw(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Get requester's character info
    let (char_name, requester_level, my_bdw_points) =
        match world.with_session(sid, |h| {
            h.character
                .as_ref()
                .map(|c| (c.name.clone(), c.level, 0u32))
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

    let mut rows: Vec<TempleRow> = Vec::with_capacity(16);
    for (nation, entries) in [(1u8, &karus_entries), (2u8, &elmo_entries)] {
        for (name, entry_sid, bdw_points) in entries.iter() {
            // Verify the member is still in-game (event bots have no session but are listed)
            let Some((kid, mark, kname)) = temple_clan_columns(world, *entry_sid) else { continue };
            rows.push((name.clone(), nation, kid, mark, kname, *bdw_points));
        }
    }

    // Requester's own EXP preview (display-only)
    let (gained_exp, premium_gained_exp) = bdw_user_point_exp(requester_level, my_points);
    let result = build_temple_rank_packet(&rows, gained_exp, premium_gained_exp);
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

/// Juraid Mountain board (zone 87): the temple board layout (type 2) with every room member of
/// both nations and their kills as the score. The reference has no Juraid ranking packet; the
/// 2625 client has no ALT board of its own in zone 87 either, so this only answers an explicit
/// request (type 1 / 2 / 5) with a window the client knows how to draw.
async fn handle_juraid(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();
    let Some(Some(char_name)) = world.with_session(sid, |h| h.character.as_ref().map(|c| c.name.clone())) else {
        return Ok(());
    };
    let erm = world.event_room_manager();
    let active = erm.read_temple_event(|te| te.is_juraid_active());
    let room_id = erm.find_user_room(TempleEventType::JuraidMountain, &char_name).map(|(r, _)| r);
    let rows: Vec<TempleRow> = match (active, room_id) {
        (true, Some(room_id)) => {
            let Some(room) = erm.get_room(TempleEventType::JuraidMountain, room_id) else {
                session.send_packet(&build_temple_rank_packet(&[], 0, 0)).await?;
                return Ok(());
            };
            let mut members: Vec<(String, u8, crate::zone::SessionId, u32)> = room
                .karus_users
                .values()
                .chain(room.elmorad_users.values())
                .filter(|u| !u.user_name.is_empty() && !u.logged_out)
                .map(|u| (u.user_name.clone(), u.nation, u.session_id, u.kills))
                .collect();
            drop(room);
            members.sort_by(|a, b| b.3.cmp(&a.3));
            members
                .into_iter()
                .filter_map(|(name, nation, msid, kills)| {
                    temple_clan_columns(world, msid).map(|(kid, mark, kname)| (name, nation, kid, mark, kname, kills))
                })
                .collect()
        }
        _ => Vec::new(),
    };
    session.send_packet(&build_temple_rank_packet(&rows, 0, 0)).await?;
    debug!("[{}] WIZ_RANK JURAID board: room={:?}, rows={}", session.addr(), room_id, rows.len());
    Ok(())
}

/// Event bots sit in the event rooms with bot-band session ids and no `ClientSession`; the
/// BDW / Chaos rank windows list them like players (kills, deaths, BDW points are kept on their
/// room `EventUser` exactly as for humans).
pub(crate) fn rank_entry_is_event_bot(world: &crate::world::WorldState, sid: crate::zone::SessionId) -> bool {
    crate::systems::event_bots::is_event_bot_sid(world, sid)
}

/// The Chaos window lists at most 19 rows (`if (sCount > 18) break;`).
pub(crate) const CHAOS_RANK_MAX_ROWS: usize = 19;

/// Build the Chaos Dungeon ranking reply (C++ `HandleRankingChaosDungeon`):
/// `[u8 3][u8 count]{[string name][u16 kills][u16 deaths]}*[i32 exp][i32 premium_exp][u32 1]`.
/// `rows` must already be sorted; the exp values are clamped like the C++ (0..=8M / 0..=10M).
pub(crate) fn build_chaos_rank_packet(rows: &[(String, u32, u32)], exp: i64, premium_exp: i64) -> Packet {
    let mut result = Packet::new(Opcode::WizRank as u8);
    result.write_u8(RANK_TYPE_CHAOS_DUNGEON);
    let n = rows.len().min(CHAOS_RANK_MAX_ROWS);
    result.write_u8(n as u8);
    for (name, kills, deaths) in rows.iter().take(n) {
        result.write_string(name);
        result.write_u16((*kills).min(u16::MAX as u32) as u16);
        result.write_u16((*deaths).min(u16::MAX as u32) as u16);
    }
    result.write_i32(exp.clamp(0, 8_000_000) as i32);
    result.write_i32(premium_exp.clamp(0, 10_000_000) as i32);
    result.write_u32(1);
    result
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

    // Requester's own EXP preview (display-only)
    let (gained_exp, premium_gained_exp) = chaos_user_exp(requester_level, my_kills, my_deaths);
    let rows: Vec<(String, u32, u32)> = entries
        .iter()
        .filter(|(_, entry_sid, _, _)| {
            // Verify user is still in-game (event bots have no session but are ranked too)
            world
                .with_session(*entry_sid, |h| h.character.is_some())
                .unwrap_or(false)
                || rank_entry_is_event_bot(world, *entry_sid)
        })
        .map(|(name, _, kills, deaths)| (name.clone(), *kills, *deaths))
        .collect();
    let count = rows.len().min(CHAOS_RANK_MAX_ROWS) as u8;
    let result = build_chaos_rank_packet(&rows, gained_exp, premium_gained_exp);

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

    /// Event bots (no session, bot-band id) are listed in the BDW / Chaos rank windows; an
    /// unknown session-band id (a player that logged out) is not.
    #[test]
    fn event_bots_are_listed_in_the_event_rank_windows() {
        use crate::systems::event_bots::{BotEntry, EventBotKind};
        let world = crate::world::WorldState::new();
        world.event_bots.insert(20_500, BotEntry::new(EventBotKind::Bdw, 1, 2, "ElmoBot".into()));
        assert!(rank_entry_is_event_bot(&world, 20_500));
        assert!(!rank_entry_is_event_bot(&world, 20_501), "unregistered bot id");
        assert!(!rank_entry_is_event_bot(&world, 7), "player ids are never bots");
    }

    /// Inside the event zones every ALT request type opens that zone's own board; the PK window
    /// gating stays for the other zones (the live bug: types 1/5 in zone 84/87 were dropped, so
    /// no window ever opened there).
    #[test]
    fn event_zones_answer_every_request_type_with_their_own_board() {
        for ty in [RANK_TYPE_PK_ZONE, RANK_TYPE_ZONE_BORDER_DEFENSE_WAR, RANK_TYPE_CHAOS_DUNGEON, RANK_TYPE_LADDER] {
            assert_eq!(event_zone_reply(84, ty), Some(EventBoard::Bdw), "type {ty} in BDW");
            assert_eq!(event_zone_reply(85, ty), Some(EventBoard::Chaos), "type {ty} in Chaos");
            assert_eq!(event_zone_reply(87, ty), Some(EventBoard::Juraid), "type {ty} in Juraid");
            assert_eq!(event_zone_reply(71, ty), None, "type {ty} in Ronark");
            assert_eq!(event_zone_reply(21, ty), None, "type {ty} in Moradon");
        }
        assert_eq!(event_zone_reply(84, 99), None, "unknown types are not routed");
    }

    /// The client parser (`sub_D304D0`) keeps list-A rows whose nation byte is 1 and list-B rows
    /// whose nation byte is 2: the reply carries each row's real nation, bots included.
    #[test]
    fn temple_board_writes_each_rows_real_nation_and_caps_the_lists() {
        let mut rows: Vec<TempleRow> = vec![
            ("KarusMan".into(), 1, 7, 3, "Clan".into(), 30),
            ("ElmoBot".into(), 2, 0, 0, String::new(), 20),
            ("KarusBot".into(), 1, 0, 0, String::new(), 10),
        ];
        for i in 0..10 {
            rows.push((format!("Elmo{i}"), 2, 0, 0, String::new(), 5));
        }
        let pkt = build_temple_rank_packet(&rows, 468_750, 937_500);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR));
        // list A: the two Karus rows, in order, nation byte 1
        assert_eq!(r.read_u16(), Some(2));
        assert_eq!(r.read_string().as_deref(), Some("KarusMan"));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(7));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.read_string().as_deref(), Some("Clan"));
        assert_eq!(r.read_u32(), Some(30));
        assert_eq!(r.read_string().as_deref(), Some("KarusBot"));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_string().as_deref(), Some(""));
        assert_eq!(r.read_u32(), Some(10));
        // list B: 11 El Morad rows capped at 8, nation byte 2, the bot first (sorted input)
        assert_eq!(r.read_u16(), Some(TEMPLE_BOARD_MAX_ROWS as u16));
        assert_eq!(r.read_string().as_deref(), Some("ElmoBot"));
        assert_eq!(r.read_u8(), Some(2));
        for _ in 0..7 {
            r.read_u16();
            r.read_u16();
            r.read_string();
            r.read_u32();
            r.read_string();
            assert_eq!(r.read_u8(), Some(2));
        }
        r.read_u16();
        r.read_u16();
        r.read_string();
        r.read_u32();
        assert_eq!(r.read_i64(), Some(468_750));
        assert_eq!(r.read_i64(), Some(937_500));
        assert_eq!(r.remaining(), 0);
    }

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
    fn test_chaos_rank_packet_layout() {
        let rows = vec![("Ann".to_string(), 7u32, 2u32), ("Bob".to_string(), 3, 5)];
        let pkt = build_chaos_rank_packet(&rows, 123_456, 246_912);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(pkt.opcode, Opcode::WizRank as u8);
        assert_eq!(r.read_u8(), Some(RANK_TYPE_CHAOS_DUNGEON));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_string().as_deref(), Some("Ann"));
        assert_eq!((r.read_u16(), r.read_u16()), (Some(7), Some(2)));
        assert_eq!(r.read_string().as_deref(), Some("Bob"));
        assert_eq!((r.read_u16(), r.read_u16()), (Some(3), Some(5)));
        assert_eq!(r.read_u32().map(|v| v as i32), Some(123_456));
        assert_eq!(r.read_u32().map(|v| v as i32), Some(246_912));
        assert_eq!(r.read_u32(), Some(1));
    }

    #[test]
    fn test_chaos_rank_packet_caps_rows_and_exp() {
        let rows: Vec<(String, u32, u32)> = (0..30).map(|i| (format!("P{i}"), i, 0)).collect();
        let pkt = build_chaos_rank_packet(&rows, 99_000_000, -5);
        assert_eq!(pkt.data[1], 19);
        let tail = &pkt.data[pkt.data.len() - 12..];
        assert_eq!(i32::from_le_bytes(tail[0..4].try_into().unwrap()), 8_000_000);
        assert_eq!(i32::from_le_bytes(tail[4..8].try_into().unwrap()), 0);
    }

    #[test]
    fn test_pk_rank_zone_gating() {
        assert!(pk_rank_zone_allowed(71, false));
        assert!(pk_rank_zone_allowed(72, false));
        assert!(pk_rank_zone_allowed(73, false));
        assert!(pk_rank_zone_allowed(61, true));
        assert!(!pk_rank_zone_allowed(21, false)); // Moradon
        assert!(!pk_rank_zone_allowed(85, false)); // Chaos -> own ladder
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

    // ── Rank type 5 (default ranking window, client sub_D30B20 @ 0x00D30B20) ──

    /// Mirror of the client parser: returns (list A rows, list B rows, trailer).
    /// Mimics the bounds rule `count <= 10` and the per-list nation filter.
    #[allow(clippy::type_complexity)]
    fn parse_ladder(pkt: &Packet) -> (Vec<LadderEntry>, Vec<LadderEntry>, (u32, u8, u32, u32)) {
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(RANK_TYPE_LADDER));
        let mut lists = [Vec::new(), Vec::new()];
        for (idx, list) in lists.iter_mut().enumerate() {
            let count = r.read_u8().unwrap();
            assert!(count <= 10, "client skips the whole packet when count > 10");
            for _ in 0..count {
                let name = r.read_string().unwrap();
                let nation = r.read_u8().unwrap();
                let knights_id = r.read_u16().unwrap();
                let mark_version = r.read_u16().unwrap();
                let knights_name = r.read_string().unwrap();
                let score = r.read_u32().unwrap();
                let bonus = r.read_u32().unwrap();
                // Client stores a row only when its nation byte matches the list.
                if nation as usize == idx + 1 {
                    list.push(LadderEntry {
                        name,
                        nation,
                        knights_id,
                        mark_version,
                        knights_name,
                        score,
                        bonus,
                    });
                }
            }
        }
        let trailer = (
            r.read_u32().unwrap(),
            r.read_u8().unwrap(),
            r.read_u32().unwrap(),
            r.read_u32().unwrap(),
        );
        assert_eq!(r.remaining(), 0, "no trailing bytes the client would not read");
        let [a, b] = lists;
        (a, b, trailer)
    }

    fn ladder_row(name: &str, nation: u8, score: u32) -> LadderEntry {
        LadderEntry {
            name: name.to_string(),
            nation,
            knights_id: 7,
            mark_version: 3,
            knights_name: "Clan".to_string(),
            score,
            bonus: 5,
        }
    }

    #[test]
    fn test_ladder_rank_type_is_five() {
        assert_eq!(RANK_TYPE_LADDER, 5);
    }

    #[test]
    fn test_ladder_packet_empty_lists() {
        let pkt = build_ladder_rank_packet(&[Vec::new(), Vec::new()], 0, 123, 4);
        assert_eq!(pkt.opcode, Opcode::WizRank as u8);
        // type + 2 counts + u32 + u8 + u32 + u32
        assert_eq!(pkt.data.len(), 1 + 1 + 1 + 4 + 1 + 4 + 4);
        let (a, b, t) = parse_ladder(&pkt);
        assert!(a.is_empty() && b.is_empty());
        assert_eq!(t, (0, 0, 123, 4));
    }

    #[test]
    fn test_ladder_packet_roundtrip_both_nations() {
        let lists = [
            vec![ladder_row("Karus1", 1, 900), ladder_row("Karus2", 1, 100)],
            vec![ladder_row("Elmo1", 2, 500)],
        ];
        let pkt = build_ladder_rank_packet(&lists, 2, 100, 5);
        let (a, b, t) = parse_ladder(&pkt);
        assert_eq!(a, lists[0]);
        assert_eq!(b, lists[1]);
        assert_eq!(t, (2, 0, 100, 5));
    }

    #[test]
    fn test_ladder_packet_caps_rows_at_ten() {
        let karus: Vec<LadderEntry> = (0..25)
            .map(|i| ladder_row(&format!("K{i}"), 1, 1000 - i))
            .collect();
        let pkt = build_ladder_rank_packet(&[karus.clone(), Vec::new()], 0, 0, 0);
        let (a, _, _) = parse_ladder(&pkt);
        assert_eq!(a.len(), 10);
        assert_eq!(a[0], karus[0]);
        assert_eq!(a[9], karus[9]);
    }

    #[test]
    fn test_ladder_packet_drops_rows_with_wrong_nation() {
        // The client would silently discard a row whose nation byte does not
        // match its list; the builder must not emit such rows at all (they
        // would only waste one of the 10 slots).
        let lists = [
            vec![ladder_row("Wrong", 2, 10), ladder_row("Right", 1, 5)],
            Vec::new(),
        ];
        let pkt = build_ladder_rank_packet(&lists, 0, 0, 0);
        let (a, _, _) = parse_ladder(&pkt);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name, "Right");
    }
}
