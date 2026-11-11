//! "2 Medya" için gömülü WebView2 oturumu.
//!
//! Tarayıcı verisi `.gecici/webview2` altında tutulur; böylece kullanıcı
//! üretici sitelerine kendi mailiyle bir kez giriş yapar ve sonraki açılışlarda
//! çerezler korunur. İndirme başladığında hedef, aktif sektörün
//! `media/stock/<sektor>/` klasörüne yönlendirilir; tamamlanan dosya ana
//! uygulamadaki Qwen2.5-VL + FFmpeg onay hattına verilir. WebView2 indirmeyi
//! yönlendiremezse son MP4 dosyası sistemin İndirilenler klasöründen alınır.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Tıklanabilir video üreticileri. Fotoğraf harici araçlarda üretilmez:
/// kareler yerel AI panelinden (RTX 3050) gelir.
pub const VIDEO_URETICILERI: &[(&str, &str)] = &[
    ("Luma Dream Machine", "https://lumalabs.ai/dream-machine"),
    ("Kling AI", "https://klingai.com/"),
    ("Pika.art", "https://pika.art/"),
    ("Runway (Gen-2/3 deneme)", "https://runwayml.com/"),
    ("PixVerse", "https://pixverse.ai/"),
    (
        "Stable Video Diffusion (SVD)",
        "https://stability.ai/stable-video",
    ),
    ("Haiper AI", "https://haiper.ai/"),
];

/// WebView2'den ana uygulamaya dönen olaylar.
#[derive(Debug, Clone)]
pub enum Olay {
    Indirme {
        yol: Option<PathBuf>,
        basarili: bool,
    },
}

/// Açık WebView2 alt penceresi. Alan `Drop` olduğunda WebView kapanır.
pub struct Tarayici {
    webview: wry::WebView,
    son_bounds: Option<(f32, f32, f32, f32)>,
}

impl Tarayici {
    /// Gömülü tarayıcıyı eframe penceresinin alt penceresi olarak açar.
    ///
    /// `indirme` hedefi mutlaka mevcut bir klasör olarak oluşturulur. WebView2
    /// indirme yolu için mutlak yol ister; bu yüzden göreli yollar çalışma
    /// dizinine göre çözülür.
    pub fn yeni(
        frame: &eframe::Frame,
        hedef: &str,
        veri: &Path,
        indirme: &Path,
        tx: Sender<Olay>,
        bounds: wry::Rect,
    ) -> Result<Self, String> {
        let veri = mutlak(veri)?;
        let indirme = mutlak(indirme)?;
        fs::create_dir_all(&veri)
            .map_err(|hata| format!("Tarayıcı veri klasörü oluşturulamadı: {hata}"))?;
        fs::create_dir_all(&indirme)
            .map_err(|hata| format!("İndirme klasörü oluşturulamadı: {hata}"))?;

        // Aynı veri dizini her açılışta kullanılır: WebView2 çerezleri,
        // oturumları ve site izinlerini burada kalıcı tutar.
        let mut baglam = wry::WebContext::new(Some(veri));
        let indirme_klasoru = indirme.clone();
        let webview = wry::WebViewBuilder::new_with_web_context(&mut baglam)
            .with_url(hedef)
            .with_bounds(bounds)
            .with_download_started_handler(move |_url, yol| {
                // Bazı siteler dosya adı vermeden indirme başlatır.
                let ad = yol
                    .file_name()
                    .and_then(|ad| ad.to_str())
                    .filter(|ad| !ad.trim().is_empty())
                    .unwrap_or("indirme");
                let hedef = benzersiz_dosya(&indirme_klasoru, ad);
                *yol = hedef;
                true
            })
            .with_download_completed_handler(move |_url, yol, basarili| {
                let _ = tx.send(Olay::Indirme { yol, basarili });
            })
            .build_as_child(frame)
            .map_err(|hata| {
                format!("WebView2 açılamadı: {hata}. Microsoft Edge WebView2 Runtime kurulu mu?")
            })?;

        Ok(Self {
            webview,
            son_bounds: None,
        })
    }

    /// Pencere yeniden boyutlandığında alt pencereyi aynı dikdörtgene oturtur.
    /// Aynı imza tekrar gelirse WebView2'yi gereksiz yere yeniden konumlamaz.
    pub fn sinirlari_ayarla(
        &mut self,
        bounds: wry::Rect,
        imza: (f32, f32, f32, f32),
    ) -> Result<(), String> {
        if self.son_bounds == Some(imza) {
            return Ok(());
        }
        self.webview
            .set_bounds(bounds)
            .map_err(|hata| format!("Tarayıcı konumu güncellenemedi: {hata}"))?;
        self.son_bounds = Some(imza);
        Ok(())
    }
}

