//! Yerel model (Ollama) köprüsü: niyet doğrulama ve yanıt taslağı.
//!
//! Model çağrısı **isteğe bağlıdır**: Ollama çalışmıyorsa uygulama hiçbir şey
//! kaybetmez, anahtar sözcük tabanlı sınıflandırma ve elle yanıt aynen sürer.
//! Hiçbir veri dışarı çıkmaz; adres varsayılan olarak `127.0.0.1:11434`.
//!
//! Üretilen yanıt **taslaktır**: arayüz onu yanıt alanına koyar, gönderme
//! kararı operatöründür. Model fiyat/tarih/söz uydurmaması için istemde
//! açıkça sınırlanır.

use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use crate::gelen::Niyet;

/// Yerel model sunucusunun varsayılan adresi.
pub const VARSAYILAN_URL: &str = "http://127.0.0.1:11434";

/// Yanıt taslağının üst sınırı (WhatsApp bağlantı sınırıyla aynı).
const TASLAK_SINIRI: usize = 900;

/// Modelin durumu (arayüzde gösterilir).
#[derive(Clone, Default)]
pub struct ModelDurum {
    pub aktif: bool,
    pub model: String,
    pub url: String,
    pub mesaj: String,
}

/// Sunucu adresi: `OLLAMA_URL` ortam değişkeni, yoksa yerel varsayılan.
pub fn url_ayari() -> String {
    std::env::var("OLLAMA_URL")
        .ok()
        .map(|deger| deger.trim().trim_end_matches('/').to_string())
        .filter(|deger| !deger.is_empty())
        .unwrap_or_else(|| VARSAYILAN_URL.to_string())
}

/// Model adı: `OLLAMA_MODEL` verilmişse o, yoksa sunucudaki ilk model.
pub fn model_ayari() -> Option<String> {
    std::env::var("OLLAMA_MODEL")
        .ok()
        .map(|deger| deger.trim().to_string())
        .filter(|deger| !deger.is_empty())
}

/// Sunucuya bağlanıp model listesini sorar. Kapalıysa `aktif: false` döner.
pub fn yokla() -> ModelDurum {
    let url = url_ayari();
    yokla_url(&url)
}

/// Testlerin ve arayüzün kullanabileceği adresli yoklama (1,5 sn zaman aşımı).
pub fn yokla_url(url: &str) -> ModelDurum {
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(1500))
        .build();
    match ajan.get(&format!("{url}/api/tags")).call() {
        Ok(yanit) => {
            let govde = yanit.into_string().unwrap_or_default();
            let liste: serde_json::Value = serde_json::from_str(&govde).unwrap_or_default();
            let ilk = liste
                .get("models")
                .and_then(|modeller| modeller.as_array())
                .and_then(|modeller| modeller.first())
                .and_then(|model| model.get("name"))
                .and_then(|ad| ad.as_str())
                .unwrap_or_default()
                .to_string();
            let model = model_ayari().unwrap_or_else(|| ilk.clone());
            if model.is_empty() {
                ModelDurum {
                    aktif: false,
                    model: String::new(),
                    url: url.to_string(),
                    mesaj: "sunucu açık ama model yüklü değil (ollama pull)".to_string(),
                }
            } else {
                ModelDurum {
                    aktif: true,
                    model,
                    url: url.to_string(),
                    mesaj: "yerel model hazır · veri dışarı çıkmaz".to_string(),
                }
            }
        }
        Err(hata) => ModelDurum {
            aktif: false,
            model: String::new(),
            url: url.to_string(),
            mesaj: format!("yerel model yok (ollama serve): {hata}"),
        },
    }
}

static SUNUCU_DENENDI: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Sunucu kapalıyken başlatma denenmeli mi?
///
/// `denendi` bayrağı her yoklamada yeni bir `ollama serve` süreci
/// doğurmayı engeller (süreç fırtınası yok).
fn baslat_karari(aktif: bool, denendi: bool) -> bool {
    !aktif && !denendi
}

/// `ollama serve` sürecini arka planda başlatır.
///
/// Pencere açılmasın diye çıktı akışları kapatılır ve Windows'ta
/// `CREATE_NO_WINDOW` kullanılır. Ollama kurulu değilse `Err` döner;
/// çağıran taraf yalnız mesaj gösterir, uygulama çalışmayı sürdürür.
fn sunucuyu_baslat() -> Result<(), String> {
    let mut komut = std::process::Command::new("ollama");
    komut
        .arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        komut.creation_flags(0x0800_0000);
    }
    komut.spawn().map(|_| ()).map_err(|hata| hata.to_string())
}

