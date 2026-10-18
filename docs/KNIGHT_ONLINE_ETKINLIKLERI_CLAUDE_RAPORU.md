# Knight Online Etkinlikleri: Claude için Uygulama Raporu

**Kapsam:** Ultima'nın Zindanı, Ateş & Buz (Fire & Ice / Ice-Fire Golem) ve Full Moon Rift  
**Araştırma tarihi:** 11 Ekim 2026  
**Amaç:** Bu raporu Claude'a vererek mevcut Knight Online Rust sunucu projesine bu etkinlikleri uyarlatmak.

## Önemli bağlam

Knight Online etkinlikleri dönemsel olarak açılıyor ve kuralları güncelleniyor. Aşağıdaki bilgiler resmî NTTGame duyurularındaki güncel/yakın dönem tasarımları temel alır; özel sunucuya aynı ödül, zaman, HP veya erişim şartlarını doğrudan kopyalamak zorunlu değildir. Öncelik, tanınan oyun döngüsünü doğru aktarmak ve sunucunun mevcut istemci/protokol/harita yetenekleriyle uyumlu tasarlamaktır.

Mevcut çalışma alanı README'si projeyi **Rust ile yazılmış, Knight Online v2603 istemcisiyle protokol uyumluluğunu hedefleyen clean-room oyun sunucusu** olarak tanımlıyor. Belgelenen sistemler arasında 77 zone, binlerce NPC/monster spawn, savaş, grup, eşya, Lua NPC diyalogları, Castle Siege ve bazı etkinlik altyapıları bulunuyor. Claude öncelikle bu bilgileri koddan doğrulamalı. İstemci tarafından desteklenmeyen yeni UI/packet/opcode varsaymamalı; etkinlik tasarımını mevcut protokol ve içerik sınırlarına uyarlamalı.

---

## 1. Ultima'nın Zindanı (Ultima's Dungeon)

### Etkinliğin özeti

Bifrost temalı, kayıt penceresiyle katılım alınan, odaya oyuncuların toplandığı bir PvE raid/boss etkinliğidir. Ana hedef Ultima'yı öldürmektir. Ultima başlangıçta dokunulmazdır; önce zindandaki mühür yapısı kırılır. Boss'un can eşiklerinde tüm arenayı etkileyen saldırılarından kaçış yapılarının güvenli alanlarına girilerek kurtulunur.

### Giriş ve oda kuralları

- Örnek 2026 duyurusunda etkinlik günde iki giriş penceresiyle (10:00 ve 21:30) duyurulmuştur. Eski/global duyurularda 12:00–23:00 gibi farklı saatler görülür; saatler yapılandırılabilir olmalı.
- Oyuncu kayıt penceresinden sıraya girer; sıra ve oda kapasitesi önemlidir.
- Karakter seviyesi en az 80.
- Başvuru anında en az 50 aylık Ladder Point istenir; başvuru penceresi kapanırken de tekrar kontrol edilir. Bu sırada puan 50'nin altına düşerse kayıtlı oyuncu içeri alınmaz.
- Duyuruda giriş için Premium veya 50 Ladder Point ifadesi de yer alır; daha yeni duyuruda başvuru/son kontrol 50 puana özellikle bağlanmıştır. Erişim şartı versiyon/duyuru ayarı olmalıdır.
- Bir odada en çok 30 kişi bulunur. Kayıt sayısı 30'u aştığında yeni oda açılması için en az 5 ilave kayıt gerekir.
- Oyuncu geri sayım sırasında Cancel ile kaydını iptal edebilir.
- Minimum 5 kişilik yeni oda koşulu kayıtlı ama şartı sağlamayan oyuncu yüzünden bozulabilir. Oyuncu koşulları teleport öncesi tekrar doğrulanmalı, geçersiz oyuncular ayıklanmalı.

### Raid akışı ve mekanikler

