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
                $title = trim((string)($_POST['title'] ?? ''));
                $body = trim((string)($_POST['body'] ?? ''));
                if ($title === '' || $body === '') {
                    throw new RuntimeException('Başlık ve içerik gerekli.');
                }
                $image = jstko_admin_save_upload('image') ?? '';
                $stmt = $pdo->prepare(
                    'INSERT INTO web_news (title, body, image, published, created_by) VALUES (:t, :b, :i, :p, :c)'
                );
                $stmt->execute([
                    't' => $title, 'b' => $body, 'i' => $image,
                    'p' => !empty($_POST['published']), 'c' => $user['str_account_id'],
                ]);
                $notice = 'Haber eklendi.';
            } elseif ($action === 'toggle') {
                $id = (int)($_POST['id'] ?? 0);
                $pdo->prepare('UPDATE web_news SET published = NOT published WHERE id = :id')->execute(['id' => $id]);
                $notice = 'Yayın durumu değiştirildi.';
            } elseif ($action === 'delete') {
                $id = (int)($_POST['id'] ?? 0);
                $pdo->prepare('DELETE FROM web_news WHERE id = :id')->execute(['id' => $id]);
                $notice = 'Haber silindi.';
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$news = jstko_db()->query('SELECT * FROM web_news ORDER BY created_at DESC')->fetchAll();

$pageTitle = 'Haberler';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-6">Güncelleme Notları / Haberler</h1>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3">Yeni Haber Ekle</h2>
    <form method="post" enctype="multipart/form-data" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="create">
      <input class="input" type="text" name="title" placeholder="Başlık" required maxlength="200">
      <textarea class="input" name="body" rows="4" placeholder="İçerik" required></textarea>
      <input type="file" name="image" accept="image/*" class="text-xs text-slate-400">
      <label class="flex items-center gap-2 text-sm text-slate-400"><input type="checkbox" name="published" value="1" checked> Hemen yayınla</label>
      <button type="submit" class="btn-primary">Ekle</button>
    </form>
  </div>

  <div class="space-y-3">
    <?php foreach ($news as $n): ?>
      <div class="card p-4 flex items-center justify-between gap-4">
        <div>
          <div class="font-medium text-white"><?= jstko_e($n['title']) ?></div>
          <div class="text-xs text-slate-500 mt-1"><?= jstko_time_ago($n['created_at']) ?> · <?= jstko_pg_bool($n['published']) ? '<span class="text-emerald-400">Yayında</span>' : '<span class="text-slate-500">Taslak</span>' ?></div>
        </div>
        <div class="flex gap-2 shrink-0">
          <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="toggle"><input type="hidden" name="id" value="<?= (int)$n['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= jstko_pg_bool($n['published']) ? 'Yayından Kaldır' : 'Yayınla' ?></button>
          </form>
          <form method="post" onsubmit="return confirm('Silinsin mi?')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="id" value="<?= (int)$n['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button>
          </form>
        </div>
      </div>
    <?php endforeach; ?>
    <?php if (!$news): ?><div class="card p-10 text-center text-slate-500">Henüz haber yok.</div><?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/../partials/footer.php'; ?>
