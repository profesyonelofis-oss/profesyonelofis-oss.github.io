"""main.rs: Google Places katmanını sök, Overpass hattını bağla."""

import re
from pathlib import Path

YOL = Path("src/main.rs")
s = YOL.read_text(encoding="utf-8")


def kes(bas: str, son: str, yeni: str = "", ad: str = "") -> None:
    """`bas` ile `son` arasındaki bloğu (son hariç) `yeni` ile değiştirir."""
    global s
    assert s.count(bas) == 1, f"{ad}: başlangıç bulunamadı/çok: {s.count(bas)}"
    i = s.index(bas)
    j = s.index(son, i)
    s = s[:i] + yeni + s[j:]


def rep(eski: str, yeni: str, adet: int = 1, ad: str = "") -> None:
    global s
    c = s.count(eski)
    if c == 0:
        print(f"ATLANDI {ad}: metin zaten yok")
        return
    assert c == adet, f"{ad}: beklenen {adet}, bulunan {c}"
    s = s.replace(eski, yeni)


def fn_sil(ad: str, girinti: str = "") -> None:
    """Bir fonksiyonu (önceki doc/attribute satırlarıyla) süslü parantez
    sayarak siler."""
    global s
    im = re.search(rf"(?m)^{re.escape(girinti)}(?:pub(?:\(crate\))? )?fn {re.escape(ad)}\b", s)
    assert im, f"fonksiyon yok: {ad}"
    bas = im.start()
    # Önceki doc/attribute satırlarını da al.
    while True:
        onceki = s.rfind("\n", 0, bas - 1)
        if onceki < 0:
            break
        satir = s[onceki + 1 : bas].strip()
        if satir.startswith("///") or satir.startswith("#[") or satir.startswith("//"):
            bas = onceki + 1
        else:
            break
    derinlik = 0
    gordum = False
    i = im.end()
    while i < len(s):
        if s[i] == "{":
            derinlik += 1
            gordum = True
        elif s[i] == "}":
            derinlik -= 1
            if gordum and derinlik == 0:
                son = i + 1
                while son < len(s) and s[son] in "\r\n":
                    son += 1
                s = s[:bas] + s[son:]
                return
        i += 1
    raise AssertionError(f"fonksiyon kapanışı bulunamadı: {ad}")


# --- modüller --------------------------------------------------------------
rep("mod anahtar;\n", "", ad="anahtar modülü")
rep("mod musteriler;\n", "mod musteriler;\nmod overpass;\n", ad="overpass modülü")

# --- Google ayar / kota / anahtar alanları ---------------------------------
rep(
    """    /// Google kotası göstergesi: kalan günlük/aylık hak ve son kullanım.
    kota_durum: KotaDurumu,
    /// Gösterge her karede dosya okumasın diye son okuma zamanı.
    kota_son_okuma: Option<std::time::Instant>,
    /// Google API anahtarı formu: açık mı, girilen değer, hata satırı.
    anahtar_panel_acik: bool,
    anahtar_girdi: String,
    anahtar_hata: Option<String>,
""",
    "",
    ad="kota/anahtar alanları",
)
rep(
    """            kota_durum: KotaDurumu::default(),
            kota_son_okuma: None,
            anahtar_panel_acik: false,
            anahtar_girdi: String::new(),
            anahtar_hata: None,
""",
    "",
    ad="kota/anahtar başlangıç",
)
rep(
    "    /// Katman 1'in interaktif haritası (Google Maps döşemeleri).",
    "    /// Katman 1'in interaktif haritası (OpenStreetMap döşemeleri).",
    ad="harita alanı belgesi",
)

fn_sil("kota_tazele", "    ")
rep("        self.anahtar_paneli(ctx);\n", "", ad="anahtar paneli çağrısı")

# --- üst bar pili ---------------------------------------------------------
rep(
    """                    ui.add_space(10.0);
                    if google_pili(ui, &self.kota_durum).clicked() {
                        self.anahtar_formunu_ac();
                    }
""",
    """                    ui.add_space(10.0);
                    kontur_pill(ui, "OpenStreetMap · anahtarsız");
""",
    ad="üst bar pili",
)

