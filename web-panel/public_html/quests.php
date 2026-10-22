<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Görevler';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

$q = trim((string)($_GET['q'] ?? ''));
$nation = (int)($_GET['nation'] ?? 0);          // 1 Karus, 2 El Morad
$lvl = (int)($_GET['lvl'] ?? 0);                // character level: shows quests up to it
$page = max(1, (int)($_GET['p'] ?? 1));
$per = 20;

$where = ['h.s_event_data_index > 0'];
$args = [];
if ($q !== '') {
    $where[] = '(nt.str_name ILIKE :q OR t.str_talk ILIKE :q)';
    $args['q'] = '%' . $q . '%';
}
if ($nation === 1 || $nation === 2) $where[] = 'h.b_nation IN (' . $nation . ', 3, 0)';
if ($lvl > 0) $where[] = 'h.b_level <= ' . $lvl;
$cond = implode(' AND ', $where);

$base = "SELECT DISTINCT ON (h.s_event_data_index)
            h.s_event_data_index q, h.b_level lvl, h.b_nation nation, h.b_zone zone, h.s_npc_id npc,
            h.n_exchange_index ex, nt.str_name npc_name, z.zone_name, t.str_talk
         FROM quest_helper h
         LEFT JOIN npc_template nt ON nt.s_sid = h.s_npc_id AND nt.is_monster = false
         LEFT JOIN zone_info z ON z.zone_no = h.b_zone
         LEFT JOIN quest_talk t ON t.i_num = h.n_event_talk_index
         WHERE $cond
         ORDER BY h.s_event_data_index, h.b_event_status";

$stmt = $pdo->prepare("SELECT count(*) FROM ($base) x");
$stmt->execute($args);
$total = (int)$stmt->fetchColumn();
$pages = max(1, (int)ceil($total / $per));
$page = min($page, $pages);

$stmt = $pdo->prepare("SELECT * FROM ($base) x ORDER BY lvl, q LIMIT $per OFFSET " . (($page - 1) * $per));
$stmt->execute($args);
$quests = $stmt->fetchAll();

// kill targets and exchange rewards of this page
$qids = array_map(fn($r) => (int)$r['q'], $quests);
$exIds = array_values(array_filter(array_unique(array_map(fn($r) => (int)$r['ex'], $quests))));
$kills = [];
if ($qids) {
    $stmt = $pdo->query("SELECT * FROM quest_monster WHERE s_quest_num IN (" . implode(',', $qids) . ")");
    foreach ($stmt->fetchAll() as $m) $kills[(int)$m['s_quest_num']] = $m;
}
$exch = [];
if ($exIds) {
    $stmt = $pdo->query("SELECT * FROM item_exchange WHERE n_index IN (" . implode(',', $exIds) . ")");
    foreach ($stmt->fetchAll() as $e) $exch[(int)$e['n_index']] = $e;
}
$mobIds = [];
foreach ($kills as $m) for ($g = 1; $g <= 4; $g++) foreach (['a', 'b', 'c', 'd'] as $s) { $id = (int)$m["s_num{$g}{$s}"]; if ($id > 0) $mobIds[$id] = true; }
$itemIds = [];
foreach ($exch as $e) for ($i = 1; $i <= 5; $i++) { $id = (int)$e["exchange_item_num$i"]; if ($id > 0) $itemIds[$id] = true; }
$mobs = [];
if ($mobIds) {
    $stmt = $pdo->query("SELECT s_sid, str_name FROM npc_template WHERE is_monster = true AND s_sid IN (" . implode(',', array_keys($mobIds)) . ")");
    foreach ($stmt->fetchAll() as $r) $mobs[(int)$r['s_sid']] = $r['str_name'];
}
$items = [];
if ($itemIds) {
    $stmt = $pdo->query("SELECT num, str_name, item_icon_id1 FROM item WHERE num IN (" . implode(',', array_keys($itemIds)) . ")");
    foreach ($stmt->fetchAll() as $r) $items[(int)$r['num']] = $r;
}

