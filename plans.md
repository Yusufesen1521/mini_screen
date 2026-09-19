# mini_screen yol haritasi

Masaustunde duran, bilgisayara USB ile baglanan ikinci ekran. Kullanimin
buyuk cogunlugu bilgisayara bagli halde gececek; bagimsiz mod ikincil ve
kasitli olarak basit tutulacak.

## Sabit mimari kararlar

Bunlar tartisilip karara baglandi, yeniden acilmayacak.

**1. Bagli modda cizimi PC yapar, cihaz sadece basar.**
Cihaz aptal bir cerceve. Butun render PC tarafinda, cihaza sadece sikistirilmis
piksel bolgeleri gider. Gorseli guzellestirmek, widget eklemek, tema degistirmek
PC tarafinda kod yazmak demek; gomulu tarafa dokunulmaz.

**2. Bagimsiz modda auth gerektiren hicbir sey yok.**
Cihaz yalniz basina kaldiginda sadece anahtarsiz ve basit isler yapar: saat,
hava durumu, kayitli GIF veya duvar kagidi. OAuth, token yenileme, sifreli
kimlik saklama gibi yukler firmware'e hic girmez. Auth isteyen ne varsa
(borsa, takvim, e-posta) bagli modda kalir, veriyi PC ceker ve hazir piksel
olarak gonderir.

**3. Medya bilgisi Spotify ya da YouTube API'sinden degil, isletim sisteminin
medya oturumundan okunur.**
Windows'ta SMTC, Linux'ta MPRIS. Tek kod ile Spotify, YouTube, VLC, Apple Music
ve digerleri kapsanir. OAuth yok, saglayici kilidi yok, ticari kullanimda
API sartlari riski yok.

**4. Telefondan BLE ile ayar yapilmayacak.**
Ayar icin cihazin kendi web arayuzu kullanilir. Tek kod, her platform, app
store yok. BLE sadece iOS bildirimleri (ANCS) icin kullanilacak.

**5. Performans hedefi 24 FPS tam kare esdegeri, ve bu hedef karsilandi.**
Bunun ustune cikmak icin is yapilmayacak. Olculen durum: baglanti
0.20 MB/s, arayuz iceriginde RLE 21 kat sikistiriyor, yani ekrana ulasan
efektif piksel hizi 4.2 MB/s. SPI'in olculen tavani 4.40 MB/s. Yani
baglanti ekranin basabileceginin yuzde 95'ini zaten besliyor; daha hizli
bir tasima ya da daha iyi bir sikistirma tek kare kazandirmaz.

Ustelik metrigin kendisi yapay: gercek kullanimda tam kare gonderilmiyor,
sadece degisen widget'in dikdortgeni gidiyor.

