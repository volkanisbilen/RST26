//! A* Pathfinding for NPC movement.
//! -  — A* algorithm
//! -  — node structure
//! -  — PathFind(), StepMove(), IsNoPathFind()
//! -  — MAX_PATH_LINE, TILE_SIZE
//! The pathfinding operates on the SMD event grid. Each cell in the grid represents
//! one tile (TILE_SIZE = 4 world units = unit_dist). World coordinates are converted
//! to grid coordinates by dividing by `unit_dist`.
//! ## Algorithm
//! - 8-directional A* with orthogonal cost 10, diagonal cost 11
//! - Heuristic: Chebyshev distance (max of dx, dy) matching C++ child node heuristic
//! - Open set: BinaryHeap (min-heap by f score)
//! - Closed set: HashSet of visited (x, z) cells
//! - Max waypoints: 100 (MAX_PATH_LINE)

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::zone::MapData;

/// Maximum number of waypoints in a path.
pub const MAX_PATH_LINE: usize = 100;

/// Cost for orthogonal movement (N, E, S, W).
/// (C++ naming is confusing: "DIAGONAL" constant is used for orthogonal moves)
const COST_ORTHOGONAL: i32 = 10;

/// Cost for diagonal movement (NE, SE, SW, NW).
/// (C++ naming is confusing: "CROSS" constant is used for diagonal moves)
const COST_DIAGONAL: i32 = 11;

/// Absolute ceiling for A* iterations — prevents runaway searches on huge maps.
/// The actual per-search limit is computed dynamically via `calc_max_iterations()`.
const MAX_ITERATIONS_CAP: u32 = 250_000;

/// 8-directional neighbor offsets: (dx, dz, cost).
/// Matches C++ FindChildPath ordering: UL, U, UR, R, LR, L(ower), LL, L(eft).
const NEIGHBORS: [(i32, i32, i32); 8] = [
    (-1, -1, COST_DIAGONAL),  // UpperLeft
    (0, -1, COST_ORTHOGONAL), // Upper
    (1, -1, COST_DIAGONAL),   // UpperRight
    (1, 0, COST_ORTHOGONAL),  // Right
    (1, 1, COST_DIAGONAL),    // LowerRight
    (0, 1, COST_ORTHOGONAL),  // Lower
    (-1, 1, COST_DIAGONAL),   // LowerLeft
    (-1, 0, COST_ORTHOGONAL), // Left
];

/// Compute the dynamic iteration limit matching `PathFind.cpp:84-89`.
/// C++ formula: `maxtry = abs(start_x - dest_x) * mapW + abs(start_y - dest_y) * mapH + 1`
/// then hard limit = `maxtry * 2`.
/// This scales with distance: short paths get a small budget (fast bail-out),
/// long paths get a larger budget.  Clamped to `[500, MAX_ITERATIONS_CAP]`.
fn calc_max_iterations(sx: i32, sz: i32, gx: i32, gz: i32, grid_size: i32) -> u32 {
    let dx = (sx - gx).unsigned_abs();
    let dz = (sz - gz).unsigned_abs();
    let maxtry = dx as u64 * grid_size as u64 + dz as u64 * grid_size as u64 + 1;
    let limit = maxtry.saturating_mul(2);
    // Floor of 500 so trivially close paths don't abort too early.
    (limit.min(MAX_ITERATIONS_CAP as u64).max(500)) as u32
}

/// Result of a pathfinding query.
#[derive(Debug, Clone)]
pub struct PathResult {
    /// Waypoints in world coordinates (x, z). Empty if no path found.
    pub waypoints: Vec<(f32, f32)>,
    /// Whether a complete path to the goal was found.
    pub found: bool,
}

