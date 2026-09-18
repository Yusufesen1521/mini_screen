// pins.h - tum pin, zamanlama ve yerlesim sabitleri
//
// Donanim: ESP32-S3 DevKitC-1 (N16R8) + Lockerbox 3.2" ILI9341 240x320
//
// Ekran veri pinleri TFT_eSPI icin platformio.ini build_flags icinde
// tanimlanir (TFT_CS, TFT_RST, TFT_DC, TFT_MOSI, TFT_SCLK). Kutuphane bu
// makrolari derleme aninda gormek zorunda oldugu icin tek kaynak orasidir.
// Burada sadece okunur isimlerle yeniden yayinlaniyorlar, deger tekrari yok.

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
// Ekran SPI (MISO bagli degil, tanimlanmiyor)
#define PIN_TFT_CS    TFT_CS    // GPIO 10
#define PIN_TFT_RST   TFT_RST   // GPIO 9
#define PIN_TFT_DC    TFT_DC    // GPIO 14
#define PIN_TFT_MOSI  TFT_MOSI  // GPIO 11
#define PIN_TFT_SCLK  TFT_SCLK  // GPIO 12

// Arka isik. TFT_eSPI'ye birakilmadi, LEDC ile PWM surulyor.
#define PIN_TFT_BL    21

// ---------------------------------------------------------------------------
// Arka isik PWM (LEDC)
// ---------------------------------------------------------------------------
#define BL_PWM_CHANNEL          0     // sadece Arduino core 2.x icin gerekli
#define BL_PWM_FREQ_HZ          5000
#define BL_PWM_RESOLUTION_BITS  8     // duty 0-255, parlaklik ile birebir
#define BL_BRIGHTNESS_MAX       255
#define BL_BRIGHTNESS_OFF       0
#define BL_BRIGHTNESS_DEFAULT   200

// Acilis darbesi: ekran hic goruntu vermese bile firmware'in calistigini ve
// arka isik hattinin saglam oldugunu gozle dogrulamak icin.
#define BL_HEARTBEAT_PULSES     2
#define BL_HEARTBEAT_MS         120

// ---------------------------------------------------------------------------
// Seri port
// ---------------------------------------------------------------------------
#define SERIAL_BAUD        115200
#define SERIAL_WAIT_MS     1500   // USB CDC hazir olana kadar en fazla bekleme

// ---------------------------------------------------------------------------
// Ekran yerlesimi (dikey / portrait)
// ---------------------------------------------------------------------------
#define DISPLAY_ROTATION   0
#define SCREEN_WIDTH       240
#define SCREEN_HEIGHT      320

#define FONT_TITLE         4
#define FONT_LABEL         2
#define FONT_COUNTER       4

// Baslik seridi
#define TITLE_Y            6
#define TITLE_AREA_HEIGHT  36
#define TITLE_TEXT         "MINI SCREEN"

// Renk bloklari: RGB/BGR sirasinin dogrulugunu gozle kontrol etmek icin
#define BLOCK_COUNT        4
#define BLOCK_X            0
#define BLOCK_WIDTH        SCREEN_WIDTH
#define BLOCK_HEIGHT       40
#define BLOCK_FIRST_Y      40
#define BLOCK_LABEL_INSET  8

// Sayac etiketi (bir kez cizilir, sayac bolgesinin disinda kalir)
#define COUNTER_LABEL      "COUNTER"
#define COUNTER_LABEL_X    BLOCK_LABEL_INSET
#define COUNTER_LABEL_Y    208

// Sadece bu dikdortgen her saniye guncellenir
#define COUNTER_X          0
#define COUNTER_Y          228
#define COUNTER_WIDTH      SCREEN_WIDTH
#define COUNTER_HEIGHT     44
#define COUNTER_INTERVAL_MS 1000

// Kose isaretleri (offset kontrolu icin birer piksel)
#define CORNER_MARK_COUNT  4

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
