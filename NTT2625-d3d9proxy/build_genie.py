"""Builds the HopeGuard Genie window (re_genie.uif) from the game's own Knight Genie UIF.

Native code finds every control by id among the direct children of its tab base (CN3UIBase
child list, non-recursive), so each control stays in its base; only positions change.
Tabs: Main = attack + assist bases, Misc = etc + recovery bases (client_patches.cpp rewires
the five native tab functions). Explanation / Recover / Support tab buttons are parked off
screen. Controls prefixed hg_ are new (the proxy drives them); the rest are native.
Frame: re_auto_01 / re_auto_02 sliced at plain columns / rows and widened to 534 x 620.
Usage: python build_genie.py [preview.png]
"""
import sys, os, struct
import uifed

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, 're_genie.uif')            # originals extracted from the UI pack
SRC_SUB = os.path.join(HERE, 're_genie_sub.uif')
OUTDIR = os.path.join(HERE, 'genie_out')              # pack names: python pack_replace.py <UI> ui genie_out\*.uif
OUT = os.path.join(OUTDIR, 're_genie.uif')
OUT_SUB = os.path.join(OUTDIR, 're_genie_sub.uif')
PREVIEW = sys.argv[1] if len(sys.argv) > 1 else None

W, H = 580, 650
PARK = -4000

root = uifed.load(SRC)
genie = root.children[0]
B = {b.id: b for b in genie.children if b.type == 'BASE'}
attack, assist, recovery, etc, explain = B['attack'], B['assist'], B['recovery'], B['etc'], B['explain']

# ---------------------------------------------------------------- helpers
def get(base, id_):
    for c in base.children:
        if c.id == id_: return c
    raise KeyError(id_)

def park(e):
    l, t, _, _ = e.rect
    e.move(PARK - l, PARK - t)

def fit(e, rect):
    """move + scale e and its subtree into rect (affine on every descendant rect)"""
    l0, t0, r0, b0 = e.rect
    l, t, r, b = rect
    sx = (r - l) / max(1, r0 - l0); sy = (b - t) / max(1, b0 - t0)
    for d in e.walk():
        dl, dt, dr, db = d.rect
        d.rect = [round(l + (dl - l0) * sx), round(t + (dt - t0) * sy), round(l + (dr - l0) * sx), round(t + (db - t0) * sy)]

def fit_scroll(e, x, y, w):
    """horizontal scrollbar: arrows keep 25 px, track stretches"""
    l0, t0, r0, b0 = e.rect
    e.move(x - l0, y - t0)
    e.rect = [x, y, x + w, y + (b0 - t0)]
    btns = [c for c in e.children if c.type == 'BUTTON']
    trk = [c for c in e.children if c.type == 'TRACKBAR'][0]
    lb, rb = sorted(btns, key=lambda c: c.rect[0])
    rl, rt, rr, rbm = rb.rect
    rb.move(x + w - 26 - rl, 0)
    tl, tt, tr, tb = trk.rect
    trk.rect = [x + 24, tt, x + w - 26, tb]
    img_bg, thumb = trk.children[0], trk.children[1]
    img_bg.rect = [x + 24, tt, x + w - 26, tb]
    th = thumb.rect; thumb.move(x + 24 - th[0], 0)

IMG_T = next(c for c in attack.children if c.type == 'IMAGE')          # any image: template
STR_T = get(attack, 'str_not_attack')                                    # Verdana 10 white, left
STR_C = get(attack, 'Attack_range_str')                                  # centred value text
CHK_T = get(attack, 'btn_not_attack')
BTN_ONOFF = (get(attack, 'btn_auto_attack_on'), get(attack, 'btn_auto_attack_off'))

def image(parent, tex, src, rect, index=0, size=512):
    """src = pixel box in a size x size atlas; decorations go first in the child list (drawn under controls)"""
    e = IMG_T.clone(); e.children = []
    e.id = ''
    e.tex = 'ui\\' + tex
    e.uv = [src[0] / size, src[1] / size, src[2] / size, src[3] / size]
    e.rect = list(rect); e.mrect = [0, 0, 0, 0]
    return parent.add(e, index)

