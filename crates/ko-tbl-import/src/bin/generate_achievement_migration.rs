use anyhow::{Context, Result};
use ko_tbl_import::{decrypt, parser};
use std::{env, fs, path::Path};

fn load(path: &Path) -> Result<parser::TblTable> {
    let raw = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    parser::parse_tbl(&plain, new_structure)
}

fn emit_table(
    sql: &mut String,
    table: &str,
    columns: &[&str],
    tbl: &parser::TblTable,
    source_columns: &[usize],
) {
    sql.push_str(&format!(
        "\nINSERT INTO {table} ({}) VALUES\n",
        columns.join(", ")
    ));
    for (row_no, row) in tbl.rows.iter().enumerate() {
        let values = source_columns
            .iter()
            .map(|&index| row[index].to_sql_literal())
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(&format!(
            "  ({values}){}\n",
            if row_no + 1 == tbl.rows.len() {
                ""
            } else {
                ","
            }
        ));
    }
    let updates = columns
        .iter()
        .skip(1)
        .map(|column| format!("{column} = EXCLUDED.{column}"))
        .collect::<Vec<_>>()
        .join(", ");
    sql.push_str(&format!(
        "ON CONFLICT ({}) DO UPDATE SET {updates};\n",
        columns[0]
    ));
    let ids = tbl
        .rows
        .iter()
        .map(|row| row[0].to_sql_literal())
        .collect::<Vec<_>>()
        .join(", ");
    sql.push_str(&format!(
        "DELETE FROM {table} WHERE {} NOT IN ({ids});\n",
        columns[0]
    ));
}

fn main() -> Result<()> {
    let data_dir = env::args()
        .nth(1)
        .context("usage: generate_achievement_migration <client Data> <output.sql>")?;
    let output = env::args().nth(2).context("missing output.sql")?;
    let root = Path::new(&data_dir);
    let main = load(&root.join("ACHIEVE_main.tbl"))?;
    let war = load(&root.join("ACHIEVE_war.tbl"))?;
    let normal = load(&root.join("ACHIEVE_normal.tbl"))?;
    let monster = load(&root.join("ACHIEVE_mon.tbl"))?;
    let com = load(&root.join("ACHIEVE_com.tbl"))?;
    let title = load(&root.join("ACHIEVE_title.tbl"))?;

    let mut sql = String::from(
        "-- Generated from the v2615 client ACHIEVE_*.tbl files. Do not hand-edit applied migrations.\n\
         -- Julia's v2615 NPC.tbl row is: 31741, pid 31200, behavior fields all zero.\n\
         UPDATE npc_template SET by_type = 0, i_selling_group = 0 WHERE s_sid = 31741;\n\
         UPDATE npc_spawn SET is_monster = FALSE WHERE zone_id = 21 AND npc_id = 31741;\n"
    );
    emit_table(
        &mut sql,
        "achieve_main",
        &[
            "s_index",
            "type",
            "title_id",
            "point",
            "item_num",
            "count",
            "zone_id",
            "unknown2",
            "achieve_type",
            "req_time",
            "byte1",
            "byte2",
        ],
        &main,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13],
    );
    emit_table(
        &mut sql,
        "achieve_war",
        &["s_index", "type", "s_count"],
        &war,
        &[0, 1, 2],
    );
    emit_table(
        &mut sql,
        "achieve_normal",
        &["s_index", "type", "count"],
        &normal,
        &[0, 1, 2],
    );
    emit_table(
        &mut sql,
        "achieve_monster",
        &[
            "s_index",
            "type",
            "byte",
            "monster1_1",
            "monster1_2",
            "monster1_3",
            "monster1_4",
            "mon_count1",
            "monster2_1",
            "monster2_2",
            "monster2_3",
            "monster2_4",
            "mon_count2",
        ],
        &monster,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    );
    emit_table(
        &mut sql,
        "achieve_com",
        &["s_index", "type", "req1", "req2"],
        &com,
        &[0, 1, 2, 3],
    );
    emit_table(
        &mut sql,
        "achieve_title",
        &[
            "s_index",
            "str",
            "hp",
            "dex",
            "\"int\"",
            "mp",
            "attack",
            "defence",
            "s_loyalty_bonus",
            "s_exp_bonus",
            "s_short_sword_ac",
            "s_jamadar_ac",
            "s_sword_ac",
            "s_blow_ac",
            "s_axe_ac",
            "s_spear_ac",
            "s_arrow_ac",
            "s_fire_bonus",
            "s_ice_bonus",
            "s_light_bonus",
            "s_fire_resist",
            "s_ice_resist",
            "s_light_resist",
            "s_magic_resist",
            "s_curse_resist",
            "s_poison_resist",
        ],
        &title,
        &[
            0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
            26, 27,
        ],
    );
    sql.push_str("\nDO $$ BEGIN\n  IF (SELECT COUNT(*) FROM achieve_main) <> 458 THEN RAISE EXCEPTION 'v2615 achieve_main sync failed'; END IF;\n  IF (SELECT COUNT(*) FROM achieve_normal) <> 47 THEN RAISE EXCEPTION 'v2615 achieve_normal sync failed'; END IF;\n  IF (SELECT COUNT(*) FROM achieve_com) <> 72 THEN RAISE EXCEPTION 'v2615 achieve_com sync failed'; END IF;\n  IF (SELECT COUNT(*) FROM achieve_title) <> 137 THEN RAISE EXCEPTION 'v2615 achieve_title sync failed'; END IF;\nEND $$;\n");
    fs::write(&output, sql).with_context(|| format!("write {output}"))?;
    println!(
        "generated {output}: main={} war={} normal={} monster={} com={} title={}",
        main.rows.len(),
        war.rows.len(),
        normal.rows.len(),
        monster.rows.len(),
        com.rows.len(),
        title.rows.len()
    );
    Ok(())
}