# --- Google motoru: sabitler + ayar + kota ---------------------------------
kes(
    "const GOOGLE_TEXT_SEARCH: &str",
    "/// Önce ortam değişkeni, sonra `.env` dosyası",
    ad="google sabitleri",
)
kes(
    "/// Günlük/aylık istek sayacı; `google_kota.json` içinde tutulur.",
    "/// Kalıcı uygulama ayarları:",
    ad="kota bloğu",
)
kes(
    "fn kota_oku() -> GoogleKota {",
    "fn bugun_anahtari() -> String {",
    ad="kota dosyası",
)
kes(
    '/// Google\'ın "Monday: 9:00 AM',
    "/// Demo sayfasında gösterilecek bölüm ailesi.",
    """/// Kaydın kaynak bağlantısı: hepsi OpenStreetMap kaydıdır.
fn kaynak_etiketi(_kaynak: &str) -> &'static str {
    "OSM kaydını aç ↗"
}

""",
    ad="google motoru",
)

# --- konum çözümü ----------------------------------------------------------
rep(
    "    /// Google Geocoding görünüm kutusundan türetilen kapsama yarıçapı.",
    "    /// Nominatim sınır kutusundan türetilen kapsama yarıçapı.",
    ad="konum belgesi",
)
kes(
    "// ---------------------------------------------------------------------------\n// Gerçek veri kaynağı: Google Maps Platform",
    "struct Konum {",
    """// ---------------------------------------------------------------------------\n// Veri kaynağı: OpenStreetMap (Overpass + Nominatim + OSM döşemeleri)\n//\n// İl/ilçe merkezi Nominatim'den, işletmeler Overpass API'den anahtarsız\n// okunur; harita OSM döşemeleriyle çizilir. Web sitesi etiketi olan\n// işletmeler elenir, yalnızca sitesiz esnaf listelenir.\n// ---------------------------------------------------------------------------\n\nstruct Konum {""",
    ad="motor açıklaması",
)

# --- önbellekteki Google kayıtlarını dışla ---------------------------------
rep(
    """            .filter(|lead| lead.site_linki.is_empty())
            .cloned()""",
    """            .filter(|lead| lead.site_linki.is_empty() && !lead.kaynak.contains("google.com"))
            .cloned()""",
    ad="önbellek google süzgeci",
)

