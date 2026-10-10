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

W, H = 534, 620
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

def image(parent, tex, src, rect, index=0):
    # decorations go first in the child list so they are drawn under the controls
    """src = pixel box in a 512x512 atlas"""
    e = IMG_T.clone(); e.children = []
    e.id = ''
    e.tex = 'ui\\' + tex
    e.uv = [src[0] / 512, src[1] / 512, src[2] / 512, src[3] / 512]
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
    fit(e, [x, y, x + 14, y + 14])
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
COLS = [((4, 93), 2), ((62, 93), 91), ((62, 93), 122), ((93, 382), 153), ((336, 382), 442), ((382, 424), 488)]
ROWS_TOP = [((5, 255), 0), ((204, 255), 250), ((255, 486), 301)]
k = 0
for (sy0, sy1), dy in ROWS_TOP:
    for (sx0, sx1), dx in COLS:
        image(genie, 're_auto_01.dxt', (sx0, sy0, sx1, sy1), [dx, dy, dx + sx1 - sx0, dy + sy1 - sy0], k); k += 1
for (sx0, sx1), dx in COLS:
    image(genie, 're_auto_02.dxt', (sx0 + 1, 164, sx1 + 1, 241), [dx, 531, dx + sx1 - sx0, 608], k); k += 1
genie.rect = [0, 0, W, H]
for b in B.values(): b.rect = [0, 0, W, H]

# title + close
park(get(genie, 'str_genie_name'))
t = text(genie, 'hg_title', 'HopeGuard Genie', [197, 27, 366, 47], 0xFFEDEA72, center=True, h=10, bold=True,
         tmpl=get(genie, 'str_genie_name'))
close = get(genie, 'btn_close'); fit(close, [492, 12, 526, 45])

# tabs: Main = btn_attack, Misc = btn_etc
for tid in ('btn_explan', 'btn_recovery', 'btn_assist'): park(get(genie, tid))
for tid, label, x in (('btn_attack', 'Main', 118), ('btn_etc', 'Misc', 268)):
    tb = get(genie, tid)
    fit(tb, [x, 71, x + 148, 100])
    for s in (d for d in tb.walk() if d.type == 'STRING'):
        s.text = label; s.fontH = 10; s.fstyle = 1
        s.rect = [x, 79, x + 148, 93]

# explanation page is not used
for c in list(explain.children): park(c)

# ================================================================= MAIN (attack + assist)
X0, X1 = 44, 492

# -- skills: style buttons + 8 slots (attack or recovery set, native toggle)
section(attack, 'Skills', X0, 106, X1, 'hg_sec_skills')
park(get(attack, 'str_add_skill'))
st = [c for c in attack.children if c.type == 'IMAGE' and c.uv and abs(c.uv[0] - 0.3633) < 1e-3]
for img, y in zip(st, (140, 140)): pass
for img, (x, y) in zip(sorted(st, key=lambda c: c.rect[1]), ((48, 138), (118, 138))):
    img.rect = [x, y, x + 63, y + 40]
fit(get(attack, 'btn_attack_style'), [52, 145, 107, 171])
fit(get(attack, 'btn_recovery_style'), [122, 145, 177, 171])
for s in (d for d in get(attack, 'btn_recovery_style').walk() if d.type == 'STRING'): s.text = 'Recovery'

SX, SD, SS = 196, 37, 35
old_frames = [c for c in attack.children if c.type == 'IMAGE' and c.uv and abs(c.uv[0] - 0.2051) < 1e-3]
for c in old_frames: c.remove()
nums = get(attack, 'base_attack_slot_num')
num_imgs = sorted(nums.children, key=lambda c: (c.rect[1], c.rect[0]))    # row 1 (1..4), row 2 (5..8)
for i in range(8):
    x = SX + i * SD; y = 141
    slot_frame(attack, x, y, SS)
    for aid in (str(i + 1), str(i + 9)):
        a = get(attack, aid); a.rect = [x + 2, y + 2, x + 34, y + 34]
    ni = num_imgs[i]; ni.rect = [x - 1, y - 1, x + 12, y + 13]
nums.rect = [0, 0, W, H]

