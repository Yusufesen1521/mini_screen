//! Donanim izleme paneli. lopaka.app uzerinde cizilen tasarimin karsiligi.
//!
//! Tasarim 480x320 icin cizilmisti, panel ise 320x240. Iki olcunun en-boy
//! orani ayni degil (1.50 ve 1.33), yani duz olceklemek her seyi ezerdi.
//! Bu yuzden olceklenmedi, **yeniden yerlestirildi**: tasarimin yapisi
//! (ust IP seridi, egik CPU/GPU rozetleri, iki sicaklik halkasi, alt
//! STATS seridi) korundu, olculer 320x240'a gore yeniden secildi.
//!
//! Tasarimdan bilincli iki sapma var:
//!
//! 1. Halkanin icinde sicakligin altina kullanim yuzdesi eklendi.
//! 2. GPU halkasi kirmizi degil; gerekcesi `theme::HW_GPU_ARC` yaninda.
//!
//! **Halkalar sicakligi gosterir, kullanimi degil.** Yay boyu
//! `theme::TEMP_MIN_C`..`TEMP_MAX_C` araligina gore doluyor. Halkanin
//! icindeki buyuk sayi sicaklik, altindaki kucuk sayi kullanim yuzdesi.
//!
//! **Okunamayan olcum hic cizilmez.** Sicaklik yoksa o halka cizilmez ve
//! kalan halka ortalanir; GPU hic yoksa dikey ayirici da cizilmez. Isim
//! ya da IP okunamadiysa o satir bos kalir. Hicbir yerde "N/A" yazmiyoruz.
//!
//! Maliyet notu: iki kenar yumusatmali halka, yani rasterlemenin pahali
//! kismi. Widget saniyede iki kez tazeleniyor, 24 FPS degil; `gauges`
//! ile ayni gerekce, olcumu `docs/measurements.md` icinde.

use std::time::Duration;

use crate::register_widget;
use crate::render::{Canvas, Color, FontKind, Rect};
use crate::sensors::Snapshot;
use crate::theme;
use crate::widget::{Context, Widget};

// --- Ikonlar -----------------------------------------------------------
// Tasarimdan oldugu gibi alindi, 16x16 tek bit. Bit duzeni
// `Canvas::bitmap1` ile ayni: satir bas bayt sinirinda, en anlamli bit
// solda.

const ICON_W: u16 = 16;
const ICON_H: u16 = 16;

/// Kure. Ust seritte IP satirinin basinda.
const ICON_GLOBE: [u8; 32] = [
    0x03, 0xc0, 0x0d, 0xb0, 0x32, 0x4c, 0x24, 0x24, 0x44, 0x22, 0x7f, 0xfe, 0x88, 0x11, 0x88, 0x11,
    0x88, 0x11, 0x88, 0x11, 0x7f, 0xfe, 0x44, 0x22, 0x24, 0x24, 0x32, 0x4c, 0x0d, 0xb0, 0x03, 0xc0,
];

/// Termometre. Iki rozette de ayni ikon kullaniliyor.
const ICON_TEMP: [u8; 32] = [
    0x1c, 0x00, 0x22, 0x02, 0x2b, 0x05, 0x2a, 0x02, 0x2b, 0x38, 0x2a, 0x60, 0x2b, 0x40, 0x2a, 0x40,
    0x2a, 0x60, 0x49, 0x38, 0x9c, 0x80, 0xae, 0x80, 0xbe, 0x80, 0x9c, 0x80, 0x41, 0x00, 0x3e, 0x00,
];

/// Cubuk grafik. Alt seridin STATS rozetinde.
const ICON_STATS: [u8; 32] = [
    0x00, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x18, 0x86, 0x18, 0x86, 0x18, 0x86, 0xd8, 0x86, 0xd8,
    0xb6, 0xd8, 0xb6, 0xd8, 0xb6, 0xd8, 0xb6, 0xd8, 0xb6, 0xd8, 0x80, 0x00, 0xff, 0xfc, 0x00, 0x00,
];

