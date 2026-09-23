//! Cihaz kesfi, baglanti ve akis kontrollu gonderim.
//!
//! Faz 1 protokolunun PC tarafi. Cizim burada yok: bu katman sadece
//! hazir piksel bolgelerini cihaza tasir.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use crate::protocol as p;

/// Espressif USB satici kimligi. Yerlesik USB CDC portu bununla bulunuyor.
pub const ESPRESSIF_VID: u16 = 0x303A;

const SERIAL_BAUD: u32 = 115_200;
const READ_TIMEOUT: Duration = Duration::from_millis(200);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);
const ACK_TIMEOUT: Duration = Duration::from_secs(2);
const READ_CHUNK: usize = 4096;
/// Inbox'ta en fazla bu kadar islenmemis cerceve tutulur.
const INBOX_LIMIT: usize = 64;
/// Onay beklerken bos dongu yapmamak icin kisa uyku.
const POLL_IDLE_SLEEP: Duration = Duration::from_millis(1);
/// Yeniden baglanma denemeleri arasindaki bekleme.
const RECONNECT_POLL: Duration = Duration::from_millis(250);
/// Acilista port bekleme suresi. Yeniden numaralandirma bunun altinda kaliyor.
pub const OPEN_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
/// Tanilama icin saklanan en fazla zaman asimi olayi. 24 saatlik kosuda
/// yaklasik 55 olay bekleniyor, bu sinir bellegi baglamak icin.
const TIMEOUT_LOG_LIMIT: usize = 256;
/// Vazgecilen siralarin hatirlanma suresi.
///
/// Sira numarasi u8, yani 256 cercevede bir tekrar ediyor. 24 FPS'te bu
/// yaklasik 10 saniye. Vazgecilen bir sirayi bundan uzun tutarsak ayni
/// numarayi tasiyan **yeni** bir cercevenin onayini "gec gelen onay"
/// sanariz. Bu sure hem o tekrardan hem de ACK_TIMEOUT'tan kisa.
const ABANDONED_TTL: Duration = Duration::from_secs(4);

#[derive(Debug, Clone)]
pub struct Caps {
    pub proto_version: u8,
    pub fw_major: u8,
    pub fw_minor: u8,
    pub fw_patch: u8,
    pub width: u16,
    pub height: u16,
    pub codecs: u8,
    pub max_payload: u16,
    pub rx_slots: u8,
    pub selftest_ok: bool,
    pub mac: [u8; 6],
}

impl Caps {
    /// CAPS payload duzeni `docs/protocol.md` icinde tablo halinde.
    fn parse(b: &[u8]) -> Option<Caps> {
        if b.len() < 20 {
            return None;
        }
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&b[14..20]);
        Some(Caps {
            proto_version: b[0],
            fw_major: b[1],
            fw_minor: b[2],
            fw_patch: b[3],
            width: u16::from_le_bytes([b[4], b[5]]),
            height: u16::from_le_bytes([b[6], b[7]]),
            codecs: b[9],
            max_payload: u16::from_le_bytes([b[10], b[11]]),
            rx_slots: b[12],
            selftest_ok: b[13] == 1,
            mac,
        })
    }

    pub fn firmware(&self) -> String {
        format!("{}.{}.{}", self.fw_major, self.fw_minor, self.fw_patch)
    }

    pub fn mac_string(&self) -> String {
        self.mac
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":")
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LinkStats {
    pub nacks: u32,
    pub ack_timeouts: u32,
    pub frames_sent: u64,
    pub bytes_sent: u64,
}

