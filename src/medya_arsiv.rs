#![allow(dead_code)]

//! Hibrit medya arşivi: seçilmiş sektörler için sabit düzen, bütçe ve kalite denetimi.
//!
//! Her sektör `media/stock/<sektor>/` altında iki klasöre ayrılır:
//!
//! ```text
//! video/hero_1.mp4                    10–12 sn, 720p+, sessiz H.264, en çok 7 MB
//! photos/img_1.webp ... img_2.webp    1600 px ve üzeri
//! KAYNAK.json                        dosya başına kaynak + lisans
//! ```
//!
//! Bütçe: sektör başına 288 MB tavan, tüm arşiv için 90 GB (92.160 MB).
//! Tavan, sektör sayısı × 288 MB olarak türetilir; aşılırsa indirme durdurulur.

use std::fs;
use std::path::{Path, PathBuf};

/// Havuzun kök klasörü.
pub const KLASOR: &str = "media/stock";
/// Kapsanan sektör sayısı.
pub const SEKTOR_SAYISI: usize = crate::SEKTORLER.len();
/// Sektör başına hero video sayısı.
pub const HERO_SAYISI: usize = 2;
/// Sektör başına fotoğraf sayısı.
pub const FOTO_SAYISI: usize = 2;
/// Hero videonun en kısa süresi (saniye).
pub const HERO_MIN_SN: f64 = 10.0;
/// Hero videonun en uzun süresi (saniye).
pub const HERO_MAKS_SN: f64 = 12.0;
/// Sektör bütçesi (MB).
pub const SEKTOR_BUTCE_MB: u64 = 288;
/// Arşiv tavanı: sektör sayısı × sektör bütçesi.
pub const ARSIV_MAKS_MB: u64 = SEKTOR_BUTCE_MB * SEKTOR_SAYISI as u64;
/// Bir megabayt (MiB).
pub const MB: u64 = 1024 * 1024;
/// Arşiv tavanı (bayt).
pub const ARSIV_MAKS_BAYT: u64 = ARSIV_MAKS_MB * MB;

/// Hero videonun en küçük ölçüsü (720p).
pub const HERO_MIN_GENISLIK: u32 = 1280;
pub const HERO_MIN_YUKSEKLIK: u32 = 720;
/// Kabul edilen en düşük video bit oranı.
pub const HERO_MIN_BIT_ORANI: u64 = 1_500_000;
/// Fotoğrafın en küçük ölçüsü.
pub const FOTO_MIN_GENISLIK: u32 = 1600;
pub const FOTO_MIN_YUKSEKLIK: u32 = 1000;

/// Arşivdeki medya türü.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedyaTuru {
    Hero,
    Foto,
}

impl MedyaTuru {
    pub fn onek(self) -> &'static str {
        match self {
            Self::Hero => "hero",
            Self::Foto => "img",
        }
    }

    pub fn klasor(self) -> &'static str {
        match self {
            Self::Hero => "video",
            Self::Foto => "photos",
        }
    }

    pub fn adet(self) -> usize {
        match self {
            Self::Hero => HERO_SAYISI,
            Self::Foto => FOTO_SAYISI,
        }
    }

    pub fn uzantilar(self) -> &'static [&'static str] {
        match self {
            Self::Hero => &["mp4"],
            Self::Foto => &["webp", "jpg"],
        }
    }
}

/// Arşivdeki tek dosyanın doğrulama üst verisi.
#[derive(Clone, Debug)]
pub struct MedyaDosya {
    pub ad: String,
    pub tur: MedyaTuru,
    pub boyut_bayt: u64,
    /// Video süresi; fotoğrafta 0.
    pub sure_sn: f64,
    pub genislik: u32,
    pub yukseklik: u32,
    /// Video bit oranı; fotoğrafta 0.
    pub bit_orani: u64,
    pub lisans: String,
    pub kaynak: String,
    pub filigran: bool,
    pub sektor_uyum: bool,
}

/// Bir sektörün tüm dosyaları.
#[derive(Clone, Debug, Default)]
pub struct SektorArsiv {
    pub sektor: String,
    pub dosyalar: Vec<MedyaDosya>,
}

/// Başarılı denetimin özeti.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SektorRapor {
    pub dosya: usize,
    pub toplam_bayt: u64,
}

/// Arşivin hedef büyüklüğü.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hedef {
    pub sektor: usize,
    pub hero: usize,
    pub foto: usize,
    pub ust_sinir_bayt: u64,
}

/// Tüm sektörler için beklenen dosya sayıları.
pub fn hedef() -> Hedef {
    Hedef {
        sektor: SEKTOR_SAYISI,
        hero: SEKTOR_SAYISI * HERO_SAYISI,
        foto: SEKTOR_SAYISI * FOTO_SAYISI,
        ust_sinir_bayt: ARSIV_MAKS_BAYT,
    }
}

