//! Google API anahtarı için şifreli yerel kasa.
//!
//! Anahtar diske asla düz metin yazılmaz: AES-256-GCM ile makineye bağlı bir
//! anahtarla şifrelenir, dosyada yalnızca base64 metin durur. Makine anahtarı
//! kullanıcı adı + bilgisayar adı + rastgele tuz (`api_tuz.bin`) üzerinden
//! SHA-256 ile türetilir; tuz dosyası silinirse eski şifre çözülemez ve
//! kullanıcıdan anahtar yeniden istenir.
//!
//! Eski sürümden kalan `.env` anahtarı ilk açılışta bir kez okunup kasaya
//! taşınır; böylece kullanıcı anahtarı yeniden yazmak zorunda kalmaz.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};

/// Şifreli anahtar dosyası (proje kökünde).
pub const KASA_DOSYASI: &str = "api_anahtari.enc";
/// Makine anahtarının tuzu; kasa ile aynı klasörde durur.
const TUZ_DOSYASI: &str = "api_tuz.bin";
/// Dosya biçim sürümü: ileride algoritma değişirse eski dosya reddedilir.
const SURUM: u8 = 1;
/// Türetmeye katılan sabit; başka bir uygulama aynı tuzu kullanamaz.
const EK_BILGI: &[u8] = b"nexus-enterprise/google-api/v1";

/// Anahtarın nereden geldiği: arayüzde gösterilir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kaynak {
    /// Şifreli kasadan okundu.
    Kasa,
    /// Eski `.env` / ortam değişkeninden alındı (kasaya taşındı).
    Ortam,
    /// Hiç anahtar yok.
    Yok,
}

/// Bellekteki kasa: çözülmüş anahtar ve kaynağı.
pub struct Kasa {
    anahtar: Option<String>,
    kaynak: Kaynak,
}

impl Kasa {
    /// Varsayılan konumdan yükler; kasa boşsa `.env` yedeğine bakar.
    pub fn yukle() -> Self {
        Self::yukle_yoldan(KASA_DOSYASI, TUZ_DOSYASI)
    }

    /// Testler ve taşınabilir kullanım için yol verilebilir yükleme.
    pub fn yukle_yoldan(kasa_yolu: &str, tuz_yolu: &str) -> Self {
        if let Some(anahtar) = kasa_oku(kasa_yolu, tuz_yolu) {
            return Self {
                anahtar: Some(anahtar),
                kaynak: Kaynak::Kasa,
            };
        }
        if let Some(anahtar) = ortam_anahtari() {
            // Eski anahtar ilk açılışta şifreli kasaya taşınır; `.env` silinse
            // de uygulama anahtarsız kalmaz.
            let _ = Self::kaydet_yoldan(&anahtar, kasa_yolu, tuz_yolu);
            return Self {
                anahtar: Some(anahtar),
                kaynak: Kaynak::Ortam,
            };
        }
        Self {
            anahtar: None,
            kaynak: Kaynak::Yok,
        }
    }

    pub fn anahtar(&self) -> Option<&str> {
        self.anahtar.as_deref()
    }

    /// Testlerin kaynağı doğrulaması için.
    #[cfg(test)]
    pub fn kaynak(&self) -> Kaynak {
        self.kaynak
    }

    /// Arayüzdeki durum satırı.
    pub fn kaynak_metni(&self) -> &'static str {
        match self.kaynak {
            Kaynak::Kasa => "Şifreli yerel kasadan okundu (api_anahtari.enc)",
            Kaynak::Ortam => "Ortam değişkeninden alındı ve kasaya taşındı",
            Kaynak::Yok => "Kayıtlı anahtar yok",
        }
    }

    /// Anahtarı şifreleyip varsayılan kasaya yazar.
    pub fn kaydet(anahtar: &str) -> Result<(), String> {
        Self::kaydet_yoldan(anahtar, KASA_DOSYASI, TUZ_DOSYASI)
    }

    /// Yol verilebilir kayıt (testler için).
    pub fn kaydet_yoldan(anahtar: &str, kasa_yolu: &str, tuz_yolu: &str) -> Result<(), String> {
        let anahtar = anahtar.trim();
        if !gecerli_anahtar(anahtar) {
            return Err(
                "Anahtar biçimi geçersiz: boşluksuz 30-100 karakter olmalı (Google AIza… anahtarı)."
                    .to_string(),
            );
        }
        let sifreli = sifrele(anahtar, tuz_yolu)?;
        std::fs::write(kasa_yolu, sifreli)
            .map_err(|hata| format!("Anahtar kasası yazılamadı: {hata}"))
    }

    /// Kayıtlı anahtarı siler (dosya da kaldırılır).
    pub fn sil() -> Result<(), String> {
        match std::fs::remove_file(KASA_DOSYASI) {
            Ok(()) => Ok(()),
            Err(hata) if hata.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(hata) => Err(format!("Kasa silinemedi: {hata}")),
        }
    }
}

