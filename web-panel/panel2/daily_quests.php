<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Günlük Görevler';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();

$quests = $pdo->query(
    "SELECT dq.*,
            z.zone_name,
            m1.str_name mob1_name, m2.str_name mob2_name, m3.str_name mob3_name, m4.str_name mob4_name,
            i1.str_name r1_name, i1.item_icon_id1 r1_icon,
            i2.str_name r2_name, i2.item_icon_id1 r2_icon,
            i3.str_name r3_name, i3.item_icon_id1 r3_icon,
            i4.str_name r4_name, i4.item_icon_id1 r4_icon
     FROM daily_quests dq
     LEFT JOIN zone_info z ON z.zone_no = dq.zone_id
     LEFT JOIN npc_template m1 ON m1.s_sid = dq.mob_id_1 AND m1.is_monster = true
     LEFT JOIN npc_template m2 ON m2.s_sid = dq.mob_id_2 AND m2.is_monster = true
     LEFT JOIN npc_template m3 ON m3.s_sid = dq.mob_id_3 AND m3.is_monster = true
     LEFT JOIN npc_template m4 ON m4.s_sid = dq.mob_id_4 AND m4.is_monster = true
     LEFT JOIN item i1 ON i1.num = dq.reward_1
     LEFT JOIN item i2 ON i2.num = dq.reward_2
     LEFT JOIN item i3 ON i3.num = dq.reward_3
     LEFT JOIN item i4 ON i4.num = dq.reward_4
     ORDER BY dq.id"
)->fetchAll();

?>
<section class="max-w-5xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Günlük Görevler</h1>
  <p class="text-sm text-slate-400 mb-6">Her gün tekrarlanan görevler ve verdikleri ödüller.</p>

  <div class="grid gap-4">
    <?php foreach ($quests as $q):
        $mobs = array_filter([$q['mob1_name'], $q['mob2_name'], $q['mob3_name'], $q['mob4_name']]);
        $rewards = [];
        for ($i = 1; $i <= 4; $i++) {
            if (!empty($q["reward_{$i}"]) && !empty($q["r{$i}_name"])) {
                $rewards[] = ['id' => (int)$q["reward_{$i}"], 'name' => $q["r{$i}_name"], 'icon' => $q["r{$i}_icon"], 'count' => (int)$q["count_{$i}"]];
            }
        }
    ?>
    <div class="card p-5">
      <div class="flex items-start justify-between gap-4 flex-wrap">
        <div>
          <h2 class="text-lg font-semibold text-white"><?= jstko_e($q['quest_name'] ?: ('Görev #' . $q['id'])) ?></h2>
          <p class="text-sm text-slate-400 mt-1">
            <?php if ($mobs): ?>
              <?= jstko_e(implode(', ', $mobs)) ?> &times; <?= (int)$q['kill_count'] ?> adet öldür
            <?php endif; ?>
            <?php if (!empty($q['zone_name'])): ?> &middot; <span class="text-slate-500"><?= jstko_e($q['zone_name']) ?></span><?php endif; ?>
          </p>
          <?php if (!empty($q['min_level']) || !empty($q['max_level'])): ?>
            <p class="text-xs text-slate-500 mt-1">Seviye <?= (int)$q['min_level'] ?: 1 ?>-<?= (int)$q['max_level'] ?: 83 ?></p>
          <?php endif; ?>
        </div>
      </div>
      <?php if ($rewards): ?>
        <div class="flex flex-wrap gap-2 mt-4">
          <?php foreach ($rewards as $r): ?>
          <div data-item="<?= (int)$r['id'] ?>" class="item-card !p-1.5 !gap-2">
            <img src="<?= jstko_e(jstko_item_icon_url($r['icon'])) ?>" alt="" class="item-ico !w-8 !h-8">
            <div class="text-xs">
              <div class="text-white font-medium"><?= jstko_e($r['name']) ?></div>
              <div class="text-slate-500">&times;<?= $r['count'] ?: 1 ?></div>
            </div>
          </div>
          <?php endforeach; ?>
        </div>
      <?php endif; ?>
    </div>
    <?php endforeach; ?>
    <?php if (!$quests): ?><p class="text-sm text-slate-500">Tanımlı günlük görev yok.</p><?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
