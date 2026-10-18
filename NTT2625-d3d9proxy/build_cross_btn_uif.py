r"""Cross Exchange button as a native control of the game's inventory window (re_inventory.uif), so the game draws
it itself (under tooltips / dragged items, like every other inventory button). Art: the inventory's own bag tab
button (btn_bag1, ui\re_skill03.dxt) with its per-state caption strings, stretched into the free strip between
Btn_VipVault [242,276,271,305] and Btn_search [390,282,414,306]. The proxy (cross_exch.cpp) catches the click
on 'btn_cross' in the inventory's ReceiveMessage and opens the Cross Exchange panel (rce_store.cpp).
Output: genie_out\re_inventory.uif  ->  python pack_replace.py <client>\UI ui genie_out\re_inventory.uif
"""
import os
import uifed
import uifrender as U

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'genie_out', 're_inventory.uif')
src = os.path.join(HERE, 'genie_out', 're_inventory.orig.uif')
if not os.path.exists(src):                      # keep the game's original as the base for every rebuild
    open(src, 'wb').write(U.Tex(U.GAME).raw('re_inventory.uif'))
root = uifed.load(src)
assert root.find('btn_cross') is None

RECT = [276, 279, 386, 305]
bag = root.find('btn_bag1')
b = bag.clone()
b.id = 'btn_cross'
b.rect = RECT; b.mrect = RECT; b.btn = RECT
b.style = 0x00010000          # plain push button (the bag tab is a toggle: 0x00020000)
imgs = [x for x in b.children if x.type == 'IMAGE']
lit = imgs[2]                  # the bag tab's selected (lit) art + its bright caption
lit_uv = lit.uv
lit_color = next(x for x in lit.walk() if x.type == 'STRING').color
for im in b.walk():
    if im is b: continue
    if im.type == 'IMAGE':
        im.rect = RECT; im.mrect = RECT
        im.uv = lit_uv         # every state looks pressed/lit: no on/off look, a click just opens the panel
    elif im.type == 'STRING':
        r = [RECT[0] + 2, RECT[1] + 7, RECT[2] - 2, RECT[3] - 6]
        im.rect = r; im.mrect = r
        im.text = 'Cross Exchange'
        im.color = lit_color
root.add(b)
uifed.save(root, OUT)
chk = uifed.load(OUT).find('btn_cross')
print('btn_cross ->', OUT, chk.rect, [x.text for x in chk.walk() if x.type == 'STRING'])
