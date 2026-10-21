'use strict';
/* Item upgrade (anvil) rates: look up one item's upgrade path, edit single rules, change many at once.
   Backend: crates/ko-admin/src/upgrades.rs (table item_upgrade_settings, rate out of 10000). */

const UPG_TYPES = {
  4: 'Unique (mavi) eşya',
  5: 'Normal / Rare eşya',
  11: 'Reborn eşya',
  12: 'Reborn Unique eşya',
};
const upgTypeName = t => (UPG_TYPES[t] ? UPG_TYPES[t] + ' (tip ' + t + ')' : 'Tip ' + t);
const upgGradeName = g => (g === 99 ? 'Her kademe (99)' : '+' + g + ' → +' + (g + 1));
const upgPct = r => (r / 100).toFixed(2).replace(/\.00$/, '') + '%';
const upgMat = (id, name) => (id > 0 ? (name || ('#' + id)) : '');

PAGE.upgrades = async (main) => {
  const body = h('div');
  const state = { tab: 'item' };
  const tabs = h('div', { class: 'seg', style: 'margin-bottom:14px' });
  const defs = [['item', '🔎 Item Sorgula'], ['table', '📋 Oran Tablosu'], ['bulk', '⚙ Toplu Ayar']];
  function drawTabs() {
    clear(tabs);
    defs.forEach(([k, label]) => tabs.append(h('button', { class: state.tab === k ? 'on' : '', onclick: () => { state.tab = k; show() } }, label)));
  }
  async function show() {
    drawTabs(); clear(body);
    if (state.tab === 'item') await upgItemTab(body);
    else if (state.tab === 'table') await upgTableTab(body);
    else await upgBulkTab(body);
  }
  main.append(
    head('Item Upgrade Oranları', 'Anvil / yükseltme başarı oranları ve altın bedelleri. Değişiklikler kaydedilince oyun sunucusuna otomatik yüklenir.'),
    h('div', { class: 'hint', style: 'margin-bottom:12px' },
      'Nasıl çalışır: oyuncu itemi anvile koyup kağıt (scroll) ve istersen Trina / Karivdis gibi malzemeleri ekler. Sunucu, itemin tipi + kademesi + koyulan malzemelerle eşleşen kuralı bulur ve oradaki başarı yüzdesini uygular. Başarısızlıkta item yanar ve altın yine alınır.'),
    tabs, body);
  await show();
};

/* ---------- shared pieces ---------- */

function upgRateEditor(rule, onSaved) {
  const pct = h('input', { type: 'number', min: '0', max: '100', step: '0.01', value: String(rule.rate / 100), style: 'width:90px' });
  const coins = h('input', { type: 'number', min: '0', step: '1', value: String(rule.coins), style: 'width:130px' });
  const save = h('button', { class: 'btn sm primary', onclick: async () => {
    const p = Number(pct.value), c = Number(coins.value);
    if (!(p >= 0 && p <= 100)) return toast('Yüzde 0 ile 100 arasında olmalı', 'err');
    if (!(c >= 0)) return toast('Altın 0 veya daha büyük olmalı', 'err');
    await api('/upgrades/rules/' + encodeURIComponent(rule.ctid), { body: { rate: Math.round(p * 100), coins: Math.round(c) } });
    toast('Kaydedildi', 'ok'); onSaved && onSaved();
  } }, 'Kaydet');
  return { pct, coins, save };
}

function upgFilterBar(f, groups, materials, onChange) {
  const typeSel = h('select', { onchange: () => { f.item_type = typeSel.value === '' ? null : Number(typeSel.value); f.item_rate = null; onChange(true) } },
    h('option', { value: '' }, 'Tüm tipler'),
    ...[...new Set(groups.map(g => g.item_type))].map(t => h('option', { value: t, selected: f.item_type === t }, upgTypeName(t))));
  const rateSel = h('select', { onchange: () => { f.item_rate = rateSel.value === '' ? null : Number(rateSel.value); onChange() } },
    h('option', { value: '' }, 'Tüm sınıflar'),
    ...groups.filter(g => f.item_type == null || g.item_type === f.item_type).map(g =>
      h('option', { value: g.item_rate, selected: f.item_rate === g.item_rate }, 'Sınıf kodu ' + g.item_rate + ' (' + g.n + ' kural)')));
  const from = h('input', { type: 'number', min: '0', max: '99', placeholder: 'en az', value: f.grade_from ?? '', style: 'width:80px', onchange: () => { f.grade_from = from.value === '' ? null : Number(from.value); onChange() } });
  const to = h('input', { type: 'number', min: '0', max: '99', placeholder: 'en çok', value: f.grade_to ?? '', style: 'width:80px', onchange: () => { f.grade_to = to.value === '' ? null : Number(to.value); onChange() } });
  const mat = h('select', { onchange: () => { f.material = mat.value === '' ? null : Number(mat.value); onChange() } },
    h('option', { value: '' }, 'Tüm malzemeler'),
    ...materials.map(m => h('option', { value: m.id, selected: f.material === m.id }, m.name + ' #' + m.id)));
  return h('div', { class: 'row', style: 'gap:10px;flex-wrap:wrap;align-items:flex-end;margin-bottom:12px' },
    field('Eşya tipi', typeSel), field('Sınıf', rateSel), field('Kademe (+) başlangıç', from), field('Kademe (+) bitiş', to), field('Malzeme / kağıt', mat));
}

