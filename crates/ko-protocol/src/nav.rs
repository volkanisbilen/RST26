//! Client-derived bot navigation data.
//!
//! The server `.smd` files are not what the 2625 client renders and collides
//! against: `freezone_b.smd` (zone 71) is an older Ronark Land terrain (190k of
//! 263k height samples differ from the client's `Freezone_b.gtd`),
//! `freezone_c.smd` (zone 73) is a copy of `freezone_b.smd` (2048 m) while the
//! client's `FreeZone_c` is a 1024 m map, and `freezone_a.smd` carries 1.6k
//! garbage collision faces and only 9.7k sane ones against the 12.6k faces in
//! `FreeZone_a.opd`. No rule layered on top of that data can be right, so the
//! bot mask is built from the client's own map files instead and shipped as
//! `Map/nav/<zone>.nav`.
//!
//! ## Client formats (decoded from `D:\ai\JstKO Client\Zones`, verified by
//! matching `FreeZone_a.gtd` heights against `freezone_a.smd`: 65,279 of
//! 66,049 samples identical, the rest local edits)
//!
//! `.gtd` (`CN3Terrain`):
//! ```text
//! i32 name_len, u8[name_len] obfuscated file name   (CN3BaseFileAccess)
//! i32 flag                                           (0 or 1, format marker)
//! MAPDATA[size*size]  { u32 texture bitfield; f32 height }   x-major: [x*size+z]
//! ... patch bounds / tile texture tables / lightmaps (not needed)
//! ```
//! `size` is not stored; it is 257 (1024 m maps) or 513 (2048 m maps), i.e.
//! `map_width / 4 + 1`, and is recovered from the file length. The terrain
//! unit is the engine constant `TERRAIN_UNIT = 4.0`.
//!
//! `.opd` (`CN3ShapeMgr`):
//! ```text
//! i32 name_len, u8[name_len] obfuscated file name
//! i32 version
//! f32 map_width, f32 map_length
//! i32 face_count, [3 x (f32 x, y, z)] * face_count       baked collision faces
//! cell grid (ceil(width/16) x ceil(length/16)):
//!     u32 exists; if exists: i32 shape_ref_count, u16[...] shape refs,
//!     16 x { i32 poly_count, u32[poly_count*3] face vertex indices }
//! i32 shape_count, shapes (version, name, transform, parts ...)   (not needed)
//! ```
//! The client's movement collision (`sub_6B0510`, swept sphere r = 1.124 vs
//! triangles, and `sub_6B20C0`, ray vs triangles) walks the cell lists into
//! this baked face array (`this+88` / `v149[22]`), never the shape meshes, so
//! the faces above are the complete authoritative object collision. A face
//! only blocks when its normal is wall-like (`hypot(nx, nz) > 0.643`); the
//! player always stands on the terrain height (`sub_8C7B40`), there is no
//! stepping onto object tops, so object tops are never walkable surfaces and
//! shallow object faces (ramps, floors) are ignored exactly as the client
//! ignores them.
//!
//! ## `.nav` file
//! ```text
//! "KONV" u32 version=1, i32 size, f32 unit, u32 flags (bit0: heights present)
//! f32[size*size] heights (x-major)
//! u64[ceil(size*size/64)] blocked bitset (bit = x*size+z)
//! ```
//! ## `.walked` file (player trace heatmap)
//! ```text
//! "KOWK" u32 version=1, i32 size
//! u64[ceil(size*size/64)] cells a real player stood in
//! u64[ceil(size*size*4/64)] edges walked: bit = (x*size+z)*4 + dir,
//!     dir 0: (x,z)->(x+1,z), 1: ->(x,z+1), 2: ->(x+1,z+1), 3: ->(x-1,z+1)
//! ```

use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::smd::{
    collision_face_is_sane, rasterize_wall_faces, read_f32, read_i32, read_shape_mgr_collision,
    terrain_quad_blocked, CollisionFace,
};

/// Engine constant: distance between two terrain samples (`TERRAIN_UNIT`).
pub const CLIENT_TERRAIN_UNIT: f32 = 4.0;

