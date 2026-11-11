from pathlib import Path

p = Path('src/main.rs')
s = p.read_text(encoding='utf-8')

s = s.replace('mod sektor_medya;', 'mod medya_havuz;\nmod sektor_medya;', 1)

eski = '''    let medya_slug = sektor_medya::slug(&isletme.category);
    let stok_kareler = sektor_medya::fotolar(&medya_slug);'''
yeni = '''    let medya_slug = sektor_medya::slug(&isletme.category);
    // Yerel medya havuzu: ağa çıkmadan diskteki gerçek video ve kareler
    // işletme tohumuyla dağıtılır. Aynı sektördeki iki işletme farklı
    // kombinasyon alır.
    let havuz = medya_havuz::dagit(
        &medya_slug,
        stok_video::tohum(&isletme.name, &isletme.address),
        4,
    );
    let havuz_kareleri: Vec<(String, String)> = havuz
        .kareler
        .iter()
        .filter_map(|yol| yerel_foto(&yol.to_string_lossy()))
        .collect();
    let havuzdan = !havuz_kareleri.is_empty();
    // Havuz kendi karelerini getiriyorsa atıf da havuzu gösterir.
    let kare_atif = Some(
        if havuzdan {
            "Yerel medya havuzu"
        } else {
            sektor_medya::ATIF
        }
        .to_string(),
    );
    let stok_kareler = if havuzdan {
        havuz_kareleri
    } else {
        sektor_medya::fotolar(&medya_slug)
    };'''
assert eski in s, 'medya slug blogu'
s = s.replace(eski, yeni, 1)

eski = '(Some(kare.clone()), Some(sektor_medya::ATIF.to_string()))'
assert eski in s, 'oto foto atif'
s = s.replace(eski, '(Some(kare.clone()), kare_atif.clone())', 1)

eski = '            Some(sektor_medya::ATIF.to_string()),'
assert eski in s, 'oto galeri atif'
s = s.replace(eski, '            kare_atif.clone(),', 1)

eski = '''                    .map(|bilgi| demo_site::VideoGirdi {
                        dosya: bilgi.dosya.clone(),
                        poster: bilgi.poster.clone(),
                        atif: bilgi.sayfa_atfi(),
                    })
            })
    } else {
        None
    };'''
yeni = '''                    .map(|bilgi| demo_site::VideoGirdi {
                        dosya: bilgi.dosya.clone(),
                        poster: bilgi.poster.clone(),
                        atif: bilgi.sayfa_atfi(),
                    })
            })
            // Yerel havuz: internet yok, diskteki gerçek döngü. Tohum hero
            // türünü seçer: aynı sektörde bir işletme videoyla, diğeri
            // hareketli fotoğrafla açılır.
            .or_else(|| {
                havuz
                    .video
                    .as_ref()
                    .filter(|_| havuz.video_hero)
                    .map(|_| demo_site::VideoGirdi {
                        dosya: "media/hero.mp4".to_string(),
                        poster: "media/poster.webp".to_string(),
                        atif: Some("Yerel medya havuzu".to_string()),
                    })
            })
    } else {
        None
    };'''
assert eski in s, 'video blogu'
s = s.replace(eski, yeni, 1)

eski = '''        video,
        brif,'''
assert eski in s, 'girdi video alani'
s = s.replace(
    eski,
    '''        video,
        medya_tohum: stok_video::tohum(&isletme.name, &isletme.address),
        brif,''',
    1,
)

eski = '''    let girdi = demo_girdi_profille(isletme, gonderen, false, profil_ez);
    let (kucuk, kirpilan) = demo_site::uret_butceli(&girdi, DEMO_BUTCE);'''
