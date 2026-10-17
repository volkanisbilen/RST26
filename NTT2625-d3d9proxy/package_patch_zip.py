# Builds the next incremental patch zip for the launcher patch system (Launcher/ project +
# ko-admin's patch_tcp.rs / patch.rs).
#
# Tracks what was last packaged in patch_state.json (committed to git, so the version counter
# survives across machines). Each run compares the tracked files' current SHA-256 against that
# state; if anything changed or was added, it zips just those files (entries use forward
# slashes, matching the client's own folder layout: "Data/Item_Org_us.tbl", "d3d9.dll", ...)
# as <next_version>.zip, and advances the state. Deletions aren't handled (the launcher's
# zip_extract only ever adds/overwrites files) -- if a file needs to go away, say so and it'll
# be handled by hand.
#
# Run this (from NTT2625-d3d9proxy) after every change meant to reach players, then run the
# scp command it prints to upload client_patch_release/zips/<new>.zip to the VPS. No ko-admin
# restart needed -- patch_tcp.rs re-scans the zips/ folder on every request.
#
# Usage: python package_patch_zip.py [client_dir]   (default: D:\ai\JstKO Client)
import hashlib
import json
import os
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
CLIENT_DIR = sys.argv[1] if len(sys.argv) > 1 else r'D:\ai\JstKO Client'
OUT_DIR = os.path.join(HERE, 'client_patch_release')
STATE_PATH = os.path.join(HERE, 'patch_state.json')
BLACK_LION_DIR = os.path.join(os.path.dirname(HERE), 'client_patch', 'black_lion_chr')

VPS_HOST = 'root@84.247.183.23'
VPS_ZIPS_DIR = '/opt/ko26xx-build-2625/client_patch_release/zips'


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def collect():
    entries = []  # (rel_path_forward_slashes, abs_path)

    dll = os.path.join(CLIENT_DIR, 'd3d9.dll')
    if os.path.isfile(dll):
        entries.append(('d3d9.dll', dll))

    # Drop a freshly built Launcher.exe at CLIENT_DIR\launcher2.exe (NOT Launcher.exe itself --
    # that one is still running when this script runs on a dev box, and players' running
    # Launcher.exe needs the file to land under this alternate name too, since
    # LauncherEngine.cpp's ApplySelfUpdateIfPresent() looks for exactly this name to swap itself
    # out for). It gets delivered like any other patch file; the running launcher renames it
    # into place and asks the player to restart.
    launcher2 = os.path.join(CLIENT_DIR, 'launcher2.exe')
    if os.path.isfile(launcher2):
        entries.append(('launcher2.exe', launcher2))

    # Launch guard (launch_ticket.h): its whitelisted Launcher.exe hashes live in the same
    # header d3d9.dll is built from, so whenever that header changes xldr must be rebuilt and
    # redistributed in lockstep -- otherwise the ticket it writes never validates and every
    # client refuses to start ("no ticket").
    xldr = os.path.join(CLIENT_DIR, 'XIGNCODE', 'xldr_KnightOnline_NA_loader_win32.exe')
    if os.path.isfile(xldr):
        entries.append(('XIGNCODE/xldr_KnightOnline_NA_loader_win32.exe', xldr))

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


def load_state():
    if os.path.isfile(STATE_PATH):
        with open(STATE_PATH, encoding='utf-8') as f:
            return json.load(f)
    return {'version': 0, 'files': {}}


def main():
    state = load_state()
    entries = collect()

    changed = []
    new_hashes = {}
    for rel, full in entries:
        digest = sha256_of(full)
        new_hashes[rel] = digest
        if state['files'].get(rel) != digest:
            changed.append((rel, full))

    if not changed:
        print('nothing changed since version %d, no zip built' % state['version'])
        return

    new_version = state['version'] + 1
    zips_dir = os.path.join(OUT_DIR, 'zips')
    os.makedirs(zips_dir, exist_ok=True)
    zip_path = os.path.join(zips_dir, '%d.zip' % new_version)

    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        for rel, full in changed:
            zf.write(full, rel)

    state['version'] = new_version
    state['files'] = new_hashes
    with open(STATE_PATH, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(state, f, indent=1)

    size = os.path.getsize(zip_path)
    print('built %s.zip: %d changed file(s), %.2f MB -> %s' % (new_version, len(changed), size / 1e6, zip_path))
    print()
    print('upload with:')
    print('  scp "%s" %s:%s/' % (zip_path, VPS_HOST, VPS_ZIPS_DIR))
    print('(ko-admin reads the zips/ folder fresh on every request, no restart needed)')


if __name__ == '__main__':
    main()