# -- support skills (assist): 12 slots + party ticks
for c in [c for c in assist.children if c.type == 'IMAGE']: c.remove()        # old plates, lines, frames
section(assist, 'Support Skills', X0, 184, X1, 'hg_sec_support')
park(get(assist, 'str_party_set')); park(get(assist, 'str_ass_use_set'))
text(assist, 'hg_party_hint', 'tick = cast on party too', [330, 190, X1, 206], 0xFF9A9A9A)
for i in range(12):
    x = 47 + i * SD; y = 214
    slot_frame(assist, x, y, SS)
    a = get(assist, str(21 + i)); a.rect = [x + 2, y + 2, x + 34, y + 34]
    fit(get(assist, 'btn_party_%d' % (i + 1)), [x + 10, y + 38, x + 24, y + 52])
    park(get(assist, 'str_%02d' % (i + 1)))
unnamed_area = [c for c in assist.children if c.type == 'AREA' and c.id == '']
for c in unnamed_area: park(c)

# -- general options
section(attack, 'General Options', X0, 272, X1, 'hg_sec_general')
CX = (46, 196, 346)
for sid in ('str_att_mode', 'str_rc_mode'): park(get(attack, sid))
title_plate(attack, 'hg_t_attack_mode', 'Attack Mode', CX[0], 304, 146)
title_plate(attack, 'hg_t_self_mode', 'Self Skill Mode', CX[1], 304, 146)
title_plate(assist, 'hg_t_party_mode', 'Party Skill Mode', CX[2], 304, 146)
for c in [c for c in attack.children if c.type == 'IMAGE' and c.uv and abs(c.uv[0] - 0.0312) < 1e-3]: c.remove()
onoff(attack, get(attack, 'btn_auto_attack_on'), get(attack, 'btn_auto_attack_off'), CX[0], 326)
onoff(attack, get(attack, 'btn_auto_recovery_on'), get(attack, 'btn_auto_recovery_off'), CX[1], 326)
onoff(assist, get(assist, 'btn_assist_on'), get(assist, 'btn_assist_off'), CX[2], 326)

def native_check(base, bid, sid, x, y, label, w=125):
    # the game rewrites these labels from Texts_us.tbl, so the native string is parked and a fixed one shown
    fit(get(base, bid), [x, y, x + 14, y + 14])
    park(get(base, sid))
    text(base, 'hg_lbl_' + bid, label, [x + 19, y - 1, x + 19 + w, y + 15])

check(attack, 'hg_r_attack', CX[0] + 4, 364, 'R Attack', 125)
check(attack, 'hg_leader_target', CX[1] + 4, 364, 'Party Leader Target', 125)
native_check(attack, 'btn_rapid_hit', 'str_rapid_hit', CX[2] + 4, 364, '3 - 5 Combo')      # native "Combo"
native_check(attack, 'btn_not_attack', 'str_not_attack', CX[0] + 4, 384, 'Basic Attack Off')
native_check(attack, 'btn_hide_attack_range', 'str_hide_attack_range', CX[1] + 4, 384, 'Hide Attack Range')
native_check(assist, 'btn_removecurse_onoff', 'str_removecurse_onoff', CX[2] + 4, 384, 'Auto Curse Removal')

# -- lower left: percentages / range
for c in [c for c in attack.children if c.type == 'IMAGE' and c.uv and abs(c.uv[0] - 0.0352) < 1e-3]: c.remove()
for c in [c for c in attack.children if c.type == 'IMAGE' and c.tex and 're_map' in c.tex.lower()]: c.remove()
for c in [c for c in attack.children if c.type == 'IMAGE' and c.uv and abs(c.uv[0] - 0.1953) < 1e-3]: c.remove()
for sid in ('str_skill_rate', 'str_att_range', 'str_act_range'): park(get(attack, sid))
LX, LW = 46, 230

def slider_row(base, label_id, label, sb, val, y, color=0xFFFFFFFF):
    text(base, label_id, label, [LX + 2, y, LX + 180, y + 14], color, h=9)
    v = val; v.rect = [LX + LW - 44, y, LX + LW, y + 14]
    fit_scroll(sb, LX, y + 15, LW)

