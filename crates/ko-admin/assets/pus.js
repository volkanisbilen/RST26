'use strict';
/* Power Up Store: ESN codes (KC / TL e-pins) – generate, track who used which code, KC/TL ledger. */

const PUS_RESULT={ok:['Kullanıldı','ok'],invalid:['Geçersiz','bad'],used:['Zaten kullanılmış','bad'],disabled:['Pasif / süresi dolmuş','bad'],blocked:['Çok fazla hata','bad']};
const PUS_REASON={esn:'ESN kodu',give:'Bakiye eklendi',buy_kc:'KC ile satın alma',buy_tl:'TL ile satın alma'};
const pusNum=n=>Number(n||0).toLocaleString('tr-TR');
const pusAmount=(kc,tl)=>[kc?pusNum(kc)+' KC':null,tl?pusNum(tl)+' TL':null].filter(Boolean).join(' + ')||'—';

function pusDownload(name,text){
  const a=document.createElement('a');a.href=URL.createObjectURL(new Blob([text],{type:'text/plain;charset=utf-8'}));a.download=name;a.click();
  setTimeout(()=>URL.revokeObjectURL(a.href),2000);
}

function pusGenerate(done){
  const label=h('input',{placeholder:'Örn. Ekim kampanyası'});
  const kc=h('input',{type:'number',min:'0',value:'0'}),tl=h('input',{type:'number',min:'0',value:'0'});
  const qty=h('input',{type:'number',min:'1',max:'2000',value:'10'});
  const days=h('input',{type:'number',min:'0',value:'0'});
  const note=h('input',{placeholder:'Not (kime verildi, nereden satıldı…)'});
  modal('ESN kodu üret',h('div',{class:'col'},h('div',{class:'form'},
    field('Parti adı',label),field('Adet (en fazla 2000)',qty),field('KC değeri',kc),field('TL değeri',tl),
    field('Geçerlilik (gün, 0 = süresiz)',days)),field('Not',note,true)),[
    {label:'İptal',cls:'ghost'},
    {label:'Üret',cls:'primary',keep:true,fn:async close=>{
      const body={label:label.value.trim(),kc:+kc.value||0,tl:+tl.value||0,quantity:+qty.value||0,expires_days:+days.value||0,note:note.value.trim()};
      if(!body.kc&&!body.tl){toast('KC veya TL değeri gir','err');return true}
      try{
        const r=await api('/pus/generate',{body});
        const ex=await api('/pus/batch/'+r.batch+'/export');
        pusDownload('esn_parti_'+r.batch+'.txt',ex.codes.map(c=>c.code).join('\r\n')+'\r\n');
        toast(body.quantity+' kod üretildi, liste indirildi','ok');close();done()
      }catch(_){}return true}}]);
}

async function pusBatches(box,reload,openCodes){
  const d=await api('/pus/summary');
  clear(box).append(
    h('div',{class:'row',style:'gap:10px;flex-wrap:wrap;margin-bottom:12px'},
      [['Üretilen kod',pusNum(d.total)],['Kullanılan',pusNum(d.used)],['Kalan',pusNum(d.total-d.used)],
       ['Dağıtılan KC',pusNum(d.kc_used)+' / '+pusNum(d.kc_issued)],['Dağıtılan TL',pusNum(d.tl_used)+' / '+pusNum(d.tl_issued)]]
      .map(([k,v])=>h('div',{class:'card',style:'padding:10px 16px;min-width:130px'},h('div',{class:'xs muted'},k),h('b',{},v)))),
    h('div',{class:'card'},h('div',{class:'tw'},d.batches.length?h('table',{},
      h('thead',{},h('tr',{},['#','Parti','Değer','Adet','Kullanılan','Üreten','Tarih','Geçerlilik','Not',''].map(x=>h('th',{},x)))),
      h('tbody',{},d.batches.map(b=>h('tr',{},h('td',{},b.id),h('td',{},h('b',{},b.label||'—')),h('td',{},pusAmount(b.kc,b.tl)),
        h('td',{},b.quantity),h('td',{},h('span',{class:'chip '+(b.used?'ok':'')},b.used+' / '+b.quantity)),
        h('td',{},b.created_by),h('td',{class:'xs muted nowrap'},when(b.created_at)),
        h('td',{class:'xs muted nowrap'},b.expires_at?when(b.expires_at):'süresiz'),h('td',{class:'xs muted'},b.note),
        h('td',{class:'nowrap'},
          h('button',{class:'btn sm',onclick:()=>openCodes(b.id)},'Kodlar'),' ',
          h('button',{class:'btn sm',onclick:async()=>{const ex=await api('/pus/batch/'+b.id+'/export');pusDownload('esn_parti_'+b.id+'.txt',ex.codes.map(c=>c.code).join('\r\n')+'\r\n')}},'İndir'),' ',
          b.used?null:h('button',{class:'btn sm danger',onclick:async()=>{
            if(!await confirmBox('Parti #'+b.id+' silinsin mi?','Kullanılmamış '+b.quantity+' kodun hepsi silinir.','Sil',true))return;
            try{await api('/pus/batch/'+b.id+'/delete',{body:{}});toast('Silindi','ok');reload()}catch(_){}}},'Sil')))))):
      h('p',{class:'muted',style:'padding:14px'},'Henüz kod üretilmedi.'))));
}

