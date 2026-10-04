//! WIZ_PET (0x76) handler — pet system.
//! Sub-opcodes (from client):
//! - 1 (ModeFunction):
//!   - 5 (NormalMode): switch between attack(3)/defence(4)/looting(8)/chat(9)
//!   - 16 (FoodMode): feed pet with food items
//! - 2 (PetUseSkill): pet casts a skill (delegated to magic system)
//! Server-initiated packets (sub-opcode 1):
//! - 5/1/1 — spawn info
//! - 5/2   — death notification
//! - 7     — HP change
//! - 8     — damage display
//! - 10    — EXP change
//! - 11    — level-up broadcast
//! - 13    — MP change
//! - 0x0F  — satisfaction update
//! - 0x10  — food response
//! Pet modes (`GameDefine.h:1153`):
//! - MODE_ATTACK = 3
//! - MODE_DEFENCE = 4
//! - MODE_LOOTING = 8
//! - MODE_CHAT = 9
//! - MODE_SATISFACTION_UPDATE = 0x0F
//! - MODE_FOOD = 0x10

use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;
use tracing::debug;

use crate::npc::{build_npc_inout, NpcInstance, NPC_IN};
use crate::session::{ClientSession, SessionState};

/// Pet mode constants — `GameDefine.h:1153-1158`.
const MODE_SUMMON: u8 = 2;

/// Base NPC template used for summoned Kaul pets.
/// npc_template.s_sid=19000, by_type=15, s_pid=25500.
const PET_RUNTIME_TEMPLATE_SID: u16 = 19_000;
pub(crate) const MODE_ATTACK: u8 = 3;
const MODE_DEFENCE: u8 = 4;
const MODE_LOOTING: u8 = 8;
const MODE_CHAT: u8 = 9;
const MODE_SATISFACTION_UPDATE: u8 = 0x0F;
const MODE_FOOD: u8 = 0x10;

/// Pet sub-opcode constants.
const PET_MODE_FUNCTION: u8 = 1;
const PET_USE_SKILL: u8 = 2;

/// WIZ_MAGIC_PROCESS sub-opcode for effecting (visual play).
const MAGIC_EFFECTING_SUBCODE: u8 = 3;

/// Mode function sub-opcodes.
const NORMAL_MODE: u8 = 5;
const FOOD_MODE: u8 = 16;

/// Server-initiated sub-codes under PET_MODE_FUNCTION.
const PET_HP_CHANGE_CODE: u8 = 7;
const PET_DAMAGE_DISPLAY_CODE: u8 = 8;
const PET_EXP_CHANGE_CODE: u8 = 10;
const PET_LEVEL_UP_CODE: u8 = 11;
const PET_MP_CHANGE_CODE: u8 = 13;

/// Food item IDs and their satisfaction percentages.
const FOOD_ITEM_20: u32 = 389570000; // +20% satisfaction
const FOOD_ITEM_50: u32 = 389580000; // +50% satisfaction
const FOOD_ITEM_100: u32 = 389590000; // +100% satisfaction

use super::SLOT_MAX;

/// Maximum satisfaction value.
const MAX_SATISFACTION: i16 = 10000;

/// Maximum pet level — used in exp/level-up logic.
#[cfg(test)]
const MAX_PET_LEVEL: u8 = 60;

/// Pet inventory slot count
const PET_INVENTORY_TOTAL: u8 = 4;

/// Handle WIZ_PET from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot use pets
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut r = PacketReader::new(&pkt.data);
    let opcode = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match opcode {
        PET_MODE_FUNCTION => handle_mode_function(session, &mut r).await,
        PET_USE_SKILL => handle_pet_use_skill(session, &mut r).await,
        _ => {
            debug!(
                "[{}] WIZ_PET: unhandled sub-opcode {}",
                session.addr(),
                opcode
            );
            Ok(())
        }
    }
}

