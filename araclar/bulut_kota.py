"""Google Cloud metrik senkronu, guvenlik marji ve pre-flight kilit.

google.rs'in kota bolumu bastan yazilir; Istemci, arayuz ve ayar dosyasi
noktasal olarak yamalanir.
"""

import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return Path(yol).read_text(encoding="utf-8")


def yaz(yol, metin):
    Path(yol).write_text(metin, encoding="utf-8", newline="")


def degistir(metin, eski, yeni, adet=1):
    assert metin.count(eski) == adet, f"beklenen {adet}, bulunan {metin.count(eski)}: {eski[:90]!r}"
    return metin.replace(eski, yeni)


# ===========================================================================
# 1) google.rs — kota bolumu (dosya basindan "Yanit modelleri" bolumune kadar)
# ===========================================================================
g = oku("src/google.rs")
i = g.index("struct Yanit {")
i = g.rindex("// ---------------------------------------------------------------------------", 0, i)
i = g.rindex("// ---------------------------------------------------------------------------", 0, i) + 1
govde = g[i:]

YENI_BAS = r'''//! Google Maps Platform istemcisi: Places API (New) Text Search + Geocoding,
//! kota muhasebesi ve Google Cloud gerçek kullanım senkronu.
//!
//! Tüm coğrafi arama ve koordinat çözme yalnızca bu modülden geçer; istekler
//! Tokio çalışma zamanında asenkron `reqwest` ile atılır. Sayaç `google_kota`
//! dosyasında tutulur, oturum sayacı süreç belleğindedir.
//!
//! Maliyet koruması beş katmanlıdır:
//! 1. Anahtar yalnızca bu modülde okunur (`anahtar::Kasa`), başka modül ağ
//!    isteği atmaz; harita paneli ağa hiç çıkmaz.
//! 2. Günlük/aylık istek sınırı istek gönderilmeden **önce** sayılır
//!    (pre-flight); sayım ile artırma aynı kilit altındadır, eşzamanlı
//!    istekler kotayı iki kez harcayamaz.
//! 3. Günlük limitin son `GUVENLIK_MARJI` isteği otomatik akışa kapalıdır;
//!    yalnızca kullanıcı onayıyla harcanır.
//! 4. Google Cloud Monitoring'dan gerçek istek sayısı 60 saniyede bir
//!    okunur, arayüzdeki yüzdeler bulut verisini esas alır.
//! 5. Aynı il/ilçe/sektör sonucu `sonuc_onbellek.json`'dan okunur; önbellek
//!    tazeyken Google'a hiç istek gitmez (main.rs).

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Places API (New) Text Search uç noktası.
pub const PLACES_URL: &str = "https://places.googleapis.com/v1/places:searchText";
/// Geocoding API uç noktası.
pub const GEOCODE_URL: &str = "https://maps.googleapis.com/maps/api/geocode/json";
/// Kalıcı istek sayacı.
pub const KOTA_DOSYASI: &str = "google_kota.json";
/// Yalnızca gereken alanlar istenir: faturalama kademesi ve yanıt boyutu küçük
/// kalır. `websiteUri` zorunlu, çünkü sitesiz filtre bu alana dayanır.
pub const ALAN_MASKESI: &str = "places.id,places.displayName,places.formattedAddress,places.location,places.websiteUri,places.nationalPhoneNumber,places.regularOpeningHours.weekdayDescriptions,places.rating,places.userRatingCount,places.businessStatus";
/// Google'ın aylık 200 $ ücretsiz kredisi yaklaşık 10.000 Text Search isteğine
/// denk gelir; yüzde göstergesi bu referansa göre hesaplanır.
pub const AYLIK_UCRETSIZ_ISTEK: usize = 10_000;
/// Günlük limitin bu oranı aşıldığında sarı alarm verilir.
pub const ALARM_ORANI: f64 = 0.80;
pub const VARSAYILAN_GUNLUK: usize = 20;
pub const VARSAYILAN_AYLIK: usize = 400;
pub const VARSAYILAN_SAYFA: usize = 2;
/// Güvenlik marjı: günlük limitin son bu kadar isteği otomatik taramaya
/// kapalıdır (15 → 11'de kilit); kalan istekler manuel onay ister.
pub const GUVENLIK_MARJI: usize = 4;
/// Google Cloud metriklerinin arka plan senkron aralığı (saniye).
pub const BULUT_SENKRON_SN: u64 = 60;
/// Cloud Monitoring v3 kökü: projenin gerçek istek kullanımı.
const MONITORING_URL: &str = "https://monitoring.googleapis.com/v3/projects";
/// Sayılan servisler: faturalanan iki Google Maps uç noktası.
const IZLENEN_SERVISLER: [&str; 2] = [
    "places.googleapis.com",
    "geocoding-backend.googleapis.com",
];

// ---------------------------------------------------------------------------
// Kota sayacı
// ---------------------------------------------------------------------------

/// Günlük/aylık istek sayacı ve buluttan gelen gerçek kullanım.
/// `google_kota.json` içinde tutulur.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Kota {
    #[serde(default)]
    pub gun: String,
    #[serde(default)]
    pub gunluk: usize,
    #[serde(default)]
    pub ay: String,
    #[serde(default)]
    pub aylik: usize,
    /// Kurulumdan bu yana toplam istek (bilgi amaçlı).
    #[serde(default)]
    pub toplam: u64,
    /// Google Cloud Monitoring'dan gelen gerçek günlük istek sayısı.
    #[serde(default)]
    pub bulut_gunluk: Option<usize>,
    /// Google Cloud Monitoring'dan gelen gerçek aylık istek sayısı.
    #[serde(default)]
    pub bulut_aylik: Option<usize>,
    /// Son başarılı senkronun yerel saat damgası.
    #[serde(default)]
    pub bulut_zaman: Option<String>,
    /// Son senkron hatası (panelde gösterilir, sayaçları değiştirmez).
    #[serde(default)]
    pub bulut_hata: Option<String>,
}

/// İstek ayrılmadan önce verilen karar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Karar {
    Izin,
    /// Günlük güvenlik marjı: kalan istekler manuel onay ister.
    GuvenlikMarji,
    GunlukDolu,
    AylikDolu,
}

/// Panelin renk eşiği.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seviye {
    Normal,
    Sari,
    Kirmizi,
}

/// Günlük limitin son `GUVENLIK_MARJI` isteği otomatik akışa kapalıdır.
/// Küçük limitlerde en az 1 istek otomatik kalır (1'lik limitte marj yok).
pub fn guvenlik_limiti(gunluk_limit: usize) -> usize {
    gunluk_limit.saturating_sub(GUVENLIK_MARJI).max(1)
}

impl Kota {
    pub fn yukle_yoldan(yol: &str) -> Self {
        std::fs::read_to_string(yol)
            .ok()
            .and_then(|metin| serde_json::from_str(&metin).ok())
            .unwrap_or_default()
    }

    pub fn kaydet_yoldan(&self, yol: &str) -> Result<(), String> {
        let metin = serde_json::to_string_pretty(self)
            .map_err(|hata| format!("Kota yazılamadı: {hata}"))?;
        std::fs::write(yol, metin).map_err(|hata| format!("Kota dosyası yazılamadı: {hata}"))
    }

    /// Gün/ay değiştiyse sayaçları sıfırlar ve istek için bir pay ayırır.
    /// İstek gönderilmeden sayılır ki hata olsa bile bütçe aşılmış olmasın.
    /// `onay` false iken güvenlik marjına girildiğinde pay ayrılmaz.
    pub fn karar(
        &mut self,
        bugun: &str,
        bu_ay: &str,
        gunluk_limit: usize,
        aylik_limit: usize,
        onay: bool,
    ) -> Karar {
        if self.gun != bugun {
            self.gun = bugun.to_string();
            self.gunluk = 0;
        }
        if self.ay != bu_ay {
            self.ay = bu_ay.to_string();
            self.aylik = 0;
        }
        if self.aylik >= aylik_limit {
            return Karar::AylikDolu;
        }
        if self.gunluk >= gunluk_limit {
            return Karar::GunlukDolu;
        }
        if !onay && self.gunluk >= guvenlik_limiti(gunluk_limit) {
            return Karar::GuvenlikMarji;
        }
        self.gunluk += 1;
        self.aylik += 1;
        self.toplam = self.toplam.saturating_add(1);
        Karar::Izin
    }

    pub fn ozet(&self, gunluk_limit: usize, aylik_limit: usize) -> Ozet {
        Ozet::hesapla(self, gunluk_limit, aylik_limit)
    }
}

/// Arayüzdeki canlı göstergenin tüm sayıları.
#[derive(Clone, Debug, PartialEq)]
pub struct Ozet {
    pub oturum: u64,
    pub gunluk: usize,
    pub aylik: usize,
    pub toplam: u64,
    pub gunluk_limit: usize,
    pub aylik_limit: usize,
    pub kalan_gun: usize,
    pub kalan_ay: usize,
    pub gun_yuzde: f64,
    pub ay_yuzde: f64,
    pub ucretsiz_yuzde: f64,
    pub seviye: Seviye,
    /// Otomatik akışın duracağı eşik (günlük limit - 4, en az 1).
    pub guvenlik_limiti: usize,
    /// Güvenlik marjına girildi: otomatik tarama kilitli.
    pub manuel_gerekli: bool,
    /// Bir sonraki UTC gece yarısına kalan saniye.
    pub sifirlanma: i64,
    /// Şu anda uçuştaki istek sayısı.
    pub ucus: u64,
    /// Google Cloud'dan gelen gerçek kullanım (varsa).
    pub bulut_gunluk: Option<usize>,
    pub bulut_aylik: Option<usize>,
    pub bulut_zaman: Option<String>,
    pub bulut_hata: Option<String>,
}

impl Ozet {
    fn hesapla(kota: &Kota, gunluk_limit: usize, aylik_limit: usize) -> Self {
        let gunluk_limit = gunluk_limit.max(1);
        let aylik_limit = aylik_limit.max(1);
        let guvenlik_limiti = guvenlik_limiti(gunluk_limit);
        let manuel_gerekli = kota.gunluk >= guvenlik_limiti && kota.gunluk < gunluk_limit;
        let gun_yuzde = kotlen(kota.gunluk, gunluk_limit);
        let ay_yuzde = kotlen(kota.aylik, aylik_limit);
        // Bulut verisi geldiyse gerçek kullanım o; yerel sayaç yalnızca taban.
        let aylik_gorunen = kota.bulut_aylik.unwrap_or(0).max(kota.aylik);
        let ucretsiz_yuzde = kotlen(aylik_gorunen, AYLIK_UCRETSIZ_ISTEK);
        let seviye = if gun_yuzde >= 1.0 || ay_yuzde >= 1.0 {
            Seviye::Kirmizi
        } else if manuel_gerekli || gun_yuzde >= ALARM_ORANI || ay_yuzde >= ALARM_ORANI {
            Seviye::Sari
        } else {
            Seviye::Normal
        };
        Self {
            oturum: OTURUM.load(Ordering::Relaxed),
            gunluk: kota.gunluk,
            aylik: kota.aylik,
            toplam: kota.toplam,
            gunluk_limit,
            aylik_limit,
            kalan_gun: gunluk_limit.saturating_sub(kota.gunluk),
            kalan_ay: aylik_limit.saturating_sub(kota.aylik),
            gun_yuzde,
            ay_yuzde,
            ucretsiz_yuzde,
            seviye,
            guvenlik_limiti,
            manuel_gerekli,
            sifirlanma: sifirlanma_saniye(),
            ucus: UCUS.load(Ordering::Relaxed),
            bulut_gunluk: kota.bulut_gunluk,
            bulut_aylik: kota.bulut_aylik,
            bulut_zaman: kota.bulut_zaman.clone(),
            bulut_hata: kota.bulut_hata.clone(),
        }
    }
}

/// Anlık gösterge: günlük limitin son 4 isteği manuel onaya kilitli.
pub fn manuel_gerekli(gunluk_limit: usize) -> bool {
    let kilit = kota();
    let sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.gunluk >= guvenlik_limiti(gunluk_limit) && sayac.gunluk < gunluk_limit.max(1)
}

fn kotlen(kullanilan: usize, limit: usize) -> f64 {
    if limit == 0 {
        0.0
    } else {
        kullanilan as f64 / limit as f64
    }
}

static KOTA: OnceLock<Mutex<Kota>> = OnceLock::new();
static OTURUM: AtomicU64 = AtomicU64::new(0);
/// Uçuştaki istek sayısı: panelde "canlı istek" olarak görünür.
static UCUS: AtomicU64 = AtomicU64::new(0);

/// Süreç genelindeki kota sayacı: arayüz her karede okuyabilir.
pub fn kota() -> &'static Mutex<Kota> {
    KOTA.get_or_init(|| Mutex::new(Kota::yukle_yoldan(KOTA_DOSYASI)))
}

/// İstek öncesi kota kararı (pre-flight). Kontrol, pay ayırma ve diske yazma
/// **aynı Mutex kilidi** altında yapılır: eşzamanlı iki istek aynı kotayı iki
/// kez harcayamaz. İzin verilirse uçuş sayacı artar.
pub fn kota_karari(gunluk_limit: usize, aylik_limit: usize, onay: bool) -> Karar {
    let bugun = bugun_anahtari();
    let bu_ay = ay_anahtari();
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    let karar = sayac.karar(&bugun, &bu_ay, gunluk_limit, aylik_limit, onay);
    if karar == Karar::Izin {
        OTURUM.fetch_add(1, Ordering::Relaxed);
        UCUS.fetch_add(1, Ordering::Relaxed);
        let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
    }
    karar
}

/// Ayrılan isteğin uçuş sayacını düşürür (Drop ile otomatik).
pub struct Ucus;

impl Drop for Ucus {
    fn drop(&mut self) {
        UCUS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Ayrılmış bir istek için uçuş işareti.
pub fn ucus() -> Ucus {
    Ucus
}

/// Arayüz için anlık özet (saniyelik tazeleme).
pub fn ozet(gunluk_limit: usize, aylik_limit: usize) -> Ozet {
    let kilit = kota();
    let sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.ozet(gunluk_limit, aylik_limit)
}

pub fn bugun_anahtari() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn ay_anahtari() -> String {
    chrono::Local::now().format("%Y-%m").to_string()
}

/// Bir sonraki UTC gece yarısına kalan saniye: Google günlük kota kesimi.
pub fn sifirlanma_saniye() -> i64 {
    let saniye = chrono::Utc::now().timestamp();
    86_400 - saniye.rem_euclid(86_400)
}

// ---------------------------------------------------------------------------
// Google Cloud gerçek kullanım senkronu (Cloud Monitoring v3)
// ---------------------------------------------------------------------------

/// Google Cloud'dan okunan gerçek istek kullanımı.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BulutOzet {
    pub gunluk: usize,
    pub aylik: usize,
    /// Senkron saati (yerel, HH:MM:SS).
    pub zaman: String,
}

/// Projenin Places + Geocoding istek sayısını Cloud Monitoring'dan okur.
/// Okuma isteği Maps kotası harcamaz; API anahtarı yeterlidir.
pub fn bulut_senkron(anahtar: &str, proje: &str) -> Result<BulutOzet, String> {
    let proje = proje.trim();
    if proje.is_empty() {
        return Err("Google Cloud proje kimliği girilmemiş (panelden ekleyin).".to_string());
    }
    if !crate::anahtar::gecerli_anahtar(anahtar) {
        return Err("Kayıtlı API anahtarı yok; bulut verisi okunamadı.".to_string());
    }
    let http = reqwest::Client::builder()
        .user_agent("NexusEnterprise/0.1 (bulut metrik senkronu)")
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|hata| format!("Metrik istemcisi kurulamadı: {hata}"))?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|hata| format!("Metrik çalışma zamanı kurulamadı: {hata}"))?;
    let simdi = chrono::Utc::now();
    let ay_basi = format!("{}-01T00:00:00Z", simdi.format("%Y-%m"));
    let bitis = simdi.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    rt.block_on(bulut_istek(&http, anahtar, proje, &ay_basi, &bitis))
}

async fn bulut_istek(
    http: &reqwest::Client,
    anahtar: &str,
    proje: &str,
    baslangic: &str,
    bitis: &str,
) -> Result<BulutOzet, String> {
    let filtre = format!(
        "metric.type=\"serviceruntime.googleapis.com/api/request_count\" AND (resource.labels.service=\"{}\" OR resource.labels.service=\"{}\")",
        IZLENEN_SERVISLER[0], IZLENEN_SERVISLER[1]
    );
    let parametreler = vec![
        ("filter", filtre),
        ("interval.startTime", baslangic.to_string()),
        ("interval.endTime", bitis.to_string()),
        ("aggregation.alignmentPeriod", "3600s".to_string()),
        ("aggregation.perSeriesAligner", "ALIGN_SUM".to_string()),
        ("aggregation.crossSeriesReducer", "REDUCE_SUM".to_string()),
        ("key", anahtar.to_string()),
    ];
    let cevap = http
        .get(format!("{MONITORING_URL}/{proje}/timeSeries"))
        .query(&parametreler)
        .send()
        .await
        .map_err(|hata| format!("Google Cloud metriği alınamadı: {hata}"))?;
    let kod = cevap.status();
    let metin = cevap
        .text()
        .await
        .map_err(|hata| format!("Metrik yanıtı okunamadı: {hata}"))?;
    if !kod.is_success() {
        return Err(format!(
            "Google Cloud Monitoring {}: {}",
            kod.as_u16(),
            hata_metni(&metin)
        ));
    }
    let yanit: BulutYanit =
        serde_json::from_str(&metin).map_err(|hata| format!("Metrik yanıtı çözümlenemedi: {hata}"))?;
    let bugun = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let (gunluk, aylik) = bulut_topla(&yanit, &bugun);
    Ok(BulutOzet {
        gunluk,
        aylik,
        zaman: chrono::Local::now().format("%H:%M:%S").to_string(),
    })
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutYanit {
    #[serde(rename = "timeSeries", default)]
    time_series: Vec<BulutSeri>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutSeri {
    #[serde(default)]
    points: Vec<BulutNokta>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutNokta {
    #[serde(default)]
    interval: BulutAralik,
    #[serde(default)]
    value: BulutDeger,
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutAralik {
    #[serde(rename = "startTime", default)]
    start: String,
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutDeger {
    #[serde(rename = "int64Value", default)]
    tam: Option<String>,
    #[serde(rename = "doubleValue", default)]
    ondalik: Option<f64>,
}

impl BulutDeger {
    fn sayi(&self) -> f64 {
        self.tam
            .as_deref()
            .and_then(|deger| deger.parse::<f64>().ok())
            .or(self.ondalik)
            .unwrap_or(0.0)
    }
}

/// Saatlik kovaları toplar: aylık tüm kovalar, günlük yalnızca bugünün
/// (UTC) kovaları. Saatlik hizalama gece yarısına tam oturur.
fn bulut_topla(yanit: &BulutYanit, bugun: &str) -> (usize, usize) {
    let mut gunluk = 0.0f64;
    let mut aylik = 0.0f64;
    for seri in &yanit.time_series {
        for nokta in &seri.points {
            let deger = nokta.value.sayi();
            aylik += deger;
            if nokta.interval.start.get(..10) == Some(bugun) {
                gunluk += deger;
            }
        }
    }
    (gunluk.round() as usize, aylik.round() as usize)
}

/// Başarılı senkronu kalıcı sayaca yazar (arayüz dosyadan okur).
pub fn bulut_yaz(sonuc: &BulutOzet) {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_gunluk = Some(sonuc.gunluk);
    sayac.bulut_aylik = Some(sonuc.aylik);
    sayac.bulut_zaman = Some(sonuc.zaman.clone());
    sayac.bulut_hata = None;
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
}

/// Senkron hatasını panelde göstermek için saklar; sayaçlara dokunmaz.
pub fn bulut_hata_yaz(hata: &str) {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_hata = Some(hata.chars().take(180).collect());
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
}

'''

