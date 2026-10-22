<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Online Oyuncular';
require __DIR__ . '/partials/header.php';

$rows = jstko_db()->query(
    "SELECT c.str_char_id name, u.nation, u.class, u.level, u.rebirth_level, k.id_name clan
     FROM currentuser c LEFT JOIN userdata u ON lower(u.str_user_id) = lower(c.str_char_id)
     LEFT JOIN knights k ON k.id_num = u.knights
     ORDER BY u.level DESC NULLS LAST, c.str_char_id LIMIT 500"
)->fetchAll();
$karus = $elmo = 0;
foreach ($rows as $r) { if ((int)$r['nation'] === 1) $karus++; elseif ((int)$r['nation'] === 2) $elmo++; }
?>
<section class="max-w-6xl mx-auto px-4 py-10">
  <div class="flex flex-wrap items-end justify-between gap-4 mb-6">
    <div>
      <h1>Online Oyuncular</h1>
      <p class="text-sm text-slate-400">Şu anda oyunda olan karakterler. Liste kendiliğinden yenilenmez, sayfayı yenile.</p>
    </div>
    <div class="flex flex-wrap gap-2">
      <span class="stat-pill"><span class="live-dot"></span><b><?= jstko_number(count($rows)) ?></b> oyuncu</span>
      <span class="stat-pill"><span class="chip-nation karus">Karus</span><b><?= $karus ?></b></span>
      <span class="stat-pill"><span class="chip-nation elmo">El Morad</span><b><?= $elmo ?></b></span>
    </div>
  </div>

  <div class="flex flex-wrap gap-3 mb-5">
    <input id="olq" type="text" placeholder="Karakter veya klan ara..." class="flex-1 min-w-[200px] bg-ink-700 border border-ink-500 rounded-lg px-4 py-2.5 text-sm text-white placeholder:text-slate-500">
    <div class="flex gap-2" id="olnat">
      <button type="button" class="tab-pill on" data-n="0">Hepsi</button>
      <button type="button" class="tab-pill" data-n="1">Karus</button>
      <button type="button" class="tab-pill" data-n="2">El Morad</button>
    </div>
  </div>

  <div class="card overflow-x-auto">
    <table class="w-full text-sm">
      <thead class="text-left text-slate-500 text-xs uppercase">
        <tr><th class="py-3 px-4">Karakter</th><th class="py-3 px-4">Irk</th><th class="py-3 px-4">Sınıf</th><th class="py-3 px-4">Seviye</th><th class="py-3 px-4">Klan</th></tr>
      </thead>
      <tbody id="oltb">
        <?php foreach ($rows as $r): $n = (int)$r['nation']; $cls = $n === 1 ? 'karus' : 'elmo'; ?>
        <tr class="border-t border-gold-600/10" data-n="<?= $n ?>" data-s="<?= jstko_e(mb_strtolower($r['name'] . ' ' . ($r['clan'] ?? ''))) ?>">
          <td class="py-2.5 px-4"><span class="pl-name"><span class="pl-avatar <?= $cls ?>"><?= jstko_e(mb_strtoupper(mb_substr($r['name'], 0, 1))) ?></span><?= jstko_e($r['name']) ?><span class="dot-on" title="Online"></span></span></td>
          <td class="py-2.5 px-4"><span class="chip-nation <?= $cls ?>"><?= jstko_e(jstko_nation_name($n)) ?></span></td>
          <td class="py-2.5 px-4"><span class="cls-chip"><?= jstko_e(jstko_class_name((int)$r['class'])) ?></span></td>
          <td class="py-2.5 px-4"><span class="chip-lvl">Lv <?= (int)$r['level'] ?><?= $r['rebirth_level'] ? ' / ' . (int)$r['rebirth_level'] : '' ?></span></td>
          <td class="py-2.5 px-4 text-slate-400"><?= jstko_e($r['clan'] ?: 'Klan Yok') ?></td>
        </tr>
        <?php endforeach; ?>
        <?php if (!$rows): ?><tr><td colspan="5" class="py-10 text-center text-slate-500">Şu anda oyunda kimse yok.</td></tr><?php endif; ?>
      </tbody>
    </table>
    <div id="olnone" class="hidden py-10 text-center text-slate-500 text-sm">Eşleşen oyuncu yok.</div>
  </div>
</section>
<script>
(function () {
  var q = document.getElementById('olq'), rows = document.querySelectorAll('#oltb tr[data-s]'), nat = 0, none = document.getElementById('olnone');
  function apply() {
    var t = q.value.trim().toLowerCase(), shown = 0;
    rows.forEach(function (tr) {
      var ok = (!nat || tr.dataset.n === String(nat)) && (!t || tr.dataset.s.indexOf(t) !== -1);
      tr.style.display = ok ? '' : 'none'; if (ok) shown++;
    });
    none.classList.toggle('hidden', shown > 0 || !rows.length);
  }
  q.addEventListener('input', apply);
  document.querySelectorAll('#olnat button').forEach(function (b) {
    b.addEventListener('click', function () {
      nat = +b.dataset.n;
      document.querySelectorAll('#olnat button').forEach(function (x) { x.classList.toggle('on', x === b); });
      apply();
    });
  });
})();
</script>
<?php require __DIR__ . '/partials/footer.php'; ?>
