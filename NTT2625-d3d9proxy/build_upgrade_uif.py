"""Anvil instant rate: the game's own Item Upgrade window (re_itemupgrade.uif) gets an "Upgrade Rate / Coins"
row between the material frame and the Confirm / Cancel buttons (like the reference picture). Field art from
the game's ui\\re_skill04.dxt (the genie's field piece), labels in the window's own gold font (text_gold).
The proxy (anvil_rate.cpp) writes the values the server sends (WIZ_HSACS_HOOK + UPGRADE_RATE 0xC4).
Output: genie_out\\re_itemupgrade.uif  ->  python pack_replace.py <client>\\UI ui genie_out\\re_itemupgrade.uif
"""
import os, sys
import uifed
import uifrender as U

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'genie_out', 're_itemupgrade.uif')
src = os.path.join(HERE, 'genie_out', 're_itemupgrade.orig.uif')
if not os.path.exists(src):                      # keep the game's original as the base for every rebuild
    open(src, 'wb').write(U.Tex(U.GAME).raw('re_itemupgrade.uif'))
root = uifed.load(src)

def find(e, id_):
    for x in e.walk():
        if x.id == id_: return x
    return None

gold = find(root, 'text_gold')
img_t = next(x for x in root.walk() if x.type == 'IMAGE' and x.tex)
SK_FIELD = (506, 229, 696, 261)

def image(rect, box, tex='re_skill04.dxt', size=1024):
    e = img_t.clone(); e.children = []; e.id = ''
    e.tex = 'ui\\' + tex
    e.uv = [box[0] / size, box[1] / size, box[2] / size, box[3] / size]
    e.rect = list(rect); e.mrect = [0, 0, 0, 0]
    return e

def text(id_, s, rect, color, center=False):
    e = gold.clone(); e.children = []; e.id = id_; e.text = s
    e.rect = list(rect); e.mrect = [0, 0, 0, 0]; e.color = color
    e.style = 0x04900000 if center else 0x04300000
    return e

Y0, Y1 = 285, 302
fields = [image([150, Y0, 214, Y1], SK_FIELD), image([262, Y0, 368, Y1], SK_FIELD)]
win = gold.parent                                             # the window base that holds text_gold
# after the window's background images, before its first button (earlier in the file = drawn under)
at = next(i for i, c in enumerate(win.children) if c.type == 'BUTTON')
for f in reversed(fields):
    win.add(f, at)
for t in (text('hg_upg_rate_lbl', 'Upgrade Rate :', [64, Y0 + 2, 148, Y1], 0xFFFFFFFF),
          text('hg_upg_rate', '-', [152, Y0 + 2, 212, Y1], 0xFFEBC738, center=True),
          text('hg_upg_coins_lbl', 'Coins :', [218, Y0 + 2, 260, Y1], 0xFFFFFFFF),
          text('hg_upg_coins', '-', [264, Y0 + 2, 366, Y1], 0xFFEBC738, center=True)):
    win.add(t)

uifed.save(root, OUT)
print('wrote', OUT)
