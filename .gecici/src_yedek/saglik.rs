//! Açılış denetimi ve otomatik onarım (self-healing).
//!
//! Ana ekran çizilmeden önce dört kritik bileşen sınanır: kalıcı veri
//! dosyaları, WhatsApp köprüsü, yapay zeka servisi ve harita bağlantısı.
//! Maddelerin yanında teknik hata kodu değil, kullanıcının ne yapacağını
//! anladığı sade bir Türkçe açıklama durur.
//!
//! Denetimler ve onarım arka plan iş parçacığında koşar; pencere donmaz.
//! Onarım kendi çözebildiği sorunları (bozuk/eksik JSON, salt okunur dosya,
//! bayat önbellek, kapalı Ollama) kendisi giderir. Denetim kullanıcı
//! tarafından durdurulabilir.

use eframe::egui;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;

use crate::{ai, gelen, hedef, musteriler};

/// Maddelerin sırası ve başlıkları.
const ADLAR: [&str; 4] = [
    "Kalıcı veri dosyaları",
    "WhatsApp köprüsü (8777)",
    "Yapay zeka servisi (11434)",
    "Google API ve harita",
];

/// Denetlenen kalıcı dosyalar (varlık + yazma izni + JSON bütünlüğü).
const DOSYALAR: [&str; 3] = [musteriler::DEFTER_DOSYASI, hedef::DOSYA, gelen::DOSYA];

/// Onarımda temizlenen geçici önbellekler (kalıcı veri değildir).
const ONBELLEKLER: [&str; 2] = [crate::KONUM_ONBELLEK_DOSYASI, crate::SONUC_ONBELLEK_DOSYASI];

/// Yazma izni yoklamasında kullanılan geçici dosya adı.
const YAZMA_TESTI: &str = ".nexus_yazma_testi";

/// Veri dosyası eksik/bozuksa gösterilen açıklama.
pub const VERI_HATA: &str =
    "Başarısız! Müşteri defteri veya hedef listesi dosyası eksik ya da bozulmuş.";
/// Dosyalara yazılamıyorsa gösterilen açıklama.
pub const YAZMA_HATA: &str =
    "Başarısız! Veri dosyalarına yazma izni yok; defter ve hedef listesi kaydedilemiyor.";
/// Köprü portu başkası tarafından tutuluyorsa gösterilen açıklama.
pub const KOPRU_HATA: &str =
    "Başarısız! WhatsApp mesajlaşma köprüsü (8777 portu) başka bir program tarafından kullanılıyor.";
/// Yerel model sunucusu kapalıysa gösterilen açıklama.
pub const OLLAMA_HATA: &str = "Başarısız! Yapay zeka servisi (Ollama) arka planda çalışmıyor. Lütfen Ollama uygulamasını başlatın veya modelin yüklü olduğundan emin olun.";
/// Sunucu açık ama model yoksa gösterilen açıklama.
pub const MODEL_HATA: &str = "Başarısız! Yapay zeka servisi açık ama kullanılabilir model yok. Terminalde 'ollama pull llama3' komutunu çalıştırın.";
/// Harita servisine ulaşılamazsa gösterilen açıklama.
pub const HARITA_HATA: &str =
    "Başarısız! İnternet bağlantısı yok veya Google API uç noktalarına erişilemiyor.";

/// Tek denetimin durumu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Durum {
    Bekliyor,
    Calisiyor,
    Gecti,
    Kaldi,
}

impl Durum {
    /// Maddelerin yanındaki işaret.
    pub fn isaret(self) -> &'static str {
        match self {
            Durum::Bekliyor => "•",
            Durum::Calisiyor => "…",
            Durum::Gecti => "✓",
            Durum::Kaldi => "✕",
        }
    }

    /// Satır rengi (renk körlüğü için işaret ayrıca durur).
    pub fn renk(self) -> egui::Color32 {
        match self {
            Durum::Bekliyor => egui::Color32::from_rgb(150, 158, 152),
            Durum::Calisiyor => egui::Color32::from_rgb(176, 132, 32),
            Durum::Gecti => egui::Color32::from_rgb(32, 122, 74),
            Durum::Kaldi => egui::Color32::from_rgb(176, 48, 44),
        }
    }
}