/// Anahtar biçimi: Google anahtarları `AIza` ile başlar, 39 karakterdir; yine
/// de farklı biçimleri reddetmemek için geniş ama boşluksuz bir aralık.
pub fn gecerli_anahtar(anahtar: &str) -> bool {
    let anahtar = anahtar.trim();
    (30..=100).contains(&anahtar.len())
        && !anahtar.contains(char::is_whitespace)
        && anahtar
            .chars()
            .all(|karakter| karakter.is_ascii_alphanumeric() || matches!(karakter, '-' | '_'))
}

/// Kasadaki dosyayı çözer; dosya yoksa/bozuksa `None`.
fn kasa_oku(kasa_yolu: &str, tuz_yolu: &str) -> Option<String> {
    let metin = std::fs::read_to_string(kasa_yolu).ok()?;
    let bayt = STANDARD.decode(metin.trim()).ok()?;
    let anahtar = coz(&bayt, tuz_yolu).ok()?;
    if gecerli_anahtar(&anahtar) {
        Some(anahtar)
    } else {
        None
    }
}

/// Anahtarı şifreler: [sürüm][12 bayt nonce][ciphertext] → base64.
fn sifrele(anahtar: &str, tuz_yolu: &str) -> Result<String, String> {
    let sifre = Aes256Gcm::new_from_slice(&makine_anahtari(tuz_yolu)?)
        .map_err(|hata| format!("Şifreleme anahtarı kurulamadı: {hata}"))?;
    let mut nonce_baytlari = [0u8; 12];
    getrandom::getrandom(&mut nonce_baytlari)
        .map_err(|hata| format!("Rastgele değer üretilemedi: {hata}"))?;
    let nonce = Nonce::from_slice(&nonce_baytlari);
    let sifreli = sifre
        .encrypt(nonce, anahtar.as_bytes())
        .map_err(|_| "Anahtar şifrelenemedi.".to_string())?;
    let mut govde = Vec::with_capacity(1 + 12 + sifreli.len());
    govde.push(SURUM);
    govde.extend_from_slice(&nonce_baytlari);
    govde.extend_from_slice(&sifreli);
    Ok(STANDARD.encode(govde))
}

/// Şifreli gövdeyi çözer; sürüm uyuşmazlığı veya bozulma hataya döner.
fn coz(govde: &[u8], tuz_yolu: &str) -> Result<String, String> {
    if govde.len() < 1 + 12 + 16 {
        return Err("Kasa dosyası çok kısa veya bozuk.".to_string());
    }
    if govde[0] != SURUM {
        return Err("Kasa dosyası bu sürümle uyumlu değil.".to_string());
    }
    let nonce = Nonce::from_slice(&govde[1..13]);
    let sifre = Aes256Gcm::new_from_slice(&makine_anahtari(tuz_yolu)?)
        .map_err(|hata| format!("Şifreleme anahtarı kurulamadı: {hata}"))?;
    let acik = sifre
        .decrypt(nonce, &govde[13..])
        .map_err(|_| "Anahtar çözülemedi: tuz dosyası değişmiş veya kasa bozulmuş.".to_string())?;
    String::from_utf8(acik).map_err(|_| "Kasa içeriği metin değil.".to_string())
}