/// Handle PetUseSkill (sub-opcode 2).
/// The pet casts a skill on a target NPC. Builds and broadcasts a
/// `WIZ_MAGIC_PROCESS` effecting packet from the pet's NPC perspective.
/// If the pet is in defence mode, auto-switches to attack mode.
async fn handle_pet_use_skill(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world();

    let pet_info = world.with_session(sid, |h| {
        h.pet_data.as_ref().map(|p| (p.nid, p.state_change))
    });

    let (pet_nid, pet_mode) = match pet_info {
        Some(Some((nid, mode))) => (nid, mode),
        _ => return Ok(()),
    };

    // A pet must be spawned before it can cast or begin a family attack.
    if pet_nid == 0 || world.get_npc_instance(pet_nid as u32).is_none() {
        return Ok(());
    }

    let sub_code = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };
    let skill_id = match r.read_u32() {
        Some(v) => v,
        None => return Ok(()),
    };

    if skill_id < 300000 {
        return Ok(());
    }

    let caster_id = r.read_u32().unwrap_or(0);
    let target_id = r.read_u32().unwrap_or(0);

    debug!(
        "[{}] WIZ_PET: PetUseSkill received sub_code={} skill_id={} caster={} target={} pet_nid={} mode={}",
        session.addr(),
        sub_code,
        skill_id,
        caster_id,
        target_id,
        pet_nid,
        pet_mode
    );

    // v2615 uses the full 32-bit runtime NPC ID here.  Validate the target
    // before arming the background attack tick; player IDs and dead/missing
    // NPCs are not valid pet attack targets.
    if target_id < crate::npc::NPC_BAND
        || world.get_npc_instance(target_id).is_none()
        || world.is_npc_dead(target_id)
    {
        return Ok(());
    }

    // Build and broadcast WIZ_MAGIC_PROCESS effecting packet from the pet's
    // perspective so the skill visual plays on all nearby clients.
    let mut magic_pkt = Packet::new(Opcode::WizMagicProcess as u8);
    magic_pkt.write_u8(MAGIC_EFFECTING_SUBCODE);
    magic_pkt.write_u32(skill_id);
    magic_pkt.write_u32(pet_nid as u32);
    magic_pkt.write_u32(target_id);
    magic_pkt.write_u16(0); // data[0]
    magic_pkt.write_u16(0); // data[1]
    magic_pkt.write_u16(0); // data[2]
    magic_pkt.write_u16(0); // data[3]
    magic_pkt.write_u16(0); // data[4]
    magic_pkt.write_u16(0); // data[5]

    if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(magic_pkt),
            None,
            event_room,
        );
    }

    // Start or retarget the pet's server-side auto-attack.
    //
    // pet_attack_tick only processes pets when attack_started=true and
    // attack_target_id contains a valid runtime NPC ID.
    world.update_session(sid, |h| {
        if let Some(ref mut pet) = h.pet_data {
            pet.state_change = MODE_ATTACK;
            pet.attack_started = true;
            pet.attack_target_id = target_id as i32;
        }
    });

    // The v2615 client keeps its own copy of the pet mode.  The reference
    // server acknowledges the automatic DEFENCE -> ATTACK transition after
    // Designated Pet Attack.  Without this packet the server attacks, but the
    // client still considers the pet defensive and suppresses the remaining
    // active pet skills before they ever reach WIZ_PET.
    if pet_mode == MODE_DEFENCE {
        let mode_pkt = build_pet_mode_change_packet(MODE_ATTACK);
        session.send_packet(&mode_pkt).await?;
    }

    // Decrease satisfaction by 10 per skill use
    pet_satisfaction_update(session, -10).await;

    debug!(
        "[{}] WIZ_PET: PetUseSkill attack_started skill_id={} pet_nid={} target={}",
        session.addr(),
        skill_id,
        pet_nid,
        target_id
    );
    Ok(())
}

/// Build the v2615 acknowledgement that synchronizes the pet mode in the
/// client UI with the authoritative server-side state.
fn build_pet_mode_change_packet(mode: u8) -> Packet {
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(NORMAL_MODE);
    resp.write_u8(mode);
    resp.write_u16(1); // success
    resp
}

/// Handle ModeFunction (sub-opcode 1).
async fn handle_mode_function(
    session: &mut ClientSession,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let has_pet = session
        .world()
        .with_session(session.session_id(), |h| h.pet_data.is_some())
        .unwrap_or(false);

    if !has_pet {
        return Ok(());
    }

    let sub_code = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };
    let mode = match r.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };

    match sub_code {
        NORMAL_MODE => handle_normal_mode(session, mode, r).await,
        FOOD_MODE => handle_food_mode(session, mode, r).await,
        _ => {
            debug!(
                "[{}] WIZ_PET: ModeFunction unhandled sub_code={}",
                session.addr(),
                sub_code
            );
            Ok(())
        }
    }
}

