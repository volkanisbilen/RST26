//! Client texts of the Moradon "Box Breaker" NPC (`Quests/31514_BoxBreaker.lua`).
//!
//! The NPC dialog is a normal `SelectMsg`, so every button / header is a text id of the client's
//! `Quest_Menu_us.tbl` (buttons) and `Quest_Talk_us.tbl` (headers). The ids below are free in the
//! v2625 client (highest existing menu id is 60010) and in the live `quest_menu` / `quest_talk`
//! tables. The names of the box buttons are read from the `BOX_TEXT` table of the Lua script, so
//! the script stays the single source of truth for which box has which text.
//!
//! Builder: `cargo run -p ko-tbl-import --bin patch_box_breaker -- <client Data dir> <out dir>`.

use crate::parser::{CellValue, ColumnType, TblTable};

/// One client text in English and in plain-ASCII Turkish (no diacritics: safe in every codepage).
pub struct Text {
    pub id: u32,
    pub en: &'static str,
    pub tr: &'static str,
}

/// Quest_Menu_us.tbl button texts of the fixed UI (the box names are added from the Lua table).
pub const MENU_TEXTS: &[Text] = &[
    Text { id: 62101, en: "Break 1", tr: "1 adet kir" },
    Text { id: 62102, en: "Break 10", tr: "10 adet kir" },
    Text { id: 62103, en: "Break all of this kind", tr: "Bu turun hepsini kir" },
    Text { id: 62104, en: "Break all my boxes", tr: "Tum kutularimi kir" },
    Text { id: 62105, en: "Previous page", tr: "Onceki sayfa" },
    Text { id: 62106, en: "Next page", tr: "Sonraki sayfa" },
    Text { id: 62107, en: "Back to the box list", tr: "Kutu listesine don" },
    Text { id: 62108, en: "Other box (name in chat)", tr: "Diger kutu (adi sohbette)" },
    Text { id: 62109, en: "Yes, break everything", tr: "Evet, hepsini kir" },
    Text { id: 62110, en: "No, go back", tr: "Hayir, geri don" },
];

/// Quest_Talk_us.tbl header texts.
pub const TALK_TEXTS: &[Text] = &[
    Text {
        id: 62101,
        en: "Welcome! I break boxes, chests, gems and fragments for you - the same rewards as the Chaotic Generator, free of charge. Pick one below; your amounts are listed in the chat window.",
        tr: "Hos geldin! Kutulari, sandiklari, taslari ve parcalari senin icin kiriyorum - Chaotic Generator ile ayni oduller, ucretsiz. Asagidan birini sec; adetlerin sohbet penceresinde yazili.",
    },
    Text {
        id: 62102,
        en: "You carry nothing I can break. Bring boxes, chests, gems or fragments and come back.",
        tr: "Kirabilecegim hicbir sey tasimiyorsun. Kutu, sandik, tas veya parca getirip tekrar gel.",
    },
    Text {
        id: 62103,
        en: "How many do you want to break? The chosen kind and your amount are in the chat window.",
        tr: "Kac tane kirmak istiyorsun? Secilen tur ve adetin sohbet penceresinde yazili.",
    },
    Text {
        id: 62104,
        en: "This breaks everything breakable in your bag - boxes, chests, gems and fragments - up to 200 pieces at a time. This cannot be undone. Are you sure?",
        tr: "Cantandaki kirilabilen her seyi kirar - kutular, sandiklar, taslar ve parcalar - tek seferde en fazla 200 adet. Geri alinamaz. Emin misin?",
    },
    Text {
        id: 62105,
        en: "Done! The rewards are in your bag. A summary is in the chat window.",
        tr: "Tamam! Oduller cantanda. Ozet sohbet penceresinde.",
    },
    Text {
        id: 62106,
        en: "I stopped after 200 pieces. There is more left - talk to me again to continue. A summary is in the chat window.",
        tr: "200 adetten sonra durdum. Devami var - devam etmek icin tekrar konus. Ozet sohbet penceresinde.",
    },
    Text {
        id: 62107,
        en: "Your bag is full. Free some slots and talk to me again. What was already broken is listed in the chat window.",
        tr: "Cantan dolu. Biraz yer ac ve tekrar konus. Kirilanlar sohbet penceresinde yazili.",
    },
    Text {
        id: 62108,
        en: "You carry too much weight. Drop something and try again. What was already broken is listed in the chat window.",
        tr: "Cok fazla agirlik tasiyorsun. Bir seyler birakip tekrar dene. Kirilanlar sohbet penceresinde yazili.",
    },
    Text {
        id: 62109,
        en: "I can't do that right now. Stay next to me and make sure you still own the box.",
        tr: "Su an bunu yapamam. Yanimda dur ve kutuyu hala tasidigindan emin ol.",
    },
];

