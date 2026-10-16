<?php
declare(strict_types=1);

require_once __DIR__ . '/db.php';

function jstko_pus_normalize_code(string $raw): string
{
    $upper = strtoupper($raw);
    return preg_replace('/[^A-Z0-9]/', '', $upper) ?? '';
}

function jstko_pus_canonical_code(string $raw): string
{
    $n = jstko_pus_normalize_code($raw);
    return implode('-', str_split($n, 4));
}

/**
 * Mirrors ko-game's handle_redeem (pus_codes.rs) exactly, but credits tb_user directly instead
 * of a live in-game session -- if the player is online right now, the game's own in-memory
 * balance only catches up on its next refresh/relogin; no money is lost either way.
 *
 * @return array{ok:bool, message:string}
 */
function jstko_pus_redeem(string $accountId, string $charNameForLog, string $ip, string $rawCode): array
{
    $pdo = jstko_db();
    $normalized = jstko_pus_normalize_code($rawCode);
    $code = jstko_pus_canonical_code($rawCode);

    $logAttempt = function (string $result) use ($pdo, $accountId, $charNameForLog, $ip, $rawCode) {
        $stmt = $pdo->prepare('INSERT INTO pus_code_attempt (account, char_name, ip, code, result) VALUES (:a, :c, :ip, :code, :r)');
        $stmt->execute(['a' => $accountId, 'c' => $charNameForLog, 'ip' => $ip, 'code' => substr($rawCode, 0, 40), 'r' => $result]);
    };

    if (strlen($normalized) < 8 || strlen($normalized) > 20) {
        $logAttempt('invalid');
        return ['ok' => false, 'message' => 'Geçersiz kod formatı.'];
    }

    $stmt = $pdo->prepare(
        "SELECT COUNT(*) FROM pus_code_attempt WHERE account = :a AND result IN ('invalid','used','disabled')
         AND at > now() - interval '10 minutes'"
    );
    $stmt->execute(['a' => $accountId]);
    if ((int)$stmt->fetchColumn() >= 5) {
        $logAttempt('blocked');
        return ['ok' => false, 'message' => 'Çok fazla yanlış deneme yaptınız, 10 dakika sonra tekrar deneyin.'];
    }

    $pdo->beginTransaction();
    try {
        $stmt = $pdo->prepare(
            'UPDATE pus_code SET used_at = now(), used_account = :a, used_char = :c, used_ip = :ip
             WHERE code = :code AND used_at IS NULL AND NOT disabled AND (expires_at IS NULL OR expires_at > now())
             RETURNING kc, tl'
        );
        $stmt->execute(['a' => $accountId, 'c' => $charNameForLog, 'ip' => $ip, 'code' => $code]);
        $claim = $stmt->fetch();

        if (!$claim) {
            $pdo->rollBack();
            $stmt = $pdo->prepare(
                'SELECT used_at IS NOT NULL AS used, disabled, (expires_at IS NOT NULL AND expires_at <= now()) AS expired
                 FROM pus_code WHERE code = :code'
            );
            $stmt->execute(['code' => $code]);
            $state = $stmt->fetch();
            if (!$state) {
                $logAttempt('invalid');
                return ['ok' => false, 'message' => 'Bu kod geçerli değil.'];
            }
            if ($state['used'] === 't' || $state['used'] === true) {
                $logAttempt('used');
                return ['ok' => false, 'message' => 'Bu kod daha önce kullanılmış.'];
            }
            $logAttempt('disabled');
            return ['ok' => false, 'message' => 'Bu kodun süresi dolmuş veya devre dışı bırakılmış.'];
        }

        $kc = (int)$claim['kc'];
        $tl = (int)$claim['tl'];

        $stmt = $pdo->prepare(
            'UPDATE tb_user SET cash_point = cash_point + :kc, bonus_cash_point = bonus_cash_point + :tl
             WHERE str_account_id = :a RETURNING cash_point, bonus_cash_point'
        );
        $stmt->execute(['kc' => $kc, 'tl' => $tl, 'a' => $accountId]);
        $balances = $stmt->fetch();

        $stmt = $pdo->prepare(
            'INSERT INTO kc_ledger (account, char_name, kc_delta, tl_delta, kc_after, tl_after, reason, ref)
             VALUES (:a, :c, :kcd, :tld, :kca, :tla, :reason, :ref)'
        );
        $stmt->execute([
            'a' => $accountId, 'c' => $charNameForLog, 'kcd' => $kc, 'tld' => $tl,
            'kca' => $balances['cash_point'], 'tla' => $balances['bonus_cash_point'],
            'reason' => 'esn', 'ref' => $code,
        ]);
        $pdo->commit();
    } catch (Throwable $e) {
        $pdo->rollBack();
        error_log('jstko_pus_redeem: ' . $e->getMessage());
        return ['ok' => false, 'message' => 'Bir hata oluştu, lütfen tekrar deneyin.'];
    }

    $logAttempt('ok');
    $parts = [];
    if ($kc > 0) $parts[] = jstko_number($kc) . ' KC';
    if ($tl > 0) $parts[] = jstko_number($tl) . ' TL';
    $amount = $parts ? implode(' + ', $parts) : '0';
    return ['ok' => true, 'message' => "Kod başarıyla kullanıldı: +{$amount} hesabınıza eklendi."];
}
