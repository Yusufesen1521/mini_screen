//! Sistem paneli. Plan 2.5.
//!
//! Her olcum bir satir: solda etiket ve buyuk deger, sagda detay ve
//! doluluk cubugu. Her metrigin kendi rengi var, kullanici sayiyi
//! okumadan hangi satira baktigini anlasin.
//!
//! Olcum notu (`docs/measurements.md`): metin, dikdortgen ve cubuk
//! cizmek kare basina 0.02 ms. Pahali olan canli vektor grafigiydi, bu
//! yuzden burada yol cizimi yok.
//!
//! **Eksik olcum satiri hic cizilmez.** Faz 2 cikis kriteri: "GPU yoksa
//! ya da HWiNFO kapaliysa o alan temiz sekilde gizleniyor, hata
//! gostermiyor". Hicbir yerde "N/A", "---" ya da 0 yazmiyoruz; satirin
//! kendisi yok sayiliyor ve kalanlar yukari kayiyor.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, Color, FontKind, Rect};
use crate::sensors::Snapshot;
use crate::theme;
use crate::widget::{Context, Widget};

/// Satir ici dikey konumlar, satirin ust kenarina gore.
const LABEL_DY: u16 = 4;
const VALUE_DY: u16 = 15;
const DETAIL_DY: u16 = 5;
const BAR_DY: u16 = 25;

/// Cubugun bittigi yer, ekranin sag kenarindan bu kadar iceride.
const BAR_RIGHT_PAD: u16 = 12;

/// Sol kenardaki renk isareti.
const EDGE_W: u16 = 3;
const EDGE_INSET: u16 = 7;

/// Bir olcum satiri. `bar` yoksa cubuk cizilmez.
struct Row {
    label: &'static str,
    value: String,
    detail: Option<String>,
    bar: Option<f32>,
    color: Color,
}

#[derive(Default)]
pub struct SysinfoPanel {
    /// Son cizilen satirlarin ozeti. Degismediyse yeniden cizmiyoruz.
    last: Vec<(String, Option<String>, i16)>,
}

fn human_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.1}G", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.0}M", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.0}K", n as f64 / KB as f64)
    } else {
        format!("{}B", n)
    }
}

/// Goruntuden cizilecek satirlari uretir.
///
/// Okunamayan her olcum burada elenir; cizim tarafi `Option` gormez.
fn rows_from(s: &Snapshot) -> Vec<Row> {
    let mut rows = Vec::new();

    if let Some(cpu) = s.cpu_percent {
        rows.push(Row {
            label: "CPU",
            value: format!("{:.0}%", cpu),
            detail: s.cpu_temp_c.map(|t| format!("{:.0} C", t)),
            bar: Some(cpu),
            color: theme::CPU,
        });
    }

    if let (Some(used), Some(total)) = (s.mem_used, s.mem_total) {
        rows.push(Row {
            label: "RAM",
            value: format!("{:.0}%", s.mem_percent().unwrap_or(0.0)),
            detail: Some(format!("{} / {}", human_bytes(used), human_bytes(total))),
            bar: s.mem_percent(),
            color: theme::RAM,
        });
    }

    // GPU kaynagi bu makinede yok, satirlar hic uretilmiyor.
    if let Some(g) = s.gpu_percent {
        rows.push(Row {
            label: "GPU",
            value: format!("{:.0}%", g),
            detail: s.gpu_temp_c.map(|t| format!("{:.0} C", t)),
            bar: Some(g),
            color: theme::CPU,
        });
    }
    if let (Some(used), Some(total)) = (s.gpu_mem_used, s.gpu_mem_total) {
        rows.push(Row {
            label: "VRAM",
            value: format!("{:.0}%", s.gpu_mem_percent().unwrap_or(0.0)),
            detail: Some(format!("{} / {}", human_bytes(used), human_bytes(total))),
            bar: s.gpu_mem_percent(),
            color: theme::RAM,
        });
    }

    if let (Some(used), Some(total)) = (s.disk_used, s.disk_total) {
        rows.push(Row {
            label: "DISK",
            value: format!("{:.0}%", s.disk_percent().unwrap_or(0.0)),
            detail: Some(format!("{} / {}", human_bytes(used), human_bytes(total))),
            bar: s.disk_percent(),
            color: theme::DISK,
        });
    }

    if let (Some(rx), Some(tx)) = (s.net_rx_bps, s.net_tx_bps) {
        rows.push(Row {
            label: "AĞ",
            value: format!("↓{}/s", human_bytes(rx)),
            detail: Some(format!("↑{}/s", human_bytes(tx))),
            bar: None,
            color: theme::NET,
        });
    }

    rows
}

