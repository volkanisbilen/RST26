r"""Adds menu texts to the client's Data\Quest_Menu_us.tbl (NPC SelectMsg button texts).
Usage: python add_quest_menu.py <gameDir>   (backup: Quest_Menu_us.tbl.before_menu)"""
import sys, os, struct, shutil
import kotbl

NEW = {48100: 'Special Store [Loyalty & Manner]',     # MannerStore: 29999_Pontus.lua EVENT 1400
       48101: 'Job Change', 48102: 'Other services'}  # Job Change panel: 16085_Hamus.lua EVENT 165 / 900

game = sys.argv[1]
p = os.path.join(game, 'Data', 'Quest_Menu_us.tbl')
raw = open(p, 'rb').read()
d = kotbl.decrypt(raw, 'rev')
assert kotbl.encrypt(d, 'rev') == raw, 'codec does not round-trip this file'
at = 5
nc = struct.unpack_from('<I', d, at)[0]; at += 4 + 4 * nc
nr_at = at; nr = struct.unpack_from('<I', d, at)[0]; at += 4
rows = []
for _ in range(nr):
    i = struct.unpack_from('<i', d, at)[0]; n = struct.unpack_from('<I', d, at + 4)[0]
    rows.append((i, d[at + 8:at + 8 + n])); at += 8 + n
have = {i for i, _ in rows}
add = [(i, s.encode('cp1252')) for i, s in NEW.items() if i not in have]
if not add:
    print('already there'); sys.exit(0)
body = b''.join(struct.pack('<iI', i, len(s)) + s for i, s in rows + add)
plain = d[:nr_at] + struct.pack('<I', nr + len(add)) + body + d[at:]
bak = p + '.before_menu'
if not os.path.exists(bak): shutil.copy2(p, bak)
open(p, 'wb').write(kotbl.encrypt(plain, 'rev'))
chk = kotbl.decrypt(open(p, 'rb').read(), 'rev')
assert chk == plain
print('added', [i for i, _ in add], '->', p)
