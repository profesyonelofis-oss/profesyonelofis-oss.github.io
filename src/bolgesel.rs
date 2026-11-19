//! Bölgesel pinleme ve yerel SEO çalışması.
//!
//! Bir işletme için seçilen bölgelerin (il/ilçe ya da O-5 koridoru) her biri
//! adına üç çıktı üretir:
//! 1. bölge pin/arama bağlantısı (`maps/search/?api=1&query=...`),
//! 2. bölgeye özel yerel açılış sayfası (`bolgesel/<işletme>/<il>-<ilçe>.html`),
//! 3. işletme sahibine/müşteriye gönderilecek WhatsApp metni.
//!
//! Ayrıca işletmenin kendi ana sayfası (`index.html`), bölge planı (`plan.md`)
//! ve Google işletme profili için arama artırma planı (`profil.md`) yazılır.
//!
//! Sınır: uygulama **yeni işletme kaydı/pin oluşturmaz**. Haritaya aynı
//! işletmeyi farklı adreslerde eklemek Google'ın kurallarına aykırıdır ve
//! profilin askıya alınmasına yol açar. Buradaki çalışma, tek gerçek konumu
//! olan işletmenin hizmet verdiği bölgelerde bulunmasını sağlayan içerik,
//! bağlantı ve mesajları üretir.

use crate::*;
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};

/// Çalışma dosyalarının yazıldığı klasör.
pub const KLASOR: &str = "bolgesel";

/// O-5 (Gebze–İzmir) otoyolu ve iki uçtaki şehirlerin hizmet noktaları.
// Kapsam: O-5 üzerinde İzmir Kemalpaşa ↔ Balıkesir Savaştepe arası.
// Otoyolun bu kesiminde sırayla geçilen ilçeler; başka bölge eklenmez.
pub const KORIDOR: &[(&str, &str)] = &[
    ("İzmir", "Kemalpaşa"),
    ("Manisa", "Turgutlu"),
    ("Manisa", "Saruhanlı"),
    ("Manisa", "Akhisar"),
    ("Manisa", "Kırkağaç"),
    ("Manisa", "Soma"),
    ("Balıkesir", "Savaştepe"),
];

/// Bölgesel çalışmada kullanılan işletme bilgileri.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Isletme {
    pub ad: String,
    pub telefon: String,
    pub adres: String,
    pub sektor: String,
    /// Posta kodu; boşsa şemaya yazılmaz.
    pub posta: String,
}

impl Isletme {
    /// Şemaya yazılacak `PostalAddress` nesnesi.
    ///
    /// Adres `"İlçe / İl"` biçiminde tutulduğu için ayrıştırma buradan yapılır;
    /// posta kodu doluysa eklenir, boşsa alan hiç yazılmaz.
    pub fn adres_verisi(&self) -> serde_json::Value {
        let (ilce, il) = self
            .adres
            .split_once('/')
            .map(|(a, b)| (a.trim(), b.trim()))
            .unwrap_or((self.adres.trim(), ""));
        let mut veri = serde_json::json!({
            "@type": "PostalAddress",
            "addressLocality": ilce,
            "addressRegion": il,
            "addressCountry": "TR"
        });
        let posta = self.posta.trim();
        if !posta.is_empty() {
            veri["postalCode"] = serde_json::json!(posta);
        }
        veri
    }
}

/// Üretilen sayfaların yayınlanacağı alan adı (sitemap/robots için).
/// Yayına alırken kendi alan adınızla değiştirin.
pub const ALAN_ADI: &str = "https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page";

/// İşletmenin doğrulanmış pin noktası (`pinler.json` içindeki `google:` kaydı).
///
/// `araclar/konum_guncelle.py` bu sabiti de günceller; elle düzenlemeyin.
/// Bölge sayfalarındaki mesafe, varış süresi ve `geo` alanı buradan beslenir.
pub const ISLETME_KONUM: (f64, f64) = (38.7335, 27.5775);

/// O-5 kavşak numarası ve otoyol kilometresi (koridor ilçeleri).
const KAVSAKLAR: &[(&str, &str, f64)] = &[
    ("Kemalpaşa", "K22", 389.6),
    ("Turgutlu", "O-5", 360.0),
    ("Saruhanlı", "K19", 339.6),
    ("Akhisar", "K18", 315.1),
    ("Kırkağaç", "K17", 299.5),
    ("Soma", "K16", 277.4),
    ("Savaştepe", "K15", 262.4),
];

/// İlçenin kavşak kodu ve O-5 kilometresi.
fn kavsak_bilgi(ilce: &str) -> (&'static str, f64) {
    KAVSAKLAR
        .iter()
        .find(|(ad, _, _)| *ad == ilce)
        .map(|(_, kod, km)| (*kod, *km))
        .unwrap_or(("O-5", 389.6))
}

/// İzmir (Kemalpaşa) girişinden ilçe kavşağına O-5 kilometresi.
fn koridor_girisinden(ilce: &str) -> f64 {
    (389.6 - kavsak_bilgi(ilce).1).max(0.0)
}

/// İki nokta arası büyük daire mesafesi (km).
/// Google işletme kaydının yer kimliği (Haritalar bağlantısından).
pub const YER_KIMLIGI: &str = "ChIJK4F59corgiMRMXtablaVzVk";
/// Müşteriyi doğrudan yorum yazma ekranına götürür.
pub const YORUM_LINK: &str =
    "https://search.google.com/local/writereview?placeid=ChIJK4F59corgiMRMXtablaVzVk";
/// Profildeki gerçek puan ve yorum sayısı; şemada aynen yayınlanır.
pub const PUAN: f64 = 5.0;
pub const YORUM_SAYISI: u32 = 16;

pub fn mesafe_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = 6371.0_f64;
    let (en1, bo1) = (a.0.to_radians(), a.1.to_radians());
    let (en2, bo2) = (b.0.to_radians(), b.1.to_radians());
    let h = ((en2 - en1) / 2.0).sin().powi(2)
        + en1.cos() * en2.cos() * ((bo2 - bo1) / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

/// Bölgenin harita noktası; koridor tablosunda yoksa `None`.
pub fn bolge_noktasi(il: &str, ilce: &str) -> Option<(f64, f64)> {
    crate::otoyol_pin::NOKTALAR
        .iter()
        .find(|nokta| nokta.il == il && nokta.ilce == ilce)
        .map(|nokta| (nokta.enlem, nokta.boylam))
}

/// Mesafeye göre gerçekçi varış süresi (dk).
///
/// 25 km'ye kadar şehir içi 42 km/sa, sonrası otoyol 78 km/sa; 6 dakika hazırlık.
pub fn tahmini_dakika(km: f64) -> u32 {
    let saat = if km <= 25.0 { km / 42.0 } else { km / 78.0 };
    (saat * 60.0 + 6.0).round().max(8.0) as u32
}

/// İşletmeden bölgeye pusula yönü (koordinatlardan hesaplanır).
pub fn yon_metni(nereden: (f64, f64), nereye: (f64, f64)) -> &'static str {
    let orta = ((nereden.0 + nereye.0) / 2.0).to_radians();
    let dy = nereye.0 - nereden.0;
    let dx = (nereye.1 - nereden.1) * orta.cos();
    let aci = (dx.atan2(dy).to_degrees() + 360.0) % 360.0;
    let yonler = [
        "kuzey",
        "kuzeydoğu",
        "doğu",
        "güneydoğu",
        "güney",
        "güneybatı",
        "batı",
        "kuzeybatı",
    ];
    yonler[(((aci + 22.5) / 45.0).floor() as usize) % 8]
}

/// İlçenin O-5 üzerindeki gerçek konumu.
///
/// Kavşak numarası ve kilometreler O-5 güzergâh tablosundan; mahalle adları
/// ilçe kayıtlarından alınır. Bilinmeyen alan boş bırakılır ve sayfada çizilmez.
pub struct Rehber {
    pub il: &'static str,
    pub ilce: &'static str,
    pub kavsak: &'static str,
    pub km: &'static str,
    pub yol: &'static str,
    pub servis: &'static str,
    pub mahalleler: &'static str,
    pub not_: &'static str,
}

/// O-5 Kemalpaşa–Savaştepe kesimindeki yedi ilçe.
pub const REHBER: &[Rehber] = &[
    Rehber {
        il: "İzmir",
        ilce: "Kemalpaşa",
        kavsak: "K22 Kemalpaşa Kavşağı",
        km: "389.6 km",
        yol: "D.300",
        servis: "",
        mahalleler: "Akalan, Kuyucak, Ulucak, Çınarköy, Cumalı, Beşpınar",
        not_: "Nif Dağı eteklerinde 48 mahalle; İzmir gişeleri 384. km'de, K21 Akalan ve K23 Kuyucak kavşakları aynı kesimde.",
    },
    Rehber {
        il: "Manisa",
        ilce: "Turgutlu",
        kavsak: "Ankara–İzmir Otoyolu Kavşağı",
        km: "360.0 km",
        yol: "E-96 / D.300",
        servis: "",
        mahalleler: "Avşar, Ayvacık, Baktırlı, Bozkır, Çampınar, Urganlı",
        not_: "Turgutlu viyadüğü 402.8 km'de; E-23 İzmir–Ankara karayolu ilçenin güneyinden, demiryolu kuzeyinden geçer. Urganlı kaplıcaları ve Ovacık yaylası ilçe sınırında.",
    },
    Rehber {
        il: "Manisa",
        ilce: "Saruhanlı",
        kavsak: "K19 Saruhanlı Kavşağı",
        km: "339.6 km",
        yol: "D.565",
        servis: "",
        mahalleler: "Atatürk, Şehitler, Yılmaz, Adiloba, Alibeyli, Çaltepe",
        not_: "D.565 ile Manisa merkezine bağlanır; 1956'da ilçe olan Saruhanlı tarım ağırlıklı bir ilçedir.",
    },
    Rehber {
        il: "Manisa",
        ilce: "Akhisar",
        kavsak: "K18 Akhisar Kavşağı",
        km: "315.1 km",
        yol: "D.565",
        servis: "Oksijen Akhisar Servis Alanı · 320. km · Ballıca Servis Alanı · 322. km",
        mahalleler: "Hürriyet, Kayalıoğlu, Dereköy, Beyoba, Musalar, Sarılar",
        not_: "Thyateira antik kenti, Manisa–Bandırma demiryolu ve tarıma dayalı sanayi ilçenin ana hatları.",
    },
    Rehber {
        il: "Manisa",
        ilce: "Kırkağaç",
        kavsak: "K17 Kırkağaç Kavşağı",
        km: "299.5 km",
        yol: "D.240",
        servis: "",
        mahalleler: "Cinosman, Hacıhimmet, Hıdırağa, Kadriye, Karaali, Yenimahalle",
        not_: "Yund Dağı eteklerinde kurulu; kavunuyla bilinir, D.240 ile Soma ve Akhisar'a bağlanır.",
    },
    Rehber {
        il: "Manisa",
        ilce: "Soma",
        kavsak: "K16 Soma Kavşağı",
        km: "277.4 km",
        yol: "D.240 / 45.04",
        servis: "",
        mahalleler: "Merkez, Darkale, Cenkyeri",
        not_: "Linyit madenleri ve iki termik santral ilçenin ekonomisini belirler; Ayıtlı ve Döşeme Deresi viyadükleri kavşaktan sonra gelir.",
    },
    Rehber {
        il: "Balıkesir",
        ilce: "Savaştepe",
        kavsak: "K15 Savaştepe Kavşağı",
        km: "262.4 km",
        yol: "10.51 il yolu",
        servis: "Savaştepe Servis Alanı · 266.1 km",
        mahalleler: "Fatih, Hürriyet, İstiklal, Zafer",
        not_: "Rakım 277 m, yüzölçümü 427 km²; Savaştepe Viyadüğü (255.4 km) kavşaktan önce geçilir.",
    },
];

/// İlçenin O-5 rehber kaydı.
pub fn rehber_bilgisi(il: &str, ilce: &str) -> Option<&'static Rehber> {
    REHBER
        .iter()
        .find(|kayit| kayit.il == il && kayit.ilce == ilce)
}

/// Bölge başına güzergâh bilgisi: `(il, ilçe, ana hat, bağlantı notu)`.
///
/// Ana hatlar O-5 / O-4 / O-33 / O-30 / D-100 / D-565 gibi koridorları belirtir;
/// not, ilçeye hangi kavşak ya da çıkıştan girildiğini söyler. Mesafe ve süre
/// ayrıca hesaplandığı için burada sayı yok.
pub const GUZERGAH: &[(&str, &str, &str, &str)] = &[
    ("İstanbul", "Tuzla", "O-4 / D-100", "Tuzla ve Şifa Mahallesi çıkışları"),
    ("İstanbul", "Pendik", "O-4 / D-100", "Pendik kavşağı, sahil hattı"),
    ("İstanbul", "Kartal", "D-100", "Kartal kavşağı, sahil yolu"),
    ("Kocaeli", "Gebze", "O-5 / D-100", "Gebze gişeleri"),
    ("Kocaeli", "Dilovası", "O-5 / D-100", "Dilovası sanayi çıkışı"),
    ("Kocaeli", "Çayırova", "O-4 / D-100", "Çayırova kavşağı"),
    ("Yalova", "Altınova", "O-5", "Altınova feribot bağlantısı"),
    ("Yalova", "Çiftlikköy", "D-575 / O-5", "Çiftlikköy sahili"),
    ("Yalova", "Çınarcık", "D-575", "Çınarcık yolu"),
    ("Bursa", "Orhangazi", "O-5", "Orhangazi çıkışı, İznik yolu"),
    ("Bursa", "Gemlik", "O-5", "Gemlik liman kavşağı"),
    ("Bursa", "Nilüfer", "O-5", "Bursa batı gişeleri"),
    ("Bursa", "Karacabey", "O-5", "Karacabey kavşağı"),
    ("Balıkesir", "Savaştepe", "O-5", "Savaştepe kavşağı"),
    ("Balıkesir", "Merkez", "O-5 / D-565", "Balıkesir kavşağı"),
    ("Balıkesir", "Susurluk", "O-5", "Susurluk kavşağı"),
    ("Manisa", "Soma", "O-5 / D-240", "K16 Soma kavşağı · 277. km"),
    ("Manisa", "Kırkağaç", "O-5", "Kırkağaç kavşağı"),
    ("Manisa", "Akhisar", "O-5", "Akhisar kavşağı"),
    ("Manisa", "Saruhanlı", "O-5 / D-565", "K19 Saruhanlı kavşağı · 339. km"),
    ("Manisa", "Turgutlu", "O-5", "Turgutlu viyadüğü ve Ankara–İzmir kavşağı"),
    ("İzmir", "Kemalpaşa", "O-5 / D.300", "K22 Kemalpaşa kavşağı · 389. km"),
    ("Manisa", "Şehzadeler", "D-565", "Manisa merkez"),
    ("Manisa", "Yunusemre", "D-565 / O-33", "Manisa güney"),
    ("İzmir", "Aliağa", "O-33 / D-550", "Aliağa sanayi çıkışı"),
    ("İzmir", "Menemen", "O-33 / D-550", "Menemen kavşağı"),
    ("İzmir", "Çiğli", "O-33", "Çiğli kavşağı"),
    ("İzmir", "Bornova", "O-30 / O-33", "Bornova tüneli"),
    ("İzmir", "Gaziemir", "O-30", "Gaziemir kavşağı"),
    ("İzmir", "Torbalı", "O-31 / D-550", "Torbalı çıkışı"),
    ("Otoyol", "O-5 Otoyolu", "O-5", "Gebze–İzmir hattı"),
    ("Otoyol", "İzmir–İstanbul Otoyolu", "O-5", "kesintisiz hat"),
];

