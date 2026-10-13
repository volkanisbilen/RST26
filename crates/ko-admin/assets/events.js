'use strict';
/* Event center: live state + manual start/stop of every event, the automatic schedule,
   the native temple/war schedule and the command history. Replaces PAGE.events from pages.js. */

const EV_STATE={idle:['Boşta',''],registration:['Kayıt açık','blue'],active:['Çalışıyor','ok']};
const evDayBit=i=>1<<i;                      // Sunday = 1 … Saturday = 64 (same as the game server)
const evDaysText=m=>m===127?'Her gün':DAYS.map((d,i)=>(m&evDayBit(i))?d:null).filter(Boolean).join(' ')||'—';
const evTime=(hh,mm)=>String(hh).padStart(2,'0')+':'+String(mm).padStart(2,'0');

async function evRun(ev,action,arg){
  const label={start:'Başlat',startnow:'Hemen başlat',stop:'Durdur'}[action];
  if(ev.arg_required&&action!=='stop'&&!String(arg||'').trim()){toast(ev.name+': '+ev.arg_label+' gerekli','err');return false}
  if(!await confirmBox(ev.name+' — '+label,action==='stop'?'Etkinlik durdurulacak; içindeki oyuncular etkilenebilir.':'Komut oyun sunucusuna gönderilecek.',label,action==='stop'))return false;
  let id;
  try{id=(await api('/events/run/'+ev.key,{body:{action,arg:String(arg||'').trim()}})).id}catch(_){return false}
  for(let i=0;i<14;i++){
    await new Promise(r=>setTimeout(r,700));
    let r;try{r=await api('/events/command/'+id,{quiet:true})}catch(_){continue}
    if(r.status==='done'){toast(ev.name+': '+(r.result||'tamam'),'ok');return true}
    if(r.status==='failed'||r.status==='expired'){toast(ev.name+': '+(r.result||r.status),'err');return false}
  }
  toast('Komut sıraya alındı; oyun sunucusu yanıt vermedi (kapalı olabilir).','err');
  return false;
}

function evCard(ev,reload){
  const [txt,cls]=EV_STATE[ev.state]||[ev.state,''];
  const arg=ev.arg_label?h('input',{placeholder:ev.arg_label,style:'width:100%;margin:8px 0 0'}):null;
  const btn=(label,action,cls2)=>h('button',{class:'btn sm '+(cls2||''),onclick:async()=>{if(await evRun(ev,action,arg?arg.value:''))reload()}},label);
  return h('div',{class:'card ev-card '+ev.state},
    h('div',{class:'row',style:'justify-content:space-between;gap:8px;align-items:flex-start'},
      h('b',{},ev.name),h('span',{class:'chip '+cls},txt)),
    h('div',{class:'xs muted',style:'min-height:18px;margin-top:4px'},ev.detail||''),
    arg,
    h('div',{class:'row',style:'margin-top:10px;gap:6px;flex-wrap:wrap'},
      btn(ev.has_now?'Kayıt aç':'Başlat','start','primary'),
      ev.has_now?btn('Hemen başlat','startnow'):null,
      ev.can_stop?btn('Durdur','stop','danger'):null));
}

function evLive(box,d,reload){
  clear(box);
  if(!d.events.length){box.append(h('div',{class:'empty'},'Henüz etkinlik listesi yok. Oyun sunucusu yeni sürümle bir kez açılıp migration (20261013000002) uygulanınca burada görünür.'));return}
  const groups=[];d.events.forEach(e=>{let g=groups.find(x=>x[0]===e.group);if(!g)groups.push(g=[e.group,[]]);g[1].push(e)});
  groups.forEach(([name,list])=>box.append(h('h3',{style:'margin:18px 0 8px'},name),
    h('div',{class:'ev-grid'},list.map(e=>evCard(e,reload)))));
}