const NAV_MAGIC: &[u8; 4] = b"KONV";
const NAV_VERSION: u32 = 1;
const WALKED_MAGIC: &[u8; 4] = b"KOWK";
const WALKED_VERSION: u32 = 1;

/// Which client map a server zone shows. Decoded from the client's
/// `Data/Zones.tbl` (row id = zone * 10, column 1 = `Zones\<name>.gtd`) and
/// the server `zone_info.smd_name` column.
pub const ZONE_CLIENT_MAPS: &[(u16, &str, &str)] = &[
    (1, "karus2004", "karus2004.smd"),
    (2, "elmo2004", "elmo2004.smd"),
    (11, "EslantZone", "k_eslant.smd"),
    (12, "EslantZone", "e_eslant.smd"),
    (21, "Moradon", "moradon_0826.smd"),
    (71, "Freezone_b", "freezone_b.smd"),
    (72, "FreeZone_a", "freezone_a.smd"),
    (73, "FreeZone_c", "freezone_c.smd"),
    (84, "In_dungeon04", "In_dungeon04.smd"),
    (85, "sky_war_2009", "sky_war_2009.smd"),
    (87, "In_dungeon05", "In_Dungeon05.smd"),
];

/// `(client map base name, server smd name)` for a zone, if known.
pub fn client_map_for_zone(zone: u16) -> Option<(&'static str, &'static str)> {
    ZONE_CLIENT_MAPS.iter().find(|(z, _, _)| *z == zone).map(|(_, c, s)| (*c, *s))
}

/// Case-insensitive lookup of `<dir>/<base>.<ext>` (the client ships mixed
/// case names, e.g. `FreeZone_a.gtd` next to `Freezone_a.opdext`).
pub fn find_client_file(dir: &Path, base: &str, ext: &str) -> Option<std::path::PathBuf> {
    let want = format!("{base}.{ext}").to_ascii_lowercase();
    let direct = dir.join(format!("{base}.{ext}"));
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().to_ascii_lowercase() == want)
        .map(|e| e.path())
}

// ── client terrain (.gtd) ───────────────────────────────────────────────

/// Terrain heights of a client `.gtd`, x-major like the server SMD.
#[derive(Debug, Clone)]
pub struct ClientTerrain {
    pub size: i32,
    pub unit: f32,
    pub heights: Vec<f32>,
}

impl ClientTerrain {
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::parse(&std::fs::read(path)?)
    }

    /// Parse a `.gtd`. The sample count is not stored in the file; the
    /// largest `(4n+1)^2` MAPDATA block that fits after the header is used,
    /// which recovers 257 and 513 for every shipped map.
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut r = Cursor::new(bytes);
        let name_len = read_i32(&mut r)?;
        if !(0..=260).contains(&name_len) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "gtd: bad name length"));
        }
        r.seek(SeekFrom::Current(name_len as i64))?;
        let _flag = read_i32(&mut r)?;
        let start = r.position() as usize;
        let avail = bytes.len().saturating_sub(start) / 8;
        let size = [1025i32, 513, 257, 129, 65, 33]
            .into_iter()
            .find(|s| (*s as usize) * (*s as usize) <= avail)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "gtd: too small for a terrain"))?;
        let n = (size * size) as usize;
        let mut heights = Vec::with_capacity(n);
        for i in 0..n {
            let o = start + i * 8 + 4;
            let h = f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
            if !h.is_finite() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("gtd: non-finite height at sample {i}")));
            }
            heights.push(h);
        }
        Ok(Self { size, unit: CLIENT_TERRAIN_UNIT, heights })
    }

    pub fn height_at(&self, x: i32, z: i32) -> Option<f32> {
        (x >= 0 && z >= 0 && x < self.size && z < self.size).then(|| self.heights[(x * self.size + z) as usize])
    }
}

// ── client object collision (.opd) ──────────────────────────────────────

/// Collision block of a client `.opd`.
#[derive(Debug, Clone)]
pub struct ClientCollision {
    pub map_width: f32,
    pub map_length: f32,
    pub faces: Vec<CollisionFace>,
}

