//! Authored navigation openings for the event arenas (zones 84 / 85 / 87).
//!
//! The bot navigation mask of these zones is built from the client's baked collision
//! (`Map/nav/<zone>.nav`) and is the **only** walkability source for them: the server SMDs of
//! BDW (`In_dungeon04.smd`) and Chaos (`sky_war_2009.smd`) are 1024 m maps that do not belong to
//! the 256 m client arenas (the BDW event grid is `1` almost everywhere, the Chaos one is all
//! zero), so OR-ing the SMD event grid into the mask (the previous rule) made nearly every BDW
//! cell walkable and sent the bots straight into the rocks.
//!
//! The client mask alone leaves a few designed passages closed, for two reasons that were
//! verified cell by cell against the client `.gtd` / `.opd` files
//! (`docs/reverse-engineering/2625/CLIENT_MAP_NAV.md`, "Event arenas"):
//!
//! * **Drops.** The spawn castles of BDW and the south room of Chaos sit on a raised terrain
//!   level (BDW spawn ~11 m above the base at ~3 m, Chaos south room 6.9 m above the ring at
//!   1.9 m). The client only refuses *uphill* steps steeper than 40 degrees, so players leave by
//!   walking off the ledge; the mask is undirected and marks the ledge blocked both ways.
//! * **Gates narrower than two cells.** The BDW base gates are 6.4 m gaps between two bastions
//!   (`.opd` faces 68.3..82.3 m / 173..187 m) straddling two 4 m cells; the 1.124 m body
//!   clearance touches both cells, so a gap a player walks through is closed in the mask.
//!   The Chaos corridor gates are cage doors that open when the event starts (dynamic objects,
//!   baked closed).
//!
//! So each arena gets a short list of **openings**: world-space rectangles that bots may walk
//! in addition to the mask. They are verified by the real-map reachability tests in
//! `event_bots::tests` (which also assert that sampled rock cells stay blocked) and are
//! consulted by `MapData::bot_cell_walkable` for the event zones only.
//!
//! Dynamic gates (Juraid bridges) are not openings: a closed bridge is a per-room dynamic
//! obstacle (`zone::DynBlockGuard`) installed by the Juraid objective.

use crate::world::types::{ZONE_BDW, ZONE_CHAOS, ZONE_JURAID};

/// `(x0, z0, x1, z1)` in world units, inclusive.
pub type Rect = (f32, f32, f32, f32);

/// Chaos arena (`sky_war_2009`, 256 m): the centre's north corridor ends in the baked cage
/// gate at z 192..200 (open while the event runs) and its south corridor in the gate at
/// z 72..80 into the raised south room.
const CHAOS: &[Rect] = &[
    (104.0, 186.0, 148.0, 206.0), // north corridor gate -> outer ring
    (104.0, 66.0, 148.0, 86.0),   // south corridor gate -> south room
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

/// Border Defence War (`In_dungeon04`, 256 m, point-symmetric about (128, 128)).
/// Cells are 4 m; a rectangle must contain the centre of every cell it opens.
const BDW: &[Rect] = &[
    // Karus base gate: 6.4 m gap between the bastions at x 68.3..82.3 (z 115.8..128.8 and
    // z 135.2..148.2 in the .opd), i.e. cells x 17..20, z 32..33.
    (68.0, 128.5, 83.0, 135.5),
    // El Morad base gate: gap between the bastions at x 173..187 (z 114.3..127.1 and
    // z 134.1..146.4), cells x 43..46, z 31..33.
    (173.0, 124.5, 187.0, 135.5),
    // Karus spawn castle ledge: terrain drops from ~11 m (z <= 100) to ~4 m (z >= 104) over
    // one cell row at x 28..60; players walk off it, bots may too (cells x 7..14, z 25).
    (28.0, 100.0, 60.0, 104.0),
    // El Morad spawn castle ledge: ~11 m (z >= 164) down to ~4 m (z <= 160) at x 188..224
    // (cells x 47..55, z 40).
    (188.0, 160.0, 224.0, 164.0),
];

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
        assert!(openings(71).is_empty() && openings(21).is_empty() && openings(72).is_empty());
        assert!(!openings(ZONE_BDW).is_empty() && !openings(ZONE_CHAOS).is_empty() && !openings(ZONE_JURAID).is_empty());
        // BDW gates open only the gap cells, not the bastions beside them.
        assert!(contains(openings(ZONE_BDW), 74.0, 130.0) && contains(openings(ZONE_BDW), 78.0, 134.0));
        assert!(!contains(openings(ZONE_BDW), 74.0, 122.0) && !contains(openings(ZONE_BDW), 74.0, 142.0));
        assert!(contains(openings(ZONE_BDW), 182.0, 130.0) && !contains(openings(ZONE_BDW), 182.0, 142.0));
        assert!(contains(openings(ZONE_CHAOS), 126.0, 196.0) && !contains(openings(ZONE_CHAOS), 126.0, 130.0));
    }
}
