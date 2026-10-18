'use strict';
/* ───────── pages: dashboard, players, accounts, clans, items, quest, race, start, events, letters, system, audit ───────── */
function donut(a,b,labelA,labelB){
  const total=Math.max(1,a+b),r=42,C=2*Math.PI*r,la=C*a/total;
  const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 120 120');svg.setAttribute('width','120');
  svg.innerHTML=`<circle cx="60" cy="60" r="${r}" fill="none" stroke="#1b3267" stroke-width="16"/><circle cx="60" cy="60" r="${r}" fill="none" stroke="#fb7185" stroke-width="16" stroke-dasharray="${la} ${C-la}" transform="rotate(-90 60 60)"/><circle cx="60" cy="60" r="${r}" fill="none" stroke="#60a5fa" stroke-width="16" stroke-dasharray="${C-la} ${la}" stroke-dashoffset="${-la}" transform="rotate(-90 60 60)"/>`;
  return h('div',{class:'row',style:'flex-wrap:nowrap'},svg,h('div',{class:'col',style:'gap:4px'},h('span',{class:'chip karus'},labelA+' '+fmt(a)),h('span',{class:'chip elmo'},labelB+' '+fmt(b))));
}
PAGE.dashboard=async function(main){
  const d=await api('/dashboard');const up=d.heartbeat_age!=null&&d.heartbeat_age<20;
  const kpi=(l,v,sub,i)=>h('div',{class:'card kpi'},h('div',{class:'row muted sm'},ico(i),l),h('b',{},fmt(v)),sub?h('span',{class:'muted sm'},sub):null);
  main.append(head('Genel Bakış','Sunucu ve içerik özeti'),
    h('div',{class:'grid g4'},kpi('Hesap',d.accounts,'bugün +'+fmt(d.new_today),'users'),kpi('Karakter',d.characters,null,'shield'),kpi('Çevrimiçi',d.online,null,'zap'),kpi('Klan',d.clans,null,'clan'),kpi('Yasaklı hesap',d.banned,null,'flag'),kpi('Item şablonu',d.items,null,'box'),kpi('Canavar şablonu',d.monsters,null,'sword')),
    h('div',{class:'grid g2',style:'margin-top:14px'},
      h('div',{class:'card'},h('h3',{},'Ulus dağılımı'),donut(d.karus,d.elmorad,'Karus','El Morad')),
      h('div',{class:'card'},h('h3',{},'Sunucu durumu'),
        h('p',{},h('span',{class:'dot '+(up?'ok':'bad')}),' Oyun sunucusu: ',up?'çevrimiçi (son sinyal '+d.heartbeat_age+' sn önce)':'kapalı veya sinyal yok'),
        h('p',{class:'muted'},'Veritabanı saati: ',when(d.db_time),' · Bekleyen komut: ',fmt(d.pending_commands)),
        h('div',{class:'hint'},'Panel PostgreSQL ile doğrudan çalışır. Sunucu kapalıyken yapılan değişiklikler açılışta okunur; açıkken yeniden yükleme komutu sıraya alınır.'))),
    h('div',{class:'grid g2',style:'margin-top:14px'},
      h('div',{class:'card'},h('h3',{},'Son yönetici işlemleri'),h('div',{class:'tw'},h('table',{},h('tbody',{},(d.recent_audit||[]).map(a=>h('tr',{},h('td',{class:'xs muted nowrap'},when(a.occurred_at)),h('td',{},a.actor),h('td',{},h('span',{class:'chip blue'},a.action)),h('td',{class:'muted'},a.target))))))),
      h('div',{class:'card'},h('h3',{},'Çevrimiçi oyuncular'),h('div',{class:'tw',id:'onl'},'Yükleniyor…'))));
  const on=await api('/online');
  clear($('#onl')).append(on.length?h('table',{},h('thead',{},h('tr',{},['Karakter','Ulus','Sınıf','Lv','Bölge'].map(x=>h('th',{},x)))),
    h('tbody',{},on.map(o=>h('tr',{class:'click',onclick:()=>location.hash='#/char/'+encodeURIComponent(o.name)},h('td',{},o.name),h('td',{},nationChip(o.nation)),h('td',{},o.class?className(o.class):'—'),h('td',{},o.level),h('td',{},o.zone))))):h('div',{class:'empty'},'Şu an çevrimiçi oyuncu yok.'));
};

