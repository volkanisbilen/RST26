<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Itemler';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

/** Icon filename the client uses for an item_icon_id1, mirrors the admin panel's resolver
 * (itemicon_<lead>_<dddd>_<dd>_<d>.jpg -- last 7 digits split 4/2/1 behind the leading digits). */
function jstko_item_icon_candidates(int $id): array
{
    $build = function (int $v): ?string {
        $s = (string)$v;
        $n = strlen($s);
        if ($n < 8) return null;
        $lead = substr($s, 0, $n - 7);
        $a = substr($s, $n - 7, 4);
        $b = substr($s, $n - 3, 2);
        $c = substr($s, $n - 1, 1);
        return "itemicon_{$lead}_{$a}_{$b}_{$c}.jpg";
    };
    $out = [];
    if ($f = $build($id)) $out[] = $f;
    if ($f = $build(intdiv($id, 10) * 10)) $out[] = $f;
    return $out;
}

function jstko_item_icon_url(?int $iconId): string
{
    if (!$iconId) return '/assets/itemicon/noicon.png';
    foreach (jstko_item_icon_candidates($iconId) as $file) {
        if (is_file(__DIR__ . '/assets/itemicon/' . $file)) {
            return '/assets/itemicon/' . $file;
        }
    }
    return '/assets/itemicon/noicon.png';
}

$q = trim((string)($_GET['q'] ?? ''));
$detailId = isset($_GET['id']) ? (int)$_GET['id'] : 0;

$detail = null;
$droppedBy = [];
if ($detailId > 0) {
    $stmt = $pdo->prepare('SELECT * FROM item WHERE num = :id');
    $stmt->execute(['id' => $detailId]);
    $detail = $stmt->fetch() ?: null;

    if ($detail) {
        // Codes a monster/NPC drop slot can reference for this item: the item id itself
        // (if it's a literal-id drop, num >= 100000000), or any MAKE_ITEM_GROUP that
        // contains this item (admin-created groups can be any id, including < 100).
        $codes = [$detailId];
        $stmt = $pdo->prepare('SELECT group_num FROM make_item_group WHERE :id = ANY(items)');
        $stmt->execute(['id' => $detailId]);
        foreach ($stmt->fetchAll() as $row) $codes[] = (int)$row['group_num'];
        $codes = array_values(array_unique($codes));

        if ($codes) {
            $placeholders = implode(',', array_fill(0, count($codes), '?'));
            $sql = "
                SELECT nt.s_sid, nt.str_name, nt.s_level, MAX(slot.pct) pct, true is_monster
                FROM monster_item mi
                JOIN npc_template nt ON nt.item_table = mi.s_index AND nt.is_monster = true
                CROSS JOIN LATERAL (VALUES
                    (mi.item01, mi.percent01), (mi.item02, mi.percent02), (mi.item03, mi.percent03),
                    (mi.item04, mi.percent04), (mi.item05, mi.percent05), (mi.item06, mi.percent06),
                    (mi.item07, mi.percent07), (mi.item08, mi.percent08), (mi.item09, mi.percent09),
                    (mi.item10, mi.percent10), (mi.item11, mi.percent11), (mi.item12, mi.percent12)
                ) AS slot(item_id, pct)
                WHERE slot.item_id IN ($placeholders)
                GROUP BY nt.s_sid, nt.str_name, nt.s_level
                UNION ALL
                SELECT nt.s_sid, nt.str_name, nt.s_level, MAX(slot.pct) pct, false is_monster
                FROM npc_item ni
                JOIN npc_template nt ON nt.item_table = ni.s_index AND nt.is_monster = false
                CROSS JOIN LATERAL (VALUES
                    (ni.item01, ni.percent01), (ni.item02, ni.percent02), (ni.item03, ni.percent03),
                    (ni.item04, ni.percent04), (ni.item05, ni.percent05), (ni.item06, ni.percent06),
                    (ni.item07, ni.percent07), (ni.item08, ni.percent08), (ni.item09, ni.percent09),
                    (ni.item10, ni.percent10), (ni.item11, ni.percent11), (ni.item12, ni.percent12)
                ) AS slot(item_id, pct)
                WHERE slot.item_id IN ($placeholders)
                GROUP BY nt.s_sid, nt.str_name, nt.s_level
                ORDER BY pct DESC, s_level ASC
                LIMIT 200
            ";
            $stmt = $pdo->prepare($sql);
            $stmt->execute(array_merge($codes, $codes));
            $droppedBy = $stmt->fetchAll();
        }
    }
}

