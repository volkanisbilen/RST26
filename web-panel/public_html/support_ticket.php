<?php
require_once __DIR__ . '/includes/bootstrap.php';

$user = jstko_current_user();
$id = (int)($_GET['id'] ?? $_POST['id'] ?? 0);
$token = (string)($_GET['t'] ?? $_POST['t'] ?? '');

$stmt = jstko_db()->prepare('SELECT * FROM web_ticket WHERE id = :id');
$stmt->execute(['id' => $id]);
$ticket = $stmt->fetch();

// owner of the ticket: the logged-in account, or (guest ticket) whoever holds the private link
$allowed = false;
if ($ticket) {
    if (jstko_pg_bool($ticket['guest'])) {
        $allowed = $token !== '' && $ticket['access_token'] !== '' && hash_equals((string)$ticket['access_token'], $token);
    } else {
        $allowed = $user && $ticket['account_id'] === $user['str_account_id'];
    }
}
if (!$allowed) {
    if ($ticket && !jstko_pg_bool($ticket['guest']) && !$user) jstko_redirect('/login.php');
    http_response_code(404);
    echo 'Talep bulunamadı.';
    exit;
}
$isGuest = jstko_pg_bool($ticket['guest']);
$author = $isGuest ? ('Misafir' . ($ticket['guest_account'] !== '' ? ' (' . $ticket['guest_account'] . ')' : '')) : $user['str_account_id'];
$self = $isGuest ? '/support_ticket.php?id=' . $id . '&t=' . urlencode($token) : '/support_ticket.php?id=' . $id;

if ($_SERVER['REQUEST_METHOD'] === 'POST' && jstko_csrf_check() && $ticket['status'] === 'open'
    && jstko_rate_limit('ticket_reply', jstko_client_ip(), 30, 3600)) {
    $body = trim((string)($_POST['body'] ?? ''));
    if ($body !== '' && mb_strlen($body) <= 4000) {
        $pdo = jstko_db();
        $pdo->prepare('INSERT INTO web_ticket_reply (ticket_id, author, is_staff, body) VALUES (:id, :a, FALSE, :b)')
            ->execute(['id' => $id, 'a' => $author, 'b' => $body]);
        $pdo->prepare('UPDATE web_ticket SET updated_at = now() WHERE id = :id')->execute(['id' => $id]);
        jstko_redirect($self);
    }
}

$replies = jstko_db()->prepare('SELECT * FROM web_ticket_reply WHERE ticket_id = :id ORDER BY created_at');
$replies->execute(['id' => $id]);
$replies = $replies->fetchAll();

$pageTitle = $ticket['subject'];
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-3xl mx-auto px-4 py-10">
  <?php if (!$isGuest): ?><a href="<?= JSTKO_BASE ?>/support.php" class="text-sm text-slate-400 hover:text-gold-400">&larr; Taleplerim</a><?php endif; ?>
  <h1 class="text-2xl font-bold text-white mt-2 mb-6"><?= jstko_e($ticket['subject']) ?></h1>

  <?php if ($isGuest && !empty($_GET['new'])): ?>
    <div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6">
      Talebiniz alındı. Yanıtı <b><?= jstko_e($ticket['contact_email']) ?></b> adresine ve bu sayfaya göndereceğiz.
      Bu sayfanın adresini (bağlantıyı) kaydedin, yazışmaya buradan devam edebilirsiniz; e-postanıza da gönderdik.
    </div>
  <?php endif; ?>

  <div class="space-y-4 mb-8">
    <div class="card p-4">
      <div class="text-xs text-slate-500 mb-2"><?= jstko_e($author) ?> · <?= jstko_time_ago($ticket['created_at']) ?></div>
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
      <input type="hidden" name="id" value="<?= $id ?>">
      <?php if ($isGuest): ?><input type="hidden" name="t" value="<?= jstko_e($token) ?>"><?php endif; ?>
      <textarea class="input" name="body" rows="3" placeholder="Yanıtınız..." required maxlength="4000"></textarea>
      <button type="submit" class="btn-primary">Yanıtla</button>
    </form>
  </div>
  <?php else: ?>
    <div class="text-sm text-slate-500">Bu talep kapatılmıştır.</div>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
