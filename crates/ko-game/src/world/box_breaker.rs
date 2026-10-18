//! Box Breaker NPC back end (Lua bindings `GetBreakableBoxes`, `BoxPick`, `BoxBreak`, `BoxBreakAll`).
//!
//! The Moradon "Box Breaker" NPC (`Quests/31514_BoxBreaker.lua`) is a guaranteed alternative to
//! the Chaotic Generator window. It breaks exactly the same items with exactly the same reward
//! tables:
//!
//! * a breakable box is an item with `item.effect2 = 251` that is countable and has at least one
//!   `item_exchange` row with `random_flag IN (1,2,3)` and `origin_item_num1 = <box id>`
//!   (`WorldState::get_bifrost_exchanges`, the data the generator and `ko-admin` already use);
//! * the reward is drawn like `bifrost_piece_exchange`: every exchange row contributes
//!   `exchange_item_count1 / 5` slots of a 10000-slot array, rows with `random_flag >= 101` or
//!   without a reward are skipped, rows that need more boxes than the player owns are skipped,
//!   and one slot is picked uniformly (`build_reward_pool` + `pick_from_pool` are the same
//!   distribution without materialising the array);
//! * a pool that references a reward item missing from the item table aborts, exactly like the
//!   generator's `BifrostCheckExchange`.
//!
//! Differences on purpose: no gold is charged, boxes are consumed from the bag only (never
//! equipped slots) and stacks that are rented / sealed / duplicate-flagged are ignored (the item
//! flags are an enum, so they are compared with equality, unlike the generator's bit mask).
//!
//! Safety: every call re-validates everything from server state (the Lua side only passes an item
//! id and a count that came from a menu the server itself sent). Each box is consumed with the
//! player's own `update_inventory` lock, then the reward is given; if the reward cannot be given
//! the box is handed back, so a box is never lost or duplicated.

use super::*;

use ko_db::models::ItemExchangeRow;

/// `item.effect2` value that marks a box / piece as breakable by the Chaotic Generator.
pub const BOX_EFFECT2: i32 = 251;
/// Hard cap of boxes broken by one menu action (keeps one click bounded).
pub const MAX_BREAK_PER_CALL: u32 = 200;
/// Size of the generator's weighted array.
const POOL_CAP: u32 = 10_000;

/// A breakable box type the player owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakableBox {
    pub item_id: u32,
    /// Total in usable (not rented / sealed / duplicate-flagged) bag stacks.
    pub count: u32,
    pub name: String,
}

/// Why a break run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BreakStop {
    /// Everything that was asked for has been broken (or the player ran out of that box).
    #[default]
    Done,
    /// The per-call cap was reached while more boxes remain.
    Capped,
    /// The player owns no usable box of the requested type.
    NoBox,
    /// The item is not a breakable box.
    NotBreakable,
    /// The player is dead / trading / merchanting / mining / fishing / not in game.
    Busy,
    /// The player is not next to the NPC they are talking to.
    TooFar,
    /// There is no free bag slot for the next reward.
    BagFull,
    /// The next reward would exceed the weight limit.
    Overweight,
    /// The reward table of the box is empty or references a missing item.
    BadPool,
}

impl BreakStop {
    /// Short code handed to the Lua script.
    pub fn code(self) -> &'static str {
        match self {
            BreakStop::Done => "done",
            BreakStop::Capped => "capped",
            BreakStop::NoBox => "nobox",
            BreakStop::NotBreakable => "notbox",
            BreakStop::Busy => "busy",
            BreakStop::TooFar => "far",
            BreakStop::BagFull => "full",
            BreakStop::Overweight => "heavy",
            BreakStop::BadPool => "badpool",
        }
    }
}

/// Result of a break run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BreakOutcome {
    /// Boxes consumed (== rewards given).
    pub broken: u32,
    /// Aggregated rewards `(item id, count)`, in first-seen order.
    pub rewards: Vec<(u32, u32)>,
    pub stop: BreakStop,
}

impl BreakOutcome {
    fn stopped(stop: BreakStop) -> Self {
        Self {
            broken: 0,
            rewards: Vec::new(),
            stop,
        }
    }

