// --- Kitaplık (yerel içe aktarma) -----------------------------------------

/// Stok API araması kapalı: panel anahtar istemez, dışarıya istek gitmez.
/// Kod silinmedi; ileride `true` yapıldığında arama arayüzü yeniden bağlanır.
pub const API_ACIK: bool = false;

/// Panelden gelen yerel video isteği.
pub struct YerelIstek {
    pub kaynak_yol: String,
    /// Kırpmanın başlayacağı saniye.
    pub baslangic: f64,
    pub kaynak_linki: String,
    pub cekim: String,
    pub lisans: String,
    pub atif_gerekli: bool,
}

/// Desteklenen uzantılar: mp4, mov, webm (ve m4v).
pub fn uzanti_gecerli(yol: &str) -> bool {
    match Path::new(yol)
        .extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp4" | "mov" | "webm" | "m4v") => true,
        _ => false,
    }
}

/// Kaynak videonun süresi (saniye); ffprobe yoksa veya dosya bozuksa hata.
pub fn sure_sn(yol: &str) -> Result<f64, String> {
    let cikti = std::process::Command::new("ffprobe")
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
        .map_err(|_| "ffprobe bulunamadı: video süresi okunamıyor.".to_string())?;
    if !cikti.status.success() {
        return Err(
            "Video okunamadı: dosya bozuk veya desteklenmeyen bir biçimde.".to_string(),
        );
    }
    String::from_utf8_lossy(&cikti.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "Video süresi okunamadı.".to_string())
}

/// Başlangıç saniyesini videonun sınırlarına çeker: son 8 saniyeden önce.
fn baslangic_sinirla(baslangic: f64, sure: f64) -> f64 {
    if !baslangic.is_finite() || baslangic < 0.0 {
        return 0.0;
    }
    baslangic.min((sure - VIDEO_SN).max(0.0))
}

/// Dönüştürülen videonun piksel ölçüsü (poster/video künyesi için).
fn boyut_oku(yol: &Path) -> Option<(u32, u32)> {
    let cikti = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(yol)
        .output()
        .ok()?;
    if !cikti.status.success() {
        return None;
    }
    let metin = String::from_utf8_lossy(&cikti.stdout);
    let (genislik, yukseklik) = metin.trim().split_once(',')?;
    Some((
        genislik.trim().parse().ok()?,
        yukseklik.trim().parse().ok()?,
    ))
}

fn kaynak_oku(klasor: &Path) -> serde_json::Value {
    std::fs::read_to_string(klasor.join("KAYNAK.json"))
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_else(|| serde_json::json!({"surum": 2, "videolar": []}))
}

/// Kayıt listesi: yeni biçimde `videolar`, eski biçimde tek nesne.
fn kayit_listesi(json: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(liste) = json.get("videolar").and_then(|v| v.as_array()) {
        return liste.clone();
    }
    if json.get("dosya").is_some() {
        vec![json.clone()]
    } else {
        Vec::new()
    }
}

fn bilgi_uret(klasor: &Path, grup_id: &str, kayit: &serde_json::Value) -> Option<VideoBilgi> {
    let dosya_adi = metin(kayit, "dosya");
    if dosya_adi.is_empty() {
        return None;
    }
    let poster_adi = match metin(kayit, "poster") {
        ad if ad.is_empty() => "poster.webp".to_string(),
        ad => ad,
    };
    let video = klasor.join(&dosya_adi);
    let poster = klasor.join(&poster_adi);
    if !video.is_file() || !poster.is_file() {
        return None;
    }
    Some(VideoBilgi {
        grup: grup_id.to_string(),
        dosya_adi,
        poster_adi,
        dosya: "media/hero.mp4".to_string(),
        poster: "media/poster.webp".to_string(),
        saglayici: metin(kayit, "saglayici"),
        sahip: metin(kayit, "sahip"),
        kaynak_sayfasi: metin(kayit, "kaynak_sayfasi"),
        lisans: metin(kayit, "lisans"),
        atif: metin(kayit, "atif"),
        atif_gerekli: kayit
            .get("atif_gerekli")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        sure: kayit.get("sure_sn").and_then(|v| v.as_f64()).unwrap_or(0.0),
        genislik: kayit
            .get("genislik")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        yukseklik: kayit
            .get("yukseklik")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        video_bayt: std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0),
        poster_bayt: std::fs::metadata(&poster).map(|m| m.len()).unwrap_or(0),
    })
}