def text(parent, id_, s, rect, color=0xFFFFFFFF, center=False, h=None, bold=False, tmpl=None):
    e = (tmpl or (STR_C if center else STR_T)).clone(); e.children = []
    e.id = id_; e.text = s; e.rect = list(rect); e.mrect = [0, 0, 0, 0]
    e.color = color
    if h: e.fontH = h
    if bold: e.fstyle = 1
    return parent.add(e)

def check(parent, id_, x, y, label=None, w=190, color=0xFFFFFFFF):
    e = CHK_T.clone(); e.id = id_
    for img in (c for c in e.children if c.type == 'IMAGE'): img.rect = [x, y, x + 14, y + 14]
    e.rect = [x - 2, y - 2, x + 19 + w, y + 16]        # click anywhere on the row
    parent.add(e)
    if label: text(parent, id_ + '_str', label, [x + 19, y - 1, x + 19 + w, y + 15], color)
    return e

# atlas pieces (re_auto_02 unless noted)
PLATE = (24, 127, 216, 154)        # section title plate with the round stud
ONOFF_FRAME = (16, 11, 170, 40)
SLOT = (105, 47, 146, 87)
BOXFRAME = (186, 10, 249, 50)      # style-button frame
BLACK = (25, 49, 93, 68)
BTN_N, BTN_H, BTN_D, BTN_X = (391, 20, 459, 41), (313, 14, 381, 42), (313, 61, 381, 82), (391, 61, 459, 82)
LINE = 're_map.dxt', (250, 128, 338, 137)

def section(parent, title, x, y, w, sid):
    image(parent, 're_auto_02.dxt', PLATE, [x, y, x + 192, y + 27])
    text(parent, sid, title, [x + 26, y + 6, x + 190, y + 22], 0xFF80FFFF, h=10, bold=True)

def push_button(parent, id_, label, rect):
    b = BTN_ONOFF[0].clone(); b.id = id_
    fit(b, rect)
    imgs = [c for c in b.children if c.type == 'IMAGE']
    for img, src in zip(imgs, (BTN_N, BTN_D, BTN_H, BTN_X)):
        img.uv = [src[0] / 512, src[1] / 512, src[2] / 512, src[3] / 512]
        img.rect = list(rect)
    for k, img in enumerate(imgs):      # states: normal, down, over, disabled
        for s in (d for d in img.walk() if d.type == 'STRING'):
            s.text = label; s.rect = [rect[0], rect[1] + 3, rect[2], rect[3] - 3]
            s.color = 0xFF808080 if k == 3 else 0xFFFFFFFF
    return parent.add(b)

def onoff(parent, on, off, x, y, w=146):
    image(parent, 're_auto_02.dxt', ONOFF_FRAME, [x, y, x + w, y + 30])
    bw = (w - 10) // 2
    fit(on, [x + 4, y + 5, x + 4 + bw, y + 24])
    fit(off, [x + 6 + bw, y + 5, x + 6 + 2 * bw, y + 24])

def title_plate(parent, sid, s, x, y, w, color=0xFFFFE08A):
    image(parent, 're_auto_02.dxt', BLACK, [x, y, x + w, y + 20])
    text(parent, sid, s, [x, y + 3, x + w, y + 17], color, center=True, h=9, bold=True)

def slot_frame(parent, x, y, s=35):
    image(parent, 're_auto_02.dxt', SLOT, [x, y, x + s, y + s])

# ---------------------------------------------------------------- frame (genie base)
old_imgs = [c for c in genie.children if c.type == 'IMAGE']
for c in old_imgs: c.remove()
COLS = [((4, 93), 2), ((62, 93), 91), ((62, 93), 122), ((62, 85), 153), ((93, 382), 176), ((336, 382), 465), ((336, 359), 511), ((382, 424), 534)]
ROWS_TOP = [((5, 255), 0), ((204, 255), 250), ((225, 255), 301), ((255, 486), 331)]
k = 0
for (sy0, sy1), dy in ROWS_TOP:
    for (sx0, sx1), dx in COLS:
        image(genie, 're_auto_01.dxt', (sx0, sy0, sx1, sy1), [dx, dy, dx + sx1 - sx0, dy + sy1 - sy0], k); k += 1
for (sx0, sx1), dx in COLS:
    image(genie, 're_auto_02.dxt', (sx0 + 1, 164, sx1 + 1, 241), [dx, 561, dx + sx1 - sx0, 638], k); k += 1
