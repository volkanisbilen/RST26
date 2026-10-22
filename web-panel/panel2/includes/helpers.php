<?php
declare(strict_types=1);

function jstko_nation_name(int $nation): string
{
    return match ($nation) {
        1 => 'Karus',
        2 => 'El Morad',
        default => '-',
    };
}

function jstko_class_name(int $class): string
{
    $m = $class % 100;
    if (in_array($m, [1, 5, 6], true)) return 'Savaşçı';
    if (in_array($m, [2, 7, 8], true)) return 'Rogue';
    if (in_array($m, [3, 9, 10], true)) return 'Büyücü';
    if (in_array($m, [4, 11, 12], true)) return 'Rahip';
    if ($m >= 13) return 'Kurian';
    return '-';
}

/**
 * pdo_pgsql returns PostgreSQL BOOLEAN columns (incl. EXISTS(...) subselects) as the strings
 * 't'/'f', not PHP bool -- and 'f' is truthy in PHP (it's a non-empty string), so every such
 * value from a query must go through this before being used as a condition.
 */
function jstko_pg_bool(mixed $v): bool
{
    return $v === true || $v === 't' || $v === 1 || $v === '1';
}

function jstko_is_gm(int $authority): bool
{
    return $authority === 0 || $authority === 2;
}

function jstko_number(int|float $n): string
{
    return number_format((float)$n, 0, ',', '.');
}

function jstko_time_ago(string $timestamp): string
{
    $then = strtotime($timestamp);
    if ($then === false) return $timestamp;
    $diff = max(0, time() - $then);
    if ($diff < 60) return 'az önce';
    if ($diff < 3600) return floor($diff / 60) . ' dk önce';
    if ($diff < 86400) return floor($diff / 3600) . ' sa önce';
    if ($diff < 86400 * 30) return floor($diff / 86400) . ' gün önce';
    return floor($diff / (86400 * 30)) . ' ay önce';
}

function jstko_random_token(int $bytes = 32): string
{
    return bin2hex(random_bytes($bytes));
}

/** Icon filename the client uses for an item_icon_id1, mirrors the admin panel's resolver
 * (itemicon_<lead>_<dddd>_<dd>_<d>.jpg -- last 7 digits split 4/2/1 behind the leading digits). */
function jstko_item_icon_candidates(int $id): array
{
    $build = function (int $v): ?string {
        $s = (string)$v;
        $n = strlen($s);
        if ($n < 8) return null;
        $lead = substr($s, 0, $n - 7);
        $a = substr($s, $n - 7, 4);
        $b = substr($s, $n - 3, 2);
        $c = substr($s, $n - 1, 1);
        return "itemicon_{$lead}_{$a}_{$b}_{$c}.jpg";
    };
    $out = [];
    if ($f = $build($id)) $out[] = $f;
    if ($f = $build(intdiv($id, 10) * 10)) $out[] = $f;
    return $out;
}

function jstko_item_icon_url(?int $iconId): string
{
    if (!$iconId) return '/assets/itemicon/noicon.png';
    foreach (jstko_item_icon_candidates($iconId) as $file) {
        if (is_file(rtrim((string)($_SERVER['DOCUMENT_ROOT'] ?? ''), '/\\') . '/assets/itemicon/' . $file) || is_file(__DIR__ . '/../assets/itemicon/' . $file)) {
            return '/assets/itemicon/' . $file;
        }
    }
    return '/assets/itemicon/noicon.png';
}

/** Internal link inside this panel (CMS menu urls etc.): adds the sub folder prefix, leaves full urls alone. */
function jstko_url(string $u): string
{
    return (strpos($u, '/') === 0 && strpos($u, '//') !== 0 && strpos($u, '/uploads/') !== 0) ? JSTKO_BASE . $u : $u;
}

function jstko_e(?string $s): string
{
    return htmlspecialchars((string)$s, ENT_QUOTES, 'UTF-8');
}

function jstko_redirect(string $path): never
{
    header('Location: ' . (strpos($path, '/') === 0 ? JSTKO_BASE : '') . $path);
    exit;
}

/** Account ID rule shared with the register form and the game client (login.rs::string_is_valid). */
function jstko_valid_account_id(string $id): bool
{
    return (bool)preg_match('/^[A-Za-z0-9:_]{3,21}$/', $id);
}

function jstko_csrf_token(): string
{
    if (empty($_SESSION['csrf'])) {
        $_SESSION['csrf'] = jstko_random_token(16);
    }
    return $_SESSION['csrf'];
}

function jstko_csrf_field(): string
{
    return '<input type="hidden" name="csrf" value="' . jstko_e(jstko_csrf_token()) . '">';
}

function jstko_csrf_check(): bool
{
    $token = $_POST['csrf'] ?? '';
    return is_string($token) && !empty($_SESSION['csrf']) && hash_equals($_SESSION['csrf'], $token);
}
