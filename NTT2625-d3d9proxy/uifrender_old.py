r"""Renders an old-format (2369 / HSACSX / 2383) UIF to PNG with the game's own atlases (kopanel.load_tex:
UI pack first). Same element reader as build_cr_assets.py. IMAGE = atlas uv crop scaled to the rect;
STRING = its text. Children are drawn in file order (earlier = under).
Usage: python uifrender_old.py file.uif out.png [--ids]   (--ids also writes element ids / rects to out.txt)"""
import sys, os, struct
from PIL import Image, ImageDraw, ImageFont
import kopanel as K

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
    e['style'] = r.u(); r.u(); r.s_(); r.s_(); r.s_()
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

def load(path):
    d = open(path, 'rb').read()
    r = R(d); root = elem(r, 0)
    assert r.o == len(d), 'UIF not fully consumed (%d/%d)' % (r.o, len(d))
    return root

_tex = {}
def tex(name):
    if name not in _tex:
        try: _tex[name] = K.tex(name)
        except Exception: _tex[name] = None
    return _tex[name]

def bounds(e, acc):
    l, t, r_, b = e['rect']
    if r_ > l and b > t: acc.append((l, t, r_, b))
    for k in e['children']: bounds(k, acc)
    return acc

def draw(e, img, dr, ox, oy, visible_only=False, lines=None, depth=0):
    l, t, r_, b = e['rect']
    if lines is not None:
        lines.append('%s%-8s %-28s %s %s' % ('  ' * depth, e['type'], e['id'][:28], e['rect'], e.get('tex', '') or repr(e.get('text', ''))[:30]))
    if e['type'] == 'IMAGE' and e.get('tex') and r_ > l and b > t:
        T = tex(e['tex'])
        if T is not None:
            W, H = T.size; u0, v0, u1, v1 = e['uv']
            box = (round(u0 * W), round(v0 * H), round(u1 * W), round(v1 * H))
            if box[2] > box[0] and box[3] > box[1]:
                part = T.crop(box).resize((r_ - l, b - t), Image.LANCZOS)
                img.alpha_composite(part, (l - ox, t - oy))
    elif e['type'] == 'STRING' and e.get('text'):
        c = e.get('color', 0xFFFFFFFF)
        col = ((c >> 16) & 255, (c >> 8) & 255, c & 255, 255)
        try: font = ImageFont.truetype('verdana.ttf', max(9, abs(e.get('fontH', 12))))
        except Exception: font = ImageFont.load_default()
        dr.text((l - ox + 2, t - oy), e['text'], fill=col, font=font)
    # children in file order: earlier drawn first
    for k in e['children']:
        draw(k, img, dr, ox, oy, visible_only, lines, depth + 1)

if __name__ == '__main__':
    root = load(sys.argv[1])
    rects = bounds(root, [])
    L = min(r[0] for r in rects); T = min(r[1] for r in rects); Rr = max(r[2] for r in rects); B = max(r[3] for r in rects)
    img = Image.new('RGBA', (Rr - L, B - T), (40, 60, 40, 255))
    dr = ImageDraw.Draw(img)
    lines = [] if '--ids' in sys.argv else None
    draw(root, img, dr, L, T, lines=lines)
    img.save(sys.argv[2])
    if lines is not None:
        open(os.path.splitext(sys.argv[2])[0] + '.txt', 'w', encoding='utf-8').write('\n'.join(lines))
    print('rendered', sys.argv[2], img.size, 'origin', (L, T))
