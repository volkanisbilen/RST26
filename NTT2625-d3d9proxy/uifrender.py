"""Renders a 2625 .uif (via uifed) to PNG with the game's own textures - layout preview only.
Buttons draw their first state; bases listed in `hide` are skipped.
Usage: python uifrender.py file.uif out.png [gameDir] [hideId,hideId,...]
"""
import os, sys, struct, hashlib
from PIL import Image, ImageDraw, ImageFont
import uifed

GAME = r'F:\KnightOnlineEn - Kopya\KnightOnlineEn - Kopya'
PASS = b'owsd9012%$1as!wpow1033b%!@%12'


def rc4_stream(key, n):
    S = list(range(256)); j = 0
    for i in range(256):
        j = (j + S[i] + key[i % len(key)]) & 255; S[i], S[j] = S[j], S[i]
    out = bytearray(n); i = j = 0
    for k in range(n):
        i = (i + 1) & 255; j = (j + S[i]) & 255; S[i], S[j] = S[j], S[i]
        out[k] = S[(S[i] + S[j]) & 255]
    return bytes(out)
KEY = hashlib.sha1(PASS).digest()[:16]


class Tex:
    def __init__(s, game):
        s.pack = {}; s.cache = {}
        hdr = os.path.join(game, 'UI', 'ui.hdr')
        h = open(hdr, 'rb').read(); o = 8
        while o < len(h):
            l = struct.unpack_from('<H', h, o)[0]; nm = h[o + 2:o + 2 + l].decode('cp1252', 'replace').lower()
            off, sz = struct.unpack_from('<II', h, o + 2 + l); o += 2 + l + 8; s.pack[nm] = (off, sz)
        s.src = open(os.path.join(game, 'UI', 'ui.src'), 'rb')

    def raw(s, name):
        base = os.path.basename(name.replace('\\', '/')).lower()
        off, sz = s.pack[base]; s.src.seek(off); return s.src.read(sz)

    def get(s, name):
        if name in s.cache: return s.cache[name]
        b = s.raw(name)
        nl = struct.unpack_from('<i', b, 0)[0]; o = 4 + nl
        assert b[o:o + 3] == b'NTF', name
        ver = b[o + 3]; w, h, fmt, mip = struct.unpack_from('<IIII', b, o + 4); o += 20
        if fmt in (21, 22): pitch, rows = w * 4, h
        elif fmt in (25, 26): pitch, rows = w * 2, h
        else:
            bs = 8 if fmt == 0x31545844 else 16; pitch, rows = (w // 4) * bs, h // 4
        data = bytearray(b[o:o + pitch * rows])
        if ver == 7:
            ks = rc4_stream(KEY, pitch)
            for y in range(rows):
                row = data[y * pitch:(y + 1) * pitch]
                data[y * pitch:(y + 1) * pitch] = bytes(a ^ c for a, c in zip(row, ks))
        if fmt == 21: img = Image.frombytes('RGBA', (w, h), bytes(data), 'raw', 'BGRA')
        elif fmt == 22: img = Image.frombytes('RGBX', (w, h), bytes(data), 'raw', 'BGRX').convert('RGBA')
        elif fmt in (25, 26):
            px = bytearray(w * h * 4)
            for k, v in enumerate(struct.unpack_from('<%dH' % (w * h), data)):
                if fmt == 25:
                    r_, g, b_, a = (v >> 10 & 31) * 255 // 31, (v >> 5 & 31) * 255 // 31, (v & 31) * 255 // 31, 255 if v & 0x8000 else 0
                else:
                    a, r_, g, b_ = (v >> 12 & 15) * 17, (v >> 8 & 15) * 17, (v >> 4 & 15) * 17, (v & 15) * 17
                px[k * 4:k * 4 + 4] = bytes((r_, g, b_, a))
            img = Image.frombytes('RGBA', (w, h), bytes(px))
        else:
            n = {0x31545844: 1, 0x33545844: 2, 0x35545844: 3}[fmt]
            img = Image.frombytes('RGBA', (w, h), bytes(data), 'bcn', n)
        s.cache[name] = img
        return img


_fonts = {}
def font(h, bold):
    k = (h, bold)
    if k not in _fonts:
        path = r'C:\Windows\Fonts\tahomabd.ttf' if bold else r'C:\Windows\Fonts\tahoma.ttf'
        _fonts[k] = ImageFont.truetype(path, max(8, round(h * 4 / 3)))
    return _fonts[k]


def render(root, tex, size, hide=(), state=None, outline=False):
    """state: {buttonId: stateIndex}"""
    state = state or {}
    canvas = Image.new('RGBA', size, (0, 0, 0, 0))
    dr = ImageDraw.Draw(canvas)

    def draw(e):
        if e.typ is not None:
            if e.id in hide: return
            l, t, r, b = e.rect
            if e.type == 'IMAGE' and r > l and b > t and e.tex:
                img = tex.get(e.tex); W, H = img.size
                u0, v0, u1, v1 = e.uv
                part = img.crop((round(u0 * W), round(v0 * H), round(u1 * W), round(v1 * H)))
                if part.size != (r - l, b - t): part = part.resize((r - l, b - t), Image.BILINEAR)
                canvas.alpha_composite(part, (l, t)) if l >= 0 and t >= 0 else None
            elif e.type == 'STRING' and e.text:
                c = e.color; col = ((c >> 16) & 255, (c >> 8) & 255, c & 255, 255)
                f = font(e.fontH, bool(e.fstyle & 1))
                txt = e.text if e.text.isascii() else '?'
                style = e.style
                bb = dr.textbbox((0, 0), txt, font=f)
                tw, th = bb[2] - bb[0], bb[3] - bb[1]
                x = l + (r - l - tw) // 2 if style & 0x00800000 else l
                y = t + (b - t - th) // 2 - bb[1] if style & 0x00200000 or style & 0x00800000 else t
                dr.text((x, y), txt, font=f, fill=col)
            elif outline and e.type in ('AREA', 'BUTTON', 'EDIT', 'SCROLLBAR'):
                dr.rectangle([l, t, r - 1, b - 1], outline=(255, 0, 255, 160))
        if e.typ is not None and e.type == 'BUTTON':
            si = state.get(e.id, 0)
            imgs = [c for c in e.children if c.type == 'IMAGE']
            strs = [c for c in e.children if c.type == 'STRING']
            if imgs: draw(imgs[min(si, len(imgs) - 1)])
            if strs: draw(strs[min(si, len(strs) - 1)])
            for c in e.children:
                if c.type not in ('IMAGE', 'STRING'): draw(c)
            return
        for c in e.children: draw(c)

    draw(root)
    return canvas


if __name__ == '__main__':
    root = uifed.load(sys.argv[1])
    game = sys.argv[3] if len(sys.argv) > 3 else GAME
    hide = sys.argv[4].split(',') if len(sys.argv) > 4 else ()
    g = root.children[0]
    w, h = g.rect[2], g.rect[3]
    bg = Image.new('RGBA', (w + 20, h + 20), (40, 60, 40, 255))
    bg.alpha_composite(render(root, Tex(game), (w + 20, h + 20), hide, outline=True))
    bg.save(sys.argv[2])