g = YENI_BAS + govde

# --- Istemci: manuel onay alani ------------------------------------------------
g = degistir(
    g,
    """    pub gunluk_limit: usize,
    pub aylik_limit: usize,
    pub sayfa_siniri: usize,
}""",
    """    pub gunluk_limit: usize,
    pub aylik_limit: usize,
    pub sayfa_siniri: usize,
    /// Güvenlik marjı aşılsın mı: yalnızca kullanıcı onayıyla true.
    manuel_onay: bool,
}""",
)

g = degistir(
    g,
    """        gunluk_limit: usize,
        aylik_limit: usize,
        sayfa_siniri: usize,
    ) -> Result<Self, String> {""",
    """        gunluk_limit: usize,
        aylik_limit: usize,
        sayfa_siniri: usize,
        manuel_onay: bool,
    ) -> Result<Self, String> {""",
)

g = degistir(
    g,
    """            sayfa_siniri: sayfa_siniri.clamp(1, 3),
        })""",
    """            sayfa_siniri: sayfa_siniri.clamp(1, 3),
            manuel_onay,
        })""",
)

# --- ara(): pre-flight + guvenlik marji + ucus ---------------------------------
g = degistir(
    g,
    """            match kota_karari(self.gunluk_limit, self.aylik_limit) {
                Karar::Izin => {}
                Karar::GunlukDolu => {""",
    """            match kota_karari(self.gunluk_limit, self.aylik_limit, self.manuel_onay) {
                Karar::Izin => {}
                Karar::GuvenlikMarji => {
                    durduruldu = Some(format!(
                        "GÜVENLİK MARJI: günlük {}/{} kullanıldı; kalan {} istek panelden onay ister.",
                        ozet(self.gunluk_limit, self.aylik_limit).gunluk,
                        self.gunluk_limit,
                        self.gunluk_limit.saturating_sub(guvenlik_limiti(self.gunluk_limit))
                    ));
                    break;
                }
                Karar::GunlukDolu => {""",
)

