from pathlib import Path

yapilan = []

def yaz(yol, eski, yeni):
    p = Path(yol)
    s = p.read_text(encoding='utf-8')
    if eski not in s:
        raise SystemExit(f'BULUNAMADI ({yol}): ' + eski[:100])
    p.write_text(s.replace(eski, yeni, 1), encoding='utf-8')
    yapilan.append(yol)

# --- demo_site: aci2 kullanımı -------------------------------------------
yaz('src/demo_site.rs', '--sn-vinyet:{vinyet};--sn-aci:{aci}"',
    '--sn-vinyet:{vinyet};--sn-aci:{aci};--sn-aci2:{aci2}"')
yaz('src/demo_site.rs', '        aci = dunya.aci,\n    ));',
    '        aci = dunya.aci,\n        aci2 = dunya.aci2,\n    ));')

# --- sektor_medya testi ---------------------------------------------------
yaz('src/sektor_medya.rs', '    #[test]\n    fn varsayilan_secim_bir_video_uc_fotograf() {',
    '''    #[test]
    fn dis_medya_varsayilan_olarak_kapali() {
        assert!(!DIS_KAYNAK_ACIK, "harici stok medya kapalı olmalı");
        let sm = SektorMedya::yeni("Kuaför", "Güzellik & Bakım");
        let kok = std::env::temp_dir().join(format!("nexus_kapali_{}", toplam()));
        assert!(fotolar_yoldan(&kok, "kuafor").is_empty());
        assert!(video_yoldan(&kok, "kuafor").is_none());
        assert!(onayla(&sm).is_err(), "indirme kapalı olmalı");
        assert_eq!(onayli_medya_sayisi(), 0);
        assert_eq!(tara(&sm).asama, Asama::Hata);
    }

    #[test]
    fn varsayilan_secim_bir_video_uc_fotograf() {''')

# --- stok_video: Ken Burns ----------------------------------------------
yaz('src/stok_video.rs', '/// `KAYNAK.json` dosyasını klasöre yazar.', '''/// Müşteri fotoğrafını Ken Burns (zoom + pan) sinematik döngüye çevirir:
/// 720p, sessiz, 8 sn, her kare anahtar kare; poster de üretilir.
/// Dış kaynak kullanılmaz, yalnız yerel dosya ve ffmpeg gerekir.
pub fn foto_ken_burns(grup_id: &str, kaynak_yol: &str) -> Result<Hazir, String> {
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
    let klasor = Path::new(MEDYA_KOK).join(grup_id);
    std::fs::create_dir_all(&klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    let video = klasor.join("hero.mp4");
    let poster = klasor.join("poster.webp");
    // Önce büyüt, sonra zoompan: 720p çıktıda titreme olmaz.
    let filtre = "scale=2560:-2,zoompan=z='min(1.0+0.0011*on,1.28)':x='iw/2-(iw/zoom)/2+sin(on/70)*18':y='ih/2-(ih/zoom)/2-cos(on/90)*14':d=200:s=1280x720:fps=25,format=yuv420p";
    let durum = std::process::Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-loop", "1", "-i"])
        .arg(kaynak)
        .args([
            "-t", "8", "-vf", filtre, "-an", "-c:v", "libx264", "-preset", "medium", "-crf", "23",
            "-g", "1", "-keyint_min", "1", "-sc_threshold", "0", "-movflags", "+faststart",
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
        "sure_sn": VIDEO_SN,
        "genislik": 1280,
        "yukseklik": 720,
        "video_bayt": video_bayt,
        "poster_bayt": poster_bayt,
        "tarih": tarih_metni(),
    });
    kaynak_yaz(&klasor, &bilgi)?;
    Ok(Hazir {
        grup: grup_id.to_string(),
        dosya_adi: "hero.mp4".to_string(),
        poster_adi: "poster.webp".to_string(),
        video_bayt,
        poster_bayt,
        atif: "Müşteri fotoğrafı".to_string(),
        sure: VIDEO_SN,
    })
}

/// `KAYNAK.json` dosyasını klasöre yazar.''')

