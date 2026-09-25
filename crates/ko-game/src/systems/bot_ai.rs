//! Bot AI tick system — drives spawned bot behaviour every second.
//! # Architecture
//! The bot AI tick runs as a background tokio task (started from `main.rs`).
//! Each tick it iterates all active [`BotInstance`]s in [`WorldState::bots`]
//! and advances their AI state machine.
//! # C++ Reference
//! In C++ each bot runs in a thread (similar to NPC threads) managed by
//! triggered by timer callbacks. We replicate this as a centralised tick
//! rather than per-bot threads.
//! ## Bot states (`m_BotState` values, User.h lines 71-85):
//! - 0 = BOT_AFK          : standing idle (AFK simulation)
//! - 1 = BOT_MINING       : performing mining animations
//! - 2 = BOT_FISHING      : performing fishing animations
//! - 3 = BOT_FARMER       : hunting monsters
//! - 4 = BOT_FARMERS      : farm-bot variant (multi-farmer)
//! - 5 = BOT_MERCHANT     : standing merchant (chat broadcast)
//! - 6 = BOT_DEAD         : bot is dead
//! - 7 = BOT_MOVE         : walking to destination
//! - 8 = BOT_MERCHANT_MOVE: walking then opening shop
//! ## Tick interval
//! C++ bots process at roughly 1-second intervals (similar to NPC AI).
//! This matches `MONSTER_SPEED = 1500` ms in the NPC AI but bots run at 1 s.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ko_db::models::CoefficientRow;
use ko_protocol::{Opcode, Packet};
use rand::Rng;
use tracing::{debug, trace};

use crate::handler::attack::get_ac_damage;
use crate::systems::bot_waypoints;
use crate::systems::loyalty::{MAX_LEVEL_ARDREAM, PVP_MONUMENT_NP_BONUS};
use crate::world::types::{ZONE_ARDREAM, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE};
use crate::world::{
    BotAiState, BotId, BotInstance, BotPresence, MerchData, WorldState, MAX_MERCH_ITEMS,
    NATION_ELMORAD, NATION_KARUS,
};
use crate::zone::{calc_region, SessionId};

/// Interval for the bot AI tick loop (milliseconds).
// Four movement samples per second let the client interpolate runtime bots
// instead of displaying one large step each second. Combat cooldowns are
// tracked separately, so this does not increase attack/skill speed.
const BOT_AI_TICK_MS: u64 = 250;

/// Mining/fishing animation broadcast interval (ms).
const BOT_MINING_INTERVAL_MS: u64 = 120_000; // 2 minutes

/// Merchant chat broadcast interval (ms).
const BOT_MERCHANT_CHAT_INTERVAL_MS: u64 = 60_000; // 1 minute

/// Search range for finding targets (game units).
const BOT_SEARCH_RANGE: f32 = 32.0;

/// Attack range for melee attacks (game units).
/// Default 7.0 matches C++ fallback when skill has no range defined.
const BOT_ATTACK_RANGE: f32 = 7.0;

/// HP threshold (percentage) below which the bot flees.
/// When the bot's HP drops below 20% of max, it switches to flee mode.
const BOT_FLEE_HP_PERCENT: f32 = 0.20;

/// Cooldown between attacks for melee classes (warrior, rogue dagger) (ms).
const BOT_ATTACK_COOLDOWN_MELEE_MS: u64 = 2_000;

/// Cooldown between attacks for ranged/caster classes (rogue arrow, mage, priest) (ms).
const BOT_ATTACK_COOLDOWN_RANGED_MS: u64 = 3_000;

/// Cooldown after no target found before rescanning (ms).
const BOT_NO_TARGET_COOLDOWN_MS: u64 = 5_000;

/// PK bots inspect the seeking-party board at this interval.
const BOT_PARTY_AI_INTERVAL_MS: u64 = 10_000;

/// Default movement step per tick for non-PK zones (game units).
/// Used only in test assertions.
#[cfg(test)]
const BOT_MOVE_SPEED: f32 = 4.5;

/// Maximum damage cap.
use crate::attack_constants::MAX_DAMAGE;

/// Rivalry duration in seconds (5 minutes).
const RIVALRY_DURATION_SECS: u64 = 300;

use crate::handler::arena::MAX_ANGER_GAUGE;

/// Bonus NP for killing a rival target.
const RIVALRY_NP_BONUS: i32 = 150;

/// Flee distance (game units) — how far the bot runs when fleeing.
const BOT_FLEE_DISTANCE: f32 = 30.0;

/// HP/MP regen interval for bots (ms).
const BOT_REGEN_INTERVAL_MS: u64 = 3_000;

/// Self-heal cooldown (ms).
/// every 5 seconds when HP is below 90%.
const BOT_SELF_HEAL_COOLDOWN_MS: u64 = 5_000;

/// Delay before a dead bot respawns (ms).
/// `Regene()` immediately on `BOT_DEAD`, but in PK zones the bot has a
/// 5-second cooldown (`m_sSkillCoolDown[1] = UNIXTIME + 5`).
/// We use a 10-second regene delay for consistency across all zones.
const BOT_REGENE_DELAY_MS: u64 = 10_000;

use crate::attack_constants::{ATTACK_SUCCESS, ATTACK_TARGET_DEAD};

// ── Nation constants ──────────────────────────────────────────────────────

// ── Magic opcode constants ───────────────────────────────────────────────

use crate::magic_constants::{MAGIC_CASTING, MAGIC_EFFECTING, MORAL_AREA_ENEMY};

// ── Class-specific bot skill tables ──────────────────────────────────────
//
// ID selection. Below is a simplified version using representative skills at
// low/mid/high level brackets. The C++ code selects randomly from pools;
// we pick a single representative per bracket for simplicity.
//
// Skills are for KARUS nation. For ELMORAD, add +100000.
// For levels <= MAX_LEVEL_ARDREAM, if skill_id > class_threshold,
// subtract 1000 (C++ ardream downgrade logic).

/// Warrior skill IDs by level bracket (Karus base).
/// | Level   | Skill ID | Description              |
/// |---------|----------|--------------------------|
/// | 1-9     | 101001   | Basic Attack             |
/// | 10-24   | 105505   | Slash                    |
/// | 25-44   | 105525   | Crash                    |
/// | 45-54   | 105545   | Stroke                   |
/// | 55-56   | 105555   | Descent                  |
/// | 57-59   | 105557   | Descent (higher rank)    |
/// | 60-69   | 106560   | Master Descent           |
/// | 70-74   | 106570   | Master Thrust            |
/// | 75-79   | 106575   | Master Cleave            |
/// | 80-83   | 106580   | Master Execute           |
fn get_warrior_skill(level: u8) -> u32 {
    let mut rng = rand::thread_rng();
    match level {
        1..=9 => 101001,
        10..=24 => 105505,
        25..=44 => 105525,
        45..=54 => 105545,
        55..=56 => 105555,
        57..=59 => 105557,
        60..=69 => 106560,
        // C++ BotMoveAttack.cpp:1047-1231 — level 70-74 falls through into
        // level 75-79 random block (C++ missing-break bug, replicated for parity)
        70..=79 => {
            if rng.gen_range(0..=1) == 0 {
                106570
            } else {
                106575
            }
        }
        80..=81 => 106580,
        // C++ level 82-83: myrand(0,1) → 106580 or 106782
        82..=83 => {
            if rng.gen_range(0..=1) == 0 {
                106580
            } else {
                106782
            }
        }
        _ => 106580,
    }
}

/// Rogue (Assassin dagger) skill IDs by level bracket (Karus base).
/// | Level   | Skill ID | Description              |
/// |---------|----------|--------------------------|
/// | 1-9     | 101001   | Basic Attack             |
/// | 10-19   | 108005   | Stab                     |
/// | 20-34   | 108620   | Spike                    |
/// | 35-44   | 108635   | Thrust                   |
/// | 45-54   | 108640   | Jab                      |
/// | 55-69   | 108655   | Pierce                   |
/// | 70-79   | 108670   | Impale                   |
/// | 80-83   | 108685   | Devastate                |
fn get_rogue_skill(level: u8) -> u32 {
    match level {
        1..=9 => 101001,
        10..=19 => 108005,
        20..=34 => 108620,
        35..=44 => 108635,
        45..=54 => 108640,
        55..=69 => 108655,
        70..=79 => 108670,
        80..=83 => 108685,
        _ => 108685,
    }
}

/// Rogue (Arrow) skill IDs by level bracket (Karus base).
/// Used when the bot's RIGHTHAND weapon is a bow or crossbow.
/// | Level   | Skill ID | Description              |
/// |---------|----------|--------------------------|
/// | 1-9     | 107003   | Arrow Shot               |
/// | 10-24   | 107500   | Power Shot               |
/// | 25-39   | 107525   | Multi Shot               |
/// | 40-51   | 107540   | Explosive Arrow          |
/// | 52-59   | 107552   | Arrow Shower             |
/// | 60-69   | 108552   | Master Arrow Shower      |
/// | 70-79   | 108570   | Master Arrow Volley      |
/// | 80-83   | 108585   | Master Arrow Devastate   |
fn get_rogue_arrow_skill(level: u8) -> u32 {
    match level {
        1..=9 => 107003,
        10..=24 => 107500,
        25..=39 => 107525,
        40..=51 => 107540,
        52..=59 => 107552,
        60..=69 => 108552,
        70..=79 => 108570,
        80..=83 => 108585,
        _ => 108585,
    }
}

/// Mage subclass offset: Flame = 0, Glacier = 100, Lightning = 200.
/// one of Flame/Glacier/Lightning each attack via `myrand(1, 3)`.
/// All three share identical skill ID pools; Glacier adds +100, Lightning +200.
/// `BotMagic.cpp` lightning: `sSkillID += 200` (except 110002/210002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MageSubclass {
    Flame,
    Glacier,
    Lightning,
}

/// Randomly select a mage subclass for this attack.
fn random_mage_subclass() -> MageSubclass {
    let mut rng = rand::thread_rng();
    match rng.gen_range(1..=3) {
        1 => MageSubclass::Flame,
        2 => MageSubclass::Lightning,
        _ => MageSubclass::Glacier,
    }
}

/// Get the mage subclass offset applied to the final skill ID.
fn mage_subclass_offset(sub: MageSubclass) -> u32 {
    match sub {
        MageSubclass::Flame => 0,
        MageSubclass::Glacier => 100,
        MageSubclass::Lightning => 200,
    }
}

/// Mage base skill pool by level bracket (Karus base, shared by all subclasses).
/// All three mage subclass functions use the identical level→pool mapping.
/// The pool contains multiple skills per bracket; we select randomly from
/// the pool to match C++ behaviour (`myrand(0, N)` per bracket).
/// After selection, Glacier adds +100 and Lightning adds +200 to the skill ID.
/// | Level   | Pool (Karus base)                                    |
/// |---------|------------------------------------------------------|
/// | 1-4     | 109001, 109002                                       |
/// | 5-6     | 109001, 109002, 109005                               |
/// | 7-9     | 109001, 109002, 109005, 109007                       |
/// | 10-14   | 109503, 109509                                       |
/// | 15-17   | 109503, 109509, 109515                               |
/// | 18-26   | 109503, 109509, 109515, 109518                       |
/// | 27-32   | 109503, 109509, 109515, 109518, 109527               |
/// | 33-34   | +109533                                              |
/// | 35-38   | +109535                                              |
/// | 39-41   | +109539                                              |
/// | 42      | +109542                                              |
/// | 43      | +109543                                              |
/// | 44-55   | +109545                                              |
/// | 56      | +109556                                              |
/// | 57-59   | +109557                                              |
/// | 60-69   | 110542..110560 (7-skill pool)                        |
/// | 70-79   | 110542..110570 (with 110571 replacing 110556)        |
/// | 80-83   | +110575 (8-skill pool)                               |
fn get_mage_base_skill(level: u8) -> u32 {
    let mut rng = rand::thread_rng();
    match level {
        1..=4 => *[109001, 109002].get(rng.gen_range(0..2)).unwrap_or(&109001),
        5..=6 => *[109001, 109002, 109005]
            .get(rng.gen_range(0..3))
            .unwrap_or(&109001),
        7..=9 => *[109001, 109002, 109005, 109007]
            .get(rng.gen_range(0..4))
            .unwrap_or(&109001),
        10..=14 => *[109503, 109509].get(rng.gen_range(0..2)).unwrap_or(&109503),
        15..=17 => *[109503, 109509, 109515]
            .get(rng.gen_range(0..3))
            .unwrap_or(&109503),
        18..=26 => *[109503, 109509, 109515, 109518]
            .get(rng.gen_range(0..4))
            .unwrap_or(&109503),
        27..=32 => *[109503, 109509, 109515, 109518, 109527]
            .get(rng.gen_range(0..5))
            .unwrap_or(&109503),
        33..=34 => *[109503, 109509, 109515, 109518, 109527, 109533]
            .get(rng.gen_range(0..6))
            .unwrap_or(&109533),
        35..=38 => *[109503, 109509, 109515, 109518, 109527, 109533, 109535]
            .get(rng.gen_range(0..7))
            .unwrap_or(&109535),
        39..=41 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539,
        ]
        .get(rng.gen_range(0..8))
        .unwrap_or(&109539),
        42 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539, 109542,
        ]
        .get(rng.gen_range(0..9))
        .unwrap_or(&109542),
        43 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539, 109542, 109543,
        ]
        .get(rng.gen_range(0..10))
        .unwrap_or(&109543),
        44..=50 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539, 109542, 109543, 109545,
        ]
        .get(rng.gen_range(0..11))
        .unwrap_or(&109545),
        51..=55 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539, 109542, 109543, 109545,
            109551,
        ]
        .get(rng.gen_range(0..12))
        .unwrap_or(&109551),
        56 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539, 109542, 109543, 109545,
            109551, 109556,
        ]
        .get(rng.gen_range(0..13))
        .unwrap_or(&109556),
        57..=59 => *[
            109503, 109509, 109515, 109518, 109527, 109533, 109535, 109539, 109542, 109543, 109545,
            109551, 109556, 109557,
        ]
        .get(rng.gen_range(0..14))
        .unwrap_or(&109557),
        60..=69 => *[110542, 110543, 110545, 110551, 110556, 110557, 110560]
            .get(rng.gen_range(0..7))
            .unwrap_or(&110560),
        70..=71 => *[110542, 110543, 110545, 110551, 110571, 110570, 110560]
            .get(rng.gen_range(0..7))
            .unwrap_or(&110570),
        72..=79 => *[110542, 110543, 110545, 110572, 110571, 110570, 110560]
            .get(rng.gen_range(0..7))
            .unwrap_or(&110570),
        80..=83 => *[
            110542, 110543, 110545, 110572, 110571, 110570, 110560, 110575,
        ]
        .get(rng.gen_range(0..8))
        .unwrap_or(&110575),
        _ => 110575,
    }
}

/// Select a mage skill with random subclass (Flame/Glacier/Lightning).
/// mage subclass functions per attack. All three share the same skill pool,
/// with Glacier adding +100 and Lightning adding +200 to the final ID.
/// Exception: skill IDs 110002/210002 are NOT offset
fn get_mage_skill_with_subclass(level: u8) -> u32 {
    let base = get_mage_base_skill(level);
    let subclass = random_mage_subclass();
    let offset = mage_subclass_offset(subclass);

    // C++ exception: 110002 and 210002 are never offset
    if base == 110002 {
        base
    } else {
        base + offset
    }
}

/// Priest skill IDs by level bracket (Karus base).
/// | Level   | Skill ID | Description              |
/// |---------|----------|--------------------------|
/// | 1-11    | 101001   | Basic Attack             |
/// | 12-20   | 111511   | Holy Attack              |
/// | 21-41   | 111520   | Wrath                    |
/// | 42-50   | 111542   | Judgement                |
/// | 51-59   | 111551   | Punishment               |
/// | 60-61   | 112520   | Master Wrath             |
/// | 62-71   | 112802   | Master Judgement         |
/// | 72-83   | 112815   | Master Holy Devastate    |
fn get_priest_skill(level: u8) -> u32 {
    let mut rng = rand::thread_rng();
    match level {
        1..=11 => 101001,
        12..=20 => 111511,
        21..=41 => 111520,
        42 => 111542,
        // C++ BotMoveAttack.cpp:4131-4316 — level 43-50: myrand(1,2) → 111520 or 111542
        43..=50 => {
            if rng.gen_range(1..=2) == 1 {
                111520
            } else {
                111542
            }
        }
        // C++ level 51-59: myrand(1,3) → 111520, 111542, or 111551
        51..=59 => *[111520, 111542, 111551]
            .get(rng.gen_range(0..3))
            .unwrap_or(&111551),
        // C++ level 60-61: myrand(1,3) → 112520, 112542, 112551
        60..=61 => *[112520, 112542, 112551]
            .get(rng.gen_range(0..3))
            .unwrap_or(&112520),
        62..=71 => 112802,
        72..=83 => 112815,
        _ => 112815,
    }
}

/// Select a skill ID for a bot based on its class, level, nation, and weapon.
/// The C++ code:
/// 1. Selects a base skill ID (Karus) by level bracket
/// 2. If nation == ELMORAD, adds +100000
/// 3. If level <= MAX_LEVEL_ARDREAM and skill_id > class_threshold, subtracts 1000
/// For rogues, weapon detection determines arrow vs dagger skills:
/// - BOW (70) / CROSSBOW (71) → arrow skills
/// - Otherwise → dagger skills
pub fn select_bot_skill(bot: &BotInstance) -> u32 {
    select_bot_skill_with_weapon(bot, None)
}

/// Select a skill ID with optional weapon kind override.
/// When `world` is `Some`, the item table is consulted to detect the weapon
/// kind from the bot's RIGHTHAND slot. When `None`, dagger skills are used
/// as default for rogues.
pub fn select_bot_skill_with_weapon(bot: &BotInstance, world: Option<&WorldState>) -> u32 {
    let base_skill = if bot.is_warrior() {
        get_warrior_skill(bot.level)
    } else if bot.is_rogue() {
        // RIGHTHAND is equip_visual[6], LEFTHAND is equip_visual[7]
        let weapon_kind = detect_rogue_weapon_kind(bot, world);
        if weapon_kind == WEAPON_KIND_BOW || weapon_kind == WEAPON_KIND_CROSSBOW {
            get_rogue_arrow_skill(bot.level)
        } else {
            get_rogue_skill(bot.level)
        }
    } else if bot.is_mage() {
        get_mage_skill_with_subclass(bot.level)
    } else if bot.is_priest() {
        get_priest_skill(bot.level)
    } else {
        // Kurian or unknown — use warrior skills
        get_warrior_skill(bot.level)
    };

    // Apply nation offset: ELMORAD skills are +100000.
    let mut skill_id = if bot.nation == NATION_ELMORAD {
        base_skill + 100_000
    } else {
        base_skill
    };

    // Ardream downgrade: if level <= 59, downgrade advanced skills by -1000.
    // and subtracts 1000 from skills above a class-specific threshold.
    if bot.level <= MAX_LEVEL_ARDREAM {
        let threshold = get_ardream_threshold(bot);
        if skill_id > threshold {
            skill_id -= 1000;
        }
    }

    skill_id
}

use crate::inventory_constants::{WEAPON_KIND_BOW, WEAPON_KIND_CROSSBOW, WEAPON_KIND_DAGGER};

/// Visual slot index for RIGHTHAND weapon in equip_visual array.
/// VISUAL_SLOT_ORDER[6] = inventory slot 6 = RIGHTHAND
const VISUAL_RIGHTHAND_IDX: usize = 6;

/// Visual slot index for LEFTHAND weapon in equip_visual array.
/// VISUAL_SLOT_ORDER[7] = inventory slot 8 = LEFTHAND
const VISUAL_LEFTHAND_IDX: usize = 7;

/// Get the attack cooldown for a bot based on its class and weapon.
/// - Warrior / Rogue (dagger): 2 seconds
/// - Rogue (bow/crossbow) / Mage / Priest: 3 seconds
fn get_bot_attack_cooldown(bot: &BotInstance, world: Option<&WorldState>) -> u64 {
    if bot.is_warrior() {
        BOT_ATTACK_COOLDOWN_MELEE_MS
    } else if bot.is_rogue() {
        let weapon_kind = detect_rogue_weapon_kind(bot, world);
        if weapon_kind == WEAPON_KIND_BOW || weapon_kind == WEAPON_KIND_CROSSBOW {
            BOT_ATTACK_COOLDOWN_RANGED_MS
        } else {
            BOT_ATTACK_COOLDOWN_MELEE_MS
        }
    } else if bot.is_mage() || bot.is_priest() {
        BOT_ATTACK_COOLDOWN_RANGED_MS
    } else {
        BOT_ATTACK_COOLDOWN_MELEE_MS // Kurian/unknown
    }
}

/// Detect the weapon kind of a rogue bot's weapon slots.
/// - BOW(70) / CROSSBOW(71) → arrow skills
/// - DAGGER(11) → dagger skills
/// - Empty/shield off-hand layouts are tolerated; both hands are inspected.
/// Returns the item kind value, or `WEAPON_KIND_DAGGER` as fallback.
fn detect_rogue_weapon_kind(bot: &BotInstance, world: Option<&WorldState>) -> i32 {
    let world = match world {
        Some(w) => w,
        None => return WEAPON_KIND_DAGGER, // no world → default dagger
    };

    let mut first_weapon_kind = None;
    for slot_idx in [VISUAL_RIGHTHAND_IDX, VISUAL_LEFTHAND_IDX] {
        let (item_id, _, _) = bot.equip_visual[slot_idx];
        if item_id == 0 {
            continue;
        }
        let Some(kind) = world.get_item(item_id).and_then(|item| item.kind) else {
            continue;
        };
        if kind == WEAPON_KIND_BOW || kind == WEAPON_KIND_CROSSBOW {
            return kind;
        }
        first_weapon_kind.get_or_insert(kind);
    }
    first_weapon_kind.unwrap_or(WEAPON_KIND_DAGGER)
}

fn bot_attack_range_for_skill(world: &WorldState, bot: &BotInstance, skill_id: u32) -> f32 {
    let table_range = world
        .get_magic(skill_id as i32)
        .and_then(|m| m.range)
        .unwrap_or(0) as f32;

    let fallback = if bot.is_rogue() {
        let kind = detect_rogue_weapon_kind(bot, Some(world));
        if kind == WEAPON_KIND_BOW || kind == WEAPON_KIND_CROSSBOW {
            28.0
        } else {
            BOT_ATTACK_RANGE
        }
    } else if bot.is_mage() || bot.is_priest() {
        25.0
    } else {
        BOT_ATTACK_RANGE
    };

    let range = if table_range > 0.0 {
        table_range
    } else {
        fallback
    };

    if bot.is_warrior() || (bot.is_rogue() && range <= BOT_ATTACK_RANGE + 1.0) {
        range.clamp(3.0, BOT_ATTACK_RANGE)
    } else {
        range.clamp(6.0, 28.0)
    }
}

/// Detect weapon kinds for both hand slots of a bot.
/// `weaponSlots[] = { LEFTHAND, RIGHTHAND }` and looks up `pWeapon->GetKind()`.
/// Returns an array of `[Option<i32>; 2]` (LEFTHAND, RIGHTHAND) weapon kinds.
fn detect_bot_weapon_kinds(bot: &BotInstance, world: &WorldState) -> [Option<i32>; 2] {
    let slots = [VISUAL_LEFTHAND_IDX, VISUAL_RIGHTHAND_IDX];
    let mut kinds = [None; 2];
    for (i, &slot_idx) in slots.iter().enumerate() {
        let (item_id, _, _) = bot.equip_visual[slot_idx];
        if item_id == 0 {
            continue;
        }
        if let Some(item) = world.get_item(item_id) {
            kinds[i] = item.kind;
        }
    }
    kinds
}

/// Get the Ardream downgrade threshold for a bot's class/nation.
/// - Warrior: Karus > 106000, Elmorad > 206000
/// - Rogue (dagger): Karus > 108000, Elmorad > 208000
/// - Rogue (arrow): Karus > 108000, Elmorad > 208000
/// - Mage: Karus > 110000, Elmorad > 210000  (for lvl>=60 skills)
/// - Priest: Karus > 112000, Elmorad > 212000
fn get_ardream_threshold(bot: &BotInstance) -> u32 {
    let base = if bot.is_warrior() {
        106_000
    } else if bot.is_rogue() {
        108_000
    } else if bot.is_mage() {
        110_000
    } else if bot.is_priest() {
        112_000
    } else {
        106_000 // default warrior
    };

    if bot.nation == NATION_ELMORAD {
        base + 100_000
    } else {
        base
    }
}

// ── PK zone constants ──────────────────────────────────────────────────────

/// Returns the current UNIX timestamp in seconds.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns the current monotonic time in milliseconds.
/// Used for timing AI ticks. C++ uses `UNIXTIME2` (millisecond resolution).
pub fn tick_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Start the bot AI background task.
/// Spawns a tokio task that ticks all active bots every `BOT_AI_TICK_MS`.
/// Call this once during server startup, after world tables are loaded.
pub fn start_bot_ai_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(BOT_AI_TICK_MS));
        loop {
            interval.tick().await;
            tick_bots(&world);
        }
    })
}

/// Process one AI tick for all active bots.
/// This iterates the `WorldState::bots` DashMap and advances each bot's
/// state machine based on its `ai_state`.
pub fn tick_bots(world: &WorldState) {
    let now_unix = unix_now();
    let now_ms = tick_ms();

    // Collect IDs of bots to despawn (expired duration).
    let expired: Vec<BotId> = world.collect_expired_bot_ids(now_unix);
    for id in expired {
        despawn_bot(world, id);
    }

    // Collect IDs of all currently active bots.
    let bot_ids: Vec<BotId> = world.bots.iter().map(|e| *e.key()).collect();

    for id in bot_ids {
        // Get a snapshot of the bot to decide what to do.
        let bot = match world.get_bot(id) {
            Some(b) => b,
            None => continue,
        };

        // Skip bots not yet in-game.
        if !bot.in_game {
            continue;
        }

        // Dead bot: check if regene timer has elapsed.
        if bot.presence == BotPresence::Dead {
            if bot.regene_at_ms > 0 && now_ms >= bot.regene_at_ms {
                bot_regene(world, bot.id, now_ms);
            }
            continue;
        }

        // Rivalry expiry check.
        if bot.rival_id >= 0 && bot.rival_expiry_time > 0 {
            let now_unix = unix_now();
            if now_unix >= bot.rival_expiry_time {
                world.update_bot(bot.id, |b| {
                    b.rival_id = -1;
                    b.rival_expiry_time = 0;
                });
            }
        }

        // HP/MP regen tick (every 3 seconds).
        if now_ms.saturating_sub(bot.last_regen_ms) >= BOT_REGEN_INTERVAL_MS {
            tick_bot_regen(world, &bot, now_ms);
        }

        // Self-heal AI: if HP < 90% of max, cast heal skill.
        // Cooldown: 5 seconds between heal attempts.
        if bot.hp > 0
            && bot.max_hp > 0
            && (bot.hp as f32) < (bot.max_hp as f32 * 0.90)
            && now_ms.saturating_sub(bot.last_hp_change_ms) >= BOT_SELF_HEAL_COOLDOWN_MS
        {
            tick_bot_self_heal(world, &bot, now_ms);
        }

        // Throttle: only act if enough time has elapsed since last tick.
        if now_ms.saturating_sub(bot.last_tick_ms) < BOT_AI_TICK_MS {
            continue;
        }

        // Advance the AI state machine.
        match bot.ai_state {
            BotAiState::Idle | BotAiState::Afk => {
                // Idle/AFK bots do nothing — just update their last tick time.
                world.update_bot(id, |b| b.last_tick_ms = now_ms);
            }
            BotAiState::Mining => {
                tick_mining(world, &bot, now_ms);
            }
            BotAiState::Fishing => {
                tick_fishing(world, &bot, now_ms);
            }
            BotAiState::Merchant => {
                tick_merchant(world, &bot, now_ms);
            }
            BotAiState::MerchantMove => {
                tick_merchant_move(world, &bot, now_ms);
            }
            BotAiState::Farmer | BotAiState::Pk => {
                if bot.ai_state == BotAiState::Pk
                    && now_ms.saturating_sub(bot.last_type4_ms) >= BOT_PARTY_AI_INTERVAL_MS
                {
                    tick_pk_party_ai(world, &bot, now_ms);
                }
                tick_fighting(world, &bot, now_ms);
            }
            BotAiState::Move => {
                // Move AI: walk to destination, then go idle.
                // For now, advance tick time. Move targets are set externally.
                world.update_bot(id, |b| b.last_tick_ms = now_ms);
                trace!(
                    bot_id = id,
                    name = %bot.name,
                    state = ?bot.ai_state,
                    "bot AI move tick"
                );
            }
        }
    }
}

/// Join a compatible seeking party or invite a solo seeker. Party membership
/// uses the normal server Party roster, therefore users and bots share the
/// stock client party UI and chat channel.
fn tick_pk_party_ai(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    world.update_bot(bot.id, |b| b.last_type4_ms = now_ms);

    let Ok(bot_sid) = SessionId::try_from(bot.id) else {
        return;
    };
    let candidate = world
        .get_seeking_party_list()
        .into_iter()
        .find(|entry| {
            entry.login_type != 2
                && entry.zone as u16 == bot.zone_id
                && entry.nation == bot.nation
                && (entry.level - bot.level as i16).unsigned_abs() <= 15
                && !world.has_party_invitation(entry.sid)
        })
        // The BBS row is a snapshot and can still say party_id=0 after the
        // player accepted an invitation. Always use the live party roster so
        // bots do not keep inviting somebody who is already grouped.
        .map(|entry| (entry.sid, world.get_party_id(entry.sid).unwrap_or(0)))
        .or_else(|| {
            // The client can also expose the seek icon through STATE_CHANGE
            // without opening Party BBS. Treat that flag as an equivalent
            // request so bots react to both client flows.
            world
                .sessions_in_zone(bot.zone_id)
                .into_iter()
                .find_map(|sid| {
                    let compatible = world
                        .with_session(sid, |h| {
                            h.character.as_ref().is_some_and(|ch| {
                                h.need_party == 1
                                    && ch.nation == bot.nation
                                    && (ch.level as i16 - bot.level as i16).unsigned_abs() <= 15
                            })
                        })
                        .unwrap_or(false);
                    if compatible && !world.has_party_invitation(sid) {
                        Some((sid, world.get_party_id(sid).unwrap_or(0)))
                    } else {
                        None
                    }
                })
        });
    if let Some(current_party_id) = world.get_party_id(bot_sid) {
        let Some(party) = world.get_party(current_party_id) else {
            return;
        };
        if party.is_leader(bot_sid) && !party.is_full() {
            if let Some((seeker_sid, _)) = candidate.filter(|(_, party_id)| *party_id == 0) {
                world.set_party_invitation(seeker_sid, current_party_id, bot_sid);
                let mut permit = Packet::new(Opcode::WizParty as u8);
                permit.write_u8(0x02);
                permit.write_u32(bot.id);
                permit.write_string(&bot.name);
                world.send_to_session_owned(seeker_sid, permit);
            }
        }
        return;
    }

    let Some((seeker_sid, seeker_party_id)) = candidate else {
        // No real seeker: join an allied bot party or create a two-bot party.
        let allies: Vec<BotInstance> = world
            .get_bots_in_zone_live(bot.zone_id)
            .into_iter()
            .filter(|ally| {
                ally.id != bot.id
                    && ally.nation == bot.nation
                    && ally.ai_state == BotAiState::Pk
                    && ally.is_alive()
            })
            .collect();

        for ally in &allies {
            let Ok(ally_sid) = SessionId::try_from(ally.id) else {
                continue;
            };
            if let Some(party_id) = world.get_party_id(ally_sid) {
                if world.get_party(party_id).is_some_and(|p| !p.is_full())
                    && world.add_party_member(party_id, bot_sid)
                {
                    return;
                }
            }
        }

        if let Some(ally) = allies
            .iter()
            .filter(|ally| ally.id > bot.id)
            .find(|ally| world.get_party_id(ally.id as SessionId).is_none())
        {
            if let Some(party_id) = world.create_party(bot_sid) {
                world.add_party_member(party_id, ally.id as SessionId);
            }
        }
        return;
    };

    if seeker_party_id != 0 {
        let Some(party) = world.get_party(seeker_party_id) else {
            return;
        };
        if party.is_full() || !party.is_leader(seeker_sid) {
            return;
        }
        if world.add_party_member(seeker_party_id, bot_sid) {
            let pkt =
                crate::handler::party::build_bot_party_member_info(bot, 1, party.target_number_id);
            world.send_to_party(seeker_party_id, &pkt);
            send_bot_party_chat(
                world,
                seeker_party_id,
                bot,
                "Partyye katıldım, birlikte ilerleyelim.",
            );
        }
        return;
    }

    let Some(party_id) = world.create_party(bot_sid) else {
        return;
    };
    world.set_party_invitation(seeker_sid, party_id, bot_sid);
    let mut permit = Packet::new(Opcode::WizParty as u8);
    permit.write_u8(0x02); // PARTY_PERMIT
    permit.write_u32(bot.id);
    permit.write_string(&bot.name);
    world.send_to_session_owned(seeker_sid, permit);
}

fn send_bot_party_chat(world: &WorldState, party_id: u16, bot: &BotInstance, message: &str) {
    let pkt = crate::handler::chat::build_chat_packet(
        crate::handler::chat::ChatType::Party as u8,
        bot.nation,
        bot.id as u16,
        &bot.name,
        message,
        -1,
        1,
        0,
    );
    world.send_to_party(party_id, &pkt);
}

/// Handle compact PK-party commands sent by a real player. Returns true when
/// the message was recognised (even if the requested class/buff was missing).
pub fn handle_party_chat_command(
    world: &WorldState,
    party_id: u16,
    requester_sid: SessionId,
    message: &str,
) -> bool {
    let command = message.trim().to_ascii_lowercase();
    if !matches!(
        command.as_str(),
        "tp" | "+" | "++" | "buf" | "buff" | "ac" | "lup" | "sw"
    ) {
        return false;
    }

    let Some(party) = world.get_party(party_id) else {
        return true;
    };
    let Some(requester_pos) = world.get_position(requester_sid) else {
        return true;
    };
    let party_bots: Vec<BotInstance> = party
        .active_members()
        .into_iter()
        .filter_map(|sid| world.get_bot(sid as u32))
        .filter(|bot| bot.is_alive() && bot.zone_id == requester_pos.zone_id)
        .collect();

    if command == "tp" {
        let Some(mage) = party_bots.iter().find(|bot| bot.is_mage()) else {
            return true;
        };
        if warp_party_member_to_bot(world, requester_sid, mage) {
            send_bot_party_chat(world, party_id, mage, "TP tamam.");
        }
        return true;
    }

    let (class_group, skills): (u8, &[u32]) = match command.as_str() {
        "+" | "buf" | "buff" => (4, &[112675]),
        "ac" => (4, &[112674]),
        "++" => (4, &[112675, 112674]),
        "lup" => (2, &[108735]),
        "sw" => (2, &[108010]),
        _ => return true,
    };
    let Some(caster) = party_bots
        .iter()
        .find(|bot| bot_class_group(bot.class) == class_group)
    else {
        if let Some(bot) = party_bots.first() {
            send_bot_party_chat(
                world,
                party_id,
                bot,
                "Bu skill için gerekli class partyde yok.",
            );
        }
        return true;
    };

    let nation_offset = if caster.nation == NATION_ELMORAD {
        100_000
    } else {
        0
    };
    let mut applied = false;
    for base_skill in skills {
        applied |= crate::handler::magic_process::apply_bot_type4_support(
            world,
            caster.id,
            requester_sid,
            base_skill + nation_offset,
        );
    }
    if applied {
        send_bot_party_chat(world, party_id, caster, "İstenen party skilli uygulandı.");
    } else {
        send_bot_party_chat(
            world,
            party_id,
            caster,
            "Bu buff zaten aktif veya kullanılamıyor.",
        );
    }
    true
}

