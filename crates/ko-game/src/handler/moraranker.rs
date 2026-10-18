//! WIZ_RANKER (0xD9) — native v2615 MORANKER memo UI.
//!
//! C2S:
//! - `[01][ranker_type]` open statue
//! - `[02][ranker_type][u16 length][memo]` save own memo
//! - `[03][ranker_type]` native talk acknowledgement
//!
//! S2C open success:
//! - `[01][i16 1][string owner][string memo]`

use ko_db::repositories::moraranker::MorankerRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{info, warn};

use crate::session::{ClientSession, SessionState};
use crate::world::moraranker::{MORANKER_TYPE_FIRST, MORANKER_TYPE_LAST};

const OPEN: u8 = 1;
const SAVE_MEMO: u8 = 2;
const TALK_ACK: u8 = 3;
const RESULT_OK: i16 = 1;
const RESULT_NOT_RANKER: i16 = -1;
const RESULT_INVALID: i16 = -2;
const MAX_MEMO_BYTES: usize = 500;

fn valid_type(value: u8) -> bool {
    (MORANKER_TYPE_FIRST..=MORANKER_TYPE_LAST).contains(&value)
}

async fn send_result(session: &mut ClientSession, sub: u8, result: i16) -> anyhow::Result<()> {
    let mut response = Packet::new(Opcode::WizRanker as u8);
    response.write_u8(sub);
    response.write_i16(result);
    session.send_packet(&response).await?;
    Ok(())
}

pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let Some(sub) = reader.read_u8() else {
        return Ok(());
    };
    let Some(ranker_type) = reader.read_u8() else {
        return Ok(());
    };
    if !valid_type(ranker_type) {
        return send_result(session, sub, RESULT_INVALID).await;
    }

    let Some(owner) = session.world().moraranker_owner(ranker_type) else {
        return send_result(session, sub, RESULT_NOT_RANKER).await;
    };
    let pool = session.pool().clone();
    let repository = MorankerRepository::new(&pool);

    match sub {
        OPEN => {
            let memo = repository.load_memo(&owner).await?.unwrap_or_default();
            let mut response = Packet::new(Opcode::WizRanker as u8);
            response.write_u8(OPEN);
            response.write_i16(RESULT_OK);
            response.write_string(&owner);
            response.write_string(&memo);
            session.send_packet(&response).await?;
            info!(
                viewer = session.character_id().unwrap_or_default(),
                ranker_type = %(ranker_type as char),
                owner = %owner,
                "MORANKER memo opened"
            );
        }
        SAVE_MEMO => {
            let Some(memo) = reader.read_string() else {
                return send_result(session, SAVE_MEMO, RESULT_INVALID).await;
            };
            let is_owner = session
                .character_id()
                .is_some_and(|name| name.eq_ignore_ascii_case(&owner));
            if !is_owner {
                warn!(
                    character = session.character_id().unwrap_or_default(),
                    owner = %owner,
                    "rejected MORANKER memo update from non-owner"
                );
                return send_result(session, SAVE_MEMO, RESULT_NOT_RANKER).await;
            }
            if memo.as_bytes().len() > MAX_MEMO_BYTES {
                return send_result(session, SAVE_MEMO, RESULT_INVALID).await;
            }
            let updated = repository.update_memo(&owner, &memo).await?;
            send_result(
                session,
                SAVE_MEMO,
                if updated { RESULT_OK } else { RESULT_INVALID },
            )
            .await?;
            if updated {
                info!(owner = %owner, memo_len = memo.len(), "MORANKER memo updated");
            }
        }
        TALK_ACK => {
            // This button only requests the native acknowledgement message; the
            // memo itself was already delivered by OPEN.
            send_result(session, TALK_ACK, RESULT_OK).await?;
        }
        _ => return send_result(session, sub, RESULT_INVALID).await,
    }

    Ok(())
}
