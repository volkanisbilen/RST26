'use strict';
/* Rewards: one page per event / system in the style of the Collection Race page (cards with item tiles), plus the
   advanced "all tables" editor. Every change is saved to the database and the game server reloads the table. */

let RW_META=null;
async function rwMeta(){if(!RW_META)RW_META=await api('/rewards/systems');return RW_META}
const rwSys=(m,k)=>m.systems.find(s=>s.key===k);
const rwPk=(s,r)=>{const o={};for(const k of s.pk)o[k]=r[k];return o};
const rwEv=(m,v)=>m.events[v]?m.events[v]+' ('+v+')':String(v);

/* an item tile like the Collection Race page: icon, name, "x count · detail" */
function rwTile(id,count,detail,onclick){
  const it={id,icon:0};
  const slot=itemSlot(it,{drag:false,count:0});
  const nm=h('b',{},id?('#'+id):'—'),line=h('span',{class:'pct'},(count>1||count===0?'×'+count:'×1')+(detail?' · '+detail:''));
  if(id)getItem(id).then(x=>{if(x){const img=slot.querySelector('img');if(img)img.src=icon(x.item_icon_id1);nm.textContent=x.str_name||('#'+id)}});
  return h('div',{class:'ds-slot full',onclick},slot,h('div',{class:'t'},nm,line));
}
const rwAdd=(label,onclick)=>h('div',{class:'ds-slot',style:'cursor:pointer;justify-content:center',onclick},ico('plus'),label||'Ödül ekle');

/* edit / add dialog for one row of a system (generic: made from the column list) */
function rwEdit(m,s,r,prefill,done){
  const F={};let activeItem=null;
  const fields=s.cols.map(c=>{
    if(!r&&s.serial&&s.pk[0]===c.name)return null;
    if(!r&&s.next_pk&&s.pk[0]===c.name)return null;
    const v=r?r[c.name]:(prefill&&c.name in prefill?prefill[c.name]:(c.kind==='bool'?true:(c.kind==='text'?'':0)));
    let inp;
    if(c.kind==='bool'){inp=h('input',{type:'checkbox',checked:!!v});F[c.name]=()=>inp.checked}
    else if(c.kind==='event'){inp=h('select',{},Object.entries(m.events).map(([k,n])=>h('option',{value:k,selected:Number(k)===Number(v)},k+' · '+n)));F[c.name]=()=>Number(inp.value)}
    else if(c.kind==='text'){inp=h('input',{value:v||''});F[c.name]=()=>inp.value}
    else{inp=h('input',{type:'number',value:v===null||v===undefined?0:v});F[c.name]=()=>Number(inp.value)}
    if((r&&s.pk.includes(c.name)&&s.key!=='daily_reward_cum')||(prefill&&prefill._lock&&prefill._lock.includes(c.name)))inp.disabled=true;
    let extra=null;
    if(c.kind==='item'){
      const pv=h('span',{class:'xs muted'},'');
      const show=async()=>{const id=Number(inp.value);pv.textContent=id?'…':'';if(!id)return;const it=await getItem(id);pv.textContent=it?(it.str_name||'')+' (#'+id+')':'bulunamadı'};
      inp.addEventListener('input',debounce(show,300));inp.addEventListener('focus',()=>{activeItem=inp});show();extra=pv;
    }
    return field(c.label,h('div',{},inp,extra),false);
  }).filter(Boolean);
  const hasItem=s.cols.some(c=>c.kind==='item');
  const lib=hasItem?libraryPanel({onPick:it=>{if(activeItem){activeItem.value=it.id;activeItem.dispatchEvent(new Event('input'))}else toast('Önce bir eşya kutusuna tıkla','err')}}):null;
  modal(r?'Ödülü düzenle':'Ödül ekle',h('div',{class:'col'},h('div',{class:'form'},fields),lib?h('div',{},h('p',{class:'xs muted'},'Bir eşya kutusuna tıkla, aşağıdan eşya ara / seç.'),lib.el):null),[
    {label:'İptal',cls:'ghost'},
    r&&s.key!=='daily_reward_cum'?{label:'Sil',cls:'danger',fn:async()=>{await rwRemove(s,r,done,true)}}:null,
    {label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
      const row={};for(const k of Object.keys(F))row[k]=F[k]();
      try{const d=await api('/rewards/'+s.key+'/save',{body:{pk:r?rwPk(s,r):null,row}});toast(d.note||'Kaydedildi','ok');close();done()}catch(_){}return true}}].filter(Boolean),'wide');
}
async function rwRemove(s,r,done,silent){
  if(!silent&&!await confirmBox('Satır silinsin mi?','Bu ödül kaydı silinecek ve oyunda hemen kalkacak.','Sil',true))return;
  try{const d=await api('/rewards/'+s.key+'/delete',{body:{pk:rwPk(s,r)}});toast(d.note||'Silindi','ok');done()}catch(_){}
}

