# Client map formats and the bot navigation mask (`Map/nav`)

Bots in zones 71/72/73 climbed mountains and crossed walls because the
server `.smd` terrain/collision is **not the map the 2625 client plays on**:

| zone | server smd | client map (`Data/Zones.tbl`, row id = zone*10) | finding |
|------|------------|------------------------------------------------|---------|
| 71 Ronark Land | `freezone_b.smd` 513x513 | `Zones/Freezone_b.gtd/.opd` (Feb 2024) 513x513 | 190,833 of 263,169 height samples differ (max 86 m); opd has 73,712 collision faces, smd 22,035 |
| 72 Ardream | `freezone_a.smd` 257x257 | `Zones/FreeZone_a.*` 257x257 | terrain matches (330 samples differ); smd has 11,319 faces of which 1,591 are garbage (faces 648..2270), opd has 12,636 sane faces |
| 73 Ronark Land Base | `freezone_c.smd` 513x513 | `Zones/FreeZone_c.*` 257x257 (1024 m) | `freezone_c.smd` heights are byte-identical to `freezone_b.smd`: the server map is a different map altogether |
| 21 Moradon | `moradon_0826.smd` | `Zones/Moradon.*` (Aug 2025) | 61k samples differ |
| 84 BDW / 85 Chaos | 257x257 smd | 65x65 (256 m) client maps | different maps; BDW keeps event-grid navigation |

So the bot mask is now generated from the client's own files and shipped as
`Map/nav/<zone>.nav`; the server loads it in preference to the SMD mask.

## Formats (decoded from the files; constants from `KnightOnline.exe` decompile)

### `.gtd` — `CN3Terrain`
```
i32 name_len; u8[name_len] obfuscated name        (CN3BaseFileAccess)
i32 flag                                          (0 or 1, seen both)
MAPDATA[size*size] { u32 tex bitfield; f32 height }   x-major: [x*size+z]
... patch bounds, tile texture tables, lightmaps (not needed)
```
`size` is not stored: 257 for 1024 m maps, 513 for 2048 m, recovered from the
file length (`ko_protocol::nav::ClientTerrain::parse`). Terrain unit is 4.0.
Evidence: `FreeZone_a.gtd` decoded this way reproduces `freezone_a.smd`'s
height grid (65,719 of 66,049 samples equal to 0.01 m) in the same x-major
order.

### `.opd` — `CN3ShapeMgr`
```
i32 name_len; u8[name_len] name; i32 version
f32 map_width; f32 map_length
i32 face_count; [3 x (f32 x,y,z)] * face_count     baked collision faces
cell grid ceil(width/16) x ceil(length/16):
    u32 exists; if exists: i32 n; u16[n] shape refs; 16 x { i32 polys; u32[polys*3] }
i32 shape_count; shapes ...                         (not needed)
```
Identical block to the one `SmdFile` already reads from the `.smd`
(`smd::read_shape_mgr_collision` is shared). The client's movement
collision `sub_6B0510` (swept sphere, radius clamped to 1.124) and
`sub_6B20C0` (ray) iterate the cell polygon lists into this face array
(`this+88`), never the shape meshes, so the baked faces are the complete
object collision; `.n3shape` meshes are not needed. A face counts only when
`hypot(nx, nz) > 0.643` (line 658360 of the decompile), the player's Y is
always the terrain height (`sub_8C7B40`): object tops are never walkable and
shallow object faces are ignored — exactly what the mask does.

### `.nav` (ours)
```
"KONV" u32 1; i32 size; f32 unit; u32 flags(bit0 heights)
f32[size*size] heights; u64[ceil(size*size/64)] blocked bitset (bit = x*size+z)
```
### `.walked` (ours, player trace)
```
"KOWK" u32 1; i32 size; u64[...] walked cells; u64[...] walked edges (4 per cell)
```

## Rules used to rasterise (unchanged from `smd.rs`, now fed correct data)
* terrain quad blocked if either client triangle (checkerboard split) is
  steeper than sin 0.643 (40 deg), or in the 4 m edge band;
* cell blocked if a wall-like face comes within 1.124 m of the cell box and
  overlaps feet+0.5 .. feet+2.848;
* diagonal corner cutting is refused by A* and the segment walker.

## Regenerate
```
cargo run --release -p ko-protocol --bin navgen -- --client "D:/ai/JstKO Client" \
    --map-dir Map --out Map/nav --debug <scratch>/nav-debug [--zones 71,72,73]
```
Prints per zone: walkable cells, reachability from the bot spawn anchors
per quadrant, and the disagreement with the SMD mask (SMD-only cells = old
leaks, client-only = old over-blocking). PNGs: green reachable, grey
walkable, red old-mask-only, blue client-only, dark blocked.

## Runtime
* `MapData` holds `BotNav { nav: Option<NavMask>, walked: WalkedMap }`
  behind a `RwLock<Arc<..>>`; `is_bot_movable_grid`, `terrain_height`,
  `client_incline_blocks` and the bot A* use the client data when present.
  BDW (`bot_event_only`) keeps the event-grid rule.
* Move handler records every validated non-GM (authority 0/250/251 excluded)
  move into the zone's `WalkedMap` (jumps > 12 m ignored); flushed every
  5 min to `Map/nav/<zone>.walked`, loaded at start.
* With `>= KO_NAV_WALKED_MIN_CELLS` (default 1500) walked cells, A* adds
  `COST_UNWALKED_PENALTY` (10 on the 10/11 scale, i.e. unwalked cells cost double) for entering unwalked
  cells; walked cells never override a blocked mask cell.
* GM: `+navreload [zone]`, `+navstats [zone]`, `+navdump [zone]` (PNG under
  `Map/nav/debug`, yellow bots, cyan players, green walked).
