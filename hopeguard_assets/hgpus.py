"""HopeGuard proxy-asset tool: decrypt / encrypt and convert the client's HopeGuard\\*.pus skin files.

A .pus file is "PUSI" + u32 width + u32 height + width*height premultiplied BGRA pixels. Most of them are
stored XOR-encrypted in place (same size, no trailer): the key comes from the file size and the lower-case
file name (OPSGUARD\\hgcrypt.h, AssetSeed / Crypt); the decrypted "PUSI" magic tells them from plain ones.

Folders (this directory):
  original\\   copy of the client's HopeGuard folder (untouched .pus as the client has them)   <- `pull`
  png\\        every .pus decoded to a normal PNG, same relative path + ".png"                 <- `decode`
  build\\      PNGs you changed, encoded back to .pus (encrypted when the original was)       <- `encode`

Commands (python hgpus.py <command> [options]):
  pull [clientDir]        copy the client's HopeGuard folder to original\\ (default D:\\ai\\2625_Client)
  decode [subfolder]      original\\**.pus -> png\\**.png   (all, or only one folder such as drop_ui)
  encode [subfolder]      png\\ -> build\\ : only PNGs that differ from what `decode` wrote are encoded
  install [clientDir]     build\\ -> the client's HopeGuard folder (the old file is saved as .bak_hgpus once)
  test                    round trip proof: encrypt(decrypt(x)) == x for every original, PNG decode/encode keeps pixels
"""
import hashlib
import json
import os
import shutil
import struct
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ORIG = os.path.join(HERE, 'original')
PNG = os.path.join(HERE, 'png')
BUILD = os.path.join(HERE, 'build')
MANIFEST = os.path.join(HERE, 'manifest.json')
DEFAULT_CLIENT = r'D:\ai\2625_Client'

M32 = 0xFFFFFFFF
K_ASSET, K_WORD = 0xC2B2AE35, 0x9E3779B9


def mix(x):
    x &= M32
    x ^= x >> 16
    x = (x * 0x7FEB352D) & M32
    x ^= x >> 15
    x = (x * 0x846CA68B) & M32
    x ^= x >> 16
    return x


def name_hash(name):
    h = 0x811C9DC5
    for c in name.encode('latin-1', 'replace'):
        h ^= c
        h = (h * 0x01000193) & M32
    return h


def asset_seed(size, name):
    return mix((mix(size ^ K_ASSET) + name_hash(name)) & M32)


def crypt(data, name):
    """XOR the whole file with the asset keystream (encrypt and decrypt are the same operation)."""
    seed = asset_seed(len(data), name.lower())
    out = bytearray(data)
    words = (len(out) + 3) // 4
    for w in range(words):
        k = mix(seed ^ ((w * K_WORD + 1) & M32))
        for j in range(4):
            i = w * 4 + j
            if i >= len(out):
                break
            out[i] ^= (k >> (8 * j)) & 0xFF
    return bytes(out)


def decode_pus(raw, name):
    """-> (plain bytes, was_encrypted)"""
    if raw[:4] == b'PUSI':
        return raw, False
    plain = crypt(raw, name)
    if plain[:4] == b'PUSI':
        return plain, True
    raise ValueError('not a PUSI file (plain or encrypted): ' + name)


