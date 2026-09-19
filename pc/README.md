# mini_screen PC uygulamasi

Faz 2. Dil secimi ve gerekcesi `plans.md` icindeki 2.1 bolumunde,
olcumler `docs/measurements.md` icindeki "Faz 2 hazirligi" bolumunde.

## Yapi

| Crate | Ne yapar |
|---|---|
| `core` | Kutuphane. Protokol, tasima, rasterleme, dirty tracking. GUI bilmez. |
| `cli` | Ince kabuk. Calistirilabilir adi `mscreen`. |

Mimari karar: cekirdek disaridan konusulan bir servis, her zaman calisan
taraf ile ara sira acilan editor ayri. Faz 2'nin hicbir cikis kriteri GUI
istemiyor, bu yuzden `core` icinde hicbir arayuz kutuphanesi yok.

## Derleme

```bash
cd pc
cargo build --release
cargo test
```

**Bu makinede `build.bat` gerekiyor.** Sistemde iki Visual Studio kurulu
ve rustup'in sectigi VS 18 kurulumunda `msvcrt.lib` eksik, bu yuzden
baglama `LNK1104` ile dusuyor. `build.bat` once VS 2022 ortamini yukluyor:

```bash
build.bat build --release
build.bat test
build.bat clippy --all-targets
```

VS 18 kurulumu tamamlanirsa bu betige gerek kalmaz.

## Kullanim

```bash
mscreen ports     # cihaz portlarini listeler
mscreen hello     # el sikisir, cihaz yeteneklerini basar
mscreen status    # cihaz sayaclarini okur
```

`--port <ad>` ile port elle verilebilir, verilmezse Espressif VID'i
(0x303A) ile aranir.

## Bilinen davranis

Cihaz USB CDC portu kapatilinca yeniden numaralandiriliyor. Arka arkaya
iki komut calistirilirsa ikincisi bu pencereye denk gelip "var olmayan
aygit" hatasi aliyordu. `Link::open_retry` bu yuzden var. Kopma
sirasinda toparlanmanin tamami 2.3'un isi ve henuz bitmedi.
