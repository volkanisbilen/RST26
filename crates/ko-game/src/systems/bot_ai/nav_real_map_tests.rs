//! Runs the real bot AI (`tick_bots`) on the real PK maps and checks every
//! position and every step against an independent model of what the 2625
//! client lets a player do (see the "Client movement rules" notes in
//! `ko_protocol::smd`):
//!
//! * zone 71 Ronark Land      -> `freezone_b.smd`
//! * zone 72 Ardream          -> `freezone_a.smd`
//! * zone 73 Ronark Land Base -> `freezone_c.smd`
//!
//! The model here re-reads the SMD with `SmdFile::load_with_faces` and never
//! looks at `bot_blocked`, so it cannot share a bug with the mask it checks.

use super::*;
use crate::zone::{MapData, ZoneAbilities, ZoneAbilityType, ZoneInfo};
use ko_protocol::smd::{
    client_incline_blocks, client_terrain_height, SmdFile, WallIndex, CLIENT_BODY_RADIUS,
};
use std::path::{Path, PathBuf};

fn map_path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map").join(file)
}

/// The "what may a player do" reference for one map.
struct PlayerModel {
    smd: SmdFile,
    walls: WallIndex,
}

impl PlayerModel {
    fn load(file: &str) -> Option<Self> {
        let path = map_path(file);
        if !path.exists() {
            eprintln!("skipping {file}: map not found");
            return None;
        }
        let (smd, faces) = SmdFile::load_with_faces(&path).expect("load SMD");
        let extent = (smd.map_size - 1) as f32 * smd.unit_dist;
        Some(Self {
            walls: WallIndex::new(&faces, extent),
            smd,
        })
    }

    /// Reason a player could not stand at `(x, z)`, if any.
    fn cannot_stand(&self, x: f32, z: f32) -> Option<String> {
        let (size, unit) = (self.smd.map_size, self.smd.unit_dist);
        let extent = (size - 1) as f32 * unit;
        if !(x.is_finite() && z.is_finite()) || x < 4.0 || z < 4.0 || x > extent - 4.0 || z > extent - 4.0 {
            return Some(format!("({x:.2},{z:.2}) outside the client's walkable map edge"));
        }
        if self.smd.get_event_id((x / unit) as i32, (z / unit) as i32) <= 0 {
            return Some(format!("({x:.2},{z:.2}) on an event-grid blocked tile"));
        }
        let feet = client_terrain_height(&self.smd.heights, size, unit, x, z)?;
        let clearance = self.walls.clearance(x, feet, z);
        if clearance < CLIENT_BODY_RADIUS - 0.02 {
            return Some(format!("({x:.2},{z:.2}) only {clearance:.2} from a wall (body radius {CLIENT_BODY_RADIUS})"));
        }
        None
    }

    /// Reason a player could not walk `from -> to`, if any. Checks every
    /// quarter metre of the segment: the standing rules at each sample plus
    /// the client's uphill-slope refusal in the direction of travel.
    fn illegal_step(&self, from: (f32, f32), to: (f32, f32)) -> Option<String> {
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let len = dx.hypot(dz);
        let samples = ((len / 0.25).ceil() as usize).max(1);
        let (dir_x, dir_z) = if len > 1e-6 { (dx / len, dz / len) } else { (0.0, 0.0) };
        for i in 0..=samples {
            let t = i as f32 / samples as f32;
            let (x, z) = (from.0 + dx * t, from.1 + dz * t);
            if let Some(why) = self.cannot_stand(x, z) {
                return Some(why);
            }
            if client_incline_blocks(&self.smd.heights, self.smd.map_size, self.smd.unit_dist, x, z, dir_x, dir_z) {
                return Some(format!("({x:.2},{z:.2}) terrain too steep to climb heading ({dir_x:.2},{dir_z:.2})"));
            }
        }
        None
    }
}

fn zone_info() -> ZoneInfo {
    ZoneInfo {
        smd_name: String::new(),
        zone_name: "NavSim".into(),
        zone_type: ZoneAbilityType::Neutral,
        min_level: 0,
        max_level: 83,
        init_x: 0.0,
        init_z: 0.0,
        init_y: 0.0,
        abilities: ZoneAbilities::default(),
        status: 1,
    }
}

fn world_with_map(zone_id: u16, file: &str) -> Option<WorldState> {
    world_with_mask(zone_id, file, false)
}

