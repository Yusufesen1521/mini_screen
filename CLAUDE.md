# mini_screen

Masaustunde duran, bilgisayara USB ile baglanan ikinci ekran.

**Yol haritasi ve faz tanimlari `plans.md` icinde. Is yapmadan once oku.**
Su anki asama: **Faz 2, sekiz kriterin besi kapali.** Faz 0 ve Faz 1
bitti. PC kodu `pc/` altinda Rust ile, ayrintisi `pc/README.md`.

**Devam ederken once `plans.md` icindeki "Faz 2'de kalan is" bolumunu
oku.** Orada kalan dort is ve bekleyen panel gorus acisi konusu tek tek
yazili. Ilk yapilacak sey kullaniciya bekleme ekranini sormak: kod
yazildi ve yuklendi ama gorsel onay alinmadi.

Calisma kurali: bir faz, cikis kriterlerinin tamami tek tek dogrulanmadan
bitmis sayilmaz ve sonraki faza gecilmez. Olcum gerektiren kriterlerde gercek
sayi yazilir.

## Donanim (elde fiziksel olarak sadece bunlar var)

- ESP32-S3 DevKitC-1, N16R8 varyanti (16 MB flash, 8 MB oktal PSRAM)
- Lockerbox 3.2" TFT SPI, ILI9341, 240x320, v1.0 (LCDWiki MSP3218 muadili)
- Breadboard ve erkek-erkek jumper kablolar
- **Iki USB kablosu.** Kartin iki portu ayni anda takilabilir ve bu
  guvenli: Espressif kilavuzu "USB-to-UART Port and ESP32-S3 USB Port
  (either one or both)" diyor ve bunu onerilen varsayilan besleme yolu
  olarak gosteriyor. Disladigi sey USB ile 5V/3V3 pinlerinden ayni anda
  beslemek. Ikisini de ayni bilgisayara tak, toprak ortak olsun.
  Protokol yerlesik USB portundan, `Serial0` loglari UART kopru
  portundan gider; artik ikisi ayni anda mumkun.

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

Butonlar (iki bacakli switch, oteki bacak GND, dahili pull-up):

| Buton | GPIO | Islev |
|---|---|---|
| Ayar | 4 | kisa basis deger, uzun basis parametre |
| GIF | 5 | sonraki GIF |

Kullanilamaz pinler: GPIO 26-37 dahili flash ve oktal PSRAM tarafindan
kullaniliyor. GPIO 0, 3, 19, 20, 45, 46 strapping veya USB gorevli.

Bos ve kullanilabilir: GPIO 6, 7, 8, 13, 15, 16, 17, 18 ve saga taraftaki
1, 2, 35-42, 47, 48. Rotary encoder ve ek butonlar buradan secilecek.
(4 ve 5 test butonlarinda.)

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

## Panel register ayarlari

TFT_eSPI'nin ILI9341 init dizisi Adafruit'in jenerik varsayilanlarini
kullaniyor ve bu panelde **polarite tersleme titremesi** yapiyordu: sabit
ve koyu pikseller, ozellikle yuksek parlaklikta, parliyor sonuyordu.

`src/panel_settings.cpp` icindeki degerler gozle bulundu ve
`tft.init()` **sonrasinda** uygulaniyor (once uygulanirsa kutuphane
ustune yazar):

| Register | Secilen | Stok |
|---|---|---|
| VCOM2 (C7) | 0xB8 | 0x86 |
| VCOM1 (C5) | 30 30 | 3E 28 |
| Kare hizi (B1) | 112 Hz | 100 Hz |

Bu degerleri degistirmeden once `docs/measurements.md` icindeki Faz 1.5b
bolumunu oku. Bulma yontemi de orada: `env:paneltune` sabit bir gri skala
gosteriyor ve butonla canli ayar yapiliyor.

**Kalan sinir:** `rgb_test2.gif` klibinde parlaklik 180 ustunde titreme
her ayarla devam ediyor. Panelin sinirlarindan biri kabul edildi, TN
panel IPS degil. Bu konuyu tekrar acma, butun kombinasyonlar denendi.

Arka isik GPIO 21'den dogrudan suruluyor; pin 20-40 mA verebiliyor. Daha
parlak gerekirse cozum MOSFET ile 3V3'ten surmek, gerilim yukseltmek
degil: modulun akim sinirlama direnci 3.3V icin secilmis.

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

## Ortamlar

| Ortam | Ne yapar |
|---|---|
| `esp32-s3-devkitc-1` | Protokol alicisi, varsayilan |
| `gifplay` | GIF oynatici, iki butonlu ayar arayuzu |
| `paneltune` | Sabit gri skala, panel register ayari |
| `bench` / `bench80` | Ekran hatti olcumu, 40 ve 80 MHz |
| `usbdbg` | USB surucusu tanilamasi, cikti ekranda |

## Komutlar

```bash
cd pc && build.bat build --release         # PC uygulamasi (Rust)
cd pc && build.bat test                    # PC testleri
cd pc && build.bat clippy --all-targets    # lint, CI bunu -D warnings ile kosuyor
pio run                                    # protokol firmware
pio run -e gifplay -t upload               # GIF oynatici
python tools/prepare_gif.py                # gif/ -> data/, ekran olcusune
pio run -e gifplay -t uploadfs             # GIF'leri cihaza yaz
python tools/link_test.py conformance      # protokol uyum testleri
```

**Protokol testleri kablonun yerlesik USB portunda olmasini gerektirir.**
GIF ve log tarafi UART kopru portundan calisir. Tek kablo varsa hangi
testi yapacagina gore tasi.

