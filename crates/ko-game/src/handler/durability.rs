//! Combat durability system — item wear-out from attacking and defending.
//! When a player attacks, their weapon slots (RIGHTHAND, LEFTHAND) lose
//! `rand(2..=5)` durability. When a player takes damage, their armour
//! slots (HEAD, BREAST, LEG, GLOVE, FOOT) lose `rand(2..=5)` durability.
//! Special modes:
//! - `REPAIR_ALL`: restore full durability on all 7 combat slots
//! - `ACID_ALL`: reduce armour durability by an exact amount (magic)
//! - `UTC_ATTACK` / `UTC_DEFENCE`: reduce by an exact amount (under castle)
//! Visual thresholds: when durability crosses 70% or 30% boundaries,
//! `UserLookChange` is broadcast so nearby players see the worn equipment.
//! When durability reaches 0, the item is considered broken:
//! - `SendDurability(slot, 0)` notifies the client
//! - `set_user_ability` recalculates stats (broken items contribute nothing)
//! - `SendItemMove(1, 1)` refreshes the client's equipment view

use ko_protocol::{Opcode, Packet};
use rand::Rng;
use std::sync::Arc;

use crate::world::WorldState;
use crate::zone::SessionId;

// ── Equipment slot indices (from canonical inventory_constants, cast to u8
//    for WEAPON_SLOTS/ARMOUR_SLOTS/REPAIR_SLOTS arrays and send_durability API) ──
const HEAD: u8 = crate::inventory_constants::HEAD as u8;
const BREAST: u8 = crate::inventory_constants::BREAST as u8;
const RIGHTHAND: u8 = crate::inventory_constants::RIGHTHAND as u8;
const LEFTHAND: u8 = crate::inventory_constants::LEFTHAND as u8;
const LEG: u8 = crate::inventory_constants::LEG as u8;
const GLOVE: u8 = crate::inventory_constants::GLOVE as u8;
const FOOT: u8 = crate::inventory_constants::FOOT as u8;
const SLOT_MAX: u8 = crate::inventory_constants::SLOT_MAX as u8;

/// Item slot type: left-hand shield
const ITEM_SLOT_1H_LEFT_HAND: i32 = 2;

// ── Durability wear-out type constants (C++ GameDefine.h:1265-1270) ───────

/// Weapon wear (RIGHTHAND + LEFTHAND).
pub const WORE_TYPE_ATTACK: i32 = 0x01;
/// Armour wear (HEAD + BREAST + LEG + GLOVE + FOOT).
pub const WORE_TYPE_DEFENCE: i32 = 0x02;
/// Repair all 7 combat slots to full durability.
pub const WORE_TYPE_REPAIR_ALL: i32 = 0x03;
/// Acid: reduce armour durability by exact amount.
pub const WORE_TYPE_ACID_ALL: i32 = 0x04;
/// Under-castle attack wear (exact amount).
pub const WORE_TYPE_UTC_ATTACK: i32 = 0x05;
/// Under-castle defence wear (exact amount).
pub const WORE_TYPE_UTC_DEFENCE: i32 = 0x06;

/// Weapon slots that take damage on ATTACK / UTC_ATTACK.
const WEAPON_SLOTS: &[u8] = &[RIGHTHAND, LEFTHAND];
/// Armour slots that take damage on DEFENCE / ACID_ALL / UTC_DEFENCE.
const ARMOUR_SLOTS: &[u8] = &[HEAD, BREAST, LEG, GLOVE, FOOT];
/// All combat slots (weapons + armour) for REPAIR_ALL.
const REPAIR_SLOTS: &[u8] = &[RIGHTHAND, LEFTHAND, HEAD, BREAST, LEG, GLOVE, FOOT];

