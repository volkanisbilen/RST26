"""Slave Priest skin for the d3d9 proxy (sp_panel.cpp), built only from the game's own UI atlases.

Mini window  : HSACSX\\Dat\\DAT-7.uif (2369) as is - background strip + Start / Stop / Settings buttons
               (atlas ui\\re_auto_02.dxt, present in the 2625 ui pack).
Settings page: our own layout (see sp_panel.cpp) made of original atlas pieces taken from DAT-32.uif:
               re_akara_01 frame, re_skill04 title plate / row bars / value plate / Enable+Disable buttons /
               close button / scroll arrows / skill slot, plus the game's priest skill icons.
Output: sp_ui\\*.pus + sp_layout.h
Usage: python build_sp_assets.py [datDir] [gameDir]
"""
import struct, sys, os, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
DAT = sys.argv[1] if len(sys.argv) > 1 else r'D:\2369 Proje\HSACSX-Client-2369\HSACSX\Dat'
GAME = sys.argv[2] if len(sys.argv) > 2 else r'F:\KnightOnlineEn - Kopya (2)\KnightOnlineEn - Kopya'
FALLBACK_DIRS = []
OUT = os.path.join(HERE, 'sp_ui')

_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
_a = _cr.index('# ------------------------------------------------------------------ textures')
_b = _cr.index('# ------------------------------------------------------------------ build')
exec(compile(_cr[_a:_b], 'build_cr_assets.py', 'exec'))

# ------------------------------------------------------------------ UIF reader (same format as build_rce_assets.py)
TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON'}
def read_uif(path):
    d = open(path, 'rb').read(); st = {'o': 0}
    def I():
        v = struct.unpack_from('<i', d, st['o'])[0]; st['o'] += 4; return v
    def U():
        v = struct.unpack_from('<I', d, st['o'])[0]; st['o'] += 4; return v
    def F():
        v = struct.unpack_from('<f', d, st['o'])[0]; st['o'] += 4; return v
    def S():
        n = I(); v = d[st['o']:st['o'] + n].decode('cp1252', 'replace'); st['o'] += n; return v
    def elem(typ):
        e = {'type': TYPES.get(typ, str(typ)), 'name': S()}
        e['children'] = [elem(I()) for _ in range(I())]
        e['id'] = S(); e['rect'] = [I() for _ in range(4)]; [I() for _ in range(4)]; U(); U(); S(); S(); S()
        t = e['type']
        if t == 'IMAGE': e['tex'] = S(); e['uv'] = [F() for _ in range(4)]; F()
        elif t == 'STRING': e['font'] = S(); e['fontH'] = I(); U(); e['color'] = U(); e['text'] = S()
        elif t == 'BUTTON': [I() for _ in range(4)]; S(); S()
        elif t == 'STATIC': S()
        elif t == 'PROGRESS': F(); I(); I()
        elif t == 'AREA': I()
        return e
    S(); n = I()
    return [elem(I()) for _ in range(n)]

def find(elems, id_):
    for e in elems:
        if e['id'] == id_: return e
        f = find(e['children'], id_)
        if f: return f
def imgs(e):
    return [k for k in e['children'] if k['type'] == 'IMAGE' and k.get('tex')]

os.makedirs(OUT, exist_ok=True)
sprites = []   # (name, w, h)
def save(name, img):
    save_pus(img, name)
    sprites.append((name, img.size[0], img.size[1]))
def piece(tex, uv, size):
    img = load_tex(tex); W, H = img.size
    box = (round(uv[0] * W), round(uv[1] * H), round(uv[2] * W), round(uv[3] * H))
    p = img.crop(box)
    return p.resize(size, Image.BILINEAR) if size and p.size != size else p
def button_states(prefix, e):
    st = imgs(e)
    for si, sn in enumerate(('n', 'd', 'o', 'x')):
        if si < len(st): save('%s_%s' % (prefix, sn), crop(st[si]))

# ------------------------------------------------------------------ mini window (DAT-7 as is)
mini = read_uif(os.path.join(DAT, 'DAT-7.uif'))
mini_bg = [e for e in mini if e['type'] == 'IMAGE'][0]
save('mini_bg', crop(mini_bg))
for bid, short in (('btn_priest_start', 'mini_start'), ('btn_priest_stop', 'mini_stop'), ('btn_priest_open', 'mini_open')):
    button_states(short, find(mini, bid))
mini_label = [e for e in mini if e['type'] == 'STRING'][0]

