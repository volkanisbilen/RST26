<?php
require_once __DIR__ . '/includes/bootstrap.php';
require_once __DIR__ . '/includes/captcha.php';
require_once __DIR__ . '/includes/mailer.php';

$user = jstko_current_user();   // null = visitor who is not logged in (can still write to support)
$error = null;
$old = ['name' => '', 'email' => '', 'account' => '', 'char' => '', 'subject' => '', 'body' => ''];

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    foreach ($old as $k => $_) $old[$k] = trim((string)($_POST[$k] ?? ''));
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen sayfayı yenileyip tekrar deneyin.';
    } elseif (!jstko_rate_limit($user ? 'ticket' : 'ticket_guest', jstko_client_ip(), $user ? 10 : 5, 3600)) {
        $error = 'Çok fazla talep gönderdiniz, lütfen daha sonra tekrar deneyin.';
    } elseif (!$user && ($captchaErr = jstko_captcha_error()) !== null) {
        $error = $captchaErr;
    } else {
        $subject = $old['subject'];
        $body = $old['body'];
        if ($subject === '' || $body === '') {
            $error = 'Konu ve mesaj gerekli.';
        } elseif (mb_strlen($body) > 4000) {
            $error = 'Mesaj en fazla 4000 karakter olabilir.';
        } elseif (!$user && !filter_var($old['email'], FILTER_VALIDATE_EMAIL)) {
            $error = 'Size dönebilmemiz için geçerli bir e-posta adresi yazın.';
        } else {
            $pdo = jstko_db();
            if ($user) {
                $stmt = $pdo->prepare('INSERT INTO web_ticket (account_id, subject, body) VALUES (:a, :s, :b) RETURNING id');
                $stmt->execute(['a' => $user['str_account_id'], 's' => mb_substr($subject, 0, 200), 'b' => $body]);
                jstko_redirect('/support_ticket.php?id=' . (int)$stmt->fetchColumn());
            }
            $token = bin2hex(random_bytes(16));
            $stmt = $pdo->prepare(
                'INSERT INTO web_ticket (account_id, subject, body, guest, contact_email, guest_account, guest_char, client_ip, access_token)
                 VALUES (:a, :s, :b, TRUE, :e, :ga, :gc, :ip, :t) RETURNING id'
            );
            $stmt->execute([
                'a' => 'misafir', 's' => mb_substr($subject, 0, 200), 'b' => $body, 'e' => mb_substr($old['email'], 0, 120),
                'ga' => mb_substr($old['account'], 0, 40), 'gc' => mb_substr($old['char'], 0, 40),
                'ip' => jstko_client_ip(), 't' => $token,
            ]);
            $id = (int)$stmt->fetchColumn();
            $link = (isset($_SERVER['HTTPS']) && $_SERVER['HTTPS'] !== 'off' ? 'https://' : 'http://') . ($_SERVER['HTTP_HOST'] ?? '') . JSTKO_BASE
                . '/support_ticket.php?id=' . $id . '&t=' . $token;
            try {
                jstko_send_mail($old['email'], $old['name'] ?: $old['email'], 'Destek talebiniz alındı (#' . $id . ')',
                    jstko_mail_layout('Destek talebiniz alındı', '<p>Talebiniz bize ulaştı. Yanıtı ve yazışmayı bu bağlantıdan takip edebilirsiniz:</p><p><a href="' . htmlspecialchars($link, ENT_QUOTES) . '">' . htmlspecialchars($link) . '</a></p><p>Bu bağlantıyı kimseyle paylaşmayın.</p>'));
            } catch (Throwable $e) {
                error_log('support guest mail: ' . $e->getMessage());
            }
            jstko_redirect('/support_ticket.php?id=' . $id . '&t=' . $token . '&new=1');
        }
    }
}

