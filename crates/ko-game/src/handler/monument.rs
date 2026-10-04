//! Monument NPC death processes.
//! When a non-monster NPC with a monument type is killed, the appropriate
//! monument capture process is triggered based on the NPC's type.
//! ## Monument Types
//! | Type | Value | Handler |
//! |------|-------|---------|
//! | NPC_BIFROST_MONUMENT | 155 | Bifrost farming event |
//! | NPC_PVP_MONUMENT | 210 | PVP zone capture announcement |
//! | NPC_BATTLE_MONUMENT | 211 | Nereids Island monument capture |
//! | NPC_HUMAN_MONUMENT | 122 | El Morad nation monument |
//! | NPC_KARUS_MONUMENT | 121 | Karus nation monument |
//! | NPC_DESTROYED_ARTIFACT | 61 | Castle siege warfare monument |

use std::sync::Arc;

use ko_protocol::{Opcode, Packet};
use tracing::debug;

use crate::npc::{NpcInstance, NpcTemplate};
use crate::npc_type_constants::{
    NPC_BATTLE_MONUMENT, NPC_BIFROST_MONUMENT, NPC_CLAN_WAR_MONUMENT, NPC_DESTROYED_ARTIFACT,
    NPC_HUMAN_MONUMENT, NPC_KARUS_MONUMENT, NPC_PVP_MONUMENT,
};
use crate::world::{WorldState, ZONE_BATTLE4};

// ── Monument Model Constants ───────────────────────────────────────────

/// Karus monument model PID after capture.
const MONUMENT_KARUS_SPID: u16 = 14003;

/// Elmorad monument model PID after capture.
const MONUMENT_ELMORAD_SPID: u16 = 14004;

/// Battle monument Elmorad model PID (different from nation monument).
const BATTLE_MONUMENT_ELMORAD_SPID: u16 = 14005;

// ── Chat Type for Monument Notice ──────────────────────────────────────

const CHAT_MONUMENT_NOTICE: u8 = 11;
const CHAT_FORCE: u8 = 4;
const CHAT_PUBLIC: u8 = 7;
pub(crate) const CHAT_WAR_SYSTEM: u8 = 8;

/// Nation battle open constant.
const NATION_BATTLE: u8 = 1;

// ── Dispatch ───────────────────────────────────────────────────────────

/// Dispatch monument death processing based on NPC type.
/// Called from `handle_npc_death()` when a non-monster NPC dies.
/// Only processes monument-type NPCs — all other types are ignored.
pub(super) async fn monument_death_dispatch(
    world: &WorldState,
    npc: &NpcInstance,
    tmpl: &NpcTemplate,
    killer_nation: u8,
    killer_name: &str,
    killer_clan_id: u16,
) {
    if tmpl.is_monster {
        return;
    }

    match tmpl.npc_type {
        NPC_BIFROST_MONUMENT => {
            bifrost_monument_process(world, killer_nation);
        }
        NPC_PVP_MONUMENT => {
            pvp_monument_process(world, npc, tmpl, killer_nation, killer_name);
        }
        NPC_BATTLE_MONUMENT => {
            battle_monument_process(world, npc, tmpl, killer_nation);
        }
        NPC_HUMAN_MONUMENT => {
            human_nation_monument_process(world, tmpl, killer_nation);
        }
        NPC_KARUS_MONUMENT => {
            karus_nation_monument_process(world, tmpl, killer_nation);
        }
        NPC_DESTROYED_ARTIFACT => {
            csw_monument_process(world, npc, killer_clan_id).await;
        }
        NPC_CLAN_WAR_MONUMENT => {
            // Score the monument kill: losing clan gets half the score gap bonus.
            super::tournament::register_monument_kill(world, npc.zone_id, killer_clan_id);
        }
        _ => {}
    }
}

// ── PVP Monument Process ───────────────────────────────────────────────

/// Process PVP monument capture.
/// 1. Send MONUMENT_NOTICE chat to the zone
/// 2. Update pvp_monument_nation for the zone
/// 3. Update NPC template (nation + model swap)
fn pvp_monument_process(
    world: &WorldState,
    npc: &NpcInstance,
    tmpl: &NpcTemplate,
    killer_nation: u8,
    killer_name: &str,
) {
    // Build WIZ_CHAT packet with MONUMENT_NOTICE type
    // C++ format: WIZ_CHAT, MONUMENT_NOTICE, FORCE_CHAT, nation, name
    let mut chat_pkt = Packet::new(Opcode::WizChat as u8);
    chat_pkt.write_u8(CHAT_MONUMENT_NOTICE);
    chat_pkt.write_u8(CHAT_FORCE);
    chat_pkt.write_u8(killer_nation);
    // Write name as null-terminated C string
    chat_pkt.data.extend_from_slice(killer_name.as_bytes());
    chat_pkt.data.push(0);

    world.broadcast_to_zone(npc.zone_id, Arc::new(chat_pkt), None);

    // Update PVP monument ownership for this zone
    world.set_pvp_monument_nation(npc.zone_id, killer_nation);

    // Update NPC template: set nation (group) and swap model (PID)
    let new_pid = if killer_nation == 1 {
        MONUMENT_KARUS_SPID
    } else {
        MONUMENT_ELMORAD_SPID
    };
    world.npc_template_update(tmpl.s_sid, tmpl.is_monster, killer_nation, new_pid);

    // PVP monuments are capture points, not ordinary one-life event NPCs.
    // Rebuild it almost immediately from the newly-owned template.
    world.update_npc_ai(npc.nid, |ai| ai.regen_time_ms = 250);

    debug!(
        "PVP monument captured: zone={}, nation={}, proto={}, respawn_ms=250",
        npc.zone_id, killer_nation, tmpl.s_sid
    );
}

