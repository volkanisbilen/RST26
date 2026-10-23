<?php
/** @var string|null $pageTitle set by the including page before require'ing this partial */
require_once __DIR__ . '/../includes/bootstrap.php';
$user = jstko_current_user();
$siteName = jstko_c('site_name', 'JstKO');
$logo = jstko_c_image('logo_image');
$bgImage = jstko_c_image('site_bg_image');
$cmsMenu = jstko_cms_menu();
$B = JSTKO_BASE;
$cur = basename((string)($_SERVER['SCRIPT_NAME'] ?? ''));

// default menu (used until a menu is built in the CMS); [label, file, children?]
$defaultMenu = [
    ['Anasayfa', 'index.php', null],
    ['Sıralamalar', 'rankings.php', null],
    ['Droplar', 'items.php', null],
    ['Haberler', null, [['Haberler & Duyurular', 'news.php?kind=news'], ['Güncelleme Notları', 'news.php?kind=update']]],
    ['Rehber', null, [['Görevler', 'quests.php'], ['Craft', 'craft.php'], ['Upgrade Oranları', 'upgrade_rates.php']]],
    ['Etkinlikler', null, [
        ['Etkinlik Takvimi', 'events.php'], ['Günlük Görevler', 'daily_quests.php'], ['Collection Race', 'collection_race.php'],
        ['Coin Etkinliği', 'event_coin.php'], ['Puzzle Etkinliği', 'event_puzzle.php'], ['Çark Ödülleri', 'event_roulette.php'], ['Lottery', 'event_lottery.php'],
    ]],
    ['Online Oyuncular', 'online.php', null],
    ["Online GM'ler", 'gms.php', null],
    ['Karakter Kurtar', 'character_rescue.php', null],
    ['Destek', 'support.php', null],
];
$menu = [];
if ($cmsMenu) {
    foreach ($cmsMenu as $mi) {
        $kids = [];
        foreach ($mi['children'] as $ch) $kids[] = ['label' => $ch['label'], 'url' => jstko_url($ch['url'])];
        $menu[] = ['label' => $mi['label'], 'url' => jstko_url($mi['url']), 'kids' => $kids, 'active' => false];
    }
} else {
    foreach ($defaultMenu as [$label, $file, $kids]) {
        $k = [];
        $active = $file === $cur;
        foreach ($kids ?? [] as [$kl, $kf]) { $k[] = ['label' => $kl, 'url' => $B . '/' . $kf]; if (basename(explode('?', $kf)[0]) === $cur) $active = true; }
        $menu[] = ['label' => $label, 'url' => $file === 'index.php' ? $B . '/' : ($file ? $B . '/' . $file : '#'), 'kids' => $k, 'active' => $active];
    }
}
?>
<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title><?= jstko_e(($pageTitle ?? '') !== '' ? $pageTitle . ' · ' . $siteName : $siteName) ?></title>
<script src="https://cdn.tailwindcss.com/3.4.14"></script>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&family=Inter:wght@400;500;600;700;800&display=swap" rel="stylesheet">
<?php require __DIR__ . '/theme.php'; ?>
<script>
  tailwind.config = {
    theme: {
      extend: {
        fontFamily: { sans: ['Inter', 'sans-serif'], display: ['Cinzel', 'serif'] },
        colors: {
          ink: { 900: '#050912', 800: '#080e1a', 700: '#0d1525', 600: '#131d31', 500: '#222d44' },
          gold: { 300: '#f8dc8a', 400: '#f5c451', 500: '#e0a52a', 600: '#b97d12' },
        },
      },
    },
  };
</script>
</head>
<body class="bg-ink-900 text-slate-200 font-sans min-h-screen flex flex-col"<?php if ($bgImage): ?> style="background-image:linear-gradient(rgba(5,9,18,.80),rgba(5,9,18,.92)),url('<?= jstko_e($bgImage) ?>');background-size:cover;background-position:center;background-attachment:fixed"<?php endif; ?>>