    fn add_reward(&mut self, item_id: u32) {
        if let Some(entry) = self.rewards.iter_mut().find(|(id, _)| *id == item_id) {
            entry.1 += 1;
        } else {
            self.rewards.push((item_id, 1));
        }
    }

    fn merge(&mut self, other: BreakOutcome) {
        self.broken += other.broken;
        for (id, count) in other.rewards {
            if let Some(entry) = self.rewards.iter_mut().find(|(rid, _)| *rid == id) {
                entry.1 += count;
            } else {
                self.rewards.push((id, count));
            }
        }
    }
}

/// Bag stacks of `box_id` that may be broken: right item, non-empty and not rented / char-sealed /
/// duplicate / sealed. The flag values are an enum (see `ITEM_FLAG_*`), so equality is used.
fn slot_is_usable(slot: &UserItemSlot, box_id: u32) -> bool {
    slot.item_id == box_id
        && slot.count > 0
        && !matches!(
            slot.flag,
            ITEM_FLAG_RENTED | ITEM_FLAG_CHAR_SEAL | ITEM_FLAG_DUPLICATE | ITEM_FLAG_SEALED
        )
}

/// Weighted reward pool of the Chaotic Generator: `(reward item id, slots)` per usable exchange
/// row, in the order of `exchanges`, capped to [`POOL_CAP`] slots in total.
///
/// `owned` is how many usable boxes the player currently has (rows that need more are skipped,
/// like the generator's `check_exist_item(piece, origin_item_count1)`).
pub(crate) fn build_reward_pool(exchanges: &[ItemExchangeRow], owned: u32) -> Vec<(u32, u32)> {
    let mut pool: Vec<(u32, u32)> = Vec::new();
    let mut total: u32 = 0;
    for ex in exchanges {
        if ex.random_flag >= 101 {
            continue;
        }
        let required = ex.origin_item_count1.max(0) as u32;
        if required > 0 && owned < required {
            continue;
        }
        if ex.exchange_item_num1 <= 0 {
            continue;
        }
        // `sExchangeItemCount[0] / 5` entries of the 10000-slot array; never negative.
        let slots = (ex.exchange_item_count1.max(0) as u32) / 5;
        let take = slots.min(POOL_CAP - total);
        if take > 0 {
            pool.push((ex.exchange_item_num1 as u32, take));
            total += take;
        }
        if total >= POOL_CAP {
            break;
        }
    }
    pool
}

/// Total number of slots of a pool.
pub(crate) fn pool_total(pool: &[(u32, u32)]) -> u32 {
    pool.iter().map(|(_, slots)| *slots).sum()
}

/// Reward at `roll` (0-based, `< pool_total`) of the pool.
pub(crate) fn pick_from_pool(pool: &[(u32, u32)], roll: u32) -> Option<u32> {
    let mut acc: u32 = 0;
    for &(item_id, slots) in pool {
        acc += slots;
        if roll < acc {
            return Some(item_id);
        }
    }
    None
}

impl WorldState {
    /// True when `item_id` is an item the Chaotic Generator can break: it exists, is countable,
    /// has `effect2 = 251` and owns at least one `random_flag 1..=3` exchange row.
    pub fn is_breakable_box(&self, item_id: u32) -> bool {
        let Some(tmpl) = self.get_item(item_id) else {
            return false;
        };
        tmpl.effect2.unwrap_or(0) == BOX_EFFECT2
            && tmpl.countable.unwrap_or(0) != 0
            && !self.get_bifrost_exchanges(item_id).is_empty()
    }

    /// Usable bag count of one box type.
    fn usable_box_count(&self, sid: SessionId, box_id: u32) -> u32 {
        self.with_session(sid, |h| {
            h.inventory
                .iter()
                .skip(Self::SLOT_MAX)
                .take(Self::HAVE_MAX)
                .filter(|s| slot_is_usable(s, box_id))
                .map(|s| s.count as u32)
                .sum()
        })
        .unwrap_or(0)
    }

