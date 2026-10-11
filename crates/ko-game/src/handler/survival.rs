//! Manes Survival 2615 client protocol.
//!
//! Byte contract verified against the unpacked client CSurvival dispatcher
//! (`sub_716A10`):
//! - S2C D0 01 01 u16 remaining_seconds u16 participant_count: open/refresh entry UI
//! - C2S D0 01 02 u16 action: 1 apply, 2 cancel
//! - S2C D0 01 02 i16 result [u16 participant_count when result=1]
//! - S2C D0 06 01 u8 first_count [u16 skill_id] u8 second_count
//!   [u16 skill_id]: load `MANES_MAGIC.tbl` rows and open the skill-choice UI
//! - C2S D0 06 02 u8 list_type u16 manes_magic_id: submit a selection
//!   (`list_type=1` skill, `list_type=2` potion)
//! - S2C D0 06 02 u8 list_type i16 result: complete that selection (`result=1`)
//! - S2C D0 06 04 u8 count [u16 skill_id]: refresh only the skill list
//! - C2S D0 06 05 u8 state [u16 skill_id when state != 2]: complete selection
//! - S2C D0 06 05 i16 result: accept selection and close the choice UI
//!
//! The similarly shaped 0xD3 handler belongs to the Ronark-war UI and must not
//! be used for Manes Survival registration.

use std::sync::Arc;

use ko_protocol::{Packet, PacketReader};
use tracing::{debug, warn};

use crate::session::{ClientSession, SessionState};

const WIZ_SURVIVAL: u8 = 0xD0;
const CATEGORY_REGISTRATION: u8 = 1;
const REG_OPEN: u8 = 1;
const REG_APPLY: u8 = 2;
const ACTION_APPLY: u16 = 1;
const ACTION_CANCEL: u16 = 2;
const CATEGORY_EVENT: u8 = 2;
const EVENT_START: u8 = 1;
const EVENT_BOSS_STATUS: u8 = 2;
const EVENT_SCORE: u8 = 3;
const EVENT_FINISH_RESTORE: u8 = 4;
const CATEGORY_STATUS: u8 = 4;
const STATUS_EXP: u8 = 1;
const STATUS_SCORE: u8 = 3;
const CATEGORY_SKILL: u8 = 6;
const SKILL_OPEN: u8 = 1;
const SELECTION_SUBMIT: u8 = 2;
const SKILL_COMPLETE: u8 = 5;
const SKILL_RESULT_FAILURE: u8 = 0;
const SELECTION_LIST_SKILL: u8 = 1;
const SELECTION_LIST_POTION: u8 = 2;
pub const REGISTRATION_DURATION_SECONDS: u16 = 600;
pub const EVENT_DURATION_SECONDS: u16 = 1_200;

pub fn build_registration_open(remaining_seconds: u16, participant_count: u16) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_REGISTRATION);
    pkt.write_u8(REG_OPEN);
    pkt.write_u16(remaining_seconds);
    pkt.write_u16(participant_count);
    pkt
}

pub fn build_registration_result(result: i16, participant_count: u16) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_REGISTRATION);
    pkt.write_u8(REG_APPLY);
    pkt.write_u16(result as u16);
    if result == 1 {
        pkt.write_u16(participant_count);
    }
    pkt
}

/// Initialise the v2615 Manes Survival client state.
///
/// Verified against `sub_716A10 -> sub_7113D0`, operation 1:
/// `D0 02 01 u8 survival_setting u16 seconds u16 max_exp u16 exp u8 level`.
///
/// The client stores the first value as the level EXP requirement and the
/// second value as the current Survival EXP. Sending them in the opposite
/// order makes the temporary level HUD repeatedly rebuild itself with an
/// impossible `current > maximum` state.
pub fn build_event_start(
    survival_setting: u8,
    remaining_seconds: u16,
    survival_exp: u16,
    survival_max_exp: u16,
    survival_level: u8,
) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_EVENT);
    pkt.write_u8(EVENT_START);
    pkt.write_u8(survival_setting);
    pkt.write_u16(remaining_seconds);
    pkt.write_u16(survival_max_exp);
    pkt.write_u16(survival_exp);
    pkt.write_u8(survival_level);
    pkt
}

/// Update current Manes EXP without rebuilding level, HP, MP or the temporary
/// Survival inventory.
///
/// Verified against `sub_716A10 -> sub_713040`, category 4 operation 1:
/// `D0 04 01 u8 result u16 current_exp`.
pub fn build_event_exp_update(current_exp: u16) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_STATUS);
    pkt.write_u8(STATUS_EXP);
    pkt.write_u8(1);
    pkt.write_u16(current_exp);
    pkt
}