// --- Yerlesim ----------------------------------------------------------
// Hepsi `area` sol ust kosesine gore, 320x240 icin secildi.

const PAD: u16 = 6;

/// Ust serit: kure ikonu ve IP adresi.
const TOPBAR_H: u16 = 18;
const SIZE_IP: f32 = 11.0;

/// Rozet satiri. Ust kenarindaki ince krom cizgi tasarimdan geliyor.
const CHIP_Y: u16 = 18;
const CHIP_H: u16 = 18;
/// Rozetin ust kenar genisligi. Alt kenar `CHIP_SLANT` kadar dar.
/// Tasarimdaki 88 pikselin 320'ye olceklenmis hali.
const CHIP_W: u16 = 58;
const CHIP_SLANT: u16 = 10;
/// STATS rozeti daha genis: yazisi uc harfli CPU/GPU'dan uzun ve dar
/// olan alt kenara tasiyordu.
const STATS_CHIP_W: u16 = 70;
const HAIRLINE_H: u16 = 2;
const SIZE_CHIP_LABEL: f32 = 11.0;
const SIZE_DEVICE_NAME: f32 = 10.0;
/// Cihaz adi ile rozet arasindaki bosluk.
const NAME_GAP: u16 = 4;
/// Cihaz adinin sutun kenarina (ya da ayiriciya) birakilan payi.
const NAME_EDGE_PAD: u16 = 4;

/// Gosterge alani. Iki sutun, aralarinda dikey ayirici. Ust siniri
/// burasi, alt siniri `STATS_Y`.
const GAUGE_TOP: u16 = 40;
const RING_RADIUS: f32 = 42.0;
const RING_WIDTH: f32 = 9.0;
/// Yay altta 120 derecelik bosluk birakiyor, tasarimdaki gibi.
const ARC_START_DEG: f32 = 150.0;
const ARC_SWEEP_DEG: f32 = 240.0;
/// Halka merkezinin gosterge alaninin ustunden uzakligi.
const RING_CY: u16 = 52;
/// Dikey ayirici tasarimda uc piksel kalinliginda.
const DIVIDER_W: u16 = 3;

/// Halka ici metin. `DY` degerleri merkeze gore.
const SIZE_TEMP: f32 = 26.0;
const SIZE_UNIT: f32 = 13.0;
const SIZE_USAGE: f32 = 12.0;
const TEMP_DY: i16 = -20;
const USAGE_DY: i16 = 10;

/// Alt serit.
const STATS_Y: u16 = 150;
const STATS_CHIP_Y: u16 = 153;
/// Ilk cubuk satirinin ust kenari.
const BAR_ROW_Y: u16 = 178;
const BAR_ROW_H: u16 = 28;
const BAR_H: u16 = 10;
/// Satir icinde cubugun metin satirinin altina indigi mesafe.
const BAR_DY: u16 = 14;
const SIZE_BAR_LABEL: f32 = 10.0;
/// Alt seride en fazla bu kadar cubuk sigiyor.
const MAX_BAR_ROWS: usize = 2;

/// Isim kisaltmada atilan isaretler.
const NOISE_MARKS: &[&str] = &["(R)", "(TM)", "(C)", "®", "™"];
/// Tek basina hicbir sey soylemeyen kelimeler.
const DROP_WORDS: &[&str] = &["CPU", "Processor", "Processors"];
/// Atilan uretici onekleri. Model adi zaten hangi uretici oldugunu soyluyor.
const VENDOR_PREFIXES: &[&str] = &["AMD", "Intel", "NVIDIA"];
/// Sigmayan metnin sonuna konan isaret.
const ELLIPSIS: &str = "…";

/// Bir sicaklik gostergesi.
struct Gauge {
    label: &'static str,
    /// Cihazin adi, kisaltilmis. Okunamadiysa `None`.
    name: Option<String>,
    temp_c: f32,
    /// Kullanim yuzdesi. Halkanin icinde, sicakligin altinda.
    usage: Option<f32>,
    color: Color,
}

