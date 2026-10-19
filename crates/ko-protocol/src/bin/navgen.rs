//! `navgen` — build `Map/nav/<zone>.nav` bot walkability masks from the
//! client's own map files (`Zones/<map>.gtd` terrain + `Zones/<map>.opd`
//! baked collision faces), and report how they differ from the mask the
//! server derives from its `.smd` files.
//!
//! ```text
//! cargo run -p ko-protocol --bin navgen -- \
//!     --client "D:/ai/JstKO Client" --map-dir Map --out Map/nav \
//!     --debug <scratch>/nav-debug --zones 71,72,73
//! ```
//! Without `--zones` every zone in `ko_protocol::nav::ZONE_CLIENT_MAPS` whose
//! client files exist is generated. See `ko_protocol::nav` for the formats.

use ko_protocol::nav::{
    client_map_for_zone, find_client_file, render_grid_png, ClientCollision, ClientTerrain, NavMask,
    ZONE_CLIENT_MAPS,
};
use ko_protocol::smd::SmdFile;
use std::collections::VecDeque;
use std::path::PathBuf;

struct Args {
    client: PathBuf,
    map_dir: PathBuf,
    out: PathBuf,
    debug: Option<PathBuf>,
    zones: Vec<u16>,
}

fn parse_args() -> Args {
    let mut a = Args {
        client: PathBuf::from("D:/ai/JstKO Client"),
        map_dir: PathBuf::from("Map"),
        out: PathBuf::from("Map/nav"),
        debug: None,
        zones: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let v = it.next();
        match (k.as_str(), v) {
            ("--client", Some(v)) => a.client = v.into(),
            ("--map-dir", Some(v)) => a.map_dir = v.into(),
            ("--out", Some(v)) => a.out = v.into(),
            ("--debug", Some(v)) => a.debug = Some(v.into()),
            ("--zones", Some(v)) => a.zones = v.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
            _ => {
                eprintln!("usage: navgen [--client DIR] [--map-dir Map] [--out Map/nav] [--debug DIR] [--zones 71,72,73]");
                std::process::exit(2);
            }
        }
    }
    if a.zones.is_empty() {
        a.zones = ZONE_CLIENT_MAPS.iter().map(|(z, _, _)| *z).collect();
    }
    a
}

/// Bot start anchors used for the reachability report (same points the bot
/// AI spawns at; see `bot_ai::get_bot_respawn_position`).
fn anchors(zone: u16) -> &'static [(f32, f32)] {
    match zone {
        71 => &[(1375.0, 1098.0), (622.0, 898.0), (1050.0, 842.0), (886.0, 1102.0)],
        72 => &[(851.0, 136.0), (190.0, 897.0)],
        73 => &[(515.0, 104.0), (513.0, 916.0)],
        // BDW: Karus / El Morad spawn rooms, altar, delivery bases.
        84 => &[(51.0, 58.0), (201.0, 207.0), (127.0, 131.0), (31.0, 131.0), (223.0, 131.0)],
        // Chaos: start position and the outer ring.
        85 => &[(126.0, 130.0), (40.0, 40.0), (216.0, 216.0)],
        _ => &[],
    }
}

fn flood(size: i32, ok: &dyn Fn(i32, i32) -> bool, starts: &[(i32, i32)]) -> Vec<bool> {
    let mut seen = vec![false; (size * size) as usize];
    let mut q = VecDeque::new();
    for &(x, z) in starts {
        if ok(x, z) && !seen[(x * size + z) as usize] {
            seen[(x * size + z) as usize] = true;
            q.push_back((x, z));
        }
    }
    while let Some((x, z)) = q.pop_front() {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let (nx, nz) = (x + dx, z + dz);
                if (dx == 0 && dz == 0) || nx < 0 || nz < 0 || nx >= size || nz >= size {
                    continue;
                }
                if seen[(nx * size + nz) as usize] || !ok(nx, nz) {
                    continue;
                }
                if dx != 0 && dz != 0 && !(ok(x + dx, z) && ok(x, z + dz)) {
                    continue;
                }
                seen[(nx * size + nz) as usize] = true;
                q.push_back((nx, nz));
            }
        }
    }
    seen
}

