//! Yerel (çevrimdışı) AI fotoğraf üretimi.
//!
//! `yerel_ai/servis.py` uygulama arka planında gizli pencerede çalışır ve
//! 6 GB VRAM'e sığan Stable Diffusion 1.5 fotogerçekçi ince ayarını CUDA'da
//! yükler. Uygulama yalnız `127.0.0.1` üzerinden JSON ile konuşur; üretilen
//! kare doğrudan `media/stock/<sektör>/` altına JPEG olarak yazılır. 2 Medya
//! ekranındaki arşiv taraması (500 ms) dosyayı kendiliğinden yakalar, böylece
//! ayrı bir dosya izleyiciye veya klasör aramaya gerek kalmaz.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crate::sektor_medya;

/// 2 Medya paneline önceden yazılan istem: Türkiye şartlarına uygun çekici.
pub const VARSAYILAN_PROMPT: &str = "Professional photorealistic photo of a European Turkish light commercial flatbed tow truck carrying a white Fiat Egea sedan, on a Turkish highway, blue TR band license plate, daytime, sharp focus, 4k resolution";

/// Türkiye dokusuna uygun hazır istemler: panelin üstünde tek tıkla prompt'a
/// yazılır. Varsayılan metin çekici olduğu için ilk şablon da çekicidir;
/// diğerleri Manisa/Merkez esnafı ve kebapçı gibi sektör karşılıklarıdır.
pub const PROMPT_SABLONLARI: &[(&str, &str)] = &[
    (
        "Manisa/Merkez çekici",
        "Professional photorealistic photo of a European Turkish light commercial flatbed tow truck carrying a white Fiat Egea sedan, parked on a street in Manisa Merkez, blue TR band license plate, Turkish street signs, daytime, sharp focus, 4k resolution",
    ),
    (
        "Manisa kebapçı",
        "Professional photorealistic photo of a traditional Turkish kebab restaurant storefront in Manisa Merkez, charcoal grill with adana kebab and lahmacun, warm evening light, Turkish signage, sharp focus, 4k resolution",
    ),
    (
        "Merkez esnaf dükkânı",
        "Professional photorealistic photo of a small Turkish neighborhood shop in Manisa Merkez, friendly shopkeeper behind the counter, fresh produce crates, Turkish signage on the window, daylight, sharp focus, 4k resolution",
    ),
];

/// İstenmeyen öğeler: SD 1.5'in metin/bozuk tekerlek eğilimini kırar.
pub const NEGATIF_PROMPT: &str = "blurry, low quality, lowres, jpeg artifacts, watermark, text, letters, logo, signature, deformed wheels, extra wheels, duplicate, cropped, cartoon, illustration, painting, oversaturated, tanker, tank truck, fuel truck, cement mixer, box truck, cargo container, graffiti";

/// Servis portu (yalnız loopback).
pub const PORT: u16 = 47621;
/// Servis betiği, proje köküne göre.
pub const BETIK: &str = "yerel_ai/servis.py";
/// 4:3 yatay kare: galeri ve hero alanına uyar, SD 1.5'te bozulmaz.
pub const GENISLIK: u32 = 768;
pub const YUKSEKLIK: u32 = 512;
pub const ADIM: u32 = 28;
pub const REHBER: f32 = 6.5;
/// Model yükleme (ilk indirme dâhil) uzun sürebilir; istek zaman aşımı geniş.
const URETIM_ZAMAN_ASIMI: Duration = Duration::from_secs(600);

/// Servisin durumu; UI rozetini ve düğme etkinliğini belirler.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Durum {
    /// Süreç başlatılmadı.
    Kapali,
    /// Python süreci yeni açıldı, port henüz dinlenmiyor.
    Baslatiliyor,
    /// Süreç ayakta; model ağırlıkları okunuyor.
    Yukleniyor(String),
    /// Model GPU'da hazır, üretim yapılabilir.
    Hazir,
    /// Süreç öldü veya model yüklenemedi.
    Hata(String),
}

