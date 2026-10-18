//! Server-wide monster behaviour rules on top of the NPC table:
//! - monsters fight only when they are attacked (the table's act_type 1); `+aggro level <n>` can make monsters
//!   of level >= n attack passers-by again (0 = off, the default),
//! - a monster that is hit/engaged calls the monsters (any kind) within `assist_range` metres,
//! - GM characters are ignored by monsters unless `gm_aggro` is on (so GM test characters can walk),
//! - a few template fixes (Juraid's Deva Bird has no search range, the Manes dragon HP).
//!
//! Values live in `monster_behavior.cfg` next to the server binary and are changed in game with `+aggro`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::world::types::{
    ZONE_ARDREAM, ZONE_MORADON, ZONE_MORADON2, ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
    ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE,
};
use crate::npc::NpcTemplate;

const FILE: &str = "monster_behavior.cfg";

pub const DEVA_BIRD_SID: u16 = 8106;
pub const MANES_DRAGON_SID: u16 = 10733;
/// The Manes dragon has this many times its table HP.
pub const MANES_DRAGON_HP_FACTOR: u32 = 2;

/// 0 = off: monsters only fight when they are attacked (the table's own aggressive monsters excepted).
static AGGRO_MIN_LEVEL: AtomicU32 = AtomicU32::new(0);
static ASSIST_RANGE_BITS: AtomicU32 = AtomicU32::new(0x3F80_0000); // 1.0
static GM_AGGRO: AtomicBool = AtomicBool::new(false);

pub fn aggro_min_level() -> u32 {
    AGGRO_MIN_LEVEL.load(Ordering::Relaxed)
}

pub fn assist_range() -> f32 {
    f32::from_bits(ASSIST_RANGE_BITS.load(Ordering::Relaxed))
}

pub fn gm_aggro() -> bool {
    GM_AGGRO.load(Ordering::Relaxed)
}

pub fn set_aggro_min_level(level: u32) {
    AGGRO_MIN_LEVEL.store(level, Ordering::Relaxed);
    save();
}

pub fn set_assist_range(metres: f32) {
    ASSIST_RANGE_BITS.store(metres.clamp(0.0, 100.0).to_bits(), Ordering::Relaxed);
    save();
}

pub fn set_gm_aggro(on: bool) {
    GM_AGGRO.store(on, Ordering::Relaxed);
    save();
}

/// Read `monster_behavior.cfg` (missing file = defaults).
pub fn load() {
    let Ok(text) = std::fs::read_to_string(FILE) else {
        return;
    };
    // files without "version=2" come from the first release (level 40 aggression): that part is not wanted any more
    let current = text.lines().any(|l| l.trim() == "version=2");
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "aggro_level" => {
                if !current {
                    continue;
                }
                if let Ok(v) = value.parse::<u32>() {
                    AGGRO_MIN_LEVEL.store(v, Ordering::Relaxed);
                }
            }
            "assist_range" => {
                if let Ok(v) = value.parse::<f32>() {
                    ASSIST_RANGE_BITS.store(v.clamp(0.0, 100.0).to_bits(), Ordering::Relaxed);
                }
            }
            "gm_aggro" => GM_AGGRO.store(value == "1", Ordering::Relaxed),
            _ => {}
        }
    }
}

fn save() {
    let text = format!(
        "version=2\naggro_level={}\nassist_range={}\ngm_aggro={}\n",
        aggro_min_level(),
        assist_range(),
        u8::from(gm_aggro())
    );
    if let Err(e) = std::fs::write(FILE, text) {
        tracing::warn!("monster behaviour settings not saved: {e}");
    }
}

/// Fixes applied to a freshly loaded template.
pub fn adjust_template(t: &mut NpcTemplate) {
    if !t.is_monster {
        return;
    }
    match t.s_sid {
        // the Juraid final boss has search/tracing range 0: it never got an AI and stood still
        DEVA_BIRD_SID => {
            t.search_range = t.search_range.max(60);
            t.tracing_range = t.tracing_range.max(120);
            t.act_type = 5; // aggressive
        }
        MANES_DRAGON_SID => t.max_hp = t.max_hp.saturating_mul(MANES_DRAGON_HP_FACTOR),
        _ => {}
    }
}

/// `(is_aggressive, attack_type, has_friends)` for a monster AI state.
pub fn ai_flags(tmpl: &NpcTemplate, zone_id: u16) -> (bool, u8, bool) {
    let is_moradon = matches!(
        zone_id,
        ZONE_MORADON | ZONE_MORADON2 | ZONE_MORADON3 | ZONE_MORADON4 | ZONE_MORADON5
    );
    let table_aggressive = !matches!(tmpl.act_type, 1..=4);
    let min = aggro_min_level();
    // `+aggro level` is a global "make everyone aggressive" override; it must not reach into
    // Moradon, which is meant to fight alone and only when attacked regardless of that setting
    // (a template-flagged aggressive monster there is untouched -- this only blocks the override).
    let min_level_aggro = min > 0 && tmpl.is_monster && tmpl.level as u32 >= min && !is_moradon;
    let aggressive = table_aggressive || min_level_aggro;
    let pk_zone = matches!(
        zone_id,
        ZONE_RONARK_LAND | ZONE_ARDREAM | ZONE_RONARK_LAND_BASE
    );
    let has_friends = matches!(tmpl.act_type, 3 | 4) || tmpl.is_monster;
    (aggressive, u8::from(aggressive), has_friends && !pk_zone)
}
