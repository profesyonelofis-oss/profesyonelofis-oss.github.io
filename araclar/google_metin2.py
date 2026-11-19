"""Arayüzde kalan OSM/Overpass metinlerini Google'a çevirir."""

import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return Path(yol).read_text(encoding="utf-8")


def yaz(yol, metin):
    Path(yol).write_text(metin, encoding="utf-8", newline="")


m = oku("src/main.rs")
degisimler = [
    ('kontur_pill(ui, "OpenStreetMap · anahtarsız")', 'kontur_pill(ui, "Google Places · kota kontrollü")'),
    ('egui::RichText::new("OpenStreetMap · canlı")', 'egui::RichText::new("Google Places · canlı")'),
    ('kontur_pill(ui, "OSM · Overpass")', 'kontur_pill(ui, "Places API (New)")'),
    (
        '"OpenStreetMap kayıtları çekiliyor, sonuçlar birazdan listelenecek."',
        '"Google Places kayıtları çekiliyor, sonuçlar birazdan listelenecek."',
    ),
    (
        '"Liste boş: önce bir bölge taraması yapın ya da Overpass\'tan 20 hedef çekin."',
        '"Liste boş: önce bir bölge taraması yapın ya da Google\'dan 20 hedef çekin."',
    ),
    ('"Overpass\'tan 20 hedef çek"', '"Google\'dan 20 hedef çek"'),
    (
        '"https://www.openstreetmap.org/?mlat={enlem}&mlon={boylam}#map=17/{enlem}/{boylam}"',
        '"https://www.google.com/maps?q={enlem},{boylam}"',
    ),
    ('let kaynak = "OpenStreetMap";', 'let kaynak = "Google Maps";'),
    ('(OpenStreetMap) otomatik olarak üretildi', '(Google Maps) otomatik olarak üretildi'),
    (
        "// Veri kaynağı: OpenStreetMap (Overpass + Nominatim + OSM döşemeleri)",
        "// Veri kaynağı: Google Places API (New) + Geocoding",
    ),
    (
        "/// tarandığında Overpass'a ikinci istek atılmaz.",
        "/// tarandığında Google'a ikinci istek atılmaz.",
    ),
    (
        "/// Kaydın kaynak bağlantısı: hepsi OpenStreetMap kaydıdır.",
        "/// Kaydın kaynak bağlantısı: Google Maps kaydı.",
    ),
    (
        "/// sayaçtan bir mesaj harcar. Liste temizlenebilir, Overpass'tan 20 yeni",
        "/// sayaçtan bir mesaj harcar. Liste temizlenebilir, Google'dan 20 yeni",
    ),
]
for eski, yeni in degisimler:
    if eski in m:
        m = m.replace(eski, yeni)
    else:
        print("ATLANDI:", eski[:70])
yaz("src/main.rs", m)

k = oku("src/katmanlar.rs")
k = k.replace("· OpenStreetMap\"", "· Google Places\"")
yaz("src/katmanlar.rs", k)
print("arayüz metinleri güncellendi")

for yol in ["src/main.rs", "src/katmanlar.rs"]:
    for no, satir in enumerate(oku(yol).split("\n"), 1):
        if "overpass" in satir.lower() or "openstreetmap" in satir.lower():
            print(f"  {yol}:{no}: {satir.strip()[:110]}")
