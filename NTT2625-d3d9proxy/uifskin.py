"""Turns the supplied UIFs (uif_in\\*.uif, read by uifview.py) into skins for the proxy panels:
a background sprite with every static image / label, button state sprites, and rect tables.

  load_uif(name)                      -> list of top-level elements
  Skin(root_nodes, size, offset)      -> compose(is_dynamic) -> PIL image with all static parts
  state_image(button, state)          -> the button's state picture (PIL, button size)
  save_pus(img, path)                 -> "PUSI" w h + premultiplied BGRA (what the panels load)
  walk(nodes) / find(nodes, id)       -> element helpers
"""
import os
import struct

from PIL import Image, ImageDraw

import uifrender as U
import uifview as V

HERE = os.path.dirname(os.path.abspath(__file__))
CLIENT = r'D:\ai\2625_Client'
_tex = None


def tex():
    global _tex
    if _tex is None:
        _tex = U.Tex(CLIENT)
    return _tex


def load_uif(name):
    return V.load(os.path.join(HERE, 'uif_in', name))


def walk(nodes):
    for n in nodes:
        yield n
        yield from walk(n.children)


def find(nodes, id_):
    for n in walk(nodes):
        if n.id == id_:
            return n
    return None


def save_pus(img, path):
    img = img.convert('RGBA')
    w, h = img.size
    raw = bytearray()
    for r, g, b, a in img.getdata():
        raw += bytes((b * a // 255, g * a // 255, r * a // 255, a))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb') as f:
        f.write(b'PUSI' + struct.pack('<II', w, h) + bytes(raw))


def _crop(e):
    img = tex().get(e.tex)
    W, H = img.size
    u0, v0, u1, v1 = e.uv
    return img.crop((round(u0 * W), round(v0 * H), round(u1 * W), round(v1 * H)))


def draw_image(canvas, e, off):
    l, t, r, b = e.rect
    if r <= l or b <= t or not e.tex:
        return
    try:
        part = _crop(e)
    except Exception:  # noqa  (texture missing in the pack)
        return
    if part.size != (r - l, b - t):
        part = part.resize((r - l, b - t), Image.BILINEAR)
    x, y = l + off[0], t + off[1]
    # alpha_composite does not clip: paste the visible part only
    cx0, cy0 = max(0, -x), max(0, -y)
    cx1 = min(part.size[0], canvas.size[0] - x)
    cy1 = min(part.size[1], canvas.size[1] - y)
    if cx1 > cx0 and cy1 > cy0:
        canvas.alpha_composite(part.crop((cx0, cy0, cx1, cy1)), (x + cx0, y + cy0))


def draw_string(canvas, e, off):
    if not e.text:
        return
    l, t, r, b = e.rect
    c = e.color
    col = ((c >> 16) & 255, (c >> 8) & 255, c & 255, 255)
    f = U.font(e.fontH, bool(e.fstyle & 1))
    txt = e.text if e.text.isascii() else '?'
    dr = ImageDraw.Draw(canvas)
    bb = dr.textbbox((0, 0), txt, font=f)
    tw, th = bb[2] - bb[0], bb[3] - bb[1]
    x = l + (r - l - tw) // 2 if e.style & 0x00800000 else l
    y = t + (b - t - th) // 2 - bb[1] if e.style & 0x00A00000 else t
    dr.text((x + off[0], y + off[1]), txt, font=f, fill=col)


def pick_state(btn, tag):
    """the image child that belongs to a button state (0 normal, 1 down, 2 on / hover, 3 disabled)"""
    imgs = [c for c in btn.children if c.type == 'IMAGE']
    for c in imgs:
        if c.state == tag:
            return c
    return imgs[min(tag, len(imgs) - 1)] if imgs else None


def compose(nodes, size, off, skip=lambda e: False, button_state=0):
    """Draw every static element. `skip(e)` removes an element and its children (dynamic parts)."""
    canvas = Image.new('RGBA', size, (0, 0, 0, 0))

    def draw(e):
        if skip(e):
            return
        if e.type == 'IMAGE':
            draw_image(canvas, e, off)
        elif e.type == 'STRING':
            draw_string(canvas, e, off)
        elif e.type == 'BUTTON':
            strs = [c for c in e.children if c.type == 'STRING']
            img = pick_state(e, button_state)
            if img is not None:
                draw_image(canvas, img, off)
            for s in strs:
                if not skip(s):
                    draw_string(canvas, s, off)
        else:
            for c in e.children:
                draw(c)

    for n in nodes:
        draw(n)
    return canvas


def state_image(btn, state):
    """The picture of one button state (0 normal, 1 down, 2 hover, 3 disabled), cut to the button rect."""
    e = pick_state(btn, state)
    if e is None:
        return None
    l, t, r, b = e.rect
    out = Image.new('RGBA', (max(1, r - l), max(1, b - t)), (0, 0, 0, 0))
    draw_image(out, e, (-l, -t))
    return out


def rect_of(e, off=(0, 0)):
    l, t, r, b = e.rect
    return (l + off[0], t + off[1], r + off[0], b + off[1])


def header_line(name, rect):
    return 'static const int %s[4] = { %d, %d, %d, %d };\n' % ((name,) + tuple(rect))