g = degistir(
    g,
    """            let govde = arama_govdesi(il, ilce, sektor, jeton.as_deref());
            let yanit = self""",
    """            // Ayrılan istek uçuşta: yanıt gelince sayaç kendiliğinden düşer.
            let _ucus = ucus();
            let govde = arama_govdesi(il, ilce, sektor, jeton.as_deref());
            let yanit = self""",
)

# --- geocode(): guvenlik marji -------------------------------------------------
g = degistir(
    g,
    """        if let Karar::GunlukDolu | Karar::AylikDolu =
            kota_karari(self.gunluk_limit, self.aylik_limit)
        {
            return Err("Google istek sınırı doldu; konum çözülemedi.".to_string());
        }""",
    """        match kota_karari(self.gunluk_limit, self.aylik_limit, self.manuel_onay) {
            Karar::Izin => {}
            Karar::GuvenlikMarji => {
                return Err(format!(
                    "Güvenlik marjı: günlük {}/{} istek kullanıldı, konum çözümü onay bekliyor.",
                    ozet(self.gunluk_limit, self.aylik_limit).gunluk,
                    self.gunluk_limit
                ))
            }
            Karar::GunlukDolu | Karar::AylikDolu => {
                return Err("Google istek sınırı doldu; konum çözülemedi.".to_string())
            }
        }
        let _ucus = ucus();""",
)

