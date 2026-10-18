'use strict';
/* Etkinlik Botları: per-event configuration of the event bot system (game server:
   systems/event_bots.rs, design: docs/EVENT_BOTS.md). Everything is OFF by default. A save writes
   event_bot_config and queues an event_bots_reload admin_command, so a running game server picks
   it up without a restart. "Botları temizle" removes every event bot standing in a room now. */

const EB_SUPPORT = {
  full: ['Tam destek', 'ok'],
  partial: ['Kısmi destek', 'blue'],
  unsupported: ['Desteklenmiyor', 'bad'],
};

PAGE.eventbots = async function (main) {
  const body = h('div', {});
  main.append(
    head('Etkinlik Botları',
      'Etkinlik odalarında oyuncu eksiğini botlarla doldur. Varsayılan KAPALI. Botlar ödül almaz, oyuncusuz oda açmaz, etkinlik bitince silinir.',
      h('button', { class: 'btn danger', onclick: async () => {
        if (!await confirmBox('Etkinlik botlarını temizle', 'Şu anda odalarda duran tüm etkinlik botları silinsin mi? (Etkinlik bozulmaz; yalnızca botlar çıkar.)', 'Temizle', true)) return;
        try { await api('/event-bots/clear', { body: {} }); toast('Temizleme oyun sunucusuna sıraya alındı', 'ok'); } catch (_) {}
      } }, ico('trash'), 'Botları temizle')),
    body);

  async function load() {
    clear(body).append(h('div', { class: 'empty' }, 'Yükleniyor…'));
    let data;
    try { data = await api('/event-bots', { quiet: true }); }
    catch (_) { clear(body).append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }
    clear(body).append(...data.events.map(renderEvent));
  }
  load();
};

function renderEvent(ev) {
  const [supText, supCls] = EB_SUPPORT[ev.support] || ['?', 'off'];
  const locked = ev.support === 'unsupported';
  const enabled = h('input', { type: 'checkbox', checked: !!ev.enabled, disabled: locked });
  const count = h('input', { type: 'number', min: '0', max: String(ev.capacity), value: ev.bots_per_team, disabled: locked });
  const minLv = h('input', { type: 'number', min: '1', max: '83', value: ev.min_level, disabled: locked });
  const maxLv = h('input', { type: 'number', min: '1', max: '83', value: ev.max_level, disabled: locked });
  const minHumans = h('input', { type: 'number', min: '1', max: '30', value: ev.min_humans, disabled: locked });
  const fill = h('select', { disabled: locked },
    h('option', { value: '1', selected: !!ev.fill_only_when_short }, 'Sadece eksikse doldur (takımı hedef sayıya tamamla)'),
    h('option', { value: '0', selected: !ev.fill_only_when_short }, 'Her zaman ekle (takım başına bu kadar bot)'));

  const save = h('button', { class: 'btn primary', disabled: locked, onclick: async () => {
    const payload = {
      enabled: enabled.checked,
      bots_per_team: Number(count.value),
      min_level: Number(minLv.value),
      max_level: Number(maxLv.value),
      fill_only_when_short: fill.value === '1',
      min_humans: Number(minHumans.value),
    };
    if (payload.min_level > payload.max_level) { toast('Minimum seviye maksimumdan büyük olamaz', 'err'); return; }
    if (!(payload.bots_per_team >= 0 && payload.bots_per_team <= ev.capacity)) { toast('Takım başına bot sayısı 0-' + ev.capacity + ' olmalı', 'err'); return; }
    try {
      await api('/event-bots/' + ev.key, { body: payload });
      toast(ev.name + ': kaydedildi — oyun sunucusuna yeniden yükleme sıraya alındı', 'ok');
    } catch (_) {}
  } }, 'Kaydet');

  return h('div', { class: 'card', style: 'margin-bottom:12px' },
    h('div', { class: 'row', style: 'justify-content:space-between;gap:12px;flex-wrap:wrap' },
      h('h3', { style: 'margin:0' }, ev.name),
      h('span', { class: 'chip ' + supCls }, supText)),
    h('div', { class: 'hint', style: 'margin:6px 0 10px' }, ev.note),
    locked ? null : h('div', {},
      h('label', { class: 'row', style: 'gap:8px;margin-bottom:10px;font-weight:600' }, enabled, 'Bu etkinlikte botlar etkin'),
      h('div', { class: 'form' },
        field('Takım başına bot (' + ev.team_label + ', en çok ' + ev.capacity + ')', count),
        field('Bot min. seviye', minLv),
        field('Bot maks. seviye', maxLv),
        field('Dolum modu', fill),
        field('Bot eklemek için en az kaç oyuncu', minHumans)),
      h('div', { class: 'hint', style: 'margin:6px 0 10px' },
        'Oyuncu olmayan odaya bot girmez. Ayarlar kayıttan sonra bir sonraki etkinlik/oda açılışında geçerli olur; devam eden etkinliğe bot eklenmez.'),
      save));
}
