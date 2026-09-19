#!/usr/bin/env python3
"""mini_screen baglanti test araci, Faz 1.

Bu bir urun degil, olcum ve uyum araci. Faz 2'deki masaustu uygulamasi
ayri yazilacak; bu arac protokolun dogru calistigini kanitlamak icin.

Kullanim:
  python tools/link_test.py ports
  python tools/link_test.py hello
  python tools/link_test.py bench [--frames 30]
  python tools/link_test.py conformance
  python tools/link_test.py endurance --minutes 30
"""

import argparse
import random
import struct
import sys
import time

import serial
import serial.tools.list_ports

import protocol as P

ESPRESSIF_VID = 0x303A
SCREEN_W = 320
SCREEN_H = 240
STRIPE_ROWS = 80          # 320*80*2 + 10 = 51210 bayt, 65535 sinirinin altinda


# ---------------------------------------------------------------------------
# Baglanti
# ---------------------------------------------------------------------------

def find_port():
    for p in serial.tools.list_ports.comports():
        if p.vid == ESPRESSIF_VID:
            return p.device
    return None


def open_link(port=None):
    port = port or find_port()
    if port is None:
        sys.exit("Cihaz bulunamadi. Yerlesik USB portu takili mi? "
                 "'ports' komutuyla bakabilirsin, --port ile elle verebilirsin.")
    ser = serial.Serial(port, 115200, timeout=0.2, write_timeout=5.0)
    ser.reset_input_buffer()
    return ser