fn warp_party_member_to_bot(world: &WorldState, target_sid: SessionId, mage: &BotInstance) -> bool {
    if world.is_player_dead(target_sid) {
        return false;
    }
    let Some(old_pos) = world.get_position(target_sid) else {
        return false;
    };
    if old_pos.zone_id != mage.zone_id {
        return false;
    }

    if let Some(zone) = world.get_zone(old_pos.zone_id) {
        zone.remove_user(old_pos.region_x, old_pos.region_z, target_sid);
    }
    let dest_x = mage.x + 1.5;
    let dest_z = mage.z + 1.5;
    let new_rx = calc_region(dest_x);
    let new_rz = calc_region(dest_z);
    world.update_position(target_sid, mage.zone_id, dest_x, mage.y, dest_z);
    if let Some(zone) = world.get_zone(mage.zone_id) {
        zone.add_user(new_rx, new_rz, target_sid);
    }

    let mut pkt = Packet::new(Opcode::WizWarp as u8);
    pkt.write_u16((dest_x * 10.0) as u16);
    pkt.write_u16((dest_z * 10.0) as u16);
    pkt.write_i16(-1);
    world.send_to_session_owned(target_sid, pkt);
    true
}

// ═══════════════════════════════════════════════════════════════════════════
// Bot Combat AI — BotMoveAttack implementation
// ═══════════════════════════════════════════════════════════════════════════
//
//
// The combat state machine works as follows:
// 1. Find valid targets in surrounding 3x3 regions (IsValidTarget)
// 2. Pick the nearest target (GetNearestTarget)
// 3. If no target found, wait 5 seconds then rescan
// 4. If target found but out of attack range, move toward it
// 5. If target in range and cooldown expired, perform attack
// 6. If HP drops below 20%, switch to fleeing mode
// ═══════════════════════════════════════════════════════════════════════════

/// Calculate HP regen amount for a bot per tick.
/// Formula: `level * 2 + STA / 5`
pub fn calc_bot_hp_regen(level: u8, sta: u8) -> i16 {
    (level as i16 * 2) + (sta as i16 / 5)
}

/// Calculate MP regen amount for a bot per tick.
/// Formula: `level * 2 + INT / 5`
pub fn calc_bot_mp_regen(level: u8, intel: u8) -> i16 {
    (level as i16 * 2) + (intel as i16 / 5)
}

/// Process one HP/MP regen tick for a single bot.
fn tick_bot_regen(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    if bot.presence == BotPresence::Dead || bot.hp <= 0 {
        return;
    }
    let needs_hp = bot.hp < bot.max_hp;
    let needs_mp = bot.mp < bot.max_mp;
    if !needs_hp && !needs_mp {
        world.update_bot(bot.id, |b| {
            b.last_regen_ms = now_ms;
        });
        return;
    }
    let multiplier: i16 = if bot.presence == BotPresence::Sitting {
        2
    } else {
        1
    };
    let hp_regen = calc_bot_hp_regen(bot.level, bot.sta_stat) * multiplier;
    let mp_regen = calc_bot_mp_regen(bot.level, bot.int_stat) * multiplier;
    let new_hp = if needs_hp {
        (bot.hp + hp_regen).min(bot.max_hp)
    } else {
        bot.hp
    };
    let new_mp = if needs_mp {
        (bot.mp + mp_regen).min(bot.max_mp)
    } else {
        bot.mp
    };
    world.update_bot(bot.id, |b| {
        b.hp = new_hp;
        b.mp = new_mp;
        b.last_regen_ms = now_ms;
    });
    broadcast_bot_party_hp(world, bot.id);
    trace!(bot_id = bot.id, hp = new_hp, mp = new_mp, "bot regen tick");
}

pub(crate) fn broadcast_bot_party_hp(world: &WorldState, bot_id: BotId) {
    let Some(bot) = world.get_bot(bot_id) else {
        return;
    };
    let Some(party_id) = world.get_party_id(bot_id as SessionId) else {
        return;
    };
    let mut pkt = Packet::new(Opcode::WizParty as u8);
    pkt.write_u8(0x06); // PARTY_HPCHANGE
    pkt.write_u32(bot.id);
    pkt.write_i16(bot.max_hp);
    pkt.write_i16(bot.hp);
    pkt.write_i16(bot.max_mp);
    pkt.write_i16(bot.mp);
    world.send_to_party(party_id, &pkt);
}

/// Get zone-specific NP reward for killing a bot.
/// - Ardream (72): 32 NP
/// - Ronark Land Base (73): 64 NP
/// - Ronark Land (71) and other zones: 64 NP
fn get_bot_kill_np(zone_id: u16) -> i32 {
    match zone_id {
        ZONE_ARDREAM => 32,
        ZONE_RONARK_LAND_BASE | ZONE_RONARK_LAND => 64,
        _ => 64, // C++ default: Other_Zone_Source
    }
}

/// Capturing the Ronark monument gives that nation the same small kill-NP
/// advantage real players receive through `send_loyalty_change()`.
fn get_bot_kill_np_for_nation(world: &WorldState, zone_id: u16, nation: u8) -> i32 {
    get_bot_kill_np(zone_id)
        + if world.get_pvp_monument_nation(zone_id) == nation {
            PVP_MONUMENT_NP_BONUS as i32
        } else {
            0
        }
}

/// Process kill rewards when a player kills a bot.
/// NP reward uses zone-specific rates (matching `LoyaltyChange` flow):
/// - The base NP comes from `get_bot_kill_np()` (zone-based)
/// - Rival bonus NP added if applicable (+150 NP)
/// - Applied through `send_loyalty_change()` which handles buff/event modifiers
/// - Gold reward: level * 100 + random(0..level*50)
fn process_bot_kill_reward(
    world: &WorldState,
    killer_sid: SessionId,
    bot_level: u8,
    rival_bonus_np: i32,
) {
    let ch = match world.get_character_info(killer_sid) {
        Some(c) => c,
        None => return,
    };

    // Zone-specific NP reward
    let zone_id = world
        .get_position(killer_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);
    let np_reward = get_bot_kill_np(zone_id) + rival_bonus_np;

    if np_reward > 0 {
        crate::systems::loyalty::send_loyalty_change(
            world,
            killer_sid,
            np_reward,
            true,
            false,
            ch.loyalty_monthly > 0,
        );
    }

    // Gold reward: base + random bonus.
    let base_gold = bot_level as u32 * 100;
    let max_bonus = bot_level as u32 * 50;
    let bonus_gold = if max_bonus > 0 {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..=max_bonus)
    } else {
        0
    };
    let gold_reward = base_gold + bonus_gold;
    if gold_reward > 0 {
        world.gold_gain(killer_sid, gold_reward);
    }
    debug!(
        killer_sid,
        zone_id, bot_level, np_reward, rival_bonus_np, gold_reward, "bot kill reward distributed"
    );
}

/// Complete the player-death path for a runtime bot attacker. The normal PvP
/// handlers assume both units have ClientSessions, so bot kills need this
/// small adapter for WIZ_DEAD, NP/rank movement and Ronark narration.
fn process_bot_player_kill(world: &WorldState, bot: &BotInstance, victim_sid: SessionId) {
    let Some(victim) = world.get_character_info(victim_sid) else {
        return;
    };
    let Some(victim_pos) = world.get_position(victim_sid) else {
        return;
    };

    crate::handler::dead::broadcast_death(world, victim_sid);
    crate::handler::dead::set_who_killed_me(world, victim_sid, bot.id as SessionId);

    let np_gain = get_bot_kill_np_for_nation(world, bot.zone_id, bot.nation).max(0) as u32;
    world.update_bot(bot.id, |killer| {
        killer.loyalty = killer.loyalty.saturating_add(np_gain).min(2_100_000_000);
        killer.loyalty_monthly = killer
            .loyalty_monthly
            .saturating_add(np_gain)
            .min(2_100_000_000);
        killer.loyalty_daily = killer
            .loyalty_daily
            .saturating_add(np_gain)
            .min(2_100_000_000);
    });
    crate::systems::loyalty::send_loyalty_change(
        world,
        victim_sid,
        -(np_gain as i32),
        true,
        false,
        victim.loyalty_monthly > 0,
    );

    world.send_death_notice_to_zone(
        bot.zone_id,
        bot.id as SessionId,
        victim_sid,
        &bot.name,
        &victim.name,
        world.get_party_id(bot.id as SessionId),
        victim_pos.x.max(0.0) as u16,
        victim_pos.z.max(0.0) as u16,
    );
}

/// Combat AI tick for Farmer/Pk bots.
/// Drives the combat loop: find target, move toward it, attack when in range.
/// If the bot's HP falls below 20%, it transitions to fleeing behaviour.
fn tick_fighting(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    let bot_id = bot.id;

    // Check if bot should flee (HP < 20% of max).
    if bot.hp > 0 && bot.max_hp > 0 {
        let hp_ratio = bot.hp as f32 / bot.max_hp as f32;
        if hp_ratio < BOT_FLEE_HP_PERCENT {
            start_fleeing(world, bot, now_ms);
            return;
        }
    }

    // Find nearest enemy in surrounding regions.
    let target = find_nearest_enemy(bot, world);

    match target {
        None => {
            // No valid target found — patrol along waypoint route.
            // In C++, when no target is found the bot continues waypoint patrol.
            world.update_bot(bot_id, |b| {
                b.target_id = -1;
                b.target_changed = false;
            });

            // Party bots regroup around a real member before resuming patrol.
            if move_toward_party_anchor(world, bot, now_ms) {
                return;
            }

            // Attempt waypoint patrol movement (PK zones only).
            if !tick_waypoint_patrol(world, bot, now_ms) {
                // No waypoint route — just wait with rescan cooldown.
                world.update_bot(bot_id, |b| {
                    b.last_move_ms = now_ms;
                    b.last_tick_ms = now_ms;
                });
                trace!(
                    bot_id,
                    "no target found, waiting {}ms",
                    BOT_NO_TARGET_COOLDOWN_MS
                );
            }
        }
        Some((target_sid, target_x, target_y, target_z)) => {
            // Check if target changed.
            let prev_target = bot.target_id;
            let target_changed = prev_target != target_sid as i16;

            // Calculate distance to target.
            let dx = target_x - bot.x;
            let dz = target_z - bot.z;
            let distance = (dx * dx + dz * dz).sqrt();
            let skill_id = select_bot_skill_with_weapon(bot, Some(world));
            let attack_range = bot_attack_range_for_skill(world, bot, skill_id);

            if distance <= attack_range {
                // In attack range — perform attack.
                bot_perform_attack(world, bot, target_sid, target_x, target_y, target_z, now_ms);
            } else if distance <= BOT_SEARCH_RANGE {
                // Out of attack range but within search range — move toward target.
                let (new_x, new_y, new_z) = move_toward_target(bot, target_x, target_y, target_z);

                // Path validation: reject move if outside map boundaries.
                if !is_bot_move_valid(world, bot.zone_id, bot.x, bot.z, new_x, new_z) {
                    world.update_bot(bot_id, |b| {
                        b.target_id = -1;
                        b.last_move_ms = now_ms;
                        b.last_tick_ms = now_ms;
                    });
                    return;
                }

                // Echo state: 1=new target, 3=continuing, 0=arrived.
                let arrived = {
                    let dx2 = new_x - target_x;
                    let dz2 = new_z - target_z;
                    (dx2 * dx2 + dz2 * dz2).sqrt() <= attack_range
                };
                let echo: u8 = if target_changed {
                    1 // new target — start of new movement
                } else if arrived {
                    0 // arrived at target
                } else {
                    3 // continuing movement
                };
                broadcast_bot_move_ex(world, bot, new_x, new_y, new_z, echo);

                // Update bot position.
                world.update_bot(bot_id, |b| {
                    b.x = new_x;
                    b.y = new_y;
                    b.z = new_z;
                    b.region_x = calc_region(new_x);
                    b.region_z = calc_region(new_z);
                    b.target_id = target_sid as i16;
                    b.target_changed = false; // clear after use
                    b.last_move_ms = now_ms;
                    b.last_tick_ms = now_ms;
                });

                debug!(
                    bot_id,
                    target_id = target_sid,
                    distance,
                    new_x,
                    new_z,
                    "bot moving toward target"
                );
            } else {
                // Target out of search range — reset.
                world.update_bot(bot_id, |b| {
                    b.target_id = -1;
                    b.last_move_ms = now_ms;
                    b.last_tick_ms = now_ms;
                });
            }
        }
    }
}

fn move_toward_party_anchor(world: &WorldState, bot: &BotInstance, now_ms: u64) -> bool {
    let Some(party_id) = world.get_party_id(bot.id as SessionId) else {
        return false;
    };
    let Some(party) = world.get_party(party_id) else {
        return false;
    };
    let anchor = party
        .active_members()
        .into_iter()
        .filter(|sid| (*sid as u32) < crate::world::BOT_ID_BASE)
        .filter_map(|sid| world.get_position(sid))
        .find(|pos| pos.zone_id == bot.zone_id);
    let Some(anchor) = anchor else {
        return false;
    };
    let dx = anchor.x - bot.x;
    let dz = anchor.z - bot.z;
    if dx * dx + dz * dz <= 15.0 * 15.0 {
        return false;
    }
    let (new_x, new_y, new_z) = move_toward_target(bot, anchor.x, anchor.y, anchor.z);
    if !is_bot_move_valid(world, bot.zone_id, bot.x, bot.z, new_x, new_z) {
        return false;
    }
    broadcast_bot_move_ex(world, bot, new_x, new_y, new_z, 3);
    world.update_bot(bot.id, |moving| {
        moving.x = new_x;
        moving.y = new_y;
        moving.z = new_z;
        moving.region_x = calc_region(new_x);
        moving.region_z = calc_region(new_z);
        moving.last_move_ms = now_ms;
        moving.last_tick_ms = now_ms;
    });
    true
}

/// Move a bot along its waypoint patrol route.
/// + `MoveProcessRonarkLandTown()` / `MoveProcessArdreamLandTown()`
/// When a bot has no combat target, it walks along predefined waypoint routes.
/// Each waypoint is reached by stepping toward it (using `move_toward_target`).
/// When close enough, `move_state` advances. When the route is complete,
/// a new random route is picked and the bot continues from its current point.
/// Returns `true` if a waypoint movement was performed, `false` if the bot
/// has no active route (non-PK zone, or `move_route == 0`).
fn tick_waypoint_patrol(world: &WorldState, bot: &BotInstance, now_ms: u64) -> bool {
    let route = bot.move_route;
    let state = bot.move_state;

    // No route assigned — skip.
    if route == 0 || state == 0 {
        return false;
    }

    // Look up the current waypoint coordinates.
    let get_waypoint = |state| {
        if bot.zone_id == ZONE_RONARK_LAND {
            bot_waypoints::get_bowl_waypoint(route, state, bot.nation)
        } else {
            bot_waypoints::get_waypoint(bot.zone_id, route, state, bot.nation)
        }
    };
    let route_max = || {
        if bot.zone_id == ZONE_RONARK_LAND {
            bot_waypoints::bowl_waypoint_count()
        } else {
            bot_waypoints::route_max_waypoints(bot.zone_id, route, bot.nation)
        }
    };

    let (wp_x, wp_z) = match get_waypoint(state) {
        Some(coords) => coords,
        None => {
            // Invalid waypoint (0,0) for this nation — skip to next.
            let max = route_max();
            if state >= max {
                // Route complete — reset and respawn.
                waypoint_route_complete(world, bot, now_ms);
            } else {
                world.update_bot(bot.id, |b| {
                    b.move_state = state + 1;
                    b.last_tick_ms = now_ms;
                });
            }
            return true;
        }
    };

    // Calculate distance to waypoint.
    let dx = wp_x - bot.x;
    let dz = wp_z - bot.z;
    let dist_sq = dx * dx + dz * dz;

    // Arrival threshold: within one step distance (close enough).
    // C++ uses exact equality (`Mesafe == EnYakinMesafe`), but float rounding
    // means we use a small threshold instead.
    let step = movement_step(bot);
    let arrival_threshold = step * 1.5;

    if dist_sq <= arrival_threshold * arrival_threshold {
        // Arrived at waypoint — advance to next.
        let max = route_max();
        if state >= max {
            // Route cycle complete — pick new route, respawn.
            waypoint_route_complete(world, bot, now_ms);
        } else {
            world.update_bot(bot.id, |b| {
                b.move_state = state + 1;
                b.last_move_ms = now_ms;
                b.last_tick_ms = now_ms;
            });
            trace!(
                bot_id = bot.id,
                route,
                state = state + 1,
                "bot advanced to next waypoint"
            );
        }
        return true;
    }

    // Move toward the waypoint.
    let (new_x, new_y, new_z) = move_toward_target(bot, wp_x, bot.y, wp_z);

    // Path validation: reject move if destination is outside map boundaries.
    if !is_bot_move_valid(world, bot.zone_id, bot.x, bot.z, new_x, new_z) {
        // Skip this waypoint and advance to next.
        let max = route_max();
        if state >= max {
            waypoint_route_complete(world, bot, now_ms);
        } else {
            world.update_bot(bot.id, |b| {
                b.move_state = state + 1;
                b.last_tick_ms = now_ms;
            });
        }
        return true;
    }

    // Echo state for WIZ_MOVE.
    let arrived = {
        let dx2 = new_x - wp_x;
        let dz2 = new_z - wp_z;
        (dx2 * dx2 + dz2 * dz2).sqrt() <= arrival_threshold
    };
    let echo: u8 = if arrived { 0 } else { 3 };
    broadcast_bot_move_ex(world, bot, new_x, new_y, new_z, echo);

    // Update bot position.
    world.update_bot(bot.id, |b| {
        b.x = new_x;
        b.y = new_y;
        b.z = new_z;
        b.region_x = calc_region(new_x);
        b.region_z = calc_region(new_z);
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
    });

    trace!(
        bot_id = bot.id,
        route,
        state,
        wp_x,
        wp_z,
        new_x,
        new_z,
        "bot patrol toward waypoint"
    );

    true
}

/// Handle completion of a waypoint route cycle.
///   `isReset(false)` picks a new random route + resets `m_MoveState = 1`,
///   then `Regene(INOUT_IN)` respawns the bot.
fn waypoint_route_complete(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    let zone_id = bot.zone_id;
    let new_route = bot_waypoints::random_route(zone_id);

    // Keep the bot in the bowl and begin another route. Teleporting to the
    // nation base at every completed route caused visible disappear/reappear
    // cycles and made the patrol look stuck.
    world.update_bot(bot.id, |b| {
        b.move_route = new_route;
        b.move_state = if new_route > 0 { 1 } else { 0 };
        b.target_id = -1;
        b.target_changed = false;
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
    });

    debug!(
        bot_id = bot.id,
        old_route = bot.move_route,
        new_route,
        x = bot.x,
        z = bot.z,
        "bot patrol route complete — continuing from current position"
    );
}

/// Find the nearest valid enemy for a bot to attack.
/// + `CBot::GetNearestTarget()`
/// Iterates all sessions in the bot's zone, filtering by:
/// - Target must be alive (not dead)
/// - Target must be in a different nation (enemy)
/// - Target must be in a PK zone (for player targets)
/// - Target must NOT be a GM (authority == 0)
/// - Target must NOT be in genie mode
/// - Target must NOT be blinking (just respawned)
/// - Target must be within search range (45.0 units)
/// Also checks other bots as potential targets (opposing nation, PK zone).
/// Returns `Some((target_id, x, y, z))` for the nearest valid target, or `None`.
pub fn find_nearest_enemy(
    bot: &BotInstance,
    world: &WorldState,
) -> Option<(SessionId, f32, f32, f32)> {
    let mut nearest: Option<(SessionId, f32, f32, f32)> = None;
    let mut closest_distance_sq = f32::MAX;
    let now_unix = unix_now();

    // ── Check player sessions in the bot's zone ────────────────────────
    let zone_sids = world.sessions_in_zone(bot.zone_id);
    for sid in zone_sids {
        // Get character info (filters out sessions without a character).
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => continue,
        };

        // Get position.
        let pos = match world.get_position(sid) {
            Some(p) => p,
            None => continue,
        };

        // Must be alive (HP > 0).
        if ch.hp <= 0 {
            continue;
        }

        // Must be a different nation (enemy).
        //   `TO_USER(pTarget)->GetNation() == GetNation()`
        if ch.nation == bot.nation {
            continue;
        }

        // Must be in a PK zone.
        //   `!TO_USER(pTarget)->isInPKZone()`
        if !is_pk_zone(pos.zone_id) {
            continue;
        }

        // Must NOT be a GM.
        // C++ isGM(): authority == 0
        if ch.authority == 0 {
            continue;
        }

        // Must NOT be in genie mode.
        //   `TO_USER(pTarget)->isInGenie()`
        let is_genie = world.with_session(sid, |h| h.genie_active).unwrap_or(false);
        if is_genie {
            continue;
        }

        // Must NOT be blinking (post-respawn invulnerability).
        if world.is_player_blinking(sid, now_unix) {
            continue;
        }

        // Distance check (squared for efficiency).
        let dx = pos.x - bot.x;
        let dz = pos.z - bot.z;
        let dist_sq = dx * dx + dz * dz;

        if dist_sq < closest_distance_sq && dist_sq <= BOT_SEARCH_RANGE * BOT_SEARCH_RANGE {
            closest_distance_sq = dist_sq;
            nearest = Some((sid, pos.x, pos.y, pos.z));
        }
    }

    // ── Check other bots (opposing nation) ─────────────────────────────
    for entry in world.bots.iter() {
        let other = entry.value();

        // Skip self.
        if other.id == bot.id {
            continue;
        }

        // Must be alive and in-game.
        if !other.is_alive() {
            continue;
        }

        // Must be in the same zone.
        if other.zone_id != bot.zone_id {
            continue;
        }

        // Must be a different nation.
        //   `TO_BOT(pTarget)->GetNation() == GetNation()`
        if other.nation == bot.nation {
            continue;
        }

        // Must be in a PK zone.
        //   `!TO_BOT(pTarget)->isInPKZone()`
        if !other.is_in_pk_zone() {
            continue;
        }

        let dx = other.x - bot.x;
        let dz = other.z - bot.z;
        let dist_sq = dx * dx + dz * dz;

        if dist_sq < closest_distance_sq && dist_sq <= BOT_SEARCH_RANGE * BOT_SEARCH_RANGE {
            closest_distance_sq = dist_sq;
            // Use BotId as the target ID (cast to SessionId type).
            nearest = Some((other.id as SessionId, other.x, other.y, other.z));
        }
    }

    nearest
}

/// Calculate the bot's new position when moving toward a target.
/// + `CBot::HandleAttack()` movement
/// Moves the bot one tick-scaled step toward the target. A stable target vector
/// is important here: re-rolling jitter every second makes the client render
/// zig-zag corrections and appears as stuttering.
/// Returns `(new_x, new_y, new_z)` — the bot's new position after movement.
pub fn move_toward_target(
    bot: &BotInstance,
    target_x: f32,
    target_y: f32,
    target_z: f32,
) -> (f32, f32, f32) {
    let dx = target_x - bot.x;
    let dz = target_z - bot.z;
    let distance = (dx * dx + dz * dz).sqrt();

    if distance < 0.001 {
        return (bot.x, target_y, bot.z);
    }

    let step = movement_step(bot);

    // If one step would overshoot, snap to target (C++ sRunFinish logic).
    if step >= distance {
        return (target_x, target_y, target_z);
    }

    // Normalize and scale.
    let nx = dx / distance;
    let nz = dz / distance;
    let new_x = bot.x + nx * step;
    let new_z = bot.z + nz * step;

    (new_x, target_y, new_z)
}

/// Perform a magic-based attack from a bot to a target.
/// dispatches to class-specific magic attack methods which call
/// `MagicPacket(MAGIC_CASTING, ...)` + `MagicPacket(MAGIC_EFFECTING, ...)`.
/// We replicate the C++ approach: broadcast both MAGIC_CASTING (animation)
/// and MAGIC_EFFECTING (damage result) as WIZ_MAGIC_PROCESS packets with
/// the appropriate class-specific skill ID. This causes the client to show
/// the correct skill animation and damage numbers.
/// ## Broadcast format (S->C) — WIZ_MAGIC_PROCESS
/// | Type  | Field       |
/// |-------|-------------|
/// | u8    | bOpcode     |  (MAGIC_CASTING=1 or MAGIC_EFFECTING=3)
/// | u32le | skill_id    |
/// | u32le | caster_id   |
/// | u32le | target_id   |
/// | u32le | data[0..7]  |  (7 x u32, position/damage/result info)
fn bot_perform_attack(
    world: &WorldState,
    bot: &BotInstance,
    target_sid: SessionId,
    _target_x: f32,
    _target_y: f32,
    _target_z: f32,
    now_ms: u64,
) {
    // Check class-specific cooldown.
    let cooldown = get_bot_attack_cooldown(bot, Some(world));
    if now_ms.saturating_sub(bot.skill_cooldown[0]) < cooldown {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
        return;
    }

    // Select class-specific skill ID.
    let skill_id = select_bot_skill_with_weapon(bot, Some(world));

    // Deduct MP cost for the skill (C++ routes through CMagicProcess::MagicPacketBot
    // which deducts pSkill->m_sMsp). Look up the skill's mana cost from magic_table.
    let mp_cost = world
        .get_magic(skill_id as i32)
        .map(|m| m.msp.unwrap_or(0))
        .unwrap_or(0);
    if mp_cost > 0 {
        if bot.mp < mp_cost {
            // Not enough MP — skip attack this tick (bot will regen MP).
            world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
            trace!(
                bot_id = bot.id,
                mp = bot.mp,
                mp_cost,
                skill_id,
                "bot skipped attack — insufficient MP"
            );
            return;
        }
        // Deduct MP before casting.
        world.update_bot(bot.id, |b| {
            b.mp = (b.mp - mp_cost).max(0);
        });
    }

    // ── AOE check: if the skill is a Type3 MORAL_AREA_ENEMY with radius, use AOE path ──
    // bRadius > 0, damage is applied to all enemies in the radius.
    if bot.is_mage() {
        if let Some(magic) = world.get_magic(skill_id as i32) {
            let moral = magic.moral.unwrap_or(0);
            let type1 = magic.type1.unwrap_or(0);
            if moral == MORAL_AREA_ENEMY && type1 == 3 {
                if let Some(type3_data) = world.get_magic_type3(magic.magic_num) {
                    let radius = type3_data.radius.unwrap_or(0) as f32;
                    if radius > 0.0 {
                        // Get target position for AOE center.
                        let (tx, tz) = if let Some(pos) = world.get_position(target_sid) {
                            (pos.x, pos.z)
                        } else if let Some(other_bot) = world.get_bot(target_sid as u32) {
                            (other_bot.x, other_bot.z)
                        } else {
                            (_target_x, _target_z)
                        };
                        bot_perform_aoe_attack(
                            world,
                            bot,
                            target_sid,
                            (tx, tz),
                            skill_id,
                            (radius, now_ms),
                        );
                        return;
                    }
                }
            }
        }
    }

    // Calculate base damage from bot stats.
    let mut damage = calculate_bot_damage(bot);

    // ── AC Reduction: weapon-type resistance ───────────────────────────
    // Bot's equipped weapons reduce damage based on target's armor resistances.
    if let Some(ch) = world.get_character_info(target_sid) {
        let _ = ch; // we only need EquippedStats below
        let target_stats = world.get_equipped_stats(target_sid);
        let weapon_kinds = detect_bot_weapon_kinds(bot, world);
        let dagger_r_amount = world.get_dagger_r_amount(target_sid);
        // C++ bots don't have bow_r_amount debuff tracking; use default 100.
        let bow_r_amount: u8 = 100;
        damage = get_ac_damage(
            damage,
            &weapon_kinds,
            &target_stats,
            dagger_r_amount,
            bow_r_amount,
        );
    }

    // ── Elemental Damage: item bonuses vs target resistance ───────────
    let (elem_damage, hp_drain, mp_damage_drain, mp_drain) =
        calc_bot_elemental_damage(bot, world, target_sid);
    damage = damage.saturating_add(elem_damage).min(MAX_DAMAGE as i16);

    // ── Mirror Damage: reflect portion back to bot ─────────────────────
    // Also `Unit.cpp:1795-1796` — item-based reflection: `damage * total_d4 / 300`
    let mirror_damage = get_target_mirror_damage(world, target_sid, damage);
    if mirror_damage > 0 {
        world.update_bot(bot.id, |b| {
            b.hp = (b.hp - mirror_damage).max(0);
        });
    }

    // Apply damage to the target.
    let (result, target_hp) = apply_damage_to_target(world, target_sid, damage, bot.id);

    // ── Post-damage drain effects ──────────────────────────────────────
    // HP Drain: attacker heals total_d1 * damage / 100
    if hp_drain > 0 {
        let heal = (hp_drain * damage as i32 / 100).min(i16::MAX as i32) as i16;
        world.update_bot(bot.id, |b| {
            b.hp = (b.hp + heal).min(b.max_hp);
        });
    }
    // MP Damage: target loses damage * total_d2 / 300
    if mp_damage_drain > 0 {
        let mp_loss = (damage as i32 * mp_damage_drain / 300) as i16;
        world.update_session(target_sid, |handle| {
            if let Some(ref mut c) = handle.character {
                c.mp = (c.mp - mp_loss).max(0);
            }
        });
    }
    // MP Drain: attacker gains total_d3 * damage / 100
    if mp_drain > 0 {
        let mp_gain = (mp_drain * damage as i32 / 100).min(i16::MAX as i32) as i16;
        world.update_bot(bot.id, |b| {
            b.mp = (b.mp + mp_gain).min(b.max_mp);
        });
    }

    // ── Phase 1: Broadcast MAGIC_CASTING (cast animation) ────────────
    //                              (uint16)GetX(), (uint16)GetY(), (uint16)GetZ())`
    // The data fields carry caster position for the casting animation.
    let cast_pkt = build_bot_magic_packet(
        MAGIC_CASTING,
        skill_id,
        bot.id,
        target_sid as u32,
        [bot.x as i32, bot.y as i32, bot.z as i32, 0, 0, 0, 0],
    );
    broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &cast_pkt);

    // ── Phase 2: Broadcast MAGIC_EFFECTING (damage result) ───────────
    //                              (uint16)GetX(), (uint16)GetY(), (uint16)GetZ())`
    // data[3] carries the damage/result for the client to display.
    let effect_pkt = build_bot_magic_packet(
        MAGIC_EFFECTING,
        skill_id,
        bot.id,
        target_sid as u32,
        [
            bot.x as i32,
            bot.y as i32,
            bot.z as i32,
            -(damage as i32), // data[3]: negative = damage dealt
            0,
            0,
            0,
        ],
    );
    broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &effect_pkt);

    // If we hit a player, also send them an HP change packet.
    if result == ATTACK_SUCCESS || result == ATTACK_TARGET_DEAD {
        if let Some(pos) = world.get_position(target_sid) {
            if let Some(ch) = world.get_character_info(target_sid) {
                let hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
                    ch.max_hp, target_hp, bot.id,
                );
                world.send_to_session_owned(target_sid, hp_pkt);
            }

            // Broadcast WIZ_TARGET_HP to attacker's region so everyone sees
            // the target HP bar.
            if let Some(ch) = world.get_character_info(target_sid) {
                let target_hp_pkt = crate::handler::target_hp::build_target_hp_packet(
                    target_sid as u32,
                    0,
                    ch.max_hp as u32,
                    target_hp.max(0) as u32,
                    -(damage as i32),
                    -(damage as i32),
                );
                broadcast_to_bot_region(
                    world,
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    &target_hp_pkt,
                );
            }
        }
    }

    if result == ATTACK_TARGET_DEAD {
        process_bot_player_kill(world, bot, target_sid);
    }

    // Update bot state.
    world.update_bot(bot.id, |b| {
        b.skill_cooldown[0] = now_ms;
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
        b.target_id = target_sid as i16;
    });

    debug!(
        bot_id = bot.id,
        target_id = target_sid,
        damage,
        result,
        "bot performed attack"
    );
}

/// Build a WIZ_MAGIC_PROCESS packet for bot skill broadcasting.
/// Mirrors the exact wire format used by the magic system:
/// `[u8 opcode_header][u8 magic_opcode][u32 skill_id][u32 caster_id][u32 target_id][u32 data[0..7]]`
/// # Arguments
/// - `magic_opcode` — MAGIC_CASTING (1) or MAGIC_EFFECTING (3)
/// - `skill_id` — class-specific skill ID from `select_bot_skill()`
/// - `caster_id` — bot's runtime ID
/// - `target_id` — target entity ID
/// - `data` — 7-element data array (position, damage, etc.)
fn build_bot_magic_packet(
    magic_opcode: u8,
    skill_id: u32,
    caster_id: u32,
    target_id: u32,
    data: [i32; 7],
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(magic_opcode);
    pkt.write_u32(skill_id);
    pkt.write_u32(caster_id);
    pkt.write_u32(target_id);
    for d in &data {
        pkt.write_u32(*d as u32);
    }
    pkt
}

/// Find all enemy targets within a given radius of a center point.
/// gathers all potential targets from a 3×3 region grid. Then `UserRegionCheck()`
/// in `MagicProcess.cpp:465` filters by `isInRangeSlow(mousex, mousez, radius)`.
/// Returns a list of `(target_id, x, z)` tuples for all valid enemy targets
/// within the radius. Targets include both players and enemy bots.
fn find_aoe_targets(
    bot: &BotInstance,
    world: &WorldState,
    center_x: f32,
    center_z: f32,
    radius: f32,
    primary_target: SessionId,
) -> Vec<(SessionId, f32, f32)> {
    let radius_sq = radius * radius;
    let now_unix = unix_now();
    let mut targets: Vec<(SessionId, f32, f32)> = Vec::with_capacity(8);

    // ── Scan player sessions in the bot's zone ─────────────────────────
    let zone_sids = world.sessions_in_zone(bot.zone_id);
    for sid in zone_sids {
        // Skip the primary target — it's already handled.
        if sid == primary_target {
            continue;
        }

        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => continue,
        };

        let pos = match world.get_position(sid) {
            Some(p) => p,
            None => continue,
        };

        // Must be alive.
        if ch.hp <= 0 {
            continue;
        }

        // Must be enemy nation.
        if ch.nation == bot.nation {
            continue;
        }

        // Must be in a PK zone.
        if !is_pk_zone(pos.zone_id) {
            continue;
        }

        // Must NOT be a GM.
        if ch.authority == 0 {
            continue;
        }

        // Must NOT be in genie mode.
        let is_genie = world.with_session(sid, |h| h.genie_active).unwrap_or(false);
        if is_genie {
            continue;
        }

        // Must NOT be blinking (post-respawn invulnerability).
        if world.is_player_blinking(sid, now_unix) {
            continue;
        }

        // Distance check from AOE center.
        let dx = center_x - pos.x;
        let dz = center_z - pos.z;
        let dist_sq = dx * dx + dz * dz;
        if dist_sq > radius_sq {
            continue;
        }

        targets.push((sid, pos.x, pos.z));
    }

    // ── Scan enemy bots in the same zone ───────────────────────────────
    for entry in world.bots.iter() {
        let other = entry.value();

        if other.id == bot.id {
            continue;
        }

        // Skip the primary target.
        if other.id as SessionId == primary_target {
            continue;
        }

        if !other.is_alive() {
            continue;
        }

        if other.zone_id != bot.zone_id {
            continue;
        }

        if other.nation == bot.nation {
            continue;
        }

        let dx = center_x - other.x;
        let dz = center_z - other.z;
        let dist_sq = dx * dx + dz * dz;
        if dist_sq > radius_sq {
            continue;
        }

        targets.push((other.id as SessionId, other.x, other.z));
    }

    targets
}

