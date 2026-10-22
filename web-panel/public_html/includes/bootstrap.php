<?php
declare(strict_types=1);
error_reporting(E_ALL);
ini_set('display_errors', '1'); // TEMP while we diagnose the first deploy -- flip back to '0' once it works

// Folder this panel is served from ("" at the web root, "/panel2" in a sub folder): every internal link is
// prefixed with it, uploads and item icons keep pointing at the main site's shared files.
if (!defined('JSTKO_BASE')) {
    // the URL of the running script minus its path inside this folder = the folder's URL
    // (immune to symlinked / aliased document roots, unlike comparing DOCUMENT_ROOT)
    $rel = '';
    $root = str_replace(chr(92), '/', (string)realpath(__DIR__ . '/..'));
    $script = str_replace(chr(92), '/', (string)realpath((string)($_SERVER['SCRIPT_FILENAME'] ?? '')));
    $name = str_replace(chr(92), '/', (string)($_SERVER['SCRIPT_NAME'] ?? ''));
    if ($root !== '' && $script !== '' && strpos($script, $root) === 0) {
        $inner = substr($script, strlen($root));
        if ($inner !== '' && substr($name, -strlen($inner)) === $inner) $rel = substr($name, 0, -strlen($inner));
    }
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
