//! Fire & Ice Golem war (Ronark Land Bowl).
//!
//! 50 El Morad Flame Golems and 50 Karus Ice Golems stand around the Bowl. A nation can only hurt the
//! other nation's golems; a golem that dies is re-spawned for the nation that killed it (a conversion,
//! the total stays 100). Every conversion is one point. After 10 minutes - or as soon as a nation
//! reaches 100 points - the nation with the higher score wins.
//!
//! GM: `+golemopen`, `+golemend`, `+golemauto on|off` (14:00 and 22:00 server time).
//! Client identities: mob_us.tbl 9929 "Flame Golem" / 9930 "Ice Golem".

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;
use rand::seq::SliceRandom;
use rand::Rng;
use tracing::info;

use crate::systems::timed_notice::build_notice_packet;
use crate::world::WorldState;
use crate::zone::SessionId;

pub const ZONE_RONARK: u16 = 71;
const PROTO_FLAME: u16 = 9929; // El Morad
const PROTO_ICE: u16 = 9930; // Karus
const PER_NATION: usize = 50;
const TOTAL: u32 = 100;
const DURATION_SECS: u64 = 600;
const SCORE_ANNOUNCE_SECS: u64 = 60;
const AUTO_HOURS: [u32; 2] = [14, 22];

const ITEM_DUAL_POTION: u32 = 811_182_000;
const ITEM_HEROS_VOW: u32 = 811_065_000;
const ITEM_EVENT_EMBLEM: u32 = 1_900_004_000;
const ITEM_EMBLEM_VOUCHER_30D: u32 = 931_734_000;
const ITEM_LUNAR_WAR_BELT: u32 = 340_710_000;

static AUTO: AtomicBool = AtomicBool::new(false);

struct State {
    active: bool,
    started: u64,
    next_announce: u64,
    /// [karus, elmorad] conversions
    score: [u32; 2],
    /// live golems: nid -> owner nation (1 Karus, 2 El Morad)
    golems: HashMap<u32, u8>,
    /// everybody who killed a golem: sid -> nation
    killers: HashMap<SessionId, u8>,
    last_auto_day_hour: (u64, u32),
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            active: false,
            started: 0,
            next_announce: 0,
            score: [0; 2],
            golems: HashMap::new(),
            killers: HashMap::new(),
            last_auto_day_hour: (0, 99),
        })
    })
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn announce(world: &WorldState, msg: &str) {
    world.broadcast_to_all(Arc::new(build_notice_packet(8, msg)), None);
}

fn proto_for(nation: u8) -> u16 {
    if nation == 2 { PROTO_FLAME } else { PROTO_ICE }
}

fn nation_name(nation: u8) -> &'static str {
    if nation == 1 { "Karus" } else { "El Morad" }
}

/// Server hour/day used by the optional schedule.
fn day_and_hour() -> (u64, u32) {
    use chrono::Timelike;
    let now = chrono::Local::now();
    (now.timestamp() as u64 / 86_400, now.hour())
}

pub fn set_auto(on: bool) {
    AUTO.store(on, Ordering::Relaxed);
}

pub fn is_active() -> bool {
    state().lock().active
}

/// May `attacker` hurt this golem? A nation can only attack the other nation's golems.
pub fn can_attack(world: &WorldState, attacker_sid: SessionId, npc_nid: u32) -> bool {
    let st = state().lock();
    let Some(&owner) = st.golems.get(&npc_nid) else {
        return true;
    };
    drop(st);
    world
        .get_character_info(attacker_sid)
        .is_some_and(|c| c.nation != owner)
}

/// Is this NPC one of the event golems?
pub fn is_golem(npc_nid: u32) -> bool {
    state().lock().golems.contains_key(&npc_nid)
}

// ── Start / end ──────────────────────────────────────────────────────────

