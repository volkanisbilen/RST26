
# Manes Survival ve Knight Royale — Claude için teknik uygulama raporu

**Araştırma tarihi:** 12 Ekim 2026  
**Hedef:** KORustGameServer çalışma alanına iki Knight Online etkinliğinin uygulanması  
**Not:** Knight Online resmi etkinlik duyurularında Manes Survival modu çoğunlukla **Arena of Manes / Manes' Arena** adıyla geçer.

## Proje bağlamı ve Claude'dan beklenti

Bu repo Rust tabanlı, v2603 istemcisiyle uyumlu bir Knight Online sunucusudur. README; combat, party, spawn/monster, NPC Lua dialogları, event modülleri ve bazı özel etkinliklerden söz ediyor. Bunların gerçek kod durumunu Claude kendisi keşfetmeli. Rapordaki etkinlik UI'ı, skill butonu, map, monster, item veya opcode'larının mevcut client/server'da bulunduğu varsayılmamalı.

Uygulamada güvenli yaklaşım:

- Başlamadan repo talimatlarını, event/instance/queue altyapısını, combat/skill/inventory/party/DB sistemlerini ve protocol/client kanıtlarını incele.
- Yeni opcode veya desteklenmeyen UI davranışı uydurma. Mevcut protokolün yapabildiği en yakın deneyimi kullan; sınırlamayı açıkça belirt.
- Zaman, ödül, kapasite, gas ayarları ve loot konfigüre edilsin. Aşağıdaki resmî değerler referans olarak ele alınsın, özel sunucu dengesi diye körlemesine kopyalanmasın.
- State server-authoritative olsun; skor, XP, kill attribution, item, gaz, cooldown ve sonuç istemcinin iddiasına göre belirlenmesin.

---

# 1. Manes Survival / Arena of Manes

## 1.1 Konsept

Kalabalık PvPvE arenada tüm oyuncular eşit ve düşük güçte başlar. Oyuncular yaratık keserek seviye/XP toplar, seviye atlayınca aktif/pasif beceri ve HP/MP iksiri seçer, sandık ve yaratıklardan ekipman/loot edinir. İlk 3 dakika PvP kapalıdır; bu arada oyuncular PvE yapıp gelişir. PvP açıldıktan sonra skor sıralaması için oyuncu öldürme ve seviye yükseltme puan üretir. Arenanın merkezindeki Dark Dragon, önemli final hedefidir. Dark Dragon'u öldüren oyuncu sıralamadan çıkarılır ve birincilik/Manes Orb ödülü kazanır. Ölüm oyuncuyu oyundan tamamen çıkarmaz; respawn olur ve EXP kaybetmez.

Bu nedenle etkinlik salt “son hayatta kalan kazanır” battle royale değildir. Survival baskısı, gazın bu etkinlikte olduğuna dair bu raporda kaynak yoktur; gaz Knight Royale'e aittir. Manes'te ana yarış puan/rank ve Dark Dragon hedefidir.

## 1.2 Katılım ve maç kapasitesi

- Ekranda 10 dakika açık kalan kayıt penceresindeki Apply butonuyla katılım alınır.
- 2026 duyurusu: günde iki maç, Server Time 15:00 ve 20:30.
- Her oda en az 4, en çok 80 oyuncu alabilir; server genelinde toplam 240 katılımcı limiti açıklanmıştır.
- Birden fazla arena instance/map kullanılabilir. Kişileri kapasiteye göre deterministik şekilde odalara dağıt.
- Kayıt kapandığında minimum 4 oyuncu yoksa iptal/ertelemeyi config'le; oyuncuya mevcut sistem chat/world notice ile bilgi ver.
- Aynı karakterin birden fazla oda/queue kaydı, disconnect sonrası mükerrer teleport ve event sürerken ikinci kayıt engellenmeli.
- Saatler/takvim dönemseldir; Europe/Istanbul saati ile server timezone'u karıştırma. Repo clock standardını esas al ve config'e açıkça yaz.

## 1.3 Başlangıçta oyuncu durumu

- Katılan herkes **event level 1** ile başlar.
- Eşit şart: event'e ait ortak başlangıç skill seti; normal karakter gelişimi/ekipmanı doğrudan avantaj sağlamamalı.
- Envanter/çantalar event başlarken boştur veya event loadout'a sıfırlanır; oyuncuya aynı baz loadout verilir. Orijinal eşyalar/stats/skill bar kaydedilir ve maç çıkışında eksiksiz restore edilir.
- Level cap 30.
- Her seviye için mob öldürme XP'si; diğer oyuncu öldürme de gelişim/skor yoludur. HP, MP, attack damage ve defense seviye ile yükselir. Kaynak duyuru exact level curve vermiyor; XP threshold ve stat curve config/tasarım kararı olmalı.
- Ölümde yeniden doğma vardır; ölüm EXP kaybettirmez. Respawn delay, başlangıç noktası, koruma süresi ve ekipman düşürme davranışlarının tamamı resmi duyuruda net değil; config ve anti-camping tasarımıyla belirle.

