"""Adds loose files to a KO 2625 resource pack (<dir>\\item.hdr + item.src), append-only.

hdr: [u32 version=200000][u32 count, RC4-encrypted (SHA1(pw)[:16], CryptDeriveKey 128-bit)]
     count * ([u16 nameLen][name][u32 offset][u32 size]); src = raw file data at offset.
Entries whose name already exists are skipped (never overwritten). Backs up the .hdr and
writes <src>.orig_size once, so `python pack_add.py --undo <dir> <base>` can roll back.
Usage: python pack_add.py <packDir> <baseName> file1 [file2 ...]
"""
import struct, sys, os, hashlib, time, shutil

def rc4(key, n):
    S = list(range(256)); j = 0
    for i in range(256):
        j = (j + S[i] + key[i % len(key)]) & 255; S[i], S[j] = S[j], S[i]
    i = j = 0; o = bytearray()
    for _ in range(n):
        i = (i + 1) & 255; j = (j + S[i]) & 255; S[i], S[j] = S[j], S[i]; o.append(S[(S[i] + S[j]) & 255])
    return bytes(o)
KS = rc4(hashlib.sha1(b'owsd9012%$1as!wpow1033b%!@%12').digest()[:16], 4)
xor4 = lambda b: bytes(a ^ k for a, k in zip(b, KS))

def read_hdr(p):
    h = open(p, 'rb').read()
    ver = struct.unpack_from('<I', h, 0)[0]
    cnt = struct.unpack('<I', xor4(h[4:8]))[0]
    o = 8; names = {}
    for _ in range(cnt):
        l = struct.unpack_from('<H', h, o)[0]; nm = h[o + 2:o + 2 + l]
        off, sz = struct.unpack_from('<II', h, o + 2 + l); o += 2 + l + 8
        names[nm.lower()] = (off, sz)
    assert o == len(h), 'hdr not fully consumed'
    return h, ver, cnt, names

def main():
    if sys.argv[1] == '--undo':
        d, base = sys.argv[2], sys.argv[3]
        hdr, src = os.path.join(d, base + '.hdr'), os.path.join(d, base + '.src')
        size = int(open(src + '.orig_size').read())
        shutil.copy(hdr + '.orig', hdr)
        with open(src, 'r+b') as f: f.truncate(size)
        print('restored', hdr, 'and truncated', src, 'to', size); return
    d, base, files = sys.argv[1], sys.argv[2], sys.argv[3:]
    hdr, src = os.path.join(d, base + '.hdr'), os.path.join(d, base + '.src')
    h, ver, cnt, names = read_hdr(hdr)
    if not os.path.exists(hdr + '.orig'):
        shutil.copy(hdr, hdr + '.orig')
        open(src + '.orig_size', 'w').write(str(os.path.getsize(src)))
    add = bytearray(); n_new = 0
    with open(src, 'ab') as fs:
        for f in files:
            nm = os.path.basename(f).lower().encode('cp1252')
            if nm in names:
                print('  skip (exists)', nm.decode()); continue
            data = open(f, 'rb').read()
            off = fs.tell(); fs.write(data)
            add += struct.pack('<H', len(nm)) + nm + struct.pack('<II', off, len(data))
            names[nm] = (off, len(data)); n_new += 1
            print('  add', nm.decode(), 'off', off, 'size', len(data))
    new_hdr = struct.pack('<I', ver) + xor4(struct.pack('<I', cnt + n_new)) + h[8:] + bytes(add)
    open(hdr, 'wb').write(new_hdr)
    _, _, cnt2, names2 = read_hdr(hdr)
    print('entries', cnt, '->', cnt2, 'verify ok' if cnt2 == cnt + n_new else 'VERIFY FAILED')

if __name__ == '__main__':
    main()
