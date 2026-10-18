"""Cinderella War ("Fun Class") skin for the d3d9 proxy (cind_panel.cpp).

Layout = 2585 HSACSX\\UI\\re_funclass.hsacsx (decrypted with hsacsx_dec.py; old UIF layout), same rects as the
2585 client panel. Pieces:
  frame + header   : ui\\tournament_chaos_2021_a01.dxt   (game UI pack)
  close button     : ui\\re_skill04.dxt                  (game UI pack)
  nation emblems   : ui\\el_ka_symbol.dxt                (game UI pack; the 2585 file used 1098event_1.dxt,
                                                         which exists in neither client)
  class portraits  : funclass.dxt, "SELECT CLASS" tab : selectclassdxt.dxt (2585 HSACSX\\UI, loose files)
Output: cind_ui\\*.pus + cind_layout.h
Usage: python build_cind_assets.py [2585 HSACSX UI dir] [gameDir]
"""
import struct, sys, os, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
HSUI = sys.argv[1] if len(sys.argv) > 1 else r'F:\2585 Proje\Release\HSACSX\UI'
GAME = sys.argv[2] if len(sys.argv) > 2 else r'F:\KnightOnlineEn - Kopya (2)\KnightOnlineEn - Kopya'
FALLBACK_DIRS = [HSUI]
OUT = os.path.join(HERE, 'cind_ui')

_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
_a = _cr.index('# ------------------------------------------------------------------ textures')
_b = _cr.index('# ------------------------------------------------------------------ build')
exec(compile(_cr[_a:_b], 'build_cr_assets.py', 'exec'))
sys.path.insert(0, HERE)
from hsacsx_dec import decrypt

# ------------------------------------------------------------------ UIF (old layout: int child count)
TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON'}
class R:
    def __init__(s, d): s.d, s.o = d, 0
    def i(s): v = struct.unpack_from('<i', s.d, s.o)[0]; s.o += 4; return v
    def u(s): v = struct.unpack_from('<I', s.d, s.o)[0]; s.o += 4; return v
    def f(s): v = struct.unpack_from('<f', s.d, s.o)[0]; s.o += 4; return v
    def s_(s):
        n = s.i(); v = s.d[s.o:s.o + n].decode('cp1252', 'replace'); s.o += n; return v
def elem(r, typ):
    e = {'type': TYPES[typ], 'name': r.s_()}
    kids = [elem(r, r.i()) for _ in range(r.i())]
    e['id'] = r.s_(); e['rect'] = [r.i() for _ in range(4)]; [r.i() for _ in range(4)]
    r.u(); r.u(); r.s_(); r.s_(); r.s_()
    if e['type'] == 'IMAGE': e['tex'] = r.s_(); e['uv'] = [r.f() for _ in range(4)]; r.f()
    elif e['type'] == 'STRING': e['font'] = r.s_(); e['fontH'] = r.i(); r.u(); e['color'] = r.u(); e['text'] = r.s_()
    elif e['type'] == 'BUTTON': [r.i() for _ in range(4)]; r.s_(); r.s_()
    elif e['type'] == 'STATIC': r.s_()
    e['children'] = kids
    return e
def load_uif(path):
    rr = R(decrypt(open(path, 'rb').read()))
    return elem(rr, 0)
def find(e, id_):
    if e['id'] == id_: return e
    for k in e['children']:
        f = find(k, id_)
        if f: return f
    return None

ROOT = load_uif(os.path.join(HSUI, 're_funclass.hsacsx'))
ICON = load_uif(os.path.join(HSUI, 're_war_icon.hsacsx'))

os.makedirs(OUT, exist_ok=True)
sprites = []
def put(name, img, rect):
    save_pus(img, name); sprites.append((name, rect))

# frame: every top-level image except the missing 1098event emblems
frame = [k for k in ROOT['children'] if k['type'] == 'IMAGE' and '1098event' not in k['tex'].lower()]
FR = bbox(frame)
put('frame', compose(frame, FR), FR)

