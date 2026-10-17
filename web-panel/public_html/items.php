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

/** Postgres int[] literal ("{1,2,3}") -> PHP int array. */
function jstko_pg_int_array(?string $raw): array
{
    if ($raw === null || $raw === '' || $raw === '{}') return [];
    $raw = trim($raw, '{}');
    if ($raw === '') return [];
    return array_map('intval', explode(',', $raw));
}

function jstko_pct_text(float $pct): string
{
    if ($pct >= 10000) return '%100';
    if (abs($pct - round($pct / 100) * 100) < 0.01) return '%' . (int)round($pct / 100);
    return '%' . number_format($pct / 100, 2);
}

/**
 * Codes a monster/NPC drop slot can carry for a given item: the item id itself
 * (literal-id drops, num >= 100000000, or anything really), plus every
 * MAKE_ITEM_GROUP that contains it — each weighted by how many of that
 * group's (up to 25) slots actually hold this item, out of the group's
 * non-zero slot count (matches the server's own resolve_drop_item weighting).
 * Returns code => [weight, total].
 */
function jstko_drop_codes_for_item(PDO $pdo, int $itemId): array
{
    $codes = [$itemId => [1, 1]];
    $stmt = $pdo->prepare('SELECT group_num, items FROM make_item_group WHERE :id = ANY(items)');
    $stmt->execute(['id' => $itemId]);
    foreach ($stmt->fetchAll() as $row) {
        $items = jstko_pg_int_array($row['items']);
        $nonzero = count(array_filter($items, fn($v) => $v > 0));
        $occ = count(array_filter($items, fn($v) => $v === $itemId));
        if ($nonzero > 0 && $occ > 0) {
            $codes[(int)$row['group_num']] = [$occ, $nonzero];
        }
    }
    return $codes;
}

/** All (item slot)->pct candidates a monster_item/npc_item row holds, as [code => pct]. */
function jstko_row_slots(array $row): array
{
    $out = [];
    for ($i = 1; $i <= 12; $i++) {
        $n = str_pad((string)$i, 2, '0', STR_PAD_LEFT);
        $code = (int)$row["item{$n}"];
        $pct = (int)$row["percent{$n}"];
        if ($code > 0 && $pct > 0) $out[] = [$code, $pct];
    }
    return $out;
}

/** Full drop list for one npc_template's item_table (s_item), expanding groups with proper weights. */
function jstko_monster_drops(PDO $pdo, int $sIndex, bool $isMonster): array
{
    $table = $isMonster ? 'monster_item' : 'npc_item';
    $stmt = $pdo->prepare("SELECT * FROM {$table} WHERE s_index = :idx");
    $stmt->execute(['idx' => $sIndex]);
    $row = $stmt->fetch();
    if (!$row) return [];

    $slots = jstko_row_slots($row);
    if (!$slots) return [];

    // Resolve each slot: direct item (code already a real item), or a group to expand.
    $groupCodes = array_unique(array_column($slots, 0));
    $placeholders = implode(',', array_fill(0, count($groupCodes), '?'));
    $stmt = $pdo->prepare("SELECT group_num, items FROM make_item_group WHERE group_num IN ($placeholders)");
    $stmt->execute($groupCodes);
    $groups = [];
    foreach ($stmt->fetchAll() as $g) $groups[(int)$g['group_num']] = jstko_pg_int_array($g['items']);

    $acc = []; // item_id => cumulative pct
    foreach ($slots as [$code, $pct]) {
        if (isset($groups[$code])) {
            $items = array_filter($groups[$code], fn($v) => $v > 0);
            $nonzero = count($items);
            if ($nonzero === 0) continue;
            $counts = array_count_values($items);
            foreach ($counts as $itemId => $occ) {
                $acc[$itemId] = ($acc[$itemId] ?? 0) + $pct * $occ / $nonzero;
            }
        } else {
            // No group with this code: it's either a literal item id, or (code < 100 with
            // no matching group) a level/nation production code we can't preview statically.
            if ($code >= 100) {
                $acc[$code] = ($acc[$code] ?? 0) + $pct;
            }
        }
    }
    if (!$acc) return [];

    $ids = array_keys($acc);
    $placeholders = implode(',', array_fill(0, count($ids), '?'));
    $stmt = $pdo->prepare("SELECT num, str_name, item_icon_id1 FROM item WHERE num IN ($placeholders)");
    $stmt->execute($ids);
    $names = [];
    foreach ($stmt->fetchAll() as $it) $names[(int)$it['num']] = $it;

    $out = [];
    foreach ($acc as $itemId => $pct) {
        $it = $names[$itemId] ?? null;
        $out[] = [
            'num' => $itemId,
            'str_name' => $it['str_name'] ?? ('Item #' . $itemId),
            'item_icon_id1' => $it['item_icon_id1'] ?? null,
            'pct' => $pct,
        ];
    }
    usort($out, fn($a, $b) => $b['pct'] <=> $a['pct']);
    return $out;
}