impl Durum {
    /// Kısa rozet metni.
    pub fn etiket(&self) -> &str {
        match self {
            Durum::Kapali => "SERVİS KAPALI",
            Durum::Baslatiliyor => "SERVİS BAŞLIYOR",
            Durum::Yukleniyor(_) => "MODEL YÜKLENİYOR",
            Durum::Hazir => "GPU HAZIR",
            Durum::Hata(_) => "SERVİS HATASI",
        }
    }

    /// Üretim isteği gönderilebilir mi?
    pub fn hazir(&self) -> bool {
        matches!(self, Durum::Hazir)
    }
}

/// Python + model servisinin sahibi; üretim isteğini arka planda yürütür.
pub struct YerelAi {
    /// Panele yazılan istem (varsayılan çekici prompt'u ile başlar).
    pub prompt: String,
    pub durum: Durum,
    /// Panele yazılan son durum/açıklama satırı.
    pub mesaj: String,
    /// Üretim sürüyor mu (çift tıklama koruması).
    pub uretiyor: bool,
    /// Üretimin başladığı an: geçen süre göstergesi için.
    pub basladi: Option<Instant>,
    /// En son üretilen dosyanın yolu (arşivde vurgulanır).
    pub son_yol: Option<String>,
    /// Son üretimin saniye cinsinden süresi.
    pub son_sure: Option<f32>,
    /// `/saglik` yanıtından gelen cihaz ve VRAM bilgisi.
    pub altyapi: String,
    cocuk: Option<Child>,
    rx: Option<Receiver<Sonuc>>,
    son_yoklama: Option<Instant>,
    otomatik_denendi: bool,
}

/// Arka plan işinin sonucu: (üretilen dosya, saniye).
type Sonuc = Result<(String, f32), String>;

/// `port` loopback'te dinleniyor mu? Önceki oturumdan kalan servisi
/// sahiplenmeden önce yoklanır.
fn port_mesgul(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(250),
    )
    .is_ok()
}

impl YerelAi {
    pub fn yeni() -> Self {
        Self {
            prompt: VARSAYILAN_PROMPT.to_string(),
            durum: Durum::Kapali,
            mesaj: String::new(),
            uretiyor: false,
            basladi: None,
            son_yol: None,
            son_sure: None,
            altyapi: String::new(),
            cocuk: None,
            rx: None,
            son_yoklama: None,
            otomatik_denendi: false,
        }
    }

    /// Python süreci ayakta mı (UI'daki başlat/yeniden başlat düğmesi için)?
    pub fn calisiyor(&self) -> bool {
        self.cocuk.is_some()
    }

    /// 2 Medya ilk açıldığında servisi bir kez kendiliğinden başlatır.
    /// Kullanıcı kapattıysa yeniden denemez; "Servisi başlat" düğmesi vardır.
    pub fn hazirla(&mut self) {
        if self.cocuk.is_some() || self.otomatik_denendi {
            return;
        }
        self.otomatik_denendi = true;
        self.baslat();
    }

