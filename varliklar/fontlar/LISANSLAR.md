# Gömülü fontlar

Demo sayfaları bu fontları **base64 olarak kendi içinde** taşır; dış istek yok.
Hepsi **SIL Open Font License 1.1**: ticari kullanım serbest, müşteri işine
gömülebilir, atıf zorunlu değil.

| Dosya | Font | Kullanım | Lisans | Kaynak |
|---|---|---|---|---|
| `space-grotesk-tr.woff2` | Space Grotesk (değişken, wght 300–700) | Mağaza, kurum, ofis, teknik servis başlıkları | SIL OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/spacegrotesk |
| `geist-latin.woff2` / `geist-latin-ext.woff2` | Geist (değişken, wght 400–600) | Tüm sayfalarda gövde metni | SIL OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/geist |
| `instrument-serif-latin.woff2` / `instrument-serif-latin-ext.woff2` | Instrument Serif (400) | Restoran, kafe, pastane, kuaför, klinik, otel, eğlence başlıkları | SIL OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/instrumentserif |
| `cormorant-garamond-latin.woff2` / `-latin-ext.woff2` | Cormorant Garamond (değişken, wght 300–600) | "Gece" yerleşiminin manşet ve başlıkları | SIL OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/cormorantgaramond |
| `sora-latin.woff2` / `sora-latin-ext.woff2` | Sora (değişken, wght 300–600) | "Gece" yerleşiminin gövde metni | SIL OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/sora |
| `plus-jakarta-sans-latin.woff2` / `-latin-ext.woff2` | Plus Jakarta Sans (değişken, wght 200–800) | "Işık" yerleşiminin gövde metni | SIL OFL 1.1 | https://github.com/google/fonts/tree/main/ofl/plusjakartasans |

## Neden bu çift

- **Instrument Serif** sıcak sektörlerin başlığında: yüksek kontrastlı, editoryal;
  büyük puntoda "ajans işi" duruyor. Gövdede kullanılmıyor (küçük puntoda okunaksız).
- **Space Grotesk** mağaza/kurum/teknik sektörlerde: iki demo yan yana
  konduğunda aynı şablon gibi durmasın diye başlık yüzü sektöre göre değişiyor.
- **Geist** gövdede. **Inter bilinçli olarak kullanılmıyor**: AI üretimi
  sitelerin en belirgin tipografi izi.
- Üçü de Türkçe karakterlerin tamamını (ı İ ş Ş ğ Ğ ç Ç ö Ö ü Ü) içeriyor;
  `unicode-range` ile latin / latin-ext parçaları ayrıldı.

## Boyut (base64 öncesi)

| Dosya | Boyut |
|---|---|
| `space-grotesk-tr.woff2` | 14 KB |
| `geist-latin.woff2` + `geist-latin-ext.woff2` | 29 KB + 17 KB |
| `instrument-serif-latin.woff2` + `instrument-serif-latin-ext.woff2` | 21 KB + 12 KB |

Geist ve Instrument Serif doğrudan Google Fonts'un `latin` / `latin-ext`
parçaları olarak indirildi (woff2, alt küme sunucuda yapılıyor):

```bash
UA="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
curl -s -A "$UA" "https://fonts.googleapis.com/css2?family=Geist:wght@400..600&display=swap"
curl -s -A "$UA" "https://fonts.googleapis.com/css2?family=Instrument+Serif&display=swap"
# CSS'teki /* latin */ ve /* latin-ext */ bloklarındaki woff2 adresleri indirilir
```

Space Grotesk daha önce Türkçe + ASCII alt kümesi olarak fontTools ile
üretilmişti; Türkçe harfleri içerdiği için tek dosya yeterli.
