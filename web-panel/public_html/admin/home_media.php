<?php
require_once __DIR__ . '/../includes/bootstrap.php';
require_once __DIR__ . '/../includes/admin_upload.php';
$user = jstko_require_gm();

$notice = null;
$error = null;
$by = $user['str_account_id'];

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } else {
        try {
            $mode = (string)($_POST['media_mode'] ?? 'video');
            if (!in_array($mode, ['video', 'image', 'none'], true)) $mode = 'video';
            // one YouTube link per line; every page load of the home page shows a random one
            $links = [];
            foreach (preg_split('/\R+/', (string)($_POST['video_urls'] ?? '')) as $line) {
                $line = trim($line);
                if ($line === '') continue;
                if (!preg_match('~(?:youtu\.be/|v=|embed/|shorts/)([A-Za-z0-9_-]{11})~', $line)) {
                    throw new RuntimeException('Geçerli bir YouTube bağlantısı değil: ' . $line);
                }
                $links[] = $line;
            }
            if (count($links) > 20) throw new RuntimeException('En çok 20 video eklenebilir.');
            $video = $links[0] ?? '';
            jstko_set_content('video_urls', implode("\n", $links), 'text', $by);
            jstko_set_content('media_mode', $mode, 'text', $by);
            jstko_set_content('video_url', $video, 'text', $by);
            jstko_set_content('video_title', trim((string)($_POST['video_title'] ?? '')), 'text', $by);
            jstko_set_content('video_caption', trim((string)($_POST['video_caption'] ?? '')), 'text', $by);
            jstko_set_content('media_link', trim((string)($_POST['media_link'] ?? '')), 'text', $by);
            $uploaded = jstko_admin_save_upload('video_poster', jstko_image_spec('video_poster'));
            if ($uploaded !== null) {
                jstko_set_content('video_poster', $uploaded, 'image', $by);
            } elseif (!empty($_POST['clear_poster'])) {
                jstko_set_content('video_poster', '', 'image', $by);
            }
            $notice = 'Kaydedildi. Anasayfada hemen görünür.';
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$mode = jstko_c('media_mode', jstko_c('video_url') !== '' ? 'video' : 'image');
$poster = jstko_c_image('video_poster');

$pageTitle = 'Tanıtım Alanı';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1>Tanıtım Alanı</h1>
  <p class="text-sm text-slate-400 mb-6">Anasayfada "Güncelleme Notları"nın solundaki büyük kutu. İstersen YouTube tanıtım videosu, istersen tıklanabilir bir görsel koy, istersen tamamen gizle.</p>

  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <form method="post" enctype="multipart/form-data" class="card p-6 space-y-5">
    <?= jstko_csrf_field() ?>

    <fieldset>
      <legend class="text-sm font-semibold text-white mb-2">Ne gösterilsin?</legend>
      <div class="grid sm:grid-cols-3 gap-3 text-sm">
        <?php foreach (['video' => ['Video', 'Birden çok YouTube videosu, sessiz kendiliğinden oynar, her açılışta rastgele biri'], 'image' => ['Görsel', 'Resim; istersen tıklayınca bir sayfaya gider (etkinlik, duyuru, indirme…)'], 'none' => ['Gizle', 'Kutu görünmez, Güncelleme Notları tam genişlikte olur']] as $v => [$t, $d]): ?>
          <label class="block cursor-pointer rounded-lg border p-3 <?= $mode === $v ? 'border-gold-400 bg-gold-500/10' : 'border-ink-500' ?>">
            <input type="radio" name="media_mode" value="<?= $v ?>" <?= $mode === $v ? 'checked' : '' ?> class="mr-1"> <b class="text-white"><?= $t ?></b>
            <span class="block text-xs text-slate-400 mt-1"><?= $d ?></span>
          </label>
        <?php endforeach; ?>
      </div>
    </fieldset>

    <label class="block text-sm text-slate-300">YouTube video bağlantıları <span class="text-slate-500">(her satıra bir tane, en çok 20; sadece "Video" seçiliyken)</span>
      <textarea class="input mt-1 font-mono text-xs" name="video_urls" rows="7" placeholder="https://www.youtube.com/watch?v=...&#10;https://youtu.be/..."><?= jstko_e(trim(jstko_c('video_urls') !== '' ? jstko_c('video_urls') : jstko_c('video_url'))) ?></textarea>
      <span class="block text-xs text-slate-500 mt-1">Anasayfa her açıldığında bunlardan rastgele biri sessiz olarak kendiliğinden oynar. Ziyaretçi sağ üstteki düğmelerle sesi açabilir veya başka bir videoya geçebilir.</span>
    </label>

    <label class="block text-sm text-slate-300">Görsele tıklayınca gidilecek adres <span class="text-slate-500">(sadece "Görsel" seçiliyken, boş bırakılırsa tıklanmaz)</span>
      <input class="input mt-1" type="text" name="media_link" value="<?= jstko_e(jstko_c('media_link')) ?>" placeholder="/news.php?kind=news  veya  https://...">
    </label>

    <div class="grid sm:grid-cols-2 gap-4">
      <label class="block text-sm text-slate-300">Başlık
        <input class="input mt-1" type="text" name="video_title" value="<?= jstko_e(jstko_c('video_title')) ?>" placeholder="JstKO Tanıtım Videosu">
      </label>
      <label class="block text-sm text-slate-300">Alt yazı
        <input class="input mt-1" type="text" name="video_caption" value="<?= jstko_e(jstko_c('video_caption')) ?>" placeholder="Efsane geri döndü! JstKO dünyasını keşfet.">
      </label>
    </div>

    <div class="text-sm text-slate-300">
      <div class="mb-2">Kutunun görseli <span class="text-slate-500">(video için kapak resmi; boş bırakılırsa slider'ın ilk görseli kullanılır)</span></div>
      <?php if ($poster): ?>
        <img src="<?= jstko_e($poster) ?>" class="h-24 rounded border border-ink-500 mb-2" alt="">
        <label class="flex items-center gap-2 text-xs text-slate-400 mb-2"><input type="checkbox" name="clear_poster" value="1"> Görseli kaldır</label>
      <?php endif; ?>
      <?= jstko_image_input('video_poster', 'video_poster') ?>
    </div>

    <button type="submit" class="btn-primary">Kaydet</button>
  </form>
</section>
<?= jstko_image_check_script() ?>
<?php require __DIR__ . '/../partials/footer.php'; ?>