/// Bir sektördeki tüm videolar (diskte var olanlar).
pub fn kayitlar(grup_id: &str) -> Vec<VideoBilgi> {
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    let json = kaynak_oku(&klasor);
    kayit_listesi(&json)
        .iter()
        .filter_map(|kayit| bilgi_uret(&klasor, grup_id, kayit))
        .collect()
}

/// Uyumluluk: grubun ilk videosu.
pub fn video_bilgisi(grup_id: &str) -> Option<VideoBilgi> {
    kayitlar(grup_id).into_iter().next()
}

/// İşletme adı + adresten sabit tohum: komşu işletmeler farklı video alır.
pub fn tohum(ad: &str, adres: &str) -> u64 {
    let mut ozet: u64 = 0xcbf2_9ce4_8422_2325;
    for bayt in format!("{ad}|{adres}").to_lowercase().as_bytes() {
        ozet ^= u64::from(*bayt);
        ozet = ozet.wrapping_mul(0x100_0000_01b3);
    }
    ozet
}

/// Tohuma göre video seçer; video yoksa `None` (shader sürümüne düşülür).
pub fn sec(grup_id: &str, tohum: u64) -> Option<VideoBilgi> {
    let liste = kayitlar(grup_id);
    if liste.is_empty() {
        return None;
    }
    Some(liste[(tohum % liste.len() as u64) as usize].clone())
}

/// Kütüphanedeki tüm sektör videoları (panel listesi).
pub fn kutuphane() -> Vec<VideoBilgi> {
    let mut cikti = Vec::new();
    for grup in GRUPLAR {
        cikti.extend(kayitlar(grup.id));
    }
    cikti
}

