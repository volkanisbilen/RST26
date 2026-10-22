<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Craft';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

$npcs = [19073 => 'Shozin (Silah / Zırh / Draki)', 31402 => 'Kuyumcu (Takı)'];
$npc = (int)($_GET['npc'] ?? 19073);
if (!isset($npcs[$npc])) $npc = 19073;
$q = trim((string)($_GET['q'] ?? ''));

$stmt = $pdo->prepare("SELECT * FROM item_special_sewing WHERE npc_id = :n AND give_item_id > 0 ORDER BY description, n_index");
$stmt->execute(['n' => $npc]);
$rows = $stmt->fetchAll();

// item names + icons for everything the recipes mention
$ids = [];
foreach ($rows as $r) {
    $ids[(int)$r['give_item_id']] = true;
    for ($i = 1; $i <= 10; $i++) if ((int)$r["req_item_id_$i"] > 0) $ids[(int)$r["req_item_id_$i"]] = true;
}
$items = [];
if ($ids) {
    $keys = array_keys($ids);
    $stmt = $pdo->prepare("SELECT num, str_name, item_icon_id1 FROM item WHERE num IN (" . implode(',', array_fill(0, count($keys), '?')) . ")");
    $stmt->execute($keys);
    foreach ($stmt->fetchAll() as $it) $items[(int)$it['num']] = $it;
}

// one card per material set; the possible results (class variants) are listed inside
$groups = [];
foreach ($rows as $r) {
    $mats = [];
    for ($i = 1; $i <= 10; $i++) {
        $id = (int)$r["req_item_id_$i"];
        if ($id > 0) $mats[] = [$id, (int)$r["req_item_count_$i"]];
    }
    usort($mats, fn($a, $b) => $a[0] <=> $b[0]);
    $key = json_encode($mats);
    if (!isset($groups[$key])) $groups[$key] = ['desc' => (string)$r['description'], 'mats' => $mats, 'results' => []];
    $groups[$key]['results'][] = ['id' => (int)$r['give_item_id'], 'count' => (int)$r['give_item_count'], 'rate' => (int)$r['success_rate']];
}

if ($q !== '') {
    $needle = mb_strtolower($q);
    $groups = array_filter($groups, function ($g) use ($items, $needle) {
        if (mb_stripos($g['desc'], $needle) !== false) return true;
        foreach ($g['mats'] as [$id]) if (mb_stripos((string)($items[$id]['str_name'] ?? ''), $needle) !== false) return true;
        foreach ($g['results'] as $res) if (mb_stripos((string)($items[$res['id']]['str_name'] ?? ''), $needle) !== false) return true;
        return false;
    });
}

function jstko_craft_pct(int $rate): string
{
    $p = $rate / 100;
    return '%' . (abs($p - round($p)) < 0.005 ? (string)(int)round($p) : number_format($p, 2));
}
function jstko_craft_icon(array $items, int $id): string
{
    return jstko_e(jstko_item_icon_url(($items[$id]['item_icon_id1'] ?? null) ?: null));
}
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Craft</h1>
  <p class="text-sm text-slate-400 mb-6">Moradon'daki Shozin ve Kuyumcu'nun birleştirme tarifleri: hangi malzemeler birleştirilince ne elde edilir ve başarı oranı.</p>

  <div class="flex flex-wrap gap-2 mb-4">
    <?php foreach ($npcs as $id => $label): ?>
      <a href="<?= JSTKO_BASE ?>/craft.php?npc=<?= $id ?>" class="px-4 py-2 rounded-lg text-sm font-semibold border transition <?= $id === $npc ? 'bg-gold-500 text-ink-900 border-gold-500' : 'bg-ink-700 border-ink-500 text-slate-300 hover:border-gold-400' ?>"><?= jstko_e($label) ?></a>
    <?php endforeach; ?>
  </div>

  <form method="get" class="flex gap-2 mb-8">
    <input type="hidden" name="npc" value="<?= $npc ?>">
    <input type="text" name="q" value="<?= jstko_e($q) ?>" placeholder="Eşya adı yaz (malzeme veya sonuç)..."
           class="flex-1 bg-ink-700 border border-ink-500 rounded-lg px-4 py-2.5 text-sm text-white placeholder:text-slate-500 focus:outline-none focus:border-gold-400">
    <button type="submit" class="px-5 py-2.5 rounded-lg bg-gold-500 text-ink-900 font-semibold text-sm hover:bg-gold-400 transition">Ara</button>
  </form>

  <?php if (!$groups): ?>
    <p class="text-sm text-slate-500">Eşleşen tarif yok.</p>
  <?php else: ?>
    <p class="text-xs text-slate-500 mb-3"><?= count($groups) ?> tarif</p>
    <div class="space-y-4">
      <?php foreach ($groups as $g): ?>
        <div class="card p-4">
          <?php if ($g['desc'] !== ''): ?><div class="text-sm font-semibold text-gold-400 mb-3"><?= jstko_e($g['desc']) ?></div><?php endif; ?>
          <div class="text-xs text-slate-500 mb-2">Gerekli malzemeler (sıra önemli değil)</div>
          <div class="flex flex-wrap gap-3 mb-4">
            <?php foreach ($g['mats'] as [$id, $cnt]): ?>
              <div data-item="<?= (int)$id ?>" class="item-card !p-1.5 !gap-2">
                <img src="<?= jstko_craft_icon($items, $id) ?>" alt="" class="item-ico !w-9 !h-9">
                <div class="text-sm text-white"><?= jstko_e($items[$id]['str_name'] ?? ('#' . $id)) ?> <span class="text-gold-400 font-semibold">×<?= $cnt ?></span></div>
              </div>
            <?php endforeach; ?>
          </div>
          <div class="text-xs text-slate-500 mb-2"><?= count($g['results']) > 1 ? 'Olası sonuçlar (sınıfına uygun olan verilir)' : 'Sonuç' ?></div>
          <div class="grid sm:grid-cols-2 gap-2">
            <?php foreach ($g['results'] as $res): ?>
              <div data-item="<?= (int)$res['id'] ?>" class="item-card">
                <img src="<?= jstko_craft_icon($items, $res['id']) ?>" alt="" class="item-ico">
                <div class="min-w-0 flex-1">
                  <div class="text-sm font-medium text-white truncate"><?= jstko_e($items[$res['id']]['str_name'] ?? ('#' . $res['id'])) ?><?= $res['count'] > 1 ? ' ×' . $res['count'] : '' ?></div>
                  <div class="text-xs text-gold-400 font-semibold">Başarı oranı: <?= jstko_craft_pct($res['rate']) ?></div>
                </div>
              </div>
            <?php endforeach; ?>
          </div>
        </div>
      <?php endforeach; ?>
    </div>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