async function pusCodes(box,state){
  const q=h('input',{placeholder:'Kod, hesap, karakter veya not ara',style:'flex:1;min-width:220px',value:state.q||''});
  const st=h('select',{},[['','Hepsi'],['unused','Kullanılmamış'],['used','Kullanılmış'],['disabled','Pasif']].map(([v,l])=>h('option',{value:v},l)));st.value=state.state||'';
  const list=h('div',{});
  let offset=0;
  const load=async()=>{
    const p=new URLSearchParams({state:st.value,q:q.value.trim(),offset});
    if(state.batch)p.set('batch',state.batch);
    const d=await api('/pus/codes?'+p);
    clear(list).append(
      h('p',{class:'muted xs'},pusNum(d.total)+' kod'+(state.batch?' (parti #'+state.batch+')':'')),
      h('div',{class:'card'},h('div',{class:'tw'},h('table',{},
        h('thead',{},h('tr',{},['Kod','Parti','Değer','Durum','Kullanan hesap','Karakter','Tarih','IP','Not',''].map(x=>h('th',{},x)))),
        h('tbody',{},d.rows.map(c=>{
          const exp=c.expires_at&&new Date(c.expires_at)<new Date();
          const chip=c.used_at?h('span',{class:'chip ok'},'kullanıldı'):c.disabled?h('span',{class:'chip bad'},'pasif'):exp?h('span',{class:'chip bad'},'süresi doldu'):h('span',{class:'chip blue'},'hazır');
          return h('tr',{},h('td',{style:'font-family:monospace'},c.code,' ',h('a',{href:'javascript:void 0',title:'Kopyala',onclick:()=>{navigator.clipboard&&navigator.clipboard.writeText(c.code);toast('Kopyalandı','ok')}},ico('copy'))),
            h('td',{class:'xs'},'#'+c.batch_id+(c.label?' '+c.label:'')),h('td',{},pusAmount(c.kc,c.tl)),h('td',{},chip),
            h('td',{},c.used_account?h('a',{href:'#/account/'+encodeURIComponent(c.used_account)},c.used_account):'—'),
            h('td',{},c.used_char||'—'),h('td',{class:'xs muted nowrap'},c.used_at?when(c.used_at):'—'),h('td',{class:'xs muted'},c.used_ip||''),
            h('td',{class:'xs muted'},c.note),
            h('td',{class:'nowrap'},c.used_at?null:h('button',{class:'btn sm '+(c.disabled?'':'danger'),onclick:async()=>{
              try{await api('/pus/code/'+c.id,{body:{disabled:!c.disabled}});load()}catch(_){}}},c.disabled?'Aç':'Pasifleştir'),' ',
              h('button',{class:'btn sm',onclick:()=>{
                const n=h('input',{value:c.note,style:'width:100%'});
                modal('Kod notu',h('div',{},n),[{label:'İptal',cls:'ghost'},{label:'Kaydet',cls:'primary',keep:true,fn:async close=>{
                  try{await api('/pus/code/'+c.id,{body:{note:n.value}});close();load()}catch(_){}return true}}])}},'Not')));
        }))))),
      h('div',{class:'row',style:'margin-top:8px'},
        offset>0?h('button',{class:'btn sm',onclick:()=>{offset=Math.max(0,offset-100);load()}},'‹ Önceki'):null,
        offset+100<d.total?h('button',{class:'btn sm',onclick:()=>{offset+=100;load()}},'Sonraki ›'):null));
  };
  const go=()=>{offset=0;load()};
  q.addEventListener('input',debounce(go,300));st.addEventListener('change',go);
  clear(box).append(h('div',{class:'row',style:'margin-bottom:10px;gap:8px'},q,st),list);
  load();
}

