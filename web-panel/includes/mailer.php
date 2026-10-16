<?php
// Minimal dependency-free SMTP client (AUTH LOGIN, STARTTLS or implicit SSL) so the panel sends
// activation/reset mail on generic shared hosting without composer/PHPMailer. Logs failures to
// PHP's error log; a mail failure never throws out to the caller (registration/reset still
// succeeds -- the player just needs a resend, handled by the caller).
declare(strict_types=1);

require_once __DIR__ . '/config.php';

function jstko_send_mail(string $toEmail, string $toName, string $subject, string $htmlBody): bool
{
    $c = jstko_config()['smtp'];
    $host = $c['host'];
    $port = (int)$c['port'];
    $encryption = $c['encryption'] ?? '';

    $transport = $encryption === 'ssl' ? 'ssl://' . $host : $host;
    $sock = @stream_socket_client($transport . ':' . $port, $errno, $errstr, 15);
    if (!$sock) {
        error_log("jstko_send_mail: connect failed: $errstr ($errno)");
        return false;
    }
    stream_set_timeout($sock, 15);

    $read = function () use ($sock): string {
        $data = '';
        while (($line = fgets($sock, 515)) !== false) {
            $data .= $line;
            if (strlen($line) < 4 || $line[3] === ' ') break;
        }
        return $data;
    };
    $write = function (string $cmd) use ($sock) {
        fwrite($sock, $cmd . "\r\n");
    };
    $expect = function (string $resp, string $code) {
        return str_starts_with($resp, $code);
    };

    $resp = $read();
    if (!$expect($resp, '220')) {
        error_log("jstko_send_mail: no 220 greeting: $resp");
        fclose($sock);
        return false;
    }

    $write('EHLO ' . ($_SERVER['SERVER_NAME'] ?? 'localhost'));
    $resp = $read();

    if ($encryption === 'tls') {
        $write('STARTTLS');
        $resp = $read();
        if (!$expect($resp, '220')) {
            error_log("jstko_send_mail: STARTTLS refused: $resp");
            fclose($sock);
            return false;
        }
        if (!stream_socket_enable_crypto($sock, true, STREAM_CRYPTO_METHOD_TLS_CLIENT)) {
            error_log('jstko_send_mail: TLS handshake failed');
            fclose($sock);
            return false;
        }
        $write('EHLO ' . ($_SERVER['SERVER_NAME'] ?? 'localhost'));
        $resp = $read();
    }

    if (!empty($c['username'])) {
        $write('AUTH LOGIN');
        $resp = $read();
        if (!$expect($resp, '334')) {
            error_log("jstko_send_mail: AUTH LOGIN refused: $resp");
            fclose($sock);
            return false;
        }
        $write(base64_encode($c['username']));
        $read();
        $write(base64_encode($c['password']));
        $resp = $read();
        if (!$expect($resp, '235')) {
            error_log("jstko_send_mail: auth failed: $resp");
            fclose($sock);
            return false;
        }
    }

    $fromEmail = $c['from_email'];
    $fromName = $c['from_name'];

    $write('MAIL FROM:<' . $fromEmail . '>');
    $resp = $read();
    if (!$expect($resp, '250')) {
        error_log("jstko_send_mail: MAIL FROM refused: $resp");
        fclose($sock);
        return false;
    }

    $write('RCPT TO:<' . $toEmail . '>');
    $resp = $read();
    if (!$expect($resp, '250') && !$expect($resp, '251')) {
        error_log("jstko_send_mail: RCPT TO refused: $resp");
        fclose($sock);
        return false;
    }

    $write('DATA');
    $resp = $read();
    if (!$expect($resp, '354')) {
        error_log("jstko_send_mail: DATA refused: $resp");
        fclose($sock);
        return false;
    }

    $encodeHeader = fn(string $s) => '=?UTF-8?B?' . base64_encode($s) . '?=';
    $boundaryEscaped = str_replace(["\r", "\n", '.'], '', $htmlBody);
    // Escape leading dots per RFC 5321 (a line of just "." ends DATA).
    $body = preg_replace('/^\./m', '..', $boundaryEscaped);

    $headers = [
        'From: ' . $encodeHeader($fromName) . ' <' . $fromEmail . '>',
        'To: ' . $encodeHeader($toName) . ' <' . $toEmail . '>',
        'Subject: ' . $encodeHeader($subject),
        'MIME-Version: 1.0',
        'Content-Type: text/html; charset=UTF-8',
        'Content-Transfer-Encoding: 8bit',
        'Date: ' . date('r'),
    ];
    $write(implode("\r\n", $headers) . "\r\n\r\n" . $body . "\r\n.");
    $resp = $read();
    if (!$expect($resp, '250')) {
        error_log("jstko_send_mail: message rejected: $resp");
        fclose($sock);
        return false;
    }

    $write('QUIT');
    fclose($sock);
    return true;
}

function jstko_mail_layout(string $title, string $bodyHtml): string
{
    $siteName = jstko_e(jstko_config()['site']['name']);
    return <<<HTML
    <div style="font-family:Arial,sans-serif;background:#0b0e14;padding:32px;color:#e5e7eb">
      <div style="max-width:480px;margin:0 auto;background:#141925;border-radius:12px;padding:32px;border:1px solid #232a3b">
        <h1 style="color:#facc15;font-size:20px;margin:0 0 16px">{$siteName}</h1>
        <h2 style="font-size:16px;margin:0 0 16px;color:#fff">{$title}</h2>
        <div style="font-size:14px;line-height:1.6;color:#cbd5e1">{$bodyHtml}</div>
        <p style="margin-top:32px;font-size:12px;color:#64748b">Bu e-postayı siz talep etmediyseniz yok sayabilirsiniz.</p>
      </div>
    </div>
    HTML;
}