/// `hero_3` gibi uzantısız kök ad.
pub fn kok_ad(tur: MedyaTuru, sira: usize) -> String {
    format!("{}_{sira}", tur.onek())
}

/// Sektörün kök klasörü: `media/stock/kuafor`.
pub fn sektor_klasoru(kok: &Path, sektor: &str) -> PathBuf {
    kok.join(sektor)
}

/// Tek dosyanın yolu: `media/stock/kuafor/video/hero_1.mp4`.
pub fn dosya_yolu(kok: &Path, sektor: &str, tur: MedyaTuru, sira: usize) -> PathBuf {
    let uzanti = tur.uzantilar()[0];
    sektor_klasoru(kok, sektor)
        .join(tur.klasor())
        .join(format!("{}.{uzanti}", kok_ad(tur, sira)))
}

fn dosyanin_koku(ad: &str) -> Option<&str> {
    ad.rsplit_once('.').map(|(kok, _)| kok)
}

/// İndirme aşamasındaki kalite filtresi: uygun olmayan dosya yazılmaz.
pub fn indirme_filtresi(dosya: &MedyaDosya) -> Result<(), String> {
    if dosya.lisans.trim().is_empty() {
        return Err(format!("{}: lisans yok", dosya.ad));
    }
    if dosya.kaynak.trim().is_empty() {
        return Err(format!("{}: kaynak yok", dosya.ad));
    }
    if dosya.filigran {
        return Err(format!("{}: filigranlı", dosya.ad));
    }
    if !dosya.sektor_uyum {
        return Err(format!("{}: sektörle uyumsuz", dosya.ad));
    }
    if dosya.boyut_bayt == 0 {
        return Err(format!("{}: boş dosya", dosya.ad));
    }
    if dosya.boyut_bayt > 7 * MB {
        return Err(format!("{}: video 7 MB sınırını aşıyor", dosya.ad));
    }
    match dosya.tur {
        MedyaTuru::Hero => {
            if dosya.sure_sn < HERO_MIN_SN || dosya.sure_sn > HERO_MAKS_SN {
                return Err(format!("{}: süre 10–12 sn değil", dosya.ad));
            }
            if dosya.genislik < HERO_MIN_GENISLIK || dosya.yukseklik < HERO_MIN_YUKSEKLIK {
                return Err(format!("{}: çözünürlük 720p altında", dosya.ad));
            }
            if dosya.bit_orani < HERO_MIN_BIT_ORANI {
                return Err(format!("{}: bit oranı düşük", dosya.ad));
            }
        }
        MedyaTuru::Foto => {
            if dosya.genislik < FOTO_MIN_GENISLIK || dosya.yukseklik < FOTO_MIN_YUKSEKLIK {
                return Err(format!("{}: fotoğraf çözünürlüğü düşük", dosya.ad));
            }
        }
    }
    Ok(())
}

/// Bir sektörü adlandırma, sayı, kalite ve bütçe açısından denetler.
pub fn denetle(arsiv: &SektorArsiv) -> Result<SektorRapor, Vec<String>> {
    let mut hatalar = Vec::new();
    let mut eslesen: Vec<usize> = Vec::with_capacity(arsiv.dosyalar.len());

    for tur in [MedyaTuru::Hero, MedyaTuru::Foto] {
        for sira in 1..=tur.adet() {
            let kok = kok_ad(tur, sira);
            let adaylar: Vec<usize> = arsiv
                .dosyalar
                .iter()
                .enumerate()
                .filter(|(_, d)| d.tur == tur && dosyanin_koku(&d.ad) == Some(kok.as_str()))
                .map(|(sira, _)| sira)
                .collect();
            match adaylar.as_slice() {
                [] => hatalar.push(format!("{kok}: eksik")),
                [sira] => {
                    eslesen.push(*sira);
                    let dosya = &arsiv.dosyalar[*sira];
                    if !tur
                        .uzantilar()
                        .iter()
                        .any(|u| dosya.ad.ends_with(&format!(".{u}")))
                    {
                        hatalar.push(format!("{}: uzantı uygun değil", dosya.ad));
                    }
                    if let Err(hata) = indirme_filtresi(dosya) {
                        hatalar.push(hata);
                    }
                }
                _ => hatalar.push(format!("{kok}: birden fazla dosya")),
            }
        }
    }

    for (sira, dosya) in arsiv.dosyalar.iter().enumerate() {
        if !eslesen.contains(&sira) {
            hatalar.push(format!("{}: beklenmeyen dosya", dosya.ad));
        }
    }

    let toplam_bayt: u64 = arsiv.dosyalar.iter().map(|d| d.boyut_bayt).sum();
    let butce = SEKTOR_BUTCE_MB * MB;
    if toplam_bayt > butce {
        hatalar.push(format!(
            "{}: sektör toplamı {} MB, bütçe {SEKTOR_BUTCE_MB} MB",
            arsiv.sektor,
            toplam_bayt / MB
        ));
    }

    if hatalar.is_empty() {
        Ok(SektorRapor {
            dosya: arsiv.dosyalar.len(),
            toplam_bayt,
        })
    } else {
        Err(hatalar)
    }
}

