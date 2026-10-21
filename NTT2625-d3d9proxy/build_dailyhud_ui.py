"""Daily Quest kill counter skin: the game's own quest progress window (UI\\re_quest_completed.uif, the
"<< Sabertooth Hunt >>  Sabertooth : 2/15" panel) cut into three sprites that daily_panel.cpp stacks:

  hud_top.pus   title strip   (the uif's group_top, 289 x 48, the gold title goes on it)
  hud_line.pus  one text line (group_0..9, 285 x 20)
  hud_base.pus  bottom cap    (group_base, 289 x 18)

Output: daily_ui\\hud_*.pus (repo copy) and <client>\\JstKO\\daily_ui\\hud_*.pus.
The uif is read straight from the client's UI pack; its element version 3 layout differs from the older
files uifview.py knows (3 extra bytes after the common block, 4 after a string's text), handled here.
Usage: python build_dailyhud_ui.py
"""
import os
import sys
import types

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
CLIENT = r'D:\ai\JstKO Client'

import uif_crypt  # noqa: E402
import uifrender as U  # noqa: E402
import uifskin as S  # noqa: E402

src = open(os.path.join(HERE, 'uifview.py'), encoding='utf-8').read()
a = "r.s(); r.s(); r.s()   # state:"
assert a in src
code = src.replace(a, "r.s(); r.s(); r.s()\n    if e.ver >= 3: r.o += 3   # state:")
b = "e.fstyle = r.u(); e.color = r.u(); e.text = r.s()"
assert b in code
code = code.replace(b, b + "\n        if e.ver >= 3: r.o += 4")
V3 = types.ModuleType('uifview_v3')
exec(compile(code, 'uifview_v3', 'exec'), V3.__dict__)

tex = U.Tex(CLIENT)
raw = tex.raw('re_quest_completed.uif')
tmp = os.path.join(HERE, '..', 'uif_out', 're_quest_completed.plain.uif')
os.makedirs(os.path.dirname(tmp), exist_ok=True)
open(tmp, 'wb').write(raw)
nodes = V3.load(tmp)


def find(id_):
    for n in S.walk(nodes):
        if n.id == id_:
            return n
    raise KeyError(id_)


def piece(group_id):
    g = find(group_id)
    img = next(c for c in g.children if c.type == 'IMAGE')
    return S._crop(img), img


outs = {}
for out, gid in (('hud_top', 'group_top'), ('hud_line', 'group_0'), ('hud_base', 'group_base')):
    im, e = piece(gid)
    outs[out] = im
    print(out, im.size, e.uv)

title = find('txt_title')
msg = find('txt_msg_0')
print('title rect', title.rect, 'color %08X' % title.color, 'h', title.fontH)
print('msg rect', msg.rect, 'color %08X' % msg.color, 'h', msg.fontH)

for d in (os.path.join(HERE, 'daily_ui'), os.path.join(CLIENT, 'JstKO', 'daily_ui')):
    for name, im in outs.items():
        S.save_pus(im, os.path.join(d, name + '.pus'))
print('written')