# --- anahtar_testi: yeni imza --------------------------------------------------
g = degistir(
    g,
    """        VARSAYILAN_GUNLUK,
        VARSAYILAN_AYLIK,
        1,
    )?;""",
    """        VARSAYILAN_GUNLUK,
        VARSAYILAN_AYLIK,
        1,
        false,
    )?;""",
)

# --- testler: karar imzasi + yeni testler -------------------------------------
g = g.replace('kota.karar("2026-11-03", "2026-11", 2, 3)',
              'kota.karar("2026-11-03", "2026-11", 2, 3, true)')
g = g.replace('kota.karar("2026-11-04", "2026-11", 2, 3)',
              'kota.karar("2026-11-04", "2026-11", 2, 3, true)')
g = g.replace('kota.karar("2026-12-01", "2026-12", 2, 3)',
              'kota.karar("2026-12-01", "2026-12", 2, 3, true)')
g = g.replace('kota.karar("2026-11-03", "2026-11", 10, 100)',
              'kota.karar("2026-11-03", "2026-11", 10, 100, true)')
g = degistir(
    g,
    "let istemci = Istemci::yeni(anahtar, 5, 50, 1).expect(\"istemci kurulamadı\");",
    "let istemci = Istemci::yeni(anahtar, 5, 50, 1, false).expect(\"istemci kurulamadı\");",
)