# nation emblems from the game's own symbol atlas, in the 1098event slots (left = Karus, right = Elmorad)
sym = load_tex(r'ui\el_ka_symbol.dxt')
def emblem(box):
    part = sym.crop(box); part = part.crop(part.getbbox())
    w, h = part.size; k = 26.0 / max(w, h)
    part = part.resize((max(1, round(w * k)), max(1, round(h * k))), Image.LANCZOS)
    c = Image.new('RGBA', (26, 26), (0, 0, 0, 0)); c.alpha_composite(part, ((26 - part.size[0]) // 2, (26 - part.size[1]) // 2))
    return c
slots = [k for k in ROOT['children'] if k['type'] == 'IMAGE' and '1098event' in k['tex'].lower()]
slots.sort(key=lambda e: e['rect'][0])
put('emb_karus', emblem((0, 270, 128, 440)), slots[0]['rect'])
put('emb_elmo', emblem((0, 0, 140, 136)), slots[1]['rect'])

# buttons: UIF state images = normal, down, over(, disabled)
def states(b, prefix, names=('n', 'd', 'o')):
    imgs = [k for k in b['children'] if k['type'] == 'IMAGE']
    for si, sname in enumerate(names):
        if si < len(imgs): put('%s_%s' % (prefix, sname), compose([imgs[si]], b['rect']), b['rect'])
for bid in ('btn_warrior', 'btn_rogue', 'btn_mage', 'btn_priest', 'btn_close'):
    states(find(ROOT, bid), bid)

# collapsed "SELECT CLASS" tab (re_war_icon btn_funclass), placed at 0,0
fc = find(ICON, 'btn_funclass'); l, t, r, b = fc['rect']
fimgs = [k for k in fc['children'] if k['type'] == 'IMAGE']
for si, sname in enumerate(('n', 'd', 'o')):
    e = dict(fimgs[si]); e['rect'] = [0, 0, r - l, b - t]
    put('tab_%s' % sname, compose([e], e['rect']), e['rect'])

# preview
prev = compose(frame, FR)
for n, rr in sprites:
    if n.endswith('_n') and not n.startswith('tab') or n.startswith('emb'):
        prev.alpha_composite(Image.open(os.path.join(OUT, n + '.png')), (rr[0] - FR[0], rr[1] - FR[1]))
prev.save(os.path.join(OUT, '_preview.png'))

# ------------------------------------------------------------------ layout header (rects relative to the frame)
L, T = FR[0], FR[1]
def rs(r): return '%d, %d, %d, %d' % (r[0] - L, r[1] - T, r[2] - L, r[3] - T)
texts = []
def collect(e):
    if e['type'] == 'STRING': texts.append(e)
    for k in e['children']: collect(k)
collect(ROOT)
with open(os.path.join(HERE, 'cind_layout.h'), 'w') as f:
    f.write('// Generated by build_cind_assets.py from 2585 re_funclass.hsacsx -- do not edit by hand.\n#pragma once\n')
    f.write('static const int CIS_W = %d, CIS_H = %d;\n' % (FR[2] - FR[0], FR[3] - FR[1]))
    f.write('static const int CIS_TAB_W = %d, CIS_TAB_H = %d;\n' % (r - l, b - t))
    f.write('struct CisSprite { const char* name; int l, t, r, b; };\nstatic const CisSprite CIS_SPRITES[] = {\n')
    for n, rr in sprites:
        if n.startswith('tab'): f.write('    { "%s", 0, 0, %d, %d },\n' % (n, rr[2], rr[3]))
        else: f.write('    { "%s", %s },\n' % (n, rs(rr)))
    f.write('};\n')
    for bid in ('btn_warrior', 'btn_rogue', 'btn_mage', 'btn_priest', 'btn_close'):
        f.write('static const int CIS_%s[4] = { %s };\n' % (bid.upper(), rs(find(ROOT, bid)['rect'])))
    anon = 0
    for e in texts:
        nm = e['id'] or 'label_%d' % anon
        if not e['id']: anon += 1
        f.write('static const int CIS_%s[4] = { %s };  // "%s" color %08X h%d\n' % (nm.upper(), rs(e['rect']), e['text'], e['color'], e['fontH']))
print('frame', FR, 'sprites', len(sprites), '->', OUT)