$tickets = [];
if ($user) {
    $st = jstko_db()->prepare('SELECT * FROM web_ticket WHERE account_id = :a ORDER BY updated_at DESC');
    $st->execute(['a' => $user['str_account_id']]);
    $tickets = $st->fetchAll();
}

$pageTitle = 'Destek';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-3xl mx-auto px-4 py-10">
  <h1>Destek</h1>
  <p class="text-sm text-slate-400 mb-6">
    <?= $user ? 'Sorununuzu yazın, yetkililerimiz en kısa sürede dönüş yapacak.'
              : 'Giriş yapamıyor veya hesabınızı doğrulayamıyorsanız buradan giriş yapmadan da bize yazabilirsiniz. Yanıtı e-postanıza göndeririz.' ?>
  </p>

  <div class="card p-6 mb-8">
    <h2 class="font-bold text-white mb-3">Yeni Destek Talebi</h2>
    <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-4"><?= jstko_e($error) ?></div><?php endif; ?>
    <form method="post" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <?php if (!$user): ?>
        <div class="grid sm:grid-cols-2 gap-3">
          <label class="block text-xs text-slate-400">Adınız
            <input class="input mt-1" type="text" name="name" value="<?= jstko_e($old['name']) ?>" maxlength="60" placeholder="İsteğe bağlı">
          </label>
          <label class="block text-xs text-slate-400">E-posta adresiniz *
            <input class="input mt-1" type="email" name="email" value="<?= jstko_e($old['email']) ?>" maxlength="120" required placeholder="size@ornek.com">
          </label>
          <label class="block text-xs text-slate-400">Oyun hesabı adı
            <input class="input mt-1" type="text" name="account" value="<?= jstko_e($old['account']) ?>" maxlength="40" placeholder="Biliyorsanız">
          </label>
          <label class="block text-xs text-slate-400">Karakter adı
            <input class="input mt-1" type="text" name="char" value="<?= jstko_e($old['char']) ?>" maxlength="40" placeholder="Biliyorsanız">
          </label>
        </div>
      <?php endif; ?>
      <input class="input" type="text" name="subject" value="<?= jstko_e($old['subject']) ?>" placeholder="Konu" required maxlength="200">
      <textarea class="input" name="body" rows="5" placeholder="Mesajınız..." required maxlength="4000"><?= jstko_e($old['body']) ?></textarea>
      <?php if (!$user): ?><?= jstko_captcha_field() ?><?php endif; ?>
      <button type="submit" class="btn-primary">Talep Oluştur</button>
    </form>
  </div>

  <?php if ($user): ?>
  <h2 class="font-bold text-white mb-3">Taleplerim</h2>
  <?php if (!$tickets): ?>
    <div class="card p-10 text-center text-slate-500">Henüz destek talebiniz yok.</div>
  <?php else: ?>
  <div class="space-y-2">
    <?php foreach ($tickets as $t): ?>
      <a href="<?= JSTKO_BASE ?>/support_ticket.php?id=<?= (int)$t['id'] ?>" class="card p-4 flex items-center justify-between hover:border-gold-400 transition">
        <div>
          <div class="font-medium text-white"><?= jstko_e($t['subject']) ?></div>
          <div class="text-xs text-slate-500 mt-1"><?= jstko_time_ago($t['created_at']) ?></div>
        </div>
        <span class="text-xs px-2 py-1 rounded-full <?= $t['status'] === 'open' ? 'bg-amber-900/30 text-amber-300 border border-amber-700' : 'bg-slate-800 text-slate-400 border border-slate-700' ?>">
          <?= $t['status'] === 'open' ? 'Açık' : 'Kapalı' ?>
        </span>
      </a>
    <?php endforeach; ?>
  </div>
  <?php endif; ?>
  <?php else: ?>
    <p class="text-sm text-slate-500">Hesabınız varsa <a href="<?= JSTKO_BASE ?>/login.php" class="text-gold-400 underline">giriş yaparak</a> eski taleplerinizi de görebilirsiniz.</p>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
