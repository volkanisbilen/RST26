<?php
require_once __DIR__ . '/includes/bootstrap.php';

$id = (int)($_GET['id'] ?? 0);
$B = JSTKO_BASE;
$kinds = ['news' => 'Haberler & Duyurular', 'update' => 'Güncelleme Notları'];

// ---------------------------------------------------------------- one entry
if ($id > 0) {
    $stmt = jstko_db()->prepare('SELECT * FROM web_news WHERE id = :id AND published');
    $stmt->execute(['id' => $id]);
    $item = $stmt->fetch();
    if (!$item) {
        http_response_code(404);
        $pageTitle = 'Bulunamadı';
        require __DIR__ . '/partials/header.php';
        echo '<section class="max-w-3xl mx-auto px-4 py-16 text-center text-slate-500">Yazı bulunamadı.</section>';
        require __DIR__ . '/partials/footer.php';
        exit;
    }
    $ik = ($item['kind'] ?? 'news') === 'update' ? 'update' : 'news';
    $pageTitle = $item['title'];
    require __DIR__ . '/partials/header.php';
    ?>
<article class="max-w-3xl mx-auto px-4 py-10">
  <a href="<?= $B ?>/news.php?kind=<?= $ik ?>" class="text-sm text-slate-400 hover:text-gold-400">&larr; <?= jstko_e($kinds[$ik]) ?></a>
  <h1 class="text-2xl sm:text-3xl font-bold text-white mt-2 mb-2"><?= jstko_e($item['title']) ?></h1>
  <div class="text-xs text-slate-500 mb-6"><?= jstko_time_ago($item['created_at']) ?></div>
  <?php if ($item['image']): ?>
    <img src="/uploads/<?= jstko_e(ltrim($item['image'], '/')) ?>" class="w-full rounded-xl mb-6 border border-ink-500" alt="">
  <?php endif; ?>
  <div class="prose prose-invert max-w-none text-slate-200 text-sm leading-relaxed whitespace-pre-wrap"><?= jstko_e($item['body']) ?></div>
</article>
<?php
    require __DIR__ . '/partials/footer.php';
    exit;
}

// ---------------------------------------------------------------- list of one kind
$kind = ($_GET['kind'] ?? 'news') === 'update' ? 'update' : 'news';
$per = 12;
$pdo = jstko_db();
$stmt = $pdo->prepare('SELECT count(*) FROM web_news WHERE published AND kind = :k');
$stmt->execute(['k' => $kind]);
$total = (int)$stmt->fetchColumn();
$pages = max(1, (int)ceil($total / $per));
$page = min(max(1, (int)($_GET['p'] ?? 1)), $pages);
$stmt = $pdo->prepare("SELECT id, title, body, image, created_at FROM web_news WHERE published AND kind = :k ORDER BY created_at DESC LIMIT $per OFFSET " . (($page - 1) * $per));
$stmt->execute(['k' => $kind]);
$rows = $stmt->fetchAll();

$pageTitle = $kinds[$kind];
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1><?= jstko_e($kinds[$kind]) ?></h1>
  <p class="text-sm text-slate-400 mb-6"><?= $kind === 'update' ? 'Sunucudaki değişiklikler ve yenilikler.' : 'Sunucudan haberler, etkinlik ve duyurular.' ?></p>

  <div class="flex flex-wrap gap-2 mb-6">
    <?php foreach ($kinds as $key => $label): ?>
      <a href="<?= $B ?>/news.php?kind=<?= $key ?>" class="tab-pill <?= $kind === $key ? 'on' : '' ?>"><?= jstko_e($label) ?></a>
    <?php endforeach; ?>
  </div>

  <?php if (!$rows): ?>
    <div class="card p-10 text-center text-slate-500">Henüz yazı yok.</div>
  <?php else: ?>
    <div class="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
      <?php foreach ($rows as $i => $n):
          $img = $n['image'] !== '' ? '/uploads/' . ltrim($n['image'], '/') : '';
          $excerpt = trim(preg_replace('/\s+/', ' ', (string)$n['body']));
          if (mb_strlen($excerpt) > 110) $excerpt = mb_substr($excerpt, 0, 110) . '…';
      ?>
        <a href="<?= $B ?>/news.php?id=<?= (int)$n['id'] ?>" class="news-card group block min-h-[210px] <?= $img ? 'bg-ink-700' : 'news-ph' ?>" style="--i:<?= $i ?>">
          <?php if ($img): ?><img src="<?= jstko_e($img) ?>" alt="" loading="lazy" class="absolute inset-0 w-full h-full object-cover group-hover:scale-105 transition duration-500"><?php endif; ?>
          <span class="absolute inset-0 bg-gradient-to-t from-ink-900/95 via-ink-900/60 to-ink-900/10"></span>
          <span class="relative flex flex-col justify-end h-full min-h-[210px] p-4">
            <span><span class="tag"><?= $kind === 'update' ? 'GÜNCELLEME' : 'DUYURU' ?></span></span>
            <span class="block text-base font-semibold text-white mt-2 line-clamp-2"><?= jstko_e($n['title']) ?></span>
            <?php if ($excerpt !== ''): ?><span class="block text-xs text-slate-300/90 mt-1 line-clamp-2"><?= jstko_e($excerpt) ?></span><?php endif; ?>
            <span class="block text-[11px] text-slate-400 mt-2"><?= date('d.m.Y', strtotime($n['created_at'])) ?></span>
          </span>
        </a>
      <?php endforeach; ?>
    </div>
    <?php if ($pages > 1): ?>
      <nav class="flex flex-wrap items-center justify-center gap-2 mt-8" aria-label="Sayfalar">
        <?php if ($page > 1): ?><a href="?kind=<?= $kind ?>&p=<?= $page - 1 ?>" class="tab-pill">&larr; Önceki</a><?php endif; ?>
        <span class="text-sm text-slate-400"><?= $page ?> / <?= $pages ?></span>
        <?php if ($page < $pages): ?><a href="?kind=<?= $kind ?>&p=<?= $page + 1 ?>" class="tab-pill">Sonraki &rarr;</a><?php endif; ?>
      </nav>
    <?php endif; ?>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