impl WorldState {
    /// Reduce (or repair) durability on a player's equipment slots.
    ///
    ///
    /// `wore_type` determines which slots are affected.
    /// `damage` is only used for `ACID_ALL`, `UTC_ATTACK`, and `UTC_DEFENCE` (exact amount);
    /// for `ATTACK` and `DEFENCE`, `rand(2..=5)` is used instead.
    pub fn item_wore_out(&self, sid: SessionId, wore_type: i32, damage: i32) {
        // Manes plays with a temporary client-side loadout; the real equipment stays untouched.
        if self.manes_survival_manager.is_participant_playing(sid) {
            return;
        }
        let worerate = match wore_type {
            WORE_TYPE_ACID_ALL | WORE_TYPE_UTC_ATTACK | WORE_TYPE_UTC_DEFENCE => damage,
            _ => {
                let mut rng = rand::thread_rng();
                rng.gen_range(2..=5)
            }
        };

        if worerate == 0 {
            return;
        }

        // Determine which slots to process
        let slots: &[u8] = match wore_type {
            WORE_TYPE_ATTACK => WEAPON_SLOTS,
            WORE_TYPE_DEFENCE => ARMOUR_SLOTS,
            WORE_TYPE_REPAIR_ALL => REPAIR_SLOTS,
            WORE_TYPE_ACID_ALL => ARMOUR_SLOTS,
            WORE_TYPE_UTC_ATTACK => WEAPON_SLOTS,
            WORE_TYPE_UTC_DEFENCE => ARMOUR_SLOTS,
            _ => return,
        };

        let inventory = self.get_inventory(sid);
        let mut any_broken = false;
        let mut recalc_needed = false;

        // Pre-fetch position + event_room once for broadcast (avoids per-slot DashMap reads)
        let broadcast_ctx = self.with_session(sid, |h| (h.position, h.event_room));

        for &slot in slots {
            let idx = slot as usize;
            let item_slot = match inventory.get(idx) {
                Some(s) if s.item_id != 0 => s,
                _ => continue,
            };

            let item_template = match self.get_item(item_slot.item_id) {
                Some(t) => t,
                None => continue,
            };

            // Items whose template durability is 1 (or less) are non-degradable
            // by design; wearing them by 2-5 per hit/death would break them on
            // the first event and turn them red.
            if wore_type != WORE_TYPE_REPAIR_ALL && item_template.duration.unwrap_or(0) <= 1 {
                continue;
            }

            // Skip if item is already broken (durability <= 0) and not repairing
            if item_slot.durability <= 0 && wore_type != WORE_TYPE_REPAIR_ALL {
                continue;
            }

            // For ATTACK/UTC_ATTACK: skip shields in weapon slots
            if matches!(wore_type, WORE_TYPE_ATTACK | WORE_TYPE_UTC_ATTACK)
                && (slot == LEFTHAND || slot == RIGHTHAND)
                && item_template.slot.unwrap_or(-1) == ITEM_SLOT_1H_LEFT_HAND
            {
                continue;
            }

            // ── REPAIR_ALL: restore full durability ─────────────────────
            if wore_type == WORE_TYPE_REPAIR_ALL {
                let max_dur = item_template.duration.unwrap_or(0);
                self.update_session(sid, |h| {
                    if let Some(s) = h.inventory.get_mut(idx) {
                        s.durability = max_dur;
                    }
                });
                self.send_durability(sid, slot, max_dur as u16);
                if let Some((ref pos, event_room)) = broadcast_ctx {
                    self.send_user_look_change(
                        sid,
                        slot,
                        item_slot.item_id,
                        max_dur as u16,
                        pos,
                        event_room,
                    );
                }
                recalc_needed = true;
                continue;
            }

            // ── Normal wear: reduce durability ──────────────────────────
            let max_dur = item_template.duration.unwrap_or(1).max(1) as f64;
            let before_percent = ((item_slot.durability as f64 / max_dur) * 100.0) as i32;

            let new_dur = if worerate > item_slot.durability as i32 {
                0i16
            } else {
                item_slot.durability - worerate as i16
            };

            // Update durability in session inventory
            self.update_session(sid, |h| {
                if let Some(s) = h.inventory.get_mut(idx) {
                    s.durability = new_dur.max(0);
                }
            });

            if new_dur <= 0 {
                self.send_durability(sid, slot, 0);
                any_broken = true;
                continue;
            }

            // Send durability update for non-broken items
            self.send_durability(sid, slot, new_dur as u16);

            let cur_percent = ((new_dur as f64 / max_dur) * 100.0) as i32;

            // Check if we crossed a 5% boundary
            if (cur_percent / 5) != (before_percent / 5) {
                // Trigger visual change at 65-69% or 25-29% thresholds
                if (65..70).contains(&cur_percent) || (25..30).contains(&cur_percent) {
                    if let Some((ref pos, event_room)) = broadcast_ctx {
                        self.send_user_look_change(
                            sid,
                            slot,
                            item_slot.item_id,
                            new_dur as u16,
                            pos,
                            event_room,
                        );
                    }
                }
            }
        }

        if any_broken {
            self.set_user_ability(sid);
            self.send_item_move_refresh(sid);
        } else if recalc_needed {
            // REPAIR_ALL also needs stat recalculation
            self.set_user_ability(sid);
        }
    }

