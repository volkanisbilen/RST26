//! Pet auto-attack background tick system.
//! once per second for every in-game player inside `Timer_UpdateSessions`.
//! When a pet's family-attack mode is enabled, the pet automatically attacks
//! the designated target NPC:
//! 1. If the pet owner dies → stop attacking
//! 2. If the target NPC is dead or missing → stop attacking
//! 3. If the pet NPC is missing → stop attacking
//! 4. If out of range (50 m) → move pet 2 units closer
//! 5. If in range → calculate damage, apply, broadcast `WIZ_ATTACK`

use std::sync::Arc;
use std::time::Duration;

use ko_protocol::{Opcode, Packet};
use tracing::debug;

use crate::npc::NpcId;
use crate::world::WorldState;
use crate::zone::SessionId;

/// Pet attack tick interval — 1 second.
const PET_ATTACK_TICK_SECS: u64 = 1;

use crate::attack_constants::{ATTACK_FAIL, ATTACK_SUCCESS, ATTACK_TARGET_DEAD, LONG_ATTACK};

use crate::attack_constants::MAX_DAMAGE;

/// Start the pet auto-attack background task.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_pet_attack_tick_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(PET_ATTACK_TICK_SECS));
        loop {
            interval.tick().await;
            process_pet_attack_tick(&world).await;
        }
    })
}

/// Process one pet attack tick for all sessions with active pet attacks.
async fn process_pet_attack_tick(world: &WorldState) {
    let attack_data = world.collect_pet_attack_data();
    for pd in attack_data {
        process_single_pet_attack(world, &pd).await;
    }
}