/// Update the center Red Dragon HP/timer panel.
///
/// Verified against `sub_716A10 -> sub_7113D0`, category 2 operation 2:
/// `D0 02 02 string boss_name u32 max_hp u32 current_hp u32 seconds`.
pub fn build_event_boss_status(
    boss_name: &str,
    max_hp: u32,
    current_hp: u32,
    remaining_seconds: u32,
) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_EVENT);
    pkt.write_u8(EVENT_BOSS_STATUS);
    pkt.write_string(boss_name);
    pkt.write_u32(max_hp);
    pkt.write_u32(current_hp);
    pkt.write_u32(remaining_seconds);
    pkt
}

/// Update the score label in the center Survival panel.
///
/// Verified against `sub_716A10 -> sub_713040`, category 4 operation 3:
/// `D0 04 03 u32 score`.
pub fn build_event_score_update(score: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_STATUS);
    pkt.write_u8(STATUS_SCORE);
    pkt.write_u32(score);
    pkt
}

/// Restore the normal character level/EXP and the 28 ordinary bag slots when
/// Manes finishes.
///
/// Verified against `sub_716A10 -> sub_7113D0`, category 2 operation 4. The
/// client disables Survival mode, restores the persistent character values,
/// reads exactly 28 regular inventory slots through `sub_85F250`, and replaces
/// the temporary Manes bag before the zone change completes.
pub async fn build_event_finish_restore(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
) -> Option<Packet> {
    use crate::inventory_constants::{HAVE_MAX, SLOT_MAX};

    let character = world.get_character_info(sid)?;
    let inventory = world.get_persistent_inventory(sid);
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_EVENT);
    pkt.write_u8(EVENT_FINISH_RESTORE);
    pkt.write_i64(character.max_exp);
    pkt.write_i64(character.exp as i64);
    pkt.write_u8(character.level);

    for index in SLOT_MAX..SLOT_MAX + HAVE_MAX {
        let slot = inventory.get(index).cloned().unwrap_or_default();
        pkt.write_u32(slot.item_id);
        pkt.write_i16(slot.durability);
        pkt.write_u16(slot.count);
        pkt.write_u8(slot.flag);
        pkt.write_u16(slot.remaining_rental_minutes());
        if let Some(pool) = world.db_pool() {
            crate::handler::unique_item_info::write_unique_item_info(
                world,
                pool,
                slot.item_id,
                slot.serial_num,
                character.rebirth_level,
                &mut pkt,
            )
            .await;
        } else {
            pkt.write_u32(0);
        }
        pkt.write_u32(slot.expire_time);
    }

    Some(pkt)
}

/// Open the v2615 Manes skill-choice window.
///
/// Verified against `sub_716A10 -> sub_714AB0`, operation 1. The client reads
/// two counted lists of `u16` MANES_MAGIC identifiers, resolves the rows via
/// `sub_710140`, then calls `sub_5037D0 -> sub_75C520` to populate and show the
/// choice UI. Names, descriptions, and icons are client table data and are not
/// part of this packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManesRankEntry {
    pub nation: u8,
    pub class: u16,
    pub clan_name: String,
    pub character_name: String,
    pub level: u16,
    pub score: u32,
}

/// Build the v2615 ALT / My Score response.
///
/// Verified against the unpacked client:
/// `sub_716A10 -> sub_7113D0`, category 2 operation 3, delegates to
/// `CUISurvivalRank::sub_751500`. The client consumes:
///
/// `u8 result, u16 my_rank, u32 my_score, u8 count`
///
/// followed by at most 20 rows:
///
/// `u8 nation, u16 class, string clan_name, string character_name,
///  u16 survival_level, u32 score`.
pub fn build_event_score(my_score: u32, my_rank: u16, entries: &[ManesRankEntry]) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_EVENT);
    pkt.write_u8(EVENT_SCORE);
    pkt.write_u8(1);
    pkt.write_u16(my_rank);
    pkt.write_u32(my_score);
    pkt.write_u8(entries.len().min(20) as u8);
    for entry in entries.iter().take(20) {
        pkt.write_u8(entry.nation);
        pkt.write_u16(entry.class);
        pkt.write_string(&entry.clan_name);
        pkt.write_string(&entry.character_name);
        pkt.write_u16(entry.level);
        pkt.write_u32(entry.score);
    }
    pkt
}