## 1.4 Event içi gelişim / skill draft

Her mob kill'inden gelen seviye için bir seçim ekranı açılır. Resmî açıklama:

- Üst kısımdaki yeteneklerden **bir tanesi** seçilir.
- Alt kısımdaki HP ve MP potion seçeneklerinin **ikisi de** alınabilir; satın alım için gereken Noah karakterde olmalı.
- Seçim ekranı maç sırasında sınırlı sayıda açılır; içeride hayatta kalmak zor olduğundan potion almak tavsiye edilir.
- Yalnız bu event sisteminden elde edilen skill'ler kullanılabilir.
- Normal class skill kısıtlaması yoktur: oyuncular farklı sınıf becerilerini seçerek kendi skill build'lerini oluşturabilir.
- Yetenek kategorileri Active ve Passive; yeteneklerin rank/level kademeleri bulunur ve fayda kademeye göre artar.
- Bir active skill'in önceki kademesi yoksa yüksek kademe seçilemez. Skill tree prerequisite kuralı.
- Seçilen active skill mevcut skill bar'a otomatik eklenir; oyuncu event skill bar'larında yerini değiştirebilir.
- Passive skill'ler açıklamasında stackable ise birleşir (ör. attack +10% ve +5% = +15%); stack olmayan türde en yüksek/uygun rank geçerli olmalı.
- Etkinlikte passive skill'leri takip etmek için ayrı UI bar vardır. Bu client özelliği yerel client/packet incelemesiyle doğrulanmalı.
- Resmî yazı “her level” seçim sunulduğunu söyler. Aynı seçimin tekrar gösterilmesi/duplicate passive seçimi ayrıntısı kaynakta belirgin değil; draft havuzu ve duplicate policy konfigüre edilsin.

Claude, tam beceri adları, rank gereksinimleri, formüller ve pot fiyatlarını yerel skill/item datalarından bulmalı. Kaynakta veya client'ta bulunmayan becerileri isim uydurarak eklememeli. Event skill sistemi desteklenmiyorsa UI'sız güvenli alternatif veya client/proxy gereksinim listesi sunmalı.

## 1.5 PvP koruması, skor ve kill puanlaması

- İlk **3 dakika** oyuncular birbirine hasar veremez; moblar öldürülebilir.
- İlk 3 dakika sonrası sistem/bildirim oyuncu hasarını açar.
- Puan elde etme kaynakları: player kill ve level-up.
- Leaderboard'da level/rank olarak üstte olan oyuncuyu öldürmek daha fazla score verir; kendisinden daha düşük ranktakini öldürmek daha az score verir.
- Exact puan formülleri ve level-up score miktarı duyurulmamış; config olmalı. Sıfır/çok küçük sunucu popülasyonunda sıralama dilimleri doğru hesaplanmalı.
- Oyuncu HP düşüşünün rakip HP barında görünmesi kapalıdır. Client bunu desteklemiyorsa yeni paket uydurma; bu görsel davranışın sınırlamasını belirt.
- ALT ile score/status penceresi açılır; boss HP, kalan süre ve puan gösterilir. Mevcut client'ta bu UI mesajlarının desteklenip desteklenmediğini doğrula.
- Oyuncu kill attribution: son vuruş veya damage contribution? Resmî kısa açıklama bunu tarif etmiyor. Adil sunucu kararı ver: disconnect, DoT, assist, kill-steal ve PvP koruması dâhil net kural belirle. En azından bir kill/ölüm ödülünü bir kere puanla.
- Ölüm sonrası respawn puan abuse'u, sabit oyuncular arasında kill-trading ve party teaming riskleri düşünülmeli. Resmî etkinlikte birlikte hareket ettiği tespit edilen oyuncuların maç süresince suspend edileceği notu var; özel sunucuda otomatik ceza vermek yerine şüpheli davranışı logla veya GM review uygula.

## 1.6 Harita, PvE ve boss'lar

