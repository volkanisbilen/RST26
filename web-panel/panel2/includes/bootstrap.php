<?php
declare(strict_types=1);
error_reporting(E_ALL);
ini_set('display_errors', '1'); // TEMP while we diagnose the first deploy -- flip back to '0' once it works

// Folder this panel is served from ("" at the web root, "/panel2" in a sub folder): every internal link is
// prefixed with it, uploads and item icons keep pointing at the main site's shared files.
if (!defined('JSTKO_BASE')) {
    $docRoot = realpath((string)($_SERVER['DOCUMENT_ROOT'] ?? '')) ?: '';
    $here = realpath(__DIR__ . '/..') ?: '';
    $rel = ($docRoot !== '' && strpos($here, $docRoot) === 0) ? str_replace('\\', '/', substr($here, strlen($docRoot))) : '';
    define('JSTKO_BASE', rtrim($rel, '/'));
}
require_once __DIR__ . '/config.php';
require_once __DIR__ . '/db.php';
require_once __DIR__ . '/helpers.php';
require_once __DIR__ . '/ko_password.php';
require_once __DIR__ . '/auth.php';
require_once __DIR__ . '/content.php';
require_once __DIR__ . '/cms.php';

jstko_boot_session();
