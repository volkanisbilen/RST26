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

// Collection Race has its own schedule table (auto_start/auto_hour/auto_minute, daily
// repeat) separate from event_schedule_main_list.
$races = $pdo->query(
    "SELECT event_name, min_level, max_level, auto_start, auto_hour, auto_minute, is_repeat
     FROM collection_race_settings ORDER BY event_name"
)->fetchAll();

// These events have no persisted automatic schedule in this server (manes_survival.rs,
// rift_entry.rs, ultima.rs, golem_war.rs) -- a GM opens them by hand whenever. Golem War's
// "auto" toggle (daily 14:00/22:00) exists but lives only in server memory and resets to
// off on every restart, so it isn't something this page can show reliably either. Listed
// here so the page covers every event, without claiming a fixed time that may not hold.
$manualEvents = ['Manes Survival', 'Full Moon Rift', "Ultima'nın Zindanı", 'Golem Savaşı'];
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

        <?php foreach ($races as $r):
            $active = jstko_pg_bool($r['auto_start']);
            $repeat = jstko_pg_bool($r['is_repeat']);
            $minLv = (int)$r['min_level'];
            $maxLv = (int)$r['max_level'];
        ?>
        <tr class="table-row border-t border-ink-500/50 <?= $active ? '' : 'opacity-40' ?>">
          <td class="py-2.5 px-4 font-medium text-white"><?= jstko_e($r['event_name']) ?> <span class="text-xs text-slate-500">(Collection Race)</span></td>
          <td class="py-2.5 px-4">
            <span class="text-[10px] px-1.5 py-0.5 rounded bg-gold-500/20 text-gold-400 border border-gold-500/40"><?= $repeat ? 'Her gün' : 'Tek seferlik' ?></span>
          </td>
          <td class="py-2.5 px-4 text-slate-300"><?= sprintf('%02d:%02d', (int)$r['auto_hour'], (int)$r['auto_minute']) ?></td>
          <td class="py-2.5 px-4 text-slate-400"><?= ($minLv > 0 || $maxLv > 0) ? jstko_e(($minLv > 0 ? $minLv : '1') . '-' . ($maxLv > 0 ? $maxLv : '83')) : '-' ?></td>
          <td class="py-2.5 px-4">
            <span class="text-xs px-2 py-1 rounded <?= $active ? 'bg-emerald-900/40 text-emerald-300 border border-emerald-700' : 'bg-ink-700 text-slate-500 border border-ink-500' ?>"><?= $active ? 'Aktif' : 'Kapalı' ?></span>
          </td>
        </tr>
        <?php endforeach; ?>
      </tbody>
    </table>
  </div>

  <h2 class="text-lg font-semibold text-white mt-10 mb-3">GM Tarafından Manuel Açılan Etkinlikler</h2>
  <p class="text-sm text-slate-400 mb-4">Bunların sabit bir takvimi yok; bir GM uygun gördüğünde başlatır.</p>
  <div class="card overflow-x-auto">
    <table class="w-full text-sm">
      <tbody>
        <?php foreach ($manualEvents as $name): ?>
        <tr class="table-row border-t border-ink-500/50 first:border-t-0">
          <td class="py-2.5 px-4 font-medium text-white"><?= jstko_e($name) ?></td>
          <td class="py-2.5 px-4 text-slate-500 text-right">Manuel</td>
        </tr>
        <?php endforeach; ?>
      </tbody>
    </table>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
