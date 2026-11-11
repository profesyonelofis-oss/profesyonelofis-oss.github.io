//! Gelir katmanı: hizmet kataloğu, paketler, abonelik takibi ve gelir panosu.
//!
//! Satılan her hizmet tek katalogda durur; müşteriye paket ya da tek tek hizmet
//! atanır. Katman yalnız yerel veriyle çalışır (`gelir.json`): ödeme kuruluşu,
//! reklam API'si veya dış servis bağlanmaz; amaç neyin, kime, kaça satıldığını
//! ve aylık tekrarlayan geliri tek ekranda tutmaktır.

use crate::*;
use eframe::egui;
use std::fs;

/// Abonelik ve tekliflerin yazıldığı dosya.
pub const DOSYA: &str = "gelir.json";
const SURUM: u32 = 1;

/// Gelir hedefleri: (aylık TL hedefi, etiket).
/// Kur dalgalandığı için hedef TL olarak tutulur; etiket yalnız yön gösterir.
pub const HEDEFLER: [(u32, &str); 3] = [
    (100_000, "2.500 $"),
    (200_000, "5.000 $"),
    (400_000, "10.000 $"),
];

/// Katalogdaki tek hizmet.
pub struct Hizmet {
    pub kod: &'static str,
    pub ad: &'static str,
    pub grup: &'static str,
    /// Önerilen aylık fiyat (TL). 0 ise hizmet yalnız tek seferlik satılır.
    pub aylik: u32,
    /// Tek seferlik ücret (TL); 0 ise yok.
    pub tek_sefer: u32,
    pub zorluk: &'static str,
    pub aciklama: &'static str,
    /// Uygulamada altyapısı hazır mı (katalogda işaret olarak gösterilir).
    pub hazir: bool,
}

/// Satılabilir hizmetlerin tamamı. Fiyatlar öneridir, teklifte değiştirilebilir.
pub const KATALOG: &[Hizmet] = &[
    // --- Profil ------------------------------------------------------------
    Hizmet {
        kod: "gbp",
        ad: "Google profil yönetimi",
        grup: "Profil",
        aylik: 1_800,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Post, foto, çalışma saati ve soru-cevap yönetimi.",
        hazir: true,
    },
    Hizmet {
        kod: "yorum",
        ad: "Yorum toplama ve yanıtlama",
        grup: "Profil",
        aylik: 900,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Müşteriden yorum isteme, gelen yorumu yanıtlama.",
        hazir: true,
    },
    Hizmet {
        kod: "nap",
        ad: "NAP / citation temizliği",
        grup: "Profil",
        aylik: 600,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Ad-adres-telefon tutarlılığı, rehber kayıtları.",
        hazir: true,
    },
    Hizmet {
        kod: "rapor",
        ad: "Aylık görünürlük raporu",
        grup: "Profil",
        aylik: 400,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Yanıt, lead ve görüntüleme özeti; müşteriye tek sayfa.",
        hazir: false,
    },
    // --- Site --------------------------------------------------------------
    Hizmet {
        kod: "kiralik_site",
        ad: "Site kiralama",
        grup: "Site",
        aylik: 2_000,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Kurulum yerine aylık; site yayında kalır, mülkiyet sende.",
        hazir: true,
    },
    Hizmet {
        kod: "hosting",
        ad: "Hosting + SSL + yedek",
        grup: "Site",
        aylik: 700,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "Alan adı, sertifika, çalışırlık kontrolü, günlük yedek.",
        hazir: true,
    },
    Hizmet {
        kod: "menu_qr",
        ad: "Dijital menü / QR",
        grup: "Site",
        aylik: 400,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Restoran, kafe ve kuaför için QR menü/fiyat listesi.",
        hazir: false,
    },
    // --- Mesaj -------------------------------------------------------------
    Hizmet {
        kod: "resepsiyonist",
        ad: "WhatsApp AI resepsiyonist",
        grup: "Mesaj",
        aylik: 2_500,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "Gelen mesajı yanıtlar, niyeti ayırır, randevuya yönlendirir.",
        hazir: true,
    },
    Hizmet {
        kod: "randevu",
        ad: "Randevu ve hatırlatma",
        grup: "Mesaj",
        aylik: 800,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "Randevu alma, hatırlatma ve gelmeyen müşteri takibi.",
        hazir: false,
    },
    Hizmet {
        kod: "sms",
        ad: "E-posta / SMS kampanya",
        grup: "Mesaj",
        aylik: 800,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "Eski müşteriye dönüş kampanyası; izinli liste şart.",
        hazir: false,
    },
    // --- Görünürlük --------------------------------------------------------
    Hizmet {
        kod: "yerel_seo",
        ad: "Yerel SEO",
        grup: "Görünürlük",
        aylik: 4_500,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "Hizmet/ilçe sayfaları, citation, aylık sıralama raporu.",
        hazir: false,
    },
    Hizmet {
        kod: "geo",
        ad: "GEO / AI'da görünme",
        grup: "Görünürlük",
        aylik: 7_500,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "ChatGPT, Gemini ve AI özetlerinde adının geçmesi.",
        hazir: false,
    },
    Hizmet {
        kod: "rakip",
        ad: "Rakip / sıralama raporu",
        grup: "Görünürlük",
        aylik: 600,
        tek_sefer: 0,
        zorluk: "kolay",
        aciklama: "Aynı bölgedeki ilk sayfa oyuncularının puan/yorum dağılımı.",
        hazir: false,
    },
    // --- Reklam ------------------------------------------------------------
    Hizmet {
        kod: "google_ads",
        ad: "Google Ads yönetimi",
        grup: "Reklam",
        aylik: 3_000,
        tek_sefer: 0,
        zorluk: "zor",
        aciklama: "Kampanya kurulumu ve optimizasyon; reklam bütçesi ayrı.",
        hazir: false,
    },
    Hizmet {
        kod: "sosyal",
        ad: "Sosyal medya yönetimi",
        grup: "Reklam",
        aylik: 3_500,
        tek_sefer: 0,
        zorluk: "orta",
        aciklama: "Aylık içerik takvimi, paylaşım ve yorum yanıtı.",
        hazir: false,
    },
    // --- Tek seferlik ------------------------------------------------------
    Hizmet {
        kod: "site_kurulum",
        ad: "Site kurulumu",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 8_500,
        zorluk: "kolay",
        aciklama: "Demo + yayın + WhatsApp bağlantısı; tek seferlik.",
        hazir: true,
    },
    Hizmet {
        kod: "gbp_kurulum",
        ad: "Google profil kurulumu",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 3_000,
        zorluk: "kolay",
        aciklama: "Kategori, hizmet, saat ve fotoğrafların tam optimizasyonu.",
        hazir: false,
    },
    Hizmet {
        kod: "gbp_kurtarma",
        ad: "Google profil kurtarma",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 6_000,
        zorluk: "kolay",
        aciklama: "Askıya alınan/doğrulaması düşen profilin geri açılması.",
        hazir: false,
    },
    Hizmet {
        kod: "geo_audit",
        ad: "AI görünürlük auditi",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 4_000,
        zorluk: "kolay",
        aciklama: "AI asistanlarında adının geçip geçmediğinin ölçümü.",
        hazir: false,
    },
    Hizmet {
        kod: "seo_audit",
        ad: "Yerel SEO auditi",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 2_500,
        zorluk: "kolay",
        aciklama: "Eksik listesi ve yol haritası; satış görüşmesinde kullanılır.",
        hazir: true,
    },
    Hizmet {
        kod: "ads_kurulum",
        ad: "Google Ads kurulumu",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 3_000,
        zorluk: "orta",
        aciklama: "Hesap, dönüşüm takibi ve ilk kampanyanın kurulması.",
        hazir: false,
    },
    Hizmet {
        kod: "cekim",
        ad: "Fotoğraf / video çekimi",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 10_000,
        zorluk: "kolay",
        aciklama: "Yerinde çekim; taşerona verilir, sen yönetirsin.",
        hazir: false,
    },
    Hizmet {
        kod: "egitim",
        ad: "Eğitim / danışmanlık (saat)",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 2_000,
        zorluk: "kolay",
        aciklama: "Müşteriye veya ajansa verilen saatlik eğitim.",
        hazir: false,
    },
    Hizmet {
        kod: "lead",
        ad: "Lead satışı (adet)",
        grup: "Tek seferlik",
        aylik: 0,
        tek_sefer: 350,
        zorluk: "kolay",
        aciklama: "Nitelikli lead başına ücret; defterdeki kayıtlardan.",
        hazir: true,
    },
];