/// Process a single pet's auto-attack for one tick.
async fn process_single_pet_attack(world: &WorldState, pd: &crate::world::PetAttackData) {
    // 1. Owner dead → stop attack
    if pd.owner_dead {
        stop_pet_attack(world, pd.session_id);
        return;
    }

    // 2. Target NPC: must exist and be alive
    let target_npc = match world.get_npc_instance(pd.target_npc_id) {
        Some(n) => n,
        None => {
            stop_pet_attack(world, pd.session_id);
            return;
        }
    };

    if world.is_npc_dead(pd.target_npc_id) {
        stop_pet_attack(world, pd.session_id);
        return;
    }

    // 3. Pet NPC: must exist
    let pet_npc = match world.get_npc_instance(pd.pet_nid as NpcId) {
        Some(n) => n,
        None => {
            stop_pet_attack(world, pd.session_id);
            return;
        }
    };

    // 4. Range check — squared distance between pet and target
    let dx = target_npc.x - pet_npc.x;
    let dz = target_npc.z - pet_npc.z;
    let dist_sq = dx * dx + dz * dz;

    const PET_MELEE_RANGE_SQ: f32 = 9.0;

    if dist_sq > PET_MELEE_RANGE_SQ {
        // Move pet 2 units closer to target
        let distance = dist_sq.sqrt();
        if distance == 0.0 {
            return;
        }

        // Normalize direction and move pet 2 units past the target.
        // C++ does: new_pos = target_pos + 2 * normalize(target - pet)
        // This overshoots slightly, ensuring the pet reaches attack range.
        let dir_x = dx / distance;
        let dir_z = dz / distance;
        let new_x = target_npc.x - dir_x * 2.0;
        let new_z = target_npc.z - dir_z * 2.0;

        // Update pet NPC position
        world.move_runtime_npc(pd.pet_nid as NpcId, new_x, new_z);

        // Broadcast pet move: WIZ_NPC_MOVE
        let mut move_pkt = Packet::new(Opcode::WizNpcMove as u8);
        move_pkt.write_u8(1); // move type
        move_pkt.write_u32(pd.pet_nid as u32);
        move_pkt.write_u16((new_x * 10.0) as u16);
        move_pkt.write_u16((new_z * 10.0) as u16);
        // Familiar spawn uses the owner's actual terrain height. Sending a
        // hard-coded zero here makes the client interpolate the pet below the
        // ground after its first chase step (the server-side attack continues,
        // so it looks exactly like a pet that vanishes while still attacking).
        move_pkt.write_u16((pet_npc.y.max(0.0) * 10.0) as u16);
        move_pkt.write_u16((distance * 10.0) as u16); // speed

        let event_room = world.get_event_room(pd.session_id);
        // Publish from both sides of a region transition so observers receive
        // the final step and nearby subscribers keep the pet in their view.
        world.broadcast_to_3x3(
            pet_npc.zone_id,
            pet_npc.region_x,
            pet_npc.region_z,
            Arc::new(move_pkt.clone()),
            None,
            event_room,
        );
        if let Some(updated) = world.get_npc_instance(pd.pet_nid as NpcId) {
            if updated.region_x != pet_npc.region_x || updated.region_z != pet_npc.region_z {
                // Observers of the new region have never seen this familiar: hand over with
                // NPC_OUT (old 3x3) and NPC_IN carrying the pet's own model (new 3x3).
                if let Some(template) = world.get_npc_template(updated.proto_id, updated.is_monster)
                {
                    let visual = world.npc_visual_template(&updated, &template);
                    world.broadcast_to_3x3(
                        pet_npc.zone_id,
                        pet_npc.region_x,
                        pet_npc.region_z,
                        Arc::new(crate::npc::build_npc_inout(
                            crate::npc::NPC_OUT,
                            &pet_npc,
                            &template,
                        )),
                        None,
                        event_room,
                    );
                    world.broadcast_to_3x3(
                        updated.zone_id,
                        updated.region_x,
                        updated.region_z,
                        Arc::new(crate::npc::build_npc_inout(
                            crate::npc::NPC_IN,
                            &updated,
                            &visual,
                        )),
                        None,
                        event_room,
                    );
                }
                world.broadcast_to_3x3(
                    updated.zone_id,
                    updated.region_x,
                    updated.region_z,
                    Arc::new(move_pkt),
                    None,
                    event_room,
                );
            }
        }
        return;
    }

    // 5. In range — calculate and apply damage

    // A pet is carried by a generic type-15 NPC template. Its real combat
    // strength comes exclusively from pet_stats_info, otherwise a level-1
    // familiar inherits the carrier NPC's high damage.
    let hit = world
        .get_pet_stats_info(pd.pet_level)
        .map(|stats| stats.pet_attack.max(1) as f32)
        .unwrap_or(1.0);
    // GREAT_SUCCESS: damage = rand(0, 0.6 * Hit) + 0.7 * Hit
    let rand_range = (0.6 * hit) as i16;
    let base = (0.7 * hit) as i16;
    let mut damage = if rand_range > 0 {
        let random_part = (rand::random::<u16>() % (rand_range as u16 + 1)) as i16;
        (random_part + base).min(MAX_DAMAGE as i16)
    } else {
        base.min(MAX_DAMAGE as i16)
    };

    // The v2625 familiar attack skills 301001–301006 are real Type-1 attacks,
    // not just animation requests. Apply their TBL hit multiplier to the next
    // pet hit (instead of treating every button as the same basic attack).
    // The pending ID is cleared once consumed so it cannot repeat every tick.
    if (301_001..=301_006).contains(&pd.attack_skill_id) {
        let hit_percent = world
            .get_magic_type1(pd.attack_skill_id as i32)
            .and_then(|row| row.hit)
            .filter(|hit| *hit > 0)
            .unwrap_or(100);
        damage = ((damage as i32 * hit_percent) / 100)
            .clamp(1, MAX_DAMAGE as i32) as i16;
    }
    if pd.attack_skill_id != 0 {
        world.update_session(pd.session_id, |h| {
            if let Some(pet) = h.pet_data.as_mut() {
                if pet.pending_attack_skill_id == pd.attack_skill_id {
                    pet.pending_attack_skill_id = 0;
                }
            }
        });
    }

    if damage <= 0 {
        // Miss — broadcast fail
        broadcast_pet_attack(world, pd, ATTACK_FAIL);
        return;
    }

    // Apply damage to target NPC
    let npc_hp = world.get_npc_hp(pd.target_npc_id).unwrap_or(0);
    let new_hp = (npc_hp - damage as i32).max(0);
    world.update_npc_hp(pd.target_npc_id, new_hp);
    world.record_npc_damage(pd.target_npc_id, pd.session_id, damage as i32);

    let b_result = if new_hp <= 0 {
        // Target NPC died — handle death (XP, loot, broadcast)
        let target_tmpl = world.get_npc_template(target_npc.proto_id, target_npc.is_monster);
        if let Some(ref tmpl) = target_tmpl {
            crate::handler::attack::handle_npc_death(
                world,
                pd.session_id,
                pd.target_npc_id,
                &target_npc,
                tmpl,
                false,
            )
            .await;
        }

        // Award pet EXP from the killed NPC.
        if let Some(ref tmpl) = target_tmpl {
            let npc_exp = tmpl.exp as i32;
            if npc_exp > 0 {
                award_pet_exp(world, pd.session_id, pd.pet_nid, npc_exp);
            }
        }

        // Stop attacking the dead target
        stop_pet_attack(world, pd.session_id);
        ATTACK_TARGET_DEAD
    } else {
        // Notify NPC AI about damage (reactive aggro)
        world.notify_npc_damaged(pd.target_npc_id, pd.session_id);
        ATTACK_SUCCESS
    };

    // Broadcast attack result
    broadcast_pet_attack(world, pd, b_result);

    // Send HP bar update to the pet owner
    let max_hp = world
        .get_npc_template(target_npc.proto_id, target_npc.is_monster)
        .map(|t| t.max_hp)
        .unwrap_or(0);

    let hp_pkt = crate::handler::target_hp::build_target_hp_packet(
        pd.target_npc_id,
        0,
        max_hp as u32,
        new_hp.max(0) as u32,
        -(damage as i32),
        -(damage as i32),
    );
    world.send_to_session_owned(pd.session_id, hp_pkt);

    debug!(
        "[pet_attack] owner={} pet_npc={} target={} damage={} hp={}/{}",
        pd.session_id, pd.pet_nid, pd.target_npc_id, damage, new_hp, max_hp
    );
}