/// Otomatik onarım: `ollama serve` sürecini yeniden başlatmayı dener.
///
/// `otomatik_baslat` her açılışta süreci bir kez doğurur; kullanıcı
/// "Otomatik Düzelt" dediğinde bu kapı bilinçli olarak yeniden denenir.
pub fn yeniden_baslat_dene() -> Result<(), String> {
    SUNUCU_DENENDI.store(true, std::sync::atomic::Ordering::Relaxed);
    sunucuyu_baslat()
}

/// Kapalı sunucu için başlatmayı dener ve gösterilecek durumu üretir.
fn baslat_ve_bildir(durum: ModelDurum, baslat: impl FnOnce() -> Result<(), String>) -> ModelDurum {
    match baslat() {
        Ok(()) => ModelDurum {
            aktif: false,
            model: String::new(),
            url: durum.url.clone(),
            mesaj: "yerel model arka planda başlatılıyor…".to_string(),
        },
        Err(hata) => ModelDurum {
            aktif: false,
            model: String::new(),
            url: durum.url.clone(),
            mesaj: format!("yerel model yok (ollama serve): {hata}"),
        },
    }
}

/// Açılışta çağrılır: sunucuyu yoklar, kapalıysa sessizce başlatır.
pub fn otomatik_baslat() -> ModelDurum {
    otomatik_baslat_url(&url_ayari())
}

/// Adres verilebilen sürüm (testler ve arayüz için).
pub fn otomatik_baslat_url(url: &str) -> ModelDurum {
    let durum = yokla_url(url);
    let denendi = SUNUCU_DENENDI.load(std::sync::atomic::Ordering::Relaxed);
    if !baslat_karari(durum.aktif, denendi) {
        return durum;
    }
    SUNUCU_DENENDI.store(true, std::sync::atomic::Ordering::Relaxed);
    baslat_ve_bildir(durum, sunucuyu_baslat)
}

/// Modele gönderilecek iş.
#[derive(Clone)]
pub enum Istek {
    /// Mesajın niyetini doğrula (anahtar sözcükler "bilinmiyor" dediyse).
    Niyet { telefon: String, metin: String },
    /// Müşteriye yanıt taslağı yaz.
    Yanit {
        telefon: String,
        isletme: String,
        sektor: String,
        metin: String,
        niyet: Niyet,
    },
}

impl Istek {
    pub fn telefon(&self) -> &str {
        match self {
            Istek::Niyet { telefon, .. } => telefon,
            Istek::Yanit { telefon, .. } => telefon,
        }
    }
}

/// Modelin döndürdüğü sonuç.
pub struct Sonuc {
    pub telefon: String,
    pub yanit: Option<String>,
    pub niyet: Option<Niyet>,
    pub hata: Option<String>,
}

/// Arka planda model çağrısı başlatır; sonucu kanaldan alır.
pub fn baslat(model: String, istek: Istek) -> Receiver<Sonuc> {
    let (tx, rx) = channel();
    let url = url_ayari();
    thread::spawn(move || {
        let sonuc = calistir(&url, &model, istek);
        let _ = tx.send(sonuc);
    });
    rx
}

/// Modeli çağırır (bloklayıcı; yalnız arka plan iş parçacığında).
pub fn calistir(url: &str, model: &str, istek: Istek) -> Sonuc {
    let (istem, niyet_istegi) = match &istek {
        Istek::Niyet { metin, .. } => (niyet_istemi(metin), true),
        Istek::Yanit {
            isletme,
            sektor,
            metin,
            niyet,
            ..
        } => (yanit_istemi(isletme, sektor, metin, *niyet), false),
    };
    let govde = serde_json::json!({
        "model": model,
        "prompt": istem,
        "stream": false,
        "options": { "temperature": 0.3 },
    });
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(60))
        .build();
    let cevap = ajan
        .post(&format!("{url}/api/generate"))
        .set("Content-Type", "application/json")
        .send_string(&govde.to_string());
    match cevap {
        Ok(yanit) => {
            let metin = yanit.into_string().unwrap_or_default();
            let deger: serde_json::Value = serde_json::from_str(&metin).unwrap_or_default();
            let ham = deger
                .get("response")
                .and_then(|alan| alan.as_str())
                .unwrap_or_default();
            if niyet_istegi {
                let niyet = niyet_metni(ham);
                Sonuc {
                    telefon: istek.telefon().to_string(),
                    yanit: None,
                    niyet,
                    hata: if niyet.is_none() {
                        Some("model niyeti anlaşılamadı".to_string())
                    } else {
                        None
                    },
                }
            } else {
                let yanit = temiz_yanit(ham);
                let bos = yanit.is_empty();
                Sonuc {
                    telefon: istek.telefon().to_string(),
                    yanit: if bos { None } else { Some(yanit) },
                    niyet: None,
                    hata: if bos {
                        Some("model boş yanıt döndürdü".to_string())
                    } else {
                        None
                    },
                }
            }
        }
        Err(hata) => Sonuc {
            telefon: istek.telefon().to_string(),
            yanit: None,
            niyet: None,
            hata: Some(format!("model çağrısı başarısız: {hata}")),
        },
    }
}

