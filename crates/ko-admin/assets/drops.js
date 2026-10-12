'use strict';
/* ───────── drop studio + group manager (drag & drop) ───────── */
const DROPSTATE={open:[],last:500};
const pctText=p=>'%'+(Number(p)/100).toFixed(2);
const isGroupId=id=>id>=100&&id<=99999999;
function toastAction(msg,label,fn){
  const t=h('div',{class:'toast ok row',style:'gap:12px'},h('span',{},msg),h('button',{class:'btn sm',onclick:()=>{t.remove();fn()}},label));
  $('#toasts').append(t);setTimeout(()=>t.remove(),9000);
}
const emptySlots=()=>Array.from({length:12},(_,i)=>({slot:i+1,item_id:0,percent:0,name:'',icon:0}));
function normSlots(m){const out=emptySlots();for(const s of m.slots||[])if(s.slot>=1&&s.slot<=12)out[s.slot-1]={...s};return out}
const payload=m=>m.slots.map(s=>[s.item_id||0,s.item_id?s.percent:0]);

PAGE.drops=async function(main,[sidArg]){
  const q=h('input',{placeholder:'Canavar adı veya SID…',style:'width:100%'});
  const mlist=h('div',{class:'mlist'});
  const work=h('div');
  const trash=h('div',{class:'trash'},ico('trash'),h('div',{class:'sm'},'Slotu buraya sürükle: boşalt'));
  trash.addEventListener('dragover',e=>{e.preventDefault();trash.classList.add('over')});
  trash.addEventListener('dragleave',()=>trash.classList.remove('over'));
  trash.addEventListener('drop',async e=>{e.preventDefault();trash.classList.remove('over');const p=parse(e);if(p&&p.dslot)await mutate(findMon(p.dslot.sid),m=>{m.slots[p.dslot.slot-1]={slot:p.dslot.slot,item_id:0,percent:0,name:'',icon:0}},'Slot boşaltıldı')});
  const findMon=sid=>DROPSTATE.open.find(m=>m.sid===sid);
  const parse=e=>{try{return JSON.parse(e.dataTransfer.getData('text/plain'))}catch(_){return null}};
  let selected=null; // {sid,slot} focus for library clicks

  async function persist(m,reason){
    await api('/drops/'+m.sid+'/replace',{body:{slots:payload(m),reason:reason||'drop editörü'}});
  }
  async function mutate(m,fn,msg){
    if(!m)return;
    const before=JSON.parse(JSON.stringify(m.slots));
    fn(m);
    try{await persist(m);draw();toastAction(msg||'Kaydedildi','Geri al',async()=>{m.slots=before;try{await persist(m,'geri alma');draw();toast('Geri alındı','ok')}catch(_){}})}
    catch(_){m.slots=before;draw()}
  }
  async function openMon(sid){
    if(findMon(sid))return;
    const r=await api('/drops?q='+sid+'&limit=5');
    const m=r.find(x=>x.sid===sid);if(!m)return toast('Canavar bulunamadı','err');
    m.slots=normSlots(m);
    api('/drop-usage/'+m.s_item,{quiet:true}).then(u=>{m.shared=u;draw()}).catch(()=>{});
    DROPSTATE.open.unshift(m);draw();
  }
  async function search(){
    const t=q.value.trim();
    const ms=await api('/drops?q='+encodeURIComponent(t)+'&limit=40',{quiet:true}).catch(()=>[]);
    clear(mlist).append(...ms.map(m=>h('div',{class:'mrow',onclick:()=>openMon(m.sid)},ico('sword'),h('div',{style:'min-width:0'},h('b',{},m.name),h('br'),h('small',{},'SID '+m.sid+' · tablo '+m.s_item)))));
    if(!ms.length)mlist.append(h('div',{class:'empty'},'Canavar yok'));
  }
  function slotCell(m,s){
    const full=s.item_id>0,grp=full&&isGroupId(s.item_id);
    const cell=h('div',{class:'ds-slot'+(full?' full':''),draggable:full?'true':null,dataset:{sid:m.sid,slot:s.slot}},
      h('span',{class:'n'},s.slot),
      grp?h('div',{class:'slot full'},ico('layers')):full?itemSlot({id:s.item_id,icon:s.icon},{drag:false}):h('div',{class:'slot'},h('span',{class:'num'},'+')),
      h('div',{class:'t'},full?[h('b',{},grp?'Grup '+s.item_id:(s.name||'#'+s.item_id)),h('span',{class:'pct'+(s.percent>=5000?' pct-hot':'')},pctText(s.percent))]:h('span',{class:'faint'},'boş')));
    cell.addEventListener('dragstart',e=>{e.stopPropagation();e.dataTransfer.setData('text/plain',JSON.stringify({dslot:{sid:m.sid,slot:s.slot}}))});
    cell.addEventListener('dragover',e=>{e.preventDefault();e.stopPropagation();cell.classList.add('over')});
    cell.addEventListener('dragleave',()=>cell.classList.remove('over'));
    cell.addEventListener('drop',async e=>{
      e.preventDefault();e.stopPropagation();cell.classList.remove('over');const p=parse(e);if(!p)return;
      if(p.pick){const pc=p.pick.group?100:DROPSTATE.last;
        return mutate(m,mm=>{mm.slots[s.slot-1]={slot:s.slot,item_id:p.pick.id,percent:pc,name:p.pick.name,icon:p.pick.icon}},'Slot '+s.slot+' güncellendi');}
      if(p.dslot){
        const src=findMon(p.dslot.sid);if(!src||(src.sid===m.sid&&p.dslot.slot===s.slot))return;
        const a={...src.slots[p.dslot.slot-1]},b={...m.slots[s.slot-1]};
        const copy=e.altKey||e.ctrlKey;
        if(src.sid===m.sid){return mutate(m,mm=>{mm.slots[s.slot-1]={...a,slot:s.slot};if(!copy)mm.slots[p.dslot.slot-1]={...b,slot:p.dslot.slot}},copy?'Kopyalandı':'Yer değiştirdi')}
        await mutate(m,mm=>{mm.slots[s.slot-1]={...a,slot:s.slot}},copy?'Kopyalandı':'Taşındı');
        if(!copy)await mutate(src,ss=>{ss.slots[p.dslot.slot-1]={slot:p.dslot.slot,item_id:0,percent:0,name:'',icon:0}},'Kaynak boşaltıldı');
      }
    });
    cell.addEventListener('click',()=>{selected={sid:m.sid,slot:s.slot};slotDialog(m,s)});
    return cell;
  }
  function slotDialog(m,s){
    const id=h('input',{type:'number',value:s.item_id,min:'0'});
    const pc=h('input',{type:'number',value:s.percent,min:'0',max:'10000',step:'10'});
    const range_=h('input',{type:'range',min:'0',max:'10000',step:'10',value:s.percent,style:'width:100%'});
    const lab=h('span',{class:'chip blue'},pctText(s.percent));
    const sync=(v)=>{pc.value=v;range_.value=v;lab.textContent=pctText(v)};
    pc.addEventListener('input',()=>sync(pc.value));range_.addEventListener('input',()=>sync(range_.value));
    const presets=[10,100,500,1000,2500,5000,10000].map(v=>h('button',{class:'btn sm',onclick:()=>sync(v)},pctText(v)));
    const lib=libraryPanel({onPick:it=>{id.value=it.id}});
    modal(m.name+' · slot '+s.slot,h('div',{class:'col'},
      h('div',{class:'form'},field('Item ID veya grup no (0 = boş)',id),field('Oran (ham 0–10000)',pc)),
      h('div',{class:'row'},range_,lab),h('div',{class:'row'},presets),
      h('div',{class:'hint'},'Ham değer 10000 = %100. Önceki girilen oran sonraki sürükle-bırak için varsayılan olur.'),lib.el),[
      {label:'İptal',cls:'ghost'},{label:'Boşalt',cls:'danger',fn:()=>mutate(m,mm=>{mm.slots[s.slot-1]={slot:s.slot,item_id:0,percent:0,name:'',icon:0}},'Slot boşaltıldı')},
      {label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
        const it=Number(id.value),p=Number(pc.value);DROPSTATE.last=p||DROPSTATE.last;
        let info=it?await getItem(it):null;
        await mutate(m,mm=>{mm.slots[s.slot-1]={slot:s.slot,item_id:it,percent:it?p:0,name:info?info.str_name:'',icon:info?info.item_icon_id1:0}},'Slot '+s.slot+' kaydedildi');close();return true}}],'xl');
  }
  function card(m){
    const hdr=h('header',{draggable:'true',title:'Tüm tabloyu başka bir kartın üstüne sürükleyerek kopyala'},
      ico('sword'),h('h3',{style:'margin:0'},m.name),h('span',{class:'chip'},'SID '+m.sid),h('span',{class:'chip'},'tablo '+m.s_item),
      m.shared&&m.shared.length>1?h('button',{class:'chip blue',style:'cursor:pointer',onclick:()=>sharedDialog(m)},'Paylaşımlı · '+m.shared.length+' canavar'):null,h('span',{class:'grow'}),
      h('button',{class:'btn sm',onclick:()=>mutate(m,mm=>{mm.slots=emptySlots()},'Tablo temizlendi')},'Temizle'),
      h('button',{class:'btn sm ghost',onclick:()=>{DROPSTATE.open=DROPSTATE.open.filter(x=>x!==m);draw()}},ico('x')));
    hdr.addEventListener('dragstart',e=>e.dataTransfer.setData('text/plain',JSON.stringify({dtable:{sid:m.sid}})));
    const c=h('div',{class:'card dcard'},hdr,h('div',{class:'dslots'},m.slots.map(s=>slotCell(m,s))));
    c.addEventListener('dragover',e=>e.preventDefault());
    c.addEventListener('drop',async e=>{
      const p=parse(e);if(!p||!p.dtable||p.dtable.sid===m.sid)return;e.preventDefault();
      const src=findMon(p.dtable.sid);if(!src)return;
      if(!await confirmBox('Tabloyu kopyala',src.name+' drop tablosu '+m.name+' üzerine yazılacak (geri alınabilir).','Kopyala'))return;
      mutate(m,mm=>{mm.slots=src.slots.map((s,i)=>({...s,slot:i+1}))},'Tablo kopyalandı');
    });
    return c;
  }
  function sharedDialog(m){
    modal('Ortak drop tablosu '+m.s_item,h('div',{class:'col'},h('p',{class:'muted'},'Bu tabloyu kullanan tüm canavarlar aynı dropu verir; burada yaptığın değişiklik hepsini etkiler.'),
      h('div',{class:'mlist'},m.shared.map(x=>h('div',{class:'mrow',onclick:()=>openMon(x.sid)},h('b',{},x.name),h('small',{},'SID '+x.sid+' · Lv '+x.level))))),[{label:'Kapat',cls:'ghost'}]);
  }
  function draw(){
    clear(work);
    if(!DROPSTATE.open.length)work.append(h('div',{class:'card empty'},ico('sword'),h('p',{},'Soldan bir canavar seç. Birden fazla canavarı yan yana açıp slotları aralarında sürükleyebilirsin.'),
      h('p',{class:'sm'},'Sürükle = taşı/yer değiştir · Ctrl/Alt ile bırak = kopyala · Başlığı başka karta sürükle = tüm tabloyu kopyala')));
    for(const m of DROPSTATE.open)work.append(card(m));
  }
  const lib=libraryPanel({groups:true,onPick:it=>{
    if(!selected)return toast('Önce bir slota tıkla ya da öğeyi slota sürükle');
    const m=findMon(selected.sid);if(!m)return;
    mutate(m,mm=>{mm.slots[selected.slot-1]={slot:selected.slot,item_id:it.id,percent:it.group?100:DROPSTATE.last,name:it.name,icon:it.icon}},'Slot '+selected.slot+' güncellendi');}});
  const bulk=h('button',{class:'btn sm danger',onclick:bulkDialog},'Tüm canavarlara toplu oran…');
  main.append(head('Drop Editörü','Sürükle-bırak ile canavar drop tablolarını düzenle; her değişiklik tek işlemde kaydedilir ve çalışan sunucuya yüklenir.',bulk),
    h('div',{class:'studio'},
      h('div',{class:'panel'},h('h4',{},'Canavarlar'),q,mlist,h('div',{style:'margin-top:12px'},trash)),
      work,
      h('div',{class:'panel lib'},lib.el)));
  q.addEventListener('input',debounce(search,300));search();draw();
  if(sidArg)openMon(Number(sidArg));
};
function bulkDialog(){
  const p=h('input',{type:'number',value:'0'});
  modal('Tüm canavar droplarını ayarla',h('div',{class:'col'},h('div',{class:'hint warn'},'DİKKAT: Tüm canavarların tüm drop oranları göreli olarak değişir (20 = %+20, −30 = %−30). Geri alma yoktur; önce veritabanı yedeği alın.'),h('div',{class:'form'},field('Yüzde değişim',p))),[
    {label:'İptal',cls:'ghost'},{label:'Uygula',cls:'danger',keep:true,fn:async close=>{
      if(!await confirmBox('Emin misiniz?','Tüm canavarların drop oranı %'+p.value+' değişecek.','Evet, uygula',true))return true;
      try{const d=await api('/drops/bulk-adjust',{body:{percent:Number(p.value)}});toast(d.updated_rows+' satır güncellendi','ok');close()}catch(_){}return true}}]);
}

