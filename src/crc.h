// crc.h - protokolun kullandigi iki CRC
//
// CRC-8 : polinom 0x07, baslangic 0x00, yansitma yok, cikis XOR yok
//         "123456789" -> 0xF4
// CRC-16: CRC-16/CCITT-FALSE, polinom 0x1021, baslangic 0xFFFF
//         "123456789" -> 0x29B1

#pragma once

#include <stddef.h>
#include <stdint.h>

// CRC-16 tablosunu hazirlar. Diger fonksiyonlardan once bir kez cagrilmali.
void crcBegin();

uint8_t  crc8(const uint8_t *data, size_t len);
uint16_t crc16(const uint8_t *data, size_t len);
