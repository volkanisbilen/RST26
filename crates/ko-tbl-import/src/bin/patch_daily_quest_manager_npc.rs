//! Generate the client table patch for the dedicated daily quest manager.
//!
//! The v2615 client validates NPC identity against NPC_us.tbl.  A server-only
//! proto therefore appears in the database but does not render in game.

use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue, TblTable},
};
use sqlx::Row;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

const DAILY_MANAGER_ID: u32 = 31_999;
const DAILY_MANAGER_PID: u32 = 30_500;
const SCROLL_MERCHANT_ID: u32 = 29_514;
const DAILY_MENU_HEADER: u32 = 60_010;
const DAILY_MENU_PREVIOUS: u32 = 60_021;
const DAILY_MENU_NEXT: u32 = 60_022;
const DAILY_QUEST_TEXT_BASE: u32 = 61_000;
// v2615's active Quest_Guide IDs end at 3593. IDs in the 30000 range are
// syntactically valid packets but are ignored by the Quest Tips UI.
const DAILY_QUEST_TIP_ID_BASE: u32 = 3_600;
// Normal v2615 quest-helper indices end at 20052. Keep the synthetic rows in
// the adjacent free range; very large helper indices are ignored by this UI.
const DAILY_QUEST_HELPER_INDEX_BASE: u32 = 21_000;

struct DailyQuestClientDefinition {
    id: i16,
    quest_name: Option<String>,
    mob_ids: [i32; 4],
    kill_count: i32,
    reward_ids: [i32; 4],
    reward_counts: [i32; 4],
    zone_id: i16,
    min_level: i16,
    max_level: i16,
}

fn load(path: &Path) -> Result<(TblTable, bool)> {
    let raw = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    Ok((parser::parse_tbl(&plain, new_structure)?, new_structure))
}

fn save(path: &Path, table: &TblTable, new_structure: bool) -> Result<()> {
    let plain = parser::serialize_tbl(table, new_structure);
    fs::write(path, decrypt::encrypt_tbl(&plain))
        .with_context(|| format!("write {}", path.display()))
}

fn patch_table(source: &Path, output: &Path, display_name: &str) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 7 {
        bail!("unexpected NPC table schema in {}", source.display());
    }

    // 29514 must already exist in the v2615 NPC_us.tbl. Fail loudly if the
    // user supplied a different client instead of silently producing a patch
    // for the wrong data set.
    let scroll_present = table
        .rows
        .iter()
        .any(|row| matches!(row.first(), Some(CellValue::U32(id)) if *id == SCROLL_MERCHANT_ID));
    if !scroll_present {
        bail!(
            "{} is missing the required Scroll Merchant proto 29514",
            source.display()
        );
    }

    table
        .rows
        .retain(|row| !matches!(row.first(), Some(CellValue::U32(id)) if *id == DAILY_MANAGER_ID));
    table.rows.push(vec![
        CellValue::U32(DAILY_MANAGER_ID),
        CellValue::Str(display_name.to_owned()),
        CellValue::U32(DAILY_MANAGER_PID),
        // Same client interaction fields as the working [Manager] Billbor
        // quest NPC (18005). Without them the NPC model renders but the
        // v2615 client does not create a right-click dialogue route.
        CellValue::U32(200_004),
        CellValue::U32(200_005),
        CellValue::U32(0),
        CellValue::U32(0),
    ]);
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    save(output, &table, new_structure)
}

