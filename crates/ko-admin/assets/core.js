'use strict';
/* ───────── core: DOM helpers, api, shell, tooltip, palette ───────── */
const BASE='/jstkoadminpanel',API=BASE+'/api';
let csrf='',actor='';
const $=(s,r=document)=>r.querySelector(s);
const $$=(s,r=document)=>[...r.querySelectorAll(s)];
function h(tag,props,...kids){
  const e=document.createElement(tag);
  for(const [k,v] of Object.entries(props||{})){
    if(v===false||v==null)continue;
    if(k==='class')e.className=v;
    else if(k==='style')e.style.cssText=v;
    else if(k==='dataset')Object.assign(e.dataset,v);
    else if(k.startsWith('on'))e.addEventListener(k.slice(2),v);
    else if(k==='value'||k==='checked'||k==='disabled'||k==='selected')e[k]=v;
    else e.setAttribute(k,v===true?'':v);
  }
  for(const c of kids.flat(Infinity)){if(c==null||c===false)continue;e.append(c.nodeType?c:document.createTextNode(String(c)))}
  return e;
}
const clear=e=>{while(e.firstChild)e.removeChild(e.firstChild);return e};
const fmt=n=>Number(n||0).toLocaleString('tr-TR');
const when=s=>s?new Date(s).toLocaleString('tr-TR'):'—';
const debounce=(fn,ms=250)=>{let t;return(...a)=>{clearTimeout(t);t=setTimeout(()=>fn(...a),ms)}};
const esc=s=>String(s??'');
const icon=id=>BASE+'/icons/'+(id||0)+'.jpg';
const iconErr=function(){this.onerror=null;this.src=icon(0)};

/* icons */
const ICO={
  home:'M3 11l9-8 9 8M5 10v10h5v-6h4v6h5V10',users:'M16 11a4 4 0 1 0-8 0 4 4 0 0 0 8 0M4 21c0-4 3.6-6 8-6s8 2 8 6',
  shield:'M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z',bag:'M6 8h12l1 12H5zM9 8a3 3 0 0 1 6 0',box:'M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10',
  sword:'M14 4l6-1-1 6-9 9-3 1 1-3zM5 19l-2 2',scroll:'M7 4h11v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-2h3zM7 4a2 2 0 0 0-2 2v10M10 9h5M10 13h5',
  cal:'M5 5h14v15H5zM5 10h14M9 3v4M15 3v4',flag:'M5 21V4M5 4h12l-2 4 2 4H5',map:'M9 4L3 6v14l6-2 6 2 6-2V4l-6 2zM9 4v14M15 6v14',
  mail:'M3 6h18v12H3zM3 7l9 6 9-6',cog:'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6M19 12l2-1-1-3-2 .5-1.5-1.5L17 5l-3-1-1 2h-2L10 4 7 5l.5 2.5L6 9l-2-.5-1 3 2 1v2l-2 1 1 3 2-.5 1.5 1.5L7 21l3 1 1-2h2l1 2 3-1-.5-2.5L18 17l2 .5 1-3-2-1z',
  list:'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',search:'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14M21 21l-5-5',
  clan:'M3 20v-2a4 4 0 0 1 4-4h2a4 4 0 0 1 4 4v2M9 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6M17 11a3 3 0 1 0 0-6M21 20v-2a4 4 0 0 0-3-3.8',
  layers:'M12 3l9 5-9 5-9-5zM3 13l9 5 9-5',menu:'M4 6h16M4 12h16M4 18h16',out:'M10 4H5v16h5M15 8l5 4-5 4M20 12H9',
  db:'M4 6c0-2 16-2 16 0s-16 2-16 0v12c0 2 16 2 16 0V6M4 12c0 2 16 2 16 0',trash:'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13',plus:'M12 5v14M5 12h14',
  pin:'M12 22s7-6 7-12a7 7 0 1 0-14 0c0 6 7 12 7 12M12 12a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5',gift:'M4 11h16v10H4zM3 7h18v4H3zM12 7v14M12 7S10 3 8 4s0 3 4 3zM12 7s2-4 4-3 0 3-4 3',
  copy:'M9 9h11v11H9zM5 15V4h11',undo:'M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3',x:'M6 6l12 12M18 6L6 18',zap:'M13 2L4 14h7l-1 8 9-12h-7z',
};
function ico(n,cls){const s=document.createElementNS('http://www.w3.org/2000/svg','svg');s.setAttribute('viewBox','0 0 24 24');const p=document.createElementNS('http://www.w3.org/2000/svg','path');p.setAttribute('d',ICO[n]||ICO.box);s.append(p);return h('span',{class:'ico '+(cls||'')},s)}

