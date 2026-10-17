<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Etkinlikler';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

$mains = $pdo->query(
    "SELECT eventid, event_type, zoneid, name, status, hour1, minute1, hour2, minute2,
            hour3, minute3, hour4, minute4, hour5, minute5, min_level, max_level
     FROM event_schedule_main_list ORDER BY name"
)->fetchAll();

$days = [];
foreach ($pdo->query(
    "SELECT eventid, sunday, monday, tuesday, wednesday, thursday, friday, saturday
     FROM event_schedule_day_list"
)->fetchAll() as $d) {
    $days[(int)$d['eventid']] = $d;
}

$dayLabels = ['monday' => 'Pzt', 'tuesday' => 'Sal', 'wednesday' => 'Çar', 'thursday' => 'Per',
    'friday' => 'Cum', 'saturday' => 'Cmt', 'sunday' => 'Paz'];

function jstko_event_times(array $e): array
{
    $out = [];
    for ($i = 1; $i <= 5; $i++) {
        $h = (int)$e["hour{$i}"];
        $m = (int)$e["minute{$i}"];
        if ($h < 0 || $m < 0) continue;
        $out[] = sprintf('%02d:%02d', $h, $m);
    }
    return $out;
}
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Etkinlikler</h1>
  <p class="text-sm text-slate-400 mb-6">Sunucudaki tüm etkinlikler, hangi günler ve saatlerde açıldıkları.</p>

  <div class="card overflow-x-auto">
    <table class="w-full text-sm">
      <thead class="text-left text-slate-500 text-xs uppercase bg-ink-600/50">
        <tr>
          <th class="py-3 px-4">Etkinlik</th>
          <th class="py-3 px-4">Günler</th>
          <th class="py-3 px-4">Saatler</th>
          <th class="py-3 px-4">Seviye</th>
          <th class="py-3 px-4">Durum</th>
        </tr>
      </thead>
      <tbody>
        <?php foreach ($mains as $e):
            $active = jstko_pg_bool($e['status']);
            $d = $days[(int)$e['eventid']] ?? null;
            $times = jstko_event_times($e);
            $minLv = (int)$e['min_level'];
            $maxLv = (int)$e['max_level'];
        ?>
        <tr class="table-row border-t border-ink-500/50 <?= $active ? '' : 'opacity-40' ?>">
          <td class="py-2.5 px-4 font-medium text-white"><?= jstko_e($e['name']) ?></td>
          <td class="py-2.5 px-4">
            <?php if (!$d): ?>
              <span class="text-slate-500">-</span>
            <?php else: ?>
              <div class="flex flex-wrap gap-1">
                <?php foreach ($dayLabels as $col => $label):
                    $on = jstko_pg_bool($d[$col]);
                ?>
                  <span class="text-[10px] px-1.5 py-0.5 rounded <?= $on ? 'bg-gold-500/20 text-gold-400 border border-gold-500/40' : 'bg-ink-700 text-slate-600 border border-ink-500' ?>"><?= $label ?></span>
                <?php endforeach; ?>
              </div>
            <?php endif; ?>
          </td>
          <td class="py-2.5 px-4 text-slate-300"><?= $times ? jstko_e(implode(' · ', $times)) : '-' ?></td>
          <td class="py-2.5 px-4 text-slate-400"><?= ($minLv > 0 || $maxLv > 0) ? jstko_e(($minLv > 0 ? $minLv : '1') . '-' . ($maxLv > 0 ? $maxLv : '83')) : '-' ?></td>
          <td class="py-2.5 px-4">
            <span class="text-xs px-2 py-1 rounded <?= $active ? 'bg-emerald-900/40 text-emerald-300 border border-emerald-700' : 'bg-ink-700 text-slate-500 border border-ink-500' ?>"><?= $active ? 'Aktif' : 'Kapalı' ?></span>
          </td>
        </tr>
        <?php endforeach; ?>
      </tbody>
    </table>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