- Arena kalabalık bir haritadır. Haritanın dışından merkeze doğru düşük, orta, yüksek seviyeli PvE yaratık kümeleri bulunur. Bu, başlangıç oyuncusuna daha güvenli alan ve risk/ödül ilerlemesi sağlar.
- İç alanda düşük seviye boss Gabol, Scpri, Byzo ve Skelc görülebilir; mini-map üzerinde izlenebilir oldukları açıklanmış.
- Merkezde kalenin içinde dört orta/yüksek seviye boss ve bir Dark Dragon bulunur.
- Boss öldürme oyuncuya rastgele passive skill verir.
- Dark Dragon HP'si, süre ve skor event status window'da izlenebilir.
- Dark Dragon'u öldüren oyuncu leaderboard'dan otomatik çıkarılır ve Manes Orb birincilik ödülünü kazanır. Bu özel zafer, event sonuçlandırmasını/ödülünü tek seferlik yapmalı. Boss credit son vuruş mu, katkı mı resmi metinde belirtilmiyor; config kuralı seçilmeli.
- Monster template ID, spawn koordinatı, boss HP, saldırı patterni ve map ID raporda mevcut değildir; repo map/TBL verisinde araştırılmalı, eksikse uydurulmadan teknik engel olarak listelenmeli.

## 1.7 Sıralama ve Manes Orb ödülleri

Resmî örnek ödül tablosu:

| Sonuç / sıra | Manes Orb |
|---|---:|
| 1. ve Dark Dragon'u öldüren | 60 |
| 2. | 40 |
| 3. | 35 |
| İlk %15'lik dilim (podium dışı) | 25 |
| %15–50 | 20 |
| %51–80 | 10 |
| Katılım ödülü | 1 |

- Ödüller katılımcı sayısına göre değişebilir.
- Sıralamada Dark Dragon Slayer birinciliğe yerleşir/normal leaderboard'dan çıkarılır; diğer ranklerin nasıl kaydırılacağı özel uygulamada net bir deterministic kural olmalı.
- Dilim yuvarlama, eşit puan tie-break, disconnect olan ama maçta katılmış oyuncu ve minimum süre/katkı şartları tanımlanmalı.
- “Top 15%” dilimi 2./3. sıra ile örtüşebileceğinden podium önce atanmalı, sonra yüzdelik hesaplanmalı.
- Manes Orb event para birimidir; NPC Trader Julia'da ödülle takas edilir ve etkinlik dönemi sonunda silinir.
- Kaynakta Trader Julia'nın ödül fiyat/listesinin bu Manes duyurusunda metin olarak tamamı yoktur. Görsel tabloya bağlıdır. Claude DB'de mevcut exchange tablosunu aramalı; bulamazsa config/iş listesi oluşturmalı, item ID ve fiyat uydurmamalı.

## 1.8 Manes Arena state machine

Önerilen server state'leri:

`Disabled -> Scheduled -> RegistrationOpen (10m) -> Preparing/Teleport -> PvEProtection (180s) -> PvPActive -> FinalBoss/Closing -> Ranking -> Rewarding -> Cleanup -> Scheduled`

Bazı boss/maç zamanları protection ile paralel yürüyebilir; event scheduler tasarımında anlaşılırlığı koru.

Her arena instance için participant set, event-level/XP, stat snapshot, event inventory, skills/passives, score, kills/deaths, respawn, boss contribution, disconnect deadline ve reward-issued kayıtları tutulmalı. Disconnect oyuncunun yeniden katılma toleransı, yoksa slotun açılması davranışı belirlenmeli. Maç kapanınca tüm geçici state temizlenmeli.

## 1.9 Asgari doğrulama senaryoları

- 3 ve 4 kayıt sınırı; 80 kişi oda kapasitesi; 240 server limiti; 81./241. katılımcı davranışı.
- 10 dakikalık queue sınırında katılım ve iki paralel maç.
- Event level 1 başlangıcı, 30 level cap, XP threshold ve seviye başına tek skill draft.
- Skill tree rank prerequisite, active/passive ayrımı ve non-stacking kuralı.
- PvP ilk 180 saniye boyunca hasar engeli, 180 saniye sonrası açılması.
- Seviye atlama/kill score, üst ve alt rank kill ağırlığı, eşit score tie-break.
- Ölüm/respawn sonrası XP'nin korunması ve stat/skill/inventory restore.
- Dark Dragon ölümü ve birincilik ödülünün yalnızca bir kere dağıtılması.
- Yüzdelik ödül dilimlerinin 1, 2, 3, 4, 10, 80 oyuncu gibi küçük/büyük popülasyonlarda doğru olması.
- Disconnect, yeniden giriş, event restart, instance cleanup ve dolu çanta ödül fallback'i.

---

# 2. Knight Royale

## 2.1 Konsept ve Manes Arena'dan farkı

Knight Royale, battle royale formatıdır: oyuncular aynı gear ile rastgele noktalarda başlar, PvE mob ve sandıklardan güçlenir, zehirli gaz arenayı daraltır; amaç diğer oyuncuları alt edip son hayatta kalan olmaktır. 2026 resmi kurallarında 3. gaz bölgesi sonrasında hareketli Pathos' Delegate boss'u ve Mithril ile silah crafting de bulunur.