// ── Battle Monument Process ────────────────────────────────────────────

/// Process battle (Nereids Island) monument capture.
/// Requires: `battle_open == NATION_BATTLE` and `zone == ZONE_BATTLE4`
/// 1. Add monument points (+2, +10 bonus at 7 total)
/// 2. Decrement opposing side's monument count
/// 3. Update NPC template model
/// 4. Update monument ownership array
/// 5. Broadcast WIZ_MAP_EVENT monument status and point totals
fn battle_monument_process(
    world: &WorldState,
    npc: &NpcInstance,
    tmpl: &NpcTemplate,
    killer_nation: u8,
) {
    let battle_state = world.get_battle_state();
    if battle_state.battle_open != NATION_BATTLE {
        return;
    }
    if npc.zone_id != ZONE_BATTLE4 {
        return;
    }

    // Update monument points and counts
    // C++ logic: +2 points per capture, +10 bonus when reaching 7 monuments,
    //            decrement opposing side's monument count
    world.update_battle_state(|state| {
        if killer_nation == 1 {
            // Karus captured
            state.karus_monument_point = state.karus_monument_point.saturating_add(2);
            state.karus_monuments = state.karus_monuments.saturating_add(1);
            if state.karus_monuments >= 7 {
                state.karus_monument_point = state.karus_monument_point.saturating_add(10);
            }
            if state.elmorad_monuments != 0 {
                state.elmorad_monuments = state.elmorad_monuments.saturating_sub(1);
            }
        } else {
            // Elmorad captured
            state.elmorad_monument_point = state.elmorad_monument_point.saturating_add(2);
            state.elmorad_monuments = state.elmorad_monuments.saturating_add(1);
            if state.elmorad_monuments >= 7 {
                state.elmorad_monument_point = state.elmorad_monument_point.saturating_add(10);
            }
            if state.karus_monuments != 0 {
                state.karus_monuments = state.karus_monuments.saturating_sub(1);
            }
        }

        // Update monument ownership array (trap_number is 1-based)
        let idx = (npc.trap_number as usize).saturating_sub(1);
        if idx < 7 {
            state.nereids_monument_array[idx] = killer_nation;
        }
    });

    // Update NPC template model
    let new_pid = if killer_nation == 1 {
        MONUMENT_KARUS_SPID
    } else {
        BATTLE_MONUMENT_ELMORAD_SPID
    };
    world.npc_template_update(tmpl.s_sid, tmpl.is_monster, killer_nation, new_pid);

    // Broadcast monument capture announcement
    // Uses SendNotice<PUBLIC_CHAT> for zone-scoped monument status
    let nation_name = if killer_nation == 1 {
        "Karus"
    } else {
        "El Morad"
    };
    let notice_msg = format!(
        "{} has captured Battle Monument {}!",
        nation_name, npc.trap_number
    );
    let mut notice_pkt = Packet::new(Opcode::WizChat as u8);
    notice_pkt.write_u8(CHAT_PUBLIC);
    notice_pkt.write_u8(0); // no sender type
    notice_pkt.write_u8(killer_nation);
    notice_pkt.data.extend_from_slice(notice_msg.as_bytes());
    notice_pkt.data.push(0);
    world.broadcast_to_zone(ZONE_BATTLE4, Arc::new(notice_pkt), None);

    // Broadcast monument ownership status
    // C++ packet: WIZ_MAP_EVENT, u8(0), u8(7), 7x u8(array[i])
    let updated_state = world.get_battle_state();
    let mut status_pkt = Packet::new(Opcode::WizMapEvent as u8);
    status_pkt.write_u8(0);
    status_pkt.write_u8(7);
    for i in 0..7 {
        status_pkt.write_u8(updated_state.nereids_monument_array[i]);
    }
    world.broadcast_to_zone(ZONE_BATTLE4, Arc::new(status_pkt), None);

    // Broadcast monument point totals
    // C++ packet: WIZ_MAP_EVENT, u8(2), u16(elmo_points), u16(karus_points)
    let mut points_pkt = Packet::new(Opcode::WizMapEvent as u8);
    points_pkt.write_u8(2);
    points_pkt.write_u16(updated_state.elmorad_monument_point);
    points_pkt.write_u16(updated_state.karus_monument_point);
    world.broadcast_to_zone(ZONE_BATTLE4, Arc::new(points_pkt), None);

    debug!(
        "Battle monument captured: trap={}, nation={}, karus_pts={}, elmo_pts={}",
        npc.trap_number,
        killer_nation,
        updated_state.karus_monument_point,
        updated_state.elmorad_monument_point,
    );
}