/// Tek bir ACK zaman asimi olayinin cevresi.
///
/// Seyrek zaman asiminin sebebi bulunamadi ve 24 saatlik cikis kriterinin
/// onunde duruyor. Tahmin yerine olcum: vazgecme aninda dogru olan ne
/// varsa kaydediliyor. Ayirt edici iki alan `waiting_bytes` ve
/// `bytes_during`; hangi tarafin suclu oldugunu onlar soyluyor.
#[derive(Debug, Clone, Copy)]
pub struct AckTimeout {
    /// Baglanti acildigindan beri gecen sure.
    pub at: Duration,
    /// Vazgecilen cercevenin sira numarasi.
    pub seq: u8,
    /// O ana kadar gonderilen cerceve ve bayt. Olay bir sayacin belirli
    /// bir degerinde tekrarliyorsa buradan gorunur.
    pub frames_sent: u64,
    pub bytes_sent: u64,
    /// Vazgecerken surucude okunmayi bekleyen bayt. Sifirdan buyukse
    /// veri gelmisti ve biz isleyemedik, yani kabahat PC tarafinda.
    pub waiting_bytes: u32,
    /// Iki saniyelik bekleyis boyunca porttan okunan toplam bayt.
    /// Sifirsa hat tamamen sustu (surucu askiya alma ya da cihaz
    /// takildi); sifirdan buyukse trafik akiyordu ama bu onay gelmedi.
    pub bytes_during: usize,
    /// Vazgectikten sonra kalan bekleyen cerceve sayisi.
    pub pending: usize,
}

/// Onay gecikmesi tanilamasi.
#[derive(Debug, Default, Clone)]
pub struct AckDiag {
    /// Basarili bekleyislerin en uzunu. Tuketen sifirlar.
    ///
    /// Zaman asimindan cok daha sik olusan "kil payi" bekleyisleri
    /// gosterir. Bu deger ACK_TIMEOUT'a yaklasiyorsa olay nadir bir
    /// kayip degil, surekli bir gecikmenin ucudur.
    pub max_wait: Duration,
    /// Vazgecildikten sonra yine de gelen onay sayisi.
    pub late_acks: u32,
    /// Gec gelen onaylarin vazgecme anina gore en buyuk gecikmesi.
    pub max_late: Duration,
    /// Beklenmeyen cerceve tipleri. Sessizce inbox'ta birikmelerini
    /// engelliyoruz: onceki ACK hatasi tam olarak boyle gizlenmisti.
    pub unexpected: u32,
    /// Zaman asimi olaylari, olus sirasiyla.
    pub timeouts: Vec<AckTimeout>,
}

/// Bulunan bir cihaz portu.
#[derive(Debug, Clone)]
pub struct DevicePort {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub serial: Option<String>,
}

/// VID ile cihaz portlarini bulur.
///
/// Cihaz cikarilip takildiginda port ayni isimle geri geliyor, Faz 1'de
/// olculdu. Yine de isim degisebilir varsayimiyla her seferinde aranir.
pub fn find_ports() -> Vec<DevicePort> {
    let mut out = Vec::new();
    let ports = match serialport::available_ports() {
        Ok(v) => v,
        Err(_) => return out,
    };
    for info in ports {
        if let serialport::SerialPortType::UsbPort(usb) = &info.port_type {
            if usb.vid == ESPRESSIF_VID {
                out.push(DevicePort {
                    name: info.port_name.clone(),
                    vid: usb.vid,
                    pid: usb.pid,
                    serial: usb.serial_number.clone(),
                });
            }
        }
    }
    out
}

#[derive(Debug)]
pub enum LinkError {
    NotFound,
    Io(std::io::Error),
    Serial(serialport::Error),
    /// El sikisma zaman asimina ugradi. Genelde cihazda protokol firmware'i yok.
    NoCaps,
    /// Cihaz kendini sinamayi gecemedi, protokol guvenilmez.
    SelfTestFailed,
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::NotFound => write!(
                f,
                "Cihaz bulunamadi. Yerlesik USB portu takili mi? \
                 'ports' komutuyla bakabilirsin."
            ),
            LinkError::Io(e) => write!(f, "G/C hatasi: {}", e),
            LinkError::Serial(e) => write!(f, "Seri port hatasi: {}", e),
            LinkError::NoCaps => write!(f, "CAPS gelmedi. Cihaz protokol firmware'i ile yuklu mu?"),
            LinkError::SelfTestFailed => {
                write!(f, "Cihaz kendini sinamayi gecemedi, protokol guvenilmez.")
            }
        }
    }
}

impl std::error::Error for LinkError {}

impl From<std::io::Error> for LinkError {
    fn from(e: std::io::Error) -> Self {
        LinkError::Io(e)
    }
}

impl From<serialport::Error> for LinkError {
    fn from(e: serialport::Error) -> Self {
        LinkError::Serial(e)
    }
}

