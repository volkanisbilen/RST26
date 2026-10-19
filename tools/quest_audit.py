#!/usr/bin/env python3
"""Static quest audit: every quest Lua script + the quest data tables.

It answers "which quests cannot work, and why" WITHOUT running the game:

  * every call that hands out / takes / checks an item is checked against the
    item table (unknown ids, ids that do not fit 32 bits, impossible counts,
    virtual ids used as real items, wrong argument shape, ...)
  * every RunQuestExchange / CheckExchange / ... id is looked up in item_exchange,
    and the exchange row itself is validated (unknown items, MISALIGNED rows
    whose columns are shifted by one, ...)
  * every SaveEvent / GetQuestStatus / CountMonsterQuestSub id is looked up in
    quest_helper / quest_monster
  * every quest_helper row is checked against the Lua files (file present, the
    trigger / complete EVENT has a handler, the NPC exists and is spawned, the
    exchange index exists)
  * dead dialog buttons (a SelectMsg button or `EVENT = N` jump with no handler)
    and handlers that sit BEFORE the jump that should reach them

Data comes from CSV snapshots of the database tables (so the tool needs neither
a database driver nor a running server):

    python tools/quest_audit.py --data-dir /tmp/quest_audit_data --dump     # needs psql + $DATABASE_URL (read-only SELECTs)
    python tools/quest_audit.py --data-dir /tmp/quest_audit_data            # audit, writes docs/QUEST_AUDIT.md

Options: --quests DIR (default ./Quests), --bindings FILE (default
crates/ko-game/src/lua_engine/bindings.rs, used to learn which Lua functions
exist), --out FILE, --json FILE (machine readable findings), --fail-on-error.
"""
from __future__ import annotations

import argparse
import collections
import csv
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import quest_lua  # noqa: E402

csv.field_size_limit(1 << 24)

# Virtual item ids (item.countable == 2): they are handled by the exchange code,
# they are not inventory items.
VIRTUAL = {
    810000000: "event marker", 900000000: "Coin", 900001000: "EXP",
    900002000: "Country CONT (NP)", 900003000: "Ladder Point", 900004000: "Random",
    900005000: "Hunt (kill counter)", 900006000: "Job change", 900007000: "Skill",
    900008000: "Kill opponent country", 900009000: "Transport", 900010000: "Level up",
    900011000: "War", 900012000: "Chat", 900016000: "Premium XP", 900020000: "Cash",
}
ITEMCOUNT_MAX = 9999
U32_MAX = 4294967295

# functions whose argument #1 (0-based, after UID) is an item id, and where the
# count argument sits.
ITEM_FUNCS = {
    "HowmuchItem": (1, None), "CheckExistItem": (1, 2), "GiveItem": (1, 2),
    "RobItem": (1, 2), "isRoomForItem": (1, 2), "CheckWeight": (1, 2),
}
EXCHANGE_FUNCS = {
    "RunQuestExchange", "RunExchange", "RunGiveItemExchange", "CheckExchange",
    "RunCountExchange", "RunRandomExchange", "GetMaxExchange",
}
# exchange ids of these live in item_give_exchange instead of item_exchange
GIVE_EXCHANGE_FUNCS = {"RunGiveItemExchange"}

LUA_BUILTINS = {
    "print", "tostring", "tonumber", "pairs", "ipairs", "type", "select", "error",
    "pcall", "assert", "unpack", "next", "require", "setmetatable", "getmetatable",
    "rawget", "rawset", "math", "string", "table", "os", "if", "elseif", "while",
    "for", "function", "and", "or", "not", "return", "until",
}

SEV_ORDER = {"ERROR": 0, "WARN": 1, "INFO": 2}

DUMP_QUERIES = {
    "item": "select num, str_name, kind, countable, weight, duration, race, class, req_level from item order by num",
    "item_exchange": "select * from item_exchange order by n_index",
    "item_exchange_exp": "select * from item_exchange_exp order by n_index",
    "item_give_exchange": "select exchange_index, rob_item_ids::text as rob_item_ids, rob_item_counts::text as rob_item_counts, give_item_ids::text as give_item_ids, give_item_counts::text as give_item_counts from item_give_exchange order by exchange_index",
    "quest_helper": "select * from quest_helper order by n_index",
    "quest_monster": "select * from quest_monster order by s_quest_num",
    "npc_template": "select s_sid, is_monster, str_name, s_pid, by_type, s_level from npc_template order by s_sid, is_monster",
    "npc_spawn": "select npc_id, zone_id, is_monster, count(*) as spawns from npc_spawn group by 1,2,3 order by 1,2",
    # round 3 (client-vs-server checks, see tools/quest_client_sync.py)
    "npc_spawn_full": "select id,zone_id,npc_id,is_monster,act_type,regen_type,dungeon_family,special_type,trap_number,left_x,top_z,num_npc,spawn_range,regen_time,direction,dot_cnt,room,gm_added from npc_spawn where is_monster=false order by id",
    "monster_item": "select * from monster_item order by s_index",
    "monster_tmpl": "select s_sid,is_monster,str_name,s_item,s_level from npc_template where is_monster=true order by s_sid",
}


