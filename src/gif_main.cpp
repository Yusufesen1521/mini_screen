// gif_main.cpp - cihazin kendi flashindan GIF oynatir
//
// Iki isi birden goruyor: bagimsiz modun temeli, ve ekran hattinin stres
// testi. Protokol kullanmiyor, veri LittleFS uzerinden geliyor.
//
//   pio run -e gifplay -t uploadfs   ; GIF'leri dosya sistemine yaz
//   pio run -e gifplay -t upload     ; firmware
//
// Dosya sistemindeki butun .gif dosyalari sirayla olculur, sonuclar UART0
// uzerine basilir. Sonra sonuncusu surekli oynatilir.

#include <Arduino.h>
#include <AnimatedGIF.h>
#include <LittleFS.h>
#include <TFT_eSPI.h>
#include <esp_heap_caps.h>
#include <string.h>

#include "backlight.h"
#include "log.h"
#include "pins.h"

static TFT_eSPI tft;
static AnimatedGIF gif;
static File gifFile;

// Olcekleme haritalari. Her kaynak satiri/sutunu hangi hedefe dusuyor,
// dusmuyorsa -1. Ic dongude bolme yapmamak icin onden hesaplaniyor.
static int16_t colMap[GIF_MAX_SRC_DIM];
static int16_t rowMap[GIF_MAX_SRC_DIM];

// Kare tamponu. Cizim buraya yapiliyor, ekrana tek seferde basiliyor.
//
// Onceki surum her saydam olmayan diziyi ayri ayri basiyordu: kare basina
// 2680 setAddrWindow cagrisi, olculen 37.8 ms cizim suresinin yaklasik
// 20 ms'i saf cagri yukuydu. Tampon ile kare basina tek cagri kaliyor.
//
// Saydamlik kendiliginden dogru calisiyor: saydam piksel tampona
// yazilmiyor, onceki karenin degeri oldugu gibi kaliyor.
// canvasBuf cozme hedefi ve kalici: saydam pikseller yazilmadigi icin
// onceki karenin icerigi burada birikiyor.
//
// Basma isi ayri cekirdekte. Cozme 43 ms, basma 27 ms suruyor; ardisikken
// toplam 70 ms, yani kaynagin istedigi 67 ms'nin ustunde ve GIF yavas
// oynuyor. Ayrilinca ikisinin buyugu kaliyor.
//
// Basma gorevi canvas'tan degil ayri bir tampondan okuyor, yoksa bir
// sonraki karenin cozumu basilmakta olan veriyi degistirir. Degisen
// dikdortgen canvas'tan pushBuf'a kopyalaniyor, kopya maliyeti PSRAM
// uzerinde birkac milisaniye.
#define GIF_PUSH_BUF_COUNT  2

struct GifPushJob {
  uint16_t *pixels;
  uint8_t   bufIndex;
  int16_t   x;
  int16_t   y;
  int16_t   w;
  int16_t   h;
};

static uint16_t     *canvasBuf = nullptr;
static uint16_t     *pushBuf[GIF_PUSH_BUF_COUNT] = {nullptr, nullptr};
static QueueHandle_t gifPushQueue = nullptr;
static QueueHandle_t gifFreeBufs = nullptr;

static int16_t dstW = 0;
static int16_t dstH = 0;
static int16_t dstX0 = 0;
static int16_t dstY0 = 0;

// Kare basina degisen bolge. Sadece bu dikdortgen ekrana basiliyor.
static int16_t dirtyX0, dirtyY0, dirtyX1, dirtyY1;

// Olcum
static bool     drawEnabled = true;
static uint32_t frameCount = 0;
static uint32_t frameUsTotal = 0;
static uint32_t frameUsMin = 0xFFFFFFFF;
static uint32_t frameUsMax = 0;
static uint32_t pushedPixels = 0;
static uint32_t lastEnqueueUs = 0;

// ---------------------------------------------------------------------------
// Dosya geri cagirmalari
// ---------------------------------------------------------------------------

static void *gifOpen(const char *name, int32_t *pSize)
{
  gifFile = LittleFS.open(name, "r");
  if (!gifFile) {
    return nullptr;
  }
  *pSize = (int32_t)gifFile.size();
  return (void *)&gifFile;
}