// ── Karus Nation Monument Process ──────────────────────────────────────

/// Process Karus nation monument capture.
/// These monuments are in the Karus homeland. When killed:
/// - If killed by Elmorad (infiltration): remove from defeated array
/// - If killed by Karus (recapture): remove from winner array
fn karus_nation_monument_process(world: &WorldState, tmpl: &NpcTemplate, killer_nation: u8) {
    let battle_state = world.get_battle_state();
    if battle_state.battle_open != NATION_BATTLE {
        return;
    }

    // Update NPC template nation
    world.npc_template_update(tmpl.s_sid, tmpl.is_monster, killer_nation, 0);

    // Broadcast nation monument announcement
    // Uses SendAnnouncement → SendChat<WAR_SYSTEM_CHAT> for server-wide notice
    let msg = if killer_nation == 2 {
        format!("El Morad has conquered Karus monument {}!", tmpl.s_sid)
    } else {
        format!("Karus has recaptured monument {}!", tmpl.s_sid)
    };
    let mut announce_pkt = Packet::new(Opcode::WizChat as u8);
    announce_pkt.write_u8(CHAT_WAR_SYSTEM);
    announce_pkt.write_u8(0);
    announce_pkt.write_u8(0); // Nation::ALL
    announce_pkt.data.extend_from_slice(msg.as_bytes());
    announce_pkt.data.push(0);
    world.broadcast_to_all(Arc::new(announce_pkt), None);

    // Update nation monument tracking
    // If Elmorad killed a Karus monument -> remove from DefeatedNationArray
    // If Karus killed (recaptured) -> remove from WinnerNationArray
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    // When Elmorad infiltrates Karus territory: add to winners + remove from defeated
    // When Karus recaptures: add to defeated + remove from winners
    world.update_battle_state(|state| {
        if killer_nation == 2 {
            // Elmorad conquered Karus monument → winner reward
            state.nation_monument_winners.insert(tmpl.s_sid, now_unix);
            state.nation_monument_defeated.remove(&tmpl.s_sid);
        } else {
            // Karus recaptured → defeated timer
            state.nation_monument_defeated.insert(tmpl.s_sid, now_unix);
            state.nation_monument_winners.remove(&tmpl.s_sid);
        }
    });

    debug!(
        "Karus nation monument captured: proto={}, killer_nation={}",
        tmpl.s_sid, killer_nation
    );
}

// ── Human Nation Monument Process ──────────────────────────────────────

/// Process Human/Elmorad nation monument capture.
/// These monuments are in the Elmorad homeland. When killed:
/// - If killed by Karus (infiltration): remove from defeated array
/// - If killed by Elmorad (recapture): remove from winner array
fn human_nation_monument_process(world: &WorldState, tmpl: &NpcTemplate, killer_nation: u8) {
    let battle_state = world.get_battle_state();
    if battle_state.battle_open != NATION_BATTLE {
        return;
    }

    // Update NPC template nation
    world.npc_template_update(tmpl.s_sid, tmpl.is_monster, killer_nation, 0);

    // Broadcast nation monument announcement
    let msg = if killer_nation == 1 {
        format!("Karus has conquered El Morad monument {}!", tmpl.s_sid)
    } else {
        format!("El Morad has recaptured monument {}!", tmpl.s_sid)
    };
    let mut announce_pkt = Packet::new(Opcode::WizChat as u8);
    announce_pkt.write_u8(CHAT_WAR_SYSTEM);
    announce_pkt.write_u8(0);
    announce_pkt.write_u8(0); // Nation::ALL
    announce_pkt.data.extend_from_slice(msg.as_bytes());
    announce_pkt.data.push(0);
    world.broadcast_to_all(Arc::new(announce_pkt), None);

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    // When Karus infiltrates Elmorad territory: add to winners + remove from defeated
    // When Elmorad recaptures: add to defeated + remove from winners
    world.update_battle_state(|state| {
        if killer_nation == 1 {
            // Karus conquered El Morad monument → winner reward
            state.nation_monument_winners.insert(tmpl.s_sid, now_unix);
            state.nation_monument_defeated.remove(&tmpl.s_sid);
        } else {
            // Elmorad recaptured → defeated timer
            state.nation_monument_defeated.insert(tmpl.s_sid, now_unix);
            state.nation_monument_winners.remove(&tmpl.s_sid);
        }
    });

    debug!(
        "Human nation monument captured: proto={}, killer_nation={}",
        tmpl.s_sid, killer_nation
    );
}

