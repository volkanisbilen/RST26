<?php
require_once __DIR__ . '/includes/bootstrap.php';

$slug = (string)($_GET['slug'] ?? '');
$rows = jstko_cms_rows('SELECT * FROM web_page WHERE slug = :s AND published', ['s' => $slug]);
$item = $rows[0] ?? null;
if (!$item) {
    http_response_code(404);
    $pageTitle = 'Bulunamadı';
    require __DIR__ . '/partials/header.php';
    echo '<section class="max-w-3xl mx-auto px-4 py-16 text-center text-slate-500">Sayfa bulunamadı.</section>';
    require __DIR__ . '/partials/footer.php';
    exit;
}

$pageTitle = $item['title'];
require __DIR__ . '/partials/header.php';
?>
<?php if ($item['header_image'] !== ''): ?>
  <div class="relative h-40 sm:h-56 overflow-hidden border-b border-ink-500">
    <img src="/uploads/<?= jstko_e($item['header_image']) ?>" class="w-full h-full object-cover opacity-60" alt="">
    <div class="absolute inset-0 bg-gradient-to-t from-ink-900 to-transparent"></div>
  </div>
<?php endif; ?>
<article class="max-w-3xl mx-auto px-4 py-10">
  <?php if ($tb = jstko_banner_html('page_top')): ?><div class="mb-6 space-y-3"><?= $tb ?></div><?php endif; ?>
  <h1 class="text-2xl sm:text-3xl font-bold text-white mb-6"><?= jstko_e($item['title']) ?></h1>
  <div class="page-body text-slate-200 text-sm leading-relaxed space-y-4"><?= $item['body'] /* trusted HTML from a GM */ ?></div>
</article>
<?php require __DIR__ . '/partials/footer.php'; ?>
