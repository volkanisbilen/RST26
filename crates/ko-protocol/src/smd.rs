//! SMD file parser — Knight Online map terrain data.
//! SMD files contain terrain heights, collision data, event tile grids,
//! spawn points, and warp gate definitions. The server uses the event grid
//! for walkability checks and warp gate triggering.
//! ## Binary Format (sequential reads)
//! 1. Terrain: `i32 map_size`, `f32 unit_dist`, `f32[map_size²] heights`
//! 2. Collision: `f32 width`, `f32 height`, faces, cell grid (variable)
//! 3. Object events: `i32 count`, `[24 bytes × count]`
//! 4. Map tiles: `i16[map_size²]` event IDs
//! 5. Regene events: `i32 count`, `[20 bytes × count]`
//! 6. Warp list: `i32 count`, `[320 bytes × count]`

use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Parsed SMD file — only the data the server needs.
#[derive(Debug)]
pub struct SmdFile {
    /// Grid unit count (e.g. 1025).
    pub map_size: i32,
    /// Distance per grid unit (e.g. 4.0).
    pub unit_dist: f32,
    /// Actual map width in meters (from collision data).
    pub map_width: f32,
    /// Actual map height in meters (from collision data).
    pub map_height: f32,
    /// Event tile grid — `map_size × map_size` entries.
    /// 0 = walkable, nonzero = blocked or event trigger.
    pub event_grid: Vec<i16>,
    /// Terrain heights and conservative bot navigation mask, in x-major order.
    pub heights: Vec<f32>,
    pub bot_blocked: Vec<bool>,
    /// Warp gates loaded from the file.
    pub warps: Vec<WarpInfo>,
    /// NPC/monster spawn points.
    pub regene_events: Vec<RegeneEvent>,
}

/// Warp gate definition.
#[derive(Debug, Clone)]
pub struct WarpInfo {
    pub warp_id: i16,
    pub name: String,
    /// Announce text shown in the warp list UI (char[256] in C++).
    pub announce: String,
    pub pay: u32,
    pub dest_zone: i16,
    pub dest_x: f32,
    pub dest_y: f32,
    pub dest_z: f32,
    pub radius: f32,
    /// Nation restriction: 0 = both, 1 = Karus, 2 = Elmorad.
    pub nation: i16,
}

/// NPC/monster spawn point.
#[derive(Debug, Clone)]
pub struct RegeneEvent {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub area_z: f32,
    pub area_x: f32,
    pub point_id: i32,
}

/// Constants matching the C++ collision system.
pub const CELL_MAIN_DEVIDE: i32 = 4;
pub const CELL_SUB_SIZE: i32 = 4;
pub const CELL_MAIN_SIZE: i32 = CELL_MAIN_DEVIDE * CELL_SUB_SIZE; // 16

/// Size of the packed `_WARP_INFO` struct in bytes.
/// Layout: i16(2) + char[32](32) + char[256](256) + u16(2) + u32(4) +
///   i16(2) + u16(2) + f32×4(16) + i16(2) + u16(2) = 320
const WARP_INFO_SIZE: usize = 320;

/// Size of the packed `_OBJECT_EVENT` struct read from the file.
const OBJECT_EVENT_SIZE_LEGACY: usize = 24;
/// Packed object-event size used by the Manes map exporter.
const OBJECT_EVENT_SIZE_HEADERED: usize = 19;

// ─── Client movement rules ──────────────────────────────────────────────
//
// Everything below was read out of the 2625 `KnightOnline.exe` (Hex-Rays
// decompile, addresses are VAs). A real player cannot be placed anywhere
// the client's own move validation would refuse to step, so the bot
// navigation mask must be a subset of what that validation allows.
//
// * `sub_910720` (CPlayerMySelf move predictor, behind every key/mouse
//   step): for the next position it calls, on the world object
//   (`dword_1114570 + 36`, a CN3TerrainManager):
//     - vtbl+76  `sub_8C9300`  map-edge test: 4.0 <= x,z <= 4*size - 8.
//     - vtbl+112 `sub_8C8F20` -> `sub_6B20C0`  ray vs collision faces.
//     - vtbl+108 `sub_8C8EA0` -> `sub_6B0510`  swept SPHERE vs collision
//       faces, radius clamped to 1.124 (inputs outside 1.1..1.2 become
//       1.124), centre lifted `radius + 0.1` above a probe point that is
//       itself 0.5 above the feet => the sphere covers feet+0.6 ..
//       feet+2.848.  A face only counts when its normal is "wall like":
//       `hypot(nx, nz) > 0.643` (normalised normal) and it faces against
//       the move direction.
//     - vtbl+80  `sub_8C9210` CheckIncline(pos, dir, 0.643): terrain normal
//       from `sub_8C13A0`; blocked when `hypot(nx, nz) > 0.643` and
//       `dot(n.xz, dir.xz) <= 0` (moving uphill / along the contour).
//   0.643 is sin(40 deg): the real climb limit is a 40 degree slope
//   (gradient 0.8396), NOT 45 degrees (gradient 1.0).
// * Terrain triangles (`sub_8C7B40` height, `sub_8C13A0` normal) are split
//   along the (0,0)-(1,1) diagonal when (ix + iz) is even and along the
//   (1,0)-(0,1) diagonal when it is odd, i.e. a checkerboard.
//
/// sin of the steepest slope a player can walk up (client constant 0.643).
pub const CLIENT_SLOPE_SIN_LIMIT: f32 = 0.643;
/// Radius of the player's collision sphere (client clamps it to 1.124).
pub const CLIENT_BODY_RADIUS: f32 = 1.124;
/// Collision sphere bottom relative to the feet (0.5 probe lift + 0.1).
pub const CLIENT_BODY_LOW: f32 = 0.6;
/// Collision sphere top relative to the feet (bottom + 2 * radius).
pub const CLIENT_BODY_HIGH: f32 = 0.6 + 2.0 * CLIENT_BODY_RADIUS;
/// Players cannot stand closer than this to the map edge (`sub_8C9300`).
pub const CLIENT_MAP_EDGE_MARGIN: f32 = 4.0;
/// Horizontal clearance bots keep from every wall-like collision face. Equal
/// to the client's collision sphere so a bot never stands where the player
/// model would be pushed out of the wall.
pub const BOT_WALL_CLEARANCE: f32 = CLIENT_BODY_RADIUS;

/// Gradient `(dh/dx, dh/dz)` per world unit of the terrain triangle that
/// holds the fractional quad position `(fx, fz)`, using the client's
/// checkerboard triangulation. `h = [h(0,0), h(0,1), h(1,0), h(1,1)]` with
/// the first index along x.
fn client_quad_gradient(h: [f32; 4], parity_even: bool, fx: f32, fz: f32, unit: f32) -> (f32, f32) {
    let [h00, h01, h10, h11] = h;
    let (gx, gz) = if parity_even {
        // Diagonal (0,0)-(1,1).
        if fz > fx {
            (h11 - h01, h01 - h00)
        } else {
            (h10 - h00, h11 - h10)
        }
    } else if fx + fz <= 1.0 {
        // Diagonal (1,0)-(0,1), lower triangle (0,0),(1,0),(0,1).
        (h10 - h00, h01 - h00)
    } else {
        (h11 - h01, h11 - h10)
    };
    (gx / unit, gz / unit)
}