pub fn build_skill_selection_open(skill_choices: &[u16], potion_choices: &[u16]) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_SKILL);
    pkt.write_u8(SKILL_OPEN);
    pkt.write_u8(skill_choices.len() as u8);
    for skill_id in skill_choices {
        pkt.write_u16(*skill_id);
    }
    pkt.write_u8(potion_choices.len() as u8);
    for potion_id in potion_choices {
        pkt.write_u16(*potion_id);
    }
    pkt
}

/// Confirm a v2615 Manes skill selection.
///
/// `sub_714AB0`, operation 5, reads a signed 16-bit result code. Returning a
/// MANES_MAGIC row ID here is interpreted as an error and leaves Complete in a
/// retry loop.
pub fn build_skill_selection_result(result: i16) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_SKILL);
    pkt.write_u8(SKILL_COMPLETE);
    pkt.write_u16(result as u16);
    pkt
}

/// Complete a Manes skill/potion list selection.
///
/// Verified against `sub_714AB0`, operation 2. The client reads the selection
/// list type followed by a signed result code. Result 1 commits the selected
/// `MANES_MAGIC` row in the corresponding skill or potion UI.
pub fn build_selection_submit_result(list_type: u8, result: i16) -> Packet {
    let mut pkt = Packet::new(WIZ_SURVIVAL);
    pkt.write_u8(CATEGORY_SKILL);
    pkt.write_u8(SELECTION_SUBMIT);
    pkt.write_u8(list_type);
    pkt.write_u16(result as u16);
    pkt
}

pub fn broadcast_registration_status(world: &crate::world::WorldState, elapsed_seconds: u32) {
    let remaining =
        REGISTRATION_DURATION_SECONDS.saturating_sub(elapsed_seconds.min(u16::MAX as u32) as u16);
    let participants = world
        .manes_survival_manager
        .participant_count()
        .min(u16::MAX as usize) as u16;
    let packet = Arc::new(build_registration_open(remaining, participants));
    for sid in world.get_in_game_session_ids() {
        world.send_to_session_arc(sid, Arc::clone(&packet));
    }
}

pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let category = reader.read_u8().unwrap_or(0);
    let operation = reader.read_u8().unwrap_or(0);

    if category == CATEGORY_EVENT && operation == EVENT_SCORE {
        let world = session.world();
        let manager = &world.manes_survival_manager;
        let mut ranked: Vec<_> = manager
            .participant_ids()
            .into_iter()
            .filter_map(|sid| {
                let progress = manager.progress(sid)?;
                let character = world.get_character_info(sid)?;
                let score = u32::from(crate::systems::manes_survival::score_for_level(
                    progress.level,
                ));
                let clan_name = world
                    .get_knights(character.knights_id)
                    .map(|clan| clan.name)
                    .unwrap_or_default();
                Some((
                    sid,
                    progress.level,
                    score,
                    ManesRankEntry {
                        nation: character.nation,
                        class: character.class,
                        clan_name,
                        character_name: character.name.clone(),
                        level: u16::from(progress.level),
                        score,
                    },
                ))
            })
            .collect();
        ranked.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.0.cmp(&right.0))
        });

        let rank = ranked
            .iter()
            .position(|entry| entry.0 == session.session_id())
            .map(|index| u16::try_from(index + 1).unwrap_or(u16::MAX))
            .unwrap_or(0);
        let score = ranked
            .iter()
            .find(|entry| entry.0 == session.session_id())
            .map(|entry| entry.2)
            .unwrap_or(0);
        let entries: Vec<_> = ranked.into_iter().take(20).map(|entry| entry.3).collect();
        session
            .send_packet(&build_event_score(score, rank, &entries))
            .await?;
        debug!(
            "[{}] Manes My Score requested sid={} score={} rank={} entries={}",
            session.addr(),
            session.session_id(),
            score,
            rank,
            entries.len()
        );
        return Ok(());
    }

    if category == CATEGORY_SKILL && operation == SELECTION_SUBMIT {
        let list_type = reader.read_u8().unwrap_or(SKILL_RESULT_FAILURE);
        let manes_magic_id = reader.read_u16().unwrap_or(0);
        let no_trailing_data = reader.remaining() == 0;

        // The main skill list uses the same operation as the potion list.
        // The v2615 client commits the selected MANES_MAGIC row to its skill
        // bar only after receiving `D0 06 02 01 01 00`.
        if list_type == SELECTION_LIST_SKILL {
            let valid = no_trailing_data
                && session
                    .world()
                    .manes_survival_manager
                    .offered_skill(session.session_id(), manes_magic_id);
            session
                .send_packet(&build_selection_submit_result(
                    list_type,
                    if valid { 1 } else { 0 },
                ))
                .await?;
            if valid {
                let magic_id = session
                    .world()
                    .manes_survival_manager
                    .unlock_magic(session.session_id(), manes_magic_id)
                    .expect("validated MANES_MAGIC row must map to MAGIC");
                if let Some(progress) = session
                    .world()
                    .manes_survival_manager
                    .progress(session.session_id())
                {
                    crate::handler::attack::sync_manes_vitals_and_level(
                        session.world(),
                        session.session_id(),
                        progress,
                        false,
                    );
                }
                debug!(
                    "[{}] Manes skill list selection accepted sid={} manes_magic_id={} magic_id={}",
                    session.addr(),
                    session.session_id(),
                    manes_magic_id,
                    magic_id
                );
            } else {
                warn!(
                    "[{}] Manes skill list selection rejected sid={} list_type={} manes_magic_id={} remaining={}",
                    session.addr(),
                    session.session_id(),
                    list_type,
                    manes_magic_id,
                    reader.remaining()
                );
            }
            return Ok(());
        }

        let sid = session.session_id();
        let world = session.world().clone();
        let purchase = world.manes_survival_manager.potion_purchase(manes_magic_id);
        let valid_request = list_type == SELECTION_LIST_POTION
            && no_trailing_data
            && world
                .manes_survival_manager
                .offered_potion(sid, manes_magic_id)
            && purchase.is_some();
        let (item_id, item_count, price) = purchase.unwrap_or_default();
        let enough_gold = world
            .get_character_info(sid)
            .map(|character| character.gold >= price)
            .unwrap_or(false);

        if valid_request
            && enough_gold
            && world.check_weight(sid, item_id, item_count)
            && world.gold_lose(sid, price)
        {
            if world.give_item(sid, item_id, item_count) {
                session
                    .send_packet(&build_selection_submit_result(list_type, 1))
                    .await?;
                debug!(
                    "[{}] Manes potion purchased sid={} manes_magic_id={} item_id={} count={} price={}",
                    session.addr(),
                    sid,
                    manes_magic_id,
                    item_id,
                    item_count,
                    price
                );
            } else {
                world.gold_gain(sid, price);
                session
                    .send_packet(&build_selection_submit_result(list_type, 0))
                    .await?;
                warn!(
                    "[{}] Manes potion purchase rolled back sid={} manes_magic_id={} item_id={}",
                    session.addr(),
                    sid,
                    manes_magic_id,
                    item_id
                );
            }
        } else {
            session
                .send_packet(&build_selection_submit_result(list_type, 0))
                .await?;
            warn!(
                "[{}] Manes potion purchase rejected sid={} list_type={} manes_magic_id={} remaining={} enough_gold={}",
                session.addr(),
                sid,
                list_type,
                manes_magic_id,
                reader.remaining(),
                enough_gold
            );
        }
        return Ok(());
    }

    if category == CATEGORY_SKILL && operation == SKILL_COMPLETE {
        let requested_state = reader.read_u8().unwrap_or(SKILL_RESULT_FAILURE);
        let skill_id = reader.read_u16().unwrap_or(0);
        let valid = requested_state != SKILL_RESULT_FAILURE
            && reader.remaining() == 0
            && session
                .world()
                .manes_survival_manager
                .offered_skill(session.session_id(), skill_id);
        if valid {
            session
                .send_packet(&build_skill_selection_result(1))
                .await?;
            debug!(
                "[{}] Manes skill selection completed sid={} skill_id={}",
                session.addr(),
                session.session_id(),
                skill_id
            );
        } else {
            warn!(
                "[{}] Manes skill selection rejected sid={} state={} skill_id={} remaining={}",
                session.addr(),
                session.session_id(),
                requested_state,
                skill_id,
                reader.remaining()
            );
        }
        return Ok(());
    }

    if category != CATEGORY_REGISTRATION || operation != REG_APPLY {
        debug!(
            "[{}] WIZ_SURVIVAL unsupported C2S category={} operation={} remaining={}",
            session.addr(),
            category,
            operation,
            reader.remaining()
        );
        return Ok(());
    }

    let action = reader.read_u16().unwrap_or(0);
    let sid = session.session_id();
    let world = session.world().clone();
    let manager = &world.manes_survival_manager;

    match action {
        ACTION_APPLY => {
            let registered = manager.register(sid);
            let participants = manager.participant_count().min(u16::MAX as usize) as u16;
            let result = if registered { 1 } else { -1 };
            // Refresh the participant count first. The apply result must be
            // the final registration packet seen by this client; otherwise
            // REG_OPEN immediately resets the button from Cancel to Apply.
            broadcast_registration_status(&world, 0);
            session
                .send_packet(&build_registration_result(result, participants))
                .await?;
            debug!(
                "[{}] Manes registration apply sid={} result={} participants={}",
                session.addr(),
                sid,
                result,
                manager.participant_count()
            );
        }
        ACTION_CANCEL => {
            manager.unregister(sid);
            let participants = manager.participant_count().min(u16::MAX as usize) as u16;
            // As with apply, leave the action result last so the client keeps
            // the correct Apply/Cancel button state after the count refresh.
            broadcast_registration_status(&world, 0);
            session
                .send_packet(&build_registration_result(2, participants))
                .await?;
            debug!(
                "[{}] Manes registration cancel sid={} participants={}",
                session.addr(),
                sid,
                manager.participant_count()
            );
        }
        _ => warn!(
            "[{}] WIZ_SURVIVAL invalid registration action={}",
            session.addr(),
            action
        ),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_start_matches_v2615_client_contract() {
        let packet = build_event_start(3, 1_200, 0, 200, 1);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(
            packet.data,
            vec![0x02, 0x01, 0x03, 0xB0, 0x04, 0xC8, 0x00, 0x00, 0x00, 0x01]
        );
    }

    #[test]
    fn event_exp_update_does_not_rebuild_vitals() {
        let packet = build_event_exp_update(380);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(packet.data, vec![0x04, 0x01, 0x01, 0x7C, 0x01]);
    }

    #[test]
    fn event_boss_status_matches_v2615_client_contract() {
        let packet = build_event_boss_status("red dragon", 500_000, 450_000, 516);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(
            packet.data,
            vec![
                0x02, 0x02, 0x0A, 0x00, b'r', b'e', b'd', b' ', b'd', b'r', b'a', b'g', b'o', b'n',
                0x20, 0xA1, 0x07, 0x00, 0xD0, 0xDD, 0x06, 0x00, 0x04, 0x02, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn event_score_update_matches_v2615_client_contract() {
        let packet = build_event_score_update(180);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(packet.data, vec![0x04, 0x03, 0xB4, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn event_score_matches_alt_my_score_contract() {
        let packet = build_event_score(
            180,
            1,
            &[ManesRankEntry {
                nation: 2,
                class: 205,
                clan_name: "JstKO".to_string(),
                character_name: "JstVK".to_string(),
                level: 10,
                score: 180,
            }],
        );
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(
            packet.data,
            vec![
                0x02, 0x03, 0x01, 0x01, 0x00, 0xB4, 0x00, 0x00, 0x00, 0x01, 0x02, 0xCD, 0x00, 0x05,
                0x00, b'J', b's', b't', b'K', b'O', 0x05, 0x00, b'J', b's', b't', b'V', b'K', 0x0A,
                0x00, 0xB4, 0x00, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn skill_selection_result_matches_v2615_client_contract() {
        let packet = build_skill_selection_result(1);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(packet.data, vec![0x06, 0x05, 0x01, 0x00]);
    }

    #[test]
    fn skill_selection_submit_result_matches_v2615_client_contract() {
        let packet = build_selection_submit_result(SELECTION_LIST_SKILL, 1);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(packet.data, vec![0x06, 0x02, 0x01, 0x01, 0x00]);
    }

    #[test]
    fn potion_selection_submit_result_matches_v2615_client_contract() {
        let packet = build_selection_submit_result(SELECTION_LIST_POTION, 1);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(packet.data, vec![0x06, 0x02, 0x02, 0x01, 0x00]);
    }

    #[test]
    fn skill_selection_open_matches_v2615_client_contract() {
        let packet = build_skill_selection_open(&[301, 401, 5901], &[6201, 6501]);
        assert_eq!(packet.opcode, 0xD0);
        assert_eq!(
            packet.data,
            vec![
                0x06, 0x01, 0x03, 0x2D, 0x01, 0x91, 0x01, 0x0D, 0x17, 0x02, 0x39, 0x18, 0x65, 0x19
            ]
        );
    }

    #[test]
    fn every_level_open_contains_skill_and_potion_lists() {
        let packet = build_skill_selection_open(&[302, 401, 6001], &[6401, 6701]);
        assert_eq!(packet.data[0..3], [0x06, 0x01, 0x03]);
        assert_eq!(packet.data[9], 0x02);
    }
}