/// Cihazla kurulmus baglanti.
///
/// Akis kontrolu kaydirmali pencere. Cihaz CAPS ile `rx_slots` bildiriyor,
/// pencere o kadar cerceve acik tutuyor. Ayrintisi `docs/protocol.md`.
pub struct Link {
    port: Box<dyn serialport::SerialPort>,
    parser: p::Parser,
    seq: u8,
    window: usize,
    pending: Vec<u8>,
    pub caps: Option<Caps>,
    pub stats: LinkStats,
    pub diag: AckDiag,
    /// Baglantinin acildigi an. Olaylarin zamani buna gore.
    opened: Instant,
    /// Vazgecilen siralar ve vazgecme anlari. Gec gelen onayi yakalamak
    /// icin; `ABANDONED_TTL` gecince atiliyor.
    abandoned: Vec<(u8, Instant)>,
    /// Cihazdan gelen LOG satirlari. Tuketen bosaltir.
    pub logs: Vec<String>,
    /// Cihaz paneli yeniden init etti ve tam kare istedi. Tuketen siler.
    need_full: bool,
    // Sicak yolda yeniden kullanilan tamponlar. Kare basina tahsis yok.
    frame_buf: Vec<u8>,
    region_buf: Vec<u8>,
    rle_buf: Vec<u8>,
    read_buf: Vec<u8>,
    inbox: Vec<p::Frame>,
}

impl Link {
    pub fn open(port_name: Option<&str>) -> Result<Link, LinkError> {
        let name = match port_name {
            Some(n) => n.to_string(),
            None => find_ports()
                .first()
                .ok_or(LinkError::NotFound)?
                .name
                .clone(),
        };
        let port = serialport::new(&name, SERIAL_BAUD)
            .timeout(READ_TIMEOUT)
            .open()?;
        let _ = port.clear(serialport::ClearBuffer::Input);

        let mut link = Link {
            port,
            parser: p::Parser::new(),
            seq: 0,
            window: 1,
            pending: Vec::new(),
            caps: None,
            stats: LinkStats::default(),
            diag: AckDiag::default(),
            opened: Instant::now(),
            abandoned: Vec::new(),
            logs: Vec::new(),
            need_full: false,
            frame_buf: Vec::with_capacity(p::MAX_PAYLOAD),
            region_buf: Vec::with_capacity(p::MAX_PAYLOAD),
            rle_buf: Vec::with_capacity(p::MAX_PAYLOAD),
            read_buf: vec![0u8; READ_CHUNK],
            inbox: Vec::new(),
        };
        let _ = link.port.set_timeout(READ_TIMEOUT);
        link.write_timeout_best_effort();
        Ok(link)
    }

    /// Port gelene kadar deneyerek acar.
    ///
    /// Gerekcesi olculdu: cihaz USB CDC portunu kapatinca yeniden
    /// numaralandiriliyor ve o pencerede acma "var olmayan aygit" ile
    /// basarisiz oluyor. Arka arkaya iki komut calistirinca ikincisi
    /// duzenli olarak bu pencereye denk geliyordu.
    pub fn open_retry(port_name: Option<&str>, timeout: Duration) -> Result<Link, LinkError> {
        let deadline = Instant::now() + timeout;
        loop {
            match Link::open(port_name) {
                Ok(l) => return Ok(l),
                Err(e) => {
                    if Instant::now() >= deadline {
                        return Err(e);
                    }
                }
            }
            std::thread::sleep(RECONNECT_POLL);
        }
    }

    fn write_timeout_best_effort(&mut self) {
        // serialport 4.x'te yazma zaman asimi acilista veriliyor; burada
        // ayri ayarlanamiyorsa sessizce gecilir, okuma tarafi zaten sinirli.
        let _ = WRITE_TIMEOUT;
    }

    fn next_seq(&mut self) -> u8 {
        let s = self.seq;
        self.seq = self.seq.wrapping_add(1);
        s
    }

