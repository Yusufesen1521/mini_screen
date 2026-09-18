"""mini_screen USB protokolu, PC tarafi.

Bayt duzeyinde tanim: docs/protocol.md
Cihaz tarafindaki karsiligi: include/protocol.h

Bu modul sadece protokolu bilir, seri port acmaz.
"""

import struct

VERSION = 1

SOF = b"\xA5\x5A"
HDR_SIZE = 10
CRC_SIZE = 2
MAX_PAYLOAD = 65535

FLAG_ACK_REQ = 0x01

# PC -> cihaz
MSG_HELLO = 0x01
MSG_FRAME_REGION = 0x10
MSG_SET_BACKLIGHT = 0x20
MSG_PING = 0x30
MSG_GET_STATUS = 0x40

# Cihaz -> PC
MSG_CAPS = 0x81
MSG_ACK = 0x82
MSG_NACK = 0x83
MSG_ERROR = 0x84
MSG_STATUS = 0x85
MSG_PONG = 0x86
MSG_LOG = 0x87

MSG_NAMES = {
    MSG_HELLO: "HELLO", MSG_FRAME_REGION: "FRAME_REGION",
    MSG_SET_BACKLIGHT: "SET_BACKLIGHT", MSG_PING: "PING",
    MSG_GET_STATUS: "GET_STATUS", MSG_CAPS: "CAPS", MSG_ACK: "ACK",
    MSG_NACK: "NACK", MSG_ERROR: "ERROR", MSG_STATUS: "STATUS",
    MSG_PONG: "PONG", MSG_LOG: "LOG",
}

ERR_NAMES = {
    0x01: "BAD_HEADER_CRC", 0x02: "BAD_PAYLOAD_CRC", 0x03: "BAD_VERSION",
    0x04: "UNKNOWN_TYPE", 0x05: "PAYLOAD_TOO_LARGE", 0x06: "BAD_REGION",
    0x07: "BAD_CODEC", 0x08: "DECODE_ERROR", 0x09: "OVERRUN",
}

REGION_HDR_SIZE = 10
PIXFMT_RGB565_LE = 0x01
CODEC_NONE = 0x00
CODEC_RLE16 = 0x01


# ---------------------------------------------------------------------------
# CRC
# ---------------------------------------------------------------------------

def crc8(data):
    """Polinom 0x07, baslangic 0x00. '123456789' -> 0xF4"""
    crc = 0x00
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = ((crc << 1) ^ 0x07) & 0xFF if crc & 0x80 else (crc << 1) & 0xFF
    return crc


_CRC16_TABLE = []


def _build_crc16_table():
    for i in range(256):
        v = i << 8
        for _ in range(8):
            v = ((v << 1) ^ 0x1021) & 0xFFFF if v & 0x8000 else (v << 1) & 0xFFFF
        _CRC16_TABLE.append(v)


_build_crc16_table()


def crc16(data):
    """CRC-16/CCITT-FALSE. '123456789' -> 0x29B1"""
    crc = 0xFFFF
    for b in data:
        crc = ((crc << 8) ^ _CRC16_TABLE[(crc >> 8) ^ b]) & 0xFFFF
    return crc


# ---------------------------------------------------------------------------
# Cerceve
# ---------------------------------------------------------------------------

def build_frame(msg_type, seq, payload=b"", flags=0, version=VERSION, bad_crc=False):
    """Cerceve olusturur. bad_crc sadece uyum testleri icin."""
    hdr = struct.pack("<BBBBHB", version, msg_type, flags, seq, len(payload), 0)
    frame = bytearray(SOF)
    frame += hdr
    frame.append(crc8(hdr))
    frame += payload
    c = crc16(payload)
    if bad_crc:
        c ^= 0xFFFF
    frame += struct.pack("<H", c)
    return bytes(frame)


def build_raw_header(msg_type, seq, declared_len, flags=0, version=VERSION):
    """Payload'i olmayan, sadece baslik. Asiri buyuk LEN testleri icin."""
    hdr = struct.pack("<BBBBHB", version, msg_type, flags, seq, declared_len, 0)
    return SOF + hdr + bytes([crc8(hdr)])