/* api / toast / modal */
function toast(msg,kind){const t=h('div',{class:'toast '+(kind||'')},msg);$('#toasts').append(t);setTimeout(()=>t.remove(),kind==='err'?7000:3600)}
async function api(path,opt={}){
  const method=opt.method||(opt.body!==undefined?'POST':'GET');
  const r=await fetch(API+path,{method,credentials:'same-origin',headers:{'Content-Type':'application/json','x-admin-csrf':csrf},body:opt.body!==undefined?JSON.stringify(opt.body):undefined});
  if(r.status===204)return null;
  let d=null;try{d=await r.json()}catch(_){}
  if(r.status===401){showLogin();throw new Error('auth')}
  if(!r.ok){const m=(d&&d.error)||('Hata '+r.status);if(!opt.quiet)toast(m,'err');const e=new Error(m);e.status=r.status;throw e}
  return d;
}
function modal(title,body,buttons,size){
  const ov=h('div',{class:'overlay'});
  const close=()=>{ov.remove();document.removeEventListener('keydown',esc_)};
  const esc_=e=>{if(e.key==='Escape')close()};
  document.addEventListener('keydown',esc_);
  const m=h('div',{class:'modal '+(size||''),role:'dialog','aria-label':title},h('h2',{},title),body,
    buttons&&buttons.length?h('div',{class:'btns'},buttons.map(b=>h('button',{class:'btn '+(b.cls||''),onclick:async()=>{
      if(b.fn){const keep=await b.fn(close);if(keep===true)return}
      if(!b.keep)close()}},b.label))):null);
  ov.addEventListener('mousedown',e=>{if(e.target===ov)close()});
  ov.append(m);document.body.append(ov);
  const f=m.querySelector('input:not([type=checkbox]),select');if(f&&!('ontouchstart' in window))f.focus();
  return close;
}
const confirmBox=(title,text,okLabel='Onayla',danger=false)=>new Promise(res=>{
  modal(title,h('p',{class:'muted'},text),[{label:'Vazgeç',cls:'ghost',fn:()=>res(false)},{label:okLabel,cls:danger?'danger':'primary',fn:()=>res(true)}]);
});

