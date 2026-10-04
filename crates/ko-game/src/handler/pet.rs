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
use tracing::{debug, info};

use crate::npc::{build_npc_inout, NpcInstance, NPC_IN};
use crate::session::{ClientSession, SessionState};
use crate::world::{PetState, UserItemSlot};

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
    info!(
        sid = session.session_id(),
        sub_opcode = opcode,
        remaining_bytes = r.remaining(),
        "PET_EVENT client_packet"
    );

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
        _ => {
            info!(sid, "PET_SKILL rejected reason=no_pet_state");
            return Ok(());
        }
    };

    // A pet must be spawned before it can cast or begin a family attack.
    if pet_nid == 0 || world.get_npc_instance(pet_nid as u32).is_none() {
        info!(sid, pet_nid, "PET_SKILL rejected reason=pet_not_spawned");
        return Ok(());
    }

    // The supplied 2625 decompilation's PetMagicMng.cpp:156 builds WIZ_PET
    // subcommand 2 as one byte plus nine dwords. The first three dwords after
    // that byte are skill, caster and target IDs.
    let Some(request) = read_pet_skill_request(r) else {
        info!(sid, "PET_SKILL rejected reason=malformed_request");
        return Ok(());
    };
    let sub_code = request.sub_code;
    let skill_id = request.skill_id;

    if skill_id < 300000 {
        info!(sid, skill_id, "PET_SKILL rejected reason=invalid_skill_id");
        return Ok(());
    }

    // Keep IDs full width: truncating either one drops runtime NPC targets.
    let caster_id = request.caster_id;
    let target_id = request.target_id;

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
    info!(
        sid,
        pet_nid,
        pet_mode,
        skill_id,
        caster_id,
        target_id,
        sub_code,
        "PET_SKILL request"
    );

    // Pet window recovery skills are intentionally targeted at the owner's
    // own pet. The reference server sends these through MagicPacketNpc(),
    // while the old Rust path rejected them because it only accepted monster
    // targets. Consume the matching pet-slot potion and update the authoritative
    // HP/MP state before sending the normal client packets.
    if let Some((hp_restore, mp_restore, required_item)) = pet_recovery_skill(skill_id) {
        if target_id != pet_nid as u32 {
            return Ok(());
        }
        let level = world
            .with_session(sid, |h| h.pet_data.as_ref().map(|p| p.level))
            .flatten()
            .unwrap_or(1);
        let Some(stats) = world.get_pet_stats_info(level.clamp(1, 60)) else {
            return Ok(());
        };
        let mut consumed = false;
        let mut current_hp = 0u16;
        let mut current_mp = 0u16;
        world.update_session(sid, |h| {
            let Some(pet) = h.pet_data.as_mut() else {
                return;
            };
            let Some(slot) = pet
                .items
                .iter_mut()
                .find(|item| item.item_id == required_item && item.count > 0)
            else {
                return;
            };
            slot.count -= 1;
            if slot.count == 0 {
                *slot = UserItemSlot::default();
            }
            pet.hp = (pet.hp as u32 + hp_restore as u32).min(stats.pet_max_hp.max(1) as u32) as u16;
            pet.mp = (pet.mp as u32 + mp_restore as u32).min(stats.pet_max_sp.max(0) as u32) as u16;
            current_hp = pet.hp;
            current_mp = pet.mp;
            consumed = true;
        });
        if !consumed {
            return Ok(());
        }
        world.update_npc_hp(pet_nid as u32, current_hp as i32);
        let mut magic_pkt = Packet::new(Opcode::WizMagicProcess as u8);
        magic_pkt.write_u8(MAGIC_EFFECTING_SUBCODE);
        magic_pkt.write_u32(skill_id);
        magic_pkt.write_u32(pet_nid as u32);
        magic_pkt.write_u32(pet_nid as u32);
        magic_pkt.write_u16(0); // pet x (int16, reference MagicPacketNpc layout)
        magic_pkt.write_u16(0); // pet y
        magic_pkt.write_u16(0); // pet z
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
        session
            .send_packet(&build_pet_hp_change_packet(
                stats.pet_max_hp.max(1) as u16,
                current_hp,
                pet_nid as u32,
            ))
            .await?;
        session
            .send_packet(&build_pet_mp_change_packet(
                stats.pet_max_sp.max(0) as u16,
                current_mp,
                pet_nid,
            ))
            .await?;
        save_pet_items(session).await;
        pet_satisfaction_update(session, -10).await;
        return Ok(());
    }

    // Validate the full-width runtime NPC target before arming the background
    // attack tick; player IDs and dead/missing NPCs are not valid here.
    if target_id < crate::npc::NPC_BAND
        || world.get_npc_instance(target_id).is_none()
        || world.is_npc_dead(target_id)
    {
        info!(sid, pet_nid, skill_id, target_id, "PET_SKILL rejected reason=invalid_or_dead_npc_target");
        return Ok(());
    }

    // Build and broadcast WIZ_MAGIC_PROCESS effecting packet from the pet's
    // perspective so the skill visual plays on all nearby clients.
    let mut magic_pkt = Packet::new(Opcode::WizMagicProcess as u8);
    magic_pkt.write_u8(MAGIC_EFFECTING_SUBCODE);
    magic_pkt.write_u32(skill_id);
    // Match PetMainHandler.cpp's MagicPacketNpc payload exactly: two signed
    // 16-bit unit IDs followed by the pet's three 16-bit coordinates. The old
    // Rust packet used 32-bit IDs plus six zero words, corrupting the client
    // magic-process decode and suppressing pet skill effects.
    let Some(pet_instance) = world.get_npc_instance(pet_nid as u32) else {
        return Ok(());
    };
    magic_pkt.write_u16(pet_nid);
    magic_pkt.write_u16(target_id as u16);
    magic_pkt.write_u16(pet_instance.x.max(0.0) as u16);
    magic_pkt.write_u16(pet_instance.y.max(0.0) as u16);
    magic_pkt.write_u16(pet_instance.z.max(0.0) as u16);

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
            pet.pending_attack_skill_id = if (301_001..=301_006).contains(&skill_id) {
                skill_id
            } else {
                0
            };
        }
    });

    // The v2625 client keeps its own copy of the pet mode. The reference
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PetSkillRequest {
    sub_code: u8,
    skill_id: u32,
    caster_id: u32,
    target_id: u32,
}

