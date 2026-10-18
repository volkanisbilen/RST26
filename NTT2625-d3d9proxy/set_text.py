r"""Sets (or shows) one text of a 2-column client text table (id, string), e.g. Data\Texts_us.tbl.
Usage: python set_text.py <gameDir> <tbl> <id> [new text]     (backup: <tbl>.before_settext, kept from the first run)
Used for the event menu (taskbar 'Opens the event'): type 0 entry label 33252 (0x81E4) -> 'Manner Store'."""
import sys, os, struct, shutil
import kotbl

game, tbl, tid = sys.argv[1], sys.argv[2], int(sys.argv[3])
new = sys.argv[4] if len(sys.argv) > 4 else None
p = os.path.join(game, 'Data', tbl)
raw = open(p, 'rb').read()
d = kotbl.decrypt(raw, 'rev')
assert kotbl.encrypt(d, 'rev') == raw, 'codec does not round-trip this file'
at = 5
nc = struct.unpack_from('<I', d, at)[0]
cols = struct.unpack_from('<%dI' % nc, d, at + 4)
assert nc == 2, 'not a 2-column table: %r' % (cols,)
at += 4 + 4 * nc
nr_at = at; nr = struct.unpack_from('<I', d, at)[0]; at += 4
rows = []
for _ in range(nr):
    i = struct.unpack_from('<i', d, at)[0]; n = struct.unpack_from('<I', d, at + 4)[0]
    rows.append([i, d[at + 8:at + 8 + n]]); at += 8 + n
cur = [r for r in rows if r[0] == tid]
print('current:', cur[0][1].decode('cp1252', 'replace') if cur else '(none)')
if new is None: sys.exit(0)
if cur: cur[0][1] = new.encode('cp1252')
else: rows.append([tid, new.encode('cp1252')]); nr += 1
body = b''.join(struct.pack('<iI', i, len(s)) + s for i, s in rows)
plain = d[:nr_at] + struct.pack('<I', nr) + body + d[at:]
bak = p + '.before_settext'
if not os.path.exists(bak): shutil.copy2(p, bak)
open(p, 'wb').write(kotbl.encrypt(plain, 'rev'))
assert kotbl.decrypt(open(p, 'rb').read(), 'rev') == plain
print('set', tid, '->', new)
