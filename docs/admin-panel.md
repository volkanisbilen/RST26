# JstKO management panel

The panel is served by the game process at `/admin/`. It binds to
`127.0.0.1:8090` by default; keep it loopback-only and reach a remote server
through an SSH tunnel or private VPN. Do not expose port 8090 to the Internet.

Configure the login only through the process environment. The username defaults
to `volkan` and can be overridden with `ADMIN_USERNAME`:

- `ADMIN_PASSWORD_HASH=<output from tools/new-admin-password-hash.ps1>`
- Optional: `ADMIN_USERNAME=volkan`
- Optional: `ADMIN_BIND_ADDR=127.0.0.1:8090`

Generate a fresh password hash interactively with PowerShell:

```powershell
$env:ADMIN_USERNAME = 'volkan'
$env:ADMIN_PASSWORD_HASH = (.\tools\new-admin-password-hash.ps1)
```

The password is entered without echo and is not stored in the repository. Use
a new, unique password of at least 12 characters; do not reuse a password that
has been shared in chat or used for another service. Restart the game process
after configuring environment variables. Changes to starting-character sets
and zone spawn points are read from PostgreSQL by the game on startup; the
panel reports this for spawn edits. Character inventory, warehouse, and
character-stat edits are blocked while the character is online.

The first successful migration creates `admin_audit_log`. Mutating operations
are recorded there with the authenticated operator, action, target, and source
address.