/// Decode the fixed prefix of the v2625 PetMagicMng.cpp WIZ_PET skill packet.
/// Its trailing coordinate and auxiliary dwords are not needed by this handler.
fn read_pet_skill_request(reader: &mut PacketReader<'_>) -> Option<PetSkillRequest> {
    Some(PetSkillRequest {
        sub_code: reader.read_u8()?,
        skill_id: reader.read_u32()?,
        caster_id: reader.read_u32()?,
        target_id: reader.read_u32()?,
    })
}

/// Recovery skills and their backing pet-inventory item. These IDs are the
/// exact allow-list in the reference PetMainHandler.cpp.
fn pet_recovery_skill(skill_id: u32) -> Option<(u16, u16, u32)> {
    Some(match skill_id {
        490010 => (45, 0, 389_010_000),
        490011 => (90, 0, 389_011_000),
        490012 => (180, 0, 389_012_000),
        490013 => (360, 0, 389_013_000),
        490014 => (720, 0, 389_014_000),
        500145 => (720, 0, 389_390_000),
        490016 => (0, 120, 389_016_000),
        490017 => (0, 240, 389_017_000),
        490018 => (0, 480, 389_018_000),
        490019 => (0, 960, 389_019_000),
        490020 => (0, 1920, 389_020_000),
        500146 => (0, 1920, 389_400_000),
        _ => return None,
    })
}

