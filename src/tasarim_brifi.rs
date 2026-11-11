//! AI tasarım brifi.
//!
//! Sohbet ortamındaki ajanın ürettiği `tasarim_brifi.json`, demo motorunun
//! sabit varsayılanlarını ezmek için kullanılır. Dosya yoksa veya bozuksa
//! üretim durmaz; motor sektörün kendi tasarım kimliğine döner.
//!
//! Brif bilinçli olarak **veri**dir, kod değildir: yalnız palet, tipografi,
//! hareket, efekt anahtarları ve beş bölümün metinleri okunur. Renkler hex
//! olarak doğrulanır, fontlar CSS enjeksiyonuna karşı süzülür, metinler
//! sınırlanır. Böylece AI ajanı tasarımı yönlendirir; sayfaya rastgele
//! HTML/CSS/JS sokamaz.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

/// Brif dosyası (çalışma dizinine göre).
pub const BRIF_YOLU: &str = "tasarim_brifi.json";
/// Ajans kompozisyonu beş bölümden oluşur.
pub const AZAMI_BOLUM: usize = 5;
/// Bir bölümde en fazla altı hizmet/kalem gösterilir.
pub const AZAMI_KALEM: usize = 6;
const AZAMI_BASLIK: usize = 120;
const AZAMI_METIN: usize = 720;
const AZAMI_KALEM_METIN: usize = 260;

/// Aynı anda birden fazla demo üretilirse brif yazma/okuma yarışı olmasın.
static BRIF_KILIDI: Mutex<()> = Mutex::new(());

/// Brifteki renkler. Boş veya geçersiz alan sektör paletine düşer.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Palet {
    #[serde(default)]
    pub vurgu: String,
    #[serde(default)]
    pub vurgu2: String,
    #[serde(default)]
    pub zemin: String,
    #[serde(default)]
    pub yazi: String,
}

/// Gömülü font aileleri. Yalnız güvenli font karakterleri kabul edilir.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Tipografi {
    #[serde(default)]
    pub baslik: String,
    #[serde(default)]
    pub govde: String,
}

/// Awwwards tarzı efekt anahtarları. Eksik alan açık kabul edilir.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Efektler {
    #[serde(default = "evet")]
    pub kenburns: bool,
    #[serde(default = "evet")]
    pub glass: bool,
    #[serde(default = "evet")]
    pub parallax: bool,
    #[serde(default = "evet")]
    pub magnetic: bool,
    #[serde(default = "evet")]
    pub grain: bool,
    #[serde(default = "evet")]
    pub imlec: bool,
    /// Örn. "kenburns+sinema"; JSON'da açıkça görünür.
    #[serde(default = "sinematik_stili")]
    pub sinematik: String,
    /// Örn. "glassmorphism".
    #[serde(default = "cam_stili")]
    pub cam: String,
    /// Örn. "scroll-parallax".
    #[serde(default = "kaydirma_stili")]
    pub kaydirma: String,
    /// Örn. "staggered fade-in".
    #[serde(default = "giris_stili")]
    pub giris: String,
    /// Kademeli girişte kare başına gecikme (ms).
    #[serde(default = "stagger_varsayilan")]
    pub stagger_ms: u32,
}

fn sinematik_stili() -> String {
    "kenburns+sinema".to_string()
}

fn cam_stili() -> String {
    "glassmorphism".to_string()
}

fn kaydirma_stili() -> String {
    "scroll-parallax".to_string()
}

fn giris_stili() -> String {
    "staggered fade-in".to_string()
}

fn stagger_varsayilan() -> u32 {
    80
}

fn stil_varsayilan() -> String {
    "luxe-sinema".to_string()
}

/// Sektör grubundan görsel rejim seçer.
pub fn stil_adi(grup: &str, acil: bool) -> &'static str {
    if acil {
        "neon-acil"
    } else {
        match grup {
            "mutfak" | "konaklama" | "guzellik" | "etkinlik" => "luxe-sinema",
            "saglik" | "kurumsal" | "egitim" => "temiz-cam",
            "servis" | "spor" | "acil" => "atletik-teknik",
            _ => "luxe-sinema",
        }
    }
}

fn evet() -> bool {
    true
}

impl Default for Efektler {
    fn default() -> Self {
        Self {
            kenburns: true,
            glass: true,
            parallax: true,
            magnetic: true,
            grain: true,
            imlec: true,
            sinematik: sinematik_stili(),
            cam: cam_stili(),
            kaydirma: kaydirma_stili(),
            giris: giris_stili(),
            stagger_ms: stagger_varsayilan(),
        }
    }
}

/// Bir hizmet/yetkinlik satırı.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Kalem {
    #[serde(default)]
    pub ad: String,
    #[serde(default)]
    pub aciklama: String,
}

