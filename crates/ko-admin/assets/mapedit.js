'use strict';
/* ───────── map editor: canvas, pan/zoom, drag spawns, rectangle select ───────── */
PAGE.maps=async function(main,[zoneArg]){
  const zones=await api('/zones');
  let zone=Number(zoneArg)||(zones.find(z=>z.zone===21)||zones[0]).zone;
  let info=null,spawns=[],bg=null,bgKind='map';
  const view={k:1,ox:0,oy:0};
  const S={monsters:true,npcs:true,warps:true,regene:false,flipZ:false,filter:'',sel:new Set(),hover:null,place:null,rect:null,dirty:true};
  const canvas=h('canvas');const stat=h('div',{class:'mapstat'},'—');
  const panel=h('div',{class:'panel',style:'position:sticky;top:78px;max-height:calc(100vh - 100px);overflow:auto'});
  const box=h('div',{class:'mapbox'},canvas,stat);
  const zsel=h('select',{onchange:function(){zone=Number(this.value);location.hash='#/maps/'+zone}},zones.map(z=>h('option',{value:z.zone,selected:z.zone===zone},z.zone+' · '+z.name+' ('+z.spawns+')')));
  const tools=h('div',{class:'maptools'});box.append(tools);
  const ctx2=()=>canvas.getContext('2d');
  const W=()=>info?info.width:2048,H=()=>info?info.height:2048;
  const sx=x=>view.ox+x*view.k;
  const sy=z=>view.oy+(S.flipZ?(H()-z):z)*view.k;
  const wx=px=>(px-view.ox)/view.k;
  const wz=py=>{const v=(py-view.oy)/view.k;return S.flipZ?H()-v:v};
  function fit(){
    const r=canvas.getBoundingClientRect();if(!r.width)return;
    view.k=Math.min(r.width/W(),r.height/H())*0.96;view.ox=(r.width-W()*view.k)/2;view.oy=(r.height-H()*view.k)/2;S.dirty=true;
  }
  function resize(){const r=box.getBoundingClientRect();const d=devicePixelRatio||1;canvas.width=r.width*d;canvas.height=r.height*d;S.dirty=true}
  const colorFor=s=>{if(!s.monster)return '#22d3ee';const l=Math.min(1,(s.level||1)/83);return `hsl(${215-l*55} 90% ${68-l*10}%)`};
  const visible=s=>(s.monster?S.monsters:S.npcs);
  const matches=s=>!S.filter||String(s.name).toLowerCase().includes(S.filter)||String(s.npc)===S.filter;
  function draw(){
    if(!document.body.contains(canvas))return;
    if(!S.dirty)return requestAnimationFrame(draw);S.dirty=false;
    const c=ctx2(),d=devicePixelRatio||1,r=canvas.getBoundingClientRect();
    c.setTransform(d,0,0,d,0,0);c.clearRect(0,0,r.width,r.height);
    c.fillStyle='#050c20';c.fillRect(0,0,r.width,r.height);
    const x0=sx(0),y0=sy(S.flipZ?H():0),w=W()*view.k,hh=H()*view.k;
    if(bg&&bg.complete&&bg.naturalWidth){c.save();c.globalAlpha=.95;
      if(S.flipZ&&bgKind==='terrain'){c.translate(x0,y0+hh);c.scale(1,-1);c.drawImage(bg,0,0,w,hh)}
      else c.drawImage(bg,x0,y0,w,hh);c.restore()}
    c.strokeStyle='#3b82f655';c.lineWidth=1;c.strokeRect(x0,y0,w,hh);
    // grid every 100 m
    c.strokeStyle='#3b82f61a';c.fillStyle='#6b88c8';c.font='10px Segoe UI';
    const step=view.k*100<28?500:100;
    for(let g=0;g<=W();g+=step){const x=sx(g);c.beginPath();c.moveTo(x,y0);c.lineTo(x,y0+hh);c.stroke();if(view.k*step>40)c.fillText(g,x+2,y0+10)}
    for(let g=0;g<=H();g+=step){const y=sy(g);c.beginPath();c.moveTo(x0,y);c.lineTo(x0+w,y);c.stroke();if(view.k*step>40)c.fillText(g,x0+2,y-2)}
    if(S.regene&&info)for(const p of info.regene){c.fillStyle='#fff';c.fillRect(sx(p.x)-2,sy(p.z)-2,4,4)}
    for(const s of spawns){
      if(!visible(s))continue;const x=sx(s.x),y=sy(s.z);if(x<-20||y<-20||x>r.width+20||y>r.height+20)continue;
      const m=matches(s),selected=S.sel.has(s.id);
      c.globalAlpha=m?1:.18;
      if(selected||S.hover===s.id){c.strokeStyle=selected?'#fff':'#9cd1ff';c.lineWidth=1.5;c.beginPath();c.arc(x,y,Math.max(5,s.range*view.k),0,7);c.stroke()}
      c.fillStyle=selected?'#fff':colorFor(s);
      c.beginPath();
      if(s.monster)c.arc(x,y,selected?5:3.6,0,7);else{c.moveTo(x,y-5);c.lineTo(x+5,y);c.lineTo(x,y+5);c.lineTo(x-5,y);c.closePath()}
      c.fill();c.globalAlpha=1;
    }
    if(S.rect){c.fillStyle='#3b82f622';c.strokeStyle='#60a5fa';c.lineWidth=1;const r2=S.rect;c.fillRect(r2.x,r2.y,r2.w,r2.h);c.strokeRect(r2.x,r2.y,r2.w,r2.h)}
    if(S.place&&S.place.at){c.strokeStyle='#22d3ee';c.beginPath();c.arc(S.place.at.x,S.place.at.y,9,0,7);c.stroke()}
    requestAnimationFrame(draw);
  }
  function hit(px,py){
    let best=null,bd=100;
    for(const s of spawns){if(!visible(s))continue;const dx=sx(s.x)-px,dy=sy(s.z)-py,dd=dx*dx+dy*dy;if(dd<bd){bd=dd;best=s}}
    return best;
  }
  /* pointer handling */
  let drag=null;
  const pos=e=>{const r=canvas.getBoundingClientRect();return {x:e.clientX-r.left,y:e.clientY-r.top}};
  canvas.addEventListener('wheel',e=>{
    e.preventDefault();const p=pos(e);const f=e.deltaY<0?1.18:1/1.18;const nk=Math.max(.05,Math.min(40,view.k*f));
    const wxx=wx(p.x),wzz=wz(p.y);view.k=nk;view.ox=p.x-wxx*nk;view.oy=p.y-(S.flipZ?(H()-wzz):wzz)*nk;S.dirty=true;
  },{passive:false});
  canvas.addEventListener('pointerdown',e=>{
    canvas.setPointerCapture(e.pointerId);const p=pos(e);
    if(S.place){const x=Math.round(wx(p.x)),z=Math.round(wz(p.y));placeSpawn(x,z);return}
    const s=hit(p.x,p.y);
    if(e.shiftKey&&!s){drag={t:'rect',x:p.x,y:p.y};S.rect={x:p.x,y:p.y,w:0,h:0};return}
    if(s){
      if(e.ctrlKey||e.metaKey){S.sel.has(s.id)?S.sel.delete(s.id):S.sel.add(s.id)}
      else if(!S.sel.has(s.id)){S.sel=new Set([s.id])}
      drag={t:'spawn',id:s.id,sx:p.x,sy:p.y,moved:false,orig:new Map(spawns.filter(q=>S.sel.has(q.id)).map(q=>[q.id,[q.x,q.z]]))};
      drawPanel();S.dirty=true;return;
    }
    drag={t:'pan',x:p.x,y:p.y,ox:view.ox,oy:view.oy};canvas.classList.add('grabbing');
  });
  canvas.addEventListener('pointermove',e=>{
    const p=pos(e);const wxx=wx(p.x),wzz=wz(p.y);
    stat.textContent='X '+wxx.toFixed(0)+'  Z '+wzz.toFixed(0)+'  ·  '+spawns.filter(visible).length+' spawn';
    if(S.place){S.place.at=p;S.dirty=true;return}
    if(!drag){const s=hit(p.x,p.y);const hid=s?s.id:null;if(hid!==S.hover){S.hover=hid;S.dirty=true;canvas.title=s?s.name+' (#'+s.npc+') · spawn '+s.id:''}return}
    if(drag.t==='pan'){view.ox=drag.ox+p.x-drag.x;view.oy=drag.oy+p.y-drag.y;S.dirty=true}
    else if(drag.t==='rect'){S.rect={x:Math.min(drag.x,p.x),y:Math.min(drag.y,p.y),w:Math.abs(p.x-drag.x),h:Math.abs(p.y-drag.y)};S.dirty=true}
    else if(drag.t==='spawn'){
      const dx=(p.x-drag.sx)/view.k,dz=(S.flipZ?-1:1)*(p.y-drag.sy)/view.k;
      if(Math.abs(p.x-drag.sx)+Math.abs(p.y-drag.sy)>3)drag.moved=true;
      if(drag.moved){for(const q of spawns)if(drag.orig.has(q.id)){const o=drag.orig.get(q.id);q.x=Math.round(o[0]+dx);q.z=Math.round(o[1]+dz)}S.dirty=true}
    }
  });
  canvas.addEventListener('pointerup',async e=>{
    canvas.classList.remove('grabbing');
    if(!drag)return;const d=drag;drag=null;
    if(d.t==='rect'){const r=S.rect;S.rect=null;S.sel=new Set(spawns.filter(s=>visible(s)&&matches(s)&&sx(s.x)>=r.x&&sx(s.x)<=r.x+r.w&&sy(s.z)>=r.y&&sy(s.z)<=r.y+r.h).map(s=>s.id));drawPanel();S.dirty=true;return}
    if(d.t==='spawn'&&d.moved){
      const moved=spawns.filter(q=>d.orig.has(q.id));
      try{for(const q of moved)await api('/spawns/'+q.id,{body:{x:q.x,z:q.z}});toast(moved.length+' spawn taşındı (sunucu yeniden başlatılınca uygulanır)','ok')}
      catch(_){for(const q of moved){const o=d.orig.get(q.id);q.x=o[0];q.z=o[1]}}
      drawPanel();S.dirty=true;
    }
  });
  document.addEventListener('keydown',function onKey(e){
    if(!document.body.contains(canvas)){document.removeEventListener('keydown',onKey);return}
    if(e.key==='Escape'){S.place=null;S.sel=new Set();canvas.classList.remove('place');drawPanel();S.dirty=true}
    if(e.key==='Delete'&&S.sel.size&&!/INPUT|TEXTAREA|SELECT/.test(document.activeElement.tagName))delSelected();
  });
  /* data */
  async function load(){
    info=await api('/zone/'+zone+'/info');
    spawns=await api('/spawns?zone='+zone);
    bgKind=info.image?'map':'terrain';setBg();
    fit();S.sel=new Set();drawPanel();drawTools();S.dirty=true;
  }
  function setBg(){
    bg=new Image();const url=bgKind==='map'?BASE+'/zone/'+zone+'/image':BASE+'/zone/'+zone+'/terrain.png';
    bg.onload=()=>{S.dirty=true};bg.onerror=()=>{if(bgKind==='map'&&info.terrain){bgKind='terrain';setBg()}};bg.src=url;
  }
  function drawTools(){
    clear(tools).append(
      h('button',{class:'btn sm',onclick:fit},'Sığdır'),
      h('button',{class:'btn sm',onclick:()=>{S.flipZ=!S.flipZ;fit();drawTools()}},S.flipZ?'Z ↑':'Z ↓'),
      info.image&&info.terrain?h('button',{class:'btn sm',onclick:()=>{bgKind=bgKind==='map'?'terrain':'map';setBg();drawTools()}},bgKind==='map'?'Arazi':'Harita'):null,
      h('button',{class:'btn sm '+(S.monsters?'primary':''),onclick:()=>{S.monsters=!S.monsters;drawTools();S.dirty=true}},'Canavar'),
      h('button',{class:'btn sm '+(S.npcs?'primary':''),onclick:()=>{S.npcs=!S.npcs;drawTools();S.dirty=true}},'NPC'),
      h('button',{class:'btn sm '+(S.regene?'primary':''),onclick:()=>{S.regene=!S.regene;drawTools();S.dirty=true}},'Doğma noktası'));
  }
  /* side panel */
  function drawPanel(){
    clear(panel);
    const sel=spawns.filter(s=>S.sel.has(s.id));
    if(S.place)return panel.append(h('h4',{},'Yeni spawn'),h('p',{},ico('plus'),' ',S.place.npc.name+' (#'+S.place.npc.id+') — haritada yerleştireceğin noktaya tıkla.'),h('button',{class:'btn',onclick:()=>{S.place=null;canvas.classList.remove('place');drawPanel()}},'Vazgeç'));
    if(sel.length===1)return singlePanel(sel[0]);
    if(sel.length>1)return panel.append(h('h4',{},sel.length+' spawn seçili'),h('p',{class:'muted sm'},'Seçili işaretlerden birini sürükleyerek hepsini birlikte taşıyabilirsin.'),
      h('div',{class:'row'},h('button',{class:'btn danger',onclick:delSelected},ico('trash'),'Seçilenleri sil'),h('button',{class:'btn ghost',onclick:()=>{S.sel=new Set();drawPanel();S.dirty=true}},'Seçimi kaldır')));
    // overview
    const f=h('input',{placeholder:'Haritada ada/SID göre bul…',value:S.filter,style:'width:100%'});f.addEventListener('input',debounce(()=>{S.filter=f.value.trim().toLowerCase();S.dirty=true},150));
    const q=h('input',{placeholder:'Yeni spawn için NPC/canavar ara…',style:'width:100%'});const res=h('div',{class:'mlist'});
    q.addEventListener('input',debounce(async()=>{
      const r=q.value.trim()?await api('/npc-search?q='+encodeURIComponent(q.value.trim()),{quiet:true}).catch(()=>[]):[];
      clear(res).append(...r.map(n=>h('div',{class:'mrow',onclick:()=>{S.place={npc:n};canvas.classList.add('place');drawPanel()}},ico(n.monster?'sword':'users'),h('div',{},h('b',{},n.name),h('br'),h('small',{},'#'+n.id+' · '+(n.monster?'canavar':'NPC')+' · Lv '+n.level)))));
    },250));
    panel.append(h('h4',{},'Harita'),zsel,h('div',{class:'legend',style:'margin:12px 0'},h('span',{},h('i',{style:'background:#6fa8ff'}),'Canavar'),h('span',{},h('i',{style:'background:#22d3ee;border-radius:2px'}),'NPC'),h('span',{},h('i',{style:'background:#fff'}),'Seçili')),
      h('label',{class:'f'},'Bul / vurgula',f),h('h4',{style:'margin-top:16px'},'Yeni spawn ekle'),q,res,
      h('div',{class:'hint',style:'margin-top:14px'},'Sürükle: kaydır · Tekerlek: yakınlaştır · İşarete tıkla: seç · İşareti sürükle: taşı · Shift+sürükle: çoklu seçim · Ctrl+tık: seçime ekle/çıkar · Delete: sil'),
      h('div',{class:'hint warn',style:'margin-top:10px'},'Spawn değişiklikleri veritabanına yazılır; oyun sunucusu haritaları açılışta okuduğu için yeniden başlatma gerekir.'));
  }
  function singlePanel(s){
    const F={npc:h('input',{type:'number',value:s.npc}),x:h('input',{type:'number',value:s.x}),z:h('input',{type:'number',value:s.z}),num:h('input',{type:'number',value:s.num}),range:h('input',{type:'number',value:s.range}),regen:h('input',{type:'number',value:s.regen}),dir:h('input',{type:'number',value:s.dir}),act:h('input',{type:'number',value:s.act})};
    panel.append(h('h4',{},'Spawn #'+s.id),
      h('div',{class:'row',style:'margin-bottom:10px'},ico(s.monster?'sword':'users'),h('b',{},s.name),h('span',{class:'chip'},'#'+s.npc),h('span',{class:'chip'},'Lv '+s.level)),
      h('div',{class:'form'},field('NPC/canavar SID',F.npc),field('X',F.x),field('Z',F.z),field('Adet',F.num),field('Yarıçap',F.range),field('Yeniden doğma (sn)',F.regen),field('Yön',F.dir),field('Davranış (act)',F.act)),
      h('div',{class:'row',style:'margin-top:14px'},
        h('button',{class:'btn primary',onclick:async()=>{
          const p={};for(const k of Object.keys(F)){const v=Number(F[k].value);if(v!==s[k])p[k]=v}
          if(!Object.keys(p).length)return toast('Değişiklik yok');
          try{await api('/spawns/'+s.id,{body:p});Object.assign(s,p);toast('Kaydedildi','ok');if(p.npc)load();else{drawPanel();S.dirty=true}}catch(_){}}},'Kaydet'),
        h('button',{class:'btn',onclick:async()=>{try{const d=await api('/spawns',{body:{zone,npc:s.npc,monster:s.monster,x:s.x+10,z:s.z+10,num:s.num,range:s.range,regen:s.regen,dir:s.dir,act:s.act}});toast('Kopyalandı','ok');await load();S.sel=new Set([d.id]);drawPanel()}catch(_){}}},ico('copy'),'Kopyala'),
        h('button',{class:'btn danger',onclick:delSelected},ico('trash'))),
      h('div',{class:'row',style:'margin-top:12px'},h('a',{href:'#/drops/'+s.npc},s.monster?'Drop tablosuna git →':'')));
  }
  async function delSelected(){
    const ids=[...S.sel];if(!ids.length)return;
    if(!await confirmBox('Spawn sil',ids.length+' spawn kalıcı olarak silinecek.','Sil',true))return;
    try{for(const id of ids)await api('/spawns/'+id+'?q=harita',{method:'DELETE'});toast('Silindi','ok');await load()}catch(_){}
  }
  async function placeSpawn(x,z){
    const n=S.place.npc;S.place=null;canvas.classList.remove('place');
    try{const d=await api('/spawns',{body:{zone,npc:n.id,monster:n.monster,x,z}});toast('Spawn eklendi','ok');await load();S.sel=new Set([d.id]);drawPanel()}catch(_){drawPanel()}
  }
  main.append(head('Harita Editörü','Gerçek SMD arazisi / harita görseli üzerinde canavar ve NPC spawn noktalarını sürükle-bırak düzenle.'),
    h('div',{class:'mapwrap'},box,panel));
  new ResizeObserver(()=>{resize();fit()}).observe(box);
  resize();draw();await load();
};
