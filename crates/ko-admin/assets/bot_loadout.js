'use strict';
/* Bot Donanımı: manual per-(class, level bracket) corrections for GM PK bots -- lets the
   project owner pin exact stats/items/skills instead of trusting the game server's procedural
   generator. Blank fields / unchecked skills stay on the automatic pick. Saves queue a
   bot_loadout_reload admin_command the same way the PK Bot Dalgası page queues "bot". */

const BL_CLASSES = [[1, 'Savaşçı'], [2, 'Rogue'], [3, 'Büyücü'], [4, 'Rahip'], [13, 'Kurian/Porutu']];
const BL_BRACKETS = [59, 69, 83];
const BL_SLOT_LABEL = {
  1: 'Kask (Head)', 4: 'Göğüs (Breast)', 5: 'Omuz (Shoulder)', 6: 'Sağ El (Right Hand)',
  8: 'Sol El (Left Hand / Kalkan)', 10: 'Bacak (Leg)', 12: 'Eldiven (Glove)', 13: 'Bot (Foot)',
};
const blNumOrNull = v => (v === '' || v == null ? null : Number(v));

PAGE.botloadout = async function (main) {
  let cls = 1, bracket = 59;
  const body = h('div', {});
  const clsSeg = h('div', { class: 'seg' }, BL_CLASSES.map(([id, label]) =>
    h('button', { class: id === cls ? 'on' : '', onclick: e => { cls = id; [...clsSeg.children].forEach(b => b.classList.toggle('on', b === e.currentTarget)); load(); } }, label)));
  const brSeg = h('div', { class: 'seg' }, BL_BRACKETS.map(b =>
    h('button', { class: b === bracket ? 'on' : '', onclick: e => { bracket = b; [...brSeg.children].forEach(x => x.classList.toggle('on', x === e.currentTarget)); load(); } }, 'Lv ' + b)));

  async function load() {
    clear(body).append(h('div', { class: 'empty' }, 'Yükleniyor…'));
    let data, catalog;
    try {
      [data, catalog] = await Promise.all([
        api('/bot-loadout/' + cls + '/' + bracket, { quiet: true }),
        api('/bot-loadout/catalog/' + cls, { quiet: true }),
      ]);
    } catch (_) { clear(body).append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }
    renderSection(body, cls, bracket, data, catalog, load);
  }

  main.append(
    head('Bot Donanımı', 'GM PK botlarının item / stat / skill seçimini elle düzelt — boş bırakılan alanlar otomatik üretime döner.'),
    h('div', { class: 'row', style: 'gap:16px;margin-bottom:12px;flex-wrap:wrap' }, clsSeg, brSeg),
    body,
  );
  load();
};

