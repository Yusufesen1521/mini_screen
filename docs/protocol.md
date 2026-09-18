# mini_screen USB protokolu

Surum 1 (taslak, henuz uygulanmadi)

PC ile cihaz arasindaki USB CDC baglantisinin bayt duzeyinde tanimi.

## Tasarimi yonlendiren olcumler

Bu protokol `docs/measurements.md` icindeki Faz 1.1 olcumlerine gore
sekillendi. Ozellikle:

- **USB darbogaz, ekran degil.** SPI 4.40 MB/s, USB Full Speed yaklasik
  1.00 MB/s. Protokolun isi bayt tasarruf etmek, ekrani hizlandirmak degil.
- **Sikistirma hedefi 4.4 kat.** Bunun altinda USB, ustunde SPI sinirlar.
- **Kucuk bolgenin cezasi yok.** 80x80 ile tam kare ayni MB/s veriyor, yani
  protokol serbestce bolebilir.
- **Bayt sirasi cevirmek bedava.** PC'nin pikselleri onceden cevirmesinin
  olculebilir faydasi yok, o yuzden en kolay sira secildi.

## Genel kurallar

- Tasima: USB CDC, ESP32-S3 yerlesik USB portu (GPIO 19/20), Full Speed.
- Butun cok baytli sayilar **little-endian**. Hem x86 hem ESP32 boyle
  calisiyor, iki tarafta da donusum yok.
- Baglanti guvenilir kabul edilir. USB kendi katmaninda CRC ve yeniden
  gonderim yapiyor. Buradaki CRC veri duzeltmek icin degil, **cerceve
  senkronunu dogrulamak** icin.
- Cihaz hicbir zaman kilitlenmez. Bozuk girdi karsisinda NACK gonderip
  senkron aramaya doner.

## Cerceve yapisi

Her mesaj ayni cercevenin icinde tasinir.

| Offset | Boyut | Alan | Aciklama |
|---|---|---|---|
| 0 | 2 | SOF | `A5 5A`, cerceve baslangic imzasi |
| 2 | 1 | VER | Protokol surumu, su an `01` |
| 3 | 1 | TYPE | Mesaj tipi |
| 4 | 1 | FLAGS | bit0 = ACK iste, digerleri rezerve, 0 gonderilir |
| 5 | 1 | SEQ | Sira numarasi, 0-255 arasi dolaniyor |
| 6 | 2 | LEN | Payload uzunlugu, little-endian |
| 8 | 1 | RSV | Rezerve, `00`. Payload'i cift adrese hizalar. |
| 9 | 1 | HDRCRC | Offset 2-8 arasi 7 bayt uzerinde CRC-8 |
| 10 | LEN | PAYLOAD | Mesaja ozel icerik |
| 10+LEN | 2 | CRC16 | Payload uzerinde CRC-16, little-endian |

Sabit yuk 12 bayt. 320x48 bir bolgede bu yuzde 0.04, 8x8 bir bolgede yuzde 9.

**Neden ayri bir baslik CRC'si var:** LEN alanina guvenip payload okumaya
baslamadan once basligin saglam oldugunu bilmek gerekiyor. Bozuk bir LEN
degeri cihazi hic gelmeyecek 65 bin bayti beklerken kilitleyebilirdi.
HDRCRC bunu engelliyor.

**Neden RSV var:** payload offset 10'da, yani cift adreste basliyor. RGB565
pikselleri 16 bitlik olduguna gore, alim tamponundan dogrudan `pushImage`
cagirabilmek icin bu hizalama gerekli. Aksi halde her bolge icin fazladan
bir kopyalama gerekirdi.

**LEN neden 16 bit:** en buyuk payload 65535 bayt. Tam kare 153600 bayt
ettigi icin sikismamis tam kare uc parcaya bolunur. Olcum 2 bolmenin
bedava oldugunu gosterdigi icin bu bir kayip degil, karsiliginda baslik
kucuk kaliyor.

## CRC tanimlari

**CRC-8:** polinom `0x07`, baslangic `0x00`, yansitma yok, cikis XOR yok.
`123456789` icin `0xF4`.

**CRC-16:** CRC-16/CCITT-FALSE. Polinom `0x1021`, baslangic `0xFFFF`,
yansitma yok, cikis XOR yok. `123456789` icin `0x29B1`.

Payload bos oldugunda CRC-16 baslangic degeri olan `0xFFFF` gonderilir.

CRC-32 yerine CRC-16 secildi: USB zaten kendi katmaninda koruma yapiyor,
buradaki kontrol cerceve hizalamasini dogrulamak icin. CRC-16 tum 16 bite
kadar patlama hatalarini yakaliyor, rastgele veride hata kacirma olasiligi
65536'da bir. Iki bayt tasarrufu ve daha az hesap karsiliginda yeterli.