# --- tarama hattı ----------------------------------------------------------
kes(
    "fn tarama_calistir(tx: Sender<Mesaj>, il: String, ilce: Option<String>, sektor: String) {",
    "/// İl/ilçe merkezini Google Geocoding API'den çözer.",
    '''fn tarama_calistir(tx: Sender<Mesaj>, il: String, ilce: Option<String>, sektor: String) {
    let ajan = http_ajani();
    let mut konum_onbellek = KonumOnbellek::yukle();
    let ilce_adi = ilce.clone().unwrap_or_default();
    let konum = onbellek_konum(&ajan, &mut konum_onbellek, &il, &ilce_adi);
    if let Some(konum) = &konum {
        // Kamera seçilen bölgeye uçar: ilçe sokak ölçeği, il geneli ülke ölçeği.
        let _ = tx.send(Mesaj::Odakla {
            enlem: konum.enlem,
            boylam: konum.boylam,
            zoom: if ilce.is_some() { 13 } else { 9 },
        });
    }
    match ilce {
        Some(ilce) => tarama_tek_ilce(&tx, &ajan, &il, &ilce, &sektor, konum.as_ref()),
        None => tarama_il_geneli(&tx, &ajan, &il, &sektor, konum.as_ref()),
    }
}

/// İl/ilçe merkezini önbellekten, yoksa Nominatim/Overpass sınırından çözer.
fn onbellek_konum(
    ajan: &ureq::Agent,
    onbellek: &mut KonumOnbellek,
    il: &str,
    ilce: &str,
) -> Option<Konum> {
    if !ilce.trim().is_empty() {
        if let Some(konum) = onbellek.al(il, ilce) {
            return Some(konum);
        }
    }
    let konum = konum_bul(ajan, il, ilce).ok()?;
    let kayit_adi = if ilce.trim().is_empty() {
        "il merkezi"
    } else {
        ilce
    };
    onbellek.ekle(il, kayit_adi, &konum);
    onbellek.kaydet();
    Some(konum)
}

/// Overpass sorgusu için bölge: önbellekteki merkezden kutu, yoksa Nominatim.
fn bolge_hesapla(
    ajan: &ureq::Agent,
    il: &str,
    ilce: &str,
    konum: Option<&Konum>,
) -> Result<overpass::Bolge, String> {
    if let Some(konum) = konum {
        let d_enlem = konum.yaricap_m as f64 / 111_320.0;
        let d_boylam = d_enlem / konum.enlem.to_radians().cos().abs().max(0.1);
        return Ok(overpass::Bolge {
            enlem: konum.enlem,
            boylam: konum.boylam,
            kutu: [
                konum.enlem - d_enlem,
                konum.boylam - d_boylam,
                konum.enlem + d_enlem,
                konum.boylam + d_boylam,
            ],
            yaricap_m: konum.yaricap_m,
        });
    }
    overpass::bolge_bul(ajan, il, ilce)
}

/// Tek ilçe taraması: önbellek → Overpass (uç nokta yedeklemeli).
fn tarama_tek_ilce(
    tx: &Sender<Mesaj>,
    ajan: &ureq::Agent,
    il: &str,
    ilce: &str,
    sektor: &str,
    konum: Option<&Konum>,
) {
    let mut onbellek = SonucOnbellek::yukle();
    let merkez = konum
        .map(|konum| {
            format!(
                " · merkez {:.4}, {:.4} · ~{} km",
                konum.enlem,
                konum.boylam,
                (konum.yaricap_m / 1000).max(1)
            )
        })
        .unwrap_or_default();
    let _ = tx.send(Mesaj::Log(format!(
        "[>] Overpass / OSM: {il} / {ilce} · {sektor} (en fazla {SONUC_SINIRI} kayıt){merkez}..."
    )));
    match ilce_ara(tx, ajan, &mut onbellek, il, ilce, sektor, konum) {
        Ok((liste, onbellekten)) => {
            if !onbellekten {
                let _ = tx.send(Mesaj::Log(
                    "[i] Veri kaynağı: OpenStreetMap · Overpass API · anahtarsız, kotasız."
                        .to_string(),
                ));
            }
            let yuksek = liste
                .iter()
                .filter(|isletme| potansiyel_puani(isletme) >= SICAK_ESIK)
                .count();
            let _ = tx.send(Mesaj::Log(format!(
                "[✓] {} sitesiz potansiyel müşteri bulundu · {yuksek} sıcak fırsat.",
                liste.len()
            )));
            let _ = tx.send(Mesaj::TaramaBitti(liste));
        }
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {ilce} taranamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
        }
    }
}

/// İlçe sonucu: önce yerel önbellek, yoksa Overpass. Dönen bayrak sonucun
/// önbellekten geldiğini söyler (günlükte ayrı yazılır).
fn ilce_ara(
    tx: &Sender<Mesaj>,
    ajan: &ureq::Agent,
    onbellek: &mut SonucOnbellek,
    il: &str,
    ilce: &str,
    sektor: &str,
    konum: Option<&Konum>,
) -> Result<(Vec<BusinessLead>, bool), String> {
    if let Some(kayit) = onbellek.al(il, ilce, sektor) {
        let _ = tx.send(Mesaj::Log(format!(
            "[i] {ilce}: sonuçlar önbellekten alındı ({} kayıt, {} gün önce); Overpass'a istek atılmadı.",
            kayit.leads.len(),
            kayit.gun
        )));
        return Ok((kayit.leads, true));
    }
    let bolge = bolge_hesapla(ajan, il, ilce, konum)?;
    let sonuc = overpass::ara(ajan, bolge, sektor, SONUC_SINIRI * 15)?;
    let _ = tx.send(Mesaj::Log(format!(
        "[i] Overpass ({}): {} ham kayıt okundu · {} web siteli işletme elendi.",
        overpass::sunucu_adi(sonuc.sunucu),
        sonuc.ham,
        sonuc.siteli
    )));
    let mut leads = sonuc.isletmeler;
    // Telefonu olmayan kayıt müşteri adayı değildir.
    leads.retain(|lead| telefon_gecerli(&lead.phone));
    leads_sirala(&mut leads);
    leads.truncate(SONUC_SINIRI);
    onbellek.ekle(il, ilce, sektor, &leads, sonuc.siteli);
    onbellek.kaydet();
    Ok((leads, false))
}

/// İl geneli: il sınırı tek Overpass sorgusunda taranır (ilçe ilçe gezinmek
/// yüzlerce ek istek gerektirirdi; tek kutu hem hızlı hem nazik).
fn tarama_il_geneli(
    tx: &Sender<Mesaj>,
    ajan: &ureq::Agent,
    il: &str,
    sektor: &str,
    konum: Option<&Konum>,
) {
    let _ = tx.send(Mesaj::Ilerleme {
        tamamlanan: 0,
        toplam: 1,
    });
    let _ = tx.send(Mesaj::Log(format!(
        "[>] {il} il geneli: Overpass tek sorguda il sınırını tarıyor ({sektor}){}...",
        konum
            .map(|konum| format!(
                " · merkez {:.4}, {:.4} · ~{} km",
                konum.enlem,
                konum.boylam,
                (konum.yaricap_m / 1000).max(1)
            ))
            .unwrap_or_default()
    )));
    let bolge = match bolge_hesapla(ajan, il, "", konum) {
        Ok(bolge) => bolge,
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {il} sınırı bulunamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
            return;
        }
    };
    let sonuc = match overpass::ara(ajan, bolge, sektor, 600) {
        Ok(sonuc) => sonuc,
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {il} taranamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
            return;
        }
    };
    let _ = tx.send(Mesaj::Log(format!(
        "[i] Overpass ({}): {} ham kayıt · {} web siteli işletme elendi.",
        overpass::sunucu_adi(sonuc.sunucu),
        sonuc.ham,
        sonuc.siteli
    )));
    let mut bulunanlar = sonuc.isletmeler;
    bulunanlar.retain(|lead| telefon_gecerli(&lead.phone));
    incelemeye_gore_sirala(&mut bulunanlar);
    bulunanlar.truncate(SONUC_SINIRI);
    let _ = tx.send(Mesaj::Ilerleme {
        tamamlanan: 1,
        toplam: 1,
    });
    let yuksek = bulunanlar
        .iter()
        .filter(|isletme| potansiyel_puani(isletme) >= SICAK_ESIK)
        .count();
    let _ = tx.send(Mesaj::Log(format!(
        "[✓] {il} il geneli bitti: {} sitesiz kayıt ({} sıcak fırsat); telefonu olmayan kayıt listeye alınmadı.",
        bulunanlar.len(),
        yuksek
    )));
    let _ = tx.send(Mesaj::TaramaBitti(bulunanlar));
}

''',
    ad="tarama hattı",
)

