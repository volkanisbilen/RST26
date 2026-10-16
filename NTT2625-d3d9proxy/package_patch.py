# Packages the launcher patch set: d3d9.dll, Data\*.tbl, JstKO\**\*.pus, and the Black Lion
# weapon .n3chr files (client_patch\black_lion_chr) into client_patch_release\ (manifest.json +
# files\...), ready to upload to the VPS for ko-admin's /jstko_patch/* endpoints (patch.rs) to
# serve to Launcher.exe (launcher_patch.cpp).
#
# Run this (and upload the result) every time one of those files changes on purpose -- a tbl
# rename, a new .pus skin, a d3d9.dll rebuild, etc. Players' launchers pick up the change the
# next time they start the game; nothing else needs to run.
#
# Usage: python package_patch.py [client_dir] [out_dir]
#   default client_dir: D:\ai\JstKO Client
#   default out_dir:    NTT2625-d3d9proxy\client_patch_release (next to this script)
import hashlib
import json
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CLIENT_DIR = sys.argv[1] if len(sys.argv) > 1 else r'D:\ai\JstKO Client'
OUT_DIR = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, 'client_patch_release')
BLACK_LION_DIR = os.path.join(os.path.dirname(HERE), 'client_patch', 'black_lion_chr')

VPS_HOST = 'root@84.247.183.23'
VPS_PATCH_DIR = '/opt/ko26xx-build-2625/client_patch_release'


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def collect():
    entries = []  # (rel_path_with_forward_slashes, abs_path)

    dll = os.path.join(CLIENT_DIR, 'd3d9.dll')
    if os.path.isfile(dll):
        entries.append(('d3d9.dll', dll))

    data_dir = os.path.join(CLIENT_DIR, 'Data')
    for name in sorted(os.listdir(data_dir)):
        if name.lower().endswith('.tbl'):
            entries.append(('Data/' + name, os.path.join(data_dir, name)))

    jstko_dir = os.path.join(CLIENT_DIR, 'JstKO')
    for root, _dirs, files in os.walk(jstko_dir):
        for name in sorted(files):
            if name.lower().endswith('.pus'):
                full = os.path.join(root, name)
                rel = 'JstKO/' + os.path.relpath(full, jstko_dir).replace(os.sep, '/')
                entries.append((rel, full))

    if os.path.isdir(BLACK_LION_DIR):
        for name in sorted(os.listdir(BLACK_LION_DIR)):
            if name.lower().endswith('.n3chr'):
                entries.append(('Chr/' + name, os.path.join(BLACK_LION_DIR, name)))

    entries.sort(key=lambda e: e[0].lower())
    return entries


def main():
    entries = collect()
    files_dir = os.path.join(OUT_DIR, 'files')
    if os.path.isdir(files_dir):
        shutil.rmtree(files_dir)
    os.makedirs(files_dir, exist_ok=True)

    manifest = []
    for rel, full in entries:
        digest = sha256_of(full)
        size = os.path.getsize(full)
        manifest.append({'path': rel, 'sha256': digest, 'size': size})
        dest = os.path.join(files_dir, *rel.split('/'))
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.copy2(full, dest)

    with open(os.path.join(OUT_DIR, 'manifest.json'), 'w', encoding='utf-8', newline='\n') as f:
        json.dump(manifest, f, indent=1)

    total = sum(e['size'] for e in manifest)
    print('packaged %d files (%.1f MB) into %s' % (len(manifest), total / 1e6, OUT_DIR))
    print()
    print('upload with:')
    print('  scp -r "%s" %s:%s_new && ssh %s "rm -rf %s && mv %s_new %s"' % (
        OUT_DIR, VPS_HOST, VPS_PATCH_DIR, VPS_HOST, VPS_PATCH_DIR, VPS_PATCH_DIR, VPS_PATCH_DIR))
    print('(ko-admin reads manifest.json fresh on every request, so no restart is needed after the swap)')


if __name__ == '__main__':
    main()
