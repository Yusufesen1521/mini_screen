#include "panel_settings.h"

#include "pins.h"

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

// index alanlari gozle bulunan nihai degerleri gosteriyor.
//
// Once sabit bir gri skala uzerinde, sonra gercek GIF iceriginde
// dogrulandi. Sabit desende en iyi gorunen degerler (VCOM2 0x90,
// VCOM1 2B 2B) hareketli icerikte ayni sonucu vermedi; asagidakiler
// canli testin sonucu.
PanelParam panelParams[] = {
  { "VCOM2  (C7)", 0xC7, kVcom2,     COUNT_OF(kVcom2),     7 },  // 0xB8
  { "VCOM1  (C5)", 0xC5, kVcom1,     COUNT_OF(kVcom1),     3 },  // 30 30
  { "KareHz (B1)", 0xB1, kFrameRate, COUNT_OF(kFrameRate), 2 },  // 112 Hz
  { "Tersle (B4)", 0xB4, kInversion, COUNT_OF(kInversion), 0 },  // 0x02 stok
  { "Guc1   (C0)", 0xC0, kPower1,    COUNT_OF(kPower1),    0 },  // 0x23 stok
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

// ---------------------------------------------------------------------------
// Geri okuma
// ---------------------------------------------------------------------------
// ILI9341 okuma komutlari. Adlar veri sayfasindaki kisaltmalar.
static const uint8_t CMD_RDDID   = 0x04;
static const uint8_t CMD_RDDPM   = 0x0A;
static const uint8_t CMD_RDDMAD  = 0x0B;
static const uint8_t CMD_RDDCOL  = 0x0C;
static const uint8_t CMD_RDDIM   = 0x0D;
static const uint8_t CMD_RDDSM   = 0x0E;
static const uint8_t CMD_RDDSDR  = 0x0F;
static const uint8_t CMD_RDID4   = 0xD3;

PanelStatus panelReadStatus(TFT_eSPI &tft)
{
  PanelStatus s = {};
  for (uint8_t i = 0; i < 4; i++) {
    s.id[i]  = tft.readcommand8(CMD_RDDID, i);
    s.id4[i] = tft.readcommand8(CMD_RDID4, i);
  }
  s.power    = tft.readcommand8(CMD_RDDPM, 0);
  s.madctl   = tft.readcommand8(CMD_RDDMAD, 0);
  s.pixfmt   = tft.readcommand8(CMD_RDDCOL, 0);
  s.imgfmt   = tft.readcommand8(CMD_RDDIM, 0);
  s.signal   = tft.readcommand8(CMD_RDDSM, 0);
  s.selfdiag = tft.readcommand8(CMD_RDDSDR, 0);
  return s;
}

PanelHealth panelReadHealth(TFT_eSPI &tft)
{
  PanelHealth h = {};
  h.power    = tft.readcommand8(CMD_RDDPM, 0);
  h.pixfmt   = tft.readcommand8(CMD_RDDCOL, 0);
  h.selfdiag = tft.readcommand8(CMD_RDDSDR, 0);
  return h;
}

bool panelHealthy(const PanelHealth &h)
{
  // Guc modunda sadece anlamli bitlere bakiliyor: uyku disi, normal kip,
  // ekran acik. Booster ve idle bitleri karara girmiyor.
  if ((h.power & PANEL_PM_MASK) != PANEL_PM_EXPECTED) {
    return false;
  }
  return h.pixfmt == PANEL_COLMOD_EXPECTED;
}

uint16_t panelPixelRoundtrip(TFT_eSPI &tft, int32_t x, int32_t y, uint16_t color)
{
  tft.drawPixel(x, y, color);
  return tft.readPixel(x, y);
}
