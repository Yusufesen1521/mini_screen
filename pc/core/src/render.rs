//! Offscreen rasterleme.
//!
//! Tek bir rasterleyici var ve ciktisi hem cihaza hem onizlemeye gidiyor.
//! Faz 4'un "onizleme ile cihazdaki goruntu birebir ayni, fark yuzde 0"
//! kriteri bunu zorunlu kiliyor: onizleme kendi yolundan cizerse o kriter
//! hicbir zaman tutmaz.
//!
//! Olcum notu (`docs/measurements.md`): metin, dikdortgen ve cubuk cizmek
//! pratikte bedava, kare basina 0.02 ms. Pahali olan tek sey kenar
//! yumusatmali vektor yolu. `anti_alias` bu yuzden varsayilan olarak
//! kapali ve acmak bilincli bir karar olmali.

use tiny_skia::{Paint, PathBuilder, Pixmap, Rect as SkRect, Stroke, Transform};

/// Ekran ustunde bir dikdortgen. Olculer piksel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn pixel_count(&self) -> usize {
        self.w as usize * self.h as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    fn to_sk(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, 255)
    }
}

/// Glif onbellekli metin cizici.
///
/// Her karede yeniden rasterlemek anlamsiz olurdu; onbellek olcumde
/// 27 girisle doyuyor ve metin maliyetini kare basina 0.004 ms'e indiriyor.
struct TextRenderer {
    font: fontdue::Font,
    cache: std::collections::HashMap<(char, u32), (fontdue::Metrics, Vec<u8>)>,
}

/// Gomulu font yok: sistem fontu araniyor.
///
/// Faz 4'un piksel karsilastirmasi ayni makinede ayni cekirdegi
/// kullandigi icin bu yeterli. Makineler arasi birebir ayni goruntu
/// gerekirse font gomulmeli, o zaman lisans da secilmeli.
const FONT_CANDIDATES: &[&str] = &[
    "C:/Windows/Fonts/consola.ttf",
    "C:/Windows/Fonts/segoeui.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
];

impl TextRenderer {
    fn load() -> Option<TextRenderer> {
        for path in FONT_CANDIDATES {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
                {
                    return Some(TextRenderer {
                        font,
                        cache: std::collections::HashMap::new(),
                    });
                }
            }
        }
        None
    }
}