1. Geçerli oyuncular etkinlik örneğine teleport edilir ve oda oturumu açılır.
2. Girişten 2 dakika sonra Ultima ve mühür yapısı ortaya çıkar.
3. Mühür yok edilmeden Ultima hasar almaz.
4. Mühür yapısına temel saldırı (R) sabit hasar verir. Yay takılı karakterin yapıya vurabilmesi için yayı çıkarması gerekir. R tuşu/weapon kontrolü istemci-protokol kabiliyetleriyle doğrulanmalı.
5. Mühür kırılınca Ultima hasar alabilir. Boss'u öldürmek için 30 dakika vardır.
6. Ultima hedef aldığı oyuncunun çevresine alan saldırısı yapabilir; oyuncular canlarının %30'unu kaybedebilir.
7. Boss HP %70, %40, %10 olduğunda yıkıcı alan saldırısı başlar.
8. Saldırı öncesi boss'un dört tarafında hedeflenemeyen, hasar verilemeyen kaçış yapıları belirir. Her birinin çevresinde güvenli daire vardır; saldırı başladığında güvenli alana geçmek için 5 saniye bulunur.
9. Saldırıdan hiç oyuncu ölmezse Ultima 10 saniyeliğine lethal duruma geçer; bu sürede Warrior sınıfı Ultima'ya 1.5 kat hasar verir.
10. Başarılı öldürme, süre aşımı ve oyuncu/oda temizliği durum makinesiyle yönetilmelidir.

### Örnek boss ölçeklemesi

| Odadaki oyuncu | HP | Savunma | Saldırı |
|---|---:|---:|---:|
| 4–10 | 1.000.000 | 2.683 | 7.338 |
| 11–20 | 2.000.000 | 4.025 | 11.007 |
| 21–30 | 3.000.000 | 8.051 | 22.015 |

Resmî duyurudaki eşikler 10 ve 20'de çakışan biçimde yazılmış; kodda aralıklar çakışmasız tanımlanmalı. 1–3 oyuncu ve boş oda senaryoları da ayrıca belirlenmeli.

### Ödül kurgusu (resmî örnek)

- İlk yıkıcı saldırıdan önce ölen oyuncu ödül alamaz.
- İlk saldırıdan sağ çıkıp ikinci saldırıyı görmeden ölen: Ascent Scroll.
- İkinci saldırıdan sağ çıkıp üçüncü saldırıyı görmeden ölen: Fragment of Arrogance.
- Üçüncü saldırıdan sağ çıkıp Ultima ölmeden ölen: Fragment of Gluttony.
- Ultima ölünceye dek ölmeden kalan: Fragment of Gluttony ve Fragment of Arrogance.
- Boss kutusunda 30 günlük Lunar Tattoo şansı vardır; boss'a verilen hasar oranı yükseldikçe loot ihtimali artar.
- Envanteri dolu oyuncu ödül alamaz. Ödül teslimi atomik/tekrarsız olmalı; dolu envanter/disconnect ele alınmalı.

---

## 2. Ateş & Buz / Fire & Ice Golem

### Etkinliğin özeti ve akış

Ronark Land Bowl'da iki ulusun golemlerini kullanarak yapılan kısa, açık dünya ulus savaşıdır. Amaç rakip ulusun golemlerini öldürüp kendi ulusuna dönüştürmek ve süre sonunda skoru önde bitirmektir.

- Örnek 2026 takviminde günde iki kez, sunucu saatine göre 14:00 ve 22:00'de Bowl merkezinde çalışır. Türkçe duyurularda TR saatleri 11:00/22:00 olarak da geçer; saat dilimi yapılandırılmalı.
- 50 El Morad Flame Golem ve 50 Karus Ice Golem spawn olur; toplam 100 hedef.
- El Morad sadece Karus golemine; Karus sadece El Morad golemine saldırabilir.
- Rakip golem ölünce kendi ulusunun golemi olarak yeniden doğar. Bir kill hedefi eksiltmez, sahipliği değiştirir.
- Her dönüşüm ilgili ulusun skorunu artırır. 10 dakika sonunda daha çok golemi dönüştüren ulus kazanır.
- Bir ulus 100 puana ulaşır/tüm golemleri kendi tarafına çevirirse etkinlik hemen biter.
- Skor UI'sı mevcut client'ta desteklenmiyorsa yeni UI varsayma; mevcut event board/chat/world message kullan.
- Her etkinlik koşusunda toplam 1 Lunar War Belt düşme şansı bulunabilir; garanti değildir.
- Parti üyesi golem öldürdüğünde partideki tüm üyeler ödül adayı olur; oyuncu/party tekrar sayılmamalı.
- Ronark Land Blitz, Fire & Ice saatleri boyunca kapalıdır.
- Golemler AoE saldırılardan normal hasarın yalnızca %30'unu alır (güncel duyuru kuralı).

