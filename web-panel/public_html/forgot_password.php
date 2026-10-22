<?php
require_once __DIR__ . '/includes/bootstrap.php';

$sent = false;
$error = null;

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } elseif (!jstko_rate_limit('forgot_password', jstko_client_ip(), 5, 600)) {
        $error = 'Çok fazla deneme yaptınız, lütfen daha sonra tekrar deneyin.';
    } else {
        jstko_request_password_reset((string)($_POST['account_id'] ?? ''), (string)($_POST['email'] ?? ''));
        $sent = true;
    }
}

$pageTitle = 'Şifremi Unuttum';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-md mx-auto px-4 py-16">
  <div class="card p-8">
    <h1 class="text-2xl font-bold text-white mb-1">Şifremi Unuttum</h1>
    <p class="text-sm text-slate-400 mb-6">Kayıtlı kullanıcı adın ve e-posta adresini gir, sana bir yenileme bağlantısı gönderelim.</p>

    <?php if ($sent): ?>
      <div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-4 text-sm">
        Bilgiler eşleşiyorsa, e-posta adresine bir şifre yenileme bağlantısı gönderdik.
      </div>
    <?php else: ?>
      <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-4"><?= jstko_e($error) ?></div><?php endif; ?>
      <form method="post" class="space-y-4">
        <?= jstko_csrf_field() ?>
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">KULLANICI ADI</label>
          <input class="input" type="text" name="account_id" required maxlength="21">
        </div>
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">E-POSTA ADRESİ</label>
          <input class="input" type="email" name="email" required maxlength="50">
        </div>
        <button type="submit" class="btn-primary w-full">Bağlantı Gönder</button>
      </form>
    <?php endif; ?>
    <p class="text-sm text-slate-400 mt-5"><a href="<?= JSTKO_BASE ?>/login.php" class="text-gold-400 hover:underline">Girişe dön</a></p>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
