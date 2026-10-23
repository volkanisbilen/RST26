'use strict';
/* Pazar Botları: oyun sunucusunun açtığı sistem pazarları. Bir GRUP bir yerdir (bölge, başlangıç noktası, ırk, isim ön eki);
   HAVUZ o grubun sattığı eşyalardır, her 12 eşya bir stall daha açar. Stall hiç tükenmez (her satıştan sonra yeniden dolar).
   Backend: crates/ko-admin/src/merchants.rs, oyun tarafı: systems/system_merchant.rs. Kaydedince oyun sunucusu stalleri birkaç saniyede yeniden kurar. */

const MB_NATION = { 0: 'Her iki ırk (sırayla)', 1: 'Karus', 2: 'El Morad' };

PAGE.merchantbots = async function (main) {
  let data;
  try { data = await api('/merchant-bots'); } catch (_) { main.append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }

  const status = h('span', { class: 'chip' });
  const paintStatus = () => {
    status.className = 'chip ' + (data.server_online ? 'ok' : 'bad');
    clear(status).append(data.server_online ? 'Oyun sunucusu çevrimiçi' : 'Oyun sunucusu kapalı — değişiklikler beklemede');
  };
  paintStatus();
  const holder = h('div');
  const tabs = h('div', { class: 'row', style: 'gap:8px;margin-bottom:12px' });
  let tab = 'groups';
  const stalls = n => Math.ceil(n / 12);
  const zoneName = z => (data.zones.find(x => x.zone === z) || {}).name || ('Bölge ' + z);

  async function refresh() { data = await api('/merchant-bots'); paintStatus(); draw(); }

  function drawTabs() {
    clear(tabs).append(
      h('button', { class: 'btn ' + (tab === 'groups' ? 'primary' : 'ghost'), onclick: () => { tab = 'groups'; draw(); } }, 'Pazar grupları'),
      h('button', { class: 'btn ' + (tab === 'sales' ? 'primary' : 'ghost'), onclick: () => { tab = 'sales'; draw(); } }, 'Satış kayıtları'),
      h('span', { class: 'grow' }),
      h('button', { class: 'btn ghost', onclick: async () => { try { await api('/merchant-bots/reload', { body: {} }); toast('Yeniden yükleme sıraya alındı', 'ok'); } catch (_) { /* toasted */ } } }, ico('undo'), 'Sunucuda yeniden kur'),
      h('button', { class: 'btn primary', onclick: () => groupDialog(null) }, ico('plus'), 'Yeni grup'));
  }

  function draw() {
    drawTabs();
    clear(holder);
    if (tab === 'sales') { drawSales(); return; }
    if (!data.groups.length) holder.append(h('div', { class: 'card empty' }, 'Henüz grup yok. "Yeni grup" ile başla.'));
    data.groups.forEach(g => holder.append(groupCard(g)));
  }

  /* ───── settings dialog (create + edit) ───── */
  function groupDialog(g) {
    const v = g || { title: '', zone: 21, nation: 0, anchor_x: 800, anchor_z: 600, columns: 6, spacing: 3, name_prefix: '', advert: '', max_stalls: 12, enabled: true, kind: 'custom' };
    const title = h('input', { value: v.title, style: 'width:100%' });
    const zone = h('select', {}, ...data.zones.map(z => h('option', { value: z.zone, selected: z.zone === v.zone }, z.name + ' (' + z.zone + ')')));
    const nation = h('select', {}, ...[0, 1, 2].map(n => h('option', { value: n, selected: n === v.nation }, MB_NATION[n])));
    const num = (val, step) => h('input', { type: 'number', value: val, step: step || '1', style: 'width:110px' });
    const ax = num(v.anchor_x, '0.5'), az = num(v.anchor_z, '0.5'), cols = num(v.columns), sp = num(v.spacing, '0.5'), mx = num(v.max_stalls);
    const prefix = h('input', { value: v.name_prefix, maxlength: 14, style: 'width:160px' });
    const advert = h('input', { value: v.advert, maxlength: 60, style: 'width:100%' });
    const en = h('input', { type: 'checkbox', checked: v.enabled });
    const kind = h('select', {}, h('option', { value: 'custom', selected: v.kind !== 'quest' }, 'Serbest havuz'), h('option', { value: 'quest', selected: v.kind === 'quest' }, 'Görev itemleri (bölgenin görevlerinden doldurulabilir)'));
    if (g) kind.disabled = true;
    modal(g ? 'Grup ayarları' : 'Yeni pazar grubu', h('div', { class: 'form' },
      field('Başlık', title), field('Bölge', zone), field('Irk', nation), field('Havuz türü', kind),
      field('Başlangıç X', ax), field('Başlangıç Z', az), field('Satır başına stall', cols), field('Stall aralığı (m)', sp),
      field('Stall isim ön eki (harf/rakam, en çok 14)', prefix), field('En çok stall', mx),
      field('Stall üstündeki yazı', advert), field('Aktif', h('label', { class: 'row', style: 'gap:6px' }, en, 'Pazarlar açık'))),
      [{ label: 'Vazgeç', cls: 'ghost' }, {
        label: 'Kaydet', cls: 'primary', fn: async () => {
          const body = {
            title: title.value, zone: Number(zone.value), nation: Number(nation.value), anchor_x: Number(ax.value), anchor_z: Number(az.value),
            columns: Number(cols.value), spacing: Number(sp.value), name_prefix: prefix.value, advert: advert.value, max_stalls: Number(mx.value), enabled: en.checked,
          };
          try {
            if (g) await api('/merchant-bots/' + g.id, { body });
            else await api('/merchant-bots', { body: { ...body, kind: kind.value } });
            toast('Kaydedildi — oyun sunucusu stalleri yeniden kuruyor', 'ok');
            await refresh();
          } catch (_) { return true; /* keep the dialog open, api() toasted */ }
        },
      }], 'wide');
  }

  /* ───── one group ───── */
  function groupCard(g) {
    let items = g.items.map(i => ({ ...i }));
    let dirty = false;
    const body = h('div');
    const info = h('span', { class: 'muted sm' });
    const saveBtn = h('button', { class: 'btn primary', disabled: true, onclick: save }, ico('plus'), 'Havuzu kaydet');

    function mark() { dirty = true; saveBtn.disabled = false; paintInfo(); }
    function paintInfo() {
      const on = items.filter(i => i.enabled).length;
      clear(info).append('Havuz: ' + items.length + ' eşya (' + on + ' aktif) → ' + stalls(on) + ' stall' + (stalls(on) > g.max_stalls ? ' — en çok ' + g.max_stalls + ' stall açılır, fazlası satılmaz' : '') + (dirty ? ' · kaydedilmedi' : ''));
    }
    async function save() {
      saveBtn.disabled = true;
      try {
        await api('/merchant-bots/' + g.id + '/pool', { body: { items: items.map(i => ({ item_id: i.item_id, price: Math.max(1, Math.round(Number(i.price) || 1)), stock: Math.max(1, Math.round(Number(i.stock) || 1)), is_kc: !!i.is_kc, enabled: !!i.enabled })) } });
        toast(g.title + ': havuz kaydedildi — oyun sunucusu yeniden kuruyor', 'ok');
        await refresh();
      } catch (_) { saveBtn.disabled = false; toast('Kaydedilemedi: tüm eşyalar gerçek ve takas edilebilir olmalı, fiyat 1–2.000.000.000, adet 1–9999', 'err'); }
    }

    function drawItems() {
      clear(body);
      if (!items.length) { body.append(h('div', { class: 'empty', style: 'padding:14px' }, 'Havuz boş. "Eşya ekle" ile başla.')); paintInfo(); return; }
      const rows = items.map((it, idx) => {
        const price = h('input', { type: 'number', min: '1', max: '2000000000', value: it.price, style: 'width:130px', oninput: () => { it.price = price.value; mark(); } });
        const stock = h('input', { type: 'number', min: '1', max: '9999', value: it.stock, style: 'width:70px', oninput: () => { it.stock = stock.value; mark(); } });
        const kc = h('input', { type: 'checkbox', checked: it.is_kc, onchange: () => { it.is_kc = kc.checked; mark(); } });
        const en = h('input', { type: 'checkbox', checked: it.enabled, onchange: () => { it.enabled = en.checked; mark(); drawItems(); } });
        const img = h('div', { class: 'slot full', style: 'width:34px;height:34px' }, h('img', { src: icon(it.icon), onerror: iconErr }));
        attachTip(img, () => ({ id: it.item_id }));
        const stallNo = it.enabled ? stalls(items.slice(0, idx + 1).filter(x => x.enabled).length) : '–';
        return h('tr', { style: it.enabled ? '' : 'opacity:.5' },
          h('td', {}, img),
          h('td', {}, h('div', {}, it.name || ('#' + it.item_id)), h('div', { class: 'xs muted' }, '#' + it.item_id + (it.tradable ? '' : ' · TAKAS EDİLEMEZ') + (it.known ? '' : ' · tabloda yok'))),
          h('td', {}, price), h('td', {}, stock), h('td', { style: 'text-align:center' }, kc), h('td', { style: 'text-align:center' }, en),
          h('td', { class: 'muted' }, String(stallNo)),
          h('td', {}, h('button', { class: 'btn sm ghost', title: 'Yukarı', onclick: () => { if (idx > 0) { [items[idx - 1], items[idx]] = [items[idx], items[idx - 1]]; mark(); drawItems(); } } }, '↑'),
            h('button', { class: 'btn sm ghost', title: 'Aşağı', onclick: () => { if (idx < items.length - 1) { [items[idx + 1], items[idx]] = [items[idx], items[idx + 1]]; mark(); drawItems(); } } }, '↓'),
            h('button', { class: 'btn sm danger', title: 'Kaldır', onclick: () => { items.splice(idx, 1); mark(); drawItems(); } }, ico('x'))));
      });
      body.append(h('div', { style: 'overflow:auto;max-height:520px' }, h('table', {},
        h('thead', {}, h('tr', {}, ['', 'Eşya', 'Fiyat (tane başı)', 'Stok', 'KC', 'Aktif', 'Stall', ''].map(t => h('th', {}, t)))), h('tbody', {}, rows))));
      paintInfo();
    }

    function addDialog() {
      const q = h('input', { type: 'search', placeholder: 'Eşya adı veya numarası…', style: 'width:100%' });
      const res = h('div', { style: 'max-height:380px;overflow:auto;margin-top:8px' });
      let timer;
      const run = async () => {
        const term = q.value.trim();
        if (!term) { clear(res); return; }
        const list = await api('/items?q=' + encodeURIComponent(term) + '&limit=40', { quiet: true }).catch(() => []);
        clear(res).append(...list.map(it => h('div', { class: 'row', style: 'gap:10px;padding:6px;cursor:pointer;border-bottom:1px solid var(--line,#2a3345)', onclick: () => {
          if (items.some(x => x.item_id === it.id)) { toast('Bu eşya havuzda zaten var', 'err'); return; }
          if (it.countable === 2) { toast('Bu eşya takas edilemez', 'err'); return; }
          items.push({ item_id: it.id, name: it.name, icon: it.icon || 0, price: 1000, stock: it.countable === 1 ? 500 : 1, is_kc: false, enabled: true, tradable: true, known: true });
          mark(); drawItems(); toast(it.name + ' eklendi (fiyatı değiştirmeyi unutma)', 'ok');
        } }, h('div', { class: 'slot full', style: 'width:34px;height:34px' }, h('img', { src: icon(it.icon), onerror: iconErr })), h('div', {}, h('div', {}, it.name), h('div', { class: 'xs muted' }, '#' + it.id))))
        );
        if (!list.length) res.append(h('div', { class: 'muted', style: 'padding:8px' }, 'Sonuç yok'));
      };
      q.addEventListener('input', () => { clearTimeout(timer); timer = setTimeout(run, 250); });
      modal('Havuza eşya ekle', h('div', {}, h('div', { class: 'hint' }, 'Tıklayınca eşya havuza girer (varsayılan fiyat 1000, sonra tablodan düzelt). Takas edilemeyen eşyalar kaydedilmez.'), q, res), [{ label: 'Kapat', cls: 'ghost' }], 'wide');
    }

    function multiplyDialog() {
      const m = h('input', { type: 'number', step: '0.1', value: '1', style: 'width:110px' });
      modal('Fiyatları çarp', h('div', {}, h('div', { class: 'hint' }, 'Havuzdaki tüm eşyaların fiyatı bu sayıyla çarpılır (örn. 0.5 = yarı fiyat, 2 = iki katı).'), field('Çarpan', m)),
        [{ label: 'Vazgeç', cls: 'ghost' }, { label: 'Uygula', cls: 'primary', fn: () => {
          const k = Number(m.value);
          if (!(k > 0)) { toast('Çarpan 0\'dan büyük olmalı', 'err'); return true; }
          items.forEach(i => { i.price = Math.min(2000000000, Math.max(1, Math.round(Number(i.price) * k))); });
          mark(); drawItems();
        } }]);
    }

    const head_ = h('div', { class: 'row', style: 'margin-bottom:10px;gap:10px;flex-wrap:wrap' },
      h('h2', {}, g.title), h('span', { class: 'chip' }, zoneName(g.zone)), h('span', { class: 'chip blue' }, MB_NATION[g.nation]),
      h('span', { class: 'chip ' + (g.enabled ? 'ok' : 'bad') }, g.enabled ? 'Aktif' : 'Kapalı'), h('span', { class: 'grow' }),
      h('button', { class: 'btn ghost', onclick: () => groupDialog(g) }, ico('cog'), 'Ayarlar'),
      h('button', { class: 'btn danger', onclick: async () => {
        if (!await confirmBox('Grubu sil', '"' + g.title + '" ve havuzu silinir, stalleri kalkar.', 'Sil', true)) return;
        try { await api('/merchant-bots/' + g.id, { method: 'DELETE' }); await refresh(); } catch (_) { /* toasted */ }
      } }, ico('x'), 'Sil'));
    const liveLine = h('div', { class: 'muted sm', style: 'margin-bottom:10px' },
      'Canlı: ', h('b', {}, g.live_stalls + ' stall'), ' · ', g.status || 'sunucu henüz durum yazmadı',
      g.status_age >= 0 ? ' (' + Math.round(g.status_age) + ' sn önce)' : '',
      ' · Konum: ' + g.anchor_x + ', ' + g.anchor_z + ' · Satış: ' + g.sales_24h + ' (24 sa) / ' + g.sales_total + ' toplam · Ciro: ' + fmt(g.gold_total) + ' altın' + (g.kc_total ? ' + ' + fmt(g.kc_total) + ' KC' : ''));
    const bar = h('div', { class: 'row', style: 'gap:8px;margin:10px 0;flex-wrap:wrap' },
      h('button', { class: 'btn', onclick: addDialog }, ico('plus'), 'Eşya ekle'),
      g.kind === 'quest' ? h('button', { class: 'btn ghost', onclick: async () => {
        try { const r = await api('/merchant-bots/' + g.id + '/fill-quest', { body: {} }); toast(r.added + ' görev eşyası eklendi', 'ok'); await refresh(); } catch (_) { /* toasted */ }
      } }, ico('scroll'), 'Bölgenin görev itemlerini ekle') : null,
      h('button', { class: 'btn ghost', onclick: multiplyDialog }, 'Fiyatları çarp'),
      h('span', { class: 'grow' }), info, saveBtn);
    drawItems();
    return h('div', { class: 'card', style: 'margin-bottom:16px' }, head_, liveLine, bar, body);
  }

  async function drawSales() {
    const list = await api('/merchant-bots/sales?limit=80', { quiet: true }).catch(() => []);
    if (!list.length) { holder.append(h('div', { class: 'card empty' }, 'Henüz satış yok.')); return; }
    holder.append(h('div', { class: 'card' }, h('table', {},
      h('thead', {}, h('tr', {}, ['Zaman', 'Grup', 'Stall', 'Alıcı', 'Eşya', 'Adet', 'Tutar'].map(t => h('th', {}, t)))),
      h('tbody', {}, list.map(s => h('tr', {},
        h('td', { class: 'muted' }, new Date(s.at * 1000).toLocaleString('tr-TR')), h('td', {}, s.group), h('td', {}, s.stall), h('td', {}, s.buyer),
        h('td', {}, h('div', { class: 'row', style: 'gap:6px' }, h('div', { class: 'slot full', style: 'width:26px;height:26px' }, h('img', { src: icon(s.icon), onerror: iconErr })), s.name || ('#' + s.item_id))),
        h('td', {}, String(s.count)), h('td', {}, fmt(s.price) + (s.is_kc ? ' KC' : ' altın'))))))));
  }

  main.append(
    head('Pazar Botları', 'Oyun sunucusunun açtığı satıcı stalleri: görev itemleri (Luferson, El Morad, Eslant) ve Moradon pazarı', status),
    h('div', { class: 'card', style: 'margin-bottom:14px' },
      h('div', { class: 'hint' }, 'Her grup bir yerdir; havuzdaki her 12 eşya yan yana bir stall açar (stall sayısı "en çok stall" ile sınırlı). Botlar gerçek oyuncu gibi pazar kurar, oyuncular normal pazar penceresinden alır. Stoklar her satıştan sonra hemen dolar, botun kazandığı altın silinir.'),
      h('div', { class: 'hint', style: 'margin-top:8px' }, 'Fiyat tane başınadır. Konum geçersizse (duvar vb.) sunucu en yakın geçerli yeri bulur; bulamazsa grubun durum satırında nedenini yazar. Yalnızca takas edilebilen gerçek eşyalar kaydedilir.')),
    tabs, holder);
  draw();
};
