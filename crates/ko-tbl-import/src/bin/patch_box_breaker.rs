//! Builds the client text patch of the Moradon "Box Breaker" NPC (proto 31514).
//!
//! usage: patch_box_breaker <client Data dir> <output dir> [--lang tr] [--rename-npc]
//!
//! * writes `Quest_Menu_us.tbl` and `Quest_Talk_us.tbl` with the texts 62101.. / 62201.. used by
//!   `Quests/31514_BoxBreaker.lua` (English by default, plain-ASCII Turkish with `--lang tr`);
//! * `--rename-npc` also writes `NPC_us.tbl`, `NPC_tk.tbl` and `NPC.tbl` where proto 31514 is
//!   renamed from "[Monster Strategist]" to "[Box Breaker]" (cosmetic, the model stays).
//!
//! Rows that already exist are replaced, new rows are appended (all new ids are above the current
//! maximum, so the id order of the tables is kept), so running it twice is harmless. The source
//! files are never modified.

use anyhow::{bail, Context, Result};
use ko_tbl_import::{
    box_breaker_texts as texts, decrypt,
    raw_tbl::{put_str, RawTbl, COL_STRING, COL_U32},
};
use std::{env, fs, path::Path};

const LUA: &str = include_str!("../../../../Quests/31514_BoxBreaker.lua");
const NPC_PROTO: u32 = 31_514;
const NPC_NAME: &str = "[Box Breaker]";

/// Loads a client table byte-preserving and proves the round trip: the untouched table must
/// re-encode to exactly the bytes of the source file.
fn load(path: &Path) -> Result<(RawTbl, bool)> {
    let raw = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let (plain, new_structure) = decrypt::decrypt_tbl(&raw)?;
    let table = RawTbl::parse(&plain, new_structure)?;
    if table.to_bytes() != plain {
        bail!("{}: raw plaintext round trip differs", path.display());
    }
    let again = decrypt::encrypt_tbl(&table.to_bytes());
    if again != raw {
        let first = again.iter().zip(raw.iter()).position(|(a, b)| a != b);
        bail!(
            "{}: re-encrypting the untouched table does not reproduce the source bytes (src {} / rebuilt {} bytes, first diff {:?})",
            path.display(),
            raw.len(),
            again.len(),
            first
        );
    }
    println!(
        "{}: byte-exact round trip OK ({} bytes, {} rows)",
        path.display(),
        raw.len(),
        table.rows.len()
    );
    Ok((table, new_structure))
}

fn save(path: &Path, table: &RawTbl, new_structure: bool) -> Result<()> {
    let plain = table.to_bytes();
    let enc = decrypt::encrypt_tbl(&plain);
    fs::write(path, &enc).with_context(|| format!("write {}", path.display()))?;
    // read the written file back: decrypts to the same plaintext and re-parses to the same rows
    let (back, ns) = decrypt::decrypt_tbl(&enc)?;
    if back != plain || ns != new_structure || RawTbl::parse(&back, ns)?.rows != table.rows {
        bail!("{}: written file does not read back identically", path.display());
    }
    Ok(())
}

fn rename_npc(source: &Path, output: &Path) -> Result<()> {
    let (mut table, new_structure) = load(source)?;
    if table.types.len() != 7 || table.types[0] != COL_U32 || table.types[1] != COL_STRING {
        bail!("unexpected NPC table schema in {}", source.display());
    }
    let mut hit = 0;
    for i in 0..table.rows.len() {
        if table.id(&table.rows[i]) == Some(NPC_PROTO) {
            let range = table.cell(&table.rows[i], 1)?;
            let mut name = Vec::new();
            put_str(&mut name, NPC_NAME);
            table.rows[i].splice(range, name);
            hit += 1;
        }
    }
    if hit == 0 {
        bail!("proto {NPC_PROTO} not found in {}", source.display());
    }
    save(output, &table, new_structure)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 2 {
        bail!("usage: patch_box_breaker <client Data dir> <output dir> [--lang tr] [--rename-npc]");
    }
    let (data, out) = (Path::new(&args[0]), Path::new(&args[1]));
    let turkish = args.windows(2).any(|w| w[0] == "--lang" && w[1] == "tr");
    let rename = args.iter().any(|a| a == "--rename-npc");
    fs::create_dir_all(out)?;

    let boxes = texts::parse_box_texts(LUA);
    if boxes.is_empty() {
        bail!("no BOX_TEXT entries found in the Lua script");
    }

    let (mut menu, ns) = load(&data.join("Quest_Menu_us.tbl"))?;
    texts::apply_menu(&mut menu, &boxes, turkish)?;
    save(&out.join("Quest_Menu_us.tbl"), &menu, ns)?;
    println!(
        "Quest_Menu_us.tbl: {} fixed + {} box texts ({} rows)",
        texts::MENU_TEXTS.len(),
        boxes.len(),
        menu.rows.len()
    );

    let (mut talk, ns) = load(&data.join("Quest_Talk_us.tbl"))?;
    texts::apply_talk(&mut talk, turkish)?;
    save(&out.join("Quest_Talk_us.tbl"), &talk, ns)?;
    println!(
        "Quest_Talk_us.tbl: {} header texts ({} rows)",
        texts::TALK_TEXTS.len(),
        talk.rows.len()
    );

    if rename {
        for file in ["NPC_us.tbl", "NPC_tk.tbl", "NPC.tbl"] {
            rename_npc(&data.join(file), &out.join(file))?;
            println!("{file}: proto {NPC_PROTO} renamed to {NPC_NAME}");
        }
    }
    Ok(())
}
