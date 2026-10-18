JSTKO 2625 WebView2 Power Up Store
==================================

Bu paket KnightOnline.exe içindeki resmi mağaza kimlik doğrulama adresini
JSTKO mağaza servisine yönlendirir. PUS düğmesi yine oyunun kendi WebView2
penceresini açar; dış tarayıcı kullanılmaz.

Uygulama:

  powershell -ExecutionPolicy Bypass -File .\patch-pus-url.ps1 `
    -ClientExe "D:\oyun\KnightOnline.exe"

Betik ilk çalıştırmada KnightOnline.exe.pus-backup adlı yedek oluşturur.
Yalnızca 2625 EXE içindeki tek PUS URL alanını değiştirir; beklenen adres
bulunamazsa dosyaya dokunmadan hata verir.

Sunucu gereksinimleri:

  PUS_BIND_ADDR=0.0.0.0:8081

VPS güvenlik duvarında TCP 8081 portu açık olmalıdır. Canlı kullanımda bir
alan adı ve HTTPS ters vekili kullanılırsa StoreUrl bu HTTPS adresiyle yeniden
yamanmalıdır.
