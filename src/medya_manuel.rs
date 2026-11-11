//! Kullanıcı tarafından eklenen medya: link veya dosya → `media/pending`.
//!
//! Otomatik arama yoktur. Panelden gelen doğrudan görsel/video linki ya da
//! bilgisayardan yüklenen dosya burada işlenir:
//!
//! 1. Kaynak indirilir (en çok 300 MB video / 80 MB fotoğraf).
//! 2. Çözünürlük kapısı: video en az 720p, fotoğraf en az 1080p.
//! 3. Fotoğraf 1920 px WebP'ye, video 1080p H.264 MP4'e dönüştürülür.
//! 4. Qwen2.5-VL doğrulaması: %100 eşleşme + %100 Türkiye değilse dosya silinir.
//! 5. Geçen dosya `media/pending/<slug>/` altına yazılır ve `KAYNAK.json`
//!    içine kaynak/skor/yerellik bilgisi işlenir.
//!
//! Onay ve red [`crate::medya_onay`] üzerinden yürür.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::medya_arsiv::MB;
use crate::medya_dogrulama::GUVEN_ESIK;

/// Tanıtıcı: stok siteleri tarayıcı olmayan isteği reddetmesin.
const AJAN: &str = "NexusEnterprise/2.0 (manual media; local demo)";
/// Link indirmesinde fotoğraf üst sınırı.
pub const LINK_FOTO_MAKS_BAYT: u64 = 80 * MB;
/// Link indirmesinde video üst sınırı.
pub const LINK_VIDEO_MAKS_BAYT: u64 = 300 * MB;
/// Yüklenen dosya için genel üst sınır (panel gövde sınırıyla aynı).
pub const DOSYA_MAKS_BAYT: u64 = 320 * MB;
/// Manuel videonun en fazla işlenecek süresi (sn); uzun kaynaklar kırpılır.
pub const VIDEO_MAKS_SN: u32 = 60;
/// Kabul edilen en kısa video süresi.
pub const VIDEO_MIN_SN: u32 = 8;
/// Video çıktısının hedef ölçüsü.
const VIDEO_GENISLIK: u32 = 1920;
const VIDEO_YUKSEKLIK: u32 = 1080;
/// Fotoğraf çıktısının hedef genişliği.
const FOTO_GENISLIK: u32 = 1920;

/// Başarılı eklemenin panel mesajı.
#[derive(Clone, Debug)]
pub struct Ekleme {
    pub slug: String,
    pub video: bool,
    pub skor: u8,
    /// Onay bekleyen havuzdaki göreli yol (`video/hero_1.mp4`).
    pub dosya: String,
}

impl Ekleme {
    /// Panelde gösterilecek tek satır.
    pub fn mesaj(&self) -> String {
        format!(
            "{}: {} eklendi · %{} eşleşme · 1080p · onay bekliyor",
            self.slug,
            if self.video { "video" } else { "fotoğraf" },
            self.skor
        )
    }
}

/// Kullanıcının verdiği doğrudan medya linkini indirip havuza ekler.
pub fn link_ekle(kok: &Path, slug: &str, link: &str) -> Result<Ekleme, String> {
    let link = link.trim();
    url_gecerli(link)?;
    let (veri, icerik_turu) = indir(link)?;
    let ad = baslik_url(link);
    ekle_bayt(kok, slug, &ad, &icerik_turu, &veri, Some(link))
}

/// Bilgisayardan yüklenen ham dosyayı havuza ekler.
///
/// `icerik_turu` tarayıcının bildirdiği MIME türüdür; boş olabilir. Tür,
/// MIME + dosya adı + sihirli baytların birleşiminden belirlenir.
pub fn dosya_ekle(
    kok: &Path,
    slug: &str,
    ad: &str,
    icerik_turu: &str,
    veri: &[u8],
) -> Result<Ekleme, String> {
    if veri.is_empty() {
        return Err("yüklenen dosya boş".to_string());
    }
    if veri.len() as u64 > DOSYA_MAKS_BAYT {
        return Err(format!(
            "dosya {:.1} MB; üst sınır {} MB",
            veri.len() as f64 / MB as f64,
            DOSYA_MAKS_BAYT / MB
        ));
    }
    let baslik = baslik_dosya(ad);
    ekle_bayt(kok, slug, &baslik, icerik_turu, veri, None)
}

