# HopeGuard asset tool (.pus skins)

The client's `HopeGuard\*_ui\*.pus` files are the skins of the proxy (DLL) windows. A `.pus` is
`"PUSI" + width + height + premultiplied BGRA pixels`; most of them are XOR-encrypted in place
(key from the file size and the lower-case file name, see `OPSGUARD\hgcrypt.h`).
`hgpus.py` decrypts them to normal PNG files and encrypts changed PNGs back.

## Workflow
1. `python hgpus.py pull`      copy the client's HopeGuard folder to `original\`
2. `python hgpus.py decode`    PNGs appear in `png\` (same folders, file name + `.png`)
   (`python hgpus.py decode drop_ui` for one folder only)
3. Edit the PNGs (keep the size; transparency is kept).
4. `python hgpus.py encode`    only PNGs you changed are converted, into `build\` (encrypted when the original was)
5. `python hgpus.py install`   copies `build\` into `D:\ai\2625_Client\HopeGuard` (the old file is saved once as `.bak_hgpus`)
6. `python hgpus.py test`      proof: every original survives decrypt -> PNG -> encode byte-exact

Untouched PNGs are never re-encoded, so the original `.pus` files stay byte-identical.
`original\`, `png\`, `build\` and `manifest.json` are not committed (about 100 MB); the tool is.
