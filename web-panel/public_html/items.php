<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Droplar';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

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

/** Full drop list for one npc_template's item_table (s_item), expanding MAKE_ITEM_GROUP
 * slots with the correct per-item weight (matches the server's resolve_drop_item). */
function jstko_monster_drops(PDO $pdo, int $sIndex, bool $isMonster): array
{
    $table = $isMonster ? 'monster_item' : 'npc_item';
    $stmt = $pdo->prepare("SELECT * FROM {$table} WHERE s_index = :idx");
    $stmt->execute(['idx' => $sIndex]);
    $row = $stmt->fetch();
    if (!$row) return [];

    $slots = jstko_row_slots($row);
    if (!$slots) return [];

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
        } elseif ($code >= 100) {
            // No group with this code: a literal item id (code < 100 with no matching
            // group is a level/nation production code we can't preview statically).
            $acc[$code] = ($acc[$code] ?? 0) + $pct;
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
            'str_name' => $it['str_name'] ?? null,
            'item_icon_id1' => $it['item_icon_id1'] ?? null,
            'pct' => $pct,
        ];
    }
    // Drop items we can't resolve a name for are internal codes, not useful to show.
    $out = array_values(array_filter($out, fn($d) => $d['str_name'] !== null));
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
    // PDO stringifies a bound PHP bool via (string) cast -- true -> "1" (Postgres accepts
    // it) but false -> "" (Postgres rejects it: "invalid input syntax for type boolean").
    // Pass the Postgres literal explicitly instead of relying on PDO's implicit cast.
    $stmt->execute(['sid' => $sSid, 'mon' => $isMonster ? 'true' : 'false']);
    return array_column($stmt->fetchAll(), 'zone_name');
}

$q = trim((string)($_GET['q'] ?? ''));
$mobToken = trim((string)($_GET['mob'] ?? '')); // "<s_sid>:<m|n>"

$mob = null;
$mobDrops = [];
$mobZones = [];

if ($mobToken !== '' && preg_match('/^(\d+):([mn])$/', $mobToken, $mm)) {
    $mobSid = (int)$mm[1];
    $isMonster = $mm[2] === 'm';
    $stmt = $pdo->prepare(
        "SELECT nt.s_sid, nt.str_name, nt.s_level, nt.s_item
         FROM npc_template nt
         WHERE nt.s_sid = :sid AND nt.is_monster = :mon
           AND EXISTS (SELECT 1 FROM npc_spawn ns WHERE ns.npc_id = nt.s_sid AND ns.is_monster = nt.is_monster)"
    );
    $stmt->execute(['sid' => $mobSid, 'mon' => $isMonster ? 'true' : 'false']);
    $mob = $stmt->fetch() ?: null;
    if ($mob) {
        $sItem = (int)$mob['s_item'];
        $mobDrops = $sItem > 0 ? jstko_monster_drops($pdo, $sItem, $isMonster) : [];
        $mobZones = jstko_npc_zones($pdo, $mobSid, $isMonster);
    }
}

$mobResults = [];
if ($q !== '' && !$mob) {
    // Only creatures that actually spawn somewhere -- a template with no npc_spawn row
    // isn't in the game right now, so there'd be nothing useful to show for it.
    $stmt = $pdo->prepare(
        "SELECT DISTINCT nt.s_sid, nt.str_name, nt.s_level, nt.is_monster
         FROM npc_template nt
         WHERE nt.str_name ILIKE :q
           AND EXISTS (SELECT 1 FROM npc_spawn ns WHERE ns.npc_id = nt.s_sid AND ns.is_monster = nt.is_monster)
         ORDER BY nt.str_name, nt.s_level LIMIT 80"
    );
    $stmt->execute(['q' => '%' . $q . '%']);
    $mobResults = $stmt->fetchAll();
}

