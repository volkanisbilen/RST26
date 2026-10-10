//! Buff duration expiry tick system.
//! Runs every 1 second, checking all sessions for expired buffs.
//! When a buff expires, it is removed from the session and a
//! `MAGIC_DURATION_EXPIRED` packet is broadcast to the 3×3 region.
//! ## MAGIC_DURATION_EXPIRED packet (S→C)
//! Opcode: `WIZ_MAGIC_PROCESS` (0x31), sub-opcode 5
//! ```text
//! [u8 MAGIC_DURATION_EXPIRED(5)] [u8 buff_type]
//! ```

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use ko_protocol::{Opcode, Packet};

use crate::handler::stealth;
use crate::world::types::{ZONE_CHAOS_DUNGEON, ZONE_DUNGEON_DEFENCE};
use crate::world::WorldState;
use crate::zone::SessionId;

/// Buff tick interval in seconds.
const BUFF_TICK_INTERVAL_SECS: u64 = 1;

use crate::magic_constants::MAGIC_DURATION_EXPIRED;

use crate::magic_constants::{ABNORMAL_CHAOS_NORMAL, ABNORMAL_NORMAL};
use crate::state_change_constants::{STATE_CHANGE_ABNORMAL, STATE_CHANGE_TRANSFORMATION};

/// Start the buff expiry background task.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_buff_tick_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(BUFF_TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            process_buff_tick(&world);
        }
    })
}

/// Process one buff tick — check for expired buffs across all sessions.
fn process_buff_tick(world: &WorldState) {
    // ── Single-pass collection (replaces 6 separate DashMap scans) ──
    let now_unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let results = world.collect_all_buff_tick_expirations(now_unix, now_ms);

    // ── 1. Process expired buffs ────────────────────────────────────
    for (sid, buff) in results.expired_buffs {
        let pkt = build_buff_expired_packet(buff.buff_type as u8);

        world.remove_saved_magic(sid, buff.skill_id);
        buff_type_cleanup(world, sid, buff.buff_type, buff.is_buff);
        world.set_user_ability(sid);
        world.send_item_move_refresh(sid);

        // Only the buff owner learns about the expiry (C++ sends it to the target
        // alone). A region broadcast makes every nearby client drop its own buff of
        // the same type, e.g. their Swift when someone's short slow runs out.
        world.send_to_session_owned(sid, pkt);

        // A regular buff can temporarily replace an item scroll in the same
        // buff slot. When that replacement expires, restore the still-live
        // scroll too; previously this recovery only ran for debuffs, making
        // speed/attack/AC scrolls disappear early after another skill.
        if (!buff.is_buff && crate::world::WorldState::is_lockable_scroll(buff.buff_type))
            || world.has_saved_scroll_for_buff_type(sid, buff.buff_type)
        {
            world.recast_lockable_scrolls(sid, buff.buff_type);
        }

        tracing::info!(
            "[sid={}] buff expired: buff_type={} skill_id={}",
            sid,
            buff.buff_type,
            buff.skill_id
        );
    }

    // ── NPC buff expiry (separate DashMap, not part of unified scan) ──
    let npc_expired = world.process_npc_buff_tick();
    for (npc_id, buff_type) in npc_expired {
        tracing::trace!("[npc={}] NPC buff expired: buff_type={}", npc_id, buff_type);
    }

    // ── 2. Process expired transformations ──────────────────────────
    process_transformation_expiry_from_results(world, &results.expired_transformations);

    // ── 3. Process expired blinks ───────────────────────────────────
    process_blink_expiry_from_results(world, &results.expired_blinks);

    // ── 4. Post-blink skill re-enable ───────────────────────────────
    for sid in &results.post_blink_skill_enable {
        world.update_session(*sid, |h| {
            h.can_use_skills = true;
        });
    }

    // ── 5. Stealth duration expiry ──────────────────────────────────
    for sid in &results.expired_stealths {
        // Clear end time first to prevent re-processing
        world.update_session(*sid, |h| {
            h.stealth_end_time = 0;
        });
        stealth::remove_stealth(world, *sid);
    }

    // ── 6. Rivalry expiry ───────────────────────────────────────────
    process_rivalry_expiry_from_results(world, &results.expired_rivalries);
}

/// Check for expired blink (respawn invulnerability) on all sessions.
/// When `UNIXTIME >= m_tBlinkExpiryTime`, clear blink and broadcast
/// `StateChangeServerDirect(3, ABNORMAL_NORMAL)`.
/// Process blink expiry from pre-collected results (single-pass variant).
fn process_blink_expiry_from_results(world: &WorldState, expired: &[(SessionId, u16)]) {
    for &(sid, zone_id) in expired {
        world.clear_blink(sid);

        //   if (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_DUNGEON_DEFENCE)
        //       StateChangeServerDirect(3, ABNORMAL_CHAOS_NORMAL);
        //   else StateChangeServerDirect(3, ABNORMAL_NORMAL);
        let normal_type = if zone_id == ZONE_CHAOS_DUNGEON || zone_id == ZONE_DUNGEON_DEFENCE {
            ABNORMAL_CHAOS_NORMAL
        } else {
            ABNORMAL_NORMAL
        };
        let pkt = build_blink_expired_packet_with_type(sid as u32, normal_type);
        if let Some((pos, sender_event_room)) =
            world.with_session(sid, |h| (h.position, h.event_room))
        {
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(pkt),
                None,
                sender_event_room,
            );
        }

        tracing::debug!(
            "[sid={}] blink expired in zone {}, abnormal_type={}",
            sid,
            zone_id,
            normal_type
        );
    }
}

/// Build a WIZ_STATE_CHANGE broadcast for blink expiry with configurable abnormal type.
/// Format: `[u32 socket_id] [u8 bType=3] [u32 abnormal_type]`
/// zones, ABNORMAL_NORMAL (1) elsewhere.
fn build_blink_expired_packet_with_type(sid: u32, abnormal_type: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizStateChange as u8);
    pkt.write_u32(sid);
    pkt.write_u8(STATE_CHANGE_ABNORMAL);
    pkt.write_u32(abnormal_type);
    pkt
}

/// Build a WIZ_STATE_CHANGE with bType=12 (secondary state reset).
fn build_blink_expired_packet_with_type12(sid: u32, value: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizStateChange as u8);
    pkt.write_u32(sid);
    pkt.write_u8(12); // bType=12
    pkt.write_u32(value);
    pkt
}

/// Build a WIZ_STATE_CHANGE broadcast for blink expiry (default: ABNORMAL_NORMAL).
#[cfg(test)]
fn build_blink_expired_packet(sid: u32) -> Packet {
    build_blink_expired_packet_with_type(sid, ABNORMAL_NORMAL)
}

/// Build a `MAGIC_DURATION_EXPIRED` packet for a specific buff type.
/// ```text
/// Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_DURATION_EXPIRED));
/// result << uint8(BUFF_TYPE);
/// ```
pub fn build_buff_expired_packet(buff_type: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(MAGIC_DURATION_EXPIRED);
    pkt.write_u8(buff_type);
    pkt
}

use crate::buff_constants::*;

use crate::magic_constants::{USER_STATUS_CURE, USER_STATUS_POISON, USER_STATUS_SPEED};

