// mini_screen - USB protokol alicisi
//
// Bagli modda PC ciziyor, cihaz sadece basiyor. Bu firmware USB CDC
// uzerinden gelen piksel bolgelerini ekrana yaziyor.
//
// Protokolun bayt duzeyinde tanimi: docs/protocol.md
//
// Port dagilimi:
//   Serial  (yerlesik USB, GPIO 19/20) -> protokol
//   Serial0 (UART0, TX/RX pinleri)     -> log ve yukleme
// Ikisi de takili olmali.

#include <Arduino.h>
#include <TFT_eSPI.h>
#include <esp_heap_caps.h>
#include <string.h>

#include "backlight.h"
#include "crc.h"
#include "framer.h"
#include "log.h"
#include "pins.h"
#include "protocol.h"
#include "rle16.h"

#define RX_CHUNK_SIZE     1024
#define PAYLOAD_CAP       PROTO_MAX_PAYLOAD
#define DECODE_PIXELS     ((size_t)SCREEN_WIDTH * SCREEN_HEIGHT)

static TFT_eSPI tft;
static Framer framer;

static uint8_t  *payloadBuf = nullptr;   // gelen cerceve payload'i
static uint8_t   rxChunk[RX_CHUNK_SIZE];

// Ekrana basma isi ayri bir cekirdekte.
//
// Tek gorevliyken okuma ve basma ardisik oluyordu ve toplam sure ikisinin
// toplamiydi. Olcum: arayuz iceriginde 36 ms okuma + 35 ms basma = 71 ms,
// yani 14 FPS. Ayrilinca limit ikisinin buyugu oluyor.
//
// Iki cozme tamponu var: biri ekrana basilirken digerine cozuluyor.
// CODEC_NONE de kopyalaniyor, cunku payload tamponu bir sonraki cerceve
// tarafindan ezilir.
#define DECODE_BUF_COUNT  2

struct PushJob {
  uint16_t *pixels;
  uint8_t   bufIndex;
  int32_t   x;
  int32_t   y;
  int32_t   w;
  int32_t   h;
  uint8_t   seq;
  bool      ackReq;
};

static uint16_t     *decodeBuf[DECODE_BUF_COUNT] = {nullptr, nullptr};
static QueueHandle_t pushQueue = nullptr;   // PushJob
static QueueHandle_t freeBufs = nullptr;    // uint8_t, bos tampon indeksi

static bool     connected = false;
static bool     selfTestOk = false;
static uint32_t lastRegionUs = 0;

// ---------------------------------------------------------------------------
// Cerceve gonderme
// ---------------------------------------------------------------------------

// Gonderim kilidi. Iki cekirdek de cerceve gonderiyor: loop() NACK, CAPS,
// STATUS ve LOG; pushTask ise ACK. Kilit olmadan iki cerceve birbirinin
// icine giriyor ve karsi taraf senkron kaybediyor. Olculen sonucu: arada
// bir ACK kayboluyordu ve PC 2 saniyelik zaman asimina takiliyordu.
static SemaphoreHandle_t txLock = nullptr;

// Cerceve tek parca halinde gonderilsin diye toplama tamponu. Cihazdan
// PC'ye giden en buyuk mesaj LOG, o da acilis tamponu kadar.
static uint8_t txAssemble[LOG_BOOT_BUF_SIZE + PROTO_HDR_SIZE + PROTO_CRC_SIZE];