$results = [];
if ($q !== '' && !$detail) {
    $stmt = $pdo->prepare(
        "SELECT num, str_name, item_icon_id1, kind, req_level
         FROM item WHERE str_name ILIKE :q ORDER BY str_name LIMIT 100"
    );
    $stmt->execute(['q' => '%' . $q . '%']);
    $results = $stmt->fetchAll();
}
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Item Arama</h1>
  <p class="text-sm text-slate-400 mb-6">Item adıyla ara, hangi canavar/NPC'den düştüğünü gör.</p>

  <form method="get" class="flex gap-2 mb-8">
    <input type="text" name="q" value="<?= jstko_e($q) ?>" placeholder="Item adı yaz..." autofocus
           class="flex-1 bg-ink-700 border border-ink-500 rounded-lg px-4 py-2.5 text-sm text-white placeholder:text-slate-500 focus:outline-none focus:border-gold-400">
    <button type="submit" class="px-5 py-2.5 rounded-lg bg-gold-500 text-ink-900 font-semibold text-sm hover:bg-gold-400 transition">Ara</button>
  </form>

  <?php if ($detail): ?>
    <a href="/items.php<?= $q !== '' ? '?q=' . urlencode($q) : '' ?>" class="text-sm text-gold-400 hover:underline mb-4 inline-block">&larr; Arama sonuçlarına dön</a>
    <div class="card p-6 mb-8 flex gap-5 items-start">
      <img src="<?= jstko_e(jstko_item_icon_url($detail['item_icon_id1'] ?: null)) ?>" alt="" class="w-16 h-16 rounded border border-ink-500 bg-ink-700 object-contain shrink-0">
      <div>
        <h2 class="text-xl font-bold text-white"><?= jstko_e($detail['str_name'] ?: ('Item #' . $detail['num'])) ?></h2>
        <p class="text-xs text-slate-500 mt-0.5">#<?= (int)$detail['num'] ?></p>
        <div class="grid grid-cols-2 sm:grid-cols-3 gap-x-6 gap-y-1 mt-3 text-sm text-slate-300">
          <?php if (!empty($detail['req_level'])): ?><div>Gerekli Seviye: <span class="text-white"><?= (int)$detail['req_level'] ?></span></div><?php endif; ?>
          <?php if (!empty($detail['damage'])): ?><div>Hasar: <span class="text-white"><?= (int)$detail['damage'] ?></span></div><?php endif; ?>
          <?php if (!empty($detail['ac'])): ?><div>Savunma: <span class="text-white"><?= (int)$detail['ac'] ?></span></div><?php endif; ?>
          <?php if (!empty($detail['weight'])): ?><div>Ağırlık: <span class="text-white"><?= (int)$detail['weight'] ?></span></div><?php endif; ?>
        </div>
        <?php if (!empty($detail['description'])): ?><p class="text-sm text-slate-400 mt-3"><?= jstko_e($detail['description']) ?></p><?php endif; ?>
      </div>
    </div>

    <h3 class="text-lg font-semibold text-white mb-3">Hangi canavar/NPC'den düşer</h3>
    <?php if (!$droppedBy): ?>
      <p class="text-sm text-slate-500">Bu item hiçbir canavar/NPC'nin drop listesinde bulunamadı (quest ödülü, satın alma veya etkinlik ödülü olabilir).</p>
    <?php else: ?>
      <div class="card overflow-x-auto">
        <table class="w-full text-sm">
          <thead class="text-left text-slate-500 text-xs uppercase bg-ink-600/50">
            <tr><th class="py-3 px-4">Canavar/NPC</th><th class="py-3 px-4">Seviye</th><th class="py-3 px-4">Tür</th><th class="py-3 px-4 text-right">Düşme Oranı</th></tr>
          </thead>
          <tbody>
            <?php foreach ($droppedBy as $m): $pct = (int)$m['pct']; ?>
            <tr class="table-row border-t border-ink-500/50">
              <td class="py-2.5 px-4 font-medium text-white"><?= jstko_e($m['str_name'] ?: ('#' . $m['s_sid'])) ?></td>
              <td class="py-2.5 px-4 text-slate-300">Lv<?= (int)$m['s_level'] ?></td>
              <td class="py-2.5 px-4 text-slate-400"><?= jstko_pg_bool($m['is_monster']) ? 'Canavar' : 'NPC' ?></td>
              <td class="py-2.5 px-4 text-right text-gold-400 font-semibold"><?= $pct >= 10000 ? '%100' : ($pct % 100 === 0 ? '%' . intdiv($pct, 100) : '%' . number_format($pct / 100, 2)) ?></td>
            </tr>
            <?php endforeach; ?>
          </tbody>
        </table>
      </div>
    <?php endif; ?>

  <?php elseif ($q !== ''): ?>
    <?php if (!$results): ?>
      <p class="text-sm text-slate-500">"<?= jstko_e($q) ?>" için sonuç bulunamadı.</p>
    <?php else: ?>
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
        <?php foreach ($results as $r): ?>
        <a href="/items.php?id=<?= (int)$r['num'] ?><?= $q !== '' ? '&q=' . urlencode($q) : '' ?>"
           class="card p-3 flex items-center gap-3 hover:border-gold-400 border border-transparent transition">
          <img src="<?= jstko_e(jstko_item_icon_url($r['item_icon_id1'] ?: null)) ?>" alt="" class="w-10 h-10 rounded border border-ink-500 bg-ink-700 object-contain shrink-0">
          <div class="min-w-0">
            <div class="text-sm font-medium text-white truncate"><?= jstko_e($r['str_name'] ?: ('#' . $r['num'])) ?></div>
            <?php if (!empty($r['req_level'])): ?><div class="text-xs text-slate-500">Lv<?= (int)$r['req_level'] ?>+</div><?php endif; ?>
          </div>
        </a>
        <?php endforeach; ?>
      </div>
    <?php endif; ?>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
