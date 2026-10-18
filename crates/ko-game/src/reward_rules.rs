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
            rule("golem", "win", 0, 100, 1_900_004_106, 1, 1),
            rule("golem", "win", 2, 100, 811_065_000, 22, 0),
            rule("golem", "win", 1, 100, 931_734_000, 1, 30),
            rule("golem", "lose", 0, 100, 811_182_000, 1, 0),
            rule("golem", "lose", 0, 100, 1_900_004_106, 1, 1),
            rule("golem", "lose", 1, 100, 811_065_000, 22, 0),
            rule("golem", "draw", 0, 100, 811_182_000, 3, 0),
            rule("golem", "draw", 0, 100, 1_900_004_106, 1, 1),
            rule("golem", "draw", 3, 100, 811_065_000, 22, 0),
            rule("golem", "draw", 1, 100, 931_734_000, 1, 30),
            rule("golem", "belt", 1, 5, 340_710_124, 1, 0),
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

    /// [`give_rule_item`](Self::give_rule_item), but an item the bag cannot take is mailed to the
    /// player instead of being lost.
    pub fn give_rule_item_or_mail(
        &self,
        sid: crate::zone::SessionId,
        r: &RewardRule,
        sender: &str,
    ) -> Delivery {
        self.give_item_or_mail(sid, r.item_id, r.item_count, r.days.max(0) as u32, sender)
    }

    /// Put an event reward into the bag. When it does not fit (full bag, overweight is not checked
    /// by `give_item`) the item goes into a system letter, which keeps the expiry days, and the
    /// player is told. `Delivery::Failed` means neither worked (unknown item, no database, no
    /// async runtime); the reason is logged.
    pub fn give_item_or_mail(
        &self,
        sid: crate::zone::SessionId,
        item_id: u32,
        count: u16,
        days: u32,
        sender: &str,
    ) -> Delivery {
        if item_id == 0 {
            return Delivery::Bag;
        }
        let in_bag = if days > 0 {
            self.give_item_with_expiry(sid, item_id, count, days)
        } else {
            self.give_item(sid, item_id, count)
        };
        if in_bag {
            return Delivery::Bag;
        }
        let Some(item) = self.get_item(item_id) else {
            tracing::warn!(sid, item_id, sender, "event reward skipped: item does not exist");
            return Delivery::Failed;
        };
        let name = item.str_name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| format!("#{item_id}"));
        let duration = item.duration.unwrap_or(0).max(0) as u16;
        let recipient = match self.with_session(sid, |h| h.character.as_ref().map(|c| c.name.clone())) {
            Some(Some(n)) if !n.is_empty() => n,
            _ => return Delivery::Failed,
        };
        let (Some(pool), Ok(handle)) = (self.db_pool().cloned(), tokio::runtime::Handle::try_current()) else {
            tracing::warn!(sid, item_id, sender, "event reward could not be mailed: no database or runtime");
            return Delivery::Failed;
        };
        let sender = sender.to_string();
        handle.spawn(async move {
            match crate::handler::letter::create_system_letter_ex(
                &pool,
                &sender,
                &recipient,
                "Event reward",
                "Your bag was full. Take the reward from this letter.",
                item_id,
                count.max(1),
                duration,
                days,
            )
            .await
            {
                Ok(true) => tracing::info!(recipient = %recipient, item_id, count, "event reward mailed (bag full)"),
                Ok(false) => tracing::warn!(recipient = %recipient, item_id, "event reward letter: recipient not found"),
                Err(e) => tracing::warn!(recipient = %recipient, item_id, "event reward letter failed: {e}"),
            }
        });
        self.send_to_session_owned(
            sid,
            crate::systems::timed_notice::build_notice_packet(
                8,
                &format!("Your bag is full: {name} was sent to your mailbox."),
            ),
        );
        Delivery::Mail
    }
}

/// Where an event reward item ended up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// In the player's bag.
    Bag,
    /// The bag was full; the item was mailed.
    Mail,
    /// Nowhere (see the log).
    Failed,
}

impl Delivery {
    /// The reward reached the player one way or the other.
    pub fn delivered(self) -> bool {
        !matches!(self, Delivery::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_item_is_reported_not_silently_dropped() {
        let world = WorldState::new();
        // no such session / no such item: neither the bag nor the mail can take it
        assert_eq!(world.give_item_or_mail(1, 123_456_789, 1, 0, "Test"), Delivery::Failed);
        assert!(!Delivery::Failed.delivered());
        assert!(Delivery::Bag.delivered());
        assert!(Delivery::Mail.delivered());
    }

    #[test]
    fn item_id_zero_is_a_noop_success() {
        let world = WorldState::new();
        assert_eq!(world.give_item_or_mail(1, 0, 1, 0, "Test"), Delivery::Bag);
    }

    #[test]
    fn fire_and_ice_defaults_use_items_that_exist_in_the_item_table() {
        // 1900004000 and 340710000 are not in the item table (the real ids carry the extension
        // digits); the live event_reward_rule rows were fixed by migration 20261018170000.
        for r in defaults("golem") {
            assert_ne!(r.item_id, 1_900_004_000, "Event Emblem is item 1900004106");
            assert_ne!(r.item_id, 340_710_000, "Lunar War Belt is item 340710124");
        }
    }
}