static void gifCloseCb(void *handle)
{
  (void)handle;
  if (gifFile) {
    gifFile.close();
  }
}

static int32_t gifRead(GIFFILE *pFile, uint8_t *pBuf, int32_t len)
{
  File *f = static_cast<File *>(pFile->fHandle);
  const int32_t remain = pFile->iSize - pFile->iPos;
  if (len > remain) {
    len = remain;
  }
  if (len <= 0) {
    return 0;
  }
  const int32_t got = (int32_t)f->read(pBuf, (size_t)len);
  pFile->iPos += got;
  return got;
}

static int32_t gifSeek(GIFFILE *pFile, int32_t pos)
{
  File *f = static_cast<File *>(pFile->fHandle);
  f->seek(pos);
  pFile->iPos = pos;
  return pos;
}

// ---------------------------------------------------------------------------
// Cizim: kare tamponuna yaz, degisen bolgeyi isaretle
// ---------------------------------------------------------------------------

static void gifDraw(GIFDRAW *pDraw)
{
  const int srcY = pDraw->iY + pDraw->y;
  if (srcY < 0 || srcY >= GIF_MAX_SRC_DIM) {
    return;
  }
  const int16_t dstY = rowMap[srcY];
  if (dstY < 0) {
    return;   // bu kaynak satiri olceklemede duruyor
  }

  const uint8_t *src = pDraw->pPixels;
  const uint16_t *pal = pDraw->pPalette;
  const bool hasTrans = pDraw->ucHasTransparency != 0;
  const uint8_t transIdx = pDraw->ucTransparent;
  uint16_t *row = &canvasBuf[(size_t)dstY * dstW];

  bool touched = false;

  for (int i = 0; i < pDraw->iWidth; i++) {
    const int srcX = pDraw->iX + i;
    if (srcX < 0 || srcX >= GIF_MAX_SRC_DIM) {
      continue;
    }
    const int16_t dstX = colMap[srcX];
    if (dstX < 0) {
      continue;
    }

    const uint8_t idx = src[i];
    if (hasTrans && idx == transIdx) {
      continue;   // saydam: onceki kare kalsin
    }

    row[dstX] = pal[idx];
    touched = true;
    if (dstX < dirtyX0) dirtyX0 = dstX;
    if (dstX > dirtyX1) dirtyX1 = dstX;
  }

  if (touched) {
    if (dstY < dirtyY0) dirtyY0 = dstY;
    if (dstY > dirtyY1) dirtyY1 = dstY;
  }
}

// ---------------------------------------------------------------------------
// Olcekleme
// ---------------------------------------------------------------------------

// En boy orani korunarak ekrana sigdirir. Ayni hedef indekse dusen ilk
// kaynak indeksi alinir, otekiler -1 isaretlenir.
static void buildMaps(int srcW, int srcH)
{
  dstW = (int16_t)min((int)SCREEN_WIDTH, srcW * SCREEN_HEIGHT / srcH);
  dstH = (int16_t)(dstW * srcH / srcW);
  dstX0 = (int16_t)((SCREEN_WIDTH - dstW) / 2);
  dstY0 = (int16_t)((SCREEN_HEIGHT - dstH) / 2);

  for (int i = 0; i < GIF_MAX_SRC_DIM; i++) {
    colMap[i] = -1;
    rowMap[i] = -1;
  }
  for (int x = 0; x < srcW && x < GIF_MAX_SRC_DIM; x++) {
    const int d = x * dstW / srcW;
    if (x == 0 || (x - 1) * dstW / srcW != d) {
      colMap[x] = (int16_t)d;
    }
  }
  for (int y = 0; y < srcH && y < GIF_MAX_SRC_DIM; y++) {
    const int d = y * dstH / srcH;
    if (y == 0 || (y - 1) * dstH / srcH != d) {
      rowMap[y] = (int16_t)d;
    }
  }
}

// ---------------------------------------------------------------------------
// Oynatma
// ---------------------------------------------------------------------------