    /// Every breakable box type in the player's bag with its usable count, sorted by item id.
    pub fn breakable_boxes(&self, sid: SessionId) -> Vec<BreakableBox> {
        let mut counts: Vec<(u32, u32)> = Vec::new();
        self.with_session(sid, |h| {
            for slot in h.inventory.iter().skip(Self::SLOT_MAX).take(Self::HAVE_MAX) {
                if slot.item_id == 0 || !slot_is_usable(slot, slot.item_id) {
                    continue;
                }
                if let Some(entry) = counts.iter_mut().find(|(id, _)| *id == slot.item_id) {
                    entry.1 += slot.count as u32;
                } else {
                    counts.push((slot.item_id, slot.count as u32));
                }
            }
        });
        counts.sort_by_key(|(id, _)| *id);
        counts
            .into_iter()
            .filter(|(id, _)| self.is_breakable_box(*id))
            .map(|(item_id, count)| BreakableBox {
                item_id,
                count,
                name: self.box_display_name(item_id),
            })
            .collect()
    }

    /// Item name for chat lines (ASCII only: the chat packet is not unicode safe).
    pub fn box_display_name(&self, item_id: u32) -> String {
        let name = self
            .get_item(item_id)
            .and_then(|t| t.str_name)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("Item {item_id}"));
        name.chars()
            .map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { '?' })
            .collect()
    }

    /// Remember the box type the player picked in the menu. Returns the usable count (0 and the
    /// pick is cleared when the item is not a breakable box or the player owns none).
    pub fn box_breaker_pick(&self, sid: SessionId, item_id: u32) -> u32 {
        let owned = if self.is_breakable_box(item_id) {
            self.usable_box_count(sid, item_id)
        } else {
            0
        };
        self.update_session(sid, |h| {
            h.box_breaker_pick = if owned > 0 { item_id } else { 0 };
        });
        owned
    }

    /// The remembered pick (0 = none).
    pub fn box_breaker_picked(&self, sid: SessionId) -> u32 {
        self.with_session(sid, |h| h.box_breaker_pick).unwrap_or(0)
    }

    /// Player-state gate shared by all break calls.
    fn box_breaker_gate(&self, sid: SessionId) -> Option<BreakStop> {
        if self.get_character_info(sid).is_none() {
            return Some(BreakStop::Busy);
        }
        if self.is_player_dead(sid)
            || self.is_trading(sid)
            || self.is_merchanting(sid)
            || self.is_selling_merchant_preparing(sid)
            || self.is_mining(sid)
            || self.is_fishing(sid)
        {
            return Some(BreakStop::Busy);
        }
        // The player must still be next to the NPC whose menu is open (same range as the click).
        let nid = self.with_session(sid, |h| h.event_nid).unwrap_or(-1);
        if nid <= 0 {
            return Some(BreakStop::TooFar);
        }
        let (Some(pos), Some(npc)) = (self.get_position(sid), self.get_npc_instance(nid as u16 as u32))
        else {
            return Some(BreakStop::TooFar);
        };
        let (dx, dz) = (pos.x - npc.x, pos.z - npc.z);
        if pos.zone_id != npc.zone_id
            || (dx * dx + dz * dz).sqrt() > crate::npc_type_constants::MAX_NPC_RANGE
        {
            return Some(BreakStop::TooFar);
        }
        None
    }

    /// Take exactly one usable box out of the bag (client is told the new stack count).
    fn take_one_usable_box(&self, sid: SessionId, box_id: u32) -> bool {
        let mut info: Option<(u8, u32, u16)> = None;
        let ok = self.update_inventory(sid, |inv| {
            for i in Self::SLOT_MAX..(Self::SLOT_MAX + Self::HAVE_MAX) {
                let Some(slot) = inv.get_mut(i) else { break };
                if !slot_is_usable(slot, box_id) {
                    continue;
                }
                slot.count -= 1;
                let (count, durability) = (slot.count, slot.durability);
                if slot.count == 0 {
                    *slot = UserItemSlot::default();
                }
                info = Some(((i - Self::SLOT_MAX) as u8, count as u32, durability.max(0) as u16));
                return true;
            }
            false
        });
        if let (true, Some((pos, count, durability))) = (ok, info) {
            let mut pkt = Packet::new(Opcode::WizItemCountChange as u8);
            pkt.write_u16(1); // count_type
            pkt.write_u8(1); // slot_section: inventory
            pkt.write_u8(pos);
            pkt.write_u32(box_id);
            pkt.write_u32(count);
            pkt.write_u8(0); // bNewItem = false (removal)
            pkt.write_u16(durability);
            pkt.write_u32(0); // reserved
            pkt.write_u32(0); // expiration
            pkt.write_u16(0); // v2625 SendStackChange trailing field
            self.send_to_session_owned(sid, pkt);
            self.set_user_ability(sid);
        }
        ok
    }

    /// Break up to `limit` boxes of one type (`limit == 0` means every usable box of the type,
    /// bounded by [`MAX_BREAK_PER_CALL`]).
    pub fn box_breaker_break(&self, sid: SessionId, box_id: u32, limit: u32) -> BreakOutcome {
        let outcome = self.box_breaker_break_inner(sid, box_id, limit);
        if outcome.broken > 0 {
            self.box_breaker_audit(sid, &[(box_id, outcome.broken)], &outcome);
        }
        outcome
    }

    /// Break up to `limit` boxes (0 = as many as possible, bounded by [`MAX_BREAK_PER_CALL`]) of
    /// every breakable type in the bag, lowest item id first. Stops at the first bag / weight
    /// problem so the player is told why.
    pub fn box_breaker_break_all(&self, sid: SessionId, limit: u32) -> BreakOutcome {
        let budget_total = if limit == 0 {
            MAX_BREAK_PER_CALL
        } else {
            limit.min(MAX_BREAK_PER_CALL)
        };
        let mut total = BreakOutcome::default();
        let mut consumed: Vec<(u32, u32)> = Vec::new();
        let boxes = self.breakable_boxes(sid);
        if boxes.is_empty() {
            return BreakOutcome::stopped(match self.box_breaker_gate(sid) {
                Some(stop) => stop,
                None => BreakStop::NoBox,
            });
        }
        let mut remaining = budget_total;
        for b in boxes {
            if remaining == 0 {
                total.stop = BreakStop::Capped;
                break;
            }
            let part = self.box_breaker_break_inner(sid, b.item_id, remaining);
            if part.broken > 0 {
                consumed.push((b.item_id, part.broken));
            }
            let stop = part.stop;
            remaining -= part.broken.min(remaining);
            total.merge(part);
            match stop {
                BreakStop::Done | BreakStop::NoBox => {}
                BreakStop::Capped => {
                    total.stop = BreakStop::Capped;
                    break;
                }
                // Bag / weight / state problems end the whole run.
                other => {
                    total.stop = other;
                    break;
                }
            }
        }
        // The budget ran out exactly while boxes are still left: tell the player to come again.
        if total.stop == BreakStop::Done && remaining == 0 && !self.breakable_boxes(sid).is_empty() {
            total.stop = BreakStop::Capped;
        }
        if total.broken > 0 {
            self.box_breaker_audit(sid, &consumed, &total);
        }
        total
    }

    fn box_breaker_break_inner(&self, sid: SessionId, box_id: u32, limit: u32) -> BreakOutcome {
        if let Some(stop) = self.box_breaker_gate(sid) {
            return BreakOutcome::stopped(stop);
        }
        let Some(box_tmpl) = self.get_item(box_id) else {
            return BreakOutcome::stopped(BreakStop::NotBreakable);
        };
        if box_tmpl.effect2.unwrap_or(0) != BOX_EFFECT2 || box_tmpl.countable.unwrap_or(0) == 0 {
            return BreakOutcome::stopped(BreakStop::NotBreakable);
        }
        let exchanges = self.get_bifrost_exchanges(box_id);
        if exchanges.is_empty() {
            return BreakOutcome::stopped(BreakStop::NotBreakable);
        }
        // BifrostCheckExchange: a reward item missing from the item table aborts the whole box type.
        for ex in &exchanges {
            if ex.exchange_item_num1 > 0 && self.get_item(ex.exchange_item_num1 as u32).is_none() {
                tracing::warn!(
                    "box_breaker: box {} has a reward row ({}) with unknown item {}",
                    box_id,
                    ex.n_index,
                    ex.exchange_item_num1
                );
                return BreakOutcome::stopped(BreakStop::BadPool);
            }
        }

        let want = if limit == 0 {
            MAX_BREAK_PER_CALL
        } else {
            limit.min(MAX_BREAK_PER_CALL)
        };
        let box_weight = box_tmpl.weight.unwrap_or(0).max(0) as u32;
        let mut out = BreakOutcome::default();

        while out.broken < want {
            let owned = self.usable_box_count(sid, box_id);
            if owned == 0 {
                out.stop = if out.broken == 0 {
                    BreakStop::NoBox
                } else {
                    BreakStop::Done
                };
                return out;
            }
            let pool = build_reward_pool(&exchanges, owned);
            let total_slots = pool_total(&pool);
            if total_slots == 0 {
                out.stop = BreakStop::BadPool;
                return out;
            }
            let roll = {
                use rand::Rng;
                rand::thread_rng().gen_range(0..total_slots)
            };
            let Some(reward_id) = pick_from_pool(&pool, roll) else {
                out.stop = BreakStop::BadPool;
                return out;
            };
            let Some(reward_tmpl) = self.get_item(reward_id) else {
                out.stop = BreakStop::BadPool;
                return out;
            };

            // Weight: the box leaves the bag, the reward enters it.
            let reward_weight = reward_tmpl.weight.unwrap_or(0).max(0) as u32;
            let (item_weight, max_weight) = self
                .with_session(sid, |h| (h.equipped_stats.item_weight, h.equipped_stats.max_weight))
                .unwrap_or((0, 0));
            if item_weight.saturating_sub(box_weight) + reward_weight > max_weight {
                out.stop = BreakStop::Overweight;
                return out;
            }

            // Room: a free slot / stack for the reward, or the box stack is about to disappear.
            let box_stack_frees_slot = self
                .with_session(sid, |h| {
                    h.inventory
                        .iter()
                        .skip(Self::SLOT_MAX)
                        .take(Self::HAVE_MAX)
                        .find(|s| slot_is_usable(s, box_id))
                        .is_some_and(|s| s.count == 1)
                })
                .unwrap_or(false);
            if self.find_slot_for_item(sid, reward_id, 1).is_none() && !box_stack_frees_slot {
                out.stop = BreakStop::BagFull;
                return out;
            }

            // Consume one box, then give the reward; hand the box back if that fails.
            if !self.take_one_usable_box(sid, box_id) {
                out.stop = BreakStop::NoBox;
                return out;
            }
            if !self.give_item(sid, reward_id, 1) {
                let refunded = self.give_item(sid, box_id, 1);
                tracing::warn!(
                    "box_breaker: could not give reward {} for box {} (sid={}), box refunded={}",
                    reward_id,
                    box_id,
                    sid,
                    refunded
                );
                out.stop = BreakStop::BagFull;
                return out;
            }
            out.broken += 1;
            out.add_reward(reward_id);
            self.box_breaker_announce_epic(sid, reward_id, reward_tmpl.item_type.unwrap_or(0));
        }

        out.stop = if limit == 0 && self.usable_box_count(sid, box_id) > 0 {
            BreakStop::Capped
        } else {
            BreakStop::Done
        };
        out
    }

    /// Same server-wide shout as the Chaotic Generator for unique rewards.
    fn box_breaker_announce_epic(&self, sid: SessionId, reward_id: u32, item_type: i32) {
        if item_type != 4 && reward_id != 379_068_000 {
            return;
        }
        let (name, personal_rank) = self
            .with_session(sid, |h| {
                (
                    h.character
                        .as_ref()
                        .map(|c| c.name.clone())
                        .unwrap_or_default(),
                    h.personal_rank,
                )
            })
            .unwrap_or_default();
        let mut pkt = Packet::new(Opcode::WizLogosshout as u8);
        pkt.write_u8(0x02);
        pkt.write_u8(0x04);
        pkt.write_sbyte_string(&name);
        pkt.write_u32(reward_id);
        pkt.write_u8(personal_rank);
        self.broadcast_to_all(Arc::new(pkt), None);
    }

    /// One log line plus the usual give / rob audit rows for a finished run.
    fn box_breaker_audit(&self, sid: SessionId, consumed: &[(u32, u32)], outcome: &BreakOutcome) {
        let name = self.get_session_name(sid).unwrap_or_default();
        tracing::info!(
            "box_breaker: {} broke {:?} -> {:?} (stop={})",
            name,
            consumed,
            outcome.rewards,
            outcome.stop.code()
        );
        let Some(pool) = self.db_pool() else {
            return;
        };
        let account = self
            .with_session(sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let pos = self.get_position(sid);
        let (zone, x, z) = pos
            .as_ref()
            .map(|p| (p.zone_id as i16, p.x as i16, p.z as i16))
            .unwrap_or((0, 0, 0));
        for &(item_id, count) in consumed {
            crate::handler::audit_log::log_rob_item(pool, &account, &name, zone, x, z, item_id, count, 0);
        }
        for &(item_id, count) in &outcome.rewards {
            crate::handler::audit_log::log_give_item(
                pool,
                &account,
                &name,
                zone,
                x,
                z,
                "box_breaker",
                item_id,
                count.min(u16::MAX as u32) as u16,
            );
        }
    }
}

