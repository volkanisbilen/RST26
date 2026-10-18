//! Loads and indexes quest-related TBL data from client files.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context};
use ko_tbl_import::decrypt;
use ko_tbl_import::parser::{CellValue, TblTable};

/// Load and parse a single TBL file from disk.
fn load_tbl(path: &Path) -> anyhow::Result<TblTable> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let (decrypted, new_structure) =
        decrypt::decrypt_tbl(&data).with_context(|| format!("decrypting {}", path.display()))?;
    ko_tbl_import::parser::parse_tbl(&decrypted, new_structure)
        .with_context(|| format!("parsing {}", path.display()))
}

/// Extract i32 from CellValue (coercing small types).
fn cell_to_i32(cell: &CellValue) -> i32 {
    match cell {
        CellValue::I8(v) => *v as i32,
        CellValue::U8(v) => *v as i32,
        CellValue::I16(v) => *v as i32,
        CellValue::U16(v) => *v as i32,
        CellValue::I32(v) => *v,
        CellValue::U32(v) => *v as i32,
        CellValue::I64(v) => *v as i32,
        CellValue::U64(v) => *v as i32,
        CellValue::F32(v) => *v as i32,
        CellValue::F64(v) => *v as i32,
        CellValue::Str(_) => 0,
    }
}

/// Extract string from CellValue.
fn cell_to_string(cell: &CellValue) -> String {
    match cell {
        CellValue::Str(s) => s.clone(),
        other => format!("{:?}", other),
    }
}

// ─── Quest Helper (from TBL) ────────────────────────────────────────────────

/// A quest_helper row from the TBL file.
/// Column layout matches Quest_Helper.tbl (27 columns).
#[derive(Debug, Clone)]
pub struct TblQuestHelper {
    pub index: i32,            // col_0: n_index
    pub msg_type: i32,         // col_1: b_message_type
    pub min_level: i32,        // col_2: b_level
    pub exp: i32,              // col_3: n_exp
    pub class: i32,            // col_4: b_class
    pub nation: i32,           // col_5: b_nation
    pub quest_type: i32,       // col_6: b_quest_type
    pub zone: i32,             // col_7: b_zone
    pub npc_id: i32,           // col_8: s_npc_id
    pub event_data_index: i32, // col_9: s_event_data_index (quest ID)
    pub event_status: i32,     // col_10: b_event_status
    pub event_trigger: i32,    // col_11: n_event_trigger_index
    pub event_complete: i32,   // col_12: n_event_complete_index
    pub exchange_index: i32,   // col_13: n_exchange_index
    pub event_talk: i32,       // col_14: n_event_talk_index
    pub lua_filename: String,  // col_15: str_lua_filename
    pub quest_menu: i32,       // col_16: s_quest_menu
    pub npc_main: i32,         // col_17: s_npc_main
    pub quest_solo: i32,       // col_18: s_quest_solo
}

// ─── Quest Talk ─────────────────────────────────────────────────────────────

/// A quest_talk row (text_id → dialog text + extras).
#[derive(Debug, Clone)]
pub struct TblQuestTalk {
    pub text_id: i32,
    pub text: String,
    pub extra_1: i32,
    pub extra_2: i32,
}

// ─── Quest Menu ─────────────────────────────────────────────────────────────

/// A quest_menu row (menu_id → button/menu text).
#[derive(Debug, Clone)]
pub struct TblQuestMenu {
    pub menu_id: i32,
    pub text: String,
}

// ─── Item Exchange ──────────────────────────────────────────────────────────

/// An item_exchange row.
#[derive(Debug, Clone)]
pub struct TblItemExchange {
    pub index: i32,
    pub random_flag: i32,
    pub origin_items: Vec<(i32, i32)>, // (item_id, count) × 5
    pub exchange_items: Vec<(i32, i32, i32)>, // (item_id, count, time) × 5
}

// ─── Quest NPC Desc ─────────────────────────────────────────────────────────

/// An NPC description row from quest_npc_desc.
#[derive(Debug, Clone)]
pub struct TblQuestNpcDesc {
    pub row_index: i32, // col_0: sequential index
    pub npc_id: i32,    // col_1: actual NPC ID
    pub col_8: i32,     // col_8: portrait text_id
}

// ─── Quest Monster Exchange ─────────────────────────────────────────────────