/* players & accounts */
PAGE.players=async function(main){
  let kind='';const out=h('div',{class:'tw'});
  const sb=searchBar('Hesap, karakter adı veya e-posta…',()=>run());
  const seg=h('div',{class:'seg'},[['','Tümü'],['online','Çevrimiçi'],['banned','Yasaklı'],['gm','GM'],['unverified','Onaysız e-posta']].map(([k,l])=>h('button',{class:k===kind?'on':'',onclick:function(){kind=k;$$('button',seg).forEach(b=>b.classList.toggle('on',b===this));run()}},l)));
  main.append(head('Hesaplar & Oyuncular','Hesap, karakter veya e-posta ile ara'),h('div',{class:'row',style:'margin-bottom:12px'},sb.el,seg),h('div',{class:'card'},out));
  async function run(){
    const rows=await api('/accounts?kind='+kind+'&q='+encodeURIComponent(sb.input.value.trim()));
    clear(out).append(rows.length?h('table',{},h('thead',{},h('tr',{},['Hesap','Karakterler','KC','TL','Yetki','Durum','Kayıt'].map(x=>h('th',{},x)))),
      h('tbody',{},rows.map(a=>h('tr',{class:'click',onclick:()=>location.hash='#/account/'+encodeURIComponent(a.account)},
        h('td',{},h('b',{},a.account)),h('td',{class:'muted'},(a.chars||[]).join(', ')),h('td',{},fmt(a.kc)),h('td',{},fmt(a.tl)),
        h('td',{},a.authority<0?h('span',{class:'chip bad'},'Yasaklı'):a.authority===0?h('span',{class:'chip blue'},'GM'):'Normal'),
        h('td',{},a.online?h('span',{class:'chip ok'},'çevrimiçi'):''),h('td',{class:'xs muted'},when(a.created)))))):h('div',{class:'empty'},'Sonuç yok.'));
  }
  run();sb.input.focus();
};
PAGE.account=async function(main,[id]){
  const a=await api('/accounts/'+encodeURIComponent(id));
  main.append(h('div',{class:'row',style:'margin-bottom:14px'},h('a',{href:'#/players',class:'btn ghost'},'← Hesaplar'),h('h1',{},a.account),a.online?h('span',{class:'chip ok'},'çevrimiçi'):null,a.authority<0?h('span',{class:'chip bad'},'yasaklı'):null),
    h('div',{class:'grid g2'},
      h('div',{class:'card'},h('h3',{},'Hesap bilgileri'),
        h('div',{class:'grid g2',style:'gap:10px;margin-bottom:12px'},h('div',{class:'card flat kpi'},h('span',{class:'muted sm'},'KC (cash_point)'),h('b',{},fmt(a.kc))),h('div',{class:'card flat kpi'},h('span',{class:'muted sm'},'TL bakiyesi (bonus_cash_point)'),h('b',{},fmt(a.tl)))),
        h('p',{class:'muted'},'Premium tipi ',a.premium,' · Kayıt ',when(a.created),' · E-posta ',a.email||'—'),
        h('p',{class:'muted'},'Ad soyad ',((a.name||'')+' '+(a.surname||'')).trim()||'—',' · Telefon ',a.phone||'—'),
        h('p',{class:'muted'},'Gizli soru ',a.security_question||'—',' · Son IP ',a.client_ip||'—',' · Banka altını ',fmt(a.warehouse_coins)),
        h('p',{class:'muted'},'Yetki (str_authority) ',h('b',{},a.authority),' — ',a.authority<0?'yasaklı':a.authority===0||a.authority===2?'GM (web panel /admin erişimi açık)':'normal oyuncu'),
        h('p',{class:'muted'},'E-posta onayı (account_check) ',a.account_check?h('span',{class:'chip ok'},'onaylı'):h('span',{class:'chip bad'},'onaysız — giriş yapamaz')),
        a.authority<0?h('p',{class:'muted'},'Ceza: ',when(a.punishment_date),' · ',a.punishment_days||0,' gün (0 = süresiz)'):null,
        h('div',{class:'row'},h('button',{class:'btn',onclick:()=>balanceDialog(a)},'KC / TL düzenle'),h('button',{class:'btn',onclick:()=>authorityDialog(a)},'Yetki değiştir'),
          !a.account_check?h('button',{class:'btn primary',onclick:async()=>{try{await api('/accounts/'+encodeURIComponent(a.account)+'/activate',{body:{}});toast('Hesap onaylandı','ok');route()}catch(_){}}},'Hesabı onayla (mail atlanarak)'):null,
          a.authority<0?h('button',{class:'btn',onclick:()=>banDialog(a,false)},'Yasağı kaldır'):h('button',{class:'btn danger',onclick:()=>banDialog(a,true)},'Hesabı yasakla')),
        h('div',{class:'hint',style:'margin-top:12px'},'Parola, mühür parolası ve OTP hiçbir yerde gösterilmez. Bakiye değişikliği hesap çevrimiçiyken reddedilir.')),
      h('div',{class:'card'},h('h3',{},'Karakterler'),(a.characters||[]).length?h('div',{class:'mlist'},(a.characters||[]).map(c=>h('div',{class:'mrow',onclick:()=>location.hash='#/char/'+encodeURIComponent(c.name)},ico('shield'),
        h('div',{},h('b',{},c.name),h('br'),h('small',{},className(c.class)+' · Lv '+c.level+(c.rebirth?' +'+c.rebirth:'')+' · bölge '+c.zone)),h('span',{class:'grow'}),nationChip(c.nation),c.online?h('span',{class:'chip ok'},'çevrimiçi'):null))):h('div',{class:'empty'},'Karakter yok'))));
};
function balanceDialog(a){
  const f=h('select',{},h('option',{value:'kc'},'KC (cash_point)'),h('option',{value:'tl'},'TL bakiyesi (bonus_cash_point)'));
  const m=h('select',{},h('option',{value:'add'},'Ekle / çıkar (+/−)'),h('option',{value:'set'},'Yeni değer belirle'));
  const n=h('input',{type:'number',value:'0'}),r=h('input',{placeholder:'Sebep (zorunlu)'});const info=h('p',{class:'muted sm'});
  const upd=()=>{const cur=f.value==='kc'?a.kc:a.tl,v=Number(n.value||0);info.textContent='Şimdi: '+fmt(cur)+' → Sonra: '+fmt(m.value==='set'?v:cur+v)};
  [f,m,n].forEach(e=>e.addEventListener('input',upd));upd();
  modal('KC / TL düzenle — '+a.account,h('div',{class:'form'},field('Alan',f),field('İşlem',m),field('Miktar',n),field('Sebep',r,true),h('div',{class:'full'},info)),[
    {label:'İptal',cls:'ghost'},{label:'Uygula',cls:'primary',keep:true,fn:async close=>{
      try{const d=await api('/accounts/'+encodeURIComponent(a.account)+'/balance',{body:{field:f.value,mode:m.value,amount:Number(n.value),reason:r.value}});toast('Bakiye: '+fmt(d.before)+' → '+fmt(d.after),'ok');close();route()}catch(_){}return true}}]);
}
function authorityDialog(a){
  const f=h('select',{},h('option',{value:'1',selected:a.authority!==0&&a.authority!==2},'1 · Normal oyuncu'),
    h('option',{value:'0',selected:a.authority===0},'0 · GM (web panel /admin erişimi)'),
    h('option',{value:'2',selected:a.authority===2},'2 · GM (web panel /admin erişimi)'));
  const r=h('input',{placeholder:'Sebep (zorunlu)'});
  modal('Yetki değiştir — '+a.account,h('div',{class:'form'},field('Yetki',f),field('Sebep',r,true),
    h('p',{class:'xs muted full'},'Yasaklamak/yasağı kaldırmak için bu diyalog yerine "Hesabı yasakla" butonunu kullan.')),[
    {label:'İptal',cls:'ghost'},{label:'Uygula',cls:'primary',keep:true,fn:async close=>{
      try{await api('/accounts/'+encodeURIComponent(a.account)+'/authority',{body:{authority:Number(f.value),reason:r.value}});toast('Yetki güncellendi','ok');close();route()}catch(_){}return true}}]);
}
function banDialog(a,ban){
  const d=h('input',{type:'number',value:'0',min:'0'}),r=h('input',{placeholder:'Sebep (zorunlu)'});
  modal((ban?'Yasakla — ':'Yasağı kaldır — ')+a.account,h('div',{class:'form'},ban?field('Süre (gün, 0 = süresiz)',d):null,field('Sebep',r,true)),[
    {label:'İptal',cls:'ghost'},{label:'Onayla',cls:ban?'danger':'primary',keep:true,fn:async close=>{
      try{await api('/accounts/'+encodeURIComponent(a.account)+'/ban',{body:{ban,days:Number(d.value||0),reason:r.value}});toast('Kaydedildi','ok');close();route()}catch(_){}return true}}]);
}

