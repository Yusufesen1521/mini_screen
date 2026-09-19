// pins.h - tum pin, zamanlama ve yerlesim sabitleri
//
// Donanim: ESP32-S3 DevKitC-1 (N16R8) + Lockerbox 3.2" ILI9341 240x320
//
// Ekran pinleri TFT_eSPI icin platformio.ini build_flags icinde tanimlanir
// (TFT_CS, TFT_RST, TFT_DC, TFT_MOSI, TFT_SCLK). Kutuphane bu makrolari
// derleme aninda gormek zorunda oldugu icin tek kaynak orasidir. Burada
// sadece okunur isimlerle yeniden yayinlaniyorlar.

#pragma once

#include <stdint.h>

// ---------------------------------------------------------------------------
// Derleme zamani kontrolu
// ---------------------------------------------------------------------------
#if !defined(TFT_CS) || !defined(TFT_RST) || !defined(TFT_DC) || \
    !defined(TFT_MOSI) || !defined(TFT_SCLK)
#error "TFT pin tanimlari eksik. platformio.ini icindeki build_flags kontrol edilmeli."
#endif

// TFT_eSPI 2.5.43, ESP32-S3'te port secilmemisse SPI_PORT degerini FSPI (= 0)
// yapiyor; REG_SPI_BASE(0) sifir donduruyor ve kutuphane tft.init() icinde
// 0x10 adresine yazarak StoreProhibited ile cokuyor. Bu yuzden port secimi
// zorunlu tutuluyor.
#if !defined(USE_FSPI_PORT) && !defined(USE_HSPI_PORT)
#error "USE_FSPI_PORT ya da USE_HSPI_PORT tanimlanmali, yoksa tft.init() cokuyor."
#endif

// ---------------------------------------------------------------------------
// Pinler
// ---------------------------------------------------------------------------
// Ekran SPI. MISO baglanmiyor, ekrandan geri okuma yapilmiyor.
#define PIN_TFT_CS    TFT_CS    // GPIO 10
#define PIN_TFT_RST   TFT_RST   // GPIO 9
#define PIN_TFT_DC    TFT_DC    // GPIO 14
#define PIN_TFT_MOSI  TFT_MOSI  // GPIO 11
#define PIN_TFT_SCLK  TFT_SCLK  // GPIO 12

// Arka isik. TFT_eSPI'ye birakilmadi, LEDC ile PWM surulyor.
#define PIN_TFT_BL    21

// Test butonu. Iki bacakli mekanik switch: bir bacak bu pine, oteki GND'ye.
// Dahili pull-up kullaniliyor, basilinca LOW okunuyor, harici direnc yok.
//
// GPIO 4 ve 5 secildi: bos, strapping gorevi yok, dahili pull-up
// destekliyorlar. GPIO 26'ya BAGLAMA, orasi oktal PSRAM tarafindan
// kullaniliyor.
//
// Iki ayri buton var cunku tek butona ucuncu bir islev yuklemek kullanisli
// degil. Ayar butonu: kisa basis deger, uzun basis parametre. GIF butonu:
// sonraki GIF.
#define PIN_BUTTON_TUNE   4
#define PIN_BUTTON_GIF    5

// ---------------------------------------------------------------------------
// Arka isik PWM (LEDC)
// ---------------------------------------------------------------------------
#define BL_PWM_CHANNEL          0     // sadece Arduino core 2.x icin gerekli
#define BL_PWM_FREQ_HZ          5000
#define BL_PWM_RESOLUTION_BITS  8     // duty 0-255, parlaklik ile birebir
#define BL_BRIGHTNESS_MAX       255
#define BL_BRIGHTNESS_OFF       0