def dump(data_dir: Path) -> None:
    url = os.environ.get("DATABASE_URL")
    if not url:
        sys.exit("DATABASE_URL is not set")
    data_dir.mkdir(parents=True, exist_ok=True)
    for name, query in DUMP_QUERIES.items():
        target = data_dir / f"{name}.csv"
        cmd = ["psql", url, "-v", "ON_ERROR_STOP=1", "-c",
               f"\\copy ({query}) to '{target.as_posix()}' with csv header"]
        subprocess.run(cmd, check=True)
        print("dumped", target)


def read_csv(path: Path):
    with path.open(encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh))


def parse_pg_array(text: str):
    text = text.strip()
    if text.startswith("{") and text.endswith("}"):
        text = text[1:-1]
    return [int(x) for x in text.split(",") if x.strip() not in ("", "NULL")]


class Data:
    def __init__(self, d: Path):
        self.items = {int(r["num"]): r for r in read_csv(d / "item.csv")}
        self.exchange = {int(r["n_index"]): r for r in read_csv(d / "item_exchange.csv")}
        self.exchange_exp = {int(r["n_index"]): r for r in read_csv(d / "item_exchange_exp.csv")}
        self.give_exchange = {}
        p = d / "item_give_exchange.csv"
        if p.exists():
            for r in read_csv(p):
                self.give_exchange[int(r["exchange_index"])] = r
        self.helpers = read_csv(d / "quest_helper.csv")
        for r in self.helpers:
            for k, v in list(r.items()):
                if k != "str_lua_filename":
                    r[k] = int(v) if v not in ("", None) else 0
        self.monsters = {int(r["s_quest_num"]): r for r in read_csv(d / "quest_monster.csv")}
        self.npcs: dict[int, list[dict]] = collections.defaultdict(list)
        for r in read_csv(d / "npc_template.csv"):
            self.npcs[int(r["s_sid"])].append(r)
        self.spawned: dict[int, set] = collections.defaultdict(set)
        for r in read_csv(d / "npc_spawn.csv"):
            self.spawned[int(r["npc_id"])].add((int(r["zone_id"]), r["is_monster"] in ("t", "true", "True", "1")))
        self.helper_by_index = {r["n_index"]: r for r in self.helpers}
        self.quest_ids = {r["s_event_data_index"] for r in self.helpers}

    def item_ok(self, i: int) -> bool:
        return i == 0 or i in self.items or i in VIRTUAL


def exchange_shifted(r) -> bool:
    """Rows imported from the 27-column client Item_Exchange.tbl straight into the
    26-column server table are shifted one column to the right: the first
    origin 'item' is 0 and its 'count' holds the real item id."""
    return int(r["origin_item_num1"]) == 0 and int(r["origin_item_count1"]) >= 100000
    # (a genuine row never has count1 that large with num1 == 0)


class Finding:
    __slots__ = ("code", "sev", "file", "line", "event", "detail", "quests")

    def __init__(self, code, sev, file, line, event, detail):
        self.code, self.sev, self.file, self.line, self.event, self.detail = code, sev, file, line, event, detail
        self.quests: set[int] = set()

    def as_dict(self):
        return {"code": self.code, "sev": self.sev, "file": self.file, "line": self.line,
                "event": self.event, "detail": self.detail, "quests": sorted(self.quests)}


def load_registered_functions(bindings: Path) -> set[str]:
    src = bindings.read_text(encoding="utf-8")
    start = src.index("pub fn register_all")
    end = src.index("\nfn lua_", start)
    return set(re.findall(r'g\.set\(\s*"([A-Za-z0-9_]+)"', src[start:end]))


_INT_RANGES = {
    "i8": (-(1 << 7), (1 << 7) - 1), "u8": (0, (1 << 8) - 1),
    "i16": (-(1 << 15), (1 << 15) - 1), "u16": (0, (1 << 16) - 1),
    "i32": (-(1 << 31), (1 << 31) - 1), "u32": (0, (1 << 32) - 1),
    "i64": (-(1 << 63), (1 << 63) - 1), "u64": (0, (1 << 64) - 1),
}


def _split_top(text: str) -> list[str]:
    out, depth, cur = [], 0, ""
    for ch in text:
        if ch in "(<[":
            depth += 1
        elif ch in ")>]":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def load_binding_signatures(bindings: Path) -> dict:
    """Lua global name -> list of Rust parameter types (the first one is the
    player id), or None when the binding is variadic (`LuaMultiValue`)."""
    src = bindings.read_text(encoding="utf-8")
    start = src.index("pub fn register_all")
    end = src.index("\nfn lua_", start)
    reg = {}
    pat = r'g\.set\(\s*"([A-Za-z0-9_]+)"\s*,\s*lua\.create_function\(\s*([A-Za-z0-9_]+)\s*\)'
    for m in re.finditer(pat, src[start:end]):
        reg[m.group(1)] = m.group(2)
    sigs = {}
    for lua_name, rust_name in reg.items():
        m = re.search(r"\nfn " + re.escape(rust_name) + r"\(\s*_?lua: &Lua,\s*(.*?)\)\s*->", src, re.S)
        if not m:
            continue
        params = m.group(1).strip().rstrip(",")
        if not params:
            sigs[lua_name] = []
        elif params.startswith("("):
            tm = re.match(r"\(([^)]*)\)\s*:\s*\((.*)\)\s*$", params, re.S)
            sigs[lua_name] = _split_top(tm.group(2)) if tm else None
        else:
            ty = params.split(":", 1)[1].strip() if ":" in params else ""
            sigs[lua_name] = None if "LuaMultiValue" in ty else [ty]
    return sigs


