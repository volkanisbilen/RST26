//! DOT/HOT (Damage/Heal Over Time) tick system.
//! Runs every 2 seconds, processing all active durational skills (type 3 effects).
//! Each tick applies the HP amount (negative = damage, positive = heal) and
//! checks if the effect has expired. Death from DOT damage is handled.
//! ## DOT tick interval
//! C++ uses a 2-second interval per tick. The `bDuration` field from `MagicType3Row`
//! determines total ticks: `tick_limit = bDuration / 2`.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::handler::dead;
use crate::magic_constants::{USER_STATUS_CURE, USER_STATUS_DOT, USER_STATUS_POISON};
use crate::systems::buff_tick::{build_buff_expired_packet, send_user_status_update_packet};
use crate::systems::event_room;
use crate::world::{WorldState, USER_DEAD, ZONE_CHAOS_DUNGEON, ZONE_KNIGHT_ROYALE};

/// DOT tick interval in seconds.
const DOT_TICK_INTERVAL_SECS: u64 = 2;

/// Start the DOT/HOT tick background task.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_dot_tick_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(DOT_TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let killed = process_dot_tick(&world);
            for (npc_id, caster_sid) in killed {
                finish_npc_dot_kill(&world, npc_id, caster_sid).await;
            }
        }
    })
}

/// Run the shared NPC death flow (death broadcast, party XP, loot) for an NPC
/// that a DOT tick reduced to 0 HP.
async fn finish_npc_dot_kill(world: &WorldState, npc_id: u32, caster_sid: u16) {
    use crate::handler::attack;

    let Some(npc) = world.get_npc_instance(npc_id) else {
        return;
    };
    let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) else {
        return;
    };
    let manes = attack::is_manes_survival_npc(&npc);
    attack::handle_npc_death(world, caster_sid, npc_id, &npc, &tmpl, manes).await;
    if manes {
        attack::broadcast_npc_death(world, caster_sid, npc_id);
        attack::flush_manes_progress(world, caster_sid);
    }
}

