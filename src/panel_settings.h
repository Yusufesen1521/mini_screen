// panel_settings.h - ILI9341 register ayarlari, gozle bulunanlar
//
// TFT_eSPI'nin ILI9341 init dizisi Adafruit'in jenerik varsayilanlarini
// kullaniyor ve bu panelde polarite tersleme titremesi yapiyordu: sabit
// ve koyu pikseller, ozellikle yuksek parlaklikta, kare hizinda parliyor
// sonuyordu.
//
// Asagidaki degerler sabit bir gri skala uzerinde gozle bulundu. Ayrintisi
// docs/measurements.md icinde. Baslangic indeksleri secilen degeri
// gosteriyor; listeler duruyor ki karsilastirma yapilabilsin.
//
// Ayarlar tft.init() cagrisindan SONRA uygulanmali, yoksa kutuphanenin
// init dizisi ustune yazar.

#pragma once

#include <stdint.h>

class TFT_eSPI;

struct PanelValue {
  const char *label;
  uint8_t     data[3];
  uint8_t     len;
};

struct PanelParam {
  const char       *name;
  uint8_t           cmd;
  const PanelValue *values;
  uint8_t           count;
  uint8_t           index;
};

extern PanelParam    panelParams[];
extern const uint8_t panelParamCount;

// Tek bir parametreyi panele yazar.
void panelApply(TFT_eSPI &tft, const PanelParam &p);

// Butun parametreleri secili degerleriyle yazar. tft.init() sonrasi
// cagrilmali.
void panelApplyAll(TFT_eSPI &tft);
