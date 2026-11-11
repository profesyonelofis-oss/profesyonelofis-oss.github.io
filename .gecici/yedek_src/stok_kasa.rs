#![allow(dead_code)]
//! Stok video servislerinin (Pexels, Pixabay) API anahtarları için şifreli kasa.
//!
//! Google anahtarının kasasından (`anahtar.rs`) ayrı dosyalar kullanır: mevcut
//! kasa biçimine ve `api_anahtari.enc` içeriğine dokunulmaz. Anahtarlar tek bir
//! JSON gövdesi hâlinde AES-256-GCM ile şifrelenir; diskte yalnızca base64
//! metin durur. Makine anahtarı kullanıcı adı + bilgisayar adı + rastgele tuz
//! (`stok_tuz.bin`) üzerinden SHA-256 ile türetilir.
//!
//! Anahtar hiçbir yerde düz metin olarak yazılmaz ve arayüz dışında okunmaz:
//! yalnızca arama/indirme isteğini atan modül (`stok_video`) çözer.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};

/// Şifreli anahtar dosyası (proje kökünde).
pub const KASA_DOSYASI: &str = "stok_anahtar.enc";
/// Makine anahtarının tuzu; kasa ile aynı klasörde durur.
const TUZ_DOSYASI: &str = "stok_tuz.bin";
/// Dosya biçim sürümü: ileride algoritma değişirse eski dosya reddedilir.
const SURUM: u8 = 1;
/// Türetmeye katılan sabit; Google kasasından ayrı bir anahtar üretir.
const EK_BILGI: &[u8] = b"nexus-enterprise/stok-video/v1";

/// Bellekteki stok anahtarları. Boş alan "anahtar kayıtlı değil" demektir.
#[derive(Clone, Debug, Default)]
pub struct StokKasa {
    pexels: Option<String>,
    pixabay: Option<String>,
}

impl StokKasa {
    /// Varsayılan konumdan yükler.
    pub fn yukle() -> Self {
        Self::yukle_yoldan(KASA_DOSYASI, TUZ_DOSYASI)
    }

