"""Replaces existing entries of a KO 2625 resource pack (<dir>\\<base>.hdr + .src).

The new data is appended to the .src and the entry's offset/size are rewritten in the .hdr
(same entry count, so the RC4-encrypted count stays valid). The old bytes stay in the .src.
First run backs up <hdr>.orig and <src>.orig_size (shared with pack_add.py), so
`python pack_add.py --undo <dir> <base>` restores the original pack.
Usage: python pack_replace.py <packDir> <baseName> file1 [file2 ...]   (matched by file name)
"""
import struct, sys, os, shutil
from pack_add import read_hdr

def main():
    d, base, files = sys.argv[1], sys.argv[2], sys.argv[3:]
    hdr, src = os.path.join(d, base + '.hdr'), os.path.join(d, base + '.src')
    h, ver, cnt, names = read_hdr(hdr)
    if not os.path.exists(hdr + '.orig'):
        shutil.copy(hdr, hdr + '.orig')
        open(src + '.orig_size', 'w').write(str(os.path.getsize(src)))
    h = bytearray(h)
    # entry positions
    pos = {}; o = 8
    for _ in range(cnt):
        l = struct.unpack_from('<H', h, o)[0]; nm = bytes(h[o + 2:o + 2 + l]).lower()
        pos[nm] = o + 2 + l; o += 2 + l + 8
    with open(src, 'ab') as fs:
        for f in files:
            nm = os.path.basename(f).lower().encode('cp1252')
            if nm not in pos:
                print('  not in pack:', nm.decode()); continue
            data = open(f, 'rb').read()
            off = fs.tell(); fs.write(data)
            struct.pack_into('<II', h, pos[nm], off, len(data))
            print('  replace', nm.decode(), 'off', off, 'size', len(data))
    open(hdr, 'wb').write(bytes(h))
    _, _, cnt2, names2 = read_hdr(hdr)
    with open(src, 'rb') as fs:
        for f in files:
            nm = os.path.basename(f).lower().encode('cp1252')
            if nm in names2:
                off, sz = names2[nm]; fs.seek(off)
                print('  verify', nm.decode(), 'OK' if fs.read(sz) == open(f, 'rb').read() else 'FAILED')
    print('entries', cnt2)

if __name__ == '__main__':
    main()
