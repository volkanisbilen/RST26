<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Çark Ödülleri';
require __DIR__ . '/partials/header.php';
$pdo = jstko_db();

$rows = $pdo->query(
    "SELECT r.roulette_type, r.slot, r.item_count, r.weight, i.str_name, i.item_icon_id1
     FROM native_roulette_reward r LEFT JOIN item i ON i.num = r.item_id
     ORDER BY r.roulette_type, r.slot"
)->fetchAll();

$byType = [];
foreach ($rows as $r) $byType[(int)$r['roulette_type']][] = $r;
ksort($byType);
?>
<section class="max-w-4xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Rulet / Marble Çarkı</h1>
  <p class="text-sm text-slate-400 mb-6">Çark ödülleri ve çıkma ağırlıkları (ağırlık yüksek olan daha sık çıkar).</p>

  <?php foreach ($byType as $type => $slots): ?>
    <h2 class="text-lg font-semibold text-white mb-3 mt-8 first:mt-0">Çark <?= (int)$type ?></h2>
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
      <?php foreach ($slots as $s): ?>
      <div class="card p-3 flex items-center gap-3">
        <img src="<?= jstko_e(jstko_item_icon_url($s['item_icon_id1'] ?: null)) ?>" alt="" class="w-10 h-10 rounded border border-ink-500 bg-ink-700 object-contain shrink-0">
        <div class="min-w-0">
          <div class="text-sm font-medium text-white truncate"><?= jstko_e($s['str_name'] ?: '?') ?></div>
          <div class="text-xs text-gold-400">&times;<?= (int)$s['item_count'] ?: 1 ?> &middot; ağırlık <?= (int)$s['weight'] ?></div>
        </div>
      </div>
      <?php endforeach; ?>
    </div>
  <?php endforeach; ?>
  <?php if (!$byType): ?><p class="text-sm text-slate-500">Bu etkinlik için ödül tanımlı değil.</p><?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
