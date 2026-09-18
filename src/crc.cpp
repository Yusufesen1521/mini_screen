#include "crc.h"

// CRC-8 basliga uygulaniyor, yani 7 bayta. Tablo kurmaya degmez.
uint8_t crc8(const uint8_t *data, size_t len)
{
  uint8_t crc = 0x00;

  for (size_t i = 0; i < len; i++) {
    crc ^= data[i];
    for (uint8_t bit = 0; bit < 8; bit++) {
      crc = (crc & 0x80) ? (uint8_t)((crc << 1) ^ 0x07) : (uint8_t)(crc << 1);
    }
  }
  return crc;
}

// CRC-16 en buyuk 64 KB payload'a uygulaniyor. Bit bit hesaplamak bayt basina
// 8 dongu demek; tablo ile bu bire iniyor. Tablo 512 bayt.
static uint16_t crc16Table[256];
static bool crc16TableReady = false;

void crcBegin()
{
  if (crc16TableReady) {
    return;
  }

  for (uint32_t i = 0; i < 256; i++) {
    uint16_t value = (uint16_t)(i << 8);
    for (uint8_t bit = 0; bit < 8; bit++) {
      value = (value & 0x8000) ? (uint16_t)((value << 1) ^ 0x1021)
                               : (uint16_t)(value << 1);
    }
    crc16Table[i] = value;
  }
  crc16TableReady = true;
}

uint16_t crc16(const uint8_t *data, size_t len)
{
  uint16_t crc = 0xFFFF;

  for (size_t i = 0; i < len; i++) {
    crc = (uint16_t)((crc << 8) ^ crc16Table[(crc >> 8) ^ data[i]]);
  }
  return crc;
}
