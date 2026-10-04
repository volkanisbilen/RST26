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
const CELL_MAIN_DEVIDE: i32 = 4;
const CELL_SUB_SIZE: i32 = 4;
const CELL_MAIN_SIZE: i32 = CELL_MAIN_DEVIDE * CELL_SUB_SIZE; // 16

/// Size of the packed `_WARP_INFO` struct in bytes.
/// Layout: i16(2) + char[32](32) + char[256](256) + u16(2) + u32(4) +
///   i16(2) + u16(2) + f32×4(16) + i16(2) + u16(2) = 320
const WARP_INFO_SIZE: usize = 320;

/// Size of the packed `_OBJECT_EVENT` struct read from the file.
const OBJECT_EVENT_SIZE_LEGACY: usize = 24;
/// Packed object-event size used by the Manes map exporter.
const OBJECT_EVENT_SIZE_HEADERED: usize = 19;

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

    /// Parse SMD from a reader.
    pub fn parse<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
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
        let mut bot_blocked = vec![true; heights.len()];
        for x in 0..map_size - 1 {
            for z in 0..map_size - 1 {
                let i = (x * map_size + z) as usize;
                let h = [heights[i], heights[i + 1], heights[i + map_size as usize], heights[i + map_size as usize + 1]];
                // Both terrain triangles must have a gradient <= 45 degrees.
                let slope1 = (h[2] - h[0]).hypot(h[1] - h[0]) / unit_dist;
                let slope2 = (h[3] - h[1]).hypot(h[3] - h[2]) / unit_dist;
                bot_blocked[i] = h.iter().any(|v| !v.is_finite()) || slope1 > 1.0 || slope2 > 1.0;
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
        let (map_width, map_height) = Self::skip_collision_data(reader, map_size, unit_dist, &heights, &mut bot_blocked)?;
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

        Ok(SmdFile {
            map_size,
            unit_dist,
            map_width,
            map_height,
            event_grid,
            heights,
            bot_blocked,
            warps,
            regene_events,
        })
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

    /// Skip collision data (CN3ShapeMgr::LoadCollisionData), extracting map dimensions.
    ///
    fn skip_collision_data<R: Read + Seek>(reader: &mut R, size: i32, unit: f32, heights: &[f32], blocked: &mut [bool]) -> io::Result<(f32, f32)> {
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
        // Each face = 3 vertices, each vertex = 3 floats (12 bytes)
        for _ in 0..face_count {
            let mut v = [[0.0f32; 3]; 3];
            for vertex in &mut v {
                for coordinate in vertex { *coordinate = read_f32(reader)?; }
            }
            if v.iter().flatten().any(|c| !c.is_finite()) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid collision vertex"));
            }
            let a = [v[1][0]-v[0][0], v[1][1]-v[0][1], v[1][2]-v[0][2]];
            let b = [v[2][0]-v[0][0], v[2][1]-v[0][1], v[2][2]-v[0][2]];
            let n = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
            // Floors/bridges do not form walls; retain steep collision faces.
            if n[1].abs() >= n[0].hypot(n[2]) { continue; }
            let min = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
            let max = |axis: usize| v.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
            for x in ((min(0) / unit).floor() as i32).max(0)..=((max(0) / unit).floor() as i32).min(size-2) {
                for z in ((min(2) / unit).floor() as i32).max(0)..=((max(2) / unit).floor() as i32).min(size-2) {
                    // Separating-axis test: do not block the empty corners of a
                    // long diagonal wall's bounding rectangle.
                    let center = [(x as f32 + 0.5) * unit, (z as f32 + 0.5) * unit];
                    let half = unit * 0.5 + 0.2;
                    let separated = (0..3).any(|edge| {
                        let next = (edge + 1) % 3;
                        let axis = [v[next][2] - v[edge][2], v[edge][0] - v[next][0]];
                        let lo = v.iter().map(|p| p[0]*axis[0] + p[2]*axis[1]).fold(f32::INFINITY, f32::min);
                        let hi = v.iter().map(|p| p[0]*axis[0] + p[2]*axis[1]).fold(f32::NEG_INFINITY, f32::max);
                        let c = center[0]*axis[0] + center[1]*axis[1];
                        let r = half * (axis[0].abs() + axis[1].abs());
                        hi < c-r || lo > c+r
                    });
                    if separated { continue; }
                    let i = (x * size + z) as usize;
                    let h = [heights[i], heights[i+1], heights[i+size as usize], heights[i+size as usize+1]];
                    let low = h.iter().copied().fold(f32::INFINITY, f32::min);
                    let high = h.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    if max(1) > low + 0.4 && min(1) < high + 2.0 { blocked[i] = true; }
                }
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

        Ok((map_width, map_length))
    }

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
fn read_i16<R: Read>(r: &mut R) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_i32<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32<R: Read>(r: &mut R) -> io::Result<f32> {
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