/// Bölgenin güzergâh notu; tabloda yoksa `None`.
pub fn guzergah(il: &str, ilce: &str) -> Option<(&'static str, &'static str)> {
    GUZERGAH
        .iter()
        .find(|(bolge_il, bolge_ilce, _, _)| *bolge_il == il && *bolge_ilce == ilce)
        .map(|(_, _, yol, not)| (*yol, *not))
}

/// O-5 koridorundaki bölgeler.
pub fn koridor_bolgeleri() -> Vec<(String, String)> {
    KORIDOR
        .iter()
        .map(|(il, ilce)| ((*il).to_string(), (*ilce).to_string()))
        .collect()
}

/// Hedef segment: İzmir ↔ Balıkesir / Savaştepe. 1. sıra önce burada hedeflenir.
pub fn oncelikli_bolgeler() -> Vec<(String, String)> {
    // Yayın sırası: İzmir'den Savaştepe'ye doğru (otoyol etiketleri en sonda).
    const SIRA: [&str; 4] = ["İzmir", "Manisa", "Balıkesir", "Otoyol"];
    let mut liste: Vec<(usize, (String, String))> = KORIDOR
        .iter()
        .filter(|(il, ilce)| {
            matches!(
                (*il, *ilce),
                ("İzmir", _) | ("Manisa", _) | ("Balıkesir", "Savaştepe") | ("Otoyol", _)
            )
        })
        .map(|(il, ilce)| ((*il).to_string(), (*ilce).to_string()))
        .map(|bolge| {
            let sira = SIRA
                .iter()
                .position(|il| *il == bolge.0)
                .unwrap_or(SIRA.len());
            (sira, bolge)
        })
        .collect();
    liste.sort_by_key(|(sira, _)| *sira);
    liste.into_iter().map(|(_, bolge)| bolge).collect()
}

/// Seçim anahtarı: aynı ilçe adı iki ilde geçebildiği için il birlikte tutulur.
pub fn anahtar(il: &str, ilce: &str) -> String {
    format!("{il}|{ilce}")
}

/// Sorgu metnini bağlantıya uygun biçime çevirir (UTF-8, yüzde kodlama).
pub fn adres_kod(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len() * 2);
    for bayt in metin.as_bytes() {
        match bayt {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                cikti.push(*bayt as char);
            }
            b' ' => cikti.push_str("%20"),
            _ => cikti.push_str(&format!("%{bayt:02X}")),
        }
    }
    cikti
}

/// Bölge için harita bağlantısı: bölge aramasını açar, sahte pin oluşturmaz.
pub fn bolge_link(sektor: &str, il: &str, ilce: &str) -> String {
    let sektor = sektor.trim();
    let sorgu = if sektor.is_empty() {
        format!("{ilce}, {il}")
    } else if il == "Otoyol" {
        // Otoyol sayfasında il adı yerine "otoyol" geçsin.
        format!("{sektor} {ilce}")
    } else {
        format!("{sektor} {ilce} {il}")
    };
    format!(
        "https://www.google.com/maps/search/?api=1&query={}",
        adres_kod(&sorgu)
    )
}

/// İşletme sahibine gönderilecek bölgesel çalışma metni.
pub fn mesaj(isletme: &Isletme, il: &str, ilce: &str) -> String {
    let link = bolge_link(&isletme.sektor, il, ilce);
    format!(
        "Merhaba, {ad} için {ilce} bölgesine özel harita ve arama çalışmasını hazırladım.\n\nİlçe için hazırladığım yerel sayfa: {ilce} bölgesinde {sektor} arayan müşteriye işletmenizi gösteriyor.\nBölge bağlantısı: {link}\n\nSayfayı sitenize ekleyip Google işletme profilinize bölge hizmet alanını işaretlediğimizde, {ilce} aramalarında görünürlük artar. Hazır çalışmayı göndermemi ister misiniz?",
        ad = isletme.ad,
        sektor = if isletme.sektor.trim().is_empty() {
            "hizmet"
        } else {
            isletme.sektor.trim()
        },
    )
}

/// Telefon numarasından WhatsApp bağlantısı (`wa.me/90...`).
pub fn wa_link(telefon: &str) -> String {
    let rakam: String = telefon.chars().filter(|k| k.is_ascii_digit()).collect();
    let ulke = if rakam.starts_with("90") {
        rakam
    } else if rakam.starts_with('0') {
        format!("9{}", rakam)
    } else if rakam.len() == 10 {
        format!("90{rakam}")
    } else {
        rakam
    };
    format!("https://wa.me/{ulke}")
}

/// WhatsApp bağlantısı + hazır mesaj (Türkçe karakterler yüzde kodlanır).
pub fn wa_mesaj(telefon: &str, metin: &str) -> String {
    let mut kodlu = String::new();
    for bayt in metin.as_bytes() {
        match bayt {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                kodlu.push(*bayt as char)
            }
            b' ' => kodlu.push('+'),
            _ => kodlu.push_str(&format!("%{bayt:02X}")),
        }
    }
    format!("{}?text={kodlu}", wa_link(telefon))
}

/// Coğrafi meta etiketleri için il plaka kodu.
fn il_kodu(il: &str) -> &'static str {
    match il {
        "Manisa" => "45",
        "İzmir" => "35",
        "Balıkesir" => "10",
        _ => "TR",
    }
}

fn sektor_metni(isletme: &Isletme) -> String {
    if isletme.sektor.trim().is_empty() {
        "Hizmet".to_string()
    } else {
        isletme.sektor.trim().to_string()
    }
}

