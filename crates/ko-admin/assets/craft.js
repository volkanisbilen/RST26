'use strict';
/* Craft / Birleştirme: the recipes of the Shozin (19073) and jeweler (31402) combine window.
   Backend: crates/ko-admin/src/craft.rs (table item_special_sewing, rate out of 10000). */

const CRAFT_STATUS = { ok: 'İstemci formülüyle birebir', generic: 'İstemci formülü (genel takı)', result_differs: 'İstemcide sonuç farklı', missing: 'Sunucuda yok' };

PAGE.craft = async (main) => {
  const f = { npc: 19073, q: '', desc: '', client: '' };
  const holder = h('div');
  async function load() {
    const q = new URLSearchParams();
    q.set('npc', f.npc);
    if (f.q) q.set('q', f.q);
    if (f.desc) q.set('desc', f.desc);
    if (f.client) q.set('client', f.client);
    const d = await api('/craft/recipes?' + q.toString());
    draw(d);
  }
  function draw(d) {
    clear(holder);
    const npcSel = h('select', { onchange: () => { f.npc = Number(npcSel.value); f.desc = ''; load(); } },
      ...d.npcs.map(n => h('option', { value: n.id, selected: n.id === f.npc }, n.label)));
    const descSel = h('select', { onchange: () => { f.desc = descSel.value; load(); } }, h('option', { value: '' }, 'Tüm gruplar'),
      ...d.descs.map(x => h('option', { value: x.d, selected: f.desc === x.d }, (x.d || '(adsız)') + ' · ' + x.n)));
    const cliSel = h('select', { onchange: () => { f.client = cliSel.value; load(); } },
      h('option', { value: '' }, 'Hepsi'), h('option', { value: 'yes', selected: f.client === 'yes' }, 'Sadece istemcide formülü olanlar'),
      h('option', { value: 'no', selected: f.client === 'no' }, 'İstemcide formülü olmayanlar (sonraya bırakılanlar)'));
    const search = h('input', { type: 'search', placeholder: 'Eşya adı / numara ara…', value: f.q, style: 'width:230px', onchange: () => { f.q = search.value; load(); } });
    holder.append(
      h('div', { class: 'hint', style: 'margin-bottom:12px' },
        'Nasıl çalışır: oyuncu NPC\'ye "Manufacturing / Assemble" der, birleştirme penceresine tarifin malzemelerini aynen koyar (eşya ve adet birebir). Sonuç tablodaki başarı oranıyla verilir. ',
        h('b', {}, 'Shadow Piece'), ' (700009000) sağdaki ayrı slota konursa: "Shadow başarı" işaretli tarifler %100 başarılı olur, işaretsizlerde oran %40 artar (en çok %100). Başarısızlıkta malzemeler ve Shadow Piece gider. Craft yalnızca Moradon\'da çalışır.'),
      h('div', { class: 'row', style: 'gap:10px;flex-wrap:wrap;align-items:flex-end;margin-bottom:12px' },
        field('NPC', npcSel), field('Grup', descSel), field('İstemci formülü', cliSel), field('Ara', search), h('span', { class: 'grow' }),
        h('button', { class: 'btn primary', onclick: () => craftNewDialog(f.npc, load) }, '+ Yeni tarif')),
      h('div', { class: 'muted sm', style: 'margin-bottom:8px' }, d.total_groups + ' tarif grubu' + (d.total_groups > d.groups.length ? ' (ilk ' + d.groups.length + ' gösteriliyor, filtreyi daralt)' : '')));
    if (!d.groups.length) { holder.append(h('div', { class: 'card empty' }, 'Bu filtreyle tarif yok.')); return; }
    d.groups.forEach(g => holder.append(craftGroup(g, d.shadow_item)));
  }
  main.append(head('Craft / Birleştirme', 'Shozin ve Kuyumcu\'nun birleştirme tarifleri, oranlar ve Shadow Piece etkisi.'), holder);
  await load();

  function craftGroup(g, shadowItem) {
    const mats = h('div', { style: 'display:flex;gap:6px;flex-wrap:wrap;align-items:center' }, g.materials.map(m =>
      h('div', { style: 'text-align:center;width:62px', title: (m.name || '#' + m.id) + ' ×' + m.count + ' (#' + m.id + ')' },
        h('div', { class: 'slot full', style: 'width:40px;height:40px;margin:0 auto;position:relative' }, h('img', { src: icon(m.icon), onerror: iconErr }),
          h('span', { style: 'position:absolute;right:1px;bottom:0;font-size:11px;font-weight:700;text-shadow:0 0 3px #000' }, String(m.count))),
        h('div', { class: 'xs muted', style: 'line-height:1.1;margin-top:2px;overflow:hidden;max-height:24px' }, m.name || ('#' + m.id)))));
    const badge = g.client
      ? h('span', { class: 'chip ' + (g.client.status === 'result_differs' ? 'bad' : 'ok'), title: 'mixtable satırı ' + g.client.mix }, 'İstemci: ' + (g.client.category || 'formül kitabında') + (g.client.status === 'generic' ? ' (genel takı)' : ''))
      : h('span', { class: 'chip', title: 'İstemcinin formül kitabında yok; malzemeler elle konunca yine craft olur' }, 'İstemci formülü yok');
    const allN = g.variants.map(v => v.n);
    const bulkRate = h('input', { type: 'number', min: '0', max: '100', step: '0.01', placeholder: '% oran', style: 'width:90px' });
    const bulkShadow = h('select', {}, h('option', { value: '' }, 'Shadow: değiştirme'), h('option', { value: '1' }, 'Shadow başarı: açık'), h('option', { value: '0' }, 'Shadow başarı: kapalı'));
    const variants = g.variants.map(v => craftVariant(v));
    return h('div', { class: 'card', style: 'padding:12px;margin-bottom:12px' },
      h('div', { class: 'row', style: 'align-items:center;gap:10px;margin-bottom:8px' }, h('b', {}, g.desc || '(adsız)'), badge, h('span', { class: 'grow' }),
        g.variants.length > 1 ? h('span', { class: 'row', style: 'gap:6px' }, bulkRate, bulkShadow, h('button', { class: 'btn sm', onclick: async () => {
          const body = { n: allN };
          if (bulkRate.value !== '') body.rate = Math.round(Number(bulkRate.value) * 100);
          if (bulkShadow.value !== '') body.shadow = bulkShadow.value === '1';
          if (body.rate === undefined && body.shadow === undefined) return toast('Oran veya Shadow seç', 'err');
          await api('/craft/bulk', { body }); toast(allN.length + ' sonuç güncellendi', 'ok'); load();
        } }, 'Gruba uygula')) : null),
      h('div', { class: 'muted sm', style: 'margin-bottom:4px' }, 'Gerekli malzemeler (hepsi birden):'), mats,
      h('div', { class: 'muted sm', style: 'margin:10px 0 4px' }, g.variants.length > 1 ? 'Olası sonuçlar (oyuncunun sınıfına uyan verilir, hepsi uyuyorsa rastgele biri):' : 'Sonuç:'),
      variants);
  }

  function craftVariant(v) {
    const pct = h('input', { type: 'number', min: '0', max: '100', step: '0.01', value: String(v.rate / 100), style: 'width:80px' });
    const sh = h('input', { type: 'checkbox', checked: v.shadow });
    const eff = h('span', { class: 'sm' });
    const refresh = () => {
      const r = Math.max(0, Math.min(100, Number(pct.value) || 0));
      const withShadow = sh.checked ? 100 : Math.min(100, r * 1.4);
      eff.textContent = 'Shadow Piece ile: %' + withShadow.toFixed(2).replace(/\.00$/, '');
    };
    pct.oninput = refresh; sh.onchange = refresh; refresh();
    return h('div', { class: 'row', style: 'gap:10px;align-items:center;padding:6px 0;border-top:1px solid rgba(255,255,255,.06);flex-wrap:wrap' },
      h('div', { class: 'slot full', style: 'width:36px;height:36px;flex:none' }, h('img', { src: icon(v.icon), onerror: iconErr })),
      h('div', { style: 'min-width:220px;flex:1' }, h('b', {}, v.name || ('#' + v.give)), v.count > 1 ? ' ×' + v.count : '', h('br'), h('small', { class: 'muted' }, '#' + v.give + ' · tarif ' + v.n)),
      h('label', { class: 'row', style: 'gap:4px;align-items:center' }, 'Oran %', pct),
      h('label', { class: 'row', style: 'gap:4px;align-items:center', title: 'İşaretliyse Shadow Piece ile craft kesin başarılı olur' }, sh, 'Shadow başarı'),
      eff,
      h('button', { class: 'btn sm primary', onclick: async () => {
        const r = Number(pct.value);
        if (!(r >= 0 && r <= 100)) return toast('Oran 0-100 arası olmalı', 'err');
        await api('/craft/recipes/' + v.n, { body: { rate: Math.round(r * 100), shadow: sh.checked } });
        toast('Kaydedildi — oyuna yüklendi', 'ok');
      } }, 'Kaydet'),
      h('button', { class: 'btn sm danger', onclick: async () => {
        if (!await confirmBox('Tarifi sil', (v.name || ('#' + v.give)) + ' sonucu veren bu tarif silinecek.', 'Sil', true)) return;
        await api('/craft/recipes/' + v.n, { method: 'DELETE' }); toast('Silindi', 'ok'); load();
      } }, 'Sil'));
  }
};