/// Göreli yolu çalışma dizinine göre mutlak yola çevirir.
fn mutlak(yol: &Path) -> Result<PathBuf, String> {
    if yol.is_absolute() {
        return Ok(yol.to_path_buf());
    }
    std::env::current_dir()
        .map(|dizin| dizin.join(yol))
        .map_err(|hata| format!("Çalışma dizini okunamadı: {hata}"))
}

/// Aktif sektörün doğrudan indirme klasörü: `media/stock/<slug>/`.
pub fn indirme_klasoru(slug: &str) -> PathBuf {
    Path::new(crate::sektor_medya::KLASOR).join(slug)
}

/// Dosya zaten hedef sektör klasöründe mi? WebView2 mutlak, uygulama ise
/// göreli yol verebildiği için karşılaştırma mutlak yollar üzerinden yapılır.
pub fn hedef_klasorde(yol: &Path, hedef: &Path) -> bool {
    let (Ok(ust), Ok(hedef)) = (
        yol.parent()
            .ok_or_else(|| "üst klasör yok".to_string())
            .and_then(mutlak),
        mutlak(hedef),
    ) else {
        return false;
    };
    ust == hedef
}

/// Dosya proje içindeki `media/stock` ağacında mı?
pub fn stok_altinda(yol: &Path) -> bool {
    let (Ok(yol), Ok(kok)) = (mutlak(yol), mutlak(Path::new(crate::sektor_medya::KLASOR))) else {
        return false;
    };
    yol.starts_with(kok)
}

/// Sistemin İndirilenler klasörleri. Windows'ta profil ve OneDrive
/// yönlendirmeleri birlikte denenir; olmayan adaylar sessizce atlanır.
pub fn sistem_indirme_klasorleri() -> Vec<PathBuf> {
    let mut adaylar = Vec::new();
    if let Some(profil) = std::env::var_os("USERPROFILE") {
        let profil = PathBuf::from(profil);
        for ad in ["Downloads", "İndirilenler", "Indirilenler"] {
            adaylar.push(profil.join(ad));
        }
        // OneDrive bazen "OneDrive - Şirket" adıyla ayrı bir profil klasörü
        // kullanır; altındaki Downloads da gerçek indirme hedefi olabilir.
        if let Ok(girisler) = fs::read_dir(&profil) {
            for giris in girisler.flatten() {
                let ad = giris.file_name();
                if ad
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .starts_with("onedrive")
                {
                    adaylar.push(giris.path().join("Downloads"));
                }
            }
        }
    }
    if let Some(one_drive) = std::env::var_os("OneDrive") {
        adaylar.push(PathBuf::from(one_drive).join("Downloads"));
    }
    let mut gorulen = HashSet::new();
    adaylar.retain(|yol| yol.is_dir() && gorulen.insert(yol.clone()));
    adaylar
}

/// İndirilenler klasörüne sonradan düşen, henüz işlenmemiş en yeni videoyu
/// bulur. Yalnız `baslangic` sonrasında değişen dosyalar dikkate alınır;
/// böylece tarayıcı açılmadan önceki eski indirmeler yanlışlıkla alınmaz.
pub fn son_indirilen_video(baslangic: SystemTime, haric: &HashSet<PathBuf>) -> Option<PathBuf> {
    son_video_klasorlerde(&sistem_indirme_klasorleri(), baslangic, haric)
}

