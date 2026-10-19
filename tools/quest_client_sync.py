#!/usr/bin/env python3
"""Client-vs-server quest table sync (quest audit round 3).

Compares the CLIENT quest tables (ground truth: what the player sees) with the
live server snapshot, categorises every difference and generates an idempotent,
live-safe migration for what can be derived from the client tables.

Inputs
  --client-dump DIR   text dumps made with `tbl_dump <file> 0 1000000 > DIR/<Name>.txt` of the client
                      `Data/` tables: Quest_Helper, Quest_Monster_Exchange, Item_Exchange,
                      Item_Exchange_exp, NpcMopMap_Info (client NPC placement)
  --data-dir DIR      CSV snapshot made by `tools/quest_audit.py --dump` (+ npc_spawn_full.csv:
                      `\\copy (select ... from npc_spawn where is_monster=false) ...`, see PLACEMENT_SQL)
  --quests DIR        Lua scripts (default ./Quests)

Outputs
  --sql FILE          migration (psql / sqlx), default migrations/20261020100000_quest_client_sync.sql
  --after DIR         the snapshot with the migration applied (feed it to quest_audit.py to see the effect)
  --report FILE       JSON: per-category counts + the lists that were NOT changed

Categories (docs/QUEST_AUDIT.md, Round 3)
  A  server row missing          (quest_helper / quest_monster / item_exchange / npc_spawn)
  B  values differ from client   (level, kill counts/targets, exchange origin/reward/flag)
  C  Lua missing / wrong branch  (reported by quest_audit.py)
  D  NPC not spawned             (client placement exists -> npc_spawn row; otherwise reported)
  E  reward / event items absent from the server item table (reported)
"""
from __future__ import annotations

import argparse
import collections
import csv
import json
import os
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import quest_lua  # noqa: E402

csv.field_size_limit(1 << 24)

VIRTUAL = {810000000, 900000000, 900001000, 900002000, 900003000, 900004000, 900005000, 900006000,
           900007000, 900008000, 900009000, 900010000, 900011000, 900012000, 900016000, 900020000,
           900019000}
GOLD, EXP, NP, LADDER, SKILL = 900000000, 900001000, 900002000, 900003000, 900007000
VIRTUAL_REWARD = {GOLD, EXP, NP, LADDER}

# client Quest_Helper columns (two extra columns after n_exp that the server table does not have)
HELPER_COLS = ["n_index", "b_message_type", "b_level", "n_exp", "x4", "x5", "b_class", "b_nation",
               "b_quest_type", "b_zone", "s_npc_id", "s_event_data_index", "b_event_status",
               "n_event_trigger_index", "n_event_complete_index", "n_exchange_index",
               "n_event_talk_index", "str_lua_filename", "s_quest_menu", "s_npc_main", "s_quest_solo"]
SERVER_HELPER_COLS = [c for c in HELPER_COLS if c not in ("x4", "x5")]
SMALLINT_HELPER_COLS = ["b_message_type", "b_level", "b_class", "b_nation", "b_quest_type", "b_zone",
                        "s_npc_id", "s_event_data_index", "b_event_status", "s_quest_solo"]
EX_ORIGIN =[f"origin_item_{k}{i}" for i in range(1, 6) for k in ("num", "count")]
EX_REWARD = [f"exchange_item_{k}{i}" for i in range(1, 6) for k in ("num", "count")]

# client NpcMopMap_Info c2 (map group) -> server zone id (derived by joining the ~250 NPCs that exist in both)
MOP_ZONE = {10: 1, 11: 1, 20: 2, 21: 2, 30: 21, 31: 21, 80: 11, 90: 12, 100: 72, 110: 71, 130: 73, 140: 66}

# Exchange rows tied to the client-only quests that are NOT replaced although they differ, with the reason.
KEEP_SERVER_EXCHANGE = {
    182: "Kesar/Julius level-choice row: client lists 2 selections + ladder points, server pays 4 EXP tiers",
}
# Rows already judged "differs on purpose" by audit rounds 1+2 (kept).
KEEP_SERVER_REWARD = {944, 945, 985, 986, 987, 988, 989, 990, 991, 992, 1042, 1043, 6242}
# NPC files whose server exchange rows are random item pools of a different design (kept, reported)
KEEP_SERVER_FILES = {"16047_Moira.lua"}