/// `legacy = true` swaps in the pre-fix navigation mask (see
/// `legacy_blocked`) so the harness can prove it would have caught the bug.
fn world_with_mask(zone_id: u16, file: &str, legacy: bool) -> Option<WorldState> {
    let path = map_path(file);
    if !path.exists() {
        return None;
    }
    let world = WorldState::new();
    let (mut smd, faces) = SmdFile::load_with_faces(&path).expect("load SMD");
    if legacy {
        smd.bot_blocked = legacy_blocked(&smd, &faces);
    }
    world.set_zone_with_map(zone_id, zone_info(), MapData::new(smd));
    Some(world)
}

/// The mask the server used before the client-rule rewrite: 45 degree slope
/// on a fixed diagonal, floors skipped below 45 degrees, 0.2 m wall margin,
/// vertical window `low+0.4 .. high+2.0`, no map-edge band.
fn legacy_blocked(smd: &SmdFile, faces: &[ko_protocol::smd::CollisionFace]) -> Vec<bool> {
    let (size, unit, h) = (smd.map_size, smd.unit_dist, &smd.heights);
    let mut blocked = vec![true; h.len()];
    for x in 0..size - 1 {
        for z in 0..size - 1 {
            let i = (x * size + z) as usize;
            let q = [h[i], h[i + 1], h[i + size as usize], h[i + size as usize + 1]];
            let s1 = (q[2] - q[0]).hypot(q[1] - q[0]) / unit;
            let s2 = (q[3] - q[1]).hypot(q[3] - q[2]) / unit;
            blocked[i] = q.iter().any(|v| !v.is_finite()) || s1 > 1.0 || s2 > 1.0;
        }
    }
    for v in faces {
        let a = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
        let b = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
        let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        if n[1].abs() >= n[0].hypot(n[2]) {
            continue;
        }
        let min = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
        let max = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
        for x in ((min(0) / unit).floor() as i32).max(0)..=((max(0) / unit).floor() as i32).min(size - 2) {
            for z in ((min(2) / unit).floor() as i32).max(0)..=((max(2) / unit).floor() as i32).min(size - 2) {
                let center = [(x as f32 + 0.5) * unit, (z as f32 + 0.5) * unit];
                let half = unit * 0.5 + 0.2;
                let separated = (0..3).any(|edge| {
                    let next = (edge + 1) % 3;
                    let axis = [v[next][2] - v[edge][2], v[edge][0] - v[next][0]];
                    let lo = v.iter().map(|p| p[0] * axis[0] + p[2] * axis[1]).fold(f32::INFINITY, f32::min);
                    let hi = v.iter().map(|p| p[0] * axis[0] + p[2] * axis[1]).fold(f32::NEG_INFINITY, f32::max);
                    let c = center[0] * axis[0] + center[1] * axis[1];
                    let r = half * (axis[0].abs() + axis[1].abs());
                    hi < c - r || lo > c + r
                });
                if separated {
                    continue;
                }
                let i = (x * size + z) as usize;
                let q = [h[i], h[i + 1], h[i + size as usize], h[i + size as usize + 1]];
                let low = q.iter().copied().fold(f32::INFINITY, f32::min);
                let high = q.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                if max(1) > low + 0.4 && min(1) < high + 2.0 {
                    blocked[i] = true;
                }
            }
        }
    }
    blocked
}

#[derive(Default)]
struct SimStats {
    bots: usize,
    steps: usize,
    moved_bots: usize,
    max_step: f32,
    total_distance: f32,
    respawns: usize,
}

/// Spawns a PK wave and ticks the AI `ticks` times (bypassing the 250 ms wall
/// clock throttle), asserting every spawn point, every position and every
/// step is legal for a player.
fn simulate(zone_id: u16, file: &str, per_nation: usize, ticks: usize) -> Option<SimStats> {
    simulate_with(zone_id, file, per_nation, ticks, false)
}

