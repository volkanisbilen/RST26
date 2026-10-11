//! Full Moon Rift entry: a countdown registration through the client's temple-event window (the same
//! window Chaos Dungeon uses). When the countdown ends the registrants are split into groups of at
//! most 8 (per nation), each group gets an automatic party and its own rift room.

use std::sync::atomic::{AtomicI16, Ordering};
use std::sync::{Arc, OnceLock};

use ko_protocol::{Opcode, Packet};
use parking_lot::Mutex;

use super::dungeon_defence::{self as dd, DdDifficulty};
use crate::session::ClientSession;
use crate::systems::timed_notice::build_notice_packet;
use crate::world::WorldState;
use crate::zone::SessionId;

/// Temple-event id the registration window is opened with (`+riftid <n>` changes it live).
static EVENT_ID: AtomicI16 = AtomicI16::new(24);

pub fn event_id() -> i16 {
    EVENT_ID.load(Ordering::Relaxed)
}

pub fn set_event_id(id: i16) {
    EVENT_ID.store(id, Ordering::Relaxed);
}

const MIN_LEVEL: u8 = 70;
const GROUP_SIZE: usize = 8;
const DEFAULT_REG_MINUTES: u64 = 3;

const SUB_JOIN: u8 = 8;
const SUB_DISBAND: u8 = 9;
const SUB_COUNTER: u8 = 16;

struct State {
    open: bool,
    deadline: u64,
    registrants: Vec<(SessionId, String)>,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            open: false,
            deadline: 0,
            registrants: Vec::new(),
        })
    })
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn notice(world: &WorldState, sid: SessionId, msg: &str) {
    world.send_to_session_owned(sid, build_notice_packet(8, msg));
}

fn counter_packet(count: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(SUB_COUNTER);
    pkt.write_u16(event_id() as u16);
    pkt.write_u16(count);
    pkt
}

fn reply(op: u8, result: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(op);
    pkt.write_u8(result);
    pkt.write_i16(event_id());
    pkt
}

fn broadcast_count(world: &WorldState, st: &State) {
    let count = Arc::new(counter_packet(st.registrants.len().min(u16::MAX as usize) as u16));
    for sid in world.get_in_game_session_ids() {
        world.send_to_session_arc(sid, Arc::clone(&count));
    }
}

/// Is the Rift registration window currently open?
pub fn registration_open() -> bool {
    state().lock().open
}

fn is_gm(world: &WorldState, sid: SessionId) -> bool {
    world
        .get_character_info(sid)
        .is_some_and(|c| c.authority == 0 || c.authority == 2)
}

/// WIZ_EVENT join (Apply pressed).
pub async fn temple_join(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let result = match try_register(&world, sid) {
        Ok(()) => 1u8,
        Err(msg) => {
            notice(&world, sid, msg);
            4u8
        }
    };
    session.send_packet(&reply(SUB_JOIN, result)).await?;
    broadcast_count(&world, &state().lock());
    Ok(())
}

/// WIZ_EVENT disband (Cancel pressed).
pub async fn temple_disband(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    state().lock().registrants.retain(|(s, _)| *s != sid);
    session.send_packet(&reply(SUB_DISBAND, 1)).await?;
    broadcast_count(&world, &state().lock());
    Ok(())
}

fn try_register(world: &WorldState, sid: SessionId) -> Result<(), &'static str> {
    let Some(ch) = world.get_character_info(sid) else {
        return Err("Registration failed.");
    };
    let gm = is_gm(world, sid);
    let mut st = state().lock();
    if !st.open {
        return Err("Full Moon Rift registration is not open.");
    }
    if st.registrants.iter().any(|(s, _)| *s == sid) {
        return Err("You are already registered.");
    }
    if !gm {
        if ch.level < MIN_LEVEL {
            return Err("You need to be level 70 or higher.");
        }
        if world.is_player_dead(sid) {
            return Err("You cannot register while dead.");
        }
        if world.get_event_room(sid) != 0 {
            return Err("You are already in an event.");
        }
        if !world.check_exist_item(sid, dd::DUNGEON_DEFENCE_RIFT_ITEM, 1) {
            return Err("You need a Full Moon Rift Voucher.");
        }
    }
    st.registrants.push((sid, ch.name.clone()));
    Ok(())
}

// ── GM control ───────────────────────────────────────────────────────────

/// `+riftreg [minutes]`
pub fn gm_open(world: &WorldState, minutes: Option<u64>) -> &'static str {
    let mut st = state().lock();
    if st.open {
        return "Full Moon Rift registration is already open.";
    }
    let minutes = minutes.unwrap_or(DEFAULT_REG_MINUTES).clamp(1, 60);
    st.open = true;
    st.deadline = now_unix() + minutes * 60;
    st.registrants.clear();
    let open = Arc::new(crate::systems::event_room::build_event_start_broadcast(
        event_id(),
        (minutes * 60).min(u16::MAX as u64) as u16,
    ));
    let count = Arc::new(counter_packet(0));
    for sid in world.get_in_game_session_ids() {
        world.send_to_session_arc(sid, Arc::clone(&open));
        world.send_to_session_arc(sid, Arc::clone(&count));
    }
    drop(st);
    world.broadcast_to_all(
        Arc::new(build_notice_packet(
            8,
            &format!("Full Moon Rift registration is open for {minutes} minutes. Apply in the event window (Full Moon Rift Voucher, level 70+)."),
        )),
        None,
    );
    "Full Moon Rift registration opened."
}

