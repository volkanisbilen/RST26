r"""Exports every itemicon_*.dxt of the client's UI\ui.src as a 45x45 JPG into the admin panel's itemicon folder.
Usage: python export_itemicons.py <clientDir> <outDir>
Existing files are kept (the supplied icons win).
"""
import os, sys, struct, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
GAME, OUT = sys.argv[1], sys.argv[2]
FALLBACK_DIRS = []
_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
exec(compile(_cr[_cr.index('# ------------------------------------------------------------------ textures'):
                 _cr.index('# ------------------------------------------------------------------ build')], 'cr', 'exec'))

import uif_crypt
_orig_raw = raw_tex
def raw_tex(name):
    b, src = _orig_raw(name)
    nl = struct.unpack_from('<i', b, 0)[0]
    if not (0 <= nl < 260 and b[4 + nl:7 + nl] == b'NTF'):
        b = uif_crypt.decrypt(b)        # HopeGuard pack-entry cipher
    return b, src

os.makedirs(OUT, exist_ok=True)
made = skipped = failed = 0
for name in sorted(n for n in pack if n.startswith('itemicon_') and n.endswith('.dxt')):
    dst = os.path.join(OUT, name[:-4] + '.jpg')
    if os.path.exists(dst):
        skipped += 1
        continue
    try:
        img = load_tex(name).convert('RGBA')
        img = img.crop((0, 0, 45, 45)) if img.size[0] >= 45 and img.size[1] >= 45 else img.resize((45, 45))
        bg = Image.new('RGB', (45, 45), (0, 0, 0))
        bg.paste(img, mask=img.split()[3])
        bg.save(dst, 'JPEG', quality=92)
        made += 1
    except Exception as e:
        failed += 1
        print('failed', name, e)
    _cache.clear()
print('made', made, 'kept', skipped, 'failed', failed)
