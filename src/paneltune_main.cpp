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
#include "panel_settings.h"
#include "pins.h"

static TFT_eSPI tft;
static uint8_t paramIndex = 0;

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
  const PanelParam &p = panelParams[paramIndex];
  char line[64];
  snprintf(line, sizeof(line), "%s  %s", p.name, p.values[p.index].label);

  tft.fillRect(0, STATUS_Y, SCREEN_WIDTH, SCREEN_HEIGHT - STATUS_Y, TFT_BLACK);
  tft.setTextDatum(TL_DATUM);
  tft.setTextColor(gray565(200), TFT_BLACK);
  tft.drawString(line, 4, STATUS_Y + 1, FONT_LABEL);

  snprintf(line, sizeof(line), "parametre %u/%u   deger %u/%u",
           (unsigned)(paramIndex + 1), (unsigned)panelParamCount,
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

  const bool raw = (digitalRead(PIN_BUTTON_TUNE) != LOW);
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
    paramIndex = (uint8_t)((paramIndex + 1) % panelParamCount);
    drawStatus();
  } else if (!down && pressed) {
    pressed = false;
    if (!longFired) {
      PanelParam &p = panelParams[paramIndex];
      p.index = (uint8_t)((p.index + 1) % p.count);
      panelApply(tft, p);
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

  pinMode(PIN_BUTTON_TUNE, INPUT_PULLUP);

  backlightBegin();
  backlightSet(BL_BRIGHTNESS_OFF);

  tft.init();
  tft.setRotation(DISPLAY_ROTATION);
  panelApplyAll(tft);   // gozle bulunan degerlerle basla
  drawPattern();
  drawStatus();

  backlightSet(BL_BRIGHTNESS_MAX);
}

void loop()
{
  pollButton();
  delay(5);
}
