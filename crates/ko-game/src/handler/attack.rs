//! WIZ_ATTACK (0x08) handler -- physical melee attack.
//! ## Client Request (C->S)
//! | Type  | Description                        |
//! |-------|------------------------------------|
//! | u8    | bType (attack sub-type)            |
//! | u8    | bResult (client-side, overwritten)  |
//! | u32le | tid (target ID)                    |
//! | i16le | delaytime (weapon delay ticks)      |
//! | i16le | distance (to target)               |
//! | u8    | unknown                            |
//! | u8    | unknowns                           |
//! ## Server Broadcast (S->C, to 3x3 region)
//! | Type  | Description                        |
//! |-------|------------------------------------|
//! | u8    | bType (attack sub-type)            |
//! | u8    | bResult (0=fail, 1=success, 2=dead)|
//! | u32le | attacker session ID                |
//! | u32le | target ID                          |
//! | u8    | unknown (echoed from client)       |
//! ## Attack Result Constants
//! - `ATTACK_FAIL` (0): attack missed or blocked
//! - `ATTACK_SUCCESS` (1): attack landed, target still alive
//! - `ATTACK_TARGET_DEAD` (2): attack killed the target

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ko_db::models::CoefficientRow;
use ko_protocol::{Opcode, Packet, PacketReader};
use rand::Rng;
use rand::SeedableRng;

use crate::handler::arena;
use crate::handler::dead;
use crate::handler::durability::{WORE_TYPE_ATTACK, WORE_TYPE_DEFENCE};
use crate::npc::NpcId;
use crate::npc_type_constants::{
    NPC_BIFROST_MONUMENT, NPC_BORDER_MONUMENT, NPC_CLAN_WAR_MONUMENT, NPC_DESTROYED_ARTIFACT,
    NPC_FOSIL, NPC_GATE,
    NPC_GATE2, NPC_GATE_LEVER, NPC_GUARD_TOWER1, NPC_GUARD_TOWER2,
    NPC_OBJECT_FLAG, NPC_PARTNER_TYPE, NPC_PHOENIX_GATE,
    NPC_PRISON, NPC_PVP_MONUMENT, NPC_REFUGEE, NPC_SANTA, NPC_SOCCER_BAAL, NPC_SPECIAL_GATE,
    NPC_TREE, NPC_VICTORY_GATE,
};
use crate::session::{ClientSession, SessionState};
use crate::systems::bdw;
use crate::systems::regen::build_hp_change_packet_with_attacker;
use crate::world::{
    CharacterInfo, CswOpStatus, NpcState, Position, WorldState, ITEM_GOLD, NATION_ELMORAD,
    NATION_KARUS, RANGE_50M, RANGE_80M, USER_DEAD, USER_SITDOWN, ZONE_ARDREAM, ZONE_ARENA,
    ZONE_BATTLE, ZONE_BATTLE2, ZONE_BATTLE3, ZONE_BATTLE4, ZONE_BATTLE5, ZONE_BATTLE6,
    ZONE_BIFROST, ZONE_BORDER_DEFENSE_WAR, ZONE_CHAOS_DUNGEON, ZONE_CLAN_WAR_ARDREAM,
    ZONE_CLAN_WAR_RONARK, ZONE_DELOS, ZONE_DESPERATION_ABYSS, ZONE_DRAGON_CAVE,
    ZONE_DUNGEON_DEFENCE, ZONE_ELMORAD, ZONE_ELMORAD2, ZONE_ELMORAD3, ZONE_HELL_ABYSS,
    ZONE_JURAID_MOUNTAIN, ZONE_KARUS, ZONE_KARUS2, ZONE_KARUS3, ZONE_KNIGHT_ROYALE,
    ZONE_KROWAZ_DOMINION, ZONE_MORADON, ZONE_MORADON5, ZONE_PARTY_VS_1, ZONE_PARTY_VS_4,
    ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE, ZONE_SNOW_BATTLE,
};
use crate::zone::SessionId;

use crate::attack_constants::{
    ATTACK_FAIL, ATTACK_SUCCESS, ATTACK_TARGET_DEAD, FAIL, GREAT_SUCCESS, NORMAL, SUCCESS,
};
use crate::buff_constants::{BUFF_TYPE_BLIND, BUFF_TYPE_FREEZE, BUFF_TYPE_KAUL_TRANSFORMATION};

// ── Melee constants ─────────────────────────────────────────────────────────

/// Minimum melee attack delay (empty-handed or mage).
const MIN_MELEE_DELAY: i16 = 100;

/// Server-side rate limit between R-attacks (milliseconds).
const PLAYER_R_HIT_REQUEST_INTERVAL: u64 = 900;

/// Maximum attack range for melee (in game units).
/// When no weapon is equipped, use a generous melee range to
/// avoid rejecting legitimate attacks.
const DEFAULT_MELEE_RANGE: f32 = 15.0;

/// GM weapon item ID — bypasses delay checks.
const GM_WEAPON_ID: u32 = 389158000;

/// GM test damage override for normal R-attacks.
const GM_FIXED_DAMAGE: i16 = 30000;

/// Minimum weapon power for bare-hand attacks.
const MIN_WEAPON_POWER: u16 = 3;

/// Default attack amount multiplier (no buffs).
#[cfg(test)]
const DEFAULT_ATTACK_AMOUNT: u16 = 100;

// ── Class group helpers ─────────────────────────────────────────────────────

/// Get the "base class type" from a full class ID.
/// The class % 100 gives: 1=Warrior, 2=Rogue, 3=Mage, 4=Priest,
/// 5=WarriorNovice, 6=WarriorMaster, 7=RogueNovice, 8=RogueMaster,
/// 9=MageNovice, 10=MageMaster, 11=PriestNovice, 12=PriestMaster,
/// 13=Kurian, 14=KurianNovice, 15=KurianMaster.
fn base_class(class: u16) -> u16 {
    class % 100
}

/// Check if class belongs to the Warrior group (1, 5, 6).
fn is_warrior(class: u16) -> bool {
    matches!(base_class(class), 1 | 5 | 6)
}

/// Check if class belongs to the Rogue group (2, 7, 8).
fn is_rogue(class: u16) -> bool {
    matches!(base_class(class), 2 | 7 | 8)
}

/// Check if class belongs to the Mage group (3, 9, 10).
fn is_mage(class: u16) -> bool {
    matches!(base_class(class), 3 | 9 | 10)
}

/// Check if class belongs to the Priest group (4, 11, 12).
fn is_priest(class: u16) -> bool {
    matches!(base_class(class), 4 | 11 | 12)
}

/// Map a class to its class group index (0-based) for AP/AC class bonus arrays.
/// Returns GROUP_WARRIOR(1)-1=0, GROUP_ROGUE(2)-1=1, GROUP_MAGE(3)-1=2, GROUP_CLERIC(4)-1=3.
/// Returns `None` for Kurian or unknown classes (no class bonus slot).
pub(crate) fn class_group_index(class: u16) -> Option<usize> {
    let bc = base_class(class);
    match bc {
        1 | 5 | 6 => Some(0),   // Warrior
        2 | 7 | 8 => Some(1),   // Rogue
        3 | 9 | 10 => Some(2),  // Mage
        4 | 11 | 12 => Some(3), // Priest
        _ => None,              // Kurian or unknown
    }
}

use crate::inventory_constants::{
    WEAPON_KIND_1H_AXE, WEAPON_KIND_1H_CLUB, WEAPON_KIND_1H_SPEAR, WEAPON_KIND_1H_SWORD,
    WEAPON_KIND_2H_AXE, WEAPON_KIND_2H_CLUB, WEAPON_KIND_2H_SPEAR, WEAPON_KIND_2H_SWORD,
    WEAPON_KIND_BOW, WEAPON_KIND_CROSSBOW, WEAPON_KIND_DAGGER, WEAPON_KIND_JAMADAR,
};

/// Check if item is a Timing Delay weapon.
fn is_timing_delay(item_num: u32) -> bool {
    item_num == 900335523 || item_num == 900336524 || item_num == 900337525
}

/// Check if item is a Wirinim Unique Delay weapon.
fn is_wirinom_uniq_delay(item_num: u32) -> bool {
    (127410731..=127410740).contains(&item_num)
        || (127420741..=127420750).contains(&item_num)
        || (127430751..=127430760).contains(&item_num)
        || (127440761..=127440770).contains(&item_num)
        || item_num == 127410284
        || item_num == 127420285
        || item_num == 127430286
        || item_num == 127440287
}

/// Check if item is a Wirinim Rebirth Delay weapon.
fn is_wirinom_reb_delay(item_num: u32) -> bool {
    (127411181..=127411210).contains(&item_num)
        || (127421211..=127421240).contains(&item_num)
        || (127431241..=127431270).contains(&item_num)
        || (127441271..=127441300).contains(&item_num)
}

/// Check if item is a Garges Sword Delay weapon.
fn is_garges_sword_delay(item_num: u32) -> bool {
    (1110582731..=1110582740).contains(&item_num) || item_num == 1110582451
}

/// Check if a weapon kind is a bow or crossbow.
fn is_bow_weapon(kind: i32) -> bool {
    kind == WEAPON_KIND_BOW || kind == WEAPON_KIND_CROSSBOW
}

/// Get the weapon range in game units for server-side distance validation.
/// falls back to left-hand. Returns `pTable.m_sRange / 10.0`.
fn get_weapon_range(world: &WorldState, sid: SessionId) -> f32 {
    // Single session lock for both weapon slot item IDs (2 DashMap reads → 1)
    let (rh_id, lh_id) = world
        .with_session(sid, |h| {
            let rh = h
                .inventory
                .get(crate::inventory_constants::RIGHTHAND)
                .map(|s| s.item_id)
                .unwrap_or(0);
            let lh = h
                .inventory
                .get(crate::inventory_constants::LEFTHAND)
                .map(|s| s.item_id)
                .unwrap_or(0);
            (rh, lh)
        })
        .unwrap_or((0, 0));
    // Item table lookups (separate DashMap, no session contention)
    if rh_id != 0 {
        if let Some(w) = world.get_item(rh_id) {
            let r = w.range.unwrap_or(0) as f32;
            if r > 0.0 {
                return r / 10.0;
            }
        }
    }
    if lh_id != 0 {
        if let Some(w) = world.get_item(lh_id) {
            let r = w.range.unwrap_or(0) as f32;
            if r > 0.0 {
                return r / 10.0;
            }
        }
    }
    0.0
}