def check_call_types(c, sig, env):
    """Yield (severity, message) for arguments the binding would reject (mlua raises a
    conversion error and the whole script run is aborted)."""
    if sig is None:
        return
    required = 0
    for t in sig:
        if t.startswith("Option<"):
            break
        required += 1
    if len(c.args) < required:
        yield ("ERROR", f"`{c.name}` needs {required} argument(s) (has {len(c.args)}): signature "
                        f"({', '.join(sig)}); the call raises a conversion error and aborts the script")
        return
    for i, t in enumerate(sig):
        if i >= len(c.args):
            break
        base = t[7:-1] if t.startswith("Option<") else t
        a = c.args[i]
        if base in _INT_RANGES:
            lo, hi = _INT_RANGES[base]
            if len(a) == 1 and a[0].kind == "str":
                yield ("ERROR", f"`{c.name}` argument {i + 1} is the string \"{a[0].val}\" but the binding wants {base}")
                continue
            vals = c.arg_const(i, env)
            lit = (len(a) == 1 and a[0].kind == "num") or (len(a) == 2 and a[0].val == "-" and a[1].kind == "num")
            if vals is None and lit:
                yield ("ERROR", f"`{c.name}` argument {i + 1} is not an integer ({' '.join(x.val for x in a)})")
            for v in sorted(vals or ()):
                if not lo <= v <= hi:
                    yield ("ERROR" if lit else "WARN",
                           f"`{c.name}` argument {i + 1} = {v} does not fit {base} ({lo}..{hi}): the call raises a "
                           f"conversion error and aborts the script")