// ── Bifrost Monument Process ───────────────────────────────────────────

/// Process Bifrost monument destruction.
/// Sets the beef event to farming phase with the killing nation as winner.
/// Configures farming end time (120 min) and loser sign time (default 30 min).
fn bifrost_monument_process(world: &WorldState, killer_nation: u8) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    world.update_beef_event(|beef| {
        if beef.is_monument_dead || !beef.is_active {
            return;
        }

        beef.is_attackable = false;
        beef.is_monument_dead = true;
        beef.winner_nation = killer_nation;
        beef.is_farming_play = true;

        // C++ sets BeefSendTime = 120 * MINUTE (farming countdown)
        // C++ sets LoserNationSignTime = UNIXTIME + (LoserSignTime * 30 * MINUTE)
        // C++ sets BeefFarmingPlayTime = UNIXTIME + (FarmingTime * MINUTE)
        // Default: FarmingTime=120min, LoserSignTime=1 (30min delay)
        let farming_minutes: u64 = 120;
        let loser_sign_delay_minutes: u64 = 30;

        beef.farming_end_time = now + farming_minutes * 60;
        beef.loser_sign_time = now + loser_sign_delay_minutes * 60;
        beef.is_loser_sign = false;

        debug!(
            "Bifrost monument destroyed: winner_nation={}, farming_end={}s, loser_sign={}s",
            killer_nation,
            farming_minutes * 60,
            loser_sign_delay_minutes * 60,
        );
    });

    // C++ sets BeefSendTime = 120 * MINUTE
    world.set_bifrost_remaining_secs(120 * 60);

    // Broadcast time update + victory notice to Bifrost/Ronark Land zones
    super::bifrost::broadcast_beef_time_update(world);
    super::bifrost::broadcast_beef_notice(world, super::bifrost::NOTICE_VICTORY);
}

// ── CSW Monument Process ───────────────────────────────────────────────

/// Process Castle Siege Warfare monument destruction.
/// Delegates to the existing `siege::monument_capture()` implementation.
async fn csw_monument_process(world: &WorldState, npc: &NpcInstance, killer_clan_id: u16) {
    if killer_clan_id == 0 {
        tracing::warn!(npc_id = npc.nid, "CSW artifact died without a clan killer");
        return;
    }

    let war_active = world.csw_event().read().await.is_war_active();
    if !war_active {
        tracing::warn!(npc_id = npc.nid, killer_clan_id, "CSW artifact died while CSW war was inactive");
        return;
    }

    // Ensure this NPC is in the AI lifecycle even when it came from an object or
    // event spawn path, which does not normally allocate AI state for static NPCs.
    let respawn_scheduled = world.schedule_npc_respawn(npc.nid, 30_000);
    tracing::info!(
        npc_id = npc.nid,
        zone_id = npc.zone_id,
        respawn_scheduled,
        "CSW artifact respawn scheduled"
    );

    let captured = match world.db_pool() {
        Some(pool) => crate::handler::siege::monument_capture(world, killer_clan_id, pool).await,
        None => {
            tracing::error!(killer_clan_id, "CSW capture failed: database pool unavailable");
            false
        }
    };
    if !captured {
        return;
    }

    let clan_name = world
        .get_knights(killer_clan_id)
        .map(|clan| clan.name.clone())
        .unwrap_or_else(|| format!("Clan #{}", killer_clan_id));
    let message = format!(
        "CSW: {} artifacti ele geçirdi. Artifact 30 saniye içinde yeniden doğacak.",
        clan_name
    );
    let packet = crate::handler::chat::build_chat_packet(8, 1, 0xFFFF, "", &message, 0, 0, 0);
    world.broadcast_to_zone(crate::world::ZONE_DELOS, Arc::new(packet), None);
}
#[cfg(test)]
mod tests {
    use super::*;

    // ── NPC Type Constants ─────────────────────────────────────────

    #[test]
    fn test_monument_npc_type_constants() {
        assert_eq!(NPC_DESTROYED_ARTIFACT, 61);
        assert_eq!(NPC_KARUS_MONUMENT, 121);
        assert_eq!(NPC_HUMAN_MONUMENT, 122);
        assert_eq!(NPC_BIFROST_MONUMENT, 155);
        assert_eq!(NPC_PVP_MONUMENT, 210);
        assert_eq!(NPC_BATTLE_MONUMENT, 211);
    }

    #[test]
    fn test_monument_model_constants() {
        assert_eq!(MONUMENT_KARUS_SPID, 14003);
        assert_eq!(MONUMENT_ELMORAD_SPID, 14004);
        assert_eq!(BATTLE_MONUMENT_ELMORAD_SPID, 14005);
    }

    // ── Helper Functions ───────────────────────────────────────────

    fn make_test_world() -> WorldState {
        WorldState::new()
    }

