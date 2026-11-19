"""Servis bazlı bulut metrikleri, anlık uyandırma ve birebir UI metinleri."""

import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return Path(yol).read_text(encoding="utf-8")


def yaz(yol, metin):
    Path(yol).write_text(metin, encoding="utf-8", newline="")


def degistir(metin, eski, yeni):
    assert metin.count(eski) == 1, f"beklenen 1, bulunan {metin.count(eski)}: {eski[:90]!r}"
    return metin.replace(eski, yeni)


g = oku("src/google.rs")

# --- 1) Kota: servis bazlı bulut alanları ------------------------------------
g = degistir(
    g,
    """    /// Son senkron hatası (panelde gösterilir, sayaçları değiştirmez).
    #[serde(default)]
    pub bulut_hata: Option<String>,
}""",
    """    /// Buluttan gelen Places istek sayısı (bu ay).
    #[serde(default)]
    pub bulut_places: Option<usize>,
    /// Buluttan gelen Geocoding istek sayısı (bu ay).
    #[serde(default)]
    pub bulut_geocode: Option<usize>,
    /// Son senkron hatası (panelde gösterilir, sayaçları değiştirmez).
    #[serde(default)]
    pub bulut_hata: Option<String>,
}""",
)

# --- 2) Ozet: servis kırılımı -------------------------------------------------
g = degistir(
    g,
    """    pub bulut_zaman: Option<String>,
    pub bulut_hata: Option<String>,
}""",
    """    pub bulut_zaman: Option<String>,
    pub bulut_hata: Option<String>,
    /// Bulut aylık kırılımı: Places ve Geocoding ayrı faturalanır.
    pub bulut_places: Option<usize>,
    pub bulut_geocode: Option<usize>,
}""",
)

g = degistir(
    g,
    """            bulut_zaman: kota.bulut_zaman.clone(),
            bulut_hata: kota.bulut_hata.clone(),
        }""",
    """            bulut_zaman: kota.bulut_zaman.clone(),
            bulut_hata: kota.bulut_hata.clone(),
            bulut_places: kota.bulut_places,
            bulut_geocode: kota.bulut_geocode,
        }""",
)

# --- 3) BulutOzet: servis kırılımı -------------------------------------------
g = degistir(
    g,
    """pub struct BulutOzet {
    pub gunluk: usize,
    pub aylik: usize,
    /// Senkron saati (yerel, HH:MM:SS).
    pub zaman: String,
}""",
    """pub struct BulutOzet {
    pub gunluk: usize,
    pub aylik: usize,
    /// Bu ayın Places (New) istek sayısı.
    pub places: usize,
    /// Bu ayın Geocoding istek sayısı.
    pub geocode: usize,
    /// Senkron saati (yerel, HH:MM:SS).
    pub zaman: String,
}""",
)

# --- 4) Sorgu: servis etiketini koru (crossSeriesReducer kaldırıldı) ---------
g = degistir(
    g,
    """        ("aggregation.perSeriesAligner", "ALIGN_SUM".to_string()),
        ("aggregation.crossSeriesReducer", "REDUCE_SUM".to_string()),
        ("key", anahtar.to_string()),""",
    """        ("aggregation.perSeriesAligner", "ALIGN_SUM".to_string()),
        // Seri bazında toplanır: resource.labels.service ile Places/Geocoding
        // ayrımı korunur, böylece panelde servis kırılımı görünür.
        ("key", anahtar.to_string()),""",
)

g = degistir(
    g,
    """    let bugun = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let (gunluk, aylik) = bulut_topla(&yanit, &bugun);
    Ok(BulutOzet {
        gunluk,
        aylik,
        zaman: chrono::Local::now().format("%H:%M:%S").to_string(),
    })""",
    """    let bugun = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let (gunluk, aylik, places, geocode) = bulut_topla(&yanit, &bugun);
    Ok(BulutOzet {
        gunluk,
        aylik,
        places,
        geocode,
        zaman: chrono::Local::now().format("%H:%M:%S").to_string(),
    })""",
)

