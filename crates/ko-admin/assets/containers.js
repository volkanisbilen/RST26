'use strict';
/* ───────── item containers: character bags, banks, clan bank ───────── */
const EQUIP=[[0,'Sağ küpe'],[1,'Başlık'],[2,'Sol küpe'],[3,'Kolye'],[4,'Göğüs'],[5,'Omuz'],[6,'Sağ el'],[7,'Kemer'],[8,'Sol el'],[9,'Sağ yüzük'],[10,'Bacak'],[11,'Sol yüzük'],[12,'Eldiven'],[13,'Ayakkabı'],null];
const range=(a,b)=>Array.from({length:b-a},(_,k)=>a+k);

async function loadContainer(type,owner,tab){
  if(owner==null)return null;
  const d=await api('/container?type='+type+'&owner='+encodeURIComponent(owner)+'&tab='+tab);
  const map=new Map();for(const s of d.slots)map.set(s.slot,s);
  return {type,owner:d.owner,tab:d.tab,online:d.online,capacity:d.capacity,extra:d.extra,map,picked:null,moveFrom:null};
}
function slotEl(ctx,idx,sec,label,reload){
  const d=ctx.map.get(idx);
  const el=h('div',{class:'slot '+(d?'full':'empty')+(label?' eqlabel':''),draggable:d?'true':null,'data-label':label||'',
    dataset:{container:ctx.type,slotIndex:idx,tab:ctx.tab}},
    d?h('img',{src:icon(d.icon),alt:'',loading:'lazy',onerror:iconErr}):null,
    d&&d.count>1?h('span',{class:'cnt'},d.count):null,d&&d.flag?h('span',{class:'fl'},'★'):null,h('span',{class:'num'},idx));
  const note=sec+' · DB '+idx+(ctx.type==='clan'?' · sekme '+ctx.tab:'');
  if(d)attachTip(el,()=>({id:d.id,slot:d,note}));else el.title=sec+' — boş (DB '+idx+')';
  if(ctx.moveFrom&&ctx.moveFrom.slot===idx&&ctx.moveFrom.tab===ctx.tab)el.classList.add('sel');
  el.addEventListener('dragstart',e=>{el.classList.add('dragging');e.dataTransfer.setData('text/plain',JSON.stringify({from:idx,tab:ctx.tab,type:ctx.type}));e.dataTransfer.effectAllowed='move'});
  el.addEventListener('dragend',()=>el.classList.remove('dragging'));
  el.addEventListener('dragover',e=>{e.preventDefault();el.classList.add('over')});
  el.addEventListener('dragleave',()=>el.classList.remove('over'));
  el.addEventListener('drop',async e=>{
    e.preventDefault();el.classList.remove('over');
    let p;try{p=JSON.parse(e.dataTransfer.getData('text/plain'))}catch(_){return}
    if(p.pick){openSlot(ctx,idx,sec,reload,p.pick);return}
    if(p.type!==ctx.type||(p.from===idx&&p.tab===ctx.tab))return;
    await doMove(ctx,p.tab,p.from,idx,reload);
  });
  el.addEventListener('click',async()=>{
    if(ctx.moveFrom){const f=ctx.moveFrom;ctx.moveFrom=null;if(f.slot!==idx||f.tab!==ctx.tab)await doMove(ctx,f.tab,f.slot,idx,reload);else reload();return}
    openSlot(ctx,idx,sec,reload,ctx.picked);
  });
  return el;
}
async function doMove(ctx,fromTab,from,to,reload){
  try{await api('/container/move',{body:{type:ctx.type,owner:ctx.owner,tab:fromTab,from,to,to_tab:ctx.tab}});toast('Taşındı','ok')}catch(_){}
  reload();
}
async function openSlot(ctx,idx,sec,reload,pick){
  const d=ctx.map.get(idx);
  const cur=pick?{id:pick.id,count:1,durability:0,flag:0,expire:0}:(d||{id:0,count:1,durability:0,flag:0,expire:0});
  const id=h('input',{type:'number',value:cur.id,min:'0'});
  const cnt=h('input',{type:'number',value:cur.count||1,min:'0'});
  const dur=h('input',{type:'number',value:cur.durability||0});
  const flg=h('input',{type:'number',value:cur.flag||0});
  const exp=h('input',{type:'number',value:cur.expire||0});
  const why=h('input',{placeholder:'Sebep (silme için zorunlu)'});
  const card=h('div',{style:'min-height:120px'});
  async function showCard(){
    clear(card);const v=Number(id.value);if(!v)return card.append(h('div',{class:'empty'},'Boş yuva'));
    const it=await getItem(v);
    card.append(it?h('div',{class:'row',style:'align-items:flex-start;flex-wrap:nowrap'},h('div',{class:'slot full',style:'width:64px;height:64px;flex:none'},h('img',{src:icon(it.item_icon_id1),onerror:iconErr})),
      h('div',{style:'min-width:0'},renderTip(it,Number(id.value)===cur.id?d:null))):h('div',{class:'hint warn'},'Bu ID item tablosunda yok.'));
  }
  id.addEventListener('change',showCard);showCard();
  modal('Yuva '+idx+' · '+sec,h('div',{class:'grid g2',style:'gap:16px'},
    h('div',{},card,h('p',{class:'muted sm'},'DB indeksi '+idx+(ctx.type==='clan'?' · sekme '+ctx.tab:'')+(ctx.online?' · OYUNCU ÇEVRİMİÇİ — kayıt reddedilir':''))),
    h('div',{class:'form'},field('Item ID (0 = boşalt)',id),field('Adet',cnt),field('Dayanıklılık (0 = varsayılan)',dur),field('Flag',flg),field('Süre (expire)',exp),field('Sebep',why,true))),[
    {label:'Taşı…',cls:'ghost',fn:()=>{ctx.moveFrom={slot:idx,tab:ctx.tab};toast('Hedef yuvaya tıklayın');reload()}},
    d?{label:'Sil',cls:'danger',keep:true,fn:async close=>{
      if(why.value.trim().length<3){toast('Silmek için sebep yazın','err');return true}
      try{await api('/container/set',{body:{type:ctx.type,owner:ctx.owner,tab:ctx.tab,slot:idx,id:0,reason:why.value}});toast('Silindi','ok');close();ctx.picked=null;reload()}catch(_){}return true}}:null,
    {label:'İptal',cls:'ghost'},
    {label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
      try{await api('/container/set',{body:{type:ctx.type,owner:ctx.owner,tab:ctx.tab,slot:idx,id:Number(id.value),count:Number(cnt.value),durability:Number(dur.value),flag:Number(flg.value),expire:Number(exp.value),reason:why.value}});
        toast('Kaydedildi','ok');close();ctx.picked=null;reload()}catch(_){}return true}}
  ].filter(Boolean),'wide');
}
const grid=(cls,cells)=>h('div',{class:'slots '+cls},cells);

