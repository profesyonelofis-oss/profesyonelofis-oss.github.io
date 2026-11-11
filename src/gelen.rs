//! Gelen kutusu: `.exe` köprüsünden gelen/giden WhatsApp mesajları.
//!
//! Katman 5'in arkasındaki gerçek iş burada döner. Uygulama açıldığında
//! 127.0.0.1 üzerinde küçük bir HTTP dinleyici başlar; köprü (veya demo
//! sayfasındaki `.exe` etiketi) buraya JSON bırakır:
//!
//! - `POST /gelen`  → müşteriden gelen mesaj (`telefon`, `ad`, `metin`)
//! - `POST /gonder` → bizden çıkan mesaj (köprü gönderdiyse geri bildirim)
//! - `GET  /durum`  → `{"ok":true,"surum":1}` (arayüz oturum yoklaması)
//!
//! Dışarıya hiçbir bağlantı açılmaz, port yalnız yerel arayüze bağlanır.
//! Mesajlar `gelen_kutusu.json` dosyasında tutulur; niyet (sıcak / fiyat /
//! beklemede / ret) anahtar sözcüklerle yerelde sınıflanır, modele gitmez.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Gelen kutusunun kalıcı dosyası.
pub const DOSYA: &str = "gelen_kutusu.json";

/// Köprünün varsayılan yerel portu (demo sayfasındaki köprü de bunu kullanır).
pub const VARSAYILAN_PORT: u16 = 8777;

/// Defterde tutulan en fazla mesaj; eskiler atılır.
const EN_FAZLA_KAYIT: usize = 600;

/// Mesajın yönü.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Yon {
    #[default]
    Gelen,
    Giden,
}

impl Yon {
    pub fn etiket(self) -> &'static str {
        match self {
            Yon::Gelen => "gelen",
            Yon::Giden => "giden",
        }
    }
}

/// Mesajın niyeti. Sıcak > fiyat > beklemede > ret önceliğiyle sınıflanır:
/// "fiyatı ne kadar, yaptırmak istiyorum" gibi cümleler fiyat sayılır çünkü
/// operatörün ilk yapacağı iş teklifi göndermektir.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Niyet {
    #[default]
    Bilinmiyor,
    Sicak,
    Fiyat,
    Beklemede,
    Ret,
}

impl Niyet {
    pub const HEPSI: [Niyet; 5] = [
        Niyet::Bilinmiyor,
        Niyet::Sicak,
        Niyet::Fiyat,
        Niyet::Beklemede,
        Niyet::Ret,
    ];

    pub fn etiket(self) -> &'static str {
        match self {
            Niyet::Sicak => "sıcak",
            Niyet::Fiyat => "fiyat soruyor",
            Niyet::Beklemede => "beklemede",
            Niyet::Ret => "ret",
            Niyet::Bilinmiyor => "belirsiz",
        }
    }

    /// Operatörün önce bakması gereken sıra: sıcak ve fiyat en üstte.
    pub fn sira(self) -> u8 {
        match self {
            Niyet::Sicak => 0,
            Niyet::Fiyat => 1,
            Niyet::Beklemede => 2,
            Niyet::Ret => 4,
            Niyet::Bilinmiyor => 3,
        }
    }
}

/// Satış akışı iki duruma indirildi: demo/mesaj gitti ya da satış kapandı.
/// (Arama, "ilgilendi", "teklif verildi" gibi ara adımlar yok; inbound modelde
/// müşteri kendi yazdığı için boru hattı bu iki kutudan oluşur.)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Asama {
    #[default]
    DemoAtildi,
    SatisKapandi,
}

impl Asama {
    pub fn etiket(self) -> &'static str {
        match self {
            Asama::DemoAtildi => "demo atıldı / mesaj gitti",
            Asama::SatisKapandi => "satış kapandı",
        }
    }
}

/// Tek bir mesaj kaydı.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Kayit {
    /// "2026-11-03 14:22" biçiminde yerel zaman.
    pub zaman: String,
    #[serde(default)]
    pub telefon: String,
    #[serde(default)]
    pub ad: String,
    pub metin: String,
    #[serde(default)]
    pub yon: Yon,
    #[serde(default)]
    pub niyet: Niyet,
    #[serde(default)]
    pub asama: Asama,
    #[serde(default)]
    pub okundu: bool,
    /// "köprü" / "elle" / "uygulama".
    #[serde(default)]
    pub kaynak: String,
}

