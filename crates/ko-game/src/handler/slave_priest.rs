//! Akara's Slave Priest (hook `WIZ_EXT_HOOK` 0xE9 / sub `SLAVEPRIEST` 0xEC).
//!
//! The Slave Priest scroll (item 751740000, skill 495146) gives its owner a long buff
//! ([`SP_BUFF_TYPE`]). While it runs, the JstKO mini window controls a "priest" that looks after
//! the owner and the party members within 25 m:
//! heal below a chosen HP percentage, Superioris / Undying HP buff, AC buff, cure, resurrect.
//!
//! - C2S `[op 1 start/update | 2 stop | 3 state][healPct][hpBuff 0/1/2][ac][heal][cure][res][party]`
//! - S2C `[op 1 show | 2 hide | 3 state]` and for op 1/3
//!   `[active][healPct][hpBuff][ac][heal][cure][res][party][u32 secondsLeft]`
//!
//! The priest uses the Slave Priest skill rows (5009xx) of the magic tables. Their effects are applied
//! silently: the client's skill table has no entries for them, so no effect packets are sent.

use std::sync::LazyLock;
use std::time::Instant;

use dashmap::DashMap;
use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::ClientSession;
use crate::world::{WorldState, USER_DEAD};
use crate::zone::SessionId;

pub const EXT_SUB_SLAVEPRIEST: u8 = 0xEC;
/// Buff type of the scroll's skill 495146 (see migration 20261014000002).
pub const SP_BUFF_TYPE: i32 = 196;

const RANGE_SQ: f32 = 25.0 * 25.0;
const SKILL_HEAL: u32 = 500967; // Superior healing (1920)
const SKILL_HEAL_COMPLETE: u32 = 500968; // Complete healing (10000)
const SKILL_GROUP_HEAL: u32 = 500969; // Group massive healing (960)
const SKILL_GROUP_HEAL_COMPLETE: u32 = 500970; // Group complete healing (10000)
const SKILL_HP_SUPERIORIS: u32 = 500965; // HP +2500
const SKILL_HP_UNDYING: u32 = 500966; // HP +60 %
const SKILL_AC: u32 = 500964; // AC +400
const BUFF_TYPE_HP: i32 = 1;
const BUFF_TYPE_AC: i32 = 2;

#[derive(Clone, Copy)]
struct SpState {
    active: bool,
    heal_pct: u8,
    hp_buff: u8,
    ac: u8,
    heal: u8,
    cure: u8,
    res: u8,
    party: u8,
    last_act: Instant,
    last_res: Instant,
}

impl Default for SpState {
    fn default() -> Self {
        Self {
            active: false,
            heal_pct: 60,
            hp_buff: 1,
            ac: 1,
            heal: 1,
            cure: 1,
            res: 1,
            party: 1,
            last_act: Instant::now(),
            last_res: Instant::now(),
        }
    }
}

static STATES: LazyLock<DashMap<SessionId, SpState>> = LazyLock::new(DashMap::new);

/// The visible priest (runtime NPC) of each owner.
static PRIESTS: LazyLock<DashMap<SessionId, u32>> = LazyLock::new(DashMap::new);

/// NPC template used as the priest's body ("Priest of Akara").
const PRIEST_TEMPLATE_SID: u16 = 30002;
/// The priest walks to its owner when it is farther away than this (metres).
const FOLLOW_DIST: f32 = 4.0;

fn remove_priest(world: &WorldState, owner: SessionId) {
    if let Some((_, nid)) = PRIESTS.remove(&owner) {
        world.kill_npc(nid);
    }
}

