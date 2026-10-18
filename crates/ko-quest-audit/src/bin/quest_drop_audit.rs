use anyhow::{Context, Result};
use ko_quest_audit::tbl_loader::TblData;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const VIRTUAL_ITEMS: &[i32] = &[
    900_000_000, // Noah
    900_001_000, // EXP
    900_002_000, // chat/event
    900_004_000, // skill
    900_005_000, // ITEM_HUNT (kill counter, not inventory)
    900_006_000,
    900_007_000,
    900_008_000,
    900_009_000,
    900_010_000,
    900_011_000,
    900_012_000,
    900_016_000,
    810_000_000,
];

fn main() -> Result<()> {
    let data_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: quest_drop_audit <client Data dir>")?;
    let data = TblData::load(&data_dir)?;

    let produced_items: BTreeSet<i32> = data
        .item_exchanges
        .values()
        .flat_map(|exchange| exchange.exchange_items.iter().map(|&(item, _, _)| item))
        .filter(|&item| item > 0)
        .collect();
    let mut candidates: BTreeMap<(i32, i32), (BTreeSet<i32>, BTreeSet<i32>, BTreeSet<i32>, bool)> =
        BTreeMap::new();

    for (quest_id, helper_indexes) in &data.helpers_by_quest {
        let monster_ids: BTreeSet<i32> = data
            .monster_exchanges
            .get(quest_id)
            .into_iter()
            .flat_map(|exchange| exchange.groups.iter())
            .flat_map(|group| group.monster_ids.iter().copied())
            .filter(|&monster_id| monster_id >= 100)
            .collect();

        for &helper_index in helper_indexes {
            let helper = &data.quest_helpers[helper_index];
            // Status 1 is the active/turn-in row. Other states repeat the same
            // exchange and would only duplicate the audit.
            if helper.event_status != 1 || helper.exchange_index <= 0 {
                continue;
            }
            let Some(exchange) = data.item_exchanges.get(&helper.exchange_index) else {
                continue;
            };
            for &(item_id, count) in &exchange.origin_items {
                if item_id <= 0 || count <= 0 || VIRTUAL_ITEMS.contains(&item_id) {
                    continue;
                }
                let entry = candidates.entry((*quest_id, item_id)).or_insert_with(|| {
                    (
                        BTreeSet::new(),
                        BTreeSet::new(),
                        BTreeSet::new(),
                        produced_items.contains(&item_id),
                    )
                });
                entry.0.extend(monster_ids.iter().copied());
                entry.1.insert(count);
                entry.2.insert(helper.exchange_index);
            }
        }
    }

    println!("quest_id,item_id,required_counts,exchange_ids,produced_by_exchange,monster_ids");
    for ((quest_id, item_id), (monsters, counts, exchanges, produced)) in candidates {
        let counts = counts
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join("|");
        let exchanges = exchanges
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join("|");
        let monsters = monsters
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join("|");
        println!("{quest_id},{item_id},{counts},{exchanges},{produced},{monsters}");
    }
    Ok(())
}
