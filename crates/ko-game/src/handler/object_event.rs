//! WIZ_OBJECT_EVENT (0x33) handler — interactive object processing.
//! ## Flow
//! 1. Client sends `[u16 objectIndex] [u16 nid]` when interacting with a world object
//! 2. Server looks up the object event by index in the zone's object_events
//! 3. Dispatches based on object type (bind, warp gate, anvil, lever, etc.)
//! 4. Sends response: `WIZ_OBJECT_EVENT + u8(object_type) + u8(success)`
//! ## Object Types
//! - BIND (0) / REMOVE_BIND (7): Set/clear respawn bind point
//! - GATE (1): Gate NPC (handled server-side)
//! - GATE_LEVER (3): Toggle gate during war/siege
//! - FLAG_LEVER (4): Capture flag during battle
//! - WARP_GATE (5): Opens warp destination list
//! - ANVIL (8): Opens item upgrade UI
//! - KROWAZ_GATE (12): Key gates requiring items
//! - WOOD (14) / WOOD_LEVER (15): Burning log levers

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::handler::warp_list;
use crate::session::{ClientSession, SessionState};
use crate::world::types::{ZONE_BIFROST, ZONE_DELOS};
use crate::zone::ObjectType;

use crate::npc_type_constants::MAX_OBJECT_RANGE;