static void gifPushTask(void *arg)
{
  (void)arg;
  GifPushJob job;

  for (;;) {
    if (xQueueReceive(gifPushQueue, &job, portMAX_DELAY) != pdTRUE) {
      continue;
    }
    tft.startWrite();
    tft.setAddrWindow(job.x, job.y, job.w, job.h);
    tft.pushPixels(job.pixels, (uint32_t)job.w * job.h);
    tft.endWrite();
    xQueueSend(gifFreeBufs, &job.bufIndex, portMAX_DELAY);
  }
}

// Hazirlanmis ama henuz gonderilmemis is. Gonderim ani ekranin
// guncellendigi an demek, o yuzden zamanlama buna gore yapiliyor.
static GifPushJob pendingJob;
static bool       pendingValid = false;

// Degisen dikdortgeni bos bir tampona kopyalar. Gondermez.
static void prepareDirty()
{
  pendingValid = false;

  if (dirtyY1 < dirtyY0 || dirtyX1 < dirtyX0) {
    return;   // bu karede degisen yok
  }

  const int16_t w = dirtyX1 - dirtyX0 + 1;
  const int16_t h = dirtyY1 - dirtyY0 + 1;
  pushedPixels += (uint32_t)w * h;

  if (!drawEnabled) {
    return;
  }

  uint8_t bufIndex = 0;
  if (xQueueReceive(gifFreeBufs, &bufIndex, pdMS_TO_TICKS(1000)) != pdTRUE) {
    return;   // basma gorevi takildi, kareyi atla
  }

  uint16_t *dst = pushBuf[bufIndex];
  for (int16_t y = 0; y < h; y++) {
    memcpy(&dst[(size_t)y * w],
           &canvasBuf[(size_t)(dirtyY0 + y) * dstW + dirtyX0],
           (size_t)w * 2);
  }

  pendingJob = GifPushJob{
    dst, bufIndex,
    (int16_t)(dstX0 + dirtyX0), (int16_t)(dstY0 + dirtyY0), w, h,
  };
  pendingValid = true;
}

// Hazir isi basma gorevine verir. Ekran bu anda guncelleniyor.
static void submitDirty()
{
  if (!pendingValid) {
    return;
  }
  lastEnqueueUs = micros();
  xQueueSend(gifPushQueue, &pendingJob, portMAX_DELAY);
  pendingValid = false;
}

// Olcum yollari icin: hazirla ve hemen gonder.
static void pushDirty()
{
  prepareDirty();
  submitDirty();
}

// Bekleyen butun basma islerinin bitmesini bekler. Olcum sonunda gerekli.
static void pushDrain()
{
  for (uint8_t i = 0; i < GIF_PUSH_BUF_COUNT; i++) {
    uint8_t idx;
    xQueueReceive(gifFreeBufs, &idx, portMAX_DELAY);
  }
  for (uint8_t i = 0; i < GIF_PUSH_BUF_COUNT; i++) {
    xQueueSend(gifFreeBufs, &i, portMAX_DELAY);
  }
}

static void resetDirty()
{
  dirtyX0 = dstW;
  dirtyY0 = dstH;
  dirtyX1 = -1;
  dirtyY1 = -1;
}

// bSync true ise GIF kendi zamanlamasina uyar, false ise olabildigince
// hizli oynar.
static void playLoops(uint8_t loops, bool sync, const char *label)
{
  frameCount = 0;
  frameUsTotal = 0;
  frameUsMin = 0xFFFFFFFF;
  frameUsMax = 0;
  pushedPixels = 0;

  const uint32_t wallStart = millis();

  for (uint8_t loop = 0; loop < loops; loop++) {
    int result = 1;
    while (result > 0) {
      int delayMs = 0;
      const uint32_t t0 = micros();

      resetDirty();
      result = gif.playFrame(sync, &delayMs);
      pushDirty();

      const uint32_t us = micros() - t0;
      if (result >= 0) {
        frameCount++;
        frameUsTotal += us;
        if (us < frameUsMin) frameUsMin = us;
        if (us > frameUsMax) frameUsMax = us;
      }
    }
    gif.reset();
  }
  if (drawEnabled) {
    pushDrain();
  }

  const uint32_t wallMs = millis() - wallStart;
  if (frameCount == 0) {
    logPrintf("  %-12s kare yok, hata %d\n", label, gif.getLastError());
    return;
  }

  logPrintf("  %-12s %5.1f FPS  ort %lu us (min %lu max %lu)  %lu px/kare\n",
            label, 1000.0 * frameCount / (double)wallMs,
            (unsigned long)(frameUsTotal / frameCount),
            (unsigned long)frameUsMin, (unsigned long)frameUsMax,
            (unsigned long)(pushedPixels / frameCount));
}