/// A node in the A* open set.
#[derive(Debug, Clone, Eq, PartialEq)]
struct AStarNode {
    /// Total estimated cost (f = g + h).
    f: i32,
    /// Cost from start to this node.
    g: i32,
    /// Grid x coordinate.
    x: i32,
    /// Grid z coordinate.
    z: i32,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap, so we reverse for min-heap behavior
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.z.cmp(&other.z))
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Chebyshev distance heuristic scaled by orthogonal cost.
/// Note: C++ uses signed max which can be negative; we use abs for correctness.
/// The C++ initial node uses Euclidean, but child nodes use Chebyshev — we use
/// Chebyshev consistently for admissibility.
fn heuristic(x: i32, z: i32, goal_x: i32, goal_z: i32) -> i32 {
    let dx = (x - goal_x).abs();
    let dz = (z - goal_z).abs();
    // Chebyshev scaled: orthogonal steps cost 10, diagonals cost 11
    let diag = dx.min(dz);
    let straight = dx.max(dz) - diag;
    diag * COST_DIAGONAL + straight * COST_ORTHOGONAL
}

/// Find a path from start to goal on the map's collision grid.
/// Coordinates are in world space. Internally converts to grid coordinates using
/// the SMD's `unit_dist` (typically 4.0, matching C++ TILE_SIZE).
/// The path is returned as world-coordinate waypoints. The start position is NOT
/// included in the waypoints; the goal position IS included as the last waypoint.
pub fn find_path(
    map: &MapData,
    start_x: f32,
    start_z: f32,
    goal_x: f32,
    goal_z: f32,
) -> PathResult {
    find_path_impl(map, start_x, start_z, goal_x, goal_z, false)
}

pub fn find_bot_path(map: &MapData, start_x: f32, start_z: f32, goal_x: f32, goal_z: f32) -> PathResult {
    find_path_impl(map, start_x, start_z, goal_x, goal_z, true)
}

fn find_path_impl(map: &MapData, start_x: f32, start_z: f32, goal_x: f32, goal_z: f32, bot: bool) -> PathResult {
    let is_walkable = |map: &MapData, x, z, size| {
        if bot { x >= 0 && z >= 0 && x < size && z < size && map.is_bot_movable_grid(x, z) }
        else { is_walkable(map, x, z, size) }
    };
    let unit_dist = map.unit_dist();
    if unit_dist <= 0.0 {
        return PathResult {
            waypoints: Vec::new(),
            found: false,
        };
    }

    // Convert world coords to grid coords
    let sx = (start_x / unit_dist) as i32;
    let sz = (start_z / unit_dist) as i32;
    let gx = (goal_x / unit_dist) as i32;
    let gz = (goal_z / unit_dist) as i32;

    let map_grid_size = map.grid_size();

    // Validate start and goal are in-bounds and walkable
    if !is_walkable(map, sx, sz, map_grid_size) || !is_walkable(map, gx, gz, map_grid_size) {
        return PathResult {
            waypoints: Vec::new(),
            found: false,
        };
    }

    // Trivial case: already at goal
    if sx == gx && sz == gz {
        return PathResult {
            waypoints: vec![(goal_x, goal_z)],
            found: true,
        };
    }

    // A* search
    let mut open = BinaryHeap::new();
    let mut g_scores: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h = heuristic(sx, sz, gx, gz);
    open.push(AStarNode {
        f: h,
        g: 0,
        x: sx,
        z: sz,
    });
    g_scores.insert((sx, sz), 0);

    let max_iterations = calc_max_iterations(sx, sz, gx, gz, map_grid_size);
    let mut iterations: u32 = 0;

    while let Some(current) = open.pop() {
        iterations += 1;
        if iterations > max_iterations {
            break;
        }

        let (cx, cz) = (current.x, current.z);

        // Goal reached
        if cx == gx && cz == gz {
            let waypoints =
                reconstruct_path(&came_from, (gx, gz), (sx, sz), goal_x, goal_z, unit_dist);
            return PathResult {
                waypoints,
                found: true,
            };
        }

        // Skip if we already found a better path to this node
        if let Some(&best_g) = g_scores.get(&(cx, cz)) {
            if current.g > best_g {
                continue;
            }
        }

        // Explore neighbors
        for &(dx, dz, cost) in &NEIGHBORS {
            let nx = cx + dx;
            let nz = cz + dz;

            if !is_walkable(map, nx, nz, map_grid_size) {
                continue;
            }

            // A diagonal is legal only when both adjacent orthogonal cells
            // are open. Without this, paths can squeeze through blocked SMD
            // corners even though every emitted waypoint itself is walkable.
            if dx != 0
                && dz != 0
                && (!is_walkable(map, cx + dx, cz, map_grid_size)
                    || !is_walkable(map, cx, cz + dz, map_grid_size))
            {
                continue;
            }

            let new_g = current.g + cost;

            if let Some(&existing_g) = g_scores.get(&(nx, nz)) {
                if new_g >= existing_g {
                    continue;
                }
            }

            g_scores.insert((nx, nz), new_g);
            came_from.insert((nx, nz), (cx, cz));
            let h = heuristic(nx, nz, gx, gz);
            open.push(AStarNode {
                f: new_g + h,
                g: new_g,
                x: nx,
                z: nz,
            });
        }
    }

    PathResult {
        waypoints: Vec::new(),
        found: false,
    }
}