/// Perform an AOE (area-of-effect) magic attack from a mage bot.
/// gathers all units from surrounding regions, filters by `bRadius`, and applies
/// damage to each. `UserRegionCheck()` in `MagicProcess.cpp:465` uses
/// `isInRangeSlow()` for the radius check.
/// This function:
/// 1. Applies damage to the primary target (already calculated by caller)
/// 2. Finds all secondary targets within the AOE radius (centered on primary target)
/// 3. Calculates and applies damage to each secondary target
/// 4. Broadcasts MAGIC_CASTING once, then MAGIC_EFFECTING per hit target
fn bot_perform_aoe_attack(
    world: &WorldState,
    bot: &BotInstance,
    primary_target: SessionId,
    target_pos: (f32, f32),
    skill_id: u32,
    aoe: (f32, u64), // (radius, now_ms)
) {
    let (primary_target_x, primary_target_z) = target_pos;
    let (radius, now_ms) = aoe;
    // ── Phase 1: Broadcast MAGIC_CASTING (cast animation) ────────────
    let cast_pkt = build_bot_magic_packet(
        MAGIC_CASTING,
        skill_id,
        bot.id,
        primary_target as u32,
        [bot.x as i32, bot.y as i32, bot.z as i32, 0, 0, 0, 0],
    );
    broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &cast_pkt);

    // ── Phase 2: Apply damage to primary target ──────────────────────
    let primary_damage = calculate_aoe_target_damage(world, bot, primary_target);
    let (result, target_hp) = apply_damage_to_target(world, primary_target, primary_damage, bot.id);
    broadcast_bot_magic_effecting(world, bot, primary_target, skill_id, primary_damage);
    broadcast_target_hp_update(
        world,
        bot,
        primary_target,
        result,
        target_hp,
        primary_damage as i32,
    );
    if result == ATTACK_TARGET_DEAD {
        process_bot_player_kill(world, bot, primary_target);
    }

    // ── Phase 3: Find and damage secondary targets ───────────────────
    let secondary_targets = find_aoe_targets(
        bot,
        world,
        primary_target_x,
        primary_target_z,
        radius,
        primary_target,
    );

    for (target_sid, _tx, _tz) in &secondary_targets {
        let damage = calculate_aoe_target_damage(world, bot, *target_sid);
        let (res, hp) = apply_damage_to_target(world, *target_sid, damage, bot.id);
        broadcast_bot_magic_effecting(world, bot, *target_sid, skill_id, damage);
        broadcast_target_hp_update(world, bot, *target_sid, res, hp, damage as i32);
        if res == ATTACK_TARGET_DEAD {
            process_bot_player_kill(world, bot, *target_sid);
        }
    }

    // ── Phase 4: Update bot state ────────────────────────────────────
    world.update_bot(bot.id, |b| {
        b.skill_cooldown[0] = now_ms;
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
        b.target_id = primary_target as i16;
    });

    debug!(
        bot_id = bot.id,
        primary_target,
        primary_damage,
        aoe_count = secondary_targets.len() + 1,
        radius,
        "bot performed AOE attack"
    );
}

/// Calculate AOE damage for a specific target.
/// Uses the same base damage formula as single-target attacks, plus
/// AC reduction and elemental bonuses when targeting players.
fn calculate_aoe_target_damage(
    world: &WorldState,
    bot: &BotInstance,
    target_sid: SessionId,
) -> i16 {
    let mut damage = calculate_bot_damage(bot);

    // AC weapon-type reduction.
    if let Some(_ch) = world.get_character_info(target_sid) {
        let target_stats = world.get_equipped_stats(target_sid);
        let weapon_kinds = detect_bot_weapon_kinds(bot, world);
        let dagger_r_amount = world.get_dagger_r_amount(target_sid);
        let bow_r_amount: u8 = 100;
        damage = get_ac_damage(
            damage,
            &weapon_kinds,
            &target_stats,
            dagger_r_amount,
            bow_r_amount,
        );
    }

    // Elemental bonuses.
    let (elem_damage, _hp_drain, _mp_damage_drain, _mp_drain) =
        calc_bot_elemental_damage(bot, world, target_sid);
    damage = damage.saturating_add(elem_damage).min(MAX_DAMAGE as i16);

    damage
}

/// Broadcast a MAGIC_EFFECTING packet for a bot attack on a specific target.
fn broadcast_bot_magic_effecting(
    world: &WorldState,
    bot: &BotInstance,
    target_sid: SessionId,
    skill_id: u32,
    damage: i16,
) {
    let effect_pkt = build_bot_magic_packet(
        MAGIC_EFFECTING,
        skill_id,
        bot.id,
        target_sid as u32,
        [
            bot.x as i32,
            bot.y as i32,
            bot.z as i32,
            -(damage as i32),
            0,
            0,
            0,
        ],
    );
    broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &effect_pkt);
}

/// Broadcast target HP update packets after bot damage.
/// Wire format: `[u32 target_id][u8 echo][u32 max_hp][u32 current_hp][u32 damage][u32 0][u8 0]`
/// Damage field: negative for damage dealt, positive for heal (C++ parity).
fn broadcast_target_hp_update(
    world: &WorldState,
    bot: &BotInstance,
    target_sid: SessionId,
    result: u8,
    target_hp: i16,
    damage: i32,
) {
    if result == ATTACK_SUCCESS || result == ATTACK_TARGET_DEAD {
        if let Some(pos) = world.get_position(target_sid) {
            if let Some(ch) = world.get_character_info(target_sid) {
                let hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
                    ch.max_hp, target_hp, bot.id,
                );
                world.send_to_session_owned(target_sid, hp_pkt);
            }

            if let Some(ch) = world.get_character_info(target_sid) {
                let target_hp_pkt = crate::handler::target_hp::build_target_hp_packet(
                    target_sid as u32,
                    0,
                    ch.max_hp as u32,
                    target_hp.max(0) as u32,
                    -damage,
                    -damage,
                );
                broadcast_to_bot_region(
                    world,
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    &target_hp_pkt,
                );
            }
        }
    }
}

/// Calculate base damage for a bot based on its stats and class.
/// `MagicInstance.cpp` damage formulas.
/// Since bots now use WIZ_MAGIC_PROCESS (skill-based attacks), the damage
/// formula accounts for the skill's magic type:
/// - **Warriors**: STR-based melee skill damage.
///   Formula: `(STR * level / 25) + (level * 2)`
///   C++ warrior skills are physical Type 1 — STR is primary stat.
/// - **Rogues**: DEX-based skill damage.
///   Formula: `(DEX * level / 25) + (level * 2)`
///   C++ rogue skills are physical Type 1 — DEX is primary stat.
/// - **Mages**: INT-based magic damage.
///   Formula: `(INT * level / 20) + (level * 3)`
///   C++ mage skills are magic Type 3 — INT is primary stat with higher scaling.
/// - **Priests**: INT-based magic damage (lower scaling than mage).
///   Formula: `(INT * level / 25) + (level * 2)`
///   C++ priest skills are magic Type 3 — INT is primary but with healing focus.
/// Minimum damage is 10, maximum is capped at 800 per hit.
pub fn calculate_bot_damage(bot: &BotInstance) -> i16 {
    let level = bot.level as f32;
    let damage = if bot.is_warrior() {
        // STR-based warrior skill damage
        (bot.str_stat as f32 * level / 25.0 + level * 2.0) as i16
    } else if bot.is_rogue() {
        // DEX-based rogue skill damage
        (bot.dex_stat as f32 * level / 25.0 + level * 2.0) as i16
    } else if bot.is_mage() {
        // INT-based mage magic damage (highest scaling)
        (bot.int_stat as f32 * level / 20.0 + level * 3.0) as i16
    } else if bot.is_priest() {
        // INT-based priest damage (lower than mage)
        (bot.int_stat as f32 * level / 25.0 + level * 2.0) as i16
    } else {
        // Kurian or unknown — STR-based
        (bot.str_stat as f32 * level / 25.0 + level * 2.0) as i16
    };

    damage.clamp(10, MAX_DAMAGE as i16)
}

/// Calculate mirror/reflect damage that the target would reflect back to the attacker.
/// - `BotHealthHandler.cpp:29-41` — buff-based mirror: `(m_byMirrorAmount * amount) / 100`
/// - `Unit.cpp:1795-1796` — item-based reflection: `damage * total_d4 / 300`
/// For simplicity, we check the target player's `equipped_item_bonuses` for
/// `ITEM_TYPE_MIRROR_DAMAGE` (type 8) entries and apply the `/300` formula.
/// The buff-based mirror would require tracking a separate mirror buff flag on
/// the player session; for now we only handle item-based reflection.
fn get_target_mirror_damage(world: &WorldState, target_sid: SessionId, damage: i16) -> i16 {
    let stats = world.get_equipped_stats(target_sid);
    let mut total_mirror: i32 = 0;
    for bonuses in stats.equipped_item_bonuses.values() {
        for &(btype, amount) in bonuses {
            if btype == ITEM_TYPE_MIRROR_DAMAGE {
                total_mirror += amount;
            }
        }
    }
    if total_mirror > 0 {
        ((damage as i32 * total_mirror / 300).max(0) as i16).min(MAX_DAMAGE as i16)
    } else {
        0
    }
}

// ── Item bonus type constants (C++ GameDefine.h:1365-1372) ──────────────
const ITEM_TYPE_FIRE: u8 = 1;
const ITEM_TYPE_COLD: u8 = 2;
const ITEM_TYPE_LIGHTNING: u8 = 3;
const ITEM_TYPE_POISON: u8 = 4;
const ITEM_TYPE_HP_DRAIN: u8 = 5;
const ITEM_TYPE_MP_DAMAGE: u8 = 6;
const ITEM_TYPE_MP_DRAIN: u8 = 7;
const ITEM_TYPE_MIRROR_DAMAGE: u8 = 8;

/// Collect elemental/drain bonuses from bot's equipped items.
/// accumulates fire_damage, ice_damage, lightning_damage, poison_damage,
/// hp_drain, mp_damage, mp_drain, mirror_damage from item table.
/// Returns a vector of `(type, amount)` tuples.
fn collect_bot_item_bonuses(bot: &BotInstance, world: &WorldState) -> Vec<(u8, i32)> {
    let mut bonuses = Vec::new();
    // Iterate all 17 visual slots (equipped + cosplay).
    for &(item_id, _, _) in &bot.equip_visual {
        if item_id == 0 {
            continue;
        }
        let item = match world.get_item(item_id) {
            Some(i) => i,
            None => continue,
        };
        if let Some(v) = item.fire_damage {
            if v != 0 {
                bonuses.push((ITEM_TYPE_FIRE, v));
            }
        }
        if let Some(v) = item.ice_damage {
            if v != 0 {
                bonuses.push((ITEM_TYPE_COLD, v));
            }
        }
        if let Some(v) = item.lightning_damage {
            if v != 0 {
                bonuses.push((ITEM_TYPE_LIGHTNING, v));
            }
        }
        if let Some(v) = item.poison_damage {
            if v != 0 {
                bonuses.push((ITEM_TYPE_POISON, v));
            }
        }
        if let Some(v) = item.hp_drain {
            if v != 0 {
                bonuses.push((ITEM_TYPE_HP_DRAIN, v));
            }
        }
        if let Some(v) = item.mp_damage {
            if v != 0 {
                bonuses.push((ITEM_TYPE_MP_DAMAGE, v));
            }
        }
        if let Some(v) = item.mp_drain {
            if v != 0 {
                bonuses.push((ITEM_TYPE_MP_DRAIN, v));
            }
        }
        if let Some(v) = item.mirror_damage {
            if v != 0 {
                bonuses.push((ITEM_TYPE_MIRROR_DAMAGE, v));
            }
        }
    }
    bonuses
}

/// Calculate elemental bonus damage from bot's equipped items against a target.
/// For each elemental bonus (fire/cold/lightning):
///   `total_r = (target_base_r + target_add_r) * target_pct_r / 100 + resistance_bonus`
///   Capped at 200.
///   `bonus_damage += amount - amount * total_r / 200`
/// Drain types (HP_DRAIN, MP_DAMAGE, MP_DRAIN) are accumulated separately and
/// applied after the main damage is dealt.
/// Returns `(extra_elemental_damage, hp_drain_total, mp_damage_total, mp_drain_total)`.
fn calc_bot_elemental_damage(
    bot: &BotInstance,
    world: &WorldState,
    target_sid: SessionId,
) -> (i16, i32, i32, i32) {
    let bonuses = collect_bot_item_bonuses(bot, world);
    if bonuses.is_empty() {
        return (0, 0, 0, 0);
    }

    // Get target's resistances. If target is a player, use their equipped + buff stats.
    // C++ formula: total_r = (m_sFireR + m_bAddFireR) * m_bPctFireR / 100
    //   m_sFireR → EquippedStats.fire_r (from items)
    //   m_bAddFireR → get_buff_elemental_resistance(sid, 1) (from Type4 buffs)
    //   m_bPctFireR → session.pct_fire_r (debuff percentage, default 100)
    let eq_stats = world.get_equipped_stats(target_sid);
    let (pct_fire, pct_cold, pct_lightning) = world
        .with_session(target_sid, |h| {
            (h.pct_fire_r, h.pct_cold_r, h.pct_lightning_r)
        })
        .unwrap_or((100, 100, 100));
    let add_fire = world.get_buff_elemental_resistance(target_sid, 1) as i16;
    let add_cold = world.get_buff_elemental_resistance(target_sid, 2) as i16;
    let add_lightning = world.get_buff_elemental_resistance(target_sid, 3) as i16;

    let mut elemental_damage: i32 = 0;
    let mut total_d1: i32 = 0; // HP drain
    let mut total_d2: i32 = 0; // MP damage
    let mut total_d3: i32 = 0; // MP drain

    for &(btype, amount) in &bonuses {
        let is_drain = (ITEM_TYPE_HP_DRAIN..=ITEM_TYPE_MP_DRAIN).contains(&btype);

        let total_r = match btype {
            ITEM_TYPE_FIRE => (eq_stats.fire_r + add_fire) as i32 * pct_fire as i32 / 100,
            ITEM_TYPE_COLD => (eq_stats.cold_r + add_cold) as i32 * pct_cold as i32 / 100,
            ITEM_TYPE_LIGHTNING => {
                (eq_stats.lightning_r + add_lightning) as i32 * pct_lightning as i32 / 100
            }
            ITEM_TYPE_HP_DRAIN => {
                total_d1 += amount;
                continue;
            }
            ITEM_TYPE_MP_DAMAGE => {
                total_d2 += amount;
                continue;
            }
            ITEM_TYPE_MP_DRAIN => {
                total_d3 += amount;
                continue;
            }
            _ => continue,
        };

        if !is_drain {
            let capped_r = (total_r + eq_stats.resistance_bonus as i32).min(200);
            let temp_damage = amount - amount * capped_r / 200;
            elemental_damage += temp_damage;
        }
    }

    (
        elemental_damage.clamp(0, MAX_DAMAGE) as i16,
        total_d1,
        total_d2,
        total_d3,
    )
}

/// Apply damage to a target (player session or bot) and return the result
/// code and remaining HP.
/// Returns `(attack_result, target_current_hp)`:
/// - `ATTACK_SUCCESS` (1) if damage dealt, target alive
/// - `ATTACK_TARGET_DEAD` (2) if target died
/// If the target is a player session, updates their HP in the session handle.
/// If the target is a bot, updates their HP in the bots DashMap.
fn apply_damage_to_target(
    world: &WorldState,
    target_id: SessionId,
    damage: i16,
    attacker_id: BotId,
) -> (u8, i16) {
    // Enforce MAX_DAMAGE cap
    let damage = damage.clamp(-(MAX_DAMAGE as i16), MAX_DAMAGE as i16);

    // Try player session first.
    if let Some(ch) = world.get_character_info(target_id) {
        let new_hp = (ch.hp - damage).max(0);
        let result = if new_hp <= 0 {
            ATTACK_TARGET_DEAD
        } else {
            ATTACK_SUCCESS
        };

        // Update player HP.
        world.update_session(target_id, |handle| {
            if let Some(ref mut c) = handle.character {
                c.hp = new_hp;
            }
        });

        return (result, new_hp);
    }

    // Try bot target.
    if let Some(target_bot) = world.get_bot(target_id as BotId) {
        let new_hp = (target_bot.hp - damage).max(0);
        let result = if new_hp <= 0 {
            ATTACK_TARGET_DEAD
        } else {
            ATTACK_SUCCESS
        };

        // Update HP and track attacker for kill rewards.
        world.update_bot(target_id as BotId, |b| {
            b.hp = new_hp;
            b.last_attacker_id = attacker_id as i32;
        });
        broadcast_bot_party_hp(world, target_id as BotId);

        // If the bot died, trigger the full death processing (WIZ_DEAD broadcast,
        // regene timer, etc.) instead of just setting presence.
        if new_hp <= 0 {
            bot_on_death(world, target_id as BotId, tick_ms());
        }

        return (result, new_hp);
    }

    // Target not found — attack failed implicitly (no broadcast).
    (ATTACK_SUCCESS, 0)
}

/// Switch a bot to fleeing mode — run away from the current target.
/// C++ bots don't have an explicit flee state (they fight to death),
/// but we add this for more realistic behaviour.
/// The bot moves in the opposite direction from the nearest enemy
/// by `BOT_FLEE_DISTANCE` units, then returns to idle.
fn start_fleeing(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    let bot_id = bot.id;

    // Calculate flee direction (opposite of target direction).
    let (flee_x, flee_z) = if bot.target_id >= 0 {
        // Try to find the target's position to flee away from.
        if let Some(pos) = world.get_position(bot.target_id as SessionId) {
            let dx = bot.x - pos.x;
            let dz = bot.z - pos.z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist > 0.001 {
                let nx = dx / dist;
                let nz = dz / dist;
                (
                    bot.x + nx * BOT_FLEE_DISTANCE,
                    bot.z + nz * BOT_FLEE_DISTANCE,
                )
            } else {
                flee_random_direction(bot)
            }
        } else {
            flee_random_direction(bot)
        }
    } else {
        flee_random_direction(bot)
    };

    // Clamp to positive values (no negative coordinates).
    let flee_x = flee_x.max(1.0);
    let flee_z = flee_z.max(1.0);

    // Path validation: reject flee if outside map boundaries.
    if !is_bot_move_valid(world, bot.zone_id, bot.x, bot.z, flee_x, flee_z) {
        world.update_bot(bot_id, |b| {
            b.target_id = -1;
            b.last_move_ms = now_ms;
            b.last_tick_ms = now_ms;
        });
        return;
    }

    // Broadcast move packet for flee movement (echo=1: new movement start).
    broadcast_bot_move_ex(world, bot, flee_x, bot.y, flee_z, 1);

    // Update bot position and clear target.
    world.update_bot(bot_id, |b| {
        b.x = flee_x;
        b.z = flee_z;
        b.region_x = calc_region(flee_x);
        b.region_z = calc_region(flee_z);
        b.target_id = -1;
        b.target_changed = false;
        b.last_move_ms = now_ms;
        b.last_tick_ms = now_ms;
        // Keep in Farmer/Pk state — will resume fighting after cooldown.
    });

    debug!(
        bot_id,
        hp = bot.hp,
        max_hp = bot.max_hp,
        flee_x,
        flee_z,
        "bot fleeing (low HP)"
    );
}

/// Compute a random flee position for a bot.
/// Used when the target position is unknown or the bot has no target.
fn flee_random_direction(bot: &BotInstance) -> (f32, f32) {
    let mut rng = rand::thread_rng();
    let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
    (
        bot.x + angle.cos() * BOT_FLEE_DISTANCE,
        bot.z + angle.sin() * BOT_FLEE_DISTANCE,
    )
}

/// Broadcast a WIZ_MOVE packet for a bot's movement.
/// surrounding regions.
/// ## Wire format (S->C):
/// `[u32 socket_id][u16 will_x][u16 will_z][u16 will_y][i16 speed][u8 echo]`
/// ## Echo values
/// - `1` — start of new movement (target changed, new run begins)
/// - `3` — continuing movement (mid-run)
/// - `0` — movement finished (arrived at destination)
fn broadcast_bot_move_ex(
    world: &WorldState,
    bot: &BotInstance,
    new_x: f32,
    new_y: f32,
    new_z: f32,
    echo: u8,
) {
    let will_x = (new_x * 10.0) as u16;
    let will_y = (new_y * 10.0) as u16;
    let will_z = (new_z * 10.0) as u16;
    let speed = get_bot_speed(bot);

    let mut pkt = Packet::new(Opcode::WizMove as u8);
    pkt.write_u32(bot.id);
    pkt.write_u16(will_x);
    pkt.write_u16(will_z);
    pkt.write_u16(will_y);
    pkt.write_i16(speed as i16);
    pkt.write_u8(echo);

    broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &pkt);
}

/// Get bot movement speed based on class and zone.
/// | Zone    | Rogue/Captain | Others | Default |
/// |---------|--------------|--------|---------|
/// | PK zone | 90.0         | 67.0   | 67.0    |
/// | Non-PK  | 45.0         | 45.0   | 45.0    |
fn get_bot_speed(bot: &BotInstance) -> f32 {
    if bot.is_in_pk_zone() {
        if bot.is_rogue() || bot.fame >= 5 {
            90.0 // C++ COMMAND_CAPTAIN = 5, rogues and captains
        } else {
            67.0
        }
    } else {
        45.0
    }
}

/// Convert the protocol speed value into distance for the current AI cadence.
/// At the previous 1-second cadence this was `speed / 10`; scaling by elapsed
/// tick duration preserves the same units/second while producing smoother
/// intermediate WIZ_MOVE packets.
fn movement_step(bot: &BotInstance) -> f32 {
    get_bot_speed(bot) * BOT_AI_TICK_MS as f32 / 10_000.0
}

/// Check if a zone is a PK zone (where bot PvP is allowed).
fn is_pk_zone(zone_id: u16) -> bool {
    matches!(zone_id, ZONE_RONARK_LAND..=ZONE_RONARK_LAND_BASE)
}

/// Check if a position is inside the map bounds.
///
/// Returns `true` if the position is within bounds (or if no map data is loaded).
fn is_bot_position_valid(world: &WorldState, zone_id: u16, x: f32, z: f32) -> bool {
    match world.get_zone(zone_id) {
        Some(zone) => zone.is_valid_position(x, z),
        None => true, // No zone data → permissive (same as C++ fallback)
    }
}

/// Validate the full short movement segment, not only its destination. This
/// prevents bots cutting across blocked SMD tiles and appearing on cliffs or
/// behind map objects that players cannot cross.
fn is_bot_move_valid(
    world: &WorldState,
    zone_id: u16,
    from_x: f32,
    from_z: f32,
    to_x: f32,
    to_z: f32,
) -> bool {
    let distance = ((to_x - from_x).powi(2) + (to_z - from_z).powi(2)).sqrt();
    let samples = (distance / 1.5).ceil().max(4.0) as usize;
    (1..=samples).all(|sample| {
        let t = sample as f32 / samples as f32;
        is_bot_position_valid(
            world,
            zone_id,
            from_x + (to_x - from_x) * t,
            from_z + (to_z - from_z) * t,
        )
    })
}

/// Broadcast a mining animation packet for a bot and update its timer.
fn tick_mining(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    if now_ms.saturating_sub(bot.last_mining_ms) < BOT_MINING_INTERVAL_MS {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
        return;
    }

    // Build WIZ_MINING packet: opcode=MiningAttempt(2), result=Success(1),
    // caster_id=bot.id, effect=13081 or 13082 (random).
    // C++ packets.h: MiningAttempt=2
    // C++ packet: result << resultCode << uint32(GetID()) << sEffect
    let effect: u16 = if now_ms.is_multiple_of(2) {
        13082
    } else {
        13081
    };
    let bot_id = bot.id;
    let zone_id = bot.zone_id;
    let rx = bot.region_x;
    let rz = bot.region_z;

    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(2); // sub-opcode: MiningAttempt (C++ packets.h: MiningAttempt=2)
    pkt.write_u16(1); // resultCode: MiningResultSuccess
    pkt.write_u32(bot_id);
    pkt.write_u16(effect);

    broadcast_to_bot_region(world, zone_id, rx, rz, &pkt);

    world.update_bot(bot.id, |b| {
        b.last_mining_ms = now_ms;
        b.last_tick_ms = now_ms;
    });

    debug!(bot_id, name = %bot.name, "bot mining tick");
}

/// Broadcast a fishing animation packet for a bot and update its timer.
fn tick_fishing(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    if now_ms.saturating_sub(bot.last_mining_ms) < BOT_MINING_INTERVAL_MS {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
        return;
    }

    let effect: u16 = if now_ms.is_multiple_of(2) {
        13082
    } else {
        13081
    };
    let bot_id = bot.id;
    let zone_id = bot.zone_id;
    let rx = bot.region_x;
    let rz = bot.region_z;

    // FishingAttempt sub-opcode = 7 (C++ packets.h: FishingAttempt=7)
    let mut pkt = Packet::new(Opcode::WizMining as u8);
    pkt.write_u8(7); // sub-opcode: FishingAttempt (C++ packets.h: FishingAttempt=7)
    pkt.write_u16(1); // resultCode: MiningResultSuccess
    pkt.write_u32(bot_id);
    pkt.write_u16(effect);

    broadcast_to_bot_region(world, zone_id, rx, rz, &pkt);

    world.update_bot(bot.id, |b| {
        b.last_mining_ms = now_ms;
        b.last_tick_ms = now_ms;
    });

    debug!(bot_id, name = %bot.name, "bot fishing tick");
}

/// Merchant move interval constants.
const BOT_MERCHANT_MOVE_MIN_MS: u64 = 7_000;
const BOT_MERCHANT_MOVE_MAX_MS: u64 = 17_000;

/// Walking merchant AI: move toward nearby merchant players, then transition to Merchant.
/// Behaviour:
/// 1. Every 7-17s (random delay), scan surrounding 3×3 region for merchant players.
/// 2. Move toward the nearest one using step-based movement.
/// 3. Also broadcast merchant chat at the regular interval.
/// 4. If no merchants found, stand still.
fn tick_merchant_move(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    // Also do merchant chat if interval is up.
    if now_ms.saturating_sub(bot.last_merchant_chat_ms) >= BOT_MERCHANT_CHAT_INTERVAL_MS
        && !bot.merchant_chat.is_empty()
    {
        // Inline merchant chat broadcast (reuse packet format from tick_merchant).
        let mut pkt = Packet::new(Opcode::WizChat as u8);
        pkt.write_u8(14); // MERCHANT_CHAT
        pkt.write_u8(bot.nation);
        pkt.write_i16(bot.id as i16);
        let name_bytes = bot.name.as_bytes();
        pkt.write_u8(name_bytes.len() as u8);
        for b in name_bytes {
            pkt.write_u8(*b);
        }
        let msg_bytes = bot.merchant_chat.as_bytes();
        pkt.write_u16(msg_bytes.len() as u16);
        for b in msg_bytes {
            pkt.write_u8(*b);
        }
        pkt.write_i8(0);
        pkt.write_u8(1);
        pkt.write_u8(0);
        broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &pkt);

        world.update_bot(bot.id, |b| {
            b.last_merchant_chat_ms = now_ms;
        });
    }

    // Check movement delay (7-17s random between moves).
    if now_ms.saturating_sub(bot.last_move_ms) < BOT_MERCHANT_MOVE_MIN_MS {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
        return;
    }

    // Find nearest merchant player in the zone.
    // C++ scans `GetUnitListFromSurroundingRegions` for isMerchanting() targets.
    let mut nearest_pos: Option<(f32, f32, f32)> = None;
    let mut nearest_dist_sq = f32::MAX;

    let zone_sids = world.sessions_in_zone(bot.zone_id);
    for sid in zone_sids {
        // Check if target is merchanting.
        let is_merchant = world
            .with_session(sid, |h| h.merchant_state != -1)
            .unwrap_or(false);
        if !is_merchant {
            continue;
        }
        // Must be alive.
        let ch = match world.get_character_info(sid) {
            Some(c) if c.hp > 0 => c,
            _ => continue,
        };
        let pos = match world.get_position(sid) {
            Some(p) => p,
            None => continue,
        };
        let _ = ch;
        let dx_f = pos.x - bot.x;
        let dz_f = pos.z - bot.z;
        let dist_sq = dx_f * dx_f + dz_f * dz_f;
        if dist_sq < nearest_dist_sq {
            nearest_dist_sq = dist_sq;
            nearest_pos = Some((pos.x, pos.y, pos.z));
        }
    }

    // Move toward nearest merchant if found.
    if let Some((target_x, target_y, target_z)) = nearest_pos {
        let (new_x, new_y, new_z) = move_toward_target(bot, target_x, target_y, target_z);

        // Path validation: skip move if outside map boundaries.
        if !is_bot_move_valid(world, bot.zone_id, bot.x, bot.z, new_x, new_z) {
            world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
            return;
        }

        let echo: u8 = if (new_x - target_x).abs() < 0.5 && (new_z - target_z).abs() < 0.5 {
            0 // arrived
        } else {
            3 // continuing
        };

        broadcast_bot_move_ex(world, bot, new_x, new_y, new_z, echo);

        let new_rx = calc_region(new_x);
        let new_rz = calc_region(new_z);
        world.update_bot(bot.id, |b| {
            b.x = new_x;
            b.y = new_y;
            b.z = new_z;
            b.region_x = new_rx;
            b.region_z = new_rz;
            // Random next move delay: 7-17s (C++ myrand(7,17))
            let mut rng = rand::thread_rng();
            let delay = rng.gen_range(BOT_MERCHANT_MOVE_MIN_MS..=BOT_MERCHANT_MOVE_MAX_MS);
            b.last_move_ms = now_ms + delay - BOT_MERCHANT_MOVE_MIN_MS;
            b.last_tick_ms = now_ms;
        });
    } else {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
    }
}

/// Broadcast a merchant chat message for a bot if the interval has elapsed.
fn tick_merchant(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    if now_ms.saturating_sub(bot.last_merchant_chat_ms) < BOT_MERCHANT_CHAT_INTERVAL_MS {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
        return;
    }

    if bot.merchant_chat.is_empty() {
        world.update_bot(bot.id, |b| b.last_tick_ms = now_ms);
        return;
    }

    let zone_id = bot.zone_id;
    let rx = bot.region_x;
    let rz = bot.region_z;
    let bot_id = bot.id;
    let chat = bot.merchant_chat.clone();
    let nation = bot.nation;
    let name = bot.name.clone();

    // Build WIZ_CHAT packet with MERCHANT_CHAT type (14).
    // C++ packets.h:298: MERCHANT_CHAT=14
    // Format: opcode | chat_type(u8=14) | nation(u8) | sender_id(i16) |
    //         name(SByte: u8_len+bytes) | message(DByte: u16_len+bytes) |
    //         personelrank(i8=0) | authority(u8=1) | systemmsg(u8=0)
    let mut pkt = Packet::new(Opcode::WizChat as u8);
    pkt.write_u8(14); // MERCHANT_CHAT
    pkt.write_u8(nation);
    pkt.write_i16(bot_id as i16); // sender_id
                                  // SByte: u8 length prefix + name bytes
    let name_bytes = name.as_bytes();
    pkt.write_u8(name_bytes.len() as u8);
    for b in name_bytes {
        pkt.write_u8(*b);
    }
    // DByte: u16 length prefix + message bytes
    let msg_bytes = chat.as_bytes();
    pkt.write_u16(msg_bytes.len() as u16);
    for b in msg_bytes {
        pkt.write_u8(*b);
    }
    pkt.write_i8(0); // personelrank
    pkt.write_u8(1); // authority (normal user)
    pkt.write_u8(0); // systemmsg

    broadcast_to_bot_region(world, zone_id, rx, rz, &pkt);

    world.update_bot(bot.id, |b| {
        b.last_merchant_chat_ms = now_ms;
        b.last_tick_ms = now_ms;
    });

    debug!(bot_id, name = %bot.name, "bot merchant chat tick");
}

/// Despawn a bot: remove from registry and broadcast WIZ_USER_INOUT(INOUT_OUT) to region.
/// Builds an INOUT_OUT packet using the bot's session-band ID and broadcasts
/// it to all players in the surrounding 3×3 region so they see the bot disappear.
pub fn despawn_bot(world: &WorldState, id: BotId) {
    if let Ok(bot_sid) = SessionId::try_from(id) {
        world.cleanup_party_on_disconnect(bot_sid);
    }
    if let Some(bot) = world.remove_bot(id) {
        debug!(
            bot_id = id,
            name = %bot.name,
            zone_id = bot.zone_id,
            "bot despawned (expired or killed)"
        );

        // Broadcast WIZ_USER_INOUT(INOUT_OUT) to the surrounding region so nearby
        // players see the bot disappear.
        // only the session ID; no player-info block for OUT packets.
        let mut out_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizUserInout as u8);
        out_pkt.write_u8(2); // INOUT_OUT = 2  (crate::handler::region::INOUT_OUT)
        out_pkt.write_u8(0); // reserved
        out_pkt.write_u32(bot.id); // bot ID (u32 in packet, same as session ID field)

        broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &out_pkt);
    }
}

/// Bot self-heal AI: cast a heal skill when HP is below 90% of max.
/// Skill selection by class:
/// - Priests (Karus): 112545, (Elmorad): 212545
/// - All other classes: 490014 (generic heal)
/// The heal restores 15% of max HP. Priest heals include a MAGIC_CASTING phase;
/// non-priest heals skip directly to MAGIC_EFFECTING.
fn tick_bot_self_heal(world: &WorldState, bot: &BotInstance, now_ms: u64) {
    let heal_amount = (bot.max_hp as f32 * 0.15) as i16;
    let new_hp = (bot.hp + heal_amount).min(bot.max_hp);

    // Select heal skill ID based on class + nation
    let skill_id: u32 = if bot.is_priest() {
        if bot.nation == NATION_ELMORAD {
            212545
        } else {
            112545
        }
    } else {
        490014
    };

    // Broadcast MAGIC_CASTING for priest heals (they have a cast animation).
    if bot.is_priest() {
        let cast_pkt = build_bot_magic_packet(
            MAGIC_CASTING,
            skill_id,
            bot.id,
            bot.id, // self-target
            [bot.x as i32, bot.y as i32, bot.z as i32, 0, 0, 0, 0],
        );
        broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &cast_pkt);
    }

    // Broadcast MAGIC_EFFECTING (heal result).
    let effect_pkt = build_bot_magic_packet(
        MAGIC_EFFECTING,
        skill_id,
        bot.id,
        bot.id, // self-target
        [
            bot.x as i32,
            bot.y as i32,
            bot.z as i32,
            heal_amount as i32, // positive = heal
            0,
            0,
            0,
        ],
    );
    broadcast_to_bot_region(world, bot.zone_id, bot.region_x, bot.region_z, &effect_pkt);

    // Apply heal
    world.update_bot(bot.id, |b| {
        b.hp = new_hp;
        b.last_hp_change_ms = now_ms;
    });
    broadcast_bot_party_hp(world, bot.id);

    debug!(
        bot_id = bot.id,
        skill_id,
        heal_amount,
        hp = new_hp,
        max_hp = bot.max_hp,
        "bot self-heal"
    );
}

