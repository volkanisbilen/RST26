// Quest simulation harness (included into `bindings::tests`).
//
// Drives the REAL Lua engine over every `EVENT` handler of every quest script that calls
// `RunQuestExchange`, for both nations, several classes and every reward-window index, and compares
// the outcome (bag, exp, national points, gold) with what the CLIENT tables define.
//
//   python tools/quest_sim_plan.py --data-dir <live csv> --client-dir <tbl_dump output> --out <dir>
//   QUEST_SIM_DIR=<dir> QUEST_SIM_QUESTS=<Quests dir> cargo test -p ko-game --lib quest_sim_all -- --ignored --nocapture
//
// Result: `<dir>/result.tsv` (one line per run) and a verdict summary on stdout.

use crate::lua_engine::quest_reward::{TraceEvent, TRACE, TRACE_ON};
use crate::lua_engine::LuaEngine;
use crate::world::UserQuestInfo;
use ko_db::models::item_tables::ItemExchangeExpRow;
use ko_db::models::{QuestHelperRow, QuestMonsterRow};
use std::collections::{BTreeMap, HashMap};

fn sim_tsv(dir: &str, name: &str) -> Vec<Vec<String>> {
    std::fs::read_to_string(format!("{dir}/{name}"))
        .unwrap_or_else(|e| panic!("{dir}/{name}: {e}"))
        .lines()
        .map(|l| l.split('\t').map(|s| s.to_string()).collect())
        .collect()
}

fn n<T: std::str::FromStr + Default>(s: &str) -> T {
    s.trim().parse().unwrap_or_default()
}

#[derive(Clone, Default)]
struct ClientRow {
    flag: i32,
    origins: Vec<(u32, u32)>,
    outs: Vec<(u32, u32)>,
}

fn filled(v: &[i64]) -> Vec<(u32, u32)> {
    v.chunks(2)
        .filter(|c| c[0] > 0 && c[1] > 0)
        .map(|c| (c[0] as u32, c[1] as u32))
        .collect()
}

fn sim_virtual(id: u32) -> bool {
    matches!(id, 900_000_000 | 900_001_000 | 900_002_000 | 900_003_000 | 900_007_000)
}

struct SimBag {
    items: BTreeMap<u32, u32>,
    gold: u32,
    exp: u64,
    level: u8,
    np: u32,
}

fn sim_snapshot(world: &WorldState) -> SimBag {
    let mut items = BTreeMap::new();
    for s in world.get_inventory(1).iter().skip(WorldState::SLOT_MAX) {
        if s.item_id != 0 {
            *items.entry(s.item_id).or_insert(0) += s.count as u32;
        }
    }
    let c = world.get_character_info(1).unwrap();
    SimBag {
        items,
        gold: c.gold,
        exp: c.exp,
        level: c.level,
        np: c.loyalty,
    }
}

