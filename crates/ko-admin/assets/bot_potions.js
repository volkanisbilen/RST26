'use strict';
/* Bot Potları: botların (PK, saha, etkinlik) içtiği HP / MP potları. Botlar tabloda "potion skill" olarak duran sanal potlar içer:
   varsayılan olarak barın %60'ını geçmeyen en büyük pot seçilir. Burada hangi potların kullanılacağını kapatıp açabilir, %60 sınırını
   değiştirebilir ya da her tür için tek bir potu sabitleyebilirsin. Backend: crates/ko-admin/src/bot_potions.rs. */

PAGE.botpotions = async function (main) {
  let data;
  try { data = await api('/bot-potions'); } catch (_) { main.append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }
  const status = h('span', { class: 'chip ' + (data.server_online ? 'ok' : 'bad') }, data.server_online ? 'Oyun sunucusu çevrimiçi' : 'Oyun sunucusu kapalı — kayıt beklemede');
  const state = { hp: { cap: data.hp.cap_percent, fixed: data.hp.fixed_skill }, mp: { cap: data.mp.cap_percent, fixed: data.mp.fixed_skill } };
  const enabled = new Map(data.potions.map(p => [p.skill_id, p.enabled]));
  const holder = h('div');
  const KIND = { hp: 'HP potları', mp: 'MP potları' };
  // the choice the server makes for a bar of this size (same rule as bot_brain.rs best_potion)
  function choose(kind, bar) {
    const cap = Math.max(1, Math.floor(bar * state[kind].cap / 100));
    const list = data.potions.filter(p => p.kind === kind);
    if (state[kind].fixed) { const f = list.find(p => p.skill_id === state[kind].fixed); if (f) return f; }
    let fit = null, small = null;
    for (const p of list.filter(p => enabled.get(p.skill_id))) {
      if (p.amount <= cap && (!fit || p.amount > fit.amount)) fit = p;
      if (!small || p.amount < small.amount) small = p;
    }
    return fit || small;
  }
  const itemName = p => p.items.length ? p.items.map(i => i.name + ' #' + i.id).join(', ') : '(bu büyüyü taşıyan eşya yok)';
  const label = p => p ? p.amount + ' · ' + (p.items[0] ? p.items[0].name : 'büyü ' + p.skill_id) : '—';

  function draw() {
    clear(holder);
    for (const kind of ['hp', 'mp']) {
      const cap = h('input', { type: 'number', min: '5', max: '100', value: state[kind].cap, style: 'width:90px', oninput: () => { state[kind].cap = Number(cap.value) || 60; preview(); } });
      const fixed = h('select', { onchange: () => { state[kind].fixed = fixed.value ? Number(fixed.value) : null; preview(); } },
        h('option', { value: '' }, 'Otomatik (barın %' + state[kind].cap + '\'ine sığan en büyük)'),
        ...data.potions.filter(p => p.kind === kind).map(p => h('option', { value: p.skill_id, selected: state[kind].fixed === p.skill_id }, label(p))));
      const prev = h('div', { class: 'muted sm', style: 'margin:8px 0' });
      function preview() {
        const bars = kind === 'hp' ? [2000, 4000, 6000, 8000, 10000] : [1000, 2000, 3000, 4000, 6000];
        clear(prev).append((kind === 'hp' ? 'HP' : 'MP') + ' barına göre içilecek pot: ' + bars.map(b => b + ' → ' + label(choose(kind, b))).join(' · '));
      }
      const rows = data.potions.filter(p => p.kind === kind).map(p => {
        const cb = h('input', { type: 'checkbox', checked: enabled.get(p.skill_id), onchange: () => { enabled.set(p.skill_id, cb.checked); preview(); } });
        const img = p.items[0] ? h('div', { class: 'slot full', style: 'width:30px;height:30px' }, h('img', { src: icon(p.items[0].icon), onerror: iconErr })) : h('span');
        return h('tr', {}, h('td', {}, img), h('td', { style: 'text-align:right;font-weight:700' }, String(p.amount)), h('td', {}, itemName(p)),
          h('td', { class: 'muted' }, String(p.skill_id)), h('td', { style: 'text-align:center' }, cb));
      });
      holder.append(h('div', { class: 'card', style: 'margin-bottom:16px' },
        h('h2', { style: 'margin-bottom:8px' }, KIND[kind]),
        h('div', { class: 'form' }, field('Tek pot en çok barın yüzde kaçını doldursun', cap), field('Sabit pot (seçersen her zaman bu içilir)', fixed)),
        prev,
        h('div', { style: 'overflow:auto;max-height:420px' }, h('table', {}, h('thead', {}, h('tr', {}, ['', 'İyileştirme', 'Taşıyan eşyalar', 'Büyü no', 'Kullan'].map(t => h('th', {}, t)))), h('tbody', {}, rows)))));
      preview();
    }
  }

  const saveBtn = h('button', { class: 'btn primary', onclick: async () => {
    saveBtn.disabled = true;
    try {
      await api('/bot-potions', { body: {
        hp: { cap_percent: Math.round(state.hp.cap), fixed_skill: state.hp.fixed || null },
        mp: { cap_percent: Math.round(state.mp.cap), fixed_skill: state.mp.fixed || null },
        disabled: [...enabled.entries()].filter(([, v]) => !v).map(([k]) => k),
      } });
      toast('Kaydedildi — oyun sunucusu birkaç saniyede uygular', 'ok');
    } catch (_) { /* toasted */ }
    saveBtn.disabled = false;
  } }, ico('plus'), 'Kaydet');

  main.append(
    head('Bot Potları', 'Botların hangi HP / MP potlarını içtiği', status, saveBtn),
    h('div', { class: 'card', style: 'margin-bottom:14px' },
      h('div', { class: 'hint' }, 'Botlar gerçek envanter kullanmaz: her ömürlerinde sınırlı sayıda (Bot Zekâsı sayfasında: HP ' + data.stock.hp + ', MP ' + data.stock.mp + ' pot, ' + (data.stock.cooldown_ms / 1000).toFixed(1) + ' sn bekleme) sanal pot içerler. Hangi potun içileceği buradaki listeden seçilir; potun etkisi ve ışık efekti oyundaki gerçek pot gibidir.'),
      h('div', { class: 'hint', style: 'margin-top:8px' }, 'Varsayılan davranış: barın %60\'ını geçmeyen en büyük pot. Etkinlik / "Coloseum" gibi özel potları kapatabilir ya da tek bir potu sabitleyebilirsin. Hiç pot seçilemezse bot 360 HP / 300 MP\'lik varsayılan potu içer.')),
    holder);
  draw();
};
