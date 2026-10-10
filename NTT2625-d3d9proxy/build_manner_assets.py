"""Special Store [Loyalty & Manner] skin (manner_store.cpp), made from the game's own atlas
ui\\tournament_chaos_2021_a01.dxt (same set as the lottery / event reward panels).
Layout = the reference picture: title plate + close, five tabs (Warrior / Rogue / Mage / Priest / Power Store),
2 x 8 item slots with page arrows, item name bar, then three boxes: balance (manner / loyalty), the selected item
(price, count, BUY ITEM) and the daily "Completion Reward" (GET REWARD).
Output: manner_ui\\*.pus + manner_layout.h, item icons + names of every MANNER_STORE_ITEM in manner_ui\\.
Usage: python build_manner_assets.py
"""
import os
from PIL import Image
import kopanel as K

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'manner_ui')
T = K.tex('tournament_chaos_2021_a01.dxt')

FRAME     = K.piece(T, (333, 66, 444, 181))     # dark rounded box
PANELBG   = K.piece(T, (0, 858, 440, 916))      # brown textured bar with gold border: window body
GOLDFRAME = K.piece(T, (4, 26, 96, 120))        # gold square frame (item slot)
FIELD     = K.piece(T, (184, 26, 468, 66))      # dark field with gold border
X = [K.piece(T, (182, 66, 216, 99)), K.piece(T, (216, 66, 250, 99)), K.piece(T, (250, 66, 284, 99))]
SHIELD = {'gold': K.piece(T, (184, 98, 218, 122)), 'silver': K.piece(T, (218, 98, 252, 122))}
ARROW_UP = [K.piece(T, (510, 125, 533, 147)), K.piece(T, (510, 170, 533, 192))]   # normal / lit
BAR = {k: K.piece(T, b) for k, b in {
    'red': (4, 425, 196, 495), 'pink': (4, 495, 196, 565), 'dark': (4, 565, 196, 637),
    'grey': (4, 637, 196, 707), 'green': (4, 707, 196, 777), 'blue': (4, 777, 196, 850)}.items()}
PURPLE = K.piece(T, (0, 915, 438, 968))
REWARD_LABEL = K.piece(T, (40, 383, 395, 423))  # "Completion Reward List" lettering

W, H = 520, 404
L = {}
def r(name, x0, y0, x1, y1): L[name] = (x0, y0, x1, y1); return (x0, y0, x1, y1)

bg = K.slice9(PANELBG, W, H, 12)
def put(img, rect): bg.alpha_composite(img, rect[:2])

put(K.slice3(BAR['blue'], 300, 28, 14), r('title', 110, 8, 410, 36))
r('close', W - 34, 10, W - 10, 34)

TAB_W, TAB_H, TAB_GAP = 94, 24, 4
tx = (W - (5 * TAB_W + 4 * TAB_GAP)) // 2
for i in range(5):
    r('tab_%d' % i, tx + i * (TAB_W + TAB_GAP), 44, tx + i * (TAB_W + TAB_GAP) + TAB_W, 44 + TAB_H)

# 2 x 8 slots
put(K.slice9(FRAME, W - 24, 132, 14), r('grid', 12, 74, W - 12, 206))
S, GAP = 48, 10
gx = (W - (8 * S + 7 * GAP)) // 2
for row in range(2):
    for col in range(8):
        x, y = gx + col * (S + GAP), 82 + row * (S + 10)
        put(K.slice9(GOLDFRAME, S, S, 10), r('slot_%d' % (row * 8 + col), x, y, x + S, y + S))
