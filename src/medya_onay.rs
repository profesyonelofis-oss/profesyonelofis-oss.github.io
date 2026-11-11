//! İnsan onaylı medya havuzu: `media/pending` → onay → `media/stock`.
//!
//! Akış:
//! 1. Kullanıcı panelden doğrudan link yapıştırır veya dosya yükler; medya
//!    Qwen2.5-VL yerel doğrulamasından geçer ve **geçici havuza** yazılır:
//!    `media/pending/<slug>/video|photos/`.
//! 2. Yerel panel (`http://127.0.0.1:8080`) bekleyen medyayı önizler.
//! 3. Kullanıcı [ONAYLA] dediğinde dosya `media/stock/<slug>/` altına taşınır
//!    ve sektörün onay kaydına işlenir; [REDDET] dosyayı diskten siler.
//! 4. Onay kaydı 2 video + 2 fotoğrafa ulaşmadıkça demo üretimi kilitlidir.
//!
//! Kota sabittir: [`VIDEO_KOTA`] = 2, [`FOTO_KOTA`] = 2.

use std::fs;
use std::path::{Path, PathBuf};

/// Onay kayıt dosyası (proje kökünde).
pub const DOSYA: &str = "media_onay.json";
/// Onay bekleyen geçici medya havuzu.
pub const BEKLEYEN: &str = "media/pending";
/// Sektör başına onaylanacak video sayısı.
pub const VIDEO_KOTA: usize = 2;
/// Sektör başına onaylanacak fotoğraf sayısı.
pub const FOTO_KOTA: usize = 2;

/// Bir sektörün onay durumu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Durum {
    /// Kayıt yok: sektör için hiç medya onaylanmadı.
    #[default]
    Yok,
    /// Onay bekleyen medya var ya da kota henüz dolmadı.
    Bekliyor,
    /// Kota doldu; demo üretimi açık.
    Onaylandi,
    /// Kullanıcı medyayı reddetti; dosyalar silindi.
    Reddedildi,
}

impl Durum {
    /// Panel/CLI için kısa etiket.
    pub fn etiket(self) -> &'static str {
        match self {
            Self::Yok => "medya yok",
            Self::Bekliyor => "onay bekliyor",
            Self::Onaylandi => "onaylandı",
            Self::Reddedildi => "reddedildi",
        }
    }

    fn metin(self) -> &'static str {
        match self {
            Self::Yok => "yok",
            Self::Bekliyor => "bekliyor",
            Self::Onaylandi => "onaylandi",
            Self::Reddedildi => "reddedildi",
        }
    }

    fn metinden(metin: &str) -> Self {
        match metin {
            "bekliyor" => Self::Bekliyor,
            "onaylandi" => Self::Onaylandi,
            "reddedildi" => Self::Reddedildi,
            _ => Self::Yok,
        }
    }
}

/// Tek sektörün onay kaydı: yalnız **onaylanmış** dosyalar. Yollar köke göre
/// ve `/` ayraçlıdır (`media/stock/kuafor/photos/1.jpg`).
#[derive(Clone, Debug, Default)]
pub struct Kayit {
    pub durum: Durum,
    pub videos: Vec<String>,
    pub fotolar: Vec<String>,
}

impl Kayit {
    /// Onaylı video + fotoğraf sayısı.
    pub fn dosya_sayisi(&self) -> usize {
        self.videos.len() + self.fotolar.len()
    }

    /// Kota tamam mı: 2 video + 2 fotoğraf onaylı (durumdan bağımsız).
    pub fn tam_mi(&self) -> bool {
        self.videos.len() >= VIDEO_KOTA && self.fotolar.len() >= FOTO_KOTA
    }

    /// Demo üretimi için yeterli mi: onaylı + 2 video + 2 fotoğraf.
    pub fn demoya_hazir(&self) -> bool {
        self.durum == Durum::Onaylandi && self.tam_mi()
    }