g = degistir(
    g,
    """        assert!(guclu > 80, "tam kayıt 80 üzeri olmalı: {guclu}");
        assert!(guclu <= 100);
    }
""",
    """        assert!(guclu > 80, "tam kayıt 80 üzeri olmalı: {guclu}");
        assert!(guclu <= 100);
    }

    #[test]
    fn guvenlik_marji_son_dort_istegi_kilitler() {
        let mut kota = Kota::default();
        for _ in 0..11 {
            assert_eq!(
                kota.karar("2026-11-03", "2026-11", 15, 400, false),
                Karar::Izin
            );
        }
        assert_eq!(guvenlik_limiti(15), 11);
        assert_eq!(
            kota.karar("2026-11-03", "2026-11", 15, 400, false),
            Karar::GuvenlikMarji,
            "11/15'te otomatik akış durmalı"
        );
        assert!(
            kota.ozet(15, 400).manuel_gerekli,
            "panel manuel onay istemeli"
        );
        assert_eq!(kota.ozet(15, 400).seviye, Seviye::Sari);
        // Onayla kalan 4 istek harcanır, sonra mutlak sınır.
        for _ in 0..4 {
            assert_eq!(
                kota.karar("2026-11-03", "2026-11", 15, 400, true),
                Karar::Izin
            );
        }
        assert_eq!(
            kota.karar("2026-11-03", "2026-11", 15, 400, true),
            Karar::GunlukDolu
        );
    }

    #[test]
    fn es_zamanli_istekler_kotayi_asamaz() {
        let paylasilan = std::sync::Arc::new(Mutex::new(Kota::default()));
        let izin = std::sync::Arc::new(AtomicU64::new(0));
        let mut eller = Vec::new();
        for _ in 0..8 {
            let kilit = paylasilan.clone();
            let sayac = izin.clone();
            eller.push(std::thread::spawn(move || {
                for _ in 0..10 {
                    let mut kota = kilit.lock().unwrap_or_else(|z| z.into_inner());
                    if kota.karar("2026-11-03", "2026-11", 30, 400, true) == Karar::Izin {
                        sayac.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for el in eller {
            el.join().expect("iş parçacığı düştü");
        }
        assert_eq!(
            izin.load(Ordering::Relaxed),
            30,
            "80 denemede yalnızca 30 istek ayrılmalı"
        );
    }

    #[test]
    fn sifirlanma_saniyesi_bir_gun_icinde() {
        let kalan = sifirlanma_saniye();
        assert!(kalan > 0 && kalan <= 86_400, "{kalan}");
    }

    #[test]
    fn bulut_yaniti_gunluk_ve_aylik_toplar() {
        let metin = r#"{
            "timeSeries": [{
                "points": [
                    {"interval":{"startTime":"2026-11-03T09:00:00Z"},"value":{"int64Value":"7"}},
                    {"interval":{"startTime":"2026-11-03T08:00:00Z"},"value":{"int64Value":"3"}},
                    {"interval":{"startTime":"2026-11-02T22:00:00Z"},"value":{"doubleValue":5.0}}
                ]
            }]
        }"#;
        let yanit: BulutYanit = serde_json::from_str(metin).expect("metrik çözümlenemedi");
        let (gunluk, aylik) = bulut_topla(&yanit, "2026-11-03");
        assert_eq!(gunluk, 10, "bugünün saatlik kovaları toplanmalı");
        assert_eq!(aylik, 15, "ayın tüm kovaları toplanmalı");
    }
""",
)