async fn daily_quest_definitions(
    database_url: &str,
) -> Result<HashMap<u32, DailyQuestClientDefinition>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("connect to PostgreSQL daily_quests source")?;
    let rows = sqlx::query(
        r#"
        SELECT id, quest_name,
               mob_id_1, mob_id_2, mob_id_3, mob_id_4, kill_count,
               reward_1, reward_2, reward_3, reward_4,
               count_1, count_2, count_3, count_4,
               zone_id, min_level, max_level
        FROM daily_quests
        ORDER BY id
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("read PostgreSQL daily_quests")?;
    pool.close().await;

    let mut definitions = HashMap::new();
    for row in rows {
        definitions.insert(
            row.try_get::<i16, _>("id")? as u32,
            DailyQuestClientDefinition {
                id: row.try_get("id")?,
                quest_name: row.try_get("quest_name")?,
                mob_ids: [
                    row.try_get("mob_id_1")?,
                    row.try_get("mob_id_2")?,
                    row.try_get("mob_id_3")?,
                    row.try_get("mob_id_4")?,
                ],
                kill_count: row.try_get("kill_count")?,
                reward_ids: [
                    row.try_get("reward_1")?,
                    row.try_get("reward_2")?,
                    row.try_get("reward_3")?,
                    row.try_get("reward_4")?,
                ],
                reward_counts: [
                    row.try_get("count_1")?,
                    row.try_get("count_2")?,
                    row.try_get("count_3")?,
                    row.try_get("count_4")?,
                ],
                zone_id: row.try_get("zone_id")?,
                min_level: row.try_get("min_level")?,
                max_level: row.try_get("max_level")?,
            },
        );
    }
    if definitions.len() != 200 || !(1..=200).all(|id| definitions.contains_key(&id)) {
        bail!(
            "PostgreSQL daily_quests must contain IDs 1..200 exactly; found {} rows",
            definitions.len()
        );
    }
    Ok(definitions)
}

fn daily_quest_names(definitions: &HashMap<u32, DailyQuestClientDefinition>) -> Vec<(u32, String)> {
    let mut names = definitions
        .iter()
        .map(|(id, definition)| {
            (
                *id,
                definition
                    .quest_name
                    .clone()
                    .unwrap_or_else(|| format!("Quest {id}")),
            )
        })
        .collect::<Vec<_>>();
    names.sort_by_key(|(id, _)| *id);
    names
}

fn patch_menu(source: &Path, output: &Path, quest_names: &[(u32, String)]) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 2 {
        bail!("unexpected Quest_Menu schema in {}", source.display());
    }
    table.rows.retain(|row| {
        !matches!(row.first(), Some(CellValue::U32(id)) if (*id >= DAILY_MENU_HEADER && *id <= DAILY_MENU_NEXT) || (*id > DAILY_QUEST_TEXT_BASE && *id <= DAILY_QUEST_TEXT_BASE + 200))
    });
    table.rows.push(vec![
        CellValue::U32(DAILY_MENU_HEADER),
        CellValue::Str("Daily Quest Manager".into()),
    ]);
    for (id, name) in quest_names {
        table.rows.push(vec![
            CellValue::U32(DAILY_QUEST_TEXT_BASE + id),
            CellValue::Str(format!("Daily Quest - {name}")),
        ]);
    }
    table.rows.push(vec![
        CellValue::U32(DAILY_MENU_PREVIOUS),
        CellValue::Str("Previous Page".into()),
    ]);
    table.rows.push(vec![
        CellValue::U32(DAILY_MENU_NEXT),
        CellValue::Str("Next Page".into()),
    ]);
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    save(output, &table, new_structure)
}

fn patch_talk(source: &Path, output: &Path, quest_names: &[(u32, String)]) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 4 {
        bail!("unexpected Quest_Talk schema in {}", source.display());
    }
    table.rows.retain(|row| {
        !matches!(row.first(), Some(CellValue::U32(id)) if (*id >= DAILY_MENU_HEADER && *id <= DAILY_MENU_NEXT) || (*id > DAILY_QUEST_TEXT_BASE && *id <= DAILY_QUEST_TEXT_BASE + 200))
    });
    table.rows.push(vec![
        CellValue::U32(DAILY_MENU_HEADER),
        CellValue::Str("Select a daily quest from the numbered list.".into()),
        CellValue::U32(0),
        CellValue::U32(0),
    ]);
    for (id, name) in quest_names {
        table.rows.push(vec![
            CellValue::U32(DAILY_QUEST_TEXT_BASE + id),
            CellValue::Str(format!("Daily Quest - {name}")),
            CellValue::U32(0),
            CellValue::U32(0),
        ]);
    }
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    save(output, &table, new_structure)
}