/// Extra `Quest_Talk_us.tbl` columns (voice / emote) copied from the generic "What would you like
/// to trade?" row (4035).
pub const TALK_EXTRA: (u32, u32) = (1002, 2000);

/// One box button text read from the Lua script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxText {
    pub item_id: u32,
    pub text_id: u32,
    pub name: String,
}

/// Reads the `BOX_TEXT` table of the Lua script: lines like
/// `[379154000] = 62201, -- Red Treasure Chest`.
pub fn parse_box_texts(lua: &str) -> Vec<BoxText> {
    let mut out = Vec::new();
    for line in lua.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('[') else { continue };
        let Some((item, rest)) = rest.split_once(']') else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('=') else { continue };
        let Some((text, name)) = rest.split_once(',') else { continue };
        let (Ok(item_id), Ok(text_id)) = (item.trim().parse::<u32>(), text.trim().parse::<u32>())
        else {
            continue;
        };
        let name = name.trim().trim_start_matches("--").trim().to_string();
        if !name.is_empty() {
            out.push(BoxText { item_id, text_id, name });
        }
    }
    out
}

/// Reads `local NAME = 12345;` constants whose name starts with `prefix` (e.g. `M_`, `T_`).
pub fn parse_text_constants(lua: &str, prefix: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for line in lua.lines() {
        let Some(rest) = line.trim().strip_prefix("local ") else { continue };
        let Some((name, value)) = rest.split_once('=') else { continue };
        let name = name.trim();
        if !name.starts_with(prefix) {
            continue;
        }
        let digits: String = value
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(v) = digits.parse::<u32>() {
            out.push((name.to_string(), v));
        }
    }
    out
}

fn first_id(row: &[CellValue]) -> Option<u32> {
    match row.first() {
        Some(CellValue::U32(v)) => Some(*v),
        _ => None,
    }
}

/// Adds / replaces the button texts in a `Quest_Menu_*.tbl` table (`[UnsignedInt, String]`).
pub fn apply_menu(table: &mut TblTable, boxes: &[BoxText], turkish: bool) -> anyhow::Result<()> {
    if table.columns != [ColumnType::UnsignedInt, ColumnType::String] {
        anyhow::bail!("unexpected Quest_Menu schema: {:?}", table.columns);
    }
    let mut rows: Vec<(u32, String)> = MENU_TEXTS
        .iter()
        .map(|t| (t.id, (if turkish { t.tr } else { t.en }).to_string()))
        .collect();
    rows.extend(boxes.iter().map(|b| (b.text_id, b.name.clone())));
    for (id, text) in rows {
        table.rows.retain(|r| first_id(r) != Some(id));
        table.rows.push(vec![CellValue::U32(id), CellValue::Str(text)]);
    }
    Ok(())
}

/// Adds / replaces the header texts in a `Quest_Talk_*.tbl` table
/// (`[UnsignedInt, String, UnsignedInt, UnsignedInt]`).
pub fn apply_talk(table: &mut TblTable, turkish: bool) -> anyhow::Result<()> {
    if table.columns
        != [
            ColumnType::UnsignedInt,
            ColumnType::String,
            ColumnType::UnsignedInt,
            ColumnType::UnsignedInt,
        ]
    {
        anyhow::bail!("unexpected Quest_Talk schema: {:?}", table.columns);
    }
    for t in TALK_TEXTS {
        table.rows.retain(|r| first_id(r) != Some(t.id));
        table.rows.push(vec![
            CellValue::U32(t.id),
            CellValue::Str((if turkish { t.tr } else { t.en }).to_string()),
            CellValue::U32(TALK_EXTRA.0),
            CellValue::U32(TALK_EXTRA.1),
        ]);
    }
    Ok(())
}

/// Sorts rows by their first (id) column like the original client tables.
pub fn sort_by_id(table: &mut TblTable) {
    table.rows.sort_by_key(|r| first_id(r).unwrap_or(u32::MAX));
}

#[cfg(test)]
mod tests {
    use super::*;

    const LUA: &str = include_str!("../../../Quests/31514_BoxBreaker.lua");
    const MIGRATION: &str =
        include_str!("../../../migrations/20261018120000_box_breaker_npc.sql");

    #[test]
    fn lua_box_table_is_parsed() {
        let boxes = parse_box_texts(LUA);
        assert!(boxes.len() >= 30, "BOX_TEXT parsed: {}", boxes.len());
        // distinct item ids and text ids, all in the reserved 622xx range
        let mut items: Vec<_> = boxes.iter().map(|b| b.item_id).collect();
        items.sort_unstable();
        items.dedup();
        assert_eq!(items.len(), boxes.len(), "duplicate item id in BOX_TEXT");
        let mut texts: Vec<_> = boxes.iter().map(|b| b.text_id).collect();
        texts.sort_unstable();
        texts.dedup();
        assert_eq!(texts.len(), boxes.len(), "duplicate text id in BOX_TEXT");
        for b in &boxes {
            assert!((62201..62300).contains(&b.text_id), "{b:?}");
            assert!(!b.name.is_empty());
            assert!(b.name.len() < 100, "menu text limit is 100 chars: {b:?}");
        }
    }