/// The four heights of terrain quad `(ix, iz)`, `None` if outside the grid.
fn quad_heights(heights: &[f32], size: i32, ix: i32, iz: i32) -> Option<[f32; 4]> {
    if ix < 0 || iz < 0 || ix + 1 >= size || iz + 1 >= size {
        return None;
    }
    let i = (ix * size + iz) as usize;
    let s = size as usize;
    if i + s + 1 >= heights.len() {
        return None;
    }
    Some([heights[i], heights[i + 1], heights[i + s], heights[i + s + 1]])
}

/// Terrain height exactly as the client computes it (`sub_8C7B40`).
pub fn client_terrain_height(heights: &[f32], size: i32, unit: f32, x: f32, z: f32) -> Option<f32> {
    if !(x.is_finite() && z.is_finite()) || x < 0.0 || z < 0.0 {
        return None;
    }
    let (ix, iz) = ((x / unit) as i32, (z / unit) as i32);
    let [h00, h01, h10, h11] = quad_heights(heights, size, ix, iz)?;
    let (fx, fz) = (x / unit - ix as f32, z / unit - iz as f32);
    let value = if (ix + iz) % 2 == 0 {
        if fz > fx {
            h00 + (h01 - h00) * fz + (h11 - h01) * fx
        } else {
            h00 + (h10 - h00) * fx + (h11 - h10) * fz
        }
    } else if fx + fz <= 1.0 {
        h00 + (h10 - h00) * fx + (h01 - h00) * fz
    } else {
        h11 + (h10 - h11) * (1.0 - fz) + (h01 - h11) * (1.0 - fx)
    };
    Some(value)
}

/// `sin(slope)` of the terrain triangle under `(x, z)` (client triangulation).
pub fn client_terrain_slope_sin(heights: &[f32], size: i32, unit: f32, x: f32, z: f32) -> Option<f32> {
    if !(x.is_finite() && z.is_finite()) || x < 0.0 || z < 0.0 {
        return None;
    }
    let (ix, iz) = ((x / unit) as i32, (z / unit) as i32);
    let h = quad_heights(heights, size, ix, iz)?;
    let (fx, fz) = (x / unit - ix as f32, z / unit - iz as f32);
    let (gx, gz) = client_quad_gradient(h, (ix + iz) % 2 == 0, fx, fz, unit);
    let g = gx.hypot(gz);
    Some(g / (1.0 + g * g).sqrt())
}

/// Would the client refuse a step at `(x, z)` heading `(dir_x, dir_z)`
/// because the terrain is too steep (`sub_8C9210`)?  Moving downhill is
/// allowed by the client; moving uphill or along the contour is not.
pub fn client_incline_blocks(heights: &[f32], size: i32, unit: f32, x: f32, z: f32, dir_x: f32, dir_z: f32) -> bool {
    if !(x.is_finite() && z.is_finite()) || x < 0.0 || z < 0.0 {
        return false;
    }
    let (ix, iz) = ((x / unit) as i32, (z / unit) as i32);
    let Some(h) = quad_heights(heights, size, ix, iz) else { return false };
    let (fx, fz) = (x / unit - ix as f32, z / unit - iz as f32);
    let (gx, gz) = client_quad_gradient(h, (ix + iz) % 2 == 0, fx, fz, unit);
    let g = gx.hypot(gz);
    // Terrain normal is (-gx, 1, -gz): its horizontal part points downhill.
    g / (1.0 + g * g).sqrt() > CLIENT_SLOPE_SIN_LIMIT && (-gx * dir_x - gz * dir_z) <= 0.0
}

/// True when terrain quad `(ix, iz)` holds a triangle steeper than the
/// client's climb limit, a non-finite height, or lies in the unwalkable map
/// edge band.
pub fn terrain_quad_blocked(heights: &[f32], size: i32, unit: f32, ix: i32, iz: i32) -> bool {
    // Positions outside [4, 4*size-8] are refused by `sub_8C9300`; a cell is
    // usable only if it lies wholly inside.
    if ix < 1 || iz < 1 || ix > size - 3 || iz > size - 3 {
        return true;
    }
    let Some(h) = quad_heights(heights, size, ix, iz) else { return true };
    if h.iter().any(|v| !v.is_finite()) {
        return true;
    }
    // Both triangles of the client's split for this quad. A hair under the
    // client limit so float noise never lets a borderline triangle through.
    let limit = CLIENT_SLOPE_SIN_LIMIT - 0.002;
    let [h00, h01, h10, h11] = h;
    let triangles: [(f32, f32); 2] = if (ix + iz) % 2 == 0 {
        [(h11 - h01, h01 - h00), (h10 - h00, h11 - h10)]
    } else {
        [(h10 - h00, h01 - h00), (h11 - h01, h11 - h10)]
    };
    triangles.iter().any(|&(gx, gz)| {
        let g = (gx / unit).hypot(gz / unit);
        g / (1.0 + g * g).sqrt() > limit
    })
}

/// A collision triangle from the SMD's CN3ShapeMgr section.
pub type CollisionFace = [[f32; 3]; 3];

/// Reject garbage faces. `freezone_a.smd` (and its aliases) carries ~1.6k
/// faces of uninitialised memory; coordinates like 1e29 or -44610 are not
/// map geometry. The invisible boundary walls the map editors emit run from
/// y = -5000 to y = 5000, so that is the vertical bound.
pub fn collision_face_is_sane(v: &CollisionFace, extent: f32) -> bool {
    v.iter().flatten().all(|c| c.is_finite())
        && v.iter().all(|p| {
            p[0] >= -1000.0
                && p[0] <= extent + 1000.0
                && p[2] >= -1000.0
                && p[2] <= extent + 1000.0
                && p[1].abs() <= 5001.0
        })
}

/// Mark every terrain cell a player-blocking collision face would stop a
/// walker in. A face is a wall for the client when its normal's horizontal
/// part is > 0.643 (see the notes above); shallower faces are floors/ramps
/// the client walks over. A cell is blocked when such a face comes within
/// `BOT_WALL_CLEARANCE` of the cell box and overlaps the body's vertical
/// extent above the cell's ground.
pub fn rasterize_wall_faces(faces: &[CollisionFace], size: i32, unit: f32, heights: &[f32], blocked: &mut [bool]) {
    let margin = BOT_WALL_CLEARANCE;
    for v in faces {
        let a = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
        let b = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
        let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if !(len > 1e-9) || n[0].hypot(n[2]) / len <= CLIENT_SLOPE_SIN_LIMIT {
            continue;
        }
        let min = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
        let max = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
        let x0 = (((min(0) - margin) / unit).floor() as i32).max(0);
        let x1 = (((max(0) + margin) / unit).floor() as i32).min(size - 2);
        let z0 = (((min(2) - margin) / unit).floor() as i32).max(0);
        let z1 = (((max(2) + margin) / unit).floor() as i32).min(size - 2);
        let (face_low, face_high) = (min(1), max(1));
        for x in x0..=x1 {
            for z in z0..=z1 {
                // Separating-axis test of the face's XZ footprint against the
                // cell box grown by the body radius.
                let center = [(x as f32 + 0.5) * unit, (z as f32 + 0.5) * unit];
                let half = unit * 0.5 + margin;
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
                let h = [heights[i], heights[i + 1], heights[i + size as usize], heights[i + size as usize + 1]];
                let low = h.iter().copied().fold(f32::INFINITY, f32::min);
                let high = h.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                // 0.5 rather than the client's 0.6: stay a hair conservative.
                if face_high > low + (CLIENT_BODY_LOW - 0.1) && face_low < high + CLIENT_BODY_HIGH {
                    blocked[i] = true;
                }
            }
        }
    }
}