impl ClientCollision {
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::parse(&std::fs::read(path)?)
    }

    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut r = Cursor::new(bytes);
        let name_len = read_i32(&mut r)?;
        if !(0..=260).contains(&name_len) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "opd: bad name length"));
        }
        r.seek(SeekFrom::Current(name_len as i64))?;
        let _version = read_i32(&mut r)?;
        // Peek the width so the sanity bounds match the map.
        let pos = r.position();
        let width = read_f32(&mut r)?;
        r.seek(SeekFrom::Start(pos))?;
        if !(width.is_finite() && width > 0.0 && width <= 8192.0) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "opd: bad map width"));
        }
        let (map_width, map_length, faces) = read_shape_mgr_collision(&mut r, width)?;
        Ok(Self { map_width, map_length, faces })
    }
}

// ── nav mask ────────────────────────────────────────────────────────────

/// Authoritative per-cell bot walkability derived from the client files.
#[derive(Clone)]
pub struct NavMask {
    pub size: i32,
    pub unit: f32,
    /// Terrain heights (x-major), the ones the client stands players on.
    pub heights: Vec<f32>,
    blocked: Vec<u64>,
}

impl std::fmt::Debug for NavMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavMask")
            .field("size", &self.size)
            .field("unit", &self.unit)
            .field("walkable_cells", &self.walkable_count())
            .finish()
    }
}

fn bitset_words(bits: usize) -> usize {
    bits.div_ceil(64)
}

impl NavMask {
    /// Build the mask with the client's movement rules: terrain triangles
    /// steeper than 40 degrees (`sin > 0.643`) and the 4 m map-edge band from
    /// the terrain, plus every cell a wall-like collision face reaches into
    /// within the body radius (1.124 m) over the body's vertical extent
    /// (feet + 0.5 .. feet + 2.848). See `smd.rs` "Client movement rules".
    pub fn build(terrain: &ClientTerrain, faces: &[CollisionFace]) -> Self {
        let (size, unit) = (terrain.size, terrain.unit);
        let mut blocked = vec![true; terrain.heights.len()];
        for x in 0..size - 1 {
            for z in 0..size - 1 {
                blocked[(x * size + z) as usize] = terrain_quad_blocked(&terrain.heights, size, unit, x, z);
            }
        }
        let extent = (size - 1) as f32 * unit;
        let sane: Vec<CollisionFace> = faces.iter().copied().filter(|f| collision_face_is_sane(f, extent)).collect();
        rasterize_wall_faces(&sane, size, unit, &terrain.heights, &mut blocked);
        Self::from_bools(size, unit, terrain.heights.clone(), &blocked)
    }

    pub fn from_bools(size: i32, unit: f32, heights: Vec<f32>, blocked: &[bool]) -> Self {
        let n = (size * size) as usize;
        assert_eq!(blocked.len(), n);
        let mut words = vec![0u64; bitset_words(n)];
        for (i, &b) in blocked.iter().enumerate() {
            if b {
                words[i / 64] |= 1 << (i % 64);
            }
        }
        Self { size, unit, heights, blocked: words }
    }

    #[inline]
    pub fn in_grid(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.size && z < self.size
    }

    /// `true` when a player cannot stand in grid cell `(x, z)` (also for
    /// cells outside the grid).
    #[inline]
    pub fn is_blocked(&self, x: i32, z: i32) -> bool {
        if !self.in_grid(x, z) {
            return true;
        }
        let i = (x * self.size + z) as usize;
        self.blocked[i / 64] >> (i % 64) & 1 == 1
    }

    #[inline]
    pub fn is_walkable(&self, x: i32, z: i32) -> bool {
        !self.is_blocked(x, z)
    }

    pub fn walkable_count(&self) -> usize {
        let n = (self.size * self.size) as usize;
        n - self.blocked.iter().map(|w| w.count_ones() as usize).sum::<usize>()
    }