function jstko_quest_targets(?array $m, array $mobs): array
{
    $out = [];
    if (!$m) return $out;
    for ($g = 1; $g <= 4; $g++) {
        $cnt = (int)$m["s_count$g"];
        if ($cnt <= 0) continue;
        $names = [];
        foreach (['a', 'b', 'c', 'd'] as $s) {
            $id = (int)$m["s_num{$g}{$s}"];
            if ($id > 0) $names[] = $mobs[$id] ?? ('#' . $id);
        }
        if ($names) $out[] = implode(' / ', $names) . ' ×' . $cnt;
    }
    return $out;
}
function jstko_quest_url(array $over): string
{
    $p = array_merge($_GET, $over);
    return '/quests.php?' . http_build_query(array_filter($p, fn($v) => $v !== '' && $v !== 0 && $v !== null));
}
$nationName = [1 => 'Karus', 2 => 'El Morad', 3 => 'Her iki ırk', 0 => 'Her iki ırk'];
?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Görevler</h1>
  <p class="text-sm text-slate-400 mb-6">Oyundaki görev NPC'leri, seviye şartları, öldürülecek yaratıklar ve ödüller. NPC adı veya görev metniyle ara.</p>

  <form method="get" class="flex flex-wrap gap-2 mb-6">
    <input type="text" name="q" value="<?= jstko_e($q) ?>" placeholder="NPC adı veya görev metni..."
           class="flex-1 min-w-[200px] bg-ink-700 border border-ink-500 rounded-lg px-4 py-2.5 text-sm text-white placeholder:text-slate-500 focus:outline-none focus:border-gold-400">
    <select name="nation" class="bg-ink-700 border border-ink-500 rounded-lg px-3 py-2.5 text-sm text-white">
      <option value="0">Tüm ırklar</option>
      <option value="1" <?= $nation === 1 ? 'selected' : '' ?>>Karus</option>
      <option value="2" <?= $nation === 2 ? 'selected' : '' ?>>El Morad</option>
    </select>
    <input type="number" name="lvl" value="<?= $lvl ?: '' ?>" min="1" max="83" placeholder="Seviyem"
           class="w-28 bg-ink-700 border border-ink-500 rounded-lg px-3 py-2.5 text-sm text-white placeholder:text-slate-500">
    <button type="submit" class="px-5 py-2.5 rounded-lg bg-gold-500 text-ink-900 font-semibold text-sm hover:bg-gold-400 transition">Ara</button>
  </form>

  <p class="text-xs text-slate-500 mb-3"><?= $total ?> görev<?= $pages > 1 ? " · sayfa $page / $pages" : '' ?></p>
  <?php if (!$quests): ?>
    <p class="text-sm text-slate-500">Eşleşen görev yok.</p>
  <?php else: ?>
    <div class="space-y-3">
      <?php foreach ($quests as $r):
          $targets = jstko_quest_targets($kills[(int)$r['q']] ?? null, $mobs);
          $ex = $exch[(int)$r['ex']] ?? null;
          $rewards = [];
          if ($ex) for ($i = 1; $i <= 5; $i++) {
              $id = (int)$ex["exchange_item_num$i"];
              if ($id > 0) $rewards[] = ['id' => $id, 'count' => (int)$ex["exchange_item_count$i"]];
          }
          $talk = trim((string)$r['str_talk']);
          if (mb_strlen($talk) > 260) $talk = mb_substr($talk, 0, 260) . '…';
      ?>
      <div class="card p-4">
        <div class="flex flex-wrap items-center gap-2 mb-2">
          <span class="text-base font-semibold text-white"><?= jstko_e($r['npc_name'] ?: ('NPC #' . (int)$r['npc'])) ?></span>
          <span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-gold-400">Lv<?= (int)$r['lvl'] ?>+</span>
          <span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-slate-300"><?= jstko_e($nationName[(int)$r['nation']] ?? 'Her iki ırk') ?></span>
          <?php if ($r['zone_name']): ?><span class="text-xs px-2 py-0.5 rounded bg-ink-700 border border-ink-500 text-slate-300"><?= jstko_e($r['zone_name']) ?></span><?php endif; ?>
        </div>
        <?php if ($talk !== ''): ?><p class="text-sm text-slate-400 mb-3"><?= jstko_e($talk) ?></p><?php endif; ?>
        <?php if ($targets): ?>
          <div class="text-xs text-slate-500 mb-1">Hedef</div>
          <div class="flex flex-wrap gap-1.5 mb-3">
            <?php foreach ($targets as $t): ?><span class="text-xs px-2 py-1 rounded bg-ink-700 border border-ink-500 text-white"><?= jstko_e($t) ?></span><?php endforeach; ?>
          </div>
        <?php endif; ?>
        <?php if ($rewards): ?>
          <div class="text-xs text-slate-500 mb-1">Ödül</div>
          <div class="flex flex-wrap gap-2">
            <?php foreach ($rewards as $w): $it = $items[$w['id']] ?? null; ?>
              <div class="flex items-center gap-2 bg-ink-700 border border-ink-500 rounded-lg px-2 py-1">
                <img src="<?= jstko_e(jstko_item_icon_url(($it['item_icon_id1'] ?? null) ?: null)) ?>" alt="" class="w-8 h-8 rounded border border-ink-500 bg-ink-800 object-contain">
                <span class="text-sm text-white"><?= jstko_e($it['str_name'] ?? ('#' . $w['id'])) ?> <span class="text-gold-400 font-semibold">×<?= $w['count'] ?></span></span>
              </div>
            <?php endforeach; ?>
          </div>
        <?php endif; ?>
      </div>
      <?php endforeach; ?>
    </div>
    <?php if ($pages > 1): ?>
      <div class="flex items-center justify-center gap-2 mt-6">
        <?php if ($page > 1): ?><a href="<?= jstko_e(jstko_quest_url(['p' => $page - 1])) ?>" class="px-4 py-2 rounded-lg bg-ink-700 border border-ink-500 text-sm hover:border-gold-400">&larr; Önceki</a><?php endif; ?>
        <span class="text-sm text-slate-400"><?= $page ?> / <?= $pages ?></span>
        <?php if ($page < $pages): ?><a href="<?= jstko_e(jstko_quest_url(['p' => $page + 1])) ?>" class="px-4 py-2 rounded-lg bg-ink-700 border border-ink-500 text-sm hover:border-gold-400">Sonraki &rarr;</a><?php endif; ?>
      </div>
    <?php endif; ?>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