/* clans */
PAGE.clans=async function(main){
  const out=h('div',{class:'tw'});const sb=searchBar('Klan adı veya numarası…',()=>run());
  main.append(head('Klanlar','Klan listesi, üyeler ve banka'),h('div',{class:'row',style:'margin-bottom:12px'},sb.el),h('div',{class:'card'},out));
  async function run(){
    const rows=await api('/clans?q='+encodeURIComponent(sb.input.value.trim()));
    clear(out).append(rows.length?h('table',{},h('thead',{},h('tr',{},['Klan','Ulus','Üye','Lider','Puan','Klan altını','Banka altını'].map(x=>h('th',{},x)))),h('tbody',{},rows.map(c=>h('tr',{class:'click',onclick:()=>location.hash='#/clan/'+c.id},
      h('td',{},h('b',{},c.name),' ',h('span',{class:'muted xs'},'#'+c.id)),h('td',{},nationChip(c.nation)),h('td',{},c.members),h('td',{},c.chief),h('td',{},fmt(c.points)),h('td',{},fmt(c.gold)),h('td',{},fmt(c.bank_gold)))))):h('div',{class:'empty'},'Klan yok'));
  }
  run();
};
PAGE.clan=async function(main,[id]){
  const c=await api('/clans/'+id);
  main.append(h('div',{class:'row',style:'margin-bottom:14px'},h('a',{href:'#/clans',class:'btn ghost'},'← Klanlar'),h('h1',{},c.name),nationChip(c.nation),h('span',{class:'chip'},'#'+c.id),h('span',{class:'chip'},c.members_count+' üye'),h('span',{class:'grow'}),
    h('a',{class:'btn primary',href:'#/char/'+encodeURIComponent((c.members[0]||{}).name||c.chief)+'/clan'},ico('bag'),'Klan bankası')),
    h('div',{class:'grid g4',style:'margin-bottom:14px'},h('div',{class:'card kpi'},h('span',{class:'muted sm'},'Lider'),h('b',{style:'font-size:20px'},c.chief)),h('div',{class:'card kpi'},h('span',{class:'muted sm'},'Klan puanı'),h('b',{},fmt(c.points))),h('div',{class:'card kpi'},h('span',{class:'muted sm'},'Klan altını'),h('b',{},fmt(c.gold))),h('div',{class:'card kpi'},h('span',{class:'muted sm'},'Banka altını'),h('b',{},fmt(c.bank_gold)))),
    h('div',{class:'card'},h('h3',{},'Üyeler'),h('div',{class:'tw'},h('table',{},h('thead',{},h('tr',{},['Karakter','Sınıf','Seviye','Loyalty','Durum'].map(x=>h('th',{},x)))),h('tbody',{},c.members.map(m=>h('tr',{class:'click',onclick:()=>location.hash='#/char/'+encodeURIComponent(m.name)},h('td',{},h('b',{},m.name),m.name===c.chief?h('span',{class:'chip blue',style:'margin-left:8px'},'lider'):null),h('td',{},className(m.class)),h('td',{},m.level),h('td',{},fmt(m.loyalty)),h('td',{},m.online?h('span',{class:'chip ok'},'çevrimiçi'):''))))))));
};

/* item database */
PAGE.items=async function(main,[idArg]){
  let kind='';const res=h('div',{class:'res',style:'grid-template-columns:repeat(auto-fill,minmax(54px,1fr))'});const detail=h('div',{class:'panel',style:'position:sticky;top:78px'},h('div',{class:'empty'},'Bir item seç'));
  const sb=searchBar('Item adı veya tam ID…',()=>run());
  const KINDS=[['','Tümü'],['21','Kılıç'],['22','2 el kılıç'],['31','Balta'],['41','Gürz'],['51','Mızrak'],['70','Yay'],['60','Kalkan'],['91','Küpe'],['92','Kolye'],['93','Yüzük'],['94','Kemer'],['97','Tüketilebilir'],['252','Cospre']];
  const chips=h('div',{class:'chips'},KINDS.map(([k,l])=>h('button',{class:k===''?'on':'',onclick:function(){kind=k;$$('button',chips).forEach(b=>b.classList.toggle('on',b===this));run()}},l)));
  async function show(id){
    const it=await getItem(id);clear(detail);if(!it)return detail.append(h('div',{class:'empty'},'Bulunamadı'));
    const nz=Object.entries(it).filter(([k,v])=>v!==0&&v!==null&&v!==''&&!['description'].includes(k));
    detail.append(h('div',{class:'row',style:'flex-wrap:nowrap;align-items:flex-start'},h('div',{class:'slot full',style:'width:64px;height:64px;flex:none'},h('img',{src:icon(it.item_icon_id1),onerror:iconErr})),h('div',{style:'min-width:0;flex:1'},renderTip(it,null,null,false))),
      h('h4',{style:'margin:14px 0 6px'},'Tüm alanlar'),h('div',{class:'tw',style:'max-height:320px'},h('table',{},h('tbody',{},nz.map(([k,v])=>h('tr',{},h('td',{class:'muted xs'},k),h('td',{class:'mono xs'},String(v))))))));
  }
  async function run(){
    const t=sb.input.value.trim();clear(res);if(!t&&!kind)return;
    const items=await api('/item-search?q='+encodeURIComponent(t)+(kind?'&kind='+kind:'')+'&limit=200',{quiet:true}).catch(()=>[]);
    items.forEach(it=>{const s=itemSlot(it,{drag:false});s.addEventListener('click',()=>show(it.id));res.append(s)});
    if(!items.length)res.append(h('span',{class:'muted'},'Sonuç yok'));
  }
  const rangeBtn=h('button',{class:'btn',onclick:()=>{
    const F={from:h('input',{type:'number',placeholder:'184430000'}),to:h('input',{type:'number',placeholder:'184439999'}),icon:h('input',{type:'number',placeholder:'18443000'}),why:h('input',{placeholder:'Sebep (zorunlu)'})};
    const prev=h('div',{class:'slot full',style:'width:56px;height:56px'});
    F.icon.addEventListener('input',()=>{clear(prev).append(h('img',{src:icon(Number(F.icon.value)),onerror:iconErr}))});
    modal('Aralığa ikon ata',h('div',{class:'col'},h('div',{class:'hint'},'Item numarası aralığındaki tüm itemlerin ikon numarası değişir (varyantlar dahil). İkon numarası = itemicon_a_bbbb_cc_d dosyasındaki rakamlar birleşik: itemicon_1_8443_00_0 → 18443000.'),
      h('div',{class:'form'},field('Item no (başlangıç)',F.from),field('Item no (bitiş)',F.to),field('İkon numarası',F.icon),field('Önizleme',prev),field('Sebep',F.why,true))),[
      {label:'İptal',cls:'ghost'},{label:'Uygula',cls:'primary',keep:true,fn:async close=>{
        try{const d=await api('/icon-range',{body:{from:Number(F.from.value),to:Number(F.to.value),icon_id:Number(F.icon.value),reason:F.why.value}});toast(d.rows+' item güncellendi','ok');itemCache.clear();close()}catch(_){}return true}}]);
  }},ico('layers'),'Aralığa ikon ata');
  main.append(head('Item Veritabanı','Oyundaki gibi tooltip ile tüm item şablonları',rangeBtn),h('div',{class:'row',style:'margin-bottom:6px'},sb.el),chips,
    h('div',{class:'grid',style:'grid-template-columns:minmax(0,1fr) minmax(300px,380px);align-items:start'},h('div',{class:'card'},res),detail));
  if(idArg){sb.input.value=idArg;run();show(Number(idArg))}
};