pub fn gm_open(world: &WorldState) -> &'static str {
    let mut st = state().lock();
    if st.active {
        return "The Fire & Ice event is already running.";
    }
    let spots: Vec<(f32, f32)> = (0..PER_NATION * 2)
        .filter_map(|_| world.get_start_position_random(ZONE_RONARK))
        .collect();
    if spots.len() < PER_NATION * 2 {
        return "Ronark Land has no spawn points.";
    }
    st.golems.clear();
    st.killers.clear();
    st.score = [0; 2];
    let mut order: Vec<usize> = (0..spots.len()).collect();
    order.shuffle(&mut rand::thread_rng());
    for (n, idx) in order.into_iter().enumerate() {
        let nation = if n % 2 == 0 { 2 } else { 1 };
        let (x, z) = spots[idx];
        if let Some(&nid) = world.spawn_event_npc(proto_for(nation), true, ZONE_RONARK, x, z, 1).first() {
            st.golems.insert(nid, nation);
        }
    }
    if st.golems.is_empty() {
        return "The golem templates are missing (run the migration).";
    }
    let now = now_unix();
    st.active = true;
    st.started = now;
    st.next_announce = now + SCORE_ANNOUNCE_SECS;
    drop(st);
    announce(
        world,
        "Fire & Ice has begun in Ronark Land! Convert the enemy nation's golems - El Morad hunts Ice Golems, Karus hunts Flame Golems. 10 minutes!",
    );
    info!("Fire & Ice golem war started");
    "The Fire & Ice event started."
}

pub fn gm_end(world: &WorldState) -> &'static str {
    let mut st = state().lock();
    if !st.active {
        return "The Fire & Ice event is not running.";
    }
    finish(world, &mut st);
    "The Fire & Ice event ended."
}

// ── Death hook ───────────────────────────────────────────────────────────

/// Called from the NPC death path: converts a killed golem to the killer's nation.
pub fn on_npc_death(world: &WorldState, killer_sid: SessionId, npc: &crate::npc::NpcInstance) {
    let mut st = state().lock();
    if !st.active || st.golems.remove(&npc.nid).is_none() {
        return;
    }
    let Some(killer) = world.get_character_info(killer_sid) else { return };
    let nation = killer.nation;
    if nation != 1 && nation != 2 {
        return;
    }

    // everybody in the party counts as a participant
    st.killers.insert(killer_sid, nation);
    if let Some(party_id) = world.get_party_id(killer_sid) {
        if let Some(party) = world.get_party(party_id) {
            for m in party.active_members() {
                if (m as u32) < crate::world::BOT_ID_BASE {
                    st.killers.entry(m).or_insert(nation);
                }
            }
        }
    }

    // the golem comes back for the killer's nation
    if let Some(&nid) = world.spawn_event_npc(proto_for(nation), true, ZONE_RONARK, npc.x, npc.z, 1).first() {
        st.golems.insert(nid, nation);
    }
    st.score[(nation - 1) as usize] += 1;
    let (k, e) = (st.score[0], st.score[1]);

    if k >= TOTAL || e >= TOTAL {
        finish(world, &mut st);
    }
}

// ── Tick ─────────────────────────────────────────────────────────────────

pub fn tick(world: &WorldState) {
    let now = now_unix();
    let mut st = state().lock();
    if st.active {
        if now >= st.started + DURATION_SECS {
            finish(world, &mut st);
            return;
        }
        if now >= st.next_announce {
            st.next_announce = now + SCORE_ANNOUNCE_SECS;
            let left = (st.started + DURATION_SECS).saturating_sub(now) / 60 + 1;
            let msg = format!(
                "[Fire & Ice] Karus {} - El Morad {} ({} min left)",
                st.score[0], st.score[1], left
            );
            drop(st);
            announce(world, &msg);
        }
        return;
    }
    if AUTO.load(Ordering::Relaxed) {
        let (day, hour) = day_and_hour();
        if AUTO_HOURS.contains(&hour) && st.last_auto_day_hour != (day, hour) {
            st.last_auto_day_hour = (day, hour);
            drop(st);
            gm_open(world);
        }
    }
}

// ── Resolve + rewards ────────────────────────────────────────────────────

