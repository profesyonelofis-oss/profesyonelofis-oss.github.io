#![allow(dead_code)]
//! Ücretsiz stok video kaynaklarından (Pexels, yedek: Pixabay) sinematik hero
//! videosu bulan, indiren ve sahne için hazırlayan modül.
//!
//! Kurallar:
//! - Yalnızca resmî API'ler kullanılır; web sitesi kazınmaz.
//! - API anahtarı yalnızca `stok_kasa` (AES-256-GCM) üzerinden okunur.
//! - Dışarıya yalnızca arama kelimesi çıkar; işletme verisi isteğe girmez.
//! - Kendi istek sayacı tutulur: Pexels 200/saat + 20.000/ay, Pixabay
//!   100/60 sn + 5.000/saat (API belgelerindeki ücretsiz limitler).
//! - İndirilen her video için `KAYNAK.json` yazılır (kaynak, sahip, bağlantı,
//!   lisans). Böylece atıf ve lisans izi sayfadan bağımsız kalır.

/// Windows'ta alt süreçler için ayrı konsol penceresi açılmasını engeller.
pub(crate) fn gizli_komut(program: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut komut = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        komut.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    komut
}

use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Sektör videolarının kök klasörü: `media/sektor/<grup>/hero.mp4`.
pub const MEDYA_KOK: &str = "media/sektor";
/// İstek sayacı dosyası.
pub const KOTA_DOSYASI: &str = "stok_kota.json";
/// Dönüştürülen videonun süresi (saniye).
pub const VIDEO_SN: f64 = 8.0;
/// İndirilen ham dosyanın üst sınırı (bozuk/yanlış adres kotayı yakmasın).
const INDIRME_SINIRI: u64 = 180 * 1024 * 1024;

// --- Ücretsiz API limitleri (belgelerden) ---------------------------------
/// Pexels: 200 istek/saat.
pub const PEXELS_SAAT: usize = 200;
/// Pexels: 20.000 istek/ay.
pub const PEXELS_AY: usize = 20_000;
/// Pixabay: 100 istek/60 saniye.
pub const PIXABAY_DAKIKA: usize = 100;
/// Pixabay: 5.000 istek/saat.
pub const PIXABAY_SAAT: usize = 5_000;

/// Bir sektör grubu: panelde seçilir, arama kelimeleri önerilir.
pub struct Grup {
    /// Klasör adı: `media/sektor/<id>/`.
    pub id: &'static str,
    /// Panelde görünen ad.
    pub ad: &'static str,
    /// Üç İngilizce arama kelimesi (kullanıcı panelde değiştirebilir).
    pub kelimeler: [&'static str; 3],
    /// İşletme kategorisinde geçen Türkçe/İngilizce ipuçları.
    pub etiketler: &'static [&'static str],
}

/// Sektör grupları: her biri kendi İngilizce arama kelimeleriyle gelir.
pub const GRUPLAR: &[Grup] = &[
    Grup {
        id: "kuafor",
        ad: "Kuaför / berber",
        kelimeler: [
            "hair salon cinematic",
            "barber slow motion",
            "hairdresser close up",
        ],
        etiketler: &["kuaför", "kuafor", "berber", "saç", "sac", "hair", "barber"],
    },
    Grup {
        id: "restoran",
        ad: "Restoran / kafe",
        kelimeler: [
            "restaurant cinematic night",
            "chef plating slow motion",
            "coffee shop atmosphere",
        ],
        etiketler: &[
            "restoran",
            "kafe",
            "cafe",
            "lokanta",
            "mutfak",
            "restaurant",
            "coffee",
            "bistro",
            "bar",
        ],
    },
    Grup {
        id: "oto",
        ad: "Oto servis",
        kelimeler: [
            "car workshop cinematic",
            "mechanic working close up",
            "garage lights slow motion",
        ],
        etiketler: &[
            "oto", "servis", "tamir", "lastik", "araba", "car", "mechanic", "garage",
        ],
    },
    Grup {
        id: "klinik",
        ad: "Klinik / diş",
        kelimeler: [
            "dental clinic cinematic",
            "dentist close up light",
            "medical interior slow motion",
        ],
        etiketler: &[
            "klinik", "diş", "dis", "doktor", "sağlık", "saglik", "clinic", "dental", "medical",
        ],
    },
    Grup {
        id: "otel",
        ad: "Otel",
        kelimeler: [
            "boutique hotel cinematic",
            "hotel lobby slow motion",
            "morning light hotel room",
        ],
        etiketler: &["otel", "pansiyon", "hotel", "konaklama", "resort", "butik"],
    },
    Grup {
        id: "spor",
        ad: "Spor salonu",
        kelimeler: [
            "gym cinematic dark",
            "fitness training slow motion",
            "weights close up light",
        ],
        etiketler: &[
            "spor",
            "fitness",
            "gym",
            "antrenman",
            "pilates",
            "yoga",
            "crossfit",
        ],
    },
];

/// İşletme kategorisinden sektör grubu: eşleşme yoksa `None` (shader sürümü).
pub fn grup_esle(sektor: &str) -> Option<&'static Grup> {
    let kucuk = sektor.to_lowercase();
    GRUPLAR
        .iter()
        .find(|grup| grup.etiketler.iter().any(|etiket| kucuk.contains(etiket)))
}

/// Arama sonucu tek aday. Panelde önizleme, süre ve çekim sahibiyle listelenir.
#[derive(Clone, Debug)]
pub struct Aday {
    /// "pexels" veya "pixabay".
    pub saglayici: &'static str,
    pub id: String,
    /// Çekim sahibi (API'nin verdiği ad).
    pub sahip: String,
    /// Çekim sahibinin profil bağlantısı.
    pub sahip_linki: String,
    /// Videonun servis sayfası (lisans/kaynak izi).
    pub sayfa_linki: String,
    /// Doğrudan indirme adresi (mp4).
    pub indirme: String,
    /// Önizleme karesi (jpg).
    pub onizleme: String,
    pub genislik: u32,
    pub yukseklik: u32,
    pub sure: f64,
    pub lisans: String,
    pub lisans_url: String,
}

impl Aday {
    /// Panelde tek satır özet: `Pexels · 1920×1080 · 12 sn · sahip`.
    pub fn ozet(&self) -> String {
        format!(
            "{} · {}×{} · {:.0} sn · {}",
            self.saglayici_baslik(),
            self.genislik,
            self.yukseklik,
            self.sure,
            if self.sahip.trim().is_empty() {
                "bilinmeyen"
            } else {
                self.sahip.trim()
            }
        )
    }

    pub fn saglayici_baslik(&self) -> &'static str {
        if self.saglayici == "pexels" {
            "Pexels"
        } else {
            "Pixabay"
        }
    }
}

/// Hazırlanan video: dosyalar `media/sektor/<grup>/` altına yazıldı.
#[derive(Clone, Debug)]
pub struct Hazir {
    pub grup: String,
    /// Klasör içindeki dosya adı (`v2.mp4`).
    pub dosya_adi: String,
    /// Klasör içindeki poster adı (`v2.webp`).
    pub poster_adi: String,
    pub video_bayt: u64,
    pub poster_bayt: u64,
    pub atif: String,
    pub sure: f64,
}