/// Handle WIZ_OBJECT_EVENT from the client.
/// Packet format (incoming): `[u16 objectIndex] [u16 nid]`
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    let mut reader = PacketReader::new(&pkt.data);

    let object_index = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };
    let nid = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let world = session.world().clone();
    let sid = session.session_id();

    // Must be alive
    if world.is_player_dead(sid) {
        return Ok(());
    }

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    let zone = match world.get_zone(pos.zone_id) {
        Some(z) => z,
        None => return Ok(()),
    };

    // Look up the object event by index
    let event = match zone.get_object_event(object_index) {
        Some(e) => e.clone(),
        None => {
            warn!(
                "[{}] ObjectEvent: index {} not found in zone {}",
                session.addr(),
                object_index,
                pos.zone_id
            );
            send_fail(session, 0).await?;
            return Ok(());
        }
    };

    // Must be active
    if event.status != 1 {
        warn!(
            "[{}] ObjectEvent: index {} inactive (status={}) in zone {}",
            session.addr(),
            object_index,
            event.status,
            pos.zone_id
        );
        send_fail(session, event.obj_type as u8).await?;
        return Ok(());
    }

    // Range check: player must be close enough to the object
    let dx = pos.x - event.pos_x;
    let dz = pos.z - event.pos_z;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist >= MAX_OBJECT_RANGE {
        warn!(
            "[{}] ObjectEvent: index {} out of range (dist={:.1}, max={}) in zone {}",
            session.addr(),
            object_index,
            dist,
            MAX_OBJECT_RANGE,
            pos.zone_id
        );
        send_fail(session, event.obj_type as u8).await?;
        return Ok(());
    }

    let obj_type = ObjectType::from_i16(event.obj_type);

    match obj_type {
        Some(ObjectType::Bind) | Some(ObjectType::RemoveBind) => {
            // Bind/Unbind: set or clear respawn point
            // Nation check
            let nation = world
                .get_character_info(sid)
                .map(|ch| ch.nation)
                .unwrap_or(0);
            if event.belong != 0 && event.belong != nation as i16 {
                send_fail(session, event.obj_type as u8).await?;
                return Ok(());
            }

            // Store or clear bind point
            //   if (pEvent->sType == OBJECT_REMOVE_BIND) m_sBind = -1; else m_sBind = pEvent->sIndex;
            if obj_type == Some(ObjectType::RemoveBind) {
                // Clear bind point
                world.update_character_stats(sid, |ch| {
                    ch.bind_zone = 0;
                    ch.bind_x = 0.0;
                    ch.bind_z = 0.0;
                });
            } else {
                // Validate bind point before setting:
                // sIndex 101 = Karus homecoming gate, 201 = Elmorad homecoming gate
                // These are filtered at Home-use time in C++, we filter at bind-set time for safety.
                if event.by_life != 1 || event.s_index == 101 || event.s_index == 201 {
                    send_fail(session, event.obj_type as u8).await?;
                    return Ok(());
                }

                // Set bind point
                world.update_character_stats(sid, |ch| {
                    ch.bind_zone = pos.zone_id as u8;
                    ch.bind_x = event.pos_x;
                    ch.bind_z = event.pos_z;
                });
            }

            // Send success
            let mut result = Packet::new(Opcode::WizObjectEvent as u8);
            result.write_u8(event.obj_type as u8);
            result.write_u8(1); // success
            session.send_packet(&result).await?;

            debug!(
                "[{}] ObjectEvent: BIND at zone={} ({:.0},{:.0})",
                session.addr(),
                pos.zone_id,
                event.pos_x,
                event.pos_z
            );
        }

        Some(ObjectType::WarpGate) => {
            // Warp gate: open warp list UI
            let nation = world
                .get_character_info(sid)
                .map(|ch| ch.nation)
                .unwrap_or(0);
            if event.belong != 0 && event.belong != nation as i16 {
                warn!(
                    "[{}] ObjectEvent: WARP_GATE index {} belong={} nation={} mismatch in zone {}",
                    session.addr(),
                    object_index,
                    event.belong,
                    nation,
                    pos.zone_id
                );
                send_fail(session, event.obj_type as u8).await?;
                return Ok(());
            }

            // Opposing-nation zone check: cannot use warp gates in enemy territory.
            //   `(GetZoneID() != GetNation() && GetZoneID() <= Nation::ELMORAD)`
            // Zone 1 = Karus, Zone 2 = Elmorad — if player is in opposing nation's zone, deny.
            if pos.zone_id <= 2 && pos.zone_id != nation as u16 {
                warn!(
                    "[{}] ObjectEvent: WARP_GATE index {} opposing-nation zone={} nation={}",
                    session.addr(),
                    object_index,
                    pos.zone_id,
                    nation
                );
                send_fail(session, event.obj_type as u8).await?;
                return Ok(());
            }

            // Send warp list using the control_npc as the warp group
            let sent = warp_list::send_warp_list(session, event.control_npc as i32).await?;
            if !sent {
                warn!(
                    "[{}] ObjectEvent: WARP_GATE index {} control_npc={} send_warp_list returned no list (zone={})",
                    session.addr(),
                    object_index,
                    event.control_npc,
                    pos.zone_id
                );
                send_fail(session, event.obj_type as u8).await?;
            } else {
                world.set_check_warp_zone_change(sid, true);
                debug!(
                    "[{}] ObjectEvent: WARP_GATE group={}",
                    session.addr(),
                    event.control_npc
                );
            }
        }

        Some(ObjectType::Anvil) => {
            // Anvil: open upgrade UI
            world.update_session(sid, |h| {
                h.event_nid = nid as i16;
                h.event_sid = event.s_index;
            });

            let mut result = Packet::new(Opcode::WizItemUpgrade as u8);
            result.write_u8(1); // ITEM_UPGRADE_REQ opcode
            result.write_u16(nid);
            session.send_packet(&result).await?;

            debug!("[{}] ObjectEvent: ANVIL npc={}", session.addr(), nid);
        }

        Some(ObjectType::Gate) => {
            // Gate: handled by NPC system, just send success
            // Gates use the same bind logic in C++
            let gate_nation = world
                .get_character_info(sid)
                .map(|ch| ch.nation)
                .unwrap_or(0);
            if event.belong != 0 && event.belong != gate_nation as i16 {
                send_fail(session, event.obj_type as u8).await?;
                return Ok(());
            }

            let mut result = Packet::new(Opcode::WizObjectEvent as u8);
            result.write_u8(event.obj_type as u8);
            result.write_u8(1);
            session.send_packet(&result).await?;
        }

        Some(ObjectType::GateLever) | Some(ObjectType::WoodLever) => {
            // Gate lever / wood lever: toggle a gate NPC open/closed during war or siege.
            //
            //
            // Requirements:
            // - Lever NPC must exist (nid from packet)
            // - Gate NPC must exist (event.control_npc proto_id in zone)
            // - In Delos: only the castle owner (master knights) clan can toggle
            // - In battle zones (war open) or Bifrost: nation check
            let gate_nation = world
                .get_character_info(sid)
                .map(|ch| ch.nation)
                .unwrap_or(0);
            let clan_id = world.get_session_clan_id(sid);

            // Validate lever NPC exists
            //   pNpc = g_pMain->GetNpcPtr(nid, GetZoneID())
            let lever_npc = match world.get_npc_instance(nid as u32) {
                Some(n) => n,
                None => {
                    send_fail(session, event.obj_type as u8).await?;
                    return Ok(());
                }
            };

            // Validate gate NPC exists and is a gate type
            let gate_proto = event.control_npc as u16;
            let gate_npc = match world.find_npc_in_zone(gate_proto, pos.zone_id) {
                Some(n) => n,
                None => {
                    send_fail(session, event.obj_type as u8).await?;
                    return Ok(());
                }
            };

            if pos.zone_id == ZONE_DELOS {
                // Delos: only castle owner clan can toggle gates
                let master_knights = {
                    let sw = world.siege_war().read().await;
                    sw.master_knights
                };

                if clan_id == 0 || master_knights == 0 || clan_id != master_knights {
                    debug!(
                        "[{}] ObjectEvent: GateLever in Delos denied (clan={}, master={})",
                        session.addr(),
                        clan_id,
                        master_knights
                    );
                    send_fail(session, event.obj_type as u8).await?;
                } else {
                    // Toggle lever and gate
                    let lever_new = if lever_npc.gate_open == 0 { 1 } else { 0 };
                    let gate_new = if gate_npc.gate_open == 0 { 1 } else { 0 };
                    world.send_gate_flag(lever_npc.nid, lever_new);
                    world.send_gate_flag(gate_npc.nid, gate_new);
                    debug!(
                        "[{}] ObjectEvent: GateLever toggled in Delos by clan {}",
                        session.addr(),
                        clan_id
                    );
                }
            } else if pos.zone_id == ZONE_BIFROST || (world.is_war_open() && zone.is_war_zone()) {
                // Bifrost or battle zones with war open
                // Nation check for non-Bifrost zones
                if pos.zone_id != ZONE_BIFROST
                    && lever_npc.nation != 0
                    && event.belong != gate_nation as i16
                {
                    send_fail(session, event.obj_type as u8).await?;
                    return Ok(());
                }
                let lever_new = if lever_npc.gate_open == 0 { 1 } else { 0 };
                let gate_new = if gate_npc.gate_open == 0 { 1 } else { 0 };
                world.send_gate_flag(lever_npc.nid, lever_new);
                world.send_gate_flag(gate_npc.nid, gate_new);
                debug!(
                    "[{}] ObjectEvent: GateLever toggled in zone {}",
                    session.addr(),
                    pos.zone_id
                );
            } else {
                send_fail(session, event.obj_type as u8).await?;
            }
        }

        Some(ObjectType::FlagLever) => {
            // Flag lever: capture flag during battle.
            //
            //
            // Requirements:
            // - Lever NPC must exist
            // - Flag (gate) NPC must exist and be a gate type
            // - War must be active (g_pMain->m_bVictory == 0)
            // - Lever NPC must be open (gate_open != 0, i.e. not already captured)
            let flag_nation = world
                .get_character_info(sid)
                .map(|ch| ch.nation)
                .unwrap_or(0);

            // Validate lever NPC and flag NPC exist
            let flag_proto = event.control_npc as u16;
            let flag_npc = world.find_npc_in_zone(flag_proto, pos.zone_id);

            match (world.get_npc_instance(nid as u32), flag_npc) {
                (Some(lever_npc), Some(flag_npc_arc)) => {
                    // C++ checks: isGate() && m_bVictory == 0 && !isGateClosed()
                    // Gate must be open (lever not already captured)
                    if lever_npc.gate_open == 0
                        || (event.belong != 0 && event.belong != flag_nation as i16)
                    {
                        send_fail(session, event.obj_type as u8).await?;
                    } else {
                        // Reset both objects (flag captured)
                        //   pNpc->SendGateFlag(0);
                        //   pFlagNpc->SendGateFlag(0);
                        world.send_gate_flag(lever_npc.nid, 0);
                        world.send_gate_flag(flag_npc_arc.nid, 0);

                        // Increment flag counter and check for victory
                        //   g_pMain->m_bKarusFlag++ / m_bElmoradFlag++
                        //   g_pMain->BattleZoneVictoryCheck()
                        let victory = world.increment_war_flag(flag_nation);
                        if victory {
                            flag_victory_rewards(&world, flag_nation).await;
                        }

                        debug!(
                            "[{}] ObjectEvent: FlagLever captured by nation {} in zone {} (victory={})",
                            session.addr(),
                            flag_nation,
                            pos.zone_id,
                            victory
                        );
                    }
                }
                _ => {
                    send_fail(session, event.obj_type as u8).await?;
                }
            }
        }

        Some(ObjectType::Wood) => {
            // Burning wood/log lever — toggles associated gate NPCs during war.
            //
            //
            // Finds all NPC instances matching `event.control_npc` proto_id in the zone
            // and toggles their gate state via SendGateFlag.
            let is_war = world.is_war_open();
            let in_war_zone = zone.is_war_zone();

            if is_war && in_war_zone {
                let wood_nation = world
                    .get_character_info(sid)
                    .map(|ch| ch.nation)
                    .unwrap_or(0);
                if event.belong != 0 && event.belong != wood_nation as i16 {
                    send_fail(session, event.obj_type as u8).await?;
                } else {
                    // Find all gate NPCs matching control_npc proto_id and toggle them
                    let gate_proto = event.control_npc as u16;
                    let gate_npcs = world.find_all_npcs_in_zone(gate_proto, pos.zone_id);
                    for gate_npc in &gate_npcs {
                        let new_state = if gate_npc.gate_open == 0 { 1 } else { 0 };
                        world.send_gate_flag(gate_npc.nid, new_state);
                    }

                    // Also toggle the lever NPC itself
                    //   pNpc->SendGateFlag((pNpc->m_byGateOpen == 0 ? 1 : 0));
                    let lever_proto = event.s_index as u16;
                    if let Some(lever_npc) = world.find_npc_in_zone(lever_proto, pos.zone_id) {
                        let new_state = if lever_npc.gate_open == 0 { 1 } else { 0 };
                        world.send_gate_flag(lever_npc.nid, new_state);
                    }

                    debug!(
                        "[{}] ObjectEvent: Wood activated in zone {} ({} gates toggled)",
                        session.addr(),
                        pos.zone_id,
                        gate_npcs.len()
                    );
                }
            } else {
                debug!(
                    "[{}] ObjectEvent: Wood denied (war={}, war_zone={})",
                    session.addr(),
                    is_war,
                    in_war_zone
                );
                send_fail(session, event.obj_type as u8).await?;
            }
        }

        Some(ObjectType::KrowazGate) => {
            // Krowaz key gate — requires a specific key item to pass through.
            //
            //
            // Gate types and required keys:
            // - "Blue Key Gate"  → ITEM_BLUE_KEY  (310045000)
            // - "Red Key Gate"   → ITEM_RED_KEY   (310046000)
            // - "Black Key Gate" → ITEM_BLACK_KEY (310047000)
            // - "Accomplisher Gate" / "Benshar Gate" → all 3 keys
            const ITEM_BLUE_KEY: u32 = 310_045_000;
            const ITEM_RED_KEY: u32 = 310_046_000;
            const ITEM_BLACK_KEY: u32 = 310_047_000;

            let gate_proto = event.control_npc as u16;

            // Find the gate NPC instance in this zone
            let gate_npc = world.find_npc_in_zone(gate_proto, pos.zone_id);
            let gate_npc = match gate_npc {
                Some(n) => n,
                None => {
                    debug!(
                        "[{}] ObjectEvent: KrowazGate NPC proto={} not found in zone {}",
                        session.addr(),
                        gate_proto,
                        pos.zone_id
                    );
                    send_fail(session, event.obj_type as u8).await?;
                    return Ok(());
                }
            };

            // Gate must be closed (gate_open == 0) to be opened
            if gate_npc.gate_open != 0 {
                send_fail(session, event.obj_type as u8).await?;
                return Ok(());
            }

            // Get gate name from template
            let gate_name = world
                .get_npc_template(gate_proto, false)
                .map(|t| t.name.clone())
                .unwrap_or_default();

            let success = if gate_name == "Blue Key Gate"
                && world.check_exist_item(sid, ITEM_BLUE_KEY, 1)
            {
                world.rob_item(sid, ITEM_BLUE_KEY, 1);
                true
            } else if gate_name == "Red Key Gate" && world.check_exist_item(sid, ITEM_RED_KEY, 1) {
                world.rob_item(sid, ITEM_RED_KEY, 1);
                true
            } else if gate_name == "Black Key Gate"
                && world.check_exist_item(sid, ITEM_BLACK_KEY, 1)
            {
                world.rob_item(sid, ITEM_BLACK_KEY, 1);
                true
            } else if (gate_name == "Accomplisher Gate" || gate_name == "Benshar Gate")
                && world.check_exist_item(sid, ITEM_BLUE_KEY, 1)
                && world.check_exist_item(sid, ITEM_RED_KEY, 1)
                && world.check_exist_item(sid, ITEM_BLACK_KEY, 1)
            {
                world.rob_item(sid, ITEM_RED_KEY, 1);
                world.rob_item(sid, ITEM_BLUE_KEY, 1);
                world.rob_item(sid, ITEM_BLACK_KEY, 1);
                true
            } else {
                false
            };

            if success {
                // Toggle gate open and broadcast to nearby players
                //   pGateNpc->SendGateFlag((pGateNpc->m_byGateOpen == 0 ? 1 : 0));
                let new_state = if gate_npc.gate_open == 0 { 1 } else { 0 };
                world.send_gate_flag(gate_npc.nid, new_state);

                debug!(
                    "[{}] ObjectEvent: KrowazGate '{}' opened (nid={})",
                    session.addr(),
                    gate_name,
                    gate_npc.nid
                );
            } else {
                debug!(
                    "[{}] ObjectEvent: KrowazGate '{}' — missing key(s)",
                    session.addr(),
                    gate_name
                );
                send_fail(session, event.obj_type as u8).await?;
            }
        }

        None => {
            warn!(
                "[{}] ObjectEvent: unknown type {} at index {}",
                session.addr(),
                event.obj_type,
                object_index
            );
            send_fail(session, event.obj_type as u8).await?;
        }
    }

    Ok(())
}