genie.rect = [0, 0, W, H]
for b in B.values(): b.rect = [0, 0, W, H]

# title + close
park(get(genie, 'str_genie_name'))
t = text(genie, 'hg_title', 'HopeGuard Genie', [220, 27, 389, 47], 0xFFEDEA72, center=True, h=10, bold=True,
         tmpl=get(genie, 'str_genie_name'))
close = get(genie, 'btn_close'); fit(close, [538, 12, 572, 45])
# the dialog's own region / drag bar live in the root element (trailing bytes): clicks outside it are ignored
root.type = 'ROOT'
root.rect = [0, 0, W, H]; root.mrect = [32, 12, W - 48, 45]

# tabs: Main = btn_attack, Misc = btn_etc
for tid in ('btn_explan', 'btn_recovery', 'btn_assist'): park(get(genie, tid))
for tid, label, x in (('btn_attack', 'Main', 62), ('btn_etc', 'Misc', 286)):
    tb = get(genie, tid)
    fit(tb, [x, 71, x + 186, 100])
    for s in (d for d in tb.walk() if d.type == 'STRING'):
        s.text = label; s.fontH = 10; s.fstyle = 1
        s.rect = [x, 79, x + 186, 93]

# explanation page is not used
for c in list(explain.children): park(c)

X0, X1 = 44, 536
# ================================================================= layout copied from the GNY genie picture
# pieces: re_skill04 (same set as the Slave Priest panel / DAT-32), re_frame_2 red slot, re_auto_02 on/off
SK = 're_skill04.dxt'
SK_DARK = (507, 190, 695, 222)      # black, gold border
SK_LIT = (506, 112, 695, 144)       # gold lit
SK_BTN = (506, 151, 695, 183)       # dark gold, soft glow
SK_FIELD = (506, 229, 696, 261)     # dark field, gold border
SK_TITLE = (491, 685, 851, 770)     # navy/teal box with gold line
SK_TITLE_LIT = (678, 633, 857, 682)
SK_SLOT = (966, 264, 1017, 316)     # gold slot
RED_SLOT = (69, 72, 120, 123)       # re_frame_2 (128x128)
ON_ART, OFF_ART = (26, 92, 94, 111), (26, 49, 94, 68)

def sk(parent, src, rect, index=0):
    return image(parent, SK, src, rect, index, 1024)

