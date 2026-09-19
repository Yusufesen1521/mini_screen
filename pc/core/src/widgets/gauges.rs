//! Gosterge paneli: halka gostergeler ve alt bilgi seridi.
//!
//! `sysinfo` satir panelinin gorsel alternatifi. Ikisi de kayitli,
//! yerlesimde hangisi secilirse o ciziliyor.
//!
//! Tasarim: ustte en fazla uc halka (CPU, RAM, GPU), altta sigmayan
//! olcumler ince satirlar halinde. Halkanin icinde yuzde, altinda
//! sicaklik, disinda etiket.
//!
//! **Eksik olcum hic cizilmez.** GPU yoksa iki halka kalir ve ortalanir.
//! Hicbir yerde "N/A" ya da 0 yazmiyoruz.
//!
//! Maliyet notu: halkalar kenar yumusatmali vektor yolu, yani
//! rasterlemenin pahali kismi. Bu widget saniyede iki kez yeniden
//! ciziliyor, 24 FPS degil, o yuzden karsilanabiliyor. Olculen deger
//! `docs/measurements.md` icinde.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, Color, FontKind, Rect};
use crate::sensors::Snapshot;
use crate::theme;
use crate::widget::{Context, Widget};

/// Halka gostergenin bosluk birakilan alt acisi. 270 derece cizilip
/// altta 90 derecelik bosluk kaliyor.
const ARC_START_DEG: f32 = 135.0;
const ARC_SWEEP_DEG: f32 = 270.0;

const RING_RADIUS: f32 = 34.0;
const RING_WIDTH: f32 = 8.0;
/// Halka merkezinin gosterge alaninin ustunden uzakligi.
const RING_CY: u16 = 46;

const SIZE_RING_VALUE: f32 = 19.0;
const SIZE_RING_SUB: f32 = 10.0;
const SIZE_RING_LABEL: f32 = 11.0;

/// Halka icindeki yuzde ve altindaki sicaklik, merkeze gore.
const RING_VALUE_DY: i16 = -12;
const RING_SUB_DY: i16 = 8;
/// Etiket halkanin altinda.
const RING_LABEL_DY: i16 = 42;

/// Gosterge alaninin toplam yuksekligi. Etiketin altini da kapsiyor,
/// yoksa etiket alt seridin ayirici cizgisine giriyor.
const RINGS_H: u16 = 104;
/// Alt seritteki bir satirin yuksekligi.
const FOOT_ROW_H: u16 = 30;
/// Alt serit duzeni: etiket, cubuk, sagda deger. Deger cubugun ustune
/// binmesin diye cubuk dar tutuldu.
const FOOT_BAR_X: u16 = 58;
const FOOT_BAR_W: u16 = 118;
const FOOT_BAR_H: u16 = 6;
const FOOT_RIGHT_PAD: u16 = 12;
/// Satir icinde metnin ve cubugun dikey konumu.
const FOOT_TEXT_DY: u16 = 9;
const FOOT_BAR_DY: u16 = 12;

/// En fazla bu kadar halka cizilir, kalanlar alt serite duser.
const MAX_RINGS: usize = 3;

struct Gauge {
    label: &'static str,
    percent: f32,
    /// Halkanin icinde, yuzdenin altinda gorunen kucuk metin.
    sub: Option<String>,
    color: Color,
}

struct FootRow {
    label: &'static str,
    value: String,
    bar: Option<f32>,
    color: Color,
}

#[derive(Default)]
pub struct Gauges {
    last: Vec<(i16, Option<String>)>,
    last_foot: Vec<String>,
}

fn human_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        let g = n as f64 / GB as f64;
        // Uc haneli gigabaytta ondalik yer kapliyor ve bir sey
        // soylemiyor.
        if g >= 100.0 {
            return format!("{:.0}G", g);
        }
        format!("{:.1}G", g)
    } else if n >= MB {
        format!("{:.0}M", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.0}K", n as f64 / KB as f64)
    } else {
        format!("{}B", n)
    }
}

