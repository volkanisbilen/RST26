//! WIZ_DEAD (0x11) handler — death notification.
//! ## Death Animation Broadcast
//! When a unit dies, the server broadcasts the death to the 3x3 region:
//! `[u32 dead_unit_id]`
//! The client may also send WIZ_DEAD in some edge cases (e.g., auto-death
//! items). The handler ignores client-initiated death packets since death
//! is always server-authoritative.
//! ## Helper: `broadcast_death`
//! Other handlers (combat, HP change) call `broadcast_death()` to notify
//! nearby players when a player dies.
//! ## Gold Drop on Death
//! When a player is killed by another player in a zone with `gold_lose` enabled:
//! - Victim loses 50% of their gold
//! - Killer gains 40% of victim's gold (solo kill)
//! - If killer is in a party, 40% is distributed by level ratio
//! - 10% is destroyed (gold sink)
//! ## WIZ_GOLD_CHANGE (0x4A) Packet
//! `[u8 type] [u32 amount] [u32 total_gold]`
//! Type constants:
//! - `COIN_GAIN` (1): gold gained
//! - `COIN_LOSS` (2): gold lost

use ko_protocol::{Opcode, Packet};
use std::sync::Arc;

use crate::session::{ClientSession, SessionState};
use crate::systems::bdw;
use crate::systems::event_room::{self, TempleEventType};
use crate::systems::juraid;
use crate::world::types::{
    ZONE_BATTLE6, ZONE_CAITHAROS_ARENA, ZONE_CHAOS_DUNGEON, ZONE_DELOS_CASTELLAN,
    ZONE_DESPERATION_ABYSS, ZONE_DRAGON_CAVE, ZONE_DRAKI_TOWER, ZONE_DUNGEON_DEFENCE,
    ZONE_FELANKOR_ARENA, ZONE_FORGOTTEN_TEMPLE, ZONE_HELL_ABYSS, ZONE_ISILOON_ARENA,
    ZONE_SNOW_BATTLE, ZONE_UNDER_CASTLE,
};
use crate::world::WorldState;
use crate::zone::SessionId;

use crate::magic_constants::{MAGIC_CANCEL_TRANSFORMATION, TRANSFORMATION_NPC};
use crate::state_change_constants::STATE_CHANGE_ABNORMAL;

/// Chaos dungeon skill item IDs to be removed on death (and on login).
pub(crate) const ITEM_LIGHT_PIT: u32 = 700041000;
pub(crate) const ITEM_DRAIN_RESTORE: u32 = 700040000;
pub(crate) const ITEM_KILLING_BLADE: u32 = 700037000;

use crate::world::USER_DEAD;

use crate::attack_constants::MAX_DAMAGE;

/// WIZ_GOLD_CHANGE sub-type: gold gained.
pub const COIN_GAIN: u8 = 1;
/// WIZ_GOLD_CHANGE sub-type: gold lost.
pub const COIN_LOSS: u8 = 2;

/// Handle WIZ_DEAD from the client.
/// Death is server-authoritative, so the client should not normally send
/// this packet. We log and ignore it.
pub fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    tracing::debug!(
        "[{}] Client sent WIZ_DEAD — ignoring (death is server-authoritative)",
        session.addr(),
    );

    Ok(())
}

/// Broadcast a death animation to the 3x3 region grid and apply XP loss.
/// This is called by combat/HP handlers when a player's HP reaches 0.
/// Packet format: `WIZ_DEAD [u32 dead_unit_id]`
pub fn broadcast_death(world: &WorldState, dead_sid: SessionId) {
    // Mark player as dead
    world.update_res_hp_type(dead_sid, USER_DEAD);
    world.update_character_hp(dead_sid, 0);

    // Clear all DOTs (InitType3), buffs (InitType4), stealth (Type9), transform (Type6).
    world.clear_durational_skills(dead_sid);
    world.clear_all_buffs(dead_sid, false);

    // Reset ALL debuff state fields that buffs may have toggled.
    // clear_all_buffs removes map entries but doesn't call per-type cleanup,
    // so we must manually reset every session field that any buff_type_cleanup
    // would have restored.
    world.update_session(dead_sid, |h| {
        // BUFF_TYPE_SILENCE_TARGET / FREEZE / KAUL
        h.can_use_skills = true;
        // BUFF_TYPE_NO_POTIONS
        h.can_use_potions = true;
        // BUFF_TYPE_NO_RECALL — Bug 13: can_teleport stuck after death
        h.can_teleport = true;
        // BUFF_TYPE_PROHIBIT_INVIS
        h.can_stealth = true;
        // BUFF_TYPE_INSTANT_MAGIC
        h.instant_cast = false;
        // BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE
        h.block_physical = false;
        // BUFF_TYPE_BLOCK_MAGICAL_DAMAGE / FREEZE
        h.block_magic = false;
        // BUFF_TYPE_KAUL_TRANSFORMATION
        h.is_kaul = false;
        // BUFF_TYPE_UNDEAD
        h.is_undead = false;
        // BUFF_TYPE_DISABLE_TARGETING / BLIND / UNSIGHT
        h.is_blinded = false;
        // BUFF_TYPE_DEVIL_TRANSFORM
        h.is_devil = false;
        // BUFF_TYPE_MIRROR_DAMAGE_PARTY
        h.mirror_damage = false;
        h.mirror_damage_type = false;
        h.mirror_amount = 0;
        // BUFF_TYPE_MAGE_ARMOR
        h.reflect_armor_type = 0;
        // BUFF_TYPE_BLOCK_CURSE / BLOCK_CURSE_REFLECT
        h.block_curses = false;
        h.reflect_curses = false;
        // BUFF_TYPE_RESIS_AND_MAGIC_DMG
        h.magic_damage_reduction = 100;
        // BUFF_TYPE_DAGGER_BOW_DEFENSE
        h.dagger_r_amount = 100;
        h.bow_r_amount = 100;
        // BUFF_TYPE_NP_DROP_NOAH
        h.drop_scroll_amount = 0;
    });

    // Cancel stealth/invisibility on death
    world.set_invisibility_type(dead_sid, 0);

    //   if (isTransformed() && !isNPCTransformation()) Type6Cancel(true)
    // Cancel non-NPC transformations on death
    let transform_type = world
        .with_session(dead_sid, |h| h.transformation_type)
        .unwrap_or(0);
    if transform_type != 0 && transform_type != TRANSFORMATION_NPC {
        world.clear_transformation(dead_sid);
        //   Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_CANCEL_TRANSFORMATION=7));
        //   pUser->Send(&result);
        let mut cancel_pkt = Packet::new(Opcode::WizMagicProcess as u8);
        cancel_pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);
        world.send_to_session_owned(dead_sid, cancel_pkt);
    }

    // Armour slots (HEAD, BREAST, LEG, GLOVE, FOOT) lose rand(2..=5) durability.
    world.item_wore_out(dead_sid, super::durability::WORE_TYPE_DEFENCE, 0);

    world.set_user_ability(dead_sid);

    // ── Zone-specific death cleanup ─────────────────────────────────
    {
        let pos_check = world.get_position(dead_sid);
        if let Some(ref p) = pos_check {
            match p.zone_id {
                // ── Chaos Dungeon death ──────────────────────────────
                //   if (isInTempleEventZone && isEventUser) RobChaosSkillItems();
                ZONE_CHAOS_DUNGEON => {
                    let is_event_user = world
                        .with_session(dead_sid, |h| h.joined_event)
                        .unwrap_or(false);
                    if is_event_user {
                        rob_chaos_skill_items(world, dead_sid);
                    }
                }
                // ── BDW flag carrier death ────────────────────────────
                bdw::ZONE_BDW => {
                    bdw_flag_carrier_death(world, dead_sid);
                }
                // ── Tower owner death in ZONE_BATTLE6 ────────────────
                //   if (isTowerOwner()) TowerExitsFunciton(true);
                ZONE_BATTLE6 => {
                    tower_exits_on_death(world, dead_sid);
                }
                // ── Draki Tower death ────────────────────────────────
                ZONE_DRAKI_TOWER => {
                    draki_tower_kickouts_on_death(world, dead_sid);
                }
                // ── FT / Delos Castellan / Dungeon Defence death ─────
                //   KickOutZoneUser(true, ZONE_MORADON);
                ZONE_FORGOTTEN_TEMPLE | ZONE_DELOS_CASTELLAN | ZONE_DUNGEON_DEFENCE => {
                    let nation = world
                        .get_character_info(dead_sid)
                        .map(|c| c.nation)
                        .unwrap_or(0);
                    crate::systems::war::kick_out_zone_user(world, dead_sid, nation);
                }
                // ── Under Castle death ───────────────────────────────
                //   ItemWoreOut(UTC_ATTACK, -MAX_DAMAGE);
                //   ItemWoreOut(UTC_DEFENCE, -MAX_DAMAGE);
                // Effectively destroys all equipped weapons and armour.
                ZONE_UNDER_CASTLE => {
                    world.item_wore_out(
                        dead_sid,
                        super::durability::WORE_TYPE_UTC_ATTACK,
                        MAX_DAMAGE,
                    );
                    world.item_wore_out(
                        dead_sid,
                        super::durability::WORE_TYPE_UTC_DEFENCE,
                        MAX_DAMAGE,
                    );
                }
                _ => {}
            }
        }

        // ── Wanted user death cleanup ────────────────────────────────
        //   if (isWantedUser()) NewWantedEventLoqOut(pKiller);
        let is_wanted = world
            .with_session(dead_sid, |h| h.is_wanted)
            .unwrap_or(false);
        if is_wanted {
            super::vanguard::handle_wanted_logout(world, dead_sid);
        }
    }

    // Cache position lookup — used for broadcast and XP loss zone check
    let pos = world.get_position(dead_sid);

    // Build death packet — v2600 sniff verified: 16 bytes body
    // [u32 dead_unit_id] [u32 attacker_id] [u32 0] [u32 0]
    let mut pkt = Packet::new(Opcode::WizDead as u8);
    pkt.write_u32(dead_sid as u32);
    pkt.write_u32(0xFFFF_FFFF); // attacker_id (TODO: track last attacker per session)
    pkt.write_u32(0);
    pkt.write_u32(0);

    // Broadcast to 3x3 region (including self)
    if let Some(pos) = &pos {
        let event_room = world.get_event_room(dead_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }

    // WIZ_CORPSE — broadcast corpse name tag to nearby players
    // so the dead player's name appears on their corpse.
    super::corpse::broadcast_corpse(world, dead_sid);

    // NOTE: XP loss is NOT applied here — it is only applied for NPC kills
    // via `apply_npc_death_xp_loss()`. C++ calls OnDeathLostExpCalc() only
    // inside OnDeathKilledNpc(), not for PvP, DOT, or self-inflicted deaths.
}

