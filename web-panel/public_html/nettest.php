<?php
// Temporary diagnostic -- delete this file once the DB connection issue is resolved.
// Tests raw TCP reachability to several hosts/ports to tell a blanket outbound block apart from
// one that's specific to database-looking ports (5432, 3306, ...).
header('Content-Type: text/plain');

function test(string $label, string $host, int $port, int $timeout = 6): void
{
    $start = microtime(true);
    $errno = 0;
    $errstr = '';
    $sock = @fsockopen($host, $port, $errno, $errstr, $timeout);
    $elapsed = round((microtime(true) - $start) * 1000);
    if ($sock) {
        echo "OK      {$label} ({$host}:{$port}) -- connected in {$elapsed}ms\n";
        fclose($sock);
    } else {
        echo "FAILED  {$label} ({$host}:{$port}) -- after {$elapsed}ms: errno=$errno errstr=$errstr\n";
    }
}

$vps = '84.247.183.23';
test('VPS Postgres',        $vps, 5432);
test('VPS SSH',             $vps, 22);
test('VPS ko-admin HTTP',   $vps, 8090);
test('VPS launcher patch',  $vps, 15100);
test('Public HTTPS (1.1.1.1 / Cloudflare)', '1.1.1.1', 443);
test('Public HTTP (1.1.1.1 / Cloudflare)',  '1.1.1.1', 80);
test('Public HTTPS (Google)', 'google.com', 443);
test('Public SMTP submission (gmail)', 'smtp.gmail.com', 587);

echo "\npdo_pgsql loaded: " . (extension_loaded('pdo_pgsql') ? 'yes' : 'NO') . "\n";
echo "PHP version: " . PHP_VERSION . "\n";
