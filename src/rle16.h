// rle16.h - RGB565 piksellere uygulanan kosu kodlamasi, cozucu tarafi
//
// Birim bayt degil piksel. Kontrol bayti:
//   0x00-0x7F  duz kosu,    n = kontrol + 1        piksel (1-128)
//   0x80-0xFF  tekrar kosu, n = (kontrol & 0x7F) + 2 piksel (2-129)
//
// Kodlayici PC tarafinda. Tam tanim docs/protocol.md icinde.

#pragma once

#include <stddef.h>
#include <stdint.h>

// Cozer ve uretilen piksel sayisini dondurur.
// Hata durumunda negatif doner:
//   -1 kaynak beklenenden once bitti
//   -2 hedef tampon tasacakti
int32_t rle16Decode(const uint8_t *src, size_t srcLen,
                    uint16_t *dst, size_t dstPixels);