    /// Python servisini gizli pencerede başlatır. Model ağırlıkları ilk
    /// açılışta `yerel_ai/model` altına indirilir; sonrası çevrimdışıdır.
    pub fn baslat(&mut self) {
        if self.cocuk.is_some() {
            return;
        }
        // Önceki oturumdan kalan servis hâlâ 47621'de dinliyorsa onu kapat:
        // aksi hâlde her açılış yeni bir Python süreci (ve ~3,5 GB VRAM)
        // biriktirir, sonunda üretim bellek yetersizliğinden düşer.
        if port_mesgul(PORT) {
            let ajan = ureq::AgentBuilder::new()
                .timeout(Duration::from_millis(700))
                .build();
            let _ = ajan.get(&format!("http://127.0.0.1:{PORT}/kapat")).call();
            let bitis = Instant::now() + Duration::from_secs(6);
            while port_mesgul(PORT) && Instant::now() < bitis {
                std::thread::sleep(Duration::from_millis(150));
            }
            if port_mesgul(PORT) {
                self.durum = Durum::Hata(format!("{PORT} portu meşgul."));
                self.mesaj = format!(
                    "[!] {PORT} portunu başka bir program tutuyor; yerel model başlatılamadı."
                );
                return;
            }
        }
        let Some(betik) = betik_bul() else {
            self.durum = Durum::Hata(format!("{BETIK} bulunamadı."));
            self.mesaj = format!("[!] {BETIK} bulunamadı.");
            return;
        };
        // Python ağırlıkları ve çıktı yollarını betiğin konumundan türetir;
        // çalışma dizini proje kökü olmalı (medya arşivi göreli yazılır).
        let kok = betik
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let model_klasoru = kok.join("yerel_ai").join("model");
        let _ = std::fs::create_dir_all(&model_klasoru);
        let _ = std::fs::create_dir_all(".gecici");
        let gunluk = std::fs::File::create(".gecici/yerel_ai_servis.log").ok();
        let hata_gunluk = gunluk.as_ref().and_then(|dosya| dosya.try_clone().ok());

        let mut komut = Command::new(python_yolu());
        komut
            .arg(&betik)
            .current_dir(&kok)
            .env("YEREL_AI_PORT", PORT.to_string())
            .env("YEREL_AI_NEGATIF", NEGATIF_PROMPT)
            // Ağırlıklar proje içinde kalsın: taşınabilir ve çevrimdışı çalışır.
            .env("HF_HOME", &model_klasoru)
            .env("HF_HUB_DISABLE_SYMLINKS_WARNING", "1")
            .env("PYTHONUNBUFFERED", "1")
            .stdin(Stdio::null())
            .stdout(gunluk.map(Stdio::from).unwrap_or_else(Stdio::null))
            .stderr(hata_gunluk.map(Stdio::from).unwrap_or_else(Stdio::null));
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: konsol penceresi açılmaz.
            komut.creation_flags(0x0800_0000);
        }

