<?php
require_once __DIR__ . '/../includes/bootstrap.php';

$token = (string)($_GET['token'] ?? '');
$result = $token !== '' ? jstko_activate_account($token) : ['ok' => false, 'error' => 'Geçersiz bağlantı.'];

$pageTitle = 'Hesap Aktivasyonu';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-md mx-auto px-4 py-16">
  <div class="card p-8 text-center">
    <?php if ($result['ok']): ?>
      <div class="text-5xl mb-4">✅</div>
      <h1 class="text-xl font-bold text-white mb-2">Hesabın Aktifleştirildi!</h1>
      <p class="text-sm text-slate-400 mb-6">Artık giriş yapabilir ve oyuna başlayabilirsin.</p>
      <a href="/login.php" class="btn-primary inline-block">Giriş Yap</a>
    <?php else: ?>
      <div class="text-5xl mb-4">⚠️</div>
      <h1 class="text-xl font-bold text-white mb-2">Aktivasyon Başarısız</h1>
      <p class="text-sm text-slate-400 mb-6"><?= jstko_e($result['error']) ?></p>
      <a href="/register.php" class="text-gold-400 hover:underline text-sm">Yeniden kayıt ol</a>
    <?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
