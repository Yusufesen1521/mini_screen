// mini_screen - ekran dogrulama testi
//
// Tek hedef: Lockerbox 3.2" ILI9341 panelin ESP32-S3 uzerinde dogru
// calistigini kanitlamak. Renk sirasi, offset ve bolgesel guncelleme.
//
// Ekrana cizim isi drawTestScreen() / drawCounter() icinde toplandi. Ilerideki
// USB tabanli bolge aktarimi bu iki fonksiyonun yerini alacak, gerisi ayni
// kalabilir.

#include <Arduino.h>
#include <TFT_eSPI.h>
#include <stdarg.h>

#include "pins.h"

static TFT_eSPI tft;

// Sayac bolgesi tek seferde basilsin diye sprite kullaniliyor: hem titreme
// olmuyor hem de ileride gelecek "hazir tamponu ekrana bas" akisinin aynisi.
static TFT_eSprite counterSprite(&tft);
static bool counterSpriteReady = false;

static uint32_t counterValue = 0;
static uint32_t lastTickMs = 0;

// ---------------------------------------------------------------------------
// Seri cikti
//
// ARDUINO_USB_CDC_ON_BOOT=1 oldugu icin Serial, kartin yerlesik USB portuna
// (GPIO 19/20) baglanir. Kart UART kopru portuna takiliysa oradan hicbir sey
// gorunmez. Bu yuzden ayni cikti Serial0 (UART0, TX/RX pinleri) uzerine de
// basiliyor; hangi porta takili olursan ol log akar.
// ---------------------------------------------------------------------------

static void logBegin()
{
  Serial.begin(SERIAL_BAUD);
  Serial0.begin(SERIAL_BAUD);

  const uint32_t start = millis();
  while (!Serial && (millis() - start) < SERIAL_WAIT_MS) {
    delay(10);
  }
}

static void logPrintf(const char *fmt, ...)
{
  char line[160];

  va_list args;
  va_start(args, fmt);
  vsnprintf(line, sizeof(line), fmt, args);
  va_end(args);

  Serial.print(line);
  Serial0.print(line);
}

// ---------------------------------------------------------------------------
// Arka isik (LEDC PWM)
// ---------------------------------------------------------------------------

static void backlightBegin()
{
#if ESP_ARDUINO_VERSION_MAJOR >= 3
  ledcAttach(PIN_TFT_BL, BL_PWM_FREQ_HZ, BL_PWM_RESOLUTION_BITS);
#else
  ledcSetup(BL_PWM_CHANNEL, BL_PWM_FREQ_HZ, BL_PWM_RESOLUTION_BITS);
  ledcAttachPin(PIN_TFT_BL, BL_PWM_CHANNEL);
#endif
}

// brightness: 0 = kapali, 255 = tam parlaklik (8 bit cozunurluk ile birebir)
static void backlightSet(uint8_t brightness)
{
#if ESP_ARDUINO_VERSION_MAJOR >= 3
  ledcWrite(PIN_TFT_BL, brightness);
#else
  ledcWrite(BL_PWM_CHANNEL, brightness);
#endif
}

// Ekran baslatilmadan once kisa darbeler. Panel hic goruntu vermese bile bu
// darbeler goruluyorsa firmware calisiyor, GPIO 21 / VCC / GND saglam demektir.
static void backlightHeartbeat()
{
  for (uint8_t i = 0; i < BL_HEARTBEAT_PULSES; i++) {
    backlightSet(BL_BRIGHTNESS_DEFAULT);
    delay(BL_HEARTBEAT_MS);
    backlightSet(BL_BRIGHTNESS_OFF);
    delay(BL_HEARTBEAT_MS);
  }
}

// ---------------------------------------------------------------------------
// Acilis bilgisi
// ---------------------------------------------------------------------------

static void printSystemInfo()
{
  logPrintf("\n=== mini_screen ===\n");
  logPrintf("Chip      : %s rev %u, %u core\n",
            ESP.getChipModel(), ESP.getChipRevision(), ESP.getChipCores());
  logPrintf("Flash     : %u bayt\n", (unsigned)ESP.getFlashChipSize());

  if (psramFound()) {
    logPrintf("PSRAM     : bulundu, %u bayt (bos %u bayt)\n",
              (unsigned)ESP.getPsramSize(), (unsigned)ESP.getFreePsram());
  } else {
    logPrintf("PSRAM     : BULUNAMADI\n");
  }

  logPrintf("Bos heap  : %u bayt\n", (unsigned)ESP.getFreeHeap());
  logPrintf("SPI hizi  : %u Hz\n", (unsigned)SPI_FREQUENCY);
  logPrintf("SPI portu : %u\n", (unsigned)SPI_PORT);
  logPrintf("\n");
}

// ---------------------------------------------------------------------------
// Test ekrani (bir kez cizilir)
// ---------------------------------------------------------------------------

struct ColorBlock {
  uint16_t    color;
  uint16_t    labelColor;
  const char *label;
};

// Etiketler blogun uzerine yaziliyor: renk sirasi ya da RGB/BGR ayari yanlissa
// "RED" yazisi kirmizi olmayan bir blogun uzerinde kalir.
static const ColorBlock kColorBlocks[BLOCK_COUNT] = {
  { COLOR_BLOCK_RED,   COLOR_TEXT,       "RED"   },
  { COLOR_BLOCK_GREEN, COLOR_BACKGROUND, "GREEN" },
  { COLOR_BLOCK_BLUE,  COLOR_TEXT,       "BLUE"  },
  { COLOR_BLOCK_WHITE, COLOR_BACKGROUND, "WHITE" },
};

