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
mscreen widgets   # kayitli widget turlerini listeler
mscreen sensors   # sensor kaynaklarini yoklar ve okur
```

`--port <ad>` ile port elle verilebilir, verilmezse Espressif VID'i
(0x303A) ile aranir.

### Yerlesim secimi

`run` ve `preview` komutlari `--layout` aliyor:

| Ad | Ne cizer |
|---|---|
| `hwmon` | **Varsayilan.** Donanim izleme paneli: IP seridi, CPU ve GPU sicaklik halkalari, alt seritte doluluk cubuklari. |
| `gauges` | Onceki varsayilan: ustte saat, altinda halka gostergeler. |
| `clock` | Sadece saat ve calisma suresi. Sensorsuz makine icin. |

```bash
mscreen run                              # hwmon
mscreen preview out.png --layout gauges  # cihazsiz, PNG olarak
```

Bilinmeyen bir ad sessizce varsayilana dusmez, hata verir: yanlis yazilan
bayrak fark edilmeden calismaya devam ederdi.

`hwmon` tasarimi lopaka.app uzerinde 480x320 icin cizildi ve 320x240'a
yeniden yerlestirildi. Ayrintisi ve tasarimdan sapmalar
`core/src/widgets/hwmon.rs` dosyasinin basinda.

## Bilinen davranis

Cihaz USB CDC portu kapatilinca yeniden numaralandiriliyor. Arka arkaya
iki komut calistirilirsa ikincisi bu pencereye denk gelip "var olmayan
aygit" hatasi aliyordu. `Link::open_retry` bu yuzden var. Kopma
sirasinda toparlanmanin tamami 2.3'un isi ve henuz bitmedi.
