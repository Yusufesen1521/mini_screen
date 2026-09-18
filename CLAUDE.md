# mini_screen

Masaustunde duran, bilgisayara USB ile baglanan ikinci ekran.

**Yol haritasi ve faz tanimlari `plans.md` icinde. Is yapmadan once oku.**
Su anki asama: Faz 0 bitti (ekran dogrulama), Faz 1 siradaki (piksel hatti ve
USB protokolu).

Calisma kurali: bir faz, cikis kriterlerinin tamami tek tek dogrulanmadan
bitmis sayilmaz ve sonraki faza gecilmez. Olcum gerektiren kriterlerde gercek
sayi yazilir.

## Donanim (elde fiziksel olarak sadece bunlar var)

- ESP32-S3 DevKitC-1, N16R8 varyanti (16 MB flash, 8 MB oktal PSRAM)
- Lockerbox 3.2" TFT SPI, ILI9341, 240x320, v1.0 (LCDWiki MSP3218 muadili)
- Breadboard ve erkek-erkek jumper kablolar

**Dokunmatik YOK, olculerek dogrulandi.** Urun sayfasi dokunmatik diyordu,
poset "touch: no" diyordu. XPT2046 hatlari (T_CS GPIO 18, T_DO GPIO 13)
gecici olarak baglanip `tft.getTouchRawZ()` okundu: deger hem bagliyken hem
degilken sabit 0. Denetleyici olsaydi gurultu bile okunurdu. Dokunmatik kodu
kaldirildi, GPIO 13 ve 18 serbest. Bu konuyu tekrar acma, olculdu.

MISO baglanmiyor, `TFT_MISO` tanimlanmiyor. Ekrandan geri okuma yapilamaz.

## Pin haritasi

| Ekran | GPIO |
|---|---|
| VCC | 3V3 |
| GND | GND |
| CS | 10 |
| RESET | 9 |
| DC | 14 |
| SDI (MOSI) | 11 |
| SCK | 12 |
| LED | 21 |

Kullanilamaz pinler: GPIO 26-37 dahili flash ve oktal PSRAM tarafindan
kullaniliyor. GPIO 0, 3, 19, 20, 45, 46 strapping veya USB gorevli.

Bos ve kullanilabilir: GPIO 4, 5, 6, 7, 8, 13, 15, 16, 17, 18 ve saga taraftaki
1, 2, 35-42, 47, 48. Rotary encoder ve butonlar buradan secilecek.

## Kritik: USE_FSPI_PORT silinmemeli

TFT_eSPI 2.5.43, ESP32-S3 uzerinde `USE_FSPI_PORT` ya da `USE_HSPI_PORT`
tanimli degilse `SPI_PORT` makrosunu `FSPI` yapiyor. Arduino core 2.x icinde
ESP32-S3 icin `FSPI = 0`, `REG_SPI_BASE(0)` ise 0 donduruyor. Kutuphane SPI
register adresi olarak 0x10 kullaniyor ve `tft.init()` icinde
`Guru Meditation Error: StoreProhibited` ile cokuyor, kart sonsuz reset
dongusune giriyor, ekranda hicbir sey gorunmuyor.

`-DUSE_FSPI_PORT` ile `SPI_PORT` 2 olur (SPI2), bu da GPIO 10/11/12 pinlerinin
IOMUX karsiligi. `include/pins.h` icinde ikisinden biri yoksa derleme `#error`
ile durur.

## TFT_eSPI yapilandirmasi

Tum TFT_eSPI tanimlari `platformio.ini` icindeki `build_flags` altinda,
`-DUSER_SETUP_LOADED=1` ile. Kutuphanenin `User_Setup.h` dosyasini ASLA
degistirme, o dosya `.pio/libdeps` altinda ve kutuphane guncellemesinde silinir.

Derlemede cikan "TOUCH_CS pin not defined" uyarisi beklenen durumdur,
susturmaya calisma.

SPI hizi tek yerden degistirilir: `platformio.ini` icindeki `[display]` bolumu.
Breadboard uzerinde 40 MHz kararsiz olabilir, bozulma gorulurse 27 veya 20 MHz.

## Ekran yonu

Yatay kullaniliyor, gorunur olcu 320x240. `DISPLAY_ROTATION` degeri 3.
1 ve 3 yatay ve aralarinda 180 derece fark var, 0 ve 2 dikey.