impl Kayit {
    pub fn yeni(telefon: &str, ad: &str, metin: &str, yon: Yon, kaynak: &str) -> Self {
        Self {
            zaman: simdi(),
            telefon: telefon.trim().to_string(),
            ad: ad.trim().to_string(),
            metin: metin.trim().to_string(),
            yon,
            niyet: if yon == Yon::Gelen {
                siniflandir(metin)
            } else {
                Niyet::Bilinmiyor
            },
            asama: Asama::DemoAtildi,
            okundu: yon == Yon::Giden,
            kaynak: kaynak.to_string(),
        }
    }

    /// Başlıkta gösterilecek ad: ad yoksa telefon, o da yoksa "bilinmeyen".
    pub fn baslik(&self) -> String {
        if !self.ad.is_empty() {
            self.ad.clone()
        } else if !self.telefon.is_empty() {
            self.telefon.clone()
        } else {
            "bilinmeyen numara".to_string()
        }
    }

    /// Soğuk mesajı kısa tek satıra indirir (liste görünümü).
    pub fn ozet(&self) -> String {
        let mut satir = String::new();
        for kelime in self.metin.split_whitespace() {
            if satir.chars().count() + kelime.chars().count() > 68 {
                satir.push('…');
                break;
            }
            if !satir.is_empty() {
                satir.push(' ');
            }
            satir.push_str(kelime);
        }
        satir
    }
}

/// Yerel saat damgası: "2026-11-03 14:22".
pub fn simdi() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

/// Türkçe karakterleri sadeleştirip küçük harfe indirir; anahtar sözcük
/// eşleşmesi "yaptırmak" ile "yaptirmak" arasında fark gözetmez.
fn sade(metin: &str) -> String {
    metin
        .chars()
        .map(|harf| match harf {
            'İ' | 'I' | 'ı' | 'i' => 'i',
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'Ö' | 'ö' => 'o',
            'Ş' | 'ş' => 's',
            'Ü' | 'ü' => 'u',
            'Â' | 'â' => 'a',
            diger => diger.to_ascii_lowercase(),
        })
        .collect()
}

/// Ret cümlesinde geçen kalıplar. En önce bakılır: "istemiyorum, fiyat
/// göndermeyin" cümlesi sıcak sayılmamalı.
const RET_KALIPLARI: [&str; 12] = [
    "istemiyorum",
    "ilgilenmiyorum",
    "gerek yok",
    "ihtiyacim yok",
    "vazgec",
    "yapmayacagim",
    "kalsin",
    "engelle",
    "spam",
    "birakin",
    "mesgulum",
    "ret",
];

/// Fiyat sorusu kalıpları.
const FIYAT_KALIPLARI: [&str; 10] = [
    "fiyat", "kac para", "ne kadar", "ucret", "tutar", "odeme", "paket", "teklif", "tl", "butce",
];

/// Sıcak niyet kalıpları: iş yapma isteği.
const SICAK_KALIPLARI: [&str; 12] = [
    "ilgileniyorum",
    "yaptirmak istiyorum",
    "yaptirmak isterim",
    "goruselim",
    "konusalim",
    "arayin",
    "baslayalim",
    "anlastik",
    "istiyorum",
    "olur",
    "tamam",
    "lutfen",
];

/// Beklemede kalıpları.
const BEKLEME_KALIPLARI: [&str; 8] = [
    "dusunecegim",
    "sonra",
    "musait degil",
    "haftaya",
    "bekleyin",
    "acele",
    "yarin",
    "donerim",
];

/// Mesajın niyetini yerelde, anahtar sözcüklerle sınıflar.
///
/// Sıra: ret → fiyat → sıcak → beklemede. "Ne kadar?" hem fiyat hem sıcak
/// sayılabileceği için fiyat önce gelir; teklif göndermek ilk iştir.
pub fn siniflandir(metin: &str) -> Niyet {
    let metin = sade(metin);
    if metin.trim().is_empty() {
        return Niyet::Bilinmiyor;
    }
    let icerir = |liste: &[&str]| liste.iter().any(|kalip| metin.contains(kalip));
    if icerir(&RET_KALIPLARI) {
        return Niyet::Ret;
    }
    if icerir(&FIYAT_KALIPLARI) {
        return Niyet::Fiyat;
    }
    if icerir(&SICAK_KALIPLARI) {
        return Niyet::Sicak;
    }
    if icerir(&BEKLEME_KALIPLARI) {
        return Niyet::Beklemede;
    }
    Niyet::Bilinmiyor
}