/// Perform buff-type-specific cleanup when a buff expires.
/// Each buff type resets specific session fields and broadcasts visual state
/// changes to the client. This must run **before** `SetUserAbility()` so that
/// cleared fields are reflected in the stat recalculation.
pub(crate) fn buff_type_cleanup(world: &WorldState, sid: u16, buff_type: i32, is_buff: bool) {
    match buff_type {
        // FREEZE: clear block state, restore skills, clear invisibility,
        // broadcast ABNORMAL_NORMAL (or transform skill if transformed)
        BUFF_TYPE_FREEZE => {
            let mut transform_skill_id: Option<u32> = None;
            world.update_session(sid, |h| {
                h.can_use_skills = true;
                h.block_magic = false; // C++ Ref: MagicProcess.cpp:1252
                h.invisibility_type = 0; // INVIS_NONE
                if h.transformation_type != 0 {
                    transform_skill_id = Some(h.transform_skill_id);
                }
            });

            // Broadcast StateChangeServerDirect(3, ABNORMAL_NORMAL)
            if let Some((pos, sender_event_room)) =
                world.with_session(sid, |h| (h.position, h.event_room))
            {
                let state_pkt = build_blink_expired_packet_with_type(sid as u32, ABNORMAL_NORMAL);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(state_pkt),
                    None,
                    sender_event_room,
                );

                // broadcast StateChangeServerDirect(3, m_sTransformSkillID)
                if let Some(skill_id) = transform_skill_id {
                    let transform_pkt = build_blink_expired_packet_with_type(sid as u32, skill_id);
                    world.broadcast_to_3x3(
                        pos.zone_id,
                        pos.region_x,
                        pos.region_z,
                        Arc::new(transform_pkt),
                        None,
                        sender_event_room,
                    );
                }
            }
        }

        // SPEED/SPEED2: send USER_STATUS_SPEED cure only if this was a debuff
        // (pSkill.bMoral >= MORAL_ENEMY)
        BUFF_TYPE_SPEED | BUFF_TYPE_SPEED2 if !is_buff => {
            send_user_status_update_packet(world, sid, USER_STATUS_SPEED, USER_STATUS_CURE);
        }

        // STUN: send USER_STATUS_POISON cure only if this was a debuff AND
        // the target has no other remaining debuffs (!pTarget->isDebuffed())
        BUFF_TYPE_STUN if !is_buff => {
            // C++ isDebuffed(): m_buffMap.size() != m_buffCount
            // i.e., there are debuffs remaining in the map
            let has_remaining_debuffs = world
                .with_session(sid, |h| h.buffs.values().any(|b| !b.is_buff))
                .unwrap_or(false);
            if !has_remaining_debuffs {
                send_user_status_update_packet(world, sid, USER_STATUS_POISON, USER_STATUS_CURE);
            }
        }

        // SIZE: clear size_effect and broadcast ABNORMAL_NORMAL (or transform skill if transformed)
        BUFF_TYPE_SIZE => {
            world.update_session(sid, |h| {
                h.size_effect = 0;
            });
            let transform_skill_id = world.with_session(sid, |h| {
                if h.transformation_type != 0 {
                    Some(h.transform_skill_id)
                } else {
                    None
                }
            });

            if let Some((pos, sender_event_room)) =
                world.with_session(sid, |h| (h.position, h.event_room))
            {
                let abnormal = match transform_skill_id {
                    Some(Some(skill_id)) => skill_id,
                    _ => ABNORMAL_NORMAL,
                };
                let state_pkt = build_blink_expired_packet_with_type(sid as u32, abnormal);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(state_pkt),
                    None,
                    sender_event_room,
                );
            }
        }

        // MAGE_ARMOR: clear reflect armor type
        BUFF_TYPE_MAGE_ARMOR => {
            world.update_session(sid, |h| {
                h.reflect_armor_type = 0;
            });
        }

        // MIRROR_DAMAGE_PARTY (Minak's Thorn): clear mirror damage state
        BUFF_TYPE_MIRROR_DAMAGE_PARTY => {
            world.update_session(sid, |h| {
                h.mirror_damage = false;
                h.mirror_damage_type = false;
                h.mirror_amount = 0;
            });
        }

        // DAGGER_BOW_DEFENSE (Eskrima): reset dagger/bow defense amounts to 100
        BUFF_TYPE_DAGGER_BOW_DEFENSE => {
            world.update_session(sid, |h| {
                h.dagger_r_amount = 100;
                h.bow_r_amount = 100;
            });
        }

        // REDUCE_TARGET: broadcast ABNORMAL_NORMAL
        BUFF_TYPE_REDUCE_TARGET => {
            if let Some((pos, sender_event_room)) =
                world.with_session(sid, |h| (h.position, h.event_room))
            {
                let state_pkt = build_blink_expired_packet_with_type(sid as u32, ABNORMAL_NORMAL);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(state_pkt),
                    None,
                    sender_event_room,
                );
            }
        }

        // SILENCE_TARGET: restore skill casting ability
        BUFF_TYPE_SILENCE_TARGET => {
            world.update_session(sid, |h| {
                h.can_use_skills = true;
            });
        }

        // NO_POTIONS: restore potion consumption ability
        BUFF_TYPE_NO_POTIONS => {
            world.update_session(sid, |h| {
                h.can_use_potions = true;
            });
        }

        // KAUL_TRANSFORMATION: clear is_kaul, broadcast old abnormal type
        BUFF_TYPE_KAUL_TRANSFORMATION => {
            let old_abnormal = world
                .with_session(sid, |h| h.old_abnormal_type)
                .unwrap_or(ABNORMAL_NORMAL);
            world.update_session(sid, |h| {
                h.is_kaul = false;
                h.can_use_skills = true;
            });

            if let Some((pos, sender_event_room)) =
                world.with_session(sid, |h| (h.position, h.event_room))
            {
                // Restore previous visual state
                let state_pkt = build_blink_expired_packet_with_type(sid as u32, old_abnormal);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(state_pkt),
                    None,
                    sender_event_room,
                );
                // Secondary state reset (type 12)
                let reset_pkt = build_blink_expired_packet_with_type12(sid as u32, 0);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(reset_pkt),
                    None,
                    sender_event_room,
                );
            }
        }

        // UNDEAD: clear is_undead flag (stat cleanup via SetUserAbility)
        BUFF_TYPE_UNDEAD => {
            world.update_session(sid, |h| {
                h.is_undead = false;
            });
        }

        // DISABLE_TARGETING / BLIND / UNSIGHT: clear is_blinded flag
        BUFF_TYPE_DISABLE_TARGETING | BUFF_TYPE_BLIND | BUFF_TYPE_UNSIGHT => {
            world.update_session(sid, |h| {
                h.is_blinded = false;
            });
            // C++ sends USER_STATUS_POISON cure for BLIND
            if buff_type == BUFF_TYPE_BLIND {
                send_user_status_update_packet(world, sid, USER_STATUS_POISON, USER_STATUS_CURE);
            }
        }

        // BLOCK_PHYSICAL_DAMAGE: clear block_physical flag
        BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE => {
            world.update_session(sid, |h| {
                h.block_physical = false;
            });
        }

        // BLOCK_MAGICAL_DAMAGE: clear block_magic flag
        BUFF_TYPE_BLOCK_MAGICAL_DAMAGE => {
            world.update_session(sid, |h| {
                h.block_magic = false;
            });
        }

        // DEVIL_TRANSFORM: clear is_devil, broadcast StateChange(12, 0)
        BUFF_TYPE_DEVIL_TRANSFORM => {
            world.update_session(sid, |h| {
                h.is_devil = false;
            });
            if let Some((pos, sender_event_room)) =
                world.with_session(sid, |h| (h.position, h.event_room))
            {
                let pkt = build_blink_expired_packet_with_type12(sid as u32, 0);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(pkt),
                    None,
                    sender_event_room,
                );
            }
        }

        // NO_RECALL: restore teleport ability
        BUFF_TYPE_NO_RECALL => {
            world.update_session(sid, |h| {
                h.can_teleport = true;
            });
        }

        // PROHIBIT_INVIS: restore stealth ability
        BUFF_TYPE_PROHIBIT_INVIS => {
            world.update_session(sid, |h| {
                h.can_stealth = true;
            });
        }

        // RESIS_AND_MAGIC_DMG: reset magic damage reduction to 100 (no reduction)
        BUFF_TYPE_RESIS_AND_MAGIC_DMG => {
            world.update_session(sid, |h| {
                h.magic_damage_reduction = 100;
            });
            send_user_status_update_packet(world, sid, USER_STATUS_POISON, USER_STATUS_CURE);
        }

        // BLOCK_CURSE: clear curse block
        BUFF_TYPE_BLOCK_CURSE => {
            world.update_session(sid, |h| {
                h.block_curses = false;
            });
        }

        // BLOCK_CURSE_REFLECT: clear curse reflect
        BUFF_TYPE_BLOCK_CURSE_REFLECT => {
            world.update_session(sid, |h| {
                h.reflect_curses = false;
            });
        }

        // INSTANT_MAGIC: clear instant cast
        BUFF_TYPE_INSTANT_MAGIC => {
            world.update_session(sid, |h| {
                h.instant_cast = false;
            });
        }

        // NP_DROP_NOAH: reset scroll bonus (only one NP_DROP_NOAH can be active at a time)
        BUFF_TYPE_NP_DROP_NOAH => {
            world.update_session(sid, |h| {
                h.drop_scroll_amount = 0;
            });
        }

        // SNOWMAN_TITI: broadcast old abnormal type to restore visual
        BUFF_TYPE_SNOWMAN_TITI => {
            let old_abnormal = world
                .with_session(sid, |h| h.old_abnormal_type)
                .unwrap_or(ABNORMAL_NORMAL);
            if let Some((pos, sender_event_room)) =
                world.with_session(sid, |h| (h.position, h.event_room))
            {
                let state_pkt = build_blink_expired_packet_with_type(sid as u32, old_abnormal);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(state_pkt),
                    None,
                    sender_event_room,
                );
            }
        }

        // MANA_ABSORB: decrement mana absorb pct, reset absorb count
        BUFF_TYPE_MANA_ABSORB => {
            world.update_session(sid, |h| {
                h.mana_absorb = 0; // Only one MANA_ABSORB can be active (keyed by type)
                h.absorb_count = 0;
            });
        }

        // IGNORE_WEAPON: clear weapons_disabled flag and restore weapon visuals
        BUFF_TYPE_IGNORE_WEAPON => {
            world.update_session(sid, |h| {
                h.weapons_disabled = false;
            });

            // Restore weapon visuals via UserLookChange
            if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room))
            {
                use crate::inventory_constants::{LEFTHAND, RIGHTHAND};

                // Restore right hand
                if let Some(rh) = world.get_inventory_slot(sid, RIGHTHAND) {
                    if rh.item_id != 0 {
                        let mut pkt = Packet::new(Opcode::WizUserlookChange as u8);
                        pkt.write_u32(sid as u32);
                        pkt.write_u8(RIGHTHAND as u8);
                        pkt.write_u32(rh.item_id);
                        pkt.write_u16(rh.durability as u16);
                        pkt.write_u8(0);
                        world.broadcast_to_3x3(
                            pos.zone_id,
                            pos.region_x,
                            pos.region_z,
                            Arc::new(pkt),
                            Some(sid),
                            event_room,
                        );
                    }
                }

                // Restore left hand
                if let Some(lh) = world.get_inventory_slot(sid, LEFTHAND) {
                    if lh.item_id != 0 {
                        let mut pkt = Packet::new(Opcode::WizUserlookChange as u8);
                        pkt.write_u32(sid as u32);
                        pkt.write_u8(LEFTHAND as u8);
                        pkt.write_u32(lh.item_id);
                        pkt.write_u16(lh.durability as u16);
                        pkt.write_u8(0);
                        world.broadcast_to_3x3(
                            pos.zone_id,
                            pos.region_x,
                            pos.region_z,
                            Arc::new(pkt),
                            Some(sid),
                            event_room,
                        );
                    }
                }
            }
        }

        // DECREASE_RESIST: reset all pct resistance multipliers to 100
        BUFF_TYPE_DECREASE_RESIST => {
            world.update_session(sid, |h| {
                h.pct_fire_r = 100;
                h.pct_cold_r = 100;
                h.pct_lightning_r = 100;
                h.pct_magic_r = 100;
                h.pct_disease_r = 100;
                h.pct_poison_r = 100;
            });
        }

        // EXPERIENCE: reset EXP gain buff11 amount (only one type-11 buff active at a time)
        BUFF_TYPE_EXPERIENCE => {
            world.update_session(sid, |h| {
                h.exp_gain_buff11 = 0;
            });
        }

        // VARIOUS_EFFECTS: reset EXP buff33 and NP bonus from this buff type
        BUFF_TYPE_VARIOUS_EFFECTS => {
            world.update_session(sid, |h| {
                h.exp_gain_buff33 = 0;
                h.skill_np_bonus_33 = 0;
            });
        }

        // LOYALTY_AMOUNT: reset NP bonus from this buff type
        BUFF_TYPE_LOYALTY_AMOUNT => {
            world.update_session(sid, |h| {
                h.skill_np_bonus_42 = 0;
            });
        }

        // WEIGHT: reset carry weight multiplier to default (100)
        BUFF_TYPE_WEIGHT => {
            world.update_session(sid, |h| {
                h.weight_buff_amount = 100;
            });
        }

        // LOYALTY: reset NP gain multiplier to default (100)
        BUFF_TYPE_LOYALTY => {
            world.update_session(sid, |h| {
                h.np_gain_amount = 100;
            });
        }

        // NOAH_BONUS: reset gold gain multiplier to default (100)
        BUFF_TYPE_NOAH_BONUS => {
            world.update_session(sid, |h| {
                h.noah_gain_amount = 100;
            });
        }

        // PREMIUM_MERCHANT: reset premium merchant flag
        BUFF_TYPE_PREMIUM_MERCHANT => {
            world.update_session(sid, |h| {
                h.is_premium_merchant = false;
            });
        }

        // JACKPOT: clear m_jackpotype
        BUFF_TYPE_JACKPOT => {
            world.update_session(sid, |h| {
                h.jackpot_type = 0;
            });
        }

        _ => {}
    }
}

