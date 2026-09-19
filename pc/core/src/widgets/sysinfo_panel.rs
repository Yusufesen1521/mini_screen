//! Sistem paneli. Plan 2.5.
//!
//! Sayilar ve cubuklar. Olcum notu (`docs/measurements.md`): metin,
//! dikdortgen ve cubuk cizmek kare basina 0.02 ms, yani bu widget'in
//! rasterleme maliyeti butcede gorunmuyor. Pahali olan canli vektor
//! grafigiydi, bu yuzden burada yol cizimi yok.
//!
//! **Eksik olcum satiri hic cizilmez.** Faz 2 cikis kriteri: "GPU yoksa
//! ya da HWiNFO kapaliysa o alan temiz sekilde gizleniyor, hata
//! gostermiyor". Bu yuzden hicbir yerde "N/A", "---" ya da 0 yazmiyoruz;
//! satirin kendisi yok sayiliyor ve kalan satirlar yukari kayiyor.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, Color, Rect};
use crate::sensors::Snapshot;
use crate::widget::{Context, Widget};

const COLOR_BG: Color = Color::rgb(20, 24, 30);
const COLOR_LABEL: Color = Color::rgb(130, 140, 158);
const COLOR_VALUE: Color = Color::rgb(224, 230, 240);
const COLOR_TRACK: Color = Color::rgb(38, 42, 54);
const COLOR_BAR: Color = Color::rgb(64, 160, 220);
/// Yuksek kullanimda cubuk rengi degisiyor, sayiya bakmadan anlasilsin.
const COLOR_BAR_HOT: Color = Color::rgb(220, 110, 70);
const HOT_THRESHOLD: f32 = 85.0;

const LABEL_SIZE: f32 = 12.0;
const VALUE_SIZE: f32 = 12.0;
const PAD: u16 = 8;
const ROW_HEIGHT: u16 = 22;
const LABEL_W: u16 = 42;
const VALUE_W: u16 = 62;
const BAR_HEIGHT: u16 = 6;

/// En fazla bu kadar satir cizilir. Alan yetmezse fazlasi atlanir,
/// tasma yerine kirpma tercih ediliyor.
const MAX_ROWS: usize = 6;

/// Bir olcum satiri. `bar` yoksa sadece deger yazilir.
struct Row {
    label: &'static str,
    value: String,
    bar: Option<f32>,
}

