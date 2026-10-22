<?php
require_once __DIR__ . '/includes/bootstrap.php';

if (jstko_current_user()) {
    jstko_redirect('/account.php');
}

$error = null;
$next = $_GET['next'] ?? '/account.php';
if (!is_string($next) || !str_starts_with($next, '/')) {
    $next = '/account.php';
}

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } elseif (!jstko_rate_limit('login', jstko_client_ip(), 10, 300)) {
        $error = 'Çok fazla deneme yaptınız, birkaç dakika sonra tekrar deneyin.';
    } else {
        $result = jstko_login((string)($_POST['account_id'] ?? ''), (string)($_POST['password'] ?? ''));
        if ($result['ok']) {
            jstko_redirect($next);
        }
        $error = $result['error'];
    }
}

$pageTitle = 'Giriş Yap';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-md mx-auto px-4 py-16">
  <div class="card p-8">
    <h1 class="text-2xl font-bold text-white mb-1">Hesabınıza Giriş Yapın</h1>
    <p class="text-sm text-slate-400 mb-6">Kullanıcı adınız ve şifrenizle güvenli giriş yapın.</p>

    <?php if ($error): ?>
      <div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-4"><?= jstko_e($error) ?></div>
    <?php endif; ?>

    <form method="post" class="space-y-4">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="next" value="<?= jstko_e($next) ?>">
      <div>
        <label class="block text-xs font-medium text-slate-400 mb-1.5">KULLANICI ADI</label>
        <input class="input" type="text" name="account_id" required maxlength="21" autofocus>
      </div>
      <div>
        <label class="block text-xs font-medium text-slate-400 mb-1.5">ŞİFRE</label>
        <div class="relative">
          <input class="input pr-10" type="password" name="password" id="loginPw" required maxlength="16">
          <button type="button" data-toggle-password="loginPw" class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-500 text-xs">GÖSTER</button>
        </div>
      </div>
      <button type="submit" class="btn-primary w-full">Giriş Yap</button>
    </form>

    <div class="flex items-center justify-between mt-5 text-sm text-slate-400">
      <a href="<?= JSTKO_BASE ?>/forgot_password.php" class="hover:text-gold-400">Şifremi Unuttum</a>
      <a href="<?= JSTKO_BASE ?>/register.php" class="hover:text-gold-400 font-semibold">Hesap Oluştur</a>
    </div>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
