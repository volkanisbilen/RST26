<?php
require_once __DIR__ . '/../includes/bootstrap.php';
require_once __DIR__ . '/../includes/admin_upload.php';
$user = jstko_require_gm();

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
                $image = jstko_admin_save_upload('image', jstko_image_spec('slide'));
                if ($image === null) {
                    throw new RuntimeException('Slider için bir görsel seçmelisin.');
                }
                $pdo->prepare(
                    'INSERT INTO web_slide (title, subtitle, image, link_url, button_text, sort_order, active) VALUES (:t, :s, :i, :l, :b, :o, TRUE)'
                )->execute([
                    't' => trim((string)($_POST['title'] ?? '')),
                    's' => trim((string)($_POST['subtitle'] ?? '')),
                    'i' => $image,
                    'l' => trim((string)($_POST['link_url'] ?? '')),
                    'b' => trim((string)($_POST['button_text'] ?? '')),
                    'o' => (int)($_POST['sort_order'] ?? 0),
                ]);
                $notice = 'Slayt eklendi.';
            } elseif ($action === 'update') {
                $id = (int)($_POST['id'] ?? 0);
                $pdo->prepare('UPDATE web_slide SET title=:t, subtitle=:s, link_url=:l, button_text=:b, sort_order=:o WHERE id=:id')->execute([
                    't' => trim((string)($_POST['title'] ?? '')),
                    's' => trim((string)($_POST['subtitle'] ?? '')),
                    'l' => trim((string)($_POST['link_url'] ?? '')),
                    'b' => trim((string)($_POST['button_text'] ?? '')),
                    'o' => (int)($_POST['sort_order'] ?? 0),
                    'id' => $id,
                ]);
                $image = jstko_admin_save_upload('image', jstko_image_spec('slide'));
                if ($image !== null) {
                    $pdo->prepare('UPDATE web_slide SET image=:i WHERE id=:id')->execute(['i' => $image, 'id' => $id]);
                }
                $notice = 'Slayt güncellendi.';
            } elseif ($action === 'toggle') {
                $pdo->prepare('UPDATE web_slide SET active = NOT active WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Durum değiştirildi.';
            } elseif ($action === 'delete') {
                $pdo->prepare('DELETE FROM web_slide WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Slayt silindi.';
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$slides = jstko_cms_rows('SELECT * FROM web_slide ORDER BY sort_order, id');

$pageTitle = 'Slider';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-1">Anasayfa Slider</h1>
  <p class="text-sm text-slate-400 mb-6">Anasayfanın en üstünde otomatik dönen görseller. Sıra numarası küçük olan önce gelir.</p>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3">Yeni Slayt</h2>
    <form method="post" enctype="multipart/form-data" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="create">
      <?= jstko_image_input('image', 'slide') ?>
      <div class="grid sm:grid-cols-2 gap-3">
        <input class="input" type="text" name="title" placeholder="Başlık (isteğe bağlı)" maxlength="160">
        <input class="input" type="text" name="subtitle" placeholder="Alt yazı (isteğe bağlı)" maxlength="300">
        <input class="input" type="text" name="link_url" placeholder="Bağlantı (örn. /patches.php)">
        <input class="input" type="text" name="button_text" placeholder="Buton yazısı (örn. İncele)" maxlength="60">
        <input class="input" type="number" name="sort_order" placeholder="Sıra (0,1,2...)" value="0">
      </div>
      <button type="submit" class="btn-primary">Ekle</button>
    </form>
  </div>

  <div class="space-y-4">
    <?php foreach ($slides as $s): ?>
      <div class="card p-4">
        <img src="/uploads/<?= jstko_e($s['image']) ?>" class="w-full rounded-lg border border-ink-500 mb-3 max-h-40 object-cover" alt="">
        <form method="post" enctype="multipart/form-data" class="space-y-2">
          <?= jstko_csrf_field() ?>
          <input type="hidden" name="form_action" value="update">
          <input type="hidden" name="id" value="<?= (int)$s['id'] ?>">
          <div class="grid sm:grid-cols-2 gap-2">
            <input class="input" type="text" name="title" value="<?= jstko_e($s['title']) ?>" placeholder="Başlık">
            <input class="input" type="text" name="subtitle" value="<?= jstko_e($s['subtitle']) ?>" placeholder="Alt yazı">
            <input class="input" type="text" name="link_url" value="<?= jstko_e($s['link_url']) ?>" placeholder="Bağlantı">
            <input class="input" type="text" name="button_text" value="<?= jstko_e($s['button_text']) ?>" placeholder="Buton yazısı">
            <input class="input" type="number" name="sort_order" value="<?= (int)$s['sort_order'] ?>">
          </div>
          <?= jstko_image_input('image', 'slide') ?>
          <div class="flex flex-wrap gap-2 items-center">
            <button type="submit" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500">Kaydet</button>
            <span class="text-xs <?= jstko_pg_bool($s['active']) ? 'text-emerald-400' : 'text-slate-500' ?>"><?= jstko_pg_bool($s['active']) ? 'Yayında' : 'Gizli' ?></span>
          </div>
        </form>
        <div class="flex gap-2 mt-2">
          <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="toggle"><input type="hidden" name="id" value="<?= (int)$s['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= jstko_pg_bool($s['active']) ? 'Gizle' : 'Yayınla' ?></button></form>
          <form method="post" onsubmit="return confirm('Silinsin mi?')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="id" value="<?= (int)$s['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button></form>
        </div>
      </div>
    <?php endforeach; ?>
    <?php if (!$slides): ?><div class="card p-10 text-center text-slate-500">Henüz slayt yok.</div><?php endif; ?>
  </div>
</section>
<?= jstko_image_check_script() ?>
<?php require __DIR__ . '/../partials/footer.php'; ?>