# --- 5) Seri modeli: resource.labels.service ---------------------------------
g = degistir(
    g,
    """#[derive(Debug, Default, serde::Deserialize)]
struct BulutSeri {
    #[serde(default)]
    points: Vec<BulutNokta>,
}""",
    """#[derive(Debug, Default, serde::Deserialize)]
struct BulutSeri {
    #[serde(default)]
    resource: BulutKaynak,
    #[serde(default)]
    points: Vec<BulutNokta>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct BulutKaynak {
    #[serde(default)]
    labels: std::collections::HashMap<String, String>,
}""",
)

# --- 6) Toplama: servis kırılımı ---------------------------------------------
g = degistir(
    g,
    """fn bulut_topla(yanit: &BulutYanit, bugun: &str) -> (usize, usize) {
    let mut gunluk = 0.0f64;
    let mut aylik = 0.0f64;
    for seri in &yanit.time_series {
        for nokta in &seri.points {
            let deger = nokta.value.sayi();
            aylik += deger;
            if nokta.interval.start.get(..10) == Some(bugun) {
                gunluk += deger;
            }
        }
    }
    (gunluk.round() as usize, aylik.round() as usize)
}""",
    """fn bulut_topla(yanit: &BulutYanit, bugun: &str) -> (usize, usize, usize, usize) {
    let mut gunluk = 0.0f64;
    let mut aylik = 0.0f64;
    let mut places = 0.0f64;
    let mut geocode = 0.0f64;
    for seri in &yanit.time_series {
        let servis = seri
            .resource
            .labels
            .get("service")
            .map(String::as_str)
            .unwrap_or("");
        let mut seri_toplam = 0.0f64;
        for nokta in &seri.points {
            let deger = nokta.value.sayi();
            seri_toplam += deger;
            if nokta.interval.start.get(..10) == Some(bugun) {
                gunluk += deger;
            }
        }
        aylik += seri_toplam;
        if servis.starts_with("places") {
            places += seri_toplam;
        } else if servis.starts_with("geocoding") {
            geocode += seri_toplam;
        }
    }
    (
        gunluk.round() as usize,
        aylik.round() as usize,
        places.round() as usize,
        geocode.round() as usize,
    )
}""",
)

# --- 7) Kalıcı yazma + servis alanları ---------------------------------------
g = degistir(
    g,
    """    sayac.bulut_gunluk = Some(sonuc.gunluk);
    sayac.bulut_aylik = Some(sonuc.aylik);""",
    """    sayac.bulut_gunluk = Some(sonuc.gunluk);
    sayac.bulut_aylik = Some(sonuc.aylik);
    sayac.bulut_places = Some(sonuc.places);
    sayac.bulut_geocode = Some(sonuc.geocode);""",
)

