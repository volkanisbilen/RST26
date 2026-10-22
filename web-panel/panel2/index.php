<?php
require_once __DIR__ . '/includes/bootstrap.php';
$pageTitle = 'Anasayfa';
require __DIR__ . '/partials/header.php';

$pdo = jstko_db();
$online = (int)$pdo->query('SELECT count(*) FROM currentuser')->fetchColumn();

$top = $pdo->query(
    "SELECT u.str_user_id name, k.id_name clan, u.level lvl, u.loyalty np, u.nation,
            EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) online
     FROM userdata u LEFT JOIN knights k ON k.id_num = u.knights
     ORDER BY u.loyalty DESC LIMIT 10"
)->fetchAll();

$news = $pdo->query(
    "SELECT id, title, body, image, created_at FROM web_news WHERE published ORDER BY created_at DESC LIMIT 8"
)->fetchAll();
$notes = array_slice($news, 0, 4);
$cards = array_slice($news, 4, 4) ?: array_slice($news, 0, 4);

$heroImage = jstko_c_image('hero_image');
$slides = jstko_cms_slides();
$heroBg = [];
foreach ($slides as $sl) $heroBg[] = '/uploads/' . $sl['image'];
if (!$heroBg && $heroImage) $heroBg[] = $heroImage;

$title = jstko_c('hero_title', 'JstKO PvP Dünyasına Hoş Geldin');
// "A B C" -> first words white, last two words gold (matches the artwork); a "|" in the CMS title picks the break
if (strpos($title, '|') !== false) {
    [$t1, $t2] = array_map('trim', explode('|', $title, 2));
} else {
    $w = preg_split('/\s+/', trim($title));
    $cut = max(1, count($w) - 2);
    $t1 = implode(' ', array_slice($w, 0, $cut));
    $t2 = implode(' ', array_slice($w, $cut));
}
$sub = jstko_c('hero_subtitle', 'Ücretsiz oyna, hemen rekabete katıl.');
$dl = jstko_c('download_url');
$video = jstko_c('video_url');
$poster = jstko_c_image('video_poster') ?: ($heroBg[0] ?? '');
$ytId = '';
if ($video !== '' && preg_match('~(?:youtu\.be/|v=|embed/)([A-Za-z0-9_-]{11})~', $video, $m)) $ytId = $m[1];
$B = JSTKO_BASE;

function jstko_news_tag(string $title): string
{
    $t = mb_strtolower($title);
    if (preg_match('/etkinlik|event|turnuva/u', $t)) return 'ETKİNLİK';
    if (preg_match('/güncelle|yama|patch|sistem|iyileştir/u', $t)) return 'GÜNCELLEME';
    return 'DUYURU';
}
function jstko_news_img(array $n): string
{
    return $n['image'] !== '' ? '/uploads/' . ltrim($n['image'], '/') : '';
}
function jstko_date_tr(string $ts): string
{
    $t = strtotime($ts);
    return $t ? date('d.m.Y', $t) : '';
}
?>