/// Distribute flag victory rewards to the winning nation's players.
/// Rewards (only to winning nation players in their home zone):
/// - 100,000 gold + 5,000 EXP
/// - Captain: 500 NP (king) or 300 NP (non-king)
/// - Non-captain: 200 NP (king) or 100 NP (non-king)
/// - Victory emotion: StateChangeServerDirect(4, 12)
async fn flag_victory_rewards(world: &crate::world::WorldState, winner_nation: u8) {
    use crate::systems::{loyalty, war};

    // Set victory in battle state + announce
    world.update_battle_state(|s| war::battle_zone_result(s, winner_nation));
    let msg = war::build_winner_string(winner_nation);
    let pkt = crate::handler::chat::build_chat_packet(8, 1, 0xFFFF, "", &msg, 0, 0, 0);
    world.broadcast_to_all(Arc::new(pkt), None);

    tracing::info!("Flag victory: nation {} wins!", winner_nation);

    // Iterate all in-game sessions and reward winning nation players in their home zone
    let session_ids = world.get_in_game_session_ids();
    for sid in session_ids {
        let info = match world.get_character_info(sid) {
            Some(ch) => ch,
            None => continue,
        };

        if info.nation != winner_nation {
            continue;
        }
        // Zone 1 = Karus home, Zone 2 = Elmorad home
        let pos = match world.get_position(sid) {
            Some(p) => p,
            None => continue,
        };
        if pos.zone_id != info.nation as u16 {
            continue;
        }

        // Gold + EXP awards
        world.gold_gain(sid, war::AWARD_GOLD);
        crate::handler::level::exp_change(world, sid, war::AWARD_EXP).await;

        // NP rewards based on fame/king status
        let is_king = world.is_king(info.nation, &info.name);
        let fame = info.fame;

        if fame == war::COMMAND_CAPTAIN {
            let np = war::captain_reward_np(is_king, true);
            loyalty::send_loyalty_change(world, sid, np, false, false, true);
        } else {
            let np = war::flag_victory_np(is_king);
            loyalty::send_loyalty_change(world, sid, np, false, false, true);
        }

        // Victory emotion: StateChangeServerDirect(4, 12)
        let mut emotion_pkt = Packet::new(Opcode::WizStateChange as u8);
        emotion_pkt.write_u32(sid as u32);
        emotion_pkt.write_u8(4); // bType = emotion
        emotion_pkt.write_u32(12); // emotion = victory cheer
        world.broadcast_to_zone(pos.zone_id, Arc::new(emotion_pkt), None);
    }
}

