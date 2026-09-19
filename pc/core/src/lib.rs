//! mini_screen PC cekirdegi.
//!
//! Bu bir kutuphane, uygulama degil. Faz 2 mimarisi cekirdegi disaridan
//! konusulan bir servis olarak tasarliyor: her zaman calisan taraf
//! (cihazi tutar, cizer, gonderir) ile ara sira acilan editor birbirinden
//! ayri. Gerekcesi `plans.md` icindeki Faz 2 bolumunde.
//!
//! Faz 2'nin cikis kriterlerinin hicbiri GUI istemiyor; bu crate icinde
//! hicbir arayuz kutuphanesi yok ve olmayacak.

pub mod protocol;
pub mod transport;

/// Ekranin gorunur olcusu, yatay kullanim. Cihaz bunu CAPS ile de
/// bildiriyor; bu sabitler sadece varsayilan ve dogrulama icin.
pub const SCREEN_WIDTH: u16 = 320;
pub const SCREEN_HEIGHT: u16 = 240;

/// RGBA8888'den RGB565'e cevirir.
///
/// Panel big-endian bekliyor ama cevrimi cihaz yapiyor
/// (`setSwapBytes(true)`), olcum maliyetinin yuzde 0.2 oldugunu gosterdi.
/// PC tarafi little-endian uretir.
#[inline]
pub fn rgba8888_to_rgb565(p: u32) -> u16 {
    let r = p & 0xFF;
    let g = (p >> 8) & 0xFF;
    let b = (p >> 16) & 0xFF;
    (((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)) as u16
}

pub fn convert_rgba_to_rgb565(src: &[u32], dst: &mut [u16]) {
    debug_assert_eq!(src.len(), dst.len());
    for (d, &s) in dst.iter_mut().zip(src.iter()) {
        *d = rgba8888_to_rgb565(s);
    }
}