fn ekle_bayt(
    kok: &Path,
    slug: &str,
    ad: &str,
    icerik_turu: &str,
    veri: &[u8],
    kaynak: Option<&str>,
) -> Result<Ekleme, String> {
    let (sektor_adi, _grup) = sektor_bul(slug)?;
    let video = tur_belirle(icerik_turu, ad, veri)?;
    let gecici = gecici_yol();
    fs::write(&gecici, veri).map_err(|hata| format!("geçici dosya yazılamadı: {hata}"))?;
    let sonuc = isle_gecici(kok, sektor_adi, _grup, ad, kaynak, &gecici, video);
    let _ = fs::remove_file(&gecici);
    sonuc
}

fn isle_gecici(
    kok: &Path,
    sektor_adi: &str,
    grup: &str,
    ad: &str,
    kaynak: Option<&str>,
    gecici: &Path,
    video: bool,
) -> Result<Ekleme, String> {
    let slug = crate::sektor_medya::slug(sektor_adi);
    // 1) Çözünürlük kapısı: düşük/bulanık kaynak hiç işlenmez.
    let (genislik, yukseklik) = if video {
        video_boyut(gecici)?
    } else {
        // Uzantıya güvenme: biçim dosya içeriğinden okunur.
        image::ImageReader::open(gecici)
            .map_err(|hata| format!("fotoğraf okunamadı: {hata}"))?
            .with_guessed_format()
            .map_err(|hata| format!("fotoğraf biçimi okunamadı: {hata}"))?
            .into_dimensions()
            .map_err(|hata| format!("fotoğraf ölçüsü okunamadı: {hata}"))?
    };
    if !crate::medya_tedarik::kaynak_kaliteli(genislik, yukseklik, video) {
        return Err(format!(
            "çözünürlük {genislik}×{yukseklik}; video için en az 1280×720, fotoğraf için 1920×1080 gerekir"
        ));
    }
    if video {
        let sure = video_sure(gecici)?;
        if !(f64::from(VIDEO_MIN_SN)..=f64::from(VIDEO_MAKS_SN)).contains(&sure) {
            return Err(format!(
                "video süresi {sure:.1} sn; {VIDEO_MIN_SN}–{VIDEO_MAKS_SN} sn gerekir"
            ));
        }
    }

    // 2) Havuz klasörleri ve çakışmayan hedef adı.
    let sektor_kok = kok.join(crate::medya_onay::BEKLEYEN).join(&slug);
    let (alt, onek, uzanti) = if video {
        ("video", "hero_", ".mp4")
    } else {
        ("photos", "img_", ".webp")
    };
    let klasor = sektor_kok.join(alt);
    fs::create_dir_all(&klasor).map_err(|hata| format!("klasör oluşturulamadı: {hata}"))?;
    let sira = bos_sira(&klasor, onek, uzanti);
    let hedef = klasor.join(format!("{onek}{sira}{uzanti}"));

    // 3) Yüksek kaliteli çıktı: 1920 px WebP / 1080p H.264.
    let donusum = if video {
        video_isle(gecici, &hedef)
    } else {
        foto_isle(gecici, &hedef)
    };
    if let Err(hata) = donusum {
        let _ = fs::remove_file(&hedef);
        return Err(hata);
    }

    // 4) %100 eşleşme + %100 Türkiye kapısı; geçmezse dosya diskte kalmaz.
    let etiket = crate::medya_dogrulama::etiket_bul(sektor_adi);
    let karar = crate::medya_dogrulama::dogrula_dosya(&hedef, &etiket, video);
    let skor = match &karar {
        crate::medya_dogrulama::Sonuc::Onay { skor, .. } if *skor >= GUVEN_ESIK => *skor,
        _ => {
            let _ = fs::remove_file(&hedef);
            return Err(format!(
                "%100 Türkiye/eşleşme doğrulaması geçmedi: {}",
                karar.metin()
            ));
        }
    };

    // 5) KAYNAK.json: panelde başlık, kaynak ve doğrulama bilgisi görünsün.
    let goreli = format!("{alt}/{onek}{sira}{uzanti}");
    let mut manifest = manifest_oku(&sektor_kok);
    manifest["sektor"] = serde_json::json!(sektor_adi);
    manifest["grup"] = serde_json::json!(grup);
    manifest["manuel"] = serde_json::json!(true);
    let kayit = serde_json::json!({
        "dosya": goreli,
        "baslik": ad,
        "kaynak": kaynak.unwrap_or_default(),
        "lisans": "Kullanıcı tarafından eklendi",
        "yazar": "Manuel",
        "saglayici": "Manuel ekleme",
        "skor": skor,
        "yerel": true,
        "eklendi": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });
    let anahtar = if video { "videolar" } else { "fotolar" };
    listeye_ekle(&mut manifest, anahtar, kayit);
    if let Err(hata) = manifest_yaz(&sektor_kok, &manifest) {
        // Metadata yazılamazsa dosya onay kapısını atlayamaz: diskte bırakma.
        let _ = fs::remove_file(&hedef);
        return Err(hata);
    }

    crate::medya_islem::kaydet(
        kok,
        "EKLE",
        &crate::medya_islem::tam_yol(&hedef),
        &format!("{slug} · manuel · %{skor} · 1080p · onay bekliyor"),
    );

    Ok(Ekleme {
        slug: slug.to_string(),
        video,
        skor,
        dosya: goreli,
    })
}