# --- 8) Hata günlüğü dosyası --------------------------------------------------
g = degistir(
    g,
    """/// Senkron hatasını panelde göstermek için saklar; sayaçlara dokunmaz.
pub fn bulut_hata_yaz(hata: &str) {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_hata = Some(hata.chars().take(180).collect());
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
}""",
    """/// Senkron hatasını panelde gösterir ve `bulut_senkron.log` dosyasına yazar.
/// Anahtar kasasına dokunmaz; hata metni anahtar içermez.
pub fn bulut_hata_yaz(hata: &str) {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_hata = Some(hata.chars().take(180).collect());
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
    drop(sayac);
    if std::fs::metadata(BULUT_LOG)
        .map(|bilgi| bilgi.len() > 64 * 1024)
        .unwrap_or(false)
    {
        let _ = std::fs::remove_file(BULUT_LOG);
    }
    if let Ok(mut dosya) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(BULUT_LOG)
    {
        use std::io::Write;
        let satir = format!(
            "{} · {}\\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            hata.chars().take(300).collect::<String>()
        );
        let _ = dosya.write_all(satir.as_bytes());
    }
}

/// Senkron hatalarının zaman damgalı kaydı (64 KB'ı aşınca yenilenir).
const BULUT_LOG: &str = "bulut_senkron.log";

static UYANDIR: OnceLock<Mutex<Option<std::sync::mpsc::Sender<()>>>> = OnceLock::new();

/// Arka plan senkron döngüsünü hemen uyandırır: her API isteğinden sonra
/// gerçek kullanımın gecikmeden tazelenmesini sağlar.
pub fn senkron_talep_et() {
    let kilit = UYANDIR.get_or_init(|| Mutex::new(None));
    if let Ok(gonder) = kilit.lock() {
        if let Some(gonder) = gonder.as_ref() {
            let _ = gonder.send(());
        }
    }
}

/// Arka plan döngüsünün uyandırma kanalını bağlar (main bir kez çağırır).
pub fn senkron_uyandirici(gonder: std::sync::mpsc::Sender<()>) {
    let kilit = UYANDIR.get_or_init(|| Mutex::new(None));
    if let Ok(mut kilit) = kilit.lock() {
        *kilit = Some(gonder);
    }
}""",
)

# --- 9) Her başarılı istekten sonra senkron iste -----------------------------
g = degistir(
    g,
    """    if karar == Karar::Izin {
        OTURUM.fetch_add(1, Ordering::Relaxed);
        UCUS.fetch_add(1, Ordering::Relaxed);
        let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
    }
    karar
}""",
    """    if karar == Karar::Izin {
        OTURUM.fetch_add(1, Ordering::Relaxed);
        UCUS.fetch_add(1, Ordering::Relaxed);
        let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
        drop(sayac);
        // İstek sonrası gerçek kullanım hemen tazelenir (engellemesiz).
        senkron_talep_et();
    }
    karar
}""",
)

# --- 10) Testler: servis kırılımı --------------------------------------------
g = degistir(
    g,
    """        let yanit: BulutYanit = serde_json::from_str(metin).expect("metrik çözümlenemedi");
        let (gunluk, aylik) = bulut_topla(&yanit, "2026-11-03");
        assert_eq!(gunluk, 10, "bugünün saatlik kovaları toplanmalı");
        assert_eq!(aylik, 15, "ayın tüm kovaları toplanmalı");""",
    """        let yanit: BulutYanit = serde_json::from_str(metin).expect("metrik çözümlenemedi");
        let (gunluk, aylik, places, geocode) = bulut_topla(&yanit, "2026-11-03");
        assert_eq!(gunluk, 10, "bugünün saatlik kovaları toplanmalı");
        assert_eq!(aylik, 15, "ayın tüm kovaları toplanmalı");
        assert_eq!(places, 15, "servis etiketi Places olarak ayrılmalı");
        assert_eq!(geocode, 0, "Geocoding serisi yokken sıfır kalmalı");""",
)

g = degistir(
    g,
    """            "timeSeries": [{
                "points": [
                    {"interval":{"startTime":"2026-11-03T09:00:00Z"},"value":{"int64Value":"7"}},""",
    """            "timeSeries": [{
                "resource": {"labels": {"service": "places.googleapis.com"}},
                "points": [
                    {"interval":{"startTime":"2026-11-03T09:00:00Z"},"value":{"int64Value":"7"}},""",
)

yaz("src/google.rs", g)