<!-- ============ HERO ============ -->
<section class="relative overflow-hidden border-b border-gold-600/30" id="hero">
  <div class="absolute inset-0 bg-ink-800">
    <?php foreach ($heroBg as $i => $src): ?>
      <img src="<?= jstko_e($src) ?>" alt="" class="hero-slide absolute inset-0 w-full h-full object-cover object-[65%_center] transition-opacity duration-700 <?= $i === 0 ? 'opacity-100' : 'opacity-0' ?>">
    <?php endforeach; ?>
    <div class="absolute inset-0 bg-gradient-to-r from-ink-900 via-ink-900/70 to-transparent sm:via-ink-900/45"></div>
    <div class="absolute inset-0 bg-gradient-to-t from-ink-900/90 via-transparent to-ink-900/30"></div>
  </div>

  <div class="relative mx-auto max-w-[1400px] px-5 lg:px-8 min-h-[480px] sm:min-h-[560px] lg:min-h-[640px] flex items-center py-14">
    <div class="max-w-xl">
      <span class="hero-pill">MMORPG • Ücretsiz Oyna</span>
      <h1 class="hero-title text-4xl sm:text-5xl lg:text-6xl mt-5 mb-4">
        <?= jstko_e($t1) ?>
        <?php if ($t2 !== ''): ?><span class="gold"><?= jstko_e($t2) ?></span><?php endif; ?>
      </h1>
      <p class="text-slate-200 text-base sm:text-lg mb-7 drop-shadow"><?= jstko_e($sub) ?></p>
      <div class="flex flex-wrap gap-3">
        <?php if ($dl): ?>
          <a href="<?= jstko_e($dl) ?>" class="btn-gold !px-6 !py-3 !text-sm">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><path d="M12 3v12m0 0l-5-5m5 5l5-5M4 20h16"/></svg>
            OYUNU İNDİR
          </a>
        <?php endif; ?>
        <a href="<?= $B ?>/register.php" class="btn-ghost !px-6 !py-3 !text-sm !bg-ink-900/60">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="text-gold-400"><circle cx="12" cy="8" r="4"/><path d="M4 21c0-4.400 3.600-7 8-7s8 2.600 8 7z"/></svg>
          HESAP OLUŞTUR
        </a>
      </div>
    </div>

    <div class="online-card hidden md:flex absolute right-6 lg:right-10 bottom-8 items-center gap-4 px-5 py-4">
      <svg width="42" height="42" viewBox="0 0 24 24" fill="#f5c451"><circle cx="8" cy="8" r="3"/><circle cx="16.500" cy="8.500" r="2.500"/><circle cx="12" cy="9" r="3.500"/><path d="M2 20c0-3.300 2.700-5.500 6-5.500 1 0 1.800.2 2.600.6A7 7 0 0 0 9 20zM22 20c0-2.600-2-4.500-4.700-4.700-.7.100-1.300.3-1.900.6.900.9 1.400 2.200 1.400 4.100zM12 14.500c3.300 0 6 2 6 5.500H6c0-3.500 2.700-5.500 6-5.500z"/></svg>
      <div>
        <div class="text-[10px] font-semibold tracking-widest text-slate-400">AKTİF OYUNCU</div>
        <div class="text-3xl font-extrabold text-gold-400 leading-none my-1"><?= jstko_number($online) ?></div>
        <div class="flex items-center gap-1.5 text-[11px] text-slate-300"><span class="w-2 h-2 rounded-full bg-emerald-400 shadow-[0_0_8px_#34d399]"></span>Sunucu Online</div>
      </div>
    </div>

    <?php if (count($heroBg) > 1): ?>
      <button type="button" class="hero-arrow left-3 lg:left-5" id="heroPrev" aria-label="Önceki"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M15 5l-7 7 7 7"/></svg></button>
      <button type="button" class="hero-arrow right-3 lg:right-5" id="heroNext" aria-label="Sonraki"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M9 5l7 7-7 7"/></svg></button>
      <div class="absolute bottom-4 left-0 right-0 flex justify-center gap-2" id="heroDots">
        <?php foreach ($heroBg as $i => $_): ?><button type="button" data-i="<?= $i ?>" aria-label="Slayt <?= $i + 1 ?>" class="w-2 h-2 rounded-full <?= $i === 0 ? 'bg-gold-400' : 'bg-white/40' ?>"></button><?php endforeach; ?>
      </div>
    <?php endif; ?>
  </div>
</section>

<!-- ============ FEATURE STRIP ============ -->
<section class="feature-strip" aria-label="Öne çıkanlar">
  <div class="mx-auto max-w-[1400px] px-5 lg:px-8 py-4 grid grid-cols-2 lg:grid-cols-4 gap-x-6 gap-y-4">
    <?php
    $features = [
        ['Aktif Etkinlikler', 'Her gün yeni etkinlikler ve ödüller', '<path d="M12 2l2.9 6.3 6.9.8-5.1 4.7 1.4 6.8L12 17.2l-6.1 3.4 1.4-6.8L2.2 9.1l6.9-.8z"/>'],
        ['Adil Rekabet', 'Güvenli ve dengeli oyun deneyimi', '<path d="M12 2l8 3v6c0 5-3.400 9.300-8 11-4.600-1.700-8-6-8-11V5zm0 4.500L8 8v3.500c0 2.800 1.600 5.300 4 6.500 2.400-1.200 4-3.700 4-6.500V8z"/>'],
        ['Güçlü Topluluk', 'Binlerce oyuncu, tek aile', '<circle cx="12" cy="7.500" r="3"/><circle cx="5.500" cy="9" r="2.200"/><circle cx="18.500" cy="9" r="2.200"/><path d="M6.500 20c0-3.500 2.500-5.500 5.500-5.500s5.500 2 5.500 5.500zM1 18c0-2.600 1.800-4 4.200-4 .6 0 1.200.1 1.700.3A7 7 0 0 0 5.500 18zM23 18c0-2.600-1.800-4-4.200-4-.6 0-1.200.1-1.700.3.600 1 .900 2.200.9 3.700z"/>'],
        ['Klasik Knight Online', 'Efsanevi atmosfer, nostaljik his', '<path d="M3 8l4.500 4L12 5l4.500 7L21 8l-2 11H5zm2.300 13h13.400v2H5.300z"/>'],
    ];
    foreach ($features as [$ft, $fd, $icon]): ?>
      <div class="flex items-center gap-3 lg:justify-center">
        <span class="feature-icon"><svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor"><?= $icon ?></svg></span>
        <div class="min-w-0">
          <div class="text-sm font-semibold text-gold-400"><?= $ft ?></div>
          <div class="text-[11px] text-slate-400 leading-snug"><?= $fd ?></div>
        </div>
      </div>
    <?php endforeach; ?>
  </div>
