use anyhow::{Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("usage: tbl_dump <file> [first_row] [count]")?;
    let first: usize = env::args().nth(2).as_deref().unwrap_or("0").parse()?;
    let count: usize = env::args().nth(3).as_deref().unwrap_or("20").parse()?;
    let raw = fs::read(&path)?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let table = parser::parse_tbl(&plain, new_structure)?;
    println!(
        "file={} rows={} columns={:?}",
        Path::new(&path).display(),
        table.rows.len(),
        table.columns
    );
    for (row_index, row) in table.rows.iter().enumerate().skip(first).take(count) {
        let cells = row
            .iter()
            .enumerate()
            .map(|(i, cell)| match cell {
                CellValue::Str(value) => format!("c{i}={value:?}"),
                _ => format!("c{i}={}", cell.to_sql_literal()),
            })
            .collect::<Vec<_>>()
            .join(" | ");
        println!("row={row_index} | {cells}");
    }
    Ok(())
}