fn simulate_with(zone_id: u16, file: &str, per_nation: usize, ticks: usize, legacy: bool) -> Option<SimStats> {
    let model = PlayerModel::load(file)?;
    let world = world_with_mask(zone_id, file, legacy)?;
    spawn_pk_bot_wave(&world, zone_id, Some(per_nation), Some(59)).expect("spawn wave");
    let ids: Vec<BotId> = world.bots.iter().map(|e| *e.key()).collect();
    assert!(!ids.is_empty(), "zone {zone_id}: no bots spawned");

    let mut last: HashMap<BotId, (f32, f32)> = HashMap::new();
    let mut start: HashMap<BotId, (f32, f32)> = HashMap::new();
    for &id in &ids {
        let b = world.get_bot(id).unwrap();
        if let Some(why) = model.cannot_stand(b.x, b.z) {
            panic!("zone {zone_id}: bot {id} spawned where a player cannot stand: {why}");
        }
        last.insert(id, (b.x, b.z));
        start.insert(id, (b.x, b.z));
    }

    let mut stats = SimStats { bots: ids.len(), ..Default::default() };
    // Longest legitimate single-tick step (fastest class, 250 ms tick).
    let step_cap = 90.0 * BOT_AI_TICK_MS as f32 / 10_000.0 + 0.5;
    for tick in 0..ticks {
        for &id in &ids {
            world.update_bot(id, |b| b.last_tick_ms = 0);
        }
        tick_bots(&world);
        for &id in &ids {
            let Some(b) = world.get_bot(id) else { continue };
            let prev = last[&id];
            let cur = (b.x, b.z);
            if cur == prev {
                continue;
            }
            let dist = (cur.0 - prev.0).hypot(cur.1 - prev.1);
            if dist > step_cap {
                // The only legitimate jump is a death respawn (`bot_regene`)
                // at the nation start ring; it must land on legal ground.
                let near_spawn = [1u8, 2u8].iter().any(|&n| {
                    let (sx, sz) = get_bot_respawn_position(zone_id, n);
                    (cur.0 - sx).hypot(cur.1 - sz) < 120.0
                });
                assert!(
                    near_spawn,
                    "zone {zone_id} tick {tick}: bot {id} jumped {dist:.2} units ({prev:?} -> {cur:?}) and not to a nation spawn"
                );
                if let Some(why) = model.cannot_stand(cur.0, cur.1) {
                    panic!("zone {zone_id} tick {tick}: bot {id} respawned at {cur:?}, which a player cannot stand on: {why}");
                }
                stats.respawns += 1;
                last.insert(id, cur);
                continue;
            }
            if let Some(why) = model.illegal_step(prev, cur) {
                panic!("zone {zone_id} tick {tick}: bot {id} {prev:?} -> {cur:?} is not a legal player move: {why}");
            }
            stats.steps += 1;
            stats.max_step = stats.max_step.max(dist);
            stats.total_distance += dist;
            last.insert(id, cur);
        }
    }
    stats.moved_bots = ids
        .iter()
        .filter(|id| {
            let (sx, sz) = start[id];
            let (cx, cz) = last[id];
            (cx - sx).hypot(cz - sz) > 8.0
        })
        .count();
    Some(stats)
}

fn report(zone_id: u16, stats: &SimStats) {
    eprintln!(
        "zone {zone_id}: {} bots, {} legal steps, {:.0} units walked, longest step {:.2}, {} respawns, {} bots ended >8 units from spawn",
        stats.bots, stats.steps, stats.total_distance, stats.max_step, stats.respawns, stats.moved_bots
    );
}

#[test]
fn ronark_land_bots_never_leave_player_walkable_ground() {
    if let Some(stats) = simulate(ZONE_RONARK_LAND, "freezone_b.smd", 30, 1200) {
        report(ZONE_RONARK_LAND, &stats);
        assert!(stats.steps > 1000, "bots barely moved ({} steps)", stats.steps);
        assert!(stats.moved_bots * 2 >= stats.bots, "most bots are stuck at spawn");
    }
}

#[test]
fn ardream_bots_never_leave_player_walkable_ground() {
    if let Some(stats) = simulate(ZONE_ARDREAM, "freezone_a.smd", 30, 1200) {
        report(ZONE_ARDREAM, &stats);
        assert!(stats.steps > 1000, "bots barely moved ({} steps)", stats.steps);
        assert!(stats.moved_bots * 2 >= stats.bots, "most bots are stuck at spawn");
    }
}

#[test]
fn ronark_land_base_bots_never_leave_player_walkable_ground() {
    if let Some(stats) = simulate(ZONE_RONARK_LAND_BASE, "freezone_c.smd", 30, 1200) {
        report(ZONE_RONARK_LAND_BASE, &stats);
        assert!(stats.steps > 1000, "bots barely moved ({} steps)", stats.steps);
        assert!(stats.moved_bots * 2 >= stats.bots, "most bots are stuck at spawn");
    }
}