        match komut.spawn() {
            Ok(cocuk) => {
                self.cocuk = Some(cocuk);
                self.durum = Durum::Yukleniyor("Model hazırlanıyor…".to_string());
                self.mesaj =
                    "[i] Yerel servis başlatıldı; model GPU'ya yükleniyor (ilk seferde ağırlıklar indirilir)."
                        .to_string();
                self.son_yoklama = None;
            }
            Err(hata) => {
                self.durum = Durum::Hata(format!("Python başlatılamadı: {hata}"));
                self.mesaj = format!(
                    "[!] Python başlatılamadı ({hata}). Python 3 kurulu ve PATH'te olmalı."
                );
            }
        }
    }

    /// Süreci kapatır (önce nazik `/kapat`, sonra zorla).
    pub fn durdur(&mut self) {
        if self.cocuk.is_none() {
            return;
        }
        let ajan = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(400))
            .build();
        let _ = ajan.get(&format!("http://127.0.0.1:{PORT}/kapat")).call();
        if let Some(mut cocuk) = self.cocuk.take() {
            let _ = cocuk.kill();
            let _ = cocuk.wait();
        }
        self.durum = Durum::Kapali;
        self.rx = None;
        self.uretiyor = false;
    }

    /// Servisi kapatıp yeniden başlatır (takılan model yüklemesi için).
    pub fn yeniden_baslat(&mut self) {
        self.durdur();
        self.otomatik_denendi = true;
        self.baslat();
    }

    /// `/saglik` yoklaması: en fazla ~1,2 sn'de bir, üretim sürerken hiç.
    pub fn yokla(&mut self) {
        // Süreç kendiliğinden öldüyse sebebi günlükte; durumu hataya çek.
        if let Some(cocuk) = self.cocuk.as_mut() {
            if let Ok(Some(kod)) = cocuk.try_wait() {
                self.cocuk = None;
                self.uretiyor = false;
                self.durum = Durum::Hata(format!("Servis kapandı (çıkış kodu {kod})."));
                self.mesaj = format!(
                    "[!] Yerel servis beklenmedik şekilde kapandı. Günlük: .gecici/yerel_ai_servis.log"
                );
                return;
            }
        }
        if self.cocuk.is_none() {
            return;
        }
        if self
            .son_yoklama
            .is_some_and(|son| son.elapsed() < Duration::from_millis(1200))
        {
            return;
        }
        self.son_yoklama = Some(Instant::now());
        let ajan = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(900))
            .build();
        let Ok(yanit) = ajan.get(&format!("http://127.0.0.1:{PORT}/saglik")).call() else {
            // Henüz dinlemiyor: Python tarafı torch'u içe aktarıyor olabilir.
            if self.durum == Durum::Kapali {
                self.durum = Durum::Baslatiliyor;
            }
            return;
        };
        if let Ok(metin) = yanit.into_string() {
            self.durum = saglik_coz(&metin);
            self.altyapi = altyapi_coz(&metin);
            if let Durum::Hata(neden) = &self.durum {
                self.mesaj = format!("[!] {neden}");
            }
        }
    }

    /// Paneldeki prompt ile tek kare üretir; çıktı sektör klasörüne yazılır.
    pub fn uret(&mut self, slug: &str) -> bool {
        if self.uretiyor {
            return false;
        }
        if !self.durum.hazir() {
            self.mesaj = "[!] Yerel model henüz hazır değil.".to_string();
            return false;
        }
        let istem = self.prompt.trim().to_string();
        if istem.is_empty() {
            self.mesaj = "[!] Prompt boş olamaz.".to_string();
            return false;
        }
        let dizin = Path::new(sektor_medya::KLASOR).join(slug);
        if let Err(hata) = std::fs::create_dir_all(&dizin) {
            self.mesaj = format!("[!] Sektör klasörü açılamadı: {hata}");
            return false;
        }
        let zaman = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let hedef = cikti_yolu(&dizin, &zaman);
        let govde = istek_govdesi(&istem, &hedef);
        let ad = hedef
            .file_name()
            .map(|ad| ad.to_string_lossy().to_string())
            .unwrap_or_else(|| "foto.jpg".to_string());
        self.mesaj = format!("[i] {ad} üretiliyor… (ilk kare 20–40 sn sürebilir)");
        self.basladi = Some(Instant::now());
        self.uretiyor = true;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let bas = Instant::now();
            let sonuc = istek_gonder(&govde, &hedef);
            let _ = tx.send(sonuc.map(|yol| {
                let sure = bas.elapsed().as_secs_f32();
                (yol, sure)
            }));
        });
        true
    }

    /// Arka plan üretiminin sonucunu bir kez okur; yeni dosya yolunu döndürür.
    pub fn isle(&mut self) -> Option<String> {
        let rx = self.rx.take()?;
        match rx.try_recv() {
            Ok(Ok((yol, sure))) => {
                self.uretiyor = false;
                self.basladi = None;
                self.son_sure = Some(sure);
                self.son_yol = Some(yol.clone());
                let ad = yol.rsplit('/').next().unwrap_or(&yol).to_string();
                self.mesaj = format!("[✓] {ad} arşive eklendi ({sure:.1} sn).");
                Some(yol)
            }
            Ok(Err(hata)) => {
                self.uretiyor = false;
                self.basladi = None;
                self.mesaj = format!("[!] {hata}");
                None
            }
            Err(TryRecvError::Empty) => {
                self.rx = Some(rx);
                None
            }
            Err(TryRecvError::Disconnected) => {
                self.uretiyor = false;
                self.basladi = None;
                self.mesaj = "[!] Üretim işi beklenmedik şekilde kesildi.".to_string();
                None
            }
        }
    }

    /// Durum satırının altında gösterilecek hedef klasör ipucu.
    pub fn hedef_ipucu(slug: &str, durum: &Durum) -> String {
        match durum {
            Durum::Hazir => format!("Kayıt: media/stock/{slug}/ai_<zaman>.jpg · yerel GPU"),
            Durum::Yukleniyor(_) => {
                "Model GPU'ya yükleniyor; hazır olunca düğme etkinleşir.".to_string()
            }
            Durum::Baslatiliyor => "Python servisi açılıyor…".to_string(),
            Durum::Kapali => "Servis açık değil.".to_string(),
            Durum::Hata(neden) => neden.clone(),
        }
    }
}

