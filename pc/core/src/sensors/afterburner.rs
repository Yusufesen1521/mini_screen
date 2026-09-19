//! MSI Afterburner paylasimli bellegi. Windows'a ozel.
//!
//! **Neden bu kaynak:** plan GPU icin NVIDIA'da NVML, AMD'de ADLX
//! diyordu, yani satici basina ayri bir SDK. Afterburner'in paylasimli
//! bellegi satici bagimsiz: ayni yapidan hem NVIDIA hem AMD okunuyor.
//! Ustelik CPU sicakligi gibi anakart sensorlerini de veriyor, ki
//! `sysinfo` Windows'ta onlari vermiyor.
//!
//! **Dagitim notu:** Afterburner'i kurulumcuya gommek EULA'ya tabi ve
//! izinsiz paketlemek riskli. Dogru yaklasim tespit edip kullaniciyi
//! yonlendirmek. Bu yuzden burada kaynak opsiyonel: Afterburner kapaliysa
//! paylasimli bellek yok, `probe` None doner, ilgili alanlar `None`
//! kalir ve widget o satirlari hic cizmez.
//!
//! Yapi tanimi Afterburner SDK'sindaki `MAHM_SHARED_MEMORY_HEADER` ve
//! `MAHM_SHARED_MEMORY_ENTRY` kayitlariyla ayni. Alan sirasi degisirse
//! imza ve surum kontrolu bunu yakalar.

use std::time::Duration;

use crate::register_source;
use crate::sensors::{Snapshot, Source};

/// 'MAHM' imzasi. SDK bunu MSVC coklu karakter sabiti olarak tanimliyor,
/// yani 'M'<<24 | 'A'<<16 | 'H'<<8 | 'M'. Bayt sirasi ters hesaplanirsa
/// "MHAM" cikiyor ve imza tutmuyor; bir kez oyle yanildik.
const SIGNATURE_MAHM: u32 = 0x4D41484D;
/// Afterburner kapanirken imzayi bu degere ceviriyor.
const SIGNATURE_DEAD: u32 = 0xDEAD;
/// Denenecek esleme adlari. Afterburner yukseltilmis calisirsa nesne
/// Global ad alaninda olabiliyor, ikisi de deneniyor.
const MAP_NAMES: &[&[u8]] = &[b"MAHMSharedMemory\0", b"Global\\MAHMSharedMemory\0"];

/// Sadece okuma icin eslemek yeterli.
const FILE_MAP_READ: u32 = 0x0004;

/// Ornekleme araligi. Afterburner kendi polling araligiyla guncelliyor,
/// daha sik okumanin anlami yok.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(1000);

/// Girdi adlari. **Tam esitlik araniyor**, icerme degil: bu makinede
/// "GPU temperature" ile "GPU temperature 2" ayni anda var ve icerme
/// aramasi ikincisini birincinin ustune yaziyordu. Ayni sekilde
/// "CPU temperature" cekirdek basina "CPU1 temperature" gibi
/// kardeslere sahip.
///
/// Adlar `szSrcName`, yani yerellestirilmemis ad. Kullanicinin dili
/// degisse bile bunlar sabit kaliyor.
const KEY_GPU_TEMP: &str = "GPU temperature";
const KEY_GPU_USAGE: &str = "GPU usage";
const KEY_GPU_MEM_USAGE: &str = "Memory usage";
const KEY_CPU_TEMP: &str = "CPU temperature";

/// Bir olcumun okunamadigini Afterburner bu deger ile bildiriyor.
const UNKNOWN_VALUE: f32 = -1.0;