/// Every real-map patrol destination, walked with the real step function
/// from the nearest legal tile: no step may be illegal for a player.
#[test]
fn patrol_waypoint_walks_stay_player_legal() {
    for (zone_id, file) in [
        (ZONE_RONARK_LAND, "freezone_b.smd"),
        (ZONE_ARDREAM, "freezone_a.smd"),
        (ZONE_RONARK_LAND_BASE, "freezone_c.smd"),
    ] {
        let Some(model) = PlayerModel::load(file) else { continue };
        let Some(world) = world_with_map(zone_id, file) else { continue };
        let mut walked = 0usize;
        for nation in [1u8, 2u8] {
            let (sx, sz) = get_bot_respawn_position(zone_id, nation);
            let zone = world.get_zone(zone_id).unwrap();
            let map = zone.map_data.as_ref().unwrap();
            let Some((mut x, mut z)) = crate::systems::pathfind::nearest_bot_point(map, sx, sz, 64) else {
                panic!("zone {zone_id} nation {nation}: no walkable spawn");
            };
            let mut bot = make_walk_bot(zone_id, nation, x, z);
            for state in 1..=14u8 {
                let wp = match zone_id {
                    ZONE_RONARK_LAND => bot_waypoints::get_bowl_waypoint(1, state, nation),
                    ZONE_RONARK_LAND_BASE => bot_waypoints::get_ronark_base_orbit_waypoint(1, state, nation),
                    _ => bot_waypoints::get_waypoint(zone_id, 1, state, nation),
                };
                let Some((wx, wz)) = wp else { continue };
                let Some((gx, gz)) = crate::systems::pathfind::nearest_bot_point(map, wx, wz, 8) else { continue };
                // Walk toward the goal for a bounded number of ticks.
                for _ in 0..4000 {
                    let Some((nx, _ny, nz)) = bot_path_step_toward(&world, &bot, gx, bot.y, gz) else { break };
                    if let Some(why) = model.illegal_step((x, z), (nx, nz)) {
                        panic!("zone {zone_id}: step ({x:.2},{z:.2}) -> ({nx:.2},{nz:.2}) toward ({gx:.0},{gz:.0}): {why}");
                    }
                    walked += 1;
                    (x, z) = (nx, nz);
                    bot.x = x;
                    bot.z = z;
                    if (x - gx).hypot(z - gz) < 1.0 {
                        break;
                    }
                }
            }
        }
        eprintln!("zone {zone_id}: {walked} patrol steps checked");
        assert!(walked > 100, "zone {zone_id}: patrol walk barely moved ({walked} steps)");
    }
}

fn make_walk_bot(zone_id: u16, nation: u8, x: f32, z: f32) -> BotInstance {
    // A real instance via the GM spawn helper keeps this in sync with the
    // struct; the bot is removed from the world again immediately.
    let world = WorldState::new();
    let id = spawn_gm_bot(
        &world,
        SpawnGmBotParams { zone_id, x, y: 0.0, z, class: 1, level: 59, nation, ai_state: BotAiState::Pk },
    );
    world.get_bot(id).expect("spawned bot")
}

#[test]
fn bot_standing_on_a_wall_is_returned_to_walkable_ground_on_the_next_tick() {
    let Some(model) = PlayerModel::load("freezone_b.smd") else { return };
    let Some(world) = world_with_map(ZONE_RONARK_LAND, "freezone_b.smd") else { return };
    // Find a tile the mask refuses next to ground it accepts (a cliff foot).
    let zone = world.get_zone(ZONE_RONARK_LAND).unwrap();
    let map = zone.map_data.as_ref().unwrap();
    let (cx, cz) = (1050.0f32, 842.0f32);
    let (ox, oz) = crate::systems::pathfind::nearest_bot_point(map, cx, cz, 8).expect("bowl tile");
    let blocked = (1..200)
        .map(|d| (ox + d as f32 * 4.0, oz))
        .find(|&(x, z)| !map.is_bot_movable(x, z) && map.is_valid_position(x, z))
        .expect("a blocked tile east of the bowl");
    let id = spawn_gm_bot(
        &world,
        SpawnGmBotParams {
            zone_id: ZONE_RONARK_LAND,
            x: blocked.0,
            y: 0.0,
            z: blocked.1,
            class: 1,
            level: 59,
            nation: 1,
            ai_state: BotAiState::Pk,
        },
    );
    assert!(model.cannot_stand(blocked.0, blocked.1).is_some() || !map.is_bot_movable(blocked.0, blocked.1));
    world.update_bot(id, |b| b.last_tick_ms = 0);
    tick_bots(&world);
    let b = world.get_bot(id).unwrap();
    assert!(map.is_bot_movable(b.x, b.z), "bot still on blocked ground at ({}, {})", b.x, b.z);
    assert!(model.cannot_stand(b.x, b.z).is_none(), "relocated onto ground a player cannot stand on");
}

// ── synthetic re-route / stuck handling ─────────────────────────────────

