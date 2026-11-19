# Google Search Console — doğrulama ve sitemap gönderimi

Site: **https://profesyonelofis-oss.github.io**
Yayın: GitHub Pages (`profesyonelofis-oss/profesyonelofis-oss.github.io`, `main` dalı, kök klasör)
Hazır dosyalar: `robots.txt` (`Allow: /` + sitemap satırı), `sitemap.xml` (11 adres)

---

## 1) Mülk ekleyin

1. https://search.google.com/search-console → sol üstteki mülk seçici → **Add property**
2. **URL prefix** seçin (Domain seçmeyin; DNS kaydı gerekir)
3. Adresi tam yazın: `https://profesyonelofis-oss.github.io/`
4. **Continue**

---

## 2) Doğrulama (iki yoldan biri yeterli)

### YOL A — HTML dosyası (en kolay)

1. GSC ekranında **HTML file** seçeneğini açın → **Download file**
   (adı `google1a2b3c4d5e6f7890.html` gibi olur)
2. İndirilen dosyayı şu komutla yayına alın:

```bash
python araclar/gsc_dogrula.py --dosya "C:\Users\Ateş\Downloads\google1a2b3c4d5e6f7890.html"
```

Komut siteyi yeniden üretir, dosyayı köke ekler, GitHub Pages'e push eder ve
`https://profesyonelofis-oss.github.io/google....html` adresinin **200** verdiğini
kontrol eder. "doğrulama canlıda hazır." yazısını görünce 3. adıma geçin.

3. GSC'ye dönüp **Verify** düğmesine basın.

### YOL B — HTML etiketi (dosya indirmeden)

1. GSC ekranında **HTML tag** seçeneğini açın, `content="..."` içindeki değeri kopyalayın
2. Şu komutu çalıştırın:

```bash
python araclar/gsc_dogrula.py --meta "GSC-NIN-VERDIGI-DEGER"
```

Bu, `<meta name="google-site-verification" …>` etiketini ana sayfaya ekler, yayınlar
ve canlıda göründüğünü kontrol eder.

> Her iki yöntemde de token `dogrulama/` klasöründe saklanır; sonraki yayınlarda
> silinmez, doğrulama kalıcı kalır.

---

## 3) Sitemap gönderin

1. Sol menü → **Indexing → Sitemaps**
2. **Add a new sitemap** → `sitemap.xml` yazın → **Submit**
3. Durum birkaç saat içinde **Success** olur; 11 adresin tamamı keşfedilir

Sitemap içeriği (canlı): `https://profesyonelofis-oss.github.io/sitemap.xml`
`robots.txt` bu dosyayı kendisi bildiriyor, ayrıca ekleme zorunlu değil ama hızlandırır.

---

## 4) İndekslemeyi hızlandırın

1. Üstteki arama kutusuna `https://profesyonelofis-oss.github.io/` yazın
2. **URL Inspection** → **Request indexing**
3. Aynı işlemi `https://profesyonelofis-oss.github.io/cekici` için tekrarlayın
   (en çok aranan sayfa bu)

İndeksleme genelde 1–7 gün sürer; "Discovered – currently not indexed" normaldir,
birkaç gün sonra kendiliğinden indekslenir.

---

## 5) Kontrol listesi

| Kontrol | Beklenen |
|---|---|
| `https://profesyonelofis-oss.github.io/robots.txt` | `User-agent: *` / `Allow: /` + sitemap satırı |
| `https://profesyonelofis-oss.github.io/sitemap.xml` | 11 `<loc>`, hepsi GitHub adresinde |
| Sayfa kaynağında `canonical` | `https://profesyonelofis-oss.github.io/...` |
| Lighthouse → SEO | 100 (mobil + masaüstü) |
| GSC doğrulama | dosya 200 / meta etiketi kaynakta görünür |

Doğrulama durumunu tekrar sorgulamak için:

```bash
python araclar/gsc_dogrula.py --kontrol
```

---

## Güncelleme: tek komutla yayın

Site içeriği değiştiğinde doğrulama dosyasını yeniden eklemek gerekmez; tek komut
yeterlidir:

```bash
python araclar/yayinla.py                                  # üret + GitHub Pages'e yayınla
python araclar/yayinla.py --surge                          # surge aynasını da güncelle
python araclar/yayinla.py --mesaj "Yeni fiyat tablosu"     # commit mesajı
python araclar/yayinla.py --kuru                           # sadece üret, push etme
```

Betik sırayla: siteyi üretir → `dogrulama/` içindeki GSC dosyasını/meta etiketini
pakete ekler (doğrulama kalıcı kalır) → depoya push eder → Pages derlemesini bekler →
`/`, `/cekici`, `/robots.txt`, `/sitemap.xml` adreslerinin 200 döndüğünü kontrol eder.

---

## Notlar

- **Surge adresi** (`saruhanliotokurtarma.surge.sh`) GSC'ye eklenmemeli: Surge ücretsiz
  alt alan adlarında `robots.txt`yi `Disallow: /` olarak kendisi yazıyor, o adres
  indekslenemez. Surge yalnızca paylaşım aynası; canonical GitHub'ı gösteriyor.
- **freebuff.page** adresi de aynı build'i sunar ve canonical'ı GitHub'a verir; GSC'de
  tek mülk (GitHub) yeterli.
- Site güncellenince sitemap'i yeniden göndermek gerekmez; GSC `lastmod`/tarama ile
  kendisi alır. Yeni sayfa eklerseniz `araclar/yayin_hazirla.py` sitemap'i zaten üretir.
- Bing/Yandex için IndexNow anahtarı kökte hazır: `9f2c7a41d8b64e0fa53c1d7e6b902c48.txt`.