/// Build the v2625 acknowledgement that synchronizes the pet mode in the
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
                                        let pet_items = pet_repo.load_pet_items(row.n_serial_id).await.unwrap_or_else(|e| {
                                            tracing::warn!("[sid={}] WIZ_PET: pet item DB load failed serial={}: {}", sid, row.n_serial_id, e);
                                            Vec::new()
                                        });
                                        let restored = PetState {
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

                                        let mut restored = restored;
                                        apply_persistent_pet_items(&mut restored, pet_items);
                                        let equipment = restored.items.iter().enumerate()
                                            .filter(|(_, item)| item.item_id != 0 || item.count != 0)
                                            .map(|(slot, item)| format!("{}:{}x{}", slot, item.item_id, item.count))
                                            .collect::<Vec<_>>().join(",");

                                        world.update_session(sid, |h| {
                                            h.pet_data = Some(restored);
                                        });

                                        tracing::info!(
                                            "[sid={}] PET_LOAD source=lazy serial={} index={} pid={} equipment=[{}]",
                                            sid,
                                            row.n_serial_id,
                                            row.n_index,
                                            row.s_pid,
                                            equipment
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

            let (pos, event_room, owner, mut pet) = match snapshot {
                Some((pos, event_room, Some(owner), Some(pet))) => (pos, event_room, owner, pet),
                _ => {
                    debug!(
                        "[{}] WIZ_PET: summon rejected, owner or pet state missing",
                        session.addr()
                    );
                    return Ok(());
                }
            };

            // If a stale runtime pet exists, move it beside the owner instead
            // of silently ignoring the summon. Clients require an OUT/IN pair
            // to reliably refresh a familiar's position and transformed PID.
            if pet.nid != 0 {
                if let Some(existing) = world.get_npc_instance(pet.nid as u32) {
                    if existing.zone_id == pos.zone_id {
                        let Some(existing_template) =
                            world.get_npc_template(existing.proto_id, existing.is_monster)
                        else {
                            return Ok(());
                        };
                        let mut appearance = existing_template.as_ref().clone();
                        appearance.pid = pet.pid.max(1);
                        appearance.size = pet.size.max(1);
                        let out =
                            build_npc_inout(crate::npc::NPC_OUT, &existing, &existing_template);
                        world.broadcast_to_3x3(
                            existing.zone_id,
                            existing.region_x,
                            existing.region_z,
                            Arc::new(out),
                            None,
                            event_room,
                        );
                        let near_x = pos.x + 1.0;
                        let near_z = pos.z + 1.0;
                        world.update_npc_position(pet.nid as u32, near_x, near_z);
                        if let Some(updated) = world.get_npc_instance(pet.nid as u32) {
                            let input = build_npc_inout(NPC_IN, &updated, &appearance);
                            world.broadcast_to_3x3(
                                updated.zone_id,
                                updated.region_x,
                                updated.region_z,
                                Arc::new(input),
                                None,
                                event_room,
                            );
                            let mut object_event = Packet::new(Opcode::WizObjectEvent as u8);
                            object_event.write_u8(0x0b);
                            object_event.write_u8(0x01);
                            object_event.write_u16(pet.nid);
                            object_event.write_u8(0xc3);
                            object_event.write_u8(0x76);
                            object_event.write_u16(0);
                            world.broadcast_to_3x3(
                                updated.zone_id,
                                updated.region_x,
                                updated.region_z,
                                Arc::new(object_event),
                                None,
                                event_room,
                            );
                        }
                        debug!(
                            "[{}] WIZ_PET: existing familiar moved beside owner nid={} zone={} x={:.1} z={:.1}",
                            session.addr(), pet.nid, pos.zone_id, near_x, near_z
                        );
                        return Ok(());
                    }

                    // A runtime from another zone is invalid; remove it while
                    // keeping PetState and its four equipment slots intact.
                    world.kill_npc(pet.nid as u32);
                    world.update_session(sid, |h| {
                        if let Some(active_pet) = h.pet_data.as_mut() {
                            active_pet.nid = 0;
                        }
                    });
                    pet.nid = 0;
                }
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

            // C++ PetSpawnProcess restores a familiar at its level's full
            // health and mana. Old persisted rows often contain zero here,
            // which made the runtime NPC immediately invisible/dead despite a
            // successful summon packet.
            let stats = world.get_pet_stats_info(pet.level.clamp(1, 60));
            let max_hp = stats
                .as_ref()
                .map(|v| v.pet_max_hp.max(1) as u16)
                .unwrap_or(pet.hp.max(1));
            let max_mp = stats
                .as_ref()
                .map(|v| v.pet_max_sp.max(0) as u16)
                .unwrap_or(pet.mp);
            pet.hp = max_hp;
            pet.mp = max_mp;

            let runtime_nid = world.allocate_npc_id();

            // Sahibin hemen yanında doğur.
            let spawn_x = pos.x + 1.0;
            let spawn_z = pos.z + 1.0;

            let instance = NpcInstance {
                nid: runtime_nid,
                proto_id: PET_RUNTIME_TEMPLATE_SID,
                is_monster: false,
                zone_id: pos.zone_id,
                x: spawn_x,
                y: pos.y,
                z: spawn_z,
                direction: 0,
                region_x: crate::zone::calc_region(spawn_x),
                region_z: crate::zone::calc_region(spawn_z),
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
                    active_pet.hp = max_hp;
                    active_pet.mp = max_mp;
                    active_pet.nid = runtime_nid as u16;
                    active_pet.state_change = MODE_DEFENCE;
                    active_pet.attack_started = false;
                    active_pet.attack_target_id = -1;
                    active_pet.pending_attack_skill_id = 0;
                }
            });

            // The client renders type-15 pets from the PID and size included
            // in the NPC-IN packet. The runtime template is only a carrier;
            // override its appearance with the pet's persisted transform.
            let mut visual_template = template.as_ref().clone();
            visual_template.pid = pet.pid.max(1);
            visual_template.size = pet.size.max(1);
            let npc_in = build_npc_inout(NPC_IN, &instance, &visual_template);
            world.broadcast_to_3x3(
                instance.zone_id,
                instance.region_x,
                instance.region_z,
                Arc::new(npc_in),
                None,
                event_room,
            );

            // PetSpawnProcess follows the NPC-IN with this object event. Some
            // client builds do not instantiate the familiar model reliably
            // from NPC-IN alone; omitting this event causes the model to blink
            // or remain invisible for nearby players.
            let mut object_event = Packet::new(Opcode::WizObjectEvent as u8);
            object_event.write_u8(0x0b);
            object_event.write_u8(0x01);
            object_event.write_u16(runtime_nid as u16);
            object_event.write_u8(0xc3);
            object_event.write_u8(0x76);
            object_event.write_u16(0);
            world.broadcast_to_3x3(
                instance.zone_id,
                instance.region_x,
                instance.region_z,
                Arc::new(object_event),
                None,
                event_room,
            );

            let spawn_info = PetSpawnInfo {
                index: pet.index,
                name: pet.name.clone(),
                level: pet.level,
                exp_percent: 0,
                max_hp,
                hp: pet.hp,
                max_mp,
                mp: pet.mp,
                satisfaction: pet.satisfaction.max(0) as u16,
                attack: stats.as_ref().map(|v| v.pet_attack as u16).unwrap_or(0),
                defence: stats.as_ref().map(|v| v.pet_defence as u16).unwrap_or(0),
                resistance: stats.as_ref().map(|v| v.pet_res as u16).unwrap_or(0),
                items: pet.items.clone(),
            };

            let pet_ui = build_pet_spawn_packet(&spawn_info);
            session.send_packet(&pet_ui).await?;

            tracing::info!(
                "[sid={}] PET_SPAWN nid={} template_sid={} model_spid={} template_type={} template_is_monster={} nation={} size={} name={} zone={} pos={:.1}/{:.1}/{:.1}",
                sid,
                runtime_nid,
                PET_RUNTIME_TEMPLATE_SID,
                pet.pid,
                visual_template.npc_type,
                visual_template.is_monster,
                instance.nation,
                pet.size,
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
                        pet.pending_attack_skill_id = 0;
                    }
                }
            });

            // Send mode change confirmation
            let resp = build_pet_mode_change_packet(mode);
            session.send_packet(&resp).await?;

            info!(
                sid = session.session_id(),
                mode,
                "PET_EVENT mode_changed"
            );
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

    let mut pet_data: Option<(u32, u16)> = None;

    world.update_session(sid, |h| {
        if let Some(pet) = h.pet_data.as_mut() {
            pet_data = Some((pet.index, pet.nid));
            pet.nid = 0;
            pet.attack_started = false;
            pet.attack_target_id = -1;
            pet.pending_attack_skill_id = 0;
        }
    });

    if let Some((index, nid)) = pet_data {
        if nid != 0 {
            world.kill_npc(nid as u32);
        }
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
    /// Persisted pet equipment slots sent with the pet window.
    pub items: [UserItemSlot; PET_INVENTORY_TOTAL as usize],
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

    // Pet inventory: the four persistent equipment slots.
    //      + sRemainingRentalTime(u16) + u32(0) + nExpirationTime(u32)
    for item in &info.items {
        resp.write_u32(item.item_id); // nNum
        resp.write_u16(item.durability.max(0) as u16); // sDuration
        resp.write_u16(item.count); // sCount
        resp.write_u8(item.flag); // bFlag
        resp.write_u16(item.remaining_rental_minutes()); // sRemainingRentalTime
        resp.write_u32(0); // padding
        resp.write_u32(item.expire_time); // nExpirationTime
    }

    resp
}

