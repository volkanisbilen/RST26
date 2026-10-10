r"""Cross Exchange skin for the RCE panel (rce_store.cpp), the same as the reference picture
F:\resimlers\resimlers\Cross Exchange Sistemi.png: the HSACSX 2369 re_piecechange.hsacsx window (Chaotic Generator:
green frame, bronze corners, title plate, 7 x 4 inventory, noah field) whose upper part holds a 6 x 4 reward grid,
the coupon slot, KC / TL fields, the noah field, Exchange and the chosen reward slot.
Atlases are the game's own (re_combinateitem01/02, re_page_state, re_piecechange, re_skill01) via kopanel.
Output: rce_ui\*.pus + rce_layout.h (same names rce_store.cpp already uses, plus the Cross Exchange extras).
Usage: python build_cross_assets.py [path to decrypted re_piecechange.uif]"""
import os, sys
from PIL import Image
import kopanel as K
import uifrender_old as U

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, 'hsacsx_ui', 're_piecechange.uif')
OUT = os.path.join(HERE, 'cross_ui')
root = U.load(SRC)
W, H = 430, 602

def img_of(e):
    T = U.tex(e['tex']); Wt, Ht = T.size; u0, v0, u1, v1 = e['uv']
    l, t, r, b = e['rect']
    return T.crop((round(u0 * Wt), round(v0 * Ht), round(u1 * Wt), round(v1 * Ht))).resize((r - l, b - t), Image.LANCZOS)

def child(id_):
    for k in root['children']:
        if k['id'] == id_: return k

# ---------------------------------------------------------------- background: the window images of the root
bg = Image.new('RGBA', (W, H), (0, 0, 0, 0))
for k in root['children']:
    if k['type'] == 'IMAGE' and k.get('tex') and 're_main_ui_01' not in k['tex']             and k['rect'] != [133, 26, 296, 46]:                   # skip the count field and the 'Chaotic Generator' lettering
        bg.alpha_composite(img_of(k), tuple(k['rect'][:2]))
slot_img = bg.crop((39, 327, 82, 370))                      # an empty inventory cell of the window itself
# title plate: its centre still carries a faint 'Chaotic Generator'; rebuild it from its plain left part
plate = bg.crop((108, 18, 318, 52))
bg.paste(K.slice3(plate, plate.size[0], plate.size[1], 16, mid=(18, 44)), (108, 18))

# upper part: dark inner panel instead of the stone generator plate
L = {}
def r(name, x0, y0, x1, y1): L[name] = (x0, y0, x1, y1); return (x0, y0, x1, y1)
def fill(rect, rgba):
    bg.alpha_composite(Image.new('RGBA', (rect[2] - rect[0], rect[3] - rect[1]), rgba), rect[:2])
def frame(rect, rgba, th=1):
    x0, y0, x1, y1 = rect
    for a in ((x0, y0, x1, y0 + th), (x0, y1 - th, x1, y1), (x0, y0, x0 + th, y1), (x1 - th, y0, x1, y1)):
        fill(a, rgba)

fill((30, 56, 392, 308), (16, 18, 20, 255))
frame((30, 56, 392, 308), (96, 82, 52, 255), 2)
frame((38, 62, 384, 240), (70, 62, 44, 255), 1)              # reward grid box
frame((38, 244, 384, 302), (70, 62, 44, 255), 1)             # controls box

S, STEP = 43, 49
gx = (W - 8 - (6 * S + 5 * (STEP - S))) // 2
slots = []
for row in range(4):
    for col in range(6):
        x, y = gx + col * STEP, 68 + row * 43
        bg.alpha_composite(slot_img.resize((S, 40)), (x, y))
        slots.append((x + 2, y + 2, x + S - 2, y + 38))
coupon = r('coupon', 46, 251, 89, 294)
result = r('result', 333, 251, 376, 294)
bg.alpha_composite(slot_img, coupon[:2]); bg.alpha_composite(slot_img, result[:2])
slots.append((result[0] + 2, result[1] + 2, result[2] - 2, result[3] - 2))      # 25th reward (rare) = right cell

r('kc_label', 100, 251, 124, 267); kc = r('kc', 126, 251, 204, 267)
r('tl_label', 214, 251, 236, 267); tl = r('tl', 238, 251, 316, 267)
coin_ico = child('') and None
gold_field = r('gold', 148, 271, 298, 287)
for f in (kc, tl, gold_field):
    fill(f, (8, 10, 12, 255)); frame(f, (82, 74, 56, 255), 1)