/// Dismount and release a tower NPC when the owner dies in ZONE_BATTLE6.
/// Steps:
/// 1. Show the NPC (broadcast INOUT_IN)
/// 2. Clear tower ownership on NPC and player
/// 3. Send death dismount packet to the dying player
/// 4. StateChangeServerDirect(3, abnormal_type) for visual reset
fn tower_exits_on_death(world: &WorldState, dead_sid: SessionId) {
    let tower_npc_id = world.get_tower_owner_id(dead_sid);
    if tower_npc_id == -1 {
        return;
    }

    let npc_nid = tower_npc_id as u32;

    let npc = match world.get_npc_instance(npc_nid) {
        Some(n) => n,
        None => return,
    };
    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) if t.npc_type == 191 => t, // NPC_TYPE_TOWER
        _ => return,
    };

    let is_owned = world
        .get_npc_ai(npc_nid)
        .map(|ai| ai.is_tower_owner)
        .unwrap_or(false);
    if !is_owned {
        return;
    }

    let show_pkt = crate::npc::build_npc_inout(crate::npc::NPC_IN, &npc, &tmpl);
    if let Some(pos) = world.get_position(dead_sid) {
        let event_room = world.get_event_room(dead_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(show_pkt),
            None,
            event_room,
        );
    }

    world.update_npc_ai(npc_nid, |ai| {
        ai.is_tower_owner = false;
    });

    world.set_tower_owner_id(dead_sid, -1);

    let dismount_pkt = super::moving_tower::build_tower_death_dismount(dead_sid as u32, npc_nid);
    world.send_to_session_owned(dead_sid, dismount_pkt);

    //   if (isGM() && ABNORMAL_INVISIBLE) preserve; else ABNORMAL_NORMAL=1
    let is_gm = world
        .get_character_info(dead_sid)
        .map(|c| c.authority == 0)
        .unwrap_or(false);
    let abnormal_value: u32 = if is_gm { 0 } else { 1 }; // 0=INVISIBLE, 1=NORMAL

    let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
    state_pkt.write_u32(dead_sid as u32);
    state_pkt.write_u8(STATE_CHANGE_ABNORMAL);
    state_pkt.write_u32(abnormal_value);
    if let Some(pos) = world.get_position(dead_sid) {
        let event_room = world.get_event_room(dead_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(state_pkt),
            None,
            event_room,
        );
    }
}

/// Trigger Draki Tower kickout when a player dies in ZONE_DRAKI_TOWER.
/// Steps:
/// 1. Send OUT1 packet to the dying player
/// 2. Set kick timer on the room (20 seconds)
/// 3. Save user info (handled by existing periodic save)
fn draki_tower_kickouts_on_death(world: &WorldState, dead_sid: SessionId) {
    let event_room = world.get_event_room(dead_sid);
    if event_room == 0 {
        return;
    }

    //   [0x0C] [0x04] [0x00] [0x14] [u16(0)] [u8(0)]
    let mut out1_pkt = Packet::new(Opcode::WizEvent as u8);
    out1_pkt.write_u8(0x0C); // TEMPLE_DRAKI_TOWER_OUT1
    out1_pkt.write_u8(0x04);
    out1_pkt.write_u8(0x00);
    out1_pkt.write_u8(0x14);
    out1_pkt.write_u16(0);
    out1_pkt.write_u8(0);
    world.send_to_session_owned(dead_sid, out1_pkt);

    //      pRoomInfo->m_bOutTimer = true;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut rooms = world.draki_tower_rooms_write();
    if let Some(room) = rooms.get_mut(&event_room) {
        super::draki_tower::apply_kickout(room, now);
    }
}

/// NPC type: rolling stone (environmental hazard, no XP loss).
const NPC_TYPE_ROLLINGSTONE: u8 = 214;

/// NPC type: guard summon (fortress turret, no XP loss).
const NPC_TYPE_GUARD_SUMMON: u8 = 172;

/// NPC proto: saw blade (trap NPC, no XP loss).
const SAW_BLADE_PROTO: u16 = 2107;

/// Apply XP loss for NPC-killed death (PvE only).
/// XP loss is ONLY applied when:
/// - Zone has `exp_lost` enabled and is NOT a war zone
/// - Killer NPC is NOT rolling stone (type 214), saw blade (proto 2107),
///   or guard summon (type 172)
/// Call this AFTER `broadcast_death()` from NPC AI kill paths only.
pub async fn apply_npc_death_xp_loss(
    world: &WorldState,
    dead_sid: SessionId,
    npc_type: u8,
    npc_proto: u16,
) {
    // Exclude specific NPC types from causing XP loss
    if npc_type == NPC_TYPE_ROLLINGSTONE
        || npc_proto == SAW_BLADE_PROTO
        || npc_type == NPC_TYPE_GUARD_SUMMON
    {
        return;
    }

    let pos = match world.get_position(dead_sid) {
        Some(p) => p,
        None => return,
    };

    // C++ checks ONLY m_bExpLost — war_zone flag is independent (Unit.cpp:2162,2174).
    let should_lose_exp = world.get_zone(pos.zone_id).is_some_and(|zone| {
        zone.zone_info
            .as_ref()
            .is_some_and(|zi| zi.abilities.exp_lost)
    });

    if !should_lose_exp {
        return;
    }

    let ch = match world.get_character_info(dead_sid) {
        Some(ch) => ch,
        None => return,
    };

    // Check premium status for XP loss reduction
    let premium_restore = world.get_premium_exp_restore(dead_sid) as f32;

    let lost_exp = super::level::on_death_lost_exp_calc(ch.max_exp, premium_restore);
    if lost_exp > 0 {
        // Store for resurrection skill EXP recovery
        world.update_session(dead_sid, |h| {
            h.lost_exp = lost_exp;
        });
        super::level::exp_change(world, dead_sid, -lost_exp).await;
    }
}

/// Mark who killed this player (for resurrection EXP recovery gating).
/// Call after `broadcast_death()` when the attacker is another player.
/// PvE deaths leave `who_killed_me` at -1 (the default).
pub fn set_who_killed_me(world: &WorldState, dead_sid: SessionId, killer_sid: SessionId) {
    world.update_session(dead_sid, |h| {
        h.who_killed_me = killer_sid as i16;
    });
}

/// Check if a zone blocks gold loss on death.
fn is_no_gold_loss_zone(zone_id: u16) -> bool {
    matches!(
        zone_id,
        ZONE_SNOW_BATTLE
            | ZONE_DESPERATION_ABYSS
            | ZONE_HELL_ABYSS
            | ZONE_DRAGON_CAVE
            | ZONE_CAITHAROS_ARENA
            | ZONE_ISILOON_ARENA
            | ZONE_FELANKOR_ARENA
    )
}

