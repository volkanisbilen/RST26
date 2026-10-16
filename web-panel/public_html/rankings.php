<?php
require_once __DIR__ . '/includes/bootstrap.php';
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
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Sıralamalar</h1>
  <p class="text-sm text-slate-400 mb-6">İlk 100 <?= $tab === 'clan' ? 'klan' : 'karakter' ?>.</p>

  <div class="flex flex-wrap gap-2 mb-6">
    <?php foreach ($tabs as $key => $label): ?>
      <a href="?tab=<?= $key ?>" class="text-sm px-3 py-1.5 rounded-lg border <?= $tab === $key ? 'bg-gold-500 text-ink-900 border-gold-500 font-semibold' : 'border-ink-500 text-slate-300 hover:border-gold-400' ?> transition"><?= jstko_e($label) ?></a>
    <?php endforeach; ?>
  </div>

  <div class="card overflow-x-auto">
    <table class="w-full text-sm">
      <?php if ($tab === 'clan'): ?>
      <thead class="text-left text-slate-500 text-xs uppercase bg-ink-600/50">
        <tr><th class="py-3 px-4">#</th><th class="py-3 px-4">Klan</th><th class="py-3 px-4">Ulus</th><th class="py-3 px-4">Üye</th><th class="py-3 px-4 text-right">Puan</th></tr>
      </thead>
      <tbody>
        <?php foreach ($rows as $i => $r): ?>
        <tr class="table-row border-t border-ink-500/50">
          <td class="py-2.5 px-4 text-slate-500">#<?= $i + 1 ?></td>
          <td class="py-2.5 px-4 font-medium text-white"><?= jstko_e($r['name']) ?></td>
          <td class="py-2.5 px-4 <?= ((int)$r['nation'] === 1) ? 'text-red-400' : 'text-blue-400' ?>"><?= jstko_e(jstko_nation_name((int)$r['nation'])) ?></td>
          <td class="py-2.5 px-4 text-slate-300"><?= (int)$r['members'] ?></td>
          <td class="py-2.5 px-4 text-right text-gold-400 font-semibold"><?= jstko_number($r['points']) ?></td>
        </tr>
        <?php endforeach; ?>
      </tbody>
      <?php else: ?>
      <thead class="text-left text-slate-500 text-xs uppercase bg-ink-600/50">
        <tr><th class="py-3 px-4">#</th><th class="py-3 px-4">Oyuncu</th><th class="py-3 px-4">Klan</th><th class="py-3 px-4">Seviye</th><th class="py-3 px-4 text-right">NP</th></tr>
      </thead>
      <tbody>
        <?php foreach ($rows as $i => $r): ?>
        <tr class="table-row border-t border-ink-500/50">
          <td class="py-2.5 px-4 text-slate-500">
            <?php if ($i < 3): ?><span class="text-gold-400">🏅</span><?php endif; ?> #<?= $i + 1 ?>
          </td>
          <td class="py-2.5 px-4 font-medium text-white">
            <?php if (jstko_pg_bool($r['online'])): ?><span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400 mr-1.5"></span><?php endif; ?>
            <?= jstko_e($r['name']) ?>
          </td>
          <td class="py-2.5 px-4 text-slate-400"><?= jstko_e($r['clan'] ?: 'Klan Yok') ?></td>
          <td class="py-2.5 px-4 text-slate-300"><?= (int)$r['level'] ?><?= $r['rebirth_level'] ? ' / ' . (int)$r['rebirth_level'] : '' ?></td>
          <td class="py-2.5 px-4 text-right text-gold-400 font-semibold"><?= jstko_number($tab === 'aylik' ? $r['loyalty_monthly'] : $r['loyalty']) ?></td>
        </tr>
        <?php endforeach; ?>
      <?php endif; ?>
      <?php if (!$rows): ?><tr><td colspan="5" class="py-10 text-center text-slate-500">Veri yok.</td></tr><?php endif; ?>
      </tbody>
    </table>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