# --- konum_bul + kapsama_yaricapi -----------------------------------------
kes(
    "/// İl/ilçe merkezini Google Geocoding API'den çözer.",
    "/// Üst bardaki Google durum pili",
    '''/// İl/ilçe merkezini Nominatim'den (anahtarsız) çözer.
fn konum_bul(ajan: &ureq::Agent, il: &str, ilce: &str) -> Result<Konum, String> {
    let bolge = overpass::bolge_bul(ajan, il, ilce)?;
    Ok(Konum {
        enlem: bolge.enlem,
        boylam: bolge.boylam,
        yaricap_m: bolge.yaricap_m,
    })
}

''',
    ad="konum_bul",
)

# --- Google pili + anahtar formu -----------------------------------------
kes(
    "/// Üst bardaki Google durum pili",
    "#[cfg(test)]\nmod testler {",
    ad="google pili ve anahtar formu",
)

# --- hedef çekme ----------------------------------------------------------
kes(
    "    /// Google Places'ten tek sayfada 20 işletme çeker ve listeyi yeniler.",
    "// ---------------------------------------------------------------------------\n// Keşif motoru: yalnızca Google Maps Platform",
    '''    /// Overpass'tan tek sorguda 20 sitesiz işletme çeker ve listeyi yeniler.
    fn hedef_cek(&mut self) {
        if self.is_running {
            self.logs
                .push("[i] Tarama sürüyor; bitmesini bekleyin.".to_string());
            return;
        }
        let il = self.cities[self.selected_city_idx].0.to_string();
        let ilce = if self.il_geneli {
            String::new()
        } else {
            self.cities[self.selected_city_idx].1[self.selected_district_idx].to_string()
        };
        let sektor = self.sectors[self.selected_sector_idx].to_string();
        self.is_running = true;
        self.logs.push(format!(
            "[>] Overpass: {ilce} / {il} · {sektor} · sitesiz hedefler isteniyor..."
        ));
        let tx = self.log_tx.clone();
        thread::spawn(move || match hedef_20(&il, &ilce, &sektor) {
            Ok(liste) => {
                let _ = tx.send(Mesaj::Log(format!(
                    "[✓] Overpass hedefleri: {} sitesiz kayıt alındı.",
                    liste.len()
                )));
                let _ = tx.send(Mesaj::TaramaBitti(liste));
            }
            Err(hata) => {
                let _ = tx.send(Mesaj::Log(format!("[!] {hata}")));
                let _ = tx.send(Mesaj::TaramaBasarisiz);
            }
        });
    }
}

/// Overpass: bölgedeki web sitesi olmayan işletmeleri tek sorguda çeker.
fn hedef_20(il: &str, ilce: &str, sektor: &str) -> Result<Vec<BusinessLead>, String> {
    let ajan = http_ajani();
    let bolge = overpass::bolge_bul(&ajan, il, ilce)?;
    let sonuc = overpass::ara(&ajan, bolge, sektor, 300)?;
    let mut liste: Vec<BusinessLead> = sonuc
        .isletmeler
        .into_iter()
        .filter(|lead| telefon_gecerli(&lead.phone))
        .collect();
    if liste.is_empty() {
        return Err(format!(
            "Overpass {ilce} / {il} için sitesiz işletme döndürmedi ({} web siteli kayıt elendi)",
            sonuc.siteli
        ));
    }
    incelemeye_gore_sirala(&mut liste);
    liste.truncate(hedef::GUNLUK_HEDEF);
    Ok(liste)
}

''',
    ad="hedef çekme",
)
kes(
    "// ---------------------------------------------------------------------------\n// Keşif motoru: yalnızca Google Maps Platform",
    "fn tarama_calistir(",
    '''// ---------------------------------------------------------------------------
// Keşif motoru: yalnızca OpenStreetMap
//
// Tek veri kaynağı Overpass API'dir: işletmeler OSM etiketlerinden gelir,
// il/ilçe merkezi Nominatim'den çözülür, harita OSM döşemeleriyle çizilir.
// Hiçbir API anahtarı, kota dosyası veya faturalı servis kullanılmaz.
// ---------------------------------------------------------------------------

''',
    ad="motor başlığı",
)