// Gozle secilen varsayilan. 200 sonuk bulundu, 255 ise titremeyi
// belirginlestiriyor; 180-220 araligi gercek icerikte iyi calisiyor.
#define BL_BRIGHTNESS_DEFAULT   220
// Bekleme ekraninda arka isik kisiliyor ama sondurulmuyor: cihazin
// canli oldugu gorunsun, sadece dikkat cekmesin.
#define BL_BRIGHTNESS_STANDBY   70

// Acilis darbesi: ekran hic goruntu vermese bile firmware calistigini ve
// arka isik hattinin saglam oldugunu gozle dogrulamak icin.
#define BL_HEARTBEAT_PULSES     2
#define BL_HEARTBEAT_MS         120

// ---------------------------------------------------------------------------
// Seri port
// ---------------------------------------------------------------------------
#define SERIAL_BAUD        115200
#define SERIAL_WAIT_MS     1500   // USB CDC hazir olana kadar en fazla bekleme

// PC baglanmadan once uretilen log satirlari burada birikir, baglanti
// kurulunca protokol uzerinden gonderilir.
#define LOG_BOOT_BUF_SIZE  2048

// USB tamponlari, ESP-IDF usb_serial_jtag surucusu icin. Halka tampon
// olarak dahili RAM'de duruyor, PSRAM'e alinamaz.
//
// RX tamponu ekrana basma suresini karsilayacak kadar buyuk olmali:
// tam kare push 35 ms suruyor, o sirada gelen veri buraya birikiyor.
#define CDC_RX_BUFFER_SIZE    16384
#define CDC_TX_BUFFER_SIZE    4096
#define CDC_WRITE_TIMEOUT_MS  100

// Ekrana basma gorevi. Okuma ile basmanin ortusmesi icin ayri cekirdekte.
// Arduino loop'u 1. cekirdekte calistigi icin bu 0'a veriliyor.
#define PUSH_TASK_CORE        0
#define PUSH_TASK_PRIORITY    2
#define PUSH_TASK_STACK       4096

// Bos cozme tamponu beklerken en fazla bu kadar beklenir. Asilirsa PC
// akis penceresini asmis demektir ve OVERRUN dondurulur.
#define PUSH_WAIT_MS          1000

// ---------------------------------------------------------------------------
// Ekran yerlesimi (yatay)
//
// Panelin kendi olculeri 240x320 dikey; bunlar build_flags icindeki
// TFT_WIDTH / TFT_HEIGHT. Asagidaki degerler DISPLAY_ROTATION uygulandiktan
// sonraki gorunur olculer.
// ---------------------------------------------------------------------------
#define DISPLAY_ROTATION   3      // 1 ve 3 yatay, aralarinda 180 derece fark var
#define SCREEN_WIDTH       320
#define SCREEN_HEIGHT      240

#define FONT_TITLE         4
#define FONT_LABEL         2
#define FONT_COUNTER       4

// Baslik seridi
#define TITLE_Y            6
#define TITLE_AREA_HEIGHT  34
#define TITLE_TEXT         "MINI SCREEN"

// Renk bloklari yan yana dort sutun. RGB/BGR sirasinin dogrulugunu gozle
// kontrol etmek icin.
#define BLOCK_COUNT        4
#define BLOCK_FIRST_X      0
#define BLOCK_Y            40
#define BLOCK_WIDTH        (SCREEN_WIDTH / BLOCK_COUNT)   // 80
#define BLOCK_HEIGHT       104

// Acilis ekranindaki durum yazisi. PC baglandiginda ekran temizlenir ve
// bundan sonrasini PC cizer.
#define SPLASH_TEXT        "PC BEKLENIYOR (USB)"