## Mesaj tipleri

### PC'den cihaza

| TYPE | Ad | Payload |
|---|---|---|
| `0x01` | HELLO | PC kendini tanitir, CAPS ister |
| `0x10` | FRAME_REGION | Bir dikdortgen piksel bolgesi |
| `0x20` | SET_BACKLIGHT | 1 bayt, parlaklik 0-255 |
| `0x30` | PING | Bos |
| `0x40` | GET_STATUS | Bos |

`0x50` - `0x5F` araligi Faz 5'teki dosya yukleme icin rezerve.

### Cihazdan PC'ye

| TYPE | Ad | Payload |
|---|---|---|
| `0x81` | CAPS | Cihaz yetenekleri |
| `0x82` | ACK | 1 bayt, onaylanan SEQ |
| `0x83` | NACK | 2 bayt: sebep kodu, ilgili SEQ |
| `0x84` | ERROR | 2 bayt: sebep kodu, ek bilgi |
| `0x85` | STATUS | Sayaclar |
| `0x86` | PONG | PING'deki SEQ yankilanir |
| `0x87` | LOG | UTF-8 log metni, satir sonlari dahil |

`0x90` girdi olaylari icin rezerve (rotary encoder ve butonlar, Faz 3 ve
sonrasi).

**LOG neden protokolde:** cihazin log hatti UART0 uzerinde. Kartin UART
kopru portu takili degilse hicbir tanilama gorunmez. LOG mesaji sayesinde
tek kabloyla, sadece yerlesik USB portu takiliyken de log okunabiliyor.
Baglanti kurulmadan once uretilen acilis satirlari cihazda birikir ve
HELLO'ya verilen CAPS cevabinin hemen ardindan topluca gonderilir.

## HELLO ve CAPS

Baglantinin ilk isi el sikismak. PC once HELLO gonderir, cihaz CAPS ile
cevap verir. PC, CAPS almadan FRAME_REGION gondermemeli.

**HELLO payload (4 bayt)**

| Offset | Boyut | Alan |
|---|---|---|
| 0 | 1 | PC'nin destekledigi en yuksek protokol surumu |
| 1 | 3 | PC uygulamasi surumu: major, minor, patch |

**CAPS payload (20 bayt)**

| Offset | Boyut | Alan | Aciklama |
|---|---|---|---|
| 0 | 1 | proto_version | Cihazin konustugu surum |
| 1 | 3 | fw_version | major, minor, patch |
| 4 | 2 | screen_w | Donus uygulandiktan sonraki genislik |
| 6 | 2 | screen_h | Donus uygulandiktan sonraki yukseklik |
| 8 | 1 | pixel_format | `0x01` = RGB565 little-endian |
| 9 | 1 | codecs | Bit maskesi: bit0 NONE, bit1 RLE16 |
| 10 | 2 | max_payload | Kabul edilen en buyuk LEN |
| 12 | 1 | rx_slots | Ayni anda tamponlanabilen cerceve sayisi |
| 13 | 1 | selftest | Acilistaki kendini sinama sonucu, 1 = gecti |
| 14 | 6 | mac | Cihazin MAC adresi, kimlik olarak |

PC, cihazin bildirdigi `screen_w` ve `screen_h` disina cikmamali.
`max_payload` degerinden buyuk cerceve gondermemeli.

## FRAME_REGION

En sik kullanilan mesaj. Payload iki parcadan olusur: bolge basligi ve
piksel verisi.

**Bolge basligi (10 bayt)**

| Offset | Boyut | Alan |
|---|---|---|
| 0 | 2 | x |
| 2 | 2 | y |
| 4 | 2 | w |
| 6 | 2 | h |
| 8 | 1 | format, `0x01` = RGB565 little-endian |
| 9 | 1 | codec, `0x00` = NONE, `0x01` = RLE16 |

Ardindan `LEN - 10` bayt piksel verisi gelir. Baslik 10 bayt oldugu icin
piksel verisi cerceve basindan itibaren offset 20'de, yani yine cift
adreste basliyor.

Kurallar:

- `w` ve `h` sifir olamaz.
- `x + w` ekran genisligini, `y + h` ekran yuksekligini asamaz. Asarsa
  cihaz NACK `BAD_REGION` doner ve bolgeyi cizmez. Kirpma yapmaz; sessiz
  kirpma PC tarafindaki hatayi gizler.
- `codec = NONE` iken piksel verisi tam olarak `w * h * 2` bayt olmali.
- `codec = RLE16` iken cozulen veri tam olarak `w * h` piksel uretmeli.
  Az ya da cok uretirse NACK `DECODE_ERROR`.
- Piksel sirasi soldan saga, yukaridan asagiya.

