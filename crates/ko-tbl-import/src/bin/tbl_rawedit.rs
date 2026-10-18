//! Byte-exact row patching for client .tbl files.
//!
//! Unlike `parser::serialize_tbl` (which writes a placeholder 5-byte prefix and re-encodes
//! strings), this tool keeps the 5-byte prefix and every string as RAW BYTES, so an
//! unmodified table serializes back to exactly the same plaintext, and (with the cipher
//! being deterministic) exactly the same encrypted file.
//!
//! Subcommands:
//!   roundtrip <file>...                 prove decrypt -> raw parse -> serialize -> encrypt == original
//!   show <file> <col> <value>...        print rows whose integer column equals a value
//!   orphans <Item_Org> <Skill_Magic_Main>  list items whose effect1 magic (c24) has no Main row
//!   kratos <Data dir> <out dir>         build the Kratos skill-bar fix (Item_Org_us, Skill_Magic_Main_us, Skill_Magic_4)
//!   piece <Data dir> <out dir> <rewards.txt>  build the Chaotic Generator boxes fix (piece_Exchange_us)

use anyhow::{bail, Context, Result};
use ko_tbl_import::decrypt;
use std::{collections::HashSet, env, fs, path::Path};

#[derive(Clone, Debug)]
enum Cell {
    Num(u8, Vec<u8>), // column type, raw little-endian bytes
    Str(Vec<u8>),     // raw string bytes (no length prefix)
}

struct Raw {
    prefix: Vec<u8>, // 5 bytes for ChaosExpansion tables
    types: Vec<i32>,
    rows: Vec<Vec<Cell>>,
}

fn size_of(t: i32) -> Result<usize> {
    Ok(match t {
        1 | 2 => 1,
        3 | 4 => 2,
        5 | 6 | 8 => 4,
        9 | 10 | 11 => 8,
        _ => bail!("bad column type {t}"),
    })
}

fn rd_i32(d: &[u8], o: &mut usize) -> Result<i32> {
    let b: [u8; 4] = d.get(*o..*o + 4).context("eof")?.try_into()?;
    *o += 4;
    Ok(i32::from_le_bytes(b))
}

fn parse(plain: &[u8], new_structure: bool) -> Result<Raw> {
    let mut o = 0usize;
    let prefix = if new_structure {
        o = 5;
        plain[..5].to_vec()
    } else {
        vec![]
    };
    let nc = rd_i32(plain, &mut o)? as usize;
    let mut types = vec![];
    for _ in 0..nc {
        types.push(rd_i32(plain, &mut o)?);
    }
    let nr = rd_i32(plain, &mut o)? as usize;
    let mut rows = Vec::with_capacity(nr);
    for _ in 0..nr {
        let mut row = Vec::with_capacity(nc);
        for &t in &types {
            if t == 7 {
                let n = rd_i32(plain, &mut o)?;
                let n = n.max(0) as usize;
                row.push(Cell::Str(plain.get(o..o + n).context("eof str")?.to_vec()));
                o += n;
            } else {
                let n = size_of(t)?;
                row.push(Cell::Num(t as u8, plain.get(o..o + n).context("eof num")?.to_vec()));
                o += n;
            }
        }
        rows.push(row);
    }
    if o != plain.len() {
        bail!("trailing bytes: parsed {o} of {}", plain.len());
    }
    Ok(Raw { prefix, types, rows })
}

fn serialize(t: &Raw) -> Vec<u8> {
    let mut b = t.prefix.clone();
    b.extend((t.types.len() as i32).to_le_bytes());
    for ty in &t.types {
        b.extend(ty.to_le_bytes());
    }
    b.extend((t.rows.len() as i32).to_le_bytes());
    for r in &t.rows {
        for c in r {
            match c {
                Cell::Num(_, v) => b.extend(v),
                Cell::Str(s) => {
                    b.extend((s.len() as i32).to_le_bytes());
                    b.extend(s);
                }
            }
        }
    }
    b
}