/// Monster exchange row (quest_id → monster groups).
#[derive(Debug, Clone)]
pub struct TblQuestMonsterExchange {
    pub quest_num: i32,
    pub groups: Vec<MonsterGroup>,
}

/// A group of monsters to kill.
#[derive(Debug, Clone)]
pub struct MonsterGroup {
    pub monster_ids: Vec<i32>,
    pub count: i32,
}

// ─── All TBL Data ───────────────────────────────────────────────────────────

/// All loaded TBL data, indexed for fast lookup.
pub struct TblData {
    /// quest_helper rows, indexed by (npc_id, event_data_index).
    pub quest_helpers: Vec<TblQuestHelper>,
    /// npc_id → list of quest_helper entries for that NPC.
    pub helpers_by_npc: HashMap<i32, Vec<usize>>,
    /// event_data_index → list of quest_helper entries for that quest.
    pub helpers_by_quest: HashMap<i32, Vec<usize>>,

    /// text_id → QuestTalk.
    pub quest_talks: HashMap<i32, TblQuestTalk>,

    /// menu_id → QuestMenu.
    pub quest_menus: HashMap<i32, TblQuestMenu>,

    /// exchange_index → ItemExchange.
    pub item_exchanges: HashMap<i32, TblItemExchange>,

    /// npc_id → QuestNpcDesc.
    pub npc_descs: HashMap<i32, TblQuestNpcDesc>,

    /// quest_num → QuestMonsterExchange.
    pub monster_exchanges: HashMap<i32, TblQuestMonsterExchange>,
}

impl TblData {
    /// Load all quest-related TBL files from the given data directory.
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let quest_helpers = load_quest_helper(data_dir)?;
        let quest_talks = load_quest_talk(data_dir)?;
        let quest_menus = load_quest_menu(data_dir)?;
        let item_exchanges = load_item_exchange(data_dir)?;
        let npc_descs = load_quest_npc_desc(data_dir)?;
        let monster_exchanges = load_quest_monster_exchange(data_dir)?;

        // Build indices
        let mut helpers_by_npc: HashMap<i32, Vec<usize>> = HashMap::new();
        let mut helpers_by_quest: HashMap<i32, Vec<usize>> = HashMap::new();
        for (i, h) in quest_helpers.iter().enumerate() {
            helpers_by_npc.entry(h.npc_id).or_default().push(i);
            helpers_by_quest
                .entry(h.event_data_index)
                .or_default()
                .push(i);
        }

        tracing::info!(
            helpers = quest_helpers.len(),
            talks = quest_talks.len(),
            menus = quest_menus.len(),
            exchanges = item_exchanges.len(),
            npcs = npc_descs.len(),
            monsters = monster_exchanges.len(),
            "TBL data loaded"
        );

        Ok(Self {
            quest_helpers,
            helpers_by_npc,
            helpers_by_quest,
            quest_talks,
            quest_menus,
            item_exchanges,
            npc_descs,
            monster_exchanges,
        })
    }

    /// Get all unique NPC IDs that have quest_helper entries.
    pub fn all_quest_npc_ids(&self) -> Vec<i32> {
        let mut ids: Vec<i32> = self.helpers_by_npc.keys().copied().collect();
        ids.sort();
        ids
    }
}

// ─── Individual TBL loaders ─────────────────────────────────────────────────

fn find_tbl(data_dir: &Path, names: &[&str]) -> anyhow::Result<std::path::PathBuf> {
    for name in names {
        let path = data_dir.join(name);
        if path.exists() {
            return Ok(path);
        }
    }
    bail!(
        "TBL file not found in {}: tried {:?}",
        data_dir.display(),
        names
    );
}