Manes Arena ile karıştırılmamalı: Manes'te oyuncu ölüp yeniden doğabilir, 3 dakika PvP koruması vardır, puan/seviye leaderboard'u ve Dark Dragon ödülü vardır. Knight Royale'de tek hayatta kalan kazanır, toxic gas merkezli final savaşı vardır.

## 2.2 Kayıt, schedule ve kapasite

- Oyun içi Join butonuna basarak kayıt olunur; düğme duyurusuna göre iki defa/gün kullanılabilir.
- Kayıt süresi 10 dakika; sonra katılanlar savaş alanına teleport edilir.
- 2026 Türkiye saatleri: 16:00 ve 21:30. Eski Steam duyurularındaki 15:00/20:30 gibi saatler sunucu/versiyon değişimi olabilir; config kullan.
- Eski resmî varyantta ilk Join yapan 160 kişi iki haritaya dağıtılabiliyordu. Bu kapasite 2026 güncel sayfada tekrar açıkça listelenmiyor; özel sunucu kapasitesini config olarak seç ve hangi kaynağa dayanıldığını not et.
- Eski map routing kuralı: katılımcı >=40 ise ilk gaz bölgesinden, <40 ise üçüncü gaz bölgesinden başlama. Güncel sayfa map detayı vermiyorsa bu tarihli kuralı ayarlanabilir referans yap; güncel client/map uyumunu doğrula.
- Katılım yok veya minimum oyuncu yoksa maç iptal/ertelemeyi ve notification'ı tanımla.
- Chat, Knight Royale içinde 2026 patch notunda kapalı hale getirilmiştir. Client/server chat opcode akışını doğrula; ayrı chat UI yoksa event sohbetini engellemek için mevcut yetki/dispatch filtresi kullan.

## 2.3 Event isolation, başlangıç loadout'u ve progression

- Oyuncular savaş alanında aynı güç ve aynı event ekipmanıyla başlar.
- Spawn noktası rastgeledir.
- Level 1 başlar; diğer oyuncu veya çevre monster öldürerek level atlar.
- Level arttıkça Max HP, Max MP, saldırı hasarı ve savunma yükselir. Resmî Knight Royale metni max level veya artış eğrisini net belirtmez; eski build/teknik notlardan doğrulanmadıkça config/design decision olarak bırak.
- 2026 TR duyurusundaki başlangıç kit: 1 küçük 30 HP potion, basic bow, basic sword, 15 arrow, 2 skill (her biri 30 kullanım ile sınırlı). Eski English duyurusunda “small potion” yazıyor ama skill charge ayrıntısı yok; iki beceri başlangıçta skill bar'a eklenmeli.
- Envanterde en fazla 9 farklı item bulunabilir; stack miktarı/slot kavramını veri modeline doğru uygula.
- Maça girişte karakterin normal gear, skill, level, stat, currency ve inventory durumu güvenli biçimde snapshot edilir; event içinde geçici loadout uygulanır; çıkışta (ölüm/zafer/disconnect/timeout/server stop) original state restore edilir. Event itemları maç sonunda silinir.
- Düşman HP barında HP azalışı görünmez. Client desteği ile doğrula.
- Event içinde item alışverişi/trade/warehouse gibi normal economy yolları kapatılmalı. Server authoritative inventory ve 9 item türü limitini uygula.

## 2.4 Zehirli gaz ve alan daralması

- Maç başladıktan 3 dakika sonra gaz yayılmaya başlar.
- 2026 güncel duyuru: gaz 180 saniye arayla 1. bölgeden 5. bölgeye doğru ilerler. Harita beş gaz/zone safhası taşır.
- 3. gaz bölgesi başladıktan sonra Pathos' Delegate boss'u açılır; boss 4. ve 5. gaz bölgeleri arasında sürekli hareket eder.
- Gaz içindeyken oyuncuya açık warning gösterilir; gaz hasarı başladığında ekran görseli mor renk olur. Görsel efekt desteğini client doğrulaması olmadan varsayma.
- Gas sınırı, stage timer, transition, safe polygon/circle, damage tick, tick aralığı, gaz hasarı artış eğrisi ve içeride/outside kontrolü resmi sayısal kaynakta tam açıklanmamış. Config ile tasarla. Her aşama bir ön uyarı gönderip sonra server-side boundary güncellesin.
- Gas geçişinde sınıra takılı kalma, teleport/exploit, reconnect, latency ve arena dışına kaçma önlenmeli. Oyuncu event map dışına çıkarsa spectator/death/return kuralı uygula.
- Yalnız son oyuncu kaldığında kazanır; beraber ölüm/aynı tick tie durumu için deterministic tie-break tanımla.

## 2.5 Loot, monster, chest ve equipment tiers

