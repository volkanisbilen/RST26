# Decrypts d3d9proxy.log (obfuscated by log_crypt.cpp: repeating-key XOR + base64, one line per
# entry). KEY must match log_crypt.cpp's KEY exactly.
#
# Usage: python log_decrypt.py [path-to-d3d9proxy.log]   (default: .\d3d9proxy.log)
#        python log_decrypt.py d3d9proxy.log > decoded.txt
import base64
import sys

KEY = b"JstKO-2625-log-obfuscation-v1-9f1c7a3e-do-not-leak"


def decrypt_line(line):
    line = line.strip()
    if not line:
        return ''
    try:
        raw = base64.b64decode(line)
    except Exception:
        return '(not base64: %s)' % line
    return bytes(b ^ KEY[i % len(KEY)] for i, b in enumerate(raw)).decode('utf-8', errors='replace')


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else 'd3d9proxy.log'
    with open(path, encoding='utf-8', errors='replace') as f:
        for line in f:
            print(decrypt_line(line))


if __name__ == '__main__':
    main()