slider_row(attack, 'hg_l_self_heal', 'Self Heal Percentage', get(attack, 'scr_autorecovery_rate'), get(attack, 'Attack_autorecovery_str'), 410)
ph = get(attack, 'scr_autorecovery_rate').clone(); ph.id = 'hg_party_heal'; attack.add(ph)
phv = get(attack, 'Attack_autorecovery_str').clone(); phv.id = 'hg_party_heal_str'; attack.add(phv)
slider_row(attack, 'hg_l_party_heal', 'Party Heal Percentage', ph, phv, 454)
slider_row(attack, 'hg_l_range', 'Attack range distance', get(attack, 'scr_att_range'), get(attack, 'Attack_range_str'), 498)
native_check(attack, 'scr_act_range', 'Attack_actrange_str', LX + 4, 548, 'Hunting in Place', 95)
native_check(attack, 'btn_no_enemy_stay_in_place', 'str_no_enemy_stay_in_place', LX + 120, 548, 'Back to Start', 105)

# -- lower right: auto party, transform, monster attack list
RX, RW = 292, 200
check(assist, 'hg_auto_party', RX, 412, 'Auto Party', 70)
image(assist, 're_auto_02.dxt', BLACK, [RX + 96, 407, RX + RW, 427])
ed = get(etc, 'edit_hp_count').clone(); ed.id = 'hg_pt_code'
for d in ed.walk():
    if d.type == 'STRING': d.id = 'hg_pt_code_str'; d.text = ''
fit(ed, [RX + 100, 409, RX + RW - 4, 425]); assist.add(ed)
text(assist, 'hg_pt_hint', 'PT code', [RX + 100, 410, RX + RW - 4, 424], 0xFF707070, center=True, h=9)

# transform (native): checkbox + dropdown
tr_on = get(assist, 'btn_transform_onoff'); fit(tr_on, [RX, 438, RX + 14, 452])
tr_img = [c for c in assist.children if c.type == 'IMAGE']
tr_s = get(assist, 'str_transform'); tr_btn = get(assist, 'btn_transform_list'); grp = get(assist, 'grp_transform_list')
image(assist, 're_auto_02.dxt', BLACK, [RX + 18, 434, RX + RW, 456])
tr_s.rect = [RX + 22, 438, RX + RW - 26, 452]
fit(tr_btn, [RX + RW - 24, 433, RX + RW, 457])
gl, gt, gr, gb = grp.rect
fit(grp, [RX + 18, 457, RX + RW, 457 + (gb - gt)])

title_plate(attack, 'hg_t_mob_list', 'Monster Attack List', RX, 464, RW - 24)
push_button(attack, 'hg_mob_add', '+', [RX + RW - 22, 464, RX + RW, 484])
image(attack, 're_auto_02.dxt', BLACK, [RX, 486, RX + RW, 546])
ml = get(grp, 'list_transform').clone(); ml.id = 'hg_mob_list'
fit(ml, [RX + 4, 488, RX + RW - 2, 544]); attack.add(ml)
push_button(attack, 'hg_save', 'Save Settings', [RX, 550, RX + 98, 570])
push_button(attack, 'hg_mob_del', 'Delete From List', [RX + 102, 550, RX + RW, 570])

# ================================================================= MISC (etc + recovery)
park(get(etc, 'str_etc_title'))
for c in [c for c in etc.children if c.type == 'IMAGE']: c.remove()
park(get(etc, 'scr_etc_height'))
section(etc, 'Other', X0, 106, X1, 'hg_sec_other')

def etc_row(n, x, y, label):
    b = get(etc, 'btn_etc_%d' % n)
    # keep only the tick box of every state image (drop the 306 px row art)
    for st_img in [c for c in b.children if c.type == 'IMAGE']:
        tick = [c for c in st_img.children if c.type == 'IMAGE' and 'mailsystem' in c.tex.lower()][0]
        st_img.uv = tick.uv; st_img.tex = tick.tex; st_img.children = []
    fit(b, [x, y, x + 14, y + 14])
    for st_img in b.children: st_img.rect = [x, y, x + 14, y + 14]
    park(get(etc, 'str_etc_%02d' % n))
    text(etc, 'hg_etc_%d' % n, label, [x + 19, y - 1, x + 225, y + 15])