/// `https://...` biçiminde, yerel/özel ağa gitmeyen doğrudan medya adresi.
pub fn url_gecerli(link: &str) -> Result<(), String> {
    let kucuk = link.trim().to_ascii_lowercase();
    if kucuk.is_empty() {
        return Err("link boş".to_string());
    }
    if !(kucuk.starts_with("http://") || kucuk.starts_with("https://")) {
        return Err("yalnız http:// veya https:// linkleri kabul edilir".to_string());
    }
    let host = link
        .split("://")
        .nth(1)
        .unwrap_or_default()
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let host = host.split(':').next().unwrap_or_default();
    if host.is_empty() {
        return Err("linkte sunucu adı yok".to_string());
    }
    let ozel = host == "localhost"
        || host == "[::1]"
        || host == "0.0.0.0"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|parca| parca.parse::<u8>().ok())
                .is_some_and(|ikinci| (16..=31).contains(&ikinci)));
    if ozel {
        return Err("yerel/özel ağ adresleri kabul edilmez".to_string());
    }
    Ok(())
}

fn indir(link: &str) -> Result<(Vec<u8>, String), String> {
    // Wikimedia 429'u kısa süreli olabilir: iki kez bekleyip yeniden dene.
    let mut deneme = 0u32;
    let cevap = loop {
        let mut istek = ureq::get(link)
            .set("User-Agent", AJAN)
            .set("Accept", "image/*,video/*,*/*;q=0.8")
            .set("Accept-Language", "tr-TR,tr;q=0.9,en;q=0.7")
            .timeout(Duration::from_secs(150));
        if link.contains("wikimedia.org") {
            istek = istek.set("Referer", "https://commons.wikimedia.org/");
        }
        match istek.call() {
            Ok(cevap) => break cevap,
            Err(ureq::Error::Status(kod, _)) if (kod == 429 || kod == 503) => {
                if deneme >= 2 {
                    return Err(format!(
                        "link indirilemedi: sunucu {kod} döndü; birkaç dakika sonra tekrar deneyin"
                    ));
                }
                std::thread::sleep(Duration::from_secs(2 + u64::from(deneme) * 3));
                deneme += 1;
            }
            Err(hata) => return Err(format!("link indirilemedi: {hata}")),
        }
    };
    let icerik_turu = cevap
        .header("content-type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if icerik_turu.contains("text/html") {
        return Err(
            "link bir web sayfası; doğrudan görsel/video (jpg/png/webp/mp4) adresi yapıştırın"
                .to_string(),
        );
    }
    let video_bekleniyor = icerik_turu.starts_with("video/")
        || ["mp4", "m4v", "webm", "mov"]
            .iter()
            .any(|uzanti| link.to_ascii_lowercase().contains(&format!(".{uzanti}")));
    let sinir = if video_bekleniyor {
        LINK_VIDEO_MAKS_BAYT
    } else {
        LINK_FOTO_MAKS_BAYT
    };
    if let Some(uzunluk) = cevap
        .header("content-length")
        .and_then(|deger| deger.parse::<u64>().ok())
    {
        if uzunluk > sinir {
            return Err(format!(
                "kaynak {:.1} MB; üst sınır {} MB",
                uzunluk as f64 / MB as f64,
                sinir / MB
            ));
        }
    }
    let mut okuyucu = cevap.into_reader().take(sinir + 1);
    let mut veri = Vec::new();
    okuyucu
        .read_to_end(&mut veri)
        .map_err(|hata| format!("link okunamadı: {hata}"))?;
    if veri.len() as u64 > sinir {
        return Err(format!(
            "kaynak {} MB sınırını aştı; daha küçük bir dosya linki kullanın",
            sinir / MB
        ));
    }
    if veri.is_empty() {
        return Err("link boş yanıt döndü".to_string());
    }
    Ok((veri, icerik_turu))
}

