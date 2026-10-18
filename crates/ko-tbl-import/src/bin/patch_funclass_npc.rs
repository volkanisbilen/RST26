//! Adds the Moradon "Fun Class Event" NPC (proto 31998) to the client NPC tables.
//!
//! The client validates NPC identity against NPC_us.tbl / NPC_tk.tbl / NPC.tbl, so a
//! server-only proto never renders. Usage:
//!   patch_funclass_npc <client Data dir> <output dir>
//! The rows of the source tables are kept; only proto 31998 is added or replaced.

use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs, path::PathBuf};

const FUNCLASS_ID: u32 = 31_998;
const FUNCLASS_PID: u32 = 30_500;

fn patch(source: &PathBuf, output: &PathBuf, name: &str) -> Result<()> {
    let raw = fs::read(source).with_context(|| format!("read {}", source.display()))?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let mut table = parser::parse_tbl(&plain, new_structure)?;
    if table.columns.len() != 7 {
        bail!("unexpected NPC table schema in {}", source.display());
    }
    table
        .rows
        .retain(|row| !matches!(row.first(), Some(CellValue::U32(id)) if *id == FUNCLASS_ID));
    table.rows.push(vec![
        CellValue::U32(FUNCLASS_ID),
        CellValue::Str(name.to_owned()),
        CellValue::U32(FUNCLASS_PID),
        // Same interaction fields as the working quest NPCs (see patch_daily_quest_manager_npc).
        CellValue::U32(200_004),
        CellValue::U32(200_005),
        CellValue::U32(0),
        CellValue::U32(0),
    ]);
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    let plain = parser::serialize_tbl(&table, new_structure);
    fs::write(output, decrypt::encrypt_tbl(&plain))
        .with_context(|| format!("write {}", output.display()))
}

fn main() -> Result<()> {
    let source_dir = PathBuf::from(env::args().nth(1).context("usage: patch_funclass_npc <Data dir> <out dir>")?);
    let output_dir = PathBuf::from(env::args().nth(2).context("usage: patch_funclass_npc <Data dir> <out dir>")?);
    fs::create_dir_all(&output_dir)?;
    for file in ["NPC_us.tbl", "NPC_tk.tbl", "NPC.tbl"] {
        patch(&source_dir.join(file), &output_dir.join(file), "[Fun Class Event] Cinderella")?;
        println!("patched {file}");
    }
    Ok(())
}