impl Drop for YerelAi {
    fn drop(&mut self) {
        self.durdur();
    }
}

/// Servis betiğini bulur: önce çalışma dizini (proje kökü), sonra `.exe`'nin
/// bulunduğu klasör ve üst dizinleri. Böylece uygulama başka bir dizinden
/// başlatılsa da betik bulunur.
fn betik_bul() -> Option<PathBuf> {
    // Mutlak yol şart: betiğin üst dizinleri çalışma dizini olarak veriliyor,
    // göreli yolda bu boş kalır ve süreç açılamaz.
    let mut adaylar: Vec<PathBuf> = std::env::current_dir()
        .ok()
        .map(|kok| vec![kok.join(BETIK)])
        .unwrap_or_default();
    if let Ok(exe) = std::env::current_exe() {
        let mut dizin = exe.parent().map(Path::to_path_buf);
        for _ in 0..4 {
            let Some(simdiki) = dizin else { break };
            adaylar.push(simdiki.join(BETIK));
            dizin = simdiki.parent().map(Path::to_path_buf);
        }
    }
    adaylar.into_iter().find(|yol| yol.is_file())
}

/// `YEREL_AI_PYTHON` verilmişse onu, yoksa PATH'teki `python`'u kullanır.
fn python_yolu() -> String {
    std::env::var("YEREL_AI_PYTHON")
        .ok()
        .map(|yol| yol.trim().to_string())
        .filter(|yol| !yol.is_empty())
        .unwrap_or_else(|| "python".to_string())
}

/// Çıktı adı: `ai_<zaman>.jpg`; aynı saniyede ikinci üretimde sıra eklenir.
pub fn cikti_yolu(dizin: &Path, zaman: &str) -> PathBuf {
    let ilk = dizin.join(format!("ai_{zaman}.jpg"));
    if !ilk.exists() {
        return ilk;
    }
    for sira in 2..1000 {
        let aday = dizin.join(format!("ai_{zaman}_{sira}.jpg"));
        if !aday.exists() {
            return aday;
        }
    }
    dizin.join(format!("ai_{zaman}_{}.jpg", 999))
}

/// Servise gönderilen JSON gövdesi (ölçü/adım/rehber sabit, tohum rastgele).
pub fn istek_govdesi(prompt: &str, cikti: &Path) -> String {
    serde_json::json!({
        "prompt": prompt,
        "negatif": NEGATIF_PROMPT,
        "genislik": GENISLIK,
        "yukseklik": YUKSEKLIK,
        "adim": ADIM,
        "rehber": REHBER,
        "tohum": -1,
        "cikti": cikti.to_string_lossy().replace('\\', "/"),
    })
    .to_string()
}

/// `/saglik` yanıtını duruma çevirir.
pub fn saglik_coz(metin: &str) -> Durum {
    let Ok(deger) = serde_json::from_str::<serde_json::Value>(metin) else {
        return Durum::Baslatiliyor;
    };
    let mesaj = deger
        .get("mesaj")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    match deger
        .get("durum")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
    {
        "hazir" => Durum::Hazir,
        "hata" => Durum::Hata(if mesaj.is_empty() {
            "Model yüklenemedi.".to_string()
        } else {
            mesaj
        }),
        _ => Durum::Yukleniyor(if mesaj.is_empty() {
            "Model hazırlanıyor…".to_string()
        } else {
            mesaj
        }),
    }
}

