//! Adds (or replaces) one text row of a client `Quest_Menu_*.tbl`.
//!
//! usage: patch_menu_text <source.tbl> <output.tbl> <text id> <text>

use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 4 {
        bail!("usage: patch_menu_text <source.tbl> <output.tbl> <text id> <text>");
    }
    let id: u32 = args[2].parse().context("text id")?;
    let raw = fs::read(&args[0]).with_context(|| format!("read {}", args[0]))?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let mut table = parser::parse_tbl(&plain, new_structure)?;
    if table.columns.len() != 2 {
        bail!("unexpected menu table schema: {} columns", table.columns.len());
    }
    let had = table
        .rows
        .iter()
        .any(|r| matches!(r.first(), Some(CellValue::U32(v)) if *v == id));
    table
        .rows
        .retain(|r| !matches!(r.first(), Some(CellValue::U32(v)) if *v == id));
    table
        .rows
        .push(vec![CellValue::U32(id), CellValue::Str(args[3].clone())]);
    let out = decrypt::encrypt_tbl(&parser::serialize_tbl(&table, new_structure));
    fs::write(&args[1], out).with_context(|| format!("write {}", args[1]))?;
    println!(
        "{} row {} -> {:?} ({} rows)",
        if had { "replaced" } else { "added" },
        id,
        args[3],
        table.rows.len()
    );
    Ok(())
}