### Ödüller (2026 örneği)

**Kazanan ulus:** 1 saatlik +%5 Attack Power buff; golem öldürenlerden rastgele 1 kişiye takas edilemeyen 30 günlük Event Emblem Exchange Voucher; rastgele 2 kişiye 22 Hero's Vow; kazanan ulusta golem kesmiş her oyuncuya 3 Dual Potion ve 24 saatlik Event Emblem.

**Kaybeden ulus:** Golem kesenlerden rastgele 1 kişiye 22 Hero's Vow; golem kesmiş her oyuncuya 1 Dual Potion ve 4 saatlik Event Emblem.

**Beraberlik 50:50:** İki ulustaki katılımcılardan 1 kişiye emblem kuponu, 3 kişiye 22 Hero's Vow; katılımcılara Dual Potion ve 4 saatlik Event Emblem.

Eski duyurularda bazı ödüller Holy Starlight şeklinde değişmiş. Ödül tablosu config olmalı; özel sunucuda ekonomi dengesine göre ayarlanmalı.

### Sunucu durumları

`Scheduled -> Announced -> Active -> Resolving -> Rewarding -> Cleanup -> Scheduled/Disabled`

Her golemin ulus sahibi ve spawn/ölü/respawn state'i tutulmalı. Kill tek bir atomik sahiplik değişimi ve skor güncellemesi üretmeli. Son vuruş, AoE katkısı, party eligibility ve disconnect ödül adaylığı tanımlanmalı. Skor 100 olduğunda etkinlik yalnızca bir kez bitmeli.

---

## 3. Full Moon Rift

### Etkinliğin özeti ve giriş

Oyuncuların eşitlenmiş karakter değerleriyle kulelerini istilacı yaratık dalgalarından savunduğu, parti büyüklüğüne göre zorluğu belirlenen instanced tower-defense etkinliğidir. Normal ekipman/class skill avantajı devre dışı bırakılır; etkinliğe özel yetenekler ve Monster Coin ekonomisi kullanılır.

- Moradon'daki Lunar Order Priest Blanc üzerinden Voucher alınır ve event board'a kayıt yapılır. Günlük limit duyurular arasında 1 veya 3 voucher/account/server olarak farklı; config ile belirlenmeli.
- Voucher bir gün sonra silinir; etkinliğe girince tüketilir.
- Level 70+ ve en az 2 kişilik parti gerekir.
- Juraid Mountain başlamadan 1 saat önce ve bittikten 1 saat sonrasına kadar giriş kapalıdır.
- Parti boyutuna göre 2–3 kişi kolay, 4–7 normal, 8 zor/yüksek.
- Güncel 2026 varyantında kolay 5 aşama, normal 15, zor 20 aşama. 2021/önceki resmî sayfalar 18 veya normal 12 gibi eski farklılıklar içeriyor.

### Kule savunma kuralları

- Dalgayı geçmek için o dalgadaki tüm yaratıklar öldürülür.
- Nexus kulesi ölürse etkinlik kaybedilir ve oyuncular Moradon'a gönderilir.
- Bir oyuncunun 3 ölüm hakkı vardır; üçüncü ölümde Moradon'a gönderilir ve ödül alamaz.
- Oyuncu moblardan ölünce EXP kaybetmez.
- `/town` oyuncuyu son kule/Nexus savunma alanına taşır.
- Spawn bölgesine fazla giren oyuncu savunma alanına geri gönderilir.
- Nexus yok olursa veya bütün oyuncular ölüm hakkını kullanıp çıkarılırsa kayıp; uygun son dalga tamamlanıp Nexus hayattaysa zafer.
- Eski kaynaklarda Outer/Inner/Nexus HP 30k/30k/100k; başka varyantta 20k/25k/50k. Config üzerinden ayarlanmalı.

### Eşitlenmiş karakter ve event yetenekleri

Eski resmî teknik açıklama her oyuncuya aynı görünüm, 10.000 HP ve sabit 500 saldırı hasarı verir; normal item/class skill kapalı, Chaos ve Rift becerileri açık. Değerler referans olup serverda stat override/restore gerekir.

