//! Masaüstü paneli ile SEO sunucusu arasındaki veri köprüsü.
//!
//! Panel onaylanan işletmeleri, sektör/il/ilçe bilgisini ve seçili medyayı
//! `web/veri.json` dosyasına yazar. `sunucu` ikilisi bu dosyayı her istekte
//! okuduğu için medya eklendiği/silindiği anda yayına yansır; ayrı veritabanı,
//! mesaj kuyruğu veya yeniden derleme gerekmez.
//!
//! Dosya atomik yazılır (`.gecici/web_veri.tmp` → `web/veri.json`) ve içerik
//! değişmediyse yeniden yazılmaz: arayüz her karede çağırsa da disk yorulmaz.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::BusinessLead;

/// Son yazılan içeriğin parmak izi (aynı veriyi tekrar yazmamak için).
static SON_IMZA: AtomicU64 = AtomicU64::new(0);
/// Aynı anda iki karenin dosyaya yazmasını engeller.
static YAZMA_KILIDI: Mutex<()> = Mutex::new(());

/// Veri dosyasının köke göre yolu.
pub const VERI_YOLU: &str = "web/veri.json";

/// Medya uzantısı videoyu mu gösteriyor?
fn video(yol: &str) -> bool {
    let uzanti = Path::new(yol)
        .extension()
        .map(|uzanti| uzanti.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    matches!(uzanti.as_str(), "mp4" | "mov" | "webm" | "m4v")
}

/// Türkçe karakterleri ASCII'ye indiren URL slug'ı.
/// "Manisa Kebap Salonu" → "manisa-kebap-salonu".
pub fn slug_uret(ad: &str, ilce: &str) -> String {
    let mut cikti = String::new();
    for karakter in format!("{ad} {ilce}").chars() {
        let kucuk = karakter.to_lowercase().next().unwrap_or(karakter);
        let harf = match kucuk {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'i' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            ' ' | '_' | '/' | '\\' | '.' | ',' | '\'' | '"' => '-',
            _ => kucuk,
        };
        if harf.is_ascii_alphanumeric() || harf == '-' {
            cikti.push(harf);
        }
    }
    let temiz = cikti
        .split('-')
        .filter(|parca| !parca.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if temiz.is_empty() {
        "isletme".to_string()
    } else {
        temiz.chars().take(70).collect()
    }
}

/// Yayınlanacak işletmeleri JSON'a çevirir.
///
/// Kural: demoda seçili (`selected`) ve en az bir medyası olan işletmeler
/// yayınlanır; medyası boş kayıtlar arşivde kalır, site üretilmez.
pub fn json_uret(leads: &[BusinessLead], il: &str, ilce: &str) -> String {
    let mut siteler = Vec::new();
    let mut kullanilan: Vec<String> = Vec::new();
    for lead in leads {
        if !lead.selected {
            continue;
        }
        // Otomatik bağlam: kullanıcı medya seçmediyse sektör arşivi
        // (`media/stock/<sektor>/`) kendiliğinden bağlanır.
        let baglam = crate::seo_motoru::baglam(lead, il, ilce);
        let yollar: Vec<String> = if lead.medya.is_empty() {
            crate::seo_motoru::stok_medya(&baglam.sektor_slug)
        } else {
            lead.medya.clone()
        };
        let medya: Vec<serde_json::Value> = yollar
            .iter()
            .filter(|yol| !yol.trim().is_empty())
            .map(|yol| {
                serde_json::json!({
                    "yol": yol.replace('\\', "/"),
                    "tur": if video(yol) { "video" } else { "foto" },
                    "kapak": "",
                })
            })
            .collect();
        if medya.is_empty() {
            continue;
        }
        let mut slug = slug_uret(&lead.name, ilce);
        // Aynı ad iki ilçede olabilir: çakışmayı sayıyla çöz.
        if kullanilan.contains(&slug) {
            let mut sira = 2;
            loop {
                let aday = format!("{slug}-{sira}");
                if !kullanilan.contains(&aday) {
                    slug = aday;
                    break;
                }
                sira += 1;
            }
        }
        kullanilan.push(slug.clone());
        siteler.push(serde_json::json!({
            "slug": slug,
            "ad": lead.name,
            "sektor": lead.category,
            "sektor_slug": baglam.sektor_slug,
            "il": il,
            "ilce": ilce,
            "telefon": lead.phone,
            "adres": lead.address,
            "baslik": crate::seo_motoru::baslik(&lead.name, &baglam),
            "aciklama": crate::seo_motoru::aciklama(&lead.name, &baglam),
            "sema": crate::seo_motoru::sema_tipi(&baglam.sektor_slug),
            "medya": medya,
        }));
    }
    serde_json::json!({
        "surum": 1,
        "guncelleme": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "kaynak": "ollama_proje.exe",
        "siteler": siteler,
    })
    .to_string()
}

/// JSON'u `web/veri.json` dosyasına yazar; içerik aynıysa dokunmaz.
/// Dönen değer: gerçekten yazıldı mı?
pub fn disa_aktar(leads: &[BusinessLead], il: &str, ilce: &str) -> Result<bool, String> {
    let json = json_uret(leads, il, ilce);
    let imza = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        json.hash(&mut hasher);
        hasher.finish()
    };
    if SON_IMZA.load(Ordering::Relaxed) == imza && Path::new(VERI_YOLU).is_file() {
        return Ok(false);
    }
    let _kilit = YAZMA_KILIDI.lock().map_err(|_| "kilit".to_string())?;
    if let Some(klasor) = Path::new(VERI_YOLU).parent() {
        std::fs::create_dir_all(klasor).map_err(|hata| format!("web klasörü: {hata}"))?;
    }
    std::fs::create_dir_all(".gecici").ok();
    let gecici = ".gecici/web_veri.tmp";
    std::fs::write(gecici, &json).map_err(|hata| format!("geçici dosya: {hata}"))?;
    std::fs::rename(gecici, VERI_YOLU).map_err(|hata| format!("veri.json: {hata}"))?;
    SON_IMZA.store(imza, Ordering::Relaxed);
    Ok(true)
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

    #[test]
    fn slug_turkce_karakterleri_cevirir() {
        assert_eq!(
            slug_uret("Manisa Kebap Salonu", "Şehzadeler"),
            "manisa-kebap-salonu-sehzadeler"
        );
        assert_eq!(
            slug_uret("Çınar Oto Çekici", "Yunusemre"),
            "cinar-oto-cekici-yunusemre"
        );
        assert_eq!(slug_uret("!!!", ""), "isletme");
        assert!(slug_uret("A", "").chars().count() <= 70);
    }

    #[test]
    fn yalniz_secili_ve_medyali_isletmeler_yayinlanir() {
        let leads = vec![
            lead(
                "Manisa Kebap",
                "Restoran",
                &["media/stock/restoran/a.jpg"],
                true,
            ),
            // Arşivi olmayan sektör: otomatik bağlama devreye girmez.
            lead("Medyasiz", "ArsivsizSektor", &[], true),
            lead(
                "Secilmemis",
                "Restoran",
                &["media/stock/restoran/b.jpg"],
                false,
            ),
        ];
        let json: serde_json::Value =
            serde_json::from_str(&json_uret(&leads, "Manisa", "Şehzadeler")).unwrap();
        let siteler = json["siteler"].as_array().unwrap();
        assert_eq!(siteler.len(), 1);
        assert_eq!(siteler[0]["ad"], "Manisa Kebap");
        assert_eq!(siteler[0]["sektor_slug"], "restoran");
        assert_eq!(siteler[0]["il"], "Manisa");
        assert_eq!(siteler[0]["medya"][0]["tur"], "foto");
        // Otomatik meta optimizasyonu: 60/160 sınırları ve şema tipi.
        assert!(
            siteler[0]["baslik"].as_str().unwrap().chars().count() <= 60,
            "başlık 60 karakteri aşıyor"
        );
        assert!(
            siteler[0]["aciklama"].as_str().unwrap().chars().count() <= 160,
            "açıklama 160 karakteri aşıyor"
        );
        assert_eq!(siteler[0]["sema"], "Restaurant");
    }

    #[test]
    fn video_ve_foto_turleri_ayrilir() {
        let leads = vec![lead(
            "Kebap",
            "Restoran",
            &[
                "media/stock/restoran/hero.mp4",
                "media/stock/restoran/1.jpg",
            ],
            true,
        )];
        let json: serde_json::Value =
            serde_json::from_str(&json_uret(&leads, "Manisa", "Merkez")).unwrap();
        assert_eq!(json["siteler"][0]["medya"][0]["tur"], "video");
        assert_eq!(json["siteler"][0]["medya"][1]["tur"], "foto");
    }

    #[test]
    fn ayni_slug_cakismasi_numaralanir() {
        let leads = vec![
            lead("Aynı Ad", "Restoran", &["media/stock/restoran/a.jpg"], true),
            lead("Aynı Ad", "Restoran", &["media/stock/restoran/b.jpg"], true),
        ];
        let json: serde_json::Value =
            serde_json::from_str(&json_uret(&leads, "Manisa", "Merkez")).unwrap();
        let siteler = json["siteler"].as_array().unwrap();
        assert_ne!(siteler[0]["slug"], siteler[1]["slug"]);
    }
}