/// Keep the priest NPC beside its owner: spawn it, bring it along, rebuild it after a zone change.
fn follow_owner(world: &WorldState, owner: SessionId) {
    let Some(pos) = world.get_position(owner) else {
        return;
    };
    let existing = PRIESTS.get(&owner).map(|e| *e);
    if let Some(nid) = existing {
        match world.get_npc_instance(nid) {
            Some(npc) if npc.zone_id == pos.zone_id => {
                let (dx, dz) = (npc.x - pos.x, npc.z - pos.z);
                let dist = (dx * dx + dz * dz).sqrt();
                if dist > 40.0 {
                    remove_priest(world, owner); // lost behind (teleport): build it again below
                } else {
                    if dist >= FOLLOW_DIST {
                        let k = 2.0 / dist;
                        let event_room = world.get_event_room(owner);
                        crate::handler::move_handler::move_follower_npc(
                            world,
                            nid,
                            pos.x + dx * k,
                            pos.z + dz * k,
                            dist,
                            event_room,
                        );
                    }
                    return;
                }
            }
            _ => remove_priest(world, owner),
        }
    }
    let Some(ch) = world.get_character_info(owner) else {
        return;
    };
    let Some(template) = world.get_npc_template(PRIEST_TEMPLATE_SID, false) else {
        tracing::warn!("slave priest: NPC template {PRIEST_TEMPLATE_SID} missing");
        return;
    };
    let nid = world.allocate_npc_id();
    let (x, z) = (pos.x + 1.5, pos.z + 1.5);
    let event_room = world.get_event_room(owner);
    let instance = crate::npc::NpcInstance {
        nid,
        proto_id: PRIEST_TEMPLATE_SID,
        is_monster: false,
        zone_id: pos.zone_id,
        x,
        y: pos.y,
        z,
        direction: 0,
        region_x: crate::zone::calc_region(x),
        region_z: crate::zone::calc_region(z),
        gate_open: 0,
        object_type: 0,
        nation: ch.nation,
        special_type: 0,
        trap_number: 0,
        event_room,
        is_event_npc: false,
        summon_type: 0,
        user_name: String::new(),
        pet_name: String::new(),
        clan_name: String::new(),
        clan_id: 0,
        clan_mark_version: 0,
    };
    world.insert_npc_instance(instance.clone());
    world.init_npc_hp(nid, template.max_hp.max(1) as i32);
    world.broadcast_to_3x3(
        instance.zone_id,
        instance.region_x,
        instance.region_z,
        std::sync::Arc::new(crate::npc::build_npc_inout(crate::npc::NPC_IN, &instance, &template)),
        None,
        event_room,
    );
    PRIESTS.insert(owner, nid);
}

fn seconds_left(world: &WorldState, sid: SessionId) -> u32 {
    world
        .get_active_buffs(sid)
        .iter()
        .find(|b| b.buff_type == SP_BUFF_TYPE)
        .map(|b| b.duration_secs.saturating_sub(b.start_time.elapsed().as_secs() as u32))
        .unwrap_or(0)
}

fn state_packet(op: u8, st: &SpState, left: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_SLAVEPRIEST);
    pkt.write_u8(op);
    pkt.write_u8(u8::from(st.active));
    pkt.write_u8(st.heal_pct);
    pkt.write_u8(st.hp_buff);
    pkt.write_u8(st.ac);
    pkt.write_u8(st.heal);
    pkt.write_u8(st.cure);
    pkt.write_u8(st.res);
    pkt.write_u8(st.party);
    pkt.write_u32(left);
    pkt
}

fn hide_packet() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_SLAVEPRIEST);
    pkt.write_u8(2);
    pkt
}

/// Client request (start / stop / state).
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut r = PacketReader::new(&pkt.data);
    let op = r.read_u8().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();
    if !world.has_buff(sid, SP_BUFF_TYPE) {
        STATES.remove(&sid);
        remove_priest(&world, sid);
        return session.send_packet(&hide_packet()).await;
    }
    let mut st = STATES.get(&sid).map(|s| *s).unwrap_or_default();
    match op {
        1 => {
            st.heal_pct = r.read_u8().unwrap_or(60).clamp(5, 95);
            st.hp_buff = r.read_u8().unwrap_or(1).min(2);
            st.ac = u8::from(r.read_u8().unwrap_or(1) != 0);
            st.heal = u8::from(r.read_u8().unwrap_or(1) != 0);
            st.cure = u8::from(r.read_u8().unwrap_or(1) != 0);
            st.res = u8::from(r.read_u8().unwrap_or(1) != 0);
            st.party = u8::from(r.read_u8().unwrap_or(1) != 0);
            st.active = true;
            st.last_act = Instant::now() - std::time::Duration::from_secs(10);
        }
        2 => {
            st.active = false;
            remove_priest(&world, sid);
        }
        _ => {}
    }
    STATES.insert(sid, st);
    session
        .send_packet(&state_packet(3, &st, seconds_left(&world, sid)))
        .await
}