Monster Coin yalnız instance içinde kullanılan ve çıkışta silinen para birimidir. Mob öldürmek yakındaki oyunculara coin verir; son vuruş oyuncusuna ek coin tanımlanabilir. Haritadaki NPC'lerden skill, tower reinforcement ve guard summon alınır. Örnek summonlar: savunma guard 15.000 HP/100 hasar, AoE guard 7.000 HP/2.500 hasar, takip guard 10.000 HP/1.000 hasar; örnek maliyet 8 coin ve cooldown 400 saniye. Yetenek listesi (heal, splash saldırı, Fire Strike, Bloody Arrow, Ankle Freeze, Drain Restore, Shield of God) mevcut kaynakta doğrulanıp config olmalı.

### Güncel 20 dalgalı plan

| Aşama | Ordu | Mob sayısı |
|---:|---|---:|
| 1 | Castle Army 1 | 15 |
| 2 | Castle Army 1 | 13 |
| 3 | Castle Army 1 | 12 |
| 4 | Castle Army 1 | 16 |
| 5 | Castle Army 1 | 14 |
| 6 | Castle Army 2 | 19 |
| 7 | Castle Army 2 | 17 |
| 8 | Castle Army 2 | 14 |
| 9 | Castle Army 2 | 16 |
| 10 | Castle Army 2 | 17 |
| 11 | Draki Army | 19 |
| 12 | Draki Army | 19 |
| 13 | Draki Army | 21 |
| 14 | Draki Army | 20 |
| 15 | Draki Army | 21 |
| 16 | Bifrost Army | 23 |
| 17 | Bifrost Army | 24 |
| 18 | Bifrost Army | 25 |
| 19 | Bifrost Army | 25 |
| 20 | Bifrost Army | 23 |

Ordular: 1–5 Castle Army 1, 6–10 Castle Army 2, 11–15 Draki Army, 16–20 Bifrost Army. Eski tasarımda dalgalar arası bekleme 1–5 için 10 sn, 6–12 için 20 sn, 13–18 için 30 sn; güncel değer değişebilir. Örnek mob mekanikleri: Timarli menzilli; Astaroth sadece kuleye yönelip intihar eder; Muspell 3 sn skill kilitler; Centaur %30 can azaltır; Draki Elite 3 sn root; bazı bosslar öldüğünde yakına EXP verir. Template ID'leri yerel veriden bulunmalı.

### Jewel ve ödüller

- Eski varyantta Kale/Draki yaratığında son vuruş 1 Jewel; Bifrost son vuruşu 2 Jewel. Draki kutusu Jewel'i olasılıkla, Bifrost kutusu garanti 1 Jewel verebilir. Astaroth coin/Jewel vermez.
- Eski NPC takası: 250 Jewel Old Draki Supply Box; 350 Superior Draki Supply Box; 100 Grace of the Moon God; 200 7 günlük Lunar Tattoo.
- Gerçek item ID/adı ve ekonomiyi yerel DB/TBL'den doğrula; olmayan item varsayma.

---

## 4. Sürüm farkı ve belirsizlikler

1. Ultima saatleri ve Premium/Ladder Point erişim şartı duyurudan duyuruya değişiyor. Config ve teleport öncesi tekrar kontrol kullan.
2. Fire & Ice ödülleri dönemsel olarak Holy Starlight / Hero's Vow gibi farklılaşmış.
3. Full Moon Rift eski sürümlerde 18 veya 20 dalga, normal mod 12 veya 15 olabilir. Bu rapor güncel 5/15/20 dalga varyantını baz alıyor.
4. Voucher limiti günlük 1 veya 3 olarak değişmiş.
5. Harita, mob, skill, UI ve packet desteği v2603 client yeteneğine bağlı. Client'ın desteklemediği yeni opcode uydurulmamalı.
6. Buradaki sayısal değerler resmî referanstır; özel sunucunun mevcut item/harita/denge verisiyle doğrulanmadan birebir kullanılmamalı.

---

## 5. Claude'a verilecek ana istem