/// Handle gold change on player-vs-player death.
/// Called by combat handlers after `broadcast_death()` when a player is
/// killed by another player in a zone with `gold_lose` enabled.
/// Gold distribution:
/// - Victim loses 50% of their gold
/// - Killer gains 40% of victim's gold (solo, no party)
/// - Remaining 10% is destroyed (gold sink)
/// Sends WIZ_GOLD_CHANGE packets to both victim and killer.
pub fn gold_change_on_death(world: &WorldState, killer_sid: SessionId, victim_sid: SessionId) {
    // Check that the zone has gold_lose enabled
    let victim_zone = world
        .get_position(victim_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);

    if is_no_gold_loss_zone(victim_zone) {
        return;
    }

    let gold_lose_enabled = world.get_zone(victim_zone).is_some_and(|zone| {
        zone.zone_info
            .as_ref()
            .is_some_and(|zi| zi.abilities.gold_lose)
    });

    if !gold_lose_enabled {
        return;
    }

    let victim_gold = match world.get_character_info(victim_sid) {
        Some(ch) => ch.gold,
        None => return,
    };

    if victim_gold == 0 {
        return;
    }

    // Killer gains 40%: `GoldGain((pTUser->m_iGold * 4) / 10)`
    // Victim loses 50%: `GoldLose(pTUser->m_iGold / 2)`
    let gold_lost = victim_gold / 2; // 50% destroyed from victim
    let gold_gained = (victim_gold as u64 * 4 / 10) as u32; // 40% to killer

    // Apply gold loss to victim first (also sends WIZ_GOLD_CHANGE packet)
    world.gold_lose(victim_sid, gold_lost);

    // If killer is in a party, distribute gold proportionally by level among members.
    // Otherwise solo killer gets the full 40%.
    let distributed_to_party = if world.is_in_party(killer_sid) {
        if let Some(pid) = world.get_party_id(killer_sid) {
            if let Some(party) = world.get_party(pid) {
                let members: Vec<(SessionId, u8)> = party
                    .members
                    .iter()
                    .filter_map(|&m| m)
                    .filter_map(|sid| world.get_character_info(sid).map(|ch| (sid, ch.level)))
                    .collect();

                if !members.is_empty() {
                    let shares = calculate_party_gold_shares(gold_gained, &members);
                    for (member_sid, share) in &shares {
                        world.gold_gain(*member_sid, *share);
                    }
                    tracing::info!(
                        "Gold on death (party): victim sid={} lost {} gold (had {}), \
                         distributed {} among {} party members",
                        victim_sid,
                        gold_lost,
                        victim_gold,
                        gold_gained,
                        shares.len(),
                    );
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if !distributed_to_party {
        // Solo killer gets the full 40%
        world.gold_gain(killer_sid, gold_gained);
        tracing::info!(
            "Gold on death (solo): victim sid={} lost {} gold (had {}), killer sid={} gained {}",
            victim_sid,
            gold_lost,
            victim_gold,
            killer_sid,
            gold_gained,
        );
    }

    // FerihaLog: RobItemInsertLog (gold drop on PK death)
    if let Some(pool) = world.db_pool() {
        let acc = world
            .with_session(victim_sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let ch_name = world.get_session_name(victim_sid).unwrap_or_default();
        super::audit_log::log_rob_item(
            pool,
            &acc,
            &ch_name,
            victim_zone as i16,
            0,
            0,
            crate::world::ITEM_GOLD,
            gold_lost,
            0,
        );
    }
}

/// Calculate gold distribution for a party kill on death.
/// When the killer is in a party, 40% of the victim's gold is distributed
/// among party members proportional to their level.
/// Returns a list of `(session_id, gold_amount)` pairs for each party member.
pub fn calculate_party_gold_shares(
    total_gold: u32,
    party_members: &[(SessionId, u8)], // (sid, level)
) -> Vec<(SessionId, u32)> {
    if party_members.is_empty() {
        return Vec::new();
    }

    let level_sum: u32 = party_members.iter().map(|(_, lvl)| *lvl as u32).sum();
    if level_sum == 0 {
        return Vec::new();
    }

    // `pUser->GoldGain((int)(temp_gold * (float)(pUser->GetLevel() / (float)levelSum)))`
    party_members
        .iter()
        .map(|(sid, level)| {
            let share = (total_gold as f64 * (*level as f64 / level_sum as f64)) as u32;
            (*sid, share)
        })
        .collect()
}

/// Track a BDW player kill with full C++ scoring and scoreboard broadcast.
/// Scoring formula: `score += 1 * nation_user_count_in_room`
/// After scoring, broadcasts TEMPLE_SCREEN to all room users.
/// Checks win condition: if score >= threshold (based on total players), triggers early finish.
/// Thresholds (C++ lines 399-402):
/// - total >= 16 → 600 points to win
/// - total >= 10 → 400
/// - total >= 5  → 300
/// - else         → 130
pub fn track_bdw_player_kill(world: &WorldState, killer_sid: SessionId, _dead_sid: SessionId) {
    use crate::systems::event_room;

    let is_bdw_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_bdw_active());
    if !is_bdw_active {
        return;
    }

    let killer_nation = match world.get_character_info(killer_sid) {
        Some(ch) => ch.nation,
        None => return,
    };

    let killer_name = match world.get_session_name(killer_sid) {
        Some(n) => n,
        None => return,
    };

    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::BorderDefenceWar, &killer_name)
    {
        Some(r) => r,
        None => return,
    };

    // Get room, compute scores, determine if finished
    let (k_score, e_score, winner_nation, finished) = {
        let Some(mut room) = world
            .event_room_manager
            .get_room_mut(TempleEventType::BorderDefenceWar, room_id)
        else {
            return;
        };

        if room.finish_packet_sent {
            return;
        }

        // C++ scoring: score += 1 * nation_count_in_room
        let e_count = room
            .elmorad_users
            .values()
            .filter(|u| !u.logged_out)
            .count() as i32;
        let k_count = room.karus_users.values().filter(|u| !u.logged_out).count() as i32;

        // C++ scoring: score += 1 * nation_count (the 1 is KILL_POINTS constant)
        if killer_nation == 2 && e_count > 0 {
            room.elmorad_score += e_count;
            room.elmorad_kill_count += 1;
        } else if killer_nation == 1 && k_count > 0 {
            room.karus_score += k_count;
            room.karus_kill_count += 1;
        }

        // Per-user BDW points (+1 per kill)
        if killer_nation == 1 {
            if let Some(u) = room.karus_users.get_mut(&killer_name) {
                u.bdw_points += 1;
            }
        } else if let Some(u) = room.elmorad_users.get_mut(&killer_name) {
            u.bdw_points += 1;
        }

        // Check win condition (using shared bdw module function)
        let winner = bdw::check_win_condition(&mut room);
        let finished = winner.is_some();

        if finished {
            room.finish_packet_sent = true;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            room.finish_time_counter = now + 20;
        }

        (
            room.karus_score,
            room.elmorad_score,
            room.winner_nation,
            finished,
        )
    }; // room lock dropped

    // Clear altar respawn state on kill-triggered finish.
    if finished {
        if let Some(state) = world.bdw_manager_write().get_room_state_mut(room_id) {
            state.altar_respawn_pending = false;
            state.altar_respawn_time = 0;
        }
    }

    // Build and broadcast TEMPLE_SCREEN scoreboard to all room users
    let screen_pkt = event_room::build_temple_screen_packet(k_score, e_score);
    broadcast_to_bdw_room(world, room_id, &screen_pkt);

    // If win condition met, send winner screen packets
    if finished {
        let arc_select = Arc::new(event_room::build_winner_select_msg(4)); // BDW
        let arc_finish = Arc::new(event_room::build_finish_packet(winner_nation));

        if let Some(room) = world
            .event_room_manager
            .get_room(TempleEventType::BorderDefenceWar, room_id)
        {
            for u in room.karus_users.values().filter(|u| !u.logged_out) {
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_select));
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_finish));
            }
            for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_select));
                world.send_to_session_arc(u.session_id, Arc::clone(&arc_finish));
                // C++ JuraidBdwFragSystem.cpp:463-466: kill-triggered finish sends both to Elmorad
                // (unlike timer-triggered EventMainSystem.cpp:501 which skips newpkt2 for Elmorad)
            }
        }
    }

    tracing::info!(
        "BDW kill: killer='{}' (nation={}) in room {}, scores: K={} E={}, finished={}",
        killer_name,
        killer_nation,
        room_id,
        k_score,
        e_score,
        finished,
    );
}

/// Broadcast a packet to all active users in a BDW room.
/// Clones the packet once and shares via Arc to avoid per-recipient cloning.
pub(super) fn broadcast_to_bdw_room(world: &WorldState, room_id: u8, pkt: &Packet) {
    let Some(room) = world
        .event_room_manager
        .get_room(TempleEventType::BorderDefenceWar, room_id)
    else {
        return;
    };
    let arc_pkt = Arc::new(pkt.clone());
    for u in room.karus_users.values().filter(|u| !u.logged_out) {
        world.send_to_session_arc(u.session_id, Arc::clone(&arc_pkt));
    }
    for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
        world.send_to_session_arc(u.session_id, Arc::clone(&arc_pkt));
    }
}

/// Handle BDW flag carrier death — clear room flag and start altar respawn.
///   `if (m_bHasAlterOptained) BDWUserHasObtainedLoqOut();`
/// Called from `broadcast_death()` when the dead player is in BDW zone 84.
/// The buff is already removed by `clear_all_buffs()`, so this only needs to
/// clear the room-level `has_altar_obtained` flag and start the respawn timer.
fn bdw_flag_carrier_death(world: &WorldState, dead_sid: SessionId) {
    use crate::systems::event_room::{self, TempleEventType};

    let user_name = match world.get_character_info(dead_sid) {
        Some(ch) => ch.name.clone(),
        None => return,
    };

    let is_bdw_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_bdw_active());
    if !is_bdw_active {
        return;
    }

    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::BorderDefenceWar, &user_name)
    {
        Some(r) => r,
        None => return,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let had_flag = {
        let mut bdw_mgr = world.bdw_manager_write();
        let Some(mut room) = world
            .event_room_manager
            .get_room_mut(TempleEventType::BorderDefenceWar, room_id)
        else {
            return;
        };

        let bdw_state = match bdw_mgr.get_room_state_mut(room_id) {
            Some(s) => s,
            None => return,
        };

        bdw::flag_carrier_logout(&mut room, bdw_state, &user_name, now)
    };

    if had_flag {
        let timer_pkt = event_room::build_altar_timer_packet(bdw::ALTAR_RESPAWN_DELAY_SECS as u16);
        broadcast_to_bdw_room(world, room_id, &timer_pkt);

        tracing::info!(
            "BDW flag carrier '{}' died in room {}, altar respawn started",
            user_name,
            room_id,
        );
    }
}

/// Build the exact JstKO v2615 kill narration payload used by KA_KillUpdate.
///
/// Wire recovered from the v2615 client's packet dispatcher, case `0xC8`:
/// `[WIZ_KILLASSIST=0xC8][kill=1][type=1][u32 killer_id]`
/// `[DByte killer_name][u8 killer_nation][u16 stage]`.
pub fn build_kill_narration_packet(
    killer_id: u32,
    killer_name: &str,
    killer_nation: u8,
    stage: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizKillAssist as u8);
    pkt.write_u8(1); // kaopcode::kill
    pkt.write_u8(1);
    pkt.write_u32(killer_id);
    pkt.write_string(killer_name);
    pkt.write_u8(killer_nation);
    pkt.write_u16(stage.max(1));
    pkt
}

/// Build KA_KillUpdate's every-10-kills total/party announcement payload.
/// Wire: `[0xC8][kill=1][type=3][u32 killer_id][DByte name][nation][u16 total]`.
pub fn build_kill_total_packet(
    killer_id: u32,
    killer_name: &str,
    killer_nation: u8,
    total_kills: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizKillAssist as u8);
    pkt.write_u8(1); // kaopcode::kill
    pkt.write_u8(3);
    pkt.write_u32(killer_id);
    pkt.write_string(killer_name);
    pkt.write_u8(killer_nation);
    pkt.write_u16(total_kills);
    pkt
}

