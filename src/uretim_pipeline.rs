//! Tek tuşla üretim hattının stok medya orkestratörü.
//!
//! Kullanıcı dosya yolu aramaz. Onaylanmış medya kayıtlarından ve yerel AI'nın
//! sektör klasörüne yazdığı `ai_*` karelerinden 2 fotoğraf seçilir.
//! Fotoğraflar demo ve arşiv için `photo_N.jpg` adıyla kanonikleştirilir.
//! **Video hattın parçası değildir**: ücretsiz video sitelerinden elle eklenir,
//! arşivde varsa demoda hero alanına bağlanır.
//! Böylece demo üretimi, panelde seçim yapılmamış olsa bile aynı sözleşmeyi
//! kullanır.

use std::fs;
use std::path::Path;

use crate::medya_onay;
use crate::sektor_medya;

/// Bir sektör için otomatik seçilen onaylı medya.
#[derive(Clone, Debug, Default)]
pub struct StokSecim {
    pub video: Option<String>,
    pub fotolar: Vec<String>,
    pub mesaj: String,
}

impl StokSecim {
    /// Demo için gereken 2 fotoğraf hazır mı? Video isteğe bağlıdır.
    pub fn hazir(&self) -> bool {
        self.fotolar.len() >= 2
    }

    /// Tek tuşun tamamlaması gereken fotoğraf sayısı (0–2).
    pub fn eksik_foto(&self) -> usize {
        2usize.saturating_sub(self.fotolar.len())
    }

    /// İşletmenin medya defterine yazılacak kanonik yollar.
    pub fn medya(&self) -> Vec<String> {
        self.video
            .iter()
            .chain(self.fotolar.iter())
            .cloned()
            .collect()
    }
}

/// Proje kökündeki onay kayıtlarından sektörün medyasını hazırlar.
pub fn hazirla(slug: &str) -> StokSecim {
    hazirla_yoldan(Path::new("."), slug)
}

/// Test edilebilir kök parametreli sürüm.
pub fn hazirla_yoldan(kok: &Path, slug: &str) -> StokSecim {
    let kayit = medya_onay::kayit(kok, slug);
    let video = kayit
        .videos
        .iter()
        .find(|yol| kok.join(yol).is_file())
        .cloned();

    let mut fotolar = Vec::new();
    for (sira, kaynak) in foto_kaynaklari(kok, slug, &kayit)
        .iter()
        .take(2)
        .enumerate()
    {
        if let Some(yol) = kanonik_foto(kok, slug, kaynak, sira + 1) {
            fotolar.push(yol);
        }
    }

    let mesaj = match (fotolar.len(), video.is_some()) {
        (2.., true) => "2 fotoğraf + arşiv videosu hazır; demo otomatik üretilebilir.".to_string(),
        (2.., false) => {
            "2 fotoğraf hazır; demo fotoğrafla üretilecek (video isteğe bağlı).".to_string()
        }
        (1, _) => "1 fotoğraf hazır; eksik kare yerel AI ile üretilecek.".to_string(),
        _ => "2 fotoğraf gerekiyor; yerel AI ile üretilecek.".to_string(),
    };

    StokSecim {
        video,
        fotolar,
        mesaj,
    }
}

