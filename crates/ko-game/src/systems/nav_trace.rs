//! Client-derived bot navigation data and the real-player trace heatmap.
//!
//! * `Map/nav/<zone>.nav`    — authoritative walkability built from the client
//!   map files by `navgen` (`cargo run -p ko-protocol --bin navgen`), loaded
//!   at startup into `MapData` and reloadable with `+navreload`.
//! * `Map/nav/<zone>.walked` — cells/edges real players (not GMs, no
//!   teleports) have walked, recorded by the move handler, flushed here every
//!   few minutes and loaded at startup. Once a zone has enough of it bots
//!   prefer walked cells (A* cost bonus); they still never enter a cell the
//!   client mask blocks.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use ko_protocol::nav::{render_grid_png, NavMask, WalkedMap};
use tokio::time::interval;

use crate::world::WorldState;
use crate::zone::MapData;

/// Flush interval for the walked bitsets.
const WALKED_FLUSH_SECS: u64 = 5 * 60;

static NAV_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Directory holding `<zone>.nav` / `<zone>.walked` (`<map_dir>/nav`).
pub fn nav_dir() -> PathBuf {
    NAV_DIR.get().cloned().unwrap_or_else(|| PathBuf::from("Map/nav"))
}

pub fn set_nav_dir(dir: PathBuf) {
    let _ = NAV_DIR.set(dir);
}

pub fn nav_path(dir: &Path, zone_id: u16) -> PathBuf {
    dir.join(format!("{zone_id}.nav"))
}

pub fn walked_path(dir: &Path, zone_id: u16) -> PathBuf {
    dir.join(format!("{zone_id}.walked"))
}

/// Install the `.nav` mask (if the file exists) and the saved `.walked` trace
/// for one zone. Returns a one-line status for logs / GM replies.
pub fn load_zone_nav(map: &MapData, dir: &Path, zone_id: u16) -> String {
    let nav_file = nav_path(dir, zone_id);
    let mut status = if nav_file.exists() {
        match NavMask::load(&nav_file) {
            Ok(mask) => {
                let s = format!(
                    "zone {zone_id}: client nav {}x{} ({} walkable cells) from {}",
                    mask.size,
                    mask.size,
                    mask.walkable_count(),
                    nav_file.display()
                );
                map.set_nav(Some(mask));
                s
            }
            Err(e) => {
                map.set_nav(None);
                format!("zone {zone_id}: {} unreadable ({e}), SMD mask in use", nav_file.display())
            }
        }
    } else {
        map.set_nav(None);
        format!("zone {zone_id}: no {} , SMD mask in use", nav_file.display())
    };
    let walked_file = walked_path(dir, zone_id);
    if walked_file.exists() {
        let nav = map.bot_nav();
        match WalkedMap::load(&walked_file, nav.grid_size, map.unit_dist()) {
            Ok(w) => {
                let n = w.walked_count();
                match map.set_walked(w) {
                    Ok(()) => status.push_str(&format!("; walked trace {n} cells")),
                    Err(_) => status.push_str("; walked trace grid mismatch, discarded"),
                }
            }
            Err(e) => status.push_str(&format!("; walked trace unreadable ({e})")),
        }
    }
    status
}

/// Record a validated real-player move (the move handler calls this for
/// non-GM players only). Cheap: two atomic ORs per crossed cell.
pub fn record_player_move(world: &WorldState, zone_id: u16, from: (f32, f32), to: (f32, f32)) {
    if let Some(zone) = world.get_zone(zone_id) {
        if let Some(map) = zone.map_data.as_ref() {
            map.record_player_move(from, to);
        }
    }
}

/// Persist every zone's trace that changed since the last flush. Returns the
/// number of files written.
pub fn flush_walked(world: &WorldState) -> usize {
    let dir = nav_dir();
    let mut written = 0;
    for zone_id in world.get_all_zone_ids() {
        let Some(zone) = world.get_zone(zone_id) else { continue };
        let Some(map) = zone.map_data.as_ref() else { continue };
        let nav = map.bot_nav();
        if !nav.walked.take_dirty() {
            continue;
        }
        match nav.walked.save(&walked_path(&dir, zone_id)) {
            Ok(()) => written += 1,
            Err(e) => tracing::warn!(zone_id, error = %e, "nav: failed to write walked trace"),
        }
    }
    written
}