static void sendFrame(uint8_t type, uint8_t seq, const uint8_t *payload, uint16_t len)
{
  uint8_t hdr[PROTO_HDR_SIZE];

  hdr[PROTO_OFF_SOF0]  = PROTO_SOF0;
  hdr[PROTO_OFF_SOF1]  = PROTO_SOF1;
  hdr[PROTO_OFF_VER]   = PROTO_VERSION;
  hdr[PROTO_OFF_TYPE]  = type;
  hdr[PROTO_OFF_FLAGS] = 0;
  hdr[PROTO_OFF_SEQ]   = seq;
  hdr[PROTO_OFF_LEN]   = (uint8_t)(len & 0xFF);
  hdr[PROTO_OFF_LEN + 1] = (uint8_t)(len >> 8);
  hdr[PROTO_OFF_RSV]   = 0;
  hdr[PROTO_OFF_HDRCRC] = crc8(&hdr[PROTO_HDRCRC_START], PROTO_HDRCRC_LEN);

  const uint16_t c = crc16(payload, len);
  const uint8_t tail[PROTO_CRC_SIZE] = { (uint8_t)(c & 0xFF), (uint8_t)(c >> 8) };

  if (txLock != nullptr) {
    xSemaphoreTake(txLock, portMAX_DELAY);
  }

  const size_t total = PROTO_HDR_SIZE + len + PROTO_CRC_SIZE;
  if (total <= sizeof(txAssemble)) {
    memcpy(txAssemble, hdr, PROTO_HDR_SIZE);
    if (len > 0) {
      memcpy(&txAssemble[PROTO_HDR_SIZE], payload, len);
    }
    memcpy(&txAssemble[PROTO_HDR_SIZE + len], tail, PROTO_CRC_SIZE);
    Serial.write(txAssemble, total);
  } else {
    // Tamponu asan boy yok ama olursa kilit altinda parcali gonder
    Serial.write(hdr, sizeof(hdr));
    Serial.write(payload, len);
    Serial.write(tail, sizeof(tail));
  }

  if (txLock != nullptr) {
    xSemaphoreGive(txLock);
  }
}

// Log satirlarini protokol uzerinden gonderir. Boylece UART hatti bagli
// olmadan, tek kabloyla da tanilama gorunur.
static void logToLink(const char *text)
{
  const size_t len = strlen(text);
  if (len > 0 && len <= PAYLOAD_CAP) {
    sendFrame(MSG_LOG, 0, (const uint8_t *)text, (uint16_t)len);
  }
}

static void sendNack(uint8_t reason, uint8_t seq)
{
  const uint8_t payload[2] = { reason, seq };
  sendFrame(MSG_NACK, seq, payload, sizeof(payload));
}

static void sendCaps(uint8_t seq)
{
  uint8_t p[CAPS_PAYLOAD_SIZE] = {0};

  p[0] = PROTO_VERSION;
  p[1] = 0;  // fw major
  p[2] = 1;  // fw minor
  p[3] = 0;  // fw patch
  p[4] = (uint8_t)(SCREEN_WIDTH & 0xFF);
  p[5] = (uint8_t)(SCREEN_WIDTH >> 8);
  p[6] = (uint8_t)(SCREEN_HEIGHT & 0xFF);
  p[7] = (uint8_t)(SCREEN_HEIGHT >> 8);
  p[8] = PIXFMT_RGB565_LE;
  p[9] = CODEC_MASK_NONE | CODEC_MASK_RLE16;
  p[10] = (uint8_t)(PAYLOAD_CAP & 0xFF);
  p[11] = (uint8_t)(PAYLOAD_CAP >> 8);
  p[12] = PROTO_RX_SLOTS;
  p[CAPS_OFF_SELFTEST] = selfTestOk ? 1 : 0;
  esp_read_mac(&p[14], ESP_MAC_WIFI_STA);

  sendFrame(MSG_CAPS, seq, p, sizeof(p));
}

static void sendStatus(uint8_t seq)
{
  const Framer::Stats &s = framer.stats();
  uint8_t p[STATUS_PAYLOAD_SIZE] = {0};

  memcpy(&p[0], &s.framesOk, 4);
  memcpy(&p[4], &s.framesDropped, 4);
  memcpy(&p[8], &s.hdrCrcErrors, 2);
  memcpy(&p[10], &s.payloadCrcErrors, 2);
  memcpy(&p[12], &s.syncLosses, 2);

  const uint16_t us = (lastRegionUs > 0xFFFF) ? 0xFFFF : (uint16_t)lastRegionUs;
  memcpy(&p[14], &us, 2);

  sendFrame(MSG_STATUS, seq, p, sizeof(p));
}

// ---------------------------------------------------------------------------
// FRAME_REGION isleme
// ---------------------------------------------------------------------------

