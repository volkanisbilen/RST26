//! Fire & Ice Golem war (Ronark Land Bowl).
//!
//! 50 El Morad Flame Golems and 50 Karus Ice Golems stand around the Bowl. A nation can only hurt the
//! other nation's golems; a golem that dies is re-spawned for the nation that killed it (a conversion,
//! the total stays 100). Every conversion is one point. After 10 minutes - or as soon as a nation
//! reaches 100 points - the nation with the higher score wins.
//!
//! GM: `+golemopen`, `+golemend`, `+golemauto on|off` (14:00 and 22:00 server time).
//! Client identities: mob_us.tbl 9929 "Flame Golem" / 9930 "Ice Golem".

use std::collections::HashMap;
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
const SCORE_SYNC_SECS: u64 = 5;
const MIN_SPACING: f32 = 11.0;
/// Native Ronark-war UI packet (client sub_715330): [1] start, [2][winner] end, [3][elapsed][a][b] score.
const WIZ_RONARK_WAR: u8 = 0xD3;
/// The client's score window assumes a 30 minute battle (remaining = 1800 - elapsed).
const CLIENT_BATTLE_SECS: u64 = 1800;
/// First counter of the score packet is shown on the left (blue / Karus Ice), second on the right.
const KARUS_FIRST: bool = true;
const AUTO_HOURS: [u32; 2] = [14, 22];

#[allow(dead_code)]
const ITEM_DUAL_POTION: u32 = 811_182_000;
#[allow(dead_code)]
const ITEM_HEROS_VOW: u32 = 811_065_000;
#[allow(dead_code)]
const ITEM_EVENT_EMBLEM: u32 = 1_900_004_106;
#[allow(dead_code)]
const ITEM_EMBLEM_VOUCHER_30D: u32 = 931_734_000;
#[allow(dead_code)]
const ITEM_LUNAR_WAR_BELT: u32 = 340_710_124;

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

/// Golems currently owned by each nation: [Karus, El Morad]. The score IS this count (50 - 50 at the
/// start, every conversion moves one golem from one side to the other, 100 wins).
fn counts(st: &State) -> [u32; 2] {
    let mut c = [0u32; 2];
    for &n in st.golems.values() {
        if n == 1 || n == 2 {
            c[(n - 1) as usize] += 1;
        }
    }
    c
}

fn native(sub: u8) -> ko_protocol::Packet {
    let mut pkt = ko_protocol::Packet::new(WIZ_RONARK_WAR);
    pkt.write_u8(sub);
    pkt
}

/// Native score window update: remaining time (client counts 1800 - elapsed) and both counters.
fn score_packet(st: &State, now: u64) -> ko_protocol::Packet {
    let elapsed = now.saturating_sub(st.started).min(DURATION_SECS);
    let shown = CLIENT_BATTLE_SECS - DURATION_SECS + elapsed;
    let c = counts(st);
    let (left, right) = if KARUS_FIRST { (c[0], c[1]) } else { (c[1], c[0]) };
    let mut pkt = native(3);
    pkt.write_u32(shown as u32);
    pkt.write_u32(left);
    pkt.write_u32(right);
    pkt
}

fn send_all(world: &WorldState, pkt: ko_protocol::Packet) {
    world.broadcast_to_all(Arc::new(pkt), None);
}