#[cfg(test)]
pub(crate) fn test_exchange_row(
    n_index: i32,
    random_flag: i16,
    origin: i32,
    origin_count: i32,
    reward: i32,
    weight: i32,
) -> ItemExchangeRow {
    ItemExchangeRow {
        n_index,
        random_flag,
        origin_item_num1: origin,
        origin_item_count1: origin_count,
        origin_item_num2: 0,
        origin_item_count2: 0,
        origin_item_num3: 0,
        origin_item_count3: 0,
        origin_item_num4: 0,
        origin_item_count4: 0,
        origin_item_num5: 0,
        origin_item_count5: 0,
        exchange_item_num1: reward,
        exchange_item_count1: weight,
        exchange_item_num2: 0,
        exchange_item_count2: 0,
        exchange_item_num3: 0,
        exchange_item_count3: 0,
        exchange_item_num4: 0,
        exchange_item_count4: 0,
        exchange_item_num5: 0,
        exchange_item_count5: 0,
        exchange_item_time1: 0,
        exchange_item_time2: 0,
        exchange_item_time3: 0,
        exchange_item_time4: 0,
        exchange_item_time5: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Chaotic Generator's own algorithm (`bifrost_piece_exchange`), materialising the array.
    fn generator_array(exchanges: &[ItemExchangeRow], owned: u32) -> Vec<u32> {
        let mut rand_array: Vec<u32> = Vec::with_capacity(10000);
        for ex in exchanges {
            if ex.random_flag >= 101 {
                continue;
            }
            let req_count = ex.origin_item_count1 as u16;
            if req_count > 0 && owned < req_count as u32 {
                continue;
            }
            let slots = (ex.exchange_item_count1 / 5) as usize;
            let reward_id = ex.exchange_item_num1 as u32;
            if reward_id == 0 {
                continue;
            }
            for _ in 0..slots {
                if rand_array.len() >= 10000 {
                    break;
                }
                rand_array.push(reward_id);
            }
            if rand_array.len() >= 10000 {
                break;
            }
        }
        rand_array
    }

    fn rows() -> Vec<ItemExchangeRow> {
        vec![
            test_exchange_row(1, 1, 811053000, 1, 100, 50), // 10 slots
            test_exchange_row(2, 2, 811053000, 1, 200, 4),  // 0 slots (4 / 5)
            test_exchange_row(3, 3, 811053000, 1, 300, 155), // 31 slots
            test_exchange_row(4, 1, 811053000, 1, 0, 500),  // no reward -> skipped
            test_exchange_row(5, 101, 811053000, 1, 400, 500), // generator "101" rows are skipped
            test_exchange_row(6, 1, 811053000, 3, 500, 100), // needs 3 boxes
            test_exchange_row(7, 1, 811053000, 1, 600, 5),  // 1 slot
        ]
    }

    #[test]
    fn pool_matches_the_generator_array_for_every_roll() {
        for owned in [1u32, 2, 3, 10] {
            let ex = rows();
            let array = generator_array(&ex, owned);
            let pool = build_reward_pool(&ex, owned);
            assert_eq!(pool_total(&pool) as usize, array.len(), "owned={owned}");
            for (roll, expected) in array.iter().enumerate() {
                assert_eq!(
                    pick_from_pool(&pool, roll as u32),
                    Some(*expected),
                    "owned={owned} roll={roll}"
                );
            }
            assert_eq!(pick_from_pool(&pool, array.len() as u32), None);
        }
    }

    #[test]
    fn pool_skips_rows_that_need_more_boxes_than_owned() {
        let ex = rows();
        let one = build_reward_pool(&ex, 1);
        assert!(!one.iter().any(|(id, _)| *id == 500));
        let three = build_reward_pool(&ex, 3);
        assert!(three.contains(&(500, 20)));
    }

    #[test]
    fn pool_ignores_zero_negative_and_special_rows() {
        let ex = vec![
            test_exchange_row(1, 1, 7, 1, 100, -500), // negative weight must not wrap around
            test_exchange_row(2, 1, 7, 1, -5, 100),   // negative reward id
            test_exchange_row(3, 101, 7, 1, 300, 100),
            test_exchange_row(4, 1, 7, 1, 400, 10),
        ];
        let pool = build_reward_pool(&ex, 1);
        assert_eq!(pool, vec![(400, 2)]);
        assert_eq!(pick_from_pool(&pool, 0), Some(400));
        assert_eq!(pick_from_pool(&pool, 1), Some(400));
        assert_eq!(pick_from_pool(&pool, 2), None);
        assert!(build_reward_pool(&[], 1).is_empty());
    }

    #[test]
    fn pool_is_capped_to_10000_slots_like_the_generator() {
        let ex = vec![
            test_exchange_row(1, 1, 7, 1, 100, 30_000), // 6000 slots
            test_exchange_row(2, 1, 7, 1, 200, 30_000), // 6000 -> truncated to 4000
            test_exchange_row(3, 1, 7, 1, 300, 30_000), // nothing left
        ];
        let pool = build_reward_pool(&ex, 1);
        assert_eq!(pool, vec![(100, 6000), (200, 4000)]);
        assert_eq!(pool_total(&pool), 10_000);
        let array = generator_array(&ex, 1);
        assert_eq!(array.len(), 10_000);
        assert_eq!(array[5999], 100);
        assert_eq!(array[6000], 200);
    }

    #[test]
    fn usable_slot_rules_use_flag_equality() {
        let mk = |flag: u8, count: u16| UserItemSlot {
            item_id: 5,
            count,
            flag,
            ..Default::default()
        };
        assert!(slot_is_usable(&mk(ITEM_FLAG_NONE, 1), 5));
        assert!(slot_is_usable(&mk(ITEM_FLAG_NOT_BOUND, 1), 5));
        assert!(slot_is_usable(&mk(ITEM_FLAG_BOUND, 1), 5));
        assert!(!slot_is_usable(&mk(ITEM_FLAG_RENTED, 1), 5));
        assert!(!slot_is_usable(&mk(ITEM_FLAG_CHAR_SEAL, 1), 5));
        assert!(!slot_is_usable(&mk(ITEM_FLAG_DUPLICATE, 1), 5));
        assert!(!slot_is_usable(&mk(ITEM_FLAG_SEALED, 1), 5));
        assert!(!slot_is_usable(&mk(ITEM_FLAG_NONE, 0), 5), "empty stack");
        assert!(!slot_is_usable(&mk(ITEM_FLAG_NONE, 1), 6), "other item");
    }

    #[test]
    fn stop_codes_are_unique() {
        let all = [
            BreakStop::Done,
            BreakStop::Capped,
            BreakStop::NoBox,
            BreakStop::NotBreakable,
            BreakStop::Busy,
            BreakStop::TooFar,
            BreakStop::BagFull,
            BreakStop::Overweight,
            BreakStop::BadPool,
        ];
        let mut codes: Vec<_> = all.iter().map(|s| s.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
    }
}