/// Bölge için yerel açılış sayfası (tek dosya, dış bağımlılık yok).
pub fn sayfa(
    isletme: &Isletme,
    il: &str,
    ilce: &str,
    tarih: &str,
    bolgeler: &[(String, String)],
) -> String {
    let ad = &isletme.ad;
    let sektor = sektor_metni(isletme);
    let telefon = isletme.telefon.trim();
    let adres = isletme.adres.trim();
    let bolge = bolge_link(&sektor, il, ilce);
    // WhatsApp açıldığında mesaj hazır: müşteri sadece konum ekleyip gönderir.
    let wa = wa_mesaj(
        telefon,
        &format!("Merhaba, {ilce} bölgesindeyim. Aracım yolda kaldı; konumumu gönderiyorum."),
    );
    let arama = format!(
        "tel:{}",
        telefon
            .chars()
            .filter(|k| k.is_ascii_digit())
            .collect::<String>()
    );
    let yer = if il == "Otoyol" {
        ilce.to_string()
    } else {
        format!("{il} / {ilce}")
    };
    // Sayfa başına değişen üç değer: mesafe, varış süresi ve harita çerçevesi.
    let (bolge_enlem, bolge_boylam) = bolge_noktasi(il, ilce).unwrap_or(ISLETME_KONUM);
    // Merkez ilçede 0 km görünmesin: en az 3 km gösterilir.
    let km = mesafe_km(ISLETME_KONUM, (bolge_enlem, bolge_boylam)).max(3.0);
    let dk = tahmini_dakika(km);
    // Koridor ölçüsü: Saruhanlı'dan değil, İzmir (Kemalpaşa) girişinden.
    let koridor = koridor_girisinden(ilce);
    let kavsak_kod = kavsak_bilgi(ilce).0;
    let harita_bbox = format!(
        "{:.4},{:.4},{:.4},{:.4}",
        bolge_boylam - 0.045,
        bolge_enlem - 0.030,
        bolge_boylam + 0.045,
        bolge_enlem + 0.030
    );
    let (guzergah_yol, guzergah_not) = guzergah(il, ilce).unwrap_or(("", ""));
    // O-5 rehberi: kavşak, km, bağlantı yolu, servis alanı ve mahalleler.
    let rehber_verisi = rehber_bilgisi(il, ilce);
    let rehber_eki = match rehber_verisi {
        Some(kayit) => format!(" · O-5 {} ({})", kayit.kavsak, kayit.km),
        None => String::new(),
    };
    let mut rehber_karti = String::new();
    if let Some(kayit) = rehber_verisi {
        let satir = |etiket: &str, deger: &str| -> String {
            if deger.is_empty() {
                String::new()
            } else {
                format!("<li><span>{etiket}</span><b>{deger}</b></li>\n")
            }
        };
        let mut icerik = String::new();
        icerik.push_str(&satir("O-5 kavşağı", kayit.kavsak));
        icerik.push_str(&satir("O-5 km taşı", kayit.km));
        icerik.push_str(&satir("Bağlantı yolu", kayit.yol));
        icerik.push_str(&satir("Servis alanı", kayit.servis));
        icerik.push_str(&satir("Mahalleler", kayit.mahalleler));
        rehber_karti = format!(
            "<div class=\"kart rehber-kart\"><h2>{ilce} · O-5 bilgisi</h2>\n<ul class=\"guzergah\">\n{icerik}</ul>\n<p class=\"harita__alt\">{}</p>\n</div>\n",
            kayit.not_
        );
    }
    let guzergah_satiri = if guzergah_yol.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"harita__guzergah\"><b>Güzergâh</b> {guzergah_yol} · {guzergah_not}</p>\n"
        )
    };
    let yol_tarifi = format!(
        "https://www.google.com/maps/dir/?api=1&origin={:.6},{:.6}&destination={:.6},{:.6}",
        ISLETME_KONUM.0, ISLETME_KONUM.1, bolge_enlem, bolge_boylam
    );
    let mut h = String::new();
    h.push_str("<!doctype html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n");
    h.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    h.push_str(&format!(
        "<meta name=\"geo.region\" content=\"TR-{il_kodu}\">\n<meta name=\"geo.placename\" content=\"{ilce}\">\n<meta name=\"geo.position\" content=\"{bolge_enlem:.6};{bolge_boylam:.6}\">\n<meta name=\"ICBM\" content=\"{bolge_enlem:.6}, {bolge_boylam:.6}\">\n<title>Çekici {ilce} · 7/24{baslik_eki} | {ad}</title>\n<meta name=\"robots\" content=\"index,follow,max-image-preview:large\">\n<meta name=\"description\" content=\"{ad}, {ilce} ({il}) bölgesinde {sektor_l} hizmeti. {kavsak_kod} kavşağı · İzmir girişinden {koridor:.0} km · ortalama varış 20–30 dk. 7/24 telefon, konum ve iletişim bilgileri bu sayfada.\">\n<meta property=\"og:image\" content=\"{ALAN_ADI}/video/film-poster.jpg?v=3\">\n<meta property=\"og:image:width\" content=\"1024\">\n<meta property=\"og:image:height\" content=\"1536\">\n<meta property=\"og:image:alt\" content=\"{ilce} bölgesinde çekici ve oto kurtarma\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n",
        il_kodu = il_kodu(il),
        baslik_eki = match rehber_verisi {
            Some(kayit) => format!(" · O-5 {}", kayit.kavsak.split(' ').next().unwrap_or("")),
            None => String::new(),
        },
        sektor_l = sektor.to_lowercase()
    ));
    h.push_str(&format!(
        "<meta property=\"og:title\" content=\"Çekici {ilce} · 7/24 | {ad}\">\n<meta property=\"og:type\" content=\"website\">\n"
    ));
    // Canonical: aynı bölge sayfasının iki adresten görünmesini engeller.
    h.push_str(&format!(
        "<meta property=\"og:url\" content=\"{ALAN_ADI}/{sayfa}\">\n<meta property=\"og:locale\" content=\"tr_TR\">\n<link rel=\"canonical\" href=\"{ALAN_ADI}/{sayfa}\">\n",
        sayfa = sayfa_url(il, ilce)
    ));
    h.push_str("<style>\n:root{--v:#12523f;--z:#f6f8f5;--k:#111}\n");
    h.push_str("*{box-sizing:border-box}body{margin:0;font:16px/1.6 system-ui,Segoe UI,Arial;color:var(--k);background:var(--z)}\n");
    h.push_str("header{background:var(--v);color:#fff;padding:48px 20px}\n.wrap{max-width:900px;margin:0 auto}\n");
    h.push_str("h1{font-size:34px;line-height:1.2;margin:0 0 10px}\nh2{font-size:22px;margin:32px 0 8px}\n");
    h.push_str(".alt{opacity:.85}\n.dugme{display:inline-block;background:#fff;color:var(--v);padding:12px 20px;border-radius:999px;text-decoration:none;font-weight:600;margin:6px 6px 0 0}\n");
    h.push_str(".dugme.wa{background:#25d366;color:#08301b}\nmain{padding:28px 20px}\n.kart{background:#fff;border:1px solid #dfe6df;border-radius:14px;padding:18px;margin:14px 0}\n");
    // Konum kartı: bölgeye yaklaşan gömülü harita, üstünde mesafe/süre rozeti.
    h.push_str(".harita{position:relative;aspect-ratio:16/9;border-radius:12px;overflow:hidden;background:#e7ede7;border:1px solid #dfe6df}\n");
    h.push_str(".harita iframe{border:0;width:100%;height:100%;display:block}\n");
    h.push_str(".harita__rozet{position:absolute;left:10px;bottom:10px;display:flex;align-items:baseline;gap:6px;background:#0f2c22;color:#fff;padding:6px 11px;border-radius:999px;font:600 12px/1 system-ui}\n");
    h.push_str(".harita__rozet small{font-weight:500;font-size:11px;opacity:.82}\n");
    h.push_str(".harita__alt{font-size:13px;color:#4a554e;margin:8px 0 0}\n");
    h.push_str(".harita__guzergah{font-size:13px;color:#28362d;margin:6px 0 0;padding:8px 10px;background:#eef3ee;border-radius:8px}\n");
    h.push_str(".harita__guzergah b{font-weight:600;margin-right:6px}\n");
    h.push_str(".guzergah-kart .guzergah{list-style:none;margin:0;padding:0;display:grid;gap:2px}\n");
    h.push_str(".guzergah li{display:flex;flex-wrap:wrap;gap:8px;font-size:14px;padding:9px 0;border-bottom:1px solid #e6ece6}\n");
    h.push_str(".guzergah li:last-child{border-bottom:0}\n.guzergah li span{flex:0 0 130px;color:#5b665f}\n.guzergah li b{font-weight:600}\n");
    h.push_str("@media(max-width:600px){.guzergah li span{flex-basis:100%}}\n");
    h.push_str("ul{margin:8px 0;padding-left:20px}\nfooter{color:#5b665f;font-size:13px;padding:18px 20px;border-top:1px solid #dfe6df}\n");
    h.push_str("a{color:#0d6b52}\n@media(max-width:600px){h1{font-size:27px}}\n");
    h.push_str(".durumlar{list-style:none;margin:10px 0 0;padding:0;display:grid;gap:8px}\n");
    h.push_str(".durumlar li{background:#f2f6f2;border:1px solid #e2eae2;border-radius:10px;padding:10px 12px;display:grid;gap:2px}\n");
    h.push_str(".durumlar li b{font-size:15px}\n.durumlar li span{font-size:13.5px;color:#5b665f}\n");
    h.push_str(".durumlar + p{margin:14px 0 0}\n");
    // Mobilde sayfanın altında duran arama çubuğu: tek dokunuşla telefon.
    h.push_str(".arama-cubugu{display:none}\n");
    h.push_str("@media(max-width:760px){body{padding-bottom:80px}\n");
    h.push_str(".arama-cubugu{display:flex;gap:8px;position:fixed;left:0;right:0;bottom:0;z-index:60;padding:10px 14px calc(10px + env(safe-area-inset-bottom));background:rgba(18,82,63,.97)}\n");
    h.push_str(".arama-cubugu a{flex:1;text-align:center;text-decoration:none;font-weight:700;font-size:15px;padding:13px 10px;border-radius:999px}\n");
    h.push_str(".arama-cubugu__tel{background:#fff;color:#12523f}\n.arama-cubugu__wa{background:#25d366;color:#08301b;flex:0 0 40%}\n}\n");
    h.push_str("</style>\n");
    h.push_str("</head>\n<body>\n<header><div class=\"wrap\">\n");
    h.push_str(&format!("<h1>{ilce} çekici · 7/24 {sektor_l}</h1>\n", sektor_l = sektor.to_lowercase()));
    h.push_str(&format!(
        "<p class=\"alt\">{ilce} ve çevresinde 7/24 {sektor_l} hizmeti. {kavsak_kod} kavşağı · İzmir girişinden {koridor:.0} km · ortalama varış 20–30 dk{guzergah_eki}{rehber_eki}</p>\n",
        guzergah_eki = if guzergah_yol.is_empty() {
            String::new()
        } else {
            format!(" · {guzergah_yol} üzerinden {guzergah_not}")
        },
        sektor_l = sektor.to_lowercase()
    ));
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<a class=\"dugme\" href=\"{arama}\">{telefon}</a>\n<a class=\"dugme wa\" href=\"{wa}\" target=\"_blank\" rel=\"noopener\">WhatsApp</a>\n"
        ));
    }
    h.push_str(&format!(
        "<a class=\"dugme\" href=\"{bolge}\" target=\"_blank\" rel=\"noopener\">Bölgeyi haritada gör</a>\n"
    ));
    h.push_str("</div></header>\n<main class=\"wrap\">\n");
    h.push_str(&format!(
        "<div class=\"kart\"><h2>{ilce} bölgesinde {}</h2>\n",
        sektor.to_lowercase()
    ));
    h.push_str(&format!(
        "<p>{ad}, {adres} adresinden {ilce} bölgesine hizmet veriyor. Aşağıdaki hizmetler için günün her saati dönüş yapılır.</p>\n"
    ));
    // Hizmet listesi pinlerdeki 8 anahtar kelimeyle aynı: arama ve sayfa metni örtüşsün.
    h.push_str("<ul>\n");
    for hizmet in crate::otoyol_pin::HIZMETLER {
        // "yol yardım 7/24" gibi kelimelerde 7/24 ikinci kez yazılmasın.
        let satir = if hizmet.contains("7/24") {
            (*hizmet).to_string()
        } else {
            format!("{hizmet} · 7/24")
        };
        h.push_str(&format!("<li>{satir}</li>\n"));
    }
    h.push_str("</ul>\n");
    h.push_str(&format!(
        "<p>{ilce} bölgesinde çekici, oto kurtarma ve en yakın çekici aramalarında bize ulaşabilirsiniz. Yakıt biterse benzin destek ve mazot takviye, akü biterse akü takviye ve akücü hizmeti veriyoruz.</p>\n"
    ));
    h.push_str("</div>\n");
    // Konum kartı: harita bölge merkezine yaklaşır, rozet mesafe ve süreyi taşır.
    h.push_str(&format!(
        "<div class=\"kart harita-kart\"><h2>{ilce} haritada</h2>\n\
         <div class=\"harita\">\n\
         <iframe src=\"https://www.openstreetmap.org/export/embed.html?bbox={harita_bbox}&amp;layer=mapnik&amp;marker={bolge_enlem:.5}%2C{bolge_boylam:.5}\" loading=\"lazy\" title=\"{ilce} haritası\" referrerpolicy=\"no-referrer\"></iframe>\n\
         <span class=\"harita__rozet\"><b>{koridor:.0} km</b><small>girişten</small></span>\n\
         </div>\n\
         <p class=\"harita__alt\">İzmir girişinden {koridor:.0} km · ortalama 20–30 dk · <a href=\"{yol_tarifi}\" target=\"_blank\" rel=\"noopener\">Yol tarifi</a></p>\n\
         {guzergah_satiri}\
         <p class=\"harita__alt\">Harita yüklenmezse <a href=\"{bolge}\" target=\"_blank\" rel=\"noopener\">bölgeyi Google Haritalar'da aç</a>.</p>\n\
         </div>\n"
    ));
    // O-5 rehber kartı: kavşak, km, bağlantı yolu ve mahalleler.
    h.push_str(&rehber_karti);
    // Güzergâh kartı: ana hat, çıkış ve yön bölgeye göre değişir.
    if !guzergah_yol.is_empty() {
        let yon = yon_metni(ISLETME_KONUM, (bolge_enlem, bolge_boylam));
        h.push_str(&format!(
            "<div class=\"kart guzergah-kart\"><h2>{ilce} için güzergâh</h2>\n\
             <ul class=\"guzergah\">\n\
             <li><span>Ana hat</span><b>{guzergah_yol}</b></li>\n\
             <li><span>Çıkış / kavşak</span><b>{guzergah_not}</b></li>\n\
             <li><span>Yön</span><b>{yon} · İzmir girişinden {koridor:.0} km</b></li>\n\
             </ul>\n\
             <p class=\"harita__alt\">Tahmini varış {dk} dk · <a href=\"{yol_tarifi}\" target=\"_blank\" rel=\"noopener\">Bu güzergâhta yol tarifi al</a></p>\n\
             </div>\n"
        ));
    }
    h.push_str(&format!(
        "<div class=\"kart\"><h2>İletişim</h2>\n<p>Adres: {adres}</p>\n"
    ));
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<p>Telefon: <a href=\"{arama}\">{telefon}</a> · <a href=\"{wa}\">WhatsApp</a></p>\n"
        ));
    }
    h.push_str(&format!(
        "<p>Bölge: {yer}</p>\n<p>Tüm bölgeler: <a href=\"/\">hizmet verdiğimiz bölgeler</a></p>\n<p>Hizmetten memnun kaldıysanız <a href=\"{YORUM_LINK}\" target=\"_blank\" rel=\"noopener\">Google'da 5 yıldız verin</a>; tek cümle yeter, yerel sıralamada bize en çok bu yardımcı oluyor.</p></div>\n"
    ));
    // İç linkleme: komşu bölge sayfaları (dizin derinliğini azaltır).
    if !bolgeler.is_empty() {
        // Komşuluk sıraya göre değil gerçek mesafeye göre: her sayfa farklı üç bölgeye bağlanır.
        let mut uzakliklar: Vec<(f64, &(String, String))> = bolgeler
            .iter()
            .filter(|(bolge_il, bolge_ilce)| !(bolge_il == il && bolge_ilce == ilce))
            .map(|bolge| {
                let nokta = bolge_noktasi(&bolge.0, &bolge.1).unwrap_or(ISLETME_KONUM);
                (mesafe_km((bolge_enlem, bolge_boylam), nokta), bolge)
            })
            .collect();
        uzakliklar.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let komsular: Vec<&(String, String)> = uzakliklar.into_iter().take(3).map(|(_, bolge)| bolge).collect();
        if !komsular.is_empty() {
            h.push_str("<div class=\"kart\"><h2>Yakın bölgeler</h2><ul>\n");
            for (komsu_il, komsu_ilce) in komsular {
                let komsu_etiket = if komsu_il == "Otoyol" {
                    komsu_ilce.clone()
                } else {
                    format!("{komsu_ilce} · {komsu_il}")
                };
                h.push_str(&format!(
                    "<li><a href=\"{}\">{komsu_etiket}</a></li>\n",
                    sayfa_url(komsu_il, komsu_ilce)
                ));
            }
            h.push_str("</ul></div>\n");
        }
    }

    // O-5 sırası: koridordaki bir önceki ve sonraki ilçe.
    if let Some(konum) = KORIDOR.iter().position(|(a, b)| *a == il && *b == ilce) {
        let mut gezinme = String::new();
        if konum > 0 {
            let (onceki_il, onceki_ilce) = KORIDOR[konum - 1];
            gezinme.push_str(&format!(
                "<a href=\"{}\">← {onceki_ilce}</a>",
                sayfa_url(onceki_il, onceki_ilce)
            ));
        }
        if let Some((sonraki_il, sonraki_ilce)) = KORIDOR.get(konum + 1).copied() {
            if !gezinme.is_empty() {
                gezinme.push_str(" · ");
            }
            gezinme.push_str(&format!(
                "<a href=\"{}\">{sonraki_ilce} →</a>",
                sayfa_url(sonraki_il, sonraki_ilce)
            ));
        }
        if !gezinme.is_empty() {
            h.push_str(&format!(
                "<div class=\"kart\"><h2>O-5 üzerinde komşu ilçeler</h2><p>{gezinme}</p><p class=\"harita__alt\">Koridor: İzmir Kemalpaşa → Balıkesir Savaştepe</p></div>\n"
            ));
        }
    }
    // Nasıl çalışıyoruz: sayfaya özgün metin ekler, ince içerik riskini azaltır.
    h.push_str("<div class=\"kart\"><h2>Nasıl çalışıyoruz</h2><ol>\n");
    h.push_str("<li>Telefon veya WhatsApp'tan konumunuzu iletirsiniz.</li>\n");
    h.push_str(
        "<li>En yakın ekibi yönlendiririz; tahmini varış süresi ve fiyat baştan söylenir.</li>\n",
    );
    h.push_str("<li>Araç güvenli şekilde yüklenir, istediğiniz servise bırakılır.</li>\n");
    h.push_str("</ol></div>\n");
    // Durum kartı: müşteri kendi arızasını bulup tek dokunuşla arar.
    h.push_str(&format!("<div class=\"kart\"><h2>{ilce} bölgesinde hangi durumda ararsınız?</h2><ul class=\"durumlar\">\n"));
    for (baslik, aciklama) in [
        ("Akü bitti, araç çalışmıyor", "Yerinde akü takviyesi; gerekirse akü değişimi"),
        ("Yakıt bitti", "Benzin destek veya mazot takviye"),
        ("Lastik patladı", "Yerinde lastik değişimi, olmuyorsa çekici"),
        ("Kaza veya şarampol", "Güvenli bölgeye alma ve kurtarma"),
        ("Araç kilitli kaldı", "Anahtar ve kilit desteği"),
        ("Motor arızası", "İstediğiniz servise hasarsız teslim"),
    ] {
        h.push_str(&format!("<li><b>{baslik}</b><span>{aciklama}</span></li>\n"));
    }
    h.push_str("</ul>");
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<p><a class=\"dugme\" style=\"background:#12523f;color:#fff\" href=\"{arama}\">Hemen ara · {telefon}</a></p>"
        ));
    }
    h.push_str("</div>\n");
    h.push_str("<div class=\"kart\"><h2>Sık sorulan</h2>\n");
    let mut sss: Vec<(String, String)> = vec![
        (
            format!("{ilce} bölgesine ne kadar sürede geliyorsunuz?"),
            format!("{ilce} kavşağına İzmir girişinden {koridor:.0} km. Bölgede konuşlu ekibimizle ortalama 20–30 dakikada ulaşıyoruz; yoğun saatlerde net süre telefonla verilir."),
        ),
        (
            format!("Fiyat {ilce} için farklı mı?"),
            "Bölge mesafesine göre değişir; kesin fiyat telefonla baştan söylenir.".to_string(),
        ),
        (
            "Gece de geliyor musunuz?".to_string(),
            "Evet, 7/24 çalışıyoruz.".to_string(),
        ),
        (
            "Akü takviyesi ve akücü hizmeti var mı?".to_string(),
            format!("Evet; {ilce} bölgesinde akü takviye ve akücü hizmeti 7/24 verilir, akü değişimi gerekirse en yakın noktaya yönlendiriliriz."),
        ),
        (
            "Benzin veya mazot getiriyor musunuz?".to_string(),
            format!("Evet; benzin destek ve mazot takviye yapıyoruz. Yakıt bittiyse {ilce} içindeki en yakın istasyondan yakıt getirilir."),
        ),
    ];
    // O-5 çıkış sorusu: kavşak ve bağlantı yolu ile yanıtlanır.
    if let Some(kayit) = rehber_verisi {
        sss.push((
            format!("{ilce} bölgesine O-5'ten nasıl çıkıyorum?"),
            format!(
                "{} ({}), {} bağlantı yolundan {ilce} merkezine iner. İzmir girişinden {koridor:.0} km, bölgede konuşlu ekiple ortalama 20–30 dakika.",
                kayit.kavsak, kayit.km, kayit.yol
            ),
        ));
    }
    // Güzergâh sorusu: yalnız tabloda karşılığı olan bölgelerde eklenir.
    if !guzergah_yol.is_empty() {
        sss.insert(
            1,
            (
                format!("{ilce} bölgesine hangi çıkıştan geliyorsunuz?"),
                format!("{guzergah_yol} hattında {guzergah_not} yönüne ilerleyin, ardından {ilce} merkez tabelalarını takip edin. İzmir girişinden {koridor:.0} km, bölgede konuşlu ekiple ortalama 20–30 dakika."),
            ),
        );
    }
    for (soru, cevap) in &sss {
        h.push_str(&format!("<p><strong>{soru}</strong><br>{cevap}</p>\n"));
    }
    // Kapanış CTA'sı: SSS'den sonra son bir arama fırsatı.
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<div class=\"kart\"><h2>{ilce} için hemen ara</h2><p>Gece, hafta sonu, resmi tatil: hat 7/24 açık. Konumunuzu söyleyin, en yakın ekibi yönlendirelim.</p><a class=\"dugme\" style=\"background:#12523f;color:#fff\" href=\"{arama}\">Ara · {telefon}</a> <a class=\"dugme wa\" href=\"{wa}\" target=\"_blank\" rel=\"noopener\">WhatsApp</a></div>\n"
        ));
    }
    h.push_str("</div>\n</main>\n");
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<div class=\"arama-cubugu\"><a class=\"arama-cubugu__tel\" href=\"{arama}\">Hemen ara · {telefon}</a><a class=\"arama-cubugu__wa\" href=\"{wa}\" target=\"_blank\" rel=\"noopener\">WhatsApp</a></div>\n"
        ));
    }
    h.push_str(&format!(
        "<footer>© {tarih} {ad} · {yer} · Bu sayfa bölgesel arama çalışması için hazırlandı.</footer>\n"
    ));
    // Yapısal veri: bölge + il hizmet alanı, 7/24 saat ve 8 hizmetlik katalog.
    let teklifler: Vec<serde_json::Value> = crate::otoyol_pin::HIZMETLER
        .iter()
        .map(|hizmet| {
            serde_json::json!({
                "@type": "Offer",
                // Hizmet adı sade kalır; bölge bilgisi areaServed alanında durur.
                "itemOffered": { "@type": "Service", "name": hizmet }
            })
        })
        .collect();
    // NAP: adres işletmenin kendi konumudur; hedef ilçe yalnızca areaServed'da durur.
    // Posta kodu `Isletme` üzerinden gelir; boşsa şemaya yazılmaz.
    let isletme_adresi = isletme.adres_verisi();
    let veri = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "AutoRepair",
        "name": ad,
        "telephone": telefon,
        "contactPoint": {
            "@type": "ContactPoint",
            "telephone": telefon,
            "contactType": "emergency",
            "availableLanguage": ["Turkish"],
            "areaServed": ilce
        },
        "aggregateRating": {
            "@type": "AggregateRating",
            "ratingValue": PUAN,
            "reviewCount": YORUM_SAYISI,
            "bestRating": 5
        },
        "sameAs": [
            format!(
                "https://www.google.com/maps/search/?api=1&query={:.6},{:.6}",
                ISLETME_KONUM.0, ISLETME_KONUM.1
            ),
            "https://wa.me/905071299439"
        ],
        "priceRange": "₺₺",
        "geo": {
            "@type": "GeoCoordinates",
            "latitude": ISLETME_KONUM.0,
            "longitude": ISLETME_KONUM.1
        },
        "hasMap": format!(
            "https://www.google.com/maps/search/?api=1&query={:.6},{:.6}",
            ISLETME_KONUM.0, ISLETME_KONUM.1
        ),
        "address": isletme_adresi,
        "areaServed": [
            { "@type": "AdministrativeArea", "name": ilce },
            { "@type": "AdministrativeArea", "name": il }
        ],
        "openingHoursSpecification": {
            "@type": "OpeningHoursSpecification",
            "dayOfWeek": ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
            "opens": "00:00",
            "closes": "23:59"
        },
        "hasOfferCatalog": {
            "@type": "OfferCatalog",
            "name": format!("{sektor} hizmetleri"),
            "itemListElement": teklifler
        }
    });
    h.push_str("<script type=\"application/ld+json\">\n");
    h.push_str(&serde_json::to_string(&veri).unwrap_or_default());
    h.push('\n');
    h.push_str("</script>\n");
    // Kırıntı: sayfanın koridor içindeki yeri (dizin → il → ilçe).
    let kirinti = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": [
            { "@type": "ListItem", "position": 1, "name": "Bölgeler", "item": format!("{ALAN_ADI}/") },
            { "@type": "ListItem", "position": 2, "name": il, "item": format!("{ALAN_ADI}/") },
            { "@type": "ListItem", "position": 3, "name": ilce, "item": format!("{ALAN_ADI}/{}", sayfa_url(il, ilce)) }
        ]
    });
    h.push_str("<script type=\"application/ld+json\">\n");
    h.push_str(&serde_json::to_string(&kirinti).unwrap_or_default());
    h.push_str("\n</script>\n");
    // Hizmet şeması: çekici hizmeti + ilçe + 7/24 telefon kanalı.
    let hizmet_veri = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "Service",
        "serviceType": format!("{sektor} ve çekici hizmeti"),
        "provider": { "@type": "AutoRepair", "name": ad, "telephone": telefon },
        "areaServed": { "@type": "AdministrativeArea", "name": ilce },
        "availableChannel": {
            "@type": "ServiceChannel",
            "servicePhone": { "@type": "ContactPoint", "telephone": telefon, "contactType": "emergency" }
        }
    });
    h.push_str("<script type=\"application/ld+json\">\n");
    h.push_str(&serde_json::to_string(&hizmet_veri).unwrap_or_default());
    h.push_str("\n</script>\n");
    // SSS yapısal verisi: Google soru-cevap zengin sonuçlarını açar.
    let sss_veri = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": sss
            .iter()
            .map(|(soru, cevap)| serde_json::json!({
                "@type": "Question",
                "name": soru,
                "acceptedAnswer": { "@type": "Answer", "text": cevap }
            }))
            .collect::<Vec<_>>()
    });
    h.push_str("<script type=\"application/ld+json\">\n");
    h.push_str(&serde_json::to_string(&sss_veri).unwrap_or_default());
    h.push('\n');
    h.push_str("</script>\n</body>\n</html>\n");
    h
}

