<?php
// JSON for the item tooltip (hover on any item picture). Public data only: the same columns the game client shows.
require_once __DIR__ . '/includes/bootstrap.php';

header('Content-Type: application/json; charset=utf-8');
header('Cache-Control: public, max-age=3600');

$id = (int)($_GET['id'] ?? 0);
if ($id <= 0) { echo 'null'; exit; }

$cols = 'num, str_name, description, item_icon_id1, kind, damage, min_damage, max_damage, delay, "range", weight, duration, buy_price,'
    . ' ac, req_level, req_rank, req_str, req_sta, req_dex, req_intel, req_cha, hitrate, evasionrate,'
    . ' dagger_ac, jamadar_ac, sword_ac, club_ac, axe_ac, spear_ac, bow_ac, mace_ac,'
    . ' fire_damage, ice_damage, lightning_damage, poison_damage, hp_drain, mp_damage, mp_drain, mirror_damage,'
    . ' str_b, sta_b, dex_b, intel_b, cha_b, max_hp_b, max_mp_b, fire_r, cold_r, lightning_r, magic_r, poison_r, curse_r,'
    . ' item_class, bound, by_grade';
try {
    $st = jstko_db()->prepare("SELECT $cols FROM item WHERE num = :n");
    $st->execute(['n' => $id]);
    $row = $st->fetch();
} catch (Throwable $e) {
    $row = false;
}
if (!$row) { echo 'null'; exit; }
$row['icon'] = jstko_item_icon_url(($row['item_icon_id1'] ?? null) ? (int)$row['item_icon_id1'] : null);
echo json_encode($row, JSON_UNESCAPED_UNICODE);