    /// Testler ve taşınabilir kullanım için yol verilebilir yükleme.
    pub fn yukle_yoldan(kasa_yolu: &str, tuz_yolu: &str) -> Self {
        let Some(govde) = kasa_oku(kasa_yolu, tuz_yolu) else {
            return Self::default();
        };
        let deger: serde_json::Value = serde_json::from_str(&govde).unwrap_or_default();
        let al = |ad: &str| {
            deger
                .get(ad)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| anahtar_gecerli(s))
                .map(str::to_string)
        };
        Self {
            pexels: al("pexels"),
            pixabay: al("pixabay"),
        }
    }

    pub fn pexels(&self) -> Option<&str> {
        self.pexels.as_deref()
    }

    pub fn pixabay(&self) -> Option<&str> {
        self.pixabay.as_deref()
    }

    /// Arayüzdeki durum satırı: hangi servisin anahtarı hazır?
    pub fn durum_metni(&self) -> String {
        match (self.pexels.is_some(), self.pixabay.is_some()) {
            (true, true) => "Pexels + Pixabay anahtarı şifreli kasada".to_string(),
            (true, false) => "Pexels anahtarı şifreli kasada (yedek yok)".to_string(),
            (false, true) => "Pixabay anahtarı şifreli kasada (Pexels yok)".to_string(),
            (false, false) => "Kayıtlı stok anahtarı yok".to_string(),
        }
    }

    /// Tek bir sağlayıcının anahtarını şifreleyip kasaya yazar; diğeri korunur.
    pub fn kaydet(saglayici: &str, anahtar: &str) -> Result<(), String> {
        Self::kaydet_yoldan(saglayici, anahtar, KASA_DOSYASI, TUZ_DOSYASI)
    }

    pub fn kaydet_yoldan(
        saglayici: &str,
        anahtar: &str,
        kasa_yolu: &str,
        tuz_yolu: &str,
    ) -> Result<(), String> {
        let anahtar = anahtar.trim();
        if !anahtar_gecerli(anahtar) {
            return Err("Anahtar biçimi geçersiz: boşluksuz 20-120 karakter olmalı.".to_string());
        }
        let mevcut = Self::yukle_yoldan(kasa_yolu, tuz_yolu);
        let mut govde = serde_json::Map::new();
        if let Some(eski) = mevcut.pexels.as_deref().filter(|_| saglayici != "pexels") {
            govde.insert("pexels".into(), serde_json::Value::String(eski.into()));
        }
        if let Some(eski) = mevcut.pixabay.as_deref().filter(|_| saglayici != "pixabay") {
            govde.insert("pixabay".into(), serde_json::Value::String(eski.into()));
        }
        govde.insert(
            saglayici.into(),
            serde_json::Value::String(anahtar.to_string()),
        );
        let metin = serde_json::Value::Object(govde).to_string();
        let sifreli = sifrele(&metin, tuz_yolu)?;
        std::fs::write(kasa_yolu, sifreli)
            .map_err(|hata| format!("Stok anahtar kasası yazılamadı: {hata}"))
    }

    /// Tek sağlayıcının anahtarını siler; kasa boş kalırsa dosya kaldırılır.
    pub fn sil(saglayici: &str) -> Result<(), String> {
        Self::sil_yoldan(saglayici, KASA_DOSYASI, TUZ_DOSYASI)
    }

    pub fn sil_yoldan(saglayici: &str, kasa_yolu: &str, tuz_yolu: &str) -> Result<(), String> {
        let mevcut = Self::yukle_yoldan(kasa_yolu, tuz_yolu);
        let mut govde = serde_json::Map::new();
        for (ad, deger) in [("pexels", &mevcut.pexels), ("pixabay", &mevcut.pixabay)] {
            if ad == saglayici {
                continue;
            }
            if let Some(anahtar) = deger {
                govde.insert(ad.into(), serde_json::Value::String(anahtar.clone()));
            }
        }
        if govde.is_empty() {
            match std::fs::remove_file(kasa_yolu) {
                Ok(()) => return Ok(()),
                Err(hata) if hata.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(hata) => return Err(format!("Stok kasası silinemedi: {hata}")),
            }
        }
        let metin = serde_json::Value::Object(govde).to_string();
        let sifreli = sifrele(&metin, tuz_yolu)?;
        std::fs::write(kasa_yolu, sifreli)
            .map_err(|hata| format!("Stok anahtar kasası yazılamadı: {hata}"))
    }
}

/// Anahtar biçimi: boşluksuz, 20-120 karakter, harf/rakam/`-`/`_`.
pub fn anahtar_gecerli(anahtar: &str) -> bool {
    let anahtar = anahtar.trim();
    (20..=120).contains(&anahtar.len())
        && !anahtar.contains(char::is_whitespace)
        && anahtar
            .chars()
            .all(|karakter| karakter.is_ascii_alphanumeric() || matches!(karakter, '-' | '_'))
}

/// Kasadaki dosyayı çözer; dosya yoksa/bozuksa `None`.
fn kasa_oku(kasa_yolu: &str, tuz_yolu: &str) -> Option<String> {
    let metin = std::fs::read_to_string(kasa_yolu).ok()?;
    let bayt = STANDARD.decode(metin.trim()).ok()?;
    coz(&bayt, tuz_yolu).ok()
}

