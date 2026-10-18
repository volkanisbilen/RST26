use anyhow::{Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue},
};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    let root = env::args()
        .nth(1)
        .context("usage: tbl_text_search <directory> <term>...")?;
    let terms: Vec<String> = env::args().skip(2).map(|s| s.to_lowercase()).collect();
    anyhow::ensure!(!terms.is_empty(), "at least one search term is required");

    let mut entries: Vec<_> = fs::read_dir(&root)?.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map_or(true, |s| !s.eq_ignore_ascii_case("tbl"))
        {
            continue;
        }
        let name = file_name(&path).to_lowercase();
        if ![
            "quest", "seed", "help", "guide", "npc", "mob", "mon", "text", "caption",
        ]
        .iter()
        .any(|needle| name.contains(needle))
        {
            continue;
        }
        let raw = fs::read(&path)?;
        let Ok((plain, new_structure)) = decrypt::decrypt_tbl(&raw) else {
            continue;
        };
        let Ok(table) = parser::parse_tbl(&plain, new_structure) else {
            continue;
        };
        for (row_index, row) in table.rows.iter().enumerate() {
            let matched = row.iter().any(|cell| match cell {
                CellValue::Str(value) => {
                    let lower = value.to_lowercase();
                    terms.iter().any(|term| lower.contains(term))
                }
                _ => false,
            });
            if matched {
                let cells = row
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| match cell {
                        CellValue::Str(value) => format!("c{i}={value:?}"),
                        _ => format!("c{i}={}", cell.to_sql_literal()),
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                println!("{} row={} | {}", file_name(&path), row_index, cells);
            }
        }
    }
    Ok(())
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
}
