<?php
require_once __DIR__ . '/../includes/bootstrap.php';
require_once __DIR__ . '/../includes/admin_upload.php';
$user = jstko_require_gm();

// two lists share the table: "news" = Haberler & Duyurular, "update" = Güncelleme Notları
$kinds = ['news' => 'Haberler & Duyurular', 'update' => 'Güncelleme Notları'];
$kind = ($_GET['kind'] ?? $_POST['kind'] ?? 'news') === 'update' ? 'update' : 'news';

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
                $title = trim((string)($_POST['title'] ?? ''));
                $body = trim((string)($_POST['body'] ?? ''));
                if ($title === '' || $body === '') {
                    throw new RuntimeException('Başlık ve içerik gerekli.');
                }
                $image = jstko_admin_save_upload('image', jstko_image_spec('news')) ?? '';
                $stmt = $pdo->prepare(
                    'INSERT INTO web_news (title, body, image, published, created_by, kind) VALUES (:t, :b, :i, :p, :c, :k)'
                );
                $stmt->execute([
                    't' => $title, 'b' => $body, 'i' => $image,
                    'p' => !empty($_POST['published']) ? 'true' : 'false', 'c' => $user['str_account_id'], 'k' => $kind,
                ]);
                $notice = 'Eklendi.';
            } elseif ($action === 'edit') {
                $id = (int)($_POST['id'] ?? 0);
                $title = trim((string)($_POST['title'] ?? ''));
                $body = trim((string)($_POST['body'] ?? ''));
                $newKind = ($_POST['new_kind'] ?? $kind) === 'update' ? 'update' : 'news';
                if ($title === '' || $body === '') {
                    throw new RuntimeException('Başlık ve içerik gerekli.');
                }
                $image = jstko_admin_save_upload('image', jstko_image_spec('news'));
                if ($image !== null) {
                    $pdo->prepare('UPDATE web_news SET title = :t, body = :b, kind = :k, image = :i WHERE id = :id')
                        ->execute(['t' => $title, 'b' => $body, 'k' => $newKind, 'i' => $image, 'id' => $id]);
                } else {
                    $pdo->prepare('UPDATE web_news SET title = :t, body = :b, kind = :k WHERE id = :id')
                        ->execute(['t' => $title, 'b' => $body, 'k' => $newKind, 'id' => $id]);
                }
                $notice = $newKind !== $kind ? 'Kaydedildi, yazı "' . $kinds[$newKind] . '" listesine taşındı.' : 'Kaydedildi.';
            } elseif ($action === 'toggle') {
                $id = (int)($_POST['id'] ?? 0);
                $pdo->prepare('UPDATE web_news SET published = NOT published WHERE id = :id')->execute(['id' => $id]);
                $notice = 'Yayın durumu değiştirildi.';
            } elseif ($action === 'delete') {
                $id = (int)($_POST['id'] ?? 0);
                $pdo->prepare('DELETE FROM web_news WHERE id = :id')->execute(['id' => $id]);
                $notice = 'Silindi.';
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$st = jstko_db()->prepare('SELECT * FROM web_news WHERE kind = :k ORDER BY created_at DESC');
$st->execute(['k' => $kind]);
$news = $st->fetchAll();

$pageTitle = $kinds[$kind];
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
$label = $kind === 'update' ? 'Güncelleme Notu' : 'Haber / Duyuru';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1><?= jstko_e($kinds[$kind]) ?></h1>
  <p class="text-sm text-slate-400 mb-6">
    <?= $kind === 'update'
        ? 'Anasayfadaki "Güncelleme Notları" listesi ve /news.php?kind=update sayfası buradan beslenir.'
        : 'Anasayfadaki "Haberler & Duyurular" kartları ve /news.php?kind=news sayfası buradan beslenir.' ?>
  </p>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3">Yeni <?= jstko_e($label) ?> Ekle</h2>
    <form method="post" enctype="multipart/form-data" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="create">
      <input type="hidden" name="kind" value="<?= $kind ?>">
      <input class="input" type="text" name="title" placeholder="Başlık" required maxlength="200">
      <textarea class="input" name="body" rows="4" placeholder="İçerik" required></textarea>
      <?= jstko_image_input('image', 'news') ?>
      <p class="text-xs text-slate-500">Resim koymazsan sitede tema renklerinde degrade bir görsel gösterilir.</p>
      <label class="flex items-center gap-2 text-sm text-slate-400"><input type="checkbox" name="published" value="1" checked> Hemen yayınla</label>
      <button type="submit" class="btn-primary">Ekle</button>
    </form>
  </div>

  <div class="space-y-3">
    <?php foreach ($news as $n): ?>
      <div class="card p-4">
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <div class="font-medium text-white truncate"><?= jstko_e($n['title']) ?></div>
            <div class="text-xs text-slate-500 mt-1"><?= jstko_time_ago($n['created_at']) ?> · <?= jstko_pg_bool($n['published']) ? '<span class="text-emerald-400">Yayında</span>' : '<span class="text-slate-500">Taslak</span>' ?></div>
          </div>
          <div class="flex gap-2 shrink-0">
            <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="toggle"><input type="hidden" name="kind" value="<?= $kind ?>"><input type="hidden" name="id" value="<?= (int)$n['id'] ?>">
              <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= jstko_pg_bool($n['published']) ? 'Yayından Kaldır' : 'Yayınla' ?></button>
            </form>
            <form method="post" onsubmit="return confirm('Silinsin mi?')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="kind" value="<?= $kind ?>"><input type="hidden" name="id" value="<?= (int)$n['id'] ?>">
              <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button>
            </form>
          </div>
        </div>
        <details class="mt-3">
          <summary class="text-xs text-gold-400 cursor-pointer select-none">Düzenle / listeyi değiştir</summary>
          <form method="post" enctype="multipart/form-data" class="space-y-3 mt-3">
            <?= jstko_csrf_field() ?>
            <input type="hidden" name="form_action" value="edit">
            <input type="hidden" name="kind" value="<?= $kind ?>">
            <input type="hidden" name="id" value="<?= (int)$n['id'] ?>">
            <input class="input" type="text" name="title" value="<?= jstko_e($n['title']) ?>" required maxlength="200">
            <textarea class="input" name="body" rows="5" required><?= jstko_e($n['body']) ?></textarea>
            <label class="block text-xs text-slate-400">Liste
              <select name="new_kind" class="input mt-1">
                <?php foreach ($kinds as $k => $kl): ?><option value="<?= $k ?>" <?= $n['kind'] === $k ? 'selected' : '' ?>><?= jstko_e($kl) ?></option><?php endforeach; ?>
              </select>
            </label>
            <?php if ($n['image']): ?><img src="/uploads/<?= jstko_e(ltrim($n['image'], '/')) ?>" class="h-16 rounded border border-ink-500" alt=""><?php endif; ?>
            <?= jstko_image_input('image', 'news') ?>
            <button type="submit" class="btn-primary">Kaydet</button>
          </form>
        </details>
      </div>
    <?php endforeach; ?>
    <?php if (!$news): ?><div class="card p-10 text-center text-slate-500">Bu listede henüz yazı yok.</div><?php endif; ?>
  </div>
</section>
<?= jstko_image_check_script() ?>
<?php require __DIR__ . '/../partials/footer.php'; ?>