/// İşletmenin kendi ana sayfası: tüm bölgeler tek sayfada, arama düğmeleri.
pub fn ana_sayfa(isletme: &Isletme, bolgeler: &[(String, String)], tarih: &str) -> String {
    let sektor = sektor_metni(isletme);
    let telefon = isletme.telefon.trim();
    let wa = wa_mesaj(
        telefon,
        "Merhaba, aracım yolda kaldı; konumumu gönderiyorum.",
    );
    let arama = format!(
        "tel:{}",
        telefon
            .chars()
            .filter(|k| k.is_ascii_digit())
            .collect::<String>()
    );
    let mut h = String::new();
    h.push_str("<!doctype html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n");
    h.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    h.push_str(&format!(
        "<title>{sektor} · O-5 Kemalpaşa–Savaştepe | {ad}</title>\n<meta name=\"description\" content=\"{ad} · O-5 otoyolunda Kemalpaşa'dan Savaştepe'ye 7 ilçede 7/24 {sektor_l}. Tek numara, tek fiyat: hemen arayın.\">\n",
        ad = isletme.ad,
        sektor_l = sektor.to_lowercase()
    ));
    h.push_str(&format!(
        "<meta property=\"og:url\" content=\"{ALAN_ADI}/\">\n<meta property=\"og:locale\" content=\"tr_TR\">\n<link rel=\"canonical\" href=\"{ALAN_ADI}/\">\n"
    ));
    h.push_str("<style>\n:root{--v:#0f3d2e;--z:#f5f7f3;--k:#101413}\n");
    h.push_str("*{box-sizing:border-box}body{margin:0;font:16px/1.6 system-ui,Segoe UI,Arial;color:var(--k);background:var(--z)}\n");
    h.push_str(".wrap{max-width:980px;margin:0 auto;padding:0 20px}\nheader{background:var(--v);color:#fff;padding:56px 0 44px}\n");
    h.push_str("h1{font-size:40px;line-height:1.15;margin:0 0 12px;letter-spacing:-.5px}\nh2{font-size:24px;margin:34px 0 10px}\n");
    h.push_str(".etiket{display:inline-block;border:1px solid rgba(255,255,255,.35);border-radius:999px;padding:4px 12px;font-size:12px;margin-bottom:14px}\n");
    h.push_str(".dugme{display:inline-block;background:#fff;color:var(--v);padding:14px 24px;border-radius:999px;text-decoration:none;font-weight:700;margin:10px 8px 0 0}\n");
    h.push_str(".dugme.wa{background:#25d366;color:#08301b}\nmain{padding:10px 0 20px}\n");
    h.push_str(".kart{background:#fff;border:1px solid #dee5dd;border-radius:16px;padding:20px;margin:14px 0}\n");
    h.push_str(
        ".izgara{display:grid;grid-template-columns:repeat(auto-fill,minmax(210px,1fr));gap:8px}\n",
    );
    h.push_str(".bolge{display:block;background:#fff;border:1px solid #dee5dd;border-radius:10px;padding:10px 12px;text-decoration:none;color:#123;font-size:14px}\n");
    h.push_str(".bolge:hover{border-color:#0f3d2e}\nfooter{color:#5b665f;font-size:13px;padding:22px 0;border-top:1px solid #dee5dd;margin-top:20px}\n");
    h.push_str("ul{margin:8px 0;padding-left:20px}\n@media(max-width:640px){h1{font-size:29px}}\n");
    h.push_str(".arama-cubugu{display:none}\n");
    h.push_str("@media(max-width:760px){body{padding-bottom:80px}\n");
    h.push_str(".arama-cubugu{display:flex;gap:8px;position:fixed;left:0;right:0;bottom:0;z-index:60;padding:10px 14px calc(10px + env(safe-area-inset-bottom));background:rgba(15,61,46,.97)}\n");
    h.push_str(".arama-cubugu a{flex:1;text-align:center;text-decoration:none;font-weight:700;font-size:15px;padding:13px 10px;border-radius:999px}\n");
    h.push_str(".arama-cubugu__tel{background:#fff;color:#0f3d2e}\n.arama-cubugu__wa{background:#25d366;color:#08301b;flex:0 0 40%}\n}\n");
    h.push_str("</style>\n</head>\n<body>\n");
    h.push_str("<header><div class=\"wrap\">\n<span class=\"etiket\">7/24 · O-5 · Kemalpaşa – Savaştepe</span>\n");
    let baslik_sektor = {
        let mut harfler = sektor.chars();
        match harfler.next() {
            Some(ilk) => ilk.to_uppercase().collect::<String>() + harfler.as_str(),
            None => sektor.clone(),
        }
    };
    h.push_str(&format!(
        "<h1>{baslik_sektor} · {ad}</h1>\n",
        ad = isletme.ad
    ));
    h.push_str(&format!(
        "<p style=\"opacity:.9;max-width:640px\">{adres} merkezli ekibimiz O-5 otoyolunda Kemalpaşa'dan Savaştepe'ye kadar 7 ilçede 7/24 {sektor_l} hizmeti veriyor. Tek numara, baştan söylenen net fiyat.</p>\n",
        adres = if isletme.adres.trim().is_empty() {
            "Manisa"
        } else {
            isletme.adres.trim()
        },
        sektor_l = sektor.to_lowercase()
    ));
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<a class=\"dugme\" href=\"{arama}\">Hemen ara · {telefon}</a>\n<a class=\"dugme wa\" href=\"{wa}\" target=\"_blank\" rel=\"noopener\">WhatsApp'tan yaz</a>\n"
        ));
    }
    h.push_str("</div></header>\n<main><div class=\"wrap\">\n");
    h.push_str("<div class=\"kart\"><h2>Hizmetler</h2><ul>\n");
    for hizmet in crate::otoyol_pin::HIZMETLER {
        h.push_str(&format!("<li>{hizmet}</li>\n"));
    }
    h.push_str("</ul>\n");
    h.push_str(
        "<p>İzmir'den Balıkesir Savaştepe'ye kadar O-5 hattında çekici, oto kurtarma ve yol yardım 7/24. Yakıt biterse benzin destek ve mazot takviye; akü biterse akü takviye ve akücü.</p>\n",
    );
    h.push_str("</div>\n");
    h.push_str("<div class=\"kart\"><h2>Neden biz</h2><ul>\n");
    h.push_str("<li>Telefonda baştan net fiyat, sonradan ek ücret yok</li>\n");
    h.push_str("<li>Ortalama 20-30 dakikada olay yerinde</li>\n");
    h.push_str("<li>Gerçek konum, haritada doğrulanabilir</li>\n</ul></div>\n");
    h.push_str(&format!(
        "<h2>Hizmet verdiğimiz bölgeler ({} nokta)</h2>\n<div class=\"izgara\">\n",
        bolgeler.len()
    ));
    for (il, ilce) in bolgeler {
        let dosya = sayfa_url(il, ilce);
        let etiket = if il == "Otoyol" {
            ilce.clone()
        } else {
            format!("{ilce} · {il}")
        };
        h.push_str(&format!(
            "<a class=\"bolge\" href=\"{dosya}\">{etiket}</a>\n"
        ));
    }
    h.push_str("</div>\n");
    // O-5 sırası: koridordaki yedi ilçe, kuzeyden güneye.
    h.push_str("<div class=\"kart\"><h2>O-5 üzerinde hizmet verdiğimiz kesim</h2><p>İzmir Kemalpaşa → Balıkesir Savaştepe</p><ul>\n");
    for kayit in REHBER.iter().rev() {
        h.push_str(&format!(
            "<li><a href=\"{}\">{}</a> · {} · {}</li>\n",
            sayfa_url(kayit.il, kayit.ilce),
            kayit.ilce,
            kayit.kavsak,
            kayit.km
        ));
    }
    h.push_str("</ul></div>\n");
    if !telefon.is_empty() {
        h.push_str(&format!(
            "<div class=\"kart\"><h2>Aracınız yolda mı kaldı?</h2><p>Bulunduğunuz yeri WhatsApp'tan gönderin, en yakın ekibi yönlendirelim.</p><a class=\"dugme\" style=\"background:#0f3d2e;color:#fff\" href=\"{wa}\" target=\"_blank\" rel=\"noopener\">Konum gönder</a></div>\n"
        ));
    }
    h.push_str(&format!(
        "<div class=\"kart\"><h2>Yorumunuz sıralamayı belirliyor</h2><p>Google'da 5,0 puan ve {YORUM_SAYISI} yorumla görünüyoruz. Hizmetten memnun kaldıysanız tek cümlelik bir değerlendirme, bölgenizde üst sırada çıkmamızı sağlıyor.</p><a class=\"dugme\" style=\"background:#0f3d2e;color:#fff\" href=\"{YORUM_LINK}\" target=\"_blank\" rel=\"noopener\">Google'da değerlendir</a></div>\n<footer>© {tarih} {ad} · {telefon} · O-5 · Kemalpaşa – Savaştepe</footer>\n</div></main>\n<div class=\"arama-cubugu\"><a class=\"arama-cubugu__tel\" href=\"{arama}\">Hemen ara · {telefon}</a><a class=\"arama-cubugu__wa\" href=\"{wa}\" target=\"_blank\" rel=\"noopener\">WhatsApp</a></div>\n",
        ad = isletme.ad
    ));
    // Ana sayfa yapısal verisi: tüm hizmet alanları tek bir işletme altında.
    let alanlar: Vec<serde_json::Value> = bolgeler
        .iter()
        .map(|(il, ilce)| {
            serde_json::json!({
                "@type": "AdministrativeArea",
                "name": if il == "Otoyol" { ilce.clone() } else { format!("{ilce}, {il}") }
            })
        })
        .collect();
    let teklifler: Vec<serde_json::Value> = crate::otoyol_pin::HIZMETLER
        .iter()
        .map(|hizmet| {
            serde_json::json!({
                "@type": "Offer",
                "itemOffered": { "@type": "Service", "name": hizmet }
            })
        })
        .collect();
    let veri = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "AutoRepair",
        "name": isletme.ad,
        "telephone": telefon,
        "contactPoint": {
            "@type": "ContactPoint",
            "telephone": telefon,
            "contactType": "emergency",
            "availableLanguage": ["Turkish"],
            "areaServed": "Kemalpaşa, Turgutlu, Saruhanlı, Akhisar, Kırkağaç, Soma, Savaştepe"
        },
        "url": format!("{ALAN_ADI}/"),
        "aggregateRating": {
            "@type": "AggregateRating",
            "ratingValue": PUAN,
            "reviewCount": YORUM_SAYISI,
            "bestRating": 5
        },
        "address": isletme.adres_verisi(),
        "geo": {
            "@type": "GeoCoordinates",
            "latitude": ISLETME_KONUM.0,
            "longitude": ISLETME_KONUM.1
        },
        "hasMap": format!(
            "https://www.google.com/maps/search/?api=1&query={:.6},{:.6}",
            ISLETME_KONUM.0, ISLETME_KONUM.1
        ),
        "openingHoursSpecification": {
            "@type": "OpeningHoursSpecification",
            "dayOfWeek": ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
            "opens": "00:00",
            "closes": "23:59"
        },
        "areaServed": alanlar,
        "hasOfferCatalog": {
            "@type": "OfferCatalog",
            "name": format!("{sektor} hizmetleri"),
            "itemListElement": teklifler
        }
    });
    h.push_str("<script type=\"application/ld+json\">\n");
    h.push_str(&serde_json::to_string(&veri).unwrap_or_default());
    h.push('\n');
    h.push_str("</script>\n</body>\n</html>\n");
    h
}

