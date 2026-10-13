# Claude için uygulama şartnamesi: Knight Online 2625 yönetim paneli

Bu belge, mevcut Rust sunucusuna bağlı, üretimde kullanılabilir bir oyun yönetim panelinin tasarım ve uygulama gereksinimidir. Panel yalnızca görsel demo olmayacak; her değişikliğin hangi veriye yazıldığını, oyuna ne zaman yansıdığını ve hangi yetkiyle yapıldığını gösterecektir.

## Claude için başlangıç talimatı

Önce D:\ai\ko26xx deposunu ve çalışma ağacındaki değişiklikleri incele. Sunucunun kaynak kodu, migration’ları, modelleri ve mevcut admin ekranı tek doğruluk kaynağıdır. Var olan Rust/Axum panelini mümkün olduğunca genişlet; ayrıca PHP/Node/Python tabanlı ikinci bir servis veya paralel veri katmanı kurma. Çalışır özellik varsa yeniden yazmadan önce bul ve kullan.

İlk adımda kısa bir keşif özeti çıkar: mevcut panel mimarisi ve API’ler; ilgili DB tabloları; gerçek slot ve kapasite değerleri; çalışan oyun/event modülleri; canlı güncelleme ile restart gerektiren değişiklikler. Aşağıdaki tutarsızlıkları koddan çöz, uygulanabilir varsayımlar seç ve uygulamaya devam et. Çözülemeyen bir özelliği desteklenmiyor diye açıkça göster; sahte anahtar veya başarı mesajı üretme.

Kullanıcı değişikliklerini koru. Gerçek veritabanında veya uzak sunucuda kullanıcı istemeden işlem yapma. Kod, tablo ya da işlev varsayımını doğrulanmış gibi sunma.

## Depoya göre mevcut temel

- Sunucu Rust 2021, Tokio, PostgreSQL ve sqlx kullanıyor. DB erişiminde mevcut ko-db repository ve admin_web örüntüsünü kullan.
- Rust yönetim servisi ko-game içindeki admin_web.rs ve assets/admin/index.html üzerinden çalışıyor. Route kökü kaynakta /jstkoadminpanel/ olarak tanımlı; docs/admin-panel.md içindeki eski /admin/ açıklamasıyla farkı doğrula, mevcut URL’yi sessizce değiştirme.
- Kimlik doğrulama, kısa ömürlü oturum, CSRF/origin kontrolleri, giriş deneme sınırlaması, admin_users ve admin_audit_log altyapısı var. Yeni mutasyonları bu güvenlik ve denetim akışına bağla.
- jstkoadminpanel2 altındaki PHP panel ve görseller referans olabilir. Eski panelin 2369 varsayımlarını veya slot düzenini 2625 için doğru sayma. Rust 2625 kodu ve mevcut PostgreSQL migration’ları önceliklidir.
- Depoda jstkoadminpanel2/public/assets/img/itemicon altında item görselleri bulunuyor. item tablosundaki gerçek icon alanı üzerinden eşle; item numarasından dosya adı uydurma. Görsel bulunmazsa düzgün fallback kullan.
- Mevcut admin_web arayüzündeki envanter döngüsü 97 yuva oluşturuyor; kanonik Rust sabiti 96. Bu fark düzelmeden envanter tamamlanmış sayılmaz.

## Kesin slot ve veri doğruluğu

### 2625 karakter envanteri

Kanonik kaynak crates/ko-game/src/inventory_constants.rs. HTML içinde bu sayıları ayrıca ve bağımsız tanımlama.

| Dizi indeksi | Anlam | Adet | Görsel düzen |
|---|---|---:|---|
| 0–13 | Giyili ekipman | 14 | KO ekipman yerleşimi ve etiket |
| 14–41 | Ana çanta | 28 | 7 sütun × 4 satır |
| 42–52 | Cospre | 11 | Kostüm bölümü, 11 yuva |
| 53–88 | Magic bag | 36 | 3 çanta × 12; her biri 3 × 4 |
| 89–95 | Knight Royale ekipmanı | 7 | Ayrı ve adı belirtilmiş bölüm |
| 0–95 | Toplam item dizisi | 96 | 97. yuva oluşturma |