/// Hazır paket: katalogdan seçilmiş hizmetler + tek aylık fiyat.
pub struct Paket {
    pub kod: &'static str,
    pub ad: &'static str,
    pub fiyat: u32,
    pub hizmetler: &'static [&'static str],
}

pub const PAKETLER: [Paket; 4] = [
    Paket {
        kod: "baslangic",
        ad: "Başlangıç",
        fiyat: 2_500,
        hizmetler: &["gbp", "yorum", "rapor"],
    },
    Paket {
        kod: "buyume",
        ad: "Büyüme",
        fiyat: 5_000,
        hizmetler: &["gbp", "yorum", "rapor", "kiralik_site", "hosting", "nap"],
    },
    Paket {
        kod: "pro",
        ad: "Pro",
        fiyat: 9_000,
        hizmetler: &[
            "gbp",
            "yorum",
            "rapor",
            "kiralik_site",
            "hosting",
            "nap",
            "yerel_seo",
            "resepsiyonist",
        ],
    },
    Paket {
        kod: "gorunurluk",
        ad: "AI görünürlük",
        fiyat: 7_500,
        hizmetler: &["geo", "rakip", "geo_audit"],
    },
];

/// Katalogda koda göre hizmet.
pub fn hizmet(kod: &str) -> Option<&'static Hizmet> {
    KATALOG.iter().find(|hizmet| hizmet.kod == kod)
}

/// Kodla paket.
pub fn paket(kod: &str) -> Option<&'static Paket> {
    PAKETLER.iter().find(|paket| paket.kod == kod)
}

/// Bir müşterinin gelir kaydı.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MusteriGelir {
    pub anahtar: String,
    pub ad: String,
    /// Seçili paket kodu; doluysa aylık tutar paket fiyatıdır.
    #[serde(default)]
    pub paket: Option<String>,
    /// Paketin anlaşılan aylık fiyatı (indirim uygulanmış olabilir).
    #[serde(default)]
    pub paket_fiyati: u32,
    /// Paket yokken tek tek seçilen hizmet kodları.
    #[serde(default)]
    pub hizmetler: Vec<String>,
    /// Yapılmış tek seferlik işler (kurulum, audit, çekim...).
    #[serde(default)]
    pub tek_seferler: Vec<String>,
    /// Son faturanın kesildiği gün (`YYYY-MM-DD`).
    #[serde(default)]
    pub son_fatura: Option<String>,
    /// Ödemenin alındığı ay (`YYYY-MM`); `None` ise bu ay tahsil edilmedi.
    #[serde(default)]
    pub odendi: Option<String>,
    #[serde(default)]
    pub notlar: String,
}

impl MusteriGelir {
    /// Aylık tekrarlayan tutar.
    pub fn aylik(&self) -> u32 {
        if self.paket.is_some() {
            return self.paket_fiyati;
        }
        self.hizmetler
            .iter()
            .filter_map(|kod| hizmet(kod))
            .map(|hizmet| hizmet.aylik)
            .sum()
    }

    /// Yapılmış tek seferlik işlerin toplamı.
    pub fn tek_sefer_toplam(&self) -> u32 {
        self.tek_seferler
            .iter()
            .filter_map(|kod| hizmet(kod))
            .map(|hizmet| hizmet.tek_sefer)
            .sum()
    }

    /// Teklifte gösterilecek hizmet adları.
    pub fn kapsam(&self) -> Vec<&'static str> {
        if let Some(paket) = self.paket.as_deref().and_then(paket) {
            return paket
                .hizmetler
                .iter()
                .filter_map(|kod| hizmet(kod))
                .map(|h| h.ad)
                .collect();
        }
        self.hizmetler
            .iter()
            .filter_map(|kod| hizmet(kod))
            .map(|hizmet| hizmet.ad)
            .collect()
    }
}

/// Otomatik öneri için gereken alanlar (defter kaydından doldurulur).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OneriGirdi {
    /// İşletmenin bir sitesi var mı?
    pub site_var: bool,
    /// Defterdeki profil puanı (0-100).
    pub puan: u8,
    /// Ad, adres ya da telefon eksik/geçersiz mi?
    pub profil_eksik: bool,
}

/// Defter kaydından öneri girdisi üretir.
pub fn oneri_girdi(kayit: &musteriler::Musteri) -> OneriGirdi {
    OneriGirdi {
        site_var: !kayit.site.trim().is_empty(),
        puan: kayit.puan,
        profil_eksik: kayit.ad.trim().is_empty()
            || kayit.adres.trim().is_empty()
            || !telefon_gecerli(kayit.telefon.trim()),
    }
}

/// Eksiklere göre seçilen paket ve tek seferlik işler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Oneri {
    /// Önerilen paketin kodu (PAKETLER içinden).
    pub paket: &'static str,
    /// Paketin yanında önerilen tek seferlik işler.
    pub tek_seferler: Vec<&'static str>,
    /// Müşteriye de yazılan tek cümlelik gerekçe.
    pub gerekce: String,
}

impl Oneri {
    pub fn aylik(&self) -> u32 {
        paket(self.paket).map(|paket| paket.fiyat).unwrap_or(0)
    }

    pub fn tek_sefer_toplam(&self) -> u32 {
        self.tek_seferler
            .iter()
            .filter_map(|kod| hizmet(kod))
            .map(|hizmet| hizmet.tek_sefer)
            .sum()
    }

    /// Tek satırlık özet: "Büyüme paketi · 5.000 TL/ay + Site kurulumu · 8.500 TL".
    pub fn satir(&self) -> String {
        let mut metin = match paket(self.paket) {
            Some(paket) => format!("{} paketi · {} TL/ay", paket.ad, fiyat_yaz(paket.fiyat)),
            None => String::new(),
        };
        let ekler: Vec<String> = self
            .tek_seferler
            .iter()
            .filter_map(|kod| hizmet(kod))
            .map(|hizmet| format!("{} · {} TL", hizmet.ad, fiyat_yaz(hizmet.tek_sefer)))
            .collect();
        if !ekler.is_empty() {
            if !metin.is_empty() {
                metin.push_str(" + ");
            }
            metin.push_str(&ekler.join(" + "));
        }
        metin
    }
}

/// Kurallar tek yerde: profil eksikse önce kayıt tamamlanır; sitesi olmayana
/// site paketi; puanı 60'ın altındakine profil/yorum paketi; üstüne görünürlük.
pub fn oneri(girdi: OneriGirdi) -> Oneri {
    let paket_kodu = if girdi.profil_eksik {
        "baslangic"
    } else if !girdi.site_var {
        "buyume"
    } else if girdi.puan < 60 {
        "baslangic"
    } else {
        "pro"
    };
    let mut tek_seferler: Vec<&'static str> = Vec::new();
    if !girdi.site_var {
        tek_seferler.push("site_kurulum");
    }
    if girdi.profil_eksik {
        tek_seferler.push("gbp_kurulum");
    }
    if girdi.puan > 0 && girdi.puan < 60 {
        tek_seferler.push("seo_audit");
    }
    let gerekce = if girdi.profil_eksik {
        "Google kaydınızda ad, adres ya da telefon eksik; önce kaydı tamamlıyoruz.".to_string()
    } else if !girdi.site_var {
        format!(
            "Google profiliniz {}/100 ve siteniz yok; site ile birlikte başlıyoruz.",
            girdi.puan
        )
    } else if girdi.puan < 60 {
        format!(
            "Google profiliniz {}/100; yorum ve profil yönetimiyle yükseltiyoruz.",
            girdi.puan
        )
    } else {
        format!(
            "Google profiliniz {}/100; görünürlüğü büyütmek için paket öneriyoruz.",
            girdi.puan
        )
    };
    Oneri {
        paket: paket_kodu,
        tek_seferler,
        gerekce,
    }
}