/* quest studio */
PAGE.quest=async function(main){
  const sb=searchBar('NPC adı, SID veya Lua dosyası…',()=>run());const list=h('div',{class:'mlist'}),work=h('div');
  main.append(head('Quest Stüdyosu','NPC görev kayıtları ve Lua betikleri'),h('div',{class:'grid',style:'grid-template-columns:300px minmax(0,1fr);align-items:start'},h('div',{class:'panel'},h('h4',{},'NPC'),sb.el,list),work));
  async function run(){
    const t=sb.input.value.trim();if(!t){clear(list).append(h('div',{class:'empty sm'},'Arama yap'));return}
    const r=await api('/quest-studio/npcs?q='+encodeURIComponent(t),{quiet:true}).catch(()=>[]);
    clear(list).append(...r.map(n=>h('div',{class:'mrow',onclick:()=>open(n.npc_id)},ico('scroll'),h('div',{},h('b',{},n.name),h('br'),h('small',{},'SID '+n.npc_id+' · '+n.quest_count+' kayıt · '+(n.lua||'—'))))));
    if(!r.length)list.append(h('div',{class:'empty'},'NPC yok'));
  }
  async function open(sid){
    const d=await api('/quest-studio/npc/'+sid);clear(work);
    const lua=(d.quests.find(q=>q.lua)||{}).lua;
    work.append(h('div',{class:'card'},h('div',{class:'row',style:'margin-bottom:10px'},h('h2',{},d.npc_name),h('span',{class:'chip'},'SID '+d.npc_id),h('span',{class:'grow'}),lua?h('button',{class:'btn primary',onclick:()=>luaEditor(lua)},ico('scroll'),'Lua: '+lua):null),
      d.quests.length?h('div',{class:'tw'},h('table',{},h('thead',{},h('tr',{},['#','Lv','Tür','Bölge','Olay','Tetik','Ödül/değişim','Lua','Konuşma'].map(x=>h('th',{},x)))),
        h('tbody',{},d.quests.map(q=>h('tr',{},h('td',{},q.index),h('td',{},q.level),h('td',{},q.quest_type),h('td',{},q.zone),h('td',{},q.event_data+'/'+q.event_status),h('td',{},q.trigger),h('td',{},q.complete+' / '+q.exchange),h('td',{class:'mono xs'},q.lua),h('td',{class:'xs muted',style:'max-width:260px'},(q.main_text||q.event_text||'').slice(0,90))))))):h('div',{class:'empty'},'Bu NPC için quest kaydı yok.')));
  }
  async function luaEditor(file){
    const d=await api('/quest-studio/lua/'+encodeURIComponent(file));
    const ta=h('textarea',{class:'editor',spellcheck:'false'});ta.value=d.content;
    modal(file,h('div',{class:'col'},h('div',{class:'hint'},'Kayıtta söz dizimi derlenerek denenir, hatalıysa reddedilir. Eski dosyanın yedeği alınır.'),ta),[{label:'Kapat',cls:'ghost'},{label:'Kaydet',cls:'primary',keep:true,fn:async()=>{
      try{await api('/quest-studio/lua/'+encodeURIComponent(file),{body:{content:ta.value}});toast('Kaydedildi (yedek alındı)','ok');await api('/commands',{body:{command:'reload',args:{what:'quest_tables'}}},{quiet:true})}catch(_){}return true}}],'xl');
  }
  run();
};

/* collection race */
PAGE.race=async function(main){
  const d=await api('/collection-race/config');
  const body=h('div');main.append(head('Collection Race','Etkinlik ayarları ve ödüller',h('button',{class:'btn primary',onclick:()=>settingDialog(null)},ico('plus'),'Yeni etkinlik')),body);
  const zname=id=>(d.zones.find(z=>z.id===id)||{}).name||('Bölge '+id);
  function draw(){
    clear(body).append(...d.settings.map(s=>{
      const rw=d.rewards.filter(r=>r.event_id===s.index);
      return h('div',{class:'card',style:'margin-bottom:14px'},h('div',{class:'row',style:'margin-bottom:10px'},ico('flag'),h('h2',{},s.name||('Etkinlik '+s.index)),h('span',{class:'chip'},'#'+s.index),h('span',{class:'chip blue'},zname(s.zone)),h('span',{class:'chip'},'Lv '+s.min_level+'–'+s.max_level),h('span',{class:'chip'},s.minutes+' dk'),
        s.auto_start?h('span',{class:'chip ok'},'otomatik '+String(s.auto_hour).padStart(2,'0')+':'+String(s.auto_minute).padStart(2,'0')):null,h('span',{class:'grow'}),
        h('button',{class:'btn sm',onclick:()=>settingDialog(s)},'Düzenle'),h('button',{class:'btn sm danger',onclick:async()=>{if(!await confirmBox('Etkinliği sil','Etkinlik ve ayarı silinecek.','Sil',true))return;try{await api('/collection-race/settings/'+s.index,{method:'DELETE'});toast('Silindi','ok');route()}catch(_){}}},ico('trash'))),
        h('div',{class:'row',style:'margin-bottom:10px'},s.targets.filter(t=>t.sid).map(t=>h('span',{class:'chip'},t.name+' ×'+t.count))),
        h('h4',{style:'margin:8px 0'},'Ödüller'),
        h('div',{class:'dslots'},rw.map(r=>h('div',{class:'ds-slot full',onclick:()=>rewardDialog(s.index,r)},itemSlot({id:r.item_id,icon:0},{drag:false}),h('div',{class:'t'},h('b',{},r.item_name||('#'+r.item_id)),h('span',{class:'pct'},'×'+r.count+' · %'+r.rate)))),
          h('div',{class:'ds-slot',style:'cursor:pointer;justify-content:center',onclick:()=>rewardDialog(s.index,null)},ico('plus'),'Ödül ekle')));
    }));
    if(!d.settings.length)body.append(h('div',{class:'card empty'},'Etkinlik yok'));
  }
  function settingDialog(s){
    const v=s||{name:'',targets:[{sid:0,count:1},{sid:0,count:1},{sid:0,count:1}],min_level:1,max_level:83,zone:21,minutes:60,user_limit:1,repeat:false,auto_start:false,auto_hour:20,auto_minute:0};
    const name=h('input',{value:v.name}),minL=h('input',{type:'number',value:v.min_level}),maxL=h('input',{type:'number',value:v.max_level}),mins=h('input',{type:'number',value:v.minutes}),lim=h('input',{type:'number',value:v.user_limit});
    const zone=h('select',{},d.zones.map(z=>h('option',{value:z.id,selected:z.id===v.zone},z.id+' · '+z.name)));
    const rep=h('input',{type:'checkbox',checked:v.repeat}),auto=h('input',{type:'checkbox',checked:v.auto_start}),ah=h('input',{type:'number',value:v.auto_hour,min:'0',max:'23'}),am=h('input',{type:'number',value:v.auto_minute,min:'0',max:'59'});
    const tg=v.targets.map(t=>({sid:h('input',{type:'number',value:t.sid}),count:h('input',{type:'number',value:t.count,min:'1'})}));
    const tgrow=tg.map((t,i)=>h('div',{class:'row'},h('b',{},'Hedef '+(i+1)),field('SID',t.sid),field('Adet',t.count),h('button',{class:'btn sm',onclick:()=>npcPick(n=>{t.sid.value=n.id})},'Ara')));
    modal(s?'Etkinliği düzenle':'Yeni etkinlik',h('div',{class:'col'},h('div',{class:'form'},field('Ad',name,true),field('Bölge',zone),field('Min seviye',minL),field('Max seviye',maxL),field('Süre (dk)',mins),field('Kullanıcı limiti',lim)),
      h('div',{class:'col'},tgrow),h('div',{class:'row'},h('label',{class:'row'},rep,'Tekrarla'),h('label',{class:'row'},auto,'Otomatik başlat'),field('Saat',ah),field('Dakika',am))),[
      {label:'İptal',cls:'ghost'},{label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
        const p={name:name.value,targets:tg.map(t=>({sid:Number(t.sid.value),count:Number(t.count.value)})),min_level:Number(minL.value),max_level:Number(maxL.value),zone:Number(zone.value),minutes:Number(mins.value),user_limit:Number(lim.value),repeat:rep.checked,auto_start:auto.checked,auto_hour:Number(ah.value),auto_minute:Number(am.value)};
        try{await api(s?'/collection-race/settings/'+s.index:'/collection-race/settings',{body:p});toast('Kaydedildi','ok');close();route()}catch(_){}return true}}],'wide');
  }
  function rewardDialog(ev,r){
    const v=r||{description:'',item_id:0,count:1,rate:100,time:0,flag:0,session:0};
    const F={description:h('input',{value:v.description}),item_id:h('input',{type:'number',value:v.item_id}),count:h('input',{type:'number',value:v.count}),rate:h('input',{type:'number',value:v.rate}),time:h('input',{type:'number',value:v.time}),flag:h('input',{type:'number',value:v.flag}),session:h('input',{type:'number',value:v.session})};
    const lib=libraryPanel({onPick:it=>{F.item_id.value=it.id}});
    modal(r?'Ödülü düzenle':'Ödül ekle',h('div',{class:'col'},h('div',{class:'form'},field('Açıklama',F.description,true),field('Item ID',F.item_id),field('Adet',F.count),field('Oran (%)',F.rate),field('Süre',F.time),field('Flag',F.flag),field('Session',F.session)),lib.el),[
      {label:'İptal',cls:'ghost'},r?{label:'Sil',cls:'danger',fn:async()=>{try{await api('/collection-race/rewards/'+r.index,{method:'DELETE'});toast('Silindi','ok');route()}catch(_){}}}:null,
      {label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
        const p={};for(const k of Object.keys(F))p[k]=k==='description'?F[k].value:Number(F[k].value);
        try{await api(r?'/collection-race/rewards/'+r.index:'/collection-race/rewards',{body:[ev,p]});toast('Kaydedildi','ok');close();route()}catch(_){}return true}}].filter(Boolean),'wide');
  }
  draw();
};
function npcPick(cb){
  const q=h('input',{placeholder:'NPC / canavar adı veya SID…',style:'width:100%'}),out=h('div',{class:'mlist',style:'margin-top:8px;max-height:50vh;overflow:auto'});let close;
  q.addEventListener('input',debounce(async()=>{const r=q.value.trim()?await api('/npc-search?q='+encodeURIComponent(q.value.trim()),{quiet:true}).catch(()=>[]):[];
    clear(out).append(...r.map(n=>h('div',{class:'mrow',onclick:()=>{cb(n);close()}},ico(n.monster?'sword':'users'),h('div',{},h('b',{},n.name),h('br'),h('small',{},'#'+n.id+' · Lv '+n.level)))))},250));
  close=modal('NPC / canavar seç',h('div',{},q,out),[{label:'Kapat',cls:'ghost'}]);
}

