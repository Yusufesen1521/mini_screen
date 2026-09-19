//! Kirli dikdortgen takibi.
//!
//! Faz 2 cikis kriteri: "ekranda hicbir sey degismiyorken USB trafigi
//! sifira yakin". Bu modul o kriteri saglayan yer. Kare uretilir, onceki
//! kareyle karsilastirilir, sadece degisen dikdortgenler gonderilir.
//!
//! Karo olcusu 16x16. Olcum tam kare diff'in 0.0036 ms surdugunu
//! gosterdi, yani bu karsilastirma butcede gorunmuyor.

use crate::render::Rect;

/// Karo kenari. Kucultmek daha sivri dikdortgen verir ama karo basina
/// sabit maliyet artar; buyutmek gereksiz piksel gonderir.
pub const TILE: u16 = 16;

pub struct DirtyTracker {
    width: u16,
    height: u16,
    tcols: u16,
    trows: u16,
    prev: Vec<u16>,
    tiles: Vec<bool>,
    /// Ilk karede her sey kirli sayilir, yoksa ekran bos kalir.
    first: bool,
}

impl DirtyTracker {
    pub fn new(width: u16, height: u16) -> DirtyTracker {
        let tcols = width.div_ceil(TILE);
        let trows = height.div_ceil(TILE);
        DirtyTracker {
            width,
            height,
            tcols,
            trows,
            prev: vec![0u16; width as usize * height as usize],
            tiles: vec![false; tcols as usize * trows as usize],
            first: true,
        }
    }

    /// Bir sonraki karsilastirmada her seyi kirli say.
    ///
    /// Baglanti koptuktan sonra cihazin ekraninda ne oldugunu bilmiyoruz,
    /// bu yuzden yeniden baglanmada cagrilmali.
    pub fn force_full(&mut self) {
        self.first = true;
    }

    /// `cur` ile onceki kareyi karsilastirir, kirli dikdortgenleri `out`
    /// icine yazar ve `cur` u yeni referans yapar.
    pub fn diff(&mut self, cur: &[u16], out: &mut Vec<Rect>) {
        debug_assert_eq!(cur.len(), self.prev.len());
        out.clear();

        if self.first {
            self.first = false;
            self.prev.copy_from_slice(cur);
            out.push(Rect::new(0, 0, self.width, self.height));
            return;
        }

        for ty in 0..self.trows {
            for tx in 0..self.tcols {
                self.tiles[(ty * self.tcols + tx) as usize] = self.tile_differs(cur, tx, ty);
            }
        }

        self.merge_tiles(out);
        self.prev.copy_from_slice(cur);
    }

    fn tile_differs(&self, cur: &[u16], tx: u16, ty: u16) -> bool {
        let x0 = tx * TILE;
        let y0 = ty * TILE;
        let w = TILE.min(self.width - x0) as usize;
        let h = TILE.min(self.height - y0);
        for y in 0..h {
            let off = (y0 + y) as usize * self.width as usize + x0 as usize;
            if self.prev[off..off + w] != cur[off..off + w] {
                return true;
            }
        }
        false
    }

    /// Kirli karolari dikdortgenlere birlestirir.
    ///
    /// Once satir icinde yatay kosular, sonra ustuste gelen ayni kosular
    /// dikey olarak birlestirilir. Amac cerceve sayisini azaltmak: her
    /// dikdortgen ayri bir protokol cercevesi ve her cercevenin 12 bayt
    /// sabit maliyeti var.
    fn merge_tiles(&self, out: &mut Vec<Rect>) {
        // (x0_karo, x1_karo, y0_karo, y1_karo) olarak acik kosular
        let mut open: Vec<(u16, u16, u16, u16)> = Vec::new();
        let mut done: Vec<(u16, u16, u16, u16)> = Vec::new();

        for ty in 0..self.trows {
            let mut runs: Vec<(u16, u16)> = Vec::new();
            let mut tx = 0;
            while tx < self.tcols {
                if self.tiles[(ty * self.tcols + tx) as usize] {
                    let start = tx;
                    while tx < self.tcols && self.tiles[(ty * self.tcols + tx) as usize] {
                        tx += 1;
                    }
                    runs.push((start, tx));
                } else {
                    tx += 1;
                }
            }

            let mut still_open: Vec<(u16, u16, u16, u16)> = Vec::new();
            for (rs, re) in runs {
                // Ustteki ayni kosuyu uzat, yoksa yeni kosu ac.
                if let Some(i) = open
                    .iter()
                    .position(|&(x0, x1, _, y1)| x0 == rs && x1 == re && y1 == ty)
                {
                    let mut r = open.remove(i);
                    r.3 = ty + 1;
                    still_open.push(r);
                } else {
                    still_open.push((rs, re, ty, ty + 1));
                }
            }
            done.append(&mut open);
            open = still_open;
        }
        done.append(&mut open);

        for (x0, x1, y0, y1) in done {
            let px = x0 * TILE;
            let py = y0 * TILE;
            let pw = (x1 * TILE).min(self.width) - px;
            let ph = (y1 * TILE).min(self.height) - py;
            if pw > 0 && ph > 0 {
                out.push(Rect::new(px, py, pw, ph));
            }
        }
    }