    pub fn map_width(&self) -> f32 {
        (self.size - 1) as f32 * self.unit
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(20 + self.heights.len() * 4 + self.blocked.len() * 8);
        out.extend_from_slice(NAV_MAGIC);
        out.extend_from_slice(&NAV_VERSION.to_le_bytes());
        out.extend_from_slice(&self.size.to_le_bytes());
        out.extend_from_slice(&self.unit.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        for h in &self.heights {
            out.extend_from_slice(&h.to_le_bytes());
        }
        for w in &self.blocked {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, format!("nav: {m}"));
        if bytes.len() < 20 || &bytes[0..4] != NAV_MAGIC {
            return Err(bad("bad magic"));
        }
        let mut r = Cursor::new(&bytes[4..]);
        let version = read_i32(&mut r)? as u32;
        if version != NAV_VERSION {
            return Err(bad("unsupported version"));
        }
        let size = read_i32(&mut r)?;
        let unit = read_f32(&mut r)?;
        let flags = read_i32(&mut r)? as u32;
        if !(2..=4097).contains(&size) || !(unit.is_finite() && unit > 0.0) {
            return Err(bad("bad header"));
        }
        let n = (size * size) as usize;
        let mut heights = Vec::new();
        if flags & 1 != 0 {
            heights.reserve(n);
            for _ in 0..n {
                heights.push(read_f32(&mut r)?);
            }
        }
        let mut blocked = vec![0u64; bitset_words(n)];
        let mut buf = [0u8; 8];
        for w in &mut blocked {
            r.read_exact(&mut buf)?;
            *w = u64::from_le_bytes(buf);
        }
        Ok(Self { size, unit, heights, blocked })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_bytes())
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        Self::from_bytes(&std::fs::read(path)?)
    }
}

// ── walked heatmap ──────────────────────────────────────────────────────

/// Longest player move (metres) accepted as a walk between two consecutive
/// move packets; anything longer is a teleport, respawn or lag jump.
pub const WALKED_MAX_STEP: f32 = 12.0;

/// Cells and cell edges real players have been seen walking. Lock-free; the
/// move handler marks, a periodic task persists.
pub struct WalkedMap {
    pub size: i32,
    pub unit: f32,
    cells: Vec<AtomicU64>,
    edges: Vec<AtomicU64>,
    dirty: AtomicBool,
}

impl std::fmt::Debug for WalkedMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalkedMap").field("size", &self.size).field("walked_cells", &self.walked_count()).finish()
    }
}

impl WalkedMap {
    pub fn new(size: i32, unit: f32) -> Self {
        let n = (size.max(1) * size.max(1)) as usize;
        Self {
            size,
            unit,
            cells: (0..bitset_words(n)).map(|_| AtomicU64::new(0)).collect(),
            edges: (0..bitset_words(n * 4)).map(|_| AtomicU64::new(0)).collect(),
            dirty: AtomicBool::new(false),
        }
    }

    #[inline]
    fn in_grid(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.size && z < self.size
    }

    #[inline]
    fn set(words: &[AtomicU64], bit: usize) {
        words[bit / 64].fetch_or(1 << (bit % 64), Ordering::Relaxed);
    }

    #[inline]
    fn get(words: &[AtomicU64], bit: usize) -> bool {
        words[bit / 64].load(Ordering::Relaxed) >> (bit % 64) & 1 == 1
    }

    pub fn mark_cell(&self, x: i32, z: i32) {
        if self.in_grid(x, z) {
            Self::set(&self.cells, (x * self.size + z) as usize);
            self.dirty.store(true, Ordering::Relaxed);
        }
    }

    pub fn is_walked(&self, x: i32, z: i32) -> bool {
        self.in_grid(x, z) && Self::get(&self.cells, (x * self.size + z) as usize)
    }