/* start settings: beginner editor + spawn points */
const BEGINNER_TYPES=[[0,'Standart'],[1,'Ardream'],[2,'Ronark Land Base'],[3,'Ronark Land'],[4,'62 Seviye']];
const BEGINNER_CLASSES=[[1,'Savaşçı'],[2,'Rogue'],[3,'Büyücü'],[4,'Rahip'],[13,'Kurian']];
PAGE.start=async function(main,[tabArg]){
  let tab=tabArg||'beginner';
  const body=h('div');
  const seg=h('div',{class:'seg',style:'margin-bottom:14px'},[['beginner','Yeni karakter (Beginner)'],['pos','Doğma noktaları']].map(([k,l])=>h('button',{class:k===tab?'on':'',onclick:function(){tab=k;$$('button',seg).forEach(b=>b.classList.toggle('on',b===this));draw()}},l)));
  main.append(head('Başlangıç Ayarları','Yeni karakterlerin eşyaları, statları ve bölge doğma noktaları'),seg,body);
  const draw=()=>{clear(body);return tab==='pos'?posTab(body):beginnerTab(body)};
  draw();
};

async function beginnerTab(body){
  const ov=await api('/beginner');
  let ty=ov.active,cls=1,set=null,pickedLib=null,mb=0;
  const valFor=()=>ov.values.find(v=>v.class_type===cls&&v.job_type===ty);
  const countOf=(c,t)=>((ov.counts.find(x=>x.class_type===c&&x.beginner_type===t)||{}).items)||0;
  const legacyOf=c=>((ov.legacy.find(x=>x.class_type===c)||{}).items)||0;
  const typeBar=h('div'),classBar=h('div'),work=h('div');
  const lib=libraryPanel({onPick:it=>{pickedLib=it;toast('Seçili: '+it.name+' — yuvaya tıkla','ok')}});
  body.append(h('div',{class:'card',style:'margin-bottom:14px'},h('h3',{},'Aktif başlangıç tipi'),typeBar,
    h('div',{class:'hint',style:'margin-top:10px'},'Yeni açılan her karakter, aktif tipin eşya setini ve statlarını alır. Eşya seti anında geçerli olur; başlangıç statları sunucu yeniden başlayınca yüklenir. Aşağıda her tip ve sınıf için ayrı set düzenlenir.')),
    classBar,
    h('div',{class:'studio',style:'grid-template-columns:minmax(0,1fr) 330px'},work,h('div',{class:'panel lib'},lib.el)));

  function drawBars(){
    clear(typeBar).append(h('div',{class:'row'},BEGINNER_TYPES.map(([t,l])=>
      h('button',{class:'btn'+(t===ty?' primary':''),onclick:()=>{ty=t;load()}},l+(t===ov.active?' ★':'')))),
      ty!==ov.active?h('div',{class:'row',style:'margin-top:10px'},h('span',{class:'muted'},'Düzenlenen tip aktif değil.'),h('button',{class:'btn sm',onclick:()=>makeActive()},'“'+BEGINNER_TYPES[ty][1]+'” tipini aktif yap')):h('div',{class:'chip ok',style:'margin-top:10px'},'★ Bu tip şu an aktif'));
    clear(classBar).append(h('div',{class:'seg',style:'margin-bottom:14px'},BEGINNER_CLASSES.map(([c,l])=>
      h('button',{class:c===cls?'on':'',onclick:()=>{cls=c;load()}},l+' · '+(countOf(c,ty)||('eski '+legacyOf(c)))))));
  }
  function makeActive(){
    const why=h('input',{placeholder:'Sebep (zorunlu)'});
    modal('Aktif tipi değiştir',h('div',{class:'col'},h('p',{},'Yeni karakterler artık “'+BEGINNER_TYPES[ty][1]+'” tipinin setini alacak.'),h('div',{class:'form'},field('Sebep',why,true))),[
      {label:'İptal',cls:'ghost'},{label:'Aktif yap',cls:'primary',keep:true,fn:async close=>{
        try{const d=await api('/beginner/active',{body:{ty,reason:why.value}});toast(d.note,'ok');ov.active=ty;close();drawBars()}catch(_){}return true}}]);
  }
  async function load(){
    drawBars();
    set=await api('/beginner/set?class='+cls+'&ty='+ty);
    drawWork();
  }
  async function setSlot(abs,id,count,dur){
    try{await api('/beginner/slot',{body:{class:cls,ty,slot:abs,item_id:id,count:count||1,durability:dur||0}});await reloadSet()}catch(_){}
  }
  async function reloadSet(){
    set=await api('/beginner/set?class='+cls+'&ty='+ty);
    const n=set.legacy?0:set.items.length;
    const e=ov.counts.find(x=>x.class_type===cls&&x.beginner_type===ty);if(e)e.items=n;else ov.counts.push({class_type:cls,beginner_type:ty,items:n});
    drawBars();drawWork();
  }
  function slotView(map,abs,sec,label){
    const d=map.get(abs);
    const el=h('div',{class:'slot '+(d?'full':'')+(label?' eqlabel':''),draggable:d?'true':null,'data-label':label||''},
      d?h('img',{src:icon(d.icon),onerror:iconErr}):null,d&&d.count>1?h('span',{class:'cnt'},d.count):null,h('span',{class:'num'},abs));
    if(d)attachTip(el,()=>({id:d.id,slot:{durability:d.durability,count:d.count},note:sec+' · slot '+abs}));else el.title=sec+' — boş (slot '+abs+')';
    el.addEventListener('dragstart',e=>e.dataTransfer.setData('text/plain',JSON.stringify({bfrom:abs})));
    el.addEventListener('dragover',e=>{e.preventDefault();el.classList.add('over')});
    el.addEventListener('dragleave',()=>el.classList.remove('over'));
    el.addEventListener('drop',async e=>{
      e.preventDefault();el.classList.remove('over');let p;try{p=JSON.parse(e.dataTransfer.getData('text/plain'))}catch(_){return}
      if(p.pick){if(p.pick.group)return toast('Başlangıç setine grup konamaz','err');return setSlot(abs,p.pick.id,1,0)}
      if(p.bfrom!=null&&p.bfrom!==abs){try{await api('/beginner/move',{body:{class:cls,ty,from:p.bfrom,to:abs}});await reloadSet()}catch(_){}}
    });
    el.addEventListener('click',()=>{
      if(pickedLib&&!pickedLib.group){const it=pickedLib;pickedLib=null;return setSlot(abs,it.id,1,0)}
      slotDialog(abs,sec,d);
    });
    return el;
  }
  function slotDialog(abs,sec,d){
    const id=h('input',{type:'number',value:d?d.id:0,min:'0'}),cnt=h('input',{type:'number',value:d?d.count:1,min:'1'}),dur=h('input',{type:'number',value:d?d.durability:0});
    const card=h('div');
    const show=async()=>{clear(card);const v=Number(id.value);if(!v)return;const it=await getItem(v);card.append(it?renderTip(it,null,null,true):h('div',{class:'hint warn'},'Bu ID item tablosunda yok.'))};
    id.addEventListener('change',show);show();
    modal(sec+' · slot '+abs,h('div',{class:'grid g2',style:'gap:16px'},card,h('div',{class:'form'},field('Item ID (0 = boşalt)',id),field('Adet',cnt),field('Dayanıklılık (0 = item varsayılanı)',dur))),[
      {label:'İptal',cls:'ghost'},d?{label:'Sil',cls:'danger',fn:()=>setSlot(abs,0,1,0)}:null,
      {label:'Kaydet',cls:'primary',keep:true,fn:async close=>{await setSlot(abs,Number(id.value),Number(cnt.value),Number(dur.value));close();return true}}].filter(Boolean),'wide');
  }
  function statsCard(){
    const v=valFor();if(!v)return h('div',{class:'card empty'},'Bu sınıf/tip için stat satırı yok.');
    const F=[['level','Seviye'],['exp','EXP'],['strength','STR'],['health','HP (STA)'],['dexterity','DEX'],['intelligence','INT'],['magic_power','MP (CHA)'],['free_points','Serbest stat puanı'],['skill_point_free','Serbest beceri puanı'],['skill_point_cat1','Beceri 1. kategori'],['skill_point_cat2','Beceri 2. kategori'],['skill_point_cat3','Beceri 3. kategori'],['skill_point_master','Usta beceri'],['gold','Noah']];
    const I={};
    return h('div',{class:'card',style:'margin-top:14px'},h('h3',{},BEGINNER_CLASSES.find(x=>x[0]===cls)[1]+' · '+BEGINNER_TYPES[ty][1]+' başlangıç statları'),
      h('div',{class:'form'},F.map(([k,l])=>{I[k]=h('input',{type:'number',value:v[k]});return field(l,I[k])})),
      h('div',{class:'row',style:'margin-top:14px'},h('button',{class:'btn primary',onclick:async()=>{
        const p={class:cls,ty};for(const [k] of F)p[k]=Number(I[k].value);
        try{const d=await api('/beginner/values',{body:p});Object.assign(v,Object.fromEntries(F.map(([k])=>[k,p[k]])));toast(d.note,'ok')}catch(_){}}},'Statları kaydet')));
  }
  function copyDialog(){
    const fc=h('select',{},BEGINNER_CLASSES.map(([c,l])=>h('option',{value:c,selected:c===cls},l))),ft=h('select',{},BEGINNER_TYPES.map(([t,l])=>h('option',{value:t,selected:t===ty},l)));
    const why=h('input',{placeholder:'Sebep (zorunlu)'});
    modal('Seti kopyala',h('div',{class:'col'},h('p',{class:'muted'},'Seçtiğin kaynak set, şu an açık olan “'+BEGINNER_CLASSES.find(x=>x[0]===cls)[1]+' · '+BEGINNER_TYPES[ty][1]+'” setinin üzerine yazılır.'),h('div',{class:'form'},field('Kaynak sınıf',fc),field('Kaynak tip',ft),field('Sebep',why,true))),[
      {label:'İptal',cls:'ghost'},{label:'Kopyala',cls:'primary',keep:true,fn:async close=>{
        try{const d=await api('/beginner/copy',{body:{from_class:Number(fc.value),from_ty:Number(ft.value),to_class:cls,to_ty:ty,reason:why.value}});toast(d.rows+' yuva kopyalandı','ok');close();reloadSet()}catch(_){}return true}}]);
  }
  function drawWork(){
    clear(work);
    const map=new Map(set.items.map(i=>[i.slot,i]));
    const trash=h('div',{class:'trash',style:'margin:12px 0'},ico('trash'),h('span',{class:'sm'},' Item\'ı buraya sürükle: setten çıkar'));
    trash.addEventListener('dragover',e=>{e.preventDefault();trash.classList.add('over')});trash.addEventListener('dragleave',()=>trash.classList.remove('over'));
    trash.addEventListener('drop',e=>{e.preventDefault();trash.classList.remove('over');let p;try{p=JSON.parse(e.dataTransfer.getData('text/plain'))}catch(_){return}if(p.bfrom!=null)setSlot(p.bfrom,0,1,0)});
    const cname=BEGINNER_CLASSES.find(x=>x[0]===cls)[1];
    work.append(...[
      set.legacy?h('div',{class:'hint warn',style:'margin-bottom:12px'},'“'+cname+' · '+BEGINNER_TYPES[ty][1]+'” için özel set yok; sunucu eski sınıf setini kullanıyor. İlk düzenlemede bu set buraya kopyalanır.'):null,
      h('div',{class:'row',style:'margin-bottom:12px'},h('h2',{},cname+' · '+BEGINNER_TYPES[ty][1]),h('span',{class:'chip'},set.items.length+' item'),h('span',{class:'grow'}),
        h('button',{class:'btn',onclick:copyDialog},ico('copy'),'Başka setten kopyala')),
      invLayout((abs,sec,label)=>slotView(map,abs,sec,label),{mb,setMb:k=>{mb=k;drawWork()},charBox:h('div',{class:'cbox'},h('b',{},cname),h('div',{class:'muted sm'},BEGINNER_TYPES[ty][1]),h('div',{class:'cbox-sp'}),h('div',{class:'sm'},'Başlangıç Noah'),h('b',{},fmt((valFor()||{}).gold)))}),
      trash,statsCard()].filter(Boolean));
  }
  load();
}