function jstko_npc_zones(PDO $pdo, int $sSid, bool $isMonster): array
{
    $stmt = $pdo->prepare(
        "SELECT DISTINCT z.zone_name
         FROM npc_spawn ns JOIN zone_info z ON z.zone_no = ns.zone_id
         WHERE ns.npc_id = :sid AND ns.is_monster = :mon
         ORDER BY z.zone_name"
    );
    $stmt->execute(['sid' => $sSid, 'mon' => $isMonster]);
    return array_column($stmt->fetchAll(), 'zone_name');
}

$q = trim((string)($_GET['q'] ?? ''));
$detailId = isset($_GET['id']) ? (int)$_GET['id'] : 0;
$mobSid = isset($_GET['mob']) ? (int)$_GET['mob'] : 0;
$mobKind = ($_GET['kind'] ?? 'monster') === 'npc' ? 'npc' : 'monster';

$detail = null;
$droppedBy = [];
$mob = null;
$mobDrops = [];
$mobZones = [];

if ($detailId > 0) {
    $stmt = $pdo->prepare('SELECT * FROM item WHERE num = :id');
    $stmt->execute(['id' => $detailId]);
    $detail = $stmt->fetch() ?: null;

    if ($detail) {
        // Codes any monster_item/npc_item slot could hold to produce this item: the item
        // id itself, or any MAKE_ITEM_GROUP containing it (each weighted by how much of
        // that group's slots are this item) -- only rows referencing one of these codes
        // are touched, not every monster's drop table.
        $codes = jstko_drop_codes_for_item($pdo, $detailId);
        $codeList = array_keys($codes);
        $placeholders = implode(',', array_fill(0, count($codeList), '?'));
        $sql = "
            SELECT nt.s_sid, nt.str_name, nt.s_level, true is_monster, slot.item_id, slot.pct
            FROM monster_item mi
            JOIN npc_template nt ON nt.s_item = mi.s_index AND nt.is_monster = true
            CROSS JOIN LATERAL (VALUES
                (mi.item01, mi.percent01), (mi.item02, mi.percent02), (mi.item03, mi.percent03),
                (mi.item04, mi.percent04), (mi.item05, mi.percent05), (mi.item06, mi.percent06),
                (mi.item07, mi.percent07), (mi.item08, mi.percent08), (mi.item09, mi.percent09),
                (mi.item10, mi.percent10), (mi.item11, mi.percent11), (mi.item12, mi.percent12)
            ) AS slot(item_id, pct)
            WHERE slot.item_id IN ($placeholders)
            UNION ALL
            SELECT nt.s_sid, nt.str_name, nt.s_level, false is_monster, slot.item_id, slot.pct
            FROM npc_item ni
            JOIN npc_template nt ON nt.s_item = ni.s_index AND nt.is_monster = false
            CROSS JOIN LATERAL (VALUES
                (ni.item01, ni.percent01), (ni.item02, ni.percent02), (ni.item03, ni.percent03),
                (ni.item04, ni.percent04), (ni.item05, ni.percent05), (ni.item06, ni.percent06),
                (ni.item07, ni.percent07), (ni.item08, ni.percent08), (ni.item09, ni.percent09),
                (ni.item10, ni.percent10), (ni.item11, ni.percent11), (ni.item12, ni.percent12)
            ) AS slot(item_id, pct)
            WHERE slot.item_id IN ($placeholders)
        ";
        $stmt = $pdo->prepare($sql);
        $stmt->execute(array_merge($codeList, $codeList));
        $acc = []; // "is_monster:s_sid" => accumulated pct + display info
        foreach ($stmt->fetchAll() as $row) {
            $code = (int)$row['item_id'];
            [$weight, $total] = $codes[$code] ?? [0, 1];
            if ($weight <= 0) continue;
            $pct = (int)$row['pct'] * $weight / $total;
            $key = $row['is_monster'] . ':' . $row['s_sid'];
            if (!isset($acc[$key])) {
                $acc[$key] = ['s_sid' => $row['s_sid'], 'str_name' => $row['str_name'], 's_level' => $row['s_level'], 'is_monster' => $row['is_monster'], 'pct' => 0.0];
            }
            $acc[$key]['pct'] += $pct;
        }
        $droppedBy = array_values($acc);
        usort($droppedBy, fn($a, $b) => $b['pct'] <=> $a['pct']);
    }
}