/// Spatial index over the wall-like collision faces, used to measure how
/// close a player-sized sphere at a given position is to a wall exactly the
/// way the client's swept-sphere test does. This is the *independent model*
/// the navigation mask and the bot movement tests are verified against; the
/// runtime mask itself never consults it.
pub struct WallIndex {
    bin: f32,
    bins_x: i32,
    bins: Vec<Vec<u32>>,
    faces: Vec<CollisionFace>,
}

impl WallIndex {
    /// Index every face the client would treat as a wall (`hypot(nx,nz) >
    /// 0.643`). `extent` is the map width in world units.
    pub fn new(faces: &[CollisionFace], extent: f32) -> Self {
        let bin = 16.0;
        let bins_x = (extent / bin).ceil() as i32 + 1;
        let mut index = WallIndex {
            bin,
            bins_x,
            bins: vec![Vec::new(); (bins_x * bins_x) as usize],
            faces: Vec::new(),
        };
        for v in faces {
            let a = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
            let b = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
            let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if !(len > 1e-9) || n[0].hypot(n[2]) / len <= CLIENT_SLOPE_SIN_LIMIT {
                continue;
            }
            let id = index.faces.len() as u32;
            index.faces.push(*v);
            let lo = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
            let hi = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
            let bx0 = (((lo(0) - 2.0) / bin).floor() as i32).max(0);
            let bx1 = (((hi(0) + 2.0) / bin).floor() as i32).min(bins_x - 1);
            let bz0 = (((lo(2) - 2.0) / bin).floor() as i32).max(0);
            let bz1 = (((hi(2) + 2.0) / bin).floor() as i32).min(bins_x - 1);
            for bx in bx0..=bx1 {
                for bz in bz0..=bz1 {
                    index.bins[(bx * bins_x + bz) as usize].push(id);
                }
            }
        }
        index
    }

    /// Distance from the centre of the client's collision sphere, for a
    /// player whose feet are at `(x, feet_y, z)`, to the nearest wall face.
    /// `f32::INFINITY` when no wall lies within the 16 m bin. A value below
    /// [`CLIENT_BODY_RADIUS`] means the client would not let a player stand
    /// there (walking into it is refused).
    pub fn clearance(&self, x: f32, feet_y: f32, z: f32) -> f32 {
        let bx = (x / self.bin).floor() as i32;
        let bz = (z / self.bin).floor() as i32;
        if bx < 0 || bz < 0 || bx >= self.bins_x || bz >= self.bins_x {
            return f32::INFINITY;
        }
        let centre = [x, feet_y + CLIENT_BODY_LOW + CLIENT_BODY_RADIUS, z];
        let mut best = f32::INFINITY;
        for &id in &self.bins[(bx * self.bins_x + bz) as usize] {
            best = best.min(point_triangle_distance(centre, &self.faces[id as usize]));
        }
        best
    }
}

/// Distance from `p` to triangle `t` (Ericson, Real-Time Collision Detection 5.1.5).
fn point_triangle_distance(p: [f32; 3], t: &CollisionFace) -> f32 {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    let closest = if d1 <= 0.0 && d2 <= 0.0 {
        a
    } else {
        let bp = sub(p, b);
        let (d3, d4) = (dot(ab, bp), dot(ac, bp));
        if d3 >= 0.0 && d4 <= d3 {
            b
        } else {
            let vc = d1 * d4 - d3 * d2;
            if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
                let v = d1 / (d1 - d3);
                [a[0] + ab[0] * v, a[1] + ab[1] * v, a[2] + ab[2] * v]
            } else {
                let cp = sub(p, c);
                let (d5, d6) = (dot(ab, cp), dot(ac, cp));
                if d6 >= 0.0 && d5 <= d6 {
                    c
                } else {
                    let vb = d5 * d2 - d1 * d6;
                    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
                        let w = d2 / (d2 - d6);
                        [a[0] + ac[0] * w, a[1] + ac[1] * w, a[2] + ac[2] * w]
                    } else {
                        let va = d3 * d6 - d5 * d4;
                        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
                            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
                            [b[0] + (c[0] - b[0]) * w, b[1] + (c[1] - b[1]) * w, b[2] + (c[2] - b[2]) * w]
                        } else {
                            let denom = 1.0 / (va + vb + vc);
                            let (v, w) = (vb * denom, vc * denom);
                            [
                                a[0] + ab[0] * v + ac[0] * w,
                                a[1] + ab[1] * v + ac[1] * w,
                                a[2] + ab[2] * v + ac[2] * w,
                            ]
                        }
                    }
                }
            }
        }
    };
    let d = sub(p, closest);
    dot(d, d).sqrt()
}

impl SmdFile {
    /// Load an SMD file from disk.
    ///
    /// Parses all sections: terrain (skip heights), collision (extract dimensions, skip data),
    /// object events (skip), event tile grid (keep), regene events (keep), warps (keep).
    pub fn load(path: &Path) -> io::Result<Self> {
        let data = std::fs::read(path)?;
        let mut cursor = io::Cursor::new(&data);
        Self::parse(&mut cursor)
    }

    /// Load an SMD file and also return the sane collision faces from its
    /// CN3ShapeMgr section (used by tests and diagnostics that need the raw
    /// geometry the bot navigation mask was derived from).
    pub fn load_with_faces(path: &Path) -> io::Result<(Self, Vec<CollisionFace>)> {
        let data = std::fs::read(path)?;
        let mut cursor = io::Cursor::new(&data);
        Self::parse_with_faces(&mut cursor)
    }

