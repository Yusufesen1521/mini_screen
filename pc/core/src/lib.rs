//! mini_screen PC cekirdegi.
//!
//! Bu bir kutuphane, uygulama degil. Faz 2 mimarisi cekirdegi disaridan
//! konusulan bir servis olarak tasarliyor: her zaman calisan taraf
//! (cihazi tutar, cizer, gonderir) ile ara sira acilan editor birbirinden
//! ayri. Gerekcesi `plans.md` icindeki Faz 2 bolumunde.
//!
//! Faz 2'nin cikis kriterlerinin hicbiri GUI istemiyor; bu crate icinde
//! hicbir arayuz kutuphanesi yok ve olmayacak.

pub mod dirty;
pub mod engine;
pub mod protocol;
pub mod render;
pub mod sensors;
pub mod transport;
pub mod widget;
pub mod widgets;

/// Ekranin gorunur olcusu, yatay kullanim. Cihaz bunu CAPS ile de
/// bildiriyor; bu sabitler sadece varsayilan ve dogrulama icin.
pub const SCREEN_WIDTH: u16 = 320;
pub const SCREEN_HEIGHT: u16 = 240;