Ekipman indeksleri: 0 sağ küpe, 1 başlık, 2 sol küpe, 3 kolye, 4 göğüs, 5 omuz, 6 sağ el, 7 kemer, 8 sol el/kalkan, 9 sağ yüzük, 10 bacak, 11 sol yüzük, 12 eldiven, 13 ayakkabı. Ekran bu eşlemeyi korusun ve dizi sırasını yeniden numaralandırmasın.

Boş yuvaları göster. Kullanıcıya belirsiz “Slot 15” metni bırakma: bölüm, ekranda görülen numara ve gerçek 0 tabanlı DB indeksi detay/tooltip içinde birlikte gösterilsin. Backend kayıt sırasında bölüm ve indeksi tekrar doğrulasın.

### Normal, VIP ve clan kasaları

- Normal banka: WAREHOUSE_MAX 192 kayıt, ITEMS_PER_PAGE 24, toplam 8 sayfa. Her sayfa 6 × 4. Eski PHP panelindeki 28 kayıt/sayfa yaklaşımını kopyalama.
- VIP kasa: vip_warehouse_items migration’ı ile vip_warehouse.rs kodunu birlikte incele. DB’de 48 gerçek kayıt yuvası var. 2625 paketinde 192 kayıt alanı gönderilmesi, 192 aktif ve yazılabilir yuva olduğu anlamına gelmez. Sadece kalıcı 0–47 yuvaları düzenle. UI’da 24’lük iki görünüm sunulursa her hücrenin DB indeksi sabit kalmalı. Handler’daki sayfa/indeks davranışını uygulamadan önce uzlaştır; desteklenmeyen slotu aktifmiş gibi gösterme.
- Clan/guild bankası: guild_bank ve guild_bank_item tabloları ile guild_bank.rs içindeki runtime tab/slot indekslerini kullan. Migration yorumundaki tab adedi ile runtime sabitleri çelişirse gerçek davranışı doğrula. max_tabs, tab_index ve slot_id ayrı alanlardır. Klan seçimi ID ve isimle yapılsın; kayıt doğru knights_id ile bağlı olsun.
- Item yuvası için DB’de var olan metadata’yı koru: item_id, adet, durability, flag/original_flag, serial_num, expire_time ve kasaya özel alanlar. Bir alan değişti diye diğerlerini sıfırlama. Unique/stack kuralları ve serial doğrulansın.
- Kasa altınını item listesine karıştırma. Normal banka altını user_warehouse_coins; clan altını ilgili guild bank alanı üzerinden ayrı göster.

### Item arama ve yerleştirme

Item seçici ad veya tam ID ile arama, kategori filtresi, ikon, ad ve ID sunmalı. Aramada debounce ve büyük/küçük harf duyarsız eşleşme kullan. Bilinmeyen ID/görsel başka iteme dönüşmemeli.

Detay kartı, veritabanında gerçekten bulunan item özelliklerinden ad/ID, tür/kind/slot, ağırlık, stack sınırı, durability, sınıf/stat gereksinimleri, attack/defense, upgrade/element, trade, expiration, flag ve set/özel etki alanlarını gösterebilir. Kaynakta olmayan özelliği uydurma. İkonu gerçek item ikon alanından çöz.

Drag-and-drop yanında tıklayarak seçme ve dokunmatik kullanım olsun: “itemi seç → hedef yuvayı seç → uygula”. Taşıma/swap, item ekleme, silme ve adet düzenleme birbirinden anlaşılır eylemler olsun. Başarılı işlemden sonra gerçek backend state yeniden yüklensin; başarısız işlemde arayüz eski haline dönsün. Boş item ID’siyle silme davranışı kullanıcıya açıkça anlatılsın.

## Panel kapsamı

Arayüz Türkçe, responsive ve Knight Online görünümünde olmalı. Geniş ekranda sol navigasyon; küçük ekranda daraltılabilir menü. Başlıkta aktif sunucu, saat, bağlantı durumu ve admin adı görünür. Liste ekranlarında arama/filtre/sayfalama ile yükleniyor, boş ve hata durumları olmalı.

### 1. Genel bakış

- Sunucu ve DB erişim durumu, son heartbeat, aktif oyuncu sayısı.
- Çevrimiçi oyuncular, son girişler, cezalı hesaplar ve son admin işlemleri.
- Yaklaşan/aktif etkinlikler, son event durumu, restart bekleyen ayarlar.
- Dashboard hafif sorgularla çalışsın; her yenilemede büyük tabloları taramasın.

