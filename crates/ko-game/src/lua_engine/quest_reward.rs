//! Reward planning of `RunQuestExchange` (which rows of an exchange are paid).
//!
//! The client tables define the reward of a quest in two rows with the same index:
//!
//! * `Item_Exchange` (`item_exchange`): turn-in items + reward slots.
//! * `Item_Exchange_exp` (`item_exchange_exp`): the "exp part" (EXP, national points, an extra item).
//!
//! `random_flag` 11 and 12 (on either row) mark a **choose one** list: the player sees all the filled
//! slots (usually one item per class family, or a free choice such as food / accessories) and picks
//! ONE of them in the reward window; the client sends the 0-based index of the pick
//! (`by_selected_reward`). Every other flag (0, 10, 13 ...) pays all filled slots.
//!
//! The old engine paid ALL slots of the item row (the player picked one of 3 and got all 3) and used
//! the exp row as the choose list (`exp[selected]`), so the pick of a weapon (index 1..3) addressed an
//! empty exp slot and the whole reward was refused.

use ko_db::models::item_tables::{ItemExchangeExpRow, ItemExchangeRow};

// ---------------------------------------------------------------------------------------------
// Outcome trace (used by the quest simulation harness and handy when debugging live reports).
// ---------------------------------------------------------------------------------------------

use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// One observable step of a quest script run.
#[derive(Debug, Clone)]
pub(crate) enum TraceEvent {
    /// `RunQuestExchange` finished (`reason` is empty on success).
    Exchange {
        id: i32,
        sel: i32,
        ok: bool,
        reason: &'static str,
        paid: Vec<(u32, u32)>,
    },
    /// `SaveEvent` was called (`blocked` = completion refused because a reward was refused).
    Save {
        helper: u16,
        quest: u16,
        status: u8,
        blocked: bool,
    },
}

pub(crate) static TRACE_ON: AtomicBool = AtomicBool::new(false);
pub(crate) static TRACE: Mutex<Vec<TraceEvent>> = Mutex::new(Vec::new());

thread_local! {
    static REASON: Cell<&'static str> = const { Cell::new("") };
    static PAID: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn trace(ev: TraceEvent) {
    if TRACE_ON.load(Ordering::Relaxed) {
        if let Ok(mut t) = TRACE.lock() {
            t.push(ev);
        }
    }
}

/// Reset the per-call state at the start of a `RunQuestExchange`.
pub(crate) fn begin_call() {
    REASON.with(|r| r.set(""));
    PAID.with(|p| p.borrow_mut().clear());
}

/// Remember why the current `RunQuestExchange` refused the reward.
pub(crate) fn set_reason(reason: &'static str) {
    REASON.with(|r| r.set(reason));
}

pub(crate) fn take_reason() -> &'static str {
    REASON.with(|r| {
        let v = r.get();
        r.set("");
        if v.is_empty() {
            "unknown"
        } else {
            v
        }
    })
}

pub(crate) fn set_paid(list: &[(u32, u32)]) {
    PAID.with(|p| *p.borrow_mut() = list.to_vec());
}

pub(crate) fn take_paid() -> Vec<(u32, u32)> {
    PAID.with(|p| std::mem::take(&mut *p.borrow_mut()))
}

/// `random_flag` values that mark a choose-one list.
pub(crate) fn is_choice_flag(flag: i32) -> bool {
    flag == 11 || flag == 12
}

fn is_virtual(id: u32) -> bool {
    matches!(
        id,
        900_000_000 | 900_001_000 | 900_002_000 | 900_003_000 | 900_007_000
    )
}

/// Filled output slots `(item, count)` of an `item_exchange` row.
pub(crate) fn main_slots(e: &ItemExchangeRow) -> Vec<(u32, u32)> {
    [
        (e.exchange_item_num1, e.exchange_item_count1),
        (e.exchange_item_num2, e.exchange_item_count2),
        (e.exchange_item_num3, e.exchange_item_count3),
        (e.exchange_item_num4, e.exchange_item_count4),
        (e.exchange_item_num5, e.exchange_item_count5),
    ]
    .iter()
    .filter(|(n, c)| *n > 0 && *c > 0)
    .map(|(n, c)| (*n as u32, *c as u32))
    .collect()
}

/// Filled slots `(item, count)` of an `item_exchange_exp` row.
pub(crate) fn exp_slots(e: &ItemExchangeExpRow) -> Vec<(u32, u32)> {
    [
        (e.exchange_item_num1, e.exchange_item_count1),
        (e.exchange_item_num2, e.exchange_item_count2),
        (e.exchange_item_num3, e.exchange_item_count3),
        (e.exchange_item_num4, e.exchange_item_count4),
        (e.exchange_item_num5, e.exchange_item_count5),
    ]
    .iter()
    .filter_map(|(n, c)| match (n, c) {
        (Some(n), Some(c)) if *n > 0 && *c > 0 => Some((*n as u32, *c as u32)),
        _ => None,
    })
    .collect()
}