/* generic item library used by container pages, drop editor and group manager */
function libraryPanel(opts){
  const {onPick,groups=false,compact=false}=opts;
  const q=h('input',{placeholder:groups?'Item adı / ID veya grup no…':'Item ara: ad veya tam ID',style:'width:100%'});
  const res=h('div',{class:'res'});const note=h('p',{class:'muted sm'},'Sürükleyip bırakın veya tıklayın.');
  let mode='item',kind='';
  const KINDS=[['','Tümü'],['21','Kılıç'],['22','2 el kılıç'],['31','Balta'],['41','Gürz'],['51','Mızrak'],['70','Yay'],['60','Kalkan'],['91','Küpe'],['92','Kolye'],['93','Yüzük'],['94','Kemer'],['97','Tüketilebilir']];
  const chips=h('div',{class:'chips'},KINDS.map(([k,l])=>h('button',{class:k===''?'on':'',onclick:function(){kind=k;$$('button',chips).forEach(b=>b.classList.toggle('on',b===this));run()}},l)));
  const tabs=groups?h('div',{class:'seg libtabs'},[['item','İtemler'],['group','Gruplar']].map(([k,l])=>h('button',{class:k==='item'?'on':'',onclick:function(){mode=k;$$('button',tabs).forEach(b=>b.classList.toggle('on',b===this));chips.style.display=k==='item'?'':'none';run()}},l))):null;
  async function run(){
    const t=q.value.trim();clear(res);
    if(mode==='group'){
      const gs=await api('/drop-groups?q='+encodeURIComponent(t||'1')+'&limit=40',{quiet:true}).catch(()=>[]);
      res.style.display='block';
      gs.forEach(g=>{
        const row=h('div',{class:'mrow',draggable:'true',title:g.examples},h('div',{class:'slot full',style:'width:34px;height:34px'},h('img',{src:icon(g.icon),onerror:iconErr})),h('div',{},h('b',{},'Grup '+g.group_num),h('br'),h('small',{},g.item_count+' item · '+g.examples.slice(0,40))));
        const data={id:g.group_num,name:'Grup '+g.group_num,icon:g.icon,group:true};
        row.addEventListener('dragstart',e=>e.dataTransfer.setData('text/plain',JSON.stringify({pick:data})));
        row.addEventListener('click',()=>onPick&&onPick(data));res.append(row)});
      if(!gs.length)res.append(h('span',{class:'muted sm'},'Grup bulunamadı'));return;
    }
    res.style.display='';
    if(!t&&!kind)return;
    const items=await api('/item-search?q='+encodeURIComponent(t)+(kind?'&kind='+kind:'')+'&limit=120',{quiet:true}).catch(()=>[]);
    items.forEach(it=>{
      const s=itemSlot(it);
      s.addEventListener('dragstart',e=>e.dataTransfer.setData('text/plain',JSON.stringify({pick:it})));
      s.addEventListener('click',()=>onPick&&onPick(it));res.append(s)});
    if(!items.length)res.append(h('span',{class:'muted sm'},'Sonuç yok'));
  }
  q.addEventListener('input',debounce(run,250));
  return {el:h('div',{},h('h3',{style:'margin-bottom:8px'},'Item kütüphanesi'),tabs,q,groups?null:null,chips,note,res),run,input:q};
}
function pickerEl(ctx){
  const banner=h('div');
  const lib=libraryPanel({onPick:it=>{ctx.picked=it;clear(banner).append(h('div',{class:'banner'},ico('plus'),h('span',{},'Seçili: '+it.name+' #'+it.id),h('button',{class:'btn sm ghost',onclick:()=>{ctx.picked=null;clear(banner)}},'×')))}});
  return h('div',{},banner,lib.el);
}
function containerView(ctx){
  const wrap=h('div');
  async function reload(){
    const keepPick=ctx.picked,keepMove=ctx.moveFrom,tab=ctx.tab;
    const fresh=await loadContainer(ctx.type,ctx.owner,tab);
    Object.assign(ctx,{map:fresh.map,online:fresh.online,extra:fresh.extra,tab:fresh.tab,picked:keepPick,moveFrom:keepMove});draw();
  }
  function draw(){
    clear(wrap);
    if(ctx.online)wrap.append(h('div',{class:'hint warn',style:'margin-bottom:10px'},'Bu oyuncu çevrimiçi göründüğü için düzenleme kilitli. Görüntüleme serbest.'));
    if(ctx.moveFrom)wrap.append(h('div',{class:'banner'},'Taşıma modu: hedef yuvaya tıklayın.',h('button',{class:'btn sm',onclick:()=>{ctx.moveFrom=null;draw()}},'Vazgeç')));
    const r=()=>reload();
    if(ctx.type==='inv'){
      const S=(i,sec,label)=>slotEl(ctx,i,sec,label,r);
      const mb=ctx._mb||0;
      wrap.append(h('div',{class:'invwrap'},
        h('div',{class:'panel'},h('h4',{},'Kostüm (42–52) & Magic Bag'),
          grid('b3',[...range(42,53).map(i=>S(i,'Cospre')),h('div',{class:'slot none'})]),
          h('div',{class:'seg',style:'margin:12px 0 8px'},[0,1,2].map(k=>h('button',{class:mb===k?'on':'',onclick:()=>{ctx._mb=k;draw()}},'Bag '+(k+1)))),
          grid('b3',range(53+mb*12,65+mb*12).map(i=>S(i,'Magic Bag '+(mb+1))))),
        h('div',{class:'panel'},h('h4',{},'Envanter'),
          h('p',{class:'xs faint',style:'margin:0 0 6px'},'Ekipman 0–13'),grid('eq',EQUIP.map(e=>e?S(e[0],'Ekipman: '+e[1],e[1]):h('div',{class:'slot none'}))),
          h('p',{class:'xs faint',style:'margin:12px 0 6px'},'Ana çanta 14–41'),grid('b7',range(14,42).map(i=>S(i,'Ana çanta'))),
          h('p',{class:'xs faint',style:'margin:12px 0 6px'},'Knight Royale 89–95'),grid('b7',range(89,96).map(i=>S(i,'Knight Royale'))))));
    }else{
      const per=24,pages=ctx.type==='wh'?8:ctx.type==='vip'?2:Math.max(1,ctx.extra.max_tabs||1);
      const page=ctx.type==='clan'?ctx.tab:(ctx._page||0);
      const tabsEl=h('div',{class:'seg',style:'margin-bottom:10px'},Array.from({length:pages},(_,k)=>h('button',{class:page===k?'on':'',onclick:async()=>{
        if(ctx.type==='clan'){ctx.tab=k;await reload()}else{ctx._page=k;draw()}}},(ctx.type==='clan'?'Sekme ':'Sayfa ')+(k+1))));
      const base=ctx.type==='clan'?0:page*per;
      const info=ctx.type==='wh'?'Banka altını: '+fmt(ctx.extra.coins)+' · Hesaba ait: aynı hesaptaki tüm karakterler paylaşır.':
        ctx.type==='vip'?'VIP kasa kalıcı 48 yuva (0–47). Süre sonu: '+(ctx.extra.vault_expiry||'—'):
        'Klan #'+ctx.owner+' · Banka altını: '+fmt(ctx.extra.gold)+' · Açık sekme: '+(ctx.extra.max_tabs||1);
      wrap.append(h('div',{class:'panel',style:'display:inline-block;max-width:100%'},h('h4',{},ctx.type==='wh'?'Normal Banka (0–191)':ctx.type==='vip'?'VIP Kasa (0–47)':'Klan Bankası'),tabsEl,
        h('p',{class:'muted sm',style:'margin:0 0 10px'},info),grid('b6',Array.from({length:per},(_,k)=>slotEl(ctx,base+k,(ctx.type==='clan'?'Sekme '+(ctx.tab+1)+', yuva ':'Sayfa '+(page+1)+', yuva ')+(k+1),null,r)))));
    }
  }
  draw();
  return h('div',{class:'invwrap'},h('div',{style:'flex:1 1 560px;min-width:0'},wrap),h('div',{class:'panel',style:'flex:0 1 310px;width:310px;max-width:100%'},pickerEl(ctx)));
}

