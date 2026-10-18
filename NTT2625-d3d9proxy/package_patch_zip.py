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
# Every zip also contains a freshly written Server.ini: a copy of <client_dir>\Server.ini with
# [Version] Files=<new version> (other keys unchanged), so the launcher's stored version is always
# in step with the last patch it extracted.
#
# Run this (from NTT2625-d3d9proxy) after every change meant to reach players, then run the
# scp command it prints to upload client_patch_release/zips/<new>.zip to the VPS. No ko-admin
# restart needed -- patch_tcp.rs re-scans the zips/ folder on every request.
#
# Usage: python package_patch_zip.py [client_dir]   (default: D:\ai\JstKO Client)
#
# CONSOLIDATED mode (--consolidate): the launcher does not reliably apply every intermediate patch
# in order, so instead of one small incremental zip, EVERY zip from --first to the new version is
# (re)written with the SAME full file set (latest d3d9.dll, XIGNCODE xldr, every Data tbl, plus
# any other file that ever appeared in the existing zips in the range and still exists in the
# client) and a Server.ini whose [Version] Files= is the LATEST version in EVERY zip (the d3d9.dll also
# enforces it at game start from patch_version.h, see gen_manifest.py for the release order). The
# launcher extracts Server.ini last, so a player on any old version who downloads ANY of the zips
# ends up fully current and then simply continues through the later (identical) zips.
# Launcher.upd / launcher2.exe (12 MB self-update builds) are NOT repeated in every zip unless you
# pass --with-launcher. patch_state.json is advanced to the new version as usual.
#   python package_patch_zip.py --consolidate --first 2639 [--version 2647] [--zips-dir DIR] [client_dir]
# Back up the old zips on the VPS before uploading the rewritten ones (see the printed commands).
import argparse
import hashlib
import json
import os
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
_ap = argparse.ArgumentParser()
_ap.add_argument('client_dir', nargs='?', default=r'D:\ai\JstKO Client')
_ap.add_argument('--consolidate', action='store_true')
_ap.add_argument('--first', type=int, default=None, help='first version to (re)write in consolidated mode')
_ap.add_argument('--version', type=int, default=None, help='new latest version (default: state version + 1)')
_ap.add_argument('--with-launcher', action='store_true', help='also repeat Launcher.upd/launcher2.exe in every zip')
_ap.add_argument('--zips-dir', default=None)
ARGS = _ap.parse_args()
CLIENT_DIR = ARGS.client_dir
OUT_DIR = os.path.join(HERE, 'client_patch_release')
STATE_PATH = os.path.join(HERE, 'patch_state.json')
BLACK_LION_DIR = os.path.join(os.path.dirname(HERE), 'client_patch', 'black_lion_chr')

CRLF = chr(13) + chr(10)
LF = chr(10)

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

    # Drop a freshly built Launcher.exe at CLIENT_DIR\Launcher.upd (NOT Launcher.exe itself --
    # that one is still running when this script runs on a dev box, and players' running
    # Launcher.exe is still running too when the patch lands). d3d9proxy.cpp's LauncherSwap
    # thread (wired into every client's DllMain) waits for Launcher.exe to exit -- which it
    # always does right after starting the game -- and renames Launcher.upd over it, retrying
    # for up to 60s. No restart prompt needed.
    launcher_upd = os.path.join(CLIENT_DIR, 'Launcher.upd')
    if os.path.isfile(launcher_upd):
        entries.append(('Launcher.upd', launcher_upd))

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


def make_server_ini(new_version):
    """Server.ini for the patch zip: a copy of the client's own Server.ini with [Version] Files
    set to the new patch version (every other key/line stays as it is in the client, CRLF kept).
    The launcher extracts it over the player's Server.ini, so the stored version always matches
    the last applied patch even if the launcher's own WritePrivateProfileString was lost."""
    src = os.path.join(CLIENT_DIR, 'Server.ini')
    if os.path.isfile(src):
        with open(src, 'rb') as f:
            text = f.read().decode('latin-1')
    else:
        text = '[Server]'+CRLF+'Count=1'+CRLF+'IP0=game.jstko.com'+CRLF+'[Version]'+CRLF+'Files=0'+CRLF
    eol = CRLF  # Windows INI; GetPrivateProfile* reads either, keep CRLF like the client file
    lines = text.replace(CRLF, LF).split(LF)
    out, section, done = [], '', False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('[') and stripped.endswith(']'):
            if section == 'version' and not done:
                out.append('Files=%d' % new_version)
                done = True
            section = stripped[1:-1].strip().lower()
        elif section == 'version' and stripped.split('=', 1)[0].strip().lower() == 'files' and '=' in stripped:
            line = 'Files=%d' % new_version
            done = True
        out.append(line)
    if not done:
        while out and out[-1] == '':
            out.pop()
        if section != 'version':
            out.append('[Version]')
        out.append('Files=%d' % new_version)
    while out and out[-1] == '':
        out.pop()
    return (eol.join(out) + eol).encode('latin-1')


