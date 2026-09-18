#include "log.h"

#include <Arduino.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "pins.h"


static LogSink logSink = nullptr;

static char   bootBuf[LOG_BOOT_BUF_SIZE];
static size_t bootLen = 0;

void logBegin()
{
  // Serial (USB CDC) protokole ait, burada acilmiyor.
  Serial0.begin(SERIAL_BAUD);
}

void logPrintf(const char *fmt, ...)
{
  char line[160];

  va_list args;
  va_start(args, fmt);
  const int written = vsnprintf(line, sizeof(line), fmt, args);
  va_end(args);

  if (written <= 0) {
    return;
  }
  const size_t len = (written < (int)sizeof(line)) ? (size_t)written
                                                   : sizeof(line) - 1;

  Serial0.write((const uint8_t *)line, len);

  if (logSink != nullptr) {
    logSink(line);
    return;
  }

  // Henuz baglanti yok, biriktir. Tampon dolarsa yenileri dusurulur;
  // acilis ciktisinin basi sonundan daha degerli.
  if (bootLen + len < sizeof(bootBuf)) {
    memcpy(&bootBuf[bootLen], line, len);
    bootLen += len;
    bootBuf[bootLen] = '\0';
  }
}

void logSetSink(LogSink sink)
{
  logSink = sink;

  if (sink != nullptr && bootLen > 0) {
    const size_t pending = bootLen;
    bootLen = 0;            // once sifirla, sink icinden log gelirse donmesin
    bootBuf[pending] = '\0';
    sink(bootBuf);
  }
}