def plate(parent, sid, label, rect, color=0xFFFFFFFF, h=9):
    sk(parent, SK_DARK, rect)
    l, t, r, b = rect
    text(parent, sid, label, [l, t + (b - t - 13) // 2, r, t + (b - t - 13) // 2 + 13], color, center=True, h=h, bold=True)

def plate_c(parent, sid, label, y):
    x = (W - 192) // 2
    image(parent, 're_auto_02.dxt', PLATE, [x, y, x + 192, y + 27])
    text(parent, sid, label, [x + 16, y + 7, x + 192, y + 20], 0xFFFFFFFF, center=True, h=9, bold=True)

def restyle(btn, rect, srcs, tex=SK, size=1024):
    fit(btn, rect)
    for k, img in enumerate(c for c in btn.children if c.type == 'IMAGE'):
        s = srcs[k] if k < len(srcs) else srcs[0]
        img.tex = 'ui\\' + tex
        img.uv = [s[0] / size, s[1] / size, s[2] / size, s[3] / size]
        img.rect = list(rect)
        img.children = [c for c in img.children if c.type == 'STRING']

def pair(parent, on, off, x, y, w=146):
    sk(parent, SK_FIELD, [x, y, x + w, y + 26])
    bw = (w - 10) // 2
    fit(on, [x + 4, y + 4, x + 4 + bw, y + 22])
    fit(off, [x + 6 + bw, y + 4, x + 6 + 2 * bw, y + 22])

def new_pair(parent, pid, x, y, w=146):
    on = BTN_ONOFF[0].clone(); on.id = pid + '_on'; parent.add(on)
    off = BTN_ONOFF[1].clone(); off.id = pid + '_off'; parent.add(off)
    pair(parent, on, off, x, y, w)

def check_as_pair(base, bid, sid, pid, x, y, w=146):
    """native tick button drawn as the "On" half of an On/Off pair (ticked = lit); "Off" half is hg_"""
    park(get(base, sid))
    sk(base, SK_FIELD, [x, y, x + w, y + 26])
    bw = (w - 10) // 2
    b = get(base, bid); r = [x + 4, y + 4, x + 4 + bw, y + 22]
    fit(b, r)
    for img in (c for c in b.children if c.type == 'IMAGE'):
        ticked = abs(img.uv[0] - 0.6152) < 1e-3
        src = ON_ART if ticked else OFF_ART
        img.tex = r'ui\re_auto_02.dxt'
        img.uv = [src[0] / 512, src[1] / 512, src[2] / 512, src[3] / 512]; img.rect = list(r); img.children = []
    text(base, 'hg_lbl_' + bid, 'On', [r[0], r[1] + 3, r[2], r[3] - 3], 0xFFFFFF80, center=True, h=9)
    off = BTN_ONOFF[1].clone(); off.id = pid + '_off'; base.add(off)
    fit(off, [x + 6 + bw, y + 4, x + 6 + 2 * bw, y + 22])

def style_box(btn, rect, label):
    """native attack/recovery style button as the big framed box (selected = lit)"""
    restyle(btn, rect, (SK_TITLE, SK_TITLE, SK_TITLE, SK_TITLE))
    for img in btn.children:
        img.children = []
    l, t, r, b = rect
    lines = label.split('\n')
    y0 = t + (b - t) // 2 - 8 * len(lines)
    for n, line in enumerate(lines):
        yy = y0 + n * 16
        text(btn.parent, 'hg_lbl_%s_%d' % (btn.id, n), line, [l, yy, r, yy + 15], 0xFFFFFFFF, center=True, h=11, bold=True)

def sk_button(parent, id_, label, rect):
    b = BTN_ONOFF[0].clone(); b.id = id_
    restyle(b, rect, (SK_BTN, SK_LIT, SK_DARK, SK_DARK))
    for k, img in enumerate(c for c in b.children if c.type == 'IMAGE'):
        for s in (d for d in img.walk() if d.type == 'STRING'):
            s.text = label; s.rect = [rect[0], rect[1] + 3, rect[2], rect[3] - 3]
            s.color = 0xFFFFFFFF; s.fstyle = 1
    return parent.add(b)

def field(parent, rect):
    sk(parent, SK_FIELD, rect)

# ================================================================= tabs: dark gold boxes, selected = lit
for tid, x in (('btn_attack', 71), ('btn_etc', 306)):
    tb = get(genie, tid)
    restyle(tb, [x, 72, x + 198, 99], (SK_DARK, SK_DARK, SK_LIT, SK_DARK))
    for s in (d for d in tb.walk() if d.type == 'STRING'):
        s.rect = [x, 79, x + 198, 93]; s.fontH = 10; s.fstyle = 1

# ================================================================= MAIN (attack + assist)
for c in [c for c in attack.children if c.type == 'IMAGE']: c.remove()
for c in [c for c in assist.children if c.type == 'IMAGE']: c.remove()
for sid in ('str_add_skill', 'str_att_mode', 'str_rc_mode', 'str_skill_rate', 'str_att_range', 'str_act_range'):
    park(get(attack, sid))
for sid in ('str_party_set', 'str_ass_use_set'): park(get(assist, sid))
for c in [c for c in assist.children if c.type == 'AREA' and c.id == '']: park(c)

# -- "Attack" box + red slots (attack / recovery skill page, native toggle); "Recovery Support" + gold slots
GX, GP, CS = 202, 41, 38                                       # 3 rows x 8 slots: Attack / Recovery / Support
RY_ATT, RY_REC, RY_SUP = 108, 153, 198
style_box(get(attack, 'btn_attack_style'), [52, RY_ATT, 190, RY_ATT + 38], 'Attack')
style_box(get(attack, 'btn_recovery_style'), [52, RY_REC, 190, RY_REC + 38], 'Recovery')
sk(assist, SK_TITLE, [52, RY_SUP, 190, RY_SUP + 38])
text(assist, 'hg_lbl_support', 'Support', [52, RY_SUP + 11, 190, RY_SUP + 26], 0xFFFFFFFF, center=True, h=11, bold=True)
nums = get(attack, 'base_attack_slot_num'); nums.rect = [0, 0, W, H]
for c in nums.children: park(c)
for i in range(8):
    x = GX + i * GP
    image(attack, 're_frame_2.dxt', RED_SLOT, [x, RY_ATT, x + CS, RY_ATT + CS], 0, 128)
    get(attack, str(i + 1)).rect = [x + 3, RY_ATT + 3, x + 35, RY_ATT + 35]
    sk(attack, SK_SLOT, [x, RY_REC, x + CS, RY_REC + CS])
    get(attack, str(i + 9)).rect = [x + 3, RY_REC + 3, x + 35, RY_REC + 35]
for i in range(12):                                            # support skills; tick = also cast on party
    park(get(assist, 'str_%02d' % (i + 1)))
    if i >= 8:                                                 # only 8 shown
        park(get(assist, str(21 + i))); park(get(assist, 'btn_party_%d' % (i + 1)))
        continue
    x = GX + i * GP; y = RY_SUP
    sk(assist, SK_SLOT, [x, y, x + CS, y + CS])
    get(assist, str(21 + i)).rect = [x + 3, y + 3, x + 35, y + 35]
    fit(get(assist, 'btn_party_%d' % (i + 1)), [x + 30, y - 4, x + 42, y + 8])

# -- general options
plate_c(attack, 'hg_sec_general', 'General Options', 248)
CX, CW = (48, 217, 383), 146
plate(attack, 'hg_t_attack_mode', 'Attack Mode', [CX[0], 280, CX[0] + CW, 300])
plate(attack, 'hg_t_self_mode', 'Recovery Mode', [CX[1], 280, CX[1] + CW, 300])     # native auto recovery on/off
plate(assist, 'hg_t_party_mode', 'Support Mode', [CX[2], 280, CX[2] + CW, 300])      # native assist (support) on/off
pair(attack, get(attack, 'btn_auto_attack_on'), get(attack, 'btn_auto_attack_off'), CX[0], 302)
pair(attack, get(attack, 'btn_auto_recovery_on'), get(attack, 'btn_auto_recovery_off'), CX[1], 302)
pair(assist, get(assist, 'btn_assist_on'), get(assist, 'btn_assist_off'), CX[2], 302)
plate(attack, 'hg_t_r_attack', 'R Attack', [CX[0], 332, CX[0] + CW, 352])
plate(attack, 'hg_t_leader', 'Party Leader Target', [CX[1], 332, CX[1] + CW, 352])
plate(attack, 'hg_t_combo', '3 - 5 Combo', [CX[2], 332, CX[2] + CW, 352])
new_pair(attack, 'hg_r_attack', CX[0], 354)
new_pair(attack, 'hg_leader_target', CX[1], 354)
check_as_pair(attack, 'btn_rapid_hit', 'str_rapid_hit', 'hg_combo', CX[2], 354)      # native "Combo"
# third row: native tick options as On/Off pairs ("On" = the native tick button, "Off" = hg_*_off)
plate(attack, 'hg_t_hide_range', 'Hide Range', [CX[0], 384, CX[0] + CW, 404])
plate(attack, 'hg_t_hunt', 'Hunt in Place', [CX[1], 384, CX[1] + CW, 404])
plate(attack, 'hg_t_backstart', 'Back to Start', [CX[2], 384, CX[2] + CW, 404])
check_as_pair(attack, 'btn_hide_attack_range', 'str_hide_attack_range', 'hg_hide_range', CX[0], 406)
check_as_pair(attack, 'scr_act_range', 'Attack_actrange_str', 'hg_hunt', CX[1], 406)
check_as_pair(attack, 'btn_no_enemy_stay_in_place', 'str_no_enemy_stay_in_place', 'hg_backstart', CX[2], 406)
# "Basic Attack Off" is driven by R Attack (On = basic attack on): its own tick is not shown
park(get(attack, 'btn_not_attack')); park(get(attack, 'str_not_attack'))

# -- lower left: percentages
LX, LW = 53, 184
def slider_row(base, label_id, label, sb, val, y):
    text(base, label_id, label, [LX + 2, y, LX + 140, y + 14], 0xFFFFFFFF, h=9)
    val.rect = [LX + LW - 44, y, LX + LW, y + 14]; val.color = 0xFFFFFFFF
    fit_scroll(sb, LX, y + 16, LW)
ph = get(attack, 'scr_autorecovery_rate').clone(); ph.id = 'hg_party_heal'; attack.add(ph)
phv = get(attack, 'Attack_autorecovery_str').clone(); phv.id = 'hg_party_heal_str'; attack.add(phv)
slider_row(attack, 'hg_l_party_heal', 'Party Heal Percentage', ph, phv, 440)
slider_row(attack, 'hg_l_self_heal', 'Self Heal Percentage', get(attack, 'scr_autorecovery_rate'), get(attack, 'Attack_autorecovery_str'), 488)
slider_row(attack, 'hg_l_range', 'Attack range distance', get(attack, 'scr_att_range'), get(attack, 'Attack_range_str'), 536)

# -- lower right: auto party, monster attack list
RX, RR = 250, 527
check(assist, 'hg_auto_party', RX, 442, 'Auto Party', 70)
field(assist, [RX + 98, 435, RR, 457])
ed = get(etc, 'edit_hp_count').clone(); ed.id = 'hg_pt_code'
for d in ed.walk():
    if d.type == 'STRING': d.id = 'hg_pt_code_str'; d.text = ''
for d in [c for c in ed.children if c.type == 'IMAGE']: d.remove()
fit(ed, [RX + 102, 438, RR - 4, 454]); assist.add(ed)
text(assist, 'hg_pt_hint', 'PT KODU YAZINIZ', [RX + 102, 439, RR - 4, 453], 0xFFE08040, center=True, h=9)
plate(attack, 'hg_t_mob_list', 'Monster Attack List', [RX, 462, RR - 25, 483], h=10)
sk_button(attack, 'hg_mob_add', '+', [RR - 22, 462, RR, 483])
field(attack, [RX, 486, RR, 562])
grp = get(assist, 'grp_transform_list')
# monster list = the genie's three target slots: "+" adds the selected monster, a row click selects it
# (lit), Delete From List removes the selected row. Row = On/Off button art without text + name string.
for n in range(3):
    y = 489 + n * 24
    row = BTN_ONOFF[0].clone(); row.id = 'hg_mob_%d' % (n + 1)
    for img in (c for c in row.children if c.type == 'IMAGE'):
        img.children = [c for c in img.children if c.type != 'STRING']
    fit(row, [RX + 5, y, RR - 5, y + 21]); attack.add(row)
    text(attack, 'hg_mob_%d_str' % (n + 1), '', [RX + 12, y + 3, RR - 12, y + 18], 0xFFFFFFFF, h=10)
sk_button(attack, 'hg_save', 'Save Settings', [RX + 16, 568, RX + 134, 589])
sk_button(attack, 'hg_mob_del', 'Delete From List', [RX + 142, 568, RR - 8, 589])

# -- native options that are not in the picture: one thin row + transform
def native_check(base, bid, sid, x, y, label, w=74, h=8, color=0xFFD0D0D0, parent=None):
    b = get(base, bid)
    fit(b, [x, y, x + 14, y + 14])
    b.rect = [x - 2, y - 2, x + 17 + w, y + 16]
    park(get(base, sid))
    text(parent or base, 'hg_lbl_' + bid, label, [x + 17, y, x + 17 + w, y + 14], color, h=h)
# (the old bottom tick row moved into the On/Off pairs above)

# ================================================================= MISC (etc + recovery)
park(get(etc, 'str_etc_title'))
for c in [c for c in etc.children if c.type == 'IMAGE']: c.remove()
park(get(etc, 'scr_etc_height'))
plate_c(etc, 'hg_sec_other', 'Other', 110)

def etc_row(n, x, y, label):
    b = get(etc, 'btn_etc_%d' % n)
    for st_img in [c for c in b.children if c.type == 'IMAGE']:
        tick = [c for c in st_img.children if c.type == 'IMAGE' and 'mailsystem' in c.tex.lower()][0]
        st_img.uv = tick.uv; st_img.tex = tick.tex; st_img.children = []
    fit(b, [x, y, x + 14, y + 14])
    for st_img in b.children: st_img.rect = [x, y, x + 14, y + 14]
    b.rect = [x - 2, y - 2, x + 214, y + 16]             # click anywhere on the row
    park(get(etc, 'str_etc_%02d' % n))
    text(etc, 'hg_etc_%d' % n, label, [x + 20, y, x + 220, y + 14], 0xFFFFFFFF, h=10)

C1, C2, RY, RD = 72, 290, 146, 24
check(etc, 'hg_goto_mob', C1, RY, 'Go To Monster', 190)
check(etc, 'hg_goto_range', C1, RY + RD, 'Go To Skill Range', 190)
etc_row(7, C1, RY + 2 * RD, 'Auto Repair')
etc_row(8, C1, RY + 3 * RD, 'Use Genie Auto')
check(etc, 'hg_back_start', C2, RY, 'Back to starting point', 210)
etc_row(4, C2, RY + RD, 'Town when party is broken')
etc_row(5, C2, RY + 2 * RD, 'Town when party died')
etc_row(3, C2, RY + 3 * RD, 'Town when there is no arrow')
# native options that are not in the picture
etc_row(11, C1, RY + 4 * RD, 'Auto Repair (armor)')
etc_row(9, C1, RY + 5 * RD, 'Auto Feed Familiar')
etc_row(1, C1, RY + 6 * RD, 'Town when HP potion <')
etc_row(6, C2, RY + 4 * RD, 'Town when weapon is broken')
etc_row(10, C2, RY + 5 * RD, 'DK Mace self heal')
etc_row(2, C2, RY + 6 * RD, 'Town when MP potion <')
for eid, x in (('edit_hp_count', C1), ('edit_mp_count', C2)):
    e = get(etc, eid)
    for d in [c for c in e.children if c.type == 'IMAGE']: d.remove()
    field(etc, [x + 172, RY + 6 * RD - 4, x + 214, RY + 6 * RD + 18])
    fit(e, [x + 175, RY + 6 * RD - 1, x + 211, RY + 6 * RD + 15])

# assist controls shown on Misc; genie_hg.cpp toggles them (and sub-base hg_ax) against the Main-only ones
ax = get(attack, 'base_attack_slot_num').clone(); ax.children = []; ax.id = 'hg_ax'; ax.rect = [0, 0, W, H]; ax.mrect = [0, 0, 0, 0]
assist.add(ax)
TY = RY + 7 * RD
native_check(assist, 'btn_removecurse_onoff', 'str_removecurse_onoff', C1, TY, 'Auto Curse Removal', 190, 10, 0xFFFFFFFF, ax)
tr_on = get(assist, 'btn_transform_onoff'); fit(tr_on, [C2, TY, C2 + 14, TY + 14]); tr_on.rect = [C2 - 2, TY - 2, C2 + 86, TY + 16]
text(ax, 'hg_lbl_transform', 'Transform', [C2 + 20, TY, C2 + 90, TY + 14], 0xFFFFFFFF, h=10)
field(ax, [C2 + 90, TY - 4, C2 + 236, TY + 18])
tr_s = get(assist, 'str_transform'); tr_s.rect = [C2 + 95, TY, C2 + 212, TY + 14]
fit(get(assist, 'btn_transform_list'), [C2 + 214, TY - 4, C2 + 236, TY + 18])
gl, gt, gr, gb = grp.rect
fit(grp, [C2 + 90, TY - 4 - (gb - gt), C2 + 236, TY - 4])             # opens upwards

# -- recover: HP (left) | MP (right) as in the picture, Pet on one row below
for c in [c for c in recovery.children if c.type == 'IMAGE']: c.remove()
plate_c(recovery, 'hg_sec_recover', 'Recover', 342)
for sid in ('str_hp', 'str_mp', 'str_pet', 'str_hp_recovery', 'str_mp_recovery', 'str_pet_recovery',
            'str_hp_rate', 'str_mp_rate', 'str_pet_rate'):
    park(get(recovery, sid))
VAL = {'hp': 'RecoveryHp_str', 'mp': 'RecoveryMp_str', 'pet': 'RecoveryPet_str'}
PW = 152                                                        # HP | MP | Pet side by side
for key, label, area, cnt, x in (('hp', 'HP Potion', '17', '8', 50), ('mp', 'MP Potion', '19', '10', 214), ('pet', 'Pet Potion', '18', '9', 378)):
    plate(recovery, 'hg_t_%s' % key, label, [x, 376, x + PW, 396], h=10)
    fx = x + PW // 2 - 20
    sk(recovery, SK_SLOT, [fx, 402, fx + 41, 443])
    get(recovery, area).rect = [fx + 4, 406, fx + 36, 438]
    get(recovery, cnt).rect = [fx + 3, 426, fx + 38, 439]
    pair(recovery, get(recovery, 'btn_%s_recovery_on' % key), get(recovery, 'btn_%s_recovery_off' % key), x + 3, 450, PW - 6)
    text(recovery, 'hg_l_%s_rate' % key, '%s potion %%' % key.upper() if key != 'pet' else 'Pet potion %', [x + 2, 484, x + 110, 498], 0xFFFFFFFF, h=9)
    v = get(recovery, VAL[key]); v.rect = [x + PW - 40, 484, x + PW, 498]; v.color = 0xFFFFFFFF
    fit_scroll(get(recovery, 'scr_%s_recovery_rate' % key), x, 500, PW)
brs = get(recovery, 'base_recovery_slot_num'); brs.rect = [0, 0, W, H]
for c in brs.children: park(c)

text(etc, 'hg_note1', 'Sorunsuz bir genie kullanimi icin Main bolumundeki', [X0, 548, X1, 564], 0xFFFFE000, center=True, h=10, bold=True)
text(etc, 'hg_note2', "'Save Setting' butonundan save yapiniz.", [X0, 566, X1, 582], 0xFFFFE000, center=True, h=10, bold=True)

# assist after attack: the transform dropdown opens over the monster list
genie.children.remove(assist); genie.children.insert(genie.children.index(attack) + 1, assist)

# every button is hit-tested on its own click region: keep it equal to the (moved) rect
nb = 0
for e in root.walk():
    if e.typ is not None and e.type == 'BUTTON':
        e.btn = e.rect; nb += 1
print('click regions updated:', nb)

os.makedirs(OUTDIR, exist_ok=True)
uifed.save(root, OUT)
print('wrote', OUT, os.path.getsize(OUT))

# ---------------------------------------------------------------- mini bar: baked "Knight Genie" -> text
sub = uifed.load(SRC_SUB)
label = [c for c in sub.children if c.type == 'IMAGE' and c.rect == [31, 11, 106, 25]][0]
label.remove()
tmpl = next(e for e in sub.walk() if e.typ is not None and e.id == 'str_NameOne')
st = tmpl.clone(); st.children = []
st.id = 'hg_sub_title'; st.text = 'HopeGuard'; st.rect = [28, 11, 108, 25]
st.color = 0xFFFFC85A; st.fstyle = 1; st.fontH = 9; st.style = 0x04900000
sub.add(st)
uifed.save(sub, OUT_SUB)
print('wrote', OUT_SUB, os.path.getsize(OUT_SUB))

if PREVIEW:
    import uifrender
    from PIL import Image
    tex = uifrender.Tex(uifrender.GAME)
    MISC_IDS = {'hg_ax', 'btn_removecurse_onoff', 'btn_transform_onoff', 'str_transform', 'btn_transform_list', 'grp_transform_list'}
    shots = []
    for tab in ('main', 'misc'):
        r2 = uifed.load(OUT)
        asb = [c for c in r2.children[0].children if c.id == 'assist'][0]
        for c in list(asb.children):
            if (c.id in MISC_IDS) != (tab == 'misc') or c.id == 'grp_transform_list': c.remove()
        hide = ('explain', 'recovery', 'etc') if tab == 'main' else ('explain', 'attack')
        st = {'btn_attack': 3, 'btn_etc': 0} if tab == 'main' else {'btn_attack': 0, 'btn_etc': 3}
        bg = Image.new('RGBA', (W, H), (40, 60, 40, 255))
        bg.alpha_composite(uifrender.render(r2, tex, (W, H), hide, st))
        shots.append(bg)
    out = Image.new('RGBA', (W * 2 + 10, H), (0, 0, 0, 255))
    out.paste(shots[0], (0, 0)); out.paste(shots[1], (W + 10, 0))
    out.save(PREVIEW)
    print('preview', PREVIEW)
