<?php
declare(strict_types=1);

require_once __DIR__ . '/config.php';
require_once __DIR__ . '/db.php';
require_once __DIR__ . '/helpers.php';
require_once __DIR__ . '/ko_password.php';
require_once __DIR__ . '/mailer.php';
require_once __DIR__ . '/ratelimit.php';

function jstko_boot_session(): void
{
    if (session_status() === PHP_SESSION_ACTIVE) {
        return;
    }
    $s = jstko_config()['session'];
    session_set_cookie_params([
        'lifetime' => $s['lifetime_seconds'],
        'path' => '/',
        'secure' => !empty($_SERVER['HTTPS']),
        'httponly' => true,
        'samesite' => 'Lax',
    ]);
    session_name($s['name']);
    session_start();
}

/**
 * The logged-in account's tb_user row, or null. Re-fetched once per request (not cached across
 * requests) so a ban/authority change takes effect immediately.
 */
function jstko_current_user(): ?array
{
    static $user = false; // false = "not loaded yet", null = "loaded, no session"
    if ($user !== false) {
        return $user;
    }
    $id = $_SESSION['account_id'] ?? null;
    if (!$id) {
        $user = null;
        return null;
    }
    $stmt = jstko_db()->prepare('SELECT * FROM tb_user WHERE str_account_id = :id');
    $stmt->execute(['id' => $id]);
    $row = $stmt->fetch();
    if (!$row) {
        unset($_SESSION['account_id']);
        $user = null;
        return null;
    }
    $user = $row;
    return $row;
}

function jstko_require_login(): array
{
    $user = jstko_current_user();
    if (!$user) {
        jstko_redirect('/login.php?next=' . urlencode($_SERVER['REQUEST_URI'] ?? '/'));
    }
    return $user;
}

function jstko_require_gm(): array
{
    $user = jstko_require_login();
    if (!jstko_is_gm((int)$user['str_authority'])) {
        http_response_code(403);
        echo 'Bu sayfaya erişim yetkiniz yok.';
        exit;
    }
    return $user;
}

/** @return array{ok:bool, error?:string} */
function jstko_login(string $accountId, string $password): array
{
    $stmt = jstko_db()->prepare('SELECT * FROM tb_user WHERE str_account_id = :id');
    $stmt->execute(['id' => $accountId]);
    $user = $stmt->fetch();

    if (!$user || !hash_equals((string)$user['str_passwd'], jstko_ko_password_encode($password))) {
        return ['ok' => false, 'error' => 'Kullanıcı adı veya şifre hatalı.'];
    }
    if ((int)$user['str_authority'] < 0) {
        return ['ok' => false, 'error' => 'Bu hesap yasaklanmış.'];
    }
    if ((int)$user['account_check'] === 0) {
        return ['ok' => false, 'error' => 'Hesabınız henüz e-posta ile doğrulanmadı. Lütfen e-postanızı kontrol edin.'];
    }

    session_regenerate_id(true);
    $_SESSION['account_id'] = $user['str_account_id'];
    return ['ok' => true];
}

function jstko_logout(): void
{
    $_SESSION = [];
    if (ini_get('session.use_cookies')) {
        $params = session_get_cookie_params();
        setcookie(session_name(), '', time() - 42000, $params['path'], $params['domain'], $params['secure'], $params['httponly']);
    }
    session_destroy();
}