fn get_int(c: &Cell) -> i128 {
    match c {
        Cell::Num(t, v) => match t {
            1 => v[0] as i8 as i128,
            2 => v[0] as i128,
            3 => i16::from_le_bytes([v[0], v[1]]) as i128,
            4 => u16::from_le_bytes([v[0], v[1]]) as i128,
            5 => i32::from_le_bytes(v[..4].try_into().unwrap()) as i128,
            6 => u32::from_le_bytes(v[..4].try_into().unwrap()) as i128,
            8 => f32::from_le_bytes(v[..4].try_into().unwrap()) as i128,
            10 => i64::from_le_bytes(v[..8].try_into().unwrap()) as i128,
            11 => u64::from_le_bytes(v[..8].try_into().unwrap()) as i128,
            _ => 0,
        },
        Cell::Str(_) => 0,
    }
}

fn set_int(c: &mut Cell, val: i128) -> Result<()> {
    match c {
        Cell::Num(t, v) => {
            let ok = match t {
                1 => i8::try_from(val).map(|x| *v = vec![x as u8]).is_ok(),
                2 => u8::try_from(val).map(|x| *v = vec![x]).is_ok(),
                3 => i16::try_from(val).map(|x| *v = x.to_le_bytes().to_vec()).is_ok(),
                4 => u16::try_from(val).map(|x| *v = x.to_le_bytes().to_vec()).is_ok(),
                5 => i32::try_from(val).map(|x| *v = x.to_le_bytes().to_vec()).is_ok(),
                6 => u32::try_from(val).map(|x| *v = x.to_le_bytes().to_vec()).is_ok(),
                _ => false,
            };
            if !ok {
                bail!("value {val} does not fit column type {t}");
            }
            Ok(())
        }
        Cell::Str(_) => bail!("not numeric"),
    }
}

fn set_str(c: &mut Cell, s: &str) {
    *c = Cell::Str(s.as_bytes().to_vec());
}

fn str_of(c: &Cell) -> String {
    match c {
        Cell::Str(b) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    }
}

fn load(path: &Path) -> Result<(Raw, Vec<u8>, bool)> {
    let file = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let (plain, ns) = decrypt::decrypt_tbl(&file)?;
    Ok((parse(&plain, ns)?, file, ns))
}

fn save(t: &Raw, path: &Path) -> Result<()> {
    fs::write(path, decrypt::encrypt_tbl(&serialize(t)))?;
    Ok(())
}

