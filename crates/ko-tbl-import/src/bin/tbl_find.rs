use anyhow::{Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("usage: tbl_find <file> <column> <integer>...")?;
    let column: usize = env::args().nth(2).context("missing column")?.parse()?;
    let values: Vec<i128> = env::args()
        .skip(3)
        .map(|v| v.parse())
        .collect::<Result<_, _>>()?;
    let raw = fs::read(&path)?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let table = parser::parse_tbl(&plain, new_structure)?;
    println!("rows={} columns={:?}", table.rows.len(), table.columns);
    for (row_index, row) in table.rows.iter().enumerate() {
        let Some(value) = row.get(column).and_then(as_i128) else {
            continue;
        };
        if !values.contains(&value) {
            continue;
        }
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

fn as_i128(value: &CellValue) -> Option<i128> {
    Some(match value {
        CellValue::I8(v) => *v as i128,
        CellValue::U8(v) => *v as i128,
        CellValue::I16(v) => *v as i128,
        CellValue::U16(v) => *v as i128,
        CellValue::I32(v) => *v as i128,
        CellValue::U32(v) => *v as i128,
        CellValue::I64(v) => *v as i128,
        CellValue::U64(v) => *v as i128,
        _ => return None,
    })
}