// Bekleme ekrani. PC uygulamasi kapandiginda ya da olduginde cihaz
// donmus son kareyle kalmasin diye buraya duser.
//
// Neden zaman asimi, neden baska bir sey degil:
//
// Dirty tracking sayesinde ekran degismiyorken PC dakikalarca hicbir
// sey gondermiyor ve bu normal. Bu yuzden sayac herhangi bir gecerli
// mesajla sifirlaniyor ve PC bostayken PING gonderiyor.
//
// CDC baglilik durumu (`!Serial`) denendi ve guvenilmez cikti: aktif
// trafik sirasinda, son mesajin uzerinden 43 ms gecmisken bile "port
// kapali" dedi. Tek olcut sessizlik suresi.
//
// PC tarafindaki PING araligi bunun ucte biri civarinda olmali,
// su an 1500 ms.
#define LINK_IDLE_TIMEOUT_MS   4000
#define STANDBY_TITLE      "BAGLANTI YOK"
#define STANDBY_HINT       "PC uygulamasi calismiyor"
#define STANDBY_TITLE_Y    100
#define STANDBY_HINT_Y     134
#define SPLASH_X           8
#define SPLASH_Y           160

// Kose isaretleri (offset kontrolu icin birer piksel)
#define CORNER_MARK_COUNT  4

// ---------------------------------------------------------------------------
// GIF oynatici
// ---------------------------------------------------------------------------
// Olcum bittikten sonra surekli oynatilacak dosya. Dosya sisteminde
// yoksa bulunan son GIF oynatilir. Degistirmek icin sadece firmware
// yuklemek yeterli, dosya sistemini tekrar yazmaya gerek yok.
#define GIF_PLAY_FILE       "/getsuga.gif"

// Olcekleme haritalarinin boyu. Kaynak GIF bundan buyukse reddedilir.
#define GIF_MAX_SRC_DIM     1024

// Acilista her GIF olculsun mu.
//
// Varsayilan kapali. Acikken bes klip ucer tur, iki gecis halinde
// oynatiliyor ve bu yaklasik 90 saniye suruyor; o sure boyunca setup()
// bitmedigi icin loop() calismiyor, yani butonlar olu ve ekranda
// olcumden kalma bir kare duruyor. Cihaz bozuk sanildi.
//
// Olcum gerektiginde 1 yapilir.
#define GIF_MEASURE_AT_BOOT 0

// Olcum acikken kac tur oynatilsin
#define GIF_MEASURE_LOOPS   3

// Surekli oynatmada kac ms'de bir hiz raporu basilsin
#define GIF_REPORT_MS       5000

// ---------------------------------------------------------------------------
// Test butonu
// ---------------------------------------------------------------------------
#define BUTTON_DEBOUNCE_MS  30
#define BUTTON_LONG_MS      600    // bu sureden uzun basis parametre degistirir

// Buton basilinca alt seritte gosterilen durum yazisi
#define STATUS_HEIGHT       18
#define STATUS_SHOW_MS      2000

// Uzun basista sirayla gezilen parlaklik seviyeleri
#define BL_LEVELS           { 32, 64, 128, 180, 220, 255 }
#define BL_LEVEL_COUNT      6
#define BL_LEVEL_START      4      // BL_LEVELS icinde acilis seviyesi (220)

// Ayni anda taninan en fazla GIF sayisi
#define GIF_MAX_FILES       8

// Beklerken bu suredan uzunsa sirayi birak, kisaysa mesgul bekle
#define GIF_YIELD_US        2000

// Hedefin bu kadar gerisine dusersek birikimi sifirla
#define GIF_RESYNC_US       100000

// ---------------------------------------------------------------------------
// Renkler (RGB565)
// ---------------------------------------------------------------------------
#define COLOR_BACKGROUND    0x0000  // siyah
#define COLOR_TEXT          0xFFFF  // beyaz
#define COLOR_DIM_TEXT      0x7BEF  // gri
#define COLOR_CORNER_MARK   0xFFE0  // sari
#define COLOR_SEPARATOR     0x39E7  // koyu gri

#define COLOR_BLOCK_RED     0xF800
#define COLOR_BLOCK_GREEN   0x07E0
#define COLOR_BLOCK_BLUE    0x001F
#define COLOR_BLOCK_WHITE   0xFFFF
