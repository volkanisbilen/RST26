<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Upgrade Oranları';
require __DIR__ . '/partials/header.php';

// Success rates of the anvil, read from the same table the game server uses (item_upgrade_settings, rate out of 10000).
// Only the rates are shown, not the scrolls / gold. "Trinalı" = the Trina's Piece (Karivdis for rebirth) is put in too.
$rates = []; // [item_type][grade][scroll][mode] => best rate
try {
    $rows = jstko_db()->query(
        "SELECT item_type, item_grade, req_item_id1 a, req_item_id2 b, success_rate FROM item_upgrade_settings WHERE item_grade < 99"
    )->fetchAll();
} catch (Throwable $e) {
    $rows = [];
}
$scrolls = [379221000, 379205000, 379021000, 379159000, 379257000];
$trinas = [353000000, 352900000, 700002000, 354000000, 379258000];
foreach ($rows as $r) {
    $a = (int)$r['a']; $b = (int)$r['b']; $rate = (int)$r['success_rate']; $t = (int)$r['item_type']; $g = (int)$r['item_grade'];
    if (in_array($a, $scrolls, true) && $b === 0) { $sc = $a; $mode = 'plain'; }
    elseif (in_array($a, $scrolls, true) && in_array($b, $trinas, true)) { $sc = $a; $mode = 'trina'; }
    elseif (in_array($b, $scrolls, true) && in_array($a, $trinas, true)) { $sc = $b; $mode = 'trina'; }
    else continue;
    $cur = $rates[$t][$g][$sc][$mode] ?? -1;
    if ($rate > $cur) $rates[$t][$g][$sc][$mode] = $rate;
}

function jstko_rate_cell(?int $r): string
{
    if ($r === null) return '<span class="rate-none">—</span>';
    $p = $r / 100;
    $txt = '%' . (abs($p - round($p)) < 0.005 ? (string)(int)round($p) : number_format($p, 2, ',', ''));
    $cls = $p >= 90 ? 'rate-good' : ($p >= 25 ? 'rate-mid' : ($p > 0 ? 'rate-low' : 'rate-none'));
    return '<span class="' . $cls . ' font-bold">' . $txt . '</span>';
}

// [title, note, scroll id, [grades], column set: [[label, item_type, mode], ...], arrow prefix]
$sections = [
    ['Low Class İtemler', 'Upgrade Scroll (Low Class) · Trina\'s Piece (Low Class)', 379221000, range(1, 9),
        [['Unique Trinasız', 4, 'plain'], ['Unique Trinalı', 4, 'trina'], ['Rare Trinasız', 5, 'plain'], ['Rare Trinalı', 5, 'trina']], ''],
    ['Middle Class İtemler', 'Upgrade Scroll (Middle Class) · Trina\'s Piece (Middle Class)', 379205000, range(1, 9),
        [['Unique Trinasız', 4, 'plain'], ['Unique Trinalı', 4, 'trina'], ['Rare Trinasız', 5, 'plain'], ['Rare Trinalı', 5, 'trina']], ''],
    ['High Class İtemler', 'Blessed Upgrade Scroll · Trina\'s Piece', 379021000, range(1, 10),
        [['Unique Trinasız', 4, 'plain'], ['Unique Trinalı', 4, 'trina'], ['Rare Trinasız', 5, 'plain'], ['Rare Trinalı', 5, 'trina']], ''],
    ['Takılar', 'Accessory Compound Scroll · Accessory Trina\'s Piece', 379159000, range(0, 7),
        [['Unique Trinasız', 4, 'plain'], ['Unique Trinalı', 4, 'trina'], ['Rare Trinasız', 5, 'plain'], ['Rare Trinalı', 5, 'trina']], ''],
    ['Rebirth İtemler', 'Reverse Item Strengthen Scroll · Tears of Karivdis', 379257000, range(1, 12),
        [['Karivdis\'siz', 11, 'plain'], ['Karivdis ile', 11, 'trina'], ['Unique Karivdis\'siz', 12, 'plain'], ['Unique Karivdis ile', 12, 'trina']], 'Reb '],
];
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1>Upgrade Oranları</h1>
  <p class="text-sm text-slate-400 mb-8">Eşyanı anvile koyduğunda geçerli başarı oranları. "Trinalı" sütunları Trina's Piece (rebirthte Tears of Karivdis) kullanıldığında geçerlidir. Başarısız olursa item yanar.</p>

  <?php foreach ($sections as [$title, $note, $scroll, $grades, $cols, $pre]):
      // keep only the grades that have at least one configured rate
      $shown = array_values(array_filter($grades, function ($g) use ($rates, $cols, $scroll) {
          foreach ($cols as [$l, $t, $m]) if (isset($rates[$t][$g][$scroll][$m])) return true;
          return false;
      }));
      if (!$shown) continue;
  ?>
    <div class="mb-10">
      <div class="flex flex-wrap items-baseline gap-3 mb-3">
        <h2 class="font-display text-xl font-bold text-gold-400"><?= jstko_e($title) ?></h2>
        <span class="text-xs text-slate-500"><?= jstko_e($note) ?></span>
      </div>
      <div class="card overflow-x-auto">
        <table class="w-full text-sm">
          <thead class="text-left text-xs uppercase">
            <tr><th class="py-3 px-4">Basma</th><?php foreach ($cols as [$l]): ?><th class="py-3 px-4 text-center"><?= jstko_e($l) ?></th><?php endforeach; ?></tr>
          </thead>
          <tbody>
            <?php foreach ($shown as $g): ?>
              <tr class="border-t border-gold-600/10">
                <td class="py-2.5 px-4 font-semibold text-white whitespace-nowrap"><?= jstko_e($pre) ?>+<?= $g ?> → +<?= $g + 1 ?></td>
                <?php foreach ($cols as [$l, $t, $m]): ?>
                  <td class="py-2.5 px-4 text-center"><?= jstko_rate_cell($rates[$t][$g][$scroll][$m] ?? null) ?></td>
                <?php endforeach; ?>
              </tr>
            <?php endforeach; ?>
          </tbody>
        </table>
      </div>
    </div>
  <?php endforeach; ?>

  <?php if (!$rates): ?><div class="card p-10 text-center text-slate-500">Oranlar şu anda okunamadı.</div><?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