/// MIME + dosya adı + sihirli baytlardan medya türü: `true` video.
fn tur_belirle(icerik_turu: &str, ad: &str, veri: &[u8]) -> Result<bool, String> {
    let tur = icerik_turu.to_ascii_lowercase();
    if tur.starts_with("video/") {
        return Ok(true);
    }
    if tur.starts_with("image/") {
        return Ok(false);
    }
    if veri.len() >= 12 && &veri[4..8] == b"ftyp" {
        return Ok(true);
    }
    if veri.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Ok(true);
    }
    if veri.starts_with(&[0xFF, 0xD8, 0xFF])
        || veri.starts_with(&[0x89, 0x50, 0x4E, 0x47])
        || (veri.len() >= 12 && &veri[0..4] == b"RIFF" && &veri[8..12] == b"WEBP")
    {
        return Ok(false);
    }
    let uzanti = ad
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match uzanti.as_str() {
        "mp4" | "m4v" | "webm" | "mov" => Ok(true),
        "jpg" | "jpeg" | "png" | "webp" => Ok(false),
        _ => Err("dosya türü tanınamadı; jpg/png/webp/mp4/webm yükleyin".to_string()),
    }
}

fn video_boyut(yol: &Path) -> Result<(u32, u32), String> {
    let cikti = crate::stok_video::gizli_komut("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(yol)
        .output()
        .map_err(|hata| format!("ffprobe çalıştırılamadı: {hata}"))?;
    if !cikti.status.success() {
        return Err(format!(
            "video okunamadı: {}",
            String::from_utf8_lossy(&cikti.stderr).trim()
        ));
    }
    let metin = String::from_utf8_lossy(&cikti.stdout);
    let ilk = metin.lines().next().unwrap_or_default().trim();
    let (genislik, yukseklik) = ilk
        .split_once('x')
        .ok_or_else(|| format!("video ölçüsü okunamadı: {ilk}"))?;
    let genislik = genislik
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("video genişliği okunamadı: {ilk}"))?;
    let yukseklik = yukseklik
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("video yüksekliği okunamadı: {ilk}"))?;
    Ok((genislik, yukseklik))
}