/// A player who just arrived in Ronark Land while the event runs gets the panel too.
pub fn send_state_to_player(world: &WorldState, sid: SessionId) {
    let st = state().lock();
    if !st.active {
        return;
    }
    world.send_to_session_owned(sid, native(1));
    world.send_to_session_owned(sid, score_packet(&st, now_unix()));
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
    // spread the golems over the whole Bowl: random walkable points, as far apart as the area allows
    // (the spacing is relaxed step by step until all 100 golems have a place)
    let mut spots: Vec<(f32, f32)> = Vec::with_capacity(PER_NATION * 2);
    let mut rng = rand::thread_rng();
    for (spacing, jitter) in [(MIN_SPACING, 18.0f32), (8.0, 24.0), (5.0, 30.0), (2.0, 36.0), (0.0, 40.0)] {
        for _ in 0..4000 {
            if spots.len() >= PER_NATION * 2 {
                break;
            }
            let Some((bx, bz)) = world.get_start_position_random(ZONE_RONARK) else { break };
            let (x, z) = (bx + rng.gen_range(-jitter..jitter), bz + rng.gen_range(-jitter..jitter));
            if spots.iter().all(|&(px, pz)| (px - x).powi(2) + (pz - z).powi(2) >= spacing * spacing) {
                spots.push((x, z));
            }
        }
        if spots.len() >= PER_NATION * 2 {
            break;
        }
    }
    if spots.is_empty() {
        return "Ronark Land has no spawn points for the golems.";
    }
    // never start with fewer golems than 2 per spawn point pair: use what we found (rounded to even)
    let usable = spots.len() - spots.len() % 2;
    spots.truncate(usable);
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
    st.next_announce = now + SCORE_SYNC_SECS;
    send_all(world, native(1));
    send_all(world, score_packet(&st, now));
    drop(st);
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
    let [k, e] = counts(&st);
    send_all(world, score_packet(&st, now_unix()));

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
            st.next_announce = now + SCORE_SYNC_SECS;
            send_all(world, score_packet(&st, now));
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
    let [karus, elmo] = counts(st);

    for (&nid, _) in st.golems.iter() {
        world.kill_npc(nid);
    }
    st.golems.clear();

    let winner: Option<u8> = match karus.cmp(&elmo) {
        std::cmp::Ordering::Greater => Some(1),
        std::cmp::Ordering::Less => Some(2),
        std::cmp::Ordering::Equal => None,
    };
    let mut end = native(2);
    end.write_u8(winner.unwrap_or(0));
    send_all(world, end);

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
    let pick = |list: &[SessionId], n: usize, rng: &mut rand::rngs::ThreadRng| -> Vec<SessionId> {
        let mut v = list.to_vec();
        v.shuffle(rng);
        v.truncate(n);
        v
    };

    // rewards from the web panel (event_reward_rule, event "golem"): cond win / lose / draw / belt, pick = how many
    // random players of the group get the row (0 = all of them), chance = percent
    let rules = world.reward_rules_for("golem");
    let grant = |group: &[SessionId], cond: &str, rng: &mut rand::rngs::ThreadRng| {
        for r in rules.iter().filter(|r| r.cond == cond) {
            let chosen = if r.pick > 0 { pick(group, r.pick as usize, rng) } else { group.to_vec() };
            for sid in chosen {
                if r.chance < 100 && rng.gen_range(0..100) >= r.chance {
                    continue;
                }
                // a full bag mails the reward instead of dropping it
                if world.give_rule_item_or_mail(sid, r, "Fire & Ice").delivered() && cond == "belt" {
                    notify(sid, "[Fire & Ice] You found a special reward!");
                }
            }
        }
    };

    let all: Vec<SessionId> = killers.iter().map(|(s, _)| *s).collect();
    match winner {
        Some(w) => {
            let winners: Vec<SessionId> = killers.iter().filter(|(_, n)| *n == w).map(|(s, _)| *s).collect();
            let losers: Vec<SessionId> = killers.iter().filter(|(_, n)| *n != w).map(|(s, _)| *s).collect();
            grant(&winners, "win", &mut rng);
            for &sid in &winners {
                notify(sid, "[Fire & Ice] Victory! Your rewards were added to your bag.");
            }
            grant(&losers, "lose", &mut rng);
            for &sid in &losers {
                notify(sid, "[Fire & Ice] Your nation lost. Participation rewards were added to your bag.");
            }
        }
        None => {
            grant(&all, "draw", &mut rng);
            for &sid in &all {
                notify(sid, "[Fire & Ice] Draw. Participation rewards were added to your bag.");
            }
        }
    }
    grant(&all, "belt", &mut rng);
    info!(karus, elmo, participants = all.len(), "Fire & Ice golem war finished");
}
