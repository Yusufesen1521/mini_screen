#include "log.h"

#include <Arduino.h>
#include <stdarg.h>
#include <stdio.h>

#include "pins.h"

void logBegin()
{
  Serial.begin(SERIAL_BAUD);
  Serial0.begin(SERIAL_BAUD);

  const uint32_t start = millis();
  while (!Serial && (millis() - start) < SERIAL_WAIT_MS) {
    delay(10);
  }
}

void logPrintf(const char *fmt, ...)
{
  char line[160];

  va_list args;
  va_start(args, fmt);
  vsnprintf(line, sizeof(line), fmt, args);
  va_end(args);

  Serial.print(line);
  Serial0.print(line);
}
