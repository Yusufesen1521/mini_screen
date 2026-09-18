# Olcumler

Her fazda alinan gercek sayilar. Tahmin ve izlenim buraya yazilmaz.

Olcum araci: `src/bench_main.cpp`. Tekrar calistirmak icin:

```bash
pio run -e bench   -t upload    # 40 MHz
pio run -e bench80 -t upload    # 80 MHz
```

---

## Faz 1.1: Ekran tarafinin tavani

Tarih: 2026-09-19
Donanim: ESP32-S3 DevKitC-1 N16R8, ILI9341 3.2", breadboard ve jumper kablo
Yazilim: TFT_eSPI 2.5.43, Arduino core 2.0.17, `USE_FSPI_PORT` (SPI2)
Yontem: her olcum 3 isinma turu sonrasi 20 tekrarin ortalamasi

### 40 MHz

| Bolge | Yol | Bayt | us | MB/s |
|---|---|---|---|---|
| 320x240 tam kare | fillScreen | 153600 | 32037 | 4.79 |
| 320x240 tam kare | pushImage, SRAM | 153600 | 34898 | 4.40 |
| 320x48 sayac | pushImage, SRAM | 30720 | 6992 | 4.39 |
| 160x120 | pushImage, SRAM | 38400 | 8738 | 4.39 |
| 80x80 | pushImage, SRAM | 12800 | 2924 | 4.38 |
| 320x240 tam kare | pushImage, PSRAM | 153600 | 35677 | 4.31 |
| 320x240 tam kare | pushImage, swap acik | 153600 | 34978 | 4.39 |

Teorik tavan 5.00 MB/s. fillScreen bunun yuzde 96'sina, pushImage yuzde
88'ine ulasiyor.

### 80 MHz

| Bolge | Yol | Bayt | us | MB/s |
|---|---|---|---|---|
| 320x240 tam kare | fillScreen | 153600 | 16797 | 9.14 |
| 320x240 tam kare | pushImage, SRAM | 153600 | 19529 | 7.87 |
| 320x48 sayac | pushImage, SRAM | 30720 | 3919 | 7.84 |
| 160x120 | pushImage, SRAM | 38400 | 4894 | 7.85 |
| 80x80 | pushImage, SRAM | 12800 | 1642 | 7.80 |
| 320x240 tam kare | pushImage, PSRAM | 153600 | 20355 | 7.55 |
| 320x240 tam kare | pushImage, swap acik | 153600 | 19688 | 7.80 |

Teorik tavan 10.00 MB/s, ulasilan yuzde 79. Zamanlama acisindan calisiyor.
**Gorsel kararlilik dogrulanmadi**, cunku bu hiz benimsenmedi. Benimsenirse
once uzun sureli gorsel bozulma testi yapilmali.

### Bulgular

**1. Ekran degil USB darbogaz.**
40 MHz SPI 4.40 MB/s, USB Full Speed pratikte yaklasik 1.00 MB/s. Ekran
baglantisi USB'den 4.4 kat hizli. Faz 1'in kalan isi ekrani hizlandirmak
degil, USB tarafini ve sikistirmayi dogru kurmak.

**2. Bolge boyutunun maliyeti yok.**
80x80 ile tam kare arasinda MB/s farki yuzde 0.5. Cagri basina sabit yuk
olcumun altinda kaliyor. Protokol kucuk bolgeleri cekinmeden kullanabilir,
bolme cezasi yok.

**3. PSRAM neredeyse SRAM kadar hizli.**
Tam karede fark yuzde 2, kucuk bolgelerde olculemiyor. Alim tamponlari
8 MB PSRAM icinde durabilir, dahili SRAM'i zorlamaya gerek yok.

**4. Bayt sirasi cevirmek bedava.**
`setSwapBytes` acik ve kapali arasinda fark yuzde 0.2. Protokolde hangi
sira daha kolaysa o secilebilir; PC tarafinda onceden cevirmenin olculebilir
bir faydasi yok.

**5. Sikistirma hedefi 4.4 kat.**
USB 1.00 MB/s, SPI 4.40 MB/s. Sikistirma orani 4.4 katin altindayken USB
darbogaz, ustune ciktiginda SPI darbogaz olur. Yani RLE ile 4.4 kat hedefi
tutturmak ikisini dengeler; daha fazla sikistirmanin getirisi yok.
Bu oranda tam kare hizi yaklasik 28 FPS olur.

**6. DMA kullanilamiyor, ama gerekmiyor.**
TFT_eSPI 2.5.43'un DMA yolu ESP32-S3'te `dma_end_callback` icinde cokuyor.
Kok neden `src/bench_main.cpp` icinde ayrintili yazili: kutuphane register
indeksi ile ESP-IDF host enum'unu karistiriyor, `SPI_DMA_CONF_REG(1)` 0x30
adresini uretiyor ve DMA tamamlanma kesmesi oraya yaziyor.

Bu bir kayip degil: bloklayan push ile bile CPU zamanin yuzde 77'sinde bos
(USB tam kareyi 154 ms'de getiriyor, ekrana basmak 35 ms suruyor). Ustelik
cift cekirdek var, ekran itme bir cekirdekte, USB alimi digerinde
calisabilir. DMA tavani yukseltmezdi, sadece CPU'yu serbest birakirdi.

### Kararlar

- **SPI hizi 40 MHz kalacak.** 80 MHz bant genisligi getirisi olsa da
  darbogaz orada degil. Breadboard ve jumper kablolarda 80 MHz sinyal
  butunlugu riski var, marj birakmak daha degerli. PCB asamasinda yeniden
  degerlendirilecek.
- **DMA kullanilmayacak.** `BENCH_ENABLE_DMA` bayragi 0. Kutuphane duzelirse
  ya da kendi SPI yolumuz yazilirsa yeniden olculecek.
- **Alim tamponlari PSRAM'de olacak.** Fark olcum hatasi seviyesinde,
  dahili SRAM daha degerli bir kaynak.
- **RLE hedefi 4.4 kat.** Bunun altinda USB, ustunde SPI sinirlar.