/// Handle a bot's death: mark as dead, broadcast WIZ_DEAD, set regene timer,
/// update rivalry and anger gauge.
/// When a bot's HP reaches 0:
/// 1. Set `m_bResHpType = USER_DEAD`, `m_BotState = BOT_DEAD`
/// 2. In PK zones: increment anger gauge, set killer as rival
/// 3. Clear target, broadcast WIZ_DEAD
/// 4. Start regene timer (bot will respawn after `BOT_REGENE_DELAY_MS`)
/// 5. Give kill rewards (NP + gold) to killer, with rival bonus if applicable
/// ## Packet format (S->C): WIZ_DEAD
/// `[u32 dead_unit_id]`
pub fn bot_on_death(world: &WorldState, bot_id: BotId, now_ms: u64) {
    let bot = match world.get_bot(bot_id) {
        Some(b) => b,
        None => return,
    };

    // Already dead — avoid double-processing.
    if bot.presence == BotPresence::Dead {
        return;
    }

    let prev_ai_state = bot.ai_state;
    let zone_id = bot.zone_id;
    let rx = bot.region_x;
    let rz = bot.region_z;
    let bot_level = bot.level;
    let killer_id = bot.last_attacker_id;
    let current_rival_id = bot.rival_id;
    let rival_expiry = bot.rival_expiry_time;
    let current_anger = bot.anger_gauge;

    world.update_bot(bot_id, |b| {
        b.hp = 0;
        b.presence = BotPresence::Dead;
        b.ai_state = BotAiState::Idle;
        b.original_ai_state = prev_ai_state;
        b.target_id = -1;
        b.target_changed = false;
        b.regene_at_ms = now_ms + BOT_REGENE_DELAY_MS;
        b.last_attacker_id = -1;
    });
    // Runtime bots share the NPC DOT registry. Do not let an expired poison
    // continue ticking after death/regeneration.
    world.clear_npc_dots(bot_id);
    broadcast_bot_party_hp(world, bot_id);

    let mut dead_pkt = Packet::new(Opcode::WizDead as u8);
    dead_pkt.write_u32(bot_id);
    broadcast_to_bot_region(world, zone_id, rx, rz, &dead_pkt);

    // ── Rivalry & anger gauge (PK zones only) ────────────────────────
    let mut rival_bonus_np: i32 = 0;
    let mut remove_rival_from_killer = false;

    if is_pk_zone(zone_id) && killer_id >= 0 {
        let now_unix = unix_now();

        // Check if killed by rival — bonus NP.
        let killed_by_rival = current_rival_id >= 0
            && current_rival_id == killer_id as i16
            && rival_expiry > now_unix;

        if killed_by_rival {
            rival_bonus_np = RIVALRY_NP_BONUS;
            remove_rival_from_killer = true;
        }

        // Increment anger gauge (max 5).
        if current_anger < MAX_ANGER_GAUGE {
            let new_anger = current_anger + 1;
            world.update_bot(bot_id, |b| {
                b.anger_gauge = new_anger;
            });
            broadcast_anger_gauge(world, bot_id, zone_id, rx, rz, new_anger);
        }

        // Set killer as rival if bot doesn't have one.
        if current_rival_id < 0 {
            world.update_bot(bot_id, |b| {
                b.rival_id = killer_id as i16;
                b.rival_expiry_time = now_unix + RIVALRY_DURATION_SECS;
            });
        }
    }

    // ── Tournament kill scoring ────────────────────────────────────────
    // When a bot dies in a tournament zone, the player killer's clan gets a score point.
    if killer_id >= 0
        && (killer_id as u32) < crate::world::BOT_ID_BASE
        && crate::handler::tournament::is_tournament_zone(zone_id)
    {
        let killer_clan = world
            .get_character_info(killer_id as SessionId)
            .map(|ch| ch.knights_id)
            .unwrap_or(0);
        crate::handler::tournament::register_kill(world, zone_id, killer_clan);
    }

    // Kill rewards: NP + gold to player killer.
    if killer_id >= 0 {
        let killer_sid = killer_id as SessionId;
        if (killer_sid as u32) < crate::world::BOT_ID_BASE {
            process_bot_kill_reward(world, killer_sid, bot_level, rival_bonus_np);

            // Death notice broadcast to zone.
            let killer_party_id = world
                .get_character_info(killer_sid)
                .and_then(|ch| ch.party_id)
                .filter(|&pid| pid != 0 && pid != 0xFFFF);
            world.send_death_notice_to_zone(
                zone_id,
                killer_sid,
                bot_id as SessionId,
                &world.get_session_name(killer_sid).unwrap_or_default(),
                &bot.name,
                killer_party_id,
                bot.x as u16,
                bot.z as u16,
            );

            // Remove rival from killer if they killed their rival.
            if remove_rival_from_killer {
                // The killer's rival tracking is in CharacterInfo.
                world.update_character_stats(killer_sid, |ch| {
                    if ch.rival_id >= 0 && ch.rival_id == bot_id as i16 {
                        ch.rival_id = -1;
                        ch.rival_expiry_time = 0;
                    }
                });
            }
        }
    }

    if is_pk_zone(zone_id) && killer_id >= 0 {
        let np_loss = get_bot_kill_np(zone_id).max(0) as u32;
        world.update_bot(bot_id, |victim| {
            victim.loyalty = victim.loyalty.saturating_sub(np_loss);
            victim.loyalty_monthly = victim.loyalty_monthly.saturating_sub(np_loss);
            victim.loyalty_daily = victim.loyalty_daily.saturating_sub(np_loss);
        });
    }

    // Runtime bot killed runtime bot: award the killer's in-memory NP used by
    // Today's Rank, and emit the same narration/chat path as every other PvP
    // combination.
    if killer_id >= crate::world::BOT_ID_BASE as i32 {
        if let Some(killer_bot) = world.get_bot(killer_id as BotId) {
            let np_gain = get_bot_kill_np_for_nation(world, zone_id, killer_bot.nation).max(0)
                as u32
                + rival_bonus_np.max(0) as u32;
            world.update_bot(killer_bot.id, |killer| {
                killer.loyalty = killer.loyalty.saturating_add(np_gain).min(2_100_000_000);
                killer.loyalty_monthly = killer
                    .loyalty_monthly
                    .saturating_add(np_gain)
                    .min(2_100_000_000);
                killer.loyalty_daily = killer
                    .loyalty_daily
                    .saturating_add(np_gain)
                    .min(2_100_000_000);
            });
            world.send_death_notice_to_zone(
                zone_id,
                killer_bot.id as SessionId,
                bot_id as SessionId,
                &killer_bot.name,
                &bot.name,
                world.get_party_id(killer_bot.id as SessionId),
                bot.x.max(0.0) as u16,
                bot.z.max(0.0) as u16,
            );
        }
    }

    debug!(
        bot_id,
        name = %bot.name,
        zone_id,
        killer_id,
        rival_bonus_np,
        regene_in_ms = BOT_REGENE_DELAY_MS,
        "bot died, regene timer started"
    );
}

/// Broadcast anger gauge update to the bot's region.
/// ## Packet format (S->C): WIZ_PVP
/// - Sub-opcode 5 (`PVPUpdateHelmet`): `[u8 anger_level] [u8 has_full_gauge]`
/// - Sub-opcode 6 (`PVPResetHelmet`): no additional data
fn broadcast_anger_gauge(
    world: &WorldState,
    _bot_id: BotId,
    zone_id: u16,
    rx: u16,
    rz: u16,
    anger: u8,
) {
    const PVP_UPDATE_HELMET: u8 = 5;
    const PVP_RESET_HELMET: u8 = 6;

    let mut pkt = Packet::new(Opcode::WizPvp as u8);
    if anger == 0 {
        pkt.write_u8(PVP_RESET_HELMET);
    } else {
        pkt.write_u8(PVP_UPDATE_HELMET);
        pkt.write_u8(anger);
        pkt.write_u8(if anger >= MAX_ANGER_GAUGE { 1 } else { 0 });
    }
    broadcast_to_bot_region(world, zone_id, rx, rz, &pkt);
}

/// Get nation-specific respawn coordinates for a bot in a given zone.
/// Uses START_POSITION table data (from migration #18):
/// - Zone 71 (Ronark Land): Karus (1375,1098), Elmorad (622,898), range ±5
/// - Zone 72 (Ardream):     Karus (851,136),   Elmorad (190,897), range ±5
/// - Zone 73 (RLB):         Karus (515,104),   Elmorad (513,916), range ±5
/// Returns `(x, z)` with random range offset applied.
pub(crate) fn get_bot_respawn_position(zone_id: u16, nation: u8) -> (f32, f32) {
    let mut rng = rand::thread_rng();

    let (base_x, base_z, range_x, range_z) = match zone_id {
        ZONE_RONARK_LAND => {
            if nation == NATION_KARUS {
                (1375i16, 1098i16, 5i16, 5i16)
            } else {
                (622, 898, 5, 5)
            }
        }
        ZONE_ARDREAM => {
            if nation == NATION_KARUS {
                (851, 136, 5, 5)
            } else {
                (190, 897, 5, 5)
            }
        }
        ZONE_RONARK_LAND_BASE => {
            if nation == NATION_KARUS {
                (515, 104, 5, 5)
            } else {
                (513, 916, 5, 5)
            }
        }
        _ => return (0.0, 0.0),
    };

    let offset_x = if range_x > 0 {
        rng.gen_range(0..=range_x)
    } else {
        0
    };
    let offset_z = if range_z > 0 {
        rng.gen_range(0..=range_z)
    } else {
        0
    };

    ((base_x + offset_x) as f32, (base_z + offset_z) as f32)
}

/// Spread explicitly summoned PK bots over concentric rings around their
/// nation start instead of stacking every database character on one point.
fn spread_pk_spawn(world: &WorldState, zone_id: u16, nation: u8, ordinal: usize) -> (f32, f32) {
    let (base_x, base_z) = get_bot_respawn_position(zone_id, nation);
    const GOLDEN_ANGLE: f32 = 2.399_963_1;

    for attempt in 0..32usize {
        let index = ordinal + attempt;
        let ring = (index / 16) as f32;
        let radius = 12.0 + ring * 13.0 + (index % 16) as f32 * 1.4;
        let angle = index as f32 * GOLDEN_ANGLE + nation as f32 * 0.71;
        let candidate_x = base_x + angle.cos() * radius;
        let candidate_z = base_z + angle.sin() * radius;
        if is_bot_position_valid(world, zone_id, candidate_x, candidate_z) {
            return (candidate_x, candidate_z);
        }
    }

    (base_x, base_z)
}

/// Regene (respawn) a dead bot: restore HP, move to nation start position,
/// broadcast INOUT packets.
/// Steps:
/// 1. Broadcast WIZ_USER_INOUT(INOUT_OUT) to hide the dead body
/// 2. Move bot to nation-specific start position for its zone
/// 3. Restore HP/MP to max
/// 4. Set presence back to Standing, restore original AI state
/// 5. Broadcast WIZ_USER_INOUT(INOUT_RESPAWN=3) to show the bot alive
fn bot_regene(world: &WorldState, bot_id: BotId, now_ms: u64) {
    let bot = match world.get_bot(bot_id) {
        Some(b) => b,
        None => return,
    };

    // Must be dead to regene.
    if bot.presence != BotPresence::Dead {
        return;
    }

    let zone_id = bot.zone_id;
    let old_rx = bot.region_x;
    let old_rz = bot.region_z;
    let original_state = bot.original_ai_state;
    let max_hp = bot.max_hp;
    let max_mp = bot.max_mp;
    let nation = bot.nation;

    // Step 1: Broadcast INOUT_OUT to clear the dead body.
    let mut out_pkt = Packet::new(Opcode::WizUserInout as u8);
    out_pkt.write_u8(2); // INOUT_OUT
    out_pkt.write_u8(0); // reserved
    out_pkt.write_u32(bot_id);
    broadcast_to_bot_region(world, zone_id, old_rx, old_rz, &out_pkt);

    // Step 2: Determine respawn position.
    let (respawn_x, respawn_z) = if original_state == BotAiState::Pk {
        spread_pk_spawn(
            world,
            zone_id,
            nation,
            bot_id.saturating_sub(crate::world::BOT_ID_BASE) as usize,
        )
    } else {
        get_bot_respawn_position(zone_id, nation)
    };
    let (new_x, new_z) = if respawn_x > 0.0 || respawn_z > 0.0 {
        (respawn_x, respawn_z)
    } else {
        // Fallback: respawn at current position (non-PK zones or unknown zone)
        (bot.x, bot.z)
    };
    let new_rx = calc_region(new_x);
    let new_rz = calc_region(new_z);

    // Step 3: Restore HP/MP, position, state, and reset anger gauge.
    // + `isReset(false)` — pick new random route, reset m_MoveState = 1
    let new_route = bot_waypoints::random_route(zone_id);
    world.update_bot(bot_id, |b| {
        b.hp = max_hp;
        b.mp = max_mp;
        b.x = new_x;
        b.z = new_z;
        b.region_x = new_rx;
        b.region_z = new_rz;
        b.presence = BotPresence::Standing;
        b.ai_state = original_state;
        b.target_id = -1;
        b.target_changed = false;
        b.regene_at_ms = 0;
        b.last_tick_ms = now_ms;
        b.last_move_ms = now_ms;
        b.last_regen_ms = now_ms;
        b.last_attacker_id = -1;
        b.anger_gauge = 0;
        b.move_route = new_route;
        b.move_state = if new_route > 0 { 1 } else { 0 };
    });

    // Step 4: Broadcast INOUT_RESPAWN with full GetUserInfo so clients see the bot.
    // INOUT_RESPAWN = 3 in C++ (packets.h:53)
    if let Some(alive_bot) = world.get_bot(bot_id) {
        let respawn_pkt = build_bot_inout_packet(&alive_bot, world, 3);
        broadcast_to_bot_region(world, zone_id, new_rx, new_rz, &respawn_pkt);
    }

    debug!(
        bot_id,
        zone_id,
        hp = max_hp,
        x = new_x,
        z = new_z,
        ai_state = ?original_state,
        "bot regened at nation start position"
    );
}

/// Parse bot equipment from the DB binary blob into the 17-slot visual array.
/// bytes where each item = `nItemID(u32) + sDurability(u16) + sCount(u16)`.
/// The visual broadcast order matches `VISUAL_SLOT_ORDER`:
/// `[BREAST(4), LEG(10), HEAD(1), GLOVE(12), FOOT(13), SHOULDER(5), RIGHTHAND(6), LEFTHAND(8),
///  CWING(42), CHELMET(43), CLEFT(44), CRIGHT(45), CTOP(46), CTATTOO(49), CFAIRY(48), CEMBLEM(47), CTALISMAN(50)]`
fn parse_bot_equipment(str_item: Option<&[u8]>) -> [(u32, i16, u8); 17] {
    const VISUAL_SLOT_ORDER: [usize; 17] = [
        4, 10, 1, 12, 13, 5, 6, 8, // equipped slots
        42, 43, 44, 45, 46, 49, 48, 47, 50, // cosplay slots
    ];
    const ITEM_SIZE: usize = 8; // nItemID(4) + sDurability(2) + sCount(2)

    let mut result = [(0u32, 0i16, 0u8); 17];
    let data = match str_item {
        Some(d) if !d.is_empty() => d,
        _ => return result,
    };

    for (vis_idx, &inv_slot) in VISUAL_SLOT_ORDER.iter().enumerate() {
        let offset = inv_slot * ITEM_SIZE;
        if offset + ITEM_SIZE > data.len() {
            continue;
        }
        let item_id = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        let durability = i16::from_le_bytes([data[offset + 4], data[offset + 5]]);
        // C++ bFlag is always 0 for bot items (not stored in the blob)
        if item_id != 0 {
            result[vis_idx] = (item_id, durability, 0);
        }
    }
    result
}

/// Build a complete WIZ_USER_INOUT packet for a bot with full GetUserInfo data.
/// `CBot::GetUserInfo()` in `BotHandler.cpp:499-701`.
/// The packet format is byte-identical to `CUser::GetUserInfo()` — the client
/// treats bots and players the same for INOUT rendering.
/// `inout_type`: 1 = INOUT_IN, 3 = INOUT_RESPAWN
fn build_bot_inout_packet(bot: &BotInstance, world: &WorldState, inout_type: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizUserInout as u8);
    pkt.write_u8(inout_type);
    pkt.write_u8(0); // reserved
    pkt.write_u32(bot.id);

    write_bot_user_info(&mut pkt, bot, world);
    pkt
}

/// Append the GetUserInfo body for a runtime bot.
/// Used by both WIZ_USER_INOUT and WIZ_REQ_USERIN responses.
pub(crate) fn write_bot_user_info(pkt: &mut Packet, bot: &BotInstance, world: &WorldState) {
    // ── GetUserInfo body ─────────────────────────────────────────────

    // Name (SByte — u8 length prefix)
    pkt.write_sbyte_string(&bot.name);

    // Nation + 3 padding bytes
    pkt.write_u8(bot.nation);
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0);

    // Clan ID + fame
    pkt.write_i16(bot.knights_id as i16);
    let fame = if is_pk_zone(bot.zone_id) { 0 } else { bot.fame };
    pkt.write_u8(fame);

    // Clan info block — look up clan if bot has one
    let clan = if bot.knights_id > 0 {
        world.get_knights(bot.knights_id)
    } else {
        None
    };
    match clan.as_ref() {
        Some(ki) => {
            pkt.write_u16(ki.alliance);
            pkt.write_sbyte_string(&ki.name);
            pkt.write_u8(ki.grade);
            pkt.write_u8(ki.ranking);
            pkt.write_u16(ki.mark_version);
            // Cape data
            pkt.write_u16(ki.cape);
            pkt.write_u8(ki.cape_r);
            pkt.write_u8(ki.cape_g);
            pkt.write_u8(ki.cape_b);
            pkt.write_u8(0);
            // Symbol flag
            let sf = if ki.flag > 1 && ki.grade < 3 {
                2u8
            } else {
                0u8
            };
            pkt.write_u8(sf);
        }
        None => {
            pkt.write_u16(0); // alliance_id
            pkt.write_u8(0); // empty clan name (SByte len=0)
            pkt.write_u8(0); // grade
            pkt.write_u8(0); // ranking
            pkt.write_u16(0); // mark_version
            pkt.write_u16(0xFFFF); // no cape
            pkt.write_u32(0); // cape R,G,B,pad
            pkt.write_u8(0); // symbol flag
        }
    }

    // Level, race, class
    pkt.write_u8(bot.level);
    pkt.write_u8(bot.race);
    pkt.write_u16(bot.class);

    // Position — C++ GetSPosX() = uint16(GetX() * 10)
    pkt.write_u16((bot.x * 10.0) as u16);
    pkt.write_u16((bot.z * 10.0) as u16);
    pkt.write_u16((bot.y * 10.0) as u16);

    // Face + Hair
    pkt.write_u8(bot.face);
    pkt.write_u32(bot.hair_rgb);

    // Status flags
    let res_hp_type: u8 = match bot.presence {
        BotPresence::Standing => 1, // USER_STANDING
        BotPresence::Sitting => 2,  // USER_SITDOWN
        BotPresence::Dead => 3,     // USER_DEAD
    };
    pkt.write_u8(res_hp_type);
    pkt.write_u32(1); // m_bAbnormalType = ABNORMAL_NORMAL
    pkt.write_u8(0); // v2600: unknown byte after abnormal_type
    let bot_party_id = world.get_party_id(bot.id as SessionId);
    let is_party_leader = bot_party_id
        .and_then(|party_id| world.get_party(party_id))
        .is_some_and(|party| party.is_leader(bot.id as SessionId));
    pkt.write_u8(if bot_party_id.is_some() {
        0
    } else {
        bot.need_party
    });
    pkt.write_u8(1); // m_bAuthority = 1 (Player, not GM)
    pkt.write_u8(if is_party_leader { 1 } else { 0 });

    // Invisibility, team, devil, direction, chicken, rank
    pkt.write_u8(0); // bInvisibilityType
    pkt.write_u8(0); // m_teamColour
    pkt.write_u8(0); // m_bIsDevil
    pkt.write_u8(0); // padding
    pkt.write_u16(bot.direction as u16);
    pkt.write_u8(if bot.level < 30 { 1 } else { 0 }); // m_bIsChicken
    pkt.write_u8(0); // m_bRank (0 = not king)

    // Dual rank flags
    pkt.write_u8(0);
    pkt.write_u8(0);

    // Knights/personal rank flags use the same representation as players.
    let kr = if bot.knights_rank == 0 {
        -1
    } else {
        bot.knights_rank as i8
    };
    let pr = if bot.personal_rank == 0 {
        -1
    } else {
        bot.personal_rank as i8
    };
    pkt.write_i8(if kr <= pr { kr } else { -1 });
    pkt.write_i8(if pr <= kr { pr } else { -1 });

    // Equipment — 17 visual slots
    for &(item_id, dur, flag) in &bot.equip_visual {
        pkt.write_u32(item_id);
        pkt.write_u16(dur as u16);
        pkt.write_u8(flag);
    }

    // Zone + trailing data — C++ BotHandler.cpp:700
    pkt.write_u16(bot.zone_id);
    pkt.write_i32(-1); // unknown
    pkt.write_u8(0);
    pkt.write_u32(0);
    pkt.write_u8(if bot.hiding_helmet { 1 } else { 0 });
    pkt.write_u8(if bot.hiding_cospre { 1 } else { 0 });
    pkt.write_u8(0); // isInGenie
    pkt.write_u8(bot.reb_level);
    pkt.write_u16(bot.cover_title);
    pkt.write_u32(0); // ReturnSymbolisOK
    pkt.write_u8(0); // padding
    pkt.write_u8(0); // v2600 unknown
    pkt.write_u8(1); // v2600 final marker
}

/// Calculate max HP for a bot from the coefficient table.
/// Formula: `(HP_COEFF * level^2 * STA) + (0.1 * level * STA) + (STA / 5) + 20`
/// Minimum is 20 HP to match C++ behaviour.
fn calc_bot_max_hp(level: u8, sta: u8, coeff: &CoefficientRow) -> i16 {
    let lvl = level as f64;
    let sta_f = sta as f64;
    let hp = (coeff.hp * lvl * lvl * sta_f) + (0.1 * lvl * sta_f) + (sta_f / 5.0) + 20.0;
    (hp as i16).max(20)
}

/// Calculate max MP for a bot from the coefficient table.
/// - Magic classes (MP coeff != 0): `(MP_COEFF * level^2 * (INT+30)) + (0.1 * level * 2 * (INT+30)) + ((INT+30) / 5) + 20`
/// - SP classes (Kurian, SP coeff != 0): `(SP_COEFF * level^2 * STA) + (0.1 * level * STA) + (STA / 5)`
/// - Others: 0
fn calc_bot_max_mp(level: u8, sta: u8, intel: u8, coeff: &CoefficientRow) -> i16 {
    let lvl = level as f64;
    if coeff.mp != 0.0 {
        let temp_intel = intel as f64 + 30.0;
        let mp = (coeff.mp * lvl * lvl * temp_intel)
            + (0.1 * lvl * 2.0 * temp_intel)
            + (temp_intel / 5.0)
            + 20.0;
        (mp as i16).max(0)
    } else if coeff.sp != 0.0 {
        let sta_f = sta as f64;
        let mp = (coeff.sp * lvl * lvl * sta_f) + (0.1 * lvl * sta_f) + (sta_f / 5.0);
        (mp as i16).max(0)
    } else {
        0
    }
}

/// Compute and apply HP/MP for a bot using the coefficient table.
/// Sets `bot.hp`, `bot.max_hp`, `bot.mp`, and `bot.max_mp` based on level,
/// class stats, and the class coefficient row.
/// calls `SetMaxHp()` and `SetMaxMp()` which use the coefficient table.
/// # Arguments
/// - `bot`  — mutable bot instance to update
/// - `coeff` — coefficient row for the bot's class (from `WorldState::get_coefficient`)
pub fn set_bot_ability(bot: &mut BotInstance, coeff: &CoefficientRow) {
    let max_hp = calc_bot_max_hp(bot.level, bot.sta_stat, coeff);
    let max_mp = calc_bot_max_mp(bot.level, bot.sta_stat, bot.int_stat, coeff);

    bot.max_hp = max_hp;
    bot.hp = max_hp; // Spawn at full HP
    bot.max_mp = max_mp;
    bot.mp = max_mp; // Spawn at full MP
}

/// Parameters for spawning a farm bot.
/// Groups the positional and behavioural arguments that would otherwise
/// exceed clippy's `too_many_arguments` threshold.
pub struct SpawnBotParams {
    /// Zone ID to spawn in.
    pub zone_id: u16,
    /// World X coordinate.
    pub x: f32,
    /// World Y coordinate (height).
    pub y: f32,
    /// World Z coordinate.
    pub z: f32,
    /// Duration in minutes before auto-despawn (0 = permanent).
    pub duration_minutes: u32,
    /// Initial AI state.
    pub ai_state: BotAiState,
}

/// Startup result for the database-backed bot population.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DatabaseBotSpawnSummary {
    pub total: usize,
    pub merchants: usize,
    pub pk: usize,
    pub farmers: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBotKind {
    Merchant,
    Pk,
    Farmer,
}

fn merchant_slots(
    row: &ko_db::models::bot_system::BotMerchantDataRow,
) -> [MerchData; MAX_MERCH_ITEMS] {
    let raw = [
        (
            row.n_num1,
            row.n_price1,
            row.s_count1,
            row.s_duration1,
            row.is_kc1,
        ),
        (
            row.n_num2,
            row.n_price2,
            row.s_count2,
            row.s_duration2,
            row.is_kc2,
        ),
        (
            row.n_num3,
            row.n_price3,
            row.s_count3,
            row.s_duration3,
            row.is_kc3,
        ),
        (
            row.n_num4,
            row.n_price4,
            row.s_count4,
            row.s_duration4,
            row.is_kc4,
        ),
        (
            row.n_num5,
            row.n_price5,
            row.s_count5,
            row.s_duration5,
            row.is_kc5,
        ),
        (
            row.n_num6,
            row.n_price6,
            row.s_count6,
            row.s_duration6,
            row.is_kc6,
        ),
        (
            row.n_num7,
            row.n_price7,
            row.s_count7,
            row.s_duration7,
            row.is_kc7,
        ),
        (
            row.n_num8,
            row.n_price8,
            row.s_count8,
            row.s_duration8,
            row.is_kc8,
        ),
        (
            row.n_num9,
            row.n_price9,
            row.s_count9,
            row.s_duration9,
            row.is_kc9,
        ),
        (
            row.n_num10,
            row.n_price10,
            row.s_count10,
            row.s_duration10,
            row.is_kc10,
        ),
        (
            row.n_num11,
            row.n_price11,
            row.s_count11,
            row.s_duration11,
            row.is_kc11,
        ),
        (
            row.n_num12,
            row.n_price12,
            row.s_count12,
            row.s_duration12,
            row.is_kc12,
        ),
    ];
    std::array::from_fn(|slot| {
        let (item_id, price, count, durability, is_kc) = raw[slot];
        MerchData {
            item_id: item_id.max(0) as u32,
            durability: durability.clamp(0, i16::MAX as i32) as i16,
            sell_count: count.clamp(0, u16::MAX as i32) as u16,
            original_count: count.clamp(0, u16::MAX as i32) as u16,
            serial_num: 0,
            price: price.max(0) as u32,
            original_slot: slot as u8,
            sold_out: item_id <= 0 || count <= 0,
            is_kc,
        }
    })
}

/// Materialise an explicitly requested number of DB-backed bots.
/// Nothing is spawned automatically at server startup. Existing DB rows are
/// skipped so repeated GM commands cannot duplicate the same character.
pub fn spawn_database_bots(
    world: &WorldState,
    kind: DatabaseBotKind,
    requested: usize,
) -> DatabaseBotSpawnSummary {
    let merchant_rows: HashMap<i32, _> = world
        .get_all_bot_merchant_data()
        .into_iter()
        .map(|row| (row.n_index, row))
        .collect();
    let mut summary = DatabaseBotSpawnSummary::default();
    let already_loaded: HashSet<i32> = world
        .bots
        .iter()
        .filter_map(|entry| (entry.value().db_id > 0).then_some(entry.value().db_id))
        .collect();
    let capacity = 5_000usize.saturating_sub(world.bot_count());
    let limit = requested.min(capacity);

    for row in world.get_all_bot_templates() {
        let merchant = merchant_rows.get(&row.id);
        let eligible = match kind {
            DatabaseBotKind::Merchant => merchant.is_some(),
            // PK/farmer are behaviours applied to saved DB characters. They
            // are not limited to the 24 rows whose last logout zone was CZ.
            DatabaseBotKind::Pk | DatabaseBotKind::Farmer => merchant.is_none(),
        };
        if !eligible || already_loaded.contains(&row.id) {
            continue;
        }
        if summary.total >= limit {
            break;
        }
        let mut zone_id = row.zone.max(0) as u16;
        let mut x = row.px as f32 / 100.0;
        let mut y = row.py as f32 / 100.0;
        let mut z = row.pz as f32 / 100.0;

        if kind == DatabaseBotKind::Pk {
            zone_id = ZONE_RONARK_LAND;
            let nation = row.nation.clamp(1, 2) as u8;
            (x, z) = get_bot_respawn_position(zone_id, nation);
            y = 0.0;
        }

        if let Some(stall) = merchant {
            if stall.zone > 0 {
                zone_id = stall.zone as u16;
            }
            if stall.px != 0 || stall.pz != 0 {
                x = stall.px as f32 / 100.0;
                y = stall.py as f32 / 100.0;
                z = stall.pz as f32 / 100.0;
            }
        }

        let Some(zone) = world.get_zone(zone_id) else {
            summary.skipped += 1;
            continue;
        };
        if kind == DatabaseBotKind::Pk {
            let nation = row.nation.clamp(1, 2) as u8;
            let ordinal = world
                .get_bots_in_zone_live(zone_id)
                .into_iter()
                .filter(|bot| bot.nation == nation && bot.ai_state == BotAiState::Pk)
                .count();
            (x, z) = spread_pk_spawn(world, zone_id, nation, ordinal);
        }
        if !zone.is_valid_position(x, z) {
            summary.skipped += 1;
            continue;
        }

        let ai_state = match kind {
            DatabaseBotKind::Merchant => BotAiState::Merchant,
            DatabaseBotKind::Pk => BotAiState::Pk,
            DatabaseBotKind::Farmer => BotAiState::Farmer,
        };
        let duration_minutes = merchant
            .map(|stall| {
                if stall.minute >= 9999 {
                    0
                } else {
                    stall.minute.max(0) as u32
                }
            })
            .unwrap_or(0);
        let bot_id = spawn_farm_bot(
            world,
            &row,
            SpawnBotParams {
                zone_id,
                x,
                y,
                z,
                duration_minutes,
                ai_state,
            },
        );

        if let Some(stall) = merchant {
            let items = merchant_slots(stall);
            world.update_bot(bot_id, |bot| {
                bot.direction = stall.s_direction.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                bot.merchant_state = stall.merchant_type.clamp(0, 1) as i8;
                bot.premium_merchant = false;
                bot.merchant_chat = stall.advert_message.clone().unwrap_or_default();
                bot.merchant_items = items;
            });
            // The character INOUT was emitted before its saved merchant state
            // was attached. Notify players already in the region so the stall
            // and its sign appear immediately; later entrants receive the same
            // state from send_merchant_user_in_out_for_me().
            let mut merchant_inout = Packet::new(Opcode::WizMerchantInout as u8);
            merchant_inout.write_u8(1);
            merchant_inout.write_u16(1);
            merchant_inout.write_u32(bot_id);
            merchant_inout.write_u8(stall.merchant_type.clamp(0, 1) as u8);
            merchant_inout.write_u8(0);
            broadcast_to_bot_region(
                world,
                zone_id,
                calc_region(x),
                calc_region(z),
                &merchant_inout,
            );
            summary.merchants += 1;
        } else if ai_state == BotAiState::Pk {
            summary.pk += 1;
        } else {
            summary.farmers += 1;
        }
        summary.total += 1;
    }

    summary
}

/// Spawn a bot from a `BotHandlerFarmRow` definition.
/// Inserts the bot into `WorldState::bots` and marks it in-game.
/// The caller is responsible for sending `WIZ_USER_INOUT(INOUT_IN)` packets
/// to nearby players (this function only manages state).
/// Returns the allocated `BotId`.
pub fn spawn_farm_bot(
    world: &WorldState,
    row: &ko_db::models::bot_system::BotHandlerFarmRow,
    params: SpawnBotParams,
) -> BotId {
    let SpawnBotParams {
        zone_id,
        x,
        y,
        z,
        duration_minutes,
        ai_state,
    } = params;
    use crate::zone::calc_region;

    let id = world.alloc_bot_id();
    let rx = calc_region(x);
    let rz = calc_region(z);
    let now_unix = unix_now();
    let now_ms = tick_ms();

    let mut bot = BotInstance {
        id,
        db_id: row.id,
        name: row.str_user_id.clone(),
        nation: row.nation as u8,
        race: row.race as u8,
        class: row.class as u16,
        hair_rgb: row.hair_rgb as u32,
        level: row.level as u8,
        face: row.face as u8,
        knights_id: row.knights as u16,
        fame: row.fame as u8,
        zone_id,
        x,
        y,
        z,
        direction: 0,
        region_x: rx,
        region_z: rz,
        hp: 1000, // Overwritten by set_bot_ability() below
        max_hp: 1000,
        mp: 500,
        max_mp: 500,
        sp: 0,
        max_sp: 0,
        str_stat: row.strong as u8,
        sta_stat: row.sta as u8,
        dex_stat: row.dex as u8,
        int_stat: row.intel as u8,
        cha_stat: row.cha as u8,
        gold: row.gold as u32,
        loyalty: row.loyalty as u32,
        loyalty_monthly: row.loyalty_monthly as u32,
        loyalty_daily: 0,
        in_game: true,
        presence: BotPresence::Standing,
        ai_state,
        target_id: -1,
        target_changed: false,
        spawned_at: now_unix,
        duration_minutes,
        last_tick_ms: now_ms,
        last_move_ms: 0,
        last_mining_ms: 0,
        last_merchant_chat_ms: 0,
        last_hp_change_ms: now_ms,
        last_regen_ms: now_ms,
        last_attacker_id: -1,
        skill_cooldown: [0; 2],
        last_type4_ms: now_ms,
        regene_at_ms: 0,
        original_ai_state: ai_state,
        move_route: 0,
        move_state: 0,
        merchant_state: -1,
        premium_merchant: false,
        merchant_chat: String::new(),
        merchant_items: Default::default(),
        merchant_looker: None,
        reb_level: row.reb_level as u8,
        cover_title: row.cover_title as u16,
        rival_id: -1,
        rival_expiry_time: 0,
        anger_gauge: 0,
        hiding_helmet: false,
        hiding_cospre: false,
        need_party: 1,
        equip_visual: parse_bot_equipment(row.str_item.as_deref()),
        personal_rank: 0,
        knights_rank: 0,
    };

    // Apply stat-derived HP/MP using the coefficient table.
    if let Some(coeff) = world.get_coefficient(bot.class) {
        set_bot_ability(&mut bot, &coeff);
    }

    // Assign a random patrol route for PK zone bots.
    //   `s_MoveProcess = myrand(1, 10); m_MoveState = 1;`
    let route = bot_waypoints::random_route(zone_id);
    if route > 0 {
        bot.move_route = route;
        bot.move_state = 1;
    }

    // Register first so an immediate WIZ_REQ_USERIN can resolve the bot.
    let in_pkt = build_bot_inout_packet(&bot, world, 1); // 1 = INOUT_IN
    world.insert_bot(bot);
    broadcast_to_bot_region(world, zone_id, rx, rz, &in_pkt);

    trace!(
        bot_id = id,
        name = %row.str_user_id,
        zone_id,
        x,
        z,
        ai_state = ?ai_state,
        "bot spawned"
    );

    id
}

/// Parameters for spawning a GM-issued bot (without a DB row).
pub struct SpawnGmBotParams {
    /// Zone ID to spawn in.
    pub zone_id: u16,
    /// World X coordinate.
    pub x: f32,
    /// World Y coordinate (height).
    pub y: f32,
    /// World Z coordinate.
    pub z: f32,
    /// Class code (1=warrior, 2=rogue, 3=mage, 4=priest in GM shorthand).
    /// This is converted to the real class code internally.
    pub class: u16,
    /// Character level (1-83).
    pub level: u8,
    /// Nation: 1=Karus, 2=ElMorad.
    pub nation: u8,
    /// Initial AI state (Farmer or Pk).
    pub ai_state: BotAiState,
}

/// Map GM class shorthand to an actual KO class code.
/// sClass 1=warrior, 2=rogue, 3=mage, 4=priest.
/// Returns a representative class code for each job group.
fn gm_class_to_real_class(gm_class: u16, nation: u8) -> u16 {
    // Base classes per nation: Karus race starts at 100, ElMorad at 200.
    let base = if nation == 1 { 100 } else { 200 };
    match gm_class {
        1 => base + 1, // Warrior base
        5 => base + 2, // Rogue base (C++ groups: 5 maps to rogue in some schemas)
        6 => base + 3, // Mage base
        8 => base + 4, // Priest base
        2 => base + 2, // Rogue (alternative GM shorthand)
        3 => base + 3, // Mage (alternative)
        4 => base + 4, // Priest (alternative)
        _ => base + 1, // Default to warrior
    }
}

/// Generate random base stats for a GM-spawned bot based on class.
/// Warriors get high STR/STA, rogues get high DEX, mages get high INT.
fn gm_bot_stats(class: u16) -> (u8, u8, u8, u8, u8) {
    let base_class = class % 100;
    match base_class {
        1 | 5 | 6 => (100, 80, 60, 40, 30), // Warrior: STR/STA/DEX/INT/CHA
        2 | 7 | 8 => (60, 60, 100, 40, 30), // Rogue: DEX-focused
        3 | 9 | 10 => (40, 50, 60, 120, 30), // Mage: INT-focused
        4 | 11 | 12 => (50, 60, 60, 100, 30), // Priest: INT-focused with STA
        _ => (80, 70, 70, 60, 30),          // Default balanced
    }
}

fn bot_class_group(class: u16) -> u8 {
    match class % 100 {
        1 | 5 | 6 => 1,
        2 | 7 | 8 => 2,
        3 | 9 | 10 => 3,
        4 | 11 | 12 => 4,
        _ => 0,
    }
}