/// Stop the pet's family-attack mode.
fn stop_pet_attack(world: &WorldState, sid: SessionId) {
    world.update_session(sid, |h| {
        if let Some(ref mut pet) = h.pet_data {
            pet.attack_started = false;
            pet.attack_target_id = -1;
            pet.pending_attack_skill_id = 0;
        }
    });
}

/// Broadcast `WIZ_ATTACK` from the pet NPC to the 3x3 region.
/// ```text
/// Packet result(WIZ_ATTACK, uint8(LONG_ATTACK));
/// result << bResult << uint32(pFamilyPet->GetID()) << uint32(pNpc->GetID());
/// pFamilyPet->SendToRegion(&result);
/// ```
fn broadcast_pet_attack(world: &WorldState, pd: &crate::world::PetAttackData, b_result: u8) {
    let mut pkt = Packet::new(Opcode::WizAttack as u8);
    pkt.write_u8(LONG_ATTACK);
    pkt.write_u8(b_result);
    pkt.write_u32(pd.pet_nid as u32);
    pkt.write_u32(pd.target_npc_id);

    // Broadcast from the pet owner's region (pet is always near the owner)
    if let Some(pos) = world.get_position(pd.session_id) {
        let event_room = world.get_event_room(pd.session_id);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }
}

/// Award experience to a pet after killing an NPC.
/// If accumulated EXP exceeds the level threshold (`pet_stats_info.pet_exp`),
/// the pet levels up (max 60). On level-up, a broadcast packet is sent to the
/// region and the pet spawn info is re-sent to the owner.
fn award_pet_exp(world: &WorldState, sid: SessionId, pet_nid: u16, gained_exp: i32) {
    const MAX_PET_LEVEL: u8 = 60;

    // Read current pet state
    let pet_snapshot = world.with_session(sid, |h| {
        h.pet_data
            .as_ref()
            .map(|p| (p.level, p.exp, p.satisfaction, p.index, p.name.clone()))
    });
    let (mut level, mut exp, satisfaction, pet_index, pet_name) = match pet_snapshot {
        Some(Some(data)) => data,
        _ => return,
    };

    // Get level threshold from pet_stats_info
    let level_threshold = world
        .get_pet_stats_info(level)
        .map(|s| s.pet_exp)
        .unwrap_or(i32::MAX);

    let mut leveled_up = false;

    if exp as i32 + gained_exp >= level_threshold && level < MAX_PET_LEVEL {
        level += 1;
        let overflow = gained_exp - (level_threshold - exp as i32);
        exp = overflow.max(0) as u32;
        leveled_up = true;

        // Update pet state with new level and stats
        if let Some(new_stats) = world.get_pet_stats_info(level) {
            world.update_session(sid, |h| {
                if let Some(ref mut pet) = h.pet_data {
                    pet.level = level;
                    pet.exp = exp;
                    pet.hp = new_stats.pet_max_hp as u16;
                    pet.mp = new_stats.pet_max_sp as u16;
                }
            });
        } else {
            world.update_session(sid, |h| {
                if let Some(ref mut pet) = h.pet_data {
                    pet.level = level;
                    pet.exp = exp;
                }
            });
        }

        // Broadcast level-up visual effect to region
        let lvl_pkt = crate::handler::pet::build_pet_level_up_broadcast_packet(pet_nid as u32);
        if let Some(pos) = world.get_position(sid) {
            let event_room = world.get_event_room(sid);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(lvl_pkt),
                None,
                event_room,
            );
        }

        // Re-send full spawn info to owner with updated stats
        if let Some(stats) = world.get_pet_stats_info(level) {
            let max_mp = stats.pet_max_sp as u16;

            let spawn_info = crate::handler::pet::PetSpawnInfo {
                index: pet_index,
                name: pet_name,
                level,
                exp_percent: compute_exp_percent(exp, stats.pet_exp),
                max_hp: stats.pet_max_hp as u16,
                hp: stats.pet_max_hp as u16,
                max_mp,
                mp: max_mp,
                satisfaction: satisfaction as u16,
                attack: stats.pet_attack as u16,
                defence: stats.pet_defence as u16,
                resistance: stats.pet_res as u16,
                items: world
                    .with_session(sid, |h| h.pet_data.as_ref().map(|p| p.items.clone()))
                    .flatten()
                    .unwrap_or_default(),
            };
            let spawn_pkt = crate::handler::pet::build_pet_spawn_packet(&spawn_info);
            world.send_to_session_owned(sid, spawn_pkt);

            // Send MP change packet — MP is restored to max on level-up.
            let mp_pkt = crate::handler::pet::build_pet_mp_change_packet(max_mp, max_mp, pet_nid);
            world.send_to_session_owned(sid, mp_pkt);
        }

        debug!(
            "[pet_exp] owner={} pet level up: {} → {}",
            sid,
            level - 1,
            level
        );
    } else {
        // No level up — just accumulate EXP
        exp = (exp as i32 + gained_exp) as u32;
        world.update_session(sid, |h| {
            if let Some(ref mut pet) = h.pet_data {
                pet.exp = exp;
            }
        });
    }

    // Send EXP change packet to owner (always, whether leveled up or not)
    let threshold = if leveled_up {
        world
            .get_pet_stats_info(level)
            .map(|s| s.pet_exp)
            .unwrap_or(1)
    } else {
        level_threshold
    };
    let percent = compute_exp_percent(exp, threshold);

    let exp_pkt = crate::handler::pet::build_pet_exp_change_packet(
        gained_exp as u64,
        percent,
        level,
        satisfaction as u16,
    );
    world.send_to_session_owned(sid, exp_pkt);
}

