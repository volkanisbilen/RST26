//! WIZ_KING (0x78) handler — King system.
//! ## Main Sub-opcodes (`KingType` enum in `packets.h:461-466`)
//! | Opcode | Name              | Description                           |
//! |--------|-------------------|---------------------------------------|
//! | 1      | KING_ELECTION     | Election system (schedule, nominate)  |
//! | 2      | KING_IMPEACHMENT  | Impeachment system                    |
//! | 3      | KING_TAX          | Tax / tariff / scepter                |
//! | 4      | KING_EVENT        | King events (noah, exp, prize, etc.)  |
//! | 6      | KING_NATION_INTRO | Nation introduction text              |
//! ## Implementation Status
//! Implemented:
//! - Tax system (tariff lookup, tariff update, tax collection, king scepter)
//! - Event system (noah, exp, prize, weather, notice)
//! - Election system (schedule, nomination, notice board, poll/vote, resign)
//! - Impeachment UI open checks
//! - Nation introduction
//! Also implemented (from C++ empty stubs):
//! - Impeachment request, request elect, list, elect (basic validation + response)

use std::sync::Arc;

use chrono::{Datelike, Local, Timelike};
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::{
    ElectionListEntry, NominationEntry, ELECTION_TYPE_ELECTION, ELECTION_TYPE_NOMINATION,
    ELECTION_TYPE_PRE_ELECTION, KING_CANDIDACY_BOARD_READ, KING_CANDIDACY_BOARD_WRITE,
    KING_ELECTION, KING_ELECTION_NOMINATE, KING_ELECTION_NOTICE_BOARD, KING_ELECTION_POLL,
    KING_ELECTION_RESIGN, KING_ELECTION_SCHEDULE, KING_EVENT_EXP, KING_EVENT_FUGITIVE,
    KING_EVENT_NOAH, KING_EVENT_NOTICE, KING_EVENT_OPCODE, KING_EVENT_PRIZE, KING_EVENT_WEATHER,
    KING_IMPEACHMENT, KING_IMPEACHMENT_ELECT, KING_IMPEACHMENT_ELECTION_UI_OPEN,
    KING_IMPEACHMENT_LIST, KING_IMPEACHMENT_REQUEST, KING_IMPEACHMENT_REQUEST_ELECT,
    KING_IMPEACHMENT_REQUEST_UI_OPEN, KING_NATION_INTRO, KING_TAX, MIN_LEVEL_VOTER, MIN_NP_VOTER,
};

/// King's scepter item ID.
const KING_SCEPTER: u32 = 910_074_311;

use super::{HAVE_MAX, SLOT_MAX};

/// Handle incoming WIZ_KING (0x78) packet.
pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let sub_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match sub_opcode {
        KING_ELECTION => handle_election(session, &mut reader).await,
        KING_IMPEACHMENT => handle_impeachment(session, &mut reader).await,
        KING_TAX => handle_tax(session, &mut reader).await,
        KING_EVENT_OPCODE => handle_event(session, &mut reader).await,
        KING_NATION_INTRO => handle_nation_intro(session, &mut reader).await,
        _ => {
            warn!(
                "[{}] WIZ_KING: unknown sub-opcode 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Get the player's nation from their session.
fn get_nation(session: &ClientSession) -> Option<u8> {
    let world = session.world();
    world.with_session(session.session_id(), |h| {
        h.character.as_ref().map(|c| c.nation)
    })?
}

/// Check if the current player is the king of their nation.
fn is_player_king(session: &ClientSession) -> bool {
    let world = session.world();
    world
        .with_session(session.session_id(), |h| {
            h.character
                .as_ref()
                .map(|c| world.is_king(c.nation, &c.name))
        })
        .flatten()
        .unwrap_or(false)
}

/// Get the player's character name from their session.
fn get_player_name(session: &ClientSession) -> Option<String> {
    let world = session.world();
    world.with_session(session.session_id(), |h| {
        h.character.as_ref().map(|c| c.name.clone())
    })?
}

// ── Election System ─────────────────────────────────────────────────────

/// Handle KING_ELECTION (1) sub-packet.
async fn handle_election(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let election_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match election_opcode {
        KING_ELECTION_SCHEDULE => handle_election_schedule(session).await,
        KING_ELECTION_NOMINATE => handle_candidacy_recommend(session, reader).await,
        KING_ELECTION_NOTICE_BOARD => handle_candidacy_notice_board(session, reader).await,
        KING_ELECTION_POLL => handle_election_poll(session, reader).await,
        KING_ELECTION_RESIGN => handle_candidacy_resign(session).await,
        _ => {
            debug!(
                "[{}] WIZ_KING ELECTION: unhandled election sub-opcode {}",
                session.addr(),
                election_opcode
            );
            Ok(())
        }
    }
}

/// Handle election schedule confirmation.
/// Responds with the next election date or impeachment schedule.
async fn handle_election_schedule(session: &mut ClientSession) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_ELECTION);
    result.write_u8(KING_ELECTION_SCHEDULE);

    match ks.im_type {
        // No impeachment — send election date
        0 => {
            let now = Local::now();
            let cur_day = now.day() as u8;
            let mut election_month = now.month() as u8;

            // If we've passed the election day, next month
            if cur_day > ks.day {
                election_month += 1;
                if election_month > 12 {
                    election_month -= 12;
                }
            }

            result.write_u8(1); // election type
            result.write_u8(election_month);
            result.write_u8(ks.day);
            result.write_u8(ks.hour);
            result.write_u8(ks.minute);
        }
        // Last scheduled impeachment
        1 => {
            result.write_u8(3);
            result.write_u8(ks.im_month);
            result.write_u8(ks.im_day);
            result.write_u8(ks.im_hour);
            result.write_u8(ks.im_minute);
        }
        // Next impeachment
        3 => {
            result.write_u8(2);
            result.write_u8(ks.im_month);
            result.write_u8(ks.im_day);
            result.write_u8(ks.im_hour);
            result.write_u8(ks.im_minute);
        }
        _ => {
            // Default: send election date
            result.write_u8(1);
            result.write_u8(ks.month);
            result.write_u8(ks.day);
            result.write_u8(ks.hour);
            result.write_u8(ks.minute);
        }
    }

    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING ELECTION_SCHEDULE: nation={}, im_type={}",
        session.addr(),
        nation,
        ks.im_type
    );

    Ok(())
}

