r"""Builds HopeGuard\pus_ui\icons\<iconId>.pus from the itemicon_*.dxt textures inside the client's UI\ui.src.
Usage: python build_icons_ids.py <clientDir> [--tbl Item_Org_us.tbl.txt] [iconId ...]
  --ids : file with one icon id per line (export from the server DB: item_icon_id1 / item_icon_id2)
  --tbl : text dump of the client item table (tbl_dump output); every c6/c7 icon id in it is built
Existing .pus files are kept. Server DB icon ids that the client table does not know are passed as arguments.
"""
import os, sys, re, struct, hashlib, subprocess
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
args = sys.argv[1:]
GAME = args.pop(0)
FALLBACK_DIRS = []
OUT = None
ids = set()
while args:
    a = args.pop(0)
    if a == '--tbl':
        for line in open(args.pop(0), encoding='utf-8', errors='replace'):
            for m in re.finditer(r'\bc[67]=(\d+)', line):
                if int(m.group(1)) > 0: ids.add(int(m.group(1)))
    elif a == '--ids':   # plain list of icon ids, e.g. psql -At -c "select distinct item_icon_id1 from item"
        for line in open(args.pop(0), encoding='utf-8', errors='replace'):
            for tok in line.split():
                if tok.isdigit() and int(tok) > 0: ids.add(int(tok))
    elif a.isdigit():
        ids.add(int(a))
_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
exec(compile(_cr[_cr.index('# ------------------------------------------------------------------ textures'):
                 _cr.index('# ------------------------------------------------------------------ build')], 'cr', 'exec'))

def fn(i):
    s = str(i)
    return 'itemicon_%s_%s_%s_%s.dxt' % (s[:-7], s[-7:-3], s[-3:-1], s[-1])

dst_dir = os.path.join(GAME, 'HopeGuard', 'pus_ui', 'icons')
os.makedirs(dst_dir, exist_ok=True)
made = missing = skipped = 0
for i in sorted(ids):
    dst = os.path.join(dst_dir, '%d.pus' % i)
    if os.path.exists(dst):
        skipped += 1; continue
    try:
        try: img = load_tex(fn(i)).convert('RGBA')
        except Exception: img = load_tex(fn(i // 10 * 10)).convert('RGBA')   # colour variant -> base icon
    except Exception:
        missing += 1; print('missing', i); continue
    img = img.crop((0, 0, 45, 45)) if img.size[0] >= 45 else img.resize((45, 45))
    px = bytearray()
    for r_, g, b, a in img.getdata():
        px += bytes((b * a // 255, g * a // 255, r_ * a // 255, a))
    open(dst, 'wb').write(b'PUSI' + struct.pack('<II', 45, 45) + px)
    made += 1
print('made', made, 'already', skipped, 'missing', missing)
