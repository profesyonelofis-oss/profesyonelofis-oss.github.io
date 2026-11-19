//! Medya onayı ve arşiv bakım komutları.
//!
//! Otomatik tedarik kaldırıldı; medya yalnız `--onizleme` panelinden eklenir.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::medya_arsiv;

/// Tekilleştirme çıktısı (havuzdan çıkarılanlar buraya taşınır).
const TEKRAR_KLASOR: &str = "media/tekrar";
/// İki görselin "benzer" sayıldığı en fazla farklı bit (64 üzerinden ≈%92).
const BENZER_BIT: u32 = 5;

/// Aktif komutları yürütür. Argüman yoksa GUI açılır.
pub fn komut_satiri() -> Option<i32> {
    let arguman = std::env::args().skip(1).find(|a| a.starts_with("--"))?;
    Some(match arguman.as_str() {
        "--bos-sektorler" => bos_sektorler_listesi(),
        "--medya-tekille" => medya_tekille(std::env::args().any(|a| a == "--uygula")),
        "--sektor-denetim" => sektor_denetim(),
        "--onizleme" => onizleme(),
        "--onay-kuyrugu" => onay_kuyrugu(),
        "--onayla" => onay_isle(true),
        "--reddet" => onay_isle(false),
        "--onay-durum" => onay_durum(),
        "--dogrula-ornek" => {
            let adet = std::env::args()
                .skip_while(|a| a != "--dogrula-ornek")
                .nth(1)
                .and_then(|deger| deger.parse().ok())
                .unwrap_or(10);
            dogrula_ornek(adet)
        }
        _ => {
            eprintln!("bilinmeyen komut: {arguman}");
            2
        }
    })
}

/// `--bos-sektorler`: hiç medyası olmayan sektörleri listeler.
fn bos_sektorler_listesi() -> i32 {
    let arsiv = proje_koku().join(crate::medya_arsiv::KLASOR);
    let bos = crate::medya_tedarik::bos_sektorler(&arsiv);
    println!(
        "[BOŞ SEKTÖRLER] {} sektörde hiç video/fotoğraf yok",
        bos.len()
    );
    for (sira, ad) in bos.iter().enumerate() {
        println!(
            "  {:>3}. {} · {}",
            sira + 1,
            ad,
            crate::sektor_medya::slug(ad)
        );
    }
    0
}

/// Uzantıdan medya türü: `Some(true)` video, `Some(false)` foto, `None` medya değil.
fn medya_uzantisi(yol: &Path) -> Option<bool> {
    let uzanti = yol.extension()?.to_str()?.to_lowercase();
    Some(match uzanti.as_str() {
        "mp4" | "webm" | "mov" | "mkv" | "m4v" => true,
        "webp" | "jpg" | "jpeg" | "png" | "gif" | "avif" => false,
        _ => return None,
    })
}

/// Arşivdeki bütün medya dosyaları: (yol, video mu?).
fn arsiv_medyalari(arsiv: &Path) -> Vec<(PathBuf, bool)> {
    let mut liste = Vec::new();
    let Ok(sektorler) = fs::read_dir(arsiv) else {
        return liste;
    };
    for sektor in sektorler.filter_map(Result::ok) {
        let sektor_kok = sektor.path();
        if !sektor_kok.is_dir() {
            continue;
        }
        for alt in ["photos", "video"] {
            let Ok(dosyalar) = fs::read_dir(sektor_kok.join(alt)) else {
                continue;
            };
            for giris in dosyalar.filter_map(Result::ok) {
                let dosya = giris.path();
                if !dosya.is_file() {
                    continue;
                }
                if let Some(video) = medya_uzantisi(&dosya) {
                    liste.push((dosya, video));
                }
            }
        }
    }
    liste.sort();
    liste
}

/// Dosyanın SHA-256 özeti (birebir tekrar tespiti).
fn sha256(yol: &Path) -> Option<[u8; 32]> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut dosya = fs::File::open(yol).ok()?;
    let mut ozet = Sha256::new();
    let mut tampon = vec![0u8; 256 * 1024];
    loop {
        let okunan = dosya.read(&mut tampon).ok()?;
        if okunan == 0 {
            break;
        }
        ozet.update(&tampon[..okunan]);
    }
    Some(ozet.finalize().into())
}