Cokme ayiklama: seri porttan backtrace adreslerini al, sonra

```bash
~/.platformio/packages/toolchain-xtensa-esp32s3/bin/xtensa-esp32s3-elf-addr2line -pfiaC -e .pio/build/esp32-s3-devkitc-1/firmware.elf <adres>
```

## Bagli modda calistirma

PC uygulamasi cihaza baglaniyorsa once onu baslat, ekran ancak o zaman
canli olur. Uygulama kapaliyken cihaz 4 saniye sonra bekleme ekranina
duser, bu beklenen davranis.

```bash
cd pc && build.bat build --release
pc/target/release/mscreen.exe run            # sinirsiz
pc/target/release/mscreen.exe run --seconds 60
pc/target/release/mscreen.exe preview out.png  # cihazsiz, tasarim kontrolu
pc/target/release/mscreen.exe sensors          # sensor kaynaklarini yoklar
pc/target/release/mscreen.exe sensors --dump   # Afterburner ham girdileri
```

**GPU ve CPU sicakligi icin MSI Afterburner calisiyor olmali.** Kapaliysa
o alanlar `None` kalir ve widget o satirlari hic cizmez; bu hata degil,
tasarim. Afterburner'i kurulumcuya gommek EULA'ya tabi, dogru yaklasim
tespit edip kullaniciyi yonlendirmek.

## Kritik: `!Serial` baglanti kopma isareti degil

ESP32-S3 yerlesik USB CDC'sinde `!Serial` (HWCDC `operator bool`)
guvenilmez. Olculdu: aktif trafik sirasinda, son mesajin uzerinden
43 ms gecmisken "port kapali" dondu. Cihaz saniyeler icinde bekleme
ekranina girip cikti, arka isik kirpti, panel gorunmez oldu.

Baglantinin koptugunu anlamanin tek guvenilir yolu **sessizlik
suresi**: `LINK_IDLE_TIMEOUT_MS`. PC bostayken PING gonderiyor, cunku
dirty tracking yuzunden durgun ekranda hic cerceve gitmiyor.

## Kritik: Rust derlemesi ve VS 18

Makinede iki Visual Studio kurulu. rustup varsayilan olarak VS 18'i
seciyor ama o kurulumda `msvcrt.lib` yok, bu yuzden baglama
`LINK : fatal error LNK1104: 'msvcrt.lib' dosyasi acilamiyor` ile
dusuyor. VS 2022 kurulumunda kutuphane yerinde.

Gecici cozum `pc/build.bat`: once VS 2022 ortamini yukluyor, sonra
cargo'yu cagiriyor. Rust tarafinda duz `cargo` yerine bu betik
kullanilir.

**PC tarafinda acikli bir derleme ya da baglama hatasi gorursen ilk
supheli bu.** Once hatanin VS 18 kaynakli olup olmadigina bak, kodda
hata arama. Kullanici VS 18'i kaldirmayi planliyor; kaldirildiginda
`build.bat` gereksiz kalir ve duz `cargo build` calisir.

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
8. **GIF'ler cihaza yuklenmeden once ekran olcusune indirilir.**
   `tools/prepare_gif.py` ffmpeg ile yapiyor, ileride PC uygulamasinin
   gorevi olacak. Kazanc 14.8 yerine 19.0 FPS.
9. **Tek bir kontrole ikiden fazla islev yuklenmez.** Buton icin tavan
   kisa ve uzun basis. Daha fazlasi gerekiyorsa yeni buton eklenir.
10. **Hava durumu icin anahtarsiz kaynak:** Open-Meteo.

## Kapsam disi (istenmedikce ekleme)

- Dokunmatik / XPT2046 kodu, donanimda yok, olculdu
- LVGL, bu mimaride gerekmiyor
- BLE ile telefondan ayar
- Android bildirimleri
- Bagimsiz modda borsa, takvim, e-posta (auth gerektiriyor, bagli modda kalir)
- Arduino IDE ile ilgili dosya veya talimat
- Elde olmayan komponent varsayimi
- Seri uretim, sertifikasyon, kasa: yedi faz bittikten sonra

## Panel gorus acisi (acik konu)

Kullanici masada ortadan bakinca renklerin kotu, sagdan bakinca hicbir
seyin gorunmedigini bildirdi. Dikey bakis etkilemiyor.

Teshis: panel 240x320 dikey, biz `DISPLAY_ROTATION 3` ile yatay
kullaniyoruz. Panelin kendi dikey ekseni, yani TN'de kotu olan eksen,
ekranin yatay ekseni oluyor. Dikey bakisin etkilememesi bunun kaniti.

**ILI9341'de kontrast registeri yoktur.** Karsiligi gamadir ve
`src/panel_settings.cpp` gamayi (E0/E1) hic taramamis; VCOM, kare hizi,
tersleme ve guc gerilimi tarandi. Yani gama el degmemis bir eksen.

Denenecekler `plans.md` icindeki "Faz 2 disina cikan" bolumunde
sirali. Hicbiri TN panelini duzeltmez, sadece katlanilir yapar.

## Mevcut kodun yeri

Faz 0 ciktisi olan test firmware, gelecek mimarinin iskeletini tasiyor:
`drawCounter()` zaten hedef akisin aynisi, yani sprite ile tampon hazirlanip
tek seferde belirli bir dikdortgene basiliyor, tum ekran yeniden cizilmiyor.
Faz 1'de bu yolun yerini USB protokolu alacak. Simdilik soyut arayuz katmani
yazilmayacak.