/// Ajans akışındaki tek bölüm.
///
/// Sıra: 0 hero, 1 hakkımızda/manifesto, 2 hizmetler,
/// 3 AI asistan / acil WhatsApp, 4 iletişim.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Bolum {
    #[serde(default)]
    pub etiket: String,
    #[serde(default)]
    pub baslik: String,
    #[serde(default)]
    pub metin: String,
    #[serde(default)]
    pub kalemler: Vec<Kalem>,
}

/// AI ajanının yazdığı tasarım sözleşmesi.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Brif {
    /// "ajans" (varsayılan) veya "klasik". `demo_profil` ile birlikte çalışır.
    #[serde(default)]
    pub profil: String,
    /// Görsel rejim: neon-acil, luxe-sinema, temiz-cam, atletik-teknik.
    #[serde(default = "stil_varsayilan")]
    pub stil: String,
    #[serde(default)]
    pub palet: Palet,
    #[serde(default)]
    pub tipografi: Tipografi,
    /// "canli" · "dengeli" · "sakin".
    #[serde(default)]
    pub hareket: String,
    #[serde(default)]
    pub efektler: Efektler,
    #[serde(default)]
    pub bolumler: Vec<Bolum>,
    #[serde(default)]
    pub cta: String,
}

/// Otomatik brif üretimi için sektör ve işletme bağlamı.
#[derive(Clone, Copy, Debug)]
pub struct BrifGirdi<'a> {
    pub ad: &'a str,
    pub sektor: &'a str,
    pub adres: &'a str,
    pub vurgu: &'a str,
    pub vurgu2: &'a str,
    pub zemin: &'a str,
    pub yazi: &'a str,
    pub baslik: &'a str,
    pub govde: &'a str,
    pub cta: &'a str,
    pub grup: &'a str,
    pub hareket: &'a str,
    pub acil: bool,
    pub telefon: Option<&'a str>,
    pub saat: &'a str,
}

impl Brif {
    /// `tasarim_brifi.json` varsa okur; yoksa/bozuksa `None` döner.
    pub fn yukle() -> Option<Self> {
        Self::yukle_yoldan(Path::new(BRIF_YOLU))
    }

    /// Testlerin ve araçların kullanabileceği adresli yükleme.
    pub fn yukle_yoldan(yol: &Path) -> Option<Self> {
        let metin = fs::read_to_string(yol).ok()?;
        let mut brif: Brif = serde_json::from_str(&metin).ok()?;
        brif.temizle();
        Some(brif)
    }

    /// Belirtilen sıradaki bölüm (0-4).
    pub fn bolum(&self, sira: usize) -> Option<&Bolum> {
        self.bolumler.get(sira)
    }

    /// Renkleri, fontları ve metinleri güvenli sınırlara çeker.
    fn temizle(&mut self) {
        self.profil = match self.profil.trim().to_ascii_lowercase().as_str() {
            "klasik" => "klasik".to_string(),
            _ => "ajans".to_string(),
        };
        self.stil = match self.stil.trim().to_ascii_lowercase().as_str() {
            "neon-acil" | "luxe-sinema" | "temiz-cam" | "atletik-teknik" => {
                self.stil.trim().to_ascii_lowercase()
            }
            _ => stil_varsayilan(),
        };
        self.hareket = match self.hareket.trim().to_ascii_lowercase().as_str() {
            "canli" | "dengeli" | "sakin" => self.hareket.trim().to_ascii_lowercase(),
            _ => String::new(),
        };
        self.palet = Palet {
            vurgu: gecerli_renk(&self.palet.vurgu).unwrap_or_default(),
            vurgu2: gecerli_renk(&self.palet.vurgu2).unwrap_or_default(),
            zemin: gecerli_renk(&self.palet.zemin).unwrap_or_default(),
            yazi: gecerli_renk(&self.palet.yazi).unwrap_or_default(),
        };
        self.tipografi.baslik = temiz_font(&self.tipografi.baslik);
        self.tipografi.govde = temiz_font(&self.tipografi.govde);
        self.cta = temiz_metin(&self.cta, 80);
        self.bolumler
            .retain(|bolum| !bolum.baslik.trim().is_empty() || !bolum.metin.trim().is_empty());
        self.bolumler.truncate(AZAMI_BOLUM);
        for bolum in &mut self.bolumler {
            bolum.etiket = temiz_metin(&bolum.etiket, 60);
            bolum.baslik = temiz_metin(&bolum.baslik, AZAMI_BASLIK);
            bolum.metin = temiz_metin(&bolum.metin, AZAMI_METIN);
            bolum
                .kalemler
                .retain(|kalem| !kalem.ad.trim().is_empty() || !kalem.aciklama.trim().is_empty());
            bolum.kalemler.truncate(AZAMI_KALEM);
            for kalem in &mut bolum.kalemler {
                kalem.ad = temiz_metin(&kalem.ad, 80);
                kalem.aciklama = temiz_metin(&kalem.aciklama, AZAMI_KALEM_METIN);
            }
        }
    }