#[derive(Default)]
pub struct SysinfoPanel {
    /// Son cizilen satirlarin ozeti. Degismediyse yeniden cizmiyoruz.
    last: Vec<(String, u8)>,
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

fn human_rate(bps: u64) -> String {
    format!("{}/s", human_bytes(bps))
}

/// Goruntuden cizilecek satirlari uretir.
///
/// Okunamayan her olcum burada elenir; cizim tarafi `Option` gormez.
fn rows_from(s: &Snapshot) -> Vec<Row> {
    let mut rows = Vec::new();

    if let Some(cpu) = s.cpu_percent {
        let value = match s.cpu_temp_c {
            Some(t) => format!("{:.0}%  {:.0}C", cpu, t),
            None => format!("{:.0}%", cpu),
        };
        rows.push(Row { label: "CPU", value, bar: Some(cpu) });
    }

    if let (Some(used), Some(total)) = (s.mem_used, s.mem_total) {
        rows.push(Row {
            label: "RAM",
            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
            bar: s.mem_percent(),
        });
    }

    // GPU: bu makinede kaynak yok, satir hic cizilmiyor. Kriterin
    // gorunur kaniti bu.
    if let Some(g) = s.gpu_percent {
        let value = match s.gpu_temp_c {
            Some(t) => format!("{:.0}%  {:.0}C", g, t),
            None => format!("{:.0}%", g),
        };
        rows.push(Row { label: "GPU", value, bar: Some(g) });
    }
    if let (Some(used), Some(total)) = (s.gpu_mem_used, s.gpu_mem_total) {
        rows.push(Row {
            label: "VRAM",
            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
            bar: s.gpu_mem_percent(),
        });
    }

    if let (Some(used), Some(total)) = (s.disk_used, s.disk_total) {
        rows.push(Row {
            label: "DISK",
            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
            bar: s.disk_percent(),
        });
    }

    if let (Some(rx), Some(tx)) = (s.net_rx_bps, s.net_tx_bps) {
        rows.push(Row {
            label: "AG",
            value: format!("{} {}", human_rate(rx), human_rate(tx)),
            bar: None,
        });
    }

    rows.truncate(MAX_ROWS);
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
        // yuzde 41.3 ile 41.4 ekranda ayni piksellere dusuyor, yeniden
        // cizmek bosuna is olur.
        let now: Vec<(String, u8)> = rows_from(ctx.sensors)
            .into_iter()
            .map(|r| (r.value, r.bar.unwrap_or(-1.0).round().clamp(-1.0, 100.0) as i8 as u8))
            .collect();
        if now == self.last {
            return false;
        }
        self.last = now;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, ctx: &Context<'_>) {
        canvas.fill_rect(area, COLOR_BG);

        let rows = rows_from(ctx.sensors);
        if rows.is_empty() {
            // Hicbir sensor yoksa bos bir kutu gostermek yerine durumu
            // soyluyoruz. Bu bir hata mesaji degil, durum bilgisi.
            canvas.text(
                "sensor kaynagi yok",
                area.x + PAD,
                area.y + PAD,
                LABEL_SIZE,
                COLOR_LABEL,
            );
            return;
        }

        let bar_x = area.x + PAD + LABEL_W + VALUE_W;
        let bar_w = area
            .w
            .saturating_sub(PAD * 2 + LABEL_W + VALUE_W);

        for (i, row) in rows.iter().enumerate() {
            let y = area.y + PAD + i as u16 * ROW_HEIGHT;
            if y + ROW_HEIGHT > area.y + area.h {
                break;
            }

            canvas.text(row.label, area.x + PAD, y, LABEL_SIZE, COLOR_LABEL);
            canvas.text(&row.value, area.x + PAD + LABEL_W, y, VALUE_SIZE, COLOR_VALUE);

            if let Some(pct) = row.bar {
                if bar_w > 0 {
                    let by = y + (LABEL_SIZE as u16).saturating_sub(BAR_HEIGHT) / 2 + 3;
                    canvas.fill_rect(Rect::new(bar_x, by, bar_w, BAR_HEIGHT), COLOR_TRACK);
                    let filled =
                        (bar_w as f32 * pct.clamp(0.0, 100.0) / 100.0) as u16;
                    let color = if pct >= HOT_THRESHOLD { COLOR_BAR_HOT } else { COLOR_BAR };
                    canvas.fill_rect(Rect::new(bar_x, by, filled, BAR_HEIGHT), color);
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

        let s = Snapshot { cpu_percent: Some(42.0), ..Default::default() };
        let rows = rows_from(&s);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "CPU");
        // GPU yok, o satir hic uretilmemeli.
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
    fn sicaklik_varsa_degere_ekleniyor() {
        let mut s = Snapshot { cpu_percent: Some(30.0), ..Default::default() };
        assert_eq!(rows_from(&s)[0].value, "30%");
        s.cpu_temp_c = Some(61.0);
        assert_eq!(rows_from(&s)[0].value, "30%  61C");
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
        let mut c = Canvas::new(320, 160);
        w.render(&mut c, Rect::new(0, 0, 320, 160), &ctx);
    }

    #[test]
    fn insan_okunur_bayt() {
        assert_eq!(human_bytes(512), "512B");
        assert_eq!(human_bytes(2048), "2K");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5M");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0G");
    }

    /// Degerler degismiyorken yeniden cizim istenmemeli.
    #[test]
    fn ayni_degerler_yeniden_cizim_istemiyor() {
        let snap = Snapshot { cpu_percent: Some(40.0), ..Default::default() };
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
