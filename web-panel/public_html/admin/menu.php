<?php
require_once __DIR__ . '/../includes/bootstrap.php';
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
                $label = trim((string)($_POST['label'] ?? ''));
                $url = trim((string)($_POST['url'] ?? ''));
                $parent = (int)($_POST['parent_id'] ?? 0);
                if ($label === '') {
                    throw new RuntimeException('Menü yazısı gerekli.');
                }
                $pdo->prepare('INSERT INTO web_menu (parent_id, label, url, sort_order, active) VALUES (:p, :l, :u, :o, TRUE)')->execute([
                    'p' => $parent > 0 ? $parent : null, 'l' => $label, 'u' => $url, 'o' => (int)($_POST['sort_order'] ?? 0),
                ]);
                $notice = 'Menü öğesi eklendi.';
            } elseif ($action === 'update') {
                $pdo->prepare('UPDATE web_menu SET label=:l, url=:u, sort_order=:o WHERE id=:id')->execute([
                    'l' => trim((string)($_POST['label'] ?? '')), 'u' => trim((string)($_POST['url'] ?? '')),
                    'o' => (int)($_POST['sort_order'] ?? 0), 'id' => (int)($_POST['id'] ?? 0),
                ]);
                $notice = 'Kaydedildi.';
            } elseif ($action === 'toggle') {
                $pdo->prepare('UPDATE web_menu SET active = NOT active WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Durum değiştirildi.';
            } elseif ($action === 'delete') {
                $pdo->prepare('DELETE FROM web_menu WHERE id = :id')->execute(['id' => (int)($_POST['id'] ?? 0)]);
                $notice = 'Menü öğesi silindi.';
            }
        } catch (Throwable $e) {
            $error = $e->getMessage();
        }
    }
}

$all = jstko_cms_rows('SELECT * FROM web_menu ORDER BY sort_order, id');
$tops = array_values(array_filter($all, fn($r) => $r['parent_id'] === null));
$pages = jstko_cms_rows('SELECT slug, title FROM web_page WHERE published ORDER BY title');

$pageTitle = 'Menü';
require __DIR__ . '/../partials/header.php';
require __DIR__ . '/_nav.php';

function jstko_menu_row(array $r, bool $child): void
{
    ?>
    <div class="card p-3 <?= $child ? 'ml-8' : '' ?>">
      <form method="post" class="flex flex-wrap items-center gap-2">
        <?= jstko_csrf_field() ?>
        <input type="hidden" name="form_action" value="update">
        <input type="hidden" name="id" value="<?= (int)$r['id'] ?>">
        <input class="input w-40" type="text" name="label" value="<?= jstko_e($r['label']) ?>">
        <input class="input flex-1 min-w-[160px]" type="text" name="url" value="<?= jstko_e($r['url']) ?>" placeholder="<?= $child ? 'adres' : 'adres (açılır menü ise boş bırak)' ?>">
        <input class="input w-20" type="number" name="sort_order" value="<?= (int)$r['sort_order'] ?>" title="Sıra">
        <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500">Kaydet</button>
      </form>
      <div class="flex gap-2 mt-2">
        <form method="post"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="toggle"><input type="hidden" name="id" value="<?= (int)$r['id'] ?>">
          <button type="submit" class="text-xs px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500"><?= jstko_pg_bool($r['active']) ? 'Menüden çıkar (gizle)' : 'Menüye geri koy' ?></button></form>
        <form method="post" onsubmit="return confirm('Silinsin mi?<?= $child ? '' : ' Altındaki öğeler de silinir.' ?>')"><?= jstko_csrf_field() ?><input type="hidden" name="form_action" value="delete"><input type="hidden" name="id" value="<?= (int)$r['id'] ?>">
          <button type="submit" class="text-xs px-3 py-1.5 rounded bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60">Sil</button></form>
        <?php if (!jstko_pg_bool($r['active'])): ?><span class="text-xs text-slate-500 self-center">şu an gizli</span><?php endif; ?>
      </div>
    </div>
    <?php
}
?>
<section class="max-w-3xl mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold text-white mb-1">Üst Menü</h1>
  <p class="text-sm text-slate-400 mb-6">Sitenin üstündeki menüyü buradan düzenlersin. Sıra küçükten büyüğe dizilir. Üst öğeyi "açılır menü" yapmak için adresini boş bırak ve altına öğe ekle.</p>
  <?php if ($notice): ?><div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div><?php endif; ?>
  <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($error) ?></div><?php endif; ?>

  <div class="card p-6 mb-8">
    <h2 class="font-semibold text-white mb-3">Yeni Menü Öğesi</h2>
    <form method="post" class="space-y-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="form_action" value="create">
      <div class="grid sm:grid-cols-2 gap-3">
        <input class="input" type="text" name="label" placeholder="Menüde görünen yazı" required maxlength="60">
        <input class="input" type="text" name="url" id="menuUrl" placeholder="Adres (örn. /patches.php veya https://...)">
        <select class="input" id="menuPage">
          <option value="">— hazır sayfa seç —</option>
          <?php foreach ($pages as $p): ?><option value="/page.php?slug=<?= jstko_e($p['slug']) ?>"><?= jstko_e($p['title']) ?></option><?php endforeach; ?>
          <option value="/patches.php">Patch İndir</option>
        </select>
        <select class="input" name="parent_id">
          <option value="0">Üst seviye</option>
          <?php foreach ($tops as $t): ?><option value="<?= (int)$t['id'] ?>"><?= jstko_e($t['label']) ?> altında</option><?php endforeach; ?>
        </select>
        <input class="input" type="number" name="sort_order" placeholder="Sıra (örn. 45)" value="100">
      </div>
      <button type="submit" class="btn-primary">Ekle</button>
    </form>
  </div>

  <div class="space-y-3">
    <?php foreach ($tops as $t): ?>
      <?php jstko_menu_row($t, false); ?>
      <?php foreach ($all as $c) { if ((int)($c['parent_id'] ?? 0) === (int)$t['id']) { jstko_menu_row($c, true); } } ?>
    <?php endforeach; ?>
    <?php if (!$tops): ?><div class="card p-10 text-center text-slate-500">Menü boş; site yerleşik menüyü gösterir.</div><?php endif; ?>
  </div>
</section>
<script>
document.getElementById('menuPage').addEventListener('change', function () { if (this.value) document.getElementById('menuUrl').value = this.value; });
</script>
<?php require __DIR__ . '/../partials/footer.php'; ?>