</section>

<?php if ($b = jstko_banner_html('home_top')): ?><div class="mx-auto max-w-[1280px] px-4 pt-6 space-y-3"><?= $b ?></div><?php endif; ?>

<div class="mx-auto max-w-[1280px] px-4 lg:px-6 py-8 space-y-6">

  <!-- video + update notes -->
  <div class="grid lg:grid-cols-[1.45fr_1fr] gap-6">
    <div class="panel overflow-hidden relative min-h-[260px] sm:min-h-[340px] lg:min-h-[0] aspect-video lg:aspect-auto" id="videoBox">
      <?php if ($poster): ?><img src="<?= jstko_e($poster) ?>" alt="" class="absolute inset-0 w-full h-full object-cover"><?php endif; ?>
      <div class="absolute inset-0 bg-gradient-to-t from-ink-900/95 via-ink-900/20 to-ink-900/10"></div>
      <?php if ($ytId !== ''): ?>
        <button type="button" id="videoPlay" data-yt="<?= jstko_e($ytId) ?>" class="absolute inset-0 grid place-items-center group" aria-label="Videoyu oynat">
          <span class="w-20 h-20 rounded-full grid place-items-center bg-ink-900/70 border-2 border-gold-400 shadow-[0_0_30px_rgba(245,196,81,.5)] group-hover:scale-105 transition">
            <svg width="34" height="34" viewBox="0 0 24 24" fill="#f5c451"><path d="M8 5v14l11-7z"/></svg>
          </span>
        </button>
      <?php elseif ($video !== ''): ?>
        <a href="<?= jstko_e($video) ?>" target="_blank" rel="noopener" class="absolute inset-0 grid place-items-center group" aria-label="Videoyu aç">
          <span class="w-20 h-20 rounded-full grid place-items-center bg-ink-900/70 border-2 border-gold-400 shadow-[0_0_30px_rgba(245,196,81,.5)] group-hover:scale-105 transition"><svg width="34" height="34" viewBox="0 0 24 24" fill="#f5c451"><path d="M8 5v14l11-7z"/></svg></span>
        </a>
      <?php endif; ?>
      <div class="absolute left-5 bottom-5 right-5 pointer-events-none">
        <span class="tag !bg-ink-900/80 !text-gold-400 border border-gold-600/50 !text-[11px]"><?= jstko_e(jstko_c('site_name', 'JstKO')) ?></span>
        <div class="font-display text-xl sm:text-2xl font-bold text-white mt-2"><?= jstko_e(jstko_c('video_title', jstko_c('site_name', 'JstKO') . ' Tanıtım Videosu')) ?></div>
        <div class="text-xs sm:text-sm text-slate-300 mt-1"><?= jstko_e(jstko_c('video_caption', 'Efsane geri döndü! ' . jstko_c('site_name', 'JstKO') . ' dünyasını keşfet.')) ?></div>
      </div>
    </div>

    <div class="panel">
      <div class="panel-head">
        <h2><span class="head-icon"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M6 2h9l5 5v15H6zm8 1.500V8h4.500zM8.500 12h8v1.500h-8zm0 3.500h8V17h-8z"/></svg></span>Güncelleme Notları</h2>
        <a href="<?= $B ?>/news.php" class="more">Tümünü Gör <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 12h14m-6-6l6 6-6 6"/></svg></a>
      </div>
      <div class="p-3 space-y-2">
        <?php foreach ($notes as $n): $img = jstko_news_img($n); ?>
          <a href="<?= $B ?>/news.php?id=<?= (int)$n['id'] ?>" class="flex items-center gap-3 p-2 rounded-lg hover:bg-ink-600/60 transition group">
            <span class="w-24 h-14 sm:w-28 sm:h-16 rounded-md overflow-hidden border border-gold-600/30 bg-ink-700 flex-none">
              <?php if ($img): ?><img src="<?= jstko_e($img) ?>" alt="" loading="lazy" class="w-full h-full object-cover"><?php endif; ?>
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-sm font-semibold text-white group-hover:text-gold-400 transition line-clamp-2"><?= jstko_e($n['title']) ?></span>
              <span class="block text-[11px] text-slate-500 mt-1"><?= jstko_date_tr($n['created_at']) ?></span>
            </span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="text-slate-500 flex-none"><path d="M9 5l7 7-7 7"/></svg>
          </a>
        <?php endforeach; ?>
        <?php if (!$notes): ?><p class="text-sm text-slate-500 p-3">Henüz haber yok.</p><?php endif; ?>
        <?php if ($lp = jstko_cms_latest_patch()): ?>
          <a href="<?= $B ?>/patches.php" class="block rounded-lg border border-gold-600/40 bg-gold-500/10 px-4 py-2.5 text-sm hover:bg-gold-500/20 transition">
            <span class="text-gold-400 font-semibold">⬇ Son Patch: <?= jstko_e($lp['version']) ?></span>
            <span class="block text-xs text-slate-400">Sorun yaşıyorsan indir</span>
          </a>
        <?php endif; ?>
      </div>
    </div>
  </div>

  <?php if ($b = jstko_banner_html('home_mid')): ?><div class="space-y-3"><?= $b ?></div><?php endif; ?>

  <!-- top 10 + announcements -->
  <div class="grid lg:grid-cols-[1fr_1.45fr] gap-6">
    <div class="panel">
      <div class="panel-head">
        <h2><span class="head-icon"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M7 3h10v2h3v3c0 2.400-1.700 4.300-4 4.800-.6 1.700-2 3-3.500 3.200V19h3v2H8.500v-2h3v-2c-1.500-.2-2.900-1.500-3.500-3.200C5.700 12.300 4 10.400 4 8V5h3zm0 4H6v1c0 1 .5 1.800 1.200 2.300A9 9 0 0 1 7 8zm10 0v1c0 1.300-.1 2-.2 2.300C17.500 9.800 18 9 18 8V7z"/></svg></span>En İyi 10 Oyuncu</h2>
        <a href="<?= $B ?>/rankings.php" class="more">Tümünü Gör <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 12h14m-6-6l6 6-6 6"/></svg></a>
      </div>
      <div class="overflow-x-auto px-2 pb-2">
        <table class="w-full text-[13px]">
          <thead class="text-left text-slate-500 text-[10px] uppercase tracking-wider">
            <tr><th class="py-2 px-2 w-8">#</th><th class="py-2 px-2">Oyuncu</th><th class="py-2 px-2 hidden sm:table-cell">Klan</th><th class="py-2 px-2 text-right">Level</th><th class="py-2 px-2 text-right">NP</th></tr>
          </thead>
          <tbody>
            <?php foreach ($top as $i => $r): ?>
            <tr class="table-row border-t border-gold-600/10 <?= $i === 0 ? 'bg-gold-500/10' : '' ?>">
              <td class="py-2 px-2 <?= $i < 3 ? 'text-gold-400 font-bold' : 'text-slate-500' ?>"><?= $i + 1 ?></td>
              <td class="py-2 px-2 font-semibold text-white">
                <span class="inline-flex items-center gap-2">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" class="<?= (int)$r['nation'] === 1 ? 'text-rose-400' : 'text-sky-400' ?>"><path d="M3 8l4.500 4L12 5l4.500 7L21 8l-2 11H5z"/></svg>
                  <?= jstko_e($r['name']) ?>
                  <?php if (jstko_pg_bool($r['online'])): ?><span class="w-1.5 h-1.5 rounded-full bg-emerald-400" title="Online"></span><?php endif; ?>
                </span>
              </td>
              <td class="py-2 px-2 text-slate-400 hidden sm:table-cell"><?= jstko_e($r['clan'] ?: 'Klan Yok') ?></td>
              <td class="py-2 px-2 text-right text-slate-300"><?= (int)$r['lvl'] ?></td>
              <td class="py-2 px-2 text-right text-gold-400 font-semibold"><?= jstko_number($r['np']) ?></td>
            </tr>
            <?php endforeach; ?>
            <?php if (!$top): ?><tr><td colspan="5" class="py-6 text-center text-slate-500">Henüz veri yok.</td></tr><?php endif; ?>
          </tbody>
        </table>
      </div>
    </div>

    <div class="panel">
      <div class="panel-head">
        <h2><span class="head-icon"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M3 10v4l3 .5L9 19h2.500l-1.800-4.300L19 18V6L9.700 9.500 6 10zM21 9v6a3 3 0 0 0 0-6z"/></svg></span>Haberler &amp; Duyurular</h2>
        <a href="<?= $B ?>/news.php" class="more">Tümünü Gör <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 12h14m-6-6l6 6-6 6"/></svg></a>
      </div>
      <div class="p-3 grid sm:grid-cols-2 gap-3">
        <?php foreach ($cards as $n): $img = jstko_news_img($n); ?>
          <a href="<?= $B ?>/news.php?id=<?= (int)$n['id'] ?>" class="news-card group block min-h-[130px] bg-ink-700">
            <?php if ($img): ?><img src="<?= jstko_e($img) ?>" alt="" loading="lazy" class="absolute inset-0 w-full h-full object-cover group-hover:scale-105 transition duration-500"><?php endif; ?>
            <span class="absolute inset-0 bg-gradient-to-t from-ink-900/95 via-ink-900/55 to-ink-900/10"></span>
            <span class="relative flex flex-col justify-end h-full min-h-[130px] p-3">
              <span><span class="tag"><?= jstko_news_tag($n['title']) ?></span></span>
              <span class="block text-sm font-semibold text-white mt-1.5 line-clamp-2"><?= jstko_e($n['title']) ?></span>
              <span class="block text-[10px] text-slate-400 mt-0.5"><?= jstko_date_tr($n['created_at']) ?></span>
            </span>
          </a>
        <?php endforeach; ?>
        <?php if (!$cards): ?><p class="text-sm text-slate-500 p-3 sm:col-span-2">Henüz duyuru yok.</p><?php endif; ?>
      </div>
      <?php if ($sb = jstko_banner_html('home_side')): ?><div class="p-3 pt-0 space-y-3"><?= $sb ?></div><?php endif; ?>
    </div>
  </div>
