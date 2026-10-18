'use strict';
/* Sunucu Yönetimi: tabloları sunucuyu kapatmadan yeniden yükle (admin_command kuyruğu) ve
   korumalı "Sunucuyu yeniden başlat". Sonuçlar /server/command/{id} ile sorgulanır. */

const SRV_GROUPS=[
  ['Oyun içeriği',[
    ['drops','Drop tabloları','monster_item'],['groups','Item grupları','make_item_group'],['quest','Quest yardımcıları','quest helper'],
    ['item_exchange','Item takası (exchange)','item_exchange'],['npc_spawn','Yaratıklar (npc_spawn)','npc_spawn'],['pus','Power Up Store','cash shop']]],
  ['Etkinlik & ödül',[
    ['event_schedule','Etkinlik çizelgesi','event_schedule'],['event_rewards','Etkinlik ödülleri',''],['chaos_rewards','Chaos ödülleri',''],
    ['cinderella','Cinderella ödülleri',''],['zone_rewards','Bölge ödülleri',''],['collection_race','Collection Race tanımları',''],
    ['reward_rules','Ödül kuralları',''],['daily_quests','Günlük görevler',''],['send_messages','Giriş mesajları','']]],
  ['Sıralama & botlar',[
    ['rankings','Kullanıcı sıralaması',''],['field_bots','Saha botları',''],['bot_loadout','Bot donanımı',''],['info_panel','Bilgi paneli','']]],
];

async function srvWait(id){
  for(let i=0;i<14;i++){
    await new Promise(r=>setTimeout(r,700));
    let r;try{r=await api('/server/command/'+id,{quiet:true})}catch(_){continue}
    if(r.status==='done')return{ok:true,text:r.result||'tamam'};
    if(r.status==='failed'||r.status==='expired')return{ok:false,text:r.result||r.status};
  }
  return{ok:false,text:'Komut sıraya alındı; oyun sunucusu yanıt vermedi (kapalı olabilir).'};
}

PAGE.server=async function(main){
  let st=await api('/server/status');
  const status=h('span',{class:'chip'});
  const info=h('div',{class:'hint',style:'margin-bottom:12px'});
  const results=h('div',{class:'col',style:'gap:4px;margin-top:10px'});
  const paint=()=>{
    status.className='chip '+(st.server_online?'ok':'bad');
    clear(status).append(st.server_online?'Oyun sunucusu çevrimiçi':'Oyun sunucusu kapalı — komutlar bekler');
    clear(info).append('Çevrimiçi oyuncu: '+st.players+(st.restart_in!=null?' · Yeniden başlatma '+st.restart_in+' sn sonra':''));
  };
  const refresh=async()=>{try{st=await api('/server/status',{quiet:true});paint()}catch(_){}};

  const log=(name,r)=>{
    results.prepend(h('div',{class:'xs'},h('span',{class:'chip '+(r.ok?'ok':'bad')},r.ok?'tamam':'hata'),' ',h('b',{},name),' — ',r.text));
  };
  async function run(key,name,btn){
    btn.disabled=true;
    try{
      const id=(await api('/server/reload',{body:{what:key}})).id;
      toast(name+': komut gönderildi','ok');
      const r=await srvWait(id);
      log(name,r);toast(name+': '+r.text,r.ok?'ok':'err');
    }catch(_){}
    btn.disabled=false;
  }
  const cards=SRV_GROUPS.flatMap(([g,list])=>[h('h3',{style:'margin:18px 0 8px'},g),
    h('div',{class:'ev-grid'},list.map(([key,name])=>{
      const b=h('button',{class:'btn sm primary'},ico('cog'),'Yeniden yükle');
      b.onclick=()=>run(key,name,b);
      return h('div',{class:'card ev-card'},h('b',{},name),h('div',{style:'margin-top:10px'},b));
    }))]);

  const allBtn=h('button',{class:'btn primary'},'Temel tabloların hepsini yükle');
  allBtn.onclick=()=>run('all','Tüm temel tablolar',allBtn);

  // restart
  const delay=h('select',{},[[0,'Hemen'],[30,'30 sn'],[60,'1 dk'],[120,'2 dk'],[300,'5 dk']].map(([v,l])=>h('option',{value:v},l)));
  delay.value='60';
  const rbtn=h('button',{class:'btn danger'},'Sunucuyu yeniden başlat');
  rbtn.onclick=async()=>{
    if(!st.restart_possible){toast(st.restart_reason||'Bu panelden yeniden başlatma yapılamıyor','err');return}
    const sec=Number(delay.value);
    if(!await confirmBox('Sunucuyu yeniden başlat','Oyun sunucusu '+(sec?sec+' saniye sonra ':'hemen ')+'yeniden başlatılacak; '+st.players+' çevrimiçi oyuncu bağlantıyı kaybeder. '+(sec?'Oyunculara geri sayım duyurusu gider.':''),'Yeniden başlat',true))return;
    try{
      const d=await api('/server/restart',{body:{delay_sec:sec}});
      if(!d.ok){toast(d.error||'Başlatılamadı','err');return}
      toast('Yeniden başlatma planlandı','ok');refresh();
    }catch(_){}
  };
  const cbtn=h('button',{class:'btn'},'Planlı yeniden başlatmayı iptal et');
  cbtn.onclick=async()=>{try{const d=await api('/server/restart/cancel',{body:{}});toast(d.ok?'İptal edildi':(d.error||'İptal edilemedi'),d.ok?'ok':'err');refresh()}catch(_){}};
  const restartCard=h('div',{class:'card',style:'margin-top:18px'},h('h3',{},'Sunucuyu yeniden başlat'),
    h('div',{class:'hint',style:'margin:8px 0 12px'},st.restart_possible?'Geri sayım boyunca oyunculara duyuru gider. İşlem denetim kaydına yazılır.':('Bu panelden yeniden başlatılamıyor: '+st.restart_reason)),
    h('div',{class:'row',style:'gap:8px;flex-wrap:wrap'},field('Geri sayım',delay),rbtn,cbtn));
  if(!st.restart_possible)rbtn.disabled=true;

  main.append(head('Sunucu Yönetimi','Tabloları sunucuyu kapatmadan yeniden yükle, sunucuyu yeniden başlat',status),info,
    h('div',{class:'row'},allBtn),...cards,
    h('div',{class:'hint',style:'margin-top:14px'},'Item, büyü (magic) ve NPC şablonu tabloları oyunda canlı yeniden yüklenemez; bunlar için sunucuyu yeniden başlatın.'),
    results,restartCard);
  paint();
  const timer=setInterval(()=>{if(!main.isConnected){clearInterval(timer);return}refresh()},5000);
};