/// Dosyaya yazılan gövde.
#[derive(serde::Serialize, serde::Deserialize)]
struct Dosya {
    #[serde(default)]
    surum: u32,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    kayitlar: Vec<Kayit>,
}

/// Kalıcı gelen kutusu.
#[derive(Default)]
pub struct GelenKutusu {
    kayitlar: Vec<Kayit>,
    port: u16,
}

impl GelenKutusu {
    pub fn yukle() -> Self {
        Self::yukle_yoldan(DOSYA)
    }

    pub fn yukle_yoldan(yol: &str) -> Self {
        let metin = std::fs::read_to_string(yol).unwrap_or_default();
        let dosya: Dosya = serde_json::from_str(&metin).unwrap_or(Dosya {
            surum: 1,
            port: 0,
            kayitlar: Vec::new(),
        });
        Self {
            kayitlar: dosya.kayitlar,
            port: if dosya.port == 0 {
                VARSAYILAN_PORT
            } else {
                dosya.port
            },
        }
    }

    pub fn kaydet(&self) {
        self.kaydet_yoldan(DOSYA);
    }

    /// Verilen yola yazar; başarılıysa `true` (onarım ve testler kullanır).
    pub fn kaydet_yoldan(&self, yol: &str) -> bool {
        let dosya = Dosya {
            surum: 1,
            port: self.port,
            kayitlar: self.kayitlar.clone(),
        };
        match serde_json::to_string_pretty(&dosya) {
            Ok(metin) => std::fs::write(yol, metin).is_ok(),
            Err(_) => false,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Köprü portunu günceller (port çakışmasında boş port seçilir).
    pub fn port_ayarla(&mut self, port: u16) {
        self.port = port;
    }

    pub fn kayitlar(&self) -> &[Kayit] {
        &self.kayitlar
    }

    /// Yeni mesajı ekler, eski kayıtları sınırda tutar.
    pub fn ekle(&mut self, kayit: Kayit) {
        self.kayitlar.push(kayit);
        if self.kayitlar.len() > EN_FAZLA_KAYIT {
            let fazla = self.kayitlar.len() - EN_FAZLA_KAYIT;
            self.kayitlar.drain(0..fazla);
        }
    }

    /// Mesaj listesini kişiye göre gruplar: en son yazan üstte.
    /// Dönen liste: (telefon, ad, kayıt indeksleri, son zaman, niyet).
    pub fn kanallar(&self) -> Vec<Kanal> {
        let mut kanallar: Vec<Kanal> = Vec::new();
        for (idx, kayit) in self.kayitlar.iter().enumerate() {
            let anahtar = if kayit.telefon.is_empty() {
                kayit.ad.to_lowercase()
            } else {
                sade(&kayit.telefon)
            };
            match kanallar.iter_mut().find(|kanal| kanal.anahtar == anahtar) {
                Some(kanal) => {
                    kanal.satirlar.push(idx);
                    kanal.ad = kayit.baslik();
                    kanal.son_zaman = kayit.zaman.clone();
                    kanal.okunmamis += usize::from(kayit.yon == Yon::Gelen && !kayit.okundu);
                    // Kanalın niyeti, en son gelen müşteri mesajının niyetidir.
                    if kayit.yon == Yon::Gelen {
                        kanal.niyet = kayit.niyet;
                        kanal.asama = kayit.asama;
                    }
                }
                None => {
                    let okunmamis = usize::from(kayit.yon == Yon::Gelen && !kayit.okundu);
                    kanallar.push(Kanal {
                        anahtar,
                        ad: kayit.baslik(),
                        telefon: kayit.telefon.clone(),
                        satirlar: vec![idx],
                        son_zaman: kayit.zaman.clone(),
                        niyet: if kayit.yon == Yon::Gelen {
                            kayit.niyet
                        } else {
                            Niyet::Bilinmiyor
                        },
                        asama: kayit.asama,
                        okunmamis,
                    });
                }
            }
        }
        kanallar.sort_by(|a, b| {
            b.son_zaman
                .cmp(&a.son_zaman)
                .then_with(|| a.niyet.sira().cmp(&b.niyet.sira()))
        });
        kanallar
    }

    /// Sayılar: (toplam mesaj, okunmamış gelen, sıcak+fiyat kanalı).
    pub fn sayilar(&self) -> (usize, usize, usize) {
        let okunmamis = self
            .kayitlar
            .iter()
            .filter(|kayit| kayit.yon == Yon::Gelen && !kayit.okundu)
            .count();
        let sicak = self
            .kanallar()
            .iter()
            .filter(|kanal| matches!(kanal.niyet, Niyet::Sicak | Niyet::Fiyat))
            .count();
        (self.kayitlar.len(), okunmamis, sicak)
    }

    pub fn okundu_isaretle(&mut self, indeksler: &[usize]) {
        for indeks in indeksler {
            if let Some(kayit) = self.kayitlar.get_mut(*indeks) {
                kayit.okundu = true;
            }
        }
    }

    /// Kanalın tüm mesajlarının aşamasını değiştirir (satış kapandı vb.).
    pub fn asama_ayarla(&mut self, indeksler: &[usize], asama: Asama) {
        for indeks in indeksler {
            if let Some(kayit) = self.kayitlar.get_mut(*indeks) {
                kayit.asama = asama;
            }
        }
    }

    /// Kanalın niyetini elle düzeltir (sınıflandırma yanıldıysa).
    pub fn niyet_ayarla(&mut self, indeksler: &[usize], niyet: Niyet) {
        for indeks in indeksler {
            if let Some(kayit) = self.kayitlar.get_mut(*indeks) {
                if kayit.yon == Yon::Gelen {
                    kayit.niyet = niyet;
                }
            }
        }
    }
}

/// Bir müşteriyle olan yazışma.
pub struct Kanal {
    pub anahtar: String,
    pub ad: String,
    pub telefon: String,
    pub satirlar: Vec<usize>,
    pub son_zaman: String,
    pub niyet: Niyet,
    pub asama: Asama,
    pub okunmamis: usize,
}

// ---------------------------------------------------------------------------
// Yerel köprü (HTTP dinleyici)
// ---------------------------------------------------------------------------

/// Dinleyicinin arayüze gönderdiği olaylar.
pub enum Olay {
    Gelen(Kayit),
    Giden(Kayit),
    Uyari(String),
}

/// Köprü durumu (arayüzde gösterilir).
pub struct KopruDurumu {
    pub calisiyor: bool,
    pub port: u16,
    pub mesaj: String,
}

impl Default for KopruDurumu {
    fn default() -> Self {
        Self {
            calisiyor: false,
            port: VARSAYILAN_PORT,
            mesaj: "köprü henüz başlatılmadı".to_string(),
        }
    }
}

/// Yerel dinleyiciyi arka planda başlatır.
///
/// Dönen değer: (gerçek port, dinleyici açıldı mı, durdurma bayrağı).
/// İstenen port doluysa sıradaki boş port denenir; hiçbiri açılamazsa uyarı
/// kanal üzerinden bildirilir ve uygulama çalışmaya devam eder (elle mesaj
/// yapıştırma yolu her zaman açıktır).
pub fn dinleyici_baslat_esnek(istenen: u16, tx: Sender<Olay>) -> (u16, bool, Arc<AtomicBool>) {
    let durdur = Arc::new(AtomicBool::new(false));
    let bayrak = durdur.clone();
    let (port, listener) = match dinle(istenen) {
        Some(ikili) => ikili,
        None => {
            let _ = tx.send(Olay::Uyari(format!(
                "Köprü 127.0.0.1:{istenen} ve sonraki 9 portu açamadı; yalnız elle yapıştırma çalışır."
            )));
            return (istenen, false, durdur);
        }
    };
    thread::spawn(move || {
        let adres = format!("127.0.0.1:{port}");
        let _ = tx.send(Olay::Uyari(format!(
            "Köprü hazır: http://{adres}/gelen · /gonder · /durum"
        )));
        while !bayrak.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((akis, _)) => match istek_oku(&akis) {
                    Ok(istek) => {
                        let (durum, govde, olay) = istek_isle(&istek);
                        let _ = yanit_yaz(&akis, durum, &govde);
                        if let Some(olay) = olay {
                            let _ = tx.send(olay);
                        }
                    }
                    // Bozuk/aşırı büyük istek: bağlantı kapatılır, uygulama
                    // düşmez; istemciye açık hata kodu döner.
                    Err((durum, mesaj)) => {
                        let _ = yanit_yaz(
                            &akis,
                            durum,
                            &format!("{{\"ok\":false,\"hata\":\"{mesaj}\"}}"),
                        );
                    }
                },
                Err(hata) if hata.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(120));
                }
                Err(_) => thread::sleep(Duration::from_millis(300)),
            }
        }
    });
    (port, true, durdur)
}