if ($mobSid > 0) {
    $isMonster = $mobKind === 'monster';
    $stmt = $pdo->prepare('SELECT s_sid, str_name, s_level FROM npc_template WHERE s_sid = :sid AND is_monster = :mon');
    $stmt->execute(['sid' => $mobSid, 'mon' => $isMonster]);
    $mob = $stmt->fetch() ?: null;
    if ($mob) {
        $tStmt = $pdo->prepare('SELECT s_item FROM npc_template WHERE s_sid = :sid AND is_monster = :mon');
        $tStmt->execute(['sid' => $mobSid, 'mon' => $isMonster]);
        $sItem = (int)$tStmt->fetchColumn();
        $mobDrops = $sItem > 0 ? jstko_monster_drops($pdo, $sItem, $isMonster) : [];
        $mobZones = jstko_npc_zones($pdo, $mobSid, $isMonster);
    }
}

$itemResults = [];
$mobResults = [];
if ($q !== '' && !$detail && !$mob) {
    $stmt = $pdo->prepare(
        "SELECT num, str_name, item_icon_id1, req_level
         FROM item WHERE str_name ILIKE :q ORDER BY str_name LIMIT 60"
    );
    $stmt->execute(['q' => '%' . $q . '%']);
    $itemResults = $stmt->fetchAll();

    $stmt = $pdo->prepare(
        "SELECT s_sid, str_name, s_level, is_monster
         FROM npc_template WHERE str_name ILIKE :q ORDER BY str_name, s_level LIMIT 60"
    );
    $stmt->execute(['q' => '%' . $q . '%']);
    $mobResults = $stmt->fetchAll();
}
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Item &amp; Canavar Arama</h1>
  <p class="text-sm text-slate-400 mb-6">Item veya canavar/NPC adıyla ara; hangi canavardan ne düştüğünü, canavarın hangi haritada olduğunu gör.</p>

  <form method="get" class="flex gap-2 mb-8">
    <input type="text" name="q" value="<?= jstko_e($q) ?>" placeholder="Item veya canavar adı yaz..." autofocus
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
            <?php foreach ($droppedBy as $m): ?>
            <tr class="table-row border-t border-ink-500/50">
              <td class="py-2.5 px-4 font-medium text-white">
                <a href="/items.php?mob=<?= (int)$m['s_sid'] ?>&kind=<?= jstko_pg_bool($m['is_monster']) ? 'monster' : 'npc' ?>" class="hover:text-gold-400"><?= jstko_e($m['str_name'] ?: ('#' . $m['s_sid'])) ?></a>
              </td>
              <td class="py-2.5 px-4 text-slate-300">Lv<?= (int)$m['s_level'] ?></td>
              <td class="py-2.5 px-4 text-slate-400"><?= jstko_pg_bool($m['is_monster']) ? 'Canavar' : 'NPC' ?></td>
              <td class="py-2.5 px-4 text-right text-gold-400 font-semibold"><?= jstko_pct_text((float)$m['pct']) ?></td>
            </tr>
            <?php endforeach; ?>
          </tbody>
        </table>
      </div>
    <?php endif; ?>

  <?php elseif ($mob): ?>
    <a href="/items.php<?= $q !== '' ? '?q=' . urlencode($q) : '' ?>" class="text-sm text-gold-400 hover:underline mb-4 inline-block">&larr; Arama sonuçlarına dön</a>
    <div class="card p-6 mb-8">
      <h2 class="text-xl font-bold text-white"><?= jstko_e($mob['str_name'] ?: ('#' . $mob['s_sid'])) ?></h2>
      <p class="text-xs text-slate-500 mt-0.5">#<?= (int)$mob['s_sid'] ?> &middot; Lv<?= (int)$mob['s_level'] ?> &middot; <?= $mobKind === 'monster' ? 'Canavar' : 'NPC' ?></p>
      <?php if ($mobZones): ?>
        <div class="flex flex-wrap gap-1.5 mt-3">
          <?php foreach ($mobZones as $z): ?>
            <span class="text-xs px-2 py-1 rounded bg-ink-700 border border-ink-500 text-slate-300"><?= jstko_e($z) ?></span>
          <?php endforeach; ?>
        </div>
      <?php else: ?>
        <p class="text-xs text-slate-500 mt-3">Haritada spawn kaydı bulunamadı.</p>
      <?php endif; ?>
    </div>

    <h3 class="text-lg font-semibold text-white mb-3">Drop Listesi</h3>
    <?php if (!$mobDrops): ?>
      <p class="text-sm text-slate-500">Drop listesi tanımlı değil ya da tüm slotlar seviye/ulus bazlı üretim kodu (önizlenemiyor).</p>
    <?php else: ?>
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
        <?php foreach ($mobDrops as $d): ?>
        <a href="/items.php?id=<?= (int)$d['num'] ?>" class="card p-3 flex items-center gap-3 hover:border-gold-400 border border-transparent transition">
          <img src="<?= jstko_e(jstko_item_icon_url($d['item_icon_id1'] ?: null)) ?>" alt="" class="w-10 h-10 rounded border border-ink-500 bg-ink-700 object-contain shrink-0">
          <div class="min-w-0">
            <div class="text-sm font-medium text-white truncate"><?= jstko_e($d['str_name']) ?></div>
            <div class="text-xs text-gold-400 font-semibold"><?= jstko_pct_text((float)$d['pct']) ?></div>
          </div>
        </a>
        <?php endforeach; ?>
      </div>
    <?php endif; ?>

  <?php elseif ($q !== ''): ?>
    <?php if (!$itemResults && !$mobResults): ?>
      <p class="text-sm text-slate-500">"<?= jstko_e($q) ?>" için sonuç bulunamadı.</p>
    <?php endif; ?>

    <?php if ($mobResults): ?>
      <h3 class="text-lg font-semibold text-white mb-3">Canavar / NPC Sonuçları</h3>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 mb-8">
        <?php foreach ($mobResults as $r): ?>
        <a href="/items.php?mob=<?= (int)$r['s_sid'] ?>&kind=<?= jstko_pg_bool($r['is_monster']) ? 'monster' : 'npc' ?><?= $q !== '' ? '&q=' . urlencode($q) : '' ?>"
           class="card p-3 flex items-center justify-between hover:border-gold-400 border border-transparent transition">
          <span class="text-sm font-medium text-white"><?= jstko_e($r['str_name'] ?: ('#' . $r['s_sid'])) ?></span>
          <span class="text-xs text-slate-500">Lv<?= (int)$r['s_level'] ?> &middot; <?= jstko_pg_bool($r['is_monster']) ? 'Canavar' : 'NPC' ?></span>
        </a>
        <?php endforeach; ?>
      </div>
    <?php endif; ?>

    <?php if ($itemResults): ?>
      <h3 class="text-lg font-semibold text-white mb-3">Item Sonuçları</h3>
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
        <?php foreach ($itemResults as $r): ?>
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