- Arenadaki monsterları öldürerek eşya, potion ve ok elde edilir.
- Haritada periyodik belirli noktalarda Gift of Pathos sandıkları spawn olur. Konum ve zamanlar kaynak sayfasında sayısal olarak listelenmiyor; map data ve config üzerinden tanımla.
- Oyuncu bir maçta en fazla 9 farklı item türü taşır; loot taşma politikasını belirle (item ground drop, eski item swap, pickup reddi) ve client UI'ına uyumlu yap.
- Üç güç kademeli ekipman seti duyurulmuştur:

| Kademe | Silah | AP | Zırh parçaları | Her parçanın DEF |
|---|---|---:|---|---:|
| Düşük | Dagger | 14 | Pathos Chitin Helm/Pauldron/Pants/Boots/Gauntlets | 5 |
| Düşük | Old Bow | 7 | aynı düşük set | 5 |
| Orta | Long Sword | 42 | Pathos Krowaz Chitin Helm/Pauldron/Pants/Boots/Gauntlets | 9 |
| Orta | Bow | 20 | aynı orta set | 9 |
| Yüksek | Greatsword | 63 | Pathos Holy Knight Helm/Pauldron/Pants/Boots/Gauntlets | 15 |
| Yüksek | Composite Bow | 30 | aynı yüksek set | 15 |

Tablo temel item özetidir; silah sınırlamaları, zırh slotları ve item model ID'leri local item tablolarında doğrulanmalı.

- İki skill başlangıç becerisi: Soul Strike (rakibe 200% damage, sekme/miss şansı yok) ve Fireball (uzun menzilli ateş topu).
- Diğer consumable skill/item örnekleri: Sprint Potion rakip veya mob drop; 10 dakika boyunca hareket hızını %150 artırır. Rush Scroll Golden Puppy'den; hedefe doğru hücum eder, yakın dövüş silahı gerektirir. Blink Scroll Golden Puppy'den; hedef/yaratığa doğru sıçrama, ranged silah koşulu (TR duyurusu hedefe atılma der; English sürüm yönüne blink ve 15 range der, sürüm farkını doğrula).
- Mob ve chest drop oranları resmî metinde net rakamsal değil. Loot table config olmalı, item ID doğrulanmalı.

## 2.6 Mithril weapon crafting ve Pathos' Delegate

- Event içindeki chest ve Pathos' Delegate'den belirli olasılıkla Mithril Ingot elde edilir.
- Craft ile event içi yüksek kademe silahı güçlendir:
  - Great Sword (63 AP) + Mithril -> Mithril Greatsword (84 AP)
  - Composite Bow (30 AP) + Mithril -> Mithril Bow (42 AP)
- Craft sadece Knight Royale event instance'ında çalışmalı. Event item ID/recipe/protocol UI desteğini yerel repo ve client'ta bul. Desteklenen Craft penceresi yoksa yeni opcode uydurma.
- Pathos' Delegate 3. gaz alanı aktif olduktan sonra ortaya çıkar ve 4. ile 5. alanlar arasında dolaşır.
- Boss öldüğünde Mithril Ingot ve High Level Holy Water belirli şansla drop olur; boss'u öldüren oyuncu 3 dakika attack buff alır.
- “Boss'u öldüren” attribution son vuruş olabilir; güncel kaynak Türkçe bunu söylemekle beraber exact loot roll/주변 oyuncu kuralları belirsiz. Damage contribution/last hit loot'u config yap; double reward önle.
- NPC/deathbox loot, düşen itemin sahibinin ölümü sonrası kimin alabileceği ve loot süresi resmi duyuruda tam tariflenmemiş; event economy exploitlerine göre tasarla.

## 2.7 Soul Gem rank ödülleri

Resmî örnek sıralama ödülü:

| Sıra / sonuç | Soul Gem |
|---|---:|
| 1. | 60 |
| 2. | 40 |
| 3. | 35 |
| Kalanların ilk %15'i | 25 |
| %15–50 dilimi | 20 |
| %51–80 dilimi | 10 |
| Katılım | 1 |

- Ödüller katılımcı sayısına göre değişebilir.
- Duyuru, maça girerken çantası dolu olan oyuncunun maç sonunda Soul Gem alamayabileceğini söylüyor. Private server'da bunu önlemek için maçtan önce 1 slot ayır, veya mail/claim sistemi varsa güvenli ödül teslimi kullan.
- Soul Gem Trader Julia'da kalıcı/dönemsel ödülle takas edilir. Resmî sayfa görsel tablodaki tüm item-price satırlarını metne dökmüyor; örnek olarak 700 Soul Gem Anniversary Emblem (stat özelliği yok) diyor. Item/price listesi güvenilir data olmadan uydurulmamalı.
- Soul Gem'ler Knight Royale etkinlik dönemi bitince silinir. Dönem sona ererken açık instance ve henüz dağıtılmamış rewards kapanıştan önce finalize edilmeli. Deletion idempotent ve loglanır olsun.

