<?php
declare(strict_types=1);

/** Saves an uploaded image under public_html/uploads/, returns the stored filename or null if no file was sent. */
function jstko_admin_save_upload(string $fileField): ?string
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
    $name = bin2hex(random_bytes(8)) . '.' . $allowed[$info['mime']];
    $dest = __DIR__ . '/../public_html/uploads/' . $name;
    if (!move_uploaded_file($file['tmp_name'], $dest)) {
        throw new RuntimeException('Dosya kaydedilemedi.');
    }
    return $name;
}