/// Add display-only rows used by the right-side Quest Tips widget. The server
/// mirrors selected daily quests to IDs 30001..30200; these rows make the
/// v2615 client render their names instead of treating them as unknown quests.
fn patch_quest_guide(source: &Path, output: &Path, quest_names: &[(u32, String)]) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 7 {
        bail!("unexpected Quest_Guide schema in {}", source.display());
    }
    table.rows.retain(|row| {
        !matches!(row.first(), Some(CellValue::U32(id)) if *id > DAILY_QUEST_TIP_ID_BASE && *id <= DAILY_QUEST_TIP_ID_BASE + 200)
    });
    for (id, name) in quest_names {
        let title = format!("Daily Quest - {name}");
        let description = format!("Daily Quest in progress: {name}");
        table.rows.push(vec![
            CellValue::U32(DAILY_QUEST_TIP_ID_BASE + id),
            CellValue::U8(0),
            CellValue::I32(1),
            CellValue::I32(100),
            CellValue::Str(title),
            CellValue::Str(description.clone()),
            CellValue::Str(description),
        ]);
    }
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    save(output, &table, new_structure)
}

/// Quest Tips resolves a listed quest through the non-localized
/// `Data\\quest_helper.tbl` before consulting Quest_Guide. The v2615
/// executable names that file explicitly; Quest_Helper_us.tbl is a different
/// 20-column table and is not the source used by this UI.
fn patch_quest_helper(source: &Path, output: &Path, quest_names: &[(u32, String)]) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 21 {
        bail!("unexpected Quest_Helper schema in {}", source.display());
    }
    table.rows.retain(|row| {
        !matches!(row.get(11), Some(CellValue::I32(id)) if *id > DAILY_QUEST_TIP_ID_BASE as i32 && *id <= (DAILY_QUEST_TIP_ID_BASE + 200) as i32)
    });
    for (id, _) in quest_names {
        let tip_id = DAILY_QUEST_TIP_ID_BASE + id;
        table.rows.push(vec![
            CellValue::U32(DAILY_QUEST_HELPER_INDEX_BASE + id),
            CellValue::U8(1),  // message type
            CellValue::U8(1),  // minimum level
            CellValue::I64(0), // exp
            CellValue::U32(0), // unused/client class field
            CellValue::I32(0), // v2615 helper reserved field
            CellValue::U8(5),  // any class
            CellValue::U8(3),  // either nation
            CellValue::U8(1),  // normal quest display type
            CellValue::U8(0),  // any zone
            CellValue::I32(0), // no NPC
            CellValue::I32(tip_id as i32),
            CellValue::U8(1),  // ongoing state
            CellValue::I32(0), // no trigger
            CellValue::I32(0), // no completion trigger
            CellValue::U32(0),
            CellValue::U32(0),
            CellValue::Str("daily_quest_manager.lua".into()),
            CellValue::I32(0),
            CellValue::I32(0),
            CellValue::U8(0),
        ]);
    }
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    save(output, &table, new_structure)
}

/// Quest Tips only creates a tracked monster objective when the listed quest
/// has a Quest_Monster_Exchange row. Daily quests use one shared counter for
/// up to four accepted monster prototypes, which maps to the first group.
fn patch_quest_monster_exchange(
    source: &Path,
    output: &Path,
    quest_names: &[(u32, String)],
    definitions: &HashMap<u32, DailyQuestClientDefinition>,
) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 37 {
        bail!(
            "unexpected Quest_Monster_Exchange schema in {}",
            source.display()
        );
    }

    table.rows.retain(|row| {
        !matches!(row.first(), Some(CellValue::U32(id)) if *id > DAILY_QUEST_TIP_ID_BASE && *id <= DAILY_QUEST_TIP_ID_BASE + 200)
    });

    for (id, _) in quest_names {
        let definition = definitions
            .get(id)
            .with_context(|| format!("daily quest definition {id} not found"))?;
        let mut row = vec![CellValue::I32(0); 37];
        row[0] = CellValue::U32(DAILY_QUEST_TIP_ID_BASE + id);
        row[1] = CellValue::I32(definition.mob_ids[0]);
        row[3] = CellValue::I32(definition.mob_ids[1]);
        row[5] = CellValue::I32(definition.mob_ids[2]);
        row[7] = CellValue::I32(definition.mob_ids[3]);
        row[9] = CellValue::I32(definition.kill_count);
        table.rows.push(row);
    }

    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    save(output, &table, new_structure)
}

