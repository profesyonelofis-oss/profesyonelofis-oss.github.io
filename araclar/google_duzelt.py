"""Geçişin ikinci turu: bağımlılıklar, kalan Overpass fonksiyonları, egui API."""

import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def yaz(yol, metin):
    Path(yol).write_text(metin, encoding="utf-8", newline="")


def oku(yol):
    return Path(yol).read_text(encoding="utf-8")


# 1) Cargo.toml: bağımlılıklar [dependencies] altına taşınır.
yaz(
    "Cargo.toml",
    """[package]
name = "ollama_proje"
version = "0.1.0"
edition = "2021"

[dependencies]
chrono = "0.4"
eframe = "0.27"
image = { version = "0.25", default-features = false, features = ["png"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ureq = "2"
# Google Maps Platform: asenkron istekler Tokio + reqwest ile atılır.
tokio = { version = "1", features = ["rt-multi-thread", "time", "net"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
# API anahtarı kasası: AES-256-GCM + SHA-256 + base64.
aes-gcm = "0.10"
sha2 = "0.10"
base64 = "0.22"
getrandom = "0.2"

# Windows .exe dosya ikonu ve sürüm bilgisi (build.rs).
[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
""",
)

# 2) egui 0.27: DragValue::range yok, clamp_range var.
k = oku("src/katmanlar.rs")
k = k.replace(".range(1..=500)", ".clamp_range(1..=500)")
k = k.replace(".range(10..=10_000)", ".clamp_range(10..=10_000)")
k = k.replace(".range(1..=3)", ".clamp_range(1..=3)")
yaz("src/katmanlar.rs", k)

# 3) main.rs: eski OSM başlığı ve Overpass hedef fonksiyonları.
m = oku("src/main.rs")

bas = "// Keşif motoru: yalnızca OpenStreetMap"
if bas in m:
    i = m.index(bas)
    j = m.index("// ---------------------------------------------------------------------------", i)
    m = (
        m[:i]
        + """// Keşif motoru: yalnızca Google Maps Platform
//
// Tek veri kaynağı Google Places API (New) Text Search ve Geocoding'dir.
// Anahtar şifreli kasada durur, her istek kotadan pay ayırır; harita paneli
// ağa hiç çıkmaz. Aynı ilçe tekrar taranırken sonuç önbellekten okunur.
"""
        + m[j:]
    )

bas = "/// Overpass: bölgedeki web sitesi olmayan işletmeleri tek sorguda çeker."
son = "// ---------------------------------------------------------------------------"
if bas in m:
    i = m.index(bas)
    j = m.index(son, i)
    m = (
        m[:i]
        + '''/// Google Places: bölgedeki web sitesi olmayan işletmeleri tek sayfada çeker.
/// Günlük hedef listesi bu kayıtlardan doldurulur; kotadan bir istek harcar.
fn hedef_20(il: &str, ilce: &str, sektor: &str) -> Result<Vec<BusinessLead>, String> {
    let ayar = ayar_oku();
    let kasa = anahtar::Kasa::yukle();
    let anahtar = kasa.anahtar().ok_or_else(|| {
        "Google API anahtarı yok: üst şeritteki API & Kota Yönetim panelinden girin.".to_string()
    })?;
    let istemci = google::Istemci::yeni(
        anahtar.to_string(),
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        1,
    )?;
    let sonuc = istemci.ara(il, ilce, sektor)?;
    let mut liste: Vec<BusinessLead> = sonuc
        .kayitlar
        .into_iter()
        .map(|kayit| isletmeye_cevir(kayit, sektor))
        .filter(|lead| telefon_gecerli(&lead.phone))
        .collect();
    if liste.is_empty() {
        return Err(format!(
            "Google {ilce} / {il} için sitesiz işletme döndürmedi ({} web siteli kayıt elendi)",
            sonuc.siteli
        ));
    }
    incelemeye_gore_sirala(&mut liste);
    liste.truncate(hedef::GUNLUK_HEDEF);
    Ok(liste)
}

'''
        + m[j:]
    )

m = m.replace("/// Overpass'tan tek sorguda 20 sitesiz işletme çeker", "/// Google Places'ten tek sorguda 20 sitesiz işletme çeker")
m = m.replace('"[>] Overpass: {ilce} / {il} · {sektor} · sitesiz hedefler isteniyor..."', '"[>] Google Places: {ilce} / {il} · {sektor} · sitesiz hedefler isteniyor..."')
m = m.replace('"[✓] Overpass hedefleri: {} sitesiz kayıt alındı."', '"[✓] Google hedefleri: {} sitesiz kayıt alındı."')
yaz("src/main.rs", m)

print("düzeltmeler uygulandı")

for yol in [
    "src/main.rs",
    "src/katmanlar.rs",
    "src/saglik.rs",
    "src/demo_site.rs",
    "src/iller.rs",
    "src/musteriler.rs",
    "src/harita.rs",
]:
    for no, satir in enumerate(oku(yol).split("\n"), 1):
        if "overpass" in satir.lower() or "openstreetmap" in satir.lower():
            print(f"  {yol}:{no}: {satir.strip()[:110]}")
