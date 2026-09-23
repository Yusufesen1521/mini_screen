//! Cizim dongusu.
//!
//! Widget'lari tazeler, degisenleri tuvale cizer, onceki kareyle
//! karsilastirir ve gonderilecek kirli dikdortgenleri uretir. Bu katman
//! cihazi tanimaz: `Link` disaridan surulur. Boylece motor testte
//! donanimsiz calistirilabiliyor ve onizleme de ayni kareyi kullanabiliyor.

use std::time::{Duration, Instant};

use crate::dirty::DirtyTracker;
use crate::render::{Canvas, Rect};
use crate::sensors::{Sensors, Snapshot};
use crate::widget::{self, Context, Widget};

/// Yerlesimdeki bir kutu: hangi widget, nereye.
#[derive(Debug, Clone)]
pub struct Slot {
    pub kind: String,
    pub area: Rect,
}

impl Slot {
    pub fn new(kind: &str, area: Rect) -> Slot {
        Slot {
            kind: kind.to_string(),
            area,
        }
    }
}

#[derive(Debug)]
pub enum LayoutError {
    /// Kayitli olmayan widget turu istendi.
    UnknownKind(String),
    /// Kutu ekran disina tasiyor.
    OutOfBounds(Slot),
    /// Sifir olculu kutu.
    ZeroSize(Slot),
    /// Iki kutu ust uste biniyor.
    Overlap(Slot, Slot),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::UnknownKind(k) => {
                write!(
                    f,
                    "bilinmeyen widget turu: {}. Kayitli olanlar: {:?}",
                    k,
                    widget::kinds()
                )
            }
            LayoutError::OutOfBounds(s) => {
                write!(f, "{} ekran disina tasiyor: {:?}", s.kind, s.area)
            }
            LayoutError::ZeroSize(s) => write!(f, "{} sifir olculu", s.kind),
            LayoutError::Overlap(a, b) => {
                write!(f, "{} ve {} ust uste biniyor", a.kind, b.kind)
            }
        }
    }
}

impl std::error::Error for LayoutError {}

struct Mounted {
    widget: Box<dyn Widget>,
    area: Rect,
    next_tick: Instant,
}

pub struct Engine {
    canvas: Canvas,
    tracker: DirtyTracker,
    mounted: Vec<Mounted>,
    frame: Vec<u16>,
    sensors: Sensors,
    /// Olcum kipi: widget'lar tazelenmez ama rasterleme ve diff her
    /// turda calisir. "Ekranda hicbir sey degismiyorken trafik" cikis
    /// kriterini gercekten durgun bir ekranla olcmek icin.
    frozen: bool,
    started: Instant,
    width: u16,
    height: u16,
}

