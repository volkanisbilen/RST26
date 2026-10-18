"""Editor for 2625 .uif files (the layout uif3.py reads), byte-exact roundtrip.

Every element keeps its raw bytes: head (type + name), u16 child count, u16 version, children,
tail (id .. end). Edits patch the tail in place (rect / mrect / text) or move whole elements,
so everything we do not understand is written back unchanged.
"""
import struct, copy

TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON', 12: 'CHECK', 13: 'T13', 14: 'LIST'}


class R:
    def __init__(s, d, o=0): s.d, s.o = d, o
    def i(s): v = struct.unpack_from('<i', s.d, s.o)[0]; s.o += 4; return v
    def u(s): v = struct.unpack_from('<I', s.d, s.o)[0]; s.o += 4; return v
    def h(s): v = struct.unpack_from('<H', s.d, s.o)[0]; s.o += 2; return v
    def b(s): v = s.d[s.o]; s.o += 1; return v
    def f(s): v = struct.unpack_from('<f', s.d, s.o)[0]; s.o += 4; return v
    def s_(s):
        n = s.i(); v = s.d[s.o:s.o + n].decode('cp1252', 'replace'); s.o += n; return v


class E:
    """One element. tail = bytearray starting at the id string."""
    def __init__(s): s.children = []; s.parent = None

    # ---- tail field access (offsets recomputed on demand: strings may change length)
    def _layout(s):
        r = R(s.tail)
        L = {}
        L['id'] = r.o; s_id = r.s_()
        L['rect'] = r.o; r.o += 16
        L['mrect'] = r.o; r.o += 16
        L['style'] = r.o; r.o += 8
        L['tip'] = r.o; r.s_(); r.s_(); r.s_()
        r.o += 3
        t = s.type
        if t == 'IMAGE':
            L['tex'] = r.o; r.s_(); L['uv'] = r.o; r.o += 16
        elif t == 'STRING':
            L['font'] = r.o; r.s_(); L['fontH'] = r.o; r.o += 4; L['fstyle'] = r.o; r.o += 4
            L['color'] = r.o; r.o += 4; L['text'] = r.o; r.s_()
        elif t == 'BUTTON':
            L['btn'] = r.o                  # click region (the game hit-tests this, not rect)
        return L

    def _str_at(s, off): return R(s.tail, off).s_()
    def _set_str_at(s, off, v):
        n = struct.unpack_from('<i', s.tail, off)[0]
        enc = v.encode('cp1252')
        s.tail[off:off + 4 + n] = struct.pack('<i', len(enc)) + enc

    @property
    def id(s): return s._str_at(0)
    @id.setter
    def id(s, v): s._set_str_at(0, v)
    @property
    def rect(s): return list(struct.unpack_from('<4i', s.tail, s._layout()['rect']))
    @rect.setter
    def rect(s, v): struct.pack_into('<4i', s.tail, s._layout()['rect'], *v)
    @property
    def mrect(s): return list(struct.unpack_from('<4i', s.tail, s._layout()['mrect']))
    @mrect.setter
    def mrect(s, v): struct.pack_into('<4i', s.tail, s._layout()['mrect'], *v)
    @property
    def style(s): return struct.unpack_from('<I', s.tail, s._layout()['style'])[0]
    @style.setter
    def style(s, v): struct.pack_into('<I', s.tail, s._layout()['style'], v)
    @property
    def btn(s): return list(struct.unpack_from('<4i', s.tail, s._layout()['btn']))
    @btn.setter
    def btn(s, v): struct.pack_into('<4i', s.tail, s._layout()['btn'], *v)
    @property
    def tex(s): return s._str_at(s._layout()['tex']) if s.type == 'IMAGE' else None
    @tex.setter
    def tex(s, v): s._set_str_at(s._layout()['tex'], v)
    @property
    def uv(s): return list(struct.unpack_from('<4f', s.tail, s._layout()['uv']))
    @uv.setter
    def uv(s, v): struct.pack_into('<4f', s.tail, s._layout()['uv'], *v)
    @property
    def text(s): return s._str_at(s._layout()['text']) if s.type == 'STRING' else None
    @text.setter
    def text(s, v): s._set_str_at(s._layout()['text'], v)
    @property
    def font(s): return s._str_at(s._layout()['font'])
    @property
    def fontH(s): return struct.unpack_from('<i', s.tail, s._layout()['fontH'])[0]
    @fontH.setter
    def fontH(s, v): struct.pack_into('<i', s.tail, s._layout()['fontH'], v)
    @property
    def fstyle(s): return struct.unpack_from('<I', s.tail, s._layout()['fstyle'])[0]
    @fstyle.setter
    def fstyle(s, v): struct.pack_into('<I', s.tail, s._layout()['fstyle'], v)
    @property
    def color(s): return struct.unpack_from('<I', s.tail, s._layout()['color'])[0]
    @color.setter
    def color(s, v): struct.pack_into('<I', s.tail, s._layout()['color'], v)

    # ---- tree ops
    def walk(s):
        yield s
        for c in s.children: yield from c.walk()

    def find(s, id_):
        for e in s.walk():
            if e is not s and e.id == id_: return e
        return None

    def move(s, dx, dy):
        """shift this element and all descendants"""
        for e in s.walk():
            if e.typ is None: continue
            l, t, r, b = e.rect; e.rect = [l + dx, t + dy, r + dx, b + dy]
            ml, mt, mr, mb = e.mrect
            if (ml, mt, mr, mb) != (0, 0, 0, 0): e.mrect = [ml + dx, mt + dy, mr + dx, mb + dy]

    def move_to(s, x, y):
        l, t, _, _ = s.rect; s.move(x - l, y - t)

    def resize(s, w, h):
        l, t, _, _ = s.rect; s.rect = [l, t, l + w, t + h]

    def clone(s):
        c = copy.deepcopy(s)
        return c

    def add(s, child, index=None):
        child.parent = s
        if index is None: s.children.append(child)
        else: s.children.insert(index, child)
        return child

    def remove(s):
        s.parent.children.remove(s); s.parent = None

    def __repr__(s):
        return '<%s %r %s>' % (s.type, s.id, s.rect)


