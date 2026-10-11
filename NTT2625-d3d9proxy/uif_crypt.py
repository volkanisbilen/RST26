"""HopeGuard pack-entry cipher for UI\\ui.src *.uif (port of OPSGUARD\\hgcrypt.h).

Encrypted entry = [payload ^ keystream][u32 nonce][u32 tag]; the .hdr size covers all of it.
Usage:
  python uif_crypt.py enc  plain.uif out.bin [nonce]    -> bytes to put into the pack (pack_add.py / pack_replace.py)
  python uif_crypt.py dec  entry.bin out.uif            -> plain UIF
  python uif_crypt.py pack <client>\\UI <name.uif> plain.uif   -> replace/add that entry in ui.hdr/ui.src (backs up first)
"""
import os
import struct
import sys

K_SEED, K_WORD, K_TAG = 0x48A7C3D1, 0x9E3779B9, 0x5BD1E995
TRAILER = 8
M32 = 0xFFFFFFFF


def mix(x):
    x &= M32
    x ^= x >> 16
    x = (x * 0x7FEB352D) & M32
    x ^= x >> 15
    x = (x * 0x846CA68B) & M32
    x ^= x >> 16
    return x


def seed(nonce, size):
    return mix((mix(nonce ^ K_SEED) + size) & M32)


def tag(nonce, size):
    return mix(mix((nonce + K_TAG) & M32) ^ size)


def crypt(data, sd, pos=0):
    out = bytearray(data)
    for i in range(len(out)):
        p = pos + i
        w = mix(sd ^ (((p >> 2) * K_WORD + 1) & M32))
        out[i] ^= (w >> ((p & 3) * 8)) & 0xFF
    return bytes(out)


def encrypt(plain, nonce=None):
    if nonce is None:
        nonce = struct.unpack('<I', os.urandom(4))[0]
    size = len(plain) + TRAILER
    return crypt(plain, seed(nonce, size)) + struct.pack('<II', nonce, tag(nonce, size))


def decrypt(entry):
    size = len(entry)
    nonce, tg = struct.unpack_from('<II', entry, size - TRAILER)
    if tg != tag(nonce, size):
        raise ValueError('not a HopeGuard-encrypted entry (tag mismatch)')
    return crypt(entry[:-TRAILER], seed(nonce, size))


def _pack(client_ui, name, plain_path):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import tempfile
    import pack_add
    import pack_replace
    base = 'ui'
    entry = encrypt(open(plain_path, 'rb').read())
    tmp = os.path.join(tempfile.mkdtemp(), name.lower())
    open(tmp, 'wb').write(entry)
    _, _, _, names = pack_add.read_hdr(os.path.join(client_ui, base + '.hdr'))
    bn = name.lower().encode('cp1252')
    script = pack_replace if bn in names else pack_add
    sys.argv = ['x', client_ui, base, tmp]
    script.main()


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == 'enc':
        nonce = int(sys.argv[4], 0) if len(sys.argv) > 4 else None
        open(sys.argv[3], 'wb').write(encrypt(open(sys.argv[2], 'rb').read(), nonce))
    elif mode == 'dec':
        open(sys.argv[3], 'wb').write(decrypt(open(sys.argv[2], 'rb').read()))
    elif mode == 'pack':
        _pack(sys.argv[2], sys.argv[3], sys.argv[4])
    else:
        print(__doc__)
