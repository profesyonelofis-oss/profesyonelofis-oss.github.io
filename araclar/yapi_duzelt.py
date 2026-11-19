"""Yapısal düzeltmeler: kod sırası, clippy izni, OSM metinleri."""

from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")

# 1) Test modülünden sonra kalan keşif motorunu yukarı taşı.
BASLIK = "// Keşif motoru: yalnızca Google Maps Platform"
bas = metin.index(BASLIK)
# Başlığın üstündeki ayırıcı çizgiye kadar geri git.
bas = metin.rindex("// ---------------------------------------------------------------------------\n", 0, bas)
ek = metin[bas:]
metin = metin[:bas].rstrip() + "\n"

test = metin.index("#[cfg(test)]\nmod testler {")
metin = metin[:test] + ek.rstrip() + "\n\n" + metin[test:]

# 2) Sekiz argümanlı tarama yardımcısı: projede aynı desen var.
metin = metin.replace(
    "fn ilce_ara(\n    tx: &Sender<Mesaj>,",
    "#[allow(clippy::too_many_arguments)]\nfn ilce_ara(\n    tx: &Sender<Mesaj>,",
)

# 3) Kullanıcıya görünen OSM metinleri.
metin = metin.replace(
    "OpenStreetMap kayıtları çekiliyor, sonuçlar birazdan listelenecek.",
    "Google Places kayıtları çekiliyor, sonuçlar birazdan listelenecek.",
)
metin = metin.replace(
    "Sektör OpenStreetMap'te çok seyrek olabilir.",
    "Sektör Google Places'ta çok seyrek olabilir.",
)

# 4) Kalan OSM atıfları (yorum ve etiketler).
metin = metin.replace("OpenStreetMap'te", "Google Maps'te")
metin = metin.replace("OpenStreetMap", "Google Maps")

YOL.write_text(metin, encoding="utf-8", newline="")

# 5) Harita alt başlığı.
katmanlar = Path("src/katmanlar.rs")
k = katmanlar.read_text(encoding="utf-8")
k = k.replace(
    '"{sektor} · {} kayıt · {} koordinatsız · OpenStreetMap",',
    '"{sektor} · {} kayıt · {} koordinatsız · Google Places",',
)
katmanlar.write_text(k, encoding="utf-8")

# 6) İl/ilçe veri kaynağı notu.
iller = Path("src/iller.rs")
i = iller.read_text(encoding="utf-8")
i = i.replace(
    "//! Kaynak: OpenStreetMap/Nominatim sorgularında kullanılmak üzere derlenmiş\n//! idari bölünme listesi (volkansenturk/turkiye-iller-ilceler).",
    "//! Kaynak: kamuya açık idari bölünme listesi\n//! (volkansenturk/turkiye-iller-ilceler); harita sorgusu Google Geocoding ile yapılır.",
)
iller.write_text(i, encoding="utf-8")
print("yapi duzeltmeleri tamam")