## 2.8 Knight Royale state machine

`Disabled -> Scheduled -> RegistrationOpen (10m) -> Matchmaking/CapacitySplit -> Preparing/Teleport -> Active (Gas Zone 1..5) -> Final/One Survivor -> Ranking -> Rewarding -> Cleanup -> Scheduled`

Maç state'inde kayıt sırası, katılımcı, event loadout snapshot, geçici inventory, gas phase/deadline, player alive/dead/spectator state, level/XP, kill credit, chest respawn, mob spawn, boss location/HP/contribution, craft, Soul Gem amount ve reward-issued alanları tutulmalı.

## 2.9 Asgari doğrulama senaryoları

- Join penceresi tam 10 dk, çift kayıt, kapasite sınırı, 40 kişi map seçimi eşiği, ilk 160/oda bölme config'i.
- 3 dakika boyunca gas başlamaması, ilk gas ve sonraki 180 sn stage geçişleri, boundary içi/dışı damage; son zone.
- Aynı tick'te iki oyuncu ölümü, son oyuncu kazanma ve beraberlik/tie-break.
- Level 1 start, XP gain, stat progression config, mob kill ve PvP kill attribution.
- Başlangıç kit, 30 kullanım beceri sınırı, 9 farklı item slotu, stack davranışı, loot full inventory.
- Gear tier loot ve statlar; normal gear etkisinin eventte olmaması; event exitte orijinal gear/stat/skill restore.
- Gift of Pathos chest schedule/loot; Golden Puppy scroll; Mithril craft işleminin atomic/one-time olması.
- Pathos' Delegate yalnız 3. gas phase sonrasında spawn olur, 4–5 zone içinde hareket eder, boss ödülü bir kez verilir.
- Ranking percentiles, katılım ve tie-break; Soul Gem miktarı; dolu çanta mail/fallback.
- Disconnect/reconnect, karakter/party event dışı packetleri, chat kapatma, server restart ve event cleanup.

---

# 3. Bilinen belirsizlikler ve uygulama kararları

| Konu | Resmî kaynakta olan | Uygulama tavsiyesi |
|---|---|---|
| Saatler | Dönem/sunucuya göre değişiyor | Timezone ile config; kaynakta duyuru saatini raporla |
| Manes kapasite | 4–80 oda, 240 toplam | Config; oda sayısı ve üst limit sınır testi |
| Knight Royale kapasite | Eski sürüm 160/2 map | Güncel destek yoksa configurable default; client/map verisi ile doğrula |
| Puan formülleri | Manes'te seviye/PvP ve rank farkı var; katsayı yok | Config ve reproducible formulas |
| Manes level curve | Max 30; statlar artıyor | XP threshold/stat curve design decision, config |
| Knight Royale level cap | Duyuruda yok | Repo/client kanıtı olmadan değer uydurma |
| Gaz hasarı/safe shape | 3 dk ilk başlangıç, sonra 180 sn; beş bölge | Hasar/tick/geometry config ve test |
| Soul Gem/Manes Orb takasları | NPC var; tam tablo görselde | Veritabanında ara; item/fiyat uydurma |
| Skills, item IDs, mobs, maps | İsim/örnek kısmen var | Yerel TBL/map/client/protocol'dan doğrula |
| UI/score/HP bar | Event duyurusu UI tarifliyor | Mevcut client'ın packet desteğini incele; yeni opcode icat etme |
| Ölüm ve event state restore | Duyurular tüm teknik edge case'i vermiyor | Snapshot/restore ve disconnect cleanup test et |

---

# 4. Claude'a kopyalanacak uygulama istemi