def load_tbl(path: Path):
    rows = []
    for ln in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not ln.startswith("row="):
            continue
        r = []
        for part in ln.split(" | ")[1:]:
            _, _, v = part.partition("=")
            if v.startswith('"'):
                r.append(v[1:-1])
            else:
                try:
                    r.append(int(v))
                except ValueError:
                    try:
                        r.append(float(v))
                    except ValueError:
                        r.append(v)
        rows.append(r)
    return rows


def rc(path: Path):
    with path.open(encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh))


def sql_val(v):
    if isinstance(v, str):
        return "'" + v.replace("'", "''") + "'"
    return str(int(v))


def values(rows):
    return ",\n".join("(" + ",".join(sql_val(v) for v in r) + ")" for r in rows)


def pairs(vals):
    return [(vals[i], vals[i + 1]) for i in range(0, len(vals), 2) if vals[i]]


def run(a):
    """Analyse (and optionally write the migration / the 'after' snapshot). `a` needs the attributes
    client_dump, data_dir, quests, sql (Path or None), after (Path or None), report (Path or None)."""
    d, cd = a.data_dir, a.client_dump
    rep: dict = collections.OrderedDict()
    lua_files = {p.name.lower(): p.name for p in a.quests.glob("*.lua")}

    items = {int(r["num"]) for r in rc(d / "item.csv")}

    def item_ok(i):
        return i == 0 or i in items or i in VIRTUAL

    # ---------------------------------------------------------------- quest_helper
    ch = {r[0]: dict(zip(HELPER_COLS, r)) for r in load_tbl(cd / "Quest_Helper.txt")}
    sh_rows = rc(d / "quest_helper.csv")
    sh = {int(r["n_index"]): {k: (v if k == "str_lua_filename" else int(v)) for k, v in r.items()} for r in sh_rows}

    def sem(r):
        return (r["s_npc_id"], r["s_event_data_index"], r["b_event_status"],
                r["n_event_trigger_index"], r["n_event_complete_index"])

    server_sem = {sem(r) for r in sh.values()}
    add_h, skipped_h = [], collections.Counter()
    for i in sorted(ch):
        r = ch[i]
        if r["s_npc_id"] == 0:      # daily_quest_manager rows: the server runs its own daily-quest system
            skipped_h["daily_quest_manager (npc 0)"] += 1
            continue
        if sem(r) in server_sem:
            continue
        fn = lua_files.get(str(r["str_lua_filename"]).lower())
        if not fn:
            skipped_h["no Lua file"] += 1
            continue
        if any(not -32768 <= r[c] <= 32767 for c in SMALLINT_HELPER_COLS) or len(fn) > 40:
            skipped_h["value does not fit the column type"] += 1
            continue
        r = dict(r, str_lua_filename=fn)
        add_h.append(r)
    used_idx = set(sh) | {r["n_index"] for r in add_h if r["n_index"] not in sh}
    next_free = 40000
    collisions = []
    for r in add_h:
        if r["n_index"] in sh:
            while next_free in used_idx or next_free in ch:
                next_free += 1
            collisions.append((r["n_index"], next_free, r["s_npc_id"], r["str_lua_filename"]))
            r["new_index"] = next_free
            used_idx.add(next_free)
        else:
            r["new_index"] = r["n_index"]
    rep["A_helper_rows_added"] = len(add_h)
    rep["A_helper_rows_added_remapped_index"] = len(collisions)
    rep["A_helper_rows_added_by_file"] = dict(collections.Counter(r["str_lua_filename"] for r in add_h).most_common())
    rep["A_helper_rows_skipped"] = dict(skipped_h)

    # level fixes: client level lower than the server's (server blocks a quest the client offers)
    level_fix = []
    for i, r in ch.items():
        s = sh.get(i)
        if not s or s["s_npc_id"] != r["s_npc_id"] or s["s_event_data_index"] != r["s_event_data_index"]:
            continue
        if s["b_nation"] != r["b_nation"] and r["b_nation"] != 3 or s["b_zone"] != r["b_zone"]:
            continue  # different quest masquerading under the same index
        if r["b_level"] < s["b_level"]:
            level_fix.append((i, r["b_level"], s["b_level"]))
    rep["B_helper_level_lowered_to_client"] = len(level_fix)
    rep["B_helper_level_list"] = level_fix[:40]
    diff = collections.Counter()
    for i, r in ch.items():
        s = sh.get(i)
        if s and s["s_npc_id"] == r["s_npc_id"] and s["s_event_data_index"] == r["s_event_data_index"]:
            for k in SERVER_HELPER_COLS:
                if k != "str_lua_filename" and r[k] != s[k]:
                    diff[k] += 1
    rep["B_helper_other_differences_kept"] = dict(diff)

    # ---------------------------------------------------------------- quest_monster
    cmon = {}
    for r in load_tbl(cd / "Quest_Monster_Exchange.txt"):
        v = []
        for g in range(4):
            b = 1 + g * 9
            v += [r[b], r[b + 2], r[b + 4], r[b + 6], r[b + 8]]
        cmon[r[0]] = v
    mcols = ["s_quest_num"] + [f"s_num{g}{x}" for g in range(1, 5) for x in "abcd"][:0]
    mcols = ["s_quest_num"]
    for g in range(1, 5):
        mcols += [f"s_num{g}a", f"s_num{g}b", f"s_num{g}c", f"s_num{g}d", f"s_count{g}"]
    smon = {}
    for r in rc(d / "quest_monster.csv"):
        smon[int(r["s_quest_num"])] = [int(r[c]) for c in mcols[1:]]
    monsters = set()
    for r in rc(d / "npc_template.csv"):
        if r["is_monster"] in ("t", "true"):
            monsters.add(int(r["s_sid"]))
    mon_add, mon_upd, mon_skip = [], [], []
    for q, v in sorted(cmon.items()):
        ids = [x for g in range(4) for x in v[g * 5:g * 5 + 4] if x]
        bad = [x for x in ids if x not in monsters]
        if q not in smon:
            if bad:
                mon_skip.append((q, "unknown monster ids %s" % bad[:3]))
            else:
                mon_add.append((q, v))
        elif smon[q] != v:
            if bad:
                mon_skip.append((q, "unknown monster ids %s" % bad[:3]))
            else:
                mon_upd.append((q, v, smon[q]))
    rep["A_quest_monster_added"] = len(mon_add)
    rep["B_quest_monster_updated"] = len(mon_upd)
    rep["B_quest_monster_kept_unknown_monster_ids"] = mon_skip[:60]
    rep["B_quest_monster_updated_list"] = [(q, old, new) for q, new, old in mon_upd]

    # ---------------------------------------------------------------- item_exchange
    cex = {}
    for r in load_tbl(cd / "Item_Exchange.txt"):
        cex[r[0]] = {"flag": r[1], "origin": r[3:13], "reward": r[13:23]}
    sex_rows = rc(d / "item_exchange.csv")
    sex = {int(r["n_index"]): {"flag": int(r["random_flag"]), "origin": [int(r[c]) for c in EX_ORIGIN],
                               "reward": [int(r[c]) for c in EX_REWARD], "raw": r} for r in sex_rows}

    scripts = {p.name: quest_lua.parse(p.name, p.read_text(encoding="utf-8", errors="replace"))
               for p in sorted(a.quests.glob("*.lua"))}
    exch_funcs = {"RunQuestExchange", "RunExchange", "CheckExchange", "RunCountExchange",
                  "RunRandomExchange", "GetMaxExchange"}
    used = collections.defaultdict(set)
    for n, sc in scripts.items():
        for c in sc.calls:
            if c.name in exch_funcs:
                for x in c.arg_const(1, sc.env) or ():
                    used[x].add(n)
    for r in list(sh.values()) + add_h:
        if r["n_exchange_index"]:
            used[r["n_exchange_index"]].add(r["str_lua_filename"])
    newfiles = {r["str_lua_filename"] for r in add_h}

    ex_add, ex_replace, ex_append, ex_keep = [], [], [], []
    ex_add_bad = []
    for e in sorted(used):
        c, s = cex.get(e), sex.get(e)
        if not c:
            continue
        origin_ok = all(item_ok(x) for x, _ in pairs(c["origin"]))
        reward_ok = all(item_ok(x) for x, _ in pairs(c["reward"]))
        if s is None:
            if origin_ok:
                ex_add.append(e)
                if not reward_ok:
                    ex_add_bad.append(e)
            continue
        if (c["flag"], c["origin"], c["reward"]) == (s["flag"], s["origin"], s["reward"]):
            continue
        files = used[e]
        tied_new = bool(files & newfiles)
        if e in KEEP_SERVER_EXCHANGE or (files & KEEP_SERVER_FILES) or e in KEEP_SERVER_REWARD:
            ex_keep.append((e, "intentional/other design"))
            continue
        if tied_new:
            if origin_ok and reward_ok:
                ex_replace.append(e)
            else:
                ex_keep.append((e, "client row references items absent from the item table"))
            continue
        # non-new NPCs: only add reward items the client lists and the server row lacks
        cr, sr = pairs(c["reward"]), pairs(s["reward"])
        miss = [p for p in cr if p not in sr]
        if (c["flag"] == s["flag"] and miss and len(sr) + len(miss) <= 5
                and all(p in cr for p in sr) and all(item_ok(x) for x, _ in miss)
                and s["flag"] in (0, 10)):
            ex_append.append((e, miss))
        else:
            ex_keep.append((e, "differs from client: flag %s/%s origin %s reward %s" % (
                c["flag"], s["flag"], "diff" if c["origin"] != s["origin"] else "same",
                "diff" if c["reward"] != s["reward"] else "same")))
    rep["A_exchange_rows_added"] = len(ex_add)
    rep["A_exchange_rows_added_with_unknown_reward_item"] = ex_add_bad
    rep["B_exchange_rows_replaced_by_client"] = ex_replace
    rep["B_exchange_reward_items_appended"] = [(e, m) for e, m in ex_append]
    kc = collections.Counter(r for _, r in ex_keep)
    rep["B_exchange_differences_kept"] = {"count": len(ex_keep), "by_reason": dict(kc.most_common()),
                                           "ids": [e for e, _ in ex_keep]}
    rep["A_exchange_used_missing_everywhere"] = sorted(e for e in used if e not in sex and e not in cex)

    # ---------------------------------------------------------------- npc_spawn (client placement)
    spawn_rows = rc(d / "npc_spawn_full.csv") if (d / "npc_spawn_full.csv").exists() else []
    spawned = {int(r["npc_id"]) for r in spawn_rows}
    tm = {int(r["s_sid"]) for r in rc(d / "npc_template.csv") if r["is_monster"] in ("f", "false")}
    mop = {}
    mp = cd / "NpcMopMap_Info.txt"
    if mp.exists():
        for r in load_tbl(mp):
            mop.setdefault(r[1], r)
    quest_npcs = {r["s_npc_id"] for r in sh.values() if r["s_npc_id"]} | {r["s_npc_id"] for r in add_h}
    unspawned = sorted(n for n in quest_npcs if n not in spawned)
    sp_add, sp_noplace, sp_notmpl = [], [], []
    for n in unspawned:
        if n not in tm:
            sp_notmpl.append(n)
        elif n in mop and mop[n][2] in MOP_ZONE:
            m = mop[n]
            sp_add.append((MOP_ZONE[m[2]], n, m[4], m[5]))
        else:
            sp_noplace.append(n)
    rep["D_quest_npcs_unspawned"] = len(unspawned)
    rep["D_npc_spawn_added_from_client_placement"] = [list(x) for x in sp_add]
    rep["D_unspawned_no_client_placement"] = len(sp_noplace)
    rep["D_unspawned_no_client_placement_ids"] = sp_noplace
    rep["D_unspawned_no_npc_template"] = len(sp_notmpl)
    rep["D_unspawned_no_npc_template_ids"] = sp_notmpl

    # ---------------------------------------------------------------- F: quest items that nothing drops
    # A quest that asks for N of an item (Item_Exchange origin) while killing the monsters of
    # Quest_Monster_Exchange needs that item to drop from those monsters. Items that no monster
    # drops and no exchange produces are put into the free slots of the quest monsters' drop tables.
    drop_updates, drop_report = [], []
    if (d / "monster_item.csv").exists() and (d / "monster_tmpl.csv").exists():
        mi_rows = {int(r["s_index"]): r for r in rc(d / "monster_item.csv")}
        mon_table = {int(r["s_sid"]): int(r["s_item"]) for r in rc(d / "monster_tmpl.csv")}
        dropped = collections.defaultdict(set)
        for t, r in mi_rows.items():
            for k in range(1, 13):
                if int(r["item%02d" % k] or 0):
                    dropped[int(r["item%02d" % k])].add(t)
        produced = set()
        for r in sex_rows:
            for k in range(1, 6):
                produced.add(int(r["exchange_item_num%d" % k]))
        # rates installed by migration 20260817000005 (copied from the authoritative v2369 drop rows)
        known_rate = {900331000: 1925, 900332000: 2500, 508211000: 300, 508212000: 300,
                      508213000: 875, 900060000: 725, 900068000: 100}
        cmon_raw = {}
        for r in load_tbl(cd / "Quest_Monster_Exchange.txt"):
            groups = [([x for x in (r[1 + g * 9], r[3 + g * 9], r[5 + g * 9], r[7 + g * 9]) if x], r[9 + g * 9])
                      for g in range(4)]
            cmon_raw[r[0]] = groups
        seen = set()
        free_slots = {t: [k for k in range(1, 13) if not int(r["item%02d" % k] or 0)] for t, r in mi_rows.items()}
        for h in ch.values():
            e, q = h["n_exchange_index"], h["s_event_data_index"]
            if not e or e not in cex or q not in cmon_raw:
                continue
            groups = cmon_raw[q]
            mons = [m for g, _ in groups for m in g]
            kills = sum(c for _, c in groups if c) or 1
            for it, cnt in pairs(cex[e]["origin"]):
                if it in VIRTUAL or it == 0 or (q, it) in seen:
                    continue
                seen.add((q, it))
                tabs = sorted({mon_table[m] for m in mons if m in mon_table and mon_table[m] in mi_rows})
                if not tabs or any(it in dropped and t in dropped[it] for t in tabs):
                    continue
                if it in produced:
                    drop_report.append((q, it, "produced by an exchange (chain item), not a drop"))
                    continue
                boss = kills <= 1
                for t in tabs:
                    need_slots = min(cnt, 5) if boss else 1
                    rate = 10000 if boss else known_rate.get(it, min(10000, max(100, int(cnt / kills * 12000))))
                    for _ in range(need_slots):
                        if not free_slots[t]:
                            drop_report.append((q, it, "no free slot in drop table %d" % t))
                            break
                        slot = free_slots[t].pop(0)
                        drop_updates.append((t, slot, it, rate, q))
    rep["F_quest_item_drops_added"] = len(drop_updates)
    rep["F_quest_item_drops_list"] = drop_updates
    rep["F_quest_item_drops_not_added"] = drop_report

    # ---------------------------------------------------------------- E: items absent from item table
    miss_items = collections.defaultdict(set)
    for e in used:
        src = cex.get(e) or {"origin": [], "reward": []}
        for tag in ("origin", "reward"):
            for x, _ in pairs(src[tag]):
                if not item_ok(x):
                    miss_items[x].add(e)
    rep["E_items_in_used_client_exchanges_absent_from_item_table"] = {str(k): sorted(v)[:6] for k, v in sorted(miss_items.items())}

    # ---------------------------------------------------------------- SQL
    out = []
    w = out.append
    w("-- Quest audit round 3: make the quest tables match the CLIENT tables (Data/Quest_Helper.tbl,")
    w("-- Quest_Monster_Exchange.tbl, Item_Exchange.tbl, NpcMopMap_Info.tbl). Generated by tools/quest_client_sync.py,")
    w("-- idempotent (explicit keys, ON CONFLICT / NOT EXISTS guards), nothing is deleted.")
    w("BEGIN;")
    w("")
    w("-- A) quest_helper: client rows the server lacks (event menus of Magmom, Mackin, Aset, Rabbit, Hill/Rily,")
    w("--    Kesar/Julius, ...). A row is skipped when the server already has the same")
    w("--    (npc, quest, status, trigger, complete). %d client indexes collide with other server rows ->" % len(collisions))
    w("--    those are inserted at 40000+ (the Lua scripts never SaveEvent() these ids).")
    hcols = ",".join(SERVER_HELPER_COLS)
    hrows = []
    for r in add_h:
        hrows.append([r["new_index"]] + [r[c] for c in SERVER_HELPER_COLS[1:]])
    for k in range(0, len(hrows), 200):
        w("INSERT INTO quest_helper (%s)" % hcols)
        w("SELECT v.* FROM (VALUES\n%s\n) AS v(%s)" % (values(hrows[k:k + 200]), hcols))
        w("WHERE NOT EXISTS (SELECT 1 FROM quest_helper q WHERE q.s_npc_id = v.s_npc_id AND q.s_event_data_index = v.s_event_data_index")
        w("  AND q.b_event_status = v.b_event_status AND q.n_event_trigger_index = v.n_event_trigger_index")
        w("  AND q.n_event_complete_index = v.n_event_complete_index)")
        w("ON CONFLICT (n_index) DO NOTHING;")
    w("")
    w("-- B) quest_helper: required level lowered to the client's where the server demanded more than the client shows")
    if level_fix:
        w("UPDATE quest_helper q SET b_level = v.lvl FROM (VALUES\n%s\n) AS v(idx, lvl, old)" % values(level_fix))
        w("WHERE q.n_index = v.idx AND q.b_level = v.old;")
    w("")
    w("-- A) quest_monster rows missing / B) rows whose kill targets or counts differ from the client")
    mcols_sql = ",".join(mcols)
    mrows = [[q] + v for q, v in mon_add]
    for k in range(0, len(mrows), 200):
        w("INSERT INTO quest_monster (%s) VALUES\n%s\nON CONFLICT (s_quest_num) DO NOTHING;" % (mcols_sql, values(mrows[k:k + 200])))
    for q, new, old in mon_upd:
        sets = ",".join("%s=%d" % (c, v) for c, v in zip(mcols[1:], new))
        cond = " AND ".join("%s=%d" % (c, v) for c, v in zip(mcols[1:], old))
        w("UPDATE quest_monster SET %s WHERE s_quest_num=%d AND %s;" % (sets, q, cond))
    w("")
    w("-- A) item_exchange rows the Lua/helper rows use that exist only in the client table")
    ecols = ["n_index", "random_flag"] + EX_ORIGIN + EX_REWARD + [f"exchange_item_time{i}" for i in range(1, 6)]
    erows = []
    for e in ex_add:
        c = cex[e]
        erows.append([e, c["flag"]] + c["origin"] + c["reward"] + [0] * 5)
    for k in range(0, len(erows), 100):
        w("INSERT INTO item_exchange (%s) VALUES\n%s\nON CONFLICT (n_index) DO NOTHING;" % (",".join(ecols), values(erows[k:k + 100])))
    w("")
    w("-- B) item_exchange rows of the client-only quests whose server row was a placeholder (hunt quests with a")
    w("--    random equipment pool, wrong rewards): replaced by the client row (only when the server row still has the")
    w("--    snapshot value, so a later manual edit is never overwritten)")
    for e in ex_replace:
        c, s = cex[e], sex[e]
        sets = ["random_flag=%d" % c["flag"]] + ["%s=%d" % (col, v) for col, v in zip(EX_ORIGIN + EX_REWARD, c["origin"] + c["reward"])]
        cond = ["random_flag=%d" % s["flag"]] + ["%s=%d" % (col, v) for col, v in zip(EX_ORIGIN + EX_REWARD, s["origin"] + s["reward"])]
        w("UPDATE item_exchange SET %s WHERE n_index=%d AND %s;" % (",".join(sets), e, " AND ".join(cond)))
    w("")
    w("-- B) reward items the client lists for the exchange but the server row lacked (free slots only)")
    for e, miss in ex_append:
        s = sex[e]
        sr = pairs(s["reward"])
        free = [i for i in range(1, 6) if not s["reward"][(i - 1) * 2]]
        sets, cond = [], ["%s=%d" % (col, v) for col, v in zip(EX_REWARD, s["reward"])]
        for (item, cnt), slot in zip(miss, free):
            sets += ["exchange_item_num%d=%d" % (slot, item), "exchange_item_count%d=%d" % (slot, cnt)]
        w("UPDATE item_exchange SET %s WHERE n_index=%d AND %s;" % (",".join(sets), e, " AND ".join(cond)))
    w("")
    w("-- D) quest NPCs without any npc_spawn row, placed where the CLIENT's NpcMopMap_Info puts them")
    for zone, npc, x, z in sp_add:
        w("INSERT INTO npc_spawn (zone_id, npc_id, is_monster, act_type, regen_type, left_x, top_z, num_npc, spawn_range, regen_time, direction, dot_cnt, room)")
        w("SELECT %d, %d, false, 100, 0, %d, %d, 1, 0, 3600, 0, 0, 0 WHERE NOT EXISTS (SELECT 1 FROM npc_spawn WHERE npc_id = %d);" % (zone, npc, x, z, npc))
    w("")
    w("-- F) quest items no monster dropped (quest kill targets from Quest_Monster_Exchange, items from Item_Exchange):")
    w("--    written into the free slots of the quest monsters' drop tables, never over an existing drop")
    for t, slot, it, rate, q in drop_updates:
        w("UPDATE monster_item SET item%02d = %d, percent%02d = %d WHERE s_index = %d AND item%02d IN (0, %d); -- quest %d" % (slot, it, slot, rate, t, slot, it, q))
    w("")
    w("-- E) the two tattoo voucher items the client item table has and the server item table lacks")
    w("--    (cloned from the sibling 30-day voucher 810926000, names from the client Item_Org_us.tbl)")
    for num, name in ((810921000, "Voucher of War Wings ( 30 Days )"), (810924000, "Voucher of Wing of Pitch Black ( 30 Days )")):
        w("INSERT INTO item SELECT (jsonb_populate_record(NULL::item, to_jsonb(i) || jsonb_build_object('num', %d, 'str_name', '%s'))).*" % (num, name))
        w("FROM item i WHERE i.num = 810926000 AND NOT EXISTS (SELECT 1 FROM item x WHERE x.num = %d);" % num)
    w("")
    w("COMMIT;")
    if a.sql:
        a.sql.parent.mkdir(parents=True, exist_ok=True)
        a.sql.write_text(chr(10).join(out) + chr(10), encoding="utf-8")

    # ---------------------------------------------------------------- snapshot after the migration
    if a.after:
        a.after.mkdir(parents=True, exist_ok=True)
        for f in d.glob("*.csv"):
            shutil.copy(f, a.after / f.name)
        hdr = list(sh_rows[0].keys())
        with (a.after / "quest_helper.csv").open("a", encoding="utf-8", newline="") as fh:
            wr = csv.DictWriter(fh, hdr)
            for r in add_h:
                row = {k: r[k] for k in hdr if k in r}
                row["n_index"] = r["new_index"]
                wr.writerow(row)
        # level fixes
        lf = {i: new for i, new, _ in level_fix}
        rows = rc(a.after / "quest_helper.csv")
        for r in rows:
            if int(r["n_index"]) in lf:
                r["b_level"] = lf[int(r["n_index"])]
        with (a.after / "quest_helper.csv").open("w", encoding="utf-8", newline="") as fh:
            wr = csv.DictWriter(fh, hdr)
            wr.writeheader()
            wr.writerows(rows)
        # quest_monster
        mrows_all = rc(a.after / "quest_monster.csv")
        mh = list(mrows_all[0].keys())
        byq = {int(r["s_quest_num"]): r for r in mrows_all}
        for q, v in mon_add + [(q, n) for q, n, _ in mon_upd]:
            byq[q] = dict(zip(mh, [q] + v))
        with (a.after / "quest_monster.csv").open("w", encoding="utf-8", newline="") as fh:
            wr = csv.DictWriter(fh, mh)
            wr.writeheader()
            wr.writerows(byq[k] for k in sorted(byq))
        # item_exchange
        erows_all = rc(a.after / "item_exchange.csv")
        eh = list(erows_all[0].keys())
        bye = {int(r["n_index"]): r for r in erows_all}
        for e in ex_add:
            c = cex[e]
            bye[e] = dict(zip(eh, [e, c["flag"]] + c["origin"] + c["reward"] + [0] * 5))
        for e in ex_replace:
            c = cex[e]
            bye[e].update(dict(zip(["random_flag"] + EX_ORIGIN + EX_REWARD, [c["flag"]] + c["origin"] + c["reward"])))
        for e, miss in ex_append:
            s = sex[e]
            free = [i for i in range(1, 6) if not s["reward"][(i - 1) * 2]]
            for (item, cnt), slot in zip(miss, free):
                bye[e]["exchange_item_num%d" % slot] = item
                bye[e]["exchange_item_count%d" % slot] = cnt
        with (a.after / "item_exchange.csv").open("w", encoding="utf-8", newline="") as fh:
            wr = csv.DictWriter(fh, eh)
            wr.writeheader()
            wr.writerows(bye[k] for k in sorted(bye))
        # monster_item drops
        if drop_updates and (a.after / "monster_item.csv").exists():
            mrows_d = rc(a.after / "monster_item.csv")
            mh_d = list(mrows_d[0].keys())
            byt = {int(r["s_index"]): r for r in mrows_d}
            for t, slot, it, rate, q in drop_updates:
                byt[t]["item%02d" % slot] = it
                byt[t]["percent%02d" % slot] = rate
            with (a.after / "monster_item.csv").open("w", encoding="utf-8", newline="") as fh:
                wr = csv.DictWriter(fh, mh_d)
                wr.writeheader()
                wr.writerows(byt[k] for k in sorted(byt))
        if sp_add and (a.after / "npc_spawn_full.csv").exists():
            with (a.after / "npc_spawn_full.csv").open("a", encoding="utf-8", newline="") as fh:
                for zone, npc, x, z in sp_add:
                    fh.write("0,%d,%d,f,100,0,0,0,0,%d,%d,1,0,3600,0,0,0,f" % (zone, npc, x, z) + chr(10))
        # npc_spawn aggregate
        sp = rc(a.after / "npc_spawn.csv")
        for zone, npc, x, z in sp_add:
            sp.append({"npc_id": npc, "zone_id": zone, "is_monster": "f", "spawns": 1})
        with (a.after / "npc_spawn.csv").open("w", encoding="utf-8", newline="") as fh:
            wr = csv.DictWriter(fh, ["npc_id", "zone_id", "is_monster", "spawns"])
            wr.writeheader()
            wr.writerows(sp)

    if a.report:
        a.report.write_text(json.dumps(rep, indent=1, default=list), encoding="utf-8")
    return rep


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--client-dump", required=True, type=Path)
    ap.add_argument("--data-dir", required=True, type=Path)
    ap.add_argument("--quests", type=Path, default=Path("Quests"))
    ap.add_argument("--sql", type=Path, default=Path("migrations/20261020100000_quest_client_sync.sql"))
    ap.add_argument("--after", type=Path)
    ap.add_argument("--report", type=Path)
    rep = run(ap.parse_args())
    for k, v in rep.items():
        if isinstance(v, (int, float)):
            print(f"{k}: {v}")
        elif isinstance(v, (list, dict)):
            print(f"{k}: {len(v)} entries")


if __name__ == "__main__":
    main()