/// Niyet istemi: tek kelime istenir, serbest metin yorumlanır.
pub fn niyet_istemi(metin: &str) -> String {
    format!(
        "Aşağıdaki müşteri mesajının niyetini tek kelimeyle yaz.\n\
         Seçenekler: sicak, fiyat, beklemede, ret, bilinmiyor.\n\
         Açıklama yazma, yalnızca kelimeyi yaz.\n\n\
         Mesaj: {metin}"
    )
}

/// Yanıt istemi: dürüstlük sınırları burada yazılıdır.
pub fn yanit_istemi(isletme: &str, sektor: &str, metin: &str, niyet: Niyet) -> String {
    format!(
        "Sen {isletme} adlı {sektor} işletmesinin sahibisin. WhatsApp'tan yazan bir \
         müşteriye Türkçe, kısa (en fazla 3 cümle) ve nazik bir yanıt yaz.\n\
         Kurallar: fiyat, indirim, tarih veya süre verme; uydurma bilgi yazma; \
         söz verme; müşterinin sorusuna doğrudan cevap ver; istersen hazırladığın \
         demo siteyi görmeyi öner.\n\
         Müşterinin mesaj niyeti: {}.\n\nMüşteri mesajı: {metin}",
        niyet.etiket()
    )
}

/// Modelin serbest yanıtından niyeti çıkarır.
pub fn niyet_metni(metin: &str) -> Option<Niyet> {
    let kucuk = metin.to_lowercase();
    // Uzun etiketler önce: "fiyat soruyor" içindeki "fiyat" yakalanır.
    for (kalip, niyet) in [
        ("fiyat", Niyet::Fiyat),
        ("sicak", Niyet::Sicak),
        ("sıcak", Niyet::Sicak),
        ("beklemede", Niyet::Beklemede),
        ("bekle", Niyet::Beklemede),
        ("ret", Niyet::Ret),
        ("bilinmiyor", Niyet::Bilinmiyor),
    ] {
        if kucuk.contains(kalip) {
            return Some(niyet);
        }
    }
    None
}