def load_state():
    if os.path.isfile(STATE_PATH):
        with open(STATE_PATH, encoding='utf-8') as f:
            return json.load(f)
    return {'version': 0, 'files': {}}


def consolidated_entries(first, last):
    """Fixed full file set: normal tracked set restricted to d3d9.dll/xldr/Data tbl + union of the
    entries of the existing zips first..last that still exist in the client."""
    zips_dir = ARGS.zips_dir or os.path.join(OUT_DIR, 'zips')
    skip = {'server.ini'}
    if not ARGS.with_launcher:
        skip |= {'launcher.upd', 'launcher2.exe'}
    chosen = {}
    for rel, full in collect():
        low = rel.lower()
        if rel == 'd3d9.dll' or low.startswith('data/') or 'xldr_' in low:
            chosen[rel] = full
    for v in range(first, last + 1):
        zp = os.path.join(zips_dir, '%d.zip' % v)
        if not os.path.isfile(zp):
            continue
        with zipfile.ZipFile(zp) as zf:
            for name in zf.namelist():
                if name.lower() in skip or name in chosen:
                    continue
                full = os.path.join(CLIENT_DIR, *name.split('/'))
                if os.path.isfile(full):
                    chosen[name] = full
                else:
                    print('note: %s (in %d.zip) is not in the client dir, left out' % (name, v))
    return sorted(chosen.items(), key=lambda e: e[0].lower())


def server_ini_for(version):
    """Client's own Server.ini (CRLF kept) with [Version] Files= set to `version`."""
    path = os.path.join(CLIENT_DIR, 'Server.ini')
    with open(path, 'rb') as f:
        raw = f.read().decode('latin-1')
    eol = '\r\n' if '\r\n' in raw else '\n'
    lines = raw.replace('\r\n', '\n').split('\n')
    out, in_ver, done = [], False, False
    for ln in lines:
        st = ln.strip()
        if st.startswith('['):
            if in_ver and not done:
                out.append('Files=%d' % version); done = True
            in_ver = st.lower() == '[version]'
        elif in_ver and st.lower().startswith('files'):
            ln = 'Files=%d' % version; done = True
        out.append(ln)
    if in_ver and not done:
        out.insert(len(out) - (1 if out and out[-1] == '' else 0), 'Files=%d' % version); done = True
    if not done:
        out += ['[Version]', 'Files=%d' % version]
    return eol.join(out).encode('latin-1')


def consolidate():
    state = load_state()
    new_version = ARGS.version or state['version'] + 1
    first = ARGS.first or new_version
    entries = consolidated_entries(first, new_version)
    zips_dir = ARGS.zips_dir or os.path.join(OUT_DIR, 'zips')
    os.makedirs(zips_dir, exist_ok=True)
    print('consolidated set: %d files' % len(entries))
    for v in range(first, new_version + 1):
        zip_path = os.path.join(zips_dir, '%d.zip' % v)
        with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
            for rel, full in entries:
                zf.write(full, rel)
            info = zipfile.ZipInfo('Server.ini', date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            zf.writestr(info, server_ini_for(new_version))   # LATEST version in every zip (not the zip's own number)      # LAST entry: the launcher extracts in order
        print('wrote %d.zip  %.2f MB  sha256 %s' % (v, os.path.getsize(zip_path) / 1e6, sha256_of(zip_path)))
    state['version'] = new_version
    state['files'] = {rel: sha256_of(full) for rel, full in collect()}
    with open(STATE_PATH, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(state, f, indent=1)
    print('patch_state.json -> version %d' % new_version)
    print('upload: back up %s on the VPS first, then scp the %d.zip..%d.zip files' % (VPS_ZIPS_DIR, first, new_version))


def main():
    if ARGS.consolidate:
        return consolidate()
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
        # Always ship a freshly written Server.ini carrying the new version (forward-slash name,
        # like every other entry, so the launcher's zip_extract places it next to Launcher.exe).
        zf.writestr('Server.ini', make_server_ini(new_version))

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
