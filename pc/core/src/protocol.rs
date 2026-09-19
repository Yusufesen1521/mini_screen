//! mini_screen USB protokolu, PC tarafi.
//!
//! Bayt duzeyinde tanim: `docs/protocol.md`
//! Cihaz tarafindaki karsiligi: `include/protocol.h`
//! Python referans uygulamasi: `tools/protocol.py`
//!
//! Bu modul sadece protokolu bilir, seri port acmaz.

pub const VERSION: u8 = 1;

pub const SOF: [u8; 2] = [0xA5, 0x5A];
pub const HDR_SIZE: usize = 10;
pub const CRC_SIZE: usize = 2;
pub const MAX_PAYLOAD: usize = 65535;

pub const FLAG_ACK_REQ: u8 = 0x01;

// PC -> cihaz
pub const MSG_HELLO: u8 = 0x01;
pub const MSG_FRAME_REGION: u8 = 0x10;
pub const MSG_SET_BACKLIGHT: u8 = 0x20;
pub const MSG_PING: u8 = 0x30;
pub const MSG_GET_STATUS: u8 = 0x40;

// Cihaz -> PC
pub const MSG_CAPS: u8 = 0x81;
pub const MSG_ACK: u8 = 0x82;
pub const MSG_NACK: u8 = 0x83;
pub const MSG_ERROR: u8 = 0x84;
pub const MSG_STATUS: u8 = 0x85;
pub const MSG_PONG: u8 = 0x86;
pub const MSG_LOG: u8 = 0x87;

pub const REGION_HDR_SIZE: usize = 10;
pub const PIXFMT_RGB565_LE: u8 = 0x01;
pub const CODEC_NONE: u8 = 0x00;
pub const CODEC_RLE16: u8 = 0x01;

// RLE16 sinirlari. Cihaz tarafindaki cozucu bu degerlere gore yazildi,
// degistirilirse iki taraf birlikte degismek zorunda.
pub const MAX_RUN: usize = 129;
pub const MAX_LITERAL: usize = 128;

pub fn msg_name(t: u8) -> &'static str {
    match t {
        MSG_HELLO => "HELLO",
        MSG_FRAME_REGION => "FRAME_REGION",
        MSG_SET_BACKLIGHT => "SET_BACKLIGHT",
        MSG_PING => "PING",
        MSG_GET_STATUS => "GET_STATUS",
        MSG_CAPS => "CAPS",
        MSG_ACK => "ACK",
        MSG_NACK => "NACK",
        MSG_ERROR => "ERROR",
        MSG_STATUS => "STATUS",
        MSG_PONG => "PONG",
        MSG_LOG => "LOG",
        _ => "BILINMEYEN",
    }
}

pub fn err_name(code: u8) -> &'static str {
    match code {
        0x01 => "BAD_HEADER_CRC",
        0x02 => "BAD_PAYLOAD_CRC",
        0x03 => "BAD_VERSION",
        0x04 => "UNKNOWN_TYPE",
        0x05 => "PAYLOAD_TOO_LARGE",
        0x06 => "BAD_REGION",
        0x07 => "BAD_CODEC",
        0x08 => "DECODE_ERROR",
        0x09 => "OVERRUN",
        _ => "BILINMEYEN",
    }
}

// ---------------------------------------------------------------------------
// CRC
// ---------------------------------------------------------------------------

/// Polinom 0x07, baslangic 0x00. "123456789" -> 0xF4
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

const CRC16_TABLE: [u16; 256] = build_crc16_table();

const fn build_crc16_table() -> [u16; 256] {
    let mut t = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut v = (i as u16) << 8;
        let mut k = 0;
        while k < 8 {
            v = if v & 0x8000 != 0 {
                (v << 1) ^ 0x1021
            } else {
                v << 1
            };
            k += 1;
        }
        t[i] = v;
        i += 1;
    }
    t
}

/// CRC-16/CCITT-FALSE. "123456789" -> 0x29B1
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc = (crc << 8) ^ CRC16_TABLE[((crc >> 8) as u8 ^ b) as usize];
    }
    crc
}