# --- fotoğraf hattı -------------------------------------------------------
fn_sil("en_iyi_foto")
fn_sil("galeri_fotograflari")
fn_sil("streetview_izinli")
fn_sil("streetview_pano_ayikla")
fn_sil("streetview_onbellek_yolu")
fn_sil("streetview_butcesi")
fn_sil("streetview_pano")
fn_sil("streetview_kare_indir")
fn_sil("streetview_galerisi")

rep(
    """/// Fotoğrafı indirir; 2.5 MB üstü dosyalar demoyu şişirmemek için atlanır.
fn foto_indir(url: &str, google_anahtari: Option<&str>, boyut: u32) -> Option<(Vec<u8>, String)> {
    use std::io::Read;
    let ajan = http_ajani();
    let istek = if let Some(ad) = url.strip_prefix("google:") {
        let anahtar = google_anahtari.filter(|anahtar| !anahtar.trim().is_empty())?;
        ajan.get(&format!(
            "https://places.googleapis.com/v1/{ad}/media?maxHeightPx={boyut}&maxWidthPx={boyut}"
        ))
        .set("X-Goog-Api-Key", anahtar)
    } else if let Some(dosya) = url.strip_prefix("commons:") {""",
    """/// Fotoğrafı indirir; 2.5 MB üstü dosyalar demoyu şişirmemek için atlanır.
fn foto_indir(url: &str, boyut: u32) -> Option<(Vec<u8>, String)> {
    use std::io::Read;
    let ajan = http_ajani();
    let istek = if let Some(dosya) = url.strip_prefix("commons:") {""",
    ad="foto_indir imzası",
)
rep(
    """    let google = GoogleAyar::oku();
    let anahtar = Some(google.anahtar.as_str());
    let Some((veri, mime)) = foto_indir(&foto.url, 1600) else {""",
    """    let Some((veri, mime)) = foto_indir(&foto.url, 1600) else {""",
    ad="demo_fotografi anahtarı",
)
rep(
    """fn demo_galerisi(isletme: &BusinessLead) -> (Vec<(String, String)>, bool, Option<String>) {
    const GALERI_BOYUT: u32 = 1100;
    let google = GoogleAyar::oku();
    let anahtar = Some(google.anahtar.as_str());
    let mut kareler = Vec::new();
    for foto in &isletme.fotolar {
        if let Some((veri, mime)) = foto_indir(&foto.url, anahtar, GALERI_BOYUT) {
            kareler.push((mime, base64_kodla(&veri)));
        }
    }
    if !kareler.is_empty() {
        return (kareler, false, None);
    }
    if streetview_izinli() {
        if let Some(konum) = isletme.konum {
            let kareler = streetview_galerisi(konum);
            if !kareler.is_empty() {
                return (
                    kareler,
                    false,
                    Some("Sokak görüntüsü: Google Street View".to_string()),
                );
            }
        }
    }
""",
    """fn demo_galerisi(isletme: &BusinessLead) -> (Vec<(String, String)>, bool, Option<String>) {
    const GALERI_BOYUT: u32 = 1100;
    let mut kareler = Vec::new();
    for foto in &isletme.fotolar {
        if let Some((veri, mime)) = foto_indir(&foto.url, GALERI_BOYUT) {
            kareler.push((mime, base64_kodla(&veri)));
        }
    }
    if !kareler.is_empty() {
        return (kareler, false, None);
    }
""",
    ad="demo_galerisi",
)

