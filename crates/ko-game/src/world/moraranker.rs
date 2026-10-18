//! Native v2615 MORANKER statue lifecycle.

use std::collections::HashMap;
use std::sync::Arc;

use ko_db::repositories::moraranker::{
    MorankerCharacterRow, MorankerRepository, MorankerStatueSlotRow,
};
use ko_db::DbPool;

use crate::npc::{build_npc_inout, NpcInstance, NpcTemplate, NPC_IN, NPC_OUT};
use crate::zone::calc_region;

use super::WorldState;

pub const MORANKER_ZONE: u16 = 21;
pub const MORANKER_TYPE_FIRST: u8 = b'R';
pub const MORANKER_TYPE_LAST: u8 = b'W';

const MORANKER_PROTO_BASE: u16 = 31_882;
const MORANKER_PROTO_LAST: u16 = MORANKER_PROTO_BASE + 5;

#[derive(Clone, Copy)]
struct StatueSlot {
    npc_type: u8,
    nation: u8,
    x: f32,
    z: f32,
    direction: u8,
}

// Existing Moradon MORANKER pedestals. R/S/T are Karus rank 1..3 and
// U/V/W are El Morad rank 1..3. DB rows from `moraranker_statue_slot` override
// these fallback coordinates/directions.
const FALLBACK_STATUE_SLOTS: [StatueSlot; 6] = [
    StatueSlot {
        npc_type: b'R',
        nation: 1,
        x: 790.0,
        z: 561.0,
        direction: 66,
    },
    StatueSlot {
        npc_type: b'S',
        nation: 1,
        x: 782.0,
        z: 561.0,
        direction: 66,
    },
    StatueSlot {
        npc_type: b'T',
        nation: 1,
        x: 773.0,
        z: 561.0,
        direction: 66,
    },
    StatueSlot {
        npc_type: b'U',
        nation: 2,
        x: 842.0,
        z: 561.0,
        direction: 70,
    },
    StatueSlot {
        npc_type: b'V',
        nation: 2,
        x: 849.0,
        z: 561.0,
        direction: 70,
    },
    StatueSlot {
        npc_type: b'W',
        nation: 2,
        x: 858.0,
        z: 561.0,
        direction: 70,
    },
];

fn slot_from_row(row: MorankerStatueSlotRow) -> Option<StatueSlot> {
    Some(StatueSlot {
        npc_type: u8::try_from(row.npc_type).ok()?,
        nation: u8::try_from(row.nation).ok()?,
        x: row.x,
        z: row.z,
        direction: row.direction.rem_euclid(256) as u8,
    })
}

fn is_moraranker_template(template: &NpcTemplate) -> bool {
    (MORANKER_PROTO_BASE..=MORANKER_PROTO_LAST).contains(&template.s_sid)
        && (MORANKER_TYPE_FIRST..=MORANKER_TYPE_LAST).contains(&template.npc_type)
}

fn item_at(items: &HashMap<(String, i16), u32>, name: &str, slot: i16) -> u32 {
    items
        .get(&(name.to_ascii_lowercase(), slot))
        .copied()
        .unwrap_or(0)
}