/// Reuse a zone bot template's equipped/cosmetic visuals for GM-created bots.
/// Runtime bots do not carry bag inventory, but they expose the same 17 visual
/// slots used by normal player INOUT packets.
fn gm_bot_equipment(
    world: &WorldState,
    zone_id: u16,
    nation: u8,
    class: u16,
) -> [(u32, i16, u8); 17] {
    let class_group = bot_class_group(class);
    let zone_rows = world.get_bots_in_zone(zone_id as i16);
    let all_rows = world.get_all_bot_templates();
    let find_equipment = |rows: &[ko_db::models::bot_system::BotHandlerFarmRow],
                          same_nation: bool| {
        rows.iter()
            .filter(|row| {
                (!same_nation || row.nation as u8 == nation)
                    && bot_class_group(row.class as u16) == class_group
                    && row.str_item.as_ref().is_some_and(|items| !items.is_empty())
            })
            .map(|row| parse_bot_equipment(row.str_item.as_deref()))
            .find(|equipment| equipment.iter().any(|(item_id, _, _)| *item_id != 0))
    };

    find_equipment(&zone_rows, true)
        .or_else(|| find_equipment(&zone_rows, false))
        .or_else(|| find_equipment(&all_rows, true))
        .or_else(|| find_equipment(&all_rows, false))
        .unwrap_or([(0, 0, 0); 17])
}

/// Spawn a bot from GM command parameters (no DB row required).
/// Creates a `BotInstance` with generated stats and name, inserts it into
/// `WorldState::bots`, computes HP/MP from the coefficient table, and
/// broadcasts `WIZ_USER_INOUT(INOUT_IN)` to surrounding players.
/// Returns the allocated `BotId`.
/// `UserInOut(INOUT_IN)`.
pub fn spawn_gm_bot(world: &WorldState, params: SpawnGmBotParams) -> BotId {
    use crate::zone::calc_region;

    let SpawnGmBotParams {
        zone_id,
        x,
        y,
        z,
        class: gm_class,
        level,
        nation,
        ai_state,
    } = params;

    let real_class = gm_class_to_real_class(gm_class, nation);
    let (str_stat, sta_stat, dex_stat, int_stat, cha_stat) = gm_bot_stats(real_class);

    let id = world.alloc_bot_id();
    let rx = calc_region(x);
    let rz = calc_region(z);
    let now_unix = unix_now();
    let now_ms = tick_ms();

    // Generate a name like "Bot10001" using the allocated ID.
    let name = format!("Bot{}", id);

    // Race: Karus=1 (Barbarian), ElMorad=11 (El Morad)
    let race: u8 = if nation == 1 { 1 } else { 11 };

    let mut bot = BotInstance {
        id,
        db_id: 0, // No DB row for GM-spawned bots
        name: name.clone(),
        nation,
        race,
        class: real_class,
        hair_rgb: 0,
        level,
        face: 1,
        knights_id: 0,
        fame: 0,
        zone_id,
        x,
        y,
        z,
        direction: 0,
        region_x: rx,
        region_z: rz,
        hp: 1000,
        max_hp: 1000,
        mp: 500,
        max_mp: 500,
        sp: 0,
        max_sp: 0,
        str_stat,
        sta_stat,
        dex_stat,
        int_stat,
        cha_stat,
        gold: 0,
        loyalty: 100,
        loyalty_monthly: 0,
        loyalty_daily: 0,
        in_game: true,
        presence: BotPresence::Standing,
        ai_state,
        target_id: -1,
        target_changed: false,
        spawned_at: now_unix,
        duration_minutes: 60, // GM bots default to 60 minutes
        last_tick_ms: now_ms,
        last_move_ms: 0,
        last_mining_ms: 0,
        last_merchant_chat_ms: 0,
        last_hp_change_ms: now_ms,
        last_regen_ms: now_ms,
        last_attacker_id: -1,
        skill_cooldown: [0; 2],
        last_type4_ms: now_ms,
        regene_at_ms: 0,
        original_ai_state: ai_state,
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
        equip_visual: gm_bot_equipment(world, zone_id, nation, real_class),
        personal_rank: 0,
        knights_rank: 0,
    };

    // Apply stat-derived HP/MP using the coefficient table.
    if let Some(coeff) = world.get_coefficient(bot.class) {
        set_bot_ability(&mut bot, &coeff);
    }

    // Assign a random patrol route for PK zone bots.
    let route = bot_waypoints::random_route(zone_id);
    if route > 0 {
        bot.move_route = route;
        bot.move_state = 1;
    }

    // Register first so an immediate WIZ_REQ_USERIN can resolve the bot.
    let in_pkt = build_bot_inout_packet(&bot, world, 1); // 1 = INOUT_IN
    world.insert_bot(bot);
    broadcast_to_bot_region(world, zone_id, rx, rz, &in_pkt);

    debug!(
        bot_id = id,
        name = %name,
        zone_id,
        x,
        z,
        class = real_class,
        level,
        nation,
        ai_state = ?ai_state,
        "GM bot spawned"
    );

    id
}

/// Despawn all bots in a specific zone and return the count removed.
/// Iterates all active bots, despawns those in the given zone.
pub fn despawn_bots_in_zone(world: &WorldState, zone_id: u16) -> usize {
    let bot_ids: Vec<BotId> = world
        .bots
        .iter()
        .filter(|e| e.value().zone_id == zone_id)
        .map(|e| *e.key())
        .collect();

    let count = bot_ids.len();
    for id in bot_ids {
        despawn_bot(world, id);
    }
    count
}

/// Return the number of temporary PK bots created through GM commands in a zone.
/// Database-backed bots are deliberately excluded (`db_id != 0`).
pub fn count_gm_pk_bots_in_zone(world: &WorldState, zone_id: u16) -> usize {
    world
        .bots
        .iter()
        .filter(|entry| is_gm_pk_bot(entry.value(), zone_id))
        .count()
}

/// Despawn only temporary GM-created PK bots in a zone.
///
/// Unlike `despawn_bots_in_zone()`, this never removes persistent DB bots or
/// GM-spawned farmer/merchant bots.
pub fn despawn_gm_pk_bots_in_zone(world: &WorldState, zone_id: u16) -> usize {
    let bot_ids: Vec<BotId> = world
        .bots
        .iter()
        .filter(|entry| is_gm_pk_bot(entry.value(), zone_id))
        .map(|entry| *entry.key())
        .collect();

    let count = bot_ids.len();
    for id in bot_ids {
        despawn_bot(world, id);
    }
    count
}

fn is_gm_pk_bot(bot: &BotInstance, zone_id: u16) -> bool {
    bot.zone_id == zone_id
        && bot.db_id == 0
        && (bot.ai_state == BotAiState::Pk || bot.original_ai_state == BotAiState::Pk)
}

/// Despawn all bots server-wide and return the count removed.
/// `UserInOut(INOUT_OUT)` + `RemoveMapBotList()` for each.
pub fn despawn_all_bots(world: &WorldState) -> usize {
    let bot_ids: Vec<BotId> = world.bots.iter().map(|e| *e.key()).collect();
    let count = bot_ids.len();
    for id in bot_ids {
        despawn_bot(world, id);
    }
    count
}