</div>

<script>
(function () {
  var hero = document.getElementById('hero');
  var imgs = hero.querySelectorAll('.hero-slide'), dots = hero.querySelectorAll('#heroDots button'), cur = 0, timer;
  function show(n) {
    if (imgs.length < 2) return;
    cur = (n + imgs.length) % imgs.length;
    imgs.forEach(function (el, i) { el.classList.toggle('opacity-100', i === cur); el.classList.toggle('opacity-0', i !== cur); });
    dots.forEach(function (d, i) { d.classList.toggle('bg-gold-400', i === cur); d.classList.toggle('bg-white/40', i !== cur); });
  }
  function start() { clearInterval(timer); if (imgs.length > 1) timer = setInterval(function () { show(cur + 1); }, 6000); }
  var p = document.getElementById('heroPrev'), nx = document.getElementById('heroNext');
  if (p) p.addEventListener('click', function () { show(cur - 1); start(); });
  if (nx) nx.addEventListener('click', function () { show(cur + 1); start(); });
  dots.forEach(function (d) { d.addEventListener('click', function () { show(+d.dataset.i); start(); }); });
  start();

  var play = document.getElementById('videoPlay');
  if (play) play.addEventListener('click', function () {
    var box = document.getElementById('videoBox');
    box.innerHTML = '<iframe class="absolute inset-0 w-full h-full" src="https://www.youtube.com/embed/' + play.dataset.yt + '?autoplay=1&rel=0" title="Tanıtım videosu" allow="autoplay; encrypted-media; picture-in-picture" allowfullscreen></iframe>';
  });
})();
</script>

<?php require __DIR__ . '/partials/footer.php'; ?>