// Ekrana basma gorevi. Kendi cekirdeginde calisir, kuyruktan is alir,
// basar ve ACK'i kendisi gonderir. ACK basma bittikten sonra gittigi icin
// PC penceresi cihazin gercek islem hizina baglanmis oluyor.
static void pushTask(void *arg)
{
  (void)arg;
  PushJob job;

  for (;;) {
    if (xQueueReceive(pushQueue, &job, portMAX_DELAY) != pdTRUE) {
      continue;
    }

    const uint32_t t0 = micros();
    // Payload little-endian RGB565, panel big-endian bekliyor. Cevrimi
    // kutuphane yapiyor; olculen maliyeti yuzde 0.2.
    tft.setSwapBytes(true);
    tft.pushImage(job.x, job.y, job.w, job.h, job.pixels);
    lastRegionUs = micros() - t0;

    if (job.ackReq) {
      const uint8_t p[1] = { job.seq };
      sendFrame(MSG_ACK, job.seq, p, sizeof(p));
    }

    xQueueSend(freeBufs, &job.bufIndex, portMAX_DELAY);

    // Sirayi birak. Kuyruk surekli doluyken bu gorev hic bloklanmaz ve
    // ayni cekirdekteki IDLE gorevi calisamaz; task watchdog sistemi
    // resetler. GIF oynaticida tam olarak bu yasandi.
    vTaskDelay(1);
  }
}

static bool handleRegion(const uint8_t *payload, uint16_t len, uint8_t seq, bool ackReq)
{
  if (len < REGION_HDR_SIZE) {
    sendNack(ERR_BAD_REGION, seq);
    return false;
  }

  const uint16_t x = (uint16_t)(payload[REGION_OFF_X] | (payload[REGION_OFF_X + 1] << 8));
  const uint16_t y = (uint16_t)(payload[REGION_OFF_Y] | (payload[REGION_OFF_Y + 1] << 8));
  const uint16_t w = (uint16_t)(payload[REGION_OFF_W] | (payload[REGION_OFF_W + 1] << 8));
  const uint16_t h = (uint16_t)(payload[REGION_OFF_H] | (payload[REGION_OFF_H + 1] << 8));
  const uint8_t format = payload[REGION_OFF_FORMAT];
  const uint8_t codec = payload[REGION_OFF_CODEC];

  // Kirpma yapilmiyor: sessiz kirpma PC tarafindaki hatayi gizler.
  if (w == 0 || h == 0 ||
      (uint32_t)x + w > SCREEN_WIDTH || (uint32_t)y + h > SCREEN_HEIGHT) {
    sendNack(ERR_BAD_REGION, seq);
    return false;
  }
  if (format != PIXFMT_RGB565_LE) {
    sendNack(ERR_BAD_CODEC, seq);
    return false;
  }

  const uint8_t *data = &payload[REGION_HDR_SIZE];
  const uint16_t dataLen = (uint16_t)(len - REGION_HDR_SIZE);
  const size_t pixels = (size_t)w * h;

  // Bos bir cozme tamponu bekle. Akis kontrolu sayesinde normalde hemen
  // bulunur; bulunamazsa PC pencereyi asmis demektir.
  uint8_t bufIndex = 0;
  if (xQueueReceive(freeBufs, &bufIndex, pdMS_TO_TICKS(PUSH_WAIT_MS)) != pdTRUE) {
    sendNack(ERR_OVERRUN, seq);
    return false;
  }

  uint16_t *dst = decodeBuf[bufIndex];

  if (codec == CODEC_NONE) {
    if (dataLen != pixels * 2) {
      xQueueSend(freeBufs, &bufIndex, 0);
      sendNack(ERR_BAD_REGION, seq);
      return false;
    }
    memcpy(dst, data, dataLen);
  } else if (codec == CODEC_RLE16) {
    const int32_t decoded = rle16Decode(data, dataLen, dst, DECODE_PIXELS);
    if (decoded < 0 || (size_t)decoded != pixels) {
      xQueueSend(freeBufs, &bufIndex, 0);
      sendNack(ERR_DECODE_ERROR, seq);
      return false;
    }
  } else {
    xQueueSend(freeBufs, &bufIndex, 0);
    sendNack(ERR_BAD_CODEC, seq);
    return false;
  }

  PushJob job = {
    dst, bufIndex,
    (int32_t)x, (int32_t)y, (int32_t)w, (int32_t)h,
    seq, ackReq,
  };
  xQueueSend(pushQueue, &job, portMAX_DELAY);

  return true;
}

