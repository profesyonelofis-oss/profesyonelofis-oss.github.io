from pathlib import Path

# --- 1) stok_video: hedefli, tohumlu Ken Burns -----------------------------
p = Path('src/stok_video.rs')
s = p.read_text(encoding='utf-8')
bas = s.index('/// Musteri' if '/// Musteri' in s else '/// Müşteri fotoğrafını Ken Burns')
son = s.index('/// `KAYNAK.json` dosyasını klasöre yazar.')
yeni = '''/// Müşteri fotoğrafını Ken Burns (zoom + pan) sinematik döngüye çevirir:
/// 720p, sessiz, 8 sn, her kare anahtar kare; poster de üretilir.
/// Dış kaynak kullanılmaz, yalnız yerel dosya ve ffmpeg gerekir.
pub fn foto_ken_burns(grup_id: &str, kaynak_yol: &str) -> Result<Hazir, String> {
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    foto_ken_burns_hedef(kaynak_yol, &klasor, 0)
}

/// Aynı dönüşüm, seçilen klasöre: müşteri fotoğrafı yayın anında demo
/// klasörüne yazılır. `tohum` zoom hedefini, salınımı ve yönü belirler;
/// aynı fotoğraf iki işletmede aynı hareketi tekrarlamaz.
pub fn foto_ken_burns_hedef(
    kaynak_yol: &str,
    klasor: &Path,
    tohum: u64,
) -> Result<Hazir, String> {
    if !ffmpeg_var() {
        return Err("ffmpeg bulunamadı: fotoğraf hareketlendirilemiyor.".to_string());
    }
    let kaynak = Path::new(kaynak_yol.trim());
    if !kaynak.is_file() {
        return Err("Fotoğraf bulunamadı: dosya yolunu kontrol edin.".to_string());
    }
    let uzanti = kaynak
        .extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(uzanti.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        return Err("Yalnız jpg, jpeg, png veya webp fotoğraf kullanılabilir.".to_string());
    }
    std::fs::create_dir_all(klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    let video = klasor.join("hero.mp4");
    let poster = klasor.join("poster.webp");
    // Önce büyüt, sonra zoompan: 720p çıktıda titreme olmaz. Zoom hedefi,
    // salınım ve yön tohumdan gelir.
    let zum = 1.10 + ((tohum >> 7) % 15) as f64 * 0.012;
    let salinim = 14.0 + ((tohum >> 11) % 12) as f64;
    let yon = if (tohum >> 3) & 1 == 0 { 1.0 } else { -1.0 };
    let filtre = format!(
        "scale=2560:-2,zoompan=z='min(1.0+0.0011*on,{zum:.3})':x='iw/2-(iw/zoom)/2+sin(on/70)*{salinim:.0}*{yon:.0}':y='ih/2-(ih/zoom)/2-cos(on/90)*{salinim:.0}':d=200:s=1280x720:fps=25,format=yuv420p"
    );
    let durum = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-loop", "1", "-i"])
        .arg(kaynak)
        .args([
            "-t",
            "8",
            "-vf",
            &filtre,
            "-an",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-g",
            "1",
            "-keyint_min",
            "1",
            "-sc_threshold",
            "0",
            "-movflags",
            "+faststart",
        ])
        .arg(&video)
        .status()
        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        return Err("Fotoğraf hareketli videoya çevrilemedi.".to_string());
    }
    let durum = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-ss", "0.5", "-i"])
        .arg(&video)
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
        .arg(&poster)
        .status()
        .map_err(|hata| format!("ffmpeg (poster) çalıştırılamadı: {hata}"))?;
    if !durum.success() {
        return Err("Poster görüntüsü çıkarılamadı.".to_string());
    }
    let video_bayt = std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0);
    let poster_bayt = std::fs::metadata(&poster).map(|m| m.len()).unwrap_or(0);
    let bilgi = serde_json::json!({
        "kaynak": "müşteri fotoğrafı",
        "dosya": "hero.mp4",
        "poster": "poster.webp",
        "efekt": "ken-burns",
        "tohum": tohum,
        "sure_sn": VIDEO_SN,
        "genislik": 1280,
        "yukseklik": 720,
        "video_bayt": video_bayt,
        "poster_bayt": poster_bayt,
        "tarih": tarih_metni(),
    });
    kaynak_yaz(klasor, &bilgi)?;
    Ok(Hazir {
        grup: klasor
            .file_name()
            .and_then(|ad| ad.to_str())
            .unwrap_or("musteri")
            .to_string(),
        dosya_adi: "hero.mp4".to_string(),
        poster_adi: "poster.webp".to_string(),
        video_bayt,
        poster_bayt,
        atif: "Müşteri fotoğrafı".to_string(),
        sure: VIDEO_SN,
    })
}

'''
s = s[:bas] + yeni + s[son:]
p.write_text(s, encoding='utf-8')
print('stok_video guncellendi')
