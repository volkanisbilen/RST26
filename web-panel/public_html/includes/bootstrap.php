<?php
declare(strict_types=1);
error_reporting(E_ALL);
ini_set('display_errors', '0'); // flip to '1' while developing locally

require_once __DIR__ . '/config.php';
require_once __DIR__ . '/db.php';
require_once __DIR__ . '/helpers.php';
require_once __DIR__ . '/auth.php';
require_once __DIR__ . '/content.php';

jstko_boot_session();