async function posTab(body){
  const rows=await api('/start-positions');
  body.append(h('div',{class:'card'},h('div',{class:'hint',style:'margin-bottom:10px'},'Doğma noktaları oyun sunucusu açılırken okunur; değişiklik için yeniden başlatma gerekir.'),h('div',{class:'tw'},h('table',{},h('thead',{},h('tr',{},['Bölge','Karus X/Z','El Morad X/Z','Karus kapı','El Morad kapı',''].map(x=>h('th',{},x)))),
    h('tbody',{},rows.map(r=>h('tr',{},h('td',{},h('b',{},r.zone)),h('td',{},r.karus_x+' / '+r.karus_z),h('td',{},r.elmorad_x+' / '+r.elmorad_z),h('td',{},r.karus_gate_x+' / '+r.karus_gate_z),h('td',{},r.elmo_gate_x+' / '+r.elmo_gate_z),h('td',{},h('button',{class:'btn sm',onclick:()=>posDialog(r)},'Düzenle')))))))));
  function posDialog(r){
    const keys=['karus_x','karus_z','elmorad_x','elmorad_z','karus_gate_x','karus_gate_z','elmo_gate_x','elmo_gate_z'];const F={};
    modal('Bölge '+r.zone,h('div',{class:'form'},keys.map(k=>{F[k]=h('input',{type:'number',value:r[k]});return field(k,F[k])})),[{label:'İptal',cls:'ghost'},{label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
      const p={};keys.forEach(k=>p[k]=Number(F[k].value));try{await api('/start-positions/'+r.zone,{body:p});toast('Kaydedildi','ok');close();route()}catch(_){}return true}}]);
  }
}