/// `+riftstart`: end the registration now.
pub fn gm_start(world: &WorldState) -> &'static str {
    if !state().lock().open {
        return "Full Moon Rift registration is not open.";
    }
    start(world);
    "Full Moon Rift is starting."
}

/// `+riftend`: cancel the registration.
pub fn gm_cancel(world: &WorldState) -> &'static str {
    let mut st = state().lock();
    if !st.open {
        return "Full Moon Rift registration is not open.";
    }
    st.open = false;
    st.registrants.clear();
    drop(st);
    world.broadcast_to_all(
        Arc::new(build_notice_packet(8, "Full Moon Rift registration was cancelled.")),
        None,
    );
    "Full Moon Rift registration cancelled."
}

/// Called every second: the countdown ends the registration.
pub fn tick(world: &WorldState) {
    let due = {
        let st = state().lock();
        st.open && now_unix() >= st.deadline
    };
    if due {
        start(world);
    }
}

// ── Room setup ───────────────────────────────────────────────────────────

fn start(world: &WorldState) {
    let registrants = {
        let mut st = state().lock();
        st.open = false;
        std::mem::take(&mut st.registrants)
    };

    // re-validate and split by nation
    let mut by_nation: [Vec<SessionId>; 2] = [Vec::new(), Vec::new()];
    for (sid, name) in registrants {
        let Some(ch) = world.get_character_info(sid) else { continue };
        if ch.name != name || world.is_player_dead(sid) || world.get_event_room(sid) != 0 {
            continue;
        }
        let gm = is_gm(world, sid);
        if !gm && (ch.level < MIN_LEVEL || !world.check_exist_item(sid, dd::DUNGEON_DEFENCE_RIFT_ITEM, 1)) {
            continue;
        }
        by_nation[(ch.nation.max(1) as usize - 1).min(1)].push(sid);
    }

    let mut started = 0;
    for list in by_nation {
        for group in split_groups(list) {
            let any_gm = group.iter().any(|&s| is_gm(world, s));
            if group.len() < 2 && !any_gm {
                for &sid in &group {
                    notice(world, sid, "Full Moon Rift: not enough players in your group. Your voucher was not used.");
                }
                continue;
            }
            let difficulty = DdDifficulty::from_party_size(group.len() as u16).unwrap_or(DdDifficulty::Easy);
            let Some(_room_id) = dd::open_room(world, &group, difficulty) else {
                for &sid in &group {
                    notice(world, sid, "Full Moon Rift: no free room. Your voucher was not used.");
                }
                continue;
            };
            form_party(world, &group);
            for &sid in &group {
                if !is_gm(world, sid) {
                    world.rob_item(sid, dd::DUNGEON_DEFENCE_RIFT_ITEM, 1);
                }
                notice(
                    world,
                    sid,
                    &format!(
                        "Welcome to the Full Moon Rift ({} waves). The first wave arrives in {} seconds.",
                        difficulty.max_stages(),
                        dd::DD_INITIAL_SPAWN_TIME
                    ),
                );
            }
            started += 1;
        }
    }
    if started == 0 {
        world.broadcast_to_all(
            Arc::new(build_notice_packet(8, "Full Moon Rift was cancelled: not enough players registered.")),
            None,
        );
    }
}

/// Even groups of at most 8 (9 players become 5 + 4, 20 become 7 + 7 + 6).
fn split_groups(list: Vec<SessionId>) -> Vec<Vec<SessionId>> {
    if list.is_empty() {
        return Vec::new();
    }
    let n = list.len().div_ceil(GROUP_SIZE);
    let (base, extra) = (list.len() / n, list.len() % n);
    let mut it = list.into_iter();
    (0..n)
        .map(|i| it.by_ref().take(base + usize::from(i < extra)).collect())
        .collect()
}

/// Put the group into one party (leader = first member) and show everybody to everybody.
fn form_party(world: &WorldState, group: &[SessionId]) {
    let Some(&leader) = group.first() else { return };
    for &sid in group {
        if let Some(pid) = world.get_party_id(sid) {
            world.remove_party_member(pid, sid);
        }
    }
    let Some(party_id) = world.create_party(leader) else { return };
    for &sid in &group[1..] {
        world.add_party_member(party_id, sid);
    }
    let target_number_id = world.get_party(party_id).map(|p| p.target_number_id).unwrap_or(0);
    for &sid in group {
        world.update_session(sid, |h| h.party_type = 2);
    }
    for &to in group {
        for &about in group {
            if let Some(ch) = world.get_character_info(about) {
                let pkt = super::party::build_party_member_info(&ch, 1, target_number_id, 2, -1);
                world.send_to_session_owned(to, pkt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sizes(n: u16) -> Vec<usize> {
        split_groups((1..=n).collect()).iter().map(|g| g.len()).collect()
    }

    #[test]
    fn even_groups_of_at_most_eight() {
        assert_eq!(sizes(8), vec![8]);
        assert_eq!(sizes(9), vec![5, 4]);
        assert_eq!(sizes(20), vec![7, 7, 6]);
        assert_eq!(sizes(3), vec![3]);
        assert!(sizes(0).is_empty());
    }
}