static bool openAndMeasure(const char *path)
{
  if (!gif.open(path, gifOpen, gifCloseCb, gifRead, gifSeek, gifDraw)) {
    logPrintf("%s: acilamadi, kod %d\n", path, gif.getLastError());
    return false;
  }

  const int srcW = gif.getCanvasWidth();
  const int srcH = gif.getCanvasHeight();
  if (srcW > GIF_MAX_SRC_DIM || srcH > GIF_MAX_SRC_DIM) {
    logPrintf("%s: %dx%d, sinir %d asildi\n", path, srcW, srcH,
              (int)GIF_MAX_SRC_DIM);
    gif.close();
    return false;
  }

  buildMaps(srcW, srcH);
  tft.fillScreen(COLOR_BACKGROUND);
  memset(canvasBuf, 0, (size_t)dstW * dstH * 2);

  logPrintf("\n%s  %dx%d -> %dx%d\n", path, srcW, srcH, (int)dstW, (int)dstH);

  drawEnabled = false;
  playLoops(GIF_MEASURE_LOOPS, false, "sadece cozme");
  drawEnabled = true;
  playLoops(GIF_MEASURE_LOOPS, false, "cozme+ciz");

  return true;
}

void setup()
{
  logBegin();
  logPrintf("\n=== mini_screen GIF oynatici ===\n");

  backlightBegin();
  backlightSet(BL_BRIGHTNESS_OFF);

  tft.init();
  tft.setRotation(DISPLAY_ROTATION);
  tft.fillScreen(COLOR_BACKGROUND);
  tft.setSwapBytes(true);   // palet little-endian RGB565, panel big-endian
  backlightSet(BL_BRIGHTNESS_DEFAULT);

  if (!LittleFS.begin(false)) {
    logPrintf("HATA: LittleFS baglanamadi. once 'pio run -e gifplay -t uploadfs'\n");
    return;
  }
  logPrintf("LittleFS   : %u / %u bayt\n",
            (unsigned)LittleFS.usedBytes(), (unsigned)LittleFS.totalBytes());

  const size_t bufBytes = (size_t)SCREEN_WIDTH * SCREEN_HEIGHT * 2;
  canvasBuf = (uint16_t *)heap_caps_malloc(bufBytes, MALLOC_CAP_SPIRAM);
  bool bufsOk = (canvasBuf != nullptr);
  for (uint8_t i = 0; i < GIF_PUSH_BUF_COUNT; i++) {
    pushBuf[i] = (uint16_t *)heap_caps_malloc(bufBytes, MALLOC_CAP_SPIRAM);
    bufsOk = bufsOk && (pushBuf[i] != nullptr);
  }

  gifPushQueue = xQueueCreate(GIF_PUSH_BUF_COUNT, sizeof(GifPushJob));
  gifFreeBufs = xQueueCreate(GIF_PUSH_BUF_COUNT, sizeof(uint8_t));
  bufsOk = bufsOk && (gifPushQueue != nullptr) && (gifFreeBufs != nullptr);

  if (!bufsOk) {
    logPrintf("HATA: tamponlar ayrilamadi\n");
    return;
  }
  for (uint8_t i = 0; i < GIF_PUSH_BUF_COUNT; i++) {
    xQueueSend(gifFreeBufs, &i, 0);
  }

  xTaskCreatePinnedToCore(gifPushTask, "gifpush", PUSH_TASK_STACK, nullptr,
                          PUSH_TASK_PRIORITY, nullptr, PUSH_TASK_CORE);

  gif.begin(GIF_PALETTE_RGB565_LE);

  char lastGif[64] = {0};
  File root = LittleFS.open("/");
  for (File f = root.openNextFile(); f; f = root.openNextFile()) {
    const char *name = f.name();
    const size_t len = strlen(name);
    if (len < 4 || strcasecmp(&name[len - 4], ".gif") != 0) {
      continue;
    }
    char path[64];
    snprintf(path, sizeof(path), "%s%s", (name[0] == '/') ? "" : "/", name);
    f.close();
    if (openAndMeasure(path)) {
      strncpy(lastGif, path, sizeof(lastGif) - 1);
      gif.close();
    }
  }

  if (lastGif[0] == '\0') {
    logPrintf("\nDosya sisteminde GIF bulunamadi.\n");
    return;
  }

  logPrintf("\nSurekli oynatiliyor: %s\n", lastGif);
  buildMaps(gif.getCanvasWidth(), gif.getCanvasHeight());
  gif.open(lastGif, gifOpen, gifCloseCb, gifRead, gifSeek, gifDraw);
  buildMaps(gif.getCanvasWidth(), gif.getCanvasHeight());
  tft.fillScreen(COLOR_BACKGROUND);
  memset(canvasBuf, 0, (size_t)dstW * dstH * 2);
}

