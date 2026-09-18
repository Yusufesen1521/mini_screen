// bench_main.cpp - Faz 1.1: ekran tarafinin tavanini olc
//
// Cevaplanacak soru: bu panele saniyede kac bayt basabiliyorum ve hangi yol
// en hizlisi? Sonuclar protokol tasariminin girdisi olacak.
//
// Olculen eksenler:
//   - Kaynak tampon: dahili SRAM (DMA uyumlu) ve PSRAM
//   - Yol: pushImage (bloklayan) ve pushImageDMA
//   - Bayt sirasi: setSwapBytes acik ve kapali. PC'nin pikselleri onceden
//     dogru sirada gondermesi anlamli mi, onu olcuyoruz.
//   - Bolge boyutu: tam kare ve kucuk dikdortgenler
//
// SPI hizi derleme zamani sabiti. 40 ve 80 MHz icin ayri ortamlar var:
//   pio run -e bench   -t upload -t monitor
//   pio run -e bench80 -t upload -t monitor

#include <Arduino.h>
#include <TFT_eSPI.h>
#include <esp_heap_caps.h>

#include "backlight.h"
#include "log.h"
#include "pins.h"

// TFT_eSPI 2.5.43'un DMA yolu ESP32-S3'te cokuyor, bu yuzden kapali.
//
// Kok neden: kutuphane iki farkli numaralandirmayi karistiriyor.
//   SPI_PORT  -> register indeksi, gecerli degerler 2 (SPI2) ve 3 (SPI3)
//   spi_host  -> ESP-IDF host enum'u, SPI2_HOST=1 ve SPI3_HOST=2
// USE_FSPI_PORT ile spi_host = SPI2_HOST = 1 oluyor. dma_end_callback
// icinde (TFT_eSPI_ESP32_S3.c:828) bu deger register indeksi gibi
// kullaniliyor:
//   SPI_DMA_CONF_REG(1) = REG_SPI_BASE(1) + 0x30 = 0 + 0x30 = 0x30
// DMA tamamlanma kesmesi 0x30 adresine yaziyor ve StoreProhibited ile
// cokuyor. USE_HSPI_PORT ile cokmez ama yanlis cevre biriminin
// registerina yazar, yani sessiz bozulma olur. Ikisi de kullanilamaz.
//
// Kapatmanin maliyeti yok, cunku DMA bu projede gerekmiyor:
//   40 MHz SPI olculen    : 4.40 MB/s
//   USB Full Speed pratik : yaklasik 1.00 MB/s
// Ekran baglantisi USB baglantisindan 4.4 kat hizli. Bloklayan push ile
// bile CPU zamanin yuzde 77'sinde bos. DMA tavani yukseltmez, sadece
// transfer sirasinda CPU'yu serbest birakir, o CPU da zaten bos. Ayrica
// cift cekirdek var: ekran itme bir cekirdekte, USB alimi digerinde.
//
// initDMA cagrilmazsa pushImageDMA bastan donuyor, yani kod guvenli
// sekilde olu. Kutuphane duzelir ya da kendi SPI yolumuzu yazarsak 1 yap.
#define BENCH_ENABLE_DMA  0

#define BENCH_REPS        20
#define BENCH_WARMUP      3
#define FULL_FRAME_BYTES  ((uint32_t)SCREEN_WIDTH * SCREEN_HEIGHT * 2)

static TFT_eSPI tft;

static uint16_t *bufInternal = nullptr;
static uint16_t *bufPsram = nullptr;

struct Region {
  int16_t     w;
  int16_t     h;
  const char *name;
};

static const Region kRegions[] = {
  { SCREEN_WIDTH, SCREEN_HEIGHT, "320x240 tam kare" },
  { SCREEN_WIDTH,            48, "320x48  sayac"    },
  {          160,           120, "160x120 dortte bir" },
  {           80,            80, "80x80   kucuk"    },
};
static const uint8_t kRegionCount = sizeof(kRegions) / sizeof(kRegions[0]);

// Gercekci bir desen: duz renk SPI suresini degistirmez ama olcumun
// dejenere bir veriyle yapilmadigini gormek icin gradyan yaziyoruz.
static void fillPattern(uint16_t *buf, uint32_t pixels)
{
  for (uint32_t i = 0; i < pixels; i++) {
    buf[i] = (uint16_t)((i * 7) ^ (i >> 5));
  }
}

