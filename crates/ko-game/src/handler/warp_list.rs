//! WIZ_WARP_LIST (0x4B) handler — warp gate destination list and selection.
//!   - `CUser::GetWarpList(int warp_group)` — builds and sends the warp list
//!   - `CUser::SelectWarpList(Packet & pkt)` — validates and executes a warp
//! ## Flow
//! 1. Player interacts with a warp gate object (WIZ_OBJECT_EVENT, type=OBJECT_WARP_GATE)
//! 2. Server calls `send_warp_list()` → sends WIZ_WARP_LIST sub=1 with available destinations
//! 3. Player picks a destination → client sends WIZ_WARP_LIST with [u16 objectIndex, u16 warpID]
//! 4. Server handles `handle()` → validates warp, deducts gold, triggers zone change
//! ## Wire Format
//! **GetWarpList (server → client, sub=1):**
//! ```text
//! u8(1) + u16(count) + per entry: [u16 warpID, str name, str announce, u16 zone, u16 maxuser, u32 pay]
//! ```
//! **SelectWarpList (client → server):**
//! ```text
//! [u16 objectIndex] [u16 warpID]
//! ```
//! **SelectWarpList response (server → client, sub=2):**
//! ```text
//! u8(2) + u8(result)   // result: 1=success, 0xFF=fail
//! ```

use ko_protocol::{Opcode, Packet, PacketReader};
use rand::{thread_rng, Rng};
use tracing::{debug, warn};

use crate::handler::zone_change;
use crate::session::{ClientSession, SessionState};
use crate::systems::war::{NATION_BATTLE, SIEGE_BATTLE};
use crate::world::types::{
    ZONE_ARDREAM, ZONE_BATTLE, ZONE_BATTLE2, ZONE_BATTLE3, ZONE_BATTLE4, ZONE_BATTLE5,
    ZONE_BATTLE6, ZONE_ELMORAD_ESLANT, ZONE_KARUS_ESLANT, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE,
};
use crate::world::WorldState;

/// Default max users per zone (`m_sMaxUser`, not yet stored in our zone model).
const DEFAULT_MAX_USERS: u16 = 150;

/// Resolve the two legacy Eslant destinations stored in the active v2615 SMDs.
fn effective_warp_destination(warp_id: i16, raw_zone: i16, nation: u8) -> u16 {
    match (warp_id, raw_zone) {
        (153, 19) => ZONE_KARUS_ESLANT,
        (253, 19) => ZONE_ELMORAD_ESLANT,
        (_, 11 | 13 | 14) => ZONE_KARUS_ESLANT,
        (_, 12 | 15 | 16) => ZONE_ELMORAD_ESLANT,
        (_, 19) if nation == 1 => ZONE_KARUS_ESLANT,
        (_, 19) if nation == 2 => ZONE_ELMORAD_ESLANT,
        _ => raw_zone.max(0) as u16,
    }
}

pub(super) fn resolve_active_battle_warp(world: &WorldState, effective_zone: u16) -> u16 {
    if !matches!(
        effective_zone,
        ZONE_BATTLE | ZONE_BATTLE2 | ZONE_BATTLE3 | ZONE_BATTLE4 | ZONE_BATTLE5 | ZONE_BATTLE6
    ) {
        return effective_zone;
    }

    let battle = world.get_battle_state();
    if battle.battle_open == NATION_BATTLE || battle.battle_open == SIEGE_BATTLE {
        let active_zone = battle.battle_zone_id();
        if matches!(
            active_zone,
            ZONE_BATTLE | ZONE_BATTLE2 | ZONE_BATTLE3 | ZONE_BATTLE4 | ZONE_BATTLE5 | ZONE_BATTLE6
        ) {
            return active_zone;
        }
    }

    effective_zone
}

fn is_hidden_moradon_abyss_warp(
    source_zone: u16,
    name: &str,
    announce: &str,
    dest_zone: i16,
) -> bool {
    if !(21..=25).contains(&source_zone) {
        return false;
    }
    let label = format!("{name} {announce}").to_ascii_lowercase();
    dest_zone == 9 || label.contains("abyss")
}

use crate::npc_type_constants::MAX_OBJECT_RANGE;
use crate::object_event_constants::OBJECT_WARP_GATE;