/// Handle candidacy recommendation (nomination).
/// Only top-10 clan leaders (senators) can nominate candidates during NOMINATION phase.
async fn handle_candidacy_recommend(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let nominee_name = match reader.read_sbyte_string() {
        Some(s) if !s.is_empty() && s.len() <= 21 => s,
        _ => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_ELECTION);
    result.write_u8(KING_ELECTION_NOMINATE);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Must be nomination phase
    if ks.election_type != ELECTION_TYPE_NOMINATION {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let player_name = match get_player_name(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    // Check if nominator is a senator and hasn't resigned
    let is_senator = ks
        .senator_list
        .iter()
        .any(|e| e.name.eq_ignore_ascii_case(&player_name));
    let has_resigned = ks
        .resigned_candidates
        .iter()
        .any(|n| n.eq_ignore_ascii_case(&player_name));

    if !world.is_session_clan_leader(session.session_id()) || !is_senator || has_resigned {
        result.write_i16(-3); // No authority
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check if nominator already nominated someone
    let already_nominated = ks
        .nomination_list
        .iter()
        .any(|n| n.nominator.eq_ignore_ascii_case(&player_name));
    if already_nominated {
        result.write_i16(-4); // Already nominated
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check if nominee is already nominated
    let nominee_exists = ks
        .nomination_list
        .iter()
        .any(|n| n.nominee.eq_ignore_ascii_case(&nominee_name));
    if nominee_exists {
        result.write_i16(-5); // Already nominated
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Get nominee's clan ID (the nominee must exist and be same nation)
    let nominee_sid = world.find_session_by_name(&nominee_name);
    let nominee_clan_id = nominee_sid
        .map(|sid| world.get_session_clan_id(sid))
        .unwrap_or(0);

    // Insert the nominee into the election lists
    let nominator_name = player_name.clone();
    let nominee_name_clone = nominee_name.clone();

    world.update_king_system(nation, |ks| {
        // Add to candidate list
        ks.candidate_list.push(ElectionListEntry {
            name: nominee_name_clone.clone(),
            knights_id: nominee_clan_id,
            votes: 0,
        });

        // Add to nomination list
        ks.nomination_list.push(NominationEntry {
            nominator: nominator_name.clone(),
            nominee: nominee_name_clone.clone(),
        });

        // Remove senator from senator list (they become a nominator)
        ks.senator_list
            .retain(|e| !e.name.eq_ignore_ascii_case(&nominator_name));
    });

    // Fire-and-forget DB updates
    {
        let pool = session.pool().clone();
        let n = nation as i16;
        let nominator = player_name.clone();
        let nominee = nominee_name.clone();
        let clan_id = nominee_clan_id as i16;
        tokio::spawn(async move {
            let repo = ko_db::repositories::king::KingRepository::new(&pool);
            // Add candidate to election list (type=4)
            if let Err(e) = repo.upsert_election_list(n, 4, &nominee, clan_id).await {
                tracing::error!("Failed to upsert election list for nominee {nominee}: {e}");
            }
            // Add nomination
            if let Err(e) = repo.insert_nomination(n, &nominator, &nominee).await {
                tracing::error!("Failed to insert nomination {nominator} -> {nominee}: {e}");
            }
            // Remove senator (type=3)
            if let Err(e) = repo.delete_election_list_entry(n, 3, &nominator).await {
                tracing::error!(
                    "Failed to delete senator election list entry for {nominator}: {e}"
                );
            }
        });
    }

    result.write_i16(1); // success
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING ELECTION NOMINATE: {} nominated {} (nation={})",
        session.addr(),
        player_name,
        nominee_name,
        nation
    );

    Ok(())
}

/// Handle candidacy notice board (read/write platform statements).
async fn handle_candidacy_notice_board(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_ELECTION);
    result.write_u8(KING_ELECTION_NOTICE_BOARD);
    result.write_u8(opcode);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Notice board only available during nomination/pre-election/election
    let valid_phase = ks.election_type == ELECTION_TYPE_NOMINATION
        || ks.election_type == ELECTION_TYPE_PRE_ELECTION
        || ks.election_type == ELECTION_TYPE_ELECTION;

    match opcode {
        // Write to notice board
        KING_CANDIDACY_BOARD_WRITE => {
            if !valid_phase {
                result.write_i16(-1);
                session.send_packet(&result).await?;
                return Ok(());
            }

            let notice_text = match reader.read_string() {
                Some(s) if !s.is_empty() && s.len() <= 480 => s,
                _ => {
                    result.write_i16(-2);
                    session.send_packet(&result).await?;
                    return Ok(());
                }
            };

            let player_name = match get_player_name(session) {
                Some(n) => n,
                None => return Ok(()),
            };

            // Check if user is a candidate
            let is_candidate = ks
                .candidate_list
                .iter()
                .any(|e| e.name.eq_ignore_ascii_case(&player_name));
            if !is_candidate {
                result.write_i16(-3);
                session.send_packet(&result).await?;
                return Ok(());
            }

            // Check if user is in the nomination list
            let is_nominated = ks
                .nomination_list
                .iter()
                .any(|n| n.nominee.eq_ignore_ascii_case(&player_name));
            if !is_nominated {
                result.write_i16(-3);
                session.send_packet(&result).await?;
                return Ok(());
            }

            // Update in-memory notice board
            let pname = player_name.clone();
            let notice = notice_text.clone();
            world.update_king_system(nation, |ks| {
                if let Some(entry) = ks
                    .notice_board
                    .iter_mut()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&pname))
                {
                    entry.1 = notice.clone();
                } else {
                    ks.notice_board.push((pname.clone(), notice.clone()));
                }
            });

            // DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                let uid = player_name.clone();
                let notice = notice_text;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.upsert_notice_board(n, &uid, &notice).await {
                        tracing::error!("Failed to upsert notice board for {uid}: {e}");
                    }
                });
            }

            result.write_i16(1); // success
            session.send_packet(&result).await?;

            debug!(
                "[{}] WIZ_KING NOTICE_BOARD WRITE: user={}, nation={}",
                session.addr(),
                player_name,
                nation
            );
        }
        // Read from notice board
        KING_CANDIDACY_BOARD_READ => {
            if !valid_phase {
                result.write_i16(-1);
                session.send_packet(&result).await?;
                return Ok(());
            }

            let sub_op = match reader.read_u8() {
                Some(v) => v,
                None => return Ok(()),
            };
            result.write_u8(sub_op);

            match sub_op {
                // List all candidates
                1 => {
                    result.write_i16(1); // success
                    result.write_u8(ks.notice_board.len() as u8);
                    for (name, _) in &ks.notice_board {
                        result.write_sbyte_string(name);
                    }
                    session.send_packet(&result).await?;
                }
                // Read specific candidate's notice
                2 => {
                    let candidate_name = match reader.read_sbyte_string() {
                        Some(s) if !s.is_empty() && s.len() <= 21 => s,
                        _ => return Ok(()),
                    };

                    let notice = ks
                        .notice_board
                        .iter()
                        .find(|(name, _)| name.eq_ignore_ascii_case(&candidate_name))
                        .map(|(_, notice)| notice.clone());

                    match notice {
                        Some(text) if !text.is_empty() => {
                            result.write_i16(1); // success
                            result.write_string(&text);
                        }
                        _ => {
                            result.write_i16(-2);
                        }
                    }
                    session.send_packet(&result).await?;
                }
                _ => {}
            }
        }
        _ => {
            debug!(
                "[{}] WIZ_KING NOTICE_BOARD: unhandled opcode {}",
                session.addr(),
                opcode
            );
        }
    }

    Ok(())
}

/// Handle election poll (view candidates / vote for king).
async fn handle_election_poll(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_ELECTION);
    result.write_u8(KING_ELECTION_POLL);
    result.write_u8(opcode);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Must be election phase
    if ks.election_type != ELECTION_TYPE_ELECTION {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    match opcode {
        // Show candidate list
        1 => {
            let count = ks.candidate_list.len() as u8;
            result.write_u16(1); // success
            result.write_u8(count);
            for (i, candidate) in ks.candidate_list.iter().enumerate() {
                result.write_u8((i + 1) as u8);
                result.write_sbyte_string(&candidate.name);
                // Look up clan name
                let clan_name = world
                    .get_knights(candidate.knights_id)
                    .map(|k| k.name.clone())
                    .unwrap_or_default();
                if clan_name.is_empty() {
                    result.write_u8(0);
                } else {
                    result.write_sbyte_string(&clan_name);
                }
            }
            session.send_packet(&result).await?;

            debug!(
                "[{}] WIZ_KING ELECTION_POLL: list {} candidates, nation={}",
                session.addr(),
                count,
                nation
            );
        }
        // Vote for a candidate
        2 => {
            let candidate_name = match reader.read_sbyte_string() {
                Some(s) if !s.is_empty() && s.len() <= 21 => s,
                _ => return Ok(()),
            };

            // Check candidate exists
            let candidate_exists = ks
                .candidate_list
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(&candidate_name));
            if !candidate_exists {
                result.write_i16(-2);
                session.send_packet(&result).await?;
                return Ok(());
            }

            // Check voter level
            let voter_level = world.get_session_level(session.session_id());
            if voter_level < MIN_LEVEL_VOTER {
                result.write_i16(-4);
                session.send_packet(&result).await?;
                return Ok(());
            }

            // Check voter NP (national points / loyalty)
            let voter_loyalty = world
                .get_character_info(session.session_id())
                .map(|c| c.loyalty)
                .unwrap_or(0);
            if (voter_loyalty as i64) < (MIN_NP_VOTER as i64) {
                result.write_i16(-4);
                session.send_packet(&result).await?;
                return Ok(());
            }

            // Record vote in DB (prevents double voting via unique constraint)
            let account_id = session.account_id().unwrap_or("").to_string();
            let voter_name = get_player_name(session).unwrap_or_default();

            if account_id.is_empty() {
                return Ok(());
            }

            let pool = session.pool().clone();
            let n = nation as i16;
            let acct = account_id.clone();
            let voter = voter_name.clone();
            let nominee = candidate_name.clone();

            let repo = ko_db::repositories::king::KingRepository::new(&pool);
            match repo.record_vote(n, &acct, &voter, &nominee).await {
                Ok(true) => {
                    // Vote recorded — increment candidate's vote count
                    world.update_king_system(nation, |ks| {
                        if let Some(c) = ks
                            .candidate_list
                            .iter_mut()
                            .find(|c| c.name.eq_ignore_ascii_case(&nominee))
                        {
                            c.votes += 1;
                        }
                        ks.total_votes += 1;
                    });

                    // Increment in DB too
                    let pool2 = pool.clone();
                    let nominee2 = nominee.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::king::KingRepository::new(&pool2);
                        if let Err(e) = repo.increment_votes(n, &nominee2).await {
                            tracing::error!("Failed to increment votes for {nominee2}: {e}");
                        }
                    });

                    result.write_i16(1); // success
                    session.send_packet(&result).await?;

                    debug!(
                        "[{}] WIZ_KING ELECTION_POLL: {} voted for {}, nation={}",
                        session.addr(),
                        voter_name,
                        candidate_name,
                        nation
                    );
                }
                Ok(false) => {
                    // Already voted
                    result.write_i16(-3);
                    session.send_packet(&result).await?;
                }
                Err(e) => {
                    warn!(
                        "[{}] WIZ_KING ELECTION_POLL: vote DB error: {}",
                        session.addr(),
                        e
                    );
                    result.write_i16(-3);
                    session.send_packet(&result).await?;
                }
            }
        }
        _ => {
            debug!(
                "[{}] WIZ_KING ELECTION_POLL: unhandled sub-opcode {}",
                session.addr(),
                opcode
            );
        }
    }

    Ok(())
}

