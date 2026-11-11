//! Sektörel AI doğrulama katmanı: %100 uyum eşiği, sıfır tolerans.
//!
//! Manuel panelden (`--onizleme`) eklenen **her** video ve fotoğraf, sektörün
//! kendi adı ve grup etiketiyle (`.exe` içindeki `SEKTORLER` tablosundan; sabit
//! örnek metin yok) görsel modele sorulur. Video için üç kare çıkarılır, fotoğraf
//! tek kare olarak değerlendirilir. **Bütün kareler ve bütün sağlayıcılar** "evet"
//! demeden dosya kuyruğa alınmaz; tek bir "hayır" ya da belirsiz yanıt reddir.
//!
//! Tasarım kararları:
//! - **Manuel kapsam:** Toplu/otomatik tarama yoktur; doğrulama yalnız panelden
//!   eklenen aday dosya için çağrılır.
//! - **Modüler model:** `Saglayici` arayüzü takılıp çıkarılabilir; varsayılan
//!   sağlayıcı yerel Ollama görsel modelidir, `DOGRULAMA_MODEL` ile sabitlenir,
//!   `DOGRULAMA_IKINCI_MODEL` ile ikinci görüş eklenir (ikisi de evet demeli).
//! - **Fail-closed:** Model yok/yanıt vermiyorsa dosya kuyruğa **alınmaz**; hata
//!   açıkça yazılır, sessiz onay yoktur.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use base64::Engine as _;

use crate::sektor_medya;

/// Videodan kaç kare alınır (hepsi onaylamalı).
pub const KARE_SAYISI: usize = 3;
/// Karelerin saniye cinsinden konumu.
const KARE_ZAMANLARI: [f64; KARE_SAYISI] = [1.0, 5.0, 9.0];
/// Yanıt jeton sınırı: kısa konu satırı + tek kelimelik karar.
const EN_FAZLA_JETON: u32 = 48;

/// Kurulu görsel modeller, doğruluk sırasıyla. İlk bulunan birincil olur.
const GORSEL_MODELLER: &[&str] = &[
    "qwen2.5vl",
    "qwen2.5-vl",
    "qwen2vl",
    "llava",
    "gemma3",
    "minicpm-v",
    "llama3.2-vision",
    "granite3.2-vision",
    "moondream",
];

/// Sektörün AI istemine giden etiketi: ad, grup ve İngilizce karşılığı.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etiket {
    pub ad: String,
    pub grup: String,
    pub ingilizce: String,
}

/// Sektör adı + grup verisinden etiket üretir (sabit metin yok).
pub fn etiket(ad: &str, grup: &str) -> Etiket {
    Etiket {
        ad: ad.to_string(),
        grup: grup.to_string(),
        ingilizce: sektor_medya::ingilizce(grup).to_string(),
    }
}

/// Adı `.exe` içindeki sektör tablosunda arar; bulunamazsa adın kendisinden
/// (slug → okunur metin) etiket üretir.
pub fn etiket_bul(ad: &str) -> Etiket {
    match crate::SEKTORLER.iter().find(|tanim| tanim.ad == ad) {
        Some(tanim) => etiket(tanim.ad, tanim.grup),
        None => Etiket {
            ad: ad.to_string(),
            grup: String::new(),
            ingilizce: sektor_medya::slug(ad).replace('_', " "),
        },
    }
}

/// Klasör (slug) adından sektör etiketi: tabloda varsa gerçek ad + grup,
/// yoksa klasör adının okunur hâli. Disk dinamik olduğu için tablo zorunlu değil.
pub fn slug_etiketi(slug: &str) -> Etiket {
    match crate::SEKTORLER
        .iter()
        .find(|tanim| sektor_medya::slug(tanim.ad) == slug)
    {
        Some(tanim) => etiket_bul(tanim.ad),
        None => Etiket {
            ad: slug.replace('_', " "),
            grup: String::new(),
            ingilizce: slug.replace('_', " "),
        },
    }
}