// Blogun sol ust kosesinin x degeri. Bloklar yan yana dizili.
static int16_t blockX(uint8_t index)
{
  return BLOCK_FIRST_X + index * BLOCK_WIDTH;
}

static void drawCornerMarks()
{
  const int16_t x[CORNER_MARK_COUNT] = { 0, SCREEN_WIDTH - 1, 0, SCREEN_WIDTH - 1 };
  const int16_t y[CORNER_MARK_COUNT] = { 0, 0, SCREEN_HEIGHT - 1, SCREEN_HEIGHT - 1 };

  for (uint8_t i = 0; i < CORNER_MARK_COUNT; i++) {
    tft.drawPixel(x[i], y[i], COLOR_CORNER_MARK);
  }
}

static void drawTestScreen()
{
  tft.fillScreen(COLOR_BACKGROUND);

  // Baslik
  tft.setTextDatum(TC_DATUM);
  tft.setTextColor(COLOR_TEXT, COLOR_BACKGROUND);
  tft.drawString(TITLE_TEXT, SCREEN_WIDTH / 2, TITLE_Y, FONT_TITLE);
  tft.drawFastHLine(0, TITLE_AREA_HEIGHT - 1, SCREEN_WIDTH, COLOR_SEPARATOR);

  // Renk bloklari
  tft.setTextDatum(MC_DATUM);
  for (uint8_t i = 0; i < BLOCK_COUNT; i++) {
    const int16_t x = blockX(i);
    tft.fillRect(x, BLOCK_Y, BLOCK_WIDTH, BLOCK_HEIGHT, kColorBlocks[i].color);
    tft.setTextColor(kColorBlocks[i].labelColor, kColorBlocks[i].color);
    tft.drawString(kColorBlocks[i].label,
                   x + BLOCK_WIDTH / 2,
                   BLOCK_Y + BLOCK_HEIGHT / 2,
                   FONT_LABEL);
  }

  // Sayac etiketi (sabit, sayac bolgesinin disinda)
  tft.setTextDatum(TL_DATUM);
  tft.setTextColor(COLOR_DIM_TEXT, COLOR_BACKGROUND);
  tft.drawString(COUNTER_LABEL, COUNTER_LABEL_X, COUNTER_LABEL_Y, FONT_LABEL);

  drawCornerMarks();
}

// ---------------------------------------------------------------------------
// Bolgesel guncelleme: sadece sayac dikdortgeni
// ---------------------------------------------------------------------------

static void counterRegionBegin()
{
  counterSprite.setColorDepth(16);
  counterSpriteReady = (counterSprite.createSprite(COUNTER_WIDTH, COUNTER_HEIGHT) != nullptr);

  if (!counterSpriteReady) {
    logPrintf("UYARI: sayac sprite ayrilamadi, dogrudan ciziliyor.\n");
  }
}

// Geriye cizim suresini mikrosaniye cinsinden dondurur.
static uint32_t drawCounter(uint32_t value)
{
  char text[12];
  snprintf(text, sizeof(text), "%lu", (unsigned long)value);

  const uint32_t startUs = micros();

  if (counterSpriteReady) {
    counterSprite.fillSprite(COLOR_BACKGROUND);
    counterSprite.setTextDatum(MC_DATUM);
    counterSprite.setTextColor(COLOR_TEXT, COLOR_BACKGROUND);
    counterSprite.drawString(text, COUNTER_WIDTH / 2, COUNTER_HEIGHT / 2, FONT_COUNTER);
    counterSprite.pushSprite(COUNTER_X, COUNTER_Y);
  } else {
    tft.fillRect(COUNTER_X, COUNTER_Y, COUNTER_WIDTH, COUNTER_HEIGHT, COLOR_BACKGROUND);
    tft.setTextDatum(MC_DATUM);
    tft.setTextColor(COLOR_TEXT, COLOR_BACKGROUND);
    tft.drawString(text,
                   COUNTER_X + COUNTER_WIDTH / 2,
                   COUNTER_Y + COUNTER_HEIGHT / 2,
                   FONT_COUNTER);
  }

  return micros() - startUs;
}

// ---------------------------------------------------------------------------

void setup()
{
  logBegin();
  printSystemInfo();

  backlightBegin();
  backlightHeartbeat();

  // Ekran cizilene kadar arka isik kapali: acilis copu gozukmesin.
  backlightSet(BL_BRIGHTNESS_OFF);

  logPrintf("tft.init() cagriliyor...\n");
  tft.init();
  tft.setRotation(DISPLAY_ROTATION);
  logPrintf("tft.init() tamam, %dx%d\n", tft.width(), tft.height());

  if (tft.width() != SCREEN_WIDTH || tft.height() != SCREEN_HEIGHT) {
    logPrintf("UYARI: pins.h olculeri (%dx%d) donusle uyusmuyor.\n",
              SCREEN_WIDTH, SCREEN_HEIGHT);
  }

  drawTestScreen();
  counterRegionBegin();

  backlightSet(BL_BRIGHTNESS_DEFAULT);

  lastTickMs = millis();
  logPrintf("Ekran hazir.\n");
}

void loop()
{
  const uint32_t now = millis();
  if ((now - lastTickMs) < COUNTER_INTERVAL_MS) {
    return;
  }
  lastTickMs += COUNTER_INTERVAL_MS;

  counterValue++;
  const uint32_t drawUs = drawCounter(counterValue);
  logPrintf("sayac=%lu  cizim=%lu us\n",
            (unsigned long)counterValue, (unsigned long)drawUs);
}