<header class="site-header sticky top-0 z-40">
  <div class="mx-auto max-w-[1400px] px-4 lg:px-6">
    <div class="flex items-center justify-between h-[60px] gap-4">
      <a href="<?= $B ?>/" class="flex items-center gap-2 shrink-0" aria-label="<?= jstko_e($siteName) ?>">
        <?php if ($logo): ?>
          <span class="logo-wrap"><img src="<?= jstko_e($logo) ?>" alt="<?= jstko_e($siteName) ?>" class="h-11 w-auto"></span>
        <?php else: ?>
          <span class="logo-wrap"><span class="font-display font-extrabold text-2xl text-gold-400 tracking-wide"><?= jstko_e($siteName) ?></span></span>
        <?php endif; ?>
      </a>

      <nav class="hidden min-[1500px]:flex items-center gap-1.5 text-[13px]" aria-label="Ana menü">
        <?php foreach ($menu as $ni => $mi): ?>
          <?php if ($mi['kids']): ?>
            <div class="relative group">
              <button type="button" style="--i:<?= $ni ?>" class="nav-link flex items-center gap-1 <?= $mi['active'] ? 'is-active' : '' ?>">
                <?= jstko_e($mi['label']) ?>
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M6 9l6 6 6-6"/></svg>
              </button>
              <div class="hidden group-hover:block group-focus-within:block absolute left-0 top-full pt-2 w-56 z-50">
                <div class="drop-panel py-1.5">
                  <?php foreach ($mi['kids'] as $ch): ?>
                    <a href="<?= jstko_e($ch['url']) ?>" class="block px-4 py-2 text-[13px] hover:bg-ink-600 hover:text-gold-400"><?= jstko_e($ch['label']) ?></a>
                  <?php endforeach; ?>
                </div>
              </div>
            </div>
          <?php else: ?>
            <a href="<?= jstko_e($mi['url']) ?>" style="--i:<?= $ni ?>" class="nav-link <?= $mi['active'] ? 'is-active' : '' ?>"><?= jstko_e($mi['label']) ?></a>
          <?php endif; ?>
        <?php endforeach; ?>
      </nav>

      <div class="hidden min-[1500px]:flex items-center gap-2 shrink-0">
        <?php if ($user): ?>
          <a href="<?= $B ?>/account.php" class="flex items-center gap-2 text-[13px] font-semibold text-white hover:text-gold-400">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="text-gold-400"><circle cx="12" cy="8" r="4"/><path d="M4 21c0-4.4 3.6-7 8-7s8 2.6 8 7z"/></svg>
            <?= jstko_e($user['str_account_id']) ?>
          </a>
          <?php if (jstko_is_gm((int)$user['str_authority'])): ?>
            <a href="/admin/" class="btn-ghost text-xs">Admin</a>
          <?php endif; ?>
          <a href="<?= $B ?>/logout.php" class="btn-ghost">Çıkış</a>
        <?php else: ?>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="text-gold-400 mr-1"><circle cx="12" cy="8" r="4"/><path d="M4 21c0-4.4 3.6-7 8-7s8 2.6 8 7z"/></svg>
          <a href="<?= $B ?>/login.php" class="btn-ghost">Giriş Yap</a>
          <a href="<?= $B ?>/register.php" class="btn-gold">Kayıt Ol</a>
        <?php endif; ?>
      </div>

      <button id="menuBtn" type="button" class="min-[1500px]:hidden nav-link !p-2" aria-label="Menü" aria-controls="mobileMenu">
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6h16M4 12h16M4 18h16"/></svg>
      </button>
    </div>
  </div>

  <div id="mobileMenu" class="hidden min-[1500px]:hidden border-t border-gold-600/30 bg-ink-800/95 backdrop-blur max-h-[80vh] overflow-y-auto">
    <nav class="px-4 py-4 text-sm" aria-label="Mobil menü">
      <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
      <?php foreach ($menu as $mi): if ($mi['kids']) continue; ?>
        <a href="<?= jstko_e($mi['url']) ?>" class="nav-link justify-center text-center <?= $mi['active'] ? 'is-active' : '' ?>"><?= jstko_e($mi['label']) ?></a>
      <?php endforeach; ?>
      </div>
      <?php foreach ($menu as $mi): if (!$mi['kids']) continue; ?>
        <div class="mt-4 mb-2 text-[11px] uppercase tracking-widest text-gold-500 font-bold"><?= jstko_e($mi['label']) ?></div>
        <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
          <?php foreach ($mi['kids'] as $ch): ?>
            <a href="<?= jstko_e($ch['url']) ?>" class="nav-link justify-center text-center"><?= jstko_e($ch['label']) ?></a>
          <?php endforeach; ?>
        </div>
      <?php endforeach; ?>
      <div class="h-px bg-gold-600/30 my-4"></div>
      <?php if ($user): ?>
        <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
          <a href="<?= $B ?>/account.php" class="btn-gold col-span-2 sm:col-span-1">Hesabım (<?= jstko_e($user['str_account_id']) ?>)</a>
          <?php if (jstko_is_gm((int)$user['str_authority'])): ?><a href="/admin/" class="btn-ghost">Admin Paneli</a><?php endif; ?>
          <a href="<?= $B ?>/logout.php" class="btn-ghost">Çıkış Yap</a>
        </div>
      <?php else: ?>
        <div class="grid grid-cols-2 gap-2">
          <a href="<?= $B ?>/login.php" class="btn-ghost">Giriş Yap</a>
          <a href="<?= $B ?>/register.php" class="btn-gold">Kayıt Ol</a>
        </div>
      <?php endif; ?>
    </nav>
  </div>
</header>

<?php
$flash = $_SESSION['flash'] ?? null;
unset($_SESSION['flash']);
if ($flash):
    $tone = $flash['type'] === 'error' ? 'bg-red-900/40 border-red-700 text-red-200' : 'bg-emerald-900/40 border-emerald-700 text-emerald-200';
?>
<div class="max-w-6xl mx-auto px-4 pt-4 w-full">
  <div class="<?= $tone ?> border rounded-lg px-4 py-3 text-sm"><?= jstko_e($flash['message']) ?></div>
</div>
<?php endif; ?>

<main class="flex-1 w-full">