function upgQuery(f) {
  const q = new URLSearchParams();
  for (const k of ['item_type', 'item_rate', 'grade_from', 'grade_to', 'material']) if (f[k] != null) q.set(k, f[k]);
  return q.toString();
}

/* ---------- tab 1: single item ---------- */

async function upgItemTab(root) {
  const result = h('div');
  const lib = libraryPanel({ onPick: it => upgShowItem(result, it.id) });
  root.append(
    h('div', { class: 'card', style: 'padding:14px;margin-bottom:14px' },
      h('p', { class: 'muted sm', style: 'margin:0 0 8px' }, 'Bir item seç (örn. +0 bir silah). Hangi +\'ya kadar basılabildiğini, hangi kağıt/Trina gerektiğini ve her basamağın yüzdesini görürsün.'),
      lib.el),
    result);
}

async function upgShowItem(root, num) {
  clear(root).append(h('div', { class: 'muted' }, 'Yükleniyor…'));
  let d;
  try { d = await api('/upgrades/item/' + num); } catch (e) { clear(root).append(h('div', { class: 'card empty' }, 'Bu item için bilgi alınamadı.')); return; }
  clear(root);
  const it = d.item;
  root.append(h('div', { class: 'card', style: 'padding:14px;display:flex;gap:12px;align-items:center;margin-bottom:12px' },
    h('div', { class: 'slot full', style: 'width:44px;height:44px;flex:none' }, h('img', { src: icon(it.icon), onerror: iconErr })),
    h('div', {}, h('b', {}, it.name || ('#' + it.num)), h('br'),
      h('small', { class: 'muted' }, '#' + it.num + ' · ' + upgTypeName(it.item_type) + ' · sınıf ' + it.item_class)),
    h('span', { class: 'grow' }),
    h('div', { style: 'text-align:right' }, h('div', { class: 'muted sm' }, 'En fazla basılabilen basamak'), h('b', { style: 'font-size:22px' }, d.max_steps ? '+' + d.max_steps + ' basamak' : '—'))));
  if (!d.steps.length) {
    root.append(h('div', { class: 'card empty' }, 'Bu itemin yükseltme tarifi (new_upgrade) yok — bu item anvilde yükseltilemiyor.'));
    return;
  }
  const byDepth = new Map();
  d.steps.forEach(s => { if (!byDepth.has(s.depth)) byDepth.set(s.depth, []); byDepth.get(s.depth).push(s); });
  const refresh = () => upgShowItem(root, num);
  for (const [depth, steps] of [...byDepth.entries()].sort((a, b) => a[0] - b[0])) {
    const rows = [];
    steps.forEach(s => {
      const rules = s.rules.length ? s.rules : [null];
      rules.forEach((r, i) => {
        const cells = [];
        if (i === 0) {
          cells.push(h('td', { rowspan: String(rules.length) }, h('b', {}, (s.from_name || ('#' + s.from))), h('br'), h('small', { class: 'muted' }, '→ ' + (s.to_name || ('#' + s.to)))));
          cells.push(h('td', { rowspan: String(rules.length) }, s.scroll ? (s.scroll_name || ('#' + s.scroll)) : '—'));
        }
        if (!r) {
          cells.push(h('td', { colspan: '4', class: 'muted bad' }, 'Bu basamak için oran kuralı yok — oyunda yapılamaz. (Oran Tablosu → Yeni kural ile ekleyebilirsin)'));
        } else {
          const ed = upgRateEditor(r, refresh);
          cells.push(h('td', {}, [upgMat(r.req1, r.name1), upgMat(r.req2, r.name2)].filter(Boolean).join(' + ') || 'Ek malzeme yok'));
          cells.push(h('td', {}, ed.pct, ' %'));
          cells.push(h('td', {}, ed.coins));
          cells.push(h('td', {}, ed.save));
        }
        rows.push(h('tr', {}, cells));
      });
    });
    root.append(h('div', { class: 'card', style: 'margin-bottom:12px' },
      h('div', { style: 'padding:10px 14px', class: 'muted' }, h('b', {}, depth + '. basamak'), ' — +' + (depth - 1) + ' → +' + depth + ' gibi düşün (itemin başlangıç kademesine göre)'),
      h('div', { class: 'tw' }, h('table', {},
        h('thead', {}, h('tr', {}, ['Item', 'Kağıt', 'Malzeme (Trina vb.)', 'Başarı', 'Altın', ''].map(x => h('th', {}, x)))),
        h('tbody', {}, rows)))));
  }
  root.append(h('p', { class: 'muted sm' }, 'Not: yüzde değeri aynı tipteki tüm itemler için geçerlidir (kural item tipine ve kademeye bağlı). Tek bir itemi değil bir kuralı değiştirirsin.'));
}