```text
Bu repo içindeki Knight Online Rust sunucusuna şu iki etkinliği ekle:
1) Manes Survival (resmî ad: Arena of Manes / Manes' Arena)
2) Knight Royale

İlk olarak AGENTS.md ve diğer repository talimatlarını bulup oku. README, workspace crate yapısı, event/instance/scheduler/queue, combat, skill, monster spawn, inventory, NPC, party, DB, packet/protocol ve varsa client patch kodlarını incele. Çalışma alanındaki mevcut kullanıcı değişikliklerini bozma veya üzerine yazma.

Bu proje v2603 KO client uyumluluğunu hedefleyen Rust server. Mevcut client packetlerinde kanıtı olmayan event UI, skill bar, mini map, hp-hide, gas effect veya craft opcode'u uydurma. İsimleri geçen her map/monster/skill/item için repo TBL/DB kaynaklarında gerçek ID ve destek bul. Bulamadığını açıkça listele; dummy item ID veya sahte protocol davranışıyla tamamlandı deme. Önce repo keşif raporu yaz (dosyalar/modüller, yeniden kullanılabilir altyapı, protokol kanıtları, eksikler ve entegrasyon planı), sonra benden onay beklemeden uygulanabilir kısmı kodla. Belirsiz sayısal değerler config'e taşınsın.

MANES SURVIVAL / ARENA OF MANES REFERANSI
- İki kayıt penceresi/gün, her biri 10 dakika; 2026 örnek saatleri Server Time 15:00 ve 20:30. Oda min 4, max 80; toplam server kapasitesi 240. Eski duyuru verisi olduğundan zaman/kapasite config olsun.
- Oyuncu event level 1, eşit geçici loadout ve boş event inventory ile girer. Normal inventory, stats, equipment, skill state snapshot edilip event sonunda tüm çıkış türlerinde restore edilir. Event level cap 30. Mob ve oyuncu kill ile XP/level; seviye HP/MP/AP/DEF artırır. Resmî level curve yok; tasarla ve config yap.
- İlk 180 saniye PvP kapalı, PvE açık; sonra PvP açılır. Ölüm oyuncuyu eventten çıkarmaz, respawn olur ve EXP kaybetmez. Respawn yeri/gecikmesi/koruma süresi config.
- Level aldıkça beceri seçme ekranı: 1 active/passive choice ve iki potion seçeneği; pot için Noah gerekir. Yalnız event skills. Her class'a açık. Active rank prerequisite, active skill bar, passive stack/non-stack davranışı ve rank progression uygula ancak önce client packet desteğini doğrula.
- Skor level up ve PvP kill ile artar; higher-ranked player kill daha fazla, lower-ranked kill daha az score getirir. Katsayılar kaynakta yok, config ve test.
- Arena outer-to-inner seviyeli PvE mob alanları, Gabol/Scpri/Byzo/Skelc mini-bossları ve merkezde dört boss + Dark Dragon konsepti. Mob/boss template, map, spawn, HP yerel datadan bulunmalı.
- Boss kill rastgele passive verebilir. Dark Dragon'u öldüren oyuncu rank listesinden çıkarılıp first prize Manes Orb (referans 60) kazanır. Normal rank ödülleri: 2=40, 3=35, podium dışı top 15%=25, %15–50=20, %51–80=10, katılım=1. Participant count'a göre değişebileceği için config. Podium ve percentile overlap, tie-break, disconnect ve boss kill attribution deterministik tasarlanmalı.
- Manes Orb Trader Julia ile dönüştürülür ve event dönemi bitince silinir. Exchange price/item listesi repo data'dan bulunmadıkça uydurma.

KNIGHT ROYALE REFERANSI
- 2 event/day, kayıt penceresi 10 dakika; 2026 Türkiye saati 16:00/21:30. Oyuncular rastgele spawn olur ve eşit event loadout ile başlar. Eski kaynak: ilk 160 oyuncu, iki map; 40+ kişi zone 1, <40 zone 3. Güncel kapasiteyi repo/client kanıtı yoksa config yap.
- Başlangıç kit: 30 HP potion x1, basic bow, basic sword, 15 arrow, iki beceri (her biri 30 kullanımlık). Level 1 başlar; mob/player kill ile level/stat gelişir. Duyuru level cap/curve belirtmiyor, değer uydurma.
- Envanter en fazla 9 farklı item türü. Rakip HP barında HP düşüşü görünmez; client support yoksa bu UI'ı uydurma. Maç sonunda geçici eşyalar silinir, asıl karakter loadout'u geri yüklenir.
- Maç başladıktan 180 saniye sonra gas başlar; 180 saniyede bir zone 1'den zone 5'e ilerler. Gas polygon, warning, damage/tick ve değerler config. Yalnız yaşayan son oyuncu kazanır; eşzamanlı ölüm tie-break belirle.
- Mob kill eşya, potion ve arrow sağlar; Gift of Pathos chest belirli lokasyonlarda periyodik çıkar. Loot tablolarını data/config üzerinden yap. Gear üç tier: Dagger 14 AP/Old Bow 7 AP ve 5 DEF Chitin set; Long Sword 42/Bow 20 ve 9 DEF Krowaz set; Greatsword 63/Composite Bow 30 ve 15 DEF Holy Knight set. Item ID'leri repo datadan doğrula.
- Başlangıç skillleri Soul Strike (200% damage, no miss) ve Fireball (long range). Sprint Potion 10 dk +150% movement; Rush Scroll Golden Puppy'den ve melee şartıyla; Blink Scroll Golden Puppy'den, ranged şartıyla hedefe blink. Kaynak sürüm çelişkilerini (Blink yönü/menzili gibi) not edip config kararı ver.
- Sandık ve Pathos' Delegate'den Mithril Ingot düşebilir. Great Sword + Mithril => Mithril Greatsword (84 AP); Composite Bow + Mithril => Mithril Bow (42 AP). Craft yalnız event içinde olmalı.
- 3. gas zone aktif olunca Pathos' Delegate spawn olur, 4. ve 5. zone arasında hareket eder. Kill loot: Mithril Ingot/High Level Holy Water olasılığı ve boss slayer'a 3 dk attack buff. Attribution/drop kuralı config.
- Rank Soul Gem: 1=60, 2=40, 3=35, top 15%=25, 15–50%=20, 51–80%=10, participation=1; popülasyona göre değişebilir. Full bag ödül sorununu güvenli mail/claim ya da girişte slot reservation ile çöz. Trader Julia takas tablosu data'dan bulunmalı. Soul Gem event period sonunda silinir.
- Knight Royale içi chat 2026 patch notunda kapalı. Mevcut packet dispatch'te güvenli biçimde uygula.

MİMARİ/GÜVENLİK
- Ortak event lifecycle veya instance sistemini yeniden kullan; var olanı keşfetmeden ikinci framework yazma. Her iki oyun modunun state ve reward mantığı ayrı modüler olsun.
- Saat dilimini repo standardıyla uygula, DST ve duplicate scheduler run sorunlarını önle. Event takvimi, queue capacity, loot, wave/gas/score/reward değerleri config/DB ile ayarlanabilsin.
- Tüm state server-authoritative, reward/kill/score/crafting idempotent. İki worker'ın aynı maçı/ödülü sonuçlandırmasını engelle.
- Disconnect/reconnect, ölüm, tekrar spawn, inventory dolu, event sırasında zone exit, crash/restart, winner/last boss, cleanup ve restore yollarını kapsa. Orijinal oyuncu verisi kaybolmasın.
- Repo'da mevcut paket/protokol ya da DB desteği bulunmayan özellikleri ayrı “client/data blocker” olarak raporla ve kullanılabilir alternatif uygula.
- İlgili testleri ekle ve çalıştır: queue capacity/window, 180s PvP protection, Manes XP/level cap/draft prerequisites/scoring/percentile awards/Dark Dragon reward; Royale zone timer/boundary damage/last survivor/gas simultaneous deaths/9-item inventory/craft/boss spawn/reward idempotency/original state restore.
- Build/test başarısızlıklarını çöz veya somut hata olarak raporla. Son raporu Türkçe yaz; değişen dosyaları, tasarım varsayımlarını, item/map/client sınırlamalarını, migrations/config ve doğrulama sonuçlarını belirt.
```

