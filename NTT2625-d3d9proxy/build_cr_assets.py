"""Builds the Collection Race panel skin for the d3d9 proxy from Desktop\\uifler\\re_collection_race.uif.

Textures come from the game's own UI pack (UI\\ui.hdr / ui.src) first, then loose fallbacks
(HSACSX UI for akar.dxt). NTF v3 plain / v7 RC4 (SHA1 of the game password, stream reset per row).
Output: cr_ui\\*.pus sprites ("PUSI" w h + premultiplied BGRA) + cr_layout.h (sprite + text rects).
Usage: python build_cr_assets.py [uif] [gameDir]
"""
import struct, sys, os, json, hashlib, subprocess
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
UIF = sys.argv[1] if len(sys.argv) > 1 else os.path.expandvars(r'%USERPROFILE%\Desktop\uifler\re_collection_race.uif')
GAME = sys.argv[2] if len(sys.argv) > 2 else r'D:\NTTGame\KnightOnlineEn - Kopya'
FALLBACK_DIRS = [r'D:\25xxacs_2\Release\HSACSX\UI', r'D:\K2_Network\UI']
OUT = os.path.join(HERE, 'cr_ui')

# ------------------------------------------------------------------ UIF
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
    kids = []
    for _ in range(r.i()):
        kids.append(elem(r, r.i()))
    e['id'] = r.s_(); e['rect'] = [r.i() for _ in range(4)]; [r.i() for _ in range(4)]
    r.u(); r.u(); r.s_(); r.s_(); r.s_()
    if e['type'] == 'IMAGE':
        e['tex'] = r.s_(); e['uv'] = [r.f() for _ in range(4)]; r.f()
    elif e['type'] == 'STRING':
        e['font'] = r.s_(); e['fontH'] = r.i(); r.u(); e['color'] = r.u(); e['text'] = r.s_()
    elif e['type'] == 'BUTTON':
        [r.i() for _ in range(4)]; r.s_(); r.s_()
    elif e['type'] == 'STATIC':
        r.s_()
    elif e['type'] == 'PROGRESS':
        r.f(); r.i(); r.i()
    elif e['type'] == 'AREA':
        r.i()
    e['children'] = kids
    return e

d = open(UIF, 'rb').read()
rr = R(d); ROOT = elem(rr, 0)
assert rr.o == len(d), 'UIF not fully consumed (%d/%d)' % (rr.o, len(d))

def find(e, id_):
    if e['id'] == id_: return e
    for k in e['children']:
        f = find(k, id_)
        if f: return f
    return None

def images(e, skip_buttons=True):
    out = []
    if e['type'] == 'IMAGE' and e.get('tex') and e['id'] != 'icon':
        out.append(e)
    for k in e['children']:
        if skip_buttons and k['type'] == 'BUTTON': continue
        out += images(k, skip_buttons)
    return out

# ------------------------------------------------------------------ textures
PASS = b'owsd9012%$1as!wpow1033b%!@%12'
def rc4_stream(key, n):
    S = list(range(256)); j = 0
    for i in range(256):
        j = (j + S[i] + key[i % len(key)]) & 255; S[i], S[j] = S[j], S[i]
    out = bytearray(n); i = j = 0
    for k in range(n):
        i = (i + 1) & 255; j = (j + S[i]) & 255; S[i], S[j] = S[j], S[i]
        out[k] = S[(S[i] + S[j]) & 255]
    return bytes(out)
KEY = hashlib.sha1(PASS).digest()[:16]

pack = {}
hdr = os.path.join(GAME, 'UI', 'ui.hdr')
if os.path.exists(hdr):
    h = open(hdr, 'rb').read(); o = 8
    while o < len(h):
        l = struct.unpack_from('<H', h, o)[0]; nm = h[o + 2:o + 2 + l].decode('cp1252', 'replace').lower()
        off, sz = struct.unpack_from('<II', h, o + 2 + l); o += 2 + l + 8; pack[nm] = (off, sz)
srcf = open(os.path.join(GAME, 'UI', 'ui.src'), 'rb') if pack else None