/// Bölge planı: her nokta için yapılacaklar listesi.
pub fn plan(isletme: &Isletme, bolgeler: &[(String, String)], tarih: &str) -> String {
    let mut p = String::new();
    p.push_str(&format!("# Bölgesel çalışma · {}\n\n", isletme.ad));
    p.push_str(&format!(
        "- Tarih: {tarih}\n- Bölge sayısı: {}\n\n## Yapılacaklar\n\n",
        bolgeler.len()
    ));
    for (il, ilce) in bolgeler {
        p.push_str(&format!(
            "- [ ] {il} / {ilce} · pin bağlantısı · yerel sayfa · WhatsApp metni\n  - {}\n",
            bolge_link(&isletme.sektor, il, ilce)
        ));
    }
    p.push_str("\n## Öncelik · İzmir – Balıkesir / Savaştepe\n\n1. sıra önce bu segmentte hedeflenir; sayfalar bu sırayla yayına alınır.\n\n");
    for (il, ilce) in oncelikli_bolgeler() {
        p.push_str(&format!(
            "- [ ] {ilce} ({il}) · {}\n",
            bolge_link(&isletme.sektor, &il, &ilce)
        ));
    }
    p.push_str("\n## Not\n\nTek gerçek konum kullanılır; Google'a aynı işletmenin kopya kayıtları eklenmez.\n");
    p
}

/// Google işletme profili ve arama artırma planı (telefonun çalması için).
pub fn profil(isletme: &Isletme, bolgeler: &[(String, String)], tarih: &str) -> String {
    let sektor = sektor_metni(isletme);
    let mut p = String::new();
    p.push_str(&format!("# Arama artırma planı · {}\n\n", isletme.ad));
    p.push_str(&format!(
        "- Tarih: {tarih}\n- Sektör: {sektor}\n- Telefon: {}\n\n",
        isletme.telefon
    ));
    p.push_str("## 1. Google işletme profili (en çok aramayı bu getirir)\n\n");
    p.push_str(
        "- [ ] Kategori: `Çekici hizmeti` (ikincil: `Oto kurtarma`, `Yol yardım hizmeti`)\n",
    );
    p.push_str("- [ ] Çalışma saatleri: 7/24 açık, \"24 saat\" işaretli\n");
    p.push_str(
        "- [ ] Hizmet alanı: aşağıdaki bölgelerin tamamı (adres gizli, hizmet alanı açık)\n",
    );
    p.push_str("- [ ] Telefon: tek numara, WhatsApp ile aynı numara\n");
    p.push_str("- [ ] 10 gerçek fotoğraf: çekici, yükleme, ekip, araç, gece çekimi\n");
    p.push_str(
        "- [ ] Hizmetler: çekici, lastik, kaza kurtarma, otoyol kurtarma (her biri ayrı satır)\n",
    );
    p.push_str("- [ ] Açıklama: ilk 150 karakterde \"O-5 Kemalpaşa–Savaştepe 7/24 oto kurtarma\" geçsin\n");
    p.push_str("- [ ] Soru-cevap: \"Fiyat ne kadar?\", \"Ne kadar sürede gelirsiniz?\", \"Gece geliyor musunuz?\"\n");
    p.push_str("- [ ] Web sitesi: üretilen `index.html` adresi\n\n");
    p.push_str("## 2. Yorum (aramayı en hızlı artıran şey)\n\n");
    p.push_str("Her iş sonrası aynı gün WhatsApp'tan şu mesajı gönder:\n\n");
    p.push_str("> Merhaba, bugün aracınızı kurtardık. Google'da bize 1 dakikalık yorum bırakır mısınız? Bu bağlantı direkt yorum sayfasını açar: <profil yorum bağlantısı>\n\n");
    p.push_str("- [ ] Haftada 3 yorum hedefi\n- [ ] Her yoruma 24 saat içinde cevap (bölge adını yaz: \"Akhisar'da yanınızdaydık\")\n\n");
    p.push_str("## 3. Otoyol ve şehir sayfaları\n\n");
    p.push_str("Üretilen sayfaları sırayla yayına al; her biri tek bölgeye odaklı:\n\n");
    for (il, ilce) in bolgeler {
        p.push_str(&format!("- [ ] {ilce} ({il})\n"));
    }
    p.push_str("\n## 4. Haftalık plan\n\n");
    p.push_str("| Gün | İş |\n|---|---|\n");
    p.push_str("| 1 | Profil kategorisi, saatler, hizmet alanı, telefon |\n");
    p.push_str("| 2 | 10 fotoğraf + açıklama + hizmet listesi |\n");
    p.push_str("| 3 | Ana sayfa ve O-5 sayfasını yayına al |\n");
    p.push_str("| 4 | O-5 kuzey yarısı: Kemalpaşa, Turgutlu, Saruhanlı |\n");
    p.push_str("| 5 | O-5 güney yarısı: Akhisar, Kırkağaç, Soma, Savaştepe |\n");
    p.push_str("| 6 | Son 30 günün müşterilerine yorum mesajı |\n");
    p.push_str("| 7 | Profilde ilk gönderi: \"O-5'te 7/24 kurtarma\" + fotoğraf |\n\n");
    p.push_str("## 5. Ölçüm\n\n");
    p.push_str(
        "- [ ] Profil → Performans: arama sayısı ve telefon tıklaması haftalık not edilsin\n",
    );
    p.push_str("- [ ] Hedef: 30 günde telefon aramasında 2 kat artış\n");
    p
}

