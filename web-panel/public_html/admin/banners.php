<?php
require_once __DIR__ . '/../includes/bootstrap.php';
require_once __DIR__ . '/../includes/admin_upload.php';
$user = jstko_require_gm();

$positions = [
    'home_top'  => 'Anasayfa — slider altı (geniş)',
    'home_mid'  => 'Anasayfa — sıralama üstü (geniş)',
    'home_side' => 'Anasayfa — sağ kenar (dikey)',
    'page_top'  => 'Sayfaların üstü (geniş)',
];

$notice = null;
$error = null;

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } else {
        try {
            $action = $_POST['form_action'] ?? '';
            $pdo = jstko_db();
            if ($action === 'create') {
                $pos = (string)($_POST['position'] ?? '');
                if (!isset($positions[$pos])) {
                    throw new RuntimeException('Geçersiz konum.');
                }
                $image = jstko_admin_save_upload('image', jstko_image_spec($pos));
                if ($image === null) {
                    throw new RuntimeException('Bir görsel seçmelisin.');
                }
                $pdo->prepare('INSERT INTO web_banner (position, image, link_url, alt, sort_order, active) VALUES (:p, :i, :l, :a, :o, TRUE)')->execute([
                    'p' => $pos, 'i' => $image,
                    'l' => trim((string)($_POST['link_url'] ?? '')),
                    'a' => trim((string)($_POST['alt'] ?? '')),
                    'o' => (int)($_POST['sort_order'] ?? 0),
                ]);
                $notice = 'Banner eklendi.';
            } elseif ($action === 'toggle') {
                $pdo->prepare('UPDATE web_banner SET active = NOT active WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Durum değiştirildi.';
            } elseif ($action === 'delete') {
                $pdo->prepare('DELETE FROM web_banner WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Banner silindi.';
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$banners = jstko_cms_rows('SELECT * FROM web_banner ORDER BY position, sort_order, id');

$pageTitle = 'Banner';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-1">Banner'lar</h1>
  <p class="text-sm text-slate-400 mb-6">Konum seçtikçe o konum için gereken piksel boyutu görünür; oranı uymayan resim kabul edilmez.</p>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3">Yeni Banner</h2>
    <form method="post" enctype="multipart/form-data" class="space-y-3" id="bannerForm">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="create">
      <select name="position" id="bannerPos" class="input">
        <?php foreach ($positions as $k => $label): $sp = jstko_image_spec($k); ?>
          <option value="<?= $k ?>" data-w="<?= $sp[0] ?>" data-h="<?= $sp[1] ?>"><?= jstko_e($label) ?> — <?= $sp[0] ?>×<?= $sp[1] ?> px</option>
        <?php endforeach; ?>
      </select>
      <div>
        <input type="file" name="image" accept="image/*" class="text-xs text-slate-400" id="bannerFile">
        <div class="text-xs text-amber-300/80 mt-1" id="bannerHint"></div>
        <div class="text-xs mt-1" id="bannerMsg"></div>
      </div>
      <div class="grid sm:grid-cols-3 gap-3">
        <input class="input" type="text" name="link_url" placeholder="Bağlantı (isteğe bağlı)">
        <input class="input" type="text" name="alt" placeholder="Açıklama (alt yazı)" maxlength="160">
        <input class="input" type="number" name="sort_order" placeholder="Sıra" value="0">
      </div>
      <button type="submit" class="btn-primary">Ekle</button>
    </form>
  </div>

  <div class="space-y-3">
    <?php foreach ($banners as $b): ?>
      <div class="card p-4">
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs text-gold-400"><?= jstko_e($positions[$b['position']] ?? $b['position']) ?> · sıra <?= (int)$b['sort_order'] ?></span>
          <span class="text-xs <?= jstko_pg_bool($b['active']) ? 'text-emerald-400' : 'text-slate-500' ?>"><?= jstko_pg_bool($b['active']) ? 'Yayında' : 'Gizli' ?></span>
        </div>
        <img src="/uploads/<?= jstko_e($b['image']) ?>" class="max-h-32 rounded border border-ink-500" alt="">
        <div class="text-xs text-slate-500 mt-1 break-all"><?= jstko_e($b['link_url']) ?></div>
        <div class="flex gap-2 mt-2">
          <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="toggle"><input type="hidden" name="id" value="<?= (int)$b['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= jstko_pg_bool($b['active']) ? 'Gizle' : 'Yayınla' ?></button></form>
          <form method="post" onsubmit="return confirm('Silinsin mi?')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="id" value="<?= (int)$b['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button></form>
        </div>
      </div>
    <?php endforeach; ?>
    <?php if (!$banners): ?><div class="card p-10 text-center text-slate-500">Henüz banner yok.</div><?php endif; ?>
  </div>
</section>
<script>
(function () {
  var sel = document.getElementById('bannerPos'), file = document.getElementById('bannerFile'),
      hint = document.getElementById('bannerHint'), msg = document.getElementById('bannerMsg');
  function spec() { var o = sel.options[sel.selectedIndex]; return { w: +o.dataset.w, h: +o.dataset.h }; }
  function showHint() { var s = spec(); hint.textContent = 'Bu konum için resim: ' + s.w + ' × ' + s.h + ' px (JPG/PNG/WEBP, en çok 5 MB)'; check(); }
  function check() {
    var f = file.files && file.files[0]; if (!f) { msg.textContent = ''; return; }
    var img = new Image(), s = spec();
    img.onload = function () {
      var ok = Math.abs((img.width / img.height) - (s.w / s.h)) / (s.w / s.h) <= 0.12 && img.width >= s.w * 0.6;
      msg.className = 'text-xs mt-1 ' + (ok ? 'text-emerald-400' : 'text-red-400');
      msg.textContent = 'Seçilen: ' + img.width + '×' + img.height + ' px — ' + (ok ? 'uygun' : 'UYGUN DEĞİL, ' + s.w + '×' + s.h + ' px olmalı');
    };
    img.src = URL.createObjectURL(f);
  }
  sel.addEventListener('change', showHint); file.addEventListener('change', check); showHint();
})();
</script>
<?php require __DIR__ . '/../partials/footer.php'; ?>