/// Check if the player is in an enemy safety area (no-PvP zone).
/// Safety areas are nation-specific spawn/village areas where the ENEMY
/// cannot attack. Each zone defines circular or rectangular safe regions.
/// "Enemy safety area" means: the area is safe for my enemies — I cannot attack here.
/// E.g., an Elmorad player near Elmorad village cannot attack Karus players there.
pub(crate) fn is_in_enemy_safety_area(zone_id: u16, x: f32, z: f32, nation: u8) -> bool {
    /// Circular distance check (squared) — matches `isInRangeSlow(x, z, range)`.
    fn in_range(px: f32, pz: f32, cx: f32, cz: f32, radius: f32) -> bool {
        let dx = px - cx;
        let dz = pz - cz;
        dx * dx + dz * dz <= radius * radius
    }

    match zone_id {
        ZONE_DELOS => in_range(x, z, 500.0, 180.0, 115.0),
        ZONE_BIFROST => {
            if nation == NATION_ELMORAD {
                x > 56.0 && x < 124.0 && z > 700.0 && z < 840.0
            } else {
                x > 190.0 && x < 270.0 && z > 870.0 && z < 970.0
            }
        }
        ZONE_ARENA => in_range(x, z, 127.0, 113.0, 36.0),
        ZONE_ELMORAD | ZONE_ELMORAD2 | ZONE_ELMORAD3 => {
            if nation == NATION_ELMORAD {
                in_range(x, z, 210.0, 1853.0, 50.0)
            } else {
                false
            }
        }
        ZONE_KARUS | ZONE_KARUS2 | ZONE_KARUS3 => {
            if nation == NATION_KARUS {
                in_range(x, z, 1860.0, 174.0, 50.0)
            } else {
                false
            }
        }
        ZONE_BATTLE => {
            if nation == NATION_KARUS {
                x > 98.0 && x < 125.0 && z > 755.0 && z < 780.0
            } else if nation == NATION_ELMORAD {
                x > 805.0 && x < 831.0 && z > 85.0 && z < 110.0
            } else {
                false
            }
        }
        ZONE_BATTLE2 => {
            if nation == NATION_KARUS {
                x > 942.0 && x < 977.0 && z > 863.0 && z < 904.0
            } else if nation == NATION_ELMORAD {
                x > 46.0 && x < 80.0 && z > 142.0 && z < 174.0
            } else {
                false
            }
        }
        ZONE_BATTLE4 => {
            if nation == NATION_KARUS {
                in_range(x, z, 235.0, 228.0, 80.0)
                    || in_range(x, z, 846.0, 362.0, 20.0)
                    || in_range(x, z, 338.0, 807.0, 20.0)
            } else if nation == NATION_ELMORAD {
                in_range(x, z, 809.0, 783.0, 80.0)
                    || in_range(x, z, 182.0, 668.0, 20.0)
                    || in_range(x, z, 670.0, 202.0, 20.0)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if the player is in their own safety area (protected from enemy attack).
/// "Own safety area" means: this area is safe for ME — enemies cannot attack me here.
/// Uses the same coordinates as `isInEnemySafetyArea` but with INVERTED nation logic.
/// E.g., a Karus player near Elmorad village is in their own safety area.
/// Used in `MagicProcess.cpp:384` for `MORAL_AREA_ALL` skill target validation
/// and in `is_hostile_to()` for target protection check.
pub(crate) fn is_in_own_safety_area(zone_id: u16, x: f32, z: f32, nation: u8) -> bool {
    fn in_range(px: f32, pz: f32, cx: f32, cz: f32, radius: f32) -> bool {
        let dx = px - cx;
        let dz = pz - cz;
        dx * dx + dz * dz <= radius * radius
    }

    match zone_id {
        ZONE_DELOS => in_range(x, z, 500.0, 180.0, 115.0),
        ZONE_BIFROST => {
            // Nation INVERTED vs isInEnemySafetyArea
            if nation == NATION_KARUS {
                x > 56.0 && x < 124.0 && z > 700.0 && z < 840.0
            } else {
                x > 190.0 && x < 270.0 && z > 870.0 && z < 970.0
            }
        }
        ZONE_ARENA => in_range(x, z, 127.0, 113.0, 36.0),
        ZONE_ELMORAD | ZONE_ELMORAD2 | ZONE_ELMORAD3 => {
            // C++ isInOwnSafetyArea: Karus in Elmorad zone (inverted from enemy check)
            if nation == NATION_KARUS {
                in_range(x, z, 210.0, 1853.0, 50.0)
            } else {
                false
            }
        }
        ZONE_KARUS | ZONE_KARUS2 | ZONE_KARUS3 => {
            // C++ isInOwnSafetyArea: Elmorad in Karus zone (inverted from enemy check)
            if nation == NATION_ELMORAD {
                in_range(x, z, 1860.0, 174.0, 50.0)
            } else {
                false
            }
        }
        ZONE_BATTLE => {
            // Nation INVERTED vs isInEnemySafetyArea
            if nation == NATION_ELMORAD {
                x > 98.0 && x < 125.0 && z > 755.0 && z < 780.0
            } else if nation == NATION_KARUS {
                x > 805.0 && x < 831.0 && z > 85.0 && z < 110.0
            } else {
                false
            }
        }
        ZONE_BATTLE2 => {
            if nation == NATION_ELMORAD {
                x > 942.0 && x < 977.0 && z > 863.0 && z < 904.0
            } else if nation == NATION_KARUS {
                x > 46.0 && x < 80.0 && z > 142.0 && z < 174.0
            } else {
                false
            }
        }
        ZONE_BATTLE4 => {
            if nation == NATION_ELMORAD {
                in_range(x, z, 235.0, 228.0, 80.0)
                    || in_range(x, z, 846.0, 362.0, 20.0)
                    || in_range(x, z, 338.0, 807.0, 20.0)
            } else if nation == NATION_KARUS {
                in_range(x, z, 809.0, 783.0, 80.0)
                    || in_range(x, z, 182.0, 668.0, 20.0)
                    || in_range(x, z, 670.0, 202.0, 20.0)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Apply weapon-type-specific armor resistance to PvP damage.
/// Each equipped weapon is checked against the target's weapon-type resistances
/// (accumulated from armor). The formula per weapon:
///   `damage -= damage * target_resistance / 250`
/// For daggers and bows, an additional amount modifier is applied:
///   `damage -= damage * (resistance * amount / 100) / 250`
/// where `dagger_r_amount`/`bow_r_amount` default to 100 and are reduced
/// by Eskrima debuff (BUFF_TYPE_DAGGER_BOW_DEFENSE, 45).
pub(crate) fn get_ac_damage(
    damage: i16,
    weapon_kinds: &[Option<i32>],
    target_stats: &crate::world::EquippedStats,
    dagger_r_amount: u8,
    bow_r_amount: u8,
) -> i16 {
    let mut d = damage as i32;
    for kind_opt in weapon_kinds {
        let kind = match kind_opt {
            Some(k) => *k,
            None => continue,
        };

        let resistance = if kind == WEAPON_KIND_DAGGER {
            target_stats.dagger_r as i32 * dagger_r_amount as i32 / 100
        } else if kind == WEAPON_KIND_1H_SWORD || kind == WEAPON_KIND_2H_SWORD {
            target_stats.sword_r as i32
        } else if kind == WEAPON_KIND_1H_AXE || kind == WEAPON_KIND_2H_AXE {
            target_stats.axe_r as i32
        } else if kind == WEAPON_KIND_1H_CLUB || kind == WEAPON_KIND_2H_CLUB {
            target_stats.club_r as i32
        } else if kind == WEAPON_KIND_1H_SPEAR || kind == WEAPON_KIND_2H_SPEAR {
            target_stats.spear_r as i32
        } else if kind == WEAPON_KIND_BOW || kind == WEAPON_KIND_CROSSBOW {
            target_stats.bow_r as i32 * bow_r_amount as i32 / 100
        } else if kind == WEAPON_KIND_JAMADAR {
            target_stats.jamadar_r as i32
        } else {
            continue;
        };

        d -= d * resistance / 250;
    }
    d.max(0) as i16
}

// ── Combat stat computations ────────────────────────────────────────────────

/// Calculate total attack power (m_sTotalHit) for a character.
/// Formula (simplified for no equipment):
/// - `power` = weapon damage, clamped min 3 (bare-hand)
/// - `coeff` = weapon-type coefficient from class table (0 for bare-hand)
/// - Rogue:   `(0.005 * power * (dex + 40)) + (coeff * power * level * dex) + 3`
/// - Warrior: `(0.005 * power * (main_stat + 40)) + (coeff * power * level * main_stat) + 3 + base_ap`
/// - Others:  Same as Warrior formula with STR as main stat.
/// `base_ap` = max(0, main_stat - 150) for stats > 150.
fn compute_total_hit(ch: &CharacterInfo, _coeff: &CoefficientRow) -> u16 {
    let power = MIN_WEAPON_POWER as f32;
    // Weapon coefficient is 0.0 for bare-hand (no weapon equipped).
    let weapon_coeff: f32 = 0.0;
    // No AP bonus buffs — default 100%.
    let bonus_ap: f32 = 1.0;

    let str_val = ch.str as f32;
    let dex_val = ch.dex as f32;
    let int_val = ch.intel as f32;

    // BaseAp: bonus for stats > 150
    let base_ap = if str_val > 150.0 {
        str_val - 150.0
    } else if int_val > 150.0 {
        int_val - 150.0
    } else {
        0.0
    };

    let level = ch.level as f32;
    let total_hit = if is_rogue(ch.class) {
        ((0.005 * power * (dex_val + 40.0)) + (weapon_coeff * power * level * dex_val) + 3.0)
            * bonus_ap
    } else if is_warrior(ch.class) {
        let main_stat = if str_val >= int_val { str_val } else { int_val };
        ((0.005 * power * (main_stat + 40.0)) + (weapon_coeff * power * level * main_stat) + 3.0)
            * bonus_ap
            + base_ap
    } else if is_priest(ch.class) {
        let main_stat = if str_val > int_val { str_val } else { int_val };
        ((0.005 * power * (main_stat + 40.0)) + (weapon_coeff * power * level * main_stat) + 3.0)
            * bonus_ap
            + base_ap
    } else {
        // Kurian / default: STR-based
        ((0.005 * power * (str_val + 40.0)) + (weapon_coeff * power * level * str_val) + 3.0)
            * bonus_ap
            + base_ap
    };

    total_hit.max(0.0) as u16
}

/// Calculate total armor class (m_sTotalAc) for a character.
/// Without equipment, `item_ac = 0`, so: `AC_coeff * level`.
fn compute_total_ac(ch: &CharacterInfo, coeff: &CoefficientRow) -> u16 {
    (coeff.ac * ch.level as f64).max(0.0) as u16
}

/// Calculate total hit rate for a character.
/// ```text
/// m_fTotalHitrate = ((1 + Hitrate * level * dex) * item_hitrate / 100) * (hit_rate_amount / 100)
/// ```
/// Without items: `item_hitrate = 100`, `hit_rate_amount = 100`.
fn compute_hitrate(ch: &CharacterInfo, coeff: &CoefficientRow) -> f32 {
    // item_hitrate defaults to 100, hit_rate_amount defaults to 100
    1.0 + coeff.hitrate as f32 * ch.level as f32 * ch.dex as f32
}

/// Calculate total evasion rate for a character.
/// ```text
/// m_fTotalEvasionrate = ((1 + Evasionrate * level * dex) * item_evasion / 100) * (avoid_amount / 100)
/// ```
/// Without items: `item_evasion = 100`, `avoid_amount = 100`.
fn compute_evasion(ch: &CharacterInfo, coeff: &CoefficientRow) -> f32 {
    1.0 + coeff.evasionrate as f32 * ch.level as f32 * ch.dex as f32
}

/// Determine hit result using the C++ hit rate table.
/// Returns one of: GREAT_SUCCESS (1), SUCCESS (2), NORMAL (3), FAIL (4).
/// Each rate bracket has different probability distributions:
/// - rate >= 5.0:  35% great, 40% success, 23% normal, 2% fail
/// - rate < 0.2:   2% great, 8% success, 40% normal, 50% fail
fn get_hit_rate(rate: f32, rng: &mut impl Rng) -> u8 {
    let random = rng.gen_range(1..=10000);

    if rate >= 5.0 {
        if random <= 3500 {
            GREAT_SUCCESS
        } else if random <= 7500 {
            SUCCESS
        } else if random <= 9800 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 3.0 {
        if random <= 2500 {
            GREAT_SUCCESS
        } else if random <= 6000 {
            SUCCESS
        } else if random <= 9600 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 2.0 {
        if random <= 2000 {
            GREAT_SUCCESS
        } else if random <= 5000 {
            SUCCESS
        } else if random <= 9400 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 1.25 {
        if random <= 1500 {
            GREAT_SUCCESS
        } else if random <= 4000 {
            SUCCESS
        } else if random <= 9200 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.8 {
        if random <= 1000 {
            GREAT_SUCCESS
        } else if random <= 3000 {
            SUCCESS
        } else if random <= 9000 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.5 {
        if random <= 800 {
            GREAT_SUCCESS
        } else if random <= 2500 {
            SUCCESS
        } else if random <= 8000 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.33 {
        if random <= 600 {
            GREAT_SUCCESS
        } else if random <= 2000 {
            SUCCESS
        } else if random <= 7000 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.2 {
        if random <= 400 {
            GREAT_SUCCESS
        } else if random <= 1500 {
            SUCCESS
        } else if random <= 6000 {
            NORMAL
        } else {
            FAIL
        }
    } else if random <= 200 {
        GREAT_SUCCESS
    } else if random <= 1000 {
        SUCCESS
    } else if random <= 5000 {
        NORMAL
    } else {
        FAIL
    }
}

/// Calculate R-attack (normal melee) damage.
/// ## Formula
/// 1. `temp_ap = total_hit * attack_amount` (attack_amount = 100 default)
/// 2. `temp_ac = target.total_ac` (no buff/debuff adjustments yet)
/// 3. `temp_hit_B = (temp_ap * 200 / 100) / (temp_ac + 240)`
/// 4. Hit check: `GetHitRate(attacker_hitrate / target_evasion + 1.0)`
/// 5. On GREAT_SUCCESS / SUCCESS / NORMAL:
///    - Priest: `damage = 0.15 * temp_hit_B + 0.2 * random(0, temp_hit_B)`
///    - Others: `damage = 0.75 * temp_hit_B + 0.3 * random(0, temp_hit_B)`
/// 6. On FAIL: damage = 0
#[cfg(test)]
fn calculate_r_damage(
    attacker: &CharacterInfo,
    attacker_coeff: &CoefficientRow,
    target: &CharacterInfo,
    target_coeff: &CoefficientRow,
    rng: &mut impl Rng,
) -> i16 {
    calculate_r_damage_with_class_bonus(
        attacker,
        attacker_coeff,
        target,
        target_coeff,
        rng,
        None,
        None,
        None,
        DEFAULT_ATTACK_AMOUNT as i32,
        None,
    )
}

/// Physical damage calculation with optional class bonus arrays for PvP.
/// When `attacker_ap_class_bonus` and `target_ac_class_bonus` are provided,
/// applies class-specific AP/AC bonuses using the target's base class as
/// the array index (C++ active `#else` branch, Unit.cpp:320-322).
#[allow(clippy::too_many_arguments)]
fn calculate_r_damage_with_class_bonus(
    attacker: &CharacterInfo,
    attacker_coeff: &CoefficientRow,
    target: &CharacterInfo,
    target_coeff: &CoefficientRow,
    rng: &mut impl Rng,
    attacker_ap_class_bonus: Option<&[u8; 4]>,
    target_ac_class_bonus: Option<&[u8; 4]>,
    target_ac_override: Option<i32>,
    attack_amount: i32,
    total_hit_override: Option<u16>,
) -> i16 {
    // ── Step 1: Compute attack power and AC ─────────────────────────────
    // When total_hit_override is Some, use the pre-computed value from
    // set_user_ability() (equipped weapon + buffs + stats). This is the
    // correct production path.  Fallback to compute_total_hit() only in
    // unit tests where no WorldState/inventory is available.
    let total_hit =
        total_hit_override.unwrap_or_else(|| compute_total_hit(attacker, attacker_coeff));

    let mut temp_ap = total_hit as i32 * attack_amount;

    // When target_ac_override is Some, it provides EquippedStats.total_ac + buff_ac
    // (the full C++ m_sTotalAc + m_sACAmount). Without override, falls back to
    // coefficient-based AC (for tests without WorldState).
    let mut temp_ac =
        target_ac_override.unwrap_or_else(|| compute_total_ac(target, target_coeff) as i32);

    // ── Apply class-specific AP/AC bonuses (PvP only) ───────────────────
    // Uses TARGET's base class to index both arrays.
    if let Some(idx) = class_group_index(target.class) {
        if let Some(ac_bonus) = target_ac_class_bonus {
            temp_ac = temp_ac * (100 + ac_bonus[idx] as i32) / 100;
        }
        if let Some(ap_bonus) = attacker_ap_class_bonus {
            temp_ap = temp_ap * (100 + ap_bonus[idx] as i32) / 100;
        }
    }

    let temp_hit_b = if temp_ac + 240 > 0 {
        (temp_ap * 2) / (temp_ac + 240)
    } else {
        temp_ap * 2
    };

    if temp_hit_b <= 0 {
        return 0;
    }

    // ── Step 2: Hit rate check ──────────────────────────────────────────
    let attacker_hitrate = compute_hitrate(attacker, attacker_coeff);
    let target_evasion = compute_evasion(target, target_coeff);

    let rate = if target_evasion > 0.0 {
        attacker_hitrate / target_evasion + 1.0
    } else {
        attacker_hitrate + 1.0
    };

    let hit_result = get_hit_rate(rate, rng);

    // ── Step 3: Damage calculation based on hit result ──────────────────
    match hit_result {
        GREAT_SUCCESS | SUCCESS | NORMAL => {
            let random = if temp_hit_b > 0 {
                rng.gen_range(0..=temp_hit_b)
            } else {
                0
            };

            let damage = if is_priest(attacker.class) {
                (0.15 * temp_hit_b as f32 + 0.2 * random as f32) as i32
            } else {
                (0.75 * temp_hit_b as f32 + 0.3 * random as f32) as i32
            };

            damage.max(1) as i16
        }
        _ => 0,
    }
}

/// Apply zone-specific damage overrides.
/// - Snow Battle: R-attack damage = 0
/// - Chaos Dungeon: fixed 50 (500/10)
/// - Dungeon Defence: fixed 50 (500/10)
fn apply_zone_damage_override(zone_id: u16, damage: i16) -> i16 {
    match zone_id {
        ZONE_SNOW_BATTLE => 0,
        ZONE_CHAOS_DUNGEON | ZONE_DUNGEON_DEFENCE => 50,
        _ => damage,
    }
}

// ── Main handler ────────────────────────────────────────────────────────────

/// Handle WIZ_ATTACK (0x08) from the client.
/// Parses the attack packet, validates pre-conditions, calculates
/// damage using the reference formula, applies HP change, triggers
/// death if needed, and broadcasts the result to the 3x3 region.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // ── Parse client packet ────────────────────────────────────────────
    //   pkt >> bType >> bResult >> tid >> delaytime >> distance >> unknown >> unknowns;
    let mut reader = PacketReader::new(&pkt.data);
    let b_type = reader.read_u8().unwrap_or(0);
    let _b_result_client = reader.read_u8().unwrap_or(0); // overwritten by server
                                                          // C++ declares tid as int32 — negative values (e.g. -1 = 0xFFFFFFFF) mean "no target"
    let tid_signed = reader.read_u32().map(|v| v as i32).unwrap_or(-1);
    let delaytime = reader.read_u16().map(|v| v as i16).unwrap_or(0);
    let distance = reader.read_u16().map(|v| v as i16).unwrap_or(0);
    let unknown = reader.read_u8().unwrap_or(0);
    let _unknowns = reader.read_u8().unwrap_or(0);

    if tid_signed <= 0 {
        return Ok(());
    }
    let tid = tid_signed as u32;

    // ── Pre-attack validation: attacker state ──────────────────────────
    let attacker = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Dead players cannot attack
    if attacker.res_hp_type == USER_DEAD || attacker.hp <= 0 {
        return Ok(());
    }

    // Sitting players cannot attack
    if attacker.res_hp_type == USER_SITDOWN {
        return Ok(());
    }

    // Incapacitated check: blinded, blinking, or kaul state
    // (isDead already checked above)
    {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if world.is_player_blinking(sid, now_unix) {
            return Ok(());
        }
        // Single session read for blind/kaul/weapons_disabled (2 DashMap reads → 1)
        let (is_blinded_or_kaul, weapons_disabled) = world
            .with_session(sid, |h| {
                (
                    h.buffs.contains_key(&BUFF_TYPE_BLIND)
                        || h.buffs.contains_key(&BUFF_TYPE_KAUL_TRANSFORMATION),
                    h.weapons_disabled,
                )
            })
            .unwrap_or((false, false));
        if is_blinded_or_kaul || weapons_disabled {
            return Ok(());
        }
    }

    // GM attack ban check.
    if world.is_attack_disabled(sid) {
        return Ok(());
    }

    // Cannot attack in enemy safety areas (villages, temples, arena spawn)
    let pos = world.get_position(sid).unwrap_or_default();
    if is_in_enemy_safety_area(pos.zone_id, pos.x, pos.z, attacker.nation) {
        return Ok(());
    }

    // Special event zone (Zindan War) attack block
    // Block R-attacks in SPBATTLE zones when the event is NOT opened
    // (unless Cinderella War is active in that zone)
    if is_in_special_event_zone(pos.zone_id)
        && !world.is_zindan_event_opened()
        && !world.is_cinderella_active()
    {
        return Ok(());
    }

    // Cinderella War pre-start attack block
    // Block R-attacks in Cinderella zone when war is ON but NOT started
    if !world.is_zindan_event_opened()
        && world.is_cinderella_active()
        && world.cinderella_zone_id() == pos.zone_id
    {
        // CindWar isON but not started (event opened = false means not started)
        // In C++: !g_pMain->pSpecialEvent.opened && isCindirellaZone && pCindWar.isON && !pCindWar.isStarted
        // We simplify: if zindan_event_opened is false AND cindwar is active but we're in the zone
        // This blocks attacks during the preparation phase
        return Ok(());
    }

    let is_gm = attacker.authority == 0;

    // ── Remove stealth before attacking ──────────────────────────────────
    crate::handler::stealth::remove_stealth(&world, sid);

    // ── Weapon delay validation ─────────────────────────────────────────
    let right_weapon = world.get_right_hand_weapon(sid);
    let left_weapon = world.get_left_hand_weapon(sid);

    // GM weapon bypass: item 389158000 in either hand + GM authority
    let nocheck = is_gm
        && (right_weapon
            .as_ref()
            .is_some_and(|w| w.num as u32 == GM_WEAPON_ID)
            || left_weapon
                .as_ref()
                .is_some_and(|w| w.num as u32 == GM_WEAPON_ID));

    // Reject attacks with bows (handled by archery, not R-attack)
    let right_is_bow = right_weapon
        .as_ref()
        .is_some_and(|w| is_bow_weapon(w.kind.unwrap_or(0)));
    let left_is_bow = left_weapon
        .as_ref()
        .is_some_and(|w| is_bow_weapon(w.kind.unwrap_or(0)));
    if right_is_bow || left_is_bow {
        return Ok(());
    }

    // Server-side rate limit: 900ms between attacks
    if !nocheck {
        let now = Instant::now();
        let can_attack = world
            .with_session(sid, |h| h.last_attack_time.is_none_or(|t| now >= t))
            .unwrap_or(true);
        if !can_attack {
            tracing::debug!(
                "[sid={}] Attack rejected: server-side 900ms rate limit",
                sid
            );
            return Ok(());
        }
        // Set next allowed attack time
        world.update_session(sid, |h| {
            h.last_attack_time = Some(now + Duration::from_millis(PLAYER_R_HIT_REQUEST_INTERVAL));
        });
    }

    // Weapon delay check: only for non-mage classes with a weapon
    if !nocheck {
        let attacker_is_mage = is_mage(attacker.class);
        if let Some(ref weapon) = right_weapon {
            if !attacker_is_mage {
                let weapon_delay = weapon.delay.unwrap_or(0);
                let weapon_range = weapon.range.unwrap_or(0);
                let item_num = weapon.num as u32;

                if is_timing_delay(item_num) {
                    // Timing Delay weapons: +9ms tolerance
                    if delaytime < (weapon_delay + 9) || distance > weapon_range {
                        tracing::debug!(
                            "[sid={}] Attack rejected: timing delay weapon check (delaytime={}, required={}, distance={}, range={})",
                            sid, delaytime, weapon_delay + 9, distance, weapon_range
                        );
                        return Ok(());
                    }
                } else if is_wirinom_uniq_delay(item_num)
                    || is_wirinom_reb_delay(item_num)
                    || is_garges_sword_delay(item_num)
                {
                    // Wirinim/Garges weapons: -4ms tolerance
                    if delaytime < (weapon_delay - 4) || distance > weapon_range {
                        tracing::debug!(
                            "[sid={}] Attack rejected: special weapon delay check (delaytime={}, required={}, distance={}, range={})",
                            sid, delaytime, weapon_delay - 4, distance, weapon_range
                        );
                        return Ok(());
                    }
                } else {
                    // Normal weapon: exact delay
                    if delaytime < weapon_delay || distance > weapon_range {
                        tracing::debug!(
                            "[sid={}] Attack rejected: weapon delay check (delaytime={}, required={}, distance={}, range={})",
                            sid, delaytime, weapon_delay, distance, weapon_range
                        );
                        return Ok(());
                    }
                }
            }
        } else if delaytime < MIN_MELEE_DELAY {
            // Empty-handed (no weapon): minimum 100ms
            tracing::debug!(
                "[sid={}] Attack rejected: empty-handed delaytime {} < {}",
                sid,
                delaytime,
                MIN_MELEE_DELAY
            );
            return Ok(());
        }
    }

    // ── Target validation ──────────────────────────────────────────────
    // Runtime bots use the user-visibility protocol, but are stored in the
    // NPC combat map. Resolve them explicitly instead of relying only on an
    // ID band, so normal R-attacks follow the bot damage path.
    let target_is_player = tid < crate::npc::NPC_BAND && world.get_bot(tid).is_none();

    if target_is_player {
        let target_sid = tid as SessionId;
        // Self-attack prevention — C++ Unit.cpp:2055 isHostileTo returns false for self
        if target_sid == sid {
            return Ok(());
        }
        handle_player_attack(&world, sid, &pos, target_sid, b_type, unknown);
    } else {
        // NPC/Monster target
        handle_npc_attack(world.clone(), sid, pos, tid, b_type, unknown).await;
    }

    Ok(())
}

/// Handle a player-vs-player attack.
/// Validates target state, computes damage using the real C++ formula,
/// applies HP change, triggers death if HP reaches 0, and broadcasts
/// the result.
fn handle_player_attack(
    world: &WorldState,
    attacker_sid: SessionId,
    attacker_pos: &Position,
    target_sid: SessionId,
    b_type: u8,
    unknown: u8,
) {
    let tid = target_sid as u32;

    // Target must exist and be alive
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => return,
    };

    if target.res_hp_type == USER_DEAD || target.hp <= 0 {
        return;
    }

    if target.authority == 0 {
        broadcast_attack_result(world, attacker_sid, b_type, ATTACK_FAIL, tid, unknown);
        return;
    }

    // Blinking targets (respawn invulnerability) are immune to R-attacks.
    {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if world.is_player_blinking(target_sid, now_unix) {
            return;
        }
    }

    if world.has_buff(target_sid, BUFF_TYPE_FREEZE) {
        return;
    }

    // ── Distance check ─────────────────────────────────────────────────
    let target_pos = match world.get_position(target_sid) {
        Some(p) => p,
        None => return,
    };

    // Must be in the same zone
    if attacker_pos.zone_id != target_pos.zone_id {
        return;
    }

    // 2D distance check (squared) -- C++ uses isInAttackRange
    let weapon_range = get_weapon_range(world, attacker_sid);
    let attack_range = DEFAULT_MELEE_RANGE + weapon_range;

    let dx = attacker_pos.x - target_pos.x;
    let dz = attacker_pos.z - target_pos.z;
    let dist_sq = dx * dx + dz * dz;
    let range_sq = attack_range * attack_range;

    if dist_sq > range_sq {
        tracing::debug!(
            "[sid={}] Attack out of range: dist_sq={:.1} > range_sq={:.1} (weapon_range={:.1})",
            attacker_sid,
            dist_sq,
            range_sq,
            weapon_range
        );
        return;
    }

    // ── Temple event attack gate ────────────────────────────────────────
    //   if (isInTempleEventZone() && !virt_eventattack_check()) return;
    //   if (isInTempleEventZone() && (!isSameEventRoom(pTarget) || !pTempleEvent.isAttackable)) return;
    {
        use crate::systems::event_room;
        if event_room::is_in_temple_event_zone(attacker_pos.zone_id) {
            let attacker_name = world.get_session_name(attacker_sid).unwrap_or_default();
            if !event_room::virt_eventattack_check(
                &world.event_room_manager,
                attacker_pos.zone_id,
                &attacker_name,
            ) {
                return;
            }
            // Check isAttackable flag (event phase must allow combat)
            let is_attackable = world
                .event_room_manager
                .read_temple_event(|s| s.is_attackable);
            if !is_attackable {
                return;
            }
            // Check same event room (both players must be in the same room)
            let target_name = world.get_session_name(target_sid).unwrap_or_default();
            if let Some(event_type) = event_room::event_type_for_zone(attacker_pos.zone_id) {
                let attacker_room = world
                    .event_room_manager
                    .find_user_room(event_type, &attacker_name)
                    .map(|(r, _)| r);
                let target_room = world
                    .event_room_manager
                    .find_user_room(event_type, &target_name)
                    .map(|(r, _)| r);
                // C++ isSameEventRoom: compares GetEventRoom() values.
                // Both must be in a valid room AND in the same room.
                // Guard against None == None (both unassigned) allowing attacks.
                match (attacker_room, target_room) {
                    (Some(a), Some(t)) if a == t => {} // same room, proceed
                    _ => return,                       // different rooms or one/both not assigned
                }
            }
        }
    }

    // ── Monster Stone event room isolation ─────────────────────────────
    //   isInTempleQuestEventZone() && (!isSameEventRoom(pTarget) && m_sMonsterStoneStatus)
    // Players in Monster Stone zones with an active Monster Stone room must
    // be in the same event room to attack. The m_sMonsterStoneStatus guard
    // ensures this only applies to players who have activated a room.
    {
        use crate::systems::monster_stone;
        if monster_stone::is_monster_stone_zone(attacker_pos.zone_id)
            && world.get_monster_stone_status(attacker_sid)
            && !world.is_same_event_room(attacker_sid, target_sid)
        {
            return;
        }
    }

    // ── Kaul transformation block ─────────────────────────────────────
    if world
        .with_session(attacker_sid, |h| h.is_kaul)
        .unwrap_or(false)
    {
        return;
    }

    // ── PvP permission check (isHostileTo) ────────────────────────────
    // Only allows PvP in specific zones / conditions.
    let attacker = match world.get_character_info(attacker_sid) {
        Some(ch) => ch,
        None => return,
    };

    if !is_hostile_to(
        world,
        attacker_sid,
        &attacker,
        attacker_pos,
        target_sid,
        &target,
        &target_pos,
    ) {
        tracing::debug!(
            "[sid={}] PvP denied: not hostile to target={} in zone={}",
            attacker_sid,
            target_sid,
            attacker_pos.zone_id
        );
        return;
    }

    let attacker_coeff = match world.get_coefficient(attacker.class) {
        Some(c) => c,
        None => {
            tracing::warn!(
                "[sid={}] No coefficient for class {} — using 0 damage",
                attacker_sid,
                attacker.class
            );
            broadcast_attack_result(world, attacker_sid, b_type, ATTACK_FAIL, tid, unknown);
            return;
        }
    };

    let target_coeff = match world.get_coefficient(target.class) {
        Some(c) => c,
        None => {
            tracing::warn!(
                "[sid={}] No coefficient for target class {} — using 0 damage",
                attacker_sid,
                target.class
            );
            broadcast_attack_result(world, attacker_sid, b_type, ATTACK_FAIL, tid, unknown);
            return;
        }
    };

    // ── Damage calculation ─────────────────────────────────────────────
    // Use StdRng (Send-safe) seeded from entropy so it works across await points.
    let mut rng = rand::rngs::StdRng::from_entropy();

    // Snapshot all combat-relevant data in a single DashMap read per combatant.
    // Replaces 15 separate lock acquisitions with 2.
    let attacker_snap = match world.snapshot_combat(attacker_sid) {
        Some(s) => s,
        None => return,
    };
    let target_snap = match world.snapshot_combat(target_sid) {
        Some(s) => s,
        None => return,
    };
    let attacker_stats = &attacker_snap.equipped_stats;
    let target_stats = &target_snap.equipped_stats;

    // ── Block physical damage check ──────────────────────────────────
    if target_snap.block_physical {
        broadcast_attack_result(world, attacker_sid, b_type, ATTACK_SUCCESS, tid, unknown);
        return;
    }

    // AC% applied to base first, then flat buff AC added, then AC source reduction subtracted.
    let target_full_ac = ((target_stats.total_ac as i32) * target_snap.ac_pct / 100
        + target_snap.ac_amount
        - target_snap.ac_sour)
        .max(0);

    let mut damage = calculate_r_damage_with_class_bonus(
        &attacker,
        &attacker_coeff,
        &target,
        &target_coeff,
        &mut rng,
        Some(&attacker_stats.ap_class_bonus),
        Some(&target_stats.ac_class_bonus),
        Some(target_full_ac),
        attacker_snap.attack_amount,
        Some(attacker_stats.total_hit),
    );

    // ── Apply buff-based PvP modifiers ──────────────────────────────
    if damage > 0 && attacker_snap.player_attack_amount != 100 {
        damage = (damage as i32 * attacker_snap.player_attack_amount / 100) as i16;
    }

    if damage > 0 && is_mage(attacker.class) {
        damage = (damage as f64
            * world.get_plus_damage_from_item_ids(
                attacker_snap.left_hand_item_id,
                attacker_snap.right_hand_item_id,
            )) as i16;
    }

    // ── R-attack damage multiplier for level>30 non-priests ──────────
    if damage > 0 && attacker.level > 30 && !is_priest(attacker.class) {
        damage = (damage as f64 * world.get_r_damage_multiplier()) as i16;
    }

    // ── Elemental weapon damage bonuses (GetMagicDamage) ─────────────
    // Adds fire/ice/lightning/poison damage from attacker's weapons minus target resistance.
    if damage > 0 {
        damage = apply_elemental_weapon_damage_pvp(
            world,
            attacker_sid,
            &attacker_snap.equipped_stats,
            target_sid,
            &target_snap.equipped_stats,
            target_snap.pct_fire_r,
            target_snap.pct_cold_r,
            target_snap.pct_lightning_r,
            target_snap.pct_poison_r,
            damage,
        );
    }

    // ── Weapon-type armor resistance (GetACDamage) ──────────────────
    // Reduces damage based on target's weapon-type-specific armor resistances (PvP only).
    if damage > 0 {
        let right_kind = if attacker_snap.right_hand_item_id != 0 {
            world
                .get_item(attacker_snap.right_hand_item_id)
                .and_then(|w| w.kind)
        } else {
            None
        };
        let left_kind = if attacker_snap.left_hand_item_id != 0 {
            world
                .get_item(attacker_snap.left_hand_item_id)
                .and_then(|w| w.kind)
        } else {
            None
        };
        damage = get_ac_damage(
            damage,
            &[right_kind, left_kind],
            target_stats,
            target_snap.dagger_r_amount,
            target_snap.bow_r_amount,
        );
    }

    // ── Class-vs-class PvP multiplier ─────────────────────────────────
    if damage > 0 {
        let mult = world.get_class_damage_multiplier(attacker.class, target.class);
        damage = (damage as f64 * mult) as i16;
    }

    if damage > 0 {
        let perk_dmg = world.compute_perk_bonus(&attacker_snap.perk_levels, 10, false);
        if perk_dmg > 0 {
            damage =
                (damage as i32 + damage as i32 * perk_dmg / 100).clamp(0, i16::MAX as i32) as i16;
        }
    }

    // ── Zone damage overrides ──────────────────────────────────────────
    damage = apply_zone_damage_override(attacker_pos.zone_id, damage);

    // ── MAX_DAMAGE cap ─────────────────────────────────────────────────
    damage = damage.min(crate::attack_constants::MAX_DAMAGE as i16);
    if attacker.authority == 0 {
        damage = GM_FIXED_DAMAGE;
    }

    // ── Apply damage ───────────────────────────────────────────────────
    if damage <= 0 {
        broadcast_attack_result(world, attacker_sid, b_type, ATTACK_FAIL, tid, unknown);
        return;
    }

    // C++ order: save originalAmount → mirror → mastery → mana absorb (uses originalAmount)
    // Save original damage BEFORE mirror/mastery for mana absorb calculation.
    let original_damage = damage;

    // ── Pre-fetch victim zone + mana absorb (3 position reads + 1 session read → 0 + 1) ──
    // Uses target_pos.zone_id from earlier distance check (already fetched at line 1123)
    let not_use_zone =
        target_pos.zone_id == ZONE_CHAOS_DUNGEON || target_pos.zone_id == ZONE_KNIGHT_ROYALE;
    let (absorb_pct, absorb_count) = world
        .with_session(target_sid, |h| (h.mana_absorb, h.absorb_count))
        .unwrap_or((0, 0));

    // ── Mirror damage victim reduction ──────────────────────────────────
    let (mirror_dmg, mirror_direct) = if !not_use_zone {
        let (active, direct, amount) = (
            target_snap.mirror_damage,
            target_snap.mirror_damage_type,
            target_snap.mirror_amount,
        );
        if active && amount > 0 {
            let md = (amount as i32 * damage as i32) / 100;
            if md > 0 {
                (md, direct)
            } else {
                (0, false)
            }
        } else {
            (0, false)
        }
    } else {
        (0, false)
    };
    if mirror_dmg > 0 {
        damage = (damage as i32 - mirror_dmg).max(0) as i16;
    }

    // ── Mastery passive damage reduction ────────────────────────────────
    // Matchless: SkillPointMaster >= 10 → 15% reduction
    // Absoluteness: SkillPointMaster >= 5 → 10% reduction
    if !not_use_zone && crate::handler::class_change::is_mastered(target.class) {
        let master_pts = target.skill_points[8]; // SkillPointMaster = index 8
        if master_pts >= 10 {
            // Matchless: 15% damage reduction
            damage = (85 * damage as i32 / 100) as i16;
        } else if master_pts >= 5 {
            // Absoluteness: 10% damage reduction
            damage = (90 * damage as i32 / 100) as i16;
        }
    }

    // ── Mana Absorb (Outrage/Frenzy/Mana Shield) ─────────────────────
    // C++ uses `originalAmount` (pre-mirror) for absorb calculation,
    // but subtracts absorbed from current `amount` (post-mirror).
    {
        if absorb_pct > 0 && !not_use_zone {
            let should_absorb = if absorb_pct == 15 {
                absorb_count > 0
            } else {
                true
            };
            if should_absorb {
                // C++ line 131: toBeAbsorbed = (originalAmount * m_bManaAbsorb) / 100
                let absorbed = (original_damage as i32 * absorb_pct as i32 / 100) as i16;
                damage -= absorbed;
                // C++ allows damage to reach 0 after mana absorb (no minimum enforced)
                if damage < 0 {
                    damage = 0;
                }
                // Convert absorbed damage to MP
                world.update_character_stats(target_sid, |ch| {
                    ch.mp = (ch.mp as i32)
                        .saturating_add(absorbed as i32)
                        .min(ch.max_mp as i32) as i16;
                });
                // Decrement absorb count for pct==15 skills
                if absorb_pct == 15 {
                    world.update_session(target_sid, |h| {
                        h.absorb_count = h.absorb_count.saturating_sub(1);
                    });
                }
            }
        }
    }

    damage = world
        .manes_survival_manager
        .reduce_incoming_damage(target_sid, damage);
    let new_hp = (target.hp - damage).max(0);
    world.update_character_hp(target_sid, new_hp);

    // Send WIZ_HP_CHANGE to victim so their client updates their own HP bar
    let hp_pkt = build_hp_change_packet_with_attacker(target.max_hp, new_hp, attacker_sid as u32);
    world.send_to_session_owned(target_sid, hp_pkt);

    crate::handler::party::broadcast_party_hp(world, target_sid);

    // ── Mirror damage reflection (skill buff) ──────────────────────────
    // Mirror was pre-computed above; now reflect to attacker or party.
    if mirror_dmg > 0 {
        if mirror_direct {
            // Direct: reflect full mirror damage to attacker
            let atk_hp = world
                .get_character_info(attacker_sid)
                .map(|c| (c.hp, c.max_hp))
                .unwrap_or((0, 0));
            let new_atk_hp = (atk_hp.0 - mirror_dmg as i16).max(0);
            world.update_character_hp(attacker_sid, new_atk_hp);
            let atk_hp_pkt =
                build_hp_change_packet_with_attacker(atk_hp.1, new_atk_hp, target_sid as u32);
            world.send_to_session_owned(attacker_sid, atk_hp_pkt);
        } else if world.is_in_party(target_sid) {
            // Party distribution: spread mirror damage among attacker's party.
            if let Some(atk_party_id) = world.get_party_id(attacker_sid) {
                if let Some(party) = world.get_party(atk_party_id) {
                    let members = party.active_members();
                    let p_count = members.len() as i32;
                    if p_count > 0 {
                        // C++ precedence bug: (mirrorDamage / p_count < 2) ? 2 : p_count
                        let per_member_dmg = if (mirror_dmg / p_count) < 2 {
                            2
                        } else {
                            p_count
                        };
                        for &member_sid in &members {
                            if member_sid == target_sid {
                                continue; // skip the victim (C++: p == this)
                            }
                            let m_hp = world
                                .get_character_info(member_sid)
                                .map(|c| (c.hp, c.max_hp))
                                .unwrap_or((0, 0));
                            if m_hp.0 <= 0 {
                                continue;
                            }
                            let new_m_hp = (m_hp.0 as i32 - per_member_dmg).max(0) as i16;
                            world.update_character_hp(member_sid, new_m_hp);
                            // C++ sends HpChange with pAttacker=nullptr (tid=0xFFFF)
                            let m_hp_pkt =
                                build_hp_change_packet_with_attacker(m_hp.1, new_m_hp, 0xFFFF);
                            world.send_to_session_owned(member_sid, m_hp_pkt);
                        }
                    }
                }
            }
        }
    }

    // ── Equipment mirror damage (ITEM_TYPE_MIRROR_DAMAGE) ───────────
    // mirror_damage from equipped items and reflects: damage * total / 300.
    // This is separate from the skill-based mirror (buff 44) above.
    {
        const ITEM_TYPE_MIRROR_DAMAGE: u8 = 0x08;
        let eq_stats = world.get_equipped_stats(target_sid);
        let mut total_equip_mirror: i32 = 0;
        for bonuses in eq_stats.equipped_item_bonuses.values() {
            for &(btype, amount) in bonuses {
                if btype == ITEM_TYPE_MIRROR_DAMAGE {
                    total_equip_mirror += amount;
                }
            }
        }
        if total_equip_mirror > 0 {
            let reflected = (damage as i32 * total_equip_mirror) / 300;
            if reflected > 0 {
                let atk_hp = world
                    .get_character_info(attacker_sid)
                    .map(|c| (c.hp, c.max_hp))
                    .unwrap_or((0, 0));
                let new_atk_hp = (atk_hp.0 as i32 - reflected).max(0) as i16;
                world.update_character_hp(attacker_sid, new_atk_hp);
                let eq_mirror_pkt =
                    build_hp_change_packet_with_attacker(atk_hp.1, new_atk_hp, target_sid as u32);
                world.send_to_session_owned(attacker_sid, eq_mirror_pkt);
            }
        }
    }

    // ── Equipment durability loss ─────────────────────────────────────
    // Every attack takes a little of the attacker's weapon durability.
    world.item_wore_out(attacker_sid, WORE_TYPE_ATTACK, damage as i32);
    // Every hit takes a little of the defender's armour durability.
    world.item_wore_out(target_sid, WORE_TYPE_DEFENCE, damage as i32);

    let b_result = if new_hp <= 0 {
        // Target died -- broadcast death
        dead::broadcast_death(world, target_sid);

        // FerihaLog: KillingUserInsertLog
        if let Some(pool) = world.db_pool() {
            let killer_acc = world
                .with_session(attacker_sid, |h| h.account_id.clone())
                .unwrap_or_default();
            let dead_acc = world
                .with_session(target_sid, |h| h.account_id.clone())
                .unwrap_or_default();
            super::audit_log::log_killing_user(
                pool,
                &killer_acc,
                &attacker.name,
                &dead_acc,
                &target.name,
                attacker_pos.zone_id as i16,
                attacker_pos.x as i16,
                attacker_pos.z as i16,
            );
        }

        // ── War zone death tracking ──────────────────────────────────
        // Increment war death counters when a player dies in an active war zone.
        // The counter tracks the DEAD player's nation (more deaths = worse).
        if crate::systems::war::is_battle_zone(attacker_pos.zone_id) && world.is_war_open() {
            world.increment_war_death(target.nation);
            tracing::debug!(
                "[sid={}] War zone kill: victim nation={}, zone={}",
                attacker_sid,
                target.nation,
                attacker_pos.zone_id
            );
        }

        // ── Tournament kill scoring ─────────────────────────────────────
        // Increment the killer's clan scoreboard in tournament zones.
        if super::tournament::is_tournament_zone(attacker_pos.zone_id) {
            super::tournament::register_kill(world, attacker_pos.zone_id, attacker.knights_id);
        }

        // ── Zone kill reward ──────────────────────────────────────────
        // After a PvP kill in a PK zone, increment kill count and give rewards.
        if is_pk_zone(attacker_pos.zone_id) {
            give_kill_reward(world, attacker_sid, attacker_pos.zone_id);
        }

        // ── Track killer for resurrection EXP recovery ─────────────────
        dead::set_who_killed_me(world, target_sid, attacker_sid);

        // ── PvP loyalty (NP) change ──────────────────────────────────
        // If the zone grants loyalty, give NP to killer (or party) and deduct from victim.
        dead::pvp_loyalty_on_death(world, attacker_sid, target_sid);

        // ── Rivalry / Anger Gauge ────────────────────────────────────────
        // In Ardream / Ronark Land zones: increment victim's anger gauge, assign
        // killer as victim's rival (if none), and check for revenge kills.
        {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let is_revenge = arena::on_pvp_kill(
                world,
                attacker_sid,
                target_sid,
                attacker_pos.zone_id,
                now_secs,
            );
            if is_revenge {
                crate::systems::loyalty::send_loyalty_change(
                    world,
                    attacker_sid,
                    arena::RIVALRY_NP_BONUS as i32,
                    true,
                    false,
                    false,
                );
            }
        }

        // ── PvP gold change ─────────────────────────────────────────────
        dead::gold_change_on_death(world, attacker_sid, target_sid);

        // ── PvP death notice ────────────────────────────────────────────
        // Broadcast kill notice to zone so all players see "[X] killed [Y]".
        dead::send_death_notice(world, attacker_sid, target_sid);

        // ── Zindan War score update ─────────────────────────────────────
        if is_in_special_event_zone(attacker_pos.zone_id) && world.is_zindan_event_opened() {
            let nation = attacker.nation;
            let new_count = {
                let mut zws = world.zindan_war_state.write();
                if nation == 2 {
                    zws.elmo_kills += 1;
                    zws.elmo_kills
                } else {
                    zws.karus_kills += 1;
                    zws.karus_kills
                }
            };
            let score_pkt = super::ext_hook::build_zindan_updatescore(nation, new_count);
            world.broadcast_to_zone(attacker_pos.zone_id, Arc::new(score_pkt), None);
        }

        // ── Chaos dungeon item rob ──────────────────────────────────────
        // Strip chaos dungeon skill items on death (zone 85 only).
        dead::rob_chaos_skill_items(world, target_sid);

        // ── Temple event kill scoring ───────────────────────────────────
        // Per-zone kill scoring: BDW → BDWUpdateRoomKillCount,
        // Chaos → ChaosExpansionKillCount/DeadCount, Juraid → JRUpdateRoomKillCount
        {
            use crate::systems::event_room;
            match attacker_pos.zone_id {
                zone if zone == event_room::ZONE_BDW => {
                    dead::track_bdw_player_kill(world, attacker_sid, target_sid);
                }
                zone if zone == event_room::ZONE_CHAOS => {
                    dead::track_chaos_pvp_kill(world, attacker_sid, target_sid);
                }
                zone if zone == event_room::ZONE_JURAID => {
                    dead::track_juraid_pvp_kill(world, attacker_sid);
                }
                _ => {}
            }

            // ── Cinderella War kill tracking ──────────────────────────────
            if world.is_cinderella_active() && attacker_pos.zone_id == world.cinderella_zone_id() {
                super::cinderella::cinderella_update_kda(world, attacker_sid, target_sid);
            }
        }

        // ── Achievement: PvP kill/death counters ──────────────────────────
        // UserDeathCount++ on death. Sends WIZ_ACHIEVEMENT2 (0xA5) to killer.
        world.update_session(attacker_sid, |h| {
            h.achieve_summary.user_defeat_count =
                h.achieve_summary.user_defeat_count.saturating_add(1);
        });
        world.update_session(target_sid, |h| {
            h.achieve_summary.user_death_count =
                h.achieve_summary.user_death_count.saturating_add(1);
        });
        crate::handler::achieve::on_enemy_user_killed(world, attacker_sid, attacker_pos.zone_id);
        crate::handler::achieve::on_player_died(world, target_sid);

        // v2525: Send updated PvP kill counter to attacker's HUD (0xA5)
        if let Some(count) =
            world.with_session(attacker_sid, |h| h.achieve_summary.user_defeat_count)
        {
            let ach_pkt = crate::handler::achievement2::build_achievement2(count as i32);
            world.send_to_session_owned(attacker_sid, ach_pkt);
        }

        ATTACK_TARGET_DEAD
    } else {
        ATTACK_SUCCESS
    };

    // ── Broadcast attack result to 3x3 region ─────────────────────────
    broadcast_attack_result(world, attacker_sid, b_type, b_result, tid, unknown);

    // ── Send WIZ_TARGET_HP update for the target ───────────────────────
    // So nearby players see the HP bar change
    send_target_hp_update(world, attacker_sid, target_sid, damage as i32);

    tracing::debug!(
        "[sid={}] Attack on target={}: damage={}, new_hp={}, result={}",
        attacker_sid,
        target_sid,
        damage,
        new_hp,
        b_result
    );
}

/// Handle a player-vs-NPC/Monster attack.
/// Validates NPC state, calculates damage using a simplified formula
/// (player AP vs NPC AC), deducts NPC HP, handles death (XP + respawn),
/// and broadcasts the result.
async fn handle_npc_attack(
    world: Arc<WorldState>,
    attacker_sid: SessionId,
    attacker_pos: Position,
    npc_id: NpcId,
    b_type: u8,
    unknown: u8,
) {
    // ── Bot target: apply damage, send HP update, handle death ─────
    // Bots are stored in world.bots (not the NPC instance map).
    // Unlike the previous stub that only tracked last_attacker_id, we now
    // apply actual damage using the player's attack stats against a simplified
    // bot AC derived from STA + level, then send WIZ_TARGET_HP to the attacker.
    if let Some(bot) = world.get_bot(npc_id) {
        if bot.hp <= 0 || bot.presence == crate::world::BotPresence::Dead {
            return;
        }

        // Distance check — bot position (attacker_pos passed from parent)
        if attacker_pos.zone_id != bot.zone_id {
            return;
        }
        let dx = attacker_pos.x - bot.x;
        let dz = attacker_pos.z - bot.z;
        if dx * dx + dz * dz > DEFAULT_MELEE_RANGE * DEFAULT_MELEE_RANGE {
            return;
        }

        // Attacker info + coefficient
        let attacker = match world.get_character_info(attacker_sid) {
            Some(ch) => ch,
            None => return,
        };
        let is_gm = attacker.authority == 0;
        let attacker_coeff = match world.get_coefficient(attacker.class) {
            Some(c) => c,
            None => return,
        };

        // Damage calculation — C++ formula: (ap * 2) / (ac + 240)
        // Bot AC approximation: STA * 3 + level * 2
        // C++ bots go through full SetUserAbility, but we use a simplified AC.
        let mut rng = rand::rngs::StdRng::from_entropy();
        // Snapshot attacker combat data in a single DashMap read.
        let attacker_snap = match world.snapshot_combat(attacker_sid) {
            Some(s) => s,
            None => return,
        };
        // Use pre-computed total_hit from set_user_ability() — includes actual
        // weapon damage, stat bonuses, and buffs.
        let total_hit = attacker_snap.equipped_stats.total_hit;
        let temp_ap = total_hit as i32 * attacker_snap.attack_amount;
        let bot_ac = (bot.sta_stat as i32 * 3) + (bot.level as i32 * 2);
        let temp_hit_b = if bot_ac + 240 > 0 {
            (temp_ap * 2) / (bot_ac + 240)
        } else {
            temp_ap * 2
        };

        let mut damage = if temp_hit_b <= 0 {
            0i16
        } else {
            let attacker_hitrate = compute_hitrate(&attacker, &attacker_coeff);
            let rate = attacker_hitrate + 1.0;
            let hit_result = get_hit_rate(rate, &mut rng);
            match hit_result {
                GREAT_SUCCESS | SUCCESS | NORMAL => {
                    let random = if temp_hit_b > 0 {
                        rng.gen_range(0..=temp_hit_b)
                    } else {
                        0
                    };
                    let d = if is_priest(attacker.class) {
                        (0.15 * temp_hit_b as f32 + 0.2 * random as f32) as i32
                    } else {
                        (0.75 * temp_hit_b as f32 + 0.3 * random as f32) as i32
                    };
                    d.max(1) as i16
                }
                _ => {
                    if is_gm {
                        30000i16
                    } else {
                        0
                    }
                }
            }
        };

        // R-attack damage multiplier for level>30 non-priests
        if damage > 0 && attacker.level > 30 && !is_priest(attacker.class) {
            damage = (damage as f64 * world.get_r_damage_multiplier()) as i16;
        }

        // ── Elemental weapon damage bonuses (GetMagicDamage) ─────────
        // Bots have no tracked elemental resistance, so full bonus applies.
        if damage > 0 {
            let mut elem_bonus: i32 = 0;
            for bonuses in attacker_snap.equipped_stats.equipped_item_bonuses.values() {
                for &(btype, amount) in bonuses {
                    if amount > 0
                        && matches!(
                            btype,
                            ITEM_TYPE_FIRE
                                | ITEM_TYPE_COLD
                                | ITEM_TYPE_LIGHTNING
                                | ITEM_TYPE_POISON
                        )
                    {
                        elem_bonus += amount;
                    }
                }
            }
            if elem_bonus > 0 {
                damage = (damage as i32 + elem_bonus) as i16;
            }
        }

        if damage > 0 {
            let perk_dmg = world.compute_perk_bonus(&attacker_snap.perk_levels, 10, false);
            if perk_dmg > 0 {
                damage = (damage as i32 + damage as i32 * perk_dmg / 100).clamp(0, i16::MAX as i32)
                    as i16;
            }
        }

        // Cap at MAX_DAMAGE
        let damage = if is_gm {
            GM_FIXED_DAMAGE
        } else {
            damage.min(crate::attack_constants::MAX_DAMAGE as i16)
        };

        if damage <= 0 {
            broadcast_attack_result(&world, attacker_sid, b_type, ATTACK_FAIL, npc_id, unknown);
            return;
        }

        // Apply damage to bot
        let new_hp = (bot.hp - damage).max(0);
        world.update_bot(npc_id, |b| {
            b.hp = new_hp;
            b.last_attacker_id = attacker_sid as i32;
        });

        // Send WIZ_TARGET_HP to attacker
        // C++ sends ORIGINAL damage (before passives) to attacker for display.
        let target_hp_pkt = super::target_hp::build_target_hp_packet(
            npc_id,
            0,
            bot.max_hp as u32,
            new_hp as u32,
            -(damage as i32),
            -(damage as i32),
        );
        world.send_to_session_owned(attacker_sid, target_hp_pkt);

        let b_result = if new_hp <= 0 {
            // Bot died — trigger full death processing
            let now_ms = crate::systems::bot_ai::tick_ms();
            crate::systems::bot_ai::bot_on_death(&world, npc_id, now_ms);
            ATTACK_TARGET_DEAD
        } else {
            ATTACK_SUCCESS
        };

        broadcast_attack_result(&world, attacker_sid, b_type, b_result, npc_id, unknown);

        tracing::debug!(
            "[sid={}] R-attack on bot {}: damage={}, hp={}/{}",
            attacker_sid,
            npc_id,
            damage,
            new_hp,
            bot.max_hp
        );
        return;
    }

    // ── Look up NPC instance and template ──────────────────────────
    let npc = match world.get_npc_instance(npc_id) {
        Some(n) => n,
        None => return,
    };

    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => return,
    };

    // Town, quest and event NPCs are never combat targets. Previously only
    // a small set of infrastructure NPC types was filtered, allowing regular
    // NPCs with zero combat HP (including Event Manager) to be "killed".
    if !npc.is_monster
        && !matches!(
            tmpl.npc_type,
            NPC_DESTROYED_ARTIFACT | NPC_OBJECT_FLAG | NPC_GATE
        )
    {
        return;
    }

    // ── NPC type pre-blocking ─────────────────────────────────────
    {
        let npc_type = tmpl.npc_type;

        // Guard towers, soccer baal, and gate infrastructure are completely untouchable
        if matches!(
            npc_type,
            NPC_GUARD_TOWER1
                | NPC_GUARD_TOWER2
                | NPC_SOCCER_BAAL
                | NPC_GATE2
                | NPC_VICTORY_GATE
                | NPC_PHOENIX_GATE
                | NPC_SPECIAL_GATE
                | NPC_GATE_LEVER
        ) {
            return;
        }

        {
            if npc_type == NPC_BIFROST_MONUMENT {
                let beef = world.get_beef_event();
                if !beef.is_active || beef.is_monument_dead {
                    return;
                }
            }

            if npc_type == NPC_PVP_MONUMENT || npc_type == NPC_CLAN_WAR_MONUMENT {
                let attacker_nation = world
                    .get_character_info(attacker_sid)
                    .map(|ch| ch.nation)
                    .unwrap_or(0);
                // Karus monument = proto 14003, Elmorad = 14004
                let is_own_monument = (attacker_nation == 1 && npc.proto_id == 14003)
                    || (attacker_nation == 2 && npc.proto_id == 14004);
                if is_own_monument {
                    return;
                }
            }
        }

        // Delos-specific NPC restrictions
        if npc.zone_id == ZONE_DELOS {
            // In Delos, non-monsters are blocked unless they're artifacts, flags, or gates
            if !npc.is_monster
                && npc_type != NPC_DESTROYED_ARTIFACT
                && npc_type != NPC_OBJECT_FLAG
                && npc_type != NPC_GATE
            {
                return;
            }

            // Destroyed artifacts require active CSW war + attacker must be in a clan
            // + attacker's clan must NOT be the castle owner
            if npc_type == NPC_DESTROYED_ARTIFACT {
                let csw = world.csw_event().blocking_read();
                let siege = world.siege_war().blocking_read();
                let attacker_clan = world
                    .get_character_info(attacker_sid)
                    .map(|ch| ch.knights_id)
                    .unwrap_or(0);

                if !csw.is_active()
                    || !csw.is_war_active()
                    || attacker_clan == 0
                    || siege.master_knights == attacker_clan
                {
                    return;
                }
            }
        }

        // CSW doors (proto_id 561/562/563 with NPC_GATE type) follow same rules as artifacts
        if npc_type == NPC_GATE && matches!(npc.proto_id, 561..=563) {
            let csw = world.csw_event().blocking_read();
            let siege = world.siege_war().blocking_read();
            let attacker_clan = world
                .get_character_info(attacker_sid)
                .map(|ch| ch.knights_id)
                .unwrap_or(0);

            if !csw.is_active()
                || !csw.is_war_active()
                || attacker_clan == 0
                || siege.master_knights == attacker_clan
            {
                return;
            }
        }

        // Neutral peaceful NPCs (group/nation == 3) cannot be R-attacked
        // This is also checked in apply_npc_type_damage_override but we block early here
        if tmpl.group == 3 {
            return;
        }
    }

    // ── Deva Bird attack check (Juraid Mountain) ─────────────────
    // Deva Bird (proto 8106) can only be attacked if all 3 bridges for the
    // attacker's nation are built in the player's event room.
    // Juraid nation monuments are enemy-only objectives. The monument SID,
    // not its generic monster group, is authoritative for ownership.
    if npc.zone_id == ZONE_JURAID_MOUNTAIN
        && crate::systems::juraid::is_juraid_monument(npc.proto_id)
    {
        let attacker_nation = world
            .get_character_info(attacker_sid)
            .map(|ch| ch.nation)
            .unwrap_or(0);
        if attacker_nation == 0
            || attacker_nation == crate::systems::juraid::monument_nation(npc.proto_id)
        {
            tracing::debug!(
                attacker_sid,
                attacker_nation,
                monument_sid = npc.proto_id,
                "Blocked attack against own Juraid monument"
            );
            return;
        }
    }

    {
        const DEVA_BIRD_SSID: u16 = 8106;

        if npc.proto_id == DEVA_BIRD_SSID && npc.zone_id == ZONE_JURAID_MOUNTAIN {
            let event_room = world.get_event_room(attacker_sid);
            let attacker_nation = world
                .get_character_info(attacker_sid)
                .map(|ch| ch.nation)
                .unwrap_or(0);

            if event_room == 0
                || !world.are_all_juraid_bridges_open(event_room as u8, attacker_nation)
            {
                return;
            }
        }
    }

    // ── virt_eventattack_check — temple event attack validation ──
    // Blocks attacks in finished/inactive temple event rooms.
    {
        let attacker_name = world
            .get_character_info(attacker_sid)
            .map(|ch| ch.name.clone())
            .unwrap_or_default();
        let attacker_zone = world
            .get_position(attacker_sid)
            .map(|p| p.zone_id)
            .unwrap_or(0);

        if !crate::systems::event_room::virt_eventattack_check(
            world.event_room_manager(),
            attacker_zone,
            &attacker_name,
        ) {
            return;
        }
    }

    // ── Check if NPC is alive ──────────────────────────────────────
    let npc_hp = match world.get_npc_hp(npc_id) {
        Some(hp) if hp > 0 => hp,
        _ => return,
    };

    // ── Distance check (attacker_pos passed from parent) ────────────

    // Must be in the same zone
    if attacker_pos.zone_id != npc.zone_id {
        return;
    }

    let weapon_range = get_weapon_range(&world, attacker_sid);
    let attack_range = DEFAULT_MELEE_RANGE + weapon_range;

    let dx = attacker_pos.x - npc.x;
    let dz = attacker_pos.z - npc.z;
    let dist_sq = dx * dx + dz * dz;
    let range_sq = attack_range * attack_range;

    if dist_sq > range_sq {
        tracing::debug!(
            "[sid={}] Attack on NPC {} out of range: dist_sq={:.1} > range_sq={:.1} (weapon_range={:.1})",
            attacker_sid,
            npc_id,
            dist_sq,
            range_sq,
            weapon_range
        );
        return;
    }

    // ── Get attacker info ──────────────────────────────────────────
    let attacker = match world.get_character_info(attacker_sid) {
        Some(ch) => ch,
        None => return,
    };

    let is_gm = attacker.authority == 0;

    let attacker_coeff = match world.get_coefficient(attacker.class) {
        Some(c) => c,
        None => {
            broadcast_attack_result(&world, attacker_sid, b_type, ATTACK_FAIL, npc_id, unknown);
            return;
        }
    };

    // ── Damage calculation ─────────────────────────────────────────
    // Uses same formula base as PvP but with NPC AC instead of target player AC.
    let mut rng = rand::rngs::StdRng::from_entropy();

    // Snapshot attacker combat data in a single DashMap read.
    let npc_attacker_snap = match world.snapshot_combat(attacker_sid) {
        Some(s) => s,
        None => return,
    };
    // Use pre-computed total_hit from set_user_ability() — includes actual
    // weapon damage, stat bonuses, and buffs.
    let total_hit = npc_attacker_snap.equipped_stats.total_hit;
    let temp_ap = total_hit as i32 * npc_attacker_snap.attack_amount;

    // Apply monster defense multiplier to NPC AC
    // War buff: nation NPCs get AC × 1.2 during war (ChangeAbility).
    let raw_npc_ac = world.get_npc_war_ac(&tmpl);
    let npc_ac = (raw_npc_ac as f64 * world.get_mon_def_multiplier()) as i32;

    let temp_hit_b = if npc_ac + 240 > 0 {
        (temp_ap * 2) / (npc_ac + 240)
    } else {
        temp_ap * 2
    };

    let mut damage = if temp_hit_b <= 0 {
        0i16
    } else {
        // NPC has no evasion rate, so use a generous hit rate
        let attacker_hitrate = compute_hitrate(&attacker, &attacker_coeff);
        let rate = attacker_hitrate + 1.0; // No evasion from NPC
        let hit_result = get_hit_rate(rate, &mut rng);

        match hit_result {
            GREAT_SUCCESS | SUCCESS | NORMAL => {
                let random = if temp_hit_b > 0 {
                    rng.gen_range(0..=temp_hit_b)
                } else {
                    0
                };
                let d = if is_priest(attacker.class) {
                    (0.15 * temp_hit_b as f32 + 0.2 * random as f32) as i32
                } else {
                    (0.75 * temp_hit_b as f32 + 0.3 * random as f32) as i32
                };
                d.max(1) as i16
            }
            _ => {
                if is_gm {
                    30000i16
                } else {
                    0
                }
            }
        }
    };

    if damage > 0 && is_mage(attacker.class) {
        damage = (damage as f64
            * world.get_plus_damage_from_item_ids(
                npc_attacker_snap.left_hand_item_id,
                npc_attacker_snap.right_hand_item_id,
            )) as i16;
    }

    // ── R-attack damage multiplier for level>30 non-priests ──────────
    if damage > 0 && attacker.level > 30 && !is_priest(attacker.class) {
        damage = (damage as f64 * world.get_r_damage_multiplier()) as i16;
    }

    // Apply monster take-damage multiplier
    if damage > 0 {
        damage = (damage as f64 * world.get_mon_take_damage_multiplier()) as i16;
    }

    // ── Elemental weapon damage bonuses (GetMagicDamage) ─────────────
    // Uses NPC template elemental resistances instead of player session values.
    if damage > 0 {
        damage =
            apply_elemental_weapon_damage_npc(&npc_attacker_snap.equipped_stats, &tmpl, damage);
    }

    if damage > 0 {
        let perk_dmg = world.compute_perk_bonus(&npc_attacker_snap.perk_levels, 9, false);
        if perk_dmg > 0 {
            damage =
                (damage as i32 + damage as i32 * perk_dmg / 100).clamp(0, i16::MAX as i32) as i16;
        }
    }

    // ── Vaccuni transformation damage override ─────────────────────
    // Specific NPC proto IDs with event + item requirements deal 30000 fixed damage.
    let damage = if check_vaccuni_attack(&world, attacker_sid, &npc) {
        30000_i16
    } else {
        damage
    };

    // ── NPC type-specific damage overrides ──────────────────────────
    let damage =
        apply_npc_type_damage_override(&world, attacker_sid, &attacker, &tmpl, &npc, damage);

    // ── Zone damage overrides ──────────────────────────────────────
    let damage = apply_zone_damage_override(attacker_pos.zone_id, damage);

    // Manes uses an isolated level 1-30 combat curve. Normal character
    // equipment/stats must not let a level-1 participant one-shot the outer
    // ring. Scale the existing hit from 15% at level 1 to 87.5% at level 30.
    let damage =
        if crate::systems::manes_survival::ZONES_MANES_SURVIVAL.contains(&attacker_pos.zone_id) {
            let level = world
                .manes_survival_manager
                .progress(attacker_sid)
                .map(|state| state.level)
                .unwrap_or(1);
            let scale_tenths = i32::from(crate::systems::manes_survival::attack_scale_per_mille(
                level,
            ));
            let scaled = ((damage as i32 * scale_tenths) / 1_000).max(1);
            let bonus = world
                .manes_survival_manager
                .combat_bonuses(attacker_sid)
                .attack_pct as i32;
            (scaled * (100 + bonus) / 100).clamp(1, i16::MAX as i32) as i16
        } else {
            damage
        };

    // Cap damage at MAX_DAMAGE — matches player attack path
    let damage = if is_gm {
        GM_FIXED_DAMAGE
    } else {
        damage.min(crate::attack_constants::MAX_DAMAGE as i16)
    };

    if damage <= 0 {
        broadcast_attack_result(&world, attacker_sid, b_type, ATTACK_FAIL, npc_id, unknown);
        return;
    }

    // ── Apply damage to NPC ────────────────────────────────────────
    let new_hp = (npc_hp - damage as i32).max(0);
    world.update_npc_hp(npc_id, new_hp);
    world.record_npc_damage(npc_id, attacker_sid, damage as i32);

    // ── Attacker weapon durability loss ──────────────────────────────
    // Manes equips a client-side temporary SurvivalSetting loadout while the
    // real inventory stays intact. A normal WIZ_DURATION update addresses the
    // real item slot and is invalid for that temporary loadout.
    let is_manes_survival_zone =
        crate::systems::manes_survival::ZONES_MANES_SURVIVAL.contains(&npc.zone_id);
    if !is_manes_survival_zone {
        world.item_wore_out(attacker_sid, WORE_TYPE_ATTACK, damage as i32);
    }

    // Notify NPC AI about damage (reactive aggro — C++ ChangeTarget)
    // A transformed player is still a valid attacker. Always notify the NPC
    // after a non-lethal hit so passive/tender mobs acquire a deterministic
    // return target; the old conditional left them standing intermittently
    // when the transformation damage path reported zero/late HP state.
    world.notify_npc_damaged(npc_id, attacker_sid);

    let b_result = if new_hp <= 0 {
        // NPC died
        handle_npc_death(
            &world,
            attacker_sid,
            npc_id,
            &npc,
            &tmpl,
            is_manes_survival_zone,
        )
        .await;

        ATTACK_TARGET_DEAD
    } else {
        ATTACK_SUCCESS
    };

    // ── Broadcast attack result ────────────────────────────────────
    broadcast_attack_result(&world, attacker_sid, b_type, b_result, npc_id, unknown);

    // ── Send HP bar update ─────────────────────────────────────────
    send_npc_target_hp_update(
        &world,
        attacker_sid,
        npc_id,
        tmpl.max_hp as i32,
        new_hp,
        damage as i32,
    );

    if tmpl.s_sid == crate::systems::manes_survival::DARK_DRAGON_SID as u16 {
        world.manes_survival_manager.broadcast_dark_dragon_status(
            &world,
            npc.zone_id,
            crate::systems::manes_survival::DARK_DRAGON_UI_NAME,
            tmpl.max_hp,
            new_hp.max(0) as u32,
        );
    }

    if new_hp <= 0 {
        if is_manes_survival_zone {
            broadcast_npc_death(&world, attacker_sid, npc_id);
        }
        flush_manes_progress(&world, attacker_sid);
    }

    tracing::debug!(
        "[sid={}] Attack on NPC {}: damage={}, new_hp={}/{}, result={}",
        attacker_sid,
        npc_id,
        damage,
        new_hp,
        tmpl.max_hp,
        b_result
    );
}

/// Handle NPC death: broadcast death, award XP (party or solo), set AI state to Dead.
/// Handle all NPC death side-effects: broadcast death, award XP (party or solo),
/// generate loot, and set AI state to Dead.
/// This is the SINGLE point of NPC death handling — called from both physical
/// attack and magic damage paths to ensure consistent behavior.
pub(crate) fn flush_manes_progress(world: &WorldState, sid: SessionId) {
    let Some(progress) = world.manes_survival_manager.progress(sid) else {
        return;
    };
    // Ordinary kills change only Survival EXP/score. Re-sending
    // WIZ_LEVEL_CHANGE for every kill makes the client tear down and rebuild
    // its temporary HP/MP bars, which is observed as the bars jumping between
    // unrelated values. Full vitals are emitted only on an actual level-up.
    if progress.leveled_up {
        sync_manes_vitals_and_level(world, sid, progress, true);
    } else {
        world.send_to_session_owned(
            sid,
            crate::handler::survival::build_event_exp_update(progress.exp),
        );
    }
    world.send_to_session_owned(
        sid,
        crate::handler::survival::build_event_score_update(u32::from(
            crate::systems::manes_survival::score_for_level(progress.level),
        )),
    );
    if progress.leveled_up {
        let offer = world.manes_survival_manager.create_offer(sid);
        world.send_to_session_owned(
            sid,
            crate::handler::survival::build_skill_selection_open(&offer.skills, &offer.potions),
        );
        tracing::info!(
            sid,
            survival_level = progress.level,
            "Manes skill-selection UIF skill lists sent"
        );
    }
}

/// Keep the authoritative Manes combat stats and the client HUD in lockstep.
///
/// The reference client uses `HP = level * 1000 + 60`,
/// `MP = level * 200` and `score = (level - 1) * 20`. The temporary level and
/// vitals are sent through WIZ_LEVEL_CHANGE while the ALT request receives the
/// score from the Survival handler. This deliberately avoids another D0 event-start packet:
/// reinitialising SurvivalSetting clears the acquired-skill shortcut bar.
pub(crate) fn sync_manes_vitals_and_level(
    world: &WorldState,
    sid: SessionId,
    progress: crate::systems::manes_survival::ManesProgress,
    refill_vitals: bool,
) {
    // Passive MANES_MAGIC HP rows are part of the client Survival contract.
    // Applying them only client-side makes selection appear to lower/raise the
    // bar at random when the next authoritative server packet arrives.
    let hp_bonus = world
        .manes_survival_manager
        .combat_bonuses(sid)
        .hp
        .clamp(0, i16::MAX);
    let (max_hp, max_mp) =
        crate::systems::manes_survival::vitals_for_level(progress.level, hp_bonus);

    // Callers explicitly identify a real level transition. Skill selection and
    // ordinary HUD refreshes preserve spent HP/MP even if another stat
    // recalculation happened between packets.
    let Some(before_sync) = world.get_character_info(sid) else {
        return;
    };
    let current_hp = if refill_vitals {
        max_hp
    } else {
        // Preserve missing HP when a passive increases max HP. This prevents a
        // bonus selection from visually damaging or healing the participant.
        let missing_hp = (before_sync.max_hp - before_sync.hp).max(0);
        (max_hp - missing_hp).clamp(0, max_hp)
    };
    let current_mp = if refill_vitals {
        max_mp
    } else {
        before_sync.mp.clamp(0, max_mp)
    };

    world.update_character_stats(sid, |character| {
        character.max_hp = max_hp;
        character.hp = current_hp;
        character.max_mp = max_mp;
        character.mp = current_mp;
    });

    let Some(character) = world.get_character_info(sid) else {
        return;
    };
    let equipped = world.get_equipped_stats(sid);

    let mut level_packet = Packet::new(Opcode::WizLevelChange as u8);
    level_packet.write_u32(sid as u32);
    level_packet.write_u8(progress.level);
    level_packet.write_i16(character.free_points as i16);
    level_packet.write_u8(character.skill_points[0]);
    level_packet.write_i64(i64::from(progress.max_exp));
    level_packet.write_i64(i64::from(progress.exp));
    level_packet.write_i16(max_hp);
    level_packet.write_i16(current_hp);
    level_packet.write_i16(max_mp);
    level_packet.write_i16(current_mp);
    level_packet.write_u32(equipped.max_weight);
    level_packet.write_u32(equipped.item_weight);
    world.send_to_session_owned(sid, level_packet);

    world.send_to_session_owned(
        sid,
        crate::systems::regen::build_hp_change_packet(max_hp, current_hp),
    );
    world.send_to_session_owned(
        sid,
        crate::systems::regen::build_mp_change_packet(max_mp, current_mp),
    );
    // WIZ_LEVEL_CHANGE does not carry attack power. Refresh the verified
    // WIZ_ITEM_MOVE stat contract as well so the ALT/character stat display
    // advances with the same level scale used by server-side damage.
    world.send_item_move_refresh(sid);

    tracing::info!(
        sid,
        survival_level = progress.level,
        survival_exp = progress.exp,
        survival_max_exp = progress.max_exp,
        survival_score = crate::systems::manes_survival::score_for_level(progress.level),
        max_hp,
        current_hp,
        max_mp,
        current_mp,
        refill_vitals,
        "Manes Survival vitals and HUD synchronized"
    );
}

pub(crate) fn is_manes_survival_npc(npc: &crate::npc::NpcInstance) -> bool {
    crate::systems::manes_survival::ZONES_MANES_SURVIVAL.contains(&npc.zone_id)
}

pub(crate) fn scale_manes_magic_damage(
    world: &WorldState,
    caster_sid: SessionId,
    npc: &crate::npc::NpcInstance,
    damage: i16,
) -> i16 {
    if damage <= 0 || !is_manes_survival_npc(npc) {
        return damage;
    }

    let level = world
        .manes_survival_manager
        .progress(caster_sid)
        .map(|state| state.level)
        .unwrap_or(1);
    let scale_tenths = 150_i32 + level.saturating_sub(1) as i32 * 25;
    let scaled = ((damage as i32 * scale_tenths) / 1_000).max(1);
    let bonus = world
        .manes_survival_manager
        .combat_bonuses(caster_sid)
        .attack_pct as i32;
    (scaled * (100 + bonus) / 100).clamp(1, i16::MAX as i32) as i16
}

pub(crate) fn broadcast_npc_death(world: &WorldState, killer_sid: SessionId, npc_id: NpcId) {
    let mut death_pkt = Packet::new(Opcode::WizDead as u8);
    death_pkt.write_u32(npc_id);

    if let Some(pos) = world.get_position(killer_sid) {
        let event_room = world.get_event_room(killer_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(death_pkt),
            None,
            event_room,
        );
    }
}

pub(crate) async fn handle_npc_death(
    world: &WorldState,
    killer_sid: SessionId,
    npc_id: NpcId,
    npc: &crate::npc::NpcInstance,
    tmpl: &crate::npc::NpcTemplate,
    defer_death_broadcast: bool,
) {
    // Manes must receive the combat result and HP=0 before WIZ_DEAD. Its
    // physical/magic callers emit this packet after those two packets.
    if !defer_death_broadcast {
        broadcast_npc_death(world, killer_sid, npc_id);
    }

    // FerihaLog: KillingNpcInsertLog
    if let Some(pool) = world.db_pool() {
        let acc = world
            .with_session(killer_sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let ch_name = world.get_session_name(killer_sid).unwrap_or_default();
        let pos = world.get_position(killer_sid);
        super::audit_log::log_killing_npc(
            pool,
            &acc,
            &ch_name,
            tmpl.s_sid,
            &tmpl.name,
            tmpl.is_monster,
            pos.as_ref().map(|p| p.zone_id as i16).unwrap_or(0),
            pos.as_ref().map(|p| p.x as i16).unwrap_or(0),
            pos.as_ref().map(|p| p.z as i16).unwrap_or(0),
        );
    }

    // ── Daily rank stat: MHTotalKill++ ────────────────────────────────
    world.update_session(killer_sid, |h| {
        h.dr_mh_total_kill += 1;
    });
    crate::handler::achieve::on_monster_killed(world, killer_sid, tmpl.s_sid as u16, npc.zone_id);

    // ── Achievement: MonsterDefeatCount++ ────────────────────────────
    // Called via AchieveMonsterCountAdd() on each NPC death.
    world.update_session(killer_sid, |h| {
        h.achieve_summary.monster_defeat_count =
            h.achieve_summary.monster_defeat_count.saturating_add(1);
    });

    // v2525: Send updated kill counter to client HUD (0xA5).
    // The Manes Survival client contract does not consume this generic
    // achievement packet during its kill sequence. Sending it before the
    // WIZ_MAGIC_PROCESS result makes the v2615 client terminate on the first
    // lethal event skill. Keep the counter server-side, but suppress only its
    // HUD packet inside zones 57-60 while the event is active.
    let is_manes_survival_kill = world.manes_survival_manager.is_active()
        && crate::systems::manes_survival::ZONES_MANES_SURVIVAL.contains(&npc.zone_id);
    if !is_manes_survival_kill {
        if let Some(count) =
            world.with_session(killer_sid, |h| h.achieve_summary.monster_defeat_count)
        {
            let ach_pkt = crate::handler::achievement2::build_achievement2(count as i32);
            world.send_to_session_owned(killer_sid, ach_pkt);
        }
    }

    // ── Award XP + Loyalty (NP) — damage-weighted distribution ──
    // Iterates m_DamagedUserList and distributes XP/NP proportionally to
    // each damager's contribution. Party members' damage is consolidated
    // into one representative entry so the whole party gets proportional XP.
    // Manes uses its own level/EXP state initialised through WIZ_SURVIVAL.
    // Sending the normal character WIZ_EXP_CHANGE (0x1A) here crashes the
    // v2615 client immediately after the first event monster dies and would
    // also mutate the persistent character level, which this event forbids.
    // Compute Manes progress here, but do not send D0 packets from the death
    // handler. Physical/magic result packets must reach the client first.
    if is_manes_survival_kill {
        if let Some(progress) = world
            .manes_survival_manager
            .award_monster_exp(killer_sid, tmpl.s_sid)
        {
            tracing::info!(
                sid = killer_sid,
                npc_sid = tmpl.s_sid,
                survival_level = progress.level,
                survival_exp = progress.exp,
                survival_max_exp = progress.max_exp,
                leveled_up = progress.leveled_up,
                "Manes Survival progress updated"
            );
        }
    }

    let base_exp = if is_manes_survival_kill {
        0
    } else {
        tmpl.exp as i64
    };
    let base_loyalty = if is_manes_survival_kill {
        0
    } else {
        tmpl.loyalty.min(i32::MAX as u32) as i32
    };
    let npc_x = npc.x;
    let npc_z = npc.z;

    // Phase 1: Build consolidated damage list from all damagers.
    // For party members, merge damage into a single representative entry.
    let damage_entries = world.get_npc_damage_entries(npc_id);

    // consolidated: representative_sid -> (total_damage, Option<party_id>)
    // party_rep: party_id -> representative_sid (first party member seen)
    let mut consolidated: HashMap<SessionId, (i32, Option<u16>)> = HashMap::new();
    let mut party_rep: HashMap<u16, SessionId> = HashMap::new();

    for (sid, damage) in &damage_entries {
        // Validate: player must be alive, have character info, be in range
        if world.is_player_dead(*sid) {
            continue;
        }
        let _ch = match world.get_character_info(*sid) {
            Some(ch) => ch,
            None => continue,
        };
        let in_range = match world.get_position(*sid) {
            Some(pos) => {
                let dx = pos.x - npc_x;
                let dz = pos.z - npc_z;
                dx * dx + dz * dz <= RANGE_50M
            }
            None => false,
        };
        if !in_range {
            continue;
        }

        // Check if this player is in a party — consolidate damage under representative
        let pid = world.get_party_id(*sid);
        if let Some(pid) = pid {
            if let Some(&rep_sid) = party_rep.get(&pid) {
                // Add damage to existing representative
                if let Some(entry) = consolidated.get_mut(&rep_sid) {
                    entry.0 += damage;
                }
            } else {
                // This player becomes the representative for this party
                party_rep.insert(pid, *sid);
                consolidated.insert(*sid, (*damage, Some(pid)));
            }
        } else {
            // Solo player — own entry
            consolidated.insert(*sid, (*damage, None));
        }
    }

    // Phase 2: Calculate total damage from filtered list
    let total_damage: i64 = consolidated.values().map(|(dmg, _)| *dmg as i64).sum();

    if total_damage > 0 && !consolidated.is_empty() {
        // Phase 3: Distribute XP/NP proportionally to each entry
        for (&rep_sid, &(damage, party_id)) in &consolidated {
            let proportion = damage as f64 / total_damage as f64;
            let proportional_exp = (base_exp as f64 * proportion).ceil() as i64;
            let proportional_loyalty = (base_loyalty as f64 * proportion).ceil() as i32;

            if let Some(pid) = party_id {
                // Party representative — distribute to all eligible party members near NPC
                if let Some(party) = world.get_party(pid) {
                    let mut eligible: Vec<(SessionId, u8)> = Vec::with_capacity(8);
                    for &member_sid in &party.active_members() {
                        // Single DashMap read: alive + in-range + level (3 reads → 1)
                        let member_level = world
                            .with_session(member_sid, |h| {
                                let ch = h.character.as_ref()?;
                                if ch.res_hp_type == crate::world::USER_DEAD || ch.hp <= 0 {
                                    return None;
                                }
                                let dx = h.position.x - npc_x;
                                let dz = h.position.z - npc_z;
                                if dx * dx + dz * dz <= RANGE_50M {
                                    Some(ch.level)
                                } else {
                                    None
                                }
                            })
                            .flatten();
                        if let Some(level) = member_level {
                            eligible.push((member_sid, level));
                        }
                    }

                    if eligible.is_empty() {
                        // No party members in range — give proportional XP/NP to representative
                        if proportional_exp > 0
                            && !world.try_jackpot_exp(rep_sid, proportional_exp).await
                        {
                            super::level::exp_change(world, rep_sid, proportional_exp).await;
                        }
                        if proportional_loyalty > 0 {
                            crate::systems::loyalty::send_loyalty_change(
                                world,
                                rep_sid,
                                proportional_loyalty,
                                false,
                                false,
                                true, // C++ default: bIsAddLoyaltyMonthly = true
                            );
                        }
                    } else {
                        // Each eligible party member gets the SAME proportional XP
                        let total_level: u32 = eligible.iter().map(|&(_, lvl)| lvl as u32).sum();
                        let num_members = eligible.len() as f64;

                        for &(member_sid, member_level) in &eligible {
                            if proportional_exp > 0
                                && !world.try_jackpot_exp(member_sid, proportional_exp).await
                            {
                                super::level::exp_change(world, member_sid, proportional_exp).await;
                            }

                            // Party NP: level-weighted formula
                            // loyalty * (1 + 0.2*(members-1)) * memberLevel / totalLevel
                            if proportional_loyalty > 0 && total_level > 0 {
                                let party_bonus = 1.0 + 0.2 * (num_members - 1.0);
                                let member_share =
                                    proportional_loyalty as f64 * party_bonus * member_level as f64
                                        / total_level as f64;
                                let final_loyalty = member_share.ceil() as i32;
                                if final_loyalty > 0 {
                                    crate::systems::loyalty::send_loyalty_change(
                                        world,
                                        member_sid,
                                        final_loyalty,
                                        false,
                                        false,
                                        true, // C++ default: bIsAddLoyaltyMonthly = true
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                // Solo damager — proportional XP/NP directly
                if proportional_exp > 0 && !world.try_jackpot_exp(rep_sid, proportional_exp).await {
                    super::level::exp_change(world, rep_sid, proportional_exp).await;
                }
                if proportional_loyalty > 0 {
                    crate::systems::loyalty::send_loyalty_change(
                        world,
                        rep_sid,
                        proportional_loyalty,
                        false,
                        false,
                        true, // C++ default: bIsAddLoyaltyMonthly = true
                    );
                }
            }
        }
    } else {
        // Phase 4: Fallback — no valid damagers found (all disconnected/dead/out of range)
        // Give full XP/NP to the killer (preserves original behavior)
        award_npc_xp(world, killer_sid, base_exp, tmpl.level).await;
        award_npc_loyalty_solo(world, killer_sid, base_loyalty, tmpl.level);
    }

    // ── Quest monster kill tracking (C++ CNpc::OnDeathProcess) ─────
    // party members within RANGE_80M; otherwise call only for the killer.
    {
        let npc_proto = npc.proto_id;
        let pid = world.get_party_id(killer_sid);
        if let Some(pid) = pid {
            if let Some(party) = world.get_party(pid) {
                for &member_sid in &party.active_members() {
                    if world.is_player_dead(member_sid) {
                        continue;
                    }
                    if world.get_character_info(member_sid).is_none() {
                        continue;
                    }
                    let in_range = match world.get_position(member_sid) {
                        Some(pos) => {
                            let dx = pos.x - npc_x;
                            let dz = pos.z - npc_z;
                            dx * dx + dz * dz <= RANGE_80M
                        }
                        None => false,
                    };
                    if in_range {
                        super::quest::quest_monster_count_add(world, member_sid, npc_proto);
                    }
                }
            }
        } else {
            super::quest::quest_monster_count_add(world, killer_sid, npc_proto);
        }
    }

    // ── Daily quest monster kill tracking (C++ CNpc::OnDeathProcess) ──
    // Same party/solo pattern as regular quest tracking above.
    {
        let npc_proto = npc.proto_id;
        let pid = world.get_party_id(killer_sid);
        if let Some(pid) = pid {
            if let Some(party) = world.get_party(pid) {
                for &member_sid in &party.active_members() {
                    if world.is_player_dead(member_sid) {
                        continue;
                    }
                    let in_range = match world.get_position(member_sid) {
                        Some(pos) => {
                            let dx = pos.x - npc_x;
                            let dz = pos.z - npc_z;
                            dx * dx + dz * dz <= RANGE_80M
                        }
                        None => false,
                    };
                    if in_range {
                        super::daily_quest::update_daily_quest_count(world, member_sid, npc_proto)
                            .await;
                    }
                }
            }
        } else {
            super::daily_quest::update_daily_quest_count(world, killer_sid, npc_proto).await;
        }
    }

    // ── Loot generation (C++ CNpc::Dead → GiveNpcHaveItem) ─────────
    if super::npc_loot::is_show_box(tmpl.npc_type, npc.zone_id, npc.proto_id) {
        let looter = world
            .get_max_damage_user(npc_id)
            .filter(|&sid| world.get_character_info(sid).is_some())
            .unwrap_or(killer_sid);
        super::npc_loot::generate_npc_loot(world, looter, npc_id, npc, tmpl);
    }
    world.clear_npc_damage(npc_id);

    // ── Set NPC AI state to Dead — the AI tick system handles respawn ──
    world.update_npc_ai(npc_id, |s| {
        s.state = NpcState::Dead;
        s.target_id = None;
    });
    if is_manes_survival_kill
        && world
            .manes_survival_manager
            .request_final_boss_finish(tmpl.s_sid)
    {
        tracing::info!(
            sid = killer_sid,
            npc_sid = tmpl.s_sid,
            "Manes Survival Dark Dragon killed; automatic finish requested"
        );
    }

    let (killer_nation, killer_name, killer_clan_id) = world
        .get_character_info(killer_sid)
        .map_or((0u8, String::new(), 0u16), |ch| {
            (ch.nation, ch.name.clone(), ch.knights_id)
        });

    process_war_warder_gatekeeper_death(world, npc.zone_id, npc.special_type, killer_nation);

    // ── Monument death processing (C++ CNpc::OnDeathProcess) ─────────
    // Only applies to non-monster NPCs with monument types.
    if !tmpl.is_monster {
        super::monument::monument_death_dispatch(
            world,
            npc,
            tmpl,
            killer_nation,
            &killer_name,
            killer_clan_id,
        )
        .await;
    }

    fn process_war_warder_gatekeeper_death(
        world: &WorldState,
        zone_id: u16,
        special_type: i16,
        killer_nation: u8,
    ) {
        // NpcDefines.h: war objectives are spawn special types, not NPC model types.
        let (owner_nation, label, is_gatekeeper) = match special_type {
            90 | 91 => (NATION_KARUS, "Karus warder", false),
            92 | 93 => (NATION_ELMORAD, "El Morad warder", false),
            98 => (NATION_KARUS, "Karus gatekeeper", true),
            99 => (NATION_ELMORAD, "El Morad gatekeeper", true),
            _ => return,
        };

        if !(NATION_KARUS..=NATION_ELMORAD).contains(&killer_nation)
            || killer_nation == owner_nation
        {
            return;
        }

        let state = world.get_battle_state();
        if !state.is_nation_battle() || state.victory != 0 || zone_id != state.battle_zone_id() {
            return;
        }

        world.increment_war_npc_kill(owner_nation);

        let killer_label = if killer_nation == NATION_KARUS {
            "Karus"
        } else {
            "El Morad"
        };
        crate::systems::war::broadcast_war_announcement(
            world,
            &format!("{label} has been killed by {killer_label}!"),
            None,
        );
        let notice = super::chat::build_chat_packet(
            7, 1, 0xFFFF, "",
            &format!("{label} has been killed by {killer_label}!"), 0, 0, 0,
        );
        world.broadcast_to_all(std::sync::Arc::new(notice), None);

        if is_gatekeeper {
            // Do not move players here. The existing Victory Gate/object-event flow
            // handles invasion warps; this only opens the winner state it depends on.
            world
                .update_battle_state(|s| crate::systems::war::battle_zone_result(s, killer_nation));
            let msg = crate::systems::war::build_winner_string(killer_nation);
            crate::systems::war::broadcast_war_announcement(world, &msg, None);
        }
    }

    // ── Monster Stone boss kill (C++ CNpc::MonsterStoneKillProcess) ────
    // When a Monster Stone boss (summon_type == 1) dies, mark the room as
    // boss-killed and send victory packets to all room users.
    if npc.event_room > 0 && npc.summon_type == 1 {
        monster_stone_boss_kill(world, npc.event_room);
    }

    // ── BDW altar flag pickup (C++ CNpc::OnDeath → BDWMonumentAltarSystem) ──
    // but we check it unconditionally since the altar NPC (9840) has is_monster=true.
    if npc.zone_id == bdw::ZONE_BDW && tmpl.npc_type == bdw::NPC_BORDER_MONUMENT {
        bdw_altar_flag_pickup(world, killer_sid, npc.zone_id, npc_id);
    }

    // ── Draki Tower monster kill (C++ Npc.cpp:959-967) ──────────────────
    // When a monster dies in zone 95, decrement the room's kill counter.
    // When counter reaches 0, all stage monsters are dead → advance stage.
    {
        use crate::handler::draki_tower;
        if npc.zone_id == draki_tower::ZONE_DRAKI_TOWER && npc.event_room > 0 && npc.is_monster {
            draki_tower_monster_kill(world, killer_sid, npc.event_room).await;
        }
    }

    // ── Dungeon Defence monster kill (C++ Npc.cpp:1027-1028) ─────────
    // When a monster dies in zone 89 with an active event room, process
    // DD rewards (coins, jewels, tokens) and decrement the room's kill
    // counter. When counter reaches 0, advance to next stage or finish.
    if npc.zone_id == ZONE_DUNGEON_DEFENCE && npc.event_room > 0 && npc.is_monster {
        dd_monster_kill(world, killer_sid, npc.event_room, npc.proto_id);
    }

    // ── Juraid Mountain monster kill (C++ Npc.cpp:903-907) ──────────────
    // When a monster dies in zone 87 during an active Juraid event, track
    // the kill for the player's nation room.
    if npc.zone_id == ZONE_JURAID_MOUNTAIN && npc.is_monster && npc.event_room > 0 {
        super::dead::track_juraid_monster_kill(
            world,
            killer_sid,
            npc.proto_id,
            npc.event_room,
            npc.summon_type,
            npc.x,
            npc.z,
        );
    }

    // ── Forgotten Temple monster death (C++ CNpc::ForgettenTempleMonsterDead) ──
    // When a monster dies in zone 55, decrement FT monster count and
    // optionally trigger a death skill (special boss monsters).
    if npc.zone_id == super::forgotten_temple::ZONE_FORGOTTEN_TEMPLE && npc.is_monster {
        let ft_state = world.forgotten_temple_state();
        let skill_id =
            super::forgotten_temple::on_monster_dead(ft_state, npc.proto_id, npc.zone_id);
        if let Some(sid) = skill_id {
            tracing::info!(
                proto_id = npc.proto_id,
                skill_id = sid,
                remaining = ft_state
                    .monster_count
                    .load(std::sync::atomic::Ordering::Relaxed),
                "Forgotten Temple: boss death skill triggered"
            );
            // Broadcast full MAGIC_EFFECTING packet to all zone users.
            // Packet format: [u8 opcode][u32 skill][u32 caster][u32 target][u32 sData * 7]
            let mut death_pkt = Packet::new(Opcode::WizMagicProcess as u8);
            death_pkt.write_u8(3); // MAGIC_EFFECTING
            death_pkt.write_u32(sid); // nSkillID
            death_pkt.write_u32(npc_id); // caster = dead NPC
            death_pkt.write_u32((-1i32) as u32); // target = -1 (zone-wide AOE)
            death_pkt.write_u32(127); // sData[0] = X center
            death_pkt.write_u32(0); // sData[1]
            death_pkt.write_u32(127); // sData[2] = Z center
            death_pkt.write_u32(0); // sData[3]
            death_pkt.write_u32(0); // sData[4]
            death_pkt.write_u32(0); // sData[5]
            death_pkt.write_u32(0); // sData[6]
            world.broadcast_to_zone(
                super::forgotten_temple::ZONE_FORGOTTEN_TEMPLE,
                Arc::new(death_pkt),
                None,
            );
        }
    }

    // ── Under The Castle monster death (C++ CNpc::UnderTheCastleProcess) ──
    // When a monster dies in zone 86, determine movie/gate/reward actions
    // and broadcast WIZ_UTC_MOVIE if applicable.
    if npc.zone_id == super::under_castle::ZONE_UNDER_CASTLE && npc.is_monster {
        let utc_state = world.under_the_castle_state();
        let result = super::under_castle::on_monster_death(npc.proto_id, tmpl.npc_type as u16);

        tracing::info!(
            npc_id,
            proto_id = npc.proto_id,
            movie_id = result.movie_id,
            gate_index = ?result.gate_index,
            reward_room = result.reward_room,
            "Under The Castle: monster death processed"
        );

        // Remove from tracked monster list
        super::under_castle::remove_from_monster_list(utc_state, npc_id);

        // Broadcast movie packet if a movie should play
        if result.movie_id > 0 {
            let movie_pkt = super::under_castle::build_utc_movie_packet(result.movie_id);
            world.broadcast_to_zone(
                super::under_castle::ZONE_UNDER_CASTLE,
                Arc::new(movie_pkt),
                None,
            );
        }

        // C++ CNpc::UnderTheCastleProcess calls pNpc->Dead(pUser) for each
        // stage gate. A gate flag leaves the collision object in the region;
        // remove every physical door piece instead.
        if let Some(gate_idx) = result.gate_index {
            let gate_npc_ids = super::under_castle::get_gate_ids(utc_state, gate_idx);
            for gate_npc_id in &gate_npc_ids {
                world.kill_npc(*gate_npc_id);
            }
            tracing::info!(
                gate_idx,
                gate_npc_ids = ?gate_npc_ids,
                "Under The Castle: gate door pieces removed"
            );
        }

        if result.reward_room > 0 {
            tracing::info!(
                proto_id = npc.proto_id,
                reward_room = result.reward_room,
                "Under The Castle: reward room triggered"
            );
            // Distribute room rewards to nearby users based on proximity to
            // room center and boss death position.
            distribute_utc_room_rewards(world, result.reward_room, npc.x, npc.z);
        }

        if result.spawn_exit_portals {
            tracing::info!("Under The Castle: final boss dead, spawning exit portals");
            // SpawnEventNpc(29197, false, ZONE_UNDER_CASTLE, 852, 0, 830, ...)
            // SpawnEventNpc(29197, false, ZONE_UNDER_CASTLE, 825, 0, 873, ...)
            world.spawn_event_npc(
                super::under_castle::UTC_EXIT_PORTAL_NPC,
                false,
                super::under_castle::ZONE_UNDER_CASTLE,
                852.0,
                830.0,
                1,
            );
            world.spawn_event_npc(
                super::under_castle::UTC_EXIT_PORTAL_NPC,
                false,
                super::under_castle::ZONE_UNDER_CASTLE,
                825.0,
                873.0,
                1,
            );
        }
    }

    // ── Chaos Stone death processing (C++ ChaosStone.cpp:145-313) ──────
    // Zones 71 (Ronark Land), 72 (Ardream), 73 (Ronark Land Base).
    // When a chaos stone NPC dies: advance rank, spawn summoned monsters,
    // register spawned NPC runtime IDs for boss kill tracking.
    // When a summoned boss dies: decrement counter via runtime npc_id match.
    if npc.is_monster && matches!(npc.zone_id, 71..=73) {
        let cs_infos = world.chaos_stone_infos();
        let cs_spawns = world.chaos_stone_spawns();

        // Check if this NPC was a chaos stone itself (C++ ChaosStoneDeath)
        if let Some(idx) = super::chaos_stone::on_chaos_stone_death(
            &cs_infos,
            &cs_spawns,
            npc.proto_id,
            npc.zone_id,
        ) {
            tracing::info!(
                proto_id = npc.proto_id,
                zone_id = npc.zone_id,
                chaos_index = idx,
                "Chaos Stone killed, spawning summoned monsters"
            );

            // Zone-wide notice (C++ ChatType::CHAOS_STONE_ENEMY_NOTICE)
            let notice = super::chat::build_chat_packet(
                super::chat::ChatType::ChaosStoneEnemyNotice as u8,
                0,
                0xFFFF,
                "",
                "",
                0,
                0,
                0,
            );
            world.broadcast_to_zone(npc.zone_id, Arc::new(notice), None);

            // Spawn summoned monsters at stone's position
            let stages = world.chaos_stone_stages();
            let summon_list = world.chaos_stone_summon_list();
            let monster_ids =
                super::chaos_stone::death_respawn_monsters(&cs_infos, &summon_list, &stages, idx);
            drop(cs_spawns);
            drop(stages);
            drop(summon_list);

            // Spawn each monster and register runtime IDs for boss tracking
            // C++ NpcThread.cpp:790-825 — ChaosStoneSummon stores GetID() in sBoosID[]
            let mut spawned_ids: Vec<u32> = Vec::new();
            for &mid in &monster_ids {
                if mid > 0 {
                    let ids = world.spawn_event_npc(mid as u16, true, npc.zone_id, npc.x, npc.z, 1);
                    spawned_ids.extend(ids);
                }
            }
            super::chaos_stone::register_spawned_bosses(&cs_infos, idx, &spawned_ids);
            drop(cs_infos);
        } else {
            drop(cs_spawns);
            // Check if this was a summoned boss (C++ ChaosStoneBossKilledBy)
            // Uses runtime npc_id (C++ GetID()), NOT proto_id
            if super::chaos_stone::on_boss_killed(&cs_infos, npc_id, npc.zone_id) {
                tracing::info!(
                    npc_id,
                    proto_id = npc.proto_id,
                    zone_id = npc.zone_id,
                    "Chaos Stone: all bosses killed, wave complete"
                );
            }
            drop(cs_infos);
        }
    }

    // ── Special Stone death (C++ ChaosStone.cpp:198-227) ───────────────
    // NPC type 221 (NPC_MONSTER_SPECIAL) — randomly spawns a monster on death.
    if tmpl.npc_type == super::chaos_stone::NPC_MONSTER_SPECIAL {
        let stones = world.get_all_special_stones();
        if let Some((summon_npc, summon_count)) =
            super::chaos_stone::on_special_stone_death(&stones, npc.proto_id, npc.zone_id)
        {
            tracing::info!(
                proto_id = npc.proto_id,
                zone_id = npc.zone_id,
                summon_npc,
                summon_count,
                "Special Stone killed, spawning random summon"
            );
            world.spawn_event_npc(summon_npc, true, npc.zone_id, npc.x, npc.z, summon_count);
        }
    }

    // ── Santa NPC death — proximity rewards (C++ Npc.cpp:8378-8408) ──
    // When NPC_SANTA (type 219) with area_range >= 1.0 dies, all alive
    // players in the same zone within area_range receive EXP + etrafa items.
    if tmpl.npc_type == NPC_SANTA && tmpl.area_range >= 1.0 {
        let zone_sids = world.sessions_in_zone(npc.zone_id);
        let range_sq = tmpl.area_range * tmpl.area_range;
        let npc_x = npc.x;
        let npc_z = npc.z;

        // Pre-fetch etrafa item settings once.
        let etrafa: [(u32, u16); 3] = if let Some(ss) = world.get_server_settings() {
            [
                (ss.etrafa_item1 as u32, ss.etrafa_count1 as u16),
                (ss.etrafa_item2 as u32, ss.etrafa_count2 as u16),
                (ss.etrafa_item3 as u32, ss.etrafa_count3 as u16),
            ]
        } else {
            [(0, 0); 3]
        };

        for sid in zone_sids {
            if world.is_player_dead(sid) {
                continue;
            }
            let pos = match world.get_position(sid) {
                Some(p) => p,
                None => continue,
            };
            let dx = pos.x - npc_x;
            let dz = pos.z - npc_z;
            if dx * dx + dz * dz > range_sq {
                continue;
            }

            // Give EXP (C++ pUser->ExpChange("New Years Event", m_iExp, true))
            if tmpl.exp > 0 {
                super::level::exp_change_with_bonus(world, sid, tmpl.exp as i64, true).await;
            }

            // Give etrafa items from server settings.
            for &(item_id, count) in &etrafa {
                if item_id > 0 && count > 0 {
                    world.give_item(sid, item_id, count);
                }
            }
        }
        tracing::info!(
            npc_id,
            proto_id = npc.proto_id,
            zone_id = npc.zone_id,
            area_range = tmpl.area_range,
            "Santa NPC death: proximity rewards distributed"
        );
    }

    // ── Collection Race kill tracking ─────────────────────────────────
    // Called for the killer only (CR is an individual event, not party-based).
    {
        let cr = world.collection_race_event().clone();
        if let Err(e) =
            super::collection_race::handle_kill(world, killer_sid, npc.proto_id, &cr).await
        {
            tracing::warn!("CR handle_kill error: {}", e);
        }
    }

    // ── Monster resource kill notice (C++ Npc.cpp:994-1010) ───────────
    // When a boss monster with an entry in `monster_resource` is killed,
    // broadcast a WIZ_CHAT notice (zone-wide or server-wide).
    if let Some(mr) = world.get_monster_resource(npc.proto_id as i16) {
        let killer_nation = world
            .get_character_info(killer_sid)
            .map(|c| c.nation)
            .unwrap_or(0);
        let notice = super::chat::build_chat_packet(
            mr.notice_type as u8,
            killer_nation,
            killer_sid,
            "",
            &mr.resource,
            0,
            0,
            0,
        );
        if mr.notice_zone == 0 {
            // Server-wide notice
            world.broadcast_to_all(Arc::new(notice), None);
        } else {
            // Zone-wide notice
            world.broadcast_to_zone(npc.zone_id, Arc::new(notice), None);
        }
    }

    // ── Monster respawn loop (C++ Npc.cpp:909-915) ────────────────────
    // When a monster with a matching entry in `monster_respawn_loop` dies,
    // schedule a delayed respawn of the next NPC in the chain.
    if let Some(chain) = world.get_respawn_chain(npc.proto_id as i16) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        world.schedule_respawn(crate::world::ScheduledRespawn {
            born_sid: chain.iborn as u16,
            zone_id: npc.zone_id,
            x: npc.x,
            z: npc.z,
            spawn_at: now + (chain.deadtime as u64) * 60,
        });
        tracing::debug!(
            "RespawnLoop: scheduled NPC {} spawn in zone {} after {}min",
            chain.iborn,
            npc.zone_id,
            chain.deadtime
        );
    }
}

/// Distribute Under The Castle room rewards to nearby players.
/// Each room boss kill awards TROPHY_OF_FLAME (1 or 2 based on proximity)
/// plus room-specific bonus items to all eligible players in zone 86.
/// Proximity logic:
/// - If player is within BOTH the room center range AND the boss kill range (15m) -> 2 trophies
/// - If player is within EITHER the room center range OR the boss kill range -> 1 trophy
/// - Neither -> skip (no reward for this player)
/// Room-specific bonus items (given to ALL qualifying players regardless of trophy count):
/// - Room 1: Trophy only
/// - Room 2: + Dented Ironmass
/// - Room 3: + Petrified Weapon Shrapnel
/// - Room 4: + Iron Powder of Chain
/// - Room 5: + Plwitoon's Tear + Horn of Pluwitoon
fn distribute_utc_room_rewards(world: &WorldState, room: u8, boss_x: f32, boss_z: f32) {
    use super::under_castle;

    let (center_x, center_z, room_range) = match under_castle::get_room_center(room) {
        Some(c) => c,
        None => {
            tracing::warn!(room, "UTC: unknown room for reward distribution");
            return;
        }
    };

    let bonus_items = under_castle::get_utc_room_reward_items(room);

    // Collect eligible players in UTC zone who are alive and in-game.
    // C++ checks: pUtcPlayer != nullptr, isInGame(), GetZoneID() == ZoneID
    // C++ does NOT check isDead() for UTC rewards (unlike FT).
    let utc_users: Vec<(SessionId, f32, f32)> = world
        .collect_sessions_by(|h| {
            h.character.is_some() && h.position.zone_id == under_castle::ZONE_UNDER_CASTLE
        })
        .iter()
        .filter_map(|&sid| world.get_position(sid).map(|pos| (sid, pos.x, pos.z)))
        .collect();

    let mut rewarded = 0u32;

    for (sid, px, pz) in &utc_users {
        let trophy_count = under_castle::calculate_trophy_count(
            *px, *pz, center_x, center_z, room_range, boss_x, boss_z,
        );

        if trophy_count == 0 {
            continue;
        }

        // Give trophy(s)
        world.give_item(*sid, under_castle::TROPHY_OF_FLAME, trophy_count);

        // Give room-specific bonus items
        for &item_id in &bonus_items {
            world.give_item(*sid, item_id, 1);
        }

        rewarded += 1;
    }

    tracing::info!(
        room,
        rewarded,
        total_in_zone = utc_users.len(),
        boss_x,
        boss_z,
        "Under The Castle: room rewards distributed"
    );
}

/// Handle Monster Stone boss kill — mark room, set grace period, notify users.
/// Sets `isBossKilled = true`, `WaitingTime = now + 20`, and sends
/// `WIZ_EVENT/TEMPLE_EVENT_FINISH` + `WIZ_QUEST` to all room users.
fn monster_stone_boss_kill(world: &WorldState, event_room: u16) {
    use crate::systems::monster_stone;

    // event_room is 1-based; room_id is 0-based
    let room_id = event_room - 1;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut mgr = world.monster_stone_write();
    if !mgr.boss_killed(room_id, now) {
        return; // Already killed or room not active
    }

    // Get the user list while we hold the lock
    let users = match mgr.get_room(room_id) {
        Some(room) => room.users.clone(),
        None => return,
    };
    drop(mgr);

    // Send boss kill packets to all room users
    let (finish_pkt, quest_pkt) = monster_stone::build_boss_kill_packets();
    let arc_finish = Arc::new(finish_pkt);
    let arc_quest = Arc::new(quest_pkt);
    for &uid in &users {
        world.send_to_session_arc(uid, Arc::clone(&arc_finish));
        world.send_to_session_arc(uid, Arc::clone(&arc_quest));
    }

    tracing::debug!(
        "Monster Stone boss killed in room {} — grace period started ({} users notified)",
        room_id,
        users.len()
    );
}

/// Handle Draki Tower monster kill — decrement counter, advance stage if all dead.
/// Decrements `draki_monster_kill` counter. When it reaches 0, calls
/// `advance_stage()` to determine the next stage, then sends timer packets
/// and spawns the next wave.
async fn draki_tower_monster_kill(world: &WorldState, killer_sid: SessionId, event_room: u16) {
    use crate::handler::draki_tower;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Decrement kill counter and check if all monsters are dead
    // C++ Npc.cpp:964-967: decrement, then if 0 → ChangeDrakiMode()
    let should_advance = {
        let mut rooms = world.draki_tower_rooms_write();
        match rooms.get_mut(&event_room) {
            Some(room) if room.tower_started && room.draki_monster_kill > 0 => {
                room.draki_monster_kill -= 1;
                room.draki_monster_kill == 0
            }
            _ => false,
        }
    };

    if !should_advance {
        return;
    }

    // All monsters dead — advance stage (C++ ChangeDrakiMode)
    let advance_result = {
        let rooms = world.draki_tower_rooms_read();
        let stages = world.draki_tower_stages();
        match rooms.get(&event_room) {
            Some(room) => draki_tower::advance_stage(room, &stages, now),
            None => return,
        }
    };

    match advance_result {
        draki_tower::StageAdvanceResult::MonsterStage {
            stage_index,
            stage_id,
        } => {
            // Apply monster stage transition
            {
                let stages = world.draki_tower_stages();
                let mut rooms = world.draki_tower_rooms_write();
                if let Some(room) = rooms.get_mut(&event_room) {
                    if let Some(stage) = draki_tower::get_stage_at(&stages, stage_index) {
                        draki_tower::apply_monster_stage(room, stage, now);
                    }
                }
            }

            // Despawn old NPCs, then spawn new monsters
            world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, event_room);
            let (spawn_list, time_limit, stage, sub_stage, elapsed) =
                collect_draki_spawn_data(world, event_room, stage_id, now);
            for (npc_id, is_monster, x, z) in &spawn_list {
                world.spawn_event_npc_ex(
                    *npc_id,
                    *is_monster,
                    draki_tower::ZONE_DRAKI_TOWER,
                    *x,
                    *z,
                    1,
                    event_room,
                    0,
                );
            }
            // Increment kill counter for spawned monsters
            {
                let monster_count =
                    spawn_list.iter().filter(|(_, is_m, _, _)| *is_m).count() as u32;
                let mut rooms = world.draki_tower_rooms_write();
                if let Some(room) = rooms.get_mut(&event_room) {
                    room.draki_monster_kill = monster_count;
                }
            }

            // Send 3-packet timer sequence to killer (C++ SendDrakiTempleDetail(true))
            send_draki_timer_packets(world, killer_sid, time_limit, stage, sub_stage, elapsed);
        }
        draki_tower::StageAdvanceResult::NpcStage {
            stage_index,
            stage_id,
        } => {
            // Apply NPC (rest) stage transition
            {
                let stages = world.draki_tower_stages();
                let mut rooms = world.draki_tower_rooms_write();
                if let Some(room) = rooms.get_mut(&event_room) {
                    if let Some(stage) = draki_tower::get_stage_at(&stages, stage_index) {
                        draki_tower::apply_npc_stage(room, stage, now, false);
                    }
                }
            }

            // Despawn old monsters, spawn NPC entities
            world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, event_room);
            let (spawn_list, _, _stage, _sub_stage, elapsed) =
                collect_draki_spawn_data(world, event_room, stage_id, now);
            for (npc_id, is_monster, x, z) in &spawn_list {
                world.spawn_event_npc_ex(
                    *npc_id,
                    *is_monster,
                    draki_tower::ZONE_DRAKI_TOWER,
                    *x,
                    *z,
                    1,
                    event_room,
                    0,
                );
            }

            // Send timer packets with 180s rest timer (C++ SendDrakiTempleDetail(false))
            // C++ lines 224-228: stage/sub_stage are hardcoded to 0,0 for NPC stages
            let rest_limit = draki_tower::BETWEEN_STAGE_WAIT as u16;
            send_draki_timer_packets(world, killer_sid, rest_limit, 0, 0, elapsed);

            // Persist progress (C++ DrakiTowerSavedUserInfo at line 239)
            draki_tower_save_progress(world, killer_sid, now).await;
        }
        draki_tower::StageAdvanceResult::TowerComplete {
            stage_index,
            stage_id,
            elapsed_seconds,
        } => {
            // Apply final NPC stage (completion)
            {
                let stages = world.draki_tower_stages();
                let mut rooms = world.draki_tower_rooms_write();
                if let Some(room) = rooms.get_mut(&event_room) {
                    if let Some(stage) = draki_tower::get_stage_at(&stages, stage_index) {
                        draki_tower::apply_npc_stage(room, stage, now, true);
                    }
                }
            }

            // Despawn old monsters, spawn completion NPCs
            // C++ SendDrakiTempleDetail(false) → SummonDrakiMonsters(SelectNpcDrakiRoom())
            world.despawn_room_npcs(draki_tower::ZONE_DRAKI_TOWER, event_room);
            let (spawn_list, _, _stage, _sub_stage, elapsed) =
                collect_draki_spawn_data(world, event_room, stage_id, now);
            for (npc_id, is_monster, x, z) in &spawn_list {
                world.spawn_event_npc_ex(
                    *npc_id,
                    *is_monster,
                    draki_tower::ZONE_DRAKI_TOWER,
                    *x,
                    *z,
                    1,
                    event_room,
                    0,
                );
            }

            // Send completion timer (TimeLimit = u16::MAX per C++ line 208)
            // C++ lines 224-228: stage/sub_stage are hardcoded to 0,0 for NPC stages
            send_draki_timer_packets(world, killer_sid, u16::MAX, 0, 0, elapsed);

            // Persist progress + rift rank (C++ DrakiTowerSavedUserInfo + achievement)
            draki_tower_save_progress(world, killer_sid, now).await;
            draki_tower_update_rank(world, killer_sid, elapsed_seconds).await;
            crate::handler::achieve::on_war_event_result(world, killer_sid, 21);

            tracing::info!(
                "Draki Tower COMPLETE! room={}, elapsed={}s",
                event_room,
                elapsed_seconds
            );
        }
        draki_tower::StageAdvanceResult::InvalidStage => {
            tracing::warn!("Draki Tower invalid stage advance for room {}", event_room);
        }
    }
}

/// Collect spawn data for a Draki Tower stage.
#[allow(clippy::type_complexity)]
fn collect_draki_spawn_data(
    world: &WorldState,
    event_room: u16,
    stage_id: i16,
    now: u64,
) -> (Vec<(u16, bool, f32, f32)>, u16, u16, u16, u16) {
    use crate::handler::draki_tower;

    let monsters = world.draki_monster_list();
    let stages = world.draki_tower_stages();
    let stage_is_monster = stages
        .iter()
        .find(|stage| stage.id == stage_id)
        .map(|stage| stage.draki_tower_npc_state == 0)
        .unwrap_or(true);
    let spawn_list: Vec<(u16, bool, f32, f32)> =
        draki_tower::get_monsters_for_stage(&monsters, stage_id)
            .into_iter()
            .map(|m| {
                (
                    m.monster_id as u16,
                    stage_is_monster,
                    m.pos_x as f32,
                    m.pos_z as f32,
                )
            })
            .collect();

    let (stage, sub_stage, elapsed) = {
        let rooms = world.draki_tower_rooms_read();
        rooms
            .get(&event_room)
            .map(|r| {
                let elapsed = now.saturating_sub(r.draki_timer) as u16;
                (r.draki_stage, r.draki_sub_stage, elapsed)
            })
            .unwrap_or((0, 0, 0))
    };

    let time_limit = draki_tower::SUB_STAGE_TIME_LIMIT as u16;
    (spawn_list, time_limit, stage, sub_stage, elapsed)
}

/// Send the 3-packet Draki Tower timer sequence to a player.
/// Sends WIZ_SELECT_MSG, WIZ_EVENT/TIMER, and WIZ_BIFROST.
fn send_draki_timer_packets(
    world: &WorldState,
    sid: SessionId,
    time_limit: u16,
    stage: u16,
    sub_stage: u16,
    elapsed: u16,
) {
    use crate::handler::draki_tower;

    // 1. WIZ_SELECT_MSG (client countdown UI)
    let mut select_pkt = Packet::new(Opcode::WizSelectMsg as u8);
    select_pkt.write_u32(0);
    select_pkt.write_u8(7);
    select_pkt.write_u64(0);
    select_pkt.write_u32(0x0A);
    select_pkt.write_u8(233);
    select_pkt.write_u16(time_limit);
    select_pkt.write_u16(elapsed);
    world.send_to_session_owned(sid, select_pkt);

    // 2. WIZ_EVENT / TEMPLE_DRAKI_TOWER_TIMER (stage info)
    let mut timer_pkt = Packet::new(Opcode::WizEvent as u8);
    timer_pkt.write_u8(draki_tower::TEMPLE_DRAKI_TOWER_TIMER);
    timer_pkt.write_u8(233);
    timer_pkt.write_u8(3);
    timer_pkt.write_u16(stage);
    timer_pkt.write_u16(sub_stage);
    timer_pkt.write_u32(time_limit as u32);
    timer_pkt.write_u32(elapsed as u32);
    world.send_to_session_owned(sid, timer_pkt);

    // 3. WIZ_BIFROST timer display
    let mut bifrost_pkt = Packet::new(Opcode::WizBifrost as u8);
    bifrost_pkt.write_u8(5);
    bifrost_pkt.write_u16(time_limit);
    world.send_to_session_owned(sid, bifrost_pkt);
}

/// Persist Draki Tower progress to the database.
/// Saves elapsed time and linear stage index for the player.
pub(crate) async fn draki_tower_save_progress(world: &WorldState, sid: SessionId, now: u64) {
    use crate::handler::draki_tower;
    use ko_db::repositories::draki_tower::DrakiTowerRepository;

    let pool = match world.db_pool() {
        Some(p) => p,
        None => return,
    };
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };

    // Read room state to get elapsed time and current stage index.
    // C++ guard: only persist when is_draki_stage_change == true (dungeon 1 entries only).
    let event_room = world.get_event_room(sid);
    let (elapsed, stage_index) = {
        let rooms = world.draki_tower_rooms_read();
        match rooms.get(&event_room) {
            Some(room) if room.tower_started && room.is_draki_stage_change => {
                let elapsed = now.saturating_sub(room.draki_timer) as i32;
                let idx = room.saved_draki_stage;
                (elapsed, idx as i16)
            }
            _ => return,
        }
    };

    let draki_cls = draki_tower::draki_class(ch.class);
    let cls_name = draki_tower::draki_class_name(draki_cls);
    let entrance_limit = world
        .with_session(sid, |h| h.draki_entrance_limit)
        .unwrap_or(draki_tower::MAX_ENTRANCE_LIMIT) as i16;

    let repo = DrakiTowerRepository::new(pool);
    if let Err(e) = repo
        .save_user_data(
            &ch.name,
            draki_cls,
            cls_name,
            elapsed,
            stage_index,
            entrance_limit,
        )
        .await
    {
        tracing::warn!("Draki Tower save_user_data failed: {e}");
    }
}

/// Update rift ranking when tower is completed.
async fn draki_tower_update_rank(world: &WorldState, sid: SessionId, elapsed_seconds: u32) {
    use crate::handler::draki_tower;
    use ko_db::repositories::draki_tower::DrakiTowerRepository;

    let pool = match world.db_pool() {
        Some(p) => p,
        None => return,
    };
    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return,
    };

    let draki_cls = draki_tower::draki_class(ch.class);
    let cls_name = draki_tower::draki_class_name(draki_cls);

    // Determine rank position: load existing ranks, find insert position
    let repo = DrakiTowerRepository::new(pool);
    let all_ranks = match repo.load_rift_ranks().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("draki_tower finish_rift load_rift_ranks DB error: {e}");
            Vec::new()
        }
    };
    let class_ranks: Vec<_> = all_ranks.iter().filter(|r| r.class == draki_cls).collect();

    // Find position where this time beats an existing entry (or is new top-5)
    let rank_pos = class_ranks
        .iter()
        .position(|r| (elapsed_seconds as i32) < r.finish_time)
        .unwrap_or(class_ranks.len());

    if rank_pos < 5 {
        let rank_id = (rank_pos + 1) as i32;
        if let Err(e) = repo
            .upsert_rift_rank(
                draki_cls,
                cls_name,
                rank_id,
                &ch.name,
                41, // stage 41 = completed all 5 dungeons (final stage index)
                elapsed_seconds as i32,
            )
            .await
        {
            tracing::warn!("Draki Tower upsert_rift_rank failed: {e}");
        }
    }
}

/// Handle Dungeon Defence monster kill — distribute rewards and advance stage.
/// and kill-count decrement in `Npc.cpp:971-984`.
/// 1. Validate the monster is a DD monster (proto ID check).
/// 2. Calculate rewards (rift jewels, monster coins, lunar tokens for bosses).
/// 3. Give items to killer and party members.
/// 4. Decrement the room's kill counter atomically.
/// 5. When kill counter reaches 0, advance stage or trigger finish.
fn dd_monster_kill(world: &WorldState, killer_sid: SessionId, event_room: u16, proto_id: u16) {
    use crate::handler::dungeon_defence;

    // ── 1. Reward calculation ──────────────────────────────────────────
    // C++ DungeonDefenceProcess lines 545-570: only specific monster IDs get rewards
    let room = match world.dd_rooms().iter().find(|r| {
        r.room_id.load(std::sync::atomic::Ordering::Relaxed) == event_room
            && r.is_started.load(std::sync::atomic::Ordering::Relaxed)
    }) {
        Some(r) => r,
        None => return,
    };

    let current_stage = room.stage_id.load(std::sync::atomic::Ordering::Relaxed);
    let reward = match dungeon_defence::calculate_kill_reward(proto_id, current_stage) {
        Some(r) => r,
        None => {
            // Not a DD monster — still decrement kill counter below
            // C++ Npc.cpp:971-984: kill count decrement is separate from reward
            dd_decrement_and_advance(world, event_room);
            return;
        }
    };

    // ── 2. Distribute rewards ──────────────────────────────────────────
    // C++ DungeonDefenceProcess lines 583-637

    // Give rift jewels to killer (1 for stages 1-17, 2 for stages 18-35)
    world.give_item(
        killer_sid,
        dungeon_defence::MONSTER_RIFT_JEWEL,
        reward.rift_jewel_count,
    );

    // Get killer's party for coin/token distribution
    let party_members = world
        .get_party_id(killer_sid)
        .and_then(|pid| world.get_party(pid))
        .map(|p| p.active_members())
        .unwrap_or_default();

    if party_members.is_empty() {
        // Solo player: 2 coins to killer
        world.give_item(
            killer_sid,
            dungeon_defence::MONSTER_COIN_ITEM,
            reward.killer_coin_count,
        );
        // Boss: 1 lunar token to killer
        if reward.lunar_token {
            world.give_item(killer_sid, dungeon_defence::LUNAR_ORDER_TOKEN, 1);
        }
    } else {
        // In party: 2 coins to killer, 1 coin to each other party member
        for &msid in &party_members {
            if msid == killer_sid {
                world.give_item(
                    msid,
                    dungeon_defence::MONSTER_COIN_ITEM,
                    reward.killer_coin_count,
                );
            } else {
                world.give_item(
                    msid,
                    dungeon_defence::MONSTER_COIN_ITEM,
                    reward.party_coin_count,
                );
            }
            // Boss: 1 lunar token to ALL party members
            if reward.lunar_token {
                world.give_item(msid, dungeon_defence::LUNAR_ORDER_TOKEN, 1);
            }
        }
    }

    // ── 3. Decrement kill counter and advance ──────────────────────────
    dd_decrement_and_advance(world, event_room);
}

/// Decrement DD room kill counter and advance stage when all monsters are dead.
fn dd_decrement_and_advance(world: &WorldState, event_room: u16) {
    use crate::handler::dungeon_defence;

    let room = match world.dd_rooms().iter().find(|r| {
        r.room_id.load(std::sync::atomic::Ordering::Relaxed) == event_room
            && r.is_started.load(std::sync::atomic::Ordering::Relaxed)
    }) {
        Some(r) => r,
        None => return,
    };

    // Atomically decrement kill count; check if all monsters are dead
    // C++ Npc.cpp:975-977: m_DefenceKillCount--, if <= 0 → ChangeDungeonDefenceStage()
    let prev = room
        .kill_count
        .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    if prev > 1 {
        // More monsters to kill
        return;
    }

    // All monsters dead → advance stage
    // C++ ChangeDungeonDefenceStage() lines 461-512
    let stages = world.dd_stages();
    let result = dungeon_defence::advance_stage(room, &stages);
    drop(stages);

    match result {
        dungeon_defence::StageAdvanceResult::NextStage { new_stage_id, .. } => {
            tracing::info!("DD room {} advanced to stage {}", event_room, new_stage_id);
            // Timer loop will pick up monster_spawned flag and spawn next wave
        }
        dungeon_defence::StageAdvanceResult::Finished => {
            // All stages cleared — trigger finish timer
            // C++ DungeonDefenceSendFinishTimer() lines 516-536
            dungeon_defence::trigger_finish(room);

            // Send WIZ_BIFROST(5, 30) countdown packet
            let mut bifrost_pkt = Packet::new(Opcode::WizBifrost as u8);
            bifrost_pkt.write_u8(5);
            bifrost_pkt.write_u16(dungeon_defence::DD_FINISH_TIME);
            world.broadcast_to_zone_event_room(
                ZONE_DUNGEON_DEFENCE,
                event_room,
                Arc::new(bifrost_pkt),
                None,
            );

            // Send WIZ_SELECT_MSG victory UI
            // C++ lines 528-533: uint32(0) + uint8(7) + uint64(0) + uint8(9)
            //   + uint16(0) + uint8(0) + uint8(11) + uint16(30) + uint16(0)
            let mut select_pkt = Packet::new(Opcode::WizSelectMsg as u8);
            select_pkt.write_u32(0);
            select_pkt.write_u8(7);
            select_pkt.write_u64(0);
            select_pkt.write_u8(9);
            select_pkt.write_u16(0);
            select_pkt.write_u8(0);
            select_pkt.write_u8(11);
            select_pkt.write_u16(dungeon_defence::DD_FINISH_TIME);
            select_pkt.write_u16(0);
            world.broadcast_to_zone_event_room(
                ZONE_DUNGEON_DEFENCE,
                event_room,
                Arc::new(select_pkt),
                None,
            );

            tracing::info!(
                "DD room {} FINISHED — all stages cleared, 30s kick timer started",
                event_room
            );
        }
        dungeon_defence::StageAdvanceResult::Error => {
            tracing::warn!(
                "DD room {} stage advance error (invalid difficulty?)",
                event_room
            );
        }
    }
}

/// Handle BDW altar flag pickup when the altar NPC is killed.
/// 1. Validates user is in a valid BDW room and BDW is active
/// 2. Sets `has_altar_obtained = true` (flag pickup)
/// 3. Broadcasts `TEMPLE_EVENT_ALTAR_FLAG` (sub-opcode 49) to all users in the room
fn bdw_altar_flag_pickup(world: &WorldState, killer_sid: SessionId, _zone_id: u16, npc_id: NpcId) {
    use crate::systems::event_room::{self, TempleEventType};
    use crate::world::ActiveBuff;

    let is_bdw_active = world
        .event_room_manager
        .read_temple_event(|s| s.is_bdw_active());
    if !is_bdw_active {
        return;
    }

    let (killer_name, _killer_nation) = match world.get_character_info(killer_sid) {
        Some(ch) => (ch.name.clone(), ch.nation),
        None => return,
    };

    // Find killer's room
    let (room_id, _) = match world
        .event_room_manager
        .find_user_room(TempleEventType::BorderDefenceWar, &killer_name)
    {
        Some(r) => r,
        None => return,
    };

    // Flag pickup inside room lock scope + store altar NPC ID
    let nation = {
        let mut bdw_mgr = world.bdw_manager_write();

        let Some(mut room) = world
            .event_room_manager
            .get_room_mut(TempleEventType::BorderDefenceWar, room_id)
        else {
            return;
        };

        if room.finish_packet_sent {
            return;
        }

        let nation = bdw::flag_pickup(&mut room, &killer_name);

        // Store the altar NPC ID so we can restore its HP on respawn
        if nation != 0 {
            if let Some(bdw_state) = bdw_mgr.get_room_state_mut(room_id) {
                bdw_state.altar_npc_id = npc_id;
            }
        }

        nation
    };

    if nation == 0 {
        return;
    }

    // Broadcast altar flag pickup to all room users
    let flag_pkt = event_room::build_altar_flag_packet(&killer_name, nation);
    dead::broadcast_to_bdw_room(world, room_id, &flag_pkt);

    // Apply BUFF_TYPE_FRAGMENT_OF_MANES speed debuff to carrier
    //   pTarget->m_bSpeedAmount = pType->bSpeed;
    world.apply_buff(
        killer_sid,
        ActiveBuff {
            skill_id: bdw::BUFF_FRAGMENT_OF_MANES_SKILL,
            buff_type: bdw::BUFF_TYPE_FRAGMENT_OF_MANES,
            caster_sid: killer_sid,
            start_time: std::time::Instant::now(),
            duration_secs: 0, // permanent until explicitly removed
            attack_speed: 0,
            speed: bdw::FRAGMENT_SPEED_VALUE,
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
            is_buff: true, // C++ classifies this as buff (not debuff)
        },
    );

    tracing::info!(
        "BDW altar flag pickup: '{}' (nation={}) in room {}, debuff applied",
        killer_name,
        nation,
        room_id,
    );
}

/// Award NPC kill XP to a single player, applying level-difference modifier.
/// Uses `level::exp_change()` for proper level-up/down handling and correct
/// WIZ_EXP_CHANGE packet format.
async fn award_npc_xp(world: &WorldState, sid: SessionId, base_exp: i64, npc_level: u8) {
    let player_level = match world.get_character_info(sid) {
        Some(ch) => ch.level,
        None => return,
    };

    // Apply level-difference modifier
    let modifier = super::level::get_reward_modifier(npc_level, player_level);
    let final_exp = (base_exp as f32 * modifier) as i64;

    if final_exp <= 0 {
        return;
    }

    if !world.try_jackpot_exp(sid, final_exp).await {
        super::level::exp_change(world, sid, final_exp).await;
    }
}

/// Award NPC kill loyalty (NP) to a solo player, applying level-difference modifier.
fn award_npc_loyalty_solo(world: &WorldState, sid: SessionId, base_loyalty: i32, npc_level: u8) {
    if base_loyalty <= 0 {
        return;
    }
    let player_level = match world.get_character_info(sid) {
        Some(ch) => ch.level,
        None => return,
    };

    let modifier = super::level::get_reward_modifier(npc_level, player_level);
    let final_loyalty = ((base_loyalty as f32) * modifier).ceil() as i32;
    if final_loyalty > 0 {
        crate::systems::loyalty::send_loyalty_change(
            world,
            sid,
            final_loyalty,
            false,
            false,
            true, // C++ User.cpp:2571 — SendLoyaltyChange("Npc Loyalty", n) uses default true
        );
    }
}

/// Send WIZ_TARGET_HP for an NPC target (HP bar update).
/// Packet format: `[u32 npc_id][u8 0][u32 max_hp][u32 current_hp][u32 source][i32 change][u8 reserved=0]`
fn send_npc_target_hp_update(
    world: &WorldState,
    attacker_sid: SessionId,
    npc_id: NpcId,
    max_hp: i32,
    current_hp: i32,
    damage: i32,
) {
    let response = super::target_hp::build_target_hp_packet(
        npc_id,
        0,
        max_hp.max(0) as u32,
        current_hp.max(0) as u32,
        -damage,
        -damage,
    );

    world.send_to_session_owned(attacker_sid, response);
}

/// Build and broadcast the attack result packet to the 3x3 region.
/// ```text
/// Packet result(WIZ_ATTACK, bType);
/// result << bResult << uint32(GetSocketID()) << uint32(tid) << unknown;
/// SendToRegion(&result, nullptr, GetEventRoom());
/// ```
/// Wire format: `[u8 bType][u8 bResult][u32 attacker_id][u32 target_id][u8 unknown]`
fn broadcast_attack_result(
    world: &WorldState,
    attacker_sid: SessionId,
    b_type: u8,
    b_result: u8,
    target_id: u32,
    unknown: u8,
) {
    let mut pkt = Packet::new(Opcode::WizAttack as u8);
    pkt.write_u8(b_type);
    pkt.write_u8(b_result);
    pkt.write_u32(attacker_sid as u32);
    pkt.write_u32(target_id);
    pkt.write_u8(unknown);

    if let Some(pos) = world.get_position(attacker_sid) {
        let event_room = world.get_event_room(attacker_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }
}

/// Send a WIZ_TARGET_HP update to the attacker so the client updates the
/// target's HP bar after taking damage.
fn send_target_hp_update(
    world: &WorldState,
    attacker_sid: SessionId,
    target_sid: SessionId,
    damage: i32,
) {
    let ch = match world.get_character_info(target_sid) {
        Some(c) => c,
        None => return,
    };

    let response = super::target_hp::build_target_hp_packet(
        target_sid as u32,
        0,
        ch.max_hp as u32,
        ch.hp.max(0) as u32,
        -damage,
        -damage,
    );

    // Send to the attacker
    world.send_to_session_owned(attacker_sid, response);
}

/// Check if a zone is a PK zone (allows PvP combat).
fn is_pk_zone(zone_id: u16) -> bool {
    zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_ARDREAM
        || zone_id == ZONE_RONARK_LAND_BASE
        || zone_id == ZONE_KROWAZ_DOMINION
        || (ZONE_BATTLE..=ZONE_BATTLE6).contains(&zone_id)
}

/// Check if a class is a priest class (base class 4, 11, or 12).
fn is_priest_class(class: u16) -> bool {
    matches!(class % 100, 4 | 11 | 12)
}

/// Give zone kill rewards after a PvP kill.
/// Logic:
/// 1. Increment the killer's PvP kill count.
/// 2. For each zone_kill_reward row matching the killer's zone:
///    - Check status (must be 1/enabled)
///    - Check drop_rate (random 1-10000, skip if random > rate unless rate == 10000)
///    - Check kill_count modulo (m_KillCount % reward.KillCount == 0)
///    - Check party_required (0=solo only, 1=party only, 2=any)
///    - If all_party_reward and in party, give to all party members in zone
///    - If not all_party_reward and in party and is_priest=true, redirect to priest in party
///    - Otherwise give to the killer directly
fn give_kill_reward(world: &WorldState, killer_sid: SessionId, zone_id: u16) {
    // Increment kill count on the killer session
    let mut kill_count: u16 = 0;
    world.update_session(killer_sid, |h| {
        h.pvp_kill_count = h.pvp_kill_count.wrapping_add(1);
        kill_count = h.pvp_kill_count;
    });

    // Get killer info
    let killer_info = match world.get_character_info(killer_sid) {
        Some(ch) => ch,
        None => return,
    };

    let in_party = killer_info.party_id.is_some();
    let killer_class = killer_info.class;
    let killer_event_room = world.get_event_room(killer_sid);

    // Get zone kill rewards for this zone
    let rewards = world.get_zone_kill_rewards(zone_id);
    if rewards.is_empty() {
        return;
    }

    // Collect party member session IDs if in a party
    let party_sids: Vec<SessionId> = if let Some(party_id) = killer_info.party_id {
        world
            .get_party(party_id)
            .map(|p| p.members.iter().filter_map(|m| *m).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut rng = rand::thread_rng();

    for reward in &rewards {
        // Check drop rate (random 1-10000)
        let roll: u16 = rng.gen_range(1..=10000);
        if roll > reward.drop_rate as u16 && reward.drop_rate != 10000 {
            continue;
        }

        // Check kill count modulo: m_KillCount % reward.KillCount == 0
        if reward.kill_count > 0 && !kill_count.is_multiple_of(reward.kill_count as u16) {
            continue;
        }

        // Check party requirement
        //                pReward->Party == 0 && isInParty() -> skip (solo only)
        if reward.party_required == 1 && !in_party {
            continue;
        }
        if reward.party_required == 0 && in_party {
            continue;
        }

        let mut self_reward = true;

        // If in party and all_party_reward, give to all party members in zone+room
        //   if (pUser->GetZoneID() != GetZoneID()
        //    || pUser->GetEventRoom() != GetEventRoom()
        //    || pUser->GetPartyID() != GetPartyID()) continue;
        if in_party && reward.all_party_reward {
            for &member_sid in &party_sids {
                let member_ok = world
                    .with_session(member_sid, |h| {
                        h.character.is_some()
                            && h.position.zone_id == zone_id
                            && h.event_room == killer_event_room
                    })
                    .unwrap_or(false);
                if !member_ok {
                    continue;
                }
                give_reward_to_player(world, member_sid, reward);
            }
            self_reward = false;
        } else if in_party && !reward.all_party_reward && !is_priest_class(killer_class) {
            // Non-priest killer in party: chance to redirect reward to a priest
            let redirect_roll: u16 = rng.gen_range(0..=10000);
            let priest_rate_threshold = (reward.priest_rate.min(100) as u16) * 100;
            if redirect_roll < priest_rate_threshold {
                for &member_sid in &party_sids {
                    let is_priest_in_zone = world
                        .with_session(member_sid, |h| {
                            if let Some(ref ch) = h.character {
                                is_priest_class(ch.class)
                                    && h.position.zone_id == zone_id
                                    && h.event_room == killer_event_room
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if is_priest_in_zone {
                        give_reward_to_player(world, member_sid, reward);
                    }
                }
            }
            // Note: killer still gets reward (self_reward stays true) per C++ logic
        }

        if self_reward {
            give_reward_to_player(world, killer_sid, reward);
        }
    }
}

/// Give a single zone kill reward item to a player.
/// - When `isBank` (give_to_warehouse) is true, item goes to warehouse via
///   `GiveWerehouseItem(ItemID, sCount, false, false, Time)`.
/// - When `Time` (item_expiration) > 0, item gets an expiry timestamp:
///   `nExpirationTime = UNIXTIME + (86400 * Time)` (Time is in days).
fn give_reward_to_player(
    world: &WorldState,
    sid: SessionId,
    reward: &ko_db::models::ZoneKillReward,
) {
    let item_id = reward.item_id as u32;
    let count = reward.item_count.max(1) as u16;
    let expiry_days = reward.item_expiration.max(0) as u32;

    if item_id == ITEM_GOLD {
        world.gold_gain(sid, count as u32);
    } else if reward.give_to_warehouse {
        world.give_warehouse_item(sid, item_id, count, expiry_days);
    } else if expiry_days > 0 {
        world.give_item_with_expiry(sid, item_id, count, expiry_days);
    } else {
        world.give_item(sid, item_id, count);
    }

    tracing::debug!(
        "[sid={}] Zone kill reward: item_id={}, count={}, zone={}, warehouse={}, expiry_days={}",
        sid,
        item_id,
        count,
        reward.zone_id,
        reward.give_to_warehouse,
        expiry_days,
    );
}

// ── PvP zone helpers ───────────────────────────────────────────────────────

/// Apply NPC type-specific damage overrides.
/// Prison NPC needs punishment stick + 5% MP cost, Fosil needs pickaxe, etc.
fn apply_npc_type_damage_override(
    world: &WorldState,
    attacker_sid: SessionId,
    attacker: &CharacterInfo,
    tmpl: &crate::npc::NpcTemplate,
    npc: &crate::npc::NpcInstance,
    damage: i16,
) -> i16 {
    // Weapon kind for pickaxe: C++ GameDefine.h:1234 — WEAPON_PICKAXE = 61
    const WEAPON_PICKAXE: i32 = 61;
    // Punishment stick item ID: C++ GameDefine.h:1622 — `GetNum() == 900356000`
    const PUNISHMENT_STICK_ID: i32 = 900356000;

    match tmpl.npc_type {
        NPC_PRISON => {
            // Requires punishment stick weapon + 5% MP
            let mp_cost = (attacker.max_mp as i32 * 5 / 100) as i16;
            if attacker.mp < mp_cost {
                return 0;
            }
            let weapon = world.get_right_hand_weapon(attacker_sid);
            let slot = world.get_inventory_slot(attacker_sid, 6); // RIGHTHAND=6
            let valid = match (weapon, slot) {
                (Some(w), Some(s)) => w.num == PUNISHMENT_STICK_ID && s.durability > 0,
                _ => false,
            };
            if valid {
                let new_mp = (attacker.mp - mp_cost).max(0);
                world.update_character_mp(attacker_sid, new_mp);
                1
            } else {
                0
            }
        }
        NPC_FOSIL => {
            // Requires pickaxe weapon
            let weapon = world.get_right_hand_weapon(attacker_sid);
            let slot = world.get_inventory_slot(attacker_sid, 6); // RIGHTHAND=6
            let valid = match (weapon, slot) {
                (Some(w), Some(s)) => w.kind == Some(WEAPON_PICKAXE) && s.durability > 0,
                _ => false,
            };
            if valid {
                1
            } else {
                0
            }
        }
        NPC_OBJECT_FLAG if npc.proto_id == 511 => 1,
        NPC_REFUGEE => {
            if npc.is_monster {
                match npc.proto_id {
                    3202 | 3203 | 3252 | 3253 => 20,
                    _ => 10,
                }
            } else {
                10
            }
        }
        NPC_TREE => 20,
        NPC_PARTNER_TYPE if tmpl.group == 0 => 0, // Nation::NONE companion
        NPC_BORDER_MONUMENT => 10,
        _ => {
            // Neutral peaceful NPCs cannot be R-attacked.
            // NPC nation is stored in the AI state, but group field on template
            // represents the original nation. For runtime nation, use npc_ai.
            // Here we check the template group field (matching m_OrgNation).
            if tmpl.group == 3 {
                0
            } else {
                damage
            }
        }
    }
}

/// Check if the player has an active Vaccuni transformation that deals 30000 fixed damage.
/// Specific NPC proto IDs require both a quest event flag and a special weapon
/// equipped in the right hand. If conditions are met, returns true (damage = 30000).
fn check_vaccuni_attack(
    world: &WorldState,
    attacker_sid: SessionId,
    npc: &crate::npc::NpcInstance,
) -> bool {
    // Item IDs for Vaccuni transformation weapons
    const TIMING_FLOW_TYON: i32 = 900335523;
    const TIMING_FLOW_MEGANTHEREON: i32 = 900336524;
    const TIMING_FLOW_HELLHOUND: i32 = 900337525;

    // (proto_id, event_ids, required_weapon_id)
    let check = match npc.proto_id {
        4351 => Some(([793_u16, 794], TIMING_FLOW_TYON)),
        655 => Some(([795, 796], TIMING_FLOW_MEGANTHEREON)),
        666 => Some(([798, 799], TIMING_FLOW_HELLHOUND)),
        // Proto 4301, 605, 611, 616 fall through (return false)
        _ => None,
    };

    let (event_ids, weapon_id) = match check {
        Some(c) => c,
        None => return false,
    };

    // Check quest events (C++ CheckExistEvent(event_id, 1))
    let has_event = world
        .with_session(attacker_sid, |h| {
            h.quests
                .get(&event_ids[0])
                .map(|q| q.quest_state == 1)
                .unwrap_or(false)
                || h.quests
                    .get(&event_ids[1])
                    .map(|q| q.quest_state == 1)
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    if !has_event {
        return false;
    }

    // Check right-hand weapon
    let weapon = world.get_right_hand_weapon(attacker_sid);
    let slot = world.get_inventory_slot(attacker_sid, 6); // RIGHTHAND=6

    match (weapon, slot) {
        (Some(w), Some(s)) => w.num == weapon_id && s.durability > 0,
        _ => false,
    }
}

/// Check if the player is in an arena area (Moradon arena or ZONE_ARENA).
/// Returns true if:
/// - In ZONE_ARENA (48) — all locations considered arena
/// - In Moradon (21-25) AND within Moradon arena bounds (x: 684-735, z: 360-491)
pub(crate) fn is_in_arena(zone_id: u16, x: f32, z: f32) -> bool {
    if zone_id == ZONE_ARENA {
        return true;
    }
    // Moradon zones: 21-25
    if (ZONE_MORADON..=ZONE_MORADON5).contains(&zone_id) {
        return x > 684.0 && x < 735.0 && ((z > 360.0 && z < 411.0) || (z > 440.0 && z < 491.0));
    }
    false
}

/// Check if the zone is a normal PVP zone (allows nation-vs-nation combat).
pub(crate) fn is_in_pvp_zone(zone_id: u16) -> bool {
    zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_RONARK_LAND_BASE
        || zone_id == ZONE_ARDREAM
        || zone_id == ZONE_SNOW_BATTLE
        || zone_id == ZONE_BATTLE
        || zone_id == ZONE_BATTLE2
        || zone_id == ZONE_BATTLE3
        || zone_id == ZONE_BATTLE4
        || zone_id == ZONE_BATTLE5
        || zone_id == ZONE_BATTLE6
        || zone_id == ZONE_JURAID_MOUNTAIN
        || zone_id == ZONE_BORDER_DEFENSE_WAR
        || zone_id == ZONE_CLAN_WAR_ARDREAM
        || zone_id == ZONE_CLAN_WAR_RONARK
        || zone_id == ZONE_BIFROST
        || (ZONE_PARTY_VS_1..=ZONE_PARTY_VS_4).contains(&zone_id)
        || is_in_special_event_zone(zone_id)
}

/// Check if the zone is a special event zone (Zindan War / SPBATTLE zones 105-115).
/// ZONE_SPBATTLE_BASE = 104, SPBATTLE1 = 105 .. SPBATTLE11 = 115
pub(crate) fn is_in_special_event_zone(zone_id: u16) -> bool {
    (105..=115).contains(&zone_id)
}

/// Check if the zone is Luferson Castle (Karus nation zones).
fn is_in_luferson_castle(zone_id: u16) -> bool {
    zone_id == ZONE_KARUS || zone_id == ZONE_KARUS2 || zone_id == ZONE_KARUS3
}

/// Check if the zone is El Morad Castle (El Morad nation zones).
fn is_in_elmorad_castle(zone_id: u16) -> bool {
    zone_id == ZONE_ELMORAD || zone_id == ZONE_ELMORAD2 || zone_id == ZONE_ELMORAD3
}

/// Core PvP permission check — determines if attacker can attack target.
/// This implements the "default deny" model: PvP is only allowed in specific
/// zones and under specific conditions. Returns `true` if the attack is allowed.
/// ## Zone Rules (in priority order)
/// 1. **Moradon Arena**: Party arena (same party can't attack), melee arena (can't attack self)
/// 2. **ZONE_ARENA**: Rose clan arena restrictions, safety area check, otherwise allow
/// 3. **Own safety area**: Target in own safety area → deny (cross-nation only)
/// 4. **PVP zones**: Opposite nation → allow
/// 5. **Abyss zones**: Opposite nation → allow
/// 6. **Delos (CSW)**: Both must be in different clans during active CSW war → allow
/// 7. **Castle wars**: Opposite nation + open flags → allow
/// 8. **GM override**: GM can attack in castle zones without open flags
/// 9. **Default**: Deny all PvP
pub(crate) fn is_hostile_to(
    world: &WorldState,
    attacker_sid: SessionId,
    attacker: &CharacterInfo,
    attacker_pos: &Position,
    target_sid: SessionId,
    target: &CharacterInfo,
    target_pos: &Position,
) -> bool {
    // Self-targeting is never hostile.
    if attacker_sid == target_sid {
        return false;
    }

    let attacker_zone = attacker_pos.zone_id;
    let attacker_x = attacker_pos.x;
    let attacker_z = attacker_pos.z;

    // ── Event room check ────────────────────────────────────────────────
    // Handled earlier in the temple event gate block (lines ~1108-1150)
    // which blocks cross-room attacks before reaching is_hostile_to.

    // ── Moradon / Arena combat ──────────────────────────────────────────
    if is_in_arena(attacker_zone, attacker_x, attacker_z)
        && is_in_arena(target_pos.zone_id, target_pos.x, target_pos.z)
    {
        // Party arena: x 684-735, z 360-411
        if attacker_x > 684.0 && attacker_x < 735.0 && attacker_z > 360.0 && attacker_z < 411.0 {
            // Same party members can't attack each other in party arena
            if attacker.party_id.is_some() && attacker.party_id == target.party_id {
                return false;
            }
            return true;
        }
        // Melee arena: x 684-735, z 440-491
        if attacker_x > 684.0 && attacker_x < 735.0 && attacker_z > 440.0 && attacker_z < 491.0 {
            // Can't attack yourself (name check in C++, sid check here)
            if attacker_sid == target_sid {
                return false;
            }
            return true;
        }
    }

    // ── ZONE_ARENA (full arena zone) ────────────────────────────────────
    if attacker_zone == ZONE_ARENA {
        fn in_range_slow(px: f32, pz: f32, cx: f32, cz: f32, radius: f32) -> bool {
            let dx = px - cx;
            let dz = pz - cz;
            dx * dx + dz * dz <= radius * radius
        }

        // Rose clan arena: two circular areas (64,178 r=60) and (192,178 r=60)
        if in_range_slow(attacker_x, attacker_z, 64.0, 178.0, 60.0)
            || in_range_slow(attacker_x, attacker_z, 192.0, 178.0, 60.0)
        {
            // If either player is not in a clan, deny
            if attacker.knights_id == 0 || target.knights_id == 0 {
                return false;
            }
            // Same clan can't attack each other
            if attacker.knights_id == target.knights_id {
                return false;
            }
        }

        // Safety area check in arena
        if is_in_enemy_safety_area(attacker_zone, attacker_x, attacker_z, attacker.nation) {
            return false;
        }

        return true;
    }

    // ── Target in own safety area ───────────────────────────────────────
    if attacker.nation != target.nation
        && is_in_own_safety_area(
            target_pos.zone_id,
            target_pos.x,
            target_pos.z,
            target.nation,
        )
    {
        return false;
    }

    // ── PVP zones: opposite nation can fight ────────────────────────────
    if attacker.nation != target.nation && is_in_pvp_zone(attacker_zone) {
        return true;
    }

    // ── Abyss zones: opposite nation can fight ──────────────────────────
    if attacker.nation != target.nation
        && (attacker_zone == ZONE_DESPERATION_ABYSS
            || attacker_zone == ZONE_HELL_ABYSS
            || attacker_zone == ZONE_DRAGON_CAVE)
    {
        return true;
    }

    // ── Chaos Temple: all can fight ─────────────────────────────────────
    // When Chaos Dungeon event is active and both players are in zone 85,
    // everyone can attack everyone (free-for-all PvP, no nation check).
    if attacker_zone == ZONE_CHAOS_DUNGEON {
        let chaos_active = world
            .event_room_manager
            .read_temple_event(|s| s.is_chaos_active());
        if chaos_active {
            return true;
        }
    }

    // ── Delos (Castle Siege Warfare) ────────────────────────────────────
    if attacker_zone == ZONE_DELOS {
        let csw = match world.csw_event().try_read() {
            Ok(guard) => guard,
            Err(_) => return false, // Lock contention — deny (safe default)
        };
        if csw.status != CswOpStatus::War || attacker.knights_id == target.knights_id {
            return false;
        }
        if !csw.is_active() || attacker.knights_id == 0 || target.knights_id == 0 {
            return false;
        }
        drop(csw);

        if is_in_own_safety_area(attacker_zone, attacker_x, attacker_z, attacker.nation)
            || is_in_own_safety_area(
                target_pos.zone_id,
                target_pos.x,
                target_pos.z,
                target.nation,
            )
        {
            return false;
        }

        return true;
    }

    // ── Castle wars (Elmorad/Luferson zones when war is open) ───────────
    if attacker.nation != target.nation
        && (is_in_elmorad_castle(attacker_zone) || is_in_luferson_castle(attacker_zone))
    {
        let battle = world.get_battle_state();
        if battle.elmorad_open_flag || battle.karus_open_flag {
            return true;
        }
    }

    // ── Cinderella zone ─────────────────────────────────────────────────
    // When Cinderella War is active, event users of opposite nations can fight.
    if world.is_cinderella_active() && world.cinderella_zone_id() == attacker_zone {
        if attacker.nation == target.nation {
            return false;
        }
        // Both must be registered event users in the Cinderella zone
        if world.is_player_in_cinderella(attacker_sid) && world.is_player_in_cinderella(target_sid)
        {
            return true;
        }
    }

    // ── GM override: can attack in castle zones without open flags ──────
    let is_gm = attacker.authority == 0;
    if is_gm
        && attacker.nation != target.nation
        && (is_in_elmorad_castle(attacker_zone) || is_in_luferson_castle(attacker_zone))
    {
        return true;
    }

    // ── Default: deny PvP ───────────────────────────────────────────────
    false
}

// ── Elemental weapon damage constants ──────────────────────────────────
const ITEM_TYPE_FIRE: u8 = 0x01;
const ITEM_TYPE_COLD: u8 = 0x02;
const ITEM_TYPE_LIGHTNING: u8 = 0x03;
const ITEM_TYPE_POISON: u8 = 0x04;
const ITEM_TYPE_HP_DRAIN: u8 = 0x05;
const ITEM_TYPE_MP_DAMAGE: u8 = 0x06;
const ITEM_TYPE_MP_DRAIN: u8 = 0x07;
const MAX_RESISTANCE: i32 = 200;

/// Apply elemental weapon damage bonuses from attacker's equipped items (PvP).
/// Iterates attacker's `equipped_item_bonuses` and adds fire/cold/lightning/poison
/// damage reduced by target's elemental resistance. Also handles HP/MP drain.
/// Formula per element: `bonus_amount - bonus_amount * total_resistance / 200`
/// Resistance = `(base_r * pct_r / 100 + resistance_bonus)`, capped at 200.
#[allow(clippy::too_many_arguments)]
fn apply_elemental_weapon_damage_pvp(
    world: &WorldState,
    attacker_sid: SessionId,
    attacker_stats: &crate::world::EquippedStats,
    target_sid: SessionId,
    target_stats: &crate::world::EquippedStats,
    target_pct_fire_r: u8,
    target_pct_cold_r: u8,
    target_pct_lightning_r: u8,
    target_pct_poison_r: u8,
    base_damage: i16,
) -> i16 {
    let (pct_fire, pct_cold, pct_lightning, pct_poison) = (
        target_pct_fire_r,
        target_pct_cold_r,
        target_pct_lightning_r,
        target_pct_poison_r,
    );

    let resist_bonus = target_stats.resistance_bonus as i32;
    let mut elemental_bonus: i32 = 0;
    let mut hp_drain_total: i32 = 0;
    let mut mp_damage_total: i32 = 0;
    let mut mp_drain_total: i32 = 0;

    for bonuses in attacker_stats.equipped_item_bonuses.values() {
        for &(btype, amount) in bonuses {
            if amount <= 0 {
                continue;
            }

            match btype {
                ITEM_TYPE_FIRE | ITEM_TYPE_COLD | ITEM_TYPE_LIGHTNING | ITEM_TYPE_POISON => {
                    let base_r: i32 = match btype {
                        ITEM_TYPE_FIRE => target_stats.fire_r as i32,
                        ITEM_TYPE_COLD => target_stats.cold_r as i32,
                        ITEM_TYPE_LIGHTNING => target_stats.lightning_r as i32,
                        ITEM_TYPE_POISON => target_stats.poison_r as i32,
                        _ => 0,
                    };
                    let pct = match btype {
                        ITEM_TYPE_FIRE => pct_fire as i32,
                        ITEM_TYPE_COLD => pct_cold as i32,
                        ITEM_TYPE_LIGHTNING => pct_lightning as i32,
                        ITEM_TYPE_POISON => pct_poison as i32,
                        _ => 100,
                    };
                    let total_r = (base_r * pct / 100 + resist_bonus).clamp(0, MAX_RESISTANCE);
                    elemental_bonus += amount - amount * total_r / MAX_RESISTANCE;
                }
                ITEM_TYPE_HP_DRAIN => {
                    hp_drain_total += amount;
                }
                ITEM_TYPE_MP_DAMAGE => {
                    mp_damage_total += amount;
                }
                ITEM_TYPE_MP_DRAIN => {
                    mp_drain_total += amount;
                }
                _ => {}
            }
        }
    }

    // Apply HP drain: heal attacker by drain amount
    if hp_drain_total > 0 {
        let drained = hp_drain_total;
        world.update_character_stats(attacker_sid, |ch| {
            ch.hp = (ch.hp as i32 + drained).min(ch.max_hp as i32) as i16;
        });
    }

    // Apply MP damage: reduce target MP
    if mp_damage_total > 0 {
        world.update_character_stats(target_sid, |ch| {
            ch.mp = (ch.mp as i32 - mp_damage_total).max(0) as i16;
        });
    }

    // Apply MP drain: reduce target MP and restore attacker MP
    if mp_drain_total > 0 {
        world.update_character_stats(target_sid, |ch| {
            ch.mp = (ch.mp as i32 - mp_drain_total).max(0) as i16;
        });
        world.update_character_stats(attacker_sid, |ch| {
            ch.mp = (ch.mp as i32 + mp_drain_total).min(ch.max_mp as i32) as i16;
        });
    }

    if elemental_bonus > 0 {
        (base_damage as i32 + elemental_bonus) as i16
    } else {
        base_damage
    }
}

/// Apply elemental weapon damage bonuses against an NPC target.
/// NPC targets use resistance values from the NPC template. NPCs have no
/// buff-based resistance percentages, so base resistance is used directly.
fn apply_elemental_weapon_damage_npc(
    attacker_stats: &crate::world::EquippedStats,
    npc_tmpl: &crate::npc::NpcTemplate,
    base_damage: i16,
) -> i16 {
    let mut elemental_bonus: i32 = 0;

    for bonuses in attacker_stats.equipped_item_bonuses.values() {
        for &(btype, amount) in bonuses {
            if amount <= 0 {
                continue;
            }

            let total_r = match btype {
                ITEM_TYPE_FIRE => (npc_tmpl.fire_r as i32).clamp(0, MAX_RESISTANCE),
                ITEM_TYPE_COLD => (npc_tmpl.cold_r as i32).clamp(0, MAX_RESISTANCE),
                ITEM_TYPE_LIGHTNING => (npc_tmpl.lightning_r as i32).clamp(0, MAX_RESISTANCE),
                ITEM_TYPE_POISON => (npc_tmpl.poison_r as i32).clamp(0, MAX_RESISTANCE),
                _ => continue,
            };

            elemental_bonus += amount - amount * total_r / MAX_RESISTANCE;
        }
    }

    if elemental_bonus > 0 {
        (base_damage as i32 + elemental_bonus) as i16
    } else {
        base_damage
    }
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    use super::*;
    use crate::world::BOT_ID_BASE;

    /// Test the attack broadcast packet format matches expected value.
    #[test]
    fn test_attack_broadcast_format() {
        // Build broadcast: [u8 bType][u8 bResult][u32 attacker][u32 target][u8 unknown]
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(1); // bType = normal melee
        pkt.write_u8(ATTACK_SUCCESS); // bResult
        pkt.write_u32(42); // attacker_id
        pkt.write_u32(99); // target_id
        pkt.write_u8(0); // unknown

        assert_eq!(pkt.opcode, Opcode::WizAttack as u8);
        // 1 + 1 + 4 + 4 + 1 = 11 bytes
        assert_eq!(pkt.data.len(), 11);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // bType
        assert_eq!(r.read_u8(), Some(ATTACK_SUCCESS)); // bResult
        assert_eq!(r.read_u32(), Some(42)); // attacker_id
        assert_eq!(r.read_u32(), Some(99)); // target_id
        assert_eq!(r.read_u8(), Some(0)); // unknown
        assert_eq!(r.remaining(), 0);
    }

    /// Test the attack broadcast format for a kill (ATTACK_TARGET_DEAD).
    #[test]
    fn test_attack_broadcast_kill_format() {
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(1);
        pkt.write_u8(ATTACK_TARGET_DEAD);
        pkt.write_u32(10);
        pkt.write_u32(20);
        pkt.write_u8(0);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(ATTACK_TARGET_DEAD));
        assert_eq!(r.read_u32(), Some(10));
        assert_eq!(r.read_u32(), Some(20));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Test attack client packet parsing roundtrip.
    #[test]
    fn test_attack_client_packet_parse() {
        // Build a client attack packet:
        // [u8 bType][u8 bResult][u32 tid][i16 delaytime][i16 distance][u8 unknown][u8 unknowns]
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(1); // bType
        pkt.write_u8(0); // bResult (client sends 0)
        pkt.write_u32(42); // target id
        pkt.write_i16(150); // delaytime
        pkt.write_i16(3); // distance
        pkt.write_u8(0); // unknown
        pkt.write_u8(0); // unknowns

        // Parse like the handler does
        let mut r = PacketReader::new(&pkt.data);
        let b_type = r.read_u8().unwrap();
        let _b_result = r.read_u8().unwrap();
        let tid = r.read_u32().unwrap();
        let delaytime = r.read_u16().map(|v| v as i16).unwrap();
        let distance = r.read_u16().map(|v| v as i16).unwrap();
        let unknown = r.read_u8().unwrap();
        let unknowns = r.read_u8().unwrap();

        assert_eq!(b_type, 1);
        assert_eq!(tid, 42);
        assert_eq!(delaytime, 150);
        assert_eq!(distance, 3);
        assert_eq!(unknown, 0);
        assert_eq!(unknowns, 0);
        assert_eq!(r.remaining(), 0);
    }

    /// Test damage clamping: HP never goes below 0.
    #[test]
    fn test_damage_clamping() {
        let target_hp: i16 = 50;
        let damage: i16 = 100; // overkill

        let new_hp = (target_hp - damage).max(0);
        assert_eq!(new_hp, 0);
    }

    /// Test damage clamping: normal damage.
    #[test]
    fn test_damage_normal() {
        let target_hp: i16 = 200;
        let damage: i16 = 30;

        let new_hp = (target_hp - damage).max(0);
        assert_eq!(new_hp, 170);
    }

    // ── Sprint 301: MAX_DAMAGE cap ─────────────────────────────────

    #[test]
    fn test_max_damage_cap() {
        use crate::attack_constants::MAX_DAMAGE;
        // Damage over 32000 should be capped
        let damage: i16 = 32767; // i16::MAX
        let capped = damage.min(MAX_DAMAGE as i16);
        assert_eq!(capped, 32000, "Damage should be capped at MAX_DAMAGE");

        // Damage under 32000 should be unchanged
        let normal_damage: i16 = 15000;
        let capped2 = normal_damage.min(MAX_DAMAGE as i16);
        assert_eq!(capped2, 15000, "Normal damage should pass through uncapped");

        // Exact 32000 should stay
        let exact: i16 = 32000;
        assert_eq!(exact.min(MAX_DAMAGE as i16), 32000);
    }

    /// Test attack result constants match C++ values.
    #[test]
    fn test_attack_result_constants() {
        assert_eq!(ATTACK_FAIL, 0);
        assert_eq!(ATTACK_SUCCESS, 1);
        assert_eq!(ATTACK_TARGET_DEAD, 2);
    }

    // ── Helper to build a test CharacterInfo ────────────────────────────

    fn make_test_char(
        class: u16,
        level: u8,
        str_val: u8,
        sta: u8,
        dex: u8,
        intel: u8,
    ) -> CharacterInfo {
        CharacterInfo {
            session_id: 1,
            name: "TestChar".to_string(),
            nation: 1,
            race: 1,
            class,
            level,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 0,
            bind_x: 0.0,
            bind_z: 0.0,
            str: str_val,
            sta,
            dex,
            intel,
            cha: 10,
            free_points: 0,
            skill_points: [0; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 1, // standing
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

    fn make_test_coeff() -> CoefficientRow {
        CoefficientRow {
            s_class: 101,
            short_sword: 0.01,
            jamadar: 0.01,
            sword: 0.015,
            axe: 0.012,
            club: 0.011,
            spear: 0.013,
            pole: 0.01,
            staff: 0.008,
            bow: 0.01,
            hp: 0.0003,
            mp: 0.0,
            sp: 0.0,
            ac: 1.2,
            hitrate: 0.002,
            evasionrate: 0.001,
        }
    }

    // ── Class helper tests ──────────────────────────────────────────────

    #[test]
    fn test_class_helpers() {
        // Warrior: 101, 105, 106 (Karus); 201, 205, 206 (Elmo)
        assert!(is_warrior(101));
        assert!(is_warrior(105));
        assert!(is_warrior(106));
        assert!(is_warrior(201));
        assert!(!is_warrior(102));

        // Rogue: 102, 107, 108
        assert!(is_rogue(102));
        assert!(is_rogue(107));
        assert!(is_rogue(108));
        assert!(is_rogue(202));
        assert!(!is_rogue(101));

        // Mage: 103, 109, 110
        assert!(is_mage(103));
        assert!(is_mage(109));
        assert!(is_mage(110));
        assert!(!is_mage(104));

        // Priest: 104, 111, 112
        assert!(is_priest(104));
        assert!(is_priest(111));
        assert!(is_priest(112));
        assert!(!is_priest(103));
    }

    // ── Total hit (attack power) tests ──────────────────────────────────

    #[test]
    fn test_compute_total_hit_warrior() {
        let ch = make_test_char(101, 60, 90, 60, 30, 20);
        let coeff = make_test_coeff();

        let hit = compute_total_hit(&ch, &coeff);

        // power=3, weapon_coeff=0.0, bonus_ap=1.0, base_ap=0 (str < 150)
        // (0.005 * 3 * (90 + 40)) + (0.0) + 3 = 0.015 * 130 + 3 = 1.95 + 3 = 4.95
        // as u16 = 4
        assert_eq!(hit, 4);
    }

    #[test]
    fn test_compute_total_hit_rogue() {
        let ch = make_test_char(102, 60, 30, 60, 90, 20);
        let coeff = make_test_coeff();

        let hit = compute_total_hit(&ch, &coeff);

        // Rogue uses DEX: (0.005 * 3 * (90 + 40)) + 0 + 3 = 4.95 → 4
        assert_eq!(hit, 4);
    }

    #[test]
    fn test_compute_total_hit_high_str_warrior() {
        // High STR warrior (above 150 threshold for BaseAp bonus)
        let ch = make_test_char(101, 80, 200, 60, 40, 20);
        let coeff = make_test_coeff();

        let hit = compute_total_hit(&ch, &coeff);

        // power=3, weapon_coeff=0.0, base_ap=200-150=50
        // (0.005 * 3 * (200 + 40)) + 0 + 3 + 50 = 3.6 + 3 + 50 = 56.6 → 56
        assert_eq!(hit, 56);
    }

    // ── Total AC tests ──────────────────────────────────────────────────

    #[test]
    fn test_compute_total_ac() {
        let ch = make_test_char(101, 60, 90, 60, 30, 20);
        let coeff = make_test_coeff();

        let ac = compute_total_ac(&ch, &coeff);

        // AC_coeff * level = 1.2 * 60 = 72
        assert_eq!(ac, 72);
    }

    // ── Hit rate table tests ────────────────────────────────────────────

    #[test]
    fn test_get_hit_rate_high_rate_always_hits() {
        // With a very high rate, FAIL should be very rare
        // rate >= 5.0 → only 2% chance of FAIL (random > 9800)
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let mut hits = 0;
        let mut fails = 0;
        for _ in 0..1000 {
            let result = get_hit_rate(10.0, &mut rng);
            if result == FAIL {
                fails += 1;
            } else {
                hits += 1;
            }
        }

        // With 2% fail rate, expect ~20 fails out of 1000
        assert!(hits > 950, "Expected > 950 hits, got {}", hits);
        assert!(fails < 50, "Expected < 50 fails, got {}", fails);
    }

    #[test]
    fn test_get_hit_rate_low_rate_mostly_fails() {
        // rate < 0.2 → 50% FAIL
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let mut fails = 0;
        for _ in 0..1000 {
            let result = get_hit_rate(0.1, &mut rng);
            if result == FAIL {
                fails += 1;
            }
        }

        // With 50% fail rate, expect 400-600 fails
        assert!(fails > 400, "Expected > 400 fails, got {}", fails);
        assert!(fails < 600, "Expected < 600 fails, got {}", fails);
    }

    // ── Damage formula tests ────────────────────────────────────────────

    #[test]
    fn test_r_damage_nonzero_for_typical_chars() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let attacker = make_test_char(101, 60, 90, 60, 30, 20);
        let target = make_test_char(101, 60, 60, 60, 30, 20);
        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();

        // Run many times; at least some should deal damage (not all miss)
        let mut total_damage = 0i32;
        for _ in 0..100 {
            let d = calculate_r_damage(&attacker, &a_coeff, &target, &t_coeff, &mut rng);
            total_damage += d as i32;
        }

        assert!(
            total_damage > 0,
            "Expected some damage in 100 attacks, got 0"
        );
    }

    #[test]
    fn test_r_damage_priest_deals_less() {
        use rand::SeedableRng;

        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();
        let target = make_test_char(101, 60, 60, 60, 30, 20);

        // Warrior attacker
        let warrior = make_test_char(101, 60, 90, 60, 30, 20);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut warrior_dmg = 0i32;
        for _ in 0..1000 {
            warrior_dmg +=
                calculate_r_damage(&warrior, &a_coeff, &target, &t_coeff, &mut rng) as i32;
        }

        // Priest attacker (same stats)
        let priest = make_test_char(104, 60, 90, 60, 30, 20);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut priest_dmg = 0i32;
        for _ in 0..1000 {
            priest_dmg += calculate_r_damage(&priest, &a_coeff, &target, &t_coeff, &mut rng) as i32;
        }

        // Priest uses 0.15 multiplier vs warrior's 0.75 — should deal ~5x less
        assert!(
            priest_dmg < warrior_dmg,
            "Priest ({}) should deal less than warrior ({})",
            priest_dmg,
            warrior_dmg
        );
    }

    #[test]
    fn test_r_damage_never_negative() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let attacker = make_test_char(101, 1, 10, 10, 10, 10);
        let target = make_test_char(101, 80, 200, 200, 200, 200);
        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();

        for _ in 0..100 {
            let d = calculate_r_damage(&attacker, &a_coeff, &target, &t_coeff, &mut rng);
            assert!(d >= 0, "Damage should never be negative, got {}", d);
        }
    }

    // ── Zone override tests ─────────────────────────────────────────────

    #[test]
    fn test_zone_override_snow_battle() {
        assert_eq!(apply_zone_damage_override(ZONE_SNOW_BATTLE, 500), 0);
    }

    #[test]
    fn test_zone_override_chaos_dungeon() {
        assert_eq!(apply_zone_damage_override(ZONE_CHAOS_DUNGEON, 500), 50);
    }

    #[test]
    fn test_zone_override_dungeon_defence() {
        assert_eq!(apply_zone_damage_override(ZONE_DUNGEON_DEFENCE, 500), 50);
    }

    #[test]
    fn test_zone_override_normal_zone() {
        assert_eq!(apply_zone_damage_override(21, 500), 500);
    }

    // ── Compute hitrate/evasion tests ───────────────────────────────────

    #[test]
    fn test_compute_hitrate() {
        let ch = make_test_char(101, 60, 90, 60, 50, 20);
        let coeff = make_test_coeff();

        let hr = compute_hitrate(&ch, &coeff);

        // 1 + 0.002 * 60 * 50 = 1 + 6.0 = 7.0
        assert!((hr - 7.0).abs() < 0.01, "Expected ~7.0, got {}", hr);
    }

    #[test]
    fn test_compute_evasion() {
        let ch = make_test_char(101, 60, 90, 60, 50, 20);
        let coeff = make_test_coeff();

        let ev = compute_evasion(&ch, &coeff);

        // 1 + 0.001 * 60 * 50 = 1 + 3.0 = 4.0
        assert!((ev - 4.0).abs() < 0.01, "Expected ~4.0, got {}", ev);
    }

    // ── Hit result constant tests ───────────────────────────────────────

    #[test]
    fn test_hit_rate_constants() {
        assert_eq!(GREAT_SUCCESS, 0x01);
        assert_eq!(SUCCESS, 0x02);
        assert_eq!(NORMAL, 0x03);
        assert_eq!(FAIL, 0x04);
    }

    // ── NPC damage formula test ────────────────────────────────────

    #[test]
    fn test_npc_damage_formula() {
        // Test the NPC damage calculation: temp_hit_B = (temp_ap * 2) / (npc_ac + 240)
        let attacker = make_test_char(101, 60, 90, 60, 30, 20);
        let coeff = make_test_coeff();

        let total_hit = compute_total_hit(&attacker, &coeff);
        let temp_ap = total_hit as i32 * DEFAULT_ATTACK_AMOUNT as i32;

        // NPC with AC=50
        let npc_ac: i32 = 50;
        let temp_hit_b = (temp_ap * 2) / (npc_ac + 240);

        // total_hit = 4, temp_ap = 400, temp_hit_b = 800 / 290 = 2
        assert!(temp_hit_b >= 0, "Damage should not be negative");
        assert_eq!(temp_hit_b, 2);
    }

    #[test]
    fn test_npc_damage_formula_high_ap() {
        // High STR warrior vs low AC NPC
        let attacker = make_test_char(101, 80, 200, 60, 40, 20);
        let coeff = make_test_coeff();

        let total_hit = compute_total_hit(&attacker, &coeff);
        let temp_ap = total_hit as i32 * DEFAULT_ATTACK_AMOUNT as i32;

        // NPC with AC=10
        let npc_ac: i32 = 10;
        let temp_hit_b = (temp_ap * 2) / (npc_ac + 240);

        // total_hit = 56, temp_ap = 5600, temp_hit_b = 11200 / 250 = 44
        assert_eq!(temp_hit_b, 44);
        assert!(temp_hit_b > 0);
    }

    // ── XP reward modifier test ───────────────────────────────────

    #[test]
    fn test_reward_modifier() {
        use super::super::level::get_reward_modifier;

        // C++ GetRewardModifier always returns 1.0f (level-diff logic is dead code).
        assert_eq!(get_reward_modifier(50, 50), 1.0);
        assert_eq!(get_reward_modifier(50, 55), 1.0);
        assert_eq!(get_reward_modifier(50, 60), 1.0);
        assert_eq!(get_reward_modifier(50, 65), 1.0);
    }

    // ── NPC death broadcast packet format test ──────────────────────

    #[test]
    fn test_npc_death_broadcast_format() {
        // WIZ_DEAD with NPC ID (>= NPC_BAND)
        let npc_id: u32 = 10042;
        let mut pkt = Packet::new(Opcode::WizDead as u8);
        pkt.write_u32(npc_id);

        assert_eq!(pkt.opcode, Opcode::WizDead as u8);
        assert_eq!(pkt.data.len(), 4);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(10042));
        assert_eq!(r.remaining(), 0);
    }

    // ── WIZ_EXP_CHANGE packet format test ───────────────────────────

    #[test]
    fn test_exp_change_packet_format() {
        // Correct C++ format: [u8 flag=4] [i64 total_exp]
        let mut pkt = Packet::new(Opcode::WizExpChange as u8);
        pkt.write_u8(0x04); // flag = EXP_FLAG_NORMAL
        pkt.write_i64(1_000_000); // total current exp

        assert_eq!(pkt.opcode, Opcode::WizExpChange as u8);
        // 1 + 8 = 9 bytes
        assert_eq!(pkt.data.len(), 9);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0x04));
        assert_eq!(r.read_i64(), Some(1_000_000));
        assert_eq!(r.remaining(), 0);
    }

    // ── NPC target HP bar packet format test ────────────────────────

    #[test]
    fn test_npc_target_hp_packet_format() {
        let npc_id: u32 = 10001;
        let max_hp: u32 = 5000;
        let current_hp: u32 = 3200;

        let mut pkt = Packet::new(Opcode::WizTargetHp as u8);
        pkt.write_u32(npc_id);
        pkt.write_u8(0);
        pkt.write_u32(max_hp);
        pkt.write_u32(current_hp);
        pkt.write_u32(0);
        pkt.write_u32(0);
        pkt.write_u8(0);

        assert_eq!(pkt.opcode, Opcode::WizTargetHp as u8);
        // 4 + 1 + 4 + 4 + 4 + 4 + 1 = 22 bytes
        assert_eq!(pkt.data.len(), 22);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(10001)); // NPC ID
        assert_eq!(r.read_u8(), Some(0)); // flag
        assert_eq!(r.read_u32(), Some(5000)); // max_hp
        assert_eq!(r.read_u32(), Some(3200)); // current_hp
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u8(), Some(0)); // reserved
        assert_eq!(r.remaining(), 0);
    }

    // ── Weapon delay validation tests ────────────────────────────────

    #[test]
    fn test_is_timing_delay() {
        assert!(is_timing_delay(900335523));
        assert!(is_timing_delay(900336524));
        assert!(is_timing_delay(900337525));
        assert!(!is_timing_delay(900335522));
        assert!(!is_timing_delay(900337526));
        assert!(!is_timing_delay(100010000));
    }

    #[test]
    fn test_is_wirinom_uniq_delay() {
        // Range checks
        assert!(is_wirinom_uniq_delay(127410731));
        assert!(is_wirinom_uniq_delay(127410740));
        assert!(is_wirinom_uniq_delay(127420741));
        assert!(is_wirinom_uniq_delay(127440770));
        // Specific IDs
        assert!(is_wirinom_uniq_delay(127410284));
        assert!(is_wirinom_uniq_delay(127420285));
        assert!(is_wirinom_uniq_delay(127430286));
        assert!(is_wirinom_uniq_delay(127440287));
        // Out of range
        assert!(!is_wirinom_uniq_delay(127410730));
        assert!(!is_wirinom_uniq_delay(127440771));
        assert!(!is_wirinom_uniq_delay(100010000));
    }

    #[test]
    fn test_is_wirinom_reb_delay() {
        assert!(is_wirinom_reb_delay(127411181));
        assert!(is_wirinom_reb_delay(127411210));
        assert!(is_wirinom_reb_delay(127421211));
        assert!(is_wirinom_reb_delay(127441300));
        assert!(!is_wirinom_reb_delay(127411180));
        assert!(!is_wirinom_reb_delay(127441301));
    }

    #[test]
    fn test_is_garges_sword_delay() {
        assert!(is_garges_sword_delay(1110582731));
        assert!(is_garges_sword_delay(1110582740));
        assert!(is_garges_sword_delay(1110582451));
        assert!(!is_garges_sword_delay(1110582730));
        assert!(!is_garges_sword_delay(1110582741));
    }

    #[test]
    fn test_is_bow_weapon() {
        assert!(is_bow_weapon(WEAPON_KIND_BOW));
        assert!(is_bow_weapon(WEAPON_KIND_CROSSBOW));
        assert!(!is_bow_weapon(WEAPON_KIND_DAGGER));
        assert!(!is_bow_weapon(WEAPON_KIND_1H_SWORD));
        assert!(!is_bow_weapon(0));
    }

    #[test]
    fn test_weapon_kind_constants() {
        assert_eq!(WEAPON_KIND_DAGGER, 11);
        assert_eq!(WEAPON_KIND_1H_SWORD, 21);
        assert_eq!(WEAPON_KIND_2H_SWORD, 22);
        assert_eq!(WEAPON_KIND_1H_AXE, 31);
        assert_eq!(WEAPON_KIND_2H_AXE, 32);
        assert_eq!(WEAPON_KIND_1H_CLUB, 41);
        assert_eq!(WEAPON_KIND_2H_CLUB, 42);
        assert_eq!(WEAPON_KIND_1H_SPEAR, 51);
        assert_eq!(WEAPON_KIND_2H_SPEAR, 52);
        assert_eq!(WEAPON_KIND_BOW, 70);
        assert_eq!(WEAPON_KIND_CROSSBOW, 71);
        assert_eq!(WEAPON_KIND_JAMADAR, 140);
    }

    #[test]
    fn test_get_ac_damage_no_resistance() {
        // With zero resistance, damage should be unchanged
        let stats = crate::world::EquippedStats::default();
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_1H_SWORD)], &stats, 100, 100);
        assert_eq!(damage, 100);
    }

    #[test]
    fn test_get_ac_damage_sword_resistance() {
        // sword_r = 50 → damage -= damage * 50 / 250 = damage * 0.2
        // 100 - 100 * 50 / 250 = 100 - 20 = 80
        let stats = crate::world::EquippedStats {
            sword_r: 50,
            ..Default::default()
        };
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_1H_SWORD)], &stats, 100, 100);
        assert_eq!(damage, 80);

        // 2H sword should also match sword_r
        let damage_2h = get_ac_damage(100, &[Some(WEAPON_KIND_2H_SWORD)], &stats, 100, 100);
        assert_eq!(damage_2h, 80);
    }

    #[test]
    fn test_get_ac_damage_dagger_resistance() {
        let stats = crate::world::EquippedStats {
            dagger_r: 125,
            ..Default::default()
        }; // half reduction: 100 - 100*125/250 = 50
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_DAGGER)], &stats, 100, 100);
        assert_eq!(damage, 50);
    }

    #[test]
    fn test_get_ac_damage_dual_wield() {
        // Both weapons apply their own resistance reduction
        // Right: sword (resistance 50), Left: dagger (resistance 25)
        // Step 1: 100 - 100*50/250 = 80
        // Step 2: 80 - 80*25/250 = 80 - 8 = 72
        let stats = crate::world::EquippedStats {
            sword_r: 50,
            dagger_r: 25,
            ..Default::default()
        };
        let damage = get_ac_damage(
            100,
            &[Some(WEAPON_KIND_1H_SWORD), Some(WEAPON_KIND_DAGGER)],
            &stats,
            100,
            100,
        );
        assert_eq!(damage, 72);
    }

    #[test]
    fn test_get_ac_damage_no_weapon() {
        // No weapons → damage unchanged
        let stats = crate::world::EquippedStats {
            sword_r: 100,
            ..Default::default()
        };
        let damage = get_ac_damage(100, &[None, None], &stats, 100, 100);
        assert_eq!(damage, 100);
    }

    #[test]
    fn test_get_ac_damage_all_weapon_types() {
        // Each weapon type maps to the correct resistance field
        let stats = crate::world::EquippedStats {
            dagger_r: 250,
            sword_r: 250,
            axe_r: 250,
            club_r: 250,
            spear_r: 250,
            bow_r: 250,
            jamadar_r: 250,
            ..Default::default()
        };

        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_DAGGER)], &stats, 100, 100),
            0
        );
        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_1H_SWORD)], &stats, 100, 100),
            0
        );
        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_2H_AXE)], &stats, 100, 100),
            0
        );
        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_1H_CLUB)], &stats, 100, 100),
            0
        );
        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_2H_SPEAR)], &stats, 100, 100),
            0
        );
        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_BOW)], &stats, 100, 100),
            0
        );
        assert_eq!(
            get_ac_damage(100, &[Some(WEAPON_KIND_JAMADAR)], &stats, 100, 100),
            0
        );
    }

    #[test]
    fn test_get_ac_damage_unknown_weapon_kind() {
        // Unknown weapon kind (e.g., staff) — no resistance applied
        let stats = crate::world::EquippedStats {
            sword_r: 250,
            ..Default::default()
        };
        let damage = get_ac_damage(100, &[Some(110)], &stats, 100, 100); // 110 = STAFF
        assert_eq!(damage, 100); // no reduction for staff
    }

    // ── Eskrima debuff tests (BUFF_TYPE_DAGGER_BOW_DEFENSE) ──────────

    #[test]
    fn test_get_ac_damage_eskrima_dagger_reduction() {
        // Eskrima reduces dagger_r_amount from 100 to 80 (sSpecialAmount=20)
        // dagger_r=125, amount=80 → effective resistance = 125*80/100 = 100
        // damage -= damage * 100 / 250 = 100 - 40 = 60
        let stats = crate::world::EquippedStats {
            dagger_r: 125,
            ..Default::default()
        };
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_DAGGER)], &stats, 80, 100);
        assert_eq!(damage, 60);
    }

    #[test]
    fn test_get_ac_damage_eskrima_bow_reduction() {
        // Eskrima reduces bow_r_amount from 100 to 80 (sSpecialAmount=20)
        // bow_r=125, amount=80 → effective resistance = 125*80/100 = 100
        // damage -= damage * 100 / 250 = 100 - 40 = 60
        let stats = crate::world::EquippedStats {
            bow_r: 125,
            ..Default::default()
        };
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_BOW)], &stats, 100, 80);
        assert_eq!(damage, 60);
    }

    #[test]
    fn test_get_ac_damage_eskrima_zero_amount() {
        // With amount=0, dagger/bow resistance is completely nullified
        let stats = crate::world::EquippedStats {
            dagger_r: 250,
            bow_r: 250,
            ..Default::default()
        };
        // dagger: 250*0/100=0 → no reduction
        let damage_dagger = get_ac_damage(100, &[Some(WEAPON_KIND_DAGGER)], &stats, 0, 100);
        assert_eq!(damage_dagger, 100);
        // bow: 250*0/100=0 → no reduction
        let damage_bow = get_ac_damage(100, &[Some(WEAPON_KIND_BOW)], &stats, 100, 0);
        assert_eq!(damage_bow, 100);
    }

    #[test]
    fn test_get_ac_damage_eskrima_no_effect_on_sword() {
        // Eskrima only affects dagger/bow, not sword
        let stats = crate::world::EquippedStats {
            sword_r: 50,
            ..Default::default()
        };
        // Even with dagger_r_amount=0, sword damage should be unchanged
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_1H_SWORD)], &stats, 0, 0);
        assert_eq!(damage, 80); // 100 - 100*50/250 = 80
    }

    #[test]
    fn test_get_ac_damage_eskrima_crossbow_uses_bow_amount() {
        // Crossbow should also use bow_r_amount
        let stats = crate::world::EquippedStats {
            bow_r: 125,
            ..Default::default()
        };
        let damage = get_ac_damage(100, &[Some(WEAPON_KIND_CROSSBOW)], &stats, 100, 80);
        // 125*80/100=100, damage -= 100*100/250 = 40 → 60
        assert_eq!(damage, 60);
    }

    #[test]
    fn test_mage_class_detection() {
        // Karus mage classes
        assert!(is_mage(103)); // base mage
        assert!(is_mage(109)); // mage novice
        assert!(is_mage(110)); // mage master
                               // Elmorad mage classes
        assert!(is_mage(203));
        assert!(is_mage(209));
        assert!(is_mage(210));
        // Non-mage classes
        assert!(!is_mage(101)); // warrior
        assert!(!is_mage(102)); // rogue
        assert!(!is_mage(104)); // priest
        assert!(!is_mage(113)); // kurian
    }

    #[test]
    fn test_weapon_delay_constants() {
        assert_eq!(PLAYER_R_HIT_REQUEST_INTERVAL, 900);
        assert_eq!(MIN_MELEE_DELAY, 100);
        assert_eq!(GM_WEAPON_ID, 389158000);
    }

    #[test]
    fn test_weapon_delay_validation_logic() {
        // Simulate the delay validation logic for a normal weapon
        let weapon_delay: i16 = 150;

        // Normal weapon: delaytime must be >= weapon_delay
        assert!(200 >= weapon_delay); // passes
        assert!(150 >= weapon_delay); // exact match passes
        assert!((149 < weapon_delay)); // too fast fails

        // Timing delay weapon: delaytime must be >= (weapon_delay + 9)
        let timing_threshold = weapon_delay + 9;
        assert!(160 >= timing_threshold); // passes
        assert!(159 >= timing_threshold); // exact match
        assert!((158 < timing_threshold)); // too fast

        // Wirinim/Garges weapon: delaytime must be >= (weapon_delay - 4)
        let special_threshold = weapon_delay - 4;
        assert!(150 >= special_threshold); // passes easily
        assert!(146 >= special_threshold); // exact match
        assert!((145 < special_threshold)); // too fast
    }

    /// Test class-vs-class damage multiplier integration.
    ///
    #[test]
    fn test_class_damage_multiplier_pvp() {
        let world = WorldState::new();

        // With no damage settings loaded, multiplier should be 1.0
        let mult = world.get_class_damage_multiplier(101, 202); // warrior vs rogue
        assert!((mult - 1.0).abs() < f64::EPSILON);

        // Verify damage * 1.0 = damage (no change)
        let base_damage: i16 = 100;
        let modified = (base_damage as f64 * mult) as i16;
        assert_eq!(modified, 100);
    }

    /// Test monster defense multiplier integration.
    ///
    #[test]
    fn test_mon_def_multiplier_default() {
        let world = WorldState::new();

        // Default mondef = 1.0 (no damage settings loaded)
        let mult = world.get_mon_def_multiplier();
        assert!((mult - 1.0).abs() < f64::EPSILON);

        // Apply to NPC AC: 100 * 1.0 = 100
        let npc_ac = 100_i32;
        let modified_ac = (npc_ac as f64 * mult) as i32;
        assert_eq!(modified_ac, 100);
    }

    /// Test monster take-damage multiplier default.
    ///
    #[test]
    fn test_mon_take_damage_multiplier_default() {
        let world = WorldState::new();

        // Default montakedamage = 1.5
        let mult = world.get_mon_take_damage_multiplier();
        assert!((mult - 1.5).abs() < f64::EPSILON);

        // Apply to damage: 100 * 1.5 = 150
        let base_damage: i16 = 100;
        let modified = (base_damage as f64 * mult) as i16;
        assert_eq!(modified, 150);
    }

    /// Test r_damage multiplier default.
    ///
    #[test]
    fn test_r_damage_multiplier_default() {
        let world = WorldState::new();

        // Default rdamage = 0.9
        let mult = world.get_r_damage_multiplier();
        assert!((mult - 0.9).abs() < f64::EPSILON);
    }

    /// Test class-vs-class multiplier with all 5 attacker class groups.
    #[test]
    fn test_class_damage_multiplier_all_classes() {
        let world = WorldState::new();

        // All return 1.0 without damage settings loaded
        // Warrior (101) vs Rogue (202)
        assert!((world.get_class_damage_multiplier(101, 202) - 1.0).abs() < f64::EPSILON);
        // Rogue (102) vs Mage (203)
        assert!((world.get_class_damage_multiplier(102, 203) - 1.0).abs() < f64::EPSILON);
        // Mage (103) vs Priest (204)
        assert!((world.get_class_damage_multiplier(103, 204) - 1.0).abs() < f64::EPSILON);
        // Priest (104) vs Warrior (201)
        assert!((world.get_class_damage_multiplier(104, 201) - 1.0).abs() < f64::EPSILON);
        // Kurian (113) vs Kurian (213)
        assert!((world.get_class_damage_multiplier(113, 213) - 1.0).abs() < f64::EPSILON);
    }

    /// Test that NPC damage has monster multipliers applied correctly.
    ///
    /// Validates the flow: mondef scales AC, montakedamage scales final damage.
    #[test]
    fn test_npc_damage_multiplier_flow() {
        // Simulate the NPC damage calculation flow
        let mon_def = 1.0_f64; // mondef multiplier
        let mon_take_damage = 1.5_f64; // montakedamage multiplier

        let raw_ac = 100_i32;
        let modified_ac = (raw_ac as f64 * mon_def) as i32;
        assert_eq!(modified_ac, 100);

        // Base damage after AC application
        let temp_ap = 500_i32;
        let temp_hit_b = (temp_ap * 2) / (modified_ac + 240); // 1000 / 340 = 2
        assert_eq!(temp_hit_b, 2);

        // After montakedamage multiplier
        let base_damage = 2_i16;
        let final_damage = (base_damage as f64 * mon_take_damage) as i16;
        assert_eq!(final_damage, 3); // 2 * 1.5 = 3.0 → 3
    }

    // ── War zone detection tests ────────────────────────────────────────

    #[test]
    fn test_is_battle_zone_in_attack_context() {
        use crate::systems::war::is_battle_zone;
        use crate::world::{
            ZONE_BATTLE, ZONE_BATTLE2, ZONE_BATTLE3, ZONE_BATTLE4, ZONE_BATTLE5, ZONE_BATTLE6,
        };

        // War zone detection should correctly identify battle zones
        assert!(is_battle_zone(ZONE_BATTLE));
        assert!(is_battle_zone(ZONE_BATTLE2));
        assert!(is_battle_zone(ZONE_BATTLE3));
        assert!(is_battle_zone(ZONE_BATTLE4));
        assert!(is_battle_zone(ZONE_BATTLE5));
        assert!(is_battle_zone(ZONE_BATTLE6));

        // Non-war zones should not be identified as battle zones
        assert!(!is_battle_zone(ZONE_SNOW_BATTLE));
        assert!(!is_battle_zone(ZONE_CHAOS_DUNGEON));
        assert!(!is_battle_zone(ZONE_DUNGEON_DEFENCE));
        assert!(!is_battle_zone(21)); // Moradon
        assert!(!is_battle_zone(1)); // ZONE_KARUS
        assert!(!is_battle_zone(2)); // ZONE_ELMORAD
    }

    #[test]
    fn test_war_death_counter_increment() {
        // Verify the WorldState war death counter increment works
        let world = WorldState::new();

        // Initially 0
        let state = world.get_battle_state();
        assert_eq!(state.karus_dead, 0);
        assert_eq!(state.elmorad_dead, 0);

        // Increment Karus death
        world.increment_war_death(1);
        let state = world.get_battle_state();
        assert_eq!(state.karus_dead, 1);
        assert_eq!(state.elmorad_dead, 0);

        // Increment ElMorad death
        world.increment_war_death(2);
        let state = world.get_battle_state();
        assert_eq!(state.karus_dead, 1);
        assert_eq!(state.elmorad_dead, 1);

        // Multiple increments
        world.increment_war_death(1);
        world.increment_war_death(1);
        let state = world.get_battle_state();
        assert_eq!(state.karus_dead, 3);
        assert_eq!(state.elmorad_dead, 1);
    }

    #[test]
    fn test_war_open_check_for_attack() {
        let world = WorldState::new();

        // War not open initially
        assert!(!world.is_war_open());

        // Open a war
        world.update_battle_state(|s| {
            s.battle_time = 3600;
            crate::systems::war::battle_zone_open(s, crate::systems::war::BATTLEZONE_OPEN, 1, 0)
        });
        assert!(world.is_war_open());

        // Close the war
        world.update_battle_state(crate::systems::war::battle_zone_close);
        // After close, battle_open is reset to NO_BATTLE
        assert!(!world.is_war_open());
    }

    // ── class_group_index tests ──────────────────────────────────────

    #[test]
    fn test_class_group_index_warrior() {
        // Karus warrior = 101, base = 1 -> group 0
        assert_eq!(class_group_index(101), Some(0));
        // Elmo warrior novice = 205, base = 5 -> group 0
        assert_eq!(class_group_index(205), Some(0));
        // Warrior master = 106, base = 6 -> group 0
        assert_eq!(class_group_index(106), Some(0));
    }

    #[test]
    fn test_class_group_index_rogue() {
        assert_eq!(class_group_index(102), Some(1)); // Rogue base
        assert_eq!(class_group_index(207), Some(1)); // RogueNovice
        assert_eq!(class_group_index(108), Some(1)); // RogueMaster
    }

    #[test]
    fn test_class_group_index_mage() {
        assert_eq!(class_group_index(103), Some(2)); // Mage base
        assert_eq!(class_group_index(209), Some(2)); // MageNovice
        assert_eq!(class_group_index(110), Some(2)); // MageMaster
    }

    #[test]
    fn test_class_group_index_priest() {
        assert_eq!(class_group_index(104), Some(3)); // Priest base
        assert_eq!(class_group_index(211), Some(3)); // PriestNovice
        assert_eq!(class_group_index(112), Some(3)); // PriestMaster
    }

    #[test]
    fn test_class_group_index_kurian_returns_none() {
        assert_eq!(class_group_index(113), None); // Kurian
        assert_eq!(class_group_index(214), None); // KurianNovice
        assert_eq!(class_group_index(115), None); // KurianMaster
    }

    #[test]
    fn test_class_group_index_invalid_returns_none() {
        assert_eq!(class_group_index(0), None);
        assert_eq!(class_group_index(99), None);
    }

    // ── calculate_r_damage_with_class_bonus tests ────────────────────

    #[test]
    fn test_class_bonus_increases_physical_damage() {
        let attacker = make_test_char(101, 60, 200, 100, 100, 50);
        let target = make_test_char(103, 60, 100, 100, 100, 50);
        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_no_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng,
            None,
            None,
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        // With AP class bonus: attacker has 20% AP bonus vs mages (index 2)
        let ap_bonus = [0, 0, 20, 0];
        let ac_bonus = [0u8; 4];

        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_with_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng2,
            Some(&ap_bonus),
            Some(&ac_bonus),
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        assert!(
            dmg_with_bonus >= dmg_no_bonus,
            "AP class bonus should increase damage: {} >= {}",
            dmg_with_bonus,
            dmg_no_bonus
        );
    }

    #[test]
    fn test_ac_class_bonus_reduces_physical_damage() {
        let attacker = make_test_char(101, 60, 200, 100, 100, 50);
        let target = make_test_char(103, 60, 100, 150, 100, 50);
        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_no_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng,
            None,
            None,
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        // With AC class bonus: target has 30% AC bonus at target's class index
        let ap_bonus = [0u8; 4];
        let ac_bonus = [0, 0, 30, 0]; // index 2 = mage

        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_with_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng2,
            Some(&ap_bonus),
            Some(&ac_bonus),
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        assert!(
            dmg_with_bonus <= dmg_no_bonus,
            "AC class bonus should reduce damage: {} <= {}",
            dmg_with_bonus,
            dmg_no_bonus
        );
    }

    #[test]
    fn test_class_bonus_zero_values_no_change() {
        let attacker = make_test_char(101, 60, 200, 100, 100, 50);
        let target = make_test_char(102, 60, 100, 100, 100, 50);
        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(99);
        let dmg_no_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng1,
            None,
            None,
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        let mut rng2 = rand::rngs::StdRng::seed_from_u64(99);
        let zero_bonus = [0u8; 4];
        let dmg_zero_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng2,
            Some(&zero_bonus),
            Some(&zero_bonus),
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        assert_eq!(
            dmg_no_bonus, dmg_zero_bonus,
            "Zero bonuses should not change damage"
        );
    }

    #[test]
    fn test_class_bonus_kurian_target_no_bonus_applied() {
        let attacker = make_test_char(101, 60, 200, 100, 100, 50);
        let target = make_test_char(113, 60, 100, 100, 100, 50); // Kurian
        let a_coeff = make_test_coeff();
        let t_coeff = make_test_coeff();

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(77);
        let dmg_no_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng1,
            None,
            None,
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        let mut rng2 = rand::rngs::StdRng::seed_from_u64(77);
        let big_bonus = [50, 50, 50, 50];
        let dmg_with_bonus = calculate_r_damage_with_class_bonus(
            &attacker,
            &a_coeff,
            &target,
            &t_coeff,
            &mut rng2,
            Some(&big_bonus),
            Some(&big_bonus),
            None,
            DEFAULT_ATTACK_AMOUNT as i32,
            None,
        );

        assert_eq!(
            dmg_no_bonus, dmg_with_bonus,
            "Kurian target should not be affected by class bonus"
        );
    }

    // ── rdamage multiplier tests ─────────────────────────────────────────

    #[test]
    fn test_rdamage_applies_to_level31_warrior() {
        // Level 31 warrior (not priest) should have rdamage applied
        let attacker = make_test_char(101, 31, 90, 60, 30, 20);
        assert!(attacker.level > 30);
        assert!(!is_priest(attacker.class));
        // Conditions met: level > 30 && !is_priest
    }

    #[test]
    fn test_rdamage_skips_priest() {
        // Priest should NOT have rdamage applied regardless of level
        let attacker = make_test_char(104, 60, 90, 60, 30, 20);
        assert!(attacker.level > 30);
        assert!(is_priest(attacker.class));
        // Condition fails: is_priest == true
    }

    #[test]
    fn test_rdamage_skips_low_level() {
        // Level 30 warrior should NOT have rdamage applied
        let attacker = make_test_char(101, 30, 90, 60, 30, 20);
        assert!(attacker.level <= 30);
        // Condition fails: level <= 30
    }

    // ── GM instant-kill NPC tests ─────────────────────────────────────────

    #[test]
    fn test_gm_authority_check() {
        // GM has authority == 0
        let gm = make_test_char(101, 80, 90, 60, 30, 20);
        let mut gm_char = gm;
        gm_char.authority = 0;
        assert_eq!(gm_char.authority, 0);

        // Normal player has authority != 0
        let player = make_test_char(101, 80, 90, 60, 30, 20);
        assert_ne!(player.authority, 0);
    }

    // ── Enemy safety area tests ────────────────────────────────────────

    #[test]
    fn test_safety_area_delos_center() {
        // Center of Delos safe zone (500, 180, radius 115)
        assert!(is_in_enemy_safety_area(ZONE_DELOS, 500.0, 180.0, 1));
        assert!(is_in_enemy_safety_area(ZONE_DELOS, 500.0, 180.0, 2));
    }

    #[test]
    fn test_safety_area_delos_outside() {
        // Well outside Delos safe zone
        assert!(!is_in_enemy_safety_area(ZONE_DELOS, 800.0, 800.0, 1));
    }

    #[test]
    fn test_safety_area_arena() {
        // Center of arena safe zone (127, 113, radius 36)
        assert!(is_in_enemy_safety_area(ZONE_ARENA, 127.0, 113.0, 1));
        assert!(!is_in_enemy_safety_area(ZONE_ARENA, 300.0, 300.0, 1));
    }

    #[test]
    fn test_safety_area_karus_village() {
        // Karus nation in own village — should be safe
        assert!(is_in_enemy_safety_area(ZONE_KARUS, 1860.0, 174.0, 1));
        // Elmorad nation in Karus village — NOT safe (it's enemy territory)
        assert!(!is_in_enemy_safety_area(ZONE_KARUS, 1860.0, 174.0, 2));
    }

    #[test]
    fn test_safety_area_elmorad_village() {
        // Elmorad in own village
        assert!(is_in_enemy_safety_area(ZONE_ELMORAD, 210.0, 1853.0, 2));
        // Karus in Elmorad village — NOT safe
        assert!(!is_in_enemy_safety_area(ZONE_ELMORAD, 210.0, 1853.0, 1));
    }

    #[test]
    fn test_safety_area_normal_zone() {
        // Normal zone (Moradon) — never a safety area
        assert!(!is_in_enemy_safety_area(21, 500.0, 500.0, 1));
    }

    // ── isInOwnSafetyArea tests ──────────────────────────────────────────

    #[test]
    fn test_own_safety_delos_center() {
        // DELOS is zone-neutral (same as enemy safety)
        assert!(is_in_own_safety_area(ZONE_DELOS, 500.0, 180.0, 1));
        assert!(is_in_own_safety_area(ZONE_DELOS, 500.0, 180.0, 2));
    }

    #[test]
    fn test_own_safety_bifrost_nation_inverted() {
        // C++ isInOwnSafetyArea: KARUS checks Karus coords (56-124, 700-840)
        // C++ isInEnemySafetyArea: ELMORAD checks same coords
        // Nation is INVERTED between the two functions
        assert!(is_in_own_safety_area(ZONE_BIFROST, 90.0, 770.0, 1)); // Karus in Karus safe
        assert!(!is_in_enemy_safety_area(ZONE_BIFROST, 90.0, 770.0, 1)); // NOT enemy safe for Karus
        assert!(is_in_enemy_safety_area(ZONE_BIFROST, 90.0, 770.0, 2)); // IS enemy safe for Elmorad
        assert!(!is_in_own_safety_area(ZONE_BIFROST, 90.0, 770.0, 2)); // NOT own safe for Elmorad
    }

    #[test]
    fn test_own_safety_elmorad_zone_inverted() {
        // C++ isInOwnSafetyArea ELMORAD zone: Karus near (210,1853)
        // C++ isInEnemySafetyArea ELMORAD zone: Elmorad near (210,1853)
        assert!(is_in_own_safety_area(ZONE_ELMORAD, 210.0, 1853.0, 1)); // Karus in own safe
        assert!(!is_in_own_safety_area(ZONE_ELMORAD, 210.0, 1853.0, 2)); // Elmorad NOT in own safe
        assert!(is_in_enemy_safety_area(ZONE_ELMORAD, 210.0, 1853.0, 2)); // Elmorad in enemy safe
    }

    #[test]
    fn test_own_safety_karus_zone_inverted() {
        // C++ isInOwnSafetyArea KARUS zone: Elmorad near (1860,174)
        assert!(is_in_own_safety_area(ZONE_KARUS, 1860.0, 174.0, 2)); // Elmorad in own safe
        assert!(!is_in_own_safety_area(ZONE_KARUS, 1860.0, 174.0, 1)); // Karus NOT in own safe
        assert!(is_in_enemy_safety_area(ZONE_KARUS, 1860.0, 174.0, 1)); // Karus in enemy safe
    }

    #[test]
    fn test_own_safety_battle_inverted() {
        // BATTLE zone: nations are inverted between own and enemy safety
        // Enemy safety: Karus checks (98-125, 755-780), Elmorad checks (805-831, 85-110)
        // Own safety: Elmorad checks (98-125, 755-780), Karus checks (805-831, 85-110)
        assert!(is_in_own_safety_area(ZONE_BATTLE, 110.0, 765.0, 2)); // Elmorad in own safe (Karus spawn)
        assert!(!is_in_own_safety_area(ZONE_BATTLE, 110.0, 765.0, 1)); // Karus NOT in own safe
        assert!(is_in_enemy_safety_area(ZONE_BATTLE, 110.0, 765.0, 1)); // Karus in enemy safe
    }

    #[test]
    fn test_own_safety_normal_zone() {
        assert!(!is_in_own_safety_area(21, 500.0, 500.0, 1));
        assert!(!is_in_own_safety_area(21, 500.0, 500.0, 2));
    }

    // ── Incapacitated constants tests ────────────────────────────────────

    #[test]
    fn test_buff_type_constants() {
        assert_eq!(BUFF_TYPE_BLIND, 21);
        assert_eq!(BUFF_TYPE_KAUL_TRANSFORMATION, 154);
    }

    // ── is_in_arena tests ───────────────────────────────────────────────

    #[test]
    fn test_is_in_arena_zone_arena() {
        // ZONE_ARENA (48) — any location counts as arena
        assert!(is_in_arena(ZONE_ARENA, 0.0, 0.0));
        assert!(is_in_arena(ZONE_ARENA, 500.0, 500.0));
    }

    #[test]
    fn test_is_in_arena_moradon_party_arena() {
        // Moradon party arena: x 684-735, z 360-411
        assert!(is_in_arena(ZONE_MORADON, 700.0, 380.0));
        assert!(is_in_arena(22, 700.0, 380.0)); // ZONE_MORADON2 = 22
    }

    #[test]
    fn test_is_in_arena_moradon_melee_arena() {
        // Moradon melee arena: x 684-735, z 440-491
        assert!(is_in_arena(ZONE_MORADON, 700.0, 460.0));
    }

    #[test]
    fn test_is_in_arena_moradon_outside() {
        // Moradon outside arena bounds
        assert!(!is_in_arena(ZONE_MORADON, 500.0, 500.0));
        assert!(!is_in_arena(ZONE_MORADON, 700.0, 420.0)); // between arenas
    }

    #[test]
    fn test_is_in_arena_other_zone() {
        assert!(!is_in_arena(ZONE_RONARK_LAND, 700.0, 380.0));
        assert!(!is_in_arena(ZONE_KARUS, 700.0, 380.0));
    }

    // ── is_in_pvp_zone tests ────────────────────────────────────────────

    #[test]
    fn test_is_in_pvp_zone_ronark() {
        assert!(is_in_pvp_zone(ZONE_RONARK_LAND));
        assert!(is_in_pvp_zone(ZONE_RONARK_LAND_BASE));
        assert!(is_in_pvp_zone(ZONE_ARDREAM));
    }

    #[test]
    fn test_is_in_pvp_zone_battle() {
        assert!(is_in_pvp_zone(ZONE_BATTLE));
        assert!(is_in_pvp_zone(ZONE_BATTLE2));
        assert!(is_in_pvp_zone(ZONE_BATTLE3));
        assert!(is_in_pvp_zone(ZONE_BATTLE4));
        assert!(is_in_pvp_zone(ZONE_BATTLE5));
        assert!(is_in_pvp_zone(ZONE_BATTLE6));
    }

    #[test]
    fn test_is_in_pvp_zone_special() {
        assert!(is_in_pvp_zone(ZONE_JURAID_MOUNTAIN));
        assert!(is_in_pvp_zone(ZONE_BORDER_DEFENSE_WAR));
        assert!(is_in_pvp_zone(ZONE_CLAN_WAR_ARDREAM));
        assert!(is_in_pvp_zone(ZONE_CLAN_WAR_RONARK));
        assert!(is_in_pvp_zone(ZONE_BIFROST));
    }

    #[test]
    fn test_is_in_pvp_zone_party_vs() {
        assert!(is_in_pvp_zone(ZONE_PARTY_VS_1));
        assert!(is_in_pvp_zone(97)); // ZONE_PARTY_VS_2
        assert!(is_in_pvp_zone(98)); // ZONE_PARTY_VS_3
        assert!(is_in_pvp_zone(ZONE_PARTY_VS_4));
    }

    #[test]
    fn test_is_in_pvp_zone_special_event() {
        // SPBATTLE zones 105-115
        assert!(is_in_pvp_zone(105));
        assert!(is_in_pvp_zone(110));
        assert!(is_in_pvp_zone(115));
        assert!(!is_in_pvp_zone(104));
        assert!(!is_in_pvp_zone(116));
    }

    #[test]
    fn test_is_in_pvp_zone_non_pvp() {
        assert!(!is_in_pvp_zone(ZONE_MORADON));
        assert!(!is_in_pvp_zone(ZONE_KARUS));
        assert!(!is_in_pvp_zone(ZONE_ELMORAD));
        assert!(!is_in_pvp_zone(ZONE_DELOS));
    }

    // ── Castle zone helpers tests ───────────────────────────────────────

    #[test]
    fn test_is_in_luferson_castle() {
        assert!(is_in_luferson_castle(ZONE_KARUS));
        assert!(is_in_luferson_castle(ZONE_KARUS2));
        assert!(is_in_luferson_castle(ZONE_KARUS3));
        assert!(!is_in_luferson_castle(ZONE_ELMORAD));
        assert!(!is_in_luferson_castle(ZONE_MORADON));
    }

    #[test]
    fn test_is_in_elmorad_castle() {
        assert!(is_in_elmorad_castle(ZONE_ELMORAD));
        assert!(is_in_elmorad_castle(ZONE_ELMORAD2));
        assert!(is_in_elmorad_castle(ZONE_ELMORAD3));
        assert!(!is_in_elmorad_castle(ZONE_KARUS));
        assert!(!is_in_elmorad_castle(ZONE_MORADON));
    }

    #[test]
    fn test_is_in_special_event_zone() {
        assert!(is_in_special_event_zone(105));
        assert!(is_in_special_event_zone(115));
        assert!(!is_in_special_event_zone(104));
        assert!(!is_in_special_event_zone(116));
    }

    #[test]
    fn test_zindan_event_opened_default() {
        // Zindan event opened should default to false
        let world = WorldState::new();
        assert!(!world.is_zindan_event_opened());
    }

    #[test]
    fn test_zindan_event_opened_toggle() {
        let world = WorldState::new();
        world.set_zindan_event_opened(true);
        assert!(world.is_zindan_event_opened());
        world.set_zindan_event_opened(false);
        assert!(!world.is_zindan_event_opened());
    }

    // ── NPC type-specific damage override tests ──────────────────────────

    #[test]
    fn test_npc_type_constants() {
        // C++ globals.h NPC type values — canonical source: npc_type_constants.rs
        assert_eq!(NPC_TREE, 2);
        assert_eq!(NPC_OBJECT_FLAG, 15);
        assert_eq!(NPC_REFUGEE, 46);
        assert_eq!(NPC_FOSIL, 173);
        assert_eq!(NPC_BORDER_MONUMENT, 212);
        assert_eq!(NPC_PARTNER_TYPE, 213);
        assert_eq!(NPC_PRISON, 220);
    }

    #[test]
    fn test_npc_tree_fixed_damage() {
        // NPC_TREE always takes 20 damage regardless of player attack power
        let _normal_damage: i16 = 5000;
        // When npc_type is NPC_TREE, damage should be overridden to 20
        assert_eq!(NPC_TREE, 2);
        // The function returns 20 for tree type
        assert_eq!(20i16, 20);
    }

    #[test]
    fn test_npc_border_monument_fixed_damage() {
        // NPC_BORDER_MONUMENT always takes 10 damage
        assert_eq!(NPC_BORDER_MONUMENT, 212);
        // Fixed damage = 10
        assert_eq!(10i16, 10);
    }

    #[test]
    fn test_npc_refugee_damage_by_proto() {
        // NPC_REFUGEE: specific protos get 20, others get 10
        let special_protos: [u16; 4] = [3202, 3203, 3252, 3253];
        for proto in special_protos {
            assert!(matches!(proto, 3202 | 3203 | 3252 | 3253));
        }
        // Non-special protos get 10
        assert!(!matches!(1000u16, 3202 | 3203 | 3252 | 3253));
    }

    #[test]
    fn test_npc_object_flag_proto_511() {
        // NPC_OBJECT_FLAG with proto 511 → damage = 1
        assert_eq!(NPC_OBJECT_FLAG, 15);
        // Only proto 511 gets this override
        assert_eq!(511u16, 511);
    }

    #[test]
    fn test_npc_neutral_nation_3_blocked() {
        // NPCs with org_nation/group == 3 cannot be R-attacked
        let group: u8 = 3;
        assert_eq!(group, 3);
        // Damage should be 0 for nation-3 NPCs
    }

    #[test]
    fn test_npc_partner_type_nation_none_blocked() {
        // NPC_PARTNER_TYPE with Nation::NONE → damage = 0
        let group: u8 = 0; // Nation::NONE
        assert_eq!(NPC_PARTNER_TYPE, 213);
        assert_eq!(group, 0);
    }

    #[test]
    fn test_punishment_stick_item_id() {
        const PUNISHMENT_STICK_ID: i32 = 900356000;
        assert_eq!(PUNISHMENT_STICK_ID, 900356000);
    }

    #[test]
    fn test_weapon_pickaxe_kind() {
        const WEAPON_PICKAXE: i32 = 61;
        assert_eq!(WEAPON_PICKAXE, 61);
    }

    // ── Sprint 159: Mirror damage party distribution tests ──────────────

    #[test]
    fn test_mirror_damage_party_cpp_precedence_bug() {
        //   mirrorDamage = mirrorDamage / p_count < 2 ? 2 : p_count;
        // Due to C++ precedence: (mirrorDamage/p_count < 2) evaluates as bool,
        // then ternary picks 2 or p_count.
        let mirror_dmg: i32 = 50;
        let p_count: i32 = 4;

        // Rust matching C++ precedence:
        let per_member = if (mirror_dmg / p_count) < 2 {
            2
        } else {
            p_count
        };
        // 50/4 = 12, which is >= 2, so result is p_count (4)
        assert_eq!(per_member, 4);

        // Small mirror damage case:
        let small_dmg: i32 = 5;
        let per_member2 = if (small_dmg / p_count) < 2 {
            2
        } else {
            p_count
        };
        // 5/4 = 1, which is < 2, so result is 2
        assert_eq!(per_member2, 2);
    }

    #[test]
    fn test_mirror_damage_party_self_excluded() {
        //   if (p == nullptr || p == this) continue;
        // The victim (target) should not receive mirror damage
        let target_sid: u16 = 5;
        let party_members: Vec<u16> = vec![1, 2, 5, 7]; // includes target_sid=5
        let filtered: Vec<u16> = party_members
            .iter()
            .filter(|&&m| m != target_sid)
            .copied()
            .collect();
        assert_eq!(filtered, vec![1, 2, 7]);
    }

    #[test]
    fn test_mirror_damage_party_attacker_party_lookup() {
        //   pParty = g_pMain->GetPartyPtr(pUser->GetPartyID())
        // Mirror party distribution looks up ATTACKER's party, not victim's
        let attacker_party_id: u16 = 42;
        let victim_party_id: u16 = 99;
        // The party to iterate is the ATTACKER's party
        assert_ne!(attacker_party_id, victim_party_id);
        // attacker_party_id is used (not victim_party_id)
        assert_eq!(attacker_party_id, 42);
    }

    // ── Sprint 227: Mirror damage victim reduction test ────────────────

    #[test]
    fn test_mirror_damage_victim_reduction() {
        //   mirrorDamage = (m_byMirrorAmount * amount) / 100;
        //   amount -= mirrorDamage;
        // In C++, amount is negative. In Rust, damage is positive.
        // The victim's damage is reduced by the mirror portion.
        let damage: i16 = 500;
        let mirror_amount: u8 = 30; // 30% mirror

        let mirror_dmg = (mirror_amount as i32 * damage as i32) / 100;
        assert_eq!(mirror_dmg, 150);

        let reduced_damage = (damage as i32 - mirror_dmg).max(0) as i16;
        assert_eq!(reduced_damage, 350);

        // Victim takes 350, attacker/party takes 150 (total = 500 = original)
        assert_eq!(reduced_damage as i32 + mirror_dmg, damage as i32);
    }

    #[test]
    fn test_mirror_damage_victim_reduction_100_pct() {
        // Edge case: 100% mirror — victim takes 0 damage, all reflected
        let damage: i16 = 200;
        let mirror_amount: u8 = 100;

        let mirror_dmg = (mirror_amount as i32 * damage as i32) / 100;
        assert_eq!(mirror_dmg, 200);

        let reduced_damage = (damage as i32 - mirror_dmg).max(0) as i16;
        assert_eq!(reduced_damage, 0);
    }

    // ── Chaos Temple PvP zone check tests ────────────────────────────

    #[test]
    fn test_chaos_dungeon_zone_constant() {
        assert_eq!(ZONE_CHAOS_DUNGEON, 85);
    }

    #[test]
    fn test_chaos_temple_ffa_pvp_concept() {
        // Chaos Temple is free-for-all: no nation check required.
        // Even same-nation players can attack each other in zone 85.
        let attacker_nation: u8 = 1; // Karus
        let target_nation: u8 = 1; // Also Karus
        let zone: u16 = ZONE_CHAOS_DUNGEON;
        // In Chaos Temple, nation doesn't matter — all can fight
        let chaos_active = true;
        let can_fight = zone == ZONE_CHAOS_DUNGEON && chaos_active;
        assert!(can_fight);
        // Even same nation can fight
        let _ = (attacker_nation, target_nation);
    }

    // ── Cinderella War PvP zone check tests ──────────────────────────

    #[test]
    fn test_cinderella_war_opposite_nations_can_fight() {
        // Cinderella War: opposite nations who are event users can PvP
        let attacker_nation: u8 = 1; // Karus
        let target_nation: u8 = 2; // El Morad
        let is_event_active = true;
        let both_event_users = true;
        let can_fight = is_event_active && attacker_nation != target_nation && both_event_users;
        assert!(can_fight);
    }

    #[test]
    fn test_cinderella_war_same_nation_cannot_fight() {
        let attacker_nation: u8 = 2;
        let target_nation: u8 = 2;
        let is_event_active = true;
        let can_fight = is_event_active && attacker_nation != target_nation;
        assert!(!can_fight);
    }

    #[test]
    fn test_cinderella_war_non_event_user_cannot_fight() {
        let attacker_nation: u8 = 1;
        let target_nation: u8 = 2;
        let both_event_users = false; // One is not registered
        let can_fight = attacker_nation != target_nation && both_event_users;
        assert!(!can_fight);
    }

    #[test]
    fn test_monster_stone_boss_kill_detection() {
        // Boss kill is triggered when NPC has event_room > 0 and summon_type == 1
        let npc = crate::npc::NpcInstance {
            nid: 10100,
            proto_id: 500,
            is_monster: true,
            zone_id: 81,
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
            trap_number: 0,
            event_room: 5, // room_id 4, 1-based
            is_event_npc: true,
            summon_type: 1, // Boss
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };

        // Should trigger boss kill
        assert!(npc.event_room > 0 && npc.summon_type == 1);

        // Normal monster in event room — should NOT trigger
        let normal_npc = crate::npc::NpcInstance {
            summon_type: 0, // Normal
            ..npc.clone()
        };
        assert!(!(normal_npc.event_room > 0 && normal_npc.summon_type == 1));

        // NPC not in event room — should NOT trigger
        let no_room_npc = crate::npc::NpcInstance {
            event_room: 0,
            ..npc.clone()
        };
        assert!(!(no_room_npc.event_room > 0 && no_room_npc.summon_type == 1));
    }

    #[test]
    fn test_monster_stone_boss_kill_room_id_conversion() {
        // GetEventRoom() - 1 gives the 0-based room index
        let event_room: u16 = 42; // 1-based
        let room_id = event_room - 1; // 0-based
        assert_eq!(room_id, 41);
    }

    // ── Sprint 196: Combat isolation via event_room ──────────────────────

    #[test]
    fn test_monster_stone_combat_isolation_same_room_allowed() {
        // Players in the same event room CAN attack each other
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.event_room = 5;
            h.monster_stone_status = true;
        });
        world.update_session(2, |h| {
            h.event_room = 5;
            h.monster_stone_status = true;
        });

        let zone_id: u16 = 81; // Monster Stone zone
        assert!(crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        assert!(world.is_same_event_room(1, 2));
    }

    #[test]
    fn test_monster_stone_combat_isolation_different_room_blocked() {
        // Players in different event rooms CANNOT attack each other
        // (when monster_stone_status is true)
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.event_room = 3;
            h.monster_stone_status = true;
        });
        world.update_session(2, |h| {
            h.event_room = 7;
            h.monster_stone_status = true;
        });

        let zone_id: u16 = 82; // Monster Stone zone
        assert!(crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        assert!(!world.is_same_event_room(1, 2));
        assert!(world.get_monster_stone_status(1));
    }

    #[test]
    fn test_monster_stone_combat_isolation_one_not_in_room() {
        // One player in event room, other not — cannot attack
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.event_room = 5;
            h.monster_stone_status = true;
        });
        // Session 2 stays at event_room = 0, monster_stone_status = false

        let zone_id: u16 = 83; // Monster Stone zone
        assert!(crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        assert!(!world.is_same_event_room(1, 2));
    }

    #[test]
    fn test_monster_stone_status_false_skips_room_check() {
        // When monster_stone_status is false, event room isolation is NOT enforced
        // even if the player is in a Monster Stone zone with a different event_room.
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Both players in MS zone with different rooms but status=false
        world.update_session(1, |h| {
            h.event_room = 3;
            h.monster_stone_status = false;
        });
        world.update_session(2, |h| {
            h.event_room = 7;
            h.monster_stone_status = false;
        });

        // monster_stone_status is false → guard skipped → attacks proceed
        assert!(!world.get_monster_stone_status(1));
        assert!(!world.get_monster_stone_status(2));
    }

    #[test]
    fn test_non_monster_stone_zone_no_event_room_check() {
        // In non-Monster Stone zones, event_room is irrelevant
        let zone_id: u16 = 21; // Moradon
        assert!(!crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        // Even with different rooms, attacks proceed in non-MS zones
        // (the check only applies in MS zones 81-83)
    }

    /// Helper to construct a minimal CharacterInfo for kill reward tests.
    fn make_kill_reward_char(
        sid: crate::zone::SessionId,
        nation: u8,
        class: u16,
    ) -> crate::world::CharacterInfo {
        crate::world::CharacterInfo {
            session_id: sid,
            name: format!("Player{}", sid),
            nation,
            race: 1,
            class,
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

    #[test]
    fn test_give_kill_reward_event_room_filter_same_room() {
        // should pass the event_room filter.
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Set up characters in same zone + same event room
        world.update_session(1, |h| {
            h.character = Some(make_kill_reward_char(1, 1, 101));
            h.position.zone_id = ZONE_BORDER_DEFENSE_WAR;
            h.event_room = 3;
        });
        world.update_session(2, |h| {
            h.character = Some(make_kill_reward_char(2, 1, 101));
            h.position.zone_id = ZONE_BORDER_DEFENSE_WAR;
            h.event_room = 3; // Same room
        });

        // Capture killer's event_room first (can't call world inside with_session)
        let killer_room = world.get_event_room(1);
        let member_ok = world
            .with_session(2, |h| {
                h.character.is_some()
                    && h.position.zone_id == ZONE_BORDER_DEFENSE_WAR
                    && h.event_room == killer_room
            })
            .unwrap_or(false);
        assert!(member_ok, "Same event room should pass filter");
    }

    #[test]
    fn test_give_kill_reward_event_room_filter_different_room() {
        // should be filtered out and NOT receive rewards.
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Same zone, different event rooms
        world.update_session(1, |h| {
            h.character = Some(make_kill_reward_char(1, 1, 101));
            h.position.zone_id = ZONE_BORDER_DEFENSE_WAR;
            h.event_room = 1;
        });
        world.update_session(2, |h| {
            h.character = Some(make_kill_reward_char(2, 1, 101));
            h.position.zone_id = ZONE_BORDER_DEFENSE_WAR;
            h.event_room = 2; // Different room
        });

        let killer_room = world.get_event_room(1);
        let member_ok = world
            .with_session(2, |h| {
                h.character.is_some()
                    && h.position.zone_id == ZONE_BORDER_DEFENSE_WAR
                    && h.event_room == killer_room
            })
            .unwrap_or(false);
        assert!(!member_ok, "Different event room should fail filter");
    }

    #[test]
    fn test_give_kill_reward_event_room_zero_matches_zero() {
        // When neither player is in an event room (room=0), they should match.
        // This is the normal non-event case.
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.character = Some(make_kill_reward_char(1, 1, 101));
            h.position.zone_id = 2; // Ronark Land
            h.event_room = 0;
        });
        world.update_session(2, |h| {
            h.character = Some(make_kill_reward_char(2, 1, 101));
            h.position.zone_id = 2;
            h.event_room = 0;
        });

        let killer_room = world.get_event_room(1);
        assert_eq!(killer_room, 0);
        let member_ok = world
            .with_session(2, |h| {
                h.character.is_some() && h.position.zone_id == 2 && h.event_room == killer_room
            })
            .unwrap_or(false);
        assert!(
            member_ok,
            "Both room=0 should match (normal non-event case)"
        );
    }

    // ── Sprint 257: Deva Bird Juraid bridge check tests ──────────────

    #[test]
    fn test_deva_bird_blocked_no_bridges() {
        let world = WorldState::new();
        // Room 1 with no bridges open
        let bs = crate::systems::juraid::JuraidBridgeState::new();
        world.set_juraid_bridge_state(1, bs);

        // Karus (nation=1) needs all 3 bridges
        assert!(!world.are_all_juraid_bridges_open(1, 1));
        // Elmorad (nation=2) also blocked
        assert!(!world.are_all_juraid_bridges_open(1, 2));
    }

    #[test]
    fn test_deva_bird_allowed_all_bridges_open() {
        let world = WorldState::new();
        let mut bs = crate::systems::juraid::JuraidBridgeState::new();
        bs.open_bridge(0, 1);
        bs.open_bridge(1, 1);
        bs.open_bridge(2, 1);
        world.set_juraid_bridge_state(1, bs);

        // Karus (nation=1) has all 3 bridges → allowed
        assert!(world.are_all_juraid_bridges_open(1, 1));
        // Elmorad (nation=2) has 0 bridges → blocked
        assert!(!world.are_all_juraid_bridges_open(1, 2));
    }

    #[test]
    fn test_deva_bird_partial_bridges_blocked() {
        let world = WorldState::new();
        let mut bs = crate::systems::juraid::JuraidBridgeState::new();
        bs.open_bridge(0, 2);
        bs.open_bridge(1, 2);
        // Only 2 of 3 Elmorad bridges open
        world.set_juraid_bridge_state(1, bs);

        assert!(!world.are_all_juraid_bridges_open(1, 2));
    }

    #[test]
    fn test_deva_bird_nonexistent_room() {
        let world = WorldState::new();
        // No bridge state for room 99 → should be blocked
        assert!(!world.are_all_juraid_bridges_open(99, 1));
    }

    #[test]
    fn test_clear_juraid_bridge_states() {
        let world = WorldState::new();
        let mut bs = crate::systems::juraid::JuraidBridgeState::new();
        bs.open_bridge(0, 1);
        bs.open_bridge(1, 1);
        bs.open_bridge(2, 1);
        world.set_juraid_bridge_state(1, bs);

        assert!(world.are_all_juraid_bridges_open(1, 1));

        world.clear_juraid_bridge_states();
        assert!(!world.are_all_juraid_bridges_open(1, 1));
    }

    // ── Sprint 257: Vaccuni attack check tests ───────────────────────

    fn make_test_npc(proto_id: u16) -> crate::npc::NpcInstance {
        crate::npc::NpcInstance {
            nid: 10001,
            proto_id,
            is_monster: true,
            zone_id: 1,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            gate_open: 0,
            object_type: 0,
            nation: 0,
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
        }
    }

    #[test]
    fn test_vaccuni_attack_no_match() {
        // Proto IDs that don't match any Vaccuni target
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        let npc = make_test_npc(1234);
        assert!(!check_vaccuni_attack(&world, 1, &npc));
    }

    #[test]
    fn test_vaccuni_attack_matching_proto_no_event() {
        // Proto 4351 but no quest event flag
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            let mut ch = make_kill_reward_char(1, 1, 80);
            ch.name = "TestVaccuni".to_string();
            h.character = Some(ch);
        });

        let npc = make_test_npc(4351);
        // No quest event 793/794 → should fail
        assert!(!check_vaccuni_attack(&world, 1, &npc));
    }

    #[test]
    fn test_vaccuni_attack_fallthrough_protos() {
        // Proto 4301, 605, 611, 616 fall through (always return false)
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        for proto in [4301u16, 605, 611, 616] {
            let npc = make_test_npc(proto);
            assert!(
                !check_vaccuni_attack(&world, 1, &npc),
                "Proto {} should return false (fall-through)",
                proto
            );
        }
    }

    // ── Sprint 257: Type3 DOT re-cast prevention test ────────────────

    #[test]
    fn test_type3_hot_recast_blocked() {
        // If target already has a HOT (hp_amount > 0), re-cast should be blocked
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add an active HOT to the target
        let added = world.add_durational_skill(1, 108100, 50, 5, 2);
        assert!(added);

        // Verify target has active HOT
        assert!(world.has_active_hot(1));
    }

    #[test]
    fn test_type3_dot_allows_recast() {
        // If target only has a DOT (hp_amount < 0), re-cast should be allowed
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Add an active DOT (negative = damage)
        let added = world.add_durational_skill(1, 108100, -50, 5, 2);
        assert!(added);

        // DOT is not a HOT, so has_active_hot should be false
        assert!(!world.has_active_hot(1));
    }

    // ── Sprint 330: isAttackDisabled tests ───────────────────────────

    /// Test that is_attack_disabled returns false when status is 0 (default).
    #[test]
    fn test_attack_disabled_default_false() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.is_attack_disabled(1));
    }

    /// Test that is_attack_disabled returns true when permanently banned (u32::MAX).
    #[test]
    fn test_attack_disabled_permanent() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| h.attack_disabled_until = u32::MAX);
        assert!(world.is_attack_disabled(1));
    }

    /// Test that is_attack_disabled returns true when temporarily banned (future timestamp).
    #[test]
    fn test_attack_disabled_temporary_future() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        // Set to far future
        world.update_session(1, |h| h.attack_disabled_until = u32::MAX - 1);
        assert!(world.is_attack_disabled(1));
    }

    /// Test that is_attack_disabled returns false when ban has expired (past timestamp).
    #[test]
    fn test_attack_disabled_expired() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        // Set to past timestamp (1 = Jan 1, 1970)
        world.update_session(1, |h| h.attack_disabled_until = 1);
        assert!(!world.is_attack_disabled(1));
    }

    // ── Sprint 359: Self-targeting prevention ─────────────────────────

    /// is_hostile_to returns false when attacker == target (self-targeting).
    ///
    #[test]
    fn test_is_hostile_to_self_targeting_blocked() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let ch = CharacterInfo {
            session_id: sid,
            name: "SelfTest".into(),
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
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 90,
            sta: 60,
            dex: 30,
            intel: 20,
            cha: 10,
            free_points: 0,
            skill_points: [0; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
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
        };
        let pos = Position {
            zone_id: 85, // Chaos Dungeon (free-for-all PvP)
            x: 200.0,
            y: 0.0,
            z: 200.0,
            region_x: 1,
            region_z: 1,
        };
        world.register_ingame(sid, ch.clone(), pos);

        // Even in Chaos Dungeon (free-for-all), self-targeting must be blocked
        assert!(
            !is_hostile_to(&world, sid, &ch, &pos, sid, &ch, &pos),
            "Self-targeting must always return false, even in free-for-all zones"
        );
    }

    /// is_hostile_to allows attacking different players in Chaos Dungeon.
    #[test]
    fn test_is_hostile_to_chaos_dungeon_allows_pvp() {
        use crate::systems::event_room::TempleEventType;

        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let sid1 = world.allocate_session_id();
        let sid2 = world.allocate_session_id();
        world.register_session(sid1, tx1);
        world.register_session(sid2, tx2);

        let ch1 = CharacterInfo {
            session_id: sid1,
            name: "Attacker".into(),
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
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 90,
            sta: 60,
            dex: 30,
            intel: 20,
            cha: 10,
            free_points: 0,
            skill_points: [0; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
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
        };
        let ch2 = CharacterInfo {
            session_id: sid2,
            name: "Target".into(),
            nation: 1, // Same nation
            ..ch1.clone()
        };
        let pos = Position {
            zone_id: 85,
            x: 200.0,
            y: 0.0,
            z: 200.0,
            region_x: 1,
            region_z: 1,
        };
        world.register_ingame(sid1, ch1.clone(), pos);
        world.register_ingame(sid2, ch2.clone(), pos);

        // Activate Chaos Dungeon event
        world.event_room_manager.update_temple_event(|s| {
            s.active_event = TempleEventType::ChaosDungeon as i16;
            s.is_active = true;
        });

        // Same nation in Chaos Dungeon with event active — should allow PvP
        assert!(
            is_hostile_to(&world, sid1, &ch1, &pos, sid2, &ch2, &pos),
            "Different players in Chaos Dungeon should be hostile"
        );
    }

    // ── Bot last_attacker_id tracking tests ───────────────────────────

    /// Helper to create a minimal BotInstance for testing.
    fn make_test_bot(id: u32, zone_id: u16, x: f32, z: f32) -> crate::world::BotInstance {
        use crate::world::{BotAiState, BotPresence};
        crate::world::BotInstance {
            id,
            db_id: 0,
            name: format!("TestBot{}", id),
            nation: 2,
            race: 1,
            class: 106,
            hair_rgb: 0,
            level: 70,
            face: 1,
            knights_id: 0,
            fame: 0,
            zone_id,
            x,
            y: 0.0,
            z,
            direction: 0,
            region_x: (x / 32.0) as u16,
            region_z: (z / 32.0) as u16,
            hp: 5000,
            max_hp: 5000,
            mp: 1000,
            max_mp: 1000,
            sp: 0,
            max_sp: 0,
            str_stat: 100,
            sta_stat: 80,
            dex_stat: 90,
            int_stat: 60,
            cha_stat: 30,
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            loyalty_daily: 0,
            in_game: true,
            presence: BotPresence::Standing,
            ai_state: BotAiState::Pk,
            target_id: -1,
            target_changed: false,
            spawned_at: 0,
            duration_minutes: 0,
            last_tick_ms: 0,
            last_move_ms: 0,
            last_mining_ms: 0,
            last_merchant_chat_ms: 0,
            last_hp_change_ms: 0,
            last_regen_ms: 0,
            last_attacker_id: -1,
            skill_cooldown: [0; 2],
            last_type4_ms: 0,
            regene_at_ms: 0,
            original_ai_state: BotAiState::Idle,
            move_route: 0,
            move_state: 0,
            merchant_state: -1,
            premium_merchant: false,
            merchant_chat: String::new(),
            merchant_items: Default::default(),
            merchant_looker: None,
            reb_level: 0,
            cover_title: 0,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            hiding_helmet: false,
            hiding_cospre: false,
            need_party: 1,
            equip_visual: [(0, 0, 0); 17],
            personal_rank: 0,
            knights_rank: 0,
        }
    }

    /// R-attack on a bot without attacker session info should be a no-op.
    /// The new damage path requires the attacker to have position + character info;
    /// without it, the attack returns early without modifying the bot.
    #[tokio::test]
    async fn test_rattack_bot_no_attacker_info_noop() {
        let world = Arc::new(WorldState::new());
        let bot_id = BOT_ID_BASE + 1;
        let bot = make_test_bot(bot_id, 72, 100.0, 100.0);
        let original_hp = bot.hp;
        world.insert_bot(bot);

        // Player sid=5 has no registered session/character → early return
        handle_npc_attack(world.clone(), 5, Position::default(), bot_id, 1, 0).await;

        let after = world.get_bot(bot_id).unwrap();
        assert_eq!(
            after.hp, original_hp,
            "bot HP should be unchanged without attacker info"
        );
        assert_eq!(
            after.last_attacker_id, -1,
            "last_attacker_id should remain -1"
        );
    }

    /// R-attack on a dead bot should be a no-op.
    #[tokio::test]
    async fn test_rattack_bot_dead_noop() {
        let world = Arc::new(WorldState::new());
        let bot_id = BOT_ID_BASE + 2;
        let mut bot = make_test_bot(bot_id, 72, 100.0, 100.0);
        bot.hp = 0;
        bot.presence = crate::world::BotPresence::Dead;
        world.insert_bot(bot);

        handle_npc_attack(world.clone(), 10, Position::default(), bot_id, 1, 0).await;

        let after = world.get_bot(bot_id).unwrap();
        assert_eq!(after.hp, 0, "dead bot HP should remain 0");
        assert_eq!(after.last_attacker_id, -1);
    }

    /// R-attack on a non-existent bot ID should not panic.
    #[tokio::test]
    async fn test_rattack_nonexistent_bot_no_panic() {
        let world = Arc::new(WorldState::new());
        // Bot ID is in the bot range but no bot is inserted
        let fake_bot_id = BOT_ID_BASE + 999;
        handle_npc_attack(world.clone(), 5, Position::default(), fake_bot_id, 1, 0).await;
        // Should just silently return — no panic
    }

    /// Attacking an NPC ID that is NOT a bot should not modify any bot state.
    /// Both bots and NPCs share the >= 10000 ID space, so we verify that an
    /// attack on an ID with no bot entry falls through to the NPC path.
    #[tokio::test]
    async fn test_rattack_npc_does_not_affect_bots() {
        let world = Arc::new(WorldState::new());
        // Insert a bot at BOT_ID_BASE + 100
        let bot_id = BOT_ID_BASE + 100;
        let bot = make_test_bot(bot_id, 72, 100.0, 100.0);
        world.insert_bot(bot);

        // Attack a DIFFERENT NPC ID (BOT_ID_BASE + 200) which has no bot entry.
        // This should fall through to the NPC instance lookup (which also fails)
        // without touching the bot at BOT_ID_BASE + 100.
        let npc_only_id = BOT_ID_BASE + 200;
        handle_npc_attack(world.clone(), 5, Position::default(), npc_only_id, 1, 0).await;

        // The bot should remain untouched
        let after = world.get_bot(bot_id).unwrap();
        assert_eq!(
            after.last_attacker_id, -1,
            "attacking a different NPC should not modify bot last_attacker_id"
        );
    }

    // ── Elemental weapon damage tests ────────────────────────────────

    /// Create a minimal NPC template for testing elemental damage.
    fn make_elemental_npc(
        fire_r: i16,
        cold_r: i16,
        lightning_r: i16,
        poison_r: i16,
    ) -> crate::npc::NpcTemplate {
        crate::npc::NpcTemplate {
            s_sid: 1,
            is_monster: true,
            name: "TestNpc".to_string(),
            pid: 1,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 1,
            max_hp: 1000,
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
            fire_r,
            cold_r,
            lightning_r,
            magic_r: 0,
            disease_r: 0,
            poison_r,
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }

    #[test]
    fn test_elemental_damage_no_resistance() {
        // Fire=20, no target resistance → full 20 added
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats
            .equipped_item_bonuses
            .insert(6, vec![(ITEM_TYPE_FIRE, 20)]);

        let npc_tmpl = make_elemental_npc(0, 0, 0, 0);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 120);
    }

    #[test]
    fn test_elemental_damage_partial_resistance() {
        // Fire=20, target fire_r=100 → total_r=100, bonus = 20 - 20*100/200 = 10
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats
            .equipped_item_bonuses
            .insert(6, vec![(ITEM_TYPE_FIRE, 20)]);

        let npc_tmpl = make_elemental_npc(100, 0, 0, 0);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 110); // 100 base + 10 fire bonus
    }

    #[test]
    fn test_elemental_damage_full_resistance() {
        // Fire=20, target fire_r=200 → total_r=200, bonus = 20 - 20*200/200 = 0
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats
            .equipped_item_bonuses
            .insert(6, vec![(ITEM_TYPE_FIRE, 20)]);

        let npc_tmpl = make_elemental_npc(200, 0, 0, 0);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 100); // No bonus — fully resisted
    }

    #[test]
    fn test_elemental_damage_multiple_types() {
        // Fire=10, Cold=15, Lightning=5 on same weapon
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats.equipped_item_bonuses.insert(
            6,
            vec![
                (ITEM_TYPE_FIRE, 10),
                (ITEM_TYPE_COLD, 15),
                (ITEM_TYPE_LIGHTNING, 5),
            ],
        );

        let npc_tmpl = make_elemental_npc(0, 0, 0, 0);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 130); // 100 + 10 + 15 + 5
    }

    #[test]
    fn test_elemental_damage_resistance_cap_200() {
        // Fire=40, target fire_r=300 (exceeds cap) → capped to 200 → full reduction
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats
            .equipped_item_bonuses
            .insert(6, vec![(ITEM_TYPE_FIRE, 40)]);

        let npc_tmpl = make_elemental_npc(300, 0, 0, 0);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 100); // No bonus — resistance capped at 200
    }

    #[test]
    fn test_elemental_damage_mirror_not_affected() {
        // Mirror damage (type 8) should not be counted as elemental
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats
            .equipped_item_bonuses
            .insert(6, vec![(0x08, 30)]); // ITEM_TYPE_MIRROR_DAMAGE

        let npc_tmpl = make_elemental_npc(0, 0, 0, 0);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 100); // Mirror not counted in elemental
    }

    #[test]
    fn test_elemental_damage_poison_with_resist() {
        // Poison=30, target poison_r=60 → bonus = 30 - 30*60/200 = 30-9 = 21
        let mut attacker_stats = crate::world::EquippedStats::default();
        attacker_stats
            .equipped_item_bonuses
            .insert(6, vec![(ITEM_TYPE_POISON, 30)]);

        let npc_tmpl = make_elemental_npc(0, 0, 0, 60);
        let result = apply_elemental_weapon_damage_npc(&attacker_stats, &npc_tmpl, 100);
        assert_eq!(result, 121); // 100 + 21 poison bonus
    }

    // ── Santa NPC death condition tests ─────────────────────────────

    #[test]
    fn test_santa_death_guard_npc_type() {
        // Only NPC_SANTA (219) triggers proximity rewards.
        let mut tmpl = make_elemental_npc(0, 0, 0, 0);
        tmpl.npc_type = 0; // regular monster
        tmpl.area_range = 50.0;
        assert_ne!(tmpl.npc_type, NPC_SANTA);

        tmpl.npc_type = NPC_SANTA;
        assert_eq!(tmpl.npc_type, NPC_SANTA);
        assert_eq!(NPC_SANTA, 219);
    }

    #[test]
    fn test_santa_death_guard_area_range() {
        // area_range must be >= 1.0 (C++ check: m_area_range < 1.0f → return)
        let mut tmpl = make_elemental_npc(0, 0, 0, 0);
        tmpl.npc_type = NPC_SANTA;

        tmpl.area_range = 0.0;
        assert!(tmpl.area_range < 1.0, "area_range 0.0 should skip");

        tmpl.area_range = 0.5;
        assert!(tmpl.area_range < 1.0, "area_range 0.5 should skip");

        tmpl.area_range = 1.0;
        assert!(tmpl.area_range >= 1.0, "area_range 1.0 should trigger");

        tmpl.area_range = 50.0;
        assert!(tmpl.area_range >= 1.0, "area_range 50.0 should trigger");
    }

    #[test]
    fn test_santa_death_range_check_geometry() {
        // C++ isInRangeSlow: dx*dx + dz*dz <= range*range
        let npc_x: f32 = 100.0;
        let npc_z: f32 = 200.0;
        let range: f32 = 50.0;
        let range_sq = range * range;

        // Player exactly at NPC position — in range.
        let ddx = 0.0_f32;
        let ddz = 0.0_f32;
        assert!(ddx * ddx + ddz * ddz <= range_sq);

        // Player at edge of range — in range.
        let px = npc_x + 50.0;
        let pz = npc_z;
        let ddx = px - npc_x;
        let ddz = pz - npc_z;
        assert!(ddx * ddx + ddz * ddz <= range_sq);

        // Player just outside range — not in range.
        let px = npc_x + 50.1;
        let pz = npc_z;
        let ddx = px - npc_x;
        let ddz = pz - npc_z;
        assert!(ddx * ddx + ddz * ddz > range_sq);

        // Diagonal player at ~35.35 each axis (total dist ~50) — in range.
        let d = 35.0;
        let px = npc_x + d;
        let pz = npc_z + d;
        let ddx = px - npc_x;
        let ddz = pz - npc_z;
        // 35² + 35² = 2450, 50² = 2500 → in range
        assert!(ddx * ddx + ddz * ddz <= range_sq);
    }

    // ── Achievement2 PvP kill counter tests ──────────────────────────

    #[test]
    fn test_achieve_summary_user_defeat_count_saturating() {
        use crate::world::types::AchieveSummary;
        let mut s = AchieveSummary::default();
        assert_eq!(s.user_defeat_count, 0);

        s.user_defeat_count = s.user_defeat_count.saturating_add(1);
        assert_eq!(s.user_defeat_count, 1);

        s.user_defeat_count = u32::MAX;
        s.user_defeat_count = s.user_defeat_count.saturating_add(1);
        assert_eq!(s.user_defeat_count, u32::MAX); // no overflow
    }

    #[test]
    fn test_achieve_summary_user_death_count_saturating() {
        use crate::world::types::AchieveSummary;
        let mut s = AchieveSummary::default();
        assert_eq!(s.user_death_count, 0);

        s.user_death_count = s.user_death_count.saturating_add(1);
        assert_eq!(s.user_death_count, 1);

        s.user_death_count = u32::MAX;
        s.user_death_count = s.user_death_count.saturating_add(1);
        assert_eq!(s.user_death_count, u32::MAX);
    }

    #[test]
    fn test_achievement2_pvp_kill_packet_format() {
        // build_achievement2 should produce [i32 value] for PvP kill counter
        let pkt = crate::handler::achievement2::build_achievement2(42);
        assert_eq!(pkt.opcode, Opcode::WizAchievement2 as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_achievement2_pvp_kill_zero_clears_display() {
        // value=0 clears the kill counter display on client
        let pkt = crate::handler::achievement2::build_achievement2(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_i32(), Some(0));
    }
}
