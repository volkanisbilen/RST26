
</main>

<footer class="bg-ink-800 border-t border-ink-500 mt-12">
  <div class="max-w-6xl mx-auto px-4 py-8 text-sm text-slate-400 flex flex-col sm:flex-row items-center justify-between gap-4">
    <div>&copy; <?= date('Y') ?> <?= jstko_e(jstko_c('site_name', 'JstKO')) ?>. Tüm hakları saklıdır.</div>
    <div class="flex items-center gap-4">
      <?php if ($discord = jstko_c('discord_url')): ?><a href="<?= jstko_e($discord) ?>" class="hover:text-gold-400">Discord</a><?php endif; ?>
      <a href="/support.php" class="hover:text-gold-400">Destek</a>
    </div>
  </div>
</footer>

<script src="/assets/js/app.js"></script>
</body>
</html>