fn son_video_klasorlerde(
    klasorler: &[PathBuf],
    baslangic: SystemTime,
    haric: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let alt = baslangic.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    let mut adaylar: Vec<(u128, PathBuf)> = Vec::new();
    for klasor in klasorler {
        let Ok(girisler) = fs::read_dir(klasor) else {
            continue;
        };
        for giris in girisler.flatten() {
            let yol = giris.path();
            if !yol.is_file() || haric.contains(&yol) {
                continue;
            }
            let uzanti = yol
                .extension()
                .and_then(|uzanti| uzanti.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(uzanti.as_str(), "mp4" | "mov" | "webm") {
                continue;
            }
            let Ok(meta) = giris.metadata() else {
                continue;
            };
            let Ok(degisti) = meta.modified() else {
                continue;
            };
            let zaman = degisti.duration_since(UNIX_EPOCH).ok()?.as_nanos();
            // Dosya hâlâ yazılıyor olabilir; son 700 ms içinde değişen adayı
            // bir sonraki taramaya bırak.
            let yas = SystemTime::now()
                .duration_since(degisti)
                .unwrap_or_default();
            if zaman >= alt && yas >= Duration::from_millis(700) {
                adaylar.push((zaman, yol));
            }
        }
    }
    adaylar.sort_by_key(|(saniye, _)| *saniye);
    adaylar.pop().map(|(_, yol)| yol)
}

/// Dosyayı hedef klasöre taşır; farklı disk birimlerinde kopyalayıp kaynağı
/// siler. Aynı ada sahip dosya varsa `-2`, `-3` ekiyle çakışma çözülür.
pub fn hedefe_tasi(kaynak: &Path, hedef_klasor: &Path) -> Result<PathBuf, String> {
    if hedef_klasorde(kaynak, hedef_klasor) {
        return Ok(kaynak.to_path_buf());
    }
    fs::create_dir_all(hedef_klasor)
        .map_err(|hata| format!("İndirme klasörü oluşturulamadı: {hata}"))?;
    let ad = kaynak
        .file_name()
        .and_then(|ad| ad.to_str())
        .unwrap_or("indirme.mp4");
    let hedef = benzersiz_dosya(hedef_klasor, ad);
    if fs::rename(kaynak, &hedef).is_ok() {
        return Ok(hedef);
    }
    fs::copy(kaynak, &hedef).map_err(|hata| {
        format!(
            "{} → {} taşınamadı: {hata}",
            kaynak.display(),
            hedef.display()
        )
    })?;
    fs::remove_file(kaynak).map_err(|hata| {
        format!(
            "{} kopyalandı ancak kaynak silinemedi: {hata}",
            kaynak.display()
        )
    })?;
    Ok(hedef)
}

/// Aynı adlı iki indirme birbirini ezmesin: `ad-2.ext`, `ad-3.ext`…
fn benzersiz_dosya(klasor: &Path, ad: &str) -> PathBuf {
    let ad_yolu = Path::new(ad);
    let govde = ad_yolu
        .file_stem()
        .and_then(|govde| govde.to_str())
        .unwrap_or("indirme");
    let uzanti = ad_yolu.extension().and_then(|uzanti| uzanti.to_str());
    let mut aday = klasor.join(ad);
    let mut sira = 2u32;
    while aday.exists() {
        let yeni = match uzanti {
            Some(uzanti) if !uzanti.is_empty() => format!("{govde}-{sira}.{uzanti}"),
            _ => format!("{govde}-{sira}"),
        };
        aday = klasor.join(yeni);
        sira = sira.saturating_add(1);
    }
    aday
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn sektor_indirme_klasoru_dogrudan_stok_altinda() {
        assert_eq!(
            indirme_klasoru("cekici"),
            PathBuf::from("media").join("stock").join("cekici")
        );
    }

    #[test]
    fn mutlak_ve_goreli_hedef_ayni_klasoru_gosterir() {
        let hedef = Path::new("media/stock/cekici");
        let mutlak = std::env::current_dir()
            .unwrap()
            .join(hedef)
            .join("klip.mp4");
        assert!(hedef_klasorde(&mutlak, hedef));
        assert!(!hedef_klasorde(&mutlak, Path::new("media/stock/baska")));
        assert!(stok_altinda(&mutlak));
    }

    #[test]
    fn hedefe_tasi_dosyayi_sektor_klasorune_tasir() {
        let kok = std::env::temp_dir().join(format!("nexus_tasima_test_{}", std::process::id()));
        let kaynak_klasor = kok.join("indirilenler");
        let hedef_klasor = kok.join("media").join("stock").join("cekici");
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(&kaynak_klasor).unwrap();
        let kaynak = kaynak_klasor.join("klip.mp4");
        fs::write(&kaynak, [7u8; 16]).unwrap();
        let hedef = hedefe_tasi(&kaynak, &hedef_klasor).unwrap();
        assert_eq!(hedef, hedef_klasor.join("klip.mp4"));
        assert!(hedef.is_file());
        assert!(!kaynak.exists());
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn son_indirilen_video_yalniz_yeni_dosyayi_secer() {
        let kok = std::env::temp_dir().join(format!("nexus_tarayici_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(&kok).unwrap();
        let baslangic = SystemTime::now() - Duration::from_secs(5);
        fs::write(kok.join("eski.mp4"), [0u8; 8]).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        fs::write(kok.join("yeni.mp4"), [1u8; 8]).unwrap();
        // Tarayıcı yarım dosyayı almasın diye yaş eşiği 700 ms'dir.
        std::thread::sleep(Duration::from_millis(750));
        let bulunan = son_video_klasorlerde(std::slice::from_ref(&kok), baslangic, &HashSet::new());
        assert!(bulunan.is_some_and(|yol| yol.ends_with("yeni.mp4")));
        let _ = fs::remove_dir_all(&kok);
    }
}
