<?php
/** @var string|null $pageTitle set by the including page before require'ing this partial */
require_once __DIR__ . '/../../includes/bootstrap.php';
$user = jstko_current_user();
$siteName = jstko_c('site_name', 'JstKO');
$logo = jstko_c_image('logo_image');
?>
<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title><?= jstko_e(($pageTitle ?? '') !== '' ? $pageTitle . ' · ' . $siteName : $siteName) ?></title>
<script src="https://cdn.tailwindcss.com/3.4.14"></script>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap" rel="stylesheet">
<link rel="stylesheet" href="/assets/css/app.css">
<script>
  tailwind.config = {
    theme: {
      extend: {
        fontFamily: { sans: ['Inter', 'sans-serif'] },
        colors: {
          ink: { 900: '#0b0e14', 800: '#111621', 700: '#141925', 600: '#1b2230', 500: '#232a3b' },
          gold: { 400: '#facc15', 500: '#eab308' },
        },
      },
    },
  };
</script>
</head>
<body class="bg-ink-900 text-slate-200 font-sans min-h-screen flex flex-col">

<header class="bg-ink-800 border-b border-ink-500 sticky top-0 z-40">
  <div class="max-w-6xl mx-auto px-4">
    <div class="flex items-center justify-between h-16">
      <a href="/" class="flex items-center gap-2 font-extrabold text-lg text-white shrink-0">
        <?php if ($logo): ?>
          <img src="<?= jstko_e($logo) ?>" alt="<?= jstko_e($siteName) ?>" class="h-9 w-auto">
        <?php else: ?>
          <span class="h-8 w-8 rounded bg-gold-500 text-ink-900 grid place-items-center">K</span>
        <?php endif; ?>
        <span><?= jstko_e($siteName) ?></span>
      </a>

      <nav class="hidden md:flex items-center gap-6 text-sm font-medium">
        <a href="/" class="hover:text-gold-400 transition">Anasayfa</a>
        <a href="/rankings.php" class="hover:text-gold-400 transition">Sıralamalar</a>
        <a href="/online.php" class="hover:text-gold-400 transition">Online Oyuncular</a>
        <a href="/gms.php" class="hover:text-gold-400 transition">Online GM'ler</a>
        <a href="/character_rescue.php" class="hover:text-gold-400 transition">Karakter Kurtar</a>
        <a href="/support.php" class="hover:text-gold-400 transition">Destek</a>
      </nav>

      <div class="hidden md:flex items-center gap-3">
        <?php if ($user): ?>
          <a href="/account.php" class="text-sm font-semibold text-white hover:text-gold-400"><?= jstko_e($user['str_account_id']) ?></a>
          <?php if (jstko_is_gm((int)$user['str_authority'])): ?>
            <a href="/admin/" class="text-xs px-2 py-1 rounded bg-ink-600 text-gold-400 border border-ink-500">Admin</a>
          <?php endif; ?>
          <a href="/logout.php" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500 transition">Çıkış</a>
        <?php else: ?>
          <a href="/login.php" class="text-sm px-3 py-1.5 rounded bg-ink-600 hover:bg-ink-500 transition">Giriş Yap</a>
          <a href="/register.php" class="text-sm px-3 py-1.5 rounded bg-gold-500 text-ink-900 font-semibold hover:bg-gold-400 transition">Kayıt Ol</a>
        <?php endif; ?>
      </div>

      <button id="menuBtn" type="button" class="md:hidden p-2 rounded hover:bg-ink-600" aria-label="Menü">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6h16M4 12h16M4 18h16"/></svg>
      </button>
    </div>
  </div>

  <div id="mobileMenu" class="hidden md:hidden border-t border-ink-500 bg-ink-800">
    <nav class="flex flex-col px-4 py-3 gap-1 text-sm font-medium">
      <a href="/" class="py-2 hover:text-gold-400">Anasayfa</a>
      <a href="/rankings.php" class="py-2 hover:text-gold-400">Sıralamalar</a>
      <a href="/online.php" class="py-2 hover:text-gold-400">Online Oyuncular</a>
      <a href="/gms.php" class="py-2 hover:text-gold-400">Online GM'ler</a>
      <a href="/character_rescue.php" class="py-2 hover:text-gold-400">Karakter Kurtar</a>
      <a href="/support.php" class="py-2 hover:text-gold-400">Destek</a>
      <div class="h-px bg-ink-500 my-2"></div>
      <?php if ($user): ?>
        <a href="/account.php" class="py-2 font-semibold text-white">Hesabım (<?= jstko_e($user['str_account_id']) ?>)</a>
        <?php if (jstko_is_gm((int)$user['str_authority'])): ?><a href="/admin/" class="py-2 text-gold-400">Admin Paneli</a><?php endif; ?>
        <a href="/logout.php" class="py-2">Çıkış Yap</a>
      <?php else: ?>
        <a href="/login.php" class="py-2">Giriş Yap</a>
        <a href="/register.php" class="py-2 text-gold-400 font-semibold">Kayıt Ol</a>
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
<div class="max-w-6xl mx-auto px-4 pt-4">
  <div class="<?= $tone ?> border rounded-lg px-4 py-3 text-sm"><?= jstko_e($flash['message']) ?></div>
</div>
<?php endif; ?>

<main class="flex-1 w-full">
