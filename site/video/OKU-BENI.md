# Hero videolarını değiştirme

Ana sayfa (`/`) her açılışta bu klasördeki kliplerden birini rastgele oynatır.
Kendi çekimleriniz geldiğinde aşağıdaki adımlar yeterli; kod değişmez.

## 1. Dosyalar nerede

| Klip | Video | Poster (ilk kare) |
|---|---|---|
| `koridor` | `site/video/hero/koridor.mp4` | `site/video/hero/koridor.jpg` |
| `gunduz` | `site/video/hero/gunduz.mp4` | `site/video/hero/gunduz.jpg` |
| `gece` | `site/video/hero/gece.mp4` | `site/video/hero/gece.jpg` |
| `yagmur` | `site/video/hero/yagmur.mp4` | `site/video/hero/yagmur.jpg` |
| `kaza` | `site/video/hero/kaza.mp4` | `site/video/hero/kaza.jpg` |

Liste `site/anasayfa.html` içindeki `klipler` dizisinde tanımlı. Dosya adı ile
dizideki ad **birebir** aynı olmalı.

## 2. Video formatı

- MP4 / H.264, `yuv420p`, 24–30 fps
- 960×540 (16:9). 1280×720 de olur, boyut artar.
- **Ses yok** (autoplay sessiz çalışır; ses varsa da oynatılmaz)
- 6 saniye, başı ve sonu birbirine bağlanan (dikişsiz) döngü
- Klip başına hedef boyut: **≤ 300 KB** (mobil veri)
- `+faststart` (moov başta) — tarayıcı beklemeden oynatır

Poster: 960×540 JPEG, kalite ~84, ~40 KB.

## 3. Kendi çekimini klibe çevirme (tek komut)

Dikey/yatay telefon çekimi doğrudan verilir; kırpma, 6 saniyeye indirme,
sıkıştırma, sesin atılması, dikişsiz döngü ve poster tek komutta üretilir:

```bash
python araclar/klip_hazirla.py cekim.mp4 --ad kaza
python araclar/klip_hazirla.py cekim.mp4 --ad gece --baslangic 12   # 12. saniyeden
python araclar/klip_hazirla.py cekim.mp4 --ad gunduz --crf 33      # daha küçük dosya
```

Tüm çekimler bir klasördeyse tek komut yeterlidir: hepsi sırayla hazırlanır,
özet tablo basılır ve yayın adımı çalışır.

```bash
python araclar/klip_hazirla.py cekimler/ --yayinla
```

- Klasör modunda **değişmemiş çekimler atlanır**: kaynak dosyanın boyutu ve
  zaman damgası ile dönüştürme seçenekleri (başlangıç, süre, crf, kırpma) aynıysa
  yeniden kodlanmaz. Durum `_gecici/klip_durum.json` dosyasında tutulur; yeni
  çekim veya değişen çekim işlenir. Hepsini yeniden üretmek için `--zorla`.
- Klasör modunda çıktı adı dosya adından gelir (`kaza.mp4` → `kaza.mp4`);
  `--ad` verilirse ön ek olur (`--ad o5` → `o5_kaza.mp4`).
- `--hedef <klasör>` çıktı yerini değiştirir (deneme için faydalı).
- `--yayinla` önce paketleyip **surge aynasına** yükler, sonra freebuff için
  `yayin/deploy.json` manifestini hatırlatır. Bir çekim hata verirse diğerleri
  devam eder; sonunda hata listesi ve çıkış kodu 1 döner.
- Uzun dönüşümlerde ilerlemeyi satır satır görmek için sarmalayıcı:
  ```bash
  python araclar/klip_izle.py cekimler/ --yayinla
  ```
  Aynı seçenekleri alır; her satırı geçen süre ve `tamamlanan/toplam` sayacıyla
  basar, çıktıyı `_gecici/klip_uretim.log` dosyasına yazar (`--log` ile
  değiştirilir). Üretim ayrı süreçte döner; çıkış kodu üretimden aynen gelir.

- Girdi 1080×1920 (dikey) de 1920×1080 (yatay) de olabilir; oran otomatik
  algılanır ve 16:9'a kırpılır.
