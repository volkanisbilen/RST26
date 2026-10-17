'use strict';
/* Chaotic Generator box management: which items are breakable (item.effect2=251) and
   each one's reward pool (item_exchange). See crates/ko-admin/src/chaotic.rs. */

PAGE.chaotic = async (main, params) => {
  if (params && params[0]) return chaoticBoxDetail(main, Number(params[0]));
  const grid = h('div', { class: 'grid' });
  main.append(
    head('Chaotic Generator', 'Oyun içi Chaotic Generator NPC\'sinde kırılabilen kutular ve ödülleri.',
      h('button', { class: 'btn primary', onclick: () => chaoticEnableDialog(render) }, '+ Yeni Kutu Ekle')),
    grid
  );
  async function render() {
    clear(grid);
    const d = await api('/chaotic/boxes');
    if (!d.boxes.length) { grid.append(h('div', { class: 'card empty' }, 'Henüz aktif kutu yok. "Yeni Kutu Ekle" ile başla.')); return }
    d.boxes.forEach(b => {
      grid.append(h('div', { class: 'card', style: 'display:flex;gap:10px;align-items:center;padding:12px' },
        h('div', { class: 'slot full', style: 'width:40px;height:40px;flex:none' }, h('img', { src: icon(b.icon), onerror: iconErr })),
        h('div', { style: 'flex:1;min-width:0' }, h('b', {}, b.name || ('#' + b.item_id)), h('br'),
          h('small', { class: b.reward_count ? 'muted' : 'muted bad' }, b.reward_count ? (b.reward_count + ' ödül satırı') : 'Ödülü yok — kırılınca hata verir')),
        h('button', { class: 'btn sm', onclick: () => { location.hash = '#/chaotic/' + b.item_id } }, 'Yönet'),
        h('button', {
          class: 'btn sm danger', onclick: async () => {
            if (!await confirmBox('Devre dışı bırak', (b.name || ('#' + b.item_id)) + ' artık Chaotic Generator\'da kırılamayacak (ödül listesi silinmiyor, sonra tekrar açabilirsin).', 'Devre dışı bırak', true)) return;
            await api('/chaotic/boxes/' + b.item_id + '/disable', { body: {} });
            toast('Devre dışı bırakıldı', 'ok'); render();
          }
        }, 'Kapat')));
    });
  }
  await render();
};

function chaoticEnableDialog(onDone) {
  const ctx = { picked: null };
  const picker = pickerEl(ctx);
  modal('Yeni Kutu Ekle', h('div', {}, h('p', { class: 'muted sm' }, 'Kutuyu/parçayı seç — stacklenebilir (countable) bir item olmalı.'), picker), [
    { label: 'Vazgeç', cls: 'ghost' },
    {
      label: 'Ekle', cls: 'primary', fn: async () => {
        if (!ctx.picked) { toast('Bir item seç', 'err'); return true }
        try {
          await api('/chaotic/boxes', { body: { item_id: ctx.picked.id } });
        } catch (e) {
          if (e.status === 400) toast('Bu item stacklenebilir (countable) değil, Chaotic Generator ile kullanılamaz', 'err');
          return true;
        }
        toast('Kutu eklendi', 'ok');
        onDone();
      }
    }
  ]);
}

async function chaoticBoxDetail(main, boxId) {
  const list = await api('/chaotic/boxes');
  const box = list.boxes.find(b => b.item_id === boxId);
  const wrap = h('div');
  main.append(
    head(box ? box.name : ('Kutu #' + boxId), 'Chaotic Generator ödül havuzu — ağırlık ne kadar yüksekse o kadar sık çıkar.',
      h('a', { class: 'btn ghost', href: '#/chaotic' }, '← Geri'),
      h('button', { class: 'btn primary', onclick: () => chaoticAddRewardDialog(boxId, render) }, '+ Ödül Ekle')),
    wrap
  );
  async function render() {
    clear(wrap);
    const d = await api('/chaotic/boxes/' + boxId + '/rewards');
    if (!d.rewards.length) { wrap.append(h('div', { class: 'card empty' }, 'Bu kutunun hiç ödülü yok — oyuncular kırınca hata alır.')); return }
    wrap.append(h('div', { class: 'card' }, h('div', { class: 'tw' }, h('table', {},
      h('thead', {}, h('tr', {}, ['Item', 'Ağırlık', 'Yaklaşık Şans', ''].map(x => h('th', {}, x)))),
      h('tbody', {}, d.rewards.map(r => h('tr', {},
        h('td', {}, h('div', { class: 'row', style: 'gap:8px;align-items:center' },
          h('div', { class: 'slot full', style: 'width:28px;height:28px' }, h('img', { src: icon(r.icon), onerror: iconErr })),
          h('span', {}, r.name || ('#' + r.item_id)))),
        h('td', {}, r.weight),
        h('td', {}, r.pct.toFixed(2) + '%'),
        h('td', {}, h('button', {
          class: 'btn sm danger', onclick: async () => {
            if (!await confirmBox('Ödülü kaldır', (r.name || ('#' + r.item_id)) + ' bu kutudan kaldırılacak.', 'Kaldır', true)) return;
            await api('/chaotic/boxes/' + boxId + '/rewards/' + r.n_index, { method: 'DELETE' });
            toast('Kaldırıldı', 'ok'); render();
          }
        }, 'Kaldır'))
      ))))));
  }
  await render();
}

function chaoticAddRewardDialog(boxId, onDone) {
  const ctx = { picked: null };
  const picker = pickerEl(ctx);
  const weight = h('input', { type: 'number', min: '1', value: '30', style: 'width:100%' });
  modal('Ödül Ekle', h('div', {}, picker, field('Ağırlık (yüksek = daha sık çıkar, mevcut satırlarla kıyasla ayarla)', weight)), [
    { label: 'Vazgeç', cls: 'ghost' },
    {
      label: 'Ekle', cls: 'primary', fn: async () => {
        if (!ctx.picked) { toast('Bir item seç', 'err'); return true }
        const w = Number(weight.value) || 0;
        if (w <= 0) { toast('Geçerli bir ağırlık gir', 'err'); return true }
        await api('/chaotic/boxes/' + boxId + '/rewards', { body: { item_id: ctx.picked.id, weight: w } });
        toast('Eklendi', 'ok');
        onDone();
      }
    }
  ]);
}