**6. Kendi kodegimiz sadece RLE16. Gerisi kutuphane.**
RLE16 bolge kodegi yazildi (cihazda 50, PC'de 30 satir) ve kalsin: duz
renkli arayuz iceriginde optimuma yakin, memcpy hizinda cozuluyor, ek RAM
istemiyor. Genel amacli bir kutuphane (LZ4, Heatshrink) bu is icin daha
yavas ve daha buyuk olurdu, ustelik fazladan sikistirmanin karsiligi yok.

Fotograf, GIF ve video icin kutuphane kullanilacak, kendi kodek yazilmayacak:
- GIF: `AnimatedGIF` (bitbank2, Apache 2.0)
- JPEG: `TJpg_Decoder` (Bodmer, ticari kullanima uygun)
- Video: MJPEG, yani JPEG dizisi. ESP32-S3'te H.264 cozmek gercekci degil.

**7. SPI hizi 40 MHz.**
Olculen 4.40 MB/s, teorigin yuzde 88'i. Uzun kullanimda bozulma gorulmedi.
80 MHz olculdu (7.87 MB/s) ama darbogaz orada olmadigi icin benimsenmedi;
breadboard uzerinde sinyal butunlugu riskine girmeye degmez.

**8. Hava durumu icin anahtarsiz kaynak.**
Open-Meteo: kayit yok, anahtar yok, gunde 10 bin cagri. Ileride ticari
kullanim olursa met.no alternatifi degerlendirilir (yine anahtarsiz, ticari
kullanima acik, tek sarti tanitici bir User-Agent).

## Calisma kurali

Bir faz bitmeden sonrakine gecilmez. "Bitti" demek, o fazin cikis
kriterlerinin tamaminin tek tek dogrulanmasi demek.

- Olcum gerektiren kriterlerde gercek sayi yazilir. "Iyi gorunuyor" kabul
  edilmez.
- Her faz sonunda README ve CLAUDE.md guncellenir.
- Her faz kendi commit serisiyle ilerler, faz bitince `git tag faz-N` atilir.
- Bir kriter karsilanamiyorsa faz bitmemistir. Ya cozulur ya da kriter
  gerekcesiyle birlikte yazili olarak degistirilir; sessizce atlanmaz.

## Fazlar

| Faz | Is | Durum |
|---|---|---|
| 0 | Ekran dogrulama | Bitti |
| 1 | Piksel hatti ve USB protokolu | Siradaki |
| 2 | PC uygulamasi iskeleti ve sistem degerleri | Bekliyor |
| 3 | Medya widget'i | Bekliyor |
| 4 | Layout ve ozellestirme motoru | Bekliyor |
| 5 | Bagimsiz mod | Bekliyor |
| 6 | iOS bildirimleri | Bekliyor |
| 7 | PC guc dugmesi | Bekliyor |

Seri uretim, sertifikasyon ve kasa tasarimi bu yol haritasinin disinda.
Yedi faz bittikten sonra ayrica ele alinacak.

---

## Faz 0: Ekran dogrulama (bitti)

ILI9341 paneli ESP32-S3 uzerinde calistirmak, renk sirasini ve offseti
dogrulamak, bolgesel guncellemenin dogru kuruldugunu gostermek.

Sonuc: ekran yatay 320x240 calisiyor, 320x48 bolge guncellemesi 7.1 ms.
Dokunmatik olmadigi olculerek dogrulandi. Ayrinti README icinde.

---

## Faz 1: Piksel hatti ve USB protokolu

Projenin omurgasi. Sonraki butun fazlar bu protokolun uzerine oturacak, o
yuzden burada acele etmek en pahali hata olur.

**Cevaplanacak soru:** Bu ekrana saniyede kac piksel basabiliyorum, hem
cihazin kendi flash'indan hem PC'den?

### Isler

**1.1 Ekran tarafinin tavanini olc** (bitti)
Olcum araci `src/bench_main.cpp`, sonuclar `docs/measurements.md`.

Ozet: 40 MHz'de 4.40 MB/s, 80 MHz'de 7.87 MB/s. Bolge boyutunun maliyeti
yok, PSRAM ile SRAM arasinda yuzde 2 fark var, bayt sirasi cevirmek bedava.
TFT_eSPI'nin DMA yolu ESP32-S3'te cokuyor ama gerekmiyor.

Bu olcumun protokole etkisi:
- **Ekran degil USB darbogaz.** SPI, USB'den 4.4 kat hizli. Kalan is
  ekrani hizlandirmak degil, USB tarafini dogru kurmak.
- **Sikistirma hedefi 4.4 kat.** Altinda USB, ustunde SPI sinirlar.
  Bu oranda tam kare hizi yaklasik 28 FPS.
- **Kucuk bolge cezasi yok**, protokol serbestce bolebilir.
- **Alim tamponlari PSRAM'de olacak.**
- **SPI 40 MHz kalacak**, 80 MHz gereksiz risk.

**1.2 Protokolu tasarla ve yaz**
- Cerceve yapisi: SOF, protokol surumu, mesaj tipi, uzunluk, payload, CRC
- Mesaj tipleri: HELLO ve CAPS (cihaz kendini tanitir: cozunurluk, renk
  formati, desteklenen sikistirmalar, tampon boyutu), FRAME_REGION,
  SET_BACKLIGHT, PING, ACK, NACK, ERROR
- Bolge tanimi: x, y, w, h, piksel formati, sikistirma tipi
- Akis kontrolu: ayni anda kac bolge ucusta olabilir
- Senkron kaybindan kurtarma: SOF arama ve yeniden hizalanma
- Surum uyusmazliginda temiz hata

**1.3 RLE codec**
- Cihazda decoder, PC'de encoder
- Ayni test vektorleriyle iki tarafta da dogrulanmis
- Sikismayan veride buyumeyi sinirla: RLE ciktisi hamdan buyukse ham gonder

**1.4 PC tarafi test araci**
- Henuz uygulama degil, sadece olcum araci
- CDC portunu VID/PID ile bulur
- Test desenleri gonderir: duz renk, satranc tahtasi, gradyan, gercek
  ekran goruntusu
- FPS, MB/s, CRC hata sayisi raporlar

**1.5 Cihazda GIF oynatma**
- `AnimatedGIF` kutuphanesi, LittleFS uzerinden
- Protokolun stres testi ve ayni zamanda bagimsiz mod icin hazir oynatici

**1.6 Protokolu belgele**
- `docs/protocol.md`: her mesaj tipinin bayt duzeyinde tanimi, ornek
  cercevelerle

### Cikis kriterleri

- [x] PC'den 320x240 tam kare gonderimi calisiyor, olculen FPS belgelenmis.
      Beklenti: ham veride 5-7 FPS (USB Full Speed siniri), UI benzeri
      icerikte RLE ile 25 FPS ustu.
- [x] Bolgesel guncelleme keyfi (x, y, w, h) dikdortgen ile calisiyor
- [x] Sinir kontrolu: ekran disina tasan istek reddediliyor, NACK donuyor,
      cihaz cokmuyor
- [ ] 30 dakika kesintisiz akis: senkron kaybi yok, CRC hatasi yok
- [x] Bellek sizintisi yok (5 dk kosuda +0 bayt): test basindaki ve sonundaki bos heap farki
      yuzde 1'in altinda
- [ ] Kablo akis ortasinda cekilip takildiginda cihaz kendini topluyor,
      reset gerekmiyor
- [x] Kasitli bozuk cerceve enjekte edildiginde cihaz cokmuyor, NACK donuyor
      ve sonraki gecerli cerceveyi isliyor
- [x] Protokol surum alani calisiyor: eski cihaz yeni PC ile konusursa
      ikisi de temiz hata veriyor, tanimsiz davranis yok
- [ ] 100 bin cerceve boyunca CRC hata sayisi 0
- [ ] GIF flash'tan oynuyor, kare zamanlamasi GIF'in kendi suresine yuzde 10
      dogrulukla uyuyor
- [x] `docs/protocol.md` yazilmis ve gercek kodla uyumlu

### Bilinen riskler

- USB Full Speed siniri (yaklasik 1 MB/s) asilamaz. Cozum sikistirma ve
  bolgesel guncelleme, daha hizli bir yol yok.
- Bloklayan push sirasinda CPU mesgul. Ekran itme ile USB alimi ayri
  cekirdeklere dagitilacak, yoksa alim sirasinda kare kaybi olabilir.
- 80 MHz gorsel kararliligi olculmedi. Ileride benimsenirse once uzun
  sureli bozulma testi gerekir.

### Kapanan riskler

- ~~80 MHz kararsiz olabilir~~: olculdu, 7.87 MB/s veriyor ama gerekmedigi
  icin benimsenmedi.
- ~~TFT_eSPI DMA yolu dikkat ister~~: ESP32-S3'te tamamen bozuk oldugu
  bulundu, kullanilmiyor ve gerekmiyor.

---

## Faz 2: PC uygulamasi iskeleti ve sistem degerleri

Ilk gercek icerik. Ama asil is tek widget yapmak degil, **ikinci widget'i
kolay yapacak iskeleti kurmak.** Tek widget varken bile widget soyutlamasi
yazilacak; bu asiri muhendislik degil, Faz 4'te bastan yazmayi onlemek.

### Isler

**2.1 Teknoloji secimi**
- Tek kod tabani, uc platform. Rust ve Tauri ya da Go onerilir; karar faz
  basinda verilecek ve gerekcesi buraya yazilacak.
- Sensor okuma platform basina tamamen farkli, eklenti katmani olarak ayrilacak.

**2.2 Cekirdek mimari**
- `Widget`: render(canvas, rect), guncelleme araligi, kirli mi
- `Renderer`: offscreen canvas, RGB565'e donusturme
- `Transport`: Faz 1 protokolunun PC tarafi
- `DirtyTracker`: sadece degisen dikdortgenleri gonder
- Widget kaydi: yeni widget eklemek tek dosya yazmak olmali

**2.3 Cihaz kesfi ve baglanti yonetimi**
- VID/PID ile otomatik bulma
- Cikarma ve tekrar takmada kendiliginden toparlanma

**2.4 Sistem sensorleri**
- Anahtarsiz ve yetkisiz alinabilenler once: CPU kullanimi, RAM, disk, ag
- NVIDIA GPU icin NVML, AMD icin ADLX (resmi SDK, surucu gerekmez)
- CPU sicakligi: v1'de HWiNFO paylasimli bellegi opsiyonel kaynak. Yoksa o
  alan gizlenir. Kendi imzali kernel surucusu bu fazin kapsaminda degil.
- Linux: /sys/class/hwmon ve /proc
- macOS: temel metrikler, sicaklik kapsam disi

**2.5 Sistem paneli widget'i**
- Sayilar, kucuk grafikler, hizli guncelleme

### Cikis kriterleri

- [ ] Uygulama Windows ve Linux'ta derleniyor ve calisiyor; macOS'ta en
      azindan derleniyor ve temel metrikleri gosteriyor
- [ ] Cihaz takilinca otomatik bulunuyor, cikarilinca uygulama cokmuyor,
      tekrar takilinca kendiliginden baglaniyor. Bu dongu 20 kez arka arkaya
      sorunsuz.
- [ ] Widget soyutlamasi kanitlanmis: ikinci bir sahte widget eklemek
      cekirdekte tek satir degisiklik gerektirmiyor
- [ ] Dirty tracking calisiyor: ekranda hicbir sey degismiyorken USB trafigi
      sifira yakin. Olculmus deger belgelenmis.
- [ ] Uygulama bosta CPU kullanimi yuzde 1'in altinda, calisirken yuzde 3'un
      altinda
- [ ] Eksik sensor kaynagi widget'i bozmuyor: GPU yoksa ya da HWiNFO kapaliysa
      o alan temiz sekilde gizleniyor, hata gostermiyor
- [ ] 24 saat kesintisiz calisma: bellek artisi yok, baglanti kopmasi yok
- [ ] Uygulama kapatildiginda cihaz makul bir ekrana dusuyor, donmus son
      kareyle kalmiyor

---

## Faz 3: Medya widget'i

Spotify, YouTube ve digerleri. Sanildigindan kucuk bir is: OS medya oturumu
okunacak, API cagrisi ve OAuth yok.

### Isler

**3.1 Platform okuyuculari**
- Windows: SMTC (GlobalSystemMediaTransportControlsSessionManager)
- Linux: MPRIS, D-Bus uzerinden
- macOS: hangi kaynaklarin mumkun oldugu arastirilip durust bir kapsam
  yazilacak. MediaRemote kapandigi icin tarayici medyasi muhtemelen alinamaz.

**3.2 Album kapagi**
- Thumbnail alma, yeniden boyutlama, RGB565'e donusturme
- Gradyanlarda bantlasmayi onlemek icin dither

**3.3 Metin**
- UTF-8, Turkce karakterler dogru
- Uzun baslik: kaydirma ya da kisaltma, tasma yok

**3.4 Kontrol icin hazirlik**
- Play, pause, next komutlari yazilacak ama tetikleyecek buton henuz yok.
  Protokolde cihazdan PC'ye olay mesaji icin yer ayrilacak.

### Cikis kriterleri

- [ ] Spotify uygulamasi, Chrome'da YouTube ve VLC ile test edilmis; ucunde
      de baslik, sanatci ve kapak dogru geliyor
- [ ] Kaynak degisince (Spotify'dan YouTube'a) widget 1 saniye icinde geciyor
- [ ] Hicbir sey calmiyorken temiz bir bos durum gosteriliyor
- [ ] Kapak degisimi sadece kapak dikdortgenini guncelliyor, tum ekrani degil.
      Olculmus bayt farki belgelenmis.
- [ ] Turkce karakterler dogru goruntuleniyor
- [ ] 100 karakterlik sarki adi tasmiyor ve layout'u bozmuyor
- [ ] Medya oynatici cokerse ya da kapanirsa widget donmuyor, bos duruma
      geciyor
- [ ] Hizli sarki degisiminde (10 saniyede 20 kez ileri) widget takip ediyor,
      kuyruk birikmiyor

---

## Faz 4: Layout ve ozellestirme motoru

Artik elde uc farkli karakterde widget var: hizli guncellenen sayilar, gorsel
iceren medya karti, yavas guncellenen hava durumu. Motor artik tahmine degil
gercek ihtiyaca gore tasarlanabilir.

### Isler

**4.1 Layout modeli**
- Grid mi serbest yerlesim mi, karar ve gerekce
- JSON sema, surum alani ile

**4.2 Editor**
- Surukle birak, widget ekleme ve cikarma, boyutlandirma
- Canli onizleme

**4.3 Tema**
- Renk paleti, font, arka plan
- En az iki hazir tema

**4.4 Profiller**
- Birden fazla layout, aralarinda gecis

**4.5 Sema surumleme**
- Eski config dosyasi yeni surumde acilmali, gerekiyorsa otomatik donusturulmeli

### Cikis kriterleri

- [ ] Uc widget da suruklenip yerlestirilebiliyor ve boyutlandirilabiliyor
- [ ] Kaydedilen layout uygulama yeniden baslayinca aynen geliyor
- [ ] Onizleme ile cihazdaki goruntu birebir ayni. Piksel karsilastirmasiyla
      dogrulanmis, fark yuzde 0.
- [ ] Gecersiz layout editorde engelleniyor: ust uste binme, ekran disina
      tasma, sifir boyut
- [ ] Onceki surumden kalma config dosyasi aciliyor ve bozulmuyor
- [ ] Yeni widget tipi eklendiginde editor onu kod degisikligi olmadan
      taniyor ve listeliyor
- [ ] En az iki tema calisiyor, tema degisimi tum widget'lara uygulaniyor
- [ ] Layout degisikligi cihaza aninda yansiyor, yeniden baslatma gerekmiyor

---

## Faz 5: Bagimsiz mod

Cihaz PC'siz ayaga kalkiyor. Kapsam kasitli olarak dar: saat, hava durumu,
kayitli gorsel. Auth gerektiren hicbir sey yok.

### Isler

**5.1 WiFi kurulumu**
- Cihaz SoftAP aciyor, ekranda QR gosteriyor
- QR icinde SoftAP bilgisi var, telefon otomatik baglaniyor, kurulum sayfasi
  aciliyor
- Ayarlar NVS'te saklaniyor

**5.2 Saat**
- SNTP senkronizasyonu, zaman dilimi ayari
- Baglanti kesildiginde ic saatle devam

**5.3 Hava durumu**
- Open-Meteo istemcisi, anahtar yok
- Konum kullanici tarafindan seciliyor
- 15 dakikada bir guncelleme

**5.4 Yerel icerik**
- LittleFS uzerinde GIF ve duvar kagidi
- PC uygulamasindan dosya yukleme

**5.5 Mod gecisi**
- USB takili mi degil mi algilama
- Iki mod arasinda yumusak gecis

### Cikis kriterleri

- [ ] Ilk acilista SoftAP aciliyor, QR gosteriliyor, telefonla kurulum 60
      saniyenin altinda tamamlaniyor
- [ ] Yanlis WiFi sifresi girilirse cihaz kurtarma moduna donuyor, kilitlenmiyor
- [ ] WiFi kesildiginde cihaz yeniden baglanmaya calisiyor, ekran donmuyor,
      son bilinen veriyi eski oldugu belli olacak sekilde gosteriyor
- [ ] Hava durumu cagrisi basarisiz olursa onceki veri korunuyor, ekranda
      hata yigini gorunmuyor
- [ ] Saat SNTP ile senkron, ag kesintisinde kaymaya devam etmiyor
- [ ] USB takilinca 2 saniye icinde bagli moda geciyor, cikinca bagimsiz moda
      donuyor. Bu dongu 20 kez arka arkaya sorunsuz.
- [ ] PC'den GIF yuklenebiliyor; yukleme ortasinda kesilirse dosya sistemi
      bozulmuyor ve onceki dosya korunuyor
- [ ] 48 saat kesintisiz bagimsiz calisma: reset yok, bellek sizintisi yok
- [ ] Cihazda hicbir kullanici kimligi ya da token saklanmiyor (kod
      incelemesiyle dogrulanmis)

---

## Faz 6: iOS bildirimleri (ANCS)

BLE uzerinden Apple Notification Center Service. iPhone'a uygulama kurmaya
gerek yok. Android bu fazin disinda; gerekirse ayri bir faz olarak ele alinir.

### Isler

- BLE ANCS istemcisi ve eslesme akisi
- Bildirim gosterimi, UTF-8 metin
- Uygulama bazli filtre
- Gizlilik anahtari: icerigi gizleyip sadece uygulama adini gosterme

### Cikis kriterleri

- [ ] Eslesme bir kez yapiliyor, sonraki acilislarda otomatik baglaniyor
- [ ] Bildirim geldikten sonra 2 saniye icinde ekranda
- [ ] Uygulama bazli filtre calisiyor
- [ ] Turkce karakterler dogru; emoji gosterilmese bile cokmeye yol acmiyor
- [ ] BLE ve WiFi ayni anda calisiyor. 2.4 GHz cakismasinin etkisi olculmus
      ve belgelenmis.
- [ ] Hizli gelen 50 bildirimde kuyruk tasmiyor, eskiler duzgun dusuyor
- [ ] Gizlilik anahtari calisiyor
- [ ] Telefon menzil disina cikip geri geldiginde baglanti kendiliginden
      kuruluyor

---

## Faz 7: PC guc dugmesi

### Isler

- Wake-on-LAN istemcisi (varsayilan yol)
- Opsiyonel donanim: optokuplor ile anakart PWR_SW basligina baglanti
- PWR_LED okuyarak PC durumunu bilme
- Kisa basis ve uzun basis ayrimi
- PC uygulamasi uzerinden duzgun kapatma
- PC kapaliyken cihazin beslemede kalmasi

### Cikis kriterleri

- [ ] WoL ile acma en az iki farkli anakartta test edilmis, sonuclar yazili
- [ ] Donanim yolunda kisa basis aciyor, uzun basis zorla kapatiyor
- [ ] PC durumu (acik, kapali) ekranda dogru gosteriliyor
- [ ] Yanlislikla basma korumasi var: tek dokunusla kapanmiyor, onay istiyor
- [ ] PC kapaliyken cihaz beslemede kaliyor ve bagimsiz modda calisiyor
- [ ] Duzgun kapatma PC uygulamasi uzerinden yapiliyor, zorla kesme sadece
      kullanici acikca isterse

---

## Kapsam disi

- Dokunmatik: donanimda yok, olculdu
- LVGL: bagli modda PC ciziyor, bagimsiz mod ise cok basit. Gerekmiyor.
- BLE ile telefondan ayar: web arayuzu daha iyi bir cozum
- Android bildirimleri: kendi uygulamasini ve Play politikasini gerektiriyor
- Bagimsiz modda borsa, takvim, e-posta: auth gerektirdigi icin bagli modda
- Seri uretim, sertifikasyon, kasa: yedi faz bittikten sonra

## Ertelenen kararlar

- **Panel degisimi.** Mevcut ILI9341 TN panelin gorus acisi masaustu kullanim
  icin zayif. Urunlesme dusunulurse IPS panele gecilecek. Faz 1'deki olcumler
  bu karari besleyecek.
- **Daha hizli baglanti.** USB Full Speed siniri yaklasik 1 MB/s. Tam ekran
  yuksek kare hizi gerekirse ESP32-P4 (USB 2.0 High Speed) degerlendirilir.
  Su anki hedefler icin gerekmiyor.
- **Kendi imzali sensor surucusu.** CPU sicakligi icin. EV sertifikasi ve
  Microsoft attestation gerektiriyor, urunlesme kararina bagli.
