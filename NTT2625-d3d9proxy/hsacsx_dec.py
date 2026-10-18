"""Decrypts HSACSX *.hsacsx UIF files (HSACSX UifCrypto.cpp: RC4(SHA1(pw[:29])[:16]) restarted per 32/31-byte block,
first 4 bytes plain). Usage: python hsacsx_dec.py in.hsacsx out.uif"""
import sys, hashlib
PW = b"(A;dq1DPVFgVs1Aez$VS3R0hge@NvM_TJvblD4.af0h@r4bUzp"[:29]
KEY = hashlib.sha1(PW).digest()[:16]
def rc4(key, n):
    S = list(range(256)); j = 0
    for i in range(256):
        j = (j + S[i] + key[i % len(key)]) & 255; S[i], S[j] = S[j], S[i]
    i = j = 0; o = bytearray()
    for _ in range(n):
        i = (i + 1) & 255; j = (j + S[i]) & 255; S[i], S[j] = S[j], S[i]; o.append(S[(S[i] + S[j]) & 255])
    return bytes(o)
def decrypt(d):
    bl = 32 if len(d) % 2 == 0 else 31
    ks = rc4(KEY, bl)
    out = bytearray(d[:4])
    for p in range(4, len(d), bl):
        blk = d[p:p + bl]
        out += bytes(a ^ b for a, b in zip(blk, ks))
    return bytes(out)
if __name__ == '__main__':
    open(sys.argv[2], 'wb').write(decrypt(open(sys.argv[1], 'rb').read()))
