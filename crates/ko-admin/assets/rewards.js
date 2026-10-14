'use strict';
/* Reward center: edit every reward table of the game (event rewards, daily quests, zone rewards, stores…).
   Changes are saved to the database and the game server reloads the table at once. */

PAGE.rewards=async function(main){
  const meta=await api('/rewards/systems');
  const EVENTS=meta.events;
  let cur=meta.systems.find(s=>s.key===(sessionStorageGet('rw_sys')))||meta.systems[0],groupVal='',filter='';
  const side=h('div',{class:'col',style:'gap:4px;min-width:250px;max-width:270px'});
  const body=h('div',{style:'flex:1;min-width:0'});
  main.append(head('Ödül Merkezi','Tüm etkinlik / görev / mağaza ödülleri: ekle, düzenle, sil — anında oyunda geçerli olur'),
    h('div',{class:'row',style:'align-items:flex-start;gap:16px'},side,body));

  function sessionStorageGet(k){try{return sessionStorage.getItem(k)}catch(_){return null}}
  function drawSide(){
    clear(side);
    const groups=[...new Set(meta.systems.map(s=>s.group))];
    for(const g of groups){
      side.append(h('div',{class:'navgroup',style:'margin-top:8px'},g));
      for(const s of meta.systems.filter(x=>x.group===g))
        side.append(h('a',{class:'nav'+(s===cur?' on':''),href:'javascript:void 0',onclick:()=>{cur=s;groupVal='';filter='';try{sessionStorage.setItem('rw_sys',s.key)}catch(_){}drawSide();load()}},
          h('span',{style:'flex:1'},s.title),h('span',{class:'chip'},String(s.count<0?'?':s.count))));
    }
  }
  const colOf=(s,n)=>s.cols.find(c=>c.name===n);
  const pkOf=(s,r)=>{const o={};for(const k of s.pk)o[k]=r[k];return o};
  const evName=v=>EVENTS[v]?EVENTS[v]+' ('+v+')':String(v);

  function cell(c,v){
    if(c.kind==='item'){
      const box=h('span',{class:'row',style:'gap:6px;flex-wrap:nowrap'});
      if(!v)return h('span',{class:'muted'},'—');
      const img=h('img',{src:icon(0),style:'width:26px;height:26px',onerror:iconErr}),nm=h('span',{class:'xs'},'#'+v);
      box.append(img,nm);
      getItem(v).then(it=>{if(it){img.src=icon(it.item_icon_id1);nm.textContent=(it.str_name||('#'+v))+' · '+v}});
      return box;
    }
    if(c.kind==='bool')return h('span',{class:'chip '+(v?'ok':'')},v?'evet':'hayır');
    if(c.kind==='event')return evName(v);
    return v===null||v===undefined||v===''?h('span',{class:'muted'},'—'):String(v);
  }

  async function load(){
    clear(body).append(h('div',{class:'card empty'},'Yükleniyor…'));
    let rows;
    try{rows=(await api('/rewards/'+cur.key)).rows}catch(_){clear(body).append(h('div',{class:'card empty'},'Tablo okunamadı (veritabanında yok olabilir).'));return}
    paint(rows);
  }

  function paint(rows){
    const s=cur,gc=s.group_col;
    const gvals=gc?[...new Set(rows.map(r=>r[gc]))]:[];
    const shown=rows.filter(r=>(!gc||groupVal===''||String(r[gc])===groupVal)&&(!filter||JSON.stringify(Object.values(r)).toLowerCase().includes(filter.toLowerCase())));
    const gsel=gc?h('select',{onchange:e=>{groupVal=e.target.value;paint(rows)}},h('option',{value:''},'Hepsi ('+colOf(s,gc).label+')'),
      gvals.map(v=>h('option',{value:String(v),selected:String(v)===groupVal},colOf(s,gc).kind==='event'?evName(v):colOf(s,gc).label+' '+v))):null;
    const q=h('input',{placeholder:'Ara…',value:filter,style:'min-width:160px'});
    q.addEventListener('input',debounce(()=>{filter=q.value.trim();paint(rows);const n=body.querySelector('input[placeholder="Ara…"]');if(n){n.focus();n.setSelectionRange(n.value.length,n.value.length)}},350));
    const table=h('table',{},h('thead',{},h('tr',{},s.cols.map(c=>h('th',{},c.label)),h('th',{},''))),
      h('tbody',{},shown.map(r=>h('tr',{},s.cols.map(c=>h('td',{},cell(c,r[c.name]))),
        h('td',{class:'nowrap'},h('button',{class:'btn sm',onclick:()=>edit(r)},'Düzenle'),' ',
          s.key==='daily_reward_cum'?null:h('button',{class:'btn sm danger',onclick:()=>remove(r)},ico('trash')))))));
    clear(body).append(
      h('div',{class:'card',style:'margin-bottom:12px'},h('h2',{style:'margin:0 0 6px'},s.title),h('p',{class:'muted sm',style:'margin:0'},s.note),
        h('p',{class:'xs',style:'margin:6px 0 0;color:'+(s.live?'#4ade80':'#94a3b8')},s.live?'● Kaydedince oyun sunucusu tabloyu hemen yeniler.':'● Bu ödül oyunda kullanılırken veritabanından okunur, kaydedince geçerlidir.')),
      h('div',{class:'row',style:'margin-bottom:10px;gap:8px'},gsel,q,h('span',{class:'grow'}),s.key==='daily_reward_cum'?null:h('button',{class:'btn primary',onclick:()=>edit(null)},'+ Yeni satır')),
      h('div',{class:'card'},h('div',{class:'tw'},shown.length?table:h('p',{class:'muted',style:'padding:14px'},'Kayıt yok.'))));
  }

  async function remove(r){
    if(!await confirmBox('Satır silinsin mi?','Bu ödül kaydı silinecek ve oyunda hemen kalkacak.','Sil',true))return;
    try{const d=await api('/rewards/'+cur.key+'/delete',{body:{pk:pkOf(cur,r)}});toast(d.note||'Silindi','ok');load()}catch(_){}
  }

  function edit(r){
    const s=cur,F={};
    let activeItem=null;
    const fields=s.cols.map(c=>{
      if(!r&&s.serial&&s.pk[0]===c.name)return null;                 // serial: given by the database
      if(!r&&s.next_pk&&s.pk[0]===c.name)return null;                // next free number
      const v=r?r[c.name]:(c.name===s.group_col&&groupVal!==''?Number(groupVal):(c.kind==='bool'?true:(c.kind==='text'?'':0)));
      let inp;
      if(c.kind==='bool'){inp=h('input',{type:'checkbox',checked:!!v});F[c.name]=()=>inp.checked}
      else if(c.kind==='event'){inp=h('select',{},Object.entries(EVENTS).map(([k,n])=>h('option',{value:k,selected:Number(k)===Number(v)},k+' · '+n)));F[c.name]=()=>Number(inp.value)}
      else if(c.kind==='text'){inp=h('input',{value:v||''});F[c.name]=()=>inp.value}
      else{inp=h('input',{type:'number',value:v===null||v===undefined?0:v});F[c.name]=()=>Number(inp.value)}
      const locked=r&&s.pk.includes(c.name)&&s.key!=='daily_reward_cum';
      if(locked)inp.disabled=true;
      let extra=null;
      if(c.kind==='item'){
        const pv=h('span',{class:'xs muted'},'');
        const show=async()=>{const id=Number(inp.value);pv.textContent=id?'…':'';if(!id)return;const it=await getItem(id);pv.textContent=it?(it.str_name||'')+' (#'+id+')':'bulunamadı'};
        inp.addEventListener('input',debounce(show,300));inp.addEventListener('focus',()=>{activeItem=inp});show();extra=pv;
      }
      return field(c.label,h('div',{},inp,extra),false);
    }).filter(Boolean);
    const lib=libraryPanel({onPick:it=>{if(activeItem){activeItem.value=it.id;activeItem.dispatchEvent(new Event('input'))}else toast('Önce bir eşya kutusuna tıkla','err')}});
    const hasItem=s.cols.some(c=>c.kind==='item');
    modal(r?'Satırı düzenle':'Yeni satır',h('div',{class:'col'},h('div',{class:'form'},fields),hasItem?h('div',{},h('p',{class:'xs muted'},'Bir eşya kutusuna tıkla, aşağıdan eşya ara / seç.'),lib.el):null),[
      {label:'İptal',cls:'ghost'},
      {label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
        const row={};for(const k of Object.keys(F))row[k]=F[k]();
        try{const d=await api('/rewards/'+s.key+'/save',{body:{pk:r?pkOf(s,r):null,row}});toast(d.note||'Kaydedildi','ok');close();load()}catch(_){}return true}}],'wide');
  }

  drawSide();load();
};