r('page', W // 2 - 30, 190, W // 2 + 30, 206)
r('prev', W // 2 - 56, 189, W // 2 - 36, 207)
r('next', W // 2 + 36, 189, W // 2 + 56, 207)

put(K.slice3(PURPLE, W - 24, 26, 14), r('name_bar', 12, 212, W - 12, 238))

# bottom boxes
BY0, BY1 = 246, H - 30
put(K.slice9(FRAME, 160, BY1 - BY0, 14), r('box_balance', 12, BY0, 172, BY1))
put(K.slice9(FRAME, 160, BY1 - BY0, 14), r('box_item', 180, BY0, 340, BY1))
put(K.slice9(FRAME, W - 360, BY1 - BY0, 14), r('box_reward', 348, BY0, W - 12, BY1))

put(K.slice3(FIELD, 136, 22, 10, mid=(20, 60)), r('bal_title', 24, BY0 + 10, 160, BY0 + 32))
put(K.fit(SHIELD['silver'], 24, 24), r('bal_manner_icon', 26, BY0 + 46, 50, BY0 + 70))
r('bal_manner', 54, BY0 + 46, 166, BY0 + 70)
put(K.fit(SHIELD['gold'], 24, 24), r('bal_loyalty_icon', 26, BY0 + 82, 50, BY0 + 106))
r('bal_loyalty', 54, BY0 + 82, 166, BY0 + 106)

put(K.slice9(GOLDFRAME, S, S, 10), r('sel_slot', 192, BY0 + 12, 192 + S, BY0 + 12 + S))
put(K.slice3(FIELD, 82, 22, 10, mid=(20, 60)), r('sel_price', 246, BY0 + 12, 328, BY0 + 34))
put(K.slice3(FIELD, 82, 22, 10, mid=(20, 60)), r('sel_count', 246, BY0 + 38, 328, BY0 + 60))
r('sel_days', 188, BY0 + 64, 334, BY0 + 80)
r('buy', 200, BY0 + 84, 320, BY0 + 110)

lab = K.fit(REWARD_LABEL, 150, int(150 * REWARD_LABEL.size[1] / REWARD_LABEL.size[0]))
put(lab, r('reward_label', 348 + (W - 360 - 150) // 2, BY0 + 10, 348 + (W - 360 - 150) // 2 + 150, BY0 + 10 + lab.size[1]))
r('reward_text', 356, BY0 + 32, W - 20, BY0 + 50)
r('reward_time', 356, BY0 + 52, W - 20, BY0 + 70)
r('reward', 370, BY0 + 80, W - 34, BY0 + 108)

r('msg', 12, H - 28, W - 12, H - 8)

os.makedirs(OUT, exist_ok=True)
K.save_pus(bg, os.path.join(OUT, 'bg.pus'))
for st, key in (('n', 'dark'), ('o', 'blue'), ('s', 'green')):
    K.save_pus(K.slice3(BAR[key], TAB_W, TAB_H, 12), os.path.join(OUT, 'tab_%s.pus' % st))
for st, key in (('n', 'pink'), ('o', 'red'), ('d', 'dark')):
    K.save_pus(K.slice3(BAR[key], 120, 26, 12), os.path.join(OUT, 'buy_%s.pus' % st))
for st, key in (('n', 'red'), ('o', 'pink'), ('d', 'dark'), ('x', 'grey')):
    w = L['reward'][2] - L['reward'][0]
    K.save_pus(K.slice3(BAR[key], w, 28, 12), os.path.join(OUT, 'reward_%s.pus' % st))
for st, img in zip(('n', 'o', 'd'), X):
    K.save_pus(K.fit(img, 24, 24), os.path.join(OUT, 'close_%s.pus' % st))
for st, img in zip(('n', 'o'), ARROW_UP):
    K.save_pus(K.fit(img.rotate(90, expand=True), 20, 18), os.path.join(OUT, 'prev_%s.pus' % st))
    K.save_pus(K.fit(img.rotate(-90, expand=True), 20, 18), os.path.join(OUT, 'next_%s.pus' % st))
sel = Image.new('RGBA', (S, S), (0, 0, 0, 0))
from PIL import ImageDraw
d = ImageDraw.Draw(sel)
for k in range(3):
    d.rectangle((k, k, S - 1 - k, S - 1 - k), outline=(120, 255, 160, 230 - k * 60))
K.save_pus(sel, os.path.join(OUT, 'sel.pus'))

prev = bg.copy()
for name, img in (('buy', 'buy_n'), ('reward', 'reward_n'), ('close', 'close_n'), ('prev', 'prev_n'), ('next', 'next_n')):
    prev.alpha_composite(Image.open(os.path.join(OUT, img + '.png')), L[name][:2])
for i in range(5):
    prev.alpha_composite(Image.open(os.path.join(OUT, 'tab_%s.png' % ('s' if i == 0 else 'n'))), L['tab_%d' % i][:2])
prev.save(os.path.join(OUT, '_preview.png'))

with open(os.path.join(HERE, 'manner_layout.h'), 'w') as f:
    f.write('// Generated by build_manner_assets.py -- do not edit by hand.\n#pragma once\n')
    f.write('static const int MS_W = %d, MS_H = %d;\n' % (W, H))
    for k, v in L.items():
        f.write('static const int MS_%s[4] = { %d, %d, %d, %d };\n' % ((k.upper(),) + v))

# icons + names of every item in the store
rows = K.sql('SELECT nItemID FROM MANNER_STORE_ITEM')
ids = [int(x) for x in rows.split() if x.strip().isdigit()]
K.item_icons(ids, OUT)
print('manner store skin ->', OUT)
