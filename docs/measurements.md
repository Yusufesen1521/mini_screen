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
