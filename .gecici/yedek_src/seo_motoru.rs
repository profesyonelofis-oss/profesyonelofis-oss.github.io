//! Otomatik SEO motoru.
//!
//! Arayüzde artık medya/demo/yayın adımı yok: bu modül keşifte seçilen
//! işletmenin **bağlamını** (sektör + il/ilçe + hizmet) kurar, `media/stock/
//! <sektor>/` altındaki yerel varlıkları o işletmeye bağlar ve arama motoru
//! sınırlarına uyan meta verisini üretir:
//!
//! - `<title>` en fazla **60**, `meta description` en fazla **160** karakter.
//! - `canonical` yolu slug'dan türetilir (`/s/<slug>`).
//! - Schema.org `@type` sektörden seçilir (Restaurant, AutoRepair, …).
//!
//! Medya seçimi elle yapılmadıysa sektör arşivi otomatik taranır; diskte yeni
//! dosya düştüğü anda bir sonraki çalıştırmada siteye girer.

use std::path::{Path, PathBuf};

use crate::BusinessLead;

/// Varlıkların kökü (proje köküne göre).
pub const STOK_KOK: &str = "media/stock";

/// Bir işletmenin SEO bağlamı.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Baglam {
    pub il: String,
    pub ilce: String,
    pub sektor: String,
    pub sektor_slug: String,
}