    /// El sikisma. HELLO gonderir, CAPS bekler, pencereyi cihazin
    /// bildirdigi slot sayisina ayarlar.
    pub fn handshake(&mut self) -> Result<Caps, LinkError> {
        let seq = self.next_seq();
        p::build_frame_into(&mut self.frame_buf, p::MSG_HELLO, seq, &[p::VERSION], 0);
        let frame = std::mem::take(&mut self.frame_buf);
        self.port.write_all(&frame)?;
        self.port.flush()?;
        self.frame_buf = frame;

        let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
        while Instant::now() < deadline {
            let _ = self.pump()?;
            if let Some(i) = self.inbox.iter().position(|f| f.msg_type == p::MSG_CAPS) {
                let f = self.inbox.remove(i);
                let caps = Caps::parse(&f.payload).ok_or(LinkError::NoCaps)?;
                if !caps.selftest_ok {
                    return Err(LinkError::SelfTestFailed);
                }
                self.window = caps.rx_slots.max(1) as usize;
                self.caps = Some(caps.clone());
                return Ok(caps);
            }
        }
        Err(LinkError::NoCaps)
    }

    /// Porttan okur, cerceveleri ayristirir, LOG olanlari ayiklar.
    /// Okunan bayt sayisini doner.
    ///
    /// Bloklamaz: once bekleyen bayt var mi diye bakar. Bloklayan okuma
    /// her turda 200 ms yiyordu ve tur hizini saniyede 48'den 6'ya
    /// dusuruyordu.
    fn pump(&mut self) -> Result<usize, LinkError> {
        let waiting = self.port.bytes_to_read().unwrap_or(0) as usize;
        if waiting == 0 {
            return Ok(0);
        }
        let want = waiting.min(self.read_buf.len());
        let n = match self.port.read(&mut self.read_buf[..want]) {
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
            Err(e) => return Err(LinkError::Io(e)),
        };
        if n == 0 {
            return Ok(0);
        }
        let mut fresh = Vec::new();
        let data = &self.read_buf[..n];
        self.parser.feed(data, &mut fresh);
        for f in fresh {
            if f.msg_type == p::MSG_LOG {
                let text = String::from_utf8_lossy(&f.payload);
                for line in text.lines() {
                    if !line.trim().is_empty() {
                        self.logs.push(line.to_string());
                    }
                }
            } else {
                self.inbox.push(f);
            }
        }
        self.trim_inbox();
        Ok(n)
    }

    /// Gelen onaylari isler. En az biri islendiyse true doner.
    fn process_inbox(&mut self) -> bool {
        let mut progressed = false;
        let mut i = 0;
        while i < self.inbox.len() {
            match self.inbox[i].msg_type {
                p::MSG_ACK => {
                    let seq = self.inbox[i].seq;
                    // Bekleyenler arasinda yoksa vazgectiklerimizden
                    // olabilir: o zaman onay kaybolmamis, gecikmis.
                    if !self.retire(seq) {
                        self.note_if_late(seq);
                    }
                    self.inbox.remove(i);
                    progressed = true;
                }
                p::MSG_NACK => {
                    self.stats.nacks += 1;
                    // NACK payload'inda ikinci bayt reddedilen seq.
                    if self.inbox[i].payload.len() >= 2 {
                        let seq = self.inbox[i].payload[1];
                        self.retire(seq);
                    }
                    self.inbox.remove(i);
                    progressed = true;
                }
                // PONG canlilik isaretinin cevabi, beklenen bir sey ama
                // islenecek bir tarafi yok. Birakirsak inbox sonsuza
                // kadar buyuyor: PING saniyede bir gidiyor ve her biri
                // bir PONG doguruyor.
                p::MSG_PONG => {
                    self.inbox.remove(i);
                }
                // Cihaz paneli yeniden init etti. Kirli takibi artik
                // yalan soyluyor: cihazin ekraninda hicbir sey yok ama
                // takipci onceki kareyi hatirliyor, yani sadece degisen
                // dikdortgenleri gonderirdi.
                p::MSG_NEED_FULL => {
                    self.need_full = true;
                    self.inbox.remove(i);
                    progressed = true;
                }
                // CAPS ve STATUS bekleyen bir cagrinin mali, dokunmuyoruz.
                p::MSG_CAPS | p::MSG_STATUS => i += 1,
                // Geri kalan hicbir sey beklenmiyor. Sayilip dusuruluyor:
                // sessizce inbox'ta birikmeleri onceki ACK hatasini tam
                // olarak boyle gizlemisti.
                _ => {
                    self.diag.unexpected += 1;
                    self.inbox.remove(i);
                }
            }
        }
        progressed
    }

