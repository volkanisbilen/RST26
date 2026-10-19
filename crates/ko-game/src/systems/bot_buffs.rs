//! Type-4 buff bookkeeping for runtime bots.
//!
//! Bots have no `ClientSession`, so they cannot carry the players' `ActiveBuff` map. Every
//! friendly Type-4 skill that lands on a bot -- from a priest/rogue bot's own support rotation
//! or from a real player casting on a bot party mate -- is recorded here with its expiry and
//! the two effects the bot combat model actually uses: AC (flat + percent, read by
//! `bot_ai::bot_defense_ac`) and max HP (applied to the `BotInstance` on apply, reverted on
//! expiry). Same-type stacking is refused like for players (incl. the SPEED/SPEED2 pair).

use std::collections::HashMap;
use std::sync::OnceLock;

use parking_lot::Mutex;

use crate::buff_constants::{BUFF_TYPE_SPEED, BUFF_TYPE_SPEED2};
use crate::world::{BotId, WorldState};

/// One buff on one bot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BotBuff {
    pub skill_id: u32,
    pub buff_type: i32,
    /// Tick-ms expiry (`u64::MAX` for a buff without duration).
    pub until_ms: u64,
    pub ac: i32,
    pub ac_pct: i32,
    /// Max-HP change applied to the `BotInstance` (reverted on expiry).
    pub max_hp_delta: i16,
}

static BUFFS: OnceLock<Mutex<HashMap<BotId, Vec<BotBuff>>>> = OnceLock::new();

fn map() -> &'static Mutex<HashMap<BotId, Vec<BotBuff>>> {
    BUFFS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Live buffs of `bot_id` (expired entries are filtered, not removed -- `expire` does that).
pub fn active(bot_id: BotId, now_ms: u64) -> Vec<BotBuff> {
    map()
        .lock()
        .get(&bot_id)
        .map(|v| v.iter().copied().filter(|b| b.until_ms > now_ms).collect())
        .unwrap_or_default()
}

/// Would a buff of `buff_type` be refused on `bot_id` (same type active, or the SPEED/SPEED2
/// conflict)? Mirrors `magic_process::type4_buff_already_active`.
pub fn blocks(bot_id: BotId, buff_type: i32, now_ms: u64) -> bool {
    if buff_type <= 0 {
        return false;
    }
    active(bot_id, now_ms).iter().any(|b| {
        b.buff_type == buff_type
            || (buff_type == BUFF_TYPE_SPEED && b.buff_type == BUFF_TYPE_SPEED2)
            || (buff_type == BUFF_TYPE_SPEED2 && b.buff_type == BUFF_TYPE_SPEED)
    })
}

/// Record a Type-4 buff on a bot and apply its max-HP part to the `BotInstance`.
/// Returns the buff duration in seconds, or `None` when the buff is refused (same type already
/// active) or the bot does not exist.
pub fn apply(
    world: &WorldState,
    bot_id: BotId,
    skill_id: u32,
    type4: &ko_db::models::MagicType4Row,
    now_ms: u64,
) -> Option<i32> {
    let buff_type = type4.buff_type.unwrap_or(0);
    if blocks(bot_id, buff_type, now_ms) {
        return None;
    }
    let bot = world.get_bot(bot_id)?;
    let duration = type4.duration.unwrap_or(0).max(0);
    let until_ms = if duration > 0 { now_ms + duration as u64 * 1000 } else { u64::MAX };

    // Percent HP is relative to the bot's unbuffed max HP.
    let buffed_delta: i32 = active(bot_id, now_ms).iter().map(|b| b.max_hp_delta as i32).sum();
    let base_max_hp = (bot.max_hp as i32 - buffed_delta).max(1);
    let flat_hp = type4.max_hp.unwrap_or(0);
    let pct_hp = base_max_hp * type4.max_hp_pct.unwrap_or(0) / 100;
    let max_hp_delta = (flat_hp + pct_hp).clamp(i16::MIN as i32, i16::MAX as i32) as i16;

    if max_hp_delta != 0 {
        world.update_bot(bot_id, |b| {
            let new_max = (b.max_hp as i32 + max_hp_delta as i32).clamp(1, i16::MAX as i32) as i16;
            let new_hp = (b.hp as i32 + max_hp_delta as i32).clamp(1, new_max as i32) as i16;
            b.max_hp = new_max;
            if b.hp > 0 {
                b.hp = new_hp;
            }
        });
    }

    let mut buffs = map().lock();
    let list = buffs.entry(bot_id).or_default();
    list.retain(|b| b.until_ms > now_ms);
    list.push(BotBuff {
        skill_id,
        buff_type,
        until_ms,
        ac: type4.ac.unwrap_or(0),
        ac_pct: type4.ac_pct.unwrap_or(0),
        max_hp_delta,
    });
    Some(duration)
}