fn hp_pct(hp: i16, max: i16) -> i32 {
    if max <= 0 {
        100
    } else {
        hp as i32 * 100 / max as i32
    }
}

fn heal_amount(world: &WorldState, skill: u32) -> i32 {
    world
        .get_magic_type3(skill as i32)
        .and_then(|t| t.first_damage)
        .unwrap_or(0)
        .abs()
}

fn heal_player(world: &WorldState, sid: SessionId, amount: i32) {
    let Some(ch) = world.get_character_info(sid) else {
        return;
    };
    if ch.res_hp_type == USER_DEAD || ch.hp <= 0 || amount <= 0 {
        return;
    }
    let new_hp = (ch.hp as i32 + amount).min(ch.max_hp as i32) as i16;
    if new_hp == ch.hp {
        return;
    }
    world.update_character_hp(sid, new_hp);
    world.send_to_session_owned(
        sid,
        crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp),
    );
}

fn apply_buff_skill(world: &WorldState, owner: SessionId, target: SessionId, skill_id: u32) {
    let (Some(skill), Some(type4)) = (
        world.get_magic(skill_id as i32),
        world.get_magic_type4(skill_id as i32),
    ) else {
        tracing::warn!("slave priest: magic rows of skill {skill_id} missing (magic / magic_type4)");
        return;
    };
    let buff = crate::handler::magic_process::create_active_buff(skill_id, owner, &type4, true);
    let duration = buff.duration_secs;
    world.apply_buff(target, buff);
    crate::handler::magic_process::apply_type4_stats(
        world,
        target,
        &type4,
        skill.skill.unwrap_or(0),
        skill_id,
    );
    world.set_user_ability(target);
    world.send_item_move_refresh(target);

    // the buff icon + timer in the client (same packet a successful cast / a login recast sends)
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(3); // MAGIC_EFFECTING
    pkt.write_u32(skill_id);
    pkt.write_u32(owner as u32);
    pkt.write_u32(target as u32);
    for v in [0u32, 1, 0, duration, 0, type4.speed.unwrap_or(0) as u32, 0] {
        pkt.write_u32(v);
    }
    world.send_to_session_owned(target, pkt);
}

fn resurrect(world: &WorldState, sid: SessionId) {
    let Some(ch) = world.get_character_info(sid) else {
        return;
    };
    if ch.res_hp_type != USER_DEAD && ch.hp > 0 {
        return;
    }
    let zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    world.update_res_hp_type(sid, 1);
    world.update_character_hp(sid, ch.max_hp);
    world.update_character_mp(sid, 0);
    world.update_session(sid, |h| {
        h.who_killed_me = -1;
        h.lost_exp = 0;
    });
    if let Some(pos) = world.get_position(sid) {
        let mut regene = Packet::new(Opcode::WizRegene as u8);
        regene.write_u16((pos.x * 10.0) as u16);
        regene.write_u16((pos.z * 10.0) as u16);
        regene.write_u16(0);
        world.send_to_session_owned(sid, regene);
    }
    crate::handler::magic_process::post_resurrection_sequence(world, sid, zone);
    crate::handler::party::broadcast_party_hp(world, sid);
}