    /// Canonical `(cell, dir)` of the edge between two 8-neighbours.
    fn edge_bit(&self, a: (i32, i32), b: (i32, i32)) -> Option<usize> {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        if dx.abs() > 1 || dz.abs() > 1 || (dx == 0 && dz == 0) {
            return None;
        }
        // Orient so dz >= 0, and dx >= 0 when dz == 0.
        let (o, dx, dz) = if dz < 0 || (dz == 0 && dx < 0) { (b, -dx, -dz) } else { (a, dx, dz) };
        let dir = match (dx, dz) {
            (1, 0) => 0,
            (0, 1) => 1,
            (1, 1) => 2,
            (-1, 1) => 3,
            _ => return None,
        };
        (self.in_grid(o.0, o.1) && self.in_grid(b.0, b.1) && self.in_grid(a.0, a.1))
            .then(|| ((o.0 * self.size + o.1) as usize) * 4 + dir)
    }

    pub fn mark_edge(&self, a: (i32, i32), b: (i32, i32)) {
        if let Some(bit) = self.edge_bit(a, b) {
            Self::set(&self.edges, bit);
            self.dirty.store(true, Ordering::Relaxed);
        }
    }

    pub fn is_edge_walked(&self, a: (i32, i32), b: (i32, i32)) -> bool {
        self.edge_bit(a, b).is_some_and(|bit| Self::get(&self.edges, bit))
    }

    /// Record a validated player move `from -> to` (world coordinates). Marks
    /// every cell the segment crosses and the edges between consecutive
    /// cells; jumps longer than [`WALKED_MAX_STEP`] are ignored as noise.
    /// Returns `true` when something was recorded.
    pub fn mark_move(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        if ![from.0, from.1, to.0, to.1].iter().all(|v| v.is_finite()) || self.unit <= 0.0 {
            return false;
        }
        let dist = (to.0 - from.0).hypot(to.1 - from.1);
        if dist > WALKED_MAX_STEP {
            return false;
        }
        let cell = |p: (f32, f32)| ((p.0 / self.unit).floor() as i32, (p.1 / self.unit).floor() as i32);
        let (mut cur, end) = (cell(from), cell(to));
        if !self.in_grid(cur.0, cur.1) || !self.in_grid(end.0, end.1) {
            return false;
        }
        self.mark_cell(cur.0, cur.1);
        if cur == end {
            return true;
        }
        // Voxel traversal (same walk as `pathfind::bot_segment_walkable`).
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let step_x = dx.signum() as i32 * (dx != 0.0) as i32;
        let step_z = dz.signum() as i32 * (dz != 0.0) as i32;
        let t_delta_x = if dx != 0.0 { (self.unit / dx).abs() } else { f32::INFINITY };
        let t_delta_z = if dz != 0.0 { (self.unit / dz).abs() } else { f32::INFINITY };
        let next_x = if step_x > 0 { (cur.0 + 1) as f32 * self.unit } else { cur.0 as f32 * self.unit };
        let next_z = if step_z > 0 { (cur.1 + 1) as f32 * self.unit } else { cur.1 as f32 * self.unit };
        let mut t_max_x = if dx != 0.0 { (next_x - from.0) / dx } else { f32::INFINITY };
        let mut t_max_z = if dz != 0.0 { (next_z - from.1) / dz } else { f32::INFINITY };
        let max_steps = (end.0 - cur.0).unsigned_abs() + (end.1 - cur.1).unsigned_abs() + 2;
        for _ in 0..max_steps {
            let prev = cur;
            if t_max_x < t_max_z {
                cur.0 += step_x;
                t_max_x += t_delta_x;
            } else if t_max_z < t_max_x {
                cur.1 += step_z;
                t_max_z += t_delta_z;
            } else {
                cur.0 += step_x;
                cur.1 += step_z;
                t_max_x += t_delta_x;
                t_max_z += t_delta_z;
            }
            if !self.in_grid(cur.0, cur.1) {
                return true;
            }
            self.mark_cell(cur.0, cur.1);
            self.mark_edge(prev, cur);
            if cur == end {
                break;
            }
        }
        true
    }

    pub fn walked_count(&self) -> usize {
        self.cells.iter().map(|w| w.load(Ordering::Relaxed).count_ones() as usize).sum()
    }

