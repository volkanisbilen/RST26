<?php
require_once __DIR__ . '/includes/bootstrap.php';
require_once __DIR__ . '/includes/rescue.php';

$user = jstko_require_login();
$message = null;
$messageType = 'success';

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    if (!jstko_csrf_check()) {
        $message = 'Form süresi doldu, lütfen tekrar deneyin.';
        $messageType = 'error';
    } elseif (!jstko_rate_limit('rescue', jstko_client_ip(), 10, 300)) {
        $message = 'Çok fazla deneme yaptınız, lütfen biraz bekleyin.';
        $messageType = 'error';
    } else {
        $result = jstko_rescue_character($user['str_account_id'], (string)($_POST['char_name'] ?? ''));
        $message = $result['message'];
        $messageType = $result['ok'] ? 'success' : 'error';
    }
}

$chars = jstko_account_characters($user['str_account_id']);

$pageTitle = 'Karakter Kurtar';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-2xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Karakter Kurtar</h1>
  <p class="text-sm text-slate-400 mb-6">
    "Zaten oyunda" hatası alıp giriş yapamıyorsanız, karakteriniz sunucuda askıda kalmış olabilir. Aşağıdan kurtarabilirsiniz.
  </p>

  <?php if ($message): ?>
    <div class="<?= $messageType === 'success' ? 'bg-emerald-900/40 border-emerald-700 text-emerald-200' : 'bg-red-900/40 border-red-700 text-red-200' ?> border rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($message) ?></div>
  <?php endif; ?>

  <?php if (!$chars): ?>
    <div class="card p-10 text-center text-slate-500">Hesabınıza kayıtlı karakter bulunamadı.</div>
  <?php else: ?>
  <div class="space-y-3">
    <?php foreach ($chars as $c): ?>
      <div class="card p-4 flex items-center justify-between gap-4">
        <div>
          <div class="font-semibold text-white"><?= jstko_e($c['name']) ?></div>
          <div class="text-xs text-slate-400"><?= jstko_e(jstko_nation_name((int)$c['nation'])) ?> · <?= jstko_e(jstko_class_name((int)$c['class'])) ?> · Sv.<?= (int)$c['level'] ?></div>
        </div>
        <?php if (jstko_pg_bool($c['stuck'])): ?>
          <form method="post">
            <?= jstko_csrf_field() ?>
            <input type="hidden" name="char_name" value="<?= jstko_e($c['name']) ?>">
            <button type="submit" class="text-sm px-4 py-2 rounded-lg bg-red-600 hover:bg-red-500 text-white font-semibold transition">Askıdan Kurtar</button>
          </form>
        <?php else: ?>
          <span class="text-xs px-3 py-1.5 rounded-full bg-emerald-900/30 border border-emerald-700 text-emerald-300">Normal</span>
        <?php endif; ?>
      </div>
    <?php endforeach; ?>
  </div>
  <?php endif; ?>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