/// Makineye bağlı 32 baytlık anahtar; tuz dosyası yoksa üretilir.
fn makine_anahtari(tuz_yolu: &str) -> Result<[u8; 32], String> {
    let tuz = match std::fs::read(tuz_yolu) {
        Ok(tuz) if tuz.len() >= 16 => tuz,
        _ => {
            let mut tuz = vec![0u8; 32];
            getrandom::getrandom(&mut tuz).map_err(|hata| format!("Tuz üretilemedi: {hata}"))?;
            std::fs::write(tuz_yolu, &tuz)
                .map_err(|hata| format!("Tuz dosyası yazılamadı: {hata}"))?;
            tuz
        }
    };
    let kullanici = std::env::var("USERNAME").unwrap_or_default();
    let makine = std::env::var("COMPUTERNAME").unwrap_or_default();
    let mut ozet = Sha256::new();
    ozet.update(EK_BILGI);
    ozet.update(kullanici.as_bytes());
    ozet.update([0u8]);
    ozet.update(makine.as_bytes());
    ozet.update([0u8]);
    ozet.update(&tuz);
    Ok(ozet.finalize().into())
}

/// Eski kurulumdan kalan anahtar: ortam değişkeni, yoksa `.env`.
fn ortam_anahtari() -> Option<String> {
    for ad in ["GOOGLE_PLACES_API_KEY", "GOOGLE_MAPS_API_KEY"] {
        if let Ok(deger) = std::env::var(ad) {
            let deger = deger.trim().to_string();
            if gecerli_anahtar(&deger) {
                return Some(deger);
            }
        }
    }
    let icerik = std::fs::read_to_string(".env").ok()?;
    for satir in icerik.lines() {
        let satir = satir.trim();
        if satir.is_empty() || satir.starts_with('#') {
            continue;
        }
        let Some((ad, deger)) = satir.split_once('=') else {
            continue;
        };
        if !matches!(ad.trim(), "GOOGLE_PLACES_API_KEY" | "GOOGLE_MAPS_API_KEY") {
            continue;
        }
        let deger = deger.trim().trim_matches('"').trim_matches('\'');
        if gecerli_anahtar(deger) {
            return Some(deger.to_string());
        }
    }
    None
}

#[cfg(test)]
mod testler {
    use super::*;

    fn yollar(ad: &str) -> (String, String) {
        let klasor = std::env::temp_dir().join(format!("nexus_kasa_{}_{ad}", std::process::id()));
        let _ = std::fs::create_dir_all(&klasor);
        (
            klasor.join("kasa.enc").to_string_lossy().to_string(),
            klasor.join("tuz.bin").to_string_lossy().to_string(),
        )
    }

    #[test]
    fn anahtar_sifrelenip_geri_okunur() {
        let (kasa, tuz) = yollar("tur");
        let anahtar = "AIzaSyB1234567890abcdefghijklmnopqrstu";
        Kasa::kaydet_yoldan(anahtar, &kasa, &tuz).expect("kayıt başarısız");
        let icerik = std::fs::read_to_string(&kasa).expect("kasa okunamadı");
        assert!(
            !icerik.contains("AIzaSy"),
            "anahtar düz metin yazılmamalı: {icerik}"
        );
        let yuklenen = Kasa::yukle_yoldan(&kasa, &tuz);
        assert_eq!(yuklenen.anahtar(), Some(anahtar));
        assert_eq!(yuklenen.kaynak(), Kaynak::Kasa);
        let _ = std::fs::remove_dir_all(std::path::Path::new(&kasa).parent().unwrap());
    }

    #[test]
    fn tuz_degisirse_kasa_cozulmez() {
        let (kasa, tuz) = yollar("tuz");
        Kasa::kaydet_yoldan("AIzaSyB1234567890abcdefghijklmnopqrstu", &kasa, &tuz)
            .expect("kayıt başarısız");
        let baska = format!("{tuz}.baska");
        std::fs::write(&baska, [7u8; 32]).expect("tuz yazılamadı");
        let yuklenen = Kasa::yukle_yoldan(&kasa, &baska);
        assert_ne!(yuklenen.kaynak(), Kaynak::Kasa, "yanlış tuzla açılmamalı");
        let _ = std::fs::remove_dir_all(std::path::Path::new(&kasa).parent().unwrap());
    }

    #[test]
    fn gecersiz_anahtar_reddedilir() {
        assert!(!gecerli_anahtar(""));
        assert!(!gecerli_anahtar("kısa"));
        assert!(!gecerli_anahtar("AIzaSy ile bosluk var 1234567890"));
        assert!(gecerli_anahtar("AIzaSyB1234567890abcdefghijklmnopqrstu"));
    }
}
