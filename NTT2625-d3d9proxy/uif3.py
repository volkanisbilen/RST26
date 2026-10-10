"""Reader for the newer (2625) KO .uif layout: child count as u16 + u16 version after the name."""
import struct, sys
TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON', 12: 'CHECK', 13: 'T13', 14: 'LIST'}
class R:
    def __init__(s, d): s.d, s.o = d, 0
    def i(s): v = struct.unpack_from('<i', s.d, s.o)[0]; s.o += 4; return v
    def u(s): v = struct.unpack_from('<I', s.d, s.o)[0]; s.o += 4; return v
    def h(s): v = struct.unpack_from('<H', s.d, s.o)[0]; s.o += 2; return v
    def b(s): v = s.d[s.o]; s.o += 1; return v
    def f(s): v = struct.unpack_from('<f', s.d, s.o)[0]; s.o += 4; return v
    def s_(s):
        n = s.i(); v = s.d[s.o:s.o + n].decode('cp1252', 'replace'); s.o += n; return v
def peek(r, n=24): return r.d[r.o:r.o+n].hex()

def elem(r, typ):
    e = {'type': TYPES.get(typ, 'T%d' % typ), 'typ': typ, 'name': r.s_(), 'at': r.o}
    n = r.h(); e['ver'] = r.h()
    e['children'] = [elem(r, r.i()) for _ in range(n)]
    e['id'] = r.s_(); e['rect'] = [r.i() for _ in range(4)]; e['mrect'] = [r.i() for _ in range(4)]
    e['style'] = r.u(); r.u(); e['tip'] = r.s_(); r.s_(); r.s_()
    e['x1'] = r.b(); e['x2'] = r.h()
    t = e['type']
    if t == 'IMAGE': e['tex'] = r.s_(); e['uv'] = [round(r.f(), 4) for _ in range(4)]; e['anim'] = r.f()
    elif t == 'STRING': e['font'] = r.s_(); e['fontH'] = r.i(); e['fstyle'] = r.u(); e['color'] = r.u(); e['text'] = r.s_(); e['sx'] = r.u()
    elif t == 'BUTTON': e['btn'] = [r.i() for _ in range(4)]; r.s_(); r.s_()
    elif t == 'STATIC': r.s_()
    elif t == 'PROGRESS': pass
    elif t == 'AREA': e['area'] = r.i()
    elif t == 'EDIT': e['edit'] = [r.u(), r.u()]
    elif t == 'LIST': e['font'] = r.s_(); e['fontH'] = r.i(); e['color'] = r.u(); e['lx'] = [r.u(), r.u()]
    return e

def load(path):
    d = open(path, 'rb').read(); r = R(d)
    root = {'type': 'ROOT', 'name': r.s_()}
    n = r.h(); root['ver'] = r.h()
    root['children'] = []
    for _ in range(n):
        root['children'].append(elem(r, r.i()))
    return root, r.o, len(d)

def dump(e, depth=0, out=None):
    extra = e.get('tex', '') + (' uv=%s' % e['uv'] if 'uv' in e else '') + (' "%s"' % e['text'] if e.get('text') else '')
    print('  ' * depth + '%s %s %s %s' % (e['type'], e.get('id', ''), e.get('rect', ''), extra))
    for k in e.get('children', []): dump(k, depth + 1)

if __name__ == '__main__':
    root, o, n = load(sys.argv[1])
    dump(root)
    print('consumed', o, 'of', n)
