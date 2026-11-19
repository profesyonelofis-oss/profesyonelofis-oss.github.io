//! Manuel medya kalite eşiği ve arşiv raporu.
//!
//! Otomatik tedarik hattı kaldırıldı. Bu modülde yalnız manuel panelin
//! kullandığı 1080p kalite kapısı ile `--bos-sektorler` raporu kalır.

use std::fs;
use std::path::Path;

/// Kaynak videonun en küçük ölçüsü: 720p altı kabul edilmez.
pub const KAYNAK_VIDEO_MIN_GENISLIK: u32 = 1280;
pub const KAYNAK_VIDEO_MIN_YUKSEKLIK: u32 = 720;
/// Kaynak fotoğrafın en küçük ölçüsü: 1920x1080 altı kabul edilmez.
pub const KAYNAK_FOTO_MIN_GENISLIK: u32 = 1920;
pub const KAYNAK_FOTO_MIN_YUKSEKLIK: u32 = 1080;

/// Yükleme öncesi kalite kapısı: video 720p, fotoğraf 1080p altı işlenmez.
pub fn kaynak_kaliteli(genislik: u32, yukseklik: u32, video: bool) -> bool {
    if video {
        genislik >= KAYNAK_VIDEO_MIN_GENISLIK && yukseklik >= KAYNAK_VIDEO_MIN_YUKSEKLIK
    } else {
        genislik >= KAYNAK_FOTO_MIN_GENISLIK && yukseklik >= KAYNAK_FOTO_MIN_YUKSEKLIK
    }
}

/// Bir sektör klasöründeki onaylı video ve fotoğraf sayısı.
fn mevcut_sayilar(kok: &Path) -> (usize, usize) {
    let say = |alt: &str, onek: &str, uzanti: &str| {
        fs::read_dir(kok.join(alt))
            .map(|liste| {
                liste
                    .filter_map(Result::ok)
                    .filter(|giris| {
                        let ad = giris.file_name().to_string_lossy().to_string();
                        ad.starts_with(onek) && ad.ends_with(uzanti)
                    })
                    .count()
            })
            .unwrap_or(0)
    };
    (
        say("video", "hero_", ".mp4"),
        say("photos", "img_", ".webp"),
    )
}

/// Hiç video (`hero_*.mp4`) veya fotoğraf (`img_*.webp`) bulunmayan sektörler.
pub(crate) fn bos_sektorler(arsiv: &Path) -> Vec<String> {
    crate::SEKTORLER
        .iter()
        .filter(|tanim| {
            let (video, foto) = mevcut_sayilar(&arsiv.join(crate::sektor_medya::slug(tanim.ad)));
            video == 0 && foto == 0
        })
        .map(|tanim| tanim.ad.to_string())
        .collect()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dusuk_cozunurluk_reddedilir() {
        assert!(kaynak_kaliteli(1920, 1080, true));
        assert!(kaynak_kaliteli(1280, 720, true));
        assert!(!kaynak_kaliteli(854, 480, true));
        assert!(kaynak_kaliteli(1920, 1080, false));
        assert!(!kaynak_kaliteli(1600, 1000, false));
    }
}
