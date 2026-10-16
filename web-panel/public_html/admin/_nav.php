<?php $here = basename($_SERVER['SCRIPT_NAME']); ?>
<div class="bg-ink-700 border-b border-ink-500">
  <div class="max-w-5xl mx-auto px-4 flex gap-1 overflow-x-auto text-sm">
    <?php
    $links = [
        'index.php' => 'Genel Bakış',
        'content.php' => 'Site İçeriği',
        'news.php' => 'Haberler',
        'tickets.php' => 'Destek Talepleri',
    ];
    foreach ($links as $file => $label):
        $active = $here === $file;
    ?>
      <a href="/admin/<?= $file ?>" class="px-4 py-3 border-b-2 <?= $active ? 'border-gold-400 text-gold-400 font-semibold' : 'border-transparent text-slate-400 hover:text-white' ?> transition whitespace-nowrap"><?= jstko_e($label) ?></a>
    <?php endforeach; ?>
  </div>
</div>
