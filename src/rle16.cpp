#include "rle16.h"

#include <string.h>

int32_t rle16Decode(const uint8_t *src, size_t srcLen,
                    uint16_t *dst, size_t dstPixels)
{
  size_t si = 0;
  size_t di = 0;

  while (si < srcLen) {
    const uint8_t control = src[si++];

    if (control < 0x80) {
      const size_t n = (size_t)control + 1;

      if (si + n * 2 > srcLen) {
        return -1;
      }
      if (di + n > dstPixels) {
        return -2;
      }

      // Kaynak little-endian, hedef de little-endian. Dogrudan kopyalanir.
      memcpy(&dst[di], &src[si], n * 2);
      si += n * 2;
      di += n;
    } else {
      const size_t n = (size_t)(control & 0x7F) + 2;

      if (si + 2 > srcLen) {
        return -1;
      }
      if (di + n > dstPixels) {
        return -2;
      }

      const uint16_t pixel = (uint16_t)(src[si] | (src[si + 1] << 8));
      si += 2;

      for (size_t k = 0; k < n; k++) {
        dst[di++] = pixel;
      }
    }
  }

  return (int32_t)di;
}