    /// Clears the dirty flag and reports whether anything changed since the
    /// last call.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + (self.cells.len() + self.edges.len()) * 8);
        out.extend_from_slice(WALKED_MAGIC);
        out.extend_from_slice(&WALKED_VERSION.to_le_bytes());
        out.extend_from_slice(&self.size.to_le_bytes());
        for w in self.cells.iter().chain(self.edges.iter()) {
            out.extend_from_slice(&w.load(Ordering::Relaxed).to_le_bytes());
        }
        out
    }

    /// Load a `.walked` file for a grid of `size`; a file for a different
    /// grid size is rejected (the map changed, the trace is stale).
    pub fn from_bytes(bytes: &[u8], size: i32, unit: f32) -> io::Result<Self> {
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, format!("walked: {m}"));
        if bytes.len() < 12 || &bytes[0..4] != WALKED_MAGIC {
            return Err(bad("bad magic"));
        }
        let mut r = Cursor::new(&bytes[4..]);
        if read_i32(&mut r)? as u32 != WALKED_VERSION {
            return Err(bad("unsupported version"));
        }
        let file_size = read_i32(&mut r)?;
        if file_size != size {
            return Err(bad("grid size mismatch"));
        }
        let map = Self::new(size, unit);
        let mut buf = [0u8; 8];
        for w in map.cells.iter().chain(map.edges.iter()) {
            r.read_exact(&mut buf)?;
            w.store(u64::from_le_bytes(buf), Ordering::Relaxed);
        }
        Ok(map)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("walked.tmp");
        std::fs::write(&tmp, self.to_bytes())?;
        std::fs::rename(&tmp, path)
    }

    pub fn load(path: &Path, size: i32, unit: f32) -> io::Result<Self> {
        Self::from_bytes(&std::fs::read(path)?, size, unit)
    }
}

// ── PNG debug output ────────────────────────────────────────────────────

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

/// Minimal uncompressed (stored-deflate) RGB8 PNG encoder for debug maps.
pub fn encode_png_rgb(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
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
    png
}