    /// Brifi proje kökündeki sözleşme dosyasına yazar.
    pub fn yaz(&self, yol: &Path) -> Result<(), String> {
        let metin = serde_json::to_string_pretty(self)
            .map_err(|hata| format!("brif JSON'a çevrilemedi: {hata}"))?;
        fs::write(yol, format!("{metin}\n"))
            .map_err(|hata| format!("{} yazılamadı: {hata}", yol.display()))
    }

    /// Önce dosyayı yazar, sonra motorun kullanacağı temizlenmiş brifi okur.
    /// Yazma başarısız olsa bile bellek içindeki brif ile üretim sürer.
    pub fn yaz_ve_oku(brif: &Brif) -> Option<Brif> {
        let _kilit = BRIF_KILIDI.lock().ok()?;
        if brif.yaz(Path::new(BRIF_YOLU)).is_err() {
            return Some(brif.clone());
        }
        Brif::yukle().or_else(|| Some(brif.clone()))
    }

    /// Sektör paletini, gömülü tipografiyi ve beş zengin bölümü tek brifte
    /// birleştirir. Amaç: her üretimde aynı lüks sinematik sözleşmeyi kurmak.
    pub fn awwwards(g: &BrifGirdi<'_>) -> Self {
        let ad = temiz_metin(g.ad, 90);
        let sektor = temiz_metin(g.sektor, 70);
        let adres = temiz_metin(g.adres, 180);
        let bolge = bolge_metni(g.adres);
        let saat = if g.saat.trim().is_empty() {
            "Randevu ile"
        } else {
            g.saat.trim()
        };
        let telefon = g
            .telefon
            .map(str::trim)
            .filter(|telefon| !telefon.is_empty())
            .unwrap_or("telefon bilgisi yakında");
        let cta = if !g.cta.trim().is_empty() {
            g.cta.trim().to_string()
        } else if g.acil {
            "Acil WhatsApp".to_string()
        } else {
            "Bize ulaşın".to_string()
        };
        let hizmetler = hizmet_kalemleri(g.grup);
        let (ai_etiket, ai_baslik, ai_metin) = if g.acil {
            (
                "AI Asistan Widget / Acil WhatsApp",
                "Yolda kaldığınızda\nbir mesaj kadar yakın.",
                format!(
                    "{ad} için hazırlanan demo asistanı; acil durumu anlar, doğru hizmeti önerir ve canlı WhatsApp hattına yönlendirir. Konum paylaşımı tek dokunuşla iletilir."
                ),
            )
        } else {
            (
                "AI Asistan Widget / WhatsApp",
                "Sorunuzu yazın,\ndoğru adıma yönlendirelim.",
                format!(
                    "{ad} için tasarlanan demo asistanı; hizmetleri, çalışma saatlerini ve iletişim adımlarını sade bir dille yanıtlar. Devam etmek isterseniz WhatsApp hattı hazır."
                ),
            )
        };
        let ai_kalemleri = vec![
            Kalem {
                ad: "7/24 AI asistan".to_string(),
                aciklama: "Soruları anlar, doğru sonraki adımı önerir.".to_string(),
            },
            Kalem {
                ad: "Canlı WhatsApp".to_string(),
                aciklama: "Konuşmayı doğrudan ekibe bağlar.".to_string(),
            },
            Kalem {
                ad: "Konum paylaşımı".to_string(),
                aciklama: "Acil durumda tek dokunuşla iletilir.".to_string(),
            },
        ];
        Self {
            profil: "ajans".to_string(),
            stil: stil_adi(g.grup, g.acil).to_string(),
            palet: Palet {
                vurgu: g.vurgu.to_string(),
                vurgu2: g.vurgu2.to_string(),
                zemin: g.zemin.to_string(),
                yazi: g.yazi.to_string(),
            },
            tipografi: Tipografi {
                baslik: g.baslik.to_string(),
                govde: g.govde.to_string(),
            },
            hareket: if g.hareket.trim().is_empty() {
                "canli".to_string()
            } else {
                g.hareket.to_string()
            },
            efektler: Efektler {
                kenburns: true,
                glass: true,
                parallax: true,
                magnetic: true,
                grain: true,
                imlec: true,
                sinematik: "kenburns+sinema".to_string(),
                cam: "glassmorphism".to_string(),
                kaydirma: "scroll-parallax".to_string(),
                giris: "staggered fade-in".to_string(),
                stagger_ms: if g.acil { 55 } else { 80 },
            },
            bolumler: vec![
                Bolum {
                    etiket: format!("{sektor} · {bolge}"),
                    baslik: format!("{sektor} deneyimini\nsinematik bir dile taşıyoruz."),
                    metin: format!(
                        "{ad}, {bolge} merkezli {sektor} deneyimini sakin bir lüks, net bir ritim ve insan odaklı bir hikâyeyle yeniden yorumluyor."
                    ),
                    kalemler: Vec::new(),
                },
                Bolum {
                    etiket: "Hakkımızda".to_string(),
                    baslik: "Bir mekân değil,\nhatırlanan bir ritüel.".to_string(),
                    metin: format!(
                        "{ad} için kurduğumuz bu anlatı, ilk bakışta değil; ilk temasta kendini gösteren bir özen duygusu taşıyor. Her ayrıntı, markanın karakterini aynı sakinlikle anlatıyor."
                    ),
                    kalemler: Vec::new(),
                },
                Bolum {
                    etiket: "Hizmetler".to_string(),
                    baslik: "Her ayrıntı,\naynı imzayı taşır.".to_string(),
                    metin: format!(
                        "{sektor} deneyimini oluşturan adımlar; şeffaf kapsam, ölçülü estetik ve tutarlı bir sonuç için tasarlandı."
                    ),
                    kalemler: hizmetler
                        .into_iter()
                        .map(|(ad, aciklama)| Kalem {
                            ad: ad.to_string(),
                            aciklama: aciklama.to_string(),
                        })
                        .collect(),
                },
                Bolum {
                    etiket: ai_etiket.to_string(),
                    baslik: ai_baslik.to_string(),
                    metin: ai_metin,
                    kalemler: ai_kalemleri,
                },
                Bolum {
                    etiket: "İletişim".to_string(),
                    baslik: "Bir sonraki adım,\nsizinle başlıyor.".to_string(),
                    metin: format!(
                        "{ad} · {bolge}. Telefon: {telefon}. Çalışma saatleri: {saat}. Adres: {adres}."
                    ),
                    kalemler: Vec::new(),
                },
            ],
            cta,
        }
    }
}