/// Yerel videoyu dönüştürür: 720p, her kare anahtar kare, sessiz, 8 sn.
pub fn yerel_hazirla(grup_id: &str, istek: &YerelIstek) -> Result<Hazir, String> {
    let yol = istek.kaynak_yol.trim();
    if yol.is_empty() {
        return Err("Video yolu boş: dosyayı sürükleyin veya tam yolu yapıştırın.".to_string());
    }
    let kaynak = Path::new(yol);
    if !kaynak.is_file() {
        return Err(format!("Dosya bulunamadı: {yol}"));
    }
    if !uzanti_gecerli(yol) {
        return Err("Yalnız mp4, mov ve webm dosyaları kabul edilir.".to_string());
    }
    if !ffmpeg_var() {
        return Err(
            "ffmpeg bulunamadı: video dönüştürülemiyor. Kurulum: https://ffmpeg.org/download.html"
                .to_string(),
        );
    }
    let sure = sure_sn(yol)?;
    if sure < 1.0 {
        return Err("Video çok kısa: en az 1 saniye olmalı.".to_string());
    }
    let baslangic = baslangic_sinirla(istek.baslangic, sure);
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    std::fs::create_dir_all(&klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    let mut json = kaynak_oku(&klasor);
    let mut liste = kayit_listesi(&json);
    // Aynı sektörde birden fazla video: sıradaki boş numara bulunur.
    let mut sira = liste.len() + 1;
    let mut video_adi = format!("v{sira}.mp4");
    while klasor.join(&video_adi).exists() {
        sira += 1;
        video_adi = format!("v{sira}.mp4");
    }
    let poster_adi = format!("v{sira}.webp");
    let video_yolu = klasor.join(&video_adi);
    let poster_yolu = klasor.join(&poster_adi);

    let durum = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-ss", &format!("{baslangic:.3}"), "-i"])
        .arg(kaynak)
        .args([
            "-t",
            &format!("{VIDEO_SN:.0}"),
            "-an",
            "-vf",
            "scale=-2:720:flags=lanczos",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-profile:v",
            "high",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "1",
            "-keyint_min",
            "1",
            "-sc_threshold",
            "0",
            "-movflags",
            "+faststart",
        ])
        .arg(&video_yolu)
        .status()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        let _ = std::fs::remove_file(&video_yolu);
        return Err("Video dönüştürülemedi: dosya bozuk veya okunamıyor.".to_string());
    }

    let durum = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-ss", "0.6", "-i"])
        .arg(&video_yolu)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=-2:720",
            "-c:v",
            "libwebp",
            "-quality",
            "82",
        ])
        .arg(&poster_yolu)
        .status()
        .map_err(|hata| format!("ffmpeg (poster) çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        let _ = std::fs::remove_file(&video_yolu);
        return Err("Poster görüntüsü çıkarılamadı.".to_string());
    }

    let video_bayt = std::fs::metadata(&video_yolu).map(|m| m.len()).unwrap_or(0);
    let poster_bayt = std::fs::metadata(&poster_yolu)
        .map(|m| m.len())
        .unwrap_or(0);
    let (genislik, yukseklik) = boyut_oku(&video_yolu).unwrap_or((1280, 720));
    let atif = {
        let parcalar: Vec<&str> = [istek.cekim.trim(), istek.lisans.trim()]
            .into_iter()
            .filter(|parca| !parca.is_empty())
            .collect();
        if !parcalar.is_empty() {
            parcalar.join(" · ")
        } else if !istek.kaynak_linki.trim().is_empty() {
            format!("Kaynak: {}", istek.kaynak_linki.trim())
        } else {
            "Yerel çekim".to_string()
        }
    };
    let kullanilan_sure = VIDEO_SN.min((sure - baslangic).max(1.0));
    let kayit = serde_json::json!({
        "saglayici": "Yerel dosya",
        "sahip": istek.cekim.trim(),
        "kaynak_sayfasi": istek.kaynak_linki.trim(),
        "lisans": istek.lisans.trim(),
        "atif": atif,
        "atif_gerekli": istek.atif_gerekli,
        "dosya": video_adi,
        "poster": poster_adi,
        "sure_sn": kullanilan_sure,
        "kaynak_sure_sn": sure,
        "baslangic_sn": baslangic,
        "genislik": genislik,
        "yukseklik": yukseklik,
        "video_bayt": video_bayt,
        "poster_bayt": poster_bayt,
        "tarih": tarih_metni(),
    });
    liste.push(kayit);
    json = serde_json::json!({"surum": 2, "videolar": liste});
    kaynak_yaz(&klasor, &json)?;

    Ok(Hazir {
        grup: grup_id.to_string(),
        dosya_adi: video_adi,
        poster_adi,
        video_bayt,
        poster_bayt,
        atif,
        sure: kullanilan_sure,
    })
}

/// Tek bir kaydı siler; dosyaları ve KAYNAK.json girdisini kaldırır.
pub fn sil_kayit(grup_id: &str, dosya_adi: &str) -> Result<(), String> {
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    let mut json = kaynak_oku(&klasor);
    let mut liste = kayit_listesi(&json);
    let onceki = liste.len();
    let mut poster_adi = String::new();
    liste.retain(|kayit| {
        if metin(kayit, "dosya") == dosya_adi {
            poster_adi = metin(kayit, "poster");
            false
        } else {
            true
        }
    });
    for ad in [dosya_adi.to_string(), poster_adi] {
        if ad.is_empty() {
            continue;
        }
        let yol = klasor.join(&ad);
        if let Err(hata) = std::fs::remove_file(&yol) {
            if hata.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("{ad} silinemedi: {hata}"));
            }
        }
    }
    if liste.len() == onceki {
        return Ok(());
    }
    json = serde_json::json!({"surum": 2, "videolar": liste});
    kaynak_yaz(&klasor, &json)
}

