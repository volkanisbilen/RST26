//! Verifies the bot navigation mask (`SmdFile::bot_blocked`) of the real PK
//! maps against an independent model of what the 2625 client lets a player do
//! (see the "Client movement rules" notes in `smd.rs`):
//!
//! * zone 71 Ronark Land      -> `freezone_b.smd`
//! * zone 72 Ardream          -> `freezone_a.smd`
//! * zone 73 Ronark Land Base -> `freezone_c.smd`
//!
//! Set `KO_NAV_DEBUG_DIR` to also write PNG debug maps (walkable / steep /
//! wall-adjacent / legacy leaks) for eyeballing.

use ko_protocol::smd::{
    client_terrain_height, client_terrain_slope_sin, CollisionFace, SmdFile, WallIndex, CLIENT_BODY_RADIUS,
    CLIENT_SLOPE_SIN_LIMIT,
};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

const MAPS: [(&str, u16); 3] = [("freezone_b.smd", 71), ("freezone_a.smd", 72), ("freezone_c.smd", 73)];

fn map_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map").join(name)
}

fn load(name: &str) -> Option<(SmdFile, Vec<CollisionFace>)> {
    let path = map_path(name);
    if !path.exists() {
        eprintln!("skipping {name}: {} not found", path.display());
        return None;
    }
    Some(SmdFile::load_with_faces(&path).expect("load SMD"))
}

fn walkable(smd: &SmdFile, x: i32, z: i32) -> bool {
    smd.get_event_id(x, z) > 0 && !smd.bot_blocked[(x * smd.map_size + z) as usize]
}

/// The mask the server used before the client-rule rewrite (45 degree slope on
/// a fixed diagonal, floors skipped below 45 degrees, 0.2 m wall margin,
/// vertical window `low+0.4 .. high+2.0`), kept here only to quantify leaks.
fn legacy_blocked(smd: &SmdFile, faces: &[CollisionFace]) -> Vec<bool> {
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

/// Sample points inside cell `(x, z)` (never on its boundary).
fn cell_samples(unit: f32, x: i32, z: i32) -> impl Iterator<Item = (f32, f32)> {
    const OFFS: [f32; 3] = [0.03, 0.5, 0.97];
    OFFS.iter()
        .flat_map(move |ox| OFFS.iter().map(move |oz| ((x as f32 + ox) * unit, (z as f32 + oz) * unit)))
}

/// What a real player would be refused at `(x, z)` standing still: terrain
/// steeper than the climb limit (when stepping uphill/along contour there),
/// outside the map edge band, or a wall closer than the body sphere.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Violations {
    steep: u32,
    wall: u32,
    edge: u32,
}

fn violations(smd: &SmdFile, walls: &WallIndex, x: f32, z: f32) -> Violations {
    let mut v = Violations::default();
    let extent = (smd.map_size - 1) as f32 * smd.unit_dist;
    if x < 4.0 || z < 4.0 || x > extent - 4.0 || z > extent - 4.0 {
        v.edge += 1;
    }
    let slope = client_terrain_slope_sin(&smd.heights, smd.map_size, smd.unit_dist, x, z);
    if slope.is_none_or(|s| s > CLIENT_SLOPE_SIN_LIMIT) {
        v.steep += 1;
    }
    if let Some(feet) = client_terrain_height(&smd.heights, smd.map_size, smd.unit_dist, x, z) {
        if walls.clearance(x, feet, z) < CLIENT_BODY_RADIUS - 0.02 {
            v.wall += 1;
        }
    }
    v
}

fn cell_violations(smd: &SmdFile, walls: &WallIndex, x: i32, z: i32) -> Violations {
    let mut total = Violations::default();
    for (wx, wz) in cell_samples(smd.unit_dist, x, z) {
        let v = violations(smd, walls, wx, wz);
        total.steep += v.steep;
        total.wall += v.wall;
        total.edge += v.edge;
    }
    total
}

