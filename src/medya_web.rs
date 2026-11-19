//! Yerel önizleme ve onay paneli: `http://127.0.0.1:8080`.
//!
//! Panel yalnızca kullanıcının eklediği medyayı gösterir: her sektör için
//! [Medya / Link Ekle] alanına doğrudan link yapıştırılır veya dosya yüklenir.
//! Otomatik arama/tedarik döngüsü yoktur.
//!
//! Yüklenen medya [`crate::medya_manuel`] içinde 1920×1080 kalite kapısından
//! ve %100 Türkiye/eşleşme doğrulamasından geçer; yalnız geçenler
//! `media/pending` altında önizlemeye düşer. Onay dosyayı `media/stock` altına
//! taşır, red diskten siler.
//!
//! `--onizleme` ile başlatıldığında sunucu hazır olur olmaz sistemin
//! varsayılan tarayıcısı panel adresinde açılır ([`tarayici_ac`]).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

/// Panelin dinlediği port.
pub const PORT: u16 = 8080;
/// Tarayıcıda açılacak adres.
pub const ADRES: &str = "http://127.0.0.1:8080";

/// Sunucuya gelen tek istek.
pub struct Istek {
    pub yontem: String,
    pub hedef: String,
    pub govde: String,
    /// Ham gövde: dosya yüklemede ikili veri bozulmadan taşınır.
    pub ham: Vec<u8>,
    /// `content-type` başlığı (dosya türü ipucu).
    pub icerik_turu: String,
}

/// Sunucunun tek yanıtı.
pub struct Cevap {
    pub kod: u16,
    pub tur: &'static str,
    pub govde: Vec<u8>,
    pub yonlendirme: Option<String>,
}

/// Sunucuyu arka planda başlatır; port doluysa hata döner.
pub fn baslat(kok: PathBuf) -> std::io::Result<()> {
    let dinleyici = TcpListener::bind(("127.0.0.1", PORT))?;
    println!("[ÖNİZLEME] panel hazır: {ADRES} · kök: {}", kok.display());
    std::thread::spawn(move || {
        for akis in dinleyici.incoming().flatten() {
            let kok = kok.clone();
            std::thread::spawn(move || {
                let _ = isle(akis, &kok);
            });
        }
    });
    Ok(())
}

/// Varsayılan tarayıcıyı panel adresine yönlendiren komut (platforma göre).
fn tarayici_komutu() -> (&'static str, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        (
            "cmd",
            vec![
                "/C".to_string(),
                "start".to_string(),
                // Boş başlık: Windows adresi pencere başlığı sanmasın.
                String::new(),
                ADRES.to_string(),
            ],
        )
    }
    #[cfg(target_os = "macos")]
    {
        ("open", vec![ADRES.to_string()])
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        ("xdg-open", vec![ADRES.to_string()])
    }
}

/// İşletim sisteminin varsayılan tarayıcısında paneli açar.
///
/// Sunucu dinlemeye başladıktan sonra çağrılır; tarayıcı yoksa `false` döner ve
/// çağıran taraf adresi yazılı olarak hatırlatır.
pub fn tarayici_ac() -> bool {
    let (program, argumanlar) = tarayici_komutu();
    match std::process::Command::new(program)
        .args(&argumanlar)
        .status()
    {
        Ok(durum) => durum.success(),
        Err(_) => false,
    }
}

/// Panel şu anda 127.0.0.1:8080 üzerinde dinliyor mu?
///
/// Kısa bir TCP denemesi yapar; ikinci kez `--onizleme` çalıştırıldığında yeni
/// sunucu açmak yerine var olan sekme öne getirilir.
pub fn hazir_mi() -> bool {
    let adres = std::net::SocketAddr::from(([127, 0, 0, 1], PORT));
    TcpStream::connect_timeout(&adres, std::time::Duration::from_millis(400)).is_ok()
}

fn isle(mut akis: TcpStream, kok: &Path) -> std::io::Result<()> {
    let mut okuyucu = BufReader::new(akis.try_clone()?);
    let mut istek_satiri = String::new();
    okuyucu.read_line(&mut istek_satiri)?;
    let mut parcalar = istek_satiri.split_whitespace();
    let yontem = parcalar.next().unwrap_or("GET").to_string();
    let hedef = parcalar.next().unwrap_or("/").to_string();
    let mut uzunluk = 0usize;
    let mut icerik_turu = String::new();
    loop {
        let mut satir = String::new();
        if okuyucu.read_line(&mut satir)? == 0 {
            break;
        }
        if satir.trim().is_empty() {
            break;
        }
        let kucuk = satir.to_ascii_lowercase();
        if let Some(deger) = kucuk.strip_prefix("content-length:") {
            uzunluk = deger.trim().parse().unwrap_or(0);
        } else if let Some(deger) = kucuk.strip_prefix("content-type:") {
            icerik_turu = deger.trim().to_string();
        }
    }
    const GOVDE_MAKS: usize = 320 * 1024 * 1024;
    if uzunluk > GOVDE_MAKS {
        let cevap = metin(413, "gövde çok büyük; en fazla 320 MB");
        return cevap_yaz(&mut akis, &cevap);
    }
    let mut govde = vec![0u8; uzunluk];
    if !govde.is_empty() {
        okuyucu.read_exact(&mut govde)?;
    }
    let istek = Istek {
        yontem,
        hedef,
        // Form gövdesi küçük; 320 MB'lık yüklemede ikinci bir kopya çıkarma.
        govde: if govde.len() <= 1_048_576 {
            String::from_utf8_lossy(&govde).to_string()
        } else {
            String::new()
        },
        ham: govde,
        icerik_turu,
    };
    cevap_yaz(&mut akis, &istek_isle(kok, &istek))
}

