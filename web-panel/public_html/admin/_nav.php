<?php
$here = basename($_SERVER['SCRIPT_NAME']);
$hereKind = ($_GET['kind'] ?? $_POST['kind'] ?? 'news') === 'update' ? 'update' : 'news';
?>
<div class="bg-ink-700 border-b border-ink-500">
  <div class="max-w-5xl mx-auto px-4 flex gap-1 overflow-x-auto text-sm">
    <?php
    // [file, label, kind query (news.php only)]
    $links = [
        ['index.php', 'Genel Bakış', null],
        ['content.php', 'Site İçeriği', null],
        ['home_media.php', 'Tanıtım Alanı', null],
        ['news.php', 'Haberler & Duyurular', 'news'],
        ['news.php', 'Güncelleme Notları', 'update'],
        ['patches.php', 'Patchler', null],
        ['slides.php', 'Slider', null],
        ['banners.php', 'Banner', null],
        ['pages.php', 'Sayfalar', null],
        ['menu.php', 'Menü', null],
        ['tickets.php', 'Destek Talepleri', null],
    ];
    foreach ($links as [$file, $label, $k]):
        $active = $here === $file && ($k === null || $k === $hereKind);
        $href = '/admin/' . $file . ($k ? '?kind=' . $k : '');
    ?>
      <a href="<?= $href ?>" class="px-4 py-3 border-b-2 <?= $active ? 'border-gold-400 text-gold-400 font-semibold' : 'border-transparent text-slate-400 hover:text-white' ?> transition whitespace-nowrap"><?= jstko_e($label) ?></a>
    <?php endforeach; ?>
  </div>
</div>