fn flood(smd: &SmdFile, mask: impl Fn(i32, i32) -> bool, starts: &[(i32, i32)]) -> Vec<bool> {
    let size = smd.map_size;
    let mut seen = vec![false; (size * size) as usize];
    let mut queue = VecDeque::new();
    for &(x, z) in starts {
        if mask(x, z) && !seen[(x * size + z) as usize] {
            seen[(x * size + z) as usize] = true;
            queue.push_back((x, z));
        }
    }
    while let Some((x, z)) = queue.pop_front() {
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let (nx, nz) = (x + dx, z + dz);
                if nx < 0 || nz < 0 || nx >= size || nz >= size || seen[(nx * size + nz) as usize] || !mask(nx, nz) {
                    continue;
                }
                if dx != 0 && dz != 0 && !(mask(x + dx, z) && mask(x, z + dz)) {
                    continue;
                }
                seen[(nx * size + nz) as usize] = true;
                queue.push_back((nx, nz));
            }
        }
    }
    seen
}

fn nearest_walkable(smd: &SmdFile, mask: &impl Fn(i32, i32) -> bool, wx: f32, wz: f32) -> Option<(i32, i32)> {
    let (cx, cz) = ((wx / smd.unit_dist) as i32, (wz / smd.unit_dist) as i32);
    for r in 0..=64i32 {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs().max(dz.abs()) == r {
                    let (x, z) = (cx + dx, cz + dz);
                    if x >= 0 && z >= 0 && x < smd.map_size && z < smd.map_size && mask(x, z) {
                        return Some((x, z));
                    }
                }
            }
        }
    }
    None
}

/// Nation base / bowl anchors the bots actually start from (bot_ai.rs
/// `get_bot_respawn_position` and the Ronark bowl anchors).
fn start_points(zone: u16) -> &'static [(f32, f32)] {
    match zone {
        71 => &[(1375.0, 1098.0), (622.0, 898.0), (1050.0, 842.0), (886.0, 1102.0)],
        72 => &[(851.0, 136.0), (190.0, 897.0)],
        _ => &[(515.0, 104.0), (513.0, 916.0)],
    }
}

#[test]
fn every_bot_walkable_cell_is_player_walkable_on_real_maps() {
    for (name, zone) in MAPS {
        let Some((smd, faces)) = load(name) else { continue };
        let extent = (smd.map_size - 1) as f32 * smd.unit_dist;
        let walls = WallIndex::new(&faces, extent);
        let mut bad = Vec::new();
        let mut cells = 0u32;
        for x in 0..smd.map_size {
            for z in 0..smd.map_size {
                if !walkable(&smd, x, z) {
                    continue;
                }
                cells += 1;
                let v = cell_violations(&smd, &walls, x, z);
                if v != Violations::default() {
                    bad.push((x, z, v));
                }
            }
        }
        eprintln!("{name} (zone {zone}): {cells} bot-walkable cells, {} violate the client movement model", bad.len());
        assert!(
            bad.is_empty(),
            "{name}: {} bot-walkable cells a player could not stand in, e.g. {:?}",
            bad.len(),
            bad.iter().take(5).collect::<Vec<_>>()
        );
        assert!(cells > 20_000, "{name}: suspiciously few walkable cells ({cells})");
    }
}

/// Quantifies, per map, what the pre-fix mask let bots do that players
/// cannot, restricted to the area the bots can actually reach from their
/// spawn points. Informational numbers plus a hard "the old mask really did
/// leak" check so the report cannot silently rot.
#[test]
fn legacy_mask_leaked_into_unwalkable_areas() {
    for (name, zone) in MAPS {
        let Some((smd, faces)) = load(name) else { continue };
        let extent = (smd.map_size - 1) as f32 * smd.unit_dist;
        let walls = WallIndex::new(&faces, extent);
        let legacy = legacy_blocked(&smd, &faces);
        let legacy_ok = |x: i32, z: i32| smd.get_event_id(x, z) > 0 && !legacy[(x * smd.map_size + z) as usize];
        let new_ok = |x: i32, z: i32| walkable(&smd, x, z);
        let starts: Vec<_> = start_points(zone)
            .iter()
            .filter_map(|&(x, z)| nearest_walkable(&smd, &legacy_ok, x, z))
            .collect();
        let reach_old = flood(&smd, &legacy_ok, &starts);
        let starts_new: Vec<_> = start_points(zone)
            .iter()
            .filter_map(|&(x, z)| nearest_walkable(&smd, &new_ok, x, z))
            .collect();
        let reach_new = flood(&smd, &new_ok, &starts_new);
        let (mut old_cells, mut steep, mut wall, mut edge, mut any) = (0, 0, 0, 0, 0);
        let mut only_old = 0;
        for x in 0..smd.map_size {
            for z in 0..smd.map_size {
                let i = (x * smd.map_size + z) as usize;
                if !reach_old[i] {
                    continue;
                }
                old_cells += 1;
                if !reach_new[i] {
                    only_old += 1;
                }
                let v = cell_violations(&smd, &walls, x, z);
                steep += (v.steep > 0) as u32;
                wall += (v.wall > 0) as u32;
                edge += (v.edge > 0) as u32;
                any += (v != Violations::default()) as u32;
            }
        }
        eprintln!(
            "{name} (zone {zone}): legacy-reachable cells {old_cells}; with a point steeper than 40deg: {steep}; \
             within the body radius of a wall: {wall}; in the map-edge band: {edge}; any violation: {any}; \
             reachable by legacy but not by the new mask: {only_old}"
        );
        assert!(any > 0, "{name}: expected the legacy mask to violate the client rules somewhere");
    }
}

