#include "log.h"

#include <Arduino.h>
#include <stdarg.h>
#include <stdio.h>

#include "pins.h"

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
  vsnprintf(line, sizeof(line), fmt, args);
  va_end(args);

  Serial0.print(line);
}