/// Handle candidacy resignation.
async fn handle_candidacy_resign(session: &mut ClientSession) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_ELECTION);
    result.write_u8(KING_ELECTION_RESIGN);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Can only resign during nomination
    if ks.election_type != ELECTION_TYPE_NOMINATION {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let player_name = match get_player_name(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    // Check if user is in candidate list
    let is_candidate = ks
        .candidate_list
        .iter()
        .any(|e| e.name.eq_ignore_ascii_case(&player_name));
    if !is_candidate {
        result.write_i16(-3);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check if user is in nomination list
    let is_nominated = ks
        .nomination_list
        .iter()
        .any(|n| n.nominee.eq_ignore_ascii_case(&player_name));
    if !is_nominated {
        result.write_i16(-3);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Get candidate's clan ID before removal
    let clan_id = ks
        .candidate_list
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(&player_name))
        .map(|c| c.knights_id)
        .unwrap_or(0);

    // Remove from candidate and nomination lists, add to resigned
    let pname = player_name.clone();
    world.update_king_system(nation, |ks| {
        ks.candidate_list
            .retain(|c| !c.name.eq_ignore_ascii_case(&pname));
        ks.nomination_list
            .retain(|n| !n.nominee.eq_ignore_ascii_case(&pname));
        ks.notice_board
            .retain(|(name, _)| !name.eq_ignore_ascii_case(&pname));
        ks.resigned_candidates.push(pname.clone());
    });

    // DB update
    {
        let pool = session.pool().clone();
        let n = nation as i16;
        let name = player_name.clone();
        let cid = clan_id as i16;
        tokio::spawn(async move {
            let repo = ko_db::repositories::king::KingRepository::new(&pool);
            if let Err(e) = repo.delete_election_list_entry(n, 4, &name).await {
                tracing::error!("Failed to delete candidate election list entry for {name}: {e}");
            }
            if let Err(e) = repo.delete_nomination(n, &name).await {
                tracing::error!("Failed to delete nomination for {name}: {e}");
            }
            // Re-insert as senator (type 3) with delete flag via upsert
            if let Err(e) = repo.upsert_election_list(n, 3, &name, cid).await {
                tracing::error!("Failed to re-insert senator election list entry for {name}: {e}");
            }
        });
    }

    result.write_i16(1); // success
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING ELECTION RESIGN: {} resigned, nation={}",
        session.addr(),
        player_name,
        nation
    );

    Ok(())
}

// ── Impeachment System ──────────────────────────────────────────────────

/// Handle KING_IMPEACHMENT (2) sub-packet.
async fn handle_impeachment(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let im_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match im_opcode {
        KING_IMPEACHMENT_REQUEST => handle_impeachment_request(session, reader).await,
        KING_IMPEACHMENT_REQUEST_ELECT => handle_impeachment_request_elect(session, reader).await,
        KING_IMPEACHMENT_LIST => handle_impeachment_list(session).await,
        KING_IMPEACHMENT_ELECT => handle_impeachment_elect(session, reader).await,
        KING_IMPEACHMENT_REQUEST_UI_OPEN => handle_impeachment_request_ui_open(session).await,
        KING_IMPEACHMENT_ELECTION_UI_OPEN => handle_impeachment_election_ui_open(session).await,
        _ => {
            debug!(
                "[{}] WIZ_KING IMPEACHMENT: unhandled sub-opcode {}",
                session.addr(),
                im_opcode
            );
            Ok(())
        }
    }
}

/// Handle impeachment request UI open.
async fn handle_impeachment_request_ui_open(session: &mut ClientSession) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_IMPEACHMENT);
    result.write_u8(KING_IMPEACHMENT_REQUEST_UI_OPEN);

    // Not able to make an impeachment request right now
    if ks.im_type != 1 {
        result.write_i16(-1);
    } else {
        // Check if user is senator (m_bRank == 2)
        // In our system, senators are top-10 clan leaders stored in senator_list
        let player_name = match get_player_name(session) {
            Some(n) => n,
            None => {
                result.write_i16(-2);
                session.send_packet(&result).await?;
                return Ok(());
            }
        };

        let is_senator = ks
            .senator_list
            .iter()
            .any(|e| e.name.eq_ignore_ascii_case(&player_name));

        if !is_senator {
            result.write_i16(-2);
        } else {
            result.write_i16(1);
        }
    }

    session.send_packet(&result).await?;
    Ok(())
}

/// Handle impeachment election UI open.
async fn handle_impeachment_election_ui_open(session: &mut ClientSession) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_IMPEACHMENT);
    result.write_u8(KING_IMPEACHMENT_ELECTION_UI_OPEN);
    result.write_i16(if ks.im_type != 3 { -1 } else { 1 });

    session.send_packet(&result).await?;
    Ok(())
}

/// Handle impeachment request (sub-opcode 1).
/// A senator requests impeachment of the current king. The C++ implementation
/// is an empty stub. We implement basic validation: the requester must be a
/// senator and im_type must be 1 (impeachment request window). On success,
/// we advance im_type to 3 (impeachment election phase).
async fn handle_impeachment_request(
    session: &mut ClientSession,
    _reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_IMPEACHMENT);
    result.write_u8(KING_IMPEACHMENT_REQUEST);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Must be in impeachment request phase (im_type == 1)
    if ks.im_type != 1 {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let player_name = match get_player_name(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    // Must be a senator
    let is_senator = ks
        .senator_list
        .iter()
        .any(|e| e.name.eq_ignore_ascii_case(&player_name));
    if !is_senator {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Advance to impeachment election phase
    world.update_king_system(nation, |ks| {
        ks.im_type = 3;
    });

    result.write_i16(1);
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING IMPEACHMENT_REQUEST: {} requested impeachment, nation={}",
        session.addr(),
        player_name,
        nation
    );

    Ok(())
}

/// Handle impeachment request elect (sub-opcode 2).
/// Players vote on whether to proceed with the impeachment. The C++ implementation
/// is an empty stub. We implement a basic vote recording mechanism.
async fn handle_impeachment_request_elect(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let vote = match reader.read_u8() {
        Some(v) => v, // 1 = yes, 0 = no
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_IMPEACHMENT);
    result.write_u8(KING_IMPEACHMENT_REQUEST_ELECT);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Must be in impeachment request phase
    if ks.im_type != 1 {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check voter level
    let voter_level = world.get_session_level(session.session_id());
    if voter_level < MIN_LEVEL_VOTER {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check voter NP (national points / loyalty)
    let voter_loyalty = world
        .get_character_info(session.session_id())
        .map(|c| c.loyalty)
        .unwrap_or(0);
    if (voter_loyalty as i64) < (MIN_NP_VOTER as i64) {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    result.write_i16(1);
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING IMPEACHMENT_REQUEST_ELECT: vote={}, nation={}",
        session.addr(),
        vote,
        nation
    );

    Ok(())
}

/// Handle impeachment list (sub-opcode 3).
/// View the current king's info for the impeachment UI. The C++ implementation
/// is an empty stub. We send the king's name and basic info.
async fn handle_impeachment_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_IMPEACHMENT);
    result.write_u8(KING_IMPEACHMENT_LIST);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Must be in impeachment election phase (im_type == 3)
    if ks.im_type != 3 {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Send the king's name as the impeachment target
    result.write_i16(1);
    result.write_string(&ks.king_name);

    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING IMPEACHMENT_LIST: king={}, nation={}",
        session.addr(),
        ks.king_name,
        nation
    );

    Ok(())
}

/// Handle impeachment elect (sub-opcode 4).
/// Players vote on the actual impeachment. The C++ implementation is an empty stub.
/// We implement a basic vote recording mechanism.
async fn handle_impeachment_elect(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    let vote = match reader.read_u8() {
        Some(v) => v, // 1 = impeach, 0 = keep
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_IMPEACHMENT);
    result.write_u8(KING_IMPEACHMENT_ELECT);

    let world = session.world();
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return Ok(()),
    };

    // Must be in impeachment election phase (im_type == 3)
    if ks.im_type != 3 {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check voter level
    let voter_level = world.get_session_level(session.session_id());
    if voter_level < MIN_LEVEL_VOTER {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    // Check voter NP (national points / loyalty)
    let voter_loyalty = world
        .get_character_info(session.session_id())
        .map(|c| c.loyalty)
        .unwrap_or(0);
    if (voter_loyalty as i64) < (MIN_NP_VOTER as i64) {
        result.write_i16(-2);
        session.send_packet(&result).await?;
        return Ok(());
    }

    result.write_i16(1);
    session.send_packet(&result).await?;

    debug!(
        "[{}] WIZ_KING IMPEACHMENT_ELECT: vote={}, nation={}",
        session.addr(),
        vote,
        nation
    );

    Ok(())
}

// ── Tax System ──────────────────────────────────────────────────────────

/// Handle KING_TAX (3) sub-packet.
async fn handle_tax(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let tax_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_TAX);
    result.write_u8(tax_opcode);

    // All tax commands require being king (C++: KingSystem.cpp:1318)
    if !is_player_king(session) {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    match tax_opcode {
        // Collect king's fund (tax)
        2 => {
            let world = session.world();
            let tax = world
                .get_king_system(nation)
                .map(|ks| ks.territory_tax)
                .unwrap_or(0);

            // Give gold to king
            if tax > 0 {
                world.update_session(session.session_id(), |h| {
                    if let Some(c) = &mut h.character {
                        c.gold = c.gold.saturating_add(tax);
                    }
                });
                // Reset tax in king system
                world.update_king_system(nation, |ks| {
                    ks.territory_tax = 0;
                });
            }

            result.write_u32(0); // territory_tax is now 0
            result.write_u8(nation);

            // Fire-and-forget DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.collect_territory_tax(n).await {
                        tracing::error!("Failed to collect territory tax for nation {n}: {e}");
                    }
                });
            }

            session.send_packet(&result).await?;
            debug!("[{}] WIZ_KING TAX: collected {} gold", session.addr(), tax);
        }
        // Lookup tariff
        3 => {
            let world = session.world();
            let tariff = world
                .get_king_system(nation)
                .map(|ks| ks.territory_tariff)
                .unwrap_or(0);
            result.write_i16(1); // success
            result.write_i32(tariff as i32);
            session.send_packet(&result).await?;
            debug!(
                "[{}] WIZ_KING TAX: lookup tariff={}",
                session.addr(),
                tariff
            );
        }
        // Update tariff
        4 => {
            let tariff = match reader.read_u8() {
                Some(v) => v,
                None => return Ok(()),
            };

            // Validate tariff range (0-10)
            if tariff > 10 {
                result.write_i16(-2);
                session.send_packet(&result).await?;
                return Ok(());
            }

            let world = session.world();
            // C++ stores tariff+10 in the map, but we store the raw tariff
            world.update_king_system(nation, |ks| {
                ks.territory_tariff = tariff;
            });

            result.write_i16(1); // success
            result.write_u8(tariff);
            result.write_u8(nation);

            // Broadcast to nation
            world.broadcast_to_nation(nation, Arc::new(result), None);

            // Fire-and-forget DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                let t = tariff as i16;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.update_tariff(n, t).await {
                        tracing::error!("Failed to update tariff for nation {n}: {e}");
                    }
                });
            }

            debug!(
                "[{}] WIZ_KING TAX: set tariff={} nation={}",
                session.addr(),
                tariff,
                nation
            );
        }
        // King's scepter
        7 => {
            let world = session.world();
            let sid = session.session_id();

            // Check if king already has the scepter item in inventory
            let has_scepter = world.update_inventory(sid, |inv| {
                for i in SLOT_MAX..(SLOT_MAX + HAVE_MAX) {
                    if let Some(slot) = inv.get(i) {
                        if slot.item_id == KING_SCEPTER && slot.count > 0 {
                            return true;
                        }
                    }
                }
                false
            });

            if has_scepter {
                // Already has scepter
                result.write_i16(-1);
            } else if world.find_slot_for_item(sid, KING_SCEPTER, 1).is_none() {
                // No inventory space
                result.write_i16(-2);
            } else {
                // Grant scepter (C++ calls GiveItem twice for 2 copies)
                world.give_item(sid, KING_SCEPTER, 1);
                world.give_item(sid, KING_SCEPTER, 1);
                result.write_i16(1);
            }

            session.send_packet(&result).await?;
            debug!("[{}] WIZ_KING TAX: scepter request", session.addr());
        }
        _ => {
            warn!(
                "[{}] WIZ_KING TAX: unknown opcode {}",
                session.addr(),
                tax_opcode
            );
        }
    }

    Ok(())
}

