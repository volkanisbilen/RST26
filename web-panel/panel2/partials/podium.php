<?php
/**
 * Top-3 podium used by the ranking pages (the home page keeps its own inline copy).
 * Each row: name, sub (clan / extra line), nation (1 Karus / 2 El Morad), lvl (int|null), value (formatted), unit, online (bool).
 */
function jstko_podium_html(array $rows): string
{
    $rows = array_slice($rows, 0, 3);
    if (!$rows) return '';
    $medal = [0 => 'gold', 1 => 'silver', 2 => 'bronze'];
    $out = '<div class="podium">';
    foreach ([1, 0, 2] as $pi) {
        if (!isset($rows[$pi])) continue;
        $r = $rows[$pi];
        $nat = (int)$r['nation'] === 1 ? 'karus' : 'elmo';
        $out .= '<div class="pod pod-' . $medal[$pi] . ' pod-' . ($pi + 1) . '">'
            . '<div class="pod-rank">' . ($pi + 1) . '</div>'
            . '<div class="pod-avatar"><span class="pod-ring"></span>'
            . '<svg class="pod-crown" viewBox="0 0 24 24" fill="currentColor"><path d="M3 8l4.5 4L12 5l4.5 7L21 8l-2 11H5zm2.3 13h13.4v2H5.3z"/></svg>'
            . ($pi === 0 ? '<i class="spark s1"></i><i class="spark s2"></i><i class="spark s3"></i><i class="spark s4"></i>' : '')
            . '</div>'
            . '<div class="pod-name" title="' . jstko_e($r['name']) . '">' . jstko_e($r['name'])
            . (!empty($r['online']) ? '<span class="dot-on" title="Online"></span>' : '') . '</div>'
            . '<div class="pod-clan">' . jstko_e($r['sub'] ?? '') . '</div>'
            . '<div class="pod-meta"><span class="chip-nation ' . $nat . '">' . ($nat === 'karus' ? 'Karus' : 'El Morad') . '</span>'
            . (isset($r['lvl']) && $r['lvl'] !== null ? '<span class="chip-lvl">Lv ' . (int)$r['lvl'] . '</span>' : '') . '</div>'
            . '<div class="pod-np">' . jstko_e($r['value']) . '<small>' . jstko_e($r['unit'] ?? 'NP') . '</small></div>'
            . '<div class="pod-base"></div></div>';
    }
    return $out . '</div>';
}