/** @return array{ok:bool, error?:string} */
function jstko_register(array $f): array
{
    $accountId = trim((string)($f['account_id'] ?? ''));
    $password = (string)($f['password'] ?? '');
    $email = trim((string)($f['email'] ?? ''));
    $securityQuestion = (string)($f['security_question'] ?? '');
    $securityAnswer = trim((string)($f['security_answer'] ?? ''));
    $sealPassword = (string)($f['seal_password'] ?? '');
    $phone = preg_replace('/\D/', '', (string)($f['phone'] ?? ''));

    if (!jstko_valid_account_id($accountId)) {
        return ['ok' => false, 'error' => 'Kullanıcı adı 3-21 karakter olmalı, sadece harf/rakam/":"/"_" içerebilir.'];
    }
    if (strlen($password) < 6 || strlen($password) > 16) {
        return ['ok' => false, 'error' => 'Şifre 6-16 karakter olmalı.'];
    }
    if (!filter_var($email, FILTER_VALIDATE_EMAIL)) {
        return ['ok' => false, 'error' => 'Geçerli bir e-posta adresi girin.'];
    }
    if ($securityAnswer === '' || strlen($securityAnswer) > 50) {
        return ['ok' => false, 'error' => 'Gizli soru cevabı gerekli.'];
    }
    if (!preg_match('/^\d{8}$/', $sealPassword)) {
        return ['ok' => false, 'error' => 'İtem kilit şifresi 8 haneli rakam olmalı.'];
    }
    if ($phone !== '' && strlen($phone) > 10) {
        return ['ok' => false, 'error' => 'Telefon numarası geçersiz.'];
    }

    $pdo = jstko_db();
    $stmt = $pdo->prepare('SELECT 1 FROM tb_user WHERE str_account_id = :id');
    $stmt->execute(['id' => $accountId]);
    if ($stmt->fetch()) {
        return ['ok' => false, 'error' => 'Bu kullanıcı adı zaten alınmış.'];
    }
    $stmt = $pdo->prepare('SELECT 1 FROM tb_user WHERE lower(email) = lower(:email)');
    $stmt->execute(['email' => $email]);
    if ($stmt->fetch()) {
        return ['ok' => false, 'error' => 'Bu e-posta adresi zaten kullanılıyor.'];
    }

    $pdo->beginTransaction();
    try {
        $stmt = $pdo->prepare(
            'INSERT INTO tb_user (str_account_id, str_passwd, str_seal_passwd, email,
                security_question, security_answer, user_phone_number, str_authority, account_check, str_client_ip)
             VALUES (:id, :pw, :seal, :email, :sq, :sa, :phone, 1, 0, :ip)'
        );
        $stmt->execute([
            'id' => $accountId,
            'pw' => jstko_ko_password_encode($password),
            'seal' => $sealPassword,
            'email' => $email,
            'sq' => $securityQuestion,
            'sa' => $securityAnswer,
            'phone' => $phone !== '' ? $phone : null,
            'ip' => jstko_client_ip(),
        ]);

        $token = jstko_random_token();
        $stmt = $pdo->prepare(
            "INSERT INTO web_email_verify (token, account_id, expires_at) VALUES (:t, :id, now() + interval '48 hours')"
        );
        $stmt->execute(['t' => $token, 'id' => $accountId]);
        $pdo->commit();
    } catch (Throwable $e) {
        $pdo->rollBack();
        error_log('jstko_register: ' . $e->getMessage());
        return ['ok' => false, 'error' => 'Kayıt sırasında bir hata oluştu, lütfen tekrar deneyin.'];
    }

    jstko_send_activation_mail($accountId, $email, $token);
    return ['ok' => true];
}

function jstko_send_activation_mail(string $accountId, string $email, string $token): bool
{
    $base = rtrim(jstko_config()['site']['base_url'], '/');
    $link = $base . '/activate.php?token=' . urlencode($token);
    $body = "Merhaba <b>" . jstko_e($accountId) . "</b>,<br><br>" .
        "Hesabını aktifleştirmek için aşağıdaki bağlantıya tıkla:<br><br>" .
        '<a href="' . jstko_e($link) . '" style="color:#facc15">' . jstko_e($link) . '</a><br><br>' .
        'Bu bağlantı 48 saat geçerlidir.';
    return jstko_send_mail($email, $accountId, jstko_config()['site']['name'] . ' - Hesap Aktivasyonu', jstko_mail_layout('Hesabını Aktifleştir', $body));
}