/// Görsel modele gönderilen katı istem: konu + tek kelimelik karar.
pub fn istem_metni(etiket: &Etiket) -> String {
    let grup = if etiket.grup.is_empty() {
        String::new()
    } else {
        format!(", category {}", etiket.grup)
    };
    format!(
        "You are an extremely strict reviewer for a Turkish local business website.\n\
         Sector: {ad}{grup}. In English this business is: {en}.\n\
         Reply with exactly three lines.\n\
         Line 1: the main subject of the image in at most 6 words.\n\
         Line 2: CONFIDENCE: an integer 0-100 for how certain you are that this image is \
         a direct, literal photo of a {en}.\n\
         Line 3: YES or NO. Answer YES only if the image unmistakably and directly shows \
         this exact business type and nothing else.\n\
         Answer NO if the image is generic, stock-like, decorative, a close-up of an \
         object that could belong to many businesses, ambiguous, or could be mistaken for \
         another sector; also NO for documents, screenshots, logos, charts, news pictures \
         and text-only images.\n\
                  TURKEY RULE (absolute, zero tolerance): the image must be unmistakably taken \
         in TURKEY, with Turkish people, Turkish signage in Turkish letters and local \
         Turkish street architecture. Answer NO if there is even 1 percent doubt, or any \
         foreign signage, foreign alphabet, non-Turkish people, Western or global stock \
         look, resort or tourism ambiguity, or generic international context.
\
         Line 2 must be CONFIDENCE: 100 only when you are absolutely certain about BOTH \
         the exact sector AND the Turkish location; anything less than 100 means NO.
\
         Visible Turkish evidence is mandatory: Turkish lettering (ç ğ ı ö ş ü), a \
         Turkish shop sign, Turkish flag or plate, or unmistakably Turkish street \
         architecture. A clean, generic, Western-looking scene is NOT Turkey: answer NO.
\
         When in doubt, answer NO.",
        ad = etiket.ad,
        en = etiket.ingilizce
    )
}

/// Modelin tek kare için kararı.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Karar {
    Evet,
    Hayir,
    Belirsiz,
}

/// Sağlayıcının tek kare yanıtı: karar ve ham metin (günlük/karantina gerekçesi).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cevap {
    pub karar: Karar,
    pub metin: String,
}

/// Takılıp çıkarılabilen doğrulama motoru.
///
/// Yeni bir model/backend eklemek için bu arayüz uygulanır ve `zincir_kur`
/// içindeki zincire eklenir; çağıran taraf (tedarik hattı, tarama) değişmez.
pub trait Saglayici: Send + Sync {
    /// Günlükte görünen ad (`ollama/qwen2.5vl:3b` gibi).
    fn ad(&self) -> String;
    /// Tek kareyi sektör etiketiyle değerlendirir.
    fn cevapla(&self, kare: &[u8], etiket: &Etiket) -> Result<Cevap, String>;
    /// Modeli bellekte tutmayı bırakır (VRAM boşaltma). Gerçekten boşalttıysa
    /// `true` döner; takılabilir sağlayıcılar için varsayılan `false`.
    fn bosalt(&self) -> bool {
        false
    }
}

/// Yerel Ollama görsel modeli üzerinden doğrulama.
///
/// `kalici` birincil modeldir: sık kullanıldığı için bellekte tutulur.
/// `ikincil` yedek modeldir: yalnızca şüpheli karelerde çağrılır ve işi bitince
/// VRAM'den düşürülür (6 GB kartta iki model aynı anda yüklenmesin).
pub struct OllamaGorsel {
    model: String,
    kalici: bool,
}

impl OllamaGorsel {
    /// Birincil (sıcak tutulan) model.
    pub fn yeni(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            kalici: true,
        }
    }

    /// Yedek (yalnız şüphede çağrılan, sonra boşaltılan) model.
    pub fn ikincil(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            kalici: false,
        }
    }
}

impl Saglayici for OllamaGorsel {
    fn ad(&self) -> String {
        format!("ollama/{}", self.model)
    }

    fn cevapla(&self, kare: &[u8], etiket: &Etiket) -> Result<Cevap, String> {
        let metin = sor(&self.model, kare, &istem_metni(etiket), self.kalici)?;
        Ok(Cevap {
            karar: karar_oku(&metin),
            metin: metin.trim().replace('\n', " "),
        })
    }

    fn bosalt(&self) -> bool {
        model_bosalt(&self.model)
    }
}

/// Serbest yanıtı karara çevirir: son geçen `yes/no` kelimesi karardır
/// (istem ikinci satırda karar ister; uzun açıklamalar da çözülür).
pub fn karar_oku(cevap: &str) -> Karar {
    let temel = karar_oku_temel(cevap);
    if temel != Karar::Evet {
        return temel;
    }
    match guven_oku(cevap) {
        // Skor yoksa da onay yok: yalnız %100 (tam eşleşme) kabul edilir.
        Some(guven) if guven >= guven_esigi() => Karar::Evet,
        _ => Karar::Belirsiz,
    }
}

