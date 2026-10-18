<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Online Oyuncular';
require __DIR__ . '/partials/header.php';

$rows = jstko_db()->query(
    "SELECT c.str_char_id name, u.nation, u.class, u.level, u.rebirth_level, k.id_name clan
     FROM currentuser c LEFT JOIN userdata u ON lower(u.str_user_id) = lower(c.str_char_id)
     LEFT JOIN knights k ON k.id_num = u.knights
     ORDER BY u.level DESC NULLS LAST, c.str_char_id LIMIT 500"
)->fetchAll();
?>
<section class="max-w-6xl mx-auto px-4 py-10">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-bold text-white">Online Oyuncular</h1>
    <span class="text-sm text-slate-400"><?= jstko_number(count($rows)) ?> oyuncu şu an oyunda</span>
  </div>

  <div class="card overflow-x-auto">
    <table class="w-full text-sm">
      <thead class="text-left text-slate-500 text-xs uppercase bg-ink-600/50">
        <tr><th class="py-3 px-4">Karakter</th><th class="py-3 px-4">Ulus</th><th class="py-3 px-4">Sınıf</th><th class="py-3 px-4">Seviye</th><th class="py-3 px-4">Klan</th></tr>
      </thead>
      <tbody>
        <?php foreach ($rows as $r): ?>
        <tr class="table-row border-t border-ink-500/50">
          <td class="py-2.5 px-4 font-medium text-white"><span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400 mr-1.5"></span><?= jstko_e($r['name']) ?></td>
          <td class="py-2.5 px-4 <?= ((int)$r['nation'] === 1) ? 'text-red-400' : 'text-blue-400' ?>"><?= jstko_e(jstko_nation_name((int)$r['nation'])) ?></td>
          <td class="py-2.5 px-4 text-slate-300"><?= jstko_e(jstko_class_name((int)$r['class'])) ?></td>
          <td class="py-2.5 px-4 text-slate-300"><?= (int)$r['level'] ?><?= $r['rebirth_level'] ? ' / ' . (int)$r['rebirth_level'] : '' ?></td>
          <td class="py-2.5 px-4 text-slate-400"><?= jstko_e($r['clan'] ?: 'Klan Yok') ?></td>
        </tr>
        <?php endforeach; ?>
        <?php if (!$rows): ?><tr><td colspan="5" class="py-10 text-center text-slate-500">Şu anda oyunda kimse yok.</td></tr><?php endif; ?>
      </tbody>
    </table>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
