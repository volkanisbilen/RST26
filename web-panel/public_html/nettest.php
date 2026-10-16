<?php
// Temporary diagnostic -- delete this file once the DB connection issue is resolved.
// Tests raw TCP reachability to the Postgres host, independent of PDO/pdo_pgsql, so we can tell
// a hosting-side outbound block apart from anything else.
header('Content-Type: text/plain');
$host = '84.247.183.23';
$port = 5432;
echo "Testing TCP connect to $host:$port ...\n";
$start = microtime(true);
$errno = 0;
$errstr = '';
$sock = @fsockopen($host, $port, $errno, $errstr, 8);
$elapsed = round((microtime(true) - $start) * 1000);
if ($sock) {
    echo "SUCCESS: connected in {$elapsed}ms\n";
    fclose($sock);
} else {
    echo "FAILED after {$elapsed}ms: errno=$errno errstr=$errstr\n";
}

echo "\npdo_pgsql loaded: " . (extension_loaded('pdo_pgsql') ? 'yes' : 'NO') . "\n";
echo "PHP version: " . PHP_VERSION . "\n";
