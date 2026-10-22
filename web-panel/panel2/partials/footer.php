
</main>

<?php
$B = JSTKO_BASE;
$fLogo = jstko_c_image('logo_image');
$fName = jstko_c('site_name', 'JstKO');
$socials = [
    'discord_url'   => ['Discord',   '<path d="M20 5.5A17 17 0 0 0 15.8 4l-.5 1a15 15 0 0 0-4.6 0l-.5-1A17 17 0 0 0 4 5.5C1.700 9 1 12.400 1.300 15.700A17 17 0 0 0 6.400 18.300l1-1.600a11 11 0 0 1-1.700-.8l.4-.3a12 12 0 0 0 10.200 0l.4.300a11 11 0 0 1-1.700.8l1 1.600a17 17 0 0 0 5.100-2.600C23.100 11.800 22 8.400 20 5.500zM8.700 14c-1 0-1.800-.9-1.800-2s.8-2 1.800-2 1.800.9 1.800 2-.8 2-1.800 2zm6.600 0c-1 0-1.800-.9-1.800-2s.8-2 1.800-2 1.800.9 1.800 2-.8 2-1.800 2z"/>'],
    'youtube_url'   => ['YouTube',   '<path d="M21.600 7.200a2.500 2.500 0 0 0-1.800-1.800C18.200 5 12 5 12 5s-6.200 0-7.800.4A2.500 2.500 0 0 0 2.400 7.200C2 8.800 2 12 2 12s0 3.200.4 4.800a2.500 2.500 0 0 0 1.800 1.800C5.800 19 12 19 12 19s6.200 0 7.800-.4a2.500 2.500 0 0 0 1.800-1.800C22 15.200 22 12 22 12s0-3.200-.4-4.800zM10 15V9l5.200 3z"/>'],
    'facebook_url'  => ['Facebook',  '<path d="M14 8V6.500c0-.7.200-1 1.200-1H17V2.200C16.600 2.100 15.600 2 14.600 2 12.100 2 10.500 3.500 10.500 6.100V8H8v3.500h2.500V22H14V11.500h2.700L17.200 8z"/>'],
    'instagram_url' => ['Instagram', '<path d="M7.500 2h9A5.500 5.500 0 0 1 22 7.500v9a5.500 5.500 0 0 1-5.500 5.500h-9A5.500 5.500 0 0 1 2 16.500v-9A5.500 5.500 0 0 1 7.500 2zm0 2A3.500 3.500 0 0 0 4 7.500v9A3.500 3.500 0 0 0 7.500 20h9a3.500 3.500 0 0 0 3.500-3.500v-9A3.500 3.500 0 0 0 16.500 4zM12 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10zm0 2a3 3 0 1 0 0 6 3 3 0 0 0 0-6zm5.500-3.200a1.200 1.200 0 1 1 0 2.400 1.200 1.200 0 0 1 0-2.400z"/>'],
];
?>
<footer class="site-footer mt-10">
  <div class="mx-auto max-w-[1400px] px-4 lg:px-6 py-5 grid gap-4 md:grid-cols-3 items-center text-xs text-slate-400">
    <div class="flex items-center gap-3 justify-center md:justify-start">
      <?php if ($fLogo): ?><img src="<?= jstko_e($fLogo) ?>" alt="<?= jstko_e($fName) ?>" class="h-9 w-auto"><?php else: ?><span class="font-display font-extrabold text-xl text-gold-400"><?= jstko_e($fName) ?></span><?php endif; ?>
      <span class="hidden sm:inline">Legends Never Fade</span>
      <span class="hidden sm:inline text-slate-600">|</span>
      <span class="hidden sm:inline"><?= jstko_e($fName) ?> <?= date('Y') ?></span>
    </div>
    <div class="flex items-center justify-center gap-5">
      <?php foreach ($socials as $key => [$label, $path]): $url = jstko_c($key); if ($url === '') continue; ?>
        <a href="<?= jstko_e($url) ?>" target="_blank" rel="noopener" aria-label="<?= $label ?>" class="text-slate-300 hover:text-gold-400 transition">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><?= $path ?></svg>
        </a>
      <?php endforeach; ?>
    </div>
    <div class="flex flex-wrap items-center justify-center md:justify-end gap-x-4 gap-y-1">
      <span class="inline-flex items-center gap-1.5 text-emerald-400"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>Sunucu Online</span>
      <a href="<?= $B ?>/terms.php" class="hover:text-gold-400">Kullanım Şartları</a>
      <a href="<?= $B ?>/support.php" class="hover:text-gold-400">İletişim</a>
      <?php if ($ft = jstko_c('footer_text')): ?><span><?= jstko_e($ft) ?></span><?php endif; ?>
    </div>
  </div>
</footer>

<script src="<?= $B ?>/assets/js/app.js"></script>
</body>
</html>
