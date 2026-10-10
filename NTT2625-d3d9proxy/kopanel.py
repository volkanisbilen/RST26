"""Shared helpers for proxy panel skins made from the game's own UI atlases (UI\\ui.hdr / ui.src).

  tex(name)                 -> PIL image of a pack texture (NTF v3 / v7, see build_cr_assets.py)
  piece(tex, box)           -> crop of an atlas cell, trimmed to its visible pixels (the atlases separate
                               cells with magenta guide lines; those and fully transparent pixels are cut)
  slice3(img, w, h, edge)   -> horizontal 3-slice stretch (end caps kept, middle stretched)
  slice9(img, w, h, edge)   -> 9-slice stretch
  save_pus(img, path)       -> "PUSI" w h + premultiplied BGRA (the format the proxy panels load)
  item_icons(ids, out_dir)  -> <out_dir>\\icons\\<iconId>.pus + <out_dir>\\items.txt  (itemId|iconId|name)
"""
import os, struct, subprocess, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = r'F:\KnightOnlineEn - Kopya\KnightOnlineEn - Kopya'   # partner's client = main client since 01.10.2026
DB = 'KO_DATABASE_SERVER_00125'
SQL = r'DESKTOP-UVF79OG\KO_PVP'
FALLBACK_DIRS = []
_cr = open(os.path.join(HERE, 'build_cr_assets.py'), encoding='utf-8').read()
exec(compile(_cr[_cr.index('# ------------------------------------------------------------------ textures'):
                 _cr.index('# ------------------------------------------------------------------ build')], 'cr', 'exec'))


def tex(name):
    return load_tex(name if '\\' in name else 'ui\\' + name).convert('RGBA')


def _is_guide(p):
    r, g, b, a = p
    return a > 0 and r > 200 and b > 200 and g < 60          # magenta guide line


def piece(img, box, trim=True):
    """atlas cell: magenta guide rows / columns are removed (the cell closes up), then trimmed"""
    part = img.crop(box).copy()
    if not trim:
        return part
    w, h = part.size
    px = part.load()
    keep_cols = [x for x in range(w) if sum(_is_guide(px[x, y]) for y in range(h)) < h * 0.5]
    keep_rows = [y for y in range(h) if sum(_is_guide(px[x, y]) for x in range(w)) < w * 0.5]
    out = Image.new('RGBA', (len(keep_cols), len(keep_rows)), (0, 0, 0, 0))
    op = out.load()
    for j, y in enumerate(keep_rows):
        for i, x in enumerate(keep_cols):
            p = px[x, y]
            op[i, j] = (0, 0, 0, 0) if _is_guide(p) else p
    bb = out.getbbox()
    return out.crop(bb) if bb else out


def slice3(img, w, h, edge, mid=None):
    """stretch horizontally keeping `edge` px caps; the height is scaled as a whole.
    mid = (x0, x1) source stripe used for the middle (default: everything between the caps)"""
    sw, sh = img.size
    k = h / sh
    if h != sh:
        img = img.resize((max(1, round(sw * k)), h), Image.LANCZOS)
        edge = max(1, round(edge * k)); sw = img.size[0]
        if mid: mid = (round(mid[0] * k), max(round(mid[0] * k) + 1, round(mid[1] * k)))
    out = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    out.alpha_composite(img.crop((0, 0, edge, h)), (0, 0))
    out.alpha_composite(img.crop((sw - edge, 0, sw, h)), (w - edge, 0))
    m0, m1 = mid if mid else (edge, sw - edge)
    out.alpha_composite(img.crop((m0, 0, m1, h)).resize((max(1, w - 2 * edge), h), Image.LANCZOS), (edge, 0))
    return out


def slice9(img, w, h, edge):
    sw, sh = img.size
    e = edge
    out = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    def put(sx0, sy0, sx1, sy1, dx0, dy0, dx1, dy1):
        if sx1 <= sx0 or sy1 <= sy0 or dx1 <= dx0 or dy1 <= dy0: return
        out.alpha_composite(img.crop((sx0, sy0, sx1, sy1)).resize((dx1 - dx0, dy1 - dy0), Image.LANCZOS), (dx0, dy0))
    xs = [(0, e, 0, e), (e, sw - e, e, w - e), (sw - e, sw, w - e, w)]
    ys = [(0, e, 0, e), (e, sh - e, e, h - e), (sh - e, sh, h - e, h)]
    for sx0, sx1, dx0, dx1 in xs:
        for sy0, sy1, dy0, dy1 in ys:
            put(sx0, sy0, sx1, sy1, dx0, dy0, dx1, dy1)
    return out


