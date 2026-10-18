<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Lottery';
require __DIR__ . '/partials/header.php';
$pdo = jstko_db();

$rows = $pdo->query("SELECT * FROM lottery_event_settings ORDER BY lnum")->fetchAll();

$ids = [];
foreach ($rows as $r) {
    foreach (['req_item1', 'req_item2', 'req_item3', 'req_item4', 'req_item5', 'reward_item1', 'reward_item2', 'reward_item3', 'reward_item4'] as $col) {
        if (!empty($r[$col])) $ids[] = (int)$r[$col];
    }
}
$items = [];
if ($ids) {
    $ids = array_values(array_unique($ids));
    $placeholders = implode(',', array_fill(0, count($ids), '?'));
    $stmt = $pdo->prepare("SELECT num, str_name, item_icon_id1 FROM item WHERE num IN ($placeholders)");
    $stmt->execute($ids);
    foreach ($stmt->fetchAll() as $it) $items[(int)$it['num']] = $it;
}

function jstko_lottery_item_chip(array $items, ?int $id): string
{
    if (!$id || !isset($items[$id])) return '';
    $it = $items[$id];
    return '<div class="flex items-center gap-2 bg-ink-700 border border-ink-500 rounded-lg px-2.5 py-1.5">'
        . '<img src="' . jstko_e(jstko_item_icon_url($it['item_icon_id1'] ?: null)) . '" class="w-8 h-8 rounded border border-ink-500 bg-ink-800 object-contain">'
        . '<span class="text-xs text-white">' . jstko_e($it['str_name']) . '</span></div>';
}
?>
<section class="max-w-4xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Lottery</h1>
  <p class="text-sm text-slate-400 mb-6">Çekilişe katılmak için gereken eşyalar ve kazanılabilecek ödüller.</p>

  <div class="grid gap-4">
    <?php foreach ($rows as $r): ?>
    <div class="card p-5">
      <div class="flex items-center justify-between flex-wrap gap-2 mb-3">
        <h2 class="text-lg font-semibold text-white">Lottery #<?= (int)$r['lnum'] ?></h2>
        <div class="flex gap-3 text-xs text-slate-400">
          <span>Kişi sınırı: <?= jstko_number((int)$r['user_limit']) ?></span>
          <span>Süre: <?= jstko_number((int)$r['event_time']) ?> dk</span>
        </div>
      </div>
      <p class="text-xs text-slate-500 mb-1.5">Gereken eşyalar</p>
      <div class="flex flex-wrap gap-2 mb-3">
        <?php $any = false; foreach (['req_item1', 'req_item2', 'req_item3', 'req_item4', 'req_item5'] as $col):
            $chip = jstko_lottery_item_chip($items, $r[$col] ?: null);
            if ($chip) { $any = true; echo $chip; }
        endforeach; if (!$any): ?><span class="text-xs text-slate-600">-</span><?php endif; ?>
      </div>
      <p class="text-xs text-slate-500 mb-1.5">Ödüller (biri rastgele kazanılır)</p>
      <div class="flex flex-wrap gap-2">
        <?php $any = false; foreach (['reward_item1', 'reward_item2', 'reward_item3', 'reward_item4'] as $col):
            $chip = jstko_lottery_item_chip($items, $r[$col] ?: null);
            if ($chip) { $any = true; echo $chip; }
        endforeach; if (!$any): ?><span class="text-xs text-slate-600">-</span><?php endif; ?>
      </div>
    </div>
    <?php endforeach; ?>
    <?php if (!$rows): ?><p class="text-sm text-slate-500">Tanımlı lottery yok.</p><?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
