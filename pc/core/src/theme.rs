//! Ortak renk paleti ve olculer.
//!
//! Widget'lar kendi renklerini uydurmasin diye tek yerde. Faz 4'te tema
//! secimi gelecek; o zaman bu sabitler bir yapiya donusur, simdilik
//! sabit kalmasi yeterli.
//!
//! Panel TN ve gorus acisi dar, bu yuzden koyu zemin ve yuksek kontrast
//! secildi. Ayrica olculen sinir: `rgb_test2.gif` klibinde parlaklik 180
//! ustunde titreme var, o yuzden cok parlak genis alanlardan kaciniliyor.

use crate::render::Color;

// Zemin
pub const BG: Color = Color::rgb(14, 17, 22);
pub const BG_HEADER: Color = Color::rgb(20, 25, 33);
/// Satirlar arasi ince ayirici. Cizgi yerine zemin tonu farki
/// kullaniliyor, daha sakin duruyor.
pub const DIVIDER: Color = Color::rgb(32, 38, 48);

// Metin
pub const TEXT: Color = Color::rgb(232, 237, 245);
pub const TEXT_DIM: Color = Color::rgb(124, 135, 152);
pub const TEXT_FAINT: Color = Color::rgb(86, 95, 110);

// Vurgu ve olcum renkleri. Her metrigin kendi rengi var, kullanici
// sayiyi okumadan hangi satira baktigini anlasin.
pub const ACCENT: Color = Color::rgb(79, 163, 227);
pub const CPU: Color = Color::rgb(79, 163, 227);
pub const RAM: Color = Color::rgb(169, 139, 232);
pub const GPU: Color = Color::rgb(80, 210, 180);
pub const DISK: Color = Color::rgb(70, 192, 160);
pub const NET: Color = Color::rgb(232, 180, 79);
/// Esik ustunde cubuk bu renge doner.
pub const HOT: Color = Color::rgb(226, 100, 75);
pub const HOT_THRESHOLD: f32 = 85.0;

/// Cubuk oyugu. Dolu kismin altinda kalan kisim.
pub const TRACK: Color = Color::rgb(34, 40, 50);

// Olculer, 320x240 icin
pub const SCREEN_PAD: u16 = 12;
/// Baslik seridi yuksekligi. Saat buranin icinde.
pub const HEADER_H: u16 = 42;
/// Bir olcum satirinin yuksekligi.
pub const ROW_H: u16 = 43;

// Yazi boyutlari
pub const SIZE_CLOCK: f32 = 30.0;
pub const SIZE_DATE: f32 = 13.0;
pub const SIZE_WEEKDAY: f32 = 11.0;
pub const SIZE_LABEL: f32 = 11.0;
pub const SIZE_VALUE: f32 = 19.0;
pub const SIZE_DETAIL: f32 = 11.0;

/// Cubuk yuksekligi ve sol kenari. Deger sutunu bittigi yerde basliyor.
pub const BAR_H: u16 = 7;
pub const BAR_X: u16 = 150;

// ---------------------------------------------------------------------------
// Donanim izleme paneli (`hwmon`).
//
// Bu blok lopaka.app uzerinde cizilen tasarimdan geliyor. Tasarim 480x320
// idi, bizim panel 320x240; olculer yeniden yerlesimle uyarlandi, renkler
// oldugu gibi alindi. Ayrintisi `widgets/hwmon.rs` basinda.
// ---------------------------------------------------------------------------

/// Tasarimin krom rengi: ayiricilar, ust serit ve etiket rozetleri.
pub const HW_ACCENT: Color = Color::rgb(247, 65, 49);
/// Gosterge halkasinin bos kismi. Tasarimdaki gri.
pub const HW_TRACK: Color = Color::rgb(115, 117, 115);
/// CPU sicaklik halkasi. Tasarimdaki mavi.
pub const HW_CPU_ARC: Color = Color::rgb(33, 150, 247);
/// GPU sicaklik halkasi.
///
/// **Tasarimdan sapiyor.** Tasarim burada krom rengiyle ayni kirmiziyi
/// kullaniyordu; kirmizi ayni zamanda "sicak" isareti oldugu icin normal
/// sicaklikta yaniltici duruyor. Tema paletindeki GPU rengi kullaniliyor.
/// Tasarimin orijinaline donmek icin bu sabiti `HW_ACCENT` yap.
pub const HW_GPU_ARC: Color = Color::rgb(80, 210, 180);

/// Sicaklik halkasinin kapsadigi aralik. Bu araligin disi kirpilir.
///
/// Alt sinir oda sicakligi degil bosta calisan bir parcanin tipik
/// sicakligi: 20 C'den baslatmak halkayi surekli dolu gosteriyordu.
pub const TEMP_MIN_C: f32 = 30.0;
pub const TEMP_MAX_C: f32 = 100.0;
/// Bu sicakligin ustunde halka `HOT` rengine doner.
pub const TEMP_HOT_C: f32 = 80.0;
