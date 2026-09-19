//! Calisma suresi widget'i.
//!
//! Faz 2 cikis kriterinin kaniti: "ikinci bir sahte widget eklemek
//! cekirdekte tek satir degisiklik gerektirmiyor". Bu dosya eklenirken
//! cekirdekte hicbir satir degismedi; `register_widget!` kendini
//! kaydediyor ve build betigi modulu buluyor.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, Color, Rect};
use crate::widget::{Context, Widget};

const COLOR_BG: Color = Color::rgb(20, 24, 30);
const COLOR_LABEL: Color = Color::rgb(120, 130, 150);
const COLOR_VALUE: Color = Color::rgb(200, 210, 224);
const COLOR_BAR: Color = Color::rgb(64, 160, 220);

const LABEL_SIZE: f32 = 13.0;
const VALUE_SIZE: f32 = 24.0;
const PAD: u16 = 8;
const BAR_HEIGHT: u16 = 4;
/// Cubuk bir dakikada bir tur atiyor, yani hareket gozle gorulur.
const BAR_PERIOD_SECS: u64 = 60;

#[derive(Default)]
pub struct Uptime {
    secs: u64,
}

impl Widget for Uptime {
    fn kind(&self) -> &'static str {
        "uptime"
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(200)
    }

    fn update(&mut self, ctx: &Context) -> bool {
        let s = ctx.uptime.as_secs();
        if s == self.secs {
            return false;
        }
        self.secs = s;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, _ctx: &Context) {
        canvas.fill_rect(area, COLOR_BG);

        canvas.text(
            "CALISMA SURESI",
            area.x + PAD,
            area.y + PAD,
            LABEL_SIZE,
            COLOR_LABEL,
        );

        let h = self.secs / 3600;
        let m = (self.secs % 3600) / 60;
        let s = self.secs % 60;
        let text = format!("{:02}:{:02}:{:02}", h, m, s);
        canvas.text(
            &text,
            area.x + PAD,
            area.y + PAD + LABEL_SIZE as u16 + PAD,
            VALUE_SIZE,
            COLOR_VALUE,
        );

        // Dakika icindeki ilerleme
        let inner = area.w.saturating_sub(PAD * 2);
        let filled =
            (inner as u64 * (self.secs % BAR_PERIOD_SECS) / BAR_PERIOD_SECS) as u16;
        let bar_y = area.y + area.h.saturating_sub(PAD + BAR_HEIGHT);
        canvas.fill_rect(
            Rect::new(area.x + PAD, bar_y, inner, BAR_HEIGHT),
            Color::rgb(38, 42, 54),
        );
        canvas.fill_rect(
            Rect::new(area.x + PAD, bar_y, filled, BAR_HEIGHT),
            COLOR_BAR,
        );
    }
}

register_widget!("uptime", Uptime);
