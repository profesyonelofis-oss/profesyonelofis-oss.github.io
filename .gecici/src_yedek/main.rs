#![windows_subsystem = "windows"]

use chrono::{Datelike, Timelike};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

mod ai;
mod ai_ui;
mod anahtar;
mod demo_site;
mod domain;
mod gelen;
mod google;
mod hedef;
mod iller;
mod katman34;
mod katmanlar;
mod medya_arsiv;
mod medya_dogrulama;
mod medya_havuz;
mod medya_islem;
mod medya_manuel;
mod medya_onay;
#[cfg(target_os = "windows")]
mod medya_tarayici;
mod medya_tedarik;
mod medya_web;
mod musteriler;
mod ogrenme;
mod saglik;
mod sektor_medya;
mod sektor_tasarim;
mod stok_kasa;
mod stok_video;
mod tasarim_brifi;
mod tedarik_servis;
mod uretim_pipeline;
mod yerel_ai;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct BusinessLead {
    name: String,
    address: String,
    category: String,
    phone: String,
    selected: bool,
    kaynak: String,
    konum: Option<(f64, f64)>,
    calisma_saatleri: String,
    demo_yolu: Option<String>,
    mesaj_hazir: bool,
    /// Kayıttaki ek bilgi miktarı (0-10): mutfak türü, marka, engelli erişimi
    /// gibi puanlanmayan etiketler. İşletmenin gerçekten var olan ve birileri
    /// tarafından takip edilen bir yer olduğunun işareti olarak kullanılır.
    kayit_dolulugu: u8,
    /// Kayıtta görülen web adresi. Bu ekran yalnızca web sitesi olmayan
    /// işletmeleri listelediği için yeni kayıtlarda boş kalır; eski önbellek
    /// kayıtlarındaki değer yalnızca kaydın elenmesi için okunur.
    site_linki: String,
    /// Kaydın son düzenlenmesinden bu yana geçen gün (tazelik; eski kayıtlarda dolu).
    son_guncelleme_gun: Option<i64>,
    /// İnceleme sayısı (harici kaynaklı eski kayıtlarda): müşteri trafiği sinyali.
    inceleme_sayisi: Option<u32>,
    /// Ortalama yıldız puanı (0-5). Yalnızca harici kaynaklı eski kayıtlarda
    /// bulunur; OSM kayıtlarında yoktur. Karnede "4,2 ★ · 37 yorum"
    /// olarak gösterilir.
    #[serde(default)]
    yildiz_puani: Option<f32>,
    /// Kalıcı defterdeki iletişim durumu (yeni / gönderildi / kara liste...).
    durum: musteriler::Durum,
    /// Demo GitHub Pages'e yayınlandıysa paylaşılabilir adresi.
    #[serde(default)]
    yayin_linki: Option<String>,
    /// Aynı yayındaki dijital karnenin adresi (karne üretilemediyse boş).
    #[serde(default)]
    karne_linki: Option<String>,
    /// Defterden gelen mesaj varyantı (A/B denemesi). Gönderimde bu kullanılır.
    #[serde(default)]
    mesaj_varyanti: Option<musteriler::Varyant>,
    /// İşletmenin fotoğrafı: demo sayfasının hero sahnesinde kullanılır.
    #[serde(default)]
    foto: Option<Foto>,
    /// Galeri için ek fotoğraflar: hero'da kullanılan kare burada yoktur.
    #[serde(default)]
    fotolar: Vec<Foto>,
    /// "2 Medya" adımında bu işletme için seçilen stok dosyaları. Yollar
    /// proje köküne göredir ve müşteri defteriyle senkron tutulur.
    #[serde(default)]
    medya: Vec<String>,
}

/// Demo için kullanılacak işletme fotoğrafının kaynağı.
///
/// `url` üç biçimde olabilir:
///   (işletmenin kendi fotoğrafı; API anahtarı indirme anında eklenir).
/// - `commons:<Dosya_Adı.jpg>` — Wikimedia Commons; yazar/lisans bilgisi
///   API'den çekilip altbilgiye yazılır.
/// - `https://...` — kayıttaki doğrudan görsel adresi.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct Foto {
    url: String,
    /// Altbilgide gösterilecek atıf; Commons'ta indirme anında doldurulur.
    #[serde(default)]
    atif: String,
}

/// Önizlenen mesaj. `anahtar`, önizlemenin hangi işletmeye ait olduğunu
/// sabitler: liste yeniden taranır ya da sıralanırsa indeks kayar, ama
/// anahtar tutmuyorsa önizleme kapatılır ve yanlış işletmeye mesaj gitmez.
#[derive(Clone)]
struct Onizleme {
    idx: usize,
    anahtar: String,
    mesaj: String,
}

/// Önizleme hâlâ aynı işletmeye mi ait?
fn onizleme_gecerli(onizleme: &Onizleme, lead: &BusinessLead) -> bool {
    musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum) == onizleme.anahtar
}

/// Sonuç listesinin süzgeci. "Takip" günlük çalışma listesidir: mesaj
/// gönderilmiş, üzerinden `TAKIP_GUN` gün geçmiş ve hâlâ yanıt bekleyenler.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListeSuzgeci {
    Hepsi,
    Yeni,
    Takip,
}

/// Üst şeritteki iki çalışma alanı: Müşteri Bulma ve WhatsApp.
/// SEO, harita, önizleme ve SSR katmanları koddan kaldırıldı.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Katman {
    MusteriBulma,
    WhatsApp,
}

impl Katman {
    const HEPSI: [Katman; 2] = [Katman::MusteriBulma, Katman::WhatsApp];

    /// Sekme başlığı: rakam yok, yalnız çalışma alanının adı.
    fn etiket(self) -> &'static str {
        match self {
            Katman::MusteriBulma => "Müşteri Bulma",
            Katman::WhatsApp => "WhatsApp",
        }
    }

    fn kisa(self) -> &'static str {
        self.etiket()
    }
}

/// "2 Medya" adımında yüklenebilecek dosya türü.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MedyaTuru {
    Foto,
    Video,
}

impl MedyaTuru {
    fn baslik(self) -> &'static str {
        match self {
            MedyaTuru::Foto => "Fotoğraf",
            MedyaTuru::Video => "Video",
        }
    }

    fn alt_baslik(self) -> &'static str {
        match self {
            MedyaTuru::Foto => "JPEG · PNG · WebP",
            MedyaTuru::Video => "MP4 · MOV · WebM",
        }
    }

    fn kabul(self, yol: &std::path::Path) -> bool {
        let uzanti = yol
            .extension()
            .and_then(|uzanti| uzanti.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match self {
            MedyaTuru::Foto => matches!(uzanti.as_str(), "jpg" | "jpeg" | "png" | "webp"),
            MedyaTuru::Video => matches!(uzanti.as_str(), "mp4" | "mov" | "webm"),
        }
    }

    fn uzantilar(self) -> &'static [&'static str] {
        match self {
            MedyaTuru::Foto => &["jpg", "jpeg", "png", "webp"],
            MedyaTuru::Video => &["mp4", "mov", "webm"],
        }
    }
}

/// Mesaj gönderildikten kaç gün sonra takip listesine düşer.
const TAKIP_GUN: i64 = 3;

/// Bir günde hazırlanabilecek en fazla WhatsApp mesajı.
///
/// Mesajlar elle gönderilse de aynı numaradan arka arkaya 20-30 sohbet
/// açmak hesabı spam olarak işaretletir. Fren defterde tutulur: uygulama
/// kapatılıp açılsa da günlük sayaç sıfırlanmaz.
const GUNLUK_GONDERIM_LIMITI: usize = 20;

/// İki `YYYY-MM-DD` tarihi arasındaki gün farkı.
fn gun_farki(eski: &str, yeni: &str) -> Option<i64> {
    let once = chrono::NaiveDate::parse_from_str(eski, "%Y-%m-%d").ok()?;
    let sonra = chrono::NaiveDate::parse_from_str(yeni, "%Y-%m-%d").ok()?;
    Some((sonra - once).num_days())
}

/// Arka plandaki tarama/gönderim işinin arayüze gönderdiği mesajlar.
enum Mesaj {
    Log(String),
    /// İl geneli taramasında tamamlanan ilçe sayısı ve toplam ilçe.
    Ilerleme {
        tamamlanan: usize,
        toplam: usize,
    },
    TaramaBitti(Vec<BusinessLead>),
    TaramaBasarisiz,
}

/// Google Cloud metriklerini 60 saniyede bir arka planda senkronlar.
/// Anahtar ve proje kimliği her turda yeniden okunur: kullanıcı panelden
/// değiştirdiğinde döngü kendiliğinden yeni değerlerle devam eder.
fn bulut_dongusu_baslat() {
    let (uyandir, bekle) = std::sync::mpsc::channel();
    google::senkron_uyandirici(uyandir);
    std::thread::spawn(move || loop {
        let anahtar = anahtar::Kasa::yukle().anahtar().map(str::to_string);
        let proje = ayar_oku().google_proje;
        if let Some(anahtar) = anahtar {
            if !proje.trim().is_empty() {
                match google::bulut_senkron(&anahtar, &proje) {
                    Ok(sonuc) => google::bulut_yaz(&sonuc),
                    Err(hata) => google::bulut_hata_yaz(&hata),
                }
            }
        }
        // Yeni API isteği ya da 60 saniye: hangisi önce gelirse tazelenir.
        let _ = bekle.recv_timeout(Duration::from_secs(google::BULUT_SENKRON_SN));
    });
}

pub struct LeadApp {
    cities: Vec<(&'static str, Vec<&'static str>)>,
    sectors: Vec<&'static str>,

    selected_city_idx: usize,
    selected_district_idx: usize,
    selected_sector_idx: usize,

    city_search: String,
    district_search: String,
    sector_search: String,
    sonuc_arama: String,

    leads: Vec<BusinessLead>,
    /// Sonuç listesinin süzgeci (tümü / yalnızca yeni / takip bekleyenler).
    liste_suzgeci: ListeSuzgeci,
    /// Kalıcı müşteri defteri: durumlar, kara liste ve mükerrer engeli.
    defter: musteriler::Defter,

    whatsapp_phone: String,
    whatsapp_status: String,
    is_whatsapp_connected: bool,
    whatsapp_acik: bool,
    detay_idx: Option<usize>,
    /// Gönderim öncesi önizleme.
    onizleme: Option<Onizleme>,
    /// Takip kuyruğu paneli (yanıt bekleyen, zamanı gelmiş işletmeler).
    takip_panel_acik: bool,
    /// Bugünün gönderim listesi: günlük hakkı aşmadan sırayla yazılacaklar.
    bugun_panel_acik: bool,
    /// Günlük 20 hedefli lead listesi (diske yazılır, gün değişince sıfırlanır).
    hedef: hedef::HedefListesi,
    /// Sektör seçim paneli açık mı ve sol sütunda hangi grup seçili.
    sektor_panel_acik: bool,
    sektor_panel_grup: usize,
    /// Renklendirme paneli: demo renklerini kullanıcı seçer.
    tema_panel_acik: bool,
    /// Hafıza paneli: serbest revizyon notu, kayıt listesi ve temizleme.
    hafiza_not: String,
    hafiza_panel: bool,
    hafiza_liste: Vec<ogrenme::Kayit>,
    hafiza_temizle_onay: bool,
    /// Boş metin "otomatik" demektir (vurgu = sektör rengi, zemin/metin = varsayılan).
    tema_vurgu: String,
    tema_zemin: String,
    tema_yazi: String,
    /// Bölüm görünürlüğü ve metin/köşe özelleştirmesi.
    tema_galeri: bool,
    tema_yorumlar: bool,
    tema_harita: bool,
    tema_saatler: bool,
    tema_slogan: String,
    tema_cta: String,
    tema_ikincil: String,
    tema_kose: String,
    /// Yerleşim: boşsa klasik, "gece" ise tek ekranlık kompozisyon.
    tema_tema: String,
    /// Üretim motoru: "ajans" beş bölümlü sinematik akış, "klasik" eski şablon,
    /// "sinematik" sticky sahneli 3D akış.
    tema_profil: String,
    /// Sinematik sahne ayarları: hareket, nesne ve sahne sayısı.
    tema_sinematik_hareket: String,
    tema_sinematik_nesne: String,
    tema_sinematik_sahne: usize,
    /// Stok video bulıcu: seçili sektör grubu, düzenlenebilen üç İngilizce
    /// arama kelimesi, aday listesi ve arka plandaki işin kanalı.
    /// Müşteri fotoğrafından Ken Burns döngüsü için dosya yolu.
    stok_foto_yol: String,
    /// Stok video bulucu (yerel içe aktarma): seçili grup, dosya yolu,
    /// kırpma başlangıcı, künye alanları ve arka plan işinin kanalı.
    stok_grup: String,
    stok_yol: String,
    stok_baslangic: String,
    stok_linki: String,
    stok_cekim: String,
    stok_lisans: String,
    stok_atif_goster: bool,
    stok_durum: String,
    stok_kutuphane: Vec<stok_video::VideoBilgi>,
    stok_islem: Option<Receiver<StokOlay>>,
    stok_mesgul: bool,
    /// Panelden yazılan başlık, paragraf, liste ve yorum metinleri.
    tema_manset1: String,
    tema_manset2: String,
    tema_hakkinda_baslik: String,
    tema_hakkinda1: String,
    tema_hakkinda2: String,
    tema_bolum_etiket: String,
    tema_bolum_baslik: String,
    tema_bolum_alt: String,
    /// Dört liste satırı (ad, açıklama) ve iki yorum (söz, isim).
    tema_kalemler: Vec<(String, String)>,
    tema_yorum_metinleri: Vec<(String, String)>,
    /// Fotoğraf seçimleri: hero kaynağı, galeri kaynağı, kare sayısı ve
    /// kendi eklediği dosyaların yolları. `tema_foto_yolu` yalnız paneldeki
    /// geçici giriş alanıdır, diske yazılmaz.
    tema_foto_yolu: String,
    tema_hero: String,
    tema_hero_dosya: String,
    tema_galeri_kaynak: String,
    tema_galeri_adet: usize,
    tema_fotolar: Vec<String>,
    /// Açıkken tarama tek ilçe yerine ilin tüm ilçelerini sırayla gezer.
    il_geneli: bool,
    /// Iki taraması boş döndüyse uyarı gösterilir; taranan ilçenin adını tutar.
    bos_ilce_uyari: Option<String>,
    /// İl geneli taramasında (tamamlanan, toplam) ilçe sayısı.
    ilerleme: Option<(usize, usize)>,

    /// Açık katman (üst şeritten ya da akış düğmesinden değişir).
    katman: Katman,
    /// Radardaki pinlerin düşmeye başladığı an (liste yenilenince sıfırlanır).
    /// Katman 1'in interaktif haritası (yerel koordinat ızgarası, ağ isteği yok).
    /// Son yerleştirilen sonuç imzası: kamera yalnız yeni sonuçta odaklanır.
    /// Haritadan pin seçildiğinde listede kaydırılıp vurgulanacak kart.
    vurgu_kart: Option<usize>,
    /// Listeden seçilen işletme: harita bu pine yumuşak geçiş yapar.
    /// Sağ paneldeki gömülü Google Maps JavaScript API (WebView2) katmanı.
    /// WebView katmanı kurulamazsa nedeni; panelde gösterilir.
    /// Haritaya işlenen son pin imzası.
    /// eframe ana penceresinin HWND'si (WebView2 alt penceresi için).
    /// Katman 3'te düzenlenen işletmenin listedeki sırası.
    onizleme_lead: Option<usize>,
    /// Katman 2 (Medya): seçili işletme, yükleme yolu ve doğrulama durumu.
    medya_lead: Option<usize>,
    medya_yol: String,
    /// Yüklenen/seçilen dosyaların küçük görsel önizlemeleri (yol → doku).
    medya_onizleme: HashMap<String, Option<egui::TextureHandle>>,
    /// 2 Medya arşivinde açık sekme: video veya fotoğraf.
    medya_arsiv_turu: MedyaTuru,
    /// Tıklanan medyanın büyük önizleme penceresi.
    medya_buyuk_onizleme: Option<String>,
    /// Arşiv taramasının son imzası: yeni/değişen dosyada önbelleği tazeler.
    medya_arsiv_imza: Option<u64>,
    /// Yükleme doğrulanınca demoyu kendiliğinden üretip Katman 3'e geç.
    medya_oto_demo: bool,
    /// Arka plan yüklemesi bittiğinde bir kez tüketilen otomatik geçiş isteği.
    medya_oto_ac: bool,
    /// Otomatik geçişin hangi işletmeye ait olduğu (yükleme sırasında işletme
    /// değişirse yanlış demoya geçilmesini önler).
    medya_oto_hedef: Option<String>,
    medya_mesaj: String,
    medya_mesgul: bool,
    medya_hedef: Option<String>,
    medya_islem: Option<Receiver<Result<String, String>>>,
    /// WebView2 gömülü üretici oturumu ve indirme köprüsü.
    #[cfg(target_os = "windows")]
    medya_tarayici: Option<medya_tarayici::Tarayici>,
    #[cfg(target_os = "windows")]
    medya_tarayici_hedef: Option<(String, String)>,
    #[cfg(target_os = "windows")]
    medya_tarayici_slug: Option<String>,
    #[cfg(target_os = "windows")]
    medya_tarayici_url: Option<String>,
    #[cfg(target_os = "windows")]
    medya_tarayici_hata_url: Option<String>,
    #[cfg(target_os = "windows")]
    medya_tarayici_rect: Option<egui::Rect>,
    #[cfg(target_os = "windows")]
    medya_tarayici_mesaj: String,
    #[cfg(target_os = "windows")]
    medya_tarayici_rx: Option<Receiver<medya_tarayici::Olay>>,
    #[cfg(target_os = "windows")]
    medya_tarayici_kuyruk: Vec<(std::path::PathBuf, String, String)>,
    /// Tarayıcı açıldığı an: İndirilenler klasörü taraması bu andan sonrasını alır.
    #[cfg(target_os = "windows")]
    medya_tarayici_baslangic: Option<SystemTime>,
    #[cfg(target_os = "windows")]
    medya_tarayici_son_tarama: Option<Instant>,
    #[cfg(target_os = "windows")]
    medya_tarayici_gorulen: HashSet<std::path::PathBuf>,
    /// Doğrulama sonrası sektör kökünden temizlenecek ham indirme.
    #[cfg(target_os = "windows")]
    medya_tarayici_temizle: Option<std::path::PathBuf>,
    /// Önizlemede 550 px metin ölçüsü ve satır yüksekliği kılavuzu.
    onizleme_kilavuz: bool,
    /// Son temiz derlemenin dosya yolu ve sayfadan silinen demo izi sayısı.
    temiz_yol: Option<String>,
    temiz_sayi: Option<usize>,
    /// Temiz derlemenin yayınlandığı adres (varsa).
    temiz_link: Option<String>,
    /// Katman 4: alan adı sorgusu.
    domain_ad: String,
    domain_uzanti: usize,
    domain_sonuc: Vec<domain::Sonuc>,
    domain_rx: Option<Receiver<Vec<domain::Sonuc>>>,
    domain_calisiyor: bool,
    /// Katman 5: gelen kutusu ve yerel köprü.
    gelen: gelen::GelenKutusu,
    gelen_tx: Sender<gelen::Olay>,
    gelen_rx: Receiver<gelen::Olay>,
    gelen_durdur: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    kopru: gelen::KopruDurumu,
    kopru_son: Option<Instant>,
    gelen_secili: Option<usize>,
    gelen_yanit: String,
    elle_form: bool,
    elle_telefon: String,
    elle_ad: String,
    elle_metin: String,
    /// Yerel model (Ollama): durum, tek uçuşta tek istek ve son taslak.
    ai_durum: ai::ModelDurum,
    ai_rx: Option<Receiver<ai::Sonuc>>,
    ai_son: Option<Instant>,
    ai_taslak: Option<String>,
    /// Niyeti modelle zaten denenmiş mesajlar (aynı mesaj iki kez sorulmaz).
    ai_denenen: HashSet<String>,
    /// Tek tuşla üretim hattı: onaylı stoktan 1 video + 2 fotoğraf seçip
    /// demo üretimini kullanıcıya dosya yolu sormadan başlatır.
    pipeline_aktif: bool,
    pipeline_mesaj: String,
    /// 2 Medya · yerel AI fotoğraf üretimi: Python + CUDA servisi ve üretim
    /// durumu. Çıktı doğrudan `media/stock/<sektör>/` altına yazılır.
    yerel_ai: yerel_ai::YerelAi,
    /// Tek tuş hattı video beklemez: eksik fotoğraflar yerel AI'da üretilirken
    /// hedef işletmenin anahtarı burada tutulur; bitince temizlenir.
    pipeline_foto_hedef: Option<String>,
    /// Arşivden silinmek üzere onay bekleyen dosya (hızlı güvenlik adımı).
    medya_sil_onay: Option<String>,
    /// Panelde yapılan son değişiklikten sonra demo dosyasını yeniden yazmak
    /// için kısa debounce; önizleme her karede anında güncellenir.
    onizleme_degisim: Option<Instant>,
    /// Otomatik demo denemeleri arasındaki bekleme (hata döngüsünü önler).
    pipeline_son_deneme: Option<Instant>,
    /// Açılış denetimi: ana ekran açılmadan önce kritik bileşenler sınanır.
    baslangic: saglik::Denetim,
    /// API & Kota Yönetim penceresi: anahtar girişi, test ve canlı sayaç.
    api_panel: bool,
    api_girdi: String,
    api_mesaj: String,
    api_test: Option<Receiver<Result<String, String>>>,
    /// Panelde düzenlenen istek sınırları (ayarlar.json'a yazılır).
    api_gunluk: usize,
    api_aylik: usize,
    api_sayfa: usize,
    /// Google Cloud proje kimliği: gerçek kullanım metriği bu projeden okunur.
    api_proje: String,
    /// Manuel "Şimdi senkronla" isteğinin sonucu.
    api_bulut: Option<Receiver<Result<google::BulutOzet, String>>>,
    /// Güvenlik marjı onayı: yalnızca kullanıcı düğmeye basınca true olur,
    /// tarama başlarken tüketilir (tek kullanımlık).
    kota_onay: bool,
    logs: Vec<String>,
    /// İşlem günlüğü tek satırlık kompakt akış şeridinde mi?
    gunluk_kompakt: bool,
    is_running: bool,
    log_rx: Receiver<Mesaj>,
    log_tx: Sender<Mesaj>,
}

impl Default for LeadApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        // Katman 5'in köprüsü: dinleyici ilk açılışta başlar, olayları bu
        // kanaldan arayüze bırakır.
        let (gelen_tx, gelen_rx) = gelen::kanal();
        let gelen_kutusu = gelen::GelenKutusu::yukle();

        let cities: Vec<(&'static str, Vec<&'static str>)> = iller::ILLER
            .iter()
            .map(|(il, ilceler)| (*il, ilceler.to_vec()))
            .collect();
        // Varsayılan konum kullanıcının evi: Manisa · Şehzadeler.
        let varsayilan_il = cities
            .iter()
            .position(|(il, _)| *il == "Manisa")
            .unwrap_or(0);
        let varsayilan_ilce = cities[varsayilan_il]
            .1
            .iter()
            .position(|ilce| *ilce == "Şehzadeler")
            .unwrap_or(0);

        let sectors: Vec<&'static str> = SEKTORLER.iter().map(|sektor| sektor.ad).collect();

        let ayar = ayar_oku();
        let (whatsapp_phone, whatsapp_status, whatsapp_bagli) = whatsapp_baslangic(&ayar);
        bulut_dongusu_baslat();

        Self {
            cities,
            sectors,
            liste_suzgeci: ListeSuzgeci::Hepsi,
            defter: musteriler::Defter::yukle(),
            selected_city_idx: varsayilan_il, // Manisa (kullanıcının evi)
            selected_district_idx: varsayilan_ilce,
            selected_sector_idx: 0,

            city_search: "".to_string(),
            district_search: "".to_string(),
            sector_search: "".to_string(),
            sonuc_arama: "".to_string(),

            leads: vec![],

            whatsapp_phone,
            whatsapp_status,
            is_whatsapp_connected: whatsapp_bagli,
            whatsapp_acik: false,
            detay_idx: None,
            onizleme: None,
            takip_panel_acik: false,
            bugun_panel_acik: false,
            hedef: hedef::HedefListesi::yukle(),
            sektor_panel_acik: false,
            sektor_panel_grup: 0,
            tema_panel_acik: false,
            hafiza_not: String::new(),
            hafiza_panel: false,
            hafiza_liste: Vec::new(),
            hafiza_temizle_onay: false,
            tema_vurgu: ayar.demo_vurgu.clone(),
            tema_zemin: ayar.demo_zemin.clone(),
            tema_yazi: ayar.demo_yazi.clone(),
            tema_galeri: ayar.demo_galeri,
            tema_yorumlar: ayar.demo_yorumlar,
            tema_harita: ayar.demo_harita,
            tema_saatler: ayar.demo_saatler,
            tema_slogan: ayar.demo_slogan.clone(),
            tema_cta: ayar.demo_cta.clone(),
            tema_ikincil: ayar.demo_ikincil.clone(),
            tema_kose: ayar.demo_kose.clone(),
            tema_tema: ayar.demo_tema.clone(),
            tema_profil: ayar.demo_profil.clone(),
            tema_sinematik_hareket: ayar.demo_sinematik_hareket.clone(),
            tema_sinematik_nesne: ayar.demo_sinematik_nesne.clone(),
            tema_sinematik_sahne: ayar.demo_sinematik_sahne.clamp(3, 6),
            stok_foto_yol: String::new(),
            stok_grup: stok_video::GRUPLAR[0].id.to_string(),
            stok_yol: String::new(),
            stok_baslangic: "0".to_string(),
            stok_linki: String::new(),
            stok_cekim: String::new(),
            stok_lisans: String::new(),
            stok_atif_goster: false,
            stok_durum: String::new(),
            stok_kutuphane: stok_video::kutuphane(),
            stok_islem: None,
            stok_mesgul: false,
            tema_manset1: ayar.demo_manset1.clone(),
            tema_manset2: ayar.demo_manset2.clone(),
            tema_hakkinda_baslik: ayar.demo_hakkinda_baslik.clone(),
            tema_hakkinda1: ayar.demo_hakkinda1.clone(),
            tema_hakkinda2: ayar.demo_hakkinda2.clone(),
            tema_bolum_etiket: ayar.demo_bolum_etiket.clone(),
            tema_bolum_baslik: ayar.demo_bolum_baslik.clone(),
            tema_bolum_alt: ayar.demo_bolum_alt.clone(),
            tema_kalemler: satirlari_tamamla(ayar.demo_kalemler.clone(), 4),
            tema_yorum_metinleri: satirlari_tamamla(ayar.demo_yorum_metinleri.clone(), 2),
            tema_foto_yolu: String::new(),
            tema_hero: ayar.demo_hero_foto.clone(),
            tema_hero_dosya: ayar.demo_hero_dosya.clone(),
            tema_galeri_kaynak: ayar.demo_galeri_kaynak.clone(),
            tema_galeri_adet: ayar.demo_galeri_adet.clamp(1, GALERI_AZAMI),
            tema_fotolar: ayar.demo_fotolar.clone(),
            il_geneli: false,
            bos_ilce_uyari: None,
            ilerleme: None,

            katman: Katman::MusteriBulma,
            vurgu_kart: None,
            api_panel: false,
            api_girdi: String::new(),
            api_mesaj: String::new(),
            api_test: None,
            api_gunluk: ayar.google_gunluk_limit,
            api_aylik: ayar.google_aylik_limit,
            api_sayfa: ayar.google_sayfa,
            api_proje: ayar.google_proje.clone(),
            api_bulut: None,
            kota_onay: false,
            onizleme_lead: None,
            medya_lead: None,
            medya_yol: String::new(),
            medya_onizleme: HashMap::new(),
            medya_arsiv_turu: MedyaTuru::Video,
            medya_buyuk_onizleme: None,
            medya_arsiv_imza: None,
            medya_oto_demo: true,
            medya_oto_ac: false,
            medya_oto_hedef: None,
            medya_mesaj: String::new(),
            medya_mesgul: false,
            medya_hedef: None,
            medya_islem: None,
            yerel_ai: yerel_ai::YerelAi::yeni(),
            pipeline_foto_hedef: None,
            medya_sil_onay: None,
            #[cfg(target_os = "windows")]
            medya_tarayici: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_hedef: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_slug: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_url: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_hata_url: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_rect: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_mesaj: String::new(),
            #[cfg(target_os = "windows")]
            medya_tarayici_rx: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_kuyruk: Vec::new(),
            #[cfg(target_os = "windows")]
            medya_tarayici_baslangic: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_son_tarama: None,
            #[cfg(target_os = "windows")]
            medya_tarayici_gorulen: HashSet::new(),
            #[cfg(target_os = "windows")]
            medya_tarayici_temizle: None,
            onizleme_kilavuz: false,
            temiz_yol: None,
            temiz_sayi: None,
            temiz_link: None,
            domain_ad: String::new(),
            domain_uzanti: 0,
            domain_sonuc: Vec::new(),
            domain_rx: None,
            domain_calisiyor: false,
            kopru: gelen::KopruDurumu {
                port: gelen_kutusu.port(),
                ..Default::default()
            },
            kopru_son: None,
            gelen_secili: None,
            gelen_yanit: String::new(),
            elle_form: false,
            elle_telefon: String::new(),
            elle_ad: String::new(),
            elle_metin: String::new(),
            ai_durum: ai::ModelDurum {
                url: ai::url_ayari(),
                mesaj: "henüz yoklanmadı".to_string(),
                ..Default::default()
            },
            ai_rx: None,
            ai_son: None,
            ai_taslak: None,
            ai_denenen: HashSet::new(),
            pipeline_aktif: false,
            pipeline_mesaj: String::new(),
            onizleme_degisim: None,
            pipeline_son_deneme: None,
            baslangic: saglik::Denetim::default(),
            gelen: gelen_kutusu,
            gelen_tx,
            gelen_rx,
            gelen_durdur: None,
            logs: vec!["[Sistem] Arama çubuklu veritabanı yüklendi.".to_string()],
            gunluk_kompakt: false,

            is_running: false,
            log_rx: rx,
            log_tx: tx,
        }
    }
}

// ---------------------------------------------------------------------------
// Tasarım sistemi
//
// Beyaz zemin, koyu yeşil vurgu ve ince kenarlar. Tüm ölçüler burada
// tanımlıdır; arayüzün geri kalanı bu sabitleri kullanır.
// ---------------------------------------------------------------------------

const ARKA_PLAN: egui::Color32 = egui::Color32::from_rgb(245, 248, 245);
const PANEL: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const KART: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const KART_SECILI: egui::Color32 = egui::Color32::from_rgb(235, 243, 237);
const GIRINTI: egui::Color32 = egui::Color32::from_rgb(243, 247, 244);
const KENAR: egui::Color32 = egui::Color32::from_rgb(221, 230, 223);
const KENAR_GUCLU: egui::Color32 = egui::Color32::from_rgb(183, 204, 191);
const METIN: egui::Color32 = egui::Color32::from_rgb(18, 37, 27);
const SOLUK: egui::Color32 = egui::Color32::from_rgb(79, 100, 88);
const DIM: egui::Color32 = egui::Color32::from_rgb(110, 132, 119);
const VURGU: egui::Color32 = egui::Color32::from_rgb(20, 102, 61);
const VURGU_ACIK: egui::Color32 = egui::Color32::from_rgb(30, 130, 79);
const YESIL: egui::Color32 = egui::Color32::from_rgb(28, 122, 74);
const KIRMIZI: egui::Color32 = egui::Color32::from_rgb(168, 52, 44);
const MAVI: egui::Color32 = egui::Color32::from_rgb(42, 91, 134);
const KOYU_YAZI: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
/// Kota azalırken kullanılan uyarı tonu (kırmızıya yaklaşmadan).
const TURUNCU: egui::Color32 = egui::Color32::from_rgb(176, 106, 20);

const YUVARLAK_KART: f32 = 14.0;
const YUVARLAK_KUCUK: f32 = 9.0;

fn karistir(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let kanal = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgba_premultiplied(
        kanal(a.r(), b.r()),
        kanal(a.g(), b.g()),
        kanal(a.b(), b.b()),
        kanal(a.a(), b.a()),
    )
}

/// Açık temada yumuşak dolgu: rengi beyaza doğru seyreltir. Koyu temadaki
/// `gamma_multiply` karşılığıdır (0.12 ≈ çok açık bir zemin tonu).
fn ton(renk: egui::Color32, oran: f32) -> egui::Color32 {
    karistir(egui::Color32::WHITE, renk, oran.clamp(0.0, 1.0))
}

/// Beyaz zemin üzerinde okunacak kadar koyulaştırılmış renk (pill yazıları).
fn koyu(renk: egui::Color32, oran: f32) -> egui::Color32 {
    karistir(renk, METIN, oran.clamp(0.0, 1.0))
}

fn sektor_rengi(sektor: &str) -> egui::Color32 {
    if sektor.contains("Restoran") || sektor.contains("Kafe") {
        egui::Color32::from_rgb(255, 172, 92)
    } else if sektor.contains("Otel") {
        egui::Color32::from_rgb(126, 186, 255)
    } else if sektor.contains("Diş") || sektor.contains("Sağlık") {
        egui::Color32::from_rgb(94, 226, 191)
    } else if sektor.contains("Kuaför") || sektor.contains("Güzellik") {
        egui::Color32::from_rgb(246, 149, 200)
    } else if sektor.contains("Oto") {
        egui::Color32::from_rgb(168, 182, 206)
    } else if sektor.contains("Yazılım") || sektor.contains("Bilişim") {
        egui::Color32::from_rgb(167, 139, 250)
    } else if sektor.contains("Spor") {
        egui::Color32::from_rgb(255, 138, 128)
    } else {
        egui::Color32::from_rgb(180, 196, 255)
    }
}

/// `Color32` → `#rrggbb`. Demo sayfasında CSS değişkeni olarak kullanılır.
fn hex_renk(renk: egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", renk.r(), renk.g(), renk.b())
}

/// Demo başlığının altındaki tek cümle. Yalnızca sektörün bölüm adından
/// kurulur; işletme hakkında doğrulanmamış iddia ("en iyi", "taze malzeme",
/// "uzman kadro") yazılmaz.
fn govde_font(boyut: f32) -> egui::FontId {
    egui::FontId::new(boyut, egui::FontFamily::Proportional)
}

fn baslik_font(boyut: f32) -> egui::FontId {
    egui::FontId::new(boyut, egui::FontFamily::Name("Baslik".into()))
}

fn kod_font(boyut: f32) -> egui::FontId {
    egui::FontId::new(boyut, egui::FontFamily::Monospace)
}

/// Arama için Türkçe karakterleri sadeleştirir: "İstanbul" ile "istanbul",
/// "Iğdır" ile "igdir" aynı sonucu verir.
fn arama_anahtari(metin: &str) -> String {
    metin
        .chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' | 'i' => 'i',
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'Ö' | 'ö' => 'o',
            'Ş' | 'ş' => 's',
            'Ü' | 'ü' => 'u',
            'Â' | 'â' => 'a',
            'Î' | 'î' => 'i',
            'Û' | 'û' => 'u',
            diger => diger.to_ascii_lowercase(),
        })
        .collect()
}

fn metin_genisligi(ui: &egui::Ui, metin: &str, boyut: f32) -> f32 {
    ui.painter()
        .layout_no_wrap(metin.to_owned(), baslik_font(boyut), METIN)
        .size()
        .x
}

/// Windows'un sistem yazı tiplerini yükler: gövde için Segoe UI, başlıklar
/// için Segoe UI Semibold/Bold, günlük için Consolas. Herhangi biri yoksa
/// egui'nin gömülü yazı tipine düşer.
fn yazi_tipi_yukle(ctx: &egui::Context) {
    fn dosyadan(fonts: &mut egui::FontDefinitions, ad: &str, yol: &str) -> bool {
        match std::fs::read(yol) {
            Ok(veri) => {
                fonts
                    .font_data
                    .insert(ad.to_owned(), egui::FontData::from_owned(veri));
                true
            }
            Err(_) => false,
        }
    }

    let mut fonts = egui::FontDefinitions::default();

    let govde = dosyadan(&mut fonts, "seg_govde", r"C:\Windows\Fonts\segoeui.ttf");
    let yari = dosyadan(&mut fonts, "seg_yari", r"C:\Windows\Fonts\seguisb.ttf");
    let kalin = dosyadan(&mut fonts, "seg_kalin", r"C:\Windows\Fonts\segoeuib.ttf");
    let kod = dosyadan(&mut fonts, "consolas", r"C:\Windows\Fonts\consola.ttf");
    // Segoe UI Symbol: ✓ ✕ ★ gibi işaretler Segoe UI'de yok; yedek olarak eklenir.
    let sembol = dosyadan(&mut fonts, "seg_sembol", r"C:\Windows\Fonts\seguisym.ttf");
    // Önizlemedeki serif başlıklar: demo sayfasının Playfair'ine en yakın
    // Windows yazı tipi. Yoksa orantılı aileye düşer, çizim bozulmaz.
    let serif = dosyadan(&mut fonts, "serif_baslik", r"C:\Windows\Fonts\georgia.ttf");

    let varsayilan = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let mut orantili = varsayilan.clone();
    if govde {
        orantili.insert(0, "seg_govde".to_owned());
    }
    if sembol {
        orantili.push("seg_sembol".to_owned());
    }
    fonts
        .families
        .insert(egui::FontFamily::Proportional, orantili);

    let mut basliklar: Vec<String> = Vec::new();
    if kalin {
        basliklar.push("seg_kalin".to_owned());
    }
    if yari {
        basliklar.push("seg_yari".to_owned());
    }
    basliklar.extend(varsayilan);
    if sembol {
        basliklar.push("seg_sembol".to_owned());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("Baslik".into()), basliklar);

    let mut mono = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    if kod {
        mono.insert(0, "consolas".to_owned());
    }
    if sembol {
        mono.push("seg_sembol".to_owned());
    }
    fonts.families.insert(egui::FontFamily::Monospace, mono);

    let mut serif_aile: Vec<String> = Vec::new();
    if serif {
        serif_aile.push("serif_baslik".to_owned());
    }
    serif_aile.extend(
        fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name("Serif".into()), serif_aile);

    ctx.set_fonts(fonts);
}

fn tema_uygula(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::light();

    visuals.panel_fill = ARKA_PLAN;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = GIRINTI;
    visuals.faint_bg_color = PANEL;
    visuals.code_bg_color = GIRINTI;
    visuals.override_text_color = Some(METIN);
    visuals.hyperlink_color = MAVI;
    visuals.window_stroke = egui::Stroke::new(1.0_f32, KENAR);
    visuals.selection.bg_fill = ton(VURGU, 0.20);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, VURGU);
    visuals.window_rounding = egui::Rounding::same(YUVARLAK_KART);
    visuals.menu_rounding = egui::Rounding::same(10.0);

    let kenarli =
        |w: &mut egui::style::WidgetVisuals, dolgu: egui::Color32, kenar: egui::Color32| {
            w.bg_fill = dolgu;
            w.weak_bg_fill = dolgu;
            w.bg_stroke = egui::Stroke::new(1.0_f32, kenar);
            w.rounding = egui::Rounding::same(YUVARLAK_KUCUK);
            w.expansion = 0.0;
        };

    kenarli(&mut visuals.widgets.noninteractive, PANEL, KENAR);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, METIN);
    kenarli(&mut visuals.widgets.inactive, KART, KENAR);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, METIN);
    kenarli(&mut visuals.widgets.hovered, KART_SECILI, KENAR_GUCLU);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, METIN);
    kenarli(&mut visuals.widgets.active, VURGU, VURGU);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5_f32, KOYU_YAZI);
    kenarli(&mut visuals.widgets.open, KART, KENAR_GUCLU);
    // Kaydırma çubuğu tutamacı `widgets.inactive.bg_fill` rengini kullanır;
    // varsayılan buton çizimini kullanmadığımız için bu rengi burada seçiyoruz.
    visuals.widgets.inactive.bg_fill = ton(METIN, 0.13);
    visuals.widgets.inactive.weak_bg_fill = KART;

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.floating_width = 3.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.foreground_color = false;

    style.text_styles = [
        (egui::TextStyle::Heading, baslik_font(18.0)),
        (egui::TextStyle::Body, govde_font(13.0)),
        (egui::TextStyle::Button, govde_font(13.0)),
        (egui::TextStyle::Small, govde_font(11.0)),
        (egui::TextStyle::Monospace, kod_font(11.5)),
    ]
    .into();

    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// Ortak arayüz parçaları
// ---------------------------------------------------------------------------

/// Küçük, harf aralıklı bölüm etiketi ("FİLTRELER", "SONUÇLAR").
fn bolum_etiketi(ui: &mut egui::Ui, metin: &str) {
    ui.label(
        egui::RichText::new(metin)
            .font(baslik_font(10.0))
            .color(DIM),
    );
}

fn pill(ui: &mut egui::Ui, metin: &str, renk: egui::Color32) {
    egui::Frame::none()
        .fill(ton(renk, 0.14))
        .stroke(egui::Stroke::new(1.0_f32, ton(renk, 0.40)))
        .rounding(egui::Rounding::same(99.0))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(metin)
                    .size(10.0)
                    .color(koyu(renk, 0.45)),
            );
        });
}

/// Açılış denetimi satırı: işaret, başlık ve sade Türkçe açıklama.
fn denetim_satiri(ui: &mut egui::Ui, madde: &saglik::Madde) {
    ui.horizontal_top(|ui| {
        ui.label(
            egui::RichText::new(madde.durum.isaret())
                .size(15.0)
                .strong()
                .color(madde.durum.renk()),
        );
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(madde.ad)
                    .size(13.0)
                    .strong()
                    .color(METIN),
            );
            ui.label(egui::RichText::new(&madde.aciklama).size(11.5).color(SOLUK));
        });
        if madde.durum == saglik::Durum::Calisiyor {
            ui.add(egui::Spinner::new().size(12.0).color(VURGU));
        }
    });
}

/// Dolu (ana aksiyon) buton. `boy` verilmezse metne göre boyutlanır.
fn birincil_buton(
    ui: &mut egui::Ui,
    metin: &str,
    etkin: bool,
    boy: Option<egui::Vec2>,
) -> egui::Response {
    let boy = boy.unwrap_or_else(|| egui::vec2(metin_genisligi(ui, metin, 12.0) + 32.0, 30.0));
    let (rect, yanit) = ui.allocate_exact_size(
        boy,
        if etkin {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, etkin && yanit.hovered(), 0.12);
    let dolgu = if etkin {
        karistir(VURGU, VURGU_ACIK, t)
    } else {
        KART
    };
    let kenar = if etkin {
        karistir(VURGU, VURGU_ACIK, t)
    } else {
        KENAR
    };
    let yazi = if etkin { KOYU_YAZI } else { DIM };
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(YUVARLAK_KUCUK), dolgu);
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(YUVARLAK_KUCUK),
        egui::Stroke::new(1.0_f32, kenar),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        metin,
        baslik_font(12.0),
        yazi,
    );
    yanit
}

/// Çerçeveli ikincil buton.
fn ikincil_buton(
    ui: &mut egui::Ui,
    metin: &str,
    etkin: bool,
    boy: Option<egui::Vec2>,
) -> egui::Response {
    let boy = boy.unwrap_or_else(|| egui::vec2(metin_genisligi(ui, metin, 11.5) + 28.0, 30.0));
    let (rect, yanit) = ui.allocate_exact_size(
        boy,
        if etkin {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, etkin && yanit.hovered(), 0.12);
    let dolgu = karistir(egui::Color32::TRANSPARENT, KART_SECILI, t);
    let kenar = karistir(KENAR, KENAR_GUCLU, t);
    let yazi = if !etkin {
        DIM
    } else {
        karistir(SOLUK, METIN, t)
    };
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(YUVARLAK_KUCUK), dolgu);
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(YUVARLAK_KUCUK),
        egui::Stroke::new(1.0_f32, kenar),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        metin,
        govde_font(11.5),
        yazi,
    );
    yanit
}

/// Aç/kapa çipi: açıkken dolu ve işaretli, kapalıyken çerçeveli.
/// Tıklandıysa `true` döner.
fn secim_cipi(ui: &mut egui::Ui, metin: &str, acik: &mut bool) -> bool {
    let etiket = if *acik {
        format!("✓ {metin}")
    } else {
        metin.to_string()
    };
    let buton = egui::Button::new(egui::RichText::new(etiket).size(10.5).color(if *acik {
        KOYU_YAZI
    } else {
        SOLUK
    }))
    .fill(if *acik { VURGU } else { KART })
    .stroke(egui::Stroke::new(
        1.0_f32,
        if *acik { VURGU } else { KENAR },
    ))
    .rounding(egui::Rounding::same(999.0))
    .min_size(egui::vec2(0.0, 24.0));
    if ui.add(buton).clicked() {
        *acik = !*acik;
        return true;
    }
    false
}

/// Dolgusuz, düşük vurgulu buton (Temizle, Tümünü seç...).
fn hayalet_buton(ui: &mut egui::Ui, metin: &str) -> egui::Response {
    let boy = egui::vec2(metin_genisligi(ui, metin, 11.5) + 20.0, 28.0);
    let (rect, yanit) = ui.allocate_exact_size(boy, egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered(), 0.12);
    if t > 0.01 {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(8.0),
            karistir(egui::Color32::TRANSPARENT, KART, t),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        metin,
        govde_font(11.5),
        karistir(DIM, METIN, t),
    );
    yanit
}

/// Panelde tek satırlık metin alanı; değiştiyse `true` döner.
fn metin_kutusu(ui: &mut egui::Ui, metin: &mut String, ipucu: &str) -> bool {
    egui::Frame::none()
        .fill(GIRINTI)
        .stroke(egui::Stroke::new(1.0_f32, KENAR))
        .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(metin)
                    .frame(false)
                    .hint_text(egui::RichText::new(ipucu).size(11.5).color(DIM))
                    .font(govde_font(12.0))
                    .desired_width(f32::INFINITY),
            )
        })
        .inner
        .changed()
}

/// Panelde etiketli metin alanı: `satir` > 1 ise çok satırlı paragraf,
/// değilse tek satır (başlıkta satır atlanamaz). Değiştiyse `true` döner.
fn metin_alani(
    ui: &mut egui::Ui,
    etiket: &str,
    deger: &mut String,
    ipucu: &str,
    satir: usize,
) -> bool {
    ui.label(egui::RichText::new(etiket).size(10.0).color(SOLUK));
    let duzen = if satir <= 1 {
        egui::TextEdit::singleline(deger)
    } else {
        egui::TextEdit::multiline(deger).desired_rows(satir)
    };
    let degisti = egui::Frame::none()
        .fill(GIRINTI)
        .stroke(egui::Stroke::new(1.0_f32, KENAR))
        .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
        .inner_margin(egui::Margin::symmetric(9.0, 6.0))
        .show(ui, |ui| {
            ui.add(
                duzen
                    .frame(false)
                    .hint_text(egui::RichText::new(ipucu).size(11.0).color(DIM))
                    .font(govde_font(12.0))
                    .desired_width(f32::INFINITY),
            )
        })
        .inner
        .changed();
    ui.add_space(2.0);
    degisti
}

/// Tek seçimli çip grubu: seçilen değeri `deger`e yazar.
/// Seçim gerçekten değiştiyse `true` döner.
fn secenek_cipleri(ui: &mut egui::Ui, deger: &mut String, secenekler: &[(&str, &str)]) -> bool {
    let mut degisti = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (anahtar, ad) in secenekler {
            let secili = deger.as_str() == *anahtar;
            let metin =
                egui::RichText::new(*ad)
                    .size(10.0)
                    .color(if secili { KOYU_YAZI } else { METIN });
            let buton = egui::Button::new(metin)
                .fill(if secili { VURGU } else { KART })
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    if secili { VURGU } else { KENAR },
                ))
                .rounding(egui::Rounding::same(999.0))
                .min_size(egui::vec2(0.0, 23.0));
            if ui.add(buton).clicked() && !secili {
                *deger = anahtar.to_string();
                degisti = true;
            }
        }
    });
    degisti
}

/// Yoldan yalnız dosya adı: panelde uzun yollar taşmasın.
fn dosya_adi(yol: &str) -> String {
    yol.rsplit(['/', '\\']).next().unwrap_or(yol).to_string()
}

/// Etiketli, çerçeveli arama alanı.
fn arama_kutusu(ui: &mut egui::Ui, metin: &mut String, ipucu: &str) {
    egui::Frame::none()
        .fill(GIRINTI)
        .stroke(egui::Stroke::new(1.0_f32, KENAR))
        .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(metin)
                    .frame(false)
                    .hint_text(egui::RichText::new(ipucu).size(11.5).color(DIM))
                    .font(govde_font(12.0))
                    .desired_width(f32::INFINITY),
            );
        });
}

/// Katalogdaki grup başlıkları, listelenme sırasıyla ve tekrarsız.
fn sektor_gruplari() -> Vec<&'static str> {
    let mut gruplar: Vec<&'static str> = Vec::new();
    for tanim in SEKTORLER {
        if gruplar.last() != Some(&tanim.grup) {
            gruplar.push(tanim.grup);
        }
    }
    gruplar
}

/// Bir gruptaki sektörler, katalog sırasıyla.
fn grup_sektorleri(grup: &str) -> Vec<&'static str> {
    SEKTORLER
        .iter()
        .filter(|tanim| tanim.grup == grup)
        .map(|tanim| tanim.ad)
        .collect()
}

/// Sektörün katalogdaki sırası; panelden seçimde kullanılır.
fn sektor_sirasi(ad: &str) -> usize {
    SEKTORLER
        .iter()
        .position(|tanim| tanim.ad == ad)
        .unwrap_or(0)
}

/// Metni verilen genişliğe sığdırır, taşarsa sonuna üç nokta koyar.
fn kisalt_metin(ui: &egui::Ui, metin: &str, genislik: f32, boyut: f32) -> String {
    let olc = |parca: &str| {
        ui.painter()
            .layout_no_wrap(parca.to_owned(), govde_font(boyut), METIN)
            .size()
            .x
    };
    if olc(metin) <= genislik {
        return metin.to_string();
    }
    let mut kisa = String::new();
    for karakter in metin.chars() {
        let deneme = format!("{kisa}{karakter}…");
        if olc(&deneme) > genislik {
            break;
        }
        kisa.push(karakter);
    }
    format!("{kisa}…")
}

/// Filtre panelindeki sektör özeti; tıklanınca seçim paneli açılır.
fn sektor_secici(ui: &mut egui::Ui, secili: &str) -> egui::Response {
    let genislik = ui.available_width();
    let (rect, yanit) = ui.allocate_exact_size(egui::vec2(genislik, 42.0), egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered(), 0.10);
    let p = ui.painter();
    p.rect_filled(
        rect,
        egui::Rounding::same(YUVARLAK_KUCUK),
        karistir(GIRINTI, KART_SECILI, t * 0.55),
    );
    p.rect_stroke(
        rect,
        egui::Rounding::same(YUVARLAK_KUCUK),
        egui::Stroke::new(1.0_f32, karistir(KENAR, KENAR_GUCLU, t)),
    );
    p.text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        secili,
        govde_font(12.5),
        METIN,
    );
    p.text(
        rect.right_center() - egui::vec2(12.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        "Değiştir",
        govde_font(10.5),
        if yanit.hovered() { VURGU } else { SOLUK },
    );
    if yanit.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    yanit
}

/// Panelin sol sütunundaki grup satırı: ad, sektör sayısı, seçili vurgusu.
fn grup_satiri(ui: &mut egui::Ui, ad: &str, sayi: usize, aktif: bool) -> egui::Response {
    let genislik = ui.available_width();
    let (rect, yanit) = ui.allocate_exact_size(egui::vec2(genislik, 23.0), egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered(), 0.10);
    if aktif {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(7.0), ton(VURGU, 0.12));
    } else if t > 0.0 {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(7.0), ton(KENAR, t));
    }
    let etiket = kisalt_metin(ui, ad, genislik - 32.0, 11.0);
    ui.painter().text(
        rect.left_center() + egui::vec2(9.0, 0.0),
        egui::Align2::LEFT_CENTER,
        etiket,
        govde_font(11.0),
        if aktif {
            VURGU
        } else if yanit.hovered() {
            METIN
        } else {
            SOLUK
        },
    );
    ui.painter().text(
        rect.right_center() - egui::vec2(8.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        sayi.to_string(),
        govde_font(9.5),
        DIM,
    );
    yanit
}

/// Panelin sağ sütunundaki sektör satırı; `alt` doluysa sağda grup adı yazar.
fn sektor_satiri(ui: &mut egui::Ui, ad: &str, alt: &str, secili: bool) -> egui::Response {
    let genislik = ui.available_width();
    let (rect, yanit) = ui.allocate_exact_size(egui::vec2(genislik, 24.0), egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered(), 0.10);
    if secili {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(7.0), ton(VURGU, 0.12));
        let cubuk = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(2.0, 6.0),
            egui::vec2(3.0, 12.0),
        );
        ui.painter()
            .rect_filled(cubuk, egui::Rounding::same(2.0), VURGU);
    } else if t > 0.0 {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(7.0), ton(KENAR, t * 0.9));
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(13.0, 0.0),
        egui::Align2::LEFT_CENTER,
        ad,
        govde_font(12.0),
        if secili {
            VURGU
        } else if yanit.hovered() {
            METIN
        } else {
            SOLUK
        },
    );
    if !alt.is_empty() {
        ui.painter().text(
            rect.right_center() - egui::vec2(10.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            alt,
            govde_font(9.5),
            DIM,
        );
    }
    if yanit.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    yanit
}

/// Seçilebilir liste satırı: solda ince vurgu çubuğu, yumuşak hover.
fn liste_satiri(ui: &mut egui::Ui, secili: bool, metin: &str) -> egui::Response {
    let genislik = ui.available_width();
    let (rect, yanit) = ui.allocate_exact_size(egui::vec2(genislik, 25.0), egui::Sense::click());

    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered(), 0.10);
    let dolgu = if secili {
        ton(VURGU, 0.12)
    } else {
        karistir(egui::Color32::TRANSPARENT, KENAR, t * 0.7)
    };
    if dolgu != egui::Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(7.0), dolgu);
    }
    if secili {
        let cubuk = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(2.0, 7.0),
            egui::vec2(3.0, 11.0),
        );
        ui.painter()
            .rect_filled(cubuk, egui::Rounding::same(2.0), VURGU);
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(13.0, 0.0),
        egui::Align2::LEFT_CENTER,
        metin,
        govde_font(12.0),
        if secili {
            VURGU
        } else if yanit.hovered() {
            METIN
        } else {
            SOLUK
        },
    );
    yanit
}

fn secim_dairesi(ui: &mut egui::Ui, secili: &mut bool) -> egui::Response {
    let (rect, mut yanit) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
    if yanit.clicked() {
        *secili = !*secili;
        yanit.mark_changed();
    }
    let merkez = rect.center();
    if *secili {
        ui.painter().circle_filled(merkez, 8.5, VURGU);
        let onay = vec![
            merkez + egui::vec2(-3.7, 0.2),
            merkez + egui::vec2(-0.9, 3.0),
            merkez + egui::vec2(4.0, -3.0),
        ];
        ui.painter().add(egui::Shape::line(
            onay,
            egui::Stroke::new(1.7_f32, KOYU_YAZI),
        ));
    } else {
        let renk = if yanit.hovered() { KENAR_GUCLU } else { KENAR };
        ui.painter()
            .circle_stroke(merkez, 8.5, egui::Stroke::new(1.5_f32, renk));
    }
    yanit
}

fn log_rengi(satir: &str) -> egui::Color32 {
    if satir.starts_with("[!]") {
        KIRMIZI
    } else if satir.starts_with("[✓]") {
        YESIL
    } else if satir.starts_with("[i]") {
        MAVI
    } else {
        SOLUK
    }
}

fn avatar(ui: &mut egui::Ui, isim: &str, renk: egui::Color32) {
    avatar_boyut(ui, isim, renk, 20.0);
}

fn avatar_boyut(ui: &mut egui::Ui, isim: &str, renk: egui::Color32, yaricap: f32) {
    let kenar = yaricap * 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(kenar, kenar), egui::Sense::hover());
    let merkez = rect.center();
    let p = ui.painter();
    p.circle_filled(merkez, yaricap, ton(renk, 0.20));
    p.circle_stroke(merkez, yaricap, egui::Stroke::new(1.0_f32, ton(renk, 0.38)));
    let harf = isim
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    p.text(
        merkez,
        egui::Align2::CENTER_CENTER,
        harf,
        baslik_font(yaricap * 0.8),
        koyu(renk, 0.45),
    );
}

/// Demo dosyasını üretir (yoksa) ve tarayıcıda açar.
fn demo_ac(lead: &mut BusinessLead, gonderen: &str, logs: &mut Vec<String>) {
    if lead.demo_yolu.is_none() {
        match demo_uret(lead, gonderen) {
            Ok(yol) => lead.demo_yolu = Some(yol),
            Err(hata) => logs.push(format!("[!] {hata}")),
        }
    }
    if let Some(yol) = lead.demo_yolu.clone() {
        if let Err(hata) = tarayicida_ac(&yol) {
            logs.push(format!("[!] Demo açılamadı: {hata}"));
        } else {
            logs.push(format!(
                "[i] {}: demo ile birlikte dijital karne de hazır → {}/{}",
                lead.name,
                DEMO_KLASORU,
                karne_dosya_adi(lead)
            ));
        }
    }
}

/// Hazır mesajla WhatsApp sohbetini açar; göndermeyi kullanıcı yapar.
fn whatsapp_ac(lead: &mut BusinessLead, gonderen: &str, logs: &mut Vec<String>) {
    if lead.durum.kara_listede() {
        logs.push(format!(
            "[!] {} mesaj istemedi (kara liste); WhatsApp açılmadı.",
            lead.name
        ));
        return;
    }
    let Some(numara) = whatsapp_numarasi(&lead.phone) else {
        return;
    };
    // Mesajda paylaşılabilir link olsun: yayınlanmamışsa önce demo yayınlanır.
    // Yayın başarısız olursa mesaj link olmadan açılır, gönderim engellenmez.
    if lead.yayin_linki.is_none() {
        yayina_gonder_ve_link_al(lead, gonderen, logs);
    }
    let mesaj = whatsapp_mesaji(lead);
    if sohbet_ac(&numara, &mesaj, logs) {
        lead.mesaj_hazir = true;
        logs.push(format!(
            "[✓] {} için WhatsApp sohbeti hazırlandı; mesajı siz gönderiyorsunuz.",
            lead.name
        ));
    }
}

/// wa.me bağlantısına sığan en fazla karakter sayısı. Tarayıcılar ve
/// Windows kabuğu çok uzun adresleri kestiği için mesaj bu sınırda kırpılır.
const MESAJ_SINIRI: usize = 900;

/// Hazır metni WhatsApp sohbetinde açar; açılıp açılmadığını döndürür.
fn sohbet_ac(numara: &str, mesaj: &str, logs: &mut Vec<String>) -> bool {
    let mesaj = mesaj_sinirla(mesaj);
    let url = format!("https://wa.me/{numara}?text={}", url_encode(&mesaj));
    match tarayicida_ac(&url) {
        Ok(()) => true,
        Err(hata) => {
            logs.push(format!("[!] WhatsApp açılamadı: {hata}"));
            false
        }
    }
}

/// Mesajı bağlantıya sığacak şekilde kısaltır; kesildiğini üç nokta ile belli eder.
fn mesaj_sinirla(mesaj: &str) -> String {
    let kirpilan: String = mesaj.chars().take(MESAJ_SINIRI).collect();
    if kirpilan.chars().count() < mesaj.chars().count() {
        format!("{}…", kirpilan.trim_end())
    } else {
        mesaj.to_string()
    }
}

/// Ödemesi kalan bir işletmeye gönderilecek kısa tahsilat hatırlatması.
fn odeme_hatirlatma_mesaji(kayit: &musteriler::Musteri) -> String {
    let paket = kayit.paket.map(|paket| paket.etiket()).unwrap_or("site");
    let kalan = musteriler::Defter::kalan_bakiye(kayit);
    let mut metin = format!(
        "Merhaba, {} için hazırladığımız {paket} paketinde {kalan} ₺ ödeme kaldı.",
        kayit.ad
    );
    if let Some(link) = kayit.yayin_linki.as_deref() {
        metin.push_str(&format!(" Siteniz yayında: {link}"));
    }
    metin.push_str(
        " Uygun olduğunuzda IBAN'ı iletebilirim. İstemezseniz \"RET\" yazın, bir daha yazmam.",
    );
    metin
}

/// Yanıt bekleyen bir işletmeye gönderilecek kısa hatırlatma.
fn takip_mesaji(kayit: &musteriler::Musteri) -> String {
    let gecen = kayit
        .son_gonderim
        .as_deref()
        .or(kayit.son_temas.as_deref())
        .and_then(|tarih| musteriler::gun_farki(tarih, &bugun_anahtari()))
        .unwrap_or(0)
        .max(1);
    let mut metin = format!(
        "Merhaba, {gecen} gün önce {} için hazırladığım web sitesi taslağını paylaşmıştım. İnceleme fırsatınız oldu mu?",
        kayit.ad
    );
    if let Some(link) = kayit.yayin_linki.as_deref() {
        metin.push_str(&format!(" Taslak hâlâ yayında: {link}"));
    }
    metin.push_str(
        " İsterseniz alan adınızda 2 dakikada yayına alabilirim. İstemezseniz \"RET\" yazın, bir daha yazmam.",
    );
    metin
}

/// Tutar alanındaki metni sayıya çevirir: yalnız rakamlar, boşsa `None`.
fn girdi_tutar(metin: &str) -> Option<u32> {
    let rakamlar: String = metin.chars().filter(|harf| harf.is_ascii_digit()).collect();
    if rakamlar.is_empty() {
        return None;
    }
    rakamlar.parse().ok()
}

/// Kart ve detay panelinde müşteri durumunu tek yerde etiketler.
fn durum_pili(ui: &mut egui::Ui, durum: musteriler::Durum) {
    match durum {
        musteriler::Durum::Yeni => {}
        musteriler::Durum::MesajHazir => pill(ui, "mesaj hazır", VURGU),
        musteriler::Durum::Gonderildi => pill(ui, "gönderildi", MAVI),
        musteriler::Durum::Ilgilendi => pill(ui, "ilgileniyor", YESIL),
        musteriler::Durum::Reddetti => pill(ui, "kara liste", KIRMIZI),
    }
}

fn detay_satiri(ui: &mut egui::Ui, etiket: &str, deger: &str) {
    ui.label(
        egui::RichText::new(etiket)
            .font(baslik_font(9.5))
            .color(DIM),
    );
    ui.add_space(3.0);
    ui.label(egui::RichText::new(deger).size(12.0).color(METIN));
    ui.add_space(12.0);
}

/// Kartın sağ/sol alt şeridindeki WhatsApp ve demo düğmeleri. Genişlik ne
/// olursa olsun sığdığı alanda sağa yaslanır; `true` mesaj önizlemesi ister.
fn kart_butonlari(
    ui: &mut egui::Ui,
    lead: &mut BusinessLead,
    gonderen: &str,
    logs: &mut Vec<String>,
    wa_metin: &str,
    demo_metin: &str,
    limit_doldu: bool,
) -> bool {
    let mut onizleme_iste = false;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let numara = whatsapp_numarasi(&lead.phone);
            let wa = birincil_buton(ui, wa_metin, numara.is_some() && !limit_doldu, None);
            let wa = if numara.is_none() {
                wa.on_hover_text("Bu işletmenin telefon numarası kayıtta yok")
            } else if limit_doldu {
                wa.on_hover_text(format!(
                    "Günlük gönderim hakkı doldu ({} mesaj); yarın devam edin",
                    GUNLUK_GONDERIM_LIMITI
                ))
            } else {
                wa.on_hover_text("Mesajı göndermeden önce önizlersiniz")
            };
            if wa.clicked() {
                onizleme_iste = true;
            }

            // Demo üretimi arayüzde yok; müşteri kaydı WhatsApp akışından yürür.
            let _ = (demo_metin, gonderen, logs);
        },
    );
    onizleme_iste
}

fn kart_ciz(
    ui: &mut egui::Ui,
    lead: &mut BusinessLead,
    gonderen: &str,
    logs: &mut Vec<String>,
    acik: bool,
    limit_doldu: bool,
    vurgu: bool,
) -> (bool, bool, egui::Rect) {
    let secili = lead.selected;
    let kategori_rengi = sektor_rengi(&lead.category);
    let mut detay_iste = false;
    let mut onizleme_iste = false;

    let kenar = if acik || vurgu {
        ton(MAVI, 0.80)
    } else if secili {
        ton(VURGU, 0.55)
    } else {
        KENAR
    };

    let mut ic_sol_ust = egui::Pos2::ZERO;
    let mut serit_g = 0.0_f32;
    let mut dar_mi = false;

    let ic = egui::Frame::none()
        .fill(if secili || acik || vurgu {
            KART_SECILI
        } else {
            KART
        })
        .stroke(egui::Stroke::new(
            if vurgu { 2.0_f32 } else { 1.0_f32 },
            kenar,
        ))
        .rounding(egui::Rounding::same(YUVARLAK_KART))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let genislik = ui.available_width();
            let wa_metin = if lead.mesaj_hazir {
                "Tekrar aç"
            } else {
                "WhatsApp"
            };
            let demo_metin = if lead.demo_yolu.is_some() {
                "Demoyu aç"
            } else {
                "Demo üret"
            };
            // Düğme şeridi kendi payını alır; bilgi alanı kalan genişliğe
            // sığar. Böylece rozetler ve düğmeler hiçbir genişlikte taşmaz.
            let dar = genislik < 470.0;
            dar_mi = dar;
            let buton_g = if dar {
                0.0
            } else {
                (metin_genisligi(ui, wa_metin, 12.0)
                    + 34.0
                    + 8.0
                    + metin_genisligi(ui, demo_metin, 11.5)
                    + 30.0)
                    .min(genislik * 0.5)
            };
            let bilgi_g = if dar {
                genislik
            } else {
                (genislik - buton_g - 14.0).max(140.0)
            };
            serit_g = buton_g;
            ic_sol_ust = ui.cursor().min;

            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                secim_dairesi(ui, &mut lead.selected);
                ui.add_space(10.0);
                avatar(ui, &lead.name, kategori_rengi);
                ui.add_space(13.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(bilgi_g, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(bilgi_g);
                        ui.spacing_mut().item_spacing.y = 4.0;

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let etiket_g = metin_genisligi(ui, &lead.category, 10.0) + 24.0;
                            let ad_g = (bilgi_g - etiket_g).max(70.0);
                            let ad = kisalt_metin(ui, &lead.name, ad_g, 14.5);
                            ui.label(egui::RichText::new(ad).font(baslik_font(14.5)).color(METIN));
                            pill(ui, &lead.category, kategori_rengi);
                        });

                        // Rozetler sarılır: dar sütunda alta iner, sağa taşmaz.
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.spacing_mut().item_spacing.y = 4.0;
                            pill(ui, "Web Sitesi Yok · Hedef", VURGU);
                            let seo = seo_puani(lead);
                            pill(
                                ui,
                                &format!("Profil {seo}/100"),
                                if seo >= 70 { YESIL } else { MAVI },
                            );
                            durum_pili(ui, lead.durum);
                            if potansiyel_puani(lead) >= SICAK_ESIK {
                                pill(ui, "sıcak fırsat", YESIL);
                            }
                            if whatsapp_numarasi(&lead.phone).is_none() {
                                pill(ui, "telefon yok", KIRMIZI);
                            } else if !mobil_numara(&lead.phone) {
                                pill(ui, "sabit hat", MAVI);
                            }
                        });

                        ui.label(
                            egui::RichText::new(kisalt_metin(
                                ui,
                                &lead.address,
                                bilgi_g - 2.0,
                                11.5,
                            ))
                            .size(11.5)
                            .color(SOLUK),
                        );

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let mut alt_satir = String::new();
                            if !lead.phone.is_empty() {
                                alt_satir.push_str(&lead.phone);
                            }
                            // Kanıt: haritadaki pinin gerçek koordinatı.
                            if let Some((enlem, boylam)) = lead.konum {
                                if !alt_satir.is_empty() {
                                    alt_satir.push_str("   ·   ");
                                }
                                alt_satir.push_str(&format!("{enlem:.5}, {boylam:.5}"));
                            }
                            if !lead.calisma_saatleri.is_empty() {
                                if !alt_satir.is_empty() {
                                    alt_satir.push_str("   ·   ");
                                }
                                alt_satir.push_str(&calisma_saatleri_metni(&lead.calisma_saatleri));
                            }
                            let baglanti_g =
                                metin_genisligi(ui, kaynak_etiketi(&lead.kaynak), 10.5) + 10.0;
                            if !alt_satir.is_empty() {
                                let metin =
                                    kisalt_metin(ui, &alt_satir, bilgi_g - baglanti_g, 11.0);
                                ui.label(egui::RichText::new(metin).size(11.0).color(DIM));
                                ui.add_space(8.0);
                            }
                            ui.hyperlink_to(
                                egui::RichText::new(kaynak_etiketi(&lead.kaynak))
                                    .size(10.5)
                                    .color(DIM),
                                &lead.kaynak,
                            );
                        });
                    },
                );

                if !dar {
                    onizleme_iste |=
                        kart_butonlari(ui, lead, gonderen, logs, wa_metin, demo_metin, limit_doldu);
                }
            });

            // Dar sütunda düğmeler alt satıra iner; hiçbir düğme kırpılmaz.
            if dar {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(62.0);
                    onizleme_iste |=
                        kart_butonlari(ui, lead, gonderen, logs, wa_metin, demo_metin, limit_doldu);
                });
            }
        });

    let rect = ic.response.rect;
    // Tıklama alanı: düğme şeridi dışında kalan gövde.
    let bilgi_rect = egui::Rect::from_min_max(
        egui::pos2(ic_sol_ust.x + 62.0, ic_sol_ust.y),
        egui::pos2(
            (rect.right() - serit_g - 10.0).max(ic_sol_ust.x + 60.0),
            rect.bottom() - if dar_mi { 44.0 } else { 0.0 },
        ),
    );
    let tiklama = ui
        .interact(
            bilgi_rect,
            ui.make_persistent_id(("kart", &lead.kaynak, &lead.name))
                .with("tikla"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if tiklama.clicked() {
        detay_iste = true;
    }

    let yanit = ui.interact(
        rect,
        ui.make_persistent_id(("kart", &lead.kaynak, &lead.name))
            .with("hover"),
        egui::Sense::hover(),
    );
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered() && !secili && !acik, 0.14);
    if t > 0.01 {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(YUVARLAK_KART),
            egui::Color32::from_black_alpha((t * 9.0) as u8),
        );
    }

    (detay_iste, onizleme_iste, rect)
}

/// Bugün yazılacak işletmeler: kara listede olmayan, telefonu olan ve bugün
/// mesaj hazırlanmamış kayıtlar. Sıra korunur (liste zaten potansiyele göre
/// sıralıdır), günlük hak kadar kesilir.
#[cfg(test)]
fn bugun_sirasi(
    leadler: &[BusinessLead],
    kalan: usize,
    bugun_gonderilen: impl Fn(&BusinessLead) -> bool,
) -> Vec<usize> {
    let mut sira: Vec<usize> = (0..leadler.len())
        .filter(|idx| {
            let lead = &leadler[*idx];
            whatsapp_numarasi(&lead.phone).is_some()
                && !lead.durum.kara_listede()
                && !bugun_gonderilen(lead)
        })
        .collect();
    sira.truncate(kalan);
    sira
}

impl LeadApp {
    /// Panel: günlük 20 hedefli lead listesi. Satırda ad, adres, telefon ve
    /// koordinat durur; "Tamamlandı" satırı listeden düşürür ve günlük
    /// sayaçtan bir mesaj harcar. Liste temizlenebilir, Google'dan 20 yeni
    /// hedef çekilebilir.
    fn bugun_listesi_paneli(&mut self, ctx: &egui::Context, gonderen: &str) {
        if !self.bugun_panel_acik {
            return;
        }
        self.hedef_listesini_doldur();
        let bugun = bugun_anahtari();
        let gonderilen = self.defter.gun_gonderim(&bugun);
        let kalan = self.hedef.kalan();
        let tamam = self.hedef.tamam_sayisi();
        let satirlar = self.hedef.aktif();
        let mut acik = true;
        let mut hazirla: Option<usize> = None;
        let mut tamamla: Option<String> = None;
        let mut liste_temizle = false;
        let mut hedef_cek = false;

        egui::Window::new("bugun_listesi")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .default_width(430.0)
            .max_height(440.0)
            .anchor(egui::Align2::LEFT_TOP, [336.0, 96.0])
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                    .stroke(egui::Stroke::new(1.0_f32, KENAR)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    bolum_etiketi(ui, "GÜNLÜK HEDEF LİSTESİ");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hayalet_buton(ui, "✕").clicked() {
                            acik = false;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Bugün {tamam}/{} tamamlandı · kalan {kalan} · mesaj {gonderilen}/{}",
                        hedef::GUNLUK_HEDEF,
                        GUNLUK_GONDERIM_LIMITI
                    ))
                    .size(10.5)
                    .color(DIM),
                );
                ui.add_space(8.0);

                if satirlar.is_empty() {
                    ui.label(
                        egui::RichText::new(if self.hedef.bos() && self.leads.is_empty() {
                            "Liste boş: önce bir bölge taraması yapın ya da Google'dan 20 hedef çekin."
                        } else {
                            "Bugünün hedefleri tamamlandı; yarın yeni liste kurulur."
                        })
                        .size(11.0)
                        .color(SOLUK),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .id_source("gunluk_hedef_listesi")
                        .max_height(360.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (sira_no, kayit) in satirlar.iter().enumerate() {
                                // Metin sütunu düğmelere yer bırakır: uzun adres
                                // sarılır, "Hazırla"/"Tamamlandı" üstüne binmez.
                                ui.horizontal_top(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:02}", sira_no + 1))
                                            .size(10.0)
                                            .color(DIM),
                                    );
                                    let dugme_g = 92.0 + 6.0 + 70.0 + 10.0;
                                    let metin_g = (ui.available_width() - dugme_g).max(140.0);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(metin_g, 0.0),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.add_space(1.0);
                                            ui.label(
                                                egui::RichText::new(&kayit.ad)
                                                    .size(12.0)
                                                    .color(METIN),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(kayit.alt_satir())
                                                        .size(9.5)
                                                        .color(SOLUK),
                                                )
                                                .wrap(true),
                                            );
                                        },
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if birincil_buton(
                                                ui,
                                                "Tamamlandı",
                                                true,
                                                Some(egui::vec2(92.0, 26.0)),
                                            )
                                            .clicked()
                                            {
                                                tamamla = Some(kayit.anahtar.clone());
                                            }
                                            if kayit.lead_idx.is_some()
                                                && ikincil_buton(
                                                    ui,
                                                    "Hazırla",
                                                    true,
                                                    Some(egui::vec2(66.0, 26.0)),
                                                )
                                                .clicked()
                                            {
                                                hazirla = kayit.lead_idx;
                                            }
                                        },
                                    );
                                });
                                ui.add_space(2.0);
                            }
                        });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ikincil_buton(
                        ui,
                        "Google'dan 20 hedef çek",
                        !self.is_running,
                        Some(egui::vec2(196.0, 26.0)),
                    )
                    .clicked()
                    {
                        hedef_cek = true;
                    }
                    if ikincil_buton(
                        ui,
                        "Listeyi temizle",
                        !satirlar.is_empty(),
                        Some(egui::vec2(126.0, 26.0)),
                    )
                    .clicked()
                    {
                        liste_temizle = true;
                    }
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Tamamlandı → satır listeden düşer ve günlük sayaçtan 1 mesaj harcanır.",
                    )
                    .size(9.5)
                    .color(DIM),
                );
            });

        if liste_temizle {
            self.hedef.temizle();
            self.hedef.kaydet();
            self.logs
                .push("[i] Günlük hedef listesi temizlendi.".to_string());
        }
        if hedef_cek {
            self.hedef_cek();
        }
        if let Some(anahtar) = tamamla {
            self.hedef_tamamla(&anahtar);
        }
        if let Some(idx) = hazirla {
            if idx < self.leads.len() {
                self.bugun_panel_acik = false;
                self.gonderime_hazirla(idx, gonderen);
            }
        }
        if !acik {
            self.bugun_panel_acik = false;
        }
    }

    /// Seçilen işletmenin ayrıntılarını ve aksiyonlarını sağ panelde gösterir.
    /// Günlük gönderim hakkı doldu mu?
    fn gunluk_limit_doldu(&self) -> bool {
        self.defter.gun_gonderim(&bugun_anahtari()) >= GUNLUK_GONDERIM_LIMITI
    }

    /// WhatsApp'ı açmadan önce yayın linkini tamamlar, mesajı üretir ve
    /// önizlemeyi açar. Gönderme kararı kullanıcının kalir.
    fn gonderime_hazirla(&mut self, idx: usize, gonderen: &str) {
        if idx >= self.leads.len() {
            return;
        }
        if self.leads[idx].durum.kara_listede() {
            self.logs.push(format!(
                "[!] {} mesaj istemedi (kara liste); WhatsApp açılmadı.",
                self.leads[idx].name
            ));
            return;
        }
        if whatsapp_numarasi(&self.leads[idx].phone).is_none() {
            self.logs.push(format!(
                "[!] {}: telefon numarası kayıtta yok, mesaj hazırlanamadı.",
                self.leads[idx].name
            ));
            return;
        }
        if !mobil_numara(&self.leads[idx].phone) {
            self.logs.push(format!(
                "[i] {}: kayıtlı numara sabit hat. WhatsApp'ta hesap çıkmazsa cep numarasını sorun.",
                self.leads[idx].name
            ));
        }
        let bugun = bugun_anahtari();
        let anahtar = musteriler::anahtar_uret(
            &self.leads[idx].kaynak,
            &self.leads[idx].name,
            self.leads[idx].konum,
        );
        // Aynı işletmeye aynı gün tekrar mesaj hazırlamak haktan düşmez.
        if !self.defter.bugun_gonderildi_mi(&anahtar, &bugun) && self.gunluk_limit_doldu() {
            self.logs.push(format!(
                "[!] Günlük gönderim hakkı doldu ({}/{}). Hesabın askıya alınmaması için yarın devam edin.",
                GUNLUK_GONDERIM_LIMITI, GUNLUK_GONDERIM_LIMITI
            ));
            return;
        }
        if self.leads[idx].yayin_linki.is_none() {
            yayina_gonder_ve_link_al(&mut self.leads[idx], gonderen, &mut self.logs);
        }
        let mesaj = whatsapp_mesaji(&self.leads[idx]);
        self.onizleme = Some(Onizleme {
            idx,
            anahtar,
            mesaj,
        });
    }

    /// Önizlemedeki mesajı WhatsApp sohbetine aktarır ve gönderimi deftere işler.
    fn gonderimi_baslat(&mut self, idx: usize, gonderen: &str) {
        if idx >= self.leads.len() {
            return;
        }
        whatsapp_ac(&mut self.leads[idx], gonderen, &mut self.logs);
        self.gonderim_kaydet(idx);
    }

    /// Panelden yapılan yayın işleminden sonra linki deftere işler; link
    /// kaldırıldıysa kayıttaki adres de temizlenir.
    fn yayin_linkini_deftere_yaz(&mut self, idx: usize) {
        if idx >= self.leads.len() {
            return;
        }
        let anahtar = musteriler::anahtar_uret(
            &self.leads[idx].kaynak,
            &self.leads[idx].name,
            self.leads[idx].konum,
        );
        match self.leads[idx].yayin_linki.clone() {
            Some(link) => self.defter.link_ayarla(&anahtar, &link),
            None => self.defter.link_temizle(&anahtar),
        }
        self.defter.kaydet();
    }

    /// Gönderimi deftere yazar; aynı işletme için günde bir kez sayılır.
    fn gonderim_kaydet(&mut self, idx: usize) {
        if idx >= self.leads.len() {
            return;
        }
        let anahtar = musteriler::anahtar_uret(
            &self.leads[idx].kaynak,
            &self.leads[idx].name,
            self.leads[idx].konum,
        );
        let bugun = bugun_anahtari();
        let onceki = self.defter.gun_gonderim(&bugun);
        self.defter.gonderim_kaydet(&anahtar, &bugun);
        // İlk kez temas kuruluyorsa durum "mesaj hazır" olarak kalıcılaşır;
        // uygulama yeniden açıldığında da kart bu bilgiyi taşır.
        if self.leads[idx].durum == musteriler::Durum::Yeni {
            self.defter
                .durum_ayarla(&anahtar, musteriler::Durum::MesajHazir, &bugun);
            self.leads[idx].durum = musteriler::Durum::MesajHazir;
            self.leads[idx].mesaj_hazir = true;
        }
        let simdiki = self.defter.gun_gonderim(&bugun);
        if simdiki == onceki {
            return;
        }
        if let Some(link) = self.leads[idx].yayin_linki.clone() {
            self.defter.link_ayarla(&anahtar, &link);
        }
        self.defter.kaydet();
        self.logs.push(format!(
            "[i] Bugünkü gönderim: {simdiki}/{GUNLUK_GONDERIM_LIMITI}"
        ));
    }

    /// Takip kuyruğu: yanıt bekleyen ve üzerinden `TAKIP_GUN` gün geçmiş
    /// işletmeler; hepsine tek tuşla hatırlatma hazırlanabilir.
    fn takip_paneli(&mut self, ctx: &egui::Context) {
        if !self.takip_panel_acik {
            return;
        }
        let bugun = bugun_anahtari();
        let kayitlar: Vec<musteriler::Musteri> = self
            .defter
            .takip_edilecekler(&bugun, TAKIP_GUN)
            .into_iter()
            .cloned()
            .collect();
        // Teklifi verilmiş ama parası gelmemiş kayıtlar ayrı kuyrukta.
        let odeme_kuyrugu: Vec<musteriler::Musteri> = self
            .defter
            .odeme_bekleyenler(&bugun, TAKIP_GUN)
            .into_iter()
            .cloned()
            .collect();
        let ekran = ctx.screen_rect();
        let boyut = egui::vec2(
            (ekran.width() - 200.0).clamp(560.0, 660.0),
            (ekran.height() - 200.0).clamp(360.0, 520.0),
        );
        let konum = ekran.center() - boyut * 0.5;
        let mut kapat = false;
        let mut acilacak: Option<String> = None;
        let mut odeme_acilacak: Option<String> = None;

        egui::Area::new(egui::Id::new("takip_ortu"))
            .order(egui::Order::Foreground)
            .fixed_pos(ekran.min)
            .show(ctx, |ui| {
                let (rect, yanit) = ui.allocate_exact_size(ekran.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(155));
                if yanit.clicked() {
                    kapat = true;
                }
            });

        egui::Area::new(egui::Id::new("takip_panel"))
            .order(egui::Order::Foreground)
            .fixed_pos(konum)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR_GUCLU))
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(boyut.x - 40.0);
                        ui.set_height(boyut.y - 32.0);
                        ui.horizontal(|ui| {
                            bolum_etiketi(ui, "TAKİP KUYRUĞU");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if hayalet_buton(ui, "Kapat").clicked() {
                                        kapat = true;
                                    }
                                },
                            );
                        });
                        ui.add_space(4.0);
                        // Huni: satışın hangi aşamada tıkandığını tek satırda gösterir.
                        let huni = self.defter.ozet();
                        let yanit = huni.ilgilenen + huni.kara_liste;
                        let ulasilan = huni.gonderilen + yanit;
                        let oran = (yanit * 100).checked_div(ulasilan).unwrap_or(0);
                        egui::Frame::none()
                            .fill(GIRINTI)
                            .stroke(egui::Stroke::new(1.0_f32, KENAR))
                            .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_width(boyut.x - 64.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Huni: {} kayıt · {} mesaj · {} yanıt (%{oran}) · {} ilgileniyor · {} kara liste · {} hazır",
                                        huni.toplam,
                                        ulasilan,
                                        yanit,
                                        huni.ilgilenen,
                                        huni.kara_liste,
                                        huni.hazir
                                    ))
                                    .size(10.5)
                                    .color(SOLUK),
                                );
                                let ciro = self.defter.ciro_ozeti();
                                if ciro.teklif_sayisi > 0 {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Ciro: {} ₺ tahsil ({} müşteri) · {} ₺ bekliyor · {} teklif",
                                            ciro.odenen, ciro.odenen_adet, ciro.bekleyen, ciro.teklif_sayisi
                                        ))
                                        .size(10.5)
                                        .color(if ciro.odenen > 0 { YESIL } else { SOLUK }),
                                    );
                                }
                            });
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} işletme {} gündür yanıt bekliyor · {} işletme tahsilat bekliyor · günlük kalan hak: {}",
                                kayitlar.len(),
                                TAKIP_GUN,
                                odeme_kuyrugu.len(),
                                GUNLUK_GONDERIM_LIMITI
                                    .saturating_sub(self.defter.gun_gonderim(&bugun))
                            ))
                            .size(10.5)
                            .color(DIM),
                        );
                        ui.add_space(8.0);
                        if kayitlar.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "Şu an takip sırası boş: gönderilen mesajların hepsi yanıtlanmış ya da henüz 3 gün dolmamış.",
                                )
                                .size(11.5)
                                .color(SOLUK),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for kayit in &kayitlar {
                                    let gecen = kayit
                                        .son_gonderim
                                        .as_deref()
                                        .or(kayit.son_temas.as_deref())
                                        .and_then(|tarih| {
                                            musteriler::gun_farki(tarih, &bugun)
                                        })
                                        .unwrap_or(0);
                                    egui::Frame::none()
                                        .fill(GIRINTI)
                                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                        .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
                                        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 3.0;
                                                    ui.label(
                                                        egui::RichText::new(&kayit.ad)
                                                            .size(12.5)
                                                            .color(METIN),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "{} · {} · {gecen} gün önce{}",
                                                            kayit.kategori,
                                                            if kayit.telefon.is_empty() {
                                                                "telefon yok"
                                                            } else {
                                                                &kayit.telefon
                                                            },
                                                            if kayit.notlar.is_empty() {
                                                                String::new()
                                                            } else {
                                                                format!(" · not: {}", kayit.notlar)
                                                            }
                                                        ))
                                                        .size(10.5)
                                                        .color(DIM),
                                                    );
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let numara =
                                                            whatsapp_numarasi(&kayit.telefon);
                                                        let dugme = ikincil_buton(
                                                            ui,
                                                            "Hatırlat",
                                                            numara.is_some(),
                                                            Some(egui::vec2(86.0, 26.0)),
                                                        );
                                                        if dugme.clicked() {
                                                            acilacak =
                                                                Some(kayit.anahtar.clone());
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    ui.add_space(6.0);
                                }

                                // --- Tahsilat kuyruğu ---------------------------
                                ui.add_space(10.0);
                                let (ayrac, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 1.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(ayrac, 0.0, KENAR);
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "ÖDEME BEKLEYEN ({})",
                                        odeme_kuyrugu.len()
                                    ))
                                    .font(baslik_font(9.5))
                                    .color(DIM),
                                );
                                ui.add_space(6.0);
                                if odeme_kuyrugu.is_empty() {
                                    ui.label(
                                        egui::RichText::new(
                                            "Tahsilat sırası boş: teklif verilen kayıtların tamamı ödenmiş ya da henüz 3 gün dolmamış.",
                                        )
                                        .size(11.0)
                                        .color(SOLUK),
                                    );
                                }
                                for kayit in &odeme_kuyrugu {
                                    let kalan = musteriler::Defter::kalan_bakiye(kayit);
                                    let paket = kayit
                                        .paket
                                        .map(|paket| paket.etiket())
                                        .unwrap_or("teklif");
                                    egui::Frame::none()
                                        .fill(GIRINTI)
                                        .stroke(egui::Stroke::new(1.0_f32, ton(VURGU, 0.45)))
                                        .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
                                        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 3.0;
                                                    ui.label(
                                                        egui::RichText::new(&kayit.ad)
                                                            .size(12.5)
                                                            .color(METIN),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "{paket} · {} ₺ kaldı · {}",
                                                            kalan,
                                                            kayit.odeme.etiket()
                                                        ))
                                                        .size(10.5)
                                                        .color(VURGU),
                                                    );
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let numara =
                                                            whatsapp_numarasi(&kayit.telefon);
                                                        let dugme = ikincil_buton(
                                                            ui,
                                                            "Tahsilat",
                                                            numara.is_some(),
                                                            Some(egui::vec2(86.0, 26.0)),
                                                        );
                                                        if dugme.clicked() {
                                                            odeme_acilacak =
                                                                Some(kayit.anahtar.clone());
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    ui.add_space(6.0);
                                }
                            });
                    });
            });

        if let Some(anahtar) = acilacak {
            self.takip_sohbet_ac(&anahtar, &bugun);
        }
        if let Some(anahtar) = odeme_acilacak {
            self.odeme_sohbet_ac(&anahtar, &bugun);
        }
        if kapat {
            self.takip_panel_acik = false;
        }
    }

    /// Tahsilat kuyruğundaki bir işletmeye ödeme hatırlatması hazırlar.
    fn odeme_sohbet_ac(&mut self, anahtar: &str, bugun: &str) {
        let Some(kayit) = self.defter.kayit(anahtar).cloned() else {
            return;
        };
        let Some(numara) = whatsapp_numarasi(&kayit.telefon) else {
            self.logs
                .push(format!("[!] {}: telefon numarası kayıtta yok.", kayit.ad));
            return;
        };
        if self.defter.gun_gonderim(bugun) >= GUNLUK_GONDERIM_LIMITI {
            self.logs.push(format!(
                "[!] Günlük gönderim hakkı doldu ({}/{}). Yarın devam edin.",
                GUNLUK_GONDERIM_LIMITI, GUNLUK_GONDERIM_LIMITI
            ));
            return;
        }
        let mesaj = odeme_hatirlatma_mesaji(&kayit);
        if sohbet_ac(&numara, &mesaj, &mut self.logs) {
            self.defter.gonderim_kaydet(anahtar, bugun);
            self.defter.kaydet();
            self.logs.push(format!(
                "[✓] {} için tahsilat hatırlatması hazır; kalan {} ₺.",
                kayit.ad,
                musteriler::Defter::kalan_bakiye(&kayit)
            ));
        }
    }

    /// Takip kuyruğundaki bir işletmeye hatırlatma mesajı hazırlar.
    fn takip_sohbet_ac(&mut self, anahtar: &str, bugun: &str) {
        let Some(kayit) = self.defter.kayit(anahtar).cloned() else {
            return;
        };
        let Some(numara) = whatsapp_numarasi(&kayit.telefon) else {
            self.logs
                .push(format!("[!] {}: telefon numarası kayıtta yok.", kayit.ad));
            return;
        };
        if self.defter.gun_gonderim(bugun) >= GUNLUK_GONDERIM_LIMITI {
            self.logs.push(format!(
                "[!] Günlük gönderim hakkı doldu ({}/{}). Yarın devam edin.",
                GUNLUK_GONDERIM_LIMITI, GUNLUK_GONDERIM_LIMITI
            ));
            return;
        }
        let mesaj = takip_mesaji(&kayit);
        if sohbet_ac(&numara, &mesaj, &mut self.logs) {
            self.defter.gonderim_kaydet(anahtar, bugun);
            self.defter.kaydet();
            self.logs.push(format!(
                "[✓] {} için takip mesajı hazır; günlük hak {}/{GUNLUK_GONDERIM_LIMITI}.",
                kayit.ad,
                self.defter.gun_gonderim(bugun)
            ));
        }
    }

    fn detay_paneli(&mut self, ctx: &egui::Context, gonderen: &str) {
        let Some(idx) = self.detay_idx else {
            return;
        };
        if idx >= self.leads.len() {
            self.detay_idx = None;
            return;
        }

        let mut kapat = false;
        let mut yeni_durum: Option<musteriler::Durum> = None;
        let mut wa_iste = false;
        let mut yayin_iste = false;
        let mut yayin_kaldir_iste = false;
        // Görüşme notu: panelde düzenlenir, değişince deftere yazılır.
        let mut not_metni = self
            .defter
            .kayit(&musteriler::anahtar_uret(
                &self.leads[idx].kaynak,
                &self.leads[idx].name,
                self.leads[idx].konum,
            ))
            .map(|kayit| kayit.notlar.clone())
            .unwrap_or_default();
        let mut not_degisti = false;
        // Defterdeki son temas tarihi panelde gösterilir (iptal öncesi okunur).
        let anahtar = musteriler::anahtar_uret(
            &self.leads[idx].kaynak,
            &self.leads[idx].name,
            self.leads[idx].konum,
        );
        let son_temas = self
            .defter
            .kayit(&anahtar)
            .and_then(|kayit| kayit.son_temas.clone());
        let son_gonderim = self
            .defter
            .kayit(&anahtar)
            .and_then(|kayit| kayit.son_gonderim.clone());
        // Teklif & ödeme: panelde seçilen paket/tutar/ödeme deftere yazılır.
        let kayit = self.defter.kayit(&anahtar);
        let mut teklif_paket = kayit.and_then(|kayit| kayit.paket);
        let mut tutar_metni = kayit
            .and_then(|kayit| kayit.teklif_tutari)
            .map(|tutar| tutar.to_string())
            .unwrap_or_default();
        let mut odeme = kayit.map(|kayit| kayit.odeme).unwrap_or_default();
        let mut tahsil_metni = kayit
            .and_then(|kayit| kayit.tahsil_edilen)
            .map(|tutar| tutar.to_string())
            .unwrap_or_default();
        let teklif_tarihi = kayit.and_then(|kayit| kayit.teklif_tarihi.clone());
        let teklif_toplam = kayit.and_then(|kayit| kayit.teklif_tutari);
        let mut teklif_degisti = false;
        let mut odeme_degisti = false;
        let mut tahsil_degisti = false;
        // A/B sayaçları panelde tek satırda gösterilir.
        let varyant_ozeti = self.defter.varyant_ozeti();
        let sektor_ozeti = self
            .defter
            .varyant_ozeti_kategori(&self.leads[idx].category);
        // 30 gönderimin altında oran farkı gürültüdür; panel bunu açıkça söyler.
        let orneklem_yetersiz = varyant_ozeti.iter().any(|ozet| ozet.gonderim > 0)
            && !varyant_ozeti.iter().all(|ozet| ozet.anlamli());
        egui::SidePanel::right("detay_paneli")
            .resizable(false)
            .exact_width(348.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
            )
            .show(ctx, |ui| {
                // Sağ panelde içerik genişliği sabit: sağ boşluk her zaman korunur.
                const IC_GENISLIK: f32 = 310.0;
                let lead = &mut self.leads[idx];
                let renk = sektor_rengi(&lead.category);

                ui.allocate_ui_with_layout(
                    egui::vec2(IC_GENISLIK, 26.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        bolum_etiketi(ui, "İŞLETME DETAYI");
                        let kalan = ui.available_width();
                        ui.add_space((kalan - 62.0).max(0.0));
                        if hayalet_buton(ui, "Kapat").clicked() {
                            kapat = true;
                        }
                    },
                );
                ui.add_space(10.0);

                // WhatsApp + demo + karne + yayın düğmeleri ve kaynak bağlantısı.
                let aksiyon_yuksekligi = 242.0;
                let kaydirma = (ui.available_height() - aksiyon_yuksekligi).max(140.0);
                egui::ScrollArea::vertical()
                    .max_height(kaydirma)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(IC_GENISLIK);

                        ui.horizontal(|ui| {
                            avatar_boyut(ui, &lead.name, renk, 26.0);
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 5.0;
                                ui.label(
                                    egui::RichText::new(&lead.name)
                                        .font(baslik_font(17.0))
                                        .color(METIN),
                                );
                                ui.horizontal(|ui| {
                                    pill(ui, &lead.category, renk);
                                    durum_pili(ui, lead.durum);
                                    if potansiyel_puani(lead) >= SICAK_ESIK {
                                        pill(ui, "sıcak fırsat", YESIL);
                                    }
                                    if whatsapp_numarasi(&lead.phone).is_none() {
                                        pill(ui, "telefon yok", KIRMIZI);
                                    } else if !mobil_numara(&lead.phone) {
                                        pill(ui, "sabit hat", MAVI);
                                    }
                                });
                            });
                        });
                        ui.add_space(14.0);

                        if let Some(durum) = whatsapp_durumu(lead) {
                            let cumle = durum_kisa_cumle(&durum);
                            let durum_rengi = if cumle.starts_with("Şu an açık") {
                                YESIL
                            } else {
                                KIRMIZI
                            };
                            egui::Frame::none()
                                .fill(ton(durum_rengi, 0.12))
                                .stroke(egui::Stroke::new(1.0_f32, ton(durum_rengi, 0.35)))
                                .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
                                .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                                .show(ui, |ui| {
                                    ui.set_width(IC_GENISLIK - 22.0);
                                    ui.horizontal(|ui| {
                                        let (nokta, _) = ui.allocate_exact_size(
                                            egui::vec2(7.0, 7.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(
                                            nokta.center(),
                                            3.0,
                                            durum_rengi,
                                        );
                                        ui.label(
                                            egui::RichText::new(cumle)
                                                .size(11.5)
                                                .color(durum_rengi),
                                        );
                                    });
                                });
                            ui.add_space(14.0);
                        }

                        detay_satiri(ui, "SATIN ALMA POTANSİYELİ", &potansiyel_gerekcesi(lead));
                        detay_satiri(ui, "MÜŞTERİ DURUMU", lead.durum.etiket());
                        // A/B: hangi varyant gitti ve yanıt durumu ne âlemde.
                        let varyant = lead.mesaj_varyanti.unwrap_or_default();
                        let yanit = match lead.durum {
                            musteriler::Durum::Yeni | musteriler::Durum::MesajHazir => {
                                "henüz gönderilmedi"
                            }
                            musteriler::Durum::Gonderildi => "yanıt bekleniyor",
                            musteriler::Durum::Ilgilendi => "olumlu yanıt",
                            musteriler::Durum::Reddetti => "olumsuz yanıt",
                        };
                        detay_satiri(
                            ui,
                            "MESAJ VARYANTI",
                            &format!("{} · {yanit}", varyant.etiket()),
                        );
                        ui.horizontal(|ui| {
                            let dugme = |ui: &mut egui::Ui, metin: &str| {
                                ikincil_buton(ui, metin, true, Some(egui::vec2(94.0, 28.0)))
                                    .clicked()
                            };
                            if dugme(ui, "Gönderdim") {
                                yeni_durum = Some(musteriler::Durum::Gonderildi);
                            }
                            if dugme(ui, "İlgileniyor") {
                                yeni_durum = Some(musteriler::Durum::Ilgilendi);
                            }
                            if dugme(ui, "Reddetti") {
                                yeni_durum = Some(musteriler::Durum::Reddetti);
                            }
                        });
                        if lead.durum.kara_listede() {
                            ui.label(
                                egui::RichText::new(
                                    "Kara listede: bu işletmeye bir daha mesaj hazırlanmaz.",
                                )
                                .size(11.0)
                                .color(KIRMIZI),
                            );
                        } else if let Some(temas) = &son_temas {
                            ui.label(
                                egui::RichText::new(format!("İlk temas: {temas}"))
                                    .size(10.5)
                                    .color(DIM),
                            );
                        }
                        if varyant_ozeti.iter().any(|ozet| ozet.gonderim > 0) {
                            let satirlar: Vec<String> = musteriler::Varyant::HEPSI
                                .iter()
                                .map(|varyant| varyant_ozeti[varyant.sira()].satir())
                                .collect();
                            ui.add_space(3.0);
                            ui.label(
                                egui::RichText::new(format!("A/B: {}", satirlar.join("  ·  ")))
                                    .size(10.5)
                                    .color(DIM),
                            );
                            if orneklem_yetersiz {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Örneklem yetersiz: kol başına {}+ gönderim gerekir.",
                                        musteriler::ANLAMLI_ORNEKLEM
                                    ))
                                    .size(10.0)
                                    .color(VURGU),
                                );
                            }
                        }
                        if sektor_ozeti.iter().any(|ozet| ozet.gonderim > 0) {
                            let satirlar: Vec<String> = musteriler::Varyant::HEPSI
                                .iter()
                                .map(|varyant| sektor_ozeti[varyant.sira()].satir())
                                .collect();
                            ui.label(
                                egui::RichText::new(format!(
                                    "A/B · {}: {}",
                                    lead.category,
                                    satirlar.join("  ·  ")
                                ))
                                .size(10.5)
                                .color(DIM),
                            );
                        }
                        if let Some(tarih) = &son_gonderim {
                            let gecen = musteriler::gun_farki(tarih, &bugun_anahtari())
                                .map(|gun| format!("{gun} gün önce"))
                                .unwrap_or_else(|| tarih.clone());
                            ui.label(
                                egui::RichText::new(format!("Son gönderim: {tarih} · {gecen}"))
                                    .size(10.5)
                                    .color(DIM),
                            );
                        }
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("GÖRÜŞME NOTU")
                                .font(baslik_font(9.5))
                                .color(DIM),
                        );
                        ui.add_space(4.0);
                        let duzenleme = egui::TextEdit::multiline(&mut not_metni)
                            .desired_rows(2)
                            .desired_width(IC_GENISLIK - 8.0)
                            .font(egui::TextStyle::Small)
                            .hint_text("Ör. fiyat sordu, pazartesi arayacak");
                        if ui.add(duzenleme).changed() {
                            not_degisti = true;
                        }
                        ui.add_space(10.0);

                        // Teklif ve tahsilat: yalnızca ilgilenen işletmeler için
                        // önemlidir ama her kayıtta girilebilir.
                        let (cizgi2, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(cizgi2, 0.0, KENAR);
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("TEKLİF & ÖDEME")
                                .font(baslik_font(9.5))
                                .color(DIM),
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            for paket in musteriler::Paket::HEPSI {
                                let secili = teklif_paket == Some(paket);
                                if filtre_cipi(ui, paket.etiket(), secili).clicked() {
                                    teklif_paket = Some(paket);
                                    // Paket değişince tutar varsayılana döner;
                                    // satıcı yine de elle değiştirebilir.
                                    tutar_metni = paket.varsayilan_tutar().to_string();
                                    teklif_degisti = true;
                                }
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Tutar").size(10.5).color(SOLUK));
                            let tutar_alani = egui::TextEdit::singleline(&mut tutar_metni)
                                .desired_width(72.0)
                                .font(egui::TextStyle::Small)
                                .hint_text("TL");
                            let yanit = ui.add(tutar_alani);
                            // Yazarken her tuşta dosya yazmamak için yalnızca
                            // odak kaybolunca kaydedilir.
                            if yanit.lost_focus() && teklif_paket.is_some() {
                                teklif_degisti = true;
                            }
                            ui.label(egui::RichText::new("₺").size(10.5).color(SOLUK));
                        });
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            for durum in musteriler::Odeme::HEPSI {
                                let secili = odeme == durum;
                                if filtre_cipi(ui, durum.etiket(), secili).clicked() {
                                    odeme = durum;
                                    odeme_degisti = true;
                                }
                            }
                        });
                        // Kısmi ödemede alınan tutar ayrı tutulur; kalan bakiye
                        // buradan hesaplanır.
                        if odeme == musteriler::Odeme::Kismi {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Tahsil edilen").size(10.5).color(SOLUK),
                                );
                                let alan = egui::TextEdit::singleline(&mut tahsil_metni)
                                    .desired_width(72.0)
                                    .font(egui::TextStyle::Small)
                                    .hint_text("TL");
                                if ui.add(alan).lost_focus() {
                                    tahsil_degisti = true;
                                }
                                ui.label(egui::RichText::new("₺").size(10.5).color(SOLUK));
                            });
                        }
                        if let (Some(toplam), Some(tahsil)) =
                            (teklif_toplam, girdi_tutar(&tahsil_metni))
                        {
                            if odeme == musteriler::Odeme::Kismi && tahsil < toplam {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Kalan bakiye: {} ₺",
                                        toplam - tahsil
                                    ))
                                    .size(10.0)
                                    .color(VURGU),
                                );
                            }
                        }
                        if let Some(tarih) = &teklif_tarihi {
                            ui.label(
                                egui::RichText::new(format!("Teklif tarihi: {tarih}"))
                                    .size(10.0)
                                    .color(DIM),
                            );
                        }
                        ui.add_space(12.0);
                        if !lead.site_linki.is_empty() {
                            ui.label(
                                egui::RichText::new("WEB ADRESİ")
                                    .font(baslik_font(9.5))
                                    .color(DIM),
                            );
                            ui.add_space(3.0);
                            let gosterim = if lead.site_linki.chars().count() > 44 {
                                format!("{}…", lead.site_linki.chars().take(44).collect::<String>())
                            } else {
                                lead.site_linki.clone()
                            };
                            ui.hyperlink_to(
                                egui::RichText::new(gosterim).size(12.0).color(MAVI),
                                &lead.site_linki,
                            );
                            ui.add_space(12.0);
                        }
                        detay_satiri(ui, "ADRES", &lead.address);
                        detay_satiri(
                            ui,
                            "TELEFON",
                            if lead.phone.is_empty() {
                                "Kayıtta yok"
                            } else {
                                &lead.phone
                            },
                        );
                        let saatler = if lead.calisma_saatleri.is_empty() {
                            "Kayıtta yok".to_string()
                        } else {
                            calisma_saatleri_metni(&lead.calisma_saatleri)
                        };
                        detay_satiri(ui, "ÇALIŞMA SAATLERİ", &saatler);

                        ui.label(
                            egui::RichText::new("KONUM")
                                .font(baslik_font(9.5))
                                .color(DIM),
                        );
                        ui.add_space(3.0);
                        if let Some((enlem, boylam)) = lead.konum {
                            ui.label(
                                egui::RichText::new(format!("{enlem:.5}, {boylam:.5}"))
                                    .size(12.0)
                                    .color(METIN),
                            );
                            ui.add_space(2.0);
                            let harita = format!("https://www.google.com/maps?q={enlem},{boylam}");
                            ui.hyperlink_to(
                                egui::RichText::new("Haritada aç ↗").size(11.0).color(MAVI),
                                harita,
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Koordinat kayıtta yok")
                                    .size(12.0)
                                    .color(DIM),
                            );
                        }
                        ui.add_space(12.0);

                        let (cizgi, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(cizgi, 0.0, KENAR);
                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            secim_dairesi(ui, &mut lead.selected);
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(if lead.selected {
                                    "Bu işletme seçildi"
                                } else {
                                    "Bu işletmeyi seç"
                                })
                                .size(11.5)
                                .color(SOLUK),
                            );
                        });
                    });

                ui.add_space(10.0);
                let numara = whatsapp_numarasi(&lead.phone);
                let wa = birincil_buton(
                    ui,
                    if lead.mesaj_hazir {
                        "WhatsApp'ı yeniden aç"
                    } else {
                        "WhatsApp mesajını hazırla"
                    },
                    numara.is_some(),
                    Some(egui::vec2(IC_GENISLIK, 38.0)),
                );
                let wa = if numara.is_some() {
                    wa
                } else {
                    wa.on_hover_text("Telefon numarası kayıtta yok")
                };
                if wa.clicked() {
                    wa_iste = true;
                }

                ui.add_space(8.0);
                let demo_metin = if lead.demo_yolu.is_some() {
                    "Demoyu aç"
                } else {
                    "Demo üret"
                };
                if ikincil_buton(ui, demo_metin, true, Some(egui::vec2(IC_GENISLIK, 38.0)))
                    .clicked()
                {
                    demo_ac(lead, gonderen, &mut self.logs);
                }

                ui.add_space(8.0);
                if ikincil_buton(
                    ui,
                    "Dijital karne",
                    true,
                    Some(egui::vec2(IC_GENISLIK, 38.0)),
                )
                .on_hover_text(
                    "Eksikleri ve puanı tek sayfada gösterir; demo ile birlikte üretilir",
                )
                .clicked()
                {
                    karne_ac(lead, gonderen, &mut self.logs);
                }

                ui.add_space(8.0);
                if ikincil_buton(
                    ui,
                    if lead.yayin_linki.is_some() {
                        "Yayın linkini yenile"
                    } else {
                        "Demo linki oluştur"
                    },
                    true,
                    Some(egui::vec2(IC_GENISLIK, 38.0)),
                )
                .on_hover_text("Demoyu GitHub Pages'e yayınlar; linki müşteriye gönderebilirsiniz")
                .clicked()
                {
                    yayin_iste = true;
                }
                if let Some(link) = lead.yayin_linki.clone() {
                    ui.add_space(2.0);
                    ui.hyperlink_to(
                        egui::RichText::new("Yayındaki demo linki")
                            .size(10.5)
                            .color(MAVI),
                        link,
                    );
                    ui.add_space(2.0);
                    if hayalet_buton(ui, "Yayından kaldır")
                        .on_hover_text("Linki 404 yapar; demo ve karne yayın dalından silinir")
                        .clicked()
                    {
                        yayin_kaldir_iste = true;
                    }
                }
                ui.add_space(2.0);
                ui.hyperlink_to(
                    egui::RichText::new(kaynak_etiketi(&lead.kaynak))
                        .size(10.5)
                        .color(DIM),
                    &lead.kaynak,
                );
            });

        if not_degisti {
            self.defter.not_ayarla(&anahtar, &not_metni);
            self.defter.kaydet();
        }
        if teklif_degisti {
            if let Some(paket) = teklif_paket {
                let tutar = girdi_tutar(&tutar_metni).unwrap_or_else(|| paket.varsayilan_tutar());
                self.defter
                    .teklif_ayarla(&anahtar, paket, tutar, &bugun_anahtari());
                self.defter.kaydet();
                self.logs.push(format!(
                    "[i] {}: {} paketi · {tutar} ₺ teklif edildi.",
                    self.leads[idx].name,
                    paket.etiket()
                ));
            }
        }
        if tahsil_degisti {
            if let Some(tutar) = girdi_tutar(&tahsil_metni) {
                self.defter.tahsil_ayarla(&anahtar, tutar);
                self.defter.kaydet();
                self.logs.push(format!(
                    "[i] {}: {} ₺ tahsil edildi.",
                    self.leads[idx].name, tutar
                ));
            }
        }
        if odeme_degisti {
            self.defter.odeme_ayarla(&anahtar, odeme);
            self.defter.kaydet();
            self.logs.push(format!(
                "[i] {}: ödeme {}",
                self.leads[idx].name,
                odeme.etiket()
            ));
        }
        if wa_iste {
            self.gonderime_hazirla(idx, gonderen);
        }
        if yayin_iste {
            demo_yayinla(&mut self.leads[idx], gonderen, &mut self.logs);
            self.yayin_linkini_deftere_yaz(idx);
        }
        if yayin_kaldir_iste {
            demo_yayindan_kaldir(&mut self.leads[idx], &mut self.logs);
            self.yayin_linkini_deftere_yaz(idx);
        }
        if let Some(durum) = yeni_durum {
            let lead = &self.leads[idx];
            let anahtar = musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
            self.defter.durum_ayarla(&anahtar, durum, &bugun_anahtari());
            self.defter.kaydet();
            // "Gönderdim" onayı günlük gönderim hakkından düşer.
            if durum == musteriler::Durum::Gonderildi {
                self.gonderim_kaydet(idx);
            }
            self.leads[idx].durum = durum;
            self.logs
                .push(format!("[i] {} → {}", self.leads[idx].name, durum.etiket()));
        }
        if kapat {
            self.detay_idx = None;
        }
    }

    /// Seçili il, ilçe (ya da il geneli) ve sektörle yeni bir tarama başlatır.
    /// Takip zamanı gelmiş işletmelerin defter anahtarları: mesaj gönderileli
    /// `TAKIP_GUN` günden fazla olmuş, hâlâ yanıt bekleyenler.
    fn takip_bekleyenler(&self, bugun: &str) -> HashSet<String> {
        self.defter
            .kayitlar()
            .iter()
            .filter(|kayit| kayit.durum == musteriler::Durum::Gonderildi)
            .filter(|kayit| {
                kayit
                    .son_temas
                    .as_deref()
                    .and_then(|temas| gun_farki(temas, bugun))
                    .map(|gun| gun >= TAKIP_GUN)
                    .unwrap_or(false)
            })
            .map(|kayit| kayit.anahtar.clone())
            .collect()
    }

    fn tarama_baslat(&mut self) {
        if self.is_running {
            return;
        }
        self.is_running = true;
        self.leads.clear();
        // Yeni arama eski hedef listesini sıfırlar.
        self.hedef.temizle();
        self.hedef.kaydet();
        self.detay_idx = None;
        // Açık önizleme eski listeye aitti; indeks kayacağı için kapatılır.
        self.onizleme = None;
        self.bos_ilce_uyari = None;
        self.ilerleme = None;
        let tx = self.log_tx.clone();
        let il = self.cities[self.selected_city_idx].0.to_string();
        let ilce = if self.il_geneli {
            None
        } else {
            Some(self.cities[self.selected_city_idx].1[self.selected_district_idx].to_string())
        };
        let sektor = self.sectors[self.selected_sector_idx].to_string();
        let onay = self.kota_onay;
        self.kota_onay = false;
        thread::spawn(move || tarama_calistir(tx, il, ilce, sektor, onay));
    }

    /// İki sütunlu sektör seçim paneli: solda tüm gruplar, sağda seçili grubun
    /// sektörleri. Arama kutusu tüm katalogda arar.
    fn sektor_paneli(&mut self, ctx: &egui::Context) {
        if !self.sektor_panel_acik {
            return;
        }
        let ekran = ctx.screen_rect();
        let boyut = egui::vec2(
            (ekran.width() - 140.0).clamp(760.0, 920.0),
            (ekran.height() - 170.0).clamp(430.0, 580.0),
        );
        let konum = ekran.center() - boyut * 0.5;
        let mut kapat = false;

        // Arkaya tıklayınca kapanan karartma; panellerin üstünde kalmalı.
        egui::Area::new(egui::Id::new("sektor_ortu"))
            .order(egui::Order::Foreground)
            .fixed_pos(ekran.min)
            .show(ctx, |ui| {
                let (rect, yanit) = ui.allocate_exact_size(ekran.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(155));
                if yanit.clicked() {
                    kapat = true;
                }
            });

        egui::Area::new(egui::Id::new("sektor_panel"))
            .order(egui::Order::Foreground)
            .fixed_pos(konum)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR_GUCLU))
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(boyut.x - 40.0);
                        ui.set_height(boyut.y - 32.0);
                        self.sektor_paneli_icerik(ui, &mut kapat);
                    });
            });

        if kapat || ctx.input(|girdi| girdi.key_pressed(egui::Key::Escape)) {
            self.sektor_panel_acik = false;
            self.sector_search.clear();
        }
    }

    fn sektor_paneli_icerik(&mut self, ui: &mut egui::Ui, kapat: &mut bool) {
        let gruplar = sektor_gruplari();
        if self.sektor_panel_grup >= gruplar.len() {
            self.sektor_panel_grup = 0;
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Sektör seç")
                    .font(baslik_font(16.0))
                    .color(METIN),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} grup · {} sektör",
                    gruplar.len(),
                    SEKTORLER.len()
                ))
                .size(10.5)
                .color(DIM),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if hayalet_buton(ui, "Kapat").clicked() {
                    *kapat = true;
                }
            });
        });
        ui.add_space(8.0);
        arama_kutusu(
            ui,
            &mut self.sector_search,
            "Sektör ara (kuaför, matbaa, fayans...)",
        );
        ui.add_space(10.0);

        let govde = (ui.available_height() - 2.0).max(220.0);
        let sol_genislik = 340.0_f32.min(ui.available_width() * 0.45);
        let mut secilen_grup: Option<usize> = None;

        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(sol_genislik, govde),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    bolum_etiketi(ui, "GRUPLAR");
                    ui.add_space(6.0);
                    let aktif = gruplar[self.sektor_panel_grup];
                    egui::ScrollArea::vertical()
                        .id_source("panel_gruplar")
                        .max_height(govde - 22.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.columns(2, |kolonlar| {
                                for (idx, grup) in gruplar.iter().enumerate() {
                                    let sayi = SEKTORLER
                                        .iter()
                                        .filter(|tanim| tanim.grup == *grup)
                                        .count();
                                    if grup_satiri(
                                        &mut kolonlar[idx % 2],
                                        grup,
                                        sayi,
                                        *grup == aktif,
                                    )
                                    .clicked()
                                    {
                                        secilen_grup = Some(idx);
                                    }
                                }
                            });
                        });
                },
            );

            ui.add_space(18.0);

            let sag_genislik = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(sag_genislik, govde),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Frame::none()
                        .fill(GIRINTI)
                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(6.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.set_height(govde - 2.0);
                            let sorgu = arama_anahtari(&self.sector_search);
                            let mut secilen_sektor: Option<usize> = None;

                            if sorgu.is_empty() {
                                let grup = gruplar[self.sektor_panel_grup];
                                let sektorler = grup_sektorleri(grup);
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(grup)
                                            .font(baslik_font(12.5))
                                            .color(METIN),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} sektör",
                                                    sektorler.len()
                                                ))
                                                .size(10.0)
                                                .color(DIM),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(8.0);
                                // Grup genişledikçe liste panelin dışına taşabilir:
                                // sektör listesi kaydırılabilir olmalı.
                                egui::ScrollArea::vertical()
                                    .id_source("panel_grup_sektorler")
                                    .max_height(govde - 56.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.spacing_mut().item_spacing.y = 1.0;
                                        for ad in sektorler {
                                            let secili =
                                                self.sectors[self.selected_sector_idx] == ad;
                                            if sektor_satiri(ui, ad, "", secili).clicked() {
                                                secilen_sektor = Some(sektor_sirasi(ad));
                                            }
                                        }
                                    });
                            } else {
                                let bulunanlar: Vec<&'static str> = SEKTORLER
                                    .iter()
                                    .filter(|tanim| arama_anahtari(tanim.ad).contains(&sorgu))
                                    .map(|tanim| tanim.ad)
                                    .collect();
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new("Arama")
                                            .font(baslik_font(12.5))
                                            .color(METIN),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} sonuç",
                                                    bulunanlar.len()
                                                ))
                                                .size(10.0)
                                                .color(DIM),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(8.0);
                                egui::ScrollArea::vertical()
                                    .id_source("panel_arama")
                                    .max_height(govde - 56.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.spacing_mut().item_spacing.y = 1.0;
                                        for ad in &bulunanlar {
                                            let secili =
                                                self.sectors[self.selected_sector_idx] == *ad;
                                            if sektor_satiri(ui, ad, sektor_grubu(ad), secili)
                                                .clicked()
                                            {
                                                secilen_sektor = Some(sektor_sirasi(ad));
                                            }
                                        }
                                        if bulunanlar.is_empty() {
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new("Eşleşen sektör yok")
                                                    .size(11.0)
                                                    .color(DIM),
                                            );
                                        }
                                    });
                            }

                            if let Some(idx) = secilen_sektor {
                                self.selected_sector_idx = idx;
                                self.bos_ilce_uyari = None;
                                *kapat = true;
                            }
                        });
                },
            );
        });

        if let Some(idx) = secilen_grup {
            self.sektor_panel_grup = idx;
        }
    }

    /// Demo sekmesinin renklendirme bölümü: paletler, zemin/yazı/vurgu rengi,
    /// bölüm anahtarları, metinler ve fotoğraflar. Eskiden üst şeritten açılan
    /// ayrı bir pencereydi; artık Katman 3'ün sol panelinde çizilir.
    /// Değerler `ayarlar.json`'a yazılır ve yeni üretilen demolara uygulanır.
    pub(crate) fn renklendirme_bolumu(&mut self, ui: &mut egui::Ui) {
        let mut degisti = false;
        ui.horizontal(|ui| {
            bolum_etiketi(ui, "RENKLENDİRME");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if hayalet_buton(ui, "‹ Demo").clicked() {
                    self.tema_panel_acik = false;
                }
            });
        });
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Hareketler aynı kalır; yazı, buton, fotoğraf, renk ve bölüm sizin seçiminiz.",
            )
            .size(10.5)
            .color(DIM),
        );
        ui.add_space(10.0);

        // Yerleşim: klasik çok bölümlü sayfa ya da tek ekranlık kompozisyon.
        bolum_etiketi(ui, "YERLEŞİM");
        ui.add_space(6.0);
        let mut yerlesim = self.tema_tema.clone();
        if secenek_cipleri(
            ui,
            &mut yerlesim,
            &[("", "Klasik"), ("gece", "Gece"), ("isik", "Işık")],
        ) {
            self.tema_tema = yerlesim;
            degisti = true;
        }
        ui.add_space(12.0);

        // Üretim motoru: ajans beş bölümlü sinematik akışı, klasik eski
        // şablonu kullanır. Sahne temaları (Gece/Işık) kendi motorunu korur.
        bolum_etiketi(ui, "ÜRETİM MOTORU");
        ui.add_space(6.0);
        let mut profil = self.tema_profil.clone();
        if secenek_cipleri(
            ui,
            &mut profil,
            &[
                ("ajans", "Ajans · 5 bölüm"),
                ("klasik", "Klasik"),
                ("sinematik", "Sinematik · sahneler"),
            ],
        ) {
            self.tema_profil = profil;
            degisti = true;
        }
        if self.tema_profil == "sinematik" {
            ui.add_space(10.0);
            bolum_etiketi(ui, "SİNEMATİK SAHNELER");
            ui.add_space(4.0);
            let mut hareket = self.tema_sinematik_hareket.clone();
            if secenek_cipleri(
                ui,
                &mut hareket,
                &[("dusuk", "Düşük"), ("orta", "Orta"), ("yuksek", "Yüksek")],
            ) {
                self.tema_sinematik_hareket = hareket;
                degisti = true;
            }
            ui.add_space(4.0);
            let mut nesne = self.tema_sinematik_nesne.clone();
            if secenek_cipleri(
                ui,
                &mut nesne,
                &[
                    ("otomatik", "Otomatik"),
                    ("halka", "Halka"),
                    ("kup", "Küp"),
                    ("dalga", "Dalga"),
                    ("geometri", "Geometri"),
                ],
            ) {
                self.tema_sinematik_nesne = nesne;
                degisti = true;
            }
            ui.add_space(4.0);
            let mut sahne = self.tema_sinematik_sahne.to_string();
            if secenek_cipleri(
                ui,
                &mut sahne,
                &[("3", "3 sahne"), ("4", "4"), ("5", "5"), ("6", "6")],
            ) {
                self.tema_sinematik_sahne = sahne.parse().unwrap_or(4);
                degisti = true;
            }
            ui.add_space(8.0);
            let boyut_metni = match son_demo_boyutu() {
                Some((bayt, kirpilan)) => {
                    let mb = format!("{:.2}", bayt as f32 / 1_048_576.0).replace('.', ",");
                    if kirpilan > 0 {
                        format!(
                            "Son çıktı: {mb} MB · sınır 1,35 MB · bütçe için {kirpilan} fotoğraf çıkarıldı."
                        )
                    } else {
                        format!("Son çıktı: {mb} MB · sınır 1,35 MB · 3–6 sahne.")
                    }
                }
                None => "Henüz sinematik çıktı yok; üretince boyut burada görünür.".to_string(),
            };
            ui.label(egui::RichText::new(boyut_metni).size(10.5).color(DIM));
            ui.add_space(8.0);
            self.stok_paneli(ui);
            self.manuel_medya_paneli(ui);
        }
        ui.add_space(12.0);

        // Hazır paletler: tek tıkla tüm renkleri birlikte ayarlar.
        ui.horizontal_wrapped(|ui| {
            for (ad, zemin, yazi, vurgu) in DEMO_PALETLER {
                let secili =
                    self.tema_zemin == zemin && self.tema_yazi == yazi && self.tema_vurgu == vurgu;
                let metin = egui::RichText::new(ad).size(10.5).color(if secili {
                    KOYU_YAZI
                } else {
                    METIN
                });
                let buton = egui::Button::new(metin)
                    .fill(if secili { VURGU } else { KART })
                    .rounding(egui::Rounding::same(999.0))
                    .min_size(egui::vec2(0.0, 24.0));
                if ui.add(buton).clicked() {
                    self.tema_zemin = zemin.to_string();
                    self.tema_yazi = yazi.to_string();
                    self.tema_vurgu = vurgu.to_string();
                    degisti = true;
                }
            }
        });
        ui.add_space(12.0);

        if renk_secici(ui, "Arka plan", &mut self.tema_zemin, DEMO_ZEMIN) {
            degisti = true;
        }
        if renk_secici(ui, "Yazı rengi", &mut self.tema_yazi, DEMO_YAZI) {
            degisti = true;
        }
        if renk_secici(ui, "Vurgu rengi", &mut self.tema_vurgu, "#e8a33d") {
            degisti = true;
        }
        ui.add_space(8.0);
        let hafiza_sektor = self.sectors[self.selected_sector_idx];
        let hafiza_tasarim = sektor_tasarim::sec(hafiza_sektor);
        let ders = ogrenme::ders(hafiza_sektor);
        let hafiza_sinematik = ogrenilen_sinematik(hafiza_tasarim.sinematik, &ders);
        if ders.kayit_sayisi > 0 {
            let animasyon = ders.animasyon.as_deref().unwrap_or("kenburns");
            let not: String = ders
                .notlar
                .first()
                .map(|not| not.chars().take(60).collect())
                .unwrap_or_else(|| "onay bekliyor".to_string());
            ui.label(
                egui::RichText::new(format!(
                    "Hafıza · {animasyon} · {} kayıt · {not}",
                    ders.kayit_sayisi
                ))
                .size(9.5)
                .color(DIM),
            );
        }
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Revizyon notu").size(9.5).color(DIM));
        ui.add(
            egui::TextEdit::singleline(&mut self.hafiza_not)
                .hint_text("ör. zemin daha açık olsun")
                .desired_width(f32::INFINITY)
                .char_limit(170),
        );
        ui.add_space(4.0);
        if hayalet_buton(ui, "Paleti ve notu hafızaya al")
            .on_hover_text("Palet ve animasyon onaylanır; not sonraki demolarda ders olur")
            .clicked()
        {
            let vurgu = if self.tema_vurgu.trim().is_empty() {
                hafiza_tasarim.vurgu.to_string()
            } else {
                self.tema_vurgu.clone()
            };
            let zemin = if self.tema_zemin.trim().is_empty() {
                hafiza_tasarim.zemin.to_string()
            } else {
                self.tema_zemin.clone()
            };
            let yazi = if self.tema_yazi.trim().is_empty() {
                hafiza_tasarim.yazi.to_string()
            } else {
                self.tema_yazi.clone()
            };
            ogrenme::onay(
                hafiza_sektor,
                hafiza_tasarim.grup,
                animasyon_sinifi(hafiza_sinematik),
                &vurgu,
                &zemin,
                &yazi,
                &self.hafiza_not,
            );
            let not_var = !self.hafiza_not.trim().is_empty();
            self.hafiza_not.clear();
            self.logs.push(if not_var {
                format!("[i] Palet ve not hafızaya alındı · {hafiza_sektor}")
            } else {
                format!("[i] Palet hafızaya alındı · {hafiza_sektor}")
            });
            if self.hafiza_panel {
                self.hafiza_liste = ogrenme::kayitlar();
            }
        }
        ui.add_space(8.0);

        // Hafıza yönetimi: kayıtları gör, tek tek sil veya tümünü temizle.
        ui.horizontal(|ui| {
            let etiket = if self.hafiza_panel {
                "Hafızayı kapat"
            } else {
                "Hafızayı görüntüle / temizle"
            };
            if hayalet_buton(ui, etiket).clicked() {
                self.hafiza_panel = !self.hafiza_panel;
                self.hafiza_temizle_onay = false;
                if self.hafiza_panel {
                    self.hafiza_liste = ogrenme::kayitlar();
                }
            }
            if self.hafiza_panel && hayalet_buton(ui, "Yenile").clicked() {
                self.hafiza_liste = ogrenme::kayitlar();
            }
        });

        if self.hafiza_panel {
            ui.add_space(4.0);
            if self.hafiza_liste.is_empty() {
                ui.label(
                    egui::RichText::new("Kayıt yok. İlk onaydan sonra burada görünür.")
                        .size(10.0)
                        .color(DIM),
                );
            } else {
                let mut silinecek: Option<usize> = None;
                egui::ScrollArea::vertical()
                    .id_source("hafiza_listesi")
                    .max_height(190.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (sira, kayit) in self.hafiza_liste.iter().enumerate() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · {} · {}×",
                                        kayit.sektor, kayit.animasyon, kayit.sayi
                                    ))
                                    .size(10.0)
                                    .color(METIN),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        kayit.palet.vurgu, kayit.palet.zemin, kayit.palet.yazi
                                    ))
                                    .size(9.0)
                                    .color(SOLUK),
                                );
                                if hayalet_buton(ui, "Sil").clicked() {
                                    silinecek = Some(sira);
                                }
                            });
                            if !kayit.not.trim().is_empty() {
                                let not: String = kayit.not.chars().take(90).collect();
                                ui.label(
                                    egui::RichText::new(format!("“{not}”")).size(9.5).color(DIM),
                                );
                            }
                            ui.add_space(4.0);
                        }
                    });
                if let Some(sira) = silinecek {
                    let ad = self
                        .hafiza_liste
                        .get(sira)
                        .map(|kayit| kayit.sektor.clone())
                        .unwrap_or_default();
                    if ogrenme::sil(sira) {
                        self.hafiza_liste = ogrenme::kayitlar();
                        self.logs.push(format!("[i] Hafıza kaydı silindi · {ad}"));
                    }
                }
            }
            ui.add_space(6.0);
            if self.hafiza_temizle_onay {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Tüm kayıtlar silinsin mi?")
                            .size(10.0)
                            .color(VURGU),
                    );
                    if hayalet_buton(ui, "Evet, temizle").clicked() {
                        ogrenme::temizle();
                        self.hafiza_liste.clear();
                        self.hafiza_temizle_onay = false;
                        self.logs.push("[i] Hafıza temizlendi".to_string());
                    }
                    if hayalet_buton(ui, "Vazgeç").clicked() {
                        self.hafiza_temizle_onay = false;
                    }
                });
            } else if hayalet_buton(ui, "Tüm hafızayı temizle").clicked() {
                self.hafiza_temizle_onay = true;
            }
        }
        ui.add_space(14.0);

        // Bölümler: kapatılan bölüm sayfada hiç çizilmez.
        bolum_etiketi(ui, "BÖLÜMLER");
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for (ad, acik) in [
                ("Galeri", &mut self.tema_galeri),
                ("Yorumlar", &mut self.tema_yorumlar),
                ("Harita", &mut self.tema_harita),
                ("Saatler", &mut self.tema_saatler),
            ] {
                if secim_cipi(ui, ad, acik) {
                    degisti = true;
                }
            }
        });
        ui.add_space(14.0);

        // Metinler: sektör şablonunun üzerine yazılır, boşsa otomatik.
        egui::CollapsingHeader::new(egui::RichText::new("Metinler").size(10.5).color(SOLUK))
            .default_open(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let mut degisim = false;
                degisim |= metin_alani(
                    ui,
                    "Manşet · 1. satır",
                    &mut self.tema_manset1,
                    "otomatik",
                    1,
                );
                degisim |= metin_alani(
                    ui,
                    "Manşet · 2. satır",
                    &mut self.tema_manset2,
                    "otomatik",
                    1,
                );
                degisim |= metin_alani(
                    ui,
                    "Marka cümlesi · hero altı",
                    &mut self.tema_slogan,
                    "ör. Sabahları kendi kavrumuzla",
                    1,
                );
                ui.add_space(4.0);
                degisim |= metin_alani(
                    ui,
                    "Hakkında · başlık",
                    &mut self.tema_hakkinda_baslik,
                    "otomatik",
                    1,
                );
                degisim |= metin_alani(
                    ui,
                    "Hakkında · 1. paragraf",
                    &mut self.tema_hakkinda1,
                    "otomatik",
                    3,
                );
                degisim |= metin_alani(
                    ui,
                    "Hakkında · 2. paragraf",
                    &mut self.tema_hakkinda2,
                    "otomatik",
                    3,
                );
                ui.add_space(4.0);
                degisim |= metin_alani(
                    ui,
                    "Bölüm etiketi",
                    &mut self.tema_bolum_etiket,
                    "ör. HİZMETLER",
                    1,
                );
                degisim |= metin_alani(
                    ui,
                    "Bölüm başlığı",
                    &mut self.tema_bolum_baslik,
                    "otomatik",
                    1,
                );
                degisim |= metin_alani(
                    ui,
                    "Bölüm alt yazısı",
                    &mut self.tema_bolum_alt,
                    "otomatik",
                    2,
                );
                ui.add_space(4.0);
                // Hizmet/menü satırları: boş bırakılan satır şablonda kalır.
                for (sira, (ad, aciklama)) in self.tema_kalemler.iter_mut().enumerate() {
                    degisim |=
                        metin_alani(ui, &format!("Satır {} · ad", sira + 1), ad, "otomatik", 1);
                    degisim |= metin_alani(
                        ui,
                        &format!("Satır {} · açıklama", sira + 1),
                        aciklama,
                        "otomatik",
                        1,
                    );
                }
                ui.add_space(4.0);
                for (sira, (soz, kim)) in self.tema_yorum_metinleri.iter_mut().enumerate() {
                    degisim |=
                        metin_alani(ui, &format!("Yorum {} · söz", sira + 1), soz, "otomatik", 2);
                    degisim |= metin_alani(
                        ui,
                        &format!("Yorum {} · isim", sira + 1),
                        kim,
                        "otomatik",
                        1,
                    );
                }
                if degisim {
                    self.ayari_sessiz_yaz();
                }
            });
        ui.add_space(6.0);

        // Butonlar: birincil (telefon) ve ikincil (konum/iletişim).
        egui::CollapsingHeader::new(egui::RichText::new("Butonlar").size(10.5).color(SOLUK))
            .default_open(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let mut degisim = false;
                degisim |= metin_alani(
                    ui,
                    "Birincil buton",
                    &mut self.tema_cta,
                    "otomatik (ör. Masa ayırt)",
                    1,
                );
                degisim |= metin_alani(
                    ui,
                    "İkincil buton",
                    &mut self.tema_ikincil,
                    "otomatik (Konumu gör)",
                    1,
                );
                if degisim {
                    self.ayari_sessiz_yaz();
                }
            });
        ui.add_space(6.0);

        // Fotoğraflar: hero kaynağı, galeri kaynağı ve kendi dosyaları.
        egui::CollapsingHeader::new(
                    egui::RichText::new("Fotoğraflar").size(10.5).color(SOLUK),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let mut degisim = false;
                    ui.label(
                        egui::RichText::new("Manşet fotoğrafı")
                            .size(10.0)
                            .color(DIM),
                    );
                    degisim |= secenek_cipleri(
                        ui,
                        &mut self.tema_hero,
                        &[("", "Otomatik"), ("yok", "Fotoğrafsız"), ("dosya", "Kendi dosyam")],
                    );
                    if self.tema_hero == "dosya" {
                        if metin_kutusu(ui, &mut self.tema_hero_dosya, "C:\\fotoğraflar\\cephe.jpg") {
                            degisim = true;
                        }
                        ui.label(
                            egui::RichText::new(
                                "Dosya yolu; boşsa eklediğiniz ilk fotoğraf kullanılır.\nOkunamazsa işletme fotoğrafına dönülür.",
                            )
                            .size(9.5)
                            .color(DIM),
                        );
                    }
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Galeri kaynağı")
                            .size(10.0)
                            .color(DIM),
                    );
                    degisim |= secenek_cipleri(
                        ui,
                        &mut self.tema_galeri_kaynak,
                        &[
                            ("", "Otomatik"),
                            ("oto+ekle", "Oto + eklediklerim"),
                            ("ekle", "Yalnız eklediklerim"),
                            ("yok", "Kapalı"),
                        ],
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Kare sayısı")
                                .size(10.0)
                                .color(DIM),
                        );
                        let sayi = ui.add(
                            egui::Slider::new(&mut self.tema_galeri_adet, 1..=GALERI_AZAMI)
                                .show_value(true),
                        );
                        if sayi.changed() {
                            degisim = true;
                        }
                    });
                    ui.add_space(6.0);

                    // Eklenen dosyalar: yol tutulur, görüntü üretim anında okunur.
                    let mut silinecek: Option<usize> = None;
                    for (sira, yol) in self.tema_fotolar.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(dosya_adi(yol))
                                    .size(10.5)
                                    .color(METIN),
                            )
                            .on_hover_text(yol);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if hayalet_buton(ui, "✕").clicked() {
                                        silinecek = Some(sira);
                                    }
                                },
                            );
                        });
                    }
                    if let Some(sira) = silinecek {
                        self.tema_fotolar.remove(sira);
                        degisim = true;
                    }
                    if self.tema_fotolar.len() < GALERI_AZAMI {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.tema_foto_yolu)
                                    .frame(false)
                                    .hint_text(
                                        egui::RichText::new("Fotoğraf yolu")
                                            .size(11.0)
                                            .color(DIM),
                                    )
                                    .font(govde_font(11.0))
                                    .desired_width(150.0),
                            );
                            if ikincil_buton(
                                ui,
                                "Ekle",
                                !self.tema_foto_yolu.trim().is_empty(),
                                Some(egui::vec2(52.0, 24.0)),
                            )
                            .clicked()
                            {
                                let yol = self.tema_foto_yolu.clone();
                                if self.foto_ekle(&yol) {
                                    self.tema_foto_yolu.clear();
                                    degisim = true;
                                }
                            }
                        });
                    }
                    // Sürükle-bırak: dosyalar panelin üzerine bırakılır.
                    let birakilan =
                        ui.ctx().input_mut(|girdi| std::mem::take(&mut girdi.raw.dropped_files));
                    for dosya in birakilan {
                        if let Some(yol) = dosya.path {
                            let yol = yol.display().to_string();
                            if self.foto_ekle(&yol) {
                                degisim = true;
                            }
                        }
                    }
                    ui.label(
                        egui::RichText::new(
                            "JPEG/PNG/WebP dosyaları buraya sürükleyin · en fazla 6, 3 MB'a kadar",
                        )
                        .size(9.5)
                        .color(DIM),
                    );
                    if !self.tema_fotolar.is_empty() && hayalet_buton(ui, "Eklenenleri temizle").clicked()
                    {
                        self.tema_fotolar.clear();
                        degisim = true;
                    }
                    if degisim {
                        self.ayari_sessiz_yaz();
                    }
                });
        ui.add_space(10.0);

        // Köşe biçimi: hareket ve düzen aynı, yalnız ölçü değişir.
        bolum_etiketi(ui, "KÖŞELER");
        ui.add_space(6.0);
        let mut kose = self.tema_kose.clone();
        if secenek_cipleri(
            ui,
            &mut kose,
            &[
                ("keskin", "Keskin"),
                ("", "Dengeli"),
                ("yumusak", "Yumuşak"),
            ],
        ) {
            self.tema_kose = kose;
            degisti = true;
        }
        ui.add_space(10.0);

        // Küçük önizleme: seçilen üç renk yan yana.
        let (onizleme, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0), egui::Sense::hover());
        let boya = ui.painter();
        boya.rect_filled(
            onizleme,
            egui::Rounding::same(10.0),
            egui::Color32::from_rgb(
                renk_coz(&self.tema_zemin)[0],
                renk_coz(&self.tema_zemin)[1],
                renk_coz(&self.tema_zemin)[2],
            ),
        );
        let yazi_rengi = egui::Color32::from_rgb(
            renk_coz(&self.tema_yazi)[0],
            renk_coz(&self.tema_yazi)[1],
            renk_coz(&self.tema_yazi)[2],
        );
        boya.text(
            onizleme.left_center() + egui::vec2(14.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Örnek başlık",
            baslik_font(14.0),
            yazi_rengi,
        );
        let vurgu = egui::Color32::from_rgb(
            renk_coz(if self.tema_vurgu.is_empty() {
                "#e8a33d"
            } else {
                &self.tema_vurgu
            })[0],
            renk_coz(if self.tema_vurgu.is_empty() {
                "#e8a33d"
            } else {
                &self.tema_vurgu
            })[1],
            renk_coz(if self.tema_vurgu.is_empty() {
                "#e8a33d"
            } else {
                &self.tema_vurgu
            })[2],
        );
        boya.rect_filled(
            egui::Rect::from_min_size(
                onizleme.right_center() + egui::vec2(-78.0, -11.0),
                egui::vec2(62.0, 22.0),
            ),
            egui::Rounding::same(999.0),
            vurgu,
        );
        ui.add_space(10.0);
        ui.label(
                    egui::RichText::new("Yeni üretilen demolara uygulanır. Boş alan otomatik: renk sektörden, metinler sektör şablonundan gelir.")
                        .size(10.0)
                        .color(SOLUK),
                );
        ui.add_space(6.0);
        let uygula_genislik = ui.available_width();
        if ikincil_buton(
            ui,
            "Mevcut demolara uygula",
            true,
            Some(egui::vec2(uygula_genislik, 28.0)),
        )
        .on_hover_text(
            "Demos klasöründeki sayfaları güncel ayarlarla yeniden üretir: metinler, \
                     butonlar, fotoğraflar, bölümler ve renkler. Kaynak verisi bulunmayan \
                     elle yapılmış denemelere yalnız renkler işlenir",
        )
        .clicked()
        {
            // Panelde değişen ayar henüz diske yazılmadıysa önce yazılsın:
            // yeniden üretim ayarları dosyadan okur.
            self.temayi_kaydet();
            let zemin = gecerli_renk(&self.tema_zemin, DEMO_ZEMIN);
            let yazi = gecerli_renk(&self.tema_yazi, DEMO_YAZI);
            let vurgu = self
                .tema_vurgu
                .starts_with('#')
                .then(|| self.tema_vurgu.clone());
            let gonderen = whatsapp_numarasi(&self.whatsapp_phone)
                .map(|numara| format!("+{numara}"))
                .unwrap_or_default();
            let onbellek = SonucOnbellek::yukle();
            let (yenilenen, renklenen, atlanan) = demolara_ayarlari_uygula(
                DEMO_KLASORU,
                &onbellek,
                &zemin,
                &yazi,
                vurgu.as_deref(),
                &gonderen,
            );
            if yenilenen == 0 && renklenen == 0 && atlanan == 0 {
                self.logs
                    .push("[i] Demos klasöründe güncellenecek demo bulunamadı.".to_string());
            } else {
                let mut satir = format!("[✓] {yenilenen} demo güncel ayarlarla yeniden üretildi");
                if renklenen > 0 {
                    satir.push_str(&format!(
                        " · {renklenen} demo için kaynak veri yok, yalnız renkler uygulandı"
                    ));
                }
                if atlanan > 0 {
                    satir.push_str(&format!(" ({atlanan} atlandı)"));
                }
                satir.push('.');
                self.logs.push(satir);
                if yenilenen > 0 {
                    self.logs.push(
                        "[i] Yayındaki sayfalar kendiliğinden değişmez; linkin \
                                 güncellenmesi için demoyu yeniden yayınlayın."
                            .to_string(),
                    );
                }
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if birincil_buton(ui, "Kaydet", true, Some(egui::vec2(78.0, 30.0))).clicked() {
                self.temayi_kaydet();
                degisti = false;
            }
            if hayalet_buton(ui, "Otomatiğe dön").clicked() {
                self.tema_vurgu.clear();
                self.tema_zemin.clear();
                self.tema_yazi.clear();
                self.tema_galeri = true;
                self.tema_yorumlar = true;
                self.tema_harita = true;
                self.tema_saatler = true;
                self.tema_slogan.clear();
                self.tema_cta.clear();
                self.tema_kose.clear();
                self.tema_ikincil.clear();
                self.tema_manset1.clear();
                self.tema_manset2.clear();
                self.tema_hakkinda_baslik.clear();
                self.tema_hakkinda1.clear();
                self.tema_hakkinda2.clear();
                self.tema_bolum_etiket.clear();
                self.tema_bolum_baslik.clear();
                self.tema_bolum_alt.clear();
                self.tema_kalemler = vec![(String::new(), String::new()); 4];
                self.tema_yorum_metinleri = vec![(String::new(), String::new()); 2];
                self.tema_hero.clear();
                self.tema_hero_dosya.clear();
                self.tema_galeri_kaynak.clear();
                self.tema_galeri_adet = 3;
                self.tema_sinematik_hareket = "orta".to_string();
                self.tema_sinematik_nesne = "otomatik".to_string();
                self.tema_sinematik_sahne = 4;
                self.tema_fotolar.clear();
                self.tema_foto_yolu.clear();
                self.temayi_kaydet();
                degisti = false;
            }
        });
        // Seçim değiştiyse anında yaz: kullanıcı Kaydet'i unutsa bile korunur.
        if degisti {
            self.temayi_kaydet();
        }
    }

    /// Panel alanlarının tek ayar nesnesi: hem renk hem özelleştirme
    /// seçimleri burada toplanır, kaydetme yolları ayrı ayrı alan yazmaz.
    fn ayar_olustur(&self) -> UygulamaAyari {
        UygulamaAyari {
            whatsapp_phone: self.whatsapp_phone.clone(),
            demo_vurgu: self.tema_vurgu.clone(),
            demo_zemin: self.tema_zemin.clone(),
            demo_yazi: self.tema_yazi.clone(),
            demo_galeri: self.tema_galeri,
            demo_yorumlar: self.tema_yorumlar,
            demo_harita: self.tema_harita,
            demo_saatler: self.tema_saatler,
            demo_slogan: self.tema_slogan.clone(),
            demo_cta: self.tema_cta.clone(),
            demo_kose: self.tema_kose.clone(),
            demo_tema: self.tema_tema.clone(),
            demo_profil: self.tema_profil.clone(),
            demo_sinematik_hareket: self.tema_sinematik_hareket.clone(),
            demo_sinematik_nesne: self.tema_sinematik_nesne.clone(),
            demo_sinematik_sahne: self.tema_sinematik_sahne.clamp(3, 6),
            demo_ikincil: self.tema_ikincil.clone(),
            demo_manset1: self.tema_manset1.clone(),
            demo_manset2: self.tema_manset2.clone(),
            demo_hakkinda_baslik: self.tema_hakkinda_baslik.clone(),
            demo_hakkinda1: self.tema_hakkinda1.clone(),
            demo_hakkinda2: self.tema_hakkinda2.clone(),
            demo_bolum_etiket: self.tema_bolum_etiket.clone(),
            demo_bolum_baslik: self.tema_bolum_baslik.clone(),
            demo_bolum_alt: self.tema_bolum_alt.clone(),
            demo_kalemler: self.tema_kalemler.clone(),
            demo_yorum_metinleri: self.tema_yorum_metinleri.clone(),
            demo_hero_foto: self.tema_hero.clone(),
            demo_hero_dosya: self.tema_hero_dosya.clone(),
            demo_galeri_kaynak: self.tema_galeri_kaynak.clone(),
            demo_galeri_adet: self.tema_galeri_adet,
            demo_fotolar: self.tema_fotolar.clone(),
            google_gunluk_limit: self.api_gunluk,
            google_aylik_limit: self.api_aylik,
            google_sayfa: self.api_sayfa,
            google_proje: self.api_proje.trim().to_string(),
        }
    }

    /// Panele fotoğraf yolu ekler: tür, varlık, tekrar ve 6 dosya sınırı
    /// denetlenir. Eklenirse `true` döner; hata günlüğe yazılır.
    fn foto_ekle(&mut self, yol: &str) -> bool {
        let yol = yol.trim();
        if yol.is_empty() {
            return false;
        }
        if self.tema_fotolar.len() >= GALERI_AZAMI {
            self.logs.push(format!(
                "[!] Galeriye en fazla {GALERI_AZAMI} fotoğraf eklenebilir."
            ));
            return false;
        }
        if self.tema_fotolar.iter().any(|ekli| ekli == yol) {
            self.logs.push(format!("[i] Bu fotoğraf zaten ekli: {yol}"));
            return false;
        }
        if !foto_dosyasi_gecerli(yol) {
            self.logs.push(format!(
                "[!] Fotoğraf okunamadı (JPEG/PNG/WebP, 512 B – 3 MB): {yol}"
            ));
            return false;
        }
        self.tema_fotolar.push(yol.to_string());
        self.logs
            .push(format!("[✓] Fotoğraf eklendi: {}", dosya_adi(yol)));
        true
    }

    /// Metin alanları değiştiğinde çağrılır: diske yazar, günlüğe satır eklemez
    /// (her tuş vuruşunda günlük şişmesin).
    fn ayari_sessiz_yaz(&self) {
        ayar_yaz(&self.ayar_olustur());
    }

    /// Renklendirme ve özelleştirme seçimlerini ayar dosyasına yazar.
    fn temayi_kaydet(&mut self) {
        ayar_yaz(&self.ayar_olustur());
        let kapali: Vec<&str> = [
            (self.tema_galeri, "galeri"),
            (self.tema_yorumlar, "yorumlar"),
            (self.tema_harita, "harita"),
            (self.tema_saatler, "saatler"),
        ]
        .iter()
        .filter(|(acik, _)| !*acik)
        .map(|(_, ad)| *ad)
        .collect();
        self.logs.push(format!(
            "[✓] Demo ayarları kaydedildi: zemin {}, yazı {}, vurgu {}, köşe {}. Kapalı bölüm: {}.",
            if self.tema_zemin.is_empty() {
                "otomatik"
            } else {
                &self.tema_zemin
            },
            if self.tema_yazi.is_empty() {
                "otomatik"
            } else {
                &self.tema_yazi
            },
            if self.tema_vurgu.is_empty() {
                "sektör rengi"
            } else {
                &self.tema_vurgu
            },
            if self.tema_kose.is_empty() {
                "dengeli"
            } else {
                &self.tema_kose
            },
            if kapali.is_empty() {
                "yok".to_string()
            } else {
                kapali.join(", ")
            }
        ));
    }

    /// Gönderen numarayı denetler ve diske yazar; bir sonraki açılışta
    /// aynı numara geri okunur. Bu bir WhatsApp doğrulaması değildir,
    /// yalnızca mesajlarda kullanılacak numaranın kaydıdır.
    fn numarayi_kaydet(&mut self) {
        match whatsapp_numarasi(&self.whatsapp_phone) {
            Some(numara) => {
                self.is_whatsapp_connected = true;
                self.whatsapp_status = format!("Kayıtlı: +{numara}");
                ayar_yaz(&self.ayar_olustur());
                self.logs.push(format!(
                    "[✓] Gönderen numara kaydedildi: +{numara}. Mesajlar tek tıkla hazırlanır, göndermeyi siz onaylarsınız."
                ));
            }
            None => {
                self.is_whatsapp_connected = false;
                self.whatsapp_status = "Geçersiz numara".to_string();
                self.logs
                    .push("[!] Numara geçersiz. Örnek: +90 532 123 45 67".to_string());
            }
        }
    }

    fn whatsapp_ayarlari(&mut self, ui: &mut egui::Ui) {
        bolum_etiketi(ui, "WHATSAPP");
        ui.add_space(6.0);

        let bagli = self.is_whatsapp_connected;
        let numara = whatsapp_numarasi(&self.whatsapp_phone);

        if !self.whatsapp_acik {
            egui::Frame::none()
                .fill(KART)
                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                .rounding(egui::Rounding::same(YUVARLAK_KUCUK))
                .inner_margin(egui::Margin::symmetric(11.0, 9.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (nokta, _) =
                            ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                        ui.painter().circle_filled(
                            nokta.center(),
                            3.5,
                            if bagli { YESIL } else { DIM },
                        );
                        ui.add_space(3.0);
                        let gorunen_numara = if numara.is_some() {
                            self.whatsapp_phone.trim().to_string()
                        } else {
                            "Numara girilmedi".to_string()
                        };
                        ui.label(
                            egui::RichText::new(gorunen_numara)
                                .size(11.5)
                                .color(if bagli { METIN } else { SOLUK }),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if hayalet_buton(ui, "Düzenle").clicked() {
                                self.whatsapp_acik = true;
                            }
                        });
                    });
                });
            return;
        }

        egui::Frame::none()
            .fill(KART)
            .stroke(egui::Stroke::new(1.0_f32, KENAR))
            .rounding(egui::Rounding::same(YUVARLAK_KART))
            .inner_margin(egui::Margin::symmetric(12.0, 11.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Gönderen numara")
                            .size(10.5)
                            .color(SOLUK),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hayalet_buton(ui, "Kapat").clicked() {
                            self.whatsapp_acik = false;
                        }
                    });
                });
                ui.add_space(4.0);
                arama_kutusu(ui, &mut self.whatsapp_phone, "+90 532 123 45 67");
                ui.add_space(8.0);

                let dogrula_genislik = ui.available_width();
                let dogrula = ikincil_buton(
                    ui,
                    if self.is_whatsapp_connected {
                        "Numarayı güncelle"
                    } else {
                        "Numarayı kaydet"
                    },
                    true,
                    Some(egui::vec2(dogrula_genislik, 30.0)),
                );
                if dogrula.clicked() {
                    self.numarayi_kaydet();
                }

                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    let renk = if self.is_whatsapp_connected {
                        YESIL
                    } else {
                        DIM
                    };
                    let (nokta, _) =
                        ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                    ui.painter().circle_filled(nokta.center(), 3.5, renk);
                    ui.label(
                        egui::RichText::new(&self.whatsapp_status)
                            .size(11.0)
                            .color(renk),
                    );
                });
                ui.add_space(6.0);
                // Günlük hak: hesabın askıya alınmaması için 15 mesajla sınırlı.
                let gonderilen = self.defter.gun_gonderim(&bugun_anahtari());
                let kalan = GUNLUK_GONDERIM_LIMITI.saturating_sub(gonderilen);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Bugün {gonderilen}/{GUNLUK_GONDERIM_LIMITI}"))
                            .size(10.5)
                            .color(SOLUK),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(if kalan == 0 {
                                "hak doldu, yarın devam".to_string()
                            } else {
                                format!("{kalan} mesaj kaldı")
                            })
                            .size(10.5)
                            .color(if kalan == 0 {
                                KIRMIZI
                            } else {
                                YESIL
                            }),
                        );
                    });
                });
                if let Some(kayitli) = whatsapp_numarasi(&self.whatsapp_phone) {
                    if !mobil_numara(&kayitli) {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(
                                "Bu numara sabit hat görünüyor; gönderimi cep numaranızdan yapın.",
                            )
                            .size(10.0)
                            .color(KIRMIZI),
                        );
                    }
                }
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Otomatik gönderim yok; hesabınız askıya alınmaz.")
                        .size(10.0)
                        .color(DIM),
                );
            });
    }

    /// Açılış denetimi ve onarım paneli: maddeler canlı güncellenir, hata
    /// varsa ana ekran açılmaz; panel kapanmaz, Düzelt/Yeniden Dene sunulur.
    fn baslangic_ekrani(&mut self, ctx: &egui::Context) {
        let (mut kapat, mut durdur, mut duzelt, mut yeniden) = (false, false, false, false);
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ARKA_PLAN))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(28.0);
                    let genislik = ui.available_width().min(760.0);
                    let bosluk = ((ui.available_width() - genislik) / 2.0).max(0.0);
                    ui.horizontal(|ui| {
                        ui.add_space(bosluk);
                        ui.vertical(|ui| {
                            ui.set_max_width(genislik);
                            ui.label(
                                egui::RichText::new("Nexus Enterprise")
                                    .size(26.0)
                                    .strong()
                                    .color(METIN),
                            );
                            ui.label(
                                egui::RichText::new("Sistem Başlatılıyor ve Doğrulanıyor…")
                                    .size(15.0)
                                    .strong()
                                    .color(VURGU),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "Ana ekran açılmadan önce dört kritik bileşen sınanıyor. Sorun çıkarsa uygulama açılmaz; aşağıdaki düğmelerle onarabilir veya yeniden deneyebilirsiniz.",
                                )
                                .size(12.0)
                                .color(SOLUK),
                            );
                            ui.add_space(14.0);
                            egui::Frame::none()
                                .fill(KART)
                                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                .rounding(egui::Rounding::same(12.0))
                                .inner_margin(egui::Margin::same(18.0))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    for madde in &self.baslangic.maddeler {
                                        denetim_satiri(ui, madde);
                                        ui.add_space(3.0);
                                    }
                                    ui.add_space(8.0);
                                    let tamam = self
                                        .baslangic
                                        .maddeler
                                        .iter()
                                        .filter(|madde| {
                                            matches!(
                                                madde.durum,
                                                saglik::Durum::Gecti | saglik::Durum::Kaldi
                                            )
                                        })
                                        .count();
                                    let oran = tamam as f32 / self.baslangic.maddeler.len() as f32;
                                    ui.add(
                                        egui::ProgressBar::new(oran)
                                            .desired_height(6.0)
                                            .fill(VURGU),
                                    );
                                    ui.add_space(6.0);
                                    let (renk, metin) = if !self.baslangic.bitti {
                                        (SOLUK, "Denetim sürüyor…".to_string())
                                    } else if self.baslangic.basarili {
                                        (
                                            YESIL,
                                            "Tüm denetimler geçti; ana ekran açılıyor…"
                                                .to_string(),
                                        )
                                    } else if self.baslangic.durduruldu {
                                        (
                                            TURUNCU,
                                            "Denetim durduruldu. Yeniden Dene ile baştan çalıştırabilirsiniz."
                                                .to_string(),
                                        )
                                    } else {
                                        let kalan = self
                                            .baslangic
                                            .maddeler
                                            .iter()
                                            .filter(|madde| madde.durum == saglik::Durum::Kaldi)
                                            .count();
                                        (
                                            KIRMIZI,
                                            format!("{kalan} denetim başarısız. Uygulama güvenlik için açılmadı; bu panel açık kalır."),
                                        )
                                    };
                                    ui.label(egui::RichText::new(metin).size(12.5).color(renk));
                                });
                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                if !self.baslangic.bitti {
                                    if birincil_buton(ui, "Durdur", true, None).clicked() {
                                        durdur = true;
                                    }
                                } else {
                                    if birincil_buton(ui, "Otomatik Düzelt", true, None).clicked() {
                                        duzelt = true;
                                    }
                                    ui.add_space(8.0);
                                    if birincil_buton(
                                        ui,
                                        "Yeniden Dene",
                                        !self.baslangic.onarim_suruyor(),
                                        None,
                                    )
                                    .clicked()
                                    {
                                        yeniden = true;
                                    }
                                }
                                ui.add_space(8.0);
                                if birincil_buton(ui, "Kapat", true, None).clicked() {
                                    kapat = true;
                                }
                                if self.baslangic.onarim_suruyor() {
                                    ui.add(egui::Spinner::new().size(14.0).color(VURGU));
                                    ui.label(
                                        egui::RichText::new("Sorunlar onarılıyor…")
                                            .size(11.5)
                                            .color(SOLUK),
                                    );
                                }
                            });
                            if let Some(liste) = &self.baslangic.onarim {
                                ui.add_space(10.0);
                                egui::Frame::none()
                                    .fill(ton(YESIL, 0.07))
                                    .stroke(egui::Stroke::new(1.0_f32, ton(YESIL, 0.25)))
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(egui::Margin::same(12.0))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(
                                            egui::RichText::new("Yapılan onarımlar")
                                                .size(12.0)
                                                .strong()
                                                .color(YESIL),
                                        );
                                        for satir in liste {
                                            ui.label(
                                                egui::RichText::new(format!("• {satir}"))
                                                    .size(11.5)
                                                    .color(SOLUK),
                                            );
                                        }
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "Şimdi Yeniden Dene düğmesine basarak denetimi tekrarlayın.",
                                            )
                                            .size(11.0)
                                            .color(DIM),
                                        );
                                    });
                            }
                        });
                    });
                });
            });
        if durdur {
            self.baslangic.durdur();
        }
        if duzelt {
            self.baslangic.onar_baslat();
        }
        if yeniden {
            self.baslangic.yeniden_basla();
        }
        if kapat {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for LeadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Açılış denetimi geçmeden ana ekran çizilmez: kritik bileşenlerden
        // biri bile çalışmıyorsa uygulama açılmaz (fail-fast). Denetimler
        // arka planda koşar; pencere donmaz, çökme olmaz.
        if !self.baslangic.gecti() {
            self.baslangic.basla();
            self.baslangic.isle();
            self.baslangic.onarim_isle();
            if !self.baslangic.bitti || self.baslangic.onarim_suruyor() {
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            self.baslangic_ekrani(ctx);
            return;
        }
        self.api_test_poll();
        self.api_senkron_poll();
        while let Ok(mesaj) = self.log_rx.try_recv() {
            match mesaj {
                Mesaj::Log(satir) => self.logs.push(satir),
                Mesaj::Ilerleme { tamamlanan, toplam } => {
                    self.ilerleme = Some((tamamlanan, toplam));
                }
                Mesaj::TaramaBitti(mut bulunanlar) => {
                    self.is_running = false;
                    self.ilerleme = None;
                    self.detay_idx = None;
                    // Tarama kotayı harcadı: gösterge hemen tazelensin.
                    let adet = bulunanlar.len();
                    // Kalıcı deftere işle: mükerrer kayıt açılmaz, daha önce
                    // temas edilmiş veya kara listedeki işletmenin durumu gelir.
                    let bugun = bugun_anahtari();
                    let mut bilinen = 0usize;
                    for lead in bulunanlar.iter_mut() {
                        let anahtar =
                            musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
                        let durum = self.defter.isle(
                            &anahtar,
                            &lead.name,
                            &lead.phone,
                            &lead.address,
                            &lead.category,
                            &lead.kaynak,
                            "yok",
                            &lead.site_linki,
                            potansiyel_puani(lead),
                            &bugun,
                        );
                        if durum != musteriler::Durum::Yeni {
                            bilinen += 1;
                        }
                        lead.durum = durum;
                        // "Mesaj hazır" artık defterdeki durumdan türetilir;
                        // yeniden taramada da kalır.
                        lead.mesaj_hazir = durum != musteriler::Durum::Yeni;
                        // Mesaj varyantı defterde tutulur; aynı işletmeye hep aynı
                        // varyant gider, böylece A/B ölçümü karışmaz.
                        lead.mesaj_varyanti = Some(self.defter.varyant(&anahtar));
                        // "2 Medya" seçimleri defterle senkron gelir; tarama
                        // listesi yenilense de işletmenin medyası korunur.
                        lead.medya = self
                            .defter
                            .kayit(&anahtar)
                            .map(|kayit| kayit.medya.clone())
                            .unwrap_or_default();
                    }
                    self.defter.kaydet();
                    incelemeye_gore_sirala(&mut bulunanlar);
                    let ozet = self.defter.ozet();
                    self.logs.push(format!(
                        "[i] Müşteri defteri: {} yeni kayıt, {bilinen} bilinen işletme · toplam {} kayıt ({} kara listede).",
                        adet.saturating_sub(bilinen),
                        ozet.toplam,
                        ozet.kara_liste
                    ));
                    self.leads = bulunanlar;
                    // 20 işletmelik günlük hedef listesi bu sonuçlardan kurulur.
                    self.hedef.temizle();
                    self.hedef_listesini_doldur();
                    self.bugun_panel_acik = true;
                    // Pinler yeni sonuçla birlikte yeniden düşsün.
                    let il = self.cities[self.selected_city_idx].0;
                    if il_geneline_gecilir(adet, self.il_geneli) {
                        // İlçede kayıt yok: kullanıcı beklemeden tüm ilçeler taranır.
                        let ilce = self.cities[self.selected_city_idx]
                            .1
                            .get(self.selected_district_idx)
                            .copied()
                            .unwrap_or("");
                        self.logs.push(format!(
                            "[i] {ilce} / {il} taramasında kayıt bulunamadı; il geneli taraması otomatik başlatıldı."
                        ));
                        self.il_geneli = true;
                        self.bos_ilce_uyari = None;
                        self.tarama_baslat();
                    } else if adet == 0 {
                        self.logs.push(format!(
                            "[i] {il} genelinde de web sitesi olmayan işletme bulunamadı; başka bir sektör deneyin."
                        ));
                        self.bos_ilce_uyari = Some(format!("{il} geneli"));
                    } else {
                        self.bos_ilce_uyari = None;
                        self.logs
                            .push(format!("[✓] {adet} işletme listeye eklendi."));
                    }
                }
                Mesaj::TaramaBasarisiz => {
                    self.is_running = false;
                    self.ilerleme = None;
                }
            }
        }
        self.gelen_olaylari_isle();
        self.ai_sonucu_al();
        self.domain_sonucu_al();
        // Katman çubuğu her katmanda görünür; WhatsApp freni orada okunur.
        self.katman_bari(ctx);
        // Medya doğrulaması arka planda sürerken sonucu bekleyen ekranın
        // tazelenmesini sağla; aksi halde sonuç ilk tıklamaya kadar görünmez.
        if self.medya_mesgul || self.medya_islem.is_some() {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
        // Köprü uygulama açıkken her zaman dinler: gelen mesaj, WhatsApp
        // katmanı açılmadan da kaydedilir (daemon davranışı).
        self.kopruyu_baslat();
        self.kopru_tazele();
        if self.katman == Katman::WhatsApp {
            self.ai_tazele(ctx);
            ctx.request_repaint_after(Duration::from_millis(400));
        }
        if self.katman != Katman::MusteriBulma {
            let gonderen = whatsapp_numarasi(&self.whatsapp_phone)
                .map(|numara| format!("+{numara}"))
                .unwrap_or_default();
            self.katman_ekrani(ctx, &gonderen);
            return;
        }

        // Tarama sürerken günlük ve ilerleme çubuğu kendiliğinden güncellensin;
        // eframe yalnızca olaylarda çizdiği için düzenli yeniden çizim istenir.
        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let gonderen = whatsapp_numarasi(&self.whatsapp_phone)
            .map(|numara| format!("+{numara}"))
            .unwrap_or_default();

        let il = self.cities[self.selected_city_idx].0;
        let ilce = self.cities[self.selected_city_idx].1[self.selected_district_idx];
        let sektor = self.sectors[self.selected_sector_idx];

        // --- Veri kaynağı şeridi ve WhatsApp freni ---------------------------
        self.api_paneli(ctx);
        // WhatsApp ayarları yerleşimi bozmasın diye ayrı pencerede açılır.
        if self.whatsapp_acik {
            let mut acik = true;
            egui::Window::new("WhatsApp Köprüsü")
                .open(&mut acik)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(392.0)
                .show(ctx, |ui| self.whatsapp_ayarlari(ui));
            if !acik {
                self.whatsapp_acik = false;
            }
        }

        // --- Alt panel: işlem günlüğü (footer'da sabit) ---------------------
        // Panel her zaman pencerenin en altında durur; üst kenarından
        // sürüklenerek yüksekliği değiştirilir, "Kompakt" ile tek satırlık
        // akış şeridine iner. Böylece hiçbir çözünürlükte ekran dışına taşmaz.
        // Ekran yüksekliğine göre sınır: panel hiçbir çözünürlükte ekranın
        // %42'sinden fazlasını kaplamaz ve alt kenardan kopmaz. Çok kısa
        // pencerelerde (ör. küçük çözünürlükte tam ekran) kendiliğinden
        // kompakt şeride iner; böylece konsol asla görünmez olmaz.
        let ekran_yuksekligi = ctx.screen_rect().height();
        let kompakt = self.gunluk_kompakt || ekran_yuksekligi < 560.0;
        let tavan = (ekran_yuksekligi * 0.42).clamp(150.0, 360.0);
        let taban = 96.0_f32.min(tavan);
        let gunluk_panel = if kompakt {
            egui::TopBottomPanel::bottom("gunluk_paneli").exact_height(42.0)
        } else {
            egui::TopBottomPanel::bottom("gunluk_paneli")
                .resizable(true)
                .default_height(168.0)
                .height_range(taban..=tavan)
        };
        gunluk_panel
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(18.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (nokta, _) =
                        ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(nokta.center(), 3.0, if self.is_running { YESIL } else { DIM });
                    bolum_etiketi(ui, "İŞLEM GÜNLÜĞÜ");
                    ui.label(
                        egui::RichText::new(format!("{} satır", self.logs.len()))
                            .size(10.0)
                            .color(DIM),
                    );
                    if self.is_running {
                        ui.add(egui::Spinner::new().size(10.0).color(VURGU));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hayalet_buton(
                            ui,
                            if kompakt {
                                "Aç"
                            } else {
                                "Kompakt"
                            },
                        )
                        .on_hover_text(
                            "Konsolu tek satırlık akış şeridine indirir; açıkken üst kenarından sürükleyerek yükseltin",
                        )
                        .clicked()
                        {
                            self.gunluk_kompakt = !kompakt;
                        }
                        ui.add_space(4.0);
                        if hayalet_buton(ui, "Temizle").clicked() {
                            self.logs.clear();
                        }
                    });
                });

                if kompakt {
                    ui.add_space(2.0);
                    let son = self
                        .logs
                        .last()
                        .cloned()
                        .unwrap_or_else(|| "Henüz işlem yok.".to_string());
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(8, 12, 17))
                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                        .rounding(egui::Rounding::same(7.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new(&son)
                                    .font(kod_font(11.0))
                                    .color(log_rengi(&son)),
                            );
                        });
                } else {
                    ui.add_space(5.0);
                    let gunluk_yuksekligi = (ui.available_height() - 4.0).max(28.0);
                    // Terminal görünümü: koyu zemin, ince kenar, renkli satırlar.
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(8, 12, 17))
                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::vertical()
                                .stick_to_bottom(true)
                                .max_height(gunluk_yuksekligi)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if self.logs.is_empty() {
                                        ui.label(
                                            egui::RichText::new("Henüz işlem yok.")
                                                .size(11.0)
                                                .color(DIM),
                                        );
                                    }
                                    ui.spacing_mut().item_spacing.y = 3.5;
                                    for log in &self.logs {
                                        ui.label(
                                            egui::RichText::new(log)
                                                .font(kod_font(11.5))
                                                .color(log_rengi(log)),
                                        );
                                    }
                                });
                        });
                }
            });

        // --- Sol sütun: Müşteri Bulma (arama parametreleri) -----------------
        // Üç bölge eşit: Müşteri Bulma · Arama · Sonuç listesi. Genişlikler
        // ekranın tam üçte biri; pencere büyüse de oran bozulmaz, orta sütun
        // kalanı alır. Böylece hiçbir sütun diğerini sıkıştırmaz.
        let bolge_genisligi = (ctx.screen_rect().width() / 3.0).floor();
        let sol_genislik = bolge_genisligi.max(300.0);
        egui::SidePanel::left("kesif_sol")
            .resizable(false)
            .exact_width(sol_genislik)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Müşteri Bulma")
                                .font(baslik_font(17.0))
                                .color(METIN),
                        );
                        let yer = if self.il_geneli {
                            format!("{il} · tüm ilçeler · {sektor}")
                        } else {
                            format!("{il} · {ilce} · {sektor}")
                        };
                        ui.label(egui::RichText::new(yer).size(10.5).color(DIM));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hayalet_buton(ui, "API")
                            .on_hover_text("Google API anahtarı ve kota yönetimi")
                            .clicked()
                        {
                            self.api_panel = true;
                            self.api_mesaj.clear();
                        }
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    bolum_etiketi(ui, "PARAMETRELER");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hayalet_buton(ui, "Sıfırla").clicked() {
                            self.selected_sector_idx = 0;
                            self.selected_district_idx = 0;
                            self.il_geneli = false;
                            self.bos_ilce_uyari = None;
                            self.city_search.clear();
                            self.district_search.clear();
                            self.sector_search.clear();
                        }
                    });
                });
                ui.label(
                    egui::RichText::new(format!(
                        "{} il · {} ilçe · {} sektör",
                        iller::ILLER.len(),
                        iller::ILCELER,
                        self.sectors.len()
                    ))
                    .size(10.5)
                    .color(DIM),
                );
                ui.add_space(8.0);

                // Parametreler: kalıcı il/ilçe listeleri kaldırıldı. Seçim,
                // arama kutusuna yazınca açılan en fazla 5 satırlık öneriyle
                // yapılır; böylece sektör ve tarama düğmesi her zaman görünür.
                filtre_basligi(ui, "İl", il);
                arama_kutusu(ui, &mut self.city_search, "İl ara");
                let il_sorgu = arama_anahtari(self.city_search.trim());
                if !il_sorgu.is_empty() && il_sorgu != arama_anahtari(il) {
                    let eslesenler: Vec<(usize, &'static str)> = self
                        .cities
                        .iter()
                        .enumerate()
                        .filter(|(_, (ad, _))| arama_anahtari(ad).contains(&il_sorgu))
                        .take(5)
                        .map(|(idx, (ad, _))| (idx, *ad))
                        .collect();
                    if eslesenler.is_empty() {
                        ui.label(egui::RichText::new("Eşleşen il yok").size(10.5).color(DIM));
                    } else {
                        let yukseklik = eslesenler.len() as f32 * 26.0;
                        let mut secilen = None;
                        secim_listesi(ui, "il_onerileri", yukseklik, None, |ui| {
                            for (idx, ad) in &eslesenler {
                                if liste_satiri(ui, self.selected_city_idx == *idx, ad).clicked() {
                                    secilen = Some((*idx, *ad));
                                }
                            }
                        });
                        if let Some((idx, ad)) = secilen {
                            if idx != self.selected_city_idx {
                                self.selected_city_idx = idx;
                                self.selected_district_idx = 0;
                                self.bos_ilce_uyari = None;
                                self.district_search.clear();
                            }
                            self.city_search = ad.to_string();
                        }
                    }
                }
                ui.add_space(12.0);

                if self.selected_district_idx >= self.cities[self.selected_city_idx].1.len() {
                    self.selected_district_idx = 0;
                }
                let ilce = self.cities[self.selected_city_idx].1[self.selected_district_idx];
                if filtre_basligi_ilce(ui, "İlçe", ilce, self.il_geneli) {
                    self.il_geneli = !self.il_geneli;
                    self.bos_ilce_uyari = None;
                }
                arama_kutusu(ui, &mut self.district_search, "İlçe ara");
                let ilce_sorgu = arama_anahtari(self.district_search.trim());
                if !ilce_sorgu.is_empty() && ilce_sorgu != arama_anahtari(ilce) {
                    let eslesenler: Vec<(usize, &'static str)> = self.cities
                        [self.selected_city_idx]
                        .1
                        .iter()
                        .enumerate()
                        .filter(|(_, ad)| arama_anahtari(ad).contains(&ilce_sorgu))
                        .take(5)
                        .map(|(idx, ad)| (idx, *ad))
                        .collect();
                    if eslesenler.is_empty() {
                        ui.label(
                            egui::RichText::new("Eşleşen ilçe yok")
                                .size(10.5)
                                .color(DIM),
                        );
                    } else {
                        let yukseklik = eslesenler.len() as f32 * 26.0;
                        let mut secilen = None;
                        secim_listesi(ui, "ilce_onerileri", yukseklik, None, |ui| {
                            for (idx, ad) in &eslesenler {
                                let secili = !self.il_geneli && self.selected_district_idx == *idx;
                                if liste_satiri(ui, secili, ad).clicked() {
                                    secilen = Some((*idx, *ad));
                                }
                            }
                        });
                        if let Some((idx, ad)) = secilen {
                            self.selected_district_idx = idx;
                            self.il_geneli = false;
                            self.bos_ilce_uyari = None;
                            self.district_search = ad.to_string();
                        }
                    }
                }
                ui.add_space(12.0);

                // Sektör: seçim iki sütunlu panelde yapılır
                filtre_basligi(ui, "Sektör", sektor);
                if sektor_secici(ui, sektor).clicked() {
                    self.sektor_panel_acik = true;
                    self.sector_search.clear();
                    let grup = sektor_grubu(sektor);
                    self.sektor_panel_grup = sektor_gruplari()
                        .iter()
                        .position(|ad| *ad == grup)
                        .unwrap_or(0);
                }
                ui.add_space(10.0);

                if hayalet_buton(ui, "WhatsApp köprüsü")
                    .on_hover_text("Numara, bağlantı durumu ve otomatik yanıt ayarları")
                    .clicked()
                {
                    self.whatsapp_acik = true;
                }
                ui.add_space(8.0);
                let tara_genislik = ui.available_width();
                let tara = birincil_buton(
                    ui,
                    if self.is_running {
                        "Taranıyor..."
                    } else if self.il_geneli {
                        "İl genelini tara"
                    } else {
                        "İşletmeleri tara"
                    },
                    !self.is_running,
                    Some(egui::vec2(tara_genislik, 42.0)),
                );
                if tara.clicked() {
                    self.tarama_baslat();
                }
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(if self.il_geneli {
                            "Tüm ilçeler sırayla · en fazla 20 kayıt"
                        } else {
                            "Sorgu başına en fazla 20 kayıt"
                        })
                        .size(10.0)
                        .color(DIM),
                    );
                });
            });

        // --- Sağ panel: işletme detayı ---------------------------------------
        self.detay_paneli(ctx, &gonderen);
        self.sektor_paneli(ctx);
        self.takip_paneli(ctx);
        self.bugun_listesi_paneli(ctx, &gonderen);

        let secili = self.leads.iter().filter(|lead| lead.selected).count();
        // --- Orta sütun: arama ve sonuç listesi (iç kaydırma) ---------------
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    bolum_etiketi(ui, "ARAMA");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} sonuç · {} seçili",
                                self.leads.len(),
                                secili
                            ))
                            .size(10.0)
                            .color(DIM),
                        );
                    });
                });
                ui.add_space(4.0);
                self.kesif_sonuclari(ui);
            });
    }
}

/// İlerleme oranı: toplam sıfırsa 0, aksi halde 0..=1 aralığına kırpılır.
fn ilerleme_orani(tamamlanan: usize, toplam: usize) -> f32 {
    if toplam == 0 {
        return 0.0;
    }
    (tamamlanan as f32 / toplam as f32).clamp(0.0, 1.0)
}

/// İl geneli taramasının ilçe ilerlemesi: ince çubuk ve "12/39" sayacı.
fn ilerleme_cubugu(ui: &mut egui::Ui, tamamlanan: usize, toplam: usize) {
    let oran = ilerleme_orani(tamamlanan, toplam);
    let genislik = ui.available_width();

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "İl geneli tarama · {tamamlanan}/{} ilçe",
                toplam.max(1)
            ))
            .size(11.0)
            .color(SOLUK),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("%{}", (oran * 100.0).round() as i32))
                    .size(11.0)
                    .color(VURGU),
            );
        });
    });
    ui.add_space(5.0);

    let (rect, _) = ui.allocate_exact_size(egui::vec2(genislik, 6.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, egui::Rounding::same(3.0), GIRINTI);
    if oran > 0.0 {
        let dolu = egui::Rect::from_min_size(
            rect.min,
            egui::vec2((rect.width() * oran).max(6.0), rect.height()),
        );
        p.rect_filled(dolu, egui::Rounding::same(3.0), VURGU);
    }
}

/// Boş sonuç alanı: büyük ikon, kısa yönlendirme ve üç adımlı ipucu.
fn bos_durum(
    ui: &mut egui::Ui,
    taranıyor: bool,
    uyari: Option<(&str, &str)>,
    il_geneli: bool,
    il_geneli_baslat: &mut bool,
) {
    ui.add_space(46.0);
    ui.vertical_centered(|ui| {
        let (daire, _) = ui.allocate_exact_size(egui::vec2(76.0, 76.0), egui::Sense::hover());
        let merkez = daire.center();
        let p = ui.painter();
        p.circle_filled(merkez, 38.0, KART);
        p.circle_stroke(merkez, 37.5, egui::Stroke::new(1.0_f32, KENAR));
        p.circle_stroke(
            merkez + egui::vec2(-3.0, -3.0),
            11.0,
            egui::Stroke::new(2.0_f32, SOLUK),
        );
        p.line_segment(
            [
                merkez + egui::vec2(5.0, 5.0),
                merkez + egui::vec2(13.0, 13.0),
            ],
            egui::Stroke::new(2.0_f32, SOLUK),
        );

        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(if taranıyor {
                "İşletmeler taranıyor"
            } else if uyari.is_some() && il_geneli {
                "İl genelinde kayıt çıkmadı"
            } else if uyari.is_some() {
                "Bu ilçede kayıt çıkmadı"
            } else {
                "Taramaya hazır"
            })
            .font(baslik_font(17.0))
            .color(METIN),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(if taranıyor {
                "Google Places kayıtları çekiliyor, sonuçlar birazdan listelenecek."
            } else if uyari.is_some() && il_geneli {
                "Tüm ilçeler tarandı; bu sektörde web sitesi olmayan işletme kaydı yok.\nBaşka bir sektör seçmek daha verimli olur."
            } else if uyari.is_some() {
                "Kayıt az: bu sektör ilçede tek tük kayıtlı olabilir.\nİl geneli taraması tüm ilçeleri sırayla gezer."
            } else {
                "Soldan il, ilçe ve sektör seçip “İşletmeleri tara” butonuna basın.\nListede yalnızca telefonu olan ve web sitesi olmayan gerçek işletmeler görünür."
            })
            .size(11.5)
            .color(SOLUK),
        );

        if let Some((ilce, sektor)) = uyari.filter(|_| !taranıyor) {
            ui.add_space(16.0);
            let kart_genislik = 470.0_f32.min(ui.available_width() - 24.0);
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - kart_genislik) / 2.0).max(0.0));
                egui::Frame::none()
                    .fill(ton(VURGU, 0.08))
                    .stroke(egui::Stroke::new(1.0_f32, ton(VURGU, 0.45)))
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                    .show(ui, |ui| {
                        ui.set_width(kart_genislik - 32.0);
                        ui.label(
                            egui::RichText::new(format!("{ilce} · {sektor}"))
                                .font(baslik_font(12.5))
                                .color(VURGU),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            egui::RichText::new(if il_geneli {
                                "İl geneli taraması otomatik yapıldı ve sonuç boş döndü. Bu sektör OSM'de seyrek etiketlenmiş olabilir."
                            } else {
                                "Bu ilçede web sitesi olmayan kayıt dönmedi. Sektör burada seyrek olabilir; il genelinde tarayınca komşu ilçeler de taranır."
                            })
                            .size(11.0)
                            .color(SOLUK),
                        );
                        if !il_geneli {
                            ui.add_space(9.0);
                            if ikincil_buton(
                                ui,
                                "İl genelinde tara",
                                true,
                                Some(egui::vec2(168.0, 30.0)),
                            )
                            .clicked()
                            {
                                *il_geneli_baslat = true;
                            }
                        }
                    });
            });
        }

    });
}

/// Başlık satırının sağındaki, tıklanabilir seçim çipi.
fn filtre_cipi(ui: &mut egui::Ui, metin: &str, secili: bool) -> egui::Response {
    let boy = egui::vec2(metin_genisligi(ui, metin, 10.5) + 22.0, 22.0);
    let (rect, yanit) = ui.allocate_exact_size(boy, egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(yanit.id, yanit.hovered(), 0.10);
    let (dolgu, kenar, yazi) = if secili {
        (ton(VURGU, 0.12), ton(VURGU, 0.50), koyu(VURGU, 0.15))
    } else {
        (
            KART,
            karistir(KENAR, KENAR_GUCLU, t),
            karistir(SOLUK, METIN, t),
        )
    };
    let p = ui.painter();
    p.rect_filled(rect, egui::Rounding::same(99.0), dolgu);
    p.rect_stroke(
        rect,
        egui::Rounding::same(99.0),
        egui::Stroke::new(1.0_f32, kenar),
    );
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        metin,
        govde_font(10.5),
        yazi,
    );
    if yanit.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    yanit
}

/// İlçe başlığı: sağda seçili ilçe ve "Tüm ilçeler" çipi. Çipe tıklanınca
/// `true` döner (il geneli aç/kapa).
fn filtre_basligi_ilce(ui: &mut egui::Ui, baslik: &str, deger: &str, il_geneli: bool) -> bool {
    let mut tiklanti = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(baslik).size(11.0).color(SOLUK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if filtre_cipi(ui, "Tüm ilçeler", il_geneli).clicked() {
                tiklanti = true;
            }
            if !il_geneli {
                ui.add_space(8.0);
                let kalan = ui.available_width().max(40.0);
                ui.label(
                    egui::RichText::new(kisalt_metin(ui, deger, kalan - 4.0, 11.0))
                        .font(baslik_font(11.0))
                        .color(VURGU),
                );
            }
        });
    });
    ui.add_space(5.0);
    tiklanti
}

fn filtre_basligi(ui: &mut egui::Ui, baslik: &str, deger: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(baslik).size(11.0).color(SOLUK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let kalan = ui.available_width().max(40.0);
            ui.label(
                egui::RichText::new(kisalt_metin(ui, deger, kalan - 4.0, 11.0))
                    .font(baslik_font(11.0))
                    .color(VURGU),
            );
        });
    });
    ui.add_space(5.0);
}

fn secim_listesi(
    ui: &mut egui::Ui,
    kimlik: &str,
    yukseklik: f32,
    kaydirma_ofseti: Option<f32>,
    icerik: impl FnOnce(&mut egui::Ui),
) {
    // Liste her zaman TAM `yukseklik` kadar yer ayırır. Önceden yükseklik
    // yalnız `max_height` ile sınırlanıyordu; arama sonucu 1 satıra düşünce
    // kutu kısalıp altındaki bölümler yukarı kayıyor, iç içe kaydırma
    // alanında da bir sonraki kutuyla üst üste binebiliyordu.
    let genislik = ui.available_width();
    let dis_yukseklik = yukseklik + 10.0;
    ui.allocate_ui_with_layout(
        egui::vec2(genislik, dis_yukseklik),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(genislik);
            ui.set_height(dis_yukseklik);
            egui::Frame::none()
                .fill(GIRINTI)
                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                .rounding(egui::Rounding::same(10.0))
                .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let mut alan = egui::ScrollArea::vertical()
                        .id_source(kimlik)
                        .max_height(yukseklik)
                        .min_scrolled_height(yukseklik)
                        .auto_shrink([false, false]);
                    if let Some(ofset) = kaydirma_ofseti {
                        alan = alan.vertical_scroll_offset(ofset);
                    }
                    alan.show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 1.0;
                        icerik(ui);
                    });
                });
        },
    );
}
// ---------------------------------------------------------------------------
// Veri kaynağı: Google Places API (New) + Geocoding
//
// İl/ilçe merkezi Google Geocoding'den, işletmeler Google Places API (New)
// Text Search'ten okunur. Anahtar şifreli kasada tutulur; her istek kotadan
// pay ayırır, aynı ilçe tekrar taranırken sonuç önbellekten gelir.
// ---------------------------------------------------------------------------

struct Konum {
    enlem: f64,
    boylam: f64,
    /// Nominatim sınır kutusundan türetilen kapsama yarıçapı.
    yaricap_m: u32,
}

/// Önbellekte saklanan konum kaydı.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct KonumKaydi {
    enlem: f64,
    boylam: f64,
    yaricap_m: u32,
    /// Önbelleğe alındığı tarih (YYYY-MM-DD).
    tarih: String,
}

const KONUM_ONBELLEK_DOSYASI: &str = "konum_onbellek.json";
/// Konumun kaç gün geçerli sayılacağı; ilçe sınırları yıllarca değişmez.
const KONUM_TTL_GUN: i64 = 180;

const SONUC_ONBELLEK_DOSYASI: &str = "sonuc_onbellek.json";

/// Sonuç önbelleğinin varsayılan geçerlilik süresi (gün). `SONUC_TTL_GUN=0`
/// önbelleği tamamen kapatır, böylece her tarama canlı veri getirir.
const SONUC_TTL_GUN: i64 = 7;

fn sonuc_ttl_gun() -> i64 {
    std::env::var("SONUC_TTL_GUN")
        .ok()
        .and_then(|deger| deger.trim().parse::<i64>().ok())
        .unwrap_or(SONUC_TTL_GUN)
        .max(0)
}

/// Bir ilçe + sektör için saklanan arama sonucu.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SonucKaydi {
    tarih: String,
    siteli: usize,
    leads: Vec<BusinessLead>,
}

/// Önbellekten dönen kayıt. `gun` = kaç gün önce çekildiği.
struct SonucOnbellekKaydi {
    leads: Vec<BusinessLead>,
    gun: i64,
}

/// İlçe + sektör başına tarama sonuçları. Aynı ilçe kısa süre içinde tekrar
/// tarandığında Google'a ikinci istek atılmaz.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct SonucOnbellek {
    #[serde(default)]
    kayitlar: HashMap<String, SonucKaydi>,
}

impl SonucOnbellek {
    fn yukle() -> Self {
        fs::read_to_string(SONUC_ONBELLEK_DOSYASI)
            .ok()
            .and_then(|metin| serde_json::from_str(&metin).ok())
            .unwrap_or_default()
    }

    fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string(self) {
            let _ = fs::write(SONUC_ONBELLEK_DOSYASI, metin);
        }
    }

    fn anahtar(il: &str, ilce: &str, sektor: &str) -> String {
        format!(
            "{}|{}|{}",
            arama_anahtari(il),
            arama_anahtari(ilce),
            arama_anahtari(sektor)
        )
    }

    /// TTL içindeki kayıt; yoksa veya eskimişse `None`.
    fn al(&self, il: &str, ilce: &str, sektor: &str) -> Option<SonucOnbellekKaydi> {
        let ttl = sonuc_ttl_gun();
        if ttl == 0 {
            return None;
        }
        let kayit = self.kayitlar.get(&Self::anahtar(il, ilce, sektor))?;
        let gun = gun_farki(&kayit.tarih, &bugun_anahtari())?;
        if gun < 0 || gun > ttl {
            return None;
        }
        // Eski önbellek kayıtları web sitesi olan işletmeler içerebilir; bu
        // ekranın kuralı gereği yalnızca sitesiz kayıtlar geri döner.
        let leads: Vec<BusinessLead> = kayit
            .leads
            .iter()
            // `kaynak` her Google kaydında Maps URL'sidir; işletmenin kendi
            // sitesi `site_linki` alanındadır. Maps kaydı site sayılmaz.
            .filter(|lead| {
                let site = lead.site_linki.trim();
                site.is_empty() || site.to_lowercase().contains("google.com")
            })
            .cloned()
            .collect();
        Some(SonucOnbellekKaydi { leads, gun })
    }

    fn ekle(&mut self, il: &str, ilce: &str, sektor: &str, leads: &[BusinessLead], siteli: usize) {
        self.kayitlar.insert(
            Self::anahtar(il, ilce, sektor),
            SonucKaydi {
                tarih: bugun_anahtari(),
                siteli,
                leads: leads.to_vec(),
            },
        );
    }
}

/// İlçe → konum önbelleği. İl geneli taramada aynı ilçeler tekrar tekrar
/// çözümlenmesin diye `konum_onbellek.json` dosyasında tutulur.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct KonumOnbellek {
    #[serde(default)]
    kayitlar: HashMap<String, KonumKaydi>,
}

impl KonumOnbellek {
    fn yukle() -> Self {
        fs::read_to_string(KONUM_ONBELLEK_DOSYASI)
            .ok()
            .and_then(|metin| serde_json::from_str(&metin).ok())
            .unwrap_or_default()
    }

    fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string_pretty(self) {
            let _ = fs::write(KONUM_ONBELLEK_DOSYASI, metin);
        }
    }

    fn anahtar(il: &str, ilce: &str) -> String {
        format!("{}|{}", arama_anahtari(il), arama_anahtari(ilce))
    }

    /// Önbellekteki konum; `KONUM_TTL_GUN` günden eskiyse yok sayılır.
    fn al(&self, il: &str, ilce: &str) -> Option<Konum> {
        let kayit = self.kayitlar.get(&Self::anahtar(il, ilce))?;
        let gun = gun_farki(&kayit.tarih, &bugun_anahtari())?;
        if gun > KONUM_TTL_GUN {
            return None;
        }
        Some(Konum {
            enlem: kayit.enlem,
            boylam: kayit.boylam,
            yaricap_m: kayit.yaricap_m,
        })
    }

    fn ekle(&mut self, il: &str, ilce: &str, konum: &Konum) {
        self.kayitlar.insert(
            Self::anahtar(il, ilce),
            KonumKaydi {
                enlem: konum.enlem,
                boylam: konum.boylam,
                yaricap_m: konum.yaricap_m,
                tarih: bugun_anahtari(),
            },
        );
    }
}

fn http_ajani() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent("NexusEnterprise/0.1 (Windows masaustu isletme kesif uygulamasi)")
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(90))
        .build()
}

fn url_encode(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len() * 3);
    for &bayt in metin.as_bytes() {
        match bayt {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                cikti.push(bayt as char);
            }
            _ => cikti.push_str(&format!("%{bayt:02X}")),
        }
    }
    cikti
}

/// Kalıcı uygulama ayarları: her açılışta sıfırlanmaması gereken değerler.
const AYAR_DOSYASI: &str = "ayarlar.json";

/// Serde'nin eksik alan varsayılanı: bölümler yazılmamış eski ayar
/// dosyalarında da görünür kalsın diye `true` döner.
fn evet() -> bool {
    true
}

/// Google istek bütçesi varsayılanları: günlük 20 istek, aylık 400 istek.
/// Aylık 200 $ ücretsiz kredinin çok altında kalır (10.000 istek ≈ tüm kredi).
fn google_gunluk_varsayilan() -> usize {
    crate::google::VARSAYILAN_GUNLUK
}

fn google_aylik_varsayilan() -> usize {
    crate::google::VARSAYILAN_AYLIK
}

fn google_sayfa_varsayilan() -> usize {
    crate::google::VARSAYILAN_SAYFA
}

/// Yeni ve eski ayar dosyalarında varsayılan motor ajans profilidir.
fn profil_varsayilan() -> String {
    "ajans".to_string()
}

/// Sinematik profilin varsayılan hareket yoğunluğu.
fn sinematik_hareket_varsayilan() -> String {
    "orta".to_string()
}

/// Sinematik profilin varsayılan 3D nesnesi: sektöre göre otomatik seçim.
fn sinematik_nesne_varsayilan() -> String {
    "otomatik".to_string()
}

/// Sinematik profilin varsayılan sahne sayısı.
fn sinematik_sahne_varsayilan() -> usize {
    4
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct UygulamaAyari {
    /// Demo altbilgisinde ve mesajlarda görünen gönderen WhatsApp numarası.
    #[serde(default)]
    whatsapp_phone: String,
    /// Demo renklendirmesi (#rrggbb). Boş bırakılan alan otomatik seçilir:
    /// vurgu sektör renginden, zemin/metin varsayılan koyu temadan gelir.
    #[serde(default)]
    demo_vurgu: String,
    #[serde(default)]
    demo_zemin: String,
    #[serde(default)]
    demo_yazi: String,
    /// Demo bölümleri: kapatılan bölüm sayfada hiç çizilmez.
    #[serde(default = "evet")]
    demo_galeri: bool,
    #[serde(default = "evet")]
    demo_yorumlar: bool,
    #[serde(default = "evet")]
    demo_harita: bool,
    #[serde(default = "evet")]
    demo_saatler: bool,
    /// Marka cümlesi ve buton metni: boşsa sektör metni kullanılır.
    #[serde(default)]
    demo_slogan: String,
    #[serde(default)]
    demo_cta: String,
    /// Köşe biçimi: "keskin", "yumusak" veya boş (dengeli).
    #[serde(default)]
    demo_kose: String,
    /// Yerleşim teması: boşsa klasik çok bölümlü sayfa, "gece" ise tek ekran.
    #[serde(default)]
    demo_tema: String,
    /// Üretim motoru: "ajans" (varsayılan), "klasik" veya "sinematik".
    #[serde(default = "profil_varsayilan")]
    demo_profil: String,
    /// Sinematik profil: hareket yoğunluğu (dusuk/orta/yuksek).
    #[serde(default = "sinematik_hareket_varsayilan")]
    demo_sinematik_hareket: String,
    /// Sinematik profil: 3D nesne tipi (otomatik veya elle seçim).
    #[serde(default = "sinematik_nesne_varsayilan")]
    demo_sinematik_nesne: String,
    /// Sinematik profil: sahne sayısı (3–6).
    #[serde(default = "sinematik_sahne_varsayilan")]
    demo_sinematik_sahne: usize,
    /// İkincil buton metni: boşsa "Konumu gör"/"İletişim".
    #[serde(default)]
    demo_ikincil: String,
    /// Panelden yazılan metinler: boş alan sektör şablonundan gelir.
    #[serde(default)]
    demo_manset1: String,
    #[serde(default)]
    demo_manset2: String,
    #[serde(default)]
    demo_hakkinda_baslik: String,
    #[serde(default)]
    demo_hakkinda1: String,
    #[serde(default)]
    demo_hakkinda2: String,
    #[serde(default)]
    demo_bolum_etiket: String,
    #[serde(default)]
    demo_bolum_baslik: String,
    #[serde(default)]
    demo_bolum_alt: String,
    /// Dört liste satırı ve iki yorum: (metin, açıklama/isim).
    #[serde(default)]
    demo_kalemler: Vec<(String, String)>,
    #[serde(default)]
    demo_yorum_metinleri: Vec<(String, String)>,
    /// Fotoğraflar: hero kaynağı ("oto"/"yok"/"dosya"), kendi dosyası,
    /// galeri kaynağı ("oto"/"ekle"/"oto+ekle"/"yok"), kare sayısı ve
    /// eklenen dosyaların yolları. Yollar ayarda tutulur, görüntü değil:
    /// base64 gömülseydi ayar dosyası her fotoğrafla megabaytlara çıkardı.
    #[serde(default)]
    demo_hero_foto: String,
    #[serde(default)]
    demo_hero_dosya: String,
    #[serde(default)]
    demo_galeri_kaynak: String,
    #[serde(default = "galeri_varsayilan_adet")]
    demo_galeri_adet: usize,
    #[serde(default)]
    demo_fotolar: Vec<String>,
    /// Google API istek bütçesi: günlük sınır ve aylık sınır.
    #[serde(default = "google_gunluk_varsayilan")]
    google_gunluk_limit: usize,
    #[serde(default = "google_aylik_varsayilan")]
    google_aylik_limit: usize,
    /// Tarama başına okunacak sayfa sayısı (1-3 · sayfa başına 20 kayıt).
    #[serde(default = "google_sayfa_varsayilan")]
    google_sayfa: usize,
    /// Google Cloud proje kimliği: Cloud Monitoring gerçek kullanım sorgusu.
    #[serde(default)]
    google_proje: String,
}

impl Default for UygulamaAyari {
    fn default() -> Self {
        Self {
            whatsapp_phone: String::new(),
            demo_vurgu: String::new(),
            demo_zemin: String::new(),
            demo_yazi: String::new(),
            demo_galeri: true,
            demo_yorumlar: true,
            demo_harita: true,
            demo_saatler: true,
            demo_slogan: String::new(),
            demo_cta: String::new(),
            demo_kose: String::new(),
            demo_tema: String::new(),
            demo_profil: profil_varsayilan(),
            demo_sinematik_hareket: sinematik_hareket_varsayilan(),
            demo_sinematik_nesne: sinematik_nesne_varsayilan(),
            demo_sinematik_sahne: sinematik_sahne_varsayilan(),
            demo_ikincil: String::new(),
            demo_manset1: String::new(),
            demo_manset2: String::new(),
            demo_hakkinda_baslik: String::new(),
            demo_hakkinda1: String::new(),
            demo_hakkinda2: String::new(),
            demo_bolum_etiket: String::new(),
            demo_bolum_baslik: String::new(),
            demo_bolum_alt: String::new(),
            demo_kalemler: Vec::new(),
            demo_yorum_metinleri: Vec::new(),
            demo_hero_foto: String::new(),
            demo_hero_dosya: String::new(),
            demo_galeri_kaynak: String::new(),
            demo_galeri_adet: 3,
            demo_fotolar: Vec::new(),
            google_gunluk_limit: google_gunluk_varsayilan(),
            google_aylik_limit: google_aylik_varsayilan(),
            google_sayfa: google_sayfa_varsayilan(),
            google_proje: String::new(),
        }
    }
}

/// Galeride gösterilecek en fazla kare: sayfa şişmesin, ızgara bozulmasın.
const GALERI_AZAMI: usize = 6;

fn galeri_varsayilan_adet() -> usize {
    3
}

/// Panel listelerini sabit uzunluğa getirir: eksik satır boş gelir (otomatik),
/// fazlası atılır. Ayar dosyası elle düzenlense de panel çökmemeli.
fn satirlari_tamamla(mut satirlar: Vec<(String, String)>, adet: usize) -> Vec<(String, String)> {
    satirlar.truncate(adet);
    while satirlar.len() < adet {
        satirlar.push((String::new(), String::new()));
    }
    satirlar
}

/// Renklendirme paneli kapalıyken kullanılan varsayılan demo renkleri.
const DEMO_ZEMIN: &str = "#08080a";
const DEMO_YAZI: &str = "#f5f3ef";

/// Renklendirme panelindeki hazır paletler: (ad, zemin, yazı, vurgu).
/// Vurgu boş bırakılan palette sektör rengi kullanılır.
const DEMO_PALETLER: [(&str, &str, &str, &str); 7] = [
    ("Sektör rengi", "#08080a", "#f5f3ef", ""),
    ("Gece mavisi", "#0a1020", "#eef2ff", "#5b8cff"),
    ("Orman", "#08130f", "#eef7f1", "#3ecf8e"),
    ("Bordo", "#140a0d", "#f7eef0", "#e0577a"),
    ("Fildişi", "#f6f1e7", "#1b1a17", "#b4551f"),
    ("İs ve mangal", "#0e0d0c", "#e8dfd1", "#7a2e1e"),
    ("Fildişi ve pirinç", "#fcfbf9", "#2c221e", "#d4af37"),
];

fn ayar_oku_yoldan(yol: &str) -> UygulamaAyari {
    fs::read_to_string(yol)
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_default()
}

fn ayar_yaz_yoldan(yol: &str, ayar: &UygulamaAyari) -> std::io::Result<()> {
    let metin = serde_json::to_string_pretty(ayar)
        .map_err(|hata| std::io::Error::other(hata.to_string()))?;
    fs::write(yol, metin)
}

fn ayar_oku() -> UygulamaAyari {
    ayar_oku_yoldan(AYAR_DOSYASI)
}

/// Ayar dosyasındaki numaradan başlangıç durumunu üretir: gösterilecek
/// numara, durum satırı ve "kayıtlı" bayrağı. Kayıtlı numara yoksa alan boş
/// kalır; uydurma bir numara demo altbilgisine yazılmaz.
fn whatsapp_baslangic(ayar: &UygulamaAyari) -> (String, String, bool) {
    let numara = whatsapp_numarasi(&ayar.whatsapp_phone);
    let durum = match &numara {
        Some(numara) => format!("Kayıtlı: +{numara}"),
        None => "Numara kaydedilmedi".to_string(),
    };
    (ayar.whatsapp_phone.clone(), durum, numara.is_some())
}

fn ayar_yaz(ayar: &UygulamaAyari) {
    let _ = ayar_yaz_yoldan(AYAR_DOSYASI, ayar);
}

fn bugun_anahtari() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Kaydın kaynak bağlantısı: Google Maps kaydı.
fn kaynak_etiketi(_kaynak: &str) -> &'static str {
    "Google kaydını aç ↗"
}

/// Demo sayfasında gösterilecek bölüm ailesi.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Demo {
    Menu,
    Kafe,
    Urun,
    Klinik,
    Ofis,
    Servis,
    Eglence,
}

/// Tek adlı işletme sektörü: OSM etiket karşılığı ve demo şablonu.
pub(crate) struct Sektor {
    pub(crate) ad: &'static str,
    /// Arayüzde listenin bölümlendiği grup başlığı.
    pub(crate) grup: &'static str,
    pub(crate) demo: Demo,
}

/// Yerel arama hacmi ve kazanç potansiyeli yüksek, seçilmiş işletme
/// sektörleri. Liste bilinçli olarak kısa tutulur: her satır tek bir
/// sektördür ve arşiv `media/stock/<slug>/` altında aynı sırayla kurulur.
pub(crate) const SEKTORLER: &[Sektor] = &[
    // --- Lokanta & Kebap ---
    Sektor {
        ad: "Restoran",
        grup: "Lokanta & Kebap",
        demo: Demo::Menu,
    },
    // --- Kafe & Çay ---
    Sektor {
        ad: "Kafe",
        grup: "Kafe & Çay",
        demo: Demo::Kafe,
    },
    // --- Oto Servis ---
    // Yüksek aciliyet: 7/24 yol yardımı aramaları.
    Sektor {
        ad: "Çekici",
        grup: "Oto Servis",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Oto Kurtarma",
        grup: "Oto Servis",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Oto Servis",
        grup: "Oto Servis",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Oto Yıkama",
        grup: "Oto Servis",
        demo: Demo::Servis,
    },
    // --- Güzellik & Bakım ---
    // Yerel aramada en güçlü hizmet grupları.
    Sektor {
        ad: "Kuaför",
        grup: "Güzellik & Bakım",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Güzellik Salonu",
        grup: "Güzellik & Bakım",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Berber",
        grup: "Güzellik & Bakım",
        demo: Demo::Servis,
    },
    // --- Tesisat & Güvenlik ---
    // Sezonluk ama tekrar eden yüksek marjlı servis.
    Sektor {
        ad: "Klima Servisi",
        grup: "Tesisat & Güvenlik",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Kombi Servisi",
        grup: "Tesisat & Güvenlik",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Tesisatçı",
        grup: "Tesisat & Güvenlik",
        demo: Demo::Servis,
    },
    // --- Lojistik & Seyahat ---
    // Şehir içi/şehirler arası taşıma.
    Sektor {
        ad: "Nakliyat",
        grup: "Lojistik & Seyahat",
        demo: Demo::Servis,
    },
    Sektor {
        ad: "Evden Eve Nakliyat",
        grup: "Lojistik & Seyahat",
        demo: Demo::Servis,
    },
    // --- Sağlık Kuruluşları ---
    // Yüksek sepet tutarı, güçlü yerel niyet.
    Sektor {
        ad: "Diş Kliniği",
        grup: "Sağlık Kuruluşları",
        demo: Demo::Klinik,
    },
    // --- Yapı & Emlak ---
    // Danışmanlık ve satış odaklı ofis demoları.
    Sektor {
        ad: "Emlak",
        grup: "Yapı & Emlak",
        demo: Demo::Ofis,
    },
    // --- Hukuk & Mali ---
    Sektor {
        ad: "Avukat",
        grup: "Hukuk & Mali",
        demo: Demo::Ofis,
    },
    Sektor {
        ad: "Muhasebeci",
        grup: "Hukuk & Mali",
        demo: Demo::Ofis,
    },
    // --- Mekân & Etkinlik ---
    // Tek seferlik yüksek bütçeli rezervasyonlar.
    Sektor {
        ad: "Düğün Salonu",
        grup: "Mekân & Etkinlik",
        demo: Demo::Eglence,
    },
    // --- Temizlik & Bakım ---
    // Adrese teslim, abonelik benzeri tekrar.
    Sektor {
        ad: "Halı Yıkama",
        grup: "Temizlik & Bakım",
        demo: Demo::Servis,
    },
];

/// Sektörün arayüzdeki grup başlığı.
fn sektor_grubu(sektor: &str) -> &'static str {
    SEKTORLER
        .iter()
        .find(|tanim| tanim.ad == sektor)
        .map(|tanim| tanim.grup)
        .unwrap_or("Diğer")
}

/// Sektörün demo şablon ailesi; tanımsız ada düşülürse perakende şablonu.
fn demo_turu(sektor: &str) -> Demo {
    SEKTORLER
        .iter()
        .find(|tanim| tanim.ad == sektor)
        .map(|tanim| tanim.demo)
        .unwrap_or(Demo::Urun)
}

/// Tek ilçe taraması boş döndüyse il geneli taraması otomatik başlatılır;
/// il geneli de boşsa tekrar denenmez (aynı tarama döngüye girmesin).
fn il_geneline_gecilir(adet: usize, il_geneli: bool) -> bool {
    adet == 0 && !il_geneli
}

// ---------------------------------------------------------------------------
// Demo üretimi ve WhatsApp (wa.me) hazırlığı
//
// WhatsApp'ta toplu otomatik gönderim, resmî olmayan otomasyon kullandığı için
// numarayı askıya aldırır. Burada ücretsiz ve güvenli yol kullanılır: her
// işletme için gerçek bir HTML demo yazılır, wa.me bağlantısı mesaj hazır
// şekilde açılır; göndermeyi kullanıcı onaylar.
// ---------------------------------------------------------------------------

const DEMO_KLASORU: &str = "demos";
/// Sorgu başına listeye giren en fazla kayıt sayısı.
const SONUC_SINIRI: usize = 20;

fn html_kacis(metin: &str) -> String {
    metin
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn dosya_slug(metin: &str) -> String {
    let mut ham = String::new();
    for karakter in metin.chars() {
        let kucuk = karakter.to_lowercase().next().unwrap_or(karakter);
        ham.push(match kucuk {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            'a'..='z' | '0'..='9' => kucuk,
            _ => '-',
        });
    }
    let slug = ham
        .split('-')
        .filter(|parca| !parca.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "isletme".to_string()
    } else {
        slug.chars().take(60).collect()
    }
}

/// Numarayı wa.me biçimine çevirir; sabit hat ise WhatsApp'ta hesap yoktur.
fn mobil_numara(ham: &str) -> bool {
    let rakamlar: String = ham
        .chars()
        .filter(|karakter| karakter.is_ascii_digit())
        .collect();
    // Türkiye mobil numaraları +90 5XX XXX XX XX biçimindedir.
    let yerel = rakamlar
        .strip_prefix("90")
        .or_else(|| rakamlar.strip_prefix('0'))
        .unwrap_or(&rakamlar);
    yerel.len() == 10 && yerel.starts_with('5')
}

/// "\+90 532 123 45 67", "0532..." ve "532..." biçimlerini wa.me numarasına çevirir.
fn whatsapp_numarasi(ham: &str) -> Option<String> {
    let rakamlar: String = ham
        .chars()
        .filter(|karakter| karakter.is_ascii_digit())
        .collect();
    if rakamlar.is_empty() {
        return None;
    }
    let numara = if let Some(kalan) = rakamlar.strip_prefix("00") {
        kalan.to_string()
    } else if rakamlar.starts_with("90") && rakamlar.len() == 12 {
        rakamlar
    } else if rakamlar.starts_with('0') && rakamlar.len() == 11 {
        format!("90{}", &rakamlar[1..])
    } else if rakamlar.len() == 10 {
        format!("90{rakamlar}")
    } else {
        rakamlar
    };
    (10..=15).contains(&numara.len()).then_some(numara)
}

/// Arama sonucunda müşteri adayı olmanın ön koşulu: ulaşılabilir bir telefon.
/// Sabit hat da geçerlidir (WhatsApp hesabı çıkmazsa telefonla aranır); önemli
/// olan teklifin gideceği bir numaranın bulunması.
fn telefon_gecerli(ham: &str) -> bool {
    whatsapp_numarasi(ham).is_some()
}

/// Sektörün web sitesine duyduğu ihtiyaç: müşterisi internette arayan, menü /
/// hizmet / fiyat karşılaştıran sektörler demo satın almaya daha yakındır.
/// Ağırlıklar 5-20 arasıdır; tek başına belirleyici değildir, ulaşılabilirliğin
/// (telefon) arkasından gelir.
fn sektor_potansiyeli(sektor: &str) -> u8 {
    match demo_turu(sektor) {
        // Menü, hizmet listesi veya rezervasyon doğrudan satışa dönüşür.
        Demo::Menu | Demo::Klinik => 20,
        // Görsel vitrin, program ve referans ihtiyacı yüksek.
        Demo::Ofis => 15,
        Demo::Kafe | Demo::Servis | Demo::Eglence => 10,
        // Ürünleri pazaryerlerinde de satıldığı için site ihtiyacı görece düşük.
        Demo::Urun => 5,
    }
}

/// Bu puanın üstündeki işletmeler arayüzde "sıcak fırsat" işaretlenir ve
/// listenin en başında gösterilir.
const SICAK_ESIK: u8 = 75;

/// İşletmenin satın alma niyeti puanı (0-100). Yalnızca tarama sırasında
/// elimizde olan veriler kullanılır; ek ağ isteği yapılmaz. Ağırlıklar niyet
/// sırasına göre değil, "site yaptırmaya ne kadar yakın" sırasına göre verilir:
///
///   web varlığı: hiçbir web sitesi yok          +12
///   (liste yalnızca sitesiz işletmelerden oluştuğu için bu puan sabittir;
///    sıralamada niyet sinyali olarak diğer ölçütlere eşlik eder.)
///   telefon (mobil numara ayrıca +5)            +25
///   inceleme sayısı (varsa, gerçek müşteri)     +0..18
///   çalışma saatleri (aktif işletme)            +8
///   adres                                        +5..8
///   kaydın son 12/36 ayda güncellenmiş olması    +2..5
///   sektörün web ihtiyacı                        +5..20
///   koordinat                                    +3
///   kayıt doluluğu                               +0..5
/// 100 üzerinden SEO puanı: yalnızca Google kaydındaki gerçek alanlardan
/// hesaplanır (telefon, adres, saatler, yıldız, inceleme). Web sitesi yokluğu
/// bu ekranın hedefidir ve puana +25 olarak yansır.
fn seo_puani(isletme: &BusinessLead) -> u8 {
    google::seo_puani(
        &isletme.address,
        &isletme.phone,
        &isletme.calisma_saatleri,
        isletme.yildiz_puani,
        isletme.inceleme_sayisi,
    )
}

fn potansiyel_puani(isletme: &BusinessLead) -> u8 {
    // Listeye yalnızca web sitesi olmayan işletmeler girer.
    let mut puan: u16 = 12;
    if let Some(numara) = whatsapp_numarasi(&isletme.phone) {
        puan += 25;
        // Mobil numara genellikle işletme sahibinin kendi telefonudur.
        if numara.starts_with("905") {
            puan += 5;
        }
    }
    if let Some(inceleme) = isletme.inceleme_sayisi {
        puan += match inceleme {
            100.. => 18,
            30..=99 => 12,
            10..=29 => 6,
            _ => 0,
        };
    }
    if !isletme.calisma_saatleri.trim().is_empty() {
        puan += 8;
    }
    if isletme.address.contains("No:") {
        puan += 8;
    } else if isletme.address.contains(',') {
        puan += 5;
    }
    if let Some(gun) = isletme.son_guncelleme_gun {
        puan += if gun <= 365 {
            5
        } else if gun <= 1_095 {
            2
        } else {
            0
        };
    }
    puan += sektor_potansiyeli(&isletme.category) as u16;
    if isletme.konum.is_some() {
        puan += 3;
    }
    puan += (isletme.kayit_dolulugu.min(10) / 2) as u16;
    puan.min(100) as u8
}

/// Puanın nereden geldiğini tek satırda açıklar:
/// "86/100 · sosyal medya var, site yok · mobil numara · ...".
/// Arayüzde gerekçe olarak gösterilir; böylece sıralama şeffaf kalır.
fn potansiyel_gerekcesi(isletme: &BusinessLead) -> String {
    let mut parcalar: Vec<String> = Vec::new();
    if let Some(numara) = whatsapp_numarasi(&isletme.phone) {
        parcalar.push(if numara.starts_with("905") {
            "mobil numara".to_string()
        } else {
            "sabit numara".to_string()
        });
    }
    if let Some(inceleme) = isletme.inceleme_sayisi {
        if inceleme >= 10 {
            parcalar.push(format!("{inceleme} inceleme"));
        }
    }
    if !isletme.calisma_saatleri.trim().is_empty() {
        parcalar.push("çalışma saatleri".to_string());
    }
    if isletme.address.contains("No:") {
        parcalar.push("kapı numaralı adres".to_string());
    } else if isletme.address.contains(',') {
        parcalar.push("sokak/mahalle".to_string());
    }
    if let Some(gun) = isletme.son_guncelleme_gun {
        if gun <= 365 {
            parcalar.push("güncel kayıt".to_string());
        }
    }
    match sektor_potansiyeli(&isletme.category) {
        20 => parcalar.push("sektör ihtiyacı yüksek".to_string()),
        15 => parcalar.push("sektör ihtiyacı orta".to_string()),
        _ => {}
    }
    if parcalar.is_empty() {
        parcalar.push("yalnızca isim".to_string());
    }
    format!(
        "{}/100 · {}",
        potansiyel_puani(isletme),
        parcalar.join(" · ")
    )
}

/// Arama sonuçlarını satın alma potansiyeline göre sıralar: puanı yüksek olan
/// önce, eşitlikte alfabetik sıra (Türkçe karakterler sadeleştirilerek). Sorgu
/// başına 20 kayıt sınırı olduğu için listeye hangi kayıtların girdiği de bu
/// sıralamaya bağlıdır; il geneli taramada ilçe başına ayrılan kota da böylece
/// ilçenin en yüksek potansiyelli kayıtlarından doldurulur.
/// Eski harici kayıtlarda sıralama inceleme sayısına göredir: gerçek müşteri
/// trafiği olan işletme önce gelir (kaynağın kendi sonuç sırası bölgeye göre
/// değişebiliyor). Eşitlikte potansiyel puanı, sonra ad.
fn incelemeye_gore_sirala(liste: &mut [BusinessLead]) {
    liste.sort_by(|a, b| {
        let inceleme = |lead: &BusinessLead| lead.inceleme_sayisi.unwrap_or(0);
        inceleme(b)
            .cmp(&inceleme(a))
            .then_with(|| potansiyel_puani(b).cmp(&potansiyel_puani(a)))
            .then_with(|| arama_anahtari(&a.name).cmp(&arama_anahtari(&b.name)))
    });
}

fn leads_sirala(liste: &mut [BusinessLead]) {
    liste.sort_by(|a, b| {
        // Kara listedeki işletme ("reddetti") hiçbir zaman listenin başında
        // görünmez; kalanlar puana, eşit puanda iletişim durumuna göre sıralanır.
        a.durum
            .kara_listede()
            .cmp(&b.durum.kara_listede())
            .then_with(|| potansiyel_puani(b).cmp(&potansiyel_puani(a)))
            .then_with(|| a.durum.sira().cmp(&b.durum.sira()))
            .then_with(|| arama_anahtari(&a.name).cmp(&arama_anahtari(&b.name)))
    });
}

const GUN_ADLARI: [&str; 7] = [
    "Pazartesi",
    "Salı",
    "Çarşamba",
    "Perşembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];

fn saat_metni(dakika: u32) -> String {
    format!("{:02}:{:02}", dakika / 60, dakika % 60)
}

/// Verilen gün/saat için "açık / kapalı + açılış" cümlesi üretir.
fn durum_cumlesi(program: &[Vec<(u32, u32)>; 7], gun: usize, dakika: u32) -> String {
    for (bas, son) in &program[gun] {
        if dakika >= *bas && dakika < *son {
            if *bas == 0 && *son >= 1440 {
                return "İşletmeniz şu an açık (24 saat).".to_string();
            }
            if *son >= 1440 {
                return "İşletmeniz şu an açık (gece yarısına kadar).".to_string();
            }
            return format!("İşletmeniz şu an açık (kapanış {}).", saat_metni(*son));
        }
    }

    for i in 0..7 {
        let hedef = (gun + i) % 7;
        for (bas, _) in &program[hedef] {
            if i == 0 && *bas <= dakika {
                continue;
            }
            let etiket = match i {
                0 => "bugün",
                1 => "yarın",
                _ => GUN_ADLARI[hedef],
            };
            return format!(
                "İşletmeniz şu an kapalı; açılış {} {}.",
                etiket,
                saat_metni(*bas)
            );
        }
    }
    "İşletmeniz şu an kapalı.".to_string()
}

fn whatsapp_durumu(isletme: &BusinessLead) -> Option<String> {
    let program = haftalik_program(&isletme.calisma_saatleri)?;
    let simdi = chrono::Local::now();
    Some(durum_cumlesi(
        &program,
        simdi.weekday().num_days_from_monday() as usize,
        simdi.hour() * 60 + simdi.minute(),
    ))
}

/// Panelde gösterilecek kısa durum: "Şu an açık (kapanış 18:00)".
fn durum_kisa_cumle(cumle: &str) -> String {
    let temiz = cumle
        .trim_start_matches("İşletmeniz ")
        .trim_end_matches('.');
    let mut karakterler = temiz.chars();
    match karakterler.next() {
        Some(ilk) => ilk.to_uppercase().collect::<String>() + karakterler.as_str(),
        None => String::new(),
    }
}

/// Hazır mesaj: işletmenin defterdeki A/B varyantı kullanılır.
fn whatsapp_mesaji(isletme: &BusinessLead) -> String {
    let durum = whatsapp_durumu(isletme);
    whatsapp_mesaji_varyant(
        isletme,
        durum.as_deref(),
        isletme.mesaj_varyanti.unwrap_or_default(),
    )
}

/// Mesajdaki sektör cümlesi: demoda gerçekten hangi bölüm üretiliyorsa onu
/// anlatır (`demo_turu` ile aynı eşleme), böylece vaat ile sayfa tutarlı kalır.
fn sektor_demo_notu(sektor: &str) -> &'static str {
    match demo_turu(sektor) {
        // Hepsi 3-4 kelime: tanıtım cümlesi mesajı uzatmamalı.
        Demo::Menu => "menü ve günün yemekleri",
        Demo::Kafe => "menü ve tatlılar",
        Demo::Urun => "ürün vitrini ve teslimat",
        // Kuaför/berber/güzellik: sayfadaki galeri ve tek tıkla arama.
        // Klinik: hizmet kartlarında uzman kadro vurgusu var.
        Demo::Klinik => "hizmetler ve uzman kadro",
        Demo::Ofis => "hizmetler ve teklif",
        Demo::Servis => "hizmetler ve fiyat teklifi",
        Demo::Eglence => "program ve rezervasyon",
    }
}

/// Mesaja giren somut gerekçe: işletmenin kendi kaydından (yorum/puan,
/// web varlığı, çalışma saati). Uydurma cümle kurulmaz; elde veri yoksa o
/// kısım hiç yazılmaz.
/// Parçanın sokak/cadde olduğunu söyler: "No:12" ya da "Cad.", "Sokak", "Bulvarı"
/// gibi kelimeler. Mahalle adları ("Caddebostan") yanlışlıkla eşleşmesin diye
/// kelime sınırına bakılır.
fn sokak_mi(parca: &str) -> bool {
    let kucuk = parca.to_lowercase();
    if kucuk.contains("no:") {
        return true;
    }
    kucuk.split(|k: char| !k.is_alphabetic()).any(|kelime| {
        matches!(
            kelime,
            "cad"
                | "cadde"
                | "caddesi"
                | "cd"
                | "sok"
                | "sokak"
                | "sokagi"
                | "sokağı"
                | "bulvar"
                | "bulvari"
                | "bulvarı"
                | "blv"
                | "meydan"
                | "meydani"
                | "meydanı"
        )
    })
}

/// Mesajda tam sokak adresi yerine yalnızca mahalle/ilçe kalır:
/// "Moda Cad. No:12, Caferağa, Kadıköy / İstanbul" → "Caferağa, Kadıköy / İstanbul".
/// Mahalle yoksa yalnızca "Kadıköy / İstanbul" yazılır; adres zaten kısaysa
/// olduğu gibi döner.
fn kisa_adres(adres: &str) -> String {
    let parcalar: Vec<&str> = adres
        .split(',')
        .map(str::trim)
        .filter(|parca| !parca.is_empty())
        .collect();
    if parcalar.len() < 2 {
        return adres.trim().to_string();
    }
    // Son parça her zaman "İlçe / İl", ondan önceki mahalle ya da cadde.
    let yer = parcalar[parcalar.len() - 1];
    let onceki = parcalar[parcalar.len() - 2];
    if sokak_mi(onceki) {
        yer.to_string()
    } else {
        format!("{onceki}, {yer}")
    }
}

fn whatsapp_gerekcesi(isletme: &BusinessLead) -> Option<String> {
    let itibar = match (isletme.inceleme_sayisi, isletme.yildiz_puani) {
        (Some(adet), Some(puan)) if adet > 0 => Some(format!(
            "Kaydınızda {}★ ve {adet} yorum var",
            puan_metni(puan)
        )),
        (Some(adet), _) if adet > 0 => Some(format!("Kaydınızda {adet} yorum var")),
        (_, Some(puan)) => Some(format!("Kaydınızda {}★ puan var", puan_metni(puan))),
        _ => None,
    };
    // Kısa ve tek başına da okunabilen ifadeler: itibar yoksa cümle bununla başlar.
    // Listeye yalnızca sitesiz işletmeler girdiği için cümle sabittir.
    let site = "web siteniz yok";
    let mut cumle = match itibar {
        // Yorum/puan varsa "bu kadar görünürsünüz ama…" kurgusu en güçlüsü.
        Some(itibar) => format!("{itibar} ama {site}"),
        None => {
            let mut harfler = site.chars();
            match harfler.next() {
                Some(ilk) => ilk.to_uppercase().collect::<String>() + harfler.as_str(),
                None => String::new(),
            }
        }
    };
    if isletme.calisma_saatleri.trim().is_empty() {
        cumle.push_str(", çalışma saatiniz de yok");
    }
    Some(format!("{cumle}."))
}

fn whatsapp_mesaji_varyant(
    isletme: &BusinessLead,
    durum: Option<&str>,
    varyant: musteriler::Varyant,
) -> String {
    let ek = match durum {
        Some(durum) => format!(" {durum}"),
        None => String::new(),
    };
    // Demo tanıtımı sektör başına birkaç kelime; harita ve açık/kapalı bilgisi
    // zaten hemen önceki durum cümlesinde ve sayfada yer alıyor.
    let demo_notu = format!("demoda {}", sektor_demo_notu(&isletme.category));
    // Ticari ileti kuralları gereği: kaynağı belirt ve aynı kanaldan ret hakkı sun.
    let kaynak = "Google Maps";
    // Somut gerekçe ilk yanıtın parçası: "neden yazıyorum"un cevabı.
    let gerekce = match whatsapp_gerekcesi(isletme) {
        Some(cumle) => format!(" {cumle}"),
        None => String::new(),
    };
    let adres = kisa_adres(&isletme.address);
    // Kapanış iki varyantta ortak; yalnızca açılış ve taslak cümlesi değişir.
    let kapanis = format!(
        " Beğenirseniz alan adınızda yayına alabilirim. Numaraya {kaynak} kaydınızdan ulaştım; istemezseniz \"RET\" yazın."
    );
    match varyant {
        // A: soruyla açılır, taslak kısa bir etiketle sunulur.
        musteriler::Varyant::A => {
            let taslak = match isletme.yayin_linki.as_deref() {
                Some(link) => format!(" Taslak: {link} — {demo_notu}."),
                None => format!(" 12 saatlik demo dosyasını ekliyorum; {demo_notu}."),
            };
            format!(
                "Merhaba, {} ({adres}) için hazırladığım web sitesi taslağını incelemek ister misiniz?{gerekce}{ek}{taslak}{kapanis}",
                isletme.name
            )
        }
        // B: soru sormadan doğrudan eksikleri söyler, taslağı çözüm olarak verir.
        musteriler::Varyant::B => {
            let taslak = match isletme.yayin_linki.as_deref() {
                Some(link) => {
                    format!(" Eksikleri kapatan kısa bir web sitesi taslağı hazırladım: {link} — {demo_notu}.")
                }
                None => format!(
                    " Eksikleri kapatan kısa bir taslak hazırladım; 12 saatlik demo dosyasını ekliyorum — {demo_notu}."
                ),
            };
            format!(
                "Merhaba, {} ({adres}).{gerekce}{ek}{taslak}{kapanis}",
                isletme.name
            )
        }
    }
}

fn gun_kodu_cevir(kelime: &str) -> Option<&'static str> {
    match kelime {
        "Mo" => Some("Pzt"),
        "Tu" => Some("Sal"),
        "We" => Some("Çar"),
        "Th" => Some("Per"),
        "Fr" => Some("Cum"),
        "Sa" => Some("Cmt"),
        "Su" => Some("Paz"),
        "off" => Some("kapalı"),
        _ => None,
    }
}

fn calisma_saatleri_metni(ham: &str) -> String {
    let ham = ham.trim();
    if ham.is_empty() {
        return String::new();
    }
    if ham == "24/7" {
        return "24 saat açık".to_string();
    }

    // Gün kodları yalnızca tam kelime olarak çevrilir; "Sadece" gibi notlar bozulmaz.
    let mut cikti = String::with_capacity(ham.len());
    let mut kelime = String::new();
    for karakter in ham.chars() {
        if karakter.is_ascii_alphabetic() {
            kelime.push(karakter);
        } else {
            if !kelime.is_empty() {
                match gun_kodu_cevir(&kelime) {
                    Some(uzun) => cikti.push_str(uzun),
                    None => cikti.push_str(&kelime),
                }
                kelime.clear();
            }
            cikti.push(karakter);
        }
    }
    if !kelime.is_empty() {
        match gun_kodu_cevir(&kelime) {
            Some(uzun) => cikti.push_str(uzun),
            None => cikti.push_str(&kelime),
        }
    }
    cikti.replace("; ", " · ").replace(';', " · ")
}

fn saat_dakika(metin: &str) -> Option<u32> {
    let (saat, dakika) = metin.trim().split_once(':')?;
    let saat: u32 = saat.parse().ok()?;
    let dakika: u32 = dakika.parse().ok()?;
    if saat > 24 || dakika > 59 {
        return None;
    }
    Some(saat * 60 + dakika)
}

fn gun_indeksi(kod: &str) -> Option<usize> {
    match kod {
        "Mo" => Some(0),
        "Tu" => Some(1),
        "We" => Some(2),
        "Th" => Some(3),
        "Fr" => Some(4),
        "Sa" => Some(5),
        "Su" => Some(6),
        _ => None,
    }
}

fn gun_listesi(metin: &str) -> Option<Vec<usize>> {
    let mut gunler = Vec::new();
    for parca in metin.split(',') {
        if let Some((bas, son)) = parca.split_once('-') {
            let bas = gun_indeksi(bas)?;
            let son = gun_indeksi(son)?;
            let mut gun = bas;
            loop {
                gunler.push(gun);
                if gun == son || gunler.len() > 7 {
                    break;
                }
                gun = (gun + 1) % 7;
            }
        } else {
            gunler.push(gun_indeksi(parca)?);
        }
    }
    if gunler.is_empty() || gunler.len() > 7 {
        None
    } else {
        Some(gunler)
    }
}

/// OSM `opening_hours` değerini Pazartesi=0 olacak şekilde haftalık dakika
/// aralıklarına çözer. Desteklenmeyen (mevsimsel, PH, sunrise gibi) ifadelerde
/// `None` döner; bu durumda demo yalnızca okunur saat metnini gösterir.
fn haftalik_program(ham: &str) -> Option<[Vec<(u32, u32)>; 7]> {
    let ham = ham.trim();
    if ham.is_empty() {
        return None;
    }
    let mut program: [Vec<(u32, u32)>; 7] = Default::default();
    if ham == "24/7" {
        for gun in program.iter_mut() {
            gun.push((0, 1440));
        }
        return Some(program);
    }

    for kural in ham.split(';') {
        let kural = kural.trim();
        if kural.is_empty() {
            continue;
        }
        if kural.contains('"')
            || kural.contains("||")
            || kural.contains("PH")
            || kural.contains("week")
            || kural.contains("sunrise")
            || kural.contains("sunset")
        {
            return None;
        }

        let parcalar: Vec<&str> = kural.split_whitespace().collect();
        let (gunler, saat_metni) = match parcalar.len() {
            1 => (vec![0, 1, 2, 3, 4, 5, 6], parcalar[0]),
            2 => (gun_listesi(parcalar[0])?, parcalar[1]),
            _ => return None,
        };
        if saat_metni == "off" || saat_metni == "closed" {
            continue;
        }

        let mut araliklar = Vec::new();
        for aralik in saat_metni.split(',') {
            let (bas, son) = aralik.split_once('-')?;
            let bas = saat_dakika(bas)?;
            let son = saat_dakika(son)?;
            if bas == son {
                return None;
            }
            araliklar.push((bas, son));
        }

        for gun in gunler {
            for &(bas, son) in &araliklar {
                if bas < son {
                    program[gun].push((bas, son));
                } else {
                    // Gece yarısını aşan aralık iki güne bölünür.
                    program[gun].push((bas, 1440));
                    program[(gun + 1) % 7].push((0, son));
                }
            }
        }
    }

    if program.iter().all(|gun| gun.is_empty()) {
        None
    } else {
        Some(program)
    }
}

fn program_js(program: &[Vec<(u32, u32)>; 7]) -> String {
    let gunler: Vec<String> = program
        .iter()
        .map(|araliklar| {
            let liste: Vec<String> = araliklar
                .iter()
                .map(|(bas, son)| format!("[{bas},{son}]"))
                .collect();
            format!("[{}]", liste.join(","))
        })
        .collect();
    format!("[{}]", gunler.join(","))
}

fn base64_kodla(veri: &[u8]) -> String {
    const ALFABE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut cikti = String::with_capacity(veri.len().div_ceil(3) * 4);
    for parca in veri.chunks(3) {
        let b0 = parca[0] as u32;
        let b1 = *parca.get(1).unwrap_or(&0) as u32;
        let b2 = *parca.get(2).unwrap_or(&0) as u32;
        let uc = (b0 << 16) | (b1 << 8) | b2;
        cikti.push(ALFABE[(uc >> 18) as usize & 63] as char);
        cikti.push(ALFABE[(uc >> 12) as usize & 63] as char);
        if parca.len() > 1 {
            cikti.push(ALFABE[(uc >> 6) as usize & 63] as char);
        } else {
            cikti.push('=');
        }
        if parca.len() > 2 {
            cikti.push(ALFABE[uc as usize & 63] as char);
        } else {
            cikti.push('=');
        }
    }
    cikti
}

/// Demo sayfasına gömülen fontlar (SIL OFL 1.1): başlık Space Grotesk,
/// gövde Geist, sıcak sektörlerin başlığı Instrument Serif.
/// Inter bilinçli olarak kullanılmıyor: AI üretimi sitelerin en belirgin izi.
/// Ayrıntı: varliklar/fontlar/LISANSLAR.md
const FONT_BASLIK: &[u8] = include_bytes!("../varliklar/fontlar/space-grotesk-tr.woff2");
const FONT_LUKS_BASLIK: &[u8] =
    include_bytes!("../varliklar/fontlar/cormorant-garamond-latin.woff2");
const FONT_LUKS_BASLIK_EXT: &[u8] =
    include_bytes!("../varliklar/fontlar/cormorant-garamond-latin-ext.woff2");
const FONT_LUKS_GOVDE: &[u8] = include_bytes!("../varliklar/fontlar/sora-latin.woff2");
const FONT_LUKS_GOVDE_EXT: &[u8] = include_bytes!("../varliklar/fontlar/sora-latin-ext.woff2");
const FONT_ISIK_GOVDE: &[u8] = include_bytes!("../varliklar/fontlar/plus-jakarta-sans-latin.woff2");
const FONT_ISIK_GOVDE_EXT: &[u8] =
    include_bytes!("../varliklar/fontlar/plus-jakarta-sans-latin-ext.woff2");
const FONT_GOVDE: &[u8] = include_bytes!("../varliklar/fontlar/geist-latin.woff2");
const FONT_GOVDE_EXT: &[u8] = include_bytes!("../varliklar/fontlar/geist-latin-ext.woff2");
const FONT_SERIF: &[u8] = include_bytes!("../varliklar/fontlar/instrument-serif-latin.woff2");
const FONT_SERIF_EXT: &[u8] =
    include_bytes!("../varliklar/fontlar/instrument-serif-latin-ext.woff2");

/// Latin ve latin-ext parçalarının kapsadığı kod noktaları. Tek dosyalık
/// sayfada iki parça da gömülü; `unicode-range` yalnız Türkçe harflerin
/// (ğ ş İ ı) doğru dosyadan gelmesini sağlar.
const LATIN_ARALIK: &str = "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD";
const LATIN_EXT_ARALIK: &str = "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C4,U+2113,U+2C60-2C7F,U+A720-A7FF";

/// `@font-face` tanımları: fontlar base64 olarak sayfaya gömülür.
fn font_css() -> &'static str {
    static ONBELLEK: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ONBELLEK.get_or_init(|| {
        let yuz = |aile: &str, agirlik: &str, dosya: &[u8], aralik: &str| {
            format!(
                "@font-face{{font-family:'{aile}';font-style:normal;font-weight:{agirlik};\
                 font-display:swap;unicode-range:{aralik};\
                 src:url(data:font/woff2;base64,{}) format('woff2')}}\n",
                base64_kodla(dosya)
            )
        };
        [
            yuz("Space Grotesk", "300 700", FONT_BASLIK, "U+0000-10FFFF"),
            yuz("Geist", "400 600", FONT_GOVDE, LATIN_ARALIK),
            yuz("Geist", "400 600", FONT_GOVDE_EXT, LATIN_EXT_ARALIK),
            yuz("Instrument Serif", "400", FONT_SERIF, LATIN_ARALIK),
            yuz("Instrument Serif", "400", FONT_SERIF_EXT, LATIN_EXT_ARALIK),
            yuz(
                "Cormorant Garamond",
                "300 600",
                FONT_LUKS_BASLIK,
                LATIN_ARALIK,
            ),
            yuz(
                "Cormorant Garamond",
                "300 600",
                FONT_LUKS_BASLIK_EXT,
                LATIN_EXT_ARALIK,
            ),
            yuz("Sora", "300 600", FONT_LUKS_GOVDE, LATIN_ARALIK),
            yuz("Sora", "300 600", FONT_LUKS_GOVDE_EXT, LATIN_EXT_ARALIK),
            yuz(
                "Plus Jakarta Sans",
                "200 800",
                FONT_ISIK_GOVDE,
                LATIN_ARALIK,
            ),
            yuz(
                "Plus Jakarta Sans",
                "200 800",
                FONT_ISIK_GOVDE_EXT,
                LATIN_EXT_ARALIK,
            ),
        ]
        .concat()
    })
}

// --- İşletme fotoğrafı -----------------------------------------------------

/// İndirilen verinin gerçek türünü ilk baytlardan okur. Sunucunun bildirdiği
/// `content-type` her zaman doğru olmadığı için içerik esas alınır.
fn foto_mime(veri: &[u8]) -> Option<&'static str> {
    if veri.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if veri.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if veri.len() >= 12 && &veri[0..4] == b"RIFF" && &veri[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Fotoğrafı indirir; 2.5 MB üstü dosyalar demoyu şişirmemek için atlanır.
fn foto_indir(url: &str, _boyut: u32) -> Option<(Vec<u8>, String)> {
    use std::io::Read;
    let ajan = http_ajani();
    let istek = if let Some(dosya) = url.strip_prefix("commons:") {
        // Dosya adı Türkçe/aksanlı karakter içerebilir: yüzde kodlaması şart.
        ajan.get(&format!(
            "https://commons.wikimedia.org/wiki/Special:FilePath/{}?width=1200",
            url_encode(dosya)
        ))
    } else {
        ajan.get(url)
    };
    let yanit = istek.timeout(Duration::from_secs(20)).call().ok()?;
    let mut veri = Vec::new();
    yanit
        .into_reader()
        .take(2_500_001)
        .read_to_end(&mut veri)
        .ok()?;
    if veri.len() < 1024 || veri.len() > 2_500_000 {
        return None;
    }
    let mime = foto_mime(&veri)?;
    Some((veri, mime.to_string()))
}

/// Commons dosyasının yazar ve lisansı. CC BY-SA gibi lisanslarda atıf zorunlu
/// olduğu için fotoğraf ancak bu bilgi alınabiliyorsa kullanılır.
fn commons_atif(dosya: &str) -> Option<String> {
    let ajan = http_ajani();
    let sorgu = format!(
        "https://commons.wikimedia.org/w/api.php?action=query&format=json&prop=imageinfo&iiprop=extmetadata&titles=File:{dosya}"
    );
    let metin = ajan
        .get(&sorgu)
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    let deger: serde_json::Value = serde_json::from_str(&metin).ok()?;
    let ust = deger
        .pointer("/query/pages")
        .and_then(|sayfalar| sayfalar.as_object())
        .and_then(|sayfalar| sayfalar.values().next())
        .and_then(|sayfa| sayfa.pointer("/imageinfo/0/extmetadata"))?;
    let alan = |ad: &str| {
        ust.pointer(&format!("/{ad}/value"))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
    };
    let sanatci = alan("Artist")
        .map(|metin| html_temizle(&metin))
        .unwrap_or_default();
    let lisans = alan("LicenseShortName").unwrap_or_default();
    let atif = match (sanatci.is_empty(), lisans.is_empty()) {
        (false, false) => format!("{sanatci} · {lisans} · Wikimedia Commons"),
        (false, true) => format!("{sanatci} · Wikimedia Commons"),
        (true, false) => format!("{lisans} · Wikimedia Commons"),
        (true, true) => return None,
    };
    Some(atif)
}

/// `Artist` alanı HTML içerebilir ("<a href=...>Ad</a>").
fn html_temizle(metin: &str) -> String {
    let mut temiz = String::with_capacity(metin.len());
    let mut icerde = false;
    for karakter in metin.chars() {
        match karakter {
            '<' => icerde = true,
            '>' => icerde = false,
            _ if !icerde => temiz.push(karakter),
            _ => {}
        }
    }
    temiz.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Demoya gömülecek fotoğraf: (mime, base64) ve altbilgi atfı.
fn demo_fotografi(isletme: &BusinessLead) -> (Option<(String, String)>, Option<String>) {
    let Some(foto) = &isletme.foto else {
        return (None, None);
    };
    let Some((veri, mime)) = foto_indir(&foto.url, 1600) else {
        return (None, None);
    };
    let atif = if foto.atif.trim().is_empty() {
        foto.url
            .strip_prefix("commons:")
            .and_then(commons_atif)
            .or_else(|| Some("Wikimedia Commons".to_string()))
    } else {
        Some(foto.atif.clone())
    };
    (Some((mime, base64_kodla(&veri))), atif)
}

/// Demo galerisi: sırasıyla (1) işletmenin kendi fotoğrafları,
/// (2) sektörün gömülü örnek kareleri,
/// (3) sektörün gömülü örnek kareleri. Üçüncü değer altbilgi atfıdır
/// (lisans/atıf gereği); ikinci değer `true` ise kareler örnektir.
fn demo_galerisi(isletme: &BusinessLead) -> (Vec<(String, String)>, bool, Option<String>) {
    const GALERI_BOYUT: u32 = 1100;
    let mut kareler = Vec::new();
    for foto in &isletme.fotolar {
        if let Some((veri, mime)) = foto_indir(&foto.url, GALERI_BOYUT) {
            kareler.push((mime, base64_kodla(&veri)));
        }
    }
    if !kareler.is_empty() {
        return (kareler, false, None);
    }
    let aile = stok_ailesi(&isletme.category);
    (stok_galeri(aile), true, stok_atif(aile))
}

/// Sektör fotoğraflarının lisans/sanatçı bilgisi (Wikimedia Commons).
const STOK_MANIFEST: &str = include_str!("../varliklar/foto/kaynak.json");

/// Sektörün gömülü örnek fotoğrafları. Builder'da `include_bytes!` ile exe'ye
/// girer: çalışma anında ağ isteği yok, dosya yolu bağımlılığı yok.
fn stok_dosyalar(aile: &str) -> Vec<&'static [u8]> {
    match aile {
        "menu" => vec![
            include_bytes!("../varliklar/foto/menu/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/menu/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/menu/3.webp").as_slice(),
        ],
        "kafe" => vec![
            include_bytes!("../varliklar/foto/kafe/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/kafe/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/kafe/3.webp").as_slice(),
        ],
        "unlu" => vec![
            include_bytes!("../varliklar/foto/unlu/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/unlu/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/unlu/3.webp").as_slice(),
        ],
        "urun" => vec![
            include_bytes!("../varliklar/foto/urun/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/urun/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/urun/3.webp").as_slice(),
        ],
        "galeri" => vec![
            include_bytes!("../varliklar/foto/galeri/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/galeri/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/galeri/3.webp").as_slice(),
        ],
        "klinik" => vec![
            include_bytes!("../varliklar/foto/klinik/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/klinik/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/klinik/3.webp").as_slice(),
        ],
        "otel" => vec![
            include_bytes!("../varliklar/foto/otel/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/otel/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/otel/3.webp").as_slice(),
        ],
        "egitim" => vec![
            include_bytes!("../varliklar/foto/egitim/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/egitim/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/egitim/3.webp").as_slice(),
        ],
        "ofis" => vec![
            include_bytes!("../varliklar/foto/ofis/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/ofis/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/ofis/3.webp").as_slice(),
        ],
        "servis" => vec![
            include_bytes!("../varliklar/foto/servis/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/servis/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/servis/3.webp").as_slice(),
        ],
        "cekici" => vec![
            include_bytes!("../varliklar/foto/cekici/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/cekici/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/cekici/3.webp").as_slice(),
        ],
        "eglence" => vec![
            include_bytes!("../varliklar/foto/eglence/1.webp").as_slice(),
            include_bytes!("../varliklar/foto/eglence/2.webp").as_slice(),
            include_bytes!("../varliklar/foto/eglence/3.webp").as_slice(),
        ],
        _ => Vec::new(),
    }
}

/// Gömülü karelerin MIME türü: tüm sektör aileleri WebP fotoğraftır.
fn stok_mime(_aile: &str) -> &'static str {
    "image/webp"
}

/// Galeriye gömülecek örnek fotoğraflar (mime, base64).
fn stok_galeri(aile: &str) -> Vec<(String, String)> {
    let mime = stok_mime(aile);
    stok_dosyalar(aile)
        .iter()
        .map(|veri| (mime.to_string(), base64_kodla(veri)))
        .collect()
}

/// Örnek karelerin altbilgide yazacak atıfları (lisans gereği).
fn stok_atif(aile: &str) -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(STOK_MANIFEST).ok()?;
    let kayitlar = manifest.get(aile)?.as_array()?;
    let satirlar: Vec<String> = kayitlar
        .iter()
        .filter_map(|kayit| kayit.get("atif")?.as_str().map(str::to_string))
        .collect();
    if satirlar.is_empty() {
        return None;
    }
    Some(format!("Fotoğraflar: {}", satirlar.join("; ")))
}

/// Renk seçicide kullanılan `#rrggbb` → `[r, g, b]` dönüşümü.
fn renk_coz(hex: &str) -> [u8; 3] {
    let govde = hex.trim().trim_start_matches('#');
    if govde.len() != 6 {
        return [0, 0, 0];
    }
    let kanal = |bas: usize| u8::from_str_radix(&govde[bas..bas + 2], 16).unwrap_or(0);
    [kanal(0), kanal(2), kanal(4)]
}

/// Renklendirme panelindeki tek satır: etiket, renk seçici ve "oto" düğmesi.
/// Renk seçici değiştiyse `true` döner.
fn renk_secici(ui: &mut egui::Ui, etiket: &str, deger: &mut String, varsayilan: &str) -> bool {
    let mut degisti = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(etiket).size(11.5).color(SOLUK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let oto = ui.add(
                egui::Button::new(egui::RichText::new("oto").size(9.5).color(DIM)).frame(false),
            );
            if oto.on_hover_text("Otomatik renge dön").clicked() && !deger.is_empty() {
                deger.clear();
                degisti = true;
            }
            let mut rgb = renk_coz(if deger.is_empty() { varsayilan } else { deger });
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                *deger = renk_yaz(rgb);
                degisti = true;
            }
        });
    });
    degisti
}

/// `[r, g, b]` → `#rrggbb`.
fn renk_yaz(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Kalıcı renk ayarı için güvenli değer: geçersiz metin varsayılana döner.
fn gecerli_renk(deger: &str, varsayilan: &str) -> String {
    let govde = deger.trim().trim_start_matches('#');
    if govde.len() == 6 && govde.chars().all(|karakter| karakter.is_ascii_hexdigit()) {
        format!("#{}", govde.to_lowercase())
    } else {
        varsayilan.to_string()
    }
}

/// Çekici / kurtarma / yol yardım sektörleri kendi görsel ailesini kullanır:
/// oto tamirhanesi değil, doğrudan çekici ve kurtarıcı kareleri.
fn stok_ailesi(sektor: &str) -> &'static str {
    let sade: String = sektor
        .chars()
        .map(|harf| match harf {
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'İ' | 'I' | 'ı' | 'i' => 'i',
            'Ö' | 'ö' => 'o',
            'Ş' | 'ş' => 's',
            'Ü' | 'ü' => 'u',
            diger => diger.to_ascii_lowercase(),
        })
        .collect();
    if ["cekici", "kurtarma", "yol yardim"]
        .iter()
        .any(|anahtar| sade.contains(anahtar))
    {
        "cekici"
    } else {
        demo_turu_adi(sektor)
    }
}

fn demo_turu_adi(sektor: &str) -> &'static str {
    match demo_turu(sektor) {
        Demo::Menu => "menu",
        Demo::Kafe => "kafe",
        Demo::Urun => "urun",
        Demo::Klinik => "klinik",
        Demo::Ofis => "ofis",
        Demo::Servis => "servis",
        Demo::Eglence => "eglence",
    }
}

/// İşletme kaydını tek dosyalık demo sayfasına çevirir.
///
/// Sayfanın kendisi `demo_site` modülünde: orada sektöre özel metinler,
/// CSS ve hareket kodu durur. Burada yalnızca kayıttaki gerçek veriler
/// (ad, adres, telefon, saatler, konum, puan) hazırlanır.
/// Öğrenilen animasyon sınıfını sektör varsayılanına uygular.
fn ogrenilen_sinematik(varsayilan: bool, ders: &ogrenme::Ders) -> bool {
    match ders.animasyon.as_deref() {
        Some("kenburns+sinema") => true,
        Some("kenburns") => false,
        _ => varsayilan,
    }
}

/// Hafızaya yazılan animasyon sınıfı.
fn animasyon_sinifi(sinematik: bool) -> &'static str {
    if sinematik {
        "kenburns+sinema"
    } else {
        "kenburns"
    }
}

#[allow(dead_code)] // Üretim yolu `demo_yaz`; bu sarmalayıcı testlerde kullanılır.
fn demo_html(isletme: &BusinessLead, gonderen: &str) -> String {
    demo_html_sec(isletme, gonderen, false)
}

/// Demo sayfasını üretir. `temiz` açıkken sayfa müşteri sürümüdür: örnek
/// yorumlar ve stok galeri kareleri sayfaya hiç konmaz (bkz. `demo_temizle`).
fn demo_html_sec(isletme: &BusinessLead, gonderen: &str, temiz: bool) -> String {
    demo_site::uret(&demo_girdi(isletme, gonderen, temiz))
}

/// Demo girdisini panel ayarlarıyla hazırlar.
fn demo_girdi(isletme: &BusinessLead, gonderen: &str, temiz: bool) -> demo_site::Girdi {
    demo_girdi_profille(isletme, gonderen, temiz, None)
}

/// Seçili medyanın video olup olmadığı: uzantıya göre hızlı ayrım.
fn medya_video(yol: &str) -> bool {
    matches!(
        std::path::Path::new(yol)
            .extension()
            .and_then(|uzanti| uzanti.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "mp4" | "mov" | "webm" | "m4v" | "mkv"
    )
}

/// Video ile aynı adı taşıyan poster dosyasını bulur (varsa).
fn medya_poster(video: &std::path::Path) -> Option<std::path::PathBuf> {
    for uzanti in ["webp", "jpg", "jpeg", "png"] {
        let aday = video.with_extension(uzanti);
        if aday.is_file() {
            return Some(aday);
        }
    }
    // Stok arşivinde poster sektör kökünde tek bir dosya olabilir.
    if let Some(kok) = video.parent().and_then(|klasor| klasor.parent()) {
        let aday = kok.join("poster.webp");
        if aday.is_file() {
            return Some(aday);
        }
    }
    None
}

/// `profil_ez` yalnız testlerde kullanılır: seçili ayarı değiştirmeden
/// sinematik çıktıyı üretmeyi sağlar.
fn demo_girdi_profille(
    isletme: &BusinessLead,
    gonderen: &str,
    temiz: bool,
    profil_ez: Option<&str>,
) -> demo_site::Girdi {
    let program = haftalik_program(&isletme.calisma_saatleri);
    // Gün gün saat tablosu; program çözülemiyorsa ham metin gösterilir.
    let saat_satirlari = match &program {
        Some(program) => program
            .iter()
            .enumerate()
            .map(|(gun, araliklar)| {
                let metin = if araliklar.is_empty() {
                    "Kapalı".to_string()
                } else {
                    araliklar
                        .iter()
                        .map(|(bas, son)| format!("{}–{}", saat_metni(*bas), saat_metni(*son)))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                (GUN_ADLARI[gun].to_string(), metin)
            })
            .collect(),
        None => Vec::new(),
    };
    let puan = isletme
        .yildiz_puani
        .map(|puan| format!("{puan:.1}").replace('.', ","));
    let yorum = isletme.inceleme_sayisi.map(|adet| format!("{adet} yorum"));
    let kaynak = "Google Maps";
    // Renklendirme paneli: boş alan otomatik (sektör rengi / koyu tema).
    let ayar = ayar_oku();
    // Sektörel tasarım hafızası: palet, tipografi ve aciliyet seviyesi.
    let mut tasarim = sektor_tasarim::sec(&isletme.category);
    // Öğrenme hafızası: onaylanmış palet ve başarılı animasyon sınıfı
    // sektör varsayılanını ezer.
    let ders = ogrenme::ders(&isletme.category);
    tasarim.sinematik = ogrenilen_sinematik(tasarim.sinematik, &ders);
    let sektor_vurgusu = if tasarim.vurgu.is_empty() {
        hex_renk(sektor_rengi(&isletme.category))
    } else {
        tasarim.vurgu.to_string()
    };
    let varsayilan_zemin = if tasarim.zemin.is_empty() {
        DEMO_ZEMIN
    } else {
        tasarim.zemin
    };
    let varsayilan_yazi = if tasarim.yazi.is_empty() {
        DEMO_YAZI
    } else {
        tasarim.yazi
    };
    let hafiza_vurgu = ders
        .palet
        .as_ref()
        .map(|palet| palet.vurgu.clone())
        .unwrap_or(sektor_vurgusu);
    let hafiza_zemin = ders
        .palet
        .as_ref()
        .map(|palet| palet.zemin.clone())
        .unwrap_or_else(|| varsayilan_zemin.to_string());
    let hafiza_yazi = ders
        .palet
        .as_ref()
        .map(|palet| palet.yazi.clone())
        .unwrap_or_else(|| varsayilan_yazi.to_string());
    let vurgu = gecerli_renk(&ayar.demo_vurgu, &hafiza_vurgu);
    let zemin = gecerli_renk(&ayar.demo_zemin, &hafiza_zemin);
    let yazi = gecerli_renk(&ayar.demo_yazi, &hafiza_yazi);
    // Panelden gelen özelleştirmeler: bölüm, metin ve köşe biçimi.
    let bolumler = demo_site::Bolumler {
        galeri: ayar.demo_galeri,
        yorumlar: ayar.demo_yorumlar,
        harita: ayar.demo_harita,
        saatler: ayar.demo_saatler,
    };
    // Sektör medya kütüphanesi: onaylanmış kareler varsa hero ve galeri
    // oradan beslenir; yoksa eski akış (panel/örnek kareler) aynen sürer.
    let medya_slug = sektor_medya::slug(&isletme.category);
    // Yerel medya havuzu: ağa çıkmadan diskteki gerçek video ve kareler
    // işletme tohumuyla dağıtılır. Aynı sektördeki iki işletme farklı
    // kombinasyon alır.
    let havuz = medya_havuz::dagit(
        &medya_slug,
        stok_video::tohum(&isletme.name, &isletme.address),
        4,
    );
    // "2 Medya" adımında elle seçilen dosyalar stok dağıtımını ezer. Böylece
    // onaylanan fotoğraf/video doğrudan aynı demo üretim hattına girer.
    let secili_fotolar: Vec<(String, String)> = isletme
        .medya
        .iter()
        .filter(|yol| !medya_video(yol))
        .filter_map(|yol| yerel_foto(yol))
        .collect();
    let secili_video = isletme.medya.iter().find(|yol| medya_video(yol)).cloned();
    let havuz_kareleri: Vec<(String, String)> = if !secili_fotolar.is_empty() {
        secili_fotolar.clone()
    } else {
        havuz
            .kareler
            .iter()
            .filter_map(|yol| yerel_foto(&yol.to_string_lossy()))
            .collect()
    };
    let havuzdan = !havuz_kareleri.is_empty();
    // Havuz kendi karelerini getiriyorsa atıf da havuzu gösterir.
    let kare_atif = Some(
        if !secili_fotolar.is_empty() {
            "Seçili medya"
        } else if havuzdan {
            "Yerel medya havuzu"
        } else {
            sektor_medya::ATIF
        }
        .to_string(),
    );
    let stok_kareler = if havuzdan {
        havuz_kareleri
    } else {
        sektor_medya::fotolar(&medya_slug)
    };
    // Fotoğraflar: panelden seçilen kaynak, kare sayısı ve kendi dosyaları.
    let ek_fotolar = ek_fotolari_oku(&ayar.demo_fotolar);
    let (oto_foto, oto_foto_atif) = match stok_kareler.first() {
        Some(kare) => (Some(kare.clone()), kare_atif.clone()),
        None => demo_fotografi(isletme),
    };
    let (foto, foto_atif) = hero_fotografi(&ayar, oto_foto, oto_foto_atif, &ek_fotolar);
    // İşletmenin kendi fotoğrafı yoksa hero'ya sektörle birebir eşleşen
    // gömülü kare gelir: çekicide çekici, kafede kahve, klinikte muayene.
    let (foto, foto_atif) = if foto.is_none() && !temiz {
        let aile = stok_ailesi(&isletme.category);
        match stok_galeri(aile).into_iter().next() {
            Some((mime, veri)) => (Some((mime, veri)), stok_atif(aile)),
            None => (foto, foto_atif),
        }
    } else {
        (foto, foto_atif)
    };
    let (oto_galeri, oto_ornek, oto_atif) = if stok_kareler.len() > 1 {
        (stok_kareler[1..].to_vec(), false, kare_atif.clone())
    } else {
        demo_galerisi(isletme)
    };
    let secim = galeri_secimi(
        oto_galeri,
        ek_fotolar,
        &ayar.demo_galeri_kaynak,
        ayar.demo_galeri_adet,
    );
    let (hikaye_foto, galeri) = hikaye_ve_galeri(secim.kareler, ayar.demo_galeri);
    // Satıcının numarası: gönderen alanı telefon değilse bant çizilmez.
    let satici_tel = whatsapp_numarasi(gonderen);
    let galeri_ornek = oto_ornek && secim.oto;
    let galeri_atif = if secim.oto { oto_atif } else { None };
    // Galeri başlığındaki kaynak notu: Street View ve "eklediğiniz kareler"
    // dışındaki durumlarda `demo_site` varsayılanı kullanılır.
    let sokak = galeri_atif
        .as_deref()
        .is_some_and(|atif| atif.contains("Street View"));
    let galeri_not = if secim.ek {
        Some(if secim.oto {
            format!(
                "{} · eklediğiniz kareler",
                if sokak {
                    "Sokak görüntüsü"
                } else {
                    "İşletme ve örnek kareler"
                }
            )
        } else {
            "Eklediğiniz fotoğraflar".to_string()
        })
    } else {
        sokak.then(|| "İşletmenin sokağından".to_string())
    };
    // Son müşteri sürümü: yorum ve galeri yalnız gerçek veriyle çizilir.
    let musteri_yorumu = ayar
        .demo_yorum_metinleri
        .iter()
        .any(|(soz, _)| !soz.trim().is_empty());
    let bolumler = if temiz {
        demo_site::Bolumler {
            galeri: bolumler.galeri && !galeri.is_empty() && !galeri_ornek,
            yorumlar: bolumler.yorumlar && musteri_yorumu,
            ..bolumler
        }
    } else {
        bolumler
    };
    let galeri = if temiz && galeri_ornek {
        Vec::new()
    } else {
        galeri
    };

    // Her demo üretiminden hemen önce lüks sinematik sözleşmeyi tazele:
    // sektör paleti + gömülü tipografi + efektler + beş zengin bölüm.
    let otomatik_cta = if ayar.demo_cta.trim().is_empty() {
        tasarim.cta.to_string()
    } else {
        demo_uzun(&ayar.demo_cta, 60)
    };
    let otomatik_saat = calisma_saatleri_metni(&isletme.calisma_saatleri);
    let brif_girdi = tasarim_brifi::BrifGirdi {
        ad: &isletme.name,
        sektor: &isletme.category,
        adres: &isletme.address,
        vurgu: &vurgu,
        vurgu2: tasarim.vurgu2,
        zemin: &zemin,
        yazi: &yazi,
        baslik: tasarim.baslik,
        govde: tasarim.govde,
        cta: &otomatik_cta,
        grup: tasarim.grup,
        hareket: tasarim.hareket,
        acil: tasarim.aciliyet >= 2,
        telefon: Some(isletme.phone.as_str()).filter(|telefon| !telefon.trim().is_empty()),
        saat: &otomatik_saat,
    };
    // Varsayılan: sektör brifi (hızlı ve deterministik). Yerel Ollama ile
    // dinamik brif isteyen operatör `NEXUS_BRIF_OLLAMA=1` verir; model
    // kapalıysa veya bozuk JSON dönerse sektör brifine sessizce dönülür.
    let otomatik_brif = if std::env::var("NEXUS_BRIF_OLLAMA").is_ok_and(|deger| deger == "1") {
        tasarim_brifi::Brif::yerel_veya_awwwards(&brif_girdi)
    } else {
        tasarim_brifi::Brif::awwwards(&brif_girdi)
    };
    let brif = tasarim_brifi::Brif::yaz_ve_oku(&otomatik_brif);
    let profil = profil_ez
        .map(str::to_string)
        .unwrap_or_else(|| ayar.demo_profil.clone());
    // Onaylı sektör videosu ya da "2 Medya"da seçilen video varsa hero kodla
    // çizilmez; tam ekran video katmanı kullanılır. Video yoksa profil aynen
    // korunur ve mevcut şablon davranışı sürer.
    let stok_video_var = sektor_medya::video(&medya_slug).is_some();
    let video =
        if secili_video.is_some() || stok_video_var || profil.eq_ignore_ascii_case("sinematik") {
            // Onaylı sektör videosu varsa o; yoksa eski grup videosu, o da yoksa
            // shader sürümü (mevcut davranış korunur).
            // Klasör çıktısında dosyalar media/hero.mp4 + media/poster.webp olarak
            // durur; sayfa bu yolları bekler.
            secili_video
                .as_ref()
                .map(|_| demo_site::VideoGirdi {
                    dosya: "media/hero.mp4".to_string(),
                    poster: "media/poster.webp".to_string(),
                    atif: Some("Seçili medya".to_string()),
                })
                .or_else(|| {
                    sektor_medya::video(&medya_slug).map(|bilgi| demo_site::VideoGirdi {
                        dosya: "media/hero.mp4".to_string(),
                        poster: "media/poster.webp".to_string(),
                        atif: bilgi.atif.clone(),
                    })
                })
                .or_else(|| {
                    stok_video::grup_esle(&isletme.category)
                        .and_then(|grup| {
                            stok_video::sec(
                                grup.id,
                                stok_video::tohum(&isletme.name, &isletme.address),
                            )
                        })
                        .map(|bilgi| demo_site::VideoGirdi {
                            dosya: bilgi.dosya.clone(),
                            poster: bilgi.poster.clone(),
                            atif: bilgi.sayfa_atfi(),
                        })
                })
                // Yerel havuz: internet yok, diskteki gerçek döngü. Tohum hero
                // türünü seçer: aynı sektörde bir işletme videoyla, diğeri
                // hareketli fotoğrafla açılır.
                .or_else(|| {
                    havuz.video.as_ref().filter(|_| havuz.video_hero).map(|_| {
                        demo_site::VideoGirdi {
                            dosya: "media/hero.mp4".to_string(),
                            poster: "media/poster.webp".to_string(),
                            atif: Some(
                                "Yerel medya havuzu · Wikimedia Commons (CC0 / CC BY-SA 4.0)"
                                    .to_string(),
                            ),
                        }
                    })
                })
        } else {
            None
        };
    // Seçili/onaylı stok videosu varsa video destekli sinematik şablonu
    // kullan: böylece video yalnız bir profilde değil, 3 Demo'nun tamamında
    // tam ekran, sessiz ve döngüsel arka plan olarak çalışır.
    let profil = if video.is_some() && (secili_video.is_some() || stok_video_var) {
        "sinematik".to_string()
    } else {
        profil
    };
    // Hero video kullanılıyorsa hero'ya ayrılan kare boşa gitmesin: galeriye
    // eklenir (video varken aynı kare iki kez görünmez).
    let galeri = match (&video, &foto) {
        (Some(_), Some(kare)) => {
            let mut liste = galeri;
            liste.insert(0, kare.clone());
            liste
        }
        _ => galeri,
    };
    let girdi = demo_site::Girdi {
        ad: isletme.name.clone(),
        adres: isletme.address.clone(),
        sektor: isletme.category.clone(),
        tur: demo_turu_adi(&isletme.category),
        vurgu: vurgu.clone(),
        zemin: zemin.clone(),
        yazi: yazi.clone(),
        foto,
        foto_atif,
        galeri,
        hikaye_foto,
        satici_tel,
        galeri_ornek,
        galeri_not,
        galeri_atif,
        telefon: whatsapp_numarasi(&isletme.phone),
        telefon_goster: isletme.phone.trim().to_string(),
        konum: isletme.konum,
        program: program.as_ref().map(program_js),
        saat_metni: calisma_saatleri_metni(&isletme.calisma_saatleri),
        saat_satirlari,
        puan,
        yorum,
        karne: Some(karne_dosya_adi(isletme)),
        kaynak: kaynak.to_string(),
        gonderen: if gonderen.trim().is_empty() {
            "Nexus Enterprise".to_string()
        } else {
            gonderen.to_string()
        },
        anahtar: dosya_slug(&isletme.name),
        bolumler,
        slogan: demo_metni(&ayar.demo_slogan),
        cta: demo_metni(&ayar.demo_cta),
        kose: match ayar.demo_kose.as_str() {
            "keskin" => "keskin".to_string(),
            "yumusak" => "yumusak".to_string(),
            _ => String::new(),
        },
        tema: match ayar.demo_tema.as_str() {
            "gece" => "gece".to_string(),
            _ => String::new(),
        },
        tasarim: Some(demo_site::TasarimGirdi {
            baslik: tasarim.baslik.to_string(),
            govde: tasarim.govde.to_string(),
            aciliyet: tasarim.aciliyet,
            sinematik: tasarim.sinematik,
            rozetler: tasarim
                .rozetler
                .iter()
                .map(|metin| (*metin).to_string())
                .collect(),
            cta: tasarim.cta.to_string(),
            vurgu2: tasarim.vurgu2.to_string(),
            ad: tasarim.ad.to_string(),
            grup: tasarim.grup.to_string(),
            hareket: tasarim.hareket.to_string(),
        }),
        ikincil: demo_metni(&ayar.demo_ikincil),
        metin: demo_site::Metinler {
            manset1: demo_uzun(&ayar.demo_manset1, 60),
            manset2: demo_uzun(&ayar.demo_manset2, 60),
            hakkinda_baslik: demo_uzun(&ayar.demo_hakkinda_baslik, 70),
            hakkinda1: demo_uzun(&ayar.demo_hakkinda1, 320),
            hakkinda2: demo_uzun(&ayar.demo_hakkinda2, 320),
            bolum_etiket: demo_uzun(&ayar.demo_bolum_etiket, 24),
            bolum_baslik: demo_uzun(&ayar.demo_bolum_baslik, 70),
            bolum_alt: demo_uzun(&ayar.demo_bolum_alt, 240),
            kalemler: ayar
                .demo_kalemler
                .iter()
                .map(|(ad, aciklama)| (demo_uzun(ad, 70), demo_uzun(aciklama, 220)))
                .collect(),
            yorumlar: ayar
                .demo_yorum_metinleri
                .iter()
                .map(|(soz, kim)| (demo_uzun(soz, 220), demo_uzun(kim, 60)))
                .collect(),
        },
        profil,
        sinematik: demo_site::SinematikAyar {
            hareket: ayar.demo_sinematik_hareket.clone(),
            nesne: ayar.demo_sinematik_nesne.clone(),
            sahne: ayar.demo_sinematik_sahne.clamp(3, 6),
        },
        video,
        medya_tohum: stok_video::tohum(&isletme.name, &isletme.address),
        brif,
        font_css: font_css(),
    };
    if !temiz {
        // Her üretim hafızaya küçük bir kayıt bırakır; aynı üçlü tekrarında
        // yeni satır açılmaz, yalnız sayaç artar (dosya birkaç KB kalır).
        ogrenme::uretim(
            &isletme.category,
            tasarim.grup,
            animasyon_sinifi(tasarim.sinematik),
            &vurgu,
            &zemin,
            &yazi,
            tasarim.hareket,
        );
    }
    girdi
}

/// Sinematik demo için üst sınır: 1,35 MB. Aşılırsa galeri kareleri düşer.
const DEMO_BUTCE: usize = 1_350_000;

/// Son üretimin (boyut, bütçe için çıkarılan fotoğraf sayısı) kaydı.
static SON_DEMO: std::sync::Mutex<Option<(usize, usize)>> = std::sync::Mutex::new(None);

fn boyut_kaydet(boyut: usize, kirpilan: usize) {
    if let Ok(mut kayit) = SON_DEMO.lock() {
        *kayit = Some((boyut, kirpilan));
    }
}

/// Panelde gösterilen son çıktı boyutu.
fn son_demo_boyutu() -> Option<(usize, usize)> {
    SON_DEMO.lock().ok().and_then(|kayit| *kayit)
}

/// Başlık/paragraf alanı: boşluk atılır, uzun yazı düzeni bozmasın diye
/// `sinir` karakterde kesilir. Boşsa şablon metni kullanılır (`demo_site`).
fn demo_uzun(deger: &str, sinir: usize) -> String {
    deger.trim().chars().take(sinir).collect()
}

/// Galeriyi ikiye ayırır: en az iki kare varsa ilki "Hakkında" bölümünün
/// görseli olur, kalanlar galeride kalır. Böylece aynı fotoğraf sayfada iki
/// kez görünmez, metin bölümü de görselsiz kalmaz. Galeri bölümü kapalıysa
/// hiçbir kare alınmaz (kapalı bölümün fotoğrafı başka yerde belirmesin).
fn hikaye_ve_galeri(
    mut kareler: Vec<demo_site::FotoVerisi>,
    galeri_acik: bool,
) -> (Option<demo_site::FotoVerisi>, Vec<demo_site::FotoVerisi>) {
    if galeri_acik && kareler.len() >= 2 {
        let ilk = kareler.remove(0);
        (Some(ilk), kareler)
    } else {
        (None, kareler)
    }
}

/// Panelden eklenen fotoğraf dosyaları: yalnız JPEG/PNG/WebP ve 3 MB altı.
/// Okunamayan yol sessizce atlanır; demo örnek karelere düşer, çökmez.
fn ek_fotolari_oku(yollar: &[String]) -> Vec<(String, String)> {
    yollar
        .iter()
        .filter_map(|yol| yerel_foto(yol))
        .take(GALERI_AZAMI)
        .collect()
}

/// Dosya gerçekten fotoğraf mı: uzantıya güvenmeden ilk baytlara ve boyuta
/// bakar. Panelde yolu eklerken tam dosyayı okumamak için ayrı durur.
fn foto_dosyasi_gecerli(yol: &str) -> bool {
    use std::io::Read;
    let Ok(mut dosya) = fs::File::open(yol) else {
        return false;
    };
    let Ok(uzunluk) = dosya.metadata().map(|bilgi| bilgi.len()) else {
        return false;
    };
    if !(512..=3_000_000).contains(&uzunluk) {
        return false;
    }
    let mut bas = [0u8; 12];
    dosya.read_exact(&mut bas).is_ok() && foto_mime(&bas).is_some()
}

/// Büyük fotoğrafı demoya gömülmek üzere küçültür: 1280 px JPEG, bir kez
/// üretilip `media/onbellek/` altında saklanır. Böylece galeri kareleri
/// bütçe yüzünden sayfadan düşmez ve demolar megabaytlara çıkmaz.
fn foto_kucult(yol: &str) -> Option<(String, String)> {
    let meta = fs::metadata(yol).ok()?;
    if meta.len() <= 240_000 {
        return None;
    }
    let govde = std::path::Path::new(yol).file_stem()?.to_str()?;
    let ad = dosya_slug(govde);
    let onbellek = format!("media/onbellek/{}-{}.jpg", ad, meta.len());
    if !std::path::Path::new(&onbellek).is_file() {
        fs::create_dir_all("media/onbellek").ok()?;
        let durum = crate::stok_video::gizli_komut("ffmpeg")
            .args(["-y", "-v", "error", "-i", yol])
            .args(["-vf", "scale=1280:-2", "-q:v", "4", "-frames:v", "1"])
            .arg(&onbellek)
            .status()
            .ok()?;
        if !durum.success() {
            let _ = fs::remove_file(&onbellek);
            return None;
        }
    }
    let veri = fs::read(&onbellek).ok()?;
    if veri.len() < 512 || veri.len() >= meta.len() as usize {
        return None;
    }
    Some(("image/jpeg".to_string(), base64_kodla(&veri)))
}

/// Tek bir yerel fotoğrafı demoya gömülecek hâle getirir: (mime, base64).
fn yerel_foto(yol: &str) -> Option<(String, String)> {
    if let Some(kucuk) = foto_kucult(yol.trim()) {
        return Some(kucuk);
    }
    let veri = fs::read(yol.trim()).ok()?;
    // Çok küçük dosya bozuktur; çok büyüğü demoyu megabaytlara çıkarır.
    if veri.len() < 512 || veri.len() > 3_000_000 {
        return None;
    }
    let mime = foto_mime(&veri)?;
    Some((mime.to_string(), base64_kodla(&veri)))
}

/// Galeri karelerinin nereden geleceği.
struct GaleriSecimi {
    kareler: Vec<(String, String)>,
    /// Otomatik kaynak (işletme/örnek kareler) kullanıldı mı?
    oto: bool,
    /// Kullanıcının eklediği dosyalar kullanıldı mı?
    ek: bool,
}

/// Panel seçimine göre galeriyi kurar: otomatik kareler, yalnız eklenenler,
/// ikisi birlikte veya kapalı. `adet` toplam kare sayısını sınırlar.
fn galeri_secimi(
    oto: Vec<(String, String)>,
    ek: Vec<(String, String)>,
    kaynak: &str,
    adet: usize,
) -> GaleriSecimi {
    let adet = adet.clamp(1, GALERI_AZAMI);
    let oto_uzunluk = oto.len();
    let ek_uzunluk = ek.len();
    let (mut kareler, oto_var, ek_var) = match kaynak.trim() {
        "yok" => (Vec::new(), false, false),
        "ekle" => (ek, false, true),
        "oto+ekle" => {
            let mut hepsi = oto;
            hepsi.extend(ek);
            (hepsi, oto_uzunluk > 0, ek_uzunluk > 0)
        }
        // Boş/tanınmayan değer: otomatik kaynak (eski ayar dosyaları).
        _ => (oto, oto_uzunluk > 0, false),
    };
    kareler.truncate(adet);
    // Kırpma sonrası hangi kaynağın kaldığı yeniden hesaplanır: otomatik
    // kareler önce geldiği için kalan fazlalık eklenen dosyalardır.
    let oto_kullanilan = if oto_var {
        oto_uzunluk.min(adet).min(kareler.len())
    } else {
        0
    };
    let ek_kalan = ek_var && kareler.len() > oto_kullanilan;
    GaleriSecimi {
        kareler,
        oto: oto_kullanilan > 0,
        ek: ek_kalan,
    }
}

/// Hero fotoğrafı: panel "otomatik" derse işletme fotoğrafı, "yok" derse
/// tipografik sahne, "dosya" derse seçilen yerel dosya kullanılır.
fn hero_fotografi(
    ayar: &UygulamaAyari,
    oto: Option<(String, String)>,
    oto_atif: Option<String>,
    ek: &[(String, String)],
) -> (Option<(String, String)>, Option<String>) {
    match ayar.demo_hero_foto.trim() {
        "yok" => (None, None),
        "dosya" => {
            let secili = yerel_foto(&ayar.demo_hero_dosya).or_else(|| ek.first().cloned());
            // Dosya okunamazsa boş sahne yerine işletme fotoğrafına dön.
            match secili {
                Some(foto) => (Some(foto), None),
                None => (oto, oto_atif),
            }
        }
        _ => (oto, oto_atif),
    }
}

/// Özelleştirme metinlerini sayfaya uygun hâle getirir: baştaki/sondaki
/// boşluk atılır, uzun yazım tek satır düzeni bozmasın diye 120 karakterde
/// kesilir.
fn demo_metni(deger: &str) -> String {
    deger.trim().chars().take(120).collect()
}

/// Demo dosyasındaki bir CSS rengini okur (`--accent:#e8a33d` → `#e8a33d`).
fn css_renk_degiskeni(html: &str, ad: &str) -> Option<String> {
    let anahtar = format!("--{ad}:");
    let bas = html.find(&anahtar)? + anahtar.len();
    let kalan = &html[bas..];
    let son = kalan.find([';', '}'])?;
    let deger = kalan[..son].trim().to_string();
    deger.starts_with('#').then_some(deger)
}

/// Üretilmiş bir demonun kök renklerini değiştirir; hareket ve düzen aynı kalır.
/// Kök değişkenler yoksa (elle yazılmış sayfa) `None` döner ve dosya atlanır.
fn demo_renklerini_degistir(
    html: &str,
    zemin: &str,
    yazi: &str,
    vurgu: Option<&str>,
) -> Option<String> {
    let eski_vurgu = css_renk_degiskeni(html, "accent")?;
    let vurgu = vurgu
        .filter(|deger| deger.starts_with('#'))
        .unwrap_or(&eski_vurgu)
        .to_string();
    let karsi = if demo_site::parlak_mi(&vurgu) {
        "#12100e"
    } else {
        "#f8f5f0"
    };

    // `:root{...--ink:<değer>;` bloğunu tek seferde yeniden yaz.
    let bas = html.find(":root{--accent:")?;
    let kalan = &html[bas..];
    let ink = kalan.find("--ink:")? + "--ink:".len();
    let son = kalan[ink..].find([';', '}'])?;
    let mut yeni = String::with_capacity(html.len());
    yeni.push_str(&html[..bas]);
    yeni.push_str(&format!(
        ":root{{--accent:{vurgu};--accent-karsi:{karsi};--bg:{zemin};--ink:{yazi};"
    ));
    yeni.push_str(&html[bas + ink + son..]);

    // Açık zeminde gölge/harita/ışık katmanlarının yönü değişir.
    let sinif = if demo_site::parlak_mi(zemin) {
        "<html lang=\"tr\" class=\"tema-acik\">"
    } else {
        "<html lang=\"tr\">"
    };
    // `kucult` nitelikleri tırnaksız yazabildiği için iki biçim de değişir.
    let yeni = yeni
        .replace("<html lang=\"tr\" class=\"tema-acik\">", sinif)
        .replace("<html lang=\"tr\">", sinif)
        .replace("<html lang=tr class=tema-acik>", sinif)
        .replace("<html lang=tr>", sinif);
    // Tarayıcı çubuğu rengi de vurguyu izlesin.
    Some(yeni.replace(
        &format!("content=\"{eski_vurgu}\""),
        &format!("content=\"{vurgu}\""),
    ))
}

/// Demo ile birlikte saklanan kaynak veri: sayfa sonradan panel ayarlarıyla
/// yeniden üretilebilsin diye işletme kaydının anlık görüntüsü tutulur.
/// Koordinat, çalışma saatleri ve yorumlar yalnız burada yaşar; defterde
/// (musteri_kayitlari.json) bunların hepsi yok, o yüzden defterden üretim
/// haritayı ve saatleri kaybettirirdi.
#[derive(serde::Serialize, serde::Deserialize)]
struct DemoKaynagi {
    isletme: BusinessLead,
    #[serde(default)]
    gonderen: String,
}

/// Anlık görüntülerin demo klasöründeki alt klasörü.
const DEMO_KAYNAK_ALT_KLASORU: &str = "kaynak";

/// Demo kaynağını `kaynak/<dosya>.json` olarak saklar. Yazılamazsa sessiz
/// geçilir: demoyu üretmek bu dosyaya bağlı değil.
fn demo_kaynagi_yaz(klasor: &str, dosya_adi: &str, isletme: &BusinessLead, gonderen: &str) {
    let alt = format!("{klasor}/{DEMO_KAYNAK_ALT_KLASORU}");
    if fs::create_dir_all(&alt).is_err() {
        return;
    }
    let kok = dosya_adi.trim_end_matches(".html");
    let kaynak = DemoKaynagi {
        isletme: isletme.clone(),
        gonderen: gonderen.to_string(),
    };
    if let Ok(metin) = serde_json::to_string(&kaynak) {
        let _ = fs::write(format!("{alt}/{kok}.json"), metin);
    }
}

/// Bir demo dosyasını yeniden üretmek için kaynak veri: önce üretim anında
/// yazılan anlık görüntü, yoksa arama sonucu önbelleğindeki aynı işletme.
/// Dosya adı işletmeden türediği için eşleşme ad üzerinden yapılır.
fn demo_kaynagi_bul(
    klasor: &str,
    dosya_adi: &str,
    onbellek: &SonucOnbellek,
    gonderen: &str,
) -> Option<DemoKaynagi> {
    let kok = dosya_adi.trim_end_matches(".html");
    let yol = format!("{klasor}/{DEMO_KAYNAK_ALT_KLASORU}/{kok}.json");
    if let Ok(metin) = fs::read_to_string(&yol) {
        if let Ok(kaynak) = serde_json::from_str::<DemoKaynagi>(&metin) {
            return Some(kaynak);
        }
    }
    onbellek
        .kayitlar
        .values()
        .flat_map(|kayit| kayit.leads.iter())
        .find(|lead| demo_dosya_adi(lead) == dosya_adi)
        .map(|lead| DemoKaynagi {
            isletme: lead.clone(),
            gonderen: gonderen.to_string(),
        })
}

/// Klasördeki demo sayfalarını güncel panel ayarlarıyla günceller.
/// Kaynağı bulunan demolar tamamen yeniden üretilir: metinler, butonlar,
/// fotoğraflar, bölümler ve renkler. Kaynağı olmayan (elle yapılmış)
/// denemelere yalnız renkler işlenir; böylece hiçbir sayfa bozulmaz.
/// Karneler kendi paletini kullandığı için atlanır.
/// (yenilenen, renklenen, atlanan)
fn demolara_ayarlari_uygula(
    klasor: &str,
    onbellek: &SonucOnbellek,
    zemin: &str,
    yazi: &str,
    vurgu: Option<&str>,
    gonderen: &str,
) -> (usize, usize, usize) {
    let mut yenilenen = 0;
    let mut renklenen = 0;
    let mut atlanan = 0;
    let Ok(girdiler) = fs::read_dir(klasor) else {
        return (0, 0, 0);
    };
    let mut dosyalar: Vec<String> = girdiler
        .flatten()
        .filter_map(|girdi| {
            let yol = girdi.path();
            if yol.extension().and_then(|uzanti| uzanti.to_str()) != Some("html") {
                return None;
            }
            let ad = yol.file_name().and_then(|ad| ad.to_str())?;
            // Karne ayrı bir sayfadır; demoya bağlı karne zaten yenilenir.
            Some(ad.to_string()).filter(|ad| !ad.starts_with("karne-"))
        })
        .collect();
    dosyalar.sort();
    for dosya_adi in dosyalar {
        let yol = format!("{klasor}/{dosya_adi}");
        if let Some(kaynak) = demo_kaynagi_bul(klasor, &dosya_adi, onbellek, gonderen) {
            match demo_yaz(klasor, &kaynak.isletme, &kaynak.gonderen) {
                Ok(_) => yenilenen += 1,
                Err(_) => atlanan += 1,
            }
            continue;
        }
        let Ok(html) = fs::read_to_string(&yol) else {
            atlanan += 1;
            continue;
        };
        match demo_renklerini_degistir(&html, zemin, yazi, vurgu) {
            Some(yeni) if yeni != html => {
                if fs::write(&yol, yeni).is_ok() {
                    renklenen += 1;
                } else {
                    atlanan += 1;
                }
            }
            // Zaten bu renkte: güncelleme sayılmaz, hata da değil.
            Some(_) => {}
            None => atlanan += 1,
        }
    }
    (yenilenen, renklenen, atlanan)
}

/// Panelde "dosya" seçilmiş hero fotoğrafı varsa yayın anında Ken Burns
/// döngüsüne çevirir ve kopyalanacak klasörü döndürür.
fn musteri_video_hazirla(isletme: &BusinessLead) -> Option<std::path::PathBuf> {
    let ayar = ayar_oku();
    if ayar.demo_hero_foto.trim() != "dosya" {
        return None;
    }
    let kaynak = ayar.demo_hero_dosya.trim();
    if kaynak.is_empty() || !foto_dosyasi_gecerli(kaynak) {
        return None;
    }
    let tohum = stok_video::tohum(&isletme.name, &isletme.address);
    let klasor = std::path::Path::new("media/musteri").join(format!("{tohum:016x}"));
    if !klasor.join("hero.mp4").is_file() {
        std::fs::create_dir_all(&klasor).ok()?;
        stok_video::foto_ken_burns_hedef(kaynak, &klasor, tohum).ok()?;
    }
    klasor.join("hero.mp4").is_file().then_some(klasor)
}

fn demo_uret(isletme: &BusinessLead, gonderen: &str) -> Result<String, String> {
    demo_yaz(DEMO_KLASORU, isletme, gonderen)
}

fn demo_yaz(klasor: &str, isletme: &BusinessLead, gonderen: &str) -> Result<String, String> {
    demo_yaz_profille(klasor, isletme, gonderen, None)
}

/// Profil zorlamalı yazım: testler ve video denemeleri seçili ayarı
/// değiştirmeden sinematik çıktı üretebilsin.
fn demo_yaz_profille(
    klasor: &str,
    isletme: &BusinessLead,
    gonderen: &str,
    profil_ez: Option<&str>,
) -> Result<String, String> {
    // Manuel onay kilidi: sektörün 1 video + 2 fotoğrafu onaylanmadıkça
    // hiçbir demo üretilmez; demo ancak açık [ONAYLA] kararıyla açılır.
    // "2 Medya" adımında geçerli stok dosyaları seçildiyse demo bu seçimle
    // üretilir; seçim yoksa eski insan onayı kilidi aynen uygulanır.
    let secili_medya = !isletme.medya.is_empty()
        && isletme
            .medya
            .iter()
            .all(|yol| yol.starts_with("media/stock/") && std::path::Path::new(yol).is_file());
    if !secili_medya {
        medya_onay::demo_kilidi(
            std::path::Path::new("."),
            &sektor_medya::slug(&isletme.category),
        )?;
    }
    fs::create_dir_all(klasor)
        .map_err(|hata| format!("{klasor} klasörü oluşturulamadı: {hata}"))?;
    let dosya_adi = demo_dosya_adi(isletme);
    let dosya = format!("{klasor}/{dosya_adi}");
    // Sinematik profilde bütçe denetimi: 1,35 MB aşılırsa galeri kareleri
    // sondan başlayarak düşürülür. Diğer profiller yalnız küçültülür.
    let mut girdi = demo_girdi_profille(isletme, gonderen, false, profil_ez);
    // Müşteri kendi fotoğrafını seçtiyse yayın anında ffmpeg ile Ken Burns
    // döngüsüne çevrilir; demo hero'sunda gerçek hareketli görsel oynar.
    let musteri_video = if girdi.video.is_none() && girdi.profil.eq_ignore_ascii_case("sinematik") {
        musteri_video_hazirla(isletme)
    } else {
        None
    };
    if musteri_video.is_some() {
        girdi.video = Some(demo_site::VideoGirdi {
            dosya: "media/hero.mp4".to_string(),
            poster: "media/poster.webp".to_string(),
            atif: Some("Müşteri fotoğrafı".to_string()),
        });
    }
    let (kucuk, kirpilan) = demo_site::uret_butceli(&girdi, DEMO_BUTCE);
    boyut_kaydet(kucuk.len(), kirpilan);
    // Video'lu sinematik sayfa klasör olarak yazılır: index.html + media/
    // (hero.mp4, poster.webp, KAYNAK.json). Böylece ilk açılışta yalnız
    // poster ve metin yüklenir, video arkadan indirilir.
    if girdi.video.is_some() {
        let alt = format!("{klasor}/sinematik/{}", dosya_adi.trim_end_matches(".html"));
        let medya = format!("{alt}/media");
        fs::create_dir_all(&medya).map_err(|hata| format!("{medya} oluşturulamadı: {hata}"))?;
        let indeks = format!("{alt}/index.html");
        fs::write(&indeks, &kucuk).map_err(|hata| format!("{indeks} yazılamadı: {hata}"))?;
        let stok_slug = sektor_medya::slug(&isletme.category);
        if let Some(secili_video) = isletme.medya.iter().find(|yol| medya_video(yol)) {
            // "2 Medya" adımında seçilen video doğrudan demo klasörüne girer.
            let _ = fs::copy(secili_video, format!("{medya}/hero.mp4"));
            if let Some(poster) = medya_poster(std::path::Path::new(secili_video)) {
                let _ = fs::copy(poster, format!("{medya}/poster.webp"));
            }
        } else if let Some(kok) = &musteri_video {
            // Müşteri fotoğrafından üretilen döngü demo klasörüne kopyalanır.
            let _ = fs::copy(kok.join("hero.mp4"), format!("{medya}/hero.mp4"));
            let _ = fs::copy(kok.join("poster.webp"), format!("{medya}/poster.webp"));
            let _ = fs::copy(kok.join("KAYNAK.json"), format!("{medya}/KAYNAK.json"));
        } else if let Some(bilgi) = sektor_medya::video(&stok_slug) {
            // Sektör medya kütüphanesinden onaylanmış video.
            let kok = format!("{}/{stok_slug}", sektor_medya::KLASOR);
            let _ = fs::copy(
                format!("{kok}/{}", bilgi.dosya),
                format!("{medya}/hero.mp4"),
            );
            let _ = fs::copy(
                format!("{kok}/{}", bilgi.poster),
                format!("{medya}/poster.webp"),
            );
            let _ = fs::copy(format!("{kok}/KAYNAK.json"), format!("{medya}/KAYNAK.json"));
        } else if let Some(grup) = stok_video::grup_esle(&isletme.category) {
            let kaynak = format!("{}/{}", stok_video::MEDYA_KOK, grup.id);
            // Tohumla seçilen kaydın dosyaları sayfaya hero/poster olarak kopyalanır.
            if let Some(bilgi) =
                stok_video::sec(grup.id, stok_video::tohum(&isletme.name, &isletme.address))
            {
                let _ = fs::copy(
                    format!("{kaynak}/{}", bilgi.dosya_adi),
                    format!("{medya}/hero.mp4"),
                );
                let _ = fs::copy(
                    format!("{kaynak}/{}", bilgi.poster_adi),
                    format!("{medya}/poster.webp"),
                );
            }
            let _ = fs::copy(
                format!("{kaynak}/KAYNAK.json"),
                format!("{medya}/KAYNAK.json"),
            );
        } else if let Some(kayit) = medya_havuz::hero_video(
            &stok_slug,
            stok_video::tohum(&isletme.name, &isletme.address),
        ) {
            // Yerel havuz videosu: ağ yok, diskteki gerçek döngü.
            let _ = fs::copy(&kayit.dosya, format!("{medya}/hero.mp4"));
            if let Some(poster) = &kayit.poster {
                let _ = fs::copy(poster, format!("{medya}/poster.webp"));
            }
            // Lisans bilgisi demo klasöründe kalır: atıf ve kaynak dosyada.
            if let Some(sektor_koku) = kayit.dosya.parent().and_then(|p| p.parent()) {
                let _ = fs::copy(
                    sektor_koku.join("KAYNAK.json"),
                    format!("{medya}/KAYNAK.json"),
                );
            }
        }
        demo_kaynagi_yaz(klasor, &dosya_adi, isletme, gonderen);
        karne_yaz(klasor, isletme, None)?;
        return Ok(indeks);
    }
    fs::write(&dosya, kucuk).map_err(|hata| format!("{dosya} yazılamadı: {hata}"))?;
    // Kaynak anlık görüntüsü: sayfa uygulama kapandıktan sonra da aynı
    // işletme verisinden yeniden üretilebilsin.
    demo_kaynagi_yaz(klasor, &dosya_adi, isletme, gonderen);
    // Karne demo ile birlikte üretilir: aynı klasörde olduğu için demoda ona
    // göreli bağlantı var, satış görüşmesinde ikisi birlikte açılır.
    karne_yaz(klasor, isletme, Some(&dosya_adi))?;
    Ok(dosya)
}

/// Kayıt kimliği: OSM node/way numarası (eski kayıtlarda place id).
/// Demo ve karne dosya adlarının sonuna eklenir, böylece aynı adlı iki işletme
/// birbirinin dosyasını ezmez.
fn kayit_kimligi(isletme: &BusinessLead) -> String {
    isletme
        .kaynak
        .rsplit('/')
        .next()
        .unwrap_or("0")
        .chars()
        .map(|karakter| {
            if karakter.is_ascii_alphanumeric() || karakter == '-' {
                karakter
            } else {
                '_'
            }
        })
        .collect()
}

/// Demo dosyasının adı (klasörsüz). Karne, demoya göreli bağlantı verebilmek
/// için aynı klasöre yazılır.
fn demo_dosya_adi(isletme: &BusinessLead) -> String {
    format!(
        "{}-{}.html",
        dosya_slug(&isletme.name),
        kayit_kimligi(isletme)
    )
}

/// Dijital görünürlük karnesinin dosya adı: `karne-<isletme>-<kimlik>.html`.
fn karne_dosya_adi(isletme: &BusinessLead) -> String {
    format!(
        "karne-{}-{}.html",
        dosya_slug(&isletme.name),
        kayit_kimligi(isletme)
    )
}

/// Karnedeki tek kontrolün durumu (rozet rengi ve simgesi buradan gelir).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KarneDurumu {
    /// Eksik yok: işletme bu kontrolü geçiyor.
    Iyi,
    /// Küçük eksik: kapatılabilir, satışı engellemez.
    Orta,
    /// Satışı doğrudan gerekçelendiren eksik.
    Eksik,
}

/// Dijital görünürlük karnesindeki tek satır: ne bulundu, ne kadar puan
/// kaybettirdi ve müşteriye ne önerilecek.
struct KarneSatiri {
    baslik: &'static str,
    deger: String,
    durum: KarneDurumu,
    /// Bu eksikliğin 100 üzerinden puan kaybı. 0 ise kontrol geçildi.
    kayip: u8,
    oneri: &'static str,
}

fn karne_satiri(
    baslik: &'static str,
    deger: String,
    kayip: u8,
    oneri: &'static str,
) -> KarneSatiri {
    KarneSatiri {
        durum: match kayip {
            0 => KarneDurumu::Iyi,
            1..=6 => KarneDurumu::Orta,
            _ => KarneDurumu::Eksik,
        },
        baslik,
        deger,
        kayip,
        oneri,
    }
}

/// Yıldız puanını Türkçe ondalık ayracıyla yazar: 4.2 → "4,2".
fn puan_metni(puan: f32) -> String {
    format!("{puan:.1}").replace('.', ",")
}

/// İşletmenin dijital görünürlük karnesini ölçer: 100 üzerinden puan ve
/// kontrol satırları. Yalnızca taramada zaten toplanmış veriler kullanılır;
/// hiçbir ağ isteği yapılmaz.
///
/// Ağırlıklar (toplam 90 puan kaybı mümkün, taban puan 10):
///   web sitesi yok                  -40   (sosyal medya varsa -30,
///                                          ücretsiz site aracı varsa -20)
///   çalışma saatleri kayıtlı değil  -20
///   telefon kayıtlı değil           -15
///   yorum/puan yok                 -15   (10+ yorum -5, 50+ yorum 0,
///                                          4'ün altındaki puan ayrıca ceza)
fn karne_olc(isletme: &BusinessLead) -> (u8, Vec<KarneSatiri>) {
    let link = if isletme.site_linki.trim().is_empty() {
        String::new()
    } else {
        format!(" · {}", isletme.site_linki.trim())
    };
    let site_degeri = if link.is_empty() {
        "Web sitesi yok · sosyal medya hesabı da bulunamadı".to_string()
    } else {
        format!("Web sitesi yok, kayıtlı adres{link}")
    };
    let site_kayip = 40;
    let site_oneri = "Kendi alan adınızda mobil uyumlu bir site açın; işletme kaydınıza ekleyin.";

    let saat_metni = calisma_saatleri_metni(&isletme.calisma_saatleri);
    let (saat_degeri, saat_kayip) = if saat_metni.is_empty() {
        ("Çalışma saatleri kayıtlı değil".to_string(), 20)
    } else {
        (format!("Kayıtlı program: {saat_metni}"), 0)
    };

    let (telefon_degeri, telefon_kayip) = if isletme.phone.trim().is_empty() {
        ("Telefon numarası kayıtlı değil".to_string(), 15)
    } else if whatsapp_numarasi(&isletme.phone).is_some() {
        (
            format!("{} · WhatsApp ile ulaşılabilir", isletme.phone.trim()),
            0,
        )
    } else {
        (
            format!("{} · WhatsApp üzerinden ulaşılamıyor", isletme.phone.trim()),
            5,
        )
    };

    let mut yorum_parcalari: Vec<String> = Vec::new();
    let mut yorum_kayip: u8 = 15;
    if let Some(adet) = isletme.inceleme_sayisi {
        yorum_parcalari.push(format!("{adet} yorum"));
        yorum_kayip = match adet {
            50.. => 0,
            10..=49 => 5,
            1..=9 => 10,
            _ => 14,
        };
    }
    if let Some(puan) = isletme.yildiz_puani {
        yorum_parcalari.push(format!("{} ★", puan_metni(puan)));
        // Düşük puan, yorum sayısından bağımsız bir güven sorunudur.
        yorum_kayip = match puan {
            p if p < 3.5 => yorum_kayip.saturating_add(4),
            p if p < 4.0 => yorum_kayip.saturating_add(2),
            _ => yorum_kayip,
        };
    }
    let (yorum_degeri, yorum_kayip) = if yorum_parcalari.is_empty() {
        (
            "Harita kaydında yorum ve puan verisi yok".to_string(),
            yorum_kayip,
        )
    } else {
        (yorum_parcalari.join(" · "), yorum_kayip)
    };

    let satirlar = vec![
        karne_satiri("Web sitesi", site_degeri, site_kayip, site_oneri),
        karne_satiri(
            "Çalışma saatleri",
            saat_degeri,
            saat_kayip,
            "İşletme kaydınıza haftalık saatleri girin; \"şu an açık\" bilgisi görünür.",
        ),
        karne_satiri(
            "Telefon",
            telefon_degeri,
            telefon_kayip,
            "Profilinize tıklanabilir bir telefon numarası ekleyin.",
        ),
        karne_satiri(
            "Müşteri yorumları",
            yorum_degeri,
            yorum_kayip,
            "Müşterilerinizden yorum isteyin; 10 ve üzeri yorum güven oluşturur.",
        ),
    ];

    let kayip: u16 = satirlar.iter().map(|satir| satir.kayip as u16).sum();
    let puan = (100u16.saturating_sub(kayip)).clamp(10, 100) as u8;
    (puan, satirlar)
}

/// Puanın sözel karşılığı ve vurgu rengi (gömülü CSS değişkeni).
fn karne_sinifi(puan: u8) -> (&'static str, &'static str) {
    match puan {
        80.. => ("İyi", "#34d399"),
        60..=79 => ("Orta", "#fbbf24"),
        40..=59 => ("Zayıf", "#fb923c"),
        _ => ("Kritik", "#fb7185"),
    }
}

/// Karnenin gömülü stil sayfası. Harici CDN, script veya font isteği yok:
/// sayfa tek dosya olarak GitHub Pages / Netlify / Hostinger'a atılabilir.
const KARNE_CSS: &str = r#"
:root{color-scheme:dark;--zemin:#09090b;--kart:#18181b;--cizgi:#27272a;--metin:#f4f4f5;--soluk:#a1a1aa;--dim:#71717a;--amber:#fbbf24}
*{box-sizing:border-box}
html{-webkit-text-size-adjust:100%}
body{margin:0;background:radial-gradient(120% 90% at 50% -20%,rgba(251,191,36,.08),transparent 60%),var(--zemin);color:var(--metin);font-family:'Segoe UI',system-ui,-apple-system,sans-serif;line-height:1.55;-webkit-font-smoothing:antialiased}
a{color:inherit}
.kabuk{max-width:780px;margin:0 auto;padding:clamp(32px,6vw,56px) 20px}
.ust-etiket{margin:0;font-size:11px;font-weight:800;letter-spacing:.18em;text-transform:uppercase;color:var(--amber)}
h1{margin:10px 0 0;font-size:clamp(28px,5vw,40px);line-height:1.08;letter-spacing:-.02em}
.alt-bilgi{margin:6px 0 0;font-size:14px;color:var(--soluk)}
.kunye{margin:4px 0 0;font-size:12px;color:var(--dim)}
.kunye a{color:var(--soluk);text-decoration:none;border-bottom:1px solid var(--cizgi)}
.skor{display:grid;grid-template-columns:1fr;gap:14px;margin-top:30px}
.skor-kart{--ton:#fbbf24;border:1px solid color-mix(in srgb,var(--ton) 38%,transparent);background:color-mix(in srgb,var(--ton) 10%,transparent);border-radius:14px;padding:20px}
.skor-puan{margin:4px 0 0;font-size:52px;font-weight:900;line-height:1;color:var(--ton)}
.skor-alt{margin:0;font-size:12px;color:var(--soluk)}
.skor-sinif{margin:10px 0 0;font-size:14px;font-weight:800;color:var(--ton)}
.ozet-kart{border:1px solid var(--cizgi);background:rgba(24,24,27,.6);border-radius:14px;padding:20px}
.etiket{margin:0;font-size:11px;letter-spacing:.16em;text-transform:uppercase;color:var(--dim)}
.ozet{margin:8px 0 0;font-size:14px;color:#d4d4d8}
.ozet-sayi{margin:12px 0 0;font-size:12px;color:var(--dim)}
h2{margin:36px 0 0;font-size:18px;font-weight:800;letter-spacing:-.01em}
.liste{margin-top:12px;border:1px solid var(--cizgi);border-radius:14px;background:rgba(24,24,27,.45);overflow:hidden}
.satir{display:flex;align-items:flex-start;gap:12px;padding:16px}
.satir + .satir{border-top:1px solid var(--cizgi)}
.rozet{display:inline-flex;align-items:center;justify-content:center;flex:0 0 24px;width:24px;height:24px;margin-top:2px;border-radius:50%;border:1px solid;font-size:12px;font-weight:800}
.rozet.iyi{color:#6ee7b7;border-color:rgba(16,185,129,.45);background:rgba(16,185,129,.15)}
.rozet.orta{color:#fcd34d;border-color:rgba(245,158,11,.45);background:rgba(245,158,11,.15)}
.rozet.eksik{color:#fda4af;border-color:rgba(244,63,94,.45);background:rgba(244,63,94,.15)}
.satir-govde{min-width:0}
.satir-baslik{margin:0;font-size:14px;font-weight:700}
.durum{margin-left:4px;font-size:10px;font-weight:800;letter-spacing:.08em;text-transform:uppercase;color:var(--dim)}
.satir-deger{margin:2px 0 0;font-size:14px;color:var(--soluk);overflow-wrap:anywhere}
.adimlar{margin:12px 0 0;padding:0;list-style:none;display:grid;gap:10px;font-size:14px;color:#d4d4d8}
.adimlar li{display:flex;gap:12px;align-items:flex-start}
.adim-no{display:inline-flex;align-items:center;justify-content:center;flex:0 0 24px;width:24px;height:24px;border-radius:50%;background:#27272a;color:#fcd34d;font-size:12px;font-weight:800}
.demo-kart{margin-top:36px;border:1px solid rgba(245,158,11,.3);background:rgba(245,158,11,.06);border-radius:14px;padding:20px}
.demo-kart h2{margin:0;font-size:15px;color:#fcd34d}
.demo-kart p{margin:6px 0 0;font-size:13px;color:var(--soluk)}
.buton{display:inline-block;margin-top:14px;padding:10px 20px;border-radius:999px;background:var(--amber);color:#18181b;font-size:13px;font-weight:800;text-decoration:none}
.buton:hover{filter:brightness(1.06)}
.dipnot{margin-top:40px;padding-top:18px;border-top:1px solid var(--cizgi);font-size:12px;line-height:1.65;color:var(--dim)}
.dipnot p{margin:0}
.dipnot p + p{margin-top:8px}
@media(min-width:640px){.skor{grid-template-columns:1fr 2fr}.ozet-kart{grid-column:span 2}.kabuk{padding-left:28px;padding-right:28px}}
"#;

/// Tek sayfalık dijital görünürlük karnesi. Satış görüşmesinde müşteriye
/// gösterilecek belge: eksikleri, neden önemli olduğunu ve önerilen adımları
/// tek ekranda toplar. `demo_dosyasi` verilirse hazırlanan taslağa bağlantı
/// eklenir (aynı klasörde olduğu için göreli adres yeterli).
fn karne_html(isletme: &BusinessLead, demo_dosyasi: Option<&str>) -> String {
    let (puan, satirlar) = karne_olc(isletme);
    let (sinif, ton) = karne_sinifi(puan);
    let ad = html_kacis(&isletme.name);
    let sektor = html_kacis(&isletme.category);
    let adres = html_kacis(&isletme.address);
    let kaynak = html_kacis(&isletme.kaynak);
    let tarih = bugun_anahtari();

    // Karne linki de WhatsApp'ta başlıklı ve açıklamalı görünsün.
    let paylasim = format!(
        concat!(
            "<meta name=\"description\" content=\"{aciklama}\">\n",
            "<meta property=\"og:type\" content=\"website\">\n",
            "<meta property=\"og:site_name\" content=\"{ad}\">\n",
            "<meta property=\"og:title\" content=\"{ad} — Dijital Görünürlük Karnesi ({sinif})\">\n",
            "<meta property=\"og:description\" content=\"{aciklama}\">\n",
            "<meta property=\"og:locale\" content=\"tr_TR\">\n",
            "<meta name=\"twitter:card\" content=\"summary\">\n",
            "<meta name=\"robots\" content=\"noindex\">"
        ),
        ad = ad,
        sinif = sinif,
        aciklama = format!(
            "{sektor} · Dijital görünürlük puanı {puan}/100. Eksikler ve atılacak adımlar tek sayfada."
        )
    );

    let eksikler: Vec<&KarneSatiri> = satirlar.iter().filter(|satir| satir.kayip > 0).collect();
    let eksik_sayisi = eksikler.len();
    let tamam_sayisi = satirlar.len() - eksik_sayisi;
    let ozet = if eksikler.is_empty() {
        "Bu karnedeki dört kontrolün hepsi geçildi; acil bir eksik görünmüyor.".to_string()
    } else {
        format!(
            "{} eksik tespit edildi: {}. Müşteri işletmenizi haritada ararken bu bilgileri göremediğinde doğrudan rakip işletmeye geçiyor.",
            eksikler.len(),
            eksikler
                .iter()
                .map(|satir| satir.baslik)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let satir_html: String = satirlar
        .iter()
        .map(|satir| {
            let (rozet, rozet_sinif) = match satir.durum {
                KarneDurumu::Iyi => ("✓", "iyi"),
                KarneDurumu::Orta => ("!", "orta"),
                KarneDurumu::Eksik => ("✕", "eksik"),
            };
            let etiket = match satir.durum {
                KarneDurumu::Iyi => "tamam",
                KarneDurumu::Orta => "kısmi",
                KarneDurumu::Eksik => "eksik",
            };
            format!(
                r##"<div class="satir">
<span class="rozet {rozet_sinif}">{rozet}</span>
<div class="satir-govde">
<p class="satir-baslik">{baslik} <span class="durum">{etiket}</span></p>
<p class="satir-deger">{deger}</p>
</div>
</div>
"##,
                baslik = html_kacis(satir.baslik),
                deger = html_kacis(&satir.deger),
            )
        })
        .collect();

    let adim_html: String = eksikler
        .iter()
        .enumerate()
        .map(|(sira, satir)| {
            format!(
                r##"<li><span class="adim-no">{}</span><span>{}</span></li>"##,
                sira + 1,
                html_kacis(satir.oneri)
            )
        })
        .collect();

    let adim_bolumu = if eksikler.is_empty() {
        String::new()
    } else {
        format!(
            r##"<h2>Önerilen adımlar</h2>
<ol class="adimlar">{adim_html}</ol>
"##
        )
    };

    let demo_blok = match demo_dosyasi {
        Some(dosya) => format!(
            r##"<section class="demo-kart">
<h2>Hazırlanan site taslağı</h2>
<p>Bu karnedeki eksikleri kapatan örnek sayfa sizin için hazırlandı.</p>
<a class="buton" href="{href}">Taslağı aç</a>
</section>
"##,
            href = html_kacis(dosya)
        ),
        None => String::new(),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{ad} — Dijital Görünürlük Karnesi</title>
{paylasim}
<style>{css}</style>
</head>
<body>
<main class="kabuk">
<p class="ust-etiket">Dijital Görünürlük Karnesi</p>
<h1>{ad}</h1>
<p class="alt-bilgi">{sektor} · {adres}</p>
<p class="kunye">Rapor tarihi: {tarih} · Kaynak: <a href="{kaynak}" target="_blank" rel="noopener">işletme kaydı</a></p>

<section class="skor">
<div class="skor-kart" style="--ton:{ton}">
<p class="etiket">Dijital görünürlük puanı</p>
<p class="skor-puan">{puan}</p>
<p class="skor-alt">100 üzerinden</p>
<p class="skor-sinif">{sinif}</p>
</div>
<div class="ozet-kart">
<p class="etiket">Özet</p>
<p class="ozet">{ozet}</p>
<p class="ozet-sayi">{eksik_sayisi} eksik · {tamam_sayisi} tamam</p>
</div>
</section>

<h2>Tespitler</h2>
<div class="liste">
{satir_html}</div>
{adim_bolumu}
{demo_blok}

<footer class="dipnot">
<p>Bu karne, işletmenizin herkese açık kayıt bilgilerinden (Google Maps) otomatik olarak üretildi; hiçbir bilgi tahmin edilmedi. Eksik görünen bir bilgi varsa kaydınızı güncellemeniz karnedeki puanı da yükseltir.</p>
<p>Sektör: {sektor} · Kayıt: {kaynak}</p>
</footer>
</main>
</body>
</html>
"##,
        css = KARNE_CSS,
        paylasim = paylasim,
        ton = ton,
    )
}
/// Karneyi yazar. Demo ile aynı klasöre gider; `demo_dosyasi` verilirse
/// karneden taslağa göreli bağlantı konur.
fn karne_yaz(
    klasor: &str,
    isletme: &BusinessLead,
    demo_dosyasi: Option<&str>,
) -> Result<String, String> {
    fs::create_dir_all(klasor)
        .map_err(|hata| format!("{klasor} klasörü oluşturulamadı: {hata}"))?;
    let dosya = format!("{klasor}/{}", karne_dosya_adi(isletme));
    fs::write(&dosya, karne_html(isletme, demo_dosyasi))
        .map_err(|hata| format!("{dosya} yazılamadı: {hata}"))?;
    Ok(dosya)
}

/// Karneyi üretir (gerekirse demoyu da üretir) ve tarayıcıda açar.
fn karne_ac(lead: &mut BusinessLead, gonderen: &str, logs: &mut Vec<String>) {
    if lead.demo_yolu.is_none() {
        match demo_uret(lead, gonderen) {
            Ok(yol) => lead.demo_yolu = Some(yol),
            Err(hata) => {
                logs.push(format!("[!] {hata}"));
                return;
            }
        }
    }
    let dosya = format!("{DEMO_KLASORU}/{}", karne_dosya_adi(lead));
    if !std::path::Path::new(&dosya).exists() {
        if let Err(hata) = karne_yaz(DEMO_KLASORU, lead, Some(&demo_dosya_adi(lead))) {
            logs.push(format!("[!] {hata}"));
            return;
        }
    }
    match tarayicida_ac(&dosya) {
        Ok(()) => logs.push(format!(
            "[✓] {} için dijital görünürlük karnesi açıldı: {dosya}",
            lead.name
        )),
        Err(hata) => logs.push(format!("[!] Karne açılamadı: {hata}")),
    }
}

/// Yayın çalışma kopyasının bulunduğu klasör (asıl çalışma ağacının dışında,
/// `gh-pages` dalına bağlı ayrı bir git worktree).
const YAYIN_KLASORU: &str = ".yayin";
/// Demo dosyalarının yayınlandığı dal. GitHub Pages bu dalı kök olarak yayınlar.
const YAYIN_DALI: &str = "gh-pages";

/// Dış komutu sessiz çalıştırır (arayüzden çağrıldığı için konsol penceresi
/// açılmaz) ve çıktısını döndürür; başarısızlıkta `stderr` metnini taşır.
fn komut_calistir(dizin: &str, program: &str, args: &[&str]) -> Result<String, String> {
    let mut emir = crate::stok_video::gizli_komut(program);
    emir.args(args).current_dir(dizin);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: git çağrıları için konsol penceresi açılmasın.
        emir.creation_flags(0x0800_0000);
    }
    let cikti = emir
        .output()
        .map_err(|hata| format!("{program} çalıştırılamadı: {hata}"))?;
    if !cikti.status.success() {
        let hata = String::from_utf8_lossy(&cikti.stderr).trim().to_string();
        return Err(format!("{program} {}: {hata}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&cikti.stdout).to_string())
}

/// Uzak depo adresinden GitHub sahibini ve depo adını çıkarır.
/// `https://github.com/owner/repo.git`, `git@github.com:owner/repo.git` ve
/// sonunda `.git` olmayan biçimler desteklenir.
fn github_depo(uzak: &str) -> Option<(String, String)> {
    let temiz = uzak.trim();
    let kalan = temiz.split("github.com").nth(1)?;
    let kalan = kalan.trim_start_matches(['/', ':']);
    let parcalar: Vec<&str> = kalan.split('/').filter(|parca| !parca.is_empty()).collect();
    if parcalar.len() < 2 {
        return None;
    }
    let depo = parcalar[1].trim_end_matches(".git");
    if depo.is_empty() {
        return None;
    }
    Some((parcalar[0].to_string(), depo.to_string()))
}

/// Bir demo dosyasının paylaşılabilir GitHub Pages adresi:
/// `https://<sahip>.github.io/<depo>/demos/<dosya>`.
fn yayin_adresi(sahip: &str, depo: &str, dosya: &str) -> String {
    format!(
        "https://{}.github.io/{}/demos/{}",
        sahip.to_lowercase(),
        depo,
        dosya
    )
}

/// Yayın kök sayfası: yayınlanmış tüm demoları listeleyen sade bir dizin.
fn yayin_dizini(dosyalar: &[String]) -> String {
    let mut satirlar = String::new();
    let mut sirali: Vec<&String> = dosyalar.iter().collect();
    sirali.sort();
    for dosya in sirali {
        let etiket = dosya
            .trim_end_matches(".html")
            .trim_start_matches("karne-")
            .replace('-', " ");
        satirlar.push_str(&format!(
            "<li><a href=\"demos/{}\">{}</a></li>\n",
            html_kacis(dosya),
            html_kacis(&etiket)
        ));
    }
    format!(
        r##"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Demo yayını</title>
<style>
body{{background:#0b0e13;color:#eaeef5;font-family:system-ui,sans-serif;margin:0;padding:48px 20px}}
main{{max-width:720px;margin:0 auto}}
h1{{font-size:22px;margin:0 0 6px}}
p{{color:#8b95a7;font-size:13px;margin:0 0 24px}}
ul{{list-style:none;padding:0;margin:0;display:grid;gap:8px}}
a{{display:block;padding:12px 14px;border:1px solid #232a36;border-radius:10px;color:#eaeef5;text-decoration:none;background:#12161d}}
a:hover{{border-color:#3a4557}}
</style>
</head>
<body>
<main>
<h1>Hazırlanan demo sayfaları</h1>
<p>Bu bağlantılar taslak amaçlıdır; işletmelerin onayı olmadan yayına alınmaz.</p>
<ul>
{satirlar}</ul>
</main>
</body>
</html>
"##
    )
}

/// `gh-pages` dalına bağlı yayın çalışma kopyasını hazırlar.
fn yayin_hazirla(kok: &str) -> Result<(), String> {
    let klasor = format!("{kok}/{YAYIN_KLASORU}");
    if std::path::Path::new(&klasor).join(".git").exists() {
        return Ok(());
    }
    if std::path::Path::new(&klasor).exists() {
        return Err(format!(
            "{YAYIN_KLASORU} klasörü var ama bir git worktree değil; silip yeniden deneyin."
        ));
    }
    let dal_var = komut_calistir(
        kok,
        "git",
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{YAYIN_DALI}"),
        ],
    )
    .is_ok();
    if dal_var {
        komut_calistir(kok, "git", &["worktree", "add", YAYIN_KLASORU, YAYIN_DALI])
            .map(|_| ())
            .map_err(|hata| format!("{hata} (gerekirse: git worktree prune)"))?;
    } else {
        komut_calistir(kok, "git", &["worktree", "add", "--detach", YAYIN_KLASORU])
            .map_err(|hata| format!("{hata} (gerekirse: git worktree prune)"))?;
        komut_calistir(&klasor, "git", &["checkout", "--orphan", YAYIN_DALI])?;
        // Orphan dalda dizin boşaltılır; hata (boş dizin) yok sayılır.
        let _ = komut_calistir(&klasor, "git", &["rm", "-rf", "."]);
    }
    Ok(())
}

/// Demoyu (ve varsa karneyi) GitHub Pages dalına yayınlar ve paylaşılabilir
/// adresi döndürür. Yalnızca değişen dosyalar commit'lenir; aynı demo ikinci
/// kez yayınlandığında yeni commit oluşmaz.
fn yayina_gonder(kok: &str, dosyalar: &[String], logs: &mut Vec<String>) -> Result<String, String> {
    let klasor = format!("{kok}/{YAYIN_KLASORU}");
    let hedef_klasor = format!("{klasor}/demos");
    fs::create_dir_all(&hedef_klasor)
        .map_err(|hata| format!("{hedef_klasor} oluşturulamadı: {hata}"))?;

    let mut yayinlanan: Vec<String> = Vec::new();
    for kaynak in dosyalar {
        let ad = std::path::Path::new(kaynak)
            .file_name()
            .map(|ad| ad.to_string_lossy().to_string())
            .unwrap_or_default();
        if ad.is_empty() {
            continue;
        }
        // Karne demoya göreli bağlantı verdiği için ikisi de aynı klasöre gider.
        fs::copy(kaynak, format!("{hedef_klasor}/{ad}"))
            .map_err(|hata| format!("{ad} kopyalanamadı: {hata}"))?;
        yayinlanan.push(ad);
    }
    if yayinlanan.is_empty() {
        return Err("Yayınlanacak dosya yok.".to_string());
    }

    // Kök dizin, yayınlanmış tüm demoları listeleyen sayfa olur.
    yayin_dizini_yenile(&klasor)?;
    yayin_isle(
        kok,
        &klasor,
        &format!("Demo yayını: {}", yayinlanan.join(", ")),
        logs,
    )?;
    yayin_adresi_bul(kok, &yayinlanan[0])
}

/// `demos/` klasörünü tarayıp kök dizin sayfasını yeniler; kalan dosyaları
/// döndürür. Yayınlama ve yayından kaldırma aynı dizini kullanır.
fn yayin_dizini_yenile(klasor: &str) -> Result<Vec<String>, String> {
    let hedef_klasor = format!("{klasor}/demos");
    let mut mevcut: Vec<String> = fs::read_dir(&hedef_klasor)
        .map(|kayitlar| {
            kayitlar
                .filter_map(|kayit| kayit.ok())
                .filter(|kayit| {
                    kayit
                        .path()
                        .extension()
                        .is_some_and(|uzanti| uzanti == "html")
                })
                .map(|kayit| kayit.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    mevcut.sort();
    fs::write(format!("{klasor}/index.html"), yayin_dizini(&mevcut))
        .map_err(|hata| format!("index.html yazılamadı: {hata}"))?;
    Ok(mevcut)
}

/// Yayın dalındaki değişiklikleri commit'leyip push eder. Değişiklik yoksa
/// hiçbir şey yapmaz (aynı demo ikinci kez yayınlandığında yeni commit açılmaz).
fn yayin_isle(
    kok: &str,
    klasor: &str,
    mesaj: &str,
    logs: &mut Vec<String>,
) -> Result<bool, String> {
    if komut_calistir(klasor, "git", &["add", "-A"]).is_err() {
        // Boş depoda ilk ekleme; yine de devam edilir.
    }
    let degisiklik = komut_calistir(klasor, "git", &["status", "--porcelain"])?;
    if degisiklik.trim().is_empty() {
        logs.push("[i] Yayında değişiklik yok; mevcut link kullanılabilir.".to_string());
        return Ok(false);
    }
    let ad = komut_calistir(kok, "git", &["config", "user.name"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let eposta = komut_calistir(kok, "git", &["config", "user.email"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let ad = if ad.is_empty() {
        "Demo Yayını".to_string()
    } else {
        ad
    };
    let eposta = if eposta.is_empty() {
        "demo@localhost".to_string()
    } else {
        eposta
    };
    komut_calistir(
        klasor,
        "git",
        &[
            "-c",
            &format!("user.name={ad}"),
            "-c",
            &format!("user.email={eposta}"),
            "commit",
            "-m",
            mesaj,
        ],
    )?;
    komut_calistir(klasor, "git", &["push", "origin", YAYIN_DALI])?;
    Ok(true)
}

/// Yayındaki bir dosyanın herkese açık adresini üretir.
fn yayin_adresi_bul(kok: &str, dosya: &str) -> Result<String, String> {
    let uzak = komut_calistir(kok, "git", &["remote", "get-url", "origin"])?;
    let (sahip, depo) = github_depo(&uzak)
        .ok_or_else(|| format!("origin adresi GitHub deposu değil: {}", uzak.trim()))?;
    Ok(yayin_adresi(&sahip, &depo, dosya))
}

/// Yayındaki bir demoyu (ve karnesini) `gh-pages` dalından kaldırır; dizin
/// sayfası yenilenir ve değişiklik push edilir. Silinen demo linki artık 404
/// döner, yani yanlış ya da süresi geçmiş yayın ortada kalmaz.
fn yayindan_kaldir(kok: &str, demo_adi: &str, logs: &mut Vec<String>) -> Result<(), String> {
    yayin_hazirla(kok)?;
    let klasor = format!("{kok}/{YAYIN_KLASORU}");
    let hedef = format!("{klasor}/demos");
    let dosyalar = [demo_adi.to_string(), format!("karne-{demo_adi}")];
    let mut silinen = 0usize;
    for dosya in &dosyalar {
        let yol = format!("{hedef}/{dosya}");
        if std::path::Path::new(&yol).exists() {
            fs::remove_file(&yol).map_err(|hata| format!("{yol} silinemedi: {hata}"))?;
            silinen += 1;
        }
    }
    if silinen == 0 {
        return Err("Yayında bu demo bulunamadı; link eski olabilir.".to_string());
    }
    yayin_dizini_yenile(&klasor)?;
    yayin_isle(kok, &klasor, &format!("Yayından kaldır: {demo_adi}"), logs)?;
    Ok(())
}

/// Demoyu (yoksa önce üretip) GitHub Pages'e yayınlar ve adresi döndürür.
/// Yayın başarısız olursa `None` döner; çağıranlar akışı durdurmadan devam eder.
/// Karneyi yayına hazırlar: dosya yoksa üretir ve yolunu döndürür. Üretilemezse
/// `None` döner; bu durumda mesaja karne linki eklenmez (kırık link verilmez).
/// Yayındaki demo sayfasının kösesine karne bağlantısı ekler. Böylece mesajda
/// tek link kalır ama karne demo üzerinden tek tıkla açılır.
fn karne_rozeti_ekle(demo_yolu: &str, karne_adi: &str) -> Result<(), String> {
    let html =
        fs::read_to_string(demo_yolu).map_err(|hata| format!("{demo_yolu} okunamadı: {hata}"))?;
    // Aynı demo birden çok kez yayınlanırsa rozet çoğalmasın.
    if html.contains(karne_adi) {
        return Ok(());
    }
    let rozet = format!(
        "<a href=\"{karne_adi}\" style=\"position:fixed;right:16px;bottom:16px;z-index:9999;display:inline-block;background:#0f172a;color:#f8fafc;padding:10px 16px;border-radius:999px;font:600 13px/1.2 system-ui,sans-serif;text-decoration:none;box-shadow:0 6px 20px rgba(15,23,42,.35)\">Eksikler ve puan → dijital karne</a>"
    );
    let yeni = match html.rfind("</body>") {
        Some(yer) => format!("{}{rozet}{}", &html[..yer], &html[yer..]),
        None => format!("{html}{rozet}"),
    };
    fs::write(demo_yolu, yeni).map_err(|hata| format!("{demo_yolu} yazılamadı: {hata}"))
}

fn karne_hazirla(klasor: &str, lead: &BusinessLead, logs: &mut Vec<String>) -> Option<String> {
    let dosya = format!("{klasor}/{}", karne_dosya_adi(lead));
    if !std::path::Path::new(&dosya).exists() {
        if let Err(hata) = karne_yaz(klasor, lead, Some(&demo_dosya_adi(lead))) {
            logs.push(format!("[!] Karne üretilemedi: {hata}"));
        }
    }
    std::path::Path::new(&dosya).exists().then_some(dosya)
}

/// Yayındaki bir dosyanın adresini, demoyla aynı klasördeki başka bir dosya
/// için üretir: `…/demos/x.html` + `karne-x.html` → `…/demos/karne-x.html`.
fn yayin_alt_adresi(demo_adresi: &str, dosya: &str) -> String {
    match demo_adresi.rsplit_once('/') {
        Some((taban, _)) => format!("{taban}/{dosya}"),
        None => demo_adresi.to_string(),
    }
}

fn yayina_gonder_ve_link_al(
    lead: &mut BusinessLead,
    gonderen: &str,
    logs: &mut Vec<String>,
) -> Option<String> {
    if lead.demo_yolu.is_none() {
        match demo_uret(lead, gonderen) {
            Ok(yol) => lead.demo_yolu = Some(yol),
            Err(hata) => {
                logs.push(format!("[!] {hata}"));
                return None;
            }
        }
    }
    let demo_yolu = lead.demo_yolu.clone()?;
    let kok = std::env::current_dir()
        .map(|dizin| dizin.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    if let Err(hata) = yayin_hazirla(&kok) {
        logs.push(format!("[!] Yayın hazırlanamadı: {hata}"));
        return None;
    }

    // Karne demo ile birlikte yayınlanır: dosya yoksa (eski demo, elle açılan
    // dosya) yayından önce burada üretilir ki mesajdaki link boşa çıkmasın.
    let karne_dosyasi = karne_dosya_adi(lead);
    let karne_yolu = karne_hazirla(DEMO_KLASORU, lead, logs);

    // Karne yayına gidecekse demo sayfasına bağlantısı iliştirilir ki mesajda
    // ikinci bir adres yer almasın.
    if karne_yolu.is_some() {
        if let Err(hata) = karne_rozeti_ekle(&demo_yolu, &karne_dosyasi) {
            logs.push(format!("[!] Karne rozeti eklenemedi: {hata}"));
        }
    }

    let mut dosyalar = vec![demo_yolu];
    if let Some(yol) = karne_yolu.clone() {
        dosyalar.push(yol);
    }
    match yayina_gonder(&kok, &dosyalar, logs) {
        Ok(adres) => {
            lead.yayin_linki = Some(adres.clone());
            lead.karne_linki = karne_yolu.map(|_| yayin_alt_adresi(&adres, &karne_dosyasi));
            if let Some(karne_adresi) = lead.karne_linki.clone() {
                logs.push(format!("[i] Dijital karne de yayında: {karne_adresi}"));
            }
            logs.push(format!("[✓] {} yayınlandı: {adres}", lead.name));
            Some(adres)
        }
        Err(hata) => {
            logs.push(format!("[!] Yayın başarısız: {hata}"));
            None
        }
    }
}

/// Demoyu GitHub Pages'e yayınlar, linki günlüğe yazar ve tarayıcıda açar.
/// Yayındaki demoyu ve karnesini kaldırır; linkler kayıttan silinir.
fn demo_yayindan_kaldir(lead: &mut BusinessLead, logs: &mut Vec<String>) {
    let ad = demo_dosya_adi(lead);
    let kok = std::env::current_dir()
        .map(|dizin| dizin.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    match yayindan_kaldir(&kok, &ad, logs) {
        Ok(()) => {
            lead.yayin_linki = None;
            lead.karne_linki = None;
            logs.push(format!(
                "[✓] {} yayından kaldırıldı; link artık açılmaz.",
                lead.name
            ));
        }
        Err(hata) => logs.push(format!("[!] Yayından kaldırılamadı: {hata}")),
    }
}

fn demo_yayinla(lead: &mut BusinessLead, gonderen: &str, logs: &mut Vec<String>) {
    if let Some(adres) = yayina_gonder_ve_link_al(lead, gonderen, logs) {
        if let Err(hata) = tarayicida_ac(&adres) {
            logs.push(format!("[!] Link açılamadı: {hata}"));
        }
    }
}

/// Demos klasörünü dosya yöneticisinde açar.
fn klasoru_ac(yol: &str) -> Result<(), String> {
    let tam = fs::canonicalize(yol).unwrap_or_else(|_| std::path::PathBuf::from(yol));
    let hedef = tam.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    let sonuc = Command::new("explorer").arg(&hedef).spawn();
    #[cfg(target_os = "macos")]
    let sonuc = Command::new("open").arg(&hedef).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let sonuc = Command::new("xdg-open").arg(&hedef).spawn();
    sonuc
        .map(|_| ())
        .map_err(|hata| format!("{hedef} açılamadı: {hata}"))
}

/// Tek dosyayı dosya yöneticisinde seçili olarak gösterir.
fn klasorde_goster(dosya: &str) -> Result<(), String> {
    let tam = fs::canonicalize(dosya).unwrap_or_else(|_| std::path::PathBuf::from(dosya));
    let hedef = tam.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    let sonuc = Command::new("explorer")
        .arg(format!("/select,{hedef}"))
        .spawn();
    #[cfg(not(target_os = "windows"))]
    let sonuc = {
        let ust = tam
            .parent()
            .map(|dizin| dizin.to_string_lossy().to_string())
            .unwrap_or_else(|| hedef.clone());
        klasoru_ac(&ust)
    };
    sonuc
        .map(|_| ())
        .map_err(|hata| format!("{hedef} gösterilemedi: {hata}"))
}

fn tarayicida_ac(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let sonuc = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let sonuc = Command::new("open").arg(url).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let sonuc = Command::new("xdg-open").arg(url).spawn();
    sonuc
        .map(|_| ())
        .map_err(|hata| format!("{url} açılamadı: {hata}"))
}

/// Pencere ve görev çubuğu ikonu: derlemeye gömülü 256×256 PNG.
///
/// Görsel derleme anında `include_bytes!` ile girer, çalışırken dosya aranmaz.
/// Bozuk görsel olursa `None` döner; pencere ikonsuz açılır, uygulama durmaz.
fn uygulama_ikonu() -> Option<egui::IconData> {
    let png = include_bytes!("../varliklar/nexus_256.png");
    let gorsel = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let gorsel = gorsel.to_rgba8();
    let (genislik, yukseklik) = gorsel.dimensions();
    Some(egui::IconData {
        rgba: gorsel.into_raw(),
        width: genislik,
        height: yukseklik,
    })
}

fn calistir() -> eframe::Result<()> {
    let mut gorunum = egui::ViewportBuilder::default()
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1120.0, 720.0])
        // Windows'ta dosya sürükle-bırak olayları varsayılan olarak kapalı
        // olabilir; "2 Medya" alanları için açıkça etkinleştir.
        .with_drag_and_drop(true)
        .with_title("Nexus Enterprise - Müşteri Bulma ve WhatsApp");
    if let Some(ikon) = uygulama_ikonu() {
        gorunum = gorunum.with_icon(ikon);
    }
    let options = eframe::NativeOptions {
        viewport: gorunum,
        ..Default::default()
    };

    // Yerel modeli açılışta hazırla: kapalıysa `ollama serve` sessizce
    // başlatılır. Ayrı iş parçacığında: pencere açılışını bekletmez, hata
    // olsa da uygulama modelsiz modda çalışmayı sürdürür.
    let _ = thread::spawn(ai::otomatik_baslat);

    eframe::run_native(
        "Nexus Enterprise",
        options,
        Box::new(|cc| {
            yazi_tipi_yukle(&cc.egui_ctx);
            tema_uygula(&cc.egui_ctx);
            Box::new(LeadApp::default())
        }),
    )
}

/// Çökme günlüğünün adı; uygulamanın yanında tutulur.
const HATA_DOSYASI: &str = "nexus_hata.log";

/// Günlük dosyasının yolu: önce `.exe` klasörü (kısayoldan açılınca çalışma
/// dizini değişebilir), olmazsa çalışma dizini.
fn hata_yolu() -> std::path::PathBuf {
    let taban = std::env::current_exe()
        .ok()
        .and_then(|yol| yol.parent().map(|dizin| dizin.to_path_buf()))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    taban.join(HATA_DOSYASI)
}

/// Günlüğe tek olay ekler: zaman + başlık, ayrıntı ve ayırıcı.
fn hata_yaz(yol: &std::path::Path, baslik: &str, ayrinti: &str) -> std::io::Result<()> {
    let zaman = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let metin = format!("[{zaman}] {baslik}\n{ayrinti}\n---\n");
    let mut dosya = fs::OpenOptions::new().create(true).append(true).open(yol)?;
    dosya.write_all(metin.as_bytes())
}

/// Olayı günlüğe ve (varsa) konsola yazar; günlük yazılamazsa sessizce geçer
/// ki hata raporu yeni bir hataya yol açmasın.
fn hata_kaydet(baslik: &str, ayrinti: &str) {
    let yol = hata_yolu();
    if hata_yaz(&yol, baslik, ayrinti).is_err() {
        eprintln!("hata günlüğü yazılamadı: {}", yol.display());
    }
    eprintln!("[{baslik}] {ayrinti}");
}

/// Başlatma/çökme hatasını ekranda tutar.
///
/// Uygulama `windows_subsystem = "windows"` ile derlendiği için konsol yoktur;
/// hata yalnız stderr'e yazılsaydı `.exe` "açılıp hemen kapanmış" görünürdü.
#[cfg(windows)]
fn uyari_kutusu(metin: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            sahip: *mut core::ffi::c_void,
            metin: *const u16,
            baslik: *const u16,
            tur: u32,
        ) -> i32;
    }
    let genis =
        |deger: &str| -> Vec<u16> { deger.encode_utf16().chain(std::iter::once(0)).collect() };
    let (icerik, baslik) = (genis(metin), genis("Nexus Enterprise"));
    // 0x10 = MB_ICONERROR · 0x0 = MB_OK
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            icerik.as_ptr(),
            baslik.as_ptr(),
            0x0000_0010,
        );
    }
}

#[cfg(not(windows))]
fn uyari_kutusu(metin: &str) {
    eprintln!("{metin}");
}

/// Panikleri günlüğe ve ekrana taşır; varsayılan davranış (stderr) korunur.
fn panik_kancasi_kur() {
    let onceki = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |bilgi| {
        let konum = bilgi
            .location()
            .map(|yer| yer.to_string())
            .unwrap_or_else(|| "bilinmeyen konum".to_string());
        let mesaj = bilgi.payload_as_str().unwrap_or("bilinmeyen hata");
        let iz = std::backtrace::Backtrace::force_capture();
        hata_kaydet(
            "BEKLENMEYEN HATA (panic)",
            &format!("konum: {konum}\nmesaj: {mesaj}\nyığın izi:\n{iz}"),
        );
        uyari_kutusu(&format!(
            "Nexus Enterprise beklenmeyen bir hatayla karşılaştı.\n\n{mesaj}\n\nKonum: {konum}\n\nAyrıntılar şu dosyaya yazıldı:\n{}",
            hata_yolu().display()
        ));
        onceki(bilgi);
    }));
}

fn main() {
    // Arka plan tedarik servisi komutları GUI açmadan çalışır.
    if let Some(kod) = tedarik_servis::komut_satiri() {
        std::process::exit(kod);
    }
    panik_kancasi_kur();
    if let Err(hata) = calistir() {
        hata_kaydet("AÇILIŞ HATASI", &hata.to_string());
        uyari_kutusu(&format!(
            "Uygulama başlatılamadı.\n\n{hata}\n\nAyrıntılar şu dosyaya yazıldı:\n{}",
            hata_yolu().display()
        ));
        std::process::exit(1);
    }
}

impl LeadApp {
    /// Keşif ekranının alt bölümü: arama sonuçları listesi.
    ///
    /// Liste kendi içinde dikey kaydırılır; dış panelin yüksekliği değişmez.
    pub(crate) fn kesif_sonuclari(&mut self, ui: &mut egui::Ui) {
        let gonderen = whatsapp_numarasi(&self.whatsapp_phone)
            .map(|numara| format!("+{numara}"))
            .unwrap_or_default();
        let sektor = self.sectors[self.selected_sector_idx];
        // Araç çubuğu: sonuç araması solda, işlemler sağda.
        let bugun = bugun_anahtari();
        let takip = self.takip_bekleyenler(&bugun);
        let yeni_sayisi = self
            .leads
            .iter()
            .filter(|lead| lead.durum == musteriler::Durum::Yeni)
            .count();
        ui.horizontal_wrapped(|ui| {
            let arama_alani = 240.0_f32.min(ui.available_width() * 0.5);
            ui.allocate_ui_with_layout(
                egui::vec2(arama_alani, 34.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    arama_kutusu(ui, &mut self.sonuc_arama, "Sonuçlarda ara");
                },
            );

            ui.add_space(6.0);
            {
                if hayalet_buton(ui, "CSV indir").clicked() {
                    match self.defter.csv_kaydet() {
                        Ok(yol) => self.logs.push(format!(
                            "[✓] {} müşteri {yol} dosyasına aktarıldı.",
                            self.defter.ozet().toplam
                        )),
                        Err(hata) => {
                            self.logs.push(format!("[!] CSV yazılamadı: {hata}"));
                        }
                    }
                }
                ui.add_space(6.0);
                if hayalet_buton(ui, "Tümünü seç").clicked() {
                    for lead in self.leads.iter_mut() {
                        lead.selected = true;
                    }
                }
                ui.add_space(6.0);
                if hayalet_buton(ui, "Seçimi temizle").clicked() {
                    for lead in self.leads.iter_mut() {
                        lead.selected = false;
                    }
                }
                ui.add_space(6.0);
            }
        });

        // Günlük çalışma listesi: tümü / yalnızca yeni / takip bekleyen.
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(egui::RichText::new("LİSTE").size(9.5).color(DIM));
            ui.add_space(2.0);
            for (suzgec, etiket) in [
                (ListeSuzgeci::Hepsi, format!("Tümü ({})", self.leads.len())),
                (ListeSuzgeci::Yeni, format!("Yeni ({yeni_sayisi})")),
                (ListeSuzgeci::Takip, format!("Takip ({})", takip.len())),
            ] {
                if filtre_cipi(ui, &etiket, self.liste_suzgeci == suzgec).clicked() {
                    self.liste_suzgeci = suzgec;
                }
            }
            ui.add_space(12.0);
            let kuyruk = self.defter.takip_edilecekler(&bugun, TAKIP_GUN).len();
            if hayalet_buton(ui, &format!("Takip kuyruğu ({kuyruk})")).clicked() {
                self.takip_panel_acik = true;
            }
            let kalan_hedef = self.hedef.kalan();
            if hayalet_buton(
                ui,
                &format!("Hedef ({kalan_hedef}/{})", hedef::GUNLUK_HEDEF),
            )
            .clicked()
            {
                self.bugun_panel_acik = true;
            }
        });
        ui.add_space(10.0);

        if let Some((tamamlanan, toplam)) = self.ilerleme.filter(|_| self.is_running) {
            ilerleme_cubugu(ui, tamamlanan, toplam);
            ui.add_space(10.0);
        }

        // Sonuç listesi
        let liste_yuksekligi = ui.available_height();
        let taranıyor = self.is_running;
        let mut il_geneli_baslat = false;
        let bos_ilce = self.bos_ilce_uyari.clone();
        let il_geneli_simdi = self.il_geneli;
        let mut yeni_detay = self.detay_idx;
        let mut onizleme_idx: Option<usize> = None;
        let limit_doldu = self.gunluk_limit_doldu();
        {
            let arama = arama_anahtari(&self.sonuc_arama);
            let suzgec = self.liste_suzgeci;
            let acik_detay = self.detay_idx;
            // Vurgu tek kareliktir: tüketilmezse her karede kaydırır.
            let vurgu = self.vurgu_kart;
            self.vurgu_kart = None;
            let leads = &mut self.leads;
            let logs = &mut self.logs;
            egui::ScrollArea::vertical()
                .id_source("sonuc_listesi")
                .auto_shrink([false, false])
                .max_height(liste_yuksekligi)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if leads.is_empty() {
                        let uyari = bos_ilce.as_deref().map(|ilce| (ilce, sektor));
                        bos_durum(ui, taranıyor, uyari, il_geneli_simdi, &mut il_geneli_baslat);
                        return;
                    }
                    let mut gosterilen = 0usize;
                    for (idx, lead) in leads.iter_mut().enumerate() {
                        if !arama.is_empty() {
                            let metin = format!("{} {} {}", lead.name, lead.address, lead.category);
                            if !arama_anahtari(&metin).contains(&arama) {
                                continue;
                            }
                        }
                        let suzuldu = match suzgec {
                            ListeSuzgeci::Hepsi => false,
                            ListeSuzgeci::Yeni => lead.durum != musteriler::Durum::Yeni,
                            ListeSuzgeci::Takip => {
                                let anahtar =
                                    musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
                                !takip.contains(&anahtar)
                            }
                        };
                        if suzuldu {
                            continue;
                        }
                        gosterilen += 1;
                        let (detay_iste, wa_iste, kart_rect) = kart_ciz(
                            ui,
                            lead,
                            &gonderen,
                            logs,
                            acik_detay == Some(idx),
                            limit_doldu,
                            vurgu == Some(idx),
                        );
                        // Haritadaki pin seçildiyse kart görünür alana gelir.
                        if vurgu == Some(idx) {
                            ui.scroll_to_rect(kart_rect, Some(egui::Align::Center));
                        }
                        if detay_iste {
                            yeni_detay = if acik_detay == Some(idx) {
                                None
                            } else {
                                Some(idx)
                            };
                        }
                        if detay_iste && acik_detay != Some(idx) {
                            // Listeden seçilen işletme haritada odaklanır.
                        }
                        if wa_iste {
                            onizleme_idx = Some(idx);
                        }
                        ui.add_space(8.0);
                    }
                    if gosterilen == 0 {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Bu filtreyle eşleşen sonuç yok.")
                                .size(11.5)
                                .color(DIM),
                        );
                    }
                });
        }
        if yeni_detay != self.detay_idx {
            self.detay_idx = yeni_detay;
        }
        if let Some(idx) = onizleme_idx {
            self.gonderime_hazirla(idx, &gonderen);
        }
        if il_geneli_baslat {
            self.il_geneli = true;
            self.tarama_baslat();
        }
    }

    /// Günlük hedef listesini tarama sonuçlarından doldurur (en fazla 20).
    /// Telefonsuz, kara listedeki ve listede zaten olan kayıtlar seçilmez.
    fn hedef_listesini_doldur(&mut self) {
        let bugun = bugun_anahtari();
        self.hedef.yeni_gun(&bugun);
        let eksik = hedef::GUNLUK_HEDEF.saturating_sub(self.hedef.kalan());
        if eksik == 0 || self.leads.is_empty() {
            return;
        }
        let adaylar: Vec<(String, bool, bool)> = self
            .leads
            .iter()
            .map(|lead| {
                (
                    musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum),
                    whatsapp_numarasi(&lead.phone).is_some(),
                    lead.durum.kara_listede(),
                )
            })
            .collect();
        let mevcut = self.hedef.anahtarlar();
        let mut yeni: Vec<hedef::Hedef> = Vec::new();
        for idx in hedef::sec(&adaylar, &mevcut, eksik) {
            let lead = &self.leads[idx];
            yeni.push(hedef::Hedef {
                anahtar: adaylar[idx].0.clone(),
                ad: lead.name.clone(),
                adres: lead.address.clone(),
                telefon: lead.phone.clone(),
                sektor: lead.category.clone(),
                konum: lead.konum,
                lead_idx: Some(idx),
                tamam: false,
            });
        }
        for kayit in yeni {
            self.hedef.ekle(kayit);
        }
        self.hedef.kaydet();
    }

    /// Hedefi "Tamamlandı" işaretler: satır listeden düşer, günlük sayaçtan
    /// bir mesaj harcanır (defterde "gönderildi" olarak durur).
    fn hedef_tamamla(&mut self, anahtar: &str) {
        if !self.hedef.tamamla(anahtar) {
            return;
        }
        let bugun = bugun_anahtari();
        self.defter.gonderim_kaydet(anahtar, &bugun);
        self.defter
            .durum_ayarla(anahtar, musteriler::Durum::Gonderildi, &bugun);
        self.hedef.kaydet();
        self.logs.push(format!(
            "[✓] Hedef tamamlandı · bugün {}/{} mesaj · kalan hedef {}",
            self.defter.gun_gonderim(&bugun),
            GUNLUK_GONDERIM_LIMITI,
            self.hedef.kalan()
        ));
    }

    /// Google Places'ten tek sorguda 20 sitesiz işletme çeker ve listeyi yeniler.
    fn hedef_cek(&mut self) {
        if self.is_running {
            self.logs
                .push("[i] Tarama sürüyor; bitmesini bekleyin.".to_string());
            return;
        }
        let il = self.cities[self.selected_city_idx].0.to_string();
        let ilce = if self.il_geneli {
            String::new()
        } else {
            self.cities[self.selected_city_idx].1[self.selected_district_idx].to_string()
        };
        let sektor = self.sectors[self.selected_sector_idx].to_string();
        self.is_running = true;
        self.logs.push(format!(
            "[>] Google Places: {ilce} / {il} · {sektor} · sitesiz hedefler isteniyor..."
        ));
        let tx = self.log_tx.clone();
        thread::spawn(move || match hedef_20(&il, &ilce, &sektor) {
            Ok(liste) => {
                let _ = tx.send(Mesaj::Log(format!(
                    "[✓] Google hedefleri: {} sitesiz kayıt alındı.",
                    liste.len()
                )));
                let _ = tx.send(Mesaj::TaramaBitti(liste));
            }
            Err(hata) => {
                let _ = tx.send(Mesaj::Log(format!("[!] {hata}")));
                let _ = tx.send(Mesaj::TaramaBasarisiz);
            }
        });
    }
}

/// Google Places: bölgedeki web sitesi olmayan işletmeleri tek sayfada çeker.
/// Günlük hedef listesi bu kayıtlardan doldurulur; kotadan bir istek harcar.
fn hedef_20(il: &str, ilce: &str, sektor: &str) -> Result<Vec<BusinessLead>, String> {
    let ayar = ayar_oku();
    let kasa = anahtar::Kasa::yukle();
    let anahtar = kasa.anahtar().ok_or_else(|| {
        "Google API anahtarı yok: üst şeritteki API & Kota Yönetim panelinden girin.".to_string()
    })?;
    let istemci = google::Istemci::yeni(
        anahtar.to_string(),
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        1,
        false,
    )?;
    let sonuc = istemci.ara(il, ilce, sektor)?;
    let mut liste: Vec<BusinessLead> = sonuc
        .kayitlar
        .into_iter()
        .map(|kayit| isletmeye_cevir(kayit, sektor))
        .filter(|lead| telefon_gecerli(&lead.phone))
        .collect();
    if liste.is_empty() {
        return Err(format!(
            "Google {ilce} / {il} için sitesiz işletme döndürmedi ({} web siteli kayıt elendi)",
            sonuc.siteli
        ));
    }
    incelemeye_gore_sirala(&mut liste);
    liste.truncate(hedef::GUNLUK_HEDEF);
    Ok(liste)
}

// ---------------------------------------------------------------------------
// Keşif motoru: yalnızca Google Maps Platform
//
// Tek veri kaynağı Google Places API (New) Text Search ve Geocoding'dir.
// Anahtar şifreli kasada durur, her istek kotadan pay ayırır; harita paneli
// ağa hiç çıkmaz. Aynı ilçe tekrar taranırken sonuç önbellekten okunur.
// ---------------------------------------------------------------------------

/// Google Places + Geocoding taraması. Anahtar şifreli kasadan okunur;
/// yoksa tarama başlamaz ve kullanıcı API & Kota Yönetim paneline yönlendirilir.
fn tarama_calistir(
    tx: Sender<Mesaj>,
    il: String,
    ilce: Option<String>,
    sektor: String,
    onay: bool,
) {
    let ayar = ayar_oku();
    let kasa = anahtar::Kasa::yukle();
    let Some(anahtar) = kasa.anahtar().map(str::to_string) else {
        let _ = tx.send(Mesaj::Log(
            "[!] Google API anahtarı yok: üst şeritteki 'API & KOTA YÖNETİM' düğmesinden anahtarınızı kaydedin."
                .to_string(),
        ));
        let _ = tx.send(Mesaj::TaramaBasarisiz);
        return;
    };
    let istemci = match google::Istemci::yeni(
        anahtar,
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        ayar.google_sayfa,
        onay,
    ) {
        Ok(istemci) => istemci,
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
            return;
        }
    };
    let ozet = google::ozet(istemci.gunluk_limit, istemci.aylik_limit);
    let _ = tx.send(Mesaj::Log(format!(
        "[i] Google API hazır · günlük {}/{} istek · sayfa {}/tarama.",
        ozet.gunluk, ozet.gunluk_limit, istemci.sayfa_siniri
    )));

    let mut konum_onbellek = KonumOnbellek::yukle();
    let ilce_adi = ilce.clone().unwrap_or_default();
    let konum = onbellek_konum(&istemci, &mut konum_onbellek, &il, &ilce_adi, &tx);
    match ilce {
        Some(ilce) => tarama_tek_ilce(&tx, &istemci, &il, &ilce, &sektor, konum.as_ref()),
        None => tarama_il_geneli(&tx, &istemci, &il, &sektor, konum.as_ref()),
    }
}

/// İl/ilçe merkezini Google Geocoding'den çözer; sonuç `konum_onbellek.json`a
/// yazılır, aynı bölge bir daha faturalandırılmaz (180 gün).
fn onbellek_konum(
    istemci: &google::Istemci,
    onbellek: &mut KonumOnbellek,
    il: &str,
    ilce: &str,
    tx: &Sender<Mesaj>,
) -> Option<Konum> {
    if !ilce.trim().is_empty() {
        if let Some(konum) = onbellek.al(il, ilce) {
            return Some(konum);
        }
    }
    match istemci.geocode(il, ilce) {
        Ok((enlem, boylam, yaricap_m)) => {
            let konum = Konum {
                enlem,
                boylam,
                yaricap_m,
            };
            let kayit_adi = if ilce.trim().is_empty() {
                "il merkezi"
            } else {
                ilce
            };
            onbellek.ekle(il, kayit_adi, &konum);
            onbellek.kaydet();
            let _ = tx.send(Mesaj::Log(format!(
                "[i] Google Geocoding: {enlem:.4}, {boylam:.4} · ~{} km yarıçap.",
                (yaricap_m / 1000).max(1)
            )));
            Some(konum)
        }
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] Google Geocoding: {hata}")));
            None
        }
    }
}

/// Tek ilçe taraması: önbellek → Google Places.
fn tarama_tek_ilce(
    tx: &Sender<Mesaj>,
    istemci: &google::Istemci,
    il: &str,
    ilce: &str,
    sektor: &str,
    konum: Option<&Konum>,
) {
    let mut onbellek = SonucOnbellek::yukle();
    let merkez = konum
        .map(|konum| {
            format!(
                " · merkez {:.4}, {:.4} · ~{} km",
                konum.enlem,
                konum.boylam,
                (konum.yaricap_m / 1000).max(1)
            )
        })
        .unwrap_or_default();
    let _ = tx.send(Mesaj::Log(format!(
        "[>] Google Places: {il} / {ilce} · {sektor} (en fazla {SONUC_SINIRI} kayıt){merkez}..."
    )));
    match ilce_ara(tx, istemci, &mut onbellek, il, ilce, sektor) {
        Ok((liste, onbellekten)) => {
            if !onbellekten && !liste.is_empty() {
                let _ = tx.send(Mesaj::Log(
                    "[i] Veri kaynağı: Google Places API (New) · Geocoding · yalnızca sitesiz işletmeler."
                        .to_string(),
                ));
            }
            let yuksek = liste
                .iter()
                .filter(|isletme| potansiyel_puani(isletme) >= SICAK_ESIK)
                .count();
            let _ = tx.send(Mesaj::Log(format!(
                "[✓] {} sitesiz potansiyel müşteri bulundu · {yuksek} sıcak fırsat.",
                liste.len()
            )));
            let _ = tx.send(Mesaj::TaramaBitti(liste));
        }
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {ilce} taranamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
        }
    }
}

/// İlçe sonucu: önce yerel önbellek, yoksa Google Places. Dönen bayrak
/// sonucun önbellekten geldiğini söyler (günlükte ayrı yazılır).
fn ilce_ara(
    tx: &Sender<Mesaj>,
    istemci: &google::Istemci,
    onbellek: &mut SonucOnbellek,
    il: &str,
    ilce: &str,
    sektor: &str,
) -> Result<(Vec<BusinessLead>, bool), String> {
    if let Some(kayit) = onbellek.al(il, ilce, sektor) {
        let _ = tx.send(Mesaj::Log(format!(
            "[i] {ilce}: sonuçlar önbellekten alındı ({} kayıt, {} gün önce); Google'a istek atılmadı.",
            kayit.leads.len(),
            kayit.gun
        )));
        return Ok((kayit.leads, true));
    }
    let sonuc = istemci.ara(il, ilce, sektor)?;
    if let Some(neden) = &sonuc.durduruldu {
        let _ = tx.send(Mesaj::Log(format!("[!] {neden}")));
    }
    let _ = tx.send(Mesaj::Log(format!(
        "[i] Google Places: {} ham kayıt · {} web siteli işletme elendi · {} sayfa.",
        sonuc.kayitlar.len() + sonuc.siteli,
        sonuc.siteli,
        sonuc.sayfa
    )));
    let mut leads: Vec<BusinessLead> = sonuc
        .kayitlar
        .into_iter()
        .map(|kayit| isletmeye_cevir(kayit, sektor))
        .collect();
    // Telefonu olmayan kayıt müşteri adayı değildir.
    leads.retain(|lead| telefon_gecerli(&lead.phone));
    incelemeye_gore_sirala(&mut leads);
    leads_sirala(&mut leads);
    leads.truncate(SONUC_SINIRI);
    // Kota engeli yüzünden boş dönen sonuç önbelleğe yazılmaz: yarın aynı
    // bölge yeniden denenebilmeli.
    if !(leads.is_empty() && sonuc.durduruldu.is_some()) {
        onbellek.ekle(il, ilce, sektor, &leads, sonuc.siteli);
        onbellek.kaydet();
    }
    Ok((leads, false))
}

/// İl geneli: tek Places sorgusu ile ilin tamamı aranır; ilçe ilçe gezmek
/// düzinelerce faturalı istek üretirdi.
fn tarama_il_geneli(
    tx: &Sender<Mesaj>,
    istemci: &google::Istemci,
    il: &str,
    sektor: &str,
    _konum: Option<&Konum>,
) {
    let _ = tx.send(Mesaj::Ilerleme {
        tamamlanan: 0,
        toplam: 1,
    });
    let _ = tx.send(Mesaj::Log(format!(
        "[>] {il} il geneli: Google Places tek sorguda ili tarıyor ({sektor})..."
    )));
    let mut onbellek = SonucOnbellek::yukle();
    let bulunanlar = match ilce_ara(tx, istemci, &mut onbellek, il, "", sektor) {
        Ok((liste, _)) => liste,
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {il} taranamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
            return;
        }
    };
    let _ = tx.send(Mesaj::Ilerleme {
        tamamlanan: 1,
        toplam: 1,
    });
    let yuksek = bulunanlar
        .iter()
        .filter(|isletme| potansiyel_puani(isletme) >= SICAK_ESIK)
        .count();
    let _ = tx.send(Mesaj::Log(format!(
        "[✓] {il} il geneli bitti: {} sitesiz kayıt ({} sıcak fırsat); telefonu olmayan kayıt listeye alınmadı.",
        bulunanlar.len(),
        yuksek
    )));
    let _ = tx.send(Mesaj::TaramaBitti(bulunanlar));
}

/// Google kaydını arayüzün satır modeline çevirir.
fn isletmeye_cevir(kayit: google::BizKayit, sektor: &str) -> BusinessLead {
    BusinessLead {
        name: kayit.ad,
        address: kayit.adres,
        category: sektor.to_string(),
        phone: kayit.telefon,
        selected: true,
        kaynak: kayit.kaynak,
        konum: kayit.konum,
        calisma_saatleri: kayit.saatler,
        demo_yolu: None,
        mesaj_hazir: false,
        kayit_dolulugu: kayit.doluluk,
        site_linki: String::new(),
        son_guncelleme_gun: None,
        inceleme_sayisi: kayit.inceleme,
        yildiz_puani: kayit.yildiz,
        durum: musteriler::Durum::Yeni,
        yayin_linki: None,
        karne_linki: None,
        mesaj_varyanti: None,
        foto: None,
        fotolar: Vec::new(),
        medya: Vec::new(),
    }
}

/// Stok video arka plan işinden gelen olaylar: arayüz hiç beklemez.
/// Yerel video içe aktarma işinin arka plan olayları.
enum StokOlay {
    Hazir(stok_video::Hazir),
    Hata(String),
    Bitti,
}

impl LeadApp {
    /// Manuel medya akışının tek giriş noktası: tarayıcı onay paneli.
    /// Panelde eski otomatik tarama düğmeleri yoktur; medya yalnız buradan eklenir.
    fn manuel_medya_paneli(&self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.separator();
        bolum_etiketi(ui, "MANUEL MEDYA");
        ui.label(
            egui::RichText::new(
                "Medya ekleme ve onay tarayıcı panelinde yapılır. Panel yalnızca yapıştırdığınız linki veya yüklediğiniz dosyayı işler; AI kontrolü, önizleme, ONAYLA ve REDDET adımları burada tamamlanır.",
            )
            .size(9.8)
            .color(DIM),
        );
        ui.label(
            egui::RichText::new("Başlatmak için: ollama_proje.exe --onizleme")
                .size(9.8)
                .color(SOLUK),
        );
        ui.hyperlink_to(medya_web::ADRES, medya_web::ADRES);
    }

    fn stok_paneli(&mut self, ui: &mut egui::Ui) {
        let mut olaylar = Vec::new();
        if let Some(kanal) = &self.stok_islem {
            while let Ok(olay) = kanal.try_recv() {
                olaylar.push(olay);
            }
        }
        for olay in olaylar {
            match olay {
                StokOlay::Hazir(hazir) => {
                    self.stok_durum = format!(
                        "Kaydedildi: media/sektor/{}/{} · {:.1} MB video · {:.0} KB poster · {:.0} sn kullanıldı.",
                        hazir.grup,
                        hazir.dosya_adi,
                        hazir.video_bayt as f64 / 1_048_576.0,
                        hazir.poster_bayt as f64 / 1024.0,
                        hazir.sure
                    );
                    self.stok_kutuphane = stok_video::kutuphane();
                }
                StokOlay::Hata(hata) => self.stok_durum = hata,
                StokOlay::Bitti => {
                    self.stok_islem = None;
                    self.stok_mesgul = false;
                }
            }
        }

        bolum_etiketi(ui, "SİNEMATİK VARLIKLAR");
        ui.label(
            egui::RichText::new(
                "Yerel video · mp4, mov, webm · 720p · her kare anahtar kare · sessiz · en fazla 8 sn",
            )
            .size(9.8)
            .color(DIM),
        );
        if !stok_video::ffmpeg_var() {
            ui.label(
                egui::RichText::new(
                    "ffmpeg bulunamadı: video işlenemez. https://ffmpeg.org/download.html",
                )
                .size(10.0)
                .color(TURUNCU),
            );
        }
        ui.add_space(4.0);

        ui.horizontal_wrapped(|ui| {
            for grup in stok_video::GRUPLAR {
                let secili = self.stok_grup == grup.id;
                let metin = egui::RichText::new(grup.ad).size(10.0).color(if secili {
                    KOYU_YAZI
                } else {
                    METIN
                });
                let buton = egui::Button::new(metin)
                    .fill(if secili { VURGU } else { KART })
                    .rounding(egui::Rounding::same(999.0))
                    .min_size(egui::vec2(0.0, 22.0));
                if ui.add(buton).clicked() && !secili {
                    self.stok_grup = grup.id.to_string();
                }
            }
        });
        ui.add_space(4.0);

        // Fotoğraftan sinematik döngü: yerel dosya, ffmpeg, dış kaynak yok.
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Fotoğraftan sinematik döngü · Ken Burns zoom-pan · 720p · 8 sn · sessiz",
            )
            .size(9.8)
            .color(DIM),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.stok_foto_yol)
                    .hint_text("C:\\fotoğraflar\\cephe.jpg")
                    .desired_width(190.0),
            );
            if ui
                .add_enabled(!self.stok_mesgul, egui::Button::new("Hareketlendir"))
                .clicked()
            {
                let grup = self.stok_grup.clone();
                let yol = self.stok_foto_yol.trim().to_string();
                if yol.is_empty() {
                    self.stok_durum = "Önce bir fotoğraf yolu girin.".to_string();
                } else {
                    let (tx, rx) = channel();
                    self.stok_islem = Some(rx);
                    self.stok_mesgul = true;
                    self.stok_durum = "Fotoğraf hareketli döngüye çevriliyor…".to_string();
                    thread::spawn(move || {
                        match stok_video::foto_ken_burns(&grup, &yol) {
                            Ok(hazir) => {
                                let _ = tx.send(StokOlay::Hazir(hazir));
                            }
                            Err(hata) => {
                                let _ = tx.send(StokOlay::Hata(hata));
                            }
                        }
                        let _ = tx.send(StokOlay::Bitti);
                    });
                }
            }
        });

        // Sürükle-bırak: pencereye bırakılan dosya yol alanına düşer.
        let birakilan: Option<std::path::PathBuf> = ui.ctx().input(|girdi| {
            girdi
                .raw
                .dropped_files
                .iter()
                .find_map(|dosya| dosya.path.clone())
        });
        if let Some(yol) = birakilan {
            self.stok_yol = yol.to_string_lossy().to_string();
            self.stok_durum = format!(
                "Dosya seçildi: {}",
                yol.file_name()
                    .map(|ad| ad.to_string_lossy().to_string())
                    .unwrap_or_default()
            );
        }
        let ustunde = ui.ctx().input(|girdi| !girdi.raw.hovered_files.is_empty());
        let kenar = if ustunde {
            VURGU
        } else {
            egui::Color32::from_rgb(52, 52, 58)
        };
        egui::Frame::none()
            .fill(KART)
            .stroke(egui::Stroke::new(1.0_f32, kenar))
            .rounding(10.0)
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(
                        "Videoyu bu pencereye sürükleyin ya da tam dosya yolunu yapıştırın",
                    )
                    .size(10.5)
                    .color(DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_yol)
                        .hint_text("C:\\video\\kuafor.mp4")
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Başlangıç sn").size(10.0).color(DIM));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.stok_baslangic).desired_width(56.0),
                    );
                    ui.label(
                        egui::RichText::new("8 sn'den uzun videoda en iyi bölümü seçer")
                            .size(9.5)
                            .color(DIM),
                    );
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_linki)
                        .hint_text("Kaynak bağlantısı (isteğe bağlı)")
                        .desired_width(f32::INFINITY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_cekim)
                        .hint_text("Çekimi yapan (isteğe bağlı)")
                        .desired_width(f32::INFINITY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_lisans)
                        .hint_text("Lisans notu (isteğe bağlı)")
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(&mut self.stok_atif_goster, "Altbilgide atıf göster");
                ui.add_space(4.0);
                if ui
                    .add_enabled(!self.stok_mesgul, egui::Button::new("Dönüştür ve kaydet"))
                    .clicked()
                {
                    let ctx = ui.ctx().clone();
                    self.stok_baslat(ctx);
                }
            });

        if !self.stok_durum.is_empty() {
            ui.label(
                egui::RichText::new(&self.stok_durum)
                    .size(10.5)
                    .color(if self.stok_mesgul { DIM } else { YESIL }),
            );
        }

        let kutuphane = self.stok_kutuphane.clone();
        if kutuphane.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Kayıtlı video yok; üretimde shader'lı sinematik sürüm kullanılır.",
                )
                .size(10.0)
                .color(DIM),
            );
        } else {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("KAYITLI VİDEOLAR").size(9.5).color(DIM));
            ui.label(
                egui::RichText::new(
                    "Aynı grupta birden fazla video varsa işletme adından türeyen tohumla biri seçilir.",
                )
                .size(9.5)
                .color(DIM),
            );
            let mut silinecek: Option<(String, String)> = None;
            for bilgi in &kutuphane {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} · {}", bilgi.grup, bilgi.dosya_adi))
                            .size(10.0)
                            .color(YESIL),
                    )
                    .on_hover_text(bilgi.kunye());
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.1} MB · {:.0} KB",
                            bilgi.video_bayt as f64 / 1_048_576.0,
                            bilgi.poster_bayt as f64 / 1024.0
                        ))
                        .size(9.5)
                        .color(DIM),
                    );
                    if ui.small_button("Sil").clicked() {
                        silinecek = Some((bilgi.grup.clone(), bilgi.dosya_adi.clone()));
                    }
                });
            }
            if let Some((grup, dosya)) = silinecek {
                match stok_video::sil_kayit(&grup, &dosya) {
                    Ok(()) => {
                        self.stok_kutuphane = stok_video::kutuphane();
                        self.stok_durum = format!("{dosya} silindi.");
                    }
                    Err(hata) => self.stok_durum = hata,
                }
            }
        }
    }

    fn stok_baslat(&mut self, ctx: egui::Context) {
        if self.stok_mesgul {
            return;
        }
        let istek = stok_video::YerelIstek {
            kaynak_yol: self.stok_yol.trim().to_string(),
            baslangic: self
                .stok_baslangic
                .trim()
                .replace(',', ".")
                .parse::<f64>()
                .unwrap_or(0.0),
            kaynak_linki: self.stok_linki.trim().to_string(),
            cekim: self.stok_cekim.trim().to_string(),
            lisans: self.stok_lisans.trim().to_string(),
            atif_gerekli: self.stok_atif_goster,
        };
        let grup = self.stok_grup.clone();
        let (gonderen, kanal) = channel();
        self.stok_islem = Some(kanal);
        self.stok_mesgul = true;
        self.stok_durum = "Video dönüştürülüyor: 720p, her kare anahtar kare, sessiz…".to_string();
        thread::spawn(move || {
            let sonuc = stok_video::yerel_hazirla(&grup, &istek);
            let _ = gonderen.send(match sonuc {
                Ok(hazir) => StokOlay::Hazir(hazir),
                Err(hata) => StokOlay::Hata(hata),
            });
            let _ = gonderen.send(StokOlay::Bitti);
            ctx.request_repaint();
        });
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn cokme_gunlugu_dosyaya_eklenir() {
        let yol = std::env::temp_dir().join(format!("nexus_hata_{}.log", std::process::id()));
        let _ = fs::remove_file(&yol);
        hata_yaz(&yol, "DENEME", "ilk ayrıntı").expect("günlük yazılamadı");
        hata_yaz(&yol, "DENEME2", "ikinci ayrıntı").expect("günlük eklenemedi");
        let icerik = fs::read_to_string(&yol).expect("günlük okunamadı");
        assert!(icerik.contains("DENEME2"), "başlık yazılmadı: {icerik}");
        assert!(icerik.contains("ikinci ayrıntı"), "ayrıntı yazılmadı");
        assert_eq!(icerik.matches("---").count(), 2, "olaylar ayrılmalı");
        assert!(hata_yolu().ends_with(HATA_DOSYASI), "günlük adı sabit");
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn turkce_karakterler_url_kodlanir() {
        assert_eq!(
            url_encode("Beşiktaş, İstanbul"),
            "Be%C5%9Fikta%C5%9F%2C%20%C4%B0stanbul"
        );
    }

    /// Sayfadaki etiketleri atıp görünen metni tek satıra indirir. Manşet
    /// kelime kelime maskelendiği için cümle aramaları burada yapılır.
    #[cfg(test)]
    pub(crate) fn ornek_isletme() -> BusinessLead {
        BusinessLead {
            name: "Kafe <Deneme>".to_string(),
            address: "Beşiktaş / İstanbul".to_string(),
            category: "Restoran".to_string(),
            phone: "+90 532 123 45 67".to_string(),
            selected: true,
            kaynak: "https://www.openstreetmap.org/node/42".to_string(),
            konum: Some((41.04285, 29.00753)),
            calisma_saatleri: "Mo-Fr 09:00-18:00; Sa 10:00-16:00".to_string(),
            kayit_dolulugu: 0,
            site_linki: String::new(),
            son_guncelleme_gun: Some(30),
            inceleme_sayisi: None,
            yildiz_puani: None,
            durum: musteriler::Durum::Yeni,
            demo_yolu: None,
            mesaj_hazir: false,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: Vec::new(),
        }
    }

    #[test]
    fn eski_ayar_dosyasinda_sinematik_alanlari_varsayilana_duser() {
        let ayar: UygulamaAyari = serde_json::from_str(r#"{"demo_profil":"sinematik"}"#)
            .expect("eksik alanlı ayar okunmalı");
        assert_eq!(ayar.demo_profil, "sinematik");
        assert_eq!(ayar.demo_sinematik_hareket, "orta");
        assert_eq!(ayar.demo_sinematik_nesne, "otomatik");
        assert_eq!(ayar.demo_sinematik_sahne, 4);
    }

    /// Kuaför prototipi: `cargo test --all-targets sinematik_kuafor -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn sinematik_kuafor_demo_uret() {
        let lead = BusinessLead {
            name: "Karizma Erkek Kuaförü".to_string(),
            address: "Caferağa Mah., Kadıköy / İstanbul".to_string(),
            category: "Kuaför".to_string(),
            phone: "+90 216 444 55 66".to_string(),
            selected: true,
            kaynak: "https://www.openstreetmap.org/node/102".to_string(),
            konum: Some((40.99012, 29.02666)),
            calisma_saatleri: "Tu-Sa 10:00-20:00; Su 11:00-17:00".to_string(),
            kayit_dolulugu: 3,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: Some(154),
            yildiz_puani: Some(4.8),
            durum: musteriler::Durum::Yeni,
            demo_yolu: None,
            mesaj_hazir: false,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: Vec::new(),
        };
        let mut girdi = demo_girdi_profille(&lead, "+90 500 000 00 00", false, Some("sinematik"));
        girdi.satici_tel = Some("+90 500 000 00 00".to_string());
        let (html, kirpilan) = demo_site::uret_butceli(&girdi, DEMO_BUTCE);
        fs::create_dir_all("demos/sinematik").expect("klasör");
        fs::write("demos/sinematik/karizma-kuafor.html", &html).expect("yazılmalı");
        println!(
            "sinematik kuaför: {} B · bütçe için çıkarılan kare: {kirpilan}",
            html.len()
        );
    }

    /// Fotoğraf hero'su: havuzda videosu olmayan sektör Ken Burns ile açılır.
    #[test]
    #[ignore]
    fn sinematik_dental_demo_uret() {
        let lead = BusinessLead {
            name: "Vita Ağız ve Diş Sağlığı".to_string(),
            address: "Kızılay, Çankaya / Ankara".to_string(),
            category: "Diş Kliniği".to_string(),
            phone: "+90 312 222 33 44".to_string(),
            selected: true,
            kaynak: "https://www.openstreetmap.org/node/103".to_string(),
            konum: Some((39.92077, 32.85411)),
            calisma_saatleri: "Mo-Fr 09:00-19:00; Sa 09:00-14:00".to_string(),
            kayit_dolulugu: 2,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: Some(168),
            yildiz_puani: Some(4.9),
            durum: musteriler::Durum::Yeni,
            demo_yolu: None,
            mesaj_hazir: false,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: Vec::new(),
        };
        let girdi = demo_girdi_profille(&lead, "+90 500 000 00 00", false, Some("sinematik"));
        let (html, kirpilan) = demo_site::uret_butceli(&girdi, DEMO_BUTCE);
        fs::create_dir_all("demos/sinematik").expect("klasör");
        fs::write("demos/sinematik/vita-dis-klinigi.html", &html).expect("yazılmalı");
        println!(
            "sinematik klinik: {} B · kırpılan kare: {kirpilan}",
            html.len()
        );
    }

    fn kuafor_lead() -> BusinessLead {
        BusinessLead {
            name: "Karizma Erkek Kuaförü".to_string(),
            address: "Caferağa Mah., Kadıköy / İstanbul".to_string(),
            category: "Kuaför".to_string(),
            phone: "+90 216 444 55 66".to_string(),
            selected: true,
            kaynak: "https://www.openstreetmap.org/node/102".to_string(),
            konum: Some((40.99012, 29.02666)),
            calisma_saatleri: "Tu-Sa 10:00-20:00; Su 11:00-17:00".to_string(),
            kayit_dolulugu: 3,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: Some(154),
            yildiz_puani: Some(4.8),
            durum: musteriler::Durum::Yeni,
            demo_yolu: None,
            mesaj_hazir: false,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: Vec::new(),
        }
    }

    /// Video'lu sinematik klasör çıktısı: `media/sektor/kuafor` varsa
    /// `demos/sinematik/<işletme>/index.html` + `media/` üretir.
    /// `cargo test --all-targets stok_video_klasor -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn stok_video_klasor_demo_uret() {
        let lead = kuafor_lead();
        let tohum = stok_video::tohum(&lead.name, &lead.address);
        // Havuzdan video hero seçilmiyorsa bu işletme hareketli fotoğrafla
        // açılır: klasör çıktısı testi anlamlı olmaz.
        if stok_video::video_bilgisi("kuafor").is_none()
            && sektor_medya::video("kuafor").is_none()
            && medya_havuz::hero_video("kuafor", tohum).is_none()
        {
            println!("media/sektor/kuafor, media/stock/kuafor ve havuz videosu yok; test atlandı");
            return;
        }
        let yol = demo_yaz_profille("demos", &lead, "+90 500 000 00 00", Some("sinematik"))
            .expect("klasör demo yazılmalı");
        let video = "demos/sinematik/karizma-erkek-kuaforu-102/media/hero.mp4";
        let html_bayt = fs::metadata(&yol).expect("index.html yok").len();
        let video_bayt = fs::metadata(video).expect("hero.mp4 yok").len();
        println!("video klasör demo: {yol} · html {html_bayt} B · video {video_bayt} B");
        assert!(yol.ends_with("index.html"));
        assert!(html_bayt > 10_000, "sayfa çok küçük");
        assert!(video_bayt > 10_000, "video kopyalanmadı");
    }

    /// Teşhis: havuz dağıtımı ve demoya giren kare sayısı.
    #[test]
    #[ignore]
    fn havuz_dagitimi_yazdir() {
        let lead = kuafor_lead();
        let tohum = stok_video::tohum(&lead.name, &lead.address);
        let dagitim = medya_havuz::dagit("kuafor", tohum, 4);
        println!(
            "video {:?} · video_hero {} · kareler {:?}",
            dagitim.video.as_ref().map(|v| v.dosya.clone()),
            dagitim.video_hero,
            dagitim.kareler
        );
        for kare in &dagitim.kareler {
            println!(
                "kare {:?} → {:?}",
                kare,
                yerel_foto(&kare.to_string_lossy()).map(|foto| foto.0)
            );
        }
        let girdi = demo_girdi_profille(&lead, "+90 500 000 00 00", false, Some("sinematik"));
        println!(
            "foto {} · galeri {} · hikaye {}",
            girdi.foto.is_some(),
            girdi.galeri.len(),
            girdi.hikaye_foto.is_some()
        );
    }

    /// Mevcut demolara yeni medya akışını uygular: her kaynak anlık
    /// görüntüsünden sinematik klasör demo üretir (gerçek video ya da Ken
    /// Burns fotoğraf). Eski tek dosya kopyası klasör çıktısıyla değiştirilir.
    /// `cargo test --all-targets mevcut_demolari -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn mevcut_demolari_sinematik_yenile() {
        let mut yenilenen = 0;
        let mut atlanan = 0;
        let Ok(girdiler) = fs::read_dir("demos/kaynak") else {
            println!("demos/kaynak yok; test atlandı");
            return;
        };
        let mut yollar: Vec<std::path::PathBuf> = girdiler
            .flatten()
            .map(|giris| giris.path())
            .filter(|yol| yol.extension().and_then(|uzanti| uzanti.to_str()) == Some("json"))
            .collect();
        yollar.sort();
        for yol in yollar {
            let Ok(metin) = fs::read_to_string(&yol) else {
                atlanan += 1;
                continue;
            };
            let Ok(kaynak) = serde_json::from_str::<DemoKaynagi>(&metin) else {
                atlanan += 1;
                continue;
            };
            // Bogazici klasik şablon fikstürüdür: profili korunur.
            let profil = if kaynak.isletme.name.contains("Bogazici") {
                "klasik"
            } else {
                "sinematik"
            };
            match demo_yaz_profille("demos", &kaynak.isletme, &kaynak.gonderen, Some(profil)) {
                Ok(cikti) => {
                    println!("{profil} · {} · {cikti}", kaynak.isletme.name);
                    if cikti.ends_with("index.html") {
                        let eski = format!("demos/{}", demo_dosya_adi(&kaynak.isletme));
                        if eski != cikti && std::path::Path::new(&eski).is_file() {
                            let _ = fs::remove_file(&eski);
                        }
                    }
                    yenilenen += 1;
                }
                Err(hata) => {
                    println!("atlandı · {} · {hata}", kaynak.isletme.name);
                    atlanan += 1;
                }
            }
        }
        println!("yenilenen {yenilenen} · atlanan {atlanan}");
        assert!(yenilenen > 0, "en az bir demo yenilenmeli");
    }

    /// Yerel içe aktarma uçtan uca: örnek videoyu 720p / en çok 8 sn'ye
    /// çevirip `media/sektor/kuafor` altına yazar, tohumla seçip demo üretir.
    /// `cargo test --all-targets yerel_ice -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn yerel_ice_aktarma_ve_uretim() {
        let istek = stok_video::YerelIstek {
            kaynak_yol: "demos/sinematik/karizma-erkek-kuaforu-102/media/hero.mp4".to_string(),
            baslangic: 0.0,
            kaynak_linki: "https://ornek.test/kuafor-video".to_string(),
            cekim: "Nexus test çekimi".to_string(),
            lisans: "Yalnız test".to_string(),
            atif_gerekli: true,
        };
        let hazir = stok_video::yerel_hazirla("kuafor", &istek).expect("içe aktarma");
        println!(
            "içe aktarıldı: {} · poster {} · video {} B · poster {} B",
            hazir.dosya_adi, hazir.poster_adi, hazir.video_bayt, hazir.poster_bayt
        );
        let lead = kuafor_lead();
        let bilgi = stok_video::sec("kuafor", stok_video::tohum(&lead.name, &lead.address))
            .expect("tohumla seçim");
        println!("tohum seçimi: {}", bilgi.dosya_adi);
        assert!(bilgi.sayfa_atfi().is_some(), "atıf altbilgiye girmeli");
        let yol = demo_yaz_profille("demos", &lead, "+90 500 000 00 00", Some("sinematik"))
            .expect("demo yazılmalı");
        println!("video'lu sinematik: {yol}");
    }

    /// Geçici: üç sektörden örnek demo üretip demos/ altına yazar (elle incelemek için).
    #[test]
    #[ignore]
    fn yalniz_elle_incelerken_gecici_demo_uret() {
        let ornekler = vec![
            (
                "Bogazici Meyhane",
                "Restoran",
                "Cihangir, Beyoğlu / İstanbul",
                "+90 532 111 22 33",
                "https://www.openstreetmap.org/node/101",
                Some((41.03175, 28.98302)),
                "Mo-Sa 12:00-23:30; Su 12:00-22:00",
                Some(4.6_f32),
                Some(312_u32),
            ),
            (
                "Karizma Erkek Kuaförü",
                "Kuaför",
                "Caferağa, Kadıköy / İstanbul",
                "+90 216 444 55 66",
                "https://www.openstreetmap.org/node/102",
                Some((40.99012, 29.02666)),
                "Tu-Sa 10:00-20:00; Su 11:00-17:00",
                Some(4.8_f32),
                Some(154_u32),
            ),
            (
                "Dentapol Diş Kliniği",
                "Diş Kliniği",
                "Kızılay, Çankaya / Ankara",
                "+90 312 222 33 44",
                "https://www.openstreetmap.org/node/103",
                Some((39.92077, 32.85411)),
                "Mo-Fr 09:00-19:00; Sa 09:00-14:00",
                None,
                None,
            ),
        ];
        fs::create_dir_all("demos").expect("demos klasörü");
        for (ad, sektor, adres, tel, kaynak, konum, saat, yildiz, yorum) in ornekler {
            let lead = BusinessLead {
                name: ad.to_string(),
                address: adres.to_string(),
                category: sektor.to_string(),
                phone: tel.to_string(),
                selected: true,
                kaynak: kaynak.to_string(),
                konum,
                calisma_saatleri: saat.to_string(),
                kayit_dolulugu: 0,
                site_linki: String::new(),
                son_guncelleme_gun: None,
                inceleme_sayisi: yorum,
                yildiz_puani: yildiz,
                durum: musteriler::Durum::Yeni,
                demo_yolu: None,
                mesaj_hazir: false,
                yayin_linki: None,
                karne_linki: None,
                mesaj_varyanti: None,
                // Elle inceleme: meyhane demosuna gerçek bir Commons fotoğrafı
                // (CC BY-SA 4.0) eklenir; fotoğrafsız hâli diğer demolar gösterir.
                fotolar: Vec::new(),
                medya: Vec::new(),
                foto: if sektor == "Restoran" {
                    Some(Foto {
                        url: "commons:Restaurant_Näsinneula.jpg".to_string(),
                        atif: String::new(),
                    })
                } else {
                    None
                },
            };
            let dosya = demo_dosya_adi(&lead);
            let demo_yolu = format!("demos/{dosya}");
            fs::write(
                &demo_yolu,
                demo_site::kucult(&demo_html(&lead, "+90 500 000 00 00")),
            )
            .expect("demo yazılmalı");
            let karne_yolu = format!("demos/{}", karne_dosya_adi(&lead));
            fs::write(&karne_yolu, karne_html(&lead, Some(dosya.as_str())))
                .expect("karne yazılmalı");
            println!("yazıldı: {demo_yolu}");
        }
    }

    /// Elle deneme: tek bir çekici demosu üretir (acil katmanı görmek için).
    #[test]
    #[ignore]
    fn tek_deneme_demo_uret() {
        let lead = BusinessLead {
            name: "Gece Oto Kurtarma".to_string(),
            address: "Paşa Alanı, 10020 Balıkesir".to_string(),
            category: "Çekici".to_string(),
            phone: "+90 532 000 00 00".to_string(),
            selected: true,
            kaynak: "https://www.openstreetmap.org/node/9042".to_string(),
            konum: Some((39.646, 27.886)),
            calisma_saatleri: "Mo-Su 00:00-24:00".to_string(),
            kayit_dolulugu: 0,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: Some(87),
            yildiz_puani: Some(4.7),
            durum: musteriler::Durum::Yeni,
            demo_yolu: None,
            mesaj_hazir: false,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: Vec::new(),
        };
        let yol = demo_yaz("demos", &lead, "+90 500 000 00 00").expect("demo yazılmalı");
        println!("deneme demo: {yol}");
    }

    /// Rejim incelemesi: üç sektörden birer demo üretir; sırasıyla
    /// neon-acil (acil), luxe-sinema (mutfak) ve temiz-cam (sağlık) rejimleri.
    #[test]
    #[ignore]
    fn uc_rejim_demo_uret() {
        let ornekler = [
            (
                "Gece Oto Kurtarma",
                "Çekici",
                "Paşa Alanı, 10020 Balıkesir",
                "+90 532 000 00 00",
                9042_u64,
                (39.646_f64, 27.886_f64),
                "Mo-Su 00:00-24:00",
                Some(4.7_f32),
                Some(87_u32),
            ),
            (
                "Narin Ocak",
                "Restoran",
                "Cihangir, Beyoğlu / İstanbul",
                "+90 212 333 44 55",
                9043_u64,
                (41.03175_f64, 28.98302_f64),
                "Tu-Su 18:00-01:00",
                Some(4.8_f32),
                Some(241_u32),
            ),
            (
                "Vita Ağız ve Diş Sağlığı",
                "Diş Kliniği",
                "Kızılay, Çankaya / Ankara",
                "+90 312 222 33 44",
                9044_u64,
                (39.92077_f64, 32.85411_f64),
                "Mo-Fr 09:00-19:00; Sa 09:00-14:00",
                Some(4.9_f32),
                Some(168_u32),
            ),
        ];
        for (ad, sektor, adres, tel, kimlik, konum, saat, yildiz, yorum) in ornekler {
            let lead = BusinessLead {
                name: ad.to_string(),
                address: adres.to_string(),
                category: sektor.to_string(),
                phone: tel.to_string(),
                selected: true,
                kaynak: format!("https://www.openstreetmap.org/node/{kimlik}"),
                konum: Some(konum),
                calisma_saatleri: saat.to_string(),
                kayit_dolulugu: 0,
                site_linki: String::new(),
                son_guncelleme_gun: None,
                inceleme_sayisi: yorum,
                yildiz_puani: yildiz,
                durum: musteriler::Durum::Yeni,
                demo_yolu: None,
                mesaj_hazir: false,
                yayin_linki: None,
                karne_linki: None,
                mesaj_varyanti: None,
                foto: None,
                fotolar: Vec::new(),
                medya: Vec::new(),
            };
            let yol = demo_yaz("demos", &lead, "+90 500 000 00 00").expect("demo yazılmalı");
            println!("rejim demosu ({sektor}): {yol}");
        }
    }

    #[test]
    fn potansiyeli_yuksek_isletmeler_once_siralanir() {
        let mut telefonlu_saatli = ornek_isletme();
        telefonlu_saatli.name = "Zeynep Restoran".to_string();

        let mut telefonlu_a = ornek_isletme();
        telefonlu_a.name = "Işık Kafe".to_string();
        telefonlu_a.calisma_saatleri.clear();

        let mut telefonlu_b = ornek_isletme();
        telefonlu_b.name = "İnci Kafe".to_string();
        telefonlu_b.calisma_saatleri.clear();

        let mut saatli = ornek_isletme();
        saatli.name = "Ahmet Berber".to_string();
        saatli.phone.clear();

        let mut ciplak = ornek_isletme();
        ciplak.name = "Burak Kafe".to_string();
        ciplak.phone.clear();
        ciplak.calisma_saatleri.clear();

        let mut liste = vec![ciplak, saatli, telefonlu_a, telefonlu_b, telefonlu_saatli];
        leads_sirala(&mut liste);

        let adlar: Vec<&str> = liste.iter().map(|isletme| isletme.name.as_str()).collect();
        assert_eq!(
            adlar,
            vec![
                "Zeynep Restoran",
                "İnci Kafe",
                "Işık Kafe",
                "Ahmet Berber",
                "Burak Kafe"
            ],
            "telefon+saat > telefon > saat > hiçbiri; eşit puanda Türkçe alfabetik"
        );
    }

    #[test]
    fn potansiyel_puani_niyet_ve_ulasilabilirligi_puanlar() {
        let mut tam = ornek_isletme();
        tam.address = "Barbaros Bulvarı No:12, Beşiktaş / İstanbul".to_string();
        assert_eq!(
            potansiyel_puani(&tam),
            86,
            "site yok 12 + mobil telefon 30 + saat 8 + adres 8 + güncel 5 + sektör 20 + konum 3"
        );
        assert!(potansiyel_gerekcesi(&tam).starts_with("86/100 · mobil numara"));

        let mut incelemeli = ornek_isletme();
        incelemeli.inceleme_sayisi = Some(150);
        assert!(
            potansiyel_gerekcesi(&incelemeli).contains("150 inceleme"),
            "inceleme sayısı gerekçede görünmeli"
        );

        let mut ciplak = ornek_isletme();
        ciplak.phone.clear();
        ciplak.calisma_saatleri.clear();
        ciplak.address = "Beşiktaş / İstanbul".to_string();
        ciplak.konum = None;
        ciplak.son_guncelleme_gun = None;
        assert_eq!(
            potansiyel_puani(&ciplak),
            32,
            "yalnızca 'site yok' + sektör puanı kalır"
        );
        assert!(potansiyel_puani(&ciplak) < SICAK_ESIK);
    }

    #[test]
    fn konum_onbellegi_kaydeder_ve_okur() {
        let mut onbellek = KonumOnbellek::default();
        assert!(onbellek.al("İstanbul", "Beşiktaş").is_none());

        onbellek.ekle(
            "İstanbul",
            "Beşiktaş",
            &Konum {
                enlem: 41.04,
                boylam: 29.0,
                yaricap_m: 3_000,
            },
        );
        let konum = onbellek
            .al("İstanbul", "Beşiktaş")
            .expect("önbellekten gelmeli");
        assert_eq!(konum.yaricap_m, 3_000);
        assert!(
            onbellek.al("istanbul", "besiktas").is_some(),
            "Türkçe karakter ve büyük/küçük harf farkı aynı kaydı bulmalı"
        );

        let metin = serde_json::to_string(&onbellek).expect("json");
        let geri: KonumOnbellek = serde_json::from_str(&metin).expect("json");
        assert!(geri.al("İstanbul", "Beşiktaş").is_some());

        // 180 günden eski kayıt geçersiz sayılır.
        let anahtar = KonumOnbellek::anahtar("İstanbul", "Beşiktaş");
        if let Some(kayit) = onbellek.kayitlar.get_mut(&anahtar) {
            kayit.tarih = "2020-01-01".to_string();
        }
        assert!(onbellek.al("İstanbul", "Beşiktaş").is_none());
    }

    #[test]
    fn gun_farki_tarihleri_cozer() {
        assert_eq!(gun_farki("2026-10-01", "2026-10-04"), Some(3));
        assert_eq!(gun_farki("2026-10-04", "2026-10-01"), Some(-3));
        assert_eq!(gun_farki("bozuk", "2026-10-01"), None);
        assert_eq!(gun_farki("2026-10-01", "2026-10-01"), Some(0));
    }

    #[test]
    fn demo_renkleri_degistirilir_icerik_korunur() {
        let html = "<!DOCTYPE html>\n<html lang=\"tr\">\n<head><meta name=\"theme-color\" content=\"#e8a33d\">\
<style>:root{--accent:#e8a33d;--accent-karsi:#12100e;--bg:#08080a;--ink:#f5f3ef;--f-baslik:x}</style>\
</head><body><h1>Ateşin dili</h1></body></html>";

        let yeni = demo_renklerini_degistir(html, "#f4efe5", "#1b1a17", Some("#b4551f"))
            .expect("kök renkler bulunmalı");
        assert!(yeni.contains("--bg:#f4efe5"), "zemin değişti");
        assert!(yeni.contains("--ink:#1b1a17"), "yazı rengi değişti");
        assert!(yeni.contains("--accent:#b4551f"), "vurgu değişti");
        assert!(
            yeni.contains("--accent-karsi:#f8f5f0"),
            "koyu vurgu üzerinde açık yazı"
        );
        assert!(yeni.contains("--f-baslik:x"), "diğer değişkenler korunur");
        assert!(yeni.contains("Ateşin dili"), "içerik korunur");
        assert!(
            yeni.contains("<html lang=\"tr\" class=\"tema-acik\">"),
            "açık zeminde tema sınıfı eklenir"
        );
        assert!(
            yeni.contains("content=\"#b4551f\""),
            "tarayıcı çubuğu rengi de güncellenir"
        );
    }

    #[test]
    fn karanlik_zemine_donunce_tema_sinifi_kalkar() {
        let html = "<html lang=\"tr\" class=\"tema-acik\">\
<style>:root{--accent:#b4551f;--accent-karsi:#f8f5f0;--bg:#f4efe5;--ink:#1b1a17;}</style></html>";
        let yeni = demo_renklerini_degistir(html, "#08080a", "#f5f3ef", None)
            .expect("kök renkler bulunmalı");
        assert!(
            yeni.contains("<html lang=\"tr\">"),
            "koyu temada sınıf kaldırılır"
        );
        assert!(!yeni.contains("tema-acik\">"), "sınıf kalmamalı");
    }

    #[test]
    fn kok_rengi_olmayan_sayfa_atlanir() {
        let html = "<html><body>elle yazılmış sayfa</body></html>";
        assert!(demo_renklerini_degistir(html, "#f4efe5", "#1b1a17", None).is_none());
    }

    #[test]
    fn kaynaksiz_demolara_yalniz_renk_islenir() {
        let kok = std::env::temp_dir().join(format!("renk-testi-{}", std::process::id()));
        let demos = kok.join("demos");
        fs::create_dir_all(&demos).expect("klasör");
        let demo = demos.join("kafe-1.html");
        fs::write(
            &demo,
            "<html lang=\"tr\"><style>:root{--accent:#e8a33d;--accent-karsi:#12100e;--bg:#08080a;--ink:#f5f3ef;}</style></html>",
        )
        .expect("demo");
        let karne = demos.join("karne-kafe-1.html");
        fs::write(
            &karne,
            "<html><style>:root{--accent:#123456;}</style></html>",
        )
        .expect("karne");
        let yabanci = demos.join("notlar.html");
        fs::write(&yabanci, "<html>not</html>").expect("not");

        let (yenilenen, renklenen, atlanan) = demolara_ayarlari_uygula(
            demos.to_str().expect("yol"),
            &SonucOnbellek::default(),
            "#f4efe5",
            "#1b1a17",
            None,
            "",
        );
        assert_eq!(yenilenen, 0, "kaynak verisi yok, sayfa üretilmez");
        assert_eq!(renklenen, 1, "yalnız demo renklendirilir");
        assert_eq!(atlanan, 1, "kök rengi olmayan sayfa atlanır");

        let okunan = fs::read_to_string(&demo).expect("demo okunur");
        assert!(okunan.contains("--bg:#f4efe5"), "yeni zemin yazıldı");
        assert!(
            fs::read_to_string(&karne)
                .expect("karne")
                .contains("--accent:#123456"),
            "karne dosyasına dokunulmaz"
        );
        let _ = fs::remove_dir_all(&kok);
    }

    /// Demo yazılırken kaynak anlık görüntüsü de saklanır: sayfa uygulama
    /// kapandıktan sonra bile aynı işletme verisinden yeniden üretilebilir.
    #[test]
    fn demo_kaynagi_anlik_goruntuden_okunur() {
        let kok = std::env::temp_dir().join(format!("demo-kaynak-{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(&kok).expect("klasör");
        let klasor = kok.to_str().expect("yol");
        let mut isletme = ornek_isletme();
        // Ağ isteği olmasın: koordinatsız ve fotoğrafsız kayıt.
        isletme.konum = None;
        // Onay kilidi seçilmiş sektör tablosunu kapsar; bu test anlık görüntü
        // okumayı denetlediği için tablo dışı bir sektör kullanır.
        isletme.category = "Matbaa".to_string();

        let dosya = demo_yaz(klasor, &isletme, "+905551112233").expect("demo yazılmalı");
        let dosya_adi = demo_dosya_adi(&isletme);
        assert!(dosya.ends_with(&dosya_adi));

        let kaynak = demo_kaynagi_bul(klasor, &dosya_adi, &SonucOnbellek::default(), "")
            .expect("anlık görüntü bulunmalı");
        assert_eq!(kaynak.isletme.name, isletme.name);
        assert_eq!(kaynak.isletme.phone, isletme.phone);
        assert_eq!(kaynak.isletme.address, isletme.address);
        assert_eq!(
            kaynak.isletme.calisma_saatleri, isletme.calisma_saatleri,
            "çalışma saatleri kaybolmamalı"
        );
        assert_eq!(kaynak.gonderen, "+905551112233");
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn gercek_demo_dosyasi_yeniden_renklendirilir() {
        let kaynak = std::path::Path::new("demos/bogazici-meyhane-101.html");
        if !kaynak.exists() {
            return; // demolar üretilmemişse atlanır
        }
        let kok = std::env::temp_dir().join(format!("gercek-demo-{}", std::process::id()));
        fs::create_dir_all(&kok).expect("klasör");
        let hedef = kok.join("meyhane.html");
        fs::copy(kaynak, &hedef).expect("kopya");

        let (yenilenen, renklenen, atlanan) = demolara_ayarlari_uygula(
            kok.to_str().expect("yol"),
            &SonucOnbellek::default(),
            "#08130f",
            "#eef7f1",
            Some("#3ecf8e"),
            "",
        );
        assert_eq!(
            (yenilenen, renklenen, atlanan),
            (0, 1, 0),
            "tek demo renklendirilir"
        );

        let html = fs::read_to_string(&hedef).expect("okunur");
        assert!(html.contains("--bg:#08130f"), "zemin yazıldı");
        assert!(html.contains("--ink:#eef7f1"), "yazı rengi yazıldı");
        assert!(html.contains("--accent:#3ecf8e"), "vurgu yazıldı");
        assert!(
            html.contains("<html lang=\"tr\">"),
            "koyu zeminde açık tema sınıfı kalkar"
        );
        // Yazı tipi yığını ve metin içeriği renk değişiminden etkilenmez.
        assert!(html.contains("Space Grotesk"), "font yığını korunur");
        assert!(html.contains("--f-govde:"), "gövde font değişkeni korunur");
        assert!(html.len() > 3000, "sayfa içeriği korunur");

        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn bugun_sirasi_karalisteyi_ve_gonderilenleri_atlar() {
        let mut cep = ornek_isletme();
        cep.name = "Cep Telefonlu".to_string();
        let mut sabit = ornek_isletme();
        sabit.name = "Sabit Hatlı".to_string();
        sabit.phone = "+90 212 444 62 62".to_string();
        let mut telsiz = ornek_isletme();
        telsiz.name = "Telefonsuz".to_string();
        telsiz.phone.clear();
        let mut retli = ornek_isletme();
        retli.name = "Ret Veren".to_string();
        retli.durum = musteriler::Durum::Reddetti;
        let mut gonderilmis = ornek_isletme();
        gonderilmis.name = "Bugün Yazılan".to_string();

        let liste = vec![cep, sabit, telsiz, retli, gonderilmis];
        let sira = bugun_sirasi(&liste, 15, |lead| lead.name == "Bugün Yazılan");
        assert_eq!(
            sira,
            vec![0, 1],
            "telefonu olan, kara listede olmayan ve bugün yazılmamış olanlar"
        );

        // Günlük hak daralınca liste kesilir.
        assert_eq!(bugun_sirasi(&liste, 1, |_| false), vec![0]);
        assert!(bugun_sirasi(&liste, 0, |_| false).is_empty(), "hak kalmadı");
    }

    #[test]
    fn sabit_hat_ile_cep_numarasi_ayrilir() {
        assert!(mobil_numara("+90 532 123 45 67"), "cep numarası");
        assert!(mobil_numara("0532 123 45 67"), "başında sıfır");
        assert!(mobil_numara("5321234567"), "yalnız rakam");
        assert!(!mobil_numara("+90 212 444 62 62"), "İstanbul sabit hat");
        assert!(!mobil_numara("0312 222 33 44"), "Ankara sabit hat");
        assert!(!mobil_numara(""), "boş numara");
    }

    #[test]
    fn uzun_mesaj_baglanti_sinirinda_kirpilir() {
        let kisa = "Merhaba, siteniz hazır.";
        assert_eq!(mesaj_sinirla(kisa), kisa, "kısa mesaj değişmez");

        let uzun = "a".repeat(MESAJ_SINIRI + 50);
        let kirpilan = mesaj_sinirla(&uzun);
        assert_eq!(
            kirpilan.chars().count(),
            MESAJ_SINIRI + 1,
            "sınırda kesilip üç nokta eklenir"
        );
        assert!(kirpilan.ends_with('…'));
    }

    #[test]
    fn hazir_mesaj_baglantiya_sigar() {
        let mesaj = whatsapp_mesaji(&ornek_isletme());
        assert!(
            mesaj.chars().count() <= MESAJ_SINIRI,
            "mesaj {} karakter; sınır {MESAJ_SINIRI}",
            mesaj.chars().count()
        );
    }

    #[test]
    fn arayuz_acik_tema() {
        // Beyaz zemin + koyu yeşil vurgu; vurgu üzerindeki yazı beyaz.
        assert!(
            ARKA_PLAN.r() > 230 && ARKA_PLAN.g() > 230 && ARKA_PLAN.b() > 230,
            "sayfa zemini açık olmalı"
        );
        assert_eq!(KART, egui::Color32::WHITE, "kartlar beyaz");
        assert!(
            VURGU.g() > VURGU.r() && VURGU.g() > VURGU.b(),
            "vurgu yeşil olmalı"
        );
        assert!(VURGU.g() < 140, "vurgu koyu yeşil olmalı");
        assert_eq!(KOYU_YAZI, egui::Color32::WHITE, "vurgu üzerinde beyaz yazı");
        assert!(
            METIN.r() < 80 && METIN.g() < 80 && METIN.b() < 80,
            "metin koyu olmalı"
        );
    }

    #[test]
    fn ton_beyaza_koyu_murekkebe_gider() {
        let acik = ton(VURGU, 0.12);
        assert!(
            acik.r() > 200 && acik.g() > 200 && acik.b() > 200,
            "ton açık bir dolgu üretmeli"
        );
        let koyu_renk = koyu(VURGU, 0.45);
        assert!(
            koyu_renk.r() < VURGU.r() + 40 && koyu_renk.g() < VURGU.g(),
            "koyu() rengi metin yönünde koyulaştırmalı"
        );
    }

    #[test]
    fn whatsapp_numarasi_uluslararasi_bicime_cevrilir() {
        assert_eq!(
            whatsapp_numarasi("+90 532 123 45 67").as_deref(),
            Some("905321234567")
        );
        assert_eq!(
            whatsapp_numarasi("0532 123 45 67").as_deref(),
            Some("905321234567")
        );
        assert_eq!(
            whatsapp_numarasi("532 123 45 67").as_deref(),
            Some("905321234567")
        );
        assert_eq!(
            whatsapp_numarasi("0212 444 62 62").as_deref(),
            Some("902124446262")
        );
        assert!(whatsapp_numarasi("telefon yok").is_none());
    }

    #[test]
    fn demo_dosyasi_adi_turkce_karakterleri_sadeler() {
        assert_eq!(dosya_slug("Şişli Çiçekçi & Kahve"), "sisli-cicekci-kahve");
    }

    #[test]
    fn odeme_hatirlatmasi_kalan_bakiyeyi_ve_linki_yazar() {
        let kayit = musteriler::Musteri {
            anahtar: "osm:node/1".to_string(),
            ad: "Moda Kafe".to_string(),
            telefon: "+90 532 123 45 67".to_string(),
            kategori: "Kafe".to_string(),
            paket: Some(musteriler::Paket::CokSayfa),
            teklif_tutari: Some(9000),
            odeme: musteriler::Odeme::Kismi,
            tahsil_edilen: Some(3000),
            yayin_linki: Some("https://ornek.github.io/r/demos/kafe-1.html".to_string()),
            ..musteriler::Musteri::default()
        };
        let mesaj = odeme_hatirlatma_mesaji(&kayit);
        assert!(mesaj.contains("Moda Kafe"), "{mesaj}");
        assert!(mesaj.contains("6000 ₺"), "kalan bakiye yazılmalı: {mesaj}");
        assert!(mesaj.contains("Çok sayfa"), "paket adı yazılmalı: {mesaj}");
        assert!(mesaj.contains("https://ornek.github.io/r/demos/kafe-1.html"));
        assert!(mesaj.contains("RET"));
    }

    #[test]
    fn karne_html_paylasim_etiketleri_icerir() {
        let html = karne_html(&ornek_isletme(), None);
        assert!(html.contains("<meta property=\"og:title\""), "{html}");
        assert!(html.contains("og:description"));
        assert!(html.contains("Dijital Görünürlük Karnesi"));
        assert!(html.contains("name=\"robots\" content=\"noindex\""));
    }

    #[test]
    fn karne_harici_kaynak_yuklemez() {
        let html = karne_html(&ornek_isletme(), None);
        assert!(!html.contains("cdn.tailwindcss.com"), "CDN kalmamalı");
        assert!(!html.contains("<script src="), "harici script olmamalı");
        assert!(!html.contains("<link "), "harici stil olmamalı");
        assert!(html.contains("<style>"), "gömülü stil bulunmalı");
    }

    #[test]
    fn yayin_dizini_kalan_dosyalari_listeler() {
        let kok = std::env::temp_dir().join(format!(
            "nexus-yayin-{}-{}",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let demos = kok.join("demos");
        fs::create_dir_all(&demos).expect("dizin kurulmalı");
        fs::write(demos.join("b-2.html"), "<html></html>").expect("dosya");
        fs::write(demos.join("a-1.html"), "<html></html>").expect("dosya");
        fs::write(demos.join("notlar.txt"), "metin").expect("dosya");

        let kalan = yayin_dizini_yenile(&kok.to_string_lossy()).expect("dizin yenilenmeli");
        assert_eq!(kalan, vec!["a-1.html".to_string(), "b-2.html".to_string()]);
        let index = fs::read_to_string(kok.join("index.html")).expect("index yazılmalı");
        assert!(index.contains("demos/a-1.html") && index.contains("demos/b-2.html"));
        assert!(
            !index.contains("notlar.txt"),
            "HTML olmayan dosya listelenmez"
        );
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn kayitli_numara_acilista_yuklenir() {
        let ayar = UygulamaAyari {
            whatsapp_phone: "+90 532 123 45 67".to_string(),
            ..UygulamaAyari::default()
        };
        let (telefon, durum, bagli) = whatsapp_baslangic(&ayar);
        assert_eq!(telefon, "+90 532 123 45 67");
        assert!(bagli, "geçerli numara kayıtlı sayılmalı");
        assert_eq!(durum, "Kayıtlı: +905321234567");

        // Kayıt yoksa uydurma numara gösterilmez.
        let (telefon, durum, bagli) = whatsapp_baslangic(&UygulamaAyari::default());
        assert!(telefon.is_empty());
        assert!(!bagli);
        assert_eq!(durum, "Numara kaydedilmedi");
    }

    #[test]
    fn ayarlar_diske_yazilip_geri_okunur() {
        let yol = std::env::temp_dir().join(format!(
            "nexus-ayar-{}-{}.json",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let yol = yol.to_string_lossy().to_string();

        // Dosya yokken varsayılan (boş numara) döner.
        assert_eq!(ayar_oku_yoldan(&yol).whatsapp_phone, "");

        let ayar = UygulamaAyari {
            whatsapp_phone: "+90 532 123 45 67".to_string(),
            demo_vurgu: "#5b8cff".to_string(),
            demo_zemin: "#f6f1e7".to_string(),
            demo_yazi: "#1b1a17".to_string(),
            ..UygulamaAyari::default()
        };
        ayar_yaz_yoldan(&yol, &ayar).expect("ayar yazılmalı");
        let okunan = ayar_oku_yoldan(&yol);
        assert_eq!(okunan.whatsapp_phone, "+90 532 123 45 67");
        // Renk seçimleri de kalıcı: yeni açılışta demolara uygulanır.
        assert_eq!(okunan.demo_zemin, "#f6f1e7");
        assert_eq!(okunan.demo_vurgu, "#5b8cff");
        assert_eq!(okunan.demo_yazi, "#1b1a17");
        assert_eq!(
            whatsapp_numarasi(&okunan.whatsapp_phone).as_deref(),
            Some("905321234567")
        );
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn demo_html_paylasim_etiketleri_icerir() {
        let html = demo_html(&ornek_isletme(), "+90 500 111 22 33");
        assert!(html.contains("<meta property=\"og:title\""), "{html}");
        assert!(html.contains("og:description"));
        assert!(html.contains("og:locale\" content=\"tr_TR"));
        assert!(html.contains("name=\"description\""));
        // Başlık işletme adını ve sektörü taşır; kaçışlama korunur.
        assert!(html.contains("og:site_name\" content=\"Kafe &lt;Deneme&gt;"));
    }

    #[test]
    fn takip_mesaji_gun_sayisini_ve_linki_yazar() {
        let mut kayit = musteriler::Musteri {
            anahtar: "osm:node/1".to_string(),
            ad: "Moda Kafe".to_string(),
            telefon: "+90 532 123 45 67".to_string(),
            kategori: "Kafe".to_string(),
            ..musteriler::Musteri::default()
        };
        let bugun = bugun_anahtari();
        kayit.son_gonderim = Some(
            chrono::Local::now()
                .date_naive()
                .checked_sub_days(chrono::Days::new(5))
                .map(|gun| gun.format("%Y-%m-%d").to_string())
                .unwrap_or(bugun.clone()),
        );
        kayit.yayin_linki = Some("https://ornek.github.io/r/demos/kafe-1.html".to_string());
        let mesaj = takip_mesaji(&kayit);
        assert!(mesaj.contains("Moda Kafe"), "{mesaj}");
        assert!(mesaj.contains("5 gün önce"), "{mesaj}");
        assert!(mesaj.contains("https://ornek.github.io/r/demos/kafe-1.html"));
        assert!(mesaj.contains("RET"), "mesajın çıkış yolu olmalı");

        // Yayın linki yoksa mesaj linksiz ama tutarlı kalır.
        kayit.yayin_linki = None;
        let linksiz = takip_mesaji(&kayit);
        assert!(!linksiz.contains("http"), "{linksiz}");
        assert!(linksiz.contains("Moda Kafe"));
    }

    #[test]
    fn demo_html_iskelet_bilesenlerini_icerir() {
        let html = demo_html(&ornek_isletme(), "+90 500 111 22 33");
        // Sabit menü, mobil menü ve alt eylem barı (mobilde tek dokunuş).
        assert!(html.contains("class=\"ust\""), "sabit menü olmalı");
        assert!(
            html.contains("id=\"mobil-dugme\""),
            "mobil menü düğmesi olmalı"
        );
        assert!(html.contains("id=\"mobil-menu\""));
        assert!(
            html.contains("class=\"mobil-bar\""),
            "alt eylem barı olmalı"
        );
        assert!(html.contains("mobil-konum"));
        // Sektör rengi CSS değişkeni olarak gömülür.
        assert!(html.contains("--accent:#"), "sektör rengi gömülmeli");
        // Scroll animasyonu ve hareket azaltma tercihi.
        assert!(html.contains("IntersectionObserver"));
        assert!(html.contains("prefers-reduced-motion"));
        // JS kapalıysa içerik görünür kalır.
        assert!(html.contains("<noscript>"));
    }
}
