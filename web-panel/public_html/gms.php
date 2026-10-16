<?php
require_once __DIR__ . '/../includes/bootstrap.php';
$pageTitle = 'Online GM\'ler';
require __DIR__ . '/partials/header.php';

$rows = jstko_db()->query(
    "SELECT c.str_char_id name, u.nation, u.class, u.level
     FROM currentuser c JOIN userdata u ON lower(u.str_user_id) = lower(c.str_char_id)
     WHERE u.authority IN (0, 2)
     ORDER BY c.str_char_id"
)->fetchAll();
?>
<section class="max-w-3xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-2">Online GM'ler</h1>
  <p class="text-sm text-slate-400 mb-6">Şu anda oyunda bulunan Game Master'lar.</p>

  <?php if ($rows): ?>
  <div class="grid sm:grid-cols-2 gap-4">
    <?php foreach ($rows as $r): ?>
      <div class="card p-4 flex items-center gap-4">
        <div class="h-11 w-11 rounded-full bg-gold-500/15 border border-gold-500/40 grid place-items-center text-gold-400 font-bold">GM</div>
        <div>
          <div class="font-semibold text-white"><?= jstko_e($r['name']) ?></div>
          <div class="text-xs text-slate-400"><?= jstko_e(jstko_nation_name((int)$r['nation'])) ?> · <?= jstko_e(jstko_class_name((int)$r['class'])) ?> · Sv.<?= (int)$r['level'] ?></div>
        </div>
      </div>
    <?php endforeach; ?>
  </div>
  <?php else: ?>
    <div class="card p-10 text-center text-slate-500">Şu anda oyunda GM yok.</div>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