def fit(img, w, h):
    return img.resize((w, h), Image.LANCZOS)


def save_pus(img, path):
    img = img.convert('RGBA'); w, h = img.size
    px = bytearray()
    for r_, g, b, a in img.getdata():
        px += bytes((b * a // 255, g * a // 255, r_ * a // 255, a))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb') as f:
        f.write(b'PUSI' + struct.pack('<II', w, h) + px)
    img.save(os.path.splitext(path)[0] + '.png')


def sql(q):
    return subprocess.run(['sqlcmd', '-S', SQL, '-E', '-d', DB, '-h', '-1', '-W', '-s', '|', '-Q', 'SET NOCOUNT ON; ' + q],
                          capture_output=True, text=True, encoding='cp1254', errors='replace').stdout


def _icon_file(i):
    s = str(i)
    return 'itemicon_%s_%s_%s_%s.dxt' % (s[:-7], s[-7:-3], s[-3:-1], s[-1])


def item_icons(item_ids, out_dir):
    """icons of the given items (game's itemicon_*.dxt, 45x45) + items.txt (itemId|iconId|name), merged"""
    ids = sorted({int(i) for i in item_ids if int(i) > 0})
    if not ids:
        return {}
    rows = sql("SELECT Num, CASE WHEN ItemIconID2 <> 0 THEN ItemIconID2 ELSE ItemIconID1 END, strName FROM ITEM WHERE Num IN (%s)"
               % ','.join(map(str, ids)))
    table = {}
    lst = os.path.join(out_dir, 'items.txt')
    if os.path.exists(lst):
        for line in open(lst, encoding='cp1254', errors='replace'):
            p = line.rstrip('\n').split('|')
            if len(p) >= 3: table[int(p[0])] = (int(p[1]), p[2])
    for line in rows.splitlines():
        p = [x.strip() for x in line.split('|')]
        if len(p) >= 3 and p[0].isdigit():
            table[int(p[0])] = (int(p[1] or 0), p[2])
    os.makedirs(os.path.join(out_dir, 'icons'), exist_ok=True)
    made = 0
    for item, (icon, name) in table.items():
        dst = os.path.join(out_dir, 'icons', '%d.pus' % icon)
        if icon <= 0 or os.path.exists(dst): continue
        try:
            try: img = load_tex(_icon_file(icon)).convert('RGBA')
            except Exception: img = load_tex(_icon_file(icon // 10 * 10)).convert('RGBA')
        except Exception:
            print('  icon missing', item, icon); continue
        img = img.crop((0, 0, 45, 45)) if img.size[0] >= 45 else img.resize((45, 45))
        save_pus(img, dst); made += 1
    with open(lst, 'w', encoding='cp1254', errors='replace') as f:
        for item in sorted(table):
            f.write('%d|%d|%s\n' % (item, table[item][0], table[item][1]))
    print('items', len(table), 'icons made', made)
    return table


def green_frame(w, h, interior=(14, 19, 15, 238)):
    """The game's green message box frame with bronze corners (ui\re_message_box.dxt, the pieces of
    re_msgboxokcancel.uif: top 356x96, middle strip, bottom 356x60), stretched to w x h; dark interior."""
    T = tex('re_message_box.dxt')
    top, mid, bot = T.crop((5, 5, 361, 101)), T.crop((6, 104, 359, 117)), T.crop((5, 130, 361, 190))
    out = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    out.alpha_composite(Image.new('RGBA', (w - 16, h - 16), interior), (8, 8))
    th, bh = min(96, h // 2), min(60, h // 2)
    out.alpha_composite(slice3(top.resize((356, th)), w, th, 64), (0, 0))
    if h - th - bh > 0:
        out.alpha_composite(slice3(mid, w, 13, 64).resize((w, h - th - bh)), (0, th))
    out.alpha_composite(slice3(bot.resize((356, bh)), w, bh, 64), (0, h - bh))
    return out
