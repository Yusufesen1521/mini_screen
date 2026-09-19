//! Widget soyutlamasi ve kaydi.
//!
//! Faz 2 cikis kriteri: "ikinci bir sahte widget eklemek cekirdekte tek
//! satir degisiklik gerektirmiyor". Elle tutulan bir liste bu kriteri
//! gecemez, cunku her yeni widget o listeye bir satir ekletir. Bu yuzden
//! widget kendini kaydediyor: `inventory` baglayici seviyesinde topluyor,
//! cekirdek hicbir widget'in adini bilmiyor.
//!
//! Yeni widget eklemek: `widgets/` altina bir dosya, icinde `Widget`
//! uygulamasi ve bir `register_widget!` cagrisi. Baska hicbir yere
//! dokunulmaz.

use std::time::Duration;

use crate::render::{Canvas, Rect};

/// Widget'a her tik'te verilen disaridan gelen baglam.
///
/// Sensor degerleri buraya eklenecek (2.4). Simdilik sadece zaman var;
/// widget'lar dogrudan saate bakmasin diye burada tutuluyor, boylece
/// onizleme ve test sabit bir zamanla surulebiliyor.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// Uygulamanin basindan beri gecen sure.
    pub uptime: Duration,
    /// Yerel duvar saati, saniye hassasiyetinde bilesenler.
    pub local_hms: (u8, u8, u8),
    pub local_ymd: (i32, u8, u8),
}

pub trait Widget: Send {
    /// Layout dosyasinda ve kayitta kullanilan tur adi.
    fn kind(&self) -> &'static str;

    /// Ne siklikta tazelenmek istiyor. Motor bundan daha sik cagirmaz.
    fn interval(&self) -> Duration;

    /// Veriyi tazeler. Gorunumu degistiyse `true` doner.
    ///
    /// `false` donmek motorun yeniden cizmesini engeller; dirty tracking
    /// zaten piksel seviyesinde koruyor ama burada erken cikmak
    /// rasterleme maliyetini de kaldiriyor.
    fn update(&mut self, ctx: &Context) -> bool;

    /// Kendini `area` icine cizer. Disina tasmamali.
    fn render(&mut self, canvas: &mut Canvas, area: Rect, ctx: &Context);
}

/// Kayit girisi. `register_widget!` bunu uretir.
pub struct Registration {
    pub kind: &'static str,
    pub make: fn() -> Box<dyn Widget>,
}

inventory::collect!(Registration);

/// Widget'i kaydeder. Widget dosyasinin sonunda cagrilir.
#[macro_export]
macro_rules! register_widget {
    ($kind:expr, $ty:ty) => {
        inventory::submit! {
            $crate::widget::Registration {
                kind: $kind,
                make: || Box::new(<$ty>::default()),
            }
        }
    };
}

/// Kayitli butun widget turlerinin adlari.
pub fn kinds() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = inventory::iter::<Registration>
        .into_iter()
        .map(|r| r.kind)
        .collect();
    v.sort_unstable();
    v
}

/// Ada gore widget uretir. Bilinmeyen ad icin `None`.
pub fn make(kind: &str) -> Option<Box<dyn Widget>> {
    inventory::iter::<Registration>
        .into_iter()
        .find(|r| r.kind == kind)
        .map(|r| (r.make)())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kayitli_widgetlar_uretilebiliyor() {
        let list = kinds();
        assert!(!list.is_empty(), "hic widget kayitli degil");
        for k in list {
            let w = make(k).unwrap_or_else(|| panic!("{} uretilemedi", k));
            assert_eq!(w.kind(), k);
        }
    }

    #[test]
    fn bilinmeyen_tur_none_doner() {
        assert!(make("boyle-bir-widget-yok").is_none());
    }
}