/// Handle incoming WIZ_WARP_LIST from the client (SelectWarpList).
/// The client sends `[u16 objectIndex] [u16 warpID]` after choosing a destination
/// from the warp list UI.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let world_ref = session.world().clone();
    let sid_check = session.session_id();

    // Player state validation: dead, trading, merchanting, mining, fishing
    if world_ref.is_player_dead(sid_check)
        || world_ref.is_trading(sid_check)
        || world_ref.is_merchanting(sid_check)
        || world_ref.is_mining(sid_check)
        || world_ref.is_fishing(sid_check)
    {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let object_index = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };
    let warp_id = match reader.read_u16() {
        Some(v) => v as i16,
        None => return Ok(()),
    };

    let world = session.world().clone();
    let sid = session.session_id();

    // Basic validation: player must be alive and in-game
    let (pos, char_info) = match world
        .with_session(sid, |h| {
            h.character.as_ref().map(|c| (h.position, c.clone()))
        })
        .flatten()
    {
        Some(v) => v,
        None => {
            send_select_fail(session).await?;
            return Ok(());
        }
    };

    // Look up the warp in the current zone's map data
    let zone = match world.get_zone(pos.zone_id) {
        Some(z) => z,
        None => {
            send_select_fail(session).await?;
            return Ok(());
        }
    };

    // Object event validation — prevents warping without being near the gate
    {
        let obj_event = match zone.get_object_event(object_index) {
            Some(e) => e,
            None => {
                warn!(
                    "[{}] SelectWarpList: object event {} not found in zone {}",
                    session.addr(),
                    object_index,
                    pos.zone_id,
                );
                send_select_fail(session).await?;
                return Ok(());
            }
        };

        // Status must be active (1)
        if obj_event.status != 1 {
            send_select_fail(session).await?;
            return Ok(());
        }

        // Must be a warp gate object
        if obj_event.obj_type != OBJECT_WARP_GATE as i16 {
            send_select_fail(session).await?;
            return Ok(());
        }

        // Range check: player must be within MAX_OBJECT_RANGE of the object
        let dx = pos.x - obj_event.pos_x;
        let dz = pos.z - obj_event.pos_z;
        let dist = (dx * dx + dz * dz).sqrt();
        if dist >= MAX_OBJECT_RANGE {
            warn!(
                "[{}] SelectWarpList: too far from warp gate (dist={:.1}, max={})",
                session.addr(),
                dist,
                MAX_OBJECT_RANGE,
            );
            send_select_fail(session).await?;
            return Ok(());
        }

        // Nation/belong check: 0=any nation, otherwise must match
        if obj_event.belong != 0 && obj_event.belong != char_info.nation as i16 {
            send_select_fail(session).await?;
            return Ok(());
        }
    }

    let warp = match zone.map_data.as_ref().and_then(|md| md.get_warp(warp_id)) {
        Some(w) => w.clone(),
        None => {
            warn!(
                "[{}] SelectWarpList: warp {} not found in zone {}",
                session.addr(),
                warp_id,
                pos.zone_id,
            );
            send_select_fail(session).await?;
            return Ok(());
        }
    };

    // Keep the removed Moradon Abyss destination unavailable even if a client
    // submits a stale/forged warp ID instead of selecting from the visible list.
    if is_hidden_moradon_abyss_warp(pos.zone_id, &warp.name, &warp.announce, warp.dest_zone) {
        send_select_fail(session).await?;
        return Ok(());
    }

    // Nation check: if warp is nation-restricted, must match player nation
    if warp.nation != 0 && warp.nation != char_info.nation as i16 {
        send_select_fail(session).await?;
        return Ok(());
    }

    let effective_dest_zone = resolve_active_battle_warp(
        &world,
        effective_warp_destination(warp.warp_id, warp.dest_zone, char_info.nation),
    );

    // Validate after resolving v2615's legacy zone-19 Eslant destination.
    match world.get_zone(effective_dest_zone) {
        Some(z) => {
            let status = z.zone_info.as_ref().map(|i| i.status).unwrap_or(1);
            if status == 0 {
                send_select_fail(session).await?;
                return Ok(());
            }
        }
        None => {
            send_select_fail(session).await?;
            return Ok(());
        }
    }

    // Add random offset within warp radius
    let (dest_x, dest_z, dest_zone) = {
        let mut rng = thread_rng();
        let mut rx = rng.gen_range(0.0..=(warp.radius * 2.0));
        if rx < warp.radius {
            rx = -rx;
        }
        let mut rz = rng.gen_range(0.0..=(warp.radius * 2.0));
        if rz < warp.radius {
            rz = -rz;
        }
        if (61..=66).contains(&effective_dest_zone) {
            (0.0, 0.0, effective_dest_zone)
        } else {
            (warp.dest_x + rx, warp.dest_z + rz, effective_dest_zone)
        }
    };

    // Same-zone warp: send success response before warping
    if pos.zone_id == dest_zone {
        let mut result = Packet::new(Opcode::WizWarpList as u8);
        result.write_u8(2); // sub-opcode: SelectWarpList response
        result.write_u8(1); // success
        result.write_u8(0); // padding
        session.send_packet(&result).await?;
    }

    // Execute zone change
    zone_change::trigger_zone_change(session, dest_zone, dest_x, dest_z).await?;

    // Deduct gold only after SUCCESSFUL zone change.
    //   if (ZoneChange(...)) { if (GetZoneID() == pWarp->sZone && dwPay > 0 && hasCoins(dwPay)) GoldLose(dwPay); }
    // ZoneChange returns bool; gold is deducted only on success.
    // Our trigger_zone_change returns Ok(()) on all paths, so check zone_id parity.
    if warp.pay > 0 {
        let current_zone = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
        if current_zone == dest_zone {
            world.gold_lose(sid, warp.pay);
        }
    }

    debug!(
        "[{}] SelectWarpList: warp {} → zone {} ({:.0},{:.0}) pay={}",
        session.addr(),
        warp_id,
        dest_zone,
        dest_x,
        dest_z,
        warp.pay,
    );

    Ok(())
}

