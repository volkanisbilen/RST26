
</main>

<?php
$B = JSTKO_BASE;
$fLogo = jstko_c_image('logo_image');
$fName = jstko_c('site_name', 'JstKO');
$socials = [
    'discord_url'   => ['Discord',   '<path d="M20 5.5A17 17 0 0 0 15.8 4l-.5 1a15 15 0 0 0-4.6 0l-.5-1A17 17 0 0 0 4 5.5C1.700 9 1 12.400 1.300 15.700A17 17 0 0 0 6.400 18.300l1-1.600a11 11 0 0 1-1.700-.8l.4-.3a12 12 0 0 0 10.200 0l.4.300a11 11 0 0 1-1.700.8l1 1.600a17 17 0 0 0 5.100-2.600C23.100 11.800 22 8.400 20 5.500zM8.700 14c-1 0-1.800-.9-1.800-2s.8-2 1.800-2 1.800.9 1.800 2-.8 2-1.800 2zm6.600 0c-1 0-1.800-.9-1.800-2s.8-2 1.800-2 1.800.9 1.800 2-.8 2-1.800 2z"/>'],
    'youtube_url'   => ['YouTube',   '<path d="M21.600 7.200a2.500 2.500 0 0 0-1.800-1.800C18.200 5 12 5 12 5s-6.200 0-7.800.4A2.500 2.500 0 0 0 2.400 7.200C2 8.800 2 12 2 12s0 3.200.4 4.800a2.500 2.500 0 0 0 1.800 1.800C5.800 19 12 19 12 19s6.200 0 7.800-.4a2.500 2.500 0 0 0 1.800-1.800C22 15.200 22 12 22 12s0-3.200-.4-4.800zM10 15V9l5.200 3z"/>'],
    'facebook_url'  => ['Facebook',  '<path d="M14 8V6.500c0-.7.200-1 1.200-1H17V2.200C16.600 2.100 15.600 2 14.600 2 12.100 2 10.500 3.500 10.500 6.100V8H8v3.500h2.500V22H14V11.500h2.700L17.200 8z"/>'],
    'instagram_url' => ['Instagram', '<path d="M7.500 2h9A5.500 5.500 0 0 1 22 7.500v9a5.500 5.500 0 0 1-5.500 5.500h-9A5.500 5.500 0 0 1 2 16.500v-9A5.500 5.500 0 0 1 7.500 2zm0 2A3.500 3.500 0 0 0 4 7.500v9A3.500 3.500 0 0 0 7.500 20h9a3.500 3.500 0 0 0 3.500-3.500v-9A3.500 3.500 0 0 0 16.500 4zM12 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10zm0 2a3 3 0 1 0 0 6 3 3 0 0 0 0-6zm5.500-3.200a1.200 1.200 0 1 1 0 2.400 1.200 1.200 0 0 1 0-2.400z"/>'],
];
?>
<footer class="site-footer mt-10">
  <div class="mx-auto max-w-[1400px] px-4 lg:px-6 py-5 grid gap-4 md:grid-cols-3 items-center text-xs text-slate-400">
    <div class="flex items-center gap-3 justify-center md:justify-start">
      <?php if ($fLogo): ?><img src="<?= jstko_e($fLogo) ?>" alt="<?= jstko_e($fName) ?>" class="h-9 w-auto"><?php else: ?><span class="font-display font-extrabold text-xl text-gold-400"><?= jstko_e($fName) ?></span><?php endif; ?>
      <span class="hidden sm:inline">Legends Never Fade</span>
      <span class="hidden sm:inline text-slate-600">|</span>
      <span class="hidden sm:inline"><?= jstko_e($fName) ?> <?= date('Y') ?></span>
    </div>
    <div class="flex items-center justify-center gap-5">
      <?php foreach ($socials as $key => [$label, $path]): $url = jstko_c($key); if ($url === '') continue; ?>
        <a href="<?= jstko_e($url) ?>" target="_blank" rel="noopener" aria-label="<?= $label ?>" class="text-slate-300 hover:text-gold-400 transition">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><?= $path ?></svg>
        </a>
      <?php endforeach; ?>
    </div>
    <div class="flex flex-wrap items-center justify-center md:justify-end gap-x-4 gap-y-1">
      <span class="inline-flex items-center gap-1.5 text-emerald-400"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>Sunucu Online</span>
      <a href="<?= $B ?>/terms.php" class="hover:text-gold-400">Kullanım Şartları</a>
      <a href="<?= $B ?>/support.php" class="hover:text-gold-400">İletişim</a>
      <?php if ($ft = jstko_c('footer_text')): ?><span><?= jstko_e($ft) ?></span><?php endif; ?>
    </div>
  </div>