/// 1. sıraya çıkma yol haritası: İzmir ↔ Balıkesir / Savaştepe segmenti.
pub fn siralama_plani(isletme: &Isletme, bolgeler: &[(String, String)], tarih: &str) -> String {
    let sektor = sektor_metni(isletme);
    let tel = isletme.telefon.trim();
    let adres = if isletme.adres.trim().is_empty() {
        "Manisa"
    } else {
        isletme.adres.trim()
    };
    let oncelik = oncelikli_bolgeler();
    let mut p = String::new();
    p.push_str(&format!("# 1. sıra planı · {}\n\n", isletme.ad));
    p.push_str(&format!(
        "- Tarih: {tarih}\n- Hedef hat: İzmir ↔ Balıkesir / Savaştepe (O-5)\n- Sektör: {sektor}\n- Telefon: {tel}\n- Toplam bölge: {} · öncelikli segment: {}\n\n",
        bolgeler.len(),
        oncelik.len()
    ));
    p.push_str("## 0. Kural: 1. sıra tek ayarla gelmez\n\n");
    p.push_str(
        "Google Haritalar sıralaması üç şeyin bileşimi: **alaka** (kategori + hizmetler + sayfa metni), **mesafe** (hizmet alanı + gerçek konum), **itibar** (yorum sayısı, puan, fotoğraf, cevap hızı). Aşağıdaki plan üçünü de aynı sekiz kelimeye kilitler:\n\n",
    );
    p.push_str("`çekici` · `oto kurtarma` · `yol yardım 7/24` · `en yakın çekici` · `benzin destek` · `mazot takviye` · `akü takviye` · `akücü`\n\n");
    p.push_str("Gerçekçi süre: 30-90 gün. Haftada 3 yorum, haftada 1 gönderi ve yayına alınmış sayfalarla segmentte ilk 3'e girmek olağan; 1. sıra yorum hızına ve rakip yoğunluğuna bağlıdır.\n\n");

    p.push_str("## 1. Google işletme profili — alan alan (1. gün)\n\n");
    p.push_str("- [ ] **Birincil kategori:** Çekici hizmeti\n");
    p.push_str("- [ ] **İkincil kategoriler:** Oto kurtarma · Yol yardım hizmeti · Akü hizmeti · Lastikçi\n");
    p.push_str("- [ ] **İşletme adı:** tabeladaki gerçek ad. Adın içine \"çekici\", \"en yakın\", \"7/24\" eklenmez; Google bunu askıya alma sebebi sayar.\n");
    p.push_str(&format!(
        "- [ ] **Telefon:** tek numara ({tel}) — WhatsApp ile aynı numara.\n"
    ));
    p.push_str("- [ ] **Web sitesi:** üretilen `index.html` alan adı.\n");
    p.push_str("- [ ] **Adres:** gerçek adres; \"Müşterilerime işletmemde hizmet veriyorum\" kapalı, **hizmet alanı** açık.\n");
    p.push_str("- [ ] **Çalışma saatleri:** her gün 24 saat açık.\n");
    p.push_str(&format!(
        "- [ ] **Hizmet alanı:** aşağıdaki {} bölgenin tamamı (bölüm 5'te öncelik sırası var).\n",
        bolgeler.len()
    ));
    p.push_str("- [ ] **Hizmetler:** bölüm 2'deki 8 satırı birebir ekle; her biri ayrı hizmet, fiyat \"telefonla\".\n");
    p.push_str("- [ ] **Öznitelikler:** \"Yerinde hizmet\" · \"7/24 hizmet\" · \"Kredi kartı kabul edilir\" (gerçekse).\n");
    p.push_str("- [ ] **Fotoğraf (10 gerçek kare):** çekici yan görünüm, araca yükleme, gece çalışması, akü takviyesi, yakıt bidonu, ekip, araç içi, tabela/kaplama, otoyolda kurtarma, müşteriye teslim.\n");
    p.push_str("- [ ] **Açıklama (kopyala-yapıştır):**\n\n");
    p.push_str(&format!(
        "> {adres} merkezli ekibimiz O-5 otoyolunun Kemalpaşa–Savaştepe kesiminde (7 ilçe) 7/24 çekici ve oto kurtarma hizmeti veriyor. Aracınız yolda kaldıysa en yakın çekici olarak yanınızdayız: yol yardım 7/24, benzin destek, mazot takviye, akü takviye ve akücü hizmeti. Konumunuzu WhatsApp'tan gönderin, en yakın ekibi yönlendirelim. Telefon: {tel}\n\n"
    ));

    p.push_str("## 2. Hizmet satırları (profilde tek tek açılacak)\n\n");
    for hizmet in crate::otoyol_pin::HIZMETLER {
        p.push_str(&format!(
            "- [ ] {hizmet} — fiyat: telefonla, baştan söylenir\n"
        ));
    }
    p.push_str("\nHer hizmete aynı gün bir gerçek fotoğraf ekle; fotoğrafsız hizmet satırı sıralamada zayıf kalır.\n\n");

    p.push_str("## 3. Yorum motoru (sıralamayı en hızlı artıran şey)\n\n");
    p.push_str("- Hedef: **haftada 3 yeni yorum**, ilk 30 günde 12 yorum.\n");
    p.push_str("- Her iş sonrası aynı gün WhatsApp'tan gönder:\n\n");
    p.push_str("> Merhaba, bugün aracınıza yardım ettik. Google'da bize 1 dakikalık yorum bırakır mısınız? Şu bağlantı direkt yorum sayfasını açar: <profil yorum bağlantısı> — yorumda hangi ilçede yardım aldığınızı yazarsanız çok sevinirim.\n\n");
    p.push_str("- Her yoruma 24 saat içinde, ilçe adıyla cevap ver:\n\n");
    p.push_str("> Teşekkürler, {ilçe} bölgesinde 7/24 çekici ve akü takviye için her zaman yanınızdayız.\n\n");
    p.push_str("- Yorumda geçen hizmet adı (çekici, akü takviye, benzin destek) arama eşleşmesini güçlendirir; müşteriye bunu hatırlat.\n");
    p.push_str("- Sahte yorum, yorum satın alma ve akraba yorumları kalıcı ceza getirir; yapılmayacak.\n\n");

    p.push_str("## 4. Site ve teknik SEO (üretilen dosyalar)\n\n");
    p.push_str("- [ ] Bölge sayfalarını ve `index.html` dosyasını yayına al.\n");
    p.push_str(&format!(
        "- [ ] `sitemap.xml` ve `robots.txt` içindeki `{ALAN_ADI}` adresini kendi alan adınla değiştir.\n"
    ));
    p.push_str("- [ ] Google Search Console'a alan adını ekle, `sitemap.xml` adresini gönder.\n");
    p.push_str("- Her sayfa: `Oto kurtarma <ilçe> · <işletme>` başlığı, 8 hizmet, 5 SSS, telefon ve WhatsApp düğmesi, sayfa sonunda `index.html` linki.\n");
    p.push_str("- Sayfalara schema.org `AutoRepair` + 8 hizmetlik `OfferCatalog` gömülü; ayrıca bir şey eklemen gerekmiyor.\n");
    p.push_str("- Profildeki \"Web sitesi\" alanına `index.html` adresini yaz; profildeki site ile sayfadaki telefon/adres birebir aynı olsun (NAP tutarlılığı).\n\n");

    p.push_str("## 5. Öncelik sırası · İzmir → Savaştepe\n\n");
    p.push_str("İlk yayına alınacak ve yorum toplanacak bölgeler:\n\n");
    for (il, ilce) in &oncelik {
        p.push_str(&format!("- {ilce} ({il})\n"));
    }
    p.push_str("\nKapsam: yalnızca O-5 üzerindeki 7 ilçe. Diğer bölgeler bilinçli olarak kapsam dışı.\n\n");

    p.push_str("## 6. 30 günlük takvim\n\n");
    p.push_str("| Gün | İş |\n|---|---|\n");
    p.push_str("| 1 | Birincil/ikincil kategori, saatler, hizmet alanı, telefon |\n");
    p.push_str("| 2 | 8 hizmet satırı + profil açıklaması |\n");
    p.push_str("| 3 | 10 fotoğraf, öznitelikler |\n");
    p.push_str("| 4 | `index.html` + O-5 sayfaları yayında |\n");
    p.push_str("| 5 | Search Console + sitemap |\n");
    p.push_str("| 6-11 | İzmir sayfaları (6 bölge) + ilk yorumlar |\n");
    p.push_str("| 12-17 | Manisa sayfaları (6 bölge) + yorumlar |\n");
    p.push_str("| 18 | Savaştepe sayfası + iki otoyol sayfası |\n");
    p.push_str("| 19-29 | Haftada 3 yorum, haftada 1 gönderi, geçmiş müşterilere yorum mesajı |\n");
    p.push_str("| 30 | Ölçüm: arama sırası, telefon tıklaması, yorum sayısı |\n\n");

    p.push_str("## 7. Haftalık ölçüm\n\n");
    p.push_str("Gizli sekmede Google Maps'i aç, konumu sırayla **Saruhanlı → Akhisar → Savaştepe** yap ve şu aramalarda kaçıncı sırada olduğunu not et:\n\n");
    p.push_str("`çekici` · `oto kurtarma` · `yol yardım 7/24` · `en yakın çekici` · `benzin destek` · `mazot takviye` · `akü takviye` · `akücü`\n\n");
    p.push_str("| Hafta | Sıra (çekici) | Telefon tıklaması | Yorum |\n|---|---|---|---|\n");
    p.push_str("| 1 |  |  |  |\n| 2 |  |  |  |\n| 3 |  |  |  |\n| 4 |  |  |  |\n\n");
    p.push_str("Profil → Performans ekranındaki arama ve telefon tıklaması sayısını da haftalık yaz. Hedef: 30 günde telefon aramasında 2 kat artış.\n\n");

    p.push_str("## 8. Yapılmayacaklar\n\n");
    p.push_str(
        "- Aynı işletmeye ikinci kayıt veya her ilçeye kopya profil (askıya alınma sebebi).\n",
    );
    p.push_str("- İşletme adına anahtar kelime eklemek (\"... Çekici 7/24\").\n");
    p.push_str("- Sahte/yorum satın alma, birbirine benzeyen 30 sayfa metni.\n");
    p.push_str(
        "- Her ilçe sayfasına aynı cümleyi kopyalamak; ilçe adı, yol ve süre bilgisi değişmeli.\n",
    );
    p
}

/// `sitemap.xml`: tüm bölge sayfaları + ana sayfa.
pub fn sitemap(bolgeler: &[(String, String)], tarih: &str) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    s.push_str(&format!(
        "  <url><loc>{ALAN_ADI}/</loc><lastmod>{tarih}</lastmod><changefreq>weekly</changefreq><priority>1.0</priority></url>\n"
    ));
    for (il, ilce) in bolgeler {
        s.push_str(&format!(
            "  <url><loc>{ALAN_ADI}/{}</loc><lastmod>{tarih}</lastmod><changefreq>weekly</changefreq><priority>0.8</priority></url>\n",
            sayfa_url(il, ilce)
        ));
    }
    s.push_str("</urlset>\n");
    s
}

/// `robots.txt`: tüm sayfalar açık, sitemap bildirilir.
pub fn robots() -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: {ALAN_ADI}/sitemap.xml\n")
}

/// Profil gönderileri, yorum mesajları ve "Google'a işlendi mi" kontrol listesi.
pub fn gonderiler(isletme: &Isletme) -> String {
    let tel = isletme.telefon.trim();
    let adres = if isletme.adres.trim().is_empty() {
        "Manisa"
    } else {
        isletme.adres.trim()
    };
    let wa = wa_link(tel);
    let mut p = String::new();
    p.push_str(&format!(
        "# Profil gönderileri ve yorum metinleri · {}\n\n",
        isletme.ad
    ));
    p.push_str(&format!(
        "- Telefon: {tel}\n- WhatsApp: {wa}\n- Konum: {adres}\n\n"
    ));
    p.push_str("Her gönderiyi **Google işletme profili → Gönderiler → Ekle** ile yayınla. Haftada 1 gönderi yeter; aynı metni tekrar etme, her gönderiye gerçek bir fotoğraf ekle.\n\n");

    p.push_str("## 1. hafta · Tanıtım\n\n");
    p.push_str(&format!(
        "> O-5 ve İzmir–Balıkesir hattında 7/24 çekici. Aracınız Saruhanlı, Akhisar ya da Savaştepe çevresinde kaldıysa en yakın ekibi yönlendiriyoruz. Çekici, oto kurtarma, yol yardım 7/24, benzin destek, mazot takviye, akü takviye ve akücü hizmeti tek numarada. Fiyat telefonda baştan söylenir. Ara: {tel}\n\n"
    ));

    p.push_str("## 2. hafta · Akü ve yakıt\n\n");
    p.push_str(&format!(
        "> Akü bitti, marş basmıyor ya da yakıt kalmadıysa aracı çekmeye gerek yok. {adres} ekibimiz akü takviye, akücü, benzin destek ve mazot takviye hizmetini yerinde yapıyor. Çalışmazsa çekici hazır. 7/24: {tel}\n\n"
    ));

    p.push_str("## 3. hafta · Gece ve otoyol\n\n");
    p.push_str(&format!(
        "> Gece yarısı O-5'te kalan araçlar için de çalışıyoruz. Konumunuzu WhatsApp'tan gönderin, en yakın ekibi yönlendirelim; varış süresini ve fiyatı yola çıkmadan söylüyoruz. 7/24 çekici ve oto kurtarma: {tel}\n\n"
    ));

    p.push_str("## 4. hafta · Güven ve yorum\n\n");
    p.push_str(&format!(
        "> Aracınızı hasarsız yüklüyor, istediğiniz servise bırakıyoruz. İşimizi yapan ekip aynı gün yola çıkıyor, sonradan ek ücret çıkmıyor. Hizmet aldıysanız Google'da 1 dakikalık yorum bırakmanız bölgedeki diğer sürücülere yardımcı olur. Teşekkürler. {tel}\n\n"
    ));

    p.push_str("## Yorum isteme mesajı (iş sonrası WhatsApp)\n\n");
    p.push_str("> Merhaba, bugün aracınıza yardım ettik. Google'da bize 1 dakikalık yorum bırakır mısınız? Şu bağlantı direkt yorum sayfasını açar: <profil yorum bağlantısı> — yorumda hangi ilçede yardım aldığınızı ve hangi hizmeti aldığınızı (çekici, akü takviye, benzin destek) yazarsanız çok sevinirim.\n\n");

    p.push_str("## Yoruma cevap şablonları (24 saat içinde)\n\n");
    p.push_str("- **Fiyat:** \"Teşekkürler, {ilçe} bölgesinde fiyatı telefonda baştan söylüyoruz; sonradan ek ücret çıkmıyor.\"\n");
    p.push_str("- **Hız:** \"Teşekkürler, {ilçe} ve O-5 hattında ortalama 20-30 dakikada yanınızdayız.\"\n");
    p.push_str("- **Gece:** \"Teşekkürler, gece de 7/24 çalışıyoruz; bir dahaki sefere yine arayabilirsiniz.\"\n");
    p.push_str("- **Akü:** \"Teşekkürler, akü takviye ve akücü hizmetimiz {ilçe} bölgesinde 7/24 devam ediyor.\"\n");
    p.push_str(
        "- **Yakıt:** \"Teşekkürler, benzin destek ve mazot takviye için tekrar bekleriz.\"\n\n",
    );

    p.push_str("## Yerel dizinler (NAP her yerde aynı: ad, adres, telefon)\n\n");
    p.push_str("- [ ] Google işletme profili (doğrulanmış)\n");
    p.push_str("- [ ] Bing Places\n");
    p.push_str("- [ ] Yandex Haritalar\n");
    p.push_str("- [ ] Apple Business Connect (hesabında Türkiye görünüyorsa)\n");
    p.push_str("- [ ] Facebook sayfası\n");
    p.push_str("- [ ] Instagram profili (ad/telefon alanları dolu)\n");
    p.push_str("- [ ] WhatsApp Business profili (ad, adres, çalışma saatleri)\n");
    p.push_str("- [ ] Sektör rehberleri / oto kurtarma listeleri\n\n");

    p.push_str("## Google'a işlendi mi? Kontrol listesi\n\n");
    p.push_str("Uygulama Google'a veri göndermez; bu adımlar senin hesabında yapılır ve sonucu aşağıdaki kontrollerle görürsün.\n\n");
    p.push_str("- [ ] Profil doğrulaması tamamlandı (\"Doğrulandı\" ibaresi görünüyor)\n");
    p.push_str("- [ ] Maps'te işletme adı arandığında profil çıkıyor\n");
    p.push_str("- [ ] Search Console: alan adı doğrulandı, sitemap durumu **Başarılı**\n");
    p.push_str("- [ ] URL Denetimi: bölge sayfası \"Dizine eklendi\" diyor\n");
    p.push_str("- [ ] Rich Results Test: `AutoRepair` ve `FAQPage` hatasız\n");
    p.push_str("- [ ] Profil → Performans: arama ve telefon tıklaması verisi geliyor\n");
    p.push_str("- [ ] Haftalık ölçüm tablosu (`siralama-plani.md`) dolduruluyor\n\n");
    p.push_str("Not: Google sıralama için ücret veya \"1. sıra talebi\" kabul etmiyor; sıralamayı alaka, mesafe ve önem belirliyor.\n");
    p
}

/// İşletme klasörünün yolu.
pub fn klasor(isletme: &Isletme) -> PathBuf {
    Path::new(KLASOR).join(domain::kok(&isletme.ad))
}

/// Bölge sayfasının dosya adı.
pub fn dosya_adi(il: &str, ilce: &str) -> String {
    format!("{}-{}.html", domain::kok(il), domain::kok(ilce))
}

/// Sayfanın yayın adresi: uzantısız (statik sunucu `.html` adresini 307 ile kısaltıyor).
pub fn sayfa_url(il: &str, ilce: &str) -> String {
    dosya_adi(il, ilce).trim_end_matches(".html").to_string()
}

/// Bölge sayfasının yolu.
pub fn sayfa_yolu(isletme: &Isletme, il: &str, ilce: &str) -> PathBuf {
    klasor(isletme).join(dosya_adi(il, ilce))
}

