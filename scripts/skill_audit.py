#!/usr/bin/env python3
"""Skill / magic data audit for JstKO (server DB export vs client .tbl files).

Usage
-----
  1. Export the server tables (read-only) to CSV, e.g. with psql:
       for t in magic magic_type1 ... magic_type9; do
         psql "$DATABASE_URL" -c "COPY (SELECT * FROM $t ORDER BY 1) TO STDOUT WITH CSV HEADER" > live/$t.csv
       done
       psql "$DATABASE_URL" -c "COPY (SELECT num,kind,slot,class,countable,effect1,effect2 FROM item ORDER BY 1)
                                TO STDOUT WITH CSV HEADER" > live/item.csv
  2. python scripts/skill_audit.py --csv-dir live --client-data "D:/ai/JstKO Client/Data" \
         --tbl-dump target/debug/tbl_dump.exe [--only S1,V3] [--limit 15]

Every finding has an id so the report in docs/SKILL_AUDIT.md can refer to it.
The script is read-only: it never writes to a database or to the client folder.
"""
import argparse, csv, os, re, subprocess, sys
from collections import defaultdict, Counter

# --------------------------------------------------------------------------------------------
# Column maps: client table column index -> server column name (established empirically, every
# mapped pair agrees on >= 99% of the rows of the live data, see docs/SKILL_AUDIT.md).
# --------------------------------------------------------------------------------------------
MAIN_MAP = {11: "flying_effect", 12: "target_effect", 14: "moral", 15: "skill_level", 16: "skill",
            17: "msp", 18: "hp", 21: "use_item", 22: "cast_time", 23: "recast_time",
            27: "success_rate", 28: "type1", 29: "etc", 30: "range", 32: "use_standing"}
SUB_MAP = {
    1: ("Skill_Magic_1", "magic_type1", {1: "hit_type", 2: "hit_rate", 3: "hit", 4: "add_damage", 6: "combo_count",
                                         8: "range", 10: "add_dmg_perc_to_user", 11: "add_dmg_perc_to_npc"}),
    2: ("Skill_Magic_2", "magic_type2", {1: "hit_type", 2: "hit_rate", 3: "add_damage", 4: "add_range", 5: "need_arrow"}),
    3: ("Skill_Magic_3", "magic_type3", {1: "radius", 2: "direct_type", 3: "first_damage", 4: "time_damage",
                                         5: "duration", 6: "attribute"}),
    4: ("Skill_Magic_4", "magic_type4", {1: "buff_type", 2: "radius", 3: "duration", 4: "attack_speed", 5: "speed",
                                         6: "ac", 7: "ac_pct", 8: "attack", 9: "magic_attack", 10: "max_hp",
                                         11: "max_hp_pct", 12: "max_mp", 13: "max_mp_pct", 14: "str", 15: "sta",
                                         16: "dex", 17: "intel", 18: "cha", 19: "fire_r", 20: "cold_r",
                                         21: "lightning_r", 22: "magic_r", 23: "disease_r", 24: "poison_r",
                                         25: "exp_pct", 26: "special_amount"}),
    5: ("Skill_Magic_5", "magic_type5", {1: "type", 2: "exp_recover", 3: "need_stone"}),
    6: ("Skill_Magic_6", "magic_type6", {3: "size", 4: "transform_id", 5: "duration", 6: "max_hp", 7: "max_mp",
                                         8: "speed", 9: "attack_speed", 10: "total_hit", 11: "total_ac",
                                         21: "class", 22: "user_skill_use", 23: "need_item", 24: "skill_success_rate"}),
    7: ("Skill_Magic_7", "magic_type7", {}),
    8: ("Skill_Magic_8", "magic_type8", {1: "warp_type"}),
    9: ("skill_magic_9", "magic_type9", {3: "valid_group", 4: "nation_change", 5: "monster_num", 6: "target_change",
                                        7: "state_change", 8: "radius", 9: "hit_rate", 10: "duration"}),
}
SUB_KEY = {7: "n_index"}  # every other detail table is keyed by i_num

