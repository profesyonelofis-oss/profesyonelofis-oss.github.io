//! Google Maps Platform istemcisi: Places API (New) Text Search + Geocoding,
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
const IZLENEN_SERVISLER: [&str; 2] = ["places.googleapis.com", "geocoding-backend.googleapis.com"];

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
    /// Buluttan gelen Places istek sayısı (bu ay).
    #[serde(default)]
    pub bulut_places: Option<usize>,
    /// Buluttan gelen Geocoding istek sayısı (bu ay).
    #[serde(default)]
    pub bulut_geocode: Option<usize>,
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
    /// Bulut aylık kırılımı: Places ve Geocoding ayrı faturalanır.
    pub bulut_places: Option<usize>,
    pub bulut_geocode: Option<usize>,
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
            bulut_places: kota.bulut_places,
            bulut_geocode: kota.bulut_geocode,
        }
    }
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
    // Gün/ay anahtarı UTC: geri sayım ve Google kota kesimi aynı ana oturur.
    let bugun = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let bu_ay = chrono::Utc::now().format("%Y-%m").to_string();
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    let karar = sayac.karar(&bugun, &bu_ay, gunluk_limit, aylik_limit, onay);
    if karar == Karar::Izin {
        OTURUM.fetch_add(1, Ordering::Relaxed);
        UCUS.fetch_add(1, Ordering::Relaxed);
        let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
        drop(sayac);
        // İstek sonrası gerçek kullanım hemen tazelenir (engellemesiz).
        senkron_talep_et();
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
    /// Bu ayın Places (New) istek sayısı.
    pub places: usize,
    /// Bu ayın Geocoding istek sayısı.
    pub geocode: usize,
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
        // Seri bazında toplanır: resource.labels.service ile Places/Geocoding
        // ayrımı korunur, böylece panelde servis kırılımı görünür.
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
    let yanit: BulutYanit = serde_json::from_str(&metin)
        .map_err(|hata| format!("Metrik yanıtı çözümlenemedi: {hata}"))?;
    let bugun = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let (gunluk, aylik, places, geocode) = bulut_topla(&yanit, &bugun);
    Ok(BulutOzet {
        gunluk,
        aylik,
        places,
        geocode,
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
    resource: BulutKaynak,
    #[serde(default)]
    points: Vec<BulutNokta>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutKaynak {
    #[serde(default)]
    labels: std::collections::HashMap<String, String>,
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
fn bulut_topla(yanit: &BulutYanit, bugun: &str) -> (usize, usize, usize, usize) {
    let mut gunluk = 0.0f64;
    let mut aylik = 0.0f64;
    let mut places = 0.0f64;
    let mut geocode = 0.0f64;
    for seri in &yanit.time_series {
        let servis = seri
            .resource
            .labels
            .get("service")
            .map(String::as_str)
            .unwrap_or("");
        let mut seri_toplam = 0.0f64;
        for nokta in &seri.points {
            let deger = nokta.value.sayi();
            seri_toplam += deger;
            if nokta.interval.start.get(..10) == Some(bugun) {
                gunluk += deger;
            }
        }
        aylik += seri_toplam;
        if servis.starts_with("places") {
            places += seri_toplam;
        } else if servis.starts_with("geocoding") {
            geocode += seri_toplam;
        }
    }
    (
        gunluk.round() as usize,
        aylik.round() as usize,
        places.round() as usize,
        geocode.round() as usize,
    )
}

/// Başarılı senkronu kalıcı sayaca yazar (arayüz dosyadan okur).
pub fn bulut_yaz(sonuc: &BulutOzet) {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_gunluk = Some(sonuc.gunluk);
    sayac.bulut_aylik = Some(sonuc.aylik);
    sayac.bulut_places = Some(sonuc.places);
    sayac.bulut_geocode = Some(sonuc.geocode);
    sayac.bulut_zaman = Some(sonuc.zaman.clone());
    sayac.bulut_hata = None;
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
}

/// Anahtar/proje değiştiğinde bulut sayaç önbelleğini sıfırlar.
/// Paneldeki eski oturum verisi yeni anahtarla karışmasın diye çağrılır.
pub fn bulut_temizle() {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_gunluk = None;
    sayac.bulut_aylik = None;
    sayac.bulut_places = None;
    sayac.bulut_geocode = None;
    sayac.bulut_zaman = None;
    sayac.bulut_hata = None;
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
}

/// Senkron hatasını panelde gösterir ve `bulut_senkron.log` dosyasına yazar.
/// Anahtar kasasına dokunmaz; hata metni anahtar içermez.
pub fn bulut_hata_yaz(hata: &str) {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_hata = Some(hata.chars().take(180).collect());
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
    drop(sayac);
    if std::fs::metadata(BULUT_LOG)
        .map(|bilgi| bilgi.len() > 64 * 1024)
        .unwrap_or(false)
    {
        let _ = std::fs::remove_file(BULUT_LOG);
    }
    if let Ok(mut dosya) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(BULUT_LOG)
    {
        use std::io::Write;
        let satir = format!(
            "{} · {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            hata.chars().take(300).collect::<String>()
        );
        let _ = dosya.write_all(satir.as_bytes());
    }
}

/// Senkron hatalarının zaman damgalı kaydı (64 KB'ı aşınca yenilenir).
const BULUT_LOG: &str = "bulut_senkron.log";

static UYANDIR: OnceLock<Mutex<Option<std::sync::mpsc::Sender<()>>>> = OnceLock::new();

/// Arka plan senkron döngüsünü hemen uyandırır: her API isteğinden sonra
/// gerçek kullanımın gecikmeden tazelenmesini sağlar.
pub fn senkron_talep_et() {
    let kilit = UYANDIR.get_or_init(|| Mutex::new(None));
    if let Ok(gonder) = kilit.lock() {
        if let Some(gonder) = gonder.as_ref() {
            let _ = gonder.send(());
        }
    }
}

/// Arka plan döngüsünün uyandırma kanalını bağlar (main bir kez çağırır).
pub fn senkron_uyandirici(gonder: std::sync::mpsc::Sender<()>) {
    let kilit = UYANDIR.get_or_init(|| Mutex::new(None));
    if let Ok(mut kilit) = kilit.lock() {
        *kilit = Some(gonder);
    }
}

// ---------------------------------------------------------------------------
// Yanıt modelleri (Serde)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, serde::Deserialize)]
struct Yanit {
    #[serde(default)]
    places: Vec<Yer>,
    #[serde(rename = "nextPageToken", default)]
    next_page_token: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Yer {
    #[serde(default)]
    id: String,
    #[serde(rename = "displayName", default)]
    display_name: Option<Ad>,
    #[serde(rename = "formattedAddress", default)]
    formatted_address: Option<String>,
    #[serde(default)]
    location: Option<Koordinat>,
    #[serde(rename = "websiteUri", default)]
    website_uri: Option<String>,
    #[serde(rename = "nationalPhoneNumber", default)]
    national_phone_number: Option<String>,
    #[serde(rename = "regularOpeningHours", default)]
    regular_opening_hours: Option<Saatler>,
    #[serde(default)]
    rating: Option<f64>,
    #[serde(rename = "userRatingCount", default)]
    user_rating_count: Option<u32>,
    #[serde(rename = "businessStatus", default)]
    business_status: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Ad {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Koordinat {
    #[serde(default)]
    latitude: f64,
    #[serde(default)]
    longitude: f64,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Saatler {
    #[serde(rename = "weekdayDescriptions", default)]
    weekday_descriptions: Vec<String>,
}

/// Ayrıştırılmış işletme: `main.rs` bunu `BusinessLead`'e çevirir. Modül
/// ayrımı korunur, arayüz yapısı burada bilinmez.
#[derive(Clone, Debug)]
pub struct BizKayit {
    pub ad: String,
    pub adres: String,
    pub telefon: String,
    pub kaynak: String,
    pub konum: Option<(f64, f64)>,
    pub saatler: String,
    pub yildiz: Option<f32>,
    pub inceleme: Option<u32>,
    pub doluluk: u8,
}

/// Bir aramanın sonucu: kayıtlar + elenen siteli sayısı + okunan sayfa.
pub struct AraSonuc {
    pub kayitlar: Vec<BizKayit>,
    pub siteli: usize,
    pub sayfa: usize,
    /// Kota veya hata nedeniyle erken durulduysa açıklaması.
    pub durduruldu: Option<String>,
}

// ---------------------------------------------------------------------------
// İstemci
// ---------------------------------------------------------------------------

/// Google istemcisi: anahtar, ayarlar, HTTP havuzu ve Tokio çalışma zamanı.
pub struct Istemci {
    key: String,
    http: reqwest::Client,
    rt: tokio::runtime::Runtime,
    pub gunluk_limit: usize,
    pub aylik_limit: usize,
    pub sayfa_siniri: usize,
    /// Güvenlik marjı aşılsın mı: yalnızca kullanıcı onayıyla true.
    manuel_onay: bool,
}

impl Istemci {
    pub fn yeni(
        key: String,
        gunluk_limit: usize,
        aylik_limit: usize,
        sayfa_siniri: usize,
        manuel_onay: bool,
    ) -> Result<Self, String> {
        if !crate::anahtar::gecerli_anahtar(&key) {
            return Err("API anahtarı geçersiz görünüyor.".to_string());
        }
        let http = reqwest::Client::builder()
            .user_agent("NexusEnterprise/0.1 (Windows masaüstü keşif uygulaması)")
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|hata| format!("HTTP istemcisi kurulamadı: {hata}"))?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|hata| format!("Tokio çalışma zamanı kurulamadı: {hata}"))?;
        Ok(Self {
            key,
            http,
            rt,
            gunluk_limit: gunluk_limit.max(1),
            aylik_limit: aylik_limit.max(1),
            sayfa_siniri: sayfa_siniri.clamp(1, 3),
            manuel_onay,
        })
    }

    /// Text Search: sayfalar hâlinde en fazla `SONUC_SINIRI` kayıt toplar.
    /// Her sayfa ayrı faturalanır; sayfa sayısı ayarlardan sınırlıdır.
    pub fn ara(&self, il: &str, ilce: &str, sektor: &str) -> Result<AraSonuc, String> {
        let mut kayitlar: Vec<BizKayit> = Vec::new();
        let mut gorulen: HashSet<String> = HashSet::new();
        let mut siteli = 0usize;
        let mut jeton: Option<String> = None;
        let mut sayfa = 0usize;
        let mut durduruldu: Option<String> = None;

        while sayfa < self.sayfa_siniri && kayitlar.len() < crate::SONUC_SINIRI {
            match kota_karari(self.gunluk_limit, self.aylik_limit, self.manuel_onay) {
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
                Karar::GunlukDolu => {
                    durduruldu = Some(format!(
                        "Google günlük istek sınırı doldu ({}/{}); yarın devam edin.",
                        ozet(self.gunluk_limit, self.aylik_limit).gunluk,
                        self.gunluk_limit
                    ));
                    break;
                }
                Karar::AylikDolu => {
                    durduruldu = Some(format!(
                        "Google aylık istek sınırı doldu ({}/{}); API & Kota Yönetim'den artırın.",
                        ozet(self.gunluk_limit, self.aylik_limit).aylik,
                        self.aylik_limit
                    ));
                    break;
                }
            }
            // Ayrılan istek uçuşta: yanıt gelince sayaç kendiliğinden düşer.
            let _ucus = ucus();
            let govde = arama_govdesi(il, ilce, sektor, jeton.as_deref());
            let yanit = self
                .rt
                .block_on(places_istek(&self.http, &self.key, &govde))?;
            let (liste, sayfa_siteli) = isle(&yanit)?;
            siteli += sayfa_siteli;
            for kayit in liste {
                if gorulen.insert(kayit.kaynak.clone()) {
                    kayitlar.push(kayit);
                }
            }
            sayfa += 1;
            jeton = yanit.next_page_token;
            if jeton.is_none() {
                break;
            }
        }

        Ok(AraSonuc {
            kayitlar,
            siteli,
            sayfa,
            durduruldu,
        })
    }

    /// Geocoding: il/ilçe merkezi ve kapsama yarıçapı (metre).
    pub fn geocode(&self, il: &str, ilce: &str) -> Result<(f64, f64, u32), String> {
        match kota_karari(self.gunluk_limit, self.aylik_limit, self.manuel_onay) {
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
        let _ucus = ucus();
        let adres = if ilce.trim().is_empty() {
            format!("{il}, Türkiye")
        } else {
            format!("{ilce}, {il}, Türkiye")
        };
        let yanit = self
            .rt
            .block_on(geocode_istek(&self.http, &self.key, &adres))?;
        let ilk = yanit
            .results
            .first()
            .ok_or_else(|| format!("'{adres}' için konum bulunamadı."))?;
        let enlem = ilk.geometry.location.lat;
        let boylam = ilk.geometry.location.lng;
        if enlem.abs() > 90.0 || boylam.abs() > 180.0 {
            return Err("Google konum yanıtı geçersiz koordinat içeriyor.".to_string());
        }
        // Sınır kutusundan yarıçap: ilçe için birkaç km, il için onlarca km.
        let yaricap = ilk
            .geometry
            .viewport
            .as_ref()
            .map(|kutu| {
                let d_enlem = (kutu.northeast.lat - kutu.southwest.lat).abs() * 111_320.0;
                let d_boylam = (kutu.northeast.lng - kutu.southwest.lng).abs()
                    * 111_320.0
                    * enlem.to_radians().cos().abs().max(0.2);
                (d_enlem.max(d_boylam) / 2.0) as u32
            })
            .unwrap_or(if ilce.trim().is_empty() {
                25_000
            } else {
                6_000
            })
            .clamp(2_000, 90_000);
        Ok((enlem, boylam, yaricap))
    }

}

/// Kayıtlı anahtarı tek istekle sınar (Geocoding). Arayüzdeki "Test Et".
pub fn anahtar_testi(key: &str) -> Result<String, String> {
    let istemci = Istemci::yeni(
        key.trim().to_string(),
        VARSAYILAN_GUNLUK,
        VARSAYILAN_AYLIK,
        1,
        false,
    )?;
    let (enlem, boylam, _) = istemci.geocode("İstanbul", "")?;
    Ok(format!(
        "Anahtar geçerli · Geocoding yanıtı {enlem:.4}, {boylam:.4}"
    ))
}

// ---------------------------------------------------------------------------
// İstekler
// ---------------------------------------------------------------------------

fn arama_govdesi(
    il: &str,
    ilce: &str,
    sektor: &str,
    sayfa_jetonu: Option<&str>,
) -> serde_json::Value {
    let metin = if ilce.trim().is_empty() {
        format!("{sektor} {il}")
    } else {
        format!("{sektor} {ilce} {il}")
    };
    let mut govde = serde_json::json!({
        "textQuery": metin,
        "languageCode": "tr",
        "regionCode": "TR",
        "pageSize": 20,
    });
    if let Some(jeton) = sayfa_jetonu {
        govde["pageToken"] = serde_json::Value::String(jeton.to_string());
    }
    govde
}

async fn places_istek(
    http: &reqwest::Client,
    key: &str,
    govde: &serde_json::Value,
) -> Result<Yanit, String> {
    let cevap = http
        .post(PLACES_URL)
        .header("X-Goog-Api-Key", key)
        .header("X-Goog-FieldMask", ALAN_MASKESI)
        .json(govde)
        .send()
        .await
        .map_err(|hata| format!("Google Places isteği gönderilemedi: {hata}"))?;
    let kod = cevap.status();
    let metin = cevap
        .text()
        .await
        .map_err(|hata| format!("Google yanıtı okunamadı: {hata}"))?;
    if !kod.is_success() {
        return Err(format!(
            "Google Places {}: {}",
            kod.as_u16(),
            hata_metni(&metin)
        ));
    }
    serde_json::from_str(&metin).map_err(|hata| format!("Google yanıtı çözümlenemedi: {hata}"))
}

#[derive(Debug, Default, serde::Deserialize)]
struct GeoYanit {
    #[serde(default)]
    status: String,
    #[serde(rename = "error_message", default)]
    error_message: Option<String>,
    #[serde(default)]
    results: Vec<GeoSonuc>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct GeoSonuc {
    #[serde(default)]
    geometry: GeoGeometri,
}

#[derive(Debug, Default, serde::Deserialize)]
struct GeoGeometri {
    #[serde(default)]
    location: GeoNokta,
    #[serde(default)]
    viewport: Option<GeoKutu>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct GeoNokta {
    #[serde(default)]
    lat: f64,
    #[serde(default)]
    lng: f64,
}

#[derive(Debug, Default, serde::Deserialize)]
struct GeoKutu {
    #[serde(default)]
    northeast: GeoNokta,
    #[serde(default)]
    southwest: GeoNokta,
}

async fn geocode_istek(http: &reqwest::Client, key: &str, adres: &str) -> Result<GeoYanit, String> {
    let cevap = http
        .get(GEOCODE_URL)
        .query(&[
            ("address", adres),
            ("key", key),
            ("language", "tr"),
            ("region", "tr"),
        ])
        .send()
        .await
        .map_err(|hata| format!("Google Geocoding isteği gönderilemedi: {hata}"))?;
    let kod = cevap.status();
    let metin = cevap
        .text()
        .await
        .map_err(|hata| format!("Geocoding yanıtı okunamadı: {hata}"))?;
    if !kod.is_success() {
        return Err(format!(
            "Google Geocoding {}: {}",
            kod.as_u16(),
            hata_metni(&metin)
        ));
    }
    let yanit: GeoYanit = serde_json::from_str(&metin)
        .map_err(|hata| format!("Geocoding yanıtı çözümlenemedi: {hata}"))?;
    if yanit.status != "OK" {
        let neden = yanit
            .error_message
            .unwrap_or_else(|| format!("durum {}", yanit.status));
        return Err(format!("Google Geocoding: {neden}"));
    }
    Ok(yanit)
}

/// Google hata gövdesinden okunur cümleyi çıkarır.
fn hata_metni(metin: &str) -> String {
    serde_json::from_str::<serde_json::Value>(metin)
        .ok()
        .and_then(|deger| {
            deger
                .pointer("/error/message")
                .or_else(|| deger.get("error_message"))
                .and_then(|mesaj| mesaj.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| metin.chars().take(180).collect())
}

// ---------------------------------------------------------------------------
// Ayrıştırma ve sitesiz filtre
// ---------------------------------------------------------------------------

/// Ham yanıtı temiz kayıtlara çevirir. Web sitesi alanı dolu olan işletme
/// **mutlak** elenir; sayaçta kaç kaydın elendiği döner.
fn isle(yanit: &Yanit) -> Result<(Vec<BizKayit>, usize), String> {
    isle_ile(yanit, true)
}

/// `site_ele = false`: web sitesi olan kayıtlar da döner. Tek işletme
/// aramasında işletmenin sitesi olsa da koordinatı gerekir.
fn isle_ile(yanit: &Yanit, site_ele: bool) -> Result<(Vec<BizKayit>, usize), String> {
    let mut kayitlar = Vec::new();
    let mut siteli = 0usize;
    for yer in &yanit.places {
        let ad = yer
            .display_name
            .as_ref()
            .map(|ad| ad.text.trim())
            .unwrap_or("");
        if ad.is_empty() {
            continue;
        }
        if yer.business_status.as_deref() == Some("CLOSED_PERMANENTLY") {
            continue;
        }
        let site = yer.website_uri.as_deref().unwrap_or("").trim();
        if !site.is_empty() {
            siteli += 1;
            if site_ele {
                continue;
            }
        }
        let telefon = yer
            .national_phone_number
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        let adres = yer
            .formatted_address
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        let saatler = yer
            .regular_opening_hours
            .as_ref()
            .map(|saatler| saatleri_osm(&saatler.weekday_descriptions))
            .unwrap_or_default();
        let konum = yer
            .location
            .as_ref()
            .map(|konum| (konum.latitude, konum.longitude))
            .filter(|(enlem, boylam)| {
                enlem.abs() <= 90.0 && boylam.abs() <= 180.0 && (*enlem != 0.0 || *boylam != 0.0)
            });
        let kaynak = if yer.id.is_empty() {
            "https://www.google.com/maps".to_string()
        } else {
            format!("https://www.google.com/maps/place/?q=place_id:{}", yer.id)
        };
        let mut doluluk = 0u8;
        for dolu in [
            !adres.is_empty(),
            !telefon.is_empty(),
            !saatler.is_empty(),
            yer.rating.is_some(),
            yer.user_rating_count.is_some(),
            konum.is_some(),
        ] {
            if dolu {
                doluluk += 1;
            }
        }
        kayitlar.push(BizKayit {
            ad: ad.to_string(),
            adres,
            telefon,
            kaynak,
            konum,
            saatler,
            yildiz: yer.rating.map(|puan| puan as f32),
            inceleme: yer.user_rating_count,
            doluluk: (doluluk * 10 / 6).min(10),
        });
    }
    Ok((kayitlar, siteli))
}

/// 100 üzerinden SEO puanı: yalnızca kayıttaki gerçek alanlardan hesaplanır,
/// uydurma veri eklenmez. Web sitesi olmaması bu ekranın hedefidir (+25).
pub fn seo_puani(
    adres: &str,
    telefon: &str,
    saatler: &str,
    yildiz: Option<f32>,
    inceleme: Option<u32>,
) -> u8 {
    let mut puan: u16 = 25;
    if !telefon.trim().is_empty() {
        puan += 20;
    }
    if adres.contains("No:") || adres.contains("No.") {
        puan += 10;
    } else if adres.contains(',') {
        puan += 6;
    }
    if !saatler.trim().is_empty() {
        puan += 12;
    }
    puan += match inceleme.unwrap_or(0) {
        50.. => 18,
        10..=49 => 12,
        1..=9 => 6,
        _ => 0,
    };
    if let Some(yildiz) = yildiz {
        puan += if yildiz >= 4.3 {
            15
        } else if yildiz >= 3.5 {
            10
        } else {
            4
        };
    }
    puan.min(100) as u8
}

/// Google'ın "Monday: 9:00 AM – 6:00 PM" gün açıklamalarını OSM
/// `opening_hours` biçimine çevirir (Mo-Fr 09:00-18:00; Sa 10:00-14:00).
fn saatleri_osm(gunler: &[String]) -> String {
    const KODLAR: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    let mut araliklar: Vec<Option<String>> = vec![None; 7];
    for (sira, satir) in gunler.iter().enumerate().take(7) {
        let Some((_, deger)) = satir.split_once(':') else {
            continue;
        };
        let deger = deger.trim();
        let kucuk = deger.to_lowercase();
        if kucuk.contains("closed") {
            continue;
        }
        if kucuk.contains("24 hours") || kucuk.contains("open 24") {
            araliklar[sira] = Some("00:00-24:00".to_string());
            continue;
        }
        let parcalar: Vec<&str> = deger.split(['–', '-']).map(str::trim).collect();
        if parcalar.len() != 2 {
            continue;
        }
        let (Some(bas), Some(son)) = (saat_cevir(parcalar[0]), saat_cevir(parcalar[1])) else {
            continue;
        };
        let son = if son == "00:00" {
            "24:00".to_string()
        } else {
            son
        };
        araliklar[sira] = Some(format!("{bas}-{son}"));
    }
    if araliklar.iter().all(Option::is_none) {
        return String::new();
    }
    if araliklar
        .iter()
        .all(|aralik| aralik.as_deref() == Some("00:00-24:00"))
    {
        return "24/7".to_string();
    }
    let mut parcalar = Vec::new();
    let mut gun = 0usize;
    while gun < 7 {
        let deger = araliklar[gun].clone();
        let mut son = gun;
        while son + 1 < 7 && araliklar[son + 1] == deger {
            son += 1;
        }
        let kod = if gun == son {
            KODLAR[gun].to_string()
        } else {
            format!("{}-{}", KODLAR[gun], KODLAR[son])
        };
        parcalar.push(match deger {
            Some(aralik) => format!("{kod} {aralik}"),
            None => format!("{kod} off"),
        });
        gun = son + 1;
    }
    parcalar.join("; ")
}

/// "9:00 AM" / "6:00 PM" → "09:00" / "18:00".
fn saat_cevir(metin: &str) -> Option<String> {
    let metin = metin.trim();
    let (govde, ogleden_sonra) = if let Some(kalan) = metin
        .strip_suffix("AM")
        .or_else(|| metin.strip_suffix("am"))
        .or_else(|| metin.strip_suffix("A.M."))
    {
        (kalan.trim(), false)
    } else {
        let kalan = metin
            .strip_suffix("PM")
            .or_else(|| metin.strip_suffix("pm"))
            .or_else(|| metin.strip_suffix("P.M."))?;
        (kalan.trim(), true)
    };
    let (saat_kismi, dakika_kismi) = govde.split_once(':')?;
    let mut saat: u32 = saat_kismi.trim().parse().ok()?;
    let dakika: u32 = dakika_kismi.trim().parse().ok()?;
    if saat > 12 || dakika > 59 {
        return None;
    }
    if ogleden_sonra && saat < 12 {
        saat += 12;
    }
    if !ogleden_sonra && saat == 12 {
        saat = 0;
    }
    Some(format!("{saat:02}:{dakika:02}"))
}

#[cfg(test)]
mod testler {
    use super::*;

    const ORNEK: &str = r#"{
        "places": [
            {"id":"p1","displayName":{"text":"Sitesiz Berber"},"formattedAddress":"Cumhuriyet Mah. No: 5, Şehzadeler, Manisa","location":{"latitude":38.61,"longitude":27.43},"nationalPhoneNumber":"+90 532 000 00 01","regularOpeningHours":{"weekdayDescriptions":["Monday: 9:00 AM – 7:00 PM","Tuesday: 9:00 AM – 7:00 PM","Sunday: Closed"]},"rating":4.6,"userRatingCount":42,"businessStatus":"OPERATIONAL"},
            {"id":"p2","displayName":{"text":"Siteli Kebapçı"},"formattedAddress":"Manisa","websiteUri":"https://ornek.com","nationalPhoneNumber":"+90 532 000 00 02"},
            {"id":"p3","displayName":{"text":"Kapanmış Dükkân"},"businessStatus":"CLOSED_PERMANENTLY"},
            {"id":"p4","displayName":{"text":"Instagram Berberi"},"websiteUri":"https://instagram.com/ornek","nationalPhoneNumber":"+90 532 000 00 03"}
        ],
        "nextPageToken": "sonraki"
    }"#;

    #[test]
    fn siteli_kayitlar_mutlak_elenir() {
        let yanit: Yanit = serde_json::from_str(ORNEK).expect("örnek çözümlenemedi");
        let (kayitlar, siteli) = isle(&yanit).expect("ayrıştırma başarısız");
        assert_eq!(kayitlar.len(), 1, "yalnızca sitesiz kayıt kalmalı");
        assert_eq!(siteli, 2, "iki web siteli kayıt sayılmalı");
        assert_eq!(kayitlar[0].ad, "Sitesiz Berber");
        assert_eq!(kayitlar[0].telefon, "+90 532 000 00 01");
        assert_eq!(kayitlar[0].inceleme, Some(42));
        assert_eq!(yanit.next_page_token.as_deref(), Some("sonraki"));
    }

    #[test]
    fn calisma_saatleri_osm_bicimine_cevrilir() {
        let gunler = vec![
            "Monday: 9:00 AM – 7:00 PM".to_string(),
            "Tuesday: 9:00 AM – 7:00 PM".to_string(),
            "Wednesday: 9:00 AM – 7:00 PM".to_string(),
            "Thursday: 9:00 AM – 7:00 PM".to_string(),
            "Friday: 9:00 AM – 7:00 PM".to_string(),
            "Saturday: 10:00 AM – 2:00 PM".to_string(),
            "Sunday: Closed".to_string(),
        ];
        let metin = saatleri_osm(&gunler);
        assert!(metin.contains("Mo-Fr 09:00-19:00"), "{metin}");
        assert!(metin.contains("Sa 10:00-14:00"), "{metin}");
        assert!(metin.contains("Su off"), "{metin}");
    }

    #[test]
    fn kota_gunluk_ve_aylik_siniri_korur() {
        let mut kota = Kota::default();
        assert_eq!(kota.karar("2026-11-03", "2026-11", 2, 3, true), Karar::Izin);
        assert_eq!(kota.karar("2026-11-03", "2026-11", 2, 3, true), Karar::Izin);
        assert_eq!(
            kota.karar("2026-11-03", "2026-11", 2, 3, true),
            Karar::GunlukDolu
        );
        // Yeni gün: günlük sıfırlanır, aylık devam eder.
        assert_eq!(kota.karar("2026-11-04", "2026-11", 2, 3, true), Karar::Izin);
        assert_eq!(
            kota.karar("2026-11-04", "2026-11", 2, 3, true),
            Karar::AylikDolu
        );
        assert_eq!(kota.toplam, 3);
        // Yeni ay: aylık da sıfırlanır.
        assert_eq!(kota.karar("2026-12-01", "2026-12", 2, 3, true), Karar::Izin);
    }

    #[test]
    fn kota_alarm_esigi_80_yuzde() {
        let mut kota = Kota::default();
        for _ in 0..8 {
            kota.karar("2026-11-03", "2026-11", 10, 100, true);
        }
        let ozet = kota.ozet(10, 100);
        assert_eq!(ozet.seviye, Seviye::Sari, "günlük %80 sarı alarm");
        kota.karar("2026-11-03", "2026-11", 10, 100, true);
        kota.karar("2026-11-03", "2026-11", 10, 100, true);
        assert_eq!(kota.ozet(10, 100).seviye, Seviye::Kirmizi);
    }

    #[test]
    fn seo_puani_canli_kaydi_oder() {
        let zayif = seo_puani("", "", "", None, None);
        let guclu = seo_puani(
            "Cumhuriyet Mah. No: 5, Manisa",
            "+90 532 000 00 01",
            "Mo-Fr 09:00-19:00",
            Some(4.6),
            Some(42),
        );
        assert_eq!(zayif, 25, "web sitesi yokluğu tek başına 25 puan");
        assert!(guclu > 80, "tam kayıt 80 üzeri olmalı: {guclu}");
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
    fn bulut_senkron_proje_yoksa_net_hata_verir() {
        let hata = bulut_senkron("AIzaSyA-ornek-anahtar-000000000000000000", "   ")
            .expect_err("proje kimliği olmadan senkron başlamamalı");
        assert!(hata.contains("proje kimliği"), "{hata}");
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
                "resource": {"labels": {"service": "places.googleapis.com"}},
                "points": [
                    {"interval":{"startTime":"2026-11-03T09:00:00Z"},"value":{"int64Value":"7"}},
                    {"interval":{"startTime":"2026-11-03T08:00:00Z"},"value":{"int64Value":"3"}},
                    {"interval":{"startTime":"2026-11-02T22:00:00Z"},"value":{"doubleValue":5.0}}
                ]
            }]
        }"#;
        let yanit: BulutYanit = serde_json::from_str(metin).expect("metrik çözümlenemedi");
        let (gunluk, aylik, places, geocode) = bulut_topla(&yanit, "2026-11-03");
        assert_eq!(gunluk, 10, "bugünün saatlik kovaları toplanmalı");
        assert_eq!(aylik, 15, "ayın tüm kovaları toplanmalı");
        assert_eq!(places, 15, "servis etiketi Places olarak ayrılmalı");
        assert_eq!(geocode, 0, "Geocoding serisi yokken sıfır kalmalı");
    }

    /// Canlı doğrulama: gerçek anahtar ve ağ ile Geocoding + Places.
    /// `cargo test -- --ignored canli` ile elle çalıştırılır; 2 istek harcar.
    #[test]
    #[ignore = "canlı Google API isteği harcar"]
    fn canli_geocode_ve_arama() {
        let kasa = crate::anahtar::Kasa::yukle();
        let Some(anahtar) = kasa.anahtar().map(str::to_string) else {
            eprintln!("canlı test atlandı: kayıtlı anahtar yok");
            return;
        };
        let istemci = Istemci::yeni(anahtar, 5, 50, 1, false).expect("istemci kurulamadı");
        match istemci.geocode("Manisa", "Şehzadeler") {
            Ok((enlem, boylam, yaricap)) => {
                println!("canlı geocode: {enlem:.4}, {boylam:.4} · {yaricap} m")
            }
            Err(hata) => println!("canlı geocode HATA: {hata}"),
        }
        match istemci.ara("Manisa", "Şehzadeler", "kuaför") {
            Ok(sonuc) => {
                println!(
                    "canlı arama: {} sitesiz · {} siteli elendi · {} sayfa · engel {:?}",
                    sonuc.kayitlar.len(),
                    sonuc.siteli,
                    sonuc.sayfa,
                    sonuc.durduruldu
                );
                for kayit in sonuc.kayitlar.iter().take(3) {
                    println!("  - {} · tel: {}", kayit.ad, kayit.telefon);
                }
            }
            Err(hata) => println!("canlı arama HATA: {hata}"),
        }
    }
}
