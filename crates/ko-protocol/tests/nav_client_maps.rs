//! Verifies the client-derived navigation masks (`Map/nav/<zone>.nav`) of
//! the PK maps against an independent model of what the 2625 client lets a
//! player do, built from the client's own `.gtd`/`.opd` files (see
//! `ko_protocol::nav` for the formats and `smd.rs` "Client movement rules").
//!
//! Needs the client under `KO_CLIENT_DIR` (default `D:/ai/JstKO Client`);
//! the real-file tests skip when it is missing.

use ko_protocol::nav::{
    client_map_for_zone, find_client_file, ClientCollision, ClientTerrain, NavMask,
};
use ko_protocol::smd::{
    client_terrain_height, client_terrain_slope_sin, WallIndex, CLIENT_BODY_RADIUS, CLIENT_SLOPE_SIN_LIMIT,
};
use std::path::{Path, PathBuf};

const ZONES: [u16; 3] = [71, 72, 73];

fn client_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var("KO_CLIENT_DIR").unwrap_or_else(|_| "D:/ai/JstKO Client".into()));
    if dir.join("Zones").is_dir() {
        Some(dir)
    } else {
        eprintln!("client dir {} not found, skipping", dir.display());
        None
    }
}

fn nav_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map/nav")
}

fn load_client(zone: u16) -> Option<(ClientTerrain, ClientCollision)> {
    let dir = client_dir()?.join("Zones");
    let (name, _) = client_map_for_zone(zone)?;
    let gtd = find_client_file(&dir, name, "gtd")?;
    let opd = find_client_file(&dir, name, "opd")?;
    Some((ClientTerrain::load(&gtd).expect("gtd"), ClientCollision::load(&opd).expect("opd")))
}

#[test]
fn client_files_parse_and_match_each_other() {
    for zone in ZONES {
        let Some((terrain, collision)) = load_client(zone) else { return };
        assert!(terrain.size == 257 || terrain.size == 513, "zone {zone}: odd terrain size {}", terrain.size);
        assert_eq!(collision.map_width, (terrain.size - 1) as f32 * terrain.unit, "zone {zone}: opd/gtd extent");
        assert!(collision.faces.len() > 5000, "zone {zone}: only {} collision faces", collision.faces.len());
        let extent = collision.map_width;
        for f in &collision.faces {
            for p in f {
                assert!(p[0] >= -64.0 && p[0] <= extent + 64.0 && p[2] >= -64.0 && p[2] <= extent + 64.0, "zone {zone}: face outside map {f:?}");
            }
        }
        // Terrain heights are plausible metres, not garbage.
        let (lo, hi) = terrain.heights.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &h| (a.min(h), b.max(h)));
        assert!(lo > -500.0 && hi < 1000.0, "zone {zone}: heights {lo}..{hi}");
    }
}

#[test]
fn committed_nav_files_match_a_fresh_build() {
    for zone in ZONES {
        let Some((terrain, collision)) = load_client(zone) else { return };
        let fresh = NavMask::build(&terrain, &collision.faces);
        let path = nav_dir().join(format!("{zone}.nav"));
        let committed = NavMask::load(&path).unwrap_or_else(|e| panic!("{}: {e} (run navgen)", path.display()));
        assert_eq!(committed.size, fresh.size);
        assert_eq!(committed.heights, fresh.heights, "zone {zone}: heights changed, regenerate Map/nav");
        for x in 0..fresh.size {
            for z in 0..fresh.size {
                assert_eq!(committed.is_blocked(x, z), fresh.is_blocked(x, z), "zone {zone}: cell {x},{z} differs, regenerate Map/nav");
            }
        }
        // Round trip through bytes is exact.
        let back = NavMask::from_bytes(&fresh.to_bytes()).unwrap();
        assert_eq!(back.walkable_count(), fresh.walkable_count());
    }
}

fn cell_samples(unit: f32, x: i32, z: i32) -> impl Iterator<Item = (f32, f32)> {
    const OFFS: [f32; 3] = [0.03, 0.5, 0.97];
    OFFS.iter().flat_map(move |ox| OFFS.iter().map(move |oz| ((x as f32 + ox) * unit, (z as f32 + oz) * unit)))
}

