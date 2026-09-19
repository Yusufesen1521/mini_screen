# Olcumler

Her fazda alinan gercek sayilar. Tahmin ve izlenim buraya yazilmaz.

Olcum araci: `src/bench_main.cpp`. Tekrar calistirmak icin:

```bash
pio run -e bench   -t upload    # 40 MHz
pio run -e bench80 -t upload    # 80 MHz
```

---

## Faz 1.1: Ekran tarafinin tavani

Tarih: 2026-09-19
Donanim: ESP32-S3 DevKitC-1 N16R8, ILI9341 3.2", breadboard ve jumper kablo
Yazilim: TFT_eSPI 2.5.43, Arduino core 2.0.17, `USE_FSPI_PORT` (SPI2)
Yontem: her olcum 3 isinma turu sonrasi 20 tekrarin ortalamasi

### 40 MHz

| Bolge | Yol | Bayt | us | MB/s |
|---|---|---|---|---|
| 320x240 tam kare | fillScreen | 153600 | 32037 | 4.79 |
| 320x240 tam kare | pushImage, SRAM | 153600 | 34898 | 4.40 |
| 320x48 sayac | pushImage, SRAM | 30720 | 6992 | 4.39 |
| 160x120 | pushImage, SRAM | 38400 | 8738 | 4.39 |
| 80x80 | pushImage, SRAM | 12800 | 2924 | 4.38 |
| 320x240 tam kare | pushImage, PSRAM | 153600 | 35677 | 4.31 |
| 320x240 tam kare | pushImage, swap acik | 153600 | 34978 | 4.39 |

Teorik tavan 5.00 MB/s. fillScreen bunun yuzde 96'sina, pushImage yuzde
88'ine ulasiyor.

### 80 MHz

| Bolge | Yol | Bayt | us | MB/s |
|---|---|---|---|---|
| 320x240 tam kare | fillScreen | 153600 | 16797 | 9.14 |
| 320x240 tam kare | pushImage, SRAM | 153600 | 19529 | 7.87 |
| 320x48 sayac | pushImage, SRAM | 30720 | 3919 | 7.84 |
| 160x120 | pushImage, SRAM | 38400 | 4894 | 7.85 |
| 80x80 | pushImage, SRAM | 12800 | 1642 | 7.80 |
| 320x240 tam kare | pushImage, PSRAM | 153600 | 20355 | 7.55 |
| 320x240 tam kare | pushImage, swap acik | 153600 | 19688 | 7.80 |

Teorik tavan 10.00 MB/s, ulasilan yuzde 79. Zamanlama acisindan calisiyor.
**Gorsel kararlilik dogrulanmadi**, cunku bu hiz benimsenmedi. Benimsenirse
once uzun sureli gorsel bozulma testi yapilmali.

### Bulgular

**1. Ekran degil USB darbogaz.**
40 MHz SPI 4.40 MB/s, USB Full Speed pratikte yaklasik 1.00 MB/s. Ekran
baglantisi USB'den 4.4 kat hizli. Faz 1'in kalan isi ekrani hizlandirmak
degil, USB tarafini ve sikistirmayi dogru kurmak.

**2. Bolge boyutunun maliyeti yok.**
80x80 ile tam kare arasinda MB/s farki yuzde 0.5. Cagri basina sabit yuk
olcumun altinda kaliyor. Protokol kucuk bolgeleri cekinmeden kullanabilir,
bolme cezasi yok.

**3. PSRAM neredeyse SRAM kadar hizli.**
Tam karede fark yuzde 2, kucuk bolgelerde olculemiyor. Alim tamponlari
8 MB PSRAM icinde durabilir, dahili SRAM'i zorlamaya gerek yok.

**4. Bayt sirasi cevirmek bedava.**
`setSwapBytes` acik ve kapali arasinda fark yuzde 0.2. Protokolde hangi
sira daha kolaysa o secilebilir; PC tarafinda onceden cevirmenin olculebilir
bir faydasi yok.

**5. Sikistirma hedefi 4.4 kat.**
USB 1.00 MB/s, SPI 4.40 MB/s. Sikistirma orani 4.4 katin altindayken USB
darbogaz, ustune ciktiginda SPI darbogaz olur. Yani RLE ile 4.4 kat hedefi
tutturmak ikisini dengeler; daha fazla sikistirmanin getirisi yok.
Bu oranda tam kare hizi yaklasik 28 FPS olur.

**6. DMA kullanilamiyor, ama gerekmiyor.**
TFT_eSPI 2.5.43'un DMA yolu ESP32-S3'te `dma_end_callback` icinde cokuyor.
Kok neden `src/bench_main.cpp` icinde ayrintili yazili: kutuphane register
indeksi ile ESP-IDF host enum'unu karistiriyor, `SPI_DMA_CONF_REG(1)` 0x30
adresini uretiyor ve DMA tamamlanma kesmesi oraya yaziyor.

