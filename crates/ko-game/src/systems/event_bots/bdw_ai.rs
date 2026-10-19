//! Border Defence War objective of the event bots (child of `event_bots`, so it shares the
//! driver's private helpers). The bots are driven by the shared bot brain
//! (`bot_brain::tick_event`: threat, targeting, kite / retreat / heal, formation, consumables);
//! this module is the **objective layer** the brain asks when no fight is on, plus the BDW
//! target priorities.
//!
//! * **Altar.** Altar takers (`Role::Hotspot`) walk to the central Altar of Manes of their room
//!   and hit it down. The bot that lands the last hit carries the flag: it is slowed like a
//!   player carrier (`FRAGMENT_SPEED_VALUE`), walks to its own base and delivers through the
//!   same function the players use (`bdw_altar_delivery`), so score, win threshold and the
//!   respawn timer behave exactly as for humans. The carrier state is the normal
//!   `has_altar_obtained` flag of the room's `EventUser`.
//! * **Roamers** (`Role::Patrol`) sweep the arena between the altar and the enemy base,
//!   intercepting whatever comes; they never stand still in the open.
//! * **Enemy carrier.** Everybody converges on an enemy flag carrier (human or bot); within the
//!   brain's search range the carrier gets a large target bonus.
//! * **Ally carrier.** Escort: stay around him on the way home.
//! * **Independence.** A bot in a real player's party assists the party (the brain favours
//!   enemies that hit a party mate) but keeps acting on its own: no following.
//! * **No rewards for bots.** Bot `EventUser`s are created with `prize_given = true`; the reward
//!   collectors skip them.

use super::*;
use crate::systems::bdw;
use crate::systems::bot_brain::{self, BrainState, Drive, Role, Unit};

/// An escorting bot stays within this distance of an allied carrier.
const ESCORT_RANGE: f32 = 10.0;
/// One bot hit takes this share of the altar's maximum HP.
const ALTAR_HIT_PERCENT: i32 = 10;
/// Bot hits on the altar come at this interval.
const ALTAR_COOLDOWN_MS: u64 = 1_500;
/// Target bonus of the enemy altar carrier.
const CARRIER_BONUS: f32 = 60.0;
/// Radius of the drift around the (dead) altar while waiting for its respawn.
const ALTAR_WAIT_RADIUS: f32 = 10.0;
/// A roamer keeps one roam point for this long.
const ROAM_MIN_MS: u64 = 12_000;
const ROAM_MAX_MS: u64 = 30_000;

/// Step scale of a bot carrying the altar (the player slow-down factor of the Fragment of Manes).
pub(super) fn carrier_scale() -> f32 {
    bdw::FRAGMENT_SPEED_VALUE as f32 / 100.0
}

/// Step length of a carrying bot per tick (diagnostics / tests).
pub(super) fn carrier_step() -> f32 {
    STEP_PER_TICK * carrier_scale()
}

/// Centre of a nation's delivery zone.
pub(super) fn base_centre(nation: u8) -> (f32, f32) {
    if nation == 1 {
        ((bdw::KARUS_BASE_X_MIN + bdw::KARUS_BASE_X_MAX) / 2.0, (bdw::KARUS_BASE_Z_MIN + bdw::KARUS_BASE_Z_MAX) / 2.0)
    } else {
        (
            (bdw::ELMORAD_BASE_X_MIN + bdw::ELMORAD_BASE_X_MAX) / 2.0,
            (bdw::ELMORAD_BASE_Z_MIN + bdw::ELMORAD_BASE_Z_MAX) / 2.0,
        )
    }
}

/// The altar NPC of one room: `(npc id, x, z, hp)`.
pub(super) fn altar_of_room(world: &WorldState, room: u16) -> Option<(u32, f32, f32, i32)> {
    let npc = world
        .find_all_npcs_in_zone(bdw::ALTAR_OF_MANES, bdw::ZONE_BDW)
        .into_iter()
        .find(|n| n.event_room == room)?;
    let hp = world.get_npc_hp(npc.nid).unwrap_or(0);
    Some((npc.nid, npc.x, npc.z, hp))
}

fn dist(bot: &BotInstance, x: f32, z: f32) -> f32 {
    ((x - bot.x).powi(2) + (z - bot.z).powi(2)).sqrt()
}

fn pos_of(world: &WorldState, id: u32) -> Option<(f32, f32)> {
    if id >= BOT_ID_BASE {
        world.get_bot(id).map(|b| (b.x, b.z))
    } else {
        world.get_position(id as SessionId).map(|p| (p.x, p.z))
    }
}

