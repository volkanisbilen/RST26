<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Etkinlik Takvimi';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

// Same source the admin panel's own /#/events page reads: event_auto_schedule (when/how
// often) joined to admin_event_state (display name/group, published live by the game server).
$rows = $pdo->query(
    "SELECT s.event_key, s.enabled, s.days, s.hour, s.minute, s.arg, s.note,
            COALESCE(NULLIF(e.name, ''), s.event_key) AS name, e.grp, e.arg_label
     FROM event_auto_schedule s
     LEFT JOIN admin_event_state e ON e.event_key = s.event_key
     WHERE s.enabled = true
     ORDER BY s.hour, s.minute"
)->fetchAll();

function jstko_days_text(int $mask): string
{
    if ($mask === 127) return 'Her gün';
    $labels = ['Paz', 'Pzt', 'Sal', 'Çar', 'Per', 'Cum', 'Cmt'];
    $out = [];
    foreach ($labels as $i => $l) {
        if ($mask & (1 << $i)) $out[] = $l;
    }
    return $out ? implode(' ', $out) : '-';
}

$groups = [];
foreach ($rows as $r) {
    $groups[$r['name']][] = $r;
}
ksort($groups, SORT_NATURAL | SORT_FLAG_CASE);
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Etkinlik Takvimi</h1>
  <p class="text-sm text-slate-400 mb-6">Sunucuda otomatik açılan etkinlikler ve saatleri.</p>

  <div class="card overflow-x-auto">
    <table class="w-full text-sm">
      <thead class="text-left text-slate-500 text-xs uppercase bg-ink-600/50">
        <tr>
          <th class="py-3 px-4">Saat</th>
          <th class="py-3 px-4">Etkinlik</th>
          <th class="py-3 px-4">Günler</th>
          <th class="py-3 px-4">Argüman</th>
        </tr>
      </thead>
      <tbody>
        <?php foreach ($groups as $name => $sessions): foreach ($sessions as $i => $r): ?>
        <tr class="table-row border-t border-ink-500/50">
          <td class="py-2.5 px-4 text-gold-400 font-semibold whitespace-nowrap"><?= sprintf('%02d:%02d', (int)$r['hour'], (int)$r['minute']) ?></td>
          <td class="py-2.5 px-4">
            <div class="font-medium text-white"><?= jstko_e($name) ?></div>
            <?php if (!empty($r['note'])): ?><div class="text-xs text-slate-500"><?= jstko_e($r['note']) ?></div><?php endif; ?>
          </td>
          <td class="py-2.5 px-4 text-slate-300"><?= jstko_e(jstko_days_text((int)$r['days'])) ?></td>
          <td class="py-2.5 px-4 text-slate-400"><?= $r['arg'] !== '' ? jstko_e(($r['arg_label'] ?: '') . ($r['arg_label'] ? ': ' : '') . $r['arg']) : '-' ?></td>
        </tr>
        <?php endforeach; endforeach; ?>
      </tbody>
    </table>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
