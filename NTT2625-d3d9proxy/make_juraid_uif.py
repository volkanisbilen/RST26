"""Juraid / Border Defence join window: the supplied re_war_juraid_enter.uif is put into the pack as the game's own
re_war_borderdefence_enter.uif (the server opens that window for Juraid too, event_title.cpp retitles it).
The game expects two ids the design does not have: the title string (str_title) and the right-hand button (icon);
they are the design's unnamed title text and its close button, renamed in place (byte surgery, nothing else changes).
Usage: python make_juraid_uif.py
"""
import os
import struct
import subprocess
import sys

import uifskin as S

HERE = os.path.dirname(os.path.abspath(__file__))
nodes = S.load_uif('re_war_juraid_enter.uif')
data = bytearray(open(os.path.join(HERE, 'uif_in', 're_war_juraid_enter.uif'), 'rb').read())

edits = []
for e in S.walk(nodes):
    if e.type == 'STRING' and e.text == 'Juraid Mountain' and e.id == '':
        edits.append((e.id_off, e.id_end, 'str_title'))
    if e.type == 'BUTTON' and e.id == 'close':
        edits.append((e.id_off, e.id_end, 'icon'))
assert len(edits) == 2, edits
for a, b, new in sorted(edits, reverse=True):
    enc = new.encode('cp1252')
    data[a:b] = struct.pack('<i', len(enc)) + enc
bs = chr(92).encode()
old, new_tex = b'ui' + bs + b're_main_ui_02.dxt', b'ui' + bs + b're_tbar_ui_02.dxt'   # the design's own version of that texture
data = bytearray(bytes(data).replace(old, new_tex))
os.makedirs(os.path.join(HERE, 'uif_ready'), exist_ok=True)
out = os.path.join(HERE, 'uif_ready', 're_war_borderdefence_enter.uif')
open(out, 'wb').write(bytes(data))
r = subprocess.run([sys.executable, os.path.join(HERE, 'uif_crypt.py'), 'pack', r'D:\ai\JstKO Client\UI', 're_war_borderdefence_enter.uif', out],
                   capture_output=True, text=True)
print(r.stdout.strip()[-160:], r.stderr.strip()[-160:])