fn write_daily_quest_snapshot(
    output: &Path,
    definitions: &HashMap<u32, DailyQuestClientDefinition>,
) -> Result<()> {
    let mut ids = definitions.keys().copied().collect::<Vec<_>>();
    ids.sort_unstable();
    let mut snapshot = String::from(
        "id\tname\tmob_1\tmob_2\tmob_3\tmob_4\tkill_count\treward_1\tcount_1\treward_2\tcount_2\treward_3\tcount_3\treward_4\tcount_4\tzone\tmin_level\tmax_level\n",
    );
    for id in ids {
        let definition = &definitions[&id];
        snapshot.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            definition.id,
            definition.quest_name.as_deref().unwrap_or(""),
            definition.mob_ids[0],
            definition.mob_ids[1],
            definition.mob_ids[2],
            definition.mob_ids[3],
            definition.kill_count,
            definition.reward_ids[0],
            definition.reward_counts[0],
            definition.reward_ids[1],
            definition.reward_counts[1],
            definition.reward_ids[2],
            definition.reward_counts[2],
            definition.reward_ids[3],
            definition.reward_counts[3],
            definition.zone_id,
            definition.min_level,
            definition.max_level,
        ));
    }
    fs::write(output, snapshot).with_context(|| format!("write {}", output.display()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let source_dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "docs/data".into()));
    let output_dir = PathBuf::from(
        env::args()
            .nth(2)
            .unwrap_or_else(|| "client_patch/daily_quest_manager".into()),
    );
    fs::create_dir_all(&output_dir)?;
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://koserver:koserver123@localhost:5432/ko_server".into());
    let definitions = daily_quest_definitions(&database_url).await?;
    let quest_names = daily_quest_names(&definitions);
    write_daily_quest_snapshot(
        &output_dir.join("daily_quests_postgresql_snapshot.tsv"),
        &definitions,
    )?;

    patch_table(
        &source_dir.join("NPC_us.tbl"),
        &output_dir.join("NPC_us.tbl"),
        "[Daily Quest Manager] Aelion",
    )?;
    patch_menu(
        &source_dir.join("Quest_Menu_us.tbl"),
        &output_dir.join("Quest_Menu_us.tbl"),
        &quest_names,
    )?;
    patch_talk(
        &source_dir.join("Quest_Talk_us.tbl"),
        &output_dir.join("Quest_Talk_us.tbl"),
        &quest_names,
    )?;
    patch_quest_guide(
        &source_dir.join("Quest_Guide_us.tbl"),
        &output_dir.join("Quest_Guide_us.tbl"),
        &quest_names,
    )?;
    patch_quest_helper(
        &source_dir.join("Quest_Helper.tbl"),
        &output_dir.join("Quest_Helper.tbl"),
        &quest_names,
    )?;
    patch_quest_monster_exchange(
        &source_dir.join("Quest_Monster_Exchange.tbl"),
        &output_dir.join("Quest_Monster_Exchange.tbl"),
        &quest_names,
        &definitions,
    )?;
    patch_table(
        &source_dir.join("NPC_tk.tbl"),
        &output_dir.join("NPC_tk.tbl"),
        "[Gunluk Gorev Yoneticisi] Aelion",
    )?;
    patch_table(
        &source_dir.join("NPC.tbl"),
        &output_dir.join("NPC.tbl"),
        "[Daily Quest Manager] Aelion",
    )?;
    println!(
        "Generated client NPC patch from {} PostgreSQL daily quests in {}",
        definitions.len(),
        output_dir.display()
    );
    Ok(())
}