fn load_quest_helper(data_dir: &Path) -> anyhow::Result<Vec<TblQuestHelper>> {
    let path = find_tbl(data_dir, &["Quest_Helper.tbl", "quest_helper.tbl"])?;
    let tbl = load_tbl(&path)?;
    let col_count = tbl.columns.len();
    tracing::info!(rows = tbl.rows.len(), cols = col_count, "Quest_Helper.tbl");

    // TBL has 21 columns (2 extra vs DB's 19). Column mapping:
    // TBL col_0=n_index, col_1=b_message_type, col_2=b_level, col_3=n_exp,
    // col_4=extra1, col_5=extra2, col_6=b_class, col_7=b_nation,
    // col_8=b_quest_type, col_9=b_zone, col_10=s_npc_id,
    // col_11=s_event_data_index, col_12=b_event_status,
    // col_13=n_event_trigger_index, col_14=n_event_complete_index,
    // col_15=n_exchange_index, col_16=n_event_talk_index,
    // col_17=str_lua_filename, col_18=s_quest_menu,
    // col_19=s_npc_main, col_20=s_quest_solo
    //
    // If col_count == 19, assume DB-style layout (no extras).
    let has_extras = col_count >= 21;
    let off: usize = if has_extras { 2 } else { 0 }; // offset for extra cols

    let min_cols = if has_extras { 21 } else { 19 };
    let mut result = Vec::with_capacity(tbl.rows.len());
    for row in &tbl.rows {
        if row.len() < min_cols {
            continue;
        }
        result.push(TblQuestHelper {
            index: cell_to_i32(&row[0]),
            msg_type: cell_to_i32(&row[1]),
            min_level: cell_to_i32(&row[2]),
            exp: cell_to_i32(&row[3]),
            class: cell_to_i32(&row[4 + off]),
            nation: cell_to_i32(&row[5 + off]),
            quest_type: cell_to_i32(&row[6 + off]),
            zone: cell_to_i32(&row[7 + off]),
            npc_id: cell_to_i32(&row[8 + off]),
            event_data_index: cell_to_i32(&row[9 + off]),
            event_status: cell_to_i32(&row[10 + off]),
            event_trigger: cell_to_i32(&row[11 + off]),
            event_complete: cell_to_i32(&row[12 + off]),
            exchange_index: cell_to_i32(&row[13 + off]),
            event_talk: cell_to_i32(&row[14 + off]),
            lua_filename: cell_to_string(&row[15 + off]),
            quest_menu: cell_to_i32(&row[16 + off]),
            npc_main: cell_to_i32(&row[17 + off]),
            quest_solo: cell_to_i32(&row[18 + off]),
        });
    }
    Ok(result)
}

fn load_quest_talk(data_dir: &Path) -> anyhow::Result<HashMap<i32, TblQuestTalk>> {
    let path = find_tbl(
        data_dir,
        &["Quest_Talk_TK.tbl", "Quest_Talk.tbl", "quest_talk_tk.tbl"],
    )?;
    let tbl = load_tbl(&path)?;
    tracing::info!(
        rows = tbl.rows.len(),
        cols = tbl.columns.len(),
        "Quest_Talk_TK.tbl"
    );

    let mut result = HashMap::with_capacity(tbl.rows.len());
    for row in &tbl.rows {
        if row.len() < 2 {
            continue;
        }
        let text_id = cell_to_i32(&row[0]);
        let text = cell_to_string(&row[1]);
        let extra_1 = if row.len() > 2 {
            cell_to_i32(&row[2])
        } else {
            0
        };
        let extra_2 = if row.len() > 3 {
            cell_to_i32(&row[3])
        } else {
            0
        };
        result.insert(
            text_id,
            TblQuestTalk {
                text_id,
                text,
                extra_1,
                extra_2,
            },
        );
    }
    Ok(result)
}

fn load_quest_menu(data_dir: &Path) -> anyhow::Result<HashMap<i32, TblQuestMenu>> {
    let path = find_tbl(
        data_dir,
        &["Quest_Menu_TK.tbl", "Quest_Menu.tbl", "quest_menu_tk.tbl"],
    )?;
    let tbl = load_tbl(&path)?;
    tracing::info!(
        rows = tbl.rows.len(),
        cols = tbl.columns.len(),
        "Quest_Menu_TK.tbl"
    );

    let mut result = HashMap::with_capacity(tbl.rows.len());
    for row in &tbl.rows {
        if row.len() < 2 {
            continue;
        }
        let menu_id = cell_to_i32(&row[0]);
        let text = cell_to_string(&row[1]);
        result.insert(menu_id, TblQuestMenu { menu_id, text });
    }
    Ok(result)
}