/// The room's flag carrier `(name, sid, nation)`.
fn carrier(world: &WorldState, room: u16) -> Option<(String, u16, u8)> {
    let room = world.event_room_manager.get_room(TempleEventType::BorderDefenceWar, room as u8)?;
    bdw::find_flag_carrier(&room)
}

/// Target priority: the enemy altar carrier must die first.
pub(super) fn target_bonus(world: &WorldState, entry: &BotEntry, _bot: &BotInstance, u: &Unit) -> f32 {
    match carrier(world, entry.room) {
        Some((_, csid, cnation)) if cnation != entry.nation && csid as u32 == u.id => CARRIER_BONUS,
        _ => 0.0,
    }
}

/// Pure choice of the objective point for a bot with no fight on, from the room situation.
/// Returns what the brain should walk to, or `None` for "hit the altar now".
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Goal {
    /// Carry the flag home (slowed).
    CarryHome(f32, f32),
    /// Run at the enemy carrier.
    Intercept(f32, f32),
    /// Stay around the allied carrier.
    Escort(f32, f32),
    /// Walk to the altar / hit it.
    Altar(f32, f32),
    HitAltar,
    /// Wait for the altar respawn around its spot.
    AltarWait(f32, f32),
    /// Sweep the arena (roamer role).
    Roam,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn choose_goal(
    me: &str,
    nation: u8,
    role: Role,
    bot_pos: (f32, f32),
    carrier: Option<(&str, u8, (f32, f32))>,
    altar: Option<((f32, f32), i32)>,
    melee_range: f32,
) -> Goal {
    if let Some((cname, cnation, cpos)) = carrier {
        if cname == me {
            return Goal::CarryHome(base_centre(nation).0, base_centre(nation).1);
        }
        if cnation != nation {
            return Goal::Intercept(cpos.0, cpos.1);
        }
        return Goal::Escort(cpos.0, cpos.1);
    }
    let Some(((ax, az), hp)) = altar else { return Goal::Roam };
    if hp <= 0 {
        return if role == Role::Hotspot { Goal::AltarWait(ax, az) } else { Goal::Roam };
    }
    if role == Role::Patrol {
        return Goal::Roam;
    }
    let d = ((ax - bot_pos.0).powi(2) + (az - bot_pos.1).powi(2)).sqrt();
    if d > melee_range {
        Goal::Altar(ax, az)
    } else {
        Goal::HitAltar
    }
}

/// Roam point of a BDW roamer: somewhere on the line altar -> enemy base, spread sideways.
fn roam_point(world: &WorldState, entry: &BotEntry, altar: Option<(f32, f32)>, rng: &mut impl Rng) -> Option<(f32, f32)> {
    let (ax, az) = altar.unwrap_or((127.0, 131.0));
    let (ex, ez) = base_centre(if entry.nation == 1 { 2 } else { 1 });
    let t: f32 = rng.gen_range(0.0..0.8);
    let (px, pz) = (ax + (ex - ax) * t, az + (ez - az) * t);
    let (ox, oz) = (rng.gen_range(-18.0..18.0), rng.gen_range(-18.0..18.0));
    let zone = world.get_zone(bdw::ZONE_BDW)?;
    let map = zone.map_data.as_ref()?;
    crate::systems::pathfind::nearest_bot_point(map, px + ox, pz + oz, 6)
}

/// The BDW objective step of the brain.
pub(super) fn objective(world: &WorldState, entry: &BotEntry, bot: &BotInstance, st: &mut BrainState, now_ms: u64) -> Drive {
    let carrier = carrier(world, entry.room).and_then(|(name, csid, cnation)| {
        let pos = if name == entry.name { Some((bot.x, bot.z)) } else { pos_of(world, csid as u32) };
        pos.map(|p| (name, cnation, p))
    });
    let altar = altar_of_room(world, entry.room);
    let goal = choose_goal(
        &entry.name,
        entry.nation,
        st.role,
        (bot.x, bot.z),
        carrier.as_ref().map(|(n, c, p)| (n.as_str(), *c, *p)),
        altar.map(|(_, x, z, hp)| ((x, z), hp)),
        MELEE_RANGE,
    );
    match goal {
        Goal::CarryHome(hx, hz) => {
            if bdw::is_in_delivery_zone(entry.nation, bot.x, bot.z) {
                crate::handler::move_handler::bdw_altar_delivery(
                    world,
                    bot.id as SessionId,
                    &entry.name,
                    entry.nation,
                    bot.x,
                    bot.z,
                );
                return Drive::Handled;
            }
            Drive::GoalScaled(hx, hz, carrier_scale())
        }
        Goal::Intercept(x, z) => Drive::Goal(x, z),
        Goal::Escort(x, z) => bot_brain::wander_around(bot, st, x, z, ESCORT_RANGE, now_ms),
        Goal::Altar(x, z) => Drive::Goal(x, z),
        Goal::AltarWait(x, z) => bot_brain::wander_around(bot, st, x, z, ALTAR_WAIT_RADIUS, now_ms),
        Goal::HitAltar => {
            if let Some((altar_id, _, _, _)) = altar {
                if now_ms >= entry.next_attack_ms {
                    world.event_bots.update(bot.id, |e| e.next_attack_ms = now_ms + ALTAR_COOLDOWN_MS);
                    hit_altar(world, entry, bot, altar_id);
                }
            }
            Drive::Handled
        }
        Goal::Roam => {
            let mut rng = rand::thread_rng();
            let reached = st.hotspot.is_some_and(|(hx, hz)| dist(bot, hx, hz) < 4.0);
            if st.hotspot.is_none() || reached || now_ms >= st.hotspot_until_ms {
                let Some(p) = roam_point(world, entry, altar.map(|(_, x, z, _)| (x, z)), &mut rng) else {
                    // no map (tests): drift around the altar spot
                    let (ax, az) = altar.map(|(_, x, z, _)| (x, z)).unwrap_or((bot.x, bot.z));
                    return bot_brain::wander_around(bot, st, ax, az, 14.0, now_ms);
                };
                st.hotspot = Some(p);
                st.hotspot_until_ms = now_ms + rng.gen_range(ROAM_MIN_MS..=ROAM_MAX_MS);
            }
            match st.hotspot {
                Some((x, z)) => Drive::Goal(x, z),
                None => Drive::Handled,
            }
        }
    }
}

/// One bot hit on the altar; the last hit kills it and makes the bot the carrier.
fn hit_altar(world: &WorldState, entry: &BotEntry, bot: &BotInstance, altar_id: u32) {
    let Some(npc) = world.get_npc_instance(altar_id) else { return };
    let max_hp = world
        .get_npc_template(npc.proto_id, npc.is_monster)
        .map(|t| t.max_hp as i32)
        .unwrap_or(1000)
        .max(1);
    let hp = world.get_npc_hp(altar_id).unwrap_or(0);
    if hp <= 0 {
        return;
    }
    let damage = (max_hp * ALTAR_HIT_PERCENT / 100).max(1);
    let pos = [bot.x as i32, bot.y as i32, bot.z as i32];
    room_broadcast(
        world,
        bot.zone_id,
        bot.region_x,
        bot.region_z,
        entry.room,
        magic_packet(3, bot_ai::select_bot_skill_with_weapon(bot, Some(world)), bot.id, altar_id, [pos[0], pos[1], pos[2], -damage, 0, 0, 0]),
    );
    let new_hp = (hp - damage).max(0);
    world.update_npc_hp(altar_id, new_hp);
    if new_hp > 0 {
        return;
    }
    world.update_npc_ai(altar_id, |s| {
        s.state = crate::world::NpcState::Dead;
        s.target_id = None;
        s.npc_target_id = None;
    });
    world.clear_npc_damage(altar_id);
    let mut death = Packet::new(Opcode::WizDead as u8);
    death.write_u32(altar_id);
    world.broadcast_to_3x3(npc.zone_id, npc.region_x, npc.region_z, Arc::new(death), None, npc.event_room);
    crate::handler::attack::bdw_altar_flag_pickup_named(world, bot.id as SessionId, &entry.name, altar_id);
    // the carrier effect on the bot (players get it through a real session)
    room_broadcast(
        world,
        bot.zone_id,
        bot.region_x,
        bot.region_z,
        entry.room,
        magic_packet(
            3,
            bdw::BUFF_FRAGMENT_OF_MANES_SKILL,
            bot.id,
            bot.id,
            [0, 1, 0, 0, 0, bdw::FRAGMENT_SPEED_VALUE, 0],
        ),
    );
}

/// A dead bot that carried the altar drops it: flag cleared, respawn timer running.
pub(super) fn on_bot_death(world: &WorldState, entry: &BotEntry, bot_id: BotId) {
    crate::handler::dead::bdw_flag_carrier_death_named(world, bot_id as SessionId, &entry.name);
}

/// A party keeps at least this many slots free for real players when bots fill it.
const PARTY_FREE_SLOTS_FOR_HUMANS: usize = 2;

/// Put the room's free same-nation bots into the party of a real player (BDW only).
/// Called when a party exists in a BDW room (server-made event parties and parties players form
/// themselves). Respects the 8 member limit and keeps [`PARTY_FREE_SLOTS_FOR_HUMANS`] slots free.
/// Returns how many bots were added.
pub(super) fn fill_party(world: &WorldState, party_id: u16) -> usize {
    use crate::world::types::MAX_PARTY_USERS;
    let Some(party) = world.get_party(party_id) else { return 0 };
    let Some(leader) = party.leader_sid() else { return 0 };
    if leader as u32 >= BOT_ID_BASE {
        return 0;
    }
    let room = world.get_event_room(leader);
    let in_bdw = world.get_position(leader).is_some_and(|p| p.zone_id == bdw::ZONE_BDW);
    if room == 0 || !in_bdw || !world.event_bots.config(EventBotKind::Bdw).enabled {
        return 0;
    }
    let Some(nation) = world.get_character_info(leader).map(|c| c.nation) else { return 0 };
    let limit = MAX_PARTY_USERS - PARTY_FREE_SLOTS_FOR_HUMANS;
    let mut members = party.member_count();
    let mut added = 0;
    for id in world.event_bots.ids_of(EventBotKind::Bdw) {
        if members >= limit {
            break;
        }
        let Some(entry) = world.event_bots.get(id) else { continue };
        if entry.room != room || entry.nation != nation || world.get_party_id(id as SessionId).is_some() {
            continue;
        }
        let Some(bot) = world.get_bot(id) else { continue };
        if !world.add_party_member(party_id, id as SessionId) {
            break;
        }
        members += 1;
        added += 1;
        let target_number = world.get_party(party_id).map_or(-1, |p| p.target_number_id);
        let info = crate::handler::party::build_bot_party_member_info(&bot, 1, target_number);
        world.send_to_party(party_id, &info);
        bot_ai::broadcast_bot_party_hp(world, id);
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALTAR: (f32, f32) = (127.0, 131.0);

    #[test]
    fn the_carrier_walks_home_and_everybody_else_reacts_to_the_carrier() {
        // I carry: home (slowed by the brain through GoalScaled)
        let g = choose_goal("Me", 1, Role::Hotspot, (100.0, 100.0), Some(("Me", 1, (100.0, 100.0))), Some((ALTAR, 0)), 5.5);
        assert_eq!(g, Goal::CarryHome(base_centre(1).0, base_centre(1).1));
        // an enemy carries: intercept, whatever my role
        for role in [Role::Hotspot, Role::Patrol] {
            let g = choose_goal("Me", 1, role, (100.0, 100.0), Some(("Enemy", 2, (60.0, 70.0))), Some((ALTAR, 0)), 5.5);
            assert_eq!(g, Goal::Intercept(60.0, 70.0));
        }
        // a mate carries: escort
        let g = choose_goal("Me", 1, Role::Patrol, (100.0, 100.0), Some(("Mate", 1, (60.0, 70.0))), Some((ALTAR, 0)), 5.5);
        assert_eq!(g, Goal::Escort(60.0, 70.0));
    }

    #[test]
    fn altar_takers_go_for_the_altar_and_roamers_sweep_the_arena() {
        let g = choose_goal("Me", 1, Role::Hotspot, (60.0, 60.0), None, Some((ALTAR, 500)), 5.5);
        assert_eq!(g, Goal::Altar(ALTAR.0, ALTAR.1));
        let g = choose_goal("Me", 1, Role::Hotspot, (ALTAR.0 + 2.0, ALTAR.1), None, Some((ALTAR, 500)), 5.5);
        assert_eq!(g, Goal::HitAltar);
        let g = choose_goal("Me", 1, Role::Patrol, (60.0, 60.0), None, Some((ALTAR, 500)), 5.5);
        assert_eq!(g, Goal::Roam, "roamers are not glued to the altar");
        // dead altar: takers wait at the spot, roamers keep hunting
        assert_eq!(choose_goal("Me", 1, Role::Hotspot, (60.0, 60.0), None, Some((ALTAR, 0)), 5.5), Goal::AltarWait(ALTAR.0, ALTAR.1));
        assert_eq!(choose_goal("Me", 1, Role::Patrol, (60.0, 60.0), None, Some((ALTAR, 0)), 5.5), Goal::Roam);
        // no altar at all: roam
        assert_eq!(choose_goal("Me", 2, Role::Hotspot, (60.0, 60.0), None, None, 5.5), Goal::Roam);
    }

    #[test]
    fn carrier_is_slower_than_a_normal_step() {
        assert!(carrier_scale() < 1.0 && carrier_scale() > 0.0);
        assert!(carrier_step() < STEP_PER_TICK);
    }
}
