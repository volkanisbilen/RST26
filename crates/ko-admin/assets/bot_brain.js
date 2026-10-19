'use strict';
/* Bot Zekâsı: knobs of the shared bot decision layer (game server: systems/bot_brain.rs,
   design: docs/BOT_AI.md). One bot_brain_config row; a save queues a bot_brain_reload
   admin_command so the running game server picks it up without a restart. */

const BB_GROUPS = [
  ['Davranış', [
    ['patrol_percent', 'Sahada devriye gezen bot oranı (%)', 'Kalanı bowl / köprü / üs önü gibi savaş noktalarında toplanır.'],
    ['reaction_min_ms', 'Tepki gecikmesi en az (ms)', 'Yeni bir hedefe ilk vuruştan önceki insansı gecikme.'],
    ['reaction_max_ms', 'Tepki gecikmesi en çok (ms)', ''],
    ['target_mistake_percent', 'Hedef hatası (%)', 'Bu olasılıkla en iyi yerine ikinci en iyi hedefi seçer.'],
    ['kite_miss_percent', 'Kiting hatası (%)', 'Mage/priest yakına gelen düşmandan bu olasılıkla uzaklaşmayı unutur.'],
    ['retreat_hp_percent', 'Geri çekilme HP eşiği (%)', 'Bu HP altında kaçar / toparlanır, iyileşince savaşa döner.'],
    ['outnumbered_percent', 'Sayıca üstünlük eşiği (%)', '200 = düşman sayısı müttefiklerin iki katıysa kaçar.'],
  ]],
  ['Parti', [
    ['party_max_bots', 'Partide en çok bot', ''],
    ['party_max_per_class', 'Aynı sınıftan en çok', ''],
    ['spacing_min', 'Doğal aralık en az (m)', 'Üyeler tren gibi dizilmez; her birinin kendi aralığı vardır ve periyodik değişir.'],
    ['spacing_max', 'Doğal aralık en çok (m)', ''],
  ]],
  ['Sanal Envanter (bot başına, her canda yenilenir)', [
    ['hp_potions', 'HP potu', ''],
    ['mp_potions', 'MP potu', ''],
    ['ts_scrolls', 'Transformation scroll', 'Görsel dönüşüm (0 = kapalı).'],
    ['buff_scrolls', 'Buff scroll', ''],
    ['potion_cooldown_ms', 'Pot bekleme süresi (ms)', 'Oyuncularda 2400.'],
  ]],
  ['NP (yalnızca PK bölgeleri; etkinlik botları NP vermez)', [
    ['np_percent', 'Bot kill NP oranı (%)', 'Oyuncu kill NP\'sinin yüzdesi (33 = üçte biri).'],
    ['antifarm_secs', 'Anti-farm penceresi (sn)', 'Aynı oyuncu aynı botu bu süre içinde tekrar öldürürse NP almaz.'],
    ['spawn_grace_secs', 'Doğuş koruması (sn)', 'Yeni doğan bot bu süre içinde öldürülürse NP vermez.'],
  ]],
];

PAGE.botbrain = async function (main) {
  const body = h('div', {});
  main.append(
    head('Bot Zekâsı',
      'PK bölgesi, saha ve etkinlik botlarının ortak karar katmanı: tehdit değerlendirmesi, hedef seçimi, kaçma/iyileşme/toparlanma, sınıf formasyonu, sanal pot kullanımı ve bot kill NP kuralları. Değerler ortalama bir oyuncuyu hedefler.'),
    body);

  async function load() {
    clear(body).append(h('div', { class: 'empty' }, 'Yükleniyor…'));
    let data;
    try { data = await api('/bot-brain', { quiet: true }); }
    catch (_) { clear(body).append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }
    const limits = {};
    for (const l of data.limits) limits[l.name] = l;
    const cfg = data.config;
    const inputs = {};
    const enabled = h('input', { type: 'checkbox', checked: !!cfg.enabled });
    const cards = BB_GROUPS.map(([title, fields]) => h('div', { class: 'card', style: 'margin-bottom:12px' },
      h('h3', { style: 'margin:0 0 10px' }, title),
      h('div', { class: 'form' }, ...fields.map(([name, label, hint]) => {
        const lim = limits[name] || { min: 0, max: 32767, default: 0 };
        const inp = h('input', { type: 'number', min: String(lim.min), max: String(lim.max), value: cfg[name] });
        inputs[name] = inp;
        const wrap = h('div', {}, inp);
        if (hint) wrap.append(h('div', { class: 'hint', style: 'margin-top:4px' }, hint));
        return field(label + ' [' + lim.min + '–' + lim.max + ', varsayılan ' + lim.default + ']', wrap);
      }))));
    const save = h('button', { class: 'btn primary', onclick: async () => {
      const payload = { enabled: enabled.checked };
      for (const name in inputs) {
        const v = Number(inputs[name].value);
        const lim = limits[name];
        if (!Number.isFinite(v) || (lim && (v < lim.min || v > lim.max))) { toast(name + ' aralık dışında', 'err'); return; }
        payload[name] = Math.round(v);
      }
      if (payload.reaction_min_ms > payload.reaction_max_ms) { toast('Tepki gecikmesi: en az, en çoktan büyük olamaz', 'err'); return; }
      if (payload.spacing_min > payload.spacing_max) { toast('Aralık: en az, en çoktan büyük olamaz', 'err'); return; }
      try {
        await api('/bot-brain', { body: payload });
        toast('Bot zekâsı kaydedildi — oyun sunucusuna yeniden yükleme sıraya alındı', 'ok');
      } catch (_) {}
    } }, 'Kaydet');
    clear(body).append(
      h('div', { class: 'card', style: 'margin-bottom:12px' },
        h('label', { class: 'row', style: 'gap:8px;font-weight:600' }, enabled, 'Bot zekâsı etkin'),
        h('div', { class: 'hint', style: 'margin-top:6px' },
          'Kapalıyken PK botları eski "en yakına saldır" döngüsüne döner; pot, NP oranı ve parti kuralları da devre dışı kalır.')),
      ...cards,
      h('div', { class: 'hint', style: 'margin:6px 0 10px' },
        'Değişiklik kayıttan birkaç saniye sonra canlı botlara uygulanır (sanal pot stoğu bir sonraki doğuşta yenilenir).'),
      save);
  }
  load();
};