/// Sayfaları, ana sayfayı, planı ve profili yazar.
pub fn yaz(
    isletme: &Isletme,
    bolgeler: &[(String, String)],
    tarih: &str,
) -> Result<Vec<PathBuf>, String> {
    if bolgeler.is_empty() {
        return Err("bölge seçilmedi".to_string());
    }
    let kok = klasor(isletme);
    fs::create_dir_all(&kok).map_err(|hata| format!("klasör oluşturulamadı: {hata}"))?;
    // Önceki üretimin sayfalarını temizle (klasör uygulamaya ait).
    if let Ok(liste) = fs::read_dir(&kok) {
        for girdi in liste.flatten() {
            let yol = girdi.path();
            if yol.extension().and_then(|uzanti| uzanti.to_str()) == Some("html") {
                let _ = fs::remove_file(yol);
            }
        }
    }
    let mut yazilan = Vec::new();
    for (il, ilce) in bolgeler {
        let yol = kok.join(dosya_adi(il, ilce));
        fs::write(&yol, sayfa(isletme, il, ilce, tarih, bolgeler))
            .map_err(|hata| format!("{} yazılamadı: {hata}", yol.display()))?;
        yazilan.push(yol);
    }
    let ana = kok.join("index.html");
    fs::write(&ana, ana_sayfa(isletme, bolgeler, tarih))
        .map_err(|hata| format!("ana sayfa yazılamadı: {hata}"))?;
    yazilan.push(ana);
    let plan_yolu = kok.join("plan.md");
    fs::write(&plan_yolu, plan(isletme, bolgeler, tarih))
        .map_err(|hata| format!("plan yazılamadı: {hata}"))?;
    yazilan.push(plan_yolu);
    let profil_yolu = kok.join("profil.md");
    fs::write(&profil_yolu, profil(isletme, bolgeler, tarih))
        .map_err(|hata| format!("profil planı yazılamadı: {hata}"))?;
    yazilan.push(profil_yolu);
    let siralama_yolu = kok.join("siralama-plani.md");
    fs::write(&siralama_yolu, siralama_plani(isletme, bolgeler, tarih))
        .map_err(|hata| format!("sıralama planı yazılamadı: {hata}"))?;
    yazilan.push(siralama_yolu);
    let sitemap_yolu = kok.join("sitemap.xml");
    fs::write(&sitemap_yolu, sitemap(bolgeler, tarih))
        .map_err(|hata| format!("sitemap yazılamadı: {hata}"))?;
    yazilan.push(sitemap_yolu);
    let robots_yolu = kok.join("robots.txt");
    fs::write(&robots_yolu, robots()).map_err(|hata| format!("robots.txt yazılamadı: {hata}"))?;
    yazilan.push(robots_yolu);
    let gonderi_yolu = kok.join("gonderiler.md");
    fs::write(&gonderi_yolu, gonderiler(isletme))
        .map_err(|hata| format!("gönderi metinleri yazılamadı: {hata}"))?;
    yazilan.push(gonderi_yolu);
    Ok(yazilan)
}

impl LeadApp {
    /// Bölgesel çalışmada kullanılacak işletme: pin ekranındaki seçim, yoksa
    /// ilk pin, yoksa defterdeki ilk kayıt.
    fn bolgesel_isletme(&self) -> Option<Isletme> {
        if let Some(anahtar) = self.pin_secili.as_deref() {
            if let Some(pin) = self.pinler.kayitlar.get(anahtar) {
                return Some(Isletme {
                    sektor: sektor_tahmin(&pin.ad, &pin.adres),
                    ad: pin.ad.clone(),
                    telefon: pin.telefon.clone(),
                    adres: pin.adres.clone(),
                    posta: pin.posta.clone(),
                });
            }
        }
        if let Some((_, pin)) = self.pinler.kayitlar.iter().next() {
            return Some(Isletme {
                sektor: sektor_tahmin(&pin.ad, &pin.adres),
                ad: pin.ad.clone(),
                telefon: pin.telefon.clone(),
                adres: pin.adres.clone(),
                posta: pin.posta.clone(),
            });
        }
        self.defter.kayitlar().first().map(|kayit| Isletme {
            sektor: if kayit.kategori.trim().is_empty() {
                sektor_tahmin(&kayit.ad, &kayit.adres)
            } else {
                kayit.kategori.clone()
            },
            ad: kayit.ad.clone(),
            telefon: kayit.telefon.clone(),
            adres: kayit.adres.clone(),
            // Müşteri kaydında posta kodu alanı yok; şemaya yazılmaz.
            posta: String::new(),
        })
    }

    /// Bölgesel pinleme ekranı.
    pub(crate) fn bolgesel_ekrani(&mut self, ctx: &egui::Context) {
        let il = self.cities[self.selected_city_idx].0;
        let ilceler: Vec<(&'static str, &'static str)> = self.cities[self.selected_city_idx]
            .1
            .iter()
            .map(|ilce| (il, *ilce))
            .collect();
        let koridor = bolgesel::koridor_bolgeleri();
        let kaynak: Vec<(String, String)> = if self.bolgesel_koridor {
            koridor.clone()
        } else {
            ilceler
                .iter()
                .map(|(il, ilce)| (il.to_string(), ilce.to_string()))
                .collect()
        };
        let isletme = self.bolgesel_isletme();
        let bugun = bugun_anahtari();
        let mut secim_degis: Option<String> = None;
        let mut tumu = false;
        let mut uret = false;
        let mut kopyala = false;
        let mut yeni_isletme = false;
        let mut siralama = false;

        let secili: Vec<(String, String)> = if self.bolgesel_secim.is_empty() {
            kaynak.clone()
        } else {
            kaynak
                .iter()
                .filter(|(il, ilce)| self.bolgesel_secim.contains(&anahtar(il, ilce)))
                .cloned()
                .collect()
        };

        egui::SidePanel::left("bolgesel_bolgeler")
            .resizable(false)
            .exact_width(298.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "BÖLGELER");
                ui.label(
                    egui::RichText::new(format!("{} bölge seçili · {}", secili.len(), {
                        if self.bolgesel_koridor {
                            "O-5 koridoru"
                        } else {
                            il
                        }
                    }))
                    .size(10.0)
                    .color(DIM),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Tümü").size(10.0).color(METIN))
                                .fill(KART)
                                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                .rounding(egui::Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        tumu = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Hiçbiri").size(10.0).color(DIM))
                                .fill(KART)
                                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                .rounding(egui::Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        // Boş küme "hepsi" demek olduğu için tek bir bölge bırakılır.
                        self.bolgesel_secim.clear();
                        if let Some((il, ilce)) = kaynak.first() {
                            self.bolgesel_secim.insert(anahtar(il, ilce));
                        }
                    }
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .id_source("bolgesel_bolge_listesi")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (il, ilce) in &kaynak {
                            let secili_mi = self.bolgesel_secim.is_empty()
                                || self.bolgesel_secim.contains(&anahtar(il, ilce));
                            let etiket = if self.bolgesel_koridor && il != "Otoyol" {
                                format!("{ilce} · {il}")
                            } else {
                                ilce.to_string()
                            };
                            let buton = egui::Button::new(
                                egui::RichText::new(etiket).size(10.5).color(if secili_mi {
                                    KOYU_YAZI
                                } else {
                                    METIN
                                }),
                            )
                            .fill(if secili_mi { VURGU } else { KART })
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if secili_mi { VURGU } else { KENAR },
                            ))
                            .rounding(egui::Rounding::same(99.0));
                            if ui.add(buton).clicked() {
                                secim_degis = Some(anahtar(il, ilce));
                            }
                            ui.add_space(3.0);
                        }
                    });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "BÖLGESEL PİNLEME + SEO");
                ui.label(
                    egui::RichText::new(
                        "Her bölge için pin bağlantısı, yerel sayfa ve WhatsApp metni; işletmeye ana sayfa ve arama artırma planı üretilir. Yeni işletme kaydı oluşturulmaz.",
                    )
                    .size(10.0)
                    .color(DIM),
                );
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let Some(isletme) = isletme.as_ref() else {
                            ui.label(
                                egui::RichText::new(
                                    "İşletme yok. Harita Pini ekranından bir işletme seçin.",
                                )
                                .size(11.0)
                                .color(TURUNCU),
                            );
                            return;
                        };
                        ui.label(
                            egui::RichText::new(&isletme.ad)
                                .size(17.0)
                                .color(METIN)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{}{}",
                                if isletme.adres.trim().is_empty() {
                                    String::new()
                                } else {
                                    format!("{} · ", isletme.adres.trim())
                                },
                                if isletme.telefon.trim().is_empty() {
                                    "telefon yok".to_string()
                                } else {
                                    isletme.telefon.clone()
                                }
                            ))
                            .size(10.5)
                            .color(SOLUK),
                        );
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            pill(ui, &format!("{} bölge", secili.len()), YESIL);
                            pill(ui, &sektor_metni(isletme), DIM);
                            pill(ui, KLASOR, DIM);
                        });
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("O-5 koridoru").size(10.0).color(
                                            if self.bolgesel_koridor { KOYU_YAZI } else { METIN },
                                        ),
                                    )
                                    .fill(if self.bolgesel_koridor { VURGU } else { PANEL })
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(99.0)),
                                )
                                .clicked()
                            {
                                self.bolgesel_koridor = true;
                                self.bolgesel_secim.clear();
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(format!("{il} ilçeleri"))
                                            .size(10.0)
                                            .color(if self.bolgesel_koridor {
                                                METIN
                                            } else {
                                                KOYU_YAZI
                                            }),
                                    )
                                    .fill(if self.bolgesel_koridor { PANEL } else { VURGU })
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(99.0)),
                                )
                                .clicked()
                            {
                                self.bolgesel_koridor = false;
                                self.bolgesel_secim.clear();
                            }
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [250.0, 32.0],
                                    egui::Button::new(
                                        egui::RichText::new(format!(
                                            "Çalışmayı üret · {} bölge",
                                            secili.len()
                                        ))
                                        .size(10.5)
                                        .color(KOYU_YAZI),
                                    )
                                    .fill(VURGU)
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                uret = true;
                            }
                            if ui
                                .add_sized(
                                    [190.0, 32.0],
                                    egui::Button::new(
                                        egui::RichText::new("Mesajları kopyala")
                                            .size(10.5)
                                            .color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                kopyala = true;
                            }
                            if ui
                                .add_sized(
                                    [120.0, 32.0],
                                    egui::Button::new(
                                        egui::RichText::new("Klasörü aç").size(10.5).color(DIM),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                yeni_isletme = true;
                            }
                            if ui
                                .add_sized(
                                    [140.0, 32.0],
                                    egui::Button::new(
                                        egui::RichText::new("1. sıra planı").size(10.5).color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                siralama = true;
                            }
                        });
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("BÖLGE · DURUM").size(9.0).color(DIM));
                        egui::ScrollArea::vertical()
                            .id_source("bolgesel_durum")
                            .max_height(ui.available_height() - 12.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (il, ilce) in &secili {
                                    let var = sayfa_yolu(isletme, il, ilce).exists();
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(ilce).size(10.5).color(METIN),
                                        );
                                        pill(
                                            ui,
                                            if var { "sayfa hazır" } else { "bekliyor" },
                                            if var { YESIL } else { DIM },
                                        );
                                        ui.label(
                                            egui::RichText::new(bolge_link(
                                                &isletme.sektor,
                                                il,
                                                ilce,
                                            ))
                                            .size(8.5)
                                            .color(DIM),
                                        );
                                    });
                                }
                            });
                    });
            });

        if tumu {
            self.bolgesel_secim.clear();
        }
        if let Some(secim) = secim_degis {
            if self.bolgesel_secim.is_empty() {
                for (il, ilce) in &kaynak {
                    let k = anahtar(il, ilce);
                    if k != secim {
                        self.bolgesel_secim.insert(k);
                    }
                }
            } else if !self.bolgesel_secim.remove(&secim) {
                self.bolgesel_secim.insert(secim);
            }
            if self.bolgesel_secim.len() == kaynak.len() {
                self.bolgesel_secim.clear();
            }
        }
        if yeni_isletme {
            if let Some(isletme) = isletme.as_ref() {
                let yol = klasor(isletme);
                let _ = std::process::Command::new("explorer")
                    .arg(yol.canonicalize().unwrap_or(yol))
                    .spawn();
            }
        }
        let Some(isletme) = isletme else {
            return;
        };
        if uret {
            match yaz(&isletme, &secili, &bugun) {
                Ok(dosyalar) => {
                    self.logs.push(format!(
                        "[✓] {}: {} bölge sayfası + ana sayfa + plan + profil + 1. sıra planı + gönderiler + sitemap + robots üretildi ({}) → {}/",
                        isletme.ad,
                        secili.len(),
                        dosyalar.len(),
                        KLASOR
                    ));
                }
                Err(hata) => self.logs.push(format!("[!] {hata}")),
            }
        }
        if siralama {
            let yol = klasor(&isletme).join("siralama-plani.md");
            if yol.exists() {
                let _ = std::process::Command::new("explorer")
                    .arg(yol.canonicalize().unwrap_or(yol))
                    .spawn();
            } else {
                self.logs.push(
                    "[!] Önce \"Çalışmayı üret\" ile 1. sıra planı dosyasını oluşturun."
                        .to_string(),
                );
            }
        }
        if kopyala {
            let metin = secili
                .iter()
                .map(|(il, ilce)| mesaj(&isletme, il, ilce))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n");
            ctx.output_mut(|cikti| cikti.copied_text = metin);
            self.logs.push(format!(
                "[✓] {}: {} bölge mesajı panoya kopyalandı.",
                isletme.ad,
                secili.len()
            ));
        }
    }
}

/// İşletme adı/adresten sektör tahmini; boş sektörle "Hizmet" yazılmasın.
pub fn sektor_tahmin(ad: &str, adres: &str) -> String {
    let metin = format!("{ad} {adres}").to_lowercase();
    let tablo: [(&[&str], &str); 7] = [
        (
            &["oto kurtarma", "otokurtarma", "çekici", "yol yardım"],
            "Oto kurtarma",
        ),
        (&["lastik"], "Lastik"),
        (&["kuaför", "kuafor", "berber", "güzellik"], "Kuaför"),
        (
            &["restoran", "lokanta", "kebap", "cafe", "kafe"],
            "Restoran",
        ),
        (&["tamir", "oto servis", "oto"], "Oto servis"),
        (&["nakliyat", "taşıma"], "Nakliyat"),
        (&["temizlik"], "Temizlik"),
    ];
    for (anahtarlar, sektor) in tablo {
        if anahtarlar.iter().any(|anahtar| metin.contains(anahtar)) {
            return sektor.to_string();
        }
    }
    "Hizmet".to_string()
}