/// The rewards that were picked out of a choose-one list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picked {
    /// Indexes in the candidate list that are paid (one entry, or all when nothing was picked and no
    /// entry is class specific).
    pub indexes: Vec<usize>,
    /// The client index that was asked for (`-1` = none sent).
    pub asked: i32,
    /// True when the paid reward differs from the one at `asked` (class substitution / bad index).
    pub substituted: bool,
}

/// Choose the reward of a choose-one list.
///
/// * `sel` is the client's 0-based index (`-1` = nothing sent: the script never showed the reward
///   window, e.g. a plain yes/no dialog).
/// * `exists(id)`: the item is in the item table (virtual ids always exist).
/// * `class_ok(id)`: the player's class may use the item.
///
/// A pick is paid as it is when it exists and the player's class can use it. Otherwise the first
/// entry that exists and fits the class is paid instead (class-family lists: the player gets the
/// variant of his own class even if the window index points at another one). With no pick at all
/// and no class specific entry the whole list is paid (nothing to choose from the player's point of
/// view). `None` only when no entry exists at all.
pub(crate) fn pick_choice(
    cands: &[(u32, u32)],
    sel: i32,
    exists: &dyn Fn(u32) -> bool,
    class_ok: &dyn Fn(u32) -> bool,
) -> Option<Picked> {
    if cands.is_empty() {
        return None;
    }
    let usable = |i: usize| {
        let id = cands[i].0;
        is_virtual(id) || (exists(id) && class_ok(id))
    };
    if sel >= 0 && (sel as usize) < cands.len() && usable(sel as usize) {
        return Some(Picked {
            indexes: vec![sel as usize],
            asked: sel,
            substituted: false,
        });
    }
    if sel < 0 && cands.len() > 1 && (0..cands.len()).all(usable) {
        return Some(Picked {
            indexes: (0..cands.len()).collect(),
            asked: sel,
            substituted: true,
        });
    }
    let fallback = (0..cands.len())
        .find(|&i| usable(i))
        .or_else(|| (0..cands.len()).find(|&i| is_virtual(cands[i].0) || exists(cands[i].0)))?;
    Some(Picked {
        indexes: vec![fallback],
        asked: sel,
        substituted: true,
    })
}

/// Result of [`plan_rows`].
#[derive(Debug, Default)]
pub(crate) struct Plan {
    /// Everything that is paid, in order.
    pub list: Vec<(u32, u32)>,
    /// Indexes into `list` of the rewards that came out of a choose-one list.
    pub chosen: Vec<usize>,
    /// Human readable notes (substitutions, dropped duplicates) for the logs.
    pub notes: Vec<String>,
    /// True when a choose-one list exists but no entry could be paid.
    pub choice_failed: bool,
}

