'use strict';
/* "Bot Envanteri": worn items and HP of every live bot (table bot_snapshot, refreshed by the game
   server on request). Backend: crates/ko-admin/src/bot_inventory.rs. */

const BOTINV_FAM = { warrior: 'Savaşçı', rogue: 'Rogue', mage: 'Büyücü', priest: 'Rahip', kurian: 'Kurian' };
const BOTINV_KIND = { pk: 'PK botu', field: 'Saha botu', event: 'Etkinlik botu' };
const BOTINV_SLOTS = ['Göğüs', 'Bacak', 'Kask', 'Eldiven', 'Bot', 'Omuz', 'Sağ el', 'Sol el'];
const botinvFamily = c => { const m = c % 100; return [1, 5, 6].includes(m) ? 'warrior' : [2, 7, 8].includes(m) ? 'rogue' : [3, 9, 10].includes(m) ? 'mage' : [4, 11, 12].includes(m) ? 'priest' : 'kurian'; };
const botinvK = n => (n >= 1000 ? (n / 1000).toFixed(1).replace(/\.0$/, '') + 'k' : String(n));

PAGE.botinv = async (main) => {
  const f = { kind: '', class_family: '', q: '' };
  const holder = h('div');
  async function load() {
    const q = new URLSearchParams();
    for (const k of Object.keys(f)) if (f[k]) q.set(k, f[k]);
    const d = await api('/bot-inventory?' + q.toString());
    clear(holder);
    holder.append(h('div', { class: 'hint', style: 'margin-bottom:12px' },
      'Sabit HP hedefleri: Büyücü 5000 · Rogue 6500 · Rahip 7500 · Savaşçı ve Kurian 8500. "Formül HP" seviye + stat + eşyaların tek başına vereceği değerdir (artık uygulanmıyor, sadece karşılaştırma için). Bu liste anlık görüntüdür: "Yenile" ile güncelle.'));
    if (d.summary.length) {
      holder.append(h('div', { class: 'card', style: 'margin-bottom:14px' }, h('div', { class: 'tw' }, h('table', {},
        h('thead', {}, h('tr', {}, ['Sınıf', 'Bot sayısı', 'Gerçek HP (min–maks)', 'Ortalama HP', 'Formül HP (min–maks)', 'Ort. eşya AC'].map(x => h('th', {}, x)))),
        h('tbody', {}, d.summary.map(s => h('tr', {},
          h('td', {}, BOTINV_FAM[s.family] || s.family), h('td', {}, String(s.n)),
          h('td', {}, botinvK(s.hp_min) + ' – ' + botinvK(s.hp_max)), h('td', {}, botinvK(s.hp_avg)),
          h('td', { class: 'muted' }, botinvK(s.formula_min) + ' – ' + botinvK(s.formula_max)), h('td', {}, String(s.ac_avg)))))))));
    }
    const kindSel = h('select', { onchange: () => { f.kind = kindSel.value; load(); } }, h('option', { value: '' }, 'Tüm botlar'),
      ...Object.entries(BOTINV_KIND).map(([k, v]) => h('option', { value: k, selected: f.kind === k }, v)));
    const famSel = h('select', { onchange: () => { f.class_family = famSel.value; load(); } }, h('option', { value: '' }, 'Tüm sınıflar'),
      ...Object.entries(BOTINV_FAM).map(([k, v]) => h('option', { value: k, selected: f.class_family === k }, v)));
    const search = h('input', { type: 'search', placeholder: 'Bot adı ara…', value: f.q, style: 'width:200px', onchange: () => { f.q = search.value; load(); } });
    holder.append(h('div', { class: 'row', style: 'gap:10px;flex-wrap:wrap;align-items:flex-end;margin-bottom:10px' },
      field('Tür', kindSel), field('Sınıf', famSel), field('Ara', search), h('span', { class: 'grow' }),
      h('span', { class: 'muted sm' }, (d.taken ? 'Son görüntü: ' + d.taken + ' (UTC) · ' : 'Henüz görüntü yok · ') + d.bots.length + ' bot gösteriliyor')));
    if (!d.bots.length) { holder.append(h('div', { class: 'card empty' }, 'Gösterilecek bot yok. Sunucuda bot varsa "Yenile" ile görüntü al.')); return; }
    holder.append(h('div', { class: 'card' }, h('div', { class: 'tw' }, h('table', {},
      h('thead', {}, h('tr', {}, ['Bot', 'Tür', 'Sınıf', 'Lv', 'HP', 'Formül HP', 'Eşya HP', 'AC', 'Eşyalar'].map(x => h('th', {}, x)))),
      h('tbody', {}, d.bots.map(b => {
        const items = h('div', { style: 'display:flex;gap:4px;flex-wrap:wrap' }, (b.items || []).map(it => {
          const info = d.names[String(it.item)] || {};
          return h('div', { class: 'slot full', style: 'width:30px;height:30px', title: (BOTINV_SLOTS[it.slot] || '') + ': ' + (info.name || ('#' + it.item)) + ' (#' + it.item + ')' },
            h('img', { src: icon(info.icon || 0), onerror: iconErr }));
        }));
        const cheap = b.max_hp !== b.formula_hp;
        return h('tr', {},
          h('td', {}, h('b', {}, b.name), h('br'), h('small', { class: 'muted' }, (b.nation === 1 ? 'Karus' : 'El Morad') + ' · bölge ' + b.zone)),
          h('td', {}, BOTINV_KIND[b.kind] || b.kind),
          h('td', {}, BOTINV_FAM[botinvFamily(b.class)] + ' (' + b.class + ')'),
          h('td', {}, String(b.level)),
          h('td', {}, h('b', {}, botinvK(b.max_hp)), ' ', h('small', { class: 'muted' }, String(b.max_hp))),
          h('td', { class: cheap ? 'muted' : '' }, botinvK(b.formula_hp)),
          h('td', {}, '+' + b.item_hp), h('td', {}, String(b.ac)), h('td', {}, items));
      }))))));
  }
  main.append(
    head('Bot Envanteri', 'Botların giydiği eşyalar ve HP değerleri.',
      h('button', { class: 'btn primary', onclick: async () => {
        await api('/bot-inventory/refresh', { body: {} });
        toast('Görüntü isteniyor…', 'ok');
        setTimeout(load, 2500);
      } }, '⟳ Yenile')),
    holder);
  await load();
};
