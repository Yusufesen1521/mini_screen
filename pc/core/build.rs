//! `src/widgets/` altindaki dosyalari tarar ve modul bildirimlerini uretir.
//!
//! Gerekcesi Faz 2 cikis kriteri: "ikinci bir sahte widget eklemek
//! cekirdekte tek satir degisiklik gerektirmiyor". Elle tutulan bir
//! `pub mod` listesi o tek satiri gerektirirdi. Bu betikle yeni widget
//! eklemek gercekten sadece yeni bir dosya acmak demek.

use std::path::Path;

fn main() {
    let dir = Path::new("src/widgets");
    println!("cargo:rerun-if-changed=src/widgets");

    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem == "mod" {
                continue;
            }
            println!("cargo:rerun-if-changed=src/widgets/{}.rs", stem);
            names.push(stem.to_string());
        }
    }
    names.sort();

    // Mutlak yol veriliyor cunku uretilen dosya OUT_DIR icinden
    // include! ediliyor ve gorece yol orada cozulmez.
    let root = std::env::current_dir().expect("calisma dizini okunamadi");
    let mut out = String::new();
    for n in &names {
        let p = root.join("src/widgets").join(format!("{}.rs", n));
        out.push_str(&format!(
            "#[path = {:?}]\npub mod {};\n",
            p.to_string_lossy().replace('\\', "/"),
            n
        ));
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR yok");
    std::fs::write(Path::new(&out_dir).join("widget_mods.rs"), out)
        .expect("modul listesi yazilamadi");
}
