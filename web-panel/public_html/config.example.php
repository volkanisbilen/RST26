<?php
// Copy this file to config.php (same folder) and fill in real values. config.php is blocked
// from direct HTTP access by this folder's .htaccess -- don't remove that block.

return [
    // --- Database: the SAME Postgres the game server/ko-admin use, reached remotely from this
    // hosting (not the VPS). See web-panel/README.md for the VPS-side setup this needs
    // (listen_addresses, pg_hba.conf, firewall) and for creating a restricted DB user.
    'db' => [
        'host' => '84.247.183.23',
        'port' => 443,
        'dbname' => 'ko_server',
        'user' => 'webpanel',
        'password' => 'CHANGE_ME',
        'sslmode' => 'require',
    ],

    // --- Outgoing mail (activation / password reset links). Any SMTP account works --
    // Gmail App Password, your hosting's own mailbox, Mailgun/SendGrid SMTP, etc.
    'smtp' => [
        'host' => 'smtp.example.com',
        'port' => 587,
        'encryption' => 'tls', // 'tls' (STARTTLS, usual for port 587) or 'ssl' (port 465) or '' for none
        'username' => 'no-reply@example.com',
        'password' => 'CHANGE_ME',
        'from_email' => 'no-reply@example.com',
        'from_name' => 'JstKO',
    ],

    // --- Site ---
    'site' => [
        'name' => 'JstKO',
        // Must match this exact URL (no trailing slash) -- used to build activation/reset links.
        'base_url' => 'https://jstko.com',
        // Game server TCP endpoint shown to players / used nowhere else by the panel itself.
        'game_server_ip' => '84.247.183.23',
        'game_server_port' => 15001,
    ],

    // Session cookie name/lifetime.
    'session' => [
        'name' => 'jstko_panel',
        'lifetime_seconds' => 60 * 60 * 24 * 14, // 14 days "remember me"-less session lifetime
    ],
];