/// Kaydedilmiş sektör videosunun künyesi (KAYNAK.json + dosyalar).
#[derive(Clone, Debug)]
pub struct VideoBilgi {
    pub grup: String,
    /// Klasör içindeki dosya adı (`v2.mp4`).
    pub dosya_adi: String,
    /// Klasör içindeki poster adı (`v2.webp`).
    pub poster_adi: String,
    /// Sayfaya göreli yol: `media/hero.mp4`.
    pub dosya: String,
    /// Sayfaya göreli poster: `media/poster.webp`.
    pub poster: String,
    pub saglayici: String,
    pub sahip: String,
    pub kaynak_sayfasi: String,
    pub lisans: String,
    pub atif: String,
    pub atif_gerekli: bool,
    pub sure: f64,
    pub genislik: u32,
    pub yukseklik: u32,
    pub video_bayt: u64,
    pub poster_bayt: u64,
}

impl VideoBilgi {
    /// Panel satırı: `Kuaför · 3,4 MB video · 96 KB poster · Pexels`.
    pub fn ozet(&self) -> String {
        format!(
            "{} · {:.1} MB video · {:.0} KB poster · {} · {}×{} · kaynak {:.0} sn",
            self.grup,
            self.video_bayt as f64 / 1_048_576.0,
            self.poster_bayt as f64 / 1024.0,
            self.saglayici,
            self.genislik,
            self.yukseklik,
            self.sure
        )
    }

    /// Panelde üzerine gelince görünen künye: lisans ve kaynak bağlantısı.
    pub fn kunye(&self) -> String {
        format!(
            "Çekim: {}\nLisans: {}\nKaynak: {}",
            if self.sahip.trim().is_empty() {
                "bilinmiyor"
            } else {
                self.sahip.trim()
            },
            self.lisans,
            self.kaynak_sayfasi
        )
    }

    /// Sayfa altbilgisine eklenecek atıf; lisans atıf istemiyorsa `None`.
    pub fn sayfa_atfi(&self) -> Option<String> {
        if self.atif_gerekli && !self.atif.trim().is_empty() {
            Some(self.atif.clone())
        } else {
            None
        }
    }
}

fn simdi() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|s| s.as_secs())
        .unwrap_or(0)
}

fn url_kodla(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len() * 3);
    for &bayt in metin.as_bytes() {
        match bayt {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                cikti.push(bayt as char)
            }
            _ => cikti.push_str(&format!("%{bayt:02X}")),
        }
    }
    cikti
}

fn ajan() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent("NexusEnterprise/0.1 (stok video tarayicisi)")
        .timeout_connect(Duration::from_secs(12))
        .timeout(Duration::from_secs(60))
        .build()
}