function craftNewDialog(npc, onDone) {
  const desc = h('input', { placeholder: 'Grup / açıklama (örn. Hepa Silah)', style: 'width:100%' });
  const rows = Array.from({ length: 6 }, () => ({ id: h('input', { type: 'number', placeholder: 'eşya no', style: 'width:130px' }), cnt: h('input', { type: 'number', value: '1', min: '1', style: 'width:70px' }) }));
  const give = h('input', { type: 'number', placeholder: 'sonuç eşya no', style: 'width:150px' });
  const giveCnt = h('input', { type: 'number', value: '1', min: '1', style: 'width:70px' });
  const pct = h('input', { type: 'number', min: '0', max: '100', step: '0.01', value: '30', style: 'width:80px' });
  const sh = h('input', { type: 'checkbox', checked: true });
  modal('Yeni Craft Tarifi — ' + (npc === 31402 ? 'Kuyumcu' : 'Shozin'), h('div', {},
    field('Grup / açıklama', desc),
    h('p', { class: 'muted sm' }, 'Malzemeler (boş bırakılanlar yok sayılır, en çok 10):'),
    ...rows.map(r => h('div', { class: 'row', style: 'gap:8px;margin-bottom:4px' }, r.id, r.cnt)),
    h('div', { class: 'row', style: 'gap:8px;margin-top:8px;align-items:center' }, 'Sonuç:', give, '×', giveCnt),
    h('div', { class: 'row', style: 'gap:12px;margin-top:8px;align-items:center' }, 'Oran %', pct, h('label', { class: 'row', style: 'gap:4px' }, sh, 'Shadow başarı'))), [
    { label: 'Vazgeç', cls: 'ghost' },
    { label: 'Ekle', cls: 'primary', fn: async () => {
      const materials = rows.filter(r => Number(r.id.value) > 0).map(r => ({ id: Number(r.id.value), count: Number(r.cnt.value) || 1 }));
      if (!materials.length) { toast('En az bir malzeme gir', 'err'); return true; }
      if (!(Number(give.value) > 0)) { toast('Sonuç eşyasını gir', 'err'); return true; }
      await api('/craft/recipes', { body: { npc, description: desc.value.trim(), materials, give_item: Number(give.value), give_count: Number(giveCnt.value) || 1, rate: Math.round(Number(pct.value) * 100), shadow: sh.checked } });
      toast('Tarif eklendi', 'ok'); onDone();
    } }]);
}