/// Videonun saniye cinsinden süresi (ffprobe).
fn video_sure(yol: &Path) -> Result<f64, String> {
    let cikti = crate::stok_video::gizli_komut("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(yol)
        .output()
        .map_err(|hata| format!("ffprobe çalıştırılamadı: {hata}"))?;
    if !cikti.status.success() {
        return Err(format!(
            "video süresi okunamadı: {}",
            String::from_utf8_lossy(&cikti.stderr).trim()
        ));
    }
    String::from_utf8_lossy(&cikti.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "video süresi sayıya çevrilemedi".to_string())
}

fn ffmpeg(argumanlar: &[&str]) -> Result<(), String> {
    let cikti = crate::stok_video::gizli_komut("ffmpeg")
        .args(argumanlar)
        .output()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if cikti.status.success() {
        Ok(())
    } else {
        Err(format!(
            "dönüştürme başarısız: {}",
            String::from_utf8_lossy(&cikti.stderr).trim()
        ))
    }
}

fn foto_isle(kaynak: &Path, hedef: &Path) -> Result<(), String> {
    let filtre = format!("scale={FOTO_GENISLIK}:-2");
    ffmpeg(&[
        "-y",
        "-loglevel",
        "error",
        "-i",
        &kaynak.to_string_lossy(),
        "-vf",
        &filtre,
        "-c:v",
        "libwebp",
        "-quality",
        "90",
        &hedef.to_string_lossy(),
    ])
}

fn video_isle(kaynak: &Path, hedef: &Path) -> Result<(), String> {
    let filtre = format!(
        "scale={VIDEO_GENISLIK}:{VIDEO_YUKSEKLIK}:force_original_aspect_ratio=decrease,\
         pad={VIDEO_GENISLIK}:{VIDEO_YUKSEKLIK}:(ow-iw)/2:(oh-ih)/2,setsar=1"
    );
    let sure = VIDEO_MAKS_SN.to_string();
    ffmpeg(&[
        "-y",
        "-loglevel",
        "error",
        "-i",
        &kaynak.to_string_lossy(),
        "-t",
        &sure,
        "-vf",
        &filtre,
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "20",
        "-an",
        "-movflags",
        "+faststart",
        "-pix_fmt",
        "yuv420p",
        &hedef.to_string_lossy(),
    ])
}

fn sektor_bul(slug: &str) -> Result<(&'static str, &'static str), String> {
    crate::SEKTORLER
        .iter()
        .find(|tanim| crate::sektor_medya::slug(tanim.ad) == slug)
        .map(|tanim| (tanim.ad, tanim.grup))
        .ok_or_else(|| format!("bilinmeyen sektör: {slug}"))
}

fn baslik_url(link: &str) -> String {
    let yol = link
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let govde = yol.split("://").nth(1).unwrap_or_default();
    if !govde.contains('/') {
        return "Manuel medya".to_string();
    }
    let son = govde.rsplit('/').next().unwrap_or_default();
    let son = son.replace("%20", " ").replace("%C3%BC", "ü");
    let son = son
        .rsplit_once('.')
        .map(|(ad, _)| ad)
        .unwrap_or(son.as_str());
    let temiz = son.replace(['_', '-'], " ").trim().to_string();
    if temiz.is_empty() {
        "Manuel medya".to_string()
    } else {
        temiz.chars().take(90).collect()
    }
}

fn baslik_dosya(ad: &str) -> String {
    let son = ad
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
    let son = son
        .rsplit_once('.')
        .map(|(govde, _)| govde)
        .unwrap_or(son.as_str());
    let temiz = son.replace(['_', '-'], " ").trim().to_string();
    if temiz.is_empty() {
        "Yüklenen medya".to_string()
    } else {
        temiz.chars().take(90).collect()
    }
}

fn gecici_yol() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|sure| sure.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("nexus_manuel_{}_{nanos}.bin", std::process::id()))
}