    /// Onaylı dosya yolları (video önce).
    pub fn yollar(&self) -> Vec<String> {
        self.videos
            .iter()
            .chain(self.fotolar.iter())
            .cloned()
            .collect()
    }
}

/// Onay sayaçları (CLI ve panel).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ozet {
    pub bekleyen: usize,
    pub onayli: usize,
    pub red: usize,
}

/// Panelde gösterilen tek bekleyen medya.
#[derive(Clone, Debug)]
pub struct BekleyenMedya {
    pub slug: String,
    /// Köke göreli yol: `media/pending/kuafor/video/hero_1.mp4`.
    pub goreli: String,
    pub video: bool,
    pub boyut: u64,
    pub baslik: String,
    pub lisans: String,
    pub yazar: String,
    pub kaynak: String,
    /// Qwen2.5-VL doğrulama skoru (kayıtta varsa).
    pub skor: Option<u8>,
    /// Türkiye/yerel doku kararı (kayıtta varsa).
    pub yerel: Option<bool>,
    /// Diskteki tam yol (`D:\...\media\pending\...`), panellerde kopyalanır.
    pub tam: String,
}

/// Onay kayıt dosyasının yolu.
pub fn yol(kok: &Path) -> PathBuf {
    kok.join(DOSYA)
}

fn dosya_oku(kok: &Path) -> serde_json::Value {
    fs::read_to_string(yol(kok))
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_else(|| serde_json::json!({ "surum": 1, "sektorler": {} }))
}

fn dosya_yaz(kok: &Path, deger: &serde_json::Value) -> Result<(), String> {
    let metin = serde_json::to_string_pretty(deger)
        .map_err(|hata| format!("Onay dosyası yazılamadı: {hata}"))?;
    let gecici = kok.join(format!("{DOSYA}.gecici"));
    fs::write(&gecici, metin).map_err(|hata| format!("Onay dosyası yazılamadı: {hata}"))?;
    fs::rename(&gecici, yol(kok)).map_err(|hata| format!("Onay dosyası yazılamadı: {hata}"))
}

/// Köke göre göreli, `/` ayraçlı yol: `media/stock/kuafor/photos/1.jpg`.
pub fn goreli(kok: &Path, dosya: &Path) -> String {
    let goreli = dosya.strip_prefix(kok).unwrap_or(dosya);
    let metin = goreli.to_string_lossy().replace('\\', "/");
    metin.strip_prefix("./").unwrap_or(&metin).to_string()
}

