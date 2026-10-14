"""Puts the supplied "E" event icon (4 states) into the taskbar texture re_tbar_ui_02.dxt (the btn_question cells) and renames
the button to btn_eventnew in uif_ready\\re_taskbar_main.uif, then writes both into the client's UI pack.

Usage: python patch_event_icon.py <icon png (39x192, 4 cells separated by magenta lines)>
"""
import os
import struct
import subprocess
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SRC_TEX = r'D:\Jstko_client_editor_icin_gerekli_olan_dosyalar\ui\re_main_ui_02.dxt'
CLIENT_UI = r'D:\ai\2625_Client\UI'
OUT_TEX = os.path.join(HERE, 'uif_ready', 're_tbar_ui_02.dxt')

icon = Image.open(sys.argv[1]).convert('RGBA')
cells = []
for start in (0, 48, 96, 144):
    cells.append(icon.crop((0, start, 39, start + 46)).crop((2, 6, 36, 40)))      # 34x34 inner picture
normal, hover, down, disabled = cells
# btn_question image order in the UIF: state 0 (y315), 1 (y245), 2 (y280), 3 (y210)
targets = [(106, 315, normal), (106, 245, down), (106, 280, hover), (106, 210, disabled)]

data = bytearray(open(SRC_TEX, 'rb').read())
nl = struct.unpack_from('<i', data, 0)[0]
o = 4 + nl
assert data[o:o + 3] == b'NTF'
w, h, fmt, mip = struct.unpack_from('<IIII', data, o + 4)
assert (w, h, fmt) == (512, 512, 21), (w, h, fmt)
base = o + 20
for x0, y0, cell in targets:
    for y in range(34):
        for x in range(34):
            r, g, b, a = cell.getpixel((x, y))
            off = base + ((y0 + y) * w + (x0 + x)) * 4
            data[off:off + 4] = bytes((b, g, r, a))
os.makedirs(os.path.dirname(OUT_TEX), exist_ok=True)
open(OUT_TEX, 'wb').write(bytes(data))

uif = open(os.path.join(HERE, 'uif_ready', 're_taskbar_main.uif'), 'rb').read()
old, new = b'btn_question', b'btn_eventnew'
assert len(old) == len(new)
n = uif.count(old)
if n:
    uif = uif.replace(old, new)
    open(os.path.join(HERE, 'uif_ready', 're_taskbar_main.uif'), 'wb').write(uif)
print('btn_question renamed:', n)

for name, path in (('re_tbar_ui_02.dxt', OUT_TEX), ('re_taskbar_main.uif', os.path.join(HERE, 'uif_ready', 're_taskbar_main.uif'))):
    r = subprocess.run([sys.executable, os.path.join(HERE, 'uif_crypt.py'), 'pack', CLIENT_UI, name, path], capture_output=True, text=True)
    print(name, r.stdout.strip()[-100:], r.stderr.strip()[-100:])
