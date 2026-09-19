#include "panel_settings.h"

#include <TFT_eSPI.h>

// VCOM ofseti. Titremeye en cok etki eden ayar.
static const PanelValue kVcom2[] = {
  { "0x86 stok", {0x86}, 1 },
  { "0x80", {0x80}, 1 },
  { "0x90", {0x90}, 1 },
  { "0x98", {0x98}, 1 },
  { "0xA0", {0xA0}, 1 },
  { "0xA8", {0xA8}, 1 },
  { "0xB0", {0xB0}, 1 },
  { "0xB8", {0xB8}, 1 },
  { "0xC0", {0xC0}, 1 },
};

// VCOMH ve VCOML cifti
static const PanelValue kVcom1[] = {
  { "3E 28 stok", {0x3E, 0x28}, 2 },
  { "35 3E", {0x35, 0x3E}, 2 },
  { "2B 2B", {0x2B, 0x2B}, 2 },
  { "30 30", {0x30, 0x30}, 2 },
  { "45 15", {0x45, 0x15}, 2 },
  { "3E 3E", {0x3E, 0x3E}, 2 },
};

// Panelin kendi tazeleme hizi. Yuksek hiz terslemeyi goze daha az gosterir.
static const PanelValue kFrameRate[] = {
  { "100 Hz stok", {0x00, 0x13}, 2 },
  { "119 Hz", {0x00, 0x10}, 2 },
  { "112 Hz", {0x00, 0x11}, 2 },
  { "90 Hz",  {0x00, 0x15}, 2 },
  { "79 Hz",  {0x00, 0x18}, 2 },
  { "70 Hz",  {0x00, 0x1B}, 2 },
};

// Tersleme bicimi
static const PanelValue kInversion[] = {
  { "0x02 stok", {0x02}, 1 },
  { "0x00", {0x00}, 1 },
  { "0x01", {0x01}, 1 },
  { "0x07", {0x07}, 1 },
};

// Surme gerilimi. Gama ve siyah seviyesini etkiliyor.
static const PanelValue kPower1[] = {
  { "0x23 stok", {0x23}, 1 },
  { "0x1B", {0x1B}, 1 },
  { "0x1F", {0x1F}, 1 },
  { "0x26", {0x26}, 1 },
  { "0x2B", {0x2B}, 1 },
};

#define COUNT_OF(a) ((uint8_t)(sizeof(a) / sizeof((a)[0])))

// index alanlari gozle bulunan degerleri gosteriyor
PanelParam panelParams[] = {
  { "VCOM2  (C7)", 0xC7, kVcom2,     COUNT_OF(kVcom2),     2 },  // 0x90
  { "VCOM1  (C5)", 0xC5, kVcom1,     COUNT_OF(kVcom1),     2 },  // 2B 2B
  { "KareHz (B1)", 0xB1, kFrameRate, COUNT_OF(kFrameRate), 2 },  // 112 Hz
  { "Tersle (B4)", 0xB4, kInversion, COUNT_OF(kInversion), 0 },  // 0x02
  { "Guc1   (C0)", 0xC0, kPower1,    COUNT_OF(kPower1),    0 },  // 0x23
};

const uint8_t panelParamCount = COUNT_OF(panelParams);

void panelApply(TFT_eSPI &tft, const PanelParam &p)
{
  const PanelValue &v = p.values[p.index];
  tft.writecommand(p.cmd);
  for (uint8_t i = 0; i < v.len; i++) {
    tft.writedata(v.data[i]);
  }
}

void panelApplyAll(TFT_eSPI &tft)
{
  for (uint8_t i = 0; i < panelParamCount; i++) {
    panelApply(tft, panelParams[i]);
  }
}
