"""Generates pus_ui\\icons\\<ItemIconID1>.pus for every Right-Click Exchange coupon and reward item.
Icons come from the game's UI pack: itemicon_<A>_<BBBB>_<CC>_<D>.dxt, top-left 45x45 of the texture.
Usage: python build_rce_icons.py [gameDir]   (DB via sqlcmd -S . -E, KO_DATABASE_SERVER_00125)
"""
import os, sys, subprocess, struct, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = sys.argv[1] if len(sys.argv) > 1 else r'D:\NTTGame\KnightOnlineEn - Kopya'
FALLBACK_DIRS = []
OUT = None
_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
exec(compile(_cr[_cr.index('# ------------------------------------------------------------------ textures'):
                 _cr.index('# ------------------------------------------------------------------ build')], 'cr', 'exec'))

cols = ', '.join('ExchangeItem%d' % i for i in range(1, 26))
q = ("SET NOCOUNT ON; SELECT DISTINCT CASE WHEN i.ItemIconID2 <> 0 THEN i.ItemIconID2 ELSE i.ItemIconID1 END FROM ITEM i WHERE i.Num IN ("
     "SELECT nItemID FROM ITEM_RIGHT_EXCHANGE UNION SELECT sItemID FROM ITEM_RIGHT_CLICK_EXCHANGE UNION "
     "SELECT v FROM ITEM_RIGHT_EXCHANGE UNPIVOT (v FOR c IN (%s)) u)" % cols)
out = subprocess.run(['sqlcmd', '-S', '.', '-E', '-d', 'KO_DATABASE_SERVER_00125', '-h', '-1', '-W', '-Q', q],
                     capture_output=True, text=True).stdout
ids = sorted({int(x) for x in out.split() if x.strip().isdigit() and int(x) > 0})
print('icon ids', len(ids))

def fn(i):
    s = str(i)
    return 'itemicon_%s_%s_%s_%s.dxt' % (s[:-7], s[-7:-3], s[-3:-1], s[-1])

made = missing = 0
for dst_root in (os.path.join(HERE, 'pus_ui', 'icons'), os.path.join(GAME, 'pus_ui', 'icons')):
    os.makedirs(dst_root, exist_ok=True)
for i in ids:
    dst = [os.path.join(HERE, 'pus_ui', 'icons', '%d.pus' % i), os.path.join(GAME, 'pus_ui', 'icons', '%d.pus' % i)]
    if all(os.path.exists(p) for p in dst): continue
    try:
        try: img = load_tex(fn(i)).convert('RGBA')
        except Exception: img = load_tex(fn(i // 10 * 10)).convert('RGBA')   # colour variant -> base icon
    except Exception as e:
        missing += 1; print('  missing', i, fn(i)); continue
    img = img.crop((0, 0, 45, 45)) if img.size[0] >= 45 else img.resize((45, 45))
    px = bytearray()
    for r_, g, b, a in img.getdata():
        px += bytes((b * a // 255, g * a // 255, r_ * a // 255, a))
    blob = b'PUSI' + struct.pack('<II', 45, 45) + px
    for p in dst:
        open(p, 'wb').write(blob)
    made += 1
print('made', made, 'missing', missing)