fn grid_world(zone_id: u16, size: i32, blocked: &[(i32, i32)]) -> WorldState {
    let mut event_grid = vec![1i16; (size * size) as usize];
    for &(x, z) in blocked {
        event_grid[(x * size + z) as usize] = 0;
    }
    let smd = SmdFile {
        heights: Vec::new(),
        bot_blocked: Vec::new(),
        map_size: size,
        unit_dist: 4.0,
        map_width: (size - 1) as f32 * 4.0,
        map_height: (size - 1) as f32 * 4.0,
        event_grid,
        warps: Vec::new(),
        regene_events: Vec::new(),
    };
    let world = WorldState::new();
    world.set_zone_with_map(zone_id, zone_info(), MapData::new(smd));
    world
}

/// A wall across the whole map but for a gap at the far end: the bot cannot
/// take the straight line to its goal and must re-route through the gap,
/// never touching a blocked cell on the way.
#[test]
fn bot_reroutes_around_a_wall_and_reaches_the_goal() {
    let size = 40;
    let wall: Vec<(i32, i32)> = (0..30).map(|z| (20, z)).collect();
    let world = grid_world(ZONE_RONARK_LAND, size, &wall);
    let mut bot = make_walk_bot(ZONE_RONARK_LAND, 1, 62.0, 42.0);
    let goal = (102.0f32, 42.0f32);
    let mut through_gap = false;
    let mut reached = false;
    for _ in 0..600 {
        let Some((nx, _, nz)) = bot_path_step_toward(&world, &bot, goal.0, bot.y, goal.1) else {
            panic!("bot froze at ({}, {}) with a route available", bot.x, bot.z);
        };
        assert!(
            is_bot_move_valid(&world, ZONE_RONARK_LAND, bot.x, bot.z, nx, nz),
            "illegal step ({}, {}) -> ({nx}, {nz})",
            bot.x,
            bot.z
        );
        through_gap |= bot.x < 80.0 && nx >= 80.0 && nz >= 120.0;
        bot.x = nx;
        bot.z = nz;
        if (bot.x - goal.0).hypot(bot.z - goal.1) < 2.0 {
            reached = true;
            break;
        }
    }
    assert!(reached, "bot never arrived; stuck at ({}, {})", bot.x, bot.z);
    assert!(through_gap, "bot must have gone through the gap at z >= 120");
}

/// A goal sealed inside a box has no route: the bot must report failure
/// instead of stepping through the wall.
#[test]
fn bot_never_steps_through_a_sealed_wall() {
    let size = 30;
    let mut wall = Vec::new();
    for i in 10..=16 {
        wall.extend([(i, 10), (i, 16), (10, i), (16, i)]);
    }
    let world = grid_world(ZONE_RONARK_LAND, size, &wall);
    let bot = make_walk_bot(ZONE_RONARK_LAND, 1, 20.0, 20.0);
    // Goal in the middle of the sealed 5x5 room.
    assert!(bot_path_step_toward(&world, &bot, 54.0, bot.y, 54.0).is_none());
}

#[test]
fn unreachable_target_is_parked_then_retried() {
    let id: BotId = 0x7FFF_0001;
    assert!(!target_is_parked_unreachable(id, 12, 1_000));
    park_unreachable_target(id, 12, 1_000);
    assert!(target_is_parked_unreachable(id, 12, 1_000 + BOT_UNREACHABLE_RETRY_MS - 1));
    assert!(!target_is_parked_unreachable(id, 13, 1_001), "only the parked target is ignored");
    assert!(!target_is_parked_unreachable(id, 12, 1_000 + BOT_UNREACHABLE_RETRY_MS), "retry after the delay");
    forget_bot_navigation_state(id);
    assert!(!target_is_parked_unreachable(id, 12, 1_001));
}

/// Proof the harness has teeth: run the same simulation on the pre-fix mask
/// and require the independent player model to catch bots standing or
/// stepping where a player cannot, on every PK map.
#[test]
fn harness_catches_the_legacy_mask() {
    for (zone_id, file) in [
        (ZONE_RONARK_LAND, "freezone_b.smd"),
        (ZONE_ARDREAM, "freezone_a.smd"),
        (ZONE_RONARK_LAND_BASE, "freezone_c.smd"),
    ] {
        if !map_path(file).exists() {
            continue;
        }
        let outcome = std::panic::catch_unwind(|| simulate_with(zone_id, file, 30, 1200, true));
        match outcome {
            Ok(_) => panic!("zone {zone_id}: legacy mask passed the player-model harness; the harness cannot see the leak"),
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                eprintln!("zone {zone_id}: legacy mask caught -> {msg}");
            }
        }
    }
}