### 2. Hesaplar ve oyuncular

- Account ID, karakter adı, ulus, sınıf, level, rebirth, bölge, online durumu, loyalty, Noah, klan ve son girişe göre arama/filtre.
- Hesap ve karakter bilgilerini ayrı göster; hesap ayrıntısında tüm karakterleri listele. Parola, seal parolası ve OTP sırlarını gösterme.
- Desteklenen karakter düzenlemeleri: GM/normal oyun yetkisi, ulus/sınıf, seviye, exp, Noah, loyalty, bölge/konum ve ceza. Yetki kodlarını ve sınırları kaynak koddan doğrula.
- Hesap yetkisi ile karakter authority aynı alan değildir. GM yap/normal kullanıcıya çevir işlemi hangi alanı değiştirdiğini açıklasın. Oyun içi GM yetkisini web paneli admin erişiminden kesin olarak ayır.
- Karakter online ise envanter, stat ve kasa yazımını mevcut güvenli kurala göre kilitle. Online değişiklik desteklenecekse oturum belleği, DB ve client bildirimi birlikte tutarlı güncellenmeli; salt SQL yazımıyla canlı oturumda eski değer bırakılmamalı.
- KC = tb_user.cash_point; TL bakiyesi = tb_user.bonus_cash_point. Alanları ters çevirme. Account seçildiği görünür olsun; ilişkili karakter/account kontrolü yap. Ekle/çıkar ya da yeni bakiye belirle, önce/sonra ve sebebi göster. Negatif bakiye, overflow ve çift gönderim engellensin.
- Oyuncu online iken KC/TL için server’ın cash sync ve client packet akışını kullan veya işlemi engelle; SQL-only balance edit yapma.
- Oyunda desteklenen mevcut yollarla kick, force logout, ban/unban, mute, teleport ve item/mektup gönderimi sağla. Desteklenmeyen işlem için buton gösterme.

### 3. Envanter, banka, VIP ve clan bankası

Karakter seçilince aynı çalışma alanında Karakter Envanteri, Normal Banka, VIP Kasa ve Clan Bankası görünümleri olsun. Her görünümde seçilen karakter/account/clan ve online durumu görünür. Envanterde 14 ekipman yuvası, 28 slot çanta, cospre, magic bag ve Knight Royale bölümleri ayrı başlıklı olsun.

- İkonlu item seçici ile hedef yuvalar arasında drag/drop; yuvaya tıklayarak detay açma.
- Slot taşıma/swap, item ekleme/silme, count, durability, flag ve expiration düzenleme. Sadece desteklenen alanları göster.
- Çoklu seçimle temizleme/item ekleme mümkünse işlem önizleme, kapasite/stack kontrolü ve atomik backend kaydı olsun.
- Item template varlığı, boş yuva, stack uyumu, count sınırı, serial ve expiration doğrulansın. Unique item kopyalama riski varsayılan olarak engellensin.
- Normal banka account’a aittir; seçilen karakter hesabındaki diğer karakterlerin de aynı kasayı paylaşabileceği açıkça yazılsın.
- Clan bankası clan’a aittir; clan ID ve adı görünür. Kasa izinleri/kapasitesi gerçek DB/runtime değerinden gelsin.
- Online oyuncunun inventory mutasyonunda mevcut “online karakter kilitli” kuralını koru; runtime ve DB atomikliğini sağlamadan kilidi kaldırma.

### 4. Drop editörü ve grup itemleri

- Canavar araması template SID/ad, zone/spawn filtresi ve aktif spawn sayısıyla çalışsın. Template ile spawn kaydını karıştırma.
- Her canavarın 12 drop yuvasını ikonlu kart olarak göster: item veya group ID, ad, oran ve aktif/boş durumu.
- Oran alanı mevcut sunucu kodundaki raw 0–10000 aralığını korusun. UI yüzdeye çevirirse formülü gerçek drop roll kodundan doğrula ve ham değeri ayrıntıda göster.
- Tek item ya da make_item_group aransın. Grup seçildiğinde grup içeriği ikon/adla gösterilsin; drop’a grup ID’si kaydedildiği açık olsun. Group ID item template değildir.
- make_item_group üyeleri item aramasıyla eklenip çıkarılsın; sıralama, duplicate ve boş ID kontrol edilsin. Kaydetmeden önce tam önizleme sun.
- Toplu oran değişikliğinde etkilenecek canavar/yuva listesi, önce ve sonra değerleri kaydetmeden önce gösterilsin.
- Drop/grup güncellemesinden sonra mevcut reload/cache davranışını çalıştır; SQL’e yazmayı canlı sunucuya uygulandı diye sunma.

