<?php
require_once __DIR__ . '/../../includes/bootstrap.php';
$user = jstko_require_gm();

$id = (int)($_GET['id'] ?? 0);
$pdo = jstko_db();

if ($_SERVER['REQUEST_METHOD'] === 'POST' && jstko_csrf_check()) {
    $action = $_POST['form_action'] ?? '';
    $ticketId = (int)($_POST['ticket_id'] ?? 0);
    if ($action === 'reply') {
        $body = trim((string)($_POST['body'] ?? ''));
        if ($body !== '') {
            $pdo->prepare('INSERT INTO web_ticket_reply (ticket_id, author, is_staff, body) VALUES (:id, :a, TRUE, :b)')
                ->execute(['id' => $ticketId, 'a' => $user['str_account_id'], 'b' => $body]);
            $pdo->prepare('UPDATE web_ticket SET updated_at = now() WHERE id = :id')->execute(['id' => $ticketId]);
        }
    } elseif ($action === 'close') {
        $pdo->prepare("UPDATE web_ticket SET status = 'closed' WHERE id = :id")->execute(['id' => $ticketId]);
    } elseif ($action === 'reopen') {
        $pdo->prepare("UPDATE web_ticket SET status = 'open' WHERE id = :id")->execute(['id' => $ticketId]);
    }
    jstko_redirect('/admin/tickets.php?id=' . $ticketId);
}

$pageTitle = 'Destek Talepleri';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';

if ($id > 0) {
    $stmt = $pdo->prepare('SELECT * FROM web_ticket WHERE id = :id');
    $stmt->execute(['id' => $id]);
    $ticket = $stmt->fetch();
    if (!$ticket) {
        echo '<section class="max-w-3xl mx-auto px-4 py-16 text-center text-slate-500">Talep bulunamadı.</section>';
        require __DIR__ . '/../partials/footer.php';
        exit;
    }
    $replies = $pdo->prepare('SELECT * FROM web_ticket_reply WHERE ticket_id = :id ORDER BY created_at');
    $replies->execute(['id' => $id]);
    $replies = $replies->fetchAll();
    ?>
    <section class="max-w-3xl mx-auto px-4 py-8">
      <a href="/admin/tickets.php" class="text-sm text-slate-400 hover:text-gold-400">&larr; Tüm Talepler</a>
      <div class="flex items-center justify-between mt-2 mb-6">
        <h1 class="text-2xl font-bold text-white"><?= jstko_e($ticket['subject']) ?></h1>
        <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="ticket_id" value="<?= $id ?>">
          <input type="hidden" name="form_action" value="<?= $ticket['status'] === 'open' ? 'close' : 'reopen' ?>">
          <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= $ticket['status'] === 'open' ? 'Kapat' : 'Yeniden Aç' ?></button>
        </form>
      </div>
      <div class="text-sm text-slate-400 mb-4">Hesap: <span class="text-white font-semibold"><?= jstko_e($ticket['account_id']) ?></span></div>

      <div class="space-y-4 mb-8">
        <div class="card p-4">
          <div class="text-xs text-slate-500 mb-2"><?= jstko_e($ticket['account_id']) ?> · <?= jstko_time_ago($ticket['created_at']) ?></div>
          <div class="text-sm text-slate-200 whitespace-pre-wrap"><?= jstko_e($ticket['body']) ?></div>
        </div>
        <?php foreach ($replies as $r): ?>
          <div class="card p-4 <?= jstko_pg_bool($r['is_staff']) ? 'border-gold-500/40 bg-gold-500/5' : '' ?>">
            <div class="text-xs mb-2 <?= jstko_pg_bool($r['is_staff']) ? 'text-gold-400 font-semibold' : 'text-slate-500' ?>"><?= jstko_e($r['author']) ?> · <?= jstko_time_ago($r['created_at']) ?></div>
            <div class="text-sm text-slate-200 whitespace-pre-wrap"><?= jstko_e($r['body']) ?></div>
          </div>
        <?php endforeach; ?>
      </div>

      <div class="card p-6">
        <form method="post" class="space-y-3">
          <?= jstko_csrf_field() ?>
          <input type="hidden" name="ticket_id" value="<?= $id ?>">
          <input type="hidden" name="form_action" value="reply">
          <textarea class="input" name="body" rows="3" placeholder="Yanıtınız..." required></textarea>
          <button type="submit" class="btn-primary">Yanıtla</button>
        </form>
      </div>
    </section>
    <?php
} else {
    $tickets = $pdo->query('SELECT * FROM web_ticket ORDER BY (status=\'open\') DESC, updated_at DESC LIMIT 200')->fetchAll();
    ?>
    <section class="max-w-4xl mx-auto px-4 py-8">
      <h1 class="text-2xl font-bold text-white mb-6">Destek Talepleri</h1>
      <div class="space-y-2">
        <?php foreach ($tickets as $t): ?>
          <a href="?id=<?= (int)$t['id'] ?>" class="card p-4 flex items-center justify-between hover:border-gold-400 transition">
            <div>
              <div class="font-medium text-white"><?= jstko_e($t['subject']) ?></div>
              <div class="text-xs text-slate-500 mt-1"><?= jstko_e($t['account_id']) ?> · <?= jstko_time_ago($t['created_at']) ?></div>
            </div>
            <span class="text-xs px-2 py-1 rounded-full <?= $t['status'] === 'open' ? 'bg-amber-900/30 text-amber-300 border border-amber-700' : 'bg-slate-800 text-slate-400 border border-slate-700' ?>">
              <?= $t['status'] === 'open' ? 'Açık' : 'Kapalı' ?>
            </span>
          </a>
        <?php endforeach; ?>
        <?php if (!$tickets): ?><div class="card p-10 text-center text-slate-500">Destek talebi yok.</div><?php endif; ?>
      </div>
    </section>
    <?php
}
require __DIR__ . '/../partials/footer.php';
