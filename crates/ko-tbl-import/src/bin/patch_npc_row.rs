//! Changes the model pid of one row of the client NPC_us.tbl.
//! Usage: patch_npc_row <client Data dir> <output dir> <proto> <new pid>

use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs, path::PathBuf};

fn main() -> Result<()> {
    let a: Vec<String> = env::args().collect();
    if a.len() < 5 {
        bail!("usage: patch_npc_row <Data dir> <out dir> <proto> <new pid>");
    }
    let (src, out) = (PathBuf::from(&a[1]), PathBuf::from(&a[2]));
    let proto: u32 = a[3].parse()?;
    let pid: u32 = a[4].parse()?;
    fs::create_dir_all(&out)?;
    let raw = fs::read(src.join("NPC_us.tbl")).context("read NPC_us.tbl")?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let mut table = parser::parse_tbl(&plain, new_structure)?;
    let mut hit = 0;
    for row in table.rows.iter_mut() {
        if matches!(row.first(), Some(CellValue::U32(id)) if *id == proto) {
            row[2] = CellValue::U32(pid);
            hit += 1;
        }
    }
    if hit == 0 {
        bail!("proto {proto} not found");
    }
    let plain = parser::serialize_tbl(&table, new_structure);
    fs::write(out.join("NPC_us.tbl"), decrypt::encrypt_tbl(&plain))?;
    println!("proto {proto}: pid -> {pid}");
    Ok(())
}