fn sim_prepare(
    world: &WorldState,
    nation: u8,
    class: u16,
    qids: &[u16],
    state: u8,
    origins: &[(u32, u32)],
    flag: u8,
    step: i8,
    helper: u32,
    stackable: &HashMap<u32, bool>,
) {
    world.update_character_stats(1, |c| {
        c.nation = nation;
        c.race = if nation == 1 { 1 } else { 12 };
        c.class = class;
        c.level = 60;
        c.exp = 0;
        c.gold = 100_000_000;
        c.loyalty = 100_000;
    });
    world.set_inventory(
        1,
        vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX],
    );
    let mut slots = 0;
    for &(id, cnt) in origins {
        if sim_virtual(id) || !stackable.contains_key(&id) {
            continue;
        }
        if stackable.get(&id).copied().unwrap_or(false) {
            world.give_item(1, id, cnt.min(9999) as u16);
            slots += 1;
        } else {
            for _ in 0..cnt.min(6) {
                world.give_item(1, id, 1);
                slots += 1;
            }
        }
        if slots > 12 {
            break;
        }
    }
    world.update_session(1, |h| {
        h.quests.clear();
        for &q in qids {
            h.quests.insert(
                q,
                UserQuestInfo {
                    quest_state: state,
                    kill_counts: [250; 4],
                },
            );
        }
        h.select_msg_flag = flag;
        h.by_selected_reward = step;
        h.quest_helper_id = helper;
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn quest_sim_all() {
    let dir = std::env::var("QUEST_SIM_DIR").expect("QUEST_SIM_DIR");
    let quests = std::env::var("QUEST_SIM_QUESTS").unwrap_or("../../Quests".into());
    let (_lua, world, _rx) = setup_lua_world_with_rx();

    // --- data --------------------------------------------------------------------------------
    let mut stackable = HashMap::new();
    for r in sim_tsv(&dir, "items.tsv") {
        let id: u32 = n(&r[0]);
        let countable: i32 = n(&r[1]);
        insert_test_item(&world, id, countable != 0);
        stackable.insert(id, countable != 0);
        let class: i32 = n(&r[3]);
        if class != 0 {
            let mut it = world.get_item(id).unwrap();
            it.class = Some(class);
            world.insert_item(id, it);
        }
    }
    for r in sim_tsv(&dir, "exchange.tsv") {
        let v: Vec<i32> = r.iter().map(|s| n(s)).collect();
        let row = ItemExchangeRow {
            n_index: v[0],
            random_flag: v[1] as i16,
            origin_item_num1: v[2],
            origin_item_count1: v[3],
            origin_item_num2: v[4],
            origin_item_count2: v[5],
            origin_item_num3: v[6],
            origin_item_count3: v[7],
            origin_item_num4: v[8],
            origin_item_count4: v[9],
            origin_item_num5: v[10],
            origin_item_count5: v[11],
            exchange_item_num1: v[12],
            exchange_item_count1: v[13],
            exchange_item_num2: v[14],
            exchange_item_count2: v[15],
            exchange_item_num3: v[16],
            exchange_item_count3: v[17],
            exchange_item_num4: v[18],
            exchange_item_count4: v[19],
            exchange_item_num5: v[20],
            exchange_item_count5: v[21],
            exchange_item_time1: v[22],
            exchange_item_time2: v[23],
            exchange_item_time3: v[24],
            exchange_item_time4: v[25],
            exchange_item_time5: v[26],
        };
        world.insert_item_exchange(row.n_index, row);
    }
    for r in sim_tsv(&dir, "exp.tsv") {
        let v: Vec<i32> = r.iter().map(|s| n(s)).collect();
        let o = |i: usize| Some(v[i]);
        let row = ItemExchangeExpRow {
            n_index: v[0],
            random_flag: Some(v[1] as i16),
            exchange_item_num1: o(2),
            exchange_item_count1: o(3),
            exchange_item_num2: o(4),
            exchange_item_count2: o(5),
            exchange_item_num3: o(6),
            exchange_item_count3: o(7),
            exchange_item_num4: o(8),
            exchange_item_count4: o(9),
            exchange_item_num5: o(10),
            exchange_item_count5: o(11),
            exchange_item_time1: Some(0),
            exchange_item_time2: Some(0),
            exchange_item_time3: Some(0),
            exchange_item_time4: Some(0),
            exchange_item_time5: Some(0),
        };
        world.insert_test_item_exchange_exp(row);
    }
    let mut helper_of_lua: HashMap<String, u32> = HashMap::new();
    for r in sim_tsv(&dir, "helper.tsv") {
        let row = QuestHelperRow {
            n_index: n(&r[0]),
            b_message_type: n(&r[1]),
            b_level: n(&r[2]),
            n_exp: n(&r[3]),
            b_class: n(&r[4]),
            b_nation: n(&r[5]),
            b_quest_type: n(&r[6]),
            b_zone: n(&r[7]),
            s_npc_id: n(&r[8]),
            s_event_data_index: n(&r[9]),
            b_event_status: n(&r[10]),
            n_event_trigger_index: n(&r[11]),
            n_event_complete_index: n(&r[12]),
            n_exchange_index: n(&r[13]),
            n_event_talk_index: n(&r[14]),
            str_lua_filename: r[15].clone(),
            s_quest_menu: n(&r[16]),
            s_npc_main: n(&r[17]),
            s_quest_solo: n(&r[18]),
        };
        helper_of_lua
            .entry(row.str_lua_filename.clone())
            .or_insert(row.n_index as u32);
        world.insert_test_quest_helper(row);
    }
    for r in sim_tsv(&dir, "monster.tsv") {
        let v: Vec<i16> = r.iter().map(|s| n(s)).collect();
        world.insert_test_quest_monster(QuestMonsterRow {
            s_quest_num: v[0],
            s_num1a: v[1],
            s_num1b: v[2],
            s_num1c: v[3],
            s_num1d: v[4],
            s_count1: v[5],
            s_num2a: v[6],
            s_num2b: v[7],
            s_num2c: v[8],
            s_num2d: v[9],
            s_count2: v[10],
            s_num3a: v[11],
            s_num3b: v[12],
            s_num3c: v[13],
            s_num3d: v[14],
            s_count3: v[15],
            s_num4a: v[16],
            s_num4b: v[17],
            s_num4c: v[18],
            s_num4d: v[19],
            s_count4: v[20],
        });
    }
    let mut client_ex: HashMap<i32, ClientRow> = HashMap::new();
    for r in sim_tsv(&dir, "client_ex.tsv") {
        let v: Vec<i64> = r.iter().map(|s| n(s)).collect();
        client_ex.insert(
            v[0] as i32,
            ClientRow {
                flag: v[1] as i32,
                origins: filled(&v[2..12]),
                outs: filled(&v[12..22]),
            },
        );
    }
    let mut client_exp: HashMap<i32, ClientRow> = HashMap::new();
    for r in sim_tsv(&dir, "client_exp.tsv") {
        let v: Vec<i64> = r.iter().map(|s| n(s)).collect();
        client_exp.insert(
            v[0] as i32,
            ClientRow {
                flag: v[1] as i32,
                origins: vec![],
                outs: filled(&v[2..12]),
            },
        );
    }

    let engine = LuaEngine::with_script_dir(std::path::PathBuf::from(&quests));
    TRACE_ON.store(true, std::sync::atomic::Ordering::Relaxed);

    let mut out = String::new();
    let mut verdicts: BTreeMap<String, usize> = BTreeMap::new();
    let plan = sim_tsv(&dir, "plan.tsv");
    for row in plan.iter().filter(|r| r[3] != "-") {
        let lua = &row[0];
        let event: i32 = n(&row[1]);
        let flags: Vec<u8> = row[2].split(',').map(|s| n(s)).collect();
        let ids: Vec<i32> = row[3].split(',').map(|s| n(s)).collect();
        let qids: Vec<u16> = if row[4] == "-" {
            vec![]
        } else {
            row[4].split(',').map(|s| n(s)).collect()
        };
        let nations: Vec<u8> = row[5].split(',').map(|s| n(s)).collect();
        let uses_class = row[6] == "1";
        let flag = if flags.contains(&5) { 5 } else { flags[0] };
        let steps: Vec<i8> = if flag == 5 { vec![0, 1, 2, 3, 4] } else { vec![-1] };
        // turn-in items of every exchange the handler can reach
        let mut origins: Vec<(u32, u32)> = Vec::new();
        for id in &ids {
            if let Some(e) = world.get_item_exchange(*id) {
                for (a, b) in [
                    (e.origin_item_num1, e.origin_item_count1),
                    (e.origin_item_num2, e.origin_item_count2),
                    (e.origin_item_num3, e.origin_item_count3),
                    (e.origin_item_num4, e.origin_item_count4),
                    (e.origin_item_num5, e.origin_item_count5),
                ] {
                    if a > 0 && b > 0 {
                        origins.push((a as u32, b as u32));
                    }
                }
            }
        }
        let mut want_nations = Vec::new();
        for nat in &nations {
            match nat {
                1 => want_nations.push(1u8),
                2 => want_nations.push(2u8),
                _ => {
                    want_nations.push(1u8);
                    want_nations.push(2u8);
                }
            }
        }
        want_nations.sort();
        want_nations.dedup();
        let helper = *helper_of_lua.get(lua).unwrap_or(&0);
        let mut reached_any = false;
        for &nat in &want_nations {
            let base: u16 = if nat == 1 { 100 } else { 200 };
            let classes: Vec<u16> = if uses_class {
                [1, 2, 3, 4, 6, 8, 10, 12].iter().map(|c| base + c).collect()
            } else {
                vec![base + 1]
            };
            for &class in &classes {
                for &step in &steps {
                    for (state, with_items) in [(1u8, true), (3, true), (1, false), (0, false), (0, true)] {
                        sim_prepare(
                            &world, nat, class, &qids, state, if with_items { &origins } else { &[] }, flag, step, helper,
                            &stackable,
                        );
                        TRACE.lock().unwrap().clear();
                        let before = sim_snapshot(&world);
                        let ok_run = engine.execute(&world, 1, event, step, lua);
                        let after = sim_snapshot(&world);
                        let trace: Vec<TraceEvent> = TRACE.lock().unwrap().clone();
                        let ex_events: Vec<&TraceEvent> = trace
                            .iter()
                            .filter(|t| matches!(t, TraceEvent::Exchange { .. }))
                            .collect();
                        if ex_events.is_empty() {
                            continue;
                        }
                        reached_any = true;
                        let completed = trace.iter().any(|t| {
                            matches!(t, TraceEvent::Save { status: 2, blocked: false, .. })
                        });
                        for t in ex_events {
                            let TraceEvent::Exchange { id, sel, ok, reason, paid } = t else {
                                continue;
                            };
                            let (verdict, detail) = sim_judge(
                                *id, *ok, reason, paid, &client_ex, &client_exp, &before, &after,
                                &stackable, ok_run, completed,
                            );
                            *verdicts.entry(verdict.clone()).or_insert(0) += 1;
                            out.push_str(&format!(
                                "{lua}\t{event}\t{nat}\t{class}\t{step}\tstate{state}\t{id}\t{sel}\t{ok}\t{reason}\t{verdict}\t{detail}\n"
                            ));
                        }
                        break; // reached with this state
                    }
                }
            }
        }
        if !reached_any {
            *verdicts.entry("NOT_REACHED".into()).or_insert(0) += 1;
            out.push_str(&format!(
                "{lua}\t{event}\t-\t-\t-\t-\t{}\t-\tfalse\t\tNOT_REACHED\t\n",
                row[3]
            ));
        }
    }
    std::fs::write(format!("{dir}/result.tsv"), out).unwrap();
    println!("== quest_sim verdicts ==");
    for (k, v) in &verdicts {
        println!("{k:40} {v}");
    }
}

/// Compare one finished `RunQuestExchange` with the client rows.
#[allow(clippy::too_many_arguments)]
fn sim_judge(
    id: i32,
    ok: bool,
    reason: &str,
    _paid: &[(u32, u32)],
    client_ex: &HashMap<i32, ClientRow>,
    client_exp: &HashMap<i32, ClientRow>,
    before: &SimBag,
    after: &SimBag,
    _stackable: &HashMap<u32, bool>,
    ok_run: bool,
    completed: bool,
) -> (String, String) {
    if !ok_run {
        return ("LUA_ERROR".into(), String::new());
    }
    let Some(cx) = client_ex.get(&id) else {
        return (
            if ok { "OK_NOT_IN_CLIENT" } else { "REFUSED_NOT_IN_CLIENT" }.into(),
            reason.into(),
        );
    };
    if !ok {
        let v = if completed {
            format!("REFUSED_BUT_COMPLETED:{reason}")
        } else {
            format!("REFUSED:{reason}")
        };
        return (v, String::new());
    }
    if matches!(cx.flag, 1..=9 | 20 | 30 | 101..=200) {
        // random / premium rows: any subset can be paid
        return ("OK_RANDOM".into(), String::new());
    }
    let exp = client_exp.get(&id);
    let choose_main = matches!(cx.flag, 11 | 12) && cx.outs.len() > 1;
    let choose_exp = exp
        .map(|e| matches!(e.flag, 11 | 12) && e.outs.len() > 1)
        .unwrap_or(false);
    let gained = |item: u32| -> i64 {
        after.items.get(&item).copied().unwrap_or(0) as i64
            - before.items.get(&item).copied().unwrap_or(0) as i64
    };
    let mut problems: Vec<String> = Vec::new();
    // fixed rewards
    let mut fixed: Vec<(u32, u32)> = Vec::new();
    if !choose_main {
        fixed.extend(cx.outs.iter().copied());
    }
    if let Some(e) = exp {
        if !choose_exp {
            for &(i, c) in &e.outs {
                if !sim_virtual(i) && cx.outs.iter().any(|(m, _)| *m == i) {
                    continue;
                }
                fixed.push((i, c));
            }
        }
    }
    let exp_gain = after.exp as i64 - before.exp as i64 + (after.level as i64 - before.level as i64) * 1_000_000_000;
    let origin_ids: Vec<u32> = cx.origins.iter().map(|(i, _)| *i).collect();
    for &(i, c) in &fixed {
        match i {
            900_001_000 => {
                if exp_gain <= 0 {
                    problems.push("NO_EXP".into());
                }
            }
            900_002_000 | 900_003_000 => {
                if after.np <= before.np {
                    problems.push("NO_NP".into());
                }
            }
            900_000_000 => {
                if after.gold <= before.gold {
                    problems.push("NO_GOLD".into());
                }
            }
            900_007_000 => {}
            _ => {
                if origin_ids.contains(&i) {
                    continue;
                }
                if gained(i) < c as i64 {
                    problems.push(format!("MISSING_ITEM_{i}"));
                }
            }
        }
    }
    // choose lists: exactly one entry
    let mut lists: Vec<&Vec<(u32, u32)>> = Vec::new();
    if choose_main {
        lists.push(&cx.outs);
    }
    if choose_exp {
        lists.push(&exp.unwrap().outs);
    }
    for l in lists {
        let real: Vec<u32> = l.iter().map(|(i, _)| *i).filter(|i| !sim_virtual(*i)).collect();
        if real.is_empty() {
            continue;
        }
        let mut uniq = real.clone();
        uniq.sort();
        uniq.dedup();
        let got = uniq.iter().filter(|i| gained(**i) > 0).count();
        if got == 0 {
            problems.push("CHOICE_NONE".into());
        } else if got > 1 {
            problems.push("CHOICE_ALL".into());
        }
    }
    if problems.is_empty() {
        ("OK".into(), String::new())
    } else {
        (problems.join("+"), String::new())
    }
}
