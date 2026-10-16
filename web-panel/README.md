# JstKO Web Panel

Player-facing PHP panel (registration + e-mail activation, login, character rescue, online
players, rankings, online GMs, PUS code redeem, support tickets) plus a GM-only admin area for
editing site content/images and news without touching code. Meant for **separate PHP hosting**,
not the VPS the game server runs on — it talks to the same PostgreSQL database over the network.

No framework, no Composer dependency (works on plain shared hosting): PDO (`pdo_pgsql`) for the
database and a small built-in SMTP client for mail.

## 1. Upload and hosting requirements

Upload the whole `web-panel/public_html/` folder's **contents** (not the `public_html` folder
itself) straight into your hosting account's web root (the folder your host already serves as
`https://yourdomain/`, typically also called `public_html` in cPanel) — `config.php`, `includes/`
and everything else sit flat, side by side with `index.php`. Nothing needs to live outside the
web root; `config.php`, `config.example.php` and `includes/` are blocked from direct HTTP access
by the two `.htaccess` files already in this folder (Apache/`mod_php` hosting — if your host runs
something else, e.g. OpenLiteSpeed or Nginx with PHP-FPM, ask them to equivalently deny
`config.php`, `config.example.php` and `/includes/`).

- PHP 8.1+ with the **pdo_pgsql** extension enabled (ask your host if unsure — cPanel hosts
  usually have a "PHP Extensions" toggle for this).
- HTTPS (a free Let's Encrypt cert from the host is enough) — the activation/reset e-mails link
  back to `site.base_url`, and cookies are marked `secure` automatically when the request is HTTPS.
- Outbound TCP to the VPS's Postgres port (whatever you open it on, see below) and outbound SMTP
  (your mail provider's port, usually 587).

## 2. Configure the app

```bash
cp config.example.php config.php
```

Edit `config.php` (same folder): database host/port/user/
password, SMTP credentials, `site.base_url` (exact URL this panel will be reached at, no trailing
slash), `site.name`.

## 3. VPS side: let this hosting reach Postgres remotely

Run these **on the VPS** (`root@84.247.183.23`), not from the hosting:

```bash
# 1. A dedicated, restricted role for the panel (not the game server's own DB user).
sudo -u postgres psql -d ko_server <<'SQL'
CREATE ROLE webpanel LOGIN PASSWORD 'put-a-strong-password-here';
GRANT SELECT ON tb_user, account_char, userdata, knights, currentuser, admin_heartbeat,
    pus_code, pus_code_batch, kc_ledger TO webpanel;
GRANT INSERT, UPDATE ON tb_user TO webpanel;
GRANT SELECT, INSERT, UPDATE, DELETE ON web_email_verify, web_password_reset, web_rate_limit,
    web_content, web_news, web_ticket, web_ticket_reply TO webpanel;
GRANT INSERT ON pus_code_attempt, kc_ledger, admin_command TO webpanel;
GRANT UPDATE ON pus_code TO webpanel;
GRANT DELETE ON currentuser TO webpanel; -- character rescue, dead-server path only
SQL

# 2. Apply the panel's own migration (new tables above) if it hasn't run yet.
sudo -u postgres psql -d ko_server -f /opt/ko26xx-build-2625/migrations/20261016000001_web_panel.sql

# 3. Let Postgres accept remote connections.
#    postgresql.conf: listen_addresses = '*'  (or the VPS's own IP instead of '*')
sudo -i -u postgres psql -c "ALTER SYSTEM SET listen_addresses = '*';"

# 4. pg_hba.conf: only this role, only over SSL, only from the hosting's IP if it's static
#    (replace <hosting-ip> — if the host doesn't give you a static IP, use 0.0.0.0/0 here but
#    keep the password strong and sslmode=require in config.php, which the DSN already sets).
echo "hostssl ko_server webpanel <hosting-ip>/32 md5" | sudo tee -a /etc/postgresql/*/main/pg_hba.conf

sudo systemctl restart postgresql

# 5. Firewall: open 5432 only to the hosting (adjust if you use a non-default port or ufw profile).
sudo ufw allow from <hosting-ip> to any port 5432 proto tcp
```

If the host truly has no static IP, you can widen the `pg_hba.conf`/`ufw` rules to `0.0.0.0/0`,
but then the Postgres password alone is what's standing between the internet and this database —
make it long and random, and keep `sslmode=require`.

## 4. SMTP

Any SMTP account works: a mailbox from the same hosting, Gmail with an
[App Password](https://myaccount.google.com/apppasswords), or a transactional provider (Mailgun,
SendGrid, Brevo, …) — just fill in `config.php`'s `smtp` block. Port 587 + `encryption: 'tls'`
(STARTTLS) is the usual choice; port 465 is `encryption: 'ssl'`.

## 5. Admin / GM access

No separate admin login — log into the panel with an **existing in-game account whose
`str_authority` is 0 or 2** (Game Master), then open `/admin/`. Nothing to set up beyond that.

## 6. What this does and doesn't touch

- Registering through the panel sets `account_check = 0` until the e-mail link is clicked; the
  game server now refuses login for those accounts (see `crates/ko-game/src/login_handler/login.rs`)
  until they do. The game client's own "type a new ID/password at the login screen" auto-register
  flow is untouched and still works with no e-mail step (gated separately by the existing
  `auto_register` game option, if you want to turn that off).
- PUS code redeem (`/account.php`) reuses the exact same rules as the in-game ESN box
  (`crates/ko-game/src/handler/pus_codes.rs`): same code normalization, same 5-wrong-tries/10-min
  lockout, same atomic claim. If the player is online in-game at the same moment they redeem on
  the web, the game's own in-memory KC/TL balance only catches up on its next refresh/relogin —
  no money is lost either way, just a brief display lag.
- Character rescue mirrors ko-admin's staff `clear_online` tool, scoped to the logged-in
  account's own characters: if the game server's heartbeat is fresh it queues a graceful kick,
  otherwise it clears the stuck row directly.

## 7. Known limitation (inherited, not introduced here)

`tb_user.str_passwd` is compared **in plain text** by the game server
(`AccountRepository::authenticate`) — this panel stores/updates passwords the same way to stay
compatible. Changing that to a real hash would need a matching change on the game server's login
check; flag it if you want that done as a separate piece of work.