function evAutoDialog(d,sch,reload){
  const s=sch||{key:d.events[0]?.key,enabled:true,days:127,hour:12,minute:0,arg:'',duration_min:0,note:''};
  const key=h('select',{},d.events.map(e=>h('option',{value:e.key},e.name)));key.value=s.key;
  const time=h('input',{type:'time',value:evTime(s.hour,s.minute)});
  const days=DAYS.map((n,i)=>h('label',{class:'row',style:'gap:5px'},h('input',{type:'checkbox',checked:!!(s.days&evDayBit(i))}),n));
  const arg=h('input',{value:s.arg,placeholder:'Etkinliğe göre (kayıt süresi dk, ID …)'});
  const dur=h('input',{type:'number',min:'0',max:'600',value:s.duration_min});
  const note=h('input',{value:s.note,maxlength:'80'});
  const en=h('select',{},h('option',{value:'1'},'Açık'),h('option',{value:'0'},'Kapalı'));en.value=s.enabled?'1':'0';
  modal(sch?'Zamanlamayı düzenle':'Yeni zamanlama',h('div',{class:'col'},
    h('div',{class:'form'},field('Etkinlik',key),field('Saat (sunucu saati)',time),field('Durum',en),field('Argüman',arg),field('Otomatik durdur (dk, 0 = kendiliğinden biter)',dur),field('Not',note)),
    h('h3',{},'Günler'),h('div',{class:'row'},days)),[
    {label:'İptal',cls:'ghost'},{label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
      const mask=days.reduce((m,l,i)=>m|(l.querySelector('input').checked?evDayBit(i):0),0);
      if(!time.value||!mask){toast('Saat ve en az bir gün seçin','err');return true}
      const [hh,mm]=time.value.split(':').map(Number);
      const body={key:key.value,enabled:en.value==='1',days:mask,hour:hh,minute:mm,arg:arg.value.trim(),duration_min:Number(dur.value)||0,note:note.value.trim()};
      try{await api(sch?'/events/auto/'+sch.id:'/events/auto',{body});toast('Zamanlama kaydedildi','ok');close();reload()}catch(_){}
      return true}}],'wide');
}

function evAuto(box,d,reload){
  clear(box);
  const name=k=>(d.events.find(e=>e.key===k)||{name:k}).name;
  box.append(h('div',{class:'hint',style:'margin-bottom:12px'},'Saatler sunucu saatidir ('+(d.timezone||'?')+', şu an '+d.server_time+'). Kayıtlı etkinliklerin (Manes, Ultima, Rift) kayıt pencereleri aynı istemci penceresini kullanır; üst üste binmesin diye saatleri en az 20 dakika arayla verin. Tapınak ve savaş etkinliklerinin kendi çizelgesi ayrı sekmededir.'),
    h('div',{class:'row',style:'margin-bottom:12px'},h('button',{class:'btn primary',onclick:()=>evAutoDialog(d,null,reload)},ico('plus'),'Yeni zamanlama')),
    h('div',{class:'card'},h('div',{class:'tw'},d.schedules.length?h('table',{},
      h('thead',{},h('tr',{},['Saat','Etkinlik','Günler','Argüman','Durdur','Durum','Son çalışma',''].map(x=>h('th',{},x)))),
      h('tbody',{},d.schedules.map(s=>h('tr',{},
        h('td',{},h('b',{},evTime(s.hour,s.minute))),h('td',{},name(s.key),s.note?h('div',{class:'xs muted'},s.note):null),
        h('td',{class:'xs'},evDaysText(s.days)),h('td',{class:'mono xs'},s.arg||'—'),h('td',{class:'xs'},s.duration_min?s.duration_min+' dk':'—'),
        h('td',{},h('span',{class:'chip '+(s.enabled?'ok':'bad')},s.enabled?'açık':'kapalı')),
        h('td',{class:'xs muted'},s.last_run?when(s.last_run)+' — '+s.last_result:'—'),
        h('td',{class:'nowrap'},h('button',{class:'btn sm',onclick:()=>evAutoDialog(d,s,reload)},'Düzenle'),' ',
          h('button',{class:'btn sm danger',onclick:async()=>{if(!await confirmBox('Zamanlamayı sil',name(s.key)+' '+evTime(s.hour,s.minute),'Sil',true))return;try{await api('/events/auto/'+s.id+'/delete',{body:{}});toast('Silindi','ok');reload()}catch(_){}}},'Sil'))))))
      :h('div',{class:'empty'},'Zamanlama yok.'))));
}

