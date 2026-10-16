<?php
declare(strict_types=1);
error_reporting(E_ALL);
ini_set('display_errors', '1'); // TEMP while we diagnose the first deploy -- flip back to '0' once it works

require_once __DIR__ . '/config.php';
require_once __DIR__ . '/db.php';
require_once __DIR__ . '/helpers.php';
require_once __DIR__ . '/auth.php';
require_once __DIR__ . '/content.php';

jstko_boot_session();
