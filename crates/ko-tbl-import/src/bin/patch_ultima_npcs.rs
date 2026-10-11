//! Adds the Ultima's Dungeon NPC protos to the client NPC table (NPC_us.tbl).
//! Usage: patch_ultima_npcs <client Data dir> <output dir>

use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs, path::PathBuf};

/// (proto, name, model pid)
const NPCS: &[(u32, &str, u32)] = &[
    (31990, "Ultima", 9888),
    (31991, "Ultima", 9888),
    (31992, "Ultima", 9888),
    (31993, "Seal of Ultima", 30030),
    (31994, "Safe Circle", 5301),
    (31997, "[Ultima's Dungeon] Keeper", 30500),
];

fn main() -> Result<()> {
    let src = PathBuf::from(env::args().nth(1).context("usage: patch_ultima_npcs <Data dir> <out dir>")?);
    let out = PathBuf::from(env::args().nth(2).context("usage: patch_ultima_npcs <Data dir> <out dir>")?);
    fs::create_dir_all(&out)?;
    let file = "NPC_us.tbl";
    let raw = fs::read(src.join(file)).with_context(|| format!("read {file}"))?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let mut table = parser::parse_tbl(&plain, new_structure)?;
    if table.columns.len() != 7 {
        bail!("unexpected NPC table schema");
    }
    let ids: Vec<u32> = NPCS.iter().map(|n| n.0).collect();
    table
        .rows
        .retain(|row| !matches!(row.first(), Some(CellValue::U32(id)) if ids.contains(id)));
    for &(id, name, pid) in NPCS {
        table.rows.push(vec![
            CellValue::U32(id),
            CellValue::Str(name.to_owned()),
            CellValue::U32(pid),
            CellValue::U32(200_004),
            CellValue::U32(200_005),
            CellValue::U32(0),
            CellValue::U32(0),
        ]);
    }
    table.rows.sort_by_key(|row| match row.first() {
        Some(CellValue::U32(id)) => *id,
        _ => u32::MAX,
    });
    let plain = parser::serialize_tbl(&table, new_structure);
    fs::write(out.join(file), decrypt::encrypt_tbl(&plain))?;
    println!("patched {file} ({} protos)", NPCS.len());
    Ok(())
}