async function evNative(box){
  const d=await api('/events');
  clear(box).append(h('div',{class:'hint',style:'margin-bottom:12px'},'Tapınak etkinlikleri (BDW, Chaos, Juraid), kale kuşatması ve ulus savaşlarının oyunun kendi çizelgesi. Değişiklik veritabanına yazılır, çalışan sunucuya yeniden yükleme gider.'),
    h('div',{class:'card'},h('div',{class:'tw'},h('table',{},h('thead',{},h('tr',{},['ID','Etkinlik','Durum','Saatler','Günler','Seviye',''].map(x=>h('th',{},x)))),
      h('tbody',{},d.schedule.map(e=>h('tr',{},h('td',{},e.id),h('td',{},h('b',{},e.name||('Tür '+e.type))),h('td',{},e.status?h('span',{class:'chip ok'},'açık'):h('span',{class:'chip bad'},'kapalı')),
        h('td',{},e.times.filter(t=>t[0]>=0).map(t=>evTime(t[0],t[1])).join(' · ')||'—'),h('td',{},e.days.map((v,i)=>v?DAYS[i]:null).filter(Boolean).join(' ')||'—'),h('td',{},e.min_level+'–'+e.max_level),
        h('td',{},h('button',{class:'btn sm',onclick:()=>eventDialog(e)},'Düzenle')))))))));
}

async function evHistory(box){
  const cm=await api('/commands');
  clear(box).append(h('div',{class:'card'},h('div',{class:'tw'},h('table',{},h('tbody',{},cm.map(c=>h('tr',{},h('td',{class:'xs muted nowrap'},when(c.created_at)),h('td',{},c.command),h('td',{class:'xs muted'},JSON.stringify(c.args)),h('td',{},h('span',{class:'chip '+(c.status==='done'?'ok':c.status==='failed'?'bad':'')},c.status)),h('td',{class:'xs muted'},c.result))))))));
}

PAGE.events=async function(main){
  let tab='live',data=await api('/events/live'),timer=null;
  const body=h('div',{});
  const status=h('span',{class:'chip'});
  const paint=()=>{
    status.className='chip '+(data.server_online?'ok':'bad');
    clear(status).append(data.server_online?'Oyun sunucusu çevrimiçi':'Oyun sunucusu kapalı — komutlar bekler');
  };
  const reload=async()=>{try{data=await api('/events/live',{quiet:true});paint();show()}catch(_){}};
  const show=()=>{
    if(tab==='live')evLive(body,data,reload);
    else if(tab==='auto')evAuto(body,data,reload);
    else if(tab==='native')evNative(body);
    else evHistory(body);
  };
  const seg=h('div',{class:'seg'},[['live','Canlı kontrol'],['auto','Otomatik zamanlama'],['native','Tapınak / savaş çizelgesi'],['hist','Komut geçmişi']].map(([k,l])=>
    h('button',{class:k===tab?'on':'',onclick:e=>{tab=k;[...seg.children].forEach(b=>b.classList.toggle('on',b===e.currentTarget));show()}},l)));
  main.append(head('Etkinlik Merkezi','Tüm etkinlikler: canlı durum, elle başlat/durdur, otomatik zamanlama',status),seg,h('div',{style:'height:14px'}),body);
  paint();show();
  // the live tab refreshes by itself while the page is on screen
  timer=setInterval(()=>{if(!main.isConnected){clearInterval(timer);return}if(tab==='live'&&!document.querySelector('.overlay'))reload()},4000);
};