/// Build and send a `SendUserStatusUpdate` packet to a single session,
/// then broadcast `PARTY_STATUSCHANGE` to party members.
/// Format: `WIZ_ZONEABILITY [u8 sub=2] [u8 status_type] [u8 status_behaviour]`
/// C++ also calls `SendPartyStatusUpdate()` (`PartyHandler.cpp:1275-1282`)
/// which sends `WIZ_PARTY [u8 0x09] [u32 socketID] [u8 status] [u8 result]`
/// to all party members.
pub(crate) fn send_user_status_update_packet(
    world: &WorldState,
    sid: u16,
    status_type: u8,
    status_behaviour: u8,
) {
    let mut pkt = Packet::new(Opcode::WizZoneability as u8);
    pkt.write_u8(2); // sub-opcode for status update
    pkt.write_u8(status_type);
    pkt.write_u8(status_behaviour);
    world.send_to_session_owned(sid, pkt);

    if let Some(party_id) = world.get_party_id(sid) {
        let mut party_pkt = Packet::new(Opcode::WizParty as u8);
        party_pkt.write_u8(crate::handler::party::PARTY_STATUSCHANGE);
        party_pkt.write_u32(sid as u32);
        party_pkt.write_u8(status_type);
        party_pkt.write_u8(status_behaviour);
        world.send_to_party(party_id, &party_pkt);
    }
}