# Class constants (sSkill / 10) -> human readable
CLASS_NAMES = {
    101: "Karus Warrior(beginner)", 102: "Karus Rogue(beginner)", 103: "Karus Mage(beginner)", 104: "Karus Priest(beginner)",
    105: "Berserker", 106: "Guardian", 107: "Hunter", 108: "Penetrator", 109: "Sorcerer", 110: "Necromancer",
    111: "Shaman", 112: "Darkpriest", 113: "Kurian(beginner)", 114: "Kurian(novice)", 115: "Kurian(master)",
    201: "El Morad Warrior(beginner)", 202: "El Morad Rogue(beginner)", 203: "El Morad Mage(beginner)",
    204: "El Morad Priest(beginner)", 205: "Blade", 206: "Protector", 207: "Ranger", 208: "Assassin", 209: "Mage",
    210: "Enchanter", 211: "Cleric", 212: "Druid", 213: "Porutu(beginner)", 214: "Porutu(novice)", 215: "Porutu(master)",
}

MORAL = {1: "friend", 2: "friend_withme", 3: "friend_exceptme", 4: "party", 5: "npc", 6: "party_all", 7: "enemy",
         8: "all", 10: "area_enemy", 11: "area_friend", 12: "area_all", 13: "self_area", 14: "self"}