</footer>

<script>
document.addEventListener('DOMContentLoaded', function () {
  var btn = document.getElementById('menuBtn');
  var menu = document.getElementById('mobileMenu');
  if (btn && menu) {
    btn.addEventListener('click', function () {
      menu.classList.toggle('hidden');
    });
  }

  document.querySelectorAll('[data-toggle-password]').forEach(function (btn) {
    btn.addEventListener('click', function () {
      var input = document.getElementById(btn.getAttribute('data-toggle-password'));
      if (!input) return;
      input.type = input.type === 'password' ? 'text' : 'password';
    });
  });
});


(function () {
  var hd = document.querySelector('.site-header');
  function onScroll() { if (hd) hd.classList.toggle('scrolled', window.scrollY > 10); }
  onScroll(); window.addEventListener('scroll', onScroll, { passive: true });

  var rv = document.querySelectorAll('[data-reveal]');
  if ('IntersectionObserver' in window) {
    var io = new IntersectionObserver(function (es) { es.forEach(function (e) { if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); } }); }, { threshold: .15 });
    rv.forEach(function (el) { io.observe(el); });
  } else { rv.forEach(function (el) { el.classList.add('in'); }); }

  document.querySelectorAll('[data-count]').forEach(function (el) {
    var to = parseInt(el.getAttribute('data-count'), 10) || 0, t0 = null;
    if (!to || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    function step(t) {
      if (t0 === null) t0 = t;
      var p = Math.min(1, (t - t0) / 1400), v = Math.round(to * (1 - Math.pow(1 - p, 3)));
      el.textContent = v.toLocaleString('tr-TR');
      if (p < 1) requestAnimationFrame(step);
    }
    requestAnimationFrame(step);
  });
})();

(function () {
  var sel = 'main .card, main .panel';
  var els = document.querySelectorAll(sel), seen = new Map();
  els.forEach(function (el) {
    if (el.hasAttribute('data-reveal') || el.closest('[data-reveal]')) return;
    var p = el.parentElement, i = seen.get(p) || 0; seen.set(p, i + 1);
    el.setAttribute('data-reveal', ''); el.style.setProperty('--i', Math.min(i, 8));
    if (el.getBoundingClientRect().top < innerHeight) { /* already on screen: observer below shows it at once */ }
  });
  var rv = document.querySelectorAll('[data-reveal]:not(.in)');
  if ('IntersectionObserver' in window) {
    var io2 = new IntersectionObserver(function (es) { es.forEach(function (e) { if (e.isIntersecting) { e.target.classList.add('in'); io2.unobserve(e.target); } }); }, { threshold: .08 });
    rv.forEach(function (el) { io2.observe(el); });
  } else rv.forEach(function (el) { el.classList.add('in'); });
})();
window.JSTKO_BASE = <?= json_encode(JSTKO_BASE) ?>;