/// Apply database rows to the four runtime pet slots. Invalid/duplicate slot
/// indexes are ignored so malformed old rows cannot corrupt a summon packet.
pub(crate) fn apply_persistent_pet_items(
    pet: &mut PetState,
    rows: Vec<ko_db::models::PetUserItemRow>,
) {
    for row in rows {
        let Ok(slot) = usize::try_from(row.slot_index) else {
            continue;
        };
        if slot >= pet.items.len() || row.item_id <= 0 || row.count <= 0 {
            continue;
        }
        pet.items[slot] = UserItemSlot {
            item_id: row.item_id as u32,
            durability: row.durability,
            count: row.count as u16,
            flag: row.flag.clamp(0, u8::MAX as i16) as u8,
            original_flag: row.original_flag.clamp(0, u8::MAX as i16) as u8,
            serial_num: row.serial_num.max(0) as u64,
            expire_time: row.expire_time.max(0) as u32,
        };
    }
}

/// Persist the currently equipped pet items immediately after a pet inventory
/// movement. This is intentionally independent of the character logout path.
pub(crate) async fn save_pet_items(session: &ClientSession) {
    let world = session.world().clone();
    let sid = session.session_id();
    let pool = session.pool().clone();
    let snapshot = world
        .with_session(sid, |h| {
            h.pet_data
                .as_ref()
                .map(|pet| (pet.serial_id, pet.items.clone()))
        })
        .flatten();
    let Some((serial_id, items)) = snapshot else {
        return;
    };
    if serial_id == 0 {
        return;
    }
    let rows: Vec<ko_db::models::PetUserItemRow> = items
        .iter()
        .enumerate()
        .map(|(slot, item)| ko_db::models::PetUserItemRow {
            n_pet_serial_id: serial_id as i64,
            slot_index: slot as i16,
            item_id: item.item_id as i32,
            durability: item.durability,
            count: item.count as i16,
            flag: item.flag as i16,
            original_flag: item.original_flag as i16,
            serial_num: item.serial_num as i64,
            expire_time: item.expire_time as i32,
        })
        .collect();
    let repo = ko_db::repositories::pet::PetRepository::new(&pool);
    match repo.save_pet_items(serial_id as i64, &rows).await {
        Ok(()) => {
            let saved: Vec<String> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.item_id != 0 && item.count > 0)
                .map(|(slot, item)| format!("{}:{}x{}", slot, item.item_id, item.count))
                .collect();
            tracing::info!(
                "PET_EQUIPMENT_SAVE serial={} items=[{}]",
                serial_id,
                saved.join(",")
            );
        }
        Err(e) => tracing::warn!("pet item save failed serial={}: {}", serial_id, e),
    }
}