static void printHeader()
{
  logPrintf("\n%-22s %-10s %8s %9s %8s %8s\n",
            "bolge", "yol", "bayt", "us", "MB/s", "kare/s");
  logPrintf("%s\n",
            "--------------------------------------------------------------------------");
}

// us: tek gecisin ortalama suresi. kare/s sutunu, ayni hizla tam kare
// basilsaydi kac kare eder sorusunun cevabi.
static void printRow(const char *region, const char *path,
                     uint32_t bytes, uint32_t us)
{
  const double mbps = (double)bytes / (double)us;            // bayt/us = MB/s
  const double fullFrameUs = (double)FULL_FRAME_BYTES / mbps;
  logPrintf("%-22s %-10s %8lu %9lu %8.2f %8.1f\n",
            region, path, (unsigned long)bytes, (unsigned long)us,
            mbps, 1000000.0 / fullFrameUs);
}

// ---------------------------------------------------------------------------
// Olcum yollari
// ---------------------------------------------------------------------------

static uint32_t timePushImage(uint16_t *buf, const Region &r, bool swap)
{
  tft.setSwapBytes(swap);

  for (uint8_t i = 0; i < BENCH_WARMUP; i++) {
    tft.pushImage(0, 0, r.w, r.h, buf);
  }

  const uint64_t start = esp_timer_get_time();
  for (uint16_t i = 0; i < BENCH_REPS; i++) {
    tft.pushImage(0, 0, r.w, r.h, buf);
  }
  return (uint32_t)((esp_timer_get_time() - start) / BENCH_REPS);
}

#if BENCH_ENABLE_DMA
static uint32_t timePushImageDMA(uint16_t *buf, const Region &r, bool swap)
{
  tft.setSwapBytes(swap);

  tft.startWrite();
  for (uint8_t i = 0; i < BENCH_WARMUP; i++) {
    tft.pushImageDMA(0, 0, r.w, r.h, buf);
    tft.dmaWait();
  }

  const uint64_t start = esp_timer_get_time();
  for (uint16_t i = 0; i < BENCH_REPS; i++) {
    tft.pushImageDMA(0, 0, r.w, r.h, buf);
    tft.dmaWait();
  }
  const uint32_t us = (uint32_t)((esp_timer_get_time() - start) / BENCH_REPS);
  tft.endWrite();

  return us;
}
#endif

#if BENCH_ENABLE_DMA
// DMA gercekten ortusuyor mu: cagrinin dondugu an ile transferin bittigi an
// arasindaki fark, bir sonraki bolgeyi hazirlamak icin elimizdeki butce.
static void measureDmaOverlap(uint16_t *buf)
{
  const Region &r = kRegions[0];

  tft.setSwapBytes(false);
  tft.startWrite();

  uint64_t submitTotal = 0;
  uint64_t completeTotal = 0;

  for (uint16_t i = 0; i < BENCH_REPS; i++) {
    const uint64_t t0 = esp_timer_get_time();
    tft.pushImageDMA(0, 0, r.w, r.h, buf);
    const uint64_t t1 = esp_timer_get_time();
    tft.dmaWait();
    const uint64_t t2 = esp_timer_get_time();

    submitTotal += (t1 - t0);
    completeTotal += (t2 - t0);
  }
  tft.endWrite();

  const uint32_t submitUs = (uint32_t)(submitTotal / BENCH_REPS);
  const uint32_t totalUs = (uint32_t)(completeTotal / BENCH_REPS);

  logPrintf("\nDMA ortusme (tam kare):\n");
  logPrintf("  cagri donus suresi : %lu us\n", (unsigned long)submitUs);
  logPrintf("  transfer tamamlanma: %lu us\n", (unsigned long)totalUs);
  logPrintf("  serbest CPU butcesi: %lu us  (kare basina)\n",
            (unsigned long)(totalUs > submitUs ? totalUs - submitUs : 0));
}
#endif

static void benchBuffer(const char *bufName, uint16_t *buf)
{
  if (buf == nullptr) {
    logPrintf("\n%s: tampon ayrilamadi, atlaniyor\n", bufName);
    return;
  }

  logPrintf("\n### Kaynak tampon: %s\n", bufName);
  printHeader();

  for (uint8_t i = 0; i < kRegionCount; i++) {
    const Region &r = kRegions[i];
    const uint32_t bytes = (uint32_t)r.w * r.h * 2;

    printRow(r.name, "pushImage", bytes, timePushImage(buf, r, false));
#if BENCH_ENABLE_DMA
    printRow(r.name, "DMA",       bytes, timePushImageDMA(buf, r, false));
#endif
  }
}