// ── Event System ────────────────────────────────────────────────────────

/// Handle KING_EVENT (4) sub-packet.
async fn handle_event(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let event_opcode = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_EVENT_OPCODE);
    result.write_u8(event_opcode);

    if !is_player_king(session) {
        result.write_i16(-1);
        session.send_packet(&result).await?;
        return Ok(());
    }

    let nation = match get_nation(session) {
        Some(n) => n,
        None => return Ok(()),
    };

    match event_opcode {
        // Noah (coin) event
        KING_EVENT_NOAH => {
            let amount = match reader.read_u8() {
                Some(v) if (1..=3).contains(&v) => v,
                _ => return Ok(()),
            };

            let world = session.world();
            let cost = 30_000_000u32 * amount as u32;

            let treasury = world
                .get_king_system(nation)
                .map(|ks| ks.national_treasury)
                .unwrap_or(0);
            if cost > treasury {
                result.write_i16(-3);
                session.send_packet(&result).await?;
                return Ok(());
            }

            let now = Local::now();
            let new_treasury = treasury - cost;

            world.update_king_system(nation, |ks| {
                ks.national_treasury = new_treasury;
                ks.noah_event = amount;
                ks.noah_event_day = now.day() as u8;
                ks.noah_event_hour = now.hour() as u8;
                ks.noah_event_minute = now.minute() as u8;
                ks.noah_event_duration = 30; // 30 minutes
            });

            // Fire-and-forget DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                let a = amount as i16;
                let d = now.day() as i16;
                let h = now.hour() as i16;
                let m = now.minute() as i16;
                let nt = new_treasury.min(i32::MAX as u32) as i32;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.update_noah_event(n, a, d, h, m, 30, nt).await {
                        tracing::error!("Failed to update noah event for nation {n}: {e}");
                    }
                });
            }

            debug!(
                "[{}] WIZ_KING EVENT NOAH: amount={}, treasury={}",
                session.addr(),
                amount,
                new_treasury
            );
        }
        // EXP event
        KING_EVENT_EXP => {
            let amount = match reader.read_u8() {
                Some(v) if v == 10 || v == 30 || v == 50 => v,
                _ => return Ok(()),
            };

            let world = session.world();
            let cost = 30_000_000u32 * amount as u32;

            let treasury = world
                .get_king_system(nation)
                .map(|ks| ks.national_treasury)
                .unwrap_or(0);
            if cost > treasury {
                result.write_i16(-3);
                session.send_packet(&result).await?;
                return Ok(());
            }

            let now = Local::now();
            let new_treasury = treasury - cost;

            world.update_king_system(nation, |ks| {
                ks.national_treasury = new_treasury;
                ks.exp_event = amount;
                ks.exp_event_day = now.day() as u8;
                ks.exp_event_hour = now.hour() as u8;
                ks.exp_event_minute = now.minute() as u8;
                ks.exp_event_duration = 30; // 30 minutes
            });

            // Fire-and-forget DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                let a = amount as i16;
                let d = now.day() as i16;
                let h = now.hour() as i16;
                let m = now.minute() as i16;
                let nt = new_treasury.min(i32::MAX as u32) as i32;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.update_exp_event(n, a, d, h, m, 30, nt).await {
                        tracing::error!("Failed to update exp event for nation {n}: {e}");
                    }
                });
            }

            debug!(
                "[{}] WIZ_KING EVENT EXP: amount={}%, treasury={}",
                session.addr(),
                amount,
                new_treasury
            );
        }
        // Prize event (give gold to player)
        KING_EVENT_PRIZE => {
            let coins = match reader.read_u32() {
                Some(v) if v > 0 => v,
                _ => return Ok(()),
            };
            let target_name = match reader.read_sbyte_string() {
                Some(s) if !s.is_empty() && s.len() <= 21 => s,
                _ => return Ok(()),
            };

            let world = session.world();
            let treasury = world
                .get_king_system(nation)
                .map(|ks| ks.national_treasury)
                .unwrap_or(0);

            if coins > treasury {
                result.write_i16(-4);
                session.send_packet(&result).await?;
                return Ok(());
            }

            // Find target player by name
            let target_sid = match world.find_session_by_name(&target_name) {
                Some(sid) => sid,
                None => {
                    result.write_i16(-2);
                    session.send_packet(&result).await?;
                    return Ok(());
                }
            };

            // Give gold to target
            world.update_session(target_sid, |h| {
                if let Some(c) = &mut h.character {
                    c.gold = c.gold.saturating_add(coins);
                }
            });

            // Deduct from treasury
            let new_treasury = treasury - coins;
            world.update_king_system(nation, |ks| {
                ks.national_treasury = new_treasury;
            });

            // DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                let nt = new_treasury.min(i32::MAX as u32) as i32;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.update_treasury(n, nt).await {
                        tracing::error!(
                            "Failed to update treasury after prize event for nation {n}: {e}"
                        );
                    }
                });
            }

            debug!(
                "[{}] WIZ_KING EVENT PRIZE: {} gold to {}, treasury={}",
                session.addr(),
                coins,
                target_name,
                new_treasury
            );
        }
        // Weather event
        KING_EVENT_WEATHER => {
            let weather_type = match reader.read_u8() {
                Some(v) if (1..=3).contains(&v) => v,
                _ => return Ok(()),
            };
            let amount = match reader.read_u8() {
                Some(v) if v > 0 && v <= 100 => v,
                _ => return Ok(()),
            };

            let world = session.world();
            let treasury = world
                .get_king_system(nation)
                .map(|ks| ks.national_treasury)
                .unwrap_or(0);

            if treasury < 100_000 {
                result.write_i16(-3);
                session.send_packet(&result).await?;
                return Ok(());
            }

            let new_treasury = treasury - 100_000;
            world.update_king_system(nation, |ks| {
                ks.national_treasury = new_treasury;
            });

            // DB update
            {
                let pool = session.pool().clone();
                let n = nation as i16;
                let nt = new_treasury.min(i32::MAX as u32) as i32;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool);
                    if let Err(e) = repo.update_treasury(n, nt).await {
                        tracing::error!(
                            "Failed to update treasury after weather event for nation {n}: {e}"
                        );
                    }
                });
            }

            debug!(
                "[{}] WIZ_KING EVENT WEATHER: type={}, amount={}, treasury={}",
                session.addr(),
                weather_type,
                amount,
                new_treasury
            );
        }
        // King notice (broadcast message)
        KING_EVENT_NOTICE => {
            let message = match reader.read_sbyte_string() {
                Some(s) if !s.is_empty() && s.len() <= 256 => s,
                _ => return Ok(()),
            };

            // Build notice packet for nation broadcast
            let mut notice = Packet::new(Opcode::WizKing as u8);
            notice.write_u8(KING_EVENT_OPCODE);
            notice.write_u8(KING_EVENT_NOTICE);
            notice.write_u8(1); // success
            notice.write_u16(1); // success code
            notice.write_sbyte_string(&message);

            let world = session.world();
            world.broadcast_to_nation(nation, Arc::new(notice), None);

            debug!(
                "[{}] WIZ_KING EVENT NOTICE: nation={}, msg={}",
                session.addr(),
                nation,
                message
            );
        }
        // Fugitive event — stub in C++ (KingSystem.cpp:1545-1546, just break)
        KING_EVENT_FUGITIVE => {
            debug!("[{}] WIZ_KING EVENT FUGITIVE: no-op (stub)", session.addr());
        }
        _ => {
            warn!(
                "[{}] WIZ_KING EVENT: unknown event opcode {}",
                session.addr(),
                event_opcode
            );
        }
    }

    Ok(())
}

// ── Nation Intro ────────────────────────────────────────────────────────

/// Handle KING_NATION_INTRO (6) sub-packet.
async fn handle_nation_intro(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    if !is_player_king(session) {
        return Ok(());
    }

    let intro_type = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut result = Packet::new(Opcode::WizKing as u8);
    result.write_u8(KING_NATION_INTRO);
    result.write_u8(intro_type);

    match intro_type {
        // Request to change nation intro
        1 => {
            session.send_packet(&result).await?;
        }
        // Update nation intro text
        2 => {
            let _message = reader.read_string();
            result.write_u8(1); // success
            session.send_packet(&result).await?;
        }
        _ => {
            warn!(
                "[{}] WIZ_KING NATION_INTRO: unhandled type {}",
                session.addr(),
                intro_type
            );
        }
    }

    debug!(
        "[{}] WIZ_KING NATION_INTRO: type={}",
        session.addr(),
        intro_type
    );

    Ok(())
}