    /// Bir dikdortgenin piksellerini bitisik bir tampona kopyalar.
    ///
    /// Protokol bolgeyi soldan saga, yukaridan asagiya bitisik bekliyor.
    pub fn copy_region(src: &[u16], width: u16, r: Rect, dst: &mut Vec<u16>) {
        dst.clear();
        dst.reserve(r.pixel_count());
        for y in 0..r.h {
            let off = (r.y + y) as usize * width as usize + r.x as usize;
            dst.extend_from_slice(&src[off..off + r.w as usize]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u16 = 64;
    const H: u16 = 48;

    fn tracker() -> (DirtyTracker, Vec<u16>) {
        (DirtyTracker::new(W, H), vec![0u16; W as usize * H as usize])
    }

    #[test]
    fn ilk_kare_tam_ekran() {
        let (mut t, buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        assert_eq!(out, vec![Rect::new(0, 0, W, H)]);
    }

    /// Cikis kriterinin ta kendisi: degisiklik yoksa trafik sifir.
    #[test]
    fn degisiklik_yoksa_dikdortgen_yok() {
        let (mut t, buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        t.diff(&buf, &mut out);
        assert!(out.is_empty(), "bos karede {} dikdortgen cikti", out.len());
    }

    #[test]
    fn tek_piksel_tek_karoyu_kirletir() {
        let (mut t, mut buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        buf[20 * W as usize + 33] = 0xFFFF;
        t.diff(&buf, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], Rect::new(32, 16, TILE, TILE));
    }

    #[test]
    fn yatay_komsular_tek_dikdortgende_birlesir() {
        let (mut t, mut buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        for x in 0..48usize {
            buf[5 * W as usize + x] = 0xFFFF;
        }
        t.diff(&buf, &mut out);
        assert_eq!(out.len(), 1, "yatay kosu birlesmedi: {:?}", out);
        assert_eq!(out[0], Rect::new(0, 0, 48, TILE));
    }

    #[test]
    fn dikey_komsular_tek_dikdortgende_birlesir() {
        let (mut t, mut buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        for y in 0..40usize {
            buf[y * W as usize + 5] = 0xFFFF;
        }
        t.diff(&buf, &mut out);
        assert_eq!(out.len(), 1, "dikey kosu birlesmedi: {:?}", out);
        assert_eq!(out[0], Rect::new(0, 0, TILE, 48));
    }

    #[test]
    fn ayrik_bolgeler_ayri_kaliyor() {
        let (mut t, mut buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        buf[0] = 0xFFFF;
        buf[40 * W as usize + 60] = 0xFFFF;
        t.diff(&buf, &mut out);
        assert_eq!(out.len(), 2, "{:?}", out);
    }

    #[test]
    fn force_full_tam_ekran_verir() {
        let (mut t, buf) = tracker();
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        t.force_full();
        t.diff(&buf, &mut out);
        assert_eq!(out, vec![Rect::new(0, 0, W, H)]);
    }

    #[test]
    fn bolge_kopyasi_bitisik() {
        let mut src = vec![0u16; W as usize * H as usize];
        for (i, v) in src.iter_mut().enumerate() {
            *v = i as u16;
        }
        let mut dst = Vec::new();
        DirtyTracker::copy_region(&src, W, Rect::new(2, 1, 3, 2), &mut dst);
        assert_eq!(dst, vec![W + 2, W + 3, W + 4, 2 * W + 2, 2 * W + 3, 2 * W + 4]);
    }

    /// Ekran olcusu karoya tam bolunmuyorsa kenar karolar tasmamali.
    #[test]
    fn kenar_karolari_ekran_disina_tasmiyor() {
        let mut t = DirtyTracker::new(20, 20);
        let mut buf = vec![0u16; 400];
        let mut out = Vec::new();
        t.diff(&buf, &mut out);
        buf[19 * 20 + 19] = 0xFFFF;
        t.diff(&buf, &mut out);
        assert_eq!(out.len(), 1);
        let r = out[0];
        assert!(r.x + r.w <= 20 && r.y + r.h <= 20, "tasma: {:?}", r);
    }
}