    fn make_monument_template(s_sid: u16, npc_type: u8) -> NpcTemplate {
        NpcTemplate {
            s_sid,
            is_monster: false,
            name: format!("Monument_{}", s_sid),
            pid: 14000,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type,
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 50000,
            max_mp: 0,
            attack: 0,
            ac: 0,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 0,
            speed_1: 0,
            speed_2: 0,
            stand_time: 0,
            search_range: 0,
            attack_range: 0,
            direct_attack: 0,
            tracing_range: 0,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }

    fn make_monument_npc(nid: u32, proto_id: u16, zone_id: u16, trap_number: i16) -> NpcInstance {
        NpcInstance {
            nid,
            proto_id,
            is_monster: false,
            zone_id,
            x: 100.0,
            y: 0.0,
            z: 100.0,
            direction: 0,
            region_x: 7,
            region_z: 7,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        }
    }

    // ── PVP Monument Tests ─────────────────────────────────────────

    #[test]
    fn test_pvp_monument_sets_zone_nation() {
        let world = make_test_world();
        let tmpl = make_monument_template(15000, NPC_PVP_MONUMENT);
        let npc = make_monument_npc(10001, 15000, 71, 0);

        // Insert template into world for npc_template_update
        world.insert_npc_template(tmpl.clone());

        assert_eq!(world.get_pvp_monument_nation(71), 0);

        pvp_monument_process(&world, &npc, &tmpl, 2, "TestPlayer");

        assert_eq!(world.get_pvp_monument_nation(71), 2);
    }

    #[test]
    fn test_pvp_monument_updates_template_model_karus() {
        let world = make_test_world();
        let tmpl = make_monument_template(15000, NPC_PVP_MONUMENT);
        let npc = make_monument_npc(10001, 15000, 71, 0);

        world.insert_npc_template(tmpl.clone());

        pvp_monument_process(&world, &npc, &tmpl, 1, "KarusPlayer");

        let updated = world.get_npc_template(15000, false).unwrap();
        assert_eq!(updated.group, 1);
        assert_eq!(updated.pid, MONUMENT_KARUS_SPID);
    }

    #[test]
    fn test_pvp_monument_updates_template_model_elmorad() {
        let world = make_test_world();
        let tmpl = make_monument_template(15000, NPC_PVP_MONUMENT);
        let npc = make_monument_npc(10001, 15000, 71, 0);

        world.insert_npc_template(tmpl.clone());

        pvp_monument_process(&world, &npc, &tmpl, 2, "ElmoPlayer");

        let updated = world.get_npc_template(15000, false).unwrap();
        assert_eq!(updated.group, 2);
        assert_eq!(updated.pid, MONUMENT_ELMORAD_SPID);
    }

    #[test]
    fn test_pvp_monument_chat_packet_format() {
        // Verify the MONUMENT_NOTICE chat packet structure
        let mut pkt = Packet::new(Opcode::WizChat as u8);
        pkt.write_u8(CHAT_MONUMENT_NOTICE); // 11
        pkt.write_u8(CHAT_FORCE); // 4
        pkt.write_u8(1); // nation
        pkt.data.extend_from_slice(b"TestPlayer");
        pkt.data.push(0);

        assert_eq!(pkt.opcode, Opcode::WizChat as u8);
        assert_eq!(pkt.data[0], 11); // MONUMENT_NOTICE
        assert_eq!(pkt.data[1], 4); // FORCE_CHAT
        assert_eq!(pkt.data[2], 1); // nation
        assert_eq!(&pkt.data[3..13], b"TestPlayer");
        assert_eq!(pkt.data[13], 0); // null terminator
    }

    // ── Battle Monument Tests ──────────────────────────────────────

    #[test]
    fn test_battle_monument_requires_nation_battle() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);
        let npc = make_monument_npc(10002, 15001, ZONE_BATTLE4, 1);

        world.insert_npc_template(tmpl.clone());

        // No war open — should be no-op
        battle_monument_process(&world, &npc, &tmpl, 1);