**Bayt sirasi:** piksel verisi little-endian RGB565. Panel big-endian
bekliyor, cevrimi cihaz yapiyor (`setSwapBytes(true)`). Olcum bu cevrimin
maliyetinin yuzde 0.2 oldugunu gosterdi, yani PC tarafini karmasiklastirmaya
degmez.

**FILL_RECT neden yok:** dusunuldu ve elendi. Duz renkli bir dikdortgen
RLE ile zaten birkac bayta iniyor (tam ekran duz renk yaklasik 1.8 KB),
ayri bir mesaj tipi eklemenin karsiligi yok.

## RLE16 kodlayici

RGB565 piksellere uygulanan, PackBits benzeri basit bir kosu kodlamasi.
Birim bayt degil **piksel**, yani 16 bit.

Her belirtec bir kontrol baytiyla baslar:

| Kontrol | Anlam | Ardindan gelen |
|---|---|---|
| `0x00` - `0x7F` | Duz kosu, `n = kontrol + 1` piksel (1-128) | `n * 2` bayt |
| `0x80` - `0xFF` | Tekrar kosu, `n = (kontrol & 0x7F) + 2` piksel (2-129) | 2 bayt, tekrarlanacak piksel |

Ozellikler:

- **En kotu durum yuzde 0.4 buyume.** Hic sikismayan veride her 128 piksel
  icin 1 kontrol bayti eklenir, yani 256 bayta 1 bayt.
- **Duz renk yaklasik 86 kat siker.** Tam ekran duz renk 76800 piksel,
  129'luk kosularla 596 belirtec, 1788 bayt.
- Cozme hizi bellek kopyalama seviyesinde, ek tablo ya da durum gerekmiyor.

**Kodlayici kurali:** RLE ciktisi ham veriden kucuk degilse kodlayici
`codec = NONE` ile ham gonderir. Boylece protokol hicbir durumda veriyi
buyutmez.

**Neden LZ4 ya da baska bir sey degil:** olcum sikistirma hedefini 4.4 kat
olarak koydu. Arayuz icerigi (duz zeminler, metin, ikonlar) RLE ile bu
orani rahatlikla gecer. Fotograf ve gradyanlarda RLE ise yaramaz ama LZ4
de yaramazdi, onlarin cozumu JPEG. Codec alani 8 bit, ilerde LZ4 ya da
JPEG eklemek icin yer var.

## Akis kontrolu

**Piksel verisi icin cerceve basina ACK yok.** USB CDC zaten geri basinc
sagliyor: cihaz okumazsa host tarafindaki yazma blokluyor. Ustune bir
pencere mekanizmasi koymak her cerceveye gidis donus gecikmesi eklerdi.

Bunun yerine:

- PC, FLAGS bitinde `ACK iste` bayragini kaldirarak istedigi cerceve icin
  onay isteyebilir. Gecikme olcumu ve senkron noktalari icin.
- Cihaz hata durumunda kendiliginden NACK gonderir.
- PC, GET_STATUS ile sayaclari isteyebilir.

**STATUS payload (16 bayt)**

| Offset | Boyut | Alan |
|---|---|---|
| 0 | 4 | islenen cerceve sayisi |
| 4 | 4 | dusen cerceve sayisi |
| 8 | 2 | HDRCRC hatasi sayisi |
| 10 | 2 | payload CRC hatasi sayisi |
| 12 | 2 | senkron kaybi sayisi |
| 14 | 2 | son cerceve islenme suresi, mikrosaniye |

## Hata kodlari

| Kod | Ad | Anlam |
|---|---|---|
| `0x01` | BAD_HEADER_CRC | Baslik CRC'si tutmadi |
| `0x02` | BAD_PAYLOAD_CRC | Payload CRC'si tutmadi |
| `0x03` | BAD_VERSION | VER alani desteklenmiyor |
| `0x04` | UNKNOWN_TYPE | Bilinmeyen mesaj tipi |
| `0x05` | PAYLOAD_TOO_LARGE | LEN, max_payload degerinden buyuk |
| `0x06` | BAD_REGION | Sifir boyut ya da ekran disi |
| `0x07` | BAD_CODEC | Desteklenmeyen codec ya da format |
| `0x08` | DECODE_ERROR | RLE beklenen piksel sayisini uretmedi |
| `0x09` | OVERRUN | Alim tamponu doldu, cerceve dustu |

## Senkron ve kurtarma

Cihaz tarafindaki ayristirici dort durumlu:

```
HUNT_SOF -> HEADER -> PAYLOAD -> CRC -> (isle) -> HUNT_SOF
```

- **HUNT_SOF:** gelen baytlarda `A5 5A` dizisi aranir.
- **HEADER:** 8 bayt okunur, HDRCRC dogrulanir.
  - Tutmazsa: sayac artirilir, ilk SOF baytindan **sonraki** bayttan
    itibaren yeniden HUNT_SOF. Bir bayt geri gitmek onemli, cunku gercek
    bir SOF sahte olanla ust uste binmis olabilir.
  - LEN > max_payload ise NACK `PAYLOAD_TOO_LARGE`, HUNT_SOF.