/// Küresel tavan: 90 GB aşılırsa pipeline durur; kalan MB döner.
pub fn toplam_kota_denetle(toplam_bayt: u64) -> Result<u64, String> {
    if toplam_bayt > ARSIV_MAKS_BAYT {
        return Err(format!(
            "Arşiv {} MB tavanını aştı ({} MB): indirme ve dönüştürme pipeline'ı durduruldu",
            ARSIV_MAKS_MB,
            toplam_bayt / MB
        ));
    }
    Ok((ARSIV_MAKS_BAYT - toplam_bayt) / MB)
}

/// Diskteki mevcut arşivin özeti.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MevcutDurum {
    pub sektor: usize,
    pub hero: usize,
    pub foto: usize,
    pub toplam_bayt: u64,
}

/// `media/stock` altındaki arşiv dosyalarını sayar.
pub fn mevcut_durum(kok: &Path) -> MevcutDurum {
    let mut durum = MevcutDurum::default();
    let Ok(girisler) = fs::read_dir(kok) else {
        return durum;
    };
    let mut sektorler: Vec<PathBuf> = girisler
        .filter_map(Result::ok)
        .map(|giris| giris.path())
        .filter(|yol| yol.is_dir())
        .collect();
    sektorler.sort();
    for sektor in sektorler {
        durum.sektor += 1;
        for tur in [MedyaTuru::Hero, MedyaTuru::Foto] {
            let Ok(liste) = fs::read_dir(sektor.join(tur.klasor())) else {
                continue;
            };
            for yol in liste.filter_map(Result::ok).map(|giris| giris.path()) {
                if !yol.is_file() {
                    continue;
                }
                let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
                if !ad.starts_with(&format!("{}_", tur.onek())) {
                    continue;
                }
                match tur {
                    MedyaTuru::Hero => durum.hero += 1,
                    MedyaTuru::Foto => durum.foto += 1,
                }
                durum.toplam_bayt += yol.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    durum
}

#[cfg(test)]
mod testler {
    use super::*;

    fn mb(deger: u64) -> u64 {
        deger * MB
    }

    fn dosya(tur: MedyaTuru, sira: usize, boyut: u64) -> MedyaDosya {
        let ad = match tur {
            MedyaTuru::Hero => format!("hero_{sira}.mp4"),
            MedyaTuru::Foto => format!("img_{sira}.webp"),
        };
        MedyaDosya {
            ad,
            tur,
            boyut_bayt: boyut,
            sure_sn: if tur == MedyaTuru::Hero { 11.0 } else { 0.0 },
            genislik: if tur == MedyaTuru::Foto { 2000 } else { 1280 },
            yukseklik: if tur == MedyaTuru::Foto { 1200 } else { 720 },
            bit_orani: if tur == MedyaTuru::Hero { 3_000_000 } else { 0 },
            lisans: "CC0".to_string(),
            kaynak: "https://commons.wikimedia.org/".to_string(),
            filigran: false,
            sektor_uyum: true,
        }
    }

    fn tam_sektor(hero_boyut: u64, foto_boyut: u64) -> SektorArsiv {
        let mut dosyalar = Vec::new();
        for sira in 1..=HERO_SAYISI {
            dosyalar.push(dosya(MedyaTuru::Hero, sira, hero_boyut));
        }
        for sira in 1..=FOTO_SAYISI {
            dosyalar.push(dosya(MedyaTuru::Foto, sira, foto_boyut));
        }
        SektorArsiv {
            sektor: "kuafor".to_string(),
            dosyalar,
        }
    }

    #[test]
    fn yollar_ve_adlar_duzenli() {
        let kok = Path::new("media/stock");
        assert_eq!(
            dosya_yolu(kok, "kuafor", MedyaTuru::Hero, 1),
            PathBuf::from("media/stock/kuafor/video/hero_1.mp4")
        );
        assert_eq!(
            dosya_yolu(kok, "kuafor", MedyaTuru::Foto, 2),
            PathBuf::from("media/stock/kuafor/photos/img_2.webp")
        );
        assert_eq!(kok_ad(MedyaTuru::Hero, 1), "hero_1");
    }

    #[test]
    fn tam_sektor_butce_icinde_gecer() {
        let rapor = denetle(&tam_sektor(mb(7), mb(2))).expect("70 MB bütçeye sığmalı");
        assert_eq!(rapor.dosya, HERO_SAYISI + FOTO_SAYISI);
        assert_eq!(rapor.toplam_bayt, mb(18));
    }

    #[test]
    fn eksik_veya_fazla_dosya_reddedilir() {
        let mut arsiv = tam_sektor(mb(5), mb(2));
        arsiv.dosyalar.retain(|d| d.ad != "hero_1.mp4");
        let hata = denetle(&arsiv).expect_err("eksik hero reddedilmeli");
        assert!(hata.iter().any(|h| h.contains("hero_1")), "{hata:?}");

        let mut arsiv = tam_sektor(mb(5), mb(2));
        arsiv.dosyalar.push(dosya(MedyaTuru::Hero, 1, 0));
        assert!(denetle(&arsiv).is_err(), "çift dosya reddedilmeli");
    }

    #[test]
    fn filigran_ve_lisanssiz_medya_indirilmez() {
        let mut filigranli = dosya(MedyaTuru::Hero, 1, mb(5));
        filigranli.filigran = true;
        assert!(indirme_filtresi(&filigranli)
            .unwrap_err()
            .contains("filigran"));

        let mut lisanssiz = dosya(MedyaTuru::Foto, 1, mb(1));
        lisanssiz.lisans.clear();
        assert!(indirme_filtresi(&lisanssiz).unwrap_err().contains("lisans"));

        let mut uyumsuz = dosya(MedyaTuru::Hero, 1, mb(5));
        uyumsuz.sektor_uyum = false;
        assert!(indirme_filtresi(&uyumsuz).unwrap_err().contains("uyumsuz"));
    }

    #[test]
    fn sure_cozunurluk_ve_boyut_siniri_korunur() {
        let mut kisa = dosya(MedyaTuru::Hero, 2, mb(5));
        kisa.sure_sn = 8.0;
        assert!(indirme_filtresi(&kisa).unwrap_err().contains("10–12"));

        let mut uzun = dosya(MedyaTuru::Hero, 2, mb(5));
        uzun.sure_sn = 15.0;
        assert!(indirme_filtresi(&uzun).unwrap_err().contains("10–12"));

        let mut dusuk = dosya(MedyaTuru::Hero, 2, mb(5));
        dusuk.yukseklik = 480;
        assert!(indirme_filtresi(&dusuk).unwrap_err().contains("720p"));

        let agir = dosya(MedyaTuru::Hero, 2, mb(8));
        assert!(indirme_filtresi(&agir).unwrap_err().contains("7 MB"));
    }

    #[test]
    fn sektor_butcesi_asilinca_reddedilir() {
        assert!(
            denetle(&tam_sektor(mb(287), mb(2))).is_err(),
            "291 MB bütçeyi aşar"
        );
    }

    #[test]
    fn arsiv_tavani_pipeline_durdurur() {
        assert_eq!(toplam_kota_denetle(ARSIV_MAKS_BAYT), Ok(0));
        let hata = toplam_kota_denetle(ARSIV_MAKS_BAYT + 1).unwrap_err();
        assert!(hata.contains("durduruldu"), "{hata}");
        assert_eq!(SEKTOR_BUTCE_MB * SEKTOR_SAYISI as u64, ARSIV_MAKS_MB);
    }

    #[test]
    fn mevcut_durum_dosyalari_sayar() {
        let kok = std::env::temp_dir().join(format!("nexus_arsiv_{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        let sektor = kok.join("kuafor");
        for (tur, adet) in [(MedyaTuru::Hero, 2), (MedyaTuru::Foto, 3)] {
            let klasor = sektor.join(tur.klasor());
            fs::create_dir_all(&klasor).unwrap();
            for sira in 1..=adet {
                fs::write(klasor.join(format!("{}.mp4", kok_ad(tur, sira))), [0u8; 8]).unwrap();
            }
        }
        fs::write(sektor.join("photos/afis.png"), [0u8; 8]).unwrap();
        let durum = mevcut_durum(&kok);
        assert_eq!(durum.sektor, 1);
        assert_eq!((durum.hero, durum.foto), (2, 3));
        assert_eq!(durum.toplam_bayt, 5 * 8);
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    #[ignore]
    fn arsiv_raporu_yazdirir() {
        let durum = mevcut_durum(Path::new(KLASOR));
        println!(
            "mevcut: {} sektör · {} hero · {} foto · {} MB · kalan bütçe {} MB",
            durum.sektor,
            durum.hero,
            durum.foto,
            durum.toplam_bayt / MB,
            toplam_kota_denetle(durum.toplam_bayt).unwrap_or(0)
        );
        assert!(durum.toplam_bayt <= ARSIV_MAKS_BAYT);
    }
}