impl Engine {
    pub fn new(width: u16, height: u16, layout: &[Slot]) -> Result<Engine, LayoutError> {
        validate(width, height, layout)?;

        let now = Instant::now();
        let mut mounted = Vec::with_capacity(layout.len());
        for s in layout {
            let w =
                widget::make(&s.kind).ok_or_else(|| LayoutError::UnknownKind(s.kind.clone()))?;
            mounted.push(Mounted {
                widget: w,
                area: s.area,
                next_tick: now,
            });
        }

        Ok(Engine {
            canvas: Canvas::new(width, height),
            tracker: DirtyTracker::new(width, height),
            mounted,
            frame: vec![0u16; width as usize * height as usize],
            sensors: Sensors::probe_all(),
            frozen: false,
            started: now,
            width,
            height,
        })
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn has_font(&self) -> bool {
        self.canvas.has_font()
    }

    /// Yeniden baglanmada cagrilir: cihazin ekraninda ne oldugunu
    /// bilmiyoruz, bir sonraki tur tam ekran gonderilsin.
    pub fn force_full(&mut self) {
        self.tracker.force_full();
    }

    /// Olcum kipi. Acikken widget'lar tazelenmez, yani ekran gercekten
    /// durgun kalir; rasterleme ve diff calismaya devam eder.
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Bir tur. Zamani gelen widget'lari tazeler, degisenleri cizer,
    /// kirli dikdortgenleri `out` icine yazar.
    ///
    /// Hicbir sey degismediyse `out` bos kalir ve cihaza tek bayt
    /// gitmez. Faz 2'nin dirty tracking kriteri tam olarak bu.
    pub fn tick(&mut self, out: &mut Vec<Rect>) {
        self.sensors.poll();
        let now = Instant::now();
        // Goruntu kopyalaniyor: ctx yasarken canvas'i mut odunc almak
        // gerekiyor. Snapshot birkac sayi ve iki kisa isimden ibaret,
        // klonlamasi kare basina olculebilir bir maliyet degil.
        let snap = self.sensors.snapshot().clone();
        let ctx = self.context_with(now, &snap);

        let mut drew = false;
        if !self.frozen {
            for m in self.mounted.iter_mut() {
                if now < m.next_tick {
                    continue;
                }
                m.next_tick = now + m.widget.interval();
                if m.widget.update(&ctx) {
                    m.widget.render(&mut self.canvas, m.area, &ctx);
                    drew = true;
                }
            }
        }

        out.clear();
        // Olcum kipinde erken cikmiyoruz: diff yolunun gercekten sifir
        // dikdortgen urettigini gormek istiyoruz.
        if !drew && !self.frozen && !self.tracker_needs_full() {
            return;
        }
        self.canvas.to_rgb565(&mut self.frame);
        self.tracker.diff(&self.frame, out);
    }

    /// Ilk turda widget hic cizmese bile tam ekran gonderilmeli, yoksa
    /// cihazda onceki icerik kalir.
    fn tracker_needs_full(&self) -> bool {
        self.started.elapsed() < Duration::from_millis(1)
    }

    fn context_with<'a>(&self, now: Instant, snap: &'a Snapshot) -> Context<'a> {
        use chrono::{Datelike, Local, Timelike};
        let t = Local::now();
        Context {
            uptime: now.duration_since(self.started),
            local_hms: (t.hour() as u8, t.minute() as u8, t.second() as u8),
            local_ymd: (t.year(), t.month() as u8, t.day() as u8),
            sensors: snap,
        }
    }

    /// Bu makinede calisan sensor kaynaklari.
    pub fn sensor_sources(&self) -> Vec<&'static str> {
        self.sensors.active_names()
    }

    /// Yoklanip bulunamayan kaynaklar. Hata degil, bilgi.
    pub fn missing_sources(&self) -> &[&'static str] {
        self.sensors.missing_names()
    }

    pub fn sensors(&self) -> &Snapshot {
        self.sensors.snapshot()
    }

    /// Son cizilen kareyi PNG olarak yazar. Tasarim kontrolu icin.
    pub fn save_png(&self, path: &str) -> Result<(), String> {
        self.canvas.save_png(path)
    }

    /// Son uretilen kare, RGB565. Onizleme de bunu gosterecek.
    pub fn frame(&self) -> &[u16] {
        &self.frame
    }

    /// Bir dikdortgenin piksellerini bitisik tampona kopyalar.
    pub fn copy_region(&self, r: Rect, dst: &mut Vec<u16>) {
        DirtyTracker::copy_region(&self.frame, self.width, r, dst);
    }
}