/// Cizim yuzeyi. Widget'lar sadece bunu gorur, tiny-skia disari sizmaz.
pub struct Canvas {
    pm: Pixmap,
    text: Option<TextRenderer>,
    width: u16,
    height: u16,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Canvas {
        let pm = Pixmap::new(width as u32, height as u32).expect("gecersiz tuval olcusu");
        Canvas {
            pm,
            text: TextRenderer::load(),
            width,
            height,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Font bulunamadiysa metin cizilemez. Cagiran bunu bilmeli, sessizce
    /// bos ekran gostermek hatayi gizler.
    pub fn has_font(&self) -> bool {
        self.text.is_some()
    }

    pub fn clear(&mut self, c: Color) {
        self.pm.fill(c.to_sk());
    }

    pub fn fill_rect(&mut self, r: Rect, c: Color) {
        if r.w == 0 || r.h == 0 {
            return;
        }
        let Some(rect) = SkRect::from_xywh(r.x as f32, r.y as f32, r.w as f32, r.h as f32) else {
            return;
        };
        let mut paint = Paint {
            anti_alias: false,
            ..Default::default()
        };
        paint.set_color(c.to_sk());
        self.pm.fill_rect(rect, &paint, Transform::identity(), None);
    }

    /// Cizgi. Kenar yumusatma kapali, gerekcesi modul basinda.
    pub fn line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, width: f32, c: Color) {
        let mut pb = PathBuilder::new();
        pb.move_to(x0 as f32, y0 as f32);
        pb.line_to(x1 as f32, y1 as f32);
        let Some(path) = pb.finish() else { return };
        let mut paint = Paint {
            anti_alias: false,
            ..Default::default()
        };
        paint.set_color(c.to_sk());
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        self.pm
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    /// Metin cizer ve bittigi x konumunu doner.
    ///
    /// `y` metnin ust kenari. Font yoksa hicbir sey cizilmez ve `x` doner.
    pub fn text(&mut self, s: &str, x: u16, y: u16, size: f32, c: Color) -> u16 {
        let Some(tr) = self.text.as_mut() else {
            return x;
        };
        let key_size = (size * 10.0) as u32;
        let mut pen = x as i32;
        let data = self.pm.data_mut();
        for ch in s.chars() {
            let (m, bitmap) = tr
                .cache
                .entry((ch, key_size))
                .or_insert_with(|| tr.font.rasterize(ch, size));
            for gy in 0..m.height {
                let py = y as i32 + gy as i32 - m.height as i32 - m.ymin + size as i32;
                if py < 0 || py >= self.height as i32 {
                    continue;
                }
                for gx in 0..m.width {
                    let px = pen + gx as i32 + m.xmin;
                    if px < 0 || px >= self.width as i32 {
                        continue;
                    }
                    let a = bitmap[gy * m.width + gx] as u32;
                    if a == 0 {
                        continue;
                    }
                    let o = (py as usize * self.width as usize + px as usize) * 4;
                    // Kaynak ustu alfa harmanlama, hedef premultiplied.
                    for (k, src) in [c.r, c.g, c.b].iter().enumerate() {
                        let dst = data[o + k] as u32;
                        data[o + k] = ((*src as u32 * a + dst * (255 - a)) / 255) as u8;
                    }
                    data[o + 3] = 255;
                }
            }
            pen += m.advance_width as i32;
        }
        pen.clamp(0, self.width as i32) as u16
    }

    /// Metnin cizilmeden genisligini olcer. Tasma kontrolu icin.
    pub fn text_width(&mut self, s: &str, size: f32) -> u16 {
        let Some(tr) = self.text.as_mut() else {
            return 0;
        };
        let key_size = (size * 10.0) as u32;
        let mut w = 0f32;
        for ch in s.chars() {
            let (m, _) = tr
                .cache
                .entry((ch, key_size))
                .or_insert_with(|| tr.font.rasterize(ch, size));
            w += m.advance_width;
        }
        w as u16
    }

    /// Tuvali RGB565'e cevirir. `dst` tuval piksel sayisi kadar olmali.
    pub fn to_rgb565(&self, dst: &mut [u16]) {
        let src = self.pm.data();
        debug_assert_eq!(dst.len() * 4, src.len());
        for (i, d) in dst.iter_mut().enumerate() {
            let o = i * 4;
            let r = src[o] as u16;
            let g = src[o + 1] as u16;
            let b = src[o + 2] as u16;
            *d = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dolgu_ve_cevrim() {
        let mut c = Canvas::new(8, 4);
        c.clear(Color::rgb(255, 0, 0));
        let mut buf = vec![0u16; 8 * 4];
        c.to_rgb565(&mut buf);
        // Saf kirmizi RGB565'te 0xF800.
        assert!(
            buf.iter().all(|&p| p == 0xF800),
            "ilk piksel 0x{:04X}",
            buf[0]
        );
    }

    #[test]
    fn dikdortgen_sadece_kendi_alanini_boyar() {
        let mut c = Canvas::new(8, 4);
        c.clear(Color::rgb(0, 0, 0));
        c.fill_rect(Rect::new(2, 1, 3, 2), Color::rgb(0, 0, 255));
        let mut buf = vec![0u16; 8 * 4];
        c.to_rgb565(&mut buf);
        assert_eq!(buf[8 + 2], 0x001F, "mavi olmali");
        assert_eq!(buf[0], 0x0000, "disarisi siyah kalmali");
        assert_eq!(buf[8 + 5], 0x0000, "sag komsu siyah kalmali");
    }

    #[test]
    fn sifir_olculu_dikdortgen_cokmuyor() {
        let mut c = Canvas::new(8, 4);
        c.fill_rect(Rect::new(1, 1, 0, 0), Color::rgb(255, 255, 255));
    }
}