void loop()
{
  if (canvasBuf == nullptr) {
    delay(1000);
    return;
  }

  static uint32_t nextFrameUs = 0;

  int delayMs = 0;
  resetDirty();
  const int result = gif.playFrame(false, &delayMs);
  prepareDirty();

  // Zamanlama gonderimden ONCE yapiliyor.
  //
  // Onceki surumde bekleme gonderimden sonraydi, yani cozme suresi ne
  // kadar degisirse ekranin guncellendigi an da o kadar kayiyordu.
  // Olculen aralik 48.8 ile 92.2 ms arasinda oynuyordu; ortalama dogruydu
  // ama goz bunu titreme olarak goruyor. Simdi ekran hep planlanan anda
  // guncelleniyor, cozmenin ne kadar surdugu onemli degil.
  //
  // Kutuphanenin bSync secenegi de kullanilamaz: o yalnizca playFrame
  // icindeki sureyi olcuyor, basma disarida kaliyor.
  if (nextFrameUs == 0) {
    nextFrameUs = micros();
  }
  int32_t wait = (int32_t)(nextFrameUs - micros());
  while (wait > GIF_YIELD_US) {
    vTaskDelay(1);                 // mesgul beklemek yerine sirayi birak
    wait = (int32_t)(nextFrameUs - micros());
  }
  if (wait > 0) {
    delayMicroseconds((uint32_t)wait);
  }

  submitDirty();

  nextFrameUs += (uint32_t)delayMs * 1000u;
  if ((int32_t)(micros() - nextFrameUs) > GIF_RESYNC_US) {
    nextFrameUs = micros();        // cok geride kaldik, birikim sifirlansin
  }

  if (result <= 0) {
    gif.reset();
  }

  // Oynatim hizi ve duzensizligi. Titreme sikayeti hiz sorunundan degil
  // ekranin duzensiz araliklarla guncellenmesinden olabilir, o yuzden
  // araligin kendisi de olculuyor.
  static uint32_t loopFrames = 0;
  static uint32_t loopStartMs = 0;
  static uint32_t lastShowUs = 0;
  static uint32_t gapMin = 0xFFFFFFFF;
  static uint32_t gapMax = 0;
  static uint64_t gapSum = 0;

  const uint32_t nowUs = lastEnqueueUs;   // ekranin guncellendigi an
  if (lastShowUs != 0 && nowUs != lastShowUs) {
    const uint32_t gap = nowUs - lastShowUs;
    if (gap < gapMin) gapMin = gap;
    if (gap > gapMax) gapMax = gap;
    gapSum += gap;
  }
  lastShowUs = nowUs;

  if (loopStartMs == 0) {
    loopStartMs = millis();
  }
  loopFrames++;
  const uint32_t elapsed = millis() - loopStartMs;
  if (elapsed >= GIF_REPORT_MS && loopFrames > 1) {
    logPrintf("oynatim %5.2f FPS  aralik ort %lu us (min %lu max %lu, "
              "oynama %lu us)\n",
              1000.0 * loopFrames / (double)elapsed,
              (unsigned long)(gapSum / (loopFrames - 1)),
              (unsigned long)gapMin, (unsigned long)gapMax,
              (unsigned long)(gapMax - gapMin));
    loopFrames = 0;
    loopStartMs = millis();
    gapMin = 0xFFFFFFFF;
    gapMax = 0;
    gapSum = 0;
  }
}