/* ---------- tab 2: rule table ---------- */

async function upgTableTab(root) {
  const f = { item_type: null, item_rate: null, grade_from: null, grade_to: null, material: null };
  const holder = h('div');
  let meta = null;
  async function load(resetRate) {
    const d = await api('/upgrades/rules?' + upgQuery(f));
    meta = meta || d;
    clear(holder);
    holder.append(upgFilterBar(f, d.groups, d.materials, load));
    const rows = d.rules.map(r => {
      const ed = upgRateEditor(r, () => load());
      return h('tr', {},
        h('td', {}, upgTypeName(r.item_type)),
        h('td', {}, String(r.item_rate)),
        h('td', {}, upgGradeName(r.grade)),
        h('td', {}, [upgMat(r.req1, r.name1), upgMat(r.req2, r.name2)].filter(Boolean).join(' + ') || '—'),
        h('td', {}, ed.pct, ' %'),
        h('td', {}, ed.coins),
        h('td', { style: 'white-space:nowrap' }, ed.save, ' ', h('button', { class: 'btn sm danger', onclick: async () => {
          if (!await confirmBox('Kuralı sil', upgTypeName(r.item_type) + ' ' + upgGradeName(r.grade) + ' kuralı silinecek; bu malzemelerle yükseltme yapılamaz hale gelir.', 'Sil', true)) return;
          await api('/upgrades/rules/' + encodeURIComponent(r.ctid), { method: 'DELETE' });
          toast('Silindi', 'ok'); load();
        } }, 'Sil')));
    });
    holder.append(
      h('div', { class: 'row', style: 'margin-bottom:8px;align-items:center' },
        h('span', { class: 'muted' }, d.rules.length + ' kural' + (d.limit_hit ? ' (ilk 1500 gösteriliyor, filtreyi daralt)' : '')),
        h('span', { class: 'grow' }),
        h('button', { class: 'btn primary', onclick: () => upgNewRuleDialog(d, () => load()) }, '+ Yeni kural')),
      d.rules.length ? h('div', { class: 'card' }, h('div', { class: 'tw' }, h('table', {},
        h('thead', {}, h('tr', {}, ['Eşya tipi', 'Sınıf', 'Basamak', 'Malzemeler', 'Başarı', 'Altın', ''].map(x => h('th', {}, x)))),
        h('tbody', {}, rows)))) : h('div', { class: 'card empty' }, 'Bu filtreyle kural yok.'));
  }
  root.append(holder);
  await load();
}

function upgNewRuleDialog(meta, onDone) {
  const type = h('select', {}, ...[...new Set(meta.groups.map(g => g.item_type))].map(t => h('option', { value: t }, upgTypeName(t))));
  const rateCls = h('input', { type: 'number', value: '0', style: 'width:100%' });
  const grade = h('input', { type: 'number', min: '0', max: '99', value: '0', style: 'width:100%' });
  const mkMat = () => h('select', { style: 'width:100%' }, h('option', { value: '0' }, '— yok —'), ...meta.materials.map(m => h('option', { value: m.id }, m.name + ' #' + m.id)));
  const m1 = mkMat(), m2 = mkMat();
  const pct = h('input', { type: 'number', min: '0', max: '100', step: '0.01', value: '50', style: 'width:100%' });
  const coins = h('input', { type: 'number', min: '0', value: '0', style: 'width:100%' });
  modal('Yeni Upgrade Kuralı', h('div', {},
    h('p', { class: 'muted sm' }, 'Basamak = yükseltilecek itemin bulunduğu kademe (+n → +n+1). 99 yazarsan her kademede geçerli olur.'),
    field('Eşya tipi', type), field('Sınıf kodu', rateCls), field('Basamak (kademe)', grade),
    field('Malzeme 1 (genelde kağıt)', m1), field('Malzeme 2 (Trina, Karivdis…)', m2),
    field('Başarı yüzdesi', pct), field('Altın bedeli', coins)), [
    { label: 'Vazgeç', cls: 'ghost' },
    { label: 'Ekle', cls: 'primary', fn: async () => {
      await api('/upgrades/rules', { body: { item_type: Number(type.value), item_rate: Number(rateCls.value) || 0, grade: Number(grade.value) || 0,
        req1: Number(m1.value) || 0, req2: Number(m2.value) || 0, rate: Math.round(Number(pct.value) * 100), coins: Math.round(Number(coins.value) || 0) } });
      toast('Kural eklendi', 'ok'); onDone();
    } }]);
}