yaz("src/google.rs", g)


# ===========================================================================
# 2) main.rs — ayar, alanlar, tarama onayi, bulut dongusu
# ===========================================================================
m = oku("src/main.rs")

m = degistir(
    m,
    """    api_gunluk: usize,
    api_aylik: usize,
    api_sayfa: usize,""",
    """    api_gunluk: usize,
    api_aylik: usize,
    api_sayfa: usize,
    /// Google Cloud proje kimliği: gerçek kullanım metriği bu projeden okunur.
    api_proje: String,
    /// Manuel "Şimdi senkronla" isteğinin sonucu.
    api_bulut: Option<Receiver<Result<google::BulutOzet, String>>>,
    /// Güvenlik marjı onayı: yalnızca kullanıcı düğmeye basınca true olur,
    /// tarama başlarken tüketilir (tek kullanımlık).
    kota_onay: bool,""",
)

m = degistir(
    m,
    """            api_gunluk: ayar.google_gunluk_limit,
            api_aylik: ayar.google_aylik_limit,
            api_sayfa: ayar.google_sayfa,""",
    """            api_gunluk: ayar.google_gunluk_limit,
            api_aylik: ayar.google_aylik_limit,
            api_sayfa: ayar.google_sayfa,
            api_proje: ayar.google_proje.clone(),
            api_bulut: None,
            kota_onay: false,""",
)