def load_csv(path):
    rows = []
    with open(path, newline="", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            d = {}
            for k, v in r.items():
                if v == "":
                    d[k] = None
                else:
                    try:
                        d[k] = int(v)
                    except ValueError:
                        d[k] = v
            rows.append(d)
    return rows


def parse_dump(text):
    rows = []
    for ln in text.split("\n")[1:]:
        if not ln.startswith("row="):
            continue
        cells = []
        for p in re.split(r" \| (?=c\d+=)", ln)[1:]:
            v = p.split("=", 1)[1]
            if v.startswith('"'):
                cells.append(v[1:-1])
            else:
                try:
                    cells.append(int(v))
                except ValueError:
                    try:
                        cells.append(float(v))
                    except ValueError:
                        cells.append(v)
        rows.append(cells)
    return rows


def load_client(tbl_dump, data_dir, name):
    out = subprocess.run([tbl_dump, os.path.join(data_dir, name + ".tbl"), "0", "1000000"],
                         capture_output=True).stdout.decode("utf-8", "replace")
    return parse_dump(out)


class Findings:
    def __init__(self, only, limit):
        self.items = defaultdict(list)
        self.only = only
        self.limit = limit

    def add(self, fid, msg):
        self.items[fid].append(msg)

    def dump(self):
        for fid in sorted(self.items):
            if self.only and fid not in self.only:
                continue
            lst = self.items[fid]
            print(f"[{fid}] {len(lst)} finding(s)")
            for m in lst[: self.limit]:
                print("    " + m)
            if len(lst) > self.limit:
                print(f"    ... {len(lst) - self.limit} more")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--csv-dir", required=True)
    ap.add_argument("--client-data")
    ap.add_argument("--tbl-dump")
    ap.add_argument("--client-main", default="Skill_Magic_Main_us")
    ap.add_argument("--only", default="")
    ap.add_argument("--limit", type=int, default=15)
    a = ap.parse_args()
    F = Findings(set(filter(None, a.only.split(","))), a.limit)

    magic = {r["magic_num"]: r for r in load_csv(os.path.join(a.csv_dir, "magic.csv"))}
    sub = {}
    for t in range(1, 10):
        key = SUB_KEY.get(t, "i_num")
        sub[t] = {r[key]: r for r in load_csv(os.path.join(a.csv_dir, f"magic_type{t}.csv"))}
    items = {}
    ip = os.path.join(a.csv_dir, "item.csv")
    if os.path.exists(ip):
        items = {r["num"]: r for r in load_csv(ip)}

    # ---------------------------------------------------------------- S: structure of server data
    for n, m in sorted(magic.items()):
        if m["type1"] is None or m["moral"] is None:
            F.add("S4", f"{n} {m['en_name']!r}: magic row has NULL numeric columns (skill can never be cast)")
        t1, t2, etc = m["type1"] or 0, m["type2"] or 0, m["etc"] or 0
        if t1 and t1 not in sub:
            F.add("S1", f"{n} {m['en_name']!r}: type1={t1} is not a known type")
        elif t1 and n not in sub[t1]:
            F.add("S1", f"{n} {m['en_name']!r}: type1={t1} but magic_type{t1} has no row")
        if t2 and t2 in sub and n not in sub[t2]:
            F.add("S1", f"{n} {m['en_name']!r}: type2={t2} but magic_type{t2} has no row")
        if etc in (3, 4, 9) and t1 in (1, 2, 3) and n not in sub[etc]:
            F.add("S1b", f"{n} {m['en_name']!r}: etc={etc} (secondary effect) but magic_type{etc} has no row")
        if not t1 and not t2 and (m["skill"] or 0):
            F.add("S3", f"{n} {m['en_name']!r}: class skill {m['skill']} with type1=0 (nothing executes)")
    for t in range(1, 10):
        for k in sub[t]:
            if k not in magic:
                F.add("S2", f"magic_type{t} row {k} has no magic row")
            else:
                m = magic[k]
                if (m["type1"] or 0) != t and (m["type2"] or 0) != t and (m["etc"] or 0) != t:
                    F.add("S2b", f"magic_type{t} row {k} {m['en_name']!r} unused (magic.type1={m['type1']} type2={m['type2']} etc={m['etc']})")

    # ---------------------------------------------------------------- V: value sanity
    for n, m in sorted(magic.items()):
        name = m["en_name"]
        for col in ("cast_time", "recast_time", "msp", "hp", "range", "success_rate"):
            if (m[col] or 0) < 0:
                F.add("V1", f"{n} {name!r}: negative {col}={m[col]}")
        ui = m["use_item"] or 0
        if ui and items and ui not in items:
            F.add("V2", f"{n} {name!r}: use_item {ui} does not exist in item table")
        sk = m["skill"] or 0
        if sk:
            cls = sk // 10
            if 101 <= cls <= 115 or 201 <= cls <= 215:
                if n < 300000 and n // 100000 != cls // 100:
                    F.add("V3", f"{n} {name!r}: id nation {n//100000} != class nation {cls//100} (sSkill={sk})")
                if n < 300000 and n // 1000 != cls:
                    F.add("V4", f"{n} {name!r}: id prefix {n//1000} != sSkill/10={cls} ({CLASS_NAMES.get(cls)})")
            if (m["skill_level"] or 0) > 83 and sk % 10 == 0:
                F.add("V5", f"{n} {name!r}: required level {m['skill_level']} > 83")
        # type specific
        t3 = sub[3].get(n)
        if t3 and m["type1"] == 3:
            d, td, fd = t3["duration"] or 0, t3["time_damage"] or 0, t3["first_damage"] or 0
            if td and not d:
                F.add("V6", f"{n} {name!r}: type3 time_damage={td} but duration=0")
            if d and not td and (t3["direct_type"] or 0) != 1 and not fd:
                F.add("V6b", f"{n} {name!r}: type3 duration={d} but no damage at all")
            if d < 0 or (t3["radius"] or 0) < 0:
                F.add("V1", f"{n} {name!r}: negative type3 duration/radius")
            if (t3["direct_type"] or 0) == 1 and fd > 0 and (m["moral"] or 0) in (7, 10) and m["skill"]:
                F.add("V7", f"{n} {name!r}: type3 heal-direction (+{fd}) but enemy moral {m['moral']}")
        t4 = sub[4].get(n)
        if t4 and m["type1"] == 4:
            if (t4["duration"] or 0) < 0:
                F.add("V1", f"{n} {name!r}: negative type4 duration")
            if (t4["buff_type"] or 0) == 0:
                F.add("V8", f"{n} {name!r}: type4 buff_type=0")
        t1r = sub[1].get(n)
        if t1r and m["type1"] == 1:
            if (t1r["hit"] or 0) == 0 and (t1r["add_damage"] or 0) == 0 and not (m["etc"] or 0):
                F.add("V9", f"{n} {name!r}: type1 with hit=0 and add_damage=0")
        t2r = sub[2].get(n)
        if t2r and m["type1"] == 2 and (t2r["need_arrow"] or 0) > 0 and not ui:
            F.add("V10", f"{n} {name!r}: type2 need_arrow={t2r['need_arrow']} but use_item=0")
        t6 = sub[6].get(n)
        if t6 and m["type1"] == 6:
            if (t6["duration"] or 0) <= 0:
                F.add("V11", f"{n} {name!r}: type6 duration={t6['duration']}")
            if (t6["transform_id"] or 0) == 0:
                F.add("V11", f"{n} {name!r}: type6 transform_id=0")

    # ---------------------------------------------------------------- N: Karus / El Morad twins
    cmp_cols = ["t_1", "moral", "skill_level", "msp", "hp", "s_sp", "cast_time", "recast_time", "success_rate",
                "type1", "type2", "range", "etc", "use_standing", "item_group"]
    for n, m in sorted(magic.items()):
        if 100000 <= n < 200000 and (m["skill"] or 0):
            tw = magic.get(n + 100000)
            if tw is None:
                F.add("N1", f"{n} {m['en_name']!r}: no El Morad twin {n+100000}")
                continue
            if (tw["skill"] or 0) // 10 != (m["skill"] // 10) + 100 or (tw["skill"] or 0) % 10 != m["skill"] % 10:
                F.add("N2", f"{n}/{n+100000}: sSkill {m['skill']} vs {tw['skill']}")
            for c in cmp_cols:
                if m[c] != tw[c]:
                    F.add("N3", f"{n}/{n+100000} {m['en_name']!r}: {c} differs {m[c]} vs {tw[c]}")
            for t in (1, 2, 3, 4, 5, 6, 9):
                a1, b1 = sub[t].get(n), sub[t].get(n + 100000)
                if (a1 is None) != (b1 is None):
                    F.add("N4", f"{n}/{n+100000}: magic_type{t} row present only on one side")
                elif a1:
                    for c in a1:
                        if c in ("i_num", "name", "description", "buff_type") or a1[c] == b1[c]:
                            continue
                        F.add("N5", f"{n}/{n+100000} {m['en_name']!r}: type{t}.{c} differs {a1[c]} vs {b1[c]}")
        elif 200000 <= n < 300000 and (m["skill"] or 0) and (n - 100000) not in magic:
            F.add("N1", f"{n} {m['en_name']!r}: no Karus twin {n-100000}")

    # ---------------------------------------------------------------- C: client vs server
    if a.client_data and a.tbl_dump:
        cm = {r[0]: r for r in load_client(a.tbl_dump, a.client_data, a.client_main)}
        for n in sorted(set(magic) - set(cm)):
            F.add("C1", f"{n} {magic[n]['en_name']!r}: in server DB but not in client {a.client_main}")
        for n in sorted(set(cm) - set(magic)):
            F.add("C2", f"{n} {cm[n][2]!r}: in client {a.client_main} but not in server DB")
        for n, c in sorted(cm.items()):
            m = magic.get(n)
            if not m:
                continue
            for ci, col in MAIN_MAP.items():
                if (m[col] or 0) != c[ci]:
                    F.add("C3", f"{n} {m['en_name']!r}: {col} server={m[col]} client={c[ci]}")
        for t, (cn, sn, mp) in SUB_MAP.items():
            crow = {r[0]: r for r in load_client(a.tbl_dump, a.client_data, cn)}
            srow = sub[t]
            for k in sorted(set(crow) - set(srow)):
                F.add("C4", f"{cn} row {k}: client only (no {sn} row on server)")
            for k in sorted(set(srow) - set(crow)):
                F.add("C5", f"{sn} row {k}: server only (client {cn} has no row)")
            for k, c in sorted(crow.items()):
                s = srow.get(k)
                if not s:
                    continue
                for ci, col in mp.items():
                    if (s[col] or 0) != c[ci]:
                        F.add("C6", f"{sn} {k}: {col} server={s[col]} client={c[ci]}")
        # client Main rows whose type1 detail row is missing in the client sub table
        csub = {t: {r[0] for r in load_client(a.tbl_dump, a.client_data, SUB_MAP[t][0])} for t in SUB_MAP}
        for n, c in sorted(cm.items()):
            t1 = c[28]
            if t1 in csub and n not in csub[t1]:
                F.add("C7", f"{n} {c[2]!r}: client Main type1={t1} but {SUB_MAP[t1][0]} has no row")
    F.dump()


if __name__ == "__main__":
    main()