/* events */
const DAYS=['Paz','Pzt','Sal','Çar','Per','Cum','Cmt'];
PAGE.events=async function(main){
  const d=await api('/events');
  const ctl=(label,args,danger)=>h('button',{class:'btn '+(danger?'danger':''),onclick:async()=>{if(!await confirmBox(label,'Komut sıraya alınacak. Devam eden bir etkinlik etkilenebilir.','Sıraya al',danger))return;try{await api('/commands',{body:{command:'control',args}});toast('Komut sıraya alındı','ok')}catch(_){}}},label);
  main.append(head('Etkinlik Merkezi','Takvim ve elle kontrol'),
    h('div',{class:'hint',style:'margin-bottom:14px'},'Takvim veritabanına yazılır ve çalışan sunucuya yeniden yükleme komutu gider. Otomatik takvimi olmayan etkinlikler (Ultima, Golem Savaşı, Full Moon Rift…) oyun içi GM komutlarıyla yönetilir.'),
    h('div',{class:'card'},h('div',{class:'tw'},h('table',{},h('thead',{},h('tr',{},['ID','Etkinlik','Durum','Saatler','Günler','Seviye',''].map(x=>h('th',{},x)))),
      h('tbody',{},d.schedule.map(e=>h('tr',{},h('td',{},e.id),h('td',{},h('b',{},e.name||('Tür '+e.type))),h('td',{},e.status?h('span',{class:'chip ok'},'açık'):h('span',{class:'chip bad'},'kapalı')),
        h('td',{},e.times.filter(t=>t[0]>=0).map(t=>String(t[0]).padStart(2,'0')+':'+String(t[1]).padStart(2,'0')).join(' · ')||'—'),h('td',{},e.days.map((v,i)=>v?DAYS[i]:null).filter(Boolean).join(' ')||'—'),h('td',{},e.min_level+'–'+e.max_level),
        h('td',{},h('button',{class:'btn sm',onclick:()=>eventDialog(e)},'Düzenle')))))))),
    h('div',{class:'card',style:'margin-top:14px'},h('h3',{},'Elle kontrol (sunucu çevrimiçi olmalı)'),h('div',{class:'row'},ctl('BDW başlat',{kind:'event',action:'start',zone:0}),ctl('Chaos başlat',{kind:'event',action:'start',zone:1}),ctl('Juraid başlat',{kind:'event',action:'start',zone:2}),ctl('Etkinliği kapat',{kind:'event',action:'stop'},true),ctl('Savaş durdur',{kind:'war',action:'stop'},true))),
    h('div',{class:'card',style:'margin-top:14px'},h('h3',{},'Son komutlar'),h('div',{id:'cmds',class:'tw'},'…')));
  const cm=await api('/commands');
  clear($('#cmds')).append(h('table',{},h('tbody',{},cm.map(c=>h('tr',{},h('td',{class:'xs muted nowrap'},when(c.created_at)),h('td',{},c.command),h('td',{class:'xs muted'},JSON.stringify(c.args)),h('td',{},h('span',{class:'chip '+(c.status==='done'?'ok':c.status==='failed'?'bad':'')},c.status)),h('td',{class:'xs muted'},c.result))))));
};
function eventDialog(e){
  const st=h('select',{},h('option',{value:'1'},'Açık'),h('option',{value:'0'},'Kapalı'));st.value=e.status;
  const times=e.times.map(t=>h('input',{type:'time',value:t[0]>=0?String(t[0]).padStart(2,'0')+':'+String(t[1]).padStart(2,'0'):''}));
  const days=e.days.map((v,i)=>h('label',{class:'row',style:'gap:5px'},h('input',{type:'checkbox',checked:!!v}),DAYS[i]));
  const num=v=>h('input',{type:'number',value:v,min:'0'});const minL=num(e.min_level),maxL=num(e.max_level),loy=num(e.req_loyalty),mon=num(e.req_money),why=h('input',{placeholder:'Sebep (zorunlu)'});
  modal(e.name||('Etkinlik '+e.id),h('div',{class:'col'},h('div',{class:'form'},field('Takvim',st),field('Min seviye',minL),field('Max seviye',maxL),field('Gerekli loyalty',loy),field('Giriş ücreti',mon)),
    h('h3',{},'Başlangıç saatleri (sunucu saati)'),h('div',{class:'row'},times),h('h3',{},'Günler'),h('div',{class:'row'},days),field('Sebep',why)),[
    {label:'İptal',cls:'ghost'},{label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
      const T=times.map(i=>{if(!i.value)return[-1,-1];const [hh,mm]=i.value.split(':').map(Number);return[hh,mm]});
      try{const d=await api('/events/'+e.id,{body:{status:Number(st.value),times:T,days:days.map(l=>l.querySelector('input').checked?1:0),min_level:Number(minL.value),max_level:Number(maxL.value),req_loyalty:Number(loy.value),req_money:Number(mon.value),reason:why.value}});toast(d.note,'ok');close();route()}catch(_){}return true}}],'wide');
}