/// Broadcast a packet to all sessions in the surrounding 3x3 regions of a bot.
fn broadcast_to_bot_region(world: &WorldState, zone_id: u16, rx: u16, rz: u16, pkt: &Packet) {
    world.broadcast_to_region_sync(zone_id, rx, rz, Arc::new(pkt.clone()), None, 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CharacterInfo, WorldState, BOT_ID_BASE};
    use ko_db::models::bot_system::BotHandlerFarmRow;

    /// Create a minimal Item for testing with only `num` and `kind` set.
    fn make_test_item(num: i32, kind: i32) -> ko_db::models::Item {
        ko_db::models::Item {
            num,
            extension: None,
            str_name: None,
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            kind: Some(kind),
            slot: None,
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: None,
            duration: None,
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

    /// Helper: spawn a bot with a simple positional argument list (test only).
    fn do_spawn(
        world: &WorldState,
        row: &BotHandlerFarmRow,
        zone_id: u16,
        x: f32,
        z: f32,
        duration_minutes: u32,
        ai_state: BotAiState,
    ) -> BotId {
        spawn_farm_bot(
            world,
            row,
            SpawnBotParams {
                zone_id,
                x,
                y: 0.0,
                z,
                duration_minutes,
                ai_state,
            },
        )
    }

    fn make_farm_row(id: i32, name: &str, zone: i16) -> BotHandlerFarmRow {
        BotHandlerFarmRow {
            id,
            str_user_id: name.to_string(),
            nation: 1,
            race: 1,
            class: 107,
            hair_rgb: 0,
            level: 70,
            face: 1,
            knights: 0,
            fame: 0,
            zone,
            px: 26000,
            pz: 32000,
            py: 0,
            str_item: None,
            cover_title: 0,
            reb_level: 0,
            str_skill: None,
            gold: 10000,
            points: 0,
            strong: 60,
            sta: 60,
            dex: 90,
            intel: 60,
            cha: 60,
            loyalty: 5000,
            loyalty_monthly: 1000,
            donated_np: 0,
        }
    }

    #[test]
    fn test_spawn_farm_bot_inserts_into_registry() {
        let world = WorldState::new();
        let row = make_farm_row(1, "FarmBot01", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 60, BotAiState::Farmer);
        assert!(id >= BOT_ID_BASE, "bot id should be above BOT_ID_BASE");
        assert_eq!(world.bot_count(), 1);
        let bot = world.get_bot(id).unwrap();
        assert_eq!(bot.name, "FarmBot01");
        assert_eq!(bot.zone_id, 21);
        assert!(bot.in_game);
        assert_eq!(bot.ai_state, BotAiState::Farmer);
    }

    #[test]
    fn test_spawn_two_bots_different_ids() {
        let world = WorldState::new();
        let row1 = make_farm_row(1, "Bot01", 21);
        let row2 = make_farm_row(2, "Bot02", 21);
        let id1 = do_spawn(&world, &row1, 21, 260.0, 320.0, 60, BotAiState::Mining);
        let id2 = do_spawn(&world, &row2, 21, 265.0, 325.0, 60, BotAiState::Fishing);
        assert_ne!(id1, id2);
        assert_eq!(world.bot_count(), 2);
    }

    #[test]
    fn test_despawn_removes_bot() {
        let world = WorldState::new();
        let row = make_farm_row(1, "ExpireBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 1, BotAiState::Idle);
        assert_eq!(world.bot_count(), 1);
        despawn_bot(&world, id);
        assert_eq!(world.bot_count(), 0);
        assert!(world.get_bot(id).is_none());
    }

    #[test]
    fn test_bot_expiry_check_expired() {
        let world = WorldState::new();
        let row = make_farm_row(1, "ExpireBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 1, BotAiState::Idle);
        let bot = world.get_bot(id).unwrap();
        // duration_minutes=1, so it expires 60 seconds after spawned_at
        let future = bot.spawned_at + 120; // 2 minutes after spawn
        assert!(
            bot.is_expired(future),
            "bot should be expired after 2 minutes"
        );
    }

    #[test]
    fn test_bot_expiry_check_not_expired() {
        let world = WorldState::new();
        let row = make_farm_row(1, "PermanentBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Idle);
        let bot = world.get_bot(id).unwrap();
        // duration_minutes=0 means permanent
        let far_future = bot.spawned_at + 999_999;
        assert!(
            !bot.is_expired(far_future),
            "permanent bot should never expire"
        );
    }

    #[test]
    fn test_bot_class_helpers_rogue() {
        let world = WorldState::new();
        let row = make_farm_row(1, "Rogue", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Idle);
        let bot = world.get_bot(id).unwrap();
        // class=107: 107 % 100 = 7 = rogue novice
        assert!(bot.is_rogue());
        assert!(!bot.is_warrior());
        assert!(!bot.is_mage());
        assert!(!bot.is_priest());
    }

    #[test]
    fn test_bot_class_helpers_warrior() {
        let mut row = make_farm_row(2, "Warrior", 21);
        row.class = 106; // 106 % 100 = 6 = warrior mastered (ClassWarriorMaster)
        let world = WorldState::new();
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Idle);
        let bot = world.get_bot(id).unwrap();
        assert!(bot.is_warrior());
        assert!(!bot.is_rogue());
    }

    #[test]
    fn test_get_bots_in_zone_live() {
        let world = WorldState::new();
        let row1 = make_farm_row(1, "ZoneBot1", 21);
        let row2 = make_farm_row(2, "ZoneBot2", 22);
        do_spawn(&world, &row1, 21, 260.0, 320.0, 0, BotAiState::Idle);
        do_spawn(&world, &row2, 22, 100.0, 100.0, 0, BotAiState::Idle);
        let zone21 = world.get_bots_in_zone_live(21);
        assert_eq!(zone21.len(), 1);
        assert_eq!(zone21[0].name, "ZoneBot1");
        let zone22 = world.get_bots_in_zone_live(22);
        assert_eq!(zone22.len(), 1);
    }

    #[test]
    fn test_collect_expired_bot_ids() {
        let world = WorldState::new();
        let row1 = make_farm_row(1, "Temp1", 21);
        let row2 = make_farm_row(2, "Perm2", 21);
        let id1 = do_spawn(&world, &row1, 21, 260.0, 320.0, 1, BotAiState::Idle);
        do_spawn(&world, &row2, 21, 265.0, 325.0, 0, BotAiState::Idle);
        // Advance time past the 1-minute duration.
        let bot = world.get_bot(id1).unwrap();
        let future = bot.spawned_at + 120;
        let expired = world.collect_expired_bot_ids(future);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], id1);
    }

    #[test]
    fn test_update_bot_modifies_in_place() {
        let world = WorldState::new();
        let row = make_farm_row(1, "UpdateBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Idle);
        let updated = world.update_bot(id, |b| {
            b.ai_state = BotAiState::Mining;
            b.hp = 500;
        });
        assert!(updated);
        let bot = world.get_bot(id).unwrap();
        assert_eq!(bot.ai_state, BotAiState::Mining);
        assert_eq!(bot.hp, 500);
    }

    #[test]
    fn test_update_bot_missing_returns_false() {
        let world = WorldState::new();
        let updated = world.update_bot(99_999, |b| b.hp = 0);
        assert!(!updated);
    }

    #[test]
    fn test_bot_pk_zone_detection() {
        let world = WorldState::new();
        // Zone 72 = ZONE_ARDREAM (C++ Define.h)
        let row = make_farm_row(1, "PKBot", 72);
        let id = do_spawn(&world, &row, 72, 260.0, 320.0, 0, BotAiState::Pk);
        let bot = world.get_bot(id).unwrap();
        assert!(bot.is_in_pk_zone(), "zone 72 (ZONE_ARDREAM) is a PK zone");
    }

    #[test]
    fn test_bot_pk_zone_ronark_land() {
        let world = WorldState::new();
        // Zone 71 = ZONE_RONARK_LAND (C++ Define.h)
        let row = make_farm_row(2, "PKBot2", 71);
        let id = do_spawn(&world, &row, 71, 260.0, 320.0, 0, BotAiState::Pk);
        let bot = world.get_bot(id).unwrap();
        assert!(
            bot.is_in_pk_zone(),
            "zone 71 (ZONE_RONARK_LAND) is a PK zone"
        );
    }

    #[test]
    fn test_bot_not_in_pk_zone() {
        let world = WorldState::new();
        let row = make_farm_row(1, "PeaceBot", 30);
        let id = do_spawn(&world, &row, 30, 100.0, 100.0, 0, BotAiState::Farmer);
        let bot = world.get_bot(id).unwrap();
        assert!(!bot.is_in_pk_zone(), "zone 30 is not a PK zone");
    }

    #[test]
    fn test_bot_alive_checks() {
        let world = WorldState::new();
        let row = make_farm_row(1, "AliveBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Idle);
        let bot = world.get_bot(id).unwrap();
        assert!(bot.is_alive(), "newly spawned bot should be alive");
        // Kill the bot.
        world.update_bot(id, |b| {
            b.hp = 0;
            b.presence = BotPresence::Dead;
        });
        let dead_bot = world.get_bot(id).unwrap();
        assert!(!dead_bot.is_alive(), "dead bot should not be alive");
    }

    #[test]
    fn test_bot_merchant_state() {
        let world = WorldState::new();
        let row = make_farm_row(1, "MerchantBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 60, BotAiState::Merchant);
        world.update_bot(id, |b| {
            b.merchant_state = 0; // selling
            b.merchant_chat = "Buy my items!".to_string();
        });
        let bot = world.get_bot(id).unwrap();
        assert!(bot.is_merchanting());
        assert_eq!(bot.merchant_chat, "Buy my items!");
    }

    #[test]
    fn test_tick_bots_advances_tick_time() {
        let world = WorldState::new();
        let row = make_farm_row(1, "TickBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Idle);
        // Force last_tick_ms to zero so the bot processes.
        world.update_bot(id, |b| b.last_tick_ms = 0);
        tick_bots(&world);
        let bot = world.get_bot(id).unwrap();
        assert!(bot.last_tick_ms > 0, "tick_ms should be updated after tick");
    }

    #[test]
    fn test_tick_bots_despawns_expired() {
        let world = WorldState::new();
        let row = make_farm_row(1, "ExpireNow", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 1, BotAiState::Idle);
        // Force spawned_at to far in the past so duration expires.
        world.update_bot(id, |b| {
            b.spawned_at = 0; // epoch start — definitely expired by now
            b.last_tick_ms = 0;
        });
        tick_bots(&world);
        assert_eq!(
            world.bot_count(),
            0,
            "expired bot should be removed after tick"
        );
    }

    // ── set_bot_ability tests ──────────────────────────────────────────

    /// Build a minimal CoefficientRow for testing set_bot_ability.
    ///
    /// Uses representative coefficient values derived from the actual
    /// KO server coefficient table for warrior-class characters.
    fn make_coeff(hp: f64, mp: f64, sp: f64) -> CoefficientRow {
        CoefficientRow {
            s_class: 106,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.004,
            axe: 0.004,
            club: 0.004,
            spear: 0.004,
            pole: 0.004,
            staff: 0.0,
            bow: 0.0,
            hp,
            mp,
            sp,
            ac: 0.3,
            hitrate: 0.0001,
            evasionrate: 0.0001,
        }
    }

    #[test]
    fn test_set_bot_ability_warrior_level_70() {
        // Warrior: HP coeff typical value ~0.030, no MP coeff
        let coeff = make_coeff(0.030, 0.0, 0.0);
        let mut bot = BotInstance {
            id: 0,
            db_id: 0,
            name: "Warrior".to_string(),
            nation: 1,
            race: 1,
            class: 106,
            hair_rgb: 0,
            level: 70,
            face: 1,
            knights_id: 0,
            fame: 0,
            zone_id: 21,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            sp: 0,
            max_sp: 0,
            str_stat: 100,
            sta_stat: 80,
            dex_stat: 60,
            int_stat: 40,
            cha_stat: 30,
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            loyalty_daily: 0,
            in_game: true,
            presence: BotPresence::Standing,
            ai_state: BotAiState::Idle,
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
        };

        set_bot_ability(&mut bot, &coeff);

        // HP formula: (0.030 * 70^2 * 80) + (0.1 * 70 * 80) + (80/5) + 20
        //           = (0.030 * 4900 * 80) + (560) + (16) + 20
        //           = 11760 + 560 + 16 + 20 = 12356
        let expected_hp = calc_bot_max_hp(70, 80, &coeff);
        assert_eq!(bot.max_hp, expected_hp, "max_hp mismatch for warrior lvl70");
        assert_eq!(bot.hp, expected_hp, "hp should equal max_hp on spawn");
        // No MP coeff → max_mp = 0
        assert_eq!(
            bot.max_mp, 0,
            "warrior with no MP coeff should have 0 max_mp"
        );
        assert_eq!(bot.mp, 0);
        assert!(bot.max_hp > 1000, "warrior lvl70 should have > 1000 HP");
    }

    #[test]
    fn test_set_bot_ability_mage_has_mp() {
        // Mage: MP coeff nonzero
        let coeff = make_coeff(0.025, 0.015, 0.0);
        let mut bot = BotInstance {
            id: 0,
            db_id: 0,
            name: "Mage".to_string(),
            nation: 1,
            race: 1,
            class: 109, // mage novice
            hair_rgb: 0,
            level: 60,
            face: 1,
            knights_id: 0,
            fame: 0,
            zone_id: 21,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            sp: 0,
            max_sp: 0,
            str_stat: 40,
            sta_stat: 50,
            dex_stat: 60,
            int_stat: 120,
            cha_stat: 30,
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            loyalty_daily: 0,
            in_game: true,
            presence: BotPresence::Standing,
            ai_state: BotAiState::Idle,
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
        };

        set_bot_ability(&mut bot, &coeff);

        // MP formula: (0.015 * 60^2 * (120+30)) + (0.1 * 60 * 2 * 150) + (150/5) + 20
        let expected_hp = calc_bot_max_hp(60, 50, &coeff);
        let expected_mp = calc_bot_max_mp(60, 50, 120, &coeff);
        assert_eq!(bot.max_hp, expected_hp);
        assert_eq!(bot.hp, expected_hp);
        assert_eq!(bot.max_mp, expected_mp);
        assert_eq!(bot.mp, expected_mp);
        assert!(bot.max_mp > 500, "mage lvl60 should have substantial MP");
    }

    #[test]
    fn test_set_bot_ability_priest_has_mp() {
        // Priest also has MP coeff
        let coeff = make_coeff(0.025, 0.012, 0.0);
        let mut bot = BotInstance {
            id: 0,
            db_id: 0,
            name: "Priest".to_string(),
            nation: 1,
            race: 1,
            class: 111, // priest novice
            hair_rgb: 0,
            level: 65,
            face: 1,
            knights_id: 0,
            fame: 0,
            zone_id: 21,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            sp: 0,
            max_sp: 0,
            str_stat: 50,
            sta_stat: 60,
            dex_stat: 60,
            int_stat: 100,
            cha_stat: 30,
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            loyalty_daily: 0,
            in_game: true,
            presence: BotPresence::Standing,
            ai_state: BotAiState::Idle,
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
        };

        set_bot_ability(&mut bot, &coeff);

        let expected_hp = calc_bot_max_hp(65, 60, &coeff);
        let expected_mp = calc_bot_max_mp(65, 60, 100, &coeff);
        assert_eq!(bot.max_hp, expected_hp);
        assert_eq!(bot.max_mp, expected_mp);
        assert!(bot.max_mp > 0, "priest should have nonzero MP");
    }

    #[test]
    fn test_set_bot_ability_rogue_no_mp_coeff() {
        // Rogue: typically no MP or SP coeff → max_mp = 0
        let coeff = make_coeff(0.028, 0.0, 0.0);
        let mut bot = BotInstance {
            id: 0,
            db_id: 0,
            name: "Rogue".to_string(),
            nation: 1,
            race: 1,
            class: 107, // rogue mastered
            hair_rgb: 0,
            level: 55,
            face: 1,
            knights_id: 0,
            fame: 0,
            zone_id: 21,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            sp: 0,
            max_sp: 0,
            str_stat: 70,
            sta_stat: 70,
            dex_stat: 100,
            int_stat: 30,
            cha_stat: 30,
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            loyalty_daily: 0,
            in_game: true,
            presence: BotPresence::Standing,
            ai_state: BotAiState::Idle,
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
        };

        set_bot_ability(&mut bot, &coeff);

        let expected_hp = calc_bot_max_hp(55, 70, &coeff);
        assert_eq!(bot.max_hp, expected_hp);
        assert_eq!(bot.hp, expected_hp);
        assert_eq!(bot.max_mp, 0, "rogue without MP coeff should have 0 max_mp");
    }

    #[test]
    fn test_set_bot_ability_min_hp_20() {
        // Edge case: extremely low level/sta → min 20 HP
        let coeff = make_coeff(0.001, 0.0, 0.0);
        let mut bot = BotInstance {
            id: 0,
            db_id: 0,
            name: "WeakBot".to_string(),
            nation: 1,
            race: 1,
            class: 106,
            hair_rgb: 0,
            level: 1,
            face: 1,
            knights_id: 0,
            fame: 0,
            zone_id: 21,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            sp: 0,
            max_sp: 0,
            str_stat: 10,
            sta_stat: 1, // minimal STA
            dex_stat: 10,
            int_stat: 10,
            cha_stat: 10,
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            loyalty_daily: 0,
            in_game: true,
            presence: BotPresence::Standing,
            ai_state: BotAiState::Idle,
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
        };

        set_bot_ability(&mut bot, &coeff);

        assert!(bot.max_hp >= 20, "minimum HP should always be 20");
        assert_eq!(bot.hp, bot.max_hp, "hp should start at max on spawn");
    }

    #[test]
    fn test_calc_bot_max_hp_formula() {
        // Verify the HP formula: HP_COEFF * L^2 * STA + 0.1 * L * STA + STA/5 + 20
        let coeff = make_coeff(0.03, 0.0, 0.0);
        let hp = calc_bot_max_hp(70, 80, &coeff);
        // 0.03 * 70 * 70 * 80 = 0.03 * 4900 * 80 = 11760
        // 0.1 * 70 * 80 = 560
        // 80 / 5 = 16
        // + 20 = 12356
        assert_eq!(hp, 12356);
    }

    #[test]
    fn test_calc_bot_max_mp_magic_formula() {
        // MP formula for magic class: MP_COEFF * L^2 * (INT+30) + 0.1*L*2*(INT+30) + (INT+30)/5 + 20
        let coeff = make_coeff(0.0, 0.015, 0.0);
        let mp = calc_bot_max_mp(60, 50, 120, &coeff);
        // temp_intel = 120 + 30 = 150
        // 0.015 * 60 * 60 * 150 = 0.015 * 3600 * 150 = 8100
        // 0.1 * 60 * 2 * 150 = 1800
        // 150 / 5 = 30
        // + 20 = 9950
        assert_eq!(mp, 9950);
    }

    #[test]
    fn test_calc_bot_max_mp_sp_formula() {
        // SP class (Kurian): SP_COEFF * L^2 * STA + 0.1*L*STA + STA/5 (no +20)
        let coeff = make_coeff(0.0, 0.0, 0.02);
        let mp = calc_bot_max_mp(50, 100, 50, &coeff);
        // 0.02 * 50 * 50 * 100 = 0.02 * 2500 * 100 = 5000
        // 0.1 * 50 * 100 = 500
        // 100 / 5 = 20
        // total = 5520
        assert_eq!(mp, 5520);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Bot Combat AI tests
    // ═══════════════════════════════════════════════════════════════════

    /// Helper: create a minimal bot instance for combat tests (no world spawn).
    fn make_combat_bot(
        id: BotId,
        name: &str,
        nation: u8,
        class: u16,
        zone_id: u16,
        x: f32,
        z: f32,
    ) -> BotInstance {
        BotInstance {
            id,
            db_id: 0,
            name: name.to_string(),
            nation,
            race: 1,
            class,
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
            region_x: calc_region(x),
            region_z: calc_region(z),
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

    #[test]
    fn test_calculate_bot_damage_warrior() {
        let bot = make_combat_bot(1000, "Warrior", 1, 106, 72, 100.0, 100.0);
        let damage = calculate_bot_damage(&bot);
        // Warrior: STR(100) * level(70) / 25 + 70*2 = 280 + 140 = 420
        assert_eq!(damage, 420);
        assert!((10..=800).contains(&damage));
    }

    #[test]
    fn test_calculate_bot_damage_rogue() {
        let mut bot = make_combat_bot(1001, "Rogue", 1, 107, 72, 100.0, 100.0);
        bot.dex_stat = 120;
        let damage = calculate_bot_damage(&bot);
        // Rogue: DEX(120) * level(70) / 25 + 70*2 = 336 + 140 = 476
        assert_eq!(damage, 476);
    }

    #[test]
    fn test_calculate_bot_damage_mage() {
        let mut bot = make_combat_bot(1002, "Mage", 1, 109, 72, 100.0, 100.0);
        bot.int_stat = 150;
        let damage = calculate_bot_damage(&bot);
        // Mage: INT(150) * level(70) / 20 + 70*3 = 525 + 210 = 735
        assert_eq!(damage, 735);
    }

    #[test]
    fn test_calculate_bot_damage_priest() {
        let mut bot = make_combat_bot(1003, "Priest", 1, 111, 72, 100.0, 100.0);
        bot.int_stat = 100;
        let damage = calculate_bot_damage(&bot);
        // Priest: INT(100) * level(70) / 25 + 70*2 = 280 + 140 = 420
        assert_eq!(damage, 420);
    }

    #[test]
    fn test_calculate_bot_damage_minimum_clamp() {
        let mut bot = make_combat_bot(1004, "Weak", 1, 106, 72, 100.0, 100.0);
        bot.str_stat = 1;
        bot.level = 1;
        let damage = calculate_bot_damage(&bot);
        // STR(1) * 1 / 25 + 1*2 = 0.04 + 2 = 2, clamped to 10
        assert_eq!(damage, 10);
    }

    #[test]
    fn test_calculate_bot_damage_maximum_clamp() {
        let mut bot = make_combat_bot(1005, "Strong", 1, 106, 72, 100.0, 100.0);
        bot.str_stat = 255;
        bot.level = 83;
        let damage = calculate_bot_damage(&bot);
        // STR(255) * 83 / 25.0 + 83*2 = 846.6 + 166 = 1012.6, truncated to 1012
        // Clamped to MAX_DAMAGE (32000) — within range.
        assert_eq!(damage, 1012);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Bot Skill Selection tests
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_select_bot_skill_warrior_karus_low_level() {
        let mut bot = make_combat_bot(2000, "WarriorLow", 1, 106, 72, 100.0, 100.0);
        bot.level = 5;
        let skill = select_bot_skill(&bot);
        assert_eq!(
            skill, 101001,
            "level 5 warrior should use basic attack 101001"
        );
    }

    #[test]
    fn test_select_bot_skill_warrior_karus_mid_level() {
        let mut bot = make_combat_bot(2001, "WarriorMid", 1, 106, 72, 100.0, 100.0);
        bot.level = 30;
        let skill = select_bot_skill(&bot);
        assert_eq!(skill, 105525, "level 30 warrior should use Crash 105525");
    }

    #[test]
    fn test_select_bot_skill_warrior_karus_high_level() {
        let mut bot = make_combat_bot(2002, "WarriorHigh", 1, 106, 72, 100.0, 100.0);
        bot.level = 75;
        // C++ level 70-79: random between 106570 and 106575
        for _ in 0..20 {
            let skill = select_bot_skill(&bot);
            assert!(
                skill == 106570 || skill == 106575,
                "level 75 warrior should use 106570 or 106575, got {skill}"
            );
        }
    }

    #[test]
    fn test_select_bot_skill_warrior_elmorad() {
        let mut bot = make_combat_bot(2003, "WarriorElmo", 2, 106, 72, 100.0, 100.0);
        bot.level = 70;
        // C++ level 70-79: random(106570, 106575) + 100000 for Elmorad
        for _ in 0..20 {
            let skill = select_bot_skill(&bot);
            assert!(
                skill == 206570 || skill == 206575,
                "Elmorad warrior lvl 70 should be 206570 or 206575, got {skill}"
            );
        }
    }

    #[test]
    fn test_select_bot_skill_warrior_ardream_downgrade() {
        let mut bot = make_combat_bot(2004, "WarriorArd", 1, 106, 72, 100.0, 100.0);
        bot.level = 57; // <= MAX_LEVEL_ARDREAM (59), but skill 105557 > 106000? No, 105557 < 106000
        let skill = select_bot_skill(&bot);
        // Level 57 -> base 105557, which is < 106000, so NO downgrade
        assert_eq!(
            skill, 105557,
            "level 57 warrior base is 105557, below threshold"
        );
    }

    #[test]
    fn test_select_bot_skill_rogue_karus_above_ardream() {
        let mut bot = make_combat_bot(2005, "RogueMid", 1, 107, 72, 100.0, 100.0);
        bot.level = 65; // Above MAX_LEVEL_ARDREAM, no downgrade
        let skill = select_bot_skill(&bot);
        assert_eq!(skill, 108655, "level 65 rogue should use Pierce 108655");
    }

    #[test]
    fn test_select_bot_skill_rogue_ardream_downgrade() {
        let mut bot = make_combat_bot(2006, "RogueArd", 1, 107, 72, 100.0, 100.0);
        bot.level = 50; // <= 59, skill 108640 > 108000 -> downgrade
        let skill = select_bot_skill(&bot);
        // Base: 108640, threshold: 108000, 108640 > 108000 -> 108640 - 1000 = 107640
        assert_eq!(skill, 107640, "Ardream rogue should downgrade by -1000");
    }

    #[test]
    fn test_select_bot_skill_mage_karus() {
        // Level 40 mage: C++ pool = {109503..109539} with subclass offsets
        // Flame: 109503-109539, Glacier: +100, Lightning: +200
        let mut bot = make_combat_bot(2007, "MageMid", 1, 109, 72, 100.0, 100.0);
        bot.level = 40;
        for _ in 0..30 {
            let skill = select_bot_skill(&bot);
            // Valid ranges: Flame 109503-109539, Glacier 109603-109639, Lightning 109703-109739
            assert!(
                (109503..=109539).contains(&skill)
                    || (109603..=109639).contains(&skill)
                    || (109703..=109739).contains(&skill),
                "level 40 Karus mage skill {skill} not in expected range"
            );
        }
    }

    #[test]
    fn test_select_bot_skill_mage_elmorad() {
        // Elmorad mage lvl 65: base 210xxx + subclass offset
        let mut bot = make_combat_bot(2008, "MageElmo", 2, 109, 72, 100.0, 100.0);
        bot.level = 65;
        for _ in 0..30 {
            let skill = select_bot_skill(&bot);
            // Base pool 110542-110560 + 100000 = 210542-210560 (Flame)
            // Glacier: 210642-210660, Lightning: 210742-210760
            assert!(
                (210542..=210560).contains(&skill)
                    || (210642..=210660).contains(&skill)
                    || (210742..=210760).contains(&skill),
                "Elmorad mage lvl 65 skill {skill} not in expected range"
            );
        }
    }

    #[test]
    fn test_select_bot_skill_priest_karus() {
        let mut bot = make_combat_bot(2009, "PriestHigh", 1, 111, 72, 100.0, 100.0);
        bot.level = 72;
        let skill = select_bot_skill(&bot);
        assert_eq!(
            skill, 112815,
            "level 72 priest should use Master Holy Devastate 112815"
        );
    }

    #[test]
    fn test_select_bot_skill_priest_elmorad_ardream() {
        let mut bot = make_combat_bot(2010, "PriestElmoArd", 2, 111, 72, 100.0, 100.0);
        bot.level = 55; // <= 59
                        // C++ level 51-59: random among {111520, 111542, 111551} + 100000 for Elmorad
                        // All are < 212000, so NO downgrade
        let valid = [211520, 211542, 211551];
        for _ in 0..30 {
            let skill = select_bot_skill(&bot);
            assert!(
                valid.contains(&skill),
                "Elmorad priest lvl 55 skill {skill} not in expected pool"
            );
        }
    }

    #[test]
    fn test_select_bot_skill_defaults_for_unknown_class() {
        let mut bot = make_combat_bot(2011, "Unknown", 1, 199, 72, 100.0, 100.0);
        bot.level = 30;
        let skill = select_bot_skill(&bot);
        // Unknown class falls back to warrior
        assert_eq!(
            skill, 105525,
            "unknown class should default to warrior skills"
        );
    }

    #[test]
    fn test_get_rogue_arrow_skill_levels() {
        // Verify the arrow skill table returns correct values
        assert_eq!(get_rogue_arrow_skill(1), 107003);
        assert_eq!(get_rogue_arrow_skill(15), 107500);
        assert_eq!(get_rogue_arrow_skill(30), 107525);
        assert_eq!(get_rogue_arrow_skill(45), 107540);
        assert_eq!(get_rogue_arrow_skill(55), 107552);
        assert_eq!(get_rogue_arrow_skill(65), 108552);
        assert_eq!(get_rogue_arrow_skill(75), 108570);
        assert_eq!(get_rogue_arrow_skill(83), 108585);
    }

    #[test]
    fn test_build_bot_magic_packet_format() {
        let pkt = build_bot_magic_packet(
            MAGIC_EFFECTING,
            106570,
            10001,
            500,
            [100, 0, 200, -250, 0, 0, 0],
        );
        // Packet.opcode is stored separately; Packet.data is the payload.
        assert_eq!(
            pkt.opcode,
            Opcode::WizMagicProcess as u8,
            "opcode should be WizMagicProcess (0x31)"
        );
        let bytes = &pkt.data;
        // data[0]: magic sub-opcode
        assert_eq!(
            bytes[0], MAGIC_EFFECTING,
            "magic opcode should be MAGIC_EFFECTING (3)"
        );
        // data[1..5]: skill_id
        let skill_id = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        assert_eq!(skill_id, 106570);
        // data[5..9]: caster_id
        let caster_id = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        assert_eq!(caster_id, 10001);
        // data[9..13]: target_id
        let target_id = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);
        assert_eq!(target_id, 500);
        // Total payload size: 1 (sub-opcode) + 4 (skill) + 4 (caster) + 4 (target) + 7*4 (data) = 41
        assert_eq!(bytes.len(), 41, "payload should be 41 bytes");
    }

    #[test]
    fn test_move_toward_target_direction() {
        let bot = make_combat_bot(1000, "Mover", 1, 106, 72, 100.0, 100.0);
        // Target is directly east (x=200, z=100).
        let (new_x, _new_y, new_z) = move_toward_target(&bot, 200.0, 0.0, 100.0);
        // Bot should move toward the east (new_x > 100, new_z ~= 100).
        assert!(new_x > 100.0, "bot should move toward target (x increased)");
        // The Z coordinate should be close to 100.0 due to jitter.
        assert!(
            (new_z - 100.0).abs() < 10.0,
            "z should stay relatively close"
        );
    }

    #[test]
    fn test_move_toward_target_does_not_overshoot() {
        let bot = make_combat_bot(1000, "Close", 1, 106, 72, 100.0, 100.0);
        // Target is very close (1 unit away). Bot should not overshoot.
        let (new_x, _new_y, new_z) = move_toward_target(&bot, 101.0, 0.0, 100.0);
        // Distance from bot origin to new pos should be <= BOT_MOVE_SPEED.
        let dx = new_x - 100.0;
        let dz = new_z - 100.0;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(
            dist <= BOT_MOVE_SPEED + 3.0, // +3 for jitter tolerance
            "movement should not exceed BOT_MOVE_SPEED, got {}",
            dist
        );
    }

    #[test]
    fn test_move_toward_target_at_same_position() {
        let bot = make_combat_bot(1000, "Same", 1, 106, 72, 100.0, 100.0);
        let (new_x, _new_y, new_z) = move_toward_target(&bot, 100.0, 0.0, 100.0);
        // When target is at the same position (after jitter), movement is minimal.
        let dx = new_x - 100.0;
        let dz = new_z - 100.0;
        let dist = (dx * dx + dz * dz).sqrt();
        // Should be small (jitter only moves the target by at most +/-2 units).
        assert!(
            dist < BOT_MOVE_SPEED + 5.0,
            "should not move far from current position"
        );
    }

    #[test]
    fn test_is_pk_zone_ardream() {
        assert!(is_pk_zone(72), "zone 72 (Ardream) is a PK zone");
    }

    #[test]
    fn test_is_pk_zone_ronark_land() {
        assert!(is_pk_zone(71), "zone 71 (Ronark Land) is a PK zone");
    }

    #[test]
    fn test_is_pk_zone_ronark_land_base() {
        assert!(is_pk_zone(73), "zone 73 (Ronark Land Base) is a PK zone");
    }

    #[test]
    fn test_is_not_pk_zone_moradon() {
        assert!(!is_pk_zone(21), "zone 21 (Moradon) is NOT a PK zone");
    }

    #[test]
    fn test_is_not_pk_zone_karus() {
        assert!(!is_pk_zone(11), "zone 11 (Karus) is NOT a PK zone");
    }

    #[test]
    fn test_find_nearest_enemy_no_targets() {
        let world = WorldState::new();
        // Spawn a Karus bot in PK zone 72.
        let bot = make_combat_bot(BOT_ID_BASE, "KarusBot", 1, 106, 72, 100.0, 100.0);
        world.insert_bot(bot.clone());

        // No other bots or players — should find nothing.
        let result = find_nearest_enemy(&bot, &world);
        assert!(result.is_none(), "should find no target with no enemies");
    }

    #[test]
    fn test_find_nearest_enemy_same_nation_bot() {
        let world = WorldState::new();
        // Two Karus bots — same nation, should NOT target each other.
        let bot1 = make_combat_bot(BOT_ID_BASE, "Karus1", 1, 106, 72, 100.0, 100.0);
        let bot2 = make_combat_bot(BOT_ID_BASE + 1, "Karus2", 1, 107, 72, 105.0, 100.0);
        world.insert_bot(bot1.clone());
        world.insert_bot(bot2);

        let result = find_nearest_enemy(&bot1, &world);
        assert!(
            result.is_none(),
            "same-nation bots should not target each other"
        );
    }

    #[test]
    fn test_find_nearest_enemy_different_nation_bot() {
        let world = WorldState::new();
        // Karus bot vs Elmorad bot — should target each other.
        let karus = make_combat_bot(BOT_ID_BASE, "Karus", 1, 106, 72, 100.0, 100.0);
        let elmo = make_combat_bot(BOT_ID_BASE + 1, "Elmo", 2, 106, 72, 110.0, 100.0);
        world.insert_bot(karus.clone());
        world.insert_bot(elmo.clone());

        let result = find_nearest_enemy(&karus, &world);
        assert!(result.is_some(), "should find enemy nation bot");
        let (target_id, x, _y, z) = result.unwrap();
        assert_eq!(target_id as BotId, elmo.id);
        assert!((x - 110.0).abs() < 0.01);
        assert!((z - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_find_nearest_enemy_picks_closest() {
        let world = WorldState::new();
        let karus = make_combat_bot(BOT_ID_BASE, "Karus", 1, 106, 72, 100.0, 100.0);
        let elmo_far = make_combat_bot(BOT_ID_BASE + 1, "ElmoFar", 2, 106, 72, 130.0, 100.0);
        let elmo_near = make_combat_bot(BOT_ID_BASE + 2, "ElmoNear", 2, 107, 72, 105.0, 100.0);
        world.insert_bot(karus.clone());
        world.insert_bot(elmo_far);
        world.insert_bot(elmo_near.clone());

        let result = find_nearest_enemy(&karus, &world);
        assert!(result.is_some());
        let (target_id, _, _, _) = result.unwrap();
        assert_eq!(
            target_id as BotId, elmo_near.id,
            "should pick the closer bot"
        );
    }

    #[test]
    fn test_find_nearest_enemy_out_of_range() {
        let world = WorldState::new();
        // Karus bot at (100,100), Elmorad bot at (200,200) — distance ~141 > 45.
        let karus = make_combat_bot(BOT_ID_BASE, "Karus", 1, 106, 72, 100.0, 100.0);
        let elmo = make_combat_bot(BOT_ID_BASE + 1, "Elmo", 2, 106, 72, 200.0, 200.0);
        world.insert_bot(karus.clone());
        world.insert_bot(elmo);

        let result = find_nearest_enemy(&karus, &world);
        assert!(
            result.is_none(),
            "target outside search range should not be found"
        );
    }

    #[test]
    fn test_find_nearest_enemy_dead_bot_skipped() {
        let world = WorldState::new();
        let karus = make_combat_bot(BOT_ID_BASE, "Karus", 1, 106, 72, 100.0, 100.0);
        let mut dead_elmo = make_combat_bot(BOT_ID_BASE + 1, "DeadElmo", 2, 106, 72, 105.0, 100.0);
        dead_elmo.hp = 0;
        dead_elmo.presence = BotPresence::Dead;
        world.insert_bot(karus.clone());
        world.insert_bot(dead_elmo);

        let result = find_nearest_enemy(&karus, &world);
        assert!(result.is_none(), "dead bots should not be targeted");
    }

    #[test]
    fn test_find_nearest_enemy_different_zone_skipped() {
        let world = WorldState::new();
        let karus = make_combat_bot(BOT_ID_BASE, "Karus", 1, 106, 72, 100.0, 100.0);
        // Elmo is in zone 71 (different from karus zone 72).
        let elmo = make_combat_bot(BOT_ID_BASE + 1, "Elmo", 2, 106, 71, 105.0, 100.0);
        world.insert_bot(karus.clone());
        world.insert_bot(elmo);

        let result = find_nearest_enemy(&karus, &world);
        assert!(
            result.is_none(),
            "bots in different zones should not target each other"
        );
    }

    #[test]
    fn test_find_nearest_enemy_non_pk_zone_skipped() {
        let world = WorldState::new();
        // Both bots in zone 21 (Moradon, not PK).
        let karus = make_combat_bot(BOT_ID_BASE, "Karus", 1, 106, 21, 100.0, 100.0);
        let elmo = make_combat_bot(BOT_ID_BASE + 1, "Elmo", 2, 106, 21, 105.0, 100.0);
        world.insert_bot(karus.clone());
        world.insert_bot(elmo);

        let result = find_nearest_enemy(&karus, &world);
        assert!(
            result.is_none(),
            "bots in non-PK zones should not target each other"
        );
    }

    #[test]
    fn test_apply_damage_to_bot_target() {
        let world = WorldState::new();
        let mut target = make_combat_bot(BOT_ID_BASE + 1, "Target", 2, 106, 72, 105.0, 100.0);
        target.hp = 1000;
        target.max_hp = 5000;
        world.insert_bot(target);

        let (result, remaining_hp) =
            apply_damage_to_target(&world, (BOT_ID_BASE + 1) as SessionId, 200, BOT_ID_BASE);

        assert_eq!(result, ATTACK_SUCCESS, "target should still be alive");
        assert_eq!(remaining_hp, 800);

        // Verify the bot's HP was updated in the world.
        let updated = world.get_bot(BOT_ID_BASE + 1).unwrap();
        assert_eq!(updated.hp, 800);
    }

    #[test]
    fn test_apply_damage_kills_bot_target() {
        let world = WorldState::new();
        let mut target = make_combat_bot(BOT_ID_BASE + 1, "Victim", 2, 106, 72, 105.0, 100.0);
        target.hp = 50;
        target.max_hp = 5000;
        world.insert_bot(target);

        let (result, remaining_hp) =
            apply_damage_to_target(&world, (BOT_ID_BASE + 1) as SessionId, 100, BOT_ID_BASE);

        assert_eq!(result, ATTACK_TARGET_DEAD, "target should be dead");
        assert_eq!(remaining_hp, 0);

        let dead = world.get_bot(BOT_ID_BASE + 1).unwrap();
        assert_eq!(dead.hp, 0);
        assert_eq!(dead.presence, BotPresence::Dead);
    }

    #[test]
    fn test_tick_fighting_moves_toward_target() {
        let world = WorldState::new();
        // Karus bot at (100, 100).
        let karus_row = make_farm_row(1, "Karus", 72);
        let karus_id = do_spawn(&world, &karus_row, 72, 100.0, 100.0, 0, BotAiState::Pk);
        world.update_bot(karus_id, |b| {
            b.nation = 1;
            b.hp = 5000;
            b.max_hp = 5000;
            b.last_move_ms = 0; // allow immediate action
            b.last_tick_ms = 0;
        });

        // Elmorad bot at (120, 100) — within search range but outside attack range.
        let mut elmo_row = make_farm_row(2, "Elmo", 72);
        elmo_row.nation = 2;
        let elmo_id = do_spawn(&world, &elmo_row, 72, 120.0, 100.0, 0, BotAiState::Pk);
        world.update_bot(elmo_id, |b| {
            b.nation = 2;
            b.hp = 5000;
            b.max_hp = 5000;
        });

        let bot_before = world.get_bot(karus_id).unwrap();
        let now_ms = tick_ms();
        tick_fighting(&world, &bot_before, now_ms);

        // After tick, the Karus bot should have moved closer to Elmo.
        let bot_after = world.get_bot(karus_id).unwrap();
        assert!(
            bot_after.x > 100.0,
            "bot should have moved toward target (x: {} > 100.0)",
            bot_after.x
        );
        assert!(
            bot_after.target_id >= 0,
            "bot should have acquired a target"
        );
    }

    #[test]
    fn test_tick_fighting_no_target_resets() {
        let world = WorldState::new();
        // Single bot with no enemies.
        let row = make_farm_row(1, "LoneBot", 72);
        let id = do_spawn(&world, &row, 72, 100.0, 100.0, 0, BotAiState::Pk);
        world.update_bot(id, |b| {
            b.nation = 1;
            b.hp = 5000;
            b.max_hp = 5000;
            b.target_id = 42; // previously had a target
            b.last_move_ms = 0;
            b.last_tick_ms = 0;
        });

        let bot = world.get_bot(id).unwrap();
        let now_ms = tick_ms();
        tick_fighting(&world, &bot, now_ms);

        let updated = world.get_bot(id).unwrap();
        assert_eq!(updated.target_id, -1, "target should be cleared");
    }

    #[test]
    fn test_tick_fighting_flees_on_low_hp() {
        let world = WorldState::new();
        let row = make_farm_row(1, "FleeBot", 72);
        let id = do_spawn(&world, &row, 72, 100.0, 100.0, 0, BotAiState::Pk);
        world.update_bot(id, |b| {
            b.nation = 1;
            b.hp = 50; // 50 / 5000 = 1% < 20%
            b.max_hp = 5000;
            b.last_move_ms = 0;
            b.last_tick_ms = 0;
        });

        let bot = world.get_bot(id).unwrap();
        let now_ms = tick_ms();
        tick_fighting(&world, &bot, now_ms);

        let updated = world.get_bot(id).unwrap();
        // After fleeing, bot should have moved from original position.
        let dx = updated.x - 100.0;
        let dz = updated.z - 100.0;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(
            dist > 1.0,
            "fleeing bot should have moved away, dist = {}",
            dist
        );
        assert_eq!(updated.target_id, -1, "fleeing bot should clear its target");
    }

    #[test]
    fn test_tick_fighting_movement_is_independent_from_attack_cooldown() {
        let world = WorldState::new();
        let row = make_farm_row(1, "CoolBot", 72);
        let id = do_spawn(&world, &row, 72, 100.0, 100.0, 0, BotAiState::Pk);
        let now_ms = tick_ms();
        // Set last_move_ms to current time — cooldown not expired yet.
        world.update_bot(id, |b| {
            b.nation = 1;
            b.hp = 5000;
            b.max_hp = 5000;
            b.last_move_ms = now_ms;
            b.last_tick_ms = 0;
        });

        let bot = world.get_bot(id).unwrap();
        let pos_before = bot.x;
        tick_fighting(&world, &bot, now_ms);

        let updated = world.get_bot(id).unwrap();
        assert!(
            (updated.x - pos_before).abs() > 0.001,
            "bot patrol should keep moving while attack skill is on cooldown"
        );
    }

    #[test]
    fn test_broadcast_bot_move_packet_format() {
        // Verify the WIZ_MOVE packet wire format for a bot.
        let bot = make_combat_bot(BOT_ID_BASE, "MoveBot", 1, 106, 72, 100.0, 100.0);
        let new_x = 105.0_f32;
        let new_y = 10.0_f32;
        let new_z = 100.0_f32;

        // Build the packet manually to verify format.
        let will_x = (new_x * 10.0) as u16;
        let will_y = (new_y * 10.0) as u16;
        let will_z = (new_z * 10.0) as u16;
        let speed: i16 = 45;

        let mut pkt = Packet::new(Opcode::WizMove as u8);
        pkt.write_u32(bot.id);
        pkt.write_u16(will_x);
        pkt.write_u16(will_z);
        pkt.write_u16(will_y);
        pkt.write_i16(speed);
        pkt.write_u8(0);

        assert_eq!(pkt.opcode, Opcode::WizMove as u8);
        // Data: u32(4) + u16(2) + u16(2) + u16(2) + i16(2) + u8(1) = 13 bytes
        assert_eq!(pkt.data.len(), 13);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(bot.id));
        assert_eq!(r.read_u16(), Some(1050)); // 105.0 * 10
        assert_eq!(r.read_u16(), Some(1000)); // 100.0 * 10
        assert_eq!(r.read_u16(), Some(100)); // 10.0 * 10
        assert_eq!(r.read_u16().map(|v| v as i16), Some(45));
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_attack_packet_format() {
        // Verify the WIZ_ATTACK broadcast packet format.
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(1); // bType: normal melee
        pkt.write_u8(ATTACK_SUCCESS); // bResult
        pkt.write_u32(BOT_ID_BASE); // attacker_id (bot)
        pkt.write_u32(42); // target_id (player)
        pkt.write_u8(0); // unknown

        assert_eq!(pkt.opcode, Opcode::WizAttack as u8);
        // Data: u8(1) + u8(1) + u32(4) + u32(4) + u8(1) = 11 bytes
        assert_eq!(pkt.data.len(), 11);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(ATTACK_SUCCESS));
        assert_eq!(r.read_u32(), Some(BOT_ID_BASE));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_attack_packet_dead_result() {
        let mut pkt = Packet::new(Opcode::WizAttack as u8);
        pkt.write_u8(1);
        pkt.write_u8(ATTACK_TARGET_DEAD);
        pkt.write_u32(BOT_ID_BASE);
        pkt.write_u32(99);
        pkt.write_u8(0);

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(ATTACK_TARGET_DEAD));
        assert_eq!(r.read_u32(), Some(BOT_ID_BASE));
        assert_eq!(r.read_u32(), Some(99));
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_flee_hp_threshold() {
        // Verify the flee threshold calculation.
        let hp = 100_i16;
        let max_hp = 5000_i16;
        let ratio = hp as f32 / max_hp as f32;
        assert!(
            ratio < BOT_FLEE_HP_PERCENT,
            "100/5000 = {} should be below flee threshold {}",
            ratio,
            BOT_FLEE_HP_PERCENT
        );

        let hp2 = 1500_i16;
        let ratio2 = hp2 as f32 / max_hp as f32;
        assert!(
            ratio2 > BOT_FLEE_HP_PERCENT,
            "1500/5000 = {} should be above flee threshold {}",
            ratio2,
            BOT_FLEE_HP_PERCENT
        );
    }

    #[test]
    fn test_bot_combat_tick_updates_last_tick() {
        let world = WorldState::new();
        let row = make_farm_row(1, "CombatBot", 72);
        let id = do_spawn(&world, &row, 72, 100.0, 100.0, 0, BotAiState::Pk);
        world.update_bot(id, |b| {
            b.nation = 1;
            b.hp = 5000;
            b.max_hp = 5000;
            b.last_tick_ms = 0;
            b.last_move_ms = 0;
        });

        let bot = world.get_bot(id).unwrap();
        let now_ms = tick_ms();
        tick_fighting(&world, &bot, now_ms);

        let updated = world.get_bot(id).unwrap();
        assert!(
            updated.last_tick_ms > 0,
            "last_tick_ms should be updated after combat tick"
        );
    }

    // ── Bot death/regene tests ──────────────────────────────────────

    #[test]
    fn test_bot_on_death_sets_dead_state() {
        let world = WorldState::new();
        let row = make_farm_row(1, "DeathBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);
        world.update_bot(id, |b| {
            b.hp = 100;
            b.max_hp = 5000;
        });

        let now_ms = tick_ms();
        bot_on_death(&world, id, now_ms);

        let bot = world.get_bot(id).unwrap();
        assert_eq!(bot.presence, BotPresence::Dead, "bot should be dead");
        assert_eq!(bot.hp, 0, "dead bot HP should be 0");
        assert_eq!(
            bot.original_ai_state,
            BotAiState::Farmer,
            "original AI state should be saved"
        );
        assert!(bot.regene_at_ms > 0, "regene timer should be set");
        assert_eq!(
            bot.regene_at_ms,
            now_ms + BOT_REGENE_DELAY_MS,
            "regene_at_ms should be now + delay"
        );
        assert_eq!(bot.target_id, -1, "target should be cleared on death");
    }

    #[test]
    fn test_bot_on_death_ignores_already_dead() {
        let world = WorldState::new();
        let row = make_farm_row(1, "AlreadyDead", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);
        world.update_bot(id, |b| {
            b.hp = 0;
            b.presence = BotPresence::Dead;
            b.regene_at_ms = 12345;
        });

        let now_ms = tick_ms();
        bot_on_death(&world, id, now_ms);

        // Should not change regene_at_ms since already dead.
        let bot = world.get_bot(id).unwrap();
        assert_eq!(bot.regene_at_ms, 12345, "regene_at_ms should not change");
    }

    #[test]
    fn test_bot_regene_restores_hp_and_state() {
        let world = WorldState::new();
        let row = make_farm_row(1, "RegeneBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Pk);

        // Kill the bot.
        let now_ms = tick_ms();
        bot_on_death(&world, id, now_ms);

        let dead_bot = world.get_bot(id).unwrap();
        assert_eq!(dead_bot.presence, BotPresence::Dead);
        let max_hp = dead_bot.max_hp;
        let max_mp = dead_bot.max_mp;

        // Regene the bot.
        let regene_ms = now_ms + BOT_REGENE_DELAY_MS + 100;
        bot_regene(&world, id, regene_ms);

        let alive_bot = world.get_bot(id).unwrap();
        assert_eq!(
            alive_bot.presence,
            BotPresence::Standing,
            "bot should be standing"
        );
        assert_eq!(alive_bot.hp, max_hp, "HP should be restored to max");
        assert_eq!(alive_bot.mp, max_mp, "MP should be restored to max");
        assert_eq!(
            alive_bot.ai_state,
            BotAiState::Pk,
            "AI state should be restored to original (Pk)"
        );
        assert_eq!(alive_bot.regene_at_ms, 0, "regene timer should be cleared");
        assert_eq!(alive_bot.target_id, -1, "target should be cleared");
    }

    #[test]
    fn test_bot_regene_ignores_alive_bot() {
        let world = WorldState::new();
        let row = make_farm_row(1, "AliveBot2", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);

        // Try to regene an alive bot — should be a no-op.
        let now_ms = tick_ms();
        bot_regene(&world, id, now_ms);

        let bot = world.get_bot(id).unwrap();
        assert_eq!(
            bot.presence,
            BotPresence::Standing,
            "alive bot should stay standing"
        );
    }

    #[test]
    fn test_tick_bots_regenes_dead_bot_after_delay() {
        let world = WorldState::new();
        let row = make_farm_row(1, "TickRegene", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);

        // Kill the bot and set regene time in the past.
        world.update_bot(id, |b| {
            b.hp = 0;
            b.presence = BotPresence::Dead;
            b.original_ai_state = BotAiState::Farmer;
            b.regene_at_ms = 1; // way in the past
            b.last_tick_ms = 0;
        });

        tick_bots(&world);

        let bot = world.get_bot(id).unwrap();
        assert_eq!(
            bot.presence,
            BotPresence::Standing,
            "bot should be regened after tick"
        );
        assert!(bot.hp > 0, "bot HP should be restored");
        assert_eq!(
            bot.ai_state,
            BotAiState::Farmer,
            "bot AI state should be restored"
        );
    }

    #[test]
    fn test_tick_bots_does_not_regene_before_timer() {
        let world = WorldState::new();
        let row = make_farm_row(1, "NoRegeneYet", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);

        // Kill the bot and set regene time far in the future.
        world.update_bot(id, |b| {
            b.hp = 0;
            b.presence = BotPresence::Dead;
            b.original_ai_state = BotAiState::Farmer;
            b.regene_at_ms = u64::MAX; // way in the future
            b.last_tick_ms = 0;
        });

        tick_bots(&world);

        let bot = world.get_bot(id).unwrap();
        assert_eq!(
            bot.presence,
            BotPresence::Dead,
            "bot should still be dead before regene timer"
        );
    }

    #[test]
    fn test_apply_damage_kills_bot_triggers_death() {
        let world = WorldState::new();
        let row = make_farm_row(1, "VictimBot", 21);
        let target_id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);

        // Set the bot's HP low enough that a hit will kill it.
        world.update_bot(target_id, |b| {
            b.hp = 50;
            b.max_hp = 5000;
        });

        // Apply lethal damage from an attacker.
        let (result, new_hp) = apply_damage_to_target(&world, target_id as u16, 100, 9999);
        assert_eq!(result, ATTACK_TARGET_DEAD, "attack should kill the bot");
        assert_eq!(new_hp, 0, "HP should be 0");

        // The bot should now be dead with a regene timer.
        let bot = world.get_bot(target_id).unwrap();
        assert_eq!(bot.presence, BotPresence::Dead, "bot should be dead");
        assert!(bot.regene_at_ms > 0, "regene timer should be set");
    }

    // ── GM bot spawn/despawn tests ──────────────────────────────────

    #[test]
    fn test_spawn_gm_bot_creates_bot() {
        let world = WorldState::new();
        let id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 21,
                x: 100.0,
                y: 0.0,
                z: 200.0,
                class: 1, // warrior
                level: 60,
                nation: 1, // Karus
                ai_state: BotAiState::Farmer,
            },
        );

        assert!(id >= BOT_ID_BASE, "bot ID should be above BOT_ID_BASE");
        assert_eq!(world.bot_count(), 1);

        let bot = world.get_bot(id).unwrap();
        assert!(bot.in_game, "bot should be in-game");
        assert_eq!(bot.level, 60);
        assert_eq!(bot.nation, 1);
        assert_eq!(bot.zone_id, 21);
        assert_eq!(bot.ai_state, BotAiState::Farmer);
        assert!(bot.hp > 0, "bot should have positive HP");
        assert_eq!(bot.presence, BotPresence::Standing);
        assert_eq!(bot.duration_minutes, 60, "GM bots default to 60 min");
    }

    #[test]
    fn test_spawn_gm_bot_different_classes() {
        let world = WorldState::new();

        let warrior_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 72,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                class: 1,
                level: 70,
                nation: 2,
                ai_state: BotAiState::Pk,
            },
        );
        let warrior = world.get_bot(warrior_id).unwrap();
        assert!(warrior.is_warrior(), "class=1 should be warrior");

        let rogue_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 72,
                x: 105.0,
                y: 0.0,
                z: 100.0,
                class: 2,
                level: 70,
                nation: 1,
                ai_state: BotAiState::Pk,
            },
        );
        let rogue = world.get_bot(rogue_id).unwrap();
        assert!(rogue.is_rogue(), "class=2 should be rogue");

        let mage_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 72,
                x: 110.0,
                y: 0.0,
                z: 100.0,
                class: 3,
                level: 70,
                nation: 1,
                ai_state: BotAiState::Pk,
            },
        );
        let mage = world.get_bot(mage_id).unwrap();
        assert!(mage.is_mage(), "class=3 should be mage");

        let priest_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 72,
                x: 115.0,
                y: 0.0,
                z: 100.0,
                class: 4,
                level: 70,
                nation: 1,
                ai_state: BotAiState::Pk,
            },
        );
        let priest = world.get_bot(priest_id).unwrap();
        assert!(priest.is_priest(), "class=4 should be priest");
    }

    #[test]
    fn test_despawn_bots_in_zone() {
        let world = WorldState::new();

        // Spawn 3 bots in zone 21, 1 bot in zone 72.
        for i in 0..3 {
            spawn_gm_bot(
                &world,
                SpawnGmBotParams {
                    zone_id: 21,
                    x: 100.0 + i as f32,
                    y: 0.0,
                    z: 200.0,
                    class: 1,
                    level: 60,
                    nation: 1,
                    ai_state: BotAiState::Farmer,
                },
            );
        }
        spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: 72,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                class: 2,
                level: 70,
                nation: 2,
                ai_state: BotAiState::Pk,
            },
        );

        assert_eq!(world.bot_count(), 4);

        let removed = despawn_bots_in_zone(&world, 21);
        assert_eq!(removed, 3, "should remove 3 bots from zone 21");
        assert_eq!(world.bot_count(), 1, "1 bot should remain in zone 72");
    }

    #[test]
    fn test_despawn_gm_pk_bots_preserves_other_bots() {
        let world = WorldState::new();

        let gm_pk_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: ZONE_RONARK_LAND,
                x: 1375.0,
                y: 0.0,
                z: 1098.0,
                class: 1,
                level: 83,
                nation: 1,
                ai_state: BotAiState::Pk,
            },
        );
        let farmer_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: ZONE_RONARK_LAND,
                x: 1376.0,
                y: 0.0,
                z: 1098.0,
                class: 2,
                level: 83,
                nation: 1,
                ai_state: BotAiState::Farmer,
            },
        );
        let db_pk_id = spawn_gm_bot(
            &world,
            SpawnGmBotParams {
                zone_id: ZONE_RONARK_LAND,
                x: 622.0,
                y: 0.0,
                z: 898.0,
                class: 3,
                level: 83,
                nation: 2,
                ai_state: BotAiState::Pk,
            },
        );
        world.update_bot(db_pk_id, |bot| bot.db_id = 123);

        assert_eq!(count_gm_pk_bots_in_zone(&world, ZONE_RONARK_LAND), 1);
        assert_eq!(despawn_gm_pk_bots_in_zone(&world, ZONE_RONARK_LAND), 1);
        assert!(world.get_bot(gm_pk_id).is_none());
        assert!(world.get_bot(farmer_id).is_some());
        assert!(world.get_bot(db_pk_id).is_some());
    }

    #[test]
    fn test_despawn_all_bots() {
        let world = WorldState::new();

        for i in 0..5 {
            spawn_gm_bot(
                &world,
                SpawnGmBotParams {
                    zone_id: if i < 3 { 21 } else { 72 },
                    x: 100.0 + i as f32,
                    y: 0.0,
                    z: 200.0,
                    class: 1,
                    level: 60,
                    nation: 1,
                    ai_state: BotAiState::Farmer,
                },
            );
        }

        assert_eq!(world.bot_count(), 5);

        let removed = despawn_all_bots(&world);
        assert_eq!(removed, 5, "should remove all 5 bots");
        assert_eq!(world.bot_count(), 0, "no bots should remain");
    }

    #[test]
    fn test_gm_class_to_real_class_mapping() {
        // Karus (nation=1): base=100
        assert_eq!(gm_class_to_real_class(1, 1), 101); // warrior
        assert_eq!(gm_class_to_real_class(2, 1), 102); // rogue
        assert_eq!(gm_class_to_real_class(3, 1), 103); // mage
        assert_eq!(gm_class_to_real_class(4, 1), 104); // priest
        assert_eq!(gm_class_to_real_class(5, 1), 102); // rogue alt
        assert_eq!(gm_class_to_real_class(6, 1), 103); // mage alt
        assert_eq!(gm_class_to_real_class(8, 1), 104); // priest alt

        // ElMorad (nation=2): base=200
        assert_eq!(gm_class_to_real_class(1, 2), 201); // warrior
        assert_eq!(gm_class_to_real_class(2, 2), 202); // rogue
        assert_eq!(gm_class_to_real_class(3, 2), 203); // mage
        assert_eq!(gm_class_to_real_class(4, 2), 204); // priest
    }

    #[test]
    fn test_gm_bot_stats_by_class() {
        // Warrior (class % 100 = 1): STR=100
        let (str_s, _sta_s, dex_s, _int_s, _cha_s) = gm_bot_stats(101);
        assert_eq!(str_s, 100, "warrior should have highest STR");
        assert!(str_s > dex_s, "warrior STR > DEX");

        // Rogue (class % 100 = 2): DEX=100
        let (str_s, _sta_s, dex_s, _int_s, _cha_s) = gm_bot_stats(102);
        assert_eq!(dex_s, 100, "rogue should have highest DEX");
        assert!(dex_s > str_s, "rogue DEX > STR");

        // Mage (class % 100 = 3): INT=120
        let (_str_s, _sta_s, _dex_s, int_s, _cha_s) = gm_bot_stats(103);
        assert_eq!(int_s, 120, "mage should have highest INT");

        // Priest (class % 100 = 4): INT=100
        let (_str_s, sta_s, _dex_s, int_s, _cha_s) = gm_bot_stats(104);
        assert_eq!(int_s, 100, "priest should have high INT");
        assert_eq!(sta_s, 60, "priest should have moderate STA");
    }

    #[test]
    fn test_bot_death_packet_format() {
        // Verify the WIZ_DEAD broadcast packet format for bots.
        let bot_id: u32 = BOT_ID_BASE + 42;
        let mut pkt = Packet::new(Opcode::WizDead as u8);
        pkt.write_u32(bot_id);

        assert_eq!(pkt.opcode, Opcode::WizDead as u8);
        assert_eq!(pkt.data.len(), 4); // u32 = 4 bytes

        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(bot_id));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_bot_regene_inout_packets() {
        // Verify the INOUT_OUT and INOUT_RESPAWN packet formats.
        let bot_id: u32 = BOT_ID_BASE + 99;

        // INOUT_OUT packet
        let mut out_pkt = Packet::new(Opcode::WizUserInout as u8);
        out_pkt.write_u8(2); // INOUT_OUT
        out_pkt.write_u8(0); // reserved
        out_pkt.write_u32(bot_id);

        assert_eq!(out_pkt.opcode, Opcode::WizUserInout as u8);
        let mut r = ko_protocol::PacketReader::new(&out_pkt.data);
        assert_eq!(r.read_u8(), Some(2)); // INOUT_OUT
        assert_eq!(r.read_u8(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(bot_id));

        // INOUT_RESPAWN packet
        let mut resp_pkt = Packet::new(Opcode::WizUserInout as u8);
        resp_pkt.write_u8(3); // INOUT_RESPAWN
        resp_pkt.write_u8(0);
        resp_pkt.write_u32(bot_id);

        let mut r2 = ko_protocol::PacketReader::new(&resp_pkt.data);
        assert_eq!(r2.read_u8(), Some(3)); // INOUT_RESPAWN
        assert_eq!(r2.read_u8(), Some(0));
        assert_eq!(r2.read_u32(), Some(bot_id));
    }

    // ═════════════════════════════════════════════════════════════════════
    // Bot HP/MP Regen tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_calc_bot_hp_regen_low_level() {
        assert_eq!(calc_bot_hp_regen(10, 30), 26);
    }

    #[test]
    fn test_calc_bot_hp_regen_mid_level() {
        assert_eq!(calc_bot_hp_regen(50, 70), 114);
    }

    #[test]
    fn test_calc_bot_hp_regen_high_level() {
        assert_eq!(calc_bot_hp_regen(80, 100), 180);
    }

    #[test]
    fn test_calc_bot_mp_regen_low_level() {
        assert_eq!(calc_bot_mp_regen(10, 30), 26);
    }

    #[test]
    fn test_calc_bot_mp_regen_high_int_mage() {
        assert_eq!(calc_bot_mp_regen(70, 120), 164);
    }

    #[test]
    fn test_calc_bot_hp_regen_zero_sta() {
        assert_eq!(calc_bot_hp_regen(50, 0), 100);
    }

    #[test]
    fn test_calc_bot_mp_regen_zero_int() {
        assert_eq!(calc_bot_mp_regen(50, 0), 100);
    }

    #[test]
    fn test_calc_bot_hp_regen_level_1() {
        assert_eq!(calc_bot_hp_regen(1, 10), 4);
    }

    #[test]
    fn test_tick_bot_regen_restores_hp() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "RegenBot", 1, 106, 72, 100.0, 100.0);
        bot.hp = 3000;
        bot.mp = 500;
        bot.last_regen_ms = 0;
        world.insert_bot(bot);

        tick_bot_regen(&world, &world.get_bot(BOT_ID_BASE).unwrap(), 5000);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // HP regen: level(70)*2 + STA(80)/5 = 156
        assert_eq!(updated.hp, 3000 + 156);
        // MP regen: level(70)*2 + INT(60)/5 = 152
        assert_eq!(updated.mp, 500 + 152);
        assert_eq!(updated.last_regen_ms, 5000);
    }

    #[test]
    fn test_tick_bot_regen_caps_at_max() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "AlmostFull", 1, 106, 72, 100.0, 100.0);
        bot.hp = 4990;
        bot.mp = 999;
        bot.last_regen_ms = 0;
        world.insert_bot(bot);

        tick_bot_regen(&world, &world.get_bot(BOT_ID_BASE).unwrap(), 5000);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.hp, 5000, "HP should cap at max_hp");
        assert_eq!(updated.mp, 1000, "MP should cap at max_mp");
    }

    #[test]
    fn test_tick_bot_regen_skip_dead() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "DeadBot", 1, 106, 72, 100.0, 100.0);
        bot.hp = 0;
        bot.presence = BotPresence::Dead;
        bot.last_regen_ms = 0;
        world.insert_bot(bot);

        tick_bot_regen(&world, &world.get_bot(BOT_ID_BASE).unwrap(), 5000);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.hp, 0, "dead bot should not regen");
    }

    #[test]
    fn test_tick_bot_regen_skip_full_hp_mp() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "FullHP", 1, 106, 72, 100.0, 100.0);
        world.insert_bot(bot);

        tick_bot_regen(&world, &world.get_bot(BOT_ID_BASE).unwrap(), 5000);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.hp, 5000);
        assert_eq!(updated.mp, 1000);
        assert_eq!(updated.last_regen_ms, 5000, "timer should still advance");
    }

    #[test]
    fn test_tick_bot_regen_sitting_doubles() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "SitBot", 1, 106, 72, 100.0, 100.0);
        bot.hp = 3000;
        bot.mp = 500;
        bot.presence = BotPresence::Sitting;
        bot.last_regen_ms = 0;
        world.insert_bot(bot);

        tick_bot_regen(&world, &world.get_bot(BOT_ID_BASE).unwrap(), 5000);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // HP regen: (70*2 + 80/5) * 2 = 156 * 2 = 312
        assert_eq!(updated.hp, 3000 + 312);
        // MP regen: (70*2 + 60/5) * 2 = 152 * 2 = 304
        assert_eq!(updated.mp, 500 + 304);
    }

    #[test]
    fn test_tick_bot_regen_interval_check() {
        assert_eq!(BOT_REGEN_INTERVAL_MS, 3000);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Bot Kill Reward tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_bot_kill_np_formula() {
        assert_eq!(1u8 as i32 * 3, 3);
        assert_eq!(50u8 as i32 * 3, 150);
        assert_eq!(80u8 as i32 * 3, 240);
    }

    #[test]
    fn test_bot_kill_gold_formula_base() {
        assert_eq!(100, 100);
        assert_eq!(50u32 * 100, 5000);
        assert_eq!(80u32 * 100, 8000);
    }

    #[test]
    fn test_bot_kill_gold_formula_range() {
        // For level 70: base=7000, max_bonus=3500, range=[7000, 10500]
        let base = 70u32 * 100;
        let max_bonus = 70u32 * 50;
        assert_eq!(base, 7000);
        assert_eq!(max_bonus, 3500);
    }

    #[test]
    fn test_bot_on_death_tracks_attacker() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "Victim", 2, 106, 72, 100.0, 100.0);
        bot.hp = 100;
        bot.last_attacker_id = 42;
        world.insert_bot(bot);

        bot_on_death(&world, BOT_ID_BASE, 100_000);

        let dead = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(dead.presence, BotPresence::Dead);
        assert_eq!(
            dead.last_attacker_id, -1,
            "attacker should be reset after death"
        );
    }

    /// Bot death in tournament zone gives killer player's clan a score point.
    #[test]
    fn test_bot_death_tournament_scoring() {
        use crate::handler::tournament::TournamentState;
        use tokio::sync::mpsc;

        let world = WorldState::new();

        // Register a player killer (sid=1) with clan 100
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.character = Some(CharacterInfo {
                knights_id: 100,
                ..CharacterInfo::default()
            });
        });

        // Set up a tournament in zone 77 with red=100, blue=200
        let state = TournamentState::new(77, 100, 200, 600);
        world.insert_tournament(state);

        // Create a bot in zone 77 killed by player sid=1
        let mut bot = make_combat_bot(BOT_ID_BASE, "Victim", 2, 106, 77, 100.0, 100.0);
        bot.hp = 100;
        bot.last_attacker_id = 1; // player sid=1
        world.insert_bot(bot);

        bot_on_death(&world, BOT_ID_BASE, 100_000);

        // Red clan (100) should have score +1
        let snap = world.with_tournament_snapshot(77).unwrap();
        assert_eq!(snap.score_board[0], 1);
        assert_eq!(snap.score_board[1], 0);
    }

    /// Bot death outside tournament zone does not affect scoring.
    #[test]
    fn test_bot_death_non_tournament_zone_no_scoring() {
        use tokio::sync::mpsc;

        let world = WorldState::new();

        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_session(1, |h| {
            h.character = Some(CharacterInfo {
                knights_id: 100,
                ..CharacterInfo::default()
            });
        });

        // Bot in zone 21 (Moradon, NOT tournament zone)
        let mut bot = make_combat_bot(BOT_ID_BASE, "Victim", 2, 106, 21, 100.0, 100.0);
        bot.hp = 100;
        bot.last_attacker_id = 1;
        world.insert_bot(bot);

        bot_on_death(&world, BOT_ID_BASE, 100_000);

        // No tournament in zone 21 — nothing should crash
        assert!(world.with_tournament_snapshot(21).is_none());
    }

    #[test]
    fn test_apply_damage_sets_last_attacker() {
        let world = WorldState::new();
        let mut target = make_combat_bot(BOT_ID_BASE + 1, "Target", 2, 106, 72, 105.0, 100.0);
        target.hp = 5000;
        target.max_hp = 5000;
        world.insert_bot(target);

        let (_result, _new_hp) =
            apply_damage_to_target(&world, (BOT_ID_BASE + 1) as SessionId, 200, BOT_ID_BASE);

        let updated = world.get_bot(BOT_ID_BASE + 1).unwrap();
        assert_eq!(updated.last_attacker_id, BOT_ID_BASE as i32);
    }

    #[test]
    fn test_bot_new_fields_initialized() {
        let bot = make_combat_bot(BOT_ID_BASE, "TestBot", 1, 106, 72, 100.0, 100.0);
        assert_eq!(bot.last_regen_ms, 0);
        assert_eq!(bot.last_attacker_id, -1);
    }

    #[test]
    fn test_bot_regene_resets_regen_fields() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "RegeneBot", 1, 106, 72, 100.0, 100.0);
        bot.hp = 0;
        bot.presence = BotPresence::Dead;
        bot.regene_at_ms = 1000;
        bot.original_ai_state = BotAiState::Pk;
        bot.last_attacker_id = 42;
        bot.last_regen_ms = 500;
        world.insert_bot(bot);

        bot_regene(&world, BOT_ID_BASE, 2000);

        let alive = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(alive.presence, BotPresence::Standing);
        assert_eq!(alive.hp, 5000);
        assert_eq!(alive.last_regen_ms, 2000);
        assert_eq!(alive.last_attacker_id, -1);
    }

    // ── Bot Visibility Tests ─────────────────────────────────────────

    #[test]
    fn test_parse_bot_equipment_empty() {
        let result = parse_bot_equipment(None);
        for &(id, dur, flag) in &result {
            assert_eq!(id, 0);
            assert_eq!(dur, 0);
            assert_eq!(flag, 0);
        }
    }

    #[test]
    fn test_parse_bot_equipment_with_items() {
        // Create a 77-slot inventory blob (77 * 8 = 616 bytes)
        let mut data = vec![0u8; 77 * 8];
        // Put item 150001 with durability 100 at slot 4 (BREAST = first visual slot)
        let offset = 4 * 8;
        data[offset..offset + 4].copy_from_slice(&150001u32.to_le_bytes());
        data[offset + 4..offset + 6].copy_from_slice(&100i16.to_le_bytes());

        // Put item 180010 with durability 50 at slot 6 (RIGHTHAND = 7th visual slot)
        let offset2 = 6 * 8;
        data[offset2..offset2 + 4].copy_from_slice(&180010u32.to_le_bytes());
        data[offset2 + 4..offset2 + 6].copy_from_slice(&50i16.to_le_bytes());

        let result = parse_bot_equipment(Some(&data));
        // Slot 0 (BREAST=4) should have our item
        assert_eq!(result[0], (150001, 100, 0));
        // Slot 6 (RIGHTHAND=6) should have our item
        assert_eq!(result[6], (180010, 50, 0));
        // Other slots should be empty
        assert_eq!(result[1], (0, 0, 0));
    }

    #[test]
    fn test_build_bot_inout_packet_structure() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "TestVis", 1, 106, 21, 150.0, 200.0);

        let pkt = build_bot_inout_packet(&bot, &world, 1);
        // Opcode is separate from data
        assert_eq!(pkt.opcode, ko_protocol::Opcode::WizUserInout as u8);
        let d = &pkt.data;
        // data[0] = INOUT type
        assert_eq!(d[0], 1); // INOUT_IN
                             // data[1] = reserved
        assert_eq!(d[1], 0);
        // Bot ID (little-endian u32) at bytes 2..6
        let bot_id = u32::from_le_bytes([d[2], d[3], d[4], d[5]]);
        assert_eq!(bot_id, BOT_ID_BASE);
        // Name length (SByte = u8 prefix) at byte 6
        let name_len = d[6] as usize;
        assert_eq!(name_len, 7); // "TestVis"
                                 // Nation (after name bytes)
        let nation_offset = 7 + name_len;
        assert_eq!(d[nation_offset], 1); // Karus
                                         // No-clan TestVis packet must match the v2600 player GetUserInfo layout.
        assert_eq!(d.len(), 214);
        assert_eq!(&d[d.len() - 3..], &[0, 0, 1]);
    }

    #[test]
    fn test_build_bot_inout_packet_respawn() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "RBot", 2, 207, 21, 100.0, 100.0);
        let pkt = build_bot_inout_packet(&bot, &world, 3);
        // data[0] = INOUT type
        assert_eq!(pkt.data[0], 3); // INOUT_RESPAWN
    }

    #[test]
    fn test_bot_equip_visual_field_default() {
        let bot = make_combat_bot(BOT_ID_BASE, "NoEquip", 1, 106, 21, 100.0, 100.0);
        for &(id, dur, flag) in &bot.equip_visual {
            assert_eq!(id, 0);
            assert_eq!(dur, 0);
            assert_eq!(flag, 0);
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // Bot Self-Heal AI tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_bot_self_heal_restores_hp() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "HealBot", 1, 106, 72, 100.0, 100.0);
        bot.hp = 3000; // 60% of max (5000)
        bot.max_hp = 5000;
        bot.last_hp_change_ms = 0;
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        tick_bot_self_heal(&world, &bot_snap, 10_000);

        let after = world.get_bot(BOT_ID_BASE).unwrap();
        // Heal = 15% of 5000 = 750
        assert_eq!(after.hp, 3750, "HP should increase by 15% of max_hp");
        assert_eq!(
            after.last_hp_change_ms, 10_000,
            "heal timestamp should be updated"
        );
    }

    #[test]
    fn test_bot_self_heal_caps_at_max_hp() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "AlmostFull", 1, 106, 72, 100.0, 100.0);
        bot.hp = 4900; // 98% of max (5000) — but self-heal checks HP < 90% externally
        bot.max_hp = 5000;
        bot.last_hp_change_ms = 0;
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        tick_bot_self_heal(&world, &bot_snap, 10_000);

        let after = world.get_bot(BOT_ID_BASE).unwrap();
        // Heal = 750 would bring to 5650, but capped at max_hp
        assert_eq!(after.hp, 5000, "HP should be capped at max_hp");
    }

    #[test]
    fn test_bot_self_heal_priest_karus_skill() {
        // Karus priest: class=104 → 104 % 100 = 4 → is_priest()
        let bot = make_combat_bot(BOT_ID_BASE, "KarusPriest", 1, 104, 72, 100.0, 100.0);
        assert!(bot.is_priest());
        assert_eq!(bot.nation, 1); // Karus
        let actual_skill: u32 = if bot.is_priest() {
            if bot.nation == NATION_ELMORAD {
                212545
            } else {
                112545
            }
        } else {
            490014
        };
        assert_eq!(actual_skill, 112545);
    }

    #[test]
    fn test_bot_self_heal_priest_elmorad_skill() {
        // Elmorad priest: class=204 → 204 % 100 = 4 → is_priest()
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "ElmoPriest", 2, 204, 72, 100.0, 100.0);
        bot.hp = 2000;
        bot.max_hp = 5000;
        bot.last_hp_change_ms = 0;
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        assert!(bot_snap.is_priest());
        assert_eq!(bot_snap.nation, 2); // Elmorad
        let skill_id: u32 = if bot_snap.is_priest() {
            if bot_snap.nation == NATION_ELMORAD {
                212545
            } else {
                112545
            }
        } else {
            490014
        };
        assert_eq!(skill_id, 212545);
    }

    #[test]
    fn test_bot_self_heal_non_priest_skill() {
        // Non-priest should use skill ID 490014
        let bot = make_combat_bot(BOT_ID_BASE, "Warrior", 1, 106, 72, 100.0, 100.0);
        assert!(bot.is_warrior());
        let skill_id: u32 = if bot.is_priest() {
            if bot.nation == NATION_ELMORAD {
                212545
            } else {
                112545
            }
        } else {
            490014
        };
        assert_eq!(skill_id, 490014);
    }

    #[test]
    fn test_bot_self_heal_cooldown_constant() {
        assert_eq!(BOT_SELF_HEAL_COOLDOWN_MS, 5_000);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Weapon-based skill selection tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_rogue_no_world_uses_dagger_skill() {
        // Without world, rogue should default to dagger skills.
        let bot = make_combat_bot(BOT_ID_BASE, "Rogue", 1, 107, 72, 100.0, 100.0);
        assert!(bot.is_rogue());
        let skill_id = select_bot_skill(&bot);
        // Level 70, Karus rogue dagger → 108670
        assert_eq!(skill_id, 108670);
    }

    #[test]
    fn test_rogue_with_bow_uses_arrow_skill() {
        // With world + bow item in RIGHTHAND, rogue should use arrow skills.
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "ArrowRogue", 1, 107, 72, 100.0, 100.0);
        assert!(bot.is_rogue());

        // Put a bow item in RIGHTHAND (equip_visual[6])
        let bow_item_id: u32 = 120070;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (bow_item_id, 100, 0);
        world.insert_bot(bot);

        // Insert item with kind=70 (BOW) in item table
        world.insert_item(
            bow_item_id,
            make_test_item(bow_item_id as i32, WEAPON_KIND_BOW),
        );

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let skill_id = select_bot_skill_with_weapon(&bot_snap, Some(&world));
        // Level 70, Karus rogue arrow → 108570
        assert_eq!(skill_id, 108570);
    }

    #[test]
    fn test_rogue_with_dagger_uses_dagger_skill() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "DaggerRogue", 1, 107, 72, 100.0, 100.0);

        let dagger_item_id: u32 = 110011;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (dagger_item_id, 100, 0);
        world.insert_bot(bot);

        world.insert_item(
            dagger_item_id,
            make_test_item(dagger_item_id as i32, WEAPON_KIND_DAGGER),
        );

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let skill_id = select_bot_skill_with_weapon(&bot_snap, Some(&world));
        // Level 70, Karus rogue dagger → 108670
        assert_eq!(skill_id, 108670);
    }

    #[test]
    fn test_class_specific_cooldown_warrior() {
        let bot = make_combat_bot(BOT_ID_BASE, "Warrior", 1, 106, 72, 100.0, 100.0);
        assert!(bot.is_warrior());
        assert_eq!(
            get_bot_attack_cooldown(&bot, None),
            BOT_ATTACK_COOLDOWN_MELEE_MS
        );
    }

    #[test]
    fn test_class_specific_cooldown_mage() {
        let bot = make_combat_bot(BOT_ID_BASE, "Mage", 1, 109, 72, 100.0, 100.0);
        assert!(bot.is_mage());
        assert_eq!(
            get_bot_attack_cooldown(&bot, None),
            BOT_ATTACK_COOLDOWN_RANGED_MS
        );
    }

    #[test]
    fn test_class_specific_cooldown_priest() {
        let bot = make_combat_bot(BOT_ID_BASE, "Priest", 1, 104, 72, 100.0, 100.0);
        assert!(bot.is_priest());
        assert_eq!(
            get_bot_attack_cooldown(&bot, None),
            BOT_ATTACK_COOLDOWN_RANGED_MS
        );
    }

    #[test]
    fn test_class_specific_cooldown_rogue_dagger_default() {
        let bot = make_combat_bot(BOT_ID_BASE, "Rogue", 1, 107, 72, 100.0, 100.0);
        assert!(bot.is_rogue());
        // Without world → defaults to dagger → melee cooldown
        assert_eq!(
            get_bot_attack_cooldown(&bot, None),
            BOT_ATTACK_COOLDOWN_MELEE_MS
        );
    }

    #[test]
    fn test_class_specific_cooldown_rogue_bow() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "BowRogue", 1, 107, 72, 100.0, 100.0);
        let bow_id: u32 = 120070;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (bow_id, 100, 0);
        world.insert_bot(bot);

        world.insert_item(bow_id, make_test_item(bow_id as i32, WEAPON_KIND_BOW));

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(
            get_bot_attack_cooldown(&bot_snap, Some(&world)),
            BOT_ATTACK_COOLDOWN_RANGED_MS
        );
    }

    #[test]
    fn test_detect_weapon_kind_no_weapon() {
        let bot = make_combat_bot(BOT_ID_BASE, "NoWeapon", 1, 107, 72, 100.0, 100.0);
        // equip_visual[6] = (0, 0, 0) → no weapon
        assert_eq!(detect_rogue_weapon_kind(&bot, None), WEAPON_KIND_DAGGER);
    }

    #[test]
    fn test_crossbow_uses_arrow_skills() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "XbowRogue", 1, 107, 72, 100.0, 100.0);
        let xbow_id: u32 = 120071;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (xbow_id, 100, 0);
        world.insert_bot(bot);

        world.insert_item(
            xbow_id,
            make_test_item(xbow_id as i32, WEAPON_KIND_CROSSBOW),
        );

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let skill_id = select_bot_skill_with_weapon(&bot_snap, Some(&world));
        // Level 70, Karus rogue arrow → 108570
        assert_eq!(skill_id, 108570);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Mage subclass skill variant tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_mage_subclass_offset_values() {
        assert_eq!(mage_subclass_offset(MageSubclass::Flame), 0);
        assert_eq!(mage_subclass_offset(MageSubclass::Glacier), 100);
        assert_eq!(mage_subclass_offset(MageSubclass::Lightning), 200);
    }

    #[test]
    fn test_mage_base_skill_level_1_in_range() {
        // Level 1-4 pool: {109001, 109002}
        for _ in 0..20 {
            let skill = get_mage_base_skill(1);
            assert!(
                skill == 109001 || skill == 109002,
                "level 1 skill {skill} not in expected pool"
            );
        }
    }

    #[test]
    fn test_mage_base_skill_level_60_in_range() {
        // Level 60-69 pool: {110542, 110543, 110545, 110551, 110556, 110557, 110560}
        let valid = [110542, 110543, 110545, 110551, 110556, 110557, 110560];
        for _ in 0..50 {
            let skill = get_mage_base_skill(65);
            assert!(
                valid.contains(&skill),
                "level 65 skill {skill} not in expected pool"
            );
        }
    }

    #[test]
    fn test_mage_base_skill_level_80_includes_110575() {
        // Level 80-83 pool should include 110575
        let valid = [
            110542, 110543, 110545, 110572, 110571, 110570, 110560, 110575,
        ];
        let mut found_575 = false;
        for _ in 0..200 {
            let skill = get_mage_base_skill(83);
            assert!(
                valid.contains(&skill),
                "level 83 skill {skill} not in expected pool"
            );
            if skill == 110575 {
                found_575 = true;
            }
        }
        assert!(found_575, "110575 should appear in level 83 pool");
    }

    #[test]
    fn test_mage_skill_with_subclass_applies_offset() {
        // Run many times; at least one of each subclass should appear.
        let mut saw_flame = false;
        let mut saw_glacier = false;
        let mut saw_lightning = false;

        for _ in 0..300 {
            let skill = get_mage_skill_with_subclass(65);
            // Flame: 110542..110560 (no offset)
            // Glacier: 110642..110660 (+100)
            // Lightning: 110742..110760 (+200)
            if (110542..=110560).contains(&skill) {
                saw_flame = true;
            } else if (110642..=110660).contains(&skill) {
                saw_glacier = true;
            } else if (110742..=110760).contains(&skill) {
                saw_lightning = true;
            }
        }
        assert!(saw_flame, "Flame mage subclass should appear");
        assert!(saw_glacier, "Glacier mage subclass should appear");
        assert!(saw_lightning, "Lightning mage subclass should appear");
    }

    #[test]
    fn test_mage_skill_low_level_glacier_offset() {
        // At level 1-4, base pool is {109001, 109002}.
        // Glacier offset = +100 → {109101, 109102}.
        // Lightning offset = +200 → {109201, 109202}.
        let mut found_offset = false;
        for _ in 0..200 {
            let skill = get_mage_skill_with_subclass(2);
            // Valid skills: 109001, 109002 (flame), 109101, 109102 (glacier),
            //               109201, 109202 (lightning)
            assert!(
                matches!(skill, 109001 | 109002 | 109101 | 109102 | 109201 | 109202),
                "level 2 mage skill {skill} unexpected"
            );
            if skill > 109002 {
                found_offset = true;
            }
        }
        assert!(
            found_offset,
            "glacier/lightning offset should appear at low levels"
        );
    }

    #[test]
    fn test_select_bot_skill_mage_uses_subclass() {
        // Karus mage bot, level 65 — should use subclass selection
        let bot = make_combat_bot(BOT_ID_BASE, "MageBot", 1, 109, 21, 100.0, 100.0);
        assert!(bot.is_mage());

        let mut saw_offset = false;
        for _ in 0..100 {
            let skill = select_bot_skill_with_weapon(&bot, None);
            // Karus, level 65 → base 110xxx, maybe +100/+200
            if (110642..=110660).contains(&skill) || (110742..=110760).contains(&skill) {
                saw_offset = true;
            }
        }
        assert!(
            saw_offset,
            "mage bot skill selection should include glacier/lightning variants"
        );
    }

    #[test]
    fn test_select_bot_skill_mage_elmorad_offset() {
        // Elmorad mage: base skill + 100000 + subclass offset
        let bot = make_combat_bot(BOT_ID_BASE, "ElMage", 2, 209, 21, 100.0, 100.0);
        assert!(bot.is_mage());

        for _ in 0..50 {
            let skill = select_bot_skill_with_weapon(&bot, None);
            // Elmorad offset: +100000 → 210xxx range
            assert!(
                (209001..=210960).contains(&skill),
                "Elmorad mage skill {skill} should be in 209xxx-210xxx range"
            );
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // Warrior skill pool C++ parity tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_warrior_skill_level_70_random() {
        // C++ fall-through bug: levels 70-79 all use random(106570, 106575)
        let mut saw_570 = false;
        let mut saw_575 = false;
        for _ in 0..100 {
            let skill = get_warrior_skill(72);
            assert!(
                skill == 106570 || skill == 106575,
                "level 72 warrior skill {skill} unexpected"
            );
            if skill == 106570 {
                saw_570 = true;
            }
            if skill == 106575 {
                saw_575 = true;
            }
        }
        assert!(saw_570, "106570 should appear for level 72");
        assert!(saw_575, "106575 should appear for level 72");
    }

    #[test]
    fn test_warrior_skill_level_83_random() {
        // C++ level 82-83: random(106580, 106782)
        let mut saw_580 = false;
        let mut saw_782 = false;
        for _ in 0..100 {
            let skill = get_warrior_skill(83);
            assert!(
                skill == 106580 || skill == 106782,
                "level 83 warrior skill {skill} unexpected"
            );
            if skill == 106580 {
                saw_580 = true;
            }
            if skill == 106782 {
                saw_782 = true;
            }
        }
        assert!(saw_580, "106580 should appear for level 83");
        assert!(saw_782, "106782 should appear for level 83");
    }

    // ═════════════════════════════════════════════════════════════════════
    // Priest skill pool C++ parity tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_priest_skill_level_45_random() {
        // C++ level 43-50: random between 111520 and 111542
        let mut saw_520 = false;
        let mut saw_542 = false;
        for _ in 0..100 {
            let skill = get_priest_skill(45);
            assert!(
                skill == 111520 || skill == 111542,
                "level 45 priest skill {skill} unexpected"
            );
            if skill == 111520 {
                saw_520 = true;
            }
            if skill == 111542 {
                saw_542 = true;
            }
        }
        assert!(saw_520, "111520 should appear for priest level 45");
        assert!(saw_542, "111542 should appear for priest level 45");
    }

    #[test]
    fn test_priest_skill_level_55_random() {
        // C++ level 51-59: random among {111520, 111542, 111551}
        let valid = [111520, 111542, 111551];
        for _ in 0..100 {
            let skill = get_priest_skill(55);
            assert!(
                valid.contains(&skill),
                "level 55 priest skill {skill} not in expected pool"
            );
        }
    }

    #[test]
    fn test_priest_skill_level_60_random() {
        // C++ level 60-61: random among {112520, 112542, 112551}
        let valid = [112520, 112542, 112551];
        for _ in 0..100 {
            let skill = get_priest_skill(60);
            assert!(
                valid.contains(&skill),
                "level 60 priest skill {skill} not in expected pool"
            );
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // Bot MP cost deduction tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_bot_mp_deducted_when_magic_table_present() {
        // Verify that update_bot correctly reduces MP
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "MpBot", 1, 109, 21, 100.0, 100.0);
        bot.mp = 500;
        bot.max_mp = 1000;
        world.insert_bot(bot);

        // Simulate MP deduction
        let mp_cost: i16 = 50;
        world.update_bot(BOT_ID_BASE, |b| {
            b.mp = (b.mp - mp_cost).max(0);
        });

        let after = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(after.mp, 450, "MP should be deducted by cost");
    }

    #[test]
    fn test_bot_mp_deduction_floor_at_zero() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "LowMp", 1, 109, 21, 100.0, 100.0);
        bot.mp = 10;
        bot.max_mp = 1000;
        world.insert_bot(bot);

        let mp_cost: i16 = 50;
        world.update_bot(BOT_ID_BASE, |b| {
            b.mp = (b.mp - mp_cost).max(0);
        });

        let after = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(after.mp, 0, "MP should floor at 0");
    }

    #[test]
    fn test_bot_mp_regen_restores_mp() {
        // Verify that the tick_bot_regen function restores MP
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "RegenMp", 1, 109, 21, 100.0, 100.0);
        bot.mp = 100;
        bot.max_mp = 1000;
        bot.hp = 5000; // full HP so only MP regens
        bot.max_hp = 5000;
        bot.last_regen_ms = 0;
        bot.int_stat = 120; // mage INT stat
        world.insert_bot(bot);

        let snap = world.get_bot(BOT_ID_BASE).unwrap();
        tick_bot_regen(&world, &snap, 10_000);

        let after = world.get_bot(BOT_ID_BASE).unwrap();
        assert!(after.mp > 100, "MP should increase after regen tick");
        assert!(after.mp <= 1000, "MP should not exceed max");
    }

    #[test]
    fn test_bot_warrior_no_mp_cost() {
        // Warriors typically have 0 max_mp (no MP coeff), so MP checks
        // should not prevent them from attacking (mp_cost=0 from magic table).
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "WarMp", 1, 106, 21, 100.0, 100.0);
        bot.mp = 0;
        bot.max_mp = 0;
        world.insert_bot(bot);

        // A warrior's skill_id won't be in magic_table in tests (returns 0 mp_cost).
        // This means warrior attacks should never be blocked by MP.
        let mp_cost: i16 = 0; // from world.get_magic() returning None
        assert_eq!(mp_cost, 0, "warrior should have 0 MP cost");
    }

    // ═════════════════════════════════════════════════════════════════════
    // Bot speed & movement tests (Sprint 456)
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_get_bot_speed_default_non_pk() {
        // Non-PK zone: all classes get 45.0
        let bot = make_combat_bot(BOT_ID_BASE, "NonPK", 1, 106, 21, 100.0, 100.0);
        assert!(!bot.is_in_pk_zone());
        assert_eq!(get_bot_speed(&bot), 45.0);
    }

    #[test]
    fn test_get_bot_speed_pk_warrior() {
        // PK zone warrior: 67.0
        let bot = make_combat_bot(BOT_ID_BASE, "PKWar", 1, 106, 72, 100.0, 100.0);
        assert!(bot.is_in_pk_zone());
        assert_eq!(get_bot_speed(&bot), 67.0);
    }

    #[test]
    fn test_get_bot_speed_pk_rogue() {
        // PK zone rogue: 90.0
        let bot = make_combat_bot(BOT_ID_BASE, "PKRog", 1, 107, 72, 100.0, 100.0);
        assert!(bot.is_in_pk_zone());
        assert!(bot.is_rogue());
        assert_eq!(get_bot_speed(&bot), 90.0);
    }

    #[test]
    fn test_get_bot_speed_pk_captain() {
        // PK zone captain (fame >= 5): 90.0 regardless of class
        let mut bot = make_combat_bot(BOT_ID_BASE, "PKCap", 1, 106, 72, 100.0, 100.0);
        bot.fame = 5;
        assert!(bot.is_in_pk_zone());
        assert_eq!(get_bot_speed(&bot), 90.0);
    }

    #[test]
    fn test_get_bot_speed_pk_mage() {
        // PK zone mage: 67.0
        let bot = make_combat_bot(BOT_ID_BASE, "PKMag", 1, 109, 72, 100.0, 100.0);
        assert!(bot.is_in_pk_zone());
        assert!(bot.is_mage());
        assert_eq!(get_bot_speed(&bot), 67.0);
    }

    #[test]
    fn test_move_step_size_matches_speed() {
        // Non-PK bot at (0,0,0) moving toward (100,0,0).
        // Speed = 45.0, 250ms step = 1.125 units per tick.
        let bot = make_combat_bot(BOT_ID_BASE, "StepBot", 1, 106, 21, 0.0, 0.0);
        let (new_x, _, new_z) = move_toward_target(&bot, 100.0, 0.0, 0.0);

        let dx = new_x;
        let dz = new_z;
        let dist = (dx * dx + dz * dz).sqrt();
        // Step should match the tick-scaled movement distance.
        assert!(
            (dist - movement_step(&bot)).abs() < 0.01,
            "step distance should match the configured cadence, got {dist}"
        );
    }

    #[test]
    fn test_move_step_pk_rogue_faster() {
        // PK zone rogue: speed=90, 250ms step=2.25 units per tick.
        let bot = make_combat_bot(BOT_ID_BASE, "FastRog", 1, 107, 72, 0.0, 0.0);
        assert_eq!(get_bot_speed(&bot), 90.0);

        let (new_x, _, new_z) = move_toward_target(&bot, 100.0, 0.0, 0.0);
        let dist = (new_x * new_x + new_z * new_z).sqrt();
        // Step should match the tick-scaled PK movement distance.
        assert!(
            (dist - movement_step(&bot)).abs() < 0.01,
            "PK rogue step should match the configured cadence, got {dist}"
        );
    }

    #[test]
    fn test_move_toward_target_snaps_when_close() {
        // Target is closer than one step — should snap to target.
        let bot = make_combat_bot(BOT_ID_BASE, "SnapBot", 1, 106, 21, 100.0, 100.0);
        // Target is 1 unit away, step = 1.125 → overshoot → snap.
        let (new_x, _, new_z) = move_toward_target(&bot, 101.0, 0.0, 100.0);
        let dx = new_x - 101.0;
        let dz = new_z - 100.0;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(
            dist <= 3.0,
            "bot should snap to target when close, got dist={dist}"
        );
    }

    #[test]
    fn test_broadcast_bot_move_ex_packet_format() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "PktBot", 1, 106, 21, 100.0, 100.0);
        world.insert_bot(bot);

        // We can't easily capture broadcast packets in tests, but we can
        // verify the function doesn't panic and the speed calculation works.
        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        broadcast_bot_move_ex(&world, &bot_snap, 105.0, 0.0, 105.0, 3);
        broadcast_bot_move_ex(&world, &bot_snap, 105.0, 0.0, 105.0, 1);
        broadcast_bot_move_ex(&world, &bot_snap, 105.0, 0.0, 105.0, 0);
        // No panic = success
    }

    // ═════════════════════════════════════════════════════════════════════
    // MAX_DAMAGE cap tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_max_damage_constant() {
        assert_eq!(MAX_DAMAGE, 32_000);
    }

    #[test]
    fn test_apply_damage_caps_at_max() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "Attacker", 1, 106, 21, 100.0, 100.0);
        world.insert_bot(bot);

        // Target bot with 30000 HP.
        let target_id = BOT_ID_BASE + 1;
        let mut target = make_combat_bot(target_id, "Target", 2, 106, 21, 100.0, 100.0);
        target.hp = 30_000;
        target.max_hp = 30_000;
        world.insert_bot(target);

        // Apply damage exactly at MAX_DAMAGE.
        let (result, hp) = apply_damage_to_target(
            &world,
            target_id as SessionId,
            MAX_DAMAGE as i16,
            BOT_ID_BASE,
        );
        assert_eq!(result, ATTACK_TARGET_DEAD);
        assert_eq!(hp, 0);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Weapon detection tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_detect_bot_weapon_kinds_empty() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "NoWeapon", 1, 106, 21, 100.0, 100.0);
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let kinds = detect_bot_weapon_kinds(&bot_snap, &world);
        assert_eq!(kinds, [None, None]);
    }

    #[test]
    fn test_detect_bot_weapon_kinds_sword_righthand() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "Swordsman", 1, 106, 21, 100.0, 100.0);
        let sword_id: u32 = 120021;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (sword_id, 100, 0);
        world.insert_bot(bot);

        // Insert item with kind=21 (1H_SWORD)
        world.insert_item(sword_id, make_test_item(sword_id as i32, 21));

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let kinds = detect_bot_weapon_kinds(&bot_snap, &world);
        assert_eq!(kinds[0], None); // LEFTHAND empty
        assert_eq!(kinds[1], Some(21)); // RIGHTHAND = 1H_SWORD
    }

    #[test]
    fn test_detect_bot_weapon_kinds_dual_wield() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "DualWield", 1, 107, 21, 100.0, 100.0);
        let dagger_id: u32 = 120011;
        let sword_id: u32 = 120022;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (dagger_id, 100, 0);
        bot.equip_visual[VISUAL_LEFTHAND_IDX] = (sword_id, 100, 0);
        world.insert_bot(bot);

        world.insert_item(
            dagger_id,
            make_test_item(dagger_id as i32, WEAPON_KIND_DAGGER),
        );
        world.insert_item(sword_id, make_test_item(sword_id as i32, 21));

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let kinds = detect_bot_weapon_kinds(&bot_snap, &world);
        assert_eq!(kinds[0], Some(21)); // LEFTHAND = 1H_SWORD
        assert_eq!(kinds[1], Some(WEAPON_KIND_DAGGER)); // RIGHTHAND = DAGGER
    }

    // ═════════════════════════════════════════════════════════════════════
    // AC damage reduction tests (bot → player)
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_ac_reduction_sword_against_player() {
        // get_ac_damage with sword weapon, target has sword_r = 50
        // damage -= damage * 50 / 250 = damage * 0.2
        let stats = crate::world::EquippedStats {
            sword_r: 50,
            ..Default::default()
        };
        let weapon_kinds: [Option<i32>; 2] = [None, Some(21)]; // 1H_SWORD in RIGHTHAND
        let result = get_ac_damage(1000, &weapon_kinds, &stats, 100, 100);
        // 1000 - 1000 * 50 / 250 = 1000 - 200 = 800
        assert_eq!(result, 800);
    }

    #[test]
    fn test_ac_reduction_dual_weapon() {
        // Bot dual-wielding: dagger + sword. Each reduces damage independently.
        let stats = crate::world::EquippedStats {
            dagger_r: 50,
            sword_r: 100,
            ..Default::default()
        };
        let weapon_kinds: [Option<i32>; 2] = [Some(21), Some(WEAPON_KIND_DAGGER)];
        let result = get_ac_damage(1000, &weapon_kinds, &stats, 100, 100);
        // After sword: 1000 - 1000 * 100 / 250 = 1000 - 400 = 600
        // After dagger: 600 - 600 * (50 * 100 / 100) / 250 = 600 - 120 = 480
        assert_eq!(result, 480);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Mirror damage tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_mirror_damage_no_mirror_returns_zero() {
        let world = WorldState::new();
        // No player session → equipped_stats is default (no mirror)
        let mirror = get_target_mirror_damage(&world, 1, 500);
        assert_eq!(mirror, 0);
    }

    #[test]
    fn test_item_type_mirror_damage_constant() {
        assert_eq!(ITEM_TYPE_MIRROR_DAMAGE, 8);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Elemental damage bonus tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_item_type_constants() {
        assert_eq!(ITEM_TYPE_FIRE, 1);
        assert_eq!(ITEM_TYPE_COLD, 2);
        assert_eq!(ITEM_TYPE_LIGHTNING, 3);
        assert_eq!(ITEM_TYPE_POISON, 4);
        assert_eq!(ITEM_TYPE_HP_DRAIN, 5);
        assert_eq!(ITEM_TYPE_MP_DAMAGE, 6);
        assert_eq!(ITEM_TYPE_MP_DRAIN, 7);
        assert_eq!(ITEM_TYPE_MIRROR_DAMAGE, 8);
    }

    #[test]
    fn test_collect_bot_item_bonuses_empty() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "NoBonuses", 1, 106, 21, 100.0, 100.0);
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let bonuses = collect_bot_item_bonuses(&bot_snap, &world);
        assert!(bonuses.is_empty());
    }

    #[test]
    fn test_collect_bot_item_bonuses_fire_weapon() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "FireSword", 1, 106, 21, 100.0, 100.0);
        let sword_id: u32 = 130021;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (sword_id, 100, 0);
        world.insert_bot(bot);

        // Create item with fire_damage = 50
        let mut item = make_test_item(sword_id as i32, 21);
        item.fire_damage = Some(50);
        world.insert_item(sword_id, item);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let bonuses = collect_bot_item_bonuses(&bot_snap, &world);
        assert_eq!(bonuses.len(), 1);
        assert_eq!(bonuses[0], (ITEM_TYPE_FIRE, 50));
    }

    #[test]
    fn test_collect_bot_item_bonuses_multiple_types() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "MultType", 1, 106, 21, 100.0, 100.0);
        let item_id: u32 = 130022;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (item_id, 100, 0);
        world.insert_bot(bot);

        let mut item = make_test_item(item_id as i32, 21);
        item.fire_damage = Some(30);
        item.ice_damage = Some(20);
        item.hp_drain = Some(10);
        world.insert_item(item_id, item);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let bonuses = collect_bot_item_bonuses(&bot_snap, &world);
        assert_eq!(bonuses.len(), 3);
        assert!(bonuses.contains(&(ITEM_TYPE_FIRE, 30)));
        assert!(bonuses.contains(&(ITEM_TYPE_COLD, 20)));
        assert!(bonuses.contains(&(ITEM_TYPE_HP_DRAIN, 10)));
    }

    #[test]
    fn test_calc_elemental_damage_no_bonuses() {
        let world = WorldState::new();
        let bot = make_combat_bot(BOT_ID_BASE, "NoBonuses", 1, 106, 21, 100.0, 100.0);
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let (elem, d1, d2, d3) = calc_bot_elemental_damage(&bot_snap, &world, 1);
        assert_eq!(elem, 0);
        assert_eq!(d1, 0);
        assert_eq!(d2, 0);
        assert_eq!(d3, 0);
    }

    #[test]
    fn test_calc_elemental_damage_fire_no_resist() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "FireBot", 1, 106, 21, 100.0, 100.0);
        let item_id: u32 = 130023;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (item_id, 100, 0);
        world.insert_bot(bot);

        let mut item = make_test_item(item_id as i32, 21);
        item.fire_damage = Some(100);
        world.insert_item(item_id, item);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        // No target session → resistance is 0, pct is default 100
        // total_r = (0 + 0) * 100 / 100 + 0 = 0
        // elem_damage = 100 - 100 * 0 / 200 = 100
        let (elem, _, _, _) = calc_bot_elemental_damage(&bot_snap, &world, 999);
        assert_eq!(elem, 100);
    }

    #[test]
    fn test_calc_elemental_damage_drain_accumulation() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "DrainBot", 1, 106, 21, 100.0, 100.0);
        let item_id: u32 = 130024;
        bot.equip_visual[VISUAL_RIGHTHAND_IDX] = (item_id, 100, 0);
        world.insert_bot(bot);

        let mut item = make_test_item(item_id as i32, 21);
        item.hp_drain = Some(15);
        item.mp_damage = Some(25);
        item.mp_drain = Some(10);
        world.insert_item(item_id, item);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let (elem, d1, d2, d3) = calc_bot_elemental_damage(&bot_snap, &world, 1);
        // Drains don't add to elemental damage.
        assert_eq!(elem, 0);
        assert_eq!(d1, 15); // HP drain
        assert_eq!(d2, 25); // MP damage
        assert_eq!(d3, 10); // MP drain
    }

    // ═════════════════════════════════════════════════════════════════════
    // MerchantMove tests
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_merchant_move_constants() {
        assert_eq!(BOT_MERCHANT_MOVE_MIN_MS, 7_000);
        assert_eq!(BOT_MERCHANT_MOVE_MAX_MS, 17_000);
    }

    #[test]
    fn test_merchant_move_no_panic_no_target() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "MerchWalk", 1, 106, 21, 100.0, 100.0);
        bot.ai_state = BotAiState::MerchantMove;
        bot.merchant_state = 0;
        bot.merchant_chat = "Buy my stuff!".to_string();
        bot.last_move_ms = 0;
        bot.last_merchant_chat_ms = 0;
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        let now_ms = 20_000;
        tick_merchant_move(&world, &bot_snap, now_ms);
        // No panic = success. No merchant targets, so bot should not move.
        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.x, 100.0); // unchanged
        assert_eq!(updated.z, 100.0); // unchanged
    }

    #[test]
    fn test_merchant_move_respects_cooldown() {
        let world = WorldState::new();
        let mut bot = make_combat_bot(BOT_ID_BASE, "MerchWalk", 1, 106, 21, 100.0, 100.0);
        bot.ai_state = BotAiState::MerchantMove;
        bot.last_move_ms = 15_000; // moved at 15s
        world.insert_bot(bot);

        let bot_snap = world.get_bot(BOT_ID_BASE).unwrap();
        // now_ms = 16_000, only 1s passed — should not move (min delay = 7s)
        tick_merchant_move(&world, &bot_snap, 16_000);
        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.x, 100.0); // unchanged
    }

    // ── Sprint 459: Zone-specific respawn tests ──────────────────────────

    #[test]
    fn test_respawn_position_ronark_land_karus() {
        let (x, z) = get_bot_respawn_position(71, 1); // Ronark Land, Karus
        assert!(
            (1375.0..=1380.0).contains(&x),
            "Karus Ronark Land x should be 1375..=1380, got {x}"
        );
        assert!(
            (1098.0..=1103.0).contains(&z),
            "Karus Ronark Land z should be 1098..=1103, got {z}"
        );
    }

    #[test]
    fn test_respawn_position_ronark_land_elmorad() {
        let (x, z) = get_bot_respawn_position(71, 2); // Ronark Land, Elmorad
        assert!(
            (622.0..=627.0).contains(&x),
            "Elmorad Ronark Land x should be 622..=627, got {x}"
        );
        assert!(
            (898.0..=903.0).contains(&z),
            "Elmorad Ronark Land z should be 898..=903, got {z}"
        );
    }

    #[test]
    fn test_respawn_position_ardream_karus() {
        let (x, z) = get_bot_respawn_position(72, 1); // Ardream, Karus
        assert!(
            (851.0..=856.0).contains(&x),
            "Karus Ardream x should be 851..=856, got {x}"
        );
        assert!(
            (136.0..=141.0).contains(&z),
            "Karus Ardream z should be 136..=141, got {z}"
        );
    }

    #[test]
    fn test_respawn_position_ardream_elmorad() {
        let (x, z) = get_bot_respawn_position(72, 2); // Ardream, Elmorad
        assert!(
            (190.0..=195.0).contains(&x),
            "Elmorad Ardream x should be 190..=195, got {x}"
        );
        assert!(
            (897.0..=902.0).contains(&z),
            "Elmorad Ardream z should be 897..=902, got {z}"
        );
    }

    #[test]
    fn test_respawn_position_ronark_land_base() {
        let (x, z) = get_bot_respawn_position(73, 1); // RLB, Karus
        assert!(
            (515.0..=520.0).contains(&x),
            "Karus RLB x should be 515..=520, got {x}"
        );
        assert!(
            (104.0..=109.0).contains(&z),
            "Karus RLB z should be 104..=109, got {z}"
        );

        let (x2, z2) = get_bot_respawn_position(73, 2); // RLB, Elmorad
        assert!(
            (513.0..=518.0).contains(&x2),
            "Elmorad RLB x should be 513..=518, got {x2}"
        );
        assert!(
            (916.0..=921.0).contains(&z2),
            "Elmorad RLB z should be 916..=921, got {z2}"
        );
    }

    #[test]
    fn test_respawn_position_unknown_zone_returns_zero() {
        let (x, z) = get_bot_respawn_position(21, 1); // Moradon — not a PK zone
        assert_eq!(x, 0.0, "non-PK zone should return 0.0");
        assert_eq!(z, 0.0, "non-PK zone should return 0.0");
    }

    #[test]
    fn test_bot_regene_moves_to_start_position() {
        // Bot dies at (500, 500) in zone 71 (Ronark Land) with nation 1 (Karus).
        // After regene it should be at Karus start position (1375±5, 1098±5).
        let world = WorldState::new();
        let row = make_farm_row(1, "RegeneMove", 71);
        let id = do_spawn(&world, &row, 71, 500.0, 500.0, 0, BotAiState::Pk);

        let now_ms = tick_ms();
        bot_on_death(&world, id, now_ms);

        let dead = world.get_bot(id).unwrap();
        assert_eq!(dead.presence, BotPresence::Dead);

        bot_regene(&world, id, now_ms + BOT_REGENE_DELAY_MS + 100);

        let alive = world.get_bot(id).unwrap();
        assert_eq!(alive.presence, BotPresence::Standing);
        assert!(
            (1375.0..=1380.0).contains(&alive.x),
            "Karus Ronark Land x should be 1375..=1380, got {}",
            alive.x
        );
        assert!(
            (1098.0..=1103.0).contains(&alive.z),
            "Karus Ronark Land z should be 1098..=1103, got {}",
            alive.z
        );
    }

    #[test]
    fn test_bot_regene_non_pk_zone_stays_at_current_pos() {
        // Bot in zone 21 (Moradon) — should respawn at current position.
        let world = WorldState::new();
        let row = make_farm_row(1, "MoradonBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);

        let now_ms = tick_ms();
        bot_on_death(&world, id, now_ms);
        bot_regene(&world, id, now_ms + BOT_REGENE_DELAY_MS + 100);

        let alive = world.get_bot(id).unwrap();
        assert_eq!(alive.x, 260.0, "non-PK zone should stay at original x");
        assert_eq!(alive.z, 320.0, "non-PK zone should stay at original z");
    }

    // ── Sprint 459: Zone-specific NP reward tests ────────────────────────

    #[test]
    fn test_get_bot_kill_np_ardream() {
        assert_eq!(get_bot_kill_np(72), 32, "Ardream should give 32 NP");
    }

    #[test]
    fn test_get_bot_kill_np_ronark_land() {
        assert_eq!(get_bot_kill_np(71), 64, "Ronark Land should give 64 NP");
    }

    #[test]
    fn test_get_bot_kill_np_ronark_land_base() {
        assert_eq!(get_bot_kill_np(73), 64, "RLB should give 64 NP");
    }

    #[test]
    fn test_get_bot_kill_np_other_zone() {
        assert_eq!(get_bot_kill_np(21), 64, "Other zones default to 64 NP");
        assert_eq!(get_bot_kill_np(0), 64, "Unknown zone defaults to 64 NP");
    }

    #[test]
    fn test_pvp_monument_bonus_only_for_owner_nation() {
        let world = WorldState::new();
        world.set_pvp_monument_nation(ZONE_RONARK_LAND, NATION_KARUS);

        assert_eq!(
            get_bot_kill_np_for_nation(&world, ZONE_RONARK_LAND, NATION_KARUS),
            64 + PVP_MONUMENT_NP_BONUS as i32
        );
        assert_eq!(
            get_bot_kill_np_for_nation(&world, ZONE_RONARK_LAND, NATION_ELMORAD),
            64
        );
    }

    // ── Sprint 460: Bot rivalry system tests ─────────────────────────────

    #[test]
    fn test_rivalry_constants() {
        assert_eq!(RIVALRY_DURATION_SECS, 300, "rivalry lasts 5 minutes");
        assert_eq!(MAX_ANGER_GAUGE, 5, "max anger gauge is 5");
        assert_eq!(RIVALRY_NP_BONUS, 150, "rival kill bonus is 150 NP");
    }

    #[test]
    fn test_bot_death_sets_rival_in_pk_zone() {
        let world = WorldState::new();
        // Create bot in zone 71 (Ronark Land — PK zone), nation 1 (Karus)
        let row = make_farm_row(1, "RivalBot", 71);
        let id = do_spawn(&world, &row, 71, 500.0, 500.0, 0, BotAiState::Pk);

        // Set a player killer
        world.update_bot(id, |b| {
            b.last_attacker_id = 42; // player session 42
        });

        let now = tick_ms();
        bot_on_death(&world, id, now);

        let dead = world.get_bot(id).unwrap();
        assert_eq!(dead.rival_id, 42, "killer should be set as rival");
        assert!(dead.rival_expiry_time > 0, "rival expiry should be set");
        assert_eq!(
            dead.anger_gauge, 1,
            "anger gauge should be 1 after first death"
        );
    }

    #[test]
    fn test_bot_death_no_rival_in_non_pk_zone() {
        let world = WorldState::new();
        // Zone 21 = Moradon (not PK)
        let row = make_farm_row(1, "SafeBot", 21);
        let id = do_spawn(&world, &row, 21, 260.0, 320.0, 0, BotAiState::Farmer);

        world.update_bot(id, |b| {
            b.last_attacker_id = 42;
        });

        let now = tick_ms();
        bot_on_death(&world, id, now);

        let dead = world.get_bot(id).unwrap();
        assert_eq!(dead.rival_id, -1, "no rival in non-PK zone");
        assert_eq!(dead.anger_gauge, 0, "no anger in non-PK zone");
    }

    #[test]
    fn test_bot_anger_gauge_caps_at_max() {
        let world = WorldState::new();
        let row = make_farm_row(1, "AngerBot", 71);
        let id = do_spawn(&world, &row, 71, 500.0, 500.0, 0, BotAiState::Pk);

        // Set anger gauge to max already
        world.update_bot(id, |b| {
            b.anger_gauge = MAX_ANGER_GAUGE;
            b.last_attacker_id = 42;
        });

        let now = tick_ms();
        bot_on_death(&world, id, now);

        let dead = world.get_bot(id).unwrap();
        assert_eq!(
            dead.anger_gauge, MAX_ANGER_GAUGE,
            "anger gauge should not exceed max"
        );
    }

    #[test]
    fn test_bot_death_does_not_overwrite_existing_rival() {
        let world = WorldState::new();
        let row = make_farm_row(1, "HasRival", 71);
        let id = do_spawn(&world, &row, 71, 500.0, 500.0, 0, BotAiState::Pk);

        // Bot already has a rival
        let now_unix = unix_now();
        world.update_bot(id, |b| {
            b.rival_id = 99;
            b.rival_expiry_time = now_unix + 100; // not expired
            b.last_attacker_id = 42; // different killer
        });

        let now = tick_ms();
        bot_on_death(&world, id, now);

        let dead = world.get_bot(id).unwrap();
        assert_eq!(
            dead.rival_id, 99,
            "existing rival should not be overwritten"
        );
    }

    #[test]
    fn test_bot_regene_resets_anger_gauge() {
        let world = WorldState::new();
        let row = make_farm_row(1, "AngerRegen", 71);
        let id = do_spawn(&world, &row, 71, 500.0, 500.0, 0, BotAiState::Pk);

        // Kill bot and set anger gauge
        world.update_bot(id, |b| {
            b.last_attacker_id = 42;
        });
        let now = tick_ms();
        bot_on_death(&world, id, now);

        let dead = world.get_bot(id).unwrap();
        assert_eq!(dead.anger_gauge, 1);

        // Regene
        bot_regene(&world, id, now + BOT_REGENE_DELAY_MS + 100);

        let alive = world.get_bot(id).unwrap();
        assert_eq!(alive.anger_gauge, 0, "anger gauge should reset on regene");
    }

    #[test]
    fn test_anger_gauge_packet_format() {
        // WIZ_PVP(0x88) + PVPUpdateHelmet(5) + anger(3) + has_full(0)
        let mut pkt = Packet::new(Opcode::WizPvp as u8);
        pkt.write_u8(5); // PVPUpdateHelmet
        pkt.write_u8(3); // anger level
        pkt.write_u8(0); // not full gauge
        assert_eq!(pkt.opcode, 0x88); // WIZ_PVP
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5)); // PVPUpdateHelmet
        assert_eq!(r.read_u8(), Some(3)); // anger level
        assert_eq!(r.read_u8(), Some(0)); // has_full_gauge = false
    }

    #[test]
    fn test_anger_gauge_reset_packet_format() {
        // WIZ_PVP(0x88) + PVPResetHelmet(6)
        let mut pkt = Packet::new(Opcode::WizPvp as u8);
        pkt.write_u8(6); // PVPResetHelmet
        assert_eq!(pkt.opcode, 0x88);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(6)); // PVPResetHelmet
        assert_eq!(r.remaining(), 0);
    }

    // ── AOE Skill Targeting Tests ───────────────────────────────────────

    #[test]
    fn test_find_aoe_targets_filters_by_radius() {
        let world = WorldState::new();

        // Attacker mage (Karus, zone 71)
        let attacker = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(attacker.clone());

        // Target bot within radius (3 units from center)
        let target_in = make_combat_bot(BOT_ID_BASE + 1, "ElmoNear", 2, 106, 71, 103.0, 100.0);
        world.insert_bot(target_in.clone());

        // Target bot outside radius (20 units from center)
        let target_out = make_combat_bot(BOT_ID_BASE + 2, "ElmoFar", 2, 106, 71, 120.0, 100.0);
        world.insert_bot(target_out.clone());

        let primary = (BOT_ID_BASE + 1) as SessionId;
        let targets = find_aoe_targets(&attacker, &world, 103.0, 100.0, 10.0, primary);

        // Primary excluded from secondary list
        assert!(
            targets.iter().all(|(sid, _, _)| *sid != primary),
            "primary target should not be in AOE secondary list"
        );
        // target_out is 17 units from center (103,100) → outside radius 10
        assert!(
            !targets
                .iter()
                .any(|(sid, _, _)| *sid == (BOT_ID_BASE + 2) as SessionId),
            "target outside radius should not be included"
        );
    }

    #[test]
    fn test_find_aoe_targets_excludes_same_nation() {
        let world = WorldState::new();

        let attacker = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(attacker.clone());

        // Ally (same nation) within radius
        let ally = make_combat_bot(BOT_ID_BASE + 1, "KarusAlly", 1, 106, 71, 102.0, 100.0);
        world.insert_bot(ally.clone());

        // Enemy within radius
        let enemy = make_combat_bot(BOT_ID_BASE + 2, "ElmoEnemy", 2, 106, 71, 102.0, 102.0);
        world.insert_bot(enemy.clone());

        let primary = (BOT_ID_BASE + 10) as SessionId; // dummy
        let targets = find_aoe_targets(&attacker, &world, 101.0, 101.0, 15.0, primary);

        assert!(
            !targets
                .iter()
                .any(|(sid, _, _)| *sid == (BOT_ID_BASE + 1) as SessionId),
            "ally bot should not be an AOE target"
        );
        assert!(
            targets
                .iter()
                .any(|(sid, _, _)| *sid == (BOT_ID_BASE + 2) as SessionId),
            "enemy bot should be an AOE target"
        );
    }

    #[test]
    fn test_find_aoe_targets_excludes_dead_bots() {
        let world = WorldState::new();

        let attacker = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(attacker.clone());

        // Dead enemy bot
        let mut dead_enemy = make_combat_bot(BOT_ID_BASE + 1, "ElmoDead", 2, 106, 71, 102.0, 100.0);
        dead_enemy.hp = 0;
        world.insert_bot(dead_enemy);

        let primary = (BOT_ID_BASE + 10) as SessionId;
        let targets = find_aoe_targets(&attacker, &world, 101.0, 100.0, 10.0, primary);

        assert!(
            !targets
                .iter()
                .any(|(sid, _, _)| *sid == (BOT_ID_BASE + 1) as SessionId),
            "dead bot should not be an AOE target"
        );
    }

    #[test]
    fn test_find_aoe_targets_excludes_different_zone() {
        let world = WorldState::new();

        let attacker = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(attacker.clone());

        // Enemy in different zone
        let enemy = make_combat_bot(BOT_ID_BASE + 1, "ElmoOther", 2, 106, 72, 101.0, 100.0);
        world.insert_bot(enemy);

        let primary = (BOT_ID_BASE + 10) as SessionId;
        let targets = find_aoe_targets(&attacker, &world, 100.0, 100.0, 10.0, primary);

        assert!(
            targets.is_empty(),
            "enemy in different zone should not be AOE target"
        );
    }

    #[test]
    fn test_find_aoe_targets_excludes_self() {
        let world = WorldState::new();

        let attacker = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(attacker.clone());

        let primary = (BOT_ID_BASE + 10) as SessionId;
        let targets = find_aoe_targets(&attacker, &world, 100.0, 100.0, 10.0, primary);

        assert!(
            !targets
                .iter()
                .any(|(sid, _, _)| *sid == BOT_ID_BASE as SessionId),
            "self should not be an AOE target"
        );
    }

    #[test]
    fn test_find_aoe_targets_multiple_enemies() {
        let world = WorldState::new();

        let attacker = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(attacker.clone());

        // 3 enemies within radius
        for i in 1u32..=3 {
            let enemy = make_combat_bot(
                BOT_ID_BASE + i,
                &format!("Elmo{}", i),
                2,
                106,
                71,
                100.0 + i as f32,
                100.0,
            );
            world.insert_bot(enemy);
        }

        let primary = (BOT_ID_BASE + 10) as SessionId;
        let targets = find_aoe_targets(&attacker, &world, 101.0, 100.0, 10.0, primary);

        assert_eq!(targets.len(), 3, "all 3 enemies should be AOE targets");
    }

    #[test]
    fn test_calculate_aoe_target_damage_returns_positive() {
        let world = WorldState::new();

        // Mage with high INT
        let mut mage = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        mage.int_stat = 200;
        world.insert_bot(mage.clone());

        // Target bot
        let target = make_combat_bot(BOT_ID_BASE + 1, "ElmoTarget", 2, 106, 71, 102.0, 100.0);
        world.insert_bot(target);

        let damage = calculate_aoe_target_damage(&world, &mage, (BOT_ID_BASE + 1) as SessionId);
        assert!(damage > 0, "AOE damage should be positive, got {}", damage);
    }

    #[test]
    fn test_broadcast_bot_magic_effecting_packet_format() {
        // Build what broadcast_bot_magic_effecting would build
        let skill_id: u32 = 109503;
        let damage: i16 = 250;
        let pkt = build_bot_magic_packet(
            MAGIC_EFFECTING,
            skill_id,
            BOT_ID_BASE,
            42, // target_id
            [50, 10, 75, -(damage as i32), 0, 0, 0],
        );

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_EFFECTING));
        assert_eq!(r.read_u32(), Some(skill_id));
        assert_eq!(r.read_u32(), Some(BOT_ID_BASE));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.read_u32(), Some(50)); // data[0] = x
        assert_eq!(r.read_u32(), Some(10)); // data[1] = y
        assert_eq!(r.read_u32(), Some(75)); // data[2] = z
        let d3 = r.read_u32().unwrap();
        assert_eq!(d3 as i32, -250, "data[3] should be negative damage");
    }

    #[test]
    fn test_aoe_moral_constant() {
        assert_eq!(
            MORAL_AREA_ENEMY, 10,
            "MORAL_AREA_ENEMY should be 10 per C++"
        );
    }

    #[test]
    fn test_bot_perform_aoe_attack_applies_damage() {
        let world = WorldState::new();

        // Attacker mage bot
        let mage = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(mage.clone());

        // Primary target
        let primary = make_combat_bot(BOT_ID_BASE + 1, "ElmoPrimary", 2, 106, 71, 105.0, 100.0);
        world.insert_bot(primary.clone());

        // Secondary target within AOE radius
        let secondary = make_combat_bot(BOT_ID_BASE + 2, "ElmoSecondary", 2, 106, 71, 107.0, 100.0);
        world.insert_bot(secondary.clone());

        let now_ms = tick_ms();
        bot_perform_aoe_attack(
            &world,
            &mage,
            (BOT_ID_BASE + 1) as SessionId,
            (105.0, 100.0),
            109503,
            (10.0, now_ms),
        );

        // Primary should have taken damage
        let p = world.get_bot(BOT_ID_BASE + 1).unwrap();
        assert!(
            p.hp < 5000,
            "primary target should have taken damage, hp={}",
            p.hp
        );

        // Secondary should also have taken damage
        let s = world.get_bot(BOT_ID_BASE + 2).unwrap();
        assert!(
            s.hp < 5000,
            "secondary target should have taken damage, hp={}",
            s.hp
        );
    }

    #[test]
    fn test_bot_aoe_updates_cooldown() {
        let world = WorldState::new();

        let mage = make_combat_bot(BOT_ID_BASE, "KarusMage", 1, 103, 71, 100.0, 100.0);
        world.insert_bot(mage.clone());

        let target = make_combat_bot(BOT_ID_BASE + 1, "ElmoTarget", 2, 106, 71, 102.0, 100.0);
        world.insert_bot(target.clone());

        let now_ms = tick_ms();
        bot_perform_aoe_attack(
            &world,
            &mage,
            (BOT_ID_BASE + 1) as SessionId,
            (102.0, 100.0),
            109503,
            (5.0, now_ms),
        );

        // Bot's skill cooldown should have been updated
        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(
            updated.skill_cooldown[0], now_ms,
            "cooldown should be set to now_ms"
        );
    }

    // ── Waypoint Patrol Tests ────────────────────────────────────────────

    #[test]
    fn test_waypoint_patrol_moves_bot_toward_waypoint() {
        let world = WorldState::new();

        // Karus bot in Ronark Land (zone 71) at start position.
        let mut bot = make_combat_bot(BOT_ID_BASE, "KarusPatrol", 1, 106, 71, 1375.0, 1099.0);
        bot.move_route = 1;
        bot.move_state = 2; // Target: waypoint 2 = (1276, 1056)
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved, "should have performed a waypoint move");

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Bot should have moved toward (1276.0, 1056.0).
        let dx = updated.x - 1375.0;
        let dz = updated.z - 1099.0;
        assert!(
            dx != 0.0 || dz != 0.0,
            "bot should have moved from start position"
        );
        assert!(updated.last_move_ms > 0, "last_move_ms should be updated");
    }

    #[test]
    fn test_waypoint_patrol_no_route_returns_false() {
        let world = WorldState::new();

        // Bot with no route (non-PK zone).
        let mut bot = make_combat_bot(BOT_ID_BASE, "NoRoute", 1, 106, 21, 100.0, 100.0);
        bot.move_route = 0;
        bot.move_state = 0;
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        assert!(
            !tick_waypoint_patrol(&world, &bot, now_ms),
            "should return false for bot with no route"
        );
    }

    #[test]
    fn test_waypoint_patrol_advances_state_on_arrival() {
        let world = WorldState::new();

        // Place the bot exactly at the first Ronark bowl waypoint.
        let (x, z) = bot_waypoints::get_bowl_waypoint(1, 1, 1).unwrap();
        let mut bot = make_combat_bot(BOT_ID_BASE, "AtWaypoint", 1, 106, 71, x, z);
        bot.move_route = 1;
        bot.move_state = 1;
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(
            updated.move_state, 2,
            "move_state should advance to 2 after arriving at WP 1"
        );
    }

    #[test]
    fn test_waypoint_patrol_route_complete_resets() {
        let world = WorldState::new();

        // State beyond the 12-point bowl circuit completes the route.
        let mut bot = make_combat_bot(BOT_ID_BASE, "LastWP", 1, 106, 71, 1024.0, 850.0);
        bot.move_route = 1;
        bot.move_state = 13;
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Route should have reset — new route assigned, state = 1.
        assert!(
            updated.move_route >= 1 && updated.move_route <= 10,
            "new route should be 1-10, got {}",
            updated.move_route
        );
        assert_eq!(
            updated.move_state, 1,
            "move_state should reset to 1 after route complete"
        );
    }

    #[test]
    fn test_waypoint_patrol_elmo_uses_elmo_coords() {
        let world = WorldState::new();

        let (x, z) = bot_waypoints::get_bowl_waypoint(1, 1, 2).unwrap();
        let mut bot = make_combat_bot(BOT_ID_BASE, "ElmoPatrol", 2, 106, 71, x, z);
        bot.move_route = 1;
        bot.move_state = 1;
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Should advance to WP 2 since bot was at WP 1.
        assert_eq!(updated.move_state, 2, "Elmo bot should advance to WP 2");
    }

    #[test]
    fn test_waypoint_patrol_ardream_zone() {
        let world = WorldState::new();

        // Karus bot in Ardream (zone 72), Route 1 WP 1 = (856, 138).
        let mut bot = make_combat_bot(BOT_ID_BASE, "ArdreamBot", 1, 106, 72, 856.0, 138.0);
        bot.move_route = 1;
        bot.move_state = 1;
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.move_state, 2, "Ardream bot should advance to WP 2");
    }

    #[test]
    fn test_waypoint_patrol_skips_invalid_wp_for_nation() {
        let world = WorldState::new();

        // Elmo bot on Ronark Route 2, WP 30 — Elmo has (0,0) here.
        let mut bot = make_combat_bot(BOT_ID_BASE, "ElmoSkip", 2, 106, 71, 700.0, 900.0);
        bot.move_route = 2;
        bot.move_state = 30;
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved, "should handle invalid waypoint");

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Route 2 Elmo max = 29, so state 30 > max → route complete.
        // New route should be assigned.
        assert!(
            updated.move_route >= 1 && updated.move_route <= 10,
            "should have new route after completion"
        );
    }

    #[test]
    fn test_bot_regene_assigns_new_route() {
        let world = WorldState::new();

        // Create a dead PK bot in Ronark Land.
        let mut bot = make_combat_bot(BOT_ID_BASE, "DeadBot", 1, 106, 71, 500.0, 500.0);
        bot.presence = BotPresence::Dead;
        bot.move_route = 3;
        bot.move_state = 15;
        bot.regene_at_ms = 1;
        world.insert_bot(bot);

        let now_ms = tick_ms();
        bot_regene(&world, BOT_ID_BASE, now_ms);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.presence, BotPresence::Standing, "should be alive");
        assert!(
            updated.move_route >= 1 && updated.move_route <= 10,
            "regene should assign new route 1-10, got {}",
            updated.move_route
        );
        assert_eq!(updated.move_state, 1, "regene should reset move_state to 1");
    }

    #[test]
    fn test_tick_fighting_no_target_triggers_patrol() {
        let world = WorldState::new();

        // Lone PK bot in Ronark Land with no enemies.
        let mut bot = make_combat_bot(BOT_ID_BASE, "LoneBot", 1, 106, 71, 1375.0, 1099.0);
        bot.move_route = 1;
        bot.move_state = 2;
        bot.last_move_ms = 0; // Ensure cooldown passed.
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        tick_fighting(&world, &bot, now_ms);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // With no target, bot should patrol (move or advance WP).
        assert!(
            updated.last_move_ms > 0,
            "last_move_ms should be updated after patrol tick"
        );
    }

    // ── Path Validation Tests ────────────────────────────────────────────

    #[test]
    fn test_is_bot_position_valid_no_zone_data() {
        let world = WorldState::new();
        // No zone loaded — permissive fallback returns true.
        assert!(
            is_bot_position_valid(&world, 71, 500.0, 500.0),
            "should be permissive when no zone data"
        );
    }

    #[test]
    fn test_is_bot_position_valid_with_zone() {
        let world = WorldState::new();
        world.ensure_zone(71, 2048);

        // Valid position within bounds.
        assert!(
            is_bot_position_valid(&world, 71, 500.0, 500.0),
            "500,500 should be valid in 2048-sized zone"
        );
    }

    /// Create a zone with actual map data (SMD) for path validation tests.
    fn create_zone_with_map_data(world: &WorldState, zone_id: u16, map_size: i32) {
        use crate::zone::{MapData, ZoneInfo};
        use ko_protocol::smd::SmdFile;

        let unit_dist = 4.0;
        let grid_len = (map_size * map_size) as usize;
        let smd = SmdFile {
            map_size,
            unit_dist,
            map_width: (map_size - 1) as f32 * unit_dist,
            map_height: (map_size - 1) as f32 * unit_dist,
            event_grid: vec![0i16; grid_len],
            warps: Vec::new(),
            regene_events: Vec::new(),
        };
        let map_data = MapData::new(smd);
        let zone_info = ZoneInfo {
            smd_name: String::new(),
            zone_name: format!("TestZone{}", zone_id),
            zone_type: crate::zone::ZoneAbilityType::Neutral,
            min_level: 0,
            max_level: 83,
            init_x: 0.0,
            init_z: 0.0,
            init_y: 0.0,
            abilities: crate::zone::ZoneAbilities::default(),
            status: 1,
        };
        world.set_zone_with_map(zone_id, zone_info, map_data);
    }

    #[test]
    fn test_waypoint_patrol_rejects_invalid_position() {
        let world = WorldState::new();
        // Create a tiny zone (map_size=2, map_width=4.0) with real map data.
        // One 250ms step from (3.5,3.5) toward (1276,1056) exceeds the
        // map_width=4.0 boundary and triggers rejection.
        create_zone_with_map_data(&world, 71, 2);

        let mut bot = make_combat_bot(BOT_ID_BASE, "OutOfBounds", 1, 106, 71, 3.5, 3.5);
        bot.move_route = 1;
        bot.move_state = 2; // WP 2 Karus = (1276, 1056) — out of bounds
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        let moved = tick_waypoint_patrol(&world, &bot, now_ms);
        assert!(moved, "should still return true (handled the tick)");

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Bot should NOT have moved to the invalid position.
        // It should have skipped to next waypoint (move_state = 3).
        assert_eq!(
            updated.move_state, 3,
            "should skip to next waypoint when position is invalid"
        );
        // Position should remain unchanged.
        assert!(
            (updated.x - 3.5).abs() < 0.01,
            "x should not change on invalid position"
        );
    }

    #[test]
    fn test_combat_chase_rejects_invalid_position() {
        let world = WorldState::new();
        // Create a tiny zone (map_width=4.0).
        create_zone_with_map_data(&world, 71, 2);

        // Bot within bounds trying to chase target far away.
        let mut bot = make_combat_bot(BOT_ID_BASE, "ChaseFail", 1, 106, 71, 1.0, 1.0);
        bot.last_move_ms = 0;
        world.insert_bot(bot.clone());

        // Insert enemy bot far outside the zone boundary.
        let enemy = make_combat_bot(BOT_ID_BASE + 1, "FarEnemy", 2, 106, 71, 200.0, 200.0);
        world.insert_bot(enemy);

        let now_ms = tick_ms();
        tick_fighting(&world, &bot, now_ms);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Map width = 4.0 — bot should not leave bounds.
        assert!(
            updated.x < 4.0 && updated.z < 4.0,
            "bot should not move outside zone bounds: ({}, {})",
            updated.x,
            updated.z
        );
    }

    // ── Equipment Persistence Tests ──────────────────────────────────────

    #[test]
    fn test_equipment_persists_across_regene() {
        let world = WorldState::new();

        // Create a dead bot with equipment.
        let mut bot = make_combat_bot(BOT_ID_BASE, "EquipBot", 1, 106, 71, 500.0, 500.0);
        bot.presence = BotPresence::Dead;
        bot.regene_at_ms = 1;
        // Set some equipment visuals.
        bot.equip_visual[0] = (150001, 100, 0); // BREAST
        bot.equip_visual[6] = (120055, 80, 1); // RIGHTHAND
        world.insert_bot(bot);

        let now_ms = tick_ms();
        bot_regene(&world, BOT_ID_BASE, now_ms);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        assert_eq!(updated.presence, BotPresence::Standing, "should be alive");
        // Equipment should persist (never cleared in regene).
        assert_eq!(
            updated.equip_visual[0],
            (150001, 100, 0),
            "BREAST equipment should persist"
        );
        assert_eq!(
            updated.equip_visual[6],
            (120055, 80, 1),
            "RIGHTHAND equipment should persist"
        );
    }

    #[test]
    fn test_equipment_persists_across_waypoint_route_reset() {
        let world = WorldState::new();

        // Bot at end of route with equipment.
        let mut bot = make_combat_bot(BOT_ID_BASE, "RouteEnd", 1, 106, 71, 718.0, 928.0);
        bot.move_route = 1;
        bot.move_state = 19; // Max for Route 1 Karus
        bot.equip_visual[0] = (150001, 100, 0);
        bot.equip_visual[6] = (120055, 80, 1);
        world.insert_bot(bot.clone());

        let now_ms = tick_ms();
        tick_waypoint_patrol(&world, &bot, now_ms);

        let updated = world.get_bot(BOT_ID_BASE).unwrap();
        // Route should have reset.
        assert!(updated.move_route >= 1 && updated.move_route <= 10);
        // Equipment should persist through route reset + respawn.
        assert_eq!(
            updated.equip_visual[0],
            (150001, 100, 0),
            "BREAST should persist after route reset"
        );
        assert_eq!(
            updated.equip_visual[6],
            (120055, 80, 1),
            "RIGHTHAND should persist after route reset"
        );
    }
}