/// Alt seritteki bir doluluk cubugu.
struct BarRow {
    label: &'static str,
    detail: String,
    percent: f32,
    color: Color,
}

#[derive(Default)]
pub struct Hwmon {
    /// Son cizilen icerigin ozeti. Degismediyse yeniden cizmiyoruz.
    last: Vec<String>,
}

fn human_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        let g = n as f64 / GB as f64;
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

/// Uretici adini ekrana sigacak hale getirir.
///
/// Ham adlar cok gurultulu: "12th Gen Intel(R) Core(TM) i7-12700H" ya da
/// "AMD Ryzen 5 5600X 6-Core Processor". Rozetin yanindaki alan 320
/// piksellik ekranda 90 piksel; bu gurultu atilmazsa geriye model adi
/// kalmiyor.
///
/// Elenen her sey okuyucunun zaten bildigi ya da umursamadigi sey:
/// tescil isaretleri, frekans, kusak oneki, uretici adi, cekirdek sayisi
/// ve "Processor" gibi dolgu kelimeler.
fn short_name(raw: &str) -> String {
    let mut s = raw.to_string();
    for m in NOISE_MARKS {
        s = s.replace(m, "");
    }
    // Frekans son ekini at: "... i5-8250U CPU @ 1.60GHz".
    if let Some(i) = s.find(" @ ") {
        s.truncate(i);
    }

    let mut tokens: Vec<&str> = s.split_whitespace().collect();

    // Kusak oneki: "12th Gen ...".
    if tokens.len() > 2
        && tokens[1].eq_ignore_ascii_case("Gen")
        && tokens[0].starts_with(|c: char| c.is_ascii_digit())
    {
        tokens.drain(0..2);
    }

    // Uretici oneki. Tek kelime kaldiysa dokunulmuyor, yoksa ad bos kalir.
    if tokens.len() > 1 && VENDOR_PREFIXES.iter().any(|v| tokens[0].eq_ignore_ascii_case(v)) {
        tokens.remove(0);
    }

    let kept: Vec<&str> = tokens
        .into_iter()
        .filter(|t| !DROP_WORDS.iter().any(|d| t.eq_ignore_ascii_case(d)))
        // "6-Core", "12-Core" gibi cekirdek sayisi.
        .filter(|t| !t.to_ascii_lowercase().ends_with("-core"))
        .collect();

    let out = kept.join(" ");
    // Kural disi bir ad her seyi eledi diyelim: hicbir sey gostermemektense
    // ham adi gosteriyoruz.
    if out.is_empty() {
        raw.trim().to_string()
    } else {
        out
    }
}