fn finish(world: &WorldState, st: &mut State) {
    st.active = false;
    let (karus, elmo) = (st.score[0], st.score[1]);

    for (&nid, _) in st.golems.iter() {
        world.kill_npc(nid);
    }
    st.golems.clear();

    let winner: Option<u8> = match karus.cmp(&elmo) {
        std::cmp::Ordering::Greater => Some(1),
        std::cmp::Ordering::Less => Some(2),
        std::cmp::Ordering::Equal => None,
    };
    let result = match winner {
        Some(n) => format!("{} wins Fire & Ice!", nation_name(n)),
        None => "Fire & Ice ended in a draw!".to_string(),
    };
    announce(world, &format!("[Fire & Ice] {result} Karus {karus} - El Morad {elmo}"));

    let killers: Vec<(SessionId, u8)> = st
        .killers
        .iter()
        .filter(|(sid, _)| world.get_character_info(**sid).is_some())
        .map(|(s, n)| (*s, *n))
        .collect();
    st.killers.clear();
    let mut rng = rand::thread_rng();

    let notify = |sid: SessionId, msg: &str| {
        world.send_to_session_owned(sid, build_notice_packet(8, msg));
    };
    let give = |sid: SessionId, item: u32, count: u16| -> bool { world.give_item(sid, item, count) };

    let pick = |list: &[SessionId], n: usize, rng: &mut rand::rngs::ThreadRng| -> Vec<SessionId> {
        let mut v = list.to_vec();
        v.shuffle(rng);
        v.truncate(n);
        v
    };

    let all: Vec<SessionId> = killers.iter().map(|(s, _)| *s).collect();
    match winner {
        Some(w) => {
            let winners: Vec<SessionId> = killers.iter().filter(|(_, n)| *n == w).map(|(s, _)| *s).collect();
            let losers: Vec<SessionId> = killers.iter().filter(|(_, n)| *n != w).map(|(s, _)| *s).collect();
            let vow_winners: HashSet<SessionId> = pick(&winners, 2, &mut rng).into_iter().collect();
            let voucher: HashSet<SessionId> = pick(&winners, 1, &mut rng).into_iter().collect();
            for &sid in &winners {
                give(sid, ITEM_DUAL_POTION, 3);
                world.give_item_with_expiry(sid, ITEM_EVENT_EMBLEM, 1, 1);
                if vow_winners.contains(&sid) {
                    give(sid, ITEM_HEROS_VOW, 22);
                }
                if voucher.contains(&sid) {
                    world.give_item_with_expiry(sid, ITEM_EMBLEM_VOUCHER_30D, 1, 30);
                }
                notify(sid, "[Fire & Ice] Victory! 3 Dual Potion and a 24 hour Event Emblem were added to your bag.");
            }
            let vow_losers: HashSet<SessionId> = pick(&losers, 1, &mut rng).into_iter().collect();
            for &sid in &losers {
                give(sid, ITEM_DUAL_POTION, 1);
                world.give_item_with_expiry(sid, ITEM_EVENT_EMBLEM, 1, 1);
                if vow_losers.contains(&sid) {
                    give(sid, ITEM_HEROS_VOW, 22);
                }
                notify(sid, "[Fire & Ice] Your nation lost. 1 Dual Potion and an Event Emblem were added to your bag.");
            }
        }
        None => {
            let vow: HashSet<SessionId> = pick(&all, 3, &mut rng).into_iter().collect();
            let voucher: HashSet<SessionId> = pick(&all, 1, &mut rng).into_iter().collect();
            for &sid in &all {
                give(sid, ITEM_DUAL_POTION, 3);
                world.give_item_with_expiry(sid, ITEM_EVENT_EMBLEM, 1, 1);
                if vow.contains(&sid) {
                    give(sid, ITEM_HEROS_VOW, 22);
                }
                if voucher.contains(&sid) {
                    world.give_item_with_expiry(sid, ITEM_EMBLEM_VOUCHER_30D, 1, 30);
                }
                notify(sid, "[Fire & Ice] Draw. Participation rewards were added to your bag.");
            }
        }
    }

    // one run: a small chance that one participant finds a Lunar War Belt
    if !all.is_empty() && rng.gen_range(0..100) < 5 {
        let lucky = all[rng.gen_range(0..all.len())];
        if give(lucky, ITEM_LUNAR_WAR_BELT, 1) {
            notify(lucky, "[Fire & Ice] You found a Lunar War Belt!");
        }
    }
    info!(karus, elmo, participants = all.len(), "Fire & Ice golem war finished");
}