/* the card pages */
function rwPage(cfg){
  return async function(main){
    const m=await rwMeta();
    const body=h('div',{});
    main.append(head(cfg.title,cfg.sub||''),body);
    const draw=async()=>{
      clear(body);
      for(const sec of cfg.sections){
        const s=rwSys(m,sec.sys);
        if(!s){body.append(h('div',{class:'card empty'},'Tanımsız sistem: '+sec.sys));continue}
        let rows;
        try{rows=(await api('/rewards/'+s.key)).rows}catch(_){body.append(h('div',{class:'card empty'},s.title+': tablo okunamadı (veritabanında yok olabilir).'));continue}
        if(sec.where)rows=rows.filter(r=>Object.entries(sec.where).every(([k,v])=>String(r[k])===String(v)));
        const groups=sec.groupBy?[...new Set([...(sec.groups||[]),...rows.map(r=>r[sec.groupBy])])].sort((a,b)=>a-b):[null];
        if(sec.heading)body.append(h('h3',{style:'margin:18px 0 8px'},sec.heading),sec.note?h('p',{class:'muted sm',style:'margin:0 0 8px'},sec.note):null);
        for(const g of groups){
          const grows=sec.groupBy?rows.filter(r=>r[sec.groupBy]===g):rows;
          const prefill=Object.assign({},sec.where||{},sec.groupBy?{[sec.groupBy]:g}:{},sec.prefill||{});
          const gtitle=sec.groupBy?(sec.groupLabel?sec.groupLabel(g,m):String(g)):(sec.cardTitle||s.title);
          const add=()=>rwEdit(m,s,null,prefill,draw);
          let content;
          if(sec.mode==='rows'){
            content=h('div',{class:'col',style:'gap:8px'},grows.map(r=>rwRow(m,s,sec,r,draw)),
              h('div',{class:'dslots'},rwAdd(sec.addLabel||'Satır ekle',add)));
          }else{
            content=h('div',{class:'dslots'},grows.map(r=>rwTile(r[sec.item],sec.count?r[sec.count]:1,sec.detail?sec.detail(r):'',()=>rwEdit(m,s,r,null,draw))),rwAdd(sec.addLabel,add));
          }
          body.append(h('div',{class:'card',style:'margin-bottom:14px'},
            h('div',{class:'row',style:'margin-bottom:10px'},ico(sec.icon||'flag'),h('h2',{},gtitle),sec.groupBy&&sec.chip?h('span',{class:'chip'},sec.chip(g)):null,h('span',{class:'grow'}),h('span',{class:'chip'},grows.length+' kayıt')),
            sec.hint?h('p',{class:'muted sm',style:'margin:0 0 8px'},sec.hint):null,content));
        }
      }
      body.append(h('p',{class:'xs muted',style:'margin-top:4px'},cfg.footer||'Kaydedilen her değişiklik oyun sunucusuna hemen uygulanır.'));
    };
    draw();
  };
}