    /// Parse SMD from a reader.
    pub fn parse<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::parse_with_faces(reader).map(|(smd, _)| smd)
    }

    /// Parse SMD from a reader, also returning the sane collision faces.
    pub fn parse_with_faces<R: Read + Seek>(reader: &mut R) -> io::Result<(Self, Vec<CollisionFace>)> {
        // 1. LoadTerrain. Most server SMDs start directly with map_size/unit_dist.
        // The 2615 Manes exporter prepends two length-prefixed strings plus a
        // 10-byte exporter header. Detect that format without changing legacy maps.
        let start = reader.stream_position()?;
        let first = read_i32(reader)?;
        let candidate_unit_dist = read_f32(reader)?;
        let headered = !(first > 0
            && first <= 10000
            && candidate_unit_dist.is_finite()
            && candidate_unit_dist > 0.0
            && candidate_unit_dist <= 100.0);
        if headered {
            if first <= 0 || first > 255 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid SMD header",
                ));
            }
            reader.seek(SeekFrom::Start(start + 4 + first as u64))?;
            let author_len = read_i32(reader)?;
            if author_len < 0 || author_len > 255 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid SMD author header",
                ));
            }
            reader.seek(SeekFrom::Current(author_len as i64 + 10))?;
        } else {
            reader.seek(SeekFrom::Start(start))?;
        }

        let map_size = read_i32(reader)?;
        let unit_dist = read_f32(reader)?;

        if map_size <= 0 || map_size > 10000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid map_size: {}", map_size),
            ));
        }

        if !unit_dist.is_finite() || unit_dist <= 0.0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid terrain unit"));
        }
        let heights = (0..map_size as usize * map_size as usize)
            .map(|_| read_f32(reader)).collect::<io::Result<Vec<_>>>()?;
        // Conservative bot navigation mask: terrain too steep for the client
        // (see "Client movement rules" above), the unwalkable map edge, and
        // (below) cells a wall-like collision face reaches into.
        let mut bot_blocked = vec![true; heights.len()];
        for x in 0..map_size - 1 {
            for z in 0..map_size - 1 {
                bot_blocked[(x * map_size + z) as usize] = terrain_quad_blocked(&heights, map_size, unit_dist, x, z);
            }
        }

        let pos_after_terrain = reader.stream_position()?;
        tracing::debug!(
            map_size,
            unit_dist,
            pos_after_terrain,
            "terrain section parsed"
        );

        // 2. LoadCollisionData (CN3ShapeMgr)
        let (map_width, map_height, faces) = Self::read_collision_data(reader, map_size, unit_dist)?;
        rasterize_wall_faces(&faces, map_size, unit_dist, &heights, &mut bot_blocked);
        let pos_after_collision = reader.stream_position()?;
        tracing::debug!(
            map_width,
            map_height,
            pos_after_collision,
            "collision section parsed"
        );

        // Validate: (map_size - 1) * unit_dist should equal map_width
        let expected_width = (map_size - 1) as f32 * unit_dist;
        if (expected_width - map_width).abs() > 1.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "map dimension mismatch: expected {}, got width={}",
                    expected_width, map_width
                ),
            ));
        }

        // 3. LoadObjectEvent (skip — data loaded from DB)
        let object_event_count = read_i32(reader)?;
        if object_event_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative object event count",
            ));
        }
        let object_event_size = if headered {
            OBJECT_EVENT_SIZE_HEADERED
        } else {
            OBJECT_EVENT_SIZE_LEGACY
        };
        let skip = object_event_count as i64 * object_event_size as i64;
        reader.seek(SeekFrom::Current(skip))?;

        let pos_after_objects = reader.stream_position()?;
        tracing::debug!(
            object_event_count,
            pos_after_objects,
            "object events skipped"
        );

        // 4. LoadMapTile — event grid (keep!)
        let grid_len = (map_size as usize) * (map_size as usize);
        let mut event_grid = vec![0i16; grid_len];
        for val in event_grid.iter_mut() {
            *val = read_i16(reader)?;
        }

        let pos_after_tiles = reader.stream_position()?;
        let nonzero = event_grid.iter().filter(|&&v| v != 0).count();
        tracing::debug!(grid_len, nonzero, pos_after_tiles, "tile event grid read");

        // Some headered map exports end immediately after the event grid.
        // Their NPC placement is intentionally supplied by npc_spawn instead.
        let regene_count = match read_i32(reader) {
            Ok(count) => count,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => 0,
            Err(e) => return Err(e),
        };
        if regene_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative regene event count",
            ));
        }
        let mut regene_events = Vec::with_capacity(regene_count.max(0) as usize);
        for i in 0..regene_count {
            let pos_x = read_f32(reader)?;
            let pos_y = read_f32(reader)?;
            let pos_z = read_f32(reader)?;
            let area_z = read_f32(reader)?;
            let area_x = read_f32(reader)?;
            regene_events.push(RegeneEvent {
                pos_x,
                pos_y,
                pos_z,
                area_z,
                area_x,
                point_id: i,
            });
        }

        // 6. LoadWarpList
        let warps = Self::read_warp_list(reader)?;

        Ok((
            SmdFile {
                map_size,
                unit_dist,
                map_width,
                map_height,
                event_grid,
                heights,
                bot_blocked,
                warps,
                regene_events,
            },
            faces,
        ))
    }

    /// Check if a world position is within map boundaries.
    ///
    pub fn is_valid_position(&self, x: f32, z: f32) -> bool {
        x >= 0.0 && z >= 0.0 && x < self.map_width && z < self.map_height
    }

    /// Get the event ID for a tile grid position.
    ///
    /// Note: x and z are grid indices (world pos / unit_dist), NOT world coordinates.
    pub fn get_event_id(&self, x: i32, z: i32) -> i16 {
        if x < 0 || x >= self.map_size || z < 0 || z >= self.map_size {
            return -1;
        }
        self.event_grid[(x as usize) * (self.map_size as usize) + (z as usize)]
    }

    /// Check if a tile is walkable (event ID == 0).
    ///
    /// Note: x and z are grid indices, NOT world coordinates.
    pub fn is_movable(&self, x: i32, z: i32) -> bool {
        self.get_event_id(x, z) == 0
    }

    /// Get the event ID for a world-space position, converting to grid indices.
    ///
    pub fn get_event_id_at(&self, world_x: f32, world_z: f32) -> i16 {
        let gx = (world_x / self.unit_dist) as i32;
        let gz = (world_z / self.unit_dist) as i32;
        self.get_event_id(gx, gz)
    }

    /// Check if a world position is on a walkable tile.
    pub fn is_movable_at(&self, world_x: f32, world_z: f32) -> bool {
        self.get_event_id_at(world_x, world_z) == 0
    }

    /// Read collision data (CN3ShapeMgr::LoadCollisionData): map dimensions
    /// plus the sane collision faces. The per-cell polygon index grid that
    /// follows the faces is skipped.
    fn read_collision_data<R: Read + Seek>(reader: &mut R, size: i32, unit: f32) -> io::Result<(f32, f32, Vec<CollisionFace>)> {
        read_shape_mgr_collision(reader, (size - 1) as f32 * unit)
    }
}

