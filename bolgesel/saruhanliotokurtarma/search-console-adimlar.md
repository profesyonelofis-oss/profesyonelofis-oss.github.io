# Search Console · sitemap gönderimi (2 dakika)

> **Bana gereken tek şey:** aşağıdaki 3 adımı yapıp çıkan **doğrulama kodunu** yazman.
> Kod gelince meta etiketi + doğrulama dosyasını ekleyip siteyi 1 dakikada yeniden yayınlıyorum.
> Doğrudan bağlantı: `https://search.google.com/search-console/welcome`

Sitemap hazır ve yayında: `https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page/sitemap.xml` (9 adres).
Site `freebuff.page` alt alan adında olduğu için **alan adı (Domain) doğrulaması yapılamaz**; URL öneki yöntemi kullanılır.

## Adımlar

1. `https://search.google.com/search-console` → **Add property / Mülk ekle**
2. **URL prefix / URL öneki** seç, şunu yaz (sonunda slash olsun):
   `https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page/`
3. Doğrulama yöntemi olarak **HTML etiketi** (veya HTML dosyası) seç. Google’ın verdiği değeri kopyala:
   - HTML etiketi ise: `content="..."` içindeki uzun kod
   - HTML dosyası ise: `google1234abcd.html` dosyasının adı
4. Kodu bana gönder. Ben siteye ekleyip yeniden yayınlıyorum (yaklaşık 1 dakika).
5. Google’da **Verify / Doğrula** düğmesine bas.
6. Doğrulandıktan sonra: sol menü **Sitemaps** → `sitemap.xml` yaz → **Submit**.
7. **URL Inspection** → şu iki adresi tek tek sorgula → **Request indexing**:
   - `https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page/`
   - `https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page/manisa-saruhanli`
8. 3–7 gün sonra **Pages / Sayfalar** raporuna bak: 9 adresin “Indexed” olduğunu doğrula.

## Teknik araç (benim tarafımda hazır)

Kod eline geçtiğinde şu komut doğrulama dosyasını ve meta etiketini tüm sayfalara ekler:

```bash
python araclar/google_dogrula.py <DOGRULAMA_KODU>
```

Sonra `python araclar/yayin_hazirla.py https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page`
ve yükleme. Meta etiketi zaten varsa tekrar eklenmez.

## Nelere dikkat

- `robots.txt` yayında ve sitemap’i gösteriyor; engel yok.
- Sayfalar `index,follow` ve canonical’ları kendi adreslerini gösteriyor.
- Sitemap’te `lastmod` var; her içerik değişiminde yayın betiği tarihi yeniler.
- Kendi alan adını bağlarsan bu adımı yeni adresle bir kez daha yaparsın.