    /// Bloklamadan porttan okur ve gelen onaylari isler.
    ///
    /// **Her turda cagrilmali, gonderilecek bir sey olmasa bile.**
    /// Gerekcesi olculdu: cihazin CDC TX tamponu 4096 bayt ve ACK ile
    /// LOG cerceveleri orada birikiyor. Dirty tracking sayesinde
    /// turlarin yuzde 96'si bos geciyor; sadece gonderirken okursak
    /// tampon tasiyor ve ACK'ler sessizce dusuyor. 20 saniyeden uzun
    /// kosularda kosu basina bir ACK zaman asimi bu yuzden olusuyordu.
    pub fn poll(&mut self) -> Result<(), LinkError> {
        let _ = self.pump()?;
        self.process_inbox();
        Ok(())
    }

    /// Onaylari toplar. En az bir ACK ya da NACK gelirse true doner,
    /// ikinci deger bekleyis boyunca porttan okunan bayt sayisi.
    ///
    /// O sayi zaman asimi olayinda ayirt edici: sifirsa hat tamamen
    /// sustu, sifirdan buyukse trafik akiyordu ama bu onay gelmedi.
    fn reap(&mut self, timeout: Duration) -> Result<(bool, usize), LinkError> {
        let deadline = Instant::now() + timeout;
        let mut seen = 0usize;
        while Instant::now() < deadline {
            seen += self.pump()?;
            if self.process_inbox() {
                return Ok((true, seen));
            }
            // pump bloklamiyor, bosa donmeyelim.
            std::thread::sleep(POLL_IDLE_SLEEP);
        }
        Ok((false, seen))
    }

    /// Bekleyenler listesinden dusurur. Gercekten orada miydi, onu doner.
    fn retire(&mut self, seq: u8) -> bool {
        if let Some(i) = self.pending.iter().position(|&s| s == seq) {
            self.pending.remove(i);
            true
        } else {
            false
        }
    }

    /// Vazgectigimiz bir cerceve icin onay sonradan geldi mi.
    ///
    /// Geldiyse olay "onay kayboldu" degil "onay gecikti" demektir ve
    /// sebep baska yerde aranir. Ayrimi olcmeden bilmek mumkun degil.
    fn note_if_late(&mut self, seq: u8) {
        self.prune_abandoned();
        if let Some(i) = self.abandoned.iter().position(|&(s, _)| s == seq) {
            let (_, when) = self.abandoned.remove(i);
            let late = when.elapsed();
            self.diag.late_acks += 1;
            if late > self.diag.max_late {
                self.diag.max_late = late;
            }
        }
    }

    /// Suresi dolan vazgecme kayitlarini atar. Ayrintisi ABANDONED_TTL.
    fn prune_abandoned(&mut self) {
        self.abandoned.retain(|&(_, when)| when.elapsed() < ABANDONED_TTL);
    }

    /// Bir zaman asimi olayini butun cevresiyle kaydeder.
    fn note_timeout(&mut self, seq: u8, bytes_during: usize) {
        let waiting_bytes = self.port.bytes_to_read().unwrap_or(0);
        if self.diag.timeouts.len() < TIMEOUT_LOG_LIMIT {
            self.diag.timeouts.push(AckTimeout {
                at: self.opened.elapsed(),
                seq,
                frames_sent: self.stats.frames_sent,
                bytes_sent: self.stats.bytes_sent,
                waiting_bytes,
                bytes_during,
                pending: self.pending.len(),
            });
        }
        self.prune_abandoned();
        self.abandoned.push((seq, Instant::now()));
    }

    /// Bir pencerede olculen en uzun basarili bekleyisi alir ve sifirlar.
    pub fn take_max_wait(&mut self) -> Duration {
        std::mem::take(&mut self.diag.max_wait)
    }