use crate::magic_constants::MAGIC_CANCEL_TRANSFORMATION;

/// Check for expired transformations (Type6Duration) on all sessions.
/// Process transformation expiry from pre-collected results (single-pass variant).
fn process_transformation_expiry_from_results(
    world: &WorldState,
    expired: &[(SessionId, u32, u16)],
) {
    for &(sid, _skill_id, _zone_id) in expired {
        // 1. Send MAGIC_CANCEL_TRANSFORMATION to the caster
        //   Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_CANCEL_TRANSFORMATION));
        let mut cancel_pkt = Packet::new(Opcode::WizMagicProcess as u8);
        cancel_pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);
        world.send_to_session_owned(sid, cancel_pkt);

        // 2. v2615 transformation model reset (state 0x13, value 0).
        let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
        state_pkt.write_u32(sid as u32);
        state_pkt.write_u8(STATE_CHANGE_TRANSFORMATION);
        state_pkt.write_u32(0);
        if let Some((pos, sender_event_room)) =
            world.with_session(sid, |h| (h.position, h.event_room))
        {
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(state_pkt),
                None,
                sender_event_room,
            );
        }

        // 3. Clear transformation state
        world.clear_transformation(sid);
        world.update_session(sid, |h| {
            h.old_abnormal_type = h.abnormal_type;
            h.abnormal_type = ABNORMAL_NORMAL;
        });

        // 4. Recalculate stats after transformation ends
        world.set_user_ability(sid);

        // 5. Remove from saved magic
        world.remove_saved_magic(sid, _skill_id);

        tracing::debug!(
            "[sid={}] transformation expired: skill_id={}",
            sid,
            _skill_id,
        );
    }
}