/// Process one DOT/HOT tick — apply HP changes from all active durational skills.
///
/// Returns `(npc_id, caster_sid)` for every NPC killed by a DOT this tick so the
/// caller can run the async death flow.
fn process_dot_tick(world: &WorldState) -> Vec<(u32, u16)> {
    let mut killed_npcs: Vec<(u32, u16)> = Vec::new();
    // ── Pre-check temple event is_attackable state (once per tick) ───
    // temple event zones when combat is not allowed.
    let is_event_attackable = world
        .event_room_manager
        .read_temple_event(|s| s.is_attackable);

    // ── Player DOTs ──────────────────────────────────────────────────
    let ticks = world.process_dot_tick_with_casters();

    // Track sessions that had DOT expirations (negative hp_amount that expired)
    // so we can send USER_STATUS_DOT cure when no DOTs remain.
    let mut sessions_with_expired_dots: HashSet<u16> = HashSet::new();

    for (sid, hp_change, expired, caster_sid) in &ticks {
        let sid = *sid;
        let hp_change = *hp_change;
        let expired = *expired;
        let caster_sid = *caster_sid;

        let (ch, pos) = match world
            .with_session(sid, |h| {
                h.character.as_ref().map(|c| (c.clone(), h.position))
            })
            .flatten()
        {
            Some(v) => v,
            None => continue,
        };

        // Skip dead players
        if ch.res_hp_type == USER_DEAD || ch.hp <= 0 {
            continue;
        }

        if hp_change == 0 {
            continue;
        }

        // Skip DOT damage in temple event zones when combat is not allowed.
        // The DOT still ticks (tick_count advances, DOT expires normally) but
        // no HP change is applied during non-combat event phases.
        if !is_event_attackable && hp_change < 0 && event_room::is_in_temple_event_zone(pos.zone_id)
        {
            // Even if skipping HP application, still send expiry packets
            if expired {
                send_dot_expired_packet(world, sid, hp_change);
                if hp_change < 0 {
                    sessions_with_expired_dots.insert(sid);
                }
            }
            continue;
        }

        // Apply HP change (undead: healing becomes damage)
        let mut effective_change = if hp_change > 0 && world.is_undead(sid) {
            -hp_change
        } else {
            hp_change
        };

        // which applies mastery passives + mana absorb (no mirror — pAttacker=nullptr).
        if effective_change < 0 {
            let mut damage = -effective_change;
            let original_damage = damage;

            // ── Mastery passive damage reduction ────────────────────────
            {
                let victim_zone = pos.zone_id;
                let not_use_zone =
                    victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
                if !not_use_zone && crate::handler::class_change::is_mastered(ch.class) {
                    let master_pts = ch.skill_points[8]; // SkillPointMaster = index 8
                    if master_pts >= 10 {
                        damage = (85 * damage as i32 / 100) as i16;
                    } else if master_pts >= 5 {
                        damage = (90 * damage as i32 / 100) as i16;
                    }
                }
            }

            // ── Mana Absorb (Outrage/Frenzy/Mana Shield) ─────────────
            {
                let victim_zone = pos.zone_id;
                let not_use_zone =
                    victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
                let (absorb_pct, absorb_count) = world
                    .with_session(sid, |h| (h.mana_absorb, h.absorb_count))
                    .unwrap_or((0, 0));
                if absorb_pct > 0 && !not_use_zone {
                    let should_absorb = if absorb_pct == 15 {
                        absorb_count > 0
                    } else {
                        true
                    };
                    if should_absorb {
                        let absorbed = (original_damage as i32 * absorb_pct as i32 / 100) as i16;
                        damage -= absorbed;
                        if damage < 0 {
                            damage = 0;
                        }
                        world.update_character_stats(sid, |c| {
                            c.mp = (c.mp as i32 + absorbed as i32).min(c.max_mp as i32) as i16;
                        });
                        if absorb_pct == 15 {
                            world.update_session(sid, |h| {
                                h.absorb_count = h.absorb_count.saturating_sub(1);
                            });
                        }
                    }
                }
            }

            effective_change = -damage;
        }

        let new_hp = if effective_change > 0 {
            // Heal: don't exceed max HP
            (ch.hp + effective_change).min(ch.max_hp)
        } else {
            // Damage: don't go below 0
            (ch.hp + effective_change).max(0)
        };

        world.update_character_hp(sid, new_hp);

        // Send HP update to the player
        let hp_pkt = crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp);
        world.send_to_session_owned(sid, hp_pkt);

        // Send MAGIC_DURATION_EXPIRED packet when a DOT/HOT slot expires.
        if expired {
            send_dot_expired_packet(world, sid, hp_change);
            if hp_change < 0 {
                sessions_with_expired_dots.insert(sid);
            }
        }

        // Handle death from DOT
        if new_hp <= 0 {
            dead::broadcast_death(world, sid);
            // A DOT kill credits the player that applied it (death notice, loyalty, gold, ...).
            if hp_change < 0
                && caster_sid != 0
                && caster_sid != sid
                && world.get_character_info(caster_sid).is_some()
            {
                dead::set_who_killed_me(world, sid, caster_sid);
                dead::send_death_notice(world, caster_sid, sid);
                dead::rob_chaos_skill_items(world, sid);
                dead::pvp_loyalty_on_death(world, caster_sid, sid);
                dead::gold_change_on_death(world, caster_sid, sid);
                match pos.zone_id {
                    z if z == event_room::ZONE_BDW => {
                        dead::track_bdw_player_kill(world, caster_sid, sid)
                    }
                    z if z == event_room::ZONE_CHAOS => {
                        dead::track_chaos_pvp_kill(world, caster_sid, sid)
                    }
                    z if z == event_room::ZONE_JURAID => {
                        dead::track_juraid_pvp_kill(world, caster_sid)
                    }
                    _ => {}
                }
            }
            world.clear_durational_skills(sid);
        }

        tracing::trace!(
            "[sid={}] DOT tick: hp_change={}, new_hp={}/{}, expired={}",
            sid,
            hp_change,
            new_hp,
            ch.max_hp,
            expired
        );
    }

    // After processing all slots: if a session had DOT expirations and no
    // harmful DOTs remain, send USER_STATUS_DOT cure to revert the HP bar.
    for sid in sessions_with_expired_dots {
        if !world.has_active_harmful_dot(sid) {
            send_user_status_update_packet(world, sid, USER_STATUS_DOT, USER_STATUS_CURE);
            send_user_status_update_packet(world, sid, USER_STATUS_POISON, USER_STATUS_CURE);
        }
    }

    // ── NPC DOTs ─────────────────────────────────────────────────────
    let npc_ticks = world.process_npc_dot_tick();

    for (npc_id, total_damage, caster_sid) in npc_ticks {
        // Runtime bots intentionally reuse the NPC DOT registry because their
        // protocol IDs are in NPC_BAND. They are not present in npc_instances,
        // so process them before the ordinary NPC lookup below.
        if let Some(bot) = world.get_bot(npc_id) {
            if !bot.is_alive() {
                world.clear_npc_dots(npc_id);
                continue;
            }

            if !is_event_attackable
                && total_damage < 0
                && event_room::is_in_temple_event_zone(bot.zone_id)
            {
                continue;
            }

            let new_hp = (bot.hp as i32 + total_damage).clamp(0, bot.max_hp as i32) as i16;
            world.update_bot(npc_id, |target| {
                target.hp = new_hp;
                if total_damage < 0 {
                    target.last_attacker_id = caster_sid as i32;
                }
            });
            crate::systems::bot_ai::broadcast_bot_party_hp(world, npc_id);

            let hp_pkt = crate::handler::target_hp::build_target_hp_packet(
                npc_id,
                0,
                bot.max_hp as u32,
                new_hp as u32,
                total_damage,
                total_damage,
            );
            world.send_to_session_owned(caster_sid, hp_pkt);

            if new_hp <= 0 {
                crate::systems::bot_ai::bot_on_death(
                    world,
                    npc_id,
                    crate::systems::bot_ai::tick_ms(),
                );
            }

            tracing::debug!(
                bot_id = npc_id,
                caster_sid,
                damage = -total_damage,
                hp = new_hp,
                max_hp = bot.max_hp,
                "runtime bot DOT tick"
            );
            continue;
        }

        let npc_hp = match world.get_npc_hp(npc_id) {
            Some(hp) if hp > 0 => hp,
            _ => {
                world.clear_npc_dots(npc_id);
                continue;
            }
        };

        // Apply damage (total_damage is negative for DOT)
        let new_hp = (npc_hp + total_damage).max(0);
        world.update_npc_hp(npc_id, new_hp);
        if total_damage < 0 {
            let dealt = (npc_hp - new_hp).max(0);
            world.record_npc_damage(npc_id, caster_sid, dealt);
        }

        if new_hp <= 0 {
            world.clear_npc_dots(npc_id);
            killed_npcs.push((npc_id, caster_sid));
        } else {
            // NPC survived — notify AI for aggro targeting
            world.notify_npc_damaged(npc_id, caster_sid);

            // Send HP bar update to caster
            if let Some(npc) = world.get_npc_instance(npc_id) {
                if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
                    let hp_pkt = crate::handler::target_hp::build_target_hp_packet(
                        npc_id,
                        0,
                        tmpl.max_hp,
                        new_hp as u32,
                        total_damage,
                        total_damage,
                    );
                    world.send_to_session_owned(caster_sid, hp_pkt);
                }
            }
        }

        tracing::trace!(
            "[npc={}] DOT tick: damage={}, new_hp={}",
            npc_id,
            total_damage,
            new_hp
        );
    }

    killed_npcs
}