    /// Bir piksel bolgesini cihaza basar.
    ///
    /// Sikistirma kararini `encode_region_into` veriyor: RLE kazandirmazsa
    /// ham gidiyor. Pencere doluysa once yer acilir.
    pub fn send_region(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        pixels: &[u16],
    ) -> Result<(), LinkError> {
        let mut region = std::mem::take(&mut self.region_buf);
        let mut rle = std::mem::take(&mut self.rle_buf);
        p::encode_region_into(&mut region, &mut rle, x, y, w, h, pixels);

        let seq = self.next_seq();
        let mut frame = std::mem::take(&mut self.frame_buf);
        p::build_frame_into(
            &mut frame,
            p::MSG_FRAME_REGION,
            seq,
            &region,
            p::FLAG_ACK_REQ,
        );

        while self.pending.len() >= self.window {
            let waited = Instant::now();
            let (ok, seen) = self.reap(ACK_TIMEOUT)?;
            if ok {
                // Kil payi bekleyisler zaman asimindan cok daha sik.
                // En uzununu tutuyoruz ki olay nadir bir kayip mi yoksa
                // buyuyen bir gecikmenin ucu mu, gorulebilsin.
                let w = waited.elapsed();
                if w > self.diag.max_wait {
                    self.diag.max_wait = w;
                }
            } else {
                // Bu cerceveden umudu kes, yoksa tikanip kaliriz.
                self.stats.ack_timeouts += 1;
                let dropped = self.pending.remove(0);
                self.note_timeout(dropped, seen);
            }
        }

        let res = self.port.write_all(&frame);
        self.stats.frames_sent += 1;
        self.stats.bytes_sent += frame.len() as u64;
        self.pending.push(seq);

        self.frame_buf = frame;
        self.region_buf = region;
        self.rle_buf = rle;
        res?;
        Ok(())
    }

    /// Bekleyen butun onaylari toplar. Kapanistan once cagrilir.
    pub fn drain(&mut self, timeout: Duration) -> Result<bool, LinkError> {
        while !self.pending.is_empty() {
            let (ok, seen) = self.reap(timeout)?;
            if !ok {
                self.stats.ack_timeouts += self.pending.len() as u32;
                let left = std::mem::take(&mut self.pending);
                for seq in left {
                    self.note_timeout(seq, seen);
                }
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Canlilik isareti. Cihaz mesaj gelmezse bekleme ekranina dusuyor;
    /// dirty tracking yuzunden ekran durgunken hic cerceve gitmedigi
    /// icin bu gerekli. Ayrintisi `include/pins.h` icinde
    /// `LINK_IDLE_TIMEOUT_MS` uzerinde.
    pub fn ping(&mut self) -> Result<(), LinkError> {
        let seq = self.next_seq();
        let mut frame = std::mem::take(&mut self.frame_buf);
        p::build_frame_into(&mut frame, p::MSG_PING, seq, &[], 0);
        let res = self.port.write_all(&frame);
        self.stats.bytes_sent += frame.len() as u64;
        self.frame_buf = frame;
        res?;
        Ok(())
    }

    pub fn set_backlight(&mut self, level: u8) -> Result<(), LinkError> {
        let seq = self.next_seq();
        let mut frame = std::mem::take(&mut self.frame_buf);
        p::build_frame_into(&mut frame, p::MSG_SET_BACKLIGHT, seq, &[level], 0);
        let res = self.port.write_all(&frame);
        self.frame_buf = frame;
        res?;
        Ok(())
    }

    /// Cihaz sayaclarini ister ve STATUS cercevesini dondurur.
    pub fn status(&mut self, timeout: Duration) -> Result<Option<p::Frame>, LinkError> {
        let seq = self.next_seq();
        let mut frame = std::mem::take(&mut self.frame_buf);
        p::build_frame_into(&mut frame, p::MSG_GET_STATUS, seq, &[], 0);
        let res = self.port.write_all(&frame);
        self.frame_buf = frame;
        res?;

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let _ = self.pump()?;
            if let Some(i) = self.inbox.iter().position(|f| f.msg_type == p::MSG_STATUS) {
                return Ok(Some(self.inbox.remove(i)));
            }
        }
        Ok(None)
    }

    /// Cihaz tam kare istedi mi. Okuyan bayragi dusurur, yani her istek
    /// bir kez islenir.
    pub fn take_need_full(&mut self) -> bool {
        std::mem::take(&mut self.need_full)
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    /// Kimsenin almadigi cerceveleri atar.
    ///
    /// `inbox` sadece bekleyen bir cevap icin tutuluyor; okunmayan bir
    /// tur birikirse bellek sizar. Bu yuzden kutu bir siniri asarsa en
    /// eskiler dusuruluyor.
    fn trim_inbox(&mut self) {
        if self.inbox.len() > INBOX_LIMIT {
            let excess = self.inbox.len() - INBOX_LIMIT;
            self.inbox.drain(..excess);
        }
    }
}