// ── debug rendering ─────────────────────────────────────────────────────

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Minimal uncompressed (stored-deflate) PNG writer, RGB8.
fn write_png(path: &Path, w: u32, h: u32, rgb: &[u8]) {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((w * 3 + 1) as usize * h as usize);
    for row in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[row * w as usize * 3..(row + 1) * w as usize * 3]);
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65_535).peekable();
    while let Some(c) = chunks.next() {
        z.push(chunks.peek().is_none() as u8);
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png).expect("write png");
}

/// Writes one PNG per map when `KO_NAV_DEBUG_DIR` is set. Colours: grey =
/// walkable now; green = walkable under both masks and reachable from the
/// spawns; red = reachable only under the legacy mask (leak); orange =
/// terrain between 40 and 45 degrees; dark = blocked.
#[test]
fn render_debug_maps() {
    let Some(dir) = std::env::var_os("KO_NAV_DEBUG_DIR") else { return };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, zone) in MAPS {
        let Some((smd, faces)) = load(name) else { continue };
        let legacy = legacy_blocked(&smd, &faces);
        let legacy_ok = |x: i32, z: i32| smd.get_event_id(x, z) > 0 && !legacy[(x * smd.map_size + z) as usize];
        let new_ok = |x: i32, z: i32| walkable(&smd, x, z);
        let starts = |m: &dyn Fn(i32, i32) -> bool| -> Vec<(i32, i32)> {
            start_points(zone).iter().filter_map(|&(x, z)| nearest_walkable(&smd, &m, x, z)).collect()
        };
        let reach_old = flood(&smd, &legacy_ok, &starts(&legacy_ok));
        let reach_new = flood(&smd, &new_ok, &starts(&new_ok));
        let size = smd.map_size as usize;
        let scale = if size < 300 { 4 } else { 2 };
        let (w, h) = (size * scale, size * scale);
        let mut rgb = vec![0u8; w * h * 3];
        for x in 0..size {
            for z in 0..size {
                let i = x * size + z;
                let q = smd.heights[i];
                let steep_band = x + 1 < size && z + 1 < size && {
                    let s = client_terrain_slope_sin(&smd.heights, smd.map_size, smd.unit_dist, (x as f32 + 0.5) * smd.unit_dist, (z as f32 + 0.5) * smd.unit_dist)
                        .unwrap_or(1.0);
                    s > CLIENT_SLOPE_SIN_LIMIT && s <= 0.7071
                };
                let colour = if reach_old[i] && !reach_new[i] {
                    [255, 40, 40]
                } else if reach_new[i] {
                    [60, 170, 60]
                } else if new_ok(x as i32, z as i32) {
                    [110, 110, 110]
                } else if steep_band {
                    [255, 160, 0]
                } else {
                    let g = (q.clamp(-30.0, 210.0) + 30.0) / 240.0;
                    let v = (20.0 + g * 60.0) as u8;
                    [v, v, v]
                };
                for sx in 0..scale {
                    for sz in 0..scale {
                        // x to the right, z up.
                        let (px, py) = (x * scale + sx, (size - 1 - z) * scale + (scale - 1 - sz));
                        let o = (py * w + px) * 3;
                        rgb[o..o + 3].copy_from_slice(&colour);
                    }
                }
            }
        }
        write_png(&dir.join(format!("zone{zone}_{}.png", name.trim_end_matches(".smd"))), w as u32, h as u32, &rgb);
    }
}