/* ---------- tab 3: bulk ---------- */

async function upgBulkTab(root) {
  const f = { item_type: null, item_rate: null, grade_from: null, grade_to: null, material: null };
  const d0 = await api('/upgrades/rules?item_type=-1'); // only the filter lists are needed
  const holder = h('div'), out = h('div', { class: 'card', style: 'padding:14px;margin-top:12px' });
  const s = { field: 'rate', mode: 'set', value: 50 };
  function draw() {
    clear(holder);
    holder.append(upgFilterBar(f, d0.groups, d0.materials, (reset) => { if (reset) f.item_rate = null; draw(); preview(); }));
    const fieldSel = h('select', { onchange: () => { s.field = fieldSel.value; draw(); preview(); } },
      h('option', { value: 'rate', selected: s.field === 'rate' }, 'Başarı yüzdesi'), h('option', { value: 'coins', selected: s.field === 'coins' }, 'Altın bedeli'));
    const modeSel = h('select', { onchange: () => { s.mode = modeSel.value; draw(); preview(); } },
      h('option', { value: 'set', selected: s.mode === 'set' }, s.field === 'rate' ? 'Şu değere ayarla (%)' : 'Şu değere ayarla'),
      h('option', { value: 'mul', selected: s.mode === 'mul' }, 'Çarp (100 = aynı, 150 = %50 artır, 50 = yarıya indir)'),
      h('option', { value: 'add', selected: s.mode === 'add' }, s.field === 'rate' ? 'Yüzde puanı ekle / çıkar' : 'Altın ekle / çıkar'));
    const val = h('input', { type: 'number', step: 'any', value: String(s.value), style: 'width:140px', oninput: () => { s.value = Number(val.value); } , onchange: preview });
    holder.append(h('div', { class: 'row', style: 'gap:10px;flex-wrap:wrap;align-items:flex-end' }, field('Ne değişsin', fieldSel), field('Nasıl', modeSel), field('Değer', val),
      h('button', { class: 'btn', onclick: preview }, 'Önizle'),
      h('button', { class: 'btn primary', onclick: apply }, 'Uygula')));
  }
  const payload = prev => ({ ...f, field: s.field, mode: s.mode, preview: prev,
    // % inputs are percent, the server wants rate points (x100) for "set" / "add" on the rate field
    value: s.field === 'rate' && s.mode !== 'mul' ? s.value * 100 : s.value });
  async function preview() {
    if (!Number.isFinite(s.value)) return;
    const r = await api('/upgrades/bulk', { body: payload(true) });
    clear(out).append(h('div', {}, h('b', {}, r.matched + ' kural eşleşti, ' + r.changed + ' tanesi değişecek.')));
    if (r.sample && r.sample.length) {
      const fmtV = v => (s.field === 'rate' ? upgPct(v) : fmt(v));
      out.append(h('div', { class: 'muted sm', style: 'margin-top:6px' }, 'Örnek: ' + r.sample.map(x => fmtV(x.old) + ' → ' + fmtV(x.new)).join('   ·   ')));
    }
  }
  async function apply() {
    if (!Number.isFinite(s.value)) return toast('Bir değer gir', 'err');
    const r = await api('/upgrades/bulk', { body: payload(true) });
    if (!r.changed) return toast('Değişecek kural yok', 'err');
    if (!await confirmBox('Toplu değişiklik', r.changed + ' kural değiştirilecek (' + r.matched + ' eşleşti). Emin misin?', 'Uygula', true)) return;
    const done = await api('/upgrades/bulk', { body: payload(false) });
    toast(done.changed + ' kural güncellendi', 'ok');
    preview();
  }
  root.append(
    h('div', { class: 'hint', style: 'margin-bottom:12px' }, 'Önce filtrelerle hangi kuralların etkileneceğini seç (hiçbirini seçmezsen hepsi), sonra ne yapılacağını yaz. "Önizle" hiçbir şeyi değiştirmez.'),
    holder, out);
  draw(); preview();
}
