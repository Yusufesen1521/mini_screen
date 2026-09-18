// usbdbg_main.cpp - USB surucusunu ekranda tanila
//
// Log hatti UART0 uzerinde ve tek kablo protokol icin USB portunda. Yani
// surucu ayaga kalkmazsa seri porttan hicbir sey gorulemiyor. Cozum: tani
// ciktisini ekrana basmak. Ekran USB'den bagimsiz calisiyor ve 13 satir
// aliyor.
//
// Sinanan hipotez: ARDUINO_USB_CDC_ON_BOOT=1 iken USB Serial/JTAG cevre
// biriminin saatini Arduino'nun HWCDC surucusu aciyor. 0 yapildiginda kimse
// acmiyor ve usb_serial_jtag_driver_install sessizce ise yaramiyor.
//
// pio run -e usbdbg -t upload

#include <Arduino.h>
#include <TFT_eSPI.h>
#include <driver/usb_serial_jtag.h>
#include <hal/usb_serial_jtag_ll.h>
#include <soc/usb_serial_jtag_struct.h>
#include <stdarg.h>

#include "backlight.h"
#include "pins.h"

#define DBG_LINE_HEIGHT  16
#define DBG_TOP          24
#define DBG_LEFT         6
#define DBG_MAX_LINES    12

static TFT_eSPI tft;
static uint8_t  lineCount = 0;

static void dbgLine(const char *fmt, ...)
{
  if (lineCount >= DBG_MAX_LINES) {
    return;
  }

  char text[64];
  va_list args;
  va_start(args, fmt);
  vsnprintf(text, sizeof(text), fmt, args);
  va_end(args);

  tft.setTextDatum(TL_DATUM);
  tft.setTextColor(COLOR_TEXT, COLOR_BACKGROUND);
  tft.drawString(text, DBG_LEFT, DBG_TOP + lineCount * DBG_LINE_HEIGHT, FONT_LABEL);
  lineCount++;
}

// Alt bolum: surekli guncellenen sayaclar
static void dbgStatus(const char *line1, const char *line2)
{
  const int16_t y = SCREEN_HEIGHT - 2 * DBG_LINE_HEIGHT - 6;

  tft.fillRect(0, y, SCREEN_WIDTH, 2 * DBG_LINE_HEIGHT + 4, COLOR_BACKGROUND);
  tft.setTextDatum(TL_DATUM);
  tft.setTextColor(COLOR_CORNER_MARK, COLOR_BACKGROUND);
  tft.drawString(line1, DBG_LEFT, y, FONT_LABEL);
  tft.drawString(line2, DBG_LEFT, y + DBG_LINE_HEIGHT, FONT_LABEL);
}

static uint8_t  rxBuf[512];
static uint32_t rxTotal = 0;
static uint32_t txCalls = 0;
static int      lastTx = 0;
static int      lastRx = 0;
static int      txEverWritable = 0;

void setup()
{
  backlightBegin();
  backlightSet(BL_BRIGHTNESS_OFF);

  tft.init();
  tft.setRotation(DISPLAY_ROTATION);
  tft.fillScreen(COLOR_BACKGROUND);

  tft.setTextDatum(TC_DATUM);
  tft.setTextColor(COLOR_TEXT, COLOR_BACKGROUND);
  tft.drawString("USB SURUCU TANI", SCREEN_WIDTH / 2, 4, FONT_LABEL);
  tft.drawFastHLine(0, 20, SCREEN_WIDTH, COLOR_SEPARATOR);

  backlightSet(BL_BRIGHTNESS_DEFAULT);

  dbgLine("CDC_ON_BOOT = %d", (int)ARDUINO_USB_CDC_ON_BOOT);
  dbgLine("basta txfifo:%d pad:%d pull:%d",
          usb_serial_jtag_ll_txfifo_writable(),
          (int)USB_SERIAL_JTAG.conf0.usb_pad_enable,
          (int)USB_SERIAL_JTAG.conf0.dp_pullup);

  // PHY ve pad yapilandirmasi. ESP-IDF surucusu bunu yapmiyor, konsolun
  // zaten yaptigini varsayiyor. CDC_ON_BOOT=1 iken Arduino'nun HWCDC
  // surucusu yapiyordu (HWCDC.cpp:336-343); 0 yapinca kimse yapmiyor.
  usb_serial_jtag_ll_enable_bus_clock(true);
  USB_SERIAL_JTAG.conf0.phy_sel = 0;           // dahili PHY
  USB_SERIAL_JTAG.conf0.pad_pull_override = 0;
  USB_SERIAL_JTAG.conf0.dp_pullup = 1;
  USB_SERIAL_JTAG.conf0.usb_pad_enable = 1;

  dbgLine("PHY kuruldu");
  dbgLine("sonra txfifo:%d pad:%d pull:%d",
          usb_serial_jtag_ll_txfifo_writable(),
          (int)USB_SERIAL_JTAG.conf0.usb_pad_enable,
          (int)USB_SERIAL_JTAG.conf0.dp_pullup);

  // Surucuyu kur
  usb_serial_jtag_driver_config_t cfg = {
    .tx_buffer_size = CDC_TX_BUFFER_SIZE,
    .rx_buffer_size = CDC_RX_BUFFER_SIZE,
  };
  const esp_err_t err = usb_serial_jtag_driver_install(&cfg);
  dbgLine("install = %d", (int)err);
  dbgLine("  %s", esp_err_to_name(err));
  dbgLine("rx tampon %u  tx %u",
          (unsigned)CDC_RX_BUFFER_SIZE, (unsigned)CDC_TX_BUFFER_SIZE);
}

void loop()
{
  static uint32_t lastTick = 0;
  static uint32_t ticks = 0;

  const int got = usb_serial_jtag_read_bytes(rxBuf, sizeof(rxBuf), 0);
  if (got > 0) {
    lastRx = got;
    rxTotal += (uint32_t)got;
    lastTx = usb_serial_jtag_write_bytes(rxBuf, (size_t)got, pdMS_TO_TICKS(50));
    txCalls++;
  }

  if (millis() - lastTick >= 500) {
    lastTick = millis();
    ticks++;

    if (usb_serial_jtag_ll_txfifo_writable()) {
      txEverWritable = 1;
    }

    char l1[64];
    char l2[64];
    snprintf(l1, sizeof(l1), "tick %lu  rx %lu  son rx %d",
             (unsigned long)ticks, (unsigned long)rxTotal, lastRx);
    snprintf(l2, sizeof(l2), "tx %lu donus %d  txfifo gordu %d",
             (unsigned long)txCalls, lastTx, txEverWritable);
    dbgStatus(l1, l2);

    // Saniyede bir kendiliginden de yazmayi dene
    if (ticks % 2 == 0) {
      char beat[48];
      const int n = snprintf(beat, sizeof(beat), "beat %lu rx=%lu\n",
                             (unsigned long)ticks, (unsigned long)rxTotal);
      lastTx = usb_serial_jtag_write_bytes(beat, (size_t)n, pdMS_TO_TICKS(50));
      txCalls++;
    }
  }
}