/// Halkaya cikacak olcumler. Okunamayanlar hic uretilmiyor.
fn gauges_from(s: &Snapshot) -> Vec<Gauge> {
    let mut v = Vec::new();

    if let Some(cpu) = s.cpu_percent {
        v.push(Gauge {
            label: "CPU",
            percent: cpu,
            sub: s.cpu_temp_c.map(|t| format!("{:.0}°C", t)),
            color: theme::CPU,
        });
    }
    if let Some(p) = s.mem_percent() {
        v.push(Gauge {
            label: "RAM",
            percent: p,
            sub: s.mem_used.map(human_bytes),
            color: theme::RAM,
        });
    }
    if let Some(g) = s.gpu_percent {
        v.push(Gauge {
            label: "GPU",
            percent: g,
            sub: s.gpu_temp_c.map(|t| format!("{:.0}°C", t)),
            color: theme::GPU,
        });
    }

    v.truncate(MAX_RINGS);
    v
}

/// Alt seride cikacak olcumler.
fn foot_from(s: &Snapshot) -> Vec<FootRow> {
    let mut v = Vec::new();

    if let (Some(used), Some(total)) = (s.gpu_mem_used, s.gpu_mem_total) {
        v.push(FootRow {
            label: "VRAM",
            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
            bar: s.gpu_mem_percent(),
            color: theme::GPU,
        });
    }
    if let (Some(used), Some(total)) = (s.disk_used, s.disk_total) {
        v.push(FootRow {
            label: "DISK",
            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
            bar: s.disk_percent(),
            color: theme::DISK,
        });
    }
    if let (Some(rx), Some(tx)) = (s.net_rx_bps, s.net_tx_bps) {
        v.push(FootRow {
            label: "AĞ",
            value: format!("↓{}/s  ↑{}/s", human_bytes(rx), human_bytes(tx)),
            bar: None,
            color: theme::NET,
        });
    }
    v
}

impl Widget for Gauges {
    fn kind(&self) -> &'static str {
        "gauges"
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn update(&mut self, ctx: &Context<'_>) -> bool {
        // Yuzde tam sayiya yuvarlanarak karsilastiriliyor: halkanin
        // acisi zaten piksel cozunurlugunde o kadar degisiyor.
        let g: Vec<(i16, Option<String>)> = gauges_from(ctx.sensors)
            .into_iter()
            .map(|x| (x.percent.round() as i16, x.sub))
            .collect();
        let f: Vec<String> = foot_from(ctx.sensors)
            .into_iter()
            .map(|x| x.value)
            .collect();
        if g == self.last && f == self.last_foot {
            return false;
        }
        self.last = g;
        self.last_foot = f;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, ctx: &Context<'_>) {
        canvas.fill_rect(area, theme::BG);

        let gauges = gauges_from(ctx.sensors);
        if gauges.is_empty() {
            canvas.text(
                "sensor kaynagi yok",
                area.x + theme::SCREEN_PAD,
                area.y + theme::SCREEN_PAD,
                theme::SIZE_LABEL,
                FontKind::Sans,
                theme::TEXT_FAINT,
            );
            return;
        }

        // Halkalar esit genislikte sutunlara bolunuyor; iki tane kalirsa
        // kendiliginden ortalanmis oluyor.
        let cols = gauges.len() as u16;
        let col_w = area.w / cols;
        let cy = area.y + RING_CY;

        for (i, g) in gauges.iter().enumerate() {
            let cx = area.x + col_w * i as u16 + col_w / 2;
            self.draw_ring(canvas, cx, cy, g);
        }

        // Alt serit
        let foot_top = area.y + RINGS_H;
        let right = area.x + area.w - FOOT_RIGHT_PAD;
        for (i, row) in foot_from(ctx.sensors).iter().enumerate() {
            let top = foot_top + i as u16 * FOOT_ROW_H;
            if top + FOOT_ROW_H > area.y + area.h {
                break;
            }
            canvas.fill_rect(Rect::new(area.x, top, area.w, 1), theme::DIVIDER);
            canvas.text(
                row.label,
                area.x + theme::SCREEN_PAD,
                top + FOOT_TEXT_DY,
                theme::SIZE_LABEL,
                FontKind::Sans,
                theme::TEXT_FAINT,
            );
            // Deger her zaman sagda. Cubuk varsa etiket ile deger
            // arasinda duruyor.
            canvas.text_right(
                &row.value,
                right,
                top + FOOT_TEXT_DY,
                theme::SIZE_DETAIL,
                FontKind::Mono,
                theme::TEXT_DIM,
            );
            if let Some(pct) = row.bar {
                let bar_x = area.x + FOOT_BAR_X;
                let by = top + FOOT_BAR_DY;
                canvas.fill_rect(Rect::new(bar_x, by, FOOT_BAR_W, FOOT_BAR_H), theme::TRACK);
                let filled = (FOOT_BAR_W as f32 * pct.clamp(0.0, 100.0) / 100.0) as u16;
                let color = if pct >= theme::HOT_THRESHOLD {
                    theme::HOT
                } else {
                    row.color
                };
                canvas.fill_rect(Rect::new(bar_x, by, filled, FOOT_BAR_H), color);
            }
        }
    }
}

