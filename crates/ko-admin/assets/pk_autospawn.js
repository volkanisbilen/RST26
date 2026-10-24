'use strict';
/* "PK Otomatik Bot": PK zones that get bots on their own when the game server starts.
   Backend: crates/ko-admin/src/pk_autospawn.rs (table pk_bot_autospawn). */

PAGE.pkauto = async (main) => {
  const holder = h('div');
  async function load() {
    const d = await api('/pk-autospawn');
    clear(holder);
    holder.append(h('div', { class: 'hint', style: 'margin-bottom:12px' },
      'Sunucu her açıldığında (yaklaşık 25 saniye sonra) açık olan bölgelere, her millet için istediğin sayıda PK botu otomatik doğar (Warrior, Rogue, Mage, Priest — Kurian/Porutu botları otomatik gelmez, yalnızca "PK Bot Dalgası" sayfasından eklersen gelir). Bölgede zaten bot varsa sadece eksik kalan kadarı eklenir. Ayarı kaydet; "Şimdi uygula" ile sunucuyu yeniden başlatmadan da çalıştırabilirsin.'));
    d.zones.forEach(z => {
      const en = h('input', { type: 'checkbox', checked: z.enabled });
      const cnt = h('input', { type: 'number', min: '0', max: '1000', value: String(z.count), style: 'width:110px' });
      const lvl = h('input', { type: 'number', min: '1', max: String(z.max_level), value: z.level == null ? '' : String(z.level), placeholder: String(z.max_level), style: 'width:90px' });
      holder.append(h('div', { class: 'card', style: 'padding:14px;margin-bottom:10px' },
        h('div', { class: 'row', style: 'gap:14px;flex-wrap:wrap;align-items:flex-end' },
          h('div', { style: 'min-width:170px' }, h('b', {}, z.label), h('br'), h('small', { class: 'muted' }, 'bölge ' + z.zone)),
          h('label', { class: 'f' }, 'Açılışta bot gönder', h('div', {}, en)),
          field('Her millet için bot sayısı', cnt),
          field('Seviye (boşsa ' + z.max_level + ')', lvl),
          h('button', { class: 'btn primary', onclick: async () => {
            const c = Number(cnt.value);
            if (!(c >= 0 && c <= 1000)) return toast('Bot sayısı 0 ile 1000 arasında olmalı', 'err');
            await api('/pk-autospawn/' + z.zone, { body: { enabled: en.checked, count: Math.round(c), level: lvl.value === '' ? null : Number(lvl.value) } });
            toast('Kaydedildi', 'ok'); load();
          } }, 'Kaydet'))));
    });
  }
  main.append(
    head('PK Otomatik Bot', 'Sunucu açılışında PK bölgelerine otomatik bot gönder.',
      h('button', { class: 'btn', onclick: async () => {
        await api('/pk-autospawn/apply', { body: {} });
        toast('Uygulanıyor — botlar birkaç saniye içinde doğar', 'ok');
      } }, '▶ Şimdi uygula')),
    holder);
  await load();
};
