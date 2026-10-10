r"""Stat / skill reset as native controls of the game's own windows (the proxy part is reset_bar.cpp):
  re_page_state.uif : the empty row under "Contribution" gets  KC [hg_kc]  TL [hg_tl]  [hg_stat_reset "Stat Reset"]
                      (value boxes = the Level row's box art, captions / values = clones of the Level caption and
                      Text_Level, button = clone of btn_preset "Stat Preset")
  re_skill.uif      : hg_skill_reset "Skill Reset" = clone of btn_preset, right above it
Base = genie_out\<name>.orig.uif (the game's originals: python PatchTools\uif_crypt.py extract <client> <name>.uif ...).
Output: genie_out\<name>.uif (plain) + genie_out\enc\<name>.uif (HopeGuard encrypted, what goes into the pack / patch)
        ->  python pack_replace.py <client>\UI ui genie_out\enc\re_page_state.uif genie_out\enc\re_skill.uif
"""
import os, sys
import uifed

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'genie_out')
sys.path.insert(0, os.path.normpath(os.path.join(HERE, '..', '..', 'PatchTools')))
import uif_crypt

def retarget(e, rect):
    """the element and everything in it onto one rect (button states and their captions)"""
    for x in e.walk():
        x.rect = list(rect)
        if x.type == 'BUTTON': x.btn = list(rect)

def save(root, name):
    plain = uifed.dumps(root)
    open(os.path.join(OUT, name), 'wb').write(plain)
    os.makedirs(os.path.join(OUT, 'enc'), exist_ok=True)
    blob = uif_crypt.encrypt_blob(plain)
    assert uif_crypt.is_encrypted(blob) and uif_crypt.decrypt_blob(blob) == plain
    open(os.path.join(OUT, 'enc', name), 'wb').write(blob)
    print(name, len(plain), 'bytes')

# ------------------------------------------------------------------ character page
root = uifed.load(os.path.join(OUT, 're_page_state.orig.uif'))
assert root.find('hg_stat_reset') is None
Y = 220                                             # the row: box art 20 high like the rows above (pitch 22 / 23)
KC, TL, BTN = [51, Y, 117, Y + 20], [143, Y, 209, Y + 20], [214, Y - 1, 292, Y + 21]
# box art: the Level row's value box inside the background piece [1,145,166,267] (re_page_state.dxt 512x512)
bg = next(x for x in root.children if x.type == 'IMAGE' and x.rect == [1, 145, 166, 267])
u0, v0 = bg.uv[0] * 512 - bg.rect[0], bg.uv[1] * 512 - bg.rect[1]
def box_piece(rect, src):                           # src = uif rect of the art
    e = bg.clone(); e.rect = list(rect)
    e.uv = [(u0 + src[0]) / 512, (v0 + src[1]) / 512, (u0 + src[2]) / 512, (v0 + src[3]) / 512]
    return e
ART = [77, 152, 163, 172]
at = next(i for i, c in enumerate(root.children) if c.type == 'BUTTON')      # after the background, under everything else
for r in (KC, TL):                                  # left end + right end of the art, so the edges stay sharp
    half = (r[2] - r[0]) // 2
    root.add(box_piece([r[0], r[1], r[0] + half, r[3]], [ART[0], ART[1], ART[0] + half, ART[3]]), at)
    root.add(box_piece([r[0] + half, r[1], r[2], r[3]], [ART[2] - half, ART[1], ART[2], ART[3]]), at)
caption = next(x for x in root.children if x.type == 'STRING' and x.text == 'Level')
value = root.find('Text_Level')
for id_, text, tmpl, r in (('hg_kc_lbl', 'KC', caption, [29, Y + 2, 51, Y + 19]), ('hg_kc', '0', value, [KC[0], Y + 2, KC[2], Y + 19]),
                           ('hg_tl_lbl', 'TL', caption, [121, Y + 2, 143, Y + 19]), ('hg_tl', '0', value, [TL[0], Y + 2, TL[2], Y + 19])):
    e = tmpl.clone(); e.id = id_; e.text = text; e.rect = r
    root.add(e)
b = root.find('btn_preset').clone(); b.id = 'hg_stat_reset'; retarget(b, BTN)
for x in b.walk():
    if x.type == 'STRING': x.text = 'Stat Reset'
root.add(b)
save(root, 're_page_state.uif')

# ------------------------------------------------------------------ skill window
root = uifed.load(os.path.join(OUT, 're_skill.orig.uif'))
assert root.find('hg_skill_reset') is None
preset = root.find('btn_preset')
b = preset.clone(); b.id = 'hg_skill_reset'
l, t, r_, bt = preset.rect
dy = -(bt - t + 9)                                  # sits on the frame of the Preset box
b.move(0, dy); b.btn = [l, t + dy, r_, bt + dy]
for x in b.walk():
    if x.type == 'STRING': x.text = 'Skill Reset'
root.add(b)
save(root, 're_skill.uif')
