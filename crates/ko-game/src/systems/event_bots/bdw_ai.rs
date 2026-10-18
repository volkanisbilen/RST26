//! Border Defence War objective AI of the event bots (child of `event_bots`, so it shares the
//! driver's private helpers).
//!
//! * **Altar.** Bots walk to the central Altar of Manes of their room and hit it down. The bot
//!   that lands the last hit carries the flag: it is slowed like a player carrier
//!   (`FRAGMENT_SPEED_VALUE`), walks to its own base and delivers through the same function the
//!   players use (`bdw_altar_delivery`), so score, win threshold and the respawn timer behave
//!   exactly as for humans. The carrier state is the normal `has_altar_obtained` flag of the
//!   room's `EventUser`.
//! * **No rewards for bots.** Bot `EventUser`s are created with `prize_given = true`; the reward
//!   collectors skip them, so a bot carrying or delivering the altar earns nothing for itself.
//!   The nation's score still rises (that is what the fight is about) and real teammates profit
//!   through the nation result only.
//! * **Enemy carrier.** Every bot hunts an enemy flag carrier (human or bot).
//! * **Party.** A bot that sits in a party led by a real player follows him (field-bot style)
//!   and fights what comes near; it does not take the altar on its own, the leader decides.
//! * Enemies closer than [`AGGRO_RANGE`] are fought first (the regular driver code runs).

use super::*;
use crate::systems::bdw;

/// Enemies closer than this are fought before the objective.
const AGGRO_RANGE: f32 = 22.0;
/// A party bot starts walking back to its leader when farther than this.
const FOLLOW_TRIGGER: f32 = 10.0;
/// An escorting (party-less) bot stays within this distance of an allied carrier.
const ESCORT_RANGE: f32 = 10.0;
/// One bot hit takes this share of the altar's maximum HP.
const ALTAR_HIT_PERCENT: i32 = 10;
/// Bot hits on the altar come at this interval.
const ALTAR_COOLDOWN_MS: u64 = 1_500;
/// Fake target id for "walk to this point".
const POINT: u32 = u32::MAX;

/// Step length of a bot carrying the altar (the player slow-down factor of the Fragment of Manes).
pub(super) fn carrier_step() -> f32 {
    STEP_PER_TICK * bdw::FRAGMENT_SPEED_VALUE as f32 / 100.0
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

fn to(x: f32, z: f32) -> Target {
    Target { id: POINT, x, z }
}

fn dist(bot: &BotInstance, x: f32, z: f32) -> f32 {
    ((x - bot.x).powi(2) + (z - bot.z).powi(2)).sqrt()
}

/// Real leader of the bot's party and his position (same zone), if there is one.
fn human_leader(world: &WorldState, bot: &BotInstance) -> Option<(SessionId, f32, f32)> {
    let pid = world.get_party_id(bot.id as SessionId)?;
    let leader = world.get_party(pid)?.leader_sid()?;
    if leader as u32 >= BOT_ID_BASE {
        return None;
    }
    let pos = world.get_position(leader)?;
    (pos.zone_id == bot.zone_id).then_some((leader, pos.x, pos.z))
}

/// One BDW decision. Returns `true` when the bot was driven this tick; `false` lets the regular
/// fight-the-nearest-enemy code run.
pub(super) fn drive(world: &WorldState, entry: &BotEntry, bot: &BotInstance, now_ms: u64) -> bool {
    use crate::systems::event_room::TempleEventType;

    let carrier = {
        let Some(room) = world
            .event_room_manager
            .get_room(TempleEventType::BorderDefenceWar, entry.room as u8)
        else {
            return false;
        };
        bdw::find_flag_carrier(&room)
    };

    // 1. I carry the altar: home, slowly.
    if carrier.as_ref().is_some_and(|(name, _, _)| *name == entry.name) {
        if bdw::is_in_delivery_zone(entry.nation, bot.x, bot.z) {
            crate::handler::move_handler::bdw_altar_delivery(
                world,
                bot.id as SessionId,
                &entry.name,
                entry.nation,
                bot.x,
                bot.z,
            );
            return true;
        }
        let (hx, hz) = base_centre(entry.nation);
        walk_toward_step(world, entry, bot, to(hx, hz), carrier_step());
        return true;
    }

    // 2. A close enemy is fought by the regular code.
    let near = find_target(world, entry, bot);
    if near.is_some_and(|t| dist(bot, t.x, t.z) <= AGGRO_RANGE) {
        return false;
    }

    // 3. An enemy carries the altar: everybody hunts him.
    if let Some((_, csid, cnation)) = &carrier {
        if *cnation != entry.nation {
            if let Some(t) = resolve_target(world, entry, bot, *csid as u32) {
                engage(world, entry, bot, t, now_ms);
                return true;
            }
        }
    }

    // 4. In a party of a real player: stay with the leader.
    if let Some((_, lx, lz)) = human_leader(world, bot) {
        // closer than FOLLOW_TRIGGER the bot idles (a fight breaks out through step 2)
        if dist(bot, lx, lz) > FOLLOW_TRIGGER {
            walk_toward(world, entry, bot, to(lx, lz), now_ms);
        }
        return true;
    }

    // 5. An ally carries: escort him.
    if let Some((name, csid, cnation)) = &carrier {
        if *cnation == entry.nation && *name != entry.name {
            if let Some(pos) = pos_of(world, *csid as u32) {
                if dist(bot, pos.0, pos.1) > ESCORT_RANGE {
                    walk_toward(world, entry, bot, to(pos.0, pos.1), now_ms);
                }
                return true;
            }
        }
    }

    // 6. Nobody carries: take the altar.
    let Some((altar_id, ax, az, hp)) = altar_of_room(world, entry.room) else {
        return false;
    };
    let d = dist(bot, ax, az);
    if hp <= 0 {
        // waiting for the respawn: be there when it comes back
        if d > MELEE_RANGE * 2.0 {
            walk_toward(world, entry, bot, to(ax, az), now_ms);
        }
        return true;
    }
    if d > MELEE_RANGE {
        walk_toward(world, entry, bot, to(ax, az), now_ms);
        return true;
    }
    if now_ms >= entry.next_attack_ms {
        world.event_bots.update(bot.id, |e| e.next_attack_ms = now_ms + ALTAR_COOLDOWN_MS);
        hit_altar(world, entry, bot, altar_id);
    }
    true
}

fn pos_of(world: &WorldState, id: u32) -> Option<(f32, f32)> {
    if id >= BOT_ID_BASE {
        world.get_bot(id).map(|b| (b.x, b.z))
    } else {
        world.get_position(id as SessionId).map(|p| (p.x, p.z))
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
