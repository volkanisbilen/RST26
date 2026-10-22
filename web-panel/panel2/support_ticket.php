<?php
require_once __DIR__ . '/includes/bootstrap.php';

$user = jstko_require_login();
$id = (int)($_GET['id'] ?? 0);

$stmt = jstko_db()->prepare('SELECT * FROM web_ticket WHERE id = :id AND account_id = :a');
$stmt->execute(['id' => $id, 'a' => $user['str_account_id']]);
$ticket = $stmt->fetch();
if (!$ticket) {
    http_response_code(404);
    echo 'Talep bulunamadı.';
    exit;
}

if ($_SERVER['REQUEST_METHOD'] === 'POST' && jstko_csrf_check() && $ticket['status'] === 'open') {
    $body = trim((string)($_POST['body'] ?? ''));
    if ($body !== '') {
        $pdo = jstko_db();
        $pdo->prepare('INSERT INTO web_ticket_reply (ticket_id, author, is_staff, body) VALUES (:id, :a, FALSE, :b)')
            ->execute(['id' => $id, 'a' => $user['str_account_id'], 'b' => $body]);
        $pdo->prepare('UPDATE web_ticket SET updated_at = now() WHERE id = :id')->execute(['id' => $id]);
        jstko_redirect('/support_ticket.php?id=' . $id);
    }
}

$replies = jstko_db()->prepare('SELECT * FROM web_ticket_reply WHERE ticket_id = :id ORDER BY created_at');
$replies->execute(['id' => $id]);
$replies = $replies->fetchAll();

$pageTitle = $ticket['subject'];
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-3xl mx-auto px-4 py-10">
  <a href="<?= JSTKO_BASE ?>/support.php" class="text-sm text-slate-400 hover:text-gold-400">&larr; Taleplerim</a>
  <h1 class="text-2xl font-bold text-white mt-2 mb-6"><?= jstko_e($ticket['subject']) ?></h1>

  <div class="space-y-4 mb-8">
    <div class="card p-4">
      <div class="text-xs text-slate-500 mb-2"><?= jstko_e($user['str_account_id']) ?> · <?= jstko_time_ago($ticket['created_at']) ?></div>
      <div class="text-sm text-slate-200 whitespace-pre-wrap"><?= jstko_e($ticket['body']) ?></div>
    </div>
    <?php foreach ($replies as $r): ?>
      <div class="card p-4 <?= jstko_pg_bool($r['is_staff']) ? 'border-gold-500/40 bg-gold-500/5' : '' ?>">
        <div class="text-xs mb-2 <?= jstko_pg_bool($r['is_staff']) ? 'text-gold-400 font-semibold' : 'text-slate-500' ?>">
          <?= jstko_pg_bool($r['is_staff']) ? '🛡 ' . jstko_e($r['author']) . ' (Yetkili)' : jstko_e($r['author']) ?> · <?= jstko_time_ago($r['created_at']) ?>
        </div>
        <div class="text-sm text-slate-200 whitespace-pre-wrap"><?= jstko_e($r['body']) ?></div>
      </div>
    <?php endforeach; ?>
  </div>

  <?php if ($ticket['status'] === 'open'): ?>
  <div class="card p-6">
    <form method="post" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <textarea class="input" name="body" rows="3" placeholder="Yanıtınız..." required></textarea>
      <button type="submit" class="btn-primary">Yanıtla</button>
    </form>
  </div>
  <?php else: ?>
    <div class="text-sm text-slate-500">Bu talep kapatılmıştır.</div>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