/* a row with several reward items (event rewards, daily quests …): title, item tiles, number chips */
function rwRow(m,s,sec,r,draw){
  const items=(sec.rowItems||[]).filter(([ic])=>r[ic]).map(([ic,cc,tc])=>rwTile(r[ic],cc?r[cc]:1,tc&&r[tc]?r[tc]+' gün':'',()=>rwEdit(m,s,r,null,draw)));
  const chips=(sec.chips||[]).filter(([c])=>r[c]).map(([c,l])=>h('span',{class:'chip'},l+' '+Number(r[c]).toLocaleString('tr-TR')));
  return h('div',{class:'card',style:'padding:10px;background:#0b1633'},
    h('div',{class:'row',style:'margin-bottom:8px'},h('b',{},sec.rowTitle?sec.rowTitle(r,m):''),r.enabled===false||r.status===false?h('span',{class:'chip bad'},'kapalı'):null,h('span',{class:'grow'}),
      h('button',{class:'btn sm',onclick:()=>rwEdit(m,s,r,null,draw)},'Düzenle'),h('button',{class:'btn sm danger',onclick:()=>rwRemove(s,r,draw)},ico('trash'))),
    chips.length?h('div',{class:'row',style:'margin-bottom:8px;flex-wrap:wrap'},chips):null,
    h('div',{class:'dslots'},items.length?items:h('span',{class:'muted sm'},'Eşya yok')));
}

const EVT_ITEMS=[['item_id1','item_count1','item_expiration1'],['item_id2','item_count2','item_expiration2'],['item_id3','item_count3','item_expiration3']];
const EVT_CHIPS=[['experience','Exp'],['loyalty','NP'],['cash','KC'],['noah','Coin']];
const evSection=(local,heading)=>({sys:'event_rewards',where:{local_id:local},mode:'rows',cardTitle:heading,rowItems:EVT_ITEMS,chips:EVT_CHIPS,
  rowTitle:r=>(r.is_winner?'Kazananlar':'Kaybedenler')+(r.description?' · '+r.description:''),prefill:{local_id:local,status:true},addLabel:'Ödül satırı ekle',
  hint:'Kazanan ve kaybeden takım için ayrı satır ekle; her satır 3 eşya + exp / NP / KC / coin verebilir.'});