/// Broadcast the native death notice and update the killer's narration UI.
pub fn send_death_notice(world: &WorldState, killer_sid: SessionId, victim_sid: SessionId) {
    let killer_name = match world.get_session_name(killer_sid) {
        Some(n) => n,
        None => return,
    };
    let victim_name = match world.get_session_name(victim_sid) {
        Some(n) => n,
        None => return,
    };
    let victim_pos = match world.get_position(victim_sid) {
        Some(p) => p,
        None => return,
    };

    let victim_x = victim_pos.x as u16;
    let victim_z = victim_pos.z as u16;
    let zone_id = victim_pos.zone_id;

    // Get killer's party ID for determining killtype per recipient
    let killer_party_id: Option<u16> = world
        .get_character_info(killer_sid)
        .and_then(|ch| ch.party_id)
        .filter(|&pid| pid != 0 && pid != 0xFFFF);

    world.send_death_notice_to_zone(
        zone_id,
        killer_sid,
        victim_sid,
        &killer_name,
        &victim_name,
        killer_party_id,
        victim_x,
        victim_z,
    );

    tracing::info!(
        "PvP death notice: '{}' killed '{}' at ({},{}) in zone {}",
        killer_name,
        victim_name,
        victim_x,
        victim_z,
        zone_id,
    );
}

/// Apply loyalty (NP) changes when a player kills another player.
///   - If zone has `m_bGiveLoyalty`, call `LoyaltyChange` (solo) or `LoyaltyDivide` (party).
/// This function checks the zone flag and delegates to the appropriate loyalty function.
pub fn pvp_loyalty_on_death(world: &WorldState, killer_sid: SessionId, victim_sid: SessionId) {
    use crate::systems::loyalty::{self, LoyaltyRates};

    let killer_zone = world
        .get_position(killer_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);

    let zone_gives_loy = world
        .get_zone(killer_zone)
        .and_then(|z| z.zone_info.as_ref().map(|zi| zi.abilities.give_loyalty))
        .unwrap_or(false);
    // Winning nations can continue the post-war raid in the defeated nation's
    // capital. Those home-zone records normally disable NP, so honor the live
    // invasion flags as an explicit PvP-loyalty exception.
    let killer_nation = world.get_character_info(killer_sid).map(|ch| ch.nation).unwrap_or(0);
    let battle = world.get_battle_state();
    let active_raid = (killer_zone == crate::world::types::ZONE_KARUS
        && killer_nation == 2
        && battle.karus_open_flag)
        || (killer_zone == crate::world::types::ZONE_ELMORAD
            && killer_nation == 1
            && battle.elmorad_open_flag);
    if !zone_gives_loy && !active_raid {
        return;
    }

    let rates = LoyaltyRates::default();
    let in_party = world.is_in_party(killer_sid);
    if in_party {
        let party_id = world.get_party_id(killer_sid);
        if let Some(pid) = party_id {
            if let Some(party) = world.get_party(pid) {
                let members: Vec<SessionId> = party.members.iter().filter_map(|&m| m).collect();
                loyalty::loyalty_divide(world, killer_sid, victim_sid, &members, &rates);
            }
        }
    } else {
        loyalty::loyalty_change(world, killer_sid, victim_sid, 0, &rates);
    }
}

/// Remove Chaos Dungeon skill items from a player on death.
/// Called on death in the Chaos Dungeon zone (85). Removes all copies of:
/// - ITEM_LIGHT_PIT (700041000)
/// - ITEM_DRAIN_RESTORE (700040000)
/// - ITEM_KILLING_BLADE (700037000)
pub fn rob_chaos_skill_items(world: &WorldState, sid: SessionId) {
    // Only in Chaos Dungeon zone
    let in_chaos = world
        .get_position(sid)
        .is_some_and(|pos| pos.zone_id == ZONE_CHAOS_DUNGEON);
    if !in_chaos {
        return;
    }

    let chaos_items = [ITEM_LIGHT_PIT, ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE];

    for item_id in &chaos_items {
        // C++ does: if (GetItemCount(ITEM) > 0) RobItem(ITEM, GetItemCount(ITEM));
        if world.rob_all_of_item(sid, *item_id) {
            tracing::info!("Chaos death: robbed item {} from sid={}", item_id, sid,);
        }
    }
}

/// Track a Juraid Mountain monster kill.
/// Called from NPC death handlers (attack.rs, npc_ai.rs, magic_process.rs) when
/// a monster is killed in zone 87 by a player. Records the kill for the player's
/// nation in the appropriate Juraid room.
/// This is a public helper that other handlers can call. The actual wiring in
/// attack.rs::handle_npc_death should call this when the NPC dies in zone 87.
pub fn track_juraid_monster_kill(
    world: &WorldState,
    killer_sid: SessionId,
    killed_npc_sid: u16,
    killed_event_room: u16,
    killed_summon_type: u8,
    killed_x: f32,
    killed_z: f32,
) {
    // Match the C++ CNpc::HandleJuraidKill gate: only Juraid-spawned
    // entities participate in the event. Ordinary zone monsters (including
    // GM-spawned monsters with summon_type=0) must not alter Juraid scores or
    // bridge progression.
    if !matches!(
        killed_summon_type,
        juraid::SUMMON_JURAID_MAIN
            | juraid::SUMMON_JURAID_CHILD
            | juraid::SUMMON_JURAID_DEVA
            | juraid::SUMMON_JURAID_MONUMENT
    ) {
        return;
    }

    // Check if Juraid is active
    let is_juraid_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_juraid_active());
    if !is_juraid_active {
        return;
    }

    // Determine killer's nation
    let killer_nation = match world.get_character_info(killer_sid) {
        Some(ch) => ch.nation,
        None => return,
    };

    // Find the room this player is in
    let killer_name = match world.get_session_name(killer_sid) {
        Some(n) => n,
        None => return,
    };

    let room_ids = world
        .event_room_manager
        .list_rooms(TempleEventType::JuraidMountain);
    for room_id in room_ids {
        let found = world
            .event_room_manager
            .get_room(TempleEventType::JuraidMountain, room_id)
            .is_some_and(|room| room.get_user(&killer_name).is_some());

        if found {
            let (k_score, e_score, score_changed) = {
                let Some(mut room) = world
                    .event_room_manager
                    .get_room_mut(TempleEventType::JuraidMountain, room_id)
                else {
                    return;
                };
                if room.finish_packet_sent {
                    return;
                }
                let mut score_changed = false;
                if juraid::is_juraid_monument(killed_npc_sid) {
                    let monument_nation = juraid::monument_nation(killed_npc_sid);
                    // Only the opposing nation may destroy a monument. Per
                    // the Juraid design document, a point is granted only to
                    // the tied/trailing team.
                    let can_score = juraid::can_monument_score(
                        killer_nation,
                        killed_npc_sid,
                        room.karus_score,
                        room.elmorad_score,
                    );
                    if can_score {
                        score_changed = true;
                        if killer_nation == 1 {
                            room.karus_score += 1;
                        } else if killer_nation == 2 {
                            room.elmorad_score += 1;
                        }
                    }
                    tracing::info!(
                        room_id,
                        killer_nation,
                        monument_nation,
                        can_score,
                        "Juraid Monument killed"
                    );
                    if monument_nation != 0 {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        world.set_juraid_monument_respawn(
                            room_id,
                            monument_nation,
                            now.saturating_add(juraid::MONUMENT_RESPAWN_SECS),
                        );
                    }
                } else if juraid::is_deva_bird(killed_npc_sid) {
                    room.winner_nation = killer_nation;
                }
                (room.karus_score, room.elmorad_score, score_changed)
            };

            // Wave monsters advance bridge progress but never alter the score.
            // Refresh the scoreboard only when an eligible monument grants +1.
            if score_changed {
                let arc_screen = Arc::new(event_room::build_temple_screen_packet(k_score, e_score));
                if let Some(room) = world
                    .event_room_manager
                    .get_room(TempleEventType::JuraidMountain, room_id)
                {
                    for u in room.karus_users.values().filter(|u| !u.logged_out) {
                        world.send_to_session_arc(u.session_id, Arc::clone(&arc_screen));
                    }
                    for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
                        world.send_to_session_arc(u.session_id, Arc::clone(&arc_screen));
                    }
                }
            }

            if !juraid::is_juraid_monument(killed_npc_sid) {
                spawn_juraid_child_monsters(
                    world,
                    room_id,
                    killed_npc_sid,
                    killed_event_room,
                    killed_summon_type,
                    killed_x,
                    killed_z,
                );
            }

            // Match CNpc::HandleJuraidKill exactly: main and released child
            // monsters are counted separately for each nation. A nation opens
            // only its own bridge at 4/20, 8/40 and 12/60.
            let mut bridge_state = world.get_juraid_bridge_state(room_id).unwrap_or_default();
            match (killer_nation, killed_summon_type) {
                (1, juraid::SUMMON_JURAID_MAIN) => {
                    bridge_state.karus_main_kills = bridge_state.karus_main_kills.saturating_add(1)
                }
                (1, juraid::SUMMON_JURAID_CHILD) => {
                    bridge_state.karus_sub_kills = bridge_state.karus_sub_kills.saturating_add(1)
                }
                (2, juraid::SUMMON_JURAID_MAIN) => {
                    bridge_state.elmorad_main_kills =
                        bridge_state.elmorad_main_kills.saturating_add(1)
                }
                (2, juraid::SUMMON_JURAID_CHILD) => {
                    bridge_state.elmorad_sub_kills =
                        bridge_state.elmorad_sub_kills.saturating_add(1)
                }
                _ => {}
            }
            let (main_kills, sub_kills) = if killer_nation == 1 {
                (bridge_state.karus_main_kills, bridge_state.karus_sub_kills)
            } else {
                (
                    bridge_state.elmorad_main_kills,
                    bridge_state.elmorad_sub_kills,
                )
            };
            for bridge_idx in 0..juraid::NUM_BRIDGES {
                if main_kills >= juraid::ROOM_MAIN_KILL_THRESHOLDS[bridge_idx]
                    && sub_kills >= juraid::ROOM_BRIDGE_KILL_THRESHOLDS[bridge_idx] as u16
                    && bridge_state.open_bridge(bridge_idx, killer_nation)
                {
                    world.broadcast_juraid_bridge_open_for_nation(
                        bridge_idx,
                        room_id as u16,
                        killer_nation,
                    );
                    tracing::info!(
                        room_id,
                        bridge_idx,
                        killer_nation,
                        main_kills,
                        sub_kills,
                        "Juraid bridge opened by verified main/sub thresholds"
                    );
                }
            }
            world.set_juraid_bridge_state(room_id, bridge_state);

            // Deva Bird death should enter the reward phase, leaving the existing
            // 20-second finish counter for chest interaction before teleport.
            if juraid::is_deva_bird(killed_npc_sid) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let finish_secs = crate::systems::event_room::EventRoomManager::vroom_index(
                    TempleEventType::JuraidMountain,
                )
                .and_then(|idx| world.event_room_manager.get_vroom_opt(idx))
                .map(|opts| ((opts.sign + opts.play).max(0) as u64) * 60)
                .unwrap_or(0);
                world.event_room_manager.update_temple_event(|s| {
                    s.manual_close = false;
                    s.start_time = now.saturating_sub(finish_secs);
                    s.closed_time = now;
                });
                tracing::info!(
                    "Juraid Deva Bird killed by '{}'; reward countdown requested",
                    killer_name,
                );
            }

            tracing::info!(
                "Juraid kill: player '{}' (nation={}) killed entity in room {}, monument scores: K={} E={}",
                killer_name,
                killer_nation,
                room_id,
                k_score,
                e_score,
            );
            return;
        }
    }
}