/// Snap a blocked/out-of-grid destination to the closest walkable SMD tile.
/// The returned coordinates are the tile centre so path segments stay inside
/// the nav grid rather than aiming at a wall, cliff, or event trigger.
pub fn nearest_walkable_point(
    map: &MapData,
    world_x: f32,
    world_z: f32,
    max_grid_radius: i32,
) -> Option<(f32, f32)> {
    nearest_point_impl(map, world_x, world_z, max_grid_radius, false)
}

pub fn nearest_bot_point(map: &MapData, world_x: f32, world_z: f32, max_grid_radius: i32) -> Option<(f32, f32)> {
    nearest_point_impl(map, world_x, world_z, max_grid_radius, true)
}

fn nearest_point_impl(map: &MapData, world_x: f32, world_z: f32, max_grid_radius: i32, bot: bool) -> Option<(f32, f32)> {
    let movable = |x, z| if bot { map.is_bot_movable(x, z) } else { map.is_movable(x, z) };
    let unit = map.unit_dist();
    if unit <= 0.0 || max_grid_radius < 0 {
        return None;
    }
    if map.is_valid_position(world_x, world_z) && movable(world_x, world_z) {
        return Some((world_x, world_z));
    }

    let origin_x = (world_x / unit) as i32;
    let origin_z = (world_z / unit) as i32;
    for radius in 1..=max_grid_radius {
        let mut nearest: Option<(f32, f32, f32)> = None;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx.abs().max(dz.abs()) != radius {
                    continue;
                }
                let x = (origin_x + dx) as f32 * unit + unit * 0.5;
                let z = (origin_z + dz) as f32 * unit + unit * 0.5;
                if !map.is_valid_position(x, z) || !movable(x, z) {
                    continue;
                }
                let distance_sq = (x - world_x).powi(2) + (z - world_z).powi(2);
                if nearest.is_none_or(|(_, _, best)| distance_sq < best) {
                    nearest = Some((x, z, distance_sq));
                }
            }
        }
        if let Some((x, z, _)) = nearest {
            return Some((x, z));
        }
    }
    None
}

/// Check if a grid cell is walkable and in-bounds.
fn is_walkable(map: &MapData, gx: i32, gz: i32, grid_size: i32) -> bool {
    gx >= 0 && gz >= 0 && gx < grid_size && gz < grid_size && map.is_movable_grid(gx, gz)
}