const RW_PAGES={
  rw_bdw:{title:'Border Defence War ödülleri',sub:'Kazanan / kaybeden takım ödülleri',sections:[evSection(9,'Border Defence War')]},
  rw_chaos:{title:'Chaos Expansion ödülleri',sub:'Genel ödüller ve sıra ödülleri',sections:[evSection(10,'Chaos Expansion'),
    {sys:'chaos_rewards',mode:'rows',cardTitle:'Sıra ödülleri',rowItems:[1,2,3,4,5].map(i=>['item_id'+i,'item_count'+i,'item_expiration'+i]),chips:EVT_CHIPS,rowTitle:r=>r.rank_id+'. sıra',addLabel:'Sıra ekle',icon:'list'}]},
  rw_juraid:{title:'Juraid Mountain ödülleri',sub:'Kazanan / kaybeden takım ödülleri',sections:[evSection(11,'Juraid Mountain')]},
  rw_ft:{title:'Forgotten Temple ödülleri',sub:'Etkinliği bitiren oyuncuların ödülleri',sections:[evSection(13,'Forgotten Temple')]},
  rw_other:{title:'Diğer savaş / etkinlik ödülleri',sub:'Castle Siege, Under the Castle, Snow War, Beef …',sections:[
    {sys:'event_rewards',mode:'rows',groupBy:'local_id',groups:[1,12,14,8],groupLabel:(g,m)=>rwEv(m,g),rowItems:EVT_ITEMS,chips:EVT_CHIPS,rowTitle:r=>(r.is_winner?'Kazananlar':'Kaybedenler')+(r.description?' · '+r.description:''),addLabel:'Ödül satırı ekle'}]},
  rw_ultima:{title:'Ultima ödülleri',sub:'Ultima\'s Dungeon: hayatta kalma, safhada elenme ve boss kutusu',sections:[
    {sys:'event_rules',where:{event_key:'ultima'},groupBy:'cond',groups:['survive','stage1','stage2','stage3','box'],prefill:{event_key:'ultima',enabled:true,_lock:['event_key']},
     groupLabel:g=>({survive:'Boss yenildi, hayatta kalanlar',stage1:'1. safhada elenenler',stage2:'2. safhada elenenler',stage3:'3. ve sonraki safhada elenenler',box:'Boss kutusu'}[g]||g),
     item:'item_id',count:'item_count',detail:r=>(r.days?r.days+' gün':'')+(r.chance<100?(r.days?' · ':'')+'%'+r.chance:'')+(r.enabled?'':' · KAPALI'),
     hint:'Boss kutusunda "şans" en yüksek yüzdedir; gerçek şans oyuncunun verdiği hasar payının 3 katıdır.'}]},
  rw_golem:{title:'Fire & Ice (Golem War) ödülleri',sub:'Kazanan, kaybeden ve beraberlik grupları',sections:[
    {sys:'event_rules',where:{event_key:'golem'},groupBy:'cond',groups:['win','lose','draw','belt'],prefill:{event_key:'golem',enabled:true,_lock:['event_key']},
     groupLabel:g=>({win:'Kazanan millet',lose:'Kaybeden millet',draw:'Beraberlik (herkes)',belt:'Şans ödülü (tüm katılanlar)'}[g]||g),
     item:'item_id',count:'item_count',detail:r=>(r.pick?r.pick+' kişiye':'herkese')+(r.days?' · '+r.days+' gün':'')+(r.chance<100?' · %'+r.chance:'')+(r.enabled?'':' · KAPALI'),
     hint:'"Rastgele kişi" 0 ise gruptaki herkese, N ise gruptan rastgele N kişiye verilir.'}]},
  rw_manes:{title:'Manes Survival ödülleri',sub:'Sıralamaya göre ödüller',sections:[
    {sys:'event_rules',where:{event_key:'manes'},mode:'rows',cardTitle:'Sıra ödülleri',rowItems:[['item_id','item_count','days']],rowTitle:r=>(r.rank_to>0?(r.rank_from===r.rank_to?r.rank_from+'. sıra':r.rank_from+'. – '+r.rank_to+'. sıra'):r.rank_from+'. sıra ve sonrası')+(r.note?' · '+r.note:''),
     prefill:{event_key:'manes',cond:'rank',enabled:true,rank_from:1,rank_to:1,_lock:['event_key','cond']},addLabel:'Sıra ödülü ekle'}],
    footer:'Manes potion fiyatları istemci tablosundan (MANES_MAGIC.tbl) geldiği için burada değil.'},
  rw_cind:{title:'Cinderella (Fun War) ödülleri',sub:'Sıraya göre exp / KC / NP / coin ve eşyalar',sections:[
    {sys:'cinderella',mode:'rows',cardTitle:'Sıra ödülleri (para / puan)',rowItems:[],chips:[['exp_count','Exp'],['cash_count','KC'],['loyalty_count','NP'],['money_count','Coin']],rowTitle:r=>r.rank_id+'. sıra',addLabel:'Sıra ekle',icon:'list'},
    {sys:'cinderella_items',groupBy:'rank_id',groupLabel:g=>g+'. sıra eşyaları',item:'item_id',count:'item_count',detail:r=>'slot '+r.slot+(r.item_duration?' · '+r.item_duration+' gün':''),addLabel:'Eşya ekle'}]},
  rw_zone:{title:'Bölge ödülleri',sub:'Bölgede öldürme ve online kalma ödülleri',sections:[
    {sys:'zone_kill',groupBy:'zone_id',groupLabel:g=>'Bölge '+g+' · öldürme ödülleri',item:'item_id',count:'item_count',detail:r=>r.kill_count+' kill · %'+r.drop_rate+(r.status===1?'':' · KAPALI'),addLabel:'Ödül ekle'},
    {sys:'zone_online',mode:'rows',heading:'Online kalma ödülleri',note:'Her X dakikada bir; premium oyuncular için ayrı ödül.',cardTitle:'Bölge başına',rowItems:[['item_id','item_count','item_time'],['pre_item_id','pre_item_count','pre_item_time']],
     chips:[['minute','Dakika'],['loyalty','NP'],['cash','KC'],['tl','TL'],['pre_loyalty','Prem. NP'],['pre_cash','Prem. KC'],['pre_tl','Prem. TL']],rowTitle:r=>'Bölge '+r.zone_id,addLabel:'Bölge ekle',icon:'map'}]},
  rw_dq:{title:'Günlük görevler (Daily Quest)',sub:'Görev yaz, yaratık ve ödülleri belirle — hemen aktif olur',sections:[
    {sys:'daily_quests',mode:'rows',cardTitle:'Görevler',rowItems:[['reward_1','count_1'],['reward_2','count_2'],['reward_3','count_3'],['reward_4','count_4']],
     chips:[['kill_count','Adet'],['mob_id_1','Yaratık 1'],['mob_id_2','Yaratık 2'],['mob_id_3','Yaratık 3'],['mob_id_4','Yaratık 4'],['min_level','Min Lv'],['max_level','Max Lv'],['zone_id','Bölge']],
     rowTitle:r=>'#'+r.id+' · '+(r.quest_name||'Görev'),addLabel:'Görev ekle',icon:'scroll',hint:'Yaratık sütunlarına NPC/yaratık SID numarası yazılır (0 = boş).'}]},
  rw_login:{title:'Günlük giriş ödülleri',sub:'Ayın giriş günlerine göre ve ay sonu ödülü',sections:[
    {sys:'daily_reward',item:'item_id',detail:r=>(Number(r.day_index)+1)+'. gün',cardTitle:'Günlük ödüller',addLabel:'Gün ekle',icon:'gift'},
    {sys:'daily_reward_cum',mode:'rows',cardTitle:'Ay sonu toplam ödülü',rowItems:[['item1'],['item2'],['item3']],rowTitle:()=>'Tüm günleri tamamlayanlar'}]},
  rw_manner:{title:'Manner Store',sub:'İtibar mağazasındaki ürünler',sections:[
    {sys:'manner_store',groupBy:'tab',groupLabel:g=>'Sayfa '+g,item:'item_id',count:'item_count',detail:r=>r.price+(r.currency?' NP':' manner')+(r.days?' · '+r.days+' gün':'')+(r.enabled?'':' · KAPALI'),addLabel:'Ürün ekle',icon:'gift'}]},
  rw_wheels:{title:'Çark ve çekiliş ödülleri',sub:'Coin, Puzzle, Rulet ve Lottery etkinlikleri',sections:[
    {sys:'native_coin',item:'item_id',count:'item_count',detail:r=>r.required_points+' puan',cardTitle:'Coin etkinliği',addLabel:'Ödül ekle'},
    {sys:'native_jigsaw',item:'item_id',count:'item_count',detail:r=>r.required_total+' parça',cardTitle:'Puzzle etkinliği',addLabel:'Ödül ekle'},
    {sys:'native_roulette',groupBy:'roulette_type',groupLabel:g=>'Rulet / Marble çarkı '+g,item:'item_id',count:'item_count',detail:r=>'slot '+r.slot+' · ağırlık '+r.weight,addLabel:'Slot ekle'},
    {sys:'lottery',mode:'rows',cardTitle:'Lottery',rowItems:[['reward_item1'],['reward_item2'],['reward_item3'],['reward_item4']],chips:[['user_limit','Kişi'],['event_time','Süre']],rowTitle:r=>'Lottery #'+r.lnum,addLabel:'Lottery ekle'}]},
  rw_pus:{title:'Power Up Store',sub:'Mağaza ürünleri ve kategorileri — kaydedince mağaza hemen güncellenir',sections:[
    {sys:'pus_category',mode:'rows',cardTitle:'Kategoriler (mağaza sekmeleri)',rowItems:[],chips:[['category_id','Kategori no']],rowTitle:r=>r.category_name+(r.description?' · '+r.description:'')+(r.status?'':' (kapalı)'),addLabel:'Kategori ekle',icon:'list'},
    {sys:'pus_items',groupBy:'category',groupLabel:(g)=>'Kategori '+g,item:'item_id',count:'buy_count',detail:r=>Number(r.price).toLocaleString('tr-TR')+(r.price_type?' TL':' KC')+(r.item_name?' · '+r.item_name:''),addLabel:'Ürün ekle',icon:'box'}],
    footer:'Fiyat tipi: 0 = Knight Cash, 1 = TL. Oyuncular mağazayı yeniden açınca yeni listeyi görür.'},
};
for(const [k,cfg] of Object.entries(RW_PAGES))PAGE[k]=rwPage(cfg);