/// Kullanıcıya gösterilen denetim maddesi.
#[derive(Clone, Debug)]
pub struct Madde {
    pub ad: &'static str,
    pub durum: Durum,
    pub aciklama: String,
}

/// Arka plandan arayüze giden tek haber.
pub enum Haber {
    Basladi(usize),
    Bitti(usize, Result<String, String>),
}

/// Denetim listesinin tamamı; arayüz bu yapıyı çizer.
pub struct Denetim {
    pub maddeler: Vec<Madde>,
    rx: Option<Receiver<Haber>>,
    pub bitti: bool,
    pub basarili: bool,
    pub durduruldu: bool,
    iptal: Option<Arc<AtomicBool>>,
    onarim_rx: Option<Receiver<Vec<String>>>,
    /// Son onarımın yaptığı işler (arayüzde listelenir).
    pub onarim: Option<Vec<String>>,
}

impl Default for Denetim {
    fn default() -> Self {
        Self {
            maddeler: maddeler(),
            rx: None,
            bitti: false,
            basarili: false,
            durduruldu: false,
            iptal: None,
            onarim_rx: None,
            onarim: None,
        }
    }
}

/// Dört maddeyi "sırada" durumunda üretir.
fn maddeler() -> Vec<Madde> {
    ADLAR
        .iter()
        .map(|ad| Madde {
            ad,
            durum: Durum::Bekliyor,
            aciklama: "sırada…".to_string(),
        })
        .collect()
}

impl Denetim {
    /// Denetimleri arka planda başlatır (yalnız ilk çağrıda iş parçacığı açar).
    pub fn basla(&mut self) {
        if self.rx.is_some() {
            return;
        }
        let iptal = Arc::new(AtomicBool::new(false));
        let bayrak = iptal.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let kontroller: [fn() -> Result<String, String>; 4] =
                [veri_dosyalari, kopru, ollama, harita];
            for (sira, kontrol) in kontroller.into_iter().enumerate() {
                if bayrak.load(Ordering::Relaxed) {
                    break;
                }
                let _ = tx.send(Haber::Basladi(sira));
                let sonuc = kontrol();
                if bayrak.load(Ordering::Relaxed) {
                    break;
                }
                let _ = tx.send(Haber::Bitti(sira, sonuc));
            }
        });
        self.iptal = Some(iptal);
        self.rx = Some(rx);
    }

    /// Arka plandan gelen haberleri maddelere işler.
    pub fn isle(&mut self) {
        let Some(rx) = &self.rx else {
            return;
        };
        while let Ok(haber) = rx.try_recv() {
            match haber {
                Haber::Basladi(sira) => {
                    if let Some(madde) = self.maddeler.get_mut(sira) {
                        madde.durum = Durum::Calisiyor;
                        madde.aciklama = "deneniyor…".to_string();
                    }
                }
                Haber::Bitti(sira, sonuc) => {
                    if let Some(madde) = self.maddeler.get_mut(sira) {
                        match sonuc {
                            Ok(not) => {
                                madde.durum = Durum::Gecti;
                                madde.aciklama = not;
                            }
                            Err(aciklama) => {
                                madde.durum = Durum::Kaldi;
                                madde.aciklama = aciklama;
                            }
                        }
                    }
                }
            }
        }
        let hepsi_kararli = self
            .maddeler
            .iter()
            .all(|madde| matches!(madde.durum, Durum::Gecti | Durum::Kaldi));
        if hepsi_kararli && !self.bitti && !self.durduruldu {
            self.bitti = true;
            self.basarili = self
                .maddeler
                .iter()
                .all(|madde| madde.durum == Durum::Gecti);
        }
    }

    /// Ana ekran açılabilir mi? (Yalnız tüm maddeler yeşilse.)
    pub fn gecti(&self) -> bool {
        self.bitti && self.basarili
    }

    /// Kullanıcı "Durdur" derse: bekleyen maddeler denetlenmeden kalır,
    /// panel açık kalır (uygulama yine açılmaz).
    pub fn durdur(&mut self) {
        if let Some(bayrak) = &self.iptal {
            bayrak.store(true, Ordering::Relaxed);
        }
        if self.bitti {
            return;
        }
        self.bitti = true;
        self.basarili = false;
        self.durduruldu = true;
        for madde in &mut self.maddeler {
            if matches!(madde.durum, Durum::Bekliyor | Durum::Calisiyor) {
                madde.durum = Durum::Bekliyor;
                madde.aciklama = "denetlenmedi · durduruldu".to_string();
            }
        }
    }

    /// Sayaçları ve maddeleri başlangıç durumuna alır (iş parçacığı açmaz).
    pub fn sifirla(&mut self) {
        self.maddeler = maddeler();
        self.rx = None;
        self.iptal = None;
        self.bitti = false;
        self.basarili = false;
        self.durduruldu = false;
        self.onarim = None;
    }

    /// "Yeniden Dene": denetimi baştan, kapanmadan çalıştırır.
    pub fn yeniden_basla(&mut self) {
        if self.onarim_suruyor() {
            return;
        }
        self.sifirla();
        self.basla();
    }

    /// "Otomatik Düzelt": onarımı arka planda başlatır.
    pub fn onar_baslat(&mut self) {
        if self.onarim_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(onar());
        });
        self.onarim_rx = Some(rx);
    }

    /// Onarım bitti mi? Sonucu `onarim` alanına yazar.
    pub fn onarim_isle(&mut self) {
        let Some(rx) = &self.onarim_rx else {
            return;
        };
        if let Ok(liste) = rx.try_recv() {
            self.onarim = Some(liste);
            self.onarim_rx = None;
        }
    }

    pub fn onarim_suruyor(&self) -> bool {
        self.onarim_rx.is_some()
    }
}