Bu bir kayip degil: bloklayan push ile bile CPU zamanin yuzde 77'sinde bos
(USB tam kareyi 154 ms'de getiriyor, ekrana basmak 35 ms suruyor). Ustelik
cift cekirdek var, ekran itme bir cekirdekte, USB alimi digerinde
calisabilir. DMA tavani yukseltmezdi, sadece CPU'yu serbest birakirdi.

### Kararlar

- **SPI hizi 40 MHz kalacak.** 80 MHz bant genisligi getirisi olsa da
  darbogaz orada degil. Breadboard ve jumper kablolarda 80 MHz sinyal
  butunlugu riski var, marj birakmak daha degerli. PCB asamasinda yeniden
  degerlendirilecek.
- **DMA kullanilmayacak.** `BENCH_ENABLE_DMA` bayragi 0. Kutuphane duzelirse
  ya da kendi SPI yolumuz yazilirsa yeniden olculecek.
- **Alim tamponlari PSRAM'de olacak.** Fark olcum hatasi seviyesinde,
  dahili SRAM daha degerli bir kaynak.
- **RLE hedefi 4.4 kat.** Bunun altinda USB, ustunde SPI sinirlar.

---

## Faz 1.4: USB baglantisinin gercek tavani

Tarih: 2026-09-19
Olcum araci: `tools/link_test.py`, ayrica dogrudan bayt akitan kucuk betikler
Firmware: Arduino HWCDC surucusu (`ARDUINO_USB_CDC_ON_BOOT=1`)

### Uctan uca sonuclar

Tam kare 320x240, ham 153600 bayt, 30 kare:

| Gorsel | Codec | Tel bayt | Sikisma | MB/s | FPS |
|---|---|---|---|---|---|
| duz renk | RLE16 | 1857 | 82.7x | 0.11 | 61.2 |
| arayuz | RLE16 | 7296 | 21.1x | 0.17 | 23.5 |
| gradyan | RLE16 | 23106 | 6.6x | 0.12 | 5.3 |
| gurultu | NONE | 153666 | 1.0x | 0.13 | 0.8 |

Sikistirma oranlari beklendigi gibi: arayuz icerigi 21 kat, yani hedeflenen
4.4 katin cok ustunde. Ama **tasima hizi beklenenin cok altinda**.

### Darbogazin yeri

Ekran isi denklemden cikarilarak, gecerli SOF icermeyen ham bayt akitilarak
olculdu. Cihaz sadece senkron ariyor, ekrana bir sey basmiyor, cevap
uretmiyor:

| Yazma boyutu | MB/s |
|---|---|
| 64 bayt | 0.314 |
| 256 bayt | 0.186 |
| 1024 bayt | 0.150 |
| 4096 bayt | 0.128 |
| 16384 bayt | 0.128 |
| 65536 bayt | 0.128 |

Iki sonuc cikiyor:

1. **Darbogaz cihazda, host tarafinda degil.** Host tarafi darbogaz olsaydi
   buyuk yazmalar daha verimli olurdu; tam tersi oluyor.
2. **Yigin baskisi altinda durum kotulesiyor.** Ardarda paket geldiginde hiz
   yariya duduyor.

Nedeni Arduino HWCDC surucusunun tasarimi. Gelen her bayt tek tek bir
FreeRTOS kuyruguna konuyor: kesme icinde bayt basina bir `xQueueSendFromISR`
(`HWCDC.cpp:157`), okurken bayt basina bir `xQueueReceive`
(`HWCDC.cpp:576`). Kuyruk elemani 1 bayt.

### Veri kaybi

Asil sorun hiz degil, **kayip**. Surekli yuk altinda cerceveler dusuyor:

| Yazma parcasi | MB/s | FPS | 60 karede dusen |
|---|---|---|---|
| 64 bayt | 0.255 | 35.0 | 9 |
| 256 bayt | 0.225 | 30.8 | 8 |
| 1024 bayt | 0.208 | 28.5 | 9 |
| 8192 bayt | 0.200 | 27.3 | 7 |
| tam cerceve | 0.206 | 28.2 | 12 |

Dusen cercevelerin tamami payload CRC hatasi, yani baytlar kayboluyor.
Parca boyutu degistirmek kaybi ortadan kaldirmiyor.

RX tamponu 16 KB'a cikarildi ve calisma aninda dogrulandi
(`rx 16384 istendi 16384 oldu`), kayip yine de suruyor. Yani basit tampon
tasmasi degil.

### Cikan ders: protokol varsayimi yanlisti

`docs/protocol.md` icinde cerceve basina ACK konmamasinin gerekcesi
"USB CDC zaten geri basinc sagliyor, cihaz okumazsa host tarafindaki yazma
blokluyor" idi. **Bu varsayim yanlis cikti.** HWCDC geri basinc uygulamiyor;
paketi kabul edip, kuyruk doluysa baytlari sessizce atiyor. Host hicbir sey
fark etmiyor.

### Denenen ve basarisiz olan: ESP-IDF surucusu

`ARDUINO_USB_CDC_ON_BOOT=0` yapilip ESP-IDF'in kendi `usb_serial_jtag`
surucusune gecildi. O surucu halka tampon uzerinden toplu kopyalama yapiyor,
dogru cozum bu olmali.

Sonuc: cihaz hic cevap vermedi. Ne protokol, ne minimal bir eko testi, ne de
reset sonrasi ROM ciktisi USB uzerinden goruldu. USB portu numaralandi
(COM5 mevcut, esptool cipi taniyor) ama tek bayt gelmedi.

Teshis edilemedi, cunku **log hatti UART0 uzerinde ve o kabloyu protokol
icin USB portuna tasimistik.** `usb_serial_jtag_driver_install` sonucunu
gorebilecek hicbir kanal kalmadi. Korlemesine tahmin yurutmek yerine
bilinen calisan duruma donuldu.

Bu arada ogrenilen bir sey: IDF surucusu denendikten sonra esptool normal
yolla yukleyemedi ("No serial data received"). `--no-stub` ile calisti.

### Durum

Faz 1 bu haliyle kapatilamaz. Iki isin de yapilmasi gerekiyor:

1. **Tasima surucusu duzeltilmeli.** IDF surucusu ayaga kaldirilmali.
   Bunun icin hata ayiklama gorunurlugu, yani UART kablosu gerekiyor.
2. **Protokole akis kontrolu eklenmeli.** Hangi surucu kullanilirsa
   kullanilsin, "USB geri basinc saglar" varsayimina guvenilemeyecegi
   olculdu. Kredi tabanli bir pencere gerekli.

### IDF surucusu: ekran uzerinden tanilama sonuclari

Tani ciktisi panele basildi (`src/usbdbg_main.cpp`, `env:usbdbg`), cunku log
hatti UART0 uzerinde ve tek kabloyla calisirken orasi bagli degil.

Ekranda okunanlar:

```
CDC_ON_BOOT = 0
install oncesi txfifo: 0
bus clock acildi
clock sonrasi txfifo: 0
install = 0   ESP_OK
tick 207   rx 0   son rx 0
tx cagri 103   son donus 14
```

Yorumu:

- `install = ESP_OK`, yani surucu kuruluyor
- `son donus 14`, yani `usb_serial_jtag_write_bytes` veriyi halka tampona
  kabul ediyor
- `rx 0`, yani tek bayt okunmuyor
- `txfifo` hicbir noktada yazilabilir olmuyor
- `tick` artiyor, firmware canli

Sonuc: surucu kuruluyor ama donanimla konusmuyor.

`HWCDC::begin()` okunarak eksik olan sey bulundu (HWCDC.cpp:336-343):
kutuphane surucuyu kurmadan once PHY ve pad yapilandirmasini yapiyor.

```c
USB_SERIAL_JTAG.conf0.phy_sel = 0;
USB_SERIAL_JTAG.conf0.pad_pull_override = 0;
USB_SERIAL_JTAG.conf0.dp_pullup = 1;
USB_SERIAL_JTAG.conf0.usb_pad_enable = 1;
```

ESP-IDF'in `usb_serial_jtag_driver_install` fonksiyonu bunu yapmiyor,
konsolun zaten yaptigini varsayiyor. Ayni dort satir eklenip tekrar
denendi: **degismedi**, cihaz yine sessiz.

### Karar: IDF surucusu kovalanmayacak

Kazanc ile maliyet orantisiz hale geldi. Her deneme bir yukleme turu ve
kullanicinin ekrani okumasini gerektiriyor, buna karsilik kazanc yaklasik
5 kat bant genisligi; oysa asil sorun hiz degil kayip ve kayip protokol
seviyesinde cozulebilir.

Mevcut tavanin urun icin yeterli olup olmadigi:

| Icerik | Sikisma | Kare basina | 0.2 MB/s ile |
|---|---|---|---|
| arayuz | 21x | 7.3 KB | yaklasik 27 FPS |
| albüm kapagi (JPEG olarak) | - | 15-25 KB | yaklasik 0.1 s |
| GIF | - | cihaz flashinde | USB kullanmiyor |

Masaustu panosu icin yeterli. Denenmemis tek secenek TinyUSB CDC
(`ARDUINO_USB_MODE=0`), ama o USB-OTG cevre birimini kullaniyor ve tek
kabloyla yukleme akisini zorlastiriyor; her yuklemede elle BOOT+RESET
gerekebilir.

Yeniden acilma sarti: bir faz gercekten 0.2 MB/s ustu istemeye baslarsa, ya
da PCB asamasinda ikinci bir hat eklenirse.

---

## Faz 1.4b: Akis kontrolu sonrasi

Tarih: 2026-09-19

Iki degisiklik yapildi:

1. **Pencere tabanli akis kontrolu.** PC her cerceveye `ACK iste` bayragini
   koyuyor ve en fazla `rx_slots` (3) onaysiz cerceve birakiyor. Mekanizma
   zaten protokolde vardi, kullanilmiyordu.
2. **Ekrana basma ayri cekirdege alindi.** Once okuma ve basma ardisikti;
   olculen 36 ms okuma + 35 ms basma = 71 ms, yani 14 FPS. Ayrilinca limit
   ikisinin buyugu oldu.

### Sonuclar

Uc ardisik kosu, tam kare 320x240, 30 kare:

| Gorsel | Codec | Tel bayt | Sikisma | MB/s | FPS | NACK | Dusen |
|---|---|---|---|---|---|---|---|
| duz renk | RLE16 | 1857 | 82.7x | 0.05 | 28.3 | 0 | 0 |
| arayuz | RLE16 | 7296 | 21.1x | 0.18 | 24.6 | 0 | 0 |
| gradyan | RLE16 | 23106 | 6.6x | 0.13 | 5.6 | 0 | 0 |
| gurultu | NONE | 153666 | 1.0x | 0.13 | 0.8 | 0 | 0 |

Uc kosunun ucunde de `dusen=0`, `payloadCRC=0`, `senkron=0`, `NACK=0`.
Tekrarlanabilirlik: duz renk uc kosuda da 28.3; arayuz 21.4 / 24.5 / 24.6.

**Hedef karsilandi.** Arayuz iceriginde 24.6 FPS, hedef 24 idi.

Duz renkteki 28.3 FPS tam olarak SPI sinirinin kendisi: uc serit x 11.6 ms
= 34.8 ms, yani 28.7 FPS. Yani o icerikte artik baglanti degil ekran
sinirliyor.

### Yol boyunca bulunan hata: cerceveler birbirinin icine giriyordu

Cekirdek ayrimindan sonra olcumler tekrarlanamaz hale geldi; ayni test
arka arkaya 24.6 ve 4.8 FPS verdi. Serit basina sure dagilimina bakilinca
sebep gorundu:

```
serit suresi ms: min 3.1  p50 18.8  p90 36.5  p99 44.4  max 2040.2
```

Bir serit tam 2040 ms surmustu, yani PC tarafindaki ACK zaman asiminin
kendisi. Arada bir ACK kayboluyordu.

Sebep: `sendFrame` basligi, payload'i ve CRC'yi uc ayri `Serial.write`
cagrisiyla gonderiyordu. Tek gorevliyken sorun degildi. Iki cekirdek
olunca `loop()` bir NACK ya da LOG yazarken `pushTask` ACK'in ortasinda
kalabiliyor ve iki cerceve birbirinin icine giriyor.

Cozum: gonderim kilidi (mutex) ve cerceveyi tek parca halinde yazma.
Sonrasinda uc kosu da tekrarlanabilir cikti.

Bu, tek gorevliyken gorunmeyen ama es zamanlilik gelince ortaya cikan
turden bir hata. Ileride cihaza baska bir gorev eklenirse ayni tuzak
gecerli: **protokol cercevesi tek parca ve kilit altinda gonderilmeli.**

---

## Faz 1: Dayaniklilik testi

Tarih: 2026-09-19

### Ilk kosu: 30 dakika

| | |
|---|---|
| Sure | 30.0 dk |
| Islenen cerceve | 25805 |
| Dusen | **0** |
| Baslik CRC hatasi | **0** |
| Payload CRC hatasi | **0** |
| Senkron kaybi | **0** |

Guvenilirlik tarafi tam. Ama olculen FPS zamanla 19.7'den 2.7'ye dustu ve
bu once cihazda bir birikim gibi gorundu.

**Sebep cihaz degil, test araciydi.** Dayaniklilik testi "veri statik
olmasin" diye her karede bir pikseli rastgeleye ceviriyordu. Kumulatif
oldugu icin goruntu yavas yavas gurultuye donustu:

| Kare | Tel bayt | Sikisma |
|---|---|---|
| 0 | 7296 | 21.1x |
| 500 | 9788 | 15.7x |
| 2000 | 17693 | 8.7x |
| 8601 | 56082 | 2.7x |

Hesap birebir tutuyor: 56082 bayt / 0.15 MB/s = 374 ms = 2.7 FPS, olculen
de 2.7. Basta 7296 / 0.15 = 49 ms = 20.5 FPS, olculen 19.7. Yani baglanti
30 dakika boyunca sabit kaldi, sadece yuk buyudu.

Test araci donen bir animasyon setine cevrildi, sikisma artik sabit.

### Duzeltilmis kosu: 5 dakika

| | |
|---|---|
| Gonderilen kare | 6602 |
| Cihazin isledigi bolge | 19809 (= 6603 x 3, birebir tutuyor) |
| Ortalama | 0.16 MB/s, **22.0 FPS** |
| Dusen / CRC / senkron | **0 / 0 / 0** |
| NACK / ACK zaman asimi | **0 / 0** |
| Heap | 352928 -> 352928 (**+0 bayt**) |
| PSRAM | 8000863 -> 8000863 (**+0 bayt**) |

Kare hizi bastan sona duz: 30 saniyelik dilimlerde 632, 637, 638, 644,
635, 632, 616, 689, 742. Birikim yok.

### Not: olcum araci da test edilmeli

Bu bolumde iki ayri hata test aracindaydi, cihazda degil: kumulatif
gurultu ve `frames` sayacinin iki kez artirilmasi (PC 14562 kare
sayarken cihaz 7282 islemisti, tam iki kati). Ikincisi cihaz sayaciyla
karsilastirilinca yakalandi.

**Ders: PC tarafinin saydigi ile cihazin saydigi her kosuda
karsilastirilmali.** Tutmuyorsa once araci suphelen.

### Duzeltilmis kosu: 30 dakika

Kabul kosusu. Once `conformance` calistirildi, 15 testin 15'i gecti.

| | |
|---|---|
| Sure | 30.0 dk |
| Gonderilen kare | 44391 |
| Cihazin isledigi bolge | 133176 (= 44392 x 3) |
| Ortalama | 0.18 MB/s, **24.7 FPS** |
| Dusen | **0** |
| Baslik CRC hatasi | **0** |
| Payload CRC hatasi | **0** |
| Senkron kaybi | **0** |
| NACK / ACK zaman asimi | **0 / 0** |
| Heap | 352928 -> 352928 (**+0 bayt, %0.00**) |
| PSRAM | 8000863 -> 8000863 (**+0 bayt**) |

Kare hizi bastan sona duz. 30 saniyelik dilimler 20. dakikadan sonra
sirasiyla 740, 740, 742, 740, 741, 741, 741, 741, 741, 741, 741, 741,
741, 741, 740, 740, 741 kare. Ilk kosudaki dusus tamamen kayboldu,
yani o dusus gercekten test aracinin kumulatif gurultusundendi.

PC 44391 kare sayarken cihaz 133176 bolge isledi, yani 44392 kare
karsiligi. Bir kare fark el sikisma sirasinda gonderilen kareden
geliyor; 5 dakikalik kosuda da ayni birebir fark vardi (6602 karsiligi
6603). Sayaclar tutuyor.

**100 bin cerceve kriteri de bu kosuyla karsilandi.** Her bolge ayri bir
protokol cercevesi ve her birinin kendi baslik ve payload CRC'si var;
133176 cerceve, sifir CRC hatasi.

### Kablo cekip takma testi

Elle yapildi, otomatiklestirilemez. 5 dakikalik kosunun 36. saniyesinde
(736 kare gonderilmisti) kablo cekildi, birkac saniye sonra takildi.

**Onemli sart: USB kartin tek besleme kaynagi.** Ayri bir guc hatti yok,
yani kablo cekilince cihaz tamamen guc kaybediyor ve takilinca soguk
acilis yapiyor. Kriterin "reset gerekmiyor" kismi bu yuzden "elle BOOT
ya da RESET basmak, yeniden yukleme yapmak gerekmiyor" olarak okundu.

Cekildiginde:

| | |
|---|---|
| Ekran | Arka isikla birlikte tamamen sondu (guc kesildi) |
| Test araci | 5 sn yazma zaman asimi, sonra `SerialTimeoutException` |

Takildiginda:

| | |
|---|---|
| Port | COM5 kendiliginden geri geldi, ayni numara, ayni seri no |
| Elle mudahale | Yok. BOOT+RESET yok, yeniden yukleme yok |
| Kendini sinama | TAMAM (crc8, crc16, dort rle16 durumu) |
| Ekran | Kendiliginden geri geldi, oncesinde 0.5 sn boslik |
| El sikisma | Sorunsuz: protokol 1, 320x240, rx slot 3 |

**0.5 saniyelik boslik kasitli.** `setup()` once arka isigi
`BL_BRIGHTNESS_OFF` yapiyor, `tft.init()` ve tampon ayirmadan sonra
`drawSplash()` cizip arka isigi ancak o zaman aciyor. Panel ilklenirken
copu gostermemek icin. Kusur degil, tasarim.

Takma sonrasi 2 dakikalik dogrulama kosusu, cekmeden onceki 30 dakikalik
kosuyla yan yana:

| | Cekmeden once | Taktiktan sonra |
|---|---|---|
| Ortalama | 24.7 FPS | 24.6 FPS |
| Heap | 352928 (+0) | 352928 (+0) |
| Dusen / CRC / senkron | 0 / 0 / 0 | 0 / 0 / 0 |
| NACK / ACK zaman asimi | 0 / 0 | 0 / 0 |

Cihaz sayaci 0'dan basladi (8859 = 2953 x 3), soguk acilis beklendigi
gibi. Baglanti kopma oncesiyle ayni hizda devam ediyor.

### Bu testte cikan is: arac kopmayi karsilamiyor

`link_test.py` kablo kopunca traceback ile oluyor. Faz 1 kriteri cihaz
hakkinda oldugu icin bunu engellemiyor, ama Faz 2'nin cikis
kriterlerinden biri zaten "cihaz cikarilinca uygulama cokmuyor". Duzgun
kopma karsilama masaustu uygulamasinda yapilacak, olcum aracinda degil.

---

## Faz 1.5: GIF oynatma

Tarih: 2026-09-19
Kaynak: `gif/getsuga.gif`, 480x270, 55 kare, 15 FPS, 2.59 MB
Olcum araci: `src/gif_main.cpp`, `env:gifplay`. Cikti UART0 uzerine.

Butun kareler tam boy ve hepsinde saydamlik bayragi var: kodlayici
degismeyen pikselleri saydam isaretlemis. Saydam pikseller yazilmiyor,
onceki kare kaliyor.

### Sonuclar

Her satir 3 tur (165 kare) ortalamasi. "cozme" cizim tamamen kapaliyken.

| GIF | Sadece cozme | Cozme + cizme |
|---|---|---|
| 480x270 (orijinal) | 17.3 FPS | **14.8 FPS** |
| 320x180 (on olceklenmis) | 23.1 FPS | **19.0 FPS** |

Surekli oynatmada olculen: **14.95 - 15.18 FPS**, yani kaynagin kendi
hizi. GIF dogru hizda oynuyor.

### Yol boyunca yapilan uc iyilestirme

**1. Kare tamponu.** Ilk surum saydam olmayan her diziyi ayri ayri
basiyordu: kare basina **2680 setAddrWindow cagrisi**. Olculen 37.8 ms
cizim suresinin yaklasik 20 ms'i saf cagri yukuydu (dizi basina 7.4 us).
Kareyi tampona cizip tek seferde basmak bunu 26.7 ms'ye indirdi.

**2. On olcekleme.** Kaynak 480x270, ekranda gosterilen 320x180. Yani
129600 piksel cozulup 57600'u gosteriliyordu. PC tarafinda ffmpeg ile
320x180'e indirilince cozme 57.9 ms'den 43.4 ms'ye dustu.

Beklenenden az bir kazanc: piksel sayisi 2.25 kat azaldi ama sure sadece
1.33 kat. Cunku LZW cozme isi piksel sayisiyla degil **sikistirilmis veri
boyutuyla** orantili, o da 2.59 MB'dan 2.17 MB'a inmisti.

**3. Cozme ve basma ayri cekirdeklere.** Protokol tarafindaki ayni desen.
Ardisikken cozme 43 + basma 27 = 70 ms idi, yani kaynagin istedigi 67
ms'nin ustunde ve GIF yavas oynuyordu. Ayrilinca basma tamamen gizlendi,
geriye sadece degisen dikdortgeni kopyalamanin maliyeti (yaklasik 9 ms)
kaldi.

### Kutuphanenin bSync secenegi kullanilmiyor

`AnimatedGIF::playFrame(true, ...)` yalnizca **kendi icinde** gecen sureyi
olcup kare gecikmesinden dusuyor (AnimatedGIF.cpp:335). Basma isi
playFrame disinda yapildigi icin bu sure hesaba girmiyordu: 43.4 ms cozme
gorup 23.6 ms uyuyor, sonra 26.7 ms basiliyordu, toplam 93.7 ms. Olculen
93.2 ms ile birebir uyusuyor.

Zamanlama artik `loop()` icinde kendimiz yapiyoruz: hedef zamani biriktirip
gerekirse bekliyoruz, geride kalinca birikimi sifirliyoruz.

### Partisyon tablosu degistirildi

`default_16MB.csv` app0 ve app1 icin 6.4'er MB ayiriyor, dosya sistemine
3.5 MB birakiyordu. Uygulama 400 KB civarinda, yani 12.8 MB bos duruyordu
ve tek bir GIF dosya sisteminin ucte ikisini yiyordu.

`partitions.csv` ile app bolumleri 2'ser MB'a indirildi, dosya sistemi
**11.9 MB** oldu. OTA icin iki app bolumu korundu.

Bolumun adi `spiffs` kaldi ama icerik LittleFS: Arduino'nun LittleFS
kutuphanesi varsayilan olarak bu etiketi ariyor.

### Sonraki asamalar icin cikan kural

**GIF'ler cihaza yuklenmeden once ekran olcusune indirilmeli.** Kazanc
14.8 FPS yerine 19.0 FPS, yani yuzde 28 pay. Bu is PC uygulamasinin
gorevi ve zaten mimari karar 1 ile uyumlu: agir isi PC yapar.

Orijinal olcu de calisiyor (14.8 FPS, kaynagin yuzde 99'u), yani on
olcekleme sart degil ama pay birakiyor.

### Titreme: zamanlama hatasiydi

Oynatma hiz olarak dogruydu (15.0 FPS, kaynak da 15.0) ama ekranda titreme
goruldu. "Yetisiyor" ile "duzgun araliklarla gosteriyor" ayni sey degil.

Ekranin gercekten guncellendigi an, yani basma isinin kuyruga verildigi an
olculdu:

| | Once | Sonra |
|---|---|---|
| En kisa aralik | 48812 us | 59999 us |
| En uzun aralik | 92157 us | 70001 us |
| Oynama | **43345 us** | **2 us** |

Sebep: bekleme **gonderimden sonra** yapiliyordu. Cozme suresi kare basina
31-55 ms arasinda degisiyor; bekleme sonda oldugu icin dongu basi duzenli
kaliyordu ama ekranin guncellendigi an cozme suresi kadar kayiyordu.
Ortalama dogru, tek tek kareler neredeyse iki kat farkli araliklarla.

Duzeltme: cozme bitince kare tampona hazirlaniyor, **planlanan ana kadar
bekleniyor**, sonra gonderiliyor. Boylece cozmenin ne kadar surdugu
onemsiz hale geliyor. Kalan 10 ms fark GIF'in kendi karisik gecikmesi
(60 ve 70 ms), hata payi mikrosaniye seviyesinde.

Bu isin calismasi cozmenin hedef suresini asmamasina bagli: cozme 31-55 ms
arti kopyalama 9 ms, hedef 60-70 ms. Pay dar ama yetiyor. On olceklenmis
GIF'te pay daha genis, bu da on olceklemenin ikinci gerekcesi.

**Genel ders:** bir seyin ortalama hizinin dogru olmasi duzgun gorundugu
anlamina gelmiyor. Zamanlama, isin bittigi yerde degil **sonucun gorundugu
yerde** olculmeli.

---

## Faz 1.5b: Panel titremesi ve register ayari

Tarih: 2026-09-19

### Belirti

Ekranda titreme goruldu. Kullanicinin gozlemleri:

- **Sabit** pikseller titriyor, **hareketli** olanlar titremiyor
- Koyu ve siyaha yakin tonlarda en kotu, acik tonlarda yok
- Parlaklikla artiyor: 180 ustunde her zaman, 128'de bazi kliplerde
- Cizgi seklinde degil, tum ekranda
- "Ekran yenilenirken anlik parlaklik degisimi" gibi

### Teshis

Belirleyici gozlem: GIF acilirken bir kez siyahla doldurulup **bir daha hic
dokunulmayan** kenar bantlari da titriyordu. O pikselleri veri yolumuz
yeniden yazmiyor, yani sorun bizim tarafimizda olamaz.

Bu **polarite tersleme titremesi**. TFT panellerde piksel gerilimi her
karede polarite degistirir (sivi kristali DC'den korumak icin). VCOM
referansi tam ortalanmamissa pozitif ve negatif kare biraz farkli parlaklik
verir ve piksel iki seviye arasinda gidip gelir. Hareket bunu gizler, sabit
ve koyu pikseller ise acikca gosterir, cunku gama egrisi orada en dik.

Kaynagi: TFT_eSPI'nin ILI9341 init dizisindeki VCOM degerleri Adafruit'in
jenerik varsayilanlari, bu panele gore ayarlanmis degil.

### Yontem

`src/paneltune_main.cpp` ve `env:paneltune`: ekranda hic yeniden yazilmayan
sabit bir gri skala (7 bant, seviye 0-64), butonla canli register
degistirme. Desen hic tuslanmadigi icin gorulen her titreme kesin olarak
panelin.

Sonra ayni ayarlar GIF oynaticiya tasindi ve gercek icerikte dogrulandi.
**Bu adim gerekliydi:** sabit desende en iyi gorunen degerler (VCOM2 0x90,
VCOM1 2B 2B) hareketli icerikte ayni sonucu vermedi.

### Sonuc

| Register | Secilen | TFT_eSPI stok |
|---|---|---|
| VCOM2 (C7) | **0xB8** | 0x86 |
| VCOM1 (C5) | **30 30** | 3E 28 |
| Kare hizi (B1) | **112 Hz** | 100 Hz |
| Tersleme (B4) | 0x02 | 0x02 (degismedi) |
| Guc1 (C0) | 0x23 | 0x23 (degismedi) |

Parlaklik varsayilani 220. 255 titremeyi belirginlestiriyor, 200 sonuk
bulundu; 180-220 araligi gercek icerikte iyi calisiyor.

Degerler `src/panel_settings.cpp` icinde, `tft.init()` sonrasi uygulaniyor.
Stok degerler listede duruyor ki karsilastirma yapilabilsin.

### Kalan sinir

`rgb_test2.gif` klibinde parlaklik 180 ve ustunde titreme her ayarla
devam ediyor. Butun register kombinasyonlari denendi, degismedi. Diger
kliplerde ayni parlaklikta sorun yok.

Bu panelin sinirlarindan biri kabul edildi: TN panel, IPS degil. Kontrast
dusuk ve polarite terslemesi belirli icerik ile parlaklik birlesimlerinde
tamamen yok edilemiyor. PCB asamasinda IPS panele gecmek bu konuyu
kokten cozer.

### Not: parlaklik ve surme akimi

Arka isik GPIO 21'den dogrudan suruluyor ve ESP32-S3 pin basina 20-40 mA
verebiliyor. Panel bundan fazlasini isterse pin yetismez. Daha parlak
gerekirse cozum MOSFET ile 3V3'ten surmek; **gerilim yukseltmek degil**,
modulun akim sinirlama direnci 3.3V icin secilmis ve 5V LED'leri yakar.

---

## Faz 2 hazirligi: PC tarafi kare maliyeti ve dil secimi

Tarih: 2026-09-19
Makine: AMD Zen 3, 16 mantiksal cekirdek, Windows 10

Faz 2'nin cikis kriteri PC uygulamasindan bosta yuzde 1, calisirken yuzde 3
CPU istiyor. Bu olcum o butcenin nereye harcandigini bulmak ve dil secimini
tahmine degil sayiya baglamak icin yapildi.

### Neden olculdu: Faz 1 bu yuku hic test etmedi

`link_test.py` animasyon karelerini onceden kodluyor ve kosu sirasinda
sadece hazir baytlari yaziyor. Yani Faz 1'de olculen 24.7 FPS **sifir
kodlama maliyetiyle** alindi. Gercek uygulama her karede rasterleme, diff
ve RLE kodlama yapacak. O yuk hic olculmemisti.

### Yontem

Kare basina boru hatti: RGBA8888 -> RGB565 donusumu, 16x16 karo diff,
RLE16 kodlama. 320x240, 3000 kare, her dil 3 kez kosuldu.

Gecerlilik sarti: dort uygulamanin da ayni isi yaptigi kanitlandi.
Saglama toplami dordunde de 62619013 ve uretilen RLE ciktisi
`tools/protocol.py` icindeki `rle16_encode` ile bayt bayt ayni, 12702 bayt.
Eslesme olmasaydi olcum gecersiz sayilacakti.

Icerik: sistem paneli benzeri sentetik kare. 12.1x sikisiyor, olculen
gercek arayuz icerigi 21x sikisiyordu, yani test icerigi gercekten zor
tarafta. Sayilar iyimser degil.

### Sonuc: protokol asamalari

| Uygulama | En kotu (tam kare) | 24 FPS'te cekirdek payi | Tipik (kirli) | Butceye marj |
|---|---|---|---|---|
| C++ `-O2 -march=native` | 0.057 ms | %0.14 | %0.06 | 21x |
| Rust 1.98, release + LTO | 0.079 ms | %0.19 | %0.08 | 16x |
| C++ `-O2` tasinabilir | 0.086 ms | %0.21 | %0.10 | 14x |
| C# .NET 10 Release | 0.139 ms | %0.34 | %0.30 | 9x |
| Python saf dongu | 8.10 ms (sadece RLE) | %19.4 | - | **gecmiyor** |

Kosular arasi sapma binde birler seviyesinde.

C# olcumunde GC gen0/gen1/gen2 sayaci **0/0/0** ve tahsis edilen bellek
**0 bayt**. Tamponlar onceden ayrilinca sicak dongude GC hic devreye
girmiyor.

### Sonuc: rasterleme

Rust, `tiny-skia` sekiller icin, `fontdue` glif rasterlemesi icin
(onbellekli, 27 giris).

| Asama | ms/kare | Pay |
|---|---|---|
| Arka plan dolgusu | 0.005 | %1 |
| Baslik + metin | 0.004 | %1 |
| 4 satir metin + cubuk | 0.011 | %2 |
| Sparkline cizgi (stroke) | 0.372 | %58 |
| Sparkline alan (fill) | 0.244 | %38 |
| TOPLAM | 0.636 | |

Kenar yumusatma kapatilinca toplam 0.636'dan 0.267 ms'e iniyor, yani AA
tek basina 2.4 kat.

**Tam boru hatti** (rasterleme + donusum + diff + tam kare RLE, 24 FPS'te
her kare): 0.724 ms/kare, bir cekirdegin **yuzde 1.74'u**. Butcenin
icinde ama marj 1.7x, protokol asamalarinin tek basina verdigi 16x degil.

### Cikan dort sonuc

**1. CPU maliyeti dil secimini belirlemiyor.** En yavas ciddi aday olan
C# bile butcenin dokuzda birini kullaniyor. Derlenen ya da JIT'lenen
hicbir dil bu kriterde elenmiyor.

**2. Saf yorumlanan diller eleniyor.** Python tek basina RLE icin yuzde
19.4, ustelik donusum ve diff haric. Tasinabilir C++'tan 94 kat yavas.

**3. Butce rasterlemeye gidiyor, onun da yuzde 97'si tek bir seye:
kenar yumusatmali vektor yolu.** Metin, dikdortgen ve cubuk toplamda
yuzde 4, pratikte bedava. Bu maliyet dilden bagimsiz, ayni rasterleyiciyi
kullanan her dil ayni parayi oder.

**4. Hat, kodlayicidan 800 kat yavas.** Tam kare cikti 12742 bayt; bunu
0.18 MB/s'lik hatta basmak 70.8 ms suruyor, kodlamasi 0.086 ms. Darbogaz
sabit karar 5'te yazildigi yerde, USB'de. CPU tarafi gurultu seviyesinde.

### Tasarim kurallari (dil bagimsiz)

- Sayi, metin ve cubuk gosteren widget'lar bedava sayilir. Sistem paneli,
  saat, medya metni: toplam 0.02 ms.
- Pahali olan tek sey canli vektor grafigi. Sparkline yol yerine dikey
  sutunlarla cizilirse maliyet sifira yakin iner.
- Gerekmedikce kenar yumusatma acilmaz.

### Olculmeyen

Bellek ayak izi ve bosta davranis olculmedi. H maddesinin CPU tarafi
kapandi, bellek tarafi Faz 2 icinde olculecek.

---

## Faz 2.4 ve 2.5: sensorler, sistem paneli, dirty tracking olcumu

Tarih: 2026-09-19
Makine: AMD Zen 3, 16 mantiksal cekirdek, Windows 10, Radeon RX 5500 XT

### Bu makinede hangi sensorler var

| Olcum | Durum |
|---|---|
| CPU kullanimi | var, 16 cekirdek |
| RAM | var, 10.0 / 15.8 GiB |
| Disk | var, 909 / 2328 GiB |
| Ag rx/tx | var |
| CPU sicakligi | **yok** (Windows'ta sysinfo vermiyor, HWiNFO kaynagi henuz yazilmadi) |
| GPU yuku, sicaklik, VRAM | **yok** (NVIDIA yok, ADLX kaynagi henuz yazilmadi) |

Eksik olanlar bu fazda isimize yaradi: "eksik sensor kaynagi widget'i
bozmuyor" kriterini varsayimla degil gercek bir eksiklikle dogruladik.
Panelde CPU sicakligi ve GPU satirlari **hic cizilmiyor**, kalan satirlar
yukari kayiyor. Ekranda ne hata ne bos deger var.

### Dirty tracking: durgun ekranda trafik

Olcum kipi (`mscreen run --static`) ilk kareden iki saniye sonra
widget'lari dondurup rasterleme ve diff'i calistirmaya devam ediyor.
Yani ekran gercekten durgun ama boru hatti her turda isliyor.

35 saniye, yaklasik 1690 tur:

| | |
|---|---|
| Uretilen kirli dikdortgen | **0** |
| Gonderilen bayt | **0 bayt/sn** |
| Bos gecen tur orani | **%100** |

Kriter "sifira yakin" diyordu, olculen **tam sifir**.

### Canli ekranda trafik

Saat saniyede bir, sistem paneli yarim saniyede bir tazeleniyor.
60 saniyelik kosu:

| | |
|---|---|
| Gonderilen cerceve | 447 |
| Toplam | 150776 bayt |
| Ortalama | **2513 bayt/sn** |
| Bos gecen tur orani | **%96** |
| NACK / ACK zaman asimi | 0 / 0 |

Karsilastirma: dirty tracking olmasa her tur tam kare gonderilirdi.
Tam kare ciktisi yaklasik 12742 bayt; 24 FPS'te 306 KB/sn eder. Olculen
2.5 KB/sn, yani **yaklasik 122 kat azalma.**

### CPU ve bellek

`mscreen run`, 30 saniyelik ornekleme:

| Durum | Bir cekirdegin | Toplam CPU'nun | Bellek (RSS) |
|---|---|---|---|
| Calisirken (canli ekran) | **%0.62** | %0.039 | 25.8 MB |
| Bosta (durgun ekran) | **%0.42** | %0.026 | 25.6 MB |

Kriter bosta yuzde 1, calisirken yuzde 3 istiyordu. Bir cekirdek
uzerinden okunsa bile ikisi de saglaniyor.

### Bulunan hata: ACK'ler sessizce dusuyordu

20 saniyeden uzun her kosuda kosu basina tam bir ACK zaman asimi
gorunuyordu, kisa kosularda gorunmuyordu.

**Sebep:** porttan sadece gonderim penceresi dolunca okuyorduk. Dirty
tracking sayesinde turlarin yuzde 96'si bos geciyor ve o turlarda porta
hic bakilmiyordu. Cihazin CDC TX tamponu 4096 bayt; ACK ve LOG
cerceveleri orada birikip tasiyordu.

**Cozum:** `Link::poll` her turda cagriliyor, gonderilecek bir sey olmasa
bile. 60 saniyelik kosuda ACK zaman asimi 0.

Ilk duzeltme yeni bir sorun yaratti: okuma bloklayiciydi ve her tur 200 ms
yiyordu, tur hizi saniyede 48'den 6'ya dustu. Okuma `bytes_to_read` ile
bloklamayan hale getirildi, tur hizi geri geldi.

**Ders:** dirty tracking gonderimi seyrektiyorsa okuma da seyreklesmemeli.
Iki yon ayri dusunulmeli.

---

## Faz 2: gosterge paneli ve Afterburner kaynagi

Tarih: 2026-09-19

### Afterburner paylasimli bellegi

Plan GPU icin satici basina SDK diyordu (NVIDIA'da NVML, AMD'de ADLX).
MSI Afterburner'in paylasimli bellegi satici bagimsiz ve ayni yapidan
hem NVIDIA hem AMD okunuyor. Ustelik `sysinfo`'nun Windows'ta
veremedigi CPU sicakligini da veriyor.

Bu makinede okunanlar:

| Olcum | Once | Simdi |
|---|---|---|
| CPU sicakligi | yok | 66 C |
| GPU yuku | yok | okunuyor |
| GPU sicakligi | yok | 54 C |
| VRAM | yok | 1.4 / 8.0 GiB |

Paylasimli bellekte 85 girdi var. Kullanilanlar: `GPU temperature`,
`GPU usage`, `Memory usage` (VRAM, megabayt; toplam icin `maxLimit`),
`CPU temperature`.

**Iki tuzak, ikisi de koda not dusuldu:**

1. Imza sabiti. SDK 'MAHM' degerini MSVC coklu karakter sabiti olarak
   tanimliyor, yani `0x4D41484D`. Bayt sirasi cevrilirse `0x4D48414D`
   cikiyor ve imza hic tutmuyor. Ilk denemede oyle yanildik; ham baslik
   dokumu alinca `"MHAM"` gorundu ve hata anlasildi.
2. Ad eslestirmesi. Icerme ile arayinca `GPU temperature 2` girdisi
   `GPU temperature` girdisinin ustune yaziyordu; bu makinede ikinci bir
   GPU sicaklik sensoru var. Tam esitlige cevrildi. Ayni tuzak
   `CPU temperature` icin de gecerli, cunku `CPU1 temperature` gibi
   cekirdek basina kardesleri var.

**Not:** Win32 hata kodunu okumak teshisi hizlandirdi. Esleme aciliyor
ama girdi gelmiyorsa sorun yetki degil ayristirmadir; ham basligi
basmak bunu hemen gosterdi.

### Halka gostergeler: kenar yumusatma maliyeti

Onceki olcum kenar yumusatmali vektor yolunun rasterlemenin yuzde
97'sini yedigini gostermisti. Halka gosterge tam olarak o: yay cizimi.
Yine de karsilaniyor, cunku **24 FPS varsayimi bu widget icin gecerli
degil.** Gosterge paneli saniyede iki kez yeniden ciziliyor.

Uc halka, her biri iki yay (oyuk ve dolu), yaklasik uc pikselde bir
dugum:

| Tasarim | CPU (bir cekirdegin) | Bellek |
|---|---|---|
| Satirli panel (yay yok) | %0.62 | 25.8 MB |
| Halkali panel (6 yay) | **%0.49** | 45.4 MB |

Halkali tasarim daha az CPU harcıyor, cunku degerleri daha seyrek
degisiyor ve yeniden cizim daha nadir tetikleniyor. Kriter yuzde 3
istiyordu, ikisi de rahat geciyor.

**Bellek 25.8'den 45.4 MB'a cikti.** Sebep ikinci font (oranti fontu)
ve glif onbellegi. Kabul edildi ama 24 saatlik kriterde izlenecek;
artis surekli degil, onbellek doyunca duruyor.

### Cikan kural

Rasterleme maliyetini widget'in yeniden cizim sikligiyla birlikte
dusun. "Kenar yumusatma pahali" tek basina bir yasak degil; saniyede
iki kez cizilen bir seyde bedava sayilir, her karede cizilen bir seyde
butceyi yer.