def pus_to_image(plain):
    w, h = struct.unpack_from('<II', plain, 4)
    px = plain[12:12 + w * h * 4]
    if len(px) != w * h * 4:
        raise ValueError('short pixel data')
    out = bytearray(w * h * 4)
    for i in range(0, len(px), 4):
        b, g, r, a = px[i:i + 4]
        if a == 0:
            out[i:i + 4] = b'\0\0\0\0'
        elif a == 255:
            out[i:i + 4] = bytes((r, g, b, 255))
        else:
            out[i:i + 4] = bytes((min(255, (r * 255 + a // 2) // a), min(255, (g * 255 + a // 2) // a),
                                  min(255, (b * 255 + a // 2) // a), a))
    return Image.frombytes('RGBA', (w, h), bytes(out))


def image_to_pus(img):
    img = img.convert('RGBA')
    w, h = img.size
    raw = img.tobytes()
    head = b'PUSI' + struct.pack('<II', w, h)
    body = bytearray(w * h * 4)
    for i in range(0, len(raw), 4):
        r, g, b, a = raw[i:i + 4]
        body[i:i + 4] = bytes(((b * a + 127) // 255, (g * a + 127) // 255, (r * a + 127) // 255, a))
    return head + bytes(body)


def sha(b):
    return hashlib.sha1(b).hexdigest()


def load_manifest():
    return json.load(open(MANIFEST)) if os.path.exists(MANIFEST) else {}


def walk(base, sub=None):
    root = os.path.join(base, sub) if sub else base
    for d, _, files in os.walk(root):
        for f in sorted(files):
            yield os.path.relpath(os.path.join(d, f), base)


def cmd_pull(client):
    src = os.path.join(client, 'HopeGuard')
    n = 0
    for rel in walk(src):
        dst = os.path.join(ORIG, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(os.path.join(src, rel), dst)
        n += 1
    print('copied', n, 'files from', src)


def cmd_decode(sub=None):
    man = load_manifest()
    n = bad = 0
    for rel in walk(ORIG, sub):
        if not rel.lower().endswith('.pus'):
            continue
        raw = open(os.path.join(ORIG, rel), 'rb').read()
        try:
            plain, enc = decode_pus(raw, os.path.basename(rel))
        except ValueError as e:
            print('  skip', rel, e)
            bad += 1
            continue
        dst = os.path.join(PNG, rel + '.png')
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        pus_to_image(plain).save(dst)
        man[rel.replace('\\', '/')] = {'encrypted': enc, 'png_sha': sha(open(dst, 'rb').read()), 'orig_sha': sha(raw)}
        n += 1
    json.dump(man, open(MANIFEST, 'w'), indent=1)
    print('decoded', n, 'files to', PNG, '(skipped %d)' % bad)


def cmd_encode(sub=None):
    man = load_manifest()
    n = same = 0
    for rel in walk(PNG, sub):
        if not rel.lower().endswith('.png'):
            continue
        key = rel[:-4].replace('\\', '/')
        info = man.get(key)
        data = open(os.path.join(PNG, rel), 'rb').read()
        if info and info['png_sha'] == sha(data):
            same += 1
            continue                      # unchanged: the original .pus stays byte-exact
        plain = image_to_pus(Image.open(os.path.join(PNG, rel)))
        enc = info['encrypted'] if info else True
        out = crypt(plain, os.path.basename(key)) if enc else plain
        dst = os.path.join(BUILD, key)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        open(dst, 'wb').write(out)
        print('  encoded', key, '(encrypted)' if enc else '(plain)')
        n += 1
    print('encoded', n, 'changed PNG(s); unchanged:', same, '->', BUILD)


def cmd_install(client):
    dstroot = os.path.join(client, 'HopeGuard')
    n = 0
    for rel in walk(BUILD):
        src = os.path.join(BUILD, rel)
        dst = os.path.join(dstroot, rel)
        if os.path.exists(dst) and not os.path.exists(dst + '.bak_hgpus'):
            shutil.copy2(dst, dst + '.bak_hgpus')
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(src, dst)
        n += 1
        print('  installed', rel)
    print('installed', n, 'file(s) into', dstroot)


def _close(a, b, tol=1):
    """the premultiplied bytes of two .pus bodies differ by at most `tol` levels"""
    if len(a) != len(b):
        return False
    return all(abs(x - y) <= tol for x, y in zip(a[12:], b[12:])) and a[:12] == b[:12]


def cmd_test():
    files = [r for r in walk(ORIG) if r.lower().endswith('.pus')]
    enc_ok = pix_ok = stable = fail = 0
    for rel in files:
        raw = open(os.path.join(ORIG, rel), 'rb').read()
        name = os.path.basename(rel)
        try:
            plain, enc = decode_pus(raw, name)
        except ValueError:
            fail += 1
            print('FAIL decode', rel)
            continue
        if (crypt(plain, name) if enc else plain) == raw:
            enc_ok += 1
        else:
            print('FAIL encrypt round trip', rel)
        img = pus_to_image(plain)
        again = image_to_pus(img)
        if again == plain:
            pix_ok += 1
        elif _close(plain, again):
            stable += 1           # semi-transparent pixels can differ by a level or two after premultiply / unpremultiply
        else:
            fail += 1
            print('FAIL pixels', rel)
    print('files %d | encrypt round trip ok %d | pixel round trip exact %d, png-stable %d | failures %d'
          % (len(files), enc_ok, pix_ok, stable, fail))
    return fail == 0 and enc_ok == len(files)


if __name__ == '__main__':
    args = sys.argv[1:]
    if not args:
        print(__doc__)
    elif args[0] == 'pull':
        cmd_pull(args[1] if len(args) > 1 else DEFAULT_CLIENT)
    elif args[0] == 'decode':
        cmd_decode(args[1] if len(args) > 1 else None)
    elif args[0] == 'encode':
        cmd_encode(args[1] if len(args) > 1 else None)
    elif args[0] == 'install':
        cmd_install(args[1] if len(args) > 1 else DEFAULT_CLIENT)
    elif args[0] == 'test':
        sys.exit(0 if cmd_test() else 1)
    else:
        print(__doc__)
