<?php
require_once __DIR__ . '/../includes/bootstrap.php';
require_once __DIR__ . '/../includes/admin_upload.php';
$user = jstko_require_gm();

$notice = null;
$error = null;
$editId = (int)($_GET['edit'] ?? 0);

function jstko_slugify(string $s): string
{
    $s = strtr(mb_strtolower($s, 'UTF-8'), ['ç' => 'c', 'ğ' => 'g', 'ı' => 'i', 'ö' => 'o', 'ş' => 's', 'ü' => 'u']);
    return trim(preg_replace('/[^a-z0-9]+/', '-', $s), '-');
}

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } else {
        try {
            $action = $_POST['form_action'] ?? '';
            $pdo = jstko_db();
            if ($action === 'save') {
                $id = (int)($_POST['id'] ?? 0);
                $title = trim((string)($_POST['title'] ?? ''));
                $slug = jstko_slugify((string)($_POST['slug'] ?? '') !== '' ? (string)$_POST['slug'] : $title);
                if ($title === '' || $slug === '') {
                    throw new RuntimeException('Başlık gerekli.');
                }
                $body = (string)($_POST['body'] ?? '');
                $published = !empty($_POST['published']) ? 'true' : 'false';
                $img = jstko_admin_save_upload('header_image', jstko_image_spec('page_header'));
                if ($id > 0) {
                    $pdo->prepare('UPDATE web_page SET slug=:s, title=:t, body=:b, published=:p, updated_at=now() WHERE id=:id')
                        ->execute(['s' => $slug, 't' => $title, 'b' => $body, 'p' => $published, 'id' => $id]);
                    if ($img !== null) {
                        $pdo->prepare('UPDATE web_page SET header_image=:i WHERE id=:id')->execute(['i' => $img, 'id' => $id]);
                    }
                    $notice = 'Sayfa kaydedildi.';
                } else {
                    $pdo->prepare('INSERT INTO web_page (slug, title, body, header_image, published) VALUES (:s, :t, :b, :i, :p)')
                        ->execute(['s' => $slug, 't' => $title, 'b' => $body, 'i' => $img ?? '', 'p' => $published]);
                    $notice = 'Sayfa oluşturuldu: /page.php?slug=' . $slug . ' — menüye eklemek için "Menü" sekmesini kullan.';
                }
            } elseif ($action === 'delete') {
                $pdo->prepare('DELETE FROM web_page WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Sayfa silindi.';
            }
        } catch (PDOException $e) {
            $error = str_contains($e->getMessage(), 'web_page_slug_key') ? 'Bu adres (slug) zaten kullanımda.' : $e->getMessage();
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$pages = jstko_cms_rows('SELECT id, slug, title, published, updated_at FROM web_page ORDER BY title');
$edit = null;
if ($editId > 0) {
    $r = jstko_cms_rows('SELECT * FROM web_page WHERE id = :id', ['id' => $editId]);
    $edit = $r[0] ?? null;
}

$pageTitle = 'Sayfalar';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-1">Sayfalar</h1>
  <p class="text-sm text-slate-400 mb-6">Kendi sayfalarını yaz (kurallar, rehber, hakkında...). İçerik HTML kabul eder: <code class="text-gold-400">&lt;h2&gt;</code>, <code class="text-gold-400">&lt;p&gt;</code>, <code class="text-gold-400">&lt;img&gt;</code>, <code class="text-gold-400">&lt;a&gt;</code>. Yalnızca GM'ler düzenleyebilir.</p>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3"><?= $edit ? 'Sayfayı Düzenle' : 'Yeni Sayfa' ?></h2>
    <form method="post" enctype="multipart/form-data" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="save">
      <input type="hidden" name="id" value="<?= (int)($edit['id'] ?? 0) ?>">
      <div class="grid sm:grid-cols-2 gap-3">
        <input class="input" type="text" name="title" placeholder="Başlık" required maxlength="160" value="<?= jstko_e($edit['title'] ?? '') ?>">
        <input class="input" type="text" name="slug" placeholder="adres (örn. kurallar) — boşsa başlıktan üretilir" maxlength="60" value="<?= jstko_e($edit['slug'] ?? '') ?>">
      </div>
      <textarea class="input font-mono" name="body" rows="14" placeholder="<h2>Başlık</h2>&#10;<p>Metin...</p>"><?= jstko_e($edit['body'] ?? '') ?></textarea>
      <?php if (!empty($edit['header_image'])): ?><img src="/uploads/<?= jstko_e($edit['header_image']) ?>" class="max-h-24 rounded border border-ink-500" alt=""><?php endif; ?>
      <?= jstko_image_input('header_image', 'page_header') ?>
      <label class="flex items-center gap-2 text-sm text-slate-400"><input type="checkbox" name="published" value="1" <?= !$edit || jstko_pg_bool($edit['published']) ? 'checked' : '' ?>> Yayında</label>
      <div class="flex gap-2">
        <button type="submit" class="btn-primary"><?= $edit ? 'Kaydet' : 'Oluştur' ?></button>
        <?php if ($edit): ?><a href="/admin/pages.php" class="px-4 py-2 rounded bg-ink-600 hover:bg-ink-500 text-sm">Vazgeç</a><?php endif; ?>
      </div>
    </form>
  </div>

  <div class="space-y-3">
    <?php foreach ($pages as $p): ?>
      <div class="card p-4 flex items-center justify-between gap-4">
        <div>
          <div class="font-medium text-white"><?= jstko_e($p['title']) ?></div>
          <div class="text-xs text-slate-500 mt-1"><a class="text-sky-400 hover:underline" href="/page.php?slug=<?= jstko_e($p['slug']) ?>" target="_blank">/page.php?slug=<?= jstko_e($p['slug']) ?></a> · <?= jstko_pg_bool($p['published']) ? '<span class="text-emerald-400">Yayında</span>' : 'Taslak' ?></div>
        </div>
        <div class="flex gap-2 shrink-0">
          <a href="/admin/pages.php?edit=<?= (int)$p['id'] ?>" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500">Düzenle</a>
          <form method="post" onsubmit="return confirm('Silinsin mi?')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="id" value="<?= (int)$p['id'] ?>">
            <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button></form>
        </div>
      </div>
    <?php endforeach; ?>
    <?php if (!$pages): ?><div class="card p-10 text-center text-slate-500">Henüz sayfa yok.</div><?php endif; ?>
  </div>
</section>
<?= jstko_image_check_script() ?>
<?php require __DIR__ . '/../partials/footer.php'; ?>