/// Every cell the mask calls walkable must be one a player can stand in at
/// every sample point: inside the map-edge band, not steeper than 40
/// degrees, and at least the body radius away from every wall face.
#[test]
fn every_nav_walkable_cell_is_player_walkable() {
    for zone in ZONES {
        let Some((terrain, collision)) = load_client(zone) else { return };
        let mask = NavMask::build(&terrain, &collision.faces);
        let (size, unit) = (mask.size, mask.unit);
        let extent = (size - 1) as f32 * unit;
        let walls = WallIndex::new(&collision.faces, extent);
        let mut bad = Vec::new();
        let mut cells = 0;
        for x in 0..size {
            for z in 0..size {
                if mask.is_blocked(x, z) {
                    continue;
                }
                cells += 1;
                for (wx, wz) in cell_samples(unit, x, z) {
                    if wx < 4.0 || wz < 4.0 || wx > extent - 4.0 || wz > extent - 4.0 {
                        bad.push((x, z, "edge"));
                        break;
                    }
                    let slope = client_terrain_slope_sin(&terrain.heights, size, unit, wx, wz);
                    if slope.is_none_or(|s| s > CLIENT_SLOPE_SIN_LIMIT) {
                        bad.push((x, z, "steep"));
                        break;
                    }
                    let feet = client_terrain_height(&terrain.heights, size, unit, wx, wz).unwrap();
                    if walls.clearance(wx, feet, wz) < CLIENT_BODY_RADIUS - 0.02 {
                        bad.push((x, z, "wall"));
                        break;
                    }
                }
            }
        }
        eprintln!("zone {zone}: {cells} walkable cells, {} violate the client model", bad.len());
        assert!(bad.is_empty(), "zone {zone}: {} bad cells, e.g. {:?}", bad.len(), &bad[..bad.len().min(5)]);
        assert!(cells > 20_000, "zone {zone}: suspiciously few walkable cells ({cells})");
    }
}

/// Owner-reported trouble spots: the ring of mountains around the zone 73
/// base must not be reachable from the bot spawns, and the two Ardream /
/// Ronark Land bases must be connected to the bowl.
#[test]
fn known_places_behave() {
    let Some(_) = client_dir() else { return };
    let nav73 = NavMask::load(&nav_dir().join("73.nav")).expect("73.nav");
    let reach = flood(&nav73, &[(515.0, 104.0), (513.0, 916.0)]);
    let cell = |x: f32, z: f32| ((x / 4.0) as i32, (z / 4.0) as i32);
    // SE outer mountain area of the 1024 m map.
    let mut outer = 0;
    let mut outer_reached = 0;
    for x in 880..1000 {
        for z in 30..150 {
            let (cx, cz) = cell(x as f32, z as f32);
            outer += 1;
            outer_reached += reach[(cx * nav73.size + cz) as usize] as usize;
        }
    }
    assert_eq!(outer_reached, 0, "zone 73: {outer_reached}/{outer} SE outer-mountain points reachable from the bases");
    let r73 = reach.iter().filter(|&&r| r).count();
    assert!(r73 > 15_000 && r73 < 30_000, "zone 73: reachable area {r73} cells looks wrong");
    // Both zone 73 bases reach each other.
    let (a, b) = (cell(515.0, 104.0), cell(513.0, 916.0));
    let from_a = flood(&nav73, &[(515.0, 104.0)]);
    assert!(from_a[(b.0 * nav73.size + b.1) as usize] || nearby(&from_a, &nav73, b), "zone 73: bases not connected");
    let _ = a;
    for (zone, k, e) in [(71u16, (1375.0f32, 1098.0f32), (622.0f32, 898.0f32)), (72, (851.0, 136.0), (190.0, 897.0))] {
        let nav = NavMask::load(&nav_dir().join(format!("{zone}.nav"))).expect("nav");
        let from_k = flood(&nav, &[k]);
        let ec = cell(e.0, e.1);
        assert!(nearby(&from_k, &nav, ec), "zone {zone}: Karus base cannot reach the Elmorad base");
    }
}

fn nearby(reach: &[bool], nav: &NavMask, c: (i32, i32)) -> bool {
    (-8..=8).any(|dx| (-8..=8).any(|dz| nav.in_grid(c.0 + dx, c.1 + dz) && reach[((c.0 + dx) * nav.size + c.1 + dz) as usize]))
}

fn flood(nav: &NavMask, starts: &[(f32, f32)]) -> Vec<bool> {
    let size = nav.size;
    let mut seen = vec![false; (size * size) as usize];
    let mut q = std::collections::VecDeque::new();
    for &(wx, wz) in starts {
        let (cx, cz) = ((wx / nav.unit) as i32, (wz / nav.unit) as i32);
        'find: for r in 0..=16 {
            for dx in -r..=r {
                for dz in -r..=r {
                    let (x, z) = (cx + dx, cz + dz);
                    if nav.is_walkable(x, z) {
                        seen[(x * size + z) as usize] = true;
                        q.push_back((x, z));
                        break 'find;
                    }
                }
            }
        }
    }
    while let Some((x, z)) = q.pop_front() {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let (nx, nz) = (x + dx, z + dz);
                if (dx == 0 && dz == 0) || !nav.is_walkable(nx, nz) || seen[(nx * size + nz) as usize] {
                    continue;
                }
                if dx != 0 && dz != 0 && !(nav.is_walkable(x + dx, z) && nav.is_walkable(x, z + dz)) {
                    continue;
                }
                seen[(nx * size + nz) as usize] = true;
                q.push_back((nx, nz));
            }
        }
    }
    seen
}
