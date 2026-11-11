//! Günlük 20 hedefli lead listesi.
//!
//! Tarama (OSM/Overpass) 20 kayıt döndürür; bu modül o kayıtlardan günlük hedef
//! listesini kurar ve takip eder:
//!
//! - Liste **güne bağlıdır** (`hedef_listesi.json`); gün değişince sıfırlanır.
//! - Her satırda işletmenin adı, adresi, telefonu ve koordinatı durur.
//! - "Tamamlandı" işaretlenen satır listeden düşer ve günlük sayaçtan pay
//!   harcar (mesaj atıldı sayılır), böylece gün içinde kimlere ulaşıldığı
//!   liste kalabalığı olmadan izlenir.
//! - Liste temizlenebilir; yeni aramada da sıfırdan kurulur.

use serde::{Deserialize, Serialize};

/// Günlük hedef sayısı: WhatsApp günlük gönderim tavanıyla aynı.
pub const GUNLUK_HEDEF: usize = 20;
pub const DOSYA: &str = "hedef_listesi.json";

/// Tek hedef işletme.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hedef {
    /// Defter anahtarı (`kaynak:ad:konum`): mükerrer kayıt açılmasın.
    pub anahtar: String,
    pub ad: String,
    pub adres: String,
    pub telefon: String,
    pub sektor: String,
    pub konum: Option<(f64, f64)>,
    /// İşletmenin tarama listesindeki sırası (demo/mesaj hazırlamak için).
    #[serde(default)]
    pub lead_idx: Option<usize>,
    /// Mesaj atıldı / tamamlandı: satır gizlenir, sayaç artar.
    #[serde(default)]
    pub tamam: bool,
}

impl Hedef {
    /// "38.61150, 27.42580" — kaydın gerçek koordinatı; yoksa boş.
    pub fn koordinat_metni(&self) -> String {
        match self.konum {
            Some((enlem, boylam)) => format!("{enlem:.5}, {boylam:.5}"),
            None => String::new(),
        }
    }

    /// Listede görünen alt satır: adres · telefon · koordinat.
    pub fn alt_satir(&self) -> String {
        let mut parcalar: Vec<String> = Vec::new();
        if !self.adres.trim().is_empty() {
            parcalar.push(self.adres.trim().to_string());
        }
        if !self.telefon.trim().is_empty() {
            parcalar.push(self.telefon.trim().to_string());
        }
        let koordinat = self.koordinat_metni();
        if !koordinat.is_empty() {
            parcalar.push(koordinat);
        }
        parcalar.join("   ·   ")
    }
}

/// Günlük hedef listesi (diske yazılır).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HedefListesi {
    /// Hangi güne ait ("2026-11-03"). Gün değişince liste sıfırlanır.
    pub gun: String,
    pub hedefler: Vec<Hedef>,
}

impl HedefListesi {
    /// Varsayılan dosyadan okur; bozuk/eksik dosyada boş liste.
    pub fn yukle() -> Self {
        Self::yukle_yoldan(DOSYA)
    }

    pub fn yukle_yoldan(yol: &str) -> Self {
        std::fs::read_to_string(yol)
            .ok()
            .and_then(|metin| serde_json::from_str::<Self>(&metin).ok())
            .unwrap_or_default()
    }

    pub fn kaydet(&self) {
        self.kaydet_yoldan(DOSYA);
    }