/// Reconstruct the path from A* came_from map.
/// Returns waypoints in world coordinates. The start node is excluded; the goal
/// is included as the exact goal coordinates (not grid-snapped).
/// with the last waypoint set to exact endpoint.
fn reconstruct_path(
    came_from: &HashMap<(i32, i32), (i32, i32)>,
    goal: (i32, i32),
    start: (i32, i32),
    goal_world_x: f32,
    goal_world_z: f32,
    unit_dist: f32,
) -> Vec<(f32, f32)> {
    let mut path = Vec::new();
    let mut current = goal;

    while current != start {
        path.push(current);
        match came_from.get(&current) {
            Some(&parent) => current = parent,
            None => break,
        }
    }

    path.reverse();

    // A* found the complete route, but the wire/runtime path list is capped.
    // Keep this as a true path prefix: marking the final prefix node as the
    // distant destination makes movement cut straight across the remaining
    // terrain, where the segment validator then rejects it and leaves bots
    // stationary at the cap boundary.
    let complete_path_fits = path.len() <= MAX_PATH_LINE;
    if !complete_path_fits {
        path.truncate(MAX_PATH_LINE);
    }

    // Convert grid coords to world coords.
    // C++ sets the last waypoint to exact endpoint coordinates.
    let len = path.len();
    path.iter()
        .enumerate()
        .map(|(i, &(gx, gz))| {
            if complete_path_fits && i == len - 1 {
                // Last waypoint = exact goal position
                (goal_world_x, goal_world_z)
            } else {
                // Grid center: tile_coord * unit_dist + unit_dist/2
                let wx = gx as f32 * unit_dist + unit_dist * 0.5;
                let wz = gz as f32 * unit_dist + unit_dist * 0.5;
                (wx, wz)
            }
        })
        .collect()
}

/// Advance one step along a precomputed path.
/// Given the current position, the path waypoints, the current waypoint index,
/// and the NPC's speed (distance per tick), returns the new position and the
/// updated waypoint index.
/// Returns `(new_x, new_z, new_waypoint_index)`.
/// If the path is completed, returns the last waypoint position and the path length.
pub fn step_move(
    current_x: f32,
    current_z: f32,
    path: &[(f32, f32)],
    waypoint_idx: usize,
    speed: f32,
) -> (f32, f32, usize) {
    if path.is_empty() || waypoint_idx >= path.len() {
        return (current_x, current_z, waypoint_idx);
    }

    let (target_x, target_z) = path[waypoint_idx];
    let dx = target_x - current_x;
    let dz = target_z - current_z;
    let dist = (dx * dx + dz * dz).sqrt();

    if dist <= f32::EPSILON {
        // Already at this waypoint, advance to next
        let next_idx = waypoint_idx + 1;
        if next_idx >= path.len() {
            return (target_x, target_z, next_idx);
        }
        return step_move(target_x, target_z, path, next_idx, speed);
    }

    if dist > speed {
        // Move toward the current waypoint by `speed` distance
        let ratio = speed / dist;
        let new_x = current_x + dx * ratio;
        let new_z = current_z + dz * ratio;
        (new_x, new_z, waypoint_idx)
    } else {
        // Reached or passed the waypoint; advance to next
        let remaining_speed = speed - dist;
        let next_idx = waypoint_idx + 1;
        if next_idx >= path.len() || remaining_speed <= f32::EPSILON {
            return (target_x, target_z, next_idx);
        }
        // Use remaining speed to move toward the next waypoint
        step_move(target_x, target_z, path, next_idx, remaining_speed)
    }
}

/// Move directly toward a target without pathfinding.
/// Used for short distances where obstacles are not expected (IsNoPathFind).
/// Moves at most `speed` units toward the target per call.
/// Returns `(new_x, new_z)`.
pub fn step_no_path_move(
    current_x: f32,
    current_z: f32,
    target_x: f32,
    target_z: f32,
    speed: f32,
) -> (f32, f32) {
    let dx = target_x - current_x;
    let dz = target_z - current_z;
    let dist = (dx * dx + dz * dz).sqrt();

    if dist <= f32::EPSILON {
        return (target_x, target_z);
    }

    if dist <= speed {
        (target_x, target_z)
    } else {
        let ratio = speed / dist;
        (current_x + dx * ratio, current_z + dz * ratio)
    }
}

