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
    // panel2 has no secrets of its own: it uses the config.php next to it, else the main panel's (one folder up)
    $real = null;
    foreach ([__DIR__ . '/../config.php', __DIR__ . '/../../config.php'] as $cand) {
        if (is_file($cand)) { $real = $cand; break; }
    }
    $example = __DIR__ . '/../config.example.php';
    $config = $real !== null ? require $real : require $example;
    return $config;
}
