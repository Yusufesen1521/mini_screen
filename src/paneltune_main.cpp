// paneltune_main.cpp - panel register ayarlarini gozle bulmak icin
//
// Ekranda hic yeniden yazilmayan sabit bir gri skala duruyor. Titreme
// goruluyorsa kaynagi kesin olarak panelin kendisi: bizim veri yolumuz
// o pikselleri bir daha hic tuslamiyor.
//
// Butonla ayarlar canli degistiriliyor:
//   kisa basis : suanki parametrenin sonraki degeri
//   uzun basis : sonraki parametre
//
// Sinanan sey polarite tersleme titremesi. TFT panellerde piksel gerilimi
// her karede polarite degistirir; VCOM referansi tam ortalanmamissa
// pozitif ve negatif kare farkli parlaklik verir ve piksel iki seviye
// arasinda gidip gelir. Sabit ve koyu piksellerde en cok gorunur.
//
//   pio run -e paneltune -t upload

#include <Arduino.h>
#include <TFT_eSPI.h>

#include "backlight.h"
#include "log.h"
#include "pins.h"

static TFT_eSPI tft;

// ---------------------------------------------------------------------------
// Ayarlanacak parametreler
//
// Baslangic degerleri TFT_eSPI'nin ILI9341 init dizisinden geliyor
// (ILI9341_Init.h). Bunlar Adafruit'in jenerik varsayilanlari, bu panele
// gore ayarlanmis degil.
// ---------------------------------------------------------------------------

struct Setting {
  const char *label;
  uint8_t     data[3];
  uint8_t     len;
};

// VCOM ofseti. Titremeye en cok etki eden ayar.
static const Setting kVcom2[] = {
  { "0x86 varsayilan", {0x86}, 1 },
  { "0x80", {0x80}, 1 },
  { "0x90", {0x90}, 1 },
  { "0x98", {0x98}, 1 },
  { "0xA0", {0xA0}, 1 },
  { "0xA8", {0xA8}, 1 },
  { "0xB0", {0xB0}, 1 },
  { "0xB8", {0xB8}, 1 },
  { "0xC0", {0xC0}, 1 },
};

// VCOMH ve VCOML ciftleri
static const Setting kVcom1[] = {
  { "3E 28 varsayilan", {0x3E, 0x28}, 2 },
  { "35 3E", {0x35, 0x3E}, 2 },
  { "2B 2B", {0x2B, 0x2B}, 2 },
  { "30 30", {0x30, 0x30}, 2 },
  { "45 15", {0x45, 0x15}, 2 },
  { "3E 3E", {0x3E, 0x3E}, 2 },
};

// Panelin kendi tazeleme hizi. Yuksek hiz terslemeyi goze daha az gosterir.
static const Setting kFrameRate[] = {
  { "100 Hz varsayilan", {0x00, 0x13}, 2 },
  { "119 Hz", {0x00, 0x10}, 2 },
  { "112 Hz", {0x00, 0x11}, 2 },
  { "90 Hz",  {0x00, 0x15}, 2 },
  { "79 Hz",  {0x00, 0x18}, 2 },
  { "70 Hz",  {0x00, 0x1B}, 2 },
};

// Tersleme bicimi. Satir terslemesi kare terslemesinden cok daha az titrer.
static const Setting kInversion[] = {
  { "0x02 varsayilan", {0x02}, 1 },
  { "0x00", {0x00}, 1 },
  { "0x01", {0x01}, 1 },
  { "0x07", {0x07}, 1 },
};

// Surme gerilimi. Gama ve siyah seviyesini etkiliyor.
static const Setting kPower1[] = {
  { "0x23 varsayilan", {0x23}, 1 },
  { "0x1B", {0x1B}, 1 },
  { "0x1F", {0x1F}, 1 },
  { "0x26", {0x26}, 1 },
  { "0x2B", {0x2B}, 1 },
};

struct Param {
  const char    *name;
  uint8_t        cmd;
  const Setting *values;
  uint8_t        count;
  uint8_t        index;
};

static Param params[] = {
  { "VCOM2  (C7)",  0xC7, kVcom2,      sizeof(kVcom2) / sizeof(Setting),      0 },
  { "VCOM1  (C5)",  0xC5, kVcom1,      sizeof(kVcom1) / sizeof(Setting),      0 },
  { "KareHz (B1)",  0xB1, kFrameRate,  sizeof(kFrameRate) / sizeof(Setting),  0 },
  { "Tersle (B4)",  0xB4, kInversion,  sizeof(kInversion) / sizeof(Setting),  0 },
  { "Guc1   (C0)",  0xC0, kPower1,     sizeof(kPower1) / sizeof(Setting),     0 },
};
static const uint8_t kParamCount = sizeof(params) / sizeof(params[0]);
static uint8_t paramIndex = 0;

