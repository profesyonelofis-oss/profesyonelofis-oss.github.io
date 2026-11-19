"""saglik.rs ve katmanlar.rs içindeki OSM atıflarını Google'a çevirir."""

from pathlib import Path

# --- saglik.rs: harita denetimi artık Google Maps --------------------------
saglik = Path("src/saglik.rs")
metin = saglik.read_text(encoding="utf-8")

eski = '''/// Harita döşemelerine erişim + Google Places anahtarının durumu.
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
    if !(erisim("tile.openstreetmap.org:443") || erisim("www.openstreetmap.org:443")) {
        return Err(HARITA_HATA.to_string());
    }
    if google_anahtari_var() {
        Ok("Harita hazır: OpenStreetMap + Google Places anahtarı tanımlı.".to_string())
    } else {
        Ok("Harita hazır: OpenStreetMap (Google anahtarı yok, OSM kullanılacak).".to_string())
    }
}'''

if eski not in metin:
    raise SystemExit("saglik.rs: harita() bulunamadi")

yeni = '''/// Google Maps erişimi + Places/Static Maps anahtarının durumu.
///
/// Tek dış harita bağımlılığı Google Maps Platform'dur. Ağ yoksa denetim
/// kırmızıya döner; anahtar yoksa geçer ama kullanıcıya anahtar gerektiği
/// söylenir (ekran açılır, tarama başlatınca uygulama yol gösterir).
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
    if !(erisim("maps.googleapis.com:443")
        || erisim("places.googleapis.com:443")
        || erisim("www.google.com:443"))
    {
        return Err(HARITA_HATA.to_string());
    }
    if google_anahtari_var() {
        Ok("Google Maps hazır: Places, Geocoding ve Static Maps anahtarı tanımlı.".to_string())
    } else {
        Ok("Google Maps erişimi var; API anahtarı henüz girilmemiş.".to_string())
    }
}'''

saglik.write_text(metin.replace(eski, yeni, 1), encoding="utf-8")

# --- katmanlar.rs: harita alt başlığı -------------------------------------
katmanlar = Path("src/katmanlar.rs")
metin = katmanlar.read_text(encoding="utf-8")
metin = metin.replace('" · OpenStreetMap",', '" · Google Places",')
metin = metin.replace(
    "/// Katman 1'in ortasındaki interaktif harita (OpenStreetMap döşemeleri).",
    "/// Katman 1'in ortasındaki interaktif harita (Google Maps Static).",
)
katmanlar.write_text(metin, encoding="utf-8")
print("kucuk duzeltmeler tamam")