// ---------------------------------------------------------------------------
// Denetimler
// ---------------------------------------------------------------------------

/// Dosyalar var mı, yazılabiliyor mu, JSON geçerli mi?
fn veri_dosyalari() -> Result<String, String> {
    for ad in DOSYALAR {
        if !json_gecerli_yoldan(Path::new(ad)) {
            return Err(format!(
                "{VERI_HATA} ({ad} dosyasını Otomatik Düzelt düğmesi varsayılan şablonla yeniden oluşturabilir.)"
            ));
        }
        if !yazilabilir(Path::new(ad)) {
            return Err(format!("{YAZMA_HATA} ({ad} dosyası salt okunur olabilir.)"));
        }
    }
    Ok("Üç veri dosyası okunabilir, yazılabilir ve JSON bütünlüğü sağlam.".to_string())
}

/// Köprü ya dinliyor ya da port tamamen boş olmalı.
fn kopru() -> Result<String, String> {
    let port = gelen::VARSAYILAN_PORT;
    if gelen::kopru_ayakta(port) {
        return Ok(format!(
            "Köprü {port} portunda dinliyor; demo sayfası mesaj gönderebilir."
        ));
    }
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(dinleyici) => {
            drop(dinleyici);
            Ok(format!("{port} portu boş; köprü açılışta başlatılacak."))
        }
        Err(_) => Err(KOPRU_HATA.to_string()),
    }
}

/// Yerel model: kapalıysa bir kez başlatılır, birkaç saniye beklenir.
fn ollama() -> Result<String, String> {
    let mut durum = ai::otomatik_baslat();
    let mut deneme = 0;
    while !durum.aktif && deneme < 10 {
        std::thread::sleep(Duration::from_millis(700));
        durum = ai::yokla();
        deneme += 1;
    }
    if durum.aktif {
        return Ok(format!(
            "Sunucu 11434 portunda hazır · seçili model: {}",
            durum.model
        ));
    }
    // Sunucu ayakta ama model listesi boşsa ayrı, daha yol gösterici mesaj.
    if sunucu_cevap_veriyor() {
        return Err(MODEL_HATA.to_string());
    }
    Err(OLLAMA_HATA.to_string())
}