/// Komut satırı: `--bolgesel-uret` bölge sayfalarını GUI açmadan yeniden üretir.
///
/// İşletme bilgisi `pinler.json` içindeki `google:` kaydından okunur; böylece
/// sayfalar tek şablondan, tek komutla ve aynı NAP bilgisiyle üretilir.
pub fn komut_satiri() -> Option<i32> {
    if !std::env::args().any(|arg| arg == "--bolgesel-uret") {
        return None;
    }
    let ham = match fs::read_to_string("pinler.json") {
        Ok(metin) => metin,
        Err(hata) => {
            eprintln!("pinler.json okunamadı: {hata}");
            return Some(1);
        }
    };
    let defter: serde_json::Value = match serde_json::from_str(&ham) {
        Ok(deger) => deger,
        Err(hata) => {
            eprintln!("pinler.json bozuk: {hata}");
            return Some(1);
        }
    };
    let kayit = defter["kayitlar"].as_object().and_then(|kayitlar| {
        kayitlar
            .iter()
            .find(|(anahtar, _)| anahtar.starts_with("google:"))
            .map(|(_, kayit)| kayit)
    });
    let Some(kayit) = kayit else {
        eprintln!("pinler.json içinde google kaydı yok");
        return Some(1);
    };
    let ad = kayit["ad"].as_str().unwrap_or("Saruhanlı Oto Kurtarma");
    let adres = kayit["adres"].as_str().unwrap_or("Saruhanlı / Manisa");
    let isletme = Isletme {
        ad: ad.to_string(),
        telefon: kayit["telefon"].as_str().unwrap_or("").to_string(),
        adres: adres.to_string(),
        sektor: sektor_tahmin(ad, adres),
        posta: kayit["posta"].as_str().unwrap_or("").to_string(),
    };
    let tarih = chrono::Local::now().format("%Y-%m-%d").to_string();
    match yaz(&isletme, &koridor_bolgeleri(), &tarih) {
        Ok(dosyalar) => {
            println!(
                "{} dosya üretildi: {}",
                dosyalar.len(),
                klasor(&isletme).display()
            );
            Some(0)
        }
        Err(hata) => {
            eprintln!("üretim hatası: {hata}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn isletme() -> Isletme {
        Isletme {
            ad: "Saruhanlı Oto Kurtarma".to_string(),
            telefon: "0507 129 94 39".to_string(),
            adres: "Saruhanlı / Manisa".to_string(),
            sektor: "Oto kurtarma".to_string(),
            posta: "45800".to_string(),
        }
    }

    #[test]
    fn adres_kod_turkce_harfleri_kodlar() {
        let kod = adres_kod("Şehzadeler, Manisa");
        assert!(kod.contains("%C5%9E"));
        assert!(kod.contains("Manisa"));
        assert!(!kod.contains(' '));
    }

    #[test]
    fn bolge_linki_sektor_ve_ilce_icerir() {
        let link = bolge_link("Oto kurtarma", "Şehzadeler", "Manisa");
        assert!(link.starts_with("https://www.google.com/maps/search/?api=1&query="));
        assert!(link.contains("Oto%20kurtarma"));
        assert!(link.contains("%C5%9Eehzadeler"));
        assert!(link.contains("Manisa"));
    }

    #[test]
    fn otoyol_bolgesinde_il_yerine_otoyol_gecer() {
        let link = bolge_link("Oto kurtarma", "Otoyol", "O-5 Otoyolu");
        assert!(link.contains("O-5%20Otoyolu"));
        assert!(!link.contains("Otoyol%20O-5"));
    }

    #[test]
    fn whatsapp_linki_ulke_kodu_ekler() {
        assert_eq!(wa_link("0507 129 94 39"), "https://wa.me/905071299439");
        assert_eq!(wa_link("905071299439"), "https://wa.me/905071299439");
        assert_eq!(wa_link("5071299439"), "https://wa.me/905071299439");
    }

    /// Sayfaları elle yeniden üretir (GUI/uygulama kapalıyken):
    /// `cargo test --bin ollama_proje bolge_sayfalarini_uret -- --ignored --nocapture`
    #[test]
    #[ignore = "elle: bölge sayfalarını yeniden üretir"]
    fn bolge_sayfalarini_uret() {
        let dosyalar = yaz(&isletme(), &koridor_bolgeleri(), "2026-11-10").expect("üretim");
        println!("{} dosya üretildi", dosyalar.len());
        // Kapsam: 7 bölge sayfası + dizin + sitemap/robots/plan dosyaları.
        assert!(dosyalar.len() >= 10);
    }

    #[test]
    fn sayfa_ilce_ve_yapisal_veri_icerir() {
        let html = sayfa(
            &isletme(),
            "Manisa",
            "Şehzadeler",
            "2026-11-10",
            &koridor_bolgeleri(),
        );
        assert!(html.contains("<title>Çekici Şehzadeler · 7/24 | Saruhanlı Oto Kurtarma</title>"));
        assert!(html.contains("AutoRepair"));
        assert!(html.contains("\"Şehzadeler\""));
        assert!(html.contains("FAQPage"));
        assert!(html.contains(&format!("<link rel=\"canonical\" href=\"{ALAN_ADI}/")));
        assert!(html.contains("tel:05071299439"));
        assert!(html.contains("wa.me/905071299439"));
        assert!(html.contains("Tüm bölgeler"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn ana_sayfa_tum_bolgeleri_listeler() {
        let bolgeler = vec![
            ("İzmir".to_string(), "Bornova".to_string()),
            ("İstanbul".to_string(), "Tuzla".to_string()),
            ("Otoyol".to_string(), "O-5 Otoyolu".to_string()),
        ];
        let html = ana_sayfa(&isletme(), &bolgeler, "2026-11-10");
        assert!(html.contains("3 nokta"));
        assert!(html.contains("izmir-bornova"));
        assert!(html.contains("istanbul-tuzla"));
        assert!(!html.contains(".html\""));
        assert!(html.contains("Hemen ara"));
        assert!(html.contains("areaServed"));
        assert!(html.contains(&format!("{ALAN_ADI}/")));
        assert!(html.contains("\"Bornova, İzmir\""));
    }

    #[test]
    fn koridor_yalnizca_o5_kesimini_kapsar() {
        let koridor = koridor_bolgeleri();
        assert!(koridor.iter().any(|(il, _)| il == "İzmir"));
        assert!(koridor.iter().any(|(il, _)| il == "Manisa"));
        assert!(koridor.iter().any(|(_, ilce)| ilce == "Kemalpaşa"));
        assert!(koridor.iter().any(|(_, ilce)| ilce == "Savaştepe"));
        // Kapsam dışı iller listede olmamalı.
        assert!(!koridor.iter().any(|(il, _)| il == "İstanbul"));
        assert!(!koridor.iter().any(|(il, _)| il == "Kocaeli"));
        assert!(!koridor.iter().any(|(il, _)| il == "Bursa"));
    }

    #[test]
    fn profil_plani_google_adimlarini_icerir() {
        let metin = profil(&isletme(), &koridor_bolgeleri(), "2026-11-10");
        assert!(metin.contains("Çekici hizmeti"));
        assert!(metin.contains("7/24"));
        assert!(metin.contains("Yorum"));
        assert!(metin.contains("O-5"));
    }

    #[test]
    fn mesaj_ilce_ve_baglanti_icerir() {
        let metin = mesaj(&isletme(), "Manisa", "Akhisar");
        assert!(metin.contains("Akhisar"));
        assert!(metin.contains("Saruhanlı Oto Kurtarma"));
        assert!(metin.contains("maps/search"));
    }

    #[test]
    fn sektor_ada_gore_tahmin_edilir() {
        assert_eq!(
            sektor_tahmin("Saruhanlı Oto Kurtarma", "Saruhanlı / Manisa"),
            "Oto kurtarma"
        );
        assert_eq!(sektor_tahmin("Kuaför Ayşe", "Kula"), "Kuaför");
        assert_eq!(sektor_tahmin("XYZ Ltd", "Kula"), "Hizmet");
    }

    #[test]
    fn koridor_kemalpasadan_savastepeye_uzanir() {
        let koridor = koridor_bolgeleri();
        assert!(koridor
            .iter()
            .any(|(il, ilce)| il == "Balıkesir" && ilce == "Savaştepe"));
        assert!(koridor
            .iter()
            .any(|(il, ilce)| il == "İzmir" && ilce == "Kemalpaşa"));
        assert!(koridor
            .iter()
            .any(|(il, ilce)| il == "Manisa" && ilce == "Turgutlu"));
        // Kapsam dar tutulur: yalnızca O-5 üzerindeki bu yedi ilçe.
        assert_eq!(koridor.len(), 7);
    }

    #[test]
    fn oncelik_segmenti_izmir_manisa_ve_savastepe() {
        let oncelik = oncelikli_bolgeler();
        assert!(oncelik.iter().any(|(il, _)| il == "İzmir"));
        assert!(oncelik.iter().any(|(il, _)| il == "Manisa"));
        assert!(oncelik.iter().any(|(_, ilce)| ilce == "Savaştepe"));
        assert!(!oncelik.iter().any(|(il, _)| il == "İstanbul"));
        assert_eq!(oncelik.len(), 7);
    }

    #[test]
    fn sayfa_sekiz_hizmeti_ve_sss_listeler() {
        let html = sayfa(
            &isletme(),
            "Balıkesir",
            "Savaştepe",
            "2026-11-10",
            &koridor_bolgeleri(),
        );
        for hizmet in crate::otoyol_pin::HIZMETLER {
            let satir = if hizmet.contains("7/24") {
                format!("<li>{hizmet}</li>")
            } else {
                format!("<li>{hizmet} · 7/24</li>")
            };
            assert!(html.contains(&satir), "{hizmet} yok");
        }
        assert!(html.contains("akücü"));
        assert!(html.contains("hasOfferCatalog"));
        assert!(html.contains("\"Savaştepe\""));
    }

    #[test]
    fn siralama_plani_adimlarini_icerir() {
        let metin = siralama_plani(&isletme(), &koridor_bolgeleri(), "2026-11-10");
        for beklenen in [
            "Çekici hizmeti",
            "Savaştepe",
            "akücü",
            "Yorum",
            "sitemap",
            "7/24",
        ] {
            assert!(metin.contains(beklenen), "{beklenen} yok");
        }
    }

    #[test]
    fn sitemap_ve_robots_tum_sayfalari_gosterir() {
        let bolgeler = vec![
            ("İzmir".to_string(), "Bornova".to_string()),
            ("Balıkesir".to_string(), "Savaştepe".to_string()),
        ];
        let harita = sitemap(&bolgeler, "2026-11-10");
        assert!(harita.contains("<urlset"));
        assert!(harita.contains(&sayfa_url("İzmir", "Bornova")));
        assert!(harita.contains(&sayfa_url("Balıkesir", "Savaştepe")));
        assert!(!harita.contains(".html"), "sitemap uzantısız olmalı");
        assert!(robots().contains("Sitemap:"));
    }

    #[test]
    fn yaz_dosyalari_olusturur() {
        // Test kendi klasörüne yazsın; gerçek işletme çıktısını silmesin.
        let isletme = Isletme {
            ad: "Test Oto Kurtarma".to_string(),
            ..isletme()
        };
        assert!(klasor(&isletme)
            .to_string_lossy()
            .ends_with("testotokurtarma"));
        let bolgeler = vec![
            ("Manisa".to_string(), "Şehzadeler".to_string()),
            ("İzmir".to_string(), "Bornova".to_string()),
        ];
        let dosyalar = yaz(&isletme, &bolgeler, "2026-11-10").expect("yazılmalı");
        // 2 bölge + ana sayfa + plan + profil + 1. sıra planı + sitemap + robots + gönderiler
        assert_eq!(dosyalar.len(), 9);
        assert!(klasor(&isletme).join("siralama-plani.md").exists());
        assert!(klasor(&isletme).join("sitemap.xml").exists());
        assert!(klasor(&isletme).join("robots.txt").exists());
        assert!(klasor(&isletme).join("gonderiler.md").exists());
        for yol in &dosyalar {
            assert!(yol.exists(), "{} yok", yol.display());
        }
        let ana = fs::read_to_string(klasor(&isletme).join("index.html")).expect("ana sayfa");
        assert!(ana.contains("izmir-bornova"));
        let _ = fs::remove_dir_all(klasor(&isletme));
    }

    #[test]
    fn bos_bolge_listesi_hata_doner() {
        assert!(yaz(&isletme(), &[], "2026-11-10").is_err());
    }

    #[test]
    fn sayfa_komsu_bolgelere_link_verir() {
        let bolgeler = koridor_bolgeleri();
        let html = sayfa(&isletme(), "Manisa", "Saruhanlı", "2026-11-10", &bolgeler);
        assert!(html.contains("Yakın bölgeler"));
        assert!(html.contains("href=\"manisa-akhisar\""));
        assert!(!html.contains("href=\"manisa-saruhanli\""));
        assert!(!html.contains(".html\""), "iç bağlantılar uzantısız olmalı");
    }

    #[test]
    fn gonderiler_dort_hafta_ve_kontrol_listesi_icerir() {
        let metin = gonderiler(&isletme());
        assert!(metin.contains("1. hafta"));
        assert!(metin.contains("4. hafta"));
        assert!(metin.contains("Yorum isteme"));
        assert!(metin.contains("Google'a işlendi mi"));
        assert!(metin.contains("Search Console"));
        assert!(metin.contains("akücü"));
    }

    // Elle çalıştırılan üretim yardımcısı: cargo test gercek_uretim -- --ignored
    #[test]
    #[ignore]
    fn gercek_uretim_dosyalari_yazar() {
        let dosyalar = yaz(&isletme(), &koridor_bolgeleri(), "2026-11-10").expect("üretim");
        println!("URETILEN_DOSYA={}", dosyalar.len());
    }
}