```text
Bu çalışma alanındaki Knight Online Rust sunucusuna üç etkinlik eklemek istiyorum: (1) Ultima'nın Zindanı / Ultima's Dungeon, (2) Ateş & Buz Golem / Fire & Ice, (3) Full Moon Rift.

Önce repo talimatlarını (AGENTS.md vb.), README'yi ve ilgili crate/modülleri incele. Bu proje Knight Online v2603 istemcisiyle uyumlu clean-room Rust sunucusudur. Var olmayan opcode, packet, NPC UI, skill, map, mob, item ID, DB tablosu veya oyun sistemi uydurma. Yeni istemci özelliği gerekiyorsa protocol/client kaynaklarında kanıt ara. İstemci desteği yoksa mevcut packet/chat/NPC/event mekanizmalarıyla en yakın güvenli uyarlamayı öner ve sınırlamayı yaz.

Önce kısa keşif raporu çıkar: (a) mevcut event/instance/scheduler/party/combat/spawn/NPC/reward/persistence sistemleri nerede, (b) hangi packet ve client davranışları kanıtlanmış, (c) uygulanabilirlik/eksikler, (d) ortaklaştıracağın bileşenler. Sonra belirsiz noktalar için config üzerinden değiştirilebilir varsayımlar seçip implementasyona devam et; durup onay bekleme.

1) ULTIMA'S DUNGEON
- Kayıt penceresi/sıra; oda başına en çok 30, yeni oda için en az 5 kişi; seviye 80+; başvuru ve teleport anında erişim doğrulaması. Saatler ve 50 Ladder Point/Premium şartı config olsun.
- Girişten 2 dakika sonra Ultima ve mühür spawn olur. Mühür kırılmadan boss hasar almaz. Mühür temel saldırıyla sabit hasar alır; yay/weapon sınırlaması ve R tuşu davranışını istemci desteğiyle doğrula. Boss 30 dakikada öldürülmeli.
- Ultima hedef oyuncu çevresine alan saldırısı yapabilir. HP %70/%40/%10 eşiklerinde dört yönde güvenli kaçış alanları çıkar; 5 saniyede alana girmeyen zarar görür. Hiç kimse ölmezse Ultima 10 saniye lethal olur ve Warrior 1.5x damage verir.
- Referans ölçek: 4–10 kişi 1m HP/2683 DEF/7338 ATK; 11–20 2m/4025/11007; 21–30 3m/8051/22015. Config yap, 1–3 oyuncu ve boş oda sınırlarını güvenli ele al.
- Ölüm eşiği ödülleri: ilk eşikten önce ölüm ödülsüz; 1. eşik sonrası Ascent Scroll; 2. sonrası Fragment of Arrogance; 3. sonrası Fragment of Gluttony; boss'u ölmeden bitirene ikisi. Boss loot olasılığı katkıya göre olabilir. Item ID'lerini repo verisinden doğrula; yoksa placeholder'ı açıkça raporla. Ödül iki kez verilmesin; dolu envanter/disconnect ele alınsın.

2) FIRE & ICE GOLEM
- Ronark Land Bowl'da 50 El Morad Flame Golem + 50 Karus Ice Golem; iki günlük saat penceresi config. El Morad yalnız Karus'a, Karus yalnız El Morad golemlerine vurur. Öldürülen golem öldürenin ulusuna ait olarak uygun noktada yeniden doğar; sahiplik ve skor değişir.
- 10 dakika sonunda daha çok golem dönüştüren kazanır; 100 puana ulaşan anında kazanır. Beraberlik dahil sonucu tek sefer resolve et.
- Party üyesi kill yaptığında party üyeleri ödül adayı olur; tekrar sayımı engelle. Her koşuda toplam 1 Lunar War Belt şansı opsiyonel. AoE hasar çarpanı %30 config. Fire & Ice süresinde Ronark Land Blitz çakışmasını engelle.
- Ödüller config: kazanana 1 saat +5% attack buff, aktif katılımcılara Dual Potion/Event Emblem, rastgele ek ödüller; kaybeden ve beraberlik ödülleri. Item ID uydurma.

3) FULL MOON RIFT
- Moradon NPC/board voucher, level 70+, en az 2 kişilik party; voucher ve günlük limit config; Juraid'den 1 saat önce/sonra giriş kapalı. 2–3 kişi kolay=5 wave, 4–7 normal=15, 8 zor=20.
- Instanced tower defense: referans event statları 10,000 HP/500 sabit attack; normal gear/class skill kapalı, event skill açık. Çıkışta asıl stat/ekipmanı geri yükle. Kule HP config; kaynaklarda farklı değerler var.
- Her wave tüm moblar ölünce tamamlanır. Nexus yıkılırsa kayıp. Oyuncu 3. ölümde dışarı çıkar, ödül alamaz; EXP kaybı yok; /town savunma alanına döndürür; spawn alanına girenler geri itilir.
- Güncel wave sayıları: 1–5 Castle Army 1 [15,13,12,16,14]; 6–10 Castle Army 2 [19,17,14,16,17]; 11–15 Draki Army [19,19,21,20,21]; 16–20 Bifrost Army [23,24,25,25,23]. Mob template ID ve mekanikleri repo verisinden bul.
- Monster Coin instance içi para birimi, kill ile yakındakilere dağıt, last-hit bonusu opsiyonel; çıkışta sil. NPC üzerinden event skill/guard/tower reinforcement. Timarli ranged, Astaroth kuleye intihar, Muspell 3 sn skill disable, Centaur %30 HP azaltma, Draki Elite 3 sn root mekaniklerini yalnız altyapı destekliyorsa ekle.
- Eski ödül referansı: normal mob last-hit 1 Jewel, Bifrost 2; Draki kutusu olasılıkla Jewel, Bifrost kutusu garanti 1; Astaroth coin/Jewel vermez. 100 Jewel Grace, 200 Lunar Tattoo (7 gün), 250 Old Draki Supply Box, 350 Superior Draki Supply Box. Ekonomiye göre config.

UYGULAMA İLKELERİ
- Önce ortak event lifecycle/instance altyapısı ara ve mevcut kodu kullan; gereksiz paralel sistem kurma. Etkinlikleri modüler tut.
- Scheduler saat/dilimini açık ele al; mevcut server clock standardını kullan ve DST hatasından kaçın. Takvim, ödül, stat, wave ve kapasite config/DB ile değiştirilebilir olsun.
- Server-authoritative uygula: skor, HP eşiği, kill credit, item ödülü, ölüm sayısı client verisine bırakılmasın. Yarış koşullarında atomik/tek seferlik sonuçlandırma yap.
- Disconnect/reconnect, leader quit, party dağılımı, zone çıkışı, restart, timer cancellation, cleanup senaryolarını çöz. Kapanışta stat/konum geri yükle, event moblarını temizle.
- Mevcut çalışma ağacındaki kullanıcı değişikliklerine dokunma. Migration gerekiyorsa geriye uyumlu yap.
- İlgili testleri ekle ve çalıştır: state transition, golem ownership/score, wave completion, 3-death, ödül idempotency, boss threshold/safe-zone, scheduler sınırları. Build/test hatasını düzelt veya açıkça raporla.
- Sonunda dosyaları, kararları, client sınırlamalarını, migration/config ve doğrulama sonuçlarını Türkçe özetle. Tamamlanmamış işi tamamlandı gibi gösterme.

Önce keşif raporunu hazırla, sonra kapsamı uygula. Kod/protocol kanıtı bu referanslardan farklıysa yerel kanıtı esas al ve farkı raporla.
```

