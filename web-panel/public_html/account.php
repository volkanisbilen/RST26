<?php
require_once __DIR__ . '/includes/bootstrap.php';
require_once __DIR__ . '/includes/rescue.php';
require_once __DIR__ . '/includes/pus.php';

$user = jstko_require_login();
$notice = null;
$noticeType = 'success';

if ($_SERVER['REQUEST_METHOD'] === 'POST' && jstko_csrf_check()) {
    $action = $_POST['action'] ?? '';

    if ($action === 'redeem') {
        if (!jstko_rate_limit('pus_redeem', jstko_client_ip(), 15, 600)) {
            $notice = 'Çok fazla deneme yaptınız, lütfen biraz bekleyin.';
            $noticeType = 'error';
        } else {
            $result = jstko_pus_redeem($user['str_account_id'], '', jstko_client_ip(), (string)($_POST['code'] ?? ''));
            $notice = $result['message'];
            $noticeType = $result['ok'] ? 'success' : 'error';
            if ($result['ok']) {
                $stmt = jstko_db()->prepare('SELECT * FROM tb_user WHERE str_account_id = :id');
                $stmt->execute(['id' => $user['str_account_id']]);
                $user = $stmt->fetch();
            }
        }
    } elseif ($action === 'change_password') {
        $current = (string)($_POST['current_password'] ?? '');
        $new = (string)($_POST['new_password'] ?? '');
        if (!hash_equals((string)$user['str_passwd'], jstko_ko_password_encode($current))) {
            $notice = 'Mevcut şifreniz hatalı.';
            $noticeType = 'error';
        } elseif (strlen($new) < 6 || strlen($new) > 16) {
            $notice = 'Yeni şifre 6-16 karakter olmalı.';
            $noticeType = 'error';
        } else {
            jstko_db()->prepare('UPDATE tb_user SET str_passwd = :pw WHERE str_account_id = :id')
                ->execute(['pw' => jstko_ko_password_encode($new), 'id' => $user['str_account_id']]);
            $notice = 'Şifreniz güncellendi.';
        }
    }
} elseif ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $notice = 'Form süresi doldu, lütfen tekrar deneyin.';
    $noticeType = 'error';
}

$chars = jstko_account_characters($user['str_account_id']);

$pageTitle = 'Hesabım';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-4xl mx-auto px-4 py-10">
  <h1 class="text-2xl font-bold text-white mb-1">Hesabım</h1>
  <p class="text-sm text-slate-400 mb-6"><?= jstko_e($user['str_account_id']) ?></p>

  <?php if ($notice): ?>
    <div class="<?= $noticeType === 'success' ? 'bg-emerald-900/40 border-emerald-700 text-emerald-200' : 'bg-red-900/40 border-red-700 text-red-200' ?> border rounded-lg px-4 py-3 text-sm mb-6"><?= jstko_e($notice) ?></div>
  <?php endif; ?>

  <div class="grid md:grid-cols-2 gap-4 mb-8">
    <div class="card p-6 text-center">
      <div class="text-2xl font-extrabold text-gold-400"><?= jstko_number($user['cash_point']) ?></div>
      <div class="text-xs text-slate-400 mt-1 uppercase tracking-wide">Knight Cash (KC)</div>
    </div>
    <div class="card p-6 text-center">
      <div class="text-2xl font-extrabold text-sky-400"><?= jstko_number($user['bonus_cash_point']) ?></div>
      <div class="text-xs text-slate-400 mt-1 uppercase tracking-wide">TL</div>
    </div>
  </div>

  <div class="card p-6 mb-8">
    <h2 class="font-bold text-white mb-3">PUS Kod Kullan</h2>
    <form method="post" class="flex flex-col sm:flex-row gap-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="action" value="redeem">
      <input class="input flex-1" type="text" name="code" placeholder="XXXX-XXXX-XXXX-XXXX" required maxlength="40">
      <button type="submit" class="btn-primary whitespace-nowrap">Kodu Kullan</button>
    </form>
  </div>

  <div class="card p-6 mb-8">
    <h2 class="font-bold text-white mb-3">Karakterlerim</h2>
    <?php if (!$chars): ?>
      <p class="text-sm text-slate-500">Henüz karakter oluşturulmamış.</p>
    <?php else: ?>
    <div class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead class="text-left text-slate-500 text-xs uppercase">
          <tr><th class="py-2 pr-3">Karakter</th><th class="py-2 pr-3">Ulus</th><th class="py-2 pr-3">Sınıf</th><th class="py-2 pr-3">Seviye</th><th class="py-2">Durum</th></tr>
        </thead>
        <tbody>
          <?php foreach ($chars as $c): ?>
          <tr class="table-row border-t border-ink-500/50">
            <td class="py-2 pr-3 font-medium text-white"><?= jstko_e($c['name']) ?></td>
            <td class="py-2 pr-3"><?= jstko_e(jstko_nation_name((int)$c['nation'])) ?></td>
            <td class="py-2 pr-3"><?= jstko_e(jstko_class_name((int)$c['class'])) ?></td>
            <td class="py-2 pr-3"><?= (int)$c['level'] ?></td>
            <td class="py-2"><?= jstko_pg_bool($c['stuck']) ? '<span class="text-amber-400">Askıda — Kurtar sayfasına bakın</span>' : '<span class="text-emerald-400">Normal</span>' ?></td>
          </tr>
          <?php endforeach; ?>
        </tbody>
      </table>
    </div>
    <?php endif; ?>
  </div>

  <div class="card p-6">
    <h2 class="font-bold text-white mb-3">Şifre Değiştir</h2>
    <form method="post" class="grid sm:grid-cols-3 gap-3">
      <?= jstko_csrf_field() ?>
      <input type="hidden" name="action" value="change_password">
      <input class="input" type="password" name="current_password" placeholder="Mevcut şifre" required maxlength="16">
      <input class="input" type="password" name="new_password" placeholder="Yeni şifre" required minlength="6" maxlength="16">
      <button type="submit" class="btn-primary">Güncelle</button>
    </form>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