// Batch-fetch each result's spawn zone(s) in one query per creature/NPC split, instead of
// re-querying per row.
$resultZones = [];
if ($mobResults) {
    foreach (['m' => true, 'n' => false] as $suffix => $isMonster) {
        $sids = array_values(array_unique(array_map('intval', array_column(
            array_filter($mobResults, fn($r) => jstko_pg_bool($r['is_monster']) === $isMonster), 's_sid'
        ))));
        if (!$sids) continue;
        $placeholders = implode(',', array_fill(0, count($sids), '?'));
        $stmt = $pdo->prepare(
            "SELECT ns.npc_id, z.zone_name
             FROM npc_spawn ns JOIN zone_info z ON z.zone_no = ns.zone_id
             WHERE ns.npc_id IN ($placeholders) AND ns.is_monster = ?
             GROUP BY ns.npc_id, z.zone_name ORDER BY z.zone_name"
        );
        $stmt->execute(array_merge($sids, [$isMonster ? 'true' : 'false']));
        foreach ($stmt->fetchAll() as $row) {
            $key = $row['npc_id'] . ':' . $suffix;
            $resultZones[$key] = $resultZones[$key] ?? [];
            $resultZones[$key][] = $row['zone_name'];
        }
    }
}
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Droplar</h1>
  <p class="text-sm text-slate-400 mb-6">Yaratık adıyla ara, hangi eşyaları ne oranla düşürdüğünü gör.</p>

  <form method="get" class="flex gap-2 mb-8">
    <input type="text" name="q" value="<?= jstko_e($q) ?>" placeholder="Yaratık adı yaz..." autofocus
           class="flex-1 bg-ink-700 border border-ink-500 rounded-lg px-4 py-2.5 text-sm text-white placeholder:text-slate-500 focus:outline-none focus:border-gold-400">
    <button type="submit" class="px-5 py-2.5 rounded-lg bg-gold-500 text-ink-900 font-semibold text-sm hover:bg-gold-400 transition">Ara</button>
  </form>

  <?php if ($mob): ?>
    <a href="/items.php<?= $q !== '' ? '?q=' . urlencode($q) : '' ?>" class="text-sm text-gold-400 hover:underline mb-4 inline-block">&larr; Arama sonuçlarına dön</a>
    <div class="card p-6 mb-8">
      <h2 class="text-xl font-bold text-white"><?= jstko_e($mob['str_name'] ?: 'Yaratık') ?></h2>
      <p class="text-xs text-slate-500 mt-0.5">Lv<?= (int)$mob['s_level'] ?></p>
      <?php if ($mobZones): ?>
        <div class="flex flex-wrap gap-1.5 mt-3">
          <?php foreach ($mobZones as $z): ?>
            <span class="text-xs px-2 py-1 rounded bg-ink-700 border border-ink-500 text-slate-300"><?= jstko_e($z) ?></span>
          <?php endforeach; ?>
        </div>
      <?php endif; ?>
    </div>

    <h3 class="text-lg font-semibold text-white mb-3">Drop Listesi</h3>
    <?php if (!$mobDrops): ?>
      <p class="text-sm text-slate-500">Bu yaratığın drop listesi tanımlı değil ya da tüm slotlar seviye/ulus bazlı üretim kodu (önizlenemiyor).</p>
    <?php else: ?>
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
        <?php foreach ($mobDrops as $d): ?>
        <div class="card p-3 flex items-center gap-3">
          <img src="<?= jstko_e(jstko_item_icon_url($d['item_icon_id1'] ?: null)) ?>" alt="" class="w-10 h-10 rounded border border-ink-500 bg-ink-700 object-contain shrink-0">
          <div class="min-w-0">
            <div class="text-sm font-medium text-white truncate"><?= jstko_e($d['str_name']) ?></div>
            <div class="text-xs text-gold-400 font-semibold"><?= jstko_pct_text((float)$d['pct']) ?></div>
          </div>
        </div>
        <?php endforeach; ?>
      </div>
    <?php endif; ?>

  <?php elseif ($q !== ''): ?>
    <?php if (!$mobResults): ?>
      <p class="text-sm text-slate-500">"<?= jstko_e($q) ?>" ile eşleşen, haritada şu anda bulunan bir yaratık yok.</p>
    <?php else: ?>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
        <?php foreach ($mobResults as $r):
            $suffix = jstko_pg_bool($r['is_monster']) ? 'm' : 'n';
            $token = (int)$r['s_sid'] . ':' . $suffix;
            $zones = $resultZones[(int)$r['s_sid'] . ':' . $suffix] ?? [];
        ?>
        <a href="/items.php?mob=<?= urlencode($token) ?><?= $q !== '' ? '&q=' . urlencode($q) : '' ?>"
           class="card p-3 flex items-center justify-between hover:border-gold-400 border border-transparent transition">
          <span class="text-sm font-medium text-white"><?= jstko_e($r['str_name'] ?: 'Yaratık') ?></span>
          <span class="text-xs text-slate-500">Lv<?= (int)$r['s_level'] ?> &middot; <?= jstko_e($zones ? implode(', ', $zones) : '-') ?></span>
        </a>
        <?php endforeach; ?>
      </div>
    <?php endif; ?>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