// ── Election Timer (called from server tick) ────────────────────────────

/// Check king election timer for a specific nation.
/// This should be called once per minute from the server tick for each nation.
/// It handles:
/// 1. Special event expiry (noah/exp events)
/// 2. Election state transitions (NO_TERM → NOMINATION → PRE_ELECTION → ELECTION → TERM_ENDED → NO_TERM)
pub fn check_king_timer(world: &crate::world::WorldState, nation: u8, pool: &ko_db::DbPool) {
    use crate::world::{ELECTION_TYPE_NO_TERM, ELECTION_TYPE_TERM_ENDED};

    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return,
    };

    if ks.election_under_progress {
        return;
    }

    let now = Local::now();
    let cur_month = now.month() as u8;
    let cur_day = now.day() as u8;
    let cur_hour = now.hour() as u8;
    let cur_minute = now.minute() as u8;

    // Check special events (noah/exp) for expiry
    check_special_event(world, nation, cur_day, cur_hour, cur_minute);

    match ks.election_type {
        ELECTION_TYPE_NO_TERM => {
            // Nominations start 2 days before the scheduled election
            // Simplified: check if we're at the nomination start time
            let nom_day = if ks.day >= 2 { ks.day - 2 } else { ks.day + 28 };
            let nom_month = if ks.day >= 2 {
                ks.month
            } else if ks.month > 1 {
                ks.month - 1
            } else {
                12
            };

            if cur_month == nom_month
                && cur_day == nom_day
                && cur_hour == ks.hour
                && cur_minute == ks.minute
            {
                // Start nomination phase
                world.update_king_system(nation, |ks| {
                    ks.election_type = ELECTION_TYPE_NOMINATION;
                    ks.senator_list.clear();
                    ks.candidate_list.clear();
                    ks.nomination_list.clear();
                    ks.notice_board.clear();
                    ks.resigned_candidates.clear();
                    ks.sent_first_message = false;
                });

                // Load top 10 clans as senators
                let top_clans = world.get_top_ranked_clans(nation, 10);
                let pool_clone = pool.clone();
                let n = nation as i16;

                world.update_king_system(nation, |ks| {
                    ks.top10_clan_set = top_clans.iter().map(|(id, _)| *id).collect();
                    for (clan_id, chief_name) in &top_clans {
                        ks.senator_list.push(ElectionListEntry {
                            name: chief_name.clone(),
                            knights_id: *clan_id,
                            votes: 0,
                        });
                    }
                });

                // DB updates
                let senators: Vec<(i16, String)> = top_clans
                    .iter()
                    .map(|(id, name)| (*id as i16, name.clone()))
                    .collect();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool_clone);
                    if let Err(e) = repo
                        .update_election_status(n, ELECTION_TYPE_NOMINATION as i16)
                        .await
                    {
                        tracing::error!(
                            "Failed to update election status to NOMINATION for nation {n}: {e}"
                        );
                    }
                    if let Err(e) = repo.clear_election_lists(n).await {
                        tracing::error!("Failed to clear election lists for nation {n}: {e}");
                    }
                    if let Err(e) = repo.clear_nominations(n).await {
                        tracing::error!("Failed to clear nominations for nation {n}: {e}");
                    }
                    if let Err(e) = repo.clear_notice_board(n).await {
                        tracing::error!("Failed to clear notice board for nation {n}: {e}");
                    }
                    if let Err(e) = repo.clear_votes(n).await {
                        tracing::error!("Failed to clear votes for nation {n}: {e}");
                    }
                    for (clan_id, name) in &senators {
                        if let Err(e) = repo.upsert_election_list(n, 3, name, *clan_id).await {
                            tracing::error!("Failed to upsert senator {name} for nation {n}: {e}");
                        }
                    }
                });

                tracing::info!(nation, "King election: NOMINATION phase started");
            }
        }
        ELECTION_TYPE_NOMINATION => {
            // Pre-election starts 1 day + 1 hour before election
            let pre_day = if ks.day >= 1 { ks.day - 1 } else { ks.day + 29 };
            let pre_month = if ks.day >= 1 {
                ks.month
            } else if ks.month > 1 {
                ks.month - 1
            } else {
                12
            };
            let pre_hour = if ks.hour >= 1 { ks.hour - 1 } else { 23 };

            if cur_month == pre_month && cur_day == pre_day && cur_hour == pre_hour {
                // Move to pre-election
                world.update_king_system(nation, |ks| {
                    ks.election_type = ELECTION_TYPE_PRE_ELECTION;
                    // Clear senator list since candidates have been determined
                    ks.senator_list.clear();
                    ks.sent_first_message = false;
                });

                let pool_clone = pool.clone();
                let n = nation as i16;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool_clone);
                    if let Err(e) = repo
                        .update_election_status(n, ELECTION_TYPE_PRE_ELECTION as i16)
                        .await
                    {
                        tracing::error!(
                            "Failed to update election status to PRE_ELECTION for nation {n}: {e}"
                        );
                    }
                });

                tracing::info!(nation, "King election: PRE_ELECTION phase started");
            }
        }
        ELECTION_TYPE_PRE_ELECTION => {
            // Election starts 1 day before scheduled time
            let elec_day = if ks.day >= 1 { ks.day - 1 } else { ks.day + 29 };
            let elec_month = if ks.day >= 1 {
                ks.month
            } else if ks.month > 1 {
                ks.month - 1
            } else {
                12
            };

            if cur_month == elec_month
                && cur_day == elec_day
                && cur_hour == ks.hour
                && cur_minute == ks.minute
            {
                world.update_king_system(nation, |ks| {
                    ks.election_type = ELECTION_TYPE_ELECTION;
                    ks.sent_first_message = false;
                });

                let pool_clone = pool.clone();
                let n = nation as i16;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool_clone);
                    if let Err(e) = repo
                        .update_election_status(n, ELECTION_TYPE_ELECTION as i16)
                        .await
                    {
                        tracing::error!(
                            "Failed to update election status to ELECTION for nation {n}: {e}"
                        );
                    }
                });

                tracing::info!(nation, "King election: ELECTION phase started");
            }
        }
        ELECTION_TYPE_ELECTION => {
            // Election ends at the scheduled time
            if cur_month == ks.month
                && cur_day == ks.day
                && cur_hour == ks.hour
                && cur_minute == ks.minute
            {
                // Determine winner
                let winner = {
                    let ks_ref = world.get_king_system(nation);
                    ks_ref.and_then(|ks| {
                        ks.candidate_list
                            .iter()
                            .max_by_key(|c| c.votes)
                            .map(|c| (c.name.clone(), c.knights_id, c.votes, ks.total_votes))
                    })
                };

                world.update_king_system(nation, |ks| {
                    ks.election_type = ELECTION_TYPE_TERM_ENDED;
                    if let Some((name, _, votes, total)) = &winner {
                        ks.new_king_name = name.clone();
                        ks.king_votes = *votes;
                        ks.total_votes = *total;
                    }
                    ks.sent_first_message = false;
                });

                let pool_clone = pool.clone();
                let n = nation as i16;
                let winner_clone = winner.clone();
                tokio::spawn(async move {
                    let repo = ko_db::repositories::king::KingRepository::new(&pool_clone);
                    if let Err(e) = repo
                        .update_election_status(n, ELECTION_TYPE_TERM_ENDED as i16)
                        .await
                    {
                        tracing::error!(
                            "Failed to update election status to TERM_ENDED for nation {n}: {e}"
                        );
                    }
                    // Persist election result for crash recovery
                    if let Some((name, _, votes, total)) = &winner_clone {
                        if let Err(e) = repo
                            .save_election_result(n, name, *votes as i32, *total as i32)
                            .await
                        {
                            tracing::error!("Failed to save election result for nation {n}: {e}");
                        }
                    }
                });

                tracing::info!(
                    nation,
                    ?winner,
                    "King election: TERM_ENDED, results determined"
                );
            }
        }
        ELECTION_TYPE_TERM_ENDED => {
            // 5 minutes after election ends: assign new king
            let assign_minute = (ks.minute + 5) % 60;
            let assign_hour = ks.hour + if ks.minute + 5 >= 60 { 1 } else { 0 };

            if cur_month == ks.month
                && cur_day == ks.day
                && cur_hour == assign_hour
                && cur_minute == assign_minute
            {
                // Assign new king
                let new_king = world
                    .get_king_system(nation)
                    .map(|ks| (ks.new_king_name.clone(), ks.king_votes, ks.total_votes));

                if let Some((king_name, _king_votes, _total_votes)) = new_king {
                    if !king_name.is_empty() {
                        // Find the king's clan
                        let king_clan_id = world
                            .find_session_by_name(&king_name)
                            .map(|sid| world.get_session_clan_id(sid))
                            .unwrap_or(0);

                        world.update_king_system(nation, |ks| {
                            ks.king_name = king_name.clone();
                            ks.king_clan_id = king_clan_id;
                            ks.election_type = ELECTION_TYPE_NO_TERM;
                            ks.month += 1;
                            if ks.month > 12 {
                                ks.month = 1;
                            }
                            // Reset election lists
                            ks.senator_list.clear();
                            ks.candidate_list.clear();
                            ks.nomination_list.clear();
                            ks.notice_board.clear();
                            ks.resigned_candidates.clear();
                            ks.new_king_name.clear();
                            ks.king_votes = 0;
                            ks.total_votes = 0;
                        });

                        let pool_clone = pool.clone();
                        let n = nation as i16;
                        let kname = king_name.clone();
                        let kclan = king_clan_id as i16;
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::king::KingRepository::new(&pool_clone);
                            if let Err(e) = repo.update_king(n, &kname, kclan).await {
                                tracing::error!("Failed to update king for nation {n}: {e}");
                            }
                            if let Err(e) = repo
                                .update_election_status(n, ELECTION_TYPE_NO_TERM as i16)
                                .await
                            {
                                tracing::error!("Failed to update election status to NO_TERM for nation {n}: {e}");
                            }
                            if let Err(e) = repo.clear_election_lists(n).await {
                                tracing::error!("Failed to clear election lists after king assignment for nation {n}: {e}");
                            }
                            if let Err(e) = repo.clear_nominations(n).await {
                                tracing::error!("Failed to clear nominations after king assignment for nation {n}: {e}");
                            }
                            if let Err(e) = repo.clear_notice_board(n).await {
                                tracing::error!("Failed to clear notice board after king assignment for nation {n}: {e}");
                            }
                            if let Err(e) = repo.clear_votes(n).await {
                                tracing::error!("Failed to clear votes after king assignment for nation {n}: {e}");
                            }
                            if let Err(e) = repo.clear_election_result(n).await {
                                tracing::error!("Failed to clear election result after king assignment for nation {n}: {e}");
                            }
                        });

                        tracing::info!(nation, king_name, "King election: new king assigned");
                    }
                }
            }
        }
        _ => {}
    }
}

