use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    decrypt,
    parser::{self, CellValue, TblTable},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const SHOP_GROUP: u32 = 280_000;
const SHOP_ROW: u32 = 397;
const SHOP_PRICE: i32 = 50_000_000;
const ITEMS: [u32; 15] = [
    379_063_000,
    379_064_000,
    379_065_000,
    379_066_000,
    379_116_000,
    800_076_000,
    800_028_000,
    800_078_000,
    800_091_000,
    800_092_000,
    800_093_000,
    800_094_000,
    399_127_000,
    399_128_000,
    399_129_000,
];

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

fn patch_sell_table(source: &Path, output: &Path) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 26 {
        bail!(
            "unexpected itemsell_table schema: {} columns",
            table.columns.len()
        );
    }
    table
        .rows
        .retain(|row| !matches!(row.first(), Some(CellValue::U32(SHOP_ROW))));
    let mut row = vec![CellValue::U32(SHOP_ROW), CellValue::U32(SHOP_GROUP)];
    row.extend(ITEMS.into_iter().map(CellValue::U32));
    row.extend((0..9).map(|_| CellValue::U32(0)));
    table.rows.push(row);
    save(output, &table, new_structure)
}

fn patch_item_org(source: &Path, output: &Path) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.columns.len() != 41 {
        bail!(
            "unexpected Item_Org schema: {} columns",
            table.columns.len()
        );
    }
    let mut patched = 0usize;
    for row in &mut table.rows {
        let item_id = match row.first() {
            Some(CellValue::U32(value)) => *value,
            _ => continue,
        };
        if ITEMS.contains(&item_id) {
            row[20] = CellValue::I32(SHOP_PRICE);
            patched += 1;
        }
    }
    if patched != ITEMS.len() {
        bail!("patched {patched} of {} requested item prices", ITEMS.len());
    }
    save(output, &table, new_structure)
}

fn patch_quest_menu(source: &Path, output: &Path) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    table
        .rows
        .retain(|row| !matches!(row.first(), Some(CellValue::U32(60_001 | 60_002))));
    table.rows.push(vec![
        CellValue::U32(60_001),
        CellValue::Str("Exchange 50,000,000 EXP Jar".into()),
    ]);
    table.rows.push(vec![
        CellValue::U32(60_002),
        CellValue::Str("Exchange 100,000,000 EXP Jar".into()),
    ]);
    save(output, &table, new_structure)
}

fn main() -> Result<()> {
    let source_dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "docs/data".into()));
    let output_dir = PathBuf::from(
        env::args()
            .nth(2)
            .unwrap_or_else(|| "client_patch/moradon_scroll_shop".into()),
    );
    fs::create_dir_all(&output_dir)?;
    patch_sell_table(
        &source_dir.join("itemsell_table.tbl"),
        &output_dir.join("itemsell_table.tbl"),
    )?;
    patch_item_org(
        &source_dir.join("Item_Org_us.tbl"),
        &output_dir.join("Item_Org_us.tbl"),
    )?;
    patch_quest_menu(
        &source_dir.join("Quest_Menu_us.tbl"),
        &output_dir.join("Quest_Menu_us.tbl"),
    )?;
    println!("Generated client patch in {}", output_dir.display());
    Ok(())
}