# coin icon of the window (re_skill01) next to the noah field
for k in root['children']:
    if k['type'] == 'IMAGE' and 're_skill01' in (k.get('tex') or ''):
        bg.alpha_composite(img_of(k).resize((16, 18)), (128, 270))
exch = r('exchange', 150, 289, 296, 305)

# buttons: Start (normal / over / down) images of btn_start, close = btn_close states
def states(btn_id):
    b = child(btn_id)
    return [img_of(k) for k in b['children'] if k['type'] == 'IMAGE' and k.get('tex')]
os.makedirs(OUT, exist_ok=True)
st = states('btn_start')
# the button art has 'Start' painted in the middle: stretch its plain left part instead (3-slice, caps kept)
def plain(im):
    w, h = exch[2] - exch[0], exch[3] - exch[1]
    return K.slice3(im, w, h, 8, mid=(10, 34))
for name, im in zip(('exchange_n', 'exchange_d', 'exchange_o'), (st[3] if len(st) > 3 else st[0], st[1] if len(st) > 1 else st[0], st[0])):   # grey normal, red over
    K.save_pus(plain(im), os.path.join(OUT, name + '.pus'))
cl = states('btn_close'); close = child('btn_close')['rect']
for name, im in zip(('close_n', 'close_d', 'close_o'), (cl[0], cl[2] if len(cl) > 2 else cl[0], cl[1] if len(cl) > 1 else cl[0])):
    K.save_pus(im, os.path.join(OUT, name + '.pus'))
K.save_pus(slot_img, os.path.join(OUT, 'coupon.pus'))
K.save_pus(bg, os.path.join(OUT, 'bg.pus'))

inv = [tuple(k['rect']) for i in range(28) for k in root['children'] if k['id'] == 'a_slot_%d' % i]
title = (110, 20, 316, 50)

prev = bg.copy()
for nm, rc in (('exchange_n', exch), ('close_n', close)):
    prev.alpha_composite(Image.open(os.path.join(OUT, nm + '.png')), tuple(rc[:2]))
prev.save(os.path.join(OUT, '_preview.png'))

R = lambda t: '{ %d, %d, %d, %d }' % tuple(t)
with open(os.path.join(HERE, 'cross_layout.h'), 'w') as f:
    f.write('// Generated by build_cross_assets.py (Cross Exchange skin, HSACSX re_piecechange) -- do not edit by hand.\n#pragma once\n')
    f.write('#ifndef RCE_TYPES\n#define RCE_TYPES\nstruct RceRect { int l, t, r, b; };\nstruct RceSprite { const char* name; RceRect rc; };\n#endif\n')
    f.write('static const int RCE_W = %d, RCE_H = %d;\nstatic const int RCE_SLOT_COUNT = 25;\n' % (W, H))
    f.write('static const RceSprite RCE_SPRITES[] = {\n')
    f.write('    { "bg", { 0, 0, %d, %d } },\n' % (W, H))
    f.write('    { "coupon", %s },\n' % R(coupon))
    for s in ('exchange_n', 'exchange_d', 'exchange_o'): f.write('    { "%s", %s },\n' % (s, R(exch)))
    for s in ('close_n', 'close_d', 'close_o'): f.write('    { "%s", %s },\n' % (s, R(close)))
    f.write('};\n')
    f.write('static const RceRect RCE_COUPON = %s;\n' % R((coupon[0] + 2, coupon[1] + 2, coupon[2] - 2, coupon[3] - 2)))
    f.write('static const RceRect RCE_TITLE = %s;\n' % R(title))
    f.write('static const RceRect RCE_BTN_EXCHANGE = %s;\n' % R(exch))
    f.write('static const RceRect RCE_BTN_CLOSE = %s;\n' % R(close))
    f.write('static const RceRect RCE_SLOTS[25] = {\n' + ',\n'.join('    ' + R(s) for s in slots) + '\n};\n')
    f.write('')
    f.write('static const RceRect RCE_INV[28] = {\n' + ',\n'.join('    ' + R(s) for s in inv) + '\n};\n')
    for k in ('kc_label', 'kc', 'tl_label', 'tl', 'gold', 'result'):
        f.write('static const RceRect RCE_%s = %s;\n' % (k.upper(), R(L[k])))
    f.write('static const RceRect RCE_WEIGHT = { 36, 547, 236, 567 };\n')
    f.write('static const RceRect RCE_NOAH = { 272, 546, 372, 566 };\n')
print('cross exchange skin ->', OUT, 'inv cells', len(inv))