// ---------------------------------------------------------------------------
// Cerceve dagitimi
// ---------------------------------------------------------------------------

static void onFrame(uint8_t type, uint8_t flags, uint8_t seq,
                    const uint8_t *payload, uint16_t len, void *ctx)
{
  (void)ctx;
  bool ok = true;

  switch (type) {
    case MSG_HELLO:
      if (!connected) {
        connected = true;
        tft.fillScreen(COLOR_BACKGROUND);
        logPrintf("PC baglandi, protokol modu\n");
      }
      sendCaps(seq);
      // CAPS gittikten sonra kur: biriken acilis loglari CAPS'in arkasindan
      // gelsin, PC once cihazi tanisin.
      logSetSink(logToLink);
      return;   // CAPS zaten cevap, ayrica ACK gonderme

    case MSG_FRAME_REGION:
      // ACK'i pushTask gonderiyor, ekrana basma bittikten sonra.
      handleRegion(payload, len, seq, (flags & PROTO_FLAG_ACK_REQ) != 0);
      return;

    case MSG_SET_BACKLIGHT:
      if (len != 1) {
        sendNack(ERR_BAD_REGION, seq);
        return;
      }
      backlightSet(payload[0]);
      break;

    case MSG_PING:
      sendFrame(MSG_PONG, seq, nullptr, 0);
      return;

    case MSG_GET_STATUS:
      sendStatus(seq);
      // Bellek sizintisi takibi icin. STATUS duzenini degistirmemek adina
      // ayri bir LOG satiri olarak gidiyor.
      logPrintf("heap %u  psram %u\n",
                (unsigned)ESP.getFreeHeap(), (unsigned)ESP.getFreePsram());
      return;

    default:
      sendNack(ERR_UNKNOWN_TYPE, seq);
      return;
  }

  if (ok && (flags & PROTO_FLAG_ACK_REQ)) {
    const uint8_t p[1] = { seq };
    sendFrame(MSG_ACK, seq, p, sizeof(p));
  }
}

static void onFramerError(uint8_t reason, uint8_t seq, void *ctx)
{
  (void)ctx;
  sendNack(reason, seq);
}

// ---------------------------------------------------------------------------
// Acilis ekrani
// ---------------------------------------------------------------------------

struct ColorBlock {
  uint16_t    color;
  uint16_t    labelColor;
  const char *label;
};

// Etiketler blogun uzerine yaziliyor: renk sirasi ya da RGB/BGR ayari
// yanlissa "RED" yazisi kirmizi olmayan bir blogun uzerinde kalir.
static const ColorBlock kColorBlocks[BLOCK_COUNT] = {
  { COLOR_BLOCK_RED,   COLOR_TEXT,       "RED"   },
  { COLOR_BLOCK_GREEN, COLOR_BACKGROUND, "GREEN" },
  { COLOR_BLOCK_BLUE,  COLOR_TEXT,       "BLUE"  },
  { COLOR_BLOCK_WHITE, COLOR_BACKGROUND, "WHITE" },
};