/// Gövdeyi şifreler: [sürüm][12 bayt nonce][ciphertext] → base64.
fn sifrele(govde: &str, tuz_yolu: &str) -> Result<String, String> {
    let sifre = Aes256Gcm::new_from_slice(&makine_anahtari(tuz_yolu)?)
        .map_err(|hata| format!("Şifreleme anahtarı kurulamadı: {hata}"))?;
    let mut nonce_baytlari = [0u8; 12];
    getrandom::getrandom(&mut nonce_baytlari)
        .map_err(|hata| format!("Rastgele değer üretilemedi: {hata}"))?;
    let nonce = Nonce::from_slice(&nonce_baytlari);
    let sifreli = sifre
        .encrypt(nonce, govde.as_bytes())
        .map_err(|_| "Anahtar şifrelenemedi.".to_string())?;
    let mut cikti = Vec::with_capacity(1 + 12 + sifreli.len());
    cikti.push(SURUM);
    cikti.extend_from_slice(&nonce_baytlari);
    cikti.extend_from_slice(&sifreli);
    Ok(STANDARD.encode(cikti))
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

#[cfg(test)]
mod testler {
    use super::*;

    fn yollar(ad: &str) -> (String, String) {
        let klasor = std::env::temp_dir().join(format!("nexus_stok_{}_{ad}", std::process::id()));
        let _ = std::fs::create_dir_all(&klasor);
        (
            klasor.join("kasa.enc").to_string_lossy().to_string(),
            klasor.join("tuz.bin").to_string_lossy().to_string(),
        )
    }

    #[test]
    fn iki_anahtar_da_sifrelenip_geri_okunur() {
        let (kasa, tuz) = yollar("tur");
        let pexels = "563492ad6f91700001000001a2b3c4d5e6f7";
        let pixabay = "12345678abcdef9012345678abcdef90";
        StokKasa::kaydet_yoldan("pexels", pexels, &kasa, &tuz).expect("pexels kaydı");
        StokKasa::kaydet_yoldan("pixabay", pixabay, &kasa, &tuz).expect("pixabay kaydı");
        let icerik = std::fs::read_to_string(&kasa).expect("kasa okunamadı");
        assert!(
            !icerik.contains(pexels),
            "Pexels anahtarı düz metin yazılmamalı"
        );
        assert!(
            !icerik.contains(pixabay),
            "Pixabay anahtarı düz metin yazılmamalı"
        );
        let yuklenen = StokKasa::yukle_yoldan(&kasa, &tuz);
        assert_eq!(yuklenen.pexels(), Some(pexels));
        assert_eq!(yuklenen.pixabay(), Some(pixabay));
        let _ = std::fs::remove_dir_all(std::path::Path::new(&kasa).parent().unwrap());
    }

    #[test]
    fn ikinci_kayit_digerini_silmez_ve_silme_kalani_bozmaz() {
        let (kasa, tuz) = yollar("koru");
        let pexels = "563492ad6f91700001000001a2b3c4d5e6f7";
        let pixabay = "12345678abcdef9012345678abcdef90";
        StokKasa::kaydet_yoldan("pexels", pexels, &kasa, &tuz).expect("pexels");
        StokKasa::kaydet_yoldan("pixabay", pixabay, &kasa, &tuz).expect("pixabay");
        StokKasa::sil_yoldan("pexels", &kasa, &tuz).expect("silme");
        let yuklenen = StokKasa::yukle_yoldan(&kasa, &tuz);
        assert_eq!(yuklenen.pexels(), None);
        assert_eq!(yuklenen.pixabay(), Some(pixabay));
        let _ = std::fs::remove_dir_all(std::path::Path::new(&kasa).parent().unwrap());
    }

    #[test]
    fn tuz_degisirse_kasa_cozulmez() {
        let (kasa, tuz) = yollar("tuz");
        StokKasa::kaydet_yoldan(
            "pexels",
            "563492ad6f91700001000001a2b3c4d5e6f7",
            &kasa,
            &tuz,
        )
        .expect("kayıt");
        let baska = format!("{tuz}.baska");
        std::fs::write(&baska, [9u8; 32]).expect("tuz");
        let yuklenen = StokKasa::yukle_yoldan(&kasa, &baska);
        assert!(yuklenen.pexels().is_none(), "yanlış tuzla açılmamalı");
        let _ = std::fs::remove_dir_all(std::path::Path::new(&kasa).parent().unwrap());
    }

    #[test]
    fn gecersiz_anahtar_reddedilir() {
        assert!(!anahtar_gecerli(""));
        assert!(!anahtar_gecerli("kisa"));
        assert!(!anahtar_gecerli("anahtar da bosluk var 1234567890"));
        assert!(anahtar_gecerli("563492ad6f91700001000001a2b3c4d5e6f7"));
    }
}
