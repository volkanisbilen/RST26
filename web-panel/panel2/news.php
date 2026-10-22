<?php
require_once __DIR__ . '/includes/bootstrap.php';

$id = (int)($_GET['id'] ?? 0);
$stmt = jstko_db()->prepare('SELECT * FROM web_news WHERE id = :id AND published');
$stmt->execute(['id' => $id]);
$item = $stmt->fetch();
if (!$item) {
    http_response_code(404);
    $pageTitle = 'Bulunamadı';
    require __DIR__ . '/partials/header.php';
    echo '<section class="max-w-3xl mx-auto px-4 py-16 text-center text-slate-500">Haber bulunamadı.</section>';
    require __DIR__ . '/partials/footer.php';
    exit;
}

$pageTitle = $item['title'];
require __DIR__ . '/partials/header.php';
?>
<article class="max-w-3xl mx-auto px-4 py-10">
  <a href="<?= JSTKO_BASE ?>/" class="text-sm text-slate-400 hover:text-gold-400">&larr; Anasayfa</a>
  <h1 class="text-2xl sm:text-3xl font-bold text-white mt-2 mb-2"><?= jstko_e($item['title']) ?></h1>
  <div class="text-xs text-slate-500 mb-6"><?= jstko_time_ago($item['created_at']) ?></div>
  <?php if ($item['image']): ?>
    <img src="/uploads/<?= jstko_e(ltrim($item['image'], '/')) ?>" class="w-full rounded-xl mb-6 border border-ink-500" alt="">
  <?php endif; ?>
  <div class="prose prose-invert max-w-none text-slate-200 text-sm leading-relaxed whitespace-pre-wrap"><?= jstko_e($item['body']) ?></div>
</article>
<?php require __DIR__ . '/partials/footer.php'; ?>