function renderSection(body, cls, bracket, data, catalog, reload) {
  clear(body);

  const statKeys = [['str', 'STR'], ['sta', 'STA'], ['dex', 'DEX'], ['int', 'INT'], ['cha', 'CHA']];
  const statInputs = {};
  statKeys.forEach(([k]) => { statInputs[k] = h('input', { type: 'number', min: '1', max: '255', placeholder: 'Otomatik', value: data.stats[k] ?? '' }); });
  const statCard = h('div', { class: 'card' },
    h('h3', {}, 'Statlar'),
    h('div', { class: 'form' }, statKeys.map(([k, label]) => field(label, statInputs[k]))),
    h('div', { class: 'hint' }, 'Boş = otomatik hesaplanan değer kullanılır (gm_bot_stats).'));

  const groups = [];
  catalog.skills.forEach(s => { let g = groups.find(x => x[0] === s.group); if (!g) groups.push(g = [s.group, []]); g[1].push(s); });
  const skillChecks = new Map();
  const skillGroups = groups.map(([g, list]) => h('div', { style: 'margin-bottom:10px' },
    h('b', { class: 'xs muted' }, g),
    h('div', { class: 'row', style: 'flex-wrap:wrap;gap:10px;margin-top:4px' },
      list.map(s => {
        const cb = h('input', { type: 'checkbox', checked: data.skills.includes(s.id) });
        skillChecks.set(s.id, cb);
        return h('label', { class: 'row', style: 'gap:4px;font-size:13px' }, cb, s.name + ' (#' + s.id + ')');
      }))));
  const skillCard = h('div', { class: 'card' },
    h('h3', {}, 'Skiller'),
    h('div', { class: 'hint', style: 'margin-bottom:8px' }, 'Hiç skill seçilmezse bu sınıfın normal otomatik skill havuzu (ve seviye kademesi) kullanılır. Bir havuzdan (örn. Heal) hiç skill seçilmezse o havuz da otomatik kalır; o havuzdan en az bir skill seçilirse yalnızca seçilenler kullanılır.'),
    skillGroups.length ? skillGroups : h('div', { class: 'muted' }, 'Bu sınıf için tanımlı skill yok.'));

  const saveStatsBtn = h('button', { class: 'btn primary', onclick: async () => {
    const skills = [...skillChecks.entries()].filter(([, cb]) => cb.checked).map(([id]) => id);
    const payload = {
      str_stat: blNumOrNull(statInputs.str.value), sta_stat: blNumOrNull(statInputs.sta.value),
      dex_stat: blNumOrNull(statInputs.dex.value), int_stat: blNumOrNull(statInputs.int.value),
      cha_stat: blNumOrNull(statInputs.cha.value), skills,
    };
    try { await api('/bot-loadout/' + cls + '/' + bracket, { body: payload }); toast('Kaydedildi — oyun sunucusuna yeniden yükleme sıraya alındı', 'ok'); reload(); } catch (_) {}
  } }, ico('plus'), 'Stat ve skilleri kaydet');

  const itemBySlot = new Map(data.items.map(it => [it.slot, it]));
  const itemRows = data.slots.map(slot => {
    const existing = itemBySlot.get(slot);
    const idInput = h('input', { type: 'number', min: '0', placeholder: 'item id (boş = otomatik)', value: existing ? existing.item_id : '' });
    const durInput = h('input', { type: 'number', min: '0', max: '32767', value: existing ? existing.durability : 100, style: 'width:100px' });
    const preview = h('div', { class: 'xs muted' }, existing && existing.item_id ? (existing.name || ('#' + existing.item_id)) : 'Otomatik seçim (pick_bot_gear)');
    const saveBtn = h('button', { class: 'btn sm', onclick: async () => {
      const itemId = Number(idInput.value) || 0;
      try { await api('/bot-loadout/' + cls + '/' + bracket + '/item', { body: { slot, item_id: itemId, durability: Number(durInput.value) || 100 } }); toast('Kaydedildi', 'ok'); reload(); } catch (_) {}
    } }, 'Kaydet');
    const clearBtn = h('button', { class: 'btn sm ghost', onclick: async () => {
      try { await api('/bot-loadout/' + cls + '/' + bracket + '/item', { body: { slot, item_id: 0, durability: 100 } }); toast('Otomatiğe döndürüldü', 'ok'); reload(); } catch (_) {}
    } }, 'Otomatiğe döndür');
    return h('div', { class: 'card', style: 'margin-bottom:8px' },
      h('div', { class: 'row', style: 'justify-content:space-between' }, h('b', {}, BL_SLOT_LABEL[slot] || ('Slot ' + slot)), preview),
      h('div', { class: 'row', style: 'gap:8px;margin-top:6px;flex-wrap:wrap;align-items:flex-end' }, field('Item ID', idInput), field('Durability', durInput), saveBtn, clearBtn));
  });

  body.append(statCard, saveStatsBtn, skillCard, h('h3', { style: 'margin-top:20px' }, 'Item Slotları (8 giyilen slot — kozmetik/branding slotları hariç)'), ...itemRows);
}