    #[test]
    fn lua_text_constants_exist_in_the_patch() {
        let menu: Vec<u32> = MENU_TEXTS.iter().map(|t| t.id).collect();
        let talk: Vec<u32> = TALK_TEXTS.iter().map(|t| t.id).collect();
        for (name, id) in parse_text_constants(LUA, "M_") {
            // M_CLOSE is the client's own "Close" text (27), everything else is ours.
            if id == 27 {
                continue;
            }
            assert!(menu.contains(&id), "{name}={id} is not in MENU_TEXTS");
        }
        for (name, id) in parse_text_constants(LUA, "T_") {
            assert!(talk.contains(&id), "{name}={id} is not in TALK_TEXTS");
        }
        // every fixed text is actually used by the script
        let used: Vec<u32> = parse_text_constants(LUA, "M_").into_iter().map(|(_, v)| v).collect();
        for id in &menu {
            assert!(used.contains(id), "menu text {id} unused by the Lua script");
        }
        let used: Vec<u32> = parse_text_constants(LUA, "T_").into_iter().map(|(_, v)| v).collect();
        for id in &talk {
            assert!(used.contains(id), "talk text {id} unused by the Lua script");
        }
    }

    #[test]
    fn text_ids_are_unique_and_short() {
        for list in [MENU_TEXTS, TALK_TEXTS] {
            let mut ids: Vec<_> = list.iter().map(|t| t.id).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), list.len());
        }
        for t in MENU_TEXTS {
            assert!(t.en.len() < 100 && t.tr.len() < 100, "{}", t.id);
        }
        for t in TALK_TEXTS {
            assert!(t.en.len() < 1000 && t.tr.len() < 1000, "{}", t.id);
            assert!(t.tr.is_ascii(), "Turkish talk text must be plain ASCII: {}", t.id);
        }
        for t in MENU_TEXTS {
            assert!(t.tr.is_ascii(), "Turkish menu text must be plain ASCII: {}", t.id);
        }
    }

    fn sql_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    #[test]
    fn migration_carries_the_same_texts() {
        for t in MENU_TEXTS {
            assert!(
                MIGRATION.contains(&format!("({}, {})", t.id, sql_quote(t.en))),
                "migration is missing menu text {} {:?}",
                t.id,
                t.en
            );
        }
        for t in TALK_TEXTS {
            assert!(
                MIGRATION.contains(&format!("({}, {})", t.id, sql_quote(t.en))),
                "migration is missing talk text {} {:?}",
                t.id,
                t.en
            );
        }
        for b in parse_box_texts(LUA) {
            assert!(
                MIGRATION.contains(&format!("({}, {})", b.text_id, sql_quote(&b.name))),
                "migration is missing box text {b:?}"
            );
        }
    }

    #[test]
    fn apply_adds_and_replaces_rows() {
        let mut menu = TblTable {
            columns: vec![ColumnType::UnsignedInt, ColumnType::String],
            rows: vec![
                vec![CellValue::U32(27), CellValue::Str("Close".into())],
                vec![CellValue::U32(62101), CellValue::Str("old".into())],
            ],
        };
        let boxes = parse_box_texts(LUA);
        apply_menu(&mut menu, &boxes, false).unwrap();
        sort_by_id(&mut menu);
        assert_eq!(menu.rows.len(), 1 + MENU_TEXTS.len() + boxes.len());
        let row = menu
            .rows
            .iter()
            .find(|r| first_id(r) == Some(62101))
            .unwrap();
        assert!(matches!(&row[1], CellValue::Str(s) if s == "Break 1"));
        assert_eq!(first_id(&menu.rows[0]), Some(27));

        let mut talk = TblTable {
            columns: vec![
                ColumnType::UnsignedInt,
                ColumnType::String,
                ColumnType::UnsignedInt,
                ColumnType::UnsignedInt,
            ],
            rows: vec![],
        };
        apply_talk(&mut talk, true).unwrap();
        assert_eq!(talk.rows.len(), TALK_TEXTS.len());
        assert_eq!(talk.rows[0].len(), 4);

        let mut wrong = TblTable { columns: vec![ColumnType::UnsignedInt], rows: vec![] };
        assert!(apply_menu(&mut wrong, &boxes, false).is_err());
        assert!(apply_talk(&mut wrong, false).is_err());
    }
}
