<?php
require_once __DIR__ . '/../../includes/bootstrap.php';
require_once __DIR__ . '/../../includes/admin_upload.php';
$user = jstko_require_gm();

$notice = null;
$error = null;

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } else {
        try {
            $action = $_POST['form_action'] ?? '';
            if ($action === 'update_text') {
                $key = (string)($_POST['key'] ?? '');
                jstko_set_content($key, (string)($_POST['value'] ?? ''), 'text', $user['str_account_id']);
                $notice = 'Kaydedildi.';
            } elseif ($action === 'update_image') {
                $key = (string)($_POST['key'] ?? '');
                $uploaded = jstko_admin_save_upload('file');
                if ($uploaded !== null) {
                    jstko_set_content($key, $uploaded, 'image', $user['str_account_id']);
                    $notice = 'Görsel güncellendi.';
                } else {
                    $notice = 'Yeni dosya seçilmedi, değişiklik yapılmadı.';
                }
            } elseif ($action === 'add_key') {
                $key = trim((string)($_POST['new_key'] ?? ''));
                $kind = (string)($_POST['new_kind'] ?? 'text') === 'image' ? 'image' : 'text';
                if (preg_match('/^[a-z0-9_]{2,60}$/', $key)) {
                    jstko_set_content($key, '', $kind, $user['str_account_id']);
                    $notice = 'Yeni içerik anahtarı eklendi: ' . $key;
                } else {
                    $error = 'Anahtar sadece küçük harf, rakam ve "_" içerebilir.';
                }
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$rows = jstko_db()->query('SELECT * FROM web_content ORDER BY key')->fetchAll();

$pageTitle = 'Site İçeriği';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-1">Site İçeriği</h1>
  <p class="text-sm text-slate-400 mb-6">Anasayfadaki görseller ve metinler buradan değişir, kod değişikliği gerekmez.</p>

  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="space-y-4">
    <?php foreach ($rows as $r): ?>
      <div class="card p-5">
        <div class="flex items-center justify-between mb-2">
          <code class="text-xs text-gold-400"><?= jstko_e($r['key']) ?></code>
          <span class="text-xs text-slate-500">güncellendi: <?= jstko_time_ago($r['updated_at']) ?></span>
        </div>
        <?php if ($r['kind'] === 'image'): ?>
          <div class="flex items-center gap-4">
            <?php if ($r['value']): ?><img src="/uploads/<?= jstko_e($r['value']) ?>" class="h-16 rounded border border-ink-500"><?php else: ?><div class="h-16 w-28 rounded border border-dashed border-ink-500 grid place-items-center text-xs text-slate-600">Görsel yok</div><?php endif; ?>
            <form method="post" enctype="multipart/form-data" class="flex items-center gap-2">
              <?= jstko_csrf_field() ?>
              <input type="hidden" name="form_action" value="update_image">
              <input type="hidden" name="key" value="<?= jstko_e($r['key']) ?>">
              <input type="file" name="file" accept="image/*" class="text-xs text-slate-400">
              <button type="submit" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500">Yükle</button>
            </form>
          </div>
        <?php else: ?>
          <form method="post" class="flex items-center gap-2">
            <?= jstko_csrf_field() ?>
            <input type="hidden" name="form_action" value="update_text">
            <input type="hidden" name="key" value="<?= jstko_e($r['key']) ?>">
            <input class="input flex-1" type="text" name="value" value="<?= jstko_e($r['value']) ?>">
            <button type="submit" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500">Kaydet</button>
          </form>
        <?php endif; ?>
      </div>
    <?php endforeach; ?>
  </div>

  <div class="card p-5 mt-6">
    <h2 class="font-semibold text-white text-sm mb-3">Yeni İçerik Anahtarı Ekle</h2>
    <form method="post" class="flex flex-wrap gap-2 items-center">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="add_key">
      <input class="input flex-1 min-w-[160px]" type="text" name="new_key" placeholder="ornek_anahtar" pattern="[a-z0-9_]+">
      <select name="new_kind" class="input w-auto">
        <option value="text">Metin</option>
        <option value="image">Görsel</option>
      </select>
      <button type="submit" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500">Ekle</button>
    </form>
  </div>
</section>
<?php require __DIR__ . '/../partials/footer.php'; ?>