/// 8x8 ortalama hash: 64 bit görsel parmak izi (küçük yeniden boyutlandırma,
/// sıkıştırma ve hafif kırpma farklarına dayanıklı).
fn gorsel_hash(yol: &Path) -> Option<u64> {
    let kucuk = image::open(yol)
        .ok()?
        .resize_exact(8, 8, image::imageops::FilterType::Nearest)
        .to_luma8();
    let pikseller: Vec<u8> = kucuk.pixels().map(|nokta| nokta.0[0]).collect();
    if pikseller.is_empty() {
        return None;
    }
    let ortalama = pikseller.iter().map(|p| u32::from(*p)).sum::<u32>()
        / u32::try_from(pikseller.len()).ok()?;
    let mut hash = 0u64;
    for (sira, piksel) in pikseller.iter().enumerate() {
        if u32::from(*piksel) >= ortalama {
            hash |= 1u64 << sira;
        }
    }
    Some(hash)
}

/// `--medya-tekille [--uygula]`: birebir ve %90+ benzer medyayı tekilleştirir.
/// Kuru çalışmada yalnız rapor verir; `--uygula` ile fazlalıklar `media/tekrar`a taşınır.
fn medya_tekille(uygula: bool) -> i32 {
    let kok = proje_koku();
    let arsiv = kok.join(medya_arsiv::KLASOR);
    let tekrar = kok.join(TEKRAR_KLASOR);
    let medyalar = arsiv_medyalari(&arsiv);
    if medyalar.is_empty() {
        println!("[TEKİLLE] arşivde medya yok");
        return 0;
    }
    let toplam_bayt: u64 = medyalar
        .iter()
        .filter_map(|(yol, _)| fs::metadata(yol).ok())
        .map(|meta| meta.len())
        .sum();
    println!(
        "[TEKİLLE] {} dosya · {} MB · benzerlik eşiği {BENZER_BIT}/64 bit",
        medyalar.len(),
        toplam_bayt / (1024 * 1024)
    );
    let mut tam: std::collections::HashMap<[u8; 32], PathBuf> = std::collections::HashMap::new();
    let mut gorsel: Vec<(u64, PathBuf)> = Vec::new();
    let mut tekrarlar: Vec<(PathBuf, String, u64)> = Vec::new();
    let (mut foto, mut video) = (0usize, 0usize);
    for (sira, (dosya, video_mu)) in medyalar.iter().enumerate() {
        if sira > 0 && sira % 500 == 0 {
            println!("  … {sira}/{}", medyalar.len());
        }
        let boyut = fs::metadata(dosya).map(|meta| meta.len()).unwrap_or(0);
        if let Some(hash) = sha256(dosya) {
            if let Some(once) = tam.get(&hash) {
                tekrarlar.push((
                    dosya.clone(),
                    format!("birebir aynı: {}", once.display()),
                    boyut,
                ));
                if *video_mu {
                    video += 1;
                } else {
                    foto += 1;
                }
                continue;
            }
            tam.insert(hash, dosya.clone());
        }
        if *video_mu {
            continue;
        }
        let Some(hash) = gorsel_hash(dosya) else {
            continue;
        };
        let mut en_yakin: Option<(u32, PathBuf)> = None;
        for (onceki, yol) in &gorsel {
            let fark = (hash ^ onceki).count_ones();
            if fark <= BENZER_BIT && en_yakin.as_ref().map(|(f, _)| fark < *f).unwrap_or(true) {
                en_yakin = Some((fark, yol.clone()));
            }
        }
        if let Some((fark, once)) = en_yakin {
            tekrarlar.push((
                dosya.clone(),
                format!("{} · %{} benzer", once.display(), 100 - fark * 100 / 64),
                boyut,
            ));
            foto += 1;
            continue;
        }
        gorsel.push((hash, dosya.clone()));
    }
    let bayt: u64 = tekrarlar.iter().map(|(_, _, boyut)| *boyut).sum();
    println!(
        "[TEKİLLE] {} tekrar bulundu ({} foto · {} video) · {} MB",
        tekrarlar.len(),
        foto,
        video,
        bayt / (1024 * 1024)
    );
    for (dosya, neden, boyut) in tekrarlar.iter().take(20) {
        println!(
            "  {} · {:.1} MB · {neden}",
            dosya.display(),
            *boyut as f64 / (1024.0 * 1024.0)
        );
    }
    if tekrarlar.len() > 20 {
        println!("  … {} kayıt daha", tekrarlar.len() - 20);
    }
    if !uygula {
        println!("[TEKİLLE] kuru çalışma · uygulamak için: --medya-tekille --uygula");
        return 0;
    }
    let mut tasinan = 0usize;
    for (dosya, _, _) in &tekrarlar {
        let goreli = dosya.strip_prefix(&arsiv).unwrap_or(dosya);
        let hedef = tekrar.join(goreli);
        if let Some(ust) = hedef.parent() {
            let _ = fs::create_dir_all(ust);
        }
        if fs::rename(dosya, &hedef).is_ok() {
            tasinan += 1;
        } else if fs::copy(dosya, &hedef).is_ok() {
            let _ = fs::remove_file(dosya);
            tasinan += 1;
        }
    }
    println!("[TEKİLLE] {tasinan} dosya havuzdan çıkarıldı → {TEKRAR_KLASOR}");
    0
}

