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
//! yonlendirmek. Bu yuzden kaynak opsiyonel: Afterburner kapaliysa
//! paylasimli bellek yok, `probe` None doner, alanlar `None` kalir ve
//! widget o satirlari hic cizmez.
//!
//! **Neden her sey tek bir `cfg(windows)` modulunde:** sabitleri ve
//! `use` satirlarini disarida birakinca Windows disi derlemelerde hepsi
//! "kullanilmiyor" uyarisi veriyor ve CI `-D warnings` ile dusuyordu.
//! Ilk denemede tam olarak bu oldu. Platforma ozel kod tek parca
//! tutulursa o platform disinda uyarabilecek bir sey kalmiyor.

#[cfg(windows)]
mod win {
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS,
    };

    use crate::register_source;
    use crate::sensors::{Snapshot, Source};

    /// 'MAHM' imzasi. SDK bunu MSVC coklu karakter sabiti olarak
    /// tanimliyor, yani 'M'<<24 | 'A'<<16 | 'H'<<8 | 'M'. Bayt sirasi
    /// ters hesaplanirsa "MHAM" cikiyor ve imza hic tutmuyor; ilk
    /// denemede oyle yanildik, ham baslik dokumu ortaya cikardi.
    const SIGNATURE_MAHM: u32 = 0x4D41484D;
    /// Afterburner kapanirken imzayi bu degere ceviriyor.
    const SIGNATURE_DEAD: u32 = 0xDEAD;

    /// Denenecek esleme adlari. Afterburner yukseltilmis calisirsa nesne
    /// Global ad alaninda olabiliyor, ikisi de deneniyor.
    const MAP_NAMES: &[&[u8]] = &[b"MAHMSharedMemory\0", b"Global\\MAHMSharedMemory\0"];

    /// Sadece okuma icin eslemek yeterli.
    const FILE_MAP_READ: u32 = 0x0004;

    /// Ornekleme araligi. Afterburner kendi araligiyla guncelliyor,
    /// daha sik okumanin anlami yok.
    const SAMPLE_INTERVAL: Duration = Duration::from_millis(1000);

    /// Girdi adlari. **Tam esitlik araniyor, icerme degil:** bu makinede
    /// "GPU temperature" ile "GPU temperature 2" ayni anda var ve icerme
    /// aramasi ikincisini birincinin ustune yaziyordu. Ayni tuzak
    /// "CPU temperature" icin de gecerli, cunku cekirdek basina
    /// "CPU1 temperature" gibi kardesleri var.
    ///
    /// Adlar `szSrcName`, yani yerellestirilmemis ad. Kullanicinin dili
    /// degisse bile bunlar sabit kaliyor.
    const KEY_GPU_TEMP: &str = "GPU temperature";
    const KEY_GPU_USAGE: &str = "GPU usage";
    const KEY_GPU_MEM_USAGE: &str = "Memory usage";
    const KEY_CPU_TEMP: &str = "CPU temperature";

    /// GPU girdisi icindeki alan sirasi. Hepsi MAX_PATH boyunda:
    /// `szGpuId`, `szFamily`, `szDevice`, `szDriver`, `szBIOS`, sonra
    /// `dwMemAmount`. Ekranda gosterilecek ad icin once `szDevice`
    /// (surucunun bildirdigi tam ad), o bossa `szFamily` kullaniliyor.
    const GPU_FAMILY_OFFSET: usize = MAX_PATH;
    const GPU_DEVICE_OFFSET: usize = MAX_PATH * 2;
    /// Bir GPU girdisinin en az bu kadar olmasi gerekiyor: bes ad alani
    /// ve bellek miktari.
    const MIN_GPU_ENTRY_SIZE: usize = MAX_PATH * 5 + 4;
    const MAX_SANE_GPU_ENTRY_SIZE: usize = 8192;
    const MAX_SANE_GPUS: usize = 16;

    /// `dwNumGpuEntries` ve `dwGpuEntrySize` icin denenecek baslik
    /// offsetleri.
    ///
    /// **Neden iki aday:** baslikta bu iki alandan once bir `time_t`
    /// duruyor ve `time_t` derleme secenegine gore 4 ya da 8 bayt.
    /// Hangisi oldugunu belgeye bakarak kestirmek yerine ikisini de
    /// deneyip akil saglamasindan gecen ciftti aliyoruz. Ad okumasi
    /// zaten dogrulaniyor, yanlis cift sessizce cope gidiyor.
    const GPU_COUNT_OFFSETS: &[(usize, usize)] = &[(24, 28), (28, 32)];

    /// Bir olcumun okunamadigini Afterburner bu deger ile bildiriyor.
    const UNKNOWN_VALUE: f32 = -1.0;

    /// Girdi yapisinda bes adet MAX_PATH alan var, deger onlardan sonra.
    const MAX_PATH: usize = 260;
    const DATA_OFFSET: usize = MAX_PATH * 5;
    /// `maxLimit`, degerden iki alan sonra.
    const MAX_LIMIT_OFFSET: usize = DATA_OFFSET + 8;
    /// Bozuk bir baslikla gigabaytlarca okumayalim.
    const MAX_SANE_ENTRIES: usize = 4096;

    /// Tek bir olcum girdisi.
    pub struct Entry {
        pub name: String,
        pub value: f32,
        /// `maxLimit`. "Memory usage" icin toplam VRAM.
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
                // Windows null doner, ikisi de kontrol ediliyor.
                unsafe {
                    let label = String::from_utf8_lossy(&name[..name.len() - 1]).into_owned();
                    let handle = OpenFileMappingA(FILE_MAP_READ, 0, name.as_ptr());
                    if handle.is_null() {
                        errors.push((label, GetLastError()));
                        continue;
                    }
                    let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0);
                    if view.Value.is_null() {
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
            // SAFETY: offsetler baslikta bildirilen olculere gore
            // hesaplaniyor, baslik gecerliligi imza ile dogrulaniyor.
            unsafe { (self.base().add(off) as *const u32).read_unaligned() }
        }

        fn read_f32(&self, off: usize) -> f32 {
            unsafe { (self.base().add(off) as *const f32).read_unaligned() }
        }

        /// Sifirla biten ASCII alan. Afterburner bu alanlari MAX_PATH
        /// boyunda sabit uzunlukta tutuyor.
        fn read_cstr(&self, off: usize, max: usize) -> String {
            // SAFETY: off + max eslenmis bolgenin icinde, cunku girdi
            // boyutu baslikta bildiriliyor ve akil saglamasindan geciyor.
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

        /// GPU adlari. Afterburner kac ekran karti goruyorsa o kadar.
        ///
        /// Girdiler olcum girdilerinden **sonra** duruyor, yani baslangic
        /// offseti `headerSize + numEntries * entrySize`. Bu hesap
        /// basliktaki `time_t` belirsizliginden etkilenmiyor; sadece kac
        /// girdi okunacagi icin baslik yoklaniyor.
        pub fn gpu_names(&self) -> Vec<String> {
            if !self.is_live() {
                return Vec::new();
            }
            let header_size = self.read_u32(8) as usize;
            let num_entries = self.read_u32(12) as usize;
            let entry_size = self.read_u32(16) as usize;
            if num_entries == 0 || num_entries > MAX_SANE_ENTRIES || entry_size == 0 {
                return Vec::new();
            }

            let Some((count, gpu_entry_size)) = self.gpu_entry_layout() else {
                return Vec::new();
            };

            let base = header_size + num_entries * entry_size;
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let e = base + i * gpu_entry_size;
                let device = self.read_cstr(e + GPU_DEVICE_OFFSET, MAX_PATH);
                let family = self.read_cstr(e + GPU_FAMILY_OFFSET, MAX_PATH);
                match printable(&device).or_else(|| printable(&family)) {
                    Some(name) => out.push(name),
                    None => return out,
                }
            }
            out
        }

        /// Kac GPU girdisi var ve her biri kac bayt. Akil saglamasindan
        /// gecen ilk aday offset cifti kazanir, hicbiri gecmezse `None`.
        fn gpu_entry_layout(&self) -> Option<(usize, usize)> {
            for (count_off, size_off) in GPU_COUNT_OFFSETS {
                let count = self.read_u32(*count_off) as usize;
                let size = self.read_u32(*size_off) as usize;
                if (1..=MAX_SANE_GPUS).contains(&count)
                    && (MIN_GPU_ENTRY_SIZE..=MAX_SANE_GPU_ENTRY_SIZE).contains(&size)
                {
                    return Some((count, size));
                }
            }
            None
        }

        /// Butun girdileri gezer.
        pub fn entries(&self) -> Vec<Entry> {
            if !self.is_live() {
                return Vec::new();
            }
            let header_size = self.read_u32(8) as usize;
            let num_entries = self.read_u32(12) as usize;
            let entry_size = self.read_u32(16) as usize;

            if entry_size < MAX_LIMIT_OFFSET + 4
                || num_entries == 0
                || num_entries > MAX_SANE_ENTRIES
            {
                return Vec::new();
            }

            let mut out = Vec::with_capacity(num_entries);
            for i in 0..num_entries {
                let base = header_size + i * entry_size;
                out.push(Entry {
                    name: self.read_cstr(base, MAX_PATH),
                    value: self.read_f32(base + DATA_OFFSET),
                    max: self.read_f32(base + MAX_LIMIT_OFFSET),
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

    pub struct AfterburnerSource {
        mem: SharedMem,
    }

    impl AfterburnerSource {
        fn probe() -> Option<Box<dyn Source>> {
            let mem = SharedMem::open()?;
            if !mem.is_live() || mem.entries().is_empty() {
                return None;
            }
            Some(Box::new(AfterburnerSource { mem }))
        }

        /// Girdi listesi. Tanilama komutu kullaniyor.
        pub fn dump() -> Vec<(String, f32, f32)> {
            match SharedMem::open() {
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
            SharedMem::open_diag().1
        }

        /// Ham baslik alanlari. Esleme acilamadiysa None.
        pub fn header() -> Option<[u32; 8]> {
            SharedMem::open().map(|m| m.header())
        }

        /// Afterburner'in gordugu GPU adlari. Tanilama komutu kullaniyor.
        pub fn gpu_names() -> Vec<String> {
            match SharedMem::open() {
                Some(m) => m.gpu_names(),
                None => Vec::new(),
            }
        }
    }

    /// Paylasimli bellekten okunan bir adin gercekten ad olup olmadigi.
    ///
    /// Offset tahmini tutmazsa buradan cop gelir. Bos, cok uzun ya da
    /// basilabilir ASCII disinda karakter iceren metin ad sayilmaz.
    fn printable(s: &str) -> Option<String> {
        let t = s.trim();
        if t.is_empty() || t.len() > 96 {
            return None;
        }
        if t.chars().any(|c| !(' '..='~').contains(&c)) {
            return None;
        }
        Some(t.to_string())
    }

    fn mb_to_bytes(mb: f32) -> u64 {
        (mb.max(0.0) as u64) * 1024 * 1024
    }

    fn usable(v: f32) -> Option<f32> {
        if v.is_finite() && v > UNKNOWN_VALUE {
            Some(v)
        } else {
            None
        }
    }

    impl Source for AfterburnerSource {
        fn name(&self) -> &'static str {
            "afterburner"
        }

        fn interval(&self) -> Duration {
            SAMPLE_INTERVAL
        }

        fn sample(&mut self, out: &mut Snapshot) {
            // Afterburner kapatildiysa imza olur. Alanlara dokunmuyoruz,
            // onceki degerler kalir. Bu bilincli: tek bir okuma
            // hatasinda ekranin bosalmasi daha kotu olurdu.
            if !self.mem.is_live() {
                return;
            }

            // Ad degismiyor ama her turda okumak ucuz ve kart takilip
            // cikarilma gibi durumlari kendiliginden takip ediyor.
            // Okunamazsa onceki deger korunuyor.
            if let Some(name) = self.mem.gpu_names().into_iter().next() {
                out.gpu_name = Some(name);
            }

            for e in self.mem.entries() {
                let Some(v) = usable(e.value) else { continue };
                match e.name.as_str() {
                    KEY_GPU_TEMP => out.gpu_temp_c = Some(v),
                    KEY_GPU_USAGE => out.gpu_percent = Some(v),
                    KEY_GPU_MEM_USAGE => {
                        // Afterburner VRAM'i megabayt olarak veriyor.
                        out.gpu_mem_used = Some(mb_to_bytes(v));
                        // Ust sinir toplam VRAM. Anlamsizsa toplami hic
                        // bildirmiyoruz, widget oran cizmez.
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

    register_source!("afterburner", AfterburnerSource::probe);
}

#[cfg(windows)]
pub use win::AfterburnerSource;

/// Windows disinda kaynak yok. Tip yine de duruyor, cunku tanilama
/// komutu butun platformlarda derleniyor.
#[cfg(not(windows))]
pub struct AfterburnerSource;

#[cfg(not(windows))]
impl AfterburnerSource {
    pub fn dump() -> Vec<(String, f32, f32)> {
        Vec::new()
    }

    pub fn diagnose() -> Vec<(String, u32)> {
        Vec::new()
    }

    pub fn header() -> Option<[u32; 8]> {
        None
    }

    pub fn gpu_names() -> Vec<String> {
        Vec::new()
    }
}
