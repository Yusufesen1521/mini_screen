//! Sensor eklenti katmani.
//!
//! Plan 2.4: "Sensor okuma platform basina tamamen farkli, eklenti
//! katmani olarak ayrilacak." Bu modul o katman.
//!
//! Tasarimin tek onemli kurali: **her olcum `Option`.** Faz 2 cikis
//! kriteri "eksik sensor kaynagi widget'i bozmuyor, GPU yoksa ya da
//! HWiNFO kapaliysa o alan temiz sekilde gizleniyor, hata gostermiyor"
//! diyor. Bunu disiplinle degil tip sistemiyle sagliyoruz: widget bir
//! degeri okumak icin `Option`'i acmak zorunda, yani "yok" durumunu
//! atlayamiyor.
//!
//! Yeni kaynak eklemek: `sensors/` altina bir dosya, icinde `Source`
//! uygulamasi ve bir `register_source!` cagrisi. Widget'larla ayni
//! mekanizma, ayni gerekce.

use std::time::{Duration, Instant};

pub mod afterburner;
pub mod system;

/// Butun sensor okumalarinin tek yerde toplandigi goruntu.
///
/// Alanlarin hepsi `Option`. `None` demek "bu makinede bu deger yok ya
/// da henuz okunmadi" demek, hata demek degil.
#[derive(Debug, Default, Clone, Copy)]
pub struct Snapshot {
    pub cpu_percent: Option<f32>,
    pub cpu_temp_c: Option<f32>,
    pub cpu_cores: Option<usize>,

    pub mem_used: Option<u64>,
    pub mem_total: Option<u64>,

    pub disk_used: Option<u64>,
    pub disk_total: Option<u64>,

    /// Saniyedeki bayt, son ornekleme araligina gore.
    pub net_rx_bps: Option<u64>,
    pub net_tx_bps: Option<u64>,

    pub gpu_percent: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub gpu_mem_used: Option<u64>,
    pub gpu_mem_total: Option<u64>,
}

impl Snapshot {
    /// Yuzde olarak kullanim, ikisi de varsa.
    pub fn ratio(used: Option<u64>, total: Option<u64>) -> Option<f32> {
        match (used, total) {
            (Some(u), Some(t)) if t > 0 => Some(100.0 * u as f32 / t as f32),
            _ => None,
        }
    }

    pub fn mem_percent(&self) -> Option<f32> {
        Snapshot::ratio(self.mem_used, self.mem_total)
    }

    pub fn disk_percent(&self) -> Option<f32> {
        Snapshot::ratio(self.disk_used, self.disk_total)
    }

    pub fn gpu_mem_percent(&self) -> Option<f32> {
        Snapshot::ratio(self.gpu_mem_used, self.gpu_mem_total)
    }
}

/// Bir sensor kaynagi.
///
/// Kaynak kendini kurmayi beceremezse `probe` `None` doner ve o kaynak
/// hic yuklenmez. Calisirken kaybolursa `sample` ilgili alanlari
/// `None` birakir; bu bir hata degil, normal durum.
pub trait Source: Send {
    fn name(&self) -> &'static str;

    /// Ornekleme araligi. Motor bundan sik cagirmaz.
    fn interval(&self) -> Duration;

    /// Okuyabildigi alanlari doldurur, okuyamadiklarina dokunmaz.
    fn sample(&mut self, out: &mut Snapshot);
}

/// Kayit girisi. `probe` bu makinede kaynak kullanilabiliyorsa bir
/// ornek uretir, kullanilamiyorsa `None`.
pub struct SourceRegistration {
    pub name: &'static str,
    pub probe: fn() -> Option<Box<dyn Source>>,
}

inventory::collect!(SourceRegistration);

#[macro_export]
macro_rules! register_source {
    ($name:expr, $probe:expr) => {
        inventory::submit! {
            $crate::sensors::SourceRegistration { name: $name, probe: $probe }
        }
    };
}

/// Butun kaynaklari yoklar, calisanlari toplar ve periyodik ornekler.
pub struct Sensors {
    active: Vec<(Box<dyn Source>, Instant)>,
    missing: Vec<&'static str>,
    snapshot: Snapshot,
}

impl Sensors {
    /// Kayitli butun kaynaklari yoklar.
    pub fn probe_all() -> Sensors {
        let now = Instant::now();
        let mut active = Vec::new();
        let mut missing = Vec::new();
        for reg in inventory::iter::<SourceRegistration> {
            match (reg.probe)() {
                Some(s) => active.push((s, now)),
                None => missing.push(reg.name),
            }
        }
        Sensors {
            active,
            missing,
            snapshot: Snapshot::default(),
        }
    }

    /// Bu makinede calisan kaynaklarin adlari.
    pub fn active_names(&self) -> Vec<&'static str> {
        self.active.iter().map(|(s, _)| s.name()).collect()
    }

    /// Yoklanip bulunamayan kaynaklarin adlari. Hata degil, bilgi.
    pub fn missing_names(&self) -> &[&'static str] {
        &self.missing
    }

    /// Zamani gelen kaynaklari orneklar.
    pub fn poll(&mut self) {
        let now = Instant::now();
        for (src, next) in self.active.iter_mut() {
            if now < *next {
                continue;
            }
            *next = now + src.interval();
            src.sample(&mut self.snapshot);
        }
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oran_hesabi_eksik_veride_none() {
        assert_eq!(Snapshot::ratio(Some(50), Some(100)), Some(50.0));
        assert_eq!(Snapshot::ratio(None, Some(100)), None);
        assert_eq!(Snapshot::ratio(Some(50), None), None);
        // Sifira bolme yok.
        assert_eq!(Snapshot::ratio(Some(50), Some(0)), None);
    }

    #[test]
    fn bos_goruntu_hepsi_none() {
        let s = Snapshot::default();
        assert!(s.cpu_percent.is_none());
        assert!(s.gpu_percent.is_none());
        assert!(s.mem_percent().is_none());
    }

    /// Kaynaklarin yoklanmasi cokmemeli, hicbiri calismasa bile.
    #[test]
    fn yoklama_cokmuyor() {
        let mut s = Sensors::probe_all();
        s.poll();
        let _ = s.snapshot();
    }
}