// ---------------------------------------------------------------------------
// Cerceve
// ---------------------------------------------------------------------------

/// Cerceve olusturur ve `out` icine yazar.
///
/// Tampon disaridan veriliyor ki sicak yolda her karede yeni tahsis
/// yapilmasin. Olcum bu yolun kare basina 0.08 ms oldugunu gosterdi,
/// tahsis eklemek bosuna.
pub fn build_frame_into(out: &mut Vec<u8>, msg_type: u8, seq: u8, payload: &[u8], flags: u8) {
    debug_assert!(payload.len() <= MAX_PAYLOAD);
    out.clear();
    out.extend_from_slice(&SOF);

    let len = payload.len() as u16;
    let hdr = [
        VERSION,
        msg_type,
        flags,
        seq,
        (len & 0xFF) as u8,
        (len >> 8) as u8,
        0, // rezerve
    ];
    out.extend_from_slice(&hdr);
    out.push(crc8(&hdr));
    out.extend_from_slice(payload);
    out.extend_from_slice(&crc16(payload).to_le_bytes());
}

pub fn build_frame(msg_type: u8, seq: u8, payload: &[u8], flags: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(HDR_SIZE + payload.len() + CRC_SIZE);
    build_frame_into(&mut v, msg_type, seq, payload, flags);
    v
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub version: u8,
    pub msg_type: u8,
    pub flags: u8,
    pub seq: u8,
    pub payload: Vec<u8>,
    pub crc_ok: bool,
}