impl Widget for SysinfoPanel {
    fn kind(&self) -> &'static str {
        "sysinfo"
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn update(&mut self, ctx: &Context<'_>) -> bool {
        // Cubuk yuzdesi tam sayiya yuvarlanarak karsilastiriliyor:
        // yuzde 41.3 ile 41.4 ekranda ayni piksellere dusuyor.
        let now: Vec<(String, Option<String>, i16)> = rows_from(ctx.sensors)
            .into_iter()
            .map(|r| {
                let b = r.bar.map(|v| v.round() as i16).unwrap_or(-1);
                (r.value, r.detail, b)
            })
            .collect();
        if now == self.last {
            return false;
        }
        self.last = now;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, ctx: &Context<'_>) {
        canvas.fill_rect(area, theme::BG);

        let rows = rows_from(ctx.sensors);
        if rows.is_empty() {
            // Hicbir sensor yoksa durumu soyluyoruz. Hata mesaji degil.
            canvas.text(
                "sensör kaynağı yok",
                area.x + theme::SCREEN_PAD,
                area.y + theme::SCREEN_PAD,
                theme::SIZE_LABEL,
                FontKind::Sans,
                theme::TEXT_FAINT,
            );
            return;
        }

        let right = area.x + area.w - BAR_RIGHT_PAD;
        let bar_x = area.x + theme::BAR_X;
        let bar_w = right.saturating_sub(bar_x);

        for (i, row) in rows.iter().enumerate() {
            let top = area.y + i as u16 * theme::ROW_H;
            if top + theme::ROW_H > area.y + area.h {
                break;
            }
            // Satirlar arasi ince ayirici, ilki haric.
            if i > 0 {
                canvas.fill_rect(Rect::new(area.x, top, area.w, 1), theme::DIVIDER);
            }
            // Sol kenarda metrigin rengi. Cubuk ince kaldigi icin renk
            // kodlamasi tek basina cubuktan okunmuyordu.
            canvas.fill_rect(
                Rect::new(area.x, top + EDGE_INSET, EDGE_W, theme::ROW_H - EDGE_INSET * 2),
                row.color,
            );

            canvas.text(
                row.label,
                area.x + theme::SCREEN_PAD,
                top + LABEL_DY,
                theme::SIZE_LABEL,
                FontKind::Sans,
                theme::TEXT_FAINT,
            );
            canvas.text(
                &row.value,
                area.x + theme::SCREEN_PAD,
                top + VALUE_DY,
                theme::SIZE_VALUE,
                FontKind::Mono,
                theme::TEXT,
            );

            if let Some(detail) = &row.detail {
                // Detay da monospace: hepsi sayi ve ok karakteri
                // oranti fontunda kucuk puntoda okunmuyor.
                canvas.text_right(
                    detail,
                    right,
                    top + DETAIL_DY,
                    theme::SIZE_DETAIL,
                    FontKind::Mono,
                    theme::TEXT_DIM,
                );
            }

            if let Some(pct) = row.bar {
                if bar_w > 0 {
                    let by = top + BAR_DY;
                    canvas.fill_rect(Rect::new(bar_x, by, bar_w, theme::BAR_H), theme::TRACK);
                    let filled = (bar_w as f32 * pct.clamp(0.0, 100.0) / 100.0) as u16;
                    let color = if pct >= theme::HOT_THRESHOLD {
                        theme::HOT
                    } else {
                        row.color
                    };
                    canvas.fill_rect(Rect::new(bar_x, by, filled, theme::BAR_H), color);
                }
            }
        }
    }
}

register_widget!("sysinfo", SysinfoPanel);

#[cfg(test)]
mod tests {
    use super::*;

    /// Cikis kriteri: eksik kaynak widget'i bozmuyor, alan gizleniyor.
    #[test]
    fn eksik_olcumler_satir_uretmiyor() {
        let s = Snapshot::default();
        assert!(rows_from(&s).is_empty(), "bos goruntuden satir cikti");

        let s = Snapshot {
            cpu_percent: Some(42.0),
            ..Default::default()
        };
        let rows = rows_from(&s);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "CPU");
        assert!(rows[0].detail.is_none(), "sicaklik yokken detay olmamali");
        assert!(!rows.iter().any(|r| r.label == "GPU"));
    }

    #[test]
    fn gpu_gelince_satir_ekleniyor() {
        let s = Snapshot {
            cpu_percent: Some(10.0),
            gpu_percent: Some(55.0),
            ..Default::default()
        };
        let labels: Vec<_> = rows_from(&s).iter().map(|r| r.label).collect();
        assert_eq!(labels, vec!["CPU", "GPU"]);
    }

    #[test]
    fn sicaklik_varsa_detaya_giriyor() {
        let mut s = Snapshot {
            cpu_percent: Some(30.0),
            ..Default::default()
        };
        assert_eq!(rows_from(&s)[0].detail, None);
        s.cpu_temp_c = Some(61.0);
        assert_eq!(rows_from(&s)[0].detail.as_deref(), Some("61 C"));
    }

    /// Turkce karakterler ve oklar kaynak dosyada dogru duruyor mu.
    #[test]
    fn turkce_etiketler() {
        let s = Snapshot {
            net_rx_bps: Some(1024),
            net_tx_bps: Some(512),
            ..Default::default()
        };
        let r = &rows_from(&s)[0];
        assert_eq!(r.label, "AĞ");
        assert!(r.value.starts_with('↓'), "deger: {}", r.value);
    }

    #[test]
    fn hicbir_sensor_yokken_cizim_cokmuyor() {
        let snap = Snapshot::default();
        let ctx = Context {
            uptime: Duration::ZERO,
            local_hms: (0, 0, 0),
            local_ymd: (2026, 1, 1),
            sensors: &snap,
        };
        let mut w = SysinfoPanel::default();
        let mut c = Canvas::new(320, 172);
        w.render(&mut c, Rect::new(0, 0, 320, 172), &ctx);
    }

    #[test]
    fn insan_okunur_bayt() {
        assert_eq!(human_bytes(512), "512B");
        assert_eq!(human_bytes(2048), "2K");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5M");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0G");
    }

    #[test]
    fn ayni_degerler_yeniden_cizim_istemiyor() {
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
        let mut w = SysinfoPanel::default();
        assert!(w.update(&ctx), "ilk turda cizim istenmeliydi");
        assert!(!w.update(&ctx), "ayni veriyle tekrar cizim istendi");
    }
}