/// Fotoğraf adayları: önce onaylı kayıtlar, sonra yerel AI'nın sektör
/// klasörüne doğrudan yazdığı `ai_*` kareleri. AI kareleri ayrı bir onay
/// adımı beklemez; üretildiği anda hattın girdisidir. En yeni üretim öndedir.
fn foto_kaynaklari(kok: &Path, slug: &str, kayit: &medya_onay::Kayit) -> Vec<String> {
    let mut adaylar: Vec<String> = kayit
        .fotolar
        .iter()
        .filter(|yol| kok.join(yol).is_file())
        .cloned()
        .collect();
    let mut ai: Vec<String> = Vec::new();
    if let Ok(girisler) = fs::read_dir(kok.join(sektor_medya::KLASOR).join(slug)) {
        for giris in girisler.flatten() {
            let yol = giris.path();
            if !yol.is_file() {
                continue;
            }
            let ad = giris.file_name().to_string_lossy().to_ascii_lowercase();
            let uzanti = ad.rsplit('.').next().unwrap_or_default().to_string();
            if !ad.starts_with("ai_") || !matches!(uzanti.as_str(), "jpg" | "jpeg" | "png" | "webp")
            {
                continue;
            }
            if let Some(goreli) = goreli(kok, &yol) {
                ai.push(goreli);
            }
        }
    }
    // Adlar zaman damgası taşır: sondaki en yeni karedir.
    ai.sort();
    ai.reverse();
    for yol in ai {
        if !adaylar.contains(&yol) {
            adaylar.push(yol);
        }
    }
    adaylar
}

/// Onaylı fotoğrafı `media/stock/<slug>/photo_N.jpg` olarak yazar.
/// Kaynak zaten aynı dosyaysa veya hedef kaynaktan yeni değilse yeniden
/// kodlama yapılmaz; pipeline her karede çalışsa da disk yorulmaz.
fn kanonik_foto(kok: &Path, slug: &str, kaynak: &str, sira: usize) -> Option<String> {
    let kaynak = kok.join(kaynak);
    if !kaynak.is_file() {
        return None;
    }
    let hedef = kok
        .join(sektor_medya::KLASOR)
        .join(slug)
        .join(format!("photo_{sira}.jpg"));
    if let Some(ust) = hedef.parent() {
        fs::create_dir_all(ust).ok()?;
    }
    if ayni_veya_daha_yeni(&kaynak, &hedef) {
        return goreli(kok, &hedef);
    }
    let resim = image::open(&kaynak).ok()?.to_rgb8();
    // JPEG alfa kanalını desteklemez; WebP/PNG kaynaklarını RGB'ye indir.
    image::save_buffer(
        &hedef,
        resim.as_raw(),
        resim.width(),
        resim.height(),
        image::ColorType::Rgb8,
    )
    .ok()?;
    goreli(kok, &hedef)
}

fn ayni_veya_daha_yeni(kaynak: &Path, hedef: &Path) -> bool {
    let Ok(kaynak_meta) = fs::metadata(kaynak) else {
        return false;
    };
    let Ok(hedef_meta) = fs::metadata(hedef) else {
        return false;
    };
    match (kaynak_meta.modified(), hedef_meta.modified()) {
        (Ok(kaynak_zaman), Ok(hedef_zaman)) => hedef_zaman >= kaynak_zaman,
        _ => false,
    }
}