m = degistir(
    m,
    """    /// Tarama başına okunacak sayfa sayısı (1-3 · sayfa başına 20 kayıt).
    #[serde(default = "google_sayfa_varsayilan")]
    google_sayfa: usize,
}""",
    """    /// Tarama başına okunacak sayfa sayısı (1-3 · sayfa başına 20 kayıt).
    #[serde(default = "google_sayfa_varsayilan")]
    google_sayfa: usize,
    /// Google Cloud proje kimliği: Cloud Monitoring gerçek kullanım sorgusu.
    #[serde(default)]
    google_proje: String,
}""",
)

m = degistir(
    m,
    """            google_gunluk_limit: google_gunluk_varsayilan(),
            google_aylik_limit: google_aylik_varsayilan(),
            google_sayfa: google_sayfa_varsayilan(),
        }""",
    """            google_gunluk_limit: google_gunluk_varsayilan(),
            google_aylik_limit: google_aylik_varsayilan(),
            google_sayfa: google_sayfa_varsayilan(),
            google_proje: String::new(),
        }""",
)

m = degistir(
    m,
    """            google_gunluk_limit: self.api_gunluk,
            google_aylik_limit: self.api_aylik,
            google_sayfa: self.api_sayfa,
        }""",
    """            google_gunluk_limit: self.api_gunluk,
            google_aylik_limit: self.api_aylik,
            google_sayfa: self.api_sayfa,
            google_proje: self.api_proje.trim().to_string(),
        }""",
)

