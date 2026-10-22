<?php
declare(strict_types=1);

/**
 * Self-contained human check for forms that can be sent without logging in (no external service or API key):
 *  - a small arithmetic question whose answer lives only in the session (one use, 15 minutes),
 *  - a hidden honeypot field bots fill in,
 *  - a minimum fill-in time (people need a few seconds to write a message).
 */

function jstko_captcha_new(): string
{
    $a = random_int(3, 19);
    $b = random_int(2, 9);
    switch (random_int(0, 2)) {
        case 0:
            $q = "$a + $b";
            $ans = $a + $b;
            break;
        case 1:
            $q = ($a + $b) . " - $b";
            $ans = $a;
            break;
        default:
            $x = random_int(2, 9);
            $q = "$x × $b";
            $ans = $x * $b;
    }
    $_SESSION['captcha'] = ['answer' => $ans, 'at' => time()];
    return $q;
}

/** True when the visitor's answer matches; the question is consumed either way. */
function jstko_captcha_check(string $input): bool
{
    $c = $_SESSION['captcha'] ?? null;
    unset($_SESSION['captcha']);
    if (!is_array($c) || time() - (int)$c['at'] > 900) return false;
    $input = trim($input);
    return $input !== '' && ctype_digit($input) && (int)$input === (int)$c['answer'];
}

/** Hidden anti-bot inputs + the visible question. Echo inside the form. */
function jstko_captcha_field(): string
{
    $q = jstko_captcha_new();
    $_SESSION['form_at'] = time();
    return '<div style="position:absolute;left:-9999px;top:-9999px" aria-hidden="true"><label>Web sitesi<input type="text" name="website" tabindex="-1" autocomplete="off"></label></div>'
        . '<label class="block text-sm text-slate-300">Ben robot değilim: <b class="text-gold-400 text-base mx-1">' . htmlspecialchars($q, ENT_QUOTES, 'UTF-8') . ' = ?</b>'
        . '<input class="input mt-1 w-32" type="text" name="captcha" inputmode="numeric" autocomplete="off" required maxlength="3" placeholder="Cevap"></label>';
}

/** Honeypot + minimum time + the question. Returns an error text, or null when the visitor passes. */
function jstko_captcha_error(): ?string
{
    if (trim((string)($_POST['website'] ?? '')) !== '') return 'Form doğrulanamadı.';
    $at = (int)($_SESSION['form_at'] ?? 0);
    if ($at === 0 || time() - $at < 4) return 'Form çok hızlı gönderildi, lütfen tekrar deneyin.';
    if (!jstko_captcha_check((string)($_POST['captcha'] ?? ''))) return 'Güvenlik sorusunun cevabı yanlış, lütfen yeniden deneyin.';
    return null;
}