fn goreli(kok: &Path, dosya: &Path) -> Option<String> {
    let goreli = dosya.strip_prefix(kok).ok()?;
    Some(goreli.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SAYAC: AtomicUsize = AtomicUsize::new(0);

    fn gecici_kok() -> PathBuf {
        let no = SAYAC.fetch_add(1, Ordering::Relaxed);
        let kok =
            std::env::temp_dir().join(format!("nexus_pipeline_{}_{}", std::process::id(), no));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(kok.join("media/stock/cekici/video")).unwrap();
        fs::create_dir_all(kok.join("media/stock/cekici/photos")).unwrap();
        kok
    }

    #[test]
    fn onayli_bir_video_ve_iki_fotografi_secer() {
        let kok = gecici_kok();
        // Gerçek PNG üretip onay kaydına bağlıyoruz; pipeline bunları
        // `photo_N.jpg` olarak kanonikleştirmeli.
        fs::write(kok.join("media/stock/cekici/video/hero_1.mp4"), [0u8; 4]).unwrap();
        let foto = image::DynamicImage::new_rgb8(2, 2);
        foto.save(kok.join("media/stock/cekici/photos/img_1.png"))
            .unwrap();
        foto.save(kok.join("media/stock/cekici/photos/img_2.png"))
            .unwrap();
        let kayit = serde_json::json!({
            "sektorler": {
                "cekici": {
                    "videolar": ["media/stock/cekici/video/hero_1.mp4"],
                    "fotolar": [
                        "media/stock/cekici/photos/img_1.png",
                        "media/stock/cekici/photos/img_2.png"
                    ]
                }
            }
        });
        fs::write(
            kok.join(medya_onay::DOSYA),
            serde_json::to_string(&kayit).unwrap(),
        )
        .unwrap();
        let secim = hazirla_yoldan(&kok, "cekici");
        assert!(secim.hazir(), "{}", secim.mesaj);
        assert_eq!(secim.medya().len(), 3);
        assert!(secim.medya()[0].ends_with("video/hero_1.mp4"));
        assert!(secim.fotolar.iter().all(|yol| yol.ends_with(".jpg")));
        assert!(kok.join(&secim.fotolar[0]).is_file());
        assert!(kok.join(&secim.fotolar[1]).is_file());
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn video_olmadan_iki_fotograf_demo_hazir_sayar() {
        let kok = gecici_kok();
        let foto = image::DynamicImage::new_rgb8(2, 2);
        foto.save(kok.join("media/stock/cekici/photos/img_1.png"))
            .unwrap();
        foto.save(kok.join("media/stock/cekici/photos/img_2.png"))
            .unwrap();
        let kayit = serde_json::json!({
            "sektorler": {
                "cekici": {
                    "videolar": [],
                    "fotolar": [
                        "media/stock/cekici/photos/img_1.png",
                        "media/stock/cekici/photos/img_2.png"
                    ]
                }
            }
        });
        fs::write(
            kok.join(medya_onay::DOSYA),
            serde_json::to_string(&kayit).unwrap(),
        )
        .unwrap();
        let secim = hazirla_yoldan(&kok, "cekici");
        assert!(secim.hazir(), "{}", secim.mesaj);
        assert!(secim.video.is_none(), "video zorunlu olmamalı");
        assert_eq!(secim.medya().len(), 2);
        assert_eq!(secim.eksik_foto(), 0);
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn yerel_ai_kareleri_onay_beklemeden_secime_girer() {
        let kok = gecici_kok();
        let foto = image::DynamicImage::new_rgb8(2, 2);
        fs::write(kok.join(medya_onay::DOSYA), "{}").unwrap();
        foto.save(kok.join("media/stock/cekici/ai_20261106_120000.jpg"))
            .unwrap();
        let tek = hazirla_yoldan(&kok, "cekici");
        assert_eq!(tek.fotolar.len(), 1, "{}", tek.mesaj);
        assert_eq!(tek.eksik_foto(), 1);
        assert!(!tek.hazir());
        foto.save(kok.join("media/stock/cekici/ai_20261106_121000.jpg"))
            .unwrap();
        let iki = hazirla_yoldan(&kok, "cekici");
        assert!(iki.hazir(), "{}", iki.mesaj);
        assert!(iki.fotolar.iter().all(|yol| yol.ends_with(".jpg")));
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn eksik_fotograf_demo_hazir_saymaz() {
        let kok = gecici_kok();
        fs::write(kok.join("media/stock/cekici/video/hero_1.mp4"), [0u8; 4]).unwrap();
        let kayit = serde_json::json!({
            "sektorler": {
                "cekici": {
                    "videolar": ["media/stock/cekici/video/hero_1.mp4"],
                    "fotolar": []
                }
            }
        });
        fs::write(
            kok.join(medya_onay::DOSYA),
            serde_json::to_string(&kayit).unwrap(),
        )
        .unwrap();
        let secim = hazirla_yoldan(&kok, "cekici");
        assert!(!secim.hazir());
        assert!(secim.video.is_some());
        assert!(secim.fotolar.is_empty());
        let _ = fs::remove_dir_all(&kok);
    }
}
