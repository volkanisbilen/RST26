#!/usr/bin/env python3
"""Build the input of the Rust quest simulation harness (crates/ko-game/src/lua_engine/quest_sim_tests.rs).

  python tools/quest_sim_plan.py --data-dir <csv snapshot> --client-dir <dir with ex.txt / ex_exp.txt (tbl_dump output)> --out <dir>

Writes TSV files into <out>: items, exchange, exp, helper, monster, client_ex, client_exp, plan.
plan.tsv = one row per (lua file, EVENT handler):  lua, event, flags (SelectMsg flags that lead here), exchange ids called,
quest ids of the file, nations of the file's helper rows, uses CheckClass (0/1).
Run the harness with:  QUEST_SIM_DIR=<out> cargo test -p ko-game --lib quest_sim_all -- --ignored --nocapture
"""
import argparse, csv, re, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import quest_lua

csv.field_size_limit(1 << 24)


def rows(p):
    with open(p, newline='', encoding='utf8') as f:
        return list(csv.DictReader(f))


def parse_tbl(p):
    d = {}
    for l in open(p, encoding='utf8', errors='ignore'):
        if not l.startswith('row='):
            continue
        f = dict(x.strip().split('=', 1) for x in l.strip().split(' | ')[1:])
        d[int(f['c0'])] = [int(f['c%d' % i]) for i in range(1, len(f))]
    return d


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--data-dir', required=True)
    ap.add_argument('--client-dir', required=True)
    ap.add_argument('--quests', default='Quests')
    ap.add_argument('--out', required=True)
    a = ap.parse_args()
    D = Path(a.data_dir); O = Path(a.out); O.mkdir(parents=True, exist_ok=True)
    # --- data tables -----------------------------------------------------
    def tsv(name, header, data):
        with open(O / name, 'w', encoding='utf8') as f:
            for r in data:
                f.write('\t'.join(str(x) for x in r) + '\n')
    items = rows(D / 'item.csv')
    ex = rows(D / 'item_exchange.csv')
    cols = ['n_index', 'random_flag'] + [f'origin_item_{x}{k}' for k in range(1, 6) for x in ('num', 'count')] + \
           [f'exchange_item_{x}{k}' for k in range(1, 6) for x in ('num', 'count')] + [f'exchange_item_time{k}' for k in range(1, 6)]
    tsv('exchange.tsv', None, [[r[c] or 0 for c in cols] for r in ex])
    ee = rows(D / 'item_exchange_exp.csv')
    ecols = ['n_index', 'random_flag'] + [f'exchange_item_{x}{k}' for k in range(1, 6) for x in ('num', 'count')]
    tsv('exp.tsv', None, [[r[c] or 0 for c in ecols] for r in ee])
    qh = rows(D / 'quest_helper.csv')
    hcols = ['n_index', 'b_message_type', 'b_level', 'n_exp', 'b_class', 'b_nation', 'b_quest_type', 'b_zone', 's_npc_id',
             's_event_data_index', 'b_event_status', 'n_event_trigger_index', 'n_event_complete_index', 'n_exchange_index',
             'n_event_talk_index', 'str_lua_filename', 's_quest_menu', 's_npc_main', 's_quest_solo']
    tsv('helper.tsv', None, [[r[c] or 0 for c in hcols] for r in qh])
    qm = rows(D / 'quest_monster.csv')
    mcols = list(qm[0].keys())
    tsv('monster.tsv', None, [[r[c] or 0 for c in mcols] for r in qm])
    ids = set()
    for r in ex:
        for k in range(1, 6):
            ids.add(r[f'origin_item_num{k}']); ids.add(r[f'exchange_item_num{k}'])
    for r in ee:
        for k in range(1, 6):
            ids.add(r[f'exchange_item_num{k}'])
    cx = parse_tbl(Path(a.client_dir) / 'ex.txt')
    for v in cx.values():
        ids.update(str(x) for x in v[2:22:2])
    ce_ = parse_tbl(Path(a.client_dir) / 'ex_exp.txt')
    for v in ce_.values():
        ids.update(str(x) for x in v[1:11:2])
    tsv('items.tsv', None, [(r['num'], r['countable'] or 0, r['weight'] or 0, r['class'] or 0) for r in items if r['num'] in ids])
    cx = parse_tbl(Path(a.client_dir) / 'ex.txt')
    with open(O / 'client_ex.tsv', 'w') as f:
        for i, v in cx.items():
            # c1 flag, c2 ?, origins c3..c12 (num,count)x5, outputs c13..c22
            f.write('\t'.join(str(x) for x in [i, v[0]] + v[2:22]) + '\n')
    ce = parse_tbl(Path(a.client_dir) / 'ex_exp.txt')
    with open(O / 'client_exp.tsv', 'w') as f:
        for i, v in ce.items():
            f.write('\t'.join(str(x) for x in [i] + v[:11]) + '\n')
    # --- plan --------------------------------------------------------------
    by_lua = {}
    for r in qh:
        by_lua.setdefault(r['str_lua_filename'], []).append(r)
    plan = []
    for p in sorted(Path(a.quests).glob('*.lua')):
        src = p.read_text(encoding='utf8', errors='ignore').lstrip('\ufeff')
        try:
            sc = quest_lua.parse(p.name, src)
        except Exception as e:
            print('parse fail', p.name, e); continue
        flags = {}
        for c in sc.calls:
            if c.name == 'SelectMsg':
                fl = c.arg_const(1, sc.env)
                for k in range(6, len(c.args), 2):
                    ev = c.arg_const(k, sc.env)
                    if ev:
                        for e in ev:
                            for f_ in (fl or {0}):
                                flags.setdefault(e, set()).add(f_)
        # EVENT = X inside a handler jumps to handler X within the same run: X sees the same dialog flag
        for _ in range(3):
            for h in sc.handlers:
                fl_h = set()
                for e in h.events:
                    fl_h |= flags.get(e, set())
                for x in h.reassign:
                    if fl_h - flags.get(x, set()):
                        flags.setdefault(x, set()).update(fl_h)
        hl = by_lua.get(p.name, [])
        qids = sorted({int(r['s_event_data_index']) for r in hl if int(r['s_event_data_index'])})
        nats = sorted({int(r['b_nation']) for r in hl}) or [3]
        uses_class = int(any(c.name == 'CheckClass' for c in sc.calls))
        for h in sc.handlers:
            ids = set()
            for c in sc.calls:
                if c.name == 'RunQuestExchange' and h.start <= c.pos <= h.end:
                    v = c.arg_const(1, sc.env)
                    if v: ids |= v
            for ev in h.events:
                plan.append([p.name, ev, ','.join(map(str, sorted(flags.get(ev, {0})))), ','.join(map(str, sorted(ids))) or '-',
                             ','.join(map(str, qids)) or '-', ','.join(map(str, nats)), uses_class])
    tsv('plan.tsv', None, plan)
    print('plan rows', len(plan), 'with exchange', sum(1 for r in plan if r[3] != '-'))


main()
