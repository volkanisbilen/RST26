'use strict';
/* Veritabanı Tabloları: generic browser/editor for any game DB table (server-side paging,
   search, sort, filter; edits target the primary key). Backend: src/db_browser.rs. */

(function () {
  const css = `
.dbb-wrap{overflow:auto;max-height:calc(100vh - 300px);min-height:160px;border:1px solid var(--line2);border-radius:10px;background:#0b1530}
.dbb-wrap table{width:max-content;min-width:100%}
.dbb-wrap th{z-index:2;vertical-align:top}
.dbb-wrap th .nm{cursor:pointer;user-select:none}.dbb-wrap th .nm:hover{color:var(--text)}
.dbb-wrap th .ty{font-size:10px;color:var(--faint);text-transform:none;letter-spacing:0;font-weight:400}
.dbb-wrap th input{width:100%;min-width:70px;margin-top:5px;padding:4px 7px;font-size:12px;text-transform:none;letter-spacing:0}
.dbb-wrap td{padding:5px 10px;font-size:12.5px;cursor:default}
.dbb-wrap td.v{max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:ui-monospace,Consolas,monospace}
.dbb-wrap td.v.open{white-space:pre-wrap;word-break:break-all;max-width:640px}
.dbb-wrap td.v.num{text-align:right}
.dbb-wrap td.v.ed{cursor:cell}
.dbb-wrap td i.null{color:var(--faint);font-style:normal;font-size:11px}
.dbb-wrap td.act{white-space:nowrap;position:sticky;left:0;background:#0d1a38;z-index:1}
.dbb-wrap th.act{left:0;z-index:3}
.dbb-wrap tbody tr:hover td{background:#10224a}.dbb-wrap tbody tr:hover td.act{background:#13285a}
.dbb-note{padding:9px 12px;border:1px solid #2a4f93;background:#0c1d44;border-radius:8px;margin:8px 0;display:flex;gap:10px;align-items:center;flex-wrap:wrap}
.dbb-form{display:grid;grid-template-columns:minmax(120px,200px) 1fr;gap:8px 12px;align-items:start;max-height:62vh;overflow:auto;padding-right:6px;min-width:min(640px,86vw)}
.dbb-form label{color:var(--muted);font-size:12px;padding-top:8px;word-break:break-all}
.dbb-form label small{display:block;color:var(--faint);font-size:10px}
.dbb-form .fld{display:flex;gap:8px;align-items:flex-start}.dbb-form .fld input:not([type=checkbox]),.dbb-form .fld select,.dbb-form .fld textarea{flex:1;width:100%}
.dbb-form textarea{min-height:70px;font-family:ui-monospace,Consolas,monospace;font-size:12px}
.dbb-list .r{display:flex;gap:12px;align-items:center;padding:8px 12px;border-bottom:1px solid #12234a;cursor:pointer}
.dbb-list .r:hover{background:#10224a}.dbb-list .r b{font-family:ui-monospace,Consolas,monospace;font-weight:500}
`;
  if (!document.getElementById('dbb-css')) {
    const st = document.createElement('style'); st.id = 'dbb-css'; st.textContent = css; document.head.append(st);
  }

  const enc = encodeURIComponent;
  const CLIP = 200;

  PAGE.dbtables = async function (main, args) {
    if (args && args[0]) return dbbGrid(main, args[0]);
    return dbbList(main);
  };

  /* ───────── table list ───────── */
  async function dbbList(main) {
    const d = await api('/dbb/tables');
    const tables = d.tables;
    const box = h('div', { class: 'dbb-list card flat', style: 'padding:0' });
    const cnt = h('span', { class: 'muted sm' });
    const inp = h('input', { placeholder: 'Tablo ara… (örn. quest, item, magic)', style: 'min-width:280px', oninput: debounce(draw, 120) });
    function draw() {
      const q = inp.value.trim().toLowerCase();
      const rows = tables.filter(t => !q || t.name.toLowerCase().includes(q));
      clear(box);
      cnt.textContent = rows.length + ' / ' + tables.length + ' tablo';
      if (!rows.length) { box.append(h('div', { class: 'empty' }, 'Eşleşen tablo yok')); return; }
      for (const t of rows) {
        box.append(h('div', { class: 'r', onclick: () => { location.hash = '#/dbtables/' + enc(t.name); } },
          h('b', {}, t.name), h('span', { class: 'grow' }),
          t.reload ? h('span', { class: 'chip blue', title: 'Oyunda canlı yenilenebilir' }, 'canlı yenile') : null,
          t.pk ? null : h('span', { class: 'chip bad', title: 'PK yok: salt okunur' }, 'salt okunur'),
          h('span', { class: 'muted sm nowrap', style: 'min-width:90px;text-align:right' }, t.est >= 0 ? '≈ ' + fmt(t.est) + ' satır' : '—')));
      }
    }
    main.append(head('Veritabanı Tabloları', 'Oyun veritabanındaki (public) herhangi bir tabloyu aç, ara, sırala ve düzenle. Şifre/token sütunları gizlidir; PK’sı olmayan tablolar salt okunurdur.'),
      h('div', { class: 'row', style: 'margin-bottom:10px' }, inp, cnt), box);
    draw();
    if (!('ontouchstart' in window)) inp.focus();
  }

  /* ───────── grid ───────── */
  async function dbbGrid(main, table) {
    let meta;
    try { meta = await api('/dbb/table/' + enc(table) + '/meta'); } catch (e) { location.hash = '#/dbtables'; return; }
    const cols = meta.columns;
    const colIdx = Object.fromEntries(cols.map((c, i) => [c.name, i]));
    const st = { page: 1, size: 50, sort: '', dir: 'asc', q: '', f: {} };
    let data = { rows: [], total: 0, exact: true, pages: 1 };
    let seq = 0, dirty = false;
    const readonly = !!meta.readonly;

    const thead = h('thead'), tbody = h('tbody');
    const wrap = h('div', { class: 'dbb-wrap' }, h('table', {}, thead, tbody));
    const pagerTop = h('div', { class: 'row', style: 'margin:8px 0' });
    const noteHost = h('div');

    /* header */
    function drawHead() {
      clear(thead);
      const tr = h('tr', {}, h('th', { class: 'act' }, ''));
      for (const c of cols) {
        const arrow = st.sort === c.name ? (st.dir === 'desc' ? ' ▼' : ' ▲') : '';
        const canQ = !c.sensitive && c.kind !== 'bytea';
        const th = h('th', { title: c.type + (c.nullable ? '' : ' · NOT NULL') + (c.default ? ' · default ' + c.default : '') + (meta.pk.includes(c.name) ? ' · PK' : '') },
          h('div', { class: 'nm', onclick: () => { if (!canQ) return; if (st.sort === c.name) st.dir = st.dir === 'asc' ? 'desc' : 'asc'; else { st.sort = c.name; st.dir = 'asc'; } st.page = 1; drawHead(); load(); } },
            (meta.pk.includes(c.name) ? '🔑 ' : '') + c.name + arrow),
          h('div', { class: 'ty' }, c.sensitive ? 'gizli' : c.type),
          canQ ? h('input', { placeholder: 'filtre (=tam)', value: st.f[c.name] || '', oninput: debounce(e => { const v = e.target.value; if (v) st.f[c.name] = v; else delete st.f[c.name]; st.page = 1; load(); }, 500) }) : null);
        tr.append(th);
      }
      thead.append(tr);
    }

    /* cells */
    function cell(c, v, ri) {
      const td = h('td', { class: 'v' + (c.kind === 'int' || c.kind === 'num' ? ' num' : '') + (!readonly && c.editable ? ' ed' : '') });
      if (c.sensitive) { td.append(h('i', { class: 'null' }, '••••')); return td; }
      if (v == null) { td.append(h('i', { class: 'null' }, 'NULL')); }
      else {
        const long = v.length > CLIP;
        td.textContent = long ? v.slice(0, CLIP) + '…' : v;
        td.title = long ? 'Tıkla: tamamını göster' : '';
        td.addEventListener('click', () => {
          if (td.querySelector('input,textarea')) return;
          const open = td.classList.toggle('open');
          if (long) td.textContent = open ? v : v.slice(0, CLIP) + '…';
        });
      }
      if (!readonly && c.editable) td.addEventListener('dblclick', () => inlineEdit(td, c, ri));
      return td;
    }

    function pkOf(row) { const pk = {}; for (const k of meta.pk) pk[k] = row[colIdx[k]]; return pk; }

    function inlineEdit(td, c, ri) {
      if (td.querySelector('input,textarea')) return;
      const row = data.rows[ri], cur = row[colIdx[c.name]];
      if (cur != null && cur.length >= 4000) { toast('Çok uzun değer (4000+ karakter): satır formundan düzenlenemez/kırpılmış', 'err'); return; }
      const multi = cur != null && (cur.includes('\n') || cur.length > 80) || c.kind === 'json';
      const el = multi ? h('textarea', { rows: 4, style: 'width:360px' }) : h('input', { style: 'min-width:120px;width:' + Math.min(360, Math.max(120, (cur || '').length * 9 + 30)) + 'px' });
      el.value = cur == null ? '' : cur;
      let done = false;
      const finish = async save => {
        if (done) return; done = true;
        if (!save) { redrawRow(ri); return; }
        const nv = el.value;
        if ((cur ?? '') === nv) { redrawRow(ri); return; }
        try {
          const r = await api('/dbb/table/' + enc(table) + '/update', { body: { pk: pkOf(row), changes: { [c.name]: nv } } });
          data.rows[ri] = r.row; dirty = true; drawNote(); redrawRow(ri); toast('Kaydedildi', 'ok');
        } catch (_) { done = false; el.focus(); }
      };
      el.addEventListener('keydown', e => {
        if (e.key === 'Escape') finish(false);
        else if (e.key === 'Enter' && (!multi || e.ctrlKey)) { e.preventDefault(); finish(true); }
      });
      el.addEventListener('blur', () => setTimeout(() => finish(false), 150));
      clear(td).append(el); el.focus(); el.select && el.select();
    }

    function buildRow(row, ri) {
      const tr = h('tr', {});
      const act = h('td', { class: 'act' });
      if (!readonly) {
        act.append(h('button', { class: 'btn sm ghost', title: 'Satırı düzenle', onclick: () => rowForm('edit', row) }, 'Düzenle'),
          h('button', { class: 'btn sm ghost', title: 'Satırı sil', onclick: () => delRow(row) }, 'Sil'));
      }
      tr.append(act);
      cols.forEach((c, i) => tr.append(cell(c, row[i], ri)));
      return tr;
    }
    function redrawRow(ri) { const old = tbody.children[ri]; if (old) old.replaceWith(buildRow(data.rows[ri], ri)); }
    function drawBody() {
      clear(tbody);
      if (!data.rows.length) { tbody.append(h('tr', {}, h('td', { colspan: cols.length + 1 }, h('div', { class: 'empty' }, 'Kayıt yok')))); return; }
      const frag = document.createDocumentFragment();
      data.rows.forEach((r, i) => frag.append(buildRow(r, i)));
      tbody.append(frag);
    }

    /* pager */
    function drawPager() {
      clear(pagerTop);
      const go = p => { st.page = Math.max(1, Math.min(data.pages, p)); load(); };
      const jump = h('input', { type: 'number', min: 1, max: data.pages, value: st.page, style: 'width:80px', onkeydown: e => { if (e.key === 'Enter') go(parseInt(e.target.value, 10) || 1); } });
      const sizeSel = h('select', { onchange: e => { st.size = +e.target.value; st.page = 1; load(); } },
        [50, 100, 200, 500].map(n => h('option', { value: n, selected: n === st.size }, n + ' / sayfa')));
      pagerTop.append(
        h('button', { class: 'btn sm', disabled: st.page <= 1, onclick: () => go(1) }, '«'),
        h('button', { class: 'btn sm', disabled: st.page <= 1, onclick: () => go(st.page - 1) }, '‹ Önceki'),
        jump, h('span', { class: 'muted sm' }, '/ ' + fmt(data.pages)),
        h('button', { class: 'btn sm', disabled: st.page >= data.pages, onclick: () => go(st.page + 1) }, 'Sonraki ›'),
        h('button', { class: 'btn sm', disabled: st.page >= data.pages, onclick: () => go(data.pages) }, '»'),
        sizeSel,
        h('span', { class: 'muted sm' }, (data.exact ? '' : '≈ ') + fmt(data.total) + ' satır' + (data.exact ? '' : ' (tahmini)')));
    }

    function drawNote() {
      clear(noteHost);
      if (readonly) noteHost.append(h('div', { class: 'dbb-note' }, '🔒 ' + meta.readonly));
      if (dirty) {
        noteHost.append(h('div', { class: 'dbb-note' },
          h('span', {}, 'Veritabanı değişti.'),
          meta.reload
            ? [h('span', { class: 'muted' }, 'Oyun bu tabloyu bellekte tutuyor:'), h('button', { class: 'btn sm primary', onclick: async e => { e.currentTarget.disabled = true; try { await api('/dbb/table/' + enc(table) + '/reload', { body: {} }); toast('Yenileme komutu kuyruğa alındı (' + meta.reload + ')', 'ok'); } catch (_) { e.currentTarget.disabled = false; } } }, 'Oyun sunucusunda yenile')]
            : h('span', { class: 'muted' }, 'Bu tablo için canlı yenileme tanımlı değil; değişikliğin oyunda görünmesi için oyun sunucusu yeniden başlatılması gerekebilir.')));
      }
    }

    async function load() {
      const my = ++seq;
      const p = new URLSearchParams({ page: st.page, size: st.size });
      if (st.sort) { p.set('sort', st.sort); p.set('dir', st.dir); }
      if (st.q) p.set('q', st.q);
      if (Object.keys(st.f).length) p.set('f', JSON.stringify(st.f));
      wrap.style.opacity = '.55';
      try {
        const d = await api('/dbb/table/' + enc(table) + '/rows?' + p, { quiet: true });
        if (my !== seq) return;
        data = d; st.page = d.page;
        drawBody(); drawPager();
        wrap.scrollTop = 0;
      } catch (e) { if (my === seq) toast(e.message, 'err'); }
      if (my === seq) wrap.style.opacity = '';
    }

    /* forms */
    function field(c, init, mode) {
      const wrapEl = h('div', { class: 'fld' });
      if (!c.editable) {
        wrapEl.append(h('input', { disabled: true, value: c.sensitive ? '•••• (gizli)' : (init ?? '') }));
        return { el: wrapEl, c, get: () => ({ skip: true }) };
      }
      const truncated = init != null && init.length >= 4000;
      let ctl;
      if (c.kind === 'bool') {
        ctl = h('select', {}, [['', mode === 'edit' ? '(NULL)' : '(varsayılan/NULL)'], ['true', 'true'], ['false', 'false']].map(([v, l]) => h('option', { value: v }, l)));
        ctl.value = init == null ? '' : (init === 't' || init === 'true' ? 'true' : 'false');
      } else if (c.kind === 'json' || (init != null && (init.includes('\n') || init.length > 100)) || (c.kind === 'text' && !c.max_len)) {
        ctl = h('textarea', { rows: c.kind === 'json' || (init && init.length > 100) ? 5 : 2 });
        ctl.value = init ?? '';
      } else {
        ctl = h('input', { placeholder: c.default ? 'varsayılan: ' + c.default : (c.nullable ? 'NULL' : '') });
        ctl.value = init ?? '';
      }
      if (truncated) ctl.disabled = true;
      let touched = mode === 'edit';
      ctl.addEventListener('input', () => { touched = true; if (nul) nul.checked = false; });
      ctl.addEventListener('change', () => { touched = true; if (nul) nul.checked = false; });
      let nul = null;
      wrapEl.append(ctl);
      if (c.nullable && c.kind !== 'bool') {
        nul = h('input', { type: 'checkbox', title: 'NULL yaz' });
        nul.checked = mode === 'edit' && init == null;
        nul.addEventListener('change', () => { touched = true; if (nul.checked) ctl.value = ''; });
        wrapEl.append(h('label', { class: 'row', style: 'gap:4px;padding:6px 0;white-space:nowrap' }, nul, 'NULL'));
      }
      const get = () => {
        if (truncated) return { skip: true };
        if (!touched) return { skip: true };
        let v;
        if (nul && nul.checked) v = null;
        else if (c.kind === 'bool') v = ctl.value === '' ? null : ctl.value;
        else v = ctl.value;
        if (mode === 'edit') { const same = (init ?? null) === v || (init == null && v === '' && c.kind !== 'text'); if (same) return { skip: true }; }
        return { v };
      };
      return { el: wrapEl, c, get };
    }

    function rowForm(mode, row) {
      const fs = cols.map((c, i) => field(c, mode === 'edit' ? row[i] : null, mode));
      const form = h('div', { class: 'dbb-form' });
      cols.forEach((c, i) => {
        form.append(h('label', {}, c.name, h('small', {}, c.type + (c.max_len ? '(' + c.max_len + ')' : '') + (c.nullable ? '' : ' · zorunlu') + (meta.pk.includes(c.name) ? ' · PK' : '') + (c.generated ? ' · otomatik' : ''))), fs[i].el);
      });
      modal(mode === 'edit' ? table + ' — satırı düzenle' : table + ' — yeni satır', form, [
        { label: 'Vazgeç', cls: 'ghost' },
        {
          label: mode === 'edit' ? 'Kaydet' : 'Ekle', cls: 'primary', fn: async () => {
            const vals = {};
            for (const f of fs) { const g = f.get(); if (!g.skip) vals[f.c.name] = g.v; }
            try {
              if (mode === 'edit') {
                if (!Object.keys(vals).length) { toast('Değişiklik yok'); return; }
                await api('/dbb/table/' + enc(table) + '/update', { body: { pk: pkOf(row), changes: vals } });
              } else {
                await api('/dbb/table/' + enc(table) + '/insert', { body: { values: vals } });
              }
              dirty = true; drawNote(); toast(mode === 'edit' ? 'Kaydedildi' : 'Eklendi', 'ok'); load();
            } catch (_) { return true; }
          },
        },
      ]);
    }

    async function delRow(row) {
      const pk = pkOf(row);
      const ok = await confirmBox('Satırı sil', table + ' tablosundan şu satır silinecek: ' + Object.entries(pk).map(([k, v]) => k + '=' + v).join(', ') + '. Geri alınamaz.', 'Sil', true);
      if (!ok) return;
      try { await api('/dbb/table/' + enc(table) + '/delete', { body: { pk } }); dirty = true; drawNote(); toast('Silindi', 'ok'); load(); } catch (_) { }
    }

    /* page frame */
    const search = h('input', { placeholder: 'Tüm sütunlarda ara…', style: 'min-width:260px', oninput: debounce(e => { st.q = e.target.value.trim(); st.page = 1; load(); }, 500) });
    main.append(
      head(table, (meta.pk.length ? 'PK: ' + meta.pk.join(', ') : 'PK yok') + ' · ' + cols.length + ' sütun · hücreye tıkla = genişlet, çift tıkla = düzenle',
        h('button', { class: 'btn', onclick: () => { location.hash = '#/dbtables'; } }, '← Tablolar'),
        h('button', { class: 'btn', onclick: () => load() }, 'Yenile'),
        readonly ? null : h('button', { class: 'btn primary', onclick: () => rowForm('add') }, ico('plus'), 'Yeni satır')),
      h('div', { class: 'row', style: 'margin-bottom:6px' }, search,
        h('button', { class: 'btn sm ghost', onclick: () => { st.q = ''; st.f = {}; st.sort = ''; st.page = 1; search.value = ''; drawHead(); load(); } }, 'Filtreleri temizle'),
        meta.reload ? h('span', { class: 'chip blue' }, 'canlı yenileme: ' + meta.reload) : null),
      noteHost, pagerTop, wrap);
    drawNote(); drawHead(); load();
  }
})();
