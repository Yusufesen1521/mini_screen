//! mini_screen cekirdeginin ince komut satiri kabugu.
//!
//! Faz 2'de arayuz bu. Editor Faz 4'un isi ve ayri bir surec olacak,
//! cekirdek kutuphanesi GUI bilmiyor.

use std::time::Duration;

use mini_screen_core::transport::{find_ports, Link, OPEN_RETRY_TIMEOUT};

const DRAIN_TIMEOUT: Duration = Duration::from_secs(3);
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    // --port ile elle port verilebilir, verilmezse VID ile bulunur.
    let port = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .cloned();

    match cmd {
        "ports" => cmd_ports(),
        "hello" => cmd_hello(port.as_deref()),
        "status" => cmd_status(port.as_deref()),
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!("mini_screen PC araci\n");
    println!("  mscreen ports              cihaz portlarini listeler");
    println!("  mscreen hello              el sikisir, cihaz yeteneklerini basar");
    println!("  mscreen status             cihaz sayaclarini okur");
    println!("\n  --port <ad>                portu elle verir");
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
            let ok = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            let dropped = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
            let hdr_err = u16::from_le_bytes([b[8], b[9]]);
            let pl_err = u16::from_le_bytes([b[10], b[11]]);
            let sync = u16::from_le_bytes([b[12], b[13]]);
            println!("islenen   : {}", ok);
            println!("dusen     : {}", dropped);
            println!("hdr CRC   : {}", hdr_err);
            println!("payloadCRC: {}", pl_err);
            println!("senkron   : {}", sync);
        }
        Some(_) => println!("STATUS geldi ama payload kisa."),
        None => println!("STATUS gelmedi."),
    }
    Ok(())
}