def exchange_rows_problems(data: Data, ex_id: int, quest_use: bool = True):
    """Return list of (kind, detail) problems of one item_exchange row."""
    r = data.exchange.get(ex_id)
    if r is None:
        return [("missing", f"exchange {ex_id} not in item_exchange")]
    out = []
    if exchange_shifted(r):
        out.append(("shifted", "row is shifted one column (origin item 1 = 0, its count = %s)" % r["origin_item_count1"]))
        return out
    for k in range(1, 6):
        o, oc = int(r[f"origin_item_num{k}"]), int(r[f"origin_item_count{k}"])
        e, ec = int(r[f"exchange_item_num{k}"]), int(r[f"exchange_item_count{k}"])
        if o and oc and not data.item_ok(o):
            out.append(("origin_item", f"origin item {o} (x{oc}) not in item table"))
        if e and ec and not data.item_ok(e):
            out.append(("output_item", f"reward item {e} (x{ec}) not in item table"))
        if (o and not oc) or (e and not ec):
            if r["random_flag"] not in ("101",):
                out.append(("zero_count", f"slot {k} has an item but count 0"))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-dir", required=True, type=Path)
    ap.add_argument("--dump", action="store_true", help="(re)create the CSV snapshots from $DATABASE_URL first")
    ap.add_argument("--quests", type=Path, default=Path("Quests"))
    ap.add_argument("--bindings", type=Path, default=Path("crates/ko-game/src/lua_engine/bindings.rs"))
    ap.add_argument("--out", type=Path, default=Path("docs/QUEST_AUDIT.md"))
    ap.add_argument("--json", type=Path)
    ap.add_argument("--client-exchange", type=Path,
                    help="output of `tbl_dump docs/data/Item_Exchange.tbl 0 100000`; enables EXCHANGE_CLIENT_ITEMS "
                         "(reward items the client table lists but the server row lost)")
    ap.add_argument("--client-dump", type=Path,
                    help="directory with tbl_dump text dumps of the CLIENT tables (Quest_Helper.txt, Quest_Monster_Exchange.txt, "
                         "Item_Exchange.txt, NpcMopMap_Info.txt): enables the client-vs-server checks CLIENT_*")
    ap.add_argument("--fail-on-error", action="store_true")
    ap.add_argument("--max-detail", type=int, default=400, help="max rows per table in the report")
    args = ap.parse_args()

    if args.dump:
        dump(args.data_dir)
    data = Data(args.data_dir)
    registered = load_registered_functions(args.bindings)
    signatures = load_binding_signatures(args.bindings)

    scripts: dict[str, quest_lua.LuaScript] = {}
    for p in sorted(args.quests.glob("*.lua")):
        scripts[p.name] = quest_lua.parse(p.name, p.read_text(encoding="utf-8", errors="replace"))

    findings: list[Finding] = []

    def add(code, sev, file, line, event, detail):
        f = Finding(code, sev, file, line, event, detail)
        findings.append(f)
        return f

    # ---- helper rows per file --------------------------------------------------
    rows_by_file: dict[str, list[dict]] = collections.defaultdict(list)
    for r in data.helpers:
        rows_by_file[r["str_lua_filename"]].append(r)

    # event graph per script: event -> set(events it can jump to)
    graphs: dict[str, dict[int, set[int]]] = {}
    handler_events: dict[str, set[int]] = {}
    for name, sc in scripts.items():
        g: dict[int, set[int]] = collections.defaultdict(set)
        evs: set[int] = set()
        for h in sc.handlers:
            evs.update(h.events)
            for e in h.events:
                g[e].update(h.reassign)
            for c in h.calls:
                if c.name == "SelectMsg":
                    # SelectMsg(UID, flag, quest, header, NPC, text1, event1, text2, event2, ...)
                    for k in range(6, len(c.args), 2):
                        v = c.arg_const(k, sc.env)
                        if v:
                            for e in v:
                                if e >= 0:
                                    for ev in h.events:
                                        g[ev].add(e)
        graphs[name] = g
        handler_events[name] = evs

    def reach(name, start):
        g = graphs.get(name, {})
        seen, stack = set(), [start]
        while stack:
            e = stack.pop()
            if e in seen:
                continue
            seen.add(e)
            stack.extend(g.get(e, ()))
        return seen

    quest_events: dict[str, list[tuple[int, set[int]]]] = collections.defaultdict(list)
    for name, rows in rows_by_file.items():
        for r in rows:
            reach_set: set[int] = set()
            for ev in (r["n_event_trigger_index"], r["n_event_complete_index"]):
                if ev:
                    reach_set |= reach(name, ev)
            quest_events[name].append((r["s_event_data_index"], reach_set))

    def attach_quests(f: Finding):
        if f.event is None:
            return
        for q, rs in quest_events.get(f.file, []):
            if f.event in rs:
                f.quests.add(q)

    # ---- per-script checks -------------------------------------------------------
    used_exchanges: dict[int, list[tuple[str, int, int | None, str]]] = collections.defaultdict(list)
    for name, sc in scripts.items():
        local_funcs = {sc.toks[i + 1].val for i in range(len(sc.toks) - 1)
                       if sc.toks[i].kind == "kw" and sc.toks[i].val == "function" and sc.toks[i + 1].kind == "id"}
        npc_from_name = None
        m = re.match(r"(\d+)_", name)
        if m:
            npc_from_name = int(m.group(1))
        for c in sc.calls:
            if c.name not in registered and c.name not in LUA_BUILTINS and c.name not in local_funcs:
                add("UNBOUND_FUNCTION", "ERROR", name, c.line, c.event,
                    f"`{c.name}(...)` is not a registered Lua binding - the script aborts at this call")
            if c.name in signatures:
                for sev_, msg_ in check_call_types(c, signatures[c.name], sc.env):
                    add("BAD_ARGS", sev_, name, c.line, c.event, msg_)
            if c.name in ITEM_FUNCS:
                ii, ci = ITEM_FUNCS[c.name]
                if len(c.args) <= ii:
                    add("BAD_ARGS", "ERROR", name, c.line, c.event,
                        f"`{c.name}` called with {len(c.args)} argument(s); it needs (UID, item_id{', count' if ci else ''}) - "
                        f"args: `{', '.join(c.arg_text(k) for k in range(len(c.args)))}`")
                    continue
                if c.arg_text(0) != "UID":
                    add("BAD_ARGS", "ERROR", name, c.line, c.event, f"`{c.name}` first argument is `{c.arg_text(0)}`, not UID")
                ids = c.arg_const(ii, sc.env)
                if ids is None:
                    continue
                counts = c.arg_const(ci, sc.env) if ci is not None else None
                for i in sorted(ids):
                    if i > U32_MAX or i < 0:
                        add("BAD_ITEM_ID", "ERROR", name, c.line, c.event,
                            f"`{c.name}` item id {i} does not fit in 32 bits (the engine would wrap it to {i & 0xFFFFFFFF})")
                    elif i == 0:
                        add("BAD_ITEM_ID", "WARN", name, c.line, c.event, f"`{c.name}` item id is 0 (placeholder)")
                    elif i in VIRTUAL:
                        if c.name in ("GiveItem", "RobItem"):
                            add("VIRTUAL_ITEM", "ERROR", name, c.line, c.event,
                                f"`{c.name}` with virtual id {i} ({VIRTUAL[i]}) - the engine cannot give/take it; use GoldGain/ExpChange/...")
                    elif i not in data.items:
                        add("ITEM_MISSING", "ERROR", name, c.line, c.event,
                            f"`{c.name}` item id {i} is not in the item table")
                    else:
                        it = data.items[i]
                        if counts and c.name in ("GiveItem", "RobItem"):
                            for n_ in sorted(counts):
                                if n_ <= 0 or n_ > ITEMCOUNT_MAX:
                                    add("BAD_COUNT", "ERROR", name, c.line, c.event,
                                        f"`{c.name}` item {i} ({it['str_name']}) count {n_} is outside 1..{ITEMCOUNT_MAX}")
                                elif n_ > 1 and it["countable"] == "0" and it["kind"] != "255":
                                    add("BAD_COUNT", "WARN", name, c.line, c.event,
                                        f"`{c.name}` item {i} ({it['str_name']}) x{n_}: the item is not stackable, "
                                        f"{'only 1 is given' if c.name == 'GiveItem' else 'a single slot never holds that many'}")
                        if c.name == "GiveItem" and len(c.args) > 3:
                            d = c.arg_const(3, sc.env)
                            if d and any(x < 0 or x > 3650 for x in d):
                                add("BAD_COUNT", "WARN", name, c.line, c.event, f"`GiveItem` expiry days {sorted(d)} look wrong")
            elif c.name in EXCHANGE_FUNCS:
                if len(c.args) < 2:
                    add("BAD_ARGS", "ERROR", name, c.line, c.event, f"`{c.name}` needs (UID, exchange_id)")
                    continue
                ids = c.arg_const(1, sc.env)
                if ids is None:
                    continue
                for i in sorted(ids):
                    used_exchanges[i].append((name, c.line, c.event, c.name))
                    table = data.give_exchange if c.name in GIVE_EXCHANGE_FUNCS else data.exchange
                    if i not in table:
                        if c.name == "CheckExchange" and i == 0:
                            continue
                        add("EXCHANGE_MISSING", "ERROR", name, c.line, c.event,
                            f"`{c.name}({i})`: exchange {i} does not exist in {'item_give_exchange' if c.name in GIVE_EXCHANGE_FUNCS else 'item_exchange'}"
                            f" - nothing is taken or given")
            elif c.name == "SaveEvent":
                ids = c.arg_const(1, sc.env)
                if ids:
                    for i in sorted(ids):
                        if i not in data.helper_by_index:
                            add("SAVEEVENT_UNKNOWN", "ERROR", name, c.line, c.event,
                                f"`SaveEvent(UID, {i})`: no quest_helper row {i} - the quest state never changes")
            elif c.name in ("GetQuestStatus", "CheckExistEvent", "QuestCheckExistEvent", "QuestCheckFinished", "CountMonsterQuestSub"):
                ids = c.arg_const(1, sc.env)
                if ids:
                    for i in sorted(ids):
                        if i not in data.quest_ids and i not in (0, -1):
                            add("QUEST_ID_UNKNOWN", "WARN", name, c.line, c.event,
                                f"`{c.name}` quest id {i} is not the event id of any quest_helper row")
                        if c.name == "CountMonsterQuestSub":
                            grp = c.arg_const(2, sc.env)
                            mq = data.monsters.get(i)
                            if mq is None:
                                add("MONSTER_QUEST_MISSING", "WARN", name, c.line, c.event,
                                    f"`CountMonsterQuestSub` quest {i} has no quest_monster row - the count is always 0")
                            elif grp:
                                for g_ in sorted(grp):
                                    if not 1 <= g_ <= 4 or int(mq[f"s_count{g_}"]) == 0:
                                        add("MONSTER_QUEST_MISSING", "WARN", name, c.line, c.event,
                                            f"`CountMonsterQuestSub` quest {i} group {g_} has no monsters/count in quest_monster")

        # dead event targets / ordering
        hev = handler_events[name]
        uses_other_event_compare = any(
            sc.toks[i].kind == "id" and sc.toks[i].val == "EVENT" and sc.toks[i + 1].val in (">=", "<=", ">", "<", "~=")
            for i in range(len(sc.toks) - 1))
        positions: dict[int, list[int]] = collections.defaultdict(list)
        for h in sc.handlers:
            for e in h.events:
                positions[e].append(h.start)
        for h in sc.handlers:
            for tgt in h.reassign:
                if tgt not in hev and not uses_other_event_compare:
                    f = add("EVENT_NO_HANDLER", "ERROR", name, h.line, h.events[0],
                            f"`EVENT = {tgt}` jumps to an event that has no handler in this script")
                elif tgt in positions and all(p < h.start for p in positions[tgt]) and tgt not in h.events:
                    add("EVENT_ORDER", "ERROR", name, h.line, h.events[0],
                        f"`EVENT = {tgt}`: every handler for {tgt} is above this block, so Lua never reaches it "
                        f"(handlers run top to bottom)")
            for c in h.calls:
                if c.name == "SelectMsg":
                    for k in range(6, len(c.args), 2):
                        v = c.arg_const(k, sc.env)
                        for e in (v or ()):
                            if e >= 0 and e not in hev and not uses_other_event_compare:
                                add("BUTTON_NO_HANDLER", "ERROR", name, c.line, h.events[0],
                                    f"SelectMsg button -> EVENT {e} has no handler in this script (dead button)")

    # ---- client Item_Exchange (optional) ---------------------------------------
    client_ex = {}
    ce_path = args.client_exchange if (args.client_exchange and args.client_exchange.exists()) else (
        args.client_dump / "Item_Exchange.txt" if args.client_dump else None)
    if ce_path and ce_path.exists():
        for line in ce_path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("row="):
                d_ = dict(x.strip().split("=", 1) for x in line.split("|")[1:])
                client_ex[int(d_["c0"])] = d_
    client_zero = {}
    for i_, c_ in client_ex.items():
        z_ = [k for k in range(5) if (int(c_[f"c{3 + 2 * k}"]) and not int(c_[f"c{4 + 2 * k}"]))
              or (int(c_[f"c{13 + 2 * k}"]) and not int(c_[f"c{14 + 2 * k}"]))]
        if z_:
            client_zero[i_] = z_

    # ---- exchange rows used by the scripts --------------------------------------
    exch_findings: dict[int, list] = {}
    for ex_id, uses in sorted(used_exchanges.items()):
        probs = exchange_rows_problems(data, ex_id)
        probs = [p for p in probs if p[0] != "missing"]
        if not probs:
            continue
        exch_findings[ex_id] = probs
        for (fname, line, ev, fn) in uses:
            for kind, detail in probs:
                sev = "ERROR"
                if kind == "zero_count":
                    sev = "INFO" if ex_id in client_zero else "WARN"   # the client row has the same empty-count slot
                add("EXCHANGE_" + kind.upper(), sev, fname, line, ev, f"`{fn}({ex_id})`: {detail}")

    # ---- reward items lost from the server rows ---------------------------------
    for ex_id, uses in sorted(used_exchanges.items()):
        r = data.exchange.get(ex_id)
        if r is None or exchange_shifted(r):
            continue
        outs = [(int(r[f"exchange_item_num{k}"]), int(r[f"exchange_item_count{k}"])) for k in range(1, 6)]
        has_exp_row = ex_id in data.exchange_exp
        has_origin = any(int(r[f"origin_item_num{k}"]) for k in range(1, 6))
        hunt_only = any(int(r[f"origin_item_num{k}"]) == 900005000 for k in range(1, 6))
        if has_origin and not any(a and c for a, c in outs) and not has_exp_row                 and not any(int(r[f"origin_item_num{k}"]) == 900012000 for k in range(1, 6)):
            for (fname, line, ev, fn) in uses:
                # a Hunt (kill counter) turn-in with no reward is a gate (teleport / chain step); the client row is empty too
                add("EXCHANGE_NO_OUTPUT", "INFO" if hunt_only else "ERROR", fname, line, ev,
                    f"`{fn}({ex_id})`: the row has no reward at all (all exchange_item columns empty) - "
                    f"the quest completes and pays nothing")
        c = client_ex.get(ex_id)
        if c:
            have = {a for a, _ in outs if a}
            lost = [int(c[f"c{13 + 2 * k}"]) for k in range(5)]
            lost = [a for a in lost if a and not (900000000 <= a < 900100000) and a in data.items and a not in have]
            if lost:
                for (fname, line, ev, fn) in uses:
                    add("EXCHANGE_CLIENT_ITEMS", "WARN", fname, line, ev,
                        f"`{fn}({ex_id})`: client Item_Exchange lists reward item(s) {lost} that the server row does not give")

    # ---- client vs server (round 3) ---------------------------------------------
    if args.client_dump:
        import types
        import quest_client_sync
        crep = quest_client_sync.run(types.SimpleNamespace(
            client_dump=args.client_dump, data_dir=args.data_dir, quests=args.quests, sql=None, after=None, report=None))
        for fn_, n_ in crep["A_helper_rows_added_by_file"].items():
            add("CLIENT_HELPER_MISSING", "WARN", fn_, 0, None,
                f"{n_} client Quest_Helper rows of `{fn_}` have no server row (quest menu / status rows the client expects)")
        for q_, new_, old_ in crep["B_quest_monster_updated_list"]:
            f_ = add("CLIENT_MONSTER_DIFF", "WARN", "(quest_monster)", 0, None,
                     f"quest {q_}: server kill targets/counts {old_[:5]}.. differ from the client {new_[:5]}..")
            f_.quests.add(q_)
        if crep["A_quest_monster_added"]:
            add("CLIENT_MONSTER_DIFF", "WARN", "(quest_monster)", 0, None,
                f"{crep['A_quest_monster_added']} client Quest_Monster_Exchange rows have no quest_monster row")
        if crep["B_helper_level_lowered_to_client"]:
            add("CLIENT_HELPER_LEVEL", "WARN", "(quest_helper)", 0, None,
                f"{crep['B_helper_level_lowered_to_client']} quest_helper rows demand a higher level than the client shows")
        for e_ in crep["B_exchange_rows_replaced_by_client"]:
            add("CLIENT_EXCHANGE_DIFF", "WARN", "(item_exchange)", 0, None,
                f"exchange {e_}: placeholder row of a client-only quest, the client row is different")
        for e_, m_ in crep["B_exchange_reward_items_appended"]:
            add("CLIENT_EXCHANGE_DIFF", "WARN", "(item_exchange)", 0, None,
                f"exchange {e_}: client lists reward item(s) {m_} the server row lacks")
        kept = crep["B_exchange_differences_kept"]
        if kept["count"]:
            add("CLIENT_EXCHANGE_DIFF", "INFO", "(item_exchange)", 0, None,
                f"{kept['count']} used exchange rows differ from the client on purpose (server design): {kept['by_reason']}")
        for t_, slot_, it_, rate_, q_ in crep["F_quest_item_drops_list"]:
            f_ = add("CLIENT_QUEST_ITEM_NO_DROP", "ERROR", "(monster_item)", 0, None,
                     f"quest {q_}: item {it_} is required by the exchange but no kill-target monster drops it (drop table {t_})")
            f_.quests.add(q_)
        if crep["D_npc_spawn_added_from_client_placement"]:
            add("CLIENT_NPC_PLACEMENT", "WARN", "(npc_spawn)", 0, None,
                f"{len(crep['D_npc_spawn_added_from_client_placement'])} quest NPCs have no npc_spawn row but the client "
                f"NpcMopMap_Info places them: {[x[1] for x in crep['D_npc_spawn_added_from_client_placement']]}")
        add("CLIENT_NPC_UNSPAWNED", "INFO", "(npc_spawn)", 0, None,
            f"{crep['D_unspawned_no_client_placement']} quest NPCs without npc_spawn row and without client placement data; "
            f"{crep['D_unspawned_no_npc_template']} more are not in npc_template at all")

    # ---- quest_helper rows ------------------------------------------------------
    for r in data.helpers:
        fname = r["str_lua_filename"]
        qid = r["s_event_data_index"]
        if fname not in scripts:
            ci = [s for s in scripts if s.lower() == fname.lower()]
            add("HELPER_LUA_MISSING", "ERROR", fname, 0, None,
                f"quest_helper {r['n_index']} (quest {qid}) uses `{fname}` which is not in Quests/"
                + (f" (case differs: `{ci[0]}`)" if ci else ""))
            continue
        for label, ev in (("trigger", r["n_event_trigger_index"]), ("complete", r["n_event_complete_index"])):
            if ev and ev not in handler_events[fname]:
                sc = scripts[fname]
                other = any(sc.toks[i].kind == "id" and sc.toks[i].val == "EVENT" and sc.toks[i + 1].val in (">=", "<=", ">", "<")
                            for i in range(len(sc.toks) - 1))
                if not other:
                    f = add("HELPER_EVENT_NO_HANDLER", "ERROR", fname, 0, ev,
                            f"quest_helper {r['n_index']} (quest {qid}, lvl {r['b_level']}) {label} EVENT {ev} has no handler in `{fname}`")
                    f.quests.add(qid)
        n = r["s_npc_id"]
        if n:
            if n not in data.npcs:
                f = add("HELPER_NPC_MISSING", "WARN", fname, 0, None,
                        f"quest_helper {r['n_index']} (quest {qid}) NPC {n} is not in npc_template")
                f.quests.add(qid)
            elif not any(not mon for (_, mon) in data.spawned.get(n, set())):
                f = add("HELPER_NPC_UNSPAWNED", "WARN", fname, 0, None,
                        f"quest_helper {r['n_index']} (quest {qid}) NPC {n} has no npc_spawn row - nobody to talk to")
                f.quests.add(qid)
        ex_i = r["n_exchange_index"]
        if ex_i > 0 and ex_i not in data.exchange:
            f = add("HELPER_EXCHANGE_MISSING", "WARN", fname, 0, None,
                    f"quest_helper {r['n_index']} (quest {qid}) n_exchange_index {ex_i} not in item_exchange")
            f.quests.add(qid)

    # ---- quest_monster ----------------------------------------------------------
    for q, mq in sorted(data.monsters.items()):
        for g in range(1, 5):
            cnt = int(mq[f"s_count{g}"])
            for col in "abcd":
                mid = int(mq[f"s_num{g}{col}"])
                if mid and cnt and not any(m_["is_monster"] in ("t", "true", "True") for m_ in data.npcs.get(mid, [])):
                    f = add("MONSTER_NOT_IN_TEMPLATE", "ERROR", "quest_monster", 0, None,
                            f"quest {q} group {g}: monster {mid} is not a monster in npc_template - kills never count")
                    f.quests.add(q)

    reachable_all: dict[str, set[int]] = {}
    for name, rows_ in quest_events.items():
        u: set[int] = set()
        for _, rs in rows_:
            u |= rs
        reachable_all[name] = u
    for f in findings:
        if f.file in scripts:
            attach_quests(f)
            # A problem inside a handler that no quest_helper row (and no dialog
            # button / EVENT jump chain starting at one) can ever reach is dead code.
            if (f.event is not None and f.file in reachable_all and f.sev == "ERROR"
                    and f.code not in ("HELPER_EVENT_NO_HANDLER",)
                    and f.event not in reachable_all[f.file]):
                f.sev = "INFO"
                f.detail += " [unreachable: no quest_helper row leads to this EVENT]"
            elif f.event is not None and f.file not in reachable_all and f.sev == "ERROR" and f.file in scripts                     and f.code not in ("HELPER_EVENT_NO_HANDLER",):
                f.sev = "WARN"
                f.detail += " [script has no quest_helper row at all]"

    # ---- unreferenced scripts ---------------------------------------------------
    referenced = {r["str_lua_filename"] for r in data.helpers}
    orphans = sorted(n for n in scripts if n not in referenced)

    # ---- output ----------------------------------------------------------------
    by_code = collections.Counter((f.code, f.sev) for f in findings)
    quest_info = {}
    for r in data.helpers:
        quest_info.setdefault(r["s_event_data_index"], r)
    broken_quests: dict[int, list[Finding]] = collections.defaultdict(list)
    for f in findings:
        if f.sev == "ERROR":
            for q in f.quests:
                broken_quests[q].append(f)

    lines: list[str] = []
    w = lines.append
    nerr = sum(1 for f in findings if f.sev == "ERROR")
    nwarn = sum(1 for f in findings if f.sev == "WARN")
    w(f"Scripts scanned: **{len(scripts)}** Lua files, **{sum(len(s.calls) for s in scripts.values())}** calls, "
      f"**{len(data.helpers)}** quest_helper rows, **{len(data.quest_ids)}** quests, "
      f"**{len(data.exchange)}** item_exchange rows ({len(used_exchanges)} referenced from Lua), "
      f"**{len(data.items)}** items.  ")
    w(f"Findings: **{nerr} ERROR**, **{nwarn} WARN**, {len(findings) - nerr - nwarn} INFO.  ")
    w(f"Quests with at least one ERROR on a path they reach: **{len(broken_quests)}** of {len(data.quest_ids)}.")
    w("")
    w("### Findings by check")
    w("")
    w("| Check | Severity | Count |")
    w("|---|---|---:|")
    for (code, sev), n in sorted(by_code.items(), key=lambda kv: (SEV_ORDER[kv[0][1]], -kv[1])):
        w(f"| `{code}` | {sev} | {n} |")
    w("")
    shifted_all = sorted(i for i, r in data.exchange.items() if exchange_shifted(r))
    w(f"### item_exchange rows that are misaligned (shifted one column): {len(shifted_all)} total, "
      f"{sum(1 for i in shifted_all if i in used_exchanges)} used by Lua")
    w("")
    if shifted_all:
        used = [i for i in shifted_all if i in used_exchanges]
        w("Used by quest scripts: " + (", ".join(f"`{i}` ({'/'.join(sorted({u[0] for u in used_exchanges[i]}))})" for i in used) or "none"))
        w("")
    # level progression view
    w("### Main progression (levels 1-83): quests with an ERROR on a reachable path")
    w("")
    prog = []
    for q, fl in sorted(broken_quests.items()):
        r = quest_info.get(q)
        if r and 1 <= r["b_level"] <= 83:
            prog.append((r["b_level"], q, r, fl))
    prog.sort(key=lambda x: (x[0], x[1]))
    w("| Lvl | Quest | Nation | NPC (Lua) | Problem |")
    w("|---:|---:|---|---|---|")
    for lvl, q, r, fl in prog[: args.max_detail]:
        seen = []
        for f in fl:
            t = f"`{f.code}` L{f.line} {f.detail}"
            if t not in seen:
                seen.append(t)
        nat = {0: "any", 1: "Karus", 2: "El Morad", 3: "both"}.get(r["b_nation"], str(r["b_nation"]))
        w(f"| {lvl} | {q} | {nat} | {r['s_npc_id']} (`{r['str_lua_filename']}`) | " + "<br>".join(seen[:4]).replace("|", "\\|") + " |")
    if not prog:
        w("| - | - | - | - | none |")
    w("")
    w("### All ERROR findings by script")
    w("")
    per_file: dict[str, list[Finding]] = collections.defaultdict(list)
    for f in findings:
        if f.sev == "ERROR":
            per_file[f.file].append(f)
    shown = 0
    for fname in sorted(per_file):
        fl = per_file[fname]
        w(f"<details><summary><code>{fname}</code> - {len(fl)} error(s)</summary>")
        w("")
        seen = set()
        for f in sorted(fl, key=lambda x: (x.line, x.code)):
            key = (f.code, f.line, f.detail)
            if key in seen:
                continue
            seen.add(key)
            qs = f" (quests {', '.join(map(str, sorted(f.quests)[:8]))}{'...' if len(f.quests) > 8 else ''})" if f.quests else ""
            w(f"- L{f.line} `{f.code}`: {f.detail}{qs}")
        w("")
        w("</details>")
        w("")
        shown += 1
    w(f"### Lua files no quest_helper row points to ({len(orphans)})")
    w("")
    w(", ".join(f"`{o}`" for o in orphans) if orphans else "none")
    w("")

    text = "\n".join(lines) + "\n"
    begin, end = "<!-- AUTO:BEGIN -->", "<!-- AUTO:END -->"
    if args.out.exists():
        old = args.out.read_text(encoding="utf-8")
        if begin in old and end in old:
            head = old[: old.index(begin) + len(begin)]
            tail = old[old.index(end):]
            args.out.write_text(head + "\n" + text + tail, encoding="utf-8")
        else:
            args.out.write_text(old.rstrip() + "\n\n" + begin + "\n" + text + end + "\n", encoding="utf-8")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(begin + "\n" + text + end + "\n", encoding="utf-8")
    if args.json:
        args.json.write_text(json.dumps([f.as_dict() for f in findings], indent=1), encoding="utf-8")
    print(f"{len(scripts)} scripts, {len(findings)} findings ({nerr} error, {nwarn} warn) -> {args.out}")
    if args.fail_on_error and nerr:
        sys.exit(1)


if __name__ == "__main__":
    main()