/// Simple top-down raster of a grid: `colour(x, z)` per cell, `x` to the
/// right and `z` up, `scale` pixels per cell.
pub fn render_grid_png(size: i32, scale: usize, colour: impl Fn(i32, i32) -> [u8; 3]) -> Vec<u8> {
    let s = size as usize;
    let (w, h) = (s * scale, s * scale);
    let mut rgb = vec![0u8; w * h * 3];
    for x in 0..s {
        for z in 0..s {
            let c = colour(x as i32, z as i32);
            for sx in 0..scale {
                for sz in 0..scale {
                    let (px, py) = (x * scale + sx, (s - 1 - z) * scale + (scale - 1 - sz));
                    let o = (py * w + px) * 3;
                    rgb[o..o + 3].copy_from_slice(&c);
                }
            }
        }
    }
    encode_png_rgb(w as u32, h as u32, &rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_terrain(size: i32) -> ClientTerrain {
        ClientTerrain { size, unit: 4.0, heights: vec![10.0; (size * size) as usize] }
    }

    #[test]
    fn nav_mask_round_trips_through_bytes() {
        let t = flat_terrain(9);
        // One wall face across x = 18 between z = 8 and z = 24, 3 m tall.
        let wall: CollisionFace = [[18.0, 10.0, 8.0], [18.0, 13.0, 24.0], [18.0, 10.0, 24.0]];
        let mask = NavMask::build(&t, &[wall]);
        let back = NavMask::from_bytes(&mask.to_bytes()).unwrap();
        assert_eq!(back.size, 9);
        assert_eq!(back.heights, mask.heights);
        for x in 0..9 {
            for z in 0..9 {
                assert_eq!(back.is_blocked(x, z), mask.is_blocked(x, z), "cell {x},{z}");
            }
        }
        // Edge band blocked, interior open, the wall's cells blocked.
        assert!(mask.is_blocked(0, 4) && mask.is_blocked(7, 4) && mask.is_blocked(4, 0));
        assert!(mask.is_walkable(2, 2));
        assert!(mask.is_blocked(4, 3) && mask.is_blocked(4, 4), "wall at x=18 blocks the cells either side");
        assert!(mask.is_walkable(2, 4) && mask.is_walkable(6, 4));
        assert!(mask.is_blocked(-1, 0) && mask.is_blocked(9, 0));
    }

    #[test]
    fn steep_terrain_is_blocked_by_the_client_slope_rule() {
        let mut t = flat_terrain(9);
        // A 4 m rise across one cell = 45 degrees, steeper than the 40 degree limit.
        for z in 0..9 {
            t.heights[(5 * 9 + z) as usize] = 14.0;
        }
        let mask = NavMask::build(&t, &[]);
        assert!(mask.is_blocked(4, 4), "45 degree quad must be blocked");
        assert!(mask.is_walkable(2, 4));
    }

    #[test]
    fn walked_map_marks_cells_and_edges_and_round_trips() {
        let w = WalkedMap::new(16, 4.0);
        assert!(w.mark_move((10.0, 10.0), (22.0, 10.0)));
        assert!(w.is_walked(2, 2) && w.is_walked(3, 2) && w.is_walked(4, 2) && w.is_walked(5, 2));
        assert!(w.is_edge_walked((2, 2), (3, 2)) && w.is_edge_walked((5, 2), (4, 2)));
        assert!(!w.is_edge_walked((2, 2), (2, 3)));
        assert!(!w.is_walked(6, 2));
        // Diagonal edge, both orientations.
        assert!(w.mark_move((30.0, 30.0), (34.5, 34.5)));
        assert!(w.is_edge_walked((7, 7), (8, 8)) && w.is_edge_walked((8, 8), (7, 7)));
        // Teleport-sized jumps are ignored.
        assert!(!w.mark_move((2.0, 2.0), (60.0, 2.0)));
        assert!(!w.is_walked(10, 0));
        assert!(w.take_dirty() && !w.take_dirty());
        let back = WalkedMap::from_bytes(&w.to_bytes(), 16, 4.0).unwrap();
        assert_eq!(back.walked_count(), w.walked_count());
        assert!(back.is_edge_walked((7, 7), (8, 8)));
        assert!(WalkedMap::from_bytes(&w.to_bytes(), 17, 4.0).is_err(), "stale grid size must be rejected");
    }

    #[test]
    fn gtd_and_opd_parsers_read_synthetic_files() {
        // .gtd: name, flag, 33x33 MAPDATA.
        let mut gtd = Vec::new();
        gtd.extend_from_slice(&4i32.to_le_bytes());
        gtd.extend_from_slice(b"test");
        gtd.extend_from_slice(&0i32.to_le_bytes());
        for x in 0..33 {
            for z in 0..33 {
                gtd.extend_from_slice(&0x7FE0_0001u32.to_le_bytes());
                gtd.extend_from_slice(&((x + z) as f32).to_le_bytes());
            }
        }
        gtd.extend_from_slice(&[0u8; 100]);
        let t = ClientTerrain::parse(&gtd).unwrap();
        assert_eq!(t.size, 33);
        assert_eq!(t.height_at(3, 5), Some(8.0));
        // .opd: name, version, width/length, one face, 8x8 empty cells.
        let mut opd = Vec::new();
        opd.extend_from_slice(&4i32.to_le_bytes());
        opd.extend_from_slice(b"test");
        opd.extend_from_slice(&0i32.to_le_bytes());
        opd.extend_from_slice(&128.0f32.to_le_bytes());
        opd.extend_from_slice(&128.0f32.to_le_bytes());
        opd.extend_from_slice(&1i32.to_le_bytes());
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
            opd.extend_from_slice(&v.to_le_bytes());
        }
        for _ in 0..64 {
            opd.extend_from_slice(&0u32.to_le_bytes());
        }
        opd.extend_from_slice(&0i32.to_le_bytes());
        let c = ClientCollision::parse(&opd).unwrap();
        assert_eq!(c.map_width, 128.0);
        assert_eq!(c.faces.len(), 1);
        assert_eq!(c.faces[0][2], [7.0, 8.0, 9.0]);
    }
}
