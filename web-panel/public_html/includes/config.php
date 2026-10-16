<?php
// Loads web-panel/config.php (real secrets, not committed) if present, else falls back to the
// example file so the app at least runs (against placeholder values) for local preview.
declare(strict_types=1);

function jstko_config(): array
{
    static $config = null;
    if ($config !== null) {
        return $config;
    }
    $real = __DIR__ . '/../config.php';
    $example = __DIR__ . '/../config.example.php';
    $config = is_file($real) ? require $real : require $example;
    return $config;
}