impl Baglam {
    /// "Şehzadeler Manisa" — boş parçalar atlanır.
    pub fn yer(&self) -> String {
        [self.ilce.trim(), self.il.trim()]
            .into_iter()
            .filter(|parca| !parca.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// "Restoran" — sektör boşsa "hizmet".
    pub fn hizmet(&self) -> &str {
        if self.sektor.trim().is_empty() {
            "hizmet"
        } else {
            self.sektor.trim()
        }
    }

    /// `/s/<slug>` biçimindeki canonical yolu.
    pub fn canonical(&self, slug: &str) -> String {
        format!("/s/{slug}")
    }
}

/// İşletme + tarama bağlamından SEO bağlamı üretir.
pub fn baglam(lead: &BusinessLead, il: &str, ilce: &str) -> Baglam {
    Baglam {
        il: il.to_string(),
        ilce: ilce.to_string(),
        sektor: lead.category.clone(),
        sektor_slug: crate::sektor_medya::slug(&lead.category),
    }
}

/// Sektör arşivindeki dosyaları (video önce, sonra fotoğraf) döndürür.
/// Yollar proje köküne göredir: `media/stock/restoran/ai_1.jpg`.
pub fn stok_medya(sektor_slug: &str) -> Vec<String> {
    stok_medya_kok(Path::new(STOK_KOK), sektor_slug)
}

/// Test edilebilir sürüm: verilen kök altındaki sektör klasörünü tarar.
pub fn stok_medya_kok(kok: &Path, sektor_slug: &str) -> Vec<String> {
    let slug = sektor_slug.trim();
    if slug.is_empty() || slug.contains("..") || slug.contains(':') || slug.contains('/') {
        return Vec::new();
    }
    let Ok(kayitlar) = std::fs::read_dir(kok.join(slug)) else {
        return Vec::new();
    };
    let mut fotolar: Vec<String> = Vec::new();
    let mut videolar: Vec<String> = Vec::new();
    for kayit in kayitlar.flatten() {
        let yol = kayit.path();
        if !yol.is_file() {
            continue;
        }
        let uzanti = yol
            .extension()
            .map(|uzanti| uzanti.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let goreli = goreli_yol(kok, slug, &yol);
        match uzanti.as_str() {
            "mp4" | "webm" | "mov" | "m4v" => videolar.push(goreli),
            "jpg" | "jpeg" | "png" | "webp" => fotolar.push(goreli),
            _ => {}
        }
    }
    // Deterministik sıra: aynı arşiv her zaman aynı sayfayı üretir.
    fotolar.sort();
    videolar.sort();
    let mut cikti = videolar;
    cikti.extend(fotolar);
    cikti.truncate(6);
    cikti
}

fn goreli_yol(kok: &Path, slug: &str, dosya: &Path) -> String {
    let ad = dosya
        .file_name()
        .map(|ad| ad.to_string_lossy().to_string())
        .unwrap_or_default();
    let taban = kok.to_string_lossy().replace('\\', "/");
    let taban = taban.trim_start_matches("./").trim_end_matches('/');
    format!("{taban}/{slug}/{ad}")
}

/// Arama motoru sınırına sığdırır: sınırı aşarsa `…` ile keser.
fn kirp(metin: &str, sinir: usize) -> String {
    if metin.chars().count() <= sinir {
        return metin.to_string();
    }
    let mut kisa: String = metin.chars().take(sinir.saturating_sub(1)).collect();
    kisa.push('…');
    kisa
}

/// `<title>`: en fazla 60 karakter. Uzunsa yer, sonra sektör düşer.
pub fn baslik(ad: &str, baglam: &Baglam) -> String {
    let ad = if ad.trim().is_empty() {
        "İşletme"
    } else {
        ad.trim()
    };
    let yer = baglam.yer();
    let hizmet = baglam.hizmet().to_lowercase();
    let adaylar = [
        format!("{ad} · {yer} | {hizmet}"),
        format!("{ad} · {yer}"),
        format!("{ad} | {hizmet}"),
        ad.to_string(),
    ];
    for aday in adaylar {
        if aday.chars().count() <= 60 {
            return aday;
        }
    }
    kirp(ad, 60)
}

/// `meta description`: en fazla 160 karakter, çağrı odaklı tek cümle.
pub fn aciklama(ad: &str, baglam: &Baglam) -> String {
    let ad = if ad.trim().is_empty() {
        "İşletme"
    } else {
        ad.trim()
    };
    let yer = baglam.yer();
    let hizmet = baglam.hizmet().to_lowercase();
    let adaylar = [
        format!("{ad}, {yer} adresinde {hizmet} hizmeti. Telefon, konum ve fotoğraflar bu sayfada; hemen arayın."),
        format!("{ad} · {hizmet} · {yer}. Telefon, konum ve fotoğraflar tek sayfada."),
        format!("{ad} · {hizmet} · {yer}."),
    ];
    for aday in adaylar {
        if aday.chars().count() <= 160 {
            return aday;
        }
    }
    kirp(ad, 160)
}

/// Sektöre göre Schema.org `@type`.
pub fn sema_tipi(sektor_slug: &str) -> &'static str {
    match sektor_slug.to_lowercase().as_str() {
        "restoran" | "kebap" | "kafe" | "lokanta" | "pastane" => "Restaurant",
        "cekici" | "oto" | "tamirci" | "auto" | "servis" => "AutoRepair",
        "kuafor" | "berber" | "guzellik" => "HairSalon",
        "market" | "bakkal" => "Store",
        "otel" | "pansiyon" => "LodgingBusiness",
        "emlak" => "RealEstateAgent",
        "dis" | "klinik" | "doktor" => "MedicalBusiness",
        _ => "LocalBusiness",
    }
}

/// Motorun tek çalıştırmada yaptığı işin özeti (arayüz günlüğü için).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ozet {
    /// Yayına hazır işletme sayısı.
    pub siteler: usize,
    /// Otomatik bağlanan medya dosyası sayısı.
    pub medya: usize,
    /// Meta/şema üretilen işletme sayısı.
    pub etiket: usize,
}

/// Motoru işletme listesi üzerinde çalıştırır: arşivde medyası olan her
/// işletmeyi seçer ve boş medya alanını sektör arşiviyle doldurur.
/// Kullanıcı elle medya seçtiyse o seçim korunur.
pub fn uygula(leads: &mut [BusinessLead], il: &str, ilce: &str) -> Ozet {
    let mut ozet = Ozet::default();
    for lead in leads.iter_mut() {
        let baglam = baglam(lead, il, ilce);
        if lead.medya.is_empty() {
            let stok = stok_medya(&baglam.sektor_slug);
            if stok.is_empty() {
                continue;
            }
            ozet.medya += stok.len();
            lead.medya = stok;
        }
        lead.selected = true;
        ozet.siteler += 1;
        ozet.etiket += 1;
    }
    ozet
}

/// Medya dosyasının uzantısı video mu?
pub fn video(yol: &str) -> bool {
    PathBuf::from(yol)
        .extension()
        .map(|uzanti| {
            matches!(
                uzanti.to_string_lossy().to_lowercase().as_str(),
                "mp4" | "webm" | "mov" | "m4v"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn lead(ad: &str, kategori: &str, medya: &[&str], selected: bool) -> BusinessLead {
        BusinessLead {
            name: ad.to_string(),
            address: "Cumhuriyet Mah. No:3".to_string(),
            category: kategori.to_string(),
            phone: "0555 111 22 33".to_string(),
            selected,
            kaynak: "osm".to_string(),
            konum: None,
            calisma_saatleri: String::new(),
            demo_yolu: None,
            mesaj_hazir: false,
            kayit_dolulugu: 0,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: None,
            yildiz_puani: None,
            durum: crate::musteriler::Durum::Yeni,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: medya.iter().map(|yol| yol.to_string()).collect(),
        }
    }

    fn gecici_kok(ad: &str) -> PathBuf {
        let kok = std::env::temp_dir().join(format!("nexus_seo_{ad}"));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(kok.join("restoran")).unwrap();
        kok
    }

    #[test]
    fn baslik_60_karakteri_asmaz() {
        let b = Baglam {
            il: "Manisa".into(),
            ilce: "Şehzadeler".into(),
            sektor: "Restoran".into(),
            sektor_slug: "restoran".into(),
        };
        let kisa = baslik("Manisa Kebap Salonu", &b);
        assert!(kisa.chars().count() <= 60, "{kisa}");

        let uzun = baslik(&"Çok Uzun İşletme Adı ".repeat(6), &b);
        assert!(uzun.chars().count() <= 60, "{uzun}");
        assert!(uzun.ends_with('…'));
    }

    #[test]
    fn aciklama_160_karakteri_asmaz() {
        let b = Baglam {
            il: "Manisa".into(),
            ilce: "Yunusemre".into(),
            sektor: "Oto Çekici".into(),
            sektor_slug: "cekici".into(),
        };
        let metin = aciklama("Çınar Oto Çekici", &b);
        assert!(metin.chars().count() <= 160, "{metin}");
        assert!(metin.contains("Manisa"));

        let uzun = aciklama(&"Uzun Ad ".repeat(30), &b);
        assert!(uzun.chars().count() <= 160);
    }

    #[test]
    fn sema_tipi_sektore_gore_secilir() {
        assert_eq!(sema_tipi("restoran"), "Restaurant");
        assert_eq!(sema_tipi("cekici"), "AutoRepair");
        assert_eq!(sema_tipi("bilinmeyen"), "LocalBusiness");
    }

    #[test]
    fn stok_medya_videoyu_one_alir_ve_sinirlar() {
        let kok = gecici_kok("medya");
        std::fs::write(kok.join("restoran/b.jpg"), b"x").unwrap();
        std::fs::write(kok.join("restoran/a.jpg"), b"x").unwrap();
        std::fs::write(kok.join("restoran/hero.mp4"), b"x").unwrap();
        std::fs::write(kok.join("restoran/not.txt"), b"x").unwrap();
        let medya = stok_medya_kok(&kok, "restoran");
        assert_eq!(medya.len(), 3);
        assert!(medya[0].ends_with("hero.mp4"));
        assert!(medya[1].ends_with("a.jpg"));
        assert!(stok_medya_kok(&kok, "").is_empty());
        assert!(stok_medya_kok(&kok, "../gizli").is_empty());
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn uygula_bos_medyayi_sektor_arsivinden_doldurur() {
        let kok = gecici_kok("uygula");
        std::fs::write(kok.join("restoran/a.jpg"), b"x").unwrap();
        let mut leads = vec![
            lead("Kebap", "Restoran", &[], false),
            lead("Eldeki", "Restoran", &["media/stock/restoran/b.jpg"], false),
            lead("BosSektor", "Berber", &[], false),
        ];
        // Gerçek arşiv yolu kullanılmadığı için yalnız medya kuralı sınanır.
        let ozet = uygula(&mut leads, "Manisa", "Merkez");
        assert_eq!(ozet.siteler, 2);
        assert!(leads[0].selected);
        assert_eq!(leads[1].medya, vec!["media/stock/restoran/b.jpg"]);
        assert!(!leads[2].selected);
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn canonical_yolu_slug_ile_kurulur() {
        let b = Baglam {
            il: "Manisa".into(),
            ilce: "Merkez".into(),
            sektor: "Restoran".into(),
            sektor_slug: "restoran".into(),
        };
        assert_eq!(b.canonical("manisa-kebap"), "/s/manisa-kebap");
        assert_eq!(b.yer(), "Merkez Manisa");
    }
}
