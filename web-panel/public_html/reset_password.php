<?php
require_once __DIR__ . '/includes/bootstrap.php';

$token = (string)($_GET['token'] ?? $_POST['token'] ?? '');
$account = jstko_password_reset_token_account($token);
$error = null;
$success = false;

if ($account && $_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } elseif (($_POST['password'] ?? '') !== ($_POST['password_confirm'] ?? '')) {
        $error = 'Şifreler eşleşmiyor.';
    } else {
        $result = jstko_reset_password($token, (string)($_POST['password'] ?? ''));
        if ($result['ok']) {
            $success = true;
        } else {
            $error = $result['error'];
        }
    }
}

$pageTitle = 'Şifre Yenile';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-md mx-auto px-4 py-16">
  <div class="card p-8">
    <?php if (!$account && !$success): ?>
      <h1 class="text-xl font-bold text-white mb-2">Bağlantı Geçersiz</h1>
      <p class="text-sm text-slate-400 mb-6">Bu şifre yenileme bağlantısının süresi dolmuş ya da zaten kullanılmış.</p>
      <a href="/forgot_password.php" class="text-gold-400 hover:underline text-sm">Yeni bağlantı iste</a>
    <?php elseif ($success): ?>
      <h1 class="text-xl font-bold text-white mb-2">Şifren Güncellendi</h1>
      <p class="text-sm text-slate-400 mb-6">Yeni şifrenle giriş yapabilirsin.</p>
      <a href="/login.php" class="btn-primary inline-block">Giriş Yap</a>
    <?php else: ?>
      <h1 class="text-2xl font-bold text-white mb-1">Yeni Şifre Belirle</h1>
      <p class="text-sm text-slate-400 mb-6">Hesap: <span class="text-white font-semibold"><?= jstko_e($account) ?></span></p>
      <?php if ($error): ?><div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-4"><?= jstko_e($error) ?></div><?php endif; ?>
      <form method="post" class="space-y-4">
        <?= jstko_csrf_field() ?>
        <input type="hidden" name="token" value="<?= jstko_e($token) ?>">
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">YENİ ŞİFRE</label>
          <input class="input" type="password" name="password" required minlength="6" maxlength="16">
        </div>
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">YENİ ŞİFRE (TEKRAR)</label>
          <input class="input" type="password" name="password_confirm" required minlength="6" maxlength="16">
        </div>
        <button type="submit" class="btn-primary w-full">Şifreyi Güncelle</button>
      </form>
    <?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
