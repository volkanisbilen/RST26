<?php
require_once __DIR__ . '/includes/bootstrap.php';

$user = jstko_require_login();
$error = null;

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } elseif (!jstko_rate_limit('ticket', jstko_client_ip(), 10, 3600)) {
        $error = 'Çok fazla talep oluşturdunuz, lütfen daha sonra tekrar deneyin.';
    } else {
        $subject = trim((string)($_POST['subject'] ?? ''));
        $body = trim((string)($_POST['body'] ?? ''));
        if ($subject === '' || $body === '') {
            $error = 'Konu ve mesaj gerekli.';
        } else {
            $stmt = jstko_db()->prepare('INSERT INTO web_ticket (account_id, subject, body) VALUES (:a, :s, :b) RETURNING id');
            $stmt->execute(['a' => $user['str_account_id'], 's' => substr($subject, 0, 200), 'b' => $body]);
            $id = $stmt->fetchColumn();
            jstko_redirect('/support_ticket.php?id=' . (int)$id);
        }
    }
}

$tickets = jstko_db()->prepare('SELECT * FROM web_ticket WHERE account_id = :a ORDER BY updated_at DESC');
$tickets->execute(['a' => $user['str_account_id']]);
$tickets = $tickets->fetchAll();

$pageTitle = 'Destek';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-3xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-6">Destek</h1>

  <div class="card p-6 mb-8">
    <h2 class="font-bold text-white mb-3">Yeni Destek Talebi</h2>
    <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-4"><?= jstko_e($error) ?></div><?php endif; ?>
    <form method="post" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input class="input" type="text" name="subject" placeholder="Konu" required maxlength="200">
      <textarea class="input" name="body" rows="4" placeholder="Mesajınız..." required></textarea>
      <button type="submit" class="btn-primary">Talep Oluştur</button>
    </form>
  </div>

  <h2 class="font-bold text-white mb-3">Taleplerim</h2>
  <?php if (!$tickets): ?>
    <div class="card p-10 text-center text-slate-500">Henüz destek talebiniz yok.</div>
  <?php else: ?>
  <div class="space-y-2">
    <?php foreach ($tickets as $t): ?>
      <a href="/support_ticket.php?id=<?= (int)$t['id'] ?>" class="card p-4 flex items-center justify-between hover:border-gold-400 transition">
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
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