/* ───────── item knowledge + game-style tooltip ───────── */
const KIND={11:'Hançer',21:'Tek el kılıç',22:'Çift el kılıç',31:'Tek el balta',32:'Çift el balta',41:'Tek el gürz',42:'Çift el gürz',51:'Tek el mızrak',52:'Çift el mızrak',60:'Kalkan',70:'Yay',71:'Tatar yayı',91:'Küpe',92:'Kolye',93:'Yüzük',94:'Kemer',95:'Eşya',97:'Tüketilebilir',110:'Asa',120:'Pazubent',130:'Zırh parçası',140:'Pet',151:'Pet',252:'Cospre',255:'Unique'};
const itemCache=new Map();
function getItem(id){
  if(!id)return Promise.resolve(null);
  if(!itemCache.has(id))itemCache.set(id,api('/item/'+id,{quiet:true}).catch(()=>null));
  return itemCache.get(id);
}
function tipLines(it,slot){
  const n=(k)=>Number(it[k]||0);const L=[];
  const line=(a,b,cls)=>L.push(h('div',{class:'ln '+(cls||'')},h('span',{},a),h('span',{},b)));
  const kindName=KIND[it.kind]||('Tür '+it.kind);
  if(n('damage')||n('min_damage')||n('max_damage'))line('Saldırı gücü',(n('min_damage')||n('damage'))+(n('max_damage')?' – '+n('max_damage'):''));
  if(n('delay'))line('Saldırı hızı',n('delay'));
  if(n('range'))line('Menzil',n('range'));
  if(n('ac'))line('Savunma',n('ac'));
  if(n('hitrate'))line('İsabet',n('hitrate'));
  if(n('evasionrate'))line('Kaçınma',n('evasionrate'));
  line('Ağırlık',(n('weight')/10).toFixed(1));
  if(n('duration')||slot?.durability){line('Dayanıklılık',(slot&&slot.durability?slot.durability+' / ':'')+n('duration'))}
  const bon=[['str_b','STR'],['sta_b','STA'],['dex_b','DEX'],['intel_b','INT'],['cha_b','CHA'],['max_hp_b','Maks. HP'],['max_mp_b','Maks. MP']].filter(([k])=>n(k)).map(([k,l])=>h('div',{class:'bn'},l+' '+(n(k)>0?'+':'')+n(k)));
  const dmg=[['fire_damage','Ateş hasarı'],['ice_damage','Buz hasarı'],['lightning_damage','Şimşek hasarı'],['poison_damage','Zehir hasarı'],['hp_drain','HP çekme'],['mp_damage','MP hasarı'],['mp_drain','MP çekme'],['mirror_damage','Yansıtma']].filter(([k])=>n(k)).map(([k,l])=>h('div',{class:'bn'},l+' +'+n(k)));
  const res=[['fire_r','Ateş direnci'],['cold_r','Buz direnci'],['lightning_r','Şimşek direnci'],['magic_r','Büyü direnci'],['poison_r','Zehir direnci'],['curse_r','Lanet direnci']].filter(([k])=>n(k)).map(([k,l])=>h('div',{class:'bn'},l+' +'+n(k)));
  const ac2=[['dagger_ac','Hançer savunması'],['sword_ac','Kılıç savunması'],['club_ac','Gürz savunması'],['axe_ac','Balta savunması'],['spear_ac','Mızrak savunması'],['bow_ac','Yay savunması'],['jamadar_ac','Jamadar savunması']].filter(([k])=>n(k)).map(([k,l])=>h('div',{class:'bn'},l+' +'+n(k)));
  const rq=[];
  if(n('req_level'))rq.push(h('div',{class:'rq'},'Gerekli seviye: '+n('req_level')+(n('req_level_max')&&n('req_level_max')<255?' – '+n('req_level_max'):'')));
  for(const [k,l] of [['req_str','STR'],['req_sta','STA'],['req_dex','DEX'],['req_intel','INT'],['req_cha','CHA']])if(n(k))rq.push(h('div',{class:'rq'},'Gerekli '+l+': '+n(k)));
  if(n('class'))rq.push(h('div',{class:'rq'},'Sınıf kodu: '+n('class')));
  if(n('req_rank'))rq.push(h('div',{class:'rq'},'Gerekli rütbe: '+n('req_rank')));
  return {kindName,main:L,bonus:[...bon,...dmg,...res,...ac2],rq};
}
function renderTip(it,slot,slotInfo){
  const t=tipLines(it,slot);
  const grade=it.kind===255?'unique':(t.bonus.length?'magic':'');
  const box=h('div',{},
    h('div',{class:'nm '+grade},it.str_name||('#'+it.num)),
    h('div',{class:'ty'},t.kindName+(it.countable?' · Yığılabilir':'')),
    t.main,
    t.bonus.length?[h('hr'),t.bonus]:null,
    t.rq.length?[h('hr'),t.rq]:null,
    it.description?h('div',{class:'ds'},String(it.description).replace(/\|/g,'\n').replace(/<[^>]+>/g,'')):null,
    h('div',{class:'ft'},'#'+it.num+(slotInfo?' · '+slotInfo:'')+(slot&&slot.count>1?' · Adet '+slot.count:'')+(it.buy_price?' · Fiyat '+fmt(it.buy_price):'')));
  return box;
}
const tipEl=()=>$('#tip');
let tipToken=0;
function moveTip(e){
  const t=tipEl();if(t.style.display!=='block')return;
  const pad=16,w=t.offsetWidth,hh=t.offsetHeight;
  let x=e.clientX+pad,y=e.clientY+pad;
  if(x+w>innerWidth-8)x=e.clientX-w-pad;if(y+hh>innerHeight-8)y=Math.max(8,innerHeight-hh-8);if(x<8)x=8;
  t.style.left=x+'px';t.style.top=y+'px';
}
function attachTip(el,getInfo){
  el.addEventListener('mouseenter',async e=>{
    const info=getInfo();if(!info||!info.id)return;
    const my=++tipToken;const t=tipEl();clear(t).append(h('div',{class:'muted'},'Yükleniyor…'));t.style.display='block';moveTip(e);
    const it=await getItem(info.id);
    if(my!==tipToken)return;
    clear(t).append(it?renderTip(it,info.slot,info.note):h('div',{class:'muted'},'Item #'+info.id+' (tabloda yok)'));moveTip(e);
  });
  el.addEventListener('mousemove',moveTip);
  el.addEventListener('mouseleave',()=>{tipToken++;tipEl().style.display='none'});
  el.addEventListener('dragstart',()=>{tipToken++;tipEl().style.display='none'});
}
/* an item "chip" used everywhere (library, groups, drops) */
function itemSlot(it,opts={}){
  const el=h('div',{class:'slot full'+(opts.cls?' '+opts.cls:''),draggable:opts.drag===false?null:'true'},
    h('img',{src:icon(it.icon),loading:'lazy',alt:'',onerror:iconErr}),opts.count>1?h('span',{class:'cnt'},opts.count):null);
  attachTip(el,()=>({id:it.id||it.item_id,note:opts.note}));
  return el;
}

