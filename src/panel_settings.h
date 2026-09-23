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

// ---------------------------------------------------------------------------
// Panelden geri okuma
// ---------------------------------------------------------------------------
// MISO GPIO 13'e baglandigi icin ILI9341'in kimlik ve durum registerleri
// okunabiliyor. Bu, firmware'in "panel hala init edilmis durumda mi" diye
// sorabilmesinin tek yolu: yazma tarafi paneli hic dinlemese bile sessizce
// basarili gorunur.
//
// Alanlar ham. Yorum cagirana birakildi, cunku once gercekte ne geldigini
// olcmek gerekiyor; olculmemis bir "saglikli" tanimi yazmak tahmin olurdu.

struct PanelStatus {
  uint8_t id[4];     // 0x04 RDDID
  uint8_t id4[4];    // 0xD3 RDID4, ILI9341'de icinde 93 41 gecmeli
  uint8_t power;     // 0x0A RDDPM, guc modu
  uint8_t madctl;    // 0x0B RDDMADCTL, eksen ve renk sirasi
  uint8_t pixfmt;    // 0x0C RDDCOLMOD, piksel formati
  uint8_t imgfmt;    // 0x0D RDDIM
  uint8_t signal;    // 0x0E RDDSM
  uint8_t selfdiag;  // 0x0F RDDSDR, denetleyicinin kendi tanisi
};

// Registerleri okur. Yavas: SPI_READ_FREQUENCY uzerinden gidiyor ve
// ILI9341 okuma cevrimi 6.6 MHz ile sinirli. Sicak yolda cagrilmamali.
PanelStatus panelReadStatus(TFT_eSPI &tft);

// Karar icin gereken uc register. `panelReadStatus` on iki register
// okuyor ve acilista bir kez cagrilmak icin; bu ise periyodik cagrilabilir.
struct PanelHealth {
  uint8_t power;     // 0x0A RDDPM
  uint8_t pixfmt;    // 0x0C RDDCOLMOD
  uint8_t selfdiag;  // 0x0F RDDSDR
};

PanelHealth panelReadHealth(TFT_eSPI &tft);

// Denetleyici hala bizim biraktigimiz durumda mi.
//
// Esikler tahmin degil, olculdu: init sonrasi bu panelde RDDPM 0x9C,
// RDDCOLMOD 0x05, RDDSDR 0xC0 okunuyor. Olcum docs/measurements.md icinde.
bool panelHealthy(const PanelHealth &h);

// Tek pikselin gidip geri gelmesi. Register okumasi hattin calistigini
// gosterir, bu ise cerceve belleginin gercekten yazildigini gosterir.
// Yazilan renk `color`, donen degerse okunan.
uint16_t panelPixelRoundtrip(TFT_eSPI &tft, int32_t x, int32_t y, uint16_t color);
