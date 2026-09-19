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
        "run" => cmd_run(port.as_deref(), seconds),
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
    println!("  mscreen run                cizim dongusunu baslatir");
    println!("\n  --port <ad>                portu elle verir");
    println!("  --seconds <n>              run icin sure siniri, 0 = sinirsiz");
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
            println!("islenen   : {}", u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            println!("dusen     : {}", u32::from_le_bytes([b[4], b[5], b[6], b[7]]));
            println!("hdr CRC   : {}", u16::from_le_bytes([b[8], b[9]]));
            println!("payloadCRC: {}", u16::from_le_bytes([b[10], b[11]]));
            println!("senkron   : {}", u16::from_le_bytes([b[12], b[13]]));
        }
        Some(_) => println!("STATUS geldi ama payload kisa."),
        None => println!("STATUS gelmedi."),
    }
    Ok(())
}

fn cmd_run(port: Option<&str>, seconds: u64) -> anyhow::Result<()> {
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
    println!("Cikmak icin Ctrl+C.\n");

    let mut dirty: Vec<Rect> = Vec::new();
    let mut pixels: Vec<u16> = Vec::new();

    let start = Instant::now();
    let mut window_start = start;
    let mut window_bytes = 0u64;
    let mut window_rects = 0u64;
    let mut window_ticks = 0u64;
    let mut idle_ticks = 0u64;

    loop {
        if seconds > 0 && start.elapsed() >= Duration::from_secs(seconds) {
            break;
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

        if window_start.elapsed() >= STATS_PERIOD {
            let secs = window_start.elapsed().as_secs_f64();
            println!(
                "{:>5.0} sn  tur={} bos={} ({:.0}%)  dikdortgen={}  {:.0} bayt/sn  NACK={} ACKzaman={}",
                start.elapsed().as_secs_f64(),
                window_ticks,
                idle_ticks,
                100.0 * idle_ticks as f64 / window_ticks as f64,
                window_rects,
                window_bytes as f64 / secs,
                link.stats.nacks,
                link.stats.ack_timeouts,
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
        link.stats.frames_sent,
        link.stats.bytes_sent,
        link.stats.nacks,
        link.stats.ack_timeouts
    );
    Ok(())
}