/// Re-send a spawned pet using its current persisted appearance. Transform
/// recipes change PID/size, which cannot be applied to an existing client NPC
/// without an OUT/IN refresh.
pub(crate) fn refresh_pet_appearance(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
) {
    let Some((pet, event_room)) = world
        .with_session(sid, |h| h.pet_data.clone().map(|p| (p, h.event_room)))
        .flatten()
    else {
        return;
    };
    if pet.nid == 0 {
        return;
    }
    let Some(instance) = world.get_npc_instance(pet.nid as u32) else {
        return;
    };
    let Some(template) = world.get_npc_template(instance.proto_id, instance.is_monster) else {
        return;
    };
    let out = build_npc_inout(crate::npc::NPC_OUT, &instance, &template);
    let mut appearance = template.as_ref().clone();
    appearance.pid = pet.pid.max(1);
    appearance.size = pet.size.max(1);
    let input = build_npc_inout(NPC_IN, &instance, &appearance);
    world.broadcast_to_3x3(
        instance.zone_id,
        instance.region_x,
        instance.region_z,
        Arc::new(out),
        None,
        event_room,
    );
    world.broadcast_to_3x3(
        instance.zone_id,
        instance.region_x,
        instance.region_z,
        Arc::new(input),
        None,
        event_room,
    );
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
            items: Default::default(),
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
    fn test_2625_pet_skill_request_reads_full_width_unit_ids() {
        let mut packet = Packet::new(Opcode::WizPet as u8);
        packet.write_u8(PET_USE_SKILL);
        packet.write_u8(1); // PetMagicMng mode
        packet.write_u32(490_321);
        packet.write_u32(105_001);
        packet.write_u32(106_777);
        // PetMagicMng.cpp:156 appends six more dwords after the IDs.
        for value in [0, 0, 0, 0, 0, 0] {
            packet.write_u32(value);
        }

        let mut reader = PacketReader::new(&packet.data);
        assert_eq!(reader.read_u8(), Some(PET_USE_SKILL));
        let request = read_pet_skill_request(&mut reader).unwrap();
        assert_eq!(request.sub_code, 1);
        assert_eq!(request.skill_id, 490_321);
        assert_eq!(request.caster_id, 105_001);
        assert_eq!(request.target_id, 106_777);
        assert!(request.target_id > u16::MAX as u32);
        assert_eq!(reader.remaining(), 24);
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