fn ranker_template(
    slot_index: usize,
    slot: StatueSlot,
    character: &MorankerCharacterRow,
    items: &HashMap<(String, i16), u32>,
) -> NpcTemplate {
    let name = character.str_user_id.clone();
    NpcTemplate {
        s_sid: MORANKER_PROTO_BASE + slot_index as u16,
        is_monster: false,
        name: name.clone(),
        pid: 0,
        size: 130,
        weapon_1: item_at(items, &name, 6),
        weapon_2: item_at(items, &name, 8),
        group: character.race.clamp(0, u8::MAX as i16) as u8,
        act_type: character.face.clamp(0, u8::MAX as i16) as u8,
        npc_type: slot.npc_type,
        family_type: 0,
        selling_group: item_at(items, &name, 12),
        level: 1,
        max_hp: 1,
        max_mp: 0,
        attack: character.class.max(0) as u16,
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
        magic_1: item_at(items, &name, 4),
        magic_2: item_at(items, &name, 1),
        magic_3: item_at(items, &name, 10),
        magic_attack: 0,
        fire_r: 0,
        cold_r: 0,
        lightning_r: 0,
        magic_r: 0,
        disease_r: 0,
        poison_r: 0,
        exp: character.hair_rgb as u32,
        loyalty: character.loyalty.max(0) as u32,
        money: item_at(items, &name, 13),
        item_table: 0,
        area_range: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::FALLBACK_STATUE_SLOTS;

    #[test]
    fn fallback_statue_slots_are_rotated_180_degrees_from_current_db_facing() {
        let directions: Vec<u8> = FALLBACK_STATUE_SLOTS
            .iter()
            .map(|slot| slot.direction)
            .collect();

        assert_eq!(directions, vec![66, 66, 66, 70, 70, 70]);
    }
}

impl WorldState {
    /// Return the live owner name for a native R..W statue type.
    pub fn moraranker_owner(&self, npc_type: u8) -> Option<String> {
        if !(MORANKER_TYPE_FIRST..=MORANKER_TYPE_LAST).contains(&npc_type) {
            return None;
        }
        self.npc_templates
            .iter()
            .find(|entry| !entry.is_monster && entry.npc_type == npc_type)
            .map(|entry| entry.name.clone())
    }

    /// Rebuild all six statues from current NP rankings and equipped items.
    /// Existing native ranker NPCs are removed first, preventing duplicates
    /// from legacy K_NPCPOS rows.
    pub async fn reload_moraranker(&self, pool: &DbPool, broadcast: bool) -> anyhow::Result<()> {
        let repo = MorankerRepository::new(pool);
        let ranked = repo.load_top_six().await?;
        let statue_slots = match repo.load_statue_slots().await {
            Ok(rows) if rows.len() == 6 => {
                let slots: Vec<_> = rows.into_iter().filter_map(slot_from_row).collect();
                if slots.len() == 6 {
                    slots
                } else {
                    FALLBACK_STATUE_SLOTS.to_vec()
                }
            }
            Ok(rows) => {
                tracing::warn!(
                    rows = rows.len(),
                    "MORANKER statue slot table did not return six rows; using fallback slots"
                );
                FALLBACK_STATUE_SLOTS.to_vec()
            }
            Err(error) => {
                tracing::warn!(
                    %error,
                    "MORANKER statue slot table unavailable; using fallback slots"
                );
                FALLBACK_STATUE_SLOTS.to_vec()
            }
        };
        let names: Vec<&str> = ranked.iter().map(|row| row.str_user_id.as_str()).collect();
        let equipped = repo.load_equipment(&names).await?;
        let item_map: HashMap<(String, i16), u32> = equipped
            .into_iter()
            .map(|row| {
                (
                    (row.str_user_id.to_ascii_lowercase(), row.slot_index),
                    row.item_id as u32,
                )
            })
            .collect();

        let old_instances: Vec<_> = self
            .npc_instances
            .iter()
            .filter_map(|entry| {
                let instance = entry.value().clone();
                let template = self.get_npc_template(instance.proto_id, instance.is_monster)?;
                is_moraranker_template(&template).then_some((instance, template))
            })
            .collect();

        for (instance, template) in old_instances {
            if broadcast {
                let packet = build_npc_inout(NPC_OUT, &instance, &template);
                self.broadcast_to_3x3(
                    instance.zone_id,
                    instance.region_x,
                    instance.region_z,
                    Arc::new(packet),
                    None,
                    instance.event_room,
                );
            }
            if let Some(zone) = self.get_zone(instance.zone_id) {
                zone.remove_npc(instance.region_x, instance.region_z, instance.nid);
            }
            self.npc_instances.remove(&instance.nid);
            self.npc_hp.remove(&instance.nid);
            self.npc_ai.remove(&instance.nid);
        }
        self.npc_templates
            .retain(|_, template| !is_moraranker_template(template));

        let by_nation: HashMap<u8, Vec<&MorankerCharacterRow>> = [1_u8, 2_u8]
            .into_iter()
            .map(|nation| {
                (
                    nation,
                    ranked
                        .iter()
                        .filter(|row| row.nation as u8 == nation)
                        .take(3)
                        .collect(),
                )
            })
            .collect();

        for (slot_index, slot) in statue_slots.into_iter().enumerate() {
            let nation_rank = if slot.nation == 1 {
                slot_index
            } else {
                slot_index - 3
            };
            let Some(character) = by_nation
                .get(&slot.nation)
                .and_then(|rows| rows.get(nation_rank))
                .copied()
            else {
                continue;
            };

            let template = Arc::new(ranker_template(slot_index, slot, character, &item_map));
            let nid = self.allocate_npc_id();
            let region_x = calc_region(slot.x);
            let region_z = calc_region(slot.z);
            let instance = Arc::new(NpcInstance {
                nid,
                proto_id: template.s_sid,
                is_monster: false,
                zone_id: MORANKER_ZONE,
                x: slot.x,
                y: 5.2,
                z: slot.z,
                direction: slot.direction,
                region_x,
                region_z,
                gate_open: 0,
                object_type: 0,
                nation: slot.nation,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            });

            self.npc_templates
                .insert((template.s_sid, false), template.clone());
            self.npc_instances.insert(nid, instance.clone());
            self.npc_hp.insert(nid, 1);
            if let Some(zone) = self.get_zone(MORANKER_ZONE) {
                zone.add_npc(region_x, region_z, nid);
            }
            if broadcast {
                let packet = build_npc_inout(NPC_IN, &instance, &template);
                self.broadcast_to_3x3(MORANKER_ZONE, region_x, region_z, Arc::new(packet), None, 0);
            }

            tracing::info!(
                ranker_type = %(slot.npc_type as char),
                nation = slot.nation,
                character = %character.str_user_id,
                np = character.loyalty,
                monthly_np = character.loyalty_monthly,
                x = slot.x,
                z = slot.z,
                direction = slot.direction,
                memo_len = character.str_memo.as_deref().unwrap_or_default().len(),
                "MORANKER statue loaded"
            );
        }

        Ok(())
    }
}
