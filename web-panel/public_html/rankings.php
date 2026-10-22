<?php
require_once __DIR__ . '/includes/bootstrap.php';
require_once __DIR__ . '/partials/podium.php';
$pageTitle = 'Sıralamalar';
require __DIR__ . '/partials/header.php';

$tab = $_GET['tab'] ?? 'genel';
$allowed = ['genel', 'aylik', 'karus', 'elmorad', 'clan'];
if (!in_array($tab, $allowed, true)) $tab = 'genel';

$pdo = jstko_db();

if ($tab === 'clan') {
    $rows = $pdo->query(
        "SELECT id_name name, nation, members, points
         FROM knights ORDER BY points DESC LIMIT 100"
    )->fetchAll();
} else {
    $order = $tab === 'aylik' ? 'u.loyalty_monthly' : 'u.loyalty';
    $where = match ($tab) {
        'karus' => 'WHERE u.nation = 1',
        'elmorad' => 'WHERE u.nation = 2',
        default => '',
    };
    $rows = $pdo->query(
        "SELECT u.str_user_id name, u.nation, u.class, u.level, u.rebirth_level, u.loyalty, u.loyalty_monthly, k.id_name clan,
                EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) online
         FROM userdata u LEFT JOIN knights k ON k.id_num = u.knights
         $where
         ORDER BY $order DESC LIMIT 100"
    )->fetchAll();
}

$tabs = [
    'genel' => 'Genel Sıralama', 'aylik' => 'Aylık Sıralama', 'karus' => 'Karus',
    'elmorad' => 'El Morad', 'clan' => 'Klan Sıralaması',
];

// podium rows (same shape for players and clans)
$pod = [];
foreach (array_slice($rows, 0, 3) as $r) {
    if ($tab === 'clan') {
        $pod[] = ['name' => $r['name'], 'sub' => (int)$r['members'] . ' üye', 'nation' => $r['nation'], 'lvl' => null, 'value' => jstko_number($r['points']), 'unit' => 'Puan', 'online' => false];
    } else {
        $pod[] = ['name' => $r['name'], 'sub' => $r['clan'] ?: 'Klan Yok', 'nation' => $r['nation'], 'lvl' => (int)$r['level'],
                  'value' => jstko_number($tab === 'aylik' ? $r['loyalty_monthly'] : $r['loyalty']), 'unit' => 'NP', 'online' => jstko_pg_bool($r['online'])];
    }
}
$rest = array_slice($rows, 3);
$maxVal = 1;
if ($rows) $maxVal = max(1, (int)($tab === 'clan' ? $rows[0]['points'] : ($tab === 'aylik' ? $rows[0]['loyalty_monthly'] : $rows[0]['loyalty'])));
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1>Sıralamalar</h1>
  <p class="text-sm text-slate-400 mb-6">İlk 100 <?= $tab === 'clan' ? 'klan' : 'karakter' ?>.</p>

  <div class="flex flex-wrap gap-2 mb-6">
    <?php foreach ($tabs as $key => $label): ?>
      <a href="?tab=<?= $key ?>" class="tab-pill <?= $tab === $key ? 'on' : '' ?>"><?= jstko_e($label) ?></a>
    <?php endforeach; ?>
  </div>

  <?php if (!$rows): ?>
    <div class="card p-10 text-center text-slate-500">Veri yok.</div>
  <?php else: ?>
  <div class="panel overflow-hidden">
    <?= jstko_podium_html($pod) ?>
    <ul class="rank-list">
      <?php foreach ($rest as $i => $r): $pos = $i + 4;
          $val = $tab === 'clan' ? (int)$r['points'] : (int)($tab === 'aylik' ? $r['loyalty_monthly'] : $r['loyalty']);
          $pct = max(3, (int)round($val / $maxVal * 100));
          $nat = (int)$r['nation'] === 1 ? 'karus' : 'elmo'; ?>
        <li class="rank-row rank-row-wide" style="--d:<?= min($i, 14) ?>">
          <span class="rank-pos"><?= $pos ?></span>
          <span class="rank-nation <?= $nat ?>" title="<?= $nat === 'karus' ? 'Karus' : 'El Morad' ?>"></span>
          <span class="rank-who">
            <b><?= jstko_e($r['name']) ?><?php if ($tab !== 'clan' && jstko_pg_bool($r['online'])): ?><span class="dot-on" title="Online"></span><?php endif; ?></b>
            <em><?= $tab === 'clan' ? (int)$r['members'] . ' üye' : jstko_e($r['clan'] ?: 'Klan Yok') ?></em>
          </span>
          <?php if ($tab !== 'clan'): ?>
            <span class="cls-chip hidden sm:inline-block"><?= jstko_e(jstko_class_name((int)$r['class'])) ?></span>
            <span class="chip-lvl">Lv <?= (int)$r['level'] ?><?= $r['rebirth_level'] ? ' / ' . (int)$r['rebirth_level'] : '' ?></span>
          <?php else: ?>
            <span class="chip-nation <?= $nat ?>"><?= $nat === 'karus' ? 'Karus' : 'El Morad' ?></span><span></span>
          <?php endif; ?>
          <span class="rank-np"><b><?= jstko_number($val) ?></b><i><u style="width:<?= $pct ?>%"></u></i></span>
        </li>
      <?php endforeach; ?>
    </ul>
  </div>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
