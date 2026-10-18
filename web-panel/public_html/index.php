<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Anasayfa';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();
$online = (int)$pdo->query('SELECT count(*) FROM currentuser')->fetchColumn();

$openDate = jstko_c('server_open_date');
$daysOpen = null;
if ($openDate !== '') {
    $ts = strtotime($openDate);
    if ($ts !== false) {
        $daysOpen = max(0, (int)floor((time() - $ts) / 86400));
    }
}

$top = $pdo->query(
    "SELECT u.str_user_id name, k.id_name clan, u.loyalty np,
            EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) online
     FROM userdata u LEFT JOIN knights k ON k.id_num = u.knights
     ORDER BY u.loyalty DESC LIMIT 10"
)->fetchAll();

$news = $pdo->query(
    "SELECT id, title, body, image, created_at FROM web_news WHERE published ORDER BY created_at DESC LIMIT 5"
)->fetchAll();

$heroImage = jstko_c_image('hero_image');
?>

<section class="relative overflow-hidden border-b border-ink-500">
  <?php if ($heroImage): ?>
    <div class="absolute inset-0">
      <img src="<?= jstko_e($heroImage) ?>" class="w-full h-full object-cover opacity-30" alt="">
      <div class="absolute inset-0 bg-gradient-to-t from-ink-900 via-ink-900/80 to-ink-900/40"></div>
    </div>
  <?php endif; ?>
  <div class="relative max-w-6xl mx-auto px-4 py-16 sm:py-24 grid lg:grid-cols-2 gap-10 items-center">
    <div>
      <p class="inline-block text-xs font-semibold tracking-wide uppercase text-gold-400 bg-gold-500/10 border border-gold-500/30 rounded-full px-3 py-1 mb-4">
        MMORPG'nin ücretsiz ve en eğlenceli sunucusu
      </p>
      <h1 class="text-3xl sm:text-5xl font-extrabold text-white leading-tight mb-4"><?= jstko_e(jstko_c('hero_title', 'Hoş Geldin')) ?></h1>
      <p class="text-slate-400 text-base sm:text-lg mb-8"><?= jstko_e(jstko_c('hero_subtitle')) ?></p>
      <div class="flex flex-wrap gap-3">
        <?php if ($dl = jstko_c('download_url')): ?>
          <a href="<?= jstko_e($dl) ?>" class="btn-primary">⬇ Oyunu İndir</a>
        <?php endif; ?>
        <a href="/register.php" class="px-5 py-2.5 rounded-lg border border-ink-500 hover:border-gold-400 hover:text-gold-400 transition font-semibold">Hesap Oluştur</a>
      </div>
    </div>
    <div class="grid grid-cols-2 gap-4">
      <div class="card p-6 text-center">
        <div class="text-3xl font-extrabold text-emerald-400"><?= jstko_number($online) ?></div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wide">Şu An Oyunda</div>
      </div>
      <?php if ($daysOpen !== null): ?>
      <div class="card p-6 text-center">
        <div class="text-3xl font-extrabold text-white"><?= jstko_number($daysOpen) ?></div>
        <div class="text-xs text-slate-400 mt-1 uppercase tracking-wide">Gündür Açık</div>
      </div>
      <?php endif; ?>
    </div>
  </div>
</section>

<section class="max-w-6xl mx-auto px-4 py-12 grid lg:grid-cols-3 gap-8">
  <div class="lg:col-span-2 card p-6">
    <div class="flex items-center justify-between mb-4">
      <h2 class="font-bold text-white text-lg">En İyi 10 Oyuncu</h2>
      <a href="/rankings.php" class="text-sm text-gold-400 hover:underline">Tümünü gör</a>
    </div>
    <div class="overflow-x-auto">
    <table class="w-full text-sm">
      <thead class="text-left text-slate-500 text-xs uppercase">
        <tr><th class="py-2 pr-3">#</th><th class="py-2 pr-3">Oyuncu</th><th class="py-2 pr-3">Klan</th><th class="py-2 text-right">NP</th></tr>
      </thead>
      <tbody>
        <?php foreach ($top as $i => $r): ?>
        <tr class="table-row border-t border-ink-500/50">
          <td class="py-2 pr-3 text-slate-500"><?= $i + 1 ?></td>
          <td class="py-2 pr-3 font-medium text-white">
            <?php if (jstko_pg_bool($r['online'])): ?><span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400 mr-1.5" title="Online"></span><?php endif; ?>
            <?= jstko_e($r['name']) ?>
          </td>
          <td class="py-2 pr-3 text-slate-400"><?= jstko_e($r['clan'] ?: 'Klan Yok') ?></td>
          <td class="py-2 text-right text-gold-400 font-semibold"><?= jstko_number($r['np']) ?></td>
        </tr>
        <?php endforeach; ?>
        <?php if (!$top): ?><tr><td colspan="4" class="py-6 text-center text-slate-500">Henüz veri yok.</td></tr><?php endif; ?>
      </tbody>
    </table>
    </div>
  </div>

  <div class="card p-6">
    <h2 class="font-bold text-white text-lg mb-4">Güncelleme Notları</h2>
    <div class="space-y-4">
      <?php foreach ($news as $n): ?>
        <a href="/news.php?id=<?= (int)$n['id'] ?>" class="block border-b border-ink-500/50 pb-3 last:border-0 last:pb-0 group">
          <div class="font-semibold text-white group-hover:text-gold-400 transition text-sm"><?= jstko_e($n['title']) ?></div>
          <div class="text-xs text-slate-500 mt-1"><?= jstko_time_ago($n['created_at']) ?></div>
        </a>
      <?php endforeach; ?>
      <?php if (!$news): ?><p class="text-sm text-slate-500">Henüz haber yok.</p><?php endif; ?>
    </div>
  </div>
</section>

<?php require __DIR__ . '/partials/footer.php'; ?>