    pub fn kaydet_yoldan(&self, yol: &str) {
        if let Ok(metin) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(yol, metin);
        }
    }

    /// Gün değiştiyse listeyi boşaltır (dünün hedefleri bugüne taşınmaz).
    pub fn yeni_gun(&mut self, gun: &str) {
        if self.gun != gun {
            self.gun = gun.to_string();
            self.hedefler.clear();
        }
    }

    /// Günü değiştirmeden listeyi boşaltır ("Listeyi temizle").
    pub fn temizle(&mut self) {
        self.hedefler.clear();
    }

    pub fn bos(&self) -> bool {
        self.hedefler.is_empty()
    }

    /// Listeden düşmemiş (tamamlanmamış) hedefler.
    pub fn aktif(&self) -> Vec<Hedef> {
        self.hedefler
            .iter()
            .filter(|hedef| !hedef.tamam)
            .cloned()
            .collect()
    }

    pub fn kalan(&self) -> usize {
        self.hedefler.iter().filter(|h| !h.tamam).count()
    }

    pub fn tamam_sayisi(&self) -> usize {
        self.hedefler.iter().filter(|h| h.tamam).count()
    }

    /// Listedeki tüm anahtarlar (tamamlananlar dâhil): mükerrer eklenmesin.
    pub fn anahtarlar(&self) -> Vec<String> {
        self.hedefler
            .iter()
            .map(|hedef| hedef.anahtar.clone())
            .collect()
    }

    /// Hedef ekler: mükerrer anahtar ve dolu kontenjan reddedilir.
    pub fn ekle(&mut self, hedef: Hedef) -> bool {
        if self.kalan() >= GUNLUK_HEDEF {
            return false;
        }
        if self
            .hedefler
            .iter()
            .any(|mevcut| mevcut.anahtar == hedef.anahtar)
        {
            return false;
        }
        self.hedefler.push(hedef);
        true
    }

    /// Hedefi tamamlandı olarak işaretler; yeni işaretlendiyse `true`.
    pub fn tamamla(&mut self, anahtar: &str) -> bool {
        for hedef in self.hedefler.iter_mut() {
            if hedef.anahtar == anahtar && !hedef.tamam {
                hedef.tamam = true;
                return true;
            }
        }
        false
    }
}

/// Aday seçimi: `(anahtar, telefon_var, kara_listede)` üçlülerinden en fazla
/// `adet` işletme seçer. Telefonsuz ve kara listedeki kayıtlar atlanır;
/// listede zaten olan anahtarlar tekrar seçilmez. Sıra korunur (tarama sırası
/// zaten potansiyele göredir).
pub fn sec(adaylar: &[(String, bool, bool)], mevcut: &[String], adet: usize) -> Vec<usize> {
    let mut secilen = Vec::new();
    for (idx, (anahtar, telefon_var, kara_listede)) in adaylar.iter().enumerate() {
        if secilen.len() >= adet {
            break;
        }
        if !telefon_var || *kara_listede {
            continue;
        }
        if mevcut.iter().any(|var| var == anahtar) {
            continue;
        }
        if secilen
            .iter()
            .any(|onceki: &usize| adaylar[*onceki].0 == *anahtar)
        {
            continue;
        }
        secilen.push(idx);
    }
    secilen
}

#[cfg(test)]
mod testler {
    use super::*;

    fn hedef(anahtar: &str) -> Hedef {
        Hedef {
            anahtar: anahtar.to_string(),
            ad: format!("İşletme {anahtar}"),
            adres: "Şehzadeler / Manisa".to_string(),
            telefon: "+90 555 000 00 00".to_string(),
            sektor: "Kafe".to_string(),
            konum: Some((38.6115, 27.4258)),
            lead_idx: Some(0),
            tamam: false,
        }
    }

    #[test]
    fn sec_en_fazla_yirmi_kayit_alir() {
        let adaylar: Vec<(String, bool, bool)> =
            (0..45).map(|i| (format!("k{i}"), true, false)).collect();
        let secilen = sec(&adaylar, &[], GUNLUK_HEDEF);
        assert_eq!(secilen.len(), GUNLUK_HEDEF);
        assert_eq!(secilen.first(), Some(&0));
        assert_eq!(secilen.last(), Some(&(GUNLUK_HEDEF - 1)));
    }

    #[test]
    fn sec_telefonsuz_ve_kara_listedekileri_atlar() {
        let adaylar = vec![
            ("a".to_string(), false, false), // telefon yok
            ("b".to_string(), true, true),   // kara liste
            ("c".to_string(), true, false),  // uygun
        ];
        assert_eq!(sec(&adaylar, &[], 20), vec![2]);
    }

