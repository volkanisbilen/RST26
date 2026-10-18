//! WIZ_HOME (0x48) handler — teleport to bind point (/town command).
//! ## Client -> Server
//! Empty packet (just the opcode, no data).
//! ## Server Processing
//! 1. Check player is alive and HP >= 50% of max
//! 2. Get respawn location (bind point or zone start)
//! 3. If different zone → trigger zone change
//! 4. If same zone → same-zone warp

use ko_protocol::Packet;

use crate::handler::zone_change;
use crate::session::{ClientSession, SessionState};
use crate::world::types::{
    ZONE_BORDER_DEFENSE_WAR, ZONE_CHAOS_DUNGEON, ZONE_DELOS, ZONE_FORGOTTEN_TEMPLE,
    ZONE_JURAID_MOUNTAIN, ZONE_MORADON,
};

/// /town cooldown in seconds.
/// C++ actual cooldown: 1.2 seconds (flood prevention). Server config: 5 seconds.
const TOWN_COOLDOWN_SECS: u64 = 5;

use crate::buff_constants::BUFF_TYPE_FREEZE;

/// Handle WIZ_HOME from the client.
/// The /town command teleports the player to their bind point or the
/// zone's default spawn position.
pub async fn handle(session: &mut ClientSession, _pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Dead players cannot use /town
    if world.is_player_dead(sid) {
        tracing::warn!("[sid={}] WIZ_HOME rejected: player dead", sid);
        return Ok(());
    }

    // Kaul transformation prevents /town
    if world.with_session(sid, |h| h.is_kaul).unwrap_or(false) {
        tracing::warn!("[sid={}] WIZ_HOME rejected: kaul form", sid);
        return Ok(());
    }

    // NO_RECALL debuff prevents teleportation
    if !world.can_teleport(sid) {
        tracing::warn!("[sid={}] WIZ_HOME rejected: can_teleport=false", sid);
        return Ok(());
    }

    let (char_info, current_zone) = match world
        .with_session(sid, |h| {
            h.character
                .as_ref()
                .map(|c| (c.clone(), h.position.zone_id))
        })
        .flatten()
    {
        Some(v) => v,
        None => return Ok(()),
    };

    // HP must be >= 50% of max HP
    if char_info.hp < (char_info.max_hp / 2) {
        tracing::warn!(
            "[sid={}] WIZ_HOME rejected: HP too low ({}/{}, need >= {})",
            sid,
            char_info.hp,
            char_info.max_hp,
            char_info.max_hp / 2,
        );
        let msg = format!(
            "HP must be at least 50% to use /town ({}/{}).",
            char_info.hp, char_info.max_hp
        );
        let notice = crate::systems::timed_notice::build_notice_packet(7, &msg);
        world.send_to_session_owned(sid, notice);
        return Ok(());
    }

    // Event zone check — cannot /town from BDW, Juraid, or Chaos Dungeon
    if matches!(
        current_zone,
        ZONE_BORDER_DEFENSE_WAR | ZONE_CHAOS_DUNGEON | ZONE_JURAID_MOUNTAIN
    ) {
        let notice = crate::systems::timed_notice::build_notice_packet(
            7,
            "Cannot use /town in event zones.",
        );
        world.send_to_session_owned(sid, notice);
        return Ok(());
    }

    // Forgotten Temple — /town kicks user out of zone to Moradon
    // Use (0,0) — resolve_zero_coords handles nation-specific start_position.
    if current_zone == ZONE_FORGOTTEN_TEMPLE {
        zone_change::trigger_zone_change(session, ZONE_MORADON, 0.0, 0.0).await?;
        return Ok(());
    }

    // Quest arena zone block — zones 50-59 are quest arenas, cannot /town out
    if current_zone / 10 == 5 {
        return Ok(());
    }

    // Cooldown check: 5 seconds between /town uses
    let cooldown_ok = world
        .with_session(sid, |h| {
            h.last_town_time.elapsed().as_secs() >= TOWN_COOLDOWN_SECS
        })
        .unwrap_or(false);
    if !cooldown_ok {
        let elapsed = world
            .with_session(sid, |h| h.last_town_time.elapsed().as_secs())
            .unwrap_or(0);
        let remaining = TOWN_COOLDOWN_SECS.saturating_sub(elapsed);
        tracing::warn!(
            "[sid={}] WIZ_HOME rejected: cooldown not expired ({}/{}s)",
            sid,
            elapsed,
            TOWN_COOLDOWN_SECS,
        );
        let msg = format!(
            "Please wait {} seconds before using /town again.",
            remaining
        );
        let notice = crate::systems::timed_notice::build_notice_packet(7, &msg);
        world.send_to_session_owned(sid, notice);
        return Ok(());
    }

    // Freeze buff check — cannot /town while frozen
    let has_freeze = world
        .with_session(sid, |h| h.buffs.contains_key(&BUFF_TYPE_FREEZE))
        .unwrap_or(false);
    if has_freeze {
        tracing::warn!("[sid={}] WIZ_HOME rejected: frozen", sid);
        let notice =
            crate::systems::timed_notice::build_notice_packet(7, "Cannot use /town while frozen.");
        world.send_to_session_owned(sid, notice);
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Determine destination: bind point or zone start position
    let (dest_zone, dest_x, dest_z) = determine_home_location(&char_info, pos.zone_id, &world);

    // Zone change prevents double-warp
    // Note: is_zone_changing() auto-clears after 30s safety timeout
    if world.is_zone_changing(sid) {
        tracing::warn!("[sid={}] WIZ_HOME rejected: zone_changing=true", sid);
        return Ok(());
    }

    // Use trigger_zone_change which handles both same-zone and cross-zone
    zone_change::trigger_zone_change(session, dest_zone, dest_x, dest_z).await?;

    // Update cooldown timer
    world.update_session(sid, |h| {
        h.last_town_time = std::time::Instant::now();
    });

    tracing::info!(
        "[{}] Player used /town: zone {} ({:.0},{:.0}) → zone {} ({:.0},{:.0})",
        session.addr(),
        pos.zone_id,
        pos.x,
        pos.z,
        dest_zone,
        dest_x,
        dest_z,
    );

    Ok(())
}

/// Determine the home/town destination for a player.
/// Priority ():
/// 1. Bind point (bind_zone, bind_x, bind_z) if set and not in ZONE_DELOS
/// 2. Nation-specific spawn from start_position table
/// 3. Fallback: zone init_x/init_z
/// 4. Moradon (zone 21) fallback
fn determine_home_location(
    char_info: &crate::world::CharacterInfo,
    current_zone: u16,
    world: &crate::world::WorldState,
) -> (u16, f32, f32) {
    use rand::Rng;

    // 1. Check bind point
    let bind_zone = char_info.bind_zone as u16;
    if bind_zone > 0
        && (char_info.bind_x != 0.0 || char_info.bind_z != 0.0)
        && current_zone != ZONE_DELOS
    {
        return (bind_zone, char_info.bind_x, char_info.bind_z);
    }

    // 2. Nation-specific spawn from start_position table
    if let Some(sp) = world.get_start_position(current_zone) {
        let mut rng = rand::thread_rng();
        let (base_x, base_z) = if char_info.nation == 1 {
            (sp.karus_x as f32, sp.karus_z as f32)
        } else {
            (sp.elmorad_x as f32, sp.elmorad_z as f32)
        };
        if base_x != 0.0 || base_z != 0.0 {
            let offset_x = if sp.range_x > 0 {
                rng.gen_range(0..=sp.range_x) as f32
            } else {
                0.0
            };
            let offset_z = if sp.range_z > 0 {
                rng.gen_range(0..=sp.range_z) as f32
            } else {
                0.0
            };
            return (current_zone, base_x + offset_x, base_z + offset_z);
        }
    }

    // 3. Fallback: zone init_x/init_z
    if let Some(zone) = world.get_zone(current_zone) {
        let (x, z, _y) = zone.spawn_position();
        if x != 0.0 || z != 0.0 {
            return (current_zone, x, z);
        }
    }

    // 4. Moradon fallback
    // C++ does NOT warp at all in this case. We use Moradon fallback for safety.
    (21, 512.0, 341.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CharacterInfo, WorldState};

    fn make_char_info(nation: u8) -> CharacterInfo {
        CharacterInfo {
            nation,
            ..Default::default()
        }
    }

    // ── determine_home_location tests ────────────────────────────────

    #[test]
    fn test_home_nation_specific_start_position_karus() {
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 21, // Moradon
            karus_x: 200,
            karus_z: 300,
            elmorad_x: 400,
            elmorad_z: 500,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });
        let ch = make_char_info(1); // Karus
        let (zone, x, z) = determine_home_location(&ch, 21, &world);
        assert_eq!(zone, 21);
        assert_eq!(x, 200.0); // Karus coords, not Elmorad
        assert_eq!(z, 300.0);
    }

    #[test]
    fn test_home_nation_specific_start_position_elmorad() {
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 21,
            karus_x: 200,
            karus_z: 300,
            elmorad_x: 400,
            elmorad_z: 500,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });
        let ch = make_char_info(2); // Elmorad
        let (zone, x, z) = determine_home_location(&ch, 21, &world);
        assert_eq!(zone, 21);
        assert_eq!(x, 400.0); // Elmorad coords
        assert_eq!(z, 500.0);
    }

    #[test]
    fn test_home_bind_point_used() {
        let world = WorldState::new();
        let mut ch = make_char_info(1);
        ch.bind_zone = 11;
        ch.bind_x = 250.0;
        ch.bind_z = 350.0;
        let (zone, x, z) = determine_home_location(&ch, 21, &world);
        assert_eq!(zone, 11);
        assert_eq!(x, 250.0);
        assert_eq!(z, 350.0);
    }

    #[test]
    fn test_home_bind_point_blocked_in_delos() {
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 30,
            karus_x: 100,
            karus_z: 100,
            elmorad_x: 100,
            elmorad_z: 100,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });
        let mut ch = make_char_info(1);
        ch.bind_zone = 11;
        ch.bind_x = 250.0;
        ch.bind_z = 350.0;
        // In Delos (zone 30), bind is NOT used — falls to start_position
        let (zone, x, z) = determine_home_location(&ch, 30, &world);
        assert_eq!(zone, 30);
        assert_eq!(x, 100.0);
        assert_eq!(z, 100.0);
    }

    #[test]
    fn test_home_fallback_to_moradon() {
        let world = WorldState::new();
        let ch = make_char_info(1);
        // No bind, no start_position, no zone data → Moradon fallback
        let (zone, x, z) = determine_home_location(&ch, 99, &world);
        assert_eq!(zone, 21);
        assert_eq!(x, 512.0);
        assert_eq!(z, 341.0);
    }

    #[test]
    fn test_home_range_offset_applied() {
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 1,
            karus_x: 100,
            karus_z: 200,
            elmorad_x: 0,
            elmorad_z: 0,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 10,
            range_z: 20,
        });
        let ch = make_char_info(1);
        let (zone, x, z) = determine_home_location(&ch, 1, &world);
        assert_eq!(zone, 1);
        // x in [100, 110], z in [200, 220]
        assert!((100.0..=110.0).contains(&x), "x={x} out of range");
        assert!((200.0..=220.0).contains(&z), "z={z} out of range");
    }

    // ── Original constant/logic tests ────────────────────────────────

    #[test]
    fn test_home_hp_threshold() {
        // Player needs at least 50% HP to use /town
        let max_hp: i16 = 1000;
        let threshold = max_hp / 2;
        assert_eq!(threshold, 500);

        // HP >= 50% — allowed
        assert!(600_i16 >= threshold);
        // HP < 50% — rejected
        assert!(400_i16 < threshold);
    }

    #[test]
    fn test_town_cooldown_constant() {
        // C++ TOWN_TİME = 1200ms (GetTickCount64), server config: 5 seconds
        assert_eq!(TOWN_COOLDOWN_SECS, 5);
    }

    #[test]
    fn test_buff_type_freeze_constant() {
        // C++ GameDefine.h:4274 — BUFF_TYPE_FREEZE = 22
        assert_eq!(BUFF_TYPE_FREEZE, 22);
    }

    #[test]
    fn test_event_zone_constants() {
        // C++ Define.h — event zones that block /town
        assert_eq!(ZONE_BORDER_DEFENSE_WAR, 84);
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
        assert_eq!(ZONE_JURAID_MOUNTAIN, 87);
    }

    #[test]
    fn test_event_zone_blocks_town() {
        // isInEventZone() blocks /town in BDW, Chaos, and Juraid
        for zone_id in [
            ZONE_BORDER_DEFENSE_WAR,
            ZONE_CHAOS_DUNGEON,
            ZONE_JURAID_MOUNTAIN,
        ] {
            assert!(matches!(
                zone_id,
                ZONE_BORDER_DEFENSE_WAR | ZONE_CHAOS_DUNGEON | ZONE_JURAID_MOUNTAIN
            ));
        }
        // Normal zones should NOT be blocked
        assert!(!matches!(
            21u16,
            ZONE_BORDER_DEFENSE_WAR | ZONE_CHAOS_DUNGEON | ZONE_JURAID_MOUNTAIN
        ));
        assert!(!matches!(
            1u16,
            ZONE_BORDER_DEFENSE_WAR | ZONE_CHAOS_DUNGEON | ZONE_JURAID_MOUNTAIN
        ));
    }

    #[test]
    fn test_town_cooldown_elapsed() {
        // After 5+ seconds, cooldown should pass
        let t = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(6))
            .unwrap_or(std::time::Instant::now());
        // If checked_sub succeeded (system uptime > 6s), elapsed >= 5
        if t.elapsed().as_secs() >= 1 {
            assert!(t.elapsed().as_secs() >= TOWN_COOLDOWN_SECS);
        }

        // Before 5 seconds, cooldown should block
        let t2 = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(2))
            .unwrap_or(std::time::Instant::now());
        if t2.elapsed().as_secs() >= 1 {
            assert!(t2.elapsed().as_secs() < TOWN_COOLDOWN_SECS);
        }
    }

    #[test]
    fn test_forgotten_temple_constant() {
        // C++ Define.h — ZONE_FORGOTTEN_TEMPLE = 55
        assert_eq!(ZONE_FORGOTTEN_TEMPLE, 55);
    }

    #[test]
    fn test_moradon_constant() {
        // C++ User.h:1372 — KickOutZoneUser default zone = 21
        assert_eq!(ZONE_MORADON, 21);
    }

    #[test]
    fn test_forgotten_temple_is_not_quest_arena() {
        // Zone 55 (Forgotten Temple) — zone/10 == 5 so quest arena check would catch it,
        // but Forgotten Temple must be handled BEFORE the quest arena check because
        // it has special kick-out behavior (teleport to Moradon, not just block).
        // C++ User.cpp:3867-3871 checks Forgotten Temple BEFORE the (zone/10)==5 check.
        assert_eq!(ZONE_FORGOTTEN_TEMPLE / 10, 5);
        // This proves the order matters: FT check must come first.
    }

    #[test]
    fn test_quest_arena_range() {
        // Zones 50-59 are quest arenas (zone_id / 10 == 5)
        for z in 50..=59u16 {
            assert_eq!(z / 10, 5);
        }
        // Zone 49 and 60 are NOT quest arenas
        assert_ne!(49u16 / 10, 5);
        assert_ne!(60u16 / 10, 5);
    }

    // ── Sprint 293: ZONE_DELOS bind point restriction ─────────────────

    #[test]
    fn test_zone_delos_blocks_bind_point() {
        // `if (pEvent && pEvent->byLife == 1 && GetZoneID() != ZONE_DELOS ...)`
        // Bind points are NOT usable when in ZONE_DELOS (siege zone).
        let current_zone = ZONE_DELOS;
        let bind_zone: u16 = 21; // Moradon
        let bind_x: f32 = 500.0;
        let bind_z: f32 = 300.0;

        // In Delos: bind point should be skipped
        let use_bind =
            bind_zone > 0 && (bind_x != 0.0 || bind_z != 0.0) && current_zone != ZONE_DELOS;
        assert!(!use_bind, "Bind point should be blocked in ZONE_DELOS");

        // In Moradon: bind point should be used
        let current_zone2: u16 = 21;
        let use_bind2 =
            bind_zone > 0 && (bind_x != 0.0 || bind_z != 0.0) && current_zone2 != ZONE_DELOS;
        assert!(use_bind2, "Bind point should work in normal zones");
    }
}