/// Hazırlanmış teklif: metin, tutarlar ve gönderim durumu saklanır.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Teklif {
    pub anahtar: String,
    pub ad: String,
    /// Teklifin hazırlandığı gün (`YYYY-MM-DD`).
    pub gun: String,
    #[serde(default)]
    pub paket: Option<String>,
    #[serde(default)]
    pub aylik: u32,
    #[serde(default)]
    pub tek_sefer: u32,
    #[serde(default)]
    pub ozet: String,
    #[serde(default)]
    pub metin: String,
    /// Müşteriye gönderildi olarak işaretlendi mi?
    #[serde(default)]
    pub gonderildi: bool,
}

/// Müşterinin eksiklerine göre hazır teklif üretir.
pub fn teklif_olustur(anahtar: &str, ad: &str, girdi: OneriGirdi, gun: &str) -> Teklif {
    let oneri = oneri(girdi);
    let kayit = MusteriGelir {
        anahtar: anahtar.to_string(),
        ad: ad.trim().to_string(),
        paket: Some(oneri.paket.to_string()),
        paket_fiyati: oneri.aylik(),
        hizmetler: Vec::new(),
        tek_seferler: oneri
            .tek_seferler
            .iter()
            .map(|kod| kod.to_string())
            .collect(),
        son_fatura: None,
        odendi: None,
        notlar: String::new(),
    };
    let mut metin = teklif_metni(ad, &kayit);
    metin.push_str(&format!("\n\n{}", oneri.gerekce));
    Teklif {
        anahtar: anahtar.to_string(),
        ad: ad.trim().to_string(),
        gun: gun.to_string(),
        paket: kayit.paket.clone(),
        aylik: kayit.aylik(),
        tek_sefer: kayit.tek_sefer_toplam(),
        ozet: oneri.satir(),
        metin,
        gonderildi: false,
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Dosya {
    #[serde(default)]
    surum: u32,
    #[serde(default)]
    kayitlar: Vec<MusteriGelir>,
    #[serde(default)]
    teklifler: Vec<Teklif>,
    /// Tekliflerin en son kendiliğinden hazırlandığı gün (`YYYY-MM-DD`).
    #[serde(default)]
    son_otomatik: String,
}

/// Gelir kayıtlarının kalıcı tutulduğu yer.
#[derive(Default)]
pub struct Gelir {
    dosya: Dosya,
}

impl Gelir {
    pub fn yukle() -> Self {
        let metin = fs::read_to_string(DOSYA).unwrap_or_default();
        Self::metinden(&metin)
    }

    pub fn metinden(metin: &str) -> Self {
        let mut dosya: Dosya = serde_json::from_str(metin).unwrap_or_default();
        dosya.surum = SURUM;
        Self { dosya }
    }

    pub fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string_pretty(&self.dosya) {
            let _ = fs::write(DOSYA, metin);
        }
    }

    pub fn kayitlar(&self) -> &[MusteriGelir] {
        &self.dosya.kayitlar
    }

    pub fn kayit(&self, anahtar: &str) -> Option<&MusteriGelir> {
        self.dosya
            .kayitlar
            .iter()
            .find(|kayit| kayit.anahtar == anahtar)
    }

    fn kayit_mut(&mut self, anahtar: &str, ad: &str) -> &mut MusteriGelir {
        if let Some(sira) = self
            .dosya
            .kayitlar
            .iter()
            .position(|kayit| kayit.anahtar == anahtar)
        {
            let kayit = &mut self.dosya.kayitlar[sira];
            kayit.ad = ad.to_string();
            return kayit;
        }
        self.dosya.kayitlar.push(MusteriGelir {
            anahtar: anahtar.to_string(),
            ad: ad.to_string(),
            ..MusteriGelir::default()
        });
        self.dosya.kayitlar.last_mut().expect("az önce eklendi")
    }

    /// Paketi uygular: tek tek seçilmiş hizmetler temizlenir.
    pub fn paket_uygula(&mut self, anahtar: &str, ad: &str, paket_kodu: &str) {
        let Some(secili) = paket(paket_kodu) else {
            return;
        };
        let kayit = self.kayit_mut(anahtar, ad);
        kayit.paket = Some(secili.kod.to_string());
        kayit.paket_fiyati = secili.fiyat;
        kayit.hizmetler.clear();
    }

    pub fn paket_kaldir(&mut self, anahtar: &str, ad: &str) {
        let kayit = self.kayit_mut(anahtar, ad);
        kayit.paket = None;
        kayit.paket_fiyati = 0;
    }

    /// Tek hizmeti ekler/çıkarır. Paket varsa paket bozulur ve toplam
    /// katalog fiyatlarından hesaplanır.
    pub fn hizmet_ayarla(&mut self, anahtar: &str, ad: &str, kod: &str, ekle: bool) {
        let kayit = self.kayit_mut(anahtar, ad);
        kayit.paket = None;
        kayit.paket_fiyati = 0;
        if ekle {
            if !kayit.hizmetler.iter().any(|kalan| kalan == kod) {
                kayit.hizmetler.push(kod.to_string());
            }
        } else {
            kayit.hizmetler.retain(|kalan| kalan != kod);
        }
    }

    /// Tek seferlik işi işaretler/işareti kaldırır.
    pub fn tek_sefer_ayarla(&mut self, anahtar: &str, ad: &str, kod: &str, ekle: bool) {
        let kayit = self.kayit_mut(anahtar, ad);
        if ekle {
            if !kayit.tek_seferler.iter().any(|kalan| kalan == kod) {
                kayit.tek_seferler.push(kod.to_string());
            }
        } else {
            kayit.tek_seferler.retain(|kalan| kalan != kod);
        }
    }

    /// Bugünün tarihiyle fatura işaretler.
    /// Bu ayın ödemesi alındı olarak işaretler.
    pub fn odeme_isle(&mut self, anahtar: &str, ad: &str, ay: &str) {
        let kayit = self.kayit_mut(anahtar, ad);
        kayit.odendi = Some(ay.to_string());
    }

    /// Bu ayın ödemesi alındı mı?
    pub fn odendi(&self, anahtar: &str, ay: &str) -> bool {
        self.kayit(anahtar)
            .and_then(|kayit| kayit.odendi.as_deref())
            == Some(ay)
    }

    pub fn fatura_kes(&mut self, anahtar: &str, ad: &str) {
        let bugun = bugun_anahtari();
        let kayit = self.kayit_mut(anahtar, ad);
        kayit.son_fatura = Some(bugun);
    }

    pub fn aylik(&self, anahtar: &str) -> u32 {
        self.kayit(anahtar).map(MusteriGelir::aylik).unwrap_or(0)
    }

    /// Aylık tekrarlayan gelir (MRR).
    pub fn mrr(&self) -> u32 {
        self.dosya.kayitlar.iter().map(MusteriGelir::aylik).sum()
    }

    /// Aylık ödemesi olan müşteri sayısı.
    pub fn abone_sayisi(&self) -> usize {
        self.dosya
            .kayitlar
            .iter()
            .filter(|kayit| kayit.aylik() > 0)
            .count()
    }

    /// Hazırlanmış teklifler.
    pub fn teklifler(&self) -> &[Teklif] {
        &self.dosya.teklifler
    }

    pub fn teklif_sayisi(&self) -> usize {
        self.dosya.teklifler.len()
    }

    pub fn teklif(&self, anahtar: &str, gun: &str) -> Option<&Teklif> {
        self.dosya
            .teklifler
            .iter()
            .find(|teklif| teklif.anahtar == anahtar && teklif.gun == gun)
    }

    /// Teklifi ekler. Aynı müşteri için aynı gün gönderilmemiş teklif varsa
    /// üzerine yazar; yeni kayıt eklendiyse `true` döner.
    pub fn teklif_ekle(&mut self, teklif: Teklif) -> bool {
        if let Some(eski) = self.dosya.teklifler.iter_mut().find(|eski| {
            eski.anahtar == teklif.anahtar && eski.gun == teklif.gun && !eski.gonderildi
        }) {
            *eski = teklif;
            return false;
        }
        self.dosya.teklifler.push(teklif);
        let sinir = self.dosya.teklifler.len().saturating_sub(300);
        if sinir > 0 {
            self.dosya.teklifler.drain(..sinir);
        }
        true
    }

    pub fn teklif_gonderildi(&mut self, anahtar: &str, gun: &str) {
        if let Some(teklif) = self
            .dosya
            .teklifler
            .iter_mut()
            .find(|teklif| teklif.anahtar == anahtar && teklif.gun == gun)
        {
            teklif.gonderildi = true;
        }
    }

    /// Teklifler en son hangi gün kendiliğinden hazırlandı?
    pub fn son_otomatik(&self) -> &str {
        &self.dosya.son_otomatik
    }

    pub fn son_otomatik_ayarla(&mut self, gun: &str) {
        self.dosya.son_otomatik = gun.to_string();
    }

    pub fn teklif_sil(&mut self, anahtar: &str, gun: &str) {
        self.dosya
            .teklifler
            .retain(|teklif| !(teklif.anahtar == anahtar && teklif.gun == gun));
    }

    /// Bu müşteri için bugün teklif hazırlandı mı?
    pub fn bugun_teklif_var(&self, anahtar: &str, gun: &str) -> bool {
        self.teklif(anahtar, gun).is_some()
    }
}

/// WhatsApp'a gönderilecek teklif metni.
pub fn teklif_metni(ad: &str, kayit: &MusteriGelir) -> String {
    let mut metin = format!("Merhaba {},", ad.trim());
    match kayit.paket.as_deref().and_then(paket) {
        Some(paket) => metin.push_str(&format!(
            "\n\n{} paketimizi ayda {} TL olarak sunuyoruz. Kapsam:",
            paket.ad,
            fiyat_yaz(kayit.paket_fiyati)
        )),
        None => metin.push_str(&format!(
            "\n\nAylık hizmet teklifimiz: {} TL. Kapsam:",
            fiyat_yaz(kayit.aylik())
        )),
    }
    for kapsam in kayit.kapsam() {
        metin.push_str(&format!("\n• {kapsam}"));
    }
    if kayit.tek_sefer_toplam() > 0 {
        metin.push_str("\n\nTek seferlik işler:");
        for kod in &kayit.tek_seferler {
            if let Some(hizmet) = hizmet(kod) {
                metin.push_str(&format!(
                    "\n• {} · {} TL",
                    hizmet.ad,
                    fiyat_yaz(hizmet.tek_sefer)
                ));
            }
        }
        metin.push_str(&format!(
            "\nTek seferlik toplam: {} TL.",
            fiyat_yaz(kayit.tek_sefer_toplam())
        ));
    }
    metin.push_str("\n\nUygun olursa bu hafta başlayabiliriz. Sorularınızı yanıtlarım.");
    metin
}

/// Binlik ayraçlı fiyat metni: 12500 → "12.500".
fn fiyat_yaz(tutar: u32) -> String {
    let rakamlar = tutar.to_string();
    let mut cikti = String::new();
    for (sira, karakter) in rakamlar.chars().enumerate() {
        if sira > 0 && (rakamlar.len() - sira) % 3 == 0 {
            cikti.push('.');
        }
        cikti.push(karakter);
    }
    cikti
}

impl LeadApp {
    /// Bugünkü uygun kayıtlar için teklif üretir: (üretilen, atlanan).
    fn teklifleri_hazirla(&mut self) -> (usize, usize) {
        let gun = bugun_anahtari();
        let uygun_sayisi = self
            .defter
            .kayitlar()
            .iter()
            .filter(|kayit| !kayit.durum.kara_listede())
            .count();
        let adaylar: Vec<(String, String, OneriGirdi)> = self
            .defter
            .kayitlar()
            .iter()
            .filter(|kayit| !kayit.durum.kara_listede())
            .filter(|kayit| {
                self.gelir.aylik(&kayit.anahtar) == 0
                    && !self.gelir.bugun_teklif_var(&kayit.anahtar, &gun)
            })
            .map(|kayit| (kayit.anahtar.clone(), kayit.ad.clone(), oneri_girdi(kayit)))
            .collect();
        for (anahtar, ad, girdi) in &adaylar {
            self.gelir
                .teklif_ekle(teklif_olustur(anahtar, ad, *girdi, &gun));
        }
        self.gelir.kaydet();
        (adaylar.len(), uygun_sayisi.saturating_sub(adaylar.len()))
    }

    /// Teklifleri günde bir kez kendiliğinden hazırlar: kullanıcı hiçbir
    /// düğmeye basmadan bütün uygun müşterilerin teklif metni ve tutarları
    /// hazır olur. Gönderim bilerek otomatik değildir; WhatsApp ya da
    /// Kopyala düğmesiyle elle yapılır.
    pub(crate) fn teklifleri_otomatik_hazirla(&mut self) {
        let gun = bugun_anahtari();
        if self.gelir.son_otomatik() == gun {
            return;
        }
        let (hazir, _) = self.teklifleri_hazirla();
        self.gelir.son_otomatik_ayarla(&gun);
        self.gelir.kaydet();
        if hazir > 0 {
            self.logs.push(format!(
                "[✓] {hazir} teklif kendiliğinden hazırlandı; göndermek için WhatsApp ya da Kopyala."
            ));
        }
    }

    /// Gelir katmanının tamamı: pano, müşteri listesi ve hizmet seçimi.
    ///
    /// Tek bir `CentralPanel` içinde iki sütun çizilir; böylece katman
    /// değişiminde panel sırası sorunu çıkmaz.
    /// Tek düğme: teklifleri hazırlar, 10 saatlik planı kurar/devam ettirir,
    /// abonelerin aylık teslim dosyalarını üretir. Mesaj göndermez.
    pub(crate) fn her_seyi_calistir(&mut self) {
        let gun = bugun_anahtari();
        let ay: String = gun.chars().take(7).collect();
        let (teklif, _) = self.teklifleri_hazirla();
        if self.plan.gorevler.is_empty() {
            self.plan = plan::Plan::olustur(&self.defter, &self.gelir, plan::simdi());
        }
        self.plan.calisiyor = true;
        self.plan.kaydet();

        let borclar = otomasyon::teslim_borclari(&self.gelir, &self.defter, &self.otomasyon, &ay);
        let mut uretilen = 0;
        for (anahtar, _) in &borclar {
            let isletme = self.defter.kayit(anahtar).cloned();
            let kayit = self.gelir.kayit(anahtar).cloned();
            if let (Some(isletme), Some(kayit)) = (isletme, kayit) {
                if kayit.aylik() > 0 && teslimat::yaz(&isletme, &kayit, &gun).is_ok() {
                    self.otomasyon.teslim_et(anahtar, &ay);
                    uretilen += 1;
                }
            }
        }
        self.otomasyon.kaydet();
        let isler = otomasyon::gunun_isleri(&self.defter, &self.gelir, &gun);
        self.logs.push(format!(
            "[✓] Her şey çalıştı: {teklif} teklif · {} plan görevi · {uretilen} teslim · {} kişi mesaj sırasında.",
            self.plan.gorevler.len(),
            isler.len()
        ));
    }

    pub(crate) fn gelir_ekrani(&mut self, ctx: &egui::Context) {
        self.teklifleri_otomatik_hazirla();
        let (mut secilen, mut hedef_degis) = (None, None);
        let mut paket_sec: Option<String> = None;
        let mut paket_birak = false;
        let mut hizmet_degis: Option<(String, bool)> = None;
        let mut tek_sefer_degis: Option<(String, bool)> = None;
        let mut teklif_iste = false;
        let mut fatura_iste = false;
        let mut kopyala_iste = false;
        let mut yeni_teklif = false;
        let mut oneri_uygula = false;
        let mut toplu_teklif = false;
        let mut sayfa_degis: Option<u8> = None;
        let mut teklif_aksiyon: Option<TeklifAksiyon> = None;
        // Hizmet çıktıları: seçili müşterinin dosyaları üretilsin mi, yayına gitsin mi.
        let mut cikti_iste = false;
        let mut cikti_yayin_iste = false;
        // Tek düğme: her şeyi çalıştır / mesajları kopyala.
        let mut hepsini_calistir = false;
        let mut mesajlari_kopyala = false;

        let mrr = self.gelir.mrr();
        let abone = self.gelir.abone_sayisi();
        let hedef_sira = self.gelir_hedef.min(HEDEFLER.len() - 1);
        let (hedef, hedef_etiket) = HEDEFLER[hedef_sira];
        let oran = (mrr as f32 / hedef as f32).clamp(0.0, 1.0);
        let ortalama = if abone > 0 { mrr / abone as u32 } else { 5_000 };
        let eksik = hedef.saturating_sub(mrr);
        let eksik_abone = if ortalama > 0 {
            (eksik + ortalama - 1) / ortalama
        } else {
            0
        };

        let secili_anahtar = self.gelir_secili.clone();
        let musteri = secili_anahtar.as_ref().and_then(|anahtar| {
            self.defter
                .kayit(anahtar)
                .map(|kayit| (kayit.ad.clone(), kayit.telefon.clone()))
        });
        let gelir_kaydi = secili_anahtar
            .as_ref()
            .and_then(|anahtar| self.gelir.kayit(anahtar).cloned())
            .unwrap_or_default();
        let oneri_girdisi = secili_anahtar
            .as_ref()
            .and_then(|anahtar| self.defter.kayit(anahtar))
            .map(oneri_girdi);
        let teklifler: Vec<Teklif> = self.gelir.teklifler().to_vec();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                // --- Pano ---------------------------------------------------
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        bolum_etiketi(ui, "GELİR");
                        ui.label(
                            egui::RichText::new(format!("{} TL/ay", fiyat_yaz(mrr)))
                                .font(baslik_font(23.0))
                                .color(METIN),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{abone} abone · ortalama {} TL · hedef {} ({})",
                                fiyat_yaz(ortalama),
                                fiyat_yaz(hedef),
                                hedef_etiket
                            ))
                            .size(10.0)
                            .color(DIM),
                        );
                    });
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        let genislik = (ui.available_width() - 300.0).max(180.0);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(genislik, 8.0), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(rect, egui::Rounding::same(4.0), GIRINTI);
                        if oran > 0.0 {
                            let dolu = egui::Rect::from_min_size(
                                rect.min,
                                egui::vec2((rect.width() * oran).max(6.0), rect.height()),
                            );
                            painter.rect_filled(dolu, egui::Rounding::same(4.0), VURGU);
                        }
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "%{} · hedefe {} TL · yaklaşık {} abone daha",
                                (oran * 100.0).round() as i32,
                                fiyat_yaz(eksik),
                                eksik_abone
                            ))
                            .size(10.5)
                            .color(SOLUK),
                        );
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("HEDEF").size(9.0).color(DIM));
                            for (sira, (_, etiket)) in HEDEFLER.iter().enumerate() {
                                if filtre_cipi(ui, etiket, sira == hedef_sira).clicked()
                                    && sira != hedef_sira
                                {
                                    hedef_degis = Some(sira);
                                }
                            }
                        });
                    });
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if filtre_cipi(ui, "Yönetim", self.gelir_sayfa == 0).clicked()
                        && self.gelir_sayfa != 0
                    {
                        sayfa_degis = Some(0);
                    }
                    let teklif_etiketi =
                        format!("Teklifler · {}", self.gelir.teklif_sayisi());
                    if filtre_cipi(ui, &teklif_etiketi, self.gelir_sayfa == 1).clicked()
                        && self.gelir_sayfa != 1
                    {
                        sayfa_degis = Some(1);
                    }
                    if filtre_cipi(ui, "Otomasyon", self.gelir_sayfa == 2).clicked()
                        && self.gelir_sayfa != 2
                    {
                        sayfa_degis = Some(2);
                    }
                    if filtre_cipi(ui, "Tablo", self.gelir_sayfa == 3).clicked()
                        && self.gelir_sayfa != 3
                    {
                        sayfa_degis = Some(3);
                    }
                    if filtre_cipi(ui, "Plan", self.gelir_sayfa == 4).clicked()
                        && self.gelir_sayfa != 4
                    {
                        sayfa_degis = Some(4);
                    }
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ikincil_buton(ui, "Tümü için teklif hazırla", true, None)
                                .clicked()
                            {
                                toplu_teklif = true;
                            }
                        },
                    );
                });
                ui.add_space(6.0);

                ui.horizontal_top(|ui| {
                    // --- Sol: müşteri listesi -------------------------------
                    ui.allocate_ui_with_layout(
                        egui::vec2(290.0, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            bolum_etiketi(ui, "MÜŞTERİLER");
                            ui.add_space(4.0);
                            arama_kutusu(ui, &mut self.gelir_arama, "Müşteri ara");
                            ui.add_space(6.0);
                            let arama = arama_anahtari(&self.gelir_arama);
                            egui::ScrollArea::vertical()
                                .id_source("gelir_musteri_listesi")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let mut bulundu = 0;
                                    for kayit in self.defter.kayitlar() {
                                        if !arama.is_empty()
                                            && !arama_anahtari(&kayit.ad).contains(&arama)
                                            && !arama_anahtari(&kayit.telefon).contains(&arama)
                                        {
                                            continue;
                                        }
                                        bulundu += 1;
                                        let aylik = self.gelir.aylik(&kayit.anahtar);
                                        let secili = self.gelir_secili.as_deref()
                                            == Some(kayit.anahtar.as_str());
                                        let metin = if aylik > 0 {
                                            format!("{}  ·  {} TL", kayit.ad, fiyat_yaz(aylik))
                                        } else {
                                            kayit.ad.clone()
                                        };
                                        if liste_satiri(ui, secili, &metin).clicked() {
                                            secilen = Some(kayit.anahtar.clone());
                                        }
                                    }
                                    if bulundu == 0 {
                                        ui.label(
                                            egui::RichText::new(if self.defter.kayitlar().is_empty()
                                            {
                                                "Defter boş: önce Müşteri Bulma sekmesinde tarama yapın."
                                            } else {
                                                "Eşleşen müşteri yok."
                                            })
                                            .size(10.5)
                                            .color(DIM),
                                        );
                                    }
                                });
                        },
                    );

                    ui.separator();

                    // --- Sağ: paket ve hizmet seçimi ------------------------
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            // --- Tek düğme çubuğu: her sayfada en üstte ---------
                            egui::Frame::none()
                                .fill(GIRINTI)
                                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if birincil_buton(ui, "HER ŞEYİ ÇALIŞTIR", true, None)
                                            .clicked()
                                        {
                                            hepsini_calistir = true;
                                        }
                                        ui.add_space(6.0);
                                        if ikincil_buton(ui, "MESAJLARI KOPYALA", true, None)
                                            .clicked()
                                        {
                                            mesajlari_kopyala = true;
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "plan {}/{} · abone {} · tek düğme yeter",
                                                        self.plan.tamamlanan(),
                                                        self.plan.gorevler.len(),
                                                        self.gelir.abone_sayisi()
                                                    ))
                                                    .size(9.5)
                                                    .color(DIM),
                                                );
                                            },
                                        );
                                    });
                                });
                            ui.add_space(6.0);

                            if self.gelir_sayfa == 1 {
                                teklif_aksiyon = teklif_listesi(ui, &teklifler);
                                return;
                            }
                            if self.gelir_sayfa == 2 {
                                self.otomasyon_ekrani(ui);
                                return;
                            }
                            if self.gelir_sayfa == 3 {
                                self.tablo_ekrani(ui);
                                return;
                            }
                            if self.gelir_sayfa == 4 {
                                self.plan_ekrani(ui);
                                return;
                            }
                            if let Some(girdi) = oneri_girdisi {
                                let oneri = oneri(girdi);
                                egui::Frame::none()
                                    .fill(GIRINTI)
                                    .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                                    .rounding(egui::Rounding::same(6.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "Öneri: {}",
                                                        oneri.satir()
                                                    ))
                                                    .size(11.0)
                                                    .color(METIN),
                                                );
                                                ui.label(
                                                    egui::RichText::new(&oneri.gerekce)
                                                        .size(9.5)
                                                        .color(DIM),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    if ikincil_buton(
                                                        ui,
                                                        "Teklif oluştur",
                                                        true,
                                                        None,
                                                    )
                                                    .clicked()
                                                    {
                                                        yeni_teklif = true;
                                                    }
                                                    if hayalet_buton(ui, "Öneriyi uygula")
                                                        .clicked()
                                                    {
                                                        oneri_uygula = true;
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(6.0);
                            }
                            let Some((ad, telefon)) = musteri.clone() else {
                                ui.add_space(40.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new("Soldan bir müşteri seçin.")
                                            .size(12.0)
                                            .color(DIM),
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Paket seçin ya da tek tek hizmet işaretleyin; aylık tutar ve teklif metni kendiliğinden oluşur.",
                                        )
                                        .size(10.0)
                                        .color(DIM),
                                    );
                                });
                                return;
                            };

                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&ad)
                                            .font(baslik_font(16.0))
                                            .color(METIN),
                                    );
                                    ui.label(egui::RichText::new(&telefon).size(10.0).color(DIM));
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "aylık {} TL · tek seferlik {} TL",
                                                fiyat_yaz(gelir_kaydi.aylik()),
                                                fiyat_yaz(gelir_kaydi.tek_sefer_toplam())
                                            ))
                                            .size(11.0)
                                            .color(VURGU),
                                        );
                                    },
                                );
                            });
                            ui.add_space(6.0);

                            bolum_etiketi(ui, "PAKETLER");
                            ui.horizontal_wrapped(|ui| {
                                for paket in PAKETLER.iter() {
                                    let secili = gelir_kaydi.paket.as_deref() == Some(paket.kod);
                                    let metin = format!(
                                        "{} · {} TL",
                                        paket.ad,
                                        fiyat_yaz(paket.fiyat)
                                    );
                                    if filtre_cipi(ui, &metin, secili).clicked() && !secili {
                                        paket_sec = Some(paket.kod.to_string());
                                    }
                                }
                                if gelir_kaydi.paket.is_some()
                                    && hayalet_buton(ui, "Paketi bırak").clicked()
                                {
                                    paket_birak = true;
                                }
                            });
                            if let Some(paket) = gelir_kaydi.paket.as_deref().and_then(paket) {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}: {}",
                                        paket.ad,
                                        paket
                                            .hizmetler
                                            .iter()
                                            .filter_map(|kod| hizmet(kod))
                                            .map(|hizmet| hizmet.ad)
                                            .collect::<Vec<_>>()
                                            .join(" · ")
                                    ))
                                    .size(9.5)
                                    .color(SOLUK),
                                );
                            }
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .id_source("gelir_hizmetler")
                                .auto_shrink([false, false])
                                .max_height((ui.available_height() - 46.0).max(120.0))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    bolum_etiketi(ui, "AYLIK HİZMETLER");
                                    let mut grup = "";
                                    for hizmet in KATALOG.iter().filter(|hizmet| hizmet.aylik > 0) {
                                        if hizmet.grup != grup {
                                            grup = hizmet.grup;
                                            ui.add_space(6.0);
                                            ui.label(
                                                egui::RichText::new(grup).size(9.5).color(VURGU),
                                            );
                                        }
                                        let mut aktif = gelir_kaydi
                                            .hizmetler
                                            .iter()
                                            .any(|kod| kod == hizmet.kod);
                                        let etiket = format!(
                                            "{} · {} TL/ay{}",
                                            hizmet.ad,
                                            fiyat_yaz(hizmet.aylik),
                                            if hizmet.hazir { "  ✓" } else { "" }
                                        );
                                        if ui
                                            .checkbox(
                                                &mut aktif,
                                                egui::RichText::new(etiket).size(11.0),
                                            )
                                            .on_hover_text(hizmet.aciklama)
                                            .changed()
                                        {
                                            hizmet_degis =
                                                Some((hizmet.kod.to_string(), aktif));
                                        }
                                    }

                                    ui.add_space(12.0);
                                    bolum_etiketi(ui, "TEK SEFERLİK İŞLER");
                                    for hizmet in KATALOG.iter().filter(|h| h.tek_sefer > 0) {
                                        let mut aktif = gelir_kaydi
                                            .tek_seferler
                                            .iter()
                                            .any(|kod| kod == hizmet.kod);
                                        let etiket = format!(
                                            "{} · {} TL",
                                            hizmet.ad,
                                            fiyat_yaz(hizmet.tek_sefer)
                                        );
                                        if ui
                                            .checkbox(
                                                &mut aktif,
                                                egui::RichText::new(etiket).size(11.0),
                                            )
                                            .on_hover_text(hizmet.aciklama)
                                            .changed()
                                        {
                                            tek_sefer_degis =
                                                Some((hizmet.kod.to_string(), aktif));
                                        }
                                    }
                                });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if birincil_buton(ui, "Teklifi hazırla", true, None).clicked() {
                                    teklif_iste = true;
                                }
                                ui.add_space(6.0);
                                if ikincil_buton(ui, "Fatura kesildi", true, None).clicked() {
                                    fatura_iste = true;
                                }
                                ui.add_space(6.0);
                                if hayalet_buton(ui, "Metni kopyala").clicked() {
                                    kopyala_iste = true;
                                }
                                ui.add_space(6.0);
                                if hayalet_buton(ui, "Hizmet dosyaları").clicked() {
                                    cikti_iste = true;
                                }
                                ui.add_space(6.0);
                                if hayalet_buton(ui, "Üret + yayınla").clicked() {
                                    cikti_yayin_iste = true;
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(tarih) = &gelir_kaydi.son_fatura {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "son fatura · {tarih}"
                                                ))
                                                .size(9.5)
                                                .color(DIM),
                                            );
                                        }
                                    },
                                );
                            });
                        },
                    );
                });
            });

        // --- Değişiklikleri uygula ------------------------------------------
        if let Some(sira) = hedef_degis {
            self.gelir_hedef = sira;
        }
        if let Some(sayfa) = sayfa_degis {
            self.gelir_sayfa = sayfa;
        }
        if toplu_teklif {
            let (hazir, atlanan) = self.teklifleri_hazirla();
            self.logs.push(format!(
                "[✓] {hazir} teklif hazırlandı · {atlanan} kayıt (abone ya da bugün teklif var) atlandı."
            ));
            if hazir > 0 {
                self.gelir_sayfa = 1;
            }
        }
        if let Some(anahtar) = secilen {
            self.gelir_secili = Some(anahtar);
        }
        let Some(anahtar) = self.gelir_secili.clone() else {
            return;
        };
        let Some((ad, telefon)) = musteri else {
            return;
        };
        if oneri_uygula {
            if let Some(girdi) = oneri_girdisi {
                let oneri = oneri(girdi);
                self.gelir.paket_uygula(&anahtar, &ad, oneri.paket);
                for kod in &oneri.tek_seferler {
                    self.gelir.tek_sefer_ayarla(&anahtar, &ad, kod, true);
                }
                self.gelir.kaydet();
                self.logs
                    .push(format!("[✓] {ad}: öneri uygulandı ({}).", oneri.satir()));
            }
        }
        if yeni_teklif {
            if let Some(girdi) = oneri_girdisi {
                let teklif = teklif_olustur(&anahtar, &ad, girdi, &bugun_anahtari());
                let ozet = teklif.ozet.clone();
                self.gelir.teklif_ekle(teklif);
                self.gelir.kaydet();
                self.logs.push(format!("[✓] {ad}: teklif hazır ({ozet})."));
                self.gelir_sayfa = 1;
            }
        }
        if let Some(paket_kodu) = paket_sec {
            self.gelir.paket_uygula(&anahtar, &ad, &paket_kodu);
            self.gelir.kaydet();
            let etiket = paket(&paket_kodu).map(|paket| paket.ad).unwrap_or("paket");
            self.logs.push(format!("[✓] {ad}: {etiket} paketi atandı."));
        }
        if paket_birak {
            self.gelir.paket_kaldir(&anahtar, &ad);
            self.gelir.kaydet();
        }
        if let Some((kod, ekle)) = hizmet_degis {
            self.gelir.hizmet_ayarla(&anahtar, &ad, &kod, ekle);
            self.gelir.kaydet();
        }
        if let Some((kod, ekle)) = tek_sefer_degis {
            self.gelir.tek_sefer_ayarla(&anahtar, &ad, &kod, ekle);
            self.gelir.kaydet();
        }
        if fatura_iste {
            self.gelir.fatura_kes(&anahtar, &ad);
            self.gelir.kaydet();
            self.logs
                .push(format!("[✓] {ad}: fatura kesildi olarak işaretlendi."));
        }
        if teklif_iste || kopyala_iste {
            let kayit = self.gelir.kayit(&anahtar).cloned().unwrap_or_default();
            let metin = teklif_metni(&ad, &kayit);
            if teklif_iste {
                match whatsapp_numarasi(&telefon) {
                    Some(numara) => {
                        if sohbet_ac(&numara, &metin, &mut self.logs) {
                            self.logs
                                .push(format!("[✓] {ad}: teklif WhatsApp'ta açıldı."));
                        }
                    }
                    None => {
                        ctx.output_mut(|cikti| cikti.copied_text = metin);
                        self.logs
                            .push(format!("[i] {ad}: telefon yok, teklif panoya kopyalandı."));
                    }
                }
            } else {
                ctx.output_mut(|cikti| cikti.copied_text = metin);
                self.logs
                    .push("[✓] Teklif metni panoya kopyalandı.".to_string());
            }
        }
        // Hizmet çıktıları: satılan hizmetlerin teslim dosyalarını üret (ve istenirse yayınla).
        if cikti_iste || cikti_yayin_iste {
            match secili_anahtar.clone().and_then(|anahtar| {
                Some((
                    anahtar.clone(),
                    self.defter.kayit(&anahtar).cloned(),
                    self.gelir.kayit(&anahtar).cloned(),
                ))
            }) {
                Some((_, Some(isletme), Some(kayit))) => {
                    match teslimat::yaz(&isletme, &kayit, &bugun_anahtari()) {
                        Ok(ciktilar) => {
                            self.logs.push(format!(
                                "[✓] {}: {} hizmet dosyası üretildi → {}/",
                                isletme.ad,
                                ciktilar.len(),
                                teslimat::KLASOR
                            ));
                            for cikti in &ciktilar {
                                self.logs
                                    .push(format!("    · {} · {}", cikti.dosya, cikti.baslik));
                            }
                            if cikti_yayin_iste {
                                let yollar: Vec<String> = ciktilar
                                    .iter()
                                    .map(|cikti| format!("{}/{}", teslimat::KLASOR, cikti.dosya))
                                    .collect();
                                match yayina_gonder(".", &yollar, &mut self.logs) {
                                    Ok(link) => self.logs.push(format!("[✓] Yayında: {link}")),
                                    Err(hata) => {
                                        self.logs.push(format!("[!] Yayına gönderilemedi: {hata}"))
                                    }
                                }
                            }
                        }
                        Err(hata) => self
                            .logs
                            .push(format!("[!] Hizmet dosyaları üretilemedi: {hata}")),
                    }
                }
                Some((_, None, _)) => self
                    .logs
                    .push("[!] Müşteri defterde bulunamadı.".to_string()),
                _ => self
                    .logs
                    .push("[!] Önce bir müşteri seçin ve paket/hizmet işaretleyin.".to_string()),
            }
        }
        if hepsini_calistir {
            self.her_seyi_calistir();
        }
        if mesajlari_kopyala {
            let isler = otomasyon::gunun_isleri(&self.defter, &self.gelir, &bugun_anahtari());
            let metin = isler
                .iter()
                .map(|is| format!("{} — {}\n{}\n", is.ad, is.telefon, is.metin))
                .collect::<Vec<_>>()
                .join("\n");
            ctx.output_mut(|cikti| cikti.copied_text = metin);
            self.logs.push(format!(
                "[✓] {} mesaj panoya kopyalandı; sırayla gönderin.",
                isler.len()
            ));
        }
        if let Some(aksiyon) = teklif_aksiyon {
            match aksiyon {
                TeklifAksiyon::Sil(teklif_anahtari, gun) => {
                    self.gelir.teklif_sil(&teklif_anahtari, &gun);
                    self.gelir.kaydet();
                }
                TeklifAksiyon::Gonderildi(teklif_anahtari, gun) => {
                    self.gelir.teklif_gonderildi(&teklif_anahtari, &gun);
                    self.gelir.kaydet();
                    self.logs
                        .push("[✓] Teklif gönderildi olarak işaretlendi.".to_string());
                }
                TeklifAksiyon::Kopyala(teklif_anahtari, gun) => {
                    if let Some(teklif) = self.gelir.teklif(&teklif_anahtari, &gun).cloned() {
                        ctx.output_mut(|cikti| cikti.copied_text = teklif.metin.clone());
                        self.logs
                            .push("[✓] Teklif metni panoya kopyalandı.".to_string());
                    }
                }
                TeklifAksiyon::Gonder(teklif_anahtari, gun) => {
                    let Some(teklif) = self.gelir.teklif(&teklif_anahtari, &gun).cloned() else {
                        return;
                    };
                    let telefon = self
                        .defter
                        .kayit(&teklif_anahtari)
                        .map(|kayit| kayit.telefon.clone())
                        .unwrap_or_default();
                    match whatsapp_numarasi(&telefon) {
                        Some(numara) => {
                            if sohbet_ac(&numara, &teklif.metin, &mut self.logs) {
                                self.gelir.teklif_gonderildi(&teklif_anahtari, &gun);
                                self.gelir.kaydet();
                                self.logs
                                    .push(format!("[✓] {}: teklif WhatsApp'ta açıldı.", teklif.ad));
                            }
                        }
                        None => {
                            ctx.output_mut(|cikti| cikti.copied_text = teklif.metin.clone());
                            self.logs.push(format!(
                                "[i] {}: telefon WhatsApp'a uygun değil, teklif panoya kopyalandı.",
                                teklif.ad
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Hazır teklif listesinde seçilen işlem.
enum TeklifAksiyon {
    Gonder(String, String),
    Kopyala(String, String),
    Gonderildi(String, String),
    Sil(String, String),
}

/// Hazır teklifler sayfası: satır başına elle gönderim işlemleri.
fn teklif_listesi(ui: &mut egui::Ui, teklifler: &[Teklif]) -> Option<TeklifAksiyon> {
    let mut aksiyon = None;
    let potansiyel: u32 = teklifler
        .iter()
        .filter(|teklif| !teklif.gonderildi)
        .map(|teklif| teklif.aylik)
        .sum();
    ui.horizontal(|ui| {
        bolum_etiketi(ui, "HAZIR TEKLİFLER");
        ui.label(
            egui::RichText::new(format!(
                "{} teklif · aylık potansiyel {} TL",
                teklifler.len(),
                fiyat_yaz(potansiyel)
            ))
            .size(10.0)
            .color(DIM),
        );
    });
    ui.label(
        egui::RichText::new(
            "Teklifler günde bir kez kendiliğinden hazırlanır; mesajı sen gönderirsin (WhatsApp ya da Kopyala).",
        )
        .size(10.0)
        .color(DIM),
    );
    ui.add_space(6.0);
    if teklifler.is_empty() {
        ui.label(
            egui::RichText::new(
                "Henüz teklif yok. Üstteki \"Tümü için teklif hazırla\" düğmesine basın.",
            )
            .size(11.0)
            .color(SOLUK),
        );
        return None;
    }
    egui::ScrollArea::vertical()
        .id_source("gelir_teklif_listesi")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for teklif in teklifler {
                egui::Frame::none()
                    .fill(GIRINTI)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .rounding(egui::Rounding::same(6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&teklif.ad)
                                            .font(baslik_font(12.0))
                                            .color(METIN),
                                    );
                                    if teklif.gonderildi {
                                        ui.label(
                                            egui::RichText::new("gönderildi")
                                                .size(9.0)
                                                .color(VURGU),
                                        );
                                    }
                                    ui.label(
                                        egui::RichText::new(format!("· {}", teklif.gun))
                                            .size(9.0)
                                            .color(DIM),
                                    );
                                });
                                ui.label(egui::RichText::new(&teklif.ozet).size(10.0).color(SOLUK));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if hayalet_buton(ui, "Sil").clicked() {
                                        aksiyon = Some(TeklifAksiyon::Sil(
                                            teklif.anahtar.clone(),
                                            teklif.gun.clone(),
                                        ));
                                    }
                                    if hayalet_buton(ui, "Gönderildi").clicked() {
                                        aksiyon = Some(TeklifAksiyon::Gonderildi(
                                            teklif.anahtar.clone(),
                                            teklif.gun.clone(),
                                        ));
                                    }
                                    if ikincil_buton(ui, "Kopyala", true, None).clicked() {
                                        aksiyon = Some(TeklifAksiyon::Kopyala(
                                            teklif.anahtar.clone(),
                                            teklif.gun.clone(),
                                        ));
                                    }
                                    if birincil_buton(ui, "WhatsApp", true, None).clicked() {
                                        aksiyon = Some(TeklifAksiyon::Gonder(
                                            teklif.anahtar.clone(),
                                            teklif.gun.clone(),
                                        ));
                                    }
                                },
                            );
                        });
                    });
                ui.add_space(6.0);
            }
        });
    aksiyon
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn katalog_kodlari_tekil() {
        let mut kodlar: Vec<&str> = KATALOG.iter().map(|hizmet| hizmet.kod).collect();
        kodlar.sort_unstable();
        kodlar.dedup();
        assert_eq!(kodlar.len(), KATALOG.len());
    }

    #[test]
    fn paketler_katalogdaki_hizmetleri_kullanir() {
        for paket in PAKETLER.iter() {
            assert!(!paket.hizmetler.is_empty(), "{} boş", paket.ad);
            for kod in paket.hizmetler {
                assert!(hizmet(kod).is_some(), "{} katalogda yok", kod);
            }
        }
    }

    #[test]
    fn paket_aylik_fiyati_kullanir() {
        let mut gelir = Gelir::default();
        gelir.paket_uygula("a1", "Deneme", "buyume");
        assert_eq!(gelir.aylik("a1"), 5_000);
        assert_eq!(gelir.mrr(), 5_000);
        assert_eq!(gelir.abone_sayisi(), 1);
    }

    #[test]
    fn tek_hizmet_paketi_bozar_ve_toplami_degistirir() {
        let mut gelir = Gelir::default();
        gelir.paket_uygula("a1", "Deneme", "buyume");
        gelir.hizmet_ayarla("a1", "Deneme", "geo", true);
        let kayit = gelir.kayit("a1").expect("kayıt var");
        assert!(kayit.paket.is_none());
        assert_eq!(kayit.aylik(), 7_500);
    }

    #[test]
    fn hizmet_ekle_cikar_idempotent() {
        let mut gelir = Gelir::default();
        gelir.hizmet_ayarla("a1", "Deneme", "gbp", true);
        gelir.hizmet_ayarla("a1", "Deneme", "gbp", true);
        assert_eq!(gelir.aylik("a1"), 1_800);
        gelir.hizmet_ayarla("a1", "Deneme", "gbp", false);
        assert_eq!(gelir.aylik("a1"), 0);
    }

    #[test]
    fn tek_sefer_toplami_hesaplanir() {
        let mut gelir = Gelir::default();
        gelir.tek_sefer_ayarla("a1", "Deneme", "site_kurulum", true);
        gelir.tek_sefer_ayarla("a1", "Deneme", "seo_audit", true);
        let kayit = gelir.kayit("a1").expect("kayıt var");
        assert_eq!(kayit.tek_sefer_toplam(), 11_000);
        assert_eq!(kayit.aylik(), 0);
    }

    #[test]
    fn teklif_metni_paket_ve_fiyati_yazar() {
        let mut gelir = Gelir::default();
        gelir.paket_uygula("a1", "Deneme", "pro");
        let kayit = gelir.kayit("a1").expect("kayıt var");
        let metin = teklif_metni("Deneme Kafe", kayit);
        assert!(metin.contains("Pro paketimizi"));
        assert!(metin.contains("9.000 TL"));
        assert!(metin.contains("Google profil yönetimi"));
    }

    #[test]
    fn kayit_diskten_geri_okunur() {
        let mut gelir = Gelir::default();
        gelir.paket_uygula("a1", "Deneme", "baslangic");
        let metin = serde_json::to_string(&gelir.dosya).expect("json");
        let geri = Gelir::metinden(&metin);
        assert_eq!(geri.mrr(), 2_500);
        assert_eq!(geri.kayit("a1").map(|k| k.ad.as_str()), Some("Deneme"));
    }

    #[test]
    fn bozuk_dosya_bos_gelir_verir() {
        assert_eq!(Gelir::metinden("{bozuk").mrr(), 0);
    }

    #[test]
    fn fatura_tarihi_yazilir() {
        let mut gelir = Gelir::default();
        gelir.fatura_kes("a1", "Deneme");
        let kayit = gelir.kayit("a1").expect("kayıt var");
        assert!(kayit.son_fatura.is_some());
    }

    fn girdi(site_var: bool, puan: u8, profil_eksik: bool) -> OneriGirdi {
        OneriGirdi {
            site_var,
            puan,
            profil_eksik,
        }
    }

    #[test]
    fn oneri_sitesiz_musteriye_site_paketi_verir() {
        let oneri = oneri(girdi(false, 70, false));
        assert_eq!(oneri.paket, "buyume");
        assert!(oneri.tek_seferler.contains(&"site_kurulum"));
        assert_eq!(oneri.aylik(), 5_000);
        assert_eq!(oneri.tek_sefer_toplam(), 8_500);
        assert!(oneri.satir().contains("Büyüme"));
    }

    #[test]
    fn oneri_eksik_profilde_baslangic_ve_kurulum_verir() {
        let oneri = oneri(girdi(true, 30, true));
        assert_eq!(oneri.paket, "baslangic");
        assert!(oneri.tek_seferler.contains(&"gbp_kurulum"));
        assert!(!oneri.tek_seferler.contains(&"site_kurulum"));
        assert!(oneri.tek_seferler.contains(&"seo_audit"));
    }

    #[test]
    fn oneri_guclu_profilde_gorunurluk_paketi_verir() {
        let oneri = oneri(girdi(true, 85, false));
        assert_eq!(oneri.paket, "pro");
        assert!(oneri.tek_seferler.is_empty());
        assert_eq!(oneri.aylik(), 9_000);
    }

    #[test]
    fn teklif_olustur_tutar_ve_gerekce_yazar() {
        let teklif = teklif_olustur("a1", "Deneme Kafe", girdi(false, 70, false), "2026-11-09");
        assert_eq!(teklif.aylik, 5_000);
        assert_eq!(teklif.tek_sefer, 8_500);
        assert!(teklif.metin.contains("Büyüme paketimizi"));
        assert!(teklif.metin.contains("Site kurulumu"));
        assert!(teklif.metin.contains("100"));
        assert_eq!(teklif.gun, "2026-11-09");
        assert!(!teklif.gonderildi);
    }

    #[test]
    fn ayni_gun_teklifi_guncellenir() {
        let mut gelir = Gelir::default();
        let ilk = teklif_olustur("a1", "Deneme", girdi(true, 30, false), "2026-11-09");
        assert!(gelir.teklif_ekle(ilk));
        let ikinci = teklif_olustur("a1", "Deneme", girdi(true, 85, false), "2026-11-09");
        assert!(!gelir.teklif_ekle(ikinci));
        assert_eq!(gelir.teklif_sayisi(), 1);
        assert_eq!(gelir.teklifler()[0].paket.as_deref(), Some("pro"));
    }

    #[test]
    fn teklifler_dissten_geri_okunur_ve_isaretlenir() {
        let mut gelir = Gelir::default();
        gelir.teklif_ekle(teklif_olustur(
            "a1",
            "Deneme",
            girdi(false, 55, false),
            "2026-11-09",
        ));
        let metin = serde_json::to_string(&gelir.dosya).expect("json");
        let mut geri = Gelir::metinden(&metin);
        assert_eq!(geri.teklif_sayisi(), 1);
        assert!(geri.bugun_teklif_var("a1", "2026-11-09"));
        geri.teklif_gonderildi("a1", "2026-11-09");
        assert!(geri.teklifler()[0].gonderildi);
        geri.teklif_sil("a1", "2026-11-09");
        assert_eq!(geri.teklif_sayisi(), 0);
    }

    #[test]
    fn otomatik_hazirlama_gunu_diskte_kalir() {
        let mut gelir = Gelir::default();
        assert_eq!(gelir.son_otomatik(), "");
        gelir.son_otomatik_ayarla("2026-11-10");
        let metin = serde_json::to_string(&gelir.dosya).expect("json");
        let geri = Gelir::metinden(&metin);
        assert_eq!(geri.son_otomatik(), "2026-11-10");
    }
}