## Kaynaklar

- NTTGame, [Ultima'nın Zindanı, 17 Eylül 2026](https://www.nttgame.com/knight/tr/news/views/3/4202/1)
- NTTGame/Steam, [Ultima's Dungeon — boss mekanikleri, stat ölçeklemesi ve ödüller](https://store.steampowered.com/news/posts/?appids=389430&feed=steam_community_announcements)
- NTTGame, [22. Yıl Dönümü — Fire & Ice güncel kuralları](https://forum.nttgame.com/forum/showthread.php?1003951-22-Y%C4%B1l-Etkinlikleri)
- NTTGame, [Ateş & Buz etkinliği — Türkçe saat/ödül varyantı](https://www.nttgame.com/knight/tr/news/views/3/4042/1)
- NTTGame/Steam, [Full Moon Rift — güncel dalgalar ve Monster Coin](https://store.steampowered.com/news/posts/?appgroupname=Knight+Online&appids=389430&enddate=1787803791&feed=steam_community_announcements)
- NTTGame, [Full Moon Rift — giriş, kule, ölüm ve Jewel ödülleri](https://www.nttgame.com/knight/tr/news/views/1/3094)
- NTTGame, [Full Moon Rift 2024 giriş kuralları](https://www.nttgameonline.com/knight/tr/news/news_detail/1115)
- Proje README'sinde verilen repo: https://github.com/derdems44/KORustGameServer

> Not: Duyuru sayfaları zamanla güncellenebilir veya yönlendirilebilir. Tarih ve sayfa başlığını kontrol edin. Oyun içi özel sunucu değerlerini ayrıca doğrulayın.
