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
                $version = trim((string)($_POST['version'] ?? ''));
                $url = trim((string)($_POST['url'] ?? ''));
                $uploaded = jstko_admin_save_patch_file('file');
                if ($uploaded !== null) {
                    $url = $uploaded;
                }
                if ($version === '' || $url === '') {
                    throw new RuntimeException('Sürüm ve (dosya ya da bağlantı) gerekli.');
                }
                if ($uploaded === null && !preg_match('#^https?://#i', $url)) {
                    throw new RuntimeException('Bağlantı http:// veya https:// ile başlamalı.');
                }
                $pdo->prepare(
                    'INSERT INTO web_patch (version, title, notes, url, size_mb, active, created_by) VALUES (:v, :t, :n, :u, :s, :a, :c)'
                )->execute([
                    'v' => $version,
                    't' => trim((string)($_POST['title'] ?? '')),
                    'n' => trim((string)($_POST['notes'] ?? '')),
                    'u' => $url,
                    's' => trim((string)($_POST['size_mb'] ?? '')),
                    'a' => !empty($_POST['active']) ? 'true' : 'false',
                    'c' => $user['str_account_id'],
                ]);
                $notice = 'Patch eklendi.';
            } elseif ($action === 'toggle') {
                $pdo->prepare('UPDATE web_patch SET active = NOT active WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Durum değiştirildi.';
            } elseif ($action === 'make_latest') {
                // "son patch" is the newest active one: bump its date to now
                $pdo->prepare('UPDATE web_patch SET active = TRUE, created_at = now() WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Bu patch artık "Son Patch".';
            } elseif ($action === 'delete') {
                $id = (int)($_POST['id'] ?? 0);
                $row = $pdo->prepare('SELECT url FROM web_patch WHERE id = :id');
                $row->execute(['id' => $id]);
                $url = (string)$row->fetchColumn();
                $pdo->prepare('DELETE FROM web_patch WHERE id = :id')->execute(['id' => $id]);
                if (str_starts_with($url, '/uploads/patches/')) {
                    @unlink(__DIR__ . '/..' . $url);
                }
                $notice = 'Patch silindi.';
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$patches = jstko_cms_rows('SELECT * FROM web_patch ORDER BY created_at DESC, id DESC');
$latestId = null;
foreach ($patches as $p) {
    if (jstko_pg_bool($p['active'])) { $latestId = (int)$p['id']; break; }
}

$pageTitle = 'Patchler';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-1">Manuel Patchler</h1>
  <p class="text-sm text-slate-400 mb-6">Oyuncular <code class="text-gold-400">/patches.php</code> sayfasından indirir. Listedeki en üstteki <b>aktif</b> kayıt "Son Patch" olarak öne çıkar.</p>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3">Yeni Patch Ekle</h2>
    <form method="post" enctype="multipart/form-data" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="create">
      <div class="grid sm:grid-cols-3 gap-3">
        <input class="input" type="text" name="version" placeholder="Sürüm (örn. 2651)" required maxlength="30">
        <input class="input sm:col-span-2" type="text" name="title" placeholder="Başlık (isteğe bağlı)" maxlength="160">
      </div>
      <textarea class="input" name="notes" rows="3" placeholder="Notlar / kurulum açıklaması (isteğe bağlı)"></textarea>
      <div class="grid sm:grid-cols-2 gap-3">
        <div>
          <label class="text-xs text-slate-400">Harici bağlantı (Drive, Mega, kendi sunucun...)</label>
          <input class="input mt-1" type="text" name="url" placeholder="https://...">
        </div>
        <div>
          <label class="text-xs text-slate-400">veya dosya yükle (zip/7z/rar/exe)</label>
          <input type="file" name="file" class="text-xs text-slate-400 mt-2 block">
          <div class="text-xs text-amber-300/80 mt-1">Dosya yüklersen bağlantı yok sayılır. Hostingin yükleme sınırı (upload_max_filesize) küçükse büyük patch için bağlantı kullan.</div>
        </div>
      </div>
      <div class="grid sm:grid-cols-2 gap-3">
        <input class="input" type="text" name="size_mb" placeholder="Boyut (örn. 32 MB, isteğe bağlı)" maxlength="20">
        <label class="flex items-center gap-2 text-sm text-slate-400"><input type="checkbox" name="active" value="1" checked> Hemen yayınla</label>
      </div>
      <button type="submit" class="btn-primary">Ekle</button>
    </form>
  </div>

  <div class="space-y-3">
    <?php foreach ($patches as $p): $isLatest = (int)$p['id'] === $latestId; ?>
      <div class="card p-4 flex flex-wrap items-center justify-between gap-4 <?= $isLatest ? 'border-gold-500/60' : '' ?>">
        <div class="min-w-0">
          <div class="font-medium text-white">
            Sürüm <?= jstko_e($p['version']) ?>
            <?php if ($isLatest): ?><span class="ml-2 text-xs px-2 py-0.5 rounded-full bg-gold-500/20 text-gold-400">SON PATCH</span><?php endif; ?>
            <?= jstko_pg_bool($p['active']) ? '' : '<span class="ml-2 text-xs text-slate-500">(gizli)</span>' ?>
          </div>
          <div class="text-xs text-slate-500 mt-1 truncate"><?= jstko_e($p['title']) ?> <?= jstko_time_ago($p['created_at']) ?> · <a class="text-sky-400 hover:underline" href="<?= jstko_e($p['url']) ?>" target="_blank" rel="noopener"><?= jstko_e($p['url']) ?></a></div>
        </div>
        <div class="flex gap-2 shrink-0">
          <?php if (!$isLatest): ?>
          <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="make_latest"><input type="hidden" name="id" value="<?= (int)$p['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-gold-500/20 text-gold-400 hover:bg-gold-500/30">Son patch yap</button>
          </form>
          <?php endif; ?>
          <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="toggle"><input type="hidden" name="id" value="<?= (int)$p['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= jstko_pg_bool($p['active']) ? 'Gizle' : 'Yayınla' ?></button>
          </form>
          <form method="post" onsubmit="return confirm('Silinsin mi?')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="id" value="<?= (int)$p['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button>
          </form>
        </div>
      </div>
    <?php endforeach; ?>
    <?php if (!$patches): ?><div class="card p-10 text-center text-slate-500">Henüz patch eklenmedi.</div><?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/../partials/footer.php'; ?>