/* ───────── shell, nav, routing ───────── */
const NAV=[
  ['Genel',[['dashboard','Genel Bakış','home'],['players','Hesaplar & Oyuncular','users'],['clans','Klanlar','clan']]],
  ['Oyun İçeriği',[['items','Item Veritabanı','box'],['drops','Drop Editörü','sword'],['groups','Grup Yöneticisi','layers'],['maps','Harita Editörü','map'],['quest','Quest Stüdyosu','scroll'],['start','Başlangıç Ayarları','pin']]],
  ['Etkinlikler',[['events','Etkinlik Merkezi','cal'],['race','Collection Race','flag'],['letters','Mektup / Ödül','mail']]],
  ['Sistem',[['system','Sistem & Yöneticiler','cog'],['audit','Denetim Kaydı','list']]],
];
const PAGE={};const PAGE_TITLE={};
for(const [,items] of NAV)for(const [id,label] of items)PAGE_TITLE[id]=label;
function buildNav(){
  const host=clear($('#navhost'));
  const logo=h('span',{});logo.innerHTML='<svg viewBox="0 0 32 32" width="34" height="34"><defs><linearGradient id="g2" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#60a5fa"/><stop offset="1" stop-color="#22d3ee"/></linearGradient></defs><path d="M16 2 28 8v9c0 7-5 11-12 13C9 28 4 24 4 17V8z" fill="none" stroke="url(#g2)" stroke-width="2.2" stroke-linejoin="round"/><path d="M11 15l4 4 7-8" fill="none" stroke="url(#g2)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  host.append(h('div',{class:'brand'},logo,h('div',{},h('b',{},'JstKO'),h('span',{},'Yönetim Paneli'))));
  for(const [g,items] of NAV){
    host.append(h('div',{class:'navgroup'},g));
    for(const [id,label,i] of items)host.append(h('a',{class:'nav',href:'#/'+id,'data-p':id},ico(i),h('span',{},label)));
  }
}
async function route(){
  const parts=(location.hash.replace(/^#\//,'')||'dashboard').split('/').map(decodeURIComponent);
  const page=parts[0];
  const navId=(page==='account'||page==='char')?'players':page==='clan'?'clans':page;
  $$('#navhost .nav').forEach(a=>a.classList.toggle('on',a.dataset.p===navId));
  $('#app').classList.remove('nav-open');
  const main=clear($('#main'));
  window.scrollTo(0,0);
  const fn=PAGE[page]||PAGE.dashboard;
  try{await fn(main,parts.slice(1))}catch(e){if(e.message!=='auth')main.append(h('div',{class:'card empty'},'Sayfa yüklenemedi: '+e.message))}
}
function head(title,sub,...actions){return h('div',{class:'page-h'},h('div',{},h('h1',{},title),sub?h('p',{},sub):null),h('span',{class:'grow'}),...actions)}
function showLogin(){$('#app').classList.remove('on');$('#login').classList.add('on')}
async function enter(d){
  csrf=d.csrf;actor=d.actor;
  $('#login').classList.remove('on');$('#app').classList.add('on');
  $('#who').textContent=(actor||'?').slice(0,2).toUpperCase();$('#who').title=actor;
  buildNav();clear($('#burger')).append(ico('menu'));$$('.searchbox .ico').forEach(e=>e.replaceWith(ico('search')));
  route();pingServer();
}
$('#loginForm').addEventListener('submit',async e=>{
  e.preventDefault();
  try{const d=await api('/login',{body:{username:$('#lu').value.trim(),password:$('#lp').value}});$('#lp').value='';enter(d)}catch(_){}
});
$('#who').addEventListener('click',()=>{modal('Hesap',h('div',{class:'col'},h('p',{},'Giriş yapan: ',h('b',{},actor)),h('p',{class:'muted sm'},'Oturum 30 dakika hareketsizlikte kapanır.')),[{label:'Kapat',cls:'ghost'},{label:'Çıkış yap',cls:'danger',fn:async()=>{try{await api('/logout',{method:'POST',body:{}})}catch(_){};csrf='';showLogin()}}])});
$('#burger').addEventListener('click',()=>$('#app').classList.toggle('nav-open'));
$('#scrim').addEventListener('click',()=>$('#app').classList.remove('nav-open'));
window.addEventListener('hashchange',()=>{if($('#app').classList.contains('on'))route()});
setInterval(()=>{$('#clock').textContent=new Date().toLocaleString('tr-TR')},1000);
async function pingServer(){
  try{
    const d=await api('/dashboard',{quiet:true});
    const up=d.heartbeat_age!=null&&d.heartbeat_age<20;
    const s=clear($('#srv'));s.className='chip '+(up?'ok':'bad');
    s.append(h('span',{class:'dot '+(up?'ok':'bad')}),up?'Sunucu çevrimiçi · '+fmt(d.online)+' oyuncu':'Oyun sunucusu kapalı');
  }catch(_){}
  setTimeout(()=>{if($('#app').classList.contains('on'))pingServer()},10000);
}
(async()=>{try{enter(await api('/me',{quiet:true}))}catch(_){showLogin()}})();

/* ───────── command palette (Ctrl+K) ───────── */
function palette(){
  const q=h('input',{placeholder:'Oyuncu, hesap, item veya sayfa ara…',style:'width:100%;font-size:16px;padding:12px 14px'});
  const out=h('div',{class:'col',style:'margin-top:12px;max-height:55vh;overflow:auto;gap:4px'});
  let close;
  const go=(hash)=>{close();location.hash=hash};
  const row=(lab,sub,fn,i)=>h('div',{class:'mrow',onclick:fn},ico(i||'search'),h('div',{},h('b',{},lab),h('br'),h('small',{},sub||'')));
  const run=debounce(async()=>{
    const t=q.value.trim();clear(out);
    const pages=Object.entries(PAGE_TITLE).filter(([,l])=>!t||l.toLowerCase().includes(t.toLowerCase()));
    pages.slice(0,6).forEach(([id,l])=>out.append(row(l,'Sayfa',()=>go('#/'+id),'home')));
    if(t.length<2)return;
    const [acc,items]=await Promise.all([api('/accounts?q='+encodeURIComponent(t)+'&limit=6',{quiet:true}).catch(()=>[]),api('/item-search?q='+encodeURIComponent(t)+'&limit=6',{quiet:true}).catch(()=>[])]);
    acc.forEach(a=>out.append(row(a.account,(a.chars||[]).join(', '),()=>go('#/account/'+encodeURIComponent(a.account)),'users')));
    (acc.flatMap(a=>a.chars||[]).slice(0,4)).forEach(c=>out.append(row(c,'Karakter',()=>go('#/char/'+encodeURIComponent(c)),'shield')));
    items.forEach(i=>out.append(row(i.name,'Item #'+i.id,()=>go('#/items/'+i.id),'box')));
  },200);
  q.addEventListener('input',run);
  q.addEventListener('keydown',e=>{if(e.key==='Enter'){const f=out.querySelector('.mrow');if(f)f.click()}});
  close=modal('Hızlı arama',h('div',{},q,out),[],'');run();
}
$('#cmdk').addEventListener('click',palette);
document.addEventListener('keydown',e=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'&&$('#app').classList.contains('on')){e.preventDefault();palette()}});

/* misc shared */
const NATION={1:['Karus','karus'],2:['El Morad','elmo']};
const nationChip=n=>{const x=NATION[n]||['?',''];return h('span',{class:'chip '+x[1]},x[0])};
function className(c){const m=c%100;return ([1,5,6].includes(m)?'Savaşçı':[2,7,8].includes(m)?'Rogue':[3,9,10].includes(m)?'Büyücü':[4,11,12].includes(m)?'Rahip':m>=13?'Kurian':'?')+' ('+c+')'}
function field(label,input,full){return h('label',{class:'f'+(full?' full':'')},label,input)}
function searchBar(ph,onInput,extra){
  const i=h('input',{placeholder:ph,style:'flex:1;min-width:200px'});i.addEventListener('input',debounce(()=>onInput(i.value.trim()),300));
  return {el:h('div',{class:'row'},i,extra||null),input:i};
}