### 5. Etkinlik merkezi

Kaynakta bulunan event schedule ve sistemleri keşfedip tek listede sun: Lunar War/ülke savaşları, BDW, Chaos, Juraid, Forgotten Temple, Castle Siege, Bifrost, Cinderella/Collection Race ve repoda uygulanan diğer arena, room, world boss veya sezon etkinlikleri. Listeyi bu örneklerle sınırlama; kodda çalışmayan etkinliği aktifmiş gibi de gösterme.

- Her etkinliğin destek düzeyi: çalışır, sadece DB/takvim ayarı var, runtime kontrolü var veya kısmi/eksik. Takvim durumu, canlı runtime state ve son çalışması ayrı alanlardır.
- Takvim editörü: günler, birden fazla saat, saat dilimi, seviye şartı, giriş ücreti/loyalty, kayıt/oyun/savaş/finish/cleanup zamanları, event zone. Mevcut DB alanlarını kullan.
- Elle başlat/durdur/erken kapat işleminden önce aktif oda/oyuncu/zone etkisini göster. Devam eden event’i kapatmak için ayrıca açık onay al ve sonuç özetle.
- Ödül editörü: kazanan/kaybeden, item, adet, süre, EXP, loyalty, cash/KC ve Noah alanlarını ilgili event modeliyle düzenle. Item ID varlığını doğrula, ödül işlemi tekrar gönderimde iki kez uygulanmasın.
- “Takvimi etkinleştir” ile “hemen başlat” ayrı işlemlerdir; düğme gerçekte hangisini yaptığını açıkça yazsın.
- Saat dilimi ve DST geçişleri doğru olsun. Audit girdisinde event, event ID, zone, operatör, eski/yeni değer ve sonuç bulunsun.
- Bu iş yeni event kodu yazma talebi değildir. Sadece mevcut server mekanizmasının yönettiği ayarları açıp kapat. Runtime desteği olmayan özellikte sahte toggle gösterme.

### 6. Oyun dünyası ve içerik modülleri

Her modül verisini ve değişikliğin canlı/restart etkisini belirtmeli:

- NPC/canavar: npc_template, npc_spawn ve drop ilişkisi; spawn ekle/düzenle/sil, zone/koordinat/yön/sayı/respawn/aktiflik. Harita editörü varsa mevcut map assets ve gerçek koordinat dönüşümünü kullan.
- NPC satışları, item alış/satış grupları, item_exchange, upgrade olasılığı, item set/özel etki referansı: sadece server’ın yüklediği tablolar için düzenleyici.
- Quest/NPC Lua/TBL: var olan Quest Studio API, dosya güvenliği, syntax/parse doğrulaması ve reload bildirimi. Sınırsız dosya yolu yazımına izin verme.
- Server ayarları, EXP/NP/drop oranları ve combat config: kaynak, sınırlar, aktif etki ve restart gereksinimi göster.
- PUS/premium/kupon/promosyon ve cash kayıtlarını oyuncu balance düzeninden ayır. Alışveriş/iadeyi bozacak doğrudan DB yazımı yapma.
- Clan arama/üyeler/başkan/ulus/NP/puan/banka/izinleri. Transfer veya silme gibi eylemlerde ayrıntılı onay ve audit.
- Mektup/ödül gönderimi, duyuru, ceza, silinen item geçmişi ve yetkili logları.
- Genel SQL console veya her tabloya yazma aracı ekleme.

### 7. Yetki, denetim ve güvenlik