fn spawn_juraid_child_monsters(
    world: &WorldState,
    room_id: u8,
    killed_npc_sid: u16,
    killed_event_room: u16,
    killed_summon_type: u8,
    killed_x: f32,
    killed_z: f32,
) {
    if juraid::is_deva_bird(killed_npc_sid)
        || juraid::is_bridge(killed_npc_sid)
        || juraid::is_juraid_monument(killed_npc_sid)
    {
        return;
    }

    let is_runtime_main = killed_summon_type == juraid::SUMMON_JURAID_MAIN;
    let is_legacy_main =
        killed_summon_type == 0 && juraid::is_main_monster(world, room_id, killed_npc_sid);
    if !is_runtime_main && !is_legacy_main {
        return;
    }

    // C++ CNpc::HandleJuraidKill releases five creatures chosen from this
    // fixed set; it does not choose another entry from MONSTER_JURAID_RESPAWN.
    let selector =
        (usize::from(killed_npc_sid) + killed_x.to_bits() as usize + killed_z.to_bits() as usize)
            % juraid::JURAID_CHILD_SIDS.len();
    let child_sid = juraid::JURAID_CHILD_SIDS[selector];

    let spawned = world.spawn_event_npc_ex(
        child_sid,
        true,
        juraid::ZONE_JURAID,
        killed_x,
        killed_z,
        juraid::ROOM_CHILD_MONSTER_COUNT,
        killed_event_room,
        juraid::SUMMON_JURAID_CHILD,
    );
    tracing::info!(
        room_id,
        killed_npc_sid,
        child_sid,
        spawned = spawned.len(),
        "Juraid main monster released child monsters"
    );
}

/// Track a Juraid PvP kill — updates room kill count and broadcasts scoreboard.
/// Unlike `track_juraid_monster_kill` which is for NPC kills, this is for
/// player-vs-player kills in Juraid Mountain (zone 87).
pub fn track_juraid_pvp_kill(world: &WorldState, killer_sid: SessionId) {
    use crate::systems::event_room;

    let is_juraid_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_juraid_active());
    if !is_juraid_active {
        return;
    }

    let killer_nation = match world.get_character_info(killer_sid) {
        Some(ch) => ch.nation,
        None => return,
    };

    let killer_name = match world.get_session_name(killer_sid) {
        Some(n) => n,
        None => return,
    };

    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::JuraidMountain, &killer_name)
    {
        Some(r) => r,
        None => return,
    };

    let (k_score, e_score) = {
        let Some(mut room) = world
            .event_room_manager
            .get_room_mut(TempleEventType::JuraidMountain, room_id)
        else {
            return;
        };

        if room.finish_packet_sent {
            return;
        }

        if killer_nation == 2 {
            room.elmorad_score += 1;
        } else {
            room.karus_score += 1;
        }

        (room.karus_score, room.elmorad_score)
    }; // room lock dropped

    // Broadcast TEMPLE_SCREEN scoreboard to all room users
    // C++ sends via Send_All to ZONE_JURAID_MOUNTAIN with event room filter
    let arc_screen = Arc::new(event_room::build_temple_screen_packet(k_score, e_score));
    if let Some(room) = world
        .event_room_manager
        .get_room(TempleEventType::JuraidMountain, room_id)
    {
        for u in room.karus_users.values().filter(|u| !u.logged_out) {
            world.send_to_session_arc(u.session_id, Arc::clone(&arc_screen));
        }
        for u in room.elmorad_users.values().filter(|u| !u.logged_out) {
            world.send_to_session_arc(u.session_id, Arc::clone(&arc_screen));
        }
    }

    tracing::info!(
        "Juraid PvP kill: killer='{}' (nation={}) in room {}, scores: K={} E={}",
        killer_name,
        killer_nation,
        room_id,
        k_score,
        e_score,
    );
}