yeni = '''    let mut girdi = demo_girdi_profille(isletme, gonderen, false, profil_ez);
    // Müşteri kendi fotoğrafını seçtiyse yayın anında ffmpeg ile Ken Burns
    // döngüsüne çevrilir; demo hero'sunda gerçek hareketli görsel oynar.
    let musteri_video = if girdi.video.is_none() && girdi.profil.eq_ignore_ascii_case("sinematik")
    {
        musteri_video_hazirla(isletme)
    } else {
        None
    };
    if musteri_video.is_some() {
        girdi.video = Some(demo_site::VideoGirdi {
            dosya: "media/hero.mp4".to_string(),
            poster: "media/poster.webp".to_string(),
            atif: Some("Müşteri fotoğrafı".to_string()),
        });
    }
    let (kucuk, kirpilan) = demo_site::uret_butceli(&girdi, DEMO_BUTCE);'''
assert eski in s, 'girdi uretimi'
s = s.replace(eski, yeni, 1)

eski = '''        let stok_slug = sektor_medya::slug(&isletme.category);
        if let Some(bilgi) = sektor_medya::video(&stok_slug) {'''
yeni = '''        let stok_slug = sektor_medya::slug(&isletme.category);
        if let Some(kok) = &musteri_video {
            // Müşteri fotoğrafından üretilen döngü demo klasörüne kopyalanır.
            let _ = fs::copy(kok.join("hero.mp4"), format!("{medya}/hero.mp4"));
            let _ = fs::copy(kok.join("poster.webp"), format!("{medya}/poster.webp"));
            let _ = fs::copy(kok.join("KAYNAK.json"), format!("{medya}/KAYNAK.json"));
        } else if let Some(bilgi) = sektor_medya::video(&stok_slug) {'''
assert eski in s, 'kopyalama basi'
s = s.replace(eski, yeni, 1)

eski = '''            let _ = fs::copy(
                format!("{kaynak}/KAYNAK.json"),
                format!("{medya}/KAYNAK.json"),
            );
        }
        demo_kaynagi_yaz(klasor, &dosya_adi, isletme, gonderen);'''
yeni = '''            let _ = fs::copy(
                format!("{kaynak}/KAYNAK.json"),
                format!("{medya}/KAYNAK.json"),
            );
        } else if let Some(kayit) = medya_havuz::hero_video(
            &stok_slug,
            stok_video::tohum(&isletme.name, &isletme.address),
        ) {
            // Yerel havuz videosu: ağ yok, diskteki gerçek döngü.
            let _ = fs::copy(&kayit.dosya, format!("{medya}/hero.mp4"));
            if let Some(poster) = &kayit.poster {
                let _ = fs::copy(poster, format!("{medya}/poster.webp"));
            }
        }
        demo_kaynagi_yaz(klasor, &dosya_adi, isletme, gonderen);'''
assert eski in s, 'kopyalama sonu'
s = s.replace(eski, yeni, 1)

eski = 'fn demo_uret(isletme: &BusinessLead, gonderen: &str) -> Result<String, String> {'
yeni = '''/// Panelde "dosya" seçilmiş hero fotoğrafı varsa yayın anında Ken Burns
/// döngüsüne çevirir ve kopyalanacak klasörü döndürür.
fn musteri_video_hazirla(isletme: &BusinessLead) -> Option<std::path::PathBuf> {
    let ayar = ayar_oku();
    if ayar.demo_hero_foto.trim() != "dosya" {
        return None;
    }
    let kaynak = ayar.demo_hero_dosya.trim();
    if kaynak.is_empty() || !foto_dosyasi_gecerli(kaynak) {
        return None;
    }
    let tohum = stok_video::tohum(&isletme.name, &isletme.address);
    let klasor = std::path::Path::new("media/musteri").join(format!("{tohum:016x}"));
    if !klasor.join("hero.mp4").is_file() {
        std::fs::create_dir_all(&klasor).ok()?;
        stok_video::foto_ken_burns_hedef(kaynak, &klasor, tohum).ok()?;
    }
    klasor.join("hero.mp4").is_file().then_some(klasor)
}

fn demo_uret(isletme: &BusinessLead, gonderen: &str) -> Result<String, String> {'''
assert eski in s, 'demo_uret'
s = s.replace(eski, yeni, 1)

p.write_text(s, encoding='utf-8')
print('main guncellendi')
