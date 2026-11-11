"""Kalan Overpass/Nominatim yorumlarını ve test verisini temizler."""

from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")


def degistir(eski: str, yeni: str) -> None:
    global metin
    if eski not in metin:
        raise SystemExit("bulunamadi:\n" + eski[:220])
    metin = metin.replace(eski, yeni, 1)


degistir(
    """// Gerçek veri kaynağı: Google Maps (Nominatim + Overpass API)
//
// İlçe sınırı Nominatim ile koordinata çevrilir, işletmeler Overpass API'den
// gerçek OSM kayıtları olarak çekilir. Web sitesi etiketi (website/contact:
// website/url/contact:url) bulunmayan işletmeler listelenir.""",
    """// Gerçek veri kaynağı: Google Maps Platform
//
// İl/ilçe merkezi Google Geocoding ile koordinata çevrilir, işletmeler
// Places Text Search'ten çekilir, harita Static Maps ile çizilir. Web sitesi
// olmayan (ya da yalnızca sosyal medya kullanan) işletmeler listelenir.""",
)

degistir(
    "/// İstek başına en fazla deneme sayısı (Google, Nominatim).",
    "/// İstek başına en fazla deneme sayısı (Google Maps Platform).",
)

degistir(
    '/// aralığına kırpılarak. Overpass 429\'da "ne kadar bekleyin" bilgisini bu',
    '/// aralığına kırpılarak. Google 429\'da "ne kadar bekleyin" bilgisini bu',
)

degistir(
    """// OSM/Overpass ücretsiz ve verisi saklanabilir; ancak Türkiye'de kapsama
// boşlukları var. Yedek katman yalnızca OSM'in boş döndüğü ilçelerde devreye
// girer ve kendi istek bütçesini tutar: Google'ın ücretsiz tavanı (Text Search
// Enterprise 1.000/ay) aşılmasın diye günlük/aylık sayaç `google_kota.json`
// dosyasında saklanır. Google sonuçları önbelleğe alınmaz (şartlar yalnızca
// place ID'nin kalıcı saklanmasına izin verir); bizde her tarama canlı çeker.""",
    """// Tek veri kaynağı Google Maps Platform'dur. Ücretsiz tavan (Text Search
// Enterprise 1.000/ay) aşılmasın diye günlük/aylık sayaç `google_kota.json`
// dosyasında tutulur; her sayfa ayrı faturalanır. Sonuçlar ilçe+sektör
// bazında kısa süreliğine yerel önbelleğe alınır (SONUC_TTL_GUN), böylece
// aynı bölge yeniden taranınca kota ikinci kez harcanmaz.""",
)

degistir(
    "    /// Overpass sorgusunda kullanılan (anahtar, regex) çiftleri.\n",
    "",
)

degistir(
    'kaynak: "https://www.openstreetmap.org/node/42".to_string(),',
    'kaynak: "https://www.google.com/maps/place/?q=place_id:test".to_string(),',
)

YOL.write_text(metin, encoding="utf-8", newline="")
print("yorumlar temizlendi")