static void drawSplash()
{
  tft.fillScreen(COLOR_BACKGROUND);

  tft.setTextDatum(TC_DATUM);
  tft.setTextColor(COLOR_TEXT, COLOR_BACKGROUND);
  tft.drawString(TITLE_TEXT, SCREEN_WIDTH / 2, TITLE_Y, FONT_TITLE);
  tft.drawFastHLine(0, TITLE_AREA_HEIGHT - 1, SCREEN_WIDTH, COLOR_SEPARATOR);

  tft.setTextDatum(MC_DATUM);
  for (uint8_t i = 0; i < BLOCK_COUNT; i++) {
    const int16_t x = BLOCK_FIRST_X + i * BLOCK_WIDTH;
    tft.fillRect(x, BLOCK_Y, BLOCK_WIDTH, BLOCK_HEIGHT, kColorBlocks[i].color);
    tft.setTextColor(kColorBlocks[i].labelColor, kColorBlocks[i].color);
    tft.drawString(kColorBlocks[i].label, x + BLOCK_WIDTH / 2,
                   BLOCK_Y + BLOCK_HEIGHT / 2, FONT_LABEL);
  }

  tft.setTextDatum(TL_DATUM);
  tft.setTextColor(COLOR_DIM_TEXT, COLOR_BACKGROUND);
  tft.drawString(SPLASH_TEXT, SPLASH_X, SPLASH_Y, FONT_LABEL);

  // Kose isaretleri, offset kontrolu icin
  tft.drawPixel(0, 0, COLOR_CORNER_MARK);
  tft.drawPixel(SCREEN_WIDTH - 1, 0, COLOR_CORNER_MARK);
  tft.drawPixel(0, SCREEN_HEIGHT - 1, COLOR_CORNER_MARK);
  tft.drawPixel(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1, COLOR_CORNER_MARK);
}

// ---------------------------------------------------------------------------
// Kendini sinama
//
// Ayni test vektorleri PC tarafindaki araca da gomulu. Iki taraf ayni
// sonucu uretmezse protokol daha ilk cerceve de tutmaz.
// ---------------------------------------------------------------------------

static bool selfTest()
{
  bool ok = true;
  const uint8_t vector[] = "123456789";
  const size_t vectorLen = sizeof(vector) - 1;

  const uint8_t c8 = crc8(vector, vectorLen);
  const uint16_t c16 = crc16(vector, vectorLen);
  logPrintf("  crc8  (123456789) = 0x%02X  %s\n", c8, (c8 == 0xF4) ? "TAMAM" : "HATA");
  logPrintf("  crc16 (123456789) = 0x%04X  %s\n", c16, (c16 == 0x29B1) ? "TAMAM" : "HATA");
  ok &= (c8 == 0xF4) && (c16 == 0x29B1);

  uint16_t out[16];

  // Tekrar kosusu: 8 piksel 0xF800
  const uint8_t repeat[] = { 0x86, 0x00, 0xF8 };
  int32_t n = rle16Decode(repeat, sizeof(repeat), out, 16);
  const bool repeatOk = (n == 8) && (out[0] == 0xF800) && (out[7] == 0xF800);
  logPrintf("  rle16 tekrar kosusu       %s\n", repeatOk ? "TAMAM" : "HATA");
  ok &= repeatOk;

  // Duz kosu: 2 piksel
  const uint8_t literal[] = { 0x01, 0x11, 0x22, 0x33, 0x44 };
  n = rle16Decode(literal, sizeof(literal), out, 16);
  const bool literalOk = (n == 2) && (out[0] == 0x2211) && (out[1] == 0x4433);
  logPrintf("  rle16 duz kosu            %s\n", literalOk ? "TAMAM" : "HATA");
  ok &= literalOk;

  // Kaynak erken bitti
  const uint8_t truncated[] = { 0x86, 0x00 };
  const bool truncOk = (rle16Decode(truncated, sizeof(truncated), out, 16) == -1);
  logPrintf("  rle16 eksik kaynak        %s\n", truncOk ? "TAMAM" : "HATA");
  ok &= truncOk;

  // Hedef tasacakti
  const bool overflowOk = (rle16Decode(repeat, sizeof(repeat), out, 4) == -2);
  logPrintf("  rle16 hedef tasmasi       %s\n", overflowOk ? "TAMAM" : "HATA");
  ok &= overflowOk;

  return ok;
}

// ---------------------------------------------------------------------------

