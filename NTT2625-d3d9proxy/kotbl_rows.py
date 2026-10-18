"""Row level access to the client's encrypted .tbl files (kotbl.py codec).
  load(path)  -> Table(prefix, types, rows)     rows: list of lists, column 0 = Num
  save(path, table)  writes the table back (rows re-sorted by column 0: the client looks rows up by Num)
"""
import struct, os
import kotbl

FMT = {1: 'b', 2: 'B', 3: 'h', 4: 'H', 5: 'i', 6: 'I', 8: 'f', 9: 'd', 10: 'q', 11: 'Q'}


class Table:
    def __init__(self, prefix, types, rows):
        self.prefix, self.types, self.rows = prefix, types, rows

    def find(self, num):
        for r in self.rows:
            if r[0] == num: return r
        return None


def parse(plain):
    prefix, d = plain[:5], plain[5:]
    o = 0
    nc = struct.unpack_from('<i', d, o)[0]; o += 4
    types = list(struct.unpack_from('<%di' % nc, d, o)); o += 4 * nc
    nr = struct.unpack_from('<i', d, o)[0]; o += 4
    rows = []
    for _ in range(nr):
        r = []
        for t in types:
            if t == 7:
                n = struct.unpack_from('<i', d, o)[0]; o += 4
                r.append(d[o:o + n]); o += n
            else:
                f = '<' + FMT[t]; r.append(struct.unpack_from(f, d, o)[0]); o += struct.calcsize(f)
        rows.append(r)
    assert o == len(d), (o, len(d))
    return Table(prefix, types, rows)


def build(t):
    out = bytearray(struct.pack('<i', len(t.types)) + struct.pack('<%di' % len(t.types), *t.types) + struct.pack('<i', len(t.rows)))
    for r in t.rows:
        for ty, v in zip(t.types, r):
            if ty == 7: out += struct.pack('<i', len(v)) + v
            else: out += struct.pack('<' + FMT[ty], v)
    return t.prefix + bytes(out)


def load(path):
    return parse(kotbl.decrypt(open(path, 'rb').read(), 'rev'))


def save(path, t):
    t.rows.sort(key=lambda r: r[0])
    plain = build(t)
    open(path, 'wb').write(kotbl.encrypt(plain, 'rev'))
    assert kotbl.decrypt(open(path, 'rb').read(), 'rev') == plain