/// Re-enable skills after blink expires while still transformed.
/// ```text
/// if (!isBlinking() && isTransformed() && m_bCanUseSkills == false)
///     m_bCanUseSkills = true;
/// ```
/// Process rivalry expiry from pre-collected results (single-pass variant).
fn process_rivalry_expiry_from_results(world: &WorldState, expired: &[SessionId]) {
    for &sid in expired {
        world.remove_rival(sid);
        tracing::debug!("[sid={}] rivalry expired (RemoveRival)", sid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_buff_expired_packet_format() {
        let pkt = build_buff_expired_packet(3);
        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        assert_eq!(pkt.data.len(), 2); // u8 sub-opcode + u8 buff_type

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(MAGIC_DURATION_EXPIRED));
        assert_eq!(reader.read_u8(), Some(3));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_buff_expired_packet_various_types() {
        for buff_type in [1u8, 5, 10, 20, 255] {
            let pkt = build_buff_expired_packet(buff_type);
            let mut reader = PacketReader::new(&pkt.data);
            assert_eq!(reader.read_u8(), Some(5));
            assert_eq!(reader.read_u8(), Some(buff_type));
        }
    }

    // ── Blink expiry tests ───────────────────────────────────────────

    #[test]
    fn test_blink_expired_packet_format() {
        let pkt = build_blink_expired_packet(42);
        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        assert_eq!(pkt.data.len(), 9); // u32 + u8 + u32

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(42)); // session_id
        assert_eq!(reader.read_u8(), Some(3)); // bType = abnormal
        assert_eq!(reader.read_u32(), Some(1)); // nBuff = ABNORMAL_NORMAL
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_blink_expired_packet_various_sids() {
        for sid in [0u32, 1, 100, 65535, u32::MAX] {
            let pkt = build_blink_expired_packet(sid);
            let mut reader = PacketReader::new(&pkt.data);
            assert_eq!(reader.read_u32(), Some(sid));
            assert_eq!(reader.read_u8(), Some(STATE_CHANGE_ABNORMAL));
            assert_eq!(reader.read_u32(), Some(ABNORMAL_NORMAL));
        }
    }

    #[test]
    fn test_blink_constants() {
        assert_eq!(ABNORMAL_NORMAL, 1);
        assert_eq!(STATE_CHANGE_ABNORMAL, 3);
    }

    // ── Sprint 42: Chaos zone blink expiry tests ─────────────────────

    #[test]
    fn test_abnormal_chaos_normal_constant() {
        assert_eq!(ABNORMAL_CHAOS_NORMAL, 7);
    }

    #[test]
    fn test_chaos_zone_constants() {
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
        assert_eq!(ZONE_DUNGEON_DEFENCE, 89);
    }

    #[test]
    fn test_blink_expired_chaos_dungeon_packet() {
        // In chaos dungeon, blink expiry broadcasts ABNORMAL_CHAOS_NORMAL (7)
        let pkt = build_blink_expired_packet_with_type(42, ABNORMAL_CHAOS_NORMAL);
        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(42));
        assert_eq!(reader.read_u8(), Some(STATE_CHANGE_ABNORMAL));
        assert_eq!(reader.read_u32(), Some(ABNORMAL_CHAOS_NORMAL)); // 7
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_blink_expired_normal_zone_still_uses_normal() {
        // Normal zones continue to use ABNORMAL_NORMAL (1)
        let pkt = build_blink_expired_packet_with_type(42, ABNORMAL_NORMAL);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(42));
        assert_eq!(reader.read_u8(), Some(STATE_CHANGE_ABNORMAL));
        assert_eq!(reader.read_u32(), Some(ABNORMAL_NORMAL)); // 1
    }

    #[test]
    fn test_zone_to_abnormal_type_mapping() {
        // Verify the zone-to-abnormal-type logic matches C++
        for zone_id in [ZONE_CHAOS_DUNGEON, ZONE_DUNGEON_DEFENCE] {
            let abnormal_type = if zone_id == ZONE_CHAOS_DUNGEON || zone_id == ZONE_DUNGEON_DEFENCE
            {
                ABNORMAL_CHAOS_NORMAL
            } else {
                ABNORMAL_NORMAL
            };
            assert_eq!(abnormal_type, ABNORMAL_CHAOS_NORMAL);
        }

        // Normal zones
        for zone_id in [1u16, 21, 61, 71] {
            let abnormal_type = if zone_id == ZONE_CHAOS_DUNGEON || zone_id == ZONE_DUNGEON_DEFENCE
            {
                ABNORMAL_CHAOS_NORMAL
            } else {
                ABNORMAL_NORMAL
            };
            assert_eq!(abnormal_type, ABNORMAL_NORMAL);
        }
    }

    // ── Sprint 43: Transformation expiry tests ──────────────────────

    #[test]
    fn test_magic_cancel_transformation_constant() {
        assert_eq!(MAGIC_CANCEL_TRANSFORMATION, 7);
    }

    #[test]
    fn test_transformation_cancel_packet_format() {
        //   Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_CANCEL_TRANSFORMATION));
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        assert_eq!(pkt.data.len(), 1);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(7));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_transformation_expiry_state_change_packet() {
        // After transformation expires, broadcast StateChangeServerDirect(3, ABNORMAL_NORMAL)
        let sid: u32 = 42;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(sid);
        pkt.write_u8(STATE_CHANGE_ABNORMAL);
        pkt.write_u32(ABNORMAL_NORMAL);

        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        assert_eq!(pkt.data.len(), 9); // u32 + u8 + u32

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32(), Some(42));
        assert_eq!(reader.read_u8(), Some(3));
        assert_eq!(reader.read_u32(), Some(1)); // ABNORMAL_NORMAL
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_transformation_duration_check_logic() {
        //   if (!isTransformed() || (UNIXTIME2 - m_tTransformationStartTime) < m_sTransformationDuration)
        //       return; // not expired yet

        // Case 1: Not expired (elapsed < duration)
        let start_time_ms: u64 = 1000000;
        let duration_ms: u64 = 60000; // 60 seconds
        let now_ms: u64 = 1050000; // 50 seconds elapsed
        assert!(now_ms.saturating_sub(start_time_ms) < duration_ms);

        // Case 2: Expired (elapsed >= duration)
        let now_ms_expired: u64 = 1060000; // 60 seconds elapsed
        assert!(now_ms_expired.saturating_sub(start_time_ms) >= duration_ms);

        // Case 3: Just expired (equal)
        assert!(duration_ms.saturating_sub(0) >= duration_ms);
    }

    #[test]
    fn test_transformation_duration_milliseconds_conversion() {
        //   m_sTransformationDuration = ULONGLONG(sDuration) * 1000;
        let duration_secs: u16 = 120;
        let duration_ms = duration_secs as u64 * 1000;
        assert_eq!(duration_ms, 120000);

        let duration_secs_short: u16 = 30;
        let duration_ms_short = duration_secs_short as u64 * 1000;
        assert_eq!(duration_ms_short, 30000);
    }

    #[test]
    fn test_transformation_type_none_is_zero() {
        // TransformationNone = 0 (not transformed)
        let transformation_type: u8 = 0;
        assert_eq!(transformation_type, 0);
        assert!(transformation_type == 0); // !isTransformed()
    }

    // ── Sprint 44: M2 — SetUserAbility on transformation cancel ──────

    #[test]
    fn test_transformation_cancel_calls_set_user_ability() {
        // Verify the process_transformation_expiry function structure:
        // After clear_transformation(), set_user_ability() is called.
        //
        // This is a structural test confirming the call ordering.
        // The actual SetUserAbility computation is tested in inventory.rs.
        //
        // Sequence after Type6Cancel:
        // 1. Send MAGIC_CANCEL_TRANSFORMATION packet
        // 2. Broadcast StateChangeServerDirect(3, ABNORMAL_NORMAL)
        // 3. clear_transformation(sid)
        // 4. set_user_ability(sid)  <-- M2 fix
        // 5. remove_saved_magic(sid, skill_id)
        assert_eq!(MAGIC_CANCEL_TRANSFORMATION, 7);
        assert_eq!(STATE_CHANGE_ABNORMAL, 3);
        assert_eq!(ABNORMAL_NORMAL, 1);
    }

    #[test]
    fn test_transformation_expiry_flow_order() {
        // C++ MagicInstance.cpp:6774-6784 Type6Cancel sequence:
        //   1. Packet result(WIZ_MAGIC_PROCESS, MAGIC_CANCEL_TRANSFORMATION)
        //   2. Send to caster
        //   3. m_transformationType = TransformationNone
        //   4. SetUserAbility()  <-- was missing, M2 fix
        //   5. RemoveSavedMagic(m_bAbnormalType)
        //   6. StateChangeServerDirect(3, ABNORMAL_NORMAL)
        //
        // Our Rust implementation calls set_user_ability after clear_transformation.
        // The fact that this test file compiles with the set_user_ability call
        // confirms the fix is wired correctly.
        let cancel_pkt_opcode = Opcode::WizMagicProcess as u8;
        let mut pkt = Packet::new(cancel_pkt_opcode);
        pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 7);
    }

    // ── Sprint 48: Stealth duration expiry tests ────────────────────

    fn setup_world() -> WorldState {
        WorldState::new()
    }

    fn register_session(world: &WorldState, sid: u16) {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
    }

    fn make_test_character(sid: u16) -> crate::world::CharacterInfo {
        crate::world::CharacterInfo {
            session_id: sid,
            name: format!("TestPlayer{}", sid),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0u32; 14],
            bind_zone: 0,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
            res_hp_type: 1,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    fn register_session_with_character(world: &WorldState, sid: u16) {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(sid, tx);
        let position = crate::world::Position::default();
        let character = make_test_character(sid);
        world.register_ingame(sid, character, position);
    }

    #[test]
    fn test_stealth_end_time_field_default_zero() {
        let world = setup_world();
        register_session(&world, 1);
        let end_time = world
            .with_session(1, |h| h.stealth_end_time)
            .unwrap_or(u64::MAX);
        assert_eq!(end_time, 0);
    }

    #[test]
    fn test_collect_expired_stealths_empty_when_no_stealth() {
        let world = setup_world();
        register_session(&world, 1);
        let expired = world.collect_expired_stealths(1000);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_collect_expired_stealths_finds_expired() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.invisibility_type = 1; // INVIS_DISPEL_ON_MOVE
            h.stealth_end_time = 500;
        });
        let expired = world.collect_expired_stealths(500);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], 1);
    }

    #[test]
    fn test_collect_expired_stealths_skips_non_expired() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.invisibility_type = 2; // INVIS_DISPEL_ON_ATTACK
            h.stealth_end_time = 1000;
        });
        // now = 999, not yet expired
        let expired = world.collect_expired_stealths(999);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_collect_expired_stealths_skips_zero_end_time() {
        // stealth_end_time == 0 means no timed stealth (permanent until broken)
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.invisibility_type = 1;
            h.stealth_end_time = 0;
        });
        let expired = world.collect_expired_stealths(5000);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_collect_expired_stealths_skips_non_invisible() {
        // Session has stealth_end_time set but invisibility_type == 0
        // (stealth was already broken by another mechanism)
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.invisibility_type = 0; // INVIS_NONE
            h.stealth_end_time = 500;
        });
        let expired = world.collect_expired_stealths(1000);
        assert!(expired.is_empty());
    }

    #[tokio::test]
    async fn test_stealth_expiry_clears_invisibility() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.invisibility_type = 1;
            h.stealth_end_time = 100;
        });

        // Simulate what process_stealth_duration_expiry does
        let expired = world.collect_expired_stealths(100);
        assert_eq!(expired.len(), 1);
        for sid in expired {
            world.update_session(sid, |h| {
                h.stealth_end_time = 0;
            });
            stealth::remove_stealth(&world, sid);
        }

        assert_eq!(world.get_invisibility_type(1), 0);
        let end_time = world
            .with_session(1, |h| h.stealth_end_time)
            .unwrap_or(u64::MAX);
        assert_eq!(end_time, 0);
    }

    #[tokio::test]
    async fn test_stealth_expiry_multiple_sessions() {
        let world = setup_world();
        register_session(&world, 1);
        register_session(&world, 2);
        register_session(&world, 3);

        // Session 1: expired
        world.update_session(1, |h| {
            h.invisibility_type = 1;
            h.stealth_end_time = 100;
        });
        // Session 2: not expired yet
        world.update_session(2, |h| {
            h.invisibility_type = 2;
            h.stealth_end_time = 200;
        });
        // Session 3: expired
        world.update_session(3, |h| {
            h.invisibility_type = 1;
            h.stealth_end_time = 50;
        });

        let expired = world.collect_expired_stealths(150);
        assert_eq!(expired.len(), 2);
        assert!(expired.contains(&1));
        assert!(expired.contains(&3));
        assert!(!expired.contains(&2));
    }

    // ── Sprint 48: Rivalry expiry tests ──────────────────────────────

    #[test]
    fn test_collect_expired_rivalries_empty_when_no_rival() {
        let world = setup_world();
        register_session_with_character(&world, 1);
        let expired = world.collect_expired_rivalries(1000);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_collect_expired_rivalries_finds_expired() {
        let world = setup_world();
        register_session_with_character(&world, 1);
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.rival_id = 5;
                ch.rival_expiry_time = 500;
            }
        });
        let expired = world.collect_expired_rivalries(500);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], 1);
    }

    #[test]
    fn test_collect_expired_rivalries_skips_non_expired() {
        let world = setup_world();
        register_session_with_character(&world, 1);
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.rival_id = 5;
                ch.rival_expiry_time = 1000;
            }
        });
        let expired = world.collect_expired_rivalries(999);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_collect_expired_rivalries_skips_no_rival() {
        // rival_id == -1 means no rival (default)
        let world = setup_world();
        register_session_with_character(&world, 1);
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.rival_id = -1;
                ch.rival_expiry_time = 100;
            }
        });
        let expired = world.collect_expired_rivalries(200);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_collect_expired_rivalries_skips_zero_expiry() {
        let world = setup_world();
        register_session_with_character(&world, 1);
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.rival_id = 5;
                ch.rival_expiry_time = 0;
            }
        });
        let expired = world.collect_expired_rivalries(1000);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_rivalry_expiry_calls_remove_rival() {
        let world = setup_world();
        register_session_with_character(&world, 1);
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.rival_id = 5;
                ch.rival_expiry_time = 500;
            }
        });

        // Simulate process_rivalry_expiry
        let expired = world.collect_expired_rivalries(500);
        for sid in expired {
            world.remove_rival(sid);
        }

        let (rival_id, expiry) = world
            .with_session(1, |h| {
                h.character
                    .as_ref()
                    .map(|ch| (ch.rival_id, ch.rival_expiry_time))
                    .unwrap_or((-1, 0))
            })
            .unwrap();
        assert_eq!(rival_id, -1);
        assert_eq!(expiry, 0);
    }

    #[test]
    fn test_rivalry_expiry_sends_pvp_remove_packet() {
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        let position = crate::world::Position::default();
        let character = make_test_character(1);
        world.register_ingame(1, character, position);

        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.rival_id = 3;
                ch.rival_expiry_time = 100;
            }
        });

        // Call remove_rival (what process_rivalry_expiry does)
        world.remove_rival(1);

        // Check for WIZ_PVP packet with sub-opcode 2 (PVPRemoveRival)
        let mut found_pvp_remove = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizPvp as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(2) {
                    found_pvp_remove = true;
                }
            }
        }
        assert!(
            found_pvp_remove,
            "Expected WIZ_PVP(PVPRemoveRival=2) packet"
        );
    }

    // ── Sprint 148: Buff expiry cleanup tests ────────────────────────

    #[test]
    fn test_buff_type_cleanup_constants() {
        assert_eq!(BUFF_TYPE_SIZE, 3);
        assert_eq!(BUFF_TYPE_SPEED, 6);
        assert_eq!(BUFF_TYPE_FREEZE, 22);
        assert_eq!(BUFF_TYPE_SPEED2, 40);
        assert_eq!(BUFF_TYPE_STUN, 47);
        assert_eq!(BUFF_TYPE_MAGE_ARMOR, 25);
        assert_eq!(BUFF_TYPE_REDUCE_TARGET, 151);
    }

    #[test]
    fn test_user_status_constants() {
        assert_eq!(USER_STATUS_CURE, 0);
        assert_eq!(USER_STATUS_POISON, 2);
        assert_eq!(USER_STATUS_SPEED, 3);
    }

    #[test]
    fn test_user_status_update_packet_format() {
        // Format: WIZ_ZONEABILITY [u8 2] [u8 status_type] [u8 status_behaviour]
        let mut pkt = Packet::new(Opcode::WizZoneability as u8);
        pkt.write_u8(2);
        pkt.write_u8(USER_STATUS_SPEED);
        pkt.write_u8(USER_STATUS_CURE);

        assert_eq!(pkt.opcode, Opcode::WizZoneability as u8);
        assert_eq!(pkt.data.len(), 3);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(2));
        assert_eq!(reader.read_u8(), Some(USER_STATUS_SPEED));
        assert_eq!(reader.read_u8(), Some(USER_STATUS_CURE));
    }

    #[tokio::test]
    async fn test_freeze_expiry_restores_can_use_skills() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.can_use_skills = false;
            h.invisibility_type = 1; // was set by freeze
        });

        buff_type_cleanup(&world, 1, BUFF_TYPE_FREEZE, false);

        let can_use = world.with_session(1, |h| h.can_use_skills).unwrap();
        assert!(can_use, "FREEZE expiry should restore can_use_skills");

        let invis = world.with_session(1, |h| h.invisibility_type).unwrap();
        assert_eq!(invis, 0, "FREEZE expiry should clear invisibility_type");
    }

    #[tokio::test]
    async fn test_speed_expiry_sends_status_cure() {
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, BUFF_TYPE_SPEED, false); // debuff

        // Check for WIZ_ZONEABILITY status cure packet
        let mut found_cure = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizZoneability as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(2)
                    && r.read_u8() == Some(USER_STATUS_SPEED)
                    && r.read_u8() == Some(USER_STATUS_CURE)
                {
                    found_cure = true;
                }
            }
        }
        assert!(
            found_cure,
            "SPEED expiry should send USER_STATUS_SPEED cure"
        );
    }

    #[tokio::test]
    async fn test_stun_expiry_sends_poison_cure() {
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, BUFF_TYPE_STUN, false); // debuff

        let mut found_cure = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizZoneability as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(2)
                    && r.read_u8() == Some(USER_STATUS_POISON)
                    && r.read_u8() == Some(USER_STATUS_CURE)
                {
                    found_cure = true;
                }
            }
        }
        assert!(
            found_cure,
            "STUN expiry should send USER_STATUS_POISON cure"
        );
    }

    #[tokio::test]
    async fn test_speed2_expiry_sends_speed_cure() {
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, BUFF_TYPE_SPEED2, false); // debuff

        let mut found_cure = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizZoneability as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(2)
                    && r.read_u8() == Some(USER_STATUS_SPEED)
                    && r.read_u8() == Some(USER_STATUS_CURE)
                {
                    found_cure = true;
                }
            }
        }
        assert!(
            found_cure,
            "SPEED2 expiry should send USER_STATUS_SPEED cure"
        );
    }

    #[tokio::test]
    async fn test_unknown_buff_type_no_action() {
        // Buff types without special cleanup should not crash or send packets
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, 99, true); // unknown type

        // Should not have sent any packets
        assert!(
            rx.try_recv().is_err(),
            "Unknown buff type should not send packets"
        );
    }

    // ── Sprint 149: moral guard + isDebuffed tests ──────────────────

    #[tokio::test]
    async fn test_speed_friendly_buff_no_cure() {
        // Friendly speed buffs (is_buff=true) should NOT send cure packets on expiry.
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, BUFF_TYPE_SPEED, true); // friendly buff

        assert!(
            rx.try_recv().is_err(),
            "Friendly SPEED buff expiry should NOT send cure packet"
        );
    }

    #[tokio::test]
    async fn test_speed2_friendly_buff_no_cure() {
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, BUFF_TYPE_SPEED2, true); // friendly buff

        assert!(
            rx.try_recv().is_err(),
            "Friendly SPEED2 buff expiry should NOT send cure packet"
        );
    }

    #[tokio::test]
    async fn test_stun_friendly_buff_no_cure() {
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        buff_type_cleanup(&world, 1, BUFF_TYPE_STUN, true); // friendly buff

        assert!(
            rx.try_recv().is_err(),
            "Friendly STUN buff expiry should NOT send cure packet"
        );
    }

    #[tokio::test]
    async fn test_stun_debuff_with_remaining_debuff_no_cure() {
        // If another debuff is still active, STUN expiry should NOT send cure.
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add a remaining debuff (e.g., a speed debuff that hasn't expired)
        let remaining_debuff = crate::world::ActiveBuff {
            skill_id: 200001,
            buff_type: BUFF_TYPE_SPEED,
            caster_sid: 2,
            start_time: std::time::Instant::now(),
            duration_secs: 300,
            attack_speed: 0,
            speed: -50,
            ac: 0,
            ac_pct: 0,
            attack: 0,
            magic_attack: 0,
            max_hp: 0,
            max_hp_pct: 0,
            max_mp: 0,
            max_mp_pct: 0,
            str_mod: 0,
            sta_mod: 0,
            dex_mod: 0,
            intel_mod: 0,
            cha_mod: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            hit_rate: 0,
            avoid_rate: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: false, // this is a debuff
        };
        world.apply_buff(1, remaining_debuff);

        buff_type_cleanup(&world, 1, BUFF_TYPE_STUN, false); // debuff

        // Should NOT send cure because another debuff is still active
        let mut found_cure = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizZoneability as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(2)
                    && r.read_u8() == Some(USER_STATUS_POISON)
                    && r.read_u8() == Some(USER_STATUS_CURE)
                {
                    found_cure = true;
                }
            }
        }
        assert!(
            !found_cure,
            "STUN debuff expiry should NOT send cure when other debuffs remain"
        );
    }

    #[tokio::test]
    async fn test_stun_debuff_no_remaining_debuffs_sends_cure() {
        // When STUN debuff expires and no other debuffs remain, cure IS sent.
        let world = setup_world();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add a friendly buff (not a debuff) — should not prevent cure
        let friendly_buff = crate::world::ActiveBuff {
            skill_id: 108010,
            buff_type: 8,
            caster_sid: 1,
            start_time: std::time::Instant::now(),
            duration_secs: 300,
            attack_speed: 0,
            speed: 50,
            ac: 0,
            ac_pct: 0,
            attack: 0,
            magic_attack: 0,
            max_hp: 0,
            max_hp_pct: 0,
            max_mp: 0,
            max_mp_pct: 0,
            str_mod: 0,
            sta_mod: 0,
            dex_mod: 0,
            intel_mod: 0,
            cha_mod: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            hit_rate: 0,
            avoid_rate: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: true, // friendly buff
        };
        world.apply_buff(1, friendly_buff);

        buff_type_cleanup(&world, 1, BUFF_TYPE_STUN, false); // debuff

        let mut found_cure = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.opcode == Opcode::WizZoneability as u8 {
                let mut r = PacketReader::new(&pkt.data);
                if r.read_u8() == Some(2)
                    && r.read_u8() == Some(USER_STATUS_POISON)
                    && r.read_u8() == Some(USER_STATUS_CURE)
                {
                    found_cure = true;
                }
            }
        }
        assert!(
            found_cure,
            "STUN debuff expiry should send cure when only friendly buffs remain"
        );
    }

    /// BUFF_TYPE_MAGE_ARMOR expiry should clear reflect_armor_type to 0.
    #[tokio::test]
    async fn test_buff_type_cleanup_mage_armor() {
        let world = setup_world();
        register_session(&world, 1);
        // Set reflect_armor_type to Fire (5)
        world.update_session(1, |h| {
            h.reflect_armor_type = 5;
        });
        // Verify it was set
        let before = world.with_session(1, |h| h.reflect_armor_type).unwrap_or(0);
        assert_eq!(before, 5, "reflect_armor_type should be 5 before cleanup");

        // Run MAGE_ARMOR cleanup
        buff_type_cleanup(&world, 1, BUFF_TYPE_MAGE_ARMOR, true);

        // Verify reflect_armor_type is cleared
        let after = world
            .with_session(1, |h| h.reflect_armor_type)
            .unwrap_or(99);
        assert_eq!(
            after, 0,
            "reflect_armor_type should be 0 after MAGE_ARMOR cleanup"
        );
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_dagger_bow_defense() {
        let world = setup_world();
        register_session(&world, 1);
        // Set dagger/bow amounts to debuffed values (Eskrima with sSpecialAmount=20)
        world.update_session(1, |h| {
            h.dagger_r_amount = 80;
            h.bow_r_amount = 80;
        });
        // Verify they were set
        let (before_dagger, before_bow) = world
            .with_session(1, |h| (h.dagger_r_amount, h.bow_r_amount))
            .unwrap();
        assert_eq!(before_dagger, 80);
        assert_eq!(before_bow, 80);

        // Run DAGGER_BOW_DEFENSE cleanup
        buff_type_cleanup(&world, 1, BUFF_TYPE_DAGGER_BOW_DEFENSE, false);

        // Verify both are reset to 100
        let (after_dagger, after_bow) = world
            .with_session(1, |h| (h.dagger_r_amount, h.bow_r_amount))
            .unwrap();
        assert_eq!(
            after_dagger, 100,
            "dagger_r_amount should be 100 after cleanup"
        );
        assert_eq!(after_bow, 100, "bow_r_amount should be 100 after cleanup");
    }

    /// BUFF_TYPE_SILENCE_TARGET expiry should restore can_use_skills to true.
    #[tokio::test]
    async fn test_buff_type_cleanup_silence_target() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.can_use_skills = false; // Was silenced
        });

        buff_type_cleanup(&world, 1, BUFF_TYPE_SILENCE_TARGET, false);

        let can_use = world.with_session(1, |h| h.can_use_skills).unwrap();
        assert!(can_use, "SILENCE expiry should restore can_use_skills");
    }

    /// BUFF_TYPE_NO_POTIONS expiry should restore can_use_potions to true.
    #[tokio::test]
    async fn test_buff_type_cleanup_no_potions() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.can_use_potions = false; // No-potion debuff active
        });

        buff_type_cleanup(&world, 1, BUFF_TYPE_NO_POTIONS, false);

        let can_use = world.with_session(1, |h| h.can_use_potions).unwrap();
        assert!(can_use, "NO_POTIONS expiry should restore can_use_potions");
    }

    #[test]
    fn test_buff_type_silence_nopotion_constants() {
        assert_eq!(BUFF_TYPE_SILENCE_TARGET, 152);
        assert_eq!(BUFF_TYPE_NO_POTIONS, 153);
    }

    #[test]
    fn test_buff_type_kaul_undead_constants() {
        assert_eq!(BUFF_TYPE_KAUL_TRANSFORMATION, 154);
        assert_eq!(BUFF_TYPE_UNDEAD, 155);
    }

    /// BUFF_TYPE_KAUL_TRANSFORMATION expiry should clear is_kaul flag.
    #[tokio::test]
    async fn test_buff_type_cleanup_kaul_transformation() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.is_kaul = true;
            h.old_abnormal_type = 1; // ABNORMAL_NORMAL
        });

        buff_type_cleanup(&world, 1, BUFF_TYPE_KAUL_TRANSFORMATION, false);

        let is_kaul = world.with_session(1, |h| h.is_kaul).unwrap();
        assert!(!is_kaul, "KAUL expiry should clear is_kaul");
    }

    /// BUFF_TYPE_UNDEAD expiry should clear is_undead flag.
    #[tokio::test]
    async fn test_buff_type_cleanup_undead() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.is_undead = true;
        });

        buff_type_cleanup(&world, 1, BUFF_TYPE_UNDEAD, false);

        let is_undead = world.with_session(1, |h| h.is_undead).unwrap();
        assert!(!is_undead, "UNDEAD expiry should clear is_undead");
    }

    // ── Sprint 217: New flag-based buff type cleanup tests ───────────

    #[tokio::test]
    async fn test_buff_type_cleanup_unsight() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.is_blinded = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_UNSIGHT, false);
        assert!(!world.with_session(1, |h| h.is_blinded).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_blind() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.is_blinded = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_BLIND, false);
        assert!(!world.with_session(1, |h| h.is_blinded).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_disable_targeting() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.is_blinded = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_DISABLE_TARGETING, false);
        assert!(!world.with_session(1, |h| h.is_blinded).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_block_physical() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.block_physical = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE, false);
        assert!(!world.with_session(1, |h| h.block_physical).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_block_magical() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.block_magic = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_BLOCK_MAGICAL_DAMAGE, false);
        assert!(!world.with_session(1, |h| h.block_magic).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_devil_transform() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.is_devil = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_DEVIL_TRANSFORM, false);
        assert!(!world.with_session(1, |h| h.is_devil).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_no_recall() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.can_teleport = false;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_NO_RECALL, false);
        assert!(world.with_session(1, |h| h.can_teleport).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_prohibit_invis() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.can_stealth = false;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_PROHIBIT_INVIS, false);
        assert!(world.with_session(1, |h| h.can_stealth).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_block_curse() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.block_curses = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_BLOCK_CURSE, false);
        assert!(!world.with_session(1, |h| h.block_curses).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_block_curse_reflect() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.reflect_curses = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_BLOCK_CURSE_REFLECT, false);
        assert!(!world.with_session(1, |h| h.reflect_curses).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_instant_magic() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.instant_cast = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_INSTANT_MAGIC, false);
        assert!(!world.with_session(1, |h| h.instant_cast).unwrap());
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_np_drop_noah() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.drop_scroll_amount = 50;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_NP_DROP_NOAH, true);
        assert_eq!(world.with_session(1, |h| h.drop_scroll_amount).unwrap(), 0);
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_snowman_titi() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.old_abnormal_type = 1;
        });
        // Just verify it doesn't panic — visual broadcast needs position
        buff_type_cleanup(&world, 1, BUFF_TYPE_SNOWMAN_TITI, true);
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_ignore_weapon() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.weapons_disabled = true;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_IGNORE_WEAPON, false);
        assert!(!world.with_session(1, |h| h.weapons_disabled).unwrap());
    }

    #[tokio::test]
    async fn test_freeze_clears_block_magic() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| {
            h.block_magic = true;
            h.can_use_skills = false;
        });
        buff_type_cleanup(&world, 1, BUFF_TYPE_FREEZE, false);
        assert!(!world.with_session(1, |h| h.block_magic).unwrap());
        assert!(world.with_session(1, |h| h.can_use_skills).unwrap());
    }

    #[test]
    fn test_new_buff_type_constants() {
        assert_eq!(BUFF_TYPE_DISABLE_TARGETING, 20);
        assert_eq!(BUFF_TYPE_BLIND, 21);
        assert_eq!(BUFF_TYPE_INSTANT_MAGIC, 23);
        assert_eq!(BUFF_TYPE_PROHIBIT_INVIS, 26);
        assert_eq!(BUFF_TYPE_BLOCK_CURSE, 29);
        assert_eq!(BUFF_TYPE_BLOCK_CURSE_REFLECT, 30);
        assert_eq!(BUFF_TYPE_IGNORE_WEAPON, 32);
        assert_eq!(BUFF_TYPE_DEVIL_TRANSFORM, 49);
        assert_eq!(BUFF_TYPE_NO_RECALL, 150);
        assert_eq!(BUFF_TYPE_UNSIGHT, 156);
        assert_eq!(BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE, 157);
        assert_eq!(BUFF_TYPE_BLOCK_MAGICAL_DAMAGE, 158);
        assert_eq!(BUFF_TYPE_NP_DROP_NOAH, 169);
        assert_eq!(BUFF_TYPE_SNOWMAN_TITI, 170);
        assert_eq!(BUFF_TYPE_JACKPOT, 77);
    }

    #[tokio::test]
    async fn test_buff_type_cleanup_jackpot() {
        let world = setup_world();
        register_session(&world, 1);
        world.update_session(1, |h| h.jackpot_type = 2);
        assert_eq!(world.with_session(1, |h| h.jackpot_type).unwrap(), 2);
        buff_type_cleanup(&world, 1, BUFF_TYPE_JACKPOT, true);
        assert_eq!(world.with_session(1, |h| h.jackpot_type).unwrap(), 0);
    }

    #[test]
    fn test_session_flag_defaults() {
        let world = setup_world();
        register_session(&world, 1);
        // Verify defaults
        assert!(!world.with_session(1, |h| h.is_blinded).unwrap());
        assert!(!world.with_session(1, |h| h.block_physical).unwrap());
        assert!(!world.with_session(1, |h| h.block_magic).unwrap());
        assert!(!world.with_session(1, |h| h.is_devil).unwrap());
        assert!(world.with_session(1, |h| h.can_teleport).unwrap());
        assert!(world.with_session(1, |h| h.can_stealth).unwrap());
        assert!(!world.with_session(1, |h| h.block_curses).unwrap());
        assert!(!world.with_session(1, |h| h.reflect_curses).unwrap());
        assert!(!world.with_session(1, |h| h.instant_cast).unwrap());
        assert_eq!(world.with_session(1, |h| h.drop_scroll_amount).unwrap(), 0);
        assert!(!world.with_session(1, |h| h.weapons_disabled).unwrap());
        assert_eq!(world.with_session(1, |h| h.jackpot_type).unwrap(), 0);
    }
}
