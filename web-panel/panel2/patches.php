<?php
require_once __DIR__ . '/includes/bootstrap.php';

$patches = jstko_cms_rows('SELECT * FROM web_patch WHERE active ORDER BY created_at DESC, id DESC');
$latest = $patches[0] ?? null;
$older = array_slice($patches, 1);

$pageTitle = 'Patch İndir';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-3xl mx-auto px-4 py-10">
  <h1 class="text-2xl sm:text-3xl font-bold text-white mb-2">Patch İndir</h1>
  <p class="text-slate-400 text-sm mb-8"><?= jstko_e(jstko_c('patch_page_intro', 'Oyunda sorun yaşıyorsan en son yamayı indirip oyun klasörüne çıkart.')) ?></p>

  <?php if ($latest): ?>
    <div class="card p-6 mb-8 border-gold-500/60">
      <div class="text-xs font-semibold uppercase tracking-wide text-gold-400 mb-1">Son Patch</div>
      <h2 class="text-xl font-bold text-white">Sürüm <?= jstko_e($latest['version']) ?><?= $latest['title'] !== '' ? ' — ' . jstko_e($latest['title']) : '' ?></h2>
      <div class="text-xs text-slate-500 mt-1"><?= jstko_time_ago($latest['created_at']) ?><?= $latest['size_mb'] !== '' ? ' · ' . jstko_e($latest['size_mb']) : '' ?></div>
      <?php if ($latest['notes'] !== ''): ?><div class="text-sm text-slate-300 mt-3 whitespace-pre-wrap"><?= jstko_e($latest['notes']) ?></div><?php endif; ?>
      <a href="<?= jstko_e($latest['url']) ?>" class="btn-primary inline-block mt-5" rel="noopener">⬇ Son Patch'i İndir</a>
    </div>
  <?php else: ?>
    <div class="card p-10 text-center text-slate-500">Şu an indirilebilir patch yok.</div>
  <?php endif; ?>

  <?php if ($older): ?>
    <h2 class="font-bold text-white mb-3">Önceki Patchler</h2>
    <div class="space-y-2">
      <?php foreach ($older as $p): ?>
        <div class="card p-4 flex items-center justify-between gap-4">
          <div>
            <div class="text-white font-medium">Sürüm <?= jstko_e($p['version']) ?><?= $p['title'] !== '' ? ' — ' . jstko_e($p['title']) : '' ?></div>
            <div class="text-xs text-slate-500"><?= jstko_time_ago($p['created_at']) ?><?= $p['size_mb'] !== '' ? ' · ' . jstko_e($p['size_mb']) : '' ?></div>
          </div>
          <a href="<?= jstko_e($p['url']) ?>" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500 shrink-0" rel="noopener">İndir</a>
        </div>
      <?php endforeach; ?>
    </div>
  <?php endif; ?>

  <div class="card p-5 mt-8 text-sm text-slate-400">
    <b class="text-white">Nasıl kurulur?</b> İndirdiğin dosyayı oyun klasörüne (KnightOnline.exe'nin olduğu yer) çıkart, var olan dosyaların üzerine yazmasına izin ver ve oyunu tekrar başlat.
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
