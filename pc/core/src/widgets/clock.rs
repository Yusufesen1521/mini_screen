//! Saat, baslik seridi olarak.
//!
//! Ekranin ust bandini kapliyor: solda buyuk saat, sagda gun ve tarih.
//! Saat monospace ciziliyor, cunku oranti fontunda rakam genislikleri
//! farkli oldugu icin sayi her saniye yatay zipliyor.
//!
//! Turkce karakterler diyakritikli yaziliyor. Faz 3'un "Turkce
//! karakterler dogru goruntuleniyor" kriterini simdiden zorluyor.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, FontKind, Rect};
use crate::theme;
use crate::widget::{Context, Widget};

/// Saat ile saniye arasindaki bosluk.
const SEC_GAP: u16 = 4;
/// Saniye, saat ve dakikadan kucuk. Boylece goz once saate gidiyor.
const SIZE_SECONDS: f32 = 13.0;
/// Baslik altindaki vurgu cizgisi.
const RULE_H: u16 = 2;

const GUNLER: [&str; 7] = [
    "Pazartesi",
    "Salı",
    "Çarşamba",
    "Perşembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];

const AYLAR: [&str; 12] = [
    "Ocak",
    "Şubat",
    "Mart",
    "Nisan",
    "Mayıs",
    "Haziran",
    "Temmuz",
    "Ağustos",
    "Eylül",
    "Ekim",
    "Kasım",
    "Aralık",
];

/// Zeller benzeri basit hafta gunu hesabi. 1 Ocak 2000 Cumartesi.
fn weekday_index(y: i32, m: u8, d: u8) -> usize {
    let (mut y, mut m) = (y, m as i32);
    if m < 3 {
        y -= 1;
        m += 12;
    }
    let k = y % 100;
    let j = y / 100;
    let h = (d as i32 + 13 * (m + 1) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // Zeller 0 = Cumartesi. Bizim dizi Pazartesi ile basliyor.
    ((h + 5) % 7) as usize
}

#[derive(Default)]
pub struct Clock {
    hms: (u8, u8, u8),
    ymd: (i32, u8, u8),
}

impl Widget for Clock {
    fn kind(&self) -> &'static str {
        "clock"
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(200)
    }

    fn update(&mut self, ctx: &Context<'_>) -> bool {
        if ctx.local_hms == self.hms && ctx.local_ymd == self.ymd {
            return false;
        }
        self.hms = ctx.local_hms;
        self.ymd = ctx.local_ymd;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, _ctx: &Context<'_>) {
        canvas.fill_rect(area, theme::BG_HEADER);
        canvas.fill_rect(
            Rect::new(area.x, area.y + area.h - RULE_H, area.w, RULE_H),
            theme::ACCENT,
        );

        let (h, m, s) = self.hms;
        let (y, mo, d) = self.ymd;

        // Saat ve dakika buyuk, saniye kucuk ve soluk.
        let hhmm = format!("{:02}:{:02}", h, m);
        let baseline = area.y + 6;
        let end = canvas.text(
            &hhmm,
            area.x + theme::SCREEN_PAD,
            baseline,
            theme::SIZE_CLOCK,
            FontKind::Mono,
            theme::TEXT,
        );
        canvas.text(
            &format!("{:02}", s),
            end + SEC_GAP,
            baseline + (theme::SIZE_CLOCK - SIZE_SECONDS) as u16 - 4,
            SIZE_SECONDS,
            FontKind::Mono,
            theme::TEXT_DIM,
        );

        // Sagda gun ve tarih, iki satir.
        let right = area.x + area.w - theme::SCREEN_PAD;
        let gun = GUNLER[weekday_index(y, mo, d)];
        let ay = AYLAR[(mo.clamp(1, 12) - 1) as usize];
        canvas.text_right(
            gun,
            right,
            area.y + 6,
            theme::SIZE_WEEKDAY,
            FontKind::Sans,
            theme::TEXT_FAINT,
        );
        canvas.text_right(
            &format!("{} {} {}", d, ay, y),
            right,
            area.y + 20,
            theme::SIZE_DATE,
            FontKind::Sans,
            theme::TEXT_DIM,
        );
    }
}

register_widget!("clock", Clock);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hafta_gunu_dogru() {
        // 19 Eylul 2026 Cumartesi.
        assert_eq!(GUNLER[weekday_index(2026, 9, 19)], "Cumartesi");
        // 1 Ocak 2000 Cumartesi.
        assert_eq!(GUNLER[weekday_index(2000, 1, 1)], "Cumartesi");
        // 29 Subat 2024 Persembe, artik yil kontrolu.
        assert_eq!(GUNLER[weekday_index(2024, 2, 29)], "Perşembe");
    }
}