/* advanced editor: every table as a plain list */
PAGE.rewards=async function(main){
  const meta=await rwMeta();
  const systems=meta.systems.filter(s=>!/^pus_/.test(s.key));
  let cur=systems[0],groupVal='',filter='';
  const side=h('div',{class:'col',style:'gap:4px;min-width:250px;max-width:270px'});
  const body=h('div',{style:'flex:1;min-width:0'});
  main.append(head('Tüm ödül tabloları','Gelişmiş: her tablo düz liste olarak'),h('div',{class:'row',style:'align-items:flex-start;gap:16px'},side,body));
  const colOf=(s,n)=>s.cols.find(c=>c.name===n);
  function drawSide(){
    clear(side);
    for(const g of [...new Set(systems.map(s=>s.group))]){
      side.append(h('div',{class:'navgroup',style:'margin-top:8px'},g));
      for(const s of systems.filter(x=>x.group===g))
        side.append(h('a',{class:'nav'+(s===cur?' on':''),href:'javascript:void 0',onclick:()=>{cur=s;groupVal='';filter='';drawSide();load()}},h('span',{style:'flex:1'},s.title),h('span',{class:'chip'},String(s.count<0?'?':s.count))));
    }
  }
  function cell(c,v){
    if(c.kind==='item'){
      if(!v)return h('span',{class:'muted'},'—');
      const img=h('img',{src:icon(0),style:'width:26px;height:26px',onerror:iconErr}),nm=h('span',{class:'xs'},'#'+v);
      getItem(v).then(it=>{if(it){img.src=icon(it.item_icon_id1);nm.textContent=(it.str_name||('#'+v))+' · '+v}});
      return h('span',{class:'row',style:'gap:6px;flex-wrap:nowrap'},img,nm);
    }
    if(c.kind==='bool')return h('span',{class:'chip '+(v?'ok':'')},v?'evet':'hayır');
    if(c.kind==='event')return rwEv(meta,v);
    return v===null||v===undefined||v===''?h('span',{class:'muted'},'—'):String(v);
  }
  async function load(){
    clear(body).append(h('div',{class:'card empty'},'Yükleniyor…'));
    let rows;try{rows=(await api('/rewards/'+cur.key)).rows}catch(_){clear(body).append(h('div',{class:'card empty'},'Tablo okunamadı (veritabanında yok olabilir).'));return}
    paint(rows);
  }
  function paint(rows){
    const s=cur,gc=s.group_col;
    const gvals=gc?[...new Set(rows.map(r=>r[gc]))]:[];
    const shown=rows.filter(r=>(!gc||groupVal===''||String(r[gc])===groupVal)&&(!filter||JSON.stringify(Object.values(r)).toLowerCase().includes(filter.toLowerCase())));
    const gsel=gc?h('select',{onchange:e=>{groupVal=e.target.value;paint(rows)}},h('option',{value:''},'Hepsi ('+colOf(s,gc).label+')'),gvals.map(v=>h('option',{value:String(v),selected:String(v)===groupVal},colOf(s,gc).kind==='event'?rwEv(meta,v):colOf(s,gc).label+' '+v))):null;
    const q=h('input',{placeholder:'Ara…',value:filter,style:'min-width:160px'});
    q.addEventListener('input',debounce(()=>{filter=q.value.trim();paint(rows);const n=body.querySelector('input[placeholder="Ara…"]');if(n){n.focus();n.setSelectionRange(n.value.length,n.value.length)}},350));
    const table=h('table',{},h('thead',{},h('tr',{},s.cols.map(c=>h('th',{},c.label)),h('th',{},''))),
      h('tbody',{},shown.map(r=>h('tr',{},s.cols.map(c=>h('td',{},cell(c,r[c.name]))),h('td',{class:'nowrap'},h('button',{class:'btn sm',onclick:()=>rwEdit(meta,s,r,null,load)},'Düzenle'),' ',s.key==='daily_reward_cum'?null:h('button',{class:'btn sm danger',onclick:()=>rwRemove(s,r,load)},ico('trash')))))));
    clear(body).append(h('div',{class:'card',style:'margin-bottom:12px'},h('h2',{style:'margin:0 0 6px'},s.title),h('p',{class:'muted sm',style:'margin:0'},s.note)),
      h('div',{class:'row',style:'margin-bottom:10px;gap:8px'},gsel,q,h('span',{class:'grow'}),s.key==='daily_reward_cum'?null:h('button',{class:'btn primary',onclick:()=>rwEdit(meta,s,null,groupVal!==''&&gc?{[gc]:Number(groupVal)}:null,load)},'+ Yeni satır')),
      h('div',{class:'card'},h('div',{class:'tw'},shown.length?table:h('p',{class:'muted',style:'padding:14px'},'Kayıt yok.'))));
  }
  drawSide();load();
};