# --- hedef çekme çağrıları ------------------------------------------------
rep("        let mut google_cek = false;", "        let mut hedef_cek = false;", ad="hedef bayrağı")
rep("                        google_cek = true;", "                        hedef_cek = true;", ad="hedef bayrağı ataması")
rep(
    "        if google_cek {\n            self.google_hedef_cek();\n        }",
    "        if hedef_cek {\n            self.hedef_cek();\n        }",
    ad="hedef çekme çağrısı",
)

# --- kullanıcıya görünen Google metinleri ---------------------------------
rep('"Google\'dan 20 hedef çek"', '"Overpass\'tan 20 hedef çek"', ad="hedef düğmesi")
rep(
    '"Liste boş: önce bir bölge taraması yapın ya da Google\'dan 20 hedef çekin."',
    '"Liste boş: önce bir bölge taraması yapın ya da Overpass\'tan 20 hedef çekin."',
    ad="boş liste metni",
)
rep(
    '"Google Places kayıtları çekiliyor, sonuçlar birazdan listelenecek."',
    '"OpenStreetMap kayıtları çekiliyor, sonuçlar birazdan listelenecek."',
    ad="bekleme metni",
)
rep(
    '"İl geneli taraması otomatik yapıldı ve sonuç boş döndü. Sektör Google Places\'ta çok seyrek olabilir."',
    '"İl geneli taraması otomatik yapıldı ve sonuç boş döndü. Bu sektör OSM\'de seyrek etiketlenmiş olabilir."',
    ad="boş ilçe metni",
)
rep(
    """                            egui::RichText::new("Google Maps · canlı")""",
    """                            egui::RichText::new("OpenStreetMap · canlı")""",
    ad="harita başlığı",
)
rep(
    """                        kontur_pill(ui, "Google Places · canlı");""",
    """                        kontur_pill(ui, "OSM · Overpass");""",
    ad="orta başlık pili",
)
rep(
    "    /// meslek kolları ve Google Business Profile kategorileri (berber, tırnak",
    "    /// meslek kolları ve OSM işletme etiketleri (berber, tırnak",
    ad="sektör belgesi",
)
rep(
    "///   Google inceleme sayısı (gerçek müşteri)     +0..18",
    "///   inceleme sayısı (varsa, gerçek müşteri)     +0..18",
    ad="puan belgesi",
)
rep(
    '            parcalar.push(format!("{inceleme} Google incelemesi"));',
    '            parcalar.push(format!("{inceleme} inceleme"));',
    ad="gerekçe metni",
)

# --- testler: Google/Street View testleri kalkar --------------------------
for test in [
    "google_istegi_page_size_kullanir",
    "google_kaynaginda_inceleme_sayisi_once_gelir",
    "google_kotasi_gunluk_ve_aylik_siniri_korur",
    "google_saatleri_osm_bicimine_cevrilir",
    "google_yanitindan_siteli_kayitlar_ayiklanir",
    "gecici_google_fotolu_demo_uret",
    "en_iyi_foto_yatay_ve_buyuk_olanı_secer",
    "galeri_fotograflari_kapagi_tekrarlamaz",
    "streetview_metadata_yaniti_cozulur",
    "streetview_onbellegi_kararli_dosya_adi_uretir",
    "streetview_kareleri_kaynagiyla_cizilir",
    "streetview_galerisi_canli",
    "streetview_canli_dene",
]:
    fn_sil(test, "    ")

for kalinti in ["google_", "Google Places", "streetview", "kota_durum", "anahtar_panel"]:
    for no, satir in enumerate(s.split("\n"), 1):
        if kalinti in satir:
            print(f"KALINTI {kalinti}:{no}: {satir.strip()}")

YOL.write_text(s, encoding="utf-8", newline="")
print(f"tamam · {len(s)} bayt · {s.count(chr(10))} satır")
