//! İşlem günlüğü: onay, red ve tedarik adımları **tam dosya yollarıyla**.
//!
//! Her satır panelde (`/islemler`) ve konsolda aynı anda görünür:
//! `[2026-11-06 10:12:03] ONAY    C:\\...\\media\\pending\\kuafor\\photos\\1.jpg → C:\\...\\media\\stock\\kuafor\\photos\\1.jpg · kuafor`
//!
//! Günlük proje kökünde `media_onay_islem.log` dosyasına eklenir; panel son
//! satırları okuyup canlı gösterir. Dosya silinirse yeniden oluşur.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// İşlem günlüğü dosyası (proje kökünde).
pub const DOSYA: &str = "media_onay_islem.log";
/// Panele en fazla bu kadar satır verilir.
const EN_FAZLA: usize = 400;

/// Günlüğün gerçek yolu: çağıran `kok` sektör/kuyruk klasörü olsa bile
/// proje kökündeki `media_onay_islem.log` kullanılır.
fn gunluk_yolu(kok: &Path) -> PathBuf {
    let mut aday = kok.to_path_buf();
    for _ in 0..4 {
        if aday.join("Cargo.toml").is_file() || aday.join("media").is_dir() {
            return aday.join(DOSYA);
        }
        match aday.parent() {
            Some(ust) if ust != aday => aday = ust.to_path_buf(),
            _ => break,
        }
    }
    kok.join(DOSYA)
}

/// Diskteki tam yolu döndürür; canonicalize'ın `\\?\` öneki kullanılmaz.
pub fn tam_yol(yol: &Path) -> String {
    std::path::absolute(yol)
        .unwrap_or_else(|_| yol.to_path_buf())
        .display()
        .to_string()
}

/// Tek işlem satırını diske ve konsola yazar.
pub fn kaydet(kok: &Path, islem: &str, yol: &str, ek: &str) {
    let saat = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let satir = if ek.is_empty() {
        format!("[{saat}] {islem:<7} {yol}")
    } else {
        format!("[{saat}] {islem:<7} {yol} · {ek}")
    };
    println!("[İŞLEM] {satir}");
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(gunluk_yolu(kok))
        .and_then(|mut dosya| writeln!(dosya, "{satir}"));
}

/// Son `adet` işlem satırı (en yeniler sonda).
pub fn son(kok: &Path, adet: usize) -> Vec<String> {
    let Ok(metin) = std::fs::read_to_string(gunluk_yolu(kok)) else {
        return Vec::new();
    };
    let satirlar: Vec<&str> = metin
        .lines()
        .filter(|satir| !satir.trim().is_empty())
        .collect();
    let bas = satirlar.len().saturating_sub(adet.min(EN_FAZLA));
    satirlar[bas..]
        .iter()
        .map(|satir| (*satir).to_string())
        .collect()
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;

    fn gecici(ad: &str) -> PathBuf {
        let kok = std::env::temp_dir().join(format!("nexus_islem_{}_{ad}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(&kok).unwrap();
        kok
    }

    #[test]
    fn islem_satiri_tam_yol_ve_ek_ile_yazilir() {
        let kok = gecici("yaz");
        let dosya = kok.join("media/pending/kuafor/photos/1.jpg");
        let tam = tam_yol(&dosya);
        assert!(Path::new(&tam).is_absolute(), "tam yol olmalı: {tam}");
        assert!(tam.contains("1.jpg"));
        kaydet(&kok, "ONAY", &tam, "kuafor");
        let liste = son(&kok, 10);
        assert_eq!(liste.len(), 1);
        assert!(liste[0].contains("ONAY"));
        assert!(liste[0].contains("media"));
        assert!(liste[0].contains("kuafor"));
        let _ = std::fs::remove_dir_all(kok);
    }

    #[test]
    fn alt_klasorden_cagrilinca_kok_gunlugu_kullanilir() {
        let kok = gecici("kok");
        std::fs::create_dir_all(kok.join("media/pending")).unwrap();
        std::fs::write(
            kok.join("Cargo.toml"),
            "[package]
name=\"x\"
",
        )
        .unwrap();
        kaydet(&kok.join("media/pending"), "İNDİ", "yol/1.jpg", "");
        assert!(kok.join(DOSYA).is_file(), "kök günlüğü yazılmalı");
        assert!(!kok.join("media/pending").join(DOSYA).exists());
        let _ = std::fs::remove_dir_all(kok);
    }

    #[test]
    fn son_satirlar_en_yenileri_verir() {
        let kok = gecici("kuyruk");
        for sira in 0..5 {
            kaydet(&kok, "RED", &format!("yol/{sira}.jpg"), "");
        }
        let liste = son(&kok, 2);
        assert_eq!(liste.len(), 2);
        assert!(liste[1].contains("yol/4.jpg"), "{liste:?}");
        let _ = std::fs::remove_dir_all(kok);
    }
}