/// 11434 portuna kısa bir TCP dokunuşu: sunucu ayakta mı?
fn sunucu_cevap_veriyor() -> bool {
    let Ok(soket) = "127.0.0.1:11434".parse::<std::net::SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&soket, Duration::from_secs(1)).is_ok()
}

/// Google Maps Platform erişimi: Places ve Geocoding uç noktaları.
///
/// Anahtar burada kullanılmaz; yalnızca ağ erişimi sınanır. Anahtarsız
/// kurulumda da denetim geçer, tarama başlarken panel yönlendirir.
fn harita() -> Result<String, String> {
    let erisim = |adres: &str| -> bool {
        let Ok(mut adresler) = adres.to_socket_addrs() else {
            return false;
        };
        let Some(soket) = adresler.next() else {
            return false;
        };
        TcpStream::connect_timeout(&soket, Duration::from_secs(3)).is_ok()
    };
    if !(erisim("places.googleapis.com:443") || erisim("maps.googleapis.com:443")) {
        return Err(HARITA_HATA.to_string());
    }
    Ok("Google API hazır: Places ve Geocoding uç noktalarına erişim var.".to_string())
}

/// Dosya yoksa/boşsa sorun değil (ilk açılış); varsa geçerli JSON olmalı.
fn json_gecerli_yoldan(yol: &Path) -> bool {
    match std::fs::read_to_string(yol) {
        Ok(metin) if metin.trim().is_empty() => true,
        Ok(metin) => serde_json::from_str::<serde_json::Value>(&metin).is_ok(),
        Err(hata) if hata.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

/// Dosyaya yazma izni var mı? Dosya henüz yoksa klasörün iznine bakılır.
fn yazilabilir(yol: &Path) -> bool {
    match std::fs::OpenOptions::new().append(true).open(yol) {
        Ok(_) => true,
        Err(hata) if hata.kind() == std::io::ErrorKind::NotFound => {
            let deneme = yol.with_file_name(YAZMA_TESTI);
            let oldu = std::fs::write(&deneme, b"nexus").is_ok();
            let _ = std::fs::remove_file(&deneme);
            oldu
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Otomatik onarım
// ---------------------------------------------------------------------------

/// Otomatik onarım: dosyaları onarır, önbellekleri temizler, modeli başlatır.
pub fn onar() -> Vec<String> {
    let mut yapilan = onar_klasorde(".");
    if ai::yokla().aktif {
        yapilan.push("Yapay zeka servisi zaten çalışıyor.".to_string());
    } else {
        match ai::yeniden_baslat_dene() {
            Ok(()) => {
                yapilan.push("Yapay zeka servisi (Ollama) arka planda başlatıldı.".to_string())
            }
            Err(_) => {
                yapilan.push("Ollama başlatılamadı; uygulamayı elle açmanız gerekiyor.".to_string())
            }
        }
    }
    yapilan
}

/// Onarımın dosya kısmı: testler bu işlevi geçici klasörde koşturur.
#[allow(clippy::permissions_set_readonly_false)]
fn onar_klasorde(klasor: &str) -> Vec<String> {
    let kok = Path::new(klasor);
    let mut yapilan = Vec::new();
    for ad in DOSYALAR {
        let yol = kok.join(ad);
        // 1) Salt okunur işaretini kaldır (yazma iznini geri ver).
        if let Ok(meta) = std::fs::metadata(&yol) {
            if meta.permissions().readonly() {
                let mut izin = meta.permissions();
                izin.set_readonly(false);
                if std::fs::set_permissions(&yol, izin).is_ok() {
                    yapilan.push(format!("{ad}: salt okunur izni kaldırıldı."));
                }
            }
        }
        // 2) Eksik/bozuk dosyayı varsayılan şablonla yeniden oluştur.
        let eksik = !yol.exists();
        let bozuk = !eksik && !json_gecerli_yoldan(&yol);
        if !eksik && !bozuk {
            continue;
        }
        if bozuk {
            let yedek = kok.join(format!("{ad}.bozuk"));
            let _ = std::fs::remove_file(&yedek);
            let _ = std::fs::rename(&yol, &yedek);
        }
        let oldu = varsayilan_yaz(&yol) && json_gecerli_yoldan(&yol);
        yapilan.push(if !oldu {
            format!("{ad}: yeniden oluşturulamadı; klasör yazma iznini kontrol edin.")
        } else if eksik {
            format!("{ad}: eksikti, varsayılan şablonla oluşturuldu.")
        } else {
            format!("{ad}: bozuktu, yedeklendi ve varsayılan şablonla yeniden oluşturuldu.")
        });
    }
    // 3) Bayat önbellekleri temizle (kalıcı veri değil).
    for ad in ONBELLEKLER {
        let yol = kok.join(ad);
        if yol.exists() && std::fs::remove_file(&yol).is_ok() {
            yapilan.push(format!("{ad}: geçici önbellek temizlendi."));
        }
    }
    if yapilan.is_empty() {
        yapilan.push("Dosyalar sağlam; onarım gerekmedi.".to_string());
    }
    yapilan
}

/// Dosya adına göre varsayılan (geçerli şema) içeriği yazar.
fn varsayilan_yaz(yol: &Path) -> bool {
    let metin = yol.to_string_lossy().to_string();
    let ad = yol
        .file_name()
        .map(|ad| ad.to_string_lossy().to_string())
        .unwrap_or_default();
    match ad.as_str() {
        musteriler::DEFTER_DOSYASI => musteriler::Defter::default().kaydet_yoldan(&metin),
        hedef::DOSYA => {
            hedef::HedefListesi::default().kaydet_yoldan(&metin);
            true
        }
        gelen::DOSYA => {
            let mut kutu = gelen::GelenKutusu::default();
            kutu.port_ayarla(gelen::VARSAYILAN_PORT);
            kutu.kaydet_yoldan(&metin)
        }
        _ => false,
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;

    fn gecici_klasor(ad: &str) -> PathBuf {
        let yol = std::env::temp_dir().join(format!("nexus_onarim_{}_{ad}", std::process::id()));
        let _ = std::fs::remove_dir_all(&yol);
        std::fs::create_dir_all(&yol).expect("geçici klasör açılmalı");
        yol
    }

    fn gecici_dosya(ad: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nexus_saglik_{}_{ad}", std::process::id()))
    }

    #[test]
    fn olmayan_dosya_hata_sayilmaz() {
        assert!(
            json_gecerli_yoldan(&gecici_dosya("yok.json")),
            "ilk açılışta dosya yoktur"
        );
    }

    #[test]
    fn bos_dosya_hata_sayilmaz() {
        let yol = gecici_dosya("bos.json");
        std::fs::write(&yol, "  \n").expect("yazılmalı");
        assert!(json_gecerli_yoldan(&yol));
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn bozuk_json_hata_verir() {
        let yol = gecici_dosya("bozuk.json");
        std::fs::write(&yol, "{ \"kayitlar\": [ ").expect("yazılmalı");
        assert!(!json_gecerli_yoldan(&yol), "yarım JSON bozuk sayılmalı");
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn gecerli_json_kabul_edilir() {
        let yol = gecici_dosya("saglam.json");
        std::fs::write(&yol, r#"{"surum":1,"kayitlar":[]}"#).expect("yazılmalı");
        assert!(json_gecerli_yoldan(&yol));
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)]
    fn salt_okunur_dosya_yazilamaz_sayilir() {
        let yol = gecici_dosya("salt_okunur.json");
        std::fs::write(&yol, "{}").expect("yazılmalı");
        let mut izin = std::fs::metadata(&yol).expect("meta").permissions();
        izin.set_readonly(true);
        std::fs::set_permissions(&yol, izin).expect("izin");
        assert!(!yazilabilir(&yol), "salt okunur dosya yazılamaz sayılmalı");
        let mut izin = std::fs::metadata(&yol).expect("meta").permissions();
        izin.set_readonly(false);
        std::fs::set_permissions(&yol, izin).expect("izin");
        assert!(yazilabilir(&yol));
        let _ = std::fs::remove_file(&yol);
    }

    #[test]
    fn maddeler_sirada_bekler_ve_gecmez() {
        let denetim = Denetim::default();
        assert_eq!(denetim.maddeler.len(), 4, "dört kritik bileşen");
        assert!(denetim
            .maddeler
            .iter()
            .all(|madde| madde.durum == Durum::Bekliyor));
        assert!(!denetim.gecti(), "denetim koşmadan ana ekran açılmamalı");
        assert!(!denetim.bitti);
    }

    #[test]
    fn hata_aciklamalari_sade_ve_cozum_odakli() {
        for mesaj in [
            VERI_HATA,
            YAZMA_HATA,
            KOPRU_HATA,
            OLLAMA_HATA,
            MODEL_HATA,
            HARITA_HATA,
        ] {
            assert!(mesaj.starts_with("Başarısız!"), "{mesaj}");
            assert!(mesaj.ends_with('.'), "cümle bitmeli: {mesaj}");
            for teknik in ["Err(", "0x", "panic", "unwrap", "HTTP", "status"] {
                assert!(!mesaj.contains(teknik), "teknik ayrıntı sızmamalı: {mesaj}");
            }
        }
        assert!(KOPRU_HATA.contains("8777"));
        assert!(OLLAMA_HATA.contains("Ollama"));
        assert!(HARITA_HATA.contains("İnternet"));
        assert!(VERI_HATA.contains("Müşteri defteri"));
        assert!(MODEL_HATA.contains("model"));
    }

    #[test]
    fn durum_isaretleri_ayirt_edilir() {
        let isaretler: Vec<&str> = [
            Durum::Bekliyor,
            Durum::Calisiyor,
            Durum::Gecti,
            Durum::Kaldi,
        ]
        .iter()
        .map(|durum| durum.isaret())
        .collect();
        let essiz: std::collections::HashSet<&str> = isaretler.iter().copied().collect();
        assert_eq!(essiz.len(), 4, "her durum ayrı işaret taşımalı");
    }

    #[test]
    fn hepsi_yesilse_ana_ekran_acilir() {
        let (tx, rx) = channel();
        let mut denetim = Denetim {
            rx: Some(rx),
            ..Default::default()
        };
        for sira in 0..4 {
            let _ = tx.send(Haber::Bitti(sira, Ok("hazır".to_string())));
        }
        denetim.isle();
        assert!(denetim.bitti);
        assert!(denetim.basarili);
        assert!(denetim.gecti(), "dört madde de geçince açılmalı");
    }

    #[test]
    fn tek_kirmizi_madde_acilisi_engeller() {
        let (tx, rx) = channel();
        let mut denetim = Denetim {
            rx: Some(rx),
            ..Default::default()
        };
        for sira in 0..3 {
            let _ = tx.send(Haber::Bitti(sira, Ok("hazır".to_string())));
        }
        let _ = tx.send(Haber::Bitti(3, Err(HARITA_HATA.to_string())));
        denetim.isle();
        assert!(denetim.bitti, "denetim bitmeli");
        assert!(!denetim.basarili);
        assert!(!denetim.gecti(), "tek kırmızı madde açılışı bloke etmeli");
        assert_eq!(denetim.maddeler[3].durum, Durum::Kaldi);
        assert_eq!(denetim.maddeler[3].aciklama, HARITA_HATA);
    }

    #[test]
    fn durdurulan_denetim_paneli_acik_tutar() {
        let mut denetim = Denetim::default();
        denetim.durdur();
        assert!(denetim.bitti);
        assert!(denetim.durduruldu);
        assert!(!denetim.gecti(), "durdurulan denetim uygulamayı açmamalı");
        assert!(denetim
            .maddeler
            .iter()
            .all(|madde| madde.durum == Durum::Bekliyor));
    }

    #[test]
    fn sifirlama_maddeleri_basa_alir() {
        let mut denetim = Denetim::default();
        for madde in &mut denetim.maddeler {
            madde.durum = Durum::Gecti;
        }
        denetim.bitti = true;
        denetim.basarili = true;
        denetim.sifirla();
        assert!(!denetim.bitti && !denetim.basarili && !denetim.durduruldu);
        assert!(denetim
            .maddeler
            .iter()
            .all(|madde| madde.durum == Durum::Bekliyor));
    }

    #[test]
    fn onarim_eksik_dosyalari_olusturur() {
        let klasor = gecici_klasor("eksik");
        let yapilan = onar_klasorde(&klasor.to_string_lossy());
        for ad in DOSYALAR {
            let yol = klasor.join(ad);
            assert!(yol.exists(), "{ad} oluşturulmalı");
            assert!(json_gecerli_yoldan(&yol), "{ad} geçerli JSON olmalı");
        }
        assert!(
            yapilan.iter().any(|satir| satir.contains("oluşturuldu")),
            "{yapilan:?}"
        );
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn onarim_bozuk_defteri_yeniden_olusturur() {
        let klasor = gecici_klasor("bozukdefter");
        let defter = klasor.join(musteriler::DEFTER_DOSYASI);
        std::fs::write(&defter, "{ \"musteriler\": [ bozuk").expect("yazılmalı");
        let yapilan = onar_klasorde(&klasor.to_string_lossy());
        assert!(json_gecerli_yoldan(&defter), "onarım geçerli JSON yazmalı");
        assert!(
            klasor
                .join(format!("{}.bozuk", musteriler::DEFTER_DOSYASI))
                .exists(),
            "bozuk dosya yedeklenmeli"
        );
        assert!(
            yapilan
                .iter()
                .any(|satir| satir.contains("yeniden oluşturuldu")),
            "{yapilan:?}"
        );
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn onarim_saglam_dosyaya_dokunmaz() {
        let klasor = gecici_klasor("saglam");
        for ad in DOSYALAR {
            std::fs::write(klasor.join(ad), r#"{"surum":1}"#).expect("yazılmalı");
        }
        let once = std::fs::read_to_string(klasor.join(musteriler::DEFTER_DOSYASI)).expect("okuma");
        let yapilan = onar_klasorde(&klasor.to_string_lossy());
        assert_eq!(
            std::fs::read_to_string(klasor.join(musteriler::DEFTER_DOSYASI)).expect("okuma"),
            once,
            "sağlam dosya değiştirilmemeli"
        );
        assert!(
            yapilan
                .iter()
                .any(|satir| satir.contains("onarım gerekmedi")),
            "{yapilan:?}"
        );
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn onarim_bayat_onbellegi_siler() {
        let klasor = gecici_klasor("onbellek");
        for ad in DOSYALAR {
            std::fs::write(klasor.join(ad), r#"{"surum":1}"#).expect("yazılmalı");
        }
        for ad in ONBELLEKLER {
            std::fs::write(klasor.join(ad), "{}").expect("yazılmalı");
        }
        let yapilan = onar_klasorde(&klasor.to_string_lossy());
        for ad in ONBELLEKLER {
            assert!(!klasor.join(ad).exists(), "{ad} silinmeli");
        }
        assert!(
            yapilan.iter().any(|satir| satir.contains("önbellek")),
            "{yapilan:?}"
        );
        let _ = std::fs::remove_dir_all(&klasor);
    }
}