/// İkinci satırdaki 0-100 güven puanı (yoksa None).
fn guven_oku(cevap: &str) -> Option<u8> {
    for satir in cevap.lines() {
        let kucuk = satir.to_lowercase();
        if !(kucuk.contains("confidence") || kucuk.contains("güven") || kucuk.contains("guven")) {
            continue;
        }
        // Yalnız "confidence/güven" etiketinden SONRAKI sayı okunur; satır
        // numarası ("Line 2") güven puanı sanılmasın.
        let kalan = kucuk
            .split_once("confidence")
            .or_else(|| kucuk.split_once("güven"))
            .or_else(|| kucuk.split_once("guven"))
            .map(|(_, kalan)| kalan);
        if let Some(kalan) = kalan {
            let sayilar = kalan.split_once(':').map(|(_, k)| k).unwrap_or(kalan);
            if let Some(sayi) = sayilar
                .split(|karakter: char| !karakter.is_ascii_digit())
                .filter(|parca| !parca.is_empty())
                .filter_map(|parca| parca.parse::<u8>().ok())
                .find(|sayi| *sayi <= 100)
            {
                return Some(sayi);
            }
        }
    }
    None
}

/// Tavizsiz eşik: yalnız %100 (tam eşleşme) onay kuyruğuna girer.
pub const GUVEN_ESIK: u8 = 100;

/// Tek eşik: %100. Ortam ayarı eşiği düşüremez; yalnız belgelenir.
fn esik_degeri(_ayar: Option<u8>) -> u8 {
    GUVEN_ESIK
}

/// Güven eşiği: her kare %100 güvenle onaylanmalıdır.
fn guven_esigi() -> u8 {
    esik_degeri(
        std::env::var("DOGRULAMA_GUVEN_ESIK")
            .ok()
            .and_then(|deger| deger.trim().parse::<u8>().ok()),
    )
}

/// Ham yanıtın son geçen yes/no kelimesi (güven eşiği uygulanmadan).
fn karar_oku_temel(cevap: &str) -> Karar {
    let kucuk = cevap.to_lowercase();
    let temiz: String = kucuk
        .chars()
        .map(|harf| if harf.is_alphanumeric() { harf } else { ' ' })
        .collect();
    let mut son = None;
    for kelime in temiz.split_whitespace() {
        match kelime {
            "yes" | "evet" => son = Some(Karar::Evet),
            "no" | "hayir" | "hayır" => son = Some(Karar::Hayir),
            _ => {}
        }
    }
    son.unwrap_or(Karar::Belirsiz)
}

/// Kare kararlarının bileşkesi: **tamamı** evet değilse onay yok.
pub fn kareler_onayli(kararlar: &[Karar]) -> bool {
    !kararlar.is_empty() && kararlar.iter().all(|karar| *karar == Karar::Evet)
}

/// Tek dosyanın doğrulama sonucu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sonuc {
    /// Bütün kareler sektörle eşleşti.
    Onay { kare: usize, skor: u8 },
    /// En az bir kare eşleşmedi/şüpheli.
    Red { kare: usize, cevaplar: Vec<String> },
    /// Doğrulama yapılamadı (model yok, kare çıkmadı): dosya arşive alınmaz.
    Yok { neden: String },
}

impl Sonuc {
    /// Günlük/rapor satırı.
    pub fn metin(&self) -> String {
        match self {
            Sonuc::Onay { kare, .. } => format!("onay ({kare} kare)"),
            Sonuc::Red { kare, cevaplar } => {
                format!("RED ({kare} kare: {})", cevaplar.join(" | "))
            }
            Sonuc::Yok { neden } => format!("doğrulanamadı: {neden}"),
        }
    }
}

fn ortam_url() -> String {
    crate::ai::url_ayari()
}