/// `base_ac` with the live AC buffs of `bot_id` folded in (flat first, then percent).
pub fn ac_with_buffs(bot_id: BotId, base_ac: i32, now_ms: u64) -> i32 {
    let live = active(bot_id, now_ms);
    if live.is_empty() {
        return base_ac;
    }
    let flat: i32 = live.iter().map(|b| b.ac).sum();
    let pct: i32 = live.iter().map(|b| b.ac_pct).sum();
    let ac = base_ac + flat;
    ac + ac * pct / 100
}

fn revert(world: &WorldState, bot_id: BotId, gone: &[BotBuff]) {
    let delta: i32 = gone.iter().map(|b| b.max_hp_delta as i32).sum();
    if delta == 0 {
        return;
    }
    world.update_bot(bot_id, |b| {
        let new_max = (b.max_hp as i32 - delta).clamp(1, i16::MAX as i32) as i16;
        b.max_hp = new_max;
        if b.hp > new_max {
            b.hp = new_max;
        }
    });
}

/// Drop every expired buff and revert its max-HP part. Cheap when nothing is buffed.
pub fn expire(world: &WorldState, now_ms: u64) {
    let expired: Vec<(BotId, Vec<BotBuff>)> = {
        let mut buffs = map().lock();
        let mut out = Vec::new();
        buffs.retain(|bot_id, list| {
            let (gone, keep): (Vec<BotBuff>, Vec<BotBuff>) =
                list.iter().partition(|b| b.until_ms <= now_ms);
            if !gone.is_empty() {
                out.push((*bot_id, gone));
            }
            *list = keep;
            !list.is_empty()
        });
        out
    };
    for (bot_id, gone) in expired {
        revert(world, bot_id, &gone);
    }
}

/// Remove all buffs of a bot (death / despawn) and revert their max-HP part.
pub fn clear(world: &WorldState, bot_id: BotId) {
    let gone = map().lock().remove(&bot_id).unwrap_or_default();
    revert(world, bot_id, &gone);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type4(buff_type: i32, duration: i32, ac: i32, max_hp: i32, max_hp_pct: i32) -> ko_db::models::MagicType4Row {
        ko_db::models::MagicType4Row {
            i_num: 0,
            buff_type: Some(buff_type),
            radius: None,
            duration: Some(duration),
            attack_speed: None,
            speed: None,
            ac: Some(ac),
            ac_pct: None,
            attack: None,
            magic_attack: None,
            max_hp: Some(max_hp),
            max_hp_pct: Some(max_hp_pct),
            max_mp: None,
            max_mp_pct: None,
            str: None,
            sta: None,
            dex: None,
            intel: None,
            cha: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            disease_r: None,
            poison_r: None,
            exp_pct: None,
            special_amount: None,
            hit_rate: None,
            avoid_rate: None,
        }
    }

    fn bot_world(max_hp: i16) -> (WorldState, BotId) {
        let world = WorldState::new();
        let id = crate::systems::bot_ai::spawn_gm_bot(
            &world,
            crate::systems::bot_ai::SpawnGmBotParams {
                zone_id: 71,
                x: 100.0,
                y: 0.0,
                z: 100.0,
                class: 4,
                level: 70,
                nation: 1,
                ai_state: crate::world::BotAiState::Pk,
            },
        );
        world.update_bot(id, |b| {
            b.max_hp = max_hp;
            b.hp = max_hp;
        });
        (world, id)
    }

    #[test]
    fn hp_buff_raises_max_hp_and_reverts_on_expiry() {
        let (world, id) = bot_world(1000);
        assert_eq!(apply(&world, id, 112_615, &type4(1, 10, 0, 0, 20), 1_000), Some(10));
        let b = world.get_bot(id).unwrap();
        assert_eq!(b.max_hp, 1200);
        assert_eq!(b.hp, 1200);
        // same type again: refused
        assert_eq!(apply(&world, id, 112_615, &type4(1, 10, 0, 0, 20), 2_000), None);
        expire(&world, 5_000);
        assert_eq!(world.get_bot(id).unwrap().max_hp, 1200, "still running");
        expire(&world, 11_001);
        let b = world.get_bot(id).unwrap();
        assert_eq!(b.max_hp, 1000);
        assert_eq!(b.hp, 1000, "hp clamped to the reverted max");
        assert!(active(id, 11_001).is_empty());
    }

    #[test]
    fn ac_buff_folds_into_defense() {
        let (world, id) = bot_world(1000);
        assert!(apply(&world, id, 112_603, &type4(2, 600, 50, 0, 0), 0).is_some());
        assert_eq!(ac_with_buffs(id, 200, 1_000), 250);
        assert_eq!(ac_with_buffs(id, 200, 700_000), 200, "expired");
        clear(&world, id);
        assert!(active(id, 0).is_empty());
    }

    #[test]
    fn speed_pair_conflicts_like_players() {
        let (world, id) = bot_world(1000);
        assert!(apply(&world, id, 108_010, &type4(BUFF_TYPE_SPEED, 60, 0, 0, 0), 0).is_some());
        assert!(blocks(id, BUFF_TYPE_SPEED2, 1));
        assert!(!blocks(id, 2, 1));
        clear(&world, id);
    }
}