/* ───────── character page ───────── */
PAGE.char=async function(main,[name,tab]){
  const c=await api('/characters/'+encodeURIComponent(name));
  tab=tab||'inv';
  const clan=await api('/characters/'+encodeURIComponent(name)+'/clan',{quiet:true}).catch(()=>({}));
  main.append(
    h('div',{class:'row',style:'margin-bottom:14px'},h('a',{href:'#/players',class:'btn ghost'},'← Oyuncular'),
      h('h1',{},c.name),nationChip(c.nation),h('span',{class:'chip blue'},className(c.class)),h('span',{class:'chip'},'Lv '+c.level+(c.rebirth?' · +'+c.rebirth:'')),
      clan.id?h('a',{class:'chip',href:'#/clan/'+clan.id},'Klan: '+clan.name):null,
      c.online?h('span',{class:'chip bad'},'ÇEVRİMİÇİ — düzenleme kilitli'):h('span',{class:'chip'},'çevrimdışı'),
      h('span',{class:'grow'}),
      c.online?h('button',{class:'btn danger',onclick:async()=>{if(!await confirmBox('Çevrimiçi kaydı temizle','Oyuncu atılacak / kayıt silinecek.','Temizle',true))return;
        const d=await api('/clear-online/'+encodeURIComponent(c.name),{body:{}});toast(d.queued?'Atma komutu sıraya alındı':'Kayıt temizlendi','ok');setTimeout(route,1500)}},'Çevrimiçi kaydını temizle'):null),
    h('div',{class:'seg',style:'margin-bottom:14px'},[['inv','Envanter'],['wh','Normal Banka'],['vip','VIP Kasa'],['clan','Klan Bankası'],['stats','Karakter Bilgileri']].map(([k,l])=>
      h('button',{class:tab===k?'on':'',onclick:()=>location.hash='#/char/'+encodeURIComponent(c.name)+'/'+k},l))));
  const body=h('div');main.append(body);
  if(tab==='stats')return charStats(body,c);
  const owner=tab==='clan'?(clan.id?String(clan.id):null):c.name;
  const ctx=await loadContainer(tab,owner,0);
  if(!ctx)return body.append(h('div',{class:'card empty'},'Bu karakter bir klanda değil.'));
  body.append(containerView(ctx));
};
function charStats(body,c){
  const F=[['level','Seviye'],['rebirth','Rebirth'],['exp','EXP'],['loyalty','Loyalty (NP)'],['gold','Noah'],['hp','HP'],['mp','MP'],['nation','Ulus (1 Karus, 2 El Morad)'],['class','Sınıf kodu'],['race','Irk'],['authority','Karakter yetkisi (0/2 GM, 1 normal)'],['zone','Bölge'],['px','Konum X ×100'],['pz','Konum Z ×100']];
  const inputs={};
  body.append(h('div',{class:'card'},h('h3',{},'Karakter bilgileri'),
    h('div',{class:'form'},F.map(([k,l])=>{inputs[k]=h('input',{type:'number',value:c[k]??0,disabled:c.online});return field(l,inputs[k])})),
    h('p',{class:'hint',style:'margin:14px 0'},'Hesap yetkisi (yasak/GM hesabı) ile karakter yetkisi farklı alanlardır; burada yalnızca userdata.authority değişir. Web paneli yöneticiliği oyun içi GM yetkisinden bağımsızdır.'),
    h('button',{class:'btn primary',disabled:c.online,onclick:async()=>{
      const p={};for(const [k] of F)if(Number(inputs[k].value)!==Number(c[k]))p[k]=Number(inputs[k].value);
      if(!Object.keys(p).length)return toast('Değişiklik yok');
      try{await api('/characters/'+encodeURIComponent(c.name),{body:p});toast('Kaydedildi','ok');route()}catch(_){}}},'Kaydet')));
}