fn cevap_yaz(akis: &mut TcpStream, cevap: &Cevap) -> std::io::Result<()> {
    let durum = match cevap.kod {
        200 => "200 OK",
        303 => "303 See Other",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        413 => "413 Payload Too Large",
        _ => "500 Internal Server Error",
    };
    let mut baslik = format!(
        "HTTP/1.1 {durum}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        cevap.tur,
        cevap.govde.len()
    );
    if let Some(yer) = &cevap.yonlendirme {
        baslik.push_str(&format!("Location: {yer}\r\n"));
    }
    baslik.push_str("\r\n");
    akis.write_all(baslik.as_bytes())?;
    akis.write_all(&cevap.govde)
}

fn metin(kod: u16, mesaj: &str) -> Cevap {
    Cevap {
        kod,
        tur: "text/plain; charset=utf-8",
        govde: mesaj.as_bytes().to_vec(),
        yonlendirme: None,
    }
}

fn yonlendir(yer: String) -> Cevap {
    Cevap {
        kod: 303,
        tur: "text/plain; charset=utf-8",
        govde: Vec::new(),
        yonlendirme: Some(yer),
    }
}

/// `a=1&b=2` gövdesini çiftlere ayırır (yüzde kodlaması çözülür).
fn alanlar(govde: &str) -> Vec<(String, String)> {
    govde
        .split('&')
        .filter_map(|parca| {
            let (anahtar, deger) = parca.split_once('=')?;
            Some((coz(anahtar), coz(deger)))
        })
        .collect()
}

fn alan(ciftler: &[(String, String)], anahtar: &str) -> String {
    ciftler
        .iter()
        .find(|(ad, _)| ad == anahtar)
        .map(|(_, deger)| deger.clone())
        .unwrap_or_default()
}

/// Yüzde kodlamasını ve `+` boşluklarını çözer.
fn coz(metin: &str) -> String {
    let baytlar = metin.as_bytes();
    let mut cikti = Vec::with_capacity(baytlar.len());
    let mut sira = 0;
    while sira < baytlar.len() {
        match baytlar[sira] {
            b'+' => {
                cikti.push(b' ');
                sira += 1;
            }
            b'%' if sira + 2 < baytlar.len() => {
                let onalti = |b: u8| match b {
                    b'0'..=b'9' => Some(b - b'0'),
                    b'a'..=b'f' => Some(b - b'a' + 10),
                    b'A'..=b'F' => Some(b - b'A' + 10),
                    _ => None,
                };
                match (onalti(baytlar[sira + 1]), onalti(baytlar[sira + 2])) {
                    (Some(ust), Some(alt)) => {
                        cikti.push(ust * 16 + alt);
                        sira += 3;
                    }
                    _ => {
                        cikti.push(baytlar[sira]);
                        sira += 1;
                    }
                }
            }
            bayt => {
                cikti.push(bayt);
                sira += 1;
            }
        }
    }
    String::from_utf8_lossy(&cikti).to_string()
}

/// Adres satırı için yüzde kodlaması.
fn kodla(metin: &str) -> String {
    let mut cikti = String::new();
    for bayt in metin.as_bytes() {
        match bayt {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                cikti.push(*bayt as char)
            }
            _ => cikti.push_str(&format!("%{bayt:02X}")),
        }
    }
    cikti
}