# tarama baslatma: onay tek kullanimlik
m = degistir(
    m,
    """        let sektor = self.sectors[self.selected_sector_idx].to_string();
        thread::spawn(move || tarama_calistir(tx, il, ilce, sektor));""",
    """        let sektor = self.sectors[self.selected_sector_idx].to_string();
        let onay = self.kota_onay;
        self.kota_onay = false;
        thread::spawn(move || tarama_calistir(tx, il, ilce, sektor, onay));""",
)

m = degistir(
    m,
    "fn tarama_calistir(tx: Sender<Mesaj>, il: String, ilce: Option<String>, sektor: String) {",
    "fn tarama_calistir(\n    tx: Sender<Mesaj>,\n    il: String,\n    ilce: Option<String>,\n    sektor: String,\n    onay: bool,\n) {",
)

m = degistir(
    m,
    """        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        ayar.google_sayfa,
    ) {""",
    """        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        ayar.google_sayfa,
        onay,
    ) {""",
)

m = degistir(
    m,
    """    let istemci = google::Istemci::yeni(
        anahtar.to_string(),
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        1,
    )?;""",
    """    let istemci = google::Istemci::yeni(
        anahtar.to_string(),
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        1,
        false,
    )?;""",
)

# manuel senkron ve bulut dongusu poll cagrisi
m = degistir(
    m,
    """        self.api_test_poll();
        while let Ok(mesaj) = self.log_rx.try_recv() {""",
    """        self.api_test_poll();
        self.api_senkron_poll();
        while let Ok(mesaj) = self.log_rx.try_recv() {""",
)

# bulut dongusu: LeadApp::new icinde bir kez baslatilir
m = degistir(
    m,
    """        let ayar = ayar_oku();
        let (whatsapp_phone, whatsapp_status, whatsapp_bagli) = whatsapp_baslangic(&ayar);""",
    """        let ayar = ayar_oku();
        let (whatsapp_phone, whatsapp_status, whatsapp_bagli) = whatsapp_baslangic(&ayar);
        bulut_dongusu_baslat();""",
)

m = degistir(
    m,
    """pub struct LeadApp {""",
    """/// Google Cloud metriklerini 60 saniyede bir arka planda senkronlar.
/// Anahtar ve proje kimliği her turda yeniden okunur: kullanıcı panelden
/// değiştirdiğinde döngü kendiliğinden yeni değerlerle devam eder.
fn bulut_dongusu_baslat() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(google::BULUT_SENKRON_SN));
        let anahtar = anahtar::Kasa::yukle().anahtar().map(str::to_string);
        let Some(anahtar) = anahtar else {
            continue;
        };
        let proje = ayar_oku().google_proje;
        if proje.trim().is_empty() {
            continue;
        }
        match google::bulut_senkron(&anahtar, &proje) {
            Ok(sonuc) => google::bulut_yaz(&sonuc),
            Err(hata) => google::bulut_hata_yaz(&hata),
        }
    });
}

pub struct LeadApp {""",
)

yaz("src/main.rs", m)

print("google.rs ve main.rs guncellendi")