/// Check if there is a clear line of sight between two world positions.
/// Walks the line from start to goal in steps of `step_dist` world units,
/// checking each tile for walkability. Returns true if the entire line is clear.
pub fn line_of_sight(
    map: &MapData,
    start_x: f32,
    start_z: f32,
    goal_x: f32,
    goal_z: f32,
    step_dist: f32,
) -> bool {
    let unit_dist = map.unit_dist();
    if unit_dist <= 0.0 {
        return false;
    }

    let grid_size = map.grid_size();

    // Check start and goal are walkable
    let sx = (start_x / unit_dist) as i32;
    let sz = (start_z / unit_dist) as i32;
    let gx = (goal_x / unit_dist) as i32;
    let gz = (goal_z / unit_dist) as i32;

    if !is_walkable(map, sx, sz, grid_size) || !is_walkable(map, gx, gz, grid_size) {
        return false;
    }

    let dx = goal_x - start_x;
    let dz = goal_z - start_z;
    let total_dist = (dx * dx + dz * dz).sqrt();

    if total_dist <= step_dist {
        return true;
    }

    let step = if step_dist > 0.0 {
        step_dist
    } else {
        unit_dist
    };

    let dir_x = dx / total_dist;
    let dir_z = dz / total_dist;
    let mut walked = 0.0f32;

    while walked < total_dist {
        walked += step;
        let (cx, cz) = if walked > total_dist {
            (goal_x, goal_z)
        } else {
            (start_x + dir_x * walked, start_z + dir_z * walked)
        };

        let tx = (cx / unit_dist) as i32;
        let tz = (cz / unit_dist) as i32;
        if !is_walkable(map, tx, tz, grid_size) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::MapData;
    use ko_protocol::smd::SmdFile;

    /// Create a test map with a given grid. 0 = walkable, nonzero = blocked.
    fn make_test_map(size: i32, blocked: &[(i32, i32)]) -> MapData {
        let grid_len = (size * size) as usize;
        let mut event_grid = vec![0i16; grid_len];
        for &(x, z) in blocked {
            if x >= 0 && x < size && z >= 0 && z < size {
                event_grid[(x as usize) * (size as usize) + (z as usize)] = 1; // blocked
            }
        }
        let unit_dist = 4.0;
        let map_width = (size - 1) as f32 * unit_dist;
        let smd = SmdFile {
            heights: Vec::new(),
            bot_blocked: Vec::new(),
            map_size: size,
            unit_dist,
            map_width,
            map_height: map_width,
            event_grid,
            warps: Vec::new(),
            regene_events: Vec::new(),
        };
        MapData::new(smd)
    }

    #[test]
    fn test_find_path_open_map() {
        // 20x20 open map, no obstacles
        let map = make_test_map(20, &[]);
        // Start at world (8, 8) -> grid (2, 2), Goal at world (40, 40) -> grid (10, 10)
        let result = find_path(&map, 8.0, 8.0, 40.0, 40.0);
        assert!(result.found, "should find path on open map");
        assert!(!result.waypoints.is_empty(), "path should have waypoints");
        // Last waypoint should be the exact goal
        let last = result.waypoints.last().unwrap();
        assert!((last.0 - 40.0).abs() < 0.01);
        assert!((last.1 - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_find_path_blocked() {
        // Create a wall that completely blocks passage
        // 10x10 map with wall at x=5, z=0..9
        let mut blocked = Vec::new();
        for z in 0..10 {
            blocked.push((5, z));
        }
        let map = make_test_map(10, &blocked);
        // Start at grid (2,5), goal at grid (8,5) — wall blocks
        let result = find_path(&map, 8.0, 20.0, 32.0, 20.0);
        assert!(!result.found, "should not find path through wall");
        assert!(result.waypoints.is_empty());
    }

    #[test]
    fn test_find_path_around_obstacle() {
        // 20x20 map with partial wall at x=10, z=5..14 (leaves gaps at z<5 and z>14)
        let mut blocked = Vec::new();
        for z in 5..15 {
            blocked.push((10, z));
        }
        let map = make_test_map(20, &blocked);
        // Start at world (20, 40) -> grid (5, 10), Goal at world (60, 40) -> grid (15, 10)
        let result = find_path(&map, 20.0, 40.0, 60.0, 40.0);
        assert!(result.found, "should find path around obstacle");
        assert!(result.waypoints.len() > 2, "path should go around");
    }

    #[test]
    fn test_find_path_does_not_cut_blocked_diagonal_corner() {
        // Start (0,0) and goal (1,1) are open, but both orthogonal cells
        // touching their diagonal are blocked. A legal route must not squeeze
        // through that corner.
        let map = make_test_map(3, &[(1, 0), (0, 1)]);
        let result = find_path(&map, 1.0, 1.0, 5.0, 5.0);
        assert!(
            !result.found,
            "diagonal corner cutting through blocked SMD cells is not walkable"
        );
    }

    #[test]
    fn test_find_path_same_position() {
        let map = make_test_map(10, &[]);
        let result = find_path(&map, 8.0, 8.0, 8.0, 8.0);
        assert!(result.found);
        assert_eq!(result.waypoints.len(), 1);
        assert!((result.waypoints[0].0 - 8.0).abs() < 0.01);
        assert!((result.waypoints[0].1 - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_find_path_start_blocked() {
        let map = make_test_map(10, &[(2, 2)]);
        let result = find_path(&map, 8.0, 8.0, 20.0, 20.0);
        assert!(!result.found, "should fail when start is blocked");
    }

    #[test]
    fn test_find_path_goal_blocked() {
        let map = make_test_map(10, &[(5, 5)]);
        let result = find_path(&map, 8.0, 8.0, 20.0, 20.0);
        assert!(!result.found, "should fail when goal is blocked");
    }

    #[test]
    fn test_find_path_diagonal_vs_orthogonal_cost() {
        // On a 10x10 open map, a diagonal path should be preferred for diagonal movement
        let map = make_test_map(10, &[]);
        // Diagonal: (0,0) to (16,16) = grid (0,0) to (4,4)
        let result = find_path(&map, 2.0, 2.0, 18.0, 18.0);
        assert!(result.found);
        // Path should be roughly diagonal (few waypoints)
        // Pure diagonal 4 steps costs 4*11=44
        // Pure orthogonal would be 4+4=8 steps at cost 8*10=80
        assert!(result.waypoints.len() <= 6, "diagonal path should be short");
    }

    #[test]
    fn test_step_move_basic() {
        let path = vec![(10.0, 0.0), (20.0, 0.0), (30.0, 0.0)];
        // Start at (0,0), speed 5, should move toward (10,0)
        let (x, z, idx) = step_move(0.0, 0.0, &path, 0, 5.0);
        assert!((x - 5.0).abs() < 0.01);
        assert!((z - 0.0).abs() < 0.01);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_step_move_reaches_waypoint() {
        let path = vec![(10.0, 0.0), (20.0, 0.0)];
        // Start at (5,0), speed 10, should reach (10,0) and advance
        let (x, z, idx) = step_move(5.0, 0.0, &path, 0, 10.0);
        assert!(
            (x - 15.0).abs() < 0.01,
            "should use remaining speed: x={}",
            x
        );
        assert!((z - 0.0).abs() < 0.01);
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_step_move_completes_path() {
        let path = vec![(5.0, 0.0)];
        // Start at (0,0), speed 10, should reach and stop at (5,0)
        let (x, z, idx) = step_move(0.0, 0.0, &path, 0, 10.0);
        assert!((x - 5.0).abs() < 0.01);
        assert!((z - 0.0).abs() < 0.01);
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_step_move_empty_path() {
        let path: Vec<(f32, f32)> = Vec::new();
        let (x, z, idx) = step_move(5.0, 5.0, &path, 0, 10.0);
        assert!((x - 5.0).abs() < 0.01);
        assert!((z - 5.0).abs() < 0.01);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_step_no_path_move_basic() {
        let (x, z) = step_no_path_move(0.0, 0.0, 10.0, 0.0, 5.0);
        assert!((x - 5.0).abs() < 0.01);
        assert!((z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_step_no_path_move_reaches_target() {
        let (x, z) = step_no_path_move(0.0, 0.0, 3.0, 4.0, 10.0);
        assert!((x - 3.0).abs() < 0.01);
        assert!((z - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_step_no_path_move_at_target() {
        let (x, z) = step_no_path_move(5.0, 5.0, 5.0, 5.0, 10.0);
        assert!((x - 5.0).abs() < 0.01);
        assert!((z - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_step_no_path_move_diagonal() {
        // Move diagonally toward (10, 10) from (0, 0) with speed 1
        let (x, z) = step_no_path_move(0.0, 0.0, 10.0, 10.0, 1.0);
        let dist = (x * x + z * z).sqrt();
        assert!((dist - 1.0).abs() < 0.01, "should move exactly 1 unit");
        // Should be at 45 degrees
        assert!((x - z).abs() < 0.01, "should be at 45 degrees");
    }

    #[test]
    fn test_line_of_sight_clear() {
        let map = make_test_map(20, &[]);
        assert!(line_of_sight(&map, 8.0, 8.0, 40.0, 40.0, 4.0));
    }

    #[test]
    fn test_line_of_sight_blocked() {
        // Wall at grid x=5
        let mut blocked = Vec::new();
        for z in 0..20 {
            blocked.push((5, z));
        }
        let map = make_test_map(20, &blocked);
        // Start at grid (2,5) -> world (8,20), goal at grid (8,5) -> world (32,20)
        assert!(!line_of_sight(&map, 8.0, 20.0, 32.0, 20.0, 4.0));
    }

    #[test]
    fn test_line_of_sight_short_distance() {
        let map = make_test_map(10, &[]);
        assert!(line_of_sight(&map, 4.0, 4.0, 5.0, 5.0, 4.0));
    }

    #[test]
    fn test_heuristic_orthogonal() {
        // Pure orthogonal: 5 cells east
        let h = heuristic(0, 0, 5, 0);
        assert_eq!(h, 5 * COST_ORTHOGONAL);
    }

    #[test]
    fn test_heuristic_diagonal() {
        // Pure diagonal: 3 cells NE
        let h = heuristic(0, 0, 3, 3);
        assert_eq!(h, 3 * COST_DIAGONAL);
    }

    #[test]
    fn test_heuristic_mixed() {
        // 5 east, 3 north = 3 diagonal + 2 orthogonal
        let h = heuristic(0, 0, 5, 3);
        assert_eq!(h, 3 * COST_DIAGONAL + 2 * COST_ORTHOGONAL);
    }

    #[test]
    fn test_max_path_line_cap() {
        // Large open map, long path
        let map = make_test_map(500, &[]);
        // Start at one corner, goal far away — path will exceed MAX_PATH_LINE
        let result = find_path(&map, 4.0, 4.0, 1980.0, 4.0);
        assert!(result.found);
        assert!(
            result.waypoints.len() <= MAX_PATH_LINE,
            "path should be capped at MAX_PATH_LINE, got {}",
            result.waypoints.len()
        );
        let (last_x, last_z) = result.waypoints.last().copied().unwrap();
        assert_ne!(
            (last_x, last_z),
            (1980.0, 4.0),
            "truncated path must end at its safe prefix, not jump to the distant goal"
        );
    }

    #[test]
    fn test_find_path_adjacent_cells() {
        let map = make_test_map(10, &[]);
        // Adjacent cells: grid (2,2) to (3,2) -> world (8,8) to (12,8)
        let result = find_path(&map, 8.0, 8.0, 12.0, 8.0);
        assert!(result.found);
        assert_eq!(result.waypoints.len(), 1);
    }

    #[test]
    fn test_find_path_boundary() {
        let map = make_test_map(10, &[]);
        // Goal at grid (9,9) -> world (36,36) — edge of map
        let result = find_path(&map, 4.0, 4.0, 36.0, 36.0);
        assert!(result.found);
    }

    #[test]
    fn test_find_path_out_of_bounds_goal() {
        let map = make_test_map(10, &[]);
        // Goal outside map bounds
        let result = find_path(&map, 4.0, 4.0, 200.0, 200.0);
        assert!(!result.found);
    }
}