fn load_item_exchange(data_dir: &Path) -> anyhow::Result<HashMap<i32, TblItemExchange>> {
    let path = find_tbl(data_dir, &["Item_Exchange.tbl", "item_exchange.tbl"])?;
    let tbl = load_tbl(&path)?;
    tracing::info!(
        rows = tbl.rows.len(),
        cols = tbl.columns.len(),
        "Item_Exchange.tbl"
    );

    // v2615 Item_Exchange layout is interleaved and contains a TBL-only
    // field at col_2. This matches the importer migration and runtime model:
    // col_3..12  = five (origin_item_num, origin_item_count) pairs
    // col_13..22 = five (exchange_item_num, exchange_item_count) pairs
    // col_23..26 = output times 1..4 (time5 is absent/zero in the TBL).
    let mut result = HashMap::with_capacity(tbl.rows.len());
    for row in &tbl.rows {
        if row.len() < 27 {
            continue;
        }
        let index = cell_to_i32(&row[0]);
        let random_flag = cell_to_i32(&row[1]);
        let mut origin_items = Vec::new();
        for i in 0..5 {
            let item_id = cell_to_i32(&row[3 + i * 2]);
            let count = cell_to_i32(&row[4 + i * 2]);
            if item_id != 0 {
                origin_items.push((item_id, count));
            }
        }
        let mut exchange_items = Vec::new();
        for i in 0..5 {
            let item_id = cell_to_i32(&row[13 + i * 2]);
            let count = cell_to_i32(&row[14 + i * 2]);
            let time = if i < 4 { cell_to_i32(&row[23 + i]) } else { 0 };
            if item_id != 0 {
                exchange_items.push((item_id, count, time));
            }
        }
        result.insert(
            index,
            TblItemExchange {
                index,
                random_flag,
                origin_items,
                exchange_items,
            },
        );
    }
    Ok(result)
}

fn load_quest_npc_desc(data_dir: &Path) -> anyhow::Result<HashMap<i32, TblQuestNpcDesc>> {
    let path = find_tbl(
        data_dir,
        &[
            "quest_npc_desc_tk.tbl",
            "Quest_Npc_Desc_TK.tbl",
            "quest_npc_desc.tbl",
        ],
    )?;
    let tbl = load_tbl(&path)?;
    tracing::info!(
        rows = tbl.rows.len(),
        cols = tbl.columns.len(),
        "quest_npc_desc_tk.tbl"
    );

    let mut result = HashMap::with_capacity(tbl.rows.len());
    for row in &tbl.rows {
        if row.len() < 9 {
            continue;
        }
        let row_index = cell_to_i32(&row[0]);
        let npc_id = cell_to_i32(&row[1]);
        let col_8 = cell_to_i32(&row[8]);
        result.insert(
            npc_id,
            TblQuestNpcDesc {
                row_index,
                npc_id,
                col_8,
            },
        );
    }
    Ok(result)
}

fn load_quest_monster_exchange(
    data_dir: &Path,
) -> anyhow::Result<HashMap<i32, TblQuestMonsterExchange>> {
    let path = find_tbl(
        data_dir,
        &["Quest_Monster_Exchange.tbl", "quest_monster_exchange.tbl"],
    );
    let path = match path {
        Ok(p) => p,
        Err(_) => {
            tracing::warn!("Quest_Monster_Exchange.tbl not found, skipping");
            return Ok(HashMap::new());
        }
    };
    let tbl = load_tbl(&path)?;
    tracing::info!(
        rows = tbl.rows.len(),
        cols = tbl.columns.len(),
        "Quest_Monster_Exchange.tbl"
    );

    // Layout: 37 cols. col_0=quest_num, then 4 groups × (8 monster IDs + 1 count) = 36 cols
    let mut result = HashMap::with_capacity(tbl.rows.len());
    for row in &tbl.rows {
        if row.len() < 10 {
            continue;
        }
        let quest_num = cell_to_i32(&row[0]);
        let mut groups = Vec::new();
        // Each group: 8 monster ID slots + 1 count
        let group_size = 9;
        let max_groups = (row.len() - 1) / group_size;
        for g in 0..max_groups.min(4) {
            let base = 1 + g * group_size;
            if base + group_size > row.len() {
                break;
            }
            let mut monster_ids = Vec::new();
            for m in 0..8 {
                let mid = cell_to_i32(&row[base + m]);
                if mid != 0 {
                    monster_ids.push(mid);
                }
            }
            let count = cell_to_i32(&row[base + 8]);
            if !monster_ids.is_empty() && count > 0 {
                groups.push(MonsterGroup { monster_ids, count });
            }
        }
        if !groups.is_empty() {
            result.insert(quest_num, TblQuestMonsterExchange { quest_num, groups });
        }
    }
    Ok(result)
}
