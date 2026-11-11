//! Alan adı sorgusu ve maliyet tablosu (Katman 4).
//!
//! Sorgu RDAP üzerinden yapılır: kayıt kuruluşu verisi ücretsiz ve herkese
//! açıktır, API anahtarı istemez. `.com`/`.net`/`.org` gibi gTLD'lerde sonuç
//! kesindir; `.com.tr` için RDAP yayını olmadığı için "doğrulanamadı" denir —
//! uydurma bir "müsait" cevabı müşteriye yanlış fiyat söyletir.

use std::time::Duration;

/// Sorgulanabilen uzantı ve yıllık maliyeti.
pub struct Uzanti {
    pub ad: &'static str,
    /// Yıllık yenileme tahmini (TL). Kayıt firmasına göre değişir; arayüzde
    /// "tahmini" olarak gösterilir.
    pub yillik: u32,
    /// Ek not: belge zorunluluğu gibi.
    pub not: &'static str,
    /// RDAP sorgusu yapılabilir mi (.tr uzantılarında yayın yok).
    pub rdap: bool,
}

/// Arayüzde listelenen uzantılar. Fiyatlar KDV hariç yıllık tahmindir.
pub const UZANTILAR: [Uzanti; 6] = [
    Uzanti {
        ad: ".com",
        yillik: 420,
        not: "herkese açık, en yaygın",
        rdap: true,
    },
    Uzanti {
        ad: ".com.tr",
        yillik: 300,
        not: "belge/vergi no gerekir",
        rdap: false,
    },
    Uzanti {
        ad: ".net",
        yillik: 460,
        not: "alternatif",
        rdap: true,
    },
    Uzanti {
        ad: ".org",
        yillik: 400,
        not: "dernek/vakıf algısı",
        rdap: true,
    },
    Uzanti {
        ad: ".io",
        yillik: 1150,
        not: "yazılım/teknoloji",
        rdap: true,
    },
    Uzanti {
        ad: ".store",
        yillik: 280,
        not: "e-ticaret",
        rdap: true,
    },
];

/// Sorgu sonucu.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Durum {
    Musait,
    Dolu,
    /// Sorgulanamadı: uzantı desteklemiyor ya da ağ hatası. Ayrıntı metinde.
    Bilinmiyor(String),
}

impl Durum {
    pub fn etiket(&self) -> &'static str {
        match self {
            Durum::Musait => "müsait",
            Durum::Dolu => "alınmış",
            Durum::Bilinmiyor(_) => "doğrulanamadı",
        }
    }

    /// Arayüzde tek satır açıklama.
    pub fn aciklama(&self) -> String {
        match self {
            Durum::Musait => "kayıt yok · hemen alınabilir".to_string(),
            Durum::Dolu => "kayıtlı · başka ad önerilir".to_string(),
            Durum::Bilinmiyor(sebep) => sebep.clone(),
        }
    }
}

/// Tek bir alan adının durumu.
pub struct Sonuc {
    pub tam: String,
    pub uzanti: usize,
    pub durum: Durum,
}

/// Kullanıcının yazdığı addan sorgulanabilir kök üretir: küçük harf, Türkçe
/// karakterler sadeleşir, boşluk ve noktalama atılır.
pub fn kok(ad: &str) -> String {
    ad.chars()
        .filter_map(|harf| match harf {
            'İ' | 'I' | 'ı' | 'i' => Some('i'),
            'Ç' | 'ç' => Some('c'),
            'Ğ' | 'ğ' => Some('g'),
            'Ö' | 'ö' => Some('o'),
            'Ş' | 'ş' => Some('s'),
            'Ü' | 'ü' => Some('u'),
            'Â' | 'â' => Some('a'),
            harf if harf.is_ascii_alphanumeric() => Some(harf.to_ascii_lowercase()),
            harf if harf.is_alphanumeric() => Some(harf),
            _ => None,
        })
        .collect()
}

/// Alan adını sorgular.
///
/// RDAP yanıtı: 200 → kayıtlı, 404 → kayıt yok; diğer kodlar ve ağ hataları
/// "doğrulanamadı" olarak döner (kesin olmayan bilgiyle fiyat söylenmez).
pub fn sorgula(ad: &str, uzanti: &Uzanti) -> Durum {
    let kok = kok(ad);
    if kok.is_empty() {
        return Durum::Bilinmiyor("alan adı boş".to_string());
    }
    if !uzanti.rdap {
        return Durum::Bilinmiyor(format!(
            "{} için RDAP yayını yok; kayıt firmasından doğrulanmalı",
            uzanti.ad
        ));
    }
    let tam = format!("{kok}{}", uzanti.ad);
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .user_agent("NexusEnterprise/1.0 (alan adi sorgusu)")
        .build();
    let adres = format!("https://rdap.org/domain/{tam}");
    match ajan.get(&adres).call() {
        Ok(_) => Durum::Dolu,
        Err(ureq::Error::Status(404, _)) => Durum::Musait,
        Err(ureq::Error::Status(429, _)) => {
            Durum::Bilinmiyor("sorgu sınırı aşıldı, biraz sonra deneyin".to_string())
        }
        Err(ureq::Error::Status(kod, _)) => Durum::Bilinmiyor(format!("RDAP {kod} döndü")),
        Err(hata) => Durum::Bilinmiyor(format!("ağ hatası: {hata}")),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kok_turkce_karakterleri_sadelestirir() {
        assert_eq!(kok("Saruhanlı Oto Kurtarma"), "saruhanliotokurtarma");
        assert_eq!(kok("Çekiç-İş"), "cekicis");
        assert_eq!(kok(""), "");
    }

    #[test]
    fn bos_ad_sorgulanmaz() {
        let durum = sorgula("", &UZANTILAR[0]);
        assert!(matches!(durum, Durum::Bilinmiyor(_)));
        assert_eq!(durum.etiket(), "doğrulanamadı");
    }

    #[test]
    fn tr_uzantisi_rdap_yok_der() {
        let com_tr = UZANTILAR
            .iter()
            .find(|uzanti| uzanti.ad == ".com.tr")
            .expect(".com.tr listelenmeli");
        let durum = sorgula("ornek", com_tr);
        match durum {
            Durum::Bilinmiyor(sebep) => assert!(sebep.contains("RDAP")),
            diger => panic!("beklenmeyen durum: {diger:?}"),
        }
    }

    #[test]
    fn fiyatlar_tanimli_ve_makul() {
        for uzanti in UZANTILAR {
            assert!(uzanti.yillik >= 200 && uzanti.yillik <= 2000);
            assert!(uzanti.ad.starts_with('.'));
            assert!(!uzanti.not.is_empty());
        }
    }
}
