# Alan adı (URL) planı

## Bugünkü durum

Yayın adresi: `https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page`
(48 karakter, tahmin edilemez). Adres platform tarafından `site-<uuid>` olarak
üretiliyor; `sites` verisinde `domain = freebuff.page`, `subdomain = null`.
Araçlarda alan adı **yazma** yetkisi yok, yani bu adres kodla kısaltılamaz.

Alan adı desteği platformda var (site kaydı silinirken "custom domains" de
kaldırılıyor); bağlama işlemi **Sites panelinden** yapılır.

## En ucuz ilk deneme (ücretsiz, 5 dakika)

Sites panelinde site ayarlarında alan/alt alan adı alanı varsa kısa bir ad dene
(ör. `saruhanlio`). Adres yine `freebuff.page` olur ama akılda kalır ve
kartvizit/WhatsApp'ta yazılabilir hâle gelir. Panel yazmaya izin vermiyorsa
aşağıdaki alan adı yoluna geç.

**Dışarıdan yoklanamaz:** `freebuff.page` joker yanıt veriyor; `saruhanlio`,
`saruhankurtarma` ve `rastgele-yok-12345` adreslerinin hepsi aynı `404`'ü
döndürüyor. Yani "bu ad boşta mı" sorusunun cevabı yalnızca panelde.

Adresi panelde kaydettikten sonra doğrulama ve canonical güncellemesi tek
komutla yapılır:

```bash
python araclar/adres_yenile.py                     # hangi adres bu siteyi sunuyor?
python araclar/adres_yenile.py https://yeni-adres  # doğrula + tüm canonical'ları çevir
```

Betik adresin gerçekten bu siteyi sunduğunu (başlık + telefon işareti)
kontrol eder; doğrulamazsa paketlemeyi yapmaz.

## Boşta olan adaylar (11 Kasım 2026, RDAP sorgusu)

| Alan adı | Durum | Not |
|---|---|---|
| `saruhanliotokurtarma.com` | **boşta** | Tam eşleşme; "oto kurtarma" aramasında en güçlü |
| `saruhankurtarma.com` | **boşta** | En kısa; telefon/kartvizit için ideal |
| `saruhanlicekici.com` | **boşta** | "çekici" aramalarına yakın |
| `cekicisaruhanli.com` | **boşta** | Alternatif sıralama |
| `saruhanlioto.com` | **boşta** | Çok kısa, jenerik |
| `otokurtarmasaruhanli.com` | **boşta** | Uzun ama anahtar kelime dolu |
| `otocekici.com` | kayıtlı | — |
| `oto-kurtarma.com` | kayıtlı | — |

Öneri: **`saruhanliotokurtarma.com`** (SEO) + yönlendirme için
`saruhankurtarma.com`. `.com.tr` daha ucuzdur ama kayıt için belge (vergi
levhası) ister; alırsan `saruhanliotokurtarma.com.tr` de aynı şekilde bağlanır.

## Geçiş: tek komut

Alan adını aldıktan ve panelde bağladıktan sonra:

```bash
python araclar/yayin_hazirla.py https://saruhanliotokurtarma.com
```

Bu komut canonical, `og:url`, JSON-LD `url`, `sitemap.xml` ve `robots.txt`
içindeki adresi **tek seferde** yeni alan adına çevirir (eski `freebuff.page`
adresi ve `ornek-alan-adi.com` yer tutucusu birlikte değiştirilir). Sonra
`yayin/deploy.json` ile yayına alınır.

## Geçiş sonrası kontrol listesi

- [ ] `https://` ve `www` tek adrese yönleniyor (tercihen `www` → kök)
- [ ] Ana sayfa + 7 ilçe + `/cekici` sayfasında canonical yeni adreste
- [ ] `/sitemap.xml` içinde tek alan adı var, eski host kalmadı
- [ ] Search Console: yeni mülk ekle, `sitemap.xml` gönder
- [ ] Google profilinde "Web sitesi" alanını yeni adresle değiştir
- [ ] `araclar/indexnow.py` ile yeni adresleri bildir
- [ ] Eski `freebuff.page` adresi açık kalsın (kırık bağlantı olmasın)

## Yeni adres: `https://saruhanliotokurtarma.surge.sh` (yayında)

- Site surge.sh üzerinde statik olarak yayınlanıyor. Canonical, `og:url`,
  JSON-LD, `sitemap.xml` ve `robots.txt` bu adresi gösteriyor. Eski
  `freebuff.page` adresi de açık ve canonical'ı yeni adrese çevrili.
- Alt sayfalar sonda eğik çizgiyle çalışır (`/cekici/`); `/cekici` → `301`.
- Yeniden yayınlama tek komut:

  ```bash
  python araclar/surge_hazirla.py --yayinla
  ```

- Hesap: `kuzeykaraoglu2017@gmail.com` (e-posta doğrulandı). Jeton
  `_gecici/surge_token.txt` içinde, sürüm kontrolüne girmiyor.
- Yeni içerik/klip ekledikten sonra hem surge'a hem `freebuff.page` adresine
  yayınla; canonical'lar paketleme sırasında otomatik ayarlanır.
- **Bilinen sınır:** surge'un ücretsiz alt alan adında `/robots.txt` her zaman
  `Disallow: /` dönüyor (adrese `?v=1` eklenince kendi dosyamız geliyor). Google
  bu adresi taramaz; bu yüzden canonical/`og:url`/sitemap **freebuff.page**
  adresinde kalıyor. Kısa adres müşteriye verilen/WhatsApp'ta paylaşılan adres
  olarak çalışır, arama trafiği freebuff adresinden yürür.
- Kalıcı ve tam çözüm aynı: kendi alan adı (`.com`), robots engeli olmadan
  tek adres.

## Not

Müşteri siteye Google'dan tıklayarak geliyor; adres uzunluğu tıklamayı
engellemiyor. Alan adı; reklam, kartvizit, tabela ve "bizi internette bul"
dendiğinde işe yarıyor. Bu yüzden öncelik sırası: telefon numarası → Google
profili → alan adı.