static void applyParam(const Param &p)
{
  const Setting &s = p.values[p.index];
  tft.writecommand(p.cmd);
  for (uint8_t i = 0; i < s.len; i++) {
    tft.writedata(s.data[i]);
  }
}

// ---------------------------------------------------------------------------
// Test deseni
//
// Koyudan acige gri bantlar. Bir kez ciziliyor, bir daha dokunulmuyor.
// Polarite terslemesi gama egrisinin en dik oldugu yerde, yani koyu
// tonlarda en cok gorunur; bantlar o bolgeyi tariyor.
// ---------------------------------------------------------------------------

#define BAND_COUNT   7
#define STATUS_Y     (SCREEN_HEIGHT - 30)

static uint16_t gray565(uint8_t level)
{
  return (uint16_t)(((level & 0xF8) << 8) | ((level & 0xFC) << 3) | (level >> 3));
}

static void drawPattern()
{
  static const uint8_t levels[BAND_COUNT] = { 0, 8, 16, 24, 32, 48, 64 };
  const int16_t bandH = STATUS_Y / BAND_COUNT;

  for (uint8_t i = 0; i < BAND_COUNT; i++) {
    const int16_t y = i * bandH;
    const int16_t h = (i == BAND_COUNT - 1) ? (STATUS_Y - y) : bandH;
    tft.fillRect(0, y, SCREEN_WIDTH, h, gray565(levels[i]));

    char text[8];
    snprintf(text, sizeof(text), "%u", (unsigned)levels[i]);
    tft.setTextDatum(TL_DATUM);
    tft.setTextColor(gray565(160), gray565(levels[i]));
    tft.drawString(text, 4, y + 2, FONT_LABEL);
  }
}

static void drawStatus()
{
  const Param &p = params[paramIndex];
  char line[64];
  snprintf(line, sizeof(line), "%s  %s", p.name, p.values[p.index].label);

  tft.fillRect(0, STATUS_Y, SCREEN_WIDTH, SCREEN_HEIGHT - STATUS_Y, TFT_BLACK);
  tft.setTextDatum(TL_DATUM);
  tft.setTextColor(gray565(200), TFT_BLACK);
  tft.drawString(line, 4, STATUS_Y + 1, FONT_LABEL);

  snprintf(line, sizeof(line), "parametre %u/%u   deger %u/%u",
           (unsigned)(paramIndex + 1), (unsigned)kParamCount,
           (unsigned)(p.index + 1), (unsigned)p.count);
  tft.drawString(line, 4, STATUS_Y + 15, FONT_LABEL);

  logPrintf("%s = %s\n", p.name, p.values[p.index].label);
}

// ---------------------------------------------------------------------------
// Buton
// ---------------------------------------------------------------------------

static void pollButton()
{
  static bool     lastRaw = true;
  static uint32_t changeMs = 0;
  static bool     pressed = false;
  static uint32_t pressMs = 0;
  static bool     longFired = false;

  const bool raw = (digitalRead(PIN_BUTTON) != LOW);
  const uint32_t now = millis();

  if (raw != lastRaw) {
    lastRaw = raw;
    changeMs = now;
    return;
  }
  if ((now - changeMs) < BUTTON_DEBOUNCE_MS) {
    return;
  }

  const bool down = !raw;

  if (down && !pressed) {
    pressed = true;
    pressMs = now;
    longFired = false;
  } else if (down && pressed && !longFired &&
             (now - pressMs) >= BUTTON_LONG_MS) {
    longFired = true;
    paramIndex = (uint8_t)((paramIndex + 1) % kParamCount);
    drawStatus();
  } else if (!down && pressed) {
    pressed = false;
    if (!longFired) {
      Param &p = params[paramIndex];
      p.index = (uint8_t)((p.index + 1) % p.count);
      applyParam(p);
      drawStatus();
    }
  }
}

// ---------------------------------------------------------------------------

void setup()
{
  logBegin();
  logPrintf("\n=== panel ayar araci ===\n");
  logPrintf("Kisa basis: sonraki deger. Uzun basis: sonraki parametre.\n");
  logPrintf("Ekrandaki desen bir kez cizilip bir daha dokunulmuyor.\n\n");

  pinMode(PIN_BUTTON, INPUT_PULLUP);

  backlightBegin();
  backlightSet(BL_BRIGHTNESS_OFF);

  tft.init();
  tft.setRotation(DISPLAY_ROTATION);
  drawPattern();
  drawStatus();

  backlightSet(BL_BRIGHTNESS_MAX);
}

void loop()
{
  pollButton();
  delay(5);
}