class Parser:
    """Cihazdan gelen cerceveleri ayristirir.

    Cihaz tarafiyla ayni kaydirmali pencere yaklasimi kullaniliyor.
    """

    def __init__(self):
        self._win = bytearray()
        self._buf = bytearray()
        self.frames = []

    def feed(self, data):
        for b in data:
            self._feed_byte(b)
        out, self.frames = self.frames, []
        return out

    def _feed_byte(self, b):
        self._buf.append(b)
        if len(self._buf) < HDR_SIZE:
            return
        while True:
            frame = self._try_extract()
            if frame is None:
                return
            self.frames.append(frame)

    def _try_extract(self):
        buf = self._buf
        for i in range(len(buf) - HDR_SIZE + 1):
            if buf[i:i + 2] != SOF:
                continue
            hdr = buf[i + 2:i + 9]
            if crc8(hdr) != buf[i + 9]:
                continue
            version, mtype, flags, seq, length, _rsv = struct.unpack("<BBBBHB", hdr)
            total = HDR_SIZE + length + CRC_SIZE
            if len(buf) - i < total:
                del buf[:i]
                return None
            payload = bytes(buf[i + HDR_SIZE:i + HDR_SIZE + length])
            got = struct.unpack("<H", buf[i + HDR_SIZE + length:i + total])[0]
            del buf[:i + total]
            return {
                "version": version, "type": mtype, "flags": flags, "seq": seq,
                "payload": payload, "crc_ok": got == crc16(payload),
            }
        if len(buf) > HDR_SIZE:
            del buf[:len(buf) - HDR_SIZE + 1]
        return None


# ---------------------------------------------------------------------------
# RLE16
# ---------------------------------------------------------------------------

MAX_RUN = 129       # tekrar kosusu en fazla
MAX_LITERAL = 128   # duz kosu en fazla


def rle16_encode(pixels):
    """pixels: uint16 dizisi. RGB565 kosu kodlamasi uygular."""
    out = bytearray()
    n = len(pixels)
    i = 0

    while i < n:
        run = 1
        while i + run < n and pixels[i + run] == pixels[i] and run < MAX_RUN:
            run += 1

        if run >= 2:
            out.append(0x80 | (run - 2))
            out += struct.pack("<H", pixels[i])
            i += run
            continue

        # Duz kosu: uc ayni piksel gorunce kes, orada tekrar kosusu daha ucuz
        start = i
        while i < n and (i - start) < MAX_LITERAL:
            if i + 2 < n and pixels[i] == pixels[i + 1] == pixels[i + 2]:
                break
            i += 1
        count = i - start
        out.append(count - 1)
        for k in range(start, i):
            out += struct.pack("<H", pixels[k])

    return bytes(out)


def rle16_decode(data, max_pixels):
    """Cihaz tarafiyla ayni cozucu. Kendi kodlayicimizi dogrulamak icin."""
    out = []
    i = 0
    n = len(data)

    while i < n:
        control = data[i]
        i += 1
        if control < 0x80:
            count = control + 1
            if i + count * 2 > n:
                return -1
            if len(out) + count > max_pixels:
                return -2
            for k in range(count):
                out.append(struct.unpack_from("<H", data, i + k * 2)[0])
            i += count * 2
        else:
            count = (control & 0x7F) + 2
            if i + 2 > n:
                return -1
            if len(out) + count > max_pixels:
                return -2
            pixel = struct.unpack_from("<H", data, i)[0]
            i += 2
            out.extend([pixel] * count)

    return out


def encode_region(x, y, w, h, pixels, force_codec=None):
    """Bolge payload'i uretir. Sikistirma kazandirmiyorsa ham gonderir."""
    raw = struct.pack("<%dH" % len(pixels), *pixels)

    if force_codec == CODEC_NONE:
        codec, data = CODEC_NONE, raw
    else:
        rle = rle16_encode(pixels)
        if force_codec == CODEC_RLE16 or len(rle) < len(raw):
            codec, data = CODEC_RLE16, rle
        else:
            codec, data = CODEC_NONE, raw

    hdr = struct.pack("<HHHHBB", x, y, w, h, PIXFMT_RGB565_LE, codec)
    return hdr + data, codec
