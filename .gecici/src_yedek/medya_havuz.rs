//! Yerel medya havuzu: `media/stock` ve `media/sektor` altındaki gerçek
//! video ve fotoğraflar. Ağa çıkılmaz, dış kaynak aranmaz; işletme adı +
//! adresinden türeyen tohumla her işletmeye farklı video/fotoğraf
//! kombinasyonu dağıtılır. Aynı sektördeki iki işletme bile aynı sırayı,
//! aynı hero türünü ve aynı hareketi almaz.

use std::path::{Path, PathBuf};

/// Havuzdaki tek bir video kaydı.
#[derive(Clone, Debug)]
pub struct VideoKaydi {
    /// Dosyanın bulunduğu sektör klasörü (`kuafor`, `restoran`, ...).
    pub sektor: String,
    pub dosya: PathBuf,
    pub poster: Option<PathBuf>,
}

/// Sektöre göre toplanmış yerel medya.
#[derive(Clone, Debug, Default)]
pub struct Havuz {
    pub videolar: Vec<VideoKaydi>,
    pub kareler: Vec<(String, PathBuf)>,
}

/// Bir işletmeye dağıtılan medya.
#[derive(Clone, Debug, Default)]
pub struct Dagitim {
    pub video: Option<VideoKaydi>,
    /// Tohumla karıştırılmış kareler: ilki hero, kalanı galeri.
    pub kareler: Vec<PathBuf>,
    /// Hero'da video mu oynayacak? `false` ise hareketli fotoğraf (Ken Burns).
    pub video_hero: bool,
}

/// 64 bit karıştırıcı (SplitMix64).
struct Karistirici {
    durum: u64,
}