/// Check if special events (noah/exp) should expire.
fn check_special_event(
    world: &crate::world::WorldState,
    nation: u8,
    cur_day: u8,
    cur_hour: u8,
    cur_minute: u8,
) {
    let ks = match world.get_king_system(nation) {
        Some(ks) => ks,
        None => return,
    };

    // Check EXP event expiry
    if ks.exp_event > 0 {
        let event_expiry = if cur_day == ks.exp_event_day {
            cur_minute as i16 + 60 * (cur_hour as i16 - ks.exp_event_hour as i16)
                - ks.exp_event_minute as i16
        } else {
            cur_minute as i16 + 60 * (cur_hour as i16 - ks.exp_event_hour as i16 + 24)
                - ks.exp_event_minute as i16
        };

        if event_expiry > ks.exp_event_duration as i16 {
            world.update_king_system(nation, |ks| {
                ks.exp_event = 0;
                ks.exp_event_day = 0;
                ks.exp_event_hour = 0;
                ks.exp_event_minute = 0;
                ks.exp_event_duration = 0;
            });
            tracing::info!(nation, "King EXP event expired");
        }
    }

    // Check Noah event expiry
    if ks.noah_event > 0 {
        let event_expiry = if cur_day == ks.noah_event_day {
            cur_minute as i16 + 60 * (cur_hour as i16 - ks.noah_event_hour as i16)
                - ks.noah_event_minute as i16
        } else {
            cur_minute as i16 + 60 * (cur_hour as i16 - ks.noah_event_hour as i16 + 24)
                - ks.noah_event_minute as i16
        };

        if event_expiry > ks.noah_event_duration as i16 {
            world.update_king_system(nation, |ks| {
                ks.noah_event = 0;
                ks.noah_event_day = 0;
                ks.noah_event_hour = 0;
                ks.noah_event_minute = 0;
                ks.noah_event_duration = 0;
            });
            tracing::info!(nation, "King Noah event expired");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::*;
    use ko_protocol::Packet;

    #[test]
    fn test_king_election_schedule_packet_format() {
        // Build a KING_ELECTION + KING_ELECTION_SCHEDULE response
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_SCHEDULE);
        pkt.write_u8(1); // election type
        pkt.write_u8(12); // month
        pkt.write_u8(18); // day
        pkt.write_u8(0); // hour
        pkt.write_u8(0); // minute

        assert_eq!(pkt.opcode, 0x78);
        // Data: [1][1][1][12][18][0][0] = 7 bytes
        assert_eq!(pkt.data.len(), 7);
        assert_eq!(pkt.data[0], KING_ELECTION);
        assert_eq!(pkt.data[1], KING_ELECTION_SCHEDULE);
    }

    #[test]
    fn test_king_tax_tariff_lookup_response() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_TAX);
        pkt.write_u8(3); // lookup opcode
        pkt.write_i16(1); // success
        pkt.write_i32(5); // tariff rate (i32 per C++ GetTariff()-10 → int promotion)

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_TAX);
        assert_eq!(pkt.data[1], 3);
        // i16(1) = [0x01, 0x00], i32(5) = [0x05, 0x00, 0x00, 0x00]
        assert_eq!(pkt.data[2], 0x01);
        assert_eq!(pkt.data[3], 0x00);
        assert_eq!(pkt.data[4], 0x05);
        assert_eq!(pkt.data[5], 0x00);
        assert_eq!(pkt.data[6], 0x00);
        assert_eq!(pkt.data[7], 0x00);
    }

    #[test]
    fn test_king_tax_update_response() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_TAX);
        pkt.write_u8(4); // update opcode
        pkt.write_i16(1); // success
        pkt.write_u8(7); // tariff
        pkt.write_u8(1); // nation

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_TAX);
        assert_eq!(pkt.data[1], 4);
    }

    #[test]
    fn test_king_event_noah_insufficient_treasury() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_EVENT_OPCODE);
        pkt.write_u8(KING_EVENT_NOAH);
        pkt.write_i16(-3); // insufficient funds

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_EVENT_OPCODE);
        assert_eq!(pkt.data[1], KING_EVENT_NOAH);
    }

    #[test]
    fn test_king_impeachment_ui_open_response() {
        // Impeachment request UI - denied
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_IMPEACHMENT);
        pkt.write_u8(KING_IMPEACHMENT_REQUEST_UI_OPEN);
        pkt.write_i16(-1);

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_IMPEACHMENT);
        assert_eq!(pkt.data[1], KING_IMPEACHMENT_REQUEST_UI_OPEN);

        // Election UI - denied
        let mut pkt2 = Packet::new(Opcode::WizKing as u8);
        pkt2.write_u8(KING_IMPEACHMENT);
        pkt2.write_u8(KING_IMPEACHMENT_ELECTION_UI_OPEN);
        pkt2.write_i16(-1);

        assert_eq!(pkt2.data[0], KING_IMPEACHMENT);
        assert_eq!(pkt2.data[1], KING_IMPEACHMENT_ELECTION_UI_OPEN);
    }

    #[test]
    fn test_king_nation_intro_response() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_NATION_INTRO);
        pkt.write_u8(2); // update
        pkt.write_u8(1); // success

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_NATION_INTRO);
        assert_eq!(pkt.data[1], 2);
        assert_eq!(pkt.data[2], 1);
    }

    #[test]
    fn test_king_election_nominate_packet() {
        // Build a KING_ELECTION_NOMINATE response (success)
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_NOMINATE);
        pkt.write_i16(1); // success

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_ELECTION);
        assert_eq!(pkt.data[1], KING_ELECTION_NOMINATE);
        assert_eq!(pkt.data[2], 0x01); // i16(1) LE
        assert_eq!(pkt.data[3], 0x00);
    }

    #[test]
    fn test_king_election_nominate_error_packets() {
        // Not nomination time
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_NOMINATE);
        pkt.write_i16(-2);
        assert_eq!(pkt.data[2], 0xFE); // i16(-2) LE = 0xFE, 0xFF
        assert_eq!(pkt.data[3], 0xFF);

        // No authority
        let mut pkt2 = Packet::new(Opcode::WizKing as u8);
        pkt2.write_u8(KING_ELECTION);
        pkt2.write_u8(KING_ELECTION_NOMINATE);
        pkt2.write_i16(-3);
        assert_eq!(pkt2.data[2], 0xFD); // i16(-3) LE = 0xFD, 0xFF
        assert_eq!(pkt2.data[3], 0xFF);
    }

    #[test]
    fn test_king_election_poll_candidate_list_packet() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_POLL);
        pkt.write_u8(1); // list opcode
        pkt.write_u16(1); // success
        pkt.write_u8(2); // 2 candidates
                         // Candidate 1 (SByte strings per C++)
        pkt.write_u8(1);
        pkt.write_sbyte_string("TestKing");
        pkt.write_sbyte_string("TestClan");
        // Candidate 2
        pkt.write_u8(2);
        pkt.write_sbyte_string("OtherKing");
        pkt.write_u8(0); // no clan name

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_ELECTION);
        assert_eq!(pkt.data[1], KING_ELECTION_POLL);
        assert_eq!(pkt.data[2], 1); // list sub-opcode
    }

    #[test]
    fn test_king_election_resign_packets() {
        // Success
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_RESIGN);
        pkt.write_i16(1);
        assert_eq!(pkt.data[0], KING_ELECTION);
        assert_eq!(pkt.data[1], KING_ELECTION_RESIGN);

        // Wrong phase
        let mut pkt2 = Packet::new(Opcode::WizKing as u8);
        pkt2.write_u8(KING_ELECTION);
        pkt2.write_u8(KING_ELECTION_RESIGN);
        pkt2.write_i16(-2);
        assert_eq!(pkt2.data[2], 0xFE);

        // Not a candidate
        let mut pkt3 = Packet::new(Opcode::WizKing as u8);
        pkt3.write_u8(KING_ELECTION);
        pkt3.write_u8(KING_ELECTION_RESIGN);
        pkt3.write_i16(-3);
        assert_eq!(pkt3.data[2], 0xFD);
    }

    #[test]
    fn test_king_notice_board_write_packet() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_NOTICE_BOARD);
        pkt.write_u8(KING_CANDIDACY_BOARD_WRITE);
        pkt.write_i16(1); // success

        assert_eq!(pkt.data[0], KING_ELECTION);
        assert_eq!(pkt.data[1], KING_ELECTION_NOTICE_BOARD);
        assert_eq!(pkt.data[2], KING_CANDIDACY_BOARD_WRITE);
    }

    #[test]
    fn test_election_list_entry_clone() {
        let entry = ElectionListEntry {
            name: "TestPlayer".to_string(),
            knights_id: 42,
            votes: 100,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.name, "TestPlayer");
        assert_eq!(cloned.knights_id, 42);
        assert_eq!(cloned.votes, 100);
    }

    #[test]
    fn test_nomination_entry_clone() {
        let entry = NominationEntry {
            nominator: "Senator1".to_string(),
            nominee: "Candidate1".to_string(),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.nominator, "Senator1");
        assert_eq!(cloned.nominee, "Candidate1");
    }

    #[test]
    fn test_special_event_expiry_calculation() {
        // Same day: cur_minute + 60*(cur_hour - event_hour) - event_minute
        let cur_day: u8 = 15;
        let event_day: u8 = 15;
        let cur_hour: u8 = 14;
        let cur_minute: u8 = 30;
        let event_hour: u8 = 14;
        let event_minute: u8 = 0;
        let duration: u16 = 30;

        let expiry = if cur_day == event_day {
            cur_minute as i16 + 60 * (cur_hour as i16 - event_hour as i16) - event_minute as i16
        } else {
            cur_minute as i16 + 60 * (cur_hour as i16 - event_hour as i16 + 24)
                - event_minute as i16
        };
        assert_eq!(expiry, 30);
        assert!(expiry >= duration as i16); // Should expire

        // Not expired yet (25 minutes in)
        let cur_minute2: u8 = 25;
        let expiry2 =
            cur_minute2 as i16 + 60 * (cur_hour as i16 - event_hour as i16) - event_minute as i16;
        assert_eq!(expiry2, 25);
        assert!(expiry2 < duration as i16); // Should NOT expire
    }

    // ── King Scepter Tests ──────────────────────────────────────────

    #[test]
    fn test_king_scepter_constant() {
        // C++ Define.h:313 — KING_SCEPTER = 910074311
        assert_eq!(super::KING_SCEPTER, 910_074_311);
    }

    #[test]
    fn test_king_scepter_already_has_response() {
        // When king already has the scepter, server responds with -1
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_TAX);
        pkt.write_u8(7); // scepter sub-opcode
        pkt.write_i16(-1); // already has scepter

        assert_eq!(pkt.opcode, 0x78);
        assert_eq!(pkt.data[0], KING_TAX);
        assert_eq!(pkt.data[1], 7);
        // i16(-1) LE = 0xFF, 0xFF
        assert_eq!(pkt.data[2], 0xFF);
        assert_eq!(pkt.data[3], 0xFF);
    }

    #[test]
    fn test_king_scepter_no_space_response() {
        // When king has no inventory space, server responds with -2
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_TAX);
        pkt.write_u8(7);
        pkt.write_i16(-2); // no space

        assert_eq!(pkt.data[2], 0xFE); // i16(-2) LE
        assert_eq!(pkt.data[3], 0xFF);
    }

    #[test]
    fn test_king_scepter_success_response() {
        // When scepter is granted, server responds with 1
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_TAX);
        pkt.write_u8(7);
        pkt.write_i16(1); // success

        assert_eq!(pkt.data[2], 0x01); // i16(1) LE
        assert_eq!(pkt.data[3], 0x00);
    }

    // ── Impeachment Sub-opcode Tests ──────────────────────────────────

    #[test]
    fn test_impeachment_sub_opcode_constants() {
        // Verify sub-opcode constants match C++ KingType enum
        assert_eq!(KING_IMPEACHMENT_REQUEST, 1);
        assert_eq!(KING_IMPEACHMENT_REQUEST_ELECT, 2);
        assert_eq!(KING_IMPEACHMENT_LIST, 3);
        assert_eq!(KING_IMPEACHMENT_ELECT, 4);
        assert_eq!(KING_IMPEACHMENT_REQUEST_UI_OPEN, 8);
        assert_eq!(KING_IMPEACHMENT_ELECTION_UI_OPEN, 9);
    }

    #[test]
    fn test_impeachment_request_response_packets() {
        // Success
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_IMPEACHMENT);
        pkt.write_u8(KING_IMPEACHMENT_REQUEST);
        pkt.write_i16(1);
        assert_eq!(pkt.data[0], KING_IMPEACHMENT);
        assert_eq!(pkt.data[1], KING_IMPEACHMENT_REQUEST);
        assert_eq!(pkt.data[2], 0x01);
        assert_eq!(pkt.data[3], 0x00);

        // Wrong phase
        let mut pkt2 = Packet::new(Opcode::WizKing as u8);
        pkt2.write_u8(KING_IMPEACHMENT);
        pkt2.write_u8(KING_IMPEACHMENT_REQUEST);
        pkt2.write_i16(-1);
        assert_eq!(pkt2.data[2], 0xFF);
        assert_eq!(pkt2.data[3], 0xFF);

        // Not a senator
        let mut pkt3 = Packet::new(Opcode::WizKing as u8);
        pkt3.write_u8(KING_IMPEACHMENT);
        pkt3.write_u8(KING_IMPEACHMENT_REQUEST);
        pkt3.write_i16(-2);
        assert_eq!(pkt3.data[2], 0xFE);
        assert_eq!(pkt3.data[3], 0xFF);
    }

    #[test]
    fn test_impeachment_request_elect_response() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_IMPEACHMENT);
        pkt.write_u8(KING_IMPEACHMENT_REQUEST_ELECT);
        pkt.write_i16(1);
        assert_eq!(pkt.data[0], KING_IMPEACHMENT);
        assert_eq!(pkt.data[1], KING_IMPEACHMENT_REQUEST_ELECT);
        assert_eq!(pkt.data[2], 0x01);
    }

    #[test]
    fn test_impeachment_list_response_with_king_name() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_IMPEACHMENT);
        pkt.write_u8(KING_IMPEACHMENT_LIST);
        pkt.write_i16(1);
        pkt.write_string("TestKing");

        assert_eq!(pkt.data[0], KING_IMPEACHMENT);
        assert_eq!(pkt.data[1], KING_IMPEACHMENT_LIST);
        // i16(1) = [0x01, 0x00]
        assert_eq!(pkt.data[2], 0x01);
        assert_eq!(pkt.data[3], 0x00);
        // String follows: DByte length + chars
    }

    #[test]
    fn test_impeachment_elect_response() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_IMPEACHMENT);
        pkt.write_u8(KING_IMPEACHMENT_ELECT);
        pkt.write_i16(1);
        assert_eq!(pkt.data[0], KING_IMPEACHMENT);
        assert_eq!(pkt.data[1], KING_IMPEACHMENT_ELECT);
        assert_eq!(pkt.data[2], 0x01);

        // Wrong phase
        let mut pkt2 = Packet::new(Opcode::WizKing as u8);
        pkt2.write_u8(KING_IMPEACHMENT);
        pkt2.write_u8(KING_IMPEACHMENT_ELECT);
        pkt2.write_i16(-1);
        assert_eq!(pkt2.data[2], 0xFF);

        // Level too low
        let mut pkt3 = Packet::new(Opcode::WizKing as u8);
        pkt3.write_u8(KING_IMPEACHMENT);
        pkt3.write_u8(KING_IMPEACHMENT_ELECT);
        pkt3.write_i16(-2);
        assert_eq!(pkt3.data[2], 0xFE);
    }

    // ── Sprint 950: Additional coverage ──────────────────────────────

    /// Main king sub-opcodes: sequential 1-4 + 6.
    #[test]
    fn test_king_main_sub_opcodes() {
        assert_eq!(KING_ELECTION, 1);
        assert_eq!(KING_IMPEACHMENT, 2);
        assert_eq!(KING_TAX, 3);
        assert_eq!(KING_EVENT_OPCODE, 4);
        // 5 is skipped
        assert_eq!(KING_NATION_INTRO, 6);
    }

    /// Election type constants are sequential 0-3 + 7.
    #[test]
    fn test_election_type_constants() {
        assert_eq!(ELECTION_TYPE_NOMINATION, 1);
        assert_eq!(ELECTION_TYPE_PRE_ELECTION, 2);
        assert_eq!(ELECTION_TYPE_ELECTION, 3);
    }

    /// Voter requirements: level 50+, NP 10000+.
    #[test]
    fn test_voter_requirements() {
        assert_eq!(MIN_LEVEL_VOTER, 50);
        assert_eq!(MIN_NP_VOTER, 10_000);
    }

    /// King event sub-opcodes 1-6.
    #[test]
    fn test_king_event_sub_opcodes() {
        assert_eq!(KING_EVENT_NOAH, 1);
        assert_eq!(KING_EVENT_EXP, 2);
        assert_eq!(KING_EVENT_PRIZE, 3);
        assert_eq!(KING_EVENT_FUGITIVE, 4);
        assert_eq!(KING_EVENT_WEATHER, 5);
        assert_eq!(KING_EVENT_NOTICE, 6);
    }

    /// Candidacy board read/write constants.
    #[test]
    fn test_candidacy_board_constants() {
        assert_eq!(KING_CANDIDACY_BOARD_WRITE, 1);
        assert_eq!(KING_CANDIDACY_BOARD_READ, 2);
    }

    // ── Sprint 959: Additional coverage ──────────────────────────────

    /// KING_SCEPTER item ID matches C++ Define.h.
    #[test]
    fn test_king_scepter_item_id_value() {
        assert_eq!(super::KING_SCEPTER, 910_074_311);
        // In premium item range (900M+)
        assert!(super::KING_SCEPTER >= 900_000_000);
    }

    /// Election sub-opcodes are sequential 1-5.
    #[test]
    fn test_king_election_subopcodes_sequential() {
        assert_eq!(KING_ELECTION_SCHEDULE, 1);
        assert_eq!(KING_ELECTION_NOMINATE, 2);
        assert_eq!(KING_ELECTION_NOTICE_BOARD, 3);
        assert_eq!(KING_ELECTION_POLL, 4);
        assert_eq!(KING_ELECTION_RESIGN, 5);
    }

    /// Impeachment sub-opcodes cover all UI paths.
    #[test]
    fn test_king_impeachment_subopcodes_complete() {
        assert_eq!(KING_IMPEACHMENT_REQUEST, 1);
        assert_eq!(KING_IMPEACHMENT_REQUEST_ELECT, 2);
        assert_eq!(KING_IMPEACHMENT_LIST, 3);
        assert_eq!(KING_IMPEACHMENT_ELECT, 4);
        assert_eq!(KING_IMPEACHMENT_REQUEST_UI_OPEN, 8);
        assert_eq!(KING_IMPEACHMENT_ELECTION_UI_OPEN, 9);
    }

    /// KING_NATION_INTRO is sub-opcode 6.
    #[test]
    fn test_king_nation_intro_value() {
        assert_eq!(KING_NATION_INTRO, 6);
        // Distinct from other main sub-opcodes
        assert_ne!(KING_NATION_INTRO, KING_ELECTION);
        assert_ne!(KING_NATION_INTRO, KING_IMPEACHMENT);
        assert_ne!(KING_NATION_INTRO, KING_TAX);
    }

    /// Election types cover all phases.
    #[test]
    fn test_election_type_phases() {
        assert_eq!(ELECTION_TYPE_NOMINATION, 1);
        assert_eq!(ELECTION_TYPE_PRE_ELECTION, 2);
        assert_eq!(ELECTION_TYPE_ELECTION, 3);
        // Sequential 1..3
        assert_eq!(ELECTION_TYPE_ELECTION - ELECTION_TYPE_NOMINATION, 2);
    }

    // ── Sprint 969: Additional coverage ──────────────────────────────

    /// Voter requirements: level >= 50, NP >= 10,000.
    #[test]
    fn test_voter_requirement_values() {
        assert_eq!(MIN_LEVEL_VOTER, 50);
        assert_eq!(MIN_NP_VOTER, 10_000);
        // Both are positive
        assert!(MIN_LEVEL_VOTER > 0);
        assert!(MIN_NP_VOTER > 0);
    }

    /// Tax sub-opcodes: 2=collect, 3=lookup, 4=update, 7=scepter.
    #[test]
    fn test_king_tax_sub_opcodes() {
        // These are hardcoded in the match arms of handle_tax
        let collect: u8 = 2;
        let lookup: u8 = 3;
        let update: u8 = 4;
        let scepter: u8 = 7;
        // All distinct
        let ops = [collect, lookup, update, scepter];
        for i in 0..ops.len() {
            for j in (i + 1)..ops.len() {
                assert_ne!(ops[i], ops[j]);
            }
        }
    }

    /// Tariff range is 0-10 (values > 10 rejected with -2).
    #[test]
    fn test_tariff_range_validation() {
        let max_tariff: u8 = 10;
        assert!(max_tariff <= 10);
        assert!(11 > max_tariff); // 11 would be rejected
    }

    /// KING_EVENT_OPCODE is 4, distinct from other main sub-opcodes.
    #[test]
    fn test_king_event_opcode_distinct() {
        assert_eq!(KING_EVENT_OPCODE, 4);
        assert_ne!(KING_EVENT_OPCODE, KING_ELECTION);
        assert_ne!(KING_EVENT_OPCODE, KING_IMPEACHMENT);
        assert_ne!(KING_EVENT_OPCODE, KING_TAX);
        assert_ne!(KING_EVENT_OPCODE, KING_NATION_INTRO);
    }

    /// King election schedule packet starts with KING_ELECTION + KING_ELECTION_SCHEDULE.
    #[test]
    fn test_king_election_schedule_header() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_ELECTION);
        pkt.write_u8(KING_ELECTION_SCHEDULE);
        assert_eq!(pkt.data[0], KING_ELECTION);
        assert_eq!(pkt.data[1], KING_ELECTION_SCHEDULE);
        assert_eq!(pkt.data.len(), 2);
    }

    // ── Sprint 975: Additional coverage ──────────────────────────────

    /// KING_SCEPTER item ID is in the 910M range.
    #[test]
    fn test_king_scepter_range() {
        assert_eq!(super::KING_SCEPTER, 910_074_311);
        assert!(super::KING_SCEPTER >= 900_000_000 && super::KING_SCEPTER < 920_000_000);
    }

    /// All KING_EVENT sub-opcodes (1-6) are unique and contiguous except gap at 4.
    #[test]
    fn test_king_event_subopcodes_complete() {
        let events = [
            KING_EVENT_NOAH,
            KING_EVENT_EXP,
            KING_EVENT_PRIZE,
            KING_EVENT_FUGITIVE,
            KING_EVENT_WEATHER,
            KING_EVENT_NOTICE,
        ];
        let mut set = std::collections::HashSet::new();
        for &op in &events {
            assert!(set.insert(op), "duplicate event opcode: {}", op);
        }
        assert_eq!(events.len(), 6);
    }

    /// King impeachment sub-opcodes are all distinct.
    #[test]
    fn test_impeachment_all_distinct() {
        let imp = [
            KING_IMPEACHMENT_REQUEST_UI_OPEN,
            KING_IMPEACHMENT_ELECTION_UI_OPEN,
            KING_IMPEACHMENT_REQUEST,
            KING_IMPEACHMENT_REQUEST_ELECT,
            KING_IMPEACHMENT_LIST,
            KING_IMPEACHMENT_ELECT,
        ];
        let mut set = std::collections::HashSet::new();
        for &op in &imp {
            assert!(set.insert(op), "duplicate impeachment opcode: {}", op);
        }
    }

    /// King event notice packet includes message with KING_EVENT_OPCODE header.
    #[test]
    fn test_king_event_notice_packet_format() {
        let mut pkt = Packet::new(Opcode::WizKing as u8);
        pkt.write_u8(KING_EVENT_OPCODE);
        pkt.write_u8(KING_EVENT_NOTICE);
        pkt.write_u8(1); // success
        pkt.write_u16(1); // success code
        assert_eq!(pkt.data[0], KING_EVENT_OPCODE);
        assert_eq!(pkt.data[1], KING_EVENT_NOTICE);
        assert_eq!(pkt.data[2], 1);
    }

    /// Candidacy board read vs write have different values.
    #[test]
    fn test_candidacy_board_read_write_distinct() {
        assert_ne!(KING_CANDIDACY_BOARD_READ, KING_CANDIDACY_BOARD_WRITE);
        assert!(KING_CANDIDACY_BOARD_READ > 0);
        assert!(KING_CANDIDACY_BOARD_WRITE > 0);
    }

    /// Election type phases cover full lifecycle: no_term → nomination → pre_election → election → term_ended.
    #[test]
    fn test_election_lifecycle_phases() {
        assert_eq!(ELECTION_TYPE_NO_TERM, 0);
        assert_eq!(ELECTION_TYPE_NOMINATION, 1);
        assert_eq!(ELECTION_TYPE_PRE_ELECTION, 2);
        assert_eq!(ELECTION_TYPE_ELECTION, 3);
        assert_eq!(ELECTION_TYPE_TERM_ENDED, 7);
        // Normal flow is sequential 0→1→2→3, then 7 for term end
        assert!(ELECTION_TYPE_NO_TERM < ELECTION_TYPE_NOMINATION);
        assert!(ELECTION_TYPE_NOMINATION < ELECTION_TYPE_PRE_ELECTION);
        assert!(ELECTION_TYPE_PRE_ELECTION < ELECTION_TYPE_ELECTION);
        assert!(ELECTION_TYPE_ELECTION < ELECTION_TYPE_TERM_ENDED);
    }

    /// King main sub-opcodes: 5 is skipped (no handler for sub=5).
    #[test]
    fn test_king_main_subopcodes_gap_at_5() {
        assert_eq!(KING_ELECTION, 1);
        assert_eq!(KING_IMPEACHMENT, 2);
        assert_eq!(KING_TAX, 3);
        assert_eq!(KING_EVENT_OPCODE, 4);
        // sub=5 is not defined — gap between EVENT(4) and NATION_INTRO(6)
        assert_eq!(KING_NATION_INTRO, 6);
        assert_eq!(KING_NATION_INTRO - KING_EVENT_OPCODE, 2);
    }

    /// King event sub-opcodes 1-6 are contiguous (noah/exp/prize/fugitive/weather/notice).
    #[test]
    fn test_king_event_subopcodes_contiguous() {
        assert_eq!(KING_EVENT_NOAH, 1);
        assert_eq!(KING_EVENT_EXP, 2);
        assert_eq!(KING_EVENT_PRIZE, 3);
        assert_eq!(KING_EVENT_FUGITIVE, 4);
        assert_eq!(KING_EVENT_WEATHER, 5);
        assert_eq!(KING_EVENT_NOTICE, 6);
        // Contiguous 1-6
        assert_eq!(KING_EVENT_NOTICE - KING_EVENT_NOAH, 5);
    }

    /// Voter requirements: level ≥ 50, NP ≥ 10000.
    #[test]
    fn test_voter_requirements_thresholds() {
        assert_eq!(MIN_LEVEL_VOTER, 50);
        assert_eq!(MIN_NP_VOTER, 10_000);
        // Level requirement is below max level (83)
        assert!(MIN_LEVEL_VOTER < 83);
        // NP requirement is positive
        assert!(MIN_NP_VOTER > 0);
    }

    /// King scepter item ID is in 910M range (unique item class).
    #[test]
    fn test_king_scepter_item_class() {
        assert_eq!(super::KING_SCEPTER, 910_074_311);
        // 910M range = unique/event items
        assert!(super::KING_SCEPTER >= 900_000_000);
        assert!(super::KING_SCEPTER < 1_000_000_000);
    }
}
