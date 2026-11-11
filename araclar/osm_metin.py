"""main.rs ve yan dosyalarda kalan Google metinlerini temizler."""

import re
from pathlib import Path

p = Path("src/main.rs")
s = p.read_text(encoding="utf-8")

# Kaynak etiketi: her iki blok da tek satıra iner.
s, n = re.subn(
    r'    let kaynak = if isletme\.kaynak\.contains\("google[^\n]*\n        "Google"\n    \} else \{\n        "Google Maps"\n    \};',
    '    let kaynak = "OpenStreetMap";',
    s,
)
print("kaynak bloğu:", n)

degisimler = [
    (
        '"https://www.google.com/maps/search/?api=1&query={enlem},{boylam}"',
        '"https://www.openstreetmap.org/?mlat={enlem}&mlon={boylam}#map=17/{enlem}/{boylam}"',
    ),
    ('"Google\'da {}★ ve {adet} yorumunuz var"', '"Kaydınızda {}★ ve {adet} yorum var"'),
    ('"Google\'da {adet} yorumunuz var"', '"Kaydınızda {adet} yorum var"'),
    ('"Google\'da {}★ puanınız var"', '"Kaydınızda {}★ puan var"'),
    ('"Google Street View · işletmenin sokağından"', '"İşletmenin sokağından"'),
    ('"Google Street View"', '"Sokak görüntüsü"'),
    ('"Google\'da yorum ve puan verisi yok"', '"Harita kaydında yorum ve puan verisi yok"'),
    (
        '"Google işletme profilinize haftalık saatleri girin; \\"şu an açık\\" bilgisi görünür."',
        '"İşletme kaydınıza haftalık saatleri girin; \\"şu an açık\\" bilgisi görünür."',
    ),
    ('"Google yorumları"', '"Müşteri yorumları"'),
    (
        '"Müşterilerinizden Google yorumu isteyin; 10 ve üzeri yorum güven oluşturur."',
        '"Müşterilerinizden yorum isteyin; 10 ve üzeri yorum güven oluşturur."',
    ),
    (
        "Müşteri işletmenizi Google'da ararken bu bilgileri göremediğinde",
        "Müşteri işletmenizi haritada ararken bu bilgileri göremediğinde",
    ),
    (
        "Bu karne, işletmenizin herkese açık kayıt bilgilerinden (Google / Google Maps) otomatik olarak üretildi",
        "Bu karne, işletmenizin herkese açık kayıt bilgilerinden (OpenStreetMap) otomatik olarak üretildi",
    ),
    (
        '"Kendi alan adınızda mobil uyumlu bir site açın; Google profilinize ekleyin."',
        '"Kendi alan adınızda mobil uyumlu bir site açın; işletme kaydınıza ekleyin."',
    ),
    (
        "// --- Kota şeridi: Google ve WhatsApp hakları -------------------------",
        "// --- Veri kaynağı şeridi ve WhatsApp freni ---------------------------",
    ),
    (
        "/// tarandığında Google kotası ikinci kez harcanmaz.",
        "/// tarandığında Overpass'a ikinci istek atılmaz.",
    ),
    (
        "/// sayaçtan bir mesaj harcar. Liste temizlenebilir, Google'dan 20 yeni",
        "/// sayaçtan bir mesaj harcar. Liste temizlenebilir, Overpass'tan 20 yeni",
    ),
    (
        "/// meslek kolları ve Google Business Profile kategorileri (berber, tırnak",
        "/// meslek kolları ve harita işletme kategorileri (berber, tırnak",
    ),
    (
        "/// Google kaynağında sıralama inceleme sayısına göredir: gerçek müşteri",
        "/// Eski harici kayıtlarda sıralama inceleme sayısına göredir: gerçek müşteri",
    ),
    (
        "/// trafiği olan işletme önce gelir (Google'ın kendi sonuç sırası bölgeye göre",
        "/// trafiği olan işletme önce gelir (kaynağın kendi sonuç sırası bölgeye göre",
    ),
    (
        "/// Mesaja giren somut gerekçe: işletmenin kendi kaydından (Google yorum/puan,",
        "/// Mesaja giren somut gerekçe: işletmenin kendi kaydından (yorum/puan,",
    ),
    (
        "///   Google yorum/puan yok           -15   (10+ yorum -5, 50+ yorum 0,",
        "///   yorum/puan yok                 -15   (10+ yorum -5, 50+ yorum 0,",
    ),
    (
        "/// Demo galerisi: sırasıyla (1) işletmenin Google fotoğrafları,",
        "/// Demo galerisi: sırasıyla (1) işletmenin kendi fotoğrafları,",
    ),
    (
        "/// (2) işletmenin koordinatından Google Street View karesi,",
        "/// (2) sektörün gömülü örnek kareleri,",
    ),
    (
        "/// Kayıt kimliği: OSM node/way numarası veya Google place id'sinin son parçası.",
        "/// Kayıt kimliği: OSM node/way numarası (eski kayıtlarda place id).",
    ),
    (
        "/// hiçbir ağ isteği yapılmaz, yani karne Google kotası harcamaz.",
        "/// hiçbir ağ isteği yapılmaz.",
    ),
    (
        "/// - `google:<places/.../photos/...>` — Places Photo API'sinden indirilir\n",
        "",
    ),
]
for eski, yeni in degisimler:
    if eski in s:
        s = s.replace(eski, yeni)
    else:
        print("ATLANDI:", eski[:60])

p.write_text(s, encoding="utf-8", newline="")

hedef = Path("src/hedef.rs")
h = hedef.read_text(encoding="utf-8")
h = h.replace("//! Tarama (OSM/Google) 20 kayıt", "//! Tarama (OSM/Overpass) 20 kayıt")
hedef.write_text(h, encoding="utf-8", newline="")

k34 = Path("src/katman34.rs")
k = k34.read_text(encoding="utf-8")
k = k.replace("aksi hâlde site Google'da hiç görünmez.", "aksi hâlde site haritada hiç görünmez.")
k34.write_text(k, encoding="utf-8", newline="")

mus = Path("src/musteriler.rs")
m = mus.read_text(encoding="utf-8")
m = m.replace(
    "/// OSM kaydı için node/way kimliği, Google için place ID, yoksa ad+koordinat.",
    "/// OSM kaydı için node/way kimliği (eski kayıtlarda place ID), yoksa ad+koordinat.",
)
mus.write_text(m, encoding="utf-8", newline="")

print("kalan Google satırı:", sum(1 for x in s.split("\n") if "Google" in x or "google" in x))
for no, satir in enumerate(s.split("\n"), 1):
    if "Google" in satir or "google" in satir:
        print(f"  {no}: {satir.strip()[:90]}")
