# mini_screen

ESP32-S3 uzerinde ILI9341 TFT ekranin dogru calistigini kanitlayan minimal
PlatformIO projesi. Bu asamada tek is ekrani dogrulamak: renk sirasi, offset ve
bolgesel guncelleme.

## Donanim

- ESP32-S3 DevKitC-1, N16R8 (16 MB flash, 8 MB oktal PSRAM)
- Lockerbox 3.2" TFT SPI, ILI9341, 240x320, v1.0
- Breadboard ve erkek-erkek jumper kablolar

Panelde dokunmatik yok. Modulun uzerindeki T_IRQ, T_DO, T_DIN, T_CS, T_CLK
pinleri bos birakilir, kodda da kullanilmaz.

## Baglanti

| Ekran pini | ESP32-S3 GPIO |
|---|---|
| VCC        | 3V3 |
| GND        | GND |
| CS         | 10  |
| RESET      | 9   |
| DC         | 14  |
| SDI (MOSI) | 11  |
| SCK        | 12  |
| LED        | 21  |

MISO baglanmaz. `TFT_MISO` bilerek tanimlanmaz, ekrandan geri okuma yapilmaz.

Kullanilmayan pinler: GPIO 26-37 dahili flash ve oktal PSRAM tarafindan
kullanilir. GPIO 0, 3, 19, 20, 45, 46 strapping veya USB gorevlidir.

## Derleme ve yukleme

```bash
pio run -t upload -t monitor
```

Sadece derlemek icin:

```bash
pio run
```

Seri port hizi 115200.

Kartta iki USB portu var. `ARDUINO_USB_CDC_ON_BOOT=1` oldugu icin Arduino
tarafindaki `Serial` nesnesi yerlesik USB portuna (GPIO 19/20) baglanir, UART
kopru portuna degil. Bu yuzden log ayni anda `Serial0` (UART0, TX/RX pinleri)
uzerine de basiliyor. Hangi porta takili olursan ol cikti gorunur.

## Yapilandirma notlari

TFT_eSPI ayarlarinin tamami `platformio.ini` icindeki `build_flags` altinda.
Kutuphanenin `User_Setup.h` dosyasi degistirilmez; o dosya `.pio/libdeps`
altinda durur ve kutuphane guncellemesinde silinir. `-DUSER_SETUP_LOADED=1`
verildigi icin kutuphane kendi varsayilan ayar dosyalarini okumaz.

### USE_FSPI_PORT neden zorunlu

TFT_eSPI 2.5.43, ESP32-S3 uzerinde `USE_FSPI_PORT` ya da `USE_HSPI_PORT`
tanimlanmamissa `SPI_PORT` makrosunu `FSPI` yapiyor. Arduino core 2.x icinde
`FSPI` degeri 0, `REG_SPI_BASE(0)` ise 0 donduruyor. Sonucta kutuphane SPI
register adresi olarak 0x10 kullaniyor ve `tft.init()` icinde
`Guru Meditation Error: StoreProhibited` ile cokuyor. Kart sonsuz reset
dongusune giriyor, ekranda hicbir sey gorunmuyor.

`-DUSE_FSPI_PORT` ile `SPI_PORT` 2 olur, yani SPI2. Bu ayni zamanda GPIO
10/11/12 pinlerinin IOMUX karsiligi, dolayisiyla en hizli yol. Alternatif
`-DUSE_HSPI_PORT` SPI3 kullanir, o da calisir ama sinyaller GPIO matrisi
uzerinden gider.

`include/pins.h` icinde bu ikisinden biri tanimli degilse derleme `#error` ile
durur, ayar kazara silinirse tekrar ayni cokmeyi yasamamak icin.

Pin ve yerlesim sabitleri `include/pins.h` icinde. Ekranin veri pinleri
TFT_eSPI tarafindan derleme aninda goruldugu icin sayisal degerleri
`platformio.ini` icinde tanimli, `pins.h` bunlari okunur isimlerle yeniden
yayinlar ve eksik olmalari durumunda derlemeyi `#error` ile durdurur.

### SPI hizi

SPI hizi tek yerden degistirilir, `platformio.ini` icindeki `[display]`
bolumunde:

```ini
[display]
spi_frequency = 40000000
```

Breadboard ve jumper kablolarda 40 MHz kararsiz olabilir. Ekranda bozulma,
karlanma, kayan satirlar veya rastgele pikseller gorursen sirayla `27000000`
ve `20000000` dene. Kablolari kisaltmak da ayni sorunu cozer.

## Ekranda ne gorunmeli

Acilista once arka isik iki kez kisa kisa yanip soner. Bu, ekran hic goruntu
vermese bile firmware'in calistigini ve arka isik hattinin saglam oldugunu
gosteren bir isaret. Sonra:

1. Ustte "MINI SCREEN" basligi ve altinda ince gri ayirici cizgi
2. Dortlu renk bloklari, yukaridan asagiya: kirmizi, yesil, mavi, beyaz.
   Her blogun uzerinde kendi adi yazar.
3. Ekranin dort kosesinde birer sari piksel
4. Bloklarin altinda gri "COUNTER" etiketi, altinda saniyede bir artan sayi
5. En altta bos siyah alan

Seri portta her saniye `sayac=<n>  cizim=<n> us` satiri akar.

## Beklenen goruntu cikmazsa

Once acilistaki arka isik darbelerine bak, teshis bunun uzerine kurulu.

**Darbeler hic gorunmuyor** (ekran tamamen olu):

1. Seri portu dinle: `pio device monitor`. Acilis bilgisi geliyor mu?
   Gelmiyorsa `pio device list` ile portu kontrol et.
2. Seride `Guru Meditation Error` gorursen firmware cokuyor demektir, sorun
   kabloda degil. Backtrace adreslerini
   `xtensa-esp32s3-elf-addr2line -pfiaC -e .pio/build/esp32-s3-devkitc-1/firmware.elf <adres>`
   ile coz.
3. Seri temizse arka isik hattina bak: LED pini GPIO 21, VCC 3V3, GND.
   Breadboard uzerinde temassizlik en sik sebep, jumperlari tek tek oynatarak
   dene.

**Darbeler gorunuyor ama ekran siyah kaliyor**: besleme ve arka isik saglam,
sorun veri hattinda.

4. CS (10), DC (14), RESET (9) baglantilarini kontrol et. Bu uc pinden biri
   yanlissa panel hicbir komut islemez ve ekran bos kalir.
5. SDI/MOSI (11) ve SCK (12) baglantilarini kontrol et.

**Goruntu var ama bozuk**:

6. Karlanma, kayan satirlar, rastgele pikseller: SPI hizini `[display]`
   altinda 27 MHz, sonra 20 MHz yap. Kablolari kisalt.
7. Renkler yanlis (kirmizi blogun uzerinde "BLUE" yaziyor gibi): RGB/BGR sirasi
   ters demektir, `build_flags` icine `-DTFT_RGB_ORDER=TFT_RGB` ekle.
8. Goruntu kaymis, kenarda ince bir serit var: kose pikselleri tam koselerde
   degilse offset sorunu vardir, `-DTFT_WIDTH` / `-DTFT_HEIGHT` degerlerini ve
   surucu tanimini kontrol et.
9. Goruntu 90 derece donuk veya aynalanmis: `DISPLAY_ROTATION` degeri
   `include/pins.h` icinde, 0-3 arasi denenebilir.

**Seride PSRAM BULUNAMADI yaziyorsa**: `board_build.arduino.memory_type =
qio_opi` satiri kaybolmustur, bu ayar N16R8 icin sart.

## Sonraki asamalar

Cihaz ileride USB uzerinden bilgisayara baglanacak, bilgisayardaki uygulama
ekran goruntusunu sikistirip gonderecek, cihaz sadece gelen bolgeleri ekrana
basacak. Girdi olarak rotary encoder ve fiziksel butonlar eklenecek.

Bu yuzden ekrana cizim isi `drawTestScreen()` ve `drawCounter()` icinde
toplandi. Sayac zaten tam olarak o akisi taklit ediyor: hazir bir tampon
olusturulup tek seferde belirli bir dikdortgene basiliyor, tum ekran yeniden
cizilmiyor.