/// Handle NormalMode (sub-code 5) — switch attack/defence/looting/chat mode.
pub(crate) async fn handle_normal_mode(
    session: &mut ClientSession,
    mode: u8,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    match mode {
        MODE_SUMMON => {
            let world = session.world().clone();
            let sid = session.session_id();

            // GAMESTART may rebuild the runtime session holder after phase 1,
            // which can clear pet_data. Restore the equipped pet lazily here.
            let pet_missing = world
                .with_session(sid, |h| h.pet_data.is_none())
                .unwrap_or(true);

            if pet_missing {
                let char_id = session.character_id().unwrap_or("").to_string();

                if !char_id.is_empty() {
                    let pool = session.pool().clone();
                    let char_repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                    let pet_repo = ko_db::repositories::pet::PetRepository::new(&pool);

                    match char_repo.load_items(&char_id).await {
                        Ok(items) => {
                            let pet_item = items.iter().find(|item| {
                                item.serial_num > 0
                                    && (item.item_id == 610_001_000
                                        || item.slot_index == 5
                                        || item.slot_index == crate::world::CFAIRY_SLOT as i16)
                            });

                            if let Some(item) = pet_item {
                                match pet_repo.load_pet_data(item.serial_num).await {
                                    Ok(Some(row)) => {
                                        let restored = crate::world::PetState {
                                            serial_id: row.n_serial_id.max(0) as u64,
                                            level: row.b_level.clamp(1, 60) as u8,
                                            satisfaction: row.s_satisfaction.clamp(0, 10_000),
                                            exp: row.n_exp.max(0) as u32,
                                            hp: row.s_hp.max(0) as u16,
                                            nid: 0,
                                            index: row.n_index.max(0) as u32,
                                            mp: row.s_mp.max(0) as u16,
                                            state_change: MODE_DEFENCE,
                                            name: row.s_pet_name,
                                            pid: row.s_pid.max(0) as u16,
                                            size: row.s_size.max(0) as u16,
                                            attack_started: false,
                                            attack_target_id: -1,
                                            ..Default::default()
                                        };

                                        world.update_session(sid, |h| {
                                            h.pet_data = Some(restored);
                                        });

                                        tracing::info!(
                                            "[sid={}] WIZ_PET: lazily restored pet serial={} index={} pid={}",
                                            sid,
                                            row.n_serial_id,
                                            row.n_index,
                                            row.s_pid
                                        );
                                    }
                                    Ok(None) => {
                                        tracing::warn!(
                                            "[sid={}] WIZ_PET: no pet_user_data for serial={}",
                                            sid,
                                            item.serial_num
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "[sid={}] WIZ_PET: pet DB load failed serial={}: {}",
                                            sid,
                                            item.serial_num,
                                            e
                                        );
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    "[sid={}] WIZ_PET: equipped pet item not found for {}",
                                    sid,
                                    char_id
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[sid={}] WIZ_PET: inventory DB load failed for {}: {}",
                                sid,
                                char_id,
                                e
                            );
                        }
                    }
                }
            }

            let snapshot = world.with_session(sid, |h| {
                (
                    h.position,
                    h.event_room,
                    h.character.clone(),
                    h.pet_data.clone(),
                )
            });

            let (pos, event_room, owner, pet) = match snapshot {
                Some((pos, event_room, Some(owner), Some(pet))) => (pos, event_room, owner, pet),
                _ => {
                    debug!(
                        "[{}] WIZ_PET: summon rejected, owner or pet state missing",
                        session.addr()
                    );
                    return Ok(());
                }
            };

            // Zaten dünyada kayıtlıysa ikinci kez NPC oluşturma.
            if pet.nid != 0 && world.get_npc_instance(pet.nid as u32).is_some() {
                debug!(
                    "[{}] WIZ_PET: summon ignored, pet already spawned nid={}",
                    session.addr(),
                    pet.nid
                );
                return Ok(());
            }

            // pet.pid is the client model/SPID (25500), not npc_template.s_sid.
            // Use the dedicated type-15 Kaul runtime template.
            let template = match world.get_npc_template(PET_RUNTIME_TEMPLATE_SID, false) {
                Some(t) => t,
                None => {
                    tracing::warn!(
                        "[{}] WIZ_PET: summon failed, runtime template missing sid={}",
                        session.addr(),
                        PET_RUNTIME_TEMPLATE_SID
                    );
                    return Ok(());
                }
            };

            let runtime_nid = world.allocate_npc_id();

            // Sahibin hemen yanında doğur.
            let spawn_x = pos.x + 1.5;
            let spawn_z = pos.z + 1.5;

            let instance = NpcInstance {
                nid: runtime_nid,
                proto_id: PET_RUNTIME_TEMPLATE_SID,
                is_monster: false,
                zone_id: pos.zone_id,
                x: spawn_x,
                y: pos.y,
                z: spawn_z,
                direction: 0,
                region_x: pos.region_x,
                region_z: pos.region_z,
                gate_open: 0,
                object_type: 0,
                nation: owner.nation,
                special_type: 0,
                trap_number: 0,
                event_room,
                is_event_npc: false,
                summon_type: 0,
                user_name: owner.name.clone(),
                pet_name: pet.name.clone(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };

            world.insert_npc_instance(instance.clone());
            world.init_npc_hp(runtime_nid, pet.hp as i32);

            world.update_session(sid, |h| {
                if let Some(ref mut active_pet) = h.pet_data {
                    active_pet.nid = runtime_nid as u16;
                    active_pet.state_change = MODE_DEFENCE;
                    active_pet.attack_started = false;
                    active_pet.attack_target_id = -1;
                }
            });

            let npc_in = build_npc_inout(NPC_IN, &instance, &template);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(npc_in),
                None,
                event_room,
            );

            let stats = world.get_pet_stats_info(pet.level);

            let spawn_info = PetSpawnInfo {
                index: pet.index,
                name: pet.name.clone(),
                level: pet.level,
                exp_percent: 0,
                max_hp: stats
                    .as_ref()
                    .map(|v| v.pet_max_hp as u16)
                    .unwrap_or(pet.hp),
                hp: pet.hp,
                max_mp: stats
                    .as_ref()
                    .map(|v| v.pet_max_sp as u16)
                    .unwrap_or(pet.mp),
                mp: pet.mp,
                satisfaction: pet.satisfaction.max(0) as u16,
                attack: stats.as_ref().map(|v| v.pet_attack as u16).unwrap_or(0),
                defence: stats.as_ref().map(|v| v.pet_defence as u16).unwrap_or(0),
                resistance: stats.as_ref().map(|v| v.pet_res as u16).unwrap_or(0),
            };

            let pet_ui = build_pet_spawn_packet(&spawn_info);
            session.send_packet(&pet_ui).await?;

            tracing::info!(
                "[sid={}] WIZ_PET: summoned nid={} template_sid={} model_spid={} name={} zone={} pos={:.1}/{:.1}/{:.1}",
                sid,
                runtime_nid,
                PET_RUNTIME_TEMPLATE_SID,
                pet.pid,
                pet.name,
                pos.zone_id,
                spawn_x,
                pos.y,
                spawn_z
            );

            return Ok(());
        }
        MODE_ATTACK | MODE_DEFENCE | MODE_LOOTING => {
            // Update pet mode
            session.world().update_session(session.session_id(), |h| {
                if let Some(ref mut pet) = h.pet_data {
                    pet.state_change = mode;
                    // If switching to defence, stop attacking
                    if mode == MODE_DEFENCE {
                        pet.attack_started = false;
                        pet.attack_target_id = -1;
                    }
                }
            });

            // Send mode change confirmation
            let resp = build_pet_mode_change_packet(mode);
            session.send_packet(&resp).await?;

            debug!("[{}] WIZ_PET: NormalMode set to {}", session.addr(), mode);
        }
        MODE_CHAT => {
            // Read chat message (DByte-prefixed string in C++)
            let chat = match r.read_string() {
                Some(v) => v,
                None => return Ok(()),
            };

            // Send chat response
            let mut resp = Packet::new(Opcode::WizPet as u8);
            resp.write_u8(PET_MODE_FUNCTION);
            resp.write_u8(NORMAL_MODE);
            resp.write_u8(MODE_CHAT);
            resp.write_u16(1); // success
            resp.data.extend_from_slice(chat.as_bytes());
            session.send_packet(&resp).await?;

            debug!("[{}] WIZ_PET: Chat message '{}'", session.addr(), chat);
        }
        _ => {
            debug!(
                "[{}] WIZ_PET: NormalMode unhandled mode={}",
                session.addr(),
                mode
            );
        }
    }
    Ok(())
}

/// Handle FoodMode (sub-code 16) — feed the pet.
async fn handle_food_mode(
    session: &mut ClientSession,
    slot_index: u8,
    r: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let item_id = match r.read_u32() {
        Some(v) => v,
        None => return Ok(()),
    };

    // Validate food item
    if item_id != FOOD_ITEM_20 && item_id != FOOD_ITEM_50 && item_id != FOOD_ITEM_100 {
        return Ok(());
    }

    let sid = session.session_id();
    let world = session.world().clone();

    // Validate inventory slot contains the food item
    let slot_valid = world
        .with_session(sid, |h| {
            let inv_slot = SLOT_MAX + slot_index as usize;
            if inv_slot >= h.inventory.len() {
                return false;
            }
            let slot = &h.inventory[inv_slot];
            slot.item_id != 0 && slot.item_id == item_id && slot.count > 0
        })
        .unwrap_or(false);

    if !slot_valid {
        return Ok(());
    }

    // Calculate new satisfaction and consume the food item
    let mut new_satisfaction: i16 = 0;
    let mut remaining_count: u16 = 0;
    let mut remaining_item_id: u32 = 0;

    world.update_session(sid, |h| {
        let pet = match h.pet_data.as_mut() {
            Some(p) => p,
            None => return,
        };

        let old_sat = pet.satisfaction;
        let increase = match item_id {
            FOOD_ITEM_20 => (old_sat as i32 * 20) / 100,
            FOOD_ITEM_50 => (old_sat as i32 * 50) / 100,
            FOOD_ITEM_100 => (old_sat as i32 * 100) / 100,
            _ => 0,
        };
        let mut new_sat = old_sat + increase as i16;
        if new_sat > MAX_SATISFACTION {
            new_sat = MAX_SATISFACTION;
        }
        pet.satisfaction = new_sat;
        new_satisfaction = new_sat;

        // Consume the food item
        let inv_slot = SLOT_MAX + slot_index as usize;
        h.inventory[inv_slot].count -= 1;
        let rem_count = h.inventory[inv_slot].count;
        if rem_count == 0 {
            h.inventory[inv_slot].item_id = 0;
            h.inventory[inv_slot].durability = 0;
            remaining_item_id = 0;
        } else {
            remaining_item_id = h.inventory[inv_slot].item_id;
        }
        remaining_count = rem_count;
    });

    // Send food response
    //                       << uint16(0) << uint32(0) << uint16(10000 - sOldSatisfaction);
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(MODE_FOOD);
    resp.write_u8(1); // success
    resp.write_u8(slot_index);
    resp.write_u32(remaining_item_id);
    resp.write_u16(remaining_count);
    resp.write_u16(0);
    resp.write_u32(0);
    resp.write_u16((MAX_SATISFACTION - new_satisfaction) as u16);
    session.send_packet(&resp).await?;

    // After feeding, C++ calls PetSatisFactionUpdate(), SetUserAbility(), SendItemWeight().
    // We already updated satisfaction above, so send the update packet directly.
    let pet_nid = world
        .with_session(sid, |h| h.pet_data.as_ref().map(|p| p.nid as u32))
        .flatten()
        .unwrap_or(0);
    if new_satisfaction > 0 {
        let mut sat_pkt = Packet::new(Opcode::WizPet as u8);
        sat_pkt.write_u8(PET_MODE_FUNCTION);
        sat_pkt.write_u8(MODE_SATISFACTION_UPDATE);
        sat_pkt.write_u16(new_satisfaction as u16);
        sat_pkt.write_u32(pet_nid);
        session.send_packet(&sat_pkt).await?;
    }

    // Weight notification is integrated into set_user_ability().
    world.set_user_ability(sid);

    debug!(
        "[{}] WIZ_PET: Fed pet with item {}, satisfaction now {}",
        session.addr(),
        item_id,
        new_satisfaction
    );
    Ok(())
}

/// Update pet satisfaction by a delta amount.
async fn pet_satisfaction_update(session: &mut ClientSession, amount: i16) {
    let sid = session.session_id();
    let world = session.world();

    let mut satisfaction: i16 = 0;
    let mut nid: u32 = 0;
    let mut died = false;

    world.update_session(sid, |h| {
        let pet = match h.pet_data.as_mut() {
            Some(p) => p,
            None => return,
        };

        pet.satisfaction += amount;
        pet.satisfaction = pet.satisfaction.clamp(0, MAX_SATISFACTION);

        satisfaction = pet.satisfaction;
        nid = pet.nid as u32;

        if pet.satisfaction <= 0 {
            died = true;
        }
    });

    if died {
        pet_on_death(session).await;
    } else if satisfaction > 0 {
        // Send satisfaction update
        //                       << m_PettingOn->sSatisfaction
        //                       << uint32(m_PettingOn->sNid);
        let mut resp = Packet::new(Opcode::WizPet as u8);
        resp.write_u8(PET_MODE_FUNCTION);
        resp.write_u8(MODE_SATISFACTION_UPDATE);
        resp.write_u16(satisfaction as u16);
        resp.write_u32(nid);
        if let Err(e) = session.send_packet(&resp).await {
            tracing::error!(
                "[{}] Failed to send pet satisfaction update: {e}",
                session.addr()
            );
        }
    }
}

/// Handle pet death (satisfaction reached 0).
async fn pet_on_death(session: &mut ClientSession) {
    let sid = session.session_id();
    let world = session.world();

    let mut pet_index: Option<u32> = None;

    world.update_session(sid, |h| {
        if let Some(pet) = h.pet_data.take() {
            pet_index = Some(pet.index);
        }
    });

    if let Some(index) = pet_index {
        // Send death notification
        //                       << m_PettingOn->nIndex;
        let mut resp = Packet::new(Opcode::WizPet as u8);
        resp.write_u8(PET_MODE_FUNCTION);
        resp.write_u8(NORMAL_MODE);
        resp.write_u8(2); // death sub-code
        resp.write_u16(1);
        resp.write_u32(index);
        if let Err(e) = session.send_packet(&resp).await {
            tracing::error!(
                "[{}] Failed to send pet death notification: {e}",
                session.addr()
            );
        }

        debug!("[{}] WIZ_PET: Pet died (index={})", session.addr(), index);
    }
}

// ── Server-initiated pet packets ─────────────────────────────────────────

/// Pet stats snapshot for building spawn packets.
/// Extracted from PetState + pet_stats_info table data.
#[derive(Debug, Clone)]
pub struct PetSpawnInfo {
    /// Unique pet index (from DB).
    pub index: u32,
    /// Pet name.
    pub name: String,
    /// Current level (1-60).
    pub level: u8,
    /// Experience as percentage * 100 (e.g. 8100 = 81.00%).
    pub exp_percent: u16,
    /// Max HP for this level.
    pub max_hp: u16,
    /// Current HP.
    pub hp: u16,
    /// Max MP for this level.
    pub max_mp: u16,
    /// Current MP.
    pub mp: u16,
    /// Satisfaction (0-10000).
    pub satisfaction: u16,
    /// Attack power.
    pub attack: u16,
    /// Defence.
    pub defence: u16,
    /// Resistance (used for all 6 resistance slots).
    pub resistance: u16,
}

/// Build and send the pet spawn info packet.
/// This sends the full pet status window to the owning player.
/// Called when a pet is first summoned or after leveling up.
pub fn build_pet_spawn_packet(info: &PetSpawnInfo) -> Packet {
    // C++ layout:
    // WIZ_PET << u8(1) << u8(5) << u8(1) << u8(1) << u8(0) << nIndex
    //   .DByte() << strPetName << u8(119) << bLevel << u16(exp_percent)
    //   << maxHP << hp << maxMP << mp << satisfaction
    //   << attack << defence << res << res << res << res << res << res
    //   << [4x pet inventory items]

    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION); // 1
    resp.write_u8(NORMAL_MODE); // 5
    resp.write_u8(1); // success
    resp.write_u8(1); // spawn flag
    resp.write_u8(0); // padding
    resp.write_u32(info.index); // pet DB index

    // DByte string: u16 length-prefixed pet name
    resp.write_string(&info.name);

    resp.write_u8(119); // pet type constant (C++ hardcoded 119)
    resp.write_u8(info.level);
    resp.write_u16(info.exp_percent);
    resp.write_u16(info.max_hp);
    resp.write_u16(info.hp);
    resp.write_u16(info.max_mp);
    resp.write_u16(info.mp);
    resp.write_u16(info.satisfaction);
    resp.write_u16(info.attack);
    resp.write_u16(info.defence);
    // 6x resistance values (all the same in C++)
    for _ in 0..6 {
        resp.write_u16(info.resistance);
    }

    // Pet inventory: PET_INVENTORY_TOTAL (4) empty slots
    //      + sRemainingRentalTime(u16) + u32(0) + nExpirationTime(u32)
    for _ in 0..PET_INVENTORY_TOTAL {
        resp.write_u32(0); // nNum
        resp.write_u16(0); // sDuration
        resp.write_u16(0); // sCount
        resp.write_u8(0); // bFlag
        resp.write_u16(0); // sRemainingRentalTime
        resp.write_u32(0); // padding
        resp.write_u32(0); // nExpirationTime
    }

    resp
}

/// Build and send the pet HP change packet.
/// Sent to the owner when the pet takes or heals damage.
pub fn build_pet_hp_change_packet(max_hp: u16, current_hp: u16, source_id: u32) -> Packet {
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(PET_HP_CHANGE_CODE);
    resp.write_u16(max_hp);
    resp.write_u16(current_hp);
    resp.write_u32(source_id);
    resp
}

/// Build the pet damage display packet.
/// Sent to the owner to show a damage number over the pet.
pub fn build_pet_damage_display_packet(target_id: i32, damage: i16) -> Packet {
    //   << u8(0) << u8(7) << u8(0) << u8(0) << u8(0) << u8(4) << u8(0) << u8(0) << u8(0)
    //   << i16(damage)
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(PET_DAMAGE_DISPLAY_CODE);
    resp.write_i32(target_id);
    resp.write_u8(0);
    resp.write_u8(7);
    resp.write_u8(0);
    resp.write_u8(0);
    resp.write_u8(0);
    resp.write_u8(4);
    resp.write_u8(0);
    resp.write_u8(0);
    resp.write_u8(0);
    resp.write_i16(damage);
    resp
}

/// Build the pet EXP change packet.
/// Sent to the owner when the pet gains experience.
pub fn build_pet_exp_change_packet(
    gained_exp: u64,
    exp_percent: u16,
    level: u8,
    satisfaction: u16,
) -> Packet {
    //            << u8(level) << u16(satisfaction)
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(PET_EXP_CHANGE_CODE);
    resp.write_u64(gained_exp);
    resp.write_u16(exp_percent);
    resp.write_u8(level);
    resp.write_u16(satisfaction);
    resp
}

/// Build the pet level-up broadcast packet.
/// This is sent to the region (all nearby players) to trigger the
/// level-up visual effect on the pet NPC.
pub fn build_pet_level_up_broadcast_packet(pet_npc_id: u32) -> Packet {
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(PET_LEVEL_UP_CODE);
    resp.write_u32(pet_npc_id);
    resp
}

/// Build the pet MP change packet.
/// Sent to the owner when the pet's MP changes (skill usage, regen, etc).
pub fn build_pet_mp_change_packet(max_mp: u16, current_mp: u16, source_id: u16) -> Packet {
    let mut resp = Packet::new(Opcode::WizPet as u8);
    resp.write_u8(PET_MODE_FUNCTION);
    resp.write_u8(PET_MP_CHANGE_CODE);
    resp.write_u16(max_mp);
    resp.write_u16(current_mp);
    resp.write_u16(source_id);
    resp
}

/// Build the pet item tooltip info response.
/// Appended to an item info packet when inspecting a pet egg in inventory.
pub fn build_pet_item_info(
    index: u32,
    name: &str,
    attack_type: u8,
    level: u8,
    exp_percent: u16,
    satisfaction: u16,
) -> Vec<u8> {
    //         << u16(exp_percent) << satisfaction << u8(0)
    let mut data = Vec::new();
    data.extend_from_slice(&index.to_le_bytes());
    // DByte string (u16 length prefix)
    let name_bytes = name.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    data.extend_from_slice(name_bytes);
    data.push(attack_type);
    data.push(level);
    data.extend_from_slice(&exp_percent.to_le_bytes());
    data.extend_from_slice(&satisfaction.to_le_bytes());
    data.push(0); // trailing zero
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_pet_mode_constants() {
        assert_eq!(MODE_ATTACK, 3);
        assert_eq!(MODE_DEFENCE, 4);
        assert_eq!(MODE_LOOTING, 8);
        assert_eq!(MODE_CHAT, 9);
        assert_eq!(MODE_SATISFACTION_UPDATE, 0x0F);
        assert_eq!(MODE_FOOD, 0x10);
    }

    #[test]
    fn test_pet_spawn_packet_structure() {
        let info = PetSpawnInfo {
            index: 42,
            name: "TestPet".to_string(),
            level: 10,
            exp_percent: 5000,
            max_hp: 116,
            hp: 100,
            max_mp: 139,
            mp: 120,
            satisfaction: 9000,
            attack: 36,
            defence: 90,
            resistance: 18,
        };

        let pkt = build_pet_spawn_packet(&info);
        assert_eq!(pkt.opcode, Opcode::WizPet as u8);

        let mut r = PacketReader::new(&pkt.data);
        // Header
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION)); // 1
        assert_eq!(r.read_u8(), Some(NORMAL_MODE)); // 5
        assert_eq!(r.read_u8(), Some(1)); // success
        assert_eq!(r.read_u8(), Some(1)); // spawn flag
        assert_eq!(r.read_u8(), Some(0)); // padding
        assert_eq!(r.read_u32(), Some(42)); // index

        // Pet name (DByte = u16-length string)
        let name = r.read_string().unwrap();
        assert_eq!(name, "TestPet");

        assert_eq!(r.read_u8(), Some(119)); // pet type constant
        assert_eq!(r.read_u8(), Some(10)); // level
        assert_eq!(r.read_u16(), Some(5000)); // exp_percent
        assert_eq!(r.read_u16(), Some(116)); // max_hp
        assert_eq!(r.read_u16(), Some(100)); // hp
        assert_eq!(r.read_u16(), Some(139)); // max_mp
        assert_eq!(r.read_u16(), Some(120)); // mp
        assert_eq!(r.read_u16(), Some(9000)); // satisfaction
        assert_eq!(r.read_u16(), Some(36)); // attack
        assert_eq!(r.read_u16(), Some(90)); // defence
                                            // 6x resistance
        for _ in 0..6 {
            assert_eq!(r.read_u16(), Some(18));
        }
        // 4x empty pet inventory items
        for _ in 0..PET_INVENTORY_TOTAL {
            assert_eq!(r.read_u32(), Some(0)); // nNum
            assert_eq!(r.read_u16(), Some(0)); // sDuration
            assert_eq!(r.read_u16(), Some(0)); // sCount
            assert_eq!(r.read_u8(), Some(0)); // bFlag
            assert_eq!(r.read_u16(), Some(0)); // sRemainingRentalTime
            assert_eq!(r.read_u32(), Some(0)); // padding
            assert_eq!(r.read_u32(), Some(0)); // nExpirationTime
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_hp_change_packet() {
        let pkt = build_pet_hp_change_packet(500, 350, 1001);
        assert_eq!(pkt.opcode, Opcode::WizPet as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(PET_HP_CHANGE_CODE));
        assert_eq!(r.read_u16(), Some(500)); // max_hp
        assert_eq!(r.read_u16(), Some(350)); // current_hp
        assert_eq!(r.read_u32(), Some(1001)); // source_id
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_damage_display_packet() {
        let pkt = build_pet_damage_display_packet(2005, -150);
        assert_eq!(pkt.opcode, Opcode::WizPet as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(PET_DAMAGE_DISPLAY_CODE));
        // i32 target_id
        let tid_bytes = [
            r.read_u8().unwrap(),
            r.read_u8().unwrap(),
            r.read_u8().unwrap(),
            r.read_u8().unwrap(),
        ];
        assert_eq!(i32::from_le_bytes(tid_bytes), 2005);
        // Fixed padding bytes
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(7));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        // i16 damage
        let dmg_bytes = [r.read_u8().unwrap(), r.read_u8().unwrap()];
        assert_eq!(i16::from_le_bytes(dmg_bytes), -150);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_exp_change_packet() {
        let pkt = build_pet_exp_change_packet(1200, 4500, 15, 8500);
        assert_eq!(pkt.opcode, Opcode::WizPet as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(PET_EXP_CHANGE_CODE));
        assert_eq!(r.read_u64(), Some(1200)); // gained_exp
        assert_eq!(r.read_u16(), Some(4500)); // exp_percent
        assert_eq!(r.read_u8(), Some(15)); // level
        assert_eq!(r.read_u16(), Some(8500)); // satisfaction
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_level_up_broadcast_packet() {
        let pkt = build_pet_level_up_broadcast_packet(9999);
        assert_eq!(pkt.opcode, Opcode::WizPet as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(PET_LEVEL_UP_CODE));
        assert_eq!(r.read_u32(), Some(9999));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_mp_change_packet() {
        let pkt = build_pet_mp_change_packet(200, 150, 3001);
        assert_eq!(pkt.opcode, Opcode::WizPet as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(PET_MP_CHANGE_CODE));
        assert_eq!(r.read_u16(), Some(200)); // max_mp
        assert_eq!(r.read_u16(), Some(150)); // current_mp
        assert_eq!(r.read_u16(), Some(3001)); // source_id
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_item_info() {
        let data = build_pet_item_info(42, "MyPet", 119, 5, 3000, 7500);

        let mut r = PacketReader::new(&data);
        assert_eq!(r.read_u32(), Some(42)); // index
        let name = r.read_string().unwrap(); // DByte string
        assert_eq!(name, "MyPet");
        assert_eq!(r.read_u8(), Some(119)); // attack type
        assert_eq!(r.read_u8(), Some(5)); // level
        assert_eq!(r.read_u16(), Some(3000)); // exp_percent
        assert_eq!(r.read_u16(), Some(7500)); // satisfaction
        assert_eq!(r.read_u8(), Some(0)); // trailing zero
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_food_item_constants() {
        // Verify food item IDs match C++ defines
        assert_eq!(FOOD_ITEM_20, 389570000);
        assert_eq!(FOOD_ITEM_50, 389580000);
        assert_eq!(FOOD_ITEM_100, 389590000);
    }

    #[test]
    fn test_satisfaction_bounds() {
        // MAX_SATISFACTION must match C++ (10000)
        assert_eq!(MAX_SATISFACTION, 10000);
        // Max pet level must be 60
        assert_eq!(MAX_PET_LEVEL, 60);
        // Pet inventory total must be 4
        assert_eq!(PET_INVENTORY_TOTAL, 4);
    }

    #[test]
    fn test_mode_change_packet_roundtrip() {
        // Simulate building a mode change confirmation (attack mode)
        let mut resp = Packet::new(Opcode::WizPet as u8);
        resp.write_u8(PET_MODE_FUNCTION);
        resp.write_u8(NORMAL_MODE);
        resp.write_u8(MODE_ATTACK);
        resp.write_u16(1); // success

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(NORMAL_MODE));
        assert_eq!(r.read_u8(), Some(MODE_ATTACK));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_satisfaction_update_packet_roundtrip() {
        // Simulate building a satisfaction update packet
        let mut resp = Packet::new(Opcode::WizPet as u8);
        resp.write_u8(PET_MODE_FUNCTION);
        resp.write_u8(MODE_SATISFACTION_UPDATE);
        resp.write_u16(8500);
        resp.write_u32(12345);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(MODE_SATISFACTION_UPDATE));
        assert_eq!(r.read_u16(), Some(8500));
        assert_eq!(r.read_u32(), Some(12345));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_death_notification_packet_roundtrip() {
        // Simulate building a death notification
        let mut resp = Packet::new(Opcode::WizPet as u8);
        resp.write_u8(PET_MODE_FUNCTION);
        resp.write_u8(NORMAL_MODE);
        resp.write_u8(2); // death sub-code
        resp.write_u16(1);
        resp.write_u32(777);

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(NORMAL_MODE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u32(), Some(777));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_food_response_packet_roundtrip() {
        // Simulate building a food response
        let mut resp = Packet::new(Opcode::WizPet as u8);
        resp.write_u8(PET_MODE_FUNCTION);
        resp.write_u8(MODE_FOOD);
        resp.write_u8(1); // success
        resp.write_u8(3); // slot_index
        resp.write_u32(389570000); // remaining item_id
        resp.write_u16(4); // remaining count
        resp.write_u16(0);
        resp.write_u32(0);
        resp.write_u16(1500); // hunger = 10000 - satisfaction

        let mut r = PacketReader::new(&resp.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(MODE_FOOD));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u32(), Some(389570000));
        assert_eq!(r.read_u16(), Some(4));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u16(), Some(1500));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_use_skill_magic_packet_format() {
        // Verify the WIZ_MAGIC_PROCESS packet built by pet skill execution
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(MAGIC_EFFECTING_SUBCODE); // 3
        pkt.write_u32(490010); // skill_id
        pkt.write_u32(10500); // pet NPC id
        pkt.write_u32(10001); // target NPC id
        pkt.write_u16(0); // data[0..5]
        pkt.write_u16(0);
        pkt.write_u16(0);
        pkt.write_u16(0);
        pkt.write_u16(0);
        pkt.write_u16(0);

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_EFFECTING_SUBCODE));
        assert_eq!(r.read_u32(), Some(490010));
        assert_eq!(r.read_u32(), Some(10500));
        assert_eq!(r.read_u32(), Some(10001));
        // 6 x u16 data slots
        for _ in 0..6 {
            assert_eq!(r.read_u16(), Some(0));
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_use_skill_constants() {
        assert_eq!(MAGIC_EFFECTING_SUBCODE, 3);
        assert_eq!(PET_USE_SKILL, 2);
    }

    #[test]
    fn test_pet_auto_attack_mode_ack_packet() {
        let pkt = build_pet_mode_change_packet(MODE_ATTACK);
        let mut r = PacketReader::new(&pkt.data);

        assert_eq!(pkt.opcode, Opcode::WizPet as u8);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(NORMAL_MODE));
        assert_eq!(r.read_u8(), Some(MODE_ATTACK));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_v2615_pet_target_id_keeps_full_width() {
        let target_id = 49_886u32;
        let stored = target_id as i32;

        assert!(stored >= crate::npc::NPC_BAND as i32);
        assert_eq!(stored as u32, target_id);
        assert!(target_id > i16::MAX as u32);
    }

    // ── Sprint 955: Additional coverage ──────────────────────────────

    /// Pet mode constants cover all handler branches.
    #[test]
    fn test_pet_mode_all_branches() {
        assert_eq!(MODE_ATTACK, 3);
        assert_eq!(MODE_DEFENCE, 4);
        assert_eq!(NORMAL_MODE, 5);
        assert_eq!(MODE_LOOTING, 8);
        assert_eq!(MODE_CHAT, 9);
        assert_eq!(MODE_SATISFACTION_UPDATE, 0x0F);
        assert_eq!(FOOD_MODE, 16);
        // MODE_SATISFACTION_UPDATE and FOOD_MODE are adjacent
        assert_eq!(MODE_SATISFACTION_UPDATE as u8 + 1, FOOD_MODE);
    }

    /// Food item IDs are sequential by feed amount.
    #[test]
    fn test_food_item_ids() {
        assert_eq!(FOOD_ITEM_20, 389570000);
        assert_eq!(FOOD_ITEM_50, 389580000);
        assert_eq!(FOOD_ITEM_100, 389590000);
        // 10000 gap between each tier
        assert_eq!(FOOD_ITEM_50 - FOOD_ITEM_20, 10000);
        assert_eq!(FOOD_ITEM_100 - FOOD_ITEM_50, 10000);
    }

    /// Pet limits: satisfaction and level caps.
    #[test]
    fn test_pet_limits() {
        assert_eq!(MAX_SATISFACTION, 10000);
        assert_eq!(MAX_PET_LEVEL, 60);
        assert_eq!(PET_INVENTORY_TOTAL, 4);
        // Satisfaction fits in u16
        assert!(MAX_SATISFACTION <= i16::MAX);
    }

    /// Pet sub-opcode function codes.
    #[test]
    fn test_pet_function_codes() {
        assert_eq!(PET_MODE_FUNCTION, 1);
        assert_eq!(PET_USE_SKILL, 2);
        // They are distinct
        assert_ne!(PET_MODE_FUNCTION, PET_USE_SKILL);
    }

    /// Pet S2C display codes are distinct.
    #[test]
    fn test_pet_display_codes() {
        assert_eq!(PET_HP_CHANGE_CODE, 7);
        assert_eq!(PET_DAMAGE_DISPLAY_CODE, 8);
        assert_eq!(PET_EXP_CHANGE_CODE, 10);
        assert_eq!(PET_LEVEL_UP_CODE, 11);
        assert_eq!(PET_MP_CHANGE_CODE, 13);
        // All distinct
        let codes = [
            PET_HP_CHANGE_CODE,
            PET_DAMAGE_DISPLAY_CODE,
            PET_EXP_CHANGE_CODE,
            PET_LEVEL_UP_CODE,
            PET_MP_CHANGE_CODE,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j], "codes[{}] == codes[{}]", i, j);
            }
        }
    }
}