impl Gauges {
    fn draw_ring(&self, canvas: &mut Canvas, cx: u16, cy: u16, g: &Gauge) {
        let pct = g.percent.clamp(0.0, 100.0);
        let color = if pct >= theme::HOT_THRESHOLD {
            theme::HOT
        } else {
            g.color
        };

        canvas.arc(
            cx as f32,
            cy as f32,
            RING_RADIUS,
            ARC_START_DEG,
            ARC_SWEEP_DEG,
            RING_WIDTH,
            theme::TRACK,
        );
        canvas.arc(
            cx as f32,
            cy as f32,
            RING_RADIUS,
            ARC_START_DEG,
            ARC_SWEEP_DEG * pct / 100.0,
            RING_WIDTH,
            color,
        );

        let vy = (cy as i16 + RING_VALUE_DY).max(0) as u16;
        canvas.text_center(
            &format!("{:.0}%", pct),
            cx,
            vy,
            SIZE_RING_VALUE,
            FontKind::Mono,
            theme::TEXT,
        );

        if let Some(sub) = &g.sub {
            let sy = (cy as i16 + RING_SUB_DY).max(0) as u16;
            canvas.text_center(sub, cx, sy, SIZE_RING_SUB, FontKind::Mono, theme::TEXT_DIM);
        }

        let ly = (cy as i16 + RING_LABEL_DY).max(0) as u16;
        canvas.text_center(
            g.label,
            cx,
            ly,
            SIZE_RING_LABEL,
            FontKind::Sans,
            theme::TEXT_FAINT,
        );
    }
}

register_widget!("gauges", Gauges);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eksik_olcum_halka_uretmiyor() {
        let s = Snapshot::default();
        assert!(gauges_from(&s).is_empty());

        let s = Snapshot {
            cpu_percent: Some(30.0),
            ..Default::default()
        };
        let g = gauges_from(&s);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].label, "CPU");
        assert!(g[0].sub.is_none(), "sicaklik yokken alt metin olmamali");
    }

    #[test]
    fn gpu_gelince_ucuncu_halka() {
        let s = Snapshot {
            cpu_percent: Some(10.0),
            mem_used: Some(4),
            mem_total: Some(8),
            gpu_percent: Some(55.0),
            ..Default::default()
        };
        let labels: Vec<_> = gauges_from(&s).iter().map(|g| g.label).collect();
        assert_eq!(labels, vec!["CPU", "RAM", "GPU"]);
    }

    /// Halka sayisi ucte sinirli, kalanlar alt serite dusmeli.
    #[test]
    fn halka_sayisi_sinirli() {
        let s = Snapshot {
            cpu_percent: Some(1.0),
            mem_used: Some(1),
            mem_total: Some(2),
            gpu_percent: Some(1.0),
            disk_used: Some(1),
            disk_total: Some(2),
            ..Default::default()
        };
        assert!(gauges_from(&s).len() <= MAX_RINGS);
        assert!(foot_from(&s).iter().any(|r| r.label == "DISK"));
    }

    #[test]
    fn sensorsuz_cizim_cokmuyor() {
        let snap = Snapshot::default();
        let ctx = Context {
            uptime: Duration::ZERO,
            local_hms: (0, 0, 0),
            local_ymd: (2026, 1, 1),
            sensors: &snap,
        };
        let mut w = Gauges::default();
        let mut c = Canvas::new(320, 198);
        w.render(&mut c, Rect::new(0, 0, 320, 198), &ctx);
    }

    #[test]
    fn ayni_veride_yeniden_cizim_yok() {
        let snap = Snapshot {
            cpu_percent: Some(40.0),
            ..Default::default()
        };
        let ctx = Context {
            uptime: Duration::ZERO,
            local_hms: (0, 0, 0),
            local_ymd: (2026, 1, 1),
            sensors: &snap,
        };
        let mut w = Gauges::default();
        assert!(w.update(&ctx));
        assert!(!w.update(&ctx));
    }
}