/* ───────── group manager ───────── */
PAGE.groups=async function(main,[numArg]){
  const q=h('input',{placeholder:'Grup no, item ID veya item adı…',style:'width:100%'});
  const list=h('div',{class:'mlist'});const editor=h('div');
  let cur=null; // {num,items:[{item_id,name,icon}],orig:[ids],isNew}
  const parse=e=>{try{return JSON.parse(e.dataTransfer.getData('text/plain'))}catch(_){return null}};
  async function search(){
    const gs=await api('/group-items?q='+encodeURIComponent(q.value.trim())+'&limit=60',{quiet:true}).catch(()=>[]);
    clear(list).append(...gs.map(g=>h('div',{class:'mrow'+(cur&&cur.num===g.group_num?' on':''),onclick:()=>openGroup(g)},ico('layers'),h('div',{},h('b',{},'Grup '+g.group_num),h('br'),h('small',{},g.items.length+' item')))));
    if(!gs.length)list.append(h('div',{class:'empty'},'Grup yok'));
  }
  function openGroup(g){cur={num:g.group_num,items:g.items.map(i=>({item_id:i.item_id,name:i.name,icon:i.icon})),orig:g.items.map(i=>i.item_id),isNew:false};drawEditor();loadUsage()}
  const dirty=()=>cur&&(cur.isNew||JSON.stringify(cur.items.map(i=>i.item_id))!==JSON.stringify(cur.orig));
  async function newGroup(){
    const n=await api('/group-next');cur={num:n.next,items:[],orig:[],isNew:true};drawEditor();
  }
  function tile(it,idx){
    const el=itemSlot({id:it.item_id,icon:it.icon},{drag:true,note:'Grup '+cur.num+' · sıra '+(idx+1)});
    el.append(h('button',{class:'btn sm danger',style:'position:absolute;right:1px;top:1px;padding:0 5px;font-size:11px;line-height:16px',onclick:e=>{e.stopPropagation();cur.items.splice(idx,1);drawEditor()}},'×'));
    el.addEventListener('dragstart',e=>e.dataTransfer.setData('text/plain',JSON.stringify({gitem:idx})));
    el.addEventListener('dragover',e=>{e.preventDefault();el.classList.add('over')});el.addEventListener('dragleave',()=>el.classList.remove('over'));
    el.addEventListener('drop',e=>{
      e.preventDefault();e.stopPropagation();el.classList.remove('over');const p=parse(e);if(!p)return;
      if(p.pick)return addItem(p.pick,idx);
      if(p.gitem!=null&&p.gitem!==idx){const [m]=cur.items.splice(p.gitem,1);cur.items.splice(idx>p.gitem?idx-1+1:idx,0,m);drawEditor()}
    });
    return el;
  }
  function addItem(it,at){
    if(it.group)return toast('Grup içine grup eklenemez','err');
    if(cur.items.some(x=>x.item_id===it.id))return toast('Bu item zaten grupta','err');
    const row={item_id:it.id,name:it.name,icon:it.icon};
    if(at==null)cur.items.push(row);else cur.items.splice(at,0,row);drawEditor();
  }
  async function save(){
    try{await api('/group-items/'+cur.num,{body:{items:cur.items.map(i=>i.item_id)}});toast('Grup '+cur.num+' kaydedildi','ok');cur.orig=cur.items.map(i=>i.item_id);cur.isNew=false;drawEditor();search()}catch(_){}
  }
  async function del(){
    if(!await confirmBox('Grubu sil','Grup '+cur.num+' silinecek. Drop slotlarında bu numarayı kullanan canavarlar etkilenir.','Sil',true))return;
    try{await api('/group-items/'+cur.num,{method:'DELETE'});toast('Silindi','ok');cur=null;drawEditor();search()}catch(_){}
  }
  async function dup(){
    const n=await api('/group-next');const items=cur.items.map(i=>i.item_id);
    try{await api('/group-items/'+n.next,{body:{items}});toast('Grup '+n.next+' olarak kopyalandı','ok');cur={num:n.next,items:[...cur.items],orig:items,isNew:false};drawEditor();search()}catch(_){}
  }
  function bulkAdd(){
    const t=h('textarea',{rows:'5',placeholder:'389012000, 389079000, …',style:'width:100%'});
    modal('Toplu item ekle',h('div',{class:'col'},h('p',{class:'muted sm'},'Item numaralarını virgül, boşluk veya satırla ayır.'),t),[{label:'İptal',cls:'ghost'},{label:'Ekle',cls:'primary',fn:async()=>{
      const ids=[...new Set(t.value.split(/[\s,;]+/).map(Number).filter(n=>n>0))];
      for(const id of ids){const it=await getItem(id);if(!it){toast('#'+id+' tabloda yok','err');continue}addItem({id,name:it.str_name,icon:it.item_icon_id1})}}}]);
  }
  const usage=h('div');
  async function loadUsage(){
    clear(usage);if(!cur||cur.isNew)return;
    const u=await api('/group-usage/'+cur.num,{quiet:true}).catch(()=>[]);
    usage.append(h('h4',{style:'margin:0 0 8px'},'Bu grubu kullanan canavarlar ('+u.length+')'),
      u.length?h('div',{class:'mlist'},u.slice(0,50).map(x=>h('div',{class:'mrow',onclick:()=>location.hash='#/drops/'+x.sid},ico('sword'),h('div',{},h('b',{},x.name),h('br'),h('small',{},'SID '+x.sid+' · slot '+x.slot+' · '+pctText(x.percent)))))):h('p',{class:'muted sm'},'Hiçbir canavar bu grubu drop slotunda kullanmıyor.'));
  }
  function drawEditor(){
    clear(editor);
    if(!cur)return editor.append(h('div',{class:'card empty'},ico('layers'),h('p',{},'Soldan bir grup seç veya yeni grup oluştur.')));
    const dz=h('div',{class:'slots',style:'grid-template-columns:repeat(auto-fill,minmax(var(--slot),var(--slot)));min-height:140px;padding:10px;border:2px dashed #223f80;border-radius:12px;align-content:start'},cur.items.map((it,i)=>tile(it,i)));
    dz.addEventListener('dragover',e=>e.preventDefault());
    dz.addEventListener('drop',e=>{e.preventDefault();const p=parse(e);if(p&&p.pick)addItem(p.pick,null);else if(p&&p.gitem!=null){const [m]=cur.items.splice(p.gitem,1);cur.items.push(m);drawEditor()}});
    const trash=h('div',{class:'trash',style:'margin-top:10px'},ico('trash'),h('span',{class:'sm'},' Item\'i buraya sürükle: gruptan çıkar'));
    trash.addEventListener('dragover',e=>{e.preventDefault();trash.classList.add('over')});trash.addEventListener('dragleave',()=>trash.classList.remove('over'));
    trash.addEventListener('drop',e=>{e.preventDefault();trash.classList.remove('over');const p=parse(e);if(p&&p.gitem!=null){cur.items.splice(p.gitem,1);drawEditor()}});
    editor.append(h('div',{class:'card'},
      h('div',{class:'row',style:'margin-bottom:12px'},ico('layers'),h('h2',{},'Grup '+cur.num),h('span',{class:'chip'},cur.items.length+' item'),dirty()?h('span',{class:'chip blue'},'kaydedilmedi'):null,cur.isNew?h('span',{class:'chip ok'},'yeni'):null,h('span',{class:'grow'}),
        h('button',{class:'btn sm',onclick:bulkAdd},'Toplu ekle'),h('button',{class:'btn sm',disabled:cur.isNew||undefined,onclick:dup},ico('copy'),'Çoğalt'),
        h('button',{class:'btn sm',disabled:!dirty()||undefined,onclick:()=>{cur.items=cur.orig.map(id=>cur.items.find(i=>i.item_id===id)||{item_id:id,name:'',icon:0});drawEditor()}},ico('undo'),'Geri al'),
        h('button',{class:'btn sm danger',disabled:cur.isNew||undefined,onclick:del},'Sil'),
        h('button',{class:'btn sm primary',disabled:!dirty()||undefined,onclick:save},'Kaydet')),
      dz,trash),h('div',{class:'card',style:'margin-top:14px'},usage));
  }
  const lib=libraryPanel({onPick:it=>{if(!cur)return toast('Önce bir grup seç','err');addItem(it,null)}});
  main.append(head('Grup Yöneticisi','make_item_group tabloları: item ekle, sırala, çoğalt; hangi canavarların kullandığını gör.',h('button',{class:'btn primary',onclick:newGroup},ico('plus'),'Yeni grup')),
    h('div',{class:'studio'},h('div',{class:'panel'},h('h4',{},'Gruplar'),q,list),editor,h('div',{class:'panel lib'},lib.el)));
  q.addEventListener('input',debounce(search,300));await search();drawEditor();
  if(numArg){const gs=await api('/group-items?q='+encodeURIComponent(numArg)+'&limit=5',{quiet:true}).catch(()=>[]);const g=gs.find(x=>String(x.group_num)===numArg);if(g)openGroup(g)}
};
