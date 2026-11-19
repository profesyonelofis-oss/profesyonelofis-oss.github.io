"""saglik / iller / demo_site: Google kalıntılarını temizle (katmanlar ayrı geçti)."""

from pathlib import Path


def yukle(yol: str) -> str:
    return Path(yol).read_text(encoding="utf-8")


def yaz(yol: str, s: str) -> None:
    Path(yol).write_text(s, encoding="utf-8", newline="")


def rep(s: str, eski: str, yeni: str, adet: int = 1, ad: str = "") -> str:
    c = s.count(eski)
    assert c == adet, f"{ad}: beklenen {adet}, bulunan {c}"
    return s.replace(eski, yeni)


def kes(s: str, bas: str, son: str, yeni: str = "", ad: str = "") -> str:
    assert s.count(bas) == 1, f"{ad}: başlangıç yok/çok"
    i = s.index(bas)
    j = s.index(son, i)
    return s[:i] + yeni + s[j:]


# --- saglik.rs -------------------------------------------------------------
s = yukle("src/saglik.rs")
s = kes(
    s,
    "/// Google Maps erişimi + Places/Static Maps anahtarının durumu.",
    "/// Dosya yoksa/boşsa sorun değil",
    '''/// OpenStreetMap erişimi: Overpass API ve döşeme sunucusu.
///
/// Anahtar, kota veya faturalı servis yoktur; tek gereksinim ağ erişimidir.
/// Ağ yoksa denetim kırmızıya döner ve tarama başlatılamaz.
fn harita() -> Result<String, String> {
    let erisim = |adres: &str| -> bool {
        let Ok(mut adresler) = adres.to_socket_addrs() else {
            return false;
        };
        let Some(soket) = adresler.next() else {
            return false;
        };
        TcpStream::connect_timeout(&soket, Duration::from_secs(3)).is_ok()
    };
    if !(erisim("overpass-api.de:443")
        || erisim("overpass.kumi.systems:443")
        || erisim("tile.openstreetmap.org:443"))
    {
        return Err(HARITA_HATA.to_string());
    }
    Ok("OpenStreetMap hazır: Overpass ve döşeme sunucusu erişilebilir (anahtarsız).".to_string())
}

''',
    ad="saglik harita",
)
yaz("src/saglik.rs", s)

# --- iller.rs --------------------------------------------------------------
s = yukle("src/iller.rs")
s = rep(
    s,
    "//! (volkansenturk/turkiye-iller-ilceler); harita sorgusu Google Geocoding ile yapılır.",
    "//! (volkansenturk/turkiye-iller-ilceler); konum çözümü Nominatim (OSM) ile yapılır.",
    ad="iller doc",
)
yaz("src/iller.rs", s)

# --- demo_site.rs ----------------------------------------------------------
s = yukle("src/demo_site.rs")
s = rep(
    s,
    '''    let gomulu = format!("https://maps.google.com/maps?q={enlem:.5},{boylam:.5}&z=17&output=embed");
    let ac = format!("https://www.google.com/maps/search/?api=1&query={enlem:.5},{boylam:.5}");''',
    '''    // Anahtarsız OSM gömme haritası ve bağlantısı.
    let d = 0.008;
    let gomulu = format!(
        "https://www.openstreetmap.org/export/embed.html?bbox={:.5}%2C{:.5}%2C{:.5}%2C{:.5}&layer=mapnik&marker={enlem:.5}%2C{boylam:.5}",
        boylam - d,
        enlem - d,
        boylam + d,
        enlem + d
    );
    let ac = format!(
        "https://www.openstreetmap.org/?mlat={enlem:.5}&mlon={boylam:.5}#map=17/{enlem:.5}/{boylam:.5}"
    );''',
    ad="demo harita",
)
s = s.replace('kalemler.push(("Google".to_string()', 'kalemler.push(("Yorumlar".to_string()')
s = rep(
    s,
    '    /// Galeri kaynağının kısa açıklaması (ör. "Google Street View · işletmenin',
    '    /// Galeri kaynağının kısa açıklaması (ör. "işletmenin fotoğrafları ·',
    ad="demo galeri doc",
)
yaz("src/demo_site.rs", s)

print("tamam: saglik, iller, demo_site")