/// Plan the rewards of the item row (`main`) and the exp row.
///
/// `main` is the filled slot list of the item row, `main_is_choice` is its flag (11/12), `exp` the
/// optional exp row with its own choice flag. Exp slots that repeat an item of the item row are
/// dropped (the client tables list the "default" variant again in the exp row; paying it would hand
/// out the item twice).
pub(crate) fn plan_rows(
    main: &[(u32, u32)],
    main_is_choice: bool,
    exp: Option<(&[(u32, u32)], bool)>,
    sel: i32,
    exists: &dyn Fn(u32) -> bool,
    class_ok: &dyn Fn(u32) -> bool,
) -> Plan {
    let mut plan = Plan::default();
    if main_is_choice && main.len() > 1 {
        match pick_choice(main, sel, exists, class_ok) {
            Some(p) => {
                if p.substituted {
                    plan.notes
                        .push(format!("choice {} -> slots {:?}", p.asked, p.indexes));
                }
                for &i in &p.indexes {
                    plan.chosen.push(plan.list.len());
                    plan.list.push(main[i]);
                }
            }
            None => plan.choice_failed = true,
        }
    } else {
        plan.list.extend_from_slice(main);
    }
    if let Some((slots, exp_is_choice)) = exp {
        let dup = |id: u32| !is_virtual(id) && main.iter().any(|(m, _)| *m == id);
        if exp_is_choice && slots.len() > 1 {
            match pick_choice(slots, sel, exists, class_ok) {
                Some(p) => {
                    if p.substituted {
                        plan.notes
                            .push(format!("exp choice {} -> slots {:?}", p.asked, p.indexes));
                    }
                    for &i in &p.indexes {
                        plan.chosen.push(plan.list.len());
                        plan.list.push(slots[i]);
                    }
                }
                None => plan.choice_failed = true,
            }
        } else {
            for &(id, c) in slots {
                if dup(id) {
                    plan.notes.push(format!(
                        "exp row item {id} repeats the item row - not paid twice"
                    ));
                    continue;
                }
                plan.list.push((id, c));
            }
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yes(_: u32) -> bool {
        true
    }

    #[test]
    fn choice_pays_only_the_picked_slot() {
        let main = [(101, 1), (102, 1), (103, 1)];
        for sel in 0..3 {
            let p = plan_rows(&main, true, None, sel, &yes, &yes);
            assert_eq!(p.list, vec![main[sel as usize]]);
            assert!(!p.choice_failed);
        }
    }

    #[test]
    fn fixed_row_pays_everything() {
        let main = [(101, 1), (102, 2), (900_001_000, 500)];
        let p = plan_rows(&main, false, None, 2, &yes, &yes);
        assert_eq!(p.list, main.to_vec());
    }

    #[test]
    fn exp_row_is_paid_with_the_picked_item() {
        let main = [(101, 1), (102, 1)];
        let exp = [(900_001_000, 7_500_000), (900_002_000, 100)];
        let p = plan_rows(&main, true, Some((&exp, false)), 1, &yes, &yes);
        assert_eq!(
            p.list,
            vec![(102, 1), (900_001_000, 7_500_000), (900_002_000, 100)]
        );
    }

    #[test]
    fn exp_only_row_pays_exp_for_any_pick_index() {
        let main = [(101, 1), (102, 1), (103, 1), (104, 1)];
        let exp = [(900_001_000, 675_000)];
        let p = plan_rows(&main, true, Some((&exp, false)), 3, &yes, &yes);
        assert_eq!(p.list, vec![(104, 1), (900_001_000, 675_000)]);
    }

    #[test]
    fn exp_row_repeating_the_default_item_is_not_paid_twice() {
        let main = [(101, 1), (102, 1)];
        let exp = [(101, 1), (900_001_000, 25_000_000)];
        let p = plan_rows(&main, true, Some((&exp, false)), 1, &yes, &yes);
        assert_eq!(p.list, vec![(102, 1), (900_001_000, 25_000_000)]);
    }

    #[test]
    fn exp_choice_list_picks_one_slot() {
        let main = [(900_001_000, 3500)];
        let exp = [(201, 1), (202, 1), (203, 1), (204, 1)];
        let p = plan_rows(&main, false, Some((&exp, true)), 2, &yes, &yes);
        assert_eq!(p.list, vec![(900_001_000, 3500), (203, 1)]);
    }

    #[test]
    fn class_variant_replaces_a_pick_the_class_cannot_use() {
        let main = [(101, 1), (102, 1), (103, 1), (104, 1)];
        // the player is a rogue: only 102 fits
        let class_ok = |id: u32| id == 102;
        let p = plan_rows(&main, true, None, 3, &yes, &class_ok);
        assert_eq!(p.list, vec![(102, 1)]);
        assert_eq!(p.notes.len(), 1);
        // the pick that fits is kept
        let p = plan_rows(&main, true, None, 1, &yes, &class_ok);
        assert_eq!(p.list, vec![(102, 1)]);
        assert!(p.notes.is_empty());
    }

    #[test]
    fn bad_index_falls_back_instead_of_paying_nothing() {
        let main = [(101, 1), (102, 1)];
        let p = plan_rows(&main, true, None, 4, &yes, &yes);
        assert_eq!(p.list, vec![(101, 1)]);
    }

    #[test]
    fn no_pick_pays_all_unless_class_specific() {
        // free choice (food, accessories) but the script never showed the window: pay everything
        let main = [(101, 1), (102, 1)];
        let p = plan_rows(&main, true, None, -1, &yes, &yes);
        assert_eq!(p.list, main.to_vec());
        // class variants: only the variant of the player's class
        let class_ok = |id: u32| id == 102;
        let p = plan_rows(&main, true, None, -1, &yes, &class_ok);
        assert_eq!(p.list, vec![(102, 1)]);
    }

    #[test]
    fn unknown_items_are_skipped_in_a_choice() {
        let main = [(101, 1), (102, 1)];
        let exists = |id: u32| id == 102;
        let p = plan_rows(&main, true, None, 0, &exists, &yes);
        assert_eq!(p.list, vec![(102, 1)]);
        let none = |_: u32| false;
        let p = plan_rows(&main, true, None, 0, &none, &yes);
        assert!(p.choice_failed);
    }
}