/* letters */
PAGE.letters=async function(main){
  const f={recipient:h('input'),item_id:h('input',{type:'number',value:'0'}),count:h('input',{type:'number',value:'1'}),durability:h('input',{type:'number',value:'0'}),sender:h('input',{value:'Yönetim'}),subject:h('input',{value:'Ödül'}),message:h('input',{value:'İyi oyunlar!'})};
  const names={recipient:'Alıcı karakter',item_id:'Item ID',count:'Adet',durability:'Dayanıklılık',sender:'Gönderen',subject:'Konu',message:'Mesaj'};
  const lib=libraryPanel({onPick:it=>{f.item_id.value=it.id}});
  main.append(head('Mektup / Ödül','Mektup sistemiyle item gönder'),h('div',{class:'grid g2'},
    h('div',{class:'card'},h('div',{class:'form'},Object.keys(f).map(k=>field(names[k],f[k],k==='message'))),h('div',{class:'hint',style:'margin:14px 0'},'Oyuncu çevrimiçi olsa da olmasa da mektubu alır.'),
      h('button',{class:'btn primary',onclick:async()=>{try{await api('/letters/item',{body:{recipient:f.recipient.value.trim(),item_id:Number(f.item_id.value),count:Number(f.count.value),durability:Number(f.durability.value),sender:f.sender.value,subject:f.subject.value,message:f.message.value}});toast('Mektup gönderildi','ok')}catch(_){}}},ico('mail'),'Gönder')),
    h('div',{class:'card'},lib.el)));
};

/* system */
PAGE.system=async function(main){
  const users=await api('/admin-users').catch(()=>[]);
  const opts=await api('/game-options').catch(()=>null);
  const msg=h('input',{placeholder:'Duyuru metni (en fazla 200 karakter)',style:'flex:1;min-width:220px'}),nu=h('input',{placeholder:'Kullanıcı adı'}),np=h('input',{type:'password',placeholder:'Parola (en az 12 karakter)'});
  const rl=(l,w)=>h('button',{class:'btn',onclick:async()=>{try{await api('/commands',{body:{command:'reload',args:{what:w}}});toast(l+' için komut sıraya alındı','ok')}catch(_){}}},l);
  main.append(head('Sistem & Yöneticiler','Çalışan sunucuya komut gönder, panel yöneticilerini yönet'),h('div',{class:'grid g2'},
    opts?h('div',{class:'card'},h('h3',{},'Oyun seçenekleri'),
      h('label',{class:'row',style:'gap:8px;cursor:pointer'},h('input',{type:'checkbox',checked:opts.auto_register,onchange:async e=>{try{await api('/game-options',{body:{auto_register:e.target.checked}});toast(e.target.checked?'Oyun içi otomatik kayıt açık':'Oyun içi otomatik kayıt kapalı — kayıt sadece web panelden olur','ok')}catch(_){e.target.checked=!e.target.checked}}}),'Oyun içinden otomatik kayıt (giriş ekranında yeni ID/parola yazınca hesap açılsın)'),
      h('div',{class:'hint',style:'margin-top:8px'},'Kapatınca giriş ekranında tanımsız bir hesapla girilen ID/parola reddedilir; kayıt sadece web panelden yapılabilir. Anında etkili, sunucu yeniden başlatmaya gerek yok.')):null,
    h('div',{class:'card'},h('h3',{},'Çalışan sunucuya komut'),h('div',{class:'row'},rl('Hepsini yeniden yükle','all'),rl('Drop tabloları','monster_drop_tables'),rl('Item grupları','make_item_groups'),rl('Quest tabloları','quest_tables'),rl('Etkinlik takvimi','event_schedule')),
      h('div',{class:'row',style:'margin-top:14px'},msg,h('button',{class:'btn primary',onclick:async()=>{if(!msg.value.trim())return;try{await api('/commands',{body:{command:'announce',args:{message:msg.value.trim()}}});toast('Duyuru sıraya alındı','ok');msg.value=''}catch(_){}}},'Duyur'))),
    h('div',{class:'card'},h('h3',{},'Panel yöneticileri'),h('table',{},h('tbody',{},users.map(u=>h('tr',{},h('td',{},h('b',{},u.username)),h('td',{class:'xs muted'},when(u.created_at)),h('td',{},h('button',{class:'btn sm danger',onclick:async()=>{if(!await confirmBox('Yönetici sil',u.username+' silinecek.','Sil',true))return;try{await api('/admin-users/'+encodeURIComponent(u.username),{method:'DELETE'});toast('Silindi','ok');route()}catch(_){}}},ico('trash'))))))),
      h('div',{class:'row',style:'margin-top:12px'},nu,np,h('button',{class:'btn primary',onclick:async()=>{try{await api('/admin-users',{body:{username:nu.value.trim(),password:np.value}});toast('Yönetici eklendi','ok');route()}catch(_){}}},'Ekle')),
      h('div',{class:'hint',style:'margin-top:12px'},'Rol bazlı yetki henüz yok: tüm yöneticiler tam yetkilidir. Web paneli yöneticisi ile oyun içi GM yetkisi ayrı kavramlardır.'))));
};

/* audit */
PAGE.audit=async function(main){
  const out=h('div',{class:'tw'});const sb=searchBar('Yönetici, işlem veya hedef…',()=>run());
  main.append(head('Denetim Kaydı','Her değişiklik kimin, ne zaman, nereden yaptığıyla kaydedilir'),h('div',{class:'row',style:'margin-bottom:12px'},sb.el),h('div',{class:'card'},out));
  async function run(){
    const rows=await api('/audit?limit=300&q='+encodeURIComponent(sb.input.value.trim()));
    clear(out).append(h('table',{},h('thead',{},h('tr',{},['Zaman','Yönetici','İşlem','Hedef','Ayrıntı','IP'].map(x=>h('th',{},x)))),
      h('tbody',{},rows.map(r=>h('tr',{},h('td',{class:'xs muted nowrap'},when(r.occurred_at)),h('td',{},r.actor),h('td',{},h('span',{class:'chip blue'},r.action)),h('td',{},r.target),h('td',{class:'xs muted',style:'max-width:380px;overflow-wrap:anywhere'},JSON.stringify(r.details)),h('td',{class:'xs muted'},r.ip||''))))));
  }
  run();
};