/** Always returns ok (doesn't reveal whether the account/email exists) unless input is malformed. */
function jstko_request_password_reset(string $accountId, string $email): array
{
    $accountId = trim($accountId);
    $email = trim($email);
    if ($accountId === '' || $email === '') {
        return ['ok' => false, 'error' => 'Kullanıcı adı ve e-posta gerekli.'];
    }
    $pdo = jstko_db();
    $stmt = $pdo->prepare('SELECT * FROM tb_user WHERE str_account_id = :id AND lower(email) = lower(:email)');
    $stmt->execute(['id' => $accountId, 'email' => $email]);
    $user = $stmt->fetch();
    if ($user) {
        $token = jstko_random_token();
        $pdo->prepare("INSERT INTO web_password_reset (token, account_id, expires_at) VALUES (:t, :id, now() + interval '2 hours')")
            ->execute(['t' => $token, 'id' => $accountId]);

        $base = rtrim(jstko_config()['site']['base_url'], '/');
        $link = $base . '/reset_password.php?token=' . urlencode($token);
        $body = 'Şifreni yenilemek için aşağıdaki bağlantıya tıkla:<br><br>' .
            '<a href="' . jstko_e($link) . '" style="color:#facc15">' . jstko_e($link) . '</a><br><br>' .
            'Bu bağlantı 2 saat geçerlidir.';
        jstko_send_mail($email, $accountId, jstko_config()['site']['name'] . ' - Şifre Yenileme', jstko_mail_layout('Şifreni Yenile', $body));
    }
    // Same response whether or not it matched, so this can't be used to probe accounts/emails.
    return ['ok' => true];
}

function jstko_password_reset_token_account(string $token): ?string
{
    $stmt = jstko_db()->prepare("SELECT account_id FROM web_password_reset WHERE token = :t AND used_at IS NULL AND expires_at > now()");
    $stmt->execute(['t' => $token]);
    $row = $stmt->fetch();
    return $row ? $row['account_id'] : null;
}

/** @return array{ok:bool, error?:string} */
function jstko_reset_password(string $token, string $newPassword): array
{
    $accountId = jstko_password_reset_token_account($token);
    if (!$accountId) {
        return ['ok' => false, 'error' => 'Bağlantının süresi dolmuş veya geçersiz.'];
    }
    if (strlen($newPassword) < 6 || strlen($newPassword) > 16) {
        return ['ok' => false, 'error' => 'Şifre 6-16 karakter olmalı.'];
    }
    $pdo = jstko_db();
    $pdo->beginTransaction();
    try {
        $pdo->prepare('UPDATE tb_user SET str_passwd = :pw WHERE str_account_id = :id')
            ->execute(['pw' => jstko_ko_password_encode($newPassword), 'id' => $accountId]);
        $pdo->prepare('UPDATE web_password_reset SET used_at = now() WHERE token = :t')
            ->execute(['t' => $token]);
        $pdo->commit();
    } catch (Throwable $e) {
        $pdo->rollBack();
        error_log('jstko_reset_password: ' . $e->getMessage());
        return ['ok' => false, 'error' => 'Şifre değiştirme sırasında bir hata oluştu.'];
    }
    return ['ok' => true];
}

/** @return array{ok:bool, error?:string} */
function jstko_activate_account(string $token): array
{
    $pdo = jstko_db();
    $stmt = $pdo->prepare(
        "SELECT * FROM web_email_verify WHERE token = :t AND used_at IS NULL AND expires_at > now()"
    );
    $stmt->execute(['t' => $token]);
    $row = $stmt->fetch();
    if (!$row) {
        return ['ok' => false, 'error' => 'Bağlantının süresi dolmuş veya geçersiz.'];
    }
    $pdo->beginTransaction();
    try {
        $pdo->prepare('UPDATE tb_user SET account_check = 1 WHERE str_account_id = :id')
            ->execute(['id' => $row['account_id']]);
        $pdo->prepare('UPDATE web_email_verify SET used_at = now() WHERE token = :t')
            ->execute(['t' => $token]);
        $pdo->commit();
    } catch (Throwable $e) {
        $pdo->rollBack();
        error_log('jstko_activate_account: ' . $e->getMessage());
        return ['ok' => false, 'error' => 'Aktivasyon sırasında bir hata oluştu.'];
    }
    return ['ok' => true];
}