def raw_tex(name):
    base = os.path.basename(name.replace('\\', '/')).lower()
    if base in pack:
        off, sz = pack[base]; srcf.seek(off); return srcf.read(sz), 'pack'
    for dd in FALLBACK_DIRS:
        p = os.path.join(dd, base)
        if os.path.exists(p): return open(p, 'rb').read(), dd
    raise FileNotFoundError(name)

_cache = {}
def load_tex(name):
    if name in _cache: return _cache[name]
    b, src = raw_tex(name)
    nl = struct.unpack_from('<i', b, 0)[0]; o = 4 + nl
    assert b[o:o + 3] == b'NTF', name
    ver = b[o + 3]; w, h, fmt, mip = struct.unpack_from('<IIII', b, o + 4); o += 20
    if fmt in (21, 22): bpp, pitch, rows = 4, w * 4, h
    elif fmt in (25, 26): bpp, pitch, rows = 2, w * 2, h
    elif fmt in (0x31545844, 0x33545844, 0x35545844):
        bs = 8 if fmt == 0x31545844 else 16; pitch, rows = (w // 4) * bs, h // 4
    else: raise ValueError('fmt %X' % fmt)
    data = bytearray(b[o:o + pitch * rows])
    if ver == 7 and fmt in (0x31545844, 0x33545844, 0x35545844):   # DXT: one RC4 stream over the whole level
        ks = rc4_stream(KEY, len(data))
        data = bytearray(a ^ c for a, c in zip(data, ks))
    elif ver == 7:
        ks = rc4_stream(KEY, pitch)
        for y in range(rows):
            row = data[y * pitch:(y + 1) * pitch]
            data[y * pitch:(y + 1) * pitch] = bytes(a ^ c for a, c in zip(row, ks))
    if fmt == 21: img = Image.frombytes('RGBA', (w, h), bytes(data), 'raw', 'BGRA')
    elif fmt == 22: img = Image.frombytes('RGBX', (w, h), bytes(data), 'raw', 'BGRX').convert('RGBA')
    elif fmt in (25, 26):   # A1R5G5B5 / A4R4G4B4 (PIL has no raw mode for these)
        px = bytearray(w * h * 4)
        for k, v in enumerate(struct.unpack_from('<%dH' % (w * h), data)):
            if fmt == 25:
                r_, g, b_, a = (v >> 10 & 31) * 255 // 31, (v >> 5 & 31) * 255 // 31, (v & 31) * 255 // 31, 255 if v & 0x8000 else 0
            else:
                a, r_, g, b_ = (v >> 12 & 15) * 17, (v >> 8 & 15) * 17, (v >> 4 & 15) * 17, (v & 15) * 17
            px[k * 4:k * 4 + 4] = bytes((r_, g, b_, a))
        img = Image.frombytes('RGBA', (w, h), bytes(px))
    else:
        n = {0x31545844: 1, 0x33545844: 2, 0x35545844: 3}[fmt]
        img = Image.frombytes('RGBA', (w, h), bytes(data), 'bcn', n)
    print('  tex %-36s %dx%d fmt=%X v%d from %s' % (os.path.basename(name), w, h, fmt, ver, src))
    _cache[name] = img
    return img

def crop(e):
    img = load_tex(e['tex']); W, H = img.size
    u0, v0, u1, v1 = e['uv']
    box = (round(u0 * W), round(v0 * H), round(u1 * W), round(v1 * H))
    l, t, r, b = e['rect']
    part = img.crop(box)
    if part.size != (r - l, b - t) and r > l and b > t:
        part = part.resize((r - l, b - t), Image.BILINEAR)
    return part

def compose(imgs, rect):
    l, t, r, b = rect
    canvas = Image.new('RGBA', (r - l, b - t), (0, 0, 0, 0))
    for e in imgs:
        el, et, er, eb = e['rect']
        if er <= el or eb <= et: continue
        canvas.alpha_composite(crop(e), (el - l, et - t))
    return canvas

def bbox(imgs):
    ls = [e['rect'] for e in imgs if e['rect'][2] > e['rect'][0]]
    return [min(x[0] for x in ls), min(x[1] for x in ls), max(x[2] for x in ls), max(x[3] for x in ls)]

def save_pus(img, name):
    img = img.convert('RGBA'); w, h = img.size
    px = bytearray()
    for r_, g, b, a in img.getdata():
        px += bytes((b * a // 255, g * a // 255, r_ * a // 255, a))
    with open(os.path.join(OUT, name + '.pus'), 'wb') as f:
        f.write(b'PUSI' + struct.pack('<II', w, h) + px)
    img.save(os.path.join(OUT, name + '.png'))

# ------------------------------------------------------------------ build
os.makedirs(OUT, exist_ok=True)
sprites = []   # (name, rect)

def add_sprite(name, imgs):
    if not imgs: return
    rect = bbox(imgs)
    save_pus(compose(imgs, rect), name)
    sprites.append((name, rect))

top_level = [k for k in ROOT['children'] if k['type'] == 'IMAGE']
add_sprite('head', top_level)
add_sprite('time', images(find(ROOT, 'group_time')))
add_sprite('rewhead', images(find(ROOT, 'group_middle_1')))
for i in range(3):
    add_sprite('tgt%d' % i, images(find(ROOT, 'group_%d' % i)))
    add_sprite('rew%d' % i, images(find(ROOT, 'requital_%d' % i)))
for bid in ('btn_min', 'btn_max', 'btn_close'):
    b = find(ROOT, bid)
    states = [k for k in b['children'] if k['type'] == 'IMAGE']
    for si, sname in enumerate(('n', 'd', 'o')):
        if si < len(states): add_sprite('%s_%s' % (bid, sname), [states[si]])

# preview of the whole panel (all groups + normal buttons)
full = [x for x in top_level]
for gid in ('group_time', 'group_middle_1', 'group_0', 'group_1', 'group_2', 'requital_0', 'requital_1', 'requital_2'):
    full += images(find(ROOT, gid))
full += [find(ROOT, 'btn_min')['children'][0], find(ROOT, 'btn_close')['children'][0]]
compose(full, ROOT['rect']).save(os.path.join(OUT, '_preview.png'))

# ------------------------------------------------------------------ layout header
texts = {}
def collect_text(e, prefix=''):
    if e['type'] == 'STRING':
        texts.setdefault(prefix + e['id'], (e['rect'], e['color'], e['fontH'], e['font']))
    for k in e['children']:
        collect_text(k, prefix)
collect_text(find(ROOT, 'group_time')); collect_text(find(ROOT, 'group_middle_1'))
tx = {k: v for k, v in texts.items()}
for i in range(3):
    for k in ('txt_needs_first', 'txt_needs_second', 'icon'):
        e = find(find(ROOT, 'group_%d' % i), k)
        tx['%s_%d' % (k, i)] = (e['rect'], e.get('color', 0), e.get('fontH', 0), e.get('font', ''))
    for k in ('txt_return_first', 'txt_return_second', 'txt_rate', 'icon'):
        e = find(find(ROOT, 'requital_%d' % i), k)
        tx['r%s_%d' % (k, i)] = (e['rect'], e.get('color', 0), e.get('fontH', 0), e.get('font', ''))
e = find(ROOT, 'text_event_name'); tx['text_event_name'] = (e['rect'], e['color'], e['fontH'], e['font'])
for bid in ('btn_min', 'btn_close'):
    tx[bid] = (find(ROOT, bid)['rect'], 0, 0, '')

L, T = ROOT['rect'][0], ROOT['rect'][1]
def rs(r): return '{ %d, %d, %d, %d }' % (r[0] - L, r[1] - T, r[2] - L, r[3] - T)
with open(os.path.join(HERE, 'cr_layout.h'), 'w') as f:
    f.write('// Generated by build_cr_assets.py from re_collection_race.uif -- do not edit by hand.\n#pragma once\n')
    f.write('static const int CRS_W = %d, CRS_H = %d;\n' % (ROOT['rect'][2] - L, ROOT['rect'][3] - T))
    f.write('struct CrsSprite { const char* name; int l, t, r, b; };\nstatic const CrsSprite CRS_SPRITES[] = {\n')
    for n, r in sprites:
        f.write('    { "%s", %s },\n' % (n, rs(r)[2:-2]))
    f.write('};\n')
    for k, (r, col, fh, font) in sorted(tx.items()):
        cname = 'CRS_' + k.upper()
        f.write('static const int %s[4] = %s;%s\n' % (cname, rs(r), '  // color %08X h%d %s' % (col, fh, font) if fh else ''))
print('sprites', len(sprites), '->', OUT)
