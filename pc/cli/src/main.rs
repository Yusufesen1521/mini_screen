//! mini_screen cekirdeginin ince komut satiri kabugu.
//!
//! Faz 2'de arayuz bu. Editor Faz 4'un isi ve ayri bir surec olacak,
//! cekirdek kutuphanesi GUI bilmiyor.

use std::time::{Duration, Instant};

use mini_screen_core::engine::{default_layout, Engine};
use mini_screen_core::render::Rect;
use mini_screen_core::transport::{find_ports, Link, OPEN_RETRY_TIMEOUT};
use mini_screen_core::widget;

const DRAIN_TIMEOUT: Duration = Duration::from_secs(3);
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
/// Tur arasi bekleme. Widget'lar kendi araliklarini ayrica sinirliyor.
const TICK_SLEEP: Duration = Duration::from_millis(20);
const STATS_PERIOD: Duration = Duration::from_secs(5);
/// Bosta canlilik isareti araligi. Cihazin zaman asimi 4 saniye,
/// bu deger ona gore rahat bir pay birakiyor.
const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(1500);

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let port = flag_value(&args, "--port");
    let seconds: u64 = flag_value(&args, "--seconds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    match cmd {
        "ports" => cmd_ports(),
        "hello" => cmd_hello(port.as_deref()),
        "status" => cmd_status(port.as_deref()),
        "widgets" => cmd_widgets(),
        "sensors" => cmd_sensors(args.iter().any(|a| a == "--dump")),
        "preview" => cmd_preview(args.get(2).map(|s| s.as_str()).unwrap_or("preview.png")),
        "run" => cmd_run(
            port.as_deref(),
            seconds,
            args.iter().any(|a| a == "--static"),
        ),
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn print_help() {
    println!("mini_screen PC araci\n");
    println!("  mscreen ports              cihaz portlarini listeler");
    println!("  mscreen hello              el sikisir, cihaz yeteneklerini basar");
    println!("  mscreen status             cihaz sayaclarini okur");
    println!("  mscreen widgets            kayitli widget turlerini listeler");
    println!("  mscreen sensors            sensor kaynaklarini yoklar ve okur");
    println!("  mscreen sensors --dump     Afterburner girdilerini ham listeler");
    println!("  mscreen run                cizim dongusunu baslatir");
    println!("  mscreen preview [dosya]    bir kare cizip PNG olarak yazar");
    println!("\n  --port <ad>                portu elle verir");
    println!("  --seconds <n>              run icin sure siniri, 0 = sinirsiz");
    println!("  --static                   olcum kipi: ekrani dondurup trafigi olcer");
}

fn cmd_ports() -> anyhow::Result<()> {
    let ports = find_ports();
    if ports.is_empty() {
        println!("Espressif portu bulunamadi.");
        return Ok(());
    }
    for p in ports {
        println!(
            "{}  VID:PID {:04X}:{:04X}  seri {}",
            p.name,
            p.vid,
            p.pid,
            p.serial.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

fn cmd_widgets() -> anyhow::Result<()> {
    for k in widget::kinds() {
        println!("{}", k);
    }
    Ok(())
}

fn cmd_sensors(dump: bool) -> anyhow::Result<()> {
    use mini_screen_core::sensors::{afterburner, Sensors, Snapshot};

    if dump {
        let entries = afterburner::AfterburnerSource::dump();
        if entries.is_empty() {
            match afterburner::AfterburnerSource::header() {
                Some(h) => {
                    println!("Esleme acildi ama girdi okunamadi. Ham baslik:");
                    let adlar = [
                        "imza",
                        "surum",
                        "baslik boyu",
                        "girdi sayisi",
                        "girdi boyu",
                        "zaman",
                        "gpu girdi sayisi",
                        "gpu girdi boyu",
                    ];
                    for (i, v) in h.iter().enumerate() {
                        println!("  {:<18} 0x{:08X}  ({})", adlar[i], v, v);
                    }
                    let sig = h[0].to_le_bytes();
                    println!("  imza ASCII         {:?}", String::from_utf8_lossy(&sig));
                }
                None => println!("Afterburner paylasimli bellegi acilamadi."),
            }
            for (name, code) in afterburner::AfterburnerSource::diagnose() {
                let aciklama = match code {
                    2 => "ad bulunamadi (Afterburner kapali ya da baska ad alaninda)",
                    5 => "erisim reddedildi (Afterburner yukseltilmis, biz degiliz)",
                    _ => "bilinmeyen",
                };
                println!("  {:<28} hata {} : {}", name, code, aciklama);
            }
        } else {
            println!("Afterburner girdileri ({} adet):", entries.len());
            for (n, v, max) in entries {
                println!("  {:<42} {:<14} ust sinir {}", n, v, max);
            }
        }
        return Ok(());
    }

    let mut s = Sensors::probe_all();
    println!("calisan kaynaklar : {:?}", s.active_names());
    println!("bulunamayanlar    : {:?}", s.missing_names());

    // Ag hizi iki ornek arasindaki farktan cikiyor, bir tur bekliyoruz.
    s.poll();
    std::thread::sleep(Duration::from_millis(1100));
    s.poll();

    let v = s.snapshot();
    println!();
    row("CPU", v.cpu_percent.map(|x| format!("{:.1}%", x)));
    row("CPU sicaklik", v.cpu_temp_c.map(|x| format!("{:.1} C", x)));
    row("cekirdek", v.cpu_cores.map(|x| x.to_string()));
    row("RAM", pair(v.mem_used, v.mem_total));
    row("RAM yuzde", v.mem_percent().map(|x| format!("{:.1}%", x)));
    row("Disk", pair(v.disk_used, v.disk_total));
    row("Ag rx", v.net_rx_bps.map(|x| format!("{} bayt/sn", x)));
    row("Ag tx", v.net_tx_bps.map(|x| format!("{} bayt/sn", x)));
    row("GPU", v.gpu_percent.map(|x| format!("{:.1}%", x)));
    row("GPU sicaklik", v.gpu_temp_c.map(|x| format!("{:.1} C", x)));
    row("VRAM", pair(v.gpu_mem_used, v.gpu_mem_total));
    let _ = Snapshot::default();
    Ok(())
}

/// Okunamayan degerler "yok" diye gosteriliyor, sifir ya da bos degil.
fn row(label: &str, value: Option<String>) {
    match value {
        Some(v) => println!("  {:<14}: {}", label, v),
        None => println!("  {:<14}: yok", label),
    }
}

fn pair(used: Option<u64>, total: Option<u64>) -> Option<String> {
    match (used, total) {
        (Some(u), Some(t)) => Some(format!(
            "{:.1} / {:.1} GiB",
            u as f64 / 1073741824.0,
            t as f64 / 1073741824.0
        )),
        _ => None,
    }
}

/// Cihaz olmadan bir kare cizer. Tasarimi gozle kontrol etmek icin.
fn cmd_preview(path: &str) -> anyhow::Result<()> {
    use mini_screen_core::{SCREEN_HEIGHT, SCREEN_WIDTH};

    let layout = default_layout(SCREEN_WIDTH, SCREEN_HEIGHT);
    let mut engine = Engine::new(SCREEN_WIDTH, SCREEN_HEIGHT, &layout)?;
    if !engine.has_font() {
        println!("UYARI: sistem fontu bulunamadi, metin cizilmeyecek.");
    }
    // Sensorlerin ilk degerlerini alabilmesi icin birkac tur.
    let mut dirty = Vec::new();
    for _ in 0..3 {
        engine.tick(&mut dirty);
        std::thread::sleep(Duration::from_millis(600));
    }
    engine.save_png(path).map_err(|e| anyhow::anyhow!(e))?;
    println!("yazildi: {}", path);
    Ok(())
}

fn cmd_hello(port: Option<&str>) -> anyhow::Result<()> {
    let mut link = Link::open_retry(port, OPEN_RETRY_TIMEOUT)?;
    let caps = link.handshake()?;
    for line in link.take_logs() {
        println!("  [cihaz] {}", line);
    }
    println!("Cihaz baglandi");
    println!("  protokol    : {}", caps.proto_version);
    println!("  firmware    : {}", caps.firmware());
    println!("  ekran       : {}x{}", caps.width, caps.height);
    println!("  codec maske : 0x{:02X}", caps.codecs);
    println!("  max payload : {} bayt", caps.max_payload);
    println!("  rx slot     : {}", caps.rx_slots);
    println!(
        "  kendini sinama: {}",
        if caps.selftest_ok { "TAMAM" } else { "HATA" }
    );
    println!("  mac         : {}", caps.mac_string());
    link.drain(DRAIN_TIMEOUT)?;
    Ok(())
}

fn cmd_status(port: Option<&str>) -> anyhow::Result<()> {
    let mut link = Link::open_retry(port, OPEN_RETRY_TIMEOUT)?;
    link.handshake()?;
    let _ = link.take_logs();
    match link.status(STATUS_TIMEOUT)? {
        Some(f) if f.payload.len() >= 14 => {
            let b = &f.payload;
            println!(
                "islenen   : {}",
                u32::from_le_bytes([b[0], b[1], b[2], b[3]])
            );
            println!(
                "dusen     : {}",
                u32::from_le_bytes([b[4], b[5], b[6], b[7]])
            );
            println!("hdr CRC   : {}", u16::from_le_bytes([b[8], b[9]]));
            println!("payloadCRC: {}", u16::from_le_bytes([b[10], b[11]]));
            println!("senkron   : {}", u16::from_le_bytes([b[12], b[13]]));
        }
        Some(_) => println!("STATUS geldi ama payload kisa."),
        None => println!("STATUS gelmedi."),
    }
    Ok(())
}

fn cmd_run(port: Option<&str>, seconds: u64, static_mode: bool) -> anyhow::Result<()> {
    let mut link = Link::open_retry(port, OPEN_RETRY_TIMEOUT)?;
    let caps = link.handshake()?;
    let _ = link.take_logs();
    println!(
        "Cihaz baglandi: {}x{}, rx slot {}",
        caps.width, caps.height, caps.rx_slots
    );

    let layout = default_layout(caps.width, caps.height);
    let mut engine = Engine::new(caps.width, caps.height, &layout)?;
    if !engine.has_font() {
        // Sessizce bos ekran gostermek hatayi gizler.
        println!("UYARI: sistem fontu bulunamadi, metin cizilemeyecek.");
    }
    println!("Widget'lar: {:?}", widget::kinds());
    println!(
        "Sensor kaynaklari: calisan {:?}, bulunamayan {:?}",
        engine.sensor_sources(),
        engine.missing_sources()
    );
    println!("Cikmak icin Ctrl+C.\n");

    let mut dirty: Vec<Rect> = Vec::new();
    let mut pixels: Vec<u16> = Vec::new();

    let start = Instant::now();
    let mut last_sent = Instant::now();
    let mut frozen = false;
    let mut window_start = start;
    let mut window_bytes = 0u64;
    let mut window_rects = 0u64;
    let mut window_ticks = 0u64;
    let mut idle_ticks = 0u64;
    // Zaman asimi olaylarindan kaci basildi. Olay nadir ve kosu uzun;
    // oldugu anda basilmazsa hangi pencereye denk geldigi kaybolur.
    let mut reported_timeouts = 0usize;

    loop {
        if seconds > 0 && start.elapsed() >= Duration::from_secs(seconds) {
            break;
        }

        // Gonderecek bir sey olmasa bile porttan okunmali, yoksa
        // cihazin TX tamponu tasiyor. Gerekcesi Link::poll uzerinde.
        link.poll()?;
        for line in link.take_logs() {
            println!("  [cihaz] {}", line);
        }

        // Olcum kipinde ilk kare gittikten sonra ekrani donduruyoruz.
        if static_mode && !frozen && start.elapsed() >= Duration::from_secs(2) {
            engine.set_frozen(true);
            frozen = true;
            link.drain(DRAIN_TIMEOUT)?;
            window_start = Instant::now();
            window_bytes = 0;
            window_rects = 0;
            window_ticks = 0;
            idle_ticks = 0;
            println!("--- ekran donduruldu, buradan sonrasi olcum ---");
        }

        engine.tick(&mut dirty);
        window_ticks += 1;
        if dirty.is_empty() {
            idle_ticks += 1;
        }

        let before = link.stats.bytes_sent;
        for r in &dirty {
            engine.copy_region(*r, &mut pixels);
            link.send_region(r.x, r.y, r.w, r.h, &pixels)?;
        }
        window_bytes += link.stats.bytes_sent - before;
        window_rects += dirty.len() as u64;

        // Bir sey gonderdiysek sayaci sifirla, yoksa zamani gelince
        // canlilik isareti at. Cihaz mesaj gelmezse bekleme ekranina
        // dusuyor ve durgun ekranda hic cerceve gitmiyor.
        if !dirty.is_empty() {
            last_sent = Instant::now();
        } else if last_sent.elapsed() >= KEEPALIVE_INTERVAL {
            let before_ping = link.stats.bytes_sent;
            link.ping()?;
            window_bytes += link.stats.bytes_sent - before_ping;
            last_sent = Instant::now();
        }

        // Zaman asimi olustuysa cevresini hemen bas. Sebebi bulunamamis
        // bir olay bu, kaydi olusma aninda alinmali.
        while reported_timeouts < link.diag.timeouts.len() {
            let e = link.diag.timeouts[reported_timeouts];
            println!(
                "  [ACK ZAMAN ASIMI] {:.1} sn  sira={}  cerceve={}  bayt={}  \
                 beklemede={} bayt  bekleyis boyunca={} bayt  kuyruk={}",
                e.at.as_secs_f64(),
                e.seq,
                e.frames_sent,
                e.bytes_sent,
                e.waiting_bytes,
                e.bytes_during,
                e.pending,
            );
            reported_timeouts += 1;
        }

        if window_start.elapsed() >= STATS_PERIOD {
            let secs = window_start.elapsed().as_secs_f64();
            let max_wait = link.take_max_wait();
            println!(
                "{:>5.0} sn  tur={} bos={} ({:.0}%)  dikdortgen={}  {:.0} bayt/sn  \
                 NACK={} ACKzaman={} enuzunACK={:.0} ms",
                start.elapsed().as_secs_f64(),
                window_ticks,
                idle_ticks,
                100.0 * idle_ticks as f64 / window_ticks as f64,
                window_rects,
                window_bytes as f64 / secs,
                link.stats.nacks,
                link.stats.ack_timeouts,
                max_wait.as_secs_f64() * 1000.0,
            );
            window_start = Instant::now();
            window_bytes = 0;
            window_rects = 0;
            window_ticks = 0;
            idle_ticks = 0;
        }

        std::thread::sleep(TICK_SLEEP);
    }

    link.drain(DRAIN_TIMEOUT)?;
    println!(
        "\nToplam: {} cerceve, {} bayt, NACK {}, ACK zaman asimi {}",
        link.stats.frames_sent, link.stats.bytes_sent, link.stats.nacks, link.stats.ack_timeouts
    );
    println!(
        "Gec gelen onay: {} (en buyuk gecikme {:.0} ms), beklenmeyen cerceve: {}",
        link.diag.late_acks,
        link.diag.max_late.as_secs_f64() * 1000.0,
        link.diag.unexpected,
    );

    // Olaylarin dokumu. Aralarindaki sure duzenliyse sistematik bir sey
    // var demektir; 106 dakikalik kosuda araliklar 25-29 dakikaydi.
    if !link.diag.timeouts.is_empty() {
        println!("\nACK zaman asimi dokumu:");
        println!("  an (sn)   arali(sn)  sira  cerceve     bayt  beklemede  bekleyiste");
        let mut prev: Option<Duration> = None;
        for e in &link.diag.timeouts {
            let gap = match prev {
                Some(p) => format!("{:9.1}", (e.at - p).as_secs_f64()),
                None => "        -".to_string(),
            };
            println!(
                "  {:7.1} {} {:5} {:8} {:8} {:10} {:11}",
                e.at.as_secs_f64(),
                gap,
                e.seq,
                e.frames_sent,
                e.bytes_sent,
                e.waiting_bytes,
                e.bytes_during,
            );
            prev = Some(e.at);
        }
        println!(
            "\n  Okuma: 'beklemede' sifirdan buyukse veri surucude duruyordu, kabahat\n  \
             PC tarafinda. 'bekleyiste' sifirsa hat iki saniye tamamen sustu."
        );
    }
    Ok(())
}
