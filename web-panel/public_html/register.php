<?php
require_once __DIR__ . '/includes/bootstrap.php';

if (jstko_current_user()) {
    jstko_redirect('/account.php');
}

$error = null;
$success = false;
$old = [];

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $old = $_POST;
    if (!jstko_csrf_check()) {
        $error = 'Form süresi doldu, lütfen tekrar deneyin.';
    } elseif (!jstko_rate_limit('register', jstko_client_ip(), 5, 600)) {
        $error = 'Çok fazla kayıt denemesi yaptınız, lütfen daha sonra tekrar deneyin.';
    } elseif (($_POST['password'] ?? '') !== ($_POST['password_confirm'] ?? '')) {
        $error = 'Şifreler eşleşmiyor.';
    } elseif (empty($_POST['accept_terms'])) {
        $error = 'Devam etmek için kullanıcı sözleşmesini kabul etmelisiniz.';
    } else {
        $result = jstko_register($_POST);
        if ($result['ok']) {
            $success = true;
            $old = [];
        } else {
            $error = $result['error'];
        }
    }
}

$securityQuestions = [
    'anne_kizlik' => 'Annenizin kızlık soyadı nedir?',
    'ilk_evcil' => 'İlk evcil hayvanınızın adı nedir?',
    'ilkokul' => 'İlkokul öğretmeninizin adı nedir?',
    'dogum_sehri' => 'Doğduğunuz şehir neresi?',
    'ilk_araba' => 'İlk arabanızın markası nedir?',
];

$pageTitle = 'Hesap Oluştur';
require __DIR__ . '/partials/header.php';
?>
<section class="max-w-xl mx-auto px-4 py-12">
  <div class="card p-8">
    <h1 class="text-2xl font-bold text-white mb-1">Hesap Oluştur</h1>
    <p class="text-sm text-slate-400 mb-6">Kayıt sonrası e-postana gelecek bağlantıya tıklayarak hesabını aktifleştirmen gerekir.</p>

    <?php if ($success): ?>
      <div class="bg-emerald-900/40 border border-emerald-700 text-emerald-200 rounded-lg px-4 py-4 text-sm">
        Kaydın alındı! E-posta adresine gönderdiğimiz aktivasyon bağlantısına tıklayarak hesabını aktifleştirebilirsin.
        Mail gelmediyse spam/gereksiz klasörünü de kontrol et.
      </div>
    <?php else: ?>
      <?php if ($error): ?>
        <div class="bg-red-900/40 border border-red-700 text-red-200 rounded-lg px-4 py-3 text-sm mb-4"><?= jstko_e($error) ?></div>
      <?php endif; ?>

      <form method="post" class="space-y-4">
        <?= jstko_csrf_field() ?>
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">KULLANICI ADI (ID)</label>
          <input class="input" type="text" name="account_id" required minlength="3" maxlength="21" pattern="[A-Za-z0-9:_]+"
                 value="<?= jstko_e($old['account_id'] ?? '') ?>">
          <p class="text-xs text-slate-500 mt-1">3-21 karakter, sadece harf/rakam/"_"/":" kullanılabilir.</p>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-xs font-medium text-slate-400 mb-1.5">ŞİFRE</label>
            <input class="input" type="password" name="password" required minlength="6" maxlength="16">
          </div>
          <div>
            <label class="block text-xs font-medium text-slate-400 mb-1.5">ŞİFRE (TEKRAR)</label>
            <input class="input" type="password" name="password_confirm" required minlength="6" maxlength="16">
          </div>
        </div>

        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">GİZLİ SORU</label>
          <select class="input" name="security_question" required>
            <option value="">Gizli soru seçin</option>
            <?php foreach ($securityQuestions as $val => $label): ?>
              <option value="<?= jstko_e($val) ?>" <?= ($old['security_question'] ?? '') === $val ? 'selected' : '' ?>><?= jstko_e($label) ?></option>
            <?php endforeach; ?>
          </select>
        </div>
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">GİZLİ CEVABINIZ</label>
          <input class="input" type="text" name="security_answer" required maxlength="50" value="<?= jstko_e($old['security_answer'] ?? '') ?>">
        </div>

        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">E-POSTA ADRESİ</label>
          <input class="input" type="email" name="email" required maxlength="50" value="<?= jstko_e($old['email'] ?? '') ?>">
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-xs font-medium text-slate-400 mb-1.5">8 HANELİ İTEM KİLİT ŞİFRESİ</label>
            <input class="input" type="password" name="seal_password" required pattern="\d{8}" maxlength="8" inputmode="numeric">
          </div>
          <div>
            <label class="block text-xs font-medium text-slate-400 mb-1.5">6 HANELİ OTP ŞİFRESİ</label>
            <input class="input" type="password" name="otp_password" required pattern="\d{6}" maxlength="6" inputmode="numeric">
          </div>
        </div>

        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">TELEFON (OPSİYONEL)</label>
          <input class="input" type="text" name="phone" maxlength="10" placeholder="5xxxxxxxxx" value="<?= jstko_e($old['phone'] ?? '') ?>">
        </div>

        <label class="flex items-start gap-2 text-sm text-slate-400">
          <input type="checkbox" name="accept_terms" value="1" class="mt-1" required>
          <span><a href="/terms.php" target="_blank" class="text-gold-400 hover:underline">Kullanıcı Sözleşmesi</a>'ni okudum, kabul ediyorum.</span>
        </label>

        <button type="submit" class="btn-primary w-full">Kabul Et &amp; Kayıt Ol</button>
      </form>

      <p class="text-sm text-slate-400 mt-5">Zaten hesabın var mı? <a href="/login.php" class="text-gold-400 hover:underline font-semibold">Giriş Yap</a></p>
    <?php endif; ?>
  </div>
</section>
<?php require __DIR__ . '/partials/footer.php'; ?>
