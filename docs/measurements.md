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

> **Guncelleme (bekleme ekrani eklendikten sonra):** bu sayi artik
> 0 degil. Cihaz belirli sure mesaj gelmezse bekleme ekranina dustugu
> icin PC bostayken 1500 ms'de bir PING gonderiyor. Ayni olcum tekrar
> yapildi: **7-10 bayt/sn, ortalama 8**, uretilen dikdortgen yine 0.
> Kriter hala rahat geciyor (hat kapasitesinin yaklasik yuzde 0.004'u)
> ve karsiliginda canlilik tespiti kazanildi.

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

---

## Faz 2: 106 dakikalik kesintisiz kosu

Tarih: 2026-09-19

Cikis kriteri 24 saat istiyor. Kullanici o kadar uzun beklemek
istemedi, ara dogrulama olarak 3 saat planlandi ve 106 dakikada
yeterli veri toplanip durduruldu. **24 saatlik kosu hala acik bir is.**

Yerlesim: saat basligi ve halka gosterge paneli. Sensor kaynaklari
`system` ve `afterburner`.

### Sonuc

| | |
|---|---|
| Sure | 106.2 dk (6371 sn), 1271 olcum penceresi |
| Toplam tur | 299 789 |
| Bos gecen tur | 287 304 (**%95.8**) |
| Gonderilen bolge | 49 875 |
| Trafik | ortalama **2706 bayt/sn**, toplam 17.2 MB |
| NACK | **0** |
| ACK zaman asimi | **4** |
| Cihaz: dusen / hdrCRC / payloadCRC / senkron | **0 / 0 / 0 / 0** |
| Bellek (RSS) | 45.4 -> **45.5 MB** |
| CPU ortalama | **%0.54** (bir cekirdegin) |
| Tutamak / is parcacigi | 303 / 4 |

### Suruklenme yok

Ilk ceyrek ile son ceyrek karsilastirmasi:

| | Ilk ceyrek | Son ceyrek |
|---|---|---|
| Bos tur orani | %95.8 | %95.8 |
| Tur / pencere | 235.1 | 234.9 |
| Trafik | 2956 bayt/sn | 2650 bayt/sn |

Bellek 106 dakikada 0.1 MB artti, yani olcum gurultusu icinde.
Glif onbelleginin doyup durdugu dogrulandi. Tutamak ve is parcacigi
sayisi sabit.

### Acik kalan: duzenli araliklarla bir ACK dusuyor

Dort ACK zaman asimi olustu ve zamanlari carpici sekilde duzenli:

| Olustugu an | Aradaki sure |
|---|---|
| 22.7 dk | - |
| 48.0 dk | 25.3 dk |
| 74.4 dk | 26.4 dk |
| 103.8 dk | 29.4 dk |

**Trafikle ilgisi yok.** Olustuklari pencereler ortalama yukluydu
(1676-2460 bayt/sn); en yuksek trafikli pencerelerde (5942 bayt/sn) hic
olusmadi.

Bu duzenlilik rastgele kayip olmadigini soyluyor, sistematik bir sey
var. Onceki ACK kaybi sorunu (porttan sadece gonderirken okumak)
duzeltildi ve o duzeltme calisiyor; bu kalan olay farkli bir sey.
Henuz sebebi bulunmadi.

Siddeti dusuk: NACK yok, cihaz sayaclari tertemiz, ekranda gorunur bir
etki yok. Ama 24 saatlik kriterde yaklasik 55 kez olusur, o yuzden
kriter kapatilmadan once sebebi bulunmali.

Arastirilacak yonler: Windows USB secici askiya alma (selective
suspend), cihaz tarafinda periyodik bir is, ya da yaklasik 26 dakikada
bir dolan bir tampon durumu.

### Not

Cihaz sayaci 58 022 gosteriyor ama bu kosuda 49 875 bolge gonderildi.
Fark, cihazin bu kosu icin yeniden baslamamis olmasindan geliyor:
kosunun log'unda acilis banner'i yok, yani sayac bugunun butun
oturumlarinin toplami. Faz 1'de ogrenilen "PC ile cihazin saydigini
karsilastir" kurali geregi kovalandi ve acikligi giderildi.

---

## Faz 2: bekleme ekrani ve baglanti kopma testleri

Tarih: 2026-09-19

### Tasarim catismasi: dirty tracking ile canlilik tespiti

Cihaz "kare gelmiyorsa PC gitti" diyemiyor. Dirty tracking sayesinde
ekran durgunken PC dakikalarca hicbir sey gondermiyor ve bu normal
calisma. Bu yuzden:

- Cihaz sayaci **herhangi bir gecerli mesajla** sifirlaniyor.
- PC bostayken **1500 ms'de bir PING** gonderiyor.
- Cihazin zaman asimi **4000 ms**, yani yaklasik 2.7 kat pay var.

Bedeli olculdu: durgun ekranda trafik 0'dan 8 bayt/sn'ye cikti.

### Olcerek bulunan hata: `!Serial` guvenilmez

Ilk surumde ikinci bir olcut daha vardi: CDC baglilik durumu
(`!Serial`, HWCDC `operator bool`). Mantik "port kapandiysa PC gitti"
idi ve aninda tepki verecekti.

Kullanici ekranda kirpma bildirdi: arka isik kisilmiyor, "arada bir
kirpiyor", panel hic gorunmuyordu.

Tahmin etmek yerine cihaza hangi kosulun tetikledigini kaydettirdim:

```
standby sebep: cdc=1 idle=0 (43 ms)
```

**Son mesajin uzerinden 43 ms gecmisken port kapali sanildi.** Yani
`!Serial`, aktif trafik sirasinda bile "bagli degil" donuyor. Cihaz
saniyeler icinde beklemeye girip cikiyor, `backlightSet` 220 ile 70
arasinda gidip geliyor (kullanicinin gordugu kirpma) ve `drawStandby`
paneli surekli siliyordu.

Olcum: 30 saniyede 2 kez sahte gecis. Kaldirildiktan sonra 40 saniyede
sifir.

**Kural:** ESP32-S3 yerlesik USB CDC'sinde `!Serial` bir baglanti
kopma isareti olarak kullanilmaz. Tek guvenilir olcut sessizlik suresi.

### Ikinci hata: PONG cerceveleri birikiyordu

PING eklenince cihaz her birine PONG donuyor, ama PC tarafinda PONG
hicbir yerde tuketilmiyordu ve `inbox` sinirsiz buyuyordu.

| Durum | ACK zaman asimi orani |
|---|---|
| PING oncesi (106 dk kosu) | 4 / 106 dk |
| PING var, PONG tuketilmiyor | **2 / 40 sn** |
| PONG tuketiliyor, inbox sinirli | 1 / 180 sn |

PONG artik tuketiliyor ve `inbox` 64 cerceve ile sinirli.

### Baglanti kopma testleri

**20 yazilim cevrimi** (ac, el sikis, kapat; kabloya dokunmadan):

| | |
|---|---|
| Basarili | **20 / 20** |
| Sure | ortalama 681 ms, en dusuk 39, en yuksek 2173 |
| Cihaz sayaclari | dusen 0, CRC 0, senkron 0 |

En uzun deneme 2173 ms; orada `Link::open_retry` devreye girdi, yani
portun yeniden numaralandirma penceresine denk geldi ve mekanizma
isini yapti.

**5 fiziksel cevrim** (kablo cekilip takildi): bes seferin besinde de
cihaz kendine geldi ve acilis ekranini cizdi. Elle mudahale
gerekmedi. Kablo cekilince guc de kesildiginden cihaz soguk acilis
yapiyor; bekleme ekrani bu durum icin degil, **cihaz gucluyken PC
uygulamasinin kapanmasi** icin.

Kriter 20 fiziksel cevrim istiyordu. Kullanici konnektor asinmasi
endisesiyle sayiyi dusurmek istedi. Not: USB konnektorleri binlerce
cevrim icin derecelendirilir, 20 cevrim mekanik olarak onemsiz. Yine
de karar kullanicinin; kapsam 5 fiziksel artı 20 yazilim cevrimi
olarak daraltildi ve yazilim tarafi asil riskli olan yeniden
numaralandirma yolunu zaten kapsiyor.

### Gorsel onay alindi

Tarih: 2026-09-20. Kullanici ekrana bakarken uc asamali dizi kosuldu,
dordu de onaylandi:

| Soru | Sonuc |
|---|---|
| Canli kosuda panel duzgun mu, kirpma bitti mi | **Temiz, kirpma yok** |
| Kapaninca bekleme ekrani geliyor mu | **Geliyor** |
| Arka isik kisiliyor mu | **Kisiliyor** |
| Tekrar acilinca kalinti kaliyor mu | **Tam geri geliyor, kalinti yok** |

Makine tarafi da ayni yonu gosterdi. 60 saniyelik kosuda 475 cerceve,
NACK 0, ACK zaman asimi 0; cihaz log'unda **tam bir tane** gecis var:

```
baglanti yok, 4001 ms sessizlik
```

Eski hatada gecis kosu icinde tekrar tekrar olusuyordu (30 saniyede 2
sahte gecis). Simdi kosu boyunca sifir, sadece kapanista bir tane.
`!Serial` duzeltmesi hem olcumle hem gozle dogrulandi.

**Bu kriter kapandi.**

---

## Faz 2: ACK zaman asimi tanilamasi

Tarih: 2026-09-20

Seyrek ACK zaman asiminin sebebi bulunamamisti ve 24 saatlik kriterin
onunde duruyor. Tahmin listesini uzatmak yerine baglantiya olay aninda
dogru olani kaydettirdik.

### Eklenen olcum

`Link` artik her bekleyisi olcuyor ve zaman asiminda cevreyi yaziyor:

| Alan | Ne soyluyor |
|---|---|
| `max_wait` | Basarili bekleyislerin en uzunu, pencere basina |
| `waiting_bytes` | Vazgecerken surucude bekleyen bayt. Sifirdan buyukse veri gelmisti ve biz isleyemedik |
| `bytes_during` | Iki saniyelik bekleyiste okunan bayt. Sifirsa hat tamamen sustu |
| `late_acks` | Vazgectikten sonra yine de gelen onay. Varsa olay "kayip" degil "gecikme" |
| `unexpected` | Beklenmeyen cerceve tipi sayisi |

Vazgecilen sira numaralari 4 saniye hatirlaniyor. Sira numarasi u8,
yani 256 cercevede tekrar ediyor; daha uzun tutulursa ayni numarayi
tasiyan yeni bir cercevenin onayi "gec gelen onay" sanilir.

Ayrica beklenmeyen cerceve tipleri artik sessizce inbox'ta birikmiyor,
sayilip dusuruluyor. Onceki ACK hatasi tam olarak boyle gizlenmisti.

### Ilk bulgu: gecikme kuyrugu yok, kopus var

Uc kisa kosuda (22, 60 ve 25 saniye, toplam 853 cerceve) olculen en
uzun **basarili** onay bekleyisi:

| Kosu | enuzunACK |
|---|---|
| 22 sn | 0 - 3 ms |
| 60 sn | 1 - 3 ms |
| 25 sn | 0 - 2 ms |

Normal onay suresi **0-3 ms**, zaman asimi siniri ise 2000 ms. Yani
sinirin yaklasik 600 kati pay var.

**Bu, olayin ne olmadigini soyluyor.** Yavas yavas buyuyen bir
gecikme kuyrugu olsaydi sinira yaklasan bekleyisler cok daha sik
gorunurdu; 850 cercevede en yuksek deger 3 ms. Yani "tampon giderek
doluyor" ve "yuk artinca gecikiyor" aciklamalari zayifladi. Kalan
resim: hat bir anda saniyeler boyunca tamamen duruyor.

Suphe listesi buna gore daraldi:

1. Windows USB secici askiya alma (selective suspend)
2. Cihaz tarafinda uzun suren periyodik bir is

`bytes_during` alani ikisini ayirt edecek: sifir gelirse hat sustu,
sifirdan buyuk gelirse cihaz calisiyordu ama o onayi gondermedi.
Olay nadir oldugu icin bunu ancak uzun bir kosu gosterir; kosu henuz
yapilmadi.

---

## Faz 2: CPU sicakligi kaynaklar arasinda siliniyordu

Tarih: 2026-09-20

MSI Afterburner bu oturumda acikti ve ilk kez `afterburner` kaynagi
calisir durumda yakalandi. Ama panel GPU sicakligini gosterirken CPU
sicakligini gostermiyordu.

`sensors --dump` ciktisi degerin **var** oldugunu gosterdi:

```
CPU temperature      63.625     ust sinir 100
```

Sebep esleme degil, uzerine yazma. `Snapshot` kaynaklar arasinda ortak
ve birikimli. Kaynaklar `afterburner`, `system` sirasiyla kosuyor ve
`system` kaynagi soyle yaziyordu:

```rust
out.cpu_temp_c = self.cpu_temperature();
```

`sysinfo` Windows'ta CPU sicakligi vermiyor, yani bu satir her turda
Afterburner'in okudugu degeri `None` ile eziyordu. Kural artik acik:
**bir kaynagin okuyamadigi deger, baska bir kaynagin okudugunu
silmez.**

Duzeltme sonrasi ayni makinede:

| | Once | Sonra |
|---|---|---|
| CPU sicaklik | yok | **64.1 C** |
| GPU sicaklik | 55.0 C | 55.0 C |

Panelde CPU gostergesinin altinda sicaklik satiri ilk kez ciziliyor.
Regresyon testi eklendi: `baska_kaynagin_sicakligi_silinmiyor`.

**Not:** "eksik sensor kaynagi widget'i bozmuyor" kriteri bu makinede
kaynaklar gercekten yokken dogrulanmisti, o kanit gecerli kalir. Simdi
tersi de gorulmus oldu: kaynak gelince satirlar kendiliginden ciziliyor.

---

## Faz 2: `hwmon` paneli, cihazda ilk kosu

Tarih 2026-09-23. lopaka.app uzerinde 480x320 icin cizilen donanim
izleme tasarimi 320x240'a yeniden yerlestirildi ve varsayilan yerlesim
yapildi. Bu, cihazda kosturulan ilk olcumu.

Komut: `mscreen run --seconds 90`, panel `hwmon`, Afterburner acik.

| Olcum | Deger |
|---|---|
| Sure | 90 sn |
| Gonderilen cerceve | 482 |
| Toplam trafik | 187 604 bayt, yani 2.1 KB/sn |
| Bos tur orani | yuzde 97-98 |
| NACK | 0 |
| ACK zaman asimi | 0 |
| Gec gelen onay | 0 |
| En uzun ACK bekleyisi | 3 ms |

Okunan sey: **dirty tracking bu panelde de calisiyor.** Turlarin yuzde
98'i hicbir sey gondermeden geciyor; giden sey degisen sicaklik ve
yuzde rakamlarinin dikdortgenleri, tur basina 17-42 arasi.

2.1 KB/sn, olculen baglanti tavani olan 0.20 MB/s'in binde biri. Yani
iki kenar yumusatmali halkanin maliyeti tasima tarafinda gorunmuyor
bile; halkanin maliyeti PC tarafindaki rasterlemede ve o da saniyede
iki kez oluyor.

90 saniye, nadir ACK olayini yakalamak icin kisa. Bu kosu panelin
calistigini gosteriyor, o acik konuyu kapatmiyor.

### Yan kanit: widget kaydi hala calisiyor

`hwmon` eklendikten sonra calisma aninda basilan liste:
`["clock", "gauges", "hwmon", "sysinfo", "uptime"]`. Widget kendini
kaydetti, cekirdekte widget adi gecen tek satir degismedi. Faz 2'nin
widget soyutlamasi kriteri ucuncu gercek widget ile de tuttu.

---

## Faz 2: MISO baglandi, panel geri okumasi

Tarih 2026-09-23. Gerekcesi bir onceki oturumda yasanan beyaz ekran:
panel init'ini kaybetmisti, butun protokol sayaclari yesil kaliyordu ve
tek cozum karti resetlemekti. Firmware'in soracak bir yolu yoktu.

### Baglanti

Panelin SDO(MISO) ucu **GPIO 13**'e baglandi. Bu pin ESP32-S3'te SPI2
(FSPI) MISO'sunun IOMUX karsiligi, yani 10/11/12 ile ayni yoldan gidiyor,
GPIO matrisi uzerinden degil. Dokunmatik olcumunden sonra bos kalmisti.

`spi_read_frequency` 20 MHz'den **5 MHz'e** indirildi. ILI9341 veri sayfasi
RDX cevrimi icin en az 150 ns istiyor, yani okuma tavani 6.6 MHz. 20 MHz
ile birakilsa register okumasi sessizce cop dondururdu. Yazma hizi
40 MHz'de kaldi, ikisi ayri sabit.

### Init sonrasi ilk okuma

| Register | Okunan | Anlami |
|---|---|---|
| RDDPM (0x0A) | 0x9C | uyku disi 1, normal kip 1, ekran acik 1, booster 1 |
| RDDMADCTL (0x0B) | 0xE8 | MY, MX, MV set ve BGR set, yani `DISPLAY_ROTATION` 3 |
| RDDCOLMOD (0x0C) | 0x05 | 16 bit piksel, bizim bastigimizla ayni |
| RDDSDR (0x0F) | 0xC0 | denetleyicinin kendi tanisi: register yuklemesi ve islevsellik TAMAM |
| RDDID (0x04) | 00 00 00 00 | **gelmedi** |
| RDID4 (0xD3) | 00 FF 00 FF | **gelmedi** |

Piksel gidis donus testi: bes farkli renk (0xF800, 0x07E0, 0x001F,
0xFFFF, 0x0000) yazildi ve birebir geri okundu, **5 / 5**.

### Kimlik registerleri neden gelmiyor

RDDID ve RDID4 anlamsiz donuyor ama bu bir hat sorunu degil. Hat kopuk
olsa butun registerler 0x00 ya da 0xFF gelirdi; oysa bes ayri register
birbirinden farkli ve hepsi beklenen degerde, ustelik piksel gidis
donusu tam. TFT_eSPI'nin `readcommand8` fonksiyonu ILI9341'in 0xD9 indeks
registeri uzerinden okuyor ve bu yontem cok baytli kimlik komutlarinda
guvenilir calismiyor.

**Cikan kural: saglik kontrolu kimlik registerlerine dayandirilmaz.**
Karar RDDPM ve RDDCOLMOD uzerinden veriliyor, ikisi de olculdu.

### 120 saniyelik kosu, canli basma sirasinda

Periyodik kontrol `PANEL_CHECK_INTERVAL_MS` 5000 ms. Okuma sadece butun
cozme tamponlari serbestken yapiliyor: basma gorevi 0. cekirdekte tft'yi
kullaniyor, kontrol 1. cekirdekte ve TFT_eSPI iplik guvenli degil.

| Olcum | Deger |
|---|---|
| Sure | 120 sn |
| Gonderilen cerceve | 574 |
| Panel saglik okumasi | 27 |
| Farkli sonuc sayisi | **1** |
| Okunan | `PM 0x9C COLMOD 0x05 SDR 0xC0`, 27 / 27 |
| NACK, ACK zaman asimi | 0, 0 |

Yirmi yedi okumanin yirmi yedisi birebir ayni. Yani geri okuma canli
basma trafigi altinda da kararli ve yanlis alarm uretmiyor.

### Kalan is

Bu surum sadece **raporluyor**, otomatik yeniden init yok. Once okumanin
kararli oldugunu olcmek gerekiyordu, olculdu. Otomatik kurtarma ayri bir
adim.

---

## Faz 2: panel kurtarma, ariza uretilerek dogrulandi

Tarih 2026-09-24. Onceki bolumde geri okuma eklenmisti ama sadece
raporluyordu. Bu adimda iki is yapildi: Faz 1.5b register ayarlari
protokol firmware'ine de girdi, ve `panelHealthy()` false donunce
otomatik kurtarma eklendi.

### Faz 1.5b ayarlari eksikti

`main.cpp` `panelApplyAll()` cagirmiyordu. Yani protokol firmware'i,
gozle bulunan VCOM ve kare hizi degerleri olmadan, TFT_eSPI'nin stok
ILI9341 init dizisiyle kosuyordu. `gifplay` ve `paneltune` cagiriyordu.
Eklendi, `tft.init()` sonrasina.

Yan dogrulama: ayarlar uygulandiktan sonra saglik esikleri degismedi,
RDDPM yine 0x9C ve RDDCOLMOD yine 0x05 okunuyor.

### Ariza nasil uretildi

Tahmin etmemek icin gercek bir ariza uretildi. Panele **SWRESET (0x01)**
gonderildi, yani denetleyici kendi acilis haline donduruldu: uyku modu,
ekran kapali, varsayilan ayarlar. Beyaz ekran olayinda olanin aynisi.

Derleme bayragi `PANEL_FAULT_TEST_MS`, varsayilan olarak derlenmiyor:

```bash
PLATFORMIO_BUILD_FLAGS=-DPANEL_FAULT_TEST_MS=25000 pio run -t upload
```

### Sonuc

Acilistan 25 saniye sonra, PC bagli ve canli cizim yaparken:

```
TEST: panele SWRESET gonderiliyor, ariza uretiliyor
panel BOZUK  PM 0x08 COLMOD 0x05 SDR 0x00
panel kurtarma denemesi 1/3
panel yeniden init: TAMAM  PM 0x9C COLMOD 0x05 SDR 0xC0
PC'den tam kare istendi
panel yeniden init edildi, tam kare gonderiliyor
```

Son satir PC tarafindan, yani `MSG_NEED_FULL` karsiya ulasti ve
`Engine::force_full()` calisti.

**Onemli ayrinti: COLMOD ariza sirasinda da 0x05 okundu.** Yani tek
basina piksel formatina bakan bir kontrol bu arizayi kacirirdi. Yakalayan
RDDPM oldu: 0x9C'den 0x08'e dustu, yani uyku disi ve ekran acik bitleri
sifirlandi. RDDSDR de 0xC0'dan 0x00'a dustu.

**Cikan kural: saglik karari RDDPM olmadan verilemez.**

### Kurtarma sonrasi dogrulama kosusu

Ariza enjeksiyonu olmayan normal firmware ile:

| Olcum | Deger |
|---|---|
| Sure | 120 sn |
| Gonderilen cerceve | 388 |
| NACK | 0 |
| ACK zaman asimi | 0 |
| Panel saglik uyarisi | 0 |

### Acik nokta

Ariza enjeksiyonlu kosuda iki ACK zaman asimi goruldu, dogrulama
kosusunda sifir. Ikisi de kosunun sonundaki bosaltma aninda damgalandi.
Kurtarmaya baglanmadi, cunku olay ariza aninda degil 38 saniye sonra
oldu. Yine de akilda tutulmali: kurtarma sirasinda `tft.init()`,
`panelApplyAll()` ve tam ekran silme `loop()` icinde calisiyor ve o sure
boyunca porttan okuma durmus oluyor.