C1, C2, RY, RD = 50, 276, 140, 24
check(etc, 'hg_goto_mob', C1, RY, 'Go To Monster', 200)
check(etc, 'hg_goto_range', C1, RY + RD, 'Go To Skill Range', 200)
etc_row(7, C1, RY + 2 * RD, 'Auto Repair (weapon)')
etc_row(11, C1, RY + 3 * RD, 'Auto Repair (armor)')
etc_row(8, C1, RY + 4 * RD, 'Use Genie Auto')
etc_row(9, C1, RY + 5 * RD, 'Auto Feed Familiar')
check(etc, 'hg_back_start', C2, RY, 'Back to starting point', 200)
etc_row(4, C2, RY + RD, 'Town when party is broken')
etc_row(5, C2, RY + 2 * RD, 'Town when party died')
etc_row(3, C2, RY + 3 * RD, 'Town when there is no arrow')
etc_row(6, C2, RY + 4 * RD, 'Town when weapon is broken')
etc_row(10, C2, RY + 5 * RD, 'DK Mace self heal')
etc_row(1, C1, RY + 6 * RD, 'Town when HP potions <')
etc_row(2, C2, RY + 6 * RD, 'Town when MP potions <')
for eid, x in (('edit_hp_count', C1), ('edit_mp_count', C2)):
    e = get(etc, eid)
    image(etc, 're_auto_02.dxt', BLACK, [x + 170, RY + 6 * RD - 3, x + 210, RY + 6 * RD + 17])
    fit(e, [x + 172, RY + 6 * RD - 1, x + 208, RY + 6 * RD + 15])

# -- recover: HP / MP / Pet potion columns
for c in [c for c in recovery.children if c.type == 'IMAGE']: c.remove()
section(recovery, 'Recover', X0, 318, X1, 'hg_sec_recover')
for sid in ('str_hp', 'str_mp', 'str_pet', 'str_hp_recovery', 'str_mp_recovery', 'str_pet_recovery',
            'str_hp_rate', 'str_mp_rate', 'str_pet_rate'):
    park(get(recovery, sid))
cols = (('hp', 'HP Potion', '17', '8', 0xFFFF6060), ('mp', 'MP Potion', '19', '10', 0xFF60A0FF), ('pet', 'Pet Potion', '18', '9', 0xFF60FF90))
for (key, label, area, cnt, col), x in zip(cols, CX):
    title_plate(recovery, 'hg_t_%s' % key, label, x, 350, 146, col)
    fx = x + 73 - 20
    image(recovery, 're_auto_02.dxt', SLOT, [fx, 376, fx + 41, 416])
    get(recovery, area).rect = [fx + 4, 380, fx + 36, 412]
    get(recovery, cnt).rect = [fx + 3, 399, fx + 38, 412]
    onoff(recovery, get(recovery, 'btn_%s_recovery_on' % key), get(recovery, 'btn_%s_recovery_off' % key), x, 422)
    text(recovery, 'hg_l_%s_rate' % key, '%s percentage' % label.split()[0], [x + 2, 458, x + 110, 472], col, h=9)
    v = get(recovery, {'hp': 'RecoveryHp_str', 'mp': 'RecoveryMp_str', 'pet': 'RecoveryPet_str'}[key])
    v.rect = [x + 104, 458, x + 146, 472]
    fit_scroll(get(recovery, 'scr_%s_recovery_rate' % key), x, 474, 146)
brs = get(recovery, 'base_recovery_slot_num'); brs.rect = [0, 0, W, H]
for c in brs.children: park(c)

text(etc, 'hg_note', 'HopeGuard Genie', [X0, 540, X1, 556], 0xFF6B7333, center=True, h=9)

# assist after attack: the transform dropdown opens over the monster list
genie.children.remove(assist); genie.children.insert(genie.children.index(attack) + 1, assist)

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
    r2 = uifed.load(OUT)
    shots = []
    for hide, st in ((('explain', 'recovery', 'etc'), {'btn_attack': 3, 'btn_etc': 0}),
                     (('explain', 'attack', 'assist'), {'btn_attack': 0, 'btn_etc': 3})):
        bg = Image.new('RGBA', (W, H), (40, 60, 40, 255))
        bg.alpha_composite(uifrender.render(r2, tex, (W, H), hide, st))
        shots.append(bg)
    out = Image.new('RGBA', (W * 2 + 10, H), (0, 0, 0, 255))
    out.paste(shots[0], (0, 0)); out.paste(shots[1], (W + 10, 0))
    out.save(PREVIEW)
    print('preview', PREVIEW)