/// Yerel Ollama sunucusu: ücretsiz, anahtarsız, veri dışarı çıkmaz.
pub const OLLAMA_URL: &str = "http://127.0.0.1:11434";

/// Yerel model bağlantısı. Model adı boşsa sunucudaki ilk model kullanılır.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct YerelModel {
    pub url: String,
    pub model: String,
}

impl YerelModel {
    /// `OLLAMA_URL` / `OLLAMA_MODEL` ortam değişkenleriyle, yoksa yerel
    /// varsayıan adresle yapılandırılır.
    pub fn varsayilan() -> Self {
        let url = std::env::var("OLLAMA_URL")
            .ok()
            .map(|deger| deger.trim().trim_end_matches('/').to_string())
            .filter(|deger| !deger.is_empty())
            .unwrap_or_else(|| OLLAMA_URL.to_string());
        let model = std::env::var("OLLAMA_MODEL")
            .ok()
            .map(|deger| deger.trim().to_string())
            .filter(|deger| !deger.is_empty())
            .unwrap_or_default();
        Self { url, model }
    }

    /// Sunucudaki ilk modeli bulur; sunucu kapalıysa `None` (1,5 sn sınır).
    pub fn kesfet(&mut self) -> Option<String> {
        let ajan = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(1500))
            .build();
        let metin = ajan
            .get(&format!("{}/api/tags", self.url))
            .call()
            .ok()?
            .into_string()
            .ok()?;
        let deger: serde_json::Value = serde_json::from_str(&metin).ok()?;
        let model = deger
            .get("models")?
            .as_array()?
            .first()?
            .get("name")?
            .as_str()?
            .to_string();
        self.model = model.clone();
        Some(model)
    }

    /// Brifi yerel modelden ister. Hata/boş yanıt durumunda çağıran taraf
    /// sektör brifine döner; üretim bu çağrıya bağımlı değildir.
    pub fn brif(&self, istem: &str) -> Result<Brif, String> {
        let mut model = self.model.clone();
        if model.is_empty() {
            let mut kesif = self.clone();
            model = kesif
                .kesfet()
                .ok_or_else(|| "yerel model bulunamadı (ollama serve)".to_string())?;
        }
        let govde = serde_json::json!({
            "model": model,
            "prompt": istem,
            "stream": false,
            "format": brif_semasi(),
            "options": { "temperature": 0.4 },
        });
        let ajan = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(90))
            .build();
        let yanit = ajan
            .post(&format!("{}/api/generate", self.url))
            .set("Content-Type", "application/json")
            .send_string(&govde.to_string())
            .map_err(|hata| format!("yerel model çağrısı başarısız: {hata}"))?;
        let metin = yanit.into_string().unwrap_or_default();
        let deger: serde_json::Value = serde_json::from_str(&metin)
            .map_err(|hata| format!("model yanıtı JSON değil: {hata}"))?;
        let ham = deger
            .get("response")
            .and_then(|alan| alan.as_str())
            .ok_or_else(|| "model boş yanıt döndürdü".to_string())?;
        brif_metni_coz(ham)
    }
}

impl Brif {
    /// Yerel model varsa ondan brif alır; yoksa veya JSON bozuksa sektör
    /// brifine döner. Üretim yolu bu fonksiyon olmadan da çalışır.
    pub fn yerel_veya_awwwards(g: &BrifGirdi<'_>) -> Self {
        let istem = yerel_istem(g);
        YerelModel::varsayilan()
            .brif(&istem)
            .unwrap_or_else(|_| Self::awwwards(g))
    }
}