# ------------------------------------------------------------------ settings pieces (DAT-32 atlases)
s32 = read_uif(os.path.join(DAT, 'DAT-32.uif'))
SP_W, SP_H = 340, 684
save('frame', piece('ui\\re_akara_01.dxt', (0.0029, 0.0059, 0.8945, 0.7344), (SP_W, SP_H)))
save('title', piece('ui\\re_skill04.dxt', (0.4785, 0.668, 0.832, 0.7529), (173, 41)))
save('rowbar', piece('ui\\re_skill04.dxt', (0.0098, 0.8242, 0.3252, 0.8525), (170, 22)))
save('valbox', piece('ui\\re_skill04.dxt', (0.0625, 0.8242, 0.2559, 0.8516), (52, 22)))
save('slot', piece('ui\\re_skill04.dxt', (0.9434, 0.2578, 0.9932, 0.3086), (38, 36)))
button_states('on', find(s32, 'btn_hp_recovery_on'))
button_states('off', find(s32, 'btn_hp_recovery_off'))
button_states('close', find(s32, 'btn_close'))
scr = find(s32, 'scr_healing_recovery_rate')
arrows = [k for k in scr['children'] if k['type'] == 'BUTTON']
button_states('left', arrows[0])
button_states('right', arrows[1])

# skill box = DAT-32's recovery boxes: 9-slice frame from re_akara_01 corner/edge pieces + re_skill04 box fill
A1 = 'ui\\re_akara_01.dxt'
save('fr_tl', piece(A1, (0.0166, 0.8379, 0.0361, 0.8584), (16, 16)))
save('fr_tr', piece(A1, (0.0410, 0.8379, 0.0615, 0.8584), (16, 16)))
save('fr_bl', piece(A1, (0.0166, 0.8633, 0.0361, 0.8828), (16, 16)))
save('fr_br', piece(A1, (0.0410, 0.8623, 0.0615, 0.8828), (16, 16)))
save('fr_t', piece(A1, (0.1328, 0.8379, 0.1748, 0.8447), (80, 6)))
save('fr_b', piece(A1, (0.1299, 0.8535, 0.1963, 0.8604), (80, 6)))
save('fr_l', piece(A1, (0.0898, 0.8262, 0.0986, 0.8945), (8, 104)))
save('fr_r', piece(A1, (0.1045, 0.8262, 0.1123, 0.8936), (8, 104)))
save('box_bg', piece('ui\\re_skill04.dxt', (0.1025, 0.8955, 0.2949, 0.9668), (197, 73)))
save('plate', piece('ui\\re_skill04.dxt', (0.0625, 0.8242, 0.2559, 0.8516), (198, 28)))
save('track', piece('ui\\re_auto_01.dxt', (0.9238, 0.4785, 0.9824, 0.5059), (60, 14)))
save('thumb', piece('ui\\re_auto_01.dxt', (0.8867, 0.4160, 0.9082, 0.4492), (9, 16)))

# priest skill icons (Karus 112xxx icon files; icon id -> skillicon_%02d_%d with id%100, id/100)
ICONS = {'ico_heal': 112545, 'ico_group': 112560, 'ico_superioris': 112675, 'ico_undying': 112654,
         'ico_ac': 112674, 'ico_cure': 112525, 'ico_res': 112754}
for name, sid in ICONS.items():
    try:
        save(name, piece('ui\\skillicon_%02d_%d.dxt' % (sid % 100, sid // 100), (0, 0, 1, 1), (32, 32)))
    except Exception as ex:
        print('  icon missing', name, ex)

prev = Image.new('RGBA', (SP_W, SP_H))
prev.alpha_composite(Image.open(os.path.join(OUT, 'frame.png')))
prev.save(os.path.join(OUT, '_preview_frame.png'))

def r(e): return '{ %d, %d, %d, %d }' % (e['rect'][0] - mini_bg['rect'][0], e['rect'][1] - mini_bg['rect'][1],
                                           e['rect'][2] - mini_bg['rect'][0], e['rect'][3] - mini_bg['rect'][1])
with open(os.path.join(HERE, 'sp_layout.h'), 'w') as f:
    f.write('// Generated by build_sp_assets.py (DAT-7.uif / DAT-32.uif atlases) -- do not edit by hand.\n#pragma once\n')
    f.write('struct SpRect { int l, t, r, b; };\n')
    f.write('struct SpSprite { const char* name; int w, h; };\nstatic const SpSprite SP_SPRITES[] = {\n')
    for n, w, h in sprites:
        f.write('    { "%s", %d, %d },\n' % (n, w, h))
    f.write('};\n')
    bw, bh = mini_bg['rect'][2] - mini_bg['rect'][0], mini_bg['rect'][3] - mini_bg['rect'][1]
    f.write('static const int SP_MINI_W = %d, SP_MINI_H = %d;\n' % (bw, bh))
    f.write('static const SpRect SP_MINI_LABEL = %s;\n' % r(mini_label))
    f.write('static const SpRect SP_MINI_START = %s;\n' % r(find(mini, 'btn_priest_start')))
    f.write('static const SpRect SP_MINI_STOP = %s;\n' % r(find(mini, 'btn_priest_stop')))
    f.write('static const SpRect SP_MINI_OPEN = %s;\n' % r(find(mini, 'btn_priest_open')))
    f.write('static const int SP_SET_W = %d, SP_SET_H = %d;\n' % (SP_W, SP_H))
print('sprites', len(sprites), '->', OUT)
