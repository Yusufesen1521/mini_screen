//! Saat widget'i.
//!
//! Ilk gercek widget. Saniyede bir degistigi icin dirty tracking'i
//! gostermek icin de uygun: iki saniye arasinda ekranda hicbir sey
//! degismiyor ve trafik sifira dusuyor.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, Color, Rect};
use crate::widget::{Context, Widget};

const COLOR_BG: Color = Color::rgb(16, 18, 24);
const COLOR_TIME: Color = Color::rgb(232, 236, 244);
const COLOR_DATE: Color = Color::rgb(120, 130, 150);

const TIME_SIZE: f32 = 56.0;
const DATE_SIZE: f32 = 16.0;
/// Saat ile tarih arasindaki bosluk.
const GAP: u16 = 8;

const AY_ADLARI: [&str; 12] = [
    "Ocak", "Subat", "Mart", "Nisan", "Mayis", "Haziran", "Temmuz",
    "Agustos", "Eylul", "Ekim", "Kasim", "Aralik",
];

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

    fn update(&mut self, ctx: &Context) -> bool {
        // Saniye degismediyse cizmeye gerek yok. Rasterleme maliyetini
        // burada kesmek dirty tracking'den once geliyor.
        if ctx.local_hms == self.hms && ctx.local_ymd == self.ymd {
            return false;
        }
        self.hms = ctx.local_hms;
        self.ymd = ctx.local_ymd;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, _ctx: &Context) {
        canvas.fill_rect(area, COLOR_BG);

        let (h, m, s) = self.hms;
        let time = format!("{:02}:{:02}:{:02}", h, m, s);
        let (y, mo, d) = self.ymd;
        let ay = AY_ADLARI[(mo.clamp(1, 12) - 1) as usize];
        let date = format!("{} {} {}", d, ay, y);

        let tw = canvas.text_width(&time, TIME_SIZE);
        let dw = canvas.text_width(&date, DATE_SIZE);
        let block_h = TIME_SIZE as u16 + GAP + DATE_SIZE as u16;
        let top = area.y + area.h.saturating_sub(block_h) / 2;

        canvas.text(
            &time,
            area.x + area.w.saturating_sub(tw) / 2,
            top,
            TIME_SIZE,
            COLOR_TIME,
        );
        canvas.text(
            &date,
            area.x + area.w.saturating_sub(dw) / 2,
            top + TIME_SIZE as u16 + GAP,
            DATE_SIZE,
            COLOR_DATE,
        );
    }
}

register_widget!("clock", Clock);
