"""Dumps the element tree of a KO .uif (same reader as build_rce_assets.py). Usage: python uifdump.py file.uif"""
import struct, sys
TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON'}
d = open(sys.argv[1], 'rb').read(); o = 0
def I():
    global o; v = struct.unpack_from('<i', d, o)[0]; o += 4; return v
def U():
    global o; v = struct.unpack_from('<I', d, o)[0]; o += 4; return v
def F():
    global o; v = struct.unpack_from('<f', d, o)[0]; o += 4; return v
def S():
    global o; n = I(); v = d[o:o + n].decode('cp1252', 'replace'); o += n; return v
def elem(typ):
    e = {'type': TYPES.get(typ, str(typ)), 'name': S()}
    e['children'] = [elem(I()) for _ in range(I())]
    e['id'] = S(); e['rect'] = [I() for _ in range(4)]; [I() for _ in range(4)]; U(); U(); S(); S(); S()
    t = e['type']
    if t == 'IMAGE': e['tex'] = S(); e['uv'] = [round(F(), 4) for _ in range(4)]; F()
    elif t == 'STRING': e['font'] = S(); e['fontH'] = I(); U(); e['color'] = U(); e['text'] = S()
    elif t == 'BUTTON': [I() for _ in range(4)]; S(); S()
    elif t == 'STATIC': S()
    elif t == 'PROGRESS': F(); I(); I()
    elif t == 'AREA': I()
    elif t == 'SCROLLBAR': pass
    return e
def dump(e, depth=0):
    extra = e.get('tex', '') + (' uv=%s' % e['uv'] if 'uv' in e else '') + (' "%s"' % e['text'] if e.get('text') else '')
    print('  ' * depth + '%s %s %s %s' % (e['type'], e['id'], e['rect'], extra))
    for k in e['children']: dump(k, depth + 1)
S(); n = I()
for _ in range(n):
    try: dump(elem(I()))
    except Exception as ex: print('stop at', o, ex); break
