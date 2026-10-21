<?php
declare(strict_types=1);

/**
 * Saves an uploaded image under public_html/uploads/, returns the stored filename or null if no file was sent.
 * $spec = [width, height] is the recommended size: a picture whose proportions are more than 12 % off
 * the recommended ones, or that is smaller than 60 % of it, is refused (so the wrong picture is never
 * stretched into a slot).
 */
function jstko_admin_save_upload(string $fileField, ?array $spec = null): ?string
{
    if (empty($_FILES[$fileField]) || $_FILES[$fileField]['error'] === UPLOAD_ERR_NO_FILE) {
        return null;
    }
    $file = $_FILES[$fileField];
    if ($file['error'] !== UPLOAD_ERR_OK) {
        throw new RuntimeException('Dosya yüklenemedi (kod ' . $file['error'] . ').');
    }
    if ($file['size'] > 5 * 1024 * 1024) {
        throw new RuntimeException('Dosya 5 MB sınırını aşıyor.');
    }
    $info = @getimagesize($file['tmp_name']);
    $allowed = ['image/jpeg' => 'jpg', 'image/png' => 'png', 'image/webp' => 'webp', 'image/gif' => 'gif'];
    if (!$info || !isset($allowed[$info['mime']])) {
        throw new RuntimeException('Sadece JPG, PNG, WEBP veya GIF resim yüklenebilir.');
    }
    if ($spec !== null) {
        [$w, $h] = [(int)$info[0], (int)$info[1]];
        $ratioWant = $spec[0] / $spec[1];
        $ratioGot = $w / max(1, $h);
        if (abs($ratioGot - $ratioWant) / $ratioWant > 0.12 || $w < $spec[0] * 0.6) {
            throw new RuntimeException(sprintf(
                'Resim boyutu uygun değil: yüklenen %d×%d px, bu alan için %d×%d px olmalı (oran yaklaşık aynı, genişlik en az %d px).',
                $w, $h, $spec[0], $spec[1], (int)($spec[0] * 0.6)
            ));
        }
    }
    $name = bin2hex(random_bytes(8)) . '.' . $allowed[$info['mime']];
    $dest = __DIR__ . '/../uploads/' . $name;
    if (!move_uploaded_file($file['tmp_name'], $dest)) {
        throw new RuntimeException('Dosya kaydedilemedi.');
    }
    return $name;
}

/** Saves an uploaded patch archive under uploads/patches/, returns the public path (/uploads/patches/x.zip) or null. */
function jstko_admin_save_patch_file(string $fileField): ?string
{
    if (empty($_FILES[$fileField]) || $_FILES[$fileField]['error'] === UPLOAD_ERR_NO_FILE) {
        return null;
    }
    $file = $_FILES[$fileField];
    if ($file['error'] !== UPLOAD_ERR_OK) {
        throw new RuntimeException('Patch dosyası yüklenemedi (kod ' . $file['error'] . '). Hosting yükleme sınırı küçük olabilir; büyük dosya için "Harici bağlantı" kullan.');
    }
    $ext = strtolower(pathinfo((string)$file['name'], PATHINFO_EXTENSION));
    if (!in_array($ext, ['zip', '7z', 'rar', 'exe'], true)) {
        throw new RuntimeException('Sadece zip, 7z, rar veya exe yüklenebilir.');
    }
    $dir = __DIR__ . '/../uploads/patches';
    if (!is_dir($dir) && !@mkdir($dir, 0755, true)) {
        throw new RuntimeException('uploads/patches klasörü oluşturulamadı.');
    }
    $base = preg_replace('/[^A-Za-z0-9._-]/', '_', pathinfo((string)$file['name'], PATHINFO_FILENAME));
    $name = substr($base, 0, 60) . '_' . bin2hex(random_bytes(3)) . '.' . $ext;
    if (!move_uploaded_file($file['tmp_name'], $dir . '/' . $name)) {
        throw new RuntimeException('Patch dosyası kaydedilemedi.');
    }
    return '/uploads/patches/' . $name;
}