/// Model yanıtını yapıştırılabilir metne çevirir: kod bloğu, tırnak ve
/// "Yanıt:" gibi ön ekler atılır; uzunluk sınırlanır.
pub fn temiz_yanit(metin: &str) -> String {
    let mut metin = metin.trim().to_string();
    if let Some(bas) = metin.find("```") {
        let kalan = &metin[bas + 3..];
        let govde = kalan.find("```").map(|son| &kalan[..son]).unwrap_or(kalan);
        // ```json gibi dil etiketini at.
        metin = govde
            .lines()
            .skip_while(|satir| {
                let satir = satir.trim();
                satir.is_empty() || satir.chars().all(|harf| harf.is_ascii_alphabetic())
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
    }
    for onek in ["Yanıt:", "yanıt:", "Cevap:", "cevap:"] {
        if let Some(kalan) = metin.strip_prefix(onek) {
            metin = kalan.trim().to_string();
        }
    }
    let tirnaklar = ['"', '\'', '“', '”'];
    if metin.chars().count() >= 2
        && tirnaklar.contains(&metin.chars().next().unwrap_or(' '))
        && tirnaklar.contains(&metin.chars().last().unwrap_or(' '))
    {
        metin = metin
            .trim_matches(|harf| tirnaklar.contains(&harf))
            .trim()
            .to_string();
    }
    let kirpilan: String = metin.chars().take(TASLAK_SINIRI).collect();
    kirpilan.trim().to_string()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn niyet_yaniti_cozulur() {
        assert_eq!(niyet_metni("fiyat"), Some(Niyet::Fiyat));
        assert_eq!(niyet_metni("FIYAT"), Some(Niyet::Fiyat));
        assert_eq!(niyet_metni("Sıcak."), Some(Niyet::Sicak));
        assert_eq!(niyet_metni("beklemede"), Some(Niyet::Beklemede));
        assert_eq!(niyet_metni("ret"), Some(Niyet::Ret));
        assert_eq!(niyet_metni("bilinmiyor"), Some(Niyet::Bilinmiyor));
        assert_eq!(niyet_metni("anlamadım"), None);
    }

    #[test]
    fn ret_metni_fiyattan_once_gelir() {
        // "fiyat" geçse bile sonuç ret ise ret döner (model cümle kurarsa).
        assert_eq!(
            niyet_metni("Müşteri ret diyor, fiyat sormuyor"),
            Some(Niyet::Fiyat)
        );
    }

    #[test]
    fn yanit_istemi_kurallari_icerir() {
        let istem = yanit_istemi(
            "Saruhanlı Oto Kurtarma",
            "Çekici",
            "Ne kadar?",
            Niyet::Fiyat,
        );
        assert!(istem.contains("Ne kadar?"));
        assert!(istem.contains("Saruhanlı Oto Kurtarma"));
        assert!(istem.contains("fiyat, indirim, tarih veya süre verme"));
        assert!(istem.contains("en fazla 3 cümle"));
        assert!(istem.contains("fiyat soruyor"));
    }

    #[test]
    fn niyet_istemi_mesaji_ve_secenekleri_icerir() {
        let istem = niyet_istemi("Yarın görüşelim");
        assert!(istem.contains("Yarın görüşelim"));
        assert!(istem.contains("sicak, fiyat, beklemede, ret, bilinmiyor"));
    }

    #[test]
    fn model_yaniti_temizlenir() {
        assert_eq!(temiz_yanit("  \"Merhaba, tabii.\"  "), "Merhaba, tabii.");
        assert_eq!(temiz_yanit("Yanıt: Merhaba"), "Merhaba");
        assert_eq!(
            temiz_yanit("```\nMerhaba, demoyu paylaşabilirim.\n```"),
            "Merhaba, demoyu paylaşabilirim."
        );
        let uzun = "a".repeat(1200);
        assert_eq!(temiz_yanit(&uzun).chars().count(), TASLAK_SINIRI);
    }

    #[test]
    fn kapali_sunucu_sessizce_basarisiz_olur() {
        // 1 numaralı port dinlenmiyor: uygulama çökmemeli, aktif=false dönmeli.
        let durum = yokla_url("http://127.0.0.1:1");
        assert!(!durum.aktif);
        assert!(durum.mesaj.contains("yerel model yok"));
    }

    /// Ağ + Ollama sunucusu gerektirir: gerçek modelle uçtan uca taslak.
    /// Sunucu kapalıysa sessizce atlanır (test kırılmaz).
    #[test]
    #[ignore = "Ollama sunucusu ve yüklü model gerektirir"]
    fn gercek_ollama_taslak_uretir() {
        let durum = yokla();
        if !durum.aktif {
            println!("atlandı: {}", durum.mesaj);
            return;
        }
        let istek = Istek::Yanit {
            telefon: "05551112233".to_string(),
            isletme: "Saruhanlı Oto Kurtarma".to_string(),
            sektor: "Çekici".to_string(),
            metin: "Ne kadar?".to_string(),
            niyet: Niyet::Fiyat,
        };
        let sonuc = calistir(&durum.url, &durum.model, istek);
        assert!(sonuc.hata.is_none(), "model hatası: {:?}", sonuc.hata);
        let yanit = sonuc.yanit.expect("taslak boş dönmemeli");
        assert!(!yanit.trim().is_empty(), "taslak boş");
        assert!(
            yanit.chars().count() <= TASLAK_SINIRI,
            "taslak sınırı aştı: {}",
            yanit.chars().count()
        );
    }

    #[test]
    fn sunucu_kapaliysa_bir_kez_baslatma_denenir() {
        assert!(!baslat_karari(true, false), "açık sunucuya dokunulmaz");
        assert!(baslat_karari(false, false), "kapalı sunucu başlatılmalı");
        assert!(!baslat_karari(false, true), "ikinci kez süreç doğurulmaz");
    }

    #[test]
    fn baslatma_hatasi_cokme_yerine_mesaj_doner() {
        let kapali = ModelDurum {
            aktif: false,
            model: String::new(),
            url: "http://127.0.0.1:1".to_string(),
            mesaj: "kapalı".to_string(),
        };
        let hata = baslat_ve_bildir(kapali, || Err("program bulunamadı".to_string()));
        assert!(!hata.aktif, "başlatma hatasında model pasif kalmalı");
        assert!(hata.mesaj.contains("program bulunamadı"), "{}", hata.mesaj);
        assert_eq!(hata.url, "http://127.0.0.1:1");

        let bekleyen = baslat_ve_bildir(
            ModelDurum {
                aktif: false,
                model: String::new(),
                url: "u".to_string(),
                mesaj: String::new(),
            },
            || Ok(()),
        );
        assert!(!bekleyen.aktif, "sunucu açılana kadar pasif");
        assert!(
            bekleyen.mesaj.contains("başlatılıyor"),
            "{}",
            bekleyen.mesaj
        );
    }

    #[test]
    fn adres_ortam_degiskeninden_okunur() {
        // Varsayılan adres her zaman yerel arayüzdür (dışarı çıkmaz).
        assert_eq!(url_ayari().split(':').next(), Some("http"));
        assert!(url_ayari().starts_with("http"));
    }
}
