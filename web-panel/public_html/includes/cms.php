<?php
declare(strict_types=1);

require_once __DIR__ . '/db.php';

/**
 * Recommended picture size per upload slot, shown next to every upload field and enforced
 * (proportions) by jstko_admin_save_upload().
 */
function jstko_image_specs(): array
{
    return [
        'hero_image'    => [1920, 800,  'Anasayfa üst arka planı'],
        'site_bg_image' => [1920, 1080, 'Tüm sitenin arka plan resmi'],
        'logo_image'    => [300,  80,   'Logo (şeffaf PNG önerilir)'],
        'slide'         => [1920, 700,  'Slider görseli'],
        'home_top'      => [1200, 160,  'Anasayfa üst banner (geniş)'],
        'home_mid'      => [1200, 200,  'Anasayfa orta banner (geniş)'],
        'home_side'     => [360,  480,  'Anasayfa yan banner (dikey)'],
        'page_top'      => [1200, 160,  'Sayfa üstü banner'],
        'page_header'   => [1920, 400,  'Sayfa başlık görseli'],
        'news'          => [1200, 630,  'Haber görseli'],
        'video_poster'  => [1280, 720,  'Anasayfa tanıtım alanı görseli'],
    ];
}

/** [w, h] for a slot, or null for an unknown one (content keys that are not listed accept any size). */
function jstko_image_spec(string $slot): ?array
{
    $s = jstko_image_specs()[$slot] ?? null;
    return $s ? [$s[0], $s[1]] : null;
}

function jstko_image_hint(string $slot): string
{
    $s = jstko_image_specs()[$slot] ?? null;
    return $s ? sprintf('%s: %d × %d px (JPG/PNG/WEBP, en çok 5 MB)', $s[2], $s[0], $s[1]) : 'JPG/PNG/WEBP, en çok 5 MB';
}

/** Script shown once per page: warns in the browser when the chosen picture has the wrong size. */
function jstko_image_check_script(): string
{
    return <<<'JS'
<script>
document.querySelectorAll('input[type=file][data-w]').forEach(function (inp) {
  inp.addEventListener('change', function () {
    var f = inp.files && inp.files[0]; var out = inp.parentElement.querySelector('.size-msg');
    if (!out) { out = document.createElement('div'); out.className = 'size-msg text-xs mt-1'; inp.parentElement.appendChild(out); }
    if (!f) { out.textContent = ''; return; }
    var img = new Image();
    img.onload = function () {
      var w = +inp.dataset.w, h = +inp.dataset.h, ok = Math.abs((img.width / img.height) - (w / h)) / (w / h) <= 0.12 && img.width >= w * 0.6;
      out.className = 'size-msg text-xs mt-1 ' + (ok ? 'text-emerald-400' : 'text-red-400');
      out.textContent = 'Seçilen: ' + img.width + '×' + img.height + ' px — ' + (ok ? 'uygun' : 'UYGUN DEĞİL, ' + w + '×' + h + ' px olmalı');
    };
    img.src = URL.createObjectURL(f);
  });
});
</script>
JS;
}

/** Upload <input> with its size hint (and browser-side check) for a slot. */
function jstko_image_input(string $name, string $slot): string
{
    $s = jstko_image_spec($slot);
    $data = $s ? ' data-w="' . $s[0] . '" data-h="' . $s[1] . '"' : '';
    return '<div><input type="file" name="' . jstko_e($name) . '" accept="image/*" class="text-xs text-slate-400"' . $data . '>'
        . '<div class="text-xs text-amber-300/80 mt-1">' . jstko_e(jstko_image_hint($slot)) . '</div></div>';
}

/** Runs a query, returning [] when the CMS tables are not migrated yet (so the public site never breaks). */
function jstko_cms_rows(string $sql, array $params = []): array
{
    try {
        $st = jstko_db()->prepare($sql);
        $st->execute($params);
        return $st->fetchAll();
    } catch (Throwable $e) {
        error_log('jstko_cms_rows: ' . $e->getMessage());
        return [];
    }
}

function jstko_cms_slides(): array
{
    return jstko_cms_rows('SELECT * FROM web_slide WHERE active ORDER BY sort_order, id');
}

function jstko_cms_banners(string $position): array
{
    return jstko_cms_rows('SELECT * FROM web_banner WHERE active AND position = :p ORDER BY sort_order, id', ['p' => $position]);
}

/** Menu tree: top-level items each with ['children' => [...]]. Empty when the table is missing (header falls back). */
function jstko_cms_menu(): array
{
    $rows = jstko_cms_rows('SELECT * FROM web_menu WHERE active ORDER BY sort_order, id');
    $top = [];
    $kids = [];
    foreach ($rows as $r) {
        if ($r['parent_id'] === null) {
            $r['children'] = [];
            $top[(int)$r['id']] = $r;
        } else {
            $kids[] = $r;
        }
    }
    foreach ($kids as $k) {
        if (isset($top[(int)$k['parent_id']])) {
            $top[(int)$k['parent_id']]['children'][] = $k;
        }
    }
    return array_values($top);
}

function jstko_banner_html(string $position): string
{
    $out = '';
    foreach (jstko_cms_banners($position) as $b) {
        $img = '<img src="/uploads/' . jstko_e(ltrim($b['image'], '/')) . '" alt="' . jstko_e($b['alt']) . '" class="w-full h-auto rounded-xl border border-ink-500">';
        $out .= $b['link_url'] !== ''
            ? '<a href="' . jstko_e($b['link_url']) . '" class="block">' . $img . '</a>'
            : '<div>' . $img . '</div>';
    }
    return $out;
}

/** Latest active patch (newest first), or null. */
function jstko_cms_latest_patch(): ?array
{
    $r = jstko_cms_rows('SELECT * FROM web_patch WHERE active ORDER BY created_at DESC, id DESC LIMIT 1');
    return $r[0] ?? null;
}
