//! Widget'lar.
//!
//! Yeni widget eklemek icin bu klasore bir dosya acmak yeterli. Modul
//! bildirimini `build.rs` uretiyor, kayit isini `register_widget!`
//! yapiyor. Burada elle tutulan bir liste yok, gerekcesi
//! `crate::widget` modulunun basinda.

include!(concat!(env!("OUT_DIR"), "/widget_mods.rs"));
