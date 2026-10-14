"""Reads the UIF files of uif_in\\ (the layout used by the supplied 2625 UIFs, same reader as uifdump.py) and
renders a PNG preview with the game's own textures.

Usage: python uifview.py file.uif [out.png] [gameDir]      (prints the element tree, writes the PNG)
The first top-level BASE is the window size; every top-level BASE gets its own PNG (out_<id>.png).
"""
import os
import struct
import sys

from PIL import Image

import uifrender as U
import uif_crypt

_raw = U.Tex.raw


def _raw_dec(self, name):
    b = _raw(self, name)
    try:
        return uif_crypt.decrypt(b)       # the client pack is HopeGuard-encrypted
    except ValueError:
        return b


U.Tex.raw = _raw_dec

TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON', 12: 'CHECK', 13: 'T13', 14: 'LIST'}


class E:
    typ = 0
    type = 'BASE'
    name = ''
    id = ''
    rect = (0, 0, 0, 0)
    tex = ''
    uv = (0, 0, 1, 1)
    text = ''
    color = 0xFFFFFF
    fontH = 12
    fstyle = 0
    style = 0
    state = 0
    font = ''

    def __init__(s):
        s.children = []


class Reader:
    def __init__(s, d):
        s.d, s.o = d, 0

    def i(s):
        v = struct.unpack_from('<i', s.d, s.o)[0]; s.o += 4; return v

    def u(s):
        v = struct.unpack_from('<I', s.d, s.o)[0]; s.o += 4; return v

    def h(s):
        v = struct.unpack_from('<H', s.d, s.o)[0]; s.o += 2; return v

    def f(s):
        v = struct.unpack_from('<f', s.d, s.o)[0]; s.o += 4; return v

    def s(s):
        n = s.i(); v = s.d[s.o:s.o + n].decode('cp1252', 'replace'); s.o += n; return v


def elem(r, typ):
    e = E(); e.typ = typ; e.type = TYPES.get(typ, str(typ)); e.name = r.s()
    n = r.h(); e.ver = r.h()
    e.children = [elem(r, r.i()) for _ in range(n)]
    e.id_off = r.o; e.id = r.s(); e.id_end = r.o; e.rect = tuple(r.i() for _ in range(4)); [r.i() for _ in range(4)]
    e.style = r.u(); e.state = r.u(); r.s(); r.s(); r.s()   # state: the button state of an image child (0 normal, 1 down, 2 on, 3 disabled)
    t = e.type
    if t == 'IMAGE':
        e.tex = r.s(); e.uv = tuple(r.f() for _ in range(4)); r.f()
    elif t == 'STRING':
        e.font = r.s(); e.fontH = r.i(); e.fstyle = r.u(); e.color = r.u(); e.text = r.s()
    elif t == 'BUTTON':
        [r.i() for _ in range(4)]; r.s(); r.s()
    elif t == 'STATIC':
        r.s()
    elif t == 'PROGRESS':
        pass
    elif t == 'AREA':
        r.i()
    elif t == 'EDIT':
        r.i(); r.i()
    elif t == 'LIST':
        r.s(); [r.i() for _ in range(4)]
    return e


def load(path):
    r = Reader(open(path, 'rb').read())
    r.s(); n = r.h(); r.h(); out = []
    for _ in range(n):
        try:
            out.append(elem(r, r.i()))
        except Exception as ex:  # noqa
            print('stop at', r.o, ex); break
    return out


def dump(e, depth=0):
    extra = e.tex + (' uv=%s' % [round(x, 3) for x in e.uv] if e.tex else '') + (' "%s"' % e.text if e.text else '')
    print('  ' * depth + '%s %s %s %s' % (e.type, e.id, list(e.rect), extra))
    for k in e.children:
        dump(k, depth + 1)


if __name__ == '__main__':
    path = sys.argv[1]
    tops = load(path)
    for t in tops:
        dump(t)
    if len(sys.argv) > 2:
        game = sys.argv[3] if len(sys.argv) > 3 else r'D:\ai\2625_Client'
        tex = U.Tex(game)
        base = os.path.splitext(sys.argv[2])[0]
        groups = [[t] for t in tops] if all(t.type == 'BASE' for t in tops) else [tops]
        for k, grp in enumerate(groups):
            w = max(max(t.rect[2] for t in grp), 8); h = max(max(t.rect[3] for t in grp), 8)
            bg = Image.new('RGBA', (w + 20, h + 20), (40, 60, 40, 255))
            for t in grp:
                try:
                    bg.alpha_composite(U.render(t, tex, (w + 20, h + 20), (), outline=True))
                except Exception as ex:  # noqa
                    print('render failed', t.id, ex)
            bg.save('%s_%d.png' % (base, k))
