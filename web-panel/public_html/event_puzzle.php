<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Puzzle Etkinliği';
require __DIR__ . '/partials/header.php';
$pdo = jstko_db();

$rows = $pdo->query(
    "SELECT r.reward_index, r.required_total, r.item_count, i.str_name, i.item_icon_id1
     FROM native_jigsaw_reward r LEFT JOIN item i ON i.num = r.item_id
     ORDER BY r.required_total"
)->fetchAll();
?>
<section class="max-w-4xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Puzzle Etkinliği</h1>
  <p class="text-sm text-slate-400 mb-6">Yapboz parçaları toplayarak sırayla açılan ödüller.</p>

  <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
    <?php foreach ($rows as $r): ?>
    <div class="card p-3 flex items-center gap-3">
      <img src="<?= jstko_e(jstko_item_icon_url($r['item_icon_id1'] ?: null)) ?>" alt="" class="w-10 h-10 rounded border border-ink-500 bg-ink-700 object-contain shrink-0">
      <div class="min-w-0">
        <div class="text-sm font-medium text-white truncate"><?= jstko_e($r['str_name'] ?: '?') ?></div>
        <div class="text-xs text-gold-400">&times;<?= (int)$r['item_count'] ?: 1 ?> &middot; <?= jstko_number((int)$r['required_total']) ?> parça</div>
      </div>
    </div>
    <?php endforeach; ?>
    <?php if (!$rows): ?><p class="text-sm text-slate-500 col-span-full">Bu etkinlik için ödül tanımlı değil.</p><?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