impl Karistirici {
    fn yeni(tohum: u64) -> Self {
        Self {
            durum: tohum ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn sonraki(&mut self) -> u64 {
        self.durum = self.durum.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut x = self.durum;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }
}

fn uzanti(yol: &Path) -> String {
    yol.extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase())
        .unwrap_or_default()
}

fn video_mu(yol: &Path) -> bool {
    matches!(uzanti(yol).as_str(), "mp4" | "webm" | "mov" | "m4v")
}

fn foto_mu(yol: &Path) -> bool {
    matches!(uzanti(yol).as_str(), "jpg" | "jpeg" | "png" | "webp")
}

/// Klasördeki dosyalar (alt klasörlere inmeden).
fn dosyalar(klasor: &Path) -> Vec<PathBuf> {
    let Ok(girisler) = std::fs::read_dir(klasor) else {
        return Vec::new();
    };
    let mut cikti: Vec<PathBuf> = girisler
        .filter_map(|giris| giris.ok())
        .map(|giris| giris.path())
        .filter(|yol| yol.is_file())
        .collect();
    cikti.sort();
    cikti
}

/// Videonun posteri: aynı adlı `.webp`, yoksa klasördeki `poster.webp`.
fn poster_bul(video: &Path) -> Option<PathBuf> {
    let klasor = video.parent()?;
    if let Some(govde) = video.file_stem() {
        for uzanti in ["webp", "jpg", "png"] {
            let aday = klasor.join(format!("{}.{uzanti}", govde.to_string_lossy()));
            if aday.is_file() {
                return Some(aday);
            }
        }
    }
    for ad in ["poster.webp", "poster.jpg"] {
        let aday = klasor.join(ad);
        if aday.is_file() {
            return Some(aday);
        }
    }
    None
}

/// Bir sektör klasöründeki videolar: kök `hero.mp4`, `v1.mp4` ve `video/`.
fn sektor_videolari(sektor: &str, klasor: &Path) -> Vec<VideoKaydi> {
    let mut cikti = Vec::new();
    let mut adaylar = dosyalar(klasor);
    adaylar.extend(dosyalar(&klasor.join("video")));
    for yol in adaylar {
        if !video_mu(&yol) {
            continue;
        }
        // Hibrit arşivde `loop/loop_*.mp4` kısa döngülerdir; hero adayı değil.
        if yol
            .file_stem()
            .and_then(|a| a.to_str())
            .is_some_and(|ad| ad.to_ascii_lowercase().starts_with("loop_"))
        {
            continue;
        }
        cikti.push(VideoKaydi {
            sektor: sektor.to_string(),
            poster: poster_bul(&yol),
            dosya: yol,
        });
    }
    cikti
}

/// Bir sektör klasöründeki fotoğraflar: `photos/`, `fotolar/` ve kök.
/// Video posterleri (poster.webp) fotoğraf sayılmaz.
fn sektor_fotolari(sektor: &str, klasor: &Path) -> Vec<(String, PathBuf)> {
    let mut cikti = Vec::new();
    let mut adaylar = dosyalar(&klasor.join("photos"));
    adaylar.extend(dosyalar(&klasor.join("fotolar")));
    for yol in adaylar {
        if foto_mu(&yol) {
            cikti.push((sektor.to_string(), yol));
        }
    }
    cikti
}

/// Havuzu tarar: `media/stock/<sektör>` ve `media/sektor/<sektör>`.
pub fn tara(kok: &Path) -> Havuz {
    let mut havuz = Havuz::default();
    for ust in ["media/stock", "media/sektor"] {
        let dizin = kok.join(ust);
        let Ok(girisler) = std::fs::read_dir(&dizin) else {
            continue;
        };
        let mut sektorler: Vec<(String, PathBuf)> = girisler
            .filter_map(|giris| giris.ok())
            .map(|giris| giris.path())
            .filter(|yol| yol.is_dir())
            .filter_map(|yol| {
                let ad = yol.file_name()?.to_str()?.to_string();
                Some((ad, yol))
            })
            .collect();
        sektorler.sort();
        for (sektor, klasor) in sektorler {
            havuz.videolar.extend(sektor_videolari(&sektor, &klasor));
            havuz.kareler.extend(sektor_fotolari(&sektor, &klasor));
        }
    }
    havuz
}

/// Onaylı havuzdan işletmeye dağıtır; onay yoksa hiçbir dosya verilmez.
pub fn dagit(sektor: &str, tohum: u64, adet: usize) -> Dagitim {
    dagit_onayli(Path::new("."), sektor, tohum, adet)
}

/// İnsan onaylı dağıtım: yalnız kullanıcının [ONAYLA] verdiği 1 video ve
/// 2 fotoğraf demoya girer. Onaysız medya diskte dursa bile kullanılmaz.
pub fn dagit_onayli(kok: &Path, sektor: &str, tohum: u64, adet: usize) -> Dagitim {
    let izinli: Vec<String> = crate::medya_onay::onayli_medya(kok, sektor)
        .iter()
        .map(|dosya| crate::medya_onay::goreli(kok, dosya))
        .collect();
    if izinli.is_empty() {
        return Dagitim::default();
    }
    let mut dagitim = dagit_yoldan(kok, sektor, tohum, adet);
    dagitim
        .kareler
        .retain(|dosya| izinli.contains(&crate::medya_onay::goreli(kok, dosya)));
    if dagitim.kareler.len() > crate::medya_onay::FOTO_KOTA {
        dagitim.kareler.truncate(crate::medya_onay::FOTO_KOTA);
    }
    let video_izinli = dagitim
        .video
        .as_ref()
        .is_some_and(|kayit| izinli.contains(&crate::medya_onay::goreli(kok, &kayit.dosya)));
    if !video_izinli {
        dagitim.video = None;
        dagitim.video_hero = false;
    }
    dagitim
}

/// Testler ve taşınabilir kullanım için kök yolu verilebilir sürüm.
pub fn dagit_yoldan(kok: &Path, sektor: &str, tohum: u64, adet: usize) -> Dagitim {
    let havuz = tara(kok);
    let sektor_esit = |kayit: &str| sektor.is_empty() || kayit.eq_ignore_ascii_case(sektor);
    let mut videolar: Vec<VideoKaydi> = havuz
        .videolar
        .iter()
        .filter(|kayit| sektor_esit(&kayit.sektor))
        .cloned()
        .collect();
    let mut kareler: Vec<PathBuf> = havuz
        .kareler
        .iter()
        .filter(|(kayit, _)| sektor_esit(kayit))
        .map(|(_, yol)| yol.clone())
        .collect();
    if videolar.is_empty() && kareler.is_empty() {
        return Dagitim::default();
    }
    // Tohumla karıştır: komşu işletmeler farklı kareyle açılır.
    let mut k = Karistirici::yeni(tohum);
    for i in (1..kareler.len()).rev() {
        let j = (k.sonraki() % (i as u64 + 1)) as usize;
        kareler.swap(i, j);
    }
    for i in (1..videolar.len()).rev() {
        let j = (k.sonraki() % (i as u64 + 1)) as usize;
        videolar.swap(i, j);
    }
    // Havuzda video varsa hero video olur; video yoksa hareketli fotoğraf.
    let video_hero = !videolar.is_empty();
    let video = video_hero.then(|| videolar[0].clone());
    let adet = adet.max(1).min(kareler.len());
    kareler.truncate(adet);
    Dagitim {
        video,
        kareler,
        video_hero,
    }
}

/// Hero için seçilen havuz videosu (demo klasörüne kopyalanır).
pub fn hero_video(sektor: &str, tohum: u64) -> Option<VideoKaydi> {
    let dagitim = dagit(sektor, tohum, 4);
    dagitim.video
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek_havuz(ad: &str) -> PathBuf {
        let kok = std::env::temp_dir().join(format!("nexus_havuz_{}_{}", std::process::id(), ad));
        let _ = std::fs::remove_dir_all(&kok);
        for (sektor, foto) in [("kuafor", "a"), ("kuafor", "b"), ("kuafor", "c")] {
            let klasor = kok.join("media/stock").join(sektor).join("photos");
            std::fs::create_dir_all(&klasor).unwrap();
            std::fs::write(klasor.join(format!("{foto}.jpg")), b"sahte-kare").unwrap();
        }
        let video = kok.join("media/stock/kuafor/video");
        std::fs::create_dir_all(&video).unwrap();
        std::fs::write(video.join("hero.mp4"), b"sahte-video").unwrap();
        std::fs::write(video.join("poster.webp"), b"sahte-poster").unwrap();
        let diger = kok.join("media/stock/klinik/photos");
        std::fs::create_dir_all(&diger).unwrap();
        std::fs::write(diger.join("x.jpg"), b"baska-sektor").unwrap();
        kok
    }

    #[test]
    fn havuz_sektorleri_ayirir_ve_videoyu_bulur() {
        let kok = ornek_havuz("ayir");
        let havuz = tara(&kok);
        assert_eq!(havuz.videolar.len(), 1, "bir video beklenir");
        assert_eq!(havuz.kareler.len(), 4, "üç kuaför + bir klinik kare");
        // Tohum hero türünü de seçer: video hero veren bir tohum bulunur.
        let (_, kuaför) = (0..40u64)
            .map(|tohum| (tohum, dagit_yoldan(&kok, "kuafor", tohum, 3)))
            .find(|(_, dagitim)| dagitim.video_hero)
            .expect("video hero veren bir tohum bulunmalı");
        assert_eq!(kuaför.kareler.len(), 3, "yalnız kuaför kareleri");
        assert!(kuaför.video.is_some(), "video hero seçilmeli");
        assert!(kuaför
            .kareler
            .iter()
            .all(|yol| yol.to_string_lossy().contains("kuafor")));
        let _ = std::fs::remove_dir_all(kok);
    }

    #[test]
    fn ayni_sektor_farkli_tohum_farkli_kare_alir() {
        let kok = ornek_havuz("tohum");
        let a = dagit_yoldan(&kok, "kuafor", 11, 3);
        let b = dagit_yoldan(&kok, "kuafor", 12, 3);
        let c = dagit_yoldan(&kok, "kuafor", 11, 3);
        assert_eq!(a.kareler, c.kareler, "aynı tohum aynı dağıtımı almalı");
        assert_ne!(a.kareler, b.kareler, "komşu işletme farklı sıra almalı");
        let _ = std::fs::remove_dir_all(kok);
    }

    #[test]
    fn loop_dosyalari_hero_secilmez() {
        let kok = std::env::temp_dir().join(format!("nexus_havuz_loop_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        let video = kok.join("media/stock/kuafor/video");
        let loop_klasor = kok.join("media/stock/kuafor/loop");
        let foto = kok.join("media/stock/kuafor/photos");
        for klasor in [&video, &loop_klasor, &foto] {
            std::fs::create_dir_all(klasor).unwrap();
        }
        std::fs::write(video.join("hero_1.mp4"), b"hero").unwrap();
        std::fs::write(loop_klasor.join("loop_1.mp4"), b"loop").unwrap();
        std::fs::write(foto.join("img_1.webp"), b"kare").unwrap();
        let havuz = tara(&kok);
        assert_eq!(havuz.videolar.len(), 1, "yalnız hero aday olmalı");
        assert_eq!(havuz.videolar[0].dosya, video.join("hero_1.mp4"));
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn onaysiz_medya_demoya_girmez() {
        let kok = ornek_havuz("onay");
        let onaysiz = dagit_onayli(&kok, "kuafor", 5, 4);
        assert!(onaysiz.video.is_none(), "onaysız video dağıtılmamalı");
        assert!(onaysiz.kareler.is_empty(), "onaysız kare dağıtılmamalı");

        for (alt, ad) in [
            ("photos", "a.jpg"),
            ("photos", "b.jpg"),
            ("video", "hero.mp4"),
            ("video", "hero_2.mp4"),
        ] {
            let klasor = kok.join("media/pending/kuafor").join(alt);
            std::fs::create_dir_all(&klasor).unwrap();
            let dosya = klasor.join(ad);
            std::fs::write(&dosya, b"onay").unwrap();
            let goreli = crate::medya_onay::goreli(&kok, &dosya);
            crate::medya_onay::onayla_dosya(&kok, "kuafor", &goreli).unwrap();
        }
        let onayli = dagit_onayli(&kok, "kuafor", 5, 4);
        assert!(onayli.video.is_some(), "onaylı video dağıtılmalı");
        assert_eq!(
            onayli.kareler.len(),
            crate::medya_onay::FOTO_KOTA,
            "kotadan fazla kare dağıtılmamalı"
        );
        let _ = std::fs::remove_dir_all(kok);
    }

    #[test]
    fn video_olmadan_da_fotograf_dagitilir() {
        let kok = ornek_havuz("foto");
        let _ = std::fs::remove_file(kok.join("media/stock/kuafor/video/hero.mp4"));
        let dagitim = dagit_yoldan(&kok, "kuafor", 5, 4);
        assert!(dagitim.video.is_none(), "video yoksa hero fotoğraf olmalı");
        assert!(!dagitim.video_hero);
        assert_eq!(dagitim.kareler.len(), 3);
        let _ = std::fs::remove_dir_all(kok);
    }
}
