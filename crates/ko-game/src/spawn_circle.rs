//! Pure helpers for the GM `+ekle` command and the live `npc_spawn` reload:
//! circle placement geometry and spawn-table diffing. No world state here, so it is unit-testable.

use ko_db::models::NpcSpawnRow;

/// Default circle diameter in metres (radius 7.5).
pub const DEFAULT_DIAMETER: f32 = 15.0;
pub const MIN_DIAMETER: f32 = 2.0;
pub const MAX_DIAMETER: f32 = 60.0;

/// `n` points evenly spaced (360/n degrees) on a circle of `radius` around (cx, cz).
/// `start_deg` rotates the whole ring (callers may jitter it per command).
pub fn circle_points(cx: f32, cz: f32, n: usize, radius: f32, start_deg: f32) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    let step = std::f32::consts::TAU / n as f32;
    let start = start_deg.to_radians();
    (0..n)
        .map(|i| {
            let a = start + step * i as f32;
            (cx + radius * a.cos(), cz + radius * a.sin())
        })
        .collect()
}

/// Circle placement where every point must satisfy `ok` (map bounds + walkable). A point that
/// fails is pulled toward the centre in steps (75%, 50%, 25% of the radius); if nothing works it
/// falls back to the centre itself. Result always has `n` entries.
pub fn place_on_circle(
    cx: f32,
    cz: f32,
    n: usize,
    radius: f32,
    start_deg: f32,
    ok: impl Fn(f32, f32) -> bool,
) -> Vec<(f32, f32)> {
    let step = if n == 0 { 0.0 } else { std::f32::consts::TAU / n as f32 };
    let start = start_deg.to_radians();
    (0..n)
        .map(|i| {
            let a = start + step * i as f32;
            for f in [1.0f32, 0.75, 0.5, 0.25] {
                let (x, z) = (cx + radius * f * a.cos(), cz + radius * f * a.sin());
                if ok(x, z) {
                    return (x, z);
                }
            }
            (cx, cz)
        })
        .collect()
}

/// Stable signature of every npc_spawn column that influences the live instances.
pub fn spawn_signature(r: &NpcSpawnRow) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (
        r.zone_id,
        r.npc_id,
        r.is_monster,
        r.act_type,
        r.regen_type,
        r.dungeon_family,
        r.special_type,
    )
        .hash(&mut h);
    (
        r.trap_number,
        r.left_x,
        r.top_z,
        r.num_npc,
        r.spawn_range,
        r.regen_time,
        r.direction,
        r.dot_cnt,
        r.path.as_deref(),
        r.room,
    )
        .hash(&mut h);
    h.finish()
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SpawnDiff {
    /// Row ids not live yet.
    pub added: Vec<i32>,
    /// Live row ids that no longer exist in the table.
    pub removed: Vec<i32>,
    /// Row ids present in both but with a different signature.
    pub changed: Vec<i32>,
}

/// Diff the live registry (`live`: row id -> signature) against the table rows `(id, signature)`.
/// Unchanged rows appear in none of the lists, so their running instances are left alone.
pub fn diff_spawns(live: &std::collections::HashMap<i32, u64>, db: &[(i32, u64)]) -> SpawnDiff {
    let mut d = SpawnDiff::default();
    let mut seen = std::collections::HashSet::new();
    for &(id, sig) in db {
        if !seen.insert(id) {
            continue; // duplicate id in input: never act twice on one row
        }
        match live.get(&id) {
            None => d.added.push(id),
            Some(&s) if s != sig => d.changed.push(id),
            _ => {}
        }
    }
    d.removed = live.keys().copied().filter(|id| !seen.contains(id)).collect();
    d.removed.sort_unstable();
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn circle_even_spacing_and_radius() {
        let pts = circle_points(100.0, 200.0, 6, 7.5, 0.0);
        assert_eq!(pts.len(), 6);
        for (x, z) in &pts {
            let d = ((x - 100.0).powi(2) + (z - 200.0).powi(2)).sqrt();
            assert!((d - 7.5).abs() < 1e-3);
        }
        // opposite points are mirrored through the centre (360/6 spacing)
        assert!((pts[0].0 + pts[3].0 - 200.0).abs() < 1e-3);
        assert!((pts[0].1 + pts[3].1 - 400.0).abs() < 1e-3);
        assert!(circle_points(0.0, 0.0, 0, 5.0, 0.0).is_empty());
    }

    #[test]
    fn placement_clamps_invalid_points() {
        // only x < 103 is valid: the east half of the ring is pulled inward / to the centre
        let pts = place_on_circle(100.0, 100.0, 8, 7.5, 0.0, |x, _| x < 103.0);
        assert_eq!(pts.len(), 8);
        assert!(pts.iter().all(|(x, _)| *x < 103.0));
        // nothing valid -> centre fallback, still n entries
        let pts = place_on_circle(5.0, 5.0, 3, 7.5, 0.0, |_, _| false);
        assert_eq!(pts, vec![(5.0, 5.0); 3]);
        // everything valid -> exact ring
        let pts = place_on_circle(50.0, 50.0, 4, 7.5, 0.0, |_, _| true);
        assert!((pts[0].0 - 57.5).abs() < 1e-3);
    }

    #[test]
    fn diff_adds_removes_changes_and_never_duplicates() {
        let live: HashMap<i32, u64> = [(1, 10), (2, 20), (3, 30)].into_iter().collect();
        let db = [(1, 10), (2, 99), (4, 40), (4, 40)];
        let d = diff_spawns(&live, &db);
        assert_eq!(d.added, vec![4]);
        assert_eq!(d.changed, vec![2]);
        assert_eq!(d.removed, vec![3]);
        // identical table -> empty diff
        let same = [(1, 10), (2, 20), (3, 30)];
        assert_eq!(diff_spawns(&live, &same), SpawnDiff::default());
    }
}
