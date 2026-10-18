"""Generates pus_ui\\icons\\<iconId>.pus for EVERY item icon in the ITEM table (drop viewer, cross exchange, ...).
Same format/source as build_rce_icons.py: itemicon_<A>_<BBBB>_<CC>_<D>.dxt from the UI pack, 45x45 BGRA premultiplied.
Usage: python build_all_icons.py [gameDir]
"""
import os, sys, subprocess, struct, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = sys.argv[1] if len(sys.argv) > 1 else r'D:\26X V5\KnightOnlineEn - Kopya'
FALLBACK_DIRS = []
OUT = None
_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
exec(compile(_cr[_cr.index('# ------------------------------------------------------------------ textures'):
                 _cr.index('# ------------------------------------------------------------------ build')], 'cr', 'exec'))

q = "SET NOCOUNT ON; SELECT DISTINCT CASE WHEN ItemIconID2 <> 0 THEN ItemIconID2 ELSE ItemIconID1 END FROM ITEM"
out = subprocess.run(['sqlcmd', '-S', r'DESKTOP-UVF79OG\KO_PVP', '-E', '-d', 'KO_DATABASE_SERVER_00125', '-h', '-1', '-W', '-Q', q],
                     capture_output=True, text=True).stdout
ids = sorted({int(x) for x in out.split() if x.strip().isdigit() and int(x) > 0})
print('icon ids', len(ids))

def fn(i):
    s = str(i)
    return 'itemicon_%s_%s_%s_%s.dxt' % (s[:-7], s[-7:-3], s[-3:-1], s[-1])

dst_dir = os.path.join(GAME, 'pus_ui', 'icons')
os.makedirs(dst_dir, exist_ok=True)
made = missing = skipped = 0
for i in ids:
    dst = os.path.join(dst_dir, '%d.pus' % i)
    if os.path.exists(dst):
        skipped += 1; continue
    try:
        try: img = load_tex(fn(i)).convert('RGBA')
        except Exception: img = load_tex(fn(i // 10 * 10)).convert('RGBA')   # colour variant -> base icon
    except Exception:
        missing += 1; continue
    img = img.crop((0, 0, 45, 45)) if img.size[0] >= 45 else img.resize((45, 45))
    px = bytearray()
    for r_, g, b, a in img.getdata():
        px += bytes((b * a // 255, g * a // 255, r_ * a // 255, a))
    open(dst, 'wb').write(b'PUSI' + struct.pack('<II', 45, 45) + px)
    made += 1
print('made', made, 'already', skipped, 'missing', missing)