/// Metni `max_w` icine sigdirir, gerekirse sondan kirpip isaret koyar.
fn fit(canvas: &mut Canvas, s: &str, max_w: u16, size: f32, kind: FontKind) -> String {
    if canvas.text_width(s, size, kind) <= max_w {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    // Bastan kisaltarak ilk sigan uzunlugu ariyoruz. Ad kisa oldugu icin
    // dogrusal arama yeterli.
    for n in (1..chars.len()).rev() {
        let cut: String = chars[..n].iter().collect::<String>() + ELLIPSIS;
        if canvas.text_width(&cut, size, kind) <= max_w {
            return cut;
        }
    }
    String::new()
}

/// Sicakligin halkada kapladigi oran, 0..1.
fn temp_ratio(t: f32) -> f32 {
    let span = theme::TEMP_MAX_C - theme::TEMP_MIN_C;
    ((t - theme::TEMP_MIN_C) / span).clamp(0.0, 1.0)
}

/// Cizilecek gostergeler. Sicakligi okunamayan hic uretilmiyor.
fn gauges_from(s: &Snapshot) -> Vec<Gauge> {
    let mut v = Vec::new();
    if let Some(t) = s.cpu_temp_c {
        v.push(Gauge {
            label: "CPU",
            name: s.cpu_name.as_deref().map(short_name),
            temp_c: t,
            usage: s.cpu_percent,
            color: theme::HW_CPU_ARC,
        });
    }
    if let Some(t) = s.gpu_temp_c {
        v.push(Gauge {
            label: "GPU",
            name: s.gpu_name.as_deref().map(short_name),
            temp_c: t,
            usage: s.gpu_percent,
            color: theme::HW_GPU_ARC,
        });
    }
    v
}

/// Alt serit cubuklari. Tasarimda sadece bellek vardi; VRAM ve disk de
/// elimizde oldugu icin sigdigi kadari ekleniyor.
fn bars_from(s: &Snapshot) -> Vec<BarRow> {
    let mut v = Vec::new();
    if let (Some(used), Some(total), Some(p)) = (s.mem_used, s.mem_total, s.mem_percent()) {
        v.push(BarRow {
            label: "BELLEK",
            detail: format!("{} / {}", human_bytes(used), human_bytes(total)),
            percent: p,
            color: theme::RAM,
        });
    }
    if let (Some(used), Some(total), Some(p)) =
        (s.gpu_mem_used, s.gpu_mem_total, s.gpu_mem_percent())
    {
        v.push(BarRow {
            label: "VRAM",
            detail: format!("{} / {}", human_bytes(used), human_bytes(total)),
            percent: p,
            color: theme::GPU,
        });
    }
    if let (Some(used), Some(total), Some(p)) = (s.disk_used, s.disk_total, s.disk_percent()) {
        v.push(BarRow {
            label: "DISK",
            detail: format!("{} / {}", human_bytes(used), human_bytes(total)),
            percent: p,
            color: theme::DISK,
        });
    }
    v.truncate(MAX_BAR_ROWS);
    v
}

impl Widget for Hwmon {
    fn kind(&self) -> &'static str {
        "hwmon"
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn update(&mut self, ctx: &Context<'_>) -> bool {
        let s = ctx.sensors;
        let mut sig: Vec<String> = Vec::new();
        sig.push(s.local_ip.clone().unwrap_or_default());
        for g in gauges_from(s) {
            // Sicaklik ve yuzde tam sayiya yuvarlanarak karsilastiriliyor:
            // halkanin acisi zaten piksel cozunurlugunde o kadar degisiyor.
            sig.push(format!(
                "{}{}{:.0}{:.0}",
                g.label,
                g.name.unwrap_or_default(),
                g.temp_c,
                g.usage.unwrap_or(-1.0)
            ));
        }
        for b in bars_from(s) {
            sig.push(format!("{}{}{:.0}", b.label, b.detail, b.percent));
        }
        if sig == self.last {
            return false;
        }
        self.last = sig;
        true
    }

    fn render(&mut self, canvas: &mut Canvas, area: Rect, ctx: &Context<'_>) {
        canvas.fill_rect(area, theme::BG);

        self.draw_topbar(canvas, area, ctx.sensors);

        let gauges = gauges_from(ctx.sensors);
        self.draw_gauges(canvas, area, &gauges);
        self.draw_stats(canvas, area, &bars_from(ctx.sensors));
    }
}

impl Hwmon {
    /// Ust serit: kure ikonu ve IP adresi.
    fn draw_topbar(&self, canvas: &mut Canvas, area: Rect, s: &Snapshot) {
        canvas.bitmap1(
            &ICON_GLOBE,
            area.x + PAD,
            area.y + (TOPBAR_H - ICON_H) / 2,
            ICON_W,
            ICON_H,
            theme::TEXT,
        );
        let text_x = area.x + PAD + ICON_W + 5;
        let ty = area.y + 3;
        let after = canvas.text(
            "IP:",
            text_x,
            ty,
            SIZE_IP,
            FontKind::Sans,
            theme::TEXT_FAINT,
        );
        // IP okunamadiysa sadece etiket kalir. Yer tutucu yazmiyoruz.
        if let Some(ip) = &s.local_ip {
            canvas.text(ip, after + 5, ty, SIZE_IP, FontKind::Mono, theme::TEXT_DIM);
        }
    }

    /// Rozet satiri ve iki sicaklik halkasi.
    ///
    /// Tek gosterge kalirsa butun genisligi kullanip ortalaniyor ve dikey
    /// ayirici cizilmiyor.
    fn draw_gauges(&self, canvas: &mut Canvas, area: Rect, gauges: &[Gauge]) {
        // Rozet satirinin ustundeki ince krom cizgi tasarimdan geliyor ve
        // gosterge alani bos olsa da duruyor: panelin cercevesi o.
        canvas.fill_rect(
            Rect::new(area.x + PAD, area.y + CHIP_Y, area.w - 2 * PAD, HAIRLINE_H),
            theme::HW_ACCENT,
        );

        if gauges.is_empty() {
            canvas.text(
                "sicaklik kaynagi yok",
                area.x + PAD,
                area.y + GAUGE_TOP,
                SIZE_DEVICE_NAME,
                FontKind::Sans,
                theme::TEXT_FAINT,
            );
            return;
        }

        let cols = gauges.len() as u16;
        let col_w = area.w / cols;

        // Ayirici sadece iki sutun varken anlamli. Ustteki ve alttaki
        // yatay cizgilere kadar iniyor, arada bosluk kalmasin.
        if cols == 2 {
            let x = area.x + col_w - DIVIDER_W / 2;
            let h = STATS_Y + HAIRLINE_H - CHIP_Y;
            canvas.fill_rect(
                Rect::new(x, area.y + CHIP_Y, DIVIDER_W, h),
                theme::HW_ACCENT,
            );
        }

        for (i, g) in gauges.iter().enumerate() {
            let col_x = area.x + col_w * i as u16;
            // Sol sutunun rozeti solda ve egimi sagda, sag sutunun tersi.
            // Tasarimdaki simetri bu.
            let on_left = i == 0 && cols == 2;
            self.draw_chip(canvas, col_x, col_w, area.y + CHIP_Y, g, on_left || cols == 1);
            self.draw_ring(canvas, col_x + col_w / 2, area.y + GAUGE_TOP + RING_CY, g);
        }
    }

    /// Egik kenarli etiket rozeti, ikonu, yazisi ve yanindaki cihaz adi.
    ///
    /// `left` rozetin sutunun solunda mi durdugu. Solda duran rozetin
    /// egimi sagda, sagda duranin egimi solda.
    fn draw_chip(&self, canvas: &mut Canvas, col_x: u16, col_w: u16, y: u16, g: &Gauge, left: bool) {
        let chip_x = if left {
            col_x + PAD
        } else {
            col_x + col_w - PAD - CHIP_W
        };

        // Rozet, satir satir daralan bir yamuk. Dolgu yolu yerine
        // dikdortgen dizisi: kenar yumusatma yok, maliyet de yok.
        for row in 0..CHIP_H {
            let inset = CHIP_SLANT * row / CHIP_H;
            let (x, w) = if left {
                (chip_x, CHIP_W - inset)
            } else {
                (chip_x + inset, CHIP_W - inset)
            };
            canvas.fill_rect(Rect::new(x, y + row, w, 1), theme::HW_ACCENT);
        }

        // Solda duran rozette once ikon, sagda duranda once yazi: her
        // ikisinde de icerik yamugun genis tarafinda kaliyor.
        let icon_y = y + (CHIP_H - ICON_H) / 2;
        let label_y = y + 4;
        if left {
            canvas.bitmap1(&ICON_TEMP, chip_x + 3, icon_y, ICON_W, ICON_H, theme::TEXT);
            canvas.text(
                g.label,
                chip_x + 3 + ICON_W + 3,
                label_y,
                SIZE_CHIP_LABEL,
                FontKind::Sans,
                theme::TEXT,
            );
        } else {
            canvas.text(
                g.label,
                chip_x + CHIP_SLANT + 3,
                label_y,
                SIZE_CHIP_LABEL,
                FontKind::Sans,
                theme::TEXT,
            );
            canvas.bitmap1(
                &ICON_TEMP,
                chip_x + CHIP_W - ICON_W - 3,
                icon_y,
                ICON_W,
                ICON_H,
                theme::TEXT,
            );
        }

        // Cihaz adi rozetin ic tarafinda kalan bosluga yaziliyor. Sinirlar
        // formulle degil gercek konumlardan cikariliyor: ad alani birkac
        // piksel bile dar hesaplanirsa model adinin sonu kirpiliyor.
        let Some(name) = &g.name else { return };
        let name_y = y + 4;
        if left {
            let start = chip_x + CHIP_W + NAME_GAP;
            let end = col_x + col_w - NAME_EDGE_PAD;
            let text = fit(
                canvas,
                name,
                end.saturating_sub(start),
                SIZE_DEVICE_NAME,
                FontKind::Sans,
            );
            canvas.text(
                &text,
                start,
                name_y,
                SIZE_DEVICE_NAME,
                FontKind::Sans,
                theme::TEXT_DIM,
            );
        } else {
            let end = chip_x.saturating_sub(NAME_GAP);
            let start = col_x + NAME_EDGE_PAD;
            let text = fit(
                canvas,
                name,
                end.saturating_sub(start),
                SIZE_DEVICE_NAME,
                FontKind::Sans,
            );
            canvas.text_right(
                &text,
                end,
                name_y,
                SIZE_DEVICE_NAME,
                FontKind::Sans,
                theme::TEXT_DIM,
            );
        }
    }

    /// Sicaklik halkasi, icinde sicaklik ve altinda kullanim yuzdesi.
    fn draw_ring(&self, canvas: &mut Canvas, cx: u16, cy: u16, g: &Gauge) {
        let color = if g.temp_c >= theme::TEMP_HOT_C {
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
            theme::HW_TRACK,
        );
        canvas.arc(
            cx as f32,
            cy as f32,
            RING_RADIUS,
            ARC_START_DEG,
            ARC_SWEEP_DEG * temp_ratio(g.temp_c),
            RING_WIDTH,
            color,
        );

        // Sayi ve birim iki ayri boyda, bu yuzden ikisi tek bir grup gibi
        // olculup birlikte ortalaniyor.
        let num = format!("{:.0}", g.temp_c);
        let unit = "°C";
        let num_w = canvas.text_width(&num, SIZE_TEMP, FontKind::Mono);
        let unit_w = canvas.text_width(unit, SIZE_UNIT, FontKind::Sans);
        let start = cx.saturating_sub((num_w + unit_w) / 2);
        let ty = (cy as i16 + TEMP_DY).max(0) as u16;
        canvas.text(&num, start, ty, SIZE_TEMP, FontKind::Mono, theme::TEXT);
        // Birim sayinin alt hizasina oturuyor.
        canvas.text(
            unit,
            start + num_w,
            ty + (SIZE_TEMP - SIZE_UNIT) as u16,
            SIZE_UNIT,
            FontKind::Sans,
            theme::TEXT_DIM,
        );

        // Kullanim yuzdesi sicakligin altinda. Okunamadiysa hic cizilmez.
        if let Some(u) = g.usage {
            let uy = (cy as i16 + USAGE_DY).max(0) as u16;
            canvas.text_center(
                &format!("%{:.0}", u.clamp(0.0, 100.0)),
                cx,
                uy,
                SIZE_USAGE,
                FontKind::Mono,
                theme::TEXT_DIM,
            );
        }
    }

    /// Alt serit: STATS rozeti ve doluluk cubuklari.
    fn draw_stats(&self, canvas: &mut Canvas, area: Rect, bars: &[BarRow]) {
        canvas.fill_rect(
            Rect::new(area.x + PAD, area.y + STATS_Y, area.w - 2 * PAD, HAIRLINE_H),
            theme::HW_ACCENT,
        );

        // STATS rozeti: soldaki rozetlerle ayni yamuk, ayni yon.
        let chip_x = area.x + PAD;
        let chip_y = area.y + STATS_CHIP_Y;
        for row in 0..CHIP_H {
            let inset = CHIP_SLANT * row / CHIP_H;
            canvas.fill_rect(
                Rect::new(chip_x, chip_y + row, STATS_CHIP_W - inset, 1),
                theme::HW_ACCENT,
            );
        }
        canvas.bitmap1(
            &ICON_STATS,
            chip_x + 3,
            chip_y + (CHIP_H - ICON_H) / 2,
            ICON_W,
            ICON_H,
            theme::TEXT,
        );
        canvas.text(
            "STATS",
            chip_x + 3 + ICON_W + 3,
            chip_y + 4,
            SIZE_CHIP_LABEL,
            FontKind::Sans,
            theme::TEXT,
        );

        let bar_x = area.x + PAD;
        let bar_w = area.w - 2 * PAD;
        let right = area.x + area.w - PAD;
        for (i, b) in bars.iter().enumerate() {
            let top = area.y + BAR_ROW_Y + i as u16 * BAR_ROW_H;
            if top + BAR_ROW_H > area.y + area.h {
                break;
            }
            let after = canvas.text(
                b.label,
                bar_x,
                top,
                SIZE_BAR_LABEL,
                FontKind::Sans,
                theme::TEXT_FAINT,
            );
            canvas.text(
                &format!("%{:.0}", b.percent),
                after + 6,
                top,
                SIZE_BAR_LABEL,
                FontKind::Mono,
                theme::TEXT,
            );
            canvas.text_right(
                &b.detail,
                right,
                top,
                SIZE_BAR_LABEL,
                FontKind::Mono,
                theme::TEXT_DIM,
            );

            let by = top + BAR_DY;
            canvas.fill_rect(Rect::new(bar_x, by, bar_w, BAR_H), theme::TRACK);
            let filled = (bar_w as f32 * b.percent.clamp(0.0, 100.0) / 100.0) as u16;
            let color = if b.percent >= theme::HOT_THRESHOLD {
                theme::HOT
            } else {
                b.color
            };
            canvas.fill_rect(Rect::new(bar_x, by, filled, BAR_H), color);
        }
    }
}

register_widget!("hwmon", Hwmon);

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with(snap: &Snapshot) -> Context<'_> {
        Context {
            uptime: Duration::ZERO,
            local_hms: (12, 0, 0),
            local_ymd: (2026, 1, 1),
            sensors: snap,
        }
    }

    #[test]
    fn islemci_adi_kisaliyor() {
        assert_eq!(
            short_name("AMD Ryzen 5 5600X 6-Core Processor"),
            "Ryzen 5 5600X"
        );
        assert_eq!(
            short_name("12th Gen Intel(R) Core(TM) i7-12700H"),
            "Core i7-12700H"
        );
        assert_eq!(
            short_name("Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz"),
            "Core i5-8250U"
        );
        assert_eq!(short_name("NVIDIA GeForce RTX 3060"), "GeForce RTX 3060");
        assert_eq!(short_name("AMD Radeon RX 6700 XT"), "Radeon RX 6700 XT");
        assert_eq!(short_name("Apple M1 Pro"), "Apple M1 Pro");
    }

    /// Kural disi bir ad her seyi eleyemez: geriye ham ad kalmali.
    #[test]
    fn her_sey_elenirse_ham_ad_kaliyor() {
        assert_eq!(short_name("Processor"), "Processor");
        assert_eq!(short_name("  "), "");
    }

    /// Tek kelimelik uretici adi silinmemeli, yoksa ekranda hicbir sey kalmaz.
    #[test]
    fn tek_kelime_uretici_silinmiyor() {
        assert_eq!(short_name("Intel"), "Intel");
    }

    #[test]
    fn sicaklik_orani_kirpiliyor() {
        assert_eq!(temp_ratio(theme::TEMP_MIN_C - 10.0), 0.0);
        assert_eq!(temp_ratio(theme::TEMP_MAX_C + 10.0), 1.0);
        let mid = (theme::TEMP_MIN_C + theme::TEMP_MAX_C) / 2.0;
        assert!((temp_ratio(mid) - 0.5).abs() < 0.001);
    }

    /// Sicaklik okunamayan taraf hic gosterge uretmemeli.
    #[test]
    fn sicakliksiz_gosterge_uretilmiyor() {
        let s = Snapshot::default();
        assert!(gauges_from(&s).is_empty());

        let s = Snapshot {
            cpu_temp_c: Some(55.0),
            ..Default::default()
        };
        let g = gauges_from(&s);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].label, "CPU");
        assert!(g[0].usage.is_none(), "kullanim yokken yuzde olmamali");
        assert!(g[0].name.is_none(), "isim yokken ad olmamali");
    }

    #[test]
    fn gpu_gelince_iki_gosterge() {
        let s = Snapshot {
            cpu_temp_c: Some(45.0),
            gpu_temp_c: Some(62.0),
            ..Default::default()
        };
        let labels: Vec<_> = gauges_from(&s).iter().map(|g| g.label).collect();
        assert_eq!(labels, vec!["CPU", "GPU"]);
    }

    /// Alt serit iki satirla sinirli, fazlasi cizilmemeli.
    #[test]
    fn cubuk_sayisi_sinirli() {
        let s = Snapshot {
            mem_used: Some(1),
            mem_total: Some(2),
            gpu_mem_used: Some(1),
            gpu_mem_total: Some(2),
            disk_used: Some(1),
            disk_total: Some(2),
            ..Default::default()
        };
        assert_eq!(bars_from(&s).len(), MAX_BAR_ROWS);
    }

    #[test]
    fn sensorsuz_cizim_cokmuyor() {
        let snap = Snapshot::default();
        let mut w = Hwmon::default();
        let mut c = Canvas::new(320, 240);
        w.render(&mut c, Rect::new(0, 0, 320, 240), &ctx_with(&snap));
    }

    #[test]
    fn tam_veriyle_cizim_cokmuyor() {
        let snap = Snapshot {
            cpu_name: Some("AMD Ryzen 9 7950X 16-Core Processor".into()),
            gpu_name: Some("NVIDIA GeForce RTX 4070 Ti".into()),
            local_ip: Some("192.168.1.34".into()),
            cpu_percent: Some(37.0),
            cpu_temp_c: Some(64.1),
            mem_used: Some(12 * 1024 * 1024 * 1024),
            mem_total: Some(32 * 1024 * 1024 * 1024),
            gpu_percent: Some(88.0),
            gpu_temp_c: Some(71.0),
            gpu_mem_used: Some(6 * 1024 * 1024 * 1024),
            gpu_mem_total: Some(12 * 1024 * 1024 * 1024),
            ..Default::default()
        };
        let mut w = Hwmon::default();
        let mut c = Canvas::new(320, 240);
        w.render(&mut c, Rect::new(0, 0, 320, 240), &ctx_with(&snap));
    }

    #[test]
    fn ayni_veride_yeniden_cizim_yok() {
        let snap = Snapshot {
            cpu_temp_c: Some(50.0),
            ..Default::default()
        };
        let mut w = Hwmon::default();
        assert!(w.update(&ctx_with(&snap)));
        assert!(!w.update(&ctx_with(&snap)));
    }

    /// Sicaklik degisince yeniden cizilmeli: halka onu gosteriyor.
    #[test]
    fn sicaklik_degisince_yeniden_cizim() {
        let mut w = Hwmon::default();
        let a = Snapshot {
            cpu_temp_c: Some(50.0),
            ..Default::default()
        };
        assert!(w.update(&ctx_with(&a)));
        let b = Snapshot {
            cpu_temp_c: Some(51.0),
            ..Default::default()
        };
        assert!(w.update(&ctx_with(&b)));
    }
}