- **Akıllı kırpma (varsayılan):** kareden örnekler alınıp "detay enerjisi"
  profili çıkarılır. Konu ortadaysa merkezden, değilse **üst-orta** banttan
  kırpılır (gökyüzü/tavan payı korunur). Karar komut çıktısında yazılır:
  `kırpma: üst-orta · konu bandı y≈%22`.
  Zorlamak için: `--kirpma merkez` veya `--kirpma ust-orta`.
- Son 0,5 saniye başlangıçla çapraz geçiş yapar → döngüde atlama olmaz
  (kapatmak için `--klavuzsuz`).
- Çıktı: `site/video/hero/<ad>.mp4` (960×540, 24 fps, sessiz) + `<ad>.jpg` poster.
- 300 KB üstünde uyarı verir; `--crf 33` ile küçültün.

Elle yapmak isterseniz aynı iş:

```bash
ffmpeg -y -ss 00:00:03 -t 6 -i ham_cek.webm \
  -vf "scale=960:540:force_original_aspect_ratio=increase,crop=960:540,fps=24" \
  -c:v libx264 -preset slow -crf 30 -pix_fmt yuv420p -movflags +faststart -an \
  site/video/hero/kaza.mp4
ffmpeg -y -ss 3 -i site/video/hero/kaza.mp4 -frames:v 1 \
  -vf scale=960:540 -q:v 3 site/video/hero/kaza.jpg
```

## 4. Yeni klip eklemek / çıkarmak

- **Ekle:** videoyu ve posterini yukarıdaki adla koyun. `site/anasayfa.html`
  içindeki `klipler` dizisine satırı **betik kendisi ekler** (dosyalar
  `site/video/hero/` içindeyse): `{ v: 'video/hero/yeni.mp4', p: 'video/hero/yeni.jpg' }`
  Elle eklemek istemezseniz `--baglama-yok`, sonucu önce görmek için `--kuru`.
  Var olan adlar ve dosyası bulunmayan adlar tekrar eklenmez (idempotent).
- **Çıkar:** dizideki satırı silin, dosyaları kaldırın. En az bir klip kalmalı
  (tek klip varsa seçici onu oynatır).

## 5. Yayına alma

Yayından sonra kliplerin canlıda gerçekten oynadığı otomatik doğrulanır:

```bash
python araclar/canli_dogrula.py          # iki adres: kısa adres + freebuff
python araclar/canli_dogrula.py --surge  # yalnız kısa adres
python araclar/canli_dogrula.py --kisa   # indirmeden hızlı kontrol
```

Bakılanlar: ana sayfa listesinde klip var mı, `video/mp4` + boyut yerelle aynı mı,
`Range` 206 dönüyor mu, poster `image/jpeg` mi ve **dosya indirilip ffprobe ile
çözülüyor mu** (h264, 960×540, ~6 sn, ses yok). Sorun varsa çıkış kodu 1.
`--yayinla` bu kontrolü surge yüklemesinden sonra kendisi çalıştırır;
atlamak için `--dogrula-yok`.

```bash
python araclar/yayin_hazirla.py https://<site-adresi>
```

`yayin/public/video/hero/` klasörü kaynaklardan yeniden üretilir. Sonra yayın
klasörünü (`yayin/deploy.json` manifesti) yayına alın. Deploy sonrası:

- `/video/hero/<ad>.mp4` → `200 video/mp4`
- Ana sayfada videonun oynaması ve her yenilemede farklı klip gelmesi

## 6. Kontrol listesi

- [ ] Klip döngüde atlamıyor (son kare ≈ ilk kare)
- [ ] Poster videonun ilk karesiyle uyumlu
- [ ] Sol taraf koyu kalıyor (başlık ve telefon butonu orada okunur)
- [ ] Dosya ≤ 300 KB, toplam klasör ≤ 2 MB
- [ ] Mobilde (390 px) taşma yok, video oynuyor
- [ ] `prefers-reduced-motion` veya tasarruf modunda sadece poster görünüyor

## Notlar

- Eski `site/video/film-*.mp4` ve `site/sahneler/` bu hero'da kullanılmıyor;
  ana sayfa yalnızca `site/video/hero/` içindekileri yayınlar.
- Yeni sahne üretmek gerekirse: `araclar/hero_kaydet.py` (yerel kare yakalama
  sunucusu) + `araclar/hero_sahne.html` (three.js sahneleri) kullanılır.