/* ---- item tooltip: hover (or tap) any element carrying data-item="<item number>" ---- */
(function () {
  var KIND = {11:'Dagger',21:'Sword',22:'Two-handed Sword',31:'Axe',32:'Two-handed Axe',41:'Mace',42:'Two-handed Mace',51:'Spear',52:'Long Spear',60:'Shield',70:'Bow',71:'Crossbow',91:'Earring',92:'Necklace',93:'Ring',94:'Belt',95:'Item',97:'Consumable',110:'Staff',120:'Arm band',130:'Armor',140:'Pet',151:'Pet',252:'Cosplay',255:'Unique'};
  var STAT = {str_b:['Strength Bonus','#ff8a70'],sta_b:['Stamina Bonus','#ffd166'],dex_b:['Dexterity Bonus','#7ee081'],intel_b:['Intelligence Bonus','#6cb6ff'],cha_b:['Charisma Bonus','#d3a0ff'],max_hp_b:['Max HP Bonus','#ff7a9a'],max_mp_b:['Max MP Bonus','#4fd1f5']};
  var ELEM = {fire_damage:['Fire Damage','#ff9442'],ice_damage:['Ice Damage','#8fdcff'],lightning_damage:['Lightning Damage','#ffe45c'],poison_damage:['Poison Damage','#6fd36f'],hp_drain:['HP Drain','#ff7a9a'],mp_damage:['MP Damage','#4fd1f5'],mp_drain:['MP Drain','#4fd1f5'],mirror_damage:['Damage Reflect','#c4b5fd']};
  var RESIST = {fire_r:['Fire Resistance','#ff9442'],cold_r:['Ice Resistance','#8fdcff'],lightning_r:['Lightning Resistance','#ffe45c'],magic_r:['Magic Resistance','#c4b5fd'],poison_r:['Poison Resistance','#6fd36f'],curse_r:['Curse Resistance','#d3a0ff']};
  var ACS = {dagger_ac:'Dagger',sword_ac:'Sword',club_ac:'Mace',mace_ac:'Mace',axe_ac:'Axe',spear_ac:'Spear',bow_ac:'Bow',jamadar_ac:'Jamadar'};
  var tip = document.createElement('div'); tip.id = 'itemTip'; document.body.appendChild(tip);
  var cache = {}, token = 0, base = (window.JSTKO_BASE || '');

  function esc(s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]; }); }
  function speed(d) { return d <= 0 ? null : d <= 60 ? 'Very Fast' : d <= 90 ? 'Fast' : d <= 130 ? 'Normal' : d <= 170 ? 'Slow' : 'Very Slow'; }
  function render(it) {
    var n = function (k) { return Number(it[k] || 0); }, body = '';
    var row = function (l, v, c) { body += '<div class="ln"><span' + (c ? ' style="color:' + c + '"' : '') + '>' + esc(l) + ' : ' + esc(v) + '</span></div>'; };
    var bonus = Object.keys(STAT).some(function (k) { return n(k); }) || Object.keys(ELEM).some(function (k) { return n(k); });
    var unique = Number(it.kind) === 255 || n('item_class') >= 4;
    var dur = n('duration');
    var head = '<div class="th"><img src="' + esc(it.icon) + '" alt=""><div class="tn"><div class="nm' + (unique ? ' unique' : bonus ? ' magic' : '') + '">' + esc(it.str_name || ('#' + it.num)) + '</div><div class="ty">' + (unique ? 'Unique item. ' : '') + esc(KIND[it.kind] || ('Type ' + it.kind)) + '</div></div>' + (dur ? '<div class="dbar"><i style="width:100%"></i></div>' : '') + '</div>';
    if (dur) row('Durability', dur);
    body += '<hr>';
    var lo = n('min_damage') || n('damage'), hi = n('max_damage');
    if (lo || hi) row('Attack Power', lo + (hi && hi !== lo ? ' - ' + hi : ''));
    if (n('delay')) row('Attack Speed', speed(n('delay')));
    if (n('range')) row('Effective Range', (n('range') / 10).toFixed(2));
    if (n('ac')) row('Defense', n('ac'));
    if (n('hitrate')) row('Hit Rate', n('hitrate'));
    if (n('evasionrate')) row('Evasion', n('evasionrate'));
    Object.keys(ELEM).forEach(function (k) { if (n(k)) row(ELEM[k][0], n(k), ELEM[k][1]); });
    Object.keys(ACS).forEach(function (k) { if (n(k)) row(ACS[k] + ' Defense', '+' + n(k), '#9fc4ff'); });
    Object.keys(STAT).forEach(function (k) { if (n(k)) row(STAT[k][0], (n(k) > 0 ? '+' : '') + n(k), STAT[k][1]); });
    Object.keys(RESIST).forEach(function (k) { if (n(k)) row(RESIST[k][0], '+' + n(k), RESIST[k][1]); });
    row('Weight', (n('weight') / 10).toFixed(2));
    if (n('req_level')) row('Required Level', n('req_level'));
    [['req_str','Strength'],['req_sta','Stamina'],['req_dex','Dexterity'],['req_intel','Intelligence'],['req_cha','Charisma']].forEach(function (p) { if (n(p[0])) row('Required ' + p[1], n(p[0])); });
    if (n('req_rank')) row('Required Rank', n('req_rank'));
    var foot = '';
    if (n('by_grade')) foot += '<div class="gd">Item Grade : Grade ' + n('by_grade') + '</div>';
    if (n('bound')) foot += '<div class="gd">Sealed item</div>';
    if (it.description) { var d = String(it.description).replace(/\|/g, '\n').replace(/<[^>]+>/g, '').trim(); if (d) foot += '<div class="ds">*' + esc(d) + '*</div>'; }
    return head + body + (foot ? '<hr class="gold">' + foot : '') + '<div class="ft">#' + esc(it.num) + (n('buy_price') ? ' · Price ' + n('buy_price').toLocaleString('tr-TR') : '') + '</div>';
  }
  function load(id) {
    if (!cache[id]) cache[id] = fetch(base + '/tip.php?id=' + id).then(function (r) { return r.json(); }).catch(function () { return null; });
    return cache[id];
  }
  function place(x, y) {
    var w = tip.offsetWidth, h = tip.offsetHeight, pad = 16, px = x + pad, py = y + pad;
    if (px + w > innerWidth - 8) px = x - w - pad;
    if (py + h > innerHeight - 8) py = Math.max(8, innerHeight - h - 8);
    if (px < 8) px = 8;
    tip.style.left = px + 'px'; tip.style.top = py + 'px';
  }
  function hide() { token++; tip.style.display = 'none'; }
  function show(el, x, y) {
    var id = parseInt(el.getAttribute('data-item'), 10); if (!id) return;
    var my = ++token;
    tip.innerHTML = '<div class="ln" style="color:#8b96ad">Yükleniyor…</div>'; tip.style.display = 'block'; place(x, y);
    load(id).then(function (it) {
      if (my !== token) return;
      tip.innerHTML = it ? render(it) : '<div class="ln" style="color:#8b96ad">Eşya #' + id + ' bulunamadı</div>';
      place(x, y);
    });
  }
  document.addEventListener('mouseover', function (e) {
    var el = e.target.closest && e.target.closest('[data-item]');
    if (el) show(el, e.clientX, e.clientY);
  });
  document.addEventListener('mousemove', function (e) { if (tip.style.display === 'block') place(e.clientX, e.clientY); });
  document.addEventListener('mouseout', function (e) {
    var el = e.target.closest && e.target.closest('[data-item]');
    if (el && !(e.relatedTarget && el.contains(e.relatedTarget))) hide();
  });
  // touch: tap shows, tap elsewhere hides
  document.addEventListener('click', function (e) {
    if (!('ontouchstart' in window)) return;
    var el = e.target.closest && e.target.closest('[data-item]');
    if (el) { var r = el.getBoundingClientRect(); show(el, r.left, r.bottom); } else hide();
  });
})();
</script>
</body>
</html>