class Link:
    def __init__(self, ser, window=1):
        self.ser = ser
        self.parser = P.Parser()
        self.seq = 0

        # Akis kontrolu. Cihaz CAPS ile rx_slots bildiriyor, el sikismadan
        # sonra buraya yaziliyor. Ayrintisi docs/protocol.md icinde.
        self.window = window
        self.pending = []      # onay bekleyen seq listesi
        self.nacks = 0
        self.ack_timeouts = 0
        self.last_heap = None

    def next_seq(self):
        s = self.seq
        self.seq = (self.seq + 1) & 0xFF
        return s

    # -- akis kontrollu gonderim --------------------------------------------

    def send_windowed(self, frame, seq, timeout=2.0):
        """Pencere dolu ise once yer acar, sonra gonderir."""
        while len(self.pending) >= self.window:
            if not self._reap(timeout):
                self.ack_timeouts += 1
                self.pending.pop(0)     # bu cerceveden umudu kes, tikanma
        self.ser.write(frame)
        self.pending.append(seq)

    def drain(self, timeout=3.0):
        """Bekleyen butun onaylari topla. Olcum bitiminde cagrilir."""
        while self.pending:
            if not self._reap(timeout):
                self.ack_timeouts += len(self.pending)
                self.pending.clear()
                return False
        return True

    def _reap(self, timeout):
        """En az bir onay ya da ret gelene kadar bekler."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            n = self.ser.in_waiting
            data = self.ser.read(n if n else 1)
            if not data:
                continue
            progressed = False
            for f in self.parser.feed(data):
                if f["type"] == P.MSG_LOG:
                    self._print_log(f["payload"])
                elif f["type"] == P.MSG_ACK:
                    self._retire(f["seq"])
                    progressed = True
                elif f["type"] == P.MSG_NACK:
                    self.nacks += 1
                    if len(f["payload"]) >= 2:
                        self._retire(f["payload"][1])
                    progressed = True
            if progressed:
                return True
        return False

    def _retire(self, seq):
        if seq in self.pending:
            self.pending.remove(seq)

    def send(self, msg_type, payload=b"", flags=0, **kw):
        seq = self.next_seq()
        self.ser.write(P.build_frame(msg_type, seq, payload, flags, **kw))
        return seq

    def send_raw(self, data):
        self.ser.write(data)

    def poll(self, timeout=0.5):
        """Gelen cerceveleri toplar. LOG cerceveleri ayiklanip basilir."""
        deadline = time.time() + timeout
        frames = []
        while time.time() < deadline:
            n = self.ser.in_waiting
            data = self.ser.read(n if n else 1)
            if data:
                for f in self.parser.feed(data):
                    if f["type"] == P.MSG_LOG:
                        self._print_log(f["payload"])
                    else:
                        frames.append(f)
            elif frames:
                break
        return frames

    def _print_log(self, payload):
        text = payload.decode("utf-8", "replace")
        for line in text.splitlines():
            if not line.strip():
                continue
            if line.startswith("heap "):
                parts = line.split()
                try:
                    self.last_heap = (int(parts[1]), int(parts[3]))
                except (IndexError, ValueError):
                    pass
            print("  [cihaz] %s" % line)

    def expect(self, msg_type, timeout=1.0):
        for f in self.poll(timeout):
            if f["type"] == msg_type:
                return f
        return None


def describe(frame):
    name = P.MSG_NAMES.get(frame["type"], "0x%02X" % frame["type"])
    if frame["type"] == P.MSG_NACK and len(frame["payload"]) >= 2:
        reason = frame["payload"][0]
        return "%s(%s, seq=%d)" % (name, P.ERR_NAMES.get(reason, hex(reason)),
                                   frame["payload"][1])
    return name


def handshake(link, quiet=False):
    link.send(P.MSG_HELLO, struct.pack("<BBBB", P.VERSION, 0, 1, 0))
    caps = link.expect(P.MSG_CAPS, timeout=2.0)
    if caps is None:
        sys.exit("CAPS gelmedi. Cihaz protokol firmware'i ile yuklu mu?")

    p = caps["payload"]
    info = {
        "proto": p[0],
        "fw": "%d.%d.%d" % (p[1], p[2], p[3]),
        "w": struct.unpack_from("<H", p, 4)[0],
        "h": struct.unpack_from("<H", p, 6)[0],
        "pixfmt": p[8],
        "codecs": p[9],
        "max_payload": struct.unpack_from("<H", p, 10)[0],
        "rx_slots": p[12],
        "selftest": p[13],
        "mac": ":".join("%02X" % b for b in p[14:20]),
    }
    link.window = max(1, info["rx_slots"])
    link.poll(0.4)   # CAPS arkasindan gelen acilis loglari

    if not quiet:
        print("Cihaz baglandi")
        print("  protokol    : %d" % info["proto"])
        print("  firmware    : %s" % info["fw"])
        print("  ekran       : %dx%d" % (info["w"], info["h"]))
        print("  codec maske : 0x%02X (NONE%s)" %
              (info["codecs"], ", RLE16" if info["codecs"] & 0x02 else ""))
        print("  max payload : %d bayt" % info["max_payload"])
        print("  rx slot     : %d" % info["rx_slots"])
        print("  kendini sinama: %s" % ("TAMAM" if info["selftest"] else "HATA"))
        print("  mac         : %s" % info["mac"])
    return info


# ---------------------------------------------------------------------------
# Test gorselleri
# ---------------------------------------------------------------------------

def rgb565(r, g, b):
    return ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)


def image_solid(w, h, color=0x001F):
    return [color] * (w * h)


def image_ui(w, h):
    """Gercekci arayuz: duz zemin, cubuklar, metin benzeri bloklar."""
    bg = rgb565(16, 18, 24)
    px = [bg] * (w * h)
    for i, color in enumerate([rgb565(220, 60, 60), rgb565(60, 200, 120),
                               rgb565(70, 140, 240), rgb565(240, 190, 60)]):
        top = 20 + i * 34
        width = int(w * (0.35 + 0.15 * i))
        for y in range(top, min(top + 18, h)):
            for x in range(16, min(16 + width, w)):
                px[y * w + x] = color
    # Metin benzeri kisa parcalar
    rnd = random.Random(7)
    for row in range(h - 60, h - 10, 12):
        x = 16
        while x < w - 20:
            run = rnd.randint(2, 9)
            for xx in range(x, min(x + run, w)):
                for yy in range(row, min(row + 7, h)):
                    px[yy * w + xx] = 0xFFFF
            x += run + rnd.randint(3, 8)
    return px


def image_gradient(w, h):
    return [rgb565(x * 255 // max(w - 1, 1), y * 255 // max(h - 1, 1), 128)
            for y in range(h) for x in range(w)]


def image_noise(w, h, seed=1):
    rnd = random.Random(seed)
    return [rnd.randint(0, 0xFFFF) for _ in range(w * h)]


IMAGES = {
    "solid": image_solid,
    "ui": image_ui,
    "gradient": image_gradient,
    "noise": image_noise,
}


def build_stripes(px, w, h, force_codec=None):
    """Tam kareyi serit payload'larina boler. Bir kez hesaplanip tekrar
    kullanilabilsin diye gonderimden ayrildi."""
    out = []
    for top in range(0, h, STRIPE_ROWS):
        rows = min(STRIPE_ROWS, h - top)
        payload, codec = P.encode_region(0, top, w, rows,
                                         px[top * w:(top + rows) * w], force_codec)
        out.append((payload, codec))
    return out


def send_image(link, px, w, h, force_codec=None, stripes=None):
    """Tam kareyi akis kontrollu gonderir. Yazilan bayt sayisini doner."""
    if stripes is None:
        stripes = build_stripes(px, w, h, force_codec)

    written = 0
    for payload, _codec in stripes:
        seq = link.next_seq()
        frame = P.build_frame(P.MSG_FRAME_REGION, seq, payload, P.FLAG_ACK_REQ)
        link.send_windowed(frame, seq)
        written += len(frame)
    return written


# ---------------------------------------------------------------------------
# Komutlar
# ---------------------------------------------------------------------------

def cmd_ports(args):
    found = False
    for p in serial.tools.list_ports.comports():
        mark = "  <- cihaz" if p.vid == ESPRESSIF_VID else ""
        print("%-8s vid:pid=%04X:%04X  %s%s" %
              (p.device, p.vid or 0, p.pid or 0, p.description, mark))
        found = True
    if not found:
        print("Hicbir seri port bulunamadi.")


def cmd_hello(args):
    handshake(Link(open_link(args.port)))


def cmd_bench(args):
    link = Link(open_link(args.port))
    caps = handshake(link, quiet=True)
    w, h = caps["w"], caps["h"]
    raw_size = w * h * 2

    print("Tam kare %dx%d, ham %d bayt, %d kare olculuyor\n" % (w, h, raw_size, args.frames))
    print("%-10s %-8s %10s %8s %8s %8s %7s" %
          ("gorsel", "codec", "tel bayt", "sikisma", "MB/s", "FPS", "NACK"))
    print("-" * 66)

    for name in ("solid", "ui", "gradient", "noise"):
        px = IMAGES[name](w, h)
        stripes = build_stripes(px, w, h)

        # Isinma
        send_image(link, px, w, h, stripes=stripes)
        link.drain()

        nacks0 = link.nacks
        start = time.perf_counter()
        total = 0
        for _ in range(args.frames):
            total += send_image(link, px, w, h, stripes=stripes)
        link.drain()            # butun onaylar gelene kadar bekle
        elapsed = time.perf_counter() - start

        per_frame = total / args.frames
        codec_name = "RLE16" if stripes[0][1] == P.CODEC_RLE16 else "NONE"
        print("%-10s %-8s %10d %7.1fx %8.2f %8.1f %7d" %
              (name, codec_name, per_frame, raw_size / per_frame,
               total / elapsed / 1e6, args.frames / elapsed,
               link.nacks - nacks0))

    link.send(P.MSG_GET_STATUS)
    st = link.expect(P.MSG_STATUS, timeout=2.0)
    if st:
        p = st["payload"]
        ok, dropped = struct.unpack_from("<II", p, 0)
        hdr_err, pl_err, sync = struct.unpack_from("<HHH", p, 8)
        print("\nCihaz sayaclari: islenen=%d dusen=%d hdrCRC=%d payloadCRC=%d senkron=%d"
              % (ok, dropped, hdr_err, pl_err, sync))


def cmd_conformance(args):
    link = Link(open_link(args.port))
    caps = handshake(link, quiet=True)
    w, h = caps["w"], caps["h"]
    results = []

    def check(name, ok, detail=""):
        results.append((name, ok, detail))
        print("  %-34s %s %s" % (name, "TAMAM" if ok else "HATA", detail))

    def expect_nack(name, data, reason):
        link.ser.reset_input_buffer()
        link.parser = P.Parser()
        link.send_raw(data)
        for f in link.poll(1.0):
            if f["type"] == P.MSG_NACK and f["payload"][0] == reason:
                check(name, True)
                return
        check(name, False, "beklenen NACK %s gelmedi" % P.ERR_NAMES[reason])

    def alive(name):
        """Cihaz hala cevap veriyor mu."""
        link.ser.reset_input_buffer()
        link.parser = P.Parser()
        seq = link.send(P.MSG_PING)
        f = link.expect(P.MSG_PONG, timeout=1.0)
        check(name, f is not None and f["seq"] == seq)

    print("Uyum testleri\n")

    # Mutlu yol
    px = image_ui(w, h)
    link.ser.reset_input_buffer()
    link.parser = P.Parser()
    payload, _ = P.encode_region(0, 0, w, 40, px[:w * 40])
    link.send_raw(P.build_frame(P.MSG_FRAME_REGION, 1, payload, P.FLAG_ACK_REQ))
    f = link.expect(P.MSG_ACK, timeout=1.0)
    check("gecerli bolge, ACK istendi", f is not None and f["seq"] == 1)

    # Hata yollari
    expect_nack("bozuk payload CRC",
                P.build_frame(P.MSG_PING, 9, b"\x00" * 4, bad_crc=True), 0x02)
    expect_nack("desteklenmeyen surum",
                P.build_frame(P.MSG_PING, 10, b"", version=99), 0x03)
    expect_nack("bilinmeyen mesaj tipi",
                P.build_frame(0x77, 11, b""), 0x04)

    big = P.build_raw_header(P.MSG_FRAME_REGION, 12, caps["max_payload"])
    # max_payload tam sinir, bir fazlasi reddedilmeli
    if caps["max_payload"] < 0xFFFF:
        big = P.build_raw_header(P.MSG_FRAME_REGION, 12, caps["max_payload"] + 1)
        expect_nack("asiri buyuk LEN", big, 0x05)
    else:
        print("  %-34s atlandi (max_payload zaten 65535)" % "asiri buyuk LEN")

    bad_region = struct.pack("<HHHHBB", w - 4, 0, 16, 16, P.PIXFMT_RGB565_LE,
                             P.CODEC_NONE) + b"\x00" * (16 * 16 * 2)
    expect_nack("ekran disi bolge",
                P.build_frame(P.MSG_FRAME_REGION, 13, bad_region), 0x06)

    bad_codec = struct.pack("<HHHHBB", 0, 0, 4, 4, P.PIXFMT_RGB565_LE, 0x7E)
    expect_nack("bilinmeyen codec",
                P.build_frame(P.MSG_FRAME_REGION, 14, bad_codec), 0x07)

    # RLE beklenenden az piksel uretiyor
    short_rle = struct.pack("<HHHHBB", 0, 0, 8, 8, P.PIXFMT_RGB565_LE,
                            P.CODEC_RLE16) + bytes([0x82, 0x00, 0xF8])
    expect_nack("RLE piksel sayisi tutmuyor",
                P.build_frame(P.MSG_FRAME_REGION, 15, short_rle), 0x08)

    alive("hatalardan sonra hala canli")

    # Senkron testleri
    link.ser.reset_input_buffer()
    link.parser = P.Parser()
    link.send_raw(bytes(random.Random(3).randint(0, 255) for _ in range(512)))
    time.sleep(0.05)
    alive("rastgele copten sonra senkron")

    # Ust uste binen sahte SOF
    link.ser.reset_input_buffer()
    link.parser = P.Parser()
    seq = link.next_seq()
    link.send_raw(b"\xA5\x5A\xA5\x5A" + P.build_frame(P.MSG_PING, seq)[2:])
    f = link.expect(P.MSG_PONG, timeout=1.0)
    check("ust uste binen sahte SOF", f is not None and f["seq"] == seq)

    # Bayt bayt gonderim
    link.ser.reset_input_buffer()
    link.parser = P.Parser()
    seq = link.next_seq()
    for b in P.build_frame(P.MSG_PING, seq):
        link.send_raw(bytes([b]))
    f = link.expect(P.MSG_PONG, timeout=2.0)
    check("bayt bayt bolunmus cerceve", f is not None and f["seq"] == seq)

    # Yarim cerceve, zaman asimi ile toparlanmali
    link.ser.reset_input_buffer()
    link.parser = P.Parser()
    link.send_raw(P.build_raw_header(P.MSG_FRAME_REGION, 20, 1000) + b"\x00" * 100)
    time.sleep(0.7)
    alive("yarim cerceve zaman asimi")

    # Sifir boyutlu bolge
    zero = struct.pack("<HHHHBB", 0, 0, 0, 4, P.PIXFMT_RGB565_LE, P.CODEC_NONE)
    expect_nack("sifir boyutlu bolge",
                P.build_frame(P.MSG_FRAME_REGION, 21, zero), 0x06)

    alive("tum testlerden sonra canli")

    failed = [r for r in results if not r[1]]
    print("\n%d testten %d tanesi gecti" % (len(results), len(results) - len(failed)))
    return 1 if failed else 0


ANIM_FRAMES = 8


def build_animation(w, h):
    """Donen kare seti. Her karede kucuk bir blok yer degistiriyor.

    Ilk surumde her karede bir piksel rastgeleye ceviriliyordu, veri
    statik olmasin diye. Kumulatif oldugu icin 8600 kare sonra
    goruntunun yuzde 11'i gurultuye donusmus, sikisma 21 kattan 2.7 kata
    inmis ve olculen FPS 19.7'den 2.7'ye dusmustu. Cihazda hicbir
    yavaslama yoktu, sadece yuk buyuyordu. Donen set bunu onluyor:
    sikisma sabit kaliyor.
    """
    base = image_ui(w, h)
    block = 40
    color = rgb565(250, 250, 80)
    out = []
    for i in range(ANIM_FRAMES):
        px = list(base)
        x0 = 16 + i * ((w - block - 32) // max(ANIM_FRAMES - 1, 1))
        y0 = h // 2
        for y in range(y0, min(y0 + block, h)):
            row = y * w
            for x in range(x0, min(x0 + block, w)):
                px[row + x] = color
        out.append((px, build_stripes(px, w, h)))
    return out


def cmd_endurance(args):
    link = Link(open_link(args.port))
    caps = handshake(link, quiet=True)
    w, h = caps["w"], caps["h"]
    print("Animasyon kareleri hazirlaniyor...")
    anim = build_animation(w, h)

    deadline = time.time() + args.minutes * 60
    frames = 0
    total = 0
    start = time.perf_counter()
    last_report = start

    print("Dayaniklilik testi: %d dakika. Ctrl+C ile kesebilirsin.\n" % args.minutes)

    link.send(P.MSG_GET_STATUS)
    link.expect(P.MSG_STATUS, 2.0)
    heap_start = link.last_heap
    try:
        while time.time() < deadline:
            px, stripes = anim[frames % ANIM_FRAMES]
            total += send_image(link, px, w, h, stripes=stripes)
            frames += 1

            now = time.perf_counter()
            if now - last_report >= 30.0:
                print("  %6.1f dk  kare=%d  %.2f MB/s" %
                      ((now - start) / 60.0, frames, total / (now - start) / 1e6))
                last_report = now
    except KeyboardInterrupt:
        print("\nkesildi")

    elapsed = time.perf_counter() - start
    link.send(P.MSG_GET_STATUS)
    st = link.expect(P.MSG_STATUS, timeout=2.0)

    print("\nGonderilen kare : %d" % frames)
    print("Sure            : %.1f dk" % (elapsed / 60.0))
    print("Ortalama        : %.2f MB/s, %.1f FPS" %
          (total / elapsed / 1e6, frames / elapsed))
    if st:
        p = st["payload"]
        ok, dropped = struct.unpack_from("<II", p, 0)
        hdr_err, pl_err, sync = struct.unpack_from("<HHH", p, 8)
        print("Cihaz sayaclari : islenen=%d dusen=%d hdrCRC=%d payloadCRC=%d senkron=%d"
              % (ok, dropped, hdr_err, pl_err, sync))
        print("PC tarafi     : NACK=%d ACK zaman asimi=%d pencere=%d"
              % (link.nacks, link.ack_timeouts, link.window))
        if heap_start and link.last_heap:
            dh = link.last_heap[0] - heap_start[0]
            print("Heap          : %d -> %d (%+d bayt, %%%.2f)"
                  % (heap_start[0], link.last_heap[0], dh,
                     100.0 * dh / heap_start[0]))
            print("PSRAM         : %d -> %d (%+d bayt)"
                  % (heap_start[1], link.last_heap[1],
                     link.last_heap[1] - heap_start[1]))
        return 0 if (dropped == 0 and pl_err == 0 and sync == 0) else 1
    return 1


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", help="seri port, verilmezse otomatik bulunur")
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("ports").set_defaults(func=cmd_ports)
    sub.add_parser("hello").set_defaults(func=cmd_hello)

    b = sub.add_parser("bench")
    b.add_argument("--frames", type=int, default=30)
    b.set_defaults(func=cmd_bench)

    sub.add_parser("conformance").set_defaults(func=cmd_conformance)

    e = sub.add_parser("endurance")
    e.add_argument("--minutes", type=float, default=30.0)
    e.set_defaults(func=cmd_endurance)

    args = ap.parse_args()
    sys.exit(args.func(args) or 0)


if __name__ == "__main__":
    main()
