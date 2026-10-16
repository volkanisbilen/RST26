<?php
declare(strict_types=1);

require_once __DIR__ . '/db.php';

/** True if under the limit (and records this attempt); false if the caller should be refused. */
function jstko_rate_limit(string $bucket, string $ip, int $maxAttempts, int $windowSeconds): bool
{
    $pdo = jstko_db();
    $stmt = $pdo->prepare(
        "DELETE FROM web_rate_limit WHERE bucket = :b AND at < now() - make_interval(secs => :w)"
    );
    $stmt->execute(['b' => $bucket, 'w' => $windowSeconds]);

    $stmt = $pdo->prepare(
        "SELECT count(*) FROM web_rate_limit WHERE bucket = :b AND ip = :ip AND at > now() - make_interval(secs => :w)"
    );
    $stmt->execute(['b' => $bucket, 'ip' => $ip, 'w' => $windowSeconds]);
    $count = (int)$stmt->fetchColumn();
    if ($count >= $maxAttempts) {
        return false;
    }

    $stmt = $pdo->prepare('INSERT INTO web_rate_limit (bucket, ip) VALUES (:b, :ip)');
    $stmt->execute(['b' => $bucket, 'ip' => $ip]);
    return true;
}

function jstko_client_ip(): string
{
    return $_SERVER['REMOTE_ADDR'] ?? '0.0.0.0';
}