fn bos_sira(klasor: &Path, onek: &str, uzanti: &str) -> usize {
    for sira in 1..=999 {
        if !klasor.join(format!("{onek}{sira}{uzanti}")).exists() {
            return sira;
        }
    }
    999
}

fn manifest_oku(sektor_kok: &Path) -> serde_json::Value {
    fs::read_to_string(sektor_kok.join("KAYNAK.json"))
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_else(|| serde_json::json!({ "videolar": [], "fotolar": [] }))
}

fn manifest_yaz(sektor_kok: &Path, deger: &serde_json::Value) -> Result<(), String> {
    let metin = serde_json::to_string_pretty(deger)
        .map_err(|hata| format!("KAYNAK.json yazılamadı: {hata}"))?;
    fs::write(sektor_kok.join("KAYNAK.json"), metin)
        .map_err(|hata| format!("KAYNAK.json yazılamadı: {hata}"))
}

fn listeye_ekle(manifest: &mut serde_json::Value, anahtar: &str, kayit: serde_json::Value) {
    if manifest.get(anahtar).and_then(|v| v.as_array()).is_none() {
        manifest[anahtar] = serde_json::json!([]);
    }
    let dosya = kayit.get("dosya").cloned();
    if let Some(liste) = manifest[anahtar].as_array_mut() {
        if let Some(dosya) = &dosya {
            liste.retain(|eski| eski.get("dosya") != Some(dosya));
        }
        liste.push(kayit);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn png_olustur(genislik: u32, yukseklik: u32) -> Vec<u8> {
        let img = image::RgbImage::new(genislik, yukseklik);
        let mut cikti = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cikti, image::ImageFormat::Png)
            .unwrap();
        cikti.into_inner()
    }

    #[test]
    fn link_guvenligi_yerel_ve_dosya_adresini_reddeder() {
        assert!(url_gecerli("https://example.com/a.jpg").is_ok());
        assert!(url_gecerli("http://cdn.example.com/video.mp4").is_ok());
        assert!(url_gecerli("file:///C:/gizli.jpg").is_err());
        assert!(url_gecerli("ftp://example.com/a.jpg").is_err());
        assert!(url_gecerli("http://127.0.0.1/a.jpg").is_err());
        assert!(url_gecerli("http://localhost:8080/a.jpg").is_err());
        assert!(url_gecerli("http://192.168.1.5/a.jpg").is_err());
        assert!(url_gecerli("http://172.20.0.1/a.jpg").is_err());
        assert!(url_gecerli("javascript:alert(1)").is_err());
    }

    #[test]
    fn medya_turu_mime_uzanti_ve_sihirli_baytlardan_bulunur() {
        assert!(tur_belirle("video/mp4", "x", b"").unwrap());
        assert!(!tur_belirle("image/webp", "x", b"").unwrap());
        assert!(tur_belirle("", "klip.mp4", b"").unwrap());
        assert!(!tur_belirle("", "foto.jpg", b"").unwrap());
        assert!(tur_belirle("application/octet-stream", "x", b"\0\0\0\x18ftypmp42").unwrap());
        assert!(!tur_belirle("", "x", &png_olustur(4, 4)).unwrap());
        assert!(tur_belirle("text/html", "x", b"<html>").is_err());
        assert!(tur_belirle("", "x.pdf", b"%PDF").is_err());
    }

    #[test]
    fn dusuk_cozunurluk_ai_cagrilmadan_reddedilir() {
        let kok = std::env::temp_dir().join(format!("nexus_manuel_{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(&kok).unwrap();
        let hata = dosya_ekle(
            &kok,
            "kuafor",
            "foto.png",
            "image/png",
            &png_olustur(640, 360),
        )
        .unwrap_err();
        assert!(hata.contains("1920×1080"), "{hata}");
        assert!(!kok.join("media/pending/kuafor/photos/img_1.webp").exists());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn bilinmeyen_sektor_reddedilir() {
        let kok = std::env::temp_dir().join(format!("nexus_manuel_sektor_{}", std::process::id()));
        let hata = dosya_ekle(
            &kok,
            "olmayan_sektor",
            "foto.png",
            "image/png",
            &png_olustur(4, 4),
        )
        .unwrap_err();
        assert!(hata.contains("bilinmeyen sektör"), "{hata}");
    }

    /// Gerçek bir fotoğraf ve videoyu panelin kullandığı hattan geçirir:
    /// FFmpeg dönüşümü + Qwen2.5-VL doğrulaması + `pending` → `stock` taşıma.
    /// Model ve medya gerektirdiği için varsayılan test koşusunda atlanır:
    /// `E2E_FOTO=... E2E_VIDEO=... cargo test gercek_foto_ve_video_uctan_uca -- --ignored`
    #[test]
    #[ignore = "gerçek medya, FFmpeg ve yerel Qwen2.5-VL gerektirir"]
    fn gercek_foto_ve_video_uctan_uca() {
        let foto = std::env::var("E2E_FOTO").expect("E2E_FOTO yolunu verin");
        let video = std::env::var("E2E_VIDEO").expect("E2E_VIDEO yolunu verin");
        let kok = std::env::temp_dir().join(format!(
            "nexus_e2e_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|sure| sure.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&kok).unwrap();

        let mut son_kayit = None;
        for (yol, video_mu) in [(foto, false), (video, true)] {
            let veri = fs::read(&yol).expect("test medyası okunamadı");
            let ad = Path::new(&yol)
                .file_name()
                .and_then(|ad| ad.to_str())
                .expect("dosya adı yok");
            let ekleme = dosya_ekle(&kok, "kuafor", ad, "", &veri)
                .unwrap_or_else(|hata| panic!("{ad}: {hata}"));
            assert_eq!(ekleme.video, video_mu, "{ad}: tür yanlış");
            assert_eq!(ekleme.skor, 100, "{ad}: doğrulama skoru %100 değil");
            let bekleyen = format!(
                "{}/{}/{}",
                crate::medya_onay::BEKLEYEN,
                "kuafor",
                ekleme.dosya
            );
            assert!(kok.join(&bekleyen).is_file(), "{ad}: pending dosyası yok");
            let kayit = crate::medya_onay::onayla_dosya(&kok, "kuafor", &bekleyen)
                .unwrap_or_else(|hata| panic!("{ad}: onay başarısız: {hata}"));
            let hedef = kok
                .join(crate::sektor_medya::KLASOR)
                .join("kuafor")
                .join(&ekleme.dosya);
            assert!(hedef.is_file(), "{ad}: stok dosyası oluşmadı");
            assert!(!kok.join(&bekleyen).exists(), "{ad}: pending dosyası kaldı");
            son_kayit = Some(kayit);
        }

        let kayit = son_kayit.expect("kayıt yok");
        assert_eq!(kayit.fotolar.len(), 1, "fotoğraf onaya eklenmedi");
        assert_eq!(kayit.videos.len(), 1, "video onaya eklenmedi");
        assert!(kayit.fotolar[0].ends_with(".webp"));
        assert!(kayit.videos[0].ends_with(".mp4"));
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn basliklar_dosya_ve_linkten_temizlenir() {
        assert_eq!(
            baslik_dosya("C:\\resimler\\berber_dukkani.jpg"),
            "berber dukkani"
        );
        assert_eq!(
            baslik_url("https://ornek.com/a/berber-salonu.webp?x=1"),
            "berber salonu"
        );
        assert_eq!(baslik_url("https://ornek.com/"), "Manuel medya");
    }
}