yaz('src/stok_video.rs', 'mod testler {\n    use super::*;\n', '''mod testler {
    use super::*;

    #[test]
    #[ignore]
    fn foto_ken_burns_uretir() {
        let kaynak = "media/stock/kuafor/photos/1.jpg";
        if !Path::new(kaynak).is_file() {
            println!("örnek fotoğraf yok; test atlandı");
            return;
        }
        let hazir = foto_ken_burns("deneme_kb", kaynak).expect("ken burns");
        println!(
            "video {} B · poster {} B",
            hazir.video_bayt, hazir.poster_bayt
        );
        assert!(hazir.video_bayt > 40_000, "video üretilmedi");
        assert!(hazir.poster_bayt > 2_000, "poster üretilmedi");
        assert!(Path::new("media/sektor/deneme_kb/KAYNAK.json").is_file());
        let _ = std::fs::remove_dir_all("media/sektor/deneme_kb");
    }
''')

# --- main.rs: alan, ilk değer, panel UI ----------------------------------
yaz('src/main.rs', '    /// Stok video bulucu (yerel içe aktarma): seçili grup, dosya yolu,',
    '    /// Müşteri fotoğrafından Ken Burns döngüsü için dosya yolu.\n'
    '    stok_foto_yol: String,\n'
    '    /// Stok video bulucu (yerel içe aktarma): seçili grup, dosya yolu,')
yaz('src/main.rs', '            stok_grup: stok_video::GRUPLAR[0].id.to_string(),',
    '            stok_foto_yol: String::new(),\n'
    '            stok_grup: stok_video::GRUPLAR[0].id.to_string(),')

yaz('src/main.rs', '        // Sürükle-bırak: pencereye bırakılan dosya yol alanına düşer.', '''        // Fotoğraftan sinematik döngü: yerel dosya, ffmpeg, dış kaynak yok.
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Fotoğraftan sinematik döngü · Ken Burns zoom-pan · 720p · 8 sn · sessiz",
            )
            .size(9.8)
            .color(DIM),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.stok_foto_yol)
                    .hint_text("C:\\\\fotoğraflar\\\\cephe.jpg")
                    .desired_width(190.0),
            );
            if ui
                .add_enabled(!self.stok_mesgul, egui::Button::new("Hareketlendir"))
                .clicked()
            {
                let grup = self.stok_grup.clone();
                let yol = self.stok_foto_yol.trim().to_string();
                if yol.is_empty() {
                    self.stok_durum = "Önce bir fotoğraf yolu girin.".to_string();
                } else {
                    let (tx, rx) = channel();
                    self.stok_islem = Some(rx);
                    self.stok_mesgul = true;
                    self.stok_durum =
                        "Fotoğraf hareketli döngüye çevriliyor…".to_string();
                    thread::spawn(move || {
                        match stok_video::foto_ken_burns(&grup, &yol) {
                            Ok(hazir) => {
                                let _ = tx.send(StokOlay::Hazir(hazir));
                            }
                            Err(hata) => {
                                let _ = tx.send(StokOlay::Hata(hata));
                            }
                        }
                        let _ = tx.send(StokOlay::Bitti);
                    });
                }
            }
        });

        // Sürükle-bırak: pencereye bırakılan dosya yol alanına düşer.''')

yaz('src/main.rs', '        bolum_etiketi(ui, "SEKTÖREL MEDYA TARAMASI");', '''        bolum_etiketi(ui, "SEKTÖREL MEDYA");
        if !sektor_medya::DIS_KAYNAK_ACIK {
            ui.label(
                egui::RichText::new(
                    "Dış medya kapalı: arka planlar, dokular ve geçişler işletme tohumundan kod tabanlı üretiliyor. Arama/indirme yapılmaz.",
                )
                .size(9.6)
                .color(DIM),
            );
            return;
        }
        bolum_etiketi(ui, "SEKTÖREL MEDYA TARAMASI");''')

print('tamam:', yapilan)