/// Read a `CN3ShapeMgr` collision block (`LoadCollisionData`): map width and
/// length, the collision faces (garbage faces dropped, see
/// [`collision_face_is_sane`]) and the per-cell polygon index grid, which is
/// skipped. The same block sits in the server `.smd` (after the terrain) and
/// in the client `.opd` (after the file name and version), so both parsers
/// share it. `extent` is the expected map width used for the sanity bounds.
pub fn read_shape_mgr_collision<R: Read + Seek>(reader: &mut R, extent: f32) -> io::Result<(f32, f32, Vec<CollisionFace>)> {
    {
        let map_width = read_f32(reader)?;
        let map_length = read_f32(reader)?;

        // Collision face count + vertex data
        let face_count = read_i32(reader)?;
        if face_count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "negative collision face count",
            ));
        }
        let mut faces = Vec::new();
        // Each face = 3 vertices, each vertex = 3 floats (12 bytes)
        for _ in 0..face_count {
            let mut v = [[0.0f32; 3]; 3];
            for vertex in &mut v {
                for coordinate in vertex { *coordinate = read_f32(reader)?; }
            }
            // Some exported SMDs (the "freezone_a" family) contain ~1.6k
            // garbage faces. The original client tolerates this silently;
            // aborting the load took down the entire zone. Skip just those
            // faces; see `collision_face_is_sane`.
            if collision_face_is_sane(&v, extent) {
                faces.push(v);
            }
        }

        // Cell grid: iterate over map_width/16 × map_length/16 cells
        let cells_x = (map_width / CELL_MAIN_SIZE as f32).ceil() as i32;
        let cells_z = (map_length / CELL_MAIN_SIZE as f32).ceil() as i32;

        for _z in 0..cells_z {
            for _x in 0..cells_x {
                let exists = read_u32(reader)?;
                if exists == 0 {
                    continue;
                }
                // CellMain: shape_count + shape_indices + 4×4 sub-cells
                let shape_count = read_i32(reader)?;
                if shape_count > 0 {
                    // u16[shape_count]
                    reader.seek(SeekFrom::Current(shape_count as i64 * 2))?;
                }

                // 4×4 sub-cells
                for _sz in 0..CELL_MAIN_DEVIDE {
                    for _sx in 0..CELL_MAIN_DEVIDE {
                        let poly_count = read_i32(reader)?;
                        if poly_count > 0 {
                            // u32[poly_count * 3]
                            reader.seek(SeekFrom::Current(poly_count as i64 * 3 * 4))?;
                        }
                    }
                }
            }
        }

        Ok((map_width, map_length, faces))
    }
}

impl SmdFile {
    /// Read the warp list section.
    ///
    ///
    /// `_WARP_INFO` layout (320 bytes, `#pragma pack(push, 1)`):
    /// ```text
    /// Offset  Size  Field
    ///   0       2   sWarpID (i16)
    ///   2      32   strWarpName (char[32])
    ///  34     256   strAnnounce (char[256])
    /// 290       2   sUnk0 (u16)
    /// 292       4   dwPay (u32)
    /// 296       2   sZone (i16)
    /// 298       2   sUnk1 (u16)
    /// 300       4   fX (f32)
    /// 304       4   fY (f32)
    /// 308       4   fZ (f32)
    /// 312       4   fR (f32) — trigger radius
    /// 316       2   sNation (i16) — -1=both, 1=Karus, 2=Elmorad
    /// 318       2   sUnk2 (u16)
    /// ───────────
    /// 320 total
    /// ```
    fn read_warp_list<R: Read + Seek>(reader: &mut R) -> io::Result<Vec<WarpInfo>> {
        let count = match read_i32(reader) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut warps = Vec::new();
        for _ in 0..count {
            let mut buf = [0u8; WARP_INFO_SIZE];
            match reader.read_exact(&mut buf) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(e),
            }

            let warp_id = i16::from_le_bytes([buf[0], buf[1]]);
            if warp_id == 0 {
                continue; // C++: skip warp ID 0
            }

            let name = read_null_terminated(&buf[2..34]);
            let announce = read_null_terminated(&buf[34..290]);
            // bytes 290..292: sUnk0 (skip)
            let pay = u32::from_le_bytes([buf[292], buf[293], buf[294], buf[295]]);
            let dest_zone = i16::from_le_bytes([buf[296], buf[297]]);
            // bytes 298..300: sUnk1 (skip)
            let dest_x = f32::from_le_bytes([buf[300], buf[301], buf[302], buf[303]]);
            let dest_y = f32::from_le_bytes([buf[304], buf[305], buf[306], buf[307]]);
            let dest_z = f32::from_le_bytes([buf[308], buf[309], buf[310], buf[311]]);
            let radius = f32::from_le_bytes([buf[312], buf[313], buf[314], buf[315]]);
            let nation = i16::from_le_bytes([buf[316], buf[317]]);
            // bytes 318..320: sUnk2 (skip)

            warps.push(WarpInfo {
                warp_id,
                name,
                announce,
                pay,
                dest_zone,
                dest_x,
                dest_y,
                dest_z,
                radius,
                nation,
            });
        }

        Ok(warps)
    }
}

/// Read a null-terminated string from a fixed-size buffer.
fn read_null_terminated(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).to_string()
}

// Little-endian binary read helpers
pub(crate) fn read_i16<R: Read>(r: &mut R) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

pub(crate) fn read_i32<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