/// Cihazdan gelen cerceveleri ayristirir.
///
/// Cihaz tarafiyla ayni kaydirmali pencere yaklasimi: baslik CRC'si
/// tutmayan her hizalama atlanir, boylece coplu bir akistan sonra
/// kendini toparlar.
#[derive(Default)]
pub struct Parser {
    buf: Vec<u8>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
        }
    }

    pub fn feed(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        self.buf.extend_from_slice(data);
        while let Some(f) = self.try_extract() {
            out.push(f);
        }
    }

    fn try_extract(&mut self) -> Option<Frame> {
        if self.buf.len() < HDR_SIZE {
            return None;
        }
        for i in 0..=(self.buf.len() - HDR_SIZE) {
            if self.buf[i] != SOF[0] || self.buf[i + 1] != SOF[1] {
                continue;
            }
            let hdr = &self.buf[i + 2..i + 9];
            if crc8(hdr) != self.buf[i + 9] {
                continue;
            }
            let version = hdr[0];
            let msg_type = hdr[1];
            let flags = hdr[2];
            let seq = hdr[3];
            let length = u16::from_le_bytes([hdr[4], hdr[5]]) as usize;

            let total = HDR_SIZE + length + CRC_SIZE;
            if self.buf.len() - i < total {
                // Cerceve henuz tamamlanmadi. Onundeki copu at, gerisini bekle.
                self.buf.drain(..i);
                return None;
            }
            let payload = self.buf[i + HDR_SIZE..i + HDR_SIZE + length].to_vec();
            let got = u16::from_le_bytes([
                self.buf[i + HDR_SIZE + length],
                self.buf[i + HDR_SIZE + length + 1],
            ]);
            let crc_ok = got == crc16(&payload);
            self.buf.drain(..i + total);
            return Some(Frame {
                version,
                msg_type,
                flags,
                seq,
                payload,
                crc_ok,
            });
        }
        // Hicbir hizalama tutmadi. Son HDR_SIZE-1 bayti sakla, gerisi cop.
        if self.buf.len() > HDR_SIZE {
            let keep = HDR_SIZE - 1;
            let cut = self.buf.len() - keep;
            self.buf.drain(..cut);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// RLE16
// ---------------------------------------------------------------------------

/// `tools/protocol.py` icindeki `rle16_encode` ile birebir ayni semantik.
/// Ikisi de ayni baytlari uretmek zorunda, testte dogrulaniyor.
pub fn rle16_encode_into(px: &[u16], out: &mut Vec<u8>) {
    out.clear();
    let n = px.len();
    let mut i = 0usize;
    while i < n {
        let mut run = 1usize;
        while i + run < n && px[i + run] == px[i] && run < MAX_RUN {
            run += 1;
        }
        if run >= 2 {
            out.push(0x80 | (run - 2) as u8);
            out.extend_from_slice(&px[i].to_le_bytes());
            i += run;
            continue;
        }
        // Duz kosu: uc ayni piksel gorunce kes, orada tekrar kosusu daha ucuz
        let start = i;
        while i < n && (i - start) < MAX_LITERAL {
            if i + 2 < n && px[i] == px[i + 1] && px[i + 1] == px[i + 2] {
                break;
            }
            i += 1;
        }
        let count = i - start;
        out.push((count - 1) as u8);
        for &p in &px[start..i] {
            out.extend_from_slice(&p.to_le_bytes());
        }
    }
}

/// Cihaz tarafiyla ayni cozucu. Kendi kodlayicimizi dogrulamak icin.
pub fn rle16_decode(data: &[u8], max_pixels: usize) -> Result<Vec<u16>, &'static str> {
    let mut out = Vec::with_capacity(max_pixels);
    let mut i = 0usize;
    while i < data.len() {
        let control = data[i];
        i += 1;
        if control < 0x80 {
            let count = control as usize + 1;
            if i + count * 2 > data.len() {
                return Err("eksik kaynak");
            }
            if out.len() + count > max_pixels {
                return Err("hedef tasmasi");
            }
            for k in 0..count {
                out.push(u16::from_le_bytes([data[i + k * 2], data[i + k * 2 + 1]]));
            }
            i += count * 2;
        } else {
            let count = (control & 0x7F) as usize + 2;
            if i + 2 > data.len() {
                return Err("eksik kaynak");
            }
            if out.len() + count > max_pixels {
                return Err("hedef tasmasi");
            }
            let pixel = u16::from_le_bytes([data[i], data[i + 1]]);
            i += 2;
            out.extend(std::iter::repeat_n(pixel, count));
        }
    }
    Ok(out)
}

/// Bolge payload'i uretir. Sikistirma kazandirmiyorsa ham gonderir.
///
/// Donen deger secilen codec. Payload `out` icinde.
pub fn encode_region_into(
    out: &mut Vec<u8>,
    scratch: &mut Vec<u8>,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    pixels: &[u16],
) -> u8 {
    debug_assert_eq!(pixels.len(), w as usize * h as usize);
    rle16_encode_into(pixels, scratch);
    let raw_len = pixels.len() * 2;
    let codec = if scratch.len() < raw_len {
        CODEC_RLE16
    } else {
        CODEC_NONE
    };

    out.clear();
    out.reserve(REGION_HDR_SIZE + raw_len);
    out.extend_from_slice(&x.to_le_bytes());
    out.extend_from_slice(&y.to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.push(PIXFMT_RGB565_LE);
    out.push(codec);

    if codec == CODEC_RLE16 {
        out.extend_from_slice(scratch);
    } else {
        for &p in pixels {
            out.extend_from_slice(&p.to_le_bytes());
        }
    }
    codec
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cihazin acilistaki kendini sinamasi da bu iki vektoru kullaniyor.
    #[test]
    fn crc_bilinen_vektorler() {
        assert_eq!(crc8(b"123456789"), 0xF4);
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn cerceve_git_gel() {
        let payload = b"merhaba dunya";
        let frame = build_frame(MSG_HELLO, 7, payload, FLAG_ACK_REQ);
        let mut p = Parser::new();
        let mut got = Vec::new();
        p.feed(&frame, &mut got);
        assert_eq!(got.len(), 1);
        let f = &got[0];
        assert_eq!(f.msg_type, MSG_HELLO);
        assert_eq!(f.seq, 7);
        assert_eq!(f.flags, FLAG_ACK_REQ);
        assert_eq!(f.payload, payload);
        assert!(f.crc_ok);
    }

    #[test]
    fn bayt_bayt_bolunmus_cerceve() {
        let frame = build_frame(MSG_PING, 1, b"abc", 0);
        let mut p = Parser::new();
        let mut got = Vec::new();
        for b in &frame {
            p.feed(&[*b], &mut got);
        }
        assert_eq!(got.len(), 1);
        assert!(got[0].crc_ok);
    }

    #[test]
    fn coptan_sonra_senkron() {
        let mut data = vec![0xFF, 0x00, 0xA5, 0x12, 0x5A, 0xA5, 0x99];
        data.extend_from_slice(&build_frame(MSG_PING, 3, b"xy", 0));
        let mut p = Parser::new();
        let mut got = Vec::new();
        p.feed(&data, &mut got);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].seq, 3);
        assert!(got[0].crc_ok);
    }

    #[test]
    fn bozuk_payload_crc_yakalaniyor() {
        let mut frame = build_frame(MSG_PING, 1, b"abcd", 0);
        let n = frame.len();
        frame[n - 1] ^= 0xFF;
        let mut p = Parser::new();
        let mut got = Vec::new();
        p.feed(&frame, &mut got);
        assert_eq!(got.len(), 1);
        assert!(!got[0].crc_ok);
    }

    #[test]
    fn rle_git_gel() {
        let mut px = vec![0x1234u16; 300];
        for (i, v) in px.iter_mut().enumerate().take(50) {
            *v = i as u16;
        }
        let mut enc = Vec::new();
        rle16_encode_into(&px, &mut enc);
        let dec = rle16_decode(&enc, px.len()).unwrap();
        assert_eq!(dec, px);
    }

    #[test]
    fn rle_duz_renk_kazanci() {
        let px = vec![0xF800u16; 76800];
        let mut enc = Vec::new();
        rle16_encode_into(&px, &mut enc);
        // Tam ekran duz renk yaklasik 1.8 KB, docs/protocol.md boyle diyor.
        assert!(enc.len() < 2048, "duz renk {} bayta cikti", enc.len());
    }

    /// Python referans uygulamasiyla ayni baytlari urettigimizin kaniti.
    ///
    /// Beklenen degerler `tools/protocol.py` icindeki `rle16_encode`
    /// calistirilarak uretildi. Iki taraftan biri degisir de digeri
    /// degismezse bu test duser. Faz 1'de ogrenilen ders: PC tarafinin
    /// urettigi ile cihazin bekledigi her zaman karsilastirilmali.
    #[test]
    fn python_referansiyla_ayni_baytlar() {
        // Iki tarafta da ayni uretilebilen basit LCG.
        let mut st: u64 = 12345;
        let mut px: Vec<u16> = Vec::with_capacity(400);
        for _ in 0..400 {
            st = (st.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7FFF_FFFF;
            if !px.is_empty() && (st >> 16) % 3 != 0 {
                px.push(*px.last().unwrap());
            } else {
                px.push(((st >> 8) & 0xFFFF) as u16);
            }
        }
        let mut enc = Vec::new();
        rle16_encode_into(&px, &mut enc);
        assert_eq!(enc.len(), 391, "kodlanmis uzunluk python ile tutmuyor");
        assert_eq!(crc16(&enc), 0x71FA, "kodlanmis icerik python ile tutmuyor");
        assert_eq!(rle16_decode(&enc, px.len()).unwrap(), px);
    }

    #[test]
    fn sikismayan_icerik_ham_gidiyor() {
        // Her piksel farkli: RLE buyutur, codec NONE secilmeli.
        let px: Vec<u16> = (0..1024u16).collect();
        let mut out = Vec::new();
        let mut scratch = Vec::new();
        let codec = encode_region_into(&mut out, &mut scratch, 0, 0, 32, 32, &px);
        assert_eq!(codec, CODEC_NONE);
        assert_eq!(out.len(), REGION_HDR_SIZE + px.len() * 2);
    }
}
