<?php
// Single shared PDO (pgsql) connection to the game's Postgres database, reached remotely from
// this hosting. Requires the php-pgsql / pdo_pgsql extension.
declare(strict_types=1);

require_once __DIR__ . '/config.php';

function jstko_db(): PDO
{
    static $pdo = null;
    if ($pdo !== null) {
        return $pdo;
    }
    $c = jstko_config()['db'];
    $dsn = sprintf(
        'pgsql:host=%s;port=%d;dbname=%s;sslmode=%s',
        $c['host'],
        $c['port'],
        $c['dbname'],
        $c['sslmode'] ?? 'require'
    );
    $pdo = new PDO($dsn, $c['user'], $c['password'], [
        PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
        PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
        PDO::ATTR_EMULATE_PREPARES => false,
    ]);
    return $pdo;
}