pub(crate) fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub(crate) fn read_f32<R: Read>(r: &mut R) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_position() {
        let smd = SmdFile {
            heights: Vec::new(),
            bot_blocked: Vec::new(),
            map_size: 1025,
            unit_dist: 4.0,
            map_width: 4096.0,
            map_height: 4096.0,
            event_grid: vec![0i16; 1025 * 1025],
            warps: vec![],
            regene_events: vec![],
        };

        assert!(smd.is_valid_position(100.0, 100.0));
        assert!(smd.is_valid_position(0.0, 0.0));
        assert!(smd.is_valid_position(4095.9, 4095.9));
        assert!(!smd.is_valid_position(4096.0, 100.0));
        assert!(!smd.is_valid_position(100.0, 4096.0));
        assert!(!smd.is_valid_position(-1.0, 100.0));
    }

    #[test]
    fn test_get_event_id() {
        let map_size = 10;
        let mut grid = vec![0i16; (map_size * map_size) as usize];
        // Set tile (3, 5) to event 42
        grid[3 * map_size as usize + 5] = 42;
        // Set tile (0, 0) to blocked (1)
        grid[0] = 1;

        let smd = SmdFile {
            heights: Vec::new(),
            bot_blocked: Vec::new(),
            map_size,
            unit_dist: 4.0,
            map_width: 36.0,
            map_height: 36.0,
            event_grid: grid,
            warps: vec![],
            regene_events: vec![],
        };

        assert_eq!(smd.get_event_id(3, 5), 42);
        assert_eq!(smd.get_event_id(0, 0), 1);
        assert_eq!(smd.get_event_id(1, 1), 0);
        assert_eq!(smd.get_event_id(-1, 0), -1); // out of bounds
        assert_eq!(smd.get_event_id(10, 0), -1); // out of bounds
    }

    #[test]
    fn test_is_movable() {
        let map_size = 5;
        let mut grid = vec![0i16; 25];
        grid[2 * 5 + 3] = 1; // blocked tile at (2, 3)

        let smd = SmdFile {
            heights: Vec::new(),
            bot_blocked: Vec::new(),
            map_size,
            unit_dist: 4.0,
            map_width: 16.0,
            map_height: 16.0,
            event_grid: grid,
            warps: vec![],
            regene_events: vec![],
        };

        assert!(smd.is_movable(0, 0)); // walkable
        assert!(!smd.is_movable(2, 3)); // blocked
        assert!(smd.is_movable(2, 4)); // walkable neighbor
    }

    #[test]
    fn test_get_event_id_at_world_coords() {
        let map_size = 10;
        let mut grid = vec![0i16; 100];
        grid[2 * 10 + 3] = 99; // grid (2, 3)

        let smd = SmdFile {
            heights: Vec::new(),
            bot_blocked: Vec::new(),
            map_size,
            unit_dist: 4.0,
            map_width: 36.0,
            map_height: 36.0,
            event_grid: grid,
            warps: vec![],
            regene_events: vec![],
        };

        // World (8.0, 12.0) → grid (8/4=2, 12/4=3) → event 99
        assert_eq!(smd.get_event_id_at(8.0, 12.0), 99);
        // World (0.0, 0.0) → grid (0, 0) → event 0
        assert_eq!(smd.get_event_id_at(0.0, 0.0), 0);
    }

    #[test]
    fn test_read_null_terminated() {
        assert_eq!(read_null_terminated(b"hello\0world"), "hello");
        assert_eq!(read_null_terminated(b"\0"), "");
        assert_eq!(read_null_terminated(b"noterm"), "noterm");
    }

    #[test]
    fn test_warp_info_parsing() {
        // Build a synthetic 320-byte _WARP_INFO
        let mut buf = vec![0u8; 320];
        // warp_id = 5
        buf[0..2].copy_from_slice(&5i16.to_le_bytes());
        // name at offset 2
        buf[2..8].copy_from_slice(b"Gate01");
        // announce at offset 34
        buf[34..44].copy_from_slice(b"Welcome!\0\0");
        // pay at offset 292
        buf[292..296].copy_from_slice(&1000u32.to_le_bytes());
        // dest_zone at offset 296
        buf[296..298].copy_from_slice(&21i16.to_le_bytes());
        // dest_x at offset 300
        buf[300..304].copy_from_slice(&100.0f32.to_le_bytes());
        // dest_y at offset 304
        buf[304..308].copy_from_slice(&0.0f32.to_le_bytes());
        // dest_z at offset 308
        buf[308..312].copy_from_slice(&200.0f32.to_le_bytes());
        // radius at offset 312
        buf[312..316].copy_from_slice(&10.0f32.to_le_bytes());
        // nation at offset 316
        buf[316..318].copy_from_slice(&(-1i16).to_le_bytes());

        // Build a minimal SMD stream with just the warp section
        // We need: count(i32) + warp_data
        let mut data = Vec::new();
        data.extend_from_slice(&1i32.to_le_bytes()); // 1 warp
        data.extend_from_slice(&buf);

        let mut cursor = io::Cursor::new(&data);
        let warps = SmdFile::read_warp_list(&mut cursor).unwrap();
        assert_eq!(warps.len(), 1);
        assert_eq!(warps[0].warp_id, 5);
        assert_eq!(warps[0].name, "Gate01");
        assert_eq!(warps[0].announce, "Welcome!");
        assert_eq!(warps[0].pay, 1000);
        assert_eq!(warps[0].dest_zone, 21);
        assert_eq!(warps[0].dest_x, 100.0);
        assert_eq!(warps[0].dest_z, 200.0);
        assert_eq!(warps[0].radius, 10.0);
        assert_eq!(warps[0].nation, -1);
    }

    /// Regression test for the Ardream (`freezone_a.smd`) bot-navigation bug:
    /// a collision face with vertices decoding to huge-but-finite floats
    /// (~1e29, nowhere near any real map) rasterized as "blocked" across the
    /// whole grid because the old degenerate-face filter only checked
    /// `is_finite()`. Build a minimal synthetic SMD with one such face over
    /// otherwise flat, unblocked terrain and confirm it no longer blocks the
    /// whole grid once its coordinates exceed the sanity bound.
    #[test]
    fn test_garbage_coordinate_collision_face_does_not_block_whole_grid() {
        let size: i32 = 5;
        let unit: f32 = 4.0;
        let map_width = (size - 1) as f32 * unit;

        let mut data = Vec::new();
        data.extend_from_slice(&size.to_le_bytes());
        data.extend_from_slice(&unit.to_le_bytes());
        for _ in 0..(size * size) {
            data.extend_from_slice(&0.0f32.to_le_bytes()); // flat terrain
        }
        data.extend_from_slice(&map_width.to_le_bytes());
        data.extend_from_slice(&map_width.to_le_bytes());

        // One garbage face: vertices span from deeply negative to deeply
        // positive on both X and Z, which is exactly what makes the old
        // bounding-box rasterization loop cover the entire grid (the min
        // bound clamps up to 0, the max bound clamps down to size-2).
        data.extend_from_slice(&1i32.to_le_bytes());
        let verts: [[f32; 3]; 3] = [
            [-200_000.0, 0.0, -200_000.0],
            [200_000.0, 100.0, 0.0],
            [0.0, 50.0, 200_000.0],
        ];
        for vertex in &verts {
            for coordinate in vertex {
                data.extend_from_slice(&coordinate.to_le_bytes());
            }
        }

        // Cell grid: (map_width/16).ceil() per axis; all cells empty.
        let cells = ((map_width / 16.0).ceil() as u32).max(1);
        for _ in 0..(cells * cells) {
            data.extend_from_slice(&0u32.to_le_bytes()); // exists = 0
        }

        data.extend_from_slice(&0i32.to_le_bytes()); // object_event_count = 0
        for _ in 0..(size * size) {
            data.extend_from_slice(&1i16.to_le_bytes()); // event grid, all "walkable"
        }
        // No regene events / warps — EOF is treated as zero of each.

        let mut cursor = io::Cursor::new(&data);
        let smd = SmdFile::parse(&mut cursor).expect("parse synthetic SMD");

        let blocked = smd.bot_blocked.iter().filter(|&&b| b).count();
        assert!(
            blocked < smd.bot_blocked.len(),
            "a single garbage-coordinate collision face should not block the entire grid"
        );
    }

    // ── Client movement rule tests ──────────────────────────────────────

    /// Build a synthetic SMD: `size` x `size` terrain from `height(x, z)`,
    /// the given collision faces, all event tiles walkable.
    fn synthetic_smd(size: i32, height: impl Fn(i32, i32) -> f32, faces: &[CollisionFace]) -> SmdFile {
        let unit = 4.0f32;
        let width = (size - 1) as f32 * unit;
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_le_bytes());
        data.extend_from_slice(&unit.to_le_bytes());
        for x in 0..size {
            for z in 0..size {
                data.extend_from_slice(&height(x, z).to_le_bytes());
            }
        }
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&(faces.len() as i32).to_le_bytes());
        for face in faces {
            for v in face {
                for c in v {
                    data.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
        let cells = (width / 16.0).ceil() as u32;
        for _ in 0..cells * cells {
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        data.extend_from_slice(&0i32.to_le_bytes());
        for _ in 0..size * size {
            data.extend_from_slice(&1i16.to_le_bytes());
        }
        SmdFile::parse(&mut io::Cursor::new(&data)).expect("parse synthetic SMD")
    }

    fn blocked(smd: &SmdFile, x: i32, z: i32) -> bool {
        smd.bot_blocked[(x * smd.map_size + z) as usize]
    }

    /// Client `sub_8C7B40`: even quads split along (0,0)-(1,1), odd quads
    /// along (1,0)-(0,1). A lone raised vertex makes the two differ.
    #[test]
    fn client_terrain_height_uses_checkerboard_triangulation() {
        // 3x3 vertices, index x*3+z, only vertex (1,1) raised to 4 m.
        let mut heights = vec![0.0f32; 9];
        heights[1 * 3 + 1] = 4.0;
        let at = |x: f32, z: f32| client_terrain_height(&heights, 3, 4.0, x, z).unwrap();
        // The raised vertex itself.
        assert!((at(4.0, 4.0) - 4.0).abs() < 1e-4);
        // Quad (0,0) is even: diagonal (0,0)-(1,1). Its midpoint (2,2) lies on
        // the diagonal, halfway between 0 and 4.
        assert!((at(2.0, 2.0) - 2.0).abs() < 1e-4);
        // Same offset into quad (1,0) (odd, diagonal (1,0)-(0,1) i.e. world
        // (8,0)-(4,4)): the midpoint of that diagonal is (6,2), halfway
        // between h(2,0)=0 and h(1,1)=4.
        assert!((at(6.0, 2.0) - 2.0).abs() < 1e-4);
        // Point (5,3) is in the odd quad's far triangle (fx+fz = 0.5+... > 1
        // is false here), so it belongs to triangle (1,0),(2,0),(1,1):
        // h = h10 + (h20-h10)*fx + (h11-h10)*fz = 0 + 0*0.25 + 4*0.75 = 3.
        assert!((at(5.0, 3.0) - 3.0).abs() < 1e-4);
        assert!(client_terrain_height(&heights, 3, 4.0, -1.0, 2.0).is_none());
        assert!(client_terrain_height(&heights, 3, 4.0, 8.0, 2.0).is_none());
    }

    #[test]
    fn climb_limit_is_40_degrees_not_45() {
        // Uniform ramp along z, `rise` metres per 4 m cell.
        let ramp = |rise: f32| synthetic_smd(10, move |_, z| z as f32 * rise, &[]);
        // 3.0 m / 4 m = 36.9 deg: walkable. 3.6 m / 4 m = 42.0 deg: the old
        // 45 degree rule let bots up it, the client refuses it.
        assert!(!blocked(&ramp(3.0), 4, 4), "36.9 degree slope must stay walkable");
        assert!(blocked(&ramp(3.6), 4, 4), "42 degree slope must be blocked");
        assert!(blocked(&ramp(4.5), 4, 4), "48 degree slope must be blocked");
    }

    #[test]
    fn slope_check_follows_the_clients_triangle_split() {
        // One raised vertex (3 m): on an even quad the split puts the rise on
        // the shared diagonal so both triangles climb 3 m per 4 m; on an odd
        // quad one triangle stays flat and the other climbs 3 m per 4 m in
        // both axes (4.2 m/ 5.7 m diagonal -> steeper than 40 degrees).
        let corner_up = |qx: i32, qz: i32| {
            synthetic_smd(10, move |x, z| if (x, z) == (qx + 1, qz + 1) { 3.0 } else { 0.0 }, &[])
        };
        let even = corner_up(4, 4); // quad (4,4): 4+4 even
        let odd = corner_up(4, 5); // quad (4,5): odd
        assert!(!blocked(&even, 4, 4), "even split: both triangles climb 3 m over 4 m");
        assert!(blocked(&odd, 4, 5), "odd split: the upper triangle is steeper than 40 degrees");
    }

    #[test]
    fn map_edge_band_is_blocked() {
        let smd = synthetic_smd(12, |_, _| 0.0, &[]);
        for i in 0..12 {
            for edge in [0, 10, 11] {
                assert!(blocked(&smd, edge, i) && blocked(&smd, i, edge), "edge cell {edge},{i}");
            }
        }
        assert!(!blocked(&smd, 1, 1) && !blocked(&smd, 9, 9) && !blocked(&smd, 5, 5));
    }

    fn wall_x24(rise_over_run: Option<f32>) -> CollisionFace {
        // A wall along z = 8..40 at x = 24, 10 m tall; with `rise_over_run`
        // the top edge leans away so the face has that slope.
        match rise_over_run {
            None => [[24.0, 0.0, 8.0], [24.0, 0.0, 40.0], [24.0, 10.0, 8.0]],
            Some(r) => [[24.0, 0.0, 8.0], [24.0, 0.0, 40.0], [24.0 + 10.0 / r, 10.0, 8.0]],
        }
    }

    #[test]
    fn wall_faces_block_cells_within_the_body_radius() {
        let smd = synthetic_smd(16, |_, _| 0.0, &[wall_x24(None)]);
        // Wall at x=24: cells 5 (x 20..24) and 6 (24..28) touch it; cell 4
        // (16..20) and cell 7 (28..32) are 4 m away, outside the 1.124 radius.
        for z in 3..9 {
            assert!(blocked(&smd, 5, z) && blocked(&smd, 6, z), "cells beside the wall at z={z}");
            assert!(!blocked(&smd, 4, z) && !blocked(&smd, 7, z), "cells 4 m off the wall at z={z}");
        }
        // The wall ends at z=40: a cell well beyond it is free.
        assert!(!blocked(&smd, 5, 12));
    }

    /// The old rule only treated faces steeper than 45 degrees as walls; the
    /// client's sphere test treats everything steeper than 40 degrees as a
    /// wall (`hypot(nx,nz) > 0.643`).
    #[test]
    fn faces_between_40_and_45_degrees_are_walls_but_shallower_ones_are_floors() {
        let leaning = |slope_tan: f32| synthetic_smd(16, |_, _| 0.0, &[wall_x24(Some(slope_tan))]);
        // tan(42 deg) = 0.9, tan(35 deg) = 0.7
        assert!(blocked(&leaning(0.9), 6, 5), "42 degree face must block");
        assert!(!blocked(&leaning(0.7), 6, 5), "35 degree face is a ramp, not a wall");
    }

    #[test]
    fn garbage_faces_are_ignored_not_fatal() {
        let garbage: CollisionFace = [[-46658.7, -44610.7, -0.1], [1.0e29, 3.5e24, 1.9e36], [0.0, 71813.9, 0.0]];
        let smd = synthetic_smd(16, |_, _| 0.0, &[garbage]);
        assert!(!blocked(&smd, 5, 5) && !blocked(&smd, 8, 8));
    }

    #[test]
    fn wall_index_clearance_matches_distance_to_the_wall() {
        let face = wall_x24(None);
        let index = WallIndex::new(&[face], 64.0);
        // 3 m in front of the wall; the sphere centre is 1.724 up but the
        // wall is 10 m tall, so the nearest point is straight across.
        let c = index.clearance(21.0, 0.0, 20.0);
        assert!((c - 3.0).abs() < 0.01, "clearance {c}");
        // A shallow ramp is not a wall and never reports clearance.
        let ramp: CollisionFace = [[24.0, 0.0, 8.0], [24.0, 0.0, 40.0], [38.0, 10.0, 8.0]];
        assert_eq!(WallIndex::new(&[ramp], 64.0).clearance(21.0, 0.0, 20.0), f32::INFINITY);
    }

    #[test]
    fn client_incline_blocks_only_uphill_on_steep_triangles() {
        // 42 degree ramp rising along +z.
        let size = 6;
        let heights: Vec<f32> = (0..size).flat_map(|_| (0..size).map(|z| z as f32 * 3.6)).collect();
        let (x, z) = (10.0, 10.0);
        assert!(client_incline_blocks(&heights, size, 4.0, x, z, 0.0, 1.0), "walking up must be refused");
        assert!(client_incline_blocks(&heights, size, 4.0, x, z, 1.0, 0.0), "along the contour is refused too");
        assert!(!client_incline_blocks(&heights, size, 4.0, x, z, 0.0, -1.0), "walking down is allowed");
        let gentle: Vec<f32> = (0..size).flat_map(|_| (0..size).map(|z| z as f32 * 3.0)).collect();
        assert!(!client_incline_blocks(&gentle, size, 4.0, x, z, 0.0, 1.0), "36.9 degrees is climbable");
    }

    /// Integration test: parse a real SMD file (moradon_0826.smd).
    ///
    /// This test only runs when the Map directory exists with actual SMD files.
    #[test]
    fn test_parse_real_moradon_smd() {
        let path = std::path::Path::new("../../Map/moradon_0826.smd");
        if !path.exists() {
            eprintln!("Skipping test_parse_real_moradon_smd: Map/moradon_0826.smd not found");
            return;
        }

        let smd = SmdFile::load(path).unwrap_or_else(|e| {
            panic!("failed to parse moradon_0826.smd: {}", e);
        });

        // Moradon: world size should be ~4096 meters
        // map_size can vary (257 with unit_dist=16, or 1025 with unit_dist=4)
        assert!(smd.map_size > 0, "map_size must be positive");
        assert!(smd.unit_dist > 0.0, "unit_dist must be positive");

        // (map_size - 1) * unit_dist should give the actual map dimensions
        let expected_width = (smd.map_size - 1) as f32 * smd.unit_dist;
        assert!(
            (expected_width - smd.map_width).abs() < 1.0,
            "terrain/collision dimension mismatch: terrain={}, collision={}",
            expected_width,
            smd.map_width
        );

        // Map dimensions should be reasonable (100..8192)
        assert!(
            smd.map_width > 100.0 && smd.map_width < 8192.0,
            "unexpected map_width: {}",
            smd.map_width
        );

        // Event grid should be map_size²
        assert_eq!(
            smd.event_grid.len(),
            (smd.map_size as usize) * (smd.map_size as usize),
            "event_grid size mismatch"
        );

        // Event grid might be all 0 on some simplified SMD files
        let nonzero_count = smd.event_grid.iter().filter(|&&v| v != 0).count();

        // Validity checks
        assert!(smd.is_valid_position(100.0, 100.0));
        assert!(!smd.is_valid_position(smd.map_width + 1.0, 100.0));

        println!(
            "Moradon SMD: map_size={}, unit_dist={}, map_width={}, map_height={}",
            smd.map_size, smd.unit_dist, smd.map_width, smd.map_height
        );
        println!(
            "  event_grid: {} tiles ({} non-zero), warps={}, regene={}",
            smd.event_grid.len(),
            nonzero_count,
            smd.warps.len(),
            smd.regene_events.len(),
        );

        // Print first few warps for verification
        for w in smd.warps.iter().take(5) {
            println!(
                "  Warp #{}: '{}' → zone {} ({}, {}, {}) radius={} nation={}",
                w.warp_id, w.name, w.dest_zone, w.dest_x, w.dest_y, w.dest_z, w.radius, w.nation
            );
        }

        // Print first few regene events
        for r in smd.regene_events.iter().take(3) {
            println!(
                "  Regene #{}: pos=({}, {}, {}) area=({}, {})",
                r.point_id, r.pos_x, r.pos_y, r.pos_z, r.area_x, r.area_z
            );
        }

        // Check regene events have reasonable positions
        for r in &smd.regene_events {
            assert!(
                r.pos_x >= 0.0 && r.pos_x <= smd.map_width + 100.0,
                "regene pos_x out of range: {} (map_width={})",
                r.pos_x,
                smd.map_width
            );
        }
    }

    /// Test parsing multiple SMD files to verify broad compatibility.
    #[test]
    fn test_parse_multiple_smd_files() {
        let map_dir = std::path::Path::new("../../Map");
        if !map_dir.exists() {
            eprintln!("Skipping test_parse_multiple_smd_files: Map/ not found");
            return;
        }

        let mut ok_count = 0;
        let mut err_count = 0;
        let entries: Vec<_> = std::fs::read_dir(map_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "smd")
                    .unwrap_or(false)
            })
            .collect();

        for entry in &entries {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();
            match SmdFile::load(&path) {
                Ok(smd) => {
                    ok_count += 1;
                    let nonzero = smd.event_grid.iter().filter(|&&v| v != 0).count();
                    println!(
                        "OK {}: map={}x{} ({:.0}x{:.0}m) events={} warps={} regene={}",
                        name,
                        smd.map_size,
                        smd.map_size,
                        smd.map_width,
                        smd.map_height,
                        nonzero,
                        smd.warps.len(),
                        smd.regene_events.len()
                    );
                }
                Err(e) => {
                    err_count += 1;
                    eprintln!("FAIL {}: {}", name, e);
                }
            }
        }

        println!(
            "\nTotal: {} OK, {} failed out of {} files",
            ok_count,
            err_count,
            entries.len()
        );
        // Allow some failures (broken/old SMDs) but majority should parse
        assert!(
            ok_count > entries.len() / 2,
            "too many parse failures: {}/{}",
            err_count,
            entries.len()
        );
    }
}

#[cfg(test)]
mod event_grid_dump {
    /// Diagnostic: `cargo test -p ko-protocol --lib -- event_grid_dump --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump_bdw_event_grid() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map");
        for name in ["In_dungeon04.smd", "sky_war_2009.smd"] {
            let Ok(smd) = super::SmdFile::load(&root.join(name)) else { continue };
            println!("{name}: size {} unit {} nonzero {}", smd.map_size, smd.unit_dist, smd.event_grid.iter().filter(|&&v| v != 0).count());
            for z in (0..65.min(smd.map_size)).rev() {
                let row: String = (0..65.min(smd.map_size)).map(|x| match smd.get_event_id(x, z) { 0 => '#', v if v < 10 => char::from(b'0' + v as u8), _ => '+' }).collect();
                println!("{row} {z}");
            }
        }
    }
}
