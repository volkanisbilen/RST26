//! Authored navigation openings for the event arenas (zones 84 / 85 / 87).
//!
//! The bot navigation mask of these zones is built from the client's baked collision
//! (`Map/nav/<zone>.nav`). That mask is right for terrain maps but these arenas are dungeon
//! maps: doors, bridges and ramps are dynamic objects (not in the baked faces) and the floors
//! of their upper levels are object tops, which the mask builder treats as unwalkable. The
//! result is a mask whose spawn rooms / lane cages are closed pockets (BDW: Karus spawn 123
//! cells, El Morad 107; Juraid: each spawn cage 36 cells; Chaos: the centre sealed off from
//! the outer ring) although players walk straight through. The server SMD helps only for BDW
//! (its event grid is the arena layout); the Chaos and Juraid SMDs carry no event grid.
//!
//! So each arena gets a short list of **openings**: world-space rectangles that bots may walk
//! in addition to the mask / event grid. They follow the event design (the lane centre lines
//! and the doorways the mask shows as 2-cell gaps everywhere else), are verified by the
//! real-map reachability tests in `event_bots::tests` and are consulted by
//! `MapData::bot_cell_walkable` for the event zones only.
//!
//! Dynamic gates (Juraid bridges) are not openings: a closed bridge is a per-room dynamic
//! obstacle (`zone::DynBlockGuard`) installed by the Juraid objective.

use crate::world::types::{ZONE_BDW, ZONE_CHAOS, ZONE_JURAID};

/// `(x0, z0, x1, z1)` in world units, inclusive.
pub type Rect = (f32, f32, f32, f32);

/// Chaos arena (`sky_war_2009`, 256 m): the centre's north and south corridors end in a
/// baked wall the players pass over (ramps to the outer ring).
const CHAOS: &[Rect] = &[
    (104.0, 186.0, 148.0, 206.0), // north corridor end -> outer ring
    (104.0, 66.0, 148.0, 86.0),   // south corridor end -> outer ring
];

/// Juraid Mountain (`In_Dungeon05`, 1024 m): the lane centre lines (8 m wide, the width of
/// the doorways in every `######..######` chamber wall of the mask) from each spawn cage
/// through the three stages into the Deva room, plus the Deva room itself.
const JURAID: &[Rect] = &[
    // Karus: x = 224 lane up, z = 848 across, x = 511 down to the Deva room
    (219.0, 256.0, 229.0, 856.0),
    (219.0, 843.0, 516.0, 853.0),
    (506.0, 505.0, 516.0, 853.0),
    // El Morad: x = 799 lane down, z = 172 across, x = 511 up to the Deva room
    (794.0, 164.0, 804.0, 760.0),
    (506.0, 167.0, 804.0, 177.0),
    (506.0, 167.0, 516.0, 521.0),
    // Deva room
    (478.0, 488.0, 544.0, 540.0),
];

/// Border Defence War (`In_dungeon04`, 256 m): the SMD event grid already carries the arena
/// layout (doors included); nothing authored.
const BDW: &[Rect] = &[];

/// Authored openings of an event zone (empty for every other zone).
pub fn openings(zone_id: u16) -> &'static [Rect] {
    match zone_id {
        ZONE_BDW => BDW,
        ZONE_CHAOS => CHAOS,
        ZONE_JURAID => JURAID,
        _ => &[],
    }
}

/// Is the world point inside one of `rects`?
pub fn contains(rects: &[Rect], x: f32, z: f32) -> bool {
    rects.iter().any(|&(x0, z0, x1, z1)| x >= x0 && x <= x1 && z >= z0 && z <= z1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::event_bots_juraid::{chain, DEVA_ROOM, ELMORAD_SPAWN, KARUS_SPAWN};

    #[test]
    fn juraid_corridors_cover_every_waypoint_of_both_lanes() {
        let r = openings(ZONE_JURAID);
        for (nation, spawn) in [(1u8, KARUS_SPAWN), (2u8, ELMORAD_SPAWN)] {
            assert!(contains(r, spawn.0, spawn.1), "nation {nation} spawn");
            for wp in chain(nation) {
                assert!(contains(r, wp.0, wp.1), "nation {nation} waypoint {wp:?}");
            }
        }
        assert!(contains(r, DEVA_ROOM.0, DEVA_ROOM.1));
        assert!(!contains(r, 100.0, 100.0), "openings are corridors, not the whole map");
    }

    #[test]
    fn only_the_event_zones_have_openings() {
        assert!(openings(71).is_empty() && openings(21).is_empty() && openings(ZONE_BDW).is_empty());
        assert!(!openings(ZONE_CHAOS).is_empty() && !openings(ZONE_JURAID).is_empty());
        assert!(contains(openings(ZONE_CHAOS), 126.0, 196.0) && !contains(openings(ZONE_CHAOS), 126.0, 130.0));
    }
}