def _parse(r, typ):
    e = E(); e.typ = typ; e.type = TYPES.get(typ, 'T%d' % typ)
    st = r.o; r.s_(); e.head = bytes(r.d[st:r.o])        # name string (type int written by parent)
    n = r.h(); e.ver = r.h()
    for _ in range(n):
        c = _parse(r, r.i()); c.parent = e; e.children.append(c)
    st = r.o
    r.s_(); r.o += 32; r.o += 8; r.s_(); r.s_(); r.s_(); r.o += 3
    t = e.type
    if t == 'IMAGE': r.s_(); r.o += 20
    elif t == 'STRING': r.s_(); r.o += 12; r.s_(); r.o += 4
    elif t == 'BUTTON': r.o += 16; r.s_(); r.s_()
    elif t == 'STATIC': r.s_()
    elif t == 'PROGRESS': pass
    elif t == 'AREA': r.o += 4
    elif t == 'EDIT': r.o += 8
    elif t == 'LIST': r.s_(); r.o += 16
    e.tail = bytearray(r.d[st:r.o])
    return e


def load(path):
    d = open(path, 'rb').read(); r = R(d)
    root = E(); root.typ = None; root.type = 'ROOT'
    st = r.o; r.s_(); root.head = d[st:r.o]
    n = r.h(); root.ver = r.h()
    for _ in range(n):
        c = _parse(r, r.i()); c.parent = root; root.children.append(c)
    root.tail = bytearray(d[r.o:])    # trailing bytes (kept verbatim)
    return root


def _ser(e, out):
    if e.typ is not None: out += struct.pack('<i', e.typ)
    out += e.head + struct.pack('<HH', len(e.children), e.ver)
    for c in e.children: _ser(c, out)
    out += e.tail


def dumps(root):
    out = bytearray(); _ser(root, out); return bytes(out)


def save(root, path): open(path, 'wb').write(dumps(root))


def dump(e, depth=0):
    if e.typ is None: print('ROOT')
    else:
        extra = ''
        if e.type == 'IMAGE': extra = '%s uv=%s' % (e.tex, [round(x, 4) for x in e.uv])
        if e.type == 'STRING': extra = '"%s" h=%d col=%08X' % (e.text, e.fontH, e.color)
        print('  ' * depth + '%s %s %s %s' % (e.type, e.id, e.rect, extra))
    for c in e.children: dump(c, depth + 1)


if __name__ == '__main__':
    import sys
    d = open(sys.argv[1], 'rb').read()
    root = load(sys.argv[1])
    print('roundtrip', 'OK' if dumps(root) == d else 'MISMATCH', len(d))
    if len(sys.argv) > 2: dump(root)
