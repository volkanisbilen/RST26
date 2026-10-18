'use strict';
/* Saha Botları: normal haritalarda (Moradon, Luferson, El Morad, Eslant) canavar avlayan ve
   party'ye davet edilebilen botlar. Her bölge varsayılan KAPALI; kaydedince oyun sunucusuna
   field_bot_reload komutu sıraya girer (admin_command), sunucu birkaç saniyede uygular.
   Üst sınırlar oyun sunucusunda da zorlanır: bölge/ırk başına 40, toplam 150. */

const FB_MIX = [['Savaşçı', 0], ['Rogue', 1], ['Büyücü', 2], ['Rahip', 3], ['Kurian/Porutu', 4]];
const FB_OWNER = { 0: 'Her iki ırk', 1: 'Karus', 2: 'El Morad' };

PAGE.fieldbots = async function (main) {
  let data;
  try { data = await api('/field-bots'); } catch (_) { main.append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }

  const status = h('span', { class: 'chip' });
  const paintStatus = () => {
    status.className = 'chip ' + (data.server_online ? 'ok' : 'bad');
    clear(status).append(data.server_online ? 'Oyun sunucusu çevrimiçi' : 'Oyun sunucusu kapalı — değişiklikler beklemede');
  };
  paintStatus();

  const totalBox = h('div', { class: 'hint' });
  const cards = new Map(); // zone -> {inputs, live, enabled}

  const num = (v, min, max, w) => h('input', { type: 'number', min: String(min), max: String(max), value: v, style: 'width:' + (w || 90) + 'px' });

  function readZone(z) {
    const c = cards.get(z.zone);
    return {
      enabled: c.enabled.checked,
      karus_count: Number(c.karus.value) || 0,
      elmorad_count: Number(c.elmorad.value) || 0,
      level_min: Number(c.lmin.value) || 1,
      level_max: Number(c.lmax.value) || 83,
      mix: c.mix.map(i => Number(i.value) || 0),
    };
  }

  function recount() {
    let total = 0;
    for (const z of data.zones) {
      const c = cards.get(z.zone);
      if (c.enabled.checked) total += (Number(c.karus.value) || 0) + (Number(c.elmorad.value) || 0);
    }
    totalBox.className = 'hint' + (total > data.caps.total ? ' warn' : '');
    clear(totalBox).append('Aktif bölgelerde toplam hedef: ' + total + ' / ' + data.caps.total + ' bot (üst sınır). Bölge ve ırk başına en fazla ' + data.caps.per_nation_zone + '.');
  }

  function zoneCard(z) {
    const enabled = h('input', { type: 'checkbox', checked: z.enabled, onchange: recount });
    const karus = num(z.karus_count, 0, data.caps.per_nation_zone);
    const elmorad = num(z.elmorad_count, 0, data.caps.per_nation_zone);
    if (z.owner === 2) { karus.disabled = true; karus.value = 0; }
    if (z.owner === 1) { elmorad.disabled = true; elmorad.value = 0; }
    [karus, elmorad].forEach(i => i.addEventListener('input', recount));
    const lmin = num(z.level_min, 1, 83, 80);
    const lmax = num(z.level_max, 1, 83, 80);
    const mix = FB_MIX.map(([, idx]) => num(z.mix[idx], 0, 100, 70));
    const live = h('span', { class: 'muted sm' });
    const paintLive = zz => {
      const age = zz.live_age;
      clear(live).append('Canlı: Karus ' + zz.live_karus + ' · El Morad ' + zz.live_elmorad + (age >= 0 ? ' (' + Math.round(age) + ' sn önce)' : ' (henüz bilgi yok)'));
    };
    paintLive(z);

    const saveBtn = h('button', {
      class: 'btn primary', onclick: async () => {
        saveBtn.disabled = true;
        try {
          const r = await api('/field-bots/' + z.zone, { body: readZone(z) });
          const s = r.saved;
          enabled.checked = s.enabled; karus.value = s.karus_count; elmorad.value = s.elmorad_count;
          lmin.value = s.level_min; lmax.value = s.level_max; s.mix.forEach((v, i) => { mix[i].value = v; });
          recount();
          toast(z.name + ': kaydedildi — oyun sunucusuna yeniden yükleme sıraya alındı', 'ok');
        } catch (_) { /* api() already toasted the error */ }
        saveBtn.disabled = false;
      },
    }, ico('plus'), 'Kaydet');
    const respawnBtn = h('button', {
      class: 'btn ghost', onclick: async () => {
        if (!await confirmBox('Botları yeniden dağıt', z.name + ' bölgesindeki tüm saha botları kaldırılır; bölge aktifse birkaç saniye içinde yenileri doğar.', 'Yeniden dağıt', true)) return;
        try { await api('/field-bots/' + z.zone + '/respawn', { body: {} }); toast('Komut sıraya alındı', 'ok'); } catch (_) { /* toasted */ }
      },
    }, ico('undo'), 'Yeniden dağıt');

    cards.set(z.zone, { enabled, karus, elmorad, lmin, lmax, mix, paintLive });
    return h('div', { class: 'card', style: 'margin-bottom:14px' },
      h('div', { class: 'row', style: 'margin-bottom:10px;gap:10px;flex-wrap:wrap' },
        h('h2', {}, z.name), h('span', { class: 'chip' }, 'Bölge ' + z.zone), h('span', { class: 'chip blue' }, FB_OWNER[z.owner]),
        h('span', { class: 'chip' }, 'Bölge min. Lv ' + z.min_level), h('span', { class: 'grow' }),
        h('label', { class: 'row', style: 'gap:6px' }, enabled, h('b', {}, 'Aktif'))),
      h('div', { class: 'form' },
        field('Karus bot sayısı', karus), field('El Morad bot sayısı', elmorad),
        field('Seviye (min)', lmin), field('Seviye (max)', lmax)),
      h('div', { class: 'xs muted', style: 'margin:10px 0 4px' }, 'Sınıf karışımı (ağırlık 0–100; oran olarak kullanılır)'),
      h('div', { class: 'row', style: 'flex-wrap:wrap;gap:12px' },
        FB_MIX.map(([label], i) => h('label', { class: 'row', style: 'gap:4px;font-size:13px' }, label, mix[i]))),
      h('div', { class: 'row', style: 'gap:10px;margin-top:12px;flex-wrap:wrap' }, saveBtn, respawnBtn, h('span', { class: 'grow' }), live));
  }

  main.append(
    head('Saha Botları', 'Normal haritalarda canavar avlayan, party\'ye davet edilebilen botlar — varsayılan KAPALI', status),
    h('div', { class: 'card', style: 'margin-bottom:14px' },
      h('div', { class: 'hint' }, 'Saha botları seviyesine uygun canavar gruplarında oyuncular gibi (aynı yürüme kuralları, gerçek sınıf skilleri) avlanır, yaralanınca dinlenir ve boş durmaz. Oyuncular onları normal party menüsünden davet edebilir: bot takip eder, rahip heal/buff atar, party\'nin savaştığı canavarı %30 canına kadar düşürür (son vuruş oyuncuya kalır, exp/drop/quest hakkı oyuncuda kalır).'),
      h('div', { class: 'hint', style: 'margin-top:8px' }, 'Güvenlik: bir saha botunu öldürmek hiçbir zaman exp, NP, altın veya kill kredisi vermez. Aynı ırktan oyuncular (veya ırklar arası savaşa izin vermeyen bölgelerde herkes) onlara zarar veremez. Karus bölgelerinde yalnız Karus, El Morad bölgelerinde yalnız El Morad botu doğar; Moradon her iki ırka açıktır.')),
    ...data.zones.map(zoneCard),
    totalBox,
    h('div', { class: 'hint', style: 'margin-top:10px' }, 'Bölgede oyuncu yoksa botlar dondurulur (sunucu yükü yok). Oyun içi "bot sil" benzeri komutlar bu botları da kaldırır; bölge aktifse yeniden doğarlar — kalıcı kapatmak için "Aktif" kutusunu kaldırıp kaydedin.'));
  recount();

  // Live counts refresh by themselves while the page is on screen (edits are never overwritten).
  const timer = setInterval(async () => {
    if (!main.isConnected) { clearInterval(timer); return; }
    try {
      const d = await api('/field-bots', { quiet: true });
      data.server_online = d.server_online; paintStatus();
      d.zones.forEach(z => { const c = cards.get(z.zone); if (c) c.paintLive(z); });
    } catch (_) { /* transient */ }
  }, 5000);
};
