'use strict';
/* Bilgi Paneli: entries of the game client's right-top info list (title button that unfolds a
   description when clicked). Edited here, stored in info_panel_entry, pushed to online players
   by the info_panel_reload admin_command. See crates/ko-admin/src/info_panel.rs. */

PAGE.infopanel = async function (main) {
  const wrap = h('div', {});
  let limits = { title: 24, description: 255, lines: 8, shown: 12 };

  main.append(
    head('Bilgi Paneli', 'Oyun içi sağ üst listedeki başlıklar. Başlığa tıklayınca açıklama altında açılır, tekrar tıklayınca kapanır. Kaydedince çevrimiçi oyunculara anında yansır.',
      h('button', { class: 'btn primary', onclick: () => edit(null) }, '+ Yeni Giriş')),
    h('div', { class: 'card', style: 'margin-bottom:12px' },
      h('div', { class: 'hint' }, 'CLAN PREMIUM girişi otomatiktir: klan premium aktifken görünür, süre bitince kalkar (burada düzenlenmez). "Klan premium gerekir" işaretli girişler yalnızca premiumlu klan üyelerine gösterilir. Metinler ASCII\'ye çevrilir (ç→c, ş→s ...), satır sonu "|" olur.')),
    wrap);

  async function load() {
    clear(wrap).append(h('div', { class: 'empty' }, 'Yükleniyor…'));
    let d;
    try { d = await api('/info-panel', { quiet: true }); } catch (_) { clear(wrap).append(h('div', { class: 'card empty' }, 'Yüklenemedi.')); return; }
    limits = d.limits;
    render(d.entries);
  }

  async function move(entries, i, dir) {
    const j = i + dir;
    if (j < 0 || j >= entries.length) return;
    const ids = entries.map(e => e.id);
    [ids[i], ids[j]] = [ids[j], ids[i]];
    try { await api('/info-panel/reorder', { body: { ids } }); } catch (_) { return; }
    load();
  }

  function render(entries) {
    clear(wrap);
    if (!entries.length) { wrap.append(h('div', { class: 'card empty' }, 'Henüz giriş yok. "Yeni Giriş" ile ekle.')); return; }
    const active = entries.filter(e => e.enabled).length;
    if (active > limits.shown) wrap.append(h('div', { class: 'card bad' }, `Etkin giriş sayısı ${active}; oyuncuya en fazla ${limits.shown} giriş gösterilir (sıradaki ilk ${limits.shown}).`));
    entries.forEach((e, i) => {
      wrap.append(h('div', { class: 'card', style: 'display:flex;gap:10px;align-items:flex-start;padding:12px;margin-bottom:8px' },
        h('div', { style: 'flex:1;min-width:0' },
          h('b', {}, e.title),
          e.enabled ? null : h('span', { class: 'chip bad', style: 'margin-left:8px' }, 'Kapalı'),
          e.requires_clan_premium ? h('span', { class: 'chip', style: 'margin-left:8px' }, 'Klan premium gerekir') : null,
          h('div', { class: 'muted sm', style: 'white-space:pre-wrap;margin-top:4px' }, e.description.split('|').join('\n'))),
        h('button', { class: 'btn sm', disabled: i === 0, onclick: () => move(entries, i, -1) }, '↑'),
        h('button', { class: 'btn sm', disabled: i === entries.length - 1, onclick: () => move(entries, i, 1) }, '↓'),
        h('button', { class: 'btn sm', onclick: () => edit(e) }, 'Düzenle'),
        h('button', {
          class: 'btn sm danger', onclick: async () => {
            if (!await confirmBox('Sil', '"' + e.title + '" girişi silinecek.', 'Sil', true)) return;
            try { await api('/info-panel/' + e.id, { method: 'DELETE' }); } catch (_) { return; }
            toast('Silindi', 'ok'); load();
          }
        }, 'Sil')));
    });
  }

  function edit(e) {
    const title = h('input', { maxlength: String(limits.title), value: e ? e.title : '', placeholder: 'Başlık' });
    const desc = h('textarea', { rows: '6', maxlength: '600', style: 'width:100%', placeholder: 'Açıklama (her satır ayrı satır olur)' });
    desc.value = e ? e.description.split('|').join('\n') : '';
    const enabled = h('input', { type: 'checkbox', checked: e ? e.enabled : true });
    const prem = h('input', { type: 'checkbox', checked: e ? e.requires_clan_premium : false });
    const body = h('div', { class: 'form' },
      field('Başlık (en çok ' + limits.title + ' karakter)', title),
      field('Açıklama (en çok ' + limits.description + ' karakter, ' + limits.lines + ' satır)', desc, true),
      h('label', { class: 'f' }, enabled, ' Etkin'),
      h('label', { class: 'f' }, prem, ' Klan premium gerekir (yalnızca premiumlu klan üyeleri görür)'));
    modal(e ? 'Girişi düzenle' : 'Yeni giriş', body, [
      { label: 'Vazgeç', cls: 'ghost' },
      {
        label: 'Kaydet', cls: 'primary', fn: async () => {
          const payload = { title: title.value, description: desc.value, enabled: enabled.checked, requires_clan_premium: prem.checked };
          try {
            if (e) await api('/info-panel/' + e.id, { body: payload });
            else await api('/info-panel', { body: payload });
          } catch (_) { toast('Kaydedilemedi: başlık/açıklama boş olamaz, "CLAN PREMIUM" başlığı ayrılmış', 'err'); return true; }
          toast('Kaydedildi, oyunculara gönderiliyor', 'ok'); load();
        }
      }
    ]);
  }

  load();
};