- Web paneli admin hesabı ile oyun içi GM ayrı kavramlardır. admin_users yetki yapısını incele; gerekiyorsa least-privilege varsayılanlı, geriye uyumlu migration kullan.
- Önerilen panel rolleri: Salt okunur, Destek, Oyun yöneticisi, Etkinlik yöneticisi, Sistem yöneticisi. Her mutasyon API’sinde backend yetki kontrolü olmalı; sadece butonu saklamak yeterli değil.
- KC/TL, GM yetkisi, item silme/kopyalama, toplu drop, ban, clan transferi ve event force-stop için sebep alanı zorunlu. Önce ve sonra değer gösterilsin.
- Mutasyonları admin_audit_log’a actor, action, target, timestamp, remote IP ve sır içermeyen eski/yeni değer özetiyle yaz.
- CSRF/origin kontrolleri, tipli JSON girdisi, sunucu tarafı aralık doğrulaması, gerekli yerde transaction ve idempotency uygula. Item mutasyonunda doğru karakter/account/clan ilişkisini backend’de doğrula.
- Panel loopback/private erişim kuralını koru. Secret’ları environment’dan al; log veya HTML’e yazma.

## Responsive görsel tasarım ve slot hizası

### Tasarım dili

- KO oyun arayüzüne yakın koyu taş/metal zemin, bronz/altın vurgular. Mavi ve kırmızı ulus bilgisinde kullanılabilir. Önce okunabilirlik ve operasyon kolaylığı.
- Masaüstü, tablet ve telefonda taşma olmayacak. Tablolar kendi içinde yatay kayabilir; modal viewport içinde kalmalı; mobil drag yerine seçim akışı sunulmalı.
- Item görseli kırpılmadan contain ile çizilsin. Adet alt köşede; upgrade/flag/expiry işaretleri tutarlı overlay olsun. Tooltip viewport dışına taşmasın.

### Slot geometri kuralları

- Item hücreleri flex-wrap ile viewport’a göre yeni kolonlara akmasın. Ana çanta 7×4, magic bag 3×4, normal banka sayfası 6×4 gibi mantıksal grid sabit kalsın.
- Kare aspect-ratio kullan. Ekran daralınca slotları ölçülü küçült veya grid alanını kendi içinde kaydır; hücre sırasını ve indeksini değiştirme.
- Her hücre gerçek data-container, data-owner-id, data-slot-index ve gerekirse data-tab-index taşısın. Drop hedefini ekran koordinatını bölerek hesaplama; DOM hücresinin indeksi backend indeksidir.
- Drag preview/target açıkça vurgulansın. Tıklama ve drag aynı item/owner/slot kaynağını kullansın. Başarı/hata sonrası backend state’i yeniden yükle.
- Ekipman mankeninin 14 yuvası sabit semantic/named grid yerleşiminden oluşsun. Görsel konum için absolute kullanılsa bile hit area ve indeks aynı slot bileşeninden türesin. Responsive CSS ekipman türlerini başka yuvayla eşleştirmesin.
- Boş/dolu/seçili/hedef/kilitli/hata durumları ayrışsın. Uzun isim hücreyi büyütmesin; detay paneli/tooltip kullansın. Debug detayında sıfır tabanlı indeks gösterilsin.

### UI kalite kontrolü zorunluluğu

Sadece derleyip bırakma. Gerçek tarayıcıda açıp 1920, 1366, 1024, 768 ve 390 CSS px genişliklerinde envanter, normal kasa, VIP, clan bankası, drop editörü ve mobil menüyü görsel incele. Screenshot/inceleme sonucunda hizalama hatalarını düzelt.

Envanterde 0, 13, 14, 41, 42, 52, 53, 88, 89 ve 95 indekslerine ayırt edilebilir fixture yerleştir; item hiçbir bölüme kaymamalı. Normal kasada 0, 23, 24, 167, 168, 191; VIP’te gerçek kapasitenin ilk/son indeksi; clan kasasında her tabın ilk/son yuvasını kontrol et. Ekranda drop edilen yuva ile kaydedilen indeks aynı olmalı.

Kontrol verisi gerçek oyuncu/kasa kayıtlarına yazılmamalı; test fixture veya sandbox kullan. Görsel incelemede gerçek sunucu verisini değiştirme. QA bitmeden “responsive” ya da “slotlar doğru” deme.

## Uygulama sırası