    #[test]
    fn sec_mevcut_ve_mukerrer_anahtari_atlar() {
        let adaylar = vec![
            ("a".to_string(), true, false),
            ("b".to_string(), true, false),
            ("b".to_string(), true, false),
        ];
        let secilen = sec(&adaylar, &["a".to_string()], 20);
        assert_eq!(
            secilen,
            vec![1],
            "mükerrer b anahtarı ikinci kez seçilmemeli"
        );
    }

    #[test]
    fn tamamla_satiri_dusurur_ve_sayaci_artirir() {
        let mut liste = HedefListesi::default();
        liste.yeni_gun("2026-11-03");
        assert!(liste.ekle(hedef("a")));
        assert_eq!(liste.kalan(), 1);
        assert!(liste.tamamla("a"));
        assert_eq!(liste.kalan(), 0);
        assert_eq!(liste.tamam_sayisi(), 1);
        assert!(liste.aktif().is_empty());
        // İkinci kez tamamlamak sayacı artırmaz.
        assert!(!liste.tamamla("a"));
        assert_eq!(liste.tamam_sayisi(), 1);
    }

    #[test]
    fn kontenjan_yirmiyi_asmaz_ve_mukerrer_eklenmez() {
        let mut liste = HedefListesi::default();
        liste.yeni_gun("2026-11-03");
        for i in 0..GUNLUK_HEDEF {
            assert!(liste.ekle(hedef(&format!("k{i}"))));
        }
        assert!(!liste.ekle(hedef("fazla")), "21. hedef eklenmemeli");
        assert_eq!(liste.kalan(), GUNLUK_HEDEF);
        assert!(!liste.ekle(hedef("k0")), "mükerrer eklenmemeli");
    }

    #[test]
    fn tamamlanan_slot_yeni_hedefe_acilir() {
        let mut liste = HedefListesi::default();
        liste.yeni_gun("2026-11-03");
        for i in 0..GUNLUK_HEDEF {
            liste.ekle(hedef(&format!("k{i}")));
        }
        liste.tamamla("k5");
        assert!(liste.ekle(hedef("yeni")), "tamamlanan slot yeniden dolmalı");
        assert_eq!(liste.kalan(), GUNLUK_HEDEF);
        assert_eq!(liste.tamam_sayisi(), 1);
    }

    #[test]
    fn yeni_gun_listeyi_sifirlar_temizle_bosaltir() {
        let mut liste = HedefListesi::default();
        liste.yeni_gun("2026-11-03");
        liste.ekle(hedef("a"));
        liste.yeni_gun("2026-11-04");
        assert_eq!(liste.gun, "2026-11-04");
        assert!(liste.bos(), "gün değişince liste sıfırlanmalı");

        liste.ekle(hedef("b"));
        liste.temizle();
        assert!(liste.bos(), "temizle listeyi boşaltmalı");
        assert_eq!(liste.gun, "2026-11-04", "temizle günü değiştirmez");
    }

    #[test]
    fn kaydet_ve_yukle_aynı_listeyi_verir() {
        let yol = std::env::temp_dir().join(format!("hedef_test_{}.json", std::process::id()));
        let yol = yol.to_string_lossy().to_string();
        let mut liste = HedefListesi::default();
        liste.yeni_gun("2026-11-03");
        liste.ekle(hedef("a"));
        liste.ekle(hedef("b"));
        liste.tamamla("a");
        liste.kaydet_yoldan(&yol);

        let geri = HedefListesi::yukle_yoldan(&yol);
        assert_eq!(geri.gun, "2026-11-03");
        assert_eq!(geri.kalan(), 1);
        assert_eq!(geri.tamam_sayisi(), 1);
        assert_eq!(geri.anahtarlar(), vec!["a".to_string(), "b".to_string()]);
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn alt_satir_adres_telefon_koordinat_icerir() {
        let h = hedef("a");
        let metin = h.alt_satir();
        assert!(metin.contains("Şehzadeler"));
        assert!(metin.contains("+90 555"));
        assert!(metin.contains("38.61150, 27.42580"));
    }
}