`pins.h` icindeki `SCREEN_WIDTH` / `SCREEN_HEIGHT` donus sonrasi olculerdir,
`build_flags` icindeki `TFT_WIDTH` / `TFT_HEIGHT` ise panelin kendi 240x320
olcusu. Ikisini karistirma. Yon degisirse yerlesim sabitlerini de gozden
gecir; uyusmazlik olursa firmware acilista seri porta uyari basar.

## Seri port

`ARDUINO_USB_CDC_ON_BOOT=1` oldugu icin `Serial` yerlesik USB portuna
(GPIO 19/20) baglanir, UART kopru portuna degil. Bu yuzden `src/main.cpp`
icindeki `logPrintf()` ayni ciktiyi `Serial0` (UART0) uzerine de basar. Yeni log
eklerken `Serial.printf` degil `logPrintf` kullan.

## Komutlar

```bash
pio run
pio run -t upload -t monitor
```

Cokme ayiklama: seri porttan backtrace adreslerini al, sonra

```bash
~/.platformio/packages/toolchain-xtensa-esp32s3/bin/xtensa-esp32s3-elf-addr2line -pfiaC -e .pio/build/esp32-s3-devkitc-1/firmware.elf <adres>
```

## Kod kurallari

- Sihirli sayi yok. Her pin, zamanlama, yerlesim ve renk sabiti
  `include/pins.h` icinde isimlendirilmis olmali.
- Yorumlar Turkce, tutarli kalsin.
- Em dash karakteri hicbir dosyada kullanilmaz. Yerine tire, virgul, iki nokta
  veya parantez.
- Kisa ve okunur tut, gereksiz soyutlama katmani ekleme.
- Arka isik LEDC PWM ile surulur, `digitalWrite` ile degil.
- LEDC cagrilari `ESP_ARDUINO_VERSION_MAJOR` ile hem core 2.x hem 3.x icin
  korunuyor. Su an core 2.0.17 kullaniliyor.

## Sabit mimari kararlar

Bunlar karara baglandi, yeniden acilmayacak. Gerekcesi `plans.md` icinde.

1. **Bagli modda cizimi PC yapar, cihaz sadece basar.** Cihaz aptal bir
   cerceve, render PC tarafinda. Gorsel islerin cozumu gomulu tarafta
   aranmaz.
2. **Bagimsiz modda auth gerektiren hicbir sey yok.** Sadece saat, hava
   durumu, kayitli GIF ve duvar kagidi. OAuth, token saklama, sifreli kimlik
   firmware'e hic girmeyecek.
3. **Medya bilgisi Spotify ya da YouTube API'sinden degil, isletim sisteminin
   medya oturumundan okunur.** Windows'ta SMTC, Linux'ta MPRIS.
4. **Telefondan BLE ile ayar yapilmayacak.** Ayar icin cihazin kendi web
   arayuzu. BLE sadece iOS bildirimleri (ANCS) icin.
5. **Performans hedefi 24 FPS tam kare esdegeri ve bu karsilandi.** Daha
   hizli tasima ya da daha iyi sikistirma icin is yapilmayacak; baglanti
   sikistirma ile birlikte SPI tavanini zaten dolduruyor. Gerekce
   `plans.md` ve `docs/measurements.md` icinde.
6. **Kendi kodegimiz sadece RLE16.** Fotograf, GIF, video icin kutuphane
   kullanilacak (AnimatedGIF, TJpg_Decoder, MJPEG), kendi kodek yazilmaz.
7. **SPI 40 MHz.** Olculdu, kararli, darbogaz degil.
8. **Hava durumu icin anahtarsiz kaynak:** Open-Meteo.

## Kapsam disi (istenmedikce ekleme)

- Dokunmatik / XPT2046 kodu, donanimda yok, olculdu
- LVGL, bu mimaride gerekmiyor
- BLE ile telefondan ayar
- Android bildirimleri
- Bagimsiz modda borsa, takvim, e-posta (auth gerektiriyor, bagli modda kalir)
- Arduino IDE ile ilgili dosya veya talimat
- Elde olmayan komponent varsayimi
- Seri uretim, sertifikasyon, kasa: yedi faz bittikten sonra

## Mevcut kodun yeri

Faz 0 ciktisi olan test firmware, gelecek mimarinin iskeletini tasiyor:
`drawCounter()` zaten hedef akisin aynisi, yani sprite ile tampon hazirlanip
tek seferde belirli bir dikdortgene basiliyor, tum ekran yeniden cizilmiyor.
Faz 1'de bu yolun yerini USB protokolu alacak. Simdilik soyut arayuz katmani
yazilmayacak.