/// İstenen porttan başlayarak boş portu bulur (en fazla 10 deneme).
///
/// Bağlama burada, çağıran iş parçacığında yapılır: dönen port gerçekten
/// dinlenen porttur, arayüz yanlış portu gösteremez.
fn dinle(istenen: u16) -> Option<(u16, TcpListener)> {
    for ofset in 0..10u16 {
        let port = istenen.saturating_add(ofset);
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            let _ = listener.set_nonblocking(true);
            return Some((port, listener));
        }
    }
    None
}

/// Okunan HTTP isteği.
struct Istek {
    yontem: String,
    yol: String,
    govde: String,
}

/// Gövde üst sınırı: köprü yalnız kısa JSON mesajları taşır. Daha büyük
/// istekler okunmadan 413 ile reddedilir (bellek şişmesi ve panik yok).
const MAX_GOVDE: usize = 256 * 1024;

fn istek_oku(akis: &TcpStream) -> Result<Istek, (u16, String)> {
    let _ = akis.set_read_timeout(Some(Duration::from_millis(1200)));
    let _ = akis.set_write_timeout(Some(Duration::from_millis(1200)));
    let mut tampon = [0u8; 8192];
    let mut veri = Vec::new();
    // Soket, dinleyiciden non-blocking modu devralır: okuma `WouldBlock`
    // dönerse kısa uykuyla yeniden denenir. Bellek MAX_GOVDE + 1 MB ile
    // sınırlıdır; sınırı aşan gövde boşaltılır ve 413 döner (istemci yanıtı
    // kaybetmesin diye bağlantı hemen kesilmez).
    let bitis = std::time::Instant::now() + Duration::from_millis(1500);
    let mut beklenen = 0usize;
    let mut baslik_okundu = false;
    let mut fazla = false;
    loop {
        if std::time::Instant::now() >= bitis {
            break;
        }
        match (&*akis).read(&mut tampon) {
            Ok(0) => break,
            Ok(okunan) => {
                veri.extend_from_slice(&tampon[..okunan]);
                let metin = String::from_utf8_lossy(&veri).to_string();
                if !baslik_okundu {
                    if let Some(ayrac) = metin.find("\r\n\r\n") {
                        baslik_okundu = true;
                        beklenen = icerik_uzunlugu(&metin[..ayrac]);
                        fazla = beklenen > MAX_GOVDE;
                    }
                }
                if let Some(ayrac) = metin.find("\r\n\r\n") {
                    if metin.len() >= ayrac + 4 + beklenen {
                        break;
                    }
                }
                if veri.len() > MAX_GOVDE + 1024 * 1024 {
                    fazla = true;
                    break;
                }
            }
            Err(hata) if hata.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
    if fazla {
        return Err((413, "gövde çok büyük".to_string()));
    }
    let metin = String::from_utf8_lossy(&veri).to_string();
    if metin.is_empty() {
        return Err((400, "boş istek".to_string()));
    }
    let ilk_satir = metin.lines().next().unwrap_or_default();
    let mut parcalar = ilk_satir.split_whitespace();
    let yontem = parcalar.next().unwrap_or("GET").to_uppercase();
    let yol = parcalar.next().unwrap_or("/").to_string();
    let govde = metin
        .find("\r\n\r\n")
        .map(|ayrac| metin[ayrac + 4..].to_string())
        .unwrap_or_default();
    Ok(Istek { yontem, yol, govde })
}

fn icerik_uzunlugu(basliklar: &str) -> usize {
    for satir in basliklar.lines() {
        let kucuk = satir.to_ascii_lowercase();
        if let Some(deger) = kucuk.strip_prefix("content-length:") {
            return deger.trim().parse().unwrap_or(0);
        }
    }
    0
}

/// İsteği işler: (HTTP durum kodu, yanıt gövdesi, arayüze olay).
fn istek_isle(istek: &Istek) -> (u16, String, Option<Olay>) {
    let yol = istek.yol.split('?').next().unwrap_or("/");
    // Tarayıcı ön kontrolü ve yöntem kontrolü: gövde yorumlanmadan önce
    // yalnız bilinen yöntemler kabul edilir.
    if istek.yontem == "OPTIONS" {
        return (204, String::new(), None);
    }
    if istek.yontem != "GET" && istek.yontem != "POST" {
        return (
            405,
            "{\"ok\":false,\"hata\":\"yöntem desteklenmiyor\"}".to_string(),
            None,
        );
    }
    match yol {
        "/durum" | "/" => (
            200,
            "{\"ok\":true,\"surum\":1,\"uygulama\":\"Nexus Enterprise\"}".to_string(),
            None,
        ),
        "/gelen" => match serde_json::from_str::<serde_json::Value>(&istek.govde) {
            Ok(deger) => {
                let telefon = metin_al(&deger, "telefon");
                let ad = metin_al(&deger, "ad");
                let metin = {
                    let metin = metin_al(&deger, "metin");
                    if metin.is_empty() {
                        metin_al(&deger, "mesaj")
                    } else {
                        metin
                    }
                };
                if metin.is_empty() {
                    return (
                        400,
                        "{\"ok\":false,\"hata\":\"metin boş\"}".to_string(),
                        None,
                    );
                }
                let kayit = Kayit::yeni(&telefon, &ad, &metin, Yon::Gelen, "köprü");
                (
                    200,
                    "{\"ok\":true,\"niyet\":\"".to_string() + kayit.niyet.etiket() + "\"}",
                    Some(Olay::Gelen(kayit)),
                )
            }
            Err(hata) => (
                400,
                format!("{{\"ok\":false,\"hata\":\"json: {hata}\"}}"),
                None,
            ),
        },
        "/gonder" | "/tetik" => match serde_json::from_str::<serde_json::Value>(&istek.govde) {
            Ok(deger) => {
                let telefon = metin_al(&deger, "telefon");
                let ad = metin_al(&deger, "ad");
                let metin = metin_al(&deger, "metin");
                if metin.is_empty() {
                    return (
                        400,
                        "{\"ok\":false,\"hata\":\"metin boş\"}".to_string(),
                        None,
                    );
                }
                let kayit = Kayit::yeni(&telefon, &ad, &metin, Yon::Giden, "köprü");
                (200, "{\"ok\":true}".to_string(), Some(Olay::Giden(kayit)))
            }
            Err(hata) => (
                400,
                format!("{{\"ok\":false,\"hata\":\"json: {hata}\"}}"),
                None,
            ),
        },
        _ => (
            404,
            "{\"ok\":false,\"hata\":\"bilinmeyen yol\"}".to_string(),
            None,
        ),
    }
}

fn metin_al(deger: &serde_json::Value, alan: &str) -> String {
    deger
        .get(alan)
        .and_then(|alan| alan.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn yanit_yaz(akis: &TcpStream, durum: u16, govde: &str) -> Result<(), String> {
    let sebep = match durum {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Not Found",
    };
    // Demo sayfası fetch ile `no-cors` POST atar; CORS başlıkları yine de
    // eklenir ki köprü kendi sayfasından da okuyabilsin.
    let yanit = format!(
        "HTTP/1.1 {durum} {sebep}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nConnection: close\r\n\r\n{govde}",
        govde.len()
    );
    (&*akis)
        .write_all(yanit.as_bytes())
        .map_err(|hata| hata.to_string())
}

/// Köprünün ayakta olup olmadığını sorar (800 ms zaman aşımı).
pub fn kopru_ayakta(port: u16) -> bool {
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(800))
        .build();
    ajan.get(&format!("http://127.0.0.1:{port}/durum"))
        .call()
        .map(|yanit| yanit.status() == 200)
        .unwrap_or(false)
}

/// Köprüye giden mesaj bırakır: `POST /gonder`.
pub fn gonder(port: u16, telefon: &str, ad: &str, metin: &str) -> Result<(), String> {
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(1500))
        .build();
    let govde = serde_json::json!({
        "telefon": telefon,
        "ad": ad,
        "metin": metin,
    });
    ajan.post(&format!("http://127.0.0.1:{port}/gonder"))
        .set("Content-Type", "application/json")
        .send_string(&govde.to_string())
        .map(|_| ())
        .map_err(|hata| hata.to_string())
}

/// Testler ve arayüz için kanal yardımcısı: (kanal listesi, olay alıcısı).
pub fn kanal() -> (Sender<Olay>, Receiver<Olay>) {
    channel()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ret_mesaji_sicak_sayilmaz() {
        assert_eq!(siniflandir("İstemiyorum, fiyat göndermeyin"), Niyet::Ret);
        assert_eq!(siniflandir("Gerek yok teşekkürler"), Niyet::Ret);
    }

    #[test]
    fn fiyat_sorusu_fiyat_olarak_siniflanir() {
        assert_eq!(siniflandir("Bu site ne kadar?"), Niyet::Fiyat);
        assert_eq!(siniflandir("Fiyat listesi var mı"), Niyet::Fiyat);
        assert_eq!(siniflandir("Yaptırmak istiyorum, kaç para?"), Niyet::Fiyat);
    }

    #[test]
    fn sicak_mesaj_ayirt_edilir() {
        assert_eq!(siniflandir("Merhaba ilgileniyorum"), Niyet::Sicak);
        assert_eq!(siniflandir("Yarın görüşelim mi"), Niyet::Sicak);
    }

    #[test]
    fn beklemede_ve_belirsiz_ayrilir() {
        assert_eq!(siniflandir("Düşüneceğim, sonra yazarım"), Niyet::Beklemede);
        assert_eq!(siniflandir("Selam"), Niyet::Bilinmiyor);
        assert_eq!(siniflandir(""), Niyet::Bilinmiyor);
    }

    #[test]
    fn gelen_ve_giden_mesaj_kaydedilir() {
        let mut kutu = GelenKutusu::default();
        kutu.ekle(Kayit::yeni(
            "0555 111 22 33",
            "Ali",
            "Ne kadar?",
            Yon::Gelen,
            "test",
        ));
        kutu.ekle(Kayit::yeni(
            "0555 111 22 33",
            "Ali",
            "Teklif 4500 TL",
            Yon::Giden,
            "test",
        ));
        let kanallar = kutu.kanallar();
        assert_eq!(kanallar.len(), 1);
        assert_eq!(kanallar[0].satirlar.len(), 2);
        assert_eq!(kanallar[0].niyet, Niyet::Fiyat);
        assert_eq!(kanallar[0].okunmamis, 1);
        assert_eq!(kutu.sayilar(), (2, 1, 1));
    }

    #[test]
    fn satis_kapandi_asamasi_kanala_islenir() {
        let mut kutu = GelenKutusu::default();
        kutu.ekle(Kayit::yeni(
            "0555",
            "Ali",
            "Tamam anlaştık",
            Yon::Gelen,
            "test",
        ));
        let satirlar = kutu.kanallar()[0].satirlar.clone();
        kutu.asama_ayarla(&satirlar, Asama::SatisKapandi);
        kutu.okundu_isaretle(&satirlar);
        assert_eq!(kutu.kanallar()[0].asama, Asama::SatisKapandi);
        assert_eq!(kutu.sayilar().1, 0);
    }

    #[test]
    fn json_gidis_donusu_bozulmaz() {
        let mut kutu = GelenKutusu::default();
        kutu.ekle(Kayit::yeni(
            "0555 000 00 00",
            "Ayşe",
            "Fiyat?",
            Yon::Gelen,
            "test",
        ));
        let metin = serde_json::to_string_pretty(&Dosya {
            surum: 1,
            port: 8777,
            kayitlar: kutu.kayitlar().to_vec(),
        })
        .unwrap();
        let geri: Dosya = serde_json::from_str(&metin).unwrap();
        assert_eq!(geri.kayitlar.len(), 1);
        assert_eq!(geri.kayitlar[0].niyet, Niyet::Fiyat);
        assert_eq!(geri.port, 8777);
    }

    #[test]
    fn kopru_istegi_mesaji_ve_niyeti_dondurur() {
        let istek = Istek {
            yontem: "POST".to_string(),
            yol: "/gelen".to_string(),
            govde: "{\"telefon\":\"05551112233\",\"ad\":\"Veli\",\"metin\":\"Ne kadar?\"}"
                .to_string(),
        };
        let (durum, govde, olay) = istek_isle(&istek);
        assert_eq!(durum, 200);
        assert!(govde.contains("fiyat"));
        match olay {
            Some(Olay::Gelen(kayit)) => {
                assert_eq!(kayit.niyet, Niyet::Fiyat);
                assert_eq!(kayit.ad, "Veli");
            }
            _ => panic!("gelen olayı beklenirdi"),
        }
    }

    #[test]
    fn bos_metin_reddedilir_ve_durum_yolu_yanitlar() {
        let bos = Istek {
            yontem: "POST".to_string(),
            yol: "/gelen".to_string(),
            govde: "{\"ad\":\"Veli\"}".to_string(),
        };
        assert_eq!(istek_isle(&bos).0, 400);
        let durum = Istek {
            yontem: "GET".to_string(),
            yol: "/durum".to_string(),
            govde: String::new(),
        };
        let (kod, govde, olay) = istek_isle(&durum);
        assert_eq!(kod, 200);
        assert!(govde.contains("\"ok\":true"));
        assert!(olay.is_none());
    }
    #[test]
    fn dolu_portta_sonraki_bos_port_secillir() {
        // Portu biz tutuyoruz: dinleyici sıradaki boş portu seçmeli.
        let tutulan = TcpListener::bind("127.0.0.1:0").expect("test portu");
        let dolu = tutulan.local_addr().expect("adres").port();
        let (secilen, listener) = dinle(dolu).expect("boş port bulunmalı");
        assert_ne!(secilen, dolu, "dolu port seçilmemeli");
        assert!(
            (dolu + 1..=dolu.saturating_add(9)).contains(&secilen),
            "sıradaki portlar denenmeli: {secilen}"
        );
        drop(listener);
        drop(tutulan);
    }

    #[test]
    fn kopru_gercek_istekleri_isle() {
        // Gerçek TCP dinleyicisi: /durum, /gonder ve /gelen uçtan uca denenir.
        let (tx, rx) = kanal();
        let port = 18700 + (std::process::id() % 200) as u16;
        let (port, acildi, durdur) = dinleyici_baslat_esnek(port, tx);
        let mut hazir = false;
        for _ in 0..60 {
            if kopru_ayakta(port) {
                hazir = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(acildi, "dinleyici açılamadı");
        assert!(hazir, "köprü {port} portunda açılmadı");

        let ajan = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(3))
            .build();
        ajan.post(&format!("http://127.0.0.1:{port}/gelen"))
            .set("Content-Type", "application/json")
            .send_string("{\"telefon\":\"05551112233\",\"ad\":\"Müşteri\",\"metin\":\"Yaptırmak istiyorum, ne kadar?\"}")
            .expect("gelen mesaj kabul edilmeli");
        gonder(port, "05551112233", "Müşteri", "Teklif 4.500 TL")
            .expect("giden mesaj kabul edilmeli");

        let mut gelen_adet = 0usize;
        let mut giden_adet = 0usize;
        for _ in 0..40 {
            while let Ok(olay) = rx.try_recv() {
                match olay {
                    Olay::Gelen(kayit) => {
                        gelen_adet += 1;
                        assert_eq!(kayit.niyet, Niyet::Fiyat);
                    }
                    Olay::Giden(_) => giden_adet += 1,
                    Olay::Uyari(_) => {}
                }
            }
            if gelen_adet > 0 && giden_adet > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        durdur.store(true, Ordering::Relaxed);
        assert_eq!(gelen_adet, 1, "bir gelen mesaj beklenirdi");
        assert_eq!(giden_adet, 1, "bir giden mesaj beklenirdi");
    }
}

#[cfg(test)]
mod guvenlik_testleri {
    use super::*;

    fn istek(yontem: &str, yol: &str, govde: &str) -> Istek {
        Istek {
            yontem: yontem.to_string(),
            yol: yol.to_string(),
            govde: govde.to_string(),
        }
    }

    #[test]
    fn yalniz_bilinen_yontemler_islenir() {
        assert_eq!(istek_isle(&istek("PUT", "/durum", "")).0, 405);
        assert_eq!(istek_isle(&istek("DELETE", "/gelen", "")).0, 405);
        assert_eq!(istek_isle(&istek("OPTIONS", "/gelen", "")).0, 204);
        assert_eq!(istek_isle(&istek("GET", "/durum", "")).0, 200);
        assert_eq!(istek_isle(&istek("GET", "/yok", "")).0, 404);
    }

    #[test]
    fn govde_siniri_ve_icerik_uzunlugu_okunur() {
        assert_eq!(
            icerik_uzunlugu("POST /gelen HTTP/1.1\r\ncontent-length: 42"),
            42
        );
        assert_eq!(icerik_uzunlugu("GET /durum HTTP/1.1"), 0);
        assert!(MAX_GOVDE >= 16 * 1024 && MAX_GOVDE <= 1024 * 1024);
    }

    #[test]
    fn gelen_mesaj_olay_uretir() {
        let (kod, _, olay) = istek_isle(&istek(
            "POST",
            "/gelen",
            "{\"telefon\":\"5551112233\",\"ad\":\"Test\",\"metin\":\"Merhaba\"}",
        ));
        assert_eq!(kod, 200);
        assert!(matches!(olay, Some(Olay::Gelen(_))));
        assert_eq!(
            istek_isle(&istek("POST", "/gelen", "{\"metin\":\"\"}")).0,
            400,
            "boş metin reddedilmeli"
        );
    }
}
