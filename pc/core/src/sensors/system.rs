//! Anahtarsiz ve yetkisiz okunabilen temel metrikler.
//!
//! Plan 2.4'un "once anahtarsiz ve yetkisiz alinabilenler" maddesi:
//! CPU kullanimi, RAM, disk, ag. `sysinfo` uzerinden, uc platformda da
//! ayni kod. Yonetici hakki istemiyor.
//!
//! CPU sicakligi burada opsiyonel: Linux'ta `/sys/class/hwmon`
//! uzerinden genelde geliyor, Windows'ta cogu makinede gelmiyor. Gelmezse
//! alan `None` kalir ve widget o satiri gizler. HWiNFO kaynagi ayri bir
//! dosya olacak.

use std::time::{Duration, Instant};

use sysinfo::{Components, Disks, Networks, System};

use crate::register_source;
use crate::sensors::{Snapshot, Source};

/// Ornekleme araligi. `sysinfo` CPU yuzdesini iki ornek arasindaki
/// farktan hesapliyor, cok sik cagirmak gurultulu deger veriyor.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(1000);

/// CPU sicakligi icin aranan bilesen adlari, sirayla.
const CPU_TEMP_LABELS: &[&str] = &["Tctl", "Tdie", "Package id 0", "CPU", "coretemp"];

pub struct SystemSource {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last_net: Option<Instant>,
}

impl SystemSource {
    fn probe() -> Option<Box<dyn Source>> {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        // Bellek toplami okunamiyorsa bu kaynak bu makinede ise yaramaz.
        if sys.total_memory() == 0 {
            return None;
        }
        Some(Box::new(SystemSource {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last_net: None,
        }))
    }

    fn cpu_temperature(&mut self) -> Option<f32> {
        self.components.refresh(false);
        for label in CPU_TEMP_LABELS {
            for c in self.components.iter() {
                if c.label().contains(label) {
                    if let Some(t) = c.temperature() {
                        return Some(t);
                    }
                }
            }
        }
        None
    }
}

impl Source for SystemSource {
    fn name(&self) -> &'static str {
        "system"
    }

    fn interval(&self) -> Duration {
        SAMPLE_INTERVAL
    }

    fn sample(&mut self, out: &mut Snapshot) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        out.cpu_percent = Some(self.sys.global_cpu_usage());
        out.cpu_cores = Some(self.sys.cpus().len()).filter(|n| *n > 0);
        out.mem_used = Some(self.sys.used_memory());
        out.mem_total = Some(self.sys.total_memory());
        // **Okuyamadigimiz degeri silmiyoruz.** Snapshot kaynaklar
        // arasinda ortak ve birikimli; duz atama yapinca bu satir
        // Windows'ta Afterburner'in okudugu CPU sicakligini her turda
        // None ile eziyordu. Olculdu: Afterburner "CPU temperature"
        // 63.6 C verirken panelde o satir hic cizilmiyordu.
        // sysinfo Windows'ta CPU sicakligi vermiyor, Linux'ta veriyor.
        if let Some(t) = self.cpu_temperature() {
            out.cpu_temp_c = Some(t);
        }

        // Disk: butun diskler toplanir. Tek bir surucu secmek Faz 4'te
        // ayar olacak, simdilik toplam daha durust bir varsayilan.
        self.disks.refresh(false);
        let mut total = 0u64;
        let mut available = 0u64;
        for d in self.disks.iter() {
            total += d.total_space();
            available += d.available_space();
        }
        if total > 0 {
            out.disk_total = Some(total);
            out.disk_used = Some(total.saturating_sub(available));
        }

        // Ag: sysinfo yenilemeler arasindaki farki veriyor, biz de
        // gecen sureye bolup saniyeye normalize ediyoruz.
        self.networks.refresh(false);
        let now = Instant::now();
        if let Some(prev) = self.last_net {
            let secs = now.duration_since(prev).as_secs_f64();
            if secs > 0.0 {
                let mut rx = 0u64;
                let mut tx = 0u64;
                for data in self.networks.values() {
                    rx += data.received();
                    tx += data.transmitted();
                }
                out.net_rx_bps = Some((rx as f64 / secs) as u64);
                out.net_tx_bps = Some((tx as f64 / secs) as u64);
            }
        }
        self.last_net = Some(now);
    }
}

register_source!("system", SystemSource::probe);

#[cfg(test)]
mod tests {
    use super::*;

    /// Bu kaynak her gelistirme makinesinde calismali. Calismiyorsa
    /// sensor katmani degil, ortam bozuk demektir.
    #[test]
    fn temel_metrikler_okunuyor() {
        let mut src = SystemSource::probe().expect("system kaynagi yoklanamadi");
        let mut s = Snapshot::default();
        src.sample(&mut s);

        assert!(s.mem_total.unwrap_or(0) > 0, "toplam bellek sifir");
        assert!(s.mem_used.unwrap_or(0) > 0, "kullanilan bellek sifir");
        assert!(s.cpu_cores.unwrap_or(0) > 0, "cekirdek sayisi sifir");
        let cpu = s.cpu_percent.expect("cpu yuzdesi yok");
        assert!(
            (0.0..=100.0).contains(&cpu),
            "cpu yuzdesi araligin disinda: {}",
            cpu
        );
    }

    /// Baska bir kaynagin okudugu deger bu kaynak tarafindan silinmemeli.
    ///
    /// Gercek bir hataydi: Snapshot kaynaklar arasinda ortak ve
    /// birikimli, bu kaynak ise CPU sicakligini duz atama ile
    /// yaziyordu. Windows'ta sysinfo sicaklik vermedigi icin her turda
    /// Afterburner'in okudugu deger None ile eziliyordu ve panelde o
    /// satir hic cizilmiyordu.
    #[test]
    fn baska_kaynagin_sicakligi_silinmiyor() {
        let mut src = SystemSource::probe().unwrap();
        // Afterburner'in yazmis olabilecegi bir deger.
        let mut s = Snapshot {
            cpu_temp_c: Some(63.6),
            ..Snapshot::default()
        };
        src.sample(&mut s);
        assert!(
            s.cpu_temp_c.is_some(),
            "baska kaynagin okudugu CPU sicakligi silindi"
        );
    }

    /// Ilk orneklemede ag hizi hesaplanamaz, ikincide hesaplanir.
    /// Onemli olan ilk turda cokmemesi ve None kalmasi.
    #[test]
    fn ag_hizi_ilk_ornekte_none() {
        let mut src = SystemSource::probe().unwrap();
        let mut s = Snapshot::default();
        src.sample(&mut s);
        assert!(s.net_rx_bps.is_none(), "ilk ornekte ag hizi hesaplanmamali");
        src.sample(&mut s);
        assert!(s.net_rx_bps.is_some(), "ikinci ornekte ag hizi gelmeli");
    }
}
