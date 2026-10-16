<?php
require_once __DIR__ . '/../../includes/bootstrap.php';
$user = jstko_require_gm();

$pdo = jstko_db();
$stats = [
    'accounts' => $pdo->query('SELECT count(*) FROM tb_user')->fetchColumn(),
    'online' => $pdo->query('SELECT count(*) FROM currentuser')->fetchColumn(),
    'pending_email' => $pdo->query('SELECT count(*) FROM tb_user WHERE account_check = 0')->fetchColumn(),
    'open_tickets' => $pdo->query("SELECT count(*) FROM web_ticket WHERE status = 'open'")->fetchColumn(),
];

$pageTitle = 'Admin Paneli';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-5xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-6">Admin Paneli</h1>
  <div class="grid sm:grid-cols-4 gap-4">
    <div class="card p-5 text-center"><div class="text-2xl font-extrabold text-white"><?= jstko_number($stats['accounts']) ?></div><div class="text-xs text-slate-400 mt-1">Hesap</div></div>
    <div class="card p-5 text-center"><div class="text-2xl font-extrabold text-emerald-400"><?= jstko_number($stats['online']) ?></div><div class="text-xs text-slate-400 mt-1">Online</div></div>
    <div class="card p-5 text-center"><div class="text-2xl font-extrabold text-amber-400"><?= jstko_number($stats['pending_email']) ?></div><div class="text-xs text-slate-400 mt-1">Bekleyen Aktivasyon</div></div>
    <div class="card p-5 text-center"><div class="text-2xl font-extrabold text-red-400"><?= jstko_number($stats['open_tickets']) ?></div><div class="text-xs text-slate-400 mt-1">Açık Destek Talebi</div></div>
  </div>
</section>
<?php require __DIR__ . '/../partials/footer.php'; ?>