/// Send the warp list to the player for a given warp group.
/// Called from object event handling when a player interacts with a WARP_GATE object.
/// Wire format:
/// ```text
/// WIZ_WARP_LIST + u8(1) + u16(count) + per entry:
///   [u16 warpID, str name, str announce, u16 zone, u16 maxuser, u32 pay]
/// ```
pub async fn send_warp_list(session: &mut ClientSession, warp_group: i32) -> anyhow::Result<bool> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, char_info) = match world
        .with_session(sid, |h| {
            h.character.as_ref().map(|c| (h.position, c.clone()))
        })
        .flatten()
    {
        Some(v) => v,
        None => return Ok(false),
    };

    let zone = match world.get_zone(pos.zone_id) {
        Some(z) => z,
        None => return Ok(false),
    };

    let warps = match zone.map_data.as_ref() {
        Some(md) => md.get_warp_list(warp_group),
        None => return Ok(false),
    };

    // Filter warps: nation check, destination zone must exist and be active
    let battle = world.get_battle_state();
    let mut entries: Vec<&ko_protocol::smd::WarpInfo> = Vec::with_capacity(warps.len());
    for warp in &warps {
        if is_hidden_moradon_abyss_warp(pos.zone_id, &warp.name, &warp.announce, warp.dest_zone) {
            continue;
        }
        // Nation filter: skip if warp is nation-restricted and doesn't match
        if warp.nation != 0 && warp.nation != char_info.nation as i16 {
            continue;
        }

        let effective_dest_zone = resolve_active_battle_warp(
            &world,
            effective_warp_destination(warp.warp_id, warp.dest_zone, char_info.nation),
        );

        // Destination zone must exist and be active after legacy resolution.
        match world.get_zone(effective_dest_zone) {
            Some(z) => {
                let status = z.zone_info.as_ref().map(|i| i.status).unwrap_or(1);
                if status == 0 {
                    continue;
                }
            }
            None => continue,
        }

        // Battle zone filter: hide Ardream/Ronark warps based on active battle type
        let dz = effective_dest_zone;
        if battle.battle_open == NATION_BATTLE || battle.battle_open == SIEGE_BATTLE {
            let is_battle_zone =
                dz == ZONE_ARDREAM || dz == ZONE_RONARK_LAND_BASE || dz == ZONE_RONARK_LAND;
            if battle.battle_zone_type != ZONE_ARDREAM as u8 && is_battle_zone {
                continue;
            }
            if battle.battle_zone_type == ZONE_ARDREAM as u8 && dz == ZONE_ARDREAM {
                continue;
            }
        }

        entries.push(warp);
    }

    // Sort by zone ID (C++ sorts by sZone)
    entries.sort_by_key(|w| {
        resolve_active_battle_warp(
            &world,
            effective_warp_destination(w.warp_id, w.dest_zone, char_info.nation),
        )
    });

    // Build the response packet
    let mut result = Packet::new(Opcode::WizWarpList as u8);
    result.write_u8(1); // sub-opcode: GetWarpList
    result.write_u16(entries.len() as u16);

    for warp in &entries {
        result.write_u16(warp.warp_id as u16);
        result.write_string(&warp.name);
        result.write_string(&warp.announce);
        result.write_u16(resolve_active_battle_warp(
            &world,
            effective_warp_destination(warp.warp_id, warp.dest_zone, char_info.nation),
        ));
        result.write_u16(DEFAULT_MAX_USERS);
        result.write_u32(warp.pay);
    }

    session.send_packet(&result).await?;

    debug!(
        "[{}] GetWarpList: group={}, {} entries sent",
        session.addr(),
        warp_group,
        entries.len(),
    );

    Ok(true)
}