fn validate(width: u16, height: u16, layout: &[Slot]) -> Result<(), LayoutError> {
    for s in layout {
        if s.area.w == 0 || s.area.h == 0 {
            return Err(LayoutError::ZeroSize(s.clone()));
        }
        if s.area.x as u32 + s.area.w as u32 > width as u32
            || s.area.y as u32 + s.area.h as u32 > height as u32
        {
            return Err(LayoutError::OutOfBounds(s.clone()));
        }
    }
    for i in 0..layout.len() {
        for j in (i + 1)..layout.len() {
            if overlaps(layout[i].area, layout[j].area) {
                return Err(LayoutError::Overlap(layout[i].clone(), layout[j].clone()));
            }
        }
    }
    Ok(())
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Varsayilan yerlesim: donanim izleme paneli.
///
/// Tek widget butun ekrani kapliyor, cunku `hwmon` kendi ust seridini,
/// ayiricilarini ve alt seridini ciziyor. Gercek yerlesim motoru Faz
/// 4'un isi, bu sadece bir baslangic.
pub fn default_layout(width: u16, height: u16) -> Vec<Slot> {
    hwmon_layout(width, height)
}

/// Donanim izleme paneli. `default_layout` bunu donduruyor.
pub fn hwmon_layout(width: u16, height: u16) -> Vec<Slot> {
    vec![Slot::new("hwmon", Rect::new(0, 0, width, height))]
}

/// Onceki varsayilan: ustte saat, altinda halka gostergeler.
pub fn gauges_layout(width: u16, height: u16) -> Vec<Slot> {
    let header = crate::theme::HEADER_H.min(height);
    vec![
        Slot::new("clock", Rect::new(0, 0, width, header)),
        Slot::new("gauges", Rect::new(0, header, width, height - header)),
    ]
}

/// Sadece saat ve calisma suresi. Sensorsuz makinede ve testte kullanilir.
pub fn clock_layout(width: u16, height: u16) -> Vec<Slot> {
    let top_h = height * 2 / 3;
    vec![
        Slot::new("clock", Rect::new(0, 0, width, top_h)),
        Slot::new("uptime", Rect::new(0, top_h, width, height - top_h)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varsayilan_yerlesim_gecerli() {
        let l = default_layout(320, 240);
        assert!(Engine::new(320, 240, &l).is_ok());
    }

    #[test]
    fn hwmon_yerlesimi_gecerli() {
        let l = hwmon_layout(320, 240);
        assert!(Engine::new(320, 240, &l).is_ok());
    }

    #[test]
    fn gauges_yerlesimi_gecerli() {
        let l = gauges_layout(320, 240);
        assert!(Engine::new(320, 240, &l).is_ok());
    }

    /// Varsayilan yerlesim hwmon. Degistirilirse bu test uyarir.
    #[test]
    fn varsayilan_hwmon() {
        let l = default_layout(320, 240);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].kind, "hwmon");
    }

    #[test]
    fn ekran_disi_kutu_reddediliyor() {
        let l = vec![Slot::new("clock", Rect::new(200, 0, 200, 100))];
        assert!(matches!(
            Engine::new(320, 240, &l),
            Err(LayoutError::OutOfBounds(_))
        ));
    }

    #[test]
    fn sifir_olculu_kutu_reddediliyor() {
        let l = vec![Slot::new("clock", Rect::new(0, 0, 0, 100))];
        assert!(matches!(
            Engine::new(320, 240, &l),
            Err(LayoutError::ZeroSize(_))
        ));
    }

    #[test]
    fn ust_uste_binme_reddediliyor() {
        let l = vec![
            Slot::new("clock", Rect::new(0, 0, 200, 100)),
            Slot::new("uptime", Rect::new(100, 50, 200, 100)),
        ];
        assert!(matches!(
            Engine::new(320, 240, &l),
            Err(LayoutError::Overlap(_, _))
        ));
    }

    #[test]
    fn bilinmeyen_widget_reddediliyor() {
        let l = vec![Slot::new("boyle-bir-sey-yok", Rect::new(0, 0, 10, 10))];
        assert!(matches!(
            Engine::new(320, 240, &l),
            Err(LayoutError::UnknownKind(_))
        ));
    }

    /// Cikis kriteri: ekranda hicbir sey degismiyorken trafik sifir.
    #[test]
    fn durgun_ekranda_dikdortgen_uretilmiyor() {
        let l = default_layout(320, 240);
        let mut e = Engine::new(320, 240, &l).unwrap();
        let mut out = Vec::new();

        e.tick(&mut out);
        assert!(!out.is_empty(), "ilk tur tam ekran gondermeliydi");

        // Saat saniyede bir degisiyor. Ayni saniye icinde arka arkaya
        // tiklatirsak hicbir sey uretilmemeli.
        let mut bos_turlar = 0;
        for _ in 0..50 {
            e.tick(&mut out);
            if out.is_empty() {
                bos_turlar += 1;
            }
        }
        assert!(
            bos_turlar > 0,
            "hicbir tur bos gecmedi, dirty tracking calismiyor"
        );
    }
}