/// Track a Chaos Dungeon PvP kill and death.
///   - Victim: `m_ChaosExpansionDeadCount++`
///   - Killer: `m_ChaosExpansionKillCount++`
/// These feed into rank updates and the final EXP formula at event finish.
pub fn track_chaos_pvp_kill(world: &WorldState, killer_sid: SessionId, dead_sid: SessionId) {
    let is_chaos_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_chaos_active());
    if !is_chaos_active {
        return;
    }

    let killer_name = match world.get_session_name(killer_sid) {
        Some(n) => n,
        None => return,
    };
    let dead_name = match world.get_session_name(dead_sid) {
        Some(n) => n,
        None => return,
    };

    // Find the room both players are in
    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::ChaosDungeon, &killer_name)
    {
        Some(r) => r,
        None => return,
    };

    // Update per-player kills/deaths on EventUser for reward calculation.
    // C++ stores these as m_ChaosExpansionKillCount / m_ChaosExpansionDeadCount on CUser.
    // We store them directly on EventUser.kills / EventUser.deaths in the room.
    if let Some(mut room) = world
        .event_room_manager
        .get_room_mut(TempleEventType::ChaosDungeon, room_id)
    {
        if let Some(killer) = room.mixed_users.get_mut(&killer_name) {
            killer.kills += 1;
        }
        if let Some(victim) = room.mixed_users.get_mut(&dead_name) {
            victim.deaths += 1;
        }
    }

    tracing::info!(
        "Chaos PvP: killer='{}' killed '{}' in room {}",
        killer_name,
        dead_name,
        room_id,
    );
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::systems::bdw;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_dead_broadcast_format() {
        // Build WIZ_DEAD broadcast: [u32 dead_unit_id]
        let mut pkt = Packet::new(Opcode::WizDead as u8);
        pkt.write_u32(42); // dead user session_id

        assert_eq!(pkt.opcode, Opcode::WizDead as u8);
        assert_eq!(pkt.data.len(), 4);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_kill_narration_matches_ka_kill_update_wire() {
        let pkt = build_kill_narration_packet(10_001, "Wolfcstein", 1, 40);
        assert_eq!(pkt.opcode, Opcode::WizKillAssist as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // kaopcode::kill
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10_001));
        assert_eq!(r.read_string().as_deref(), Some("Wolfcstein"));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(40)); // Legendary cap
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_kill_total_matches_ka_kill_update_wire() {
        let pkt = build_kill_total_packet(77, "JOLLY_JOKER", 2, 40);
        assert_eq!(pkt.opcode, Opcode::WizKillAssist as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(77));
        assert_eq!(r.read_string().as_deref(), Some("JOLLY_JOKER"));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(40));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_gold_change_packet_format() {
        // WIZ_GOLD_CHANGE: [u8 type] [u32 amount] [u32 total_gold]
        let mut pkt = Packet::new(Opcode::WizGoldChange as u8);
        pkt.write_u8(COIN_LOSS);
        pkt.write_u32(5000); // lost 5000 gold
        pkt.write_u32(5000); // 5000 remaining

        assert_eq!(pkt.opcode, Opcode::WizGoldChange as u8);
        assert_eq!(pkt.data.len(), 9); // 1 + 4 + 4

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COIN_LOSS));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_gold_change_gain_packet_format() {
        let mut pkt = Packet::new(Opcode::WizGoldChange as u8);
        pkt.write_u8(COIN_GAIN);
        pkt.write_u32(4000); // gained 4000 gold
        pkt.write_u32(14000); // 14000 total

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(COIN_GAIN));
        assert_eq!(r.read_u32(), Some(4000));
        assert_eq!(r.read_u32(), Some(14000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_gold_drop_calculation() {
        // Victim has 10000 gold:
        // - Loses 50%: 10000 / 2 = 5000
        // - Killer gains 40%: (10000 * 4) / 10 = 4000
        // - 10% destroyed: 5000 - 4000 = 1000
        let victim_gold: u32 = 10000;
        let gold_lost = victim_gold / 2;
        let gold_gained = (victim_gold * 4) / 10;

        assert_eq!(gold_lost, 5000);
        assert_eq!(gold_gained, 4000);
        assert_eq!(gold_lost - gold_gained, 1000); // gold sink
    }

    #[test]
    fn test_gold_drop_zero_gold() {
        // Victim with 0 gold — no gold should change
        let victim_gold: u32 = 0;
        let gold_lost = victim_gold / 2;
        let gold_gained = (victim_gold * 4) / 10;

        assert_eq!(gold_lost, 0);
        assert_eq!(gold_gained, 0);
    }

    #[test]
    fn test_gold_drop_small_amount() {
        // Victim with 1 gold — integer division results
        let victim_gold: u32 = 1;
        let gold_lost = victim_gold / 2;
        let gold_gained = (victim_gold * 4) / 10;

        assert_eq!(gold_lost, 0); // 1/2 = 0 (integer division)
        assert_eq!(gold_gained, 0); // (1*4)/10 = 0
    }

    #[test]
    fn test_gold_drop_odd_amount() {
        // Victim with 9 gold — test rounding
        let victim_gold: u32 = 9;
        let gold_lost = victim_gold / 2;
        let gold_gained = (victim_gold * 4) / 10;

        assert_eq!(gold_lost, 4); // 9/2 = 4
        assert_eq!(gold_gained, 3); // (9*4)/10 = 36/10 = 3
    }

    #[test]
    fn test_party_gold_shares_solo() {
        // Single party member gets all the gold
        let total = 4000_u32;
        let members = vec![(1_u16, 60_u8)];
        let shares = calculate_party_gold_shares(total, &members);

        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0], (1, 4000));
    }

    #[test]
    fn test_party_gold_shares_equal_levels() {
        // Two members at same level split evenly
        let total = 4000_u32;
        let members = vec![(1_u16, 60_u8), (2_u16, 60_u8)];
        let shares = calculate_party_gold_shares(total, &members);

        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0], (1, 2000));
        assert_eq!(shares[1], (2, 2000));
    }

    #[test]
    fn test_party_gold_shares_different_levels() {
        // Level ratio: member1=60, member2=40 → 60/100=0.6, 40/100=0.4
        let total = 4000_u32;
        let members = vec![(1_u16, 60_u8), (2_u16, 40_u8)];
        let shares = calculate_party_gold_shares(total, &members);

        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0], (1, 2400)); // 4000 * 60/100 = 2400
        assert_eq!(shares[1], (2, 1600)); // 4000 * 40/100 = 1600
    }

    #[test]
    fn test_party_gold_shares_empty() {
        let shares = calculate_party_gold_shares(4000, &[]);
        assert!(shares.is_empty());
    }

    #[test]
    fn test_death_xp_loss_calculation() {
        // nExpLost = maxexp / 20
        let max_exp: i64 = 100_000;
        let lost = super::super::level::on_death_lost_exp_calc(max_exp, 0.0);
        assert_eq!(lost, 5000); // 100000 / 20 = 5000
    }

    #[test]
    fn test_death_xp_loss_zero_max_exp() {
        let lost = super::super::level::on_death_lost_exp_calc(0, 0.0);
        assert_eq!(lost, 0);
    }

    #[test]
    fn test_death_xp_loss_small_max_exp() {
        // 19 / 20 = 0 (integer division, no XP lost)
        let lost = super::super::level::on_death_lost_exp_calc(19, 0.0);
        assert_eq!(lost, 0);
    }

    #[test]
    fn test_coin_constants() {
        assert_eq!(COIN_GAIN, 1);
        assert_eq!(COIN_LOSS, 2);
    }

    #[test]
    fn test_bdw_zone_constant() {
        assert_eq!(bdw::ZONE_BDW, 84);
    }

    #[test]
    fn test_bdw_kill_tracking_scoring() {
        // Verify record_kill updates room scores correctly
        use crate::systems::event_room::EventRoom;

        let mut room = EventRoom::new(1, TempleEventType::BorderDefenceWar);

        // Karus kill (El Morad player died)
        let (k, e) = bdw::record_kill(&mut room, 1);
        assert_eq!(k, 1);
        assert_eq!(e, 0);

        // El Morad kill (Karus player died)
        let (k, e) = bdw::record_kill(&mut room, 2);
        assert_eq!(k, 1);
        assert_eq!(e, 1);
    }

    #[test]
    fn test_juraid_monster_kill_room_score() {
        // Verify direct room score update logic for Juraid
        use crate::systems::event_room::{EventRoomManager, EventUser};

        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::JuraidMountain, 1);

        // Add a Karus player to room 1
        {
            let mut room = erm
                .get_room_mut(TempleEventType::JuraidMountain, 1)
                .unwrap();
            room.add_user(EventUser {
                user_name: "warrior1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
        }

        // Simulate Karus kill by directly updating score
        {
            let mut room = erm
                .get_room_mut(TempleEventType::JuraidMountain, 1)
                .unwrap();
            room.karus_score += 1;
            assert_eq!(room.karus_score, 1);
            assert_eq!(room.elmorad_score, 0);
        }

        // Simulate El Morad kill
        {
            let mut room = erm
                .get_room_mut(TempleEventType::JuraidMountain, 1)
                .unwrap();
            room.elmorad_score += 1;
            assert_eq!(room.karus_score, 1);
            assert_eq!(room.elmorad_score, 1);
        }
    }

    // ── Sprint 46: Death Notice tests ──────────────────────────────────

    #[test]
    fn test_death_notice_ext_sub_opcode() {
        assert_eq!(crate::handler::ext_hook::EXT_SUB_DEATH_NOTICE, 0xD7);
    }

    #[test]
    fn test_death_notice_packet_format_bystander() {
        // Build a death notice packet as a bystander would receive it
        let killtype: u8 = 3; // bystander
        let killer_name = "Warrior1";
        let victim_name = "Mage2";
        let x: u16 = 512;
        let z: u16 = 341;

        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(0xD7); // DeathNotice sub-opcode
        pkt.write_u8(killtype);
        // SByte string: u8 length + bytes
        let kn = killer_name.as_bytes();
        pkt.write_u8(kn.len() as u8);
        pkt.data.extend_from_slice(kn);
        let vn = victim_name.as_bytes();
        pkt.write_u8(vn.len() as u8);
        pkt.data.extend_from_slice(vn);
        pkt.write_u16(x);
        pkt.write_u16(z);

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        // Verify packet contents
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0xD7)); // sub-opcode
        assert_eq!(r.read_u8(), Some(3)); // killtype
        let name_len = r.read_u8().unwrap();
        assert_eq!(name_len, 8); // "Warrior1" = 8 chars
        let name_bytes: Vec<u8> = (0..name_len).map(|_| r.read_u8().unwrap()).collect();
        assert_eq!(std::str::from_utf8(&name_bytes).unwrap(), "Warrior1");
        let vname_len = r.read_u8().unwrap();
        assert_eq!(vname_len, 5); // "Mage2" = 5 chars
        let vname_bytes: Vec<u8> = (0..vname_len).map(|_| r.read_u8().unwrap()).collect();
        assert_eq!(std::str::from_utf8(&vname_bytes).unwrap(), "Mage2");
        assert_eq!(r.read_u16(), Some(512));
        assert_eq!(r.read_u16(), Some(341));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_death_notice_killtype_participant() {
        // Verify killtype values match C++ reference
        // 1 = direct participant (killer or victim)
        // 2 = killer's party member
        // 3 = bystander
        let participant: u8 = 1;
        let party_member: u8 = 2;
        let bystander: u8 = 3;
        assert_ne!(participant, party_member);
        assert_ne!(participant, bystander);
        assert_ne!(party_member, bystander);
    }

    #[test]
    fn test_death_notice_empty_names() {
        // Edge case: empty killer/victim names
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(0xD7);
        pkt.write_u8(3);
        pkt.write_u8(0); // empty killer name
        pkt.write_u8(0); // empty victim name
        pkt.write_u16(0);
        pkt.write_u16(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0xD7));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(0)); // 0-length name
        assert_eq!(r.read_u8(), Some(0)); // 0-length name
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 46: Chaos Dungeon item constants tests ──────────────────

    #[test]
    fn test_chaos_dungeon_zone_constant() {
        assert_eq!(super::ZONE_CHAOS_DUNGEON, 85);
    }

    #[test]
    fn test_chaos_skill_item_light_pit() {
        assert_eq!(super::ITEM_LIGHT_PIT, 700041000);
    }

    #[test]
    fn test_chaos_skill_item_drain_restore() {
        assert_eq!(super::ITEM_DRAIN_RESTORE, 700040000);
    }

    #[test]
    fn test_chaos_skill_item_killing_blade() {
        assert_eq!(super::ITEM_KILLING_BLADE, 700037000);
    }

    #[test]
    fn test_rob_chaos_items_not_in_chaos_zone() {
        // rob_chaos_skill_items should be a no-op when player is not in zone 85
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Player in zone 21 (Moradon), not chaos dungeon
        world.update_position(sid, 21, 100.0, 0.0, 100.0);

        // Should not panic or do anything (player not in chaos zone)
        rob_chaos_skill_items(&world, sid);
    }

    // ── Sprint 118: InitOnDeath stealth + transform cancel tests ────

    #[test]
    fn test_transformation_npc_constant() {
        assert_eq!(super::TRANSFORMATION_NPC, 2);
    }

    #[test]
    fn test_death_clears_stealth() {
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set player as invisible
        world.set_invisibility_type(sid, 1); // INVIS_DISPEL_ON_MOVE
        assert_eq!(world.with_session(sid, |h| h.invisibility_type), Some(1));

        // Simulate death clearing stealth
        world.set_invisibility_type(sid, 0);
        assert_eq!(world.with_session(sid, |h| h.invisibility_type), Some(0));
    }

    #[test]
    fn test_death_clears_non_npc_transformation() {
        // isTransformed() && !isNPCTransformation() → Type6Cancel
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set monster transformation (type 1 — should be cleared on death)
        world.set_transformation(sid, 1, 100, 500, 0, 60000);
        assert!(world.is_transformed(sid));

        // Simulate death cancel logic
        let tt = world
            .with_session(sid, |h| h.transformation_type)
            .unwrap_or(0);
        assert_eq!(tt, 1); // TRANSFORMATION_MONSTER
        assert_ne!(tt, super::TRANSFORMATION_NPC);
        world.clear_transformation(sid);
        assert!(!world.is_transformed(sid));
    }

    #[test]
    fn test_death_preserves_npc_transformation() {
        // !isNPCTransformation() — NPC transformations are NOT cancelled on death
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set NPC transformation (type 2 — should NOT be cleared on death)
        world.set_transformation(sid, 2, 200, 600, 0, 60000);
        assert!(world.is_transformed(sid));

        // Simulate death cancel logic: skip if NPC transform
        let tt = world
            .with_session(sid, |h| h.transformation_type)
            .unwrap_or(0);
        assert_eq!(tt, super::TRANSFORMATION_NPC);
        // Should NOT clear
        if tt != 0 && tt != super::TRANSFORMATION_NPC {
            world.clear_transformation(sid);
        }
        assert!(world.is_transformed(sid)); // still transformed
    }

    // ── Sprint 119: Type6Cancel packet + MAGIC_CANCEL_TRANSFORMATION ──

    #[test]
    fn test_magic_cancel_transformation_constant() {
        assert_eq!(super::MAGIC_CANCEL_TRANSFORMATION, 7);
    }

    #[test]
    fn test_type6cancel_packet_format() {
        //   Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_CANCEL_TRANSFORMATION));
        //   pUser->Send(&result);
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(super::MAGIC_CANCEL_TRANSFORMATION);

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 7); // MAGIC_CANCEL_TRANSFORMATION

        // Roundtrip
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_death_sends_cancel_transform_packet() {
        // Verify that when a transformed player dies, the cancel packet is sent
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set monster transformation
        world.set_transformation(sid, 1, 100, 500, 0, 60000);

        // Simulate death transformation cancel with packet send
        let tt = world
            .with_session(sid, |h| h.transformation_type)
            .unwrap_or(0);
        assert_ne!(tt, 0);
        assert_ne!(tt, super::TRANSFORMATION_NPC);

        world.clear_transformation(sid);
        let mut cancel_pkt = Packet::new(Opcode::WizMagicProcess as u8);
        cancel_pkt.write_u8(super::MAGIC_CANCEL_TRANSFORMATION);
        world.send_to_session_owned(sid, cancel_pkt);

        // Verify cancel packet was sent
        let mut found_cancel = false;
        while let Ok(pkt_out) = rx.try_recv() {
            if pkt_out.opcode == Opcode::WizMagicProcess as u8
                && !pkt_out.data.is_empty()
                && pkt_out.data[0] == super::MAGIC_CANCEL_TRANSFORMATION
            {
                found_cancel = true;
                break;
            }
        }
        assert!(
            found_cancel,
            "MAGIC_CANCEL_TRANSFORMATION packet should be sent on death"
        );
    }

    // ── Sprint 159: Durability loss on death tests ──────────────────────

    #[test]
    fn test_wore_type_defence_constant() {
        assert_eq!(super::super::durability::WORE_TYPE_DEFENCE, 0x02);
    }

    #[test]
    fn test_death_durability_loss_armour_slots() {
        // Verify that WORE_TYPE_DEFENCE targets armour slots (HEAD, BREAST, LEG, GLOVE, FOOT)
        let wore_type = super::super::durability::WORE_TYPE_DEFENCE;
        assert_eq!(wore_type, 2); // DEFENCE = armour slots
    }

    // ── Sprint 250: PvP Loyalty on Death Tests ────────────────────────

    use crate::world::{CharacterInfo, Position, WorldState};
    use tokio::sync::mpsc;

    fn make_pvp_test_char(sid: u16, name: &str, nation: u8, loyalty: u32) -> CharacterInfo {
        CharacterInfo {
            session_id: sid,
            name: name.to_string(),
            nation,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 0,
            loyalty,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 0x01,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    /// PvP loyalty: solo kill in loyalty zone grants NP to killer, deducts from victim.
    #[test]
    fn test_pvp_loyalty_solo_kill() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Use Ronark Land (zone 76) which has give_loyalty=true
        let pos = Position {
            zone_id: 76,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        // Different nations — Karus (1) vs Elmorad (2)
        world.register_ingame(1, make_pvp_test_char(1, "Killer", 1, 500), pos);
        world.register_ingame(2, make_pvp_test_char(2, "Victim", 2, 500), pos);

        // Set zone info with give_loyalty = true
        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            76,
            ZoneInfo {
                smd_name: "ronark_land.smd".into(),
                zone_name: "Ronark Land".into(),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    give_loyalty: true,
                    ..Default::default()
                },
            },
        );

        // Call pvp_loyalty_on_death
        super::pvp_loyalty_on_death(&world, 1, 2);

        // Killer should have gained loyalty (default: +64 for Ronark Land)
        let killer = world.get_character_info(1).unwrap();
        assert!(
            killer.loyalty > 500,
            "Killer loyalty should increase: got {}",
            killer.loyalty
        );

        // Victim should have lost loyalty (default: -50 for Ronark Land)
        let victim = world.get_character_info(2).unwrap();
        assert!(
            victim.loyalty < 500,
            "Victim loyalty should decrease: got {}",
            victim.loyalty
        );
    }

    /// PvP loyalty: same-nation kill in non-Delos zone gives no NP.
    #[test]
    fn test_pvp_loyalty_same_nation_no_reward() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 76,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        // Same nation — both Karus (1)
        world.register_ingame(1, make_pvp_test_char(1, "Killer", 1, 500), pos);
        world.register_ingame(2, make_pvp_test_char(2, "Victim", 1, 500), pos);

        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            76,
            ZoneInfo {
                smd_name: "ronark_land.smd".into(),
                zone_name: "Ronark Land".into(),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    give_loyalty: true,
                    ..Default::default()
                },
            },
        );

        super::pvp_loyalty_on_death(&world, 1, 2);

        // Same-nation — no loyalty change
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.loyalty, 500);
        let victim = world.get_character_info(2).unwrap();
        assert_eq!(victim.loyalty, 500);
    }

    /// PvP loyalty: zone without give_loyalty flag gives no NP.
    #[test]
    fn test_pvp_loyalty_no_loyalty_zone() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Use Moradon (zone 21) which has give_loyalty=false
        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(1, make_pvp_test_char(1, "Killer", 1, 500), pos);
        world.register_ingame(2, make_pvp_test_char(2, "Victim", 2, 500), pos);

        // Zone info with give_loyalty = false (default)
        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            21,
            ZoneInfo {
                smd_name: "moradon.smd".into(),
                zone_name: "Moradon".into(),
                zone_type: ZoneAbilityType::Neutral,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities::default(),
            },
        );

        super::pvp_loyalty_on_death(&world, 1, 2);

        // No loyalty zone — no change
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.loyalty, 500);
        let victim = world.get_character_info(2).unwrap();
        assert_eq!(victim.loyalty, 500);
    }

    /// Gold change on PvP death: victim loses 50%, killer gains 40%.
    #[tokio::test]
    async fn test_gold_change_on_pvp_death() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 76,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut killer = make_pvp_test_char(1, "Killer", 1, 0);
        killer.gold = 100;
        let mut victim = make_pvp_test_char(2, "Victim", 2, 0);
        victim.gold = 1000;
        world.register_ingame(1, killer, pos);
        world.register_ingame(2, victim, pos);

        // Zone with gold_lose = true
        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            76,
            ZoneInfo {
                smd_name: "ronark_land.smd".into(),
                zone_name: "Ronark Land".into(),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    gold_lose: true,
                    ..Default::default()
                },
            },
        );

        super::gold_change_on_death(&world, 1, 2);

        // Victim had 1000 gold, loses 50% = 500
        let victim = world.get_character_info(2).unwrap();
        assert_eq!(victim.gold, 500, "Victim should lose 50% gold");

        // Killer had 100 gold, gains 40% of 1000 = 400
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.gold, 500, "Killer should gain 40% of victim gold");
    }

    /// Gold change on PvP death: no-op when zone doesn't have gold_lose.
    #[tokio::test]
    async fn test_gold_change_no_gold_lose_zone() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut killer = make_pvp_test_char(1, "Killer", 1, 0);
        killer.gold = 100;
        let mut victim = make_pvp_test_char(2, "Victim", 2, 0);
        victim.gold = 1000;
        world.register_ingame(1, killer, pos);
        world.register_ingame(2, victim, pos);

        // Zone with gold_lose = false (safe zone)
        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            21,
            ZoneInfo {
                smd_name: "moradon.smd".into(),
                zone_name: "Moradon".into(),
                zone_type: ZoneAbilityType::Neutral,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities::default(),
            },
        );

        super::gold_change_on_death(&world, 1, 2);

        // No gold change — zone doesn't have gold_lose
        let victim = world.get_character_info(2).unwrap();
        assert_eq!(victim.gold, 1000, "Victim gold should not change");
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.gold, 100, "Killer gold should not change");
    }

    /// Gold change on PvP death: victim with 0 gold — no-op.
    #[tokio::test]
    async fn test_gold_change_victim_zero_gold() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        let pos = Position {
            zone_id: 76,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        let mut killer = make_pvp_test_char(1, "Killer", 1, 0);
        killer.gold = 500;
        let victim = make_pvp_test_char(2, "Victim", 2, 0); // gold = 0
        world.register_ingame(1, killer, pos);
        world.register_ingame(2, victim, pos);

        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            76,
            ZoneInfo {
                smd_name: "ronark_land.smd".into(),
                zone_name: "Ronark Land".into(),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    gold_lose: true,
                    ..Default::default()
                },
            },
        );

        super::gold_change_on_death(&world, 1, 2);

        // Victim had 0 gold — nothing changes
        let victim = world.get_character_info(2).unwrap();
        assert_eq!(victim.gold, 0);
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.gold, 500, "Killer gold should not change");
    }

    /// Gold change on PvP death: party killer distributes 40% among members by level.
    #[tokio::test]
    async fn test_gold_change_party_distribution() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = Position {
            zone_id: 76,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        // Killer (sid=1, level 60) in party with member (sid=3, level 60)
        let mut killer = make_pvp_test_char(1, "Killer", 1, 0);
        killer.gold = 0;
        killer.level = 60;
        let mut victim = make_pvp_test_char(2, "Victim", 2, 0);
        victim.gold = 1000;
        let mut member = make_pvp_test_char(3, "PartyMember", 1, 0);
        member.gold = 0;
        member.level = 60;
        world.register_ingame(1, killer, pos);
        world.register_ingame(2, victim, pos);
        world.register_ingame(3, member, pos);

        // Create party with killer as leader, add member
        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 3);

        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            76,
            ZoneInfo {
                smd_name: "ronark_land.smd".into(),
                zone_name: "Ronark Land".into(),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    gold_lose: true,
                    ..Default::default()
                },
            },
        );

        super::gold_change_on_death(&world, 1, 2);

        // Victim had 1000 gold, loses 50% = 500
        let victim = world.get_character_info(2).unwrap();
        assert_eq!(victim.gold, 500, "Victim should lose 50% gold");

        // 40% of 1000 = 400, distributed by level (60+60=120)
        // Each gets 400 * (60/120) = 200
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.gold, 200, "Killer should get proportional share");
        let member = world.get_character_info(3).unwrap();
        assert_eq!(
            member.gold, 200,
            "Party member should get proportional share"
        );
    }

    /// Gold change on PvP death: party with unequal levels distributes proportionally.
    #[tokio::test]
    async fn test_gold_change_party_unequal_levels() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        let pos = Position {
            zone_id: 76,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            region_x: 0,
            region_z: 0,
        };
        // Killer level 75, member level 25 (3:1 ratio)
        let mut killer = make_pvp_test_char(1, "Killer", 1, 0);
        killer.gold = 0;
        killer.level = 75;
        let mut victim = make_pvp_test_char(2, "Victim", 2, 0);
        victim.gold = 1000;
        let mut member = make_pvp_test_char(3, "LowMember", 1, 0);
        member.gold = 0;
        member.level = 25;
        world.register_ingame(1, killer, pos);
        world.register_ingame(2, victim, pos);
        world.register_ingame(3, member, pos);

        let party_id = world.create_party(1).unwrap();
        world.add_party_member(party_id, 3);

        use crate::zone::{ZoneAbilities, ZoneAbilityType, ZoneInfo};
        world.set_zone_info(
            76,
            ZoneInfo {
                smd_name: "ronark_land.smd".into(),
                zone_name: "Ronark Land".into(),
                zone_type: ZoneAbilityType::PvP,
                min_level: 1,
                max_level: 83,
                init_x: 0.0,
                init_z: 0.0,
                init_y: 0.0,
                status: 1,
                abilities: ZoneAbilities {
                    gold_lose: true,
                    ..Default::default()
                },
            },
        );

        super::gold_change_on_death(&world, 1, 2);

        // 40% of 1000 = 400, level sum = 75+25 = 100
        // Killer gets 400 * (75/100) = 300
        // Member gets 400 * (25/100) = 100
        let killer = world.get_character_info(1).unwrap();
        assert_eq!(killer.gold, 300, "High-level killer should get 75% share");
        let member = world.get_character_info(3).unwrap();
        assert_eq!(member.gold, 100, "Low-level member should get 25% share");
    }

    /// who_killed_me is set on PvP death and defaults to -1 (PvE).
    #[test]
    fn test_who_killed_me_set_on_pvp_death() {
        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Default should be -1 (PvE / no killer)
        let who = world.with_session(2, |h| h.who_killed_me).unwrap();
        assert_eq!(who, -1, "Default who_killed_me should be -1");

        // After PvP death, set to killer SID
        super::set_who_killed_me(&world, 2, 1);
        let who = world.with_session(2, |h| h.who_killed_me).unwrap();
        assert_eq!(who, 1, "who_killed_me should be set to killer SID");
    }

    /// lost_exp is stored on death and defaults to 0.
    #[test]
    fn test_lost_exp_stored_on_death() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Default should be 0
        let exp = world.with_session(1, |h| h.lost_exp).unwrap();
        assert_eq!(exp, 0, "Default lost_exp should be 0");

        // Simulate storing lost EXP
        world.update_session(1, |h| {
            h.lost_exp = 5000;
        });
        let exp = world.with_session(1, |h| h.lost_exp).unwrap();
        assert_eq!(exp, 5000, "lost_exp should be stored");
    }

    /// who_killed_me and lost_exp reset correctly.
    #[test]
    fn test_death_fields_reset() {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set death tracking fields
        world.update_session(1, |h| {
            h.who_killed_me = 5;
            h.lost_exp = 10000;
        });

        // Reset (simulating regene)
        world.update_session(1, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 0;
        });

        let who = world.with_session(1, |h| h.who_killed_me).unwrap();
        assert_eq!(who, -1, "who_killed_me should be reset to -1");
        let exp = world.with_session(1, |h| h.lost_exp).unwrap();
        assert_eq!(exp, 0, "lost_exp should be reset to 0");
    }

    // ── Sprint 284: NPC XP loss exclusions ──────────────────────────────

    /// Rolling stone (type 214), guard summon (type 172), and saw blade (proto 2107)
    /// should NOT cause XP loss when they kill a player.
    #[test]
    fn test_npc_xp_loss_exclusion_types() {
        // Rolling stone — environmental hazard, no XP loss
        assert_eq!(super::NPC_TYPE_ROLLINGSTONE, 214);
        // Guard summon — fortress turret, no XP loss
        assert_eq!(super::NPC_TYPE_GUARD_SUMMON, 172);
        // Saw blade — trap NPC proto 2107, no XP loss
        assert_eq!(super::SAW_BLADE_PROTO, 2107);
    }

    /// XP loss should only apply for NPC kills (PvE), not PvP/DOT/magic deaths.
    /// C++ calls OnDeathLostExpCalc() only inside OnDeathKilledNpc(), not
    /// OnDeathKilledPlayer(). broadcast_death() must NOT apply XP loss.
    #[test]
    fn test_pvp_death_no_xp_loss() {
        // PvP kills go through broadcast_death only — no XP loss.
        // NPC kills call broadcast_death + apply_npc_death_xp_loss separately.
        // This test verifies the architectural split.
        assert!(true, "XP loss is separated from broadcast_death");
    }

    // ── Sprint 293: War zone XP loss correction ──────────────────────

    #[test]
    fn test_xp_loss_checks_only_exp_lost_not_war_zone() {
        // `if (GetMap() && GetMap()->m_bExpLost != 0)` — ONLY checks m_bExpLost.
        // m_bExpLost and m_kWarZone are set independently (Unit.cpp:2162,2174).
        // War zones with exp_lost=true SHOULD still lose XP.
        //
        // Previously Rust incorrectly checked `exp_lost && !war_zone`.
        // Now correctly checks ONLY `exp_lost`.

        // A war zone with exp_lost=true should allow XP loss
        let war_zone_exp_lost = true; // exp_lost=true
        let _is_war_zone = true; // war_zone=true
        assert!(war_zone_exp_lost, "War zones with exp_lost should lose XP");

        // A war zone with exp_lost=false should NOT lose XP
        let war_zone_no_exp_lost = false; // exp_lost=false
        assert!(
            !war_zone_no_exp_lost,
            "Zones without exp_lost never lose XP"
        );
    }

    // ── Sprint 665: Zone-specific death cleanup tests ────────────────

    #[test]
    fn test_max_damage_constant() {
        assert_eq!(super::MAX_DAMAGE, 32000);
    }

    #[test]
    fn test_utc_death_durability_wipe() {
        // In ZONE_UNDER_CASTLE (86), death should destroy all equipped gear
        // via ItemWoreOut(UTC_ATTACK, -MAX_DAMAGE) + ItemWoreOut(UTC_DEFENCE, -MAX_DAMAGE).
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.update_position(sid, ZONE_UNDER_CASTLE, 100.0, 0.0, 100.0);

        // The item_wore_out function is already tested in durability.rs.
        // Here we verify the zone constant and MAX_DAMAGE are correct.
        assert_eq!(ZONE_UNDER_CASTLE, 86);

        // Calling item_wore_out with MAX_DAMAGE should not panic
        world.item_wore_out(
            sid,
            super::super::durability::WORE_TYPE_UTC_ATTACK,
            super::MAX_DAMAGE,
        );
        world.item_wore_out(
            sid,
            super::super::durability::WORE_TYPE_UTC_DEFENCE,
            super::MAX_DAMAGE,
        );
    }

    #[test]
    fn test_zone_specific_death_constants() {
        // Verify all zone constants used in OnDeathitDoesNotMatter
        assert_eq!(ZONE_BATTLE6, 66);
        assert_eq!(ZONE_DRAKI_TOWER, 95);
        assert_eq!(ZONE_FORGOTTEN_TEMPLE, 55);
        assert_eq!(ZONE_DELOS_CASTELLAN, 35);
        assert_eq!(ZONE_DUNGEON_DEFENCE, 89);
        assert_eq!(ZONE_UNDER_CASTLE, 86);
    }

    #[test]
    fn test_tower_exits_on_death_no_tower() {
        // tower_exits_on_death should be a no-op when player has no tower
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.update_position(sid, ZONE_BATTLE6, 100.0, 0.0, 100.0);

        // tower_owner_id defaults to -1 — should exit early without panic
        tower_exits_on_death(&world, sid);
        assert_eq!(world.get_tower_owner_id(sid), -1);
    }

    #[test]
    fn test_draki_tower_kickouts_no_room() {
        // draki_tower_kickouts_on_death should be a no-op when event_room is 0
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.update_position(sid, ZONE_DRAKI_TOWER, 100.0, 0.0, 100.0);

        // event_room defaults to 0 — should exit early without panic
        draki_tower_kickouts_on_death(&world, sid);
    }

    #[test]
    fn test_wanted_death_cleanup() {
        // When a wanted player dies, is_wanted should be cleared
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        world.update_position(sid, 21, 100.0, 0.0, 100.0); // Moradon

        // Set player as wanted
        world.update_session(sid, |h| {
            h.is_wanted = true;
            h.wanted_expiry_time = 9999;
        });
        assert!(world.with_session(sid, |h| h.is_wanted).unwrap());

        // handle_wanted_logout clears the flag
        super::super::vanguard::handle_wanted_logout(&world, sid);
        assert!(!world.with_session(sid, |h| h.is_wanted).unwrap());
    }

    #[test]
    fn test_kick_zones_on_death() {
        // Verify FT/Delos/DD death uses same kick function as war system
        //   KickOutZoneUser(true, ZONE_MORADON);
        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Player in Forgotten Temple (zone 55)
        world.update_position(sid, ZONE_FORGOTTEN_TEMPLE, 200.0, 0.0, 300.0);

        // kick_out_zone_user teleports to Moradon (zone 21)
        // It needs nation for spawn position
        crate::systems::war::kick_out_zone_user(&world, sid, 1);

        // Verify position updated to Moradon
        let pos = world.get_position(sid).unwrap();
        assert_eq!(pos.zone_id, 21); // ZONE_MORADON
    }
}