#[cfg(windows)]
mod imp {
    use super::*;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS,
    };

    /// Tek bir olcum girdisi.
    pub struct Entry {
        pub name: String,
        pub value: f32,
        /// `maxLimit`. Deger ile birlikte oran hesaplamak icin.
        pub max: f32,
    }

    /// Esleme suresince acik kalan tutamak ve gorunum.
    pub struct SharedMem {
        handle: HANDLE,
        view: MEMORY_MAPPED_VIEW_ADDRESS,
    }

    // SAFETY: icerdigi tutamak ve gorunum surec omru boyunca gecerli ve
    // bu yapi bellege yalnizca okuma icin eriyor. Ham isaretci oldugu
    // icin derleyici Send cikaramiyor; iplikler arasi tasinmasi guvenli.
    unsafe impl Send for SharedMem {}

    impl SharedMem {
        pub fn open() -> Option<SharedMem> {
            Self::open_diag().0
        }

        /// Acmayi dener, basarisizsa son Win32 hata kodunu da dondurur.
        /// Tanilama icin: 2 = ad bulunamadi, 5 = erisim reddedildi.
        pub fn open_diag() -> (Option<SharedMem>, Vec<(String, u32)>) {
            let mut errors = Vec::new();
            for name in MAP_NAMES {
                // SAFETY: adlarin hepsi sifirla bitiyor. Basarisizlikta
                // Windows null tutamak doner, kontrol ediliyor.
                unsafe {
                    let handle = OpenFileMappingA(FILE_MAP_READ, 0, name.as_ptr());
                    if handle.is_null() {
                        let label = String::from_utf8_lossy(&name[..name.len() - 1]).into_owned();
                        errors.push((label, GetLastError()));
                        continue;
                    }
                    let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0);
                    if view.Value.is_null() {
                        let label = String::from_utf8_lossy(&name[..name.len() - 1]).into_owned();
                        errors.push((label, GetLastError()));
                        CloseHandle(handle);
                        continue;
                    }
                    return (Some(SharedMem { handle, view }), errors);
                }
            }
            (None, errors)
        }

        fn base(&self) -> *const u8 {
            self.view.Value as *const u8
        }

        fn read_u32(&self, off: usize) -> u32 {
            // SAFETY: cagiranlar offseti basliga gore hesapliyor ve
            // basligin varligi imza kontrolu ile dogrulanmis oluyor.
            unsafe { (self.base().add(off) as *const u32).read_unaligned() }
        }

        fn read_f32(&self, off: usize) -> f32 {
            unsafe { (self.base().add(off) as *const f32).read_unaligned() }
        }

        /// Sifirla biten ASCII alan. Afterburner bu alanlari MAX_PATH
        /// boyunda sabit uzunlukta tutuyor.
        fn read_cstr(&self, off: usize, max: usize) -> String {
            // SAFETY: off + max her zaman eslenmis bolgenin icinde,
            // cunku girdi boyutu baslikta bildiriliyor.
            let bytes = unsafe { std::slice::from_raw_parts(self.base().add(off), max) };
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(max);
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        }

        /// Baslik alanlarini oldugu gibi dondurur. Tanilama icin.
        pub fn header(&self) -> [u32; 8] {
            let mut h = [0u32; 8];
            for (i, v) in h.iter_mut().enumerate() {
                *v = self.read_u32(i * 4);
            }
            h
        }

        /// Baslik gecerli mi. Afterburner kapanirken imzayi DEAD yapiyor.
        pub fn is_live(&self) -> bool {
            let sig = self.read_u32(0);
            if sig == SIGNATURE_DEAD {
                return false;
            }
            sig == SIGNATURE_MAHM
        }

        /// Butun girdileri gezer.
        ///
        /// Her girdi: ad, deger, alt sinir, ust sinir. Ust sinir ise
        /// yariyor: "Memory usage" girdisinin ust siniri toplam VRAM.
        pub fn entries(&self) -> Vec<Entry> {
            if !self.is_live() {
                return Vec::new();
            }
            let header_size = self.read_u32(8) as usize;
            let num_entries = self.read_u32(12) as usize;
            let entry_size = self.read_u32(16) as usize;

            // Akil saglamasi: bozuk bir baslikla gigabaytlarca okumayalim.
            if entry_size == 0 || num_entries == 0 || num_entries > 4096 {
                return Vec::new();
            }

            const MAX_PATH: usize = 260;
            // szSrcName ilk alan, deger bes adet MAX_PATH alandan sonra.
            const DATA_OFFSET: usize = MAX_PATH * 5;
            if entry_size < DATA_OFFSET + 4 {
                return Vec::new();
            }

            let mut out = Vec::with_capacity(num_entries);
            for i in 0..num_entries {
                let base = header_size + i * entry_size;
                out.push(Entry {
                    name: self.read_cstr(base, MAX_PATH),
                    value: self.read_f32(base + DATA_OFFSET),
                    max: self.read_f32(base + DATA_OFFSET + 8),
                });
            }
            out
        }
    }

    impl Drop for SharedMem {
        fn drop(&mut self) {
            // SAFETY: ikisi de bu yapinin actigi kaynaklar.
            unsafe {
                UnmapViewOfFile(self.view);
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
pub struct AfterburnerSource {
    mem: imp::SharedMem,
}

#[cfg(windows)]
impl AfterburnerSource {
    fn probe() -> Option<Box<dyn Source>> {
        let mem = imp::SharedMem::open()?;
        if !mem.is_live() {
            return None;
        }
        // En az bir tanidik girdi yoksa bu bellek bize yaramaz.
        let entries = mem.entries();
        if entries.is_empty() {
            return None;
        }
        Some(Box::new(AfterburnerSource { mem }))
    }

    /// Girdi listesini oldugu gibi dondurur. Tanilama komutu kullaniyor.
    pub fn dump() -> Vec<(String, f32, f32)> {
        match imp::SharedMem::open() {
            Some(m) => m
                .entries()
                .into_iter()
                .map(|e| (e.name, e.value, e.max))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Acilamadiysa hangi ad hangi hata kodu ile basarisiz oldu.
    pub fn diagnose() -> Vec<(String, u32)> {
        imp::SharedMem::open_diag().1
    }

    /// Ham baslik alanlari. Esleme acilamadiysa None.
    pub fn header() -> Option<[u32; 8]> {
        imp::SharedMem::open().map(|m| m.header())
    }
}

#[cfg(windows)]
fn mb_to_bytes(mb: f32) -> u64 {
    (mb.max(0.0) as u64) * 1024 * 1024
}

#[cfg(windows)]
fn usable(v: f32) -> Option<f32> {
    if v.is_finite() && v > UNKNOWN_VALUE {
        Some(v)
    } else {
        None
    }
}

#[cfg(windows)]
impl Source for AfterburnerSource {
    fn name(&self) -> &'static str {
        "afterburner"
    }

    fn interval(&self) -> Duration {
        SAMPLE_INTERVAL
    }

    fn sample(&mut self, out: &mut Snapshot) {
        // Afterburner kapatildiysa imza olur. Alanlara dokunmuyoruz,
        // onceki degerler kalir ve bir sonraki turda da gelmezse
        // widget eski veriyi gosterir. Bu bilincli: tek bir okuma
        // hatasinda ekranin bosalmasi daha kotu olurdu.
        if !self.mem.is_live() {
            return;
        }
        for e in self.mem.entries() {
            let Some(v) = usable(e.value) else { continue };
            match e.name.as_str() {
                KEY_GPU_TEMP => out.gpu_temp_c = Some(v),
                KEY_GPU_USAGE => out.gpu_percent = Some(v),
                KEY_GPU_MEM_USAGE => {
                    // Afterburner VRAM'i megabayt olarak veriyor.
                    out.gpu_mem_used = Some(mb_to_bytes(v));
                    // Ust sinir toplam VRAM. Sifir ya da anlamsizsa
                    // toplami hic bildirmiyoruz, widget oran cizmez.
                    if e.max > 0.0 && e.max.is_finite() {
                        out.gpu_mem_total = Some(mb_to_bytes(e.max));
                    }
                }
                KEY_CPU_TEMP => out.cpu_temp_c = Some(v),
                _ => {}
            }
        }
    }
}

#[cfg(windows)]
register_source!("afterburner", AfterburnerSource::probe);

// Windows disinda bu kaynak hic kayitli degil; `missing` listesinde de
// gorunmez, cunku o platformda anlami yok.
#[cfg(not(windows))]
pub fn dump() -> Vec<(String, f32, f32)> {
    Vec::new()
}