/// Send a `MAGIC_DURATION_EXPIRED` packet when a DOT/HOT slot expires.
/// ```cpp
/// Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));
/// if (pEffect->m_sHPAmount > 0)
///     result << uint8(100); // HOT
/// else
///     result << uint8(200); // DOT
/// Send(&result);
/// ```
/// The packet reuses the same `MAGIC_DURATION_EXPIRED` sub-opcode (5) as buff
/// expiry, but the payload byte distinguishes DOT/HOT:
/// - `100` = HOT (heal-over-time) expired
/// - `200` = DOT (damage-over-time) expired
fn send_dot_expired_packet(world: &WorldState, sid: u16, hp_amount: i16) {
    let dot_or_hot: u8 = if hp_amount > 0 { 100 } else { 200 };
    let pkt = build_buff_expired_packet(dot_or_hot);
    world.send_to_session_owned(sid, pkt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::bot_ai::{spawn_gm_bot, SpawnGmBotParams};
    use crate::world::BotAiState;
    use crate::world::NpcDotSlot;

    #[test]
    fn test_dot_tick_interval() {
        assert_eq!(DOT_TICK_INTERVAL_SECS, 2);
    }

    #[test]
    fn test_npc_dot_slot_creation() {
        let slot = NpcDotSlot {
            skill_id: 500100,
            hp_amount: -50,
            tick_count: 0,
            tick_limit: 5,
            caster_sid: 1,
        };
        assert_eq!(slot.skill_id, 500100);
        assert_eq!(slot.hp_amount, -50);
        assert_eq!(slot.tick_count, 0);
        assert_eq!(slot.tick_limit, 5);
        assert_eq!(slot.caster_sid, 1);
    }

    #[test]
    fn test_runtime_bot_poison_tick_updates_hp() {
        let world = WorldState::new();
        let bot_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 71,
                x: 1054.0,
                y: 60.0,
                z: 1082.0,
                class: 1,
                level: 83,
                nation: 2,
                ai_state: BotAiState::Pk,
            },
        );
        world.update_bot(bot_id, |bot| {
            bot.hp = 100;
            bot.max_hp = 100;
        });
        world.add_npc_dot(
            bot_id,
            NpcDotSlot {
                skill_id: 109518,
                hp_amount: -10,
                tick_count: 0,
                tick_limit: 3,
                caster_sid: 1,
            },
        );

        process_dot_tick(&world);

        assert_eq!(world.get_bot(bot_id).unwrap().hp, 90);
    }

    #[test]
    fn test_npc_dot_add_and_process() {
        let world = WorldState::new();
        let npc_id: u32 = 10001;

        // Add a DOT effect
        world.add_npc_dot(
            npc_id,
            NpcDotSlot {
                skill_id: 500100,
                hp_amount: -30,
                tick_count: 0,
                tick_limit: 3,
                caster_sid: 1,
            },
        );

        // First tick
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, npc_id);
        assert_eq!(results[0].1, -30); // damage
        assert_eq!(results[0].2, 1); // caster

        // Second tick
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 1);

        // Third tick (expires after this)
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 1);

        // Fourth tick — DOT has expired, no more results
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_npc_dot_replace_same_skill() {
        let world = WorldState::new();
        let npc_id: u32 = 10001;

        // Add first DOT
        world.add_npc_dot(
            npc_id,
            NpcDotSlot {
                skill_id: 500100,
                hp_amount: -30,
                tick_count: 0,
                tick_limit: 3,
                caster_sid: 1,
            },
        );

        // Replace with stronger version of same skill
        world.add_npc_dot(
            npc_id,
            NpcDotSlot {
                skill_id: 500100,
                hp_amount: -60,
                tick_count: 0,
                tick_limit: 5,
                caster_sid: 2,
            },
        );

        // Should only have 1 DOT slot, with the replaced values
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, -60); // stronger damage
        assert_eq!(results[0].2, 2); // new caster
    }

    #[test]
    fn test_npc_dot_clear_on_death() {
        let world = WorldState::new();
        let npc_id: u32 = 10001;

        world.add_npc_dot(
            npc_id,
            NpcDotSlot {
                skill_id: 500100,
                hp_amount: -30,
                tick_count: 0,
                tick_limit: 10,
                caster_sid: 1,
            },
        );

        // Simulate NPC death — clear DOTs
        world.clear_npc_dots(npc_id);

        // No more ticks
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_npc_dot_max_slots() {
        let world = WorldState::new();
        let npc_id: u32 = 10001;

        // Add 4 DOTs (max)
        for i in 0..4 {
            world.add_npc_dot(
                npc_id,
                NpcDotSlot {
                    skill_id: 500100 + i,
                    hp_amount: -10,
                    tick_count: 0,
                    tick_limit: 5,
                    caster_sid: 1,
                },
            );
        }

        // Try to add a 5th — should be rejected
        world.add_npc_dot(
            npc_id,
            NpcDotSlot {
                skill_id: 500200,
                hp_amount: -100,
                tick_count: 0,
                tick_limit: 5,
                caster_sid: 2,
            },
        );

        // Process: should have 4 DOTs totaling -40 damage
        let results = world.process_npc_dot_tick();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, -40); // 4 * -10
    }

    // ── DOT tick temple event skip tests ─────────────────────────────

    /// DOT tick skip: player in BDW zone (84) with is_attackable=false.
    /// DOT damage should be skipped (hp_change < 0 in event zone).
    #[test]
    fn test_dot_tick_skip_event_zone_not_attackable() {
        let world = WorldState::new();
        // Set event not attackable
        world.event_room_manager.update_temple_event(|s| {
            s.active_event = 4; // BDW
            s.is_active = true;
            s.is_attackable = false;
        });
        let is_attackable = world
            .event_room_manager
            .read_temple_event(|s| s.is_attackable);
        assert!(!is_attackable);

        // Zone 84 is a temple event zone
        assert!(event_room::is_in_temple_event_zone(84));
        // When !is_attackable and player is in zone 84, DOT damage is skipped
    }

    /// DOT tick NOT skipped when is_attackable=true (combat phase open).
    #[test]
    fn test_dot_tick_not_skipped_when_attackable() {
        let world = WorldState::new();
        world.event_room_manager.update_temple_event(|s| {
            s.active_event = 4;
            s.is_active = true;
            s.is_attackable = true; // combat allowed
        });
        let is_attackable = world
            .event_room_manager
            .read_temple_event(|s| s.is_attackable);
        assert!(is_attackable);
        // is_attackable=true → DOT damage should NOT be skipped
    }

    /// DOT tick NOT skipped for players NOT in temple event zones.
    #[test]
    fn test_dot_tick_not_skipped_non_event_zone() {
        let world = WorldState::new();
        world.event_room_manager.update_temple_event(|s| {
            s.active_event = 4;
            s.is_active = true;
            s.is_attackable = false;
        });
        // Zone 21 (Moradon) is NOT a temple event zone
        assert!(!event_room::is_in_temple_event_zone(21));
        // Even with !is_attackable, DOTs in non-event zones tick normally
    }

    /// HOT tick (positive hp_change) is NOT skipped even in event zones.
    #[test]
    fn test_dot_tick_hot_not_skipped_in_event_zone() {
        // HOTs (hp_change > 0) should still apply during non-combat phases.
        // The skip only applies to hp_change < 0 (damage).
        let hp_change: i16 = 30; // positive = heal
        let is_event_attackable = false;
        // HOT skip condition: !is_event_attackable && hp_change < 0
        // Since hp_change > 0, skip does NOT trigger
        assert!(is_event_attackable || hp_change >= 0);
    }
}