/// Cihaz ve VRAM bilgisini tek satıra çevirir ("cuda · 5 812 MB boş").
fn altyapi_coz(metin: &str) -> String {
    let Ok(deger) = serde_json::from_str::<serde_json::Value>(metin) else {
        return String::new();
    };
    let cihaz = deger
        .get("cihaz")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_uppercase();
    let vram = deger
        .get("vram")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if vram.is_empty() {
        cihaz
    } else {
        format!("{cihaz} · {vram}")
    }
}

/// Rust tarafı çıktı denetimi: dosya var mı, boş mu, gerçek bir JPEG/PNG mi
/// ve görsel olarak açılabiliyor mu? Python servisine güvenmek yerine kare
/// arşive girmeden önce burada doğrulanır.
fn cikti_dogrula(yol: &Path) -> Result<(), String> {
    let meta =
        std::fs::metadata(yol).map_err(|hata| format!("Üretilen dosya okunamadı ({hata})."))?;
    if !meta.is_file() {
        return Err("Üretilen yol bir dosya değil.".to_string());
    }
    if meta.len() < 1024 {
        return Err(format!(
            "Üretilen dosya boş ya da yarım ({} bayt).",
            meta.len()
        ));
    }
    let mut imza = [0u8; 3];
    {
        use std::io::Read;
        let mut dosya = std::fs::File::open(yol).map_err(|hata| hata.to_string())?;
        dosya
            .read_exact(&mut imza)
            .map_err(|hata| format!("Dosya başı okunamadı: {hata}"))?;
    }
    let jpeg = imza == [0xFF, 0xD8, 0xFF];
    let png = imza == [0x89, 0x50, 0x4E];
    if !jpeg && !png {
        return Err("Üretilen dosya JPEG/PNG imzası taşımıyor.".to_string());
    }
    image::image_dimensions(yol).map_err(|hata| format!("Görsel açılamadı: {hata}"))?;
    Ok(())
}