- **PAYLOAD:** LEN bayt okunur.
- **CRC:** 2 bayt okunur ve dogrulanir. Tutmazsa NACK `BAD_PAYLOAD_CRC`,
  HUNT_SOF.
- **Zaman asimi:** HEADER ya da PAYLOAD durumundayken 500 ms boyunca bayt
  gelmezse cerceve birakilir, senkron kaybi sayaci artirilir, HUNT_SOF.

**Sahte cerceve olasiligi:** rastgele veride `A5 5A` yakalanma olasiligi
65536'da bir, ustune 8 bitlik HDRCRC geliyor. Ikisi birlikte rastgele bir
konumda gecerli baslik uretme olasiligini yaklasik 16.7 milyonda bire
indiriyor. Payload CRC'si de dogrulandigi icin pratikte sifir.

## Surumleme

- Her cerceve VER tasiyor. Su anki surum `01`.
- Cihaz tanimadigi bir VER gorurse NACK `BAD_VERSION` doner ve cerceveyi
  islemez. Baglanti kopmaz.
- Ayni surum icinde **yeni TYPE eklenebilir.** Tanimadigi bir TYPE gelirse
  cihaz NACK `UNKNOWN_TYPE` doner ama calismaya devam eder. Bu sayede yeni
  PC uygulamasi eski firmware ile konusabilir, sadece yeni ozellik calismaz.
- Alan eklemek ya da anlamini degistirmek surum artirir.

## Ornek cerceveler

Asagidaki dokumler gercek CRC degerleriyle uretildi, test vektoru olarak
kullanilabilir.

**FRAME_REGION:** (16, 32) konumunda 4x2 dolu kirmizi bolge, RLE16.
Bolge basligi, ardindan tek bir tekrar belirteci: 8 piksel, `0xF800`.

```
0000  A5 5A 01 10 00 07 0D 00 00 B2 10 00 20 00 04 00
0010  02 00 01 01 86 00 F8 C6 53
```

Ayristirmasi:

```
A5 5A        SOF
01           VER
10           TYPE = FRAME_REGION
00           FLAGS
07           SEQ
0D 00        LEN = 13
00           RSV
B2           HDRCRC
10 00        x = 16
20 00        y = 32
04 00        w = 4
02 00        h = 2
01           format = RGB565 LE
01           codec = RLE16
86           kontrol: tekrar, n = (0x06) + 2 = 8 piksel
00 F8        piksel = 0xF800 (kirmizi), little-endian
C6 53        CRC16
```

**PING:** payload yok, CRC16 baslangic degeri olarak `FF FF` gonderilir.

```
0000  A5 5A 01 30 00 08 00 00 00 CA FF FF
```

**NACK:** payload CRC hatasi (`0x02`), hatali cerceve SEQ `0x07`.

```
0000  A5 5A 01 83 00 00 02 00 00 9E 02 07 8A 0B
```

## Cihaz tarafi bellek butcesi

Protokolun uygulanabilir oldugunu gostermek icin:

| Tampon | Boyut | Yer |
|---|---|---|
| CDC halka tamponu | 16 KB | PSRAM |
| Cerceve alim tamponu | 64 KB | PSRAM |
| Cozulmus piksel tamponu x2 | 2 x 150 KB | PSRAM |
| Toplam | yaklasik 380 KB | 8 MB PSRAM icinde |

Cozulmus tamponun tam kare boyutunda olmasi gerekiyor, cunku RLE ile
kucucuk bir payload tam kare uretebilir. Iki tane olmasinin sebebi
ping-pong: biri ekrana basilirken digerine cozme yapilir.

Olcum 3 alim tamponlarinin PSRAM'de olmasinin maliyetinin yuzde 2
oldugunu gosterdi, dahili SRAM daha degerli bir kaynak.

## Cihaz tarafi is dagilimi

- **Cekirdek 0:** USB CDC okuma, cerceve ayristirma, RLE cozme
- **Cekirdek 1:** cozulmus bolgeyi ekrana basma

Bloklayan `pushImage` sirasinda CPU mesgul kaliyor. Ayri cekirdekler
sayesinde bu sure alimi engellemiyor.

## Acik sorular

Uygulama sirasinda cevaplanacak, simdi karara baglanmadi.

- CDC yazma birlestirmesi gecikmeyi ne kadar etkiliyor? PC tarafinin bir
  cerceveyi tek `write` cagrisinda gondermesi gerekiyor mu?
- `rx_slots` degeri kac olmali? Olcumle belirlenecek.
- Zaman asimi 500 ms uygun mu? Gercek CDC davranisiyla dogrulanacak.