fn fmt_row(r: &[Cell]) -> String {
    r.iter()
        .enumerate()
        .map(|(i, c)| match c {
            Cell::Str(_) => format!("c{i}={:?}", str_of(c)),
            Cell::Num(8, _) | Cell::Num(9, _) => format!("c{i}=<f>{:02x?}", if let Cell::Num(_, v) = c { v.clone() } else { vec![] }),
            _ => format!("c{i}={}", get_int(c)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_row(t: &Raw, num: i128) -> Option<usize> {
    t.rows.iter().position(|r| get_int(&r[0]) == num)
}

/// Insert a cloned row keeping the table sorted by column 0 (the client binary-searches/looks up by Num).
fn insert_sorted(t: &mut Raw, row: Vec<Cell>) {
    let key = get_int(&row[0]);
    let pos = t.rows.iter().position(|r| get_int(&r[0]) > key).unwrap_or(t.rows.len());
    t.rows.insert(pos, row);
}

fn main() -> Result<()> {
    let a: Vec<String> = env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("roundtrip") => {
            for f in &a[2..] {
                let file = fs::read(f)?;
                let (plain, ns) = decrypt::decrypt_tbl(&file)?;
                let t = parse(&plain, ns)?;
                let plain2 = serialize(&t);
                let enc2 = decrypt::encrypt_tbl(&plain2);
                // File may carry padding after the length-trimmed ciphertext; compare whole file.
                println!(
                    "{f}: rows={} plain_identical={} file_identical={} (file={} re-enc={})",
                    t.rows.len(),
                    plain == plain2,
                    file == enc2,
                    file.len(),
                    enc2.len()
                );
            }
        }
        Some("show") => {
            let (t, _, _) = load(Path::new(&a[2]))?;
            let col: usize = a[3].parse()?;
            let vals: Vec<i128> = a[4..].iter().map(|v| v.parse().unwrap()).collect();
            for (i, r) in t.rows.iter().enumerate() {
                if vals.contains(&get_int(&r[col])) {
                    println!("row={i} {}", fmt_row(r));
                }
            }
        }
        Some("orphans") => {
            let (items, _, _) = load(Path::new(&a[2]))?;
            let (main, _, _) = load(Path::new(&a[3]))?;
            let have: HashSet<i128> = main.rows.iter().map(|r| get_int(&r[0])).collect();
            for r in &items.rows {
                for col in [24usize] {
                    let m = get_int(&r[col]);
                    if m != 0 && !have.contains(&m) {
                        println!(
                            "item={} name={:?} kind={} slot={} item_type={} countable={} c{col}={m}",
                            get_int(&r[0]),
                            str_of(&r[2]),
                            get_int(&r[10]),
                            get_int(&r[12]),
                            get_int(&r[35]),
                            get_int(&r[23])
                        );
                    }
                }
            }
        }
        Some("piece") => piece(Path::new(&a[2]), Path::new(&a[3]), Path::new(&a[4]))?,
        Some("kratos") => kratos(Path::new(&a[2]), Path::new(&a[3]), a.get(4).map(String::as_str).unwrap_or("us"))?,
        _ => bail!("usage: tbl_rawedit roundtrip|show|orphans|kratos|piece ..."),
    }
    Ok(())
}

fn kratos(src: &Path, out: &Path, lang: &str) -> Result<()> {
    if lang != "us" && lang != "tk" {
        bail!("lang must be us or tk");
    }
    fs::create_dir_all(out)?;

    // ---- Item_Org_us.tbl ----
    let (mut items, _, _) = load(&src.join(format!("Item_Org_{lang}.tbl")))?;
    let k = find_row(&items, 357200000).context("item 357200000")?;
    let g = find_row(&items, 389026000).context("item 389026000")?;
    println!("ITEM before: {}", fmt_row(&items.rows[k]));
    for col in [4usize, 10, 12, 23, 35] {
        let v = get_int(&items.rows[g][col]);
        println!("  c{col}: {} -> {v}", get_int(&items.rows[k][col]));
        set_int(&mut items.rows[k][col], v)?;
    }
    println!("ITEM after : {}", fmt_row(&items.rows[k]));
    save(&items, &out.join(format!("Item_Org_{lang}.tbl")))?;

    // ---- Skill_Magic_Main_us.tbl ----
    let (mut main, _, _) = load(&src.join(format!("Skill_Magic_Main_{lang}.tbl")))?;
    if find_row(&main, 600127).is_some() {
        bail!("600127 already present in Main");
    }
    let g = find_row(&main, 490026).context("main 490026")?;
    let mut row = main.rows[g].clone();
    println!("MAIN template 490026: {}", fmt_row(&row));
    set_int(&mut row[0], 600127)?;
    set_str(&mut row[1], "Prayer of Kratos Power");
    if lang == "us" {
        set_str(&mut row[2], "Prayer Of Kratos Power");
        set_str(&mut row[3], "Supernatural power increases your attack power by 50%.");
    } else {
        // Turkish table: keep the (CP1254) description of the template row, only swap the title.
        let (b, _, _) = encoding_rs::WINDOWS_1254.encode("Kratos G\u{00fc}c\u{00fc} Duas\u{0131}");
        row[2] = Cell::Str(b.into_owned());
    }
    set_int(&mut row[12], 702)?; // server target_effect
    set_int(&mut row[21], 357200000)?; // use_item
    set_int(&mut row[34], 600127)?; // magic id back-reference
    println!("MAIN new 600127     : {}", fmt_row(&row));
    insert_sorted(&mut main, row);
    save(&main, &out.join(format!("Skill_Magic_Main_{lang}.tbl")))?;

    // ---- Skill_Magic_4.tbl (type-4 buff parameters; language independent, written once by the "us" run) ----
    if lang != "us" {
        return Ok(());
    }
    let (mut t4, _, _) = load(&src.join("Skill_Magic_4.tbl"))?;
    if find_row(&t4, 600127).is_some() {
        bail!("600127 already present in Skill_Magic_4");
    }
    let g = find_row(&t4, 490026).context("sm4 490026")?;
    let mut row = t4.rows[g].clone();
    println!("SM4 template 490026: {}", fmt_row(&row));
    set_int(&mut row[0], 600127)?;
    set_int(&mut row[3], 1800)?; // duration, same as server magic_type4 for 600127
    println!("SM4 new 600127     : {}", fmt_row(&row));
    insert_sorted(&mut t4, row);
    save(&t4, &out.join("Skill_Magic_4.tbl"))?;
    Ok(())
}

/// Chaotic Generator boxes: register 3 box items as pieces 28..30 in piece_Exchange_us.tbl.
///
/// Table layout (cols c0 key, c1 value, both u32):
///   key 0            -> c1 = piece count
///   key 1..count     -> piece item id accepted by Start
///   key 100+idx      -> c1 = N, number of reel-result rows of piece idx
///   key 10000*idx+j  -> reel display item id (j = 1..N)
/// `rewards.txt` lines: `<box item id>: <reward id> <reward id> ...` (candidate order); ids whose own or base (id/1000*1000) row is missing from
/// Item_Org_us.tbl are skipped and each box is capped at 10 reel results.
fn piece(src: &Path, out: &Path, rewards: &Path) -> Result<()> {
    const BOXES: [(u32, u32); 3] = [(28, 811053000), (29, 810341000), (30, 999915000)];
    const CAP: usize = 10;
    fs::create_dir_all(out)?;
    let (items, _, _) = load(&src.join("Item_Org_us.tbl"))?;
    let have: HashSet<i128> = items.rows.iter().map(|r| get_int(&r[0])).collect();
    let (mut t, _, _) = load(&src.join("piece_Exchange_us.tbl"))?;
    if t.types != vec![6, 6] {
        bail!("unexpected column types {:?}", t.types);
    }

    let mut cands: Vec<(u32, Vec<i128>)> = vec![];
    for line in fs::read_to_string(rewards)?.lines() {
        let Some((b, ids)) = line.split_once(':') else { continue };
        cands.push((b.trim().parse()?, ids.split_whitespace().map(|x| x.parse().unwrap()).collect()));
    }

    // key 0: piece count 27 -> 30
    let k0 = find_row(&t, 0).context("key 0")?;
    println!("key 0 c1: {} -> 30", get_int(&t.rows[k0][1]));
    if get_int(&t.rows[k0][1]) != 27 {
        bail!("key 0 expected 27 before edit");
    }
    set_int(&mut t.rows[k0][1], 30)?;

    // drop the bogus rows 270044..270046 (earlier wrong append)
    for k in [270044i128, 270045, 270046] {
        let i = find_row(&t, k).with_context(|| format!("bogus row {k} missing"))?;
        println!("delete {k} = {}", get_int(&t.rows[i][1]));
        t.rows.remove(i);
    }

    let template = t.rows[find_row(&t, 1).context("key 1")?].clone();
    let add = |t: &mut Raw, k: u32, v: u32| -> Result<()> {
        if find_row(t, k as i128).is_some() {
            bail!("key {k} already present");
        }
        let mut row = template.clone();
        set_int(&mut row[0], k as i128)?;
        set_int(&mut row[1], v as i128)?;
        insert_sorted(t, row);
        Ok(())
    };

    for (idx, box_id) in BOXES {
        add(&mut t, idx, box_id)?;
        println!("add {idx} = {box_id}");
        let list = &cands.iter().find(|(b, _)| *b == box_id).with_context(|| format!("no rewards for {box_id}"))?.1;
        let mut chosen: Vec<i128> = vec![];
        for &id in list {
            if chosen.contains(&id) {
                continue;
            }
            // Extended items (e.g. 1113367900) resolve through their base row (1113367000); every existing reel id does.
            if !have.contains(&id) && !have.contains(&(id / 1000 * 1000)) {
                println!("  skip reward {id}: not in Item_Org_us");
                continue;
            }
            chosen.push(id);
            if chosen.len() == CAP {
                break;
            }
        }
        if chosen.is_empty() {
            bail!("no valid rewards for box {box_id}");
        }
        add(&mut t, 100 + idx, chosen.len() as u32)?;
        println!("add {} = {}", 100 + idx, chosen.len());
        for (j, id) in chosen.iter().enumerate() {
            let k = 10000 * idx + j as u32 + 1;
            add(&mut t, k, *id as u32)?;
            println!("add {k} = {id}");
        }
    }
    // sanity: strictly increasing keys
    for w in t.rows.windows(2) {
        if get_int(&w[0][0]) >= get_int(&w[1][0]) {
            bail!("keys not strictly sorted at {}", get_int(&w[1][0]));
        }
    }
    save(&t, &out.join("piece_Exchange_us.tbl"))?;
    println!("rows now {}", t.rows.len());
    Ok(())
}
