//! Sektör medyası yardımcıları.
//!
//! Otomatik tarama ve indirme hattı kaldırıldı. Bu modül yalnız manuel panelin
//! ürettiği `media/stock/<slug>/` arşivini demo motoruna okur: sektör slug'ı,
//! İngilizce grup karşılığı, onaylı video ve fotoğraflar.

use std::fs;
use std::path::Path;

/// Onaylanan medyanın kök klasörü.
pub const KLASOR: &str = "media/stock";
/// Arşiv kayıt defteri (kök dizinde).
pub const MANIFEST: &str = "media_manifest.json";
/// Demo sayfasına en fazla bu kadar fotoğraf girer.
pub const HEDEF_FOTO: usize = 2;
/// Demo motoruna gömülecek fotoğrafların kaynak notu.
pub const ATIF: &str = "Sektör medya kütüphanesi";

/// Türkçe adı klasör adına çevirir: `Oto Yıkama & Bakım` → `oto_yikama_bakim`.
pub fn slug(ad: &str) -> String {
    let mut cikti = String::new();
    for harf in ad.chars() {
        let k = match harf {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'İ' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            _ => harf,
        };
        if k.is_ascii_alphanumeric() {
            cikti.push(k.to_ascii_lowercase());
        } else if !cikti.ends_with('_') {
            cikti.push('_');
        }
    }
    cikti.trim_matches('_').to_string()
}

/// Güncel 11 sektör grubunun İngilizce arama karşılığı.
pub fn ingilizce(grup: &str) -> &'static str {
    match grup {
        "Lokanta & Kebap" => "restaurant",
        "Kafe & Çay" => "cafe coffee shop",
        "Oto Servis" => "car repair garage",
        "Güzellik & Bakım" => "beauty salon",
        "Tesisat & Güvenlik" => "plumber pipes",
        "Lojistik & Seyahat" => "logistics warehouse",
        "Sağlık Kuruluşları" => "medical clinic",
        "Yapı & Emlak" => "real estate office",
        "Hukuk & Mali" => "law office",
        "Mekân & Etkinlik" => "event venue",
        "Temizlik & Bakım" => "cleaning service",
        _ => "business",
    }
}

/// Manifesti okur; dosya yoksa boş bir iskelet döner.
fn manifest_oku_yoldan(kok: &Path) -> serde_json::Value {
    fs::read_to_string(kok.join(MANIFEST))
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_else(|| serde_json::json!({"surum": 1, "sektorler": {}}))
}

/// Onaylı video kaydı.
pub struct StokVideo {
    /// Sektör klasörüne göreli yol (`video/hero.mp4`).
    pub dosya: String,
    pub poster: String,
    /// Sayfada altbilgiye yazılacak atıf (lisans + yazar).
    pub atif: Option<String>,
}

/// Sektörün onaylı videosu (dosya diskte varsa).
fn video_yoldan(kok: &Path, slug: &str) -> Option<StokVideo> {
    let deger = manifest_oku_yoldan(kok);
    let kayit = deger.pointer(&format!("/sektorler/{slug}/video"))?;
    let dosya = kayit.get("dosya")?.as_str()?.to_string();
    let poster = kayit.get("poster")?.as_str()?.to_string();
    if !kok.join(KLASOR).join(slug).join(&dosya).exists() {
        return None;
    }
    let lisans = kayit.get("lisans").and_then(|v| v.as_str()).unwrap_or("");
    let yazar = kayit.get("yazar").and_then(|v| v.as_str()).unwrap_or("");
    let atif = if lisans.is_empty() {
        None
    } else if yazar.is_empty() {
        Some(lisans.to_string())
    } else {
        Some(format!("{yazar} · {lisans}"))
    };
    Some(StokVideo {
        dosya,
        poster,
        atif,
    })
}

/// Üretimde kullanılacak onaylı video (`media/stock` kökü).
pub fn video(slug: &str) -> Option<StokVideo> {
    video_yoldan(Path::new("."), slug)
}

/// Sektörün onaylı fotoğrafları: (mime, base64) çiftleri.
fn fotolar_yoldan(kok: &Path, slug: &str) -> Vec<(String, String)> {
    let deger = manifest_oku_yoldan(kok);
    let Some(liste) = deger
        .pointer(&format!("/sektorler/{slug}/fotolar"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    let mut cikti = Vec::new();
    for kayit in liste {
        let Some(dosya) = kayit.get("dosya").and_then(|v| v.as_str()) else {
            continue;
        };
        let yol = kok.join(KLASOR).join(slug).join(dosya);
        let Ok(veri) = fs::read(&yol) else {
            continue;
        };
        if veri.len() < 512 {
            continue;
        }
        cikti.push(("image/jpeg".to_string(), base64_kodla(&veri)));
        if cikti.len() >= HEDEF_FOTO {
            break;
        }
    }
    cikti
}

/// Demo üretiminde kullanılacak onaylı fotoğraflar (`media/stock` kökü).
pub fn fotolar(slug: &str) -> Vec<(String, String)> {
    fotolar_yoldan(Path::new("."), slug)
}

/// Yükleme sırasında kullanılan standart base64.
fn base64_kodla(veri: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(veri)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn slug_turkce_harfleri_sadelesir() {
        assert_eq!(slug("Oto Servis"), "oto_servis");
        assert_eq!(slug("Kuaför & Güzellik"), "kuafor_guzellik");
        assert_eq!(slug("Çiçekçi"), "cicekci");
        assert_eq!(slug("İnşaat Ustaları"), "insaat_ustalari");
        assert_eq!(slug("  Ağaç  "), "agac");
    }

    #[test]
    fn her_grup_ingilizce_karsilik_bulur() {
        for tanim in crate::SEKTORLER {
            let kelime = ingilizce(tanim.grup);
            assert!(!kelime.is_empty(), "boş: {}", tanim.grup);
            assert_ne!(kelime, "business", "eşleme yok: {}", tanim.grup);
        }
    }
}