/// Modelin serbest yanıtındaki JSON'u brife çevirir: markdown çitleri ve
/// baştaki/sondaki açıklama metni tolere edilir, sonra `temizle` süzgeci.
pub fn brif_metni_coz(metin: &str) -> Result<Brif, String> {
    let metin = metin.trim();
    let bas = metin
        .find('{')
        .ok_or_else(|| "yanıtta JSON nesnesi yok".to_string())?;
    let son = metin
        .rfind('}')
        .ok_or_else(|| "JSON nesnesi kapanmamış".to_string())?;
    if son <= bas {
        return Err("JSON nesnesi boş".to_string());
    }
    let mut brif: Brif = serde_json::from_str(&metin[bas..=son])
        .map_err(|hata| format!("brif JSON çözülemedi: {hata}"))?;
    brif.temizle();
    if brif.bolumler.is_empty() {
        return Err("brifte bölüm yok".to_string());
    }
    Ok(brif)
}

/// Ollama'nın yapılandırılmış çıktı için beklediği JSON şeması.
/// Model şemayı desteklemiyorsa istek yine çalışır; çözümleme süzgeci
/// bozuk yanıtı reddedip sektör brifine düşer.
fn brif_semasi() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "profil": {"type": "string"},
            "stil": {
                "type": "string",
                "enum": ["neon-acil", "luxe-sinema", "temiz-cam", "atletik-teknik"]
            },
            "palet": {
                "type": "object",
                "properties": {
                    "vurgu": {"type": "string"},
                    "vurgu2": {"type": "string"},
                    "zemin": {"type": "string"},
                    "yazi": {"type": "string"}
                },
                "required": ["vurgu", "vurgu2", "zemin", "yazi"]
            },
            "tipografi": {
                "type": "object",
                "properties": {
                    "baslik": {"type": "string"},
                    "govde": {"type": "string"}
                }
            },
            "hareket": {"type": "string"},
            "efektler": {"type": "object"},
            "bolumler": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "etiket": {"type": "string"},
                        "baslik": {"type": "string"},
                        "metin": {"type": "string"},
                        "kalemler": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "ad": {"type": "string"},
                                    "aciklama": {"type": "string"}
                                }
                            }
                        }
                    },
                    "required": ["baslik", "metin"]
                }
            },
            "cta": {"type": "string"}
        },
        "required": ["stil", "palet", "bolumler", "cta"]
    })
}

/// Yerel modele verilen istem: şema, sınırlar ve dürüstlük kuralları.
pub fn yerel_istem(g: &BrifGirdi<'_>) -> String {
    let sema = r###"{"profil":"ajans","stil":"neon-acil|luxe-sinema|temiz-cam|atletik-teknik","palet":{"vurgu":"#rrggbb","vurgu2":"#rrggbb","zemin":"#rrggbb","yazi":"#rrggbb"},"tipografi":{"baslik":"'Instrument Serif',serif","govde":"'Sora',sans-serif"},"hareket":"canli|dengeli|sakin","efektler":{"kenburns":true,"glass":true,"parallax":true,"magnetic":true,"grain":true,"imlec":true,"sinematik":"kenburns+sinema","cam":"glassmorphism","kaydirma":"scroll-parallax","giris":"staggered fade-in","stagger_ms":80},"bolumler":[{"etiket":"...","baslik":"...","metin":"...","kalemler":[{"ad":"...","aciklama":"..."}]}],"cta":"..."}"###;
    // Ham dizede kaçış kolaylığı için yazılan ters bölüler istemden atılır.
    let sema = sema.replace('\\', "");
    format!(
        "Sen kıdemli bir web tasarım direktörüsün. {ad} ({sektor}, {adres}) için Türkçe, \
         lüks ve sinematik bir demo sitesi brifi yaz.\n\
         Yalnızca tek bir JSON nesnesi döndür; açıklama, markdown çiti veya ek metin yazma.\n\
         Şema: {sema}\n\
         Kurallar: tam 5 bölüm (hero, manifesto, hizmetler, AI asistan, iletişim sırasıyla); \
         güven bölümü motorda otomatik üretilir, onu yazma; renkler #rrggbb biçiminde; \
         fontlar yalnızca 'Instrument Serif', 'Cormorant Garamond', 'Space Grotesk', 'Sora', \
         'Plus Jakarta Sans' veya 'Geist' olabilir; uydurma fiyat, indirim, ödül veya \
         istatistik yazma; başlıklar en fazla 6 kelime, metinler en fazla 2 cümle olsun.",
        ad = g.ad,
        sektor = g.sektor,
        adres = g.adres,
        sema = sema
    )
}