/// Every second: start / stop the priest with the scroll buff and let it act every 2 seconds.
pub fn tick(world: &WorldState) {
    for sid in world.collect_sessions_by(|h| h.buffs.contains_key(&SP_BUFF_TYPE)) {
        if !STATES.contains_key(&sid) {
            let st = SpState::default();
            STATES.insert(sid, st);
            world.send_to_session_owned(sid, state_packet(1, &st, seconds_left(world, sid)));
        }
    }

    let owners: Vec<SessionId> = STATES.iter().map(|e| *e.key()).collect();
    for owner in owners {
        let alive_scroll = world.has_buff(owner, SP_BUFF_TYPE);
        if !alive_scroll {
            STATES.remove(&owner);
            remove_priest(world, owner);
            world.send_to_session_owned(owner, hide_packet());
            continue;
        }
        let Some(mut st) = STATES.get(&owner).map(|s| *s) else {
            continue;
        };
        if !st.active {
            // stopped: the scroll's time stands still until the priest is started again
            world.pause_buff_second(owner, SP_BUFF_TYPE);
            remove_priest(world, owner);
            continue;
        }
        follow_owner(world, owner);
        if st.last_act.elapsed().as_secs() < 2 {
            continue;
        }
        st.last_act = Instant::now();
        act(world, owner, &mut st);
        if let Some(mut e) = STATES.get_mut(&owner) {
            e.last_act = st.last_act;
            e.last_res = st.last_res;
        }
    }
}

fn act(world: &WorldState, owner: SessionId, st: &mut SpState) {
    let Some(me) = world.get_character_info(owner) else {
        return;
    };
    if me.res_hp_type == USER_DEAD || me.hp <= 0 {
        return;
    }
    let Some(my_pos) = world.get_position(owner) else {
        return;
    };

    // owner first, then the party members within range
    let mut targets: Vec<SessionId> = vec![owner];
    if st.party != 0 {
        if let Some(party) = world.get_party_id(owner).and_then(|p| world.get_party(p)) {
            for m in party.active_members() {
                if m == owner {
                    continue;
                }
                let near = world
                    .get_position(m)
                    .map(|p| {
                        p.zone_id == my_pos.zone_id
                            && (p.x - my_pos.x).powi(2) + (p.z - my_pos.z).powi(2) <= RANGE_SQ
                    })
                    .unwrap_or(false);
                if near {
                    targets.push(m);
                }
            }
        }
    }

    // resurrect (one every 5 seconds)
    if st.res != 0 && st.last_res.elapsed().as_secs() >= 5 {
        for &t in &targets {
            if world.is_player_dead(t) {
                resurrect(world, t);
                st.last_res = Instant::now();
                break;
            }
        }
    }

    // heal
    if st.heal != 0 {
        let need: Vec<(SessionId, i16, i16)> = targets
            .iter()
            .filter_map(|&t| world.get_character_info(t).map(|c| (t, c.hp, c.max_hp)))
            .filter(|&(t, hp, max)| !world.is_player_dead(t) && hp_pct(hp, max) < st.heal_pct as i32)
            .collect();
        if need.len() >= 2 {
            let skill = if need.iter().any(|&(_, hp, max)| hp_pct(hp, max) < 35) {
                SKILL_GROUP_HEAL_COMPLETE
            } else {
                SKILL_GROUP_HEAL
            };
            let amount = heal_amount(world, skill);
            for (t, _, _) in &need {
                heal_player(world, *t, amount);
            }
        } else if let Some(&(t, hp, max)) = need.first() {
            let skill = if hp_pct(hp, max) < 35 {
                SKILL_HEAL_COMPLETE
            } else {
                SKILL_HEAL
            };
            heal_player(world, t, heal_amount(world, skill));
        }
    }

    // buffs and cure
    for &t in &targets {
        if world.is_player_dead(t) {
            continue;
        }
        if st.hp_buff != 0 && !world.has_buff(t, BUFF_TYPE_HP) {
            let skill = if st.hp_buff == 2 {
                SKILL_HP_UNDYING
            } else {
                SKILL_HP_SUPERIORIS
            };
            apply_buff_skill(world, owner, t, skill);
        }
        if st.ac != 0 {
            // another, weaker AC buff does not block the priest's (+400)
            let blocked = world
                .get_active_buffs(t)
                .iter()
                .any(|b| b.buff_type == BUFF_TYPE_AC && (b.skill_id == SKILL_AC || b.ac >= 400));
            if !blocked {
                apply_buff_skill(world, owner, t, SKILL_AC);
                tracing::warn!("slave priest: AC buff {SKILL_AC} on sid {t}");
            }
        }
        if st.cure != 0 {
            crate::handler::magic_process::cure_player(world, t);
        }
    }
}