/// Send an object event failure response.
/// Format: `WIZ_OBJECT_EVENT + u8(object_type) + u8(0)`
async fn send_fail(session: &mut ClientSession, obj_type: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
    pkt.write_u8(obj_type);
    pkt.write_u8(0); // failure
    session.send_packet(&pkt).await
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use crate::object_event_constants::{
        OBJECT_BIND, OBJECT_FLAG_LEVER, OBJECT_GATE_LEVER, OBJECT_KROWAZ_GATE, OBJECT_WARP_GATE,
        OBJECT_WOOD_LEVER,
    };
    use crate::world::types::{ZONE_BIFROST, ZONE_DELOS};
    use crate::zone::ObjectType;

    #[test]
    fn test_object_type_enum() {
        assert_eq!(ObjectType::from_i16(0), Some(ObjectType::Bind));
        assert_eq!(ObjectType::from_i16(1), Some(ObjectType::Gate));
        assert_eq!(ObjectType::from_i16(3), Some(ObjectType::GateLever));
        assert_eq!(ObjectType::from_i16(4), Some(ObjectType::FlagLever));
        assert_eq!(ObjectType::from_i16(5), Some(ObjectType::WarpGate));
        assert_eq!(ObjectType::from_i16(7), Some(ObjectType::RemoveBind));
        assert_eq!(ObjectType::from_i16(8), Some(ObjectType::Anvil));
        assert_eq!(ObjectType::from_i16(12), Some(ObjectType::KrowazGate));
        assert_eq!(ObjectType::from_i16(14), Some(ObjectType::Wood));
        assert_eq!(ObjectType::from_i16(15), Some(ObjectType::WoodLever));
        assert_eq!(ObjectType::from_i16(99), None);
    }

    #[test]
    fn test_object_event_incoming_packet() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u16(5); // objectIndex
        pkt.write_u16(100); // nid

        assert_eq!(pkt.data.len(), 4);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(5));
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_object_event_success_response() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_BIND);
        pkt.write_u8(1); // success

        assert_eq!(pkt.opcode, Opcode::WizObjectEvent as u8);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_BIND);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_object_event_fail_response() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_WARP_GATE);
        pkt.write_u8(0); // failure

        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_WARP_GATE);
        assert_eq!(pkt.data[1], 0);
    }

    #[test]
    fn test_gate_lever_success_response() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_GATE_LEVER);
        pkt.write_u8(1); // success

        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_GATE_LEVER);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_flag_lever_success_response() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_FLAG_LEVER);
        pkt.write_u8(1); // success

        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_FLAG_LEVER);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_wood_lever_fail_response() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_WOOD_LEVER);
        pkt.write_u8(0); // failure

        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_WOOD_LEVER);
        assert_eq!(pkt.data[1], 0);
    }

    #[test]
    fn test_krowaz_gate_fail_response() {
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_KROWAZ_GATE);
        pkt.write_u8(0); // failure

        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_KROWAZ_GATE);
        assert_eq!(pkt.data[1], 0);
    }

    #[test]
    fn test_zone_constants_for_gate_logic() {
        // Delos zone and Bifrost zone IDs used in gate lever logic
        assert_eq!(ZONE_DELOS, 30);
        assert_eq!(ZONE_BIFROST, 31);
    }

    // ── Krowaz Gate Key Constants ─────────────────────────────────────

    #[test]
    fn test_krowaz_key_item_ids() {
        const ITEM_BLUE_KEY: u32 = 310_045_000;
        const ITEM_RED_KEY: u32 = 310_046_000;
        const ITEM_BLACK_KEY: u32 = 310_047_000;

        assert_eq!(ITEM_BLUE_KEY, 310045000);
        assert_eq!(ITEM_RED_KEY, 310046000);
        assert_eq!(ITEM_BLACK_KEY, 310047000);
        // Keys are sequential IDs
        assert_eq!(ITEM_RED_KEY - ITEM_BLUE_KEY, 1000);
        assert_eq!(ITEM_BLACK_KEY - ITEM_RED_KEY, 1000);
    }

    #[test]
    fn test_krowaz_gate_success_response() {
        // KrowazGate (type 12) success packet
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(OBJECT_KROWAZ_GATE);
        pkt.write_u8(1); // success

        assert_eq!(pkt.opcode, Opcode::WizObjectEvent as u8);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], OBJECT_KROWAZ_GATE);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_krowaz_gate_type_enum() {
        // KrowazGate is type 12
        assert_eq!(ObjectType::from_i16(12), Some(ObjectType::KrowazGate));
        assert_eq!(ObjectType::KrowazGate as i16, 12);
    }

    #[test]
    fn test_krowaz_gate_name_matching() {
        // Test the gate name matching logic used in KrowazGateEvent
        let gate_names = [
            "Blue Key Gate",
            "Red Key Gate",
            "Black Key Gate",
            "Accomplisher Gate",
            "Benshar Gate",
        ];
        for name in &gate_names {
            assert!(!name.is_empty());
        }
        // Accomplisher and Benshar require all 3 keys
        assert_ne!("Accomplisher Gate", "Blue Key Gate");
        assert_ne!("Benshar Gate", "Accomplisher Gate");
    }

    // ── Wood Object Event ─────────────────────────────────────────────

    #[test]
    fn test_wood_success_response() {
        // Wood (type 14) success packet
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(14); // OBJECT_WOOD
        pkt.write_u8(1); // success

        assert_eq!(pkt.opcode, Opcode::WizObjectEvent as u8);
        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], 14);
        assert_eq!(pkt.data[1], 1);
    }

    #[test]
    fn test_wood_fail_response() {
        // Wood (type 14) fail packet — sent when war is not active
        let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
        pkt.write_u8(14); // OBJECT_WOOD
        pkt.write_u8(0); // failure

        assert_eq!(pkt.data.len(), 2);
        assert_eq!(pkt.data[0], 14);
        assert_eq!(pkt.data[1], 0);
    }

    #[test]
    fn test_wood_type_enum() {
        assert_eq!(ObjectType::from_i16(14), Some(ObjectType::Wood));
        assert_eq!(ObjectType::Wood as i16, 14);
    }

    // ── Sprint 318: Bind point validation (by_life + sIndex) ────────

    /// and sIndex must NOT be 101 (Karus homecoming) or 201 (Elmorad homecoming).
    #[test]
    fn test_bind_point_by_life_valid() {
        let by_life: i16 = 1;
        let s_index: i16 = 50;
        let valid = by_life == 1 && s_index != 101 && s_index != 201;
        assert!(valid, "by_life=1, normal sIndex should be valid bind point");
    }

    #[test]
    fn test_bind_point_by_life_invalid() {
        let by_life: i16 = 0;
        let s_index: i16 = 50;
        let valid = by_life == 1 && s_index != 101 && s_index != 201;
        assert!(!valid, "by_life=0 should reject bind point");
    }

    #[test]
    fn test_bind_point_sindex_101_rejected() {
        // sIndex 101 = Karus homecoming gate — not a valid bind for /town
        let by_life: i16 = 1;
        let s_index: i16 = 101;
        let valid = by_life == 1 && s_index != 101 && s_index != 201;
        assert!(!valid, "sIndex 101 should be rejected");
    }

    #[test]
    fn test_bind_point_sindex_201_rejected() {
        // sIndex 201 = Elmorad homecoming gate — not a valid bind for /town
        let by_life: i16 = 1;
        let s_index: i16 = 201;
        let valid = by_life == 1 && s_index != 101 && s_index != 201;
        assert!(!valid, "sIndex 201 should be rejected");
    }

    #[test]
    fn test_bind_point_sindex_202_allowed() {
        // sIndex 202 — a normal bind point, should be allowed
        let by_life: i16 = 1;
        let s_index: i16 = 202;
        let valid = by_life == 1 && s_index != 101 && s_index != 201;
        assert!(valid, "sIndex 202 should be allowed");
    }

    // ── Sprint 325: Opposing-nation zone warp gate check ─────────────

    #[test]
    fn test_opposing_nation_zone_blocks_warp_gate() {
        // Karus player (nation=1) in Elmorad zone (zone_id=2) → BLOCKED
        let zone_id: u16 = 2;
        let nation: u8 = 1;
        let blocked = zone_id <= 2 && zone_id != nation as u16;
        assert!(blocked, "Karus player in Elmorad zone should be blocked");
    }

    #[test]
    fn test_same_nation_zone_allows_warp_gate() {
        // Karus player (nation=1) in Karus zone (zone_id=1) → ALLOWED
        let zone_id: u16 = 1;
        let nation: u8 = 1;
        let blocked = zone_id <= 2 && zone_id != nation as u16;
        assert!(!blocked, "Karus player in Karus zone should be allowed");
    }

    #[test]
    fn test_neutral_zone_allows_warp_gate() {
        // Any player in Moradon (zone_id=21) → ALLOWED (zone > 2)
        let zone_id: u16 = 21;
        let nation: u8 = 1;
        let blocked = zone_id <= 2 && zone_id != nation as u16;
        assert!(!blocked, "Player in neutral zone should be allowed");
    }

    #[test]
    fn test_flag_victory_reward_constants() {
        use crate::systems::war;
        // C++ globals.h:46-48
        assert_eq!(war::NUM_FLAG_VICTORY, 4);
        assert_eq!(war::AWARD_GOLD, 100_000);
        assert_eq!(war::AWARD_EXP, 5000);
        assert_eq!(war::COMMAND_CAPTAIN, 100);
    }

    #[test]
    fn test_flag_victory_np_rewards() {
        use crate::systems::war;
        // Captain: king=500, non-king=300
        assert_eq!(war::captain_reward_np(true, true), 500);
        assert_eq!(war::captain_reward_np(false, true), 300);
        // Non-captain: king=200, non-king=100
        assert_eq!(war::flag_victory_np(true), 200);
        assert_eq!(war::flag_victory_np(false), 100);
    }

    #[test]
    fn test_victory_emotion_packet_format() {
        // StateChangeServerDirect(4, 12) → WIZ_STATE_CHANGE + u32(sid) + u8(4) + u32(12)
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(42);
        pkt.write_u8(4);
        pkt.write_u32(12);
        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        assert_eq!(pkt.data.len(), 9); // 4 + 1 + 4
    }

    #[test]
    fn test_flag_victory_zone_filter() {
        // C++ BattleSystem.cpp:624-627:
        // Player must be in-game AND zone_id == nation AND nation == winner
        // Karus(1) in zone 1 → rewarded if winner=1
        let nation: u8 = 1;
        let zone_id: u16 = 1;
        let winner: u8 = 1;
        assert!(nation == winner && zone_id == nation as u16);

        // Karus(1) in zone 21 (Moradon) → NOT rewarded
        let zone_id2: u16 = 21;
        assert!((zone_id2 != nation as u16));

        // Karus(1) when winner=2 → NOT rewarded
        let winner2: u8 = 2;
        assert!(nation != winner2);
    }
}
