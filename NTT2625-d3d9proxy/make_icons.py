"""Creates the missing pus_ui\\icons\\<iconId>.pus (45x45) for every item icon id of the server's item table,
from the itemicon_*.dxt textures of the client's UI pack (JstKO-decrypted).

Usage: python make_icons.py <item table dump>   [clientDir]
  <item table dump> = db_backup\\ko_server_full.sql (COPY public.item) or a text file with one icon id per line
"""
import os
import sys

from PIL import Image

import uifskin as S

client = sys.argv[2] if len(sys.argv) > 2 else S.CLIENT
out_dir = os.path.join(client, 'JstKO', 'pus_ui', 'icons')
os.makedirs(out_dir, exist_ok=True)
have = {os.path.splitext(n)[0] for n in os.listdir(out_dir)}

ids = set()
src = sys.argv[1]
if src.endswith('.sql'):
    lines = open(src, encoding='utf-8', errors='replace').read().split('\n')
    start = next(k for k, l in enumerate(lines) if l.startswith('COPY public.item ('))
    cols = lines[start].split('(')[1].split(')')[0].split(', ')
    a, b = cols.index('item_icon_id1'), cols.index('item_icon_id2')
    for l in lines[start + 1:]:
        if l == '\\.':
            break
        r = l.split('\t')
        for v in (r[a], r[b]):
            if v.isdigit() and int(v) > 0:
                ids.add(int(v))
else:
    ids = {int(x) for x in open(src).read().split() if x.isdigit()}


def fn(i):
    s = str(i)
    return 'itemicon_%s_%s_%s_%s.dxt' % (s[:-7], s[-7:-3], s[-3:-1], s[-1])


made = missing = 0
for i in sorted(ids):
    if str(i) in have:
        continue
    img = None
    for cand in (i, i // 10 * 10):
        try:
            img = S.tex().get(fn(cand)).convert('RGBA')
            break
        except Exception:  # noqa
            continue
    if img is None:
        missing += 1
        continue
    img = img.crop((0, 0, 45, 45)) if img.size[0] >= 45 else img.resize((45, 45))
    S.save_pus(img, os.path.join(out_dir, '%d.pus' % i))
    made += 1
print('icon ids', len(ids), 'already there', len(ids) - made - missing, 'made', made, 'not in the pack', missing)