1. Mevcut çalışma ağacını ve Axum router/index/auth/API/audit yapısını incele.
2. Kanonik mapping çıkar: account/character/item/depo/event anahtarları; indeks/kapasite; online davranış; yetki; reload ve restart gereksinimi. Kod/migration çelişkilerini ilgili ekrana yazma açmadan çöz.
3. Panel kabuğu, dashboard, oyuncu arama ve ikonlu item araması.
4. Envanter ve üç kasa; sabit indeks, drag/tap, metadata ve doğrulanmış mutasyon.
5. Drop/group editörü; oran ve group ID akışı.
6. Etkinlik merkezi; takvim, runtime state, destek kapsamı ve ödül.
7. Oyuncu/hesap, KC/TL ve GM/normal dönüşümü; account/character ayrımı ve audit.
8. Oyun içeriği/operasyon modülleri; her biri gerçek veri ve reload durumuyla.
9. Görsel QA, güvenlik/işlem QA, migration uyumluluğu ve Türkçe kullanım özeti.

İş büyükse modülleri işlevsel dilimler halinde bitir. Boş buton, uydurma sayı, sahte başarı toast’u veya sonra bağlanacak ekran teslim sayılmaz.

## Tamamlandı kabul kriterleri

- Panel mevcut Rust/PostgreSQL uygulamasıyla çalışıyor; doğrulanmamış tablo veya servis yok.
- 96-slot envanter, 192-slot/8×24 normal banka, runtime’ın desteklediği VIP ve clan kapasiteleri doğru modellenmiş; hücre indeksleri sabit.
- Item araması doğru template/ikon döndürüyor; ekleme, kaldırma, taşıma ve stack davranışı sunucu semantiğine uygun.
- Drop editörü somut item ve make_item_group destekliyor; 12 yuva ve raw oran aralığı korunuyor; canlı cache davranışı uygulanıyor.
- Etkinlik takvimi, aç/kapat, elle başlat/durdur ve ödül editörü gerçek destek kapsamını ve restart etkisini doğru gösteriyor.
- KC/TL doğru account alanına yazılıyor; online state tutarlı; GM yetkisi web admin rolüyle karışmıyor.
- Mutasyonlarda sunucu tarafı doğrulama, atomiklik, idempotency, audit ve işlem sonrası yeniden yükleme var.
- Belirtilen viewport ve sınır indeksleri görsel QA’dan geçmiş; mobilde fare olmadan item yerleştirilebiliyor.
- Final özeti Türkçe; tamamlanan/kısmi/eksik modüller, migration/config ve doğrulama sonuçları doğru anlatılıyor. Kullanıcı ayrıca istemedikçe canlı deploy veya üretim veritabanı mutasyonu yapılmıyor.

## Başvurulacak kaynaklar

- Panel/API: crates/ko-game/src/admin_web.rs
- Mevcut arayüz: crates/ko-game/assets/admin/index.html
- Envanter sabitleri: crates/ko-game/src/inventory_constants.rs
- Normal kasa: crates/ko-game/src/handler/warehouse.rs ve migrations/20260209000021_user_warehouse.sql
- VIP kasa: crates/ko-game/src/handler/vip_warehouse.rs ve migrations/20260210000372_vip_warehouse.sql
- Clan bankası: crates/ko-game/src/handler/clan_warehouse.rs, crates/ko-game/src/handler/guild_bank.rs, migrations/20260310000005_guild_bank.sql
- Item/drop: crates/ko-db/src/repositories/item_tables.rs, migrations/20260207000005_create_item.sql, migrations/20260209000019_item_tables.sql, migrations/20260209000032_item_tables_extended.sql, migrations/20260210000053_item_group.sql ve ilgili monster_item/make_item_group migration’ları
- Etkinlik: crates/ko-game/src/systems/event_system.rs ve migrations/20260210000046_event_schedule.sql
- Auth/audit: migrations/20260928000001_admin_audit_log.sql, migrations/20260928000002_admin_users.sql, docs/admin-panel.md
- 2625 kaynak notları: docs/reverse-engineering/2625/ ve ilgili handler/protocol kaynakları

> Bu şartname ürün hedefini tanımlar. Migration, tablo yapısı, slot sınırı veya çalışan özellik değişmişse güncel kaynak ve schema doğrulanmalı; bu belge kaynak kodun yerine geçmez.