fn bolge_metni(adres: &str) -> String {
    adres
        .rsplit([',', '/'])
        .map(str::trim)
        .find(|parca| !parca.is_empty())
        .map(|parca| {
            parca
                .split_whitespace()
                .filter(|kelime| !kelime.chars().all(|harf| harf.is_ascii_digit()))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|parca| !parca.is_empty())
        .unwrap_or_else(|| "Türkiye".to_string())
}

fn hizmet_kalemleri(grup: &str) -> Vec<(&'static str, &'static str)> {
    match grup {
        "acil" => vec![
            (
                "7/24 Acil Müdahale",
                "Gece, hafta sonu ve resmî tatil dahil.",
            ),
            ("Canlı Konum", "Konum tek dokunuşla ekibe iletilir."),
            ("Yol Yardım", "Yolda kalma anını güvenli bir plana çevirir."),
            (
                "Güvenli Teslim",
                "Araç ve kullanıcı için net bir teslim ritüeli.",
            ),
        ],
        "saglik" => vec![
            ("Kişiye Özel Plan", "İhtiyaç ve beklenti baştan konuşulur."),
            ("Uzman Değerlendirme", "Süreç, uzman bakışıyla planlanır."),
            (
                "Modern Teknoloji",
                "Konforu ve güveni birlikte artıran ekipman.",
            ),
            ("Düzenli Takip", "Bakım ve iletişim aynı özenle sürer."),
        ],
        "mutfak" => vec![
            (
                "Mevsimlik Menü",
                "Mevsimin en iyi ürünleriyle kurulan denge.",
            ),
            ("Şef İmzası", "Tekrar edilemeyen bir lezzet ve sunum dili."),
            ("Özel Davetler", "Davetler için ölçülü, zarif bir deneyim."),
            ("Seçkin Servis", "Mekânda veya evde aynı imza."),
        ],
        "guzellik" => vec![
            ("Kişisel Analiz", "Her bakım, kişiye göre planlanır."),
            ("Uzman Dokunuşu", "Sonucu belirleyen ince ayrıntılar."),
            ("Premium Ürünler", "Cilde ve ritüele uygun seçimler."),
            ("Randevu Konforu", "Zamanında, sakin ve kesintisiz akış."),
        ],
        "konaklama" => vec![
            ("Butik Konfor", "Her oda, kendine ait bir hikâye taşır."),
            (
                "Kişisel Karşılama",
                "İlk andan itibaren tanıdık bir sıcaklık.",
            ),
            ("Yerel Deneyim", "Şehrin ritmini doğru duraklarda keşfetme."),
            ("Sessiz Ayrıcalık", "Gösterişten uzak, hatırlanan detaylar."),
        ],
        "kurumsal" => vec![
            (
                "Stratejik Bakış",
                "Kararı, gürültüden arındırılmış bir çerçeveye oturtur.",
            ),
            (
                "Veri Odaklı Karar",
                "Hikâyeyi ölçülebilir sinyallerle besler.",
            ),
            ("Şeffaf Süreç", "Her adım, aynı netlikte paylaşılır."),
            (
                "Sürdürülebilir Büyüme",
                "Kısa vadeli parlaklık yerine kalıcı değer.",
            ),
        ],
        "servis" => vec![
            (
                "Kusursuz Teslim",
                "İşin bittiği an da en az başlangıç kadar önemli.",
            ),
            ("Uzman Ekip", "Doğru işi, doğru uzmanlıkla buluşturur."),
            ("Şeffaf Kapsam", "Ne yapılacağı baştan netleşir."),
            ("Zamanında Çözüm", "Söz verilen an, gerçekten gelir."),
        ],
        "spor" => vec![
            (
                "Kişisel Program",
                "Hedefe göre kurulan, sürdürülebilir plan.",
            ),
            ("Uzman Eğitmen", "Doğru form ve güvenli ilerleme."),
            ("Ölçülebilir Gelişim", "Her hafta görünür bir fark."),
            ("Motivasyon", "Devamlılığı kolaylaştıran bir ritim."),
        ],
        "egitim" => vec![
            (
                "Kişiye Özel Yol Haritası",
                "Hedef, seviye ve tempoya göre plan.",
            ),
            ("Deneyimli Eğitmen", "Alanında uzman, net anlatım."),
            ("Ölçülebilir Gelişim", "İlerleme düzenli olarak görünür."),
            ("Sürekli Geri Bildirim", "Doğru zamanda, doğru yönlendirme."),
        ],
        "etkinlik" => vec![
            (
                "Konsept Tasarımı",
                "Etkinliğin karakterini tek bir fikre bağlar.",
            ),
            (
                "Kusursuz Prodüksiyon",
                "Görünmeyen ayrıntıların sessiz disiplini.",
            ),
            ("Zarif Detaylar", "Mekân, ışık ve akışın ortak dili."),
            (
                "Unutulmaz Deneyim",
                "Son konuk ayrıldığında da devam eden his.",
            ),
        ],
        _ => vec![
            (
                "İmza Deneyim",
                "Markaya ait, tekrar edilemeyen bir ilk izlenim.",
            ),
            ("Kişisel Yaklaşım", "İhtiyaç ve beklenti baştan dinlenir."),
            ("Sürdürülebilir Kalite", "Her temasta aynı özen."),
            ("Şeffaf İletişim", "Süreç boyunca net ve sakin iletişim."),
        ],
    }
}

/// `#rgb` / `#rrggbb` dışındaki değerleri reddeder.
fn gecerli_renk(deger: &str) -> Option<String> {
    let deger = deger.trim();
    let govde = deger.strip_prefix('#')?;
    let gecerli = (govde.len() == 3 || govde.len() == 6)
        && govde.chars().all(|harf| harf.is_ascii_hexdigit());
    gecerli.then(|| format!("#{}", govde.to_ascii_lowercase()))
}

/// Font yığınından CSS'i bozabilecek karakterleri ayıklar.
fn temiz_font(deger: &str) -> String {
    deger
        .chars()
        .filter(|harf| harf.is_alphanumeric() || matches!(harf, ' ' | ',' | '\'' | '"' | '-' | '_'))
        .take(160)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Kontrol karakterlerini temizler ve metni karakter sınırında keser.
fn temiz_metin(deger: &str, sinir: usize) -> String {
    deger
        .chars()
        .filter(|harf| !harf.is_control() || matches!(harf, '\n' | '\t'))
        .take(sinir)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::io::Write;

    fn gecici(ad: &str) -> std::path::PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("nexus_brif_{ad}_{}.json", std::process::id()));
        let _ = fs::remove_file(&yol);
        yol
    }

    #[test]
    fn gecerli_brif_okunur_ve_temizlenir() {
        let yol = gecici("gecerli");
        let mut dosya = fs::File::create(&yol).expect("dosya");
        write!(
            dosya,
            r##"{{
                "profil": "AJANS",
                "palet": {{"vurgu":"#F60","vurgu2":"bozuk","zemin":"#080d13","yazi":"#f6f2ea"}},
                "tipografi": {{"baslik":"'Space Grotesk', sans-serif; }}","govde":"Geist"}},
                "hareket": "CANLI",
                "efektler": {{"kenburns": false}},
                "bolumler": [
                    {{"baslik":"Bir başlık","metin":"Metin","kalemler":[{{"ad":"A","aciklama":"B"}}]}},
                    {{"baslik":"İkinci"}},
                    {{"baslik":"Üçüncü"}},
                    {{"baslik":"Dördüncü"}},
                    {{"baslik":"Beşinci"}},
                    {{"baslik":"Altıncı"}}
                ],
                "cta":"Konuşalım"
            }}"##
        )
        .expect("yaz");
        let brif = Brif::yukle_yoldan(&yol).expect("brif okunmalı");
        assert_eq!(brif.profil, "ajans");
        assert_eq!(brif.palet.vurgu, "#f60");
        assert_eq!(brif.palet.vurgu2, "");
        assert_eq!(brif.hareket, "canli");
        assert!(!brif.efektler.kenburns);
        assert!(brif.efektler.glass);
        assert_eq!(brif.bolumler.len(), AZAMI_BOLUM, "altıncı bölüm atılmalı");
        assert!(!brif.tipografi.baslik.contains(';'));
        assert!(!brif.tipografi.baslik.contains('}'));
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn awwwards_brifi_bes_bolum_ve_efekt_sozlesmesi_uretir() {
        let brif = Brif::awwwards(&BrifGirdi {
            ad: "Gece Oto Kurtarma",
            sektor: "Çekici",
            adres: "Paşa Alanı, 10020 Balıkesir",
            vurgu: "#ff7a1a",
            vurgu2: "#ffd166",
            zemin: "#080d13",
            yazi: "#f6f2ea",
            baslik: "'Space Grotesk',sans-serif",
            govde: "'Geist',sans-serif",
            cta: "Hemen ara",
            grup: "acil",
            hareket: "canli",
            acil: true,
            telefon: Some("+90 532 000 00 00"),
            saat: "7/24",
        });
        assert_eq!(brif.bolumler.len(), 5);
        assert_eq!(brif.bolum(1).expect("hakkımızda").etiket, "Hakkımızda");
        assert_eq!(brif.bolum(2).expect("hizmetler").kalemler.len(), 4);
        assert_eq!(
            brif.bolum(3).expect("asistan").etiket,
            "AI Asistan Widget / Acil WhatsApp"
        );
        assert_eq!(brif.bolum(4).expect("iletişim").etiket, "İletişim");
        assert_eq!(brif.efektler.sinematik, "kenburns+sinema");
        assert_eq!(brif.efektler.cam, "glassmorphism");
        assert_eq!(brif.efektler.kaydirma, "scroll-parallax");
        assert_eq!(brif.efektler.giris, "staggered fade-in");
        assert_eq!(brif.efektler.stagger_ms, 55);

        let yol = gecici("awwwards");
        brif.yaz(&yol).expect("brif yazılmalı");
        let okunan = Brif::yukle_yoldan(&yol).expect("yazılan brif okunmalı");
        assert_eq!(okunan.bolumler.len(), 5);
        assert_eq!(okunan.efektler.stagger_ms, 55);
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn bozuk_dosya_none_doner() {
        let yol = gecici("bozuk");
        fs::write(&yol, "{ bozuk").expect("yaz");
        assert!(Brif::yukle_yoldan(&yol).is_none());
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn olmayan_dosya_none_doner() {
        assert!(Brif::yukle_yoldan(Path::new("olmayan_tasarim_brifi.json")).is_none());
    }

    #[test]
    fn model_yanitindan_brif_cozulur() {
        let metin = "İşte brif:\n```json\n{\"stil\":\"temiz-cam\",\"bolumler\":[{\"baslik\":\"Bir başlık\",\"metin\":\"Metin\"}]}\n```";
        let brif = brif_metni_coz(metin).expect("çözülmeli");
        assert_eq!(brif.stil, "temiz-cam");
        assert_eq!(brif.bolumler.len(), 1);
        assert_eq!(brif.profil, "ajans");
    }

    #[test]
    fn bozuk_model_yaniti_hata_doner() {
        assert!(brif_metni_coz("brif yok").is_err());
        assert!(brif_metni_coz("{\"bolumler\":[]}").is_err());
        assert!(brif_metni_coz("{\"bolumler\": [}").is_err());
    }

    #[test]
    fn yerel_istem_semayi_ve_kurallari_tasir() {
        let istem = yerel_istem(&BrifGirdi {
            ad: "Gece Oto Kurtarma",
            sektor: "Çekici",
            adres: "Paşa Alanı, 10020 Balıkesir",
            vurgu: "#ff7a1a",
            vurgu2: "#ffd166",
            zemin: "#080d13",
            yazi: "#f6f2ea",
            baslik: "'Space Grotesk',sans-serif",
            govde: "'Geist',sans-serif",
            cta: "Hemen ara",
            grup: "acil",
            hareket: "canli",
            acil: true,
            telefon: Some("+90 532 000 00 00"),
            saat: "7/24",
        });
        for parca in ["\"stil\"", "5 bölüm", "fiyat", "Gece Oto Kurtarma"] {
            assert!(istem.contains(parca), "istemde eksik: {parca}");
        }
    }

    /// Canlı köprü denemesi: `cargo test --all-targets yerel_model_canli -- --ignored --nocapture`
    /// Ollama kapalıysa veya model yoksa hata yazdırır; üretim yine sektör
    /// brifine düştüğü için bu test bilgilendiricidir, akışı durdurmaz.
    #[test]
    #[ignore]
    fn yerel_model_canli() {
        let girdi = BrifGirdi {
            ad: "Gece Oto Kurtarma",
            sektor: "Çekici",
            adres: "Paşa Alanı, 10020 Balıkesir",
            vurgu: "#ff7a1a",
            vurgu2: "#ffd166",
            zemin: "#080d13",
            yazi: "#f6f2ea",
            baslik: "'Space Grotesk',sans-serif",
            govde: "'Geist',sans-serif",
            cta: "Hemen ara",
            grup: "acil",
            hareket: "canli",
            acil: true,
            telefon: Some("+90 532 000 00 00"),
            saat: "7/24",
        };
        let sonuc = YerelModel::varsayilan().brif(&yerel_istem(&girdi));
        match &sonuc {
            Ok(brif) => println!(
                "canlı brif: stil={} bölüm={} cta={}",
                brif.stil,
                brif.bolumler.len(),
                brif.cta
            ),
            Err(hata) => println!("canlı brif alınamadı (sektör brifine düşülür): {hata}"),
        }
    }

    #[test]
    fn brif_semasi_zorunlu_alanlari_ister() {
        let sema = brif_semasi();
        let zorunlu = sema
            .get("required")
            .and_then(|alan| alan.as_array())
            .expect("zorunlu alanlar");
        assert!(zorunlu.iter().any(|alan| alan == "stil"));
        assert!(zorunlu.iter().any(|alan| alan == "bolumler"));
        assert_eq!(
            sema["properties"]["stil"]["enum"]
                .as_array()
                .expect("stil enum")
                .len(),
            4
        );
    }

    #[test]
    fn kapali_yerel_sunucu_hata_doner() {
        let model = YerelModel {
            url: "http://127.0.0.1:9".to_string(),
            model: String::new(),
        };
        assert!(model.brif("test").is_err());
    }

    #[test]
    fn gecersiz_renkler_bos_qalir() {
        let brif = Brif {
            palet: Palet {
                vurgu: "red".to_string(),
                vurgu2: "#12345".to_string(),
                zemin: "#fff".to_string(),
                yazi: "#1234567".to_string(),
            },
            ..Brif::default()
        };
        // `temizle` özel; aynı doğrulamayı yardımcı üzerinden sına.
        assert_eq!(gecerli_renk(&brif.palet.vurgu), None);
        assert_eq!(gecerli_renk(&brif.palet.vurgu2), None);
        assert_eq!(gecerli_renk(&brif.palet.zemin).as_deref(), Some("#fff"));
        assert_eq!(gecerli_renk(&brif.palet.yazi), None);
    }
}
