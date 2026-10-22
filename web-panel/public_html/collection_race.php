<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Collection Race';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

$events = $pdo->query(
    "SELECT s.*, z.zone_name FROM collection_race_settings s
     LEFT JOIN zone_info z ON z.zone_no = s.event_zone
     ORDER BY s.auto_hour, s.auto_minute, s.event_index"
)->fetchAll();

$rewards = [];
foreach ($pdo->query("SELECT * FROM collection_race_reward ORDER BY event_id, idx")->fetchAll() as $r) $rewards[(int)$r['event_id']][] = $r;

$mobIds = [];
$itemIds = [];
foreach ($events as $e) for ($i = 1; $i <= 3; $i++) { $id = (int)$e["unit$i"]; if ($id > 0 && $id !== 32767) $mobIds[$id] = true; }
foreach ($rewards as $list) foreach ($list as $r) if ((int)$r['item_id'] > 0) $itemIds[(int)$r['item_id']] = true;
$mobs = [];
if ($mobIds) {
    $stmt = $pdo->query("SELECT s_sid, str_name FROM npc_template WHERE is_monster = true AND s_sid IN (" . implode(',', array_keys($mobIds)) . ")");
    foreach ($stmt->fetchAll() as $r) $mobs[(int)$r['s_sid']] = $r['str_name'];
}
$items = [];
if ($itemIds) {
    $stmt = $pdo->query("SELECT num, str_name, item_icon_id1 FROM item WHERE num IN (" . implode(',', array_keys($itemIds)) . ")");
    foreach ($stmt->fetchAll() as $r) $items[(int)$r['num']] = $r;
}

$now = (int)date('G') * 60 + (int)date('i');
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Collection Race</h1>
  <p class="text-sm text-slate-400 mb-6">Her gün belirli saatlerde otomatik başlayan toplama yarışı. Hedef yaratıkları öldürüp parçaları toplayan ödülü alır. Saatler sunucu saatidir.</p>

  <div class="space-y-4">
    <?php foreach ($events as $e):
        $at = sprintf('%02d:%02d', (int)$e['auto_hour'], (int)$e['auto_minute']);
        $start = (int)$e['auto_hour'] * 60 + (int)$e['auto_minute'];
        $running = $e['auto_start'] && $now >= $start && $now < $start + (int)$e['event_time'];
        $targets = [];
        for ($i = 1; $i <= 3; $i++) {
            $id = (int)$e["unit$i"]; $c = (int)$e["unit_count$i"];
            if ($id <= 0 || $c <= 0) continue;
            $targets[] = ($id === 32767 ? 'Herhangi bir yaratık' : ($mobs[$id] ?? ('#' . $id))) . ' ×' . $c;
        }
    ?>
    <div class="card p-4 <?= $running ? 'border border-gold-400' : '' ?>">
      <div class="flex flex-wrap items-center gap-2 mb-3">
        <span class="text-lg font-bold text-gold-400"><?= $at ?></span>
        <span class="text-base font-semibold text-white"><?= jstko_e($e['event_name']) ?></span>
        <?php if ($running): ?><span class="text-xs px-2 py-0.5 rounded bg-gold-500 text-ink-900 font-semibold">Şu an devam ediyor</span><?php endif; ?>
        <?php if (!$e['auto_start']): ?><span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-slate-400">Otomatik başlamıyor</span><?php endif; ?>
        <span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-slate-300"><?= jstko_e($e['zone_name'] ?: ('Bölge ' . (int)$e['event_zone'])) ?></span>
        <span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-slate-300">Lv<?= (int)$e['min_level'] ?>–<?= (int)$e['max_level'] ?></span>
        <span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-slate-300"><?= (int)$e['event_time'] ?> dk</span>
      </div>
      <?php if ($targets): ?>
        <div class="text-xs text-slate-500 mb-1">Toplanacaklar</div>
        <div class="flex flex-wrap gap-1.5 mb-3">
          <?php foreach ($targets as $t): ?><span class="text-xs px-2 py-1 rounded bg-ink-700 border border-ink-500 text-white"><?= jstko_e($t) ?></span><?php endforeach; ?>
        </div>
      <?php endif; ?>
      <?php if (!empty($rewards[(int)$e['event_index']])): ?>
        <div class="text-xs text-slate-500 mb-1">Ödüller</div>
        <div class="flex flex-wrap gap-2">
          <?php foreach ($rewards[(int)$e['event_index']] as $r): $it = $items[(int)$r['item_id']] ?? null; if (!$it) continue; ?>
            <div data-item="<?= (int)$r['item_id'] ?>" class="item-card !p-1.5 !gap-2">
              <img src="<?= jstko_e(jstko_item_icon_url(($it['item_icon_id1'] ?? null) ?: null)) ?>" alt="" class="item-ico !w-8 !h-8">
              <span class="text-sm text-white"><?= jstko_e($it['str_name']) ?> <span class="text-gold-400 font-semibold">×<?= (int)$r['item_count'] ?></span></span>
              <?php if ((int)$r['rate'] < 100): ?><span class="text-xs text-slate-400">%<?= (int)$r['rate'] ?></span><?php endif; ?>
            </div>
          <?php endforeach; ?>
        </div>
      <?php endif; ?>
    </div>
    <?php endforeach; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