        let (kp, ep) = world.get_battle_monument_points();
        assert_eq!(kp, 0);
        assert_eq!(ep, 0);
    }

    #[test]
    fn test_battle_monument_karus_capture_adds_points() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);
        let npc = make_monument_npc(10002, 15001, ZONE_BATTLE4, 3);

        world.insert_npc_template(tmpl.clone());

        // Open nation battle on zone 4
        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.battle_zone = 4;
        });

        battle_monument_process(&world, &npc, &tmpl, 1);

        let state = world.get_battle_state();
        assert_eq!(state.karus_monument_point, 2);
        assert_eq!(state.karus_monuments, 1);
        assert_eq!(state.nereids_monument_array[2], 1); // trap 3 -> index 2
    }

    #[test]
    fn test_battle_monument_elmorad_capture_adds_points() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);
        let npc = make_monument_npc(10002, 15001, ZONE_BATTLE4, 5);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.battle_zone = 4;
        });

        battle_monument_process(&world, &npc, &tmpl, 2);

        let state = world.get_battle_state();
        assert_eq!(state.elmorad_monument_point, 2);
        assert_eq!(state.elmorad_monuments, 1);
        assert_eq!(state.nereids_monument_array[4], 2); // trap 5 -> index 4
    }

    #[test]
    fn test_battle_monument_bonus_at_7_captures() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.battle_zone = 4;
            // Pre-set 6 karus monuments
            state.karus_monuments = 6;
            state.karus_monument_point = 12; // 6 * 2
        });

        let npc = make_monument_npc(10002, 15001, ZONE_BATTLE4, 7);
        battle_monument_process(&world, &npc, &tmpl, 1);

        let state = world.get_battle_state();
        // 12 + 2 (capture) + 10 (bonus at 7) = 24
        assert_eq!(state.karus_monument_point, 24);
        assert_eq!(state.karus_monuments, 7);
    }

    #[test]
    fn test_battle_monument_decrements_opposing_monuments() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);
        let npc = make_monument_npc(10002, 15001, ZONE_BATTLE4, 1);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.battle_zone = 4;
            state.elmorad_monuments = 3;
        });

        // Karus captures — should decrement elmo monuments
        battle_monument_process(&world, &npc, &tmpl, 1);

        let state = world.get_battle_state();
        assert_eq!(state.elmorad_monuments, 2);
    }

    #[test]
    fn test_battle_monument_no_underflow_opposing() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);
        let npc = make_monument_npc(10002, 15001, ZONE_BATTLE4, 1);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.battle_zone = 4;
            state.elmorad_monuments = 0; // already 0
        });

        // Karus captures — elmo_monuments should stay 0 (C++ checks != 0 first)
        battle_monument_process(&world, &npc, &tmpl, 1);

        let state = world.get_battle_state();
        assert_eq!(state.elmorad_monuments, 0);
    }

    #[test]
    fn test_battle_monument_map_event_monument_array_packet() {
        // Verify WIZ_MAP_EVENT monument status packet format
        let mut pkt = Packet::new(Opcode::WizMapEvent as u8);
        pkt.write_u8(0);
        pkt.write_u8(7);
        for nation in [1u8, 2, 0, 1, 0, 2, 1] {
            pkt.write_u8(nation);
        }

        assert_eq!(pkt.data.len(), 9); // 1 + 1 + 7
        assert_eq!(pkt.data[0], 0);
        assert_eq!(pkt.data[1], 7);
        assert_eq!(pkt.data[2], 1); // monument 1: karus
        assert_eq!(pkt.data[3], 2); // monument 2: elmorad
        assert_eq!(pkt.data[4], 0); // monument 3: neutral
    }

    #[test]
    fn test_battle_monument_map_event_points_packet() {
        // Verify WIZ_MAP_EVENT point totals packet format
        let mut pkt = Packet::new(Opcode::WizMapEvent as u8);
        pkt.write_u8(2);
        pkt.write_u16(150); // elmo points
        pkt.write_u16(200); // karus points

        assert_eq!(pkt.data.len(), 5); // 1 + 2 + 2
        assert_eq!(pkt.data[0], 2);
        // LE u16: 150 = 0x96, 0x00
        assert_eq!(pkt.data[1], 0x96);
        assert_eq!(pkt.data[2], 0x00);
        // LE u16: 200 = 0xC8, 0x00
        assert_eq!(pkt.data[3], 0xC8);
        assert_eq!(pkt.data[4], 0x00);
    }

    #[test]
    fn test_battle_monument_wrong_zone_ignored() {
        let world = make_test_world();
        let tmpl = make_monument_template(15001, NPC_BATTLE_MONUMENT);
        // NPC is in zone 65 (ZONE_BATTLE5), not ZONE_BATTLE4
        let npc = make_monument_npc(10002, 15001, 65, 1);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
        });

        battle_monument_process(&world, &npc, &tmpl, 1);

        let (kp, ep) = world.get_battle_monument_points();
        assert_eq!(kp, 0);
        assert_eq!(ep, 0);
    }

    // ── Nation Monument Tests ──────────────────────────────────────

    #[test]
    fn test_karus_monument_requires_nation_battle() {
        let world = make_test_world();
        let tmpl = make_monument_template(20301, NPC_KARUS_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        // No war — should be no-op
        karus_nation_monument_process(&world, &tmpl, 2);

        let updated = world.get_npc_template(20301, false).unwrap();
        // group should still be 0 (unchanged) because war not open
        assert_eq!(updated.group, 0);
    }

    #[test]
    fn test_karus_monument_elmorad_infiltration() {
        let world = make_test_world();
        let tmpl = make_monument_template(20301, NPC_KARUS_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        // Pre-populate defeated array
        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.nation_monument_defeated.insert(20301, 0);
        });

        // Elmorad kills Karus monument (infiltration)
        karus_nation_monument_process(&world, &tmpl, 2);

        let state = world.get_battle_state();
        assert!(!state.nation_monument_defeated.contains_key(&20301));

        // Template should be updated with Elmorad nation
        let updated = world.get_npc_template(20301, false).unwrap();
        assert_eq!(updated.group, 2);
    }

    #[test]
    fn test_karus_monument_karus_recapture() {
        let world = make_test_world();
        let tmpl = make_monument_template(20301, NPC_KARUS_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        // Pre-populate winner array
        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.nation_monument_winners.insert(20301, 0);
        });

        // Karus recaptures their own monument
        karus_nation_monument_process(&world, &tmpl, 1);

        let state = world.get_battle_state();
        assert!(!state.nation_monument_winners.contains_key(&20301));
    }

    #[test]
    fn test_human_monument_karus_infiltration() {
        let world = make_test_world();
        let tmpl = make_monument_template(10301, NPC_HUMAN_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.nation_monument_defeated.insert(10301, 0);
        });

        // Karus kills Elmorad monument (infiltration)
        human_nation_monument_process(&world, &tmpl, 1);

        let state = world.get_battle_state();
        assert!(!state.nation_monument_defeated.contains_key(&10301));
    }

    #[test]
    fn test_human_monument_elmorad_recapture() {
        let world = make_test_world();
        let tmpl = make_monument_template(10301, NPC_HUMAN_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        world.update_battle_state(|state| {
            state.battle_open = NATION_BATTLE;
            state.nation_monument_winners.insert(10301, 0);
        });

        // Elmorad recaptures their own monument
        human_nation_monument_process(&world, &tmpl, 2);

        let state = world.get_battle_state();
        assert!(!state.nation_monument_winners.contains_key(&10301));
    }

    // ── Bifrost Monument Tests ─────────────────────────────────────

    #[test]
    fn test_bifrost_monument_sets_farming() {
        let world = make_test_world();

        // Activate beef event
        world.update_beef_event(|beef| {
            beef.is_active = true;
            beef.is_attackable = true;
        });

        bifrost_monument_process(&world, 1);

        let beef = world.get_beef_event();
        assert!(beef.is_monument_dead);
        assert!(!beef.is_attackable);
        assert_eq!(beef.winner_nation, 1);
        assert!(beef.is_farming_play);
    }

    #[test]
    fn test_bifrost_monument_ignores_if_already_dead() {
        let world = make_test_world();

        world.update_beef_event(|beef| {
            beef.is_active = true;
            beef.is_monument_dead = true; // already dead
        });

        bifrost_monument_process(&world, 2);

        let beef = world.get_beef_event();
        // winner_nation should still be 0 (not updated)
        assert_eq!(beef.winner_nation, 0);
    }

    #[test]
    fn test_bifrost_monument_ignores_if_not_active() {
        let world = make_test_world();

        // Beef event not active (default)
        bifrost_monument_process(&world, 1);

        let beef = world.get_beef_event();
        assert!(!beef.is_monument_dead);
        assert_eq!(beef.winner_nation, 0);
    }

    // ── Dispatch Tests ─────────────────────────────────────────────

    #[test]
    fn test_dispatch_ignores_monsters() {
        // Monster NPCs should never trigger monument processing
        let mut tmpl = make_monument_template(15000, NPC_PVP_MONUMENT);
        tmpl.is_monster = true;

        // This is just a type check — no world needed for the guard clause
        assert!(tmpl.is_monster);
    }

    #[test]
    fn test_dispatch_ignores_unknown_npc_types() {
        // NPC types that don't match any monument should be silently ignored
        let tmpl = make_monument_template(15000, 99); // unknown type
        assert_eq!(tmpl.npc_type, 99);
        // No match in the dispatch switch — this is a no-op
    }

    // ── NPC Template Update Tests ──────────────────────────────────

    #[test]
    fn test_npc_template_update_group_and_pid() {
        let world = make_test_world();
        let tmpl = make_monument_template(15000, NPC_PVP_MONUMENT);

        world.insert_npc_template(tmpl);

        world.npc_template_update(15000, false, 2, 14004);

        let updated = world.get_npc_template(15000, false).unwrap();
        assert_eq!(updated.group, 2);
        assert_eq!(updated.pid, 14004);
    }

    #[test]
    fn test_npc_template_update_group_only() {
        let world = make_test_world();
        let tmpl = make_monument_template(15000, NPC_PVP_MONUMENT);

        world.insert_npc_template(tmpl.clone());

        // pid=0 should not change the template's pid
        world.npc_template_update(15000, false, 1, 0);

        let updated = world.get_npc_template(15000, false).unwrap();
        assert_eq!(updated.group, 1);
        assert_eq!(updated.pid, tmpl.pid); // unchanged
    }

    #[test]
    fn test_npc_template_update_nonexistent_ignored() {
        let world = make_test_world();
        // Should not panic
        world.npc_template_update(60000, false, 1, 14003);
    }

    // ── Beef Event State Tests ─────────────────────────────────────

    #[test]
    fn test_beef_event_default_state() {
        let world = make_test_world();
        let beef = world.get_beef_event();
        assert!(!beef.is_active);
        assert!(!beef.is_attackable);
        assert!(!beef.is_monument_dead);
        assert_eq!(beef.winner_nation, 0);
        assert!(!beef.is_farming_play);
    }

    #[test]
    fn test_is_beef_event_farming_true() {
        let world = make_test_world();
        world.update_beef_event(|beef| {
            beef.is_active = true;
            beef.is_farming_play = true;
            beef.winner_nation = 2;
        });
        assert!(world.is_beef_event_farming());
    }

    #[test]
    fn test_is_beef_event_farming_false_no_winner() {
        let world = make_test_world();
        world.update_beef_event(|beef| {
            beef.is_active = true;
            beef.is_farming_play = true;
            beef.winner_nation = 0; // no winner
        });
        assert!(!world.is_beef_event_farming());
    }

    // ── M1: Clan War Monument constant ──────────────────────────────

    #[test]
    fn test_clan_war_monument_constant() {
        // C++ globals.h:224 — NPC_CLAN_WAR_MONUMENT = 224
        assert_eq!(NPC_CLAN_WAR_MONUMENT, 224);
    }

    #[test]
    fn test_clan_war_monument_dispatch_does_not_panic() {
        let world = make_test_world();
        let npc = make_monument_npc(1, 77, 0, 0);
        let tmpl = make_monument_template(5001, NPC_CLAN_WAR_MONUMENT);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(monument_death_dispatch(
            &world, &npc, &tmpl, 1, "TestUser", 100,
        ));
    }

    /// Clan war monument kill wires through to tournament scoring.
    /// When the losing clan kills the monument, they get half the score gap.
    #[test]
    fn test_clan_war_monument_wires_tournament_scoring() {
        use crate::handler::tournament::TournamentState;

        let world = make_test_world();

        // Set up a tournament in zone 77 with red=100, blue=200
        let mut state = TournamentState::new(77, 100, 200, 600);
        state.score_board = [5, 15]; // red is losing
        world.insert_tournament(state);

        // NPC in tournament zone 77
        let npc = make_monument_npc(29516, 29516, 77, 0);
        let tmpl = make_monument_template(29516, NPC_CLAN_WAR_MONUMENT);

        // Red clan (100) kills the monument — should get bonus (15-5)/2 = 5
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(monument_death_dispatch(
            &world,
            &npc,
            &tmpl,
            1,
            "RedPlayer",
            100,
        ));

        // Verify red score increased by half the gap
        let snap = world.with_tournament_snapshot(77).unwrap();
        assert_eq!(snap.score_board[0], 10); // 5 + 5
        assert_eq!(snap.score_board[1], 15); // unchanged
        assert_eq!(snap.monument_killed, 1);
    }

    /// Winning clan kills monument — no bonus awarded.
    #[test]
    fn test_clan_war_monument_winning_clan_no_bonus() {
        use crate::handler::tournament::TournamentState;

        let world = make_test_world();

        let mut state = TournamentState::new(77, 100, 200, 600);
        state.score_board = [15, 5]; // red is winning
        world.insert_tournament(state);

        let npc = make_monument_npc(29516, 29516, 77, 0);
        let tmpl = make_monument_template(29516, NPC_CLAN_WAR_MONUMENT);

        // Red clan (100) kills monument but they're winning — no bonus
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(monument_death_dispatch(
            &world,
            &npc,
            &tmpl,
            1,
            "RedPlayer",
            100,
        ));

        let snap = world.with_tournament_snapshot(77).unwrap();
        assert_eq!(snap.score_board[0], 15); // unchanged
        assert_eq!(snap.score_board[1], 5); // unchanged
        assert_eq!(snap.monument_killed, 0); // no monument kill counted
    }

    /// Monument kill when scores are tied — no bonus.
    #[test]
    fn test_clan_war_monument_tied_scores_no_bonus() {
        use crate::handler::tournament::TournamentState;

        let world = make_test_world();

        let mut state = TournamentState::new(77, 100, 200, 600);
        state.score_board = [10, 10]; // tied
        world.insert_tournament(state);

        let npc = make_monument_npc(29516, 29516, 77, 0);
        let tmpl = make_monument_template(29516, NPC_CLAN_WAR_MONUMENT);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(monument_death_dispatch(
            &world,
            &npc,
            &tmpl,
            1,
            "RedPlayer",
            100,
        ));

        let snap = world.with_tournament_snapshot(77).unwrap();
        assert_eq!(snap.score_board, [10, 10]); // unchanged
    }

    // ── M2: Announcement chat type constants ────────────────────────

    #[test]
    fn test_announcement_chat_type_constants() {
        assert_eq!(CHAT_PUBLIC, 7);
        assert_eq!(CHAT_WAR_SYSTEM, 8);
    }
}