/// Background task: flush the walked traces every few minutes.
pub fn start_nav_trace_flush_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(WALKED_FLUSH_SECS));
        tick.tick().await;
        loop {
            tick.tick().await;
            let world = world.clone();
            let n = tokio::task::spawn_blocking(move || flush_walked(&world)).await.unwrap_or(0);
            if n > 0 {
                tracing::info!(files = n, "nav: walked traces flushed");
            }
        }
    })
}

/// Re-read `<zone>.nav` (and `.walked`) from disk for one zone or, with
/// `None`, for every zone that has map data. Returns the status lines.
pub fn reload(world: &WorldState, zone: Option<u16>) -> Vec<String> {
    let dir = nav_dir();
    flush_walked(world);
    let zones = match zone {
        Some(z) => vec![z],
        None => {
            let mut z = world.get_all_zone_ids();
            z.sort_unstable();
            z
        }
    };
    let mut out = Vec::new();
    for zone_id in zones {
        let Some(zone) = world.get_zone(zone_id) else {
            out.push(format!("zone {zone_id}: unknown"));
            continue;
        };
        let Some(map) = zone.map_data.as_ref() else {
            out.push(format!("zone {zone_id}: no map data"));
            continue;
        };
        out.push(load_zone_nav(map, &dir, zone_id));
    }
    out
}

/// One-line navigation summary for a zone.
pub fn stats(world: &WorldState, zone_id: u16) -> String {
    let Some(zone) = world.get_zone(zone_id) else { return format!("zone {zone_id}: unknown") };
    let Some(map) = zone.map_data.as_ref() else { return format!("zone {zone_id}: no map data") };
    let nav = map.bot_nav();
    let source = match &nav.nav {
        Some(m) => format!("client nav {}x{}, {} walkable", m.size, m.size, m.walkable_count()),
        None => format!("SMD mask {}x{}", map.grid_size(), map.grid_size()),
    };
    let walked = nav.walked.walked_count();
    format!(
        "zone {zone_id}: {source}; walked trace {walked} cells ({}, threshold {}); bots {}",
        if nav.walked_ready() { "active" } else { "collecting" },
        crate::zone::walked_min_cells(),
        world.get_bots_in_zone_live(zone_id).len()
    )
}

/// Write a PNG of the zone's bot mask with the walked trace and the current
/// bot positions to `<nav_dir>/debug/zone<id>_<unix>.png`.
pub fn dump_png(world: &WorldState, zone_id: u16) -> anyhow::Result<PathBuf> {
    let zone = world.get_zone(zone_id).ok_or_else(|| anyhow::anyhow!("zone {zone_id}: unknown"))?;
    let map = zone.map_data.as_ref().ok_or_else(|| anyhow::anyhow!("zone {zone_id}: no map data"))?;
    let nav = map.bot_nav();
    let size = nav.grid_size;
    let unit = map.unit_dist();
    let bots: Vec<(i32, i32)> = world
        .get_bots_in_zone_live(zone_id)
        .iter()
        .map(|b| ((b.x / unit) as i32, (b.z / unit) as i32))
        .collect();
    let players: Vec<(i32, i32)> = world
        .sessions_in_zone(zone_id)
        .into_iter()
        .filter_map(|sid| world.with_session(sid, |h| ((h.position.x / unit) as i32, (h.position.z / unit) as i32)))
        .collect();
    let scale = if size < 300 { 3 } else { 2 };
    let near = |pts: &[(i32, i32)], x: i32, z: i32, r: i32| pts.iter().any(|&(bx, bz)| (bx - x).abs() <= r && (bz - z).abs() <= r);
    let png = render_grid_png(size, scale, |x, z| {
        if near(&bots, x, z, 1) {
            [255, 230, 0]
        } else if near(&players, x, z, 1) {
            [0, 220, 255]
        } else if map.is_bot_movable_grid(x, z) {
            if nav.walked.is_walked(x, z) { [60, 190, 60] } else { [120, 120, 120] }
        } else if nav.walked.is_walked(x, z) {
            [255, 40, 40]
        } else {
            [25, 25, 25]
        }
    });
    let dir = nav_dir().join("debug");
    std::fs::create_dir_all(&dir)?;
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let path = dir.join(format!("zone{zone_id}_{ts}.png"));
    std::fs::write(&path, png)?;
    Ok(path)
}