/// Sunucudaki model adları (`ollama list` ile aynı kaynak).
fn kurulu_modeller() -> Result<Vec<String>, String> {
    let url = ortam_url();
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    let govde = ajan
        .get(&format!("{url}/api/tags"))
        .call()
        .map_err(|hata| format!("yerel model sunucusu yok ({url}): {hata}"))?
        .into_string()
        .unwrap_or_default();
    let deger: serde_json::Value = serde_json::from_str(&govde).unwrap_or_default();
    Ok(deger
        .get("models")
        .and_then(|liste| liste.as_array())
        .map(|liste| {
            liste
                .iter()
                .filter_map(|model| model.get("name").and_then(|ad| ad.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

/// Zincir kurulumu: birincil görsel model + istenirse ikinci görüş.
///
/// `DOGRULAMA_MODEL` birincil modeli sabitler, `DOGRULAMA_IKINCI_MODEL` yedek
/// modeli ekler. Zincir sıralı çalışır: yedek model yalnızca birincil modelin
/// kararsız kaldığı karelerde çağrılır (VRAM'de aynı anda iki model tutulmaz).
fn zincir_kur() -> Result<Vec<Box<dyn Saglayici>>, String> {
    let adlar = kurulu_modeller()?;
    let bul = |istenen: &str| -> Option<String> {
        let istenen = istenen.trim().to_lowercase();
        adlar
            .iter()
            .find(|ad| ad.to_lowercase() == istenen)
            .or_else(|| adlar.iter().find(|ad| ad.to_lowercase().contains(&istenen)))
            .cloned()
    };
    let birincil = match std::env::var("DOGRULAMA_MODEL") {
        Ok(istenen) if !istenen.trim().is_empty() => Some(bul(&istenen).ok_or_else(|| {
            format!(
                "DOGRULAMA_MODEL={} kurulu değil (ollama pull {})",
                istenen.trim(),
                istenen.trim()
            )
        })?),
        _ => GORSEL_MODELLER
            .iter()
            .find_map(|aile| adlar.iter().find(|ad| ad.to_lowercase().contains(aile)))
            .cloned(),
    };
    let Some(birincil) = birincil else {
        return Err("görsel model yüklü değil (ollama pull qwen2.5vl:3b)".to_string());
    };
    let mut zincir: Vec<Box<dyn Saglayici>> = vec![Box::new(OllamaGorsel::yeni(birincil))];
    if let Ok(ikinci) = std::env::var("DOGRULAMA_IKINCI_MODEL") {
        if !ikinci.trim().is_empty() {
            if let Some(ad) = bul(&ikinci) {
                if !zincir
                    .iter()
                    .any(|saglayici| saglayici.ad() == format!("ollama/{ad}"))
                {
                    zincir.push(Box::new(OllamaGorsel::ikincil(ad)));
                }
            }
        }
    }
    Ok(zincir)
}

static ZINCIR: OnceLock<Result<Vec<Box<dyn Saglayici>>, String>> = OnceLock::new();

/// Etkin sağlayıcı zinciri (süreç başına bir kez kurulur).
pub fn zincir() -> Result<&'static [Box<dyn Saglayici>], String> {
    match ZINCIR.get_or_init(zincir_kur) {
        Ok(zincir) => Ok(zincir.as_slice()),
        Err(hata) => Err(hata.clone()),
    }
}

/// Kareyi JPEG olarak çıkarır: video için `saniye` verilir, fotoğrafta `None`.
pub fn kare_cikar(dosya: &Path, saniye: Option<f64>) -> Result<Vec<u8>, String> {
    let mut komut = crate::stok_video::gizli_komut("ffmpeg");
    komut.args(["-y", "-loglevel", "error"]);
    if let Some(saniye) = saniye {
        komut.args(["-ss", &format!("{saniye:.2}")]);
    }
    komut.arg("-i").arg(dosya).args([
        "-frames:v",
        "1",
        "-vf",
        "scale=640:-2",
        "-f",
        "image2pipe",
        "-vcodec",
        "mjpeg",
        "-",
    ]);
    let cikti = komut
        .output()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if !cikti.status.success() || cikti.stdout.is_empty() {
        let hata = String::from_utf8_lossy(&cikti.stderr);
        return Err(format!("kare alınamadı: {}", hata.trim()));
    }
    Ok(cikti.stdout)
}

/// Yerel görsel modele tek kare sorar.
fn sor(model: &str, kare: &[u8], istem: &str, kalici: bool) -> Result<String, String> {
    let url = ortam_url();
    let govde = serde_json::json!({
        "model": model,
        "prompt": istem,
        "images": [base64::engine::general_purpose::STANDARD.encode(kare)],
        "stream": false,
        // Yedek model isteği sonunda belleği bırakır (keep_alive 0).
        "keep_alive": if kalici { "30m" } else { "0" },
        "options": { "temperature": 0, "num_predict": EN_FAZLA_JETON },
    });
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(180))
        .build();
    let yanit = ajan
        .post(&format!("{url}/api/generate"))
        .set("Content-Type", "application/json")
        .send_string(&govde.to_string())
        .map_err(|hata| format!("model çağrısı başarısız: {hata}"))?
        .into_string()
        .unwrap_or_default();
    let deger: serde_json::Value = serde_json::from_str(&yanit).unwrap_or_default();
    if let Some(hata) = deger.get("error").and_then(|h| h.as_str()) {
        return Err(format!("model hatası: {hata}"));
    }
    Ok(deger
        .get("response")
        .and_then(|cevap| cevap.as_str())
        .unwrap_or_default()
        .to_string())
}

/// VRAM bütçesi (MB): 6 GB kartın tamamı değil, güvenli tavan kullanılır.
fn vram_esigi_mb() -> u64 {
    std::env::var("DOGRULAMA_VRAM_ESIK_MB")
        .ok()
        .and_then(|deger| deger.trim().parse().ok())
        .unwrap_or(6000)
}

/// Yedek model için gereken güvenli pay (MB).
const YEDEK_VRAM_MB: u64 = 2500;

/// `nvidia-smi --query-gpu=memory.used` çıktısını MB'a çevirir (saf fonksiyon).
fn vram_metni_coz(metin: &str) -> Option<u64> {
    let toplam: u64 = metin
        .lines()
        .filter_map(|satir| satir.trim().parse::<u64>().ok())
        .sum();
    (toplam > 0).then_some(toplam)
}

/// Anlık VRAM kullanımı (MB); `nvidia-smi` yoksa `None`.
fn vram_kullanilan_mb() -> Option<u64> {
    let cikti = crate::stok_video::gizli_komut("nvidia-smi")
        .args(["--query-gpu=memory.used", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    if !cikti.status.success() {
        return None;
    }
    vram_metni_coz(&String::from_utf8_lossy(&cikti.stdout))
}

/// Modeli VRAM'den düşürür (`keep_alive: 0`). Boşaltma istendi mi?
fn model_bosalt(model: &str) -> bool {
    let url = ortam_url();
    let govde = serde_json::json!({ "model": model, "keep_alive": 0 });
    let ajan = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    ajan.post(&format!("{url}/api/generate"))
        .set("Content-Type", "application/json")
        .send_string(&govde.to_string())
        .is_ok()
}

/// Yedek modele geçmeden önce VRAM'i hazırlar: kullanım + pay bütçeyi aşıyorsa
/// önceki modeller boşaltılır ve belleğin düşmesi beklenir.
fn vram_hazirla(zincir: &[Box<dyn Saglayici>], sira: usize) {
    let butce = vram_esigi_mb();
    let Some(kullanilan) = vram_kullanilan_mb() else {
        return;
    };
    if kullanilan + YEDEK_VRAM_MB <= butce {
        return;
    }
    let mut bosaltti = false;
    for onceki in zincir.iter().take(sira) {
        bosaltti |= onceki.bosalt();
    }
    if !bosaltti {
        return;
    }
    // Boşalmanın oturması için kısa bekleme (en çok 6 sn).
    for _ in 0..12 {
        match vram_kullanilan_mb() {
            Some(simdi) if simdi + YEDEK_VRAM_MB <= butce => break,
            None => break,
            _ => std::thread::sleep(Duration::from_millis(500)),
        }
    }
}

/// Tek karenin kademeli (sıralı) kararı.
///
/// Birincil model **kesin** bir karar verdiyse (evet/hayır) ikinci model hiç
/// çağrılmaz: VRAM ve süre korunur. Yalnızca birincil `Belirsiz` (şüpheli)
/// derse yedek model devreye girer; yedek model işi bitince boşaltılır. Yedek
/// model de belirsizse kare onaylanmaz (fail-closed).
fn kademeli_karar(
    zincir: &[Box<dyn Saglayici>],
    kare: &[u8],
    etiket: &Etiket,
) -> Result<(Karar, Vec<String>), String> {
    let mut metinler = Vec::new();
    let mut son_hata = String::new();
    for (sira, saglayici) in zincir.iter().enumerate() {
        if sira > 0 {
            vram_hazirla(zincir, sira);
        }
        match saglayici.cevapla(kare, etiket) {
            Ok(cevap) => {
                if !cevap.metin.is_empty() {
                    metinler.push(format!("[{}] {}", saglayici.ad(), cevap.metin));
                }
                if cevap.karar != Karar::Belirsiz {
                    return Ok((cevap.karar, metinler));
                }
                // Yedek model bellekte tutulmaz; birincil sıcak kalır.
                if sira > 0 {
                    saglayici.bosalt();
                }
            }
            Err(hata) => son_hata = format!("{}: {hata}", saglayici.ad()),
        }
    }
    if metinler.is_empty() {
        return Err(if son_hata.is_empty() {
            "sağlayıcı yok".to_string()
        } else {
            son_hata
        });
    }
    Ok((Karar::Belirsiz, metinler))
}

/// Dosyayı sektör etiketiyle doğrular: video için üç kare, fotoğraf için tek kare.
pub fn dogrula_dosya(dosya: &Path, etiket: &Etiket, video: bool) -> Sonuc {
    let zincir = match zincir() {
        Ok(zincir) => zincir,
        Err(neden) => return Sonuc::Yok { neden },
    };
    let kareler: Vec<Vec<u8>> = if video {
        let mut kareler = Vec::with_capacity(KARE_SAYISI);
        for saniye in KARE_ZAMANLARI {
            match kare_cikar(dosya, Some(saniye)) {
                Ok(kare) => kareler.push(kare),
                Err(neden) => return Sonuc::Yok { neden },
            }
        }
        kareler
    } else {
        match kare_cikar(dosya, None) {
            Ok(kare) => vec![kare],
            Err(neden) => return Sonuc::Yok { neden },
        }
    };
    let mut kararlar = Vec::with_capacity(kareler.len());
    let mut cevaplar = Vec::with_capacity(kareler.len());
    for kare in &kareler {
        match kademeli_karar(zincir, kare, etiket) {
            Ok((karar, metinler)) => {
                kararlar.push(karar);
                cevaplar.extend(metinler);
            }
            Err(neden) => return Sonuc::Yok { neden },
        }
    }
    // En düşük kare güveni: video için en zayıf kare bağlayıcıdır.
    let skor = cevaplar
        .iter()
        .filter_map(|metin| guven_oku(metin))
        .min()
        .unwrap_or(0);
    if kareler_onayli(&kararlar) && skor >= guven_esigi() {
        Sonuc::Onay {
            kare: kararlar.len(),
            skor,
        }
    } else {
        Sonuc::Red {
            kare: kararlar.len(),
            cevaplar,
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Testler için takılabilir sahte sağlayıcı (ağ/model gerekmez).
    struct SahteSaglayici {
        karar: Karar,
        hata: Option<String>,
        cagri: Arc<AtomicUsize>,
        bosaltma: Arc<AtomicUsize>,
    }

    impl Saglayici for SahteSaglayici {
        fn ad(&self) -> String {
            "sahte".to_string()
        }

        fn cevapla(&self, _kare: &[u8], _etiket: &Etiket) -> Result<Cevap, String> {
            self.cagri.fetch_add(1, Ordering::Relaxed);
            match &self.hata {
                Some(hata) => Err(hata.clone()),
                None => Ok(Cevap {
                    karar: self.karar,
                    metin: format!("{:?}", self.karar),
                }),
            }
        }

        // Gerçek boşaltma yok: testlerde VRAM beklemesi tetiklenmesin.
        fn bosalt(&self) -> bool {
            self.bosaltma.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Sayaçlı sahte sağlayıcı: (sağlayıcı, çağrı sayacı, boşaltma sayacı).
    fn sayacli(
        karar: Karar,
        hata: Option<&str>,
    ) -> (Box<dyn Saglayici>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let cagri = Arc::new(AtomicUsize::new(0));
        let bosaltma = Arc::new(AtomicUsize::new(0));
        let saglayici = Box::new(SahteSaglayici {
            karar,
            hata: hata.map(str::to_string),
            cagri: Arc::clone(&cagri),
            bosaltma: Arc::clone(&bosaltma),
        });
        (saglayici, cagri, bosaltma)
    }

    #[test]
    fn sektor_etiketi_tablodan_gelir() {
        let bulunan = etiket_bul("Restoran");
        assert_eq!(bulunan.ad, "Restoran");
        assert_eq!(bulunan.grup, "Lokanta & Kebap");
        assert_eq!(bulunan.ingilizce, "restaurant");
        // Tabloda olmayan ad kendi metninden etiket üretir (sabit liste yok).
        let yedek = etiket_bul("Özel Sektör");
        assert_eq!(yedek.ad, "Özel Sektör");
        assert!(yedek.ingilizce.contains("ozel"));
    }

    #[test]
    fn disk_klasoru_tabloda_olmasa_da_etiketlenir() {
        let bilinen = slug_etiketi(&sektor_medya::slug("Restoran"));
        assert_eq!(bilinen.ad, "Restoran");
        assert_eq!(bilinen.ingilizce, "restaurant");
        // Tabloda olmayan klasör: dinamik tarama onu da kapsar.
        let yeni = slug_etiketi("yeni_acilan_isletme");
        assert_eq!(yeni.ad, "yeni acilan isletme");
        assert_eq!(yeni.ingilizce, "yeni acilan isletme");
    }

    #[test]
    fn istem_sektor_adi_ve_ingilizce_karsiligini_icerir() {
        let istem = istem_metni(&etiket("Kebapçı", "Lokanta & Kebap"));
        assert!(istem.contains("Kebapçı"));
        assert!(istem.contains("restaurant"));
        assert!(istem.contains("YES"));
        assert!(istem.contains("NO"));
    }

    #[test]
    fn konu_satirli_yanit_cozulur() {
        // Katı kural: skor %100 değilse ya da hiç yoksa onay yok.
        assert_eq!(
            karar_oku("YES"),
            Karar::Belirsiz,
            "skorsuz YES kabul edilmez"
        );
        assert_eq!(karar_oku(" Yes."), Karar::Belirsiz);
        assert_eq!(karar_oku("\nNO"), Karar::Hayir);
        assert_eq!(karar_oku("kuaför\nCONFIDENCE: 90\nYES"), Karar::Belirsiz);
        assert_eq!(karar_oku("kuaför\nCONFIDENCE: 100\nYES"), Karar::Evet);
        assert_eq!(karar_oku("a scanned invoice document\nNO"), Karar::Hayir);
        assert_eq!(karar_oku("maybe"), Karar::Belirsiz);
        assert_eq!(karar_oku(""), Karar::Belirsiz);
        assert_eq!(
            karar_oku("EVET"),
            Karar::Belirsiz,
            "skorsuz EVET kabul edilmez"
        );
        assert_eq!(karar_oku("hayır"), Karar::Hayir);
        assert_eq!(karar_oku("kuaför\nCONFIDENCE: 40\nYES"), Karar::Belirsiz);
        assert_eq!(karar_oku("kuaför\nCONFIDENCE: 95\nYES"), Karar::Belirsiz);
        assert_eq!(karar_oku("kuaför\nconfidence: 89\nYES"), Karar::Belirsiz);
        assert_eq!(esik_degeri(Some(60)), 100, "eşik %90'ın altına inemez");
        assert_eq!(esik_degeri(Some(99)), 100);
        assert_eq!(esik_degeri(None), 100);
    }

    #[test]
    fn sifir_tolerans_tek_hayir_yetmez() {
        assert!(kareler_onayli(&[Karar::Evet, Karar::Evet, Karar::Evet]));
        assert!(!kareler_onayli(&[Karar::Evet, Karar::Hayir, Karar::Evet]));
        assert!(!kareler_onayli(&[
            Karar::Evet,
            Karar::Belirsiz,
            Karar::Evet
        ]));
        assert!(!kareler_onayli(&[]));
    }

    #[test]
    fn kesin_kararda_yedek_modele_gidilmez() {
        let etiket = etiket("Restoran", "Lokanta & Kebap");
        // Birincil EVET: yedek model hiç çağrılmaz.
        let (birincil, cagri1, _) = sayacli(Karar::Evet, None);
        let (yedek, cagri2, _) = sayacli(Karar::Hayir, None);
        let zincir = vec![birincil, yedek];
        let (karar, metinler) = kademeli_karar(&zincir, b"kare", &etiket).unwrap();
        assert_eq!(karar, Karar::Evet);
        assert_eq!(cagri1.load(Ordering::Relaxed), 1);
        assert_eq!(
            cagri2.load(Ordering::Relaxed),
            0,
            "kesin kararda yedek model çağrılmamalı"
        );
        assert!(metinler[0].starts_with("[sahte] "), "{:?}", metinler);

        // Birincil HAYIR: yedek yine çağrılmaz.
        let (birincil, cagri1, _) = sayacli(Karar::Hayir, None);
        let (yedek, cagri2, _) = sayacli(Karar::Evet, None);
        let zincir = vec![birincil, yedek];
        assert_eq!(
            kademeli_karar(&zincir, b"kare", &etiket).unwrap().0,
            Karar::Hayir
        );
        assert_eq!(cagri1.load(Ordering::Relaxed), 1);
        assert_eq!(cagri2.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn supheli_karede_yedek_model_devreye_girer() {
        let etiket = etiket("Restoran", "Lokanta & Kebap");
        // Birincil belirsiz → yedek evet → onay, iki görüş de gerekçede.
        let (birincil, cagri1, _) = sayacli(Karar::Belirsiz, None);
        let (yedek, cagri2, _) = sayacli(Karar::Evet, None);
        let zincir = vec![birincil, yedek];
        let (karar, metinler) = kademeli_karar(&zincir, b"kare", &etiket).unwrap();
        assert_eq!(karar, Karar::Evet);
        assert_eq!(cagri1.load(Ordering::Relaxed), 1);
        assert_eq!(cagri2.load(Ordering::Relaxed), 1);
        assert_eq!(metinler.len(), 2, "iki modelin gerekçesi de yazılmalı");

        // İki model de belirsiz: onay yok, yedek bellekten düşürülür.
        let (birincil, _, _) = sayacli(Karar::Belirsiz, None);
        let (yedek, _, bosaltma2) = sayacli(Karar::Belirsiz, None);
        let zincir = vec![birincil, yedek];
        assert_eq!(
            kademeli_karar(&zincir, b"kare", &etiket).unwrap().0,
            Karar::Belirsiz
        );
        assert_eq!(bosaltma2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn birincil_hata_verirse_yedek_devralir() {
        let etiket = etiket("Restoran", "Lokanta & Kebap");
        let (birincil, _, _) = sayacli(Karar::Evet, Some("model kapalı"));
        let (yedek, _, _) = sayacli(Karar::Evet, None);
        let zincir = vec![birincil, yedek];
        assert_eq!(
            kademeli_karar(&zincir, b"kare", &etiket).unwrap().0,
            Karar::Evet
        );

        // Yedek yoksa hata yayılır: kare doğrulanamaz (fail-closed).
        let (tek, _, _) = sayacli(Karar::Evet, Some("model kapalı"));
        let zincir = vec![tek];
        let hata = kademeli_karar(&zincir, b"kare", &etiket).expect_err("hata yayılmalı");
        assert!(hata.contains("model kapalı"), "{hata}");
    }

    #[test]
    fn vram_olcumu_mb_olarak_cozulur() {
        assert_eq!(vram_metni_coz("5288\n"), Some(5288));
        assert_eq!(vram_metni_coz("3000\r\n400\r\n"), Some(3400));
        assert_eq!(vram_metni_coz("N/A\n"), None);
        assert_eq!(vram_metni_coz(""), None);
        assert_eq!(vram_esigi_mb(), 6000, "6 GB kart için güvenli varsayılan");
    }

    #[test]
    fn video_kare_zamanlari_sinir_icinde() {
        assert_eq!(KARE_ZAMANLARI.len(), KARE_SAYISI);
        assert!(KARE_ZAMANLARI
            .iter()
            .all(|s| *s > 0.0 && *s < crate::medya_arsiv::HERO_MIN_SN));
    }

    #[test]
    fn sonuc_metni_gerekceyi_tasir() {
        let red = Sonuc::Red {
            kare: 3,
            cevaplar: vec![
                "a barber\nYES".into(),
                "a scanned invoice\nNO".into(),
                "a barber\nYES".into(),
            ],
        };
        assert!(red.metin().starts_with("RED"));
        assert!(red.metin().contains("NO"));
        assert!(!matches!(red, Sonuc::Onay { .. }));
        assert!(matches!(
            Sonuc::Onay { kare: 3, skor: 90 },
            Sonuc::Onay { .. }
        ));
        assert!(Sonuc::Yok {
            neden: "model yok".into()
        }
        .metin()
        .contains("doğrulanamadı"));
    }
}