# ===========================================================================
# main.rs — döngü: ilk turda senkron, sonra istek/60 sn ile uyanma
# ===========================================================================
m = oku("src/main.rs")
m = degistir(
    m,
    """fn bulut_dongusu_baslat() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(google::BULUT_SENKRON_SN));
        let anahtar = anahtar::Kasa::yukle().anahtar().map(str::to_string);
        let Some(anahtar) = anahtar else {
            continue;
        };
        let proje = ayar_oku().google_proje;
        if proje.trim().is_empty() {
            continue;
        }
        match google::bulut_senkron(&anahtar, &proje) {
            Ok(sonuc) => google::bulut_yaz(&sonuc),
            Err(hata) => google::bulut_hata_yaz(&hata),
        }
    });
}""",
    """fn bulut_dongusu_baslat() {
    let (uyandir, bekle) = std::sync::mpsc::channel();
    google::senkron_uyandirici(uyandir);
    std::thread::spawn(move || loop {
        let anahtar = anahtar::Kasa::yukle().anahtar().map(str::to_string);
        let proje = ayar_oku().google_proje;
        if let Some(anahtar) = anahtar {
            if !proje.trim().is_empty() {
                match google::bulut_senkron(&anahtar, &proje) {
                    Ok(sonuc) => google::bulut_yaz(&sonuc),
                    Err(hata) => google::bulut_hata_yaz(&hata),
                }
            }
        }
        // Yeni API isteği ya da 60 saniye: hangisi önce gelirse tazelenir.
        let _ = bekle.recv_timeout(Duration::from_secs(google::BULUT_SENKRON_SN));
    });
}""",
)
yaz("src/main.rs", m)

# ===========================================================================
# katmanlar.rs — birebir istenen sayaç metinleri + servis kırılımı
# ===========================================================================
k = oku("src/katmanlar.rs")
k = degistir(
    k,
    """                        .text(format!(
                            "Günlük {} / {}  ·  kalan {}",
                            ozet.gunluk, ozet.gunluk_limit, ozet.kalan_gun
                        ))""",
    """                        .text(format!(
                            "Günlük {} / {} · kalan {}",
                            ozet.gunluk, ozet.gunluk_limit, ozet.kalan_gun
                        ))""",
)
k = degistir(
    k,
    """                        .text(format!(
                            "Aylık {} / {}  ·  kalan {}",
                            ozet.aylik, ozet.aylik_limit, ozet.kalan_ay
                        ))""",
    """                        .text(format!(
                            "Aylık {} / {} · kalan {}",
                            ozet.aylik, ozet.aylik_limit, ozet.kalan_ay
                        ))""",
)
k = degistir(
    k,
    """                        .text(format!(
                            "Aylık ücretsiz kota kullanımı %{:.2} ({} / {} istek)",
                            ozet.ucretsiz_yuzde * 100.0,
                            ozet.aylik,
                            google::AYLIK_UCRETSIZ_ISTEK
                        ))""",
    """                        .text(format!(
                            "Aylık ücretsiz kota %{:.2} ({} / {} istek)",
                            ozet.ucretsiz_yuzde * 100.0,
                            ozet.bulut_aylik.unwrap_or(ozet.aylik),
                            google::AYLIK_UCRETSIZ_ISTEK
                        ))""",
)
k = degistir(
    k,
    """                    (Some(gun), Some(ay), Some(zaman)) => format!(
                        "Bulut: bugün {gun} · bu ay {ay} istek · son senkron {zaman} · 60 sn'de bir"
                    ),""",
    """                    (Some(gun), Some(ay), Some(zaman)) => format!(
                        "Bulut: bugün {gun} · bu ay {ay} istek · Places {} · Geocoding {} · son senkron {zaman} · 60 sn'de bir",
                        ozet.bulut_places.unwrap_or(0),
                        ozet.bulut_geocode.unwrap_or(0)
                    ),""",
)
k = degistir(
    k,
    """                ui.label(egui::RichText::new(bulut_metni).size(9.0).color(DIM));""",
    """                ui.label(egui::RichText::new(bulut_metni).size(9.0).color(DIM));
                if self.api_bulut.is_some() {
                    ui.label(
                        egui::RichText::new("Google Cloud kullanımı senkronize ediliyor…")
                            .size(9.0)
                            .color(TURUNCU),
                    );
                }""",
)
yaz("src/katmanlar.rs", k)

print("servis bazlı bulut senkronu ve UI metinleri uygulandı")
