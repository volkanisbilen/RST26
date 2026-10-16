<?php
declare(strict_types=1);

require_once __DIR__ . '/db.php';

/** All web_content rows as key => value, cached for the request. */
function jstko_content(): array
{
    static $map = null;
    if ($map !== null) {
        return $map;
    }
    $map = [];
    try {
        foreach (jstko_db()->query('SELECT key, value FROM web_content') as $row) {
            $map[$row['key']] = $row['value'];
        }
    } catch (Throwable $e) {
        error_log('jstko_content: ' . $e->getMessage());
    }
    return $map;
}

function jstko_c(string $key, string $default = ''): string
{
    return jstko_content()[$key] ?? $default;
}

/** Image content value is a relative path under /uploads/; returns '' if unset. */
function jstko_c_image(string $key): string
{
    $v = jstko_c($key);
    return $v !== '' ? '/uploads/' . ltrim($v, '/') : '';
}

function jstko_set_content(string $key, string $value, string $kind, string $by): void
{
    $stmt = jstko_db()->prepare(
        'INSERT INTO web_content (key, value, kind, updated_by) VALUES (:k, :v, :kind, :by)
         ON CONFLICT (key) DO UPDATE SET value = :v, kind = :kind, updated_at = now(), updated_by = :by'
    );
    $stmt->execute(['k' => $key, 'v' => $value, 'kind' => $kind, 'by' => $by]);
}