fn nearest(size: i32, ok: &dyn Fn(i32, i32) -> bool, wx: f32, wz: f32, unit: f32) -> Option<(i32, i32)> {
    let (cx, cz) = ((wx / unit) as i32, (wz / unit) as i32);
    for r in 0..=32i32 {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs().max(dz.abs()) != r {
                    continue;
                }
                let (x, z) = (cx + dx, cz + dz);
                if x >= 0 && z >= 0 && x < size && z < size && ok(x, z) {
                    return Some((x, z));
                }
            }
        }
    }
    None
}

fn main() {
    let args = parse_args();
    std::fs::create_dir_all(&args.out).expect("create out dir");
    if let Some(d) = &args.debug {
        std::fs::create_dir_all(d).expect("create debug dir");
    }
    let mut failures = 0;
    for zone in &args.zones {
        let Some((client_name, smd_name)) = client_map_for_zone(*zone) else {
            eprintln!("zone {zone}: no client map known, skipped");
            failures += 1;
            continue;
        };
        let zones_dir = args.client.join("Zones");
        let (Some(gtd), Some(opd)) =
            (find_client_file(&zones_dir, client_name, "gtd"), find_client_file(&zones_dir, client_name, "opd"))
        else {
            eprintln!("zone {zone}: {client_name}.gtd/.opd not found under {}", zones_dir.display());
            failures += 1;
            continue;
        };
        let terrain = match ClientTerrain::load(&gtd) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("zone {zone}: {}: {e}", gtd.display());
                failures += 1;
                continue;
            }
        };
        let collision = match ClientCollision::load(&opd) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("zone {zone}: {}: {e}", opd.display());
                failures += 1;
                continue;
            }
        };
        if (collision.map_width - terrain.unit * (terrain.size - 1) as f32).abs() > 0.5 {
            eprintln!(
                "zone {zone}: WARNING terrain {} samples ({} m) but collision says {} m",
                terrain.size,
                terrain.unit * (terrain.size - 1) as f32,
                collision.map_width
            );
        }
        let mask = NavMask::build(&terrain, &collision.faces);
        let out = args.out.join(format!("{zone}.nav"));
        mask.save(&out).expect("write nav");
        let size = mask.size;
        let cells = (size * size) as usize;
        println!(
            "zone {zone} <- {client_name}: {size}x{size} ({} m), {} collision faces, walkable {} / {} cells ({:.1}%) -> {}",
            mask.map_width(),
            collision.faces.len(),
            mask.walkable_count(),
            cells,
            mask.walkable_count() as f32 * 100.0 / cells as f32,
            out.display()
        );

        // Reachability from the bot anchors under the new mask.
        let nav_ok = |x: i32, z: i32| mask.is_walkable(x, z);
        let starts: Vec<_> =
            anchors(*zone).iter().filter_map(|&(x, z)| nearest(size, &nav_ok, x, z, mask.unit)).collect();
        let reach = if starts.is_empty() { Vec::new() } else { flood(size, &nav_ok, &starts) };
        if !reach.is_empty() {
            let reachable = reach.iter().filter(|&&r| r).count();
            // Quadrant report (x right, z up): SE = high x, low z.
            let half = size / 2;
            let mut q = [[0usize; 2]; 4];
            for x in 0..size {
                for z in 0..size {
                    let i = (x * size + z) as usize;
                    let qi = match (x >= half, z >= half) {
                        (false, true) => 0,  // NW
                        (true, true) => 1,   // NE
                        (false, false) => 2, // SW
                        (true, false) => 3,  // SE
                    };
                    q[qi][0] += mask.is_walkable(x, z) as usize;
                    q[qi][1] += reach[i] as usize;
                }
            }
            println!(
                "  reachable from bot anchors: {reachable} cells; per quadrant walkable/reachable NW {}/{} NE {}/{} SW {}/{} SE {}/{}",
                q[0][0], q[0][1], q[1][0], q[1][1], q[2][0], q[2][1], q[3][0], q[3][1]
            );
        }

        // Compare with the SMD-derived mask the server used so far.
        let smd_path = args.map_dir.join(smd_name);
        let smd = match SmdFile::load(&smd_path) {
            Ok(s) => Some(s),
            Err(e) => {
                println!("  no SMD comparison ({}: {e})", smd_path.display());
                None
            }
        };
        let mut smd_ok: Option<Box<dyn Fn(i32, i32) -> bool>> = None;
        if let Some(smd) = &smd {
            if smd.map_size != size {
                println!(
                    "  SMD {smd_name} is {}x{} ({} m) but the client map is {size}x{size}: the server map is a DIFFERENT map, no cell comparison possible",
                    smd.map_size, smd.map_size, smd.map_width
                );
            } else {
                let mut height_diff = 0usize;
                let mut max_diff = 0f32;
                for (a, b) in smd.heights.iter().zip(&mask.heights) {
                    let d = (a - b).abs();
                    if d > 0.01 {
                        height_diff += 1;
                        max_diff = max_diff.max(d);
                    }
                }
                let has_events = smd.event_grid.iter().any(|&v| v != 0);
                let smd_walk = move |x: i32, z: i32| {
                    x >= 0
                        && z >= 0
                        && x < smd.map_size
                        && z < smd.map_size
                        && (!has_events || smd.get_event_id(x, z) > 0)
                        && !smd.bot_blocked[(x * smd.map_size + z) as usize]
                };
                let (mut both, mut only_smd, mut only_nav) = (0usize, 0usize, 0usize);
                for x in 0..size {
                    for z in 0..size {
                        match (smd_walk(x, z), mask.is_walkable(x, z)) {
                            (true, true) => both += 1,
                            (true, false) => only_smd += 1,
                            (false, true) => only_nav += 1,
                            _ => {}
                        }
                    }
                }
                println!(
                    "  vs {smd_name}: heights differ in {height_diff} samples (max {max_diff:.2} m); walkable in both {both}, \
                     SMD-only (old mask let bots in, client blocks) {only_smd}, client-only (old mask over-blocked) {only_nav}"
                );
                smd_ok = Some(Box::new(smd_walk));
            }
        }

        if let Some(dir) = &args.debug {
            let scale = if size < 300 { 3 } else { 2 };
            let (hmin, hmax) = mask
                .heights
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &h| (lo.min(h), hi.max(h)));
            let png = render_grid_png(size, scale, |x, z| {
                let i = (x * size + z) as usize;
                let nav = mask.is_walkable(x, z);
                let old = smd_ok.as_ref().map(|f| f(x, z));
                match (nav, old) {
                    (true, Some(false)) => [70, 110, 255],
                    (false, Some(true)) => [255, 40, 40],
                    (true, _) if !reach.is_empty() && reach[i] => [60, 170, 60],
                    (true, _) => [120, 120, 120],
                    _ => {
                        let g = ((mask.heights[i] - hmin) / (hmax - hmin).max(1.0)).clamp(0.0, 1.0);
                        let v = (15.0 + g * 60.0) as u8;
                        [v, v, v]
                    }
                }
            });
            let p = dir.join(format!("zone{zone}_{client_name}.png"));
            std::fs::write(&p, png).expect("write png");
            println!(
                "  debug map {} (green: walkable & reachable from bot anchors, grey: walkable, red: old SMD mask only, blue: client only, dark: blocked)",
                p.display()
            );
        }
    }
    if failures > 0 {
        std::process::exit(1);
    }
}