/// HTTP POST: üretilen dosyanın yolunu döndürür.
fn istek_gonder(govde: &str, hedef: &Path) -> Result<String, String> {
    let ajan = ureq::AgentBuilder::new()
        .timeout(URETIM_ZAMAN_ASIMI)
        .build();
    let yanit = ajan
        .post(&format!("http://127.0.0.1:{PORT}/uret"))
        .set("Content-Type", "application/json; charset=utf-8")
        .send_string(govde)
        .map_err(|hata| format!("Yerel servis yanıt vermedi: {hata}"))?;
    let metin = yanit
        .into_string()
        .map_err(|hata| format!("Servis yanıtı okunamadı: {hata}"))?;
    let deger: serde_json::Value =
        serde_json::from_str(&metin).map_err(|_| "Servis yanıtı anlaşılmadı.".to_string())?;
    if deger.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        let yol = deger
            .get("yol")
            .and_then(|v| v.as_str())
            .filter(|yol| !yol.trim().is_empty())
            .map(|yol| yol.replace('\\', "/"))
            .unwrap_or_else(|| hedef.to_string_lossy().replace('\\', "/"));
        // Servis "ok" dese de dosya gerçekten diskte ve geçerli bir görsel
        // olmalı: yarım yazılmış kare arşive/demoya girmesin.
        cikti_dogrula(Path::new(&yol))?;
        Ok(yol)
    } else {
        Err(deger
            .get("hata")
            .and_then(|v| v.as_str())
            .unwrap_or("Üretim başarısız.")
            .to_string())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_prompt_cekici_istemidir() {
        assert_eq!(
            VARSAYILAN_PROMPT,
            "Professional photorealistic photo of a European Turkish light commercial flatbed tow truck carrying a white Fiat Egea sedan, on a Turkish highway, blue TR band license plate, daytime, sharp focus, 4k resolution"
        );
        assert!(YerelAi::yeni().prompt.contains("flatbed tow truck"));
        assert!(VARSAYILAN_PROMPT.contains("TR band license plate"));
    }

    #[test]
    fn cikti_yolu_cakismadan_sektor_klasorune_yazar() {
        let kok = std::env::temp_dir().join(format!("nexus_ai_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(&kok).unwrap();
        let ilk = cikti_yolu(&kok, "20261106_2130");
        assert_eq!(ilk.file_name().unwrap(), "ai_20261106_2130.jpg");
        std::fs::write(&ilk, [1u8; 4]).unwrap();
        let ikinci = cikti_yolu(&kok, "20261106_2130");
        assert_ne!(ilk, ikinci);
        assert!(ikinci.to_string_lossy().ends_with("_2.jpg"));
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn istek_govdesi_prompt_ve_hedefi_tasir() {
        let govde = istek_govdesi(VARSAYILAN_PROMPT, Path::new("media/stock/cekici/ai_1.jpg"));
        let deger: serde_json::Value = serde_json::from_str(&govde).unwrap();
        assert_eq!(deger["prompt"], VARSAYILAN_PROMPT);
        assert_eq!(deger["negatif"], NEGATIF_PROMPT);
        assert_eq!(deger["cikti"], "media/stock/cekici/ai_1.jpg");
        assert_eq!(deger["genislik"], GENISLIK);
        assert_eq!(deger["adim"], ADIM);
    }

    #[test]
    fn saglik_yaniti_duruma_cevrilir() {
        assert_eq!(
            saglik_coz(r#"{"durum":"hazir","mesaj":"tamam"}"#),
            Durum::Hazir
        );
        assert_eq!(
            saglik_coz(r#"{"durum":"yukleniyor","mesaj":"Ağırlıklar okunuyor…"}"#),
            Durum::Yukleniyor("Ağırlıklar okunuyor…".to_string())
        );
        assert!(matches!(
            saglik_coz(r#"{"durum":"hata","mesaj":"CUDA yok"}"#),
            Durum::Hata(neden) if neden == "CUDA yok"
        ));
        assert!(matches!(saglik_coz("bozuk"), Durum::Baslatiliyor));
    }

    #[test]
    fn altyapi_satiri_cihaz_ve_vram_gosterir() {
        assert_eq!(
            altyapi_coz(r#"{"cihaz":"cuda","vram":"5812 / 6144 MB boş"}"#),
            "CUDA · 5812 / 6144 MB boş"
        );
        assert_eq!(altyapi_coz(r#"{"cihaz":"cpu"}"#), "CPU");
    }

    #[test]
    fn bos_prompt_uretim_baslatmaz() {
        let mut ai = YerelAi::yeni();
        ai.durum = Durum::Hazir;
        ai.prompt.clear();
        assert!(!ai.uret("cekici"));
        assert!(!ai.uretiyor);
        assert!(ai.mesaj.starts_with("[!]"));
    }

    #[test]
    fn hazir_olmayan_servis_uretim_baslatmaz() {
        let mut ai = YerelAi::yeni();
        assert!(!ai.uret("cekici"));
        assert!(!ai.uretiyor);
    }

    /// Gerçek süreç: Python servisini başlatır, modelin GPU'da hazır olmasını
    /// bekler ve kapatır. Port boşken `cargo test -- --ignored` ile çalışır.
    #[test]
    #[ignore]
    fn gercek_servis_baslatilir_ve_hazir_olur() {
        let mut ai = YerelAi::yeni();
        ai.baslat();
        assert!(ai.calisiyor(), "Python süreci açılmadı: {}", ai.mesaj);
        let bas = Instant::now();
        while bas.elapsed() < Duration::from_secs(180) {
            ai.yokla();
            if ai.durum.hazir() {
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        assert!(ai.durum.hazir(), "servis hazır olmadı: {:?}", ai.durum);
        assert!(
            ai.altyapi.contains("CUDA"),
            "GPU bilgisi yok: {}",
            ai.altyapi
        );
        ai.durdur();
        assert!(!ai.calisiyor());
    }

    /// Gerçek servis gerektirir: `YEREL_AI_PORT` açıkken çalıştırılır.
    /// `cargo test -- --ignored gercek_servis` ile elle doğrulanır.
    #[test]
    #[ignore]
    fn gercek_servis_uretim_yolunu_doner() {
        let dizin = Path::new(sektor_medya::KLASOR).join("cekici");
        std::fs::create_dir_all(&dizin).unwrap();
        let hedef = cikti_yolu(&dizin, "test");
        let govde = istek_govdesi(VARSAYILAN_PROMPT, &hedef);
        let yol = istek_gonder(&govde, &hedef).expect("yerel servis üretmeli");
        let dosya = Path::new(&yol);
        assert!(dosya.is_file(), "üretilen dosya yok: {yol}");
        assert!(
            std::fs::metadata(dosya).unwrap().len() > 20_000,
            "küçük/bozuk kare"
        );
        let _ = std::fs::remove_file(dosya);
    }
}

#[cfg(test)]
mod sablon_testleri {
    use super::*;

    #[test]
    fn prompt_sablonlari_turkiye_dokusunu_kapsar() {
        assert!(PROMPT_SABLONLARI.len() >= 3, "en az üç hazır şablon olmalı");
        let metinler: Vec<&str> = PROMPT_SABLONLARI.iter().map(|(_, metin)| *metin).collect();
        assert!(
            metinler
                .iter()
                .any(|metin| metin.contains("tow truck") && metin.contains("TR band license plate")),
            "Türk plakalı çekici şablonu bulunmalı"
        );
        assert!(
            metinler.iter().any(|metin| metin.contains("kebab")),
            "kebapçı şablonu bulunmalı"
        );
        assert!(
            metinler.iter().all(|metin| metin.contains("Manisa Merkez")),
            "şablonlar Manisa/Merkez dokusunu taşımalı"
        );
        assert!(PROMPT_SABLONLARI
            .iter()
            .all(|(ad, _)| !ad.trim().is_empty()));
    }
}

#[cfg(test)]
mod port_testleri {
    use super::*;

    #[test]
    fn dinleyen_port_mesgul_gorunur() {
        let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").expect("test portu");
        let port = dinleyici.local_addr().expect("adres").port();
        assert!(port_mesgul(port), "dinleyen port meşgul sayılmalı");
        drop(dinleyici);
    }
}

#[cfg(test)]
mod cikti_testleri {
    use super::*;

    #[test]
    fn bozuk_ya_da_yarim_cikti_reddedilir() {
        let dizin = std::env::temp_dir().join(format!("nexus_cikti_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dizin);
        std::fs::create_dir_all(&dizin).unwrap();

        let yok = dizin.join("yok.jpg");
        assert!(cikti_dogrula(&yok).is_err(), "olmayan dosya reddedilmeli");

        let kucuk = dizin.join("kucuk.jpg");
        std::fs::write(&kucuk, [0xFFu8, 0xD8, 0xFF, 0x00]).unwrap();
        assert!(
            cikti_dogrula(&kucuk).is_err(),
            "1 KB altı dosya reddedilmeli"
        );

        let imzasiz = dizin.join("imzasiz.jpg");
        std::fs::write(&imzasiz, vec![7u8; 4096]).unwrap();
        assert!(
            cikti_dogrula(&imzasiz).is_err(),
            "imzasız dosya reddedilmeli"
        );

        let gercek = dizin.join("gercek.jpg");
        let mut resim = image::RgbImage::new(64, 64);
        for (x, y, piksel) in resim.enumerate_pixels_mut() {
            *piksel = image::Rgb([
                (x * 7 % 256) as u8,
                (y * 11 % 256) as u8,
                ((x + y) * 13 % 256) as u8,
            ]);
        }
        resim.save(&gercek).unwrap();
        assert!(
            cikti_dogrula(&gercek).is_ok(),
            "geçerli JPEG kabul edilmeli"
        );

        let _ = std::fs::remove_dir_all(&dizin);
    }
}