async function pusLedger(box){
  const q=h('input',{placeholder:'Hesap veya karakter adı (boş = hepsi)',style:'flex:1;min-width:220px'});
  const list=h('div',{});
  const load=async()=>{
    const d=await api('/pus/ledger?account='+encodeURIComponent(q.value.trim()));
    clear(list).append(h('div',{class:'card'},h('div',{class:'tw'},d.rows.length?h('table',{},
      h('thead',{},h('tr',{},['Tarih','Hesap','Karakter','KC','TL','Bakiye','Neden','Kod / ref'].map(x=>h('th',{},x)))),
      h('tbody',{},d.rows.map(r=>h('tr',{},h('td',{class:'xs muted nowrap'},when(r.at)),
        h('td',{},h('a',{href:'#/account/'+encodeURIComponent(r.account)},r.account)),h('td',{},r.char_name),
        h('td',{style:'color:'+(r.kc_delta<0?'#f87171':r.kc_delta>0?'#4ade80':'')},r.kc_delta?(r.kc_delta>0?'+':'')+pusNum(r.kc_delta):''),
        h('td',{style:'color:'+(r.tl_delta<0?'#f87171':r.tl_delta>0?'#4ade80':'')},r.tl_delta?(r.tl_delta>0?'+':'')+pusNum(r.tl_delta):''),
        h('td',{class:'xs muted'},pusNum(r.kc_after)+' KC / '+pusNum(r.tl_after)+' TL'),
        h('td',{},PUS_REASON[r.reason]||r.reason),h('td',{class:'xs',style:'font-family:monospace'},r.ref))))):
      h('p',{class:'muted',style:'padding:14px'},'Kayıt yok.'))));
  };
  q.addEventListener('input',debounce(load,300));
  clear(box).append(h('div',{class:'row',style:'margin-bottom:10px'},q),list);load();
}

async function pusAttempts(box){
  const q=h('input',{placeholder:'Hesap, karakter veya IP (boş = hepsi)',style:'flex:1;min-width:220px'});
  const list=h('div',{});
  const load=async()=>{
    const d=await api('/pus/attempts?account='+encodeURIComponent(q.value.trim()));
    clear(list).append(h('div',{class:'card'},h('div',{class:'tw'},d.rows.length?h('table',{},
      h('thead',{},h('tr',{},['Tarih','Hesap','Karakter','IP','Girilen kod','Sonuç'].map(x=>h('th',{},x)))),
      h('tbody',{},d.rows.map(r=>{const [t,c]=PUS_RESULT[r.result]||[r.result,''];
        return h('tr',{},h('td',{class:'xs muted nowrap'},when(r.at)),h('td',{},h('a',{href:'#/account/'+encodeURIComponent(r.account)},r.account)),
          h('td',{},r.char_name),h('td',{class:'xs muted'},r.ip),h('td',{style:'font-family:monospace'},r.code),h('td',{},h('span',{class:'chip '+c},t)))}))):
      h('p',{class:'muted',style:'padding:14px'},'Deneme yok.'))));
  };
  q.addEventListener('input',debounce(load,300));
  clear(box).append(h('div',{class:'row',style:'margin-bottom:10px'},q),list);load();
}

PAGE.pus=async function(main){
  let tab='batches';
  const body=h('div',{});
  const show=(extra)=>{
    if(tab==='batches')pusBatches(body,()=>show(),id=>{tab='codes';sync();show({batch:id})});
    else if(tab==='codes')pusCodes(body,extra||{});
    else if(tab==='ledger')pusLedger(body);
    else pusAttempts(body);
  };
  const tabs=[['batches','Partiler'],['codes','Tüm kodlar'],['ledger','KC / TL hareketleri'],['attempts','Deneme kayıtları']];
  const seg=h('div',{class:'seg'},tabs.map(([k,l])=>h('button',{'data-k':k,class:k===tab?'on':'',onclick:()=>{tab=k;sync();show()}},l)));
  const sync=()=>[...seg.children].forEach(b=>b.classList.toggle('on',b.dataset.k===tab));
  main.append(head('KC / TL Kodları (ESN)','Power Up Store için kod üret; hangi hesap hangi kodu kullandı takip et',
    h('button',{class:'btn primary',onclick:()=>pusGenerate(()=>{tab='batches';sync();show()})},'Kod üret')),seg,h('div',{style:'height:14px'}),body);
  show();
};