fn uzanti(dosya: &Path) -> String {
    dosya
        .extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Video dosyası mı?
pub fn video_mu(dosya: &Path) -> bool {
    matches!(uzanti(dosya).as_str(), "mp4" | "webm" | "mov" | "m4v")
}

/// Tek sektörün kaydı; kayıt yoksa [`Durum::Yok`].
pub fn kayit(kok: &Path, slug: &str) -> Kayit {
    let deger = dosya_oku(kok);
    let Some(veri) = deger.pointer(&format!("/sektorler/{slug}")) else {
        return Kayit::default();
    };
    Kayit {
        durum: Durum::metinden(veri.get("durum").and_then(|v| v.as_str()).unwrap_or("yok")),
        videos: veri
            .get("videolar")
            .and_then(|v| v.as_array())
            .map(|liste| {
                liste
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .or_else(|| {
                // Eski kayıtlar tek `video` alanı taşır; okurken geriye uyumlu.
                veri.get("video")
                    .and_then(|v| v.as_str())
                    .map(|yol| vec![yol.to_string()])
            })
            .unwrap_or_default(),
        fotolar: veri
            .get("fotolar")
            .and_then(|v| v.as_array())
            .map(|liste| {
                liste
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn kayit_yaz(kok: &Path, slug: &str, kayit: &Kayit) -> Result<(), String> {
    let mut deger = dosya_oku(kok);
    if !deger.get("sektorler").is_some_and(|v| v.is_object()) {
        deger["sektorler"] = serde_json::json!({});
    }
    deger["sektorler"][slug] = serde_json::json!({
        "durum": kayit.durum.metin(),
        "videolar": kayit.videos,
        // Geriye uyum: eski okuyucular ilk videoyu görsün.
        "video": kayit.videos.first(),
        "fotolar": kayit.fotolar,
        "guncelleme": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });
    deger["surum"] = serde_json::json!(1);
    dosya_yaz(kok, &deger)
}

/// Silinen dosyayı onay kaydından düşer; kota yeniden açılır.
///
/// Dosya onaylı değildiyse ya da kayıt yoksa dokunmaz ve `false` döner.
/// Onaylı listeden düşünce durum `bekliyor`a iner ki yeni dosya onaylanabilsin.
pub fn dosya_unut(kok: &Path, goreli_yol: &str) -> bool {
    let goreli_yol = goreli_yol.replace('\\', "/");
    let Some(slug) = goreli_yol
        .strip_prefix(&format!("{}/", crate::sektor_medya::KLASOR))
        .and_then(|kalan| kalan.split('/').next())
        .map(str::to_string)
    else {
        return false;
    };
    let mut kayit = kayit(kok, &slug);
    let once = kayit.dosya_sayisi();
    kayit.videos.retain(|yol| yol != &goreli_yol);
    kayit.fotolar.retain(|yol| yol != &goreli_yol);
    if kayit.dosya_sayisi() == once {
        return false;
    }
    if !kayit.tam_mi() && kayit.durum == Durum::Onaylandi {
        kayit.durum = Durum::Bekliyor;
    }
    kayit_yaz(kok, &slug, &kayit).is_ok()
}

fn klasor_dosyalari(klasor: &Path) -> Vec<PathBuf> {
    let Ok(liste) = fs::read_dir(klasor) else {
        return Vec::new();
    };
    let mut cikti: Vec<PathBuf> = liste
        .filter_map(Result::ok)
        .map(|giris| giris.path())
        .filter(|dosya| dosya.is_file())
        .collect();
    cikti.sort();
    cikti
}

/// Sektörün onay bekleyen dosyaları (video + fotoğraf).
pub fn bekleyen_dosyalar(kok: &Path, slug: &str) -> Vec<PathBuf> {
    let sektor = kok.join(BEKLEYEN).join(slug);
    let mut cikti = Vec::new();
    for alt in ["video", "photos"] {
        cikti.extend(klasor_dosyalari(&sektor.join(alt)));
    }
    cikti
}

/// Onay bekleyen (video, fotoğraf) sayısı.
pub fn bekleyen_sayilar(kok: &Path, slug: &str) -> (usize, usize) {
    let dosyalar = bekleyen_dosyalar(kok, slug);
    let video = dosyalar.iter().filter(|dosya| video_mu(dosya)).count();
    (video, dosyalar.len() - video)
}

/// Bekleyen havuzdaki sektör slug'ları (klasör adları).
fn bekleyen_sektorler(kok: &Path) -> Vec<String> {
    let Ok(liste) = fs::read_dir(kok.join(BEKLEYEN)) else {
        return Vec::new();
    };
    let mut cikti: Vec<String> = liste
        .filter_map(Result::ok)
        .filter(|giris| giris.path().is_dir())
        .filter_map(|giris| giris.file_name().to_str().map(str::to_string))
        .collect();
    cikti.sort();
    cikti
}

fn meta_oku(kok: &Path, slug: &str) -> serde_json::Value {
    fs::read_to_string(kok.join(BEKLEYEN).join(slug).join("KAYNAK.json"))
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn meta_bul(meta: &serde_json::Value, dosya: &str) -> serde_json::Value {
    for anahtar in ["videolar", "fotolar"] {
        let Some(liste) = meta.get(anahtar).and_then(|v| v.as_array()) else {
            continue;
        };
        for kayit in liste {
            if kayit.get("dosya").and_then(|v| v.as_str()) == Some(dosya) {
                return kayit.clone();
            }
        }
    }
    serde_json::Value::Null
}

/// Panel için sektörün bekleyen medyası; kaynak/lisans ve AI skoru ile.
pub fn bekleyen_medya(kok: &Path, slug: &str) -> Vec<BekleyenMedya> {
    let meta = meta_oku(kok, slug);
    let sektor = kok.join(BEKLEYEN).join(slug);
    let mut cikti = Vec::new();
    for alt in ["video", "photos"] {
        for dosya in klasor_dosyalari(&sektor.join(alt)) {
            let ad = dosya
                .file_name()
                .and_then(|a| a.to_str())
                .unwrap_or_default();
            let anahtar = format!("{alt}/{ad}");
            let bilgi = meta_bul(&meta, &anahtar);
            let metin = |alan: &str| {
                bilgi
                    .get(alan)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            cikti.push(BekleyenMedya {
                slug: slug.to_string(),
                goreli: goreli(kok, &dosya),
                video: alt == "video",
                boyut: dosya.metadata().map(|m| m.len()).unwrap_or(0),
                baslik: metin("baslik"),
                lisans: metin("lisans"),
                yazar: metin("yazar"),
                kaynak: metin("kaynak"),
                tam: crate::medya_islem::tam_yol(&dosya),
                skor: bilgi
                    .get("skor")
                    .and_then(|v| v.as_u64())
                    .map(|skor| skor.min(100) as u8),
                yerel: bilgi.get("yerel").and_then(|v| v.as_bool()),
            });
        }
    }
    cikti
}

fn tasi(kaynak: &Path, hedef: &Path) -> Result<(), String> {
    if fs::rename(kaynak, hedef).is_ok() {
        return Ok(());
    }
    fs::copy(kaynak, hedef).map_err(|hata| format!("{} taşınamadı: {hata}", kaynak.display()))?;
    fs::remove_file(kaynak).map_err(|hata| format!("{} silinemedi: {hata}", kaynak.display()))
}

/// Tek medyayı onaylar: `media/pending/<slug>/...` → `media/stock/<slug>/...`.
///
/// Kota korunur: sektörde 1 video ve 2 fotoğraf onaylandıysa yeni dosya
/// kabul edilmez (arama da durur).
pub fn onayla_dosya(kok: &Path, slug: &str, bekleyen: &str) -> Result<Kayit, String> {
    let kaynak = kok.join(bekleyen);
    if !kaynak.is_file() {
        return Err(format!("{bekleyen}: bekleyen dosya yok"));
    }
    let goreli_yol = goreli(kok, &kaynak);
    let sektor_onek = format!("{BEKLEYEN}/{slug}/");
    let Some(alt) = goreli_yol.strip_prefix(&sektor_onek) else {
        return Err(format!("{goreli_yol}: {slug} sektörünün havuzunda değil"));
    };
    // Katı kalite: kayıtta skor varsa %100 olmayan dosya onaylanmaz.
    let meta = meta_oku(kok, slug);
    if let Some(skor) = meta_bul(&meta, alt).get("skor").and_then(|v| v.as_u64()) {
        if skor < u64::from(crate::medya_dogrulama::GUVEN_ESIK) {
            return Err(format!(
                "{slug}: eşleşme skoru %{skor} < %{}; onay kuyruğuna alınmaz",
                crate::medya_dogrulama::GUVEN_ESIK
            ));
        }
    }
    let bilgi = meta_bul(&meta, alt);
    if bilgi.get("yerel").and_then(|v| v.as_bool()) == Some(false) {
        return Err(format!(
            "{slug}: yerel doku doğrulanmadı (%100 Türkiye şartı); onay kuyruğuna alınmaz"
        ));
    }
    let mut kayit = kayit(kok, slug);
    if video_mu(&kaynak) {
        if kayit.videos.len() >= VIDEO_KOTA {
            return Err(format!("{slug}: {VIDEO_KOTA} video kotası dolu"));
        }
    } else if kayit.fotolar.len() >= FOTO_KOTA {
        return Err(format!("{slug}: {FOTO_KOTA} fotoğraf kotası dolu"));
    }
    let hedef = kok.join(crate::sektor_medya::KLASOR).join(slug).join(alt);
    if let Some(ust) = hedef.parent() {
        fs::create_dir_all(ust).map_err(|hata| format!("Klasör oluşturulamadı: {hata}"))?;
    }
    tasi(&kaynak, &hedef)?;
    crate::medya_islem::kaydet(
        kok,
        "ONAY",
        &format!(
            "{} → {}",
            crate::medya_islem::tam_yol(&kaynak),
            crate::medya_islem::tam_yol(&hedef)
        ),
        &format!("{slug} · media/stock/{slug}"),
    );
    let hedef_goreli = goreli(kok, &hedef);
    if video_mu(&hedef) {
        kayit.videos.push(hedef_goreli);
    } else {
        kayit.fotolar.push(hedef_goreli);
    }
    kayit.durum = if kayit.tam_mi() {
        Durum::Onaylandi
    } else {
        Durum::Bekliyor
    };
    kayit_yaz(kok, slug, &kayit)?;
    Ok(kayit)
}

/// Tek medyayı reddeder ve diskten siler; yeni medya yalnız kullanıcı ekleyince gelir.
pub fn reddet_dosya(kok: &Path, slug: &str, bekleyen: &str) -> Result<(), String> {
    let dosya = kok.join(bekleyen);
    let goreli_yol = goreli(kok, &dosya);
    let sektor_onek = format!("{BEKLEYEN}/{slug}/");
    if !goreli_yol.starts_with(&sektor_onek) {
        return Err(format!("{goreli_yol}: {slug} sektörünün havuzunda değil"));
    }
    fs::remove_file(&dosya).map_err(|hata| format!("{goreli_yol} silinemedi: {hata}"))?;
    crate::medya_islem::kaydet(
        kok,
        "RED",
        &crate::medya_islem::tam_yol(&dosya),
        &format!("{slug} · diskten kalıcı olarak silindi"),
    );
    if bekleyen_dosyalar(kok, slug).is_empty() {
        let mut kayit = kayit(kok, slug);
        if !kayit.demoya_hazir() {
            kayit.durum = Durum::Bekliyor;
            kayit_yaz(kok, slug, &kayit)?;
        }
    }
    Ok(())
}

/// Sektörün tüm bekleyen medyasını onaylar (kota kadar), sonra kotayı doğrular.
pub fn onayla(kok: &Path, slug: &str) -> Result<Kayit, String> {
    let mut son = kayit(kok, slug);
    let mut engel = String::new();
    for dosya in bekleyen_dosyalar(kok, slug) {
        if son.tam_mi() {
            break;
        }
        let goreli_yol = goreli(kok, &dosya);
        match onayla_dosya(kok, slug, &goreli_yol) {
            Ok(kayit) => son = kayit,
            // Kotası dolan tür atlanır; eksik tür (video/fotoğraf) aranmaya devam eder.
            Err(hata) => engel = hata,
        }
    }
    if son.demoya_hazir() {
        Ok(son)
    } else {
        let ek = if engel.is_empty() {
            String::new()
        } else {
            format!(" · son hata: {engel}")
        };
        Err(format!(
            "{slug}: onay için {VIDEO_KOTA} video + {FOTO_KOTA} fotoğraf gerekir (onaylı {} dosya, bekleyen {}){ek}",
            son.dosya_sayisi(),
            bekleyen_dosyalar(kok, slug).len()
        ))
    }
}

/// Sektörün bekleyen ve onaylı tüm medyasını siler (toplu red).
pub fn reddet(kok: &Path, slug: &str) -> Result<Vec<String>, String> {
    let mut silinen = Vec::new();
    for ust in [BEKLEYEN, crate::sektor_medya::KLASOR] {
        for alt in ["video", "photos"] {
            for dosya in klasor_dosyalari(&kok.join(ust).join(slug).join(alt)) {
                if fs::remove_file(&dosya).is_ok() {
                    crate::medya_islem::kaydet(
                        kok,
                        "RED",
                        &crate::medya_islem::tam_yol(&dosya),
                        "toplu silme",
                    );
                    silinen.push(goreli(kok, &dosya));
                }
            }
        }
    }
    let yeni = Kayit {
        durum: Durum::Reddedildi,
        ..Kayit::default()
    };
    kayit_yaz(kok, slug, &yeni)?;
    silinen.sort();
    Ok(silinen)
}

/// Onaylı medya dosyaları (video önce); onay yoksa boş liste.
pub fn onayli_medya(kok: &Path, slug: &str) -> Vec<PathBuf> {
    let kayit = kayit(kok, slug);
    if !kayit.demoya_hazir() {
        return Vec::new();
    }
    kayit
        .yollar()
        .into_iter()
        .map(|goreli_yol| kok.join(goreli_yol))
        .filter(|dosya| dosya.is_file())
        .collect()
}

/// Demo üretim kilidi: onay yoksa demo üretilmez.
///
/// Kilit yalnız seçilmiş sektör tablosundaki sektörleri kapsar; tabloda
/// olmayan özel sektörlerin tedarik edilmiş medyası olamaz.
pub fn demo_kilidi(kok: &Path, slug: &str) -> Result<(), String> {
    let tabloda = crate::SEKTORLER
        .iter()
        .any(|tanim| crate::sektor_medya::slug(tanim.ad) == slug);
    if !tabloda {
        return Ok(());
    }
    let kayit = kayit(kok, slug);
    if kayit.demoya_hazir()
        && kayit
            .yollar()
            .iter()
            .all(|goreli_yol| kok.join(goreli_yol).is_file())
    {
        return Ok(());
    }
    let bekleyen = bekleyen_dosyalar(kok, slug).len();
    Err(format!(
        "{slug}: demo üretimi kilitli · {} · bekleyen {bekleyen} medya · onay: --onizleme paneli",
        kayit.durum.etiket()
    ))
}

/// Durum sayaçları.
pub fn ozet(kok: &Path) -> Ozet {
    let mut ozet = Ozet::default();
    for (_, kayit) in kuyruk(kok) {
        if kayit.demoya_hazir() {
            ozet.onayli += 1;
        } else if kayit.durum == Durum::Reddedildi {
            ozet.red += 1;
        } else {
            ozet.bekleyen += 1;
        }
    }
    ozet
}

/// Onay kuyruğu: bekleyen medyası veya kaydı olan sektörler, ada göre sıralı.
pub fn kuyruk(kok: &Path) -> Vec<(String, Kayit)> {
    let mut sluglar: Vec<String> = dosya_oku(kok)
        .get("sektorler")
        .and_then(|v| v.as_object())
        .map(|nesne| nesne.keys().cloned().collect())
        .unwrap_or_default();
    for slug in bekleyen_sektorler(kok) {
        if !sluglar.contains(&slug) {
            sluglar.push(slug);
        }
    }
    sluglar.sort();
    let mut cikti = Vec::new();
    for slug in sluglar {
        let mut kayit = kayit(kok, &slug);
        let bekleyen = !bekleyen_dosyalar(kok, &slug).is_empty();
        if bekleyen && !kayit.demoya_hazir() {
            kayit.durum = Durum::Bekliyor;
        }
        if bekleyen || kayit.dosya_sayisi() > 0 || kayit.durum == Durum::Reddedildi {
            cikti.push((slug, kayit));
        }
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> PathBuf {
        let kok = std::env::temp_dir().join(format!("nexus_onay_{}_{ad}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(&kok).unwrap();
        kok
    }

    fn bekleyen_dosya(kok: &Path, slug: &str, alt: &str, ad: &str) -> PathBuf {
        let klasor = kok.join(BEKLEYEN).join(slug).join(alt);
        fs::create_dir_all(&klasor).unwrap();
        let yol = klasor.join(ad);
        fs::write(&yol, [0u8; 32]).unwrap();
        yol
    }

    #[test]
    fn onay_dosyayi_stoga_tasir_ve_kotayi_korur() {
        let kok = gecici("tasi");
        let video1 = bekleyen_dosya(&kok, "kuafor", "video", "hero_1.mp4");
        let video2 = bekleyen_dosya(&kok, "kuafor", "video", "hero_2.mp4");
        let foto1 = bekleyen_dosya(&kok, "kuafor", "photos", "1.jpg");
        let foto2 = bekleyen_dosya(&kok, "kuafor", "photos", "2.jpg");
        for dosya in [&video1, &foto1] {
            onayla_dosya(&kok, "kuafor", &goreli(&kok, dosya.as_path())).unwrap();
        }
        assert!(demo_kilidi(&kok, "kuafor").is_err(), "eksik kota kilidi");
        onayla_dosya(&kok, "kuafor", &goreli(&kok, foto2.as_path())).unwrap();
        assert!(demo_kilidi(&kok, "kuafor").is_err(), "tek video yetmez");
        onayla_dosya(&kok, "kuafor", &goreli(&kok, video2.as_path())).unwrap();
        assert!(demo_kilidi(&kok, "kuafor").is_ok(), "kota dolunca açılır");
        assert!(!video1.exists(), "bekleyen video taşınmalı");
        assert!(kok.join("media/stock/kuafor/video/hero_1.mp4").is_file());
        assert_eq!(onayli_medya(&kok, "kuafor").len(), VIDEO_KOTA + FOTO_KOTA);
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn silinen_dosya_onay_kaydindan_duser() {
        let kok = gecici("unut");
        let onayli = Kayit {
            durum: Durum::Onaylandi,
            videos: vec!["media/stock/kuafor/video/hero_1.mp4".to_string()],
            fotolar: vec![
                "media/stock/kuafor/photos/1.jpg".to_string(),
                "media/stock/kuafor/photos/2.jpg".to_string(),
            ],
        };
        kayit_yaz(&kok, "kuafor", &onayli).unwrap();

        assert!(dosya_unut(&kok, "media/stock/kuafor/photos/1.jpg"));
        let kalan = kayit(&kok, "kuafor");
        assert_eq!(kalan.fotolar.len(), 1);
        assert_eq!(kalan.videos.len(), 1);
        assert_eq!(kalan.durum, Durum::Bekliyor, "kota açılınca onay yenilenir");
        assert!(
            !dosya_unut(&kok, "media/stock/kuafor/photos/yok.jpg"),
            "kayıtta olmayan dosya değişiklik yapmaz"
        );
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn kota_dolunca_fazla_medya_onaylanmaz() {
        let kok = gecici("kota");
        let mut dosyalar = vec![
            bekleyen_dosya(&kok, "kuafor", "video", "hero_1.mp4"),
            bekleyen_dosya(&kok, "kuafor", "video", "hero_2.mp4"),
        ];
        for ad in ["1.jpg", "2.jpg", "3.jpg"] {
            dosyalar.push(bekleyen_dosya(&kok, "kuafor", "photos", ad));
        }
        let kayit = onayla(&kok, "kuafor").expect("2 video + 2 foto onaylanmalı");
        assert!(kayit.demoya_hazir());
        assert_eq!(kayit.videos.len(), VIDEO_KOTA);
        assert_eq!(kayit.fotolar.len(), FOTO_KOTA);
        assert_eq!(
            kayit.videos.first().map(|v| v.ends_with("hero_1.mp4")),
            Some(true)
        );
        let kalan = bekleyen_dosyalar(&kok, "kuafor").len();
        assert_eq!(kalan, 1, "kota dışı dosyalar beklemede kalır");
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn red_dosyayi_diskten_siler() {
        let kok = gecici("red");
        let dosya = bekleyen_dosya(&kok, "kuafor", "photos", "1.jpg");
        reddet_dosya(&kok, "kuafor", &goreli(&kok, dosya.as_path())).unwrap();
        assert!(!dosya.exists());
        assert_eq!(kayit(&kok, "kuafor").durum, Durum::Bekliyor);
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn toplu_red_hem_bekleyeni_hem_stogu_siler() {
        let kok = gecici("temizle");
        for dosya in [
            bekleyen_dosya(&kok, "kuafor", "video", "hero_1.mp4"),
            bekleyen_dosya(&kok, "kuafor", "video", "hero_2.mp4"),
            bekleyen_dosya(&kok, "kuafor", "photos", "1.jpg"),
            bekleyen_dosya(&kok, "kuafor", "photos", "2.jpg"),
        ] {
            onayla_dosya(&kok, "kuafor", &goreli(&kok, dosya.as_path())).unwrap();
        }
        assert!(demo_kilidi(&kok, "kuafor").is_ok());
        let silinen = reddet(&kok, "kuafor").unwrap();
        assert_eq!(silinen.len(), VIDEO_KOTA + FOTO_KOTA);
        assert_eq!(kayit(&kok, "kuafor").durum, Durum::Reddedildi);
        assert!(onayli_medya(&kok, "kuafor").is_empty());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn yuz_alti_skor_onaylanmaz() {
        let kok = gecici("esik");
        let dosya = bekleyen_dosya(&kok, "kuafor", "photos", "1.jpg");
        fs::write(
            kok.join(BEKLEYEN).join("kuafor").join("KAYNAK.json"),
            serde_json::json!({"fotolar": [{"dosya": "photos/1.jpg", "skor": 72}]}).to_string(),
        )
        .unwrap();
        let hata = onayla_dosya(&kok, "kuafor", &goreli(&kok, dosya.as_path())).unwrap_err();
        assert!(hata.contains("%100") && hata.contains("72"), "{hata}");
        assert!(dosya.is_file(), "dosya beklemede kalmalı");
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn panel_metaverisi_skor_ve_yerelligi_okur() {
        let kok = gecici("meta");
        bekleyen_dosya(&kok, "kuafor", "photos", "1.jpg");
        fs::write(
            kok.join(BEKLEYEN).join("kuafor").join("KAYNAK.json"),
            serde_json::json!({
                "fotolar": [{
                    "dosya": "photos/1.jpg",
                    "baslik": "Kuaför salonu",
                    "lisans": "CC BY 4.0",
                    "yazar": "Ayşe",
                    "kaynak": "https://commons.wikimedia.org/",
                    "skor": 100,
                    "yerel": true
                }]
            })
            .to_string(),
        )
        .unwrap();
        let liste = bekleyen_medya(&kok, "kuafor");
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].skor, Some(100));
        assert_eq!(liste[0].yerel, Some(true));
        assert_eq!(liste[0].baslik, "Kuaför salonu");
        assert_eq!(liste[0].boyut, 32);
        assert_eq!(ozet(&kok).bekleyen, 1);
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn kuyruk_bekleyen_sektorleri_listeler() {
        let kok = gecici("kuyruk");
        bekleyen_dosya(&kok, "restoran", "video", "hero_1.mp4");
        let kuyruk = kuyruk(&kok);
        assert_eq!(kuyruk.len(), 1);
        assert_eq!(kuyruk[0].0, "restoran");
        assert_eq!(kuyruk[0].1.durum, Durum::Bekliyor);
        let _ = fs::remove_dir_all(kok);
    }
}
