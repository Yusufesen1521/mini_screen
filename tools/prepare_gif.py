#!/usr/bin/env python3
"""Kaynak GIF'leri cihaz olcusune indirir.

gif/ altindaki her GIF, en boy orani korunarak ekrana sigacak sekilde
kucultulur ve data/ altina yazilir. Cihaza yuklenen bunlardir.

Neden gerekli: cihaz kendi olceklemesini yapabiliyor ama o zaman
gosterecegi pikselden fazlasini cozmek zorunda kaliyor. 480x270 bir GIF'te
olculen fark 14.8 FPS yerine 19.0 FPS, yani yuzde 28 pay. Ayrintisi
docs/measurements.md icinde.

Bu is ileride PC uygulamasinin gorevi olacak; bu betik onun yerini
tutuyor.

Kullanim:
  python tools/prepare_gif.py
  pio run -e gifplay -t uploadfs
"""

import os
import shutil
import struct
import subprocess
import sys

SRC_DIR = "gif"
DST_DIR = "data"
SCREEN_W = 320
SCREEN_H = 240


def gif_size(path):
    with open(path, "rb") as f:
        head = f.read(10)
    if head[:3] != b"GIF":
        return None
    return struct.unpack_from("<HH", head, 6)


def main():
    if shutil.which("ffmpeg") is None:
        sys.exit("ffmpeg bulunamadi, PATH icinde olmali")

    os.makedirs(DST_DIR, exist_ok=True)
    names = sorted(n for n in os.listdir(SRC_DIR) if n.lower().endswith(".gif"))
    if not names:
        sys.exit("%s altinda GIF yok" % SRC_DIR)

    total = 0
    for name in names:
        src = os.path.join(SRC_DIR, name)
        dst = os.path.join(DST_DIR, name)
        size = gif_size(src)
        if size is None:
            print("%-20s atlandi, GIF degil" % name)
            continue

        # palettegen ve paletteuse tek gecişte. dither=none secildi:
        # dithering gurultu ekliyor ve GIF sikistirmasini bozuyor, ayni
        # klipte 2.17 MB yerine 2.31 MB veriyordu.
        vf = ("scale=w=%d:h=%d:force_original_aspect_ratio=decrease:flags=lanczos,"
              "split[s0][s1];[s0]palettegen=max_colors=256[p];"
              "[s1][p]paletteuse=dither=none" % (SCREEN_W, SCREEN_H))

        subprocess.run(
            ["ffmpeg", "-hide_banner", "-loglevel", "error",
             "-i", src, "-vf", vf, "-y", dst],
            check=True)

        out_size = gif_size(dst)
        bytes_in = os.path.getsize(src)
        bytes_out = os.path.getsize(dst)
        total += bytes_out
        print("%-20s %dx%d -> %dx%d   %.2f MB -> %.2f MB" %
              (name, size[0], size[1], out_size[0], out_size[1],
               bytes_in / 1e6, bytes_out / 1e6))

    print("\ntoplam %.2f MB, dosya sistemi 11.9 MB" % (total / 1e6))


if __name__ == "__main__":
    main()