    /// Send WIZ_DURATION packet to notify client of durability change.
    ///
    ///
    /// Packet format: `[opcode=0x38] [u8 slot] [u16 durability]`
    fn send_durability(&self, sid: SessionId, slot: u8, durability: u16) {
        let mut pkt = Packet::new(Opcode::WizDuration as u8);
        pkt.write_u8(slot);
        pkt.write_u16(durability);
        self.send_to_session_owned(sid, pkt);
    }

    /// Send WIZ_USERLOOK_CHANGE to nearby players when equipment appearance changes.
    ///
    ///
    /// `pos` and `event_room` are pre-fetched by the caller to avoid per-slot
    /// DashMap reads inside a loop.
    fn send_user_look_change(
        &self,
        sid: SessionId,
        slot: u8,
        item_id: u32,
        durability: u16,
        pos: &crate::world::Position,
        event_room: u16,
    ) {
        if slot >= SLOT_MAX {
            return;
        }

        // Skip accessories (earring=91, necklace=92, ring=93, belt=94)
        if item_id != 0 {
            if let Some(item) = self.get_item(item_id) {
                let kind = item.kind.unwrap_or(0);
                if matches!(kind, 91..=94) {
                    return;
                }
            }
        }

        let mut pkt = Packet::new(Opcode::WizUserlookChange as u8);
        pkt.write_u32(sid as u32);
        pkt.write_u8(slot);
        pkt.write_u32(item_id);
        pkt.write_u16(durability);
        pkt.write_u8(0); // reserved

        self.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            Some(sid),
            event_room,
        );
    }

    /// Send WIZ_ITEM_MOVE(1,1) to refresh the client's equipment display.
    ///
    ///
    /// This tells the client to re-read its equipment stats, typically
    /// called when an item breaks (durability reaches 0).
    ///
    /// Performance: snapshots all buff aggregates in a single DashMap read
    /// instead of 16 separate lock acquisitions.
    pub fn send_item_move_refresh(&self, sid: SessionId) {
        // ── Single DashMap read via with_session: snapshot everything ─────
        struct BuffSnapshot {
            max_hp: u16,
            max_mp: u16,
            attack_amount: i32,
            ac_amount: i32,
            ac_pct: i32,
            ac_sour: i32,
            elem_r_add: [i32; 7], // [0]=unused, [1]=fire..[6]=poison
            elem_r_pct: [i32; 7], // percentage multipliers (default 100)
        }

        let Some((snap, stats)) = self
            .with_session(sid, |handle| {
                let ch = match handle.character.as_ref() {
                    Some(c) => c,
                    None => return None,
                };

                // Single-pass buff aggregation
                let mut attack_sum = 0i32;
                let mut ac_amount = 0i32;
                let mut ac_pct_mod = 0i32;
                let mut ac_sour = 0i32;
                let mut elem_add = [0i32; 7];
                for b in handle.buffs.values() {
                    // attack_amount: exclude BUFF_TYPE_DAMAGE_DOUBLE (19)
                    if b.buff_type != 19 && b.attack != 0 {
                        attack_sum += b.attack - 100;
                    }
                    // ac_amount: exclude BUFF_TYPE_WEAPON_AC (14)
                    if b.buff_type != 14 {
                        ac_amount = ac_amount.saturating_add(b.ac);
                    }
                    // ac_pct: exclude BUFF_TYPE_WEAPON_AC (14)
                    if b.ac_pct != 0 && b.buff_type != 14 {
                        ac_pct_mod += b.ac_pct - 100;
                    }
                    ac_sour = ac_sour.saturating_add(b.ac_sour);
                    // Elemental resistance adds
                    elem_add[1] = elem_add[1].saturating_add(b.fire_r);
                    elem_add[2] = elem_add[2].saturating_add(b.cold_r);
                    elem_add[3] = elem_add[3].saturating_add(b.lightning_r);
                    elem_add[4] = elem_add[4].saturating_add(b.magic_r);
                    elem_add[5] = elem_add[5].saturating_add(b.disease_r);
                    elem_add[6] = elem_add[6].saturating_add(b.poison_r);
                }

                // Resistance percentage from session fields
                let elem_pct = [
                    100i32,
                    handle.pct_fire_r as i32,
                    handle.pct_cold_r as i32,
                    handle.pct_lightning_r as i32,
                    handle.pct_magic_r as i32,
                    handle.pct_disease_r as i32,
                    handle.pct_poison_r as i32,
                ];

                let snap = BuffSnapshot {
                    max_hp: ch.max_hp as u16,
                    max_mp: ch.max_mp as u16,
                    attack_amount: (100 + attack_sum).max(1),
                    ac_amount,
                    ac_pct: 100 + ac_pct_mod,
                    ac_sour,
                    elem_r_add: elem_add,
                    elem_r_pct: elem_pct,
                };
                Some((snap, handle.equipped_stats.clone()))
            })
            .flatten()
        else {
            return;
        }; // DashMap lock released.

        // ── Compute final values from snapshot (no locks held) ───────────
        let total_ac = ((stats.total_ac as i32 * snap.ac_pct / 100) + snap.ac_amount - snap.ac_sour)
            .max(0) as u16;
        let mut total_hit = (stats.total_hit as u32 * snap.attack_amount as u32 / 100) as u16;
        if let Some(position) = self.get_position(sid) {
            if crate::systems::manes_survival::ZONES_MANES_SURVIVAL.contains(&position.zone_id) {
                if let Some(progress) = self.manes_survival_manager.progress(sid) {
                    total_hit = (u32::from(total_hit)
                        * u32::from(crate::systems::manes_survival::attack_scale_per_mille(
                            progress.level,
                        ))
                        / 1_000)
                        .clamp(1, u32::from(u16::MAX)) as u16;
                }
            }
        }

        let res_bonus = stats.resistance_bonus as i32;
        let compute_res = |base: i16, attr: usize| -> u16 {
            ((base as i32 + snap.elem_r_add[attr] + res_bonus) * snap.elem_r_pct[attr] / 100).max(0)
                as u16
        };

        let mut pkt = Packet::new(Opcode::WizItemMove as u8);
        pkt.write_u8(1); // command
        pkt.write_u8(2); // sniffer verified: original server ALWAYS sends sub=2 (876/876 packets)
        pkt.write_u16(total_hit);
        pkt.write_u16(total_ac);
        pkt.write_u32(stats.max_weight);
        pkt.write_u8(0); // reserved
        pkt.write_u8(0); // reserved
        pkt.write_u16(snap.max_hp);
        pkt.write_u16(snap.max_mp);
        pkt.write_i16(stats.stat_bonuses[0]); // STR bonus
        pkt.write_i16(stats.stat_bonuses[1]); // STA bonus
        pkt.write_i16(stats.stat_bonuses[2]); // DEX bonus
        pkt.write_i16(stats.stat_bonuses[3]); // INT bonus
        pkt.write_i16(stats.stat_bonuses[4]); // CHA bonus
        pkt.write_u16(compute_res(stats.fire_r, 1));
        pkt.write_u16(compute_res(stats.cold_r, 2));
        pkt.write_u16(compute_res(stats.lightning_r, 3));
        pkt.write_u16(compute_res(stats.magic_r, 4));
        pkt.write_u16(compute_res(stats.disease_r, 5));
        pkt.write_u16(compute_res(stats.poison_r, 6));
        self.send_to_session_owned(sid, pkt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{UserItemSlot, WorldState};
    use ko_db::models::Item;
    use tokio::sync::mpsc;

    /// Helper: create a WorldState with a registered session.
    fn setup_world() -> (WorldState, SessionId) {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        (world, 1)
    }

    /// Helper: create a test item template with known durability.
    fn make_item(num: i32, slot: i32, duration: i16, kind: i32) -> Item {
        Item {
            num,
            extension: None,
            str_name: None,
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            kind: Some(kind),
            slot: Some(slot),
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: None,
            duration: Some(duration),
            buy_price: None,
            sell_price: None,
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: None,
            effect1: None,
            effect2: None,
            req_level: None,
            req_level_max: None,
            req_rank: None,
            req_title: None,
            req_str: None,
            req_sta: None,
            req_dex: None,
            req_intel: None,
            req_cha: None,
            selling_group: None,
            item_type: None,
            hitrate: None,
            evasionrate: None,
            dagger_ac: None,
            jamadar_ac: None,
            sword_ac: None,
            club_ac: None,
            axe_ac: None,
            spear_ac: None,
            bow_ac: None,
            fire_damage: None,
            ice_damage: None,
            lightning_damage: None,
            poison_damage: None,
            hp_drain: None,
            mp_damage: None,
            mp_drain: None,
            mirror_damage: None,
            droprate: None,
            str_b: None,
            sta_b: None,
            dex_b: None,
            intel_b: None,
            cha_b: None,
            max_hp_b: None,
            max_mp_b: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            poison_r: None,
            curse_r: None,
            item_class: None,
            np_buy_price: None,
            bound: None,
            mace_ac: None,
            by_grade: None,
            drop_notice: None,
            upgrade_notice: None,
        }
    }

    /// Helper: ensure inventory has at least SLOT_MAX slots, then equip an item.
    fn equip_item(world: &WorldState, sid: SessionId, slot: u8, item_id: u32, durability: i16) {
        world.update_session(sid, |h| {
            // Ensure inventory is large enough
            if h.inventory.len() < SLOT_MAX as usize {
                h.inventory
                    .resize(SLOT_MAX as usize, UserItemSlot::default());
            }
            h.inventory[slot as usize] = UserItemSlot {
                item_id,
                durability,
                count: 1,
                flag: 0,
                original_flag: 0,
                serial_num: 1,
                expire_time: 0,
            };
        });
    }

    #[tokio::test]
    async fn test_attack_wear_reduces_weapon_durability() {
        let (world, sid) = setup_world();
        let sword_id = 120001;
        // slot=1 = ITEM_SLOT_1H_RIGHT (right-hand weapon)
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        equip_item(&world, sid, RIGHTHAND, sword_id, 1000);

        world.item_wore_out(sid, WORE_TYPE_ATTACK, 0);

        let inv = world.get_inventory(sid);
        let dur = inv[RIGHTHAND as usize].durability;
        // Should be reduced by rand(2..=5), so between 995 and 998
        assert!(
            (995..=998).contains(&dur),
            "durability should be 995-998, got {}",
            dur
        );
    }

    #[tokio::test]
    async fn test_defence_wear_reduces_armour_durability() {
        let (world, sid) = setup_world();
        // Equip armour in all 5 armour slots
        let helmet_id = 200001u32;
        let breast_id = 200002u32;
        let leg_id = 200003u32;
        let glove_id = 200004u32;
        let foot_id = 200005u32;

        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 500, 71));
        world.insert_item(breast_id, make_item(breast_id as i32, 5, 500, 72));
        world.insert_item(leg_id, make_item(leg_id as i32, 6, 500, 73));
        world.insert_item(glove_id, make_item(glove_id as i32, 8, 500, 74));
        world.insert_item(foot_id, make_item(foot_id as i32, 9, 500, 75));

        equip_item(&world, sid, HEAD, helmet_id, 500);
        equip_item(&world, sid, BREAST, breast_id, 500);
        equip_item(&world, sid, LEG, leg_id, 500);
        equip_item(&world, sid, GLOVE, glove_id, 500);
        equip_item(&world, sid, FOOT, foot_id, 500);

        world.item_wore_out(sid, WORE_TYPE_DEFENCE, 0);

        let inv = world.get_inventory(sid);
        for &slot in ARMOUR_SLOTS {
            let dur = inv[slot as usize].durability;
            assert!(
                (495..=498).contains(&dur),
                "slot {} durability should be 495-498, got {}",
                slot,
                dur
            );
        }
    }

    #[tokio::test]
    async fn test_attack_wear_skips_shield_in_lefthand() {
        let (world, sid) = setup_world();
        // Shield has slot type ITEM_SLOT_1H_LEFT_HAND = 2
        let shield_id = 150001u32;
        world.insert_item(
            shield_id,
            make_item(shield_id as i32, ITEM_SLOT_1H_LEFT_HAND, 500, 61),
        );
        equip_item(&world, sid, LEFTHAND, shield_id, 500);

        world.item_wore_out(sid, WORE_TYPE_ATTACK, 0);

        let inv = world.get_inventory(sid);
        // Shield should NOT be damaged by ATTACK type
        assert_eq!(inv[LEFTHAND as usize].durability, 500);
    }

    #[tokio::test]
    async fn test_item_breaks_at_zero_durability() {
        let (world, sid) = setup_world();
        let sword_id = 120002u32;
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        // Give weapon only 3 durability, will break with rand(2..=5)
        equip_item(&world, sid, RIGHTHAND, sword_id, 3);

        world.item_wore_out(sid, WORE_TYPE_ATTACK, 0);

        let inv = world.get_inventory(sid);
        let dur = inv[RIGHTHAND as usize].durability;
        // rand(2..=5), so durability can be 0 (if wore 3,4,5) or 1 (if wore 2)
        assert!(dur <= 1, "durability should be 0 or 1, got {}", dur);
    }

    #[tokio::test]
    async fn test_already_broken_item_skipped() {
        let (world, sid) = setup_world();
        let sword_id = 120003u32;
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        equip_item(&world, sid, RIGHTHAND, sword_id, 0);

        world.item_wore_out(sid, WORE_TYPE_ATTACK, 0);

        let inv = world.get_inventory(sid);
        // Already broken, should stay at 0
        assert_eq!(inv[RIGHTHAND as usize].durability, 0);
    }

    #[tokio::test]
    async fn test_repair_all_restores_full_durability() {
        let (world, sid) = setup_world();
        let sword_id = 120004u32;
        let helmet_id = 200010u32;
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 500, 71));
        equip_item(&world, sid, RIGHTHAND, sword_id, 100);
        equip_item(&world, sid, HEAD, helmet_id, 50);

        world.item_wore_out(sid, WORE_TYPE_REPAIR_ALL, i32::MAX);

        let inv = world.get_inventory(sid);
        assert_eq!(inv[RIGHTHAND as usize].durability, 1000);
        assert_eq!(inv[HEAD as usize].durability, 500);
    }

    #[tokio::test]
    async fn test_acid_all_reduces_armour_by_exact_amount() {
        let (world, sid) = setup_world();
        let helmet_id = 200020u32;
        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 500, 71));
        equip_item(&world, sid, HEAD, helmet_id, 500);

        world.item_wore_out(sid, WORE_TYPE_ACID_ALL, 100);

        let inv = world.get_inventory(sid);
        assert_eq!(inv[HEAD as usize].durability, 400);
    }

    #[tokio::test]
    async fn test_utc_attack_reduces_weapon_by_exact_amount() {
        let (world, sid) = setup_world();
        let sword_id = 120005u32;
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        equip_item(&world, sid, RIGHTHAND, sword_id, 500);

        world.item_wore_out(sid, WORE_TYPE_UTC_ATTACK, 50);

        let inv = world.get_inventory(sid);
        assert_eq!(inv[RIGHTHAND as usize].durability, 450);
    }

    #[tokio::test]
    async fn test_utc_defence_reduces_armour_by_exact_amount() {
        let (world, sid) = setup_world();
        let helmet_id = 200030u32;
        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 500, 71));
        equip_item(&world, sid, HEAD, helmet_id, 300);

        world.item_wore_out(sid, WORE_TYPE_UTC_DEFENCE, 25);

        let inv = world.get_inventory(sid);
        assert_eq!(inv[HEAD as usize].durability, 275);
    }

    #[tokio::test]
    async fn test_send_durability_packet_format() {
        let (world, sid) = setup_world();
        // Just verify the method doesn't panic — actual packet verification
        // would need a test channel listener.
        world.send_durability(sid, RIGHTHAND, 500);
    }

    #[tokio::test]
    async fn test_empty_slot_not_affected() {
        let (world, sid) = setup_world();
        // Don't equip anything
        world.item_wore_out(sid, WORE_TYPE_ATTACK, 0);
        // Should not panic
    }

    #[tokio::test]
    async fn test_visual_threshold_at_70_percent() {
        let (world, sid) = setup_world();
        let helmet_id = 200040u32;
        // duration=100, so 70% = 70, 30% = 30
        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 100, 71));
        // Start at 72 — after rand(2..=5) loss, could end at 67-70
        // This crosses the 70% threshold (from >=70 to <70)
        equip_item(&world, sid, HEAD, helmet_id, 72);

        world.item_wore_out(sid, WORE_TYPE_DEFENCE, 0);

        let inv = world.get_inventory(sid);
        let dur = inv[HEAD as usize].durability;
        assert!(
            (67..=70).contains(&dur),
            "durability should be 67-70, got {}",
            dur
        );
    }

    #[tokio::test]
    async fn test_visual_threshold_at_30_percent() {
        let (world, sid) = setup_world();
        let helmet_id = 200050u32;
        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 100, 71));
        // Start at 32 — after loss, could cross 30% threshold
        equip_item(&world, sid, HEAD, helmet_id, 32);

        world.item_wore_out(sid, WORE_TYPE_DEFENCE, 0);

        let inv = world.get_inventory(sid);
        let dur = inv[HEAD as usize].durability;
        assert!(
            (27..=30).contains(&dur),
            "durability should be 27-30, got {}",
            dur
        );
    }

    #[tokio::test]
    async fn test_zero_worerate_returns_early() {
        // UTC types with damage=0 should return early without changing anything
        let (world, sid) = setup_world();
        let sword_id = 120010u32;
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        equip_item(&world, sid, RIGHTHAND, sword_id, 500);

        world.item_wore_out(sid, WORE_TYPE_UTC_ATTACK, 0);

        let inv = world.get_inventory(sid);
        assert_eq!(inv[RIGHTHAND as usize].durability, 500);
    }

    #[tokio::test]
    async fn test_negative_damage_acid_reduces() {
        // C++ calls ItemWoreOut(ACID_ALL, -damage) where damage is negative healing
        // This means worerate can be negative. In C++, negative worerate means:
        // if (worerate > pItem->sDuration) pItem->sDuration = 0;
        // So a negative worerate will never be > durability, meaning:
        // pItem->sDuration -= worerate  (subtracting negative = adding)
        let (world, sid) = setup_world();
        let helmet_id = 200060u32;
        world.insert_item(helmet_id, make_item(helmet_id as i32, 7, 500, 71));
        equip_item(&world, sid, HEAD, helmet_id, 100);

        // Negative damage = increase durability (C++ behavior)
        world.item_wore_out(sid, WORE_TYPE_ACID_ALL, -50);

        let inv = world.get_inventory(sid);
        // -50 worerate: worerate(-50) > durability(100)? No.
        // durability = 100 - (-50) = 150
        assert_eq!(inv[HEAD as usize].durability, 150);
    }

    #[tokio::test]
    async fn test_invalid_wore_type_returns_early() {
        let (world, sid) = setup_world();
        let sword_id = 120011u32;
        world.insert_item(sword_id, make_item(sword_id as i32, 1, 1000, 21));
        equip_item(&world, sid, RIGHTHAND, sword_id, 500);

        world.item_wore_out(sid, 99, 0);

        let inv = world.get_inventory(sid);
        assert_eq!(inv[RIGHTHAND as usize].durability, 500);
    }
}