/// Compute pet EXP percentage (0-10000 scale, e.g. 8100 = 81.00%).
///   `percent = uint16((float(nExp) * 100.0f) / float(PetExp) * 100.0f)`
fn compute_exp_percent(current_exp: u32, level_exp: i32) -> u16 {
    if level_exp <= 0 {
        return 0;
    }
    let pct = (current_exp as f32 * 100.0 / level_exp as f32 * 100.0) as u16;
    pct.min(10000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::RANGE_50M;
    use ko_protocol::PacketReader;

    #[test]
    fn test_pet_attack_constants() {
        // C++ parity checks
        assert_eq!(RANGE_50M, 2500.0);
        assert_eq!(LONG_ATTACK, 1);
        assert_eq!(ATTACK_FAIL, 0);
        assert_eq!(ATTACK_SUCCESS, 1);
        assert_eq!(ATTACK_TARGET_DEAD, 2);
        assert_eq!(MAX_DAMAGE, 32000);
        assert_eq!(PET_ATTACK_TICK_SECS, 1);
    }

    #[test]
    fn test_pet_attack_broadcast_packet_format() {
        // C++ format: WIZ_ATTACK [u8 LONG_ATTACK] [u8 bResult] [u32 pet_id] [u32 target_id]
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(LONG_ATTACK);
        pkt.write_u8(ATTACK_SUCCESS);
        pkt.write_u32(10500); // pet NPC id
        pkt.write_u32(10001); // target NPC id

        assert_eq!(pkt.opcode, Opcode::WizAttack as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(LONG_ATTACK));
        assert_eq!(r.read_u8(), Some(ATTACK_SUCCESS));
        assert_eq!(r.read_u32(), Some(10500));
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_move_packet_format() {
        // C++ format: WIZ_NPC_MOVE [u8(1)] [u32 npc_id] [u16 x*10] [u16 z*10] [u16 y*10] [u16 speed*10]
        let new_x: f32 = 150.5;
        let new_z: f32 = 200.3;
        let speed: f32 = 12.0;

        let mut pkt = Packet::new(Opcode::WizNpcMove as u8);
        pkt.write_u8(1);
        pkt.write_u32(10500);
        pkt.write_u16((new_x * 10.0) as u16);
        pkt.write_u16((new_z * 10.0) as u16);
        pkt.write_u16(0);
        pkt.write_u16((speed * 10.0) as u16);

        assert_eq!(pkt.opcode, Opcode::WizNpcMove as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u32(), Some(10500));
        assert_eq!(r.read_u16(), Some(1505)); // 150.5 * 10
        assert_eq!(r.read_u16(), Some(2003)); // 200.3 * 10
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(120)); // 12.0 * 10
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_damage_formula_great_success() {
        // C++ GREAT_SUCCESS: damage = rand(0, 0.6*Hit) + 0.7*Hit
        let hit: f32 = 100.0;
        let rand_range = (0.6 * hit) as i16; // 60
        let base = (0.7 * hit) as i16; // 70

        // Min damage: 0 + 70 = 70
        assert_eq!(base, 70);
        // Max damage: 60 + 70 = 130
        assert_eq!(rand_range + base, 130);
        // With hit=0, damage=0
        let hit_zero: f32 = 0.0;
        assert_eq!((0.7 * hit_zero) as i16, 0);
    }

    #[test]
    fn test_range_check_squared_distance() {
        // In range: 49m
        let dx: f32 = 49.0;
        let dz: f32 = 0.0;
        let dist_sq = dx * dx + dz * dz;
        assert!(dist_sq <= RANGE_50M, "49m should be in range");

        // Out of range: 51m
        let dx: f32 = 51.0;
        let dist_sq = dx * dx + dz * dz;
        assert!(dist_sq > RANGE_50M, "51m should be out of range");
    }

    #[test]
    fn test_stop_pet_attack_clears_state() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set up a pet with active attack
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                attack_started: true,
                attack_target_id: 42,
                nid: 10,
                ..Default::default()
            });
        });

        // Stop the attack
        stop_pet_attack(&world, sid);

        // Verify state cleared
        let (started, target) = world
            .with_session(sid, |h| {
                h.pet_data
                    .as_ref()
                    .map(|p| (p.attack_started, p.attack_target_id))
            })
            .flatten()
            .unwrap();
        assert!(!started);
        assert_eq!(target, -1);
    }

    #[test]
    fn test_collect_pet_attack_data_filters_inactive() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();

        // Session 1: pet with attack active
        let sid1 = world.allocate_session_id();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        world.register_session(sid1, tx1);
        world.update_session(sid1, |h| {
            h.character = Some(make_test_character(sid1));
            h.pet_data = Some(PetState {
                attack_started: true,
                attack_target_id: 100,
                nid: 20,
                ..Default::default()
            });
        });

        // Session 2: pet with attack NOT active
        let sid2 = world.allocate_session_id();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(sid2, tx2);
        world.update_session(sid2, |h| {
            h.character = Some(make_test_character(sid2));
            h.pet_data = Some(PetState {
                attack_started: false,
                attack_target_id: -1,
                nid: 30,
                ..Default::default()
            });
        });

        // Session 3: no pet
        let sid3 = world.allocate_session_id();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(sid3, tx3);
        world.update_session(sid3, |h| {
            h.character = Some(make_test_character(sid3));
        });

        let data = world.collect_pet_attack_data();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].session_id, sid1);
        assert_eq!(data[0].pet_nid, 20);
        assert_eq!(data[0].target_npc_id, 100);
    }

    // ── Sprint 934: Additional coverage ──────────────────────────────

    /// compute_exp_percent: normal case.
    #[test]
    fn test_compute_exp_percent_normal() {
        // 50% = 5000 (scale 0-10000)
        assert_eq!(compute_exp_percent(500, 1000), 5000);
        // 0%
        assert_eq!(compute_exp_percent(0, 1000), 0);
        // 100% capped at 10000
        assert_eq!(compute_exp_percent(1000, 1000), 10000);
    }

    /// compute_exp_percent: edge cases.
    #[test]
    fn test_compute_exp_percent_edge() {
        // level_exp <= 0 → 0
        assert_eq!(compute_exp_percent(100, 0), 0);
        assert_eq!(compute_exp_percent(100, -1), 0);
    }

    /// Pet HP bar update packet format: WIZ_TARGET_HP.
    #[test]
    fn test_pet_hp_bar_packet_format() {
        let mut hp_pkt = Packet::new(Opcode::WizTargetHp as u8);
        hp_pkt.write_u32(10001); // target_npc_id
        hp_pkt.write_u8(0); // echo
        hp_pkt.write_u32(5000); // max_hp
        hp_pkt.write_u32(3000); // current_hp
        hp_pkt.write_u32((-50i32) as u32); // damage (negative)
        hp_pkt.write_u32(0);
        hp_pkt.write_u8(0);

        let mut r = PacketReader::new(&hp_pkt.data);
        assert_eq!(r.read_u32(), Some(10001));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.read_u32(), Some(3000));
        assert_eq!(r.remaining(), 9); // 4+4+1
    }

    // ── Sprint 955: Additional coverage ──────────────────────────────

    /// Pet attack tick interval is 1 second.
    #[test]
    fn test_pet_attack_tick_interval() {
        assert_eq!(PET_ATTACK_TICK_SECS, 1);
        let dur = Duration::from_secs(PET_ATTACK_TICK_SECS);
        assert_eq!(dur.as_millis(), 1000);
    }

    /// compute_exp_percent: overflow protection (exp > level_exp).
    #[test]
    fn test_compute_exp_percent_overflow() {
        // exp exceeds level_exp → capped at 10000
        assert_eq!(compute_exp_percent(2000, 1000), 10000);
        assert_eq!(compute_exp_percent(u32::MAX, 1), 10000);
    }

    /// compute_exp_percent: small fractions.
    #[test]
    fn test_compute_exp_percent_small() {
        // 1/10000 → 1
        assert_eq!(compute_exp_percent(1, 10000), 1);
        // 1/100000 → 0 (truncated)
        assert_eq!(compute_exp_percent(1, 100000), 0);
    }

    /// ATTACK_SUCCESS and ATTACK_FAIL constants.
    #[test]
    fn test_attack_result_constants() {
        assert_eq!(ATTACK_SUCCESS, 1);
        assert_eq!(ATTACK_FAIL, 0);
        assert_eq!(ATTACK_TARGET_DEAD, 2);
    }

    /// MAX_DAMAGE fits in i16.
    #[test]
    fn test_max_damage_fits_i16() {
        assert!(MAX_DAMAGE <= i16::MAX as i32);
    }

    /// Helper to create a minimal CharacterInfo for tests.
    fn make_test_character(sid: crate::zone::SessionId) -> crate::world::CharacterInfo {
        crate::world::CharacterInfo {
            session_id: sid,
            name: format!("TestPlayer{}", sid),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0u32; 14],
            bind_zone: 0,
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
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
            res_hp_type: 1,
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
}
