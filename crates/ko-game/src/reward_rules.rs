//! Event rewards that used to be hard-coded (Ultima, Fire & Ice, Manes Survival ranking), editable in the web panel
//! ("Ödül Merkezi" > `event_reward_rule`). The rows are cached in `WorldState` and reloaded by the admin command
//! queue. When the table has no row for an event the built-in defaults (the old hard-coded rewards) are used, so a
//! server that runs before the panel created the table behaves exactly as before.

use ko_db::DbPool;
use sqlx::Row;

use crate::world::WorldState;

#[derive(Debug, Clone)]
pub struct RewardRule {
    pub event_key: String,
    /// which situation the row is for (see the defaults below)
    pub cond: String,
    pub rank_from: i32,
    pub rank_to: i32,
    /// 0 = everybody of the group, N = N players picked at random
    pub pick: i32,
    /// percent, 100 = always
    pub chance: i32,
    pub item_id: u32,
    pub item_count: u16,
    pub days: i32,
}

fn rule(key: &str, cond: &str, pick: i32, chance: i32, item: u32, count: u16, days: i32) -> RewardRule {
    RewardRule { event_key: key.into(), cond: cond.into(), rank_from: 0, rank_to: 0, pick, chance, item_id: item, item_count: count, days }
}

fn rank_rule(from: i32, to: i32, item: u32, count: u16) -> RewardRule {
    RewardRule { event_key: "manes".into(), cond: "rank".into(), rank_from: from, rank_to: to, pick: 0, chance: 100, item_id: item, item_count: count, days: 0 }
}

/// The rewards the server gave before they were editable.
pub fn defaults(key: &str) -> Vec<RewardRule> {
    match key {
        "ultima" => vec![
            rule("ultima", "survive", 0, 100, 389_161_000, 1, 0),
            rule("ultima", "survive", 0, 100, 389_160_000, 1, 0),
            rule("ultima", "stage1", 0, 100, 800_051_000, 1, 0),
            rule("ultima", "stage2", 0, 100, 389_160_000, 1, 0),
            rule("ultima", "stage3", 0, 100, 389_161_000, 1, 0),
            // boss box: chance = highest percent, the real chance is damage share x 3 (at least 1)
            rule("ultima", "box", 0, 60, 810_432_000, 1, 30),
        ],
        "golem" => vec![
            rule("golem", "win", 0, 100, 811_182_000, 3, 0),
            rule("golem", "win", 0, 100, 1_900_004_000, 1, 1),
            rule("golem", "win", 2, 100, 811_065_000, 22, 0),
            rule("golem", "win", 1, 100, 931_734_000, 1, 30),
            rule("golem", "lose", 0, 100, 811_182_000, 1, 0),
            rule("golem", "lose", 0, 100, 1_900_004_000, 1, 1),
            rule("golem", "lose", 1, 100, 811_065_000, 22, 0),
            rule("golem", "draw", 0, 100, 811_182_000, 3, 0),
            rule("golem", "draw", 0, 100, 1_900_004_000, 1, 1),
            rule("golem", "draw", 3, 100, 811_065_000, 22, 0),
            rule("golem", "draw", 1, 100, 931_734_000, 1, 30),
            rule("golem", "belt", 1, 5, 340_710_000, 1, 0),
        ],
        "manes" => vec![
            rank_rule(1, 1, crate::systems::manes_survival::MANES_ORB_ITEM_ID, 150),
            rank_rule(2, 2, crate::systems::manes_survival::MANES_ORB_ITEM_ID, 100),
            rank_rule(3, 3, crate::systems::manes_survival::MANES_ORB_ITEM_ID, 50),
            rank_rule(4, 0, crate::systems::manes_survival::MANES_ORB_ITEM_ID, 20),
        ],
        _ => Vec::new(),
    }
}

pub async fn load(pool: &DbPool) -> Result<Vec<RewardRule>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT event_key, cond, rank_from, rank_to, pick, chance, item_id, item_count, days FROM event_reward_rule WHERE enabled ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| RewardRule {
            event_key: r.try_get("event_key").unwrap_or_default(),
            cond: r.try_get("cond").unwrap_or_default(),
            rank_from: r.try_get("rank_from").unwrap_or(0),
            rank_to: r.try_get("rank_to").unwrap_or(0),
            pick: r.try_get("pick").unwrap_or(0),
            chance: r.try_get("chance").unwrap_or(100),
            item_id: r.try_get::<i32, _>("item_id").unwrap_or(0).max(0) as u32,
            item_count: r.try_get::<i32, _>("item_count").unwrap_or(1).clamp(1, u16::MAX as i32) as u16,
            days: r.try_get("days").unwrap_or(0),
        })
        .collect())
}

impl WorldState {
    /// The rules of one event; the old built-in rewards when the table holds none for it.
    pub fn reward_rules_for(&self, key: &str) -> Vec<RewardRule> {
        let v: Vec<RewardRule> = self.reward_rules.read().iter().filter(|r| r.event_key == key).cloned().collect();
        if v.is_empty() {
            defaults(key)
        } else {
            v
        }
    }

    pub async fn reload_reward_rules(&self, pool: &DbPool) -> Result<usize, sqlx::Error> {
        let rows = load(pool).await?;
        let n = rows.len();
        *self.reward_rules.write() = rows;
        Ok(n)
    }

    /// Give one rule's item (with its duration); `false` when the bag could not take it.
    pub fn give_rule_item(&self, sid: crate::zone::SessionId, r: &RewardRule) -> bool {
        if r.item_id == 0 {
            return true;
        }
        if r.days > 0 {
            self.give_item_with_expiry(sid, r.item_id, r.item_count, r.days as u32)
        } else {
            self.give_item(sid, r.item_id, r.item_count)
        }
    }
}