---

# Kaynaklar

- NTTGame, [Arena of Manes / 1 Temmuz 2026 duyurusu](https://store.steampowered.com/news/posts/?appgroupname=Knight+Online&appids=389430&enddate=1783924465&feed=steam_community_announcements) — giriş, 3 dk koruma, respawn, seviye/skill, boss, puan, Manes Orb.
- NTTGame forum, [Arena of Manes is Back](https://forum.nttgame.com/forum/showthread.php?976458-Arena-of-Manes-is-Back%21=&p=3378596) — resmî duyurunun tekrar sayfası.
- NTTGame, [Manes/Arena event patch notes](https://store.steampowered.com/news/posts/?appgroupname=Knight+Online&appids=389430&enddate=1785383774&feed=steam_community_announcements) — etkinlik başlangıç ve bitiş notları.
- NTTGame, [Knight Royale Geri Döndü, 20 Ağustos 2026](https://www.nttgame.com/knight/tr/news/views/3/4188/1) — güncel TR kuralları, loadout, gas, mob/chest, item tiers, skill, Mithril, boss ve Soul Gem.
- NTTGame, [Knight Royale is Back, 20 Ağustos 2026 (EN)](https://www.nttgame.com/knight/en/news/views/3/4189/1) — İngilizce içerik doğrulaması.
- NTTGame, [2024 Knight Royale eski kapasite/zone ayrıntıları](https://forum.nttgame.com/forum/showthread.php?842250-Knight-Royale-is-Back%21=&p=3038778&viewfull=1) — 160 oyuncu, 40 oyuncu zone routing, eski kurallar.
- NTTGame, [Knight Royale 2024 genel kuralları](https://www.nttgame.com/knight/tr/news/views/3/3628/1) — eski beceri, gear ve Pathos' Delegate ayrıntıları.
- Proje içi mimari için: çalışma alanındaki README ve Rust kaynak kodu (Claude yerel kopyayı esas almalı).

> Kaynaklar resmi duyurulardır; etkinlikler dönemsel ve sürümle değişir. Özellikle schedule, kapasite, gas damage, XP curve ve exchange item listelerini kendi sunucunuz için config/yerel verilerle doğrulayın.
