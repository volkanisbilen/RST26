"""HopeGuard ACS table hash of a client folder (same as launch_guard.cpp HashTables): FNV-1a over Data\*.tbl,
sorted lower-case names + contents. Put the printed value into GameServer.ini [HOPEGUARD] TBL_HASH (comma separated)."""
import os, sys
def tbl_hash(game):
    d = os.path.join(game, 'Data'); h = 0x811C9DC5
    names = sorted(n.lower() for n in os.listdir(d) if n.lower().endswith('.tbl') and os.path.isfile(os.path.join(d, n)))
    for n in names:
        for c in n.encode('cp1252'): h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF
        for c in open(os.path.join(d, n), 'rb').read(): h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF
    return len(names), h
for g in sys.argv[1:]:
    n, h = tbl_hash(g); print('%08X  %d tables  %s' % (h, n, g))