/// HTML kaçışı: kullanıcı/medya metni sayfaya güvenle girer.
fn kacis(metin: &str) -> String {
    metin
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Panelin sunabileceği medya uzantısı mı?
fn medya_dosyasi_mi(yol: &str) -> bool {
    let kucuk = yol.to_ascii_lowercase();
    ["mp4", "m4v", "webm", "mov", "webp", "jpg", "jpeg", "png"]
        .iter()
        .any(|uzanti| kucuk.ends_with(&format!(".{uzanti}")))
}

fn medya_turu(yol: &Path) -> &'static str {
    match yol
        .extension()
        .and_then(|uzanti| uzanti.to_str())
        .map(|uzanti| uzanti.to_ascii_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "mp4" | "m4v" | "webm" | "mov" => "video/mp4",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}

/// İsteği yanıtlar; ağdan bağımsız olduğu için testler doğrudan çağırır.
pub fn istek_isle(kok: &Path, istek: &Istek) -> Cevap {
    let (yol, sorgu) = match istek.hedef.split_once('?') {
        Some((yol, sorgu)) => (yol, sorgu),
        None => (istek.hedef.as_str(), ""),
    };
    match (istek.yontem.as_str(), yol) {
        ("GET", "/") => sayfa(kok, sorgu),
        ("GET", "/medya") => medya(kok, sorgu),
        ("GET", "/durum") => durum_json(kok),
        ("POST", "/onayla") => tek_isle(kok, &istek.govde, true),
        ("POST", "/reddet") => tek_isle(kok, &istek.govde, false),
        ("POST", "/toplu") => toplu_isle(kok, &istek.govde),
        ("POST", "/ekle") => ekle_isle(kok, &istek.govde),
        ("POST", "/yukle") => yukle_isle(kok, sorgu, istek),
        ("GET", "/islemler") => islemler_json(kok),
        _ => metin(404, "sayfa yok"),
    }
}

fn tek_isle(kok: &Path, govde: &str, onay: bool) -> Cevap {
    let ciftler = alanlar(govde);
    let sektor = alan(&ciftler, "sektor");
    let dosya = alan(&ciftler, "dosya");
    if sektor.is_empty() || dosya.is_empty() {
        return yonlendir(format!("/?mesaj={}", kodla("Eksik sektör ya da dosya")));
    }
    let sonuc: Result<String, String> = if onay {
        crate::medya_onay::onayla_dosya(kok, &sektor, &dosya).map(|kayit| {
            format!(
                "{sektor}: onaylandı · {}/{} dosya",
                kayit.dosya_sayisi(),
                crate::medya_onay::VIDEO_KOTA + crate::medya_onay::FOTO_KOTA
            )
        })
    } else {
        crate::medya_onay::reddet_dosya(kok, &sektor, &dosya)
            .map(|_| format!("{sektor}: medya diskten kalıcı olarak silindi"))
    };
    match sonuc {
        Ok(mesaj) => yonlendir(format!("/?mesaj={}", kodla(&mesaj))),
        Err(hata) => yonlendir(format!("/?mesaj={}", kodla(&hata))),
    }
}

fn toplu_isle(kok: &Path, govde: &str) -> Cevap {
    let ciftler = alanlar(govde);
    let islem = alan(&ciftler, "islem");
    let secimler: Vec<&str> = ciftler
        .iter()
        .filter(|(ad, _)| ad == "secim")
        .map(|(_, deger)| deger.as_str())
        .collect();
    if secimler.is_empty() {
        return yonlendir(format!("/?mesaj={}", kodla("Seçim yapılmadı")));
    }
    let (mut onay, mut sil) = (0usize, 0usize);
    if islem == "onayla" {
        for secim in secimler {
            let Some((sektor, dosya)) = secim.split_once('|') else {
                continue;
            };
            if crate::medya_onay::onayla_dosya(kok, sektor, dosya).is_ok() {
                onay += 1;
            }
        }
    } else {
        // Toplu silmede seçilen sektörün bekleyen ve onaylı medyası birlikte silinir.
        let mut sektorler: Vec<&str> = Vec::new();
        for secim in secimler {
            if let Some((sektor, _)) = secim.split_once('|') {
                if !sektorler.contains(&sektor) {
                    sektorler.push(sektor);
                }
            }
        }
        for sektor in sektorler {
            if let Ok(silinen) = crate::medya_onay::reddet(kok, sektor) {
                sil += silinen.len();
            }
        }
    }
    let mesaj = if islem == "onayla" {
        format!("Toplu onay: {onay} medya kalıcı havuza taşındı")
    } else {
        format!("Toplu silme: {sil} medya diskten silindi")
    };
    yonlendir(format!("/?mesaj={}", kodla(&mesaj)))
}

fn medya(kok: &Path, sorgu: &str) -> Cevap {
    let ciftler = alanlar(sorgu);
    let yol = alan(&ciftler, "yol");
    if !yol.starts_with(&format!("{}/", crate::medya_onay::BEKLEYEN))
        || yol.contains("..")
        || !medya_dosyasi_mi(&yol)
    {
        return metin(400, "geçersiz medya yolu");
    }
    let tam = kok.join(&yol);
    let Ok(veri) = fs::read(&tam) else {
        return metin(404, "medya bulunamadı");
    };
    Cevap {
        kod: 200,
        tur: medya_turu(&tam),
        govde: veri,
        yonlendirme: None,
    }
}

fn ekle_isle(kok: &Path, govde: &str) -> Cevap {
    let ciftler = alanlar(govde);
    let sektor = alan(&ciftler, "sektor");
    let link = alan(&ciftler, "link");
    if sektor.is_empty() || link.is_empty() {
        return yonlendir(format!("/?mesaj={}", kodla("Sektör ve link gerekli")));
    }
    match crate::medya_manuel::link_ekle(kok, &sektor, &link) {
        Ok(ekleme) => yonlendir(format!("/?mesaj={}", kodla(&ekleme.mesaj()))),
        Err(hata) => yonlendir(format!("/?mesaj={}", kodla(&hata))),
    }
}

fn yukle_isle(kok: &Path, sorgu: &str, istek: &Istek) -> Cevap {
    let ciftler = alanlar(sorgu);
    let sektor = alan(&ciftler, "sektor");
    let ad = alan(&ciftler, "ad");
    if sektor.is_empty() {
        return yonlendir(format!("/?mesaj={}", kodla("Sektör seçilmedi")));
    }
    if istek.ham.is_empty() {
        return yonlendir(format!("/?mesaj={}", kodla("Dosya seçilmedi ya da boş")));
    }
    match crate::medya_manuel::dosya_ekle(kok, &sektor, &ad, &istek.icerik_turu, &istek.ham) {
        Ok(ekleme) => yonlendir(format!("/?mesaj={}", kodla(&ekleme.mesaj()))),
        Err(hata) => yonlendir(format!("/?mesaj={}", kodla(&hata))),
    }
}

fn islemler_json(kok: &Path) -> Cevap {
    let deger = serde_json::json!({
        "tedarik": "otomatik arama kapalı · manuel ekleme",
        "satirlar": crate::medya_islem::son(kok, 120),
    });
    Cevap {
        kod: 200,
        tur: "application/json; charset=utf-8",
        govde: serde_json::to_vec(&deger).unwrap_or_default(),
        yonlendirme: None,
    }
}

fn durum_json(kok: &Path) -> Cevap {
    let ozet = crate::medya_onay::ozet(kok);
    let sektorler: Vec<serde_json::Value> = crate::medya_onay::kuyruk(kok)
        .into_iter()
        .map(|(slug, kayit)| {
            let (video, foto) = crate::medya_onay::bekleyen_sayilar(kok, &slug);
            serde_json::json!({
                "sektor": slug,
                "durum": kayit.durum.etiket(),
                "onayli_video": kayit.videos.len(),
                "onayli_foto": kayit.fotolar.len(),
                "bekleyen_video": video,
                "bekleyen_foto": foto,
            })
        })
        .collect();
    let deger = serde_json::json!({
        "bekleyen": ozet.bekleyen,
        "onayli": ozet.onayli,
        "red": ozet.red,
        "tedarik": "otomatik arama kapalı · manuel ekleme",
        "sektorler": sektorler,
    });
    Cevap {
        kod: 200,
        tur: "application/json; charset=utf-8",
        govde: serde_json::to_vec_pretty(&deger).unwrap_or_default(),
        yonlendirme: None,
    }
}

const CSS: &str = r#"
:root{--zemin:#0f1210;--kart:#171c19;--cizgi:#27312b;--metin:#eaf2ec;--soluk:#93a39a;--vurgu:#5fbf8a;--turf:#e06a5a}
*{box-sizing:border-box}
body{margin:0;background:var(--zemin);color:var(--metin);font:15px/1.5 -apple-system,"Segoe UI",Roboto,Arial,sans-serif}
header{padding:22px 24px 10px;border-bottom:1px solid var(--cizgi);display:flex;flex-wrap:wrap;gap:16px;align-items:baseline}
h1{font-size:20px;margin:0;letter-spacing:.01em}
.sayac{color:var(--soluk);font-size:13px}
.sayac b{color:var(--metin)}
.mesaj{margin:16px 24px 0;padding:10px 14px;border:1px solid var(--cizgi);border-left:3px solid var(--vurgu);border-radius:8px;background:#131a16;color:var(--metin);font-size:13.5px}
main{padding:18px 24px 60px}
form#toplu{position:sticky;top:0;z-index:5;display:flex;flex-wrap:wrap;gap:10px;align-items:center;padding:10px 0 14px;background:linear-gradient(var(--zemin),rgba(15,18,16,.92))}
button{cursor:pointer;border-radius:8px;border:1px solid var(--cizgi);padding:9px 14px;font:inherit;font-size:13px;font-weight:600;background:#1d2621;color:var(--metin)}
button:hover{border-color:#3a4a41}
button.onay{background:#1e3a2b;border-color:#2f6b4b;color:#a9e6c3}
button.red{background:#3a221f;border-color:#6b3830;color:#f0b4a8}
.sektor{margin:22px 0 6px;border-top:1px solid var(--cizgi);padding-top:16px}
.sektor h2{font-size:16px;margin:0 0 2px}
.sektor .grup{color:var(--soluk);font-size:12.5px}
.sektor>summary{cursor:pointer;list-style:none;padding:2px 0 6px}
.sektor>summary::-webkit-details-marker{display:none}
.sektor>summary b{font-size:16px}
.ekle{display:flex;flex-wrap:wrap;gap:10px;align-items:center;margin:10px 0 4px;padding:10px;border:1px dashed var(--cizgi);border-radius:10px;background:#111713}
.ekle-baslik{flex:0 0 100%;font-size:12px;letter-spacing:.08em;text-transform:uppercase;color:#a9e6c3}
.ekle form{display:flex;flex:1 1 420px;gap:8px;min-width:280px}
.ekle input[type=url]{flex:1;min-width:180px;background:#0b0e0c;border:1px solid var(--cizgi);border-radius:8px;color:var(--metin);padding:9px 10px;font:inherit;font-size:13px}
.ekle input[type=url]:focus{outline:1px solid #3b6ea8}
.ekle .dosya{display:flex;flex-wrap:wrap;gap:8px;align-items:center;font-size:12px}
.ekle .baslat{margin-left:0}
.ekle input[type=file]{max-width:220px;color:var(--soluk);font-size:12px}
.izgara{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:14px;margin-top:12px}
.kart{background:var(--kart);border:1px solid var(--cizgi);border-radius:12px;overflow:hidden;display:flex;flex-direction:column}
.medya{aspect-ratio:16/10;background:#0b0e0c;display:flex;align-items:center;justify-content:center;overflow:hidden;position:relative}
.kaynak{position:absolute;top:8px;right:8px;padding:4px 9px;border-radius:999px;background:rgba(9,12,10,.85);border:1px solid #2f4438;color:#a9e6c3;font-size:11.5px;text-decoration:none}
.kaynak:hover{border-color:#4d7f63}
.kaynak.yok{color:var(--soluk);border-style:dashed}
.medya img,.medya video{width:100%;height:100%;object-fit:cover;display:block}
.bilgi{padding:10px 12px;display:flex;flex-direction:column;gap:3px;font-size:12.5px}
.bilgi b{font-size:13.5px;font-weight:600;line-height:1.35}
.etiket,.soluk{color:var(--soluk)}
.skor{color:#a9e6c3}
.skor.yok{color:var(--soluk)}
.yerel{color:#a9e6c3}
.yerel.yabanci{color:#f0b4a8}
.yerel.yok{color:var(--soluk)}
.islem{padding:0 12px 12px;display:flex;gap:8px}
.islem form{flex:1}
.islem button{width:100%;padding:8px 6px}
.sec{display:flex;align-items:center;gap:8px;padding:8px 12px 0;color:var(--soluk);font-size:12px}
.sec input{width:16px;height:16px;accent-color:var(--vurgu)}
.bos{padding:40px 0;color:var(--soluk)}
button.baslat{background:#20406b;border-color:#3b6ea8;color:#d6e8ff;margin-left:auto}
.yol{font-size:11.5px;margin-top:2px}
.yol summary{cursor:pointer;color:var(--soluk)}
.yol code{display:block;word-break:break-all;color:#cfe4ff;background:#0b0e0c;padding:6px 8px;border-radius:6px;margin:4px 0}
.yol .kopya{padding:4px 8px;font-size:11px}
.gunluk{margin-top:26px;border-top:1px solid var(--cizgi);padding-top:14px}
.gunluk pre{max-height:320px;overflow:auto;background:#0b0e0c;border:1px solid var(--cizgi);border-radius:10px;padding:10px;font-size:11.5px;line-height:1.55;white-space:pre-wrap;word-break:break-all;margin:8px 0 0}
.kota{color:var(--soluk);font-size:12px;margin-top:4px}
footer{padding:0 24px 30px;color:var(--soluk);font-size:12px}
"#;

fn sayfa(kok: &Path, sorgu: &str) -> Cevap {
    let ciftler = alanlar(sorgu);
    let mesaj = alan(&ciftler, "mesaj");
    let ozet = crate::medya_onay::ozet(kok);
    let mut govde = String::new();
    govde.push_str(
        "<!doctype html><html lang=\"tr\"><head><meta charset=\"utf-8\"><meta http-equiv=\"cache-control\" content=\"no-store\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Nexus · Medya Ekleme Paneli</title><style>",
    );
    govde.push_str(CSS);
    govde.push_str("</style></head><body><header><h1>Medya / Link Ekleme Paneli</h1>");
    govde.push_str(&format!(
        "<span class=\"sayac\"><b>{}</b> bekleyen sektör · <b>{}</b> onaylı · <b>{}</b> red · otomatik arama: <b>kapalı</b> · eşik: %100 eşleşme + %100 Türkiye</span>",
        ozet.bekleyen, ozet.onayli, ozet.red
    ));
    govde.push_str("</header>");
    if !mesaj.is_empty() {
        govde.push_str(&format!("<div class=\"mesaj\">{}</div>", kacis(&mesaj)));
    }
    govde.push_str(
        "<main><form id=\"toplu\" method=\"post\" action=\"/toplu\"><button name=\"islem\" value=\"onayla\" class=\"onay\">✔ Seçilenleri ONAYLA</button><button name=\"islem\" value=\"sil\" class=\"red\">🗑 Seçilenleri SİL</button><label class=\"soluk\"><input type=\"checkbox\" onclick=\"hepsini(this)\"> Tümünü seç</label></form>",
    );
    let mut kart_var = false;
    for tanim in crate::SEKTORLER {
        let slug = crate::sektor_medya::slug(tanim.ad);
        let medyalar = crate::medya_onay::bekleyen_medya(kok, &slug);
        let kayit = crate::medya_onay::kayit(kok, &slug);
        let (bekleyen_video, bekleyen_foto) = crate::medya_onay::bekleyen_sayilar(kok, &slug);
        if !medyalar.is_empty() {
            kart_var = true;
        }
        let acik = if medyalar.is_empty() { "" } else { " open" };
        govde.push_str(&format!(
            "<details class=\"sektor\"{acik}><summary><b>{ad}</b> <span class=\"grup\">{grup}</span> <span class=\"kota\">onaylı {ov} video + {of} fotoğraf · bekleyen {bv} video + {bf} fotoğraf</span></summary><div class=\"ekle\"><b class=\"ekle-baslik\">Medya / Link Ekle</b><form method=\"post\" action=\"/ekle\"><input type=\"hidden\" name=\"sektor\" value=\"{slug}\"><input class=\"link\" type=\"url\" name=\"link\" required placeholder=\"https://... doğrudan görsel/video linki\"><button class=\"baslat\">LİNKİ İNDİR</button></form><div class=\"dosya\"><input type=\"file\" id=\"dosya-{slug}\" accept=\"image/*,video/*\"><button type=\"button\" onclick=\"yukle('{slug}')\">DOSYAYI YÜKLE</button><span class=\"soluk\" id=\"durum-{slug}\">en az 1920×1080 · %100 Türkiye</span></div></div><div class=\"izgara\">",
            acik = acik,
            ad = kacis(tanim.ad),
            grup = kacis(tanim.grup),
            slug = kacis(&slug),
            ov = kayit.videos.len(),
            of = kayit.fotolar.len(),
            bv = bekleyen_video,
            bf = bekleyen_foto
        ));
        for medya in &medyalar {
            govde.push_str(&kart(medya));
        }
        govde.push_str("</div></details>");
    }
    if !kart_var {
        govde.push_str(
            "<div class=\"bos\">Bekleyen medya yok. Her sektörün panelinden doğrudan link yapıştırın veya bilgisayarınızdan dosya yükleyin.</div>",
        );
    }
    govde.push_str(
        "<section class=\"gunluk\"><h2>İşlem günlüğü · tam dosya yolları</h2><pre id=\"gunluk\">Henüz işlem yok.</pre></section></main><footer>Manuel eklenen medya önce <code>media/pending/</code> altında önizlenir; ONAYLA <code>media/stock/&lt;sektör&gt;/</code> altına taşır, REDDET/SİL diskten siler. · <a href=\"/durum\" style=\"color:inherit\">JSON durum</a></footer>",
    );
    govde.push_str(
        r#"<script>
function hepsini(k){document.querySelectorAll('input[name=secim]').forEach(function(x){x.checked=k.checked});}
function kopyala(d){var c=d.parentElement.querySelector('code').textContent;navigator.clipboard.writeText(c).then(function(){d.textContent='Kopyalandı';setTimeout(function(){d.textContent='Kopyala'},1200)},function(){d.textContent='Kopyalanamadı'})}
function yukle(s){var f=document.getElementById('dosya-'+s);var d=document.getElementById('durum-'+s);if(!f||!f.files||!f.files[0]){if(d)d.textContent='önce dosya seçin';return;}var dosya=f.files[0];if(d)d.textContent='yükleniyor… '+dosya.name;fetch('/yukle?sektor='+encodeURIComponent(s)+'&ad='+encodeURIComponent(dosya.name),{method:'POST',headers:{'Content-Type':dosya.type||'application/octet-stream'},body:dosya}).then(function(r){location.href=r.url||'/';}).catch(function(){if(d)d.textContent='yükleme başarısız';});}
function tazele(){fetch('/islemler').then(function(r){return r.json()}).then(function(v){var g=document.getElementById('gunluk');if(g){g.textContent=(v.satirlar&&v.satirlar.length)?v.satirlar.join('\n'):'Henüz işlem yok.'}}).catch(function(){})}
setInterval(tazele,3000);tazele();
</script></body></html>"#,
    );
    Cevap {
        kod: 200,
        tur: "text/html; charset=utf-8",
        govde: govde.into_bytes(),
        yonlendirme: None,
    }
}

fn kart(medya: &crate::medya_onay::BekleyenMedya) -> String {
    let adres = format!("/medya?yol={}", kodla(&medya.goreli));
    let gorsel = if medya.video {
        format!(
            "<video controls preload=\"metadata\" playsinline src=\"{}\"></video>",
            kacis(&adres)
        )
    } else {
        format!(
            "<img loading=\"lazy\" src=\"{}\" alt=\"{}\">",
            kacis(&adres),
            kacis(&medya.baslik)
        )
    };
    let dosya_adi = medya
        .goreli
        .rsplit('/')
        .next()
        .unwrap_or(&medya.goreli)
        .to_string();
    let baslik = if medya.baslik.is_empty() {
        dosya_adi
    } else {
        medya.baslik.clone()
    };
    let mb = medya.boyut as f64 / 1_048_576.0;
    let tam = kacis(&medya.tam);
    let yol = format!(
        "<details class=\"yol\"><summary>Dosya yolu</summary><code>{tam}</code><button type=\"button\" class=\"kopya\" onclick=\"kopyala(this)\">Kopyala</button></details>"
    );
    let (skor_sinif, skor) = match medya.skor {
        Some(skor) => ("skor", format!("AI eşleşme skoru: %{skor}")),
        None => ("skor yok", "AI eşleşme skoru: doğrulanmadı".to_string()),
    };
    let (yerel_sinif, yerel) = match medya.yerel {
        Some(true) => ("yerel", "Yerel doku: Türkiye ✓"),
        Some(false) => ("yerel yabanci", "Yerel doku: yabancı ✗"),
        None => ("yerel yok", "Yerel doku: doğrulanmadı"),
    };
    let kaynak = if medya.kaynak.is_empty() {
        String::new()
    } else {
        format!(
            "<a style=\"color:inherit\" href=\"{}\" target=\"_blank\" rel=\"noreferrer\">Kaynak sayfası</a>",
            kacis(&medya.kaynak)
        )
    };
    let kaynak_ust = if medya.kaynak.is_empty() {
        "<span class=\"kaynak yok\">kaynak yok</span>".to_string()
    } else {
        format!(
            "<a class=\"kaynak\" href=\"{}\" target=\"_blank\" rel=\"noreferrer\" title=\"Kaynağında aç\">↗ Kaynak</a>",
            kacis(&medya.kaynak)
        )
    };
    let kunye = [medya.yazar.as_str(), medya.lisans.as_str()]
        .into_iter()
        .filter(|parca| !parca.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let gizli = format!(
        "<input type=\"hidden\" name=\"sektor\" value=\"{}\"><input type=\"hidden\" name=\"dosya\" value=\"{}\">",
        kacis(&medya.slug),
        kacis(&medya.goreli)
    );
    format!(
        "<article class=\"kart\" title=\"{tam}\"><label class=\"sec\"><input type=\"checkbox\" form=\"toplu\" name=\"secim\" value=\"{secim}\"> Seç</label><div class=\"medya\">{gorsel}{kaynak_ust}</div><div class=\"bilgi\"><b>{baslik}</b><span class=\"etiket\">{tur} · {mb:.1} MB</span><span class=\"{skor_sinif}\">{skor}</span><span class=\"{yerel_sinif}\">{yerel}</span><span class=\"soluk\">{kunye}</span>{kaynak}{yol}</div><div class=\"islem\"><form method=\"post\" action=\"/onayla\">{gizli}<button class=\"onay\">ONAYLA</button></form><form method=\"post\" action=\"/reddet\">{gizli}<button class=\"red\">REDDET / SİL</button></form></div></article>",
        secim = kacis(&format!("{}|{}", medya.slug, medya.goreli)),
        gorsel = gorsel,
        baslik = kacis(&baslik),
        tur = if medya.video { "Video" } else { "Fotoğraf" },
        mb = mb,
        skor_sinif = skor_sinif,
        skor = kacis(&skor),
        yerel_sinif = yerel_sinif,
        yerel = kacis(yerel),
        kunye = kacis(&kunye),
        kaynak_ust = kaynak_ust,
        kaynak = kaynak,
        yol = yol,
        tam = tam,
        gizli = gizli
    )
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> PathBuf {
        let kok = std::env::temp_dir().join(format!("nexus_web_{}_{ad}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(kok.join("media/pending/kuafor/video")).unwrap();
        fs::create_dir_all(kok.join("media/pending/kuafor/photos")).unwrap();
        fs::write(kok.join("media/pending/kuafor/video/hero_1.mp4"), [1u8; 64]).unwrap();
        fs::write(kok.join("media/pending/kuafor/video/hero_2.mp4"), [4u8; 64]).unwrap();
        fs::write(kok.join("media/pending/kuafor/photos/1.jpg"), [2u8; 24]).unwrap();
        fs::write(kok.join("media/pending/kuafor/photos/2.jpg"), [3u8; 24]).unwrap();
        fs::write(
            kok.join("media/pending/kuafor/KAYNAK.json"),
            serde_json::json!({
                "videolar": [{"dosya": "video/hero_1.mp4", "baslik": "Berber dükkânı", "lisans": "CC BY 4.0", "skor": 100, "yerel": true, "kaynak": "https://commons.wikimedia.org/wiki/File:Barber.mp4"}, {"dosya": "video/hero_2.mp4", "baslik": "Saç kesimi videosu", "skor": 100, "yerel": true}],
                "fotolar": [{"dosya": "photos/1.jpg", "baslik": "Saç kesimi", "skor": 100, "yerel": true, "kaynak": "https://commons.wikimedia.org/wiki/File:Barber.jpg"}]
            })
            .to_string(),
        )
        .unwrap();
        kok
    }

    fn istek(yontem: &str, hedef: &str, govde: &str) -> Istek {
        Istek {
            yontem: yontem.to_string(),
            hedef: hedef.to_string(),
            govde: govde.to_string(),
            ham: govde.as_bytes().to_vec(),
            icerik_turu: "application/x-www-form-urlencoded".to_string(),
        }
    }

    #[test]
    fn panel_bekleyen_medyayi_skorla_gosterir() {
        let kok = gecici("panel");
        let cevap = istek_isle(&kok, &istek("GET", "/", ""));
        assert_eq!(cevap.kod, 200);
        let html = String::from_utf8(cevap.govde).unwrap();
        assert!(html.contains("Kuaför"), "sektör adı görünmeli");
        assert!(html.contains("AI eşleşme skoru: %100"), "{html}");
        assert!(html.contains("Yerel doku: Türkiye"), "yerellik görünmeli");
        assert!(html.contains("<video"), "video oynatıcı olmalı");
        assert!(html.contains("name=\"secim\""), "toplu seçim olmalı");
        assert!(
            html.contains("href=\"https://commons.wikimedia.org/wiki/File:Barber.jpg\""),
            "kaynak bağlantısı kartta olmalı"
        );
        assert!(html.contains("↗ Kaynak"), "kaynak düğmesi görünmeli");
        assert!(html.contains("no-store"), "sayfa önbelleğe alınmamalı");
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn panel_manuel_ekleme_alanini_ve_tam_yolu_gosterir() {
        let kok = gecici("tam");
        let html = String::from_utf8(istek_isle(&kok, &istek("GET", "/", "")).govde).unwrap();
        assert!(
            html.contains("Medya / Link Ekleme Paneli"),
            "başlık görünmeli"
        );
        assert!(
            html.contains("Medya / Link Ekle"),
            "link/dosya paneli görünmeli"
        );
        assert!(
            html.contains("LİNKİ İNDİR"),
            "link ekleme düğmesi görünmeli"
        );
        assert!(
            html.contains("DOSYAYI YÜKLE"),
            "dosya yükleme düğmesi görünmeli"
        );
        assert!(html.contains("action=\"/ekle\""), "link formu olmalı");
        assert!(
            !html.contains("TEDARİKİ BAŞLAT"),
            "otomatik tedarik düğmesi kalkmalı"
        );
        assert!(html.contains("Dosya yolu"), "yol bölümü görünmeli");
        assert!(html.contains("Kopyala"), "kopyalama düğmesi görünmeli");
        let tam = crate::medya_islem::tam_yol(&kok.join("media/pending/kuafor/photos/1.jpg"));
        assert!(html.contains(&kacis(&tam)), "kartta tam yol olmalı: {tam}");
        let cevap = istek_isle(&kok, &istek("GET", "/islemler", ""));
        assert_eq!(cevap.kod, 200);
        let deger: serde_json::Value = serde_json::from_slice(&cevap.govde).unwrap();
        assert!(deger["tedarik"].is_string());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn islemler_json_onay_yolunu_verir() {
        let kok = gecici("gunluk");
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F1.jpg";
        assert_eq!(istek_isle(&kok, &istek("POST", "/onayla", govde)).kod, 303);
        let cevap = istek_isle(&kok, &istek("GET", "/islemler", ""));
        let deger: serde_json::Value = serde_json::from_slice(&cevap.govde).unwrap();
        let satirlar = deger["satirlar"].as_array().unwrap();
        assert!(!satirlar.is_empty(), "onay günlüğe yazılmalı");
        let satir = satirlar[0].as_str().unwrap();
        assert!(satir.contains("ONAY"), "{satir}");
        assert!(satir.contains("pending"), "kaynak yol: {satir}");
        assert!(satir.contains("stock"), "hedef yol: {satir}");
        assert!(kok.join(crate::medya_islem::DOSYA).is_file());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn manuel_link_ve_yukleme_rotalari_calisir() {
        let kok = gecici("manuel");
        let cevap = istek_isle(
            &kok,
            &istek(
                "POST",
                "/ekle",
                "sektor=kuafor&link=ftp%3A%2F%2Fornek.com%2Fa.jpg",
            ),
        );
        assert_eq!(cevap.kod, 303);
        assert!(
            cevap.yonlendirme.unwrap().contains("http"),
            "geçersiz şema reddedilmeli"
        );

        let cevap = istek_isle(&kok, &istek("POST", "/yukle?sektor=kuafor&ad=foto.png", ""));
        assert_eq!(cevap.kod, 303);
        assert!(cevap.yonlendirme.unwrap().contains("Dosya"));

        assert_eq!(istek_isle(&kok, &istek("POST", "/tedarik", "")).kod, 404);
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn islem_sonrasi_kart_aninda_kalkar() {
        let kok = gecici("kalk");
        let html = String::from_utf8(istek_isle(&kok, &istek("GET", "/", "")).govde).unwrap();
        assert!(html.contains("photos/1.jpg"), "kart önce görünmeli");
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F1.jpg";
        assert_eq!(istek_isle(&kok, &istek("POST", "/onayla", govde)).kod, 303);
        let html = String::from_utf8(istek_isle(&kok, &istek("GET", "/", "")).govde).unwrap();
        assert!(!html.contains("photos/1.jpg"), "onaylanan kart düşmeli");
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F2.jpg";
        assert_eq!(istek_isle(&kok, &istek("POST", "/reddet", govde)).kod, 303);
        let html = String::from_utf8(istek_isle(&kok, &istek("GET", "/", "")).govde).unwrap();
        assert!(!html.contains("photos/2.jpg"), "reddedilen kart düşmeli");
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn dusuk_skorlu_medya_panelden_onaylanmaz() {
        let kok = gecici("dusuk");
        fs::write(
            kok.join("media/pending/kuafor/KAYNAK.json"),
            serde_json::json!({"fotolar": [{"dosya": "photos/2.jpg", "skor": 72}]}).to_string(),
        )
        .unwrap();
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F2.jpg";
        let cevap = istek_isle(&kok, &istek("POST", "/onayla", govde));
        assert_eq!(cevap.kod, 303);
        assert!(
            cevap.yonlendirme.unwrap().contains("72"),
            "skor mesajı görünmeli"
        );
        assert!(kok.join("media/pending/kuafor/photos/2.jpg").is_file());
        assert!(!kok.join("media/stock/kuafor/photos/2.jpg").exists());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn yabanci_doku_panelden_onaylanmaz() {
        let kok = gecici("yabanci");
        fs::write(
            kok.join("media/pending/kuafor/KAYNAK.json"),
            serde_json::json!({"fotolar": [{"dosya": "photos/2.jpg", "skor": 100, "yerel": false}]})
                .to_string(),
        )
        .unwrap();
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F2.jpg";
        let cevap = istek_isle(&kok, &istek("POST", "/onayla", govde));
        assert_eq!(cevap.kod, 303);
        assert!(cevap.yonlendirme.unwrap().contains("T%C3%BCrkiye"));
        assert!(kok.join("media/pending/kuafor/photos/2.jpg").is_file());
        assert!(!kok.join("media/stock/kuafor/photos/2.jpg").exists());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn tek_onay_dosyayi_stoga_tasir() {
        let kok = gecici("onay");
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F1.jpg";
        let cevap = istek_isle(&kok, &istek("POST", "/onayla", govde));
        assert_eq!(cevap.kod, 303);
        assert!(cevap.yonlendirme.unwrap().contains("onayland"));
        assert!(kok.join("media/stock/kuafor/photos/1.jpg").is_file());
        assert!(!kok.join("media/pending/kuafor/photos/1.jpg").exists());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn tek_red_dosyayi_siler() {
        let kok = gecici("red");
        let govde = "sektor=kuafor&dosya=media%2Fpending%2Fkuafor%2Fphotos%2F2.jpg";
        let cevap = istek_isle(&kok, &istek("POST", "/reddet", govde));
        assert_eq!(cevap.kod, 303);
        assert!(!kok.join("media/pending/kuafor/photos/2.jpg").exists());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn toplu_onay_ve_silme() {
        let kok = gecici("toplu");
        let secimler = [
            "secim=kuafor%7Cmedia%2Fpending%2Fkuafor%2Fvideo%2Fhero_1.mp4",
            "secim=kuafor%7Cmedia%2Fpending%2Fkuafor%2Fvideo%2Fhero_2.mp4",
            "secim=kuafor%7Cmedia%2Fpending%2Fkuafor%2Fphotos%2F1.jpg",
            "secim=kuafor%7Cmedia%2Fpending%2Fkuafor%2Fphotos%2F2.jpg",
        ]
        .join("&");
        let govde = format!("islem=onayla&{secimler}");
        let cevap = istek_isle(&kok, &istek("POST", "/toplu", &govde));
        assert_eq!(cevap.kod, 303);
        let kayit = crate::medya_onay::kayit(&kok, "kuafor");
        assert!(kayit.demoya_hazir(), "toplu onay kotayı doldurmalı");
        assert!(crate::medya_onay::demo_kilidi(&kok, "kuafor").is_ok());

        let govde = format!("islem=sil&{secimler}");
        let cevap = istek_isle(&kok, &istek("POST", "/toplu", &govde));
        assert_eq!(cevap.kod, 303);
        assert!(crate::medya_onay::onayli_medya(&kok, "kuafor").is_empty());
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn tarayici_komutu_panel_adresini_hedefler() {
        let (program, argumanlar) = tarayici_komutu();
        #[cfg(target_os = "windows")]
        {
            assert_eq!(program, "cmd");
            assert_eq!(argumanlar[1], "start");
            assert_eq!(argumanlar[2], "", "adres pencere başlığı sanılmasın");
        }
        #[cfg(not(target_os = "windows"))]
        let _ = program;
        assert_eq!(argumanlar.last().map(String::as_str), Some(ADRES));
    }

    #[test]
    fn medya_yolu_ve_dizin_gezintisi_engellenir() {
        let kok = gecici("medya");
        let cevap = istek_isle(
            &kok,
            &istek(
                "GET",
                "/medya?yol=media%2Fpending%2Fkuafor%2Fphotos%2F1.jpg",
                "",
            ),
        );
        assert_eq!(cevap.kod, 200);
        assert_eq!(cevap.tur, "image/jpeg");
        assert_eq!(cevap.govde.len(), 24);

        let cevap = istek_isle(&kok, &istek("GET", "/medya?yol=..%2F..%2Fgizli.txt", ""));
        assert_eq!(cevap.kod, 400, "dizin gezintisi reddedilmeli");
        let _ = fs::remove_dir_all(kok);
    }

    #[test]
    fn durum_json_sayaclari_verir() {
        let kok = gecici("durum");
        let cevap = istek_isle(&kok, &istek("GET", "/durum", ""));
        let deger: serde_json::Value = serde_json::from_slice(&cevap.govde).unwrap();
        assert_eq!(deger["bekleyen"], 1);
        assert_eq!(deger["sektorler"][0]["bekleyen_video"], 2);
        assert_eq!(deger["sektorler"][0]["bekleyen_foto"], 2);
        let _ = fs::remove_dir_all(kok);
    }
}