void setup()
{
  txLock = xSemaphoreCreateMutex();
  logBegin();
  crcBegin();

  logPrintf("\n=== mini_screen, protokol surumu %u ===\n", (unsigned)PROTO_VERSION);
  logPrintf("Chip      : %s rev %u, %u core\n",
            ESP.getChipModel(), ESP.getChipRevision(), ESP.getChipCores());
  logPrintf("Flash     : %u bayt\n", (unsigned)ESP.getFlashChipSize());
  logPrintf("PSRAM     : %s, %u bayt\n",
            psramFound() ? "bulundu" : "BULUNAMADI", (unsigned)ESP.getPsramSize());

  logPrintf("Kendini sinama:\n");
  selfTestOk = selfTest();
  logPrintf(selfTestOk ? "  sonuc: TAMAM\n" : "  sonuc: HATA, protokol guvenilmez\n");

  backlightBegin();
  backlightHeartbeat();
  backlightSet(BL_BRIGHTNESS_OFF);

  tft.init();
  tft.setRotation(DISPLAY_ROTATION);

  // Tamponlar PSRAM'de. Olcum dahili SRAM ile arasindaki farkin yuzde 2
  // oldugunu gosterdi, SRAM daha degerli bir kaynak.
  payloadBuf = (uint8_t *)heap_caps_malloc(PAYLOAD_CAP, MALLOC_CAP_SPIRAM);

  bool buffersOk = (payloadBuf != nullptr);
  for (uint8_t i = 0; i < DECODE_BUF_COUNT; i++) {
    decodeBuf[i] = (uint16_t *)heap_caps_malloc(DECODE_PIXELS * 2, MALLOC_CAP_SPIRAM);
    buffersOk = buffersOk && (decodeBuf[i] != nullptr);
  }

  pushQueue = xQueueCreate(DECODE_BUF_COUNT, sizeof(PushJob));
  freeBufs = xQueueCreate(DECODE_BUF_COUNT, sizeof(uint8_t));
  buffersOk = buffersOk && (pushQueue != nullptr) && (freeBufs != nullptr);

  if (buffersOk) {
    for (uint8_t i = 0; i < DECODE_BUF_COUNT; i++) {
      xQueueSend(freeBufs, &i, 0);
    }
  }

  if (!buffersOk) {
    logPrintf("HATA: tamponlar ayrilamadi\n");
    while (true) {
      delay(1000);
    }
  }
  logPrintf("Tamponlar : payload %u bayt, cozme %u x %u bayt (PSRAM)\n",
            (unsigned)PAYLOAD_CAP, (unsigned)DECODE_BUF_COUNT,
            (unsigned)(DECODE_PIXELS * 2));

  // Ekrana basma ayri cekirdekte. Arduino loop'u 1. cekirdekte calisiyor,
  // basma isi 0. cekirdege veriliyor; boylece okuma ve basma ortusuyor.
  xTaskCreatePinnedToCore(pushTask, "push", PUSH_TASK_STACK, nullptr,
                          PUSH_TASK_PRIORITY, nullptr, PUSH_TASK_CORE);
  logPrintf("Basma gorevi: cekirdek %d, pencere %d cerceve\n",
            (int)PUSH_TASK_CORE, (int)PROTO_RX_SLOTS);

  framer.begin(payloadBuf, PAYLOAD_CAP, onFrame, onFramerError, nullptr);

  drawSplash();
  backlightSet(BL_BRIGHTNESS_DEFAULT);

  // HWCDC varsayilan RX tamponu 256 bayt. pushImage milisaniyelerce
  // bloklarken bu tampon tasiyor ve baytlar sessizce dusuyor; sonucu
  // payload CRC hatasi olarak goruluyor. Buyutmek sart.
  const size_t rxSet = Serial.setRxBufferSize(CDC_RX_BUFFER_SIZE);
  const size_t txSet = Serial.setTxBufferSize(CDC_TX_BUFFER_SIZE);
  Serial.begin(SERIAL_BAUD);
  logPrintf("CDC tampon : rx %u istendi %u oldu, tx %u istendi %u oldu\n",
            (unsigned)CDC_RX_BUFFER_SIZE, (unsigned)rxSet,
            (unsigned)CDC_TX_BUFFER_SIZE, (unsigned)txSet);

  logPrintf("Hazir, PC bekleniyor (yerlesik USB portu)\n");
}

void loop()
{
  // Tamponu bosaltana kadar oku.
  while (true) {
    const size_t got = Serial.read(rxChunk, RX_CHUNK_SIZE);
    if (got == 0) {
      break;
    }
    framer.feed(rxChunk, got, millis());
  }

  framer.poll(millis());
}