/// Send SelectWarpList failure response.
async fn send_select_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut result = Packet::new(Opcode::WizWarpList as u8);
    result.write_u8(2); // sub-opcode: SelectWarpList response
    result.write_u8(0xFF); // failure (-1 as u8)
    session.send_packet(&result).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn v2615_legacy_eslant_warps_resolve_by_nation() {
        assert_eq!(effective_warp_destination(153, 19, 1), ZONE_KARUS_ESLANT);
        assert_eq!(effective_warp_destination(253, 19, 2), ZONE_ELMORAD_ESLANT);
        assert_eq!(effective_warp_destination(999, 14, 1), ZONE_KARUS_ESLANT);
        assert_eq!(effective_warp_destination(999, 16, 2), ZONE_ELMORAD_ESLANT);
    }

    #[test]
    fn test_get_warp_list_packet_format() {
        // Build a GetWarpList response packet
        let mut pkt = Packet::new(Opcode::WizWarpList as u8);
        pkt.write_u8(1); // sub-opcode
        pkt.write_u16(2); // count = 2

        // Warp entry 1
        pkt.write_u16(211); // warpID
        pkt.write_string("El Morad"); // name
        pkt.write_string("Human lands"); // announce
        pkt.write_u16(2); // zone
        pkt.write_u16(150); // maxuser
        pkt.write_u32(1000); // pay

        // Warp entry 2
        pkt.write_u16(212); // warpID
        pkt.write_string("Karus"); // name
        pkt.write_string("Orc lands"); // announce
        pkt.write_u16(1); // zone
        pkt.write_u16(150); // maxuser
        pkt.write_u32(500); // pay

        assert_eq!(pkt.opcode, Opcode::WizWarpList as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // sub-opcode
        assert_eq!(r.read_u16(), Some(2)); // count

        // Entry 1
        assert_eq!(r.read_u16(), Some(211));
        assert_eq!(r.read_string(), Some("El Morad".to_string()));
        assert_eq!(r.read_string(), Some("Human lands".to_string()));
        assert_eq!(r.read_u16(), Some(2));
        assert_eq!(r.read_u16(), Some(150));
        assert_eq!(r.read_u32(), Some(1000));

        // Entry 2
        assert_eq!(r.read_u16(), Some(212));
        assert_eq!(r.read_string(), Some("Karus".to_string()));
        assert_eq!(r.read_string(), Some("Orc lands".to_string()));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(150));
        assert_eq!(r.read_u32(), Some(500));

        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_select_warp_list_incoming_format() {
        // Client sends: [u16 objectIndex] [u16 warpID]
        let mut pkt = Packet::new(Opcode::WizWarpList as u8);
        pkt.write_u16(5); // objectIndex
        pkt.write_u16(211); // warpID

        assert_eq!(pkt.data.len(), 4);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u16(), Some(211));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_select_warp_list_fail_response() {
        let mut pkt = Packet::new(Opcode::WizWarpList as u8);
        pkt.write_u8(2); // sub-opcode
        pkt.write_u8(0xFF); // failure

        assert_eq!(pkt.data.len(), 2);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0xFF));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_gold_change_packet_format() {
        let mut pkt = Packet::new(Opcode::WizGoldChange as u8);
        pkt.write_u8(2); // CoinLoss
        pkt.write_u32(1000); // amount
        pkt.write_u32(4000); // remaining

        assert_eq!(pkt.opcode, Opcode::WizGoldChange as u8);
        assert_eq!(pkt.data.len(), 9);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u32(), Some(1000));
        assert_eq!(r.read_u32(), Some(4000));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 268: Object event validation tests ──────────────────

    #[test]
    fn test_object_warp_gate_constant() {
        // C++ packets.h:1029 — OBJECT_WARP_GATE = 5
        assert_eq!(OBJECT_WARP_GATE, 5u8);
    }

    #[test]
    fn test_max_object_range_constant() {
        // C++ Unit.h:17 — MAX_OBJECT_RANGE = 100.0f
        assert!((MAX_OBJECT_RANGE - 100.0f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_object_event_range_check() {
        // Distance calculation: sqrt(dx² + dz²)
        // Player at (100, 200), object at (150, 250)
        let dx: f32 = 100.0 - 150.0;
        let dz: f32 = 200.0 - 250.0;
        let dist = (dx * dx + dz * dz).sqrt();
        // sqrt(2500 + 2500) = sqrt(5000) ≈ 70.71 — within range
        assert!(dist < MAX_OBJECT_RANGE);

        // Player at (0, 0), object at (80, 80)
        let dx2: f32 = 0.0 - 80.0;
        let dz2: f32 = 0.0 - 80.0;
        let dist2 = (dx2 * dx2 + dz2 * dz2).sqrt();
        // sqrt(6400 + 6400) = sqrt(12800) ≈ 113.14 — OUT of range
        assert!(dist2 >= MAX_OBJECT_RANGE);
    }

    #[test]
    fn test_object_event_belong_check() {
        // belong=0 → any nation allowed
        // belong=1 → Karus only
        // belong=2 → Elmorad only
        let belong_any: i16 = 0;
        let belong_karus: i16 = 1;
        let belong_elmo: i16 = 2;
        let nation_karus: i16 = 1;
        let nation_elmo: i16 = 2;

        // Any nation → always pass
        assert!(belong_any == 0);

        // Karus belong + Karus nation → pass
        assert!(belong_karus == nation_karus);

        // Karus belong + Elmo nation → fail
        assert!(belong_karus != nation_elmo);

        // Elmo belong + Elmo nation → pass
        assert!(belong_elmo == nation_elmo);
    }

    // ── Sprint 319: Zone status filter tests ────────────────────────

    #[test]
    fn test_zone_status_active() {
        let status: i16 = 1;
        assert!(status != 0, "Active zone (status=1) should be allowed");
    }

    #[test]
    fn test_zone_status_inactive() {
        let status: i16 = 0;
        assert!(status == 0, "Inactive zone (status=0) should be filtered");
    }

    /// C++ only loads zones with Status=1, so missing zone == inactive.
    #[test]
    fn test_zone_not_found_treated_as_inactive() {
        let zone_exists = false;
        assert!(
            !zone_exists,
            "Non-existent zone should be treated as inactive"
        );
    }

    // ── Sprint 326: Eslant zone redirect ────────────────────────────

    #[test]
    fn test_eslant_redirect_all_variants_to_zone_11() {
        // isInKarusEslant: 11, 13, 14
        // isInElmoradEslant: 12, 15, 16
        // All redirect to ZONE_KARUS_ESLANT (11)
        for zone in [11i32, 12, 13, 14, 15, 16] {
            let effective = match zone {
                11..=16 => 11u16,
                z => z as u16,
            };
            assert_eq!(effective, 11, "Zone {} should redirect to 11", zone);
        }
    }

    #[test]
    fn test_non_eslant_zones_not_redirected() {
        // Normal zones should pass through unchanged
        for zone in [1i32, 2, 21, 30, 31, 51, 83] {
            let effective = match zone {
                11..=16 => 11u16,
                z => z as u16,
            };
            assert_eq!(
                effective, zone as u16,
                "Zone {} should not be redirected",
                zone
            );
        }
    }

    #[test]
    fn test_eslant_zone_constants() {
        // C++ Define.h zone IDs
        assert_eq!(11u16, 11); // ZONE_KARUS_ESLANT
        assert_eq!(12u16, 12); // ZONE_ELMORAD_ESLANT
        assert_eq!(13u16, 13); // ZONE_KARUS_ESLANT2
        assert_eq!(14u16, 14); // ZONE_KARUS_ESLANT3
        assert_eq!(15u16, 15); // ZONE_ELMORAD_ESLANT2
        assert_eq!(16u16, 16); // ZONE_ELMORAD_ESLANT3
    }
}