// ---------------------------------------------------------------------------

static void benchSwapBytes(uint16_t *buf)
{
  if (buf == nullptr) {
    return;
  }

  const Region &r = kRegions[0];
  const uint32_t bytes = (uint32_t)r.w * r.h * 2;

  logPrintf("\n### Bayt sirasi cevirmenin maliyeti (tam kare, dahili SRAM)\n");
  printHeader();
  printRow("swap kapali", "pushImage", bytes, timePushImage(buf, r, false));
  printRow("swap acik",   "pushImage", bytes, timePushImage(buf, r, true));
#if BENCH_ENABLE_DMA
  printRow("swap kapali", "DMA",       bytes, timePushImageDMA(buf, r, false));
  printRow("swap acik",   "DMA",       bytes, timePushImageDMA(buf, r, true));
#endif
  logPrintf("Fark buyukse PC pikselleri onceden dogru sirada gondermeli.\n");
}

static void benchFillScreen()
{
  logPrintf("\n### Taban cizgisi: fillScreen (tamponsuz, en hizli yol)\n");
  printHeader();

  for (uint8_t i = 0; i < BENCH_WARMUP; i++) {
    tft.fillScreen(TFT_BLACK);
  }

  const uint64_t start = esp_timer_get_time();
  for (uint16_t i = 0; i < BENCH_REPS; i++) {
    tft.fillScreen((i & 1) ? TFT_BLACK : TFT_NAVY);
  }
  const uint32_t us = (uint32_t)((esp_timer_get_time() - start) / BENCH_REPS);

  printRow("320x240 tam kare", "fillScreen", FULL_FRAME_BYTES, us);
}

static void allocBuffers()
{
  const size_t bytes = FULL_FRAME_BYTES;

  bufInternal = (uint16_t *)heap_caps_malloc(bytes, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL);
  bufPsram = (uint16_t *)heap_caps_malloc(bytes, MALLOC_CAP_SPIRAM);

  logPrintf("Tampon dahili SRAM : %s (%u bayt)\n",
            bufInternal ? "ayrildi" : "BASARISIZ", (unsigned)bytes);
  logPrintf("Tampon PSRAM       : %s (%u bayt)\n",
            bufPsram ? "ayrildi" : "BASARISIZ", (unsigned)bytes);

  if (bufInternal) {
    fillPattern(bufInternal, bytes / 2);
  }
  if (bufPsram) {
    fillPattern(bufPsram, bytes / 2);
  }
}

void setup()
{
  logBegin();

  logPrintf("\n=== mini_screen bench, Faz 1.1 ===\n");
  logPrintf("SPI hizi   : %u Hz\n", (unsigned)SPI_FREQUENCY);
  logPrintf("Ekran      : %ux%u, donus %u\n",
            (unsigned)SCREEN_WIDTH, (unsigned)SCREEN_HEIGHT,
            (unsigned)DISPLAY_ROTATION);
  logPrintf("Tekrar     : %u olcum, %u isinma\n",
            (unsigned)BENCH_REPS, (unsigned)BENCH_WARMUP);
  logPrintf("Bos heap   : %u bayt, bos PSRAM: %u bayt\n",
            (unsigned)ESP.getFreeHeap(), (unsigned)ESP.getFreePsram());

  backlightBegin();
  backlightSet(BL_BRIGHTNESS_OFF);

  tft.init();
  tft.setRotation(DISPLAY_ROTATION);

#if BENCH_ENABLE_DMA
  const bool dmaOk = tft.initDMA();
  logPrintf("DMA        : %s\n", dmaOk ? "hazir" : "BASLATILAMADI");
#else
  logPrintf("DMA        : kapali (kutuphane hatasi, yukariya bak)\n");
#endif

  backlightSet(BL_BRIGHTNESS_DEFAULT);

  allocBuffers();

  benchFillScreen();
  benchBuffer("dahili SRAM", bufInternal);
  benchBuffer("PSRAM", bufPsram);
  benchSwapBytes(bufInternal);

  if (bufInternal) {
#if BENCH_ENABLE_DMA
    measureDmaOverlap(bufInternal);
#endif
  }

  logPrintf("\n=== bench bitti ===\n");
  logPrintf("Bos heap sonda: %u bayt\n", (unsigned)ESP.getFreeHeap());
}

void loop()
{
  delay(1000);
}