/// ffmpeg kurulu mu? Panelde ve hazırlamada kullanılır.
pub fn ffmpeg_var() -> bool {
    gizli_komut("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|durum| durum.success())
        .unwrap_or(false)
}

// --- İstek sayacı ---------------------------------------------------------

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct KotaDefteri {
    #[serde(default)]
    pexels: Vec<u64>,
    #[serde(default)]
    pixabay: Vec<u64>,
}

fn kota_oku(yol: &str) -> KotaDefteri {
    std::fs::read_to_string(yol)
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_default()
}

fn kota_yaz(yol: &str, defter: &KotaDefteri) {
    if let Ok(metin) = serde_json::to_string(defter) {
        let _ = std::fs::write(yol, metin);
    }
}

/// Sağlayıcının sınırı doldu mu? Dolduysa Türkçe, bekleme süreli hata döner.
pub fn kota_kontrol(saglayici: &str) -> Result<(), String> {
    kota_kontrol_yoldan(saglayici, KOTA_DOSYASI, simdi())
}

fn kota_kontrol_yoldan(saglayici: &str, yol: &str, an: u64) -> Result<(), String> {
    let defter = kota_oku(yol);
    let (kayitlar, pencere, sinir, ad) = match saglayici {
        "pexels" => (&defter.pexels, 3600_u64, PEXELS_SAAT, "Pexels saatlik"),
        _ => (&defter.pixabay, 60_u64, PIXABAY_DAKIKA, "Pixabay dakikalık"),
    };
    let sayi = kayitlar
        .iter()
        .filter(|&&t| an.saturating_sub(t) < pencere)
        .count();
    if sayi >= sinir {
        let en_eski = kayitlar
            .iter()
            .filter(|&&t| an.saturating_sub(t) < pencere)
            .min()
            .copied()
            .unwrap_or(an);
        let kalan = pencere.saturating_sub(an.saturating_sub(en_eski)).max(1);
        return Err(format!(
            "{ad} sınırı doldu ({sinir}/{sinir}); yaklaşık {} dk sonra yeniden deneyin.",
            kalan.div_ceil(60)
        ));
    }
    // Aylık (Pexels) ve saatlik (Pixabay) üst sınırlar.
    match saglayici {
        "pexels" => {
            let ay = defter
                .pexels
                .iter()
                .filter(|&&t| an.saturating_sub(t) < 30 * 86_400)
                .count();
            if ay >= PEXELS_AY {
                return Err(format!(
                    "Pexels aylık ücretsiz sınırı doldu ({PEXELS_AY} istek). Gelecek ay yenilenir."
                ));
            }
        }
        _ => {
            let pix_saat = defter
                .pixabay
                .iter()
                .filter(|&&t| an.saturating_sub(t) < 3600)
                .count();
            if pix_saat >= PIXABAY_SAAT {
                return Err(format!(
                    "Pixabay saatlik ücretsiz sınırı doldu ({PIXABAY_SAAT} istek). Bir saat sonra yeniden deneyin."
                ));
            }
        }
    }
    Ok(())
}

/// İstek gönderildi: sayaç artırılır (başarısız istek de kotadan düşer).
pub fn kota_artir(saglayici: &str) {
    kota_artir_yoldan(saglayici, KOTA_DOSYASI, simdi())
}

fn kota_artir_yoldan(saglayici: &str, yol: &str, an: u64) {
    let mut defter = kota_oku(yol);
    let liste = if saglayici == "pexels" {
        &mut defter.pexels
    } else {
        &mut defter.pixabay
    };
    liste.push(an);
    // 30 günden eski kayıtlar dosyada tutulmaz.
    liste.retain(|&t| an.saturating_sub(t) < 30 * 86_400);
    kota_yaz(yol, &defter);
}

/// Paneldeki kota satırı.
pub fn kota_ozeti() -> String {
    let defter = kota_oku(KOTA_DOSYASI);
    let an = simdi();
    let p_saat = defter
        .pexels
        .iter()
        .filter(|&&t| an.saturating_sub(t) < 3600)
        .count();
    let p_ay = defter
        .pexels
        .iter()
        .filter(|&&t| an.saturating_sub(t) < 30 * 86_400)
        .count();
    let x_dk = defter
        .pixabay
        .iter()
        .filter(|&&t| an.saturating_sub(t) < 60)
        .count();
    format!(
        "Kota · Pexels {p_saat}/{PEXELS_SAAT} saat · {p_ay}/{PEXELS_AY} ay · Pixabay {x_dk}/{PIXABAY_DAKIKA} dk"
    )
}

// --- Arama ----------------------------------------------------------------

/// Kelimeyle aday arar: önce Pexels, anahtar yoksa Pixabay. `sayfa` 1'den başlar.
pub fn ara(kelime: &str, sayfa: usize) -> Result<Vec<Aday>, String> {
    let kelime = kelime.trim();
    if kelime.is_empty() {
        return Err("Arama kelimesi boş; paneldeki önerilerden birini kullanın.".to_string());
    }
    let kasa = crate::stok_kasa::StokKasa::yukle();
    if let Some(anahtar) = kasa.pexels() {
        kota_kontrol("pexels")?;
        kota_artir("pexels");
        return pexels_ara(anahtar, kelime, sayfa);
    }
    if let Some(anahtar) = kasa.pixabay() {
        kota_kontrol("pixabay")?;
        kota_artir("pixabay");
        return pixabay_ara(anahtar, kelime, sayfa);
    }
    Err(
        "API anahtarı yok: Pexels (yedek: Pixabay) anahtarını panelden girip kasaya kaydedin."
            .to_string(),
    )
}

fn pexels_ara(anahtar: &str, kelime: &str, sayfa: usize) -> Result<Vec<Aday>, String> {
    let url = format!(
        "https://api.pexels.com/videos/search?query={}&orientation=landscape&size=large&per_page=15&page={}",
        url_kodla(kelime),
        sayfa.max(1)
    );
    let yanit = ajan().get(&url).set("Authorization", anahtar).call();
    let yanit = match yanit {
        Ok(yanit) => yanit,
        Err(ureq::Error::Status(kod, yanit)) => {
            return Err(format!(
                "Pexels {kod} döndü: {}",
                yanit
                    .into_string()
                    .unwrap_or_default()
                    .chars()
                    .take(120)
                    .collect::<String>()
            ))
        }
        Err(_) => return Err("Pexels isteği gönderilemedi (bağlantı yok).".to_string()),
    };
    let metin = yanit
        .into_string()
        .map_err(|hata| format!("Pexels yanıtı okunamadı: {hata}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&metin).map_err(|hata| format!("Pexels yanıtı çözülemedi: {hata}"))?;
    let mut adaylar = Vec::new();
    let Some(videolar) = json.get("videos").and_then(|v| v.as_array()) else {
        return Ok(adaylar);
    };
    for video in videolar {
        let genislik = video.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let yukseklik = video.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let sure = video
            .get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let indirme = video
            .get("video_files")
            .and_then(|v| v.as_array())
            .and_then(|dosyalar| {
                dosyalar
                    .iter()
                    .filter(|dosya| {
                        dosya.get("file_type").and_then(|v| v.as_str()) == Some("video/mp4")
                    })
                    .filter_map(|dosya| {
                        let y = dosya.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                        let link = dosya.get("link").and_then(|v| v.as_str())?;
                        Some((y, link.to_string()))
                    })
                    .filter(|(y, _)| *y <= 1920)
                    .max_by_key(|(y, _)| *y)
                    .map(|(_, link)| link)
            })
            .unwrap_or_default();
        let onizleme = video
            .get("image")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sahip = video
            .get("user")
            .and_then(|u| u.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sahip_linki = video
            .get("user")
            .and_then(|u| u.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        adaylar.push(Aday {
            saglayici: "pexels",
            id: video
                .get("id")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default(),
            sahip,
            sahip_linki,
            sayfa_linki: video
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            indirme,
            onizleme,
            genislik,
            yukseklik,
            sure,
            lisans: "Pexels Lisansı — ticari kullanım serbest, atıf zorunlu değil".to_string(),
            lisans_url: "https://www.pexels.com/license/".to_string(),
        });
    }
    Ok(filtrele(adaylar))
}

fn pixabay_ara(anahtar: &str, kelime: &str, sayfa: usize) -> Result<Vec<Aday>, String> {
    let url = format!(
        "https://pixabay.com/api/videos/?key={}&q={}&video_type=film&safesearch=true&per_page=20&page={}",
        url_kodla(anahtar),
        url_kodla(kelime),
        sayfa.max(1)
    );
    let yanit = match ajan().get(&url).call() {
        Ok(yanit) => yanit,
        Err(ureq::Error::Status(kod, yanit)) => {
            return Err(format!(
                "Pixabay {kod} döndü: {}",
                yanit
                    .into_string()
                    .unwrap_or_default()
                    .chars()
                    .take(120)
                    .collect::<String>()
            ))
        }
        Err(_) => return Err("Pixabay isteği gönderilemedi (bağlantı yok).".to_string()),
    };
    let metin = yanit
        .into_string()
        .map_err(|hata| format!("Pixabay yanıtı okunamadı: {hata}"))?;
    let json: serde_json::Value = serde_json::from_str(&metin)
        .map_err(|hata| format!("Pixabay yanıtı çözülemedi: {hata}"))?;
    let mut adaylar = Vec::new();
    let Some(hitler) = json.get("hits").and_then(|v| v.as_array()) else {
        return Ok(adaylar);
    };
    for video in hitler {
        let dosya = ["large", "medium", "small"]
            .iter()
            .find_map(|ad| video.get("videos").and_then(|v| v.get(*ad)))
            .filter(|dosya| dosya.get("url").is_some());
        let Some(dosya) = dosya else {
            continue;
        };
        adaylar.push(Aday {
            saglayici: "pixabay",
            id: video
                .get("id")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default(),
            sahip: video
                .get("user")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            sahip_linki: video
                .get("user_id")
                .and_then(|v| v.as_u64())
                .map(|id| format!("https://pixabay.com/users/{id}/"))
                .unwrap_or_default(),
            sayfa_linki: video
                .get("pageURL")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            indirme: dosya
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            onizleme: video
                .get("picture_id")
                .and_then(|v| v.as_str())
                .map(|id| format!("https://i.vimeocdn.com/video/{id}_640x360.jpg"))
                .unwrap_or_default(),
            genislik: dosya.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            yukseklik: dosya.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            sure: video
                .get("duration")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            lisans: "Pixabay İçerik Lisansı — ticari kullanım serbest, atıf zorunlu değil"
                .to_string(),
            lisans_url: "https://pixabay.com/service/license-summary/".to_string(),
        });
    }
    Ok(filtrele(adaylar))
}

/// Filtre: yatay, en az 1080p, 5-20 sn. Yavaş çekim eğilimi: 8-14 sn arası
/// klipler öne alınır (yavaş kamera hareketleri genelde uzun çekimlerdir).
pub fn filtrele(adaylar: Vec<Aday>) -> Vec<Aday> {
    let mut kalan: Vec<Aday> = adaylar
        .into_iter()
        .filter(|aday| {
            aday.genislik >= 1920
                && aday.yukseklik >= 1080
                && aday.genislik > aday.yukseklik
                && aday.sure >= 5.0
                && aday.sure <= 20.0
                && aday.indirme.starts_with("http")
        })
        .collect();
    kalan.sort_by_key(|aday| yavas_puan(aday.sure));
    let mut gorulen = std::collections::HashSet::new();
    kalan.retain(|aday| gorulen.insert(aday.id.clone()));
    kalan.truncate(8);
    kalan
}

/// 10 saniyeye yakınlık: küçük puan önce gelir.
fn yavas_puan(sure: f64) -> u32 {
    ((sure - 10.0).abs() * 10.0) as u32
}

// --- İndirme ve hazırlama -------------------------------------------------

/// JSON'dan metin alanı okur (KAYNAK.json için).
fn metin(json: &serde_json::Value, ad: &str) -> String {
    json.get(ad)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Adayı indirir, 720p all-intra sessiz videoya çevirir, posterini çıkarır.
pub fn hazirla(aday: &Aday, grup_id: &str, kelime: &str) -> Result<Hazir, String> {
    if !ffmpeg_var() {
        return Err(
            "ffmpeg bulunamadı: video dönüştürülemiyor. Kurulum: https://ffmpeg.org/download.html"
                .to_string(),
        );
    }
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    std::fs::create_dir_all(&klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    let gecici = std::env::temp_dir().join(format!(
        "nexus_stok_{}_{}.mp4",
        aday.saglayici,
        aday.id.replace(|k: char| !k.is_ascii_alphanumeric(), "_")
    ));
    indir(&aday.indirme, &gecici)?;
    let video = klasor.join("hero.mp4");
    let poster = klasor.join("poster.webp");

    let durum = gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-i"])
        .arg(&gecici)
        .args([
            "-t",
            &format!("{VIDEO_SN:.0}"),
            "-an",
            // 720p, çift sayı yükseklik; keskin küçültme.
            "-vf",
            "scale=-2:720:flags=lanczos",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-profile:v",
            "high",
            "-pix_fmt",
            "yuv420p",
            // Her kare anahtar kare: kaydırmayla geri-ileri akıcı scrub.
            "-g",
            "1",
            "-keyint_min",
            "1",
            "-sc_threshold",
            "0",
            "-movflags",
            "+faststart",
        ])
        .arg(&video)
        .status()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        let _ = std::fs::remove_file(&gecici);
        return Err("ffmpeg videoyu dönüştüremedi (dosya bozuk olabilir).".to_string());
    }

    let durum = gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-ss", "1", "-i"])
        .arg(&video)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=-2:720",
            "-c:v",
            "libwebp",
            "-quality",
            "82",
        ])
        .arg(&poster)
        .status()
        .map_err(|hata| format!("ffmpeg (poster) çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        let _ = std::fs::remove_file(&gecici);
        return Err("Poster görüntüsü çıkarılamadı.".to_string());
    }
    let _ = std::fs::remove_file(&gecici);

    let video_bayt = std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0);
    let poster_bayt = std::fs::metadata(&poster).map(|m| m.len()).unwrap_or(0);
    let atif = format!("{} / {}", aday.sahip.trim(), aday.saglayici_baslik());
    let bilgi = serde_json::json!({
        "saglayici": aday.saglayici_baslik(),
        "video_id": aday.id,
        "sahip": aday.sahip,
        "sahip_linki": aday.sahip_linki,
        "kaynak_sayfasi": aday.sayfa_linki,
        "indirme": aday.indirme,
        "lisans": aday.lisans,
        "lisans_url": aday.lisans_url,
        "atif": atif,
        "atif_gerekli": false,
        "kelime": kelime,
        "sure_sn": aday.sure,
        "genislik": aday.genislik,
        "yukseklik": aday.yukseklik,
        "dosya": "hero.mp4",
        "poster": "poster.webp",
        "video_bayt": video_bayt,
        "poster_bayt": poster_bayt,
        "tarih": tarih_metni(),
    });
    kaynak_yaz(&klasor, &bilgi)?;
    Ok(Hazir {
        grup: grup_id.to_string(),
        dosya_adi: "hero.mp4".to_string(),
        poster_adi: "poster.webp".to_string(),
        video_bayt,
        poster_bayt,
        atif,
        sure: aday.sure,
    })
}

/// Müşteri fotoğrafını Ken Burns (zoom + pan) sinematik döngüye çevirir:
/// 720p, sessiz, 8 sn, her kare anahtar kare; poster de üretilir.
/// Dış kaynak kullanılmaz, yalnız yerel dosya ve ffmpeg gerekir.
pub fn foto_ken_burns(grup_id: &str, kaynak_yol: &str) -> Result<Hazir, String> {
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    foto_ken_burns_hedef(kaynak_yol, &klasor, 0)
}

/// Aynı dönüşüm, seçilen klasöre: müşteri fotoğrafı yayın anında demo
/// klasörüne yazılır. `tohum` zoom hedefini, salınımı ve yönü belirler;
/// aynı fotoğraf iki işletmede aynı hareketi tekrarlamaz.
pub fn foto_ken_burns_hedef(kaynak_yol: &str, klasor: &Path, tohum: u64) -> Result<Hazir, String> {
    if !ffmpeg_var() {
        return Err("ffmpeg bulunamadı: fotoğraf hareketlendirilemiyor.".to_string());
    }
    let kaynak = Path::new(kaynak_yol.trim());
    if !kaynak.is_file() {
        return Err("Fotoğraf bulunamadı: dosya yolunu kontrol edin.".to_string());
    }
    let uzanti = kaynak
        .extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(uzanti.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        return Err("Yalnız jpg, jpeg, png veya webp fotoğraf kullanılabilir.".to_string());
    }
    std::fs::create_dir_all(klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    let video = klasor.join("hero.mp4");
    let poster = klasor.join("poster.webp");
    // Önce büyüt, sonra zoompan: 720p çıktıda titreme olmaz. Zoom hedefi,
    // salınım ve yön tohumdan gelir.
    let zum = 1.10 + ((tohum >> 7) % 15) as f64 * 0.012;
    let salinim = 14.0 + ((tohum >> 11) % 12) as f64;
    let yon = if (tohum >> 3) & 1 == 0 { 1.0 } else { -1.0 };
    let filtre = format!(
        "scale=2560:-2,zoompan=z='min(1.0+0.0011*on,{zum:.3})':x='iw/2-(iw/zoom)/2+sin(on/70)*{salinim:.0}*{yon:.0}':y='ih/2-(ih/zoom)/2-cos(on/90)*{salinim:.0}':d=200:s=1280x720:fps=25,format=yuv420p"
    );
    let durum = gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-loop", "1", "-i"])
        .arg(kaynak)
        .args([
            "-t",
            "8",
            "-vf",
            &filtre,
            "-an",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-g",
            "1",
            "-keyint_min",
            "1",
            "-sc_threshold",
            "0",
            "-movflags",
            "+faststart",
        ])
        .arg(&video)
        .status()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        return Err("Fotoğraf hareketli videoya çevrilemedi.".to_string());
    }
    let durum = gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-ss", "0.5", "-i"])
        .arg(&video)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=-2:720",
            "-c:v",
            "libwebp",
            "-quality",
            "82",
        ])
        .arg(&poster)
        .status()
        .map_err(|hata| format!("ffmpeg (poster) çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        return Err("Poster görüntüsü çıkarılamadı.".to_string());
    }
    let video_bayt = std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0);
    let poster_bayt = std::fs::metadata(&poster).map(|m| m.len()).unwrap_or(0);
    let bilgi = serde_json::json!({
        "kaynak": "müşteri fotoğrafı",
        "dosya": "hero.mp4",
        "poster": "poster.webp",
        "efekt": "ken-burns",
        "tohum": tohum,
        "sure_sn": VIDEO_SN,
        "genislik": 1280,
        "yukseklik": 720,
        "video_bayt": video_bayt,
        "poster_bayt": poster_bayt,
        "tarih": tarih_metni(),
    });
    kaynak_yaz(klasor, &bilgi)?;
    Ok(Hazir {
        grup: klasor
            .file_name()
            .and_then(|ad| ad.to_str())
            .unwrap_or("musteri")
            .to_string(),
        dosya_adi: "hero.mp4".to_string(),
        poster_adi: "poster.webp".to_string(),
        video_bayt,
        poster_bayt,
        atif: "Müşteri fotoğrafı".to_string(),
        sure: VIDEO_SN,
    })
}

/// `KAYNAK.json` dosyasını klasöre yazar.
fn kaynak_yaz(klasor: &Path, bilgi: &serde_json::Value) -> Result<(), String> {
    let metin = serde_json::to_string_pretty(bilgi)
        .map_err(|hata| format!("KAYNAK.json hazırlanamadı: {hata}"))?;
    std::fs::write(klasor.join("KAYNAK.json"), metin)
        .map_err(|hata| format!("KAYNAK.json yazılamadı: {hata}"))
}

fn tarih_metni() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Adresten dosyaya indirir; boyut üst sınırı aşılırsa durur.
fn indir(url: &str, hedef: &Path) -> Result<u64, String> {
    let yanit = ajan()
        .get(url)
        .call()
        .map_err(|_| "Video indirilemedi (bağlantı yok).".to_string())?;
    let mut okuyucu = yanit.into_reader();
    let mut dosya =
        std::fs::File::create(hedef).map_err(|hata| format!("Geçici dosya yazılamadı: {hata}"))?;
    let mut toplam = 0u64;
    let mut tampon = [0u8; 64 * 1024];
    loop {
        let okunan = okuyucu
            .read(&mut tampon)
            .map_err(|hata| format!("İndirme kesildi: {hata}"))?;
        if okunan == 0 {
            break;
        }
        toplam += okunan as u64;
        if toplam > INDIRME_SINIRI {
            let _ = std::fs::remove_file(hedef);
            return Err("Video 180 MB sınırından büyük; aday atlandı.".to_string());
        }
        dosya
            .write_all(&tampon[..okunan])
            .map_err(|hata| format!("Geçici dosya yazılamadı: {hata}"))?;
    }
    Ok(toplam)
}

/// Önizleme karesini 320×180 RGBA olarak indirir (panel küçük görseli).
pub fn onizleme_rgba(url: &str) -> Result<(u32, u32, Vec<u8>), String> {
    if url.trim().is_empty() {
        return Err("Önizleme adresi yok.".to_string());
    }
    if !ffmpeg_var() {
        return Err("ffmpeg yok: önizleme gösterilemiyor.".to_string());
    }
    let gecici = std::env::temp_dir().join(format!(
        "nexus_onizleme_{}.jpg",
        std::process::id().to_string() + &url.len().to_string()
    ));
    indir(url, &gecici)?;
    let cikti = gizli_komut("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&gecici)
        .args([
            "-vf",
            "scale=320:180",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-",
        ])
        .output()
        .map_err(|hata| format!("Önizleme dönüştürülemedi: {hata}"))?;
    let _ = std::fs::remove_file(&gecici);
    if !cikti.status.success() {
        return Err("Önizleme karesi çözülemedi.".to_string());
    }
    let beklenen = 320 * 180 * 4;
    if cikti.stdout.len() < beklenen {
        return Err("Önizleme karesi eksik geldi.".to_string());
    }
    Ok((320, 180, cikti.stdout[..beklenen].to_vec()))
}

// --- Kitaplık (yerel içe aktarma) -----------------------------------------

/// Stok API araması kapalı: panel anahtar istemez, dışarıya istek gitmez.
/// Kod silinmedi; ileride `true` yapıldığında arama arayüzü yeniden bağlanır.
pub const API_ACIK: bool = false;

/// Panelden gelen yerel video isteği.
pub struct YerelIstek {
    pub kaynak_yol: String,
    /// Kırpmanın başlayacağı saniye.
    pub baslangic: f64,
    pub kaynak_linki: String,
    pub cekim: String,
    pub lisans: String,
    pub atif_gerekli: bool,
}

/// Desteklenen uzantılar: mp4, mov, webm (ve m4v).
pub fn uzanti_gecerli(yol: &str) -> bool {
    let uzanti = Path::new(yol)
        .extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase());
    matches!(uzanti.as_deref(), Some("mp4" | "mov" | "webm" | "m4v"))
}

/// Kaynak videonun süresi (saniye); ffprobe yoksa veya dosya bozuksa hata.
pub fn sure_sn(yol: &str) -> Result<f64, String> {
    let cikti = gizli_komut("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(yol)
        .output()
        .map_err(|_| "ffprobe bulunamadı: video süresi okunamıyor.".to_string())?;
    if !cikti.status.success() {
        return Err("Video okunamadı: dosya bozuk veya desteklenmeyen bir biçimde.".to_string());
    }
    String::from_utf8_lossy(&cikti.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "Video süresi okunamadı.".to_string())
}

/// Başlangıç saniyesini videonun sınırlarına çeker: son 8 saniyeden önce.
fn baslangic_sinirla(baslangic: f64, sure: f64) -> f64 {
    if !baslangic.is_finite() || baslangic < 0.0 {
        return 0.0;
    }
    baslangic.min((sure - VIDEO_SN).max(0.0))
}

/// Dönüştürülen videonun piksel ölçüsü (poster/video künyesi için).
fn boyut_oku(yol: &Path) -> Option<(u32, u32)> {
    let cikti = gizli_komut("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(yol)
        .output()
        .ok()?;
    if !cikti.status.success() {
        return None;
    }
    let metin = String::from_utf8_lossy(&cikti.stdout);
    let (genislik, yukseklik) = metin.trim().split_once(',')?;
    Some((
        genislik.trim().parse().ok()?,
        yukseklik.trim().parse().ok()?,
    ))
}

fn kaynak_oku(klasor: &Path) -> serde_json::Value {
    std::fs::read_to_string(klasor.join("KAYNAK.json"))
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_else(|| serde_json::json!({"surum": 2, "videolar": []}))
}

/// Kayıt listesi: yeni biçimde `videolar`, eski biçimde tek nesne.
fn kayit_listesi(json: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(liste) = json.get("videolar").and_then(|v| v.as_array()) {
        return liste.clone();
    }
    if json.get("dosya").is_some() {
        vec![json.clone()]
    } else {
        Vec::new()
    }
}

fn bilgi_uret(klasor: &Path, grup_id: &str, kayit: &serde_json::Value) -> Option<VideoBilgi> {
    let dosya_adi = metin(kayit, "dosya");
    if dosya_adi.is_empty() {
        return None;
    }
    let poster_adi = match metin(kayit, "poster") {
        ad if ad.is_empty() => "poster.webp".to_string(),
        ad => ad,
    };
    let video = klasor.join(&dosya_adi);
    let poster = klasor.join(&poster_adi);
    if !video.is_file() || !poster.is_file() {
        return None;
    }
    Some(VideoBilgi {
        grup: grup_id.to_string(),
        dosya_adi,
        poster_adi,
        dosya: "media/hero.mp4".to_string(),
        poster: "media/poster.webp".to_string(),
        saglayici: metin(kayit, "saglayici"),
        sahip: metin(kayit, "sahip"),
        kaynak_sayfasi: metin(kayit, "kaynak_sayfasi"),
        lisans: metin(kayit, "lisans"),
        atif: metin(kayit, "atif"),
        atif_gerekli: kayit
            .get("atif_gerekli")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        sure: kayit.get("sure_sn").and_then(|v| v.as_f64()).unwrap_or(0.0),
        genislik: kayit.get("genislik").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        yukseklik: kayit.get("yukseklik").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        video_bayt: std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0),
        poster_bayt: std::fs::metadata(&poster).map(|m| m.len()).unwrap_or(0),
    })
}

/// Bir sektördeki tüm videolar (diskte var olanlar).
pub fn kayitlar(grup_id: &str) -> Vec<VideoBilgi> {
    kayitlar_yoldan(&Path::new(MEDYA_KOK).join(grup_id), grup_id)
}

/// Yol verilebilir kayıt okuma (testler ve taşınabilir kullanım).
pub fn kayitlar_yoldan(klasor: &Path, grup_id: &str) -> Vec<VideoBilgi> {
    let json = kaynak_oku(klasor);
    kayit_listesi(&json)
        .iter()
        .filter_map(|kayit| bilgi_uret(klasor, grup_id, kayit))
        .collect()
}

/// Uyumluluk: grubun ilk videosu.
pub fn video_bilgisi(grup_id: &str) -> Option<VideoBilgi> {
    kayitlar(grup_id).into_iter().next()
}

/// İşletme adı + adresten sabit tohum: komşu işletmeler farklı video alır.
pub fn tohum(ad: &str, adres: &str) -> u64 {
    let mut ozet: u64 = 0xcbf2_9ce4_8422_2325;
    for bayt in format!("{ad}|{adres}").to_lowercase().as_bytes() {
        ozet ^= u64::from(*bayt);
        ozet = ozet.wrapping_mul(0x100_0000_01b3);
    }
    ozet
}

/// Tohuma göre video seçer; video yoksa `None` (shader sürümüne düşülür).
pub fn sec(grup_id: &str, tohum: u64) -> Option<VideoBilgi> {
    sec_yoldan(&Path::new(MEDYA_KOK).join(grup_id), grup_id, tohum)
}

/// Yol verilebilir seçim: aynı sektörde tohum mod kayıt sayısı.
pub fn sec_yoldan(klasor: &Path, grup_id: &str, tohum: u64) -> Option<VideoBilgi> {
    let liste = kayitlar_yoldan(klasor, grup_id);
    if liste.is_empty() {
        return None;
    }
    Some(liste[(tohum % liste.len() as u64) as usize].clone())
}

/// Kütüphanedeki tüm sektör videoları (panel listesi).
pub fn kutuphane() -> Vec<VideoBilgi> {
    let mut cikti = Vec::new();
    for grup in GRUPLAR {
        cikti.extend(kayitlar(grup.id));
    }
    cikti
}

/// Yerel videoyu dönüştürür: 720p, her kare anahtar kare, sessiz, 8 sn.
pub fn yerel_hazirla(grup_id: &str, istek: &YerelIstek) -> Result<Hazir, String> {
    let yol = istek.kaynak_yol.trim();
    if yol.is_empty() {
        return Err("Video yolu boş: dosyayı sürükleyin veya tam yolu yapıştırın.".to_string());
    }
    let kaynak = Path::new(yol);
    if !kaynak.is_file() {
        return Err(format!("Dosya bulunamadı: {yol}"));
    }
    if !uzanti_gecerli(yol) {
        return Err("Yalnız mp4, mov ve webm dosyaları kabul edilir.".to_string());
    }
    if !ffmpeg_var() {
        return Err(
            "ffmpeg bulunamadı: video dönüştürülemiyor. Kurulum: https://ffmpeg.org/download.html"
                .to_string(),
        );
    }
    let sure = sure_sn(yol)?;
    if sure < 1.0 {
        return Err("Video çok kısa: en az 1 saniye olmalı.".to_string());
    }
    let baslangic = baslangic_sinirla(istek.baslangic, sure);
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    std::fs::create_dir_all(&klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    let mut json = kaynak_oku(&klasor);
    let mut liste = kayit_listesi(&json);
    // Aynı sektörde birden fazla video: sıradaki boş numara bulunur.
    let mut sira = liste.len() + 1;
    let mut video_adi = format!("v{sira}.mp4");
    while klasor.join(&video_adi).exists() {
        sira += 1;
        video_adi = format!("v{sira}.mp4");
    }
    let poster_adi = format!("v{sira}.webp");
    let video_yolu = klasor.join(&video_adi);
    let poster_yolu = klasor.join(&poster_adi);

    let durum = gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-ss", &format!("{baslangic:.3}"), "-i"])
        .arg(kaynak)
        .args([
            "-t",
            &format!("{VIDEO_SN:.0}"),
            "-an",
            "-vf",
            "scale=-2:720:flags=lanczos",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-profile:v",
            "high",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "1",
            "-keyint_min",
            "1",
            "-sc_threshold",
            "0",
            "-movflags",
            "+faststart",
        ])
        .arg(&video_yolu)
        .status()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        let _ = std::fs::remove_file(&video_yolu);
        return Err("Video dönüştürülemedi: dosya bozuk veya okunamıyor.".to_string());
    }

    let durum = gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-ss", "0.6", "-i"])
        .arg(&video_yolu)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=-2:720",
            "-c:v",
            "libwebp",
            "-quality",
            "82",
        ])
        .arg(&poster_yolu)
        .status()
        .map_err(|hata| format!("ffmpeg (poster) çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        let _ = std::fs::remove_file(&video_yolu);
        return Err("Poster görüntüsü çıkarılamadı.".to_string());
    }

    let video_bayt = std::fs::metadata(&video_yolu).map(|m| m.len()).unwrap_or(0);
    let poster_bayt = std::fs::metadata(&poster_yolu)
        .map(|m| m.len())
        .unwrap_or(0);
    let (genislik, yukseklik) = boyut_oku(&video_yolu).unwrap_or((1280, 720));
    let atif = {
        let parcalar: Vec<&str> = [istek.cekim.trim(), istek.lisans.trim()]
            .into_iter()
            .filter(|parca| !parca.is_empty())
            .collect();
        if !parcalar.is_empty() {
            parcalar.join(" · ")
        } else if !istek.kaynak_linki.trim().is_empty() {
            format!("Kaynak: {}", istek.kaynak_linki.trim())
        } else {
            "Yerel çekim".to_string()
        }
    };
    let kullanilan_sure = VIDEO_SN.min((sure - baslangic).max(1.0));
    let kayit = serde_json::json!({
        "saglayici": "Yerel dosya",
        "sahip": istek.cekim.trim(),
        "kaynak_sayfasi": istek.kaynak_linki.trim(),
        "lisans": istek.lisans.trim(),
        "atif": atif,
        "atif_gerekli": istek.atif_gerekli,
        "dosya": video_adi,
        "poster": poster_adi,
        "sure_sn": kullanilan_sure,
        "kaynak_sure_sn": sure,
        "baslangic_sn": baslangic,
        "genislik": genislik,
        "yukseklik": yukseklik,
        "video_bayt": video_bayt,
        "poster_bayt": poster_bayt,
        "tarih": tarih_metni(),
    });
    liste.push(kayit);
    json = serde_json::json!({"surum": 2, "videolar": liste});
    kaynak_yaz(&klasor, &json)?;

    Ok(Hazir {
        grup: grup_id.to_string(),
        dosya_adi: video_adi,
        poster_adi,
        video_bayt,
        poster_bayt,
        atif,
        sure: kullanilan_sure,
    })
}

/// Tek bir kaydı siler; dosyaları ve KAYNAK.json girdisini kaldırır.
pub fn sil_kayit(grup_id: &str, dosya_adi: &str) -> Result<(), String> {
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    let mut json = kaynak_oku(&klasor);
    let mut liste = kayit_listesi(&json);
    let onceki = liste.len();
    let mut poster_adi = String::new();
    liste.retain(|kayit| {
        if metin(kayit, "dosya") == dosya_adi {
            poster_adi = metin(kayit, "poster");
            false
        } else {
            true
        }
    });
    for ad in [dosya_adi.to_string(), poster_adi] {
        if ad.is_empty() {
            continue;
        }
        let yol = klasor.join(&ad);
        if let Err(hata) = std::fs::remove_file(&yol) {
            if hata.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("{ad} silinemedi: {hata}"));
            }
        }
    }
    if liste.len() == onceki {
        return Ok(());
    }
    json = serde_json::json!({"surum": 2, "videolar": liste});
    kaynak_yaz(&klasor, &json)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    #[ignore]
    fn foto_ken_burns_uretir() {
        let kaynak = "media/stock/kuafor/photos/1.jpg";
        if !Path::new(kaynak).is_file() {
            println!("örnek fotoğraf yok; test atlandı");
            return;
        }
        let hazir = foto_ken_burns("deneme_kb", kaynak).expect("ken burns");
        println!(
            "video {} B · poster {} B",
            hazir.video_bayt, hazir.poster_bayt
        );
        assert!(hazir.video_bayt > 40_000, "video üretilmedi");
        assert!(hazir.poster_bayt > 2_000, "poster üretilmedi");
        assert!(Path::new("media/sektor/deneme_kb/KAYNAK.json").is_file());
        let _ = std::fs::remove_dir_all("media/sektor/deneme_kb");
    }

    /// Müşteri fotoğrafı yayın anında hedef klasöre (demo media/) yazılır.
    #[test]
    #[ignore]
    fn foto_ken_burns_hedef_klasore_yazar() {
        let kaynak = "media/stock/kuafor/photos/1.jpg";
        if !Path::new(kaynak).is_file() {
            println!("örnek fotoğraf yok; test atlandı");
            return;
        }
        let hedef = std::env::temp_dir().join(format!("nexus_kb_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&hedef);
        let hazir = foto_ken_burns_hedef(kaynak, &hedef, 0x0051_ed0f).expect("ken burns");
        println!(
            "müşteri fotoğrafı: video {} B · poster {} B",
            hazir.video_bayt, hazir.poster_bayt
        );
        assert!(hazir.video_bayt > 40_000, "video üretilmedi");
        assert!(hedef.join("poster.webp").is_file(), "poster üretilmedi");
        assert!(hedef.join("KAYNAK.json").is_file(), "künye yazılmadı");
        let _ = std::fs::remove_dir_all(&hedef);
    }

    fn aday(id: &str, genislik: u32, yukseklik: u32, sure: f64) -> Aday {
        Aday {
            saglayici: "pexels",
            id: id.to_string(),
            sahip: "Çekim Sahibi".to_string(),
            sahip_linki: "https://example.com/sahip".to_string(),
            sayfa_linki: "https://example.com/video".to_string(),
            indirme: "https://example.com/v.mp4".to_string(),
            onizleme: "https://example.com/v.jpg".to_string(),
            genislik,
            yukseklik,
            sure,
            lisans: "Pexels Lisansı".to_string(),
            lisans_url: "https://www.pexels.com/license/".to_string(),
        }
    }

    #[test]
    fn her_grup_uc_kelime_ve_etiket_tasir() {
        for grup in GRUPLAR {
            assert_eq!(grup.kelimeler.len(), 3, "{} üç kelime", grup.id);
            assert!(grup.kelimeler.iter().all(|k| !k.trim().is_empty()));
            assert!(!grup.etiketler.is_empty());
        }
    }

    #[test]
    fn sektor_grubu_turkce_kategoriden_bulunur() {
        assert_eq!(grup_esle("Erkek Kuaförü").map(|g| g.id), Some("kuafor"));
        assert_eq!(grup_esle("Diş Kliniği").map(|g| g.id), Some("klinik"));
        assert_eq!(grup_esle("Oto Tamir ve Servis").map(|g| g.id), Some("oto"));
        assert_eq!(grup_esle("Butik Otel").map(|g| g.id), Some("otel"));
        assert_eq!(grup_esle("Kahve Dükkânı").map(|g| g.id), None);
    }

    #[test]
    fn dikey_kisa_ve_dusuk_cozunurluk_elenir() {
        let adaylar = vec![
            aday("iyi", 1920, 1080, 12.0),
            aday("dikey", 1080, 1920, 12.0),
            aday("kisa", 1920, 1080, 4.0),
            aday("uzun", 1920, 1080, 25.0),
            aday("dusuk", 1280, 720, 12.0),
            aday("iyi2", 3840, 2160, 9.0),
        ];
        let kalan = filtrele(adaylar);
        let kimlikler: Vec<&str> = kalan.iter().map(|a| a.id.as_str()).collect();
        assert!(kimlikler.contains(&"iyi"));
        assert!(kimlikler.contains(&"iyi2"));
        assert!(!kimlikler.contains(&"dikey"));
        assert!(!kimlikler.contains(&"kisa"));
        assert!(!kimlikler.contains(&"uzun"));
        assert!(!kimlikler.contains(&"dusuk"));
    }

    #[test]
    fn yavas_cekim_adaylari_one_alinir() {
        let adaylar = vec![
            aday("kisa", 1920, 1080, 5.5),
            aday("orta", 1920, 1080, 10.0),
            aday("uzun", 1920, 1080, 19.0),
        ];
        let kalan = filtrele(adaylar);
        assert_eq!(kalan.first().map(|a| a.id.as_str()), Some("orta"));
        assert_eq!(kalan.last().map(|a| a.id.as_str()), Some("uzun"));
    }

    #[test]
    fn ayni_video_iki_kez_listelenmez_ve_sekizle_sinirlidir() {
        let mut adaylar: Vec<Aday> = (0..12)
            .map(|i| aday(&format!("v{i}"), 1920, 1080, 10.0))
            .collect();
        adaylar.push(aday("v0", 1920, 1080, 10.0));
        let kalan = filtrele(adaylar);
        assert_eq!(kalan.len(), 8);
        assert_eq!(kalan.iter().filter(|a| a.id == "v0").count(), 1);
    }

    #[test]
    fn kota_siniri_asilmaz_ve_turkce_uyari_verir() {
        let klasor = std::env::temp_dir().join(format!("nexus_kota_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&klasor);
        let yol = klasor.join("kota.json").to_string_lossy().to_string();
        let an = 1_000_000u64;
        for _ in 0..PEXELS_SAAT {
            kota_artir_yoldan("pexels", &yol, an);
        }
        let hata = kota_kontrol_yoldan("pexels", &yol, an + 10).expect_err("sınır dolmalı");
        assert!(hata.contains("Pexels saatlik sınırı doldu"), "{hata}");
        // Bir saat sonra pencere boşalır.
        assert!(kota_kontrol_yoldan("pexels", &yol, an + 3_601).is_ok());
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn pixabay_dakika_siniri_uygulanir() {
        let klasor = std::env::temp_dir().join(format!("nexus_kota_p_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&klasor);
        let yol = klasor.join("kota.json").to_string_lossy().to_string();
        let an = 2_000_000u64;
        for _ in 0..PIXABAY_DAKIKA {
            kota_artir_yoldan("pixabay", &yol, an);
        }
        let hata = kota_kontrol_yoldan("pixabay", &yol, an + 5).expect_err("sınır dolmalı");
        assert!(hata.contains("Pixabay dakikalık sınırı doldu"), "{hata}");
        assert!(kota_kontrol_yoldan("pixabay", &yol, an + 61).is_ok());
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn api_varsayilan_olarak_kapali() {
        let acik = API_ACIK;
        assert!(
            !acik,
            "stok bulucu API'siz, yalnız yerel dosyayla çalışmalı"
        );
        assert!(!GRUPLAR.is_empty(), "sektör grupları hazır olmalı");
    }

    fn gecici_klasor(ad: &str) -> std::path::PathBuf {
        let klasor = std::env::temp_dir().join(format!("nexus_yerel_{}_{ad}", std::process::id()));
        let _ = std::fs::create_dir_all(&klasor);
        klasor
    }

    #[test]
    fn uzanti_kontrolu_mp4_mov_webm_kabul_eder() {
        assert!(uzanti_gecerli("kuafor.mp4"));
        assert!(uzanti_gecerli("KUAfor.MOV"));
        assert!(uzanti_gecerli("tanitim.webm"));
        assert!(!uzanti_gecerli("eski.avi"));
        assert!(!uzanti_gecerli("uzantisiz"));
    }

    #[test]
    fn baslangic_video_sinirina_cekilir() {
        assert_eq!(baslangic_sinirla(-3.0, 20.0), 0.0);
        assert_eq!(baslangic_sinirla(2.0, 20.0), 2.0);
        assert_eq!(baslangic_sinirla(18.0, 20.0), 12.0);
        assert_eq!(baslangic_sinirla(4.0, 5.0), 0.0);
    }

    #[test]
    fn coklu_video_kaydi_okunur_ve_tohumla_secilir() {
        let klasor = gecici_klasor("coklu");
        for ad in ["v1.mp4", "v2.mp4"] {
            std::fs::write(klasor.join(ad), [0u8; 16]).expect("video");
        }
        for ad in ["v1.webp", "v2.webp"] {
            std::fs::write(klasor.join(ad), [1u8; 8]).expect("poster");
        }
        let json = serde_json::json!({"surum": 2, "videolar": [
            {"dosya": "v1.mp4", "poster": "v1.webp", "atif": "A · Lisans", "atif_gerekli": true},
            {"dosya": "v2.mp4", "poster": "v2.webp", "atif": "B", "atif_gerekli": false}
        ]});
        kaynak_yaz(&klasor, &json).expect("KAYNAK.json");
        assert_eq!(kayitlar_yoldan(&klasor, "kuafor").len(), 2);
        let a = sec_yoldan(&klasor, "kuafor", 0).expect("ilk");
        let b = sec_yoldan(&klasor, "kuafor", 1).expect("ikinci");
        assert_ne!(
            a.dosya_adi, b.dosya_adi,
            "komşu işletme farklı video almalı"
        );
        assert_eq!(a.dosya, "media/hero.mp4");
        assert_eq!(a.poster, "media/poster.webp");
        assert!(a.sayfa_atfi().is_some(), "atıf istenmişse altbilgiye girer");
        assert!(b.sayfa_atfi().is_none(), "atıf istenmemişse girmez");
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn eski_tek_video_bicimi_de_okunur() {
        let klasor = gecici_klasor("eski");
        std::fs::write(klasor.join("hero.mp4"), [0u8; 16]).expect("video");
        std::fs::write(klasor.join("poster.webp"), [1u8; 8]).expect("poster");
        let json = serde_json::json!({"dosya": "hero.mp4", "poster": "poster.webp"});
        kaynak_yaz(&klasor, &json).expect("KAYNAK.json");
        let liste = kayitlar_yoldan(&klasor, "kuafor");
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].dosya_adi, "hero.mp4");
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn bozuk_dosya_turkce_hata_verir() {
        let klasor = gecici_klasor("bozuk");
        let yol = klasor.join("bozuk.mp4");
        std::fs::write(&yol, b"bu bir video degil").expect("dosya");
        let istek = YerelIstek {
            kaynak_yol: yol.to_string_lossy().to_string(),
            baslangic: 0.0,
            kaynak_linki: String::new(),
            cekim: String::new(),
            lisans: String::new(),
            atif_gerekli: false,
        };
        let hata = match yerel_hazirla("kuafor", &istek) {
            Ok(_) => panic!("bozuk dosya kabul edilmemeli"),
            Err(hata) => hata,
        };
        assert!(
            hata.contains("okunamadı") || hata.contains("dönüştürülemedi"),
            "sade Türkçe hata bekleniyordu: {hata}"
        );
        let _ = std::fs::remove_dir_all(&klasor);
    }
}