/// Tek sektörün medya sayısı (photos + video).
fn sektor_medya_sayisi(sektor_kok: &Path) -> usize {
    let mut sayi = 0usize;
    for alt in ["photos", "video"] {
        let Ok(dosyalar) = fs::read_dir(sektor_kok.join(alt)) else {
            continue;
        };
        sayi += dosyalar
            .filter_map(Result::ok)
            .filter(|giris| {
                let yol = giris.path();
                yol.is_file() && medya_uzantisi(&yol).is_some()
            })
            .count();
    }
    sayi
}

/// `--onizleme`: tarayıcı onay panelini açar ve süreci ayakta tutar.
fn onizleme() -> i32 {
    let kok = proje_koku();
    match crate::medya_web::baslat(kok) {
        Ok(()) => {
            // Panel dinlemeye başladı: varsayılan tarayıcı kendiliğinden açılır.
            let acildi = crate::medya_web::tarayici_ac();
            println!(
                "[ÖNİZLEME] {} · {} · kapatmak için Ctrl+C",
                crate::medya_web::ADRES,
                if acildi {
                    "tarayıcı açıldı"
                } else {
                    "tarayıcı açılamadı, adresi elle açın"
                }
            );
        }
        Err(hata) => {
            // Panel zaten çalışıyorsa ikinci sunucu yerine sekme öne getirilir.
            if crate::medya_web::hazir_mi() {
                println!(
                    "[ÖNİZLEME] panel zaten çalışıyor: {} · tarayıcı açılıyor",
                    crate::medya_web::ADRES
                );
                let _ = crate::medya_web::tarayici_ac();
                return 0;
            }
            eprintln!(
                "[ÖNİZLEME] sunucu açılamadı (127.0.0.1:{}): {hata}",
                crate::medya_web::PORT
            );
            return 1;
        }
    }
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

/// Onay komutundan sonraki sektör adı.
fn onay_sektor_argumani(bayrak: &str) -> Option<String> {
    let mut argumanlar = std::env::args().skip(1);
    while let Some(arguman) = argumanlar.next() {
        if arguman == bayrak {
            return argumanlar.next();
        }
    }
    None
}

/// `--onay-kuyrugu`: onay bekleyen medyayı ve durumları listeler.
fn onay_kuyrugu() -> i32 {
    let kok = proje_koku();
    let kuyruk = crate::medya_onay::kuyruk(&kok);
    let ozet = crate::medya_onay::ozet(&kok);
    println!("=========================================================");
    println!(" İNSAN ONAYLI MEDYA HAVUZU · 2 video + 2 fotoğraf");
    println!("=========================================================");
    println!("  bekleyen : {} sektör", ozet.bekleyen);
    println!("  onaylı   : {} sektör", ozet.onayli);
    println!("  red      : {} sektör", ozet.red);
    if kuyruk.is_empty() {
        println!("  kuyruk boş: tedarik hattı henüz medya yazmadı");
        return 0;
    }
    for (slug, kayit) in kuyruk {
        println!(
            "  {slug:<22} {:<14} {} video + {} foto",
            kayit.durum.etiket(),
            kayit.videos.len(),
            kayit.fotolar.len()
        );
    }
    println!("  onay: --onayla <sektör> · red ve silme: --reddet <sektör>");
    0
}

/// `--onayla <sektör>` / `--reddet <sektör>`.
fn onay_isle(onay: bool) -> i32 {
    let kok = proje_koku();
    let bayrak = if onay { "--onayla" } else { "--reddet" };
    let Some(hedef) = onay_sektor_argumani(bayrak) else {
        eprintln!("sektör belirtilmedi: {bayrak} <sektör>");
        return 2;
    };
    let slug = crate::sektor_medya::slug(&hedef);
    if onay {
        match crate::medya_onay::onayla(&kok, &slug) {
            Ok(kayit) => {
                println!(
                    "[ONAY] {slug} · {} dosya açıldı · demo üretimi artık serbest",
                    kayit.dosya_sayisi()
                );
                0
            }
            Err(hata) => {
                eprintln!("[ONAY HATASI] {hata}");
                1
            }
        }
    } else {
        match crate::medya_onay::reddet(&kok, &slug) {
            Ok(silinen) => {
                println!(
                    "[RED] {slug} · {} dosya diskten silindi · sektör yeniden aranacak",
                    silinen.len()
                );
                0
            }
            Err(hata) => {
                eprintln!("[RED HATASI] {hata}");
                1
            }
        }
    }
}

/// `--onay-durum <sektör>`: tek sektörün onay durumu ve dosyaları.
fn onay_durum() -> i32 {
    let kok = proje_koku();
    let Some(hedef) = onay_sektor_argumani("--onay-durum") else {
        return onay_kuyrugu();
    };
    let slug = crate::sektor_medya::slug(&hedef);
    let kayit = crate::medya_onay::kayit(&kok, &slug);
    println!(
        "[ONAY DURUM] {slug} · {} · {} video + {} foto",
        kayit.durum.etiket(),
        kayit.videos.len(),
        kayit.fotolar.len()
    );
    for goreli_yol in kayit.yollar() {
        println!("  {goreli_yol}");
    }
    0
}

fn sektor_denetim() -> i32 {
    let arsiv = proje_koku().join(medya_arsiv::KLASOR);
    let mut tablo: std::collections::BTreeMap<String, &'static str> =
        std::collections::BTreeMap::new();
    for tanim in crate::SEKTORLER {
        tablo.insert(crate::sektor_medya::slug(tanim.ad), tanim.grup);
    }
    let mut disk: Vec<String> = Vec::new();
    if let Ok(liste) = fs::read_dir(&arsiv) {
        for giris in liste.filter_map(Result::ok) {
            if giris.path().is_dir() {
                disk.push(giris.file_name().to_string_lossy().to_string());
            }
        }
    }
    disk.sort();
    let bos: Vec<&String> = disk
        .iter()
        .filter(|ad| sektor_medya_sayisi(&arsiv.join(ad.as_str())) == 0)
        .collect();
    let fazla: Vec<&String> = disk
        .iter()
        .filter(|ad| !tablo.contains_key(ad.as_str()))
        .collect();
    let eksik: Vec<&String> = tablo
        .keys()
        .filter(|ad| !disk.iter().any(|d| d == *ad))
        .collect();
    println!("=========================================================");
    println!(" SEKTÖR DENETİMİ");
    println!("=========================================================");
    println!("  tablo (exe)        : {} sektör", tablo.len());
    println!("  diskteki klasör    : {} klasör", disk.len());
    println!("  tamamen boş klasör : {}", bos.len());
    for ad in bos.iter().take(20) {
        println!("    boş: {ad}");
    }
    println!("  tabloda olmayan    : {}", fazla.len());
    for ad in fazla.iter().take(20) {
        println!("    fazla: {ad}");
    }
    println!("  diskte olmayan     : {}", eksik.len());
    for ad in eksik.iter().take(20) {
        println!("    eksik: {ad}");
    }
    let onay = crate::medya_onay::ozet(&proje_koku());
    println!("  onay bekleyen      : {}", onay.bekleyen);
    println!("  onaylı sektör      : {}", onay.onayli);
    println!("  reddedilen sektör  : {}", onay.red);
    let mut gruplar: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for grup in tablo.values() {
        *gruplar.entry(*grup).or_default() += 1;
    }
    println!("  grup sayısı        : {}", gruplar.len());
    let mut sirali: Vec<(&str, usize)> = gruplar.into_iter().collect();
    sirali.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (grup, sayi) in sirali.iter().take(15) {
        println!("    {grup:<30} {sayi}");
    }
    0
}

/// `--dogrula-ornek <N>`: arşivden eşit aralıklı örnek alır, sıkı eşikle
/// doğrular ve yalnız raporlar (hiçbir dosyayı taşımaz/silmez).
fn dogrula_ornek(adet: usize) -> i32 {
    let arsiv = proje_koku().join(medya_arsiv::KLASOR);
    let medyalar = arsiv_medyalari(&arsiv);
    if medyalar.is_empty() {
        println!("[ÖRNEK] arşivde medya yok");
        return 0;
    }
    let adet = adet.clamp(1, 200);
    let adim = (medyalar.len() / adet).max(1);
    let esik = std::env::var("DOGRULAMA_GUVEN_ESIK").unwrap_or_else(|_| "90".to_string());
    println!(
        "[ÖRNEK DOĞRULAMA] {} dosyadan {} örnek · güven eşiği %{esik} · taşıma yok",
        medyalar.len(),
        adet
    );
    let (mut onay, mut red, mut yok) = (0usize, 0usize, 0usize);
    let mut sira = 0usize;
    let mut sayac = 0usize;
    while sayac < adet && sira < medyalar.len() {
        let (dosya, video_mu) = &medyalar[sira];
        sira += adim;
        sayac += 1;
        let goreli = dosya
            .strip_prefix(&arsiv)
            .unwrap_or(dosya)
            .to_string_lossy()
            .replace('\\', "/");
        let sektor = goreli.split('/').next().unwrap_or_default().to_string();
        let etiket = crate::medya_dogrulama::slug_etiketi(&sektor);
        let sonuc = crate::medya_dogrulama::dogrula_dosya(dosya, &etiket, *video_mu);
        let (durum, ayrinti) = match &sonuc {
            crate::medya_dogrulama::Sonuc::Onay { kare, skor } => {
                onay += 1;
                ("ONAY".to_string(), format!("{kare} kare · %{skor}"))
            }
            crate::medya_dogrulama::Sonuc::Red { cevaplar, .. } => {
                red += 1;
                ("RED".to_string(), cevaplar.join(" | "))
            }
            crate::medya_dogrulama::Sonuc::Yok { neden } => {
                yok += 1;
                ("YOK".to_string(), neden.clone())
            }
        };
        println!(
            "[{sayac}/{adet}] {goreli} · {} · {durum} · {ayrinti}",
            if *video_mu { "video" } else { "foto" }
        );
    }
    println!("[ÖRNEK ÖZET] {onay} onay · {red} red · {yok} doğrulanamadı");
    0
}

/// Proje kökü: `media/stock` içeren dizin. Exe `target/release` altından
/// çalıştırılsa bile doğru kökü bulur.
pub(crate) fn proje_koku() -> PathBuf {
    if let Ok(simdiki) = std::env::current_dir() {
        if simdiki.join("media/stock").is_dir() {
            return simdiki;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        for ust in exe.ancestors().skip(1).take(6) {
            if ust.join("media/stock").is_dir() {
                return ust.to_path_buf();
            }
            if ust.join("Cargo.toml").is_file() {
                return ust.to_path_buf();
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
